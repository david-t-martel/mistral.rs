-- Agent tools database schema
-- Feature-gated tables for agent mode (tui-agent feature)

-- Tool calls: Track all tool execution attempts
CREATE TABLE IF NOT EXISTS tool_calls (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL,
    result_json TEXT,
    success INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    duration_ms INTEGER,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tool_calls_session ON tool_calls(session_id);
CREATE INDEX IF NOT EXISTS idx_tool_calls_tool_name ON tool_calls(tool_name);
CREATE INDEX IF NOT EXISTS idx_tool_calls_created_at ON tool_calls(created_at);

-- Agent settings: Per-session agent configuration
CREATE TABLE IF NOT EXISTS agent_settings (
    session_id TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
    agent_mode_enabled INTEGER NOT NULL DEFAULT 0,
    sandbox_root TEXT,
    security_level TEXT NOT NULL DEFAULT 'standard',
    max_tool_calls INTEGER NOT NULL DEFAULT 100,
    updated_at TEXT NOT NULL
);

-- Agent mode flag for sessions table (if not exists)
-- This will be handled via ALTER TABLE in the session.rs code if needed,
-- or we can track agent mode through the agent_settings table
