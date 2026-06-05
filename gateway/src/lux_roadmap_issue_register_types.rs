use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct IssueRegisterRequest {
    pub project_root: PathBuf,
    pub repo: String,
    pub dry_run: bool,
    pub existing_issues_json: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct IssueRegisterPlan {
    pub repo: String,
    pub dry_run: bool,
    pub planned_count: usize,
    pub existing_count: usize,
    pub created_count: usize,
    pub items: Vec<IssueRegisterItem>,
}

#[derive(Debug, Serialize)]
pub struct IssueRegisterItem {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub action: IssueRegisterAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub existing_issue: Option<ExistingIssue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_url: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueRegisterAction {
    WouldCreate,
    Exists,
    Created,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExistingIssue {
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub url: Option<String>,
}
