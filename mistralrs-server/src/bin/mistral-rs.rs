//! Binary alias that exposes the server/agent entry point as `mistral-rs`.
//!
//! This shim simply reuses the existing `main.rs` implementation so both
//! `mistralrs-server.exe` and `mistral-rs.exe` remain in sync while we migrate
//! tooling and documentation toward the new canonical binary name on Windows.

include!("../main.rs");
