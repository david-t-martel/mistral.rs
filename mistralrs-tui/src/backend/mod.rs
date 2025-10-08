//! Rendering backends for the terminal UI.
//!
//! Currently only the crossterm-powered terminal backend is implemented. A
//! future GPU accelerated path can hook in via the [`Options::prefer_gpu`] flag
//! once the implementation is ready.
//!
//! Pending integration work:
//! * Promote the GPU backend from the preview module (`gpu.rs`) into the default pipeline once the
//!   outstanding safety work there is complete.
//! * Surface backend health in richer UI widgets (current implementation writes to the status line).
//! * De-duplicate layout code between terminal and GPU paths so shared widgets render identically.

use std::time::Duration;

use anyhow::Result;
use tokio::runtime::Runtime;

use crate::app::App;

/// Backend configuration shared by concrete implementations.
pub struct Options {
    #[allow(dead_code)]
    pub prefer_gpu: bool,
    /// Desired redraw cadence for the UI loop.
    pub tick_rate: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            prefer_gpu: true,
            tick_rate: Duration::from_millis(75),
        }
    }
}

/// Launch the preferred backend, falling back to the terminal renderer on
/// failure.
pub fn run(runtime: &Runtime, app: &mut App, options: Options) -> Result<()> {
    #[cfg(feature = "gpu")]
    {
        if options.prefer_gpu {
            match gpu::run(runtime, app, &options) {
                Ok(()) => return Ok(()),
                Err(err) => {
                    tracing::warn!(?err, "Falling back to terminal backend after GPU failure");
                    app.set_status(format!(
                        "GPU backend unavailable ({err}); using terminal renderer"
                    ));
                }
            }
        }
    }

    terminal::run(runtime, app, &options)
}

#[cfg(feature = "gpu")]
mod gpu;
mod terminal;
