pub mod addon_auth;
pub mod addon_routes;
pub mod addon_store;
pub mod ai_log;
pub mod auto_update;
pub mod bridge_types;
pub mod capture;
pub mod config;
pub mod cross_platform;
pub mod godot_bridge_install;
pub mod gopeak_manifest;
pub mod lux_agents_install;
pub mod lux_ai_session;
pub mod lux_ambiguity;
pub mod lux_api;
pub mod lux_bridge_lease;
pub mod lux_build;
pub mod lux_continuation_state;
pub mod lux_doctor;
pub mod lux_engines;
pub mod lux_event_log;
pub mod lux_events;
pub mod lux_hooks;
pub mod lux_io;
pub mod lux_lock;
pub mod lux_loop;
pub mod lux_mcp;
pub mod lux_metrics;
mod lux_next_goal;
mod lux_next_goal_evidence;
mod lux_next_goal_helpers;
mod lux_next_goal_types;
pub mod lux_roadmap;
pub mod lux_roadmap_issue_register;
mod lux_roadmap_issue_register_types;
mod lux_roadmap_registry;
pub mod lux_run;
pub mod lux_run_recover;
pub mod lux_run_state;
pub mod lux_spec;
pub mod lux_spec_loop;
pub mod lux_task_dag;
pub mod lux_team_profile;
pub mod lux_terminal;
pub mod lux_ticket;
pub mod lux_triage;
pub mod lux_verification;
pub mod lux_worktree;
pub mod project;
pub mod project_godot;
mod protocol;
mod server;
pub mod session;
pub mod skill_adapter;
mod uloop_runner;
pub mod uloop_sync;
pub mod unity_hub;
pub mod unity_launch;
pub mod visual_regression;

use std::{
    fs,
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{IpAddr, SocketAddr, ToSocketAddrs},
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
    sync::OnceLock,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context};
use clap::{ArgAction, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, shells::Shell};
use protocol::EventEnvelope;
use serde_json::{json, Value};

use crate::bridge_types::BridgeKind;
use skill_adapter::discovery::{
    discover_skills, read_skill_md_preview, read_skill_references, SkillEntry, SkillManifest,
};

static CONFIG_PATH_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

#[derive(Parser, Debug)]
#[command(name = "lux")]
#[command(version)]
#[command(about = "Lux CLI — Unity batch mode automation for Neon Glitch")]
pub struct Cli {
    /// Custom Lux config file path
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Skip automatic update check
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    pub no_update_check: bool,
    /// Internal: run as background update worker
    #[arg(long, global = true, hide = true, action = ArgAction::SetTrue)]
    lux_update_worker: bool,
    /// Internal: run as background uloop sync worker
    #[arg(long, global = true, hide = true, action = ArgAction::SetTrue)]
    uloop_sync_worker: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Initialize a .lux game harness workspace
    Init(LuxInitArgs),
    /// Inspect, edit, and validate Lux game specs
    Spec(LuxSpecArgs),
    SpecLoop(LuxSpecLoopArgs),
    /// Inspect the canonical .lux roadmap reality file
    Roadmap(LuxRoadmapArgs),
    /// Show Lux kanban board status
    Kanban(LuxProjectArgs),
    /// Run automated triage pipeline on recent events
    Triage(lux_triage::TriageArgs),
    /// Trigger a Lux game build
    Build(LuxBuildArgs),
    /// Open the latest Lux build in a browser
    Play(LuxProjectArgs),
    /// Run full Lux game harness verification
    Verify(LuxProjectArgs),
    /// Start or manage a spec-driven automated dev run
    Run(lux_run::RunArgs),
    /// Run the Lux stdio MCP server
    Mcp(McpArgs),
    Serve(ServeArgs),
    Unity(UnityArgs),
    /// Inspect and operate Godot project harness support
    Godot(GodotArgs),
    Skill(SkillArgs),
    AiLog(AiLogArgs),
    Compile(CompileArgs),
    /// Install and run AI coding tool hook bridges
    Hooks(lux_hooks::HooksArgs),
    Bridge(BridgeArgs),
    RunTests(RunTestsArgs),
    Screenshot(ScreenshotArgs),
    Session(SessionArgs),
    Install(InstallArgs),
    Addon(AddonArgs),
    Config(ConfigArgs),
    /// Show server and project status as JSON
    Status(StatusArgs),
    Schema,
    /// Generate shell completion scripts
    Completion {
        /// Shell type to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Update the lux binary to the latest version
    SelfUpdate(SelfUpdateArgs),
    /// Diagnose project health and auto-fix issues
    Doctor(lux_doctor::DoctorArgs),
    /// Install Lux workflow skills into .agents/skills/
    AgentsInstall(lux_agents_install::AgentsInstallArgs),
    /// Control and inspect the autonomous agent dispatch pipeline
    Autonomous(AutonomousArgs),
}

#[derive(Parser, Debug)]
struct AutonomousArgs {
    #[command(subcommand)]
    action: AutonomousCommand,
}

#[derive(Subcommand, Debug)]
enum AutonomousCommand {
    /// Show current autonomous run state
    Status(LuxProjectArgs),
    /// Preview dispatch eligibility without mutating state
    DryRun(AutonomousDryRunArgs),
    /// Trigger dispatch (requires DispatchReady state)
    Dispatch(AutonomousDispatchArgs),
    /// Show execution evidence for a run
    Evidence(AutonomousEvidenceArgs),
}

#[derive(Parser, Debug)]
struct AutonomousDryRunArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Filter dry-run to a specific ticket ID
    #[arg(long)]
    ticket: Option<String>,
}

#[derive(Parser, Debug)]
struct AutonomousDispatchArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Expected seq value for optimistic concurrency check
    #[arg(long)]
    seq: u64,
    /// Target a specific ticket ID for dispatch
    #[arg(long)]
    ticket: Option<String>,
}

#[derive(Parser, Debug)]
struct AutonomousEvidenceArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Run ID to show evidence for (defaults to current run)
    #[arg(long, visible_alias = "run")]
    run_id: Option<String>,
}

#[derive(Parser, Debug)]
struct LuxProjectArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct LuxInitArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Enable the interactive spec question flow
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    interactive: bool,
    /// Stop after a single non-interactive pass without asking questions
    #[arg(long = "no-interactive", action = ArgAction::SetTrue)]
    no_interactive: bool,
    /// Target ambiguity threshold before stopping interactive questioning
    #[arg(long, default_value_t = 0.02)]
    target_ambiguity: f64,
    /// Maximum interactive question rounds to run
    #[arg(long, default_value_t = 10)]
    max_iterations: u32,
    /// Back up existing .lux workspace and initialize from scratch
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    force: bool,
    /// Team profile preset or path for team-mode integration
    #[arg(long = "team-profile")]
    team_profile: Option<String>,
    /// Install Lux workflow skills into .agents/skills/ (default: ask)
    #[arg(long = "agents-skills", action = ArgAction::SetTrue)]
    agents_skills: bool,
    /// Do not install Lux workflow skills into .agents/skills/
    #[arg(long = "no-agents-skills", action = ArgAction::SetTrue)]
    no_agents_skills: bool,
}

#[derive(Parser, Debug)]
struct GodotArgs {
    #[command(subcommand)]
    command: GodotCommand,
}

#[derive(Subcommand, Debug)]
enum GodotCommand {
    /// Show Godot project detection and Lux capability status
    Status(GodotStatusArgs),
    /// Build through Godot support once GoPeak build verification is available
    Build(GodotBuildArgs),
}

#[derive(Parser, Debug)]
struct GodotStatusArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct GodotBuildArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

fn parse_bridge_kind(value: &str) -> anyhow::Result<BridgeKind> {
    value.parse()
}

#[derive(Parser, Debug)]
struct LuxSpecArgs {
    #[command(subcommand)]
    action: Option<LuxSpecAction>,
    /// Unity project root containing the .lux directory
    #[arg(long, global = true)]
    project_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum LuxSpecAction {
    /// Open a domain markdown spec in $EDITOR, or print its path when no editor is set
    Edit(LuxSpecEditArgs),
    Validate,
}

#[derive(Parser, Debug)]
struct LuxSpecEditArgs {
    /// Domain name, such as design, architecture, art-style, audio, narrative, levels, or ui-ux
    domain: String,
}

#[derive(Parser, Debug)]
struct LuxSpecLoopArgs {
    #[command(subcommand)]
    action: LuxSpecLoopAction,
}

#[derive(Subcommand, Debug)]
enum LuxSpecLoopAction {
    Start(LuxSpecLoopStartArgs),
    Status(LuxSpecLoopRunArgs),
    Answer(LuxSpecLoopAnswerArgs),
    Approve(LuxSpecLoopProposalArgs),
    Reject(LuxSpecLoopProposalArgs),
    Apply(LuxSpecLoopRunArgs),
}

#[derive(Parser, Debug)]
struct LuxSpecLoopStartArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long)]
    max_iterations: Option<u32>,
}

#[derive(Parser, Debug)]
struct LuxSpecLoopRunArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    run_id: String,
}

#[derive(Parser, Debug)]
struct LuxSpecLoopAnswerArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    run_id: String,
    question_id: String,
    answer: String,
}

#[derive(Parser, Debug)]
struct LuxSpecLoopProposalArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    run_id: String,
    proposal_id: String,
}

#[derive(Parser, Debug)]
struct LuxRoadmapArgs {
    #[command(subcommand)]
    action: Option<LuxRoadmapAction>,
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum LuxRoadmapAction {
    /// Initialize .lux/roadmap.json from an engine-specific template
    Init(LuxRoadmapInitArgs),
    /// Validate and print .lux/roadmap.json status
    Status,
    /// Register missing roadmap and capability gaps as GitHub issues
    IssueRegister(LuxRoadmapIssueRegisterArgs),
}

#[derive(Parser, Debug)]
struct LuxRoadmapInitArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Engine template to write
    #[arg(long, value_enum, default_value_t = LuxRoadmapEngine::Unity)]
    engine: LuxRoadmapEngine,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LuxRoadmapEngine {
    Unity,
    Godot,
    ThreeJs,
}

impl From<LuxRoadmapEngine> for lux_project::EngineKind {
    fn from(value: LuxRoadmapEngine) -> Self {
        match value {
            LuxRoadmapEngine::Unity => Self::Unity,
            LuxRoadmapEngine::Godot => Self::Godot,
            LuxRoadmapEngine::ThreeJs => Self::ThreeJs,
        }
    }
}

#[derive(Parser, Debug)]
struct LuxRoadmapIssueRegisterArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// GitHub repository in owner/name form
    #[arg(long)]
    repo: String,
    /// Print the issue plan without creating GitHub issues
    #[arg(long, action = ArgAction::SetTrue)]
    dry_run: bool,
    /// Print JSON output
    #[arg(long, action = ArgAction::SetTrue)]
    json: bool,
    /// Existing GitHub issues JSON snapshot, matching gh issue list JSON
    #[arg(long)]
    existing_issues_json: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct LuxBuildArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Build target to queue
    #[arg(long, value_enum)]
    target: LuxBuildTarget,
}

#[derive(Clone, Debug, ValueEnum)]
enum LuxBuildTarget {
    WebGl,
}

impl From<LuxBuildTarget> for lux_build::BuildTarget {
    fn from(value: LuxBuildTarget) -> Self {
        match value {
            LuxBuildTarget::WebGl => Self::WebGL,
        }
    }
}

#[derive(Parser, Debug)]
struct ConfigArgs {
    #[command(subcommand)]
    action: ConfigAction,
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    /// Display current effective config
    Show,
    /// Set a config value
    Set { key: String, value: String },
    /// Get a config value
    Get { key: String },
    /// Show config file path
    Path,
    /// Open config file in the default editor
    Edit,
}

#[derive(Parser, Debug)]
struct SkillArgs {
    #[command(subcommand)]
    action: SkillAction,
}

#[derive(Subcommand, Debug)]
enum SkillAction {
    List(SkillListArgs),
    Info(SkillInfoArgs),
    Install(SkillInstallArgs),
    Remove(SkillRemoveArgs),
    Update(SkillUpdateArgs),
}
#[derive(Parser, Debug)]
struct InstallArgs {
    name: String,
    #[arg(short, long)]
    project: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Parser, Debug)]
struct AddonArgs {
    #[command(subcommand)]
    action: AddonAction,
}

#[derive(Subcommand, Debug)]
enum AddonAction {
    List(AddonListArgs),
    Auth(AddonAuthArgs),
}

#[derive(Parser, Debug)]
struct AddonListArgs {
    #[arg(long, default_value_t = false)]
    public: bool,
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Parser, Debug)]
struct AddonAuthArgs {
    #[arg(long, default_value_t = false)]
    status: bool,
}

#[derive(Parser, Debug)]
struct SkillListArgs {
    /// Filter by skill scope
    #[arg(long, value_enum)]
    scope: Option<SkillScope>,
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Parser, Debug)]
struct SkillInfoArgs {
    name: String,
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Parser, Debug)]
struct SkillInstallArgs {
    /// Skill name (e.g. my-skill)
    name: String,
    /// Source URL or path to install from
    #[arg(short, long)]
    source: String,
    /// Install to project scope (.agents/skills/) instead of global
    #[arg(short, long)]
    project: bool,
    /// Install destination scope
    #[arg(long, value_enum)]
    scope: Option<WritableSkillScope>,
    /// Write project adaptation metadata after compatibility checks
    #[arg(long, default_value_t = false)]
    adapt: bool,
    /// Print machine-readable JSON output
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Parser, Debug)]
struct SkillRemoveArgs {
    /// Skill name to remove
    name: String,
    /// Remove from project scope
    #[arg(short, long)]
    project: bool,
    /// Remove from global scope
    #[arg(short, long)]
    global: bool,
    /// Remove from the selected scope
    #[arg(long, value_enum)]
    scope: Option<WritableSkillScope>,
}

#[derive(Parser, Debug)]
struct SkillUpdateArgs {
    /// Skill name to update
    name: String,
    /// Update the selected scope
    #[arg(long, value_enum)]
    scope: Option<WritableSkillScope>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum SkillScope {
    Core,
    Project,
    Global,
    Plugin,
}

impl SkillScope {
    fn as_str(self) -> &'static str {
        match self {
            SkillScope::Core => "core",
            SkillScope::Project => "project",
            SkillScope::Global => "global",
            SkillScope::Plugin => "plugin",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum WritableSkillScope {
    Project,
    Global,
}

impl WritableSkillScope {
    fn as_str(self) -> &'static str {
        match self {
            WritableSkillScope::Project => "project",
            WritableSkillScope::Global => "global",
        }
    }
}

#[derive(Parser, Debug)]
struct AiLogArgs {
    #[command(subcommand)]
    action: AiLogAction,
}

#[derive(Parser, Debug)]
struct SessionArgs {
    #[command(subcommand)]
    action: SessionAction,
}

#[derive(Subcommand, Debug)]
enum SessionAction {
    Record(SessionRecordArgs),
    Stop(SessionStopArgs),
    Replay(SessionReplayArgs),
    Timeline(SessionTimelineArgs),
    Report(SessionReportArgs),
}

#[derive(Parser, Debug)]
struct SessionRecordArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct SessionStopArgs {
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct SessionReplayArgs {
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long, default_value_t = 1.0)]
    speed: f64,
    #[arg(long)]
    filter_type: Option<String>,
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct SessionTimelineArgs {
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long)]
    filter_type: Option<String>,
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Parser, Debug)]
struct SessionReportArgs {
    #[arg(long)]
    session_id: Option<String>,
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum AiLogAction {
    Recent(AiLogRecentArgs),
    Tail(AiLogTailArgs),
    Context(AiLogContextArgs),
    Compact(AiLogCompactArgs),
    WorkStep(AiLogWorkStepArgs),
}

#[derive(Parser, Debug)]
struct AiLogRecentArgs {
    #[arg(long)]
    project_path: PathBuf,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, default_value_t = false)]
    json: bool,
    #[arg(long)]
    actor: Option<String>,
    #[arg(long)]
    category: Option<String>,
    #[arg(long)]
    source: Option<String>,
    #[arg(long)]
    action: Option<String>,
    #[arg(long)]
    event_type: Option<String>,
}

#[derive(Parser, Debug)]
struct AiLogTailArgs {
    #[arg(long)]
    project_path: PathBuf,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, default_value_t = false)]
    json: bool,
    /// Print a bounded snapshot and exit; continuous follow is intentionally non-blocking.
    #[arg(long, default_value_t = false)]
    follow: bool,
}

#[derive(Parser, Debug)]
struct AiLogContextArgs {
    #[arg(long)]
    project_path: PathBuf,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Parser, Debug)]
struct AiLogCompactArgs {
    #[arg(long)]
    project_path: PathBuf,
    #[arg(long, default_value_t = 5000)]
    max_lines: usize,
    #[arg(long, default_value_t = false)]
    json: bool,
    #[arg(long, default_value_t = false)]
    yes: bool,
}

#[derive(Parser, Debug)]
struct AiLogWorkStepArgs {
    #[arg(long = "name")]
    name: String,
    #[arg(long)]
    status: String,
    #[arg(long)]
    tool: Option<String>,
    #[arg(long)]
    action: Option<String>,
    #[arg(long)]
    summary: Option<String>,
    #[arg(long)]
    project_path: PathBuf,
}

#[derive(Parser, Debug)]
struct UnityArgs {
    #[command(subcommand)]
    command: UnityCommand,
}

#[derive(Subcommand, Debug)]
enum UnityCommand {
    Status(UnityStatusArgs),
    Context(UnityContextArgs),
    BackendStatus(UnityBackendStatusArgs),
    BackendListCommands(UnityBackendListCommandsArgs),
    GetLogs(UnityGetLogsArgs),
    ClearConsole(UnityClearConsoleArgs),
    FocusWindow(UnityFocusWindowArgs),
    Launch(UnityLaunchArgs),
    SceneSmoke(UnitySceneSmokeArgs),
    CreateObjects(UnityCreateObjectsArgs),
    FindGameObjects(UnityFindGameObjectsArgs),
    GetHierarchy(UnityGetHierarchyArgs),
    ControlPlayMode(UnityControlPlayModeArgs),
    Screenshot(UnityScreenshotArgs),
    SimulateMouseUi(UnitySimulateMouseUiArgs),
    SimulateKeyboard(UnitySimulateKeyboardArgs),
    SimulateMouseInput(UnitySimulateMouseInputArgs),
    RecordInput(UnityRecordInputArgs),
    ReplayInput(UnityReplayInputArgs),
    ExecuteDynamicCode(UnityExecuteDynamicCodeArgs),
    // === MIGRATED from top-level Command (v2.1 deprecation) ===
    Build(UnityBuildArgs),
    Play(UnityPlayArgs),
    Compile(UnityCompileArgs),
    Bridge(UnityBridgeArgs),
    RunTests(UnityRunTestsArgs),
    VisualRegression(UnityVisualRegressionArgs),
    /// Install or update uloop (unity-cli-loop) for Unity CLI passthrough
    InstallUloop(UnityInstallUloopArgs),
}

#[derive(Parser, Debug)]
struct UnityStatusArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct UnityContextArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    refresh: bool,
}

#[derive(Parser, Debug)]
struct UnityBackendStatusArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct UnityBackendListCommandsArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct UnityGetLogsArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct UnityClearConsoleArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct UnityFocusWindowArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct UnityLaunchArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long)]
    unity_path: Option<PathBuf>,
    #[arg(long, default_value_t = 120)]
    timeout_seconds: u64,
    #[arg(long, default_value_t = false)]
    no_wait: bool,
}

