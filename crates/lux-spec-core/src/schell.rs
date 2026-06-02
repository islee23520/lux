use serde::{Deserialize, Serialize};

use crate::validate_score;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SchellEvaluation {
    pub phase1_experience: PhaseResult,
    pub phase2_tetrad: TetradResult,
    pub phase3_core_loop: PhaseResult,
    pub phase4_motivation: PhaseResult,
    pub phase5_assessment: AssessmentResult,
}

impl Default for SchellEvaluation {
    fn default() -> Self {
        Self {
            phase1_experience: PhaseResult::missing("Experience Lens"),
            phase2_tetrad: TetradResult::default(),
            phase3_core_loop: PhaseResult::missing("Core Loop Stress Test"),
            phase4_motivation: PhaseResult::missing("Player Motivation"),
            phase5_assessment: AssessmentResult::missing(),
        }
    }
}

impl SchellEvaluation {
    pub fn validate(&self) -> Result<(), String> {
        self.phase1_experience.validate()?;
        self.phase2_tetrad.validate()?;
        self.phase3_core_loop.validate()?;
        self.phase4_motivation.validate()?;
        self.phase5_assessment.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TetradResult {
    pub mechanics: PillarRating,
    pub story: PillarRating,
    pub aesthetics: PillarRating,
    pub technology: PillarRating,
    pub harmony_score: f64,
}

impl Default for TetradResult {
    fn default() -> Self {
        Self {
            mechanics: PillarRating::missing(),
            story: PillarRating::missing(),
            aesthetics: PillarRating::missing(),
            technology: PillarRating::missing(),
            harmony_score: 0.0,
        }
    }
}

impl TetradResult {
    pub fn validate(&self) -> Result<(), String> {
        self.mechanics.validate("mechanics")?;
        self.story.validate("story")?;
        self.aesthetics.validate("aesthetics")?;
        self.technology.validate("technology")?;
        validate_score("harmony_score", self.harmony_score)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PillarRating {
    pub status: PillarStatus,
    pub description: Option<String>,
    pub score: f64,
}

impl PillarRating {
    pub fn missing() -> Self {
        Self {
            status: PillarStatus::Missing,
            description: None,
            score: 0.0,
        }
    }

    pub fn validate(&self, name: &str) -> Result<(), String> {
        validate_score(&format!("{name} score"), self.score)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PillarStatus {
    Strong,
    NeedsWork,
    Missing,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseResult {
    pub name: String,
    pub status: PillarStatus,
    pub summary: Option<String>,
    pub score: f64,
    pub questions: Vec<String>,
}

impl PhaseResult {
    pub fn missing(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: PillarStatus::Missing,
            summary: None,
            score: 0.0,
            questions: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("phase name cannot be empty".to_string());
        }
        validate_score(&format!("phase '{}' score", self.name), self.score)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AssessmentResult {
    pub status: PillarStatus,
    pub viability_score: f64,
    pub strengths: Vec<String>,
    pub risks: Vec<String>,
    pub recommendations: Vec<String>,
    pub summary: Option<String>,
}

impl AssessmentResult {
    pub fn missing() -> Self {
        Self {
            status: PillarStatus::Missing,
            viability_score: 0.0,
            strengths: Vec::new(),
            risks: Vec::new(),
            recommendations: Vec::new(),
            summary: None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_score("viability_score", self.viability_score)
    }
}
