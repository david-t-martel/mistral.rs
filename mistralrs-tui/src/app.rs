//! Core state machine powering the terminal UI.
//!
//! The [`App`] struct owns session metadata, model inventory state, metrics and
//! input handling. Rendering code consumes read-only snapshots to draw the UI.
//!
//! ### Implementation follow-ups
//! * Honour `TuiConfig::agent` and only spin up agent tooling when explicitly enabled.
//! * Push backend fallback and runtime errors into [`StatusLine`] updates so the user sees them.
//! * Replace the blocking calls inside key handlers with async helpers to keep the UI responsive
//!   during slow session loads/model attaches.

use std::sync::Arc;

#[cfg(feature = "tui-agent")]
use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use tokio::runtime::Runtime;

use crate::{
    input::{InputEvent, KeyCode, KeyEvent},
    inventory::{ModelEntry, ModelInventory},
    session::{SessionContext, SessionStore, SessionSummary},
};

#[cfg(feature = "tui-agent")]
use crate::config::AgentConfig as AgentPreferences;

#[cfg(feature = "tui-agent")]
#[cfg(feature = "tui-agent")]
use serde_json::{json, Value};

#[cfg(feature = "tui-agent")]
use crate::agent::{
    execution::ToolExecutor,
    toolkit::{AgentToolkit, ToolCallResult},
    ui::{default_tools, AgentUiState, ToolInfo},
    EventBus, ExecutionEvent,
};