#[derive(Parser, Debug)]
struct UnitySceneSmokeArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value = "Assets/_Main/Scenes/GamePlay.unity")]
    scene_path: String,
    #[arg(long, default_value_t = 10)]
    object_count: u32,
    #[arg(long, default_value_t = false)]
    batch: bool,
}

#[derive(Parser, Debug)]
struct UnityCreateObjectsArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value = "Assets/_Main/Scenes/GamePlay.unity")]
    scene_path: String,
    #[arg(long, default_value_t = 10)]
    object_count: u32,
}

#[derive(Parser, Debug)]
struct UnityFindGameObjectsArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value = "query")]
    search_mode: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    regex: Option<String>,
    #[arg(long)]
    path: Option<String>,
    #[arg(long)]
    component: Option<String>,
    #[arg(long)]
    tag: Option<String>,
    #[arg(long)]
    layer: Option<String>,
    #[arg(long, default_value = "any")]
    active_state: String,
    #[arg(long, default_value_t = 50)]
    inline_limit: i64,
}

#[derive(Parser, Debug)]
struct UnityGetHierarchyArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    all: bool,
    #[arg(long)]
    root_path: Option<String>,
    #[arg(long, default_value_t = false)]
    use_selection: bool,
}

#[derive(Parser, Debug)]
struct UnityControlPlayModeArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, value_enum)]
    action: PlayModeAction,
    #[arg(long, default_value_t = false)]
    wait: bool,
}

#[derive(Parser, Debug)]
struct UnityScreenshotArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value = "rendering")]
    capture_mode: String,
    #[arg(long, default_value_t = false)]
    annotate_elements: bool,
    #[arg(long, default_value_t = false)]
    elements_only: bool,
}

#[derive(Parser, Debug)]
struct UnitySimulateKeyboardArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, value_enum)]
    action: KeyboardInputAction,
    #[arg(long)]
    key: String,
    #[arg(long, default_value_t = 50)]
    duration_ms: i64,
}

#[derive(Parser, Debug)]
struct UnitySimulateMouseUiArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, value_enum)]
    action: MouseUiAction,
    #[arg(long)]
    x: f64,
    #[arg(long)]
    y: f64,
    #[arg(long, default_value_t = 500)]
    duration_ms: i64,
}

#[derive(Parser, Debug)]
struct UnitySimulateMouseInputArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, value_enum)]
    action: MouseInputAction,
    #[arg(long, default_value = "left")]
    button: String,
    #[arg(long, default_value_t = 0.0)]
    delta_x: f64,
    #[arg(long, default_value_t = 0.0)]
    delta_y: f64,
    #[arg(long, default_value_t = 0.0)]
    scroll_x: f64,
    #[arg(long, default_value_t = 0.0)]
    scroll_y: f64,
    #[arg(long, default_value_t = 50)]
    duration_ms: i64,
    #[arg(long, default_value_t = 5)]
    steps: i64,
}

#[derive(Parser, Debug)]
struct UnityRecordInputArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, value_enum)]
    action: RecordInputAction,
}

#[derive(Parser, Debug)]
struct UnityReplayInputArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, value_enum)]
    action: ReplayInputAction,
    #[arg(long)]
    file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct UnityExecuteDynamicCodeArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long)]
    code: Option<String>,
    #[arg(long)]
    file: Option<PathBuf>,
}

// --- Migrated from LuxBuildArgs (was top-level Command::Build) ---
#[derive(Parser, Debug)]
struct UnityBuildArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Build target to queue
    #[arg(long, value_enum)]
    target: LuxBuildTarget,
}

// --- Migrated from Play/LuxProjectArgs (was top-level Command::Play) ---
#[derive(Parser, Debug)]
struct UnityPlayArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
}

// --- Migrated from CompileArgs (was top-level Command::Compile) ---
#[derive(Parser, Debug)]
struct UnityCompileArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
}

// --- Migrated from BridgeArgs (was top-level Command::Bridge) ---
#[derive(Parser, Debug)]
struct UnityBridgeArgs {
    #[command(subcommand)]
    action: UnityBridgeAction,
}

#[derive(Subcommand, Debug)]
enum UnityBridgeAction {
    Watch(UnityBridgeWatchArgs),
    Install(UnityBridgeInstallArgs),
}

#[derive(Parser, Debug)]
struct UnityBridgeWatchArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct UnityBridgeInstallArgs {
    /// Unity project root directory
    #[arg(long, short = 'p')]
    project_path: PathBuf,
    /// Install Lux OpenCode commands into .opencode/commands/ (default: ask)
    #[arg(long = "opencode-commands", action = ArgAction::SetTrue)]
    opencode_commands: bool,
    /// Do not install Lux OpenCode commands into .opencode/commands/
    #[arg(long = "no-opencode-commands", action = ArgAction::SetTrue)]
    no_opencode_commands: bool,
}

#[derive(Parser, Debug)]
struct UnityInstallUloopArgs {
    /// Unity project root (used to determine npm install scope)
    #[arg(long, short = 'p')]
    project_path: PathBuf,
    /// Force reinstall even if already installed
    #[arg(long)]
    force: bool,
    /// Install locally (into project) instead of globally
    #[arg(long)]
    local: bool,
    /// Specific version to install
    #[arg(long)]
    version: Option<String>,
}

// --- Migrated from RunTestsArgs (was top-level Command::RunTests) ---
/// Extended to support uloop-compatible filter options
#[derive(Parser, Debug)]
struct UnityRunTestsArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Test mode platform
    #[arg(long, default_value = "EditMode")]
    test_platform: String,
    /// Path to write test results
    #[arg(long)]
    test_results: Option<PathBuf>,
    /// Path to write test log output
    #[arg(long)]
    log_file: Option<PathBuf>,
    /// Filter type for test selection (all|exact|regex|assembly)
    #[arg(long, default_value = "all")]
    filter_type: String,
    /// Filter value for test selection
    #[arg(long)]
    filter_value: Option<String>,
    /// Save scene before running tests
    #[arg(long, default_value_t = false)]
    save_before_run: bool,
}

// --- Migrated from ScreenshotArgs (was top-level Command::Screenshot) ---
/// Visual regression screenshot (baseline comparison), distinct from editor capture
#[derive(Parser, Debug)]
struct UnityVisualRegressionArgs {
    /// Unity project root containing the .lux directory
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Capture a named visual regression baseline
    #[arg(long, conflicts_with = "compare")]
    baseline: Option<String>,
    /// Compare this baseline path against the current screenshot
    #[arg(long, conflicts_with = "baseline")]
    compare: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PlayModeAction {
    Play,
    Stop,
    Pause,
    Resume,
    Status,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum KeyboardInputAction {
    Press,
    KeyDown,
    KeyUp,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum MouseUiAction {
    Click,
    LongPress,
    Drag,
    DragStart,
    DragMove,
    DragEnd,
}

impl MouseUiAction {
    fn as_str(self) -> &'static str {
        match self {
            MouseUiAction::Click => "click",
            MouseUiAction::LongPress => "long-press",
            MouseUiAction::Drag => "drag",
            MouseUiAction::DragStart => "drag-start",
            MouseUiAction::DragMove => "drag-move",
            MouseUiAction::DragEnd => "drag-end",
        }
    }
}

impl KeyboardInputAction {
    fn as_str(self) -> &'static str {
        match self {
            KeyboardInputAction::Press => "press",
            KeyboardInputAction::KeyDown => "key-down",
            KeyboardInputAction::KeyUp => "key-up",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum MouseInputAction {
    Click,
    LongPress,
    MoveDelta,
    SmoothDelta,
    Scroll,
}

impl MouseInputAction {
    fn as_str(self) -> &'static str {
        match self {
            MouseInputAction::Click => "click",
            MouseInputAction::LongPress => "long-press",
            MouseInputAction::MoveDelta => "move-delta",
            MouseInputAction::SmoothDelta => "smooth-delta",
            MouseInputAction::Scroll => "scroll",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum RecordInputAction {
    Start,
    Stop,
}

impl RecordInputAction {
    fn as_str(self) -> &'static str {
        match self {
            RecordInputAction::Start => "start",
            RecordInputAction::Stop => "stop",
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ReplayInputAction {
    Start,
    Stop,
    Status,
}

impl ReplayInputAction {
    fn as_str(self) -> &'static str {
        match self {
            ReplayInputAction::Start => "start",
            ReplayInputAction::Stop => "stop",
            ReplayInputAction::Status => "status",
        }
    }
}

impl PlayModeAction {
    fn as_str(self) -> &'static str {
        match self {
            PlayModeAction::Play => "play",
            PlayModeAction::Stop => "stop",
            PlayModeAction::Pause => "pause",
            PlayModeAction::Resume => "resume",
            PlayModeAction::Status => "status",
        }
    }
}

#[derive(Parser, Debug)]
struct ServeArgs {
    #[arg(long, env = "LUX_GATEWAY_HOST")]
    host: Option<IpAddr>,
    #[arg(long, env = "LUX_GATEWAY_PORT")]
    port: Option<u16>,
    #[arg(long, env = "LUX_GATEWAY_TOKEN")]
    token: Option<String>,
    #[arg(long, env = "LUX_GATEWAY_HISTORY", default_value_t = 256)]
    history_capacity: usize,
    /// Minutes without HTTP or WebSocket activity before graceful shutdown (0 disables)
    #[arg(long, env = "LUX_GATEWAY_IDLE_TIMEOUT")]
    idle_timeout: Option<u64>,
    /// Unity project root used for project-bound gateway APIs
    #[arg(long, env = "LUX_PROJECT_PATH")]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct McpArgs {
    #[command(subcommand)]
    action: Option<McpAction>,
    /// Default Unity project root for project-bound MCP tool calls
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum McpAction {
    /// Install a project-scoped MCP config entry for Lux
    Install(McpInstallArgs),
}

#[derive(Parser, Debug)]
struct McpInstallArgs {
    /// Unity project root that should receive .mcp.json
    #[arg(long)]
    project_path: Option<PathBuf>,
    /// Print machine-readable installation details
    #[arg(long)]
    json: bool,
}

#[derive(Parser, Debug)]
struct CompileArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct BridgeArgs {
    #[command(subcommand)]
    action: BridgeAction,
}

#[derive(Subcommand, Debug)]
enum BridgeAction {
    Watch(BridgeWatchArgs),
    Install(BridgeInstallArgs),
}

#[derive(Parser, Debug)]
struct BridgeWatchArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct BridgeInstallArgs {
    /// Unity project root directory
    #[arg(long, short = 'p')]
    project_path: PathBuf,
    /// Bridge type to install
    #[arg(long = "type", default_value = "unity", value_parser = parse_bridge_kind)]
    bridge_type: BridgeKind,
    /// Install Lux OpenCode commands into .opencode/commands/ (default: ask)
    #[arg(long = "opencode-commands", action = ArgAction::SetTrue)]
    opencode_commands: bool,
    /// Do not install Lux OpenCode commands into .opencode/commands/
    #[arg(long = "no-opencode-commands", action = ArgAction::SetTrue)]
    no_opencode_commands: bool,
}

#[derive(Parser, Debug)]
struct RunTestsArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value = "EditMode")]
    test_platform: String,
    #[arg(long)]
    test_results: Option<PathBuf>,
    #[arg(long)]
    log_file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct StatusArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct SelfUpdateArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    force: bool,
}

#[derive(Parser, Debug)]
struct ScreenshotArgs {
    /// Capture a named visual regression baseline
    #[arg(long, conflicts_with = "compare")]
    baseline: Option<String>,
    /// Compare this baseline path against the current screenshot
    #[arg(long, conflicts_with = "baseline")]
    compare: Option<PathBuf>,
    #[arg(long)]
    project_path: Option<PathBuf>,
}

#[derive(Debug, serde::Deserialize)]
struct LuxBridgeSettings {
    schema_version: u32,
    protocol: String,
    package_name: String,
    package_version: String,
    project_root: String,
    rust_gateway_path: String,
    #[serde(default)]
    gateway_url: Option<String>,
    #[serde(default)]
    unity_server_port: Option<u16>,
    generated_at_utc: String,
}

#[derive(Debug, serde::Deserialize)]
struct UnityBridgeDiscovery {
    host: String,
    port: u16,
    token: String,
}

#[derive(Debug)]
pub struct UnityBridgeBackendPing {
    pub host: String,
    pub port: u16,
    pub discovery_path: PathBuf,
    pub ping: Value,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    if cli.lux_update_worker {
        auto_update::run_update_worker().await?;
        return Ok(());
    }
    if cli.uloop_sync_worker {
        uloop_sync::run_uloop_sync_worker().await?;
        return Ok(());
    }
    auto_update::maybe_spawn_update_check(cli.no_update_check);
    uloop_sync::maybe_spawn_uloop_sync_worker();

    if let Some(path) = cli.config.clone() {
        let _ = CONFIG_PATH_OVERRIDE.set(path);
    }
    let config = load_active_config()?;
    let config = config::merge_with_cli(&config, &cli);

    execute_cli_command(cli, &config).await
}

async fn execute_cli_command(cli: Cli, config: &config::LuxConfig) -> anyhow::Result<()> {
    match cli.command {
        Command::Init(args) => run_lux_init_command(args),
        Command::Spec(args) => run_lux_spec_command(args),
        Command::SpecLoop(args) => run_lux_spec_loop_command(args),
        Command::Roadmap(args) => run_lux_roadmap_command(args),
        Command::Kanban(args) => run_lux_kanban_command(args),
        Command::Triage(args) => lux_triage::run_triage_command(&args),
        Command::Build(args) => {
            eprintln!("DEPRECATED: 'lux build' is deprecated. Use 'lux unity build' instead.");
            run_lux_build_command(args)
        }
        Command::Play(args) => {
            eprintln!("DEPRECATED: 'lux play' is deprecated. Use 'lux unity play' instead.");
            run_lux_play_command(args)
        }
        Command::Verify(args) => run_lux_verify_command(args),
        Command::Run(args) => lux_run::run_command(&args),
        Command::Mcp(args) => run_lux_mcp_command(args),
        Command::Serve(args) => serve(args, &config).await,
        Command::Unity(args) => run_lux_unity_command(args),
        Command::Godot(args) => run_lux_godot_command(args),
        Command::Skill(args) => run_skill_command(args),
        Command::AiLog(args) => run_ai_log_command(args),
        Command::Compile(args) => {
            eprintln!("DEPRECATED: 'lux compile' is deprecated. Use 'lux unity compile' instead.");
            run_batch_compile(args)
        }
        Command::Hooks(args) => lux_hooks::run_hooks_command(args),
        Command::Bridge(args) => {
            eprintln!("DEPRECATED: 'lux bridge' is deprecated. Use 'lux unity bridge' instead.");
            run_bridge_command(args)
        }
        Command::RunTests(args) => {
            eprintln!(
                "DEPRECATED: 'lux run-tests' is deprecated. Use 'lux unity run-tests' instead."
            );
            run_batch_tests(args)
        }
        Command::Screenshot(args) => {
            eprintln!("DEPRECATED: 'lux screenshot' (visual regression) is deprecated. Use 'lux unity visual-regression' instead.");
            run_screenshot_command(args)
        }
        Command::Session(args) => run_session_command(args),
        Command::Install(args) => run_install_command(args),
        Command::Addon(args) => run_addon_command(args),
        Command::Config(args) => run_config_command(args, &config),
        Command::Status(args) => run_status_command(args, config),
        Command::Schema => {
            println!(
                "{}",
                serde_json::to_string_pretty(&EventEnvelope::schema_example())?
            );
            Ok(())
        }
        Command::Completion { shell } => {
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }
        Command::SelfUpdate(args) => run_self_update_command(args),
        Command::Doctor(args) => lux_doctor::run_doctor_command(args),
        Command::AgentsInstall(args) => lux_agents_install::run_agents_install_command(args),
        Command::Autonomous(args) => run_autonomous_command(args),
    }
}

fn run_lux_mcp_command(args: McpArgs) -> anyhow::Result<()> {
    match args.action {
        Some(McpAction::Install(install_args)) => {
            let project_path = install_args
                .project_path
                .or(args.project_path)
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)
                .context("resolve project path for lux mcp install")?;
            let lux_exe = std::env::current_exe().context("resolve current lux executable")?;
            let result = lux_mcp::install_project_mcp_config(&project_path, &lux_exe)?;

            if install_args.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!(
                    "{}",
                    result["message"]
                        .as_str()
                        .unwrap_or("Lux MCP project config installed")
                );
                println!(
                    "Config: {}",
                    result["configPath"]
                        .as_str()
                        .unwrap_or("<unknown .mcp.json path>")
                );
            }
            Ok(())
        }
        None => lux_mcp::run_mcp_stdio(args.project_path.as_deref()),
    }
}

fn run_lux_init_command(args: LuxInitArgs) -> anyhow::Result<()> {
    let project_root = resolve_lux_project_root(&args.project_path)?;
    let lux_path = project_root.join(".lux");
    let reinit = lux_path.exists();
    if reinit && !args.force {
        eprintln!(
            "⚠️  Lux workspace already exists at {}. Re-evaluating...",
            lux_path.display()
        );
    }

    let options = lux_spec::LuxInitInteractiveOptions {
        interactive: if args.no_interactive {
            false
        } else {
            args.interactive
        },
        target_ambiguity: args.target_ambiguity,
        max_iterations: args.max_iterations,
    };

    let mut io = lux_spec::TerminalSpecQuestionIo;
    if reinit && args.force {
        lux_spec::lux_reinit(&project_root)?;
    }
    let lux_path = lux_spec::lux_init_interactive(&project_root, &mut io, options)?;
    eprintln!("Initialized Lux at {}", lux_path.display());
    let _capabilities = lux_engines::write_engine_capability_snapshot(
        &project_root,
        lux_project::EngineKind::Unity,
    )
    .context("failed to write engine capability snapshot")?;

    let agents_skills_dir = project_root.join(".agents").join("skills");
    let has_existing_lux_skills = agents_skills_dir.is_dir()
        && std::fs::read_dir(&agents_skills_dir).is_ok_and(|it| {
            it.filter_map(|e| e.ok())
                .any(|e| e.file_name().to_string_lossy().starts_with("lux-") && e.path().is_dir())
        });

    let mut install_force = args.force;

    let should_install_skills = if args.agents_skills {
        install_force = true;
        true
    } else if args.no_agents_skills {
        false
    } else if args.no_interactive {
        false
    } else if has_existing_lux_skills && !args.force {
        // Existing lux-* skills: ask before overwriting (preserve prior UX)
        let yes = prompt_skill_overwrite(&project_root)?;
        if yes {
            install_force = true;
        }
        yes
    } else {
        // Fresh project or no existing lux skills: ask whether to place workflow skills for AI agents
        let yes = prompt_install_agents_skills(&project_root, has_existing_lux_skills)?;
        if yes {
            install_force = true;
        }
        yes
    };

    if should_install_skills {
        let install_args = lux_agents_install::AgentsInstallArgs {
            project_path: Some(project_root.clone()),
            force: install_force,
            list_only: false,
            skill_names: None,
        };
        match lux_agents_install::run_agents_install_command(install_args) {
            Ok(()) => eprintln!(
                "✅ Lux workflow skills installed to .agents/skills/ ({})",
                lux_agents_install::list_bundled_skills().len()
            ),
            Err(err) => eprintln!("⚠️  Could not install Lux workflow skills: {err:#}"),
        }
    } else if has_existing_lux_skills && !args.agents_skills {
        eprintln!(
            "ℹ️  Skipped skill installation (existing lux-* skills preserved). Run 'lux agents-install --force' or 'lux init --agents-skills' to (re)install."
        );
    }

    Ok(())
}

fn prompt_skill_overwrite(project_root: &Path) -> anyhow::Result<bool> {
    use std::io;
    eprintln!();
    eprintln!(
        "⚠️  Existing Lux workflow skills found in {}/.agents/skills/:",
        project_root.display()
    );
    eprintln!("   Installing will OVERWRITE existing lux-* skill files.");
    eprintln!();
    eprint!("   Overwrite existing lux-* skills? [y/N]: ");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "yes" | "Y" | "YES"))
}

