// Uloop subprocess runner — delegates Unity CLI operations to unity-cli-loop (uloop)

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

const ULOOP_BINARY: &str = "uloop";

#[cfg(windows)]
const ULOOP_BIN_NAME: &str = "uloop.cmd";

#[cfg(not(windows))]
const ULOOP_BIN_NAME: &str = ULOOP_BINARY;

/// Find the uloop binary on the system.
///
/// Search order:
/// 1. Local project node_modules/.bin/uloop (or .cmd on Windows)
/// 2. Global npm bin directory
/// 3. PATH lookup via `which` or `where`
pub fn find_uloop_binary() -> Result<PathBuf> {
    if let Some(path) = find_local_node_modules_binary()? {
        return Ok(path);
    }

    if let Some(path) = find_global_npm_binary() {
        return Ok(path);
    }

    if let Some(path) = find_path_binary() {
        return Ok(path);
    }

    bail!(
        "uloop (unity-cli-loop) is not installed or was not found. \
         Install it with: npm install -g uloop-cli\n\
         Or run: lux unity install-uloop"
    )
}

/// Run a uloop command with the given arguments.
///
/// Returns `(stdout, stderr, exit_code)` without treating non-zero uloop exits as
/// Rust errors. Execution failures, such as a missing executable or spawn error,
/// are returned as `anyhow::Error`.
pub fn run_uloop_command(
    uloop_args: &[&str],
    project_path: Option<&Path>,
) -> Result<(String, String, i32)> {
    let binary = find_uloop_binary()?;
    let mut cmd = build_uloop_command(&binary, uloop_args, project_path);

    let output = cmd.output().with_context(|| {
        format!(
            "failed to execute uloop command: {}",
            command_summary(&binary, uloop_args, project_path)
        )
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, code))
}

fn build_uloop_command(binary: &Path, uloop_args: &[&str], project_path: Option<&Path>) -> Command {
    let mut cmd = Command::new(binary);
    cmd.args(uloop_args);

    if let Some(path) = project_path {
        cmd.arg("--project-path").arg(path);
    }

    cmd
}

fn find_local_node_modules_binary() -> Result<Option<PathBuf>> {
    let current_dir = std::env::current_dir().context("failed to resolve current directory")?;

    for dir in current_dir.ancestors() {
        let local_bin = dir.join("node_modules").join(".bin").join(ULOOP_BIN_NAME);
        if is_executable_candidate(&local_bin) {
            return Ok(Some(local_bin));
        }
    }

    Ok(None)
}

fn find_global_npm_binary() -> Option<PathBuf> {
    npm_prefix_binary()
        .into_iter()
        .chain(common_global_npm_binaries())
        .find(|path| is_executable_candidate(path))
}

fn npm_prefix_binary() -> Option<PathBuf> {
    let output = Command::new("npm")
        .args(["prefix", "-g"])
        .stdin(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if prefix.is_empty() {
        return None;
    }

    let prefix_path = PathBuf::from(prefix);
    #[cfg(windows)]
    let binary = prefix_path.join(ULOOP_BIN_NAME);
    #[cfg(not(windows))]
    let binary = prefix_path.join("bin").join(ULOOP_BIN_NAME);

    Some(binary)
}

fn common_global_npm_binaries() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(home) = home_dir() {
        candidates.push(home.join(".npm-global").join("bin").join(ULOOP_BIN_NAME));
        candidates.push(
            home.join(".local")
                .join("share")
                .join("npm")
                .join("bin")
                .join(ULOOP_BIN_NAME),
        );
        candidates.push(home.join(".yarn").join("bin").join(ULOOP_BIN_NAME));
    }

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/opt/homebrew/bin").join(ULOOP_BIN_NAME));
        candidates.push(PathBuf::from("/usr/local/bin").join(ULOOP_BIN_NAME));
    }

    #[cfg(windows)]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        candidates.push(PathBuf::from(appdata).join("npm").join(ULOOP_BIN_NAME));
    }

    candidates
}

fn find_path_binary() -> Option<PathBuf> {
    let lookup_command = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(lookup_command)
        .arg(ULOOP_BINARY)
        .stdin(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find(|path| is_executable_candidate(path))
}

fn is_executable_candidate(path: &Path) -> bool {
    path.is_file()
}

fn command_summary(binary: &Path, uloop_args: &[&str], project_path: Option<&Path>) -> String {
    let mut parts = vec![binary.display().to_string()];
    parts.extend(uloop_args.iter().map(|arg| shell_display(arg)));

    if let Some(path) = project_path {
        parts.push("--project-path".to_string());
        parts.push(shell_display(&path.display().to_string()));
    }

    parts.join(" ")
}

fn shell_display(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "-_=./:".contains(ch))
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .or_else(|| {
                let drive = std::env::var_os("HOMEDRIVE")?;
                let path = std::env::var_os("HOMEPATH")?;
                Some(PathBuf::from(format!(
                    "{}{}",
                    drive.to_string_lossy(),
                    path.to_string_lossy()
                )))
            })
    }

    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}
