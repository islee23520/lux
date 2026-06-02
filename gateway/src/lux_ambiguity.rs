use std::collections::HashMap;
use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::lux_spec::{DomainSpec, SpecProject};

const BUILT_IN_DOMAIN_COUNT: usize = 11;
const COMPLETION_WEIGHT: f64 = 0.40;
const AI_EVAL_WEIGHT: f64 = 0.35;
const AST_WEIGHT: f64 = 0.25;

// Ambiguity polarity: 0.0 = fully clear, 1.0 = maximally ambiguous
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

pub fn calculate_ambiguity(spec: &SpecProject) -> AmbiguityReport {
    let mut domain_scores = HashMap::new();
    let mut targeted_questions = Vec::new();
    let mut defined_count = 0usize;
    let spec_analysis = analyze_spec_fields(spec);

    for (name, domain) in built_in_domains(spec) {
        if is_domain_defined(domain) {
            defined_count += 1;
        }

        let analysis = analyze_domain(name, domain);
        if analysis.composite_score > 0.5 {
            targeted_questions.extend(analysis.questions.iter().take(3).enumerate().map(
                |(index, question)| TargetedQuestion {
                    domain: name.to_string(),
                    phase: primary_schell_phase(name).to_string(),
                    question: question.clone(),
                    priority: clamp_score(analysis.composite_score + (0.15 / (index + 1) as f64)),
                    default_value: None,
                    options: Vec::new(),
                },
            ));
        }
        domain_scores.insert(name.to_string(), analysis);
    }

    let SpecAmbiguity {
        composite_score: spec_ambiguity_score,
        questions: spec_questions,
    } = spec_analysis;
    targeted_questions.extend(spec_questions);

    let schell_phase_scores = calculate_schell_phase_scores(&domain_scores);
    let overall_score = clamp_score(
        (domain_scores
            .values()
            .map(|domain| domain.composite_score)
            .sum::<f64>()
            + spec_ambiguity_score)
            / (BUILT_IN_DOMAIN_COUNT as f64 + 1.0),
    );
    let completion_ratio = clamp_score(defined_count as f64 / BUILT_IN_DOMAIN_COUNT as f64);
    let contradictions = contradiction_recommendations(spec);
    let contradiction_score = if contradictions.is_empty() { 0.0 } else { 0.1 };
    let overall_score = clamp_score(overall_score.max(contradiction_score));
    let recommendations =
        build_recommendations(&domain_scores, &schell_phase_scores, contradictions);

    AmbiguityReport {
        overall_score,
        domain_scores,
        schell_phase_scores,
        completion_ratio,
        targeted_questions,
        recommendations,
    }
}

fn analyze_spec_fields(spec: &SpecProject) -> SpecAmbiguity {
    let fields = [
        (
            "unity",
            spec.unity.is_some(),
            1.0,
            "What Unity version is required?",
        ),
        (
            "targets",
            spec.targets
                .as_ref()
                .is_some_and(|targets| !targets.platforms.is_empty()),
            1.0,
            "What platforms will the game target?",
        ),
        (
            "packages",
            spec.packages
                .as_ref()
                .is_some_and(|packages| !packages.required.is_empty()),
            1.0,
            "Which packages are mandatory?",
        ),
        (
            "testing",
            spec.testing.is_some(),
            1.0,
            "What test framework and strategy?",
        ),
        (
            "glossary",
            spec.glossary.is_some(),
            0.25,
            "Is a glossary of project terms required?",
        ),
    ];

    let total_weight = fields.iter().map(|(_, _, weight, _)| *weight).sum::<f64>();
    let filled_weight = fields
        .iter()
        .filter(|(_, is_filled, _, _)| *is_filled)
        .map(|(_, _, weight, _)| *weight)
        .sum::<f64>();
    let clarity_score = if total_weight > 0.0 {
        clamp_score(filled_weight / total_weight)
    } else {
        0.0
    };
    let composite_score = ambiguity_from_clarity(clarity_score);

    let questions = fields
        .iter()
        .filter_map(|(_, is_filled, weight, question)| {
            if *is_filled {
                None
            } else {
                Some(TargetedQuestion {
                    domain: "spec".to_string(),
                    phase: "phase5_assessment".to_string(),
                    question: question.to_string(),
                    priority: clamp_score(composite_score + (*weight / total_weight) * 0.1),
                    default_value: None,
                    options: Vec::new(),
                })
            }
        })
        .collect::<Vec<_>>();

    let field_questions = spec_field_questions(spec);

    SpecAmbiguity {
        composite_score,
        questions: field_questions
            .into_iter()
            .chain(questions)
            .collect::<Vec<_>>(),
    }
}

