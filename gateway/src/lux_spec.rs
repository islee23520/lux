use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{stdin, stdout, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::lux_ambiguity::{self, TargetedQuestion};
use crate::lux_roadmap;
use crate::project::{self, UnityProjectDetection};
pub use lux_spec_core::SpecStatus;

#[path = "lux_specs.rs"]
mod lux_specs;

pub const SUPPORTED_SPEC_MAJOR_VERSION: &str = "1";
pub const SUPPORTED_SPEC_SCHEMA_MAJOR_VERSION: &str = "2";

fn specs_root(project_path: &Path) -> PathBuf {
    project_path.join(".lux/specs")
}

fn canonical_domains_root(project_path: &Path) -> PathBuf {
    specs_root(project_path).join("domains")
}

fn canonical_spec_path(project_path: &Path) -> PathBuf {
    specs_root(project_path).join("spec.json")
}

fn legacy_spec_path(project_path: &Path) -> PathBuf {
    project_path.join(".lux/spec.json")
}

fn legacy_domains_root(project_path: &Path) -> PathBuf {
    project_path.join(".lux/domains")
}

fn canonical_domain_path(project_path: &Path, domain: &str) -> PathBuf {
    canonical_domains_root(project_path).join(format!("{}.md", canonical_domain_file_stem(domain)))
}

fn legacy_domain_path(project_path: &Path, domain: &str) -> PathBuf {
    legacy_domains_root(project_path).join(format!("{}.md", legacy_domain_file_stem(domain)))
}

fn atomic_write_text(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let tmp_path = path.with_extension("md.tmp");
    fs::write(&tmp_path, content)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path)
        .with_context(|| format!("failed to atomically replace {}", path.display()))
}

fn default_schema_version() -> String {
    "2.0".to_string()
}

pub trait SpecQuestionIo {
    fn present_detection(&mut self, detection: Option<&UnityProjectDetection>) -> Result<()>;

    fn ask(
        &mut self,
        question: &TargetedQuestion,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<Option<String>>;

    fn report_progress(&mut self, ambiguity_score: f64, target_ambiguity: f64) -> Result<()>;
}

pub struct TerminalSpecQuestionIo;

impl SpecQuestionIo for TerminalSpecQuestionIo {
    fn present_detection(&mut self, detection: Option<&UnityProjectDetection>) -> Result<()> {
        if let Some(d) = detection {
            println!("🔍 Detected Unity project:");
            if let Some(ref v) = d.editor_version {
                println!("   Version: {}", v);
            }
            if let Some(ref rp) = d.render_pipeline {
                println!("   Render Pipeline: {}", rp);
            }
            println!("   Packages: {} detected", d.packages.len());
            if d.test_framework_detected {
                println!("   Test Framework: Unity Test Framework");
            }
        } else {
            println!("ℹ️  Not a Unity project — skipping Unity auto-detection");
        }
        Ok(())
    }

    fn ask(
        &mut self,
        question: &TargetedQuestion,
        iteration: u32,
        max_iterations: u32,
    ) -> Result<Option<String>> {
        println!("\n[{} / {}]", iteration + 1, max_iterations);
        println!("❓ {}", question.question);
        if !question.options.is_empty() {
            println!("   Options: {}", question.options.join(", "));
        }
        if let Some(ref default) = question.default_value {
            print!("   → [{}]: ", default);
        } else {
            print!("   → ");
        }
        if stdout().flush().is_err() {
            return Ok(None);
        }

        let mut input = String::new();
        match stdin().read_line(&mut input) {
            Ok(0) => Ok(question.default_value.clone()),
            Ok(_) => {
                let trimmed = input.trim();
                if trimmed.is_empty() {
                    Ok(question.default_value.clone())
                } else {
                    Ok(Some(trimmed.to_string()))
                }
            }
            Err(_) => Ok(question.default_value.clone()),
        }
    }

