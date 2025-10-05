//! UI components for agent tool visualization
//!
//! This module contains widgets for displaying and interacting with agent tools:
//! - ToolPanel: Shows available tools by category
//! - ToolBrowser: Interactive tool search and details
//! - CallHistory: Displays execution history
//! - AgentStatusBar: Shows agent mode status

use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::toolkit::ToolCall;

/// Agent UI state management
#[derive(Debug, Clone)]
pub struct AgentUiState {
    /// Whether the tool panel is visible
    pub panel_visible: bool,
    /// Whether the tool browser is visible
    pub browser_visible: bool,
    /// Whether the call history is visible
    pub history_visible: bool,
    /// Current cursor position in tool panel
    pub tool_cursor: usize,
    /// Current cursor position in history
    pub history_cursor: usize,
    /// Filter text for tool browser search
    pub filter_text: String,
}

impl Default for AgentUiState {
    fn default() -> Self {
        Self {
            panel_visible: false,
            browser_visible: false,
            history_visible: false,
            tool_cursor: 0,
            history_cursor: 0,
            filter_text: String::new(),
        }
    }
}

impl AgentUiState {
    /// Create a new agent UI state
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle tool panel visibility
    pub fn toggle_panel(&mut self) {
        self.panel_visible = !self.panel_visible;
    }

    /// Toggle tool browser visibility
    pub fn toggle_browser(&mut self) {
        self.browser_visible = !self.browser_visible;
    }

    /// Toggle call history visibility
    pub fn toggle_history(&mut self) {
        self.history_visible = !self.history_visible;
    }

    /// Reset all visibility flags
    pub fn reset(&mut self) {
        self.panel_visible = false;
        self.browser_visible = false;
        self.history_visible = false;
    }
}

/// Tool category for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCategory {
    FileSystem,
    ProcessManagement,
    Network,
    TextProcessing,
    SystemInfo,
    Other,
}

impl ToolCategory {
    /// Get display name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            ToolCategory::FileSystem => "File System",
            ToolCategory::ProcessManagement => "Process Management",
            ToolCategory::Network => "Network",
            ToolCategory::TextProcessing => "Text Processing",
            ToolCategory::SystemInfo => "System Info",
            ToolCategory::Other => "Other",
        }
    }

    /// Get color for the category
    pub fn color(&self) -> Color {
        match self {
            ToolCategory::FileSystem => Color::Blue,
            ToolCategory::ProcessManagement => Color::Green,
            ToolCategory::Network => Color::Magenta,
            ToolCategory::TextProcessing => Color::Yellow,
            ToolCategory::SystemInfo => Color::Cyan,
            ToolCategory::Other => Color::Gray,
        }
    }
}

/// Tool information for display
#[derive(Debug, Clone)]
pub struct ToolInfo {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool category
    pub category: ToolCategory,
}

impl ToolInfo {
    /// Create a new tool info
    pub fn new(name: String, description: String, category: ToolCategory) -> Self {
        Self {
            name,
            description,
            category,
        }
    }
}

/// Render the tool panel widget
pub fn render_tool_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    tools: &[ToolInfo],
    cursor: usize,
    focused: bool,
) {
    let items: Vec<ListItem> = tools
        .iter()
        .map(|tool| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    tool.name.clone(),
                    Style::default().fg(tool.category.color()),
                )),
                Line::from(Span::styled(
                    tool.description.clone(),
                    Style::default().fg(Color::Gray),
                )),
            ])
        })
        .collect();

    let mut block = Block::default()
        .title("Available Tools")
        .borders(Borders::ALL);
    if focused {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(cursor));
    frame.render_stateful_widget(list, area, &mut state);
}