#[derive(Clone, Debug)]
struct SpecAmbiguity {
    composite_score: f64,
    questions: Vec<TargetedQuestion>,
}

fn spec_field_questions(spec: &SpecProject) -> Vec<TargetedQuestion> {
    let mut questions = Vec::new();

    if spec.unity.as_ref().is_none_or(|unity| {
        unity
            .required_version
            .as_ref()
            .is_none_or(|value| value.trim().is_empty())
    }) {
        let detected_version = spec
            .unity
            .as_ref()
            .and_then(|u| u.detected_version.as_ref());
        questions.push(TargetedQuestion {
            domain: "spec".to_string(),
            phase: "unity.required_version".to_string(),
            question: "Which Unity version should this project require for all contributors?"
                .to_string(),
            priority: 0.9,
            default_value: detected_version.cloned(),
            options: Vec::new(),
        });
    }

    if spec
        .targets
        .as_ref()
        .is_none_or(|targets| targets.platforms.is_empty())
    {
        questions.push(TargetedQuestion {
            domain: "spec".to_string(),
            phase: "targets.platforms".to_string(),
            question: "Which build targets should Lux optimize for first? (comma-separated)"
                .to_string(),
            priority: 0.85,
            default_value: Some("windows, mac".to_string()),
            options: vec![
                "windows".to_string(),
                "mac".to_string(),
                "linux".to_string(),
                "android".to_string(),
                "ios".to_string(),
                "webgl".to_string(),
            ],
        });
    }

    if spec
        .packages
        .as_ref()
        .is_none_or(|packages| packages.required.is_empty())
    {
        let detected_packages: Vec<String> = spec
            .packages
            .as_ref()
            .map(|p| p.detected.iter().map(|d| d.name.clone()).collect())
            .unwrap_or_default();
        questions.push(TargetedQuestion {
            domain: "spec".to_string(),
            phase: "packages.required".to_string(),
            question:
                "Which Unity packages are mandatory for this project? (comma-separated package IDs)"
                    .to_string(),
            priority: 0.7,
            default_value: if detected_packages.is_empty() {
                None
            } else {
                Some(detected_packages.join(", "))
            },
            options: Vec::new(),
        });
    }

    if spec.testing.as_ref().is_none_or(|testing| {
        testing
            .framework
            .as_ref()
            .is_none_or(|value| value.trim().is_empty())
    }) {
        questions.push(TargetedQuestion {
            domain: "spec".to_string(),
            phase: "testing.strategy".to_string(),
            question: "What testing strategy should Lux enforce?".to_string(),
            priority: 0.75,
            default_value: Some("EditMode, PlayMode".to_string()),
            options: vec![
                "EditMode".to_string(),
                "PlayMode".to_string(),
                "coverage".to_string(),
                "smoke".to_string(),
                "integration".to_string(),
            ],
        });
    }

    questions
}