#[cfg(feature = "tui-agent")]
#[derive(Clone)]
enum ToolInputKind {
    PathsArray { key: &'static str },
    StringField { key: &'static str },
}

#[cfg(feature = "tui-agent")]
#[derive(Clone)]
enum ToolArgumentState {
    Ready(Value),
    NeedsInput(ToolArgumentRequest),
}

#[cfg(feature = "tui-agent")]
#[derive(Clone)]
struct ToolArgumentRequest {
    prompt: String,
    hint: Option<String>,
    kind: ToolInputKind,
    base_arguments: Value,
}

#[cfg(feature = "tui-agent")]
#[derive(Clone)]
struct PendingExecution {
    tool: ToolInfo,
    request: ToolArgumentRequest,
}

#[cfg(feature = "tui-agent")]
#[derive(Clone)]
struct ModelPreset {
    model_path: PathBuf,
    directory: PathBuf,
    framework: String,
}

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
    #[cfg(feature = "tui-agent")]
    agent_config: AgentPreferences,
    #[cfg(feature = "tui-agent")]
    pending_execution: Option<PendingExecution>,
}

impl App {
    /// Build a fresh [`App`] by loading the latest sessions and models from disk.
    pub async fn initialise(
        session_store: Arc<SessionStore>,
        model_inventory: Arc<ModelInventory>,
        default_model: Option<String>,
        #[cfg(feature = "tui-agent")] agent_config: Option<AgentPreferences>,
    ) -> Result<Self> {
        let mut sessions = session_store.list_recent_sessions(32).await?;
        #[cfg_attr(not(feature = "tui-agent"), allow(unused_mut))]
        let mut active_session = if let Some(summary) = sessions.first() {
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
        let agent_config = agent_config.unwrap_or_default();
        #[cfg(feature = "tui-agent")]
        {
            if agent_config.max_history > 0
                && active_session.tool_calls.len() > agent_config.max_history
            {
                let remove = active_session.tool_calls.len() - agent_config.max_history;
                active_session.tool_calls.drain(0..remove);
            }
        }
        #[cfg(feature = "tui-agent")]
        let (
            agent_toolkit,
            mut agent_ui_state,
            available_tools,
            event_bus,
            event_receiver,
            agent_status,
        ) = {
            use crate::agent::toolkit::ToolkitConfig;

            let mut toolkit_config = ToolkitConfig::default();
            if let Some(root) = agent_config.sandbox_root.clone() {
                toolkit_config.sandbox_root = root;
            }
            toolkit_config.max_history = agent_config.max_history;

            let (toolkit, status_message) = match AgentToolkit::new(toolkit_config) {
                Ok(toolkit) => (Some(toolkit), None),
                Err(err) => {
                    tracing::warn!(error = ?err, "Failed to initialise agent toolkit");
                    (None, Some(format!("Agent toolkit unavailable: {err}")))
                }
            };
            let bus = EventBus::new(100);
            let receiver = bus.subscribe();

            (
                toolkit,
                AgentUiState::new(),
                default_tools(),
                Some(bus),
                Some(receiver),
                status_message,
            )
        };
        #[cfg(feature = "tui-agent")]
        {
            active_session.agent_mode = agent_config.enabled_by_default;
            if !active_session.tool_calls.is_empty() {
                agent_ui_state.history_cursor = active_session.tool_calls.len().saturating_sub(1);
            }
        }

        #[cfg(feature = "tui-agent")]
        let initial_status = agent_status.unwrap_or_else(|| "Ready".to_string());
        #[cfg(not(feature = "tui-agent"))]
        let initial_status = "Ready".to_string();

        Ok(Self {
            session_store,
            model_inventory,
            focus: FocusArea::Chat,
            metrics: Metrics::new(),
            status: StatusLine::new(initial_status),
            should_quit: false,
            sessions,
            session_cursor: 0,
            active_session,
            model_cursor: 0,
            #[cfg(feature = "tui-agent")]
            agent_toolkit,
            #[cfg(feature = "tui-agent")]
            agent_ui_state,
            #[cfg(feature = "tui-agent")]
            available_tools,
            #[cfg(feature = "tui-agent")]
            event_bus,
            #[cfg(feature = "tui-agent")]
            event_receiver,
            #[cfg(feature = "tui-agent")]
            agent_config,
            #[cfg(feature = "tui-agent")]
            pending_execution: None,
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

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status.set(message);
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
                self.poll_execution_events(runtime);
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
            return self.handle_palette_key(key, runtime);
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
        #[cfg(feature = "tui-agent")]
        {
            self.enforce_tool_history_limit();
            if self.active_session.tool_calls.is_empty() {
                self.agent_ui_state.history_cursor = 0;
            } else {
                self.agent_ui_state.history_cursor =
                    self.active_session.tool_calls.len().saturating_sub(1);
            }
            self.pending_execution = None;
        }
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
        #[cfg(feature = "tui-agent")]
        {
            self.active_session.agent_mode = self.agent_config.enabled_by_default;
            if !self.active_session.agent_mode {
                self.agent_ui_state.reset();
            }
            self.pending_execution = None;
        }
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
        self.pending_execution = None;
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
    fn handle_palette_key(&mut self, key: KeyEvent, runtime: &Runtime) -> Result<()> {
        if self.agent_ui_state.palette_prompt_active() {
            match key.code {
                KeyCode::Esc => {
                    self.cancel_pending_palette_execution();
                    self.agent_ui_state.clear_palette_prompt();
                    self.status
                        .set("Cancelled tool input; palette remains open.");
                }
                KeyCode::Enter => {
                    if let Some(input) = self.agent_ui_state.take_palette_prompt_input() {
                        let trimmed = input.trim();
                        if trimmed.is_empty() {
                            self.status.set("Input is required for this tool.");
                            if let Some(pending) = &self.pending_execution {
                                self.agent_ui_state.begin_palette_prompt(
                                    pending.request.prompt.clone(),
                                    pending.request.hint.clone(),
                                );
                            }
                        } else {
                            let value = trimmed.to_string();
                            self.complete_pending_palette_execution(value, runtime)?;
                        }
                    }
                }
                KeyCode::Backspace => {
                    self.agent_ui_state.palette_prompt_backspace();
                }
                KeyCode::Char(c) => {
                    self.agent_ui_state.palette_prompt_push(c);
                }
                _ => {}
            }
            return Ok(());
        }

        match key.code {
            KeyCode::Esc => {
                self.agent_ui_state.hide_palette();
                self.pending_execution = None;
            }
            KeyCode::Enter => {
                let selected_tool = {
                    let filtered_tools = self.palette_filtered_tools();
                    if filtered_tools.is_empty() {
                        self.status
                            .set("No tools match the current palette filter.");
                        self.agent_ui_state.hide_palette();
                        return Ok(());
                    }

                    let selected_index = self
                        .agent_ui_state
                        .palette_cursor
                        .min(filtered_tools.len() - 1);
                    filtered_tools[selected_index].clone()
                };
                self.enqueue_tool_execution(selected_tool, runtime)?;
                if self.pending_execution.is_none() {
                    self.agent_ui_state.hide_palette();
                    self.agent_ui_state.palette_cursor = 0;
                    self.agent_ui_state.palette_filter.clear();
                } else {
                    // Prompt is active; keep palette visible for user input
                    if let Some(pending) = &self.pending_execution {
                        self.agent_ui_state.begin_palette_prompt(
                            pending.request.prompt.clone(),
                            pending.request.hint.clone(),
                        );
                        self.status.set(pending.request.prompt.clone());
                    }
                }
            }
            KeyCode::Up => {
                if self.agent_ui_state.palette_cursor > 0 {
                    self.agent_ui_state.palette_cursor -= 1;
                }
            }
            KeyCode::Down => {
                let filtered_count = self.palette_filtered_tools().len();

                if filtered_count > 0 && self.agent_ui_state.palette_cursor < filtered_count - 1 {
                    self.agent_ui_state.palette_cursor += 1;
                }
            }
            KeyCode::Backspace => {
                self.agent_ui_state.palette_filter.pop();
                // Reset cursor when filter changes
                self.agent_ui_state.palette_cursor = 0;
                self.pending_execution = None;
            }
            KeyCode::Char(c) => {
                self.agent_ui_state.palette_filter.push(c);
                // Reset cursor when filter changes
                self.agent_ui_state.palette_cursor = 0;
                self.pending_execution = None;
            }
            _ => {}
        }
        Ok(())
    }

    /// Poll for execution events and update UI state
    #[cfg(feature = "tui-agent")]
    fn poll_execution_events(&mut self, runtime: &Runtime) {
        if let Some(receiver) = self.event_receiver.as_mut() {
            // Try to receive all pending events without blocking
            while let Ok(event) = receiver.try_recv() {
                self.agent_ui_state.update_from_event(&event);

                match &event {
                    ExecutionEvent::Started {
                        call_id, tool_name, ..
                    } => {
                        if let Some(call) = self
                            .active_session
                            .tool_calls
                            .iter()
                            .find(|c| c.id == *call_id)
                        {
                            if let Some(execution) = self.agent_ui_state.active_execution.as_mut() {
                                execution.arguments = call.arguments.clone();
                            }
                        }
                        self.status.set(format!("Running tool '{}'", tool_name));
                    }
                    ExecutionEvent::Progress { .. } => {}
                    ExecutionEvent::Completed {
                        call_id,
                        tool_name,
                        result,
                        ..
                    } => {
                        if let Some(call) = self
                            .active_session
                            .tool_calls
                            .iter_mut()
                            .find(|c| c.id == *call_id)
                        {
                            call.result = Some(result.clone());
                            if call.session_id.is_none() {
                                call.session_id = Some(self.active_session.summary.id);
                            }
                            if let Some(session_id) = call.session_id {
                                if let Err(err) = runtime
                                    .block_on(self.session_store.save_tool_call(session_id, call))
                                {
                                    tracing::warn!(?err, "Failed to persist tool call");
                                }
                            }
                        }
                        let outcome = if result.success {
                            "completed successfully"
                        } else {
                            "completed with errors"
                        };
                        self.status.set(format!("Tool '{}' {}", tool_name, outcome));
                    }
                    ExecutionEvent::Failed {
                        call_id,
                        tool_name,
                        error,
                        result,
                        ..
                    } => {
                        if let Some(call) = self
                            .active_session
                            .tool_calls
                            .iter_mut()
                            .find(|c| c.id == *call_id)
                        {
                            let failure_result = result.clone().unwrap_or_else(|| ToolCallResult {
                                success: false,
                                output: serde_json::Value::Null,
                                error: Some(error.clone()),
                                duration: std::time::Duration::default(),
                            });
                            call.result = Some(failure_result.clone());
                            if call.session_id.is_none() {
                                call.session_id = Some(self.active_session.summary.id);
                            }
                            if let Some(session_id) = call.session_id {
                                if let Err(err) = runtime
                                    .block_on(self.session_store.save_tool_call(session_id, call))
                                {
                                    tracing::warn!(?err, "Failed to persist failed tool call");
                                }
                            }
                        }
                        self.status
                            .set(format!("Tool '{}' failed: {}", tool_name, error));
                    }
                }
            }
        }
    }

    #[cfg(feature = "tui-agent")]
    fn palette_filtered_tools(&self) -> Vec<&ToolInfo> {
        let filter = self.agent_ui_state.palette_filter.to_lowercase();
        self.available_tools
            .iter()
            .filter(|tool| {
                filter.is_empty()
                    || tool.name.to_lowercase().contains(&filter)
                    || tool.description.to_lowercase().contains(&filter)
            })
            .collect()
    }

    #[cfg(feature = "tui-agent")]
    fn enqueue_tool_execution(&mut self, tool: ToolInfo, runtime: &Runtime) -> Result<()> {
        if self.agent_toolkit.is_none() {
            self.status
                .set("Agent toolkit unavailable; cannot execute tool.");
            return Ok(());
        }

        let arguments = self.default_arguments_for_tool(&tool.name);
        match self.prepare_tool_arguments(&tool, arguments) {
            ToolArgumentState::Ready(args) => {
                self.pending_execution = None;
                self.launch_tool_execution(&tool, args, runtime)?;
            }
            ToolArgumentState::NeedsInput(request) => {
                self.request_tool_input(tool, request);
                return Ok(());
            }
        }

        Ok(())
    }

    #[cfg(feature = "tui-agent")]
    fn request_tool_input(&mut self, tool: ToolInfo, request: ToolArgumentRequest) {
        let prompt_title = request.prompt.clone();
        let prompt_hint = request.hint.clone();
        self.agent_ui_state
            .begin_palette_prompt(prompt_title.clone(), prompt_hint);
        self.status.set(prompt_title);
        self.pending_execution = Some(PendingExecution { tool, request });
    }

    #[cfg(feature = "tui-agent")]
    fn cancel_pending_palette_execution(&mut self) {
        self.pending_execution = None;
        self.agent_ui_state.clear_palette_prompt();
    }

    #[cfg(feature = "tui-agent")]
    fn complete_pending_palette_execution(
        &mut self,
        input: String,
        runtime: &Runtime,
    ) -> Result<()> {
        let Some(pending) = self.pending_execution.take() else {
            return Ok(());
        };

        let tool = pending.tool.clone();
        let request = pending.request.clone();
        let mut arguments = request.base_arguments.clone();
        match request.kind {
            ToolInputKind::PathsArray { key } => {
                let paths: Vec<String> = input
                    .split(',')
                    .map(|part| part.trim())
                    .filter(|part| !part.is_empty())
                    .filter_map(|part| self.normalize_user_path(part))
                    .collect();
                if paths.is_empty() {
                    self.status.set("Please provide at least one path.");
                    self.pending_execution = Some(PendingExecution {
                        tool: tool,
                        request: request,
                    });
                    if let Some(pending) = &self.pending_execution {
                        self.agent_ui_state.begin_palette_prompt(
                            pending.request.prompt.clone(),
                            pending.request.hint.clone(),
                        );
                    }
                    return Ok(());
                }
                Self::set_paths_field(&mut arguments, key, paths);
            }
            ToolInputKind::StringField { key } => {
                Self::set_string_field(&mut arguments, key, input.clone());
            }
        }

        match self.prepare_tool_arguments(&tool, arguments) {
            ToolArgumentState::Ready(args) => {
                self.agent_ui_state.clear_palette_prompt();
                self.launch_tool_execution(&tool, args, runtime)?;
                self.agent_ui_state.hide_palette();
                self.agent_ui_state.palette_cursor = 0;
                self.agent_ui_state.palette_filter.clear();
            }
            ToolArgumentState::NeedsInput(request) => {
                self.request_tool_input(tool, request);
            }
        }

        Ok(())
    }

    #[cfg(feature = "tui-agent")]
    fn launch_tool_execution(
        &mut self,
        tool: &ToolInfo,
        arguments: Value,
        runtime: &Runtime,
    ) -> Result<()> {
        let wrapper_toolkit = match self.agent_toolkit.as_ref() {
            Some(toolkit) => toolkit.clone(),
            None => {
                self.status
                    .set("Agent toolkit unavailable; cannot execute tool.");
                return Ok(());
            }
        };

        let core_toolkit = wrapper_toolkit.core().clone();
        let executor = match self.event_bus.clone() {
            Some(bus) => ToolExecutor::with_events(core_toolkit, bus.clone()),
            None => ToolExecutor::new(core_toolkit),
        };

        let session_id = self.active_session.summary.id;
        let tool_call =
            executor.create_tool_call(tool.name.clone(), arguments.clone(), Some(session_id));
        let call_id = tool_call.id;

        self.active_session.tool_calls.push(tool_call.clone());
        self.agent_ui_state.history_cursor = self.active_session.tool_calls.len().saturating_sub(1);
        self.agent_ui_state
            .start_execution(call_id, tool.name.clone(), arguments.clone());
        self.status
            .set(format!("Queued tool '{}' for execution", tool.name));

        let executor_task = executor.clone();
        let event_bus = self.event_bus.clone();
        let tool_name = tool.name.clone();
        let tool_call_for_task = tool_call.clone();
        let arguments_for_task = arguments.clone();
        runtime.spawn(async move {
            match executor_task
                .execute(&tool_name, arguments_for_task, None)
                .await
            {
                Ok(_) => {}
                Err(err) => {
                    if let Some(bus) = event_bus {
                        bus.emit(ExecutionEvent::failed(
                            tool_call_for_task.id,
                            tool_name,
                            format!("Execution error: {}", err),
                            None,
                        ));
                    }
                }
            }
        });
        self.enforce_tool_history_limit();

        Ok(())
    }

    #[cfg(feature = "tui-agent")]
    fn prepare_tool_arguments(
        &mut self,
        tool: &ToolInfo,
        mut arguments: Value,
    ) -> ToolArgumentState {
        match tool.name.as_str() {
            "ls" => {
                if !Self::value_has_non_empty_string(&arguments, "path") {
                    return ToolArgumentState::NeedsInput(ToolArgumentRequest {
                        prompt: "Enter directory to list".to_string(),
                        hint: self.sandbox_hint(),
                        kind: ToolInputKind::StringField { key: "path" },
                        base_arguments: arguments,
                    });
                }
            }
            "cat" | "head" | "tail" | "wc" | "sort" | "uniq" => {
                if !Self::value_has_non_empty_array(&arguments, "paths") {
                    return ToolArgumentState::NeedsInput(ToolArgumentRequest {
                        prompt: format!("Enter file path for {}", tool.name),
                        hint: self.sandbox_hint(),
                        kind: ToolInputKind::PathsArray { key: "paths" },
                        base_arguments: arguments,
                    });
                }
            }
            "grep" => {
                if !Self::value_has_non_empty_string(&arguments, "pattern") {
                    if !self.agent_ui_state.palette_filter.trim().is_empty() {
                        Self::set_string_field(
                            &mut arguments,
                            "pattern",
                            self.agent_ui_state.palette_filter.trim().to_string(),
                        );
                    } else if let Some(model_id) = &self.active_session.summary.model_id {
                        Self::set_string_field(&mut arguments, "pattern", model_id.clone());
                    } else {
                        return ToolArgumentState::NeedsInput(ToolArgumentRequest {
                            prompt: "Enter search pattern".to_string(),
                            hint: Some("Supports regex or literal text".to_string()),
                            kind: ToolInputKind::StringField { key: "pattern" },
                            base_arguments: arguments,
                        });
                    }
                }

                if !Self::value_has_non_empty_array(&arguments, "paths") {
                    if let Some(root) = self
                        .sandbox_root_path()
                        .and_then(|path| self.path_within_sandbox(&path))
                    {
                        Self::set_paths_field(&mut arguments, "paths", vec![root]);
                    } else {
                        return ToolArgumentState::NeedsInput(ToolArgumentRequest {
                            prompt: "Enter search directory".to_string(),
                            hint: self.sandbox_hint(),
                            kind: ToolInputKind::PathsArray { key: "paths" },
                            base_arguments: arguments,
                        });
                    }
                }
            }
            "shell" => {
                if !Self::value_has_non_empty_string(&arguments, "command") {
                    return ToolArgumentState::NeedsInput(ToolArgumentRequest {
                        prompt: "Enter command to execute".to_string(),
                        hint: Some("Commands run inside the sandbox root".to_string()),
                        kind: ToolInputKind::StringField { key: "command" },
                        base_arguments: arguments,
                    });
                }
            }
            _ => {}
        }

        ToolArgumentState::Ready(arguments)
    }

    #[cfg(feature = "tui-agent")]
    fn sandbox_hint(&self) -> Option<String> {
        self.sandbox_root_path()
            .map(|path| format!("Sandbox root: {}", path.display()))
    }

    #[cfg(feature = "tui-agent")]
    fn value_has_non_empty_array(value: &Value, key: &str) -> bool {
        value
            .get(key)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .any(|entry| entry.as_str().map_or(false, |s| !s.trim().is_empty()))
            })
            .unwrap_or(false)
    }

    #[cfg(feature = "tui-agent")]
    fn value_has_non_empty_string(value: &Value, key: &str) -> bool {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    }

    #[cfg(feature = "tui-agent")]
    fn set_paths_field(value: &mut Value, key: &'static str, paths: Vec<String>) {
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                key.to_string(),
                Value::Array(paths.into_iter().map(Value::String).collect()),
            );
        }
    }