fn prompt_install_agents_skills(project_root: &Path, has_existing: bool) -> anyhow::Result<bool> {
    use std::io;
    eprintln!();
    if has_existing {
        eprintln!(
            "Lux workflow skills are present in {}/.agents/skills/.",
            project_root.display()
        );
    } else {
        eprintln!(
            "Lux workflow skills can be installed to {}/.agents/skills/ for AI agents (Codex, OpenCode, Claude Code, etc.).",
            project_root.display()
        );
    }
    eprintln!("These skills provide project-aware prompts and harness integration.");
    eprintln!();
    eprint!("   Install Lux workflow skills to .agents/skills/? [y/N]: ");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "yes" | "Y" | "YES"))
}

fn run_lux_spec_command(args: LuxSpecArgs) -> anyhow::Result<()> {
    let project_root = resolve_lux_project_root(&args.project_path)?;
    match args.action {
        None => print_lux_spec_status(&project_root),
        Some(LuxSpecAction::Edit(edit_args)) => {
            edit_lux_spec_domain(&project_root, &edit_args.domain)
        }
        Some(LuxSpecAction::Validate) => validate_lux_spec(&project_root),
    }
}

fn print_lux_spec_status(project_root: &Path) -> anyhow::Result<()> {
    let spec = lux_spec::lux_load(project_root)?;
    let ambiguity = lux_ambiguity::calculate_ambiguity(&spec);
    println!("Lux spec: {} ({:?})", spec.project_name, spec.status);
    println!("Project: {}", project_root.display());
    println!("Overall ambiguity: {:.2}", ambiguity.overall_score);
    println!();
    println!(
        "{:<16} {:<8} {:<10} {:<10} MISSING",
        "DOMAIN", "DEFINED", "AMBIG", "COMPLETE"
    );
    for (name, domain) in lux_spec_domain_rows(&spec) {
        let score = ambiguity
            .domain_scores
            .get(name)
            .map(|value| value.composite_score)
            .unwrap_or(domain.map(|value| value.ambiguity_score).unwrap_or(1.0));
        let completion = ambiguity
            .domain_scores
            .get(name)
            .map(|value| value.completion_ratio)
            .unwrap_or(0.0);
        let missing = ambiguity
            .domain_scores
            .get(name)
            .map(|value| value.missing_fields.join(","))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{:<16} {:<8} {:<10.2} {:<10.0}% {}",
            name,
            domain.is_some_and(|value| value.defined),
            score,
            completion * 100.0,
            missing
        );
    }
    if !ambiguity.recommendations.is_empty() {
        println!();
        println!("Recommendations:");
        for recommendation in ambiguity.recommendations {
            println!("- {recommendation}");
        }
    }
    Ok(())
}

fn edit_lux_spec_domain(project_root: &Path, domain: &str) -> anyhow::Result<()> {
    let normalized = domain.replace('_', "-");
    let path = project_root
        .join(".lux")
        .join("domains")
        .join(format!("{normalized}.md"));
    if !path.exists() {
        bail!("Lux domain spec does not exist: {}", path.display());
    }

    match std::env::var_os("EDITOR") {
        Some(editor) if !editor.is_empty() => {
            let status = ProcessCommand::new(&editor)
                .arg(&path)
                .status()
                .with_context(|| format!("failed to launch editor for {}", path.display()))?;
            if !status.success() {
                bail!("editor exited with status {status}");
            }
        }
        _ => println!("{}", path.display()),
    }
    Ok(())
}

fn validate_lux_spec(project_root: &Path) -> anyhow::Result<()> {
    let spec = lux_spec::lux_load(project_root)?;
    match spec.validate() {
        Ok(()) => {
            println!(
                "Lux spec is valid: {}",
                project_root.join(".lux/specs/spec.json").display()
            );
            Ok(())
        }
        Err(error) => bail!("Lux spec validation failed: {error}"),
    }
}

fn run_lux_spec_loop_command(args: LuxSpecLoopArgs) -> anyhow::Result<()> {
    match args.action {
        LuxSpecLoopAction::Start(start_args) => {
            let project_root = resolve_lux_project_root(&start_args.project_path)?;
            let run = lux_spec_loop::start(&project_root, start_args.max_iterations)?;
            print_spec_loop_run(&run);
            Ok(())
        }
        LuxSpecLoopAction::Status(run_args) => {
            let project_root = resolve_lux_project_root(&run_args.project_path)?;
            let run = lux_spec_loop::load(&project_root, &run_args.run_id)?;
            print_spec_loop_run(&run);
            Ok(())
        }
        LuxSpecLoopAction::Answer(answer_args) => {
            let project_root = resolve_lux_project_root(&answer_args.project_path)?;
            let run = lux_spec_loop::answer(
                &project_root,
                &answer_args.run_id,
                &answer_args.question_id,
                &answer_args.answer,
            )?;
            print_spec_loop_run(&run);
            Ok(())
        }
        LuxSpecLoopAction::Approve(proposal_args) => {
            let project_root = resolve_lux_project_root(&proposal_args.project_path)?;
            let run = lux_spec_loop::approve(
                &project_root,
                &proposal_args.run_id,
                &proposal_args.proposal_id,
            )?;
            print_spec_loop_run(&run);
            Ok(())
        }
        LuxSpecLoopAction::Reject(proposal_args) => {
            let project_root = resolve_lux_project_root(&proposal_args.project_path)?;
            let run = lux_spec_loop::reject(
                &project_root,
                &proposal_args.run_id,
                &proposal_args.proposal_id,
            )?;
            print_spec_loop_run(&run);
            Ok(())
        }
        LuxSpecLoopAction::Apply(run_args) => {
            let project_root = resolve_lux_project_root(&run_args.project_path)?;
            let run = lux_spec_loop::apply_approved(&project_root, &run_args.run_id)?;
            print_spec_loop_run(&run);
            Ok(())
        }
    }
}

fn print_spec_loop_run(run: &lux_spec_loop::SpecLoopRun) {
    println!("run_id={}", run.id);
    println!("state={:?}", run.state);
    if let Some(question) = run
        .questions
        .iter()
        .find(|question| question.answer.is_none())
    {
        println!("question_id={}", question.id);
        println!("question_domain={}", question.domain);
    }
    if let Some(proposal) = run.proposals.iter().find(|proposal| {
        proposal.status == lux_spec_loop::SpecProposalStatus::Pending
            || proposal.status == lux_spec_loop::SpecProposalStatus::Approved
    }) {
        println!("proposal_id={}", proposal.id);
        println!("proposal_status={:?}", proposal.status);
    }
    println!("questions={}", run.questions.len());
    println!("proposals={}", run.proposals.len());
}

fn run_lux_roadmap_command(args: LuxRoadmapArgs) -> anyhow::Result<()> {
    match args.action.unwrap_or(LuxRoadmapAction::Status) {
        LuxRoadmapAction::Init(init_args) => {
            let project_path = init_args
                .project_path
                .as_ref()
                .or(args.project_path.as_ref())
                .cloned();
            let project_root = resolve_lux_project_root(&project_path)?;
            let roadmap_path = lux_roadmap::roadmap_file_path(&project_root);
            if roadmap_path.exists() {
                anyhow::bail!("Lux roadmap already exists: {}", roadmap_path.display());
            }
            let roadmap = lux_roadmap::roadmap_template_for_engine(init_args.engine.into());
            lux_roadmap::save(&project_root, &roadmap)?;
            println!("Lux roadmap initialized: {}", roadmap_path.display());
            Ok(())
        }
        LuxRoadmapAction::Status => {
            let project_root = resolve_lux_project_root(&args.project_path)?;
            print_lux_roadmap_status(&project_root)
        }
        LuxRoadmapAction::IssueRegister(register_args) => {
            let project_path = register_args
                .project_path
                .as_ref()
                .or(args.project_path.as_ref())
                .cloned();
            let project_root = resolve_lux_project_root(&project_path)?;
            let request = lux_roadmap_issue_register::IssueRegisterRequest {
                project_root,
                repo: register_args.repo,
                dry_run: register_args.dry_run,
                existing_issues_json: register_args.existing_issues_json,
            };
            let plan = lux_roadmap_issue_register::register_roadmap_issues(request)?;
            if register_args.json {
                println!("{}", serde_json::to_string_pretty(&plan)?);
            } else {
                println!(
                    "GitHub issue registration plan for {}: {} planned, {} existing, {} created",
                    plan.repo, plan.planned_count, plan.existing_count, plan.created_count
                );
                for item in plan.items {
                    println!("- {:?}: {}", item.action, item.title);
                }
            }
            Ok(())
        }
    }
}

fn print_lux_roadmap_status(project_root: &Path) -> anyhow::Result<()> {
    let roadmap = lux_roadmap::load(project_root)?;
    println!(
        "Lux roadmap: {}",
        lux_roadmap::roadmap_file_path(project_root).display()
    );
    println!("Schema: {}", roadmap.schema_version);
    println!("Updated: {}", roadmap.updated_at);
    println!("Authoritative: {}", roadmap.authoritative);
    println!("Capabilities: {}", roadmap.capabilities.len());
    println!("Evidence refs: {}", roadmap.evidence_refs.len());
    println!("Experimental flags: {}", roadmap.experimental_flags.len());
    println!("Phases: {}", roadmap.phases.len());
    for phase in roadmap.phases {
        println!("- {}: {:?}", phase.name, phase.status);
    }
    Ok(())
}

fn run_lux_kanban_command(args: LuxProjectArgs) -> anyhow::Result<()> {
    use lux_ticket::TicketStore;

    let project_root = resolve_lux_project_root(&args.project_path)?;
    let mut tickets = lux_ticket::FileTicketStore::new(&project_root).list(Default::default())?;
    tickets.sort_by(|left, right| {
        left.updated_at
            .cmp(&right.updated_at)
            .then(left.id.cmp(&right.id))
    });

    let statuses = [
        lux_ticket::TicketStatus::Backlog,
        lux_ticket::TicketStatus::Blocked,
        lux_ticket::TicketStatus::ToDo,
        lux_ticket::TicketStatus::InProgress,
        lux_ticket::TicketStatus::Done,
    ];
    println!("Lux kanban: {} tickets", tickets.len());
    println!(
        "{:<12} {:>5} {:>9} {:>7} {:>8} {:>4}",
        "STATUS", "TOTAL", "CRITICAL", "HIGH", "BLOCKERS", "OPEN"
    );
    for status in statuses {
        let matching = tickets
            .iter()
            .filter(|ticket| ticket.status == status)
            .collect::<Vec<_>>();
        let critical = matching
            .iter()
            .filter(|ticket| ticket.priority == lux_ticket::TicketPriority::Critical)
            .count();
        let high = matching
            .iter()
            .filter(|ticket| ticket.priority == lux_ticket::TicketPriority::High)
            .count();
        let blockers = matching
            .iter()
            .filter(|ticket| !ticket.blockers.is_empty())
            .count();
        let open = matching
            .iter()
            .filter(|ticket| ticket.status != lux_ticket::TicketStatus::Done)
            .count();
        println!(
            "{:<12} {:>5} {:>9} {:>7} {:>8} {:>4}",
            lux_ticket_status_label(&status),
            matching.len(),
            critical,
            high,
            blockers,
            open
        );
    }
    Ok(())
}

fn run_autonomous_command(args: AutonomousArgs) -> anyhow::Result<()> {
    match args.action {
        AutonomousCommand::Status(project_args) => {
            let project_root = resolve_lux_project_root(&project_args.project_path)?;
            let state = lux_run_state::RunState::load(&project_root)?;
            println!("seq:    {}", state.seq);
            println!("run_id: {}", state.run_id);
            println!("status: {}", state.status);
            if let Some(ticket_id) = &state.current_ticket_id {
                println!("ticket: {ticket_id}");
            }
            println!("updated_at: {}", state.updated_at);
            Ok(())
        }
        AutonomousCommand::DryRun(dry_run_args) => {
            let project_root = resolve_lux_project_root(&dry_run_args.project_path)?;
            let state = match lux_run_state::RunState::load(&project_root) {
                Ok(s) => s,
                Err(_) => lux_run_state::RunState::idle(&project_root)?,
            };
            use lux_ticket::TicketStore;
            let tickets =
                lux_ticket::FileTicketStore::new(&project_root).list(Default::default())?;
            let dispatchable: Vec<_> = tickets
                .iter()
                .filter(|t| lux_ticket::is_execution_grade(t))
                .filter(|t| dry_run_args.ticket.as_deref().map_or(true, |id| t.id == id))
                .collect();
            println!("dry-run: seq={} status={}", state.seq, state.status);
            println!("dispatchable tickets: {}", dispatchable.len());
            for ticket in &dispatchable {
                println!("  - {} ({})", ticket.id, ticket.title);
            }
            Ok(())
        }
        AutonomousCommand::Dispatch(dispatch_args) => {
            let project_root = resolve_lux_project_root(&dispatch_args.project_path)?;
            let new_state = lux_run_state::RunState::transition_with_seq_check(
                &project_root,
                dispatch_args.seq,
                lux_run_state::RunStatus::Executing,
                "cli dispatch",
                |_s| {},
            )?;
            println!(
                "dispatched: seq={} status={}",
                new_state.seq, new_state.status
            );
            Ok(())
        }
        AutonomousCommand::Evidence(evidence_args) => {
            let project_root = resolve_lux_project_root(&evidence_args.project_path)?;
            let run_id = match evidence_args.run_id {
                Some(id) => id,
                None => {
                    let state = lux_run_state::RunState::load(&project_root)?;
                    state.run_id
                }
            };
            let evidence_dir = project_root
                .join(".lux")
                .join("evidence")
                .join("autonomous")
                .join(&run_id);
            if !evidence_dir.exists() {
                eprintln!("no evidence found for run_id: {run_id}");
                return Ok(());
            }
            println!("evidence for run_id: {run_id}");
            println!("path: {}", evidence_dir.display());
            for entry in std::fs::read_dir(&evidence_dir)? {
                let entry = entry?;
                println!("  {}", entry.file_name().to_string_lossy());
            }
            Ok(())
        }
    }
}

fn run_lux_build_command(args: LuxBuildArgs) -> anyhow::Result<()> {
    let project_root = resolve_lux_project_root(&args.project_path)?;
    let mut manager = lux_build::BuildManager::with_project_root(Some(&project_root));
    let target: lux_build::BuildTarget = args.target.into();
    let build_id = lux_build::start_build(&mut manager, &project_root, target)?;
    let job = lux_build::get_build_status(&manager, &build_id)?;
    println!("Queued Lux build: {}", job.build_id);
    println!("Target: {}", job.target.as_unity_arg());
    if let Some(path) = job.artifact_path.as_ref() {
        println!("Artifact: {}", path.display());
    }
    if let Some(command) = job.log.first() {
        println!("Command: {command}");
    }
    Ok(())
}

fn run_lux_play_command(args: LuxProjectArgs) -> anyhow::Result<()> {
    let project_root = resolve_lux_project_root(&args.project_path)?;
    let latest = latest_lux_build_artifact(&project_root)?;
    println!("Opening Lux build: {}", latest.display());
    let status = ProcessCommand::new("open")
        .arg(&latest)
        .status()
        .with_context(|| format!("failed to open {}", latest.display()))?;
    if !status.success() {
        bail!("open exited with status {status}");
    }
    Ok(())
}

fn run_lux_verify_command(args: LuxProjectArgs) -> anyhow::Result<()> {
    let project_root = resolve_lux_project_root(&args.project_path)?;
    let result =
        lux_verification::verify_all(&project_root, lux_verification::VerificationMode::Cached)?;
    println!(
        "Lux verification: {}",
        if result.passed { "passed" } else { "failed" }
    );
    println!("Overall score: {:.2}", result.overall_score);
    println!(
        "{:<24} {:<24} {:<7} {:<6} MESSAGE",
        "CHECK", "CATEGORY", "PASSED", "SCORE"
    );
    for check in &result.checks {
        println!(
            "{:<24} {:<24} {:<7} {:<6.2} {}",
            check.name,
            format!("{:?}", check.category),
            check.passed,
            check.score,
            check.message
        );
    }
    if !result.blocker_ticket_ids.is_empty() {
        println!("Blocker tickets: {}", result.blocker_ticket_ids.join(", "));
    }
    if !result.passed {
        std::process::exit(1);
    }
    Ok(())
}

fn lux_spec_domain_rows(
    spec: &lux_spec::SpecProject,
) -> [(&'static str, Option<&lux_spec::DomainSpec>); 13] {
    [
        ("gdd", spec.domains.gdd.as_ref()),
        ("mechanics", spec.domains.mechanics.as_ref()),
        ("controls", spec.domains.controls.as_ref()),
        ("camera", spec.domains.camera.as_ref()),
        ("art-style", spec.domains.art_style.as_ref()),
        ("audio", spec.domains.audio.as_ref()),
        ("narrative", spec.domains.narrative.as_ref()),
        ("levels", spec.domains.levels.as_ref()),
        (
            "technical-architecture",
            spec.domains.technical_architecture.as_ref(),
        ),
        ("engine", spec.domains.engine.as_ref()),
        ("testing", spec.domains.testing.as_ref()),
        ("build-release", spec.domains.build_release.as_ref()),
        ("ui-ux", spec.domains.ui_ux.as_ref()),
    ]
}

fn lux_ticket_status_label(status: &lux_ticket::TicketStatus) -> &'static str {
    match status {
        lux_ticket::TicketStatus::Backlog => "Backlog",
        lux_ticket::TicketStatus::Blocked => "Blocked",
        lux_ticket::TicketStatus::ToDo => "ToDo",
        lux_ticket::TicketStatus::InProgress => "InProgress",
        lux_ticket::TicketStatus::Done => "Done",
    }
}

fn latest_lux_build_artifact(project_root: &Path) -> anyhow::Result<PathBuf> {
    let builds_dir = project_root.join(".lux/builds");
    let mut candidates = Vec::new();
    if !builds_dir.exists() {
        bail!(
            "Lux builds directory does not exist: {}",
            builds_dir.display()
        );
    }
    for entry in fs::read_dir(&builds_dir)
        .with_context(|| format!("failed to read {}", builds_dir.display()))?
    {
        let entry = entry?;
        let artifact = entry.path().join("index.html");
        if artifact.is_file() {
            let modified = artifact
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(UNIX_EPOCH);
            candidates.push((modified, artifact));
        }
    }
    candidates.sort_by(|left, right| right.0.cmp(&left.0));
    candidates
        .into_iter()
        .map(|(_, path)| path)
        .next()
        .context("no Lux WebGL build artifacts found under .lux/builds")
}

