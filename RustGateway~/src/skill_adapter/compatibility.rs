//! Structural validation, package/feature detection, render pipeline heuristics.

use crate::project::ProjectInfo;
use crate::skill_adapter::metadata::AdaptationProjectMetadata;
use serde_json::{json, Value};

/// Result of checking whether a skill manifest is compatible with a project.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompatibilityResult {
    pub compatible: bool,
    pub reasons: Vec<String>,
}

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

/// Rich compatibility judgment using `AdaptationProjectMetadata`.
///
/// Checks requiredPackages against actually-installed packages and
/// compatibleRenderPipelines against the detected pipeline. Appends
/// structured check entries to *checks*.
pub(crate) fn judge_rich_compatibility(
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

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

pub(crate) fn version_satisfies(actual: &str, minimum: &str) -> bool {
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
    let version_prefix = project_info
        .editor_version
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
