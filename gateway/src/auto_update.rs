use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context};
use directories::ProjectDirs;
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::{Deserialize, Serialize};

pub const UPDATE_INTERVAL: u64 = 24 * 60 * 60;
pub const UPDATE_MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/islee23520/Lux/main/gateway/update-manifest.json";

#[derive(Debug, Deserialize)]
pub struct UpdateManifest {
    pub latest_commit: String,
    pub install: UpdateInstall,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInstall {
    pub git: String,
    pub branch: String,
    pub package: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct UpdateCheckState {
    pub last_checked_unix: u64,
    pub last_seen_commit: Option<String>,
    pub last_error: Option<String>,
}

pub fn current_build_commit() -> Option<&'static str> {
    option_env!("LUX_BUILD_COMMIT").filter(|commit| !commit.trim().is_empty())
}

pub fn update_cache_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "linalab", "lux")
        .map(|dirs| dirs.cache_dir().join("update-check.json"))
}

pub fn update_check_due() -> bool {
    let Some(path) = update_cache_path() else {
        return false;
    };

    let Ok(contents) = fs::read_to_string(path) else {
        return true;
    };
    let Ok(state) = serde_json::from_str::<UpdateCheckState>(&contents) else {
        return true;
    };
    now_unix().saturating_sub(state.last_checked_unix) >= UPDATE_INTERVAL
}

pub fn maybe_spawn_update_check(no_update_check: bool) {
    if no_update_check || std::env::var_os("LUX_NO_UPDATE_CHECK").is_some() {
        return;
    }
    if !update_check_due() {
        return;
    }
    if let Err(err) = write_cache_state(&UpdateCheckState {
        last_checked_unix: now_unix(),
        last_seen_commit: current_build_commit().map(str::to_string),
        last_error: None,
    }) {
        eprintln!("⚠️  Could not record Lux update check state: {err:#}");
    }

    let Ok(current_exe) = std::env::current_exe() else {
        return;
    };

    let spawn_result = Command::new(current_exe)
        .arg("--lux-update-worker")
        .arg("--no-update-check")
        .arg("schema")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    if let Err(err) = spawn_result {
        eprintln!("⚠️  Could not start Lux update check in background: {err}");
    }
}

pub async fn run_update_worker() -> anyhow::Result<()> {
    match run_update_worker_inner().await {
        Ok(state) => write_cache_state(&state),
        Err(err) => {
            let message = format!("{err:#}");
            let state = UpdateCheckState {
                last_checked_unix: now_unix(),
                last_seen_commit: current_build_commit().map(str::to_string),
                last_error: Some(message.clone()),
            };
            let cache_result = write_cache_state(&state);
            eprintln!("⚠️  Lux update check failed: {message}");
            cache_result
        }
    }
}

async fn run_update_worker_inner() -> anyhow::Result<UpdateCheckState> {
    let current_commit = current_build_commit().unwrap_or("unknown").to_string();
    let manifest = fetch_update_manifest().await?;
    let latest_commit = manifest.latest_commit.trim().to_string();

    if latest_commit.is_empty() {
        bail!("update manifest latest_commit is empty");
    }

    if latest_commit != current_commit {
        eprintln!("🔄 Update available: newer Lux version detected, installing in background...");
        spawn_cargo_install(&manifest.install)?;
    }

    Ok(UpdateCheckState {
        last_checked_unix: now_unix(),
        last_seen_commit: Some(latest_commit),
        last_error: None,
    })
}

async fn fetch_update_manifest() -> anyhow::Result<UpdateManifest> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .context("failed to build update check HTTP client")?;
    let mut request = client
        .get(UPDATE_MANIFEST_URL)
        .header(USER_AGENT, "lux-cli-update-check");

    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.trim().is_empty() {
            request = request.header(AUTHORIZATION, format!("Bearer {}", token.trim()));
        }
    }

    let response = request
        .send()
        .await
        .context("failed to fetch update manifest")?;
    if !response.status().is_success() {
        bail!(
            "update manifest request returned HTTP {}",
            response.status()
        );
    }
    response
        .json::<UpdateManifest>()
        .await
        .context("failed to parse update manifest")
}

fn spawn_cargo_install(install: &UpdateInstall) -> anyhow::Result<()> {
    Command::new("cargo")
        .args([
            "install",
            "--git",
            install.git.as_str(),
            "--branch",
            install.branch.as_str(),
            "--package",
            install.package.as_str(),
            "--force",
            "--locked",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to start background cargo install")?;
    Ok(())
}

fn write_cache_state(state: &UpdateCheckState) -> anyhow::Result<()> {
    let path = update_cache_path().context("Lux update cache directory is unavailable")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create Lux update cache directory")?;
    }
    let contents = serde_json::to_vec_pretty(state).context("failed to serialize update state")?;
    fs::write(path, contents).context("failed to write Lux update cache")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
