use std::{
    error::Error,
    io::{stdout, Stdout},
    ops::ControlFlow,
    time::Duration,
};

use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::{info, warn};
use procfs::process::{all_processes, Process};
use ratatui::layout::Constraint::Percentage;
use ratatui::widgets::{Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table};
use ratatui::{
    prelude::*,
    widgets::{block::Title, Block, BorderType, Borders},
};
use uzers::get_user_by_uid;

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
    initialize_panic_handler();
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
            use KeyCode::*;
            match key.code {
                Char('q') | Esc => return Ok(ControlFlow::Break(())),
                _ => {}
            }
        }
    }
    Ok(ControlFlow::Continue(()))
}

pub fn initialize_panic_handler() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen).unwrap();
        crossterm::terminal::disable_raw_mode().unwrap();
        original_hook(panic_info);
    }));
}

fn ui(frame: &mut Frame) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)])
        .split(frame.size());

    //
    // Model
    //

    info!("Battery: {}", model::get_battery());

    let mut rows = Vec::new();
    let all_processes: Vec<Process> = all_processes()
        .expect("Can't read /proc")
        .filter_map(|p| match p {
            Ok(p) => Some(p),
            Err(e) => match e {
                procfs::ProcError::NotFound(_) => None,
                procfs::ProcError::Io(_e, _path) => None,
                x => {
                    println!("Can't read process due to error {x:?}");
                    None
                }
            },
        })
        .collect();
    for process in all_processes {
        let mut cells = Vec::new();
        let stat_result = process.stat();
        match stat_result {
            Ok(stat) => {
                let pid = Cell::new(stat.pid.to_string());
                cells.push(pid);
                let ppid = Cell::new(stat.ppid.to_string());
                cells.push(ppid);
                let command = Cell::new(stat.comm);
                cells.push(command);
                let number_of_threads = Cell::new(stat.num_threads.to_string());
                cells.push(number_of_threads);

                let mut user_name: String = "unknown".to_string();
                let uid_result = process.uid();
                match uid_result {
                    Ok(uid) => {
                        let user_option = get_user_by_uid(uid);
                        match user_option {
                            Some(u) => {
                                user_name = u.name().to_os_string().into_string().unwrap();
                            }
                            None => {
                                warn!("Nu user found found for {}", process.pid().to_string());
                            }
                        }
                        info!(
                            "user {}; {}; {:?}",
                            uid,
                            process.pid().to_string(),
                            user_name
                        );
                    }
                    Err(_e) => {
                        warn!("Nu user found for {}", process.pid().to_string());
                        break;
                    }
                }
                let user = Cell::new(user_name);
                cells.push(user);

                let mem = Cell::new(stat.vsize.to_string());
                cells.push(mem);
                let cpu = Cell::new("n/a".to_string());
                cells.push(cpu);
            }
            Err(_e) => {
                warn!("Stat not found for {}", process.pid().to_string());
                break;
            }
        }
        rows.push(Row::new(cells));
    }

    //
    // ui
    //

    let vertical_scroll = 0;

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .track_symbol(Some(" "));

    let mut scrollbar_state = ScrollbarState::new(rows.len()).position(vertical_scroll);

    let header = [
        "Pid:", "Ppid:", "Command:", "Threads:", "User:", "MemB", "Cpu%",
    ]
    .iter()
    .cloned()
    .map(Cell::from)
    .collect::<Row>()
    .height(1);

    let block = Block::default()
        .title(Title::from("brt").alignment(Alignment::Center))
        // .padding(Padding::new(0, 0, frame.size().height / 2 - 1, 0))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .border_type(BorderType::Rounded);

    let widths = [
        Percentage(5),
        Percentage(5),
        Percentage(70),
        Percentage(5),
        Percentage(5),
        Percentage(5),
        Percentage(5),
    ];
    let table = Table::new(rows, widths).block(block).header(header);

    frame.render_widget(table, layout[0]);
    frame.render_stateful_widget(
        scrollbar,
        layout[0].inner(&Margin {
            vertical: 1,
            horizontal: 1,
        }),
        &mut scrollbar_state,
    );
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
