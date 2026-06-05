use std::fs;
use std::path::PathBuf;

use lux::lux_ticket::{Ticket, TicketPriority, TicketStatus};
use lux::lux_verification::{route_verification, VerificationOpts};

fn temp_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}", uuid::Uuid::new_v4()))
}

fn verification_ticket(policy: &str) -> Ticket {
    Ticket {
        id: "ticket-verify".to_string(),
        title: "Verify".to_string(),
        description: "Run verification".to_string(),
        status: TicketStatus::InProgress,
        priority: TicketPriority::Medium,
        verification_policy: Some(policy.to_string()),
        ..Ticket::default()
    }
}

#[cfg(unix)]
#[test]
fn command_suite_evidence_rejects_symlinked_legacy_temp_file() {
    let project = temp_path("lux-verification-temp-symlink");
    let evidence_dir = project.join(".lux/evidence/autonomous/run-temp");
    fs::create_dir_all(&evidence_dir).expect("evidence dir");
    let outside = temp_path("lux-verification-outside-temp");
    fs::write(&outside, "outside-original").expect("outside file");
    std::os::unix::fs::symlink(&outside, evidence_dir.join("verify_1.txt.tmp"))
        .expect("legacy temp symlink");
    let opts = VerificationOpts {
        run_id: "run-temp".to_string(),
        working_dir: project.clone(),
        evidence_dir: PathBuf::from(".lux/evidence/autonomous/run-temp"),
    };

    let error = route_verification(&verification_ticket("command_suite:echo ok"), &opts)
        .expect_err("symlink temp must fail");

    assert!(error
        .to_string()
        .contains("failed to write verification evidence"));
    assert!(error.chain().any(|cause| cause
        .to_string()
        .contains("temporary file must not be a symlink")));
    assert_eq!(
        fs::read_to_string(outside).expect("outside content"),
        "outside-original"
    );
    let _ = fs::remove_dir_all(project);
}
