use std::path::{Path, PathBuf};

/// Normalize a path to use forward slashes consistently (works on Windows/macOS/Linux)
pub fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// Shorten an absolute path to relative form using ~ for home directory
pub fn shorten_path(path: &str, project_root: &str) -> String {
    let normalized_path = normalize_path(path);
    let normalized_root = normalize_path(project_root)
        .trim_end_matches('/')
        .to_string();

    if normalized_root.is_empty() {
        return normalized_path;
    }

    if normalized_path == normalized_root {
        return "~".to_string();
    }

    normalized_path
        .strip_prefix(&format!("{normalized_root}/"))
        .map(|suffix| format!("~/{suffix}"))
        .unwrap_or(normalized_path)
}

/// Check if a path looks like it could be absolute (has drive letter or starts with /)
pub fn is_absolute_path(path: &str) -> bool {
    let normalized = normalize_path(path);
    normalized.starts_with('/') || has_windows_drive_letter(&normalized)
}

pub fn normalize_path_buf(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(normalize_path(&path.as_ref().to_string_lossy()))
}

pub fn display_path(path: impl AsRef<Path>) -> String {
    normalize_path(&path.as_ref().to_string_lossy())
}

pub fn join_forward(base: impl AsRef<Path>, segments: &[&str]) -> PathBuf {
    let mut path = display_path(base).trim_end_matches('/').to_string();
    for segment in segments {
        if !path.is_empty() {
            path.push('/');
        }
        path.push_str(segment.trim_matches(['/', '\\']));
    }
    PathBuf::from(path)
}

pub fn shell_command_for_platform(script: &str, windows: bool) -> (String, Vec<String>) {
    if windows {
        (
            "cmd".to_string(),
            vec!["/C".to_string(), script.to_string()],
        )
    } else {
        ("sh".to_string(), vec!["-c".to_string(), script.to_string()])
    }
}

pub fn shell_command(script: &str) -> (String, Vec<String>) {
    shell_command_for_platform(script, cfg!(windows))
}

fn has_windows_drive_letter(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_converts_backslashes_to_forward_slashes() {
        assert_eq!(
            normalize_path(r#"C:\Users\Lux\Project"#),
            "C:/Users/Lux/Project"
        );
    }

    #[test]
    fn shorten_path_removes_project_prefix() {
        assert_eq!(
            shorten_path("/Users/dev/Game/Assets/Scene.unity", "/Users/dev/Game"),
            "~/Assets/Scene.unity"
        );
    }

    #[test]
    fn is_absolute_path_detects_windows_drive_letter() {
        assert!(is_absolute_path(r#"C:\Users\dev\Game"#));
        assert!(is_absolute_path("D:/Projects/Game"));
        assert!(is_absolute_path("/Users/dev/Game"));
        assert!(!is_absolute_path("Assets/Scene.unity"));
    }

    #[test]
    fn shell_command_execution_handles_platform_differences() {
        let (windows_shell, windows_args) = shell_command_for_platform("echo lux", true);
        assert_eq!(windows_shell, "cmd");
        assert_eq!(windows_args, ["/C", "echo lux"]);

        let (unix_shell, unix_args) = shell_command_for_platform("echo lux", false);
        assert_eq!(unix_shell, "sh");
        assert_eq!(unix_args, ["-c", "echo lux"]);
    }
}
