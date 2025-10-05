//! UI components for agent tool visualization
//!
//! This module will contain:
//! - Tool panel widget
//! - Tool browser widget
//! - Tool call history widget
//! - Agent status indicators

// Placeholder: UI components will be implemented in Phase 2.6.2

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders};

/// Tool panel widget showing available tools
pub struct ToolPanel {
    /// Whether the panel is focused
    pub focused: bool,
}

impl ToolPanel {
    /// Create a new tool panel
    pub fn new() -> Self {
        Self { focused: false }
    }
}

impl Default for ToolPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ToolPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Available Tools")
            .borders(Borders::ALL);
        block.render(area, buf);
    }
}