fn built_in_domains(
    spec: &SpecProject,
) -> [(&'static str, Option<&DomainSpec>); BUILT_IN_DOMAIN_COUNT] {
    [
        ("design", spec.domains.design.as_ref()),
        ("architecture", spec.domains.architecture.as_ref()),
        ("mechanics", spec.domains.mechanics.as_ref()),
        ("controls", spec.domains.controls.as_ref()),
        ("camera", spec.domains.camera.as_ref()),
        ("art_style", spec.domains.art_style.as_ref()),
        ("audio", spec.domains.audio.as_ref()),
        ("narrative", spec.domains.narrative.as_ref()),
        ("levels", spec.domains.levels.as_ref()),
        ("testing", spec.domains.testing.as_ref()),
        ("ui_ux", spec.domains.ui_ux.as_ref()),
    ]
}

fn analyze_domain(name: &str, domain: Option<&DomainSpec>) -> DomainAmbiguity {
    let expected_fields = expected_fields(name);
    let Some(domain) = domain else {
        let missing_fields = expected_fields
            .iter()
            .map(|field| (*field).to_string())
            .collect::<Vec<_>>();
        return DomainAmbiguity {
            domain_name: name.to_string(),
            completion_ratio: 0.0,
            ai_eval_score: 0.0,
            ast_parsability: 0.0,
            composite_score: 1.0,
            questions: questions_for_missing_fields(name, &missing_fields),
            missing_fields,
        };
    };

    let markdown = read_markdown(&domain.content_path);
    let (completion_ratio, missing_fields) =
        completion_ratio(domain, &expected_fields, markdown.as_deref());
    let ai_eval_score = ai_eval_score(name, domain, &expected_fields, markdown.as_deref());
    let ast_parsability = ast_parsability(markdown.as_deref());
    let composite_score = composite_score(completion_ratio, ai_eval_score, ast_parsability);
    let questions = questions_for_missing_fields(name, &missing_fields);

    DomainAmbiguity {
        domain_name: name.to_string(),
        completion_ratio,
        ai_eval_score,
        ast_parsability,
        composite_score,
        missing_fields,
        questions,
    }
}

fn is_domain_defined(domain: Option<&DomainSpec>) -> bool {
    domain.is_some_and(|domain| domain.defined)
}

fn completion_ratio(
    domain: &DomainSpec,
    expected_fields: &[&str],
    markdown: Option<&str>,
) -> (f64, Vec<String>) {
    if !domain.defined {
        return (
            0.0,
            expected_fields
                .iter()
                .map(|field| (*field).to_string())
                .collect(),
        );
    }

    let mut filled = 0usize;
    let mut missing_fields = Vec::new();
    for field in expected_fields {
        if domain.fields.get(*field).is_some_and(value_is_filled) {
            filled += 1;
        } else {
            missing_fields.push((*field).to_string());
        }
    }

    if markdown.is_some_and(|content| content.trim().chars().count() > 100) {
        filled += 1;
    } else {
        missing_fields.push("content_path".to_string());
    }

    let total = expected_fields.len() + 1;
    (clamp_score(filled as f64 / total as f64), missing_fields)
}

fn value_is_filled(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(_) | Value::Number(_) => true,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

fn ai_eval_score(
    name: &str,
    domain: &DomainSpec,
    expected_fields: &[&str],
    markdown: Option<&str>,
) -> f64 {
    if !domain.defined {
        return 0.0;
    }

    let filled_fields = expected_fields
        .iter()
        .filter(|field| domain.fields.get(**field).is_some_and(value_is_filled))
        .count();
    if filled_fields == 0 {
        return 0.2;
    }

    let field_ratio = filled_fields as f64 / expected_fields.len() as f64;
    let keyword_score = keyword_score(name, markdown);
    let base = if filled_fields == expected_fields.len() {
        0.8 + (0.2 * keyword_score)
    } else {
        0.4 + (0.3 * field_ratio) + (0.1 * keyword_score)
    };
    clamp_score(base)
}

fn keyword_score(name: &str, markdown: Option<&str>) -> f64 {
    let Some(markdown) = markdown else {
        return 0.0;
    };
    let lower = markdown.to_lowercase();
    let keywords = domain_keywords(name);
    if keywords.is_empty() {
        return 0.0;
    }
    let matches = keywords
        .iter()
        .filter(|keyword| lower.contains(**keyword))
        .count();
    clamp_score(matches as f64 / keywords.len() as f64)
}

fn ast_parsability(markdown: Option<&str>) -> f64 {
    let Some(markdown) = markdown else {
        return 0.0;
    };
    let trimmed = markdown.trim();
    if trimmed.is_empty() {
        return 0.0;
    }

    let line_count = trimmed.lines().count().max(1) as f64;
    let header_count = trimmed
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            line.starts_with("# ") || line.starts_with("## ") || line.starts_with("### ")
        })
        .count() as f64;
    let bullet_count = trimmed
        .lines()
        .filter(|line| line.trim_start().starts_with("- "))
        .count() as f64;
    let length_score = clamp_score(trimmed.chars().count() as f64 / 600.0);
    let structure_density = ((header_count * 2.0) + bullet_count) / line_count;

    clamp_score((structure_density * 0.7) + (length_score * 0.3))
}

fn composite_score(completion: f64, ai_eval: f64, ast: f64) -> f64 {
    ambiguity_from_clarity(
        (COMPLETION_WEIGHT * completion) + (AI_EVAL_WEIGHT * ai_eval) + (AST_WEIGHT * ast),
    )
}

fn ambiguity_from_clarity(clarity_score: f64) -> f64 {
    clamp_score(1.0 - clamp_score(clarity_score))
}

