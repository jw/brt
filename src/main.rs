use std::{
    error::Error,
    io::{stdout, Stdout},
    ops::ControlFlow,
    time::Duration,
};

use chrono::{DateTime, Utc};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::info;
use ratatui::{
    prelude::*,
    widgets::{block::Title, Block, BorderType, Borders, Padding, Paragraph},
};

mod logger;
mod model;

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

// These type aliases are used to make the code more readable by reducing repetition of the generic
// types. They are not necessary for the functionality of the code.
type Terminal = ratatui::Terminal<CrosstermBackend<Stdout>>;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {}

fn main() -> Result<()> {
    logger::initialize_logging();
    info!("{NAME} ({VERSION}) started.");

    let _cli = Cli::parse();

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal);
    restore_terminal(terminal)?;

    if let Err(err) = result {
        eprintln!("{err:?}");
    }
    Ok(())
}

fn setup_terminal() -> Result<Terminal> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(mut terminal: Terminal) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}

fn run(terminal: &mut Terminal) -> Result<()> {
    loop {
        terminal.draw(ui)?;
        if handle_events()?.is_break() {
            return Ok(());
        }
    }
}

fn handle_events() -> Result<ControlFlow<()>> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if let KeyCode::Char('q') = key.code {
                return Ok(ControlFlow::Break(()));
            }
        }
    }
    Ok(ControlFlow::Continue(()))
}

fn ui(frame: &mut Frame) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)])
        .split(frame.size());

    // get the model
    let now: DateTime<Utc> = Utc::now();
    let battery = model::get_battery();
    model::get_processes();

    let block = Block::default()
        .title(Title::from("brt").alignment(Alignment::Center))
        .padding(Padding::new(0, 0, frame.size().height / 2 - 1, 0))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .border_type(BorderType::Rounded);
    let paragraph =
        Paragraph::new(Line::from(now.to_string() + " | " + battery.as_str()).centered())
            .block(block);

    frame.render_widget(paragraph, layout[0]);
}

#[cfg(test)]
mod tests {
    use crate::Cli;

    #[test]
    fn verify_cli() {
        use clap::CommandFactory;
        Cli::command().debug_assert()
    }
}
