use super::Component;
use crate::action::Action;
use color_eyre::Result;
use procfs::process::{all_processes, Stat};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::prelude::Constraint::{Fill, Length, Percentage};
use ratatui::prelude::{Alignment, Color, Line, Modifier, Style};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
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
            Action::Update(since) => {
                // info!("!!! Update at ({})", since);
                self.processes = Vec::new();
                for p in all_processes()?.flatten() {
                    if let Ok(stat) = p.stat() {
                        self.processes.push(BrtProcess::from(stat));
                    }
                }
                info!("[update|{}] processes len: {}", since, self.processes.len());
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [_, layout, _] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(frame.area().height - 2),
            Constraint::Length(1),
        ])
        .areas(area);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"))
            .track_symbol(Some(" "))
            .style(Color::White);

        let selected_style = Style::default()
            .bg(Color::Rgb(0xd4, 0x54, 0x54))
            .fg(Color::White)
            .add_modifier(Modifier::BOLD);

        let rows = create_rows(&self.processes);

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
        .style(Style::default().bold());

        let processes = self.processes.len();

        let block = Block::default()
            .title_top(Line::from("proc").alignment(Alignment::Left))
            // .title_top(Line::from(self.order_string()).alignment(Alignment::Right))
            .title_bottom(Line::from(processes.to_string()).alignment(Alignment::Right))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White))
            .border_type(BorderType::Rounded);

        let widths = [
            Percentage(5),
            Percentage(15),
            Fill(1),
            Percentage(5),
            Percentage(5),
            Length(5),
            Length(5),
            Length(5),
        ];

        let table = Table::new(rows, widths)
            .block(block)
            .header(header)
            .row_highlight_style(selected_style);

        let mut scrollbar_state = ScrollbarState::new(self.processes.len()).position(4);

        frame.render_widget(table, layout);
        frame.render_stateful_widget(
            scrollbar,
            layout.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut scrollbar_state,
        );

        Ok(())
    }
}
