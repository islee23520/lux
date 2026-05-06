pub mod addon_auth;
pub mod addon_routes;
pub mod addon_store;
pub mod unity_launch;
pub mod ai_log;
pub mod cross_platform;
mod mcp;
mod protocol;
mod server;
pub mod session;
pub mod visual_regression;

use std::{
    fs,
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, shells::Shell};
use protocol::EventEnvelope;
use serde_json::{json, Value};

#[derive(Parser, Debug)]
#[command(name = "lux")]
#[command(version)]
#[command(about = "Lux CLI — Unity batch mode automation for Neon Glitch")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Serve(ServeArgs),
    Unity(UnityArgs),
    Skill(SkillArgs),
    AiLog(AiLogArgs),
    Mcp(McpArgs),
    Compile(CompileArgs),
    Bridge(BridgeArgs),
    RunTests(RunTestsArgs),
    Screenshot(ScreenshotArgs),
    Session(SessionArgs),
    Install(InstallArgs),
    Addon(AddonArgs),
    Schema,
    /// Generate shell completion scripts
    Completion {
        /// Shell type to generate completions for
        #[arg(long, value_enum)]
        shell: Option<Shell>,
    },
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
    /// Install to project scope (.lux/skills/) instead of global
    #[arg(short, long)]
    project: bool,
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
}

#[derive(Parser, Debug)]
struct SkillUpdateArgs {
    /// Skill name to update
    name: String,
}

#[derive(Parser, Debug)]
struct AiLogArgs {
    #[command(subcommand)]
    action: AiLogAction,
}

#[derive(Parser, Debug)]
struct McpArgs {
    #[command(subcommand)]
    action: McpAction,
}

#[derive(Subcommand, Debug)]
enum McpAction {
    /// Run the LUX MCP server over newline-delimited JSON-RPC stdio
    Serve,
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
    DragStart,
    DragMove,
    DragEnd,
}

