use std::collections::HashMap;
use std::default::Default;
use std::fmt;

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use log::{debug, info, warn};
use procfs::process::all_processes;
use ratatui::layout::Constraint::{Fill, Length, Percentage};
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::TableState;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tui_input::Input;

use super::{Component, Frame};
use crate::action::Action;
use crate::components::process::Order::{Command, Cpu, Name, NumberOfThreads, Pid};
use crate::model::{create_rows, to_brt_process, BrtProcess};

#[derive(Default, Copy, Clone, PartialEq, Eq, Debug)]
pub enum Order {
    #[default]
    Pid,
    Name,
    Command,
    NumberOfThreads,
    Cpu,
}

impl Order {
    fn next(&self) -> Self {
        use Order::*;
        match *self {
            Pid => Name,
            Name => Command,
            Command => NumberOfThreads,
            NumberOfThreads => Cpu,
            Cpu => Pid,
        }
    }

    fn previous(&self) -> Self {
        use Order::*;
        match *self {
            Pid => Cpu,
            Cpu => NumberOfThreads,
            NumberOfThreads => Command,
            Command => Name,
            Name => Pid,
        }
    }
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pid => write!(f, "pid"),
            Name => write!(f, "name"),
            Command => write!(f, "command"),
            NumberOfThreads => write!(f, "threads"),
            Cpu => write!(f, "cpu"),
        }
    }
}

#[derive(Default, Debug)]
pub struct Process {
    pub show_help: bool,
    pub app_ticker: usize,
    pub render_ticker: usize,
    pub input: Input,
    pub processes: HashMap<i32, BrtProcess>,
    pub order: Order,
    pub scrollbar_state: ScrollbarState,
    pub state: TableState,
    pub action_tx: Option<UnboundedSender<Action>>,
}

impl Process {
    pub fn new() -> Process {
        let mut process = Process::default();
        process.processes = process.get_processes();
        process.state = TableState::new().with_selected(Some(0));
        process
    }

    pub fn refresh(&mut self) {
        let length = self.processes.len();
        let new_processes = self.get_processes();
        let mut updated_processes = HashMap::new();
        for (pid, process) in new_processes {
            let old_process_option = self.processes.get(&pid);
            if old_process_option.is_some() {
                let mut old_process = old_process_option.unwrap().clone();
                old_process.cpus.push_back(process.cpu);
                old_process.cpus.pop_front();
                old_process.cpu_graph = crate::model::get_cpu_graph(&old_process.cpus);
                updated_processes.insert(pid, old_process);
            };
        }
        self.processes = updated_processes;
        self.state.select(Some(0));
        self.scrollbar_state = self.scrollbar_state.content_length(length);
    }

    pub fn order_string(&mut self) -> String {
        format!("{} {} {}", "<".red(), self.order, ">".red())
    }

    pub fn tick(&mut self) {
        self.app_ticker = self.app_ticker.saturating_add(1);
        // if self.app_ticker % 5 == 0 {
        // self.processes = self.get_all_processes();
        self.refresh();

        self.order_by_enum();
        info!("Refreshed process list.");
        // }
    }

    fn get_processes(&mut self) -> HashMap<i32, BrtProcess> {
        let processes: HashMap<i32, BrtProcess> = all_processes()
            .expect("Can't read /proc")
            .filter_map(|p| match p {
                Ok(p) => {
                    let brt_process = to_brt_process(&p);
                    if brt_process.is_some() {
                        Some((p.pid, brt_process?))
                    } else {
                        None
                    }
                }
                Err(e) => match e {
                    procfs::ProcError::NotFound(_) => None,
                    procfs::ProcError::Io(_e, _path) => None,
                    x => {
                        warn!("Can't read process due to error {x:?}");
                        None
                    }
                },
            })
            .collect();
        // info!("{} processes: {:#?}", processes.len(), processes);
        processes
    }

    pub fn order_by_enum(&mut self) {
        let order = self.order;
        match order {
            Pid => self.order_by_pid(),
            Name => self.order_by_program(),
            Command => self.order_by_command(),
            NumberOfThreads => self.order_by_number_of_threads(),
            Cpu => self.order_by_cpu(),
        }
    }

