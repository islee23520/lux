use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Context, Result};
use chrono::{DateTime, SecondsFormat, Utc};
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    ai_log::{self, AiLogFilter},
    lux_io::atomic_write_json,
    lux_ticket::{
        FileTicketStore, Ticket, TicketFilter, TicketPriority, TicketStatus, TicketStore,
    },
};

const DEDUPE_THRESHOLD: f64 = 0.75;

/// Run automated triage pipeline on recent events.
#[derive(Parser, Debug, Clone)]
pub struct TriageArgs {
    /// Unity project root containing the .lux directory.
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Event source to ingest before classification.
    #[arg(long, value_enum, default_value_t = TriageSource::Console)]
    pub source: TriageSource,
    /// Lower bound timestamp for source readers, as RFC3339.
    #[arg(long)]
    pub since: Option<String>,
    /// Maximum events to ingest from the selected source.
    #[arg(long, default_value_t = 100)]
    pub limit: usize,
    /// Inspect pending events without creating or updating tickets.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriageSource {
    Console,
    AiLog,
    File,
}

/// Raw ingested event before classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriageEvent {
    pub id: String,
    pub source: String,
    pub raw_content: String,
    pub severity: TriageSeverity,
    pub domain_hint: Option<String>,
    pub file_paths: Vec<String>,
    pub timestamp: String,
    pub metadata: Value,
    pub classified: bool,
    pub matched_ticket_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TriageSeverity {
    Info,
    Warn,
    Error,
    Critical,
}

/// Classification result — maps event to domain + priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriageClassification {
    pub domain: String,
    pub suggested_priority: TicketPriority,
    pub confidence: f64,
    pub tags: Vec<String>,
    pub requires_human_review: bool,
    pub reason: String,
}