    #[cfg(feature = "tui-agent")]
    fn set_string_field(value: &mut Value, key: &'static str, data: String) {
        if let Some(obj) = value.as_object_mut() {
            obj.insert(key.to_string(), Value::String(data));
        }
    }

    #[cfg(feature = "tui-agent")]
    fn default_arguments_for_tool(&self, tool_name: &str) -> serde_json::Value {
        let preset = self.active_model_preset();
        match tool_name {
            "ls" => {
                let default_path = preset
                    .as_ref()
                    .and_then(|preset| self.path_within_sandbox(&preset.directory))
                    .unwrap_or_else(|| ".".to_string());
                json!({
                    "path": default_path,
                    "all": false,
                    "long": false,
                    "human_readable": false,
                    "recursive": false,
                    "sort_by_time": false,
                    "reverse": false
                })
            }
            "cat" => self
                .default_tool_file_path()
                .map(|path| {
                    json!({
                        "paths": [path],
                        "number_lines": false,
                        "show_ends": false,
                        "squeeze_blank": false
                    })
                })
                .unwrap_or_else(|| json!({})),
            "head" => self
                .default_tool_file_path()
                .map(|path| json!({ "paths": [path], "lines": 20 }))
                .unwrap_or_else(|| json!({})),
            "tail" => self
                .default_tool_file_path()
                .map(|path| json!({ "paths": [path], "lines": 20 }))
                .unwrap_or_else(|| json!({})),
            "wc" => self
                .default_tool_file_path()
                .map(|path| {
                    json!({
                        "paths": [path],
                        "lines": true,
                        "words": true,
                        "bytes": false,
                        "chars": false
                    })
                })
                .unwrap_or_else(|| json!({})),
            "sort" => self
                .default_tool_file_path()
                .map(|path| {
                    json!({
                        "paths": [path],
                        "reverse": false,
                        "numeric": false,
                        "unique": false
                    })
                })
                .unwrap_or_else(|| json!({})),
            "uniq" => self
                .default_tool_file_path()
                .map(|path| {
                    json!({
                        "paths": [path],
                        "count": false,
                        "repeated": false,
                        "unique": false
                    })
                })
                .unwrap_or_else(|| json!({})),
            "grep" => {
                let default_path = preset
                    .as_ref()
                    .and_then(|preset| self.path_within_sandbox(&preset.directory))
                    .unwrap_or_else(|| ".".to_string());
                let default_pattern = self
                    .agent_ui_state
                    .palette_filter
                    .clone()
                    .trim()
                    .to_string();
                let pattern = if !default_pattern.is_empty() {
                    default_pattern
                } else if let Some(model_id) = &self.active_session.summary.model_id {
                    model_id.clone()
                } else {
                    "".to_string()
                };
                json!({
                    "pattern": pattern,
                    "paths": [default_path],
                    "recursive": true,
                    "ignore_case": true,
                    "line_number": true,
                    "before_context": 0,
                    "after_context": 0,
                    "count": false
                })
            }
            "shell" => {
                if let Some(preset) = preset.and_then(|preset| {
                    self.path_within_sandbox(&preset.model_path)
                        .map(|path| (preset, path))
                }) {
                    let (preset, path) = preset;
                    json!({
                        "command": format!(
                            "mistralrs-server run --model \"{}\" --framework {} --optimized",
                            path,
                            preset.framework
                        ),
                        "capture_stdout": true,
                        "capture_stderr": true,
                        "timeout": 30
                    })
                } else {
                    json!({
                        "command": "echo \"Hello from mistralrs agent\"",
                        "capture_stdout": true,
                        "capture_stderr": true,
                        "timeout": 30
                    })
                }
            }
            _ => json!({}),
        }
    }

