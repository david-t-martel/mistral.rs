//! Core state machine powering the terminal UI.
//!
//! The [`App`] struct owns session metadata, model inventory state, metrics and
//! input handling. Rendering code consumes read-only snapshots to draw the UI.

use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use tokio::runtime::Runtime;

use crate::{
    input::{InputEvent, KeyCode, KeyEvent},
    inventory::{ModelEntry, ModelInventory},
    session::{SessionContext, SessionStore, SessionSummary},
};

#[cfg(feature = "tui-agent")]
use crate::agent::{
    toolkit::AgentToolkit,
    ui::{sample_tools, AgentUiState, ToolInfo},
    EventBus, ExecutionEvent,
};

/// High-level focus targets inside the UI layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusArea {
    Sessions,
    Chat,
    Models,
    #[cfg(feature = "tui-agent")]
    AgentTools,
    #[cfg(feature = "tui-agent")]
    AgentBrowser,
    #[cfg(feature = "tui-agent")]
    AgentHistory,
}

/// Runtime telemetry surfaced to the status bar.
#[derive(Debug, Clone)]
pub struct Metrics {
    pub total_tokens: u64,
    pub last_update: DateTime<Utc>,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            total_tokens: 0,
            last_update: Utc::now(),
        }
    }
}

/// Helper used by the status bar to communicate user-facing updates.
#[derive(Debug, Clone)]
pub struct StatusLine {
    message: String,
}