impl MouseUiAction {
    fn as_str(self) -> &'static str {
        match self {
            MouseUiAction::Click => "click",
            MouseUiAction::LongPress => "long-press",
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
    #[arg(long, env = "LUX_GATEWAY_HOST", default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
    host: IpAddr,
    #[arg(long, env = "LUX_GATEWAY_PORT", default_value_t = 17340)]
    port: u16,
    #[arg(long, env = "LUX_GATEWAY_TOKEN")]
    token: String,
    #[arg(long, env = "LUX_GATEWAY_HISTORY", default_value_t = 256)]
    history_capacity: usize,
    /// Minutes without HTTP or WebSocket activity before graceful shutdown (0 disables)
    #[arg(long, env = "LUX_GATEWAY_IDLE_TIMEOUT", default_value_t = 30)]
    idle_timeout: u64,
    /// Unity project root used for project-bound gateway APIs
    #[arg(long, env = "LUX_PROJECT_PATH")]
    project_path: Option<PathBuf>,
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
}

#[derive(Parser, Debug)]
struct BridgeWatchArgs {
    #[arg(long)]
    project_path: Option<PathBuf>,
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
    unity_server_port: Option<u16>,
    generated_at_utc: String,
}

#[derive(Debug, serde::Deserialize)]
struct UnityBridgeDiscovery {
    host: String,
    port: u16,
    token: String,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    match Cli::parse().command {
        Command::Serve(args) => serve(args).await,
        Command::Unity(args) => run_lux_unity_command(args),
        Command::Skill(args) => run_skill_command(args),
        Command::AiLog(args) => run_ai_log_command(args),
        Command::Mcp(args) => run_mcp_command(args),
        Command::Compile(args) => run_batch_compile(args),
        Command::Bridge(args) => run_bridge_command(args),
        Command::RunTests(args) => run_batch_tests(args),
        Command::Screenshot(args) => run_screenshot_command(args),
        Command::Session(args) => run_session_command(args),
        Command::Install(args) => run_install_command(args),
        Command::Addon(args) => run_addon_command(args),
        Command::Schema => {
            println!(
                "{}",
                serde_json::to_string_pretty(&EventEnvelope::schema_example())?
            );
            Ok(())
        }
        Command::Completion { shell } => {
            let shell = shell.unwrap_or_else(|| {
                if std::env::var_os("SHELL")
                    .map(|s| s.to_string_lossy().contains("zsh"))
                    .unwrap_or(false)
                {
                    Shell::Zsh
                } else if std::env::var_os("PSModulePath").is_some() {
                    Shell::PowerShell
                } else {
                    Shell::Bash
                }
            });
            let mut cmd = Cli::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut std::io::stdout());
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// lux mcp
// ---------------------------------------------------------------------------

fn run_mcp_command(args: McpArgs) -> anyhow::Result<()> {
    match args.action {
        McpAction::Serve => mcp::run_stdio_server(mcp::create_lux_mcp_server()),
    }
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct SkillManifest {
    name: String,
    version: String,
    description: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    #[serde(rename = "luxVersion")]
    lux_version: Option<String>,
    author: Option<SkillAuthor>,
    keywords: Option<Vec<String>>,
    #[serde(rename = "type")]
    skill_type: String,
    source: Option<String>,
    dependencies: Option<Value>,
    #[serde(default, rename = "requiredPackages")]
    required_packages: Option<Vec<String>>,
    #[serde(default, rename = "compatibleRenderPipelines")]
    compatible_render_pipelines: Option<Vec<String>>,
    #[serde(default, rename = "contextSlimRules")]
    context_slim_rules: Option<SkillContextSlimRules>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct SkillAuthor {
    name: String,
    email: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
struct SkillContextSlimRules {
    #[serde(default, rename = "maxReferences")]
    max_references: Option<usize>,
    #[serde(default, rename = "maxSkillMdLines")]
    max_skill_md_lines: Option<usize>,
    #[serde(default, rename = "excludeTags")]
    exclude_tags: Option<Vec<String>>,
}

#[derive(Debug, serde::Serialize)]
struct SkillEntry {
    manifest: SkillManifest,
    directory_path: PathBuf,
    scope: String,
}

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
    let entries = discover_skills()?;

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
    let entries = discover_skills()?;
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
    if args.adapt && !args.project {
        fail_skill_install(args.json, "--adapt requires --project");
    }
    if discover_skills()?
        .iter()
        .any(|entry| entry.scope == "core" && entry.manifest.name == args.name)
    {
        fail_skill_install(
            args.json,
            &format!("refusing to overwrite core skill '{}'", args.name),
        );
    }

    let target_root = if args.project {
        project_skills_dir().context("failed to determine project skills directory")?
    } else {
        global_skills_dir().context("failed to determine global skills directory")?
    };
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
                "scope": if args.project { "project" } else { "global" },
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
    if args.project && args.global {
        eprintln!("Error: choose either --project or --global, not both");
        std::process::exit(1);
    }

    if discover_skills()?
        .iter()
        .any(|entry| entry.scope == "core" && entry.manifest.name == args.name)
    {
        eprintln!("Error: refusing to remove core skill '{}'", args.name);
        std::process::exit(1);
    }

    let target_dir = if args.project {
        project_skills_dir()
            .context("failed to determine project skills directory")?
            .join(&args.name)
    } else if args.global {
        global_skills_dir()
            .context("failed to determine global skills directory")?
            .join(&args.name)
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
    let entries = discover_skills()?;
    let Some(entry) = find_skill_for_update(&entries, &args.name) else {
        eprintln!("Error: skill '{}' not found", args.name);
        std::process::exit(1);
    };

    let Some(source) = entry.manifest.source.as_deref() else {
        eprintln!("Error: Skill has no source URL configured");
        std::process::exit(1);
    };

    install_skill_from_source(source, &entry.directory_path)?;
    println!(
        "Updated skill '{}' at {}",
        args.name,
        entry.directory_path.display()
    );
    Ok(())
}

fn find_skill_for_update<'a>(entries: &'a [SkillEntry], name: &str) -> Option<&'a SkillEntry> {
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
    let source_dir = Path::new(source);
    if !source_dir.is_dir() {
        bail!("source is not a directory: {}", source_dir.display());
    }

    let manifest_path = source_dir.join("manifest.json");
    let skill_md_path = source_dir.join("SKILL.md");
    if !manifest_path.is_file() {
        bail!("source skill is missing manifest.json");
    }
    if !skill_md_path.is_file() {
        bail!("source skill is missing SKILL.md");
    }

    let manifest: SkillManifest = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?,
    )
    .with_context(|| format!("failed to parse {}", manifest_path.display()))?;
    if manifest.name != name {
        bail!(
            "source manifest name '{}' does not match requested skill '{}'",
            manifest.name,
            name
        );
    }

    // -----------------------------------------------------------------------
    // Phase 1: Basic structural checks
    // -----------------------------------------------------------------------
    let mut checks = Vec::new();
    let assets_dir = project_root.join("Assets");
    let project_settings_dir = project_root.join("ProjectSettings");
    let unity_project_ok = assets_dir.is_dir() && project_settings_dir.is_dir();
    checks.push(json!({
        "name": "unity_project",
        "ok": unity_project_ok,
        "message": "Project root contains Assets/ and ProjectSettings/",
    }));
    checks.push(json!({
        "name": "source_manifest",
        "ok": true,
        "message": "Source contains manifest.json",
    }));
    checks.push(json!({
        "name": "source_skill_md",
        "ok": true,
        "message": "Source contains SKILL.md",
    }));
    checks.push(json!({
        "name": "safe_name",
        "ok": true,
        "message": "Skill name is safe for project installation",
    }));
    checks.push(json!({
        "name": "core_overwrite",
        "ok": true,
        "message": "Requested skill does not overwrite a core skill",
    }));

    if !unity_project_ok {
        bail!("--adapt requires a Unity project root containing Assets/ and ProjectSettings/");
    }

    let mut warnings = Vec::new();
    if manifest.lux_version.is_none() {
        warnings.push("source manifest does not declare luxVersion".to_string());
    }

    // -----------------------------------------------------------------------
    // Phase 2: Project metadata detection
    // -----------------------------------------------------------------------
    let project_metadata = detect_project_metadata(&project_root, &mut warnings);

    // -----------------------------------------------------------------------
    // Phase 3: Compatibility judgment
    // -----------------------------------------------------------------------
    let compatibility = judge_skill_compatibility(&manifest, &project_metadata, &mut checks);
    if !compatibility.compatible {
        bail!(
            "skill '{}' is incompatible with this project: {}",
            name,
            compatibility.reasons.join(", ")
        );
    }

    // -----------------------------------------------------------------------
    // Phase 4: Context slimming
    // -----------------------------------------------------------------------
    let context_slimming = compute_context_slimming(source_dir, &manifest, &project_metadata);

    Ok(json!({
        "schema_version": 1,
        "protocol": "lux.skill.adaptation.v1",
        "skill_name": name,
        "source": source,
        "project_root": project_root,
        "checks": checks,
        "warnings": warnings,
        "project_metadata": project_metadata.to_json(),
        "compatibility": {
            "compatible": compatibility.compatible,
            "reasons": compatibility.reasons,
        },
        "context_slimming": context_slimming,
    }))
}

struct ProjectMetadata {
    unity_version: Option<String>,
    render_pipeline: String,
    installed_packages: Vec<String>,
    has_lux_package: bool,
}

impl ProjectMetadata {
    fn to_json(&self) -> Value {
        json!({
            "unity_version": self.unity_version,
            "render_pipeline": self.render_pipeline,
            "installed_packages": self.installed_packages,
            "has_lux_package": self.has_lux_package,
        })
    }
}

struct CompatibilityResult {
    compatible: bool,
    reasons: Vec<String>,
}

fn detect_project_metadata(project_root: &Path, warnings: &mut Vec<String>) -> ProjectMetadata {
    let mut unity_version: Option<String> = None;
    let mut render_pipeline = "unknown".to_string();
    let mut installed_packages: Vec<String> = Vec::new();
    let mut has_lux_package = false;

    // Read Unity version from ProjectSettings/ProjectVersion.txt
    let version_path = project_root
        .join("ProjectSettings")
        .join("ProjectVersion.txt");
    if let Ok(version_content) = fs::read_to_string(&version_path) {
        for line in version_content.lines() {
            if let Some(version) = line.strip_prefix("m_EditorVersion: ") {
                unity_version = Some(version.trim().to_string());
                break;
            }
        }
    }

    // Read Packages/manifest.json for installed packages and render pipeline hints
    let package_manifest_path = project_root.join("Packages").join("manifest.json");
    match fs::read_to_string(&package_manifest_path) {
        Ok(package_manifest_json) => {
            has_lux_package = package_manifest_json.contains("com.linalab.lux");

            if let Ok(package_value) = serde_json::from_str::<Value>(&package_manifest_json) {
                if let Some(deps) = package_value.get("dependencies").and_then(Value::as_object) {
                    for key in deps.keys() {
                        installed_packages.push(key.clone());
                    }
                }

                // Detect render pipeline from packages
                if installed_packages.contains(&"com.unity.render-pipelines.universal".to_string())
                {
                    render_pipeline = "urp".to_string();
                } else if installed_packages
                    .contains(&"com.unity.render-pipelines.high-definition".to_string())
                {
                    render_pipeline = "hdrp".to_string();
                } else {
                    render_pipeline = "builtin".to_string();
                }
            }

            if !has_lux_package {
                warnings.push(
                    "project Packages/manifest.json does not mention com.linalab.lux".to_string(),
                );
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => warnings.push(
            "project Packages/manifest.json was not found; skipped LUX package metadata check"
                .to_string(),
        ),
        Err(error) => warnings.push(format!(
            "failed to read {}: {error}",
            package_manifest_path.display()
        )),
    }

    // Check for GraphicsSettings asset for render pipeline override
    let graphics_settings = project_root
        .join("ProjectSettings")
        .join("GraphicsSettings.asset");
    if graphics_settings.exists() && render_pipeline == "unknown" {
        if let Ok(content) = fs::read_to_string(&graphics_settings) {
            if content.contains("UniversalRenderPipelineAsset") {
                render_pipeline = "urp".to_string();
            } else if content.contains("HDRenderPipelineAsset") {
                render_pipeline = "hdrp".to_string();
            } else {
                render_pipeline = "builtin".to_string();
            }
        }
    }

    ProjectMetadata {
        unity_version,
        render_pipeline,
        installed_packages,
        has_lux_package,
    }
}

fn judge_skill_compatibility(
    manifest: &SkillManifest,
    project_metadata: &ProjectMetadata,
    checks: &mut Vec<Value>,
) -> CompatibilityResult {
    let mut reasons: Vec<String> = Vec::new();

    // Check luxVersion compatibility
    if let Some(ref lux_version) = manifest.lux_version {
        let version_ok =
            lux_version.starts_with(">=") || lux_version.starts_with('^') || lux_version == "*";
        checks.push(json!({
            "name": "lux_version_declared",
            "ok": true,
            "message": format!("Skill declares luxVersion: {}", lux_version),
        }));
        if !version_ok && !lux_version.is_empty() {
            // Specific version constraint — we just note it as a warning, not a blocker,
            // since we cannot resolve semver for the running LUX binary here.
            checks.push(json!({
                "name": "lux_version_check",
                "ok": true,
                "message": format!("luxVersion constraint '{}' noted (skipping semver resolution)", lux_version),
            }));
        }
    }

    // Check required packages
    let mut missing_packages: Vec<String> = Vec::new();
    if let Some(ref required) = manifest.required_packages {
        for package in required {
            if !project_metadata.installed_packages.contains(package) {
                missing_packages.push(package.clone());
            }
        }
        let packages_ok = missing_packages.is_empty();
        checks.push(json!({
            "name": "required_packages",
            "ok": packages_ok,
            "message": if packages_ok {
                "All required packages are installed".to_string()
            } else {
                format!("Missing required packages: {}", missing_packages.join(", "))
            },
        }));
        if !packages_ok {
            reasons.push(format!(
                "missing required packages: {}",
                missing_packages.join(", ")
            ));
        }
    }

    // Check render pipeline compatibility
    if let Some(ref compatible_pipelines) = manifest.compatible_render_pipelines {
        let pipeline_ok = compatible_pipelines.is_empty()
            || compatible_pipelines.contains(&project_metadata.render_pipeline);
        checks.push(json!({
            "name": "render_pipeline",
            "ok": pipeline_ok,
            "message": format!(
                "Project render pipeline '{}' vs skill compatibility: [{}]",
                project_metadata.render_pipeline,
                compatible_pipelines.join(", ")
            ),
        }));
        if !pipeline_ok {
            reasons.push(format!(
                "incompatible render pipeline: project uses '{}', skill requires one of [{}]",
                project_metadata.render_pipeline,
                compatible_pipelines.join(", ")
            ));
        }
    }

    CompatibilityResult {
        compatible: reasons.is_empty(),
        reasons,
    }
}

fn compute_context_slimming(
    source_dir: &Path,
    manifest: &SkillManifest,
    project_metadata: &ProjectMetadata,
) -> Value {
    let slim_rules = manifest.context_slim_rules.as_ref();

    // Count total references
    let references_dir = source_dir.join("references");
    let total_references = if references_dir.is_dir() {
        fs::read_dir(&references_dir)
            .map(|entries| entries.flatten().count())
            .unwrap_or(0)
    } else {
        0
    };

    // Count SKILL.md lines
    let skill_md_path = source_dir.join("SKILL.md");
    let total_skill_md_lines = fs::read_to_string(&skill_md_path)
        .map(|content| content.lines().count())
        .unwrap_or(0);

    // Apply max_references slimming
    let max_references = slim_rules
        .and_then(|rules| rules.max_references)
        .unwrap_or(usize::MAX);
    let _slimmed_references = total_references.min(max_references);
    let references_slashed = total_references > max_references;

    // Apply max_skill_md_lines slimming
    let max_skill_md_lines = slim_rules
        .and_then(|rules| rules.max_skill_md_lines)
        .unwrap_or(usize::MAX);
    let slimmed_skill_md_lines = total_skill_md_lines.min(max_skill_md_lines);
    let skill_md_slashed = total_skill_md_lines > max_skill_md_lines;

    // Apply exclude_tags filtering
    let excluded_tags: Vec<String> = slim_rules
        .and_then(|rules| rules.exclude_tags.clone())
        .unwrap_or_default();

    // Build tag-relevance filter: references whose filenames match project package or pipeline
    let mut included_references: Vec<String> = Vec::new();
    let mut excluded_references: Vec<String> = Vec::new();
    if references_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&references_dir) {
            let mut all_refs: Vec<String> = entries
                .flatten()
                .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
                .filter(|name| name.ends_with(".md"))
                .collect();
            all_refs.sort();

            for reference_name in &all_refs {
                let name_lower = reference_name.to_lowercase();
                let should_exclude = excluded_tags
                    .iter()
                    .any(|tag| name_lower.contains(&tag.to_lowercase()));
                if should_exclude {
                    excluded_references.push(reference_name.clone());
                } else {
                    included_references.push(reference_name.clone());
                }
            }
        }
    }

    // Respect max_references limit after tag filtering
    if included_references.len() > max_references {
        let excess = included_references.len() - max_references;
        let drained: Vec<String> =
            included_references.split_off(included_references.len() - excess);
        excluded_references.extend(drained);
    }

    let slimmed_references_actual = included_references.len();

    json!({
        "total_references": total_references,
        "slimmed_references": slimmed_references_actual,
        "references_slashed": references_slashed || slimmed_references_actual != total_references,
        "total_skill_md_lines": total_skill_md_lines,
        "slimmed_skill_md_lines": slimmed_skill_md_lines,
        "skill_md_slashed": skill_md_slashed,
        "excluded_tags": excluded_tags,
        "included_references": included_references,
        "excluded_references": excluded_references,
        "project_render_pipeline": project_metadata.render_pipeline,
    })
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

fn discover_skills() -> anyhow::Result<Vec<SkillEntry>> {
    let mut entries = Vec::new();

    scan_skill_scope(&core_skills_dir(), "core", &mut entries)?;
    if let Some(skills_dir) = project_skills_dir() {
        scan_skill_scope(&skills_dir, "project", &mut entries)?;
    }
    if let Some(skills_dir) = global_skills_dir() {
        scan_skill_scope(&skills_dir, "global", &mut entries)?;
    }

    entries.sort_by(|left, right| {
        left.manifest
            .name
            .cmp(&right.manifest.name)
            .then_with(|| left.scope.cmp(&right.scope))
    });
    Ok(entries)
}

fn scan_skill_scope(
    skills_dir: &Path,
    scope: &str,
    entries: &mut Vec<SkillEntry>,
) -> anyhow::Result<()> {
    let read_dir = match fs::read_dir(&skills_dir) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("failed to read skills directory {}", skills_dir.display())
            })
        }
    };