fn run_status_command(args: StatusArgs, config: &config::LuxConfig) -> anyhow::Result<()> {
    let server_running = is_tcp_port_open(&config.server.host, config.server.port);
    let project_root = args
        .project_path
        .or_else(|| config.general.project_root.clone())
        .or_else(|| {
            project::detect_from_cwd()
                .ok()
                .flatten()
                .map(|info| info.root)
        });

    let (project, bridge) = match project_root.as_deref() {
        Some(root) => {
            let detected = project::detect_from_path(root)?;
            let discovery_path = root.join("Library/UnityAiBridge/server.json");
            let discovery = read_unity_bridge_discovery(root).ok();
            (
                detected.as_ref().map(|info| {
                    json!({
                        "path": info.root,
                        "name": info.project_name,
                        "unity_version": info.editor_version,
                    })
                }),
                json!({
                    "installed": discovery_path.is_file(),
                    "discovery_path": discovery_path,
                    "host": discovery.as_ref().map(|value| value.host.as_str()),
                    "port": discovery.as_ref().map(|value| value.port),
                }),
            )
        }
        None => (
            None,
            json!({
                "installed": false,
                "discovery_path": Value::Null,
                "host": Value::Null,
                "port": Value::Null,
            }),
        ),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "server": {
                "running": server_running,
                "host": config.server.host,
                "port": config.server.port,
            },
            "project": project,
            "bridge": bridge,
        }))?
    );
    Ok(())
}

fn run_self_update_command(args: SelfUpdateArgs) -> anyhow::Result<()> {
    let project_path = match args.project_path {
        Some(p) => p,
        None => std::env::current_dir().context("failed to get current directory")?,
    };

    let state_path = lux_run_state::RunState::path(&project_path);
    if state_path.exists() {
        let state = lux_run_state::RunState::load(&project_path)?;
        let status = state
            .status
            .parse::<lux_run_state::RunStatus>()
            .with_context(|| {
                format!(
                    "failed to parse run status '{}' from {}",
                    state.status,
                    state_path.display()
                )
            })?;

        let is_safe = matches!(
            status,
            lux_run_state::RunStatus::Idle
                | lux_run_state::RunStatus::Completed
                | lux_run_state::RunStatus::Failed
                | lux_run_state::RunStatus::Interrupted
        );

        if !is_safe {
            if args.force {
                eprintln!(
                    "warning: lux run is active (status={}, run_id={}). Proceeding with --force.",
                    state.status, state.run_id
                );
            } else {
                anyhow::bail!(
                    "lux update refused: run is active (status={}, run_id={}). \
                     Stop or complete the run first, or pass --force to override.",
                    state.status,
                    state.run_id
                );
            }
        }
    }

    if auto_update::update_check_due() {
        eprintln!("Update check is due. Spawning background update check...");
        auto_update::maybe_spawn_update_check(false);
        eprintln!("Update check spawned. The binary will be replaced on next restart if an update is available.");
    } else {
        eprintln!("No update check due. lux is up to date.");
    }

    Ok(())
}

fn is_tcp_port_open(host: &str, port: u16) -> bool {
    is_tcp_endpoint_open(host, port)
}

fn is_tcp_endpoint_open(host: &str, port: u16) -> bool {
    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return false;
    };

    addresses.into_iter().any(|address| {
        std::net::TcpStream::connect_timeout(&address, Duration::from_millis(150)).is_ok()
    })
}

fn is_gateway_url_reachable(gateway_url: Option<&str>) -> bool {
    let Some(gateway_url) = gateway_url else {
        return false;
    };
    let Ok(url) = reqwest::Url::parse(gateway_url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let Some(port) = url.port_or_known_default() else {
        return false;
    };

    is_tcp_endpoint_open(host, port)
}

// ---------------------------------------------------------------------------
// lux config
// ---------------------------------------------------------------------------

fn active_config_path() -> PathBuf {
    CONFIG_PATH_OVERRIDE
        .get()
        .cloned()
        .unwrap_or_else(config::config_path)
}

fn load_active_config() -> anyhow::Result<config::LuxConfig> {
    config::load_from_path(active_config_path())
}

fn save_active_config(config: &config::LuxConfig) -> anyhow::Result<()> {
    config::save_to_path(active_config_path(), config)
}

fn run_config_command(
    args: ConfigArgs,
    effective_config: &config::LuxConfig,
) -> anyhow::Result<()> {
    match args.action {
        ConfigAction::Show => {
            println!("{}", toml::to_string_pretty(effective_config)?);
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let mut stored_config = load_active_config()?;
            set_config_value(&mut stored_config, &key, &value)?;
            save_active_config(&stored_config)?;
            Ok(())
        }
        ConfigAction::Get { key } => print_config_value(effective_config, &key),
        ConfigAction::Path => {
            println!("{}", active_config_path().display());
            Ok(())
        }
        ConfigAction::Edit => edit_config_file(),
    }
}

fn set_config_value(config: &mut config::LuxConfig, key: &str, value: &str) -> anyhow::Result<()> {
    match key {
        "unity.hub_path" => config.unity.hub_path = Some(PathBuf::from(value)),
        "unity.editor_path" => config.unity.editor_path = Some(PathBuf::from(value)),
        "unity.custom_install_path" => {
            config.unity.custom_install_path = Some(PathBuf::from(value))
        }
        "server.host" => config.server.host = value.to_string(),
        "server.port" => config.server.port = value.parse().context("server.port must be a u16")?,
        "server.idle_timeout_secs" => {
            config.server.idle_timeout_secs = value
                .parse()
                .context("server.idle_timeout_secs must be a u64")?
        }
        "server.token" => config.server.token = Some(value.to_string()),
        "general.project_root" => config.general.project_root = Some(PathBuf::from(value)),
        "general.log_level" => config.general.log_level = value.to_string(),
        _ => bail!("unknown config key: {key}"),
    }
    Ok(())
}

fn print_config_value(config: &config::LuxConfig, key: &str) -> anyhow::Result<()> {
    match key {
        "unity.hub_path" => print_optional_path(&config.unity.hub_path),
        "unity.editor_path" => print_optional_path(&config.unity.editor_path),
        "unity.custom_install_path" => print_optional_path(&config.unity.custom_install_path),
        "server.host" => println!("{}", config.server.host),
        "server.port" => println!("{}", config.server.port),
        "server.idle_timeout_secs" => println!("{}", config.server.idle_timeout_secs),
        "server.token" => print_optional_string(&config.server.token),
        "general.project_root" => print_optional_path(&config.general.project_root),
        "general.log_level" => println!("{}", config.general.log_level),
        _ => bail!("unknown config key: {key}"),
    }
    Ok(())
}

fn print_optional_path(value: &Option<PathBuf>) {
    if let Some(value) = value {
        println!("{}", value.display());
    }
}

fn print_optional_string(value: &Option<String>) {
    if let Some(value) = value {
        println!("{value}");
    }
}

fn edit_config_file() -> anyhow::Result<()> {
    let path = active_config_path();
    if !path.exists() {
        save_active_config(&config::LuxConfig::default())?;
    }

    let editor = std::env::var_os("VISUAL").or_else(|| std::env::var_os("EDITOR"));
    if let Some(editor) = editor {
        let status = ProcessCommand::new(editor)
            .arg(&path)
            .status()
            .with_context(|| format!("failed to open editor for {}", path.display()))?;
        if !status.success() {
            bail!("editor exited with status {status}");
        }
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = ProcessCommand::new("cmd");
        command.args(["/C", "start", "", &path.display().to_string()]);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = ProcessCommand::new("open");
        command.arg(&path);
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = ProcessCommand::new("xdg-open");
        command.arg(&path);
        command
    };

    command
        .status()
        .with_context(|| format!("failed to open config file {}", path.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// lux session
// ---------------------------------------------------------------------------

fn run_session_command(args: SessionArgs) -> anyhow::Result<()> {
    match args.action {
        SessionAction::Record(a) => run_session_record(a),
        SessionAction::Stop(a) => run_session_stop(a),
        SessionAction::Replay(a) => run_session_replay(a),
        SessionAction::Timeline(a) => run_session_timeline(a),
        SessionAction::Report(a) => run_session_report(a),
    }
}

fn run_session_record(args: SessionRecordArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let (session_id, session_path) = session::start_session(&project_root)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "sessionId": session_id,
            "sessionPath": session_path,
        }))?
    );
    Ok(())
}

fn run_session_stop(args: SessionStopArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let session_id = match args.session_id {
        Some(id) => id,
        None => session::current_session_id(&project_root)?,
    };
    let session_file = session::stop_session_in_project(&project_root, &session_id)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "sessionId": session_id,
            "eventCount": session_file.events.len(),
        }))?
    );
    Ok(())
}

fn run_session_replay(args: SessionReplayArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let session_id = match args.session_id {
        Some(id) => id,
        None => session::current_session_id(&project_root)?,
    };
    let options = session::ReplayOptions {
        speed: args.speed,
        stop_on_error: false,
        filter_types: args.filter_type.map_or_else(Vec::new, |t| vec![t]),
    };
    let result = session::replay_session_in_project(&project_root, &session_id, options)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "sessionId": session_id,
            "totalEvents": result.total_events,
            "replayedEvents": result.replayed_events,
            "errors": result.errors,
            "durationMs": result.duration_ms,
        }))?
    );
    Ok(())
}

fn run_session_timeline(args: SessionTimelineArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let timeline = session::timeline_session(
        &project_root,
        args.session_id.as_deref(),
        args.filter_type.as_deref(),
        args.limit,
    )?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&timeline)?);
        return Ok(());
    }
    println!("Session: {}", timeline.session_id);
    println!("Events ({}):", timeline.events.len());
    for event in &timeline.events {
        println!(
            "  [{}] {} - {}",
            event.event_type, event.timestamp_utc, event.summary
        );
    }
    Ok(())
}

fn run_session_report(args: SessionReportArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let session_id = match args.session_id {
        Some(id) => id,
        None => session::current_session_id(&project_root)?,
    };
    let report = session::report_session(&project_root, &session_id)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    println!("Session: {}", report.session_id);
    println!("Total events: {}", report.total_events);
    println!("Duration: {}ms", report.duration_ms);
    println!("Errors: {}", report.error_count);
    if !report.event_type_counts.is_empty() {
        println!("Event types:");
        for (event_type, count) in &report.event_type_counts {
            println!("  {}: {}", event_type, count);
        }
    }
    if !report.errors.is_empty() {
        println!("Error details:");
        for error in &report.errors {
            println!("  - {}", error);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// lux ai-log
// ---------------------------------------------------------------------------

fn run_ai_log_command(args: AiLogArgs) -> anyhow::Result<()> {
    match args.action {
        AiLogAction::Recent(recent_args) => print_ai_log_recent(recent_args),
        AiLogAction::Tail(tail_args) => print_ai_log_tail(tail_args),
        AiLogAction::Context(context_args) => print_ai_log_context(context_args),
        AiLogAction::Compact(compact_args) => compact_ai_log(compact_args),
        AiLogAction::WorkStep(work_step_args) => append_ai_log_work_step(work_step_args),
    }
}

fn print_ai_log_recent(args: AiLogRecentArgs) -> anyhow::Result<()> {
    let log_path = ai_log::ensure_log_path(&args.project_path)?;
    let entries = ai_log::read_log_entries(&log_path, &ai_log_filter_from_recent(&args))?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "projectPath": args.project_path,
                "path": log_path,
                "count": entries.len(),
                "entries": entries,
            }))?
        );
        return Ok(());
    }

    for entry in entries {
        println!("{} {}", entry.timestamp, entry.value);
    }
    Ok(())
}

fn print_ai_log_tail(args: AiLogTailArgs) -> anyhow::Result<()> {
    let log_path = ai_log::ensure_log_path(&args.project_path)?;
    let filter = ai_log::AiLogFilter {
        limit: Some(args.limit),
        ..ai_log::AiLogFilter::default()
    };
    let entries = ai_log::read_log_entries(&log_path, &filter)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "projectPath": args.project_path,
                "path": log_path,
                "follow": args.follow,
                "count": entries.len(),
                "entries": entries,
            }))?
        );
        return Ok(());
    }

    for entry in entries {
        println!("{} {}", entry.timestamp, entry.value);
    }
    if args.follow {
        eprintln!("lux ai-log tail --follow prints a bounded snapshot and exits in this CLI build");
    }
    Ok(())
}

fn print_ai_log_context(args: AiLogContextArgs) -> anyhow::Result<()> {
    let log_path = ai_log::ensure_log_path(&args.project_path)?;
    let filter = ai_log::AiLogFilter {
        limit: Some(args.limit),
        ..ai_log::AiLogFilter::default()
    };
    let entries = ai_log::read_log_entries(&log_path, &filter)?;
    let context = ai_log::build_continuation_context(&entries, Some(args.limit));

    if args.json {
        println!("{}", serde_json::to_string_pretty(&context)?);
        return Ok(());
    }

    for entry in context["entries"].as_array().into_iter().flatten() {
        println!(
            "{} [{}] {}",
            entry["timestampUtc"].as_str().unwrap_or_default(),
            entry["actor"].as_str().unwrap_or_default(),
            entry["summary"].as_str().unwrap_or_default()
        );
    }
    Ok(())
}

fn compact_ai_log(args: AiLogCompactArgs) -> anyhow::Result<()> {
    let log_path = ai_log::ensure_log_path(&args.project_path)?;
    let result = ai_log::compact_log_file(&log_path, args.max_lines)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let confirmation = if args.yes { "confirmed" } else { "manual" };
    println!(
        "Compacted AI log ({confirmation}): kept {} of {} valid lines, dropped {} total lines",
        result.valid_after, result.valid_before, result.lines_dropped
    );
    Ok(())
}

fn append_ai_log_work_step(args: AiLogWorkStepArgs) -> anyhow::Result<()> {
    let log_path = ai_log::ensure_log_path(&args.project_path)?;
    let mut step = ai_log::AiWorkStep {
        step_name: args.name,
        status: args.status,
        tool: args.tool,
        action: args.action,
        summary: args.summary,
        redaction_metadata: None,
        timestamp_utc: current_timestamp_utc(),
    };

    let mut value = serde_json::to_value(&step).context("failed to prepare AI work step")?;
    let metadata = ai_log::redact_entry(&mut value, &args.project_path.to_string_lossy());
    if !metadata.redacted_fields.is_empty() {
        step = serde_json::from_value(value).context("failed to rebuild redacted AI work step")?;
        step.redaction_metadata = Some(metadata);
    }

    ai_log::append_work_step(&log_path, &step)?;
    ai_log::apply_retention_policy(&log_path, &ai_log::RetentionPolicy::default())?;

    println!(
        "Wrote AI work step '{}' ({}) to {}",
        step.step_name,
        step.status,
        log_path.display()
    );
    Ok(())
}

fn current_timestamp_utc() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("{seconds}Z")
}

fn run_screenshot_command(args: ScreenshotArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    match (args.baseline, args.compare) {
        (Some(name), None) => {
            let path = visual_regression::capture_screenshot_baseline(&name, &project_root)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "baselinePath": cross_platform::display_path(&path),
                }))?
            );
        }
        (None, Some(baseline)) => {
            let current = visual_regression::current_screenshot_path(&project_root);
            let comparison = visual_regression::compare_screenshots(&baseline, &current);
            println!("{}", serde_json::to_string_pretty(&comparison)?);
            if !comparison.passes() {
                std::process::exit(1);
            }
        }
        _ => bail!("Specify either --baseline <name> or --compare <baseline-path>"),
    }
    Ok(())
}

fn ai_log_filter_from_recent(args: &AiLogRecentArgs) -> ai_log::AiLogFilter {
    ai_log::AiLogFilter {
        limit: Some(args.limit),
        actor: args.actor.clone(),
        category: args.category.clone(),
        source: args.source.clone(),
        action: args.action.clone(),
        event_type: args.event_type.clone(),
    }
}

// ---------------------------------------------------------------------------
// lux skill
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
struct SkillInfo<'a> {
    manifest: &'a SkillManifest,
    directory_path: &'a Path,
    references: Vec<String>,
    skill_md_preview: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    adaptation_metadata: Option<Value>,
}

fn run_skill_command(args: SkillArgs) -> anyhow::Result<()> {
    match args.action {
        SkillAction::List(list_args) => print_skill_list(list_args),
        SkillAction::Info(info_args) => print_skill_info(info_args),
        SkillAction::Install(install_args) => install_skill(install_args),
        SkillAction::Remove(remove_args) => remove_skill(remove_args),
        SkillAction::Update(update_args) => update_skill(update_args),
    }
}

fn print_skill_list(args: SkillListArgs) -> anyhow::Result<()> {
    let entries: Vec<_> = discover_skills(None)?
        .into_iter()
        .filter(|entry| {
            args.scope
                .map(|scope| entry.scope == scope.as_str())
                .unwrap_or(true)
        })
        .collect();

    if args.json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if entries.is_empty() {
        println!("No skills found");
        return Ok(());
    }

    println!("{:20} {:10} {:8} DESCRIPTION", "NAME", "VERSION", "TYPE");
    for entry in entries {
        println!(
            "{:20} {:10} {:8} {}",
            entry.manifest.name, entry.manifest.version, entry.scope, entry.manifest.description
        );
    }

    Ok(())
}

fn print_skill_info(args: SkillInfoArgs) -> anyhow::Result<()> {
    let entries = discover_skills(None)?;
    let Some(entry) = entries
        .iter()
        .find(|entry| entry.manifest.name == args.name)
    else {
        eprintln!("Error: skill '{}' not found", args.name);
        std::process::exit(1);
    };

    let references = read_skill_references(&entry.directory_path);
    let preview = read_skill_md_preview(&entry.directory_path);
    let adaptation_metadata = read_skill_adaptation_metadata(&entry.directory_path);

    if args.json {
        let info = SkillInfo {
            manifest: &entry.manifest,
            directory_path: &entry.directory_path,
            references,
            skill_md_preview: preview,
            adaptation_metadata,
        };
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    println!("Name:         {}", entry.manifest.name);
    println!(
        "Display Name: {}",
        entry.manifest.display_name.as_deref().unwrap_or("N/A")
    );
    println!("Version:      {}", entry.manifest.version);
    println!("Description:  {}", entry.manifest.description);
    println!("Type:         {}", entry.manifest.skill_type);
    println!(
        "Author:       {}",
        entry
            .manifest
            .author
            .as_ref()
            .map(|author| author.name.as_str())
            .unwrap_or("N/A")
    );
    println!(
        "Keywords:     {}",
        entry
            .manifest
            .keywords
            .as_ref()
            .filter(|keywords| !keywords.is_empty())
            .map(|keywords| keywords.join(", "))
            .unwrap_or_else(|| "N/A".to_string())
    );
    println!(
        "Lux Version:  {}",
        entry.manifest.lux_version.as_deref().unwrap_or("N/A")
    );
    println!("Location:     {}", entry.directory_path.display());
    if adaptation_metadata.is_some() {
        println!("Adapted:      yes");
    }
    println!();
    println!("References:");
    if references.is_empty() {
        println!("  N/A");
    } else {
        for reference in references {
            println!("  - {}", reference);
        }
    }
    println!();
    println!("SKILL.md preview:");
    if preview.is_empty() {
        println!("  N/A");
    } else {
        for line in preview {
            println!("  {}", line);
        }
    }

    Ok(())
}

fn install_skill(args: SkillInstallArgs) -> anyhow::Result<()> {
    if let Err(message) = validate_skill_name(&args.name) {
        fail_skill_install(args.json, &message);
    }
    let target_scope = writable_scope_from_install(&args)?;

    if args.adapt && target_scope != WritableSkillScope::Project {
        fail_skill_install(args.json, "--adapt requires --project");
    }
    if discover_skills(None)?
        .iter()
        .any(|entry| entry.scope == "core" && entry.manifest.name == args.name)
    {
        fail_skill_install(
            args.json,
            &format!("refusing to overwrite core skill '{}'", args.name),
        );
    }

    let target_root = writable_scope_dir(target_scope)?;
    let target_dir = target_root.join(&args.name);

    let adaptation = if args.adapt {
        match build_skill_adaptation_metadata(&args.name, &args.source) {
            Ok(adaptation) => Some(adaptation),
            Err(error) => fail_skill_install(args.json, &error.to_string()),
        }
    } else {
        None
    };

    if target_dir.exists() {
        fail_skill_install(
            args.json,
            &format!(
                "skill '{}' already exists at {}",
                args.name,
                target_dir.display()
            ),
        );
    }

    fs::create_dir_all(&target_dir)
        .with_context(|| format!("failed to create skill directory {}", target_dir.display()))?;

    let result = install_skill_from_source(&args.source, &target_dir);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&target_dir);
        return Err(error);
    }

    if let Some(adaptation) = &adaptation {
        let adaptation_path = target_dir.join("lux-adaptation.json");
        fs::write(&adaptation_path, serde_json::to_string_pretty(adaptation)?).with_context(
            || {
                format!(
                    "failed to write adaptation metadata {}",
                    adaptation_path.display()
                )
            },
        )?;
    }

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "installed": true,
                "name": args.name,
                "scope": target_scope.as_str(),
                "directory_path": target_dir,
                "adapted": adaptation.is_some(),
                "adaptation_metadata": adaptation.as_ref(),
            }))?
        );
        return Ok(());
    }
    println!(
        "Installed skill '{}' to {}",
        args.name,
        target_dir.display()
    );
    Ok(())
}

