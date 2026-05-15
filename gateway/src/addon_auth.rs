use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct AddonAuthConfig {
    pub github_client_id: String,
    pub github_client_secret: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

#[derive(Debug, Deserialize)]
struct GithubRepo {
    full_name: String,
}

pub async fn start_device_flow(client_id: &str) -> Result<DeviceCodeResponse> {
    let client = Client::new();
    let res = client
        .post("https://github.com/login/device/code")
        .header("Accept", "application/json")
        .form(&[("client_id", client_id)])
        .send()
        .await
        .context("Failed to send device code request")?;

    let data: DeviceCodeResponse = res
        .json()
        .await
        .context("Failed to parse device code response")?;
    Ok(data)
}

pub async fn poll_device_token(
    client_id: &str,
    device_code: &str,
) -> Result<Option<AccessTokenResponse>> {
    let client = Client::new();
    let res = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .context("Failed to send token request")?;

    let text = res.text().await?;
    if text.contains("authorization_pending") {
        return Ok(None);
    }

    let data: AccessTokenResponse =
        serde_json::from_str(&text).context("Failed to parse access token response")?;
    Ok(Some(data))
}

pub async fn check_repo_access(access_token: &str) -> Result<Vec<String>> {
    let client = Client::new();
    let res = client
        .get("https://api.github.com/user/repos?per_page=100")
        .header("Accept", "application/vnd.github.v3+json")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "Lux-Addon-Manager")
        .send()
        .await
        .context("Failed to fetch user repos")?;

    let repos: Vec<GithubRepo> = res.json().await.context("Failed to parse repos")?;
    let accessible_repos = repos
        .into_iter()
        .filter(|r| r.full_name.starts_with("linalab/com.linalab."))
        .map(|r| r.full_name)
        .collect();

    Ok(accessible_repos)
}

pub const DEFAULT_TOKEN_TTL_SECS: u64 = 86400;

pub fn issue_addon_token(gateway_token: &str, repos: &[String]) -> Result<String> {
    issue_addon_token_with_ttl(gateway_token, repos, DEFAULT_TOKEN_TTL_SECS)
}

pub fn issue_addon_token_with_ttl(
    gateway_token: &str,
    repos: &[String],
    ttl_secs: u64,
) -> Result<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(gateway_token.as_bytes())
        .context("Failed to create HMAC")?;

    let repos_str = repos.join(",");
    let expires_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + ttl_secs;

    let payload = format!("{}:{}", repos_str, expires_at);
    mac.update(payload.as_bytes());
    let signature = mac.finalize().into_bytes();

    let token_data = format!(
        "{}:{}",
        payload,
        general_purpose::URL_SAFE_NO_PAD.encode(signature)
    );
    Ok(general_purpose::URL_SAFE_NO_PAD.encode(token_data))
}

#[derive(Debug)]
pub struct VerifiedAddonToken {
    pub repos: Vec<String>,
    pub expires_at: u64,
}