    for dir_entry in read_dir {
        let dir_entry = match dir_entry {
            Ok(dir_entry) => dir_entry,
            Err(error) => {
                eprintln!("Warning: failed to read skill directory entry: {error}");
                continue;
            }
        };
        let directory_path = dir_entry.path();
        if !directory_path.is_dir() {
            continue;
        }

        let manifest_path = directory_path.join("manifest.json");
        let manifest_json = match fs::read_to_string(&manifest_path) {
            Ok(manifest_json) => manifest_json,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                eprintln!(
                    "Warning: missing manifest.json for skill directory {}",
                    directory_path.display()
                );
                continue;
            }
            Err(error) => {
                eprintln!(
                    "Warning: failed to read {}: {error}",
                    manifest_path.display()
                );
                continue;
            }
        };

        let manifest = match serde_json::from_str::<SkillManifest>(&manifest_json) {
            Ok(manifest) => manifest,
            Err(error) => {
                eprintln!(
                    "Warning: failed to parse {}: {error}",
                    manifest_path.display()
                );
                continue;
            }
        };

        entries.push(SkillEntry {
            manifest,
            directory_path,
            scope: scope.to_string(),
        });
    }

    Ok(())
}

fn core_skills_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../Skills")
}

fn project_skills_dir() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .map(|d| d.join(".lux").join("skills"))
}

