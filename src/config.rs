use crate::headings::SlugMode;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    pub links: LinkConfig,
    pub rules: HashMap<String, RuleConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct WorkspaceConfig {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            include: vec!["**/*.md".to_string()],
            exclude: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
            ],
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct LinkConfig {
    pub slug_mode: SlugMode,
    pub wikilink_resolution: WikilinkResolution,
    pub check_external: bool,
    pub warn_case_mismatch: bool,
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self {
            slug_mode: SlugMode::Gfm,
            wikilink_resolution: WikilinkResolution::ShortestPath,
            check_external: false,
            warn_case_mismatch: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WikilinkResolution {
    #[default]
    ShortestPath,
    Relative,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum RuleConfig {
    Level(RuleLevel),
    Full(RuleConfigFull),
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleLevel {
    Error,
    Warning,
    Info,
    Off,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RuleConfigFull {
    pub level: RuleLevel,
    #[serde(flatten)]
    pub options: HashMap<String, toml::Value>,
}

impl RuleConfig {
    pub fn level(&self) -> RuleLevel {
        match self {
            RuleConfig::Level(l) => *l,
            RuleConfig::Full(f) => f.level,
        }
    }

    pub fn is_off(&self) -> bool {
        self.level() == RuleLevel::Off
    }

    pub fn option_str(&self, key: &str) -> Option<&str> {
        match self {
            RuleConfig::Level(_) => None,
            RuleConfig::Full(f) => f.options.get(key).and_then(|v| v.as_str()),
        }
    }

    pub fn option_strs(&self, key: &str) -> Vec<String> {
        match self {
            RuleConfig::Level(_) => vec![],
            RuleConfig::Full(f) => f
                .options
                .get(key)
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    pub fn option_u8(&self, key: &str) -> Option<u8> {
        match self {
            RuleConfig::Level(_) => None,
            RuleConfig::Full(f) => f
                .options
                .get(key)
                .and_then(|v| v.as_integer())
                .and_then(|n| u8::try_from(n).ok()),
        }
    }
}

impl Config {
    /// Load config from a specific path, or search upward from the target directory.
    pub fn load(
        explicit_path: Option<&Path>,
        target_dir: &Path,
    ) -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(path) = explicit_path {
            let content = std::fs::read_to_string(path)?;
            let config: Config = toml::from_str(&content)?;
            return Ok(config);
        }

        // Walk up from target_dir looking for mdlint.toml
        if let Some(path) = find_config_file(target_dir) {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            return Ok(config);
        }

        // No config found, use defaults
        Ok(Config::default())
    }

    pub fn rule_config(&self, name: &str) -> Option<&RuleConfig> {
        self.rules.get(name)
    }

    pub fn rule_is_enabled(&self, name: &str) -> bool {
        match self.rules.get(name) {
            Some(rc) => !rc.is_off(),
            None => true, // enabled by default
        }
    }
}

fn find_config_file(start: &Path) -> Option<PathBuf> {
    let start = if start.is_file() {
        start.parent()?
    } else {
        start
    };

    let mut dir = start.canonicalize().ok()?;
    loop {
        let candidate = dir.join("mdlint.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.links.slug_mode, SlugMode::Gfm);
        assert!(!cfg.links.check_external);
        assert!(cfg.links.warn_case_mismatch);
    }

    #[test]
    fn test_parse_config() {
        let toml_str = r#"
[links]
slug_mode = "obsidian"
check_external = true

[rules.broken-links]
level = "error"

[rules.require-frontmatter]
level = "warning"
fields = ["title", "date"]
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.links.slug_mode, SlugMode::Obsidian);
        assert!(cfg.links.check_external);
        assert!(cfg.rule_is_enabled("broken-links"));

        let fm_rule = cfg.rule_config("require-frontmatter").unwrap();
        assert_eq!(fm_rule.level(), RuleLevel::Warning);
        assert_eq!(fm_rule.option_strs("fields"), vec!["title", "date"]);
    }

    #[test]
    fn test_rule_off() {
        let toml_str = r#"
[rules.heading-increment]
level = "off"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert!(!cfg.rule_is_enabled("heading-increment"));
    }
}
