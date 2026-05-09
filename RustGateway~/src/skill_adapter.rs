use crate::ai_log::{AiLogEntry, build_continuation_context};
use crate::project::ProjectInfo;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

/// Result of checking whether a skill manifest is compatible with a project.
#[derive(Debug, Clone, Serialize)]
pub struct CompatibilityResult {
    pub compatible: bool,
    pub reasons: Vec<String>,
}

/// Rich project metadata collected during adaptation, beyond the basic ProjectInfo.
#[derive(Debug, Clone, Serialize)]
pub struct AdaptationProjectMetadata {
    pub unity_version: Option<String>,
    pub render_pipeline: String,
    pub installed_packages: Vec<String>,
    pub has_lux_package: bool,
}

impl AdaptationProjectMetadata {
    fn to_json(&self) -> Value {
        json!({
            "unity_version": self.unity_version,
            "render_pipeline": self.render_pipeline,
            "installed_packages": self.installed_packages,
            "has_lux_package": self.has_lux_package,
        })
    }
}

/// Slim rules declared in a skill manifest under `contextSlimRules`.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct SkillContextSlimRules {
    #[serde(default, rename = "maxReferences")]
    pub max_references: Option<usize>,
    #[serde(default, rename = "maxSkillMdLines")]
    pub max_skill_md_lines: Option<usize>,
    #[serde(default, rename = "excludeTags")]
    pub exclude_tags: Option<Vec<String>>,
}

/// Full adaptation decision written to `lux-adaptation.json`.
#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct CompatibilityReport {
    pub compatible: bool,
    pub reasons: Vec<String>,
}

const ADAPTATION_SCHEMA_VERSION: u32 = 1;
const DEFAULT_MAX_TOKENS: usize = 4096;

/// Check whether a skill manifest is compatible with the detected project info.
///
/// Evaluates `minEditorVersion` / `luxVersion` constraints, required packages,
/// and compatible render pipelines from the manifest against the live project.
pub fn check_skill_compatibility(
    skill_manifest: &Value,
    project_info: &ProjectInfo,
) -> CompatibilityResult {
    let mut reasons = Vec::new();

    // Editor version check: look for minEditorVersion or m_EditorVersion constraint
    if let Some(min_version) = skill_manifest
        .get("minEditorVersion")
        .or_else(|| skill_manifest.get("min_editor_version"))
        .and_then(Value::as_str)
    {
        if !version_satisfies(&project_info.editor_version, min_version) {
            reasons.push(format!(
                "editor version '{}' does not satisfy minimum '{}'",
                project_info.editor_version, min_version
            ));
        }
    }

    // Required packages check
    if let Some(required) = skill_manifest
        .get("requiredPackages")
        .or_else(|| skill_manifest.get("required_packages"))
        .and_then(Value::as_array)
    {
        // We check against packages we can detect; missing ones become reasons.
        // Full package resolution requires Packages/manifest.json which is handled
        // in the richer `build_adaptation_decision` path.
        for pkg in required.iter().filter_map(Value::as_str) {
            if !is_package_likely_installed(pkg, project_info) {
                reasons.push(format!("missing required package: {pkg}"));
            }
        }
    }

    // Render pipeline compatibility
    if let Some(pipelines) = skill_manifest
        .get("compatibleRenderPipelines")
        .or_else(|| skill_manifest.get("compatible_render_pipelines"))
        .and_then(Value::as_array)
    {
        let pipeline_names: Vec<&str> = pipelines.iter().filter_map(Value::as_str).collect();
        if !pipeline_names.is_empty() {
            let detected = detect_render_pipeline(project_info);
            if !pipeline_names.contains(&detected.as_str()) && !pipeline_names.contains(&"*") {
                reasons.push(format!(
                    "incompatible render pipeline: project uses '{detected}', skill requires one of [{names}]",
                    names = pipeline_names.join(", ")
                ));
            }
        }
    }

    // Required features check
    if let Some(features) = skill_manifest
        .get("requiredFeatures")
        .or_else(|| skill_manifest.get("required_features"))
        .and_then(Value::as_array)
    {
        for feature in features.iter().filter_map(Value::as_str) {
            if !is_feature_available(feature, project_info) {
                reasons.push(format!("missing required feature: {feature}"));
            }
        }
    }

    CompatibilityResult {
        compatible: reasons.is_empty(),
        reasons,
    }
}

