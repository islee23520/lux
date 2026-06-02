use serde::{Deserialize, Serialize};

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const AI_CONTEXT_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct OntologySummary {
    pub schema_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_terms: Vec<String>,
}

impl Default for OntologySummary {
    fn default() -> Self {
        Self {
            schema_version: AI_CONTEXT_SCHEMA_VERSION.to_string(),
            required_terms: vec![
                "scene".to_string(),
                "stage".to_string(),
                "actor".to_string(),
                "component".to_string(),
                "transform".to_string(),
                "camera".to_string(),
                "viewport".to_string(),
                "coordinate_frames".to_string(),
                "expected_visual_state".to_string(),
                "evidence_class".to_string(),
                "blocker_class".to_string(),
                "completion_gate".to_string(),
                "schema_version".to_string(),
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AstSummary {
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub node_types: Vec<String>,
}

impl Default for AstSummary {
    fn default() -> Self {
        Self {
            source: "unknown".to_string(),
            node_count: None,
            node_types: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CoordinateMappingSummary {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub origins: Vec<String>,
}

impl Default for CoordinateMappingSummary {
    fn default() -> Self {
        Self {
            frames: vec![
                "world".to_string(),
                "local".to_string(),
                "screen".to_string(),
                "viewport".to_string(),
                "ui".to_string(),
            ],
            origins: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct EvidenceGateRequirements {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_references: Vec<String>,
}

impl Default for EvidenceGateRequirements {
    fn default() -> Self {
        Self {
            required_evidence: vec![
                "scene_ast".to_string(),
                "coordinate_map".to_string(),
                "expected_visual_state".to_string(),
                "vision_match".to_string(),
            ],
            required_references: vec![
                "ast_node".to_string(),
                "coordinate_region".to_string(),
                "contract_doc".to_string(),
                "blocker_reason".to_string(),
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BlockerSummary {
    pub kind: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AiContextPayload {
    pub ontology: OntologySummary,
    pub ast_summary: AstSummary,
    pub coordinate_mapping_summary: CoordinateMappingSummary,
    pub evidence_gate_requirements: EvidenceGateRequirements,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<BlockerSummary>,
}

impl Default for AiContextPayload {
    fn default() -> Self {
        Self {
            ontology: OntologySummary::default(),
            ast_summary: AstSummary::default(),
            coordinate_mapping_summary: CoordinateMappingSummary::default(),
            evidence_gate_requirements: EvidenceGateRequirements::default(),
            blockers: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn crate_name_matches_package_when_bootstrapped() {
        assert_eq!(CRATE_NAME, "lux-ai-core");
    }

    #[test]
    fn empty_ai_context_payload_carries_required_ontology_surface() {
        let payload = AiContextPayload::default();
        let value = serde_json::to_value(&payload).expect("payload should serialize");

        assert_eq!(
            value["ontology"]["schema_version"],
            AI_CONTEXT_SCHEMA_VERSION
        );
        assert_eq!(
            value["ontology"]["required_terms"]
                .as_array()
                .unwrap()
                .len(),
            13
        );
        assert_eq!(value["ast_summary"]["source"], "unknown");
        assert_eq!(
            value["coordinate_mapping_summary"]["frames"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert_eq!(
            value["evidence_gate_requirements"]["required_evidence"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        assert!(value.get("blockers").is_none());
    }

    #[test]
    fn ai_context_payload_rejects_unknown_fields() {
        let raw = json!({
            "ontology": {
                "schema_version": "1.0.0",
                "required_terms": []
            },
            "ast_summary": {
                "source": "scene"
            },
            "coordinate_mapping_summary": {},
            "evidence_gate_requirements": {},
            "blockers": [],
            "unexpected": true
        });

        let err = serde_json::from_value::<AiContextPayload>(raw)
            .expect_err("unknown fields should be rejected");
        assert!(err.to_string().contains("unexpected"));
    }

    #[test]
    fn ai_context_payload_round_trips_with_blockers() {
        let payload = AiContextPayload {
            blockers: vec![BlockerSummary {
                kind: "dirty_worktree".to_string(),
                reason: "git status is not clean".to_string(),
            }],
            ..AiContextPayload::default()
        };

        let json = serde_json::to_string(&payload).expect("payload should serialize");
        let decoded: AiContextPayload =
            serde_json::from_str(&json).expect("payload should deserialize");

        assert_eq!(decoded.blockers.len(), 1);
        assert_eq!(decoded.blockers[0].kind, "dirty_worktree");
    }
}