impl StatusLine {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn set(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Aggregates mutable UI state and orchestrates requests into the inference
/// backend.
pub struct App {
    session_store: Arc<SessionStore>,
    model_inventory: Arc<ModelInventory>,
    focus: FocusArea,
    metrics: Metrics,
    status: StatusLine,
    should_quit: bool,
    sessions: Vec<SessionSummary>,
    session_cursor: usize,
    active_session: SessionContext,
    model_cursor: usize,
    #[cfg(feature = "tui-agent")]
    agent_toolkit: Option<AgentToolkit>,
    #[cfg(feature = "tui-agent")]
    agent_ui_state: AgentUiState,
    #[cfg(feature = "tui-agent")]
    available_tools: Vec<ToolInfo>,
    #[cfg(feature = "tui-agent")]
    event_bus: Option<EventBus>,
    #[cfg(feature = "tui-agent")]
    event_receiver: Option<tokio::sync::broadcast::Receiver<ExecutionEvent>>,
}

impl App {
    /// Build a fresh [`App`] by loading the latest sessions and models from disk.
    pub async fn initialise(
        session_store: Arc<SessionStore>,
        model_inventory: Arc<ModelInventory>,
        default_model: Option<String>,
    ) -> Result<Self> {
        let mut sessions = session_store.list_recent_sessions(32).await?;
        let active_session = if let Some(summary) = sessions.first() {
            session_store.load_session(summary.id).await?
        } else {
            let model_id = default_model
                .or_else(|| model_inventory.default_model_id())
                .unwrap_or_else(|| "unknown".to_string());
            let session = session_store
                .create_session(&model_id, "New Session")
                .await?;
            sessions.insert(0, session.summary.clone());
            session
        };

        #[cfg(feature = "tui-agent")]
        let (event_bus, event_receiver) = {
            let bus = EventBus::new(100);
            let receiver = bus.subscribe();
            (Some(bus), Some(receiver))
        };

        Ok(Self {
            session_store,
            model_inventory,
            focus: FocusArea::Chat,
            metrics: Metrics::new(),
            status: StatusLine::new("Ready"),
            should_quit: false,
            sessions,
            session_cursor: 0,
            active_session,
            model_cursor: 0,
            #[cfg(feature = "tui-agent")]
            agent_toolkit: AgentToolkit::with_defaults().ok(),
            #[cfg(feature = "tui-agent")]
            agent_ui_state: AgentUiState::new(),
            #[cfg(feature = "tui-agent")]
            available_tools: sample_tools(),
            #[cfg(feature = "tui-agent")]
            event_bus,
            #[cfg(feature = "tui-agent")]
            event_receiver,
        })
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    pub fn status_line(&self) -> &str {
        self.status.message()
    }

    pub fn focus(&self) -> FocusArea {
        self.focus
    }

    pub fn sessions(&self) -> &[SessionSummary] {
        &self.sessions
    }

    pub fn session_cursor(&self) -> usize {
        self.session_cursor
    }

    pub fn active_session(&self) -> &SessionContext {
        &self.active_session
    }

    pub fn model_entries(&self) -> Vec<ModelEntry> {
        self.model_inventory.entries()
    }

    pub fn model_cursor(&self) -> usize {
        self.model_cursor
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    #[cfg(feature = "tui-agent")]
    pub fn agent_ui_state(&self) -> &AgentUiState {
        &self.agent_ui_state
    }

    #[cfg(feature = "tui-agent")]
    pub fn agent_ui_state_mut(&mut self) -> &mut AgentUiState {
        &mut self.agent_ui_state
    }

    #[cfg(feature = "tui-agent")]
    pub fn available_tools(&self) -> &[ToolInfo] {
        &self.available_tools
    }

    #[cfg(feature = "tui-agent")]
    pub fn agent_toolkit(&self) -> Option<&AgentToolkit> {
        self.agent_toolkit.as_ref()
    }

    #[cfg(feature = "tui-agent")]
    pub fn is_agent_mode(&self) -> bool {
        self.active_session.agent_mode
    }

    pub fn handle_event(&mut self, event: InputEvent, runtime: &Runtime) -> Result<()> {
        match event {
            InputEvent::Tick => {
                self.update_metrics_display();
                #[cfg(feature = "tui-agent")]
                self.poll_execution_events();
            }
            InputEvent::Resize(_, _) => {
                // Nothing to do yet, layout is responsive.
            }
            InputEvent::Key(key) => self.handle_key(key, runtime)?,
        }
        Ok(())
    }

    pub fn tick(&mut self, runtime: &Runtime) -> Result<()> {
        self.handle_event(InputEvent::Tick, runtime)
    }

    fn handle_key(&mut self, key: KeyEvent, runtime: &Runtime) -> Result<()> {
        if key.modifiers.control && matches!(key.code, KeyCode::Char('c')) {
            self.should_quit = true;
            return Ok(());
        }

        // Handle palette input when visible
        #[cfg(feature = "tui-agent")]
        if self.is_agent_mode() && self.agent_ui_state.palette_visible {
            return self.handle_palette_key(key);
        }

        // Agent mode key bindings
        #[cfg(feature = "tui-agent")]
        if key.modifiers.control {
            match key.code {
                KeyCode::Char('a') => {
                    self.toggle_agent_mode();
                    return Ok(());
                }
                KeyCode::Char('p') => {
                    if self.is_agent_mode() {
                        self.agent_ui_state.toggle_palette();
                    }
                    return Ok(());
                }
                KeyCode::Char('t') => {
                    self.agent_ui_state.toggle_panel();
                    if self.agent_ui_state.panel_visible {
                        self.focus = FocusArea::AgentTools;
                    }
                    return Ok(());
                }
                KeyCode::Char('b') => {
                    self.agent_ui_state.toggle_browser();
                    if self.agent_ui_state.browser_visible {
                        self.focus = FocusArea::AgentBrowser;
                    }
                    return Ok(());
                }
                KeyCode::Char('h') => {
                    self.agent_ui_state.toggle_history();
                    if self.agent_ui_state.history_visible {
                        self.focus = FocusArea::AgentHistory;
                    }
                    return Ok(());
                }
                _ => {}
            }
        }

        match key.code {
            KeyCode::Esc => {
                #[cfg(feature = "tui-agent")]
                if self.is_agent_mode() && self.agent_ui_state.palette_visible {
                    self.agent_ui_state.hide_palette();
                    return Ok(());
                }
                self.focus = FocusArea::Chat;
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('n') => {
                self.spawn_new_session(runtime)?;
            }
            KeyCode::Char('r') => {
                self.refresh_inventory()?;
            }
            KeyCode::Tab => {
                self.cycle_focus();
            }
            KeyCode::Up => {
                self.move_cursor(-1)?;
            }
            KeyCode::Down => {
                self.move_cursor(1)?;
            }
            KeyCode::Enter => {
                if self.focus == FocusArea::Sessions {
                    self.activate_cursor_session(runtime)?;
                } else if self.focus == FocusArea::Models {
                    self.attach_model_to_session(runtime)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn cycle_focus(&mut self) {
        #[cfg(feature = "tui-agent")]
        if self.is_agent_mode() {
            self.focus = match self.focus {
                FocusArea::Chat => FocusArea::Sessions,
                FocusArea::Sessions => FocusArea::AgentTools,
                FocusArea::AgentTools => FocusArea::AgentHistory,
                FocusArea::AgentHistory => FocusArea::Chat,
                _ => FocusArea::Chat,
            };
            return;
        }

        self.focus = match self.focus {
            FocusArea::Chat => FocusArea::Sessions,
            FocusArea::Sessions => FocusArea::Models,
            FocusArea::Models => FocusArea::Chat,
            #[cfg(feature = "tui-agent")]
            _ => FocusArea::Chat,
        };
    }

    fn move_cursor(&mut self, delta: isize) -> Result<()> {
        match self.focus {
            FocusArea::Sessions => {
                if self.sessions.is_empty() {
                    return Ok(());
                }
                let len = self.sessions.len() as isize;
                let new_index = (self.session_cursor as isize + delta).rem_euclid(len);
                self.session_cursor = new_index as usize;
            }
            FocusArea::Models => {
                let models = self.model_inventory.entries();
                if models.is_empty() {
                    return Ok(());
                }
                let len = models.len() as isize;
                let new_index = (self.model_cursor as isize + delta).rem_euclid(len);
                self.model_cursor = new_index as usize;
            }
            #[cfg(feature = "tui-agent")]
            FocusArea::AgentTools => {
                if self.available_tools.is_empty() {
                    return Ok(());
                }
                let len = self.available_tools.len() as isize;
                let new_index = (self.agent_ui_state.tool_cursor as isize + delta).rem_euclid(len);
                self.agent_ui_state.tool_cursor = new_index as usize;
            }
            #[cfg(feature = "tui-agent")]
            FocusArea::AgentHistory => {
                let history = &self.active_session.tool_calls;
                if history.is_empty() {
                    return Ok(());
                }
                let len = history.len() as isize;
                let new_index =
                    (self.agent_ui_state.history_cursor as isize + delta).rem_euclid(len);
                self.agent_ui_state.history_cursor = new_index as usize;
            }
            _ => {}
        }
        Ok(())
    }

    fn activate_cursor_session(&mut self, runtime: &Runtime) -> Result<()> {
        if self.session_cursor >= self.sessions.len() {
            return Ok(());
        }
        let summary = self.sessions[self.session_cursor].clone();
        let ctx = runtime.block_on(self.session_store.load_session(summary.id))?;
        self.active_session = ctx;
        self.metrics.total_tokens = self
            .active_session
            .messages
            .iter()
            .filter_map(|m| m.token_count)
            .sum::<i64>() as u64;
        self.status
            .set(format!("Switched to session '{}'.", summary.title));
        Ok(())
    }

    fn spawn_new_session(&mut self, runtime: &Runtime) -> Result<()> {
        let model_id = self
            .model_inventory
            .default_model_id()
            .or_else(|| self.active_session.summary.model_id.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let session =
            runtime.block_on(self.session_store.create_session(&model_id, "New Session"))?;
        self.sessions.insert(0, session.summary.clone());
        self.session_cursor = 0;
        self.active_session = session;
        self.status.set("Started new session");
        Ok(())
    }

    fn refresh_inventory(&mut self) -> Result<()> {
        self.model_inventory.refresh()?;
        self.status
            .set(format!("Discovered {} models", self.model_inventory.len()));
        Ok(())
    }

    #[cfg(feature = "tui-agent")]
    fn toggle_agent_mode(&mut self) {
        self.active_session.agent_mode = !self.active_session.agent_mode;
        let mode_str = if self.active_session.agent_mode {
            "enabled"
        } else {
            "disabled"
        };
        self.status.set(format!("Agent mode {}", mode_str));
        // Reset focus to chat when toggling
        self.focus = FocusArea::Chat;
        // Reset agent UI state when disabling
        if !self.active_session.agent_mode {
            self.agent_ui_state.reset();
        }
    }

    fn attach_model_to_session(&mut self, runtime: &Runtime) -> Result<()> {
        let models = self.model_inventory.entries();
        if models.is_empty() {
            self.status.set("No models available to attach");
            return Ok(());
        }
        let selected = &models[self.model_cursor];
        runtime.block_on(
            self.session_store
                .update_session_model(self.active_session.summary.id, &selected.identifier),
        )?;
        self.active_session.summary.model_id = Some(selected.identifier.clone());
        self.status.set(format!(
            "Session switched to model '{}'.",
            selected.display_name()
        ));
        Ok(())
    }

    fn update_metrics_display(&mut self) {
        self.metrics.last_update = Utc::now();
        let session_tokens: u64 = self
            .active_session
            .messages
            .iter()
            .filter_map(|m| m.token_count)
            .map(|v| v.max(0) as u64)
            .sum();
        self.metrics.total_tokens = session_tokens;
    }

    /// Handle keyboard input when palette is visible
    #[cfg(feature = "tui-agent")]
    fn handle_palette_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.agent_ui_state.hide_palette();
            }
            KeyCode::Enter => {
                // Get the selected tool and execute it
                // For now, just close the palette
                // TODO: Implement tool execution from palette
                self.agent_ui_state.hide_palette();
            }
            KeyCode::Up => {
                if self.agent_ui_state.palette_cursor > 0 {
                    self.agent_ui_state.palette_cursor -= 1;
                }
            }
            KeyCode::Down => {
                // Filter tools and check bounds
                let filter = self.agent_ui_state.palette_filter.to_lowercase();
                let filtered_count = self.available_tools.iter()
                    .filter(|t| {
                        filter.is_empty() ||
                        t.name.to_lowercase().contains(&filter) ||
                        t.description.to_lowercase().contains(&filter)
                    })
                    .count();
                
                if filtered_count > 0 && self.agent_ui_state.palette_cursor < filtered_count - 1 {
                    self.agent_ui_state.palette_cursor += 1;
                }
            }
            KeyCode::Backspace => {
                self.agent_ui_state.palette_filter.pop();
                // Reset cursor when filter changes
                self.agent_ui_state.palette_cursor = 0;
            }
            KeyCode::Char(c) => {
                self.agent_ui_state.palette_filter.push(c);
                // Reset cursor when filter changes
                self.agent_ui_state.palette_cursor = 0;
            }
            _ => {}
        }
        Ok(())
    }

    /// Poll for execution events and update UI state
    #[cfg(feature = "tui-agent")]
    fn poll_execution_events(&mut self) {
        if let Some(receiver) = self.event_receiver.as_mut() {
            // Try to receive all pending events without blocking
            while let Ok(event) = receiver.try_recv() {
                self.agent_ui_state.update_from_event(&event);
            }
        }
    }

    /// Get the event bus (for tool execution)
    #[cfg(feature = "tui-agent")]
    pub fn event_bus(&self) -> Option<&EventBus> {
        self.event_bus.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn runtime() -> Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }

    fn make_app(rt: &Runtime) -> App {
        let tmp = tempdir().expect("tempdir");
        let db_path = tmp.path().join("tui_sessions.sqlite");
        let store = rt.block_on(SessionStore::new(&db_path)).expect("store");
        let store = Arc::new(store);
        let inventory = Arc::new(ModelInventory::new(vec![], None));
        inventory.refresh().expect("inventory refresh");
        rt.block_on(App::initialise(store, inventory, None))
            .expect("initialise")
    }

    #[test]
    fn focus_cycles_in_order() {
        let rt = runtime();
        let mut app = make_app(&rt);
        assert_eq!(app.focus(), FocusArea::Chat);
        app.cycle_focus();
        assert_eq!(app.focus(), FocusArea::Sessions);
        app.cycle_focus();
        assert_eq!(app.focus(), FocusArea::Models);
        app.cycle_focus();
        assert_eq!(app.focus(), FocusArea::Chat);
    }

    #[test]
    fn moving_session_cursor_wraps() {
        let rt = runtime();
        let mut app = make_app(&rt);
        // Ensure there is at least one additional session to navigate.
        rt.block_on(app.session_store.create_session("model", "Extra"))
            .unwrap();
        rt.block_on(app.session_store.create_session("model", "Extra2"))
            .unwrap();
        // Refresh local cache.
        app.sessions = rt
            .block_on(app.session_store.list_recent_sessions(32))
            .unwrap();
        app.session_cursor = 0;
        app.focus = FocusArea::Sessions;
        app.move_cursor(1).unwrap();
        assert_eq!(app.session_cursor, 1 % app.sessions.len());
        app.move_cursor(-1).unwrap();
        assert_eq!(app.session_cursor, 0);
    }
}