    fn report_progress(&mut self, ambiguity_score: f64, target_ambiguity: f64) -> Result<()> {
        println!(
            "📊 Ambiguity: {:.3} / target {:.3}",
            ambiguity_score, target_ambiguity
        );
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct LuxInitInteractiveOptions {
    pub interactive: bool,
    pub target_ambiguity: f64,
    pub max_iterations: u32,
}

impl Default for LuxInitInteractiveOptions {
    fn default() -> Self {
        Self {
            interactive: true,
            target_ambiguity: 0.02,
            max_iterations: 10,
        }
    }
}

pub fn lux_init_interactive(
    project_path: &Path,
    io: &mut dyn SpecQuestionIo,
    options: LuxInitInteractiveOptions,
) -> Result<PathBuf> {
    let existing_spec = canonical_spec_path(project_path).is_file();
    let lux_path = lux_init(project_path)?;
    let mut spec = lux_load(project_path)?;

    if existing_spec {
        let report = lux_ambiguity::calculate_ambiguity(&spec);
        let defined_domains = count_defined_domains(&spec);
        println!("🔁 Re-evaluating existing .lux workspace");
        println!("   Defined domains: {defined_domains}");
        println!("   Current ambiguity: {:.3}", report.overall_score);
    } else {
        println!("✨ Initializing new .lux workspace");
    }

    let detection = project::detect_unity_project(project_path)?;
    io.present_detection(detection.as_ref())?;

    if let Some(ref detected_project) = detection {
        apply_detection_to_spec(&mut spec, detected_project);
    }

    spec.validate()
        .map_err(|error| anyhow::anyhow!("Validation error: {error}"))?;
    lux_save(project_path, &spec)?;

    if !options.interactive {
        io.report_progress(
            lux_ambiguity::calculate_ambiguity(&spec).overall_score,
            options.target_ambiguity,
        )?;
        return Ok(lux_path);
    }

    let max_iterations = options.max_iterations.max(1);
    for iteration in 0..max_iterations {
        let report = lux_ambiguity::calculate_ambiguity(&spec);

        if report.overall_score <= options.target_ambiguity {
            io.report_progress(report.overall_score, options.target_ambiguity)?;
            println!("✅ Spec is sufficiently detailed!");
            break;
        }

        io.report_progress(report.overall_score, options.target_ambiguity)?;

        let question = report
            .targeted_questions
            .iter()
            .filter(|question| can_answer_direct_question(&spec, question))
            .max_by(|left, right| {
                left.priority
                    .partial_cmp(&right.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(question) = question else {
            println!("ℹ️  No more spec-specific questions available.");
            break;
        };

        let answer = io.ask(question, iteration, max_iterations)?;
        match answer {
            Some(answer) => {
                answer_direct(&mut spec, question, &answer)?;
                spec.validate()
                    .map_err(|error| anyhow::anyhow!("Validation error: {error}"))?;
                lux_save(project_path, &spec)?;
            }
            None => break,
        }
    }

    lux_save(project_path, &spec)?;
    Ok(lux_path)
}

fn count_defined_domains(spec: &SpecProject) -> usize {
    let built_in_count = spec
        .domains
        .built_in_domains()
        .into_iter()
        .flatten()
        .filter(|domain| domain.defined)
        .count();

    built_in_count
        + spec
            .domains
            .custom
            .values()
            .filter(|domain| domain.defined)
            .count()
}

fn can_answer_direct_question(spec: &SpecProject, question: &TargetedQuestion) -> bool {
    if question.domain != "spec" {
        return false;
    }

    match question.phase.as_str() {
        "unity.required_version" => spec.unity.as_ref().is_none_or(|unity| {
            unity
                .required_version
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
        }),
        "targets.platforms" => spec
            .targets
            .as_ref()
            .is_none_or(|targets| targets.platforms.is_empty()),
        "packages.required" => spec
            .packages
            .as_ref()
            .is_none_or(|packages| packages.required.is_empty()),
        "testing.strategy" => spec.testing.as_ref().is_none_or(|testing| {
            testing
                .strategy
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
        }),
        _ => false,
    }
}

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

pub fn apply_detection_to_spec(spec: &mut SpecProject, detection: &UnityProjectDetection) {
    if spec.project_name.is_empty() || spec.project_name == "untitled" {
        spec.project_name = detection.project_name.clone();
    }

    if spec.unity.is_none() {
        spec.unity = Some(UnitySpec::default());
    }
    if let Some(ref mut unity) = spec.unity {
        unity.detected_version = detection.editor_version.clone();
        if unity.render_pipeline.is_none() {
            unity.render_pipeline = detection.render_pipeline.clone();
        }
        if unity.scripting_backend.is_none() {
            unity.scripting_backend = detection.scripting_backend.clone();
        }
    }

    if spec.targets.is_none() && !detection.target_platforms.is_empty() {
        spec.targets = Some(TargetsSpec {
            platforms: detection.target_platforms.clone(),
            min_sdk: HashMap::new(),
            test_platform: None,
            target_platforms: Vec::new(),
        });
    }

    if spec.packages.is_none() {
        spec.packages = Some(PackagesSpec::default());
    }
    if let Some(ref mut packages) = spec.packages {
        packages.detected = detection
            .packages
            .iter()
            .map(|dp: &crate::project::DetectedPackage| PackageEntry {
                name: dp.name.clone(),
                version: dp.version.clone(),
                reason: None,
                required_by_domain: Vec::new(),
            })
            .collect();
    }

    if detection.test_framework_detected {
        if spec.testing.is_none() {
            spec.testing = Some(TestingSpec {
                framework: Some("Unity Test Framework".to_string()),
                strategy: None,
                coverage: false,
            });
        } else if spec.testing.as_ref().unwrap().framework.is_none() {
            spec.testing.as_mut().unwrap().framework = Some("Unity Test Framework".to_string());
        }
    }

    if spec.glossary.is_none() {
        spec.glossary = Some(GlossarySpec::default());
    }
}

pub fn answer_direct(
    spec: &mut SpecProject,
    question: &TargetedQuestion,
    answer: &str,
) -> Result<()> {
    let answer = answer.trim();
    if answer.is_empty() {
        bail!("Answer cannot be empty");
    }

    if question.domain != "spec" {
        return Ok(());
    }

    match question.phase.as_str() {
        "unity.required_version" => {
            let unity = spec.unity.get_or_insert_with(UnitySpec::default);
            if unity
                .required_version
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
            {
                unity.required_version = Some(answer.to_string());
            }
        }
        "targets.platforms" => {
            let targets = spec.targets.get_or_insert_with(TargetsSpec::default);
            if targets.platforms.is_empty() {
                let mut seen = HashSet::new();
                targets.platforms = answer
                    .split(&[',', ';', '\n'][..])
                    .map(|value| value.trim().to_lowercase())
                    .filter(|value| !value.is_empty())
                    .filter(|value| seen.insert(value.clone()))
                    .collect();
            }
        }
        "packages.required" => {
            let packages = spec.packages.get_or_insert_with(PackagesSpec::default);
            let mut seen = packages
                .required
                .iter()
                .map(|package| package.name.clone())
                .collect::<HashSet<_>>();

            packages.required.extend(
                answer
                    .split(&[',', ';', '\n'][..])
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .filter(|name| seen.insert((*name).to_string()))
                    .map(|name| PackageEntry {
                        name: name.to_string(),
                        reason: None,
                        version: None,
                        required_by_domain: Vec::new(),
                    }),
            );
        }
        "testing.strategy" => {
            let testing = spec.testing.get_or_insert_with(TestingSpec::default);
            if testing
                .strategy
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
            {
                testing.strategy = Some(answer.to_string());
            }
        }
        _ => return Ok(()),
    }

    if let Err(error) = spec.validate() {
        bail!("Validation error after applying answer: {error}");
    }

    Ok(())
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecDomains {
    pub design: Option<DomainSpec>,
    pub architecture: Option<DomainSpec>,
    pub gdd: Option<DomainSpec>,
    pub mechanics: Option<DomainSpec>,
    pub controls: Option<DomainSpec>,
    pub camera: Option<DomainSpec>,
    pub art_style: Option<DomainSpec>,
    pub audio: Option<DomainSpec>,
    pub narrative: Option<DomainSpec>,
    pub levels: Option<DomainSpec>,
    pub technical_architecture: Option<DomainSpec>,
    pub engine: Option<DomainSpec>,
    pub testing: Option<DomainSpec>,
    pub build_release: Option<DomainSpec>,
    pub ui_ux: Option<DomainSpec>,
    pub custom: HashMap<String, DomainSpec>,
}

impl Default for SpecDomains {
    fn default() -> Self {
        Self {
            design: None,
            architecture: None,
            gdd: None,
            mechanics: None,
            controls: None,
            camera: None,
            art_style: None,
            audio: None,
            narrative: None,
            levels: None,
            technical_architecture: None,
            engine: None,
            testing: None,
            build_release: None,
            ui_ux: None,
            custom: HashMap::new(),
        }
    }
}

impl SpecDomains {
    pub fn built_in_domains(&self) -> [Option<&DomainSpec>; 15] {
        [
            self.design.as_ref(),
            self.architecture.as_ref(),
            self.gdd.as_ref(),
            self.mechanics.as_ref(),
            self.controls.as_ref(),
            self.camera.as_ref(),
            self.art_style.as_ref(),
            self.audio.as_ref(),
            self.narrative.as_ref(),
            self.levels.as_ref(),
            self.technical_architecture.as_ref(),
            self.engine.as_ref(),
            self.testing.as_ref(),
            self.build_release.as_ref(),
            self.ui_ux.as_ref(),
        ]
    }

    pub fn validate(&self) -> Result<(), String> {
        for domain in self.built_in_domains().into_iter().flatten() {
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnitySpec {
    pub required_version: Option<String>,
    pub detected_version: Option<String>,
    pub render_pipeline: Option<String>,
    pub scripting_backend: Option<String>,
    #[serde(default)]
    pub version_policy: Option<String>,
    #[serde(default)]
    pub color_space: Option<String>,
    #[serde(default)]
    pub input_system: Option<String>,
    #[serde(default)]
    pub api_compatibility_level: Option<String>,
    #[serde(default)]
    pub serialization_mode: Option<String>,
    #[serde(default)]
    pub project_settings_refs: Vec<String>,
}

impl Default for UnitySpec {
    fn default() -> Self {
        Self {
            required_version: None,
            detected_version: None,
            render_pipeline: None,
            scripting_backend: None,
            version_policy: None,
            color_space: None,
            input_system: None,
            api_compatibility_level: None,
            serialization_mode: None,
            project_settings_refs: Vec::new(),
        }
    }
}

impl UnitySpec {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(value) = &self.render_pipeline {
            match value.as_str() {
                "urp" | "hdrp" | "built-in" => {}
                _ => return Err("render_pipeline must be one of: urp, hdrp, built-in".to_string()),
            }
        }
        if let Some(value) = &self.scripting_backend {
            match value.as_str() {
                "il2cpp" | "mono" => {}
                _ => return Err("scripting_backend must be one of: il2cpp, mono".to_string()),
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TargetsSpec {
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub min_sdk: HashMap<String, String>,
    pub test_platform: Option<String>,
    #[serde(default)]
    pub target_platforms: Vec<TargetPlatformSpec>,
}

impl Default for TargetsSpec {
    fn default() -> Self {
        Self {
            platforms: Vec::new(),
            min_sdk: HashMap::new(),
            test_platform: None,
            target_platforms: Vec::new(),
        }
    }
}

impl TargetsSpec {
    pub fn validate(&self) -> Result<(), String> {
        for platform in &self.platforms {
            if platform.trim().is_empty() {
                return Err("targets.platforms cannot contain empty values".to_string());
            }
        }
        for (platform, sdk) in &self.min_sdk {
            if platform.trim().is_empty() {
                return Err("targets.min_sdk keys cannot be empty".to_string());
            }
            if sdk.trim().is_empty() {
                return Err(format!("targets.min_sdk['{platform}'] cannot be empty"));
            }
        }
        if let Some(platform) = &self.test_platform {
            if platform.trim().is_empty() {
                return Err("test_platform cannot be empty".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackageEntry {
    pub name: String,
    pub reason: Option<String>,
    pub version: Option<String>,
    #[serde(default)]
    pub required_by_domain: Vec<String>,
}

impl Default for PackageEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            reason: None,
            version: None,
            required_by_domain: Vec::new(),
        }
    }
}

impl PackageEntry {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("package name cannot be empty".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PackagesSpec {
    #[serde(default)]
    pub required: Vec<PackageEntry>,
    #[serde(default)]
    pub recommended: Vec<PackageEntry>,
    #[serde(default)]
    pub forbidden: Vec<PackageEntry>,
    #[serde(default)]
    pub detected: Vec<PackageEntry>,
}

impl Default for PackagesSpec {
    fn default() -> Self {
        Self {
            required: Vec::new(),
            recommended: Vec::new(),
            forbidden: Vec::new(),
            detected: Vec::new(),
        }
    }
}

impl PackagesSpec {
    pub fn validate(&self) -> Result<(), String> {
        for package in &self.required {
            package.validate()?;
        }
        for package in &self.recommended {
            package.validate()?;
        }
        for package in &self.forbidden {
            package.validate()?;
        }
        for package in &self.detected {
            package.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TestingSpec {
    pub framework: Option<String>,
    pub strategy: Option<String>,
    #[serde(default)]
    pub coverage: bool,
}

impl Default for TestingSpec {
    fn default() -> Self {
        Self {
            framework: None,
            strategy: None,
            coverage: false,
        }
    }
}

impl TestingSpec {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(framework) = &self.framework {
            if framework.trim().is_empty() {
                return Err("testing.framework cannot be empty".to_string());
            }
        }
        if let Some(strategy) = &self.strategy {
            if strategy.trim().is_empty() {
                return Err("testing.strategy cannot be empty".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GlossarySpec {
    #[serde(default = "default_glossary_path")]
    pub path: String,
    pub last_updated: Option<String>,
    #[serde(default)]
    pub term_count: u32,
}

impl Default for GlossarySpec {
    fn default() -> Self {
        Self {
            path: default_glossary_path(),
            last_updated: None,
            term_count: 0,
        }
    }
}

impl GlossarySpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.path.trim().is_empty() {
            return Err("glossary.path cannot be empty".to_string());
        }
        Ok(())
    }
}

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

fn validate_supported_version(version: &str) -> Result<(), String> {
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or_default();
    let minor = parts.next();
    let patch = parts.next();

    if parts.next().is_some()
        || major != SUPPORTED_SPEC_MAJOR_VERSION
        || minor.and_then(|part| part.parse::<u64>().ok()).is_none()
        || patch.and_then(|part| part.parse::<u64>().ok()).is_none()
    {
        return Err(format!("unsupported spec version: {version}"));
    }

    Ok(())
}

fn default_glossary_path() -> String {
    "glossary.md".to_string()
}

fn validate_score(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{name} must be between 0.0 and 1.0"));
    }
    Ok(())
}

fn clamp_score(value: f64) -> f64 {
    if value.is_nan() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

pub fn lux_init(project_path: &Path) -> Result<PathBuf> {
    let lux_path = project_path.join(".lux");
    let canonical_path = canonical_spec_path(project_path);
    let spec_path = legacy_spec_path(project_path);
    let domains_path = ensure_lux_directories(project_path)?;

    if !canonical_path.exists() {
        let now = Utc::now().to_rfc3339();
        let mut spec: SpecProject = serde_json::from_str(&get_default_spec_json()?)
            .context("failed to parse default spec template")?;
        spec.project_id = Uuid::new_v4().to_string();
        spec.project_name = project_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        spec.created_at = now.clone();
        spec.updated_at = now;

        crate::lux_io::atomic_write_json(&canonical_path, &spec)
            .with_context(|| format!("failed to write {}", canonical_path.display()))?;
    }

    if !spec_path.exists() {
        fs::copy(&canonical_path, &spec_path).with_context(|| {
            format!(
                "failed to mirror {} to {}",
                canonical_path.display(),
                spec_path.display()
            )
        })?;
    }

    lux_roadmap::init_or_load(project_path)?;

    for (domain, template) in domain_templates() {
        let path = domains_path.join(format!("{domain}.md"));
        if !path.exists() {
            fs::write(&path, template)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
    }

    let glossary_path = lux_path.join("glossary.md");
    if !glossary_path.exists() {
        fs::write(&glossary_path, include_str!("templates/glossary.md"))
            .with_context(|| format!("failed to write {}", glossary_path.display()))?;
    }

    lux_specs::ensure_specs_contract(project_path, &spec_path, &domains_path)?;

    Ok(lux_path)
}

fn ensure_lux_directories(project_path: &Path) -> Result<PathBuf> {
    let lux_path = project_path.join(".lux");
    let domains_path = lux_path.join("domains");
    fs::create_dir_all(&domains_path)
        .with_context(|| format!("failed to create {}", domains_path.display()))?;

    for directory in ["tickets", "logs", "backups", "sessions", "builds"] {
        let path = lux_path.join(directory);
        fs::create_dir_all(&path)
            .with_context(|| format!("failed to create {}", path.display()))?;
    }

    Ok(domains_path)
}

pub fn lux_reinit(project_path: &Path) -> Result<PathBuf> {
    let lux_path = project_path.join(".lux");
    if lux_path.exists() {
        let timestamp = Utc::now().format("%Y%m%d%H%M%S%f");
        let staging_path = project_path.join(format!(".lux-reinit-{timestamp}.tmp"));
        if staging_path.exists() {
            fs::remove_dir_all(&staging_path)
                .with_context(|| format!("failed to remove {}", staging_path.display()))?;
        }

        fs::rename(&lux_path, &staging_path).with_context(|| {
            format!(
                "failed to stage {} at {}",
                lux_path.display(),
                staging_path.display()
            )
        })?;

        let backup_path = lux_path.join("backups").join(format!("reinit-{timestamp}"));
        fs::create_dir_all(&backup_path)
            .with_context(|| format!("failed to create {}", backup_path.display()))?;

        for entry in fs::read_dir(&staging_path)
            .with_context(|| format!("failed to read {}", staging_path.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read {}", staging_path.display()))?;
            let destination = backup_path.join(entry.file_name());
            fs::rename(entry.path(), &destination).with_context(|| {
                format!("failed to move backup entry to {}", destination.display())
            })?;
        }

        fs::remove_dir_all(&staging_path)
            .with_context(|| format!("failed to remove {}", staging_path.display()))?;
    }

    lux_init(project_path)
}

pub fn lux_load_or_init(project_path: &Path) -> Result<SpecProject> {
    if !canonical_spec_path(project_path).is_file() && !legacy_spec_path(project_path).is_file() {
        lux_init(project_path)?;
    }
    lux_load(project_path)
}

pub fn lux_load(project_path: &Path) -> Result<SpecProject> {
    let spec_path = if canonical_spec_path(project_path).is_file() {
        canonical_spec_path(project_path)
    } else {
        legacy_spec_path(project_path)
    };
    let content = fs::read_to_string(&spec_path)
        .with_context(|| format!("failed to read {}", spec_path.display()))?;

    let mut value: Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse JSON {}", spec_path.display()))?;

    let repaired = normalize_spec_value(&mut value);

    if repaired {
        eprintln!("⚠️  [lux] Repaired malformed spec.json: writing normalized data back to disk");
        let repaired_json = serde_json::to_string_pretty(&value)
            .context("failed to serialize repaired spec.json")?;
        fs::write(&spec_path, repaired_json)
            .with_context(|| format!("failed to write repaired {}", spec_path.display()))?;
    }

    let spec: SpecProject = serde_json::from_value(value)
        .with_context(|| {
            format!(
                "failed to parse {} after automatic repair; run 'lux init --force' to reinitialize, or back up the corrupt file and reinitialize",
                spec_path.display()
            )
        })?;
    let mut spec = spec;
    sync_legacy_domain_aliases(&mut spec);

    spec.validate()
        .map_err(|error| anyhow::anyhow!("Validation error: {error}"))?;

    Ok(spec)
}

fn normalize_spec_value(value: &mut Value) -> bool {
    let Some(object) = value.as_object_mut() else {
        return false;
    };

    let mut repaired = false;

    if object.get("schema_version").is_none() {
        object.insert(
            "schema_version".to_string(),
            Value::String("2.0".to_string()),
        );
        repaired = true;
    }

    let project_name = object
        .get("project_name")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    if let Some(name) = project_name {
        let meta = object
            .entry("meta".to_string())
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if meta.get("game_title").is_none() {
            if let Some(meta_object) = meta.as_object_mut() {
                meta_object.insert("game_title".to_string(), Value::String(name));
                repaired = true;
            }
        }
    }

    if let Some(domains) = object.get_mut("domains") {
        repaired |= migrate_legacy_domain_alias(domains, "design", "gdd", "gdd", "gdd.md");
        repaired |= migrate_legacy_domain_alias(
            domains,
            "architecture",
            "technical_architecture",
            "technical-architecture",
            "technical-architecture.md",
        );
        repaired |= migrate_legacy_domain_alias(
            domains,
            "art_style",
            "art_style",
            "art_style",
            "art-style.md",
        );
        repaired |= migrate_legacy_domain_alias(domains, "ui_ux", "ui_ux", "ui_ux", "ui-ux.md");

        let kind_map: HashMap<&str, &str> = [
            ("gdd", "Production"),
            ("mechanics", "Mechanics"),
            ("controls", "Experience"),
            ("camera", "Experience"),
            ("levels", "Content"),
            ("art_style", "Content"),
            ("audio", "Content"),
            ("narrative", "Content"),
            ("ui_ux", "Experience"),
            ("technical_architecture", "Technology"),
            ("engine", "Technology"),
            ("testing", "Quality"),
            ("build_release", "Production"),
            ("design", "Experience"),
            ("architecture", "Technology"),
        ]
        .iter()
        .cloned()
        .collect();

        for (key, kind) in &kind_map {
            if let Some(domain) = domains.get_mut(*key) {
                let defined = domain
                    .get("defined")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                if let Some(domain_object) = domain.as_object_mut() {
                    if !domain_object.contains_key("kind") {
                        domain_object.insert("kind".to_string(), Value::String(kind.to_string()));
                        repaired = true;
                    }
                    if !domain_object.contains_key("status") {
                        let status = if defined { "Defined" } else { "Undefined" };
                        domain_object
                            .insert("status".to_string(), Value::String(status.to_string()));
                        repaired = true;
                    }

                    repaired |= repair_struct_array_field(
                        domain_object,
                        "requirements",
                        &format!("domains.{key}.requirements"),
                        normalize_requirement_entry,
                    );
                }
            }
        }

        if let Some(custom) = domains.get_mut("custom").and_then(Value::as_object_mut) {
            for (name, domain) in custom {
                if let Some(domain_object) = domain.as_object_mut() {
                    repaired |= repair_struct_array_field(
                        domain_object,
                        "requirements",
                        &format!("domains.custom.{name}.requirements"),
                        normalize_requirement_entry,
                    );
                }
            }
        }
    }

    if let Some(dialectic) = object.get_mut("dialectic").and_then(Value::as_object_mut) {
        repaired |= repair_struct_array_field(
            dialectic,
            "assumptions",
            "dialectic.assumptions",
            normalize_assumption_entry,
        );
    }

    repaired
}

fn migrate_legacy_domain_alias(
    domains: &mut Value,
    legacy_key: &str,
    canonical_key: &str,
    canonical_name: &str,
    canonical_content_path: &str,
) -> bool {
    let Some(domains_object) = domains.as_object_mut() else {
        return false;
    };

    let Some(legacy_domain) = domains_object.get(legacy_key).cloned() else {
        return false;
    };

    let canonical_missing = domains_object.get(canonical_key).is_none_or(Value::is_null);

    if canonical_missing {
        let mut migrated_domain = legacy_domain.clone();
        if let Some(domain_object) = migrated_domain.as_object_mut() {
            domain_object.insert(
                "name".to_string(),
                Value::String(canonical_name.to_string()),
            );
            domain_object.insert(
                "content_path".to_string(),
                Value::String(canonical_content_path.to_string()),
            );
        }
        domains_object.insert(canonical_key.to_string(), migrated_domain);
    } else if let Some(canonical_domain) = domains_object.get_mut(canonical_key) {
        if let (Some(legacy_object), Some(canonical_object)) =
            (legacy_domain.as_object(), canonical_domain.as_object_mut())
        {
            for (key, value) in legacy_object {
                canonical_object
                    .entry(key.clone())
                    .or_insert_with(|| value.clone());
            }
            canonical_object
                .entry("name".to_string())
                .or_insert_with(|| Value::String(canonical_name.to_string()));
            canonical_object
                .entry("content_path".to_string())
                .or_insert_with(|| Value::String(canonical_content_path.to_string()));
        }
    }
    if legacy_key != canonical_key {
        domains_object.remove(legacy_key);
    }

    true
}

fn repair_struct_array_field<F>(
    object: &mut serde_json::Map<String, Value>,
    field_key: &str,
    field_path: &str,
    mut normalize_entry: F,
) -> bool
where
    F: FnMut(&mut Value) -> Vec<String>,
{
    let Some(field_value) = object.get_mut(field_key) else {
        return false;
    };

    let mut reasons = Vec::new();
    let mut entries = match std::mem::take(field_value) {
        Value::Array(entries) => entries,
        other => {
            reasons.push(format!(
                "coerced {} value into array",
                json_value_kind(&other)
            ));
            vec![other]
        }
    };

    let mut repaired = !reasons.is_empty();
    let mut normalized_entries = Vec::with_capacity(entries.len());

    for mut entry in entries.drain(..) {
        let entry_reasons = normalize_entry(&mut entry);
        if !entry_reasons.is_empty() {
            repaired = true;
            reasons.extend(entry_reasons);
        }
        normalized_entries.push(entry);
    }

    *field_value = Value::Array(normalized_entries);

    if repaired {
        eprintln!(
            "⚠️  [lux] Repaired malformed {field_path} in spec.json: {}",
            reasons.join("; ")
        );
    }

    repaired
}

fn normalize_assumption_entry(entry: &mut Value) -> Vec<String> {
    match entry {
        Value::String(text) => {
            let text = std::mem::take(text);
            *entry = assumption_entry_value(text);
            vec!["converted string entry to object".to_string()]
        }
        Value::Object(object) => normalize_assumption_object(object),
        ref other => {
            let kind = json_value_kind(other);
            let text = json_value_to_text(other);
            *entry = assumption_entry_value(text);
            vec![format!("converted {kind} entry to object")]
        }
    }
}

fn normalize_requirement_entry(entry: &mut Value) -> Vec<String> {
    match entry {
        Value::String(text) => {
            let text = std::mem::take(text);
            *entry = requirement_entry_value(text);
            vec!["converted string entry to object".to_string()]
        }
        Value::Object(object) => normalize_requirement_object(object),
        ref other => {
            let kind = json_value_kind(other);
            let text = json_value_to_text(other);
            *entry = requirement_entry_value(text);
            vec![format!("converted {kind} entry to object")]
        }
    }
}

fn normalize_assumption_object(object: &mut serde_json::Map<String, Value>) -> Vec<String> {
    let mut reasons = Vec::new();

    if !matches!(object.get("id"), Some(Value::String(id)) if !id.trim().is_empty()) {
        object.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
        reasons.push("filled missing id".to_string());
    }

    if !matches!(object.get("text"), Some(Value::String(_))) {
        object.insert("text".to_string(), Value::String(String::new()));
        reasons.push("filled missing text".to_string());
    }

    if !matches!(
        object.get("confidence"),
        Some(Value::Number(_)) | Some(Value::Null)
    ) {
        object.insert("confidence".to_string(), Value::Null);
        reasons.push("normalized confidence to null".to_string());
    }

    if !matches!(
        object.get("created_at"),
        Some(Value::String(_)) | Some(Value::Null)
    ) {
        object.insert("created_at".to_string(), Value::Null);
        reasons.push("normalized created_at to null".to_string());
    }

    reasons
}

fn normalize_requirement_object(object: &mut serde_json::Map<String, Value>) -> Vec<String> {
    let mut reasons = Vec::new();

    if !matches!(object.get("id"), Some(Value::String(id)) if !id.trim().is_empty()) {
        object.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
        reasons.push("filled missing id".to_string());
    }

    if !matches!(object.get("text"), Some(Value::String(_))) {
        object.insert("text".to_string(), Value::String(String::new()));
        reasons.push("filled missing text".to_string());
    }

    if let Some(Value::String(priority)) = object.get_mut("priority") {
        let normalized = match priority.as_str() {
            "Critical" | "High" => Some("Must"),
            "Medium" => Some("Should"),
            "Low" => Some("Could"),
            "Must" | "Should" | "Could" | "Wont" => None,
            _ => None,
        };
        if let Some(normalized) = normalized {
            *priority = normalized.to_string();
            reasons.push("normalized legacy priority".to_string());
        }
    }

    reasons
}

fn assumption_entry_value(text: String) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
    object.insert("text".to_string(), Value::String(text));
    object.insert("confidence".to_string(), Value::Null);
    object.insert("created_at".to_string(), Value::Null);
    Value::Object(object)
}

fn requirement_entry_value(text: String) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("id".to_string(), Value::String(Uuid::new_v4().to_string()));
    object.insert("text".to_string(), Value::String(text));
    Value::Object(object)
}

fn json_value_to_text(value: &Value) -> String {
    value.as_str().map(str::to_string).unwrap_or_else(|| {
        if value.is_null() {
            String::new()
        } else {
            value.to_string()
        }
    })
}

fn json_value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub fn lux_save(project_path: &Path, spec: &SpecProject) -> Result<()> {
    let lux_path = project_path.join(".lux");
    let spec_path = canonical_spec_path(project_path);
    let legacy_path = legacy_spec_path(project_path);
    let backups_path = lux_path.join("backups");
    fs::create_dir_all(&backups_path)
        .with_context(|| format!("failed to create {}", backups_path.display()))?;

    if spec_path.exists() {
        let timestamp = Utc::now().format("%Y%m%d%H%M%S%f");
        let backup_path = backups_path.join(format!("spec-{timestamp}.json"));
        fs::copy(&spec_path, &backup_path).with_context(|| {
            format!(
                "failed to back up {} to {}",
                spec_path.display(),
                backup_path.display()
            )
        })?;
    }

    let mut updated = spec.clone();
    sync_legacy_domain_aliases(&mut updated);
    updated.updated_at = Utc::now().to_rfc3339();
    crate::lux_io::atomic_write_json(&spec_path, &updated)
        .with_context(|| format!("failed to write {}", spec_path.display()))?;
    crate::lux_io::atomic_write_json(&legacy_path, &updated).with_context(|| {
        format!(
            "failed to mirror {} to {}",
            spec_path.display(),
            legacy_path.display()
        )
    })
}

pub fn lux_load_domain(project_path: &Path, domain: &str) -> Result<String> {
    let canonical_path = canonical_domain_path(project_path, domain);
    let path = if canonical_path.is_file() {
        canonical_path
    } else {
        legacy_domain_path(project_path, domain)
    };
    fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))
}

pub fn lux_save_domain(project_path: &Path, domain: &str, content: &str) -> Result<()> {
    let canonical_path = canonical_domain_path(project_path, domain);
    let legacy_path = legacy_domain_path(project_path, domain);
    atomic_write_text(&canonical_path, content)
        .with_context(|| format!("failed to write {}", canonical_path.display()))?;
    atomic_write_text(&legacy_path, content)
        .with_context(|| format!("failed to mirror {}", legacy_path.display()))
}

fn custom_domain_has_provenance(spec: &SpecProject, domain: &str) -> bool {
    spec.dialectic.decisions.iter().any(|decision| {
        decision.domain.as_deref() == Some(domain)
            && decision
                .rationale
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
            && decision
                .source_question
                .as_ref()
                .is_some_and(|value| !value.trim().is_empty())
    })
}

pub fn lux_update_domain_field(
    project_path: &Path,
    domain: &str,
    key: &str,
    value: Value,
) -> Result<SpecProject> {
    let mut spec = lux_load(project_path)?;
    let normalized = canonical_domain_name(domain);
    let content_path = format!("{}.md", normalized.replace('_', "-"));
    let is_builtin_domain = matches!(
        normalized.as_str(),
        "gdd"
            | "mechanics"
            | "controls"
            | "camera"
            | "technical_architecture"
            | "art_style"
            | "audio"
            | "narrative"
            | "levels"
            | "engine"
            | "testing"
            | "build_release"
            | "ui_ux"
    );
    let custom_has_provenance = is_builtin_domain || custom_domain_has_provenance(&spec, domain);

    let domain_spec = match normalized.as_str() {
        "gdd" => spec
            .domains
            .gdd
            .get_or_insert_with(|| DomainSpec::new("gdd", "gdd.md", 1.0)),
        "mechanics" => spec
            .domains
            .mechanics
            .get_or_insert_with(|| DomainSpec::new("mechanics", "mechanics.md", 1.0)),
        "controls" => spec
            .domains
            .controls
            .get_or_insert_with(|| DomainSpec::new("controls", "controls.md", 1.0)),
        "camera" => spec
            .domains
            .camera
            .get_or_insert_with(|| DomainSpec::new("camera", "camera.md", 1.0)),
        "technical_architecture" => spec.domains.technical_architecture.get_or_insert_with(|| {
            DomainSpec::new("technical_architecture", "technical-architecture.md", 1.0)
        }),
        "art_style" => spec
            .domains
            .art_style
            .get_or_insert_with(|| DomainSpec::new("art_style", "art-style.md", 1.0)),
        "audio" => spec
            .domains
            .audio
            .get_or_insert_with(|| DomainSpec::new("audio", "audio.md", 1.0)),
        "narrative" => spec
            .domains
            .narrative
            .get_or_insert_with(|| DomainSpec::new("narrative", "narrative.md", 1.0)),
        "levels" => spec
            .domains
            .levels
            .get_or_insert_with(|| DomainSpec::new("levels", "levels.md", 1.0)),
        "engine" => spec
            .domains
            .engine
            .get_or_insert_with(|| DomainSpec::new("engine", "engine.md", 1.0)),
        "testing" => spec
            .domains
            .testing
            .get_or_insert_with(|| DomainSpec::new("testing", "testing.md", 1.0)),
        "build_release" => spec
            .domains
            .build_release
            .get_or_insert_with(|| DomainSpec::new("build_release", "build-release.md", 1.0)),
        "ui_ux" => spec
            .domains
            .ui_ux
            .get_or_insert_with(|| DomainSpec::new("ui_ux", "ui-ux.md", 1.0)),
        _ => {
            if !custom_has_provenance {
                bail!(
                    "custom domain '{domain}' requires a decision-ledger entry with rationale and source_question"
                );
            }
            spec.domains
                .custom
                .entry(canonical_domain_file_stem(domain))
                .or_insert_with(|| DomainSpec::new(domain, content_path, 1.0))
        }
    };

    domain_spec.fields.insert(key.to_string(), value);
    domain_spec.defined = true;
    sync_legacy_domain_aliases(&mut spec);
    lux_save(project_path, &spec)?;
    lux_load(project_path)
}

fn sync_legacy_domain_aliases(spec: &mut SpecProject) {
    if spec.domains.gdd.is_none() {
        spec.domains.gdd = spec.domains.design.clone();
    }
    if spec.domains.design.is_none() {
        spec.domains.design = spec.domains.gdd.clone();
    }

    if spec.domains.technical_architecture.is_none() {
        spec.domains.technical_architecture = spec.domains.architecture.clone();
    }
    if spec.domains.architecture.is_none() {
        spec.domains.architecture = spec.domains.technical_architecture.clone();
    }
}

fn canonical_domain_name(domain: &str) -> String {
    match domain {
        "design" => "gdd".to_string(),
        "architecture" => "technical_architecture".to_string(),
        "packages" => "engine".to_string(),
        "art-style" => "art_style".to_string(),
        "build-release" => "build_release".to_string(),
        "ui-ux" => "ui_ux".to_string(),
        other => other.replace('-', "_"),
    }
}

fn canonical_domain_file_stem(domain: &str) -> String {
    match canonical_domain_name(domain).as_str() {
        "art_style" => "art-style".to_string(),
        "technical_architecture" => "technical-architecture".to_string(),
        "build_release" => "build-release".to_string(),
        "ui_ux" => "ui-ux".to_string(),
        other => other.to_string(),
    }
}

fn legacy_domain_file_stem(domain: &str) -> String {
    match domain {
        "art_style" => "art-style".to_string(),
        "technical_architecture" | "architecture" => "architecture".to_string(),
        "engine" | "packages" => "packages".to_string(),
        "build_release" => "build-release".to_string(),
        "ui_ux" => "ui-ux".to_string(),
        "gdd" | "design" => "design".to_string(),
        other => other.replace('_', "-"),
    }
}

pub fn get_default_spec_json() -> Result<String> {
    Ok(include_str!("templates/spec.json").to_string())
}

pub fn render_markdown_template(
    template_name: &str,
    vars: &HashMap<String, String>,
) -> Result<String> {
    let mut rendered = match template_name {
        "gdd" | "gdd.md" => include_str!("templates/gdd.md").to_string(),
        "mechanics" | "mechanics.md" => include_str!("templates/mechanics.md").to_string(),
        "controls" | "controls.md" => include_str!("templates/controls.md").to_string(),
        "camera" | "camera.md" => include_str!("templates/camera.md").to_string(),
        "design" | "design.md" => include_str!("templates/design.md").to_string(),
        "architecture" | "architecture.md" => include_str!("templates/architecture.md").to_string(),
        "art-style" | "art_style" | "art-style.md" | "art_style.md" => {
            include_str!("templates/art-style.md").to_string()
        }
        "audio" | "audio.md" => include_str!("templates/audio.md").to_string(),
        "narrative" | "narrative.md" => include_str!("templates/narrative.md").to_string(),
        "levels" | "levels.md" => include_str!("templates/levels.md").to_string(),
        "ui-ux" | "ui_ux" | "ui-ux.md" | "ui_ux.md" => {
            include_str!("templates/ui-ux.md").to_string()
        }
        "technical-architecture"
        | "technical_architecture"
        | "technical-architecture.md"
        | "technical_architecture.md" => {
            include_str!("templates/technical-architecture.md").to_string()
        }
        "engine" | "engine.md" => include_str!("templates/engine.md").to_string(),
        "testing" | "testing.md" => include_str!("templates/testing.md").to_string(),
        "build-release" | "build_release" | "build-release.md" | "build_release.md" => {
            include_str!("templates/build-release.md").to_string()
        }
        "packages" | "packages.md" => include_str!("templates/engine.md").to_string(),
        _ => bail!("unknown markdown template: {template_name}"),
    };

    for (key, value) in vars {
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }

    Ok(rendered)
}

fn domain_templates() -> [(&'static str, &'static str); 16] {
    [
        ("gdd", include_str!("templates/gdd.md")),
        ("mechanics", include_str!("templates/mechanics.md")),
        ("controls", include_str!("templates/controls.md")),
        ("camera", include_str!("templates/camera.md")),
        ("art-style", include_str!("templates/art-style.md")),
        ("audio", include_str!("templates/audio.md")),
        ("narrative", include_str!("templates/narrative.md")),
        ("levels", include_str!("templates/levels.md")),
        ("ui-ux", include_str!("templates/ui-ux.md")),
        (
            "technical-architecture",
            include_str!("templates/technical-architecture.md"),
        ),
        ("engine", include_str!("templates/engine.md")),
        ("testing", include_str!("templates/testing.md")),
        ("build-release", include_str!("templates/build-release.md")),
        ("design", include_str!("templates/design.md")),
        ("architecture", include_str!("templates/architecture.md")),
        ("packages", include_str!("templates/packages.md")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestQuestionIo {
        presented_detection: bool,
        progress_reports: Vec<f64>,
    }

    impl TestQuestionIo {
        fn new() -> Self {
            Self {
                presented_detection: false,
                progress_reports: Vec::new(),
            }
        }
    }

    impl SpecQuestionIo for TestQuestionIo {
        fn present_detection(&mut self, _detection: Option<&UnityProjectDetection>) -> Result<()> {
            self.presented_detection = true;
            Ok(())
        }

        fn ask(
            &mut self,
            _question: &TargetedQuestion,
            _iteration: u32,
            _max_iterations: u32,
        ) -> Result<Option<String>> {
            Ok(None)
        }

        fn report_progress(&mut self, ambiguity_score: f64, _target_ambiguity: f64) -> Result<()> {
            self.progress_reports.push(ambiguity_score);
            Ok(())
        }
    }

    #[test]
    fn lux_init_on_existing_workspace_is_idempotent() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let first_path = lux_init(temp.path())?;
        let first_spec = lux_load(temp.path())?;

        let second_path = lux_init(temp.path())?;
        let second_spec = lux_load(temp.path())?;

        assert_eq!(first_path, second_path);
        assert_eq!(first_spec.project_id, second_spec.project_id);
        assert!(temp.path().join(".lux/domains/design.md").is_file());
        assert!(temp.path().join(".lux/glossary.md").is_file());
        Ok(())
    }

    #[test]
    fn lux_reinit_creates_backup_and_fresh_state() -> Result<()> {
        let temp = tempfile::tempdir()?;
        lux_init(temp.path())?;
        let original_spec = lux_load(temp.path())?;
        let marker_path = temp.path().join(".lux/logs/marker.txt");
        fs::write(&marker_path, "preserved")?;

        let lux_path = lux_reinit(temp.path())?;
        let fresh_spec = lux_load(temp.path())?;

        assert_eq!(lux_path, temp.path().join(".lux"));
        assert_ne!(original_spec.project_id, fresh_spec.project_id);
        assert!(!marker_path.exists());

        let backup_root = temp.path().join(".lux/backups");
        let mut backup_entries = fs::read_dir(&backup_root)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        backup_entries.sort();
        let backup_path = backup_entries
            .into_iter()
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("reinit-"))
            })
            .context("missing reinit backup")?;

        assert!(backup_path.join("spec.json").is_file());
        assert_eq!(
            fs::read_to_string(backup_path.join("logs/marker.txt"))?,
            "preserved"
        );
        Ok(())
    }

    #[test]
    fn lux_init_interactive_existing_spec_loads_and_continues() -> Result<()> {
        let temp = tempfile::tempdir()?;
        lux_init(temp.path())?;
        let mut spec = lux_load(temp.path())?;
        spec.project_name = "ExistingSpec".to_string();
        let project_id = spec.project_id.clone();
        lux_save(temp.path(), &spec)?;

        let mut io = TestQuestionIo::new();
        let path = lux_init_interactive(
            temp.path(),
            &mut io,
            LuxInitInteractiveOptions {
                interactive: false,
                target_ambiguity: 0.02,
                max_iterations: 1,
            },
        )?;
        let loaded = lux_load(temp.path())?;

        assert_eq!(path, temp.path().join(".lux"));
        assert_eq!(loaded.project_id, project_id);
        assert_eq!(loaded.project_name, "ExistingSpec");
        assert!(io.presented_detection);
        assert_eq!(io.progress_reports.len(), 1);
        Ok(())
    }
}