pub fn verify_addon_token(gateway_token: &str, token: &str) -> Result<VerifiedAddonToken> {
    let decoded = String::from_utf8(general_purpose::URL_SAFE_NO_PAD.decode(token)?)?;
    let parts: Vec<&str> = decoded.split(':').collect();
    if parts.len() != 3 {
        anyhow::bail!("Invalid token format");
    }

    let repos_str = parts[0];
    let expires_at: u64 = parts[1].parse()?;
    let signature_b64 = parts[2];

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if now >= expires_at {
        anyhow::bail!("Token expired");
    }

    let mut mac = Hmac::<Sha256>::new_from_slice(gateway_token.as_bytes())?;
    let payload = format!("{}:{}", repos_str, expires_at);
    mac.update(payload.as_bytes());

    let expected_signature = general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    if signature_b64 != expected_signature {
        anyhow::bail!("Invalid signature");
    }

    Ok(VerifiedAddonToken {
        repos: repos_str.split(',').map(|s| s.to_string()).collect(),
        expires_at,
    })
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GithubRepoVisibility {
    full_name: String,
    private: bool,
}

pub async fn check_repo_visibility(
    owner: &str,
    repo: &str,
    access_token: Option<&str>,
) -> Result<RepoVisibility> {
    let client = Client::new();
    let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("User-Agent", "Lux-Addon-Manager");

    if let Some(token) = access_token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let res = req.send().await.context("Failed to fetch repo info")?;

    if res.status().as_u16() == 404 {
        return Ok(RepoVisibility::NotFound);
    }

    if !res.status().is_success() {
        anyhow::bail!("GitHub API returned status {}", res.status());
    }

    let repo_info: GithubRepoVisibility = res.json().await?;
    if repo_info.private {
        Ok(RepoVisibility::Private)
    } else {
        Ok(RepoVisibility::Public)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum RepoVisibility {
    Public,
    Private,
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_issue_and_verify() {
        let gateway_token = "secret-gateway-token";
        let repos = vec!["linalab/com.linalab.test".to_string()];

        let token = issue_addon_token(gateway_token, &repos).unwrap();
        let verified = verify_addon_token(gateway_token, &token).unwrap();

        assert_eq!(repos, verified.repos);
        assert!(verified.expires_at > 0);
    }

    #[test]
    fn test_invalid_token_wrong_gateway_key() {
        let gateway_token = "secret-gateway-token";
        let repos = vec!["linalab/com.linalab.test".to_string()];

        let token = issue_addon_token(gateway_token, &repos).unwrap();
        let result = verify_addon_token("wrong-token", &token);

        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token_is_rejected() {
        let gateway_token = "secret-gateway-token";
        let repos = vec!["linalab/com.linalab.test".to_string()];

        let token = issue_addon_token_with_ttl(gateway_token, &repos, 0).unwrap();
        let result = verify_addon_token(gateway_token, &token);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("expired"),
            "expected expired error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_token_with_custom_ttl() {
        let gateway_token = "secret-gateway-token";
        let repos = vec!["linalab/com.linalab.test".to_string()];

        let token = issue_addon_token_with_ttl(gateway_token, &repos, 7200).unwrap();
        let verified = verify_addon_token(gateway_token, &token).unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let diff = verified.expires_at.abs_diff(now);
        assert!(
            diff >= 7190 && diff <= 7210,
            "expected ~7200s TTL, got diff={}",
            diff
        );
    }

    #[test]
    fn test_token_default_ttl_is_24h() {
        let gateway_token = "secret-gateway-token";
        let repos = vec!["linalab/com.linalab.test".to_string()];

        let token = issue_addon_token(gateway_token, &repos).unwrap();
        let verified = verify_addon_token(gateway_token, &token).unwrap();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let diff = verified.expires_at.abs_diff(now);
        assert!(
            diff >= 86390 && diff <= 86410,
            "expected ~86400s TTL, got diff={}",
            diff
        );
    }

    #[test]
    fn test_multiple_repos_in_token() {
        let gateway_token = "secret-gateway-token";
        let repos = vec![
            "linalab/com.linalab.lux".to_string(),
            "linalab/com.linalab.unity-log".to_string(),
            "linalab/com.linalab.easy-fps".to_string(),
        ];

        let token = issue_addon_token(gateway_token, &repos).unwrap();
        let verified = verify_addon_token(gateway_token, &token).unwrap();

        assert_eq!(repos, verified.repos);
    }

    #[test]
    fn test_malformed_token_rejected() {
        let result = verify_addon_token("key", "not-a-valid-token");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_renewal_issues_new_token() {
        let gateway_token = "secret-gateway-token";
        let repos = vec!["linalab/com.linalab.lux".to_string()];

        let token1 = issue_addon_token_with_ttl(gateway_token, &repos, 3600).unwrap();
        let verified1 = verify_addon_token(gateway_token, &token1).unwrap();

        let token2 = issue_addon_token(gateway_token, &repos).unwrap();
        let verified2 = verify_addon_token(gateway_token, &token2).unwrap();

        assert!(verified2.expires_at > verified1.expires_at);
        assert_eq!(verified1.repos, verified2.repos);
    }
}