/// Triage pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriageConfig {
    pub auto_classify: bool,
    pub dedupe_window_secs: u64,
    pub min_confidence_for_auto: f64,
    pub max_events_per_ingest: usize,
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self {
            auto_classify: true,
            dedupe_window_secs: 7 * 24 * 60 * 60,
            min_confidence_for_auto: 0.65,
            max_events_per_ingest: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriageSummary {
    pub ingested: usize,
    pub classified: usize,
    pub duplicates_found: usize,
    pub tickets_created: Vec<String>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

pub fn run_triage_command(args: &TriageArgs) -> Result<()> {
    let started = Instant::now();
    let project_root = args
        .project_path
        .clone()
        .unwrap_or(std::env::current_dir()?)
        .canonicalize()
        .unwrap_or_else(|_| {
            args.project_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("."))
        });
    let lux_dir = project_root.join(".lux");
    let mut config = TriageConfig::default();
    config.max_events_per_ingest = args.limit;

    let mut summary = TriageSummary {
        ingested: 0,
        classified: 0,
        duplicates_found: 0,
        tickets_created: Vec::new(),
        errors: Vec::new(),
        duration_ms: 0,
    };

    let ingested = match args.source {
        TriageSource::Console => ingest_from_console_log(&lux_dir, &project_root, args.limit),
        TriageSource::AiLog => {
            ingest_from_ai_log(&lux_dir, args.since.as_deref().unwrap_or(""), args.limit)
        }
        TriageSource::File => Ok(Vec::new()),
    };
    match ingested {
        Ok(ids) => summary.ingested = ids.len(),
        Err(error) => summary.errors.push(error.to_string()),
    }

    let store = FileTicketStore::new(&project_root);
    let classified = batch_classify(&lux_dir, &config)?;
    summary.classified = classified.len();

    if !args.dry_run {
        for (event_id, classification) in classified {
            let event = read_event(&lux_dir, &event_id)?;
            match deduplicate_against_tickets(&classification, &event, &store, &config)? {
                Some(ticket_id) => {
                    mark_as_duplicate(&event.id, &ticket_id, &lux_dir)?;
                    summary.duplicates_found += 1;
                }
                None => {
                    let ticket = create_ticket_from_classification(
                        &event,
                        &classification,
                        &lux_dir,
                        &store,
                    )?;
                    summary.tickets_created.push(ticket.id);
                }
            }
        }
    }

    summary.duration_ms = started.elapsed().as_millis() as u64;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

pub fn ingest_event(config: &TriageConfig, event: TriageEvent, lux_dir: &Path) -> Result<String> {
    validate_event(config, &event)?;
    let path = event_path(lux_dir, &event.id);
    if path.exists() {
        bail!("triage event already exists: {}", event.id);
    }
    atomic_write_json(&path, &event)?;
    Ok(event.id)
}

pub fn ingest_batch(
    config: &TriageConfig,
    events: Vec<TriageEvent>,
    lux_dir: &Path,
) -> Result<Vec<String>> {
    if events.len() > config.max_events_per_ingest {
        bail!(
            "triage ingest batch has {} events, max is {}",
            events.len(),
            config.max_events_per_ingest
        );
    }
    events
        .into_iter()
        .map(|event| ingest_event(config, event, lux_dir))
        .collect()
}

pub fn ingest_from_ai_log(lux_dir: &Path, since: &str, limit: usize) -> Result<Vec<String>> {
    let config = TriageConfig {
        max_events_per_ingest: limit,
        ..TriageConfig::default()
    };
    let project_root = lux_dir.parent().unwrap_or(lux_dir);
    let log_path = ai_log::ensure_log_path(project_root)?;
    if !log_path.exists() {
        return Ok(Vec::new());
    }
    let since_time = parse_optional_rfc3339(since)?;
    let entries = ai_log::read_log_entries(
        &log_path,
        &AiLogFilter {
            limit: Some(limit),
            ..AiLogFilter::default()
        },
    )?;
    let events = entries
        .into_iter()
        .filter(|entry| is_after_since(&entry.timestamp, since_time.as_ref()))
        .map(|entry| {
            let raw_content =
                serde_json::to_string(&entry.value).unwrap_or_else(|_| entry.value.to_string());
            TriageEvent {
                id: Uuid::new_v4().to_string(),
                source: "ai-log".to_string(),
                raw_content: raw_content.clone(),
                severity: severity_from_content(&raw_content),
                domain_hint: None,
                file_paths: extract_file_paths(&raw_content),
                timestamp: normalize_timestamp(&entry.timestamp),
                metadata: json!({"lineNumber": entry.line_number, "sourcePath": log_path}),
                classified: false,
                matched_ticket_id: None,
            }
        })
        .collect();
    ingest_batch(&config, events, lux_dir)
}

pub fn ingest_from_console_log(
    lux_dir: &Path,
    project_path: &Path,
    limit: usize,
) -> Result<Vec<String>> {
    let config = TriageConfig {
        max_events_per_ingest: limit,
        ..TriageConfig::default()
    };
    let candidates = [
        project_path.join(".lux/logs/unity-console.log"),
        project_path.join(".lux/logs/console.log"),
        project_path.join("Library/Logs/Unity/Editor.log"),
    ];
    let Some(path) = candidates.iter().find(|path| path.exists()) else {
        return Ok(Vec::new());
    };
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read console log {}", path.display()))?;
    let events = content
        .lines()
        .rev()
        .filter(|line| is_triage_worthy(line))
        .take(limit)
        .map(|line| TriageEvent {
            id: Uuid::new_v4().to_string(),
            source: "unity-console".to_string(),
            raw_content: line.to_string(),
            severity: severity_from_content(line),
            domain_hint: None,
            file_paths: extract_file_paths(line),
            timestamp: now_rfc3339(),
            metadata: json!({"sourcePath": path}),
            classified: false,
            matched_ticket_id: None,
        })
        .collect();
    ingest_batch(&config, events, lux_dir)
}

pub fn ingest_from_compile_error(lux_dir: &Path, error_output: &str) -> Result<String> {
    let event = TriageEvent {
        id: Uuid::new_v4().to_string(),
        source: "compile-failure".to_string(),
        raw_content: error_output.to_string(),
        severity: TriageSeverity::Critical,
        domain_hint: Some("build".to_string()),
        file_paths: extract_file_paths(error_output),
        timestamp: now_rfc3339(),
        metadata: json!({"ingestKind": "compileError"}),
        classified: false,
        matched_ticket_id: None,
    };
    ingest_event(&TriageConfig::default(), event, lux_dir)
}

pub fn classify_event(event: &TriageEvent, config: &TriageConfig) -> Result<TriageClassification> {
    if !config.auto_classify {
        return Ok(TriageClassification {
            domain: event
                .domain_hint
                .clone()
                .unwrap_or_else(|| "testing".to_string()),
            suggested_priority: priority_from_severity(&event.severity),
            confidence: 0.0,
            tags: base_tags(event, "manual-review"),
            requires_human_review: true,
            reason: "auto classification disabled".to_string(),
        });
    }

    let content = event.raw_content.to_lowercase();
    let (domain, confidence) = classify_domain_from_keywords(&event.raw_content, &event.file_paths);
    let suggested_priority = if content.contains("compilation failed") {
        TicketPriority::Critical
    } else if content.contains("playmode") && content.contains("exception") {
        TicketPriority::High
    } else if (content.contains("null ref") || content.contains("nullreference"))
        && event.file_paths.iter().any(|path| path.ends_with(".cs"))
    {
        TicketPriority::High
    } else if content.contains("error")
        && event.file_paths.iter().any(|path| path.contains("Assets/"))
    {
        TicketPriority::Critical
    } else {
        priority_from_severity(&event.severity)
    };
    let requires_human_review = confidence < config.min_confidence_for_auto;
    let reason = format!(
        "keyword classification domain={domain} confidence={confidence:.2} severity={:?}",
        event.severity
    );

    Ok(TriageClassification {
        domain: domain.clone(),
        suggested_priority,
        confidence,
        tags: base_tags(event, &domain),
        requires_human_review,
        reason,
    })
}

pub fn classify_domain_from_keywords(content: &str, file_paths: &[String]) -> (String, f64) {
    let lower = content.to_lowercase();
    if lower.contains("compilation failed") || lower.contains("compiler error") {
        return ("build".to_string(), 0.95);
    }
    if lower.contains("playmode") && lower.contains("exception") {
        return ("testing".to_string(), 0.9);
    }
    if (lower.contains("null ref") || lower.contains("nullreference"))
        && file_paths.iter().any(|path| path.ends_with(".cs"))
    {
        return ("architecture".to_string(), 0.9);
    }
    if lower.contains("error") && file_paths.iter().any(|path| path.contains("Assets/")) {
        return ("art-style".to_string(), 0.85);
    }

    let rules: [(&str, &[&str]); 9] = [
        (
            "architecture",
            &["script", "component", ".cs", "nullreference", "dependency"],
        ),
        (
            "art-style",
            &["material", "texture", "sprite", "shader", "assets/"],
        ),
        ("audio", &["audio", "sound", "clip", "mixer"]),
        (
            "build",
            &["compile", "compiler", "build", "assembly", "package"],
        ),
        ("design", &["gameplay", "mechanic", "balance", "rule"]),
        ("levels", &["scene", "level", "terrain", "prefab"]),
        ("narrative", &["dialog", "story", "quest", "localization"]),
        (
            "testing",
            &["test", "assert", "playmode", "editmode", "exception"],
        ),
        ("ui-ux", &["canvas", "button", "panel", "ui", "ux"]),
    ];

    let mut scored: Vec<_> = rules
        .iter()
        .map(|(domain, keywords)| {
            let hits = keywords
                .iter()
                .filter(|keyword| lower.contains(**keyword))
                .count();
            (*domain, hits, keywords.len())
        })
        .filter(|(_, hits, _)| *hits > 0)
        .collect();
    scored.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(right.0)));

    if let Some((domain, hits, total)) = scored.first() {
        let confidence = 0.45 + ((*hits as f64 / *total as f64).min(1.0) * 0.4);
        ((*domain).to_string(), confidence.min(0.84))
    } else {
        ("testing".to_string(), 0.35)
    }
}

