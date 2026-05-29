// Periodic sync worker for uloop command manifest parity check
// Follows auto_update.rs pattern exactly

use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use directories::ProjectDirs;
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};

/// Sync interval: check weekly (7 days) for uloop manifest updates
pub const ULOOP_SYNC_INTERVAL: u64 = 7 * 24 * 60 * 60;

/// URL for the Lux-maintained uloop command manifest
pub const ULOOP_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/islee23520/Lux/main/gateway/assets/uloop-manifest.json";

pub const BUNDLED_MANIFEST_PATH: &str = "gateway/assets/uloop-manifest.json";
const BUNDLED_MANIFEST_JSON: &str = include_str!("../assets/uloop-manifest.json");

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct UloopSyncState {
    pub last_checked_unix: u64,
    pub last_seen_version: Option<String>,
    pub coverage_pct: f64,
    pub total_commands: u32,
    pub supported_commands: u32,
    pub missing_commands: Vec<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UloopManifest {
    pub version: String,
    pub updated_at: String,
    pub commands: Vec<UloopCommandDef>,
}

#[derive(Debug, Deserialize)]
pub struct UloopCommandDef {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub args: Vec<UloopArgDef>,
}

#[derive(Debug, Deserialize)]
pub struct UloopArgDef {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    pub help: String,
    #[serde(default)]
    pub choices: Vec<String>,
}

pub fn uloop_sync_cache_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "linalab", "lux").map(|dirs| dirs.cache_dir().join("uloop-sync.json"))
}

pub fn uloop_sync_check_due() -> bool {
    let Some(path) = uloop_sync_cache_path() else {
        return true;
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return true;
    };
    let Ok(state) = serde_json::from_str::<UloopSyncState>(&contents) else {
        return true;
    };
    now_unix().saturating_sub(state.last_checked_unix) >= ULOOP_SYNC_INTERVAL
}

pub fn maybe_spawn_uloop_sync_worker() {
    if std::env::var_os("LUX_NO_ULOOP_SYNC").is_some() {
        return;
    }
    if !uloop_sync_check_due() {
        return;
    }
    if let Err(err) = write_uloop_sync_state(&UloopSyncState {
        last_checked_unix: now_unix(),
        ..Default::default()
    }) {
        eprintln!("⚠️  Could not record uloop sync check state: {err:#}");
    }

    let Ok(current_exe) = std::env::current_exe() else {
        return;
    };

    let spawn_result = Command::new(current_exe)
        .arg("--uloop-sync-worker")
        .arg("schema")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    if let Err(err) = spawn_result {
        eprintln!("⚠️  Could not start uloop sync worker: {err}");
    }
}

pub async fn run_uloop_sync_worker() -> anyhow::Result<()> {
    match run_uloop_sync_worker_inner().await {
        Ok(state) => write_uloop_sync_state(&state),
        Err(err) => {
            let message = format!("{err:#}");
            let state = UloopSyncState {
                last_checked_unix: now_unix(),
                last_error: Some(message.clone()),
                ..Default::default()
            };
            let cache_result = write_uloop_sync_state(&state);
            eprintln!("⚠️  Uloop sync failed: {message}");
            cache_result
        }
    }
}

async fn run_uloop_sync_worker_inner() -> anyhow::Result<UloopSyncState> {
    let manifest = fetch_uloop_manifest().await?;
    let total = manifest.commands.len() as u32;
    let missing: Vec<String> = manifest
        .commands
        .iter()
        .map(|command| command.name.clone())
        .filter(|name| {
            matches!(
                name.as_str(),
                "get-project-info"
                    | "get-version"
                    | "sync"
                    | "fix"
                    | "list"
                    | "package"
                    | "scene"
                    | "asset"
                    | "init"
                    | "build"
                    | "test"
                    | "run"
            )
        })
        .collect();
    let supported = total.saturating_sub(missing.len() as u32);
    let coverage_pct = if total > 0 {
        (supported as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    if !missing.is_empty() {
        eprintln!(
            "🔄 Uloop sync: {} commands not yet covered by lux unity: {:?}",
            missing.len(),
            missing
        );
    } else {
        eprintln!(
            "✅ Uloop sync: {} commands at 100% coverage ({})",
            total, manifest.version
        );
    }

    Ok(UloopSyncState {
        last_checked_unix: now_unix(),
        last_seen_version: Some(manifest.version),
        coverage_pct,
        total_commands: total,
        supported_commands: supported,
        missing_commands: missing,
        last_error: None,
    })
}

async fn fetch_uloop_manifest() -> anyhow::Result<UloopManifest> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("failed to build uloop sync HTTP client")?;
    let mut request = client
        .get(ULOOP_MANIFEST_URL)
        .header(USER_AGENT, "lux-uloop-sync");

    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            request = request.header(AUTHORIZATION, format!("Bearer {}", token.trim()));
        }
    }

    match request.send().await {
        Ok(response) if response.status().is_success() => response
            .json::<UloopManifest>()
            .await
            .context("failed to parse uloop manifest"),
        _ => {
            eprintln!("ℹ️  Remote uloop manifest unavailable, using bundled fallback");
            serde_json::from_str(BUNDLED_MANIFEST_JSON).with_context(|| {
                format!("failed to parse bundled uloop manifest at {BUNDLED_MANIFEST_PATH}")
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{UloopManifest, BUNDLED_MANIFEST_JSON, BUNDLED_MANIFEST_PATH};

    #[test]
    fn bundled_manifest_parses_when_remote_manifest_is_unavailable() {
        // Given: the gateway ships an embedded fallback manifest.
        assert_eq!(BUNDLED_MANIFEST_PATH, "gateway/assets/uloop-manifest.json");

        // When: the embedded JSON is parsed through the production manifest type.
        let manifest = serde_json::from_str::<UloopManifest>(BUNDLED_MANIFEST_JSON)
            .expect("bundled uloop manifest should parse");

        // Then: fallback coverage has a concrete command set.
        assert!(!manifest.commands.is_empty());
    }
}

fn write_uloop_sync_state(state: &UloopSyncState) -> anyhow::Result<()> {
    let path = uloop_sync_cache_path().context("uloop sync cache directory is unavailable")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create uloop sync cache directory")?;
    }
    let contents =
        serde_json::to_vec_pretty(state).context("failed to serialize uloop sync state")?;
    fs::write(path, contents).context("failed to write uloop sync state")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
