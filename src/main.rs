use std::path::PathBuf;
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
use log::{info, LevelFilter};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;
use ratatui::{
    prelude::*,
    widgets::{block::Title, Block, BorderType, Borders, Padding, Paragraph},
};

const LOG_PATTERN: &str = "{d(%Y-%m-%d %H:%M:%S)} | {l} | {f}:{L} | {m}{n}";
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
    initialize_logging();
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

pub fn initialize_logging() {
    let data_local_dir = if let Ok(s) = std::env::var("BRT_DATA") {
        PathBuf::from(s)
    } else {
        dirs::data_local_dir()
            .expect("Unable to find data directory for brt")
            .join("brt")
    };

    std::fs::create_dir_all(&data_local_dir)
        .unwrap_or_else(|_| panic!("Unable to create {:?}", data_local_dir));

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(LOG_PATTERN)))
        .build(data_local_dir.join("brt.log"))
        .expect("Failed to build log file appender.");

    let levelfilter = match std::env::var("BRT_LOG_LEVEL")
        .unwrap_or_else(|_| "info".to_string())
        .as_str()
    {
        "off" => LevelFilter::Off,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    };
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(Logger::builder().build("brt", levelfilter))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))
        .expect("Failed to build logging config.");

    log4rs::init_config(config).expect("Failed to initialize logging.");
}

fn ui(frame: &mut Frame) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)])
        .split(frame.size());
    let block = Block::default()
        .title(Title::from("brt").alignment(Alignment::Center))
        .padding(Padding::new(0, 0, frame.size().height / 2 - 1, 0))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .border_type(BorderType::Rounded);
    let now: DateTime<Utc> = Utc::now();
    let battery = get_battery();
    let paragraph =
        Paragraph::new(Line::from(now.to_string() + " | " + battery.as_str()).centered())
            .block(block);
    frame.render_widget(paragraph, layout[0]);
}

fn get_battery() -> String {
    let manager = battery::Manager::new().unwrap();
    let battery = manager.batteries().unwrap().nth(0).unwrap().unwrap();
    let percentage = battery.state_of_charge().value * 100.0;
    format!("{}%", percentage)
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