fn global_skills_dir() -> Option<PathBuf> {
    let home = if cfg!(windows) {
        std::env::var("USERPROFILE").ok()
    } else {
        std::env::var("HOME").ok()
    };
    home.map(|h| PathBuf::from(h).join(".lux").join("skills"))
}

fn read_skill_references(directory_path: &Path) -> Vec<String> {
    let references_dir = directory_path.join("references");
    let read_dir = match fs::read_dir(&references_dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return Vec::new(),
    };

    let mut references = Vec::new();
    for dir_entry in read_dir.flatten() {
        let path = dir_entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        if let Some(file_name) = path.file_name().and_then(|file_name| file_name.to_str()) {
            references.push(file_name.to_string());
        }
    }
    references.sort();
    references
}

fn read_skill_md_preview(directory_path: &Path) -> Vec<String> {
    let skill_md_path = directory_path.join("SKILL.md");
    let content = match fs::read_to_string(&skill_md_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    content.lines().take(10).map(str::to_string).collect()
}

fn read_skill_adaptation_metadata(directory_path: &Path) -> Option<Value> {
    let adaptation_path = directory_path.join("lux-adaptation.json");
    let content = fs::read_to_string(&adaptation_path).ok()?;
    match serde_json::from_str::<Value>(&content) {
        Ok(value) => Some(value),
        Err(error) => {
            eprintln!(
                "Warning: failed to parse adaptation metadata {}: {error}",
                adaptation_path.display()
            );
            None
        }
    }
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
        UnityCommand::GetLogs(get_logs_args) => print_lux_backend_console_logs(get_logs_args),
        UnityCommand::ClearConsole(clear_console_args) => {
            clear_lux_backend_clear_console(clear_console_args)
        }
        UnityCommand::FocusWindow(focus_window_args) => {
            print_lux_backend_focus_window(focus_window_args)
        }
        UnityCommand::Launch(launch_args) => run_lux_unity_launch(launch_args),
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
            print_lux_backend_control_play_mode(control_play_mode_args)
        }
        UnityCommand::Screenshot(screenshot_args) => print_lux_backend_screenshot(screenshot_args),
        UnityCommand::SimulateMouseUi(simulate_mouse_ui_args) => {
            print_lux_backend_simulate_mouse_ui(simulate_mouse_ui_args)
        }
        UnityCommand::SimulateKeyboard(simulate_keyboard_args) => {
            print_lux_backend_simulate_keyboard(simulate_keyboard_args)
        }
        UnityCommand::SimulateMouseInput(simulate_mouse_input_args) => {
            print_lux_backend_simulate_mouse_input(simulate_mouse_input_args)
        }
        UnityCommand::RecordInput(record_input_args) => {
            print_lux_backend_record_input(record_input_args)
        }
        UnityCommand::ReplayInput(replay_input_args) => {
            print_lux_backend_replay_input(replay_input_args)
        }
        UnityCommand::ExecuteDynamicCode(execute_dynamic_code_args) => {
            print_lux_backend_execute_dynamic_code(execute_dynamic_code_args)
        }
    }
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

fn run_lux_unity_launch(args: UnityLaunchArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let launch_target = resolve_unity_launch_target(&project_root)?;

    eprintln!(
        "Lux launch: launching Unity editor for {}",
        project_root.display()
    );

    ProcessCommand::new(&launch_target.executable)
        .args(&launch_target.prefix_args)
        .arg("-projectPath")
        .arg(&project_root)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| {
            format!(
                "failed to launch Unity at {}",
                launch_target.executable.display()
            )
        })?;

    if args.no_wait {
        return Ok(());
    }

    wait_for_unity_bridge_ready(&project_root, Duration::from_secs(60))
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
            "searchMode": args.search_mode,
            "name": args.name,
            "regex": args.regex,
            "path": args.path,
            "component": args.component,
            "tag": args.tag,
            "layer": args.layer,
            "activeState": args.active_state,
            "inlineLimit": args.inline_limit,
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
    let response_line = send_unity_tcp_line(
        &discovery,
        &format!("{}\n", serde_json::to_string(&request)?),
    )?;
    let response_json: Value =
        serde_json::from_str(&response_line).context("Unity TCP response was not valid JSON")?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
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

