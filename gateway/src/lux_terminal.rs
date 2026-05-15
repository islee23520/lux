use std::collections::HashMap;

use anyhow::{bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_MAX_SESSIONS: usize = 5;
const ALLOWED_COMMANDS: &[&str] = &[
    "lux", "ls", "cat", "echo", "cargo", "rustc", "node", "npm", "npx", "tsc", "git", "help",
];
const DENIED_PATTERNS: &[&str] = &[
    "rm -rf", "sudo", "chmod", "chown", "exec", "eval", "wget", "&&", "||", ";", "`", "$(", ">",
    "<", "\n", "\r",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalStatus {
    Active,
    Closed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutput {
    pub session_id: String,
    pub data: String,
    pub timestamp: String,
    pub stream: OutputStream,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSession {
    pub session_id: String,
    pub created_at: String,
    pub status: TerminalStatus,
    pub output_buffer: Vec<TerminalOutput>,
    pub history: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct TerminalManager {
    sessions: HashMap<String, TerminalSession>,
    max_sessions: usize,
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            max_sessions: DEFAULT_MAX_SESSIONS,
        }
    }

    pub fn with_max_sessions(max_sessions: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            max_sessions,
        }
    }
}

pub fn create_terminal(manager: &mut TerminalManager) -> Result<TerminalSession> {
    if manager.sessions.len() >= manager.max_sessions {
        bail!("maximum terminal sessions reached");
    }

    let session = TerminalSession {
        session_id: Uuid::new_v4().to_string(),
        created_at: timestamp(),
        status: TerminalStatus::Active,
        output_buffer: Vec::new(),
        history: Vec::new(),
    };
    manager
        .sessions
        .insert(session.session_id.clone(), session.clone());
    Ok(session)
}

pub fn send_input(
    manager: &mut TerminalManager,
    session_id: &str,
    input: &str,
) -> Result<TerminalOutput> {
    let parsed = validate_command(input)?;
    let session = manager
        .sessions
        .get_mut(session_id)
        .ok_or_else(|| anyhow::anyhow!("terminal session not found"))?;

    if session.status != TerminalStatus::Active {
        bail!("terminal session is closed");
    }

    let trimmed = input.trim();
    session.history.push(trimmed.to_string());
    let output = TerminalOutput {
        session_id: session_id.to_string(),
        data: simulate_command_output(trimmed, &parsed),
        timestamp: timestamp(),
        stream: OutputStream::Stdout,
    };
    session.output_buffer.push(output.clone());
    Ok(output)
}

pub fn get_output(manager: &TerminalManager, session_id: &str) -> Result<Vec<TerminalOutput>> {
    manager
        .sessions
        .get(session_id)
        .map(|session| session.output_buffer.clone())
        .ok_or_else(|| anyhow::anyhow!("terminal session not found"))
}

pub fn destroy_terminal(manager: &mut TerminalManager, session_id: &str) -> Result<()> {
    let mut session = manager
        .sessions
        .remove(session_id)
        .ok_or_else(|| anyhow::anyhow!("terminal session not found"))?;
    session.status = TerminalStatus::Closed;
    Ok(())
}

pub fn list_terminals(manager: &TerminalManager) -> Vec<&TerminalSession> {
    manager.sessions.values().collect()
}

pub fn validate_command(input: &str) -> Result<Vec<String>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        bail!("command must not be empty");
    }

    let lowered = trimmed.to_ascii_lowercase();
    if DENIED_PATTERNS
        .iter()
        .any(|pattern| lowered.contains(pattern))
    {
        bail!("command contains a blocked pattern");
    }

    if lowered.contains('|') {
        bail!("pipes are not allowed in terminal commands");
    }

    if lowered.starts_with("curl") || lowered.contains(" curl ") {
        bail!("curl is not allowed for terminal commands");
    }

    let args = parse_args(trimmed)?;
    let command = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("command must not be empty"))?;
    if !ALLOWED_COMMANDS.contains(&command.as_str()) {
        bail!("command is not allowed: {command}");
    }

    Ok(args)
}

fn parse_args(input: &str) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;

    for ch in input.chars() {
        match (quote, ch) {
            (Some(active), next) if next == active => quote = None,
            (Some(_), next) => current.push(next),
            (None, '\'' | '"') => quote = Some(ch),
            (None, next) if next.is_whitespace() => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            (None, next) => current.push(next),
        }
    }

    if quote.is_some() {
        bail!("unterminated quoted argument");
    }
    if !current.is_empty() {
        args.push(current);
    }
    Ok(args)
}

fn simulate_command_output(input: &str, args: &[String]) -> String {
    match args.first().map(String::as_str) {
        Some("echo") => format!(
            "{}\r\n",
            args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ")
        ),
        Some("help") => format!("Allowed commands: {}\r\n", ALLOWED_COMMANDS.join(", ")),
        Some(command) => format!("$ {input}\r\nSimulated Lux terminal executed: {command}\r\n"),
        None => String::new(),
    }
}

fn timestamp() -> String {
    Utc::now().to_rfc3339()
}
