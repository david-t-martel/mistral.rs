//! TUI wrapper for AgentToolkit
//!
//! Provides a TUI-specific interface to the agent tools with session tracking,
//! execution history, and UI integration.

use anyhow::Result;
use chrono::{DateTime, Utc};
use mistralrs_agent_tools::{AgentToolkit as CoreToolkit, SandboxConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// TUI-specific agent toolkit wrapper
///
/// Wraps the core AgentToolkit with TUI-specific functionality:
/// - Tool call tracking and history
/// - Session-based execution
/// - UI state management
#[derive(Debug, Clone)]
pub struct AgentToolkit {
    /// Core agent toolkit instance
    core: CoreToolkit,
    /// Configuration
    config: ToolkitConfig,
}

/// Configuration for the agent toolkit in TUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolkitConfig {
    /// Sandbox root directory
    pub sandbox_root: PathBuf,
    /// Whether to track tool calls
    pub track_calls: bool,
    /// Maximum number of tool calls to keep in history
    pub max_history: usize,
}

impl Default for ToolkitConfig {
    fn default() -> Self {
        Self {
            sandbox_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            track_calls: true,
            max_history: 1000,
        }
    }
}

/// Represents a single tool call execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: Uuid,
    /// Tool name
    pub tool_name: String,
    /// Arguments passed to the tool
    pub arguments: serde_json::Value,
    /// Execution result (if completed)
    pub result: Option<ToolCallResult>,
    /// Timestamp when the call was initiated
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
    /// Session ID this call belongs to
    pub session_id: Option<Uuid>,
}

/// Result of a tool call execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Whether the tool call succeeded
    pub success: bool,
    /// Output from the tool
    pub output: serde_json::Value,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution duration
    #[serde(with = "duration_serde")]
    pub duration: std::time::Duration,
}

mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

impl AgentToolkit {
    /// Create a new agent toolkit with the given configuration
    pub fn new(config: ToolkitConfig) -> Result<Self> {
        let sandbox_config = SandboxConfig::new(config.sandbox_root.clone());
        let core = CoreToolkit::new(sandbox_config);

        Ok(Self { core, config })
    }

    /// Create toolkit with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(ToolkitConfig::default())
    }

    /// Create toolkit with a specific sandbox root
    pub fn with_root(root: PathBuf) -> Result<Self> {
        let config = ToolkitConfig {
            sandbox_root: root,
            ..Default::default()
        };
        Self::new(config)
    }

    /// Get reference to the core toolkit
    pub fn core(&self) -> &CoreToolkit {
        &self.core
    }

    /// Get the current configuration
    pub fn config(&self) -> &ToolkitConfig {
        &self.config
    }

    /// Update the sandbox root
    pub fn set_sandbox_root(&mut self, root: PathBuf) -> Result<()> {
        self.config.sandbox_root = root.clone();
        let sandbox_config = SandboxConfig::new(root);
        self.core = CoreToolkit::new(sandbox_config);
        Ok(())
    }

    /// Create a new tool call record
    pub fn create_tool_call(
        &self,
        tool_name: String,
        arguments: serde_json::Value,
        session_id: Option<Uuid>,
    ) -> ToolCall {
        ToolCall {
            id: Uuid::new_v4(),
            tool_name,
            arguments,
            result: None,
            timestamp: Utc::now(),
            session_id,
        }
    }

    /// Get available tools count
    pub fn tool_count(&self) -> usize {
        // This will be implemented as we add more tool tracking
        90 // Placeholder: 90+ Unix utilities as documented
    }

    /// Get toolkit status summary
    pub fn status_summary(&self) -> String {
        format!(
            "Sandbox: {} | Tools: {}",
            self.config.sandbox_root.display(),
            self.tool_count()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolkit_creation() {
        let toolkit = AgentToolkit::with_defaults().unwrap();
        assert!(toolkit.tool_count() > 0);
    }

    #[test]
    fn test_toolkit_config_update() {
        let mut toolkit = AgentToolkit::with_defaults().unwrap();
        let new_root = PathBuf::from("/tmp");
        toolkit.set_sandbox_root(new_root.clone()).unwrap();
        assert_eq!(toolkit.config().sandbox_root, new_root);
    }

    #[test]
    fn test_tool_call_creation() {
        let toolkit = AgentToolkit::with_defaults().unwrap();
        let call =
            toolkit.create_tool_call("ls".to_string(), serde_json::json!({"path": "."}), None);
        assert_eq!(call.tool_name, "ls");
        assert!(call.result.is_none());
    }
}