    #[cfg(feature = "tui-agent")]
    fn enforce_tool_history_limit(&mut self) {
        let max = self.agent_config.max_history.max(1);
        let len = self.active_session.tool_calls.len();
        if len > max {
            let remove = len - max;
            self.active_session.tool_calls.drain(0..remove);
            let history_len = self.active_session.tool_calls.len();
            if history_len == 0 {
                self.agent_ui_state.history_cursor = 0;
            } else if self.agent_ui_state.history_cursor >= history_len {
                self.agent_ui_state.history_cursor = history_len - 1;
            }
        }
    }

    #[cfg(feature = "tui-agent")]
    fn sandbox_root_path(&self) -> Option<PathBuf> {
        self.agent_toolkit
            .as_ref()
            .map(|toolkit| toolkit.config().sandbox_root.clone())
            .or_else(|| std::env::current_dir().ok())
    }

    #[cfg(feature = "tui-agent")]
    fn active_model_preset(&self) -> Option<ModelPreset> {
        let model_id = self.active_session.summary.model_id.as_ref()?;
        let entry = self
            .model_inventory
            .entries()
            .into_iter()
            .find(|entry| &entry.identifier == model_id)?;

        let model_path = entry.path.clone();
        let directory = model_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let framework = Self::model_framework_from_format(entry.format.as_deref());

        Some(ModelPreset {
            model_path,
            directory,
            framework,
        })
    }

