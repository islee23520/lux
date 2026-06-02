mod run_status;
mod state;
mod task_dag;
mod ticket;

pub use run_status::RunStatus;
pub use state::{
    ApprovalGateType, ApprovalState, ContinuationRunConfig, ExecutorInfo, ResumeCheckpoint,
    ResumeData, RunState, StopReason, RUN_STATE_SCHEMA_VERSION,
};
pub use task_dag::{TaskDAG, TaskNode, TaskNodeProjection, TaskStatus};
pub use ticket::{
    blocker_stable_tag, is_execution_grade, should_dispatch, stable_blocker_key,
    stable_blocker_ticket_id, stable_blocker_ticket_id_from_key, validate_execution_grade,
    BlockerPolicy, BlockerTicketUpsert, DispatchPolicy, Ticket, TicketFilter, TicketPriority,
    TicketStatus, TicketStore,
};

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::{
        is_execution_grade, should_dispatch, DispatchPolicy, RunState, RunStatus, TaskDAG, Ticket,
        TicketPriority, TicketStatus, CRATE_NAME, RUN_STATE_SCHEMA_VERSION,
    };

    #[test]
    fn crate_name_matches_package_when_bootstrapped() {
        assert_eq!(CRATE_NAME, "lux-run-core");
    }

    #[test]
    fn run_state_model_is_available_from_core() {
        let state = RunState::idle().expect("idle state should be created");

        assert_eq!(state.schema_version, RUN_STATE_SCHEMA_VERSION);
        assert_eq!(state.status, RunStatus::Idle.to_string());
        assert!(state.validate().is_ok());
    }

    #[test]
    fn ticket_dispatch_contract_is_available_from_core() {
        let ticket = Ticket {
            id: "ticket-1".to_string(),
            title: "Title".to_string(),
            description: "Description".to_string(),
            status: TicketStatus::Blocked,
            priority: TicketPriority::High,
            assignee: None,
            blockers: Vec::new(),
            tags: Vec::new(),
            spec_ref: None,
            created_at: "2026-05-14T00:00:00Z".to_string(),
            updated_at: "2026-05-14T00:00:00Z".to_string(),
            execution_objective: Some("Run verification".to_string()),
            allowed_executor: Some("codex".to_string()),
            dispatch_policy: Some(DispatchPolicy::DispatchRequested),
            verification_policy: Some("manual".to_string()),
            command_allowlist: None,
            evidence_refs: None,
            blocker_policy: None,
            non_goals: None,
        };

        assert!(is_execution_grade(&ticket));
        assert!(should_dispatch(&ticket));
    }

    #[test]
    fn task_dag_model_is_available_from_core() {
        let dag = TaskDAG::default();

        assert!(dag.nodes.is_empty());
        assert!(dag.ready_nodes().is_empty());
    }
}
