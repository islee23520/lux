use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextGoal {
    pub goal_id: String,
    pub title: String,
    pub rationale: String,
    pub source_spec_refs: Vec<String>,
    pub selected_engine: String,
    pub requested_goal: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentGoal {
    pub run_id: String,
    pub goal_id: String,
    pub title: String,
    pub rationale: String,
    pub source_spec_refs: Vec<String>,
    pub selected_engine: String,
    pub requested_goal: Option<String>,
    pub selected_at: String,
}
