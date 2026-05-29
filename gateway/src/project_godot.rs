use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GodotProjectDetection {
    pub project_root: PathBuf,
    pub godot_version: Option<String>,
    pub has_godot_dir: bool,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn detects_godot_4_project() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("project.godot"), "config_version=5\n").unwrap();
        fs::create_dir(dir.path().join(".godot")).unwrap();

        let detection = detect_godot_project(dir.path()).unwrap();
        assert_eq!(detection.godot_version, Some("4.x".to_string()));
        assert!(detection.has_godot_dir);
    }

    #[test]
    fn rejects_godot_3_project() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("project.godot"), "config_version=4\n").unwrap();

        assert!(detect_godot_project(dir.path()).is_none());
    }
}
