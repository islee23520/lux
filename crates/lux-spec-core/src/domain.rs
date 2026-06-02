use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{clamp_score, validate_score};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecDomains {
    pub gdd: Option<DomainSpec>,
    pub mechanics: Option<DomainSpec>,
    pub controls: Option<DomainSpec>,
    pub camera: Option<DomainSpec>,
    pub levels: Option<DomainSpec>,
    pub art_style: Option<DomainSpec>,
    pub audio: Option<DomainSpec>,
    pub narrative: Option<DomainSpec>,
    pub ui_ux: Option<DomainSpec>,
    pub technical_architecture: Option<DomainSpec>,
    pub engine: Option<DomainSpec>,
    pub testing: Option<DomainSpec>,
    pub build_release: Option<DomainSpec>,
    pub design: Option<DomainSpec>,
    pub architecture: Option<DomainSpec>,
    pub custom: HashMap<String, DomainSpec>,
}

impl Default for SpecDomains {
    fn default() -> Self {
        Self {
            gdd: None,
            mechanics: None,
            controls: None,
            camera: None,
            levels: None,
            art_style: None,
            audio: None,
            narrative: None,
            ui_ux: None,
            technical_architecture: None,
            engine: None,
            testing: None,
            build_release: None,
            design: None,
            architecture: None,
            custom: HashMap::new(),
        }
    }
}

