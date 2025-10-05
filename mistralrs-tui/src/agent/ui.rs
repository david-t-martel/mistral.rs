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
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::time::Instant;

use super::events::ExecutionEvent;
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
    /// Active tool execution (if any)
    pub active_execution: Option<ActiveExecution>,
    /// Whether execution panel is visible
    pub execution_panel_visible: bool,
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
            active_execution: None,
            execution_panel_visible: false,
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

    /// Start tracking a new tool execution
    pub fn start_execution(
        &mut self,
        call_id: uuid::Uuid,
        tool_name: String,
        arguments: serde_json::Value,
    ) {
        self.active_execution = Some(ActiveExecution::new(call_id, tool_name, arguments));
        self.execution_panel_visible = true;
    }

    /// Update active execution from event
    pub fn update_execution(&mut self, event: &ExecutionEvent) {
        if let Some(execution) = &mut self.active_execution {
            execution.update_from_event(event);
        }
    }

    /// Clear active execution
    pub fn clear_execution(&mut self) {
        self.active_execution = None;
        self.execution_panel_visible = false;
    }

    /// Toggle execution panel visibility
    pub fn toggle_execution_panel(&mut self) {
        self.execution_panel_visible = !self.execution_panel_visible;
    }

    /// Update UI state from execution event
    pub fn update_from_event(&mut self, event: &ExecutionEvent) {
        match event {
            ExecutionEvent::Started {
                call_id, tool_name, ..
            } => {
                // Start new execution tracking
                let call_id_copy = *call_id;
                self.start_execution(
                    call_id_copy,
                    tool_name.clone(),
                    serde_json::Value::Null, // Arguments will be known from tool call history
                );
            }
            ExecutionEvent::Progress { call_id, .. }
            | ExecutionEvent::Completed { call_id, .. }
            | ExecutionEvent::Failed { call_id, .. } => {
                // Update existing execution if call_id matches
                if let Some(execution) = &mut self.active_execution {
                    if execution.call_id == *call_id {
                        execution.update_from_event(event);

                        // Clear execution panel after completion
                        if execution.completed {
                            // Keep panel visible for a moment to show final state
                            // UI tick will eventually clear it or user can dismiss
                        }
                    }
                }
            }
        }
    }
}

/// Active tool execution state for real-time display
#[derive(Debug, Clone)]
pub struct ActiveExecution {
    /// Tool call ID
    pub call_id: uuid::Uuid,
    /// Tool name
    pub tool_name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
    /// Execution start time
    pub started_at: Instant,
    /// Current progress (0.0 to 1.0)
    pub progress: f64,
    /// Streaming output lines
    pub output_lines: Vec<String>,
    /// Current status message
    pub status_message: String,
    /// Whether execution has completed
    pub completed: bool,
    /// Whether execution succeeded
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
}

impl ActiveExecution {
    /// Create new active execution from tool call
    pub fn new(call_id: uuid::Uuid, tool_name: String, arguments: serde_json::Value) -> Self {
        Self {
            call_id,
            tool_name,
            arguments,
            started_at: Instant::now(),
            progress: 0.0,
            output_lines: Vec::new(),
            status_message: "Starting...".to_string(),
            completed: false,
            success: false,
            error_message: None,
        }
    }

    /// Update progress from event
    pub fn update_from_event(&mut self, event: &ExecutionEvent) {
        match event {
            ExecutionEvent::Started { .. } => {
                self.status_message = "Executing...".to_string();
                self.progress = 0.1;
            }
            ExecutionEvent::Progress { message, .. } => {
                self.status_message = message.clone();
                self.progress = (self.progress + 0.1).min(0.9);
            }
            ExecutionEvent::Completed { result, .. } => {
                self.completed = true;
                self.success = result.success;
                self.progress = 1.0;
                if result.success {
                    self.status_message = "Completed successfully".to_string();
                    if let serde_json::Value::String(output) = &result.output {
                        self.output_lines.extend(output.lines().map(String::from));
                    }
                } else {
                    self.status_message = "Failed".to_string();
                    self.error_message = result.error.clone();
                }
            }
            ExecutionEvent::Failed { error, .. } => {
                self.completed = true;
                self.success = false;
                self.progress = 1.0;
                self.status_message = "Failed".to_string();
                self.error_message = Some(error.clone());
            }
        }
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u128 {
        self.started_at.elapsed().as_millis()
    }

    /// Get spinner character based on elapsed time
    pub fn spinner_char(&self) -> &'static str {
        const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = (self.elapsed_ms() / 100) as usize % SPINNER.len();
        SPINNER[idx]
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

/// Render the tool execution panel with real-time progress
pub fn render_execution_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    execution: &ActiveExecution,
    focused: bool,
) {
    // Split area into header, progress, output
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with tool name and status
            Constraint::Length(3), // Progress bar
            Constraint::Min(5),    // Output/details
        ])
        .split(area);

    // Render header with spinner or status icon
    let status_icon = if !execution.completed {
        execution.spinner_char()
    } else if execution.success {
        "✓"
    } else {
        "✗"
    };

    let status_color = if !execution.completed {
        Color::Yellow
    } else if execution.success {
        Color::Green
    } else {
        Color::Red
    };

    let header_text = vec![
        Span::styled(
            status_icon,
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            execution.tool_name.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" • "),
        Span::styled(
            format!("{}ms", execution.elapsed_ms()),
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let header_block = Block::default()
        .title("Tool Execution")
        .borders(Borders::ALL)
        .border_style(if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default()
        });

    let header_para = Paragraph::new(Line::from(header_text)).block(header_block);
    frame.render_widget(header_para, chunks[0]);

    // Render progress bar
    let progress_label = execution.status_message.clone();
    let progress_gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(if execution.success {
                    Color::Green
                } else if execution.completed {
                    Color::Red
                } else {
                    Color::Yellow
                })
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .label(progress_label)
        .ratio(execution.progress);

    frame.render_widget(progress_gauge, chunks[1]);

    // Render output or error
    let mut output_lines = Vec::new();

    if let Some(error) = &execution.error_message {
        output_lines.push(Line::from(Span::styled(
            format!("Error: {}", error),
            Style::default().fg(Color::Red),
        )));
    }

    // Show arguments
    output_lines.push(Line::from(Span::styled(
        "Arguments:",
        Style::default().fg(Color::Gray),
    )));
    if let Some(args_str) = serde_json::to_string_pretty(&execution.arguments).ok() {
        for line in args_str.lines().take(5) {
            output_lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    // Show output if available
    if !execution.output_lines.is_empty() {
        output_lines.push(Line::from(Span::raw("")));
        output_lines.push(Line::from(Span::styled(
            "Output:",
            Style::default().fg(Color::Gray),
        )));
        for line in execution.output_lines.iter().take(10) {
            output_lines.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(Color::White),
            )));
        }
    }

    let output_block = Block::default().title("Details").borders(Borders::ALL);

    let output_para = Paragraph::new(output_lines)
        .block(output_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(output_para, chunks[2]);
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