fn print_lux_backend_screenshot(args: UnityScreenshotArgs) -> anyhow::Result<()> {
    if args.elements_only && !args.annotate_elements {
        bail!("--elements-only requires --annotate-elements");
    }

    let project_root = resolve_project_root(&args.project_path)?;
    let discovery = read_unity_bridge_discovery(&project_root)?;
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "capture_lux_screenshot",
        "token": discovery.token,
        "params": {
            "screenshotCaptureMode": args.capture_mode,
            "screenshotAnnotateElements": args.annotate_elements,
            "screenshotElementsOnly": args.elements_only,
            "actor": "lux-cli"
        }
    });
    let response_line = send_unity_tcp_line_with_timeout(
        &discovery,
        &format!("{}\n", serde_json::to_string(&request)?),
        Duration::from_secs(15),
    )?;
    let response_json: Value =
        serde_json::from_str(&response_line).context("Unity TCP response was not valid JSON")?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!(
            "Unity backend rejected capture_lux_screenshot: {}",
            response_json
        );
    }

    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("screenshotResult"))
        .context("Unity TCP response did not include payload.screenshotResult")?;
    let file_path = payload
        .get("filePath")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include payload.screenshotResult.filePath")?;
    let file_size_bytes = payload
        .get("fileSizeBytes")
        .and_then(Value::as_i64)
        .context("Unity TCP response did not include payload.screenshotResult.fileSizeBytes")?;
    let media_type = payload
        .get("mediaType")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include payload.screenshotResult.mediaType")?;
    let capture_mode = payload
        .get("captureMode")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include payload.screenshotResult.captureMode")?;
    let annotation_count = payload
        .get("annotationCount")
        .and_then(Value::as_i64)
        .context("Unity TCP response did not include payload.screenshotResult.annotationCount")?;
    let annotated_elements = payload
        .get("annotatedElements")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let annotated = payload
        .get("annotated")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let elements_only = payload
        .get("elementsOnly")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let screenshot_saved = payload
        .get("screenshotSaved")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "filePath": file_path,
            "fileSizeBytes": file_size_bytes,
            "mediaType": media_type,
            "captureMode": capture_mode,
            "annotated": annotated,
            "elementsOnly": elements_only,
            "screenshotSaved": screenshot_saved,
            "annotationCount": annotation_count,
            "annotatedElements": annotated_elements,
        }))?
    );
    Ok(())
}

fn print_lux_backend_simulate_keyboard(args: UnitySimulateKeyboardArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let response_json = send_lux_input_simulation_request(
        &project_root,
        "simulate_lux_keyboard",
        json!({
            "inputAction": args.action.as_str(),
            "inputKey": args.key,
            "inputDurationMs": args.duration_ms,
            "actor": "lux-cli"
        }),
    )?;
    print_lux_input_simulation_result(&response_json)
}

fn print_lux_backend_simulate_mouse_input(args: UnitySimulateMouseInputArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let response_json = send_lux_input_simulation_request(
        &project_root,
        "simulate_lux_mouse_input",
        json!({
            "inputAction": args.action.as_str(),
            "inputButton": args.button,
            "inputDeltaX": args.delta_x,
            "inputDeltaY": args.delta_y,
            "inputScrollX": args.scroll_x,
            "inputScrollY": args.scroll_y,
            "inputDurationMs": args.duration_ms,
            "inputSteps": args.steps,
            "actor": "lux-cli"
        }),
    )?;
    print_lux_input_simulation_result(&response_json)
}

fn print_lux_backend_record_input(args: UnityRecordInputArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let response_json = send_lux_input_simulation_request(
        &project_root,
        "record_lux_input",
        json!({
            "inputAction": args.action.as_str(),
            "actor": "lux-cli"
        }),
    )?;
    print_lux_input_record_result(&response_json)
}

fn print_lux_backend_replay_input(args: UnityReplayInputArgs) -> anyhow::Result<()> {
    if matches!(args.action, ReplayInputAction::Start) && args.file.is_none() {
        bail!("lux unity replay-input --action start requires --file <path>");
    }

    let project_root = resolve_project_root(&args.project_path)?;
    let response_json = send_lux_input_simulation_request(
        &project_root,
        "replay_lux_input",
        json!({
            "inputAction": args.action.as_str(),
            "inputFilePath": args.file.as_ref().map(|path| path.to_string_lossy().to_string()),
            "actor": "lux-cli"
        }),
    )?;
    print_lux_input_replay_result(&response_json)
}

fn print_lux_backend_simulate_mouse_ui(args: UnitySimulateMouseUiArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let response_json = send_lux_input_simulation_request(
        &project_root,
        "simulate_lux_mouse_ui",
        json!({
            "mouseUiAction": args.action.as_str(),
            "mouseUiX": args.x,
            "mouseUiY": args.y,
            "mouseUiDurationMs": args.duration_ms,
            "actor": "lux-cli"
        }),
    )?;
    print_lux_mouse_ui_result(&response_json)
}

fn send_lux_input_simulation_request(
    project_root: &Path,
    command: &str,
    params: Value,
) -> anyhow::Result<Value> {
    let discovery = read_unity_bridge_discovery(project_root)?;
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": command,
        "token": discovery.token,
        "params": params
    });
    let response_line = send_unity_tcp_line_with_timeout(
        &discovery,
        &format!("{}\n", serde_json::to_string(&request)?),
        Duration::from_secs(10),
    )?;
    let response_json: Value =
        serde_json::from_str(&response_line).context("Unity TCP response was not valid JSON")?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!("Unity backend rejected {command}: {}", response_json);
    }

    Ok(response_json)
}

fn print_lux_mouse_ui_result(response_json: &Value) -> anyhow::Result<()> {
    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("mouseUiResult"))
        .context("Unity TCP response did not include payload.mouseUiResult")?;
    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": schema_version,
            "capturedAtUtc": captured_at_utc,
            "action": payload.get("action").cloned().unwrap_or(Value::Null),
            "x": payload.get("x").cloned().unwrap_or(Value::Null),
            "y": payload.get("y").cloned().unwrap_or(Value::Null),
            "success": payload.get("success").cloned().unwrap_or(Value::Null),
            "targetName": payload.get("targetName").cloned().unwrap_or(Value::Null),
            "targetPath": payload.get("targetPath").cloned().unwrap_or(Value::Null),
            "raycastCount": payload.get("raycastCount").cloned().unwrap_or(Value::Null),
            "dragActive": payload.get("dragActive").cloned().unwrap_or(Value::Null),
        }))?
    );
    Ok(())
}

