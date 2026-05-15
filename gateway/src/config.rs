use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LuxConfig {
    #[serde(default)]
    pub unity: UnityConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnityConfig {
    /// Path to Unity Hub installation (auto-detected if empty)
    pub hub_path: Option<PathBuf>,
    /// Path to Unity Editor executable (overrides all detection)
    pub editor_path: Option<PathBuf>,
    /// Custom Hub editor install root (e.g., D:\Unity)
    pub custom_install_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Default host for `lux serve`
    #[serde(default = "default_host")]
    pub host: String,
    /// Default port for `lux serve`
    #[serde(default = "default_port")]
    pub port: u16,
    /// Idle timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
    /// Auth token (if not set, uses LUX_TOKEN env var)
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Project root path (auto-detected from cwd if empty)
    pub project_root: Option<PathBuf>,
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            idle_timeout_secs: default_idle_timeout(),
            token: None,
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            project_root: None,
            log_level: default_log_level(),
        }
    }
}

pub fn default_host() -> String {
    "127.0.0.1".to_string()
}

pub fn default_port() -> u16 {
    17340
}

pub fn default_idle_timeout() -> u64 {
    30 * 60
}

pub fn default_log_level() -> String {
    "info".to_string()
}

pub fn config_dir() -> PathBuf {
    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("lux");
        }
    }

    BaseDirs::new()
        .map(|dirs| dirs.home_dir().join(".lux"))
        .unwrap_or_else(|| PathBuf::from(".lux"))
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn load() -> Result<LuxConfig> {
    load_from_path(config_path())
}

pub fn load_from_path(path: PathBuf) -> Result<LuxConfig> {
    if !path.exists() {
        return Ok(LuxConfig::default());
    }

    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("failed to parse config file {}", path.display()))
}

pub fn save(config: &LuxConfig) -> Result<()> {
    save_to_path(config_path(), config)
}

pub fn save_to_path(path: PathBuf, config: &LuxConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config directory {}", parent.display()))?;
    }

    let text = toml::to_string_pretty(config).context("failed to serialize Lux config")?;
    fs::write(&path, text)
        .with_context(|| format!("failed to write config file {}", path.display()))
}

pub fn merge_with_cli<T>(config: &LuxConfig, _cli_args: &T) -> LuxConfig {
    config.clone()
}