    pub fn order_by_pid(&mut self) {
        // self.processes.sort_by(|a, b| a.pid.cmp(&b.pid))
    }

    pub fn order_by_program(&mut self) {
        // self.processes.sort_by(|a, b| a.program.cmp(&b.program))
    }

    pub fn order_by_command(&mut self) {
        // self.processes.sort_by(|a, b| a.command.cmp(&b.command))
    }

    pub fn order_by_number_of_threads(&mut self) {
        // self.processes.sort_by(|a, b| {
        //     a.number_of_threads
        //         .partial_cmp(&b.number_of_threads)
        //         .unwrap()
        // })
    }

    pub fn order_by_cpu(&mut self) {
        // self.processes
        //     .sort_by(|a, b| a.cpu.partial_cmp(&b.cpu).unwrap())
    }

    pub fn render_tick(&mut self) {
        debug!("Render Tick");
        self.render_ticker = self.render_ticker.saturating_add(1);
    }

    pub fn jump(&mut self, steps: i64) {
        let location = self.state.selected().unwrap_or(0) as i64;
        let length = self.processes.len() as i64;
        debug!(
            "Move {} steps in [{}..{}] when current location is {}.",
            steps, 0, length, location
        );
        let mut index = location + steps;
        while index < 0 {
            index += length;
        }
        let new_location = (index % length) as usize;
        debug!("New location is {}.", new_location);
        self.state.select(Some(new_location));
        self.scrollbar_state = self.scrollbar_state.position(new_location);
    }
}

impl Component for Process {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        debug!("Handling {:?}.", key);
        let action = match key.code {
            KeyCode::Up => Action::Up,
            KeyCode::Down => Action::Down,
            KeyCode::PageUp => Action::PageUp,
            KeyCode::PageDown => Action::PageDown,
            KeyCode::Left => Action::Left,
            KeyCode::Right => Action::Right,
            KeyCode::Esc => Action::Quit,
            _ => Action::Update,
        };
        Ok(Some(action))
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => self.tick(),
            Action::Render => self.render_tick(),
            Action::Up => self.jump(-1),
            Action::Down => self.jump(1),
            Action::PageUp => self.jump(-20),
            Action::PageDown => self.jump(20),
            Action::Left => {
                self.order = self.order.previous();
                self.order_by_enum();
            }
            Action::Right => {
                self.order = self.order.next();
                self.order_by_enum();
            }
            _ => (),
        }
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, _rect: Rect) -> Result<()> {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Percentage(100)])
            .split(f.size());

        let rows = create_rows(&self.processes);

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
            Cell::new(""),
            Cell::new("Cpu%"),
        ]
        .iter()
        .cloned()
        .map(Cell::from)
        .collect::<Row>()
        .height(1)
        .style(Style::default().bold());

        let processes = self.processes.len();
        let process = format!("{}/{}", self.state.selected().unwrap() + 1, processes);

        let block = Block::default()
            .title(Title::from("brt").alignment(Alignment::Center))
            .title(Title::from(self.order_string()).alignment(Alignment::Right))
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
            .highlight_style(selected_style);

        f.render_stateful_widget(table, layout[0], &mut self.state);
        f.render_stateful_widget(
            scrollbar,
            layout[0].inner(&Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scrollbar_state,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn test_brt_process_new() {
        let process = BrtProcess::new();
        assert_eq!(process.pid, 0);
        assert_eq!(process.cpus, VecDeque::from(vec![0_f64; 10]));
    }

    #[test]
    fn test_process_jump() {
        let mut process = Process::new();
        process.processes = process.get_processes();
        assert_eq!(process.state.selected(), Some(0));
        process.jump(5);
        assert_eq!(process.state.selected(), Some(5));
        process.jump(5);
        assert_eq!(process.state.selected(), Some(10));
        process.jump(-15);
        assert_eq!(process.state.selected(), Some(process.processes.len() - 5));
        process.jump(4);
        assert_eq!(process.state.selected(), Some(process.processes.len() - 1));
        process.jump(1);
        assert_eq!(process.state.selected(), Some(0));
        process.jump(1);
        assert_eq!(process.state.selected(), Some(1));
    }
}
