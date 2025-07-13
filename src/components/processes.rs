use super::Component;
use crate::action::Action;
use color_eyre::Result;
use procfs::process::{all_processes, Stat};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::prelude::{Alignment, Color, Line, Modifier, Style};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    TableState,
};
use ratatui::Frame;
use std::collections::VecDeque;
use tracing::info;

#[allow(dead_code)]
#[derive(Default, Clone, Debug)]
pub struct BrtProcess {
    pub pid: i32,
    pub ppid: i32,
    pub program: String,
    pub command: String,
    pub number_of_threads: i64,
    pub user: Option<String>,
    pub resident_memory: u64,
    pub cpus: VecDeque<f64>,
    pub cpu_graph: String,
    pub cpu: f64,
}

impl From<Stat> for BrtProcess {
    fn from(stat: Stat) -> Self {
        BrtProcess {
            pid: stat.pid,
            ppid: stat.ppid,
            program: stat.comm,
            command: "".to_string(),
            number_of_threads: stat.cguest_time.unwrap(),
            user: None,
            resident_memory: 0,
            cpus: Default::default(),
            cpu_graph: "foo".to_string(),
            cpu: 0.0,
        }
    }
}

#[derive(Default)]
pub struct ProcessesComponent {
    processes: Vec<BrtProcess>,
    scrollbar_state: ScrollbarState,
    state: TableState,
    height: i64,
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
    // let user = process.user.clone();
    // let username = if user.is_some() {
    //     #[allow(clippy::unnecessary_unwrap)]
    //     user.unwrap().name().to_os_string().into_string().unwrap()
    // } else {
    //     "unknown".to_string()
    // };

    let special_style = Style::default().fg(Color::Rgb(0x0D, 0xE7, 0x56));

    // let humansize_options: FormatSizeOptions = FormatSizeOptions::from(BINARY)
    //     .space_after_value(false)
    //     .decimal_places(1)
    //     .decimal_zeroes(0);

    Row::new([
        Cell::new(Line::from(process.pid.to_string()).alignment(Alignment::Right)),
        Cell::new(process.program.to_string()).style(special_style),
        Cell::new(process.command.to_string()),
        Cell::new(
            Line::from(process.number_of_threads.to_string())
                .alignment(Alignment::Right)
                .style(special_style),
        ),
        Cell::new(Line::from("username")),
        Cell::new(
            Line::from("format_size(process.resident_memory, humansize_options)")
                .style(special_style),
        ),
        Cell::new(Line::from("process.cpu_graph.to_string()")),
        Cell::new(format!("{:.2}", process.cpu)).style(special_style),
    ])
}

pub fn create_rows(processes: &Vec<BrtProcess>) -> Vec<Row> {
    let mut rows = Vec::new();
    for process in processes {
        let row = create_row(process);
        rows.push(row);
    }
    rows
}

impl Component for ProcessesComponent {
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
                self.processes = Vec::new();
                for p in all_processes()?.flatten() {
                    if let Ok(stat) = p.stat() {
                        self.processes.push(BrtProcess::from(stat));
                    }
                }
                info!("Updated {} processes.", self.processes.len());
                self.scrollbar_state = self.scrollbar_state.content_length(self.processes.len());
                if self.state.selected().is_none() {
                    self.state.select(Some(0));
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
            Constraint::Length(5),
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
