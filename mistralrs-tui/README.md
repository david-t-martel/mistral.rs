# mistralrs-tui Roadmap

This document captures the initial requirements, design goals, and development
plan for the dedicated GPU-accelerated terminal user interface (TUI) for
mistral.rs. The intent is to grow this into a first-class workspace crate that
offers a feature-rich, high-performance terminal experience for text, vision,
diffusion, and speech workloads.

## 1. Goals & Requirements

### 1.1 Functional Requirements

- Provide a responsive, pane-based terminal UI for chatting with models,
  inspecting logs, and managing inference sessions.
- Support automated discovery of installed models (GGUF, safetensors) by
  scanning the existing `MODEL_INVENTORY.json`, Hugging Face cache, and user
  supplied directories.
- Allow seamless model switching without restarting the server, including
  architecture-specific launch options (plain, vision, diffusion, speech).
- Display real-time token usage, latency metrics (TTFT, tokens/sec), and session
  context length.
- Offer configurable chat layouts (single column, dual pane, detached
  inspector) with keyboard shortcuts and mouse support where available.
- Integrate with mistral.rs web-search/tool-calling pipelines and expose status
  indicators when those subsystems are active.
- Provide role-aware message editing, retry, and template management for quick
  prompt iteration.
- Expose session history, transcript export (Markdown/JSONL), and attachment
  previews for multimodal runs (images/audio waveforms).

### 1.2 Non-Functional Requirements

- Deliver consistent frame-times targeting 60 FPS updates on modern terminals by
  leveraging GPU-accelerated backends where supported (wgpu/vulkan/metal).
- Minimize blocking I/O by relying on async command dispatch into
  `mistralrs-server-core` via channels.
- Offer a configurable persistence layer (SQLite by default) with transactional
  integrity for session metadata.
- Maintain cross-platform support (Linux, macOS, Windows) with fallback to pure
  CPU rendering when GPU acceleration is unavailable.
- Adhere to Rust 2021 idioms, Clippy clean with `-D warnings`, and align with
  the workspace formatting/linting strategy (`cargo fmt`, `cargo clippy`).

### 1.3 Technical Requirements

- Introduce a new crate `mistralrs-tui` inside the workspace and wire it into
  the top-level `Cargo.toml` members and feature flags.
- Choose `ratatui` (>=0.29) for core layout combined with `ratatui-winit` and
  `wgpu` for accelerated rendering, falling back to `crossterm` when GPU is not
  detected.
- Use `sqlx` with SQLite for session storage, enabling async access and compile
  time query validation. Require `sqlx-cli` for migrations.
- Leverage `tokio` runtime (already in workspace) with dedicated tasks for
  streaming responses and UI input processing.
- Reuse shared types from `mistralrs-core` and `mistralrs-server-core` to avoid
  duplication (Requests, SamplingParams, etc.).

## 2. Architecture Overview

1. **Frontend Layer**: Ratatui-based renderer with modular widgets:

   - Chat panel (messages with role badges, token counts).
   - Model browser (inventory tree, filter, metadata preview).
   - Metrics sidebar (usage, VRAM, TTFT charts, queue depth).
   - Session manager (list of active/past sessions sourced from SQLite).
   - Notification area (errors, download progress, MCP/tool events).

1. **Controller Layer**: Input router translating keyboard/mouse commands into
   actions, state machine for focus management, and command palette for quick
   operations. Use `heed` or `keyed` crate for keymap definitions if necessary.

1. **Service Layer**: Async tasks handling

   - Model inventory scanning and caching.
   - Interaction with `mistralrs::MistralRs` or the HTTP/OpenAI server, possibly
     via gRPC/WebSocket in future iterations.
   - Session persistence (SQLite) and transcript export.
   - Telemetry (token usage, runtime metrics) aggregated from streaming
     responses.

1. **Persistence Layer**: SQLite database with migrations stored under
   `mistralrs-tui/migrations`. Tables include `sessions`, `messages`,
   `attachments`, `models`, and `settings`.

1. **Integration Layer**: Feature flag `tui-agent` to enable advanced agent
   workflows that rely on this TUI (per existing workspace comments).

## 3. Tooling & Dependencies

- Add dependencies to `mistralrs-tui`:
  - `ratatui`, `ratatui-winit`, `crossterm` (fallback), `wgpu`, `winit`.
  - `tokio`, `futures`, `anyhow`, `tracing`, `tracing-subscriber`.
  - `sqlx` with `sqlite` feature, `sqlx-cli` (developer install).
  - `serde`, `serde_json`, `indexmap` for configuration and persisted state.
  - `clap` for standalone CLI entry point (`mistralrs-tui` binary).
  - `notify` for watching model directories and invalidating caches.