pub fn batch_classify(
    lux_dir: &Path,
    config: &TriageConfig,
) -> Result<Vec<(String, TriageClassification)>> {
    let mut results = Vec::new();
    for mut event in list_unclassified_events(lux_dir)? {
        let classification = classify_event(&event, config)?;
        event.classified = true;
        atomic_write_json(&event_path(lux_dir, &event.id), &event)?;
        results.push((event.id, classification));
    }
    Ok(results)
}

pub fn deduplicate_against_tickets(
    classification: &TriageClassification,
    event: &TriageEvent,
    store: &dyn TicketStore,
    config: &TriageConfig,
) -> Result<Option<String>> {
    let title = ticket_title(event);
    let event_time = DateTime::parse_from_rfc3339(&event.timestamp)
        .with_context(|| format!("invalid event timestamp: {}", event.timestamp))?;
    let tickets = store.list(TicketFilter::default())?;
    let mut best: Option<(String, f64)> = None;
    for ticket in tickets {
        if ticket.status == TicketStatus::Done {
            continue;
        }
        if !ticket
            .tags
            .iter()
            .any(|tag| tag == &format!("domain:{}", classification.domain))
        {
            continue;
        }
        let ticket_time = DateTime::parse_from_rfc3339(&ticket.updated_at)
            .with_context(|| format!("invalid ticket updated_at: {}", ticket.updated_at))?;
        let age = (event_time.timestamp() - ticket_time.timestamp()).unsigned_abs();
        if age > config.dedupe_window_secs {
            continue;
        }
        let ticket_files = extract_file_paths(&ticket.description);
        let score = compute_similarity(&title, &ticket.title, &event.file_paths, &ticket_files);
        if score >= DEDUPE_THRESHOLD
            && best
                .as_ref()
                .is_none_or(|(_, best_score)| score > *best_score)
        {
            best = Some((ticket.id, score));
        }
    }
    Ok(best.map(|(ticket_id, _)| ticket_id))
}

