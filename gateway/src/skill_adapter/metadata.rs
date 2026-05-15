//! Project metadata types and detection helpers.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

/// Rich project metadata collected during adaptation, beyond the basic ProjectInfo.
#[derive(Debug, Clone, Serialize)]
pub struct AdaptationProjectMetadata {
    pub unity_version: Option<String>,
    pub render_pipeline: String,
    pub installed_packages: Vec<String>,
    pub has_lux_package: bool,
}

impl AdaptationProjectMetadata {
    pub(crate) fn to_json(&self) -> Value {
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

/// Detect rich project metadata from a Unity project root.
///
/// Reads ProjectVersion.txt for editor version, Packages/manifest.json for
/// installed packages and render pipeline inference, and GraphicsSettings.asset
/// as a fallback pipeline detector.
pub(crate) fn detect_rich_project_metadata(
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

                if installed_packages.contains(&"com.unity.render-pipelines.universal".to_string())
                {
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
                    "project Packages/manifest.json does not mention com.linalab.lux".to_string(),
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