- Document developer tooling installation:
  - `cargo install sqlx-cli --features sqlite`.
  - Ensure Vulkan/Metal/CUDA runtime components are available for wgpu; fallback
    gracefully otherwise.
  - Optional: `cargo install just` if we add command recipes, `cargo watch` for
    hot reload.

## 4. Data Model & Session Tracking

- Replace the current `rustyline` history-only approach with a session store.
- Proposed schema:
  - `sessions (id TEXT, started_at DATETIME, model_id TEXT, title TEXT, tags TEXT, token_count INTEGER, settings JSONB)`.
  - `messages (id TEXT, session_id TEXT, role TEXT, content TEXT, token_count INTEGER, latency_ms INTEGER, created_at DATETIME)`.
  - `attachments (id TEXT, session_id TEXT, path TEXT, mime TEXT, metadata JSON, created_at DATETIME)`.
  - `models (id TEXT PRIMARY KEY, path TEXT, format TEXT, size_bytes INTEGER, last_used DATETIME, capabilities JSON)`.
- Provide migrations and helper functions for CRUD operations via `sqlx`.
- Introduce abstraction trait `SessionStore` to allow alternative backends (e.g.
  Postgres) in the future.

## 5. Automated Model Discovery

- Implement a model inventory service that:
  1. Loads `MODEL_INVENTORY.json` when present.
  1. Scans configured directories (defaulting to `~/.cache/huggingface` and
     `.models/`).
  1. Supports manual refresh and background watch via `notify`.
  1. Enriches entries with checksum/VRAM estimates and supported modalities.
- Cache results in SQLite `models` table and expose filtering/search UI.

## 6. Roadmap / TODO

### Phase 0 – Bootstrapping

- [ ] Add `mistralrs-tui` crate folder with `Cargo.toml`, `src/main.rs`, and
  workspace wiring.
- [ ] Define feature flag `tui-agent` in the top-level workspace referencing the
  new crate.
- [ ] Set up linting (Clippy) and formatting (cargo fmt) for the crate.

### Phase 1 – Infrastructure

- [ ] Create SQLite schema + migrations using `sqlx migrate`.
- [ ] Implement session store abstraction and integration tests (using
  `sqlx::sqlite::SqlitePoolOptions`).
- [ ] Build model discovery service with configurable sources.
- [ ] Provide initial config file (TOML) for TUI settings (paths, theme,
  telemetry toggles).

### Phase 2 – Core UI

- [ ] Establish Ratatui app skeleton with async runtime, terminal backend
  selection (wgpu vs crossterm), and main event loop.
- [ ] Implement layout scaffolding: chat view, sidebar, footer status bar.
- [ ] Wire chat interactions to MistralRs backend (streaming tokens, error
  handling, command palette).
- [ ] Display token usage, context length, and latency per response.
- [ ] Support session switching and persistence via keyboard shortcuts.

### Phase 3 – Advanced Features

- [ ] Add model browser with inventory filtering and tooltip details.
- [ ] Implement image/audio attachment preview and upload pipeline.
- [ ] Integrate web search/tool-calling indicators and log panes.
- [ ] Add GPU diagnostics panel (VRAM usage, backend info) when available.
- [ ] Provide export features (Markdown, JSONL) and session tagging.

### Phase 4 – Polishing & Packaging

- [ ] Harden cross-platform compatibility (Windows terminal quirks, macOS
  security prompts, Wayland).
- [ ] Add test coverage for services and command handling.
- [ ] Document installation and usage, including troubleshooting for GPU
  backends.
- [ ] Prepare release artifacts, example configs, and CI integration (GitHub
  Actions job for `tui-agent`).

## 7. Testing & Profiling

- Run the crate's automated tests:

  ```bash
  cargo test -p mistralrs-tui
  # or via the helper target
  make test-tui
  ```

- Execute a configuration-only smoke test without launching the interactive
  loop:

  ```bash
  cargo run -p mistralrs-tui -- --dry-run --config /tmp/tui.toml --database /tmp/tui.db
  ```

- Generate a flamegraph (requires `cargo install flamegraph`):

  ```bash
  make tui-flamegraph
  ```

## 8. Next Steps

1. Confirm crate layout with maintainers and agree on `ratatui` + `wgpu`
   approach; prototype minimal wgpu backend to validate performance.
1. Land foundational PR adding the crate, workspace wiring, and SQLite session
   store (Phase 0 + part of Phase 1).
1. Iteratively build UI features, ensuring each milestone keeps the tool usable
   and documented.

______________________________________________________________________

Maintainers should treat this document as the living source of truth for TUI
work. As features land, update the TODO items and capture learnings, especially
around GPU backend compatibility and session storage migrations.
