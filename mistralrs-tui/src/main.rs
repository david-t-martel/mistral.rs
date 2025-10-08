//! Entry point for the standalone mistral.rs terminal UI executable.
//! The binary initialises configuration, persistence and model discovery
//! before handing control over to the rendering backend.

use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use clap::{ArgAction, Parser, ValueHint};
use tracing_subscriber::EnvFilter;

use mistralrs_tui::{
    app::App,
    backend::{self, Options},
    config::TuiConfig,
    inventory::ModelInventory,
    session::SessionStore,
};

/// CLI options for the `mistralrs-tui` binary.
#[derive(Debug, Parser)]
#[command(
    name = "mistralrs-tui",
    about = "GPU-accelerated terminal UI for mistral.rs"
)]
struct Args {
    /// Path to the configuration file (defaults to platform config dir)
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    config: Option<PathBuf>,

    /// Override the SQLite database path for session history
    #[arg(long = "database", value_name = "FILE", value_hint = ValueHint::FilePath)]
    database_path: Option<PathBuf>,

    /// Additional directories to scan for models (can be used multiple times)
    #[arg(
        long = "model-path",
        value_name = "DIR",
        value_hint = ValueHint::DirPath,
        action = ArgAction::Append
    )]
    model_paths: Vec<PathBuf>,

    /// Optional explicit MODEL_INVENTORY.json path
    #[arg(long, value_name = "FILE", value_hint = ValueHint::FilePath)]
    inventory: Option<PathBuf>,

    /// Disable GPU backend and always use terminal rendering
    #[arg(long)]
    disable_gpu: bool,

    /// Enable verbose diagnostics
    #[arg(long)]
    verbose: bool,

    /// Customize UI tick rate in milliseconds
    #[arg(long, value_name = "MS")]
    tick_rate_ms: Option<u64>,

    /// Initialise subsystems then exit without launching the interactive UI.
    /// Useful for smoke-tests and automation.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    init_tracing(args.verbose)?;

    let (mut config, config_path) =
        TuiConfig::load(args.config.as_deref()).context("loading mistralrs-tui configuration")?;

    if let Some(db) = args.database_path {
        config.database_path = db;
    }
    if !args.model_paths.is_empty() {
        config.model_search_paths = args.model_paths.clone();
    }
    if let Some(manifest) = args.inventory {
        config.inventory_file = Some(manifest);
    }
    if args.disable_gpu {
        config.prefer_gpu = false;
    }
    config.ensure_defaults();
    // Persist merged configuration for future launches.
    config
        .persist(&config_path)
        .context("saving mistralrs-tui configuration")?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .thread_name("mistralrs-tui")
        .build()
        .context("initialising tokio runtime")?;

    let session_store = runtime
        .block_on(SessionStore::new(&config.database_path))
        .context("initialising session store")?;
    let session_store = Arc::new(session_store);

    let inventory = Arc::new(ModelInventory::new(
        config.model_search_paths.clone(),
        config.effective_inventory_file(),
    ));
    inventory.refresh()?;

    #[cfg(feature = "tui-agent")]
    let initialise_future = App::initialise(
        session_store.clone(),
        inventory.clone(),
        config.default_model.clone(),
        Some(config.agent.clone()),
    );
    #[cfg(not(feature = "tui-agent"))]
    let initialise_future = App::initialise(
        session_store.clone(),
        inventory.clone(),
        config.default_model.clone(),
    );

    let mut app = runtime
        .block_on(initialise_future)
        .context("initialising application state")?;

    let mut options = Options {
        prefer_gpu: config.prefer_gpu && !args.disable_gpu,
        ..Default::default()
    };
    if let Some(ms) = args.tick_rate_ms {
        options.tick_rate = Duration::from_millis(ms.clamp(16, 1000));
    }

    if args.dry_run {
        println!(
            "Initialised mistralrs-tui (sessions: {}, models: {})",
            app.sessions().len(),
            app.model_entries().len()
        );
        return Ok(());
    }

    backend::run(&runtime, &mut app, options)
}

/// Configure the tracing subscriber based on CLI flags and environment.
fn init_tracing(verbose: bool) -> Result<()> {
    if tracing::dispatcher::has_been_set() {
        return Ok(());
    }

    let default_level = if verbose { "info" } else { "warn" };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
    Ok(())
}
