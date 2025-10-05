use std::{
    io::{self, Stdout},
    time::{Duration, Instant},
};

use anyhow::Result;
use crossterm::{
    event, execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::runtime::Runtime;

use crate::{app::App, backend::Options, input};

pub fn run(runtime: &Runtime, app: &mut App, options: &Options) -> Result<()> {
    let tick_rate = options.tick_rate;

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| crate::ui::render(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)? {
            let event = event::read()?;
            if let Some(evt) = input::from_crossterm(event) {
                app.handle_event(evt, runtime)?;
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick(runtime)?;
            last_tick = Instant::now();
        }

        if app.should_quit() {
            break;
        }
    }

    clean_up_terminal(&mut terminal)
}

fn clean_up_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
