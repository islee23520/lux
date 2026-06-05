use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const DEFAULT_CODEX_EVENTS: &[&str] = &["UserPromptSubmit"];

#[derive(Debug, Clone, Parser)]
pub struct HooksArgs {
    #[command(subcommand)]
    pub action: HooksAction,
}

#[derive(Debug, Clone, Subcommand)]
pub enum HooksAction {
    /// Install the Lux Codex native hook bridge
    Install(HooksInstallArgs),
    /// Show whether the Lux Codex native hook bridge is installed
    Status(HooksStatusArgs),
    /// Run the Lux hook bridge for a native hook event
    Run(HooksRunArgs),
}

#[derive(Debug, Clone, Parser)]
pub struct HooksInstallArgs {
    /// Unity project root that owns the .lux runtime state
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Path to Codex hooks.json; defaults to CODEX_HOME/hooks.json or ~/.codex/hooks.json
    #[arg(long)]
    pub hooks_path: Option<PathBuf>,
    /// Print the planned hook changes without writing hooks.json
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
    /// Replace an existing Lux hook command for this project
    #[arg(long, default_value_t = false)]
    pub force: bool,
    /// Print machine-readable JSON
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct HooksStatusArgs {
    /// Unity project root that owns the .lux runtime state
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Path to Codex hooks.json; defaults to CODEX_HOME/hooks.json or ~/.codex/hooks.json
    #[arg(long)]
    pub hooks_path: Option<PathBuf>,
    /// Print machine-readable JSON
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct HooksRunArgs {
    /// Native hook event name, such as UserPromptSubmit
    #[arg(long)]
    pub event: String,
    /// Unity project root that owns the .lux runtime state
    #[arg(long)]
    pub project_path: Option<PathBuf>,
    /// Print machine-readable JSON
    #[arg(long, default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInstallReport {
    pub hooks_path: PathBuf,
    pub project_path: PathBuf,
    pub dry_run: bool,
    pub changed: bool,
    pub installed: Vec<HookEventInstallReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEventInstallReport {
    pub event: String,
    pub command: String,
    pub already_installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookStatusReport {
    pub hooks_path: PathBuf,
    pub project_path: PathBuf,
    pub events: Vec<HookEventStatusReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEventStatusReport {
    pub event: String,
    pub installed: bool,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRunReport {
    pub event_id: String,
    pub event: String,
    pub project_path: PathBuf,
    pub event_log_path: PathBuf,
    pub ulw_detected: bool,
    pub omx_ultrawork: OmxUltraworkStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OmxUltraworkStatus {
    Active {
        state_path: PathBuf,
        reinforcement_count: Option<u64>,
    },
    Inactive {
        state_path: PathBuf,
        reinforcement_count: Option<u64>,
    },
    NotFound,
    Invalid {
        state_path: PathBuf,
        error: String,
    },
}

pub fn run_hooks_command(args: HooksArgs) -> Result<()> {
    match args.action {
        HooksAction::Install(install_args) => {
            let report = install_codex_hook_bridge(&install_args)?;
            if install_args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else if report.dry_run {
                eprintln!(
                    "Lux hook install preview for {}:",
                    report.hooks_path.display()
                );
                for event in &report.installed {
                    let marker = if event.already_installed {
                        "already installed"
                    } else {
                        "would install"
                    };
                    eprintln!("  {}: {marker}", event.event);
                }
            } else {
                let changed = if report.changed {
                    "updated"
                } else {
                    "unchanged"
                };
                eprintln!(
                    "Lux hook bridge {changed} at {}",
                    report.hooks_path.display()
                );
            }
            Ok(())
        }
        HooksAction::Status(status_args) => {
            let report = codex_hook_status(&status_args)?;
            if status_args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!(
                    "Lux hook bridge status for {}:",
                    report.hooks_path.display()
                );
                for event in &report.events {
                    let status = if event.installed {
                        "installed"
                    } else {
                        "missing"
                    };
                    eprintln!("  {}: {status}", event.event);
                }
            }
            Ok(())
        }
        HooksAction::Run(run_args) => {
            let report = run_hook_bridge(&run_args)?;
            if run_args.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            Ok(())
        }
    }
}

pub fn install_codex_hook_bridge(args: &HooksInstallArgs) -> Result<HookInstallReport> {
    let project_path = resolve_project_path(args.project_path.as_ref())?;
    let hooks_path = resolve_hooks_path(args.hooks_path.as_ref())?;
    let lux_exe = std::env::current_exe().context("failed to resolve current lux executable")?;
    let mut hooks_json = read_hooks_json(&hooks_path)?;
    let mut installed = Vec::new();
    let mut changed = false;

    for event in DEFAULT_CODEX_EVENTS {
        let command = hook_command(&lux_exe, event, &project_path);
        let already_installed =
            command_installed_for_event(&hooks_json, event, &project_path).is_some();
        if already_installed && !args.force {
            installed.push(HookEventInstallReport {
                event: (*event).to_string(),
                command,
                already_installed,
            });
            continue;
        }
        if !args.dry_run {
            if args.force {
                remove_lux_hook_commands_for_event(&mut hooks_json, event, &project_path);
            }
            append_hook_command(&mut hooks_json, event, &command)?;
        }
        changed = true;
        installed.push(HookEventInstallReport {
            event: (*event).to_string(),
            command,
            already_installed,
        });
    }

    if changed && !args.dry_run {
        write_json_atomic(&hooks_path, &hooks_json)?;
    }

    Ok(HookInstallReport {
        hooks_path,
        project_path,
        dry_run: args.dry_run,
        changed,
        installed,
    })
}

pub fn codex_hook_status(args: &HooksStatusArgs) -> Result<HookStatusReport> {
    let project_path = resolve_project_path(args.project_path.as_ref())?;
    let hooks_path = resolve_hooks_path(args.hooks_path.as_ref())?;
    let hooks_json = read_hooks_json(&hooks_path)?;
    let events = DEFAULT_CODEX_EVENTS
        .iter()
        .map(|event| {
            let command = command_installed_for_event(&hooks_json, event, &project_path);
            HookEventStatusReport {
                event: (*event).to_string(),
                installed: command.is_some(),
                command,
            }
        })
        .collect();
    Ok(HookStatusReport {
        hooks_path,
        project_path,
        events,
    })
}

pub fn run_hook_bridge(args: &HooksRunArgs) -> Result<HookRunReport> {
    if args.event.trim().is_empty() {
        bail!("--event must not be empty");
    }
    let project_path = resolve_project_path(args.project_path.as_ref())?;
    let mut stdin_body = String::new();
    io::stdin()
        .read_to_string(&mut stdin_body)
        .context("failed to read hook stdin")?;
    let event_id = format!("lux-hook-{}", Uuid::new_v4());
    let timestamp_utc = Utc::now().to_rfc3339();
    let parsed_stdin = serde_json::from_str::<Value>(&stdin_body).ok();
    let prompt_excerpt = prompt_excerpt(parsed_stdin.as_ref(), &stdin_body);
    let ulw_detected = contains_ulw_signal(&stdin_body)
        || prompt_excerpt
            .as_ref()
            .is_some_and(|excerpt| contains_ulw_signal(excerpt));
    let omx_ultrawork = inspect_omx_ultrawork(&project_path);
    let hook_dir = project_path.join(".lux").join("hooks");
    fs::create_dir_all(&hook_dir)
        .with_context(|| format!("failed to create {}", hook_dir.display()))?;
    let event_log_path = hook_dir.join("events.jsonl");
    let record = json!({
        "schema_version": 1,
        "event_id": event_id,
        "timestamp_utc": timestamp_utc,
        "event": args.event,
        "source": "codex-native-hook",
        "ulw_detected": ulw_detected,
        "prompt_excerpt": prompt_excerpt,
        "stdin_json_valid": parsed_stdin.is_some(),
        "omx_ultrawork": omx_ultrawork,
    });
    append_jsonl(&event_log_path, &record)?;
    if ulw_detected {
        let latest_path = hook_dir.join("ulw-check.json");
        write_json_atomic(&latest_path, &record)?;
    }
    Ok(HookRunReport {
        event_id,
        event: args.event.clone(),
        project_path,
        event_log_path,
        ulw_detected,
        omx_ultrawork,
    })
}

fn read_hooks_json(path: &Path) -> Result<Value> {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(json!({ "hooks": {} })),
        Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
    }
}

fn append_hook_command(root: &mut Value, event: &str, command: &str) -> Result<()> {
    let object = root
        .as_object_mut()
        .context("hooks.json root must be a JSON object")?;
    let hooks = object
        .entry("hooks".to_string())
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("hooks.json hooks field must be a JSON object")?;
    let event_entries = hooks
        .entry(event.to_string())
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .with_context(|| format!("hooks.json hooks.{event} must be an array"))?;
    event_entries.push(json!({
        "hooks": [
            {
                "type": "command",
                "command": command,
            }
        ]
    }));
    Ok(())
}

fn command_installed_for_event(root: &Value, event: &str, project_path: &Path) -> Option<String> {
    let entries = root
        .as_object()
        .and_then(|object| object.get("hooks"))
        .and_then(Value::as_object)
        .and_then(|hooks| hooks.get(event))
        .and_then(Value::as_array)?;
    for entry in entries {
        let hook_commands = entry
            .as_object()
            .and_then(|object| object.get("hooks"))
            .and_then(Value::as_array);
        let Some(hook_commands) = hook_commands else {
            continue;
        };
        for hook in hook_commands {
            let command = hook
                .as_object()
                .and_then(|object| object.get("command"))
                .and_then(Value::as_str);
            if let Some(command) = command {
                if is_lux_project_hook_command(command, event, project_path) {
                    return Some(command.to_string());
                }
            }
        }
    }
    None
}

fn remove_lux_hook_commands_for_event(root: &mut Value, event: &str, project_path: &Path) {
    let Some(entries) = root
        .as_object_mut()
        .and_then(|object| object.get_mut("hooks"))
        .and_then(Value::as_object_mut)
        .and_then(|hooks| hooks.get_mut(event))
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for entry in entries.iter_mut() {
        let Some(hook_commands) = entry
            .as_object_mut()
            .and_then(|object| object.get_mut("hooks"))
            .and_then(Value::as_array_mut)
        else {
            continue;
        };
        hook_commands.retain(|hook| {
            let command = hook
                .as_object()
                .and_then(|object| object.get("command"))
                .and_then(Value::as_str);
            !command
                .is_some_and(|command| is_lux_project_hook_command(command, event, project_path))
        });
    }
    entries.retain(|entry| {
        entry
            .as_object()
            .and_then(|object| object.get("hooks"))
            .and_then(Value::as_array)
            .is_some_and(|hooks| !hooks.is_empty())
    });
}

fn is_lux_project_hook_command(command: &str, event: &str, project_path: &Path) -> bool {
    command.contains("hooks run")
        && command.contains(&format!("--event {}", shell_quote(event)))
        && command.contains(&format!(
            "--project-path {}",
            shell_quote(&project_path.display().to_string())
        ))
}

fn hook_command(lux_exe: &Path, event: &str, project_path: &Path) -> String {
    format!(
        "{} hooks run --event {} --project-path {}",
        shell_quote(&lux_exe.display().to_string()),
        shell_quote(event),
        shell_quote(&project_path.display().to_string())
    )
}

fn resolve_project_path(path: Option<&PathBuf>) -> Result<PathBuf> {
    match path {
        Some(path) => Ok(path.clone()),
        None => std::env::current_dir().context("failed to resolve current directory"),
    }
}

fn resolve_hooks_path(path: Option<&PathBuf>) -> Result<PathBuf> {
    if let Some(path) = path {
        return Ok(path.clone());
    }
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        if !codex_home.trim().is_empty() {
            return Ok(PathBuf::from(codex_home).join("hooks.json"));
        }
    }
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    }
    .context("failed to resolve HOME for default Codex hooks path")?;
    Ok(PathBuf::from(home).join(".codex").join("hooks.json"))
}

fn shell_quote(value: &str) -> String {
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "/._:-".contains(character))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn prompt_excerpt(parsed: Option<&Value>, raw: &str) -> Option<String> {
    let from_json = parsed.and_then(first_prompt_like_string);
    let text = from_json.unwrap_or(raw);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.chars().take(240).collect())
    }
}

fn first_prompt_like_string(value: &Value) -> Option<&str> {
    match value {
        Value::Object(object) => {
            for key in ["prompt", "user_prompt", "message", "text", "input"] {
                if let Some(text) = object.get(key).and_then(Value::as_str) {
                    return Some(text);
                }
            }
            for child in object.values() {
                if let Some(text) = first_prompt_like_string(child) {
                    return Some(text);
                }
            }
            None
        }
        Value::Array(values) => values.iter().find_map(first_prompt_like_string),
        Value::String(text) => Some(text),
        _ => None,
    }
}

fn contains_ulw_signal(text: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|token| {
            let normalized = token.to_ascii_lowercase();
            normalized == "ulw" || normalized == "ultrawork"
        })
}

