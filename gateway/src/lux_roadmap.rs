use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};

pub use crate::lux_roadmap_registry::{
    CAPABILITY_GLOBAL_CLI, CAPABILITY_GODOT_BRIDGE_WORKFLOW, CAPABILITY_MCP_STDIO,
    CAPABILITY_THREE_JS_BRIDGE_WORKFLOW, CAPABILITY_UNITY_BRIDGE_WORKFLOW,
    GLOBAL_CLI_MCP_BRIDGE_PHASE,
};
use lux_project::EngineKind;

pub const ROADMAP_SCHEMA_VERSION: &str = "1.0";
pub const SUPPORTED_ROADMAP_SCHEMA_MAJOR_VERSION: &str = "1";
pub const REMOTE_WEBRTC_EXPERIMENTAL_FLAG: &str = "remote_webrtc";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoadmapReality {
    pub schema_version: String,
    pub updated_at: String,
    pub phases: Vec<RoadmapPhase>,
    pub capabilities: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub experimental_flags: HashMap<String, bool>,
    pub authoritative: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoadmapPhase {
    pub name: String,
    pub status: RoadmapPhaseStatus,
    pub evidence_path: Option<String>,
    pub pushed_at: Option<String>,
    pub push_git_sha: Option<String>,
    pub push_evidence_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoadmapPhaseStatus {
    Planned,
    InProgress,
    Partial,
    Scaffolded,
    Complete,
    Pushed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoadmapError {
    Missing { path: PathBuf },
    Corrupt { path: PathBuf, message: String },
    Invalid { path: PathBuf, message: String },
}

impl fmt::Display for RoadmapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RoadmapError::Missing { path } => {
                write!(formatter, "Lux roadmap is missing: {}", path.display())
            }
            RoadmapError::Corrupt { path, message } => write!(
                formatter,
                "Lux roadmap is corrupt at {}: {message}",
                path.display()
            ),
            RoadmapError::Invalid { path, message } => write!(
                formatter,
                "Lux roadmap is invalid at {}: {message}",
                path.display()
            ),
        }
    }
}

impl Error for RoadmapError {}

impl Default for RoadmapReality {
    fn default() -> Self {
        Self {
            schema_version: ROADMAP_SCHEMA_VERSION.to_string(),
            updated_at: Utc::now().to_rfc3339(),
            phases: crate::lux_roadmap_registry::default_phases(),
            capabilities: crate::lux_roadmap_registry::default_capabilities(),
            evidence_refs: Vec::new(),
            experimental_flags: crate::lux_roadmap_registry::default_experimental_flags(),
            authoritative: true,
        }
    }
}

impl RoadmapReality {
    pub fn load(path: &Path) -> Result<Self> {
        load(path)
    }

    pub fn init_or_load(path: &Path) -> Result<Self> {
        init_or_load(path)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let roadmap_path = roadmap_file_path(path);
        self.validate_at(&roadmap_path)?;

        if let Some(parent) = roadmap_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        let json = serde_json::to_string_pretty(self).with_context(|| {
            format!(
                "failed to serialize Lux roadmap for {}",
                roadmap_path.display()
            )
        })?;
        let tmp_path = roadmap_path.with_extension("json.tmp");
        fs::write(&tmp_path, json)
            .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
        fs::rename(&tmp_path, &roadmap_path).with_context(|| {
            format!(
                "failed to atomically replace file {}",
                roadmap_path.display()
            )
        })
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_at(Path::new(".lux/roadmap.json"))
    }

    pub fn flag_enabled(&self, name: &str) -> bool {
        self.experimental_flags.get(name).copied().unwrap_or(false)
    }

