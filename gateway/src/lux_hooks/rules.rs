use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ProjectGovernance {
    pub settings: ProjectSettingsReport,
    pub loaded_rule_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettingsReport {
    pub status: String,
    pub path: Option<PathBuf>,
    pub version: Option<u64>,
    pub enabled_gates: Vec<String>,
    pub forbidden_markers: Vec<String>,
    pub allow_markers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ProjectSettingsFile {
    version: u64,
    hooks: Option<HookSettings>,
    policy: Option<PolicySettings>,
}

#[derive(Debug, Deserialize)]
struct HookSettings {
    pre_work_rule_load: Option<bool>,
    post_edit_policy: Option<bool>,
    verification_evidence: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PolicySettings {
    forbidden_markers: Option<Vec<String>>,
    allow_markers: Option<Vec<String>>,
}

pub fn load_project_governance(project_path: &Path) -> Result<ProjectGovernance> {
    let settings_path = project_path.join(".lux-agent.toml");
    let settings = match std::fs::read_to_string(&settings_path) {
        Ok(text) => {
            let parsed = toml::from_str::<ProjectSettingsFile>(&text)
                .with_context(|| format!("failed to parse {}", settings_path.display()))?;
            configured_settings(settings_path, parsed)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => not_configured_settings(),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read {}", settings_path.display()))
        }
    };
    let loaded_rule_paths = discover_agents_rule_paths(project_path);
    Ok(ProjectGovernance {
        settings,
        loaded_rule_paths,
    })
}

fn configured_settings(path: PathBuf, parsed: ProjectSettingsFile) -> ProjectSettingsReport {
    let hooks = parsed.hooks.unwrap_or(HookSettings {
        pre_work_rule_load: None,
        post_edit_policy: None,
        verification_evidence: None,
    });
    let policy = parsed.policy.unwrap_or(PolicySettings {
        forbidden_markers: None,
        allow_markers: None,
    });
    let mut enabled_gates = Vec::new();
    if hooks.pre_work_rule_load.unwrap_or(false) {
        enabled_gates.push("pre_work_rule_load".to_string());
    }
    if hooks.post_edit_policy.unwrap_or(false) {
        enabled_gates.push("post_edit_policy".to_string());
    }
    if hooks.verification_evidence.unwrap_or(false) {
        enabled_gates.push("verification_evidence".to_string());
    }
    ProjectSettingsReport {
        status: "configured".to_string(),
        path: Some(path),
        version: Some(parsed.version),
        enabled_gates,
        forbidden_markers: policy
            .forbidden_markers
            .unwrap_or_else(default_forbidden_markers),
        allow_markers: policy.allow_markers.unwrap_or_else(default_allow_markers),
    }
}

fn not_configured_settings() -> ProjectSettingsReport {
    ProjectSettingsReport {
        status: "not_configured".to_string(),
        path: None,
        version: None,
        enabled_gates: Vec::new(),
        forbidden_markers: default_forbidden_markers(),
        allow_markers: default_allow_markers(),
    }
}

fn default_forbidden_markers() -> Vec<String> {
    [
        ["TO", "DO"].concat(),
        ["FIX", "ME"].concat(),
        ["HA", "CK"].concat(),
    ]
    .into_iter()
    .collect()
}

fn default_allow_markers() -> Vec<String> {
    [
        "lux-allow-failover",
        "lux-allow-legacy",
        "lux-allow-dual-write",
    ]
    .iter()
    .map(|marker| (*marker).to_string())
    .collect()
}

fn discover_agents_rule_paths(project_path: &Path) -> Vec<PathBuf> {
    let mut directories = project_path
        .ancestors()
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();
    directories.reverse();
    directories
        .into_iter()
        .map(|directory| directory.join("AGENTS.md"))
        .filter(|path| path.is_file())
        .collect()
}
