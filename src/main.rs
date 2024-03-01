use std::{
    error::Error,
    io::{stdout, Stdout},
    ops::ControlFlow,
    time::Duration,
};

use crate::model::BrtProcess;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::info;
use procfs::process::Process;
use ratatui::layout::Constraint::Percentage;
use ratatui::widgets::block::Position;
use ratatui::widgets::{
    Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState,
};
use ratatui::{
    prelude::*,
    widgets::{block::Title, Block, BorderType, Borders},
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

struct App {
    state: TableState,
    processes: Vec<BrtProcess>,
    scrollbar_state: ScrollbarState,
}

impl App {
    fn new() -> App {
        let processes = model::get_processes(model::get_all_processes());
        App {
            state: TableState::default().with_selected(0),
            scrollbar_state: ScrollbarState::new(processes.len() - 1),
            processes,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.processes.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scrollbar_state = self.scrollbar_state.position(i);
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.processes.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scrollbar_state = self.scrollbar_state.position(i);
    }
}

#[allow(dead_code)]
fn get_current_process() -> Process {
    let me = Process::myself().unwrap();
    let (virtual_mem, resident_mem) = get_memory(&me);
    info!(
        "Current pid {}; uses {}/{} bytes ({:02.2}%).",
        me.pid,
        virtual_mem,
        resident_mem,
        resident_mem as f64 / virtual_mem as f64 * 100.0
    );
    me
}

fn get_memory(process: &Process) -> (u64, u64) {
    let stat = process.stat().unwrap();
    let page_size = procfs::page_size();
    let virtual_mem = stat.vsize;
    let resident_mem = stat.rss * page_size;
    (virtual_mem, resident_mem)
}

fn main() -> Result<()> {
    logger::initialize_logging();
    initialize_panic_handler();
    info!("{NAME} ({VERSION}) started.");

    let _cli = Cli::parse();

    let mut terminal = setup_terminal()?;

    let app = App::new();

    let result = run(&mut terminal, app);
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

fn run(terminal: &mut Terminal, mut app: App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;
        if handle_events(terminal, &mut app)?.is_break() {
            return Ok(());
        }
    }
}

fn handle_events(_terminal: &mut Terminal, app: &mut App) -> Result<ControlFlow<()>> {
    if event::poll(Duration::from_millis(200))? {
        if let Event::Key(key) = event::read()? {
            use KeyCode::*;
            match key.code {
                Char('q') | Esc => return Ok(ControlFlow::Break(())),
                Char('j') | Down => app.next(),
                Char('k') | Up => app.previous(),
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

fn ui(frame: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Percentage(100)])
        .split(frame.size());

    // info!("Battery state is {}.", model::get_battery());
    let rows = model::create_rows(&app.processes);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .track_symbol(Some(" "))
        .style(Color::White);

    let selected_style = Style::default()
        .bg(Color::Rgb(0xd4, 0x54, 0x54))
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);

    let header = [
        Cell::new(Line::from("Pid:").alignment(Alignment::Right)),
        Cell::new("Program:"),
        Cell::new("Command:"),
        Cell::new(Line::from("Threads:").alignment(Alignment::Right)),
        Cell::new("User:"),
        Cell::new("MemB"),
        Cell::new("Cpu%"),
    ]
    .iter()
    .cloned()
    .map(Cell::from)
    .collect::<Row>()
    .height(1)
    .style(Style::default().bold());

    let processes = app.processes.len();
    let process = format!("{}/{}", app.state.selected().unwrap() + 1, processes);

    let block = Block::default()
        .title(Title::from("brt").alignment(Alignment::Center))
        .title(
            Title::from(process)
                .position(Position::Bottom)
                .alignment(Alignment::Right),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .border_type(BorderType::Rounded);

    let widths = [
        Percentage(5),
        Percentage(15),
        Percentage(60),
        Percentage(5),
        Percentage(5),
        Percentage(5),
        Percentage(5),
    ];

    let table = Table::new(rows, widths)
        .block(block)
        .header(header)
        .highlight_style(selected_style);

    frame.render_stateful_widget(table, layout[0], &mut app.state);
    frame.render_stateful_widget(
        scrollbar,
        layout[0].inner(&Margin {
            vertical: 1,
            horizontal: 1,
        }),
        &mut app.scrollbar_state,
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