impl SpecDomains {
    pub fn validate(&self) -> Result<(), String> {
        let built_in = [
            self.gdd.as_ref(),
            self.mechanics.as_ref(),
            self.controls.as_ref(),
            self.camera.as_ref(),
            self.levels.as_ref(),
            self.art_style.as_ref(),
            self.audio.as_ref(),
            self.narrative.as_ref(),
            self.ui_ux.as_ref(),
            self.technical_architecture.as_ref(),
            self.engine.as_ref(),
            self.testing.as_ref(),
            self.build_release.as_ref(),
            self.design.as_ref(),
            self.architecture.as_ref(),
        ];

        for domain in built_in.into_iter().flatten() {
            domain.validate()?;
        }

        for (name, domain) in &self.custom {
            if name.trim().is_empty() {
                return Err("custom domain name cannot be empty".to_string());
            }
            domain.validate()?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainSpec {
    pub name: String,
    pub content_path: String,
    pub fields: HashMap<String, Value>,
    pub ambiguity_score: f64,
    pub last_evaluated: Option<String>,
    pub defined: bool,
    #[serde(default)]
    pub kind: DomainKind,
    #[serde(default)]
    pub status: DomainStatus,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub non_goals: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<Requirement>,
    #[serde(default)]
    pub dependencies: Vec<SpecLink>,
    #[serde(default)]
    pub decisions: Vec<SpecDecision>,
    #[serde(default)]
    pub open_questions: Vec<SpecQuestion>,
    #[serde(default)]
    pub glossary_terms: Vec<String>,
    #[serde(default)]
    pub tests: Vec<String>,
}

impl DomainSpec {
    pub fn new(
        name: impl Into<String>,
        content_path: impl Into<String>,
        ambiguity_score: f64,
    ) -> Self {
        Self {
            name: name.into(),
            content_path: content_path.into(),
            fields: HashMap::new(),
            ambiguity_score: clamp_score(ambiguity_score),
            last_evaluated: None,
            defined: false,
            kind: DomainKind::default(),
            status: DomainStatus::default(),
            goals: Vec::new(),
            non_goals: Vec::new(),
            requirements: Vec::new(),
            dependencies: Vec::new(),
            decisions: Vec::new(),
            open_questions: Vec::new(),
            glossary_terms: Vec::new(),
            tests: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("domain name cannot be empty".to_string());
        }
        if self.content_path.trim().is_empty() {
            return Err(format!(
                "domain '{}' content_path cannot be empty",
                self.name
            ));
        }
        validate_score(
            &format!("domain '{}' ambiguity_score", self.name),
            self.ambiguity_score,
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub game_title: Option<String>,
    pub studio: Option<String>,
    pub genre: Option<String>,
    pub elevator_pitch: Option<String>,
    pub development_stage: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainKind {
    Experience,
    Mechanics,
    Technology,
    Content,
    Production,
    Quality,
    Custom,
}

impl Default for DomainKind {
    fn default() -> Self {
        Self::Custom
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainStatus {
    Undefined,
    Draft,
    Questioning,
    Defined,
    Validated,
}

impl Default for DomainStatus {
    fn default() -> Self {
        Self::Undefined
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequirementPriority {
    Must,
    Should,
    Could,
    Wont,
}

impl Default for RequirementPriority {
    fn default() -> Self {
        Self::Should
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequirementStatus {
    Proposed,
    Accepted,
    Rejected,
    Implemented,
    Verified,
}

impl Default for RequirementStatus {
    fn default() -> Self {
        Self::Proposed
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub priority: RequirementPriority,
    #[serde(default)]
    pub status: RequirementStatus,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    pub rationale: Option<String>,
    pub source_question: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub conflicts_with: Vec<String>,
    pub confidence: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecLink {
    pub kind: String,
    pub id: String,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DialecticState {
    #[serde(default)]
    pub questions: Vec<SpecQuestion>,
    #[serde(default)]
    pub decisions: Vec<SpecDecision>,
    #[serde(default, deserialize_with = "deserialize_assumptions")]
    pub assumptions: Vec<SpecAssumption>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpecQuestion {
    pub id: String,
    pub domain: Option<String>,
    pub text: String,
    pub answer: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
    pub answered_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpecDecision {
    pub id: String,
    pub domain: Option<String>,
    pub text: String,
    pub rationale: Option<String>,
    pub source_question: Option<String>,
    pub created_at: Option<String>,
}

fn deserialize_assumptions<'de, D>(deserializer: D) -> Result<Vec<SpecAssumption>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    struct AssumptionsVisitor;

    impl<'de> Visitor<'de> for AssumptionsVisitor {
        type Value = Vec<SpecAssumption>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("an array of assumption strings or objects")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Vec<SpecAssumption>, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut assumptions = Vec::new();
            while let Some(item) = seq.next_element::<serde_json::Value>()? {
                match item {
                    serde_json::Value::String(text) => {
                        assumptions.push(SpecAssumption {
                            id: format!("assumption-{}", assumptions.len()),
                            text,
                            confidence: None,
                            created_at: None,
                        });
                    }
                    serde_json::Value::Object(map) => {
                        let id = map
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&format!("assumption-{}", assumptions.len()))
                            .to_string();
                        let text = map
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let confidence = map.get("confidence").and_then(|v| v.as_f64());
                        let created_at = map
                            .get("created_at")
                            .and_then(|v| v.as_str())
                            .map(String::from);
                        assumptions.push(SpecAssumption {
                            id,
                            text,
                            confidence,
                            created_at,
                        });
                    }
                    _other => {
                        return Err(de::Error::invalid_type(
                            de::Unexpected::Other("non-string non-object"),
                            &self,
                        ))
                    }
                }
            }
            Ok(assumptions)
        }
    }

    deserializer.deserialize_seq(AssumptionsVisitor)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpecAssumption {
    pub id: String,
    pub text: String,
    pub confidence: Option<f64>,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RoadmapSpec {
    #[serde(default)]
    pub tickets: Vec<RoadmapTicket>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RoadmapTicket {
    pub id: String,
    pub title: String,
    pub domain: Option<String>,
    #[serde(default)]
    pub requirement_refs: Vec<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TargetPlatformSpec {
    pub name: String,
    pub status: Option<String>,
    pub priority: Option<String>,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub build_settings: HashMap<String, String>,
    #[serde(default)]
    pub performance_budget: HashMap<String, String>,
    #[serde(default)]
    pub control_scheme_refs: Vec<String>,
    #[serde(default)]
    pub test_refs: Vec<String>,
}