pub fn compute_similarity(
    title_a: &str,
    title_b: &str,
    files_a: &[String],
    files_b: &[String],
) -> f64 {
    let title_ratio = levenshtein_ratio(title_a, title_b);
    let file_ratio = jaccard(files_a, files_b);
    (file_ratio * 0.6) + (title_ratio * 0.4)
}

pub fn mark_as_duplicate(event_id: &str, ticket_id: &str, lux_dir: &Path) -> Result<()> {
    let mut event = read_event(lux_dir, event_id)?;
    event.matched_ticket_id = Some(ticket_id.to_string());
    atomic_write_json(&event_path(lux_dir, event_id), &event)
}

pub fn create_ticket_from_classification(
    event: &TriageEvent,
    classification: &TriageClassification,
    lux_dir: &Path,
    store: &dyn TicketStore,
) -> Result<Ticket> {
    let now = now_rfc3339();
    let mut tags = classification.tags.clone();
    if !tags.iter().any(|tag| tag == "auto-triaged") {
        tags.push("auto-triaged".to_string());
    }
    if classification.requires_human_review {
        tags.push("human-review".to_string());
    }
    tags.sort();
    tags.dedup();

    let ticket = Ticket {
        id: Uuid::new_v4().to_string(),
        title: ticket_title(event),
        description: ticket_description(event, classification),
        status: TicketStatus::Backlog,
        priority: classification.suggested_priority.clone(),
        assignee: None,
        blockers: Vec::new(),
        tags,
        spec_ref: Some(classification.domain.clone()),
        created_at: now.clone(),
        updated_at: now,
        execution_objective: None,
        allowed_executor: None,
        dispatch_policy: None,
        verification_policy: None,
        command_allowlist: None,
        evidence_refs: None,
        blocker_policy: None,
        non_goals: None,
    };
    let created = store.create(ticket)?;
    let mut updated_event = event.clone();
    updated_event.matched_ticket_id = Some(created.id.clone());
    atomic_write_json(&event_path(lux_dir, &event.id), &updated_event)?;
    Ok(created)
}

