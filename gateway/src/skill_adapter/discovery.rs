use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::skill_adapter::metadata::SkillContextSlimRules;

mod content;
mod frontmatter;
mod roots;

pub use content::{read_skill_md_preview, read_skill_references};
pub use roots::{core_skills_dir, global_skill_roots, plugin_skill_roots, project_skill_roots};

#[derive(Debug, Deserialize, Serialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "luxVersion")]
    pub lux_version: Option<String>,
    pub author: Option<SkillAuthor>,
    pub keywords: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub skill_type: String,
    pub source: Option<String>,
    pub dependencies: Option<Value>,
    #[serde(default, rename = "requiredPackages")]
    pub required_packages: Option<Vec<String>>,
    #[serde(default, rename = "compatibleRenderPipelines")]
    pub compatible_render_pipelines: Option<Vec<String>>,
    #[serde(default, rename = "contextSlimRules")]
    pub context_slim_rules: Option<SkillContextSlimRules>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, rename = "lazyLoad")]
    pub lazy_load: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SkillAuthor {
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SkillEntry {
    pub manifest: SkillManifest,
    pub directory_path: PathBuf,
    pub scope: String,
}

pub fn discover_skills(project_root: Option<&Path>) -> Result<Vec<SkillEntry>> {
    let mut entries = Vec::new();

    scan_scope_roots("core", &[core_skills_dir()], &mut entries)?;
    scan_scope_roots("project", &project_skill_roots(project_root), &mut entries)?;
    scan_scope_roots("global", &global_skill_roots(), &mut entries)?;
    scan_scope_roots("plugin", &plugin_skill_roots(), &mut entries)?;

    entries.sort_by(|left, right| {
        left.manifest
            .name
            .cmp(&right.manifest.name)
            .then_with(|| left.scope.cmp(&right.scope))
            .then_with(|| left.directory_path.cmp(&right.directory_path))
    });
    Ok(entries)
}

pub fn scan_scope_roots(
    scope: &str,
    roots: &[PathBuf],
    entries: &mut Vec<SkillEntry>,
) -> Result<()> {
    let mut visited = BTreeSet::new();
    for root in roots {
        let canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
        if !visited.insert(canonical) {
            continue;
        }
        scan_skill_scope(root, scope, entries)?;
    }
    Ok(())
}

fn scan_skill_scope(root: &Path, scope: &str, entries: &mut Vec<SkillEntry>) -> Result<()> {
    scan_skill_scope_at(root, scope, entries, 0)
}

fn scan_skill_scope_at(
    root: &Path,
    scope: &str,
    entries: &mut Vec<SkillEntry>,
    depth: usize,
) -> Result<()> {
    let read_dir = match fs::read_dir(root) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read skills directory {}", root.display()))
        }
    };

    for dir_entry in read_dir {
        let dir_entry = match dir_entry {
            Ok(dir_entry) => dir_entry,
            Err(error) => {
                eprintln!("Warning: failed to read skill directory entry: {error}");
                continue;
            }
        };
        let directory_path = dir_entry.path();
        if !directory_path.is_dir() {
            continue;
        }

        if is_skill_directory(&directory_path) {
            match frontmatter::read_skill_manifest(&directory_path) {
                Ok(Some(manifest)) => entries.push(SkillEntry {
                    manifest,
                    directory_path,
                    scope: scope.to_string(),
                }),
                Ok(None) => continue,
                Err(error) => {
                    eprintln!(
                        "Warning: failed to discover skill directory {}: {error}",
                        directory_path.display()
                    );
                }
            }
            continue;
        }

        if depth < 4 {
            scan_skill_scope_at(&directory_path, scope, entries, depth + 1)?;
        }
    }

    Ok(())
}

fn is_skill_directory(directory_path: &Path) -> bool {
    directory_path.join("manifest.json").is_file() || directory_path.join("SKILL.md").is_file()
}

#[cfg(test)]
mod tests {
    use super::scan_scope_roots;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir_with(prefix: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lux-skill-discovery-{prefix}-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn scan_scope_roots_discovers_skills_below_category_directories() {
        let root = temp_dir_with("categorized");
        let skill_dir = root.join("architecture").join("architecture-review");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            concat!(
                "---\n",
                "name: architecture-review\n",
                "description: Review architecture boundaries.\n",
                "category: architecture\n",
                "source: lux\n",
                "---\n",
                "# Architecture Review\n",
            ),
        )
        .unwrap();

        let mut entries = Vec::new();
        scan_scope_roots("core", &[root.clone()], &mut entries).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].manifest.name, "architecture-review");
        assert_eq!(
            entries[0].manifest.category.as_deref(),
            Some("architecture")
        );
        assert_eq!(entries[0].directory_path, skill_dir);

        fs::remove_dir_all(root).ok();
    }
}