fn print_lux_input_simulation_result(response_json: &Value) -> anyhow::Result<()> {
    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("inputSimulationResult"))
        .context("Unity TCP response did not include payload.inputSimulationResult")?;
    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": schema_version,
            "capturedAtUtc": captured_at_utc,
            "device": payload.get("device").cloned().unwrap_or(Value::Null),
            "action": payload.get("action").cloned().unwrap_or(Value::Null),
            "key": payload.get("key").cloned().unwrap_or(Value::Null),
            "button": payload.get("button").cloned().unwrap_or(Value::Null),
            "deltaX": payload.get("deltaX").cloned().unwrap_or(Value::Null),
            "deltaY": payload.get("deltaY").cloned().unwrap_or(Value::Null),
            "scrollX": payload.get("scrollX").cloned().unwrap_or(Value::Null),
            "scrollY": payload.get("scrollY").cloned().unwrap_or(Value::Null),
            "heldKeys": payload.get("heldKeys").cloned().unwrap_or_else(|| json!([])),
            "heldButtons": payload.get("heldButtons").cloned().unwrap_or_else(|| json!([])),
            "queuedActions": payload.get("queuedActions").cloned().unwrap_or(Value::Null),
        }))?
    );
    Ok(())
}

fn print_lux_input_record_result(response_json: &Value) -> anyhow::Result<()> {
    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("inputRecordResult"))
        .context("Unity TCP response did not include payload.inputRecordResult")?;
    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": schema_version,
            "capturedAtUtc": captured_at_utc,
            "action": payload.get("action").cloned().unwrap_or(Value::Null),
            "active": payload.get("active").cloned().unwrap_or(Value::Null),
            "frameCount": payload.get("frameCount").cloned().unwrap_or(Value::Null),
            "filePath": payload.get("filePath").cloned().unwrap_or(Value::Null),
            "fileSizeBytes": payload.get("fileSizeBytes").cloned().unwrap_or(Value::Null),
            "mediaType": payload.get("mediaType").cloned().unwrap_or(Value::Null),
            "message": payload.get("message").cloned().unwrap_or(Value::Null),
        }))?
    );
    Ok(())
}

fn print_lux_input_replay_result(response_json: &Value) -> anyhow::Result<()> {
    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("inputReplayResult"))
        .context("Unity TCP response did not include payload.inputReplayResult")?;
    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": schema_version,
            "capturedAtUtc": captured_at_utc,
            "action": payload.get("action").cloned().unwrap_or(Value::Null),
            "active": payload.get("active").cloned().unwrap_or(Value::Null),
            "filePath": payload.get("filePath").cloned().unwrap_or(Value::Null),
            "frameCount": payload.get("frameCount").cloned().unwrap_or(Value::Null),
            "replayedFrameCount": payload.get("replayedFrameCount").cloned().unwrap_or(Value::Null),
            "completed": payload.get("completed").cloned().unwrap_or(Value::Null),
            "message": payload.get("message").cloned().unwrap_or(Value::Null),
        }))?
    );
    Ok(())
}

fn print_lux_backend_execute_dynamic_code(args: UnityExecuteDynamicCodeArgs) -> anyhow::Result<()> {
    let code = resolve_dynamic_code_source(&args)?;
    let project_root = resolve_project_root(&args.project_path)?;
    let discovery = read_unity_bridge_discovery(&project_root)?;
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "execute_lux_dynamic_code",
        "token": discovery.token,
        "params": {
            "dynamicCode": code,
            "actor": "lux-cli"
        }
    });
    let response_line = send_unity_tcp_line_with_timeout(
        &discovery,
        &format!("{}\n", serde_json::to_string(&request)?),
        Duration::from_secs(30),
    )?;
    let response_json: Value =
        serde_json::from_str(&response_line).context("Unity TCP response was not valid JSON")?;
    if response_json.get("ok").and_then(Value::as_bool) != Some(true) {
        bail!(
            "Unity backend rejected execute_lux_dynamic_code: {}",
            response_json
        );
    }

    print_lux_dynamic_code_result(&response_json)
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

fn print_lux_dynamic_code_result(response_json: &Value) -> anyhow::Result<()> {
    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("dynamicCodeResult"))
        .context("Unity TCP response did not include payload.dynamicCodeResult")?;
    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": schema_version,
            "capturedAtUtc": captured_at_utc,
            "success": payload.get("success").cloned().unwrap_or(Value::Null),
            "action": payload.get("action").cloned().unwrap_or(Value::Null),
            "result": payload.get("result").cloned().unwrap_or(Value::Null),
            "resultType": payload.get("resultType").cloned().unwrap_or(Value::Null),
            "message": payload.get("message").cloned().unwrap_or(Value::Null),
            "diagnostics": payload.get("diagnostics").cloned().unwrap_or_else(|| json!([])),
            "logs": payload.get("logs").cloned().unwrap_or_else(|| json!([])),
            "elapsedTimeMs": payload.get("elapsedTimeMs").cloned().unwrap_or(Value::Null),
        }))?
    );
    Ok(())
}

fn print_lux_backend_control_play_mode(args: UnityControlPlayModeArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let requested_action = args.action.as_str();
    let initial_response = fetch_lux_backend_play_mode_state(&project_root, requested_action)?;
    let mut state = extract_lux_play_mode_state(&initial_response, requested_action)?;

    if args.wait && requested_action != "status" {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !play_mode_state_matches(&state, requested_action) {
            if Instant::now() >= deadline {
                bail!(
                    "timed out waiting for PlayMode action {requested_action}; last state: {}",
                    serde_json::to_string(&state)?
                );
            }

            std::thread::sleep(Duration::from_millis(250));
            let poll_response = fetch_lux_backend_play_mode_state(&project_root, "status")?;
            state = extract_lux_play_mode_state(&poll_response, requested_action)?;
        }
    }

    println!("{}", serde_json::to_string_pretty(&state)?);
    Ok(())
}

fn fetch_lux_backend_play_mode_state(project_root: &Path, action: &str) -> anyhow::Result<Value> {
    let discovery = read_unity_bridge_discovery(project_root)?;
    let request = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "control_lux_play_mode",
        "token": discovery.token,
        "params": {
            "playModeAction": action,
            "actor": "lux-cli"
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
            "Unity backend rejected control_lux_play_mode: {}",
            response_json
        );
    }

    Ok(response_json)
}