pub fn run_triage_pipeline(
    lux_dir: &Path,
    project_path: &Path,
    config: &TriageConfig,
) -> Result<TriageSummary> {
    let started = Instant::now();
    let mut summary = TriageSummary {
        ingested: 0,
        classified: 0,
        duplicates_found: 0,
        tickets_created: Vec::new(),
        errors: Vec::new(),
        duration_ms: 0,
    };
    match ingest_from_ai_log(lux_dir, "", config.max_events_per_ingest) {
        Ok(ids) => summary.ingested += ids.len(),
        Err(error) => summary
            .errors
            .push(format!("ai-log ingest failed: {error}")),
    }
    match ingest_from_console_log(lux_dir, project_path, config.max_events_per_ingest) {
        Ok(ids) => summary.ingested += ids.len(),
        Err(error) => summary
            .errors
            .push(format!("console ingest failed: {error}")),
    }
    let store = FileTicketStore::new(project_path);
    let classified = batch_classify(lux_dir, config)?;
    summary.classified = classified.len();
    for (event_id, classification) in classified {
        let event = read_event(lux_dir, &event_id)?;
        if let Some(ticket_id) =
            deduplicate_against_tickets(&classification, &event, &store, config)?
        {
            mark_as_duplicate(&event_id, &ticket_id, lux_dir)?;
            summary.duplicates_found += 1;
        } else {
            let ticket =
                create_ticket_from_classification(&event, &classification, lux_dir, &store)?;
            summary.tickets_created.push(ticket.id);
        }
    }
    summary.duration_ms = started.elapsed().as_millis() as u64;
    Ok(summary)
}

pub fn list_unclassified_events(lux_dir: &Path) -> Result<Vec<TriageEvent>> {
    let dir = lux_dir.join("triage/events");
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut events = Vec::new();
    for entry in fs::read_dir(&dir)
        .with_context(|| format!("failed to read triage events directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let event: TriageEvent = serde_json::from_str(&fs::read_to_string(&path)?)
            .with_context(|| format!("failed to parse triage event {}", path.display()))?;
        if !event.classified {
            events.push(event);
        }
    }
    events.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then(left.id.cmp(&right.id))
    });
    Ok(events)
}

fn validate_event(config: &TriageConfig, event: &TriageEvent) -> Result<()> {
    if config.max_events_per_ingest == 0 {
        bail!("max_events_per_ingest must be greater than zero");
    }
    if Uuid::parse_str(&event.id).is_err() {
        bail!("triage event id must be a UUID: {}", event.id);
    }
    if event.source.trim().is_empty() {
        bail!("triage event source is required");
    }
    if event.raw_content.trim().is_empty() {
        bail!("triage event raw_content is required");
    }
    DateTime::parse_from_rfc3339(&event.timestamp)
        .with_context(|| format!("invalid triage event timestamp: {}", event.timestamp))?;
    Ok(())
}

fn event_path(lux_dir: &Path, event_id: &str) -> PathBuf {
    lux_dir
        .join("triage/events")
        .join(format!("{event_id}.json"))
}

