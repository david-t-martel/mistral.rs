use std::sync::Arc;

use chrono::{TimeZone, Utc};
use insta::assert_snapshot;
use mistralrs_tui::{app::App, inventory::ModelInventory, session::SessionStore, ui};
use ratatui::{backend::TestBackend, layout::Rect, Terminal, TerminalOptions, Viewport};
use sqlx::SqlitePool;
use tempfile::tempdir;
use tokio::runtime::Builder;

fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let width = buffer.area.width;
    let height = buffer.area.height;
    let mut output = String::with_capacity((width as usize + 1) * height as usize);
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            output.push_str(cell.symbol());
        }
        if y + 1 < height {
            output.push('\n');
        }
    }
    output
}

#[test]
fn initial_layout_snapshot() {
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let tmp = tempdir().expect("tempdir");
    let db_path = tmp.path().join("tui_sessions.sqlite");

    let store = runtime
        .block_on(SessionStore::new(&db_path))
        .expect("session store");
    let session = runtime
        .block_on(store.create_session("snapshot-model", "Snapshot Session"))
        .expect("create session");

    runtime.block_on(async {
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display()))
            .await
            .expect("pool");
        let fixed = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let as_rfc3339 = fixed.to_rfc3339();
        sqlx::query("UPDATE sessions SET started_at = ?, updated_at = ? WHERE id = ?")
            .bind(&as_rfc3339)
            .bind(&as_rfc3339)
            .bind(session.summary.id.to_string())
            .execute(&pool)
            .await
            .expect("timestamp update");
    });

    let store = Arc::new(store);
    let inventory = Arc::new(ModelInventory::new(vec![], None));
    inventory.refresh().expect("inventory refresh");

    let app = runtime
        .block_on(App::initialise(store, inventory, None))
        .expect("app init");

    let backend = TestBackend::new(80, 24);
    let viewport = Viewport::Fixed(Rect::new(0, 0, 80, 24));
    let mut terminal =
        Terminal::with_options(backend, TerminalOptions { viewport }).expect("terminal");

    terminal
        .draw(|frame| ui::render(frame, &app))
        .expect("draw");

    let snapshot = buffer_to_string(terminal.backend().buffer());
    assert_snapshot!("initial_layout", snapshot);
}
