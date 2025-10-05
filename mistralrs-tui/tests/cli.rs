use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn dry_run_exits_cleanly() {
    let tmp = tempdir().unwrap();
    let config_path = tmp.path().join("tui.toml");
    let db_path = tmp.path().join("sessions.sqlite");

    let mut cmd = Command::cargo_bin("mistralrs-tui").unwrap();
    cmd.arg("--dry-run")
        .arg("--config")
        .arg(&config_path)
        .arg("--database")
        .arg(&db_path)
        .env("TERM", "dumb");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Initialised mistralrs-tui"));
}
