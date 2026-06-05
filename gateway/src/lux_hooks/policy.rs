use crate::lux_hooks::rules::ProjectSettingsReport;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyFinding {
    pub path: PathBuf,
    pub line: usize,
    pub marker: String,
    pub message: String,
    pub snippet: String,
}

pub fn scan_project(
    project_path: &Path,
    settings: &ProjectSettingsReport,
) -> Result<Vec<PolicyFinding>> {
    let mut findings = Vec::new();
    scan_directory(project_path, project_path, settings, &mut findings)?;
    Ok(findings)
}

fn scan_directory(
    project_path: &Path,
    directory: &Path,
    settings: &ProjectSettingsReport,
    findings: &mut Vec<PolicyFinding>,
) -> Result<()> {
    let entries = std::fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?;
    for entry in entries {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", directory.display()))?;
        let path = entry.path();
        if excluded_path(project_path, &path) {
            continue;
        }
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        if file_type.is_dir() {
            scan_directory(project_path, &path, settings, findings)?;
        } else if file_type.is_file() {
            scan_file(project_path, &path, settings, findings)?;
        }
    }
    Ok(())
}

fn scan_file(
    project_path: &Path,
    path: &Path,
    settings: &ProjectSettingsReport,
    findings: &mut Vec<PolicyFinding>,
) -> Result<()> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", path.display()))
        }
    };
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        for marker in &settings.forbidden_markers {
            if line.contains(marker) {
                findings.push(finding(
                    project_path,
                    path,
                    line_number,
                    marker,
                    "forbidden marker is not allowed",
                    line,
                ));
            }
        }
        for marker in &settings.allow_markers {
            if line.contains(marker) && !line_has_allow_evidence(line) {
                findings.push(finding(
                    project_path,
                    path,
                    line_number,
                    marker,
                    "allow marker requires same-line evidence, issue, or sunset",
                    line,
                ));
            }
        }
    }
    Ok(())
}

fn finding(
    project_path: &Path,
    path: &Path,
    line: usize,
    marker: &str,
    message: &str,
    snippet: &str,
) -> PolicyFinding {
    PolicyFinding {
        path: path
            .strip_prefix(project_path)
            .unwrap_or(path)
            .to_path_buf(),
        line,
        marker: marker.to_string(),
        message: message.to_string(),
        snippet: snippet.trim().chars().take(180).collect(),
    }
}

fn line_has_allow_evidence(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("evidence")
        || lower.contains("issue")
        || lower.contains("sunset")
        || lower.contains("removal")
}

fn excluded_path(project_path: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(project_path).unwrap_or(path);
    if relative == Path::new(".lux-agent.toml") {
        return true;
    }
    relative.components().any(|component| {
        let text = component.as_os_str().to_string_lossy();
        matches!(text.as_ref(), ".git" | ".lux" | "target" | "node_modules")
    })
}
