use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunStatus {
    Idle,
    Planning,
    Planned,
    DispatchReady,
    Executing,
    AwaitingApproval,
    AwaitingEvidence,
    ExecutingTicket,
    Verifying,
    AwaitingPlayStart,
    AwaitingFeedback,
    Paused,
    Blocked,
    RetryReady,
    Resumed,
    Completed,
    Failed,
    Interrupted,
    Recovering,
    Quarantined,
}

impl fmt::Display for RunStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Idle => "Idle",
            Self::Planning => "Planning",
            Self::Planned => "planned",
            Self::DispatchReady => "dispatch_ready",
            Self::Executing => "executing",
            Self::AwaitingApproval => "AwaitingApproval",
            Self::AwaitingEvidence => "AwaitingEvidence",
            Self::ExecutingTicket => "ExecutingTicket",
            Self::Verifying => "Verifying",
            Self::AwaitingPlayStart => "AwaitingPlayStart",
            Self::AwaitingFeedback => "AwaitingFeedback",
            Self::Paused => "Paused",
            Self::Blocked => "Blocked",
            Self::RetryReady => "retry_ready",
            Self::Resumed => "resumed",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Interrupted => "Interrupted",
            Self::Recovering => "Recovering",
            Self::Quarantined => "Quarantined",
        })
    }
}

impl std::str::FromStr for RunStatus {
    type Err = anyhow::Error;

    fn from_str(status: &str) -> Result<Self, Self::Err> {
        match status {
            "Idle" => Ok(Self::Idle),
            "Planning" => Ok(Self::Planning),
            "planned" | "Planned" => Ok(Self::Planned),
            "dispatch_ready" | "DispatchReady" => Ok(Self::DispatchReady),
            "executing" | "Executing" => Ok(Self::Executing),
            "AwaitingApproval" => Ok(Self::AwaitingApproval),
            "AwaitingEvidence" => Ok(Self::AwaitingEvidence),
            "ExecutingTicket" => Ok(Self::ExecutingTicket),
            "Verifying" => Ok(Self::Verifying),
            "AwaitingPlayStart" => Ok(Self::AwaitingPlayStart),
            "AwaitingFeedback" => Ok(Self::AwaitingFeedback),
            "Paused" => Ok(Self::Paused),
            "Blocked" => Ok(Self::Blocked),
            "retry_ready" | "RetryReady" => Ok(Self::RetryReady),
            "resumed" | "Resumed" => Ok(Self::Resumed),
            "Completed" => Ok(Self::Completed),
            "Failed" => Ok(Self::Failed),
            "Interrupted" => Ok(Self::Interrupted),
            "Recovering" => Ok(Self::Recovering),
            "Quarantined" => Ok(Self::Quarantined),
            other => Err(anyhow::anyhow!("unknown RunStatus: {}", other)),
        }
    }
}