fn read_event(lux_dir: &Path, event_id: &str) -> Result<TriageEvent> {
    let path = event_path(lux_dir, event_id);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read triage event {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse triage event {}", path.display()))
}

fn ticket_title(event: &TriageEvent) -> String {
    let mut title = event
        .raw_content
        .lines()
        .next()
        .unwrap_or("triage event")
        .trim()
        .chars()
        .take(96)
        .collect::<String>();
    if title.is_empty() {
        title = format!("{} event", event.source);
    }
    format!("[triage:{}] {title}", event.source)
}

fn ticket_description(event: &TriageEvent, classification: &TriageClassification) -> String {
    format!(
        "Auto-triaged from event {event_id}\n\nDomain: {domain}\nConfidence: {confidence:.2}\nReason: {reason}\nAffected files: {files}\n\nRaw content:\n{raw}",
        event_id = event.id,
        domain = classification.domain,
        confidence = classification.confidence,
        reason = classification.reason,
        files = event.file_paths.join(", "),
        raw = event.raw_content
    )
}

fn base_tags(event: &TriageEvent, domain: &str) -> Vec<String> {
    let mut tags = vec![
        "auto-triaged".to_string(),
        format!("source:{}", event.source),
        format!("severity:{}", severity_label(&event.severity)),
        format!("domain:{domain}"),
    ];
    if let Some(hint) = &event.domain_hint {
        tags.push(format!("domain-hint:{hint}"));
    }
    tags
}

fn priority_from_severity(severity: &TriageSeverity) -> TicketPriority {
    match severity {
        TriageSeverity::Critical => TicketPriority::Critical,
        TriageSeverity::Error => TicketPriority::High,
        TriageSeverity::Warn => TicketPriority::Medium,
        TriageSeverity::Info => TicketPriority::Low,
    }
}

fn severity_from_content(content: &str) -> TriageSeverity {
    let lower = content.to_lowercase();
    if lower.contains("critical") || lower.contains("fatal") || lower.contains("compilation failed")
    {
        TriageSeverity::Critical
    } else if lower.contains("error") || lower.contains("exception") || lower.contains("failed") {
        TriageSeverity::Error
    } else if lower.contains("warn") {
        TriageSeverity::Warn
    } else {
        TriageSeverity::Info
    }
}

fn severity_label(severity: &TriageSeverity) -> &'static str {
    match severity {
        TriageSeverity::Info => "info",
        TriageSeverity::Warn => "warn",
        TriageSeverity::Error => "error",
        TriageSeverity::Critical => "critical",
    }
}

fn extract_file_paths(content: &str) -> Vec<String> {
    let mut files = Vec::new();
    for raw in
        content.split(|c: char| c.is_whitespace() || c == '(' || c == ')' || c == ',' || c == ';')
    {
        let token =
            raw.trim_matches(|c: char| c == '"' || c == '\'' || c == ':' || c == '[' || c == ']');
        if token.contains('/')
            && (token.contains("Assets/")
                || token.ends_with(".cs")
                || token.ends_with(".prefab")
                || token.ends_with(".unity")
                || token.ends_with(".asset"))
        {
            files.push(token.to_string());
        }
    }
    files.sort();
    files.dedup();
    files
}

fn is_triage_worthy(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("error")
        || lower.contains("exception")
        || lower.contains("failed")
        || lower.contains("warn")
}

fn normalize_timestamp(timestamp: &str) -> String {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|value| value.to_rfc3339_opts(SecondsFormat::Millis, true))
        .unwrap_or_else(|_| now_rfc3339())
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn parse_optional_rfc3339(value: &str) -> Result<Option<DateTime<chrono::FixedOffset>>> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(DateTime::parse_from_rfc3339(value).with_context(
        || format!("invalid --since timestamp: {value}"),
    )?))
}

fn is_after_since(timestamp: &str, since: Option<&DateTime<chrono::FixedOffset>>) -> bool {
    let Some(since) = since else {
        return true;
    };
    DateTime::parse_from_rfc3339(timestamp).is_ok_and(|value| value >= *since)
}

fn jaccard(files_a: &[String], files_b: &[String]) -> f64 {
    let left: HashSet<_> = files_a.iter().map(|path| path.to_lowercase()).collect();
    let right: HashSet<_> = files_b.iter().map(|path| path.to_lowercase()).collect();
    if left.is_empty() && right.is_empty() {
        return 0.0;
    }
    let intersection = left.intersection(&right).count() as f64;
    let union = left.union(&right).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn levenshtein_ratio(a: &str, b: &str) -> f64 {
    let max_len = a.chars().count().max(b.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    1.0 - (levenshtein(a, b) as f64 / max_len as f64)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        current[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}