fn extract_lux_play_mode_state(
    response_json: &Value,
    requested_action: &str,
) -> anyhow::Result<Value> {
    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("playModeState"))
        .context("Unity TCP response did not include payload.playModeState")?;
    let is_playing = payload
        .get("isPlaying")
        .and_then(Value::as_bool)
        .context("Unity TCP response did not include payload.playModeState.isPlaying")?;
    let is_paused = payload
        .get("isPaused")
        .and_then(Value::as_bool)
        .context("Unity TCP response did not include payload.playModeState.isPaused")?;
    let transition_requested = payload
        .get("transitionRequested")
        .and_then(Value::as_bool)
        .context("Unity TCP response did not include payload.playModeState.transitionRequested")?;
    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;

    Ok(json!({
        "schemaVersion": schema_version,
        "capturedAtUtc": captured_at_utc,
        "action": requested_action,
        "isPlaying": is_playing,
        "isPaused": is_paused,
        "transitionRequested": transition_requested,
    }))
}

fn play_mode_state_matches(state: &Value, action: &str) -> bool {
    let is_playing = state
        .get("isPlaying")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let is_paused = state
        .get("isPaused")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let transition_requested = state
        .get("transitionRequested")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    match action {
        "play" => is_playing && !transition_requested,
        "stop" => !is_playing && !transition_requested,
        "pause" => is_playing && is_paused,
        "resume" => is_playing && !is_paused,
        "status" => true,
        _ => false,
    }
}

fn print_lux_backend_status(args: UnityBackendStatusArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let discovery_path = project_root.join("Library/UnityAiBridge/server.json");
    let discovery = match read_unity_bridge_discovery(&project_root) {
        Ok(discovery) => discovery,
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
            return Ok(());
        }
    };

    let ping = json!({
        "schemaVersion": 1,
        "requestId": uuid::Uuid::new_v4().to_string(),
        "command": "ping",
        "token": discovery.token,
        "params": {}
    });
    let ping_result =
        send_unity_tcp_line(&discovery, &format!("{}\n", serde_json::to_string(&ping)?));
    match ping_result {
        Ok(response_line) => {
            let response_json: Value = serde_json::from_str(&response_line)
                .unwrap_or_else(|_| json!({ "raw": response_line }));
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "running": true,
                    "host": discovery.host,
                    "port": discovery.port,
                    "discovery_path": discovery_path,
                    "ping": response_json,
                }))?
            );
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": false,
                    "running": false,
                    "host": discovery.host,
                    "port": discovery.port,
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

fn print_lux_backend_console_logs(args: UnityGetLogsArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let response_json = fetch_lux_backend_command_response(&project_root, "get_lux_console_logs")?;
    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("consoleLogs"))
        .context("Unity TCP response did not include payload.consoleLogs")?;

    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;
    let total_count = payload
        .get("totalCount")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include payload.consoleLogs.totalCount")?;
    let displayed_count = payload
        .get("displayedCount")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include payload.consoleLogs.displayedCount")?;
    let console_logs = payload
        .get("consoleLogs")
        .and_then(Value::as_array)
        .context("Unity TCP response did not include payload.consoleLogs.consoleLogs")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": schema_version,
            "capturedAtUtc": captured_at_utc,
            "totalCount": total_count,
            "displayedCount": displayed_count,
            "consoleLogs": console_logs,
        }))?
    );

    Ok(())
}

fn clear_lux_backend_clear_console(args: UnityClearConsoleArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let response_json = fetch_lux_backend_command_response(&project_root, "clear_lux_console")?;
    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("consoleClearResult"))
        .context("Unity TCP response did not include payload.consoleClearResult")?;

    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;
    let before_count = payload
        .get("beforeCount")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include payload.consoleClearResult.beforeCount")?;
    let after_count = payload
        .get("afterCount")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include payload.consoleClearResult.afterCount")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": schema_version,
            "capturedAtUtc": captured_at_utc,
            "beforeCount": before_count,
            "afterCount": after_count,
        }))?
    );

    Ok(())
}

