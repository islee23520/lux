use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
struct SpecsMigrationReceipt {
    canonical_root: &'static str,
    migrated_at: String,
    sources: Vec<String>,
}

const CANONICAL_DOMAIN_FILES: &[(&str, &str)] = &[
    ("gdd.md", include_str!("templates/gdd.md")),
    ("mechanics.md", include_str!("templates/mechanics.md")),
    ("controls.md", include_str!("templates/controls.md")),
    ("camera.md", include_str!("templates/camera.md")),
    ("art-style.md", include_str!("templates/art-style.md")),
    ("audio.md", include_str!("templates/audio.md")),
    ("narrative.md", include_str!("templates/narrative.md")),
    ("levels.md", include_str!("templates/levels.md")),
    (
        "technical-architecture.md",
        include_str!("templates/technical-architecture.md"),
    ),
    ("engine.md", include_str!("templates/engine.md")),
    ("testing.md", include_str!("templates/testing.md")),
    (
        "build-release.md",
        include_str!("templates/build-release.md"),
    ),
    ("ui-ux.md", include_str!("templates/ui-ux.md")),
];

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
    ensure_canonical_domain_files(&specs_domains_path)?;
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionEventKind {
    QuestionAnswered,
    ProposalApproved,
    ProposalRejected,
    ProposalApplied,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionLedgerEvent {
    pub id: String,
    pub kind: DecisionEventKind,
    pub created_at: String,
    pub run_id: Option<String>,
    pub question_id: Option<String>,
    pub proposal_id: Option<String>,
    pub domain: Option<String>,
    pub text: Option<String>,
    pub answer: Option<String>,
    pub rationale: Option<String>,
    pub source_question: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PreferenceEntry {
    pub domain: String,
    pub value: String,
    pub count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PreferenceConflict {
    pub domain: String,
    pub values: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PreferencesDocument {
    pub inferred: Vec<PreferenceEntry>,
    pub explicit: Vec<PreferenceEntry>,
    pub conflicts: Vec<PreferenceConflict>,
}

pub fn append_decision_event(project_path: &Path, event: &DecisionLedgerEvent) -> Result<()> {
    let specs_path = project_path.join(".lux/specs");
    let decisions_path = specs_path.join("decisions.jsonl");
    crate::lux_io::append_jsonl(&decisions_path, event)
        .with_context(|| format!("failed to append {}", decisions_path.display()))?;
    refresh_preferences(&specs_path)
}

pub fn refresh_preferences(specs_path: &Path) -> Result<()> {
    let decisions_path = specs_path.join("decisions.jsonl");
    let events = crate::lux_io::read_jsonl::<DecisionLedgerEvent>(&decisions_path)
        .with_context(|| format!("failed to read {}", decisions_path.display()))?;
    let mut explicit: BTreeMap<String, u32> = BTreeMap::new();
    let mut per_domain_values: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();

    for event in events {
        if !matches!(event.kind, DecisionEventKind::QuestionAnswered) {
            continue;
        }
        let Some(domain) = event.domain else {
            continue;
        };
        let Some(answer) = event.answer else {
            continue;
        };
        let key = format!("{domain}::{answer}");
        *explicit.entry(key).or_insert(0) += 1;
        let domain_values = per_domain_values.entry(domain).or_default();
        *domain_values.entry(answer).or_insert(0) += 1;
    }

    let mut inferred = Vec::new();
    let mut conflicts = Vec::new();
    for (domain, values) in per_domain_values {
        let unique_values: BTreeSet<_> = values.keys().cloned().collect();
        if unique_values.len() == 1 {
            let value = unique_values.into_iter().next().unwrap_or_default();
            let count = values.get(&value).copied().unwrap_or(0);
            if count > 1 {
                inferred.push(PreferenceEntry {
                    domain,
                    value,
                    count,
                });
            }
        } else if unique_values.len() > 1 {
            conflicts.push(PreferenceConflict {
                domain,
                values: unique_values.into_iter().collect(),
            });
        }
    }

    let explicit = explicit
        .into_iter()
        .map(|(key, count)| {
            let (domain, value) = key.split_once("::").unwrap_or((key.as_str(), ""));
            PreferenceEntry {
                domain: domain.to_string(),
                value: value.to_string(),
                count,
            }
        })
        .collect();

    let document = PreferencesDocument {
        inferred,
        explicit,
        conflicts,
    };
    crate::lux_io::atomic_write_json(&specs_path.join("preferences.json"), &document).with_context(
        || {
            format!(
                "failed to write {}",
                specs_path.join("preferences.json").display()
            )
        },
    )
}

fn atomic_write_text(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to atomically replace {}", path.display()))
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
    let mut entries = fs::read_dir(source_directory)
        .with_context(|| format!("failed to read {}", source_directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read {}", source_directory.display()))?;
    entries.sort_by_key(|entry| domain_migration_priority(&entry.file_name()));

    for entry in entries {
        let source = entry.path();
        if source.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        let file_name = entry.file_name();
        let destination_name = canonical_domain_file_name(&file_name);
        let destination = destination_directory.join(&destination_name);
        if destination.exists() && destination_name == file_name.to_string_lossy() {
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

fn ensure_canonical_domain_files(destination_directory: &Path) -> Result<()> {
    for (file_name, template) in CANONICAL_DOMAIN_FILES {
        ensure_file(&destination_directory.join(file_name), template)?;
    }
    Ok(())
}

fn canonical_domain_file_name(file_name: &std::ffi::OsStr) -> String {
    match file_name.to_string_lossy().as_ref() {
        "design.md" => "gdd.md".to_string(),
        "architecture.md" => "technical-architecture.md".to_string(),
        "packages.md" => "engine.md".to_string(),
        other => other.to_string(),
    }
}

fn domain_migration_priority(file_name: &std::ffi::OsStr) -> u8 {
    match file_name.to_string_lossy().as_ref() {
        "design.md" | "architecture.md" | "packages.md" => 0,
        _ => 1,
    }
}

fn ensure_file(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    atomic_write_text(path, content).with_context(|| format!("failed to write {}", path.display()))
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
    let receipt_path = specs_path.join("migration.json");
    crate::lux_io::atomic_write_json(&receipt_path, &receipt)
        .with_context(|| format!("failed to write {}", receipt_path.display()))
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
