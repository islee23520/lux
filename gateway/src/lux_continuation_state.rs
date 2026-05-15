use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ContinuationStatus {
    #[default]
    Idle,
    Active,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationState {
    pub session_id: Option<String>,
    pub continuation_count: u32,
    pub stagnation_count: u32,
    pub consecutive_failures: u32,
    pub last_ambiguity: Option<String>,
    pub last_ticket_baseline: Option<String>,
    pub current_ticket_id: Option<String>,
    pub status: ContinuationStatus,
    pub started_at: Option<String>,
    pub updated_at: String,
    pub stop_reason: Option<String>,
}

impl ContinuationState {
    /// Load state from .lux/continuation-state.json or return defaults if missing
    pub fn load(project_path: &Path) -> Result<Self> {
        let path = Self::state_path(project_path);
        if !path.exists() {
            return Ok(Self::default_state());
        }

        let content = fs::read_to_string(&path).with_context(|| {
            format!("failed to read continuation state file {}", path.display())
        })?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse continuation state file {}", path.display()))
    }

    /// Save state to .lux/continuation-state.json
    pub fn save(&self, project_path: &Path) -> Result<()> {
        let path = Self::state_path(project_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create continuation state directory {}",
                    parent.display()
                )
            })?;
        }

        let content =
            serde_json::to_string_pretty(self).context("failed to serialize continuation state")?;
        fs::write(&path, content)
            .with_context(|| format!("failed to write continuation state file {}", path.display()))
    }

    /// Get the path to continuation-state.json
    fn state_path(project_path: &Path) -> PathBuf {
        project_path.join(".lux/continuation-state.json")
    }

    /// Return default/empty state
    pub fn default_state() -> Self {
        Self {
            session_id: None,
            continuation_count: 0,
            stagnation_count: 0,
            consecutive_failures: 0,
            last_ambiguity: None,
            last_ticket_baseline: None,
            current_ticket_id: None,
            status: ContinuationStatus::Idle,
            started_at: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
            stop_reason: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn make_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    #[test]
    fn test_load_missing_file() {
        let temp_dir = make_temp_dir();

        let state = ContinuationState::load(temp_dir.path()).expect("load should succeed");

        assert_eq!(state.status, ContinuationStatus::Idle);
        assert_eq!(state.continuation_count, 0);
        assert_eq!(state.stagnation_count, 0);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.session_id, None);
        assert_eq!(state.current_ticket_id, None);
        assert_eq!(state.stop_reason, None);
        assert_eq!(state.started_at, None);
    }

    #[test]
    fn test_save_reload_roundtrip() {
        let temp_dir = make_temp_dir();

        let original = ContinuationState {
            session_id: Some("sess-abc123".to_string()),
            continuation_count: 5,
            stagnation_count: 2,
            consecutive_failures: 1,
            last_ambiguity: Some("unclear requirement".to_string()),
            last_ticket_baseline: Some(r#"[{\"id\":\"t1\",\"status\":\"ToDo\"}]"#.to_string()),
            current_ticket_id: Some("ticket-001".to_string()),
            status: ContinuationStatus::Active,
            started_at: Some("2025-01-01T00:00:00Z".to_string()),
            updated_at: "2025-01-01T01:00:00Z".to_string(),
            stop_reason: None,
        };

        original.save(temp_dir.path()).expect("save should succeed");
        let reloaded = ContinuationState::load(temp_dir.path()).expect("load should succeed");

        assert_eq!(reloaded.session_id, original.session_id);
        assert_eq!(reloaded.continuation_count, original.continuation_count);
        assert_eq!(reloaded.stagnation_count, original.stagnation_count);
        assert_eq!(reloaded.consecutive_failures, original.consecutive_failures);
        assert_eq!(reloaded.last_ambiguity, original.last_ambiguity);
        assert_eq!(reloaded.last_ticket_baseline, original.last_ticket_baseline);
        assert_eq!(reloaded.current_ticket_id, original.current_ticket_id);
        assert_eq!(reloaded.status, original.status);
        assert_eq!(reloaded.started_at, original.started_at);
        assert_eq!(reloaded.updated_at, original.updated_at);
        assert_eq!(reloaded.stop_reason, original.stop_reason);
    }

    #[test]
    fn test_default_state_values() {
        let state = ContinuationState::default_state();

        assert_eq!(state.status, ContinuationStatus::Idle);
        assert_eq!(state.continuation_count, 0);
        assert_eq!(state.stagnation_count, 0);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.session_id, None);
        assert_eq!(state.current_ticket_id, None);
        assert_eq!(state.stop_reason, None);
        assert_eq!(state.started_at, None);

        let updated_at = chrono::DateTime::parse_from_rfc3339(&state.updated_at)
            .expect("updated_at should be RFC3339");
        let now = chrono::Utc::now();
        let delta = now.signed_duration_since(updated_at.with_timezone(&chrono::Utc));
        assert!(
            delta.num_seconds().abs() <= 10,
            "updated_at should be recent"
        );
    }

    #[test]
    fn test_state_path() {
        let path = ContinuationState::state_path(Path::new("/tmp/myproject"));

        assert_eq!(
            path,
            Path::new("/tmp/myproject/.lux/continuation-state.json")
        );
    }
}
