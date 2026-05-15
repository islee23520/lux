//! Advisory file lock for .lux/ directory — prevents concurrent agent writes.
//! Uses PID-based liveness check with stale threshold and force-acquire protocol.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const DEFAULT_STALE_THRESHOLD_SECS: u64 = 300;
pub const LOCK_FILENAME: &str = ".lock";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LuxLockContent {
    pub pid: u64,
    pub acquired_at: String,
    pub agent_id: Option<String>,
    pub reason: Option<String>,
}

impl LuxLockContent {
    pub fn new(pid: u64, agent_id: Option<String>, reason: Option<String>) -> Self {
        Self {
            pid,
            acquired_at: Utc::now().to_rfc3339(),
            agent_id,
            reason,
        }
    }

    pub fn is_holder_alive(&self) -> bool {
        #[cfg(unix)]
        {
            // SAFETY: kill(pid, 0) is a standard POSIX liveness probe — no signal is sent.
            // Returns 0 if process exists, -1 with errno=ESRCH if not.
            let result = unsafe { libc::kill(self.pid as libc::pid_t, 0) };
            if result == 0 {
                return true;
            }
            let errno = unsafe { *libc::__error() };
            errno != libc::ESRCH
        }
        #[cfg(not(unix))]
        {
            true
        }
    }

    pub fn is_stale(&self, threshold_secs: u64) -> bool {
        if self.is_holder_alive() {
            return false;
        }
        if let Ok(acquired) = self.acquired_at.parse::<DateTime<Utc>>() {
            let elapsed = Utc::now().signed_duration_since(acquired);
            elapsed.num_seconds() > threshold_secs as i64
        } else {
            true
        }
    }
}

pub struct LuxLockGuard {
    path: PathBuf,
    acquired: bool,
}

impl LuxLockGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            acquired: false,
        }
    }
}

impl Drop for LuxLockGuard {
    fn drop(&mut self) {
        if self.acquired {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Acquire the .lux advisory lock.
///
/// Returns `Ok(LuxLockGuard)` on success; the guard releases the lock on drop.
/// Returns `Err` if the lock is held by a live process and `force` is false.
pub fn acquire_lux_lock(
    lux_dir: &Path,
    agent_id: &str,
    reason: &str,
    stale_threshold_secs: u64,
    force: bool,
) -> Result<LuxLockGuard> {
    let lock_path = lux_dir.join(LOCK_FILENAME);

    fs::create_dir_all(lux_dir)
        .with_context(|| format!("failed to create .lux directory {}", lux_dir.display()))?;

    if lock_path.exists() {
        match fs::read_to_string(&lock_path) {
            Ok(json_str) => {
                let existing: LuxLockContent =
                    serde_json::from_str(&json_str).with_context(|| {
                        format!("failed to parse lock file {}", lock_path.display())
                    })?;

                if existing.is_stale(stale_threshold_secs) {
                    eprintln!(
                        "⚠️  [lux-lock] Stale lock detected (pid={}, age > {}s). Force-acquiring.",
                        existing.pid, stale_threshold_secs
                    );
                } else if !force {
                    bail!(
                        "Lux lock held by live process (pid={}, agent={}, reason={}). \
                         Use --force to override or wait for release. \
                         Lock file: {}",
                        existing.pid,
                        existing.agent_id.as_deref().unwrap_or("unknown"),
                        existing.reason.as_deref().unwrap_or("unknown"),
                        lock_path.display()
                    );
                } else {
                    eprintln!(
                        "⚠️  [lux-lock] Force-acquiring lock from live process (pid={}).",
                        existing.pid
                    );
                }
            }
            Err(err) => {
                eprintln!(
                    "⚠️  [lux-lock] Unreadable lock file detected ({}). Force-acquiring.",
                    err
                );
            }
        }
    }

    let content = LuxLockContent::new(
        std::process::id() as u64,
        Some(agent_id.to_string()),
        Some(reason.to_string()),
    );

    let json =
        serde_json::to_string_pretty(&content).context("failed to serialize lock content")?;
    let tmp_path = lock_path.with_extension("lock.tmp");
    fs::write(&tmp_path, json)
        .with_context(|| format!("failed to write temp lock {}", tmp_path.display()))?;
    fs::rename(&tmp_path, &lock_path)
        .with_context(|| format!("failed to rename lock to {}", lock_path.display()))?;

    let mut guard = LuxLockGuard::new(lock_path);
    guard.acquired = true;
    Ok(guard)
}

/// Check if .lux/ is currently locked. Returns `None` if no lock file exists.
pub fn check_lux_lock(lux_dir: &Path) -> Result<Option<LuxLockContent>> {
    let lock_path = lux_dir.join(LOCK_FILENAME);
    if !lock_path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(&lock_path)
        .with_context(|| format!("failed to read lock file {}", lock_path.display()))?;
    let content: LuxLockContent = serde_json::from_str(&json)
        .with_context(|| format!("failed to parse lock file {}", lock_path.display()))?;
    if !content.is_holder_alive() {
        eprintln!("⚠️  [lux-lock] Stale lock detected (pid={})", content.pid);
    }
    Ok(Some(content))
}
