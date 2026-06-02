use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub fn core_skills_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../Skills/skills")
}

pub fn project_skill_roots(project_root: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(project_root) = project_root {
        roots.push(project_root.join(".lux").join("skills"));
        roots.push(project_root.join(".agents").join("skills"));
    } else if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir.join(".lux").join("skills"));
        roots.push(current_dir.join(".agents").join("skills"));
    }
    roots
}

pub fn global_skill_roots() -> Vec<PathBuf> {
    let Some(home) = home_dir() else {
        return Vec::new();
    };
    vec![
        home.join(".lux").join("skills"),
        home.join(".agents").join("skills"),
    ]
}

pub fn plugin_skill_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(value) = std::env::var_os("LUX_SKILL_PLUGIN_ROOTS") {
        for root in std::env::split_paths(&value) {
            roots.push(normalize_plugin_skill_root(root));
        }
        return roots;
    }

    let Some(home) = home_dir() else {
        return roots;
    };
    roots.extend(glob_skill_roots(
        &home.join(".grok").join("installed-plugins"),
    ));
    roots.extend(glob_skill_roots(
        &home.join(".codex").join("plugins").join("cache"),
    ));
    roots
}

fn normalize_plugin_skill_root(root: PathBuf) -> PathBuf {
    if root.file_name().and_then(|name| name.to_str()) == Some("skills") {
        return root;
    }
    let plugin_manifest = root.join(".codex-plugin").join("plugin.json");
    if let Ok(manifest_json) = fs::read_to_string(&plugin_manifest) {
        if let Ok(manifest) = serde_json::from_str::<Value>(&manifest_json) {
            if let Some(skills) = manifest.get("skills").and_then(Value::as_str) {
                let skill_root = root.join(skills);
                if skill_root.is_dir() {
                    return skill_root.canonicalize().unwrap_or(skill_root);
                }
            }
        }
    }
    let skills_dir = root.join("skills");
    if skills_dir.is_dir() {
        skills_dir
    } else {
        root
    }
}

fn glob_skill_roots(base: &Path) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    collect_skill_roots(base, 0, &mut roots);
    roots
}

fn collect_skill_roots(base: &Path, depth: usize, roots: &mut Vec<PathBuf>) {
    if depth > 3 {
        return;
    }
    let Ok(read_dir) = fs::read_dir(base) else {
        return;
    };
    for dir_entry in read_dir.flatten() {
        let path = dir_entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("skills") {
            roots.push(path);
        } else {
            collect_skill_roots(&path, depth + 1, roots);
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    }
    .map(PathBuf::from)
}