    fn validate_at(&self, path: &Path) -> Result<()> {
        let major = self
            .schema_version
            .split('.')
            .next()
            .filter(|value| !value.is_empty())
            .unwrap_or_default();
        if major != SUPPORTED_ROADMAP_SCHEMA_MAJOR_VERSION {
            return Err(RoadmapError::Invalid {
                path: path.to_path_buf(),
                message: format!("unsupported roadmap schema version {}", self.schema_version),
            }
            .into());
        }

        if self.updated_at.trim().is_empty() {
            return Err(RoadmapError::Invalid {
                path: path.to_path_buf(),
                message: "updated_at must be present".to_string(),
            }
            .into());
        }

        if !self.authoritative {
            return Err(RoadmapError::Invalid {
                path: path.to_path_buf(),
                message: "authoritative must be true for the .lux roadmap SSoT".to_string(),
            }
            .into());
        }

        for phase in &self.phases {
            if phase.name.trim().is_empty() {
                return Err(RoadmapError::Invalid {
                    path: path.to_path_buf(),
                    message: "phase name must not be empty".to_string(),
                }
                .into());
            }
            if phase
                .evidence_path
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(RoadmapError::Invalid {
                    path: path.to_path_buf(),
                    message: format!("phase {} has an empty evidence_path", phase.name),
                }
                .into());
            }
            if phase.status == RoadmapPhaseStatus::Complete && phase.evidence_path.is_none() {
                return Err(RoadmapError::Invalid {
                    path: path.to_path_buf(),
                    message: format!("completed phase {} is missing evidence_path", phase.name),
                }
                .into());
            }
            if phase.status == RoadmapPhaseStatus::Pushed {
                require_pushed_field(path, phase, "pushed_at", phase.pushed_at.as_deref())?;
                if let Some(pushed_at) = phase.pushed_at.as_deref() {
                    chrono::DateTime::parse_from_rfc3339(pushed_at).map_err(|_| {
                        RoadmapError::Invalid {
                            path: path.to_path_buf(),
                            message: format!(
                                "phase {} pushed_at is not a valid RFC3339 timestamp: {}",
                                phase.name, pushed_at
                            ),
                        }
                    })?;
                }
                require_pushed_field(path, phase, "push_git_sha", phase.push_git_sha.as_deref())?;
                require_pushed_field(
                    path,
                    phase,
                    "push_evidence_path",
                    phase.push_evidence_path.as_deref(),
                )?;
            }
        }

        Ok(())
    }
}

pub fn roadmap_template_for_engine(engine: EngineKind) -> RoadmapReality {
    RoadmapReality {
        phases: crate::lux_roadmap_registry::phases_for_engine(engine),
        capabilities: crate::lux_roadmap_registry::capabilities_for_engine(engine),
        ..RoadmapReality::default()
    }
}

fn require_pushed_field(
    path: &Path,
    phase: &RoadmapPhase,
    field_name: &str,
    value: Option<&str>,
) -> Result<()> {
    if value.is_none_or(|value| value.trim().is_empty()) {
        return Err(RoadmapError::Invalid {
            path: path.to_path_buf(),
            message: format!(
                "phase {} has status pushed but missing {field_name}",
                phase.name
            ),
        }
        .into());
    }
    Ok(())
}

pub fn load(path: &Path) -> Result<RoadmapReality> {
    let roadmap_path = roadmap_file_path(path);
    let content = match fs::read_to_string(&roadmap_path) {
        Ok(content) => content,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(RoadmapError::Missing { path: roadmap_path }.into());
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read {}", roadmap_path.display()));
        }
    };

    let roadmap: RoadmapReality =
        serde_json::from_str(&content).map_err(|error| RoadmapError::Corrupt {
            path: roadmap_path.clone(),
            message: error.to_string(),
        })?;
    roadmap.validate_at(&roadmap_path)?;
    Ok(roadmap)
}

pub fn init_or_load(path: &Path) -> Result<RoadmapReality> {
    let roadmap_path = roadmap_file_path(path);
    match load(&roadmap_path) {
        Ok(roadmap) => Ok(roadmap),
        Err(error) => {
            if error
                .downcast_ref::<RoadmapError>()
                .is_some_and(|roadmap_error| matches!(roadmap_error, RoadmapError::Missing { .. }))
            {
                let roadmap = RoadmapReality::default();
                roadmap.save(&roadmap_path)?;
                Ok(roadmap)
            } else {
                Err(error)
            }
        }
    }
}

pub fn save(path: &Path, roadmap: &RoadmapReality) -> Result<()> {
    roadmap.save(path)
}

pub fn roadmap_file_path(path: &Path) -> PathBuf {
    if path.file_name().is_some_and(|name| name == "roadmap.json") {
        return path.to_path_buf();
    }
    if path.file_name().is_some_and(|name| name == ".lux") {
        return path.join("roadmap.json");
    }
    path.join(".lux").join("roadmap.json")
}
