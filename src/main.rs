mod der_parser;
mod format;
pub mod tui;

use tui::app::App;
use ratatui::prelude::*;
use crossterm::{execute, terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use std::io::stdout;
use crossterm::event::{self, Event};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res: Result<(), std::io::Error> = loop {
        terminal.draw(|f| app.draw(f))?;
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                app.handle_input(key);
            }
        }
        if app.should_quit {
            break Ok(());
        }
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}
