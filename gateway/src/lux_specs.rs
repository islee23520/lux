use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;

#[derive(Serialize)]
struct SpecsMigrationReceipt {
    canonical_root: &'static str,
    migrated_at: String,
    sources: Vec<String>,
}

pub fn ensure_specs_contract(
    project_path: &Path,
    legacy_spec_path: &Path,
    legacy_domains_path: &Path,
) -> Result<()> {
    let specs_path = project_path.join(".lux/specs");
    let specs_domains_path = specs_path.join("domains");
    fs::create_dir_all(&specs_domains_path)
        .with_context(|| format!("failed to create {}", specs_domains_path.display()))?;

    copy_legacy_spec(legacy_spec_path, &specs_path.join("spec.json"))?;
    migrate_legacy_domains(legacy_domains_path, &specs_domains_path)?;
    ensure_file(
        &specs_path.join("gdd.md"),
        "# Game Design Document\n\nCanonical LUX game design document.\n",
    )?;
    ensure_file(&specs_path.join("decisions.jsonl"), "")?;
    ensure_file(
        &specs_path.join("preferences.json"),
        "{\n  \"inferred\": [],\n  \"explicit\": []\n}\n",
    )?;
    write_migration_receipt(&specs_path, legacy_spec_path, legacy_domains_path)
}

fn copy_legacy_spec(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        return Ok(());
    }

    fs::copy(source, destination).with_context(|| {
        format!(
            "failed to migrate {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn migrate_legacy_domains(source_directory: &Path, destination_directory: &Path) -> Result<()> {
    for entry in fs::read_dir(source_directory)
        .with_context(|| format!("failed to read {}", source_directory.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read {}", source_directory.display()))?;
        let source = entry.path();
        if source.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        let destination = destination_directory.join(entry.file_name());
        if destination.exists() {
            continue;
        }
        fs::copy(&source, &destination).with_context(|| {
            format!(
                "failed to migrate {} to {}",
                source.display(),
                destination.display()
            )
        })?;
    }

    Ok(())
}

fn ensure_file(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn write_migration_receipt(
    specs_path: &Path,
    legacy_spec_path: &Path,
    legacy_domains_path: &Path,
) -> Result<()> {
    let receipt = SpecsMigrationReceipt {
        canonical_root: ".lux/specs",
        migrated_at: Utc::now().to_rfc3339(),
        sources: migration_sources(legacy_spec_path, legacy_domains_path)?,
    };
    let receipt_json =
        serde_json::to_string_pretty(&receipt).context("failed to serialize specs migration")?;
    fs::write(specs_path.join("migration.json"), receipt_json).with_context(|| {
        format!(
            "failed to write {}",
            specs_path.join("migration.json").display()
        )
    })
}

fn migration_sources(legacy_spec_path: &Path, legacy_domains_path: &Path) -> Result<Vec<String>> {
    let mut sources = vec![relative_lux_path(legacy_spec_path)];
    for entry in fs::read_dir(legacy_domains_path)
        .with_context(|| format!("failed to read {}", legacy_domains_path.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read {}", legacy_domains_path.display()))?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("md") {
            sources.push(relative_lux_path(&path));
        }
    }
    sources.sort();
    Ok(sources)
}

fn relative_lux_path(path: &Path) -> String {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>();
    if let Some(index) = components.iter().position(|component| component == ".lux") {
        return components[index..].join("/");
    }

    PathBuf::from(path).display().to_string()
}
