use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const SCHEMA_VERSION: &str = "1.0.0";
pub const REQUIRED_TERMS: &[&str] = &[
    "scene",
    "stage",
    "actor",
    "component",
    "transform",
    "camera",
    "viewport",
    "coordinate_frames",
    "expected_visual_state",
    "evidence_class",
    "blocker_class",
    "completion_gate",
    "schema_version",
];

pub const CANONICAL_EVIDENCE_CLASSES: &[EvidenceClass] = &[
    EvidenceClass::SceneAst,
    EvidenceClass::CoordinateMap,
    EvidenceClass::ExpectedVisualState,
    EvidenceClass::Screenshot,
    EvidenceClass::VisionMatch,
    EvidenceClass::ManualQa,
];

pub const CANONICAL_BLOCKER_CLASSES: &[BlockerClass] = &[
    BlockerClass::MissingSceneAst,
    BlockerClass::MissingCoordinateMap,
    BlockerClass::MissingExpectedVisualState,
    BlockerClass::UnverifiedVisualMatch,
    BlockerClass::PixelOnlyCompletion,
    BlockerClass::UnsupportedEngine,
    BlockerClass::DirtyWorktree,
    BlockerClass::HungCommand,
];

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Default for Vec3 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateFrame {
    World,
    Local,
    Screen,
    Viewport,
    Ui,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraProjection {
    Perspective,
    Orthographic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Transform {
    pub translation: Vec3,
    pub rotation_degrees: Vec3,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::default(),
            rotation_degrees: Vec3::default(),
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Component {
    pub name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Camera {
    pub name: String,
    pub projection: CameraProjection,
    pub frame: CoordinateFrame,
    pub viewport: Viewport,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Actor {
    pub id: String,
    pub name: String,
    pub transform: Transform,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<Camera>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Stage {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actors: Vec<Actor>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Scene {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stages: Vec<Stage>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    SceneAst,
    CoordinateMap,
    ExpectedVisualState,
    Screenshot,
    VisionMatch,
    ManualQa,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerClass {
    MissingSceneAst,
    MissingCoordinateMap,
    MissingExpectedVisualState,
    UnverifiedVisualMatch,
    PixelOnlyCompletion,
    UnsupportedEngine,
    DirtyWorktree,
    HungCommand,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ExpectedVisualState {
    pub subject: String,
    pub frame: CoordinateFrame,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_classes: Vec<EvidenceClass>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CompletionGate {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_evidence: Vec<EvidenceClass>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocker_classes: Vec<BlockerClass>,
}

impl CompletionGate {
    pub fn rejects_pixel_only_completion(&self) -> bool {
        !self.required_evidence.is_empty()
            && self
                .required_evidence
                .iter()
                .all(|class| matches!(class, EvidenceClass::Screenshot))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum EvidenceClaim {
    VisualScene { subject: String },
    Location { subject: String },
    DocOnly { contract_ref: String },
    RuntimeCompletion { summary: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum EvidenceReference {
    AstNode {
        node_id: String,
    },
    CoordinateRegion {
        frame: CoordinateFrame,
        viewport: Viewport,
    },
    ContractDoc {
        path: String,
    },
    BlockerReason {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct EvidenceGateRequest {
    pub claim: EvidenceClaim,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_classes: Vec<EvidenceClass>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<EvidenceReference>,
    pub gate: CompletionGate,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum EvidenceGateDecision {
    Passed {
        details: Value,
    },
    Rejected {
        blocker: BlockerClass,
        reason: String,
        details: Value,
    },
}

impl EvidenceGateDecision {
    pub fn details(&self) -> &Value {
        match self {
            Self::Passed { details } | Self::Rejected { details, .. } => details,
        }
    }
}

pub fn route_evidence_gate(request: &EvidenceGateRequest) -> EvidenceGateDecision {
    match &request.claim {
        EvidenceClaim::DocOnly { .. } => {
            let docs_only = request
                .references
                .iter()
                .all(|reference| matches!(reference, EvidenceReference::ContractDoc { .. }));
            if docs_only {
                EvidenceGateDecision::Passed {
                    details: serde_json::json!({
                        "route": "doc_only",
                        "reference_kind": "contract_doc",
                    }),
                }
            } else {
                EvidenceGateDecision::Rejected {
                    blocker: BlockerClass::UnverifiedVisualMatch,
                    reason: "doc_only accepts contract docs only".to_string(),
                    details: serde_json::json!({
                        "route": "doc_only",
                        "reference_kind": "non_contract",
                    }),
                }
            }
        }
        EvidenceClaim::VisualScene { subject } => {
            let has_ast = request
                .evidence_classes
                .iter()
                .any(|class| matches!(class, EvidenceClass::SceneAst));
            let has_mapping = request
                .evidence_classes
                .iter()
                .any(|class| matches!(class, EvidenceClass::CoordinateMap));
            let has_ast_node_ref = request
                .references
                .iter()
                .any(|reference| matches!(reference, EvidenceReference::AstNode { .. }));
            let has_blocker_reason = request
                .references
                .iter()
                .any(|reference| matches!(reference, EvidenceReference::BlockerReason { .. }));

            if has_ast && has_mapping && has_ast_node_ref {
                EvidenceGateDecision::Passed {
                    details: serde_json::json!({
                        "route": "visual_scene",
                        "subject": subject,
                        "reference_kind": "ast_node",
                    }),
                }
            } else {
                EvidenceGateDecision::Rejected {
                    blocker: BlockerClass::MissingSceneAst,
                    reason: "visual scene claims require AST evidence and coordinate mapping"
                        .to_string(),
                    details: serde_json::json!({
                        "route": "visual_scene",
                        "subject": subject,
                        "reference_kind": if has_blocker_reason {
                            "blocker_reason"
                        } else {
                            "missing_ast_or_mapping"
                        },
                    }),
                }
            }
        }
        EvidenceClaim::Location { subject } => {
            let has_mapping = request
                .evidence_classes
                .iter()
                .any(|class| matches!(class, EvidenceClass::CoordinateMap));
            let has_region_ref = request
                .references
                .iter()
                .any(|reference| matches!(reference, EvidenceReference::CoordinateRegion { .. }));
            if has_mapping && has_region_ref {
                EvidenceGateDecision::Passed {
                    details: serde_json::json!({
                        "route": "location",
                        "subject": subject,
                        "reference_kind": "coordinate_region",
                    }),
                }
            } else {
                EvidenceGateDecision::Rejected {
                    blocker: BlockerClass::MissingCoordinateMap,
                    reason: "location claims require coordinate/camera/UI evidence".to_string(),
                    details: serde_json::json!({
                        "route": "location",
                        "subject": subject,
                        "reference_kind": "missing_coordinate_region",
                    }),
                }
            }
        }
        EvidenceClaim::RuntimeCompletion { summary } => EvidenceGateDecision::Rejected {
            blocker: BlockerClass::PixelOnlyCompletion,
            reason: "runtime completion claims cannot be satisfied by screenshot-only evidence"
                .to_string(),
            details: serde_json::json!({
                "route": "runtime_completion",
                "summary": summary,
                "reference_kind": "blocker_reason",
            }),
        },
    }
}

impl Default for CompletionGate {
    fn default() -> Self {
        Self {
            required_evidence: vec![
                EvidenceClass::SceneAst,
                EvidenceClass::CoordinateMap,
                EvidenceClass::ExpectedVisualState,
                EvidenceClass::VisionMatch,
            ],
            blocker_classes: CANONICAL_BLOCKER_CLASSES.to_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct GameVerificationOntology {
    pub schema_version: String,
    pub scene: Scene,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coordinate_frames: Vec<CoordinateFrame>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_visual_states: Vec<ExpectedVisualState>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_classes: Vec<EvidenceClass>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocker_classes: Vec<BlockerClass>,
    pub completion_gate: CompletionGate,
}

impl Default for GameVerificationOntology {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            scene: Scene {
                name: "scene".to_string(),
                stages: Vec::new(),
            },
            coordinate_frames: vec![
                CoordinateFrame::World,
                CoordinateFrame::Local,
                CoordinateFrame::Screen,
                CoordinateFrame::Viewport,
                CoordinateFrame::Ui,
            ],
            expected_visual_states: Vec::new(),
            evidence_classes: CANONICAL_EVIDENCE_CLASSES.to_vec(),
            blocker_classes: CANONICAL_BLOCKER_CLASSES.to_vec(),
            completion_gate: CompletionGate::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn crate_name_matches_package_when_bootstrapped() {
        assert_eq!(CRATE_NAME, "lux-verification-core");
    }

    #[test]
    fn ontology_required_terms_are_complete() {
        assert_eq!(
            REQUIRED_TERMS,
            &[
                "scene",
                "stage",
                "actor",
                "component",
                "transform",
                "camera",
                "viewport",
                "coordinate_frames",
                "expected_visual_state",
                "evidence_class",
                "blocker_class",
                "completion_gate",
                "schema_version",
            ]
        );
    }

    #[test]
    fn ontology_schema_version_is_pinned() {
        assert_eq!(SCHEMA_VERSION, "1.0.0");
        assert_eq!(
            GameVerificationOntology::default().schema_version,
            SCHEMA_VERSION
        );
    }

    #[test]
    fn rejects_pixel_only_completion() {
        let pixel_only_gate = CompletionGate {
            required_evidence: vec![EvidenceClass::Screenshot],
            blocker_classes: vec![BlockerClass::PixelOnlyCompletion],
        };

        assert!(pixel_only_gate.rejects_pixel_only_completion());
        assert!(!CompletionGate::default().rejects_pixel_only_completion());
    }

    #[test]
    fn blocker_schema_variants_are_canonical() {
        assert_eq!(
            CANONICAL_BLOCKER_CLASSES,
            &[
                BlockerClass::MissingSceneAst,
                BlockerClass::MissingCoordinateMap,
                BlockerClass::MissingExpectedVisualState,
                BlockerClass::UnverifiedVisualMatch,
                BlockerClass::PixelOnlyCompletion,
                BlockerClass::UnsupportedEngine,
                BlockerClass::DirtyWorktree,
                BlockerClass::HungCommand,
            ]
        );
    }

    #[test]
    fn visual_completion_requires_ast_and_mapping() {
        let request = EvidenceGateRequest {
            claim: EvidenceClaim::VisualScene {
                subject: "player spawn".to_string(),
            },
            evidence_classes: vec![EvidenceClass::Screenshot, EvidenceClass::VisionMatch],
            references: vec![EvidenceReference::BlockerReason {
                reason: "vision-only evidence".to_string(),
            }],
            gate: CompletionGate::default(),
        };

        let decision = route_evidence_gate(&request);

        assert!(matches!(
            decision,
            EvidenceGateDecision::Rejected {
                blocker: BlockerClass::MissingSceneAst,
                ..
            }
        ));
    }

    #[test]
    fn doc_only_accepts_contract_docs_only() {
        let request = EvidenceGateRequest {
            claim: EvidenceClaim::DocOnly {
                contract_ref: "docs/adr/ADR-005-Core-Package-Layer-Split.md".to_string(),
            },
            evidence_classes: vec![],
            references: vec![EvidenceReference::ContractDoc {
                path: "docs/adr/ADR-005-Core-Package-Layer-Split.md".to_string(),
            }],
            gate: CompletionGate {
                required_evidence: vec![EvidenceClass::ManualQa],
                blocker_classes: vec![BlockerClass::UnverifiedVisualMatch],
            },
        };

        let decision = route_evidence_gate(&request);

        assert!(matches!(decision, EvidenceGateDecision::Passed { .. }));

        let runtime_request = EvidenceGateRequest {
            claim: EvidenceClaim::RuntimeCompletion {
                summary: "pixel-only success".to_string(),
            },
            evidence_classes: vec![EvidenceClass::Screenshot],
            references: vec![EvidenceReference::BlockerReason {
                reason: "runtime completion claim".to_string(),
            }],
            gate: CompletionGate::default(),
        };

        let runtime_decision = route_evidence_gate(&runtime_request);

        assert!(matches!(
            runtime_decision,
            EvidenceGateDecision::Rejected {
                blocker: BlockerClass::PixelOnlyCompletion,
                ..
            }
        ));
    }

    #[test]
    fn screenshot_and_vision_evidence_must_reference_ast_or_region() {
        let request = EvidenceGateRequest {
            claim: EvidenceClaim::VisualScene {
                subject: "inventory panel".to_string(),
            },
            evidence_classes: vec![
                EvidenceClass::SceneAst,
                EvidenceClass::CoordinateMap,
                EvidenceClass::Screenshot,
                EvidenceClass::VisionMatch,
            ],
            references: vec![
                EvidenceReference::AstNode {
                    node_id: "ui.inventory-panel".to_string(),
                },
                EvidenceReference::CoordinateRegion {
                    frame: CoordinateFrame::Ui,
                    viewport: Viewport {
                        x: 10,
                        y: 20,
                        width: 300,
                        height: 200,
                    },
                },
            ],
            gate: CompletionGate::default(),
        };

        let decision = route_evidence_gate(&request);

        assert!(matches!(decision, EvidenceGateDecision::Passed { .. }));
        let details = decision.details();
        assert_eq!(details["route"], json!("visual_scene"));
    }
}
