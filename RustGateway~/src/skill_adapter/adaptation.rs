//! Adaptation decision building, serialization, and I/O.

use anyhow::{Context, Result, bail};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::project::detect_from_path;
use crate::skill_adapter::compatibility::{
    check_skill_compatibility, judge_rich_compatibility,
};
use crate::skill_adapter::metadata::detect_rich_project_metadata;
use crate::skill_adapter::slimming::compute_context_slimming;

const ADAPTATION_SCHEMA_VERSION: u32 = 1;

/// Full adaptation decision written to `lux-adaptation.json`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AdaptationDecision {
    pub schema_version: u32,
    pub protocol: &'static str,
    pub skill_name: String,
    pub source: String,
    pub project_root: PathBuf,
    pub checks: Vec<Value>,
    pub warnings: Vec<String>,
    pub project_metadata: Value,
    pub compatibility: CompatibilityReport,
    pub context_slimming: Value,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompatibilityReport {
    pub compatible: bool,
    pub reasons: Vec<String>,
}

/// Build a full adaptation decision for a skill being installed into a project.
///
/// This orchestrates all four phases:
/// 1. Structural validation checks
/// 2. Project metadata detection via ProjectInfo + rich metadata
/// 3. Compatibility judgment via check_skill_compatibility
/// 4. Context slimming computation
///
/// Returns an `AdaptationDecision` ready to serialize as `lux-adaptation.json`.
pub fn build_adaptation_decision(
    skill_name: &str,
    source: &str,
    project_root: &Path,
) -> Result<AdaptationDecision> {
    let source_dir = Path::new(source);
    if !source_dir.is_dir() {
        bail!("source is not a directory: {}", source_dir.display());
    }

    let manifest_path = source_dir.join("manifest.json");
    let skill_md_path = source_dir.join("SKILL.md");
    if !manifest_path.is_file() {
        bail!("source skill is missing manifest.json");
    }
    if !skill_md_path.is_file() {
        bail!("source skill is missing SKILL.md");
    }

    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest_value: Value = serde_json::from_str(&manifest_text)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

    if let Some(name) = manifest_value.get("name").and_then(Value::as_str) {
        if name != skill_name {
            bail!(
                "source manifest name '{}' does not match requested skill '{}'",
                name,
                skill_name
            );
        }
    }

    // Phase 1: Structural checks
    let mut checks = Vec::new();
    let assets_dir = project_root.join("Assets");
    let project_settings_dir = project_root.join("ProjectSettings");
    let unity_project_ok = assets_dir.is_dir() && project_settings_dir.is_dir();
    checks.push(json!({
        "name": "unity_project",
        "ok": unity_project_ok,
        "message": "Project root contains Assets/ and ProjectSettings/",
    }));
    checks.push(json!({
        "name": "source_manifest",
        "ok": true,
        "message": "Source contains manifest.json",
    }));
    checks.push(json!({
        "name": "source_skill_md",
        "ok": true,
        "message": "Source contains SKILL.md",
    }));

    if !unity_project_ok {
        bail!("adaptation requires a Unity project root containing Assets/ and ProjectSettings/");
    }

    // Phase 2: Project metadata detection
    let mut warnings = Vec::new();
    let project_info = detect_from_path(project_root)?
        .context("failed to detect project metadata from path")?;
    let rich_metadata = detect_rich_project_metadata(project_root, &mut warnings);

    if manifest_value.get("luxVersion").is_none() && manifest_value.get("lux_version").is_none() {
        warnings.push("source manifest does not declare luxVersion".to_string());
    }

    // Phase 3: Compatibility judgment
    let compatibility = check_skill_compatibility(&manifest_value, &project_info);
    checks.push(json!({
        "name": "compatibility",
        "ok": compatibility.compatible,
        "message": if compatibility.compatible {
            "Skill is compatible with this project".to_string()
        } else {
            format!("Incompatible: {}", compatibility.reasons.join("; "))
        },
    }));

    let rich_compat = judge_rich_compatibility(&manifest_value, &rich_metadata, &mut checks);

    let final_compatible = compatibility.compatible && rich_compat.compatible;
    let mut all_reasons = compatibility.reasons.clone();
    all_reasons.extend(rich_compat.reasons.clone());

    // Phase 4: Context slimming
    let context_slimming = compute_context_slimming(source_dir, &manifest_value, &rich_metadata);

    Ok(AdaptationDecision {
        schema_version: ADAPTATION_SCHEMA_VERSION,
        protocol: "lux.skill.adaptation.v1",
        skill_name: skill_name.to_string(),
        source: source.to_string(),
        project_root: project_root.to_path_buf(),
        checks,
        warnings,
        project_metadata: rich_metadata.to_json(),
        compatibility: CompatibilityReport {
            compatible: final_compatible,
            reasons: all_reasons,
        },
        context_slimming,
    })
}

/// Write an adaptation decision to `lux-adaptation.json` inside the target skill directory.
pub fn write_adaptation_file(skill_dir: &Path, decision: &AdaptationDecision) -> Result<PathBuf> {
    let adaptation_path = skill_dir.join("lux-adaptation.json");
    let text = serde_json::to_string_pretty(decision)
        .context("failed to serialize adaptation decision")?;
    fs::write(&adaptation_path, text).with_context(|| {
        format!(
            "failed to write adaptation file {}",
            adaptation_path.display()
        )
    })?;
    Ok(adaptation_path)
}

/// Read an existing `lux-adaptation.json` from a skill directory, if present.
pub fn read_adaptation_file(skill_dir: &Path) -> Option<Value> {
    let adaptation_path = skill_dir.join("lux-adaptation.json");
    let content = fs::read_to_string(&adaptation_path).ok()?;
    match serde_json::from_str::<Value>(&content) {
        Ok(value) => Some(value),
        Err(error) => {
            eprintln!(
                "Warning: failed to parse adaptation metadata {}: {error}",
                adaptation_path.display()
            );
            None
        }
    }
}
