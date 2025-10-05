//! Agent mode integration for mistralrs-tui
//!
//! This module provides agent tool support for the TUI, including:
//! - Tool registry and management
//! - Tool execution and sandboxing
//! - UI components for tool visualization
//!
//! The agent functionality is feature-gated behind the `tui-agent` feature.

#[cfg(feature = "tui-agent")]
pub mod toolkit;

#[cfg(feature = "tui-agent")]
pub mod ui;

#[cfg(feature = "tui-agent")]
pub mod execution;

#[cfg(feature = "tui-agent")]
pub use toolkit::AgentToolkit;
