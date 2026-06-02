use std::{
    path::Path,
    process::{Command as ProcessCommand, Stdio},
};

use anyhow::{bail, Result};
use chrono::Utc;
use serde_json::{json, Value};

use crate::lux_manual_qa_io::write_manual_qa_evidence;
use crate::lux_manual_qa_labels::{
    capability_blocks, executable_in_path, phase_label, phase_requires_screenshot_path,
    screenshot_path_from_stdout,
};
pub use crate::lux_manual_qa_types::{
    ManualQaCapabilities, ManualQaCapabilityStatus, ManualQaCommand, ManualQaEngine,
    ManualQaEvidenceRequest, ManualQaEvidenceResult, ManualQaPhase, ManualQaStatus,
};

pub fn capture_manual_qa_evidence(
    request: &ManualQaEvidenceRequest,
) -> Result<ManualQaEvidenceResult> {
    let mut evidence_paths = Vec::new();
    evidence_paths.push(write_manual_qa_evidence(
        request,
        "capabilities",
        capability_payload(request),
    )?);

    if capability_blocks(request.capabilities.screenshot) {
        evidence_paths.push(write_manual_qa_evidence(
            request,
            "screenshot-blocker",
            blocker_payload(request, "screenshot unavailable"),
        )?);
        return Ok(result(ManualQaStatus::Blocked, request, evidence_paths));
    }

    if request.engine == ManualQaEngine::Godot {
        let configured = request.godot_cli.as_deref().unwrap_or("godot");
        if executable_in_path(configured).is_none() {
            evidence_paths.push(write_manual_qa_evidence(
                request,
                "godot-blocker",
                blocker_payload(request, &format!("missing Godot CLI: {configured}")),
            )?);
            return Ok(result(ManualQaStatus::Blocked, request, evidence_paths));
        }
    }

    let required = required_phases(request);
    for phase in required {
        if !request
            .commands
            .iter()
            .any(|command| command.phase == *phase)
        {
            evidence_paths.push(write_manual_qa_evidence(
                request,
                &format!("{}-blocker", phase_label(*phase)),
                blocker_payload(request, &format!("missing {} command", phase_label(*phase))),
            )?);
            return Ok(result(ManualQaStatus::Blocked, request, evidence_paths));
        }
    }

    for command in &request.commands {
        let command_result = run_manual_qa_command(request, command)?;
        evidence_paths.push(write_manual_qa_evidence(
            request,
            phase_label(command.phase),
            command_result.payload,
        )?);
        if command_result.status != ManualQaStatus::Passed {
            return Ok(result(command_result.status, request, evidence_paths));
        }
    }

    Ok(result(ManualQaStatus::Passed, request, evidence_paths))
}

fn required_phases(request: &ManualQaEvidenceRequest) -> &'static [ManualQaPhase] {
    match request.engine {
        ManualQaEngine::Unity => &[
            ManualQaPhase::Compile,
            ManualQaPhase::Test,
            ManualQaPhase::DynamicCode,
            ManualQaPhase::Screenshot,
        ],
        ManualQaEngine::Godot => &[],
        ManualQaEngine::ThreeJs => &[ManualQaPhase::DevServer, ManualQaPhase::BrowserScreenshot],
    }
}

struct CommandCapture {
    status: ManualQaStatus,
    payload: Value,
}

fn run_manual_qa_command(
    request: &ManualQaEvidenceRequest,
    command: &ManualQaCommand,
) -> Result<CommandCapture> {
    let argv = parse_command_argv(&command.command)?;
    let output = ProcessCommand::new(&argv[0])
        .args(&argv[1..])
        .current_dir(&request.project_path)
        .stdin(Stdio::null())
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let screenshot_path = screenshot_path_from_stdout(&stdout);
            if phase_requires_screenshot_path(command.phase) {
                match screenshot_path.as_deref() {
                    Some(path)
                        if request.project_path.join(path).is_file()
                            || Path::new(path).is_file() => {}
                    Some(path) => {
                        return Ok(CommandCapture {
                            status: ManualQaStatus::Blocked,
                            payload: blocker_payload(
                                request,
                                &format!("screenshot path not found: {path}"),
                            ),
                        });
                    }
                    None => {
                        return Ok(CommandCapture {
                            status: ManualQaStatus::Blocked,
                            payload: blocker_payload(request, "screenshot path was not emitted"),
                        });
                    }
                }
            }
            Ok(CommandCapture {
                status: if output.status.success() {
                    ManualQaStatus::Passed
                } else {
                    ManualQaStatus::Failed
                },
                payload: json!({
                    "schema_version": 1,
                    "kind": "manual_qa_command",
                    "engine": request.engine,
                    "phase": command.phase,
                    "video": request.capabilities.video,
                    "command": command.command,
                    "exit_code": output.status.code(),
                    "stdout": stdout,
                    "stderr": stderr,
                    "screenshot_path": screenshot_path,
                    "captured_at": Utc::now().to_rfc3339(),
                }),
            })
        }
        Err(error) => Ok(CommandCapture {
            status: ManualQaStatus::Failed,
            payload: json!({
                "schema_version": 1,
                "kind": "manual_qa_command",
                "engine": request.engine,
                "phase": command.phase,
                "video": request.capabilities.video,
                "command": command.command,
                "launch_error": error.to_string(),
                "captured_at": Utc::now().to_rfc3339(),
            }),
        }),
    }
}

fn parse_command_argv(command_text: &str) -> Result<Vec<String>> {
    let argv = command_text
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if argv.is_empty() {
        bail!("manual QA command cannot be empty");
    }
    Ok(argv)
}

fn capability_payload(request: &ManualQaEvidenceRequest) -> Value {
    json!({
        "schema_version": 1,
        "kind": "manual_qa_capabilities",
        "engine": request.engine,
        "run_id": request.run_id,
        "capabilities": request.capabilities,
        "captured_at": Utc::now().to_rfc3339(),
    })
}

fn blocker_payload(request: &ManualQaEvidenceRequest, reason: &str) -> Value {
    json!({
        "schema_version": 1,
        "kind": "manual_qa_blocker",
        "engine": request.engine,
        "run_id": request.run_id,
        "reason": reason,
        "video": request.capabilities.video,
        "capabilities": request.capabilities,
        "captured_at": Utc::now().to_rfc3339(),
    })
}

fn result(
    status: ManualQaStatus,
    request: &ManualQaEvidenceRequest,
    evidence_paths: Vec<String>,
) -> ManualQaEvidenceResult {
    ManualQaEvidenceResult {
        status,
        engine: request.engine,
        evidence_paths,
        capabilities: request.capabilities,
    }
}
