use anyhow::{Context, Result};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct ProjectInfo {
    pub root: PathBuf,
    pub editor_version: String,
    pub project_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectedPackage {
    pub name: String,
    pub version: Option<String>,
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
    let project_name = read_project_name(path)?.unwrap_or_else(|| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string())
    });

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

    let editor_version = read_editor_version(&version_path).ok();
    let project_name = read_project_name(project_path)?.unwrap_or_else(|| {
        project_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path.display().to_string())
    });
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
