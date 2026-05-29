use std::{fs, path::PathBuf};

use lux::lux_build::{
    append_build_log, cancel_build, get_build_artifact_path, get_build_log, get_build_status,
    list_builds, mark_build_running, mark_build_succeeded, start_build, BuildManager, BuildStatus,
    BuildTarget,
};

fn temp_project_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!("lux-build-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn manager_for(root: &std::path::Path) -> BuildManager {
    BuildManager::new(root.join(".lux/builds"))
}

#[test]
fn test_start_build_creates_job() {
    let root = temp_project_root();
    let mut manager = manager_for(&root);

    let build_id = start_build(&mut manager, &root, BuildTarget::WebGL).unwrap();
    let job = get_build_status(&manager, &build_id).unwrap();

    assert_eq!(job.status, BuildStatus::Queued);
    assert_eq!(job.target, BuildTarget::WebGL);
    assert_eq!(job.project_path, root);
    assert_eq!(job.progress, 0.0);
    assert!(job.started_at.is_some());
    assert!(job.completed_at.is_none());
    assert!(job
        .log
        .iter()
        .any(|entry| entry.contains("-executeMethod LuxBatchAutomation.Compile")));
}

#[test]
fn test_build_status_lifecycle() {
    let root = temp_project_root();
    let mut manager = manager_for(&root);
    let build_id = start_build(&mut manager, &root, BuildTarget::WebGL).unwrap();

    assert_eq!(
        get_build_status(&manager, &build_id).unwrap().status,
        BuildStatus::Queued
    );
    mark_build_running(&mut manager, &build_id).unwrap();
    assert_eq!(
        get_build_status(&manager, &build_id).unwrap().status,
        BuildStatus::Running
    );
    mark_build_succeeded(&mut manager, &build_id).unwrap();

    let job = get_build_status(&manager, &build_id).unwrap();
    assert_eq!(job.status, BuildStatus::Succeeded);
    assert_eq!(job.progress, 1.0);
    assert!(job.completed_at.is_some());
}

#[test]
fn test_cancel_build() {
    let root = temp_project_root();
    let mut manager = manager_for(&root);
    let build_id = start_build(&mut manager, &root, BuildTarget::WebGL).unwrap();

    cancel_build(&mut manager, &build_id).unwrap();

    let job = get_build_status(&manager, &build_id).unwrap();
    assert_eq!(job.status, BuildStatus::Cancelled);
    assert!(job.completed_at.is_some());
    assert!(job.log.iter().any(|entry| entry == "Build cancelled"));
}

#[test]
fn test_list_builds() {
    let root = temp_project_root();
    let mut manager = manager_for(&root);

    let first = start_build(&mut manager, &root, BuildTarget::WebGL).unwrap();
    let second = start_build(&mut manager, &root, BuildTarget::WebGL).unwrap();

    let build_ids = list_builds(&manager)
        .into_iter()
        .map(|job| job.build_id.clone())
        .collect::<Vec<_>>();
    assert_eq!(build_ids.len(), 2);
    assert!(build_ids.contains(&first));
    assert!(build_ids.contains(&second));
}

#[test]
fn test_build_artifact_path() {
    let base = PathBuf::from(".lux/builds");

    let path = get_build_artifact_path("build-123", &base);

    assert_eq!(path, PathBuf::from(".lux/builds/build-123/index.html"));
}

#[test]
fn test_build_log_accumulation() {
    let root = temp_project_root();
    let mut manager = manager_for(&root);
    let build_id = start_build(&mut manager, &root, BuildTarget::WebGL).unwrap();

    append_build_log(&mut manager, &build_id, "step one").unwrap();
    append_build_log(&mut manager, &build_id, "step two").unwrap();

    let log = get_build_log(&manager, &build_id).unwrap();
    assert!(log
        .iter()
        .any(|entry| entry.contains("Queued Unity WebGL build")));
    assert_eq!(log[log.len() - 2], "step one");
    assert_eq!(log[log.len() - 1], "step two");
}