fn calculate_schell_phase_scores(
    domain_scores: &HashMap<String, DomainAmbiguity>,
) -> HashMap<String, f64> {
    let mut scores = HashMap::new();
    scores.insert(
        "phase1_experience".to_string(),
        average_domains(
            domain_scores,
            &[
                "design",
                "controls",
                "camera",
                "art_style",
                "audio",
                "ui_ux",
            ],
        ),
    );
    scores.insert(
        "phase2_tetrad".to_string(),
        average_domains(
            domain_scores,
            &["design", "narrative", "art_style", "architecture"],
        ),
    );
    scores.insert(
        "phase3_core_loop".to_string(),
        average_domains(
            domain_scores,
            &[
                "design",
                "mechanics",
                "controls",
                "camera",
                "levels",
                "ui_ux",
            ],
        ),
    );
    scores.insert(
        "phase4_motivation".to_string(),
        average_domains(domain_scores, &["design", "narrative", "levels"]),
    );
    scores.insert(
        "phase5_assessment".to_string(),
        average_domains(
            domain_scores,
            &["design", "architecture", "testing", "levels", "ui_ux"],
        ),
    );
    scores
}

fn average_domains(domain_scores: &HashMap<String, DomainAmbiguity>, domains: &[&str]) -> f64 {
    clamp_score(
        domains
            .iter()
            .filter_map(|domain| domain_scores.get(*domain))
            .map(|domain| domain.composite_score)
            .sum::<f64>()
            / domains.len() as f64,
    )
}

fn build_recommendations(
    domain_scores: &HashMap<String, DomainAmbiguity>,
    schell_phase_scores: &HashMap<String, f64>,
    contradictions: Vec<String>,
) -> Vec<String> {
    let mut recommendations = domain_scores
        .values()
        .filter(|domain| domain.composite_score > 0.5)
        .map(|domain| {
            format!(
                "Clarify {} by answering its targeted questions.",
                domain.domain_name
            )
        })
        .collect::<Vec<_>>();

    for (phase, score) in schell_phase_scores {
        if *score > 0.5 {
            recommendations.push(format!("Reduce {phase} ambiguity before implementation."));
        }
    }
    recommendations.extend(contradictions);

    recommendations.sort();
    recommendations.dedup();
    recommendations
}

fn contradiction_recommendations(spec: &SpecProject) -> Vec<String> {
    let mut grouped = HashMap::<String, Vec<String>>::new();
    for decision in &spec.dialectic.decisions {
        let Some(source_question) = decision.source_question.as_ref() else {
            continue;
        };
        let domain = decision.domain.as_deref().unwrap_or("spec");
        let key = format!(
            "{}::{}",
            domain.trim().to_lowercase(),
            source_question.trim().to_lowercase()
        );
        grouped.entry(key).or_default().push(decision.text.clone());
    }

    let mut recommendations = Vec::new();
    for (key, answers) in grouped {
        let mut unique_answers = answers
            .into_iter()
            .map(|answer| answer.trim().to_lowercase())
            .filter(|answer| !answer.is_empty())
            .collect::<Vec<_>>();
        unique_answers.sort();
        unique_answers.dedup();
        if unique_answers.len() > 1 {
            recommendations.push(format!(
                "Resolve contradiction in {key} before reducing ambiguity."
            ));
        }
    }
    recommendations
}

fn read_markdown(path: &str) -> Option<String> {
    if path.trim().is_empty() {
        return None;
    }
    fs::read_to_string(path)
        .ok()
        .filter(|content| !content.trim().is_empty())
}

fn expected_fields(name: &str) -> Vec<&'static str> {
    match name {
        "design" => vec![
            "core_loop",
            "genre",
            "player_count",
            "session_length",
            "win_condition",
        ],
        "architecture" => vec!["engine", "platform", "networking", "data_storage"],
        "mechanics" => vec![
            "movement_model",
            "interaction_rules",
            "resource_rules",
            "progression_rules",
        ],
        "controls" => vec!["input_devices", "action_map", "rebinding", "accessibility"],
        "camera" => vec![
            "camera_mode",
            "follow_rules",
            "framing",
            "occlusion_strategy",
        ],
        "art_style" => vec![
            "visual_style",
            "color_palette",
            "resolution",
            "animation_style",
        ],
        "audio" => vec!["music_style", "sfx_list", "ambient_sounds", "dynamic_audio"],
        "narrative" => vec![
            "story_arc",
            "characters",
            "dialogue_system",
            "world_building",
        ],
        "levels" => vec!["level_count", "difficulty_curve", "level_generation"],
        "testing" => vec![
            "editmode_coverage",
            "playmode_smoke",
            "manual_qa_channel",
            "evidence_gate",
        ],
        "ui_ux" => vec!["hud_layout", "menu_flow", "accessibility", "input_mapping"],
        _ => Vec::new(),
    }
}