fn remove_skill(args: SkillRemoveArgs) -> anyhow::Result<()> {
    let target_scope = writable_scope_from_remove(&args)?;
    if args.project && args.global {
        eprintln!("Error: choose only one scope");
        std::process::exit(1);
    }

    if discover_skills(None)?
        .iter()
        .any(|entry| entry.scope == "core" && entry.manifest.name == args.name)
    {
        eprintln!("Error: refusing to remove core skill '{}'", args.name);
        std::process::exit(1);
    }

    let target_dir = if let Some(scope) = target_scope {
        writable_scope_dir(scope)?.join(&args.name)
    } else {
        let project_dir = project_skills_dir()
            .context("failed to determine project skills directory")?
            .join(&args.name);
        if project_dir.exists() {
            project_dir
        } else {
            global_skills_dir()
                .context("failed to determine global skills directory")?
                .join(&args.name)
        }
    };

    if !target_dir.exists() {
        eprintln!("Error: skill '{}' not found", args.name);
        std::process::exit(1);
    }

    fs::remove_dir_all(&target_dir)
        .with_context(|| format!("failed to remove skill directory {}", target_dir.display()))?;
    println!(
        "Removed skill '{}' from {}",
        args.name,
        target_dir.display()
    );
    Ok(())
}

fn update_skill(args: SkillUpdateArgs) -> anyhow::Result<()> {
    let entries = discover_skills(None)?;
    let Some(entry) = find_skill_for_update(&entries, &args.name, args.scope) else {
        eprintln!("Error: skill '{}' not found", args.name);
        std::process::exit(1);
    };

    let Some(source) = entry.manifest.source.as_deref() else {
        eprintln!("Error: Skill has no source URL configured");
        std::process::exit(1);
    };

    if entry.directory_path.exists() {
        fs::remove_dir_all(&entry.directory_path).with_context(|| {
            format!(
                "Failed to clear skill directory: {}",
                entry.directory_path.display()
            )
        })?;
    }
    fs::create_dir_all(&entry.directory_path).with_context(|| {
        format!(
            "Failed to recreate skill directory: {}",
            entry.directory_path.display()
        )
    })?;

    install_skill_from_source(source, &entry.directory_path)?;
    println!(
        "Updated skill '{}' at {}",
        args.name,
        entry.directory_path.display()
    );
    Ok(())
}

fn find_skill_for_update<'a>(
    entries: &'a [SkillEntry],
    name: &str,
    scope: Option<WritableSkillScope>,
) -> Option<&'a SkillEntry> {
    if let Some(scope) = scope {
        return entries
            .iter()
            .find(|entry| entry.manifest.name == name && entry.scope == scope.as_str());
    }
    entries
        .iter()
        .find(|entry| entry.manifest.name == name && entry.scope == "project")
        .or_else(|| {
            entries
                .iter()
                .find(|entry| entry.manifest.name == name && entry.scope == "global")
        })
        .or_else(|| entries.iter().find(|entry| entry.manifest.name == name))
}

fn writable_scope_from_install(args: &SkillInstallArgs) -> anyhow::Result<WritableSkillScope> {
    if args.project && args.scope.is_some() {
        bail!("choose either --project or --scope, not both");
    }
    Ok(args.scope.unwrap_or(if args.project {
        WritableSkillScope::Project
    } else {
        WritableSkillScope::Global
    }))
}

fn writable_scope_from_remove(
    args: &SkillRemoveArgs,
) -> anyhow::Result<Option<WritableSkillScope>> {
    let legacy_scope = match (args.project, args.global) {
        (true, false) => Some(WritableSkillScope::Project),
        (false, true) => Some(WritableSkillScope::Global),
        (false, false) => None,
        (true, true) => bail!("choose only one scope"),
    };
    if legacy_scope.is_some() && args.scope.is_some() {
        bail!("choose either legacy scope flags or --scope, not both");
    }
    Ok(args.scope.or(legacy_scope))
}

fn writable_scope_dir(scope: WritableSkillScope) -> anyhow::Result<PathBuf> {
    match scope {
        WritableSkillScope::Project => {
            project_skills_dir().context("failed to determine project skills directory")
        }
        WritableSkillScope::Global => {
            global_skills_dir().context("failed to determine global skills directory")
        }
    }
}

fn project_skills_dir() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|directory| directory.join(".agents").join("skills"))
}

fn global_skills_dir() -> Option<PathBuf> {
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    };
    home.map(|directory| PathBuf::from(directory).join(".agents").join("skills"))
}

fn validate_skill_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("skill name must not be empty".to_string());
    }
    if name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err(format!(
            "unsafe skill name '{}': path traversal is not allowed",
            name
        ));
    }
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return Err(format!(
            "unsafe skill name '{}': use only letters, numbers, '-', '_' or '.'",
            name
        ));
    }
    Ok(())
}

fn fail_skill_install(json_output: bool, message: &str) -> ! {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "installed": false,
                "error": message,
            }))
            .expect("serialize skill install error")
        );
    } else {
        eprintln!("Error: {message}");
    }
    std::process::exit(1);
}

fn build_skill_adaptation_metadata(name: &str, source: &str) -> anyhow::Result<Value> {
    if is_url_source(source) {
        bail!("--adapt requires a local skill source directory");
    }

    let project_root = std::env::current_dir().context("failed to determine project root")?;
    let decision = skill_adapter::build_adaptation_decision(name, source, &project_root)?;

    if !decision.compatibility.compatible {
        bail!(
            "skill '{}' is incompatible with this project: {}",
            name,
            decision.compatibility.reasons.join(", ")
        );
    }
    Ok(serde_json::to_value(&decision).context("failed to serialize adaptation decision")?)
}

fn install_skill_from_source(source: &str, target_dir: &Path) -> anyhow::Result<()> {
    if is_url_source(source) {
        eprintln!("Note: URL-based skill install/update is a placeholder");
        download_skill_file(source, "manifest.json", target_dir, true)?;
        download_skill_file(source, "SKILL.md", target_dir, false)?;
        return Ok(());
    }

    let source_dir = Path::new(source);
    if !source_dir.is_dir() {
        bail!("source is not a directory: {}", source_dir.display());
    }

    copy_required_skill_file(source_dir, target_dir, "manifest.json")?;
    copy_required_skill_file(source_dir, target_dir, "SKILL.md")?;

    let references_dir = source_dir.join("references");
    if references_dir.is_dir() {
        let target_references_dir = target_dir.join("references");
        if target_references_dir.exists() {
            fs::remove_dir_all(&target_references_dir).with_context(|| {
                format!(
                    "failed to replace references directory {}",
                    target_references_dir.display()
                )
            })?;
        }
        copy_dir_recursive(&references_dir, &target_references_dir)?;
    }

    Ok(())
}

fn copy_required_skill_file(
    source_dir: &Path,
    target_dir: &Path,
    file_name: &str,
) -> anyhow::Result<()> {
    let source_path = source_dir.join(file_name);
    let target_path = target_dir.join(file_name);
    fs::copy(&source_path, &target_path).with_context(|| {
        format!(
            "failed to copy {} to {}",
            source_path.display(),
            target_path.display()
        )
    })?;
    Ok(())
}

fn copy_dir_recursive(source_dir: &Path, target_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(target_dir)
        .with_context(|| format!("failed to create directory {}", target_dir.display()))?;

    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("failed to read directory {}", source_dir.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target_dir.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn is_url_source(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

fn download_skill_file(
    source: &str,
    file_name: &str,
    target_dir: &Path,
    required: bool,
) -> anyhow::Result<()> {
    let url = format!("{}/{}", source.trim_end_matches('/'), file_name);
    let target_path = target_dir.join(file_name);
    let output = ProcessCommand::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--output",
        ])
        .arg(&target_path)
        .arg(&url)
        .output()
        .with_context(|| format!("failed to start curl for {url}"))?;

    if output.status.success() {
        return Ok(());
    }

    let _ = fs::remove_file(&target_path);
    if required {
        bail!(
            "failed to download {url}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    eprintln!("Warning: failed to download optional {file_name} from {url}");
    Ok(())
}

fn read_skill_adaptation_metadata(directory_path: &Path) -> Option<Value> {
    skill_adapter::read_adaptation_file(directory_path)
}

// ---------------------------------------------------------------------------
// lux godot
// ---------------------------------------------------------------------------

fn run_lux_godot_command(args: GodotArgs) -> anyhow::Result<()> {
    match args.command {
        GodotCommand::Status(status_args) => print_lux_godot_status(status_args),
        GodotCommand::Build(build_args) => run_lux_godot_build(build_args),
    }
}

fn print_lux_godot_status(args: GodotStatusArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let detection = project_godot::detect_godot_project(&project_root);
    let gopeak = gopeak_manifest::sync_manifest(&project_root)?;
    let detected = detection.is_some();
    let _capabilities = lux_engines::write_engine_capability_snapshot(
        &project_root,
        lux_project::EngineKind::Godot,
    )?;
    let capability_blockers = lux_project::recommended_capability_blockers(
        detected.then_some(lux_project::EngineKind::Godot),
    );
    let supported_commands = if detected {
        vec!["godot status", "bridge install --type godot"]
    } else {
        Vec::new()
    };
    let unsupported_commands = vec![
        "godot build",
        "godot run",
        "godot test",
        "godot scene inspect",
        "godot screenshot",
    ];
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": detected,
            "engine": "godot",
            "project_root": project_root,
            "detected": detection.as_ref().map(|detection| {
                json!({
                    "godot_version": detection.godot_version,
                    "has_godot_dir": detection.has_godot_dir,
                })
            }),
            "gopeak": {
                "installed": gopeak.installed,
                "available_commands": gopeak.available_commands,
                "missing_commands": gopeak.missing_commands,
            },
            "lux": {
                "supported_commands": supported_commands,
                "unsupported_commands": unsupported_commands,
                "capability_blockers": capability_blockers,
            },
            "message": if detected {
                "Godot 4 project detected"
            } else {
                "Godot 4 project not detected at the specified path"
            },
        }))?
    );
    Ok(())
}

fn run_lux_godot_build(_args: GodotBuildArgs) -> anyhow::Result<()> {
    anyhow::bail!(
        "lux godot build is not supported until GoPeak-backed build has automated verification"
    )
}

// ---------------------------------------------------------------------------
// lux unity status
// ---------------------------------------------------------------------------

fn run_lux_unity_command(args: UnityArgs) -> anyhow::Result<()> {
    match args.command {
        UnityCommand::Status(status_args) => print_lux_unity_status(status_args),
        UnityCommand::Context(context_args) => print_lux_unity_context(context_args),
        UnityCommand::BackendStatus(backend_status_args) => {
            print_lux_backend_status(backend_status_args)
        }
        UnityCommand::BackendListCommands(backend_list_commands_args) => {
            print_lux_backend_command_list(backend_list_commands_args)
        }
        UnityCommand::GetLogs(get_logs_args) => run_uloop_get_logs(get_logs_args),
        UnityCommand::ClearConsole(clear_console_args) => {
            run_uloop_clear_console(clear_console_args)
        }
        UnityCommand::FocusWindow(focus_window_args) => run_uloop_focus_window(focus_window_args),
        UnityCommand::Launch(launch_args) => run_uloop_launch(launch_args),
        UnityCommand::SceneSmoke(scene_smoke_args) => run_lux_scene_smoke(scene_smoke_args),
        UnityCommand::CreateObjects(create_objects_args) => {
            run_lux_create_objects(create_objects_args)
        }
        UnityCommand::FindGameObjects(find_game_objects_args) => {
            print_lux_backend_find_game_objects(find_game_objects_args)
        }
        UnityCommand::GetHierarchy(get_hierarchy_args) => {
            print_lux_backend_get_hierarchy(get_hierarchy_args)
        }
        UnityCommand::ControlPlayMode(control_play_mode_args) => {
            run_uloop_control_play_mode(control_play_mode_args)
        }
        UnityCommand::Screenshot(screenshot_args) => run_uloop_screenshot(screenshot_args),
        UnityCommand::SimulateMouseUi(simulate_mouse_ui_args) => {
            run_uloop_simulate_mouse_ui(simulate_mouse_ui_args)
        }
        UnityCommand::SimulateKeyboard(simulate_keyboard_args) => {
            run_uloop_simulate_keyboard(simulate_keyboard_args)
        }
        UnityCommand::SimulateMouseInput(simulate_mouse_input_args) => {
            run_uloop_simulate_mouse_input(simulate_mouse_input_args)
        }
        UnityCommand::RecordInput(record_input_args) => run_uloop_record_input(record_input_args),
        UnityCommand::ReplayInput(replay_input_args) => run_uloop_replay_input(replay_input_args),
        UnityCommand::ExecuteDynamicCode(execute_dynamic_code_args) => {
            run_uloop_execute_dynamic_code(execute_dynamic_code_args)
        }
        // === MIGRATED from top-level (v2.1) ===
        UnityCommand::Build(build_args) => {
            eprintln!("DEPRECATED: 'lux build' is deprecated. Use 'lux unity build' instead.");
            run_lux_build_command(LuxBuildArgs {
                project_path: build_args.project_path,
                target: build_args.target,
            })
        }
        UnityCommand::Play(play_args) => {
            eprintln!("DEPRECATED: 'lux play' is deprecated. Use 'lux unity play' instead.");
            run_lux_play_command(LuxProjectArgs {
                project_path: play_args.project_path,
            })
        }
        UnityCommand::Compile(compile_args) => {
            eprintln!("DEPRECATED: 'lux compile' is deprecated. Use 'lux unity compile' instead.");
            run_uloop_compile(compile_args)
        }
        UnityCommand::Bridge(bridge_args) => {
            eprintln!("DEPRECATED: 'lux bridge' is deprecated. Use 'lux unity bridge' instead.");
            let bridge_action = match bridge_args.action {
                UnityBridgeAction::Watch(w) => BridgeAction::Watch(BridgeWatchArgs {
                    project_path: w.project_path,
                }),
                UnityBridgeAction::Install(i) => BridgeAction::Install(BridgeInstallArgs {
                    project_path: i.project_path,
                    bridge_type: BridgeKind::Unity,
                    opencode_commands: i.opencode_commands,
                    no_opencode_commands: i.no_opencode_commands,
                }),
            };
            run_bridge_command(BridgeArgs {
                action: bridge_action,
            })
        }
        UnityCommand::RunTests(tests_args) => {
            eprintln!(
                "DEPRECATED: 'lux run-tests' is deprecated. Use 'lux unity run-tests' instead."
            );
            run_uloop_run_tests(tests_args)
        }
        UnityCommand::VisualRegression(vr_args) => {
            eprintln!("DEPRECATED: 'lux screenshot' (visual regression) is deprecated. Use 'lux unity visual-regression' instead.");
            run_screenshot_command(ScreenshotArgs {
                baseline: vr_args.baseline,
                compare: vr_args.compare,
                project_path: vr_args.project_path,
            })
        }
        UnityCommand::InstallUloop(install_args) => {
            let project = resolve_project_root(&Some(install_args.project_path))?;
            install_uloop_package_with_options(
                &project,
                install_args.local,
                install_args.force,
                install_args.version.as_deref(),
            )
        }
    }
}

fn push_string_arg(uloop_args: &mut Vec<String>, flag: &str, value: impl ToString) {
    uloop_args.push(flag.to_string());
    uloop_args.push(value.to_string());
}

fn push_flag_arg(uloop_args: &mut Vec<String>, flag: &str) {
    uloop_args.push(flag.to_string());
}

fn push_optional_string_arg(uloop_args: &mut Vec<String>, flag: &str, value: Option<&str>) {
    if let Some(value) = value {
        uloop_args.push(flag.to_string());
        uloop_args.push(value.to_string());
    }
}

fn push_optional_path_arg(uloop_args: &mut Vec<String>, flag: &str, value: Option<&Path>) {
    if let Some(value) = value {
        uloop_args.push(flag.to_string());
        uloop_args.push(value.to_string_lossy().into_owned());
    }
}

