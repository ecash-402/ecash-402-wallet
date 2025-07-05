pub mod app;
pub mod config;
pub mod events;
pub mod state;
pub mod widgets;

pub use app::App;
pub use config::Config;
pub use events::EventHandler;
pub use state::AppState;

use crate::error::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new().await?;
    let mut should_quit = false;

    while !should_quit {
        terminal.draw(|f| app.draw(f))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    should_quit = true;
                }
                KeyCode::Char('q')
                    if app.get_current_view() == crate::tui::state::ActiveView::Main =>
                {
                    should_quit = true;
                }
                _ => {
                    app.handle_key_event(key).await?;
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
