use super::Component;
use crate::action::Action;
use color_eyre::Result;
use humansize::{format_size, FormatSizeOptions, BINARY};
use procfs::process::{all_processes, Process, Stat};
use ratatui::layout::{Constraint, Layout, Margin, Rect, Size};
use ratatui::prelude::{Alignment, Color, Line, Modifier, Style};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    TableState,
};
use ratatui::Frame;
use std::collections::VecDeque;
use std::sync::mpsc as std_mpsc;
use std::sync::mpsc::{Receiver as ThreadReceiver, Sender as ThreadSender};
use std::thread;
use tracing::info;
use uzers::get_user_by_uid;

#[allow(dead_code)]
#[derive(Default, Clone, Debug)]
pub struct BrtProcess {
    pub pid: i32,
    pub ppid: i32,
    pub program: String,
    pub command: String,
    pub number_of_threads: i64,
    pub user: String,
    pub resident_memory: u64,
    pub cpus: VecDeque<f64>,
    pub cpu_graph: String,
    pub cpu: f64,
    pub stat: Option<Stat>,
    pub cmdline: Option<Vec<String>>,
}

fn get_uid_as_string(process: &Process) -> String {
    match process.uid() {
        Ok(uid) => {
            if let Some(user) = get_user_by_uid(uid) {
                if let Some(user) = user.name().to_str() {
                    user.to_string()
                } else {
                    "<unknown>".to_string()
                }
            } else {
                "<unknown>".to_string()
            }
        }
        Err(_) => "<unknown>".to_string(),
    }
}

impl TryFrom<Process> for BrtProcess {
    type Error = ();
    fn try_from(process: Process) -> Result<Self, Self::Error> {
        if let Ok(stat) = process.stat() {
            let new_stat = stat.clone();
            Ok(Self {
                pid: stat.pid,
                ppid: stat.ppid,
                program: stat.comm,
                command: get_cmdline_as_string(&process),
                number_of_threads: stat.num_threads,
                user: get_uid_as_string(&process),
                resident_memory: get_memory(&process),
                cpus: Default::default(),
                cpu_graph: "foo".to_string(),
                cpu: 0.0,
                stat: Some(new_stat),
                cmdline: None,
            })
        } else {
            Err(())
        }
    }
}

pub fn get_memory(process: &Process) -> u64 {
    if let Ok(statm) = process.statm() {
        let page_size = procfs::page_size();
        statm.resident * page_size
    } else {
        0
    }
}

fn get_cmdline_as_string(process: &Process) -> String {
    if let Ok(cmdline) = process.cmdline() {
        cmdline.join(" ")
    } else {
        "<zombie>".to_string()
    }
}

#[derive(Default)]
pub struct ProcessesComponent {
    processes: Vec<BrtProcess>,
    scrollbar_state: ScrollbarState,
    state: TableState,
    height: i64,
    rx: Option<ThreadReceiver<Vec<BrtProcess>>>,
}

impl ProcessesComponent {
    pub fn jump(&mut self, steps: i64) {
        let location = self.state.selected().unwrap_or(0) as i64;
        let length = self.processes.len() as i64;
        info!(
            "Move {} steps in [{}..{}] when current location is {}.",
            steps, 0, length, location
        );
        let mut index = location + steps;
        while index < 0 {
            index += length;
        }
        let new_location = (index % length) as usize;
        info!("New location is {}.", new_location);
        self.state.select(Some(new_location));
        self.scrollbar_state = self.scrollbar_state.position(new_location);
    }
}

pub fn create_row<'a>(process: &BrtProcess) -> Row<'a> {
    let special_style = Style::default().fg(Color::Rgb(0x0D, 0xE7, 0x56));

    let humansize_options: FormatSizeOptions = FormatSizeOptions::from(BINARY)
        .space_after_value(false)
        .decimal_places(1)
        .decimal_zeroes(0);

    Row::new([
        Cell::new(Line::from(process.pid.to_string()).alignment(Alignment::Right)),
        Cell::new(process.program.to_string()).style(special_style),
        Cell::new(process.command.to_string()),
        Cell::new(
            Line::from(process.number_of_threads.to_string())
                .alignment(Alignment::Right)
                .style(special_style),
        ),
        Cell::new(Line::from(process.user.to_string())),
        Cell::new(
            Line::from(format_size(process.resident_memory, humansize_options))
                .style(special_style)
                .alignment(Alignment::Right),
        ),
        Cell::new(Line::from("process.cpu_graph.to_string()")),
        Cell::new(format!("{:.2}", process.cpu)).style(special_style),
    ])
}