fn inspect_omx_ultrawork(project_path: &Path) -> OmxUltraworkStatus {
    let sessions_dir = project_path.join(".omx").join("state").join("sessions");
    let Ok(entries) = fs::read_dir(&sessions_dir) else {
        return OmxUltraworkStatus::NotFound;
    };
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in entries.flatten() {
        let path = entry.path().join("ultrawork-state.json");
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        let modified = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        if newest
            .as_ref()
            .is_none_or(|(_, newest_modified)| modified > *newest_modified)
        {
            newest = Some((path, modified));
        }
    }
    let Some((state_path, _)) = newest else {
        return OmxUltraworkStatus::NotFound;
    };
    let text = match fs::read_to_string(&state_path) {
        Ok(text) => text,
        Err(error) => {
            return OmxUltraworkStatus::Invalid {
                state_path,
                error: error.to_string(),
            }
        }
    };
    let parsed = match serde_json::from_str::<Value>(&text) {
        Ok(parsed) => parsed,
        Err(error) => {
            return OmxUltraworkStatus::Invalid {
                state_path,
                error: error.to_string(),
            }
        }
    };
    let object = parsed.as_object().cloned().unwrap_or_else(Map::new);
    let active = object
        .get("active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reinforcement_count = object.get("reinforcement_count").and_then(Value::as_u64);
    if active {
        OmxUltraworkStatus::Active {
            state_path,
            reinforcement_count,
        }
    } else {
        OmxUltraworkStatus::Inactive {
            state_path,
            reinforcement_count,
        }
    }
}

fn append_jsonl(path: &Path, record: &Value) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(record)?)
        .with_context(|| format!("failed to append {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to sync {}", path.display()))?;
    Ok(())
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .with_context(|| format!("{} has no valid UTF-8 file name", path.display()))?;
    let tmp_path = parent.join(format!(".{file_name}.tmp"));
    let mut tmp_file = File::create(&tmp_path)
        .with_context(|| format!("failed to create temporary file {}", tmp_path.display()))?;
    tmp_file
        .write_all(serde_json::to_string_pretty(value)?.as_bytes())
        .with_context(|| format!("failed to write temporary file {}", tmp_path.display()))?;
    tmp_file
        .write_all(b"\n")
        .with_context(|| format!("failed to write newline to {}", tmp_path.display()))?;
    tmp_file
        .sync_all()
        .with_context(|| format!("failed to sync temporary file {}", tmp_path.display()))?;
    drop(tmp_file);
    fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "failed to rename temporary file {} to {}",
            tmp_path.display(),
            path.display()
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "{prefix}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ))
    }

    #[test]
    fn install_codex_hook_bridge_is_idempotent_for_project() {
        let project = temp_path("lux-hooks-unit-project");
        let hooks_path = temp_path("lux-hooks-unit-codex").join("hooks.json");
        fs::create_dir_all(&project).expect("create project");

        let first = install_codex_hook_bridge(&HooksInstallArgs {
            project_path: Some(project.clone()),
            hooks_path: Some(hooks_path.clone()),
            dry_run: false,
            force: false,
            json: true,
        })
        .expect("first install");
        assert!(first.changed);

        let second = install_codex_hook_bridge(&HooksInstallArgs {
            project_path: Some(project.clone()),
            hooks_path: Some(hooks_path.clone()),
            dry_run: false,
            force: false,
            json: true,
        })
        .expect("second install");
        assert!(!second.changed);

        let status = codex_hook_status(&HooksStatusArgs {
            project_path: Some(project),
            hooks_path: Some(hooks_path),
            json: true,
        })
        .expect("hook status");
        assert!(status.events.iter().all(|event| event.installed));
    }

    #[test]
    fn ulw_signal_detection_matches_whole_tokens() {
        assert!(contains_ulw_signal("please run ulw now"));
        assert!(contains_ulw_signal("$oh-my-codex:ultrawork"));
        assert!(!contains_ulw_signal("skulwark"));
    }

    #[test]
    fn user_prompt_submit_run_records_event_log() {
        let project = temp_path("lux-hooks-run-event-project");
        fs::create_dir_all(&project).expect("create project");

        let report = run_hook_bridge(&HooksRunArgs {
            event: "UserPromptSubmit".to_string(),
            project_path: Some(project.clone()),
            json: true,
        })
        .expect("run hook");

        assert_eq!(report.event, "UserPromptSubmit");
        let text = fs::read_to_string(project.join(".lux/hooks/events.jsonl"))
            .expect("read hook event log");
        let record: Value = serde_json::from_str(text.lines().next().expect("first JSONL line"))
            .expect("parse hook record");
        assert_eq!(record["event"], "UserPromptSubmit");
        assert_eq!(record["source"], "codex-native-hook");
    }

    #[test]
    fn status_reports_project_settings_and_agents_rule_paths() {
        let project = temp_path("lux-hooks-status-project");
        let hooks_path = temp_path("lux-hooks-status-codex").join("hooks.json");
        fs::create_dir_all(project.join("Assets/Scripts")).expect("create project dirs");
        fs::write(project.join("AGENTS.md"), "# Root rules\n").expect("write root rules");
        fs::write(project.join("Assets/AGENTS.md"), "# Asset rules\n")
            .expect("write nested rules");
        fs::write(
            project.join(".lux-agent.toml"),
            r#"
version = 1

[hooks]
pre_work_rule_load = true
post_edit_policy = true
verification_evidence = true

[policy]
forbidden_markers = ["FORBIDDEN_MARKER"]
allow_markers = ["lux-allow-failover"]
"#
            .replace("FORBIDDEN_MARKER", &["TO", "DO"].concat()),
        )
        .expect("write settings");

        let report = codex_hook_status(&HooksStatusArgs {
            project_path: Some(project.clone()),
            hooks_path: Some(hooks_path),
            json: true,
        })
        .expect("hook status");
        let value = serde_json::to_value(report).expect("status JSON");

        assert_eq!(value["project_settings"]["status"], "configured");
        assert_eq!(value["project_settings"]["version"], 1);
        assert_eq!(
            value["agents_rule_paths"],
            json!([project.join("AGENTS.md"), project.join("Assets/AGENTS.md")])
        );
        assert!(value["lux_events"]
            .as_array()
            .expect("lux events")
            .iter()
            .any(|event| event["event"] == "LuxPostEditPolicy"
                && event["enabled"] == true));
    }

    #[test]
    fn post_edit_policy_fails_on_forbidden_marker_and_allow_marker_without_evidence() {
        let project = temp_path("lux-hooks-policy-project");
        fs::create_dir_all(project.join("Assets")).expect("create project dirs");
        fs::write(
            project.join(".lux-agent.toml"),
            format!(
                r#"
version = 1

[hooks]
post_edit_policy = true

[policy]
forbidden_markers = ["{}"]
allow_markers = ["lux-allow-failover"]
"#,
                ["TO", "DO"].concat()
            ),
        )
        .expect("write settings");
        fs::write(
            project.join("Assets/Script.cs"),
            format!(
                "// {}: replace\n// lux-allow-failover\npublic class Script {{}}\n",
                ["TO", "DO"].concat()
            ),
        )
        .expect("write source");

        let error = run_hook_bridge(&HooksRunArgs {
            event: "LuxPostEditPolicy".to_string(),
            project_path: Some(project.clone()),
            json: true,
        })
        .expect_err("policy should fail");
        let message = error.to_string();

        assert!(message.contains("LuxPostEditPolicy failed"));
        let text = fs::read_to_string(project.join(".lux/hooks/events.jsonl"))
            .expect("read hook event log");
        let record: Value = serde_json::from_str(text.lines().next().expect("first JSONL line"))
            .expect("parse hook record");
        assert_eq!(record["gate_result"]["status"], "failed");
        assert_eq!(record["gate_result"]["violations"][0]["line"], 1);
        assert_eq!(record["gate_result"]["violations"][1]["marker"], "lux-allow-failover");
    }

    #[test]
    fn run_records_project_settings_and_gate_result_in_lux_event_log() {
        let project = temp_path("lux-hooks-run-project-settings");
        fs::create_dir_all(&project).expect("create project");
        fs::write(
            project.join(".lux-agent.toml"),
            r#"
version = 1

[hooks]
post_edit_policy = true
"#,
        )
        .expect("write settings");

        run_hook_bridge(&HooksRunArgs {
            event: "LuxPostEditPolicy".to_string(),
            project_path: Some(project.clone()),
            json: true,
        })
        .expect("run hook");

        let text = fs::read_to_string(project.join(".lux/hooks/events.jsonl"))
            .expect("read hook event log");
        let record: Value = serde_json::from_str(text.lines().next().expect("first JSONL line"))
            .expect("parse hook record");
        assert_eq!(record["project_settings"]["status"], "configured");
        assert_eq!(record["loaded_rule_paths"], json!([]));
        assert_eq!(record["gate_result"]["status"], "passed");
    }

    #[test]
    fn missing_settings_reported_as_not_configured() {
        let project = temp_path("lux-hooks-missing-settings");
        let hooks_path = temp_path("lux-hooks-missing-codex").join("hooks.json");
        fs::create_dir_all(&project).expect("create project");

        let report = codex_hook_status(&HooksStatusArgs {
            project_path: Some(project),
            hooks_path: Some(hooks_path),
            json: true,
        })
        .expect("hook status");
        let value = serde_json::to_value(report).expect("status JSON");

        assert_eq!(value["project_settings"]["status"], "not_configured");
        assert_eq!(value["project_settings"]["path"], Value::Null);
        assert!(value["lux_events"]
            .as_array()
            .expect("lux events")
            .iter()
            .all(|event| event["enabled"] == false));
    }
}
