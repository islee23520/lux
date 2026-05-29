use std::{fs, path::Path};

use anyhow::Context;

use crate::{bridge_types::DEFAULT_GODOT_BRIDGE_PORT, project_godot};

const PLUGIN_CFG: &str = r#"[plugin]
name="Lux Bridge"
description="Lux AI Bridge for Godot"
author="Linalab"
version="0.1.0"
script="bridge.gd"
"#;

fn bridge_gd() -> String {
    format!(
        r#"extends EditorPlugin

var tcp_client: StreamPeerTCP

func _enter_tree():
    tcp_client = StreamPeerTCP.new()
    tcp_client.connect_to_host("127.0.0.1", {})

func _exit_tree():
    if tcp_client:
        tcp_client.disconnect_from_host()
"#,
        DEFAULT_GODOT_BRIDGE_PORT
    )
}

pub fn install_godot_bridge(project_path: &Path) -> anyhow::Result<()> {
    if project_godot::detect_godot_project(project_path).is_none() {
        anyhow::bail!(
            "Godot 4 project not detected at {}. Expected project.godot with config_version=5",
            project_path.display()
        );
    }

    let bridge_dir = project_path.join("addons/lux_bridge");
    fs::create_dir_all(&bridge_dir)
        .with_context(|| format!("Failed to create {}", bridge_dir.display()))?;

    let plugin_cfg_path = bridge_dir.join("plugin.cfg");
    write_if_changed(&plugin_cfg_path, PLUGIN_CFG)?;
    println!("Installed {}", plugin_cfg_path.display());

    let bridge_gd_path = bridge_dir.join("bridge.gd");
    let bridge_gd = bridge_gd();
    write_if_changed(&bridge_gd_path, &bridge_gd)?;
    println!("Installed {}", bridge_gd_path.display());

    println!("Godot bridge installed to {}", bridge_dir.display());
    Ok(())
}

fn write_if_changed(path: &Path, content: &str) -> anyhow::Result<()> {
    if path.is_file() {
        let existing = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        if existing == content {
            return Ok(());
        }
    }

    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn install_godot_bridge_creates_files_and_is_idempotent() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("project.godot"), "config_version=5\n").unwrap();

        install_godot_bridge(dir.path()).unwrap();
        let plugin_cfg_path = dir.path().join("addons/lux_bridge/plugin.cfg");
        let bridge_gd_path = dir.path().join("addons/lux_bridge/bridge.gd");
        let first_plugin_cfg = fs::read_to_string(&plugin_cfg_path).unwrap();
        let first_bridge_gd = fs::read_to_string(&bridge_gd_path).unwrap();

        install_godot_bridge(dir.path()).unwrap();

        assert_eq!(
            fs::read_to_string(plugin_cfg_path).unwrap(),
            first_plugin_cfg
        );
        assert_eq!(fs::read_to_string(bridge_gd_path).unwrap(), first_bridge_gd);
    }

    #[test]
    fn install_godot_bridge_rejects_non_godot_project() {
        let dir = tempdir().unwrap();
        let err = install_godot_bridge(dir.path()).unwrap_err().to_string();
        assert!(err.contains("Godot 4 project not detected"));
    }
}
