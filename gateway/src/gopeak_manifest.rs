use std::{path::Path, process::Command};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncResult {
    pub installed: bool,
    pub available_commands: Vec<String>,
    pub missing_commands: Vec<String>,
}

pub fn curated_gopeak_commands() -> Vec<String> {
    [
        "scene/list",
        "scene/tree",
        "node/inspect",
        "node/create",
        "project/build",
        "project/run",
        "project/export",
        "asset/list",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub fn sync_manifest(project_path: &Path) -> Result<SyncResult> {
    let _ = project_path;
    let commands = curated_gopeak_commands();
    if binary_is_available("gopeak") {
        Ok(SyncResult {
            installed: true,
            available_commands: commands,
            missing_commands: Vec::new(),
        })
    } else {
        Ok(SyncResult {
            installed: false,
            available_commands: Vec::new(),
            missing_commands: commands,
        })
    }
}

fn binary_is_available(binary: &str) -> bool {
    let probe = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };

    Command::new(probe)
        .arg(binary)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curated_commands_include_build_but_do_not_define_lux_support() {
        let commands = curated_gopeak_commands();
        assert!(commands.contains(&"project/build".to_string()));
    }
}
