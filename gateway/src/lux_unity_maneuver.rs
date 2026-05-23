//! Unity maneuver orchestration for MCP/game-dev loops.
//!
//! This module intentionally reuses existing Lux surfaces instead of adding a
//! Unity Editor window or silently simulating success. It writes structured
//! evidence under `.lux/evidence/` for both success and explicit unavailable
//! failures so callers such as MCP tools can return `isError: true` while
//! preserving completed step evidence.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::lux_io::atomic_write_json;
use crate::uloop_runner;

const DEFAULT_BRIDGE_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityManeuverRequest {
    pub project_path: PathBuf,
    #[serde(default)]
    pub ticket_id: Option<String>,
    /// Safe uloop command used as an observable validation/maneuver step after
    /// bridge discovery succeeds. Defaults to `get-hierarchy`.
    #[serde(default = "default_uloop_args")]
    pub uloop_args: Vec<String>,
    /// Allows higher-level callers to record bridge-only evidence when they are
    /// deliberately sequencing validation elsewhere. This is not a dry-run: any
    /// unavailable bridge still fails explicitly.
    #[serde(default)]
    pub skip_uloop: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnityManeuverStatus {
    Success,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnityManeuverStepStatus {
    Success,
    Failed,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityManeuverStep {
    pub name: String,
    pub status: UnityManeuverStepStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub details: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnityManeuverResult {
    pub status: UnityManeuverStatus,
    pub is_error: bool,
    pub stop_reason: String,
    pub project_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ticket_id: Option<String>,
    pub evidence_refs: Vec<String>,
    pub steps: Vec<UnityManeuverStep>,
}

pub fn run_unity_maneuver(request: UnityManeuverRequest) -> UnityManeuverResult {
    let mut steps = Vec::new();
    let mut evidence_refs = Vec::new();

    match crate::try_ping_unity_bridge_backend(&request.project_path, DEFAULT_BRIDGE_TIMEOUT) {
        Ok(ping) => steps.push(UnityManeuverStep {
            name: "bridge_ping".to_string(),
            status: UnityManeuverStepStatus::Success,
            error: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            details: json!({
                "host": ping.host,
                "port": ping.port,
                "discoveryPath": ping.discovery_path,
                "ping": ping.ping,
            }),
        }),
        Err(error) => {
            steps.push(UnityManeuverStep {
                name: "bridge_ping".to_string(),
                status: UnityManeuverStepStatus::Unavailable,
                error: Some(format_error_chain(&error)),
                exit_code: None,
                stdout: None,
                stderr: None,
                details: json!({
                    "expectedDiscoveryPath": request.project_path.join("Library/UnityAiBridge/server.json"),
                    "guidance": "Install/start the Lux Unity bridge; missing discovery is an explicit blocker, not a silent fallback."
                }),
            });
            let result = finalize_result(
                request,
                UnityManeuverStatus::Failed,
                true,
                "unity_bridge_unavailable",
                steps,
                &mut evidence_refs,
            );
            return result;
        }
    }

    if !request.skip_uloop {
        let uloop_step = run_uloop_step(&request.project_path, &request.uloop_args);
        let failed = uloop_step.status != UnityManeuverStepStatus::Success;
        steps.push(uloop_step);
        if failed {
            return finalize_result(
                request,
                UnityManeuverStatus::Failed,
                true,
                "uloop_validation_unavailable_or_failed",
                steps,
                &mut evidence_refs,
            );
        }
    }

    finalize_result(
        request,
        UnityManeuverStatus::Success,
        false,
        "unity_maneuver_complete",
        steps,
        &mut evidence_refs,
    )
}

pub fn uloop_step_from_output(
    name: impl Into<String>,
    stdout: String,
    stderr: String,
    exit_code: i32,
) -> UnityManeuverStep {
    UnityManeuverStep {
        name: name.into(),
        status: if exit_code == 0 {
            UnityManeuverStepStatus::Success
        } else {
            UnityManeuverStepStatus::Failed
        },
        error: (exit_code != 0).then(|| format!("uloop exited with code {exit_code}")),
        exit_code: Some(exit_code),
        stdout: Some(stdout),
        stderr: Some(stderr),
        details: json!({ "preservedNonZeroExit": exit_code != 0 }),
    }
}

fn run_uloop_step(project_path: &Path, uloop_args: &[String]) -> UnityManeuverStep {
    let refs = uloop_args.iter().map(String::as_str).collect::<Vec<_>>();
    match uloop_runner::run_uloop_command(&refs, Some(project_path)) {
        Ok((stdout, stderr, exit_code)) => uloop_step_from_output(
            format!("uloop:{}", uloop_args.join(" ")),
            stdout,
            stderr,
            exit_code,
        ),
        Err(error) => UnityManeuverStep {
            name: format!("uloop:{}", uloop_args.join(" ")),
            status: UnityManeuverStepStatus::Unavailable,
            error: Some(format_error_chain(&error)),
            exit_code: None,
            stdout: None,
            stderr: None,
            details: json!({
                "guidance": "Install uloop with `lux unity install-uloop` or ensure the Unity CLI loop binary is on PATH."
            }),
        },
    }
}

fn finalize_result(
    request: UnityManeuverRequest,
    status: UnityManeuverStatus,
    is_error: bool,
    stop_reason: &str,
    steps: Vec<UnityManeuverStep>,
    evidence_refs: &mut Vec<String>,
) -> UnityManeuverResult {
    let mut result = UnityManeuverResult {
        status,
        is_error,
        stop_reason: stop_reason.to_string(),
        project_path: request.project_path,
        ticket_id: request.ticket_id,
        evidence_refs: Vec::new(),
        steps,
    };
    if let Ok(evidence_ref) = write_maneuver_evidence(&result) {
        evidence_refs.push(evidence_ref);
    }
    result.evidence_refs = evidence_refs.clone();
    result
}

fn write_maneuver_evidence(result: &UnityManeuverResult) -> anyhow::Result<String> {
    let evidence_id = format!("{}-{}", Utc::now().format("%Y%m%dT%H%M%SZ"), Uuid::new_v4());
    let relative = format!(".lux/evidence/unity-maneuver/{evidence_id}.json");
    let absolute = result.project_path.join(&relative);
    let evidence = json!({
        "schemaVersion": 1,
        "kind": "unity_maneuver_evidence",
        "capturedAtUtc": Utc::now().to_rfc3339(),
        "status": result.status,
        "isError": result.is_error,
        "stopReason": result.stop_reason,
        "ticketId": result.ticket_id,
        "steps": result.steps,
    });
    atomic_write_json(&absolute, &evidence).with_context(|| {
        format!(
            "failed to write Unity maneuver evidence {}",
            absolute.display()
        )
    })?;
    Ok(relative)
}

fn default_uloop_args() -> Vec<String> {
    vec!["get-hierarchy".to_string()]
}

fn format_error_chain(error: &anyhow::Error) -> String {
    error
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn missing_bridge_discovery_is_explicit_unavailable_failure_with_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let result = run_unity_maneuver(UnityManeuverRequest {
            project_path: temp.path().to_path_buf(),
            ticket_id: Some("TICKET-1".to_string()),
            uloop_args: default_uloop_args(),
            skip_uloop: false,
        });

        assert!(result.is_error);
        assert_eq!(result.status, UnityManeuverStatus::Failed);
        assert_eq!(result.stop_reason, "unity_bridge_unavailable");
        assert_eq!(result.steps[0].status, UnityManeuverStepStatus::Unavailable);
        assert!(result.steps[0]
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("Unity AI Bridge discovery file not found"));
        assert_eq!(result.evidence_refs.len(), 1);
        let evidence_path = temp.path().join(&result.evidence_refs[0]);
        let evidence = fs::read_to_string(evidence_path).unwrap();
        assert!(evidence.contains("unity_bridge_unavailable"));
        assert!(evidence.contains("TICKET-1"));
    }

    #[test]
    fn non_zero_uloop_exit_is_preserved_as_failed_step() {
        let step = uloop_step_from_output(
            "uloop:compile",
            "stdout text".to_string(),
            "stderr text".to_string(),
            42,
        );

        assert_eq!(step.status, UnityManeuverStepStatus::Failed);
        assert_eq!(step.exit_code, Some(42));
        assert_eq!(step.stdout.as_deref(), Some("stdout text"));
        assert_eq!(step.stderr.as_deref(), Some("stderr text"));
        assert_eq!(
            step.details
                .get("preservedNonZeroExit")
                .and_then(Value::as_bool),
            Some(true)
        );
    }
}
