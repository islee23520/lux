use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

mod command;
mod gate;
mod install;
mod policy;
mod rules;
mod run;

pub use command::run_hooks_command;
pub use install::{codex_hook_status, install_codex_hook_bridge};
use policy::PolicyFinding;
use rules::ProjectSettingsReport;
pub use run::run_hook_bridge;

pub(super) const DEFAULT_CODEX_EVENTS: &[&str] = &["UserPromptSubmit"];
pub(super) const LUX_PROJECT_EVENTS: &[&str] = &[
    "LuxPreWorkRuleLoad",
    "LuxPostEditPolicy",
    "LuxVerificationEvidence",
];

#[derive(Debug, Clone, Parser)]
pub struct HooksArgs {
    #[command(subcommand)]
    pub action: HooksAction,
}

#[derive(Debug, Clone, Subcommand)]
pub enum HooksAction {
    /// Install the Lux Codex native hook bridge
    Install(HooksInstallArgs),
    /// Show whether the Lux Codex native hook bridge is installed
    Status(HooksStatusArgs),
    /// Run the Lux hook bridge for a native hook event
    Run(HooksRunArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct HooksInstallArgs {
    /// Unity project root that owns the .lux runtime state
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Path to Codex hooks.json; defaults to CODEX_HOME/hooks.json or ~/.codex/hooks.json
    #[arg(long)]
    pub hooks_path: Option<PathBuf>,
    /// Print the planned hook changes without writing hooks.json
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// Replace an existing Lux hook command for this project
    #[arg(long, default_value_t = false)]
    pub force: bool,
    /// Print machine-readable JSON
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct HooksStatusArgs {
    /// Unity project root that owns the .lux runtime state
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Path to Codex hooks.json; defaults to CODEX_HOME/hooks.json or ~/.codex/hooks.json
    #[arg(long)]
    pub hooks_path: Option<PathBuf>,
    /// Print machine-readable JSON
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct HooksRunArgs {
    /// Native hook event name, such as UserPromptSubmit
    #[arg(long)]
    pub event: String,
    /// Unity project root that owns the .lux runtime state
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Print machine-readable JSON
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInstallReport {
    pub hooks_path: PathBuf,
    pub project_path: PathBuf,
    pub dry_run: bool,
    pub changed: bool,
    pub installed: Vec<HookEventInstallReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEventInstallReport {
    pub event: String,
    pub command: String,
    pub already_installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookStatusReport {
    pub hooks_path: PathBuf,
    pub project_path: PathBuf,
    pub events: Vec<HookEventStatusReport>,
    pub lux_events: Vec<String>,
    pub project_settings: ProjectSettingsReport,
    pub loaded_rule_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEventStatusReport {
    pub event: String,
    pub installed: bool,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRunReport {
    pub event_id: String,
    pub event: String,
    pub project_path: PathBuf,
    pub event_log_path: PathBuf,
    pub source: String,
    pub ulw_detected: bool,
    pub omx_ultrawork: OmxUltraworkStatus,
    pub project_settings: ProjectSettingsReport,
    pub loaded_rule_paths: Vec<PathBuf>,
    pub gate_result: HookGateResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookGateResult {
    pub status: String,
    pub findings: Vec<PolicyFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OmxUltraworkStatus {
    Active {
        state_path: PathBuf,
        reinforcement_count: Option<u64>,
    },
    Inactive {
        state_path: PathBuf,
        reinforcement_count: Option<u64>,
    },
    NotFound,
    Invalid {
        state_path: PathBuf,
        error: String,
    },
}

pub(super) fn resolve_project_path(path: Option<&PathBuf>) -> Result<PathBuf> {
    match path {
        Some(path) => Ok(path.clone()),
        None => std::env::current_dir().context("failed to resolve current directory"),
    }
}

pub(super) fn resolve_hooks_path(path: Option<&PathBuf>) -> Result<PathBuf> {
    if let Some(path) = path {
        return Ok(path.clone());
    }
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        if !codex_home.trim().is_empty() {
            return Ok(PathBuf::from(codex_home).join("hooks.json"));
        }
    }
    let home = if cfg!(windows) {
        match std::env::var("USERPROFILE") {
            Ok(value) => Some(value),
            Err(_) => None,
        }
    } else {
        match std::env::var("HOME") {
            Ok(value) => Some(value),
            Err(_) => None,
        }
    }
    .context("failed to resolve HOME for default Codex hooks path")?;
    Ok(PathBuf::from(home).join(".codex").join("hooks.json"))
}

pub(super) fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "/._:-".contains(character))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub(super) fn write_json_atomic(path: &Path, value: &Value) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("{} has no valid UTF-8 file name", path.display()))?;
    let tmp_path = parent.join(format!(".{file_name}.tmp"));
    reject_or_remove_legacy_temp(&tmp_path)?;
    let tmp_path = unique_temp_path(path)?;
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    let mut tmp_file = options
        .open(&tmp_path)
        .with_context(|| format!("failed to create temporary file {}", tmp_path.display()))?;
    tmp_file
        .write_all(serde_json::to_string_pretty(value)?.as_bytes())
        .with_context(|| format!("failed to write temporary file {}", tmp_path.display()))?;
    tmp_file
        .write_all(b"\n")
        .with_context(|| format!("failed to write newline to {}", tmp_path.display()))?;
    tmp_file
        .sync_all()
        .with_context(|| format!("failed to sync temporary file {}", tmp_path.display()))?;
    drop(tmp_file);
    fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "failed to rename temporary file {} to {}",
            tmp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

fn reject_or_remove_legacy_temp(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to inspect temporary file {}", path.display()));
        }
    };
    if metadata.file_type().is_symlink() {
        anyhow::bail!("temporary file must not be a symlink: {}", path.display());
    }
    #[cfg(unix)]
    if metadata.nlink() > 1 {
        anyhow::bail!("temporary file must not be hardlinked: {}", path.display());
    }
    if metadata.is_file() {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove stale temporary file {}", path.display()))?;
    }
    Ok(())
}

fn unique_temp_path(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("{} has no valid UTF-8 file name", path.display()))?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX epoch")?
        .as_nanos();
    Ok(parent.join(format!(".{file_name}.{}.{}.tmp", std::process::id(), nanos)))
}