fn uloop_bool(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn uloop_pascal_word(value: &str) -> String {
    let mut output = String::new();
    let mut capitalize_next = true;
    for ch in value.chars() {
        if ch == '-' || ch == '_' || ch == ' ' {
            capitalize_next = true;
            continue;
        }
        if capitalize_next {
            for upper in ch.to_uppercase() {
                output.push(upper);
            }
            capitalize_next = false;
        } else {
            output.push(ch.to_ascii_lowercase());
        }
    }
    output
}

fn run_uloop_with_project(
    project_path: &Option<PathBuf>,
    uloop_args: Vec<String>,
) -> anyhow::Result<()> {
    let project_root = resolve_project_root(project_path)?;
    let uloop_arg_refs = uloop_args.iter().map(String::as_str).collect::<Vec<_>>();
    let (stdout, stderr, code) =
        uloop_runner::run_uloop_command(&uloop_arg_refs, Some(&project_root))?;
    print!("{stdout}");
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    std::process::exit(code);
}

fn run_uloop_get_logs(args: UnityGetLogsArgs) -> anyhow::Result<()> {
    run_uloop_with_project(&args.project_path, vec!["get-logs".to_string()])
}

fn run_uloop_clear_console(args: UnityClearConsoleArgs) -> anyhow::Result<()> {
    run_uloop_with_project(&args.project_path, vec!["clear-console".to_string()])
}

fn run_uloop_focus_window(args: UnityFocusWindowArgs) -> anyhow::Result<()> {
    run_uloop_with_project(&args.project_path, vec!["focus-window".to_string()])
}

fn run_uloop_launch(args: UnityLaunchArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let uloop_args = [
        "launch".to_string(),
        project_root.to_string_lossy().into_owned(),
    ];
    let uloop_arg_refs = uloop_args.iter().map(String::as_str).collect::<Vec<_>>();
    let (stdout, stderr, code) = uloop_runner::run_uloop_command(&uloop_arg_refs, None)?;
    print!("{stdout}");
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    std::process::exit(code);
}

fn run_uloop_control_play_mode(args: UnityControlPlayModeArgs) -> anyhow::Result<()> {
    let mut uloop_args = vec!["control-play-mode".to_string()];
    push_string_arg(
        &mut uloop_args,
        "--action",
        uloop_pascal_word(args.action.as_str()),
    );
    run_uloop_with_project(&args.project_path, uloop_args)
}

fn run_uloop_screenshot(args: UnityScreenshotArgs) -> anyhow::Result<()> {
    let mut uloop_args = vec!["screenshot".to_string()];
    push_string_arg(&mut uloop_args, "--capture-mode", args.capture_mode);
    push_string_arg(
        &mut uloop_args,
        "--annotate-elements",
        uloop_bool(args.annotate_elements),
    );
    push_string_arg(
        &mut uloop_args,
        "--elements-only",
        uloop_bool(args.elements_only),
    );
    run_uloop_with_project(&args.project_path, uloop_args)
}

fn run_uloop_simulate_mouse_ui(args: UnitySimulateMouseUiArgs) -> anyhow::Result<()> {
    let mut uloop_args = vec!["simulate-mouse-ui".to_string()];
    push_string_arg(
        &mut uloop_args,
        "--action",
        uloop_pascal_word(args.action.as_str()),
    );
    push_string_arg(&mut uloop_args, "--x", args.x);
    push_string_arg(&mut uloop_args, "--y", args.y);
    push_string_arg(&mut uloop_args, "--duration", args.duration_ms);
    run_uloop_with_project(&args.project_path, uloop_args)
}

fn run_uloop_simulate_keyboard(args: UnitySimulateKeyboardArgs) -> anyhow::Result<()> {
    let mut uloop_args = vec!["simulate-keyboard".to_string()];
    push_string_arg(
        &mut uloop_args,
        "--action",
        uloop_pascal_word(args.action.as_str()),
    );
    push_string_arg(&mut uloop_args, "--key", args.key);
    push_string_arg(&mut uloop_args, "--duration", args.duration_ms);
    run_uloop_with_project(&args.project_path, uloop_args)
}

fn run_uloop_simulate_mouse_input(args: UnitySimulateMouseInputArgs) -> anyhow::Result<()> {
    let mut uloop_args = vec!["simulate-mouse-input".to_string()];
    push_string_arg(
        &mut uloop_args,
        "--action",
        uloop_pascal_word(args.action.as_str()),
    );
    push_string_arg(&mut uloop_args, "--button", uloop_pascal_word(&args.button));
    push_string_arg(&mut uloop_args, "--delta-x", args.delta_x);
    push_string_arg(&mut uloop_args, "--delta-y", args.delta_y);
    push_string_arg(&mut uloop_args, "--scroll-x", args.scroll_x);
    push_string_arg(&mut uloop_args, "--scroll-y", args.scroll_y);
    push_string_arg(&mut uloop_args, "--duration", args.duration_ms);
    run_uloop_with_project(&args.project_path, uloop_args)
}

fn run_uloop_record_input(args: UnityRecordInputArgs) -> anyhow::Result<()> {
    let mut uloop_args = vec!["record-input".to_string()];
    push_string_arg(
        &mut uloop_args,
        "--action",
        uloop_pascal_word(args.action.as_str()),
    );
    run_uloop_with_project(&args.project_path, uloop_args)
}

fn run_uloop_replay_input(args: UnityReplayInputArgs) -> anyhow::Result<()> {
    let mut uloop_args = vec!["replay-input".to_string()];
    push_string_arg(
        &mut uloop_args,
        "--action",
        uloop_pascal_word(args.action.as_str()),
    );
    push_optional_path_arg(&mut uloop_args, "--input-path", args.file.as_deref());
    run_uloop_with_project(&args.project_path, uloop_args)
}

fn run_uloop_execute_dynamic_code(args: UnityExecuteDynamicCodeArgs) -> anyhow::Result<()> {
    let project_path = args.project_path.clone();
    let code = resolve_dynamic_code_source(&args)?;
    let mut uloop_args = vec!["execute-dynamic-code".to_string()];
    push_string_arg(&mut uloop_args, "--code", code);
    run_uloop_with_project(&project_path, uloop_args)
}

fn run_uloop_compile(args: UnityCompileArgs) -> anyhow::Result<()> {
    run_uloop_with_project(&args.project_path, vec!["compile".to_string()])
}

fn run_uloop_run_tests(args: UnityRunTestsArgs) -> anyhow::Result<()> {
    let mut uloop_args = vec!["run-tests".to_string()];
    push_string_arg(&mut uloop_args, "--test-mode", args.test_platform);
    push_string_arg(&mut uloop_args, "--filter-type", args.filter_type);
    push_optional_string_arg(
        &mut uloop_args,
        "--filter-value",
        args.filter_value.as_deref(),
    );
    if args.save_before_run {
        push_flag_arg(&mut uloop_args, "--save-before-run");
    }
    run_uloop_with_project(&args.project_path, uloop_args)
}

fn run_lux_create_objects(args: UnityCreateObjectsArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    run_lux_backend_object_command(
        &project_root,
        "create_lux_scene_objects",
        &args.scene_path,
        args.object_count,
        Duration::from_secs(10),
    )
}

fn print_lux_backend_find_game_objects(args: UnityFindGameObjectsArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let discovery = read_unity_bridge_discovery(&project_root)?;
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "find_lux_game_objects",
        "token": discovery.token,
        "params": {
            "findGameObjectsSearchMode": args.search_mode,
            "findGameObjectsName": args.name,
            "findGameObjectsRegex": args.regex,
            "findGameObjectsPath": args.path,
            "findGameObjectsComponent": args.component,
            "findGameObjectsTag": args.tag,
            "findGameObjectsLayer": args.layer,
            "findGameObjectsActiveState": args.active_state,
            "findGameObjectsInlineLimit": args.inline_limit,
        }
    });
    let response_line = send_unity_tcp_line(
        &discovery,
        &format!("{}\n", serde_json::to_string(&request)?),
    )?;
    let response_json: Value =
        serde_json::from_str(&response_line).context("Unity TCP response was not valid JSON")?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!(
            "Unity backend rejected find_lux_game_objects: {}",
            response_json
        );
    }
    println!("{}", serde_json::to_string_pretty(&response_json)?);
    Ok(())
}

fn print_lux_backend_get_hierarchy(args: UnityGetHierarchyArgs) -> anyhow::Result<()> {
    let filter_count =
        (args.all as u8) + (args.root_path.is_some() as u8) + (args.use_selection as u8);
    if filter_count > 1 {
        bail!("Specify only one hierarchy filter: --all, --root-path, or --use-selection");
    }

    let project_root = resolve_project_root(&args.project_path)?;
    let discovery = read_unity_bridge_discovery(&project_root)?;
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "get_lux_hierarchy",
        "token": discovery.token,
        "params": {
            "hierarchyAll": args.all || filter_count == 0,
            "hierarchyRootPath": args.root_path,
            "hierarchyUseSelection": args.use_selection,
        }
    });
    let response_json = send_unity_bridge_request(&discovery, &request)?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        if is_unity_registry_not_ready_for(&response_json, "get_lux_hierarchy") {
            return print_lux_backend_scene_ast_hierarchy(&discovery, filter_count == 0 || args.all);
        }

        bail!(
            "Unity backend rejected get_lux_hierarchy: {}",
            response_json
        );
    }

    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("getHierarchyResult"))
        .context("Unity TCP response did not include payload.getHierarchyResult")?;
    let file_path = payload
        .get("filePath")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include payload.getHierarchyResult.filePath")?;
    let file_size_bytes = payload
        .get("fileSizeBytes")
        .and_then(Value::as_i64)
        .context("Unity TCP response did not include payload.getHierarchyResult.fileSizeBytes")?;
    let root_count = payload
        .get("rootCount")
        .and_then(Value::as_i64)
        .context("Unity TCP response did not include payload.getHierarchyResult.rootCount")?;
    let node_count = payload
        .get("nodeCount")
        .and_then(Value::as_i64)
        .context("Unity TCP response did not include payload.getHierarchyResult.nodeCount")?;
    let active_scene = payload
        .get("activeScene")
        .cloned()
        .context("Unity TCP response did not include payload.getHierarchyResult.activeScene")?;
    let filters = payload
        .get("filters")
        .cloned()
        .context("Unity TCP response did not include payload.getHierarchyResult.filters")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "filePath": file_path,
            "fileSizeBytes": file_size_bytes,
            "rootCount": root_count,
            "nodeCount": node_count,
            "activeScene": active_scene,
            "filters": filters,
        }))?
    );
    Ok(())
}

fn print_lux_backend_scene_ast_hierarchy(
    discovery: &UnityBridgeDiscovery,
    include_all: bool,
) -> anyhow::Result<()> {
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "get_scene_ast",
        "token": discovery.token,
        "params": {
            "astRootOnly": !include_all,
        }
    });
    let response_json = send_unity_bridge_request(discovery, &request)?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!("Unity backend rejected get_scene_ast fallback: {}", response_json);
    }

    let scene_ast = response_json
        .get("payload")
        .and_then(|payload| payload.get("sceneAst"))
        .context("Unity TCP response did not include payload.sceneAst")?;
    let scene_name = scene_ast
        .get("sceneName")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include payload.sceneAst.sceneName")?;
    let scene_path = scene_ast
        .get("scenePath")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let root_count = scene_ast
        .get("rootCount")
        .and_then(Value::as_i64)
        .context("Unity TCP response did not include payload.sceneAst.rootCount")?;
    let node_count = scene_ast
        .get("totalGameObjects")
        .and_then(Value::as_i64)
        .context("Unity TCP response did not include payload.sceneAst.totalGameObjects")?;
    let component_count = scene_ast
        .get("totalComponents")
        .and_then(Value::as_i64)
        .unwrap_or_default();

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "source": "get_scene_ast",
            "sceneName": scene_name,
            "scenePath": scene_path,
            "rootCount": root_count,
            "nodeCount": node_count,
            "componentCount": component_count,
        }))?
    );
    Ok(())
}

fn send_unity_bridge_request(
    discovery: &UnityBridgeDiscovery,
    request: &Value,
) -> anyhow::Result<Value> {
    let response_line = send_unity_tcp_line(
        discovery,
        &format!("{}\n", serde_json::to_string(request)?),
    )?;
    serde_json::from_str(&response_line).context("Unity TCP response was not valid JSON")
}

fn is_unity_registry_not_ready_for(response_json: &Value, command: &str) -> bool {
    response_json.get("errorCode").and_then(Value::as_str) == Some("registry_not_ready")
        && response_json
            .get("errorMessage")
            .and_then(Value::as_str)
            .is_some_and(|message| message.contains(command))
}

fn resolve_dynamic_code_source(args: &UnityExecuteDynamicCodeArgs) -> anyhow::Result<String> {
    match (&args.code, &args.file) {
        (Some(_), Some(_)) => bail!("Specify only one dynamic code source: --code or --file"),
        (Some(code), None) => Ok(code.clone()),
        (None, Some(path)) => fs::read_to_string(path)
            .with_context(|| format!("failed to read dynamic code file at {}", path.display())),
        (None, None) => {
            bail!("lux unity execute-dynamic-code requires --code <string> or --file <path>")
        }
    }
}

fn print_lux_backend_status(args: UnityBackendStatusArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let discovery_path = project_root.join("Library/UnityAiBridge/server.json");
    let ping_result = try_ping_unity_bridge_backend(&project_root, Duration::from_secs(10));
    match ping_result {
        Ok(backend) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "running": true,
                    "host": backend.host,
                    "port": backend.port,
                    "discovery_path": backend.discovery_path,
                    "ping": backend.ping,
                }))?
            );
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": false,
                    "running": false,
                    "discovery_path": discovery_path,
                    "message": error.to_string(),
                }))?
            );
        }
    }

    Ok(())
}

fn print_lux_backend_command_list(args: UnityBackendListCommandsArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let response_json = fetch_lux_backend_protocol_info(&project_root)?;
    let protocol_info = response_json
        .get("payload")
        .and_then(|payload| payload.get("protocolInfo"))
        .context("Unity TCP response did not include payload.protocolInfo")?;

    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let backend_version = protocol_info
        .get("backendVersion")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include payload.protocolInfo.backendVersion")?;
    let commands = protocol_info
        .get("commands")
        .and_then(Value::as_array)
        .context("Unity TCP response did not include payload.protocolInfo.commands")?
        .iter()
        .map(|command| {
            command
                .as_str()
                .map(str::to_owned)
                .context("Unity TCP response included a non-string command name")
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": schema_version,
            "backendVersion": backend_version,
            "commands": commands,
            "capturedAtUtc": captured_at_utc,
        }))?
    );

    Ok(())
}

fn fetch_lux_backend_protocol_info(project_root: &Path) -> anyhow::Result<Value> {
    fetch_lux_backend_command_response(project_root, "get_protocol_info")
}

fn fetch_lux_backend_command_response(project_root: &Path, command: &str) -> anyhow::Result<Value> {
    let discovery = read_unity_bridge_discovery(project_root)?;
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": command,
        "token": discovery.token,
        "params": {}
    });
    let response_line = send_unity_tcp_line(
        &discovery,
        &format!("{}\n", serde_json::to_string(&request)?),
    )?;
    let response_json: Value =
        serde_json::from_str(&response_line).context("Unity TCP response was not valid JSON")?;

    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!("Unity TCP rejected {command}: {}", response_json);
    }

    Ok(response_json)
}

fn run_lux_scene_smoke(args: UnitySceneSmokeArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    if !args.batch {
        return run_lux_scene_smoke_live(&project_root, &args)
            .with_context(|| "Lux backend live scene-smoke failed. Start the Lux/Unity AI Bridge backend in the open Unity Editor, or pass --batch only when no Unity instance has the project open.");
    }

    let launch_target = resolve_unity_launch_target(&project_root)?;
    let results_dir = project_root.join("TestResults");
    fs::create_dir_all(&results_dir)
        .with_context(|| format!("failed to create {}", results_dir.display()))?;
    let log_path = results_dir.join("LuxSceneSmoke.log");
    let result_path = results_dir.join("LuxSceneSmokeResult.json");
    if result_path.exists() {
        fs::remove_file(&result_path)
            .with_context(|| format!("failed to remove stale {}", result_path.display()))?;
    }

    eprintln!(
        "Lux scene-smoke: launching Unity batch mode for {}",
        project_root.display()
    );

    let status = ProcessCommand::new(&launch_target.executable)
        .args(&launch_target.prefix_args)
        .arg("-batchmode")
        .arg("-nographics")
        .arg("-projectPath")
        .arg(&project_root)
        .arg("-executeMethod")
        .arg("Linalab.Lux.Editor.LuxSceneSmoke.Run")
        .arg("-logFile")
        .arg(&log_path)
        .env("LUX_SCENE_SMOKE_SCENE_PATH", &args.scene_path)
        .env(
            "LUX_SCENE_SMOKE_OBJECT_COUNT",
            args.object_count.to_string(),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| {
            format!(
                "failed to launch Unity at {}",
                launch_target.executable.display()
            )
        })?;

    if result_path.exists() {
        let result_text = fs::read_to_string(&result_path)
            .with_context(|| format!("failed to read {}", result_path.display()))?;
        println!("{}", result_text.trim());
    } else {
        println!(
            "{{ \"ok\": {}, \"message\": \"Unity exited without writing LuxSceneSmokeResult.json\", \"log\": \"{}\" }}",
            status.success(),
            log_path.display()
        );
    }

    if !status.success() {
        bail!("Lux scene-smoke failed. See log: {}", log_path.display());
    }

    Ok(())
}

fn run_lux_scene_smoke_live(project_root: &Path, args: &UnitySceneSmokeArgs) -> anyhow::Result<()> {
    let result_path = project_root.join("TestResults/LuxSceneSmokeResult.json");
    if result_path.exists() {
        fs::remove_file(&result_path)
            .with_context(|| format!("failed to remove stale {}", result_path.display()))?;
    }

    let discovery = read_unity_bridge_discovery(project_root)?;
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "run_lux_scene_smoke",
        "token": discovery.token,
        "params": {
            "scenePath": args.scene_path,
            "sceneSmokeObjectCount": args.object_count,
            "actor": "lux-cli"
        }
    });
    let response = send_unity_tcp_line_with_timeout(
        &discovery,
        &format!("{}\n", serde_json::to_string(&request)?),
        Duration::from_secs(45),
    )?;
    let response_json: Value =
        serde_json::from_str(&response).context("Unity TCP response was not valid JSON")?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!("Unity TCP rejected scene-smoke: {}", response_json);
    }

    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        if result_path.exists() {
            let result_text = fs::read_to_string(&result_path)
                .with_context(|| format!("failed to read {}", result_path.display()))?;
            println!("{}", result_text.trim());
            let result_json: Value = serde_json::from_str(&result_text)
                .context("LuxSceneSmokeResult.json was not valid JSON")?;
            if result_json.get("ok").and_then(Value::as_bool) == Some(true) {
                return Ok(());
            }
            bail!("Lux live scene-smoke failed: {}", result_text.trim());
        }

        if Instant::now() >= deadline {
            bail!("timed out waiting for {}", result_path.display());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn run_lux_backend_object_command(
    project_root: &Path,
    command: &str,
    scene_path: &str,
    object_count: u32,
    timeout: Duration,
) -> anyhow::Result<()> {
    let result_path = project_root.join("TestResults/LuxSceneSmokeResult.json");
    if result_path.exists() {
        fs::remove_file(&result_path)
            .with_context(|| format!("failed to remove stale {}", result_path.display()))?;
    }

    let discovery = read_unity_bridge_discovery(project_root)?;
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": command,
        "token": discovery.token,
        "params": {
            "scenePath": scene_path,
            "sceneSmokeObjectCount": object_count,
            "actor": "lux-cli"
        }
    });
    let response = send_unity_tcp_line_with_timeout(
        &discovery,
        &format!("{}\n", serde_json::to_string(&request)?),
        Duration::from_secs(30),
    )?;
    let response_json: Value =
        serde_json::from_str(&response).context("Unity TCP response was not valid JSON")?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!("Unity TCP rejected {command}: {}", response_json);
    }

    let deadline = Instant::now() + timeout;
    loop {
        if result_path.exists() {
            let result_text = fs::read_to_string(&result_path)
                .with_context(|| format!("failed to read {}", result_path.display()))?;
            println!("{}", result_text.trim());
            let result_json: Value = serde_json::from_str(&result_text)
                .context("LuxSceneSmokeResult.json was not valid JSON")?;
            if result_json.get("ok").and_then(Value::as_bool) == Some(true) {
                return Ok(());
            }
            bail!(
                "Lux backend command {command} failed: {}",
                result_text.trim()
            );
        }

        if Instant::now() >= deadline {
            bail!("timed out waiting for {}", result_path.display());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn read_unity_bridge_discovery(project_root: &Path) -> anyhow::Result<UnityBridgeDiscovery> {
    let discovery_path = project_root.join("Library/UnityAiBridge/server.json");
    let text = fs::read_to_string(&discovery_path).with_context(|| {
        format!(
            "Unity AI Bridge discovery file not found at {}",
            discovery_path.display()
        )
    })?;
    serde_json::from_str(&text).with_context(|| {
        format!(
            "failed to parse Unity AI Bridge discovery file at {}",
            discovery_path.display()
        )
    })
}

pub fn try_ping_unity_bridge_backend(
    project_root: &Path,
    timeout: Duration,
) -> anyhow::Result<UnityBridgeBackendPing> {
    let discovery_path = project_root.join("Library/UnityAiBridge/server.json");
    let discovery = read_unity_bridge_discovery(project_root)?;
    let ping = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "ping",
        "token": discovery.token,
        "params": {}
    });
    let response_line = send_unity_tcp_line_with_timeout(
        &discovery,
        &format!("{}\n", serde_json::to_string(&ping)?),
        timeout,
    )?;
    let response_json: Value =
        serde_json::from_str(&response_line).context("Unity TCP response was not valid JSON")?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true)
        || response_json
            .get("payload")
            .and_then(|payload| payload.get("ping"))
            .and_then(|ping| ping.get("status"))
            .and_then(Value::as_str)
            != Some("ok")
    {
        bail!("Unity TCP ping was not ready: {}", response_json);
    }

    Ok(UnityBridgeBackendPing {
        host: discovery.host,
        port: discovery.port,
        discovery_path,
        ping: response_json,
    })
}

