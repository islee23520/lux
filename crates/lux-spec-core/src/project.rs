use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    default_schema_version, validate_score, validate_supported_version, DialecticState,
    GlossarySpec, PackagesSpec, ProjectMeta, RoadmapSpec, SchellEvaluation, SpecDomains,
    SpecStatus, TargetsSpec, TestingSpec, UnitySpec, SUPPORTED_SPEC_SCHEMA_MAJOR_VERSION,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecProject {
    pub version: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    pub project_id: String,
    pub project_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub source: String,
    pub status: SpecStatus,
    #[serde(default)]
    pub meta: ProjectMeta,
    pub domains: SpecDomains,
    #[serde(default)]
    pub dialectic: DialecticState,
    #[serde(default)]
    pub roadmap: RoadmapSpec,
    #[serde(default)]
    pub unity: Option<UnitySpec>,
    #[serde(default)]
    pub targets: Option<TargetsSpec>,
    #[serde(default)]
    pub packages: Option<PackagesSpec>,
    #[serde(default)]
    pub testing: Option<TestingSpec>,
    #[serde(default)]
    pub glossary: Option<GlossarySpec>,
    pub schell_evaluation: SchellEvaluation,
    pub overall_ambiguity: f64,
}

impl Default for SpecProject {
    fn default() -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            version: "1.0.0".to_string(),
            schema_version: "2.0".to_string(),
            project_id: String::new(),
            project_name: String::new(),
            created_at: now.clone(),
            updated_at: now,
            source: "lux-init".to_string(),
            status: SpecStatus::Draft,
            meta: ProjectMeta::default(),
            domains: SpecDomains::default(),
            dialectic: DialecticState::default(),
            roadmap: RoadmapSpec::default(),
            unity: None,
            targets: None,
            packages: None,
            testing: None,
            glossary: None,
            schell_evaluation: SchellEvaluation::default(),
            overall_ambiguity: 1.0,
        }
    }
}

impl SpecProject {
    pub fn validate(&self) -> Result<(), String> {
        if !self
            .schema_version
            .starts_with(SUPPORTED_SPEC_SCHEMA_MAJOR_VERSION)
        {
            validate_supported_version(&self.version)?;
            return Ok(());
        }
        validate_score("overall_ambiguity", self.overall_ambiguity)?;
        self.domains.validate()?;
        if let Some(unity) = &self.unity {
            unity.validate()?;
        }
        if let Some(targets) = &self.targets {
            targets.validate()?;
        }
        if let Some(packages) = &self.packages {
            packages.validate()?;
        }
        if let Some(testing) = &self.testing {
            testing.validate()?;
        }
        if let Some(glossary) = &self.glossary {
            glossary.validate()?;
        }
        self.schell_evaluation.validate()?;
        Ok(())
    }
}
