use std::path::{Path, PathBuf};

use crate::lux_manual_qa_types::{ManualQaCapabilityStatus, ManualQaPhase};

pub(crate) const fn phase_label(phase: ManualQaPhase) -> &'static str {
    match phase {
        ManualQaPhase::Compile => "compile",
        ManualQaPhase::Test => "test",
        ManualQaPhase::DynamicCode => "dynamic_code",
        ManualQaPhase::Screenshot => "screenshot",
        ManualQaPhase::DevServer => "dev_server",
        ManualQaPhase::BrowserScreenshot => "browser_screenshot",
        ManualQaPhase::GodotVersion => "godot_version",
    }
}

pub(crate) const fn phase_requires_screenshot_path(phase: ManualQaPhase) -> bool {
    matches!(
        phase,
        ManualQaPhase::Screenshot | ManualQaPhase::BrowserScreenshot
    )
}

pub(crate) const fn capability_blocks(status: ManualQaCapabilityStatus) -> bool {
    matches!(status, ManualQaCapabilityStatus::Blocker)
}

pub(crate) fn evidence_label(label: &str) -> String {
    label
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

pub(crate) fn path_to_slash(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn executable_in_path(executable: &str) -> Option<PathBuf> {
    let executable_path = Path::new(executable);
    if executable_path.is_file() {
        return Some(executable_path.to_path_buf());
    }
    if executable_path.components().count() > 1 || executable_path.is_absolute() {
        return None;
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(executable))
            .find(|candidate| candidate.is_file())
    })
}

pub(crate) fn screenshot_path_from_stdout(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        line.strip_prefix("screenshot_path=")
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(ToOwned::to_owned)
    })
}
