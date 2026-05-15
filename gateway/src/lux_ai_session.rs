use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::lux_spec::{
    lux_load, AssessmentResult, DomainSpec, PhaseResult, PillarRating, PillarStatus, SpecProject,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionPhase {
    Phase1Experience,
    Phase2Tetrad,
    Phase3CoreLoop,
    Phase4Motivation,
    Phase5Assessment,
    Completed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnRole {
    Ai,
    User,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AiSession {
    pub session_id: String,
    pub project_path: PathBuf,
    pub phase: SessionPhase,
    pub turn_count: u32,
    pub max_turns: u32,
    pub history: Vec<SessionTurn>,
    pub started_at: String,
    pub status: SessionStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionTurn {
    pub turn: u32,
    pub role: TurnRole,
    pub content: String,
    pub phase: SessionPhase,
    pub timestamp: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AiResponse {
    pub message: String,
    pub phase: SessionPhase,
    pub phase_complete: bool,
    pub questions_remaining: u32,
    pub spec_updated: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseCompletion {
    pub phase: SessionPhase,
    pub complete: bool,
    pub score: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PersistedSessionLine {
    session_id: String,
    project_path: PathBuf,
    phase: SessionPhase,
    turn_count: u32,
    max_turns: u32,
    started_at: String,
    status: SessionStatus,
    turn: Option<SessionTurn>,
}

pub fn create_session(project_path: &Path) -> Result<AiSession> {
    let phase = lux_load(project_path)
        .map(|spec| determine_starting_phase(&spec))
        .unwrap_or(SessionPhase::Phase1Experience);

    Ok(AiSession {
        session_id: Uuid::new_v4().to_string(),
        project_path: project_path.to_path_buf(),
        phase,
        turn_count: 0,
        max_turns: 50,
        history: Vec::new(),
        started_at: now(),
        status: SessionStatus::Active,
    })
}

pub fn process_message(session: &mut AiSession, user_message: &str) -> Result<AiResponse> {
    if !matches!(session.status, SessionStatus::Active) {
        bail!("session is not active");
    }
    if matches!(session.phase, SessionPhase::Completed) || session.turn_count >= session.max_turns {
        complete_session(session);
        return Ok(limit_response());
    }

    add_turn(session, TurnRole::User, user_message.trim().to_string())?;
    if session.turn_count >= session.max_turns {
        complete_session(session);
        return Ok(limit_response());
    }

    let completion = evaluate_phase_completion(session)?;
    let mut response_phase = session.phase.clone();
    let mut message = dialectic_response(session, user_message, completion.complete)?;

    if completion.complete {
        let previous = session.phase.clone();
        let next = advance_phase(session)?;
        response_phase = next.clone();
        message = if matches!(next, SessionPhase::Completed) {
            format!(
                "Synthesis: {}\n\nThe five-phase Schell refinement session is complete.",
                synthesize_phase(session, &previous)
            )
        } else {
            format!(
                "Synthesis: {}\n\nTransition: {} is sufficiently grounded. Next lens: {}.\n\n{}",
                synthesize_phase(session, &previous),
                phase_label(&previous),
                phase_label(&next),
                get_next_question(session)?
            )
        };
    }

    if !matches!(session.phase, SessionPhase::Completed) && session.turn_count < session.max_turns {
        add_turn(session, TurnRole::Ai, message.clone())?;
    } else {
        complete_session(session);
    }

    Ok(AiResponse {
        message,
        phase: response_phase,
        phase_complete: completion.complete,
        questions_remaining: questions_remaining(session),
        spec_updated: false,
    })
}

pub fn get_next_question(session: &AiSession) -> Result<String> {
    let questions = questions_for_phase(&session.phase);
    if questions.is_empty() {
        return Ok("The session is complete. Apply the synthesis to the Lux spec.".to_string());
    }
    let index = ai_question_count(session, &session.phase) % questions.len();
    Ok(format!(
        "{}: {}",
        phase_label(&session.phase),
        questions[index]
    ))
}

pub fn evaluate_phase_completion(session: &AiSession) -> Result<PhaseCompletion> {
    let phase = session.phase.clone();
    if matches!(phase, SessionPhase::Completed) {
        return Ok(PhaseCompletion {
            phase,
            complete: true,
            score: 1.0,
        });
    }

    let answers = substantive_user_answers(session, &phase);
    let keyword_hits = phase_keyword_hits(session, &phase);
    let answer_score = (answers as f64 / 3.0).min(1.0);
    let keyword_score = (keyword_hits as f64 / 4.0).min(1.0);
    let score = ((answer_score * 0.75) + (keyword_score * 0.25)).min(1.0);

    Ok(PhaseCompletion {
        phase,
        complete: answers >= 3,
        score,
    })
}

pub fn advance_phase(session: &mut AiSession) -> Result<SessionPhase> {
    let completion = evaluate_phase_completion(session)?;
    if !completion.complete {
        return Ok(session.phase.clone());
    }
    session.phase = match session.phase {
        SessionPhase::Phase1Experience => SessionPhase::Phase2Tetrad,
        SessionPhase::Phase2Tetrad => SessionPhase::Phase3CoreLoop,
        SessionPhase::Phase3CoreLoop => SessionPhase::Phase4Motivation,
        SessionPhase::Phase4Motivation => SessionPhase::Phase5Assessment,
        SessionPhase::Phase5Assessment | SessionPhase::Completed => {
            session.status = SessionStatus::Completed;
            SessionPhase::Completed
        }
    };
    Ok(session.phase.clone())
}

pub fn apply_session_to_spec(session: &AiSession, spec: &mut SpecProject) -> Result<()> {
    let phase1 = joined_answers(session, &SessionPhase::Phase1Experience);
    if !phase1.is_empty() {
        spec.schell_evaluation.phase1_experience = phase_result(
            "Experience Lens",
            phase1.clone(),
            evaluate_stored_phase(session, &SessionPhase::Phase1Experience),
        );
        upsert_custom_domain(
            spec,
            "experience",
            "domains/experience.md",
            "player_experience",
            phase1,
        );
    }

    let phase2 = joined_answers(session, &SessionPhase::Phase2Tetrad);
    if !phase2.is_empty() {
        let score = evaluate_stored_phase(session, &SessionPhase::Phase2Tetrad);
        spec.schell_evaluation.phase2_tetrad.mechanics = pillar_rating(&phase2, score);
        spec.schell_evaluation.phase2_tetrad.story = pillar_rating(&phase2, score);
        spec.schell_evaluation.phase2_tetrad.aesthetics = pillar_rating(&phase2, score);
        spec.schell_evaluation.phase2_tetrad.technology = pillar_rating(&phase2, score);
        spec.schell_evaluation.phase2_tetrad.harmony_score = score;
        upsert_custom_domain(spec, "tetrad", "domains/tetrad.md", "schell_tetrad", phase2);
    }

    let phase3 = joined_answers(session, &SessionPhase::Phase3CoreLoop);
    if !phase3.is_empty() {
        spec.schell_evaluation.phase3_core_loop = phase_result(
            "Core Loop Stress Test",
            phase3.clone(),
            evaluate_stored_phase(session, &SessionPhase::Phase3CoreLoop),
        );
        upsert_custom_domain(
            spec,
            "core_loop",
            "domains/core-loop.md",
            "core_loop",
            phase3,
        );
    }

    let phase4 = joined_answers(session, &SessionPhase::Phase4Motivation);
    if !phase4.is_empty() {
        spec.schell_evaluation.phase4_motivation = phase_result(
            "Player Motivation",
            phase4.clone(),
            evaluate_stored_phase(session, &SessionPhase::Phase4Motivation),
        );
        upsert_custom_domain(
            spec,
            "motivation",
            "domains/motivation.md",
            "player_motivation",
            phase4,
        );
    }

    let phase5 = joined_answers(session, &SessionPhase::Phase5Assessment);
    if !phase5.is_empty() {
        let score = evaluate_stored_phase(session, &SessionPhase::Phase5Assessment);
        spec.schell_evaluation.phase5_assessment = AssessmentResult {
            status: status_for_score(score),
            viability_score: score,
            strengths: extract_sentences_with_keywords(
                &phase5,
                &["strength", "works", "fun", "clear"],
            ),
            risks: extract_sentences_with_keywords(
                &phase5,
                &["risk", "weak", "hard", "scope", "unclear"],
            ),
            recommendations: extract_sentences_with_keywords(
                &phase5,
                &["should", "need", "reduce", "focus", "test"],
            ),
            summary: Some(phase5),
        };
    }

    spec.updated_at = now();
    spec.overall_ambiguity = (1.0 - average_completed_score(session)).clamp(0.0, 1.0);
    spec.validate().map_err(anyhow::Error::msg)
}

pub fn save_session(session: &AiSession) -> Result<()> {
    let sessions_path = session.project_path.join(".lux/sessions");
    fs::create_dir_all(&sessions_path)
        .with_context(|| format!("failed to create {}", sessions_path.display()))?;
    let session_path = sessions_path.join(format!("{}.jsonl", session.session_id));
    let mut file = File::create(&session_path)
        .with_context(|| format!("failed to create {}", session_path.display()))?;

    if session.history.is_empty() {
        writeln!(
            file,
            "{}",
            serde_json::to_string(&persisted_line(session, None))?
        )?;
    } else {
        for turn in &session.history {
            writeln!(
                file,
                "{}",
                serde_json::to_string(&persisted_line(session, Some(turn.clone())))?
            )?;
        }
    }
    Ok(())
}

pub fn load_session(project_path: &Path, session_id: &str) -> Result<AiSession> {
    let session_path = project_path
        .join(".lux/sessions")
        .join(format!("{session_id}.jsonl"));
    let file = File::open(&session_path)
        .with_context(|| format!("failed to open {}", session_path.display()))?;
    let reader = BufReader::new(file);
    let mut last: Option<PersistedSessionLine> = None;
    let mut history = Vec::new();

    for line in reader.lines() {
        let line = line.with_context(|| format!("failed to read {}", session_path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let persisted: PersistedSessionLine = serde_json::from_str(&line).with_context(|| {
            format!("failed to parse session line in {}", session_path.display())
        })?;
        if let Some(turn) = &persisted.turn {
            history.push(turn.clone());
        }
        last = Some(persisted);
    }

    let persisted = last.context("session file did not contain session data")?;
    Ok(AiSession {
        session_id: persisted.session_id,
        project_path: persisted.project_path,
        phase: persisted.phase,
        turn_count: persisted.turn_count,
        max_turns: persisted.max_turns,
        history,
        started_at: persisted.started_at,
        status: persisted.status,
    })
}

fn add_turn(session: &mut AiSession, role: TurnRole, content: String) -> Result<()> {
    if content.trim().is_empty() {
        bail!("session message cannot be empty");
    }
    if session.turn_count >= session.max_turns {
        complete_session(session);
        bail!("session reached max turns");
    }
    session.turn_count += 1;
    session.history.push(SessionTurn {
        turn: session.turn_count,
        role,
        content,
        phase: session.phase.clone(),
        timestamp: now(),
    });
    Ok(())
}

fn determine_starting_phase(spec: &SpecProject) -> SessionPhase {
    if !phase_result_has_data(&spec.schell_evaluation.phase1_experience) {
        SessionPhase::Phase1Experience
    } else if !tetrad_has_data(spec) {
        SessionPhase::Phase2Tetrad
    } else if !phase_result_has_data(&spec.schell_evaluation.phase3_core_loop) {
        SessionPhase::Phase3CoreLoop
    } else if !phase_result_has_data(&spec.schell_evaluation.phase4_motivation) {
        SessionPhase::Phase4Motivation
    } else if spec.schell_evaluation.phase5_assessment.status == PillarStatus::Missing {
        SessionPhase::Phase5Assessment
    } else {
        SessionPhase::Completed
    }
}

fn phase_result_has_data(result: &PhaseResult) -> bool {
    result.status != PillarStatus::Missing
        || result
            .summary
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
}

fn tetrad_has_data(spec: &SpecProject) -> bool {
    let tetrad = &spec.schell_evaluation.phase2_tetrad;
    [
        &tetrad.mechanics,
        &tetrad.story,
        &tetrad.aesthetics,
        &tetrad.technology,
    ]
    .iter()
    .all(|rating| rating.status != PillarStatus::Missing)
}

fn dialectic_response(
    session: &AiSession,
    user_message: &str,
    phase_complete: bool,
) -> Result<String> {
    let answer_count = user_answer_count(session, &session.phase);
    if phase_complete {
        return Ok(format!(
            "Synthesis: {}",
            synthesize_phase(session, &session.phase)
        ));
    }

    let challenge = challenge_for_answer(user_message, &session.phase);
    let question = get_next_question(session)?;
    Ok(match answer_count % 3 {
        1 => format!("Question: {question}"),
        2 => format!("Rebuttal: {challenge}\n\nQuestion: {question}"),
        _ => format!(
            "Synthesis: {}\n\nQuestion: {question}",
            synthesize_phase(session, &session.phase)
        ),
    })
}

fn challenge_for_answer(answer: &str, phase: &SessionPhase) -> String {
    let short = compact_excerpt(answer, 96);
    match phase {
        SessionPhase::Phase1Experience => format!("If '{short}' is the intended experience, what moment proves the player actually feels it instead of only reading about it?"),
        SessionPhase::Phase2Tetrad => format!("Which tetrad pillar would break first if '{short}' changed, and how would the other pillars compensate?"),
        SessionPhase::Phase3CoreLoop => format!("What prevents the repeated action behind '{short}' from becoming predictable after five minutes?"),
        SessionPhase::Phase4Motivation => format!("Why would a tired player choose to continue pursuing '{short}' instead of stopping at the next safe point?"),
        SessionPhase::Phase5Assessment => format!("What evidence would disprove your confidence in '{short}', and how cheaply can you test it?"),
        SessionPhase::Completed => "The session is complete; no rebuttal is needed.".to_string(),
    }
}

fn synthesize_phase(session: &AiSession, phase: &SessionPhase) -> String {
    let answers: Vec<&str> = session
        .history
        .iter()
        .filter(|turn| turn.role == TurnRole::User && &turn.phase == phase)
        .map(|turn| turn.content.trim())
        .filter(|content| !content.is_empty())
        .collect();
    if answers.is_empty() {
        return "No player-facing claim has been captured yet.".to_string();
    }
    let first = compact_excerpt(answers[0], 120);
    let last = compact_excerpt(answers[answers.len() - 1], 120);
    if answers.len() == 1 {
        format!("The current claim is '{first}'. It needs contrast, evidence, and a playable test.")
    } else {
        format!("Across {} answer(s), the strongest thread moves from '{first}' toward '{last}'. Convert that into a testable design claim.", answers.len())
    }
}

fn substantive_user_answers(session: &AiSession, phase: &SessionPhase) -> usize {
    session
        .history
        .iter()
        .filter(|turn| turn.role == TurnRole::User && &turn.phase == phase)
        .filter(|turn| is_substantive(&turn.content))
        .count()
}

fn user_answer_count(session: &AiSession, phase: &SessionPhase) -> usize {
    session
        .history
        .iter()
        .filter(|turn| turn.role == TurnRole::User && &turn.phase == phase)
        .count()
}

fn ai_question_count(session: &AiSession, phase: &SessionPhase) -> usize {
    session
        .history
        .iter()
        .filter(|turn| turn.role == TurnRole::Ai && &turn.phase == phase)
        .count()
}

fn phase_keyword_hits(session: &AiSession, phase: &SessionPhase) -> usize {
    let words = keywords_for_phase(phase);
    let joined = joined_answers(session, phase).to_lowercase();
    words.iter().filter(|word| joined.contains(**word)).count()
}

fn is_substantive(content: &str) -> bool {
    let trimmed = content.trim();
    trimmed.chars().count() >= 20 || trimmed.split_whitespace().count() >= 4
}

fn questions_remaining(session: &AiSession) -> u32 {
    if matches!(session.phase, SessionPhase::Completed) {
        return 0;
    }
    3_u32.saturating_sub(substantive_user_answers(session, &session.phase) as u32)
}

fn joined_answers(session: &AiSession, phase: &SessionPhase) -> String {
    session
        .history
        .iter()
        .filter(|turn| turn.role == TurnRole::User && &turn.phase == phase)
        .map(|turn| turn.content.trim())
        .filter(|content| !content.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn evaluate_stored_phase(session: &AiSession, phase: &SessionPhase) -> f64 {
    let mut shadow = session.clone();
    shadow.phase = phase.clone();
    evaluate_phase_completion(&shadow)
        .map(|completion| {
            completion
                .score
                .max(if completion.complete { 0.7 } else { 0.35 })
        })
        .unwrap_or(0.35)
        .clamp(0.0, 1.0)
}

fn phase_result(name: &str, summary: String, score: f64) -> PhaseResult {
    PhaseResult {
        name: name.to_string(),
        status: status_for_score(score),
        summary: Some(summary),
        score,
        questions: Vec::new(),
    }
}

fn pillar_rating(description: &str, score: f64) -> PillarRating {
    PillarRating {
        status: status_for_score(score),
        description: Some(description.to_string()),
        score,
    }
}

fn status_for_score(score: f64) -> PillarStatus {
    if score >= 0.7 {
        PillarStatus::Strong
    } else if score > 0.0 {
        PillarStatus::NeedsWork
    } else {
        PillarStatus::Missing
    }
}

fn upsert_custom_domain(
    spec: &mut SpecProject,
    name: &str,
    content_path: &str,
    field: &str,
    content: String,
) {
    let mut fields = HashMap::new();
    fields.insert(field.to_string(), Value::String(content));
    let mut domain = DomainSpec::new(name, content_path, 0.35);
    domain.fields = fields;
    domain.last_evaluated = Some(now());
    domain.defined = true;
    spec.domains.custom.insert(name.to_string(), domain);
}

fn average_completed_score(session: &AiSession) -> f64 {
    let phases = [
        SessionPhase::Phase1Experience,
        SessionPhase::Phase2Tetrad,
        SessionPhase::Phase3CoreLoop,
        SessionPhase::Phase4Motivation,
        SessionPhase::Phase5Assessment,
    ];
    let scores: Vec<f64> = phases
        .iter()
        .filter(|phase| user_answer_count(session, phase) > 0)
        .map(|phase| evaluate_stored_phase(session, phase))
        .collect();
    if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f64>() / scores.len() as f64
    }
}

fn extract_sentences_with_keywords(text: &str, keywords: &[&str]) -> Vec<String> {
    text.split(['.', '\n'])
        .map(str::trim)
        .filter(|sentence| !sentence.is_empty())
        .filter(|sentence| {
            let lower = sentence.to_lowercase();
            keywords.iter().any(|keyword| lower.contains(keyword))
        })
        .take(5)
        .map(str::to_string)
        .collect()
}

fn persisted_line(session: &AiSession, turn: Option<SessionTurn>) -> PersistedSessionLine {
    PersistedSessionLine {
        session_id: session.session_id.clone(),
        project_path: session.project_path.clone(),
        phase: session.phase.clone(),
        turn_count: session.turn_count,
        max_turns: session.max_turns,
        started_at: session.started_at.clone(),
        status: session.status.clone(),
        turn,
    }
}

fn compact_excerpt(content: &str, max_chars: usize) -> String {
    let cleaned = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.chars().count() <= max_chars {
        cleaned
    } else {
        format!("{}...", cleaned.chars().take(max_chars).collect::<String>())
    }
}

fn complete_session(session: &mut AiSession) {
    session.phase = SessionPhase::Completed;
    session.status = SessionStatus::Completed;
}

fn limit_response() -> AiResponse {
    AiResponse {
        message: "Session turn limit reached. The refinement session is complete.".to_string(),
        phase: SessionPhase::Completed,
        phase_complete: true,
        questions_remaining: 0,
        spec_updated: false,
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn phase_label(phase: &SessionPhase) -> &'static str {
    match phase {
        SessionPhase::Phase1Experience => "Phase 1 Experience Lens",
        SessionPhase::Phase2Tetrad => "Phase 2 Elemental Tetrad",
        SessionPhase::Phase3CoreLoop => "Phase 3 Core Loop Stress Test",
        SessionPhase::Phase4Motivation => "Phase 4 Player Motivation",
        SessionPhase::Phase5Assessment => "Phase 5 Honest Assessment",
        SessionPhase::Completed => "Completed",
    }
}

fn keywords_for_phase(phase: &SessionPhase) -> &'static [&'static str] {
    match phase {
        SessionPhase::Phase1Experience => &[
            "player",
            "feel",
            "experience",
            "moment",
            "emotion",
            "fantasy",
        ],
        SessionPhase::Phase2Tetrad => &[
            "mechanic",
            "story",
            "aesthetic",
            "technology",
            "system",
            "world",
            "visual",
            "platform",
        ],
        SessionPhase::Phase3CoreLoop => {
            &["loop", "repeat", "action", "reward", "feedback", "choice"]
        }
        SessionPhase::Phase4Motivation => &[
            "motivation",
            "progress",
            "mastery",
            "reward",
            "goal",
            "continue",
        ],
        SessionPhase::Phase5Assessment => &["risk", "scope", "test", "viable", "strength", "weak"],
        SessionPhase::Completed => &[],
    }
}

fn questions_for_phase(phase: &SessionPhase) -> &'static [&'static str] {
    match phase {
        SessionPhase::Phase1Experience => &[
            "What exact experience should the player remember after the first session?",
            "Which emotion should peak during the strongest moment of play?",
            "What fantasy does the player get to inhabit that other games rarely provide?",
            "What should the player understand without reading instructions?",
            "Which moment would make a spectator want to try the game?",
            "What should feel surprising but still fair?",
            "What tension should exist between safety and risk?",
            "How should the player describe the game to a friend in one sentence?",
            "What should the player never feel during the core experience?",
            "Which player action best proves the intended experience is working?",
        ],
        SessionPhase::Phase2Tetrad => &[
            "Mechanics: What verbs does the player use most often?",
            "Mechanics: Which rule creates the most interesting choice?",
            "Mechanics: How does failure change the next decision?",
            "Story: What situation gives the mechanics meaning?",
            "Story: What changes in the world because the player acts?",
            "Story: What mystery or promise pulls the player forward?",
            "Aesthetics: What visual language communicates the game's mood fastest?",
            "Aesthetics: Which sound or animation confirms success?",
            "Aesthetics: What must be readable at a glance?",
            "Technology: What platform or engine constraint shapes the design?",
            "Technology: Which technical risk could invalidate the experience?",
            "Technology: What can be prototyped cheaply this week?",
        ],
        SessionPhase::Phase3CoreLoop => &[
            "What does the player do repeatedly from moment to moment?",
            "What information starts each loop iteration?",
            "What choice or skill expression sits at the center of the loop?",
            "What immediate feedback tells the player whether the action worked?",
            "What reward changes the next loop?",
            "How does the loop escalate without adding confusion?",
            "Where can the player recover from a poor decision?",
            "What would make the loop boring, and how do you prevent it?",
        ],
        SessionPhase::Phase4Motivation => &[
            "Why does the player want to continue after the first success?",
            "What long-term goal gives short-term actions meaning?",
            "What form of mastery can the player notice improving?",
            "Which reward is intrinsically fun rather than only collectible?",
            "How does curiosity survive after the rules are understood?",
            "What social, creative, or strategic identity can the player express?",
            "What makes stopping feel like a choice rather than fatigue?",
            "Which player type is most likely to love this, and why?",
        ],
        SessionPhase::Phase5Assessment => &[
            "What is the strongest reason this game should exist?",
            "What is the biggest risk to proving the experience?",
            "Which feature should be cut first if scope grows?",
            "What test would honestly show whether the core is fun?",
            "What evidence would make you pivot the concept?",
            "What is the smallest viable build that preserves the promise?",
        ],
        SessionPhase::Completed => &[],
    }
}