    #[cfg(feature = "tui-agent")]
    fn model_framework_from_format(format: Option<&str>) -> String {
        match format.map(|fmt| fmt.to_ascii_lowercase()) {
            Some(fmt) if fmt.contains("gguf") || fmt.contains("ggml") => "candle".to_string(),
            Some(fmt) if fmt.contains("safetensors") => "torch".to_string(),
            Some(fmt) => fmt,
            None => "auto".to_string(),
        }
    }

    #[cfg(feature = "tui-agent")]
    fn path_within_sandbox(&self, target: &Path) -> Option<String> {
        let root = self.sandbox_root_path()?;
        let target = target.canonicalize().ok()?;
        let root = root.canonicalize().ok()?;
        if target.starts_with(&root) {
            let relative = target.strip_prefix(&root).ok()?;
            if relative.components().count() == 0 {
                Some(".".to_string())
            } else {
                Some(relative.to_string_lossy().to_string())
            }
        } else {
            None
        }
    }

    #[cfg(feature = "tui-agent")]
    fn default_tool_file_path(&self) -> Option<String> {
        if let Some(preset) = self
            .active_model_preset()
            .and_then(|preset| self.path_within_sandbox(&preset.model_path))
        {
            return Some(preset);
        }

        let root = self.sandbox_root_path()?;
        let mut fallback = None;
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
                    if matches!(ext, "md" | "txt" | "rs" | "toml" | "log") {
                        if let Some(rel) = self.path_within_sandbox(&path) {
                            return Some(rel);
                        }
                    }
                    if fallback.is_none() {
                        fallback = Some(path);
                    }
                }
            }
        }
        fallback.and_then(|path| self.path_within_sandbox(&path))
    }

    #[cfg(feature = "tui-agent")]
    fn normalize_user_path(&self, raw: &str) -> Option<String> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        let Some(root) = self.sandbox_root_path() else {
            return Some(trimmed.to_string());
        };
        let root_canon = root.canonicalize().ok()?;

        let candidate = PathBuf::from(trimmed);
        let absolute = if candidate.is_absolute() {
            candidate.canonicalize().ok()?
        } else {
            let joined = root_canon.join(&candidate);
            joined.canonicalize().ok()?
        };

        if !absolute.starts_with(&root_canon) {
            return None;
        }

        let relative = absolute.strip_prefix(&root_canon).ok()?;
        if relative.components().count() == 0 {
            Some(".".to_string())
        } else {
            Some(relative.to_string_lossy().to_string())
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
        #[cfg(feature = "tui-agent")]
        let future = App::initialise(store, inventory, None, Some(AgentPreferences::default()));
        #[cfg(not(feature = "tui-agent"))]
        let future = App::initialise(store, inventory, None);
        rt.block_on(future).expect("initialise")
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
