use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const WORKTREE_STATE_FILE: &str = "worktree.json";
const MERGE_REQUEST_FILE: &str = "merge-request.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeConfig {
    pub base_branch: String,
    pub worktree_root: PathBuf,
    pub unity_meta_sync: bool,
    pub max_age_secs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub id: String,
    pub agent_id: String,
    pub status: WorktreeStatus,
    pub branch_name: String,
    pub base_sha: String,
    pub title: String,
    pub changed_files: Vec<String>,
    pub unity_meta_hash: String,
    pub created_at: String,
    pub updated_at: String,
    pub review_comments: Vec<ReviewComment>,
    pub merge_request_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewComment {
    pub reviewer_id: String,
    pub content: String,
    pub created_at: String,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorktreeStatus {
    Active,
    Submitted,
    Approved,
    Rejected,
    Quarantined,
    Merged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeRequest {
    pub id: String,
    pub worktree_id: String,
    pub source_branch: String,
    pub target_branch: String,
    pub status: MergeStatus,
    pub submitted_at: String,
    pub reviewed_at: Option<String>,
    pub reviewer_id: Option<String>,
    pub verdict: Option<String>,
    pub verdict_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MergeStatus {
    PendingReview,
    Approved,
    Rejected,
    Merged,
    Conflict,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityMetaValidationReport {
    pub valid: bool,
    pub expected_hash: String,
    pub actual_hash: String,
    pub checked_files: Vec<String>,
    pub errors: Vec<String>,
}

impl Worktree {
    pub fn create(config: &WorktreeConfig, agent_id: &str, title: &str) -> Result<Worktree> {
        let id = Uuid::new_v4().to_string();
        let project_path = infer_project_path_from_worktree_root(&config.worktree_root)?;
        let worktree_path = config.worktree_root.clone();
        let base_sha = git_stdout(&project_path, &["rev-parse", &config.base_branch])?;
        let branch_name = format!("lux/{}/{}", sanitize_branch_part(agent_id), id);

        if worktree_path.exists() {
            bail!(
                "worktree path already exists at {}",
                worktree_path.display()
            );
        }

        git_status(
            &project_path,
            &[
                "worktree",
                "add",
                "-b",
                &branch_name,
                path_arg(&worktree_path)?.as_str(),
                &base_sha,
            ],
        )?;

        let now = now_iso8601();
        let unity_meta_hash = if config.unity_meta_sync {
            compute_unity_meta_hash(&worktree_path)?
        } else {
            String::new()
        };
        let worktree = Worktree {
            id,
            agent_id: agent_id.to_owned(),
            status: WorktreeStatus::Active,
            branch_name,
            base_sha,
            title: title.to_owned(),
            changed_files: scan_changed_files(&worktree_path)?,
            unity_meta_hash,
            created_at: now.clone(),
            updated_at: now,
            review_comments: Vec::new(),
            merge_request_id: None,
        };
        worktree.save(&project_path.join(".lux"))?;
        Ok(worktree)
    }

    pub fn submit(&self) -> Result<MergeRequest> {
        if self.status != WorktreeStatus::Active && self.status != WorktreeStatus::Rejected {
            bail!("only active or rejected worktrees can be submitted for review");
        }
        let lux_dir = discover_lux_dir_for_id(&self.id)?;
        let worktree_path = worktree_path(&lux_dir, &self.id);
        let target_branch = target_branch_for_worktree(&worktree_path, &self.base_sha)?;
        let now = now_iso8601();
        let merge_request = MergeRequest {
            id: Uuid::new_v4().to_string(),
            worktree_id: self.id.clone(),
            source_branch: self.branch_name.clone(),
            target_branch,
            status: MergeStatus::PendingReview,
            submitted_at: now.clone(),
            reviewed_at: None,
            reviewer_id: None,
            verdict: None,
            verdict_reason: None,
        };

        let mut submitted = self.clone();
        submitted.status = WorktreeStatus::Submitted;
        submitted.changed_files = scan_changed_files(&worktree_path)?;
        submitted.unity_meta_hash = compute_unity_meta_hash(&worktree_path)?;
        submitted.updated_at = now;
        submitted.merge_request_id = Some(merge_request.id.clone());
        submitted.save(&lux_dir)?;
        save_merge_request(&lux_dir, &submitted.id, &merge_request)?;
        Ok(merge_request)
    }

    pub fn review(&mut self, reviewer_id: &str, verdict: &str, reason: &str) -> Result<()> {
        if self.status != WorktreeStatus::Submitted {
            bail!("only submitted worktrees can be reviewed");
        }
        let lux_dir = discover_lux_dir_for_id(&self.id)?;
        let normalized = verdict.trim().to_ascii_lowercase();
        let now = now_iso8601();
        let resolution = match normalized.as_str() {
            "approve" | "approved" => {
                self.status = WorktreeStatus::Approved;
                Some("accepted".to_owned())
            }
            "reject" | "rejected" => {
                self.status = WorktreeStatus::Rejected;
                Some("rejected".to_owned())
            }
            _ => bail!("review verdict must be approve or reject"),
        };
        self.review_comments.push(ReviewComment {
            reviewer_id: reviewer_id.to_owned(),
            content: reason.to_owned(),
            created_at: now.clone(),
            resolution,
        });
        self.updated_at = now.clone();

        if let Some(id) = self.merge_request_id.as_deref() {
            let mut request = load_merge_request(&lux_dir, &self.id, id)?;
            request.status = if self.status == WorktreeStatus::Approved {
                MergeStatus::Approved
            } else {
                MergeStatus::Rejected
            };
            request.reviewed_at = Some(now);
            request.reviewer_id = Some(reviewer_id.to_owned());
            request.verdict = Some(if self.status == WorktreeStatus::Approved {
                "approve".to_owned()
            } else {
                "reject".to_owned()
            });
            request.verdict_reason = Some(reason.to_owned());
            save_merge_request(&lux_dir, &self.id, &request)?;
        }

        self.save(&lux_dir)
    }

    pub fn merge(&mut self, project_path: &Path) -> Result<()> {
        if self.status != WorktreeStatus::Approved {
            bail!("worktree must be approved before merge");
        }
        let lux_dir = project_path.join(".lux");
        let source_path = worktree_path(&lux_dir, &self.id);
        if !self.validate_unity_meta(&source_path)? {
            self.quarantine("Unity meta integrity check failed before merge")?;
            bail!("Unity meta integrity check failed before merge");
        }

        let target_branch = target_branch_for_worktree(&source_path, &self.base_sha)?;
        git_status(project_path, &["checkout", &target_branch])?;
        let merge_result = Command::new("git")
            .arg("-C")
            .arg(project_path)
            .args(["merge", "--no-ff", &self.branch_name])
            .status()
            .with_context(|| {
                format!("failed to execute git merge in {}", project_path.display())
            })?;
        if !merge_result.success() {
            if let Some(id) = self.merge_request_id.as_deref() {
                let mut request = load_merge_request(&lux_dir, &self.id, id)?;
                request.status = MergeStatus::Conflict;
                save_merge_request(&lux_dir, &self.id, &request)?;
            }
            bail!("git merge failed for branch {}", self.branch_name);
        }

        if !self.validate_unity_meta(project_path)? {
            self.quarantine("Unity meta integrity check failed after merge")?;
            bail!("Unity meta integrity check failed after merge");
        }

        self.status = WorktreeStatus::Merged;
        self.changed_files = scan_changed_files(project_path).unwrap_or_default();
        self.updated_at = now_iso8601();
        if let Some(id) = self.merge_request_id.as_deref() {
            let mut request = load_merge_request(&lux_dir, &self.id, id)?;
            request.status = MergeStatus::Merged;
            save_merge_request(&lux_dir, &self.id, &request)?;
        }
        self.save(&lux_dir)
    }

    pub fn quarantine(&mut self, reason: &str) -> Result<()> {
        self.status = WorktreeStatus::Quarantined;
        self.updated_at = now_iso8601();
        self.review_comments.push(ReviewComment {
            reviewer_id: "lux-worktree".to_owned(),
            content: reason.to_owned(),
            created_at: self.updated_at.clone(),
            resolution: Some("rejected".to_owned()),
        });
        let lux_dir = discover_lux_dir_for_id(&self.id)?;
        self.save(&lux_dir)
    }

    pub fn validate_unity_meta(&self, project_path: &Path) -> Result<bool> {
        let report = self.validate_unity_meta_report(project_path)?;
        if !report.valid {
            eprintln!(
                "Unity meta validation failed for worktree {}: expected {}, actual {}",
                self.id, report.expected_hash, report.actual_hash
            );
            for error in &report.errors {
                eprintln!("  - {error}");
            }
        }
        Ok(report.valid)
    }

    pub fn validate_unity_meta_report(
        &self,
        project_path: &Path,
    ) -> Result<UnityMetaValidationReport> {
        let files = collect_unity_yaml_files(project_path)?;
        let mut errors = Vec::new();
        for file in &files {
            if file.extension().and_then(|ext| ext.to_str()) == Some("meta") {
                validate_basic_yaml(file, project_path, &mut errors)?;
            } else if is_unity_yaml_asset(file) {
                validate_basic_yaml(file, project_path, &mut errors)?;
            }
        }
        let actual_hash = hash_files(project_path, &files)?;
        let hash_matches = self.unity_meta_hash.is_empty() || actual_hash == self.unity_meta_hash;
        Ok(UnityMetaValidationReport {
            valid: errors.is_empty() && hash_matches,
            expected_hash: self.unity_meta_hash.clone(),
            actual_hash,
            checked_files: files
                .iter()
                .map(|path| relative_path(project_path, path))
                .collect(),
            errors,
        })
    }

    pub fn list_all(lux_dir: &Path) -> Result<Vec<Worktree>> {
        let root = lux_dir.join("worktrees");
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut worktrees = Vec::new();
        for entry in fs::read_dir(&root)
            .with_context(|| format!("failed to read worktrees directory {}", root.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read entry in {}", root.display()))?;
            if !entry
                .file_type()
                .with_context(|| format!("failed to inspect {}", entry.path().display()))?
                .is_dir()
            {
                continue;
            }
            let id = entry.file_name().to_string_lossy().to_string();
            match Self::load(lux_dir, &id) {
                Ok(worktree) => worktrees.push(worktree),
                Err(err) => eprintln!("failed to load worktree {id}: {err:#}"),
            }
        }
        worktrees.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        Ok(worktrees)
    }

    pub fn save(&self, lux_dir: &Path) -> Result<()> {
        let path = worktree_path(lux_dir, &self.id).join(WORKTREE_STATE_FILE);
        crate::lux_io::atomic_write_json(&path, self)
    }

    pub fn load(lux_dir: &Path, id: &str) -> Result<Worktree> {
        let path = worktree_path(lux_dir, id).join(WORKTREE_STATE_FILE);
        if !path.exists() {
            bail!("worktree state not found at {}", path.display());
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed to read worktree state {}", path.display()))?;
        serde_json::from_str(&content)
            .with_context(|| format!("failed to parse worktree state {}", path.display()))
    }
}

pub fn scan_changed_files(worktree_path: &Path) -> Result<Vec<String>> {
    let base = git_stdout(worktree_path, &["rev-parse", "HEAD"])?;
    let output = git_stdout(worktree_path, &["diff", "--name-only", &base])?;
    let staged = git_stdout(worktree_path, &["diff", "--cached", "--name-only", &base])?;
    let mut files: Vec<String> = output
        .lines()
        .chain(staged.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

pub fn compute_unity_meta_hash(project_path: &Path) -> Result<String> {
    let files = collect_unity_yaml_files(project_path)?;
    hash_files(project_path, &files)
}

fn collect_unity_yaml_files(project_path: &Path) -> Result<Vec<PathBuf>> {
    let assets_dir = project_path.join("Assets");
    if !assets_dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_unity_yaml_files_recursive(&assets_dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_unity_yaml_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry.with_context(|| format!("failed to read entry in {}", dir.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        if file_type.is_dir() {
            collect_unity_yaml_files_recursive(&path, files)?;
        } else if file_type.is_file()
            && (path.extension().and_then(|ext| ext.to_str()) == Some("meta")
                || is_unity_yaml_asset(&path))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn is_unity_yaml_asset(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("asset" | "prefab" | "unity" | "mat" | "anim" | "controller" | "overrideController")
    )
}

fn validate_basic_yaml(file: &Path, root: &Path, errors: &mut Vec<String>) -> Result<()> {
    let content = fs::read_to_string(file)
        .with_context(|| format!("failed to read Unity YAML file {}", file.display()))?;
    let mut has_mapping = false;
    let mut has_unbalanced_bracket = false;
    for (line_number, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed == "%YAML 1.1"
            || trimmed.starts_with("%TAG")
            || trimmed.starts_with("---")
        {
            continue;
        }
        if trimmed.contains(':') {
            has_mapping = true;
        }
        if trimmed.matches('[').count() != trimmed.matches(']').count()
            || trimmed.matches('{').count() != trimmed.matches('}').count()
        {
            has_unbalanced_bracket = true;
            errors.push(format!(
                "{}:{} has unbalanced inline YAML delimiters",
                relative_path(root, file),
                line_number + 1
            ));
        }
    }
    if !content.trim().is_empty() && !has_mapping {
        errors.push(format!(
            "{} does not contain a basic YAML mapping",
            relative_path(root, file)
        ));
    }
    if has_unbalanced_bracket {
        return Ok(());
    }
    Ok(())
}

fn hash_files(root: &Path, files: &[PathBuf]) -> Result<String> {
    let mut hasher = Sha256::new();
    for file in files {
        let relative = relative_path(root, file);
        hasher.update(relative.as_bytes());
        hasher.update(b"\0");
        let content = fs::read(file)
            .with_context(|| format!("failed to read Unity meta/YAML file {}", file.display()))?;
        hasher.update(content);
        hasher.update(b"\0");
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn save_merge_request(lux_dir: &Path, worktree_id: &str, request: &MergeRequest) -> Result<()> {
    let path = worktree_path(lux_dir, worktree_id).join(MERGE_REQUEST_FILE);
    crate::lux_io::atomic_write_json(&path, request)
}

fn load_merge_request(lux_dir: &Path, worktree_id: &str, request_id: &str) -> Result<MergeRequest> {
    let path = worktree_path(lux_dir, worktree_id).join(MERGE_REQUEST_FILE);
    if !path.exists() {
        bail!(
            "merge request {} not found at {}",
            request_id,
            path.display()
        );
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read merge request {}", path.display()))?;
    let request: MergeRequest = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse merge request {}", path.display()))?;
    if request.id != request_id {
        bail!(
            "merge request id mismatch: expected {}, found {}",
            request_id,
            request.id
        );
    }
    Ok(request)
}

fn worktree_path(lux_dir: &Path, id: &str) -> PathBuf {
    lux_dir.join("worktrees").join(id)
}

fn infer_project_path_from_worktree_root(worktree_root: &Path) -> Result<PathBuf> {
    let worktrees_dir = worktree_root
        .parent()
        .context("worktree_root must have a parent worktrees directory")?;
    let lux_dir = worktrees_dir
        .parent()
        .context("worktree_root must be under .lux/worktrees")?;
    if lux_dir.file_name().and_then(|name| name.to_str()) != Some(".lux") {
        bail!(
            "worktree_root {} must be under .lux/worktrees/<id>",
            worktree_root.display()
        );
    }
    lux_dir
        .parent()
        .map(Path::to_path_buf)
        .context(".lux directory must have a project parent")
}

fn discover_lux_dir_for_id(id: &str) -> Result<PathBuf> {
    let mut current = std::env::current_dir().context("failed to read current directory")?;
    loop {
        let candidate = current.join(".lux");
        if candidate.join("worktrees").join(id).exists() {
            return Ok(candidate);
        }
        if current.file_name().and_then(|name| name.to_str()) == Some(id) {
            if let Some(worktrees_dir) = current.parent() {
                if worktrees_dir.file_name().and_then(|name| name.to_str()) == Some("worktrees") {
                    if let Some(lux_dir) = worktrees_dir.parent() {
                        if lux_dir.file_name().and_then(|name| name.to_str()) == Some(".lux") {
                            return Ok(lux_dir.to_path_buf());
                        }
                    }
                }
            }
        }
        if !current.pop() {
            bail!(
                "failed to locate .lux/worktrees/{} from current directory",
                id
            );
        }
    }
}

fn target_branch_for_worktree(worktree_path: &Path, base_sha: &str) -> Result<String> {
    let branches = git_stdout(worktree_path, &["branch", "--contains", base_sha])?;
    for line in branches.lines() {
        let branch = line.trim().trim_start_matches('*').trim();
        if branch == "main" || branch == "master" {
            return Ok(branch.to_owned());
        }
    }
    Ok("main".to_owned())
}

fn git_stdout(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute git {:?} in {}", args, repo.display()))?;
    if !output.status.success() {
        bail!(
            "git {:?} failed in {}: {}",
            args,
            repo.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn git_status(repo: &Path, args: &[&str]) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute git {:?} in {}", args, repo.display()))?;
    if !status.success() {
        bail!("git {:?} failed in {}", args, repo.display());
    }
    Ok(())
}

fn path_arg(path: &Path) -> Result<String> {
    path.to_str()
        .map(ToOwned::to_owned)
        .with_context(|| format!("path is not valid UTF-8: {}", path.display()))
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn sanitize_branch_part(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    sanitized.trim_matches('-').to_owned()
}

fn now_iso8601() -> String {
    Utc::now().to_rfc3339()
}