fn print_lux_backend_focus_window(args: UnityFocusWindowArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let response_json = fetch_lux_backend_command_response(&project_root, "focus_lux_window")?;
    let payload = response_json
        .get("payload")
        .and_then(|payload| payload.get("focusWindowResult"))
        .context("Unity TCP response did not include payload.focusWindowResult")?;

    let schema_version = response_json
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .context("Unity TCP response did not include schemaVersion")?;
    let captured_at_utc = response_json
        .get("capturedAtUtc")
        .and_then(Value::as_str)
        .context("Unity TCP response did not include capturedAtUtc")?;
    let focused = payload
        .get("focused")
        .and_then(Value::as_bool)
        .context("Unity TCP response did not include payload.focusWindowResult.focused")?;

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": schema_version,
            "capturedAtUtc": captured_at_utc,
            "focused": focused,
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

fn send_unity_tcp_line(
    discovery: &UnityBridgeDiscovery,
    request_line: &str,
) -> anyhow::Result<String> {
    send_unity_tcp_line_with_timeout(discovery, request_line, Duration::from_secs(10))
}

fn wait_for_unity_bridge_ready(project_root: &Path, timeout: Duration) -> anyhow::Result<()> {
    let deadline = Instant::now() + timeout;
    let discovery_path = project_root.join("Library/UnityAiBridge/server.json");
    let mut last_error: Option<String> = None;

    loop {
        if Instant::now() >= deadline {
            let message = last_error
                .map(|error| format!(": {error}"))
                .unwrap_or_default();
            bail!(
                "timed out waiting for Unity bridge readiness at {}{}",
                discovery_path.display(),
                message
            );
        }

        match read_unity_bridge_discovery(project_root) {
            Ok(discovery) => {
                let ping = json!({
                    "schemaVersion": 1,
                    "requestId": uuid::Uuid::new_v4().to_string(),
                    "command": "ping",
                    "token": discovery.token,
                    "params": {}
                });
                match send_unity_tcp_line_with_timeout(
                    &discovery,
                    &format!("{}\n", serde_json::to_string(&ping)?),
                    Duration::from_secs(1),
                ) {
                    Ok(response_line) => {
                        let response_json: Value = serde_json::from_str(&response_line)
                            .context("Unity TCP response was not valid JSON")?;
                        if response_json.get("ok").and_then(Value::as_bool) == Some(true)
                            && response_json
                                .get("payload")
                                .and_then(|payload| payload.get("ping"))
                                .and_then(|ping| ping.get("status"))
                                .and_then(Value::as_str)
                                == Some("ok")
                        {
                            return Ok(());
                        }
                        last_error =
                            Some(format!("Unity TCP ping was not ready: {}", response_json));
                    }
                    Err(error) => {
                        last_error = Some(error.to_string());
                    }
                }
            }
            Err(error) => {
                last_error = Some(error.to_string());
            }
        }

        std::thread::sleep(Duration::from_millis(250));
    }
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
    }
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

    if let Ok(discovery) = read_unity_bridge_discovery(&project_root) {
        // Try dedicated compile command first
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
                // If command is registered, use its result
                if response_json.get("errorCode").and_then(Value::as_str) != Some("unknown_command")
                {
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
                // compile_lux_project not registered — fall through to dynamic code
                eprintln!(
                    "compile_lux_project not registered, trying execute-dynamic-code fallback..."
                );
            }
            Err(error) => {
                eprintln!("Live Unity Editor compile failed to connect, falling back to batch mode: {error}");
            }
        }

        // Fallback: use execute-dynamic-code to compile in live Editor
        let compile_code = "UnityEngine.Debug.Log(\"LUX compile fallback: counting errors from console\"); UnityEditor.AssetDatabase.Refresh(UnityEditor.ImportAssetOptions.ForceUpdate); var errorCount = 0; foreach (var log in Linalab.Lux.Editor.LuxUnityContext.GetRecentLogsSnapshot()) { if (string.Equals(log.Type, \"Error\", System.StringComparison.OrdinalIgnoreCase)) { errorCount++; } } var ok = !UnityEditor.EditorUtility.scriptCompilationFailed && errorCount == 0; return new { ok, error_count = errorCount, message = ok ? \"Compilation succeeded.\" : $\"Script compilation failed with {errorCount} error(s).\", timestamp_utc = System.DateTime.UtcNow.ToString(\"O\") };";
        let dynamic_request = json!({
            "schemaVersion": 1,
            "requestId": uuid::Uuid::new_v4().to_string(),
            "command": "execute_lux_dynamic_code",
            "token": discovery.token,
            "params": { "dynamicCode": compile_code }
        });
        match send_unity_tcp_line(
            &discovery,
            &format!("{}\n", serde_json::to_string(&dynamic_request)?),
        ) {
            Ok(response) => {
                let response_json: Value = serde_json::from_str(&response)
                    .context("dynamic code compile response was not valid JSON")?;
                let compile_ok = response_json.get("ok").and_then(Value::as_bool) == Some(true);
                if let Some(payload) = response_json
                    .get("payload")
                    .and_then(|p| p.get("dynamicCodeResult"))
                {
                    let _success = payload
                        .get("success")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let result_str = payload.get("result").and_then(Value::as_str).unwrap_or("");
                    let ok = result_str.contains("ok = True") || result_str.contains("ok=True");
                    println!(
                        "{{\"ok\": {}, \"message\": \"{}\", \"source\": \"dynamic-code\"}}",
                        ok,
                        if ok {
                            "Compilation succeeded."
                        } else {
                            "Script compilation failed. Check Unity console for errors."
                        }
                    );
                    if !ok {
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
            Err(error) => {
                eprintln!(
                    "Dynamic code compile also failed: {error}, falling back to batch mode..."
                );
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
        .arg("-batchmode")
        .arg("-quit")
        .arg("-nographics")
        .arg("-projectPath")
        .arg(&project_root)
        .arg("-executeMethod")
        .arg("Linalab.Lux.Editor.LuxBatchCompile.Compile")
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

    if compile_result_path.exists() {
        let result_text = fs::read_to_string(&compile_result_path)
            .with_context(|| format!("failed to read {}", compile_result_path.display()))?;
        println!("{}", result_text.trim());
    } else {
        println!(
            "{{ \"ok\": false, \"message\": \"Unity exited without writing CompileResult.json\", \"unity_exit_success\": {} }}",
            status.success()
        );
    }

    if !status.success() {
        eprintln!("Lux compile failed. See log: {}", log_path.display());
        std::process::exit(1);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// lux run-tests — Unity batch mode via -runTests
// ---------------------------------------------------------------------------

fn run_batch_tests(args: RunTestsArgs) -> anyhow::Result<()> {
    let project_root = resolve_project_root(&args.project_path)?;
    let launch_target = resolve_unity_launch_target(&project_root)?;
    let platform = args.test_platform;

    let results_dir = project_root.join("TestResults");
    fs::create_dir_all(&results_dir)
        .with_context(|| format!("failed to create {}", results_dir.display()))?;

    let platform_label = match platform.as_str() {
        "EditMode" => "EditMode",
        "PlayMode" => "PlayMode",
        other => other,
    };
    let test_results = match &args.test_results {
        Some(p) => p.clone(),
        None => results_dir.join(format!("{}Results.xml", platform_label)),
    };
    let log_file = match &args.log_file {
        Some(p) => p.clone(),
        None => results_dir.join(format!("{}Log.log", platform_label)),
    };

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
        let hub_editor = PathBuf::from(format!(
            "C:\\Program Files\\Unity\\Hub\\Editor\\{version}\\Editor\\Unity.exe"
        ));
        if hub_editor.is_file() {
            return Ok(UnityLaunchTarget {
                executable: hub_editor,
                prefix_args: Vec::new(),
            });
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

async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let addr = SocketAddr::new(args.host, args.port);
    let project_root = args
        .project_path
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
        token: args.token,
        history_capacity: args.history_capacity,
        project_root,
        addon_auth: crate::addon_auth::AddonAuthConfig {
            github_client_id: std::env::var("LUX_GITHUB_CLIENT_ID")
                .unwrap_or_else(|_| "placeholder_client_id".to_string()),
            github_client_secret: std::env::var("LUX_GITHUB_CLIENT_SECRET").ok(),
        },
    });
    let idle_timeout = args.idle_timeout.checked_mul(60).map(Duration::from_secs);
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
            println!("{{\"ok\": true, \"message\": \"package already installed\", \"path\": \"{}\"}}", package_dir.display());
        } else {
            println!("Package {} is already installed at {}", package_name, package_dir.display());
        }
        return Ok(());
    }

    let manifest_path = packages_dir.join("manifest.json");
    if manifest_path.exists() {
        let content = fs::read_to_string(&manifest_path)
            .with_context(|| format!("failed to read {}", manifest_path.display()))?;
        let mut manifest: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| "failed to parse manifest.json")?;

        if let Some(deps) = manifest.get_mut("dependencies").and_then(|d| d.as_object_mut()) {
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
        bail!("Packages/manifest.json not found at {}", manifest_path.display());
    }

    if args.json {
        println!("{{\"ok\": true, \"package\": \"{}\", \"repo\": \"{}\"}}", package_name, repo_url);
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
