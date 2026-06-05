use super::policy;
use super::rules::ProjectGovernance;
use super::{HookGateResult, DEFAULT_CODEX_EVENTS, LUX_PROJECT_EVENTS};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub(super) fn hook_source(event: &str) -> &'static str {
    if DEFAULT_CODEX_EVENTS.contains(&event) {
        "codex-native-hook"
    } else if LUX_PROJECT_EVENTS.contains(&event) {
        "lux-project-hook"
    } else {
        "unsupported-hook"
    }
}

pub(super) fn evaluate_gate(
    event: &str,
    project_path: &Path,
    governance: &ProjectGovernance,
    stdin_json: Option<&Value>,
) -> Result<HookGateResult> {
    if !known_event(event) {
        return Ok(HookGateResult {
            status: "unsupported".to_string(),
            findings: vec![policy::hook_finding(
                Path::new(".lux/hooks/events.jsonl"),
                "event",
                "unsupported hook event",
            )],
        });
    }
    match event {
        "LuxPostEditPolicy" => post_edit_policy_gate(project_path, governance),
        "LuxVerificationEvidence" => {
            verification_evidence_gate(project_path, governance, stdin_json)
        }
        _ => Ok(HookGateResult {
            status: "passed".to_string(),
            findings: Vec::new(),
        }),
    }
}

fn known_event(event: &str) -> bool {
    DEFAULT_CODEX_EVENTS.contains(&event) || LUX_PROJECT_EVENTS.contains(&event)
}

fn post_edit_policy_gate(
    project_path: &Path,
    governance: &ProjectGovernance,
) -> Result<HookGateResult> {
    if governance.settings.status != "configured" {
        return Ok(HookGateResult {
            status: "not_configured".to_string(),
            findings: Vec::new(),
        });
    }
    if !governance
        .settings
        .enabled_gates
        .iter()
        .any(|gate| gate == "post_edit_policy")
    {
        return Ok(HookGateResult {
            status: "disabled".to_string(),
            findings: Vec::new(),
        });
    }
    let findings = policy::scan_project(project_path, &governance.settings)?;
    Ok(HookGateResult {
        status: if findings.is_empty() {
            "passed".to_string()
        } else {
            "failed".to_string()
        },
        findings,
    })
}

fn verification_evidence_gate(
    project_path: &Path,
    governance: &ProjectGovernance,
    stdin_json: Option<&Value>,
) -> Result<HookGateResult> {
    if governance.settings.status != "configured" {
        return Ok(HookGateResult {
            status: "not_configured".to_string(),
            findings: Vec::new(),
        });
    }
    if !governance
        .settings
        .enabled_gates
        .iter()
        .any(|gate| gate == "verification_evidence")
    {
        return Ok(HookGateResult {
            status: "disabled".to_string(),
            findings: Vec::new(),
        });
    }
    let Some(path) = evidence_path(stdin_json) else {
        return Ok(failed_evidence_gate("missing evidence_path"));
    };
    let Some(relative) = path.strip_prefix(".lux/evidence/") else {
        return Ok(failed_evidence_gate(
            "evidence_path must point under .lux/evidence",
        ));
    };
    if relative.trim().is_empty() || path.contains("..") {
        return Ok(failed_evidence_gate("evidence_path must be normalized"));
    }
    if evidence_root_has_symlink(project_path) {
        return Ok(failed_evidence_gate("evidence root must not be a symlink"));
    }
    let evidence_root = project_path.join(".lux/evidence");
    let candidate = project_path.join(path);
    if std::fs::symlink_metadata(&candidate)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Ok(failed_evidence_gate("evidence_path must not be a symlink"));
    }
    let Ok(root_canonical) = evidence_root.canonicalize() else {
        return Ok(failed_evidence_gate("evidence root does not exist"));
    };
    let Ok(candidate_canonical) = candidate.canonicalize() else {
        return Ok(failed_evidence_gate("evidence_path does not exist"));
    };
    if !candidate_canonical.starts_with(&root_canonical) {
        return Ok(failed_evidence_gate(
            "evidence_path must stay under .lux/evidence",
        ));
    }
    if !candidate_canonical.is_file() {
        return Ok(failed_evidence_gate("evidence_path is not a file"));
    }
    Ok(HookGateResult {
        status: "passed".to_string(),
        findings: Vec::new(),
    })
}

fn evidence_root_has_symlink(project_path: &Path) -> bool {
    [
        project_path.join(".lux"),
        project_path.join(".lux/evidence"),
    ]
    .iter()
    .any(|path| {
        std::fs::symlink_metadata(path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false)
    })
}

fn evidence_path(stdin_json: Option<&Value>) -> Option<&str> {
    stdin_json?
        .as_object()?
        .get("evidence_path")
        .and_then(Value::as_str)
}

fn failed_evidence_gate(message: &str) -> HookGateResult {
    HookGateResult {
        status: "failed".to_string(),
        findings: vec![policy::hook_finding(
            Path::new(".lux/evidence"),
            "evidence_path",
            message,
        )],
    }
}
