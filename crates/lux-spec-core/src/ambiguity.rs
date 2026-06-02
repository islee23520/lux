use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AmbiguityReport {
    /// Backward-compatible field name carrying the canonical ambiguity score.
    pub overall_score: f64,
    pub domain_scores: HashMap<String, DomainAmbiguity>,
    pub schell_phase_scores: HashMap<String, f64>,
    pub completion_ratio: f64,
    pub targeted_questions: Vec<TargetedQuestion>,
    pub recommendations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainAmbiguity {
    pub domain_name: String,
    pub completion_ratio: f64,
    pub ai_eval_score: f64,
    pub ast_parsability: f64,
    pub composite_score: f64,
    pub missing_fields: Vec<String>,
    pub questions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TargetedQuestion {
    pub domain: String,
    pub phase: String,
    pub question: String,
    pub priority: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}
