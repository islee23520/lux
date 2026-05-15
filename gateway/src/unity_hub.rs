use anyhow::{Context, Result};
use std::{fs, path::PathBuf};

pub struct UnityHubInfo {
    pub hub_path: PathBuf,
    pub install_path: PathBuf,
}

pub struct DetectedEditor {
    pub version: String,
    pub executable: PathBuf,
}

pub fn discover_hub() -> Result<Option<UnityHubInfo>> {
    let mut candidates = Vec::new();

    if let Some(path) = std::env::var_os("LUX_UNITY_HUB_PATH") {
        candidates.push(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        candidates.push(PathBuf::from("C:\\Program Files\\Unity Hub"));
        candidates.push(PathBuf::from("C:\\Program Files (x86)\\Unity Hub"));

        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = current_user.open_subkey("Software\\Unity Technologies\\Hub") {
            if let Ok(path) = key.get_value::<String, _>("InstallPath") {
                candidates.push(PathBuf::from(path));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/Applications/Unity Hub.app"));
        candidates.push(PathBuf::from("/Applications/Unity/Hub"));
    }

    #[cfg(target_os = "linux")]
    {
        candidates.push(PathBuf::from("/opt/Unity Hub"));
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(PathBuf::from(home).join(".local/share/Unity Hub"));
        }
    }

    for hub_path in candidates {
        if hub_path.exists() {
            return Ok(Some(UnityHubInfo {
                install_path: editor_install_path_for_hub(&hub_path),
                hub_path,
            }));
        }
    }

    Ok(None)
}

pub fn list_installed_editors(hub: &UnityHubInfo) -> Result<Vec<DetectedEditor>> {
    if !hub.install_path.is_dir() {
        return Ok(Vec::new());
    }

    let mut editors = Vec::new();
    for entry in fs::read_dir(&hub.install_path).with_context(|| {
        format!(
            "failed to read Unity editor directory {}",
            hub.install_path.display()
        )
    })? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let version = entry.file_name().to_string_lossy().to_string();
        let executable = editor_executable_for_version(&hub.install_path, &version);
        if executable.is_file() {
            editors.push(DetectedEditor {
                version,
                executable,
            });
        }
    }
    editors.sort_by(|left, right| left.version.cmp(&right.version));
    Ok(editors)
}

pub fn find_editor_for_version(hub: &UnityHubInfo, version: &str) -> Result<Option<PathBuf>> {
    let executable = editor_executable_for_version(&hub.install_path, version);
    if executable.is_file() {
        return Ok(Some(executable));
    }
    Ok(None)
}

pub fn auto_detect_project_root() -> Result<Option<PathBuf>> {
    let mut current = std::env::current_dir().context("failed to read current directory")?;
    loop {
        if current
            .join("ProjectSettings")
            .join("ProjectVersion.txt")
            .is_file()
        {
            return Ok(Some(current));
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}

pub fn editor_install_path_for_hub(hub_path: &std::path::Path) -> PathBuf {
    if hub_path.file_name().is_some_and(|name| name == "Editor") {
        return hub_path.to_path_buf();
    }
    hub_path.join("Editor")
}

pub fn editor_executable_for_version(install_path: &std::path::Path, version: &str) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        install_path.join(version).join("Editor").join("Unity.exe")
    }
    #[cfg(target_os = "macos")]
    {
        install_path
            .join(version)
            .join("Unity.app")
            .join("Contents")
            .join("MacOS")
            .join("Unity")
    }
    #[cfg(target_os = "linux")]
    {
        install_path.join(version).join("Editor").join("Unity")
    }
}
