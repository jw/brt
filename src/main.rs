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
use procfs::process::{all_processes, Stat};
use ratatui::{
    prelude::*,
    widgets::{block::Title, Block, BorderType, Borders, Padding, Paragraph},
};

struct ProcessEntry {
    stat: Stat,
    cmdline: Option<Vec<String>>,
}

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

fn get_processes() -> () {
    // Get all processes
    let processes: Vec<ProcessEntry> = match all_processes() {
        Err(err) => {
            println!("Failed to read all processes: {}", err);
            return;
        }
        Ok(processes) => processes,
    }
    .filter_map(|v| {
        v.and_then(|p| {
            let stat = p.stat()?;
            let cmdline = p.cmdline().ok();
            Ok(ProcessEntry { stat, cmdline })
        })
        .ok()
    })
    .collect();
    // Iterate through all processes and start with top-level processes.
    // Those can be identified by checking if their parent PID is zero.
    for process in &processes {
        if process.stat.ppid == 0 {
            print_process(process, &processes, 0);
        }
    }
    info!("");
}

/// Take a process, print its command and recursively list all child processes.
/// This function will call itself until no further children can be found.
/// It's a depth-first tree exploration.
///
/// depth: The hierarchical depth of the process
fn print_process(process: &ProcessEntry, all_processes: &Vec<ProcessEntry>, depth: usize) {
    let cmdline = match &process.cmdline {
        Some(cmdline) => cmdline.join(" "),
        None => "zombie process".into(),
    };

    // Some processes seem to have an empty cmdline.
    if cmdline.is_empty() {
        return;
    }

    // 10 characters width for the pid
    let pid_length = 8;
    let mut pid = process.stat.pid.to_string();
    pid.push_str(&" ".repeat(pid_length - pid.len()));

    let padding = " ".repeat(4 * depth);
    info!("{}{}{}", pid, padding, cmdline);

    let children = get_children(process.stat.pid, all_processes);
    for child in &children {
        print_process(child, all_processes, depth + 1);
    }
}

/// Get all children of a specific process, by iterating through all processes and
/// checking their parent pid.
fn get_children(pid: i32, all_processes: &[ProcessEntry]) -> Vec<&ProcessEntry> {
    all_processes
        .iter()
        .filter(|process| process.stat.ppid == pid)
        .collect()
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
    get_processes();
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