fn send_unity_tcp_line(
    discovery: &UnityBridgeDiscovery,
    request_line: &str,
) -> anyhow::Result<String> {
    send_unity_tcp_line_with_timeout(discovery, request_line, Duration::from_secs(10))
}

fn send_unity_tcp_line_with_timeout(
    discovery: &UnityBridgeDiscovery,
    request_line: &str,
    timeout: Duration,
) -> anyhow::Result<String> {
    let deadline = Instant::now() + timeout;
    let mut stream = connect_unity_tcp_with_retry(discovery, deadline)?;
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_millis(250)))?;
    write_unity_tcp_with_retry(&mut stream, request_line.as_bytes(), deadline)?;

    let mut buffer = String::new();
    let mut chunk = [0_u8; 1024];
    loop {
        let size = match stream.read(&mut chunk) {
            Ok(size) => size,
            Err(error) if is_transient_socket_error(&error) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
                continue;
            }
            Err(error) => return Err(error).context("Unity TCP response read failed"),
        };
        if size == 0 {
            break;
        }
        buffer.push_str(
            std::str::from_utf8(&chunk[..size]).context("Unity TCP response was not UTF-8")?,
        );
        if let Some(index) = buffer.find('\n') {
            return Ok(buffer[..index].to_string());
        }

        if Instant::now() >= deadline {
            bail!("timed out waiting for Unity TCP response");
        }
    }

    bail!("Unity TCP connection closed before sending a response")
}

fn run_bridge_command(args: BridgeArgs) -> anyhow::Result<()> {
    match args.action {
        BridgeAction::Watch(watch_args) => watch_unity_bridge_events(watch_args),
        BridgeAction::Install(install_args) => {
            let project_root = install_args.project_path.clone();
            match install_args.bridge_type.clone() {
                BridgeKind::Unity => {
                    install_bridge_files(install_args)?;

                    // Also install uloop (unity-cli-loop) for unity CLI operations
                    if let Err(e) = install_uloop_package(&project_root) {
                        eprintln!("⚠️  uloop installation skipped (non-critical): {e}");
                        // Don't fail bridge install — uloop is optional enhancement
                    }
                }
                BridgeKind::Godot => {
                    godot_bridge_install::install_godot_bridge(&project_root)?;
                }
            }

            Ok(())
        }
    }
}

fn prompt_install_opencode_commands(project_root: &Path) -> anyhow::Result<bool> {
    use std::io;
    eprintln!();
    eprintln!(
        "Lux OpenCode command files can be installed to {}/.opencode/commands/ for OpenCode users.",
        project_root.display()
    );
    eprintln!("These are convenience command shortcuts (lux-init, lux-run, etc.).");
    eprintln!();
    eprint!("   Install Lux OpenCode commands to .opencode/commands/? [y/N]: ");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "yes" | "Y" | "YES"))
}

fn watch_unity_bridge_events(args: BridgeWatchArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let discovery = read_unity_bridge_discovery(&project_root)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut stream = connect_unity_tcp_with_retry(&discovery, deadline)?;
    stream.set_read_timeout(Some(Duration::from_millis(250)))?;
    stream.set_write_timeout(Some(Duration::from_millis(250)))?;

    let subscribe = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "subscribe_events",
        "token": discovery.token,
        "params": {
            "eventTypes": "compile_started,compile_result"
        }
    });
    let subscribe_line = format!("{}\n", serde_json::to_string(&subscribe)?);
    write_unity_tcp_with_retry(&mut stream, subscribe_line.as_bytes(), deadline)?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return Ok(()),
            Ok(_) => {
                let trimmed = line.trim_end();
                if trimmed.is_empty() {
                    continue;
                }

                let value: Value = serde_json::from_str(trimmed)
                    .context("Unity AI Bridge watch received invalid JSON")?;
                if value.get("type").and_then(Value::as_str) == Some("event") {
                    println!("{}", serde_json::to_string(&value)?);
                } else if value.get("ok").and_then(Value::as_bool) == Some(false) {
                    bail!("Unity AI Bridge event subscription failed: {}", value);
                }
            }
            Err(error) if is_transient_socket_error(&error) => continue,
            Err(error) => return Err(error).context("Unity AI Bridge watch read failed"),
        }
    }
}

fn install_bridge_files(args: BridgeInstallArgs) -> anyhow::Result<()> {
    let project_root = args.project_path;
    if !project_root.exists() {
        anyhow::bail!("Project path does not exist: {}", project_root.display());
    }
    let assets_editor = project_root.join("Assets/Editor");
    std::fs::create_dir_all(&assets_editor)
        .with_context(|| format!("Failed to create {}", assets_editor.display()))?;

    let bridge_source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(Path::new("."))
        .join("bridge")
        .join("unity");

    let bridge_editor_source = bridge_source.join("AiBridgeEditor");
    let bridge_settings_source = bridge_source.join("LuxBridgeSettings.cs");
    if !bridge_editor_source.is_dir() {
        anyhow::bail!(
            "Unity bridge source directory not found: {}",
            bridge_editor_source.display()
        );
    }
    if !bridge_settings_source.is_file() {
        anyhow::bail!(
            "Unity bridge settings source file not found: {}",
            bridge_settings_source.display()
        );
    }

    let bridge_target = assets_editor.join("LuxBridge");
    if bridge_target.exists() {
        std::fs::remove_dir_all(&bridge_target).with_context(|| {
            format!(
                "Failed to clear existing bridge directory {}",
                bridge_target.display()
            )
        })?;
    }
    copy_dir_recursive(&bridge_editor_source, &bridge_target).with_context(|| {
        format!(
            "Failed to copy {} to {}",
            bridge_editor_source.display(),
            bridge_target.display()
        )
    })?;
    let settings_target = bridge_target.join("LuxBridgeSettings.cs");
    std::fs::copy(&bridge_settings_source, &settings_target).with_context(|| {
        format!(
            "Failed to copy {} to {}",
            bridge_settings_source.display(),
            settings_target.display()
        )
    })?;

    let legacy_bridge_target = assets_editor.join("AiBridgeEditor");
    if legacy_bridge_target.exists() {
        std::fs::remove_dir_all(&legacy_bridge_target).with_context(|| {
            format!(
                "Failed to remove legacy bridge directory {}",
                legacy_bridge_target.display()
            )
        })?;
    }
    eprintln!(
        "Copied {} -> {}",
        bridge_source.display(),
        bridge_target.display()
    );

    // OpenCode commands (.opencode/commands/) — opt-in only, user chooses
    let should_install_opencode = if args.opencode_commands {
        true
    } else if args.no_opencode_commands {
        false
    } else {
        // Interactive prompt (user chooses whether to install .opencode/commands)
        prompt_install_opencode_commands(&project_root)?
    };

    if should_install_opencode {
        let opencode_dir = project_root.join(".opencode");
        let commands_dir = opencode_dir.join("commands");
        std::fs::create_dir_all(&commands_dir)
            .with_context(|| format!("failed to create {}", commands_dir.display()))?;

        let command_files = [
            (
                "lux-init.md",
                include_str!("templates/commands/lux-init.md"),
            ),
            ("lux-run.md", include_str!("templates/commands/lux-run.md")),
            (
                "lux-spec-validate.md",
                include_str!("templates/commands/lux-spec-validate.md"),
            ),
            (
                "lux-spec-edit.md",
                include_str!("templates/commands/lux-spec-edit.md"),
            ),
            (
                "lux-kanban.md",
                include_str!("templates/commands/lux-kanban.md"),
            ),
            (
                "lux-build.md",
                include_str!("templates/commands/lux-build.md"),
            ),
            (
                "lux-verify.md",
                include_str!("templates/commands/lux-verify.md"),
            ),
            (
                "lux-compile.md",
                include_str!("templates/commands/lux-compile.md"),
            ),
            (
                "lux-test.md",
                include_str!("templates/commands/lux-test.md"),
            ),
            (
                "lux-status.md",
                include_str!("templates/commands/lux-status.md"),
            ),
        ];

        for (name, content) in &command_files {
            let path = commands_dir.join(name);
            std::fs::write(&path, content)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }

        eprintln!(
            "  → Installed {} OpenCode commands at {}",
            command_files.len(),
            commands_dir.display()
        );
    } else {
        eprintln!("  → Skipped .opencode/commands/ installation (use --opencode-commands to install).");
    }

    eprintln!("Bridge installed to {}", bridge_target.display());
    eprintln!("Open Unity Editor and wait for recompile. Menu 'AI Bridge' will appear.");
    Ok(())
}

fn install_uloop_package(project_root: &Path) -> anyhow::Result<()> {
    install_uloop_package_with_options(project_root, false, false, None)
}

fn install_uloop_package_with_options(
    project_root: &Path,
    local: bool,
    force: bool,
    version: Option<&str>,
) -> anyhow::Result<()> {
    eprintln!("📦 Installing uloop (unity-cli-loop)...");

    let pkg_json = project_root.join("package.json");
    let install_local = local || pkg_json.exists();
    let package = match version {
        Some(version) if !version.trim().is_empty() => format!("uloop-cli@{}", version.trim()),
        _ => "uloop-cli".to_string(),
    };

    let mut cmd = ProcessCommand::new("npm");
    cmd.arg("install");
    if install_local {
        cmd.arg(&package).arg("--save-dev");
        eprintln!("   Installing uloop-cli as devDependency in Unity project");
    } else {
        cmd.arg("-g").arg(&package);
        eprintln!("   Installing uloop-cli globally (-g)");
    }
    if force {
        cmd.arg("--force");
    }

    let output = cmd
        .current_dir(project_root)
        .output()
        .context("failed to install uloop-cli via npm")?;

    if !output.status.success() {
        bail!(
            "npm install uloop-cli failed (exit code {}):\n{}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let installed_version = read_installed_uloop_version(project_root, install_local)
        .unwrap_or_else(|_| "unknown".to_string());
    let install_state = serde_json::json!({
        "installed_at": chrono::Utc::now().to_rfc3339(),
        "version": installed_version,
        "install_scope": if install_local { "local" } else { "global" },
        "project_root": project_root.to_string_lossy(),
    });

    let state_path = project_root.join(".lux").join("uloop-install.json");
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&state_path, serde_json::to_vec_pretty(&install_state)?)
        .context("failed to write uloop install state")?;

    eprintln!(
        "✅ uloop (unity-cli-loop) v{} installed successfully",
        install_state
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    );
    Ok(())
}

fn read_installed_uloop_version(project_root: &Path, local: bool) -> anyhow::Result<String> {
    let mut cmd = ProcessCommand::new("npm");
    cmd.arg("list");
    if !local {
        cmd.arg("-g");
    }
    cmd.arg("uloop-cli").arg("--depth=0").arg("--json");

    let output = cmd
        .current_dir(project_root)
        .output()
        .context("failed to query installed uloop-cli version")?;
    if !output.status.success() {
        bail!(
            "npm list uloop-cli failed (exit code {}):\n{}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let value: Value = serde_json::from_slice(&output.stdout)
        .context("failed to parse npm list uloop-cli output")?;
    value
        .get("dependencies")
        .and_then(|dependencies| dependencies.get("uloop-cli"))
        .and_then(|dependency| dependency.get("version"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .context("npm list output did not include uloop-cli version")
}

fn connect_unity_tcp_with_retry(
    discovery: &UnityBridgeDiscovery,
    deadline: Instant,
) -> anyhow::Result<std::net::TcpStream> {
    loop {
        match std::net::TcpStream::connect((discovery.host.as_str(), discovery.port)) {
            Ok(stream) => return Ok(stream),
            Err(error) if is_transient_socket_error(&error) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to connect to Unity AI Bridge at {}:{}",
                        discovery.host, discovery.port
                    )
                });
            }
        }
    }
}

fn write_unity_tcp_with_retry(
    stream: &mut std::net::TcpStream,
    mut bytes: &[u8],
    deadline: Instant,
) -> anyhow::Result<()> {
    while !bytes.is_empty() {
        match stream.write(bytes) {
            Ok(0) => bail!("Unity TCP connection closed while writing request"),
            Ok(size) => bytes = &bytes[size..],
            Err(error) if is_transient_socket_error(&error) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) => return Err(error).context("Unity TCP request write failed"),
        }
    }

    stream.flush().context("Unity TCP request flush failed")
}

fn is_transient_socket_error(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::WouldBlock | ErrorKind::Interrupted | ErrorKind::TimedOut
    )
}

fn print_lux_unity_context(args: UnityContextArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    if args.refresh && refresh_lux_unity_context_via_bridge(&project_root)? {
        return Ok(());
    }
    if args.refresh {
        refresh_lux_unity_context(&project_root)?;
    }

    let context_path = project_root.join("UserSettings/LuxUnityContext.json");
    let context_text = fs::read_to_string(&context_path).with_context(|| {
        format!(
            "failed to read Lux Unity context at {}. Open Unity or run `lux unity context --refresh`.",
            context_path.display()
        )
    })?;
    let context_json: Value = serde_json::from_str(&context_text).with_context(|| {
        format!(
            "failed to parse Lux Unity context at {}",
            context_path.display()
        )
    })?;

    println!("{}", serde_json::to_string_pretty(&context_json)?);
    Ok(())
}

fn refresh_lux_unity_context_via_bridge(project_root: &Path) -> anyhow::Result<bool> {
    let Ok(discovery) = read_unity_bridge_discovery(project_root) else {
        return Ok(false);
    };
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "get_selected_file_context",
        "token": discovery.token,
        "params": {}
    });
    let response_json = match send_unity_bridge_request(&discovery, &request) {
        Ok(response) => response,
        Err(_) => return Ok(false),
    };
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        return Ok(false);
    }

    let selected_file_context = response_json
        .get("payload")
        .and_then(|payload| payload.get("selectedFileContext"))
        .context("Unity TCP response did not include payload.selectedFileContext")?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "protocol": "lux.unity.context.v1",
            "source": "get_selected_file_context",
            "generated_at_utc": response_json.get("capturedAtUtc").cloned().unwrap_or(Value::Null),
            "project_root": project_root,
            "selected_file_context": selected_file_context,
        }))?
    );
    Ok(true)
}

fn refresh_lux_unity_context(project_root: &Path) -> anyhow::Result<()> {
    let launch_target = resolve_unity_launch_target(project_root)?;
    let results_dir = project_root.join("TestResults");
    fs::create_dir_all(&results_dir)
        .with_context(|| format!("failed to create {}", results_dir.display()))?;
    let log_path = results_dir.join("LuxUnityContextRefresh.log");

    eprintln!(
        "Lux unity context: refreshing via Unity batch mode for {}",
        project_root.display()
    );

    let status = ProcessCommand::new(&launch_target.executable)
        .args(&launch_target.prefix_args)
        .arg("-batchmode")
        .arg("-quit")
        .arg("-nographics")
        .arg("-projectPath")
        .arg(project_root)
        .arg("-executeMethod")
        .arg("Linalab.Lux.Editor.LuxUnityContext.Refresh")
        .arg("-logFile")
        .arg(&log_path)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| {
            format!(
                "failed to launch Unity at {}",
                launch_target.executable.display()
            )
        })?;

    if !status.success() {
        bail!(
            "Lux Unity context refresh failed. See log: {}",
            log_path.display()
        );
    }

    Ok(())
}

fn print_lux_unity_status(args: UnityStatusArgs) -> anyhow::Result<()> {
    let project_root = match args.project_path {
        Some(path) => path,
        None => find_unity_project_root(std::env::current_dir()?)
            .context("Unity project not found. Use --project-path.")?,
    };
    let settings_path = project_root.join("UserSettings/LuxBridgeSettings.json");
    let settings_text = fs::read_to_string(&settings_path).with_context(|| {
        format!(
            "failed to read Lux bridge settings at {}. Open Unity and run Tools > Linalab > Lux > Unity Bridge > Write Lux Bridge Settings.",
            settings_path.display()
        )
    })?;
    let settings: LuxBridgeSettings = serde_json::from_str(&settings_text).with_context(|| {
        format!(
            "failed to parse Lux bridge settings at {}",
            settings_path.display()
        )
    })?;
    let _capabilities =
        lux_project::persist_engine_capabilities(&project_root, lux_project::EngineKind::Unity)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "ok": true,
            "schema_version": settings.schema_version,
            "protocol": settings.protocol,
            "package_name": settings.package_name,
            "package_version": settings.package_version,
            "project_root": settings.project_root,
            "rust_gateway_path": settings.rust_gateway_path,
            "gateway_url": settings.gateway_url,
            "gateway_reachable": is_gateway_url_reachable(settings.gateway_url.as_deref()),
            "unity_server_port": settings.unity_server_port,
            "generated_at_utc": settings.generated_at_utc,
            "settings_path": settings_path,
        }))?
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// lux compile — Unity batch mode via -executeMethod
// ---------------------------------------------------------------------------

