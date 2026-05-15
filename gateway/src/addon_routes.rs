use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{addon_auth, addon_store::AddonEntry, server::GatewayState};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterAddonRequest {
    repo_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceTokenRequest {
    device_code: String,
}

#[derive(Serialize)]
struct AuthStatusResponse {
    status: String,
    accessible_repos: Vec<String>,
}

#[derive(Serialize)]
struct VisibilityResponse {
    id: String,
    name: String,
    visibility: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenewTokenRequest {
    addon_token: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenewTokenResponse {
    token: String,
    expires_at: u64,
}

fn require_token(state: &GatewayState, headers: &HeaderMap) -> Result<(), Response> {
    let token = headers
        .get("x-lux-token")
        .and_then(|value| value.to_str().ok());
    if state.accepts_token(token) {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            "invalid or missing Lux gateway token",
        )
            .into_response())
    }
}

async fn list_addons(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AddonEntry>>, Response> {
    require_token(&state, &headers)?;
    let addons = state.addon_store.lock().await.list();
    Ok(Json(addons))
}

async fn register_addon(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<RegisterAddonRequest>,
) -> Result<(StatusCode, Json<AddonEntry>), Response> {
    require_token(&state, &headers)?;

    let id = uuid::Uuid::new_v4().to_string();
    let name = request
        .repo_url
        .split('/')
        .last()
        .unwrap_or("unknown")
        .to_string();

    let addon = AddonEntry {
        id: id.clone(),
        name,
        repo_url: request.repo_url,
        version: "0.1.0".to_string(),
        description: "Auto-registered addon".to_string(),
        auth_status: "unverified".to_string(),
        accessible: false,
        visibility: "unknown".to_string(),
    };

    state.addon_store.lock().await.register(addon.clone());
    Ok((StatusCode::CREATED, Json(addon)))
}

async fn unregister_addon(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, Response> {
    require_token(&state, &headers)?;

    if state.addon_store.lock().await.unregister(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((StatusCode::NOT_FOUND, "Addon not found").into_response())
    }
}

async fn start_device_flow(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<addon_auth::DeviceCodeResponse>, Response> {
    require_token(&state, &headers)?;

    let client_id = state.config.addon_auth.github_client_id.clone();
    let response = addon_auth::start_device_flow(&client_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Json(response))
}

async fn poll_device_token(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<DeviceTokenRequest>,
) -> Result<Json<addon_auth::AccessTokenResponse>, Response> {
    require_token(&state, &headers)?;

    let client_id = state.config.addon_auth.github_client_id.clone();
    let response = addon_auth::poll_device_token(&client_id, &request.device_code)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    if let Some(token_response) = response {
        Ok(Json(token_response))
    } else {
        Err((StatusCode::ACCEPTED, "authorization_pending").into_response())
    }
}

async fn check_auth_status(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<AuthStatusResponse>, Response> {
    require_token(&state, &headers)?;

    let tokens = state.addon_tokens.lock().await;
    if let Some((_key, scoped)) = tokens.iter().next() {
        Ok(Json(AuthStatusResponse {
            status: "authenticated".to_string(),
            accessible_repos: scoped.repos.clone(),
        }))
    } else {
        Ok(Json(AuthStatusResponse {
            status: "unauthenticated".to_string(),
            accessible_repos: vec![],
        }))
    }
}

async fn verify_addon(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AddonEntry>, Response> {
    require_token(&state, &headers)?;

    let mut store = state.addon_store.lock().await;
    if let Some(mut addon) = store.get(&id) {
        addon.auth_status = "verified".to_string();
        addon.accessible = true;
        store.update_auth_status(&id, "verified".to_string(), true);
        Ok(Json(addon))
    } else {
        Err((StatusCode::NOT_FOUND, "Addon not found").into_response())
    }
}

async fn get_visibility(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<VisibilityResponse>, Response> {
    require_token(&state, &headers)?;

    let store = state.addon_store.lock().await;
    let addon = store
        .get(&id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Addon not found").into_response())?;

    Ok(Json(VisibilityResponse {
        id: addon.id.clone(),
        name: addon.name.clone(),
        visibility: addon.visibility.clone(),
    }))
}

async fn renew_addon_token(
    State(state): State<GatewayState>,
    headers: HeaderMap,
    Json(request): Json<RenewTokenRequest>,
) -> Result<Json<RenewTokenResponse>, Response> {
    require_token(&state, &headers)?;

    let gateway_token = &state.config.token;

    if let Some(addon_token) = request.addon_token {
        let verified =
            addon_auth::verify_addon_token(gateway_token, &addon_token).map_err(|e| {
                if e.to_string().contains("expired") {
                    (
                        StatusCode::UNAUTHORIZED,
                        format!("Addon token expired: {}", e),
                    )
                        .into_response()
                } else {
                    (StatusCode::UNAUTHORIZED, e.to_string()).into_response()
                }
            })?;

        let new_token =
            addon_auth::issue_addon_token(gateway_token, &verified.repos).map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to issue token: {}", e),
                )
                    .into_response()
            })?;

        let new_verified = addon_auth::verify_addon_token(gateway_token, &new_token)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

        Ok(Json(RenewTokenResponse {
            token: new_token,
            expires_at: new_verified.expires_at,
        }))
    } else {
        Err((StatusCode::BAD_REQUEST, "addon_token field is required").into_response())
    }
}

async fn discover_addons(
    State(state): State<GatewayState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AddonEntry>>, Response> {
    require_token(&state, &headers)?;

    let project_root = state.config.project_root.clone();
    if let Some(root) = project_root {
        let packages_dir = root.join("Packages");
        let found = crate::addon_store::discover_linalab_packages(&packages_dir);
        let mut store = state.addon_store.lock().await;

        for pkg_name in &found {
            if store.get_by_name(pkg_name).is_none() {
                let id = uuid::Uuid::new_v4().to_string();
                let addon = AddonEntry {
                    id: id.clone(),
                    name: pkg_name.clone(),
                    repo_url: format!("https://github.com/linalab/{}", pkg_name),
                    version: "0.1.0".to_string(),
                    description: format!("Auto-discovered {}", pkg_name),
                    auth_status: "discovered".to_string(),
                    accessible: false,
                    visibility: "unknown".to_string(),
                };
                store.register(addon);
            }
        }
    }

    let addons = state.addon_store.lock().await.list();
    Ok(Json(addons))
}

pub fn routes() -> Router<GatewayState> {
    Router::new()
        .route("/", get(list_addons))
        .route("/register", post(register_addon))
        .route("/:id", delete(unregister_addon))
        .route("/auth/device", post(start_device_flow))
        .route("/auth/token", post(poll_device_token))
        .route("/auth/status", get(check_auth_status))
        .route("/auth/renew", post(renew_addon_token))
        .route("/:id/verify", post(verify_addon))
        .route("/:id/visibility", get(get_visibility))
        .route("/discover", post(discover_addons))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addon_auth::AddonAuthConfig;
    use crate::server::{GatewayConfig, GatewayState};
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    fn create_test_state() -> GatewayState {
        let config = GatewayConfig {
            token: "test-token".to_string(),
            history_capacity: 100,
            project_root: None,
            addon_auth: AddonAuthConfig {
                github_client_id: "test-client-id".to_string(),
                github_client_secret: None,
            },
        };
        GatewayState::new(config)
    }

    fn create_test_state_with_project() -> GatewayState {
        let dir = std::env::temp_dir().join(format!("lux-addon-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("Packages/com.linalab.lux")).unwrap();
        std::fs::create_dir_all(dir.join("Assets")).unwrap();
        std::fs::create_dir_all(dir.join("ProjectSettings")).unwrap();

        let config = GatewayConfig {
            token: "test-token".to_string(),
            history_capacity: 100,
            project_root: Some(dir),
            addon_auth: AddonAuthConfig {
                github_client_id: "test-client-id".to_string(),
                github_client_secret: None,
            },
        };
        GatewayState::new(config)
    }

    fn auth_request(method: &str, uri: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("x-lux-token", "test-token")
            .body(Body::empty())
            .unwrap()
    }

    fn auth_json_request(method: &str, uri: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("x-lux-token", "test-token")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    async fn response_json(body: Body) -> serde_json::Value {
        let bytes = axum::body::to_bytes(body, 1024 * 1024).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_list_addons_unauthorized() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_register_and_list_addons() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let response = app
            .clone()
            .oneshot(auth_json_request(
                "POST",
                "/register",
                serde_json::json!({ "repoUrl": "https://github.com/linalab/com.linalab.lux" }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let json = response_json(response.into_body()).await;
        let id = json["id"].as_str().unwrap();
        assert_eq!(json["name"], "com.linalab.lux");
        assert_eq!(json["visibility"], "unknown");

        let response = app.oneshot(auth_request("GET", "/")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let list = response_json(response.into_body()).await;
        assert_eq!(list.as_array().unwrap().len(), 1);
        let _ = id;
    }

    #[tokio::test]
    async fn test_unregister_addon() {
        let state = create_test_state();
        let app = routes().with_state(state.clone());

        let response = app
            .clone()
            .oneshot(auth_json_request(
                "POST",
                "/register",
                serde_json::json!({ "repoUrl": "https://github.com/linalab/test" }),
            ))
            .await
            .unwrap();
        let json = response_json(response.into_body()).await;
        let id = json["id"].as_str().unwrap().to_string();

        let response = app
            .clone()
            .oneshot(auth_request("DELETE", &format!("/{}", id)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app
            .oneshot(auth_request("DELETE", "/nonexistent"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_visibility() {
        let state = create_test_state();
        let app = routes().with_state(state.clone());

        let response = app
            .clone()
            .oneshot(auth_json_request(
                "POST",
                "/register",
                serde_json::json!({ "repoUrl": "https://github.com/linalab/com.linalab.lux" }),
            ))
            .await
            .unwrap();
        let json = response_json(response.into_body()).await;
        let id = json["id"].as_str().unwrap();

        state
            .addon_store
            .lock()
            .await
            .set_visibility(id, addon_auth::RepoVisibility::Public);

        let response = app
            .oneshot(auth_request("GET", &format!("/{}/visibility", id)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let vis = response_json(response.into_body()).await;
        assert_eq!(vis["visibility"], "public");
        assert_eq!(vis["name"], "com.linalab.lux");
    }

    #[tokio::test]
    async fn test_get_visibility_not_found() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let response = app
            .oneshot(auth_request("GET", "/nonexistent/visibility"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_renew_token_requires_body() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let response = app
            .oneshot(auth_json_request(
                "POST",
                "/auth/renew",
                serde_json::json!({}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_renew_token_with_valid_token() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let repos = vec!["linalab/com.linalab.lux".to_string()];
        let addon_token = addon_auth::issue_addon_token("test-token", &repos).unwrap();

        let response = app
            .oneshot(auth_json_request(
                "POST",
                "/auth/renew",
                serde_json::json!({ "addonToken": addon_token }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = response_json(response.into_body()).await;
        assert!(json["token"].is_string());
        assert!(json["expiresAt"].is_number());
    }

    #[tokio::test]
    async fn test_renew_token_expired_is_401() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let repos = vec!["linalab/com.linalab.lux".to_string()];
        let expired_token =
            addon_auth::issue_addon_token_with_ttl("test-token", &repos, 0).unwrap();

        let response = app
            .oneshot(auth_json_request(
                "POST",
                "/auth/renew",
                serde_json::json!({ "addonToken": expired_token }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_renew_token_wrong_gateway_key_is_401() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let repos = vec!["linalab/com.linalab.lux".to_string()];
        let token = addon_auth::issue_addon_token("wrong-key", &repos).unwrap();

        let response = app
            .oneshot(auth_json_request(
                "POST",
                "/auth/renew",
                serde_json::json!({ "addonToken": token }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_discover_addons_no_project() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let response = app
            .oneshot(auth_request("POST", "/discover"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let list = response_json(response.into_body()).await;
        assert_eq!(list.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_discover_addons_with_project() {
        let state = create_test_state_with_project();
        let project_root = state.config.project_root.clone().unwrap();
        let app = routes().with_state(state);

        let response = app
            .oneshot(auth_request("POST", "/discover"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let list = response_json(response.into_body()).await;
        let discovered = list.as_array().unwrap();
        assert!(!discovered.is_empty());
        let names: Vec<&str> = discovered
            .iter()
            .map(|a| a["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"com.linalab.lux"));

        std::fs::remove_dir_all(project_root).unwrap();
    }

    #[tokio::test]
    async fn test_verify_addon() {
        let state = create_test_state();
        let app = routes().with_state(state.clone());

        let response = app
            .clone()
            .oneshot(auth_json_request(
                "POST",
                "/register",
                serde_json::json!({ "repoUrl": "https://github.com/linalab/test" }),
            ))
            .await
            .unwrap();
        let json = response_json(response.into_body()).await;
        let id = json["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(auth_request("POST", &format!("/{}/verify", id)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let verified = response_json(response.into_body()).await;
        assert_eq!(verified["authStatus"], "verified");
        assert_eq!(verified["accessible"], true);
    }

    #[tokio::test]
    async fn test_verify_addon_not_found() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let response = app
            .oneshot(auth_request("POST", "/nonexistent/verify"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_auth_status_unauthenticated() {
        let state = create_test_state();
        let app = routes().with_state(state);

        let response = app
            .oneshot(auth_request("GET", "/auth/status"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = response_json(response.into_body()).await;
        assert_eq!(json["status"], "unauthenticated");
    }
}