fn domain_keywords(name: &str) -> Vec<&'static str> {
    match name {
        "design" => vec!["genre", "mechanic", "loop", "player", "win"],
        "architecture" => vec!["engine", "platform", "network", "storage", "system"],
        "mechanics" => vec![
            "movement",
            "interaction",
            "resource",
            "progression",
            "combat",
        ],
        "controls" => vec!["input", "action", "rebinding", "accessibility", "device"],
        "camera" => vec!["camera", "follow", "framing", "occlusion", "viewport"],
        "art_style" => vec!["visual", "color", "resolution", "animation", "style"],
        "audio" => vec!["music", "sfx", "ambient", "dynamic", "sound"],
        "narrative" => vec!["story", "character", "dialogue", "world", "arc"],
        "levels" => vec!["level", "difficulty", "procedural", "handcrafted", "curve"],
        "testing" => vec!["editmode", "playmode", "manual", "evidence", "qa"],
        "ui_ux" => vec!["hud", "menu", "accessibility", "input", "flow"],
        _ => Vec::new(),
    }
}

fn questions_for_missing_fields(name: &str, missing_fields: &[String]) -> Vec<String> {
    let templates = question_templates(name);
    let mut questions = missing_fields
        .iter()
        .filter_map(|field| templates.get(field.as_str()).copied())
        .map(str::to_string)
        .collect::<Vec<_>>();

    for question in fallback_questions(name) {
        if questions.len() >= 3 {
            break;
        }
        if !questions.iter().any(|existing| existing == question) {
            questions.push((*question).to_string());
        }
    }

    questions
}

fn question_templates(name: &str) -> HashMap<&'static str, &'static str> {
    match name {
        "design" => HashMap::from([
            ("core_loop", "What is the core game loop?"),
            ("genre", "What genre does this game belong to?"),
            ("player_count", "How many players are supported?"),
            ("session_length", "How long should a typical session last?"),
            ("win_condition", "What is the primary win condition?"),
            ("content_path", "Where is the detailed design markdown?"),
        ]),
        "architecture" => HashMap::from([
            ("engine", "Which engine and version will ship the game?"),
            ("platform", "Which platforms must be supported?"),
            (
                "networking",
                "Does the game require networking or online services?",
            ),
            (
                "data_storage",
                "What data needs to be stored locally or remotely?",
            ),
            (
                "content_path",
                "Where is the technical architecture markdown?",
            ),
        ]),
        "mechanics" => HashMap::from([
            (
                "movement_model",
                "What movement model defines player traversal?",
            ),
            (
                "interaction_rules",
                "What interaction rules govern player actions?",
            ),
            ("resource_rules", "Which resources can change during play?"),
            (
                "progression_rules",
                "How does the mechanics progression unlock or scale?",
            ),
            ("content_path", "Where is the mechanics markdown?"),
        ]),
        "controls" => HashMap::from([
            ("input_devices", "Which input devices must be supported?"),
            (
                "action_map",
                "What action map connects controls to mechanics?",
            ),
            ("rebinding", "Which controls must be rebindable?"),
            (
                "accessibility",
                "Which control accessibility requirements are mandatory?",
            ),
            ("content_path", "Where is the controls markdown?"),
        ]),
        "camera" => HashMap::from([
            ("camera_mode", "What camera mode should the game use?"),
            (
                "follow_rules",
                "How should the camera follow the player or focus target?",
            ),
            ("framing", "What framing rules keep gameplay readable?"),
            (
                "occlusion_strategy",
                "How should the camera handle occlusion?",
            ),
            ("content_path", "Where is the camera markdown?"),
        ]),
        "art_style" => HashMap::from([
            ("visual_style", "What visual style should the game use?"),
            ("color_palette", "What color palette defines the mood?"),
            (
                "resolution",
                "What target resolution or asset scale is required?",
            ),
            (
                "animation_style",
                "What animation style should characters and UI use?",
            ),
            ("content_path", "Where is the art direction markdown?"),
        ]),
        "audio" => HashMap::from([
            ("music_style", "What music style supports the experience?"),
            (
                "sfx_list",
                "Which sound effects are required for core interactions?",
            ),
            ("ambient_sounds", "What ambient sounds define each space?"),
            ("dynamic_audio", "How should audio react to gameplay state?"),
            ("content_path", "Where is the audio design markdown?"),
        ]),
        "narrative" => HashMap::from([
            ("story_arc", "What is the main story arc?"),
            ("characters", "Who are the central characters?"),
            ("dialogue_system", "How does dialogue appear or branch?"),
            (
                "world_building",
                "What world-building rules shape the setting?",
            ),
            ("content_path", "Where is the narrative markdown?"),
        ]),
        "levels" => HashMap::from([
            ("level_count", "How many levels or spaces are planned?"),
            (
                "difficulty_curve",
                "How should difficulty progress over time?",
            ),
            (
                "level_generation",
                "Are levels procedural, handcrafted, or hybrid?",
            ),
            ("content_path", "Where is the level design markdown?"),
        ]),
        "testing" => HashMap::from([
            ("editmode_coverage", "Which EditMode tests are required?"),
            (
                "playmode_smoke",
                "Which PlayMode smoke path proves the core loop?",
            ),
            (
                "manual_qa_channel",
                "Which manual QA channel proves engine behavior?",
            ),
            (
                "evidence_gate",
                "What evidence must block completion when missing?",
            ),
            ("content_path", "Where is the testing strategy markdown?"),
        ]),
        "ui_ux" => HashMap::from([
            ("hud_layout", "What information belongs on the HUD?"),
            (
                "menu_flow",
                "What is the menu flow from launch to gameplay?",
            ),
            (
                "accessibility",
                "Which accessibility requirements are mandatory?",
            ),
            ("input_mapping", "What input mappings must be supported?"),
            ("content_path", "Where is the UI/UX markdown?"),
        ]),
        _ => HashMap::new(),
    }
}

