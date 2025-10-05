//! Session persistence utilities backed by SQLite.

use std::{fs, path::Path, str::FromStr};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    Row, SqlitePool,
};
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Clone, Debug)]
pub struct SessionStore {
    pool: SqlitePool,
}

#[derive(Clone, Debug)]
pub struct SessionSummary {
    pub id: Uuid,
    pub title: String,
    pub model_id: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub token_count: u64,
}

#[derive(Clone, Debug)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub token_count: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct SessionContext {
    pub summary: SessionSummary,
    pub messages: Vec<SessionMessage>,
    /// Agent mode enabled for this session
    #[cfg(feature = "tui-agent")]
    pub agent_mode: bool,
    /// Tool calls made during this session
    #[cfg(feature = "tui-agent")]
    pub tool_calls: Vec<crate::agent::toolkit::ToolCall>,
}

impl SessionStore {
    pub async fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "Failed to create parent directory for database: {}",
                        parent.display()
                    )
                })?;
            }
        }

        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .pragma("foreign_keys", "ON");

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .with_context(|| format!("Failed to open SQLite database at {}", path.display()))?;

        MIGRATOR
            .run(&pool)
            .await
            .context("Failed to run mistralrs-tui database migrations")?;

        Ok(Self { pool })
    }

    pub async fn create_session(&self, model_id: &str, title: &str) -> Result<SessionContext> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        sqlx::query(
            "INSERT INTO sessions (id, started_at, updated_at, model_id, title) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id.to_string())
        .bind(&now_str)
        .bind(&now_str)
        .bind(model_id)
        .bind(title)
        .execute(&self.pool)
        .await
        .context("Failed to insert session record")?;

        let summary = SessionSummary {
            id,
            title: title.to_string(),
            model_id: Some(model_id.to_string()),
            updated_at: now,
            token_count: 0,
        };

        Ok(SessionContext {
            summary,
            messages: Vec::new(),
            #[cfg(feature = "tui-agent")]
            agent_mode: false,
            #[cfg(feature = "tui-agent")]
            tool_calls: Vec::new(),
        })
    }

    pub async fn list_recent_sessions(&self, limit: i64) -> Result<Vec<SessionSummary>> {
        let rows = sqlx::query(
            "SELECT id, title, model_id, updated_at, (
                SELECT COALESCE(SUM(token_count), 0)
                FROM messages WHERE session_id = sessions.id
            ) AS token_count FROM sessions ORDER BY datetime(updated_at) DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query sessions")?;

        rows.into_iter()
            .map(|row| self.row_to_summary(row))
            .collect()
    }

    pub async fn load_session(&self, session_id: Uuid) -> Result<SessionContext> {
        let row = sqlx::query(
            "SELECT id, title, model_id, updated_at, (
                SELECT COALESCE(SUM(token_count), 0)
                FROM messages WHERE session_id = sessions.id
            ) AS token_count FROM sessions WHERE id = ?",
        )
        .bind(session_id.to_string())
        .fetch_one(&self.pool)
        .await
        .context("Failed to load session")?;

        let summary = self.row_to_summary(row)?;

        let message_rows = sqlx::query(
            "SELECT id, role, content, token_count, created_at FROM messages WHERE session_id = ? ORDER BY datetime(created_at) ASC",
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .context("Failed to load session messages")?;

        let mut messages = Vec::with_capacity(message_rows.len());
        for row in message_rows {
            messages.push(self.row_to_message(row)?);
        }

        Ok(SessionContext {
            summary,
            messages,
            #[cfg(feature = "tui-agent")]
            agent_mode: false,
            #[cfg(feature = "tui-agent")]
            tool_calls: Vec::new(),
        })
    }

    pub async fn update_session_model(&self, session_id: Uuid, model_id: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET model_id = ?, updated_at = ? WHERE id = ?")
            .bind(model_id)
            .bind(&now_str)
            .bind(session_id.to_string())
            .execute(&self.pool)
            .await
            .context("Failed to update session model")?;
        Ok(())
    }

    fn row_to_summary(&self, row: sqlx::sqlite::SqliteRow) -> Result<SessionSummary> {
        let id: String = row.try_get("id")?;
        let updated_at: String = row.try_get("updated_at")?;
        let model_id: Option<String> = row.try_get("model_id")?;
        let title: String = row.try_get("title")?;
        let token_count: i64 = row.try_get("token_count")?;

        let updated_at = DateTime::parse_from_rfc3339(&updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .context("Invalid session timestamp in database")?;

        Ok(SessionSummary {
            id: Uuid::parse_str(&id).context("Invalid session id")?,
            title,
            model_id,
            updated_at,
            token_count: token_count.max(0) as u64,
        })
    }

    fn row_to_message(&self, row: sqlx::sqlite::SqliteRow) -> Result<SessionMessage> {
        let role: String = row.try_get("role")?;
        let content: String = row.try_get("content")?;
        let token_count: Option<i64> = row.try_get("token_count")?;
        let created_at: String = row.try_get("created_at")?;

        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .context("Invalid message timestamp in database")?;

        Ok(SessionMessage {
            role,
            content,
            token_count,
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
    }

    #[test]
    fn create_and_load_session_round_trip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("sessions.sqlite");
        let runtime = runtime();
        let store = runtime
            .block_on(SessionStore::new(&db_path))
            .expect("store");
        let ctx = runtime
            .block_on(store.create_session("model", "Title"))
            .expect("create");
        let summaries = runtime
            .block_on(store.list_recent_sessions(10))
            .expect("list");
        assert!(!summaries.is_empty());
        let loaded = runtime
            .block_on(store.load_session(ctx.summary.id))
            .expect("load");
        assert_eq!(loaded.summary.title, "Title");
        assert_eq!(loaded.summary.model_id.as_deref(), Some("model"));
    }
}
