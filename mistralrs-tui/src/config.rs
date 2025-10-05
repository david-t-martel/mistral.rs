//! Configuration helpers for the TUI binary.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    pub database_path: PathBuf,
    pub model_search_paths: Vec<PathBuf>,
    pub prefer_gpu: bool,
    pub inventory_file: Option<PathBuf>,
    pub default_model: Option<String>,
    /// Agent configuration (feature-gated)
    #[cfg(feature = "tui-agent")]
    #[serde(default)]
    pub agent: AgentConfig,
}

/// Configuration for agent mode
#[cfg(feature = "tui-agent")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Enable agent mode by default for new sessions
    pub enabled_by_default: bool,
    /// Sandbox root directory for agent tools
    pub sandbox_root: Option<PathBuf>,
    /// Maximum number of tool calls to keep in history
    pub max_history: usize,
}

#[cfg(feature = "tui-agent")]
impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            enabled_by_default: false,
            sandbox_root: None,
            max_history: 1000,
        }
    }
}

impl Default for TuiConfig {
    fn default() -> Self {
        let dirs = project_dirs();
        let config_dir = dirs.config_dir();
        let data_dir = dirs.data_dir();
        let db_path = config_dir.join("tui_sessions.sqlite3");
        let default_models_path = data_dir.join("models");

        Self {
            database_path: db_path,
            model_search_paths: vec![default_models_path],
            prefer_gpu: true,
            inventory_file: find_local_inventory_file(),
            default_model: None,
            #[cfg(feature = "tui-agent")]
            agent: AgentConfig::default(),
        }
    }
}

impl TuiConfig {
    pub fn load(custom_path: Option<&Path>) -> Result<(Self, PathBuf)> {
        let default_path = default_config_path();
        let path = custom_path
            .map(PathBuf::from)
            .unwrap_or_else(|| default_path.clone());

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create config directory at {}", parent.display())
                })?;
            }
        }

        if path.exists() {
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file {}", path.display()))?;
            let mut cfg: TuiConfig = toml::from_str(&contents)
                .with_context(|| format!("Failed to parse TOML config at {}", path.display()))?;
            cfg.ensure_defaults();
            Ok((cfg, path))
        } else {
            let cfg = TuiConfig::default();
            cfg.persist(&path)?;
            Ok((cfg, path))
        }
    }

    pub fn persist(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Failed to create directory for config at {}",
                        parent.display()
                    )
                })?;
            }
        }

        let serialized = toml::to_string_pretty(self)?;
        fs::write(path, serialized)
            .with_context(|| format!("Failed to write config file {}", path.display()))
    }

    pub fn effective_inventory_file(&self) -> Option<PathBuf> {
        self.inventory_file
            .clone()
            .or_else(find_local_inventory_file)
    }

    pub fn ensure_defaults(&mut self) {
        if self.model_search_paths.is_empty() {
            self.model_search_paths = vec![project_dirs().data_dir().join("models")];
        }
        if !self.database_path.is_absolute() {
            self.database_path = project_dirs().config_dir().join(&self.database_path);
        }
    }
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "", "mistral.rs")
        .expect("Failed to determine project directories for mistral.rs")
}

fn default_config_path() -> PathBuf {
    project_dirs().config_dir().join("tui.toml")
}

fn find_local_inventory_file() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let candidate = cwd.join("MODEL_INVENTORY.json");
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_creates_default_file() {
        let tmp = tempdir().unwrap();
        let cfg_path = tmp.path().join("tui.toml");
        let (_cfg, path) = TuiConfig::load(Some(&cfg_path)).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn ensure_defaults_promotes_relative_database_path() {
        let mut cfg = TuiConfig {
            database_path: PathBuf::from("relative.db"),
            ..Default::default()
        };
        cfg.ensure_defaults();
        assert!(cfg.database_path.is_absolute());
    }
}
