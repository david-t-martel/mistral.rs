use chrono::Local;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{App, FocusArea},
    session::SessionMessage,
};

#[cfg(feature = "tui-agent")]
use crate::agent::ui::{
    render_agent_status, render_call_history, render_execution_panel, render_tool_browser,
    render_tool_panel, render_tool_palette,
};

pub fn render(frame: &mut Frame<'_>, app: &App) {
    let size = frame.area();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(size);

    #[cfg(feature = "tui-agent")]
    if app.is_agent_mode() {
        render_agent_layout(frame, layout[0], layout[1], app);
        
        // Render palette overlay if visible
        if app.agent_ui_state().palette_visible {
            let tools = app.available_tools();
            let palette_filter = &app.agent_ui_state().palette_filter;
            let palette_cursor = app.agent_ui_state().palette_cursor;
            render_tool_palette(frame, size, tools, palette_filter, palette_cursor);
        }
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(32),
            Constraint::Min(45),
            Constraint::Length(36),
        ])
        .split(layout[0]);

    render_sessions(frame, main_chunks[0], app);
    render_chat(frame, main_chunks[1], app);
    render_models(frame, main_chunks[2], app);
    render_status(frame, layout[1], app);
}

fn render_sessions(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .sessions()
        .iter()
        .map(|session| {
            let updated = session.updated_at.with_timezone(&Local);
            let subtitle = format!(
                "{} • {} tokens",
                updated.format("%b %d %H:%M"),
                session.token_count
            );
            let model = session.model_id.as_deref().unwrap_or("<model unspecified>");
            ListItem::new(vec![
                Line::from(Span::raw(session.title.clone())),
                Line::from(Span::styled(
                    model.to_string(),
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(Span::styled(subtitle, Style::default().fg(Color::Gray))),
            ])
        })
        .collect();

    let mut block = Block::default().title("Sessions").borders(Borders::ALL);
    if matches!(app.focus(), FocusArea::Sessions) {
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
    state.select(Some(app.session_cursor()));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_chat(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines: Vec<Line> = app
        .active_session()
        .messages
        .iter()
        .flat_map(render_message_lines)
        .collect();

    let mut block = Block::default().title("Conversation").borders(Borders::ALL);
    if matches!(app.focus(), FocusArea::Chat) {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_models(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let entries = app.model_entries();
    let items: Vec<ListItem> = entries
        .iter()
        .map(|entry| {
            let subtitle = entry
                .size_bytes
                .map(format_size)
                .unwrap_or_else(|| "unknown size".to_string());
            let format_line = entry
                .format
                .as_deref()
                .map(|fmt| format!("format: {}", fmt))
                .unwrap_or_else(|| "format: n/a".to_string());
            ListItem::new(vec![
                Line::from(Span::raw(entry.display_name().to_string())),
                Line::from(Span::styled(format_line, Style::default().fg(Color::Gray))),
                Line::from(Span::styled(subtitle, Style::default().fg(Color::DarkGray))),
            ])
        })
        .collect();

    let mut block = Block::default().title("Models").borders(Borders::ALL);
    if matches!(app.focus(), FocusArea::Models) {
        block = block.border_style(Style::default().fg(Color::Cyan));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("◆ ");

    let mut state = ListState::default();
    state.select(Some(app.model_cursor()));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_status(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let metrics = app.metrics();
    let status_text = format!(
        "{} | total tokens: {}",
        app.status_line(),
        metrics.total_tokens
    );

    let paragraph = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(paragraph, area);
}

fn render_message_lines(message: &SessionMessage) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let role_style = match message.role.as_str() {
        "system" => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        "assistant" => Style::default().fg(Color::Green),
        "user" => Style::default().fg(Color::Cyan),
        _ => Style::default().fg(Color::White),
    };

    let header = format!(
        "{} • {}",
        message.role,
        message.created_at.with_timezone(&Local).format("%H:%M:%S")
    );
    lines.push(Line::from(Span::styled(header, role_style)));

    for content_line in message.content.lines() {
        lines.push(Line::from(Span::raw(content_line.to_string())));
    }

    if let Some(tokens) = message.token_count {
        lines.push(Line::from(Span::styled(
            format!("tokens: {}", tokens),
            Style::default().fg(Color::DarkGray),
        )));
    }

    lines.push(Line::from(Span::raw(String::new())));
    lines
}

fn format_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }
    format!("{value:.1} {}", UNITS[idx])
}

#[cfg(feature = "tui-agent")]
fn render_agent_layout(frame: &mut Frame<'_>, main_area: Rect, status_area: Rect, app: &App) {
    let agent_ui_state = app.agent_ui_state();

    // Check if execution panel should be shown
    let show_execution_panel =
        agent_ui_state.execution_panel_visible && agent_ui_state.active_execution.is_some();

    // Handle execution panel layout specially
    if show_execution_panel {
        // Execution mode: Split vertically to show execution panel at bottom
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(main_area);

        // Top section: Sessions | Chat | ToolPanel
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),
                Constraint::Min(30),
                Constraint::Length(32),
            ])
            .split(vertical_chunks[0]);

        // Render top section
        render_sessions(frame, top_chunks[0], app);
        render_chat(frame, top_chunks[1], app);

        let tools = app.available_tools();
        let tool_cursor = agent_ui_state.tool_cursor;
        let tool_focused = matches!(app.focus(), FocusArea::AgentTools);
        render_tool_panel(frame, top_chunks[2], tools, tool_cursor, tool_focused);

        // Render execution panel at bottom
        if let Some(execution) = &agent_ui_state.active_execution {
            render_execution_panel(frame, vertical_chunks[1], execution, false);
        }

        // Render agent status bar
        let toolkit = app.agent_toolkit();
        let sandbox_path = toolkit
            .map(|t| t.config().sandbox_root.display().to_string())
            .unwrap_or_else(|| ".".to_string());
        let tool_count = toolkit.map(|t| t.tool_count()).unwrap_or(0);
        let active_calls = app.active_session().tool_calls.len();

        render_agent_status(
            frame,
            status_area,
            true,
            &sandbox_path,
            tool_count,
            active_calls,
        );
        return;
    }

    // Determine layout based on browser visibility
    let main_chunks = if agent_ui_state.browser_visible {
        // Browser mode: Sessions | Chat | ToolBrowser
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(20),
                Constraint::Min(30),
                Constraint::Length(56),
            ])
            .split(main_area)
    } else {
        // Normal agent mode: Sessions | Chat | ToolPanel | CallHistory
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(24),
                Constraint::Min(35),
                Constraint::Length(28),
                Constraint::Length(28),
            ])
            .split(main_area)
    };

    // Always render sessions and chat
    render_sessions(frame, main_chunks[0], app);
    render_chat(frame, main_chunks[1], app);

    // Render agent UI based on visibility
    if agent_ui_state.browser_visible {
        let tools = app.available_tools();
        let cursor = agent_ui_state.tool_cursor;
        let focused = matches!(app.focus(), FocusArea::AgentBrowser);
        render_tool_browser(
            frame,
            main_chunks[2],
            tools,
            cursor,
            &agent_ui_state.filter_text,
            focused,
        );
    } else {
        // Render tool panel and call history
        let tools = app.available_tools();
        let tool_cursor = agent_ui_state.tool_cursor;
        let tool_focused = matches!(app.focus(), FocusArea::AgentTools);
        render_tool_panel(frame, main_chunks[2], tools, tool_cursor, tool_focused);

        let history = &app.active_session().tool_calls;
        let history_cursor = agent_ui_state.history_cursor;
        let history_focused = matches!(app.focus(), FocusArea::AgentHistory);
        render_call_history(
            frame,
            main_chunks[3],
            history,
            history_cursor,
            history_focused,
        );
    }

    // Render agent status bar
    let toolkit = app.agent_toolkit();
    let sandbox_path = toolkit
        .map(|t| t.config().sandbox_root.display().to_string())
        .unwrap_or_else(|| ".".to_string());
    let tool_count = toolkit.map(|t| t.tool_count()).unwrap_or(0);
    let active_calls = app.active_session().tool_calls.len();

    render_agent_status(
        frame,
        status_area,
        true,
        &sandbox_path,
        tool_count,
        active_calls,
    );
}