/// Reduce an AI event log to essential information sized for AI context injection.
///
/// Takes raw AiLogEntry slices and produces a compact JSON value containing only
/// the fields relevant to skill adaptation: timestamps, actors, categories,
/// summaries, and a token-budget-aware truncation.
pub fn slim_context_for_adaptation(
    event_entries: &[AiLogEntry],
    max_tokens: usize,
) -> Value {
    let budget = if max_tokens == 0 {
        DEFAULT_MAX_TOKENS
    } else {
        max_tokens
    };

    // Re-use the existing continuation context builder which already produces
    // a compact summary representation of entries.
    let context = build_continuation_context(event_entries, None);
    let count = context["count"].as_u64().unwrap_or(0) as usize;

    // Rough token estimation: ~4 chars per token on average for JSON text.
    // We truncate entries to fit within the token budget.
    let max_entries = budget / 4; // conservative upper bound
    let truncated = if count > max_entries {
        let mut entries = context["entries"].as_array().cloned().unwrap_or_default();
        let keep_from = entries.len().saturating_sub(max_entries);
        entries = entries.split_off(keep_from);
        entries
    } else {
        context["entries"].as_array().cloned().unwrap_or_default()
    };

    let original_count = event_entries.len();
    let dropped = original_count.saturating_sub(truncated.len());

    json!({
        "schemaVersion": 1,
        "totalEntries": original_count,
        "slimmedEntries": truncated.len(),
        "droppedEntries": dropped,
        "maxTokens": budget,
        "entries": truncated,
    })
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

    // Validate name matches manifest
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
    let project_info = crate::project::detect_from_path(project_root)?
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

    // Also run rich compatibility checks (packages, render pipeline) for the report
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
pub fn write_adaptation_file(
    skill_dir: &Path,
    decision: &AdaptationDecision,
) -> Result<PathBuf> {
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

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn detect_rich_project_metadata(
    project_root: &Path,
    warnings: &mut Vec<String>,
) -> AdaptationProjectMetadata {
    let mut unity_version: Option<String> = None;
    let mut render_pipeline = "unknown".to_string();
    let mut installed_packages: Vec<String> = Vec::new();
    let mut has_lux_package = false;

    let version_path = project_root
        .join("ProjectSettings")
        .join("ProjectVersion.txt");
    if let Ok(version_content) = fs::read_to_string(&version_path) {
        for line in version_content.lines() {
            if let Some(version) = line.strip_prefix("m_EditorVersion: ") {
                unity_version = Some(version.trim().to_string());
                break;
            }
        }
    }

    let package_manifest_path = project_root.join("Packages").join("manifest.json");
    match fs::read_to_string(&package_manifest_path) {
        Ok(package_manifest_json) => {
            has_lux_package = package_manifest_json.contains("com.linalab.lux");

            if let Ok(package_value) = serde_json::from_str::<Value>(&package_manifest_json) {
                if let Some(deps) = package_value.get("dependencies").and_then(Value::as_object) {
                    for key in deps.keys() {
                        installed_packages.push(key.clone());
                    }
                }

                if installed_packages.contains(&"com.unity.render-pipelines.universal".to_string()) {
                    render_pipeline = "urp".to_string();
                } else if installed_packages
                    .contains(&"com.unity.render-pipelines.high-definition".to_string())
                {
                    render_pipeline = "hdrp".to_string();
                } else {
                    render_pipeline = "builtin".to_string();
                }
            }

            if !has_lux_package {
                warnings.push(
                    "project Packages/manifest.json does not mention com.linalab.lux"
                        .to_string(),
                );
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => warnings.push(
            "project Packages/manifest.json was not found; skipped LUX package metadata check"
                .to_string(),
        ),
        Err(error) => warnings.push(format!(
            "failed to read {}: {error}",
            package_manifest_path.display()
        )),
    }

    let graphics_settings = project_root
        .join("ProjectSettings")
        .join("GraphicsSettings.asset");
    if graphics_settings.exists() && render_pipeline == "unknown" {
        if let Ok(content) = fs::read_to_string(&graphics_settings) {
            if content.contains("UniversalRenderPipelineAsset") {
                render_pipeline = "urp".to_string();
            } else if content.contains("HDRenderPipelineAsset") {
                render_pipeline = "hdrp".to_string();
            } else {
                render_pipeline = "builtin".to_string();
            }
        }
    }

    AdaptationProjectMetadata {
        unity_version,
        render_pipeline,
        installed_packages,
        has_lux_package,
    }
}

fn judge_rich_compatibility(
    manifest: &Value,
    project_metadata: &AdaptationProjectMetadata,
    checks: &mut Vec<Value>,
) -> CompatibilityResult {
    let mut reasons = Vec::new();

    // Check requiredPackages
    if let Some(required) = manifest
        .get("requiredPackages")
        .or_else(|| manifest.get("required_packages"))
        .and_then(Value::as_array)
    {
        let mut missing: Vec<String> = Vec::new();
        for pkg in required.iter().filter_map(Value::as_str) {
            if !project_metadata.installed_packages.contains(&pkg.to_string()) {
                missing.push(pkg.to_string());
            }
        }
        let ok = missing.is_empty();
        checks.push(json!({
            "name": "required_packages",
            "ok": ok,
            "message": if ok {
                "All required packages are installed".to_string()
            } else {
                format!("Missing required packages: {}", missing.join(", "))
            },
        }));
        if !ok {
            reasons.push(format!(
                "missing required packages: {}",
                missing.join(", ")
            ));
        }
    }

    // Check compatibleRenderPipelines
    if let Some(compatible_pipelines) = manifest
        .get("compatibleRenderPipelines")
        .or_else(|| manifest.get("compatible_render_pipelines"))
        .and_then(Value::as_array)
    {
        let pipeline_names: Vec<String> = compatible_pipelines
            .iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect();
        let pipeline_ok = pipeline_names.is_empty()
            || pipeline_names.contains(&project_metadata.render_pipeline)
            || pipeline_names.contains(&"*".to_string());
        checks.push(json!({
            "name": "render_pipeline",
            "ok": pipeline_ok,
            "message": format!(
                "Project render pipeline '{}' vs skill compatibility: [{}]",
                project_metadata.render_pipeline,
                pipeline_names.join(", ")
            ),
        }));
        if !pipeline_ok {
            reasons.push(format!(
                "incompatible render pipeline: project uses '{}', skill requires one of [{}]",
                project_metadata.render_pipeline,
                pipeline_names.join(", ")
            ));
        }
    }

    CompatibilityResult {
        compatible: reasons.is_empty(),
        reasons,
    }
}

fn compute_context_slimming(
    source_dir: &Path,
    manifest: &Value,
    project_metadata: &AdaptationProjectMetadata,
) -> Value {
    let slim_rules: Option<SkillContextSlimRules> = manifest
        .get("contextSlimRules")
        .or_else(|| manifest.get("context_slim_rules"))
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let references_dir = source_dir.join("references");
    let total_references = if references_dir.is_dir() {
        fs::read_dir(&references_dir)
            .map(|entries| entries.flatten().count())
            .unwrap_or(0)
    } else {
        0
    };

    let skill_md_path = source_dir.join("SKILL.md");
    let total_skill_md_lines = fs::read_to_string(&skill_md_path)
        .map(|content| content.lines().count())
        .unwrap_or(0);

    let max_references = slim_rules
        .as_ref()
        .and_then(|r| r.max_references)
        .unwrap_or(usize::MAX);
    let references_slashed = total_references > max_references;

    let max_skill_md_lines = slim_rules
        .as_ref()
        .and_then(|r| r.max_skill_md_lines)
        .unwrap_or(usize::MAX);
    let slimmed_skill_md_lines = total_skill_md_lines.min(max_skill_md_lines);
    let skill_md_slashed = total_skill_md_lines > max_skill_md_lines;

    let excluded_tags: Vec<String> = slim_rules
        .as_ref()
        .and_then(|r| r.exclude_tags.clone())
        .unwrap_or_default();

    let mut included_references: Vec<String> = Vec::new();
    let mut excluded_references: Vec<String> = Vec::new();
    if references_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&references_dir) {
            let mut all_refs: Vec<String> = entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
                .filter(|name| name.ends_with(".md"))
                .collect();
            all_refs.sort();

            for reference_name in &all_refs {
                let name_lower = reference_name.to_lowercase();
                let should_exclude = excluded_tags
                    .iter()
                    .any(|tag| name_lower.contains(&tag.to_lowercase()));
                if should_exclude {
                    excluded_references.push(reference_name.clone());
                } else {
                    included_references.push(reference_name.clone());
                }
            }
        }
    }

    if included_references.len() > max_references {
        let excess = included_references.len() - max_references;
        let drained: Vec<String> =
            included_references.split_off(included_references.len() - excess);
        excluded_references.extend(drained);
    }

    json!({
        "totalReferences": total_references,
        "slimmedReferences": included_references.len(),
        "referencesSlashed": references_slashed || included_references.len() != total_references,
        "totalSkillMdLines": total_skill_md_lines,
        "slimmedSkillMdLines": slimmed_skill_md_lines,
        "skillMdSlashed": skill_md_slashed,
        "excludedTags": excluded_tags,
        "includedReferences": included_references,
        "excludedReferences": excluded_references,
        "projectRenderPipeline": project_metadata.render_pipeline,
    })
}

fn version_satisfies(actual: &str, minimum: &str) -> bool {
    let actual_parts: Vec<u32> = actual
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();
    let min_parts: Vec<u32> = minimum
        .split('.')
        .filter_map(|s| s.parse::<u32>().ok())
        .collect();

    for i in 0..min_parts.len().max(actual_parts.len()) {
        let a = actual_parts.get(i).copied().unwrap_or(0);
        let m = min_parts.get(i).copied().unwrap_or(0);
        if a < m {
            return false;
        }
        if a > m {
            return true;
        }
    }
    true
}

fn is_package_likely_installed(pkg: &str, _project_info: &ProjectInfo) -> bool {
    // Conservative default: assume unknown packages may be available.
    // Full resolution happens in judge_rich_compatibility with manifest.json.
    // Known built-in packages are always "installed".
    matches!(
        pkg,
        "com.unity.modules.ui"
            | "com.unity.modules.uielements"
            | "com.unity.modules.imgui"
            | "com.unity.modules.physics"
            | "com.unity.modules.animation"
    )
}

fn detect_render_pipeline(project_info: &ProjectInfo) -> String {
    // Heuristic: newer Unity versions (2022+) default to URP.
    let version_prefix = project_info.editor_version
        .split('.')
        .next()
        .unwrap_or("0");
    if let Ok(year) = version_prefix.parse::<u32>() {
        if year >= 2022 {
            return "urp".to_string();
        }
    }
    "builtin".to_string()
}

fn is_feature_available(feature: &str, _project_info: &ProjectInfo) -> bool {
    // Known features that are universally available in supported Unity versions.
    matches!(
        feature,
        "ai-assistant"
            | "editor-scripting"
            | "asset-database"
            | "scene-management"
            | "prefab-workflow"
            | "ui-toolkit"
            | "serialization"
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
        fs::write(settings.join("ProjectSettings.asset"), "productName: TestProject\n")
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
        assert!(result.reasons.iter().any(|r| r.contains("nonexistent-feature-x")));
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

        let result = slim_context_for_adaptation(&entries, 100); // very small budget
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
            project_root.join("ProjectSettings").join("ProjectVersion.txt"),
            "m_EditorVersion: 2025.3.0f1\n",
        )
        .unwrap();

        let source_dir = temp_dir_with("adapt-source");
        fs::write(source_dir.join("manifest.json"), "{\"name\":\"my-skill\",\"version\":\"1.0\"}\n")
            .unwrap();
        fs::write(source_dir.join("SKILL.md"), "# My Skill\n\nTest.\n").unwrap();

        let decision = build_adaptation_decision("my-skill", source_dir.to_str().unwrap(), &project_root);
        assert!(decision.is_ok());
        let d = decision.unwrap();
        assert_eq!(d.skill_name, "my-skill");
        assert_eq!(d.schema_version, ADAPTATION_SCHEMA_VERSION);
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