fn fallback_questions(name: &str) -> Vec<&'static str> {
    match name {
        "design" => vec![
            "What is the core game loop?",
            "What genre does this game belong to?",
            "What is the primary win condition?",
        ],
        "architecture" => vec![
            "Which engine and version will ship the game?",
            "Which platforms must be supported?",
            "Does the game require networking or online services?",
        ],
        "mechanics" => vec![
            "What movement model defines player traversal?",
            "What interaction rules govern player actions?",
            "How does the mechanics progression unlock or scale?",
        ],
        "controls" => vec![
            "Which input devices must be supported?",
            "What action map connects controls to mechanics?",
            "Which controls must be rebindable?",
        ],
        "camera" => vec![
            "What camera mode should the game use?",
            "How should the camera follow the player or focus target?",
            "What framing rules keep gameplay readable?",
        ],
        "art_style" => vec![
            "What visual style should the game use?",
            "What color palette defines the mood?",
            "What animation style should characters and UI use?",
        ],
        "audio" => vec![
            "What music style supports the experience?",
            "Which sound effects are required for core interactions?",
            "How should audio react to gameplay state?",
        ],
        "narrative" => vec![
            "What is the main story arc?",
            "Who are the central characters?",
            "How does dialogue appear or branch?",
        ],
        "levels" => vec![
            "How many levels or spaces are planned?",
            "How should difficulty progress over time?",
            "Are levels procedural, handcrafted, or hybrid?",
        ],
        "testing" => vec![
            "Which EditMode tests are required?",
            "Which PlayMode smoke path proves the core loop?",
            "Which manual QA channel proves engine behavior?",
        ],
        "ui_ux" => vec![
            "What information belongs on the HUD?",
            "What is the menu flow from launch to gameplay?",
            "Which accessibility requirements are mandatory?",
        ],
        _ => Vec::new(),
    }
}

fn primary_schell_phase(name: &str) -> &'static str {
    match name {
        "design" | "mechanics" | "controls" | "camera" | "levels" | "ui_ux" => "phase3_core_loop",
        "architecture" | "art_style" | "narrative" => "phase2_tetrad",
        "audio" => "phase1_experience",
        _ => "phase5_assessment",
    }
}

fn clamp_score(score: f64) -> f64 {
    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::composite_score;

    #[test]
    fn composite_uses_expected_weights() {
        let score = composite_score(0.5, 0.6, 0.8);
        assert!((score - 0.39).abs() < f64::EPSILON);
    }
}
