mod ambiguity;
mod domain;
mod project;
mod schell;
mod unity;
mod validation;

use serde::{Deserialize, Serialize};

pub use ambiguity::{AmbiguityReport, DomainAmbiguity, TargetedQuestion};
pub use domain::{
    DialecticState, DomainKind, DomainSpec, DomainStatus, ProjectMeta, Requirement,
    RequirementPriority, RequirementStatus, RoadmapSpec, RoadmapTicket, SpecAssumption,
    SpecDecision, SpecDomains, SpecLink, SpecQuestion, TargetPlatformSpec,
};
pub use project::SpecProject;
pub use schell::{
    AssessmentResult, PhaseResult, PillarRating, PillarStatus, SchellEvaluation, TetradResult,
};
pub use unity::{GlossarySpec, PackageEntry, PackagesSpec, TargetsSpec, TestingSpec, UnitySpec};
pub use validation::validate_supported_version;
pub(crate) use validation::{clamp_score, default_glossary_path, validate_score};

pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");
pub const SUPPORTED_SPEC_MAJOR_VERSION: &str = "1";
pub const SUPPORTED_SPEC_SCHEMA_MAJOR_VERSION: &str = "2";

pub(crate) fn default_schema_version() -> String {
    "2.0".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecStatus {
    Draft,
    Active,
    Deprecated,
}

#[cfg(test)]
mod tests {
    use super::{DomainSpec, SpecProject, SpecStatus, TargetedQuestion, CRATE_NAME};

    #[test]
    fn crate_name_matches_package_when_bootstrapped() {
        assert_eq!(CRATE_NAME, "lux-spec-core");
    }

    #[test]
    fn spec_project_model_is_available_from_core() {
        let spec = SpecProject::default();

        assert_eq!(spec.status, SpecStatus::Draft);
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn domain_spec_clamps_ambiguity_in_core() {
        let domain = DomainSpec::new("design", "design.md", 1.5);

        assert_eq!(domain.ambiguity_score, 1.0);
    }

    #[test]
    fn targeted_question_json_shape_stays_gateway_compatible() {
        let question = TargetedQuestion {
            domain: "spec".to_string(),
            phase: "targets.platforms".to_string(),
            question: "Which targets?".to_string(),
            priority: 0.5,
            default_value: None,
            options: Vec::new(),
        };

        let value = serde_json::to_value(question).expect("question should serialize");
        assert!(value.get("default_value").is_none());
        assert!(value.get("options").is_none());
    }
}
