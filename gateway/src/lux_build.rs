use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildTarget {
    WebGL,
}

impl BuildTarget {
    pub fn as_unity_arg(&self) -> &'static str {
        match self {
            Self::WebGL => "WebGL",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildStatus {
    Queued,
    Running,
    Succeeded,
    Failed(String),
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuildJob {
    pub build_id: String,
    pub project_path: PathBuf,
    pub target: BuildTarget,
    pub status: BuildStatus,
    pub progress: f64,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub artifact_path: Option<PathBuf>,
    pub log: Vec<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BuildManager {
    pub jobs: HashMap<String, BuildJob>,
    pub max_concurrent: usize,
    pub base_output_dir: PathBuf,
}

impl BuildManager {
    pub fn new(base_output_dir: PathBuf) -> Self {
        Self {
            jobs: HashMap::new(),
            max_concurrent: 1,
            base_output_dir,
        }
    }

    pub fn with_project_root(project_root: Option<&Path>) -> Self {
        let base_output_dir = project_root
            .map(|root| root.join(".lux/builds"))
            .unwrap_or_else(|| PathBuf::from(".lux/builds"));
        Self::new(base_output_dir)
    }
}

impl Default for BuildManager {
    fn default() -> Self {
        Self::new(PathBuf::from(".lux/builds"))
    }
}

pub fn start_build(
    manager: &mut BuildManager,
    project_path: &Path,
    target: BuildTarget,
) -> Result<String> {
    let running_count = manager
        .jobs
        .values()
        .filter(|job| job.status == BuildStatus::Running)
        .count();
    if running_count >= manager.max_concurrent {
        bail!("maximum concurrent Lux builds reached");
    }

    let build_id = Uuid::new_v4().to_string();
    let build_output_dir = manager.base_output_dir.join(&build_id);
    let artifact_path = get_build_artifact_path(&build_id, &manager.base_output_dir);
    let command = unity_webgl_build_command(project_path, target.clone(), &build_output_dir);

    let job = BuildJob {
        build_id: build_id.clone(),
        project_path: project_path.to_path_buf(),
        target,
        status: BuildStatus::Queued,
        progress: 0.0,
        started_at: Some(Utc::now().to_rfc3339()),
        completed_at: None,
        artifact_path: Some(artifact_path),
        log: vec![format!("Queued Unity WebGL build: {command}")],
        error: None,
    };

    manager.jobs.insert(build_id.clone(), job);
    Ok(build_id)
}

pub fn get_build_status<'a>(manager: &'a BuildManager, build_id: &str) -> Result<&'a BuildJob> {
    manager
        .jobs
        .get(build_id)
        .ok_or_else(|| anyhow::anyhow!("Lux build not found: {build_id}"))
}

pub fn cancel_build(manager: &mut BuildManager, build_id: &str) -> Result<()> {
    let job = manager
        .jobs
        .get_mut(build_id)
        .ok_or_else(|| anyhow::anyhow!("Lux build not found: {build_id}"))?;
    if matches!(
        job.status,
        BuildStatus::Succeeded | BuildStatus::Failed(_) | BuildStatus::Cancelled
    ) {
        return Ok(());
    }
    job.status = BuildStatus::Cancelled;
    job.progress = 0.0;
    job.completed_at = Some(Utc::now().to_rfc3339());
    job.log.push("Build cancelled".to_string());
    Ok(())
}

pub fn list_builds(manager: &BuildManager) -> Vec<&BuildJob> {
    let mut jobs: Vec<&BuildJob> = manager.jobs.values().collect();
    jobs.sort_by(|left, right| left.build_id.cmp(&right.build_id));
    jobs
}

pub fn get_build_log(manager: &BuildManager, build_id: &str) -> Result<Vec<String>> {
    Ok(get_build_status(manager, build_id)?.log.clone())
}

pub fn get_build_artifact_path(build_id: &str, base: &Path) -> PathBuf {
    base.join(build_id).join("index.html")
}

pub fn mark_build_running(manager: &mut BuildManager, build_id: &str) -> Result<()> {
    let job = manager
        .jobs
        .get_mut(build_id)
        .ok_or_else(|| anyhow::anyhow!("Lux build not found: {build_id}"))?;
    job.status = BuildStatus::Running;
    job.progress = job.progress.max(0.01);
    job.log.push("Build running".to_string());
    Ok(())
}

pub fn mark_build_succeeded(manager: &mut BuildManager, build_id: &str) -> Result<()> {
    let job = manager
        .jobs
        .get_mut(build_id)
        .ok_or_else(|| anyhow::anyhow!("Lux build not found: {build_id}"))?;
    job.status = BuildStatus::Succeeded;
    job.progress = 1.0;
    job.completed_at = Some(Utc::now().to_rfc3339());
    job.log.push("Build succeeded".to_string());
    Ok(())
}

pub fn append_build_log(
    manager: &mut BuildManager,
    build_id: &str,
    entry: impl Into<String>,
) -> Result<()> {
    let job = manager
        .jobs
        .get_mut(build_id)
        .ok_or_else(|| anyhow::anyhow!("Lux build not found: {build_id}"))?;
    job.log.push(entry.into());
    Ok(())
}

pub fn unity_webgl_build_command(
    project_path: &Path,
    target: BuildTarget,
    build_output_dir: &Path,
) -> String {
    format!(
        "Unity -batchmode -projectPath {} -executeMethod LuxBatchAutomation.Compile -buildTarget {} -buildPath {}",
        project_path.display(),
        target.as_unity_arg(),
        build_output_dir.display()
    )
}
