use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{default_glossary_path, TargetPlatformSpec};

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