pub fn create_rows(processes: &Vec<BrtProcess>) -> Vec<Row<'_>> {
    let mut rows = Vec::new();
    for process in processes {
        let row = create_row(process);
        rows.push(row);
    }
    rows
}

pub fn get_all_processes() -> Vec<BrtProcess> {
    info!("Getting all processes...");
    let mut processes = Vec::new();
    for p in all_processes().expect("Could not get processes").flatten() {
        if let Ok(process) = BrtProcess::try_from(p) {
            processes.push(process);
        }
    }
    info!("{} processes found.", processes.len());
    processes
}

fn processes_thread(thread_tx: ThreadSender<Vec<BrtProcess>>) {
    loop {
        // TODO(jw): Add timing
        let processes = get_all_processes();
        thread_tx.send(processes).unwrap();
    }
}

impl Component for ProcessesComponent {
    fn init(&mut self, _area: Size) -> Result<()> {
        info!("ProcessesComponent::init start");
        let (thread_tx, thread_rx) = std_mpsc::channel();
        self.rx = Some(thread_rx);

        let _ = thread::spawn(move || {
            processes_thread(thread_tx);
        });
        info!("ProcessesComponent::init end");
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            Action::Up => self.jump(-1),
            Action::Down => self.jump(1),
            Action::PageUp => self.jump(-self.height),
            Action::PageDown => self.jump(self.height),
            Action::Update(_since) => {
                if let Some(rx) = self.rx.as_mut() {
                    let mut latest: Option<Vec<BrtProcess>> = None;
                    #[allow(clippy::while_let_loop)]
                    loop {
                        match rx.try_recv() {
                            Ok(v) => latest = Some(v),
                            Err(_) => break,
                        }
                    }
                    if let Some(processes) = latest {
                        self.processes = processes;
                        info!("Updated {} processes.", self.processes.len());
                        self.scrollbar_state =
                            self.scrollbar_state.content_length(self.processes.len());
                        if self.state.selected().is_none() && !self.processes.is_empty() {
                            self.state.select(Some(0));
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [_, layout, _] = Layout::vertical([
            Constraint::Length(1),                     // brt and battery
            Constraint::Fill(frame.area().height - 2), // the process table
            Constraint::Length(1),                     // debug line
        ])
        .areas(area);

        // used by the PageUp and PageDown action
        self.height = (layout.height - 4) as i64;

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some(" "))
            .style(Color::White);

        let selected_row_style = Style::default()
            .bg(Color::Rgb(0xd4, 0x54, 0x54))
            .add_modifier(Modifier::BOLD);

        let header = [
            Cell::new(Line::from("Pid:").alignment(Alignment::Right)),
            Cell::new("Program:"),
            Cell::new("Command:"),
            Cell::new(Line::from("Threads:").alignment(Alignment::Right)),
            Cell::new("User:"),
            Cell::new("MemB"),
            Cell::new(""),
            Cell::new("Cpu%"),
        ]
        .iter()
        .cloned()
        .collect::<Row>()
        .height(1)
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
        );

        let rows = create_rows(&self.processes);
        let processes = self.processes.len();
        let process = format!("{}/{}", self.state.selected().unwrap_or(0) + 1, processes);

        let block = Block::default()
            .title_top(Line::from("proc").alignment(Alignment::Left))
            .title_bottom(Line::from(process.to_string()).alignment(Alignment::Right))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .border_type(BorderType::Rounded);

        let widths = [
            Constraint::Percentage(5),
            Constraint::Percentage(15),
            Constraint::Fill(1),
            Constraint::Percentage(5),
            Constraint::Percentage(5),
            Constraint::Length(7),
            Constraint::Length(5),
            Constraint::Length(5),
        ];

        let table = Table::new(rows, widths)
            .block(block)
            .header(header)
            .row_highlight_style(selected_row_style);

        frame.render_stateful_widget(table, layout, &mut self.state);
        frame.render_stateful_widget(
            scrollbar,
            layout.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scrollbar_state,
        );

        Ok(())
    }
}
