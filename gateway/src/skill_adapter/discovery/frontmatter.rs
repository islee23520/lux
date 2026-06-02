use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use super::SkillManifest;

pub fn read_skill_manifest(directory_path: &Path) -> Result<Option<SkillManifest>> {
    let manifest_path = directory_path.join("manifest.json");
    match fs::read_to_string(&manifest_path) {
        Ok(manifest_json) => {
            let mut manifest = serde_json::from_str::<SkillManifest>(&manifest_json)
                .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
            manifest.lazy_load = false;
            Ok(Some(manifest))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            synthesize_lazy_manifest(directory_path)
        }
        Err(error) => {
            Err(error).with_context(|| format!("failed to read {}", manifest_path.display()))
        }
    }
}

fn synthesize_lazy_manifest(directory_path: &Path) -> Result<Option<SkillManifest>> {
    let skill_md_path = directory_path.join("SKILL.md");
    let skill_md = match fs::read_to_string(&skill_md_path) {
        Ok(skill_md) => skill_md,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            eprintln!(
                "Warning: missing manifest.json and SKILL.md for skill directory {}",
                directory_path.display()
            );
            return Ok(None);
        }
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed to read {}", skill_md_path.display()))
        }
    };

    let frontmatter = parse_frontmatter(&skill_md)?;
    let fallback_name = directory_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
    let name = frontmatter
        .get("name")
        .cloned()
        .unwrap_or_else(|| fallback_name.clone());
    let category = frontmatter
        .get("category")
        .cloned()
        .or_else(|| category_from_name(&name));
    let description = frontmatter
        .get("description")
        .cloned()
        .or_else(|| first_heading_or_line(&skill_md))
        .unwrap_or_else(|| format!("Lazy-loaded skill {name}"));

    Ok(Some(SkillManifest {
        name,
        version: frontmatter
            .get("version")
            .cloned()
            .unwrap_or_else(|| "0.0.0".to_string()),
        description,
        display_name: frontmatter.get("displayName").cloned(),
        lux_version: None,
        author: None,
        keywords: None,
        skill_type: category
            .as_ref()
            .map(|_| "category".to_string())
            .unwrap_or_else(|| "reference".to_string()),
        source: None,
        dependencies: None,
        required_packages: None,
        compatible_render_pipelines: None,
        context_slim_rules: None,
        category,
        lazy_load: true,
    }))
}

fn parse_frontmatter(skill_md: &str) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    let mut lines = skill_md.lines();
    if lines.next() != Some("---") {
        return Ok(values);
    }
    for line in lines {
        if line == "---" {
            return Ok(values);
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"');
        if key.is_empty() || value.is_empty() {
            continue;
        }
        if matches!(value.as_bytes().first(), Some(b'[' | b'{' | b'&' | b'*')) {
            anyhow::bail!("frontmatter field '{key}' must be a scalar string");
        }
        values.insert(key.to_string(), value.to_string());
    }
    Ok(values)
}

fn category_from_name(name: &str) -> Option<String> {
    match name {
        "programming" => Some("programming".to_string()),
        _ => None,
    }
}

fn first_heading_or_line(skill_md: &str) -> Option<String> {
    skill_md
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim().trim_start_matches('#').trim();
            (!trimmed.is_empty() && trimmed != "---").then(|| trimmed.to_string())
        })
        .next()
}