/// Render the tool browser widget with search and details
pub fn render_tool_browser(
    frame: &mut Frame<'_>,
    area: Rect,
    tools: &[ToolInfo],
    cursor: usize,
    filter_text: &str,
    focused: bool,
) {
    // Split area into search bar and tool list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Render search bar
    let search_block = Block::default()
        .title("Search")
        .borders(Borders::ALL)
        .border_style(if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });
    let search_text = format!("🔍 {}", filter_text);
    let search_para = Paragraph::new(search_text).block(search_block);
    frame.render_widget(search_para, chunks[0]);

    // Filter tools based on search text
    let filtered_tools: Vec<&ToolInfo> = if filter_text.is_empty() {
        tools.iter().collect()
    } else {
        let filter_lower = filter_text.to_lowercase();
        tools
            .iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&filter_lower)
                    || t.description.to_lowercase().contains(&filter_lower)
            })
            .collect()
    };

    // Render filtered tool list
    let items: Vec<ListItem> = filtered_tools
        .iter()
        .map(|tool| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    tool.name.clone(),
                    Style::default()
                        .fg(tool.category.color())
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("[{}]", tool.category.display_name()),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(
                    tool.description.clone(),
                    Style::default().fg(Color::Gray),
                )),
            ])
        })
        .collect();

    let list_block = Block::default()
        .title(format!("Tools ({} found)", filtered_tools.len()))
        .borders(Borders::ALL);

    let list = List::new(items)
        .block(list_block)
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("◆ ");

    let mut state = ListState::default();
    if !filtered_tools.is_empty() {
        state.select(Some(cursor.min(filtered_tools.len() - 1)));
    }
    frame.render_stateful_widget(list, chunks[1], &mut state);
}

/// Render the call history widget
pub fn render_call_history(
    frame: &mut Frame<'_>,
    area: Rect,
    history: &[ToolCall],
    cursor: usize,
    focused: bool,
) {
    let items: Vec<ListItem> = history
        .iter()
        .map(|call| {
            let timestamp = call.timestamp.with_timezone(&Local).format("%H:%M:%S");
            let status_style = if let Some(result) = &call.result {
                if result.success {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                }
            } else {
                Style::default().fg(Color::Yellow)
            };

            let status_text = if let Some(result) = &call.result {
                if result.success {
                    format!("✓ {}ms", result.duration.as_millis())
                } else {
                    format!("✗ {}", result.error.as_deref().unwrap_or("failed"))
                }
            } else {
                "⧗ running".to_string()
            };

            ListItem::new(vec![
                Line::from(vec![
                    Span::styled(timestamp.to_string(), Style::default().fg(Color::DarkGray)),
                    Span::raw(" • "),
                    Span::styled(call.tool_name.clone(), Style::default().fg(Color::Cyan)),
                ]),
                Line::from(Span::styled(status_text, status_style)),
            ])
        })
        .collect();

    let mut block = Block::default()
        .title(format!("Call History ({})", history.len()))
        .borders(Borders::ALL);
    if focused {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    if !history.is_empty() {
        state.select(Some(cursor.min(history.len() - 1)));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

/// Render the agent status bar
pub fn render_agent_status(
    frame: &mut Frame<'_>,
    area: Rect,
    agent_mode: bool,
    sandbox_path: &str,
    tool_count: usize,
    active_calls: usize,
) {
    let mode_indicator = if agent_mode {
        Span::styled(
            " AGENT ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" OFF ", Style::default().fg(Color::DarkGray))
    };

    let status_text = vec![
        mode_indicator,
        Span::raw(" | "),
        Span::styled(
            format!("sandbox: {}", sandbox_path),
            Style::default().fg(Color::Blue),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("tools: {}", tool_count),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("active: {}", active_calls),
            if active_calls > 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            },
        ),
    ];

    let paragraph = Paragraph::new(Line::from(status_text))
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default());

    frame.render_widget(paragraph, area);
}

/// Helper function to create sample tools for testing
pub fn sample_tools() -> Vec<ToolInfo> {
    vec![
        ToolInfo::new(
            "ls".to_string(),
            "List directory contents".to_string(),
            ToolCategory::FileSystem,
        ),
        ToolInfo::new(
            "cat".to_string(),
            "Display file contents".to_string(),
            ToolCategory::FileSystem,
        ),
        ToolInfo::new(
            "grep".to_string(),
            "Search text patterns".to_string(),
            ToolCategory::TextProcessing,
        ),
        ToolInfo::new(
            "ps".to_string(),
            "Display process status".to_string(),
            ToolCategory::ProcessManagement,
        ),
        ToolInfo::new(
            "curl".to_string(),
            "Transfer data from URLs".to_string(),
            ToolCategory::Network,
        ),
        ToolInfo::new(
            "uname".to_string(),
            "Print system information".to_string(),
            ToolCategory::SystemInfo,
        ),
    ]
}
