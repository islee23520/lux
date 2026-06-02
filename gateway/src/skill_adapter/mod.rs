//! Skill adaptation: public facade.
//!
//! Re-exports all public types and functions from the sub-modules so the
//! external API surface (`lux::skill_adapter::*`) is unchanged.

pub mod adaptation;
pub mod compatibility;
pub mod discovery;
pub mod metadata;
pub mod slimming;

// Re-export public types
pub use adaptation::{AdaptationDecision, CompatibilityReport};
pub use compatibility::CompatibilityResult;
pub use metadata::{AdaptationProjectMetadata, SkillContextSlimRules};

// Re-export public functions
pub use adaptation::{build_adaptation_decision, read_adaptation_file, write_adaptation_file};
pub use compatibility::check_skill_compatibility;
pub use slimming::slim_context_for_adaptation;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_log::AiLogEntry;
    use crate::project::ProjectInfo;
    use crate::skill_adapter::compatibility::version_satisfies;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir_with(prefix: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lux-skill-adapter-{prefix}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_project_info(editor_version: &str) -> ProjectInfo {
        let root = temp_dir_with("project-info");
        let settings = root.join("ProjectSettings");
        fs::create_dir_all(&settings).unwrap();
        fs::write(
            settings.join("ProjectVersion.txt"),
            format!("m_EditorVersion: {editor_version}\n"),
        )
        .unwrap();
        fs::write(
            settings.join("ProjectSettings.asset"),
            "productName: TestProject\n",
        )
        .unwrap();
        ProjectInfo {
            root: root.clone(),
            editor_version: editor_version.to_string(),
            project_name: "TestProject".to_string(),
        }
    }

    #[test]
    fn compatibility_passes_when_no_constraints() {
        let info = make_project_info("2025.3.0f1");
        let manifest = json!({"name": "test-skill", "version": "1.0.0"});
        let result = check_skill_compatibility(&manifest, &info);
        assert!(result.compatible);
        assert!(result.reasons.is_empty());
    }

    #[test]
    fn compatibility_fails_on_unmet_min_editor_version() {
        let info = make_project_info("2025.3.0f1");
        let manifest = json!({
            "name": "test-skill",
            "minEditorVersion": "2026.1.0"
        });
        let result = check_skill_compatibility(&manifest, &info);
        assert!(!result.compatible);
        assert!(result
            .reasons
            .iter()
            .any(|r| r.contains("does not satisfy minimum")));
    }

    #[test]
    fn compatibility_passes_on_met_min_editor_version() {
        let info = make_project_info("2026.2.0f1");
        let manifest = json!({
            "name": "test-skill",
            "minEditorVersion": "2025.3.0"
        });
        let result = check_skill_compatibility(&manifest, &info);
        assert!(result.compatible);
    }

    #[test]
    fn compatibility_fails_on_missing_required_feature() {
        let info = make_project_info("2025.3.0f1");
        let manifest = json!({
            "name": "test-skill",
            "requiredFeatures": ["nonexistent-feature-x"]
        });
        let result = check_skill_compatibility(&manifest, &info);
        assert!(!result.compatible);
        assert!(result
            .reasons
            .iter()
            .any(|r| r.contains("nonexistent-feature-x")));
    }

    #[test]
    fn compatibility_checks_render_pipeline_constraint() {
        let info = make_project_info("2025.3.0f1");
        let manifest = json!({
            "name": "test-skill",
            "compatibleRenderPipelines": ["hdrp"]
        });
        let result = check_skill_compatibility(&manifest, &info);
        assert!(!result.compatible);
        assert!(result.reasons.iter().any(|r| r.contains("render pipeline")));
    }

    #[test]
    fn compatibility_wildcard_pipeline_accepts_any() {
        let info = make_project_info("2025.3.0f1");
        let manifest = json!({
            "name": "test-skill",
            "compatibleRenderPipelines": ["*"]
        });
        let result = check_skill_compatibility(&manifest, &info);
        assert!(result.compatible);
    }

    #[test]
    fn slim_context_produces_compact_output() {
        let entries = vec![
            AiLogEntry {
                line_number: 1,
                timestamp: "2026-05-10T00:00:00Z".to_string(),
                value: json!({
                    "actor": "codex",
                    "category": "tool",
                    "summary": "compiled successfully"
                }),
            },
            AiLogEntry {
                line_number: 2,
                timestamp: "2026-05-10T00:00:01Z".to_string(),
                value: json!({
                    "actor": "opencode",
                    "category": "ai-action-log",
                    "message": "appended entry"
                }),
            },
        ];

        let result = slim_context_for_adaptation(&entries, 4096);
        assert_eq!(result["totalEntries"], 2);
        assert_eq!(result["slimmedEntries"], 2);
        assert_eq!(result["droppedEntries"], 0);
        assert!(result["entries"].is_array());
        let items = result["entries"].as_array().unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn slim_context_truncates_to_token_budget() {
        let mut entries = Vec::new();
        for i in 0..100 {
            entries.push(AiLogEntry {
                line_number: i + 1,
                timestamp: format!("2026-05-10T00:00:{i:02}Z"),
                value: json!({
                    "actor": "agent",
                    "summary": format!("entry number {i}")
                }),
            });
        }

        let result = slim_context_for_adaptation(&entries, 100);
        assert_eq!(result["totalEntries"], 100);
        assert!(result["slimmedEntries"].as_u64().unwrap_or(0) < 100);
        assert!(result["droppedEntries"].as_u64().unwrap_or(0) > 0);
    }

    #[test]
    fn slim_context_handles_empty_entries() {
        let result = slim_context_for_adaptation(&[], 4096);
        assert_eq!(result["totalEntries"], 0);
        assert_eq!(result["slimmedEntries"], 0);
        assert!(result["entries"].as_array().unwrap().is_empty());
    }

    #[test]
    fn build_adaptation_decision_validates_source_structure() {
        let project_root = temp_dir_with("adapt-project");
        fs::create_dir_all(project_root.join("Assets")).unwrap();
        fs::create_dir_all(project_root.join("ProjectSettings")).unwrap();
        fs::write(
            project_root
                .join("ProjectSettings")
                .join("ProjectVersion.txt"),
            "m_EditorVersion: 2025.3.0f1\n",
        )
        .unwrap();

        let source_dir = temp_dir_with("adapt-source");
        fs::write(
            source_dir.join("manifest.json"),
            "{\"name\":\"my-skill\",\"version\":\"1.0\"}\n",
        )
        .unwrap();
        fs::write(source_dir.join("SKILL.md"), "# My Skill\n\nTest.\n").unwrap();

        let decision =
            build_adaptation_decision("my-skill", source_dir.to_str().unwrap(), &project_root);
        assert!(decision.is_ok());
        let d = decision.unwrap();
        assert_eq!(d.skill_name, "my-skill");
        assert_eq!(d.schema_version, 1);
        assert_eq!(d.protocol, "lux.skill.adaptation.v1");

        fs::remove_dir_all(project_root).ok();
        fs::remove_dir_all(source_dir).ok();
    }

    #[test]
    fn build_adaptation_decision_rejects_missing_manifest() {
        let project_root = temp_dir_with("adapt-bad-proj");
        fs::create_dir_all(project_root.join("Assets")).unwrap();
        fs::create_dir_all(project_root.join("ProjectSettings")).unwrap();

        let source_dir = temp_dir_with("adapt-bad-source");
        fs::write(source_dir.join("SKILL.md"), "# No manifest\n").unwrap();

        let result = build_adaptation_decision("bad", source_dir.to_str().unwrap(), &project_root);
        assert!(result.is_err());

        fs::remove_dir_all(project_root).ok();
        fs::remove_dir_all(source_dir).ok();
    }

    #[test]
    fn write_and_read_adaptation_file_roundtrips() {
        let skill_dir = temp_dir_with("roundtrip-skill");
        let decision = AdaptationDecision {
            schema_version: 1,
            protocol: "lux.skill.adaptation.v1",
            skill_name: "test-roundtrip".to_string(),
            source: "/tmp/source".to_string(),
            project_root: PathBuf::from("/tmp/project"),
            checks: vec![json!({"name": "check1", "ok": true, "message": "ok"})],
            warnings: vec![],
            project_metadata: json!({}),
            compatibility: CompatibilityReport {
                compatible: true,
                reasons: vec![],
            },
            context_slimming: json!({"totalReferences": 0}),
        };

        let path = write_adaptation_file(&skill_dir, &decision).unwrap();
        assert!(path.exists());

        let read_back = read_adaptation_file(&skill_dir);
        assert!(read_back.is_some());
        let value = read_back.unwrap();
        assert_eq!(value["skill_name"], "test-roundtrip");
        assert_eq!(value["compatibility"]["compatible"], true);

        fs::remove_dir_all(skill_dir).ok();
    }

    #[test]
    fn read_adaptation_file_returns_none_when_missing() {
        let dir = temp_dir_with("missing-adapt");
        assert!(read_adaptation_file(&dir).is_none());
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn version_satisfies_exact_match() {
        assert!(version_satisfies("2025.3.0f1", "2025.3.0f1"));
    }

    #[test]
    fn version_satisfies_newer_is_ok() {
        assert!(version_satisfies("2026.1.0f1", "2025.3.0"));
    }

    #[test]
    fn version_satisfies_older_rejects() {
        assert!(!version_satisfies("2024.3.0f1", "2025.3.0"));
    }
}
