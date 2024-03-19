use std::{collections::HashMap, time::Duration};

use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use log::{debug, error, info};
use ratatui::layout::Constraint::Percentage;
use ratatui::widgets::block::{Position, Title};
use ratatui::widgets::TableState;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tui_input::{backend::crossterm::EventHandler, Input};

use super::{model, Component, Frame};
use crate::action::Action;
use crate::components::model::BrtProcess;

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Insert,
    Processing,
}

pub struct Process {
    pub show_help: bool,
    pub app_ticker: usize,
    pub render_ticker: usize,
    pub mode: Mode,
    pub input: Input,
    pub processes: Vec<BrtProcess>,
    pub scrollbar_state: ScrollbarState,
    pub state: TableState,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub keymap: HashMap<KeyEvent, Action>,
    pub last_events: Vec<KeyEvent>,
}

impl Default for Process {
    fn default() -> Process {
        let processes = Self::get_processes();
        let length = processes.len();
        Process {
            show_help: false,
            app_ticker: 0,
            render_ticker: 0,
            mode: Default::default(),
            input: Default::default(),
            processes,
            scrollbar_state: Self::get_scrollbar_state(length),
            state: TableState::new().with_selected(Some(0)),
            action_tx: None,
            keymap: Default::default(),
            last_events: vec![],
        }
    }
}

impl Process {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn keymap(mut self, keymap: HashMap<KeyEvent, Action>) -> Self {
        self.keymap = keymap;
        self
    }

    pub fn tick(&mut self) {
        self.app_ticker = self.app_ticker.saturating_add(1);
        self.last_events.drain(..);
    }

    pub fn get_processes() -> Vec<BrtProcess> {
        let processes = model::get_all_processes();
        let processes = model::get_processes(&processes);
        info!("Found {} processes.", processes.len());
        processes
    }

    pub fn get_scrollbar_state(length: usize) -> ScrollbarState {
        ScrollbarState::new(length)
    }

    pub fn render_tick(&mut self) {
        log::debug!("Render Tick");
        self.render_ticker = self.render_ticker.saturating_add(1);
    }

    pub fn schedule_increment(&mut self, i: usize) {
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::EnterProcessing).unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            tx.send(Action::Increment(i)).unwrap();
            tx.send(Action::ExitProcessing).unwrap();
        });
    }

    pub fn schedule_decrement(&mut self, i: usize) {
        let tx = self.action_tx.clone().unwrap();
        tokio::spawn(async move {
            tx.send(Action::EnterProcessing).unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
            tx.send(Action::Decrement(i)).unwrap();
            tx.send(Action::ExitProcessing).unwrap();
        });
    }

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

#[allow(dead_code)]
fn j(length: i64, i: i64, steps: i64) -> i64 {
    let mut index = i + steps;
    while index < 0 {
        index += length;
    }
    index % length
}

#[test]
fn test_jump() {
    let length = 50;
    let i = 10;
    let steps = 20;
    assert_eq!(j(length, i, steps), 30);
    let i = 30;
    let steps = -20;
    assert_eq!(j(length, i, steps), 10);
    let i = 40;
    let steps = 25;
    assert_eq!(j(length, i, steps), 15);
    let i = 40;
    let steps = 200;
    assert_eq!(j(length, i, steps), 40);
    let i = 40;
    let steps = 205;
    assert_eq!(j(length, i, steps), 45);
    let i = 40;
    let steps = -10;
    assert_eq!(j(length, i, steps), 30);
    let i = 40;
    let steps = -40;
    assert_eq!(j(length, i, steps), 0);
    let i = 10;
    let steps = -10;
    assert_eq!(j(length, i, steps), 0);
    let i = 10;
    let steps = -20;
    assert_eq!(j(length, i, steps), 40);
    let i = 10;
    let steps = -11;
    assert_eq!(j(length, i, steps), 49);
    let i = 10;
    let steps = -150;
    assert_eq!(j(length, i, steps), 10);
    let i = 10;
    let steps = -155;
    assert_eq!(j(length, i, steps), 5);
}

impl Component for Process {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_key_events(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        self.last_events.push(key);
        debug!("handling {:?}.", key);
        let action = match self.mode {
            Mode::Normal | Mode::Processing => match key.code {
                KeyCode::Up => Action::Up,
                KeyCode::Down => Action::Down,
                KeyCode::PageUp => Action::PageUp,
                KeyCode::PageDown => Action::PageDown,
                KeyCode::Esc => Action::Quit,
                _ => Action::Update,
            },
            // return Ok(None),
            Mode::Insert => match key.code {
                KeyCode::Esc => Action::EnterNormal,
                KeyCode::Enter => {
                    if let Some(sender) = &self.action_tx {
                        if let Err(e) =
                            sender.send(Action::CompleteInput(self.input.value().to_string()))
                        {
                            error!("Failed to send action: {:?}", e);
                        }
                    }
                    Action::EnterNormal
                }
                _ => {
                    self.input.handle_event(&crossterm::event::Event::Key(key));
                    Action::Update
                }
            },
        };
        Ok(Some(action))
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => self.tick(),
            Action::Render => self.render_tick(),
            Action::ToggleShowHelp => self.show_help = !self.show_help,
            Action::ScheduleIncrement => self.schedule_increment(1),
            Action::ScheduleDecrement => self.schedule_decrement(1),
            Action::Up => self.jump(-1),
            Action::Down => self.jump(1),
            Action::PageUp => self.jump(-20),
            Action::PageDown => self.jump(20),
            Action::EnterNormal => {
                self.mode = Mode::Normal;
            }
            Action::EnterInsert => {
                self.mode = Mode::Insert;
            }
            Action::EnterProcessing => {
                self.mode = Mode::Processing;
            }
            Action::ExitProcessing => {
                // TODO: Make this go to previous mode instead
                self.mode = Mode::Normal;
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

        let rows = model::create_rows(&self.processes);

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

        let processes = self.processes.len();
        let process = format!("{}/{}", self.state.selected().unwrap() + 1, processes);

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