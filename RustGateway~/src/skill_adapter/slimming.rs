//! Context slimming logic for skill adaptation.

use crate::ai_log::{AiLogEntry, build_continuation_context};
use crate::skill_adapter::metadata::{AdaptationProjectMetadata, SkillContextSlimRules};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

const DEFAULT_MAX_TOKENS: usize = 4096;

/// Reduce an AI event log to essential information sized for AI context injection.
///
/// Takes raw AiLogEntry slices and produces a compact JSON value containing only
/// the fields relevant to skill adaptation: timestamps, actors, categories,
/// summaries, and a token-budget-aware truncation.
pub fn slim_context_for_adaptation(event_entries: &[AiLogEntry], max_tokens: usize) -> Value {
    let budget = if max_tokens == 0 {
        DEFAULT_MAX_TOKENS
    } else {
        max_tokens
    };

    let context = build_continuation_context(event_entries, None);
    let count = context["count"].as_u64().unwrap_or(0) as usize;

    let max_entries = budget / 4;
    let truncated = if count > max_entries {
        let mut entries = context["entries"].as_array().cloned().unwrap_or_default();
        let keep_from = entries.len().saturating_sub(max_entries);
        entries = entries.split_off(keep_from);
        entries
    } else {
        context["entries"].as_array().cloned().unwrap_or_default()
    };

    let original_count = event_entries.len();
    let dropped = original_count.saturating_sub(truncated.len());

    json!({
        "schemaVersion": 1,
        "totalEntries": original_count,
        "slimmedEntries": truncated.len(),
        "droppedEntries": dropped,
        "maxTokens": budget,
        "entries": truncated,
    })
}

/// Compute context slimming metrics from a skill source directory and manifest.
///
/// Analyzes references/, SKILL.md line count, and contextSlimRules from the
/// manifest to produce a structured report of what was kept/excluded/slimmed.
pub(crate) fn compute_context_slimming(
    source_dir: &Path,
    manifest: &Value,
    project_metadata: &AdaptationProjectMetadata,
) -> Value {
    let slim_rules: Option<SkillContextSlimRules> = manifest
        .get("contextSlimRules")
        .or_else(|| manifest.get("context_slim_rules"))
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let references_dir = source_dir.join("references");
    let total_references = if references_dir.is_dir() {
        fs::read_dir(&references_dir)
            .map(|entries| entries.flatten().count())
            .unwrap_or(0)
    } else {
        0
    };

    let skill_md_path = source_dir.join("SKILL.md");
    let total_skill_md_lines = fs::read_to_string(&skill_md_path)
        .map(|content| content.lines().count())
        .unwrap_or(0);

    let max_references = slim_rules
        .as_ref()
        .and_then(|r| r.max_references)
        .unwrap_or(usize::MAX);
    let references_slashed = total_references > max_references;

    let max_skill_md_lines = slim_rules
        .as_ref()
        .and_then(|r| r.max_skill_md_lines)
        .unwrap_or(usize::MAX);
    let slimmed_skill_md_lines = total_skill_md_lines.min(max_skill_md_lines);
    let skill_md_slashed = total_skill_md_lines > max_skill_md_lines;

    let excluded_tags: Vec<String> = slim_rules
        .as_ref()
        .and_then(|r| r.exclude_tags.clone())
        .unwrap_or_default();

    let mut included_references: Vec<String> = Vec::new();
    let mut excluded_references: Vec<String> = Vec::new();
    if references_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&references_dir) {
            let mut all_refs: Vec<String> = entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
                .filter(|name| name.ends_with(".md"))
                .collect();
            all_refs.sort();

            for reference_name in &all_refs {
                let name_lower = reference_name.to_lowercase();
                let should_exclude = excluded_tags
                    .iter()
                    .any(|tag| name_lower.contains(&tag.to_lowercase()));
                if should_exclude {
                    excluded_references.push(reference_name.clone());
                } else {
                    included_references.push(reference_name.clone());
                }
            }
        }
    }

    if included_references.len() > max_references {
        let excess = included_references.len() - max_references;
        let drained: Vec<String> =
            included_references.split_off(included_references.len() - excess);
        excluded_references.extend(drained);
    }

    json!({
        "totalReferences": total_references,
        "slimmedReferences": included_references.len(),
        "referencesSlashed": references_slashed || included_references.len() != total_references,
        "totalSkillMdLines": total_skill_md_lines,
        "slimmedSkillMdLines": slimmed_skill_md_lines,
        "skillMdSlashed": skill_md_slashed,
        "excludedTags": excluded_tags,
        "includedReferences": included_references,
        "excludedReferences": excluded_references,
        "projectRenderPipeline": project_metadata.render_pipeline,
    })
}
