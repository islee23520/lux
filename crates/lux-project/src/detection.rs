use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub root: PathBuf,
    pub editor_version: String,
    pub project_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnityProjectDetection {
    pub root: PathBuf,
    pub project_name: String,
    pub editor_version: Option<String>,
    pub render_pipeline: Option<String>,
    pub scripting_backend: Option<String>,
    pub target_platforms: Vec<String>,
    pub packages: Vec<DetectedPackage>,
    pub test_framework_detected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedPackage {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GodotProjectDetection {
    pub project_root: PathBuf,
    pub godot_version: Option<String>,
    pub has_godot_dir: bool,
}

pub fn detect_from_cwd() -> Result<Option<ProjectInfo>> {
    let mut current = std::env::current_dir().context("failed to read current directory")?;
    loop {
        if let Some(info) = detect_from_path(&current)? {
            return Ok(Some(info));
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}

pub fn detect_from_path(path: &Path) -> Result<Option<ProjectInfo>> {
    let version_path = path.join("ProjectSettings").join("ProjectVersion.txt");
    if !version_path.is_file() {
        return Ok(None);
    }

    let editor_version = read_editor_version(&version_path)?;
    let project_name = read_project_name(path)?.unwrap_or_else(|| project_name_from_path(path));

    Ok(Some(ProjectInfo {
        root: path.to_path_buf(),
        editor_version,
        project_name,
    }))
}

pub fn detect_unity_project(project_path: &Path) -> Result<Option<UnityProjectDetection>> {
    let version_path = project_path
        .join("ProjectSettings")
        .join("ProjectVersion.txt");
    if !version_path.is_file() {
        return Ok(None);
    }

    let editor_version = match read_editor_version(&version_path) {
        Ok(version) => Some(version),
        Err(error) => {
            eprintln!("Failed to read Unity editor version for project detection: {error}");
            None
        }
    };
    let project_name =
        read_project_name(project_path)?.unwrap_or_else(|| project_name_from_path(project_path));
    let packages = read_detected_packages(project_path)?;
    let render_pipeline = infer_render_pipeline(&packages);
    let test_framework_detected = packages
        .iter()
        .any(|package| package.name == "com.unity.test-framework");

    Ok(Some(UnityProjectDetection {
        root: project_path.to_path_buf(),
        project_name,
        editor_version,
        render_pipeline,
        scripting_backend: None,
        target_platforms: Vec::new(),
        packages,
        test_framework_detected,
    }))
}

pub fn detect_godot_project(path: &Path) -> Option<GodotProjectDetection> {
    let project_godot_path = path.join("project.godot");
    if !project_godot_path.is_file() {
        return None;
    }

    let content = match fs::read_to_string(project_godot_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("Failed to read project.godot for Godot detection: {error}");
            return None;
        }
    };
    let config_version = content
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("config_version=").map(str::trim));

    if config_version != Some("5") {
        return None;
    }

    Some(GodotProjectDetection {
        project_root: path.to_path_buf(),
        godot_version: Some("4.x".to_string()),
        has_godot_dir: path.join(".godot").is_dir(),
    })
}

fn project_name_from_path(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn read_editor_version(version_path: &Path) -> Result<String> {
    let text = fs::read_to_string(version_path)
        .with_context(|| format!("failed to read {}", version_path.display()))?;
    text.lines()
        .find_map(|line| line.strip_prefix("m_EditorVersion:"))
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(ToOwned::to_owned)
        .with_context(|| format!("{} did not contain m_EditorVersion", version_path.display()))
}

fn read_project_name(root: &Path) -> Result<Option<String>> {
    let settings_path = root.join("ProjectSettings").join("ProjectSettings.asset");
    if !settings_path.is_file() {
        return Ok(None);
    }

    let text = fs::read_to_string(&settings_path)
        .with_context(|| format!("failed to read {}", settings_path.display()))?;
    Ok(text
        .lines()
        .find_map(|line| line.trim().strip_prefix("productName:"))
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned))
}

fn read_detected_packages(root: &Path) -> Result<Vec<DetectedPackage>> {
    let manifest_path = root.join("Packages").join("manifest.json");
    if !manifest_path.is_file() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", manifest_path.display()))?;

    let Some(dependencies) = manifest.get("dependencies").and_then(Value::as_object) else {
        return Ok(Vec::new());
    };

    let mut packages = dependencies
        .iter()
        .map(|(name, value)| DetectedPackage {
            name: name.clone(),
            version: value.as_str().map(ToOwned::to_owned),
        })
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(packages)
}

fn infer_render_pipeline(packages: &[DetectedPackage]) -> Option<String> {
    if packages
        .iter()
        .any(|package| package.name == "com.unity.render-pipelines.universal")
    {
        return Some("urp".to_string());
    }
    if packages
        .iter()
        .any(|package| package.name == "com.unity.render-pipelines.high-definition")
    {
        return Some("hdrp".to_string());
    }
    Some("built-in".to_string())
}

#[cfg(test)]
mod tests {
    use super::detect_godot_project;
    use tempfile::tempdir;

    #[test]
    fn detects_godot_4_project() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("project.godot"), "config_version=5\n").unwrap();
        std::fs::create_dir(dir.path().join(".godot")).unwrap();

        let detection = detect_godot_project(dir.path()).unwrap();

        assert_eq!(detection.godot_version, Some("4.x".to_string()));
        assert!(detection.has_godot_dir);
    }

    #[test]
    fn rejects_godot_3_project() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("project.godot"), "config_version=4\n").unwrap();

        assert!(detect_godot_project(dir.path()).is_none());
    }
}