fn run_batch_compile(args: CompileArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;

    let bridge_marker = project_root.join("Assets/Editor/LuxBridge/LuxBatchAutomation.cs");
    if !bridge_marker.exists() {
        eprintln!(
            "Bridge not installed, auto-installing to {}...",
            project_root.display()
        );
        install_bridge_files(BridgeInstallArgs {
            project_path: project_root.clone(),
            bridge_type: BridgeKind::Unity,
            opencode_commands: false,
            no_opencode_commands: true,
        })?;
    }

    if let Ok(discovery) = read_unity_bridge_discovery(&project_root) {
        let request = json!({
            "schemaVersion": 1,
            "requestId": uuid::Uuid::new_v4().to_string(),
            "command": "compile_lux_project",
            "token": discovery.token,
            "params": {}
        });
        match send_unity_tcp_line(
            &discovery,
            &format!("{}\n", serde_json::to_string(&request)?),
        ) {
            Ok(response) => {
                let response_json: Value = serde_json::from_str(&response)
                    .context("compile TCP response was not valid JSON")?;
                if response_json.get("errorCode").and_then(Value::as_str) == Some("unknown_command")
                {
                    eprintln!("compile_lux_project not available via TCP bridge, falling back to batch mode...");
                } else {
                    let compile_ok = response_json.get("ok").and_then(Value::as_bool) == Some(true);
                    if let Some(payload) = response_json
                        .get("payload")
                        .and_then(|payload| payload.get("compileResult"))
                    {
                        println!("{}", serde_json::to_string_pretty(payload)?);
                        if payload.get("ok").and_then(Value::as_bool) != Some(true) {
                            std::process::exit(1);
                        }
                    } else {
                        println!("{}", serde_json::to_string_pretty(&response_json)?);
                    }
                    if !compile_ok {
                        std::process::exit(1);
                    }
                    return Ok(());
                }
            }
            Err(error) => {
                eprintln!("Live Unity Editor compile failed to connect, falling back to batch mode: {error}");
            }
        }
    } else {
        eprintln!("No live Unity Editor detected, falling back to batch mode...");
    }

    let launch_target = resolve_unity_launch_target(&project_root)?;

    eprintln!(
        "Lux compile: launching Unity in batch mode for {}",
        project_root.display()
    );

    let results_dir = project_root.join("TestResults");
    fs::create_dir_all(&results_dir)
        .with_context(|| format!("failed to create {}", results_dir.display()))?;
    let log_path = results_dir.join("CompileLog.log");
    let compile_result_path = results_dir.join("CompileResult.json");
    if compile_result_path.exists() {
        fs::remove_file(&compile_result_path)
            .with_context(|| format!("failed to remove stale {}", compile_result_path.display()))?;
    }

    let status = ProcessCommand::new(&launch_target.executable)
        .args(&launch_target.prefix_args)
        .args([
            "-batchmode",
            "-quit",
            "-projectPath",
            project_root.to_str().unwrap(),
            "-executeMethod",
            "Linalab.Lux.Editor.LuxBatchAutomation.Compile",
            "-logFile",
            log_path.to_str().unwrap(),
        ])
        .status()
        .with_context(|| {
            format!(
                "failed to launch Unity at {}",
                launch_target.executable.display()
            )
        })?;

    if !status.success() {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": false,
                "message": format!("Unity batch compile exited with status: {status}"),
                "logPath": log_path.to_string_lossy(),
            }))?
        );
        std::process::exit(1);
    }

    if !compile_result_path.exists() {
        bail!(
            "Unity compile result not found at {}. Log: {}",
            compile_result_path.display(),
            log_path.display()
        );
    }

    let result_text = fs::read_to_string(&compile_result_path)
        .with_context(|| format!("failed to read {}", compile_result_path.display()))?;
    println!("{result_text}");
    let result_json: Value =
        serde_json::from_str(&result_text).context("compile result JSON invalid")?;
    if result_json.get("ok").and_then(Value::as_bool) != Some(true) {
        std::process::exit(1);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// lux run-tests — Unity batch mode via -runTests
// ---------------------------------------------------------------------------

fn run_batch_tests(args: RunTestsArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let platform = args.test_platform;
    let platform_label = match platform.as_str() {
        "EditMode" => "EditMode",
        "PlayMode" => "PlayMode",
        other => other,
    };

    let results_dir = project_root.join("TestResults");
    fs::create_dir_all(&results_dir)
        .with_context(|| format!("failed to create {}", results_dir.display()))?;

    let test_results = match &args.test_results {
        Some(p) => p.clone(),
        None => results_dir.join(format!("{}Results.xml", platform_label)),
    };
    let log_file = match &args.log_file {
        Some(p) => p.clone(),
        None => results_dir.join(format!("{}Log.log", platform_label)),
    };

    if let Ok(discovery) = read_unity_bridge_discovery(&project_root) {
        let test_code = format!(
            "var asm = System.AppDomain.CurrentDomain.GetAssemblies(); \
             System.Type apiType = null; System.Type filterType = null; \
             System.Type settingsType = null; System.Type testModeType = null; \
             foreach (var a in asm) {{ \
               var t1 = a.GetType(\"UnityEditor.TestTools.TestRunner.Api.TestRunnerApi\", false); \
               if (t1 != null && apiType == null) apiType = t1; \
               var t2 = a.GetType(\"UnityEditor.TestTools.TestRunner.Api.Filter\", false); \
               if (t2 != null && filterType == null) filterType = t2; \
               var t3 = a.GetType(\"UnityEditor.TestTools.TestRunner.Api.ExecutionSettings\", false); \
               if (t3 != null && settingsType == null) settingsType = t3; \
               var t4 = a.GetType(\"UnityEditor.TestTools.TestRunner.Api.TestMode\", false); \
               if (t4 != null && testModeType == null) testModeType = t4; \
             }} \
             if (apiType == null || filterType == null || settingsType == null || testModeType == null) \
               return \"MISSING:\" + (apiType != null) + \",\" + (filterType != null) + \",\" + (settingsType != null) + \",\" + (testModeType != null); \
             var filter = System.Activator.CreateInstance(filterType); \
             var testMode = System.Enum.Parse(testModeType, \"{platform_label}\"); \
             filterType.GetField(\"testMode\").SetValue(filter, testMode); \
             var filters = System.Array.CreateInstance(filterType, 1); \
             filters.SetValue(filter, 0); \
             var settings = System.Activator.CreateInstance(settingsType, new object[]{{ filters }}); \
             var api = UnityEngine.ScriptableObject.CreateInstance(apiType); \
             var executeMethod = apiType.GetMethod(\"Execute\", new[] {{ settingsType }}); \
             var result = executeMethod.Invoke(api, new object[]{{ settings }}); \
             return \"testId=\" + (result != null ? result.ToString() : \"null\");"
        );
        let request = json!({
            "schemaVersion": 1,
            "requestId": uuid::Uuid::new_v4().to_string(),
            "command": "execute_lux_dynamic_code",
            "token": discovery.token,
            "params": {
                "dynamicCode": test_code
            }
        });
        match send_unity_tcp_line_with_timeout(
            &discovery,
            &format!("{}\n", serde_json::to_string(&request)?),
            Duration::from_secs(120),
        ) {
            Ok(response) => {
                let response_json: Value = serde_json::from_str(&response)
                    .context("run_tests dynamic code TCP response was not valid JSON")?;
                if response_json.get("errorCode").and_then(Value::as_str) == Some("unknown_command")
                {
                    eprintln!("execute_lux_dynamic_code is not registered in Unity AI Bridge. Falling back to batch mode.");
                } else {
                    let bridge_ok = response_json.get("ok").and_then(Value::as_bool) == Some(true);
                    let dynamic_ok = response_json
                        .get("payload")
                        .and_then(|p| p.get("dynamicCodeResult"))
                        .and_then(|d| d.get("success"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let overall_ok = bridge_ok && dynamic_ok;

                    if let Some(dyn_result) = response_json
                        .get("payload")
                        .and_then(|p| p.get("dynamicCodeResult"))
                    {
                        println!("{}", serde_json::to_string_pretty(dyn_result)?);
                    } else {
                        println!("{}", serde_json::to_string_pretty(&response_json)?);
                    }
                    if !overall_ok {
                        let error_msg = response_json
                            .get("errorMessage")
                            .or_else(|| response_json.get("message"))
                            .and_then(Value::as_str)
                            .unwrap_or("unknown error");
                        eprintln!("Lux run-tests via dynamic code failed: {error_msg}");
                    } else {
                        return Ok(());
                    }
                }
            }
            Err(error) => {
                eprintln!("Live Unity Editor run-tests failed to connect, falling back to batch mode: {error}");
            }
        }
    } else {
        eprintln!("No live Unity Editor detected, falling back to batch mode...");
    }

    let launch_target = resolve_unity_launch_target(&project_root)?;

    eprintln!(
        "Lux run-tests: launching Unity in batch mode for {} ({})",
        project_root.display(),
        platform_label
    );

    let status = ProcessCommand::new(&launch_target.executable)
        .args(&launch_target.prefix_args)
        .arg("-runTests")
        .arg("-batchmode")
        .arg("-nographics")
        .arg("-projectPath")
        .arg(&project_root)
        .arg("-testPlatform")
        .arg(&platform)
        .arg("-testResults")
        .arg(&test_results)
        .arg("-logFile")
        .arg(&log_file)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| {
            format!(
                "failed to launch Unity at {}",
                launch_target.executable.display()
            )
        })?;

    println!(
        "{{ \"ok\": {}, \"test_platform\": \"{}\", \"results\": \"{}\", \"log\": \"{}\" }}",
        status.success(),
        platform_label,
        test_results.display(),
        log_file.display()
    );

    if !status.success() {
        eprintln!("Lux run-tests failed. See log: {}", log_file.display());
        std::process::exit(1);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn resolve_project_root(project_path: &Option<PathBuf>) -> anyhow::Result<PathBuf> {
    match project_path {
        Some(path) => Ok(cross_platform::normalize_path_buf(path.clone())),
        None => find_unity_project_root(std::env::current_dir()?)
            .context("Unity project not found. Use --project-path."),
    }
}

fn resolve_lux_project_root(project_path: &Option<PathBuf>) -> anyhow::Result<PathBuf> {
    match project_path {
        Some(path) => Ok(cross_platform::normalize_path_buf(path.clone())),
        None => {
            let cwd = cross_platform::normalize_path_buf(std::env::current_dir()?);
            find_lux_root_from(&cwd).ok_or_else(|| {
                anyhow::anyhow!(
                    "No .lux/ directory found in {} or any parent directory. Use --project-path.",
                    cwd.display()
                )
            })
        }
    }
}

fn find_lux_root_from(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".lux").is_dir() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn find_unity_project_root(mut current: PathBuf) -> Option<PathBuf> {
    loop {
        if is_unity_project(&current) {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn is_unity_project(path: &Path) -> bool {
    path.join("Assets").is_dir() && path.join("ProjectSettings").is_dir()
}

pub struct UnityLaunchTarget {
    pub executable: PathBuf,
    pub prefix_args: Vec<String>,
}

pub fn resolve_unity_launch_target(project_root: &Path) -> anyhow::Result<UnityLaunchTarget> {
    let config = load_active_config()?;
    if let Some(editor) = config.unity.editor_path.as_ref() {
        return Ok(UnityLaunchTarget {
            executable: editor.clone(),
            prefix_args: Vec::new(),
        });
    }

    if let Some(editor) = std::env::var_os("LUX_UNITY_EDITOR") {
        return Ok(UnityLaunchTarget {
            executable: PathBuf::from(editor),
            prefix_args: Vec::new(),
        });
    }

    let version = read_unity_editor_version(project_root)?;

    #[cfg(target_os = "macos")]
    {
        let hub_editor = PathBuf::from(format!(
            "/Applications/Unity/Hub/Editor/{version}/Unity.app/Contents/MacOS/Unity"
        ));
        if hub_editor.is_file() {
            return Ok(UnityLaunchTarget {
                executable: hub_editor,
                prefix_args: Vec::new(),
            });
        }
    }

    #[cfg(target_os = "windows")]
    {
        let mut candidates = Vec::new();

        if let Some(hub_path) = config.unity.hub_path.as_ref() {
            candidates.push(
                unity_hub::editor_install_path_for_hub(hub_path)
                    .join(&version)
                    .join("Editor")
                    .join("Unity.exe"),
            );
        }

        if let Some(install_path) = config.unity.custom_install_path.as_ref() {
            candidates.push(install_path.join(&version).join("Editor").join("Unity.exe"));
        }

        if let Some(hub_path) = std::env::var_os("LUX_UNITY_HUB_PATH") {
            candidates.push(
                PathBuf::from(hub_path)
                    .join("Editor")
                    .join(&version)
                    .join("Editor")
                    .join("Unity.exe"),
            );
        }

        candidates.push(PathBuf::from(format!(
            "C:\\Program Files\\Unity\\Hub\\Editor\\{version}\\Editor\\Unity.exe"
        )));
        candidates.push(PathBuf::from(format!(
            "C:\\Program Files\\Unity Hub\\Editor\\{version}\\Editor\\Unity.exe"
        )));

        for hub_editor in candidates {
            if hub_editor.is_file() {
                return Ok(UnityLaunchTarget {
                    executable: hub_editor,
                    prefix_args: Vec::new(),
                });
            }
        }

        use winreg::{enums::HKEY_CURRENT_USER, RegKey};

        let current_user = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(unity_editor_key) =
            current_user.open_subkey("Software\\Unity Technologies\\Unity Editor 5.x")
        {
            let value_name = format!("{version}_Location_x64");
            if let Ok(editor_path) = unity_editor_key.get_value::<String, _>(&value_name) {
                let editor_path = PathBuf::from(editor_path);
                if editor_path.is_file() {
                    return Ok(UnityLaunchTarget {
                        executable: editor_path,
                        prefix_args: Vec::new(),
                    });
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let hub_editor = PathBuf::from(format!("/opt/Unity/Hub/Editor/{version}/Editor/Unity"));
        if hub_editor.is_file() {
            return Ok(UnityLaunchTarget {
                executable: hub_editor,
                prefix_args: Vec::new(),
            });
        }

        if let Some(home) = std::env::var_os("HOME") {
            let home_editor =
                PathBuf::from(home).join(format!("Unity/Hub/Editor/{version}/Editor/Unity"));
            if home_editor.is_file() {
                return Ok(UnityLaunchTarget {
                    executable: home_editor,
                    prefix_args: Vec::new(),
                });
            }
        }
    }

    bail!(
        "Unity Editor {version} not found in standard Hub locations. \
         Set LUX_UNITY_EDITOR to the Unity executable path."
    )
}

pub fn read_unity_editor_version(project_root: &Path) -> anyhow::Result<String> {
    let version_path = project_root
        .join("ProjectSettings")
        .join("ProjectVersion.txt");
    let text = fs::read_to_string(&version_path)
        .with_context(|| format!("failed to read {}", version_path.display()))?;
    text.lines()
        .find_map(|line| line.strip_prefix("m_EditorVersion:"))
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(ToOwned::to_owned)
        .context("ProjectSettings/ProjectVersion.txt did not contain m_EditorVersion")
}

// ---------------------------------------------------------------------------
// lux serve — WebSocket gateway
// ---------------------------------------------------------------------------

async fn serve(args: ServeArgs, config: &config::LuxConfig) -> anyhow::Result<()> {
    let host = args.host.unwrap_or(
        config
            .server
            .host
            .parse()
            .context("server.host must be an IP address")?,
    );
    let port = args.port.unwrap_or(config.server.port);
    let token = args
        .token
        .or_else(|| config.server.token.clone())
        .or_else(|| std::env::var("LUX_TOKEN").ok())
        .unwrap_or_default();
    let addr = SocketAddr::new(host, port);
    let project_root = args
        .project_path
        .or_else(|| config.general.project_root.clone())
        .map(|path| {
            let normalized = cross_platform::normalize_path_buf(path);
            normalized.canonicalize().with_context(|| {
                format!(
                    "failed to canonicalize project path {}",
                    normalized.display()
                )
            })
        })
        .transpose()?;
    let state = server::GatewayState::new(server::GatewayConfig {
        token,
        history_capacity: args.history_capacity,
        project_root,
        addon_auth: crate::addon_auth::AddonAuthConfig {
            github_client_id: std::env::var("LUX_GITHUB_CLIENT_ID")
                .unwrap_or_else(|_| "placeholder_client_id".to_string()),
            github_client_secret: std::env::var("LUX_GITHUB_CLIENT_SECRET").ok(),
        },
    });
    let idle_timeout = args
        .idle_timeout
        .and_then(|minutes| minutes.checked_mul(60))
        .unwrap_or(config.server.idle_timeout_secs);
    let idle_timeout = Some(Duration::from_secs(idle_timeout));
    let app = server::router(state.clone());
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind Lux gateway at {addr}"))?;

    tracing::info!(%addr, "Lux gateway listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(state, idle_timeout))
        .await
        .context("Lux gateway stopped with an error")
}

async fn shutdown_signal(state: server::GatewayState, idle_timeout: Option<Duration>) {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!(%error, "failed to listen for shutdown signal");
        }
    };

    if let Some(timeout) = idle_timeout.filter(|duration| !duration.is_zero()) {
        tokio::select! {
            _ = ctrl_c => {}
            _ = state.wait_for_idle_timeout(timeout) => {
                eprintln!(
                    "Lux gateway graceful shutdown: idle timeout reached after {} minutes without activity",
                    timeout.as_secs() / 60
                );
            }
        }
    } else {
        ctrl_c.await;
    }
}

fn run_install_command(args: InstallArgs) -> anyhow::Result<()> {
    let project_path = resolve_project_root(&args.project)?;
    if !is_unity_project(&project_path) {
        bail!("target is not a Unity project: missing Assets/ or ProjectSettings/");
    }

    let package_name = &args.name;
    if !package_name.starts_with("com.linalab.") {
        bail!("package name must follow com.linalab.<name> convention");
    }

    let repo_url = format!("https://github.com/linalab/{}", package_name);
    let packages_dir = project_path.join("Packages");
    let package_dir = packages_dir.join(package_name);

    if package_dir.exists() {
        if args.json {
            println!(
                "{{\"ok\": true, \"message\": \"package already installed\", \"path\": \"{}\"}}",
                package_dir.display()
            );
        } else {
            println!(
                "Package {} is already installed at {}",
                package_name,
                package_dir.display()
            );
        }
        return Ok(());
    }

    let manifest_path = packages_dir.join("manifest.json");
    if manifest_path.exists() {
        let content = fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?;
        let mut manifest: serde_json::Value =
            serde_json::from_str(&content).with_context(|| "failed to parse manifest.json")?;

        if let Some(deps) = manifest
            .get_mut("dependencies")
            .and_then(|d| d.as_object_mut())
        {
            if deps.contains_key(package_name) {
                if args.json {
                    println!("{{\"ok\": true, \"message\": \"package already in manifest\"}}");
                } else {
                    println!("Package {} already listed in manifest.json", package_name);
                }
                return Ok(());
            }

            deps.insert(
                package_name.clone(),
                serde_json::Value::String(format!("git+{}", repo_url)),
            );

            let output = serde_json::to_string_pretty(&manifest);
            match output {
                Ok(json) => {
                    fs::write(&manifest_path, json)?;
                }
                Err(e) => {
                    bail!("failed to serialize manifest: {}", e);
                }
            }
        } else {
            bail!("manifest.json has no dependencies object");
        }
    } else {
        bail!(
            "Packages/manifest.json not found at {}",
            manifest_path.display()
        );
    }

    if args.json {
        println!(
            "{{\"ok\": true, \"package\": \"{}\", \"repo\": \"{}\"}}",
            package_name, repo_url
        );
    } else {
        println!("Added {} to project (source: {})", package_name, repo_url);
        println!("Unity will resolve the package on next refresh.");
    }
    Ok(())
}

fn run_addon_command(args: AddonArgs) -> anyhow::Result<()> {
    match args.action {
        AddonAction::List(a) => run_addon_list(a),
        AddonAction::Auth(a) => run_addon_auth(a),
    }
}

fn run_addon_list(args: AddonListArgs) -> anyhow::Result<()> {
    let known = crate::addon_store::KNOWN_LINALAB_PACKAGES;
    let packages: Vec<&str> = if args.public {
        known.to_vec()
    } else {
        known.to_vec()
    };

    if args.json {
        let list: Vec<serde_json::Value> = packages
            .iter()
            .map(|name| {
                json!({
                    "name": name,
                    "repo": format!("https://github.com/linalab/{}", name),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&list)?);
    } else {
        if args.public {
            println!("Public linalab packages:");
        } else {
            println!("Registered linalab packages:");
        }
        for name in &packages {
            println!("  {} (https://github.com/linalab/{})", name, name);
        }
    }
    Ok(())
}

fn run_addon_auth(args: AddonAuthArgs) -> anyhow::Result<()> {
    if args.status {
        println!("Auth status: not authenticated");
        println!("Run 'lux addon auth' to start GitHub Device Flow authentication.");
        return Ok(());
    }

    let client_id = std::env::var("LUX_GITHUB_CLIENT_ID")
        .map_err(|_| anyhow::anyhow!("LUX_GITHUB_CLIENT_ID not set"))?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let response = addon_auth::start_device_flow(&client_id).await?;
        println!("To authenticate, visit: {}", response.verification_uri);
        println!("Enter code: {}", response.user_code);
        println!("Waiting for authorization...");

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(response.interval)).await;
            match addon_auth::poll_device_token(&client_id, &response.device_code).await {
                Ok(Some(token)) => {
                    println!("Authentication successful!");
                    let repos = addon_auth::check_repo_access(&token.access_token).await?;
                    if repos.is_empty() {
                        println!("No linalab packages accessible.");
                    } else {
                        println!("Accessible repos:");
                        for repo in &repos {
                            println!("  {}", repo);
                        }
                    }
                    break;
                }
                Ok(None) => continue,
                Err(e) => {
                    bail!("Authentication failed: {}", e);
                }
            }
        }
        Ok(())
    })
}
