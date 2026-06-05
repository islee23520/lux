use super::rules::load_project_governance;
use super::{
    resolve_hooks_path, resolve_project_path, shell_quote, write_json_atomic,
    HookEventInstallReport, HookEventStatusReport, HookInstallReport, HookStatusReport,
    HooksInstallArgs, HooksStatusArgs, DEFAULT_CODEX_EVENTS, LUX_PROJECT_EVENTS,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::io;
use std::path::Path;

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
    let governance = load_project_governance(&project_path)?;
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
        lux_events: LUX_PROJECT_EVENTS
            .iter()
            .map(|event| (*event).to_string())
            .collect(),
        project_settings: governance.settings,
        loaded_rule_paths: governance.loaded_rule_paths,
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
