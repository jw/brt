use super::Component;
use crate::action::Action;
use color_eyre::Result;
use humansize::{format_size, FormatSizeOptions, BINARY};
use procfs::process::{all_processes, Process};
use ratatui::layout::{Constraint, Layout, Margin, Rect, Size};
use ratatui::prelude::{Alignment, Color, Line, Modifier, Style};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
    TableState,
};
use ratatui::Frame;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc as std_mpsc;
use std::sync::mpsc::{Receiver as ThreadReceiver, Sender as ThreadSender};
use std::thread;
use std::time::Duration;
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
    pub cpu: f64,
    pub cpu_ticks: u64,
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
            Ok(Self {
                pid: stat.pid,
                ppid: stat.ppid,
                program: stat.comm,
                command: get_cmdline_as_string(&process),
                number_of_threads: stat.num_threads,
                user: get_uid_as_string(&process),
                resident_memory: get_memory(&process),
                cpu: 0.0,
                cpu_ticks: stat.utime + stat.stime,
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

fn cpu_percentage(
    previous_ticks: u64,
    current_ticks: u64,
    elapsed: Duration,
    ticks_per_second: u64,
) -> f64 {
    if elapsed.is_zero() {
        return 0.0;
    }
    let delta_ticks = current_ticks.saturating_sub(previous_ticks);
    // `delta_ticks` only ever advances in whole kernel-tick units, but
    // `elapsed` is a continuous wall-clock duration that's essentially
    // never an exact multiple of one tick. Dividing a quantized numerator
    // by a continuous denominator systematically undershoots — a process
    // fully busy for the whole window still shows just under 100% (see
    // issue #65). Rounding elapsed to the nearest whole tick puts both
    // sides of the ratio in the same unit, so a genuinely fully-busy
    // window reaches exactly 100%.
    let elapsed_ticks = (elapsed.as_secs_f64() * ticks_per_second as f64).round();
    if elapsed_ticks <= 0.0 {
        return 0.0;
    }
    delta_ticks as f64 / elapsed_ticks * 100.0
}

// ~4s of trend at the 200ms Action::Update cadence, rendered as half as
// many Braille characters (two samples per character, see cpu_graph).
const CPU_HISTORY_LEN: usize = 20;

// Unicode Braille Patterns: each codepoint is 0x2800 | bits, with dots
// arranged in a 2-wide x 4-tall grid:
//   row0: left=0x01  right=0x08
//   row1: left=0x02  right=0x10
//   row2: left=0x04  right=0x20
//   row3: left=0x40  right=0x80
// Indexed by a 0..=4 bar height, filling dots bottom-up (row3 first) so a
// column reads as a little bar chart rather than a single dot.
const LEFT_DOTS: [u8; 5] = [0x00, 0x40, 0x44, 0x46, 0x47];
const RIGHT_DOTS: [u8; 5] = [0x00, 0x80, 0xA0, 0xB0, 0xB8];

fn dot_height(pct: f64) -> usize {
    ((pct / 100.0).clamp(0.0, 1.0) * 4.0).round() as usize
}

/// Renders a CPU%-over-time history as Braille bar-graph characters, two
/// samples (older, newer) packed per character for double the horizontal
/// density of a one-sample-per-character scheme.
fn cpu_graph(history: &VecDeque<f64>) -> String {
    history
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|pair| {
            let left = LEFT_DOTS[dot_height(pair[0])];
            let right = pair.get(1).map_or(0, |&p| RIGHT_DOTS[dot_height(p)]);
            char::from_u32(0x2800 | left as u32 | right as u32).unwrap()
        })
        .collect()
}

#[derive(Default)]
pub struct ProcessesComponent {
    processes: Vec<BrtProcess>,
    cpu_history: HashMap<i32, VecDeque<f64>>,
    scrollbar_state: ScrollbarState,
    state: TableState,
    height: i64,
    rx: Option<ThreadReceiver<Vec<BrtProcess>>>,
}

impl ProcessesComponent {
    pub fn jump(&mut self, steps: i64) {
        if self.processes.is_empty() {
            return;
        }
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

    fn record_cpu_history(&mut self, pid: i32, cpu: f64) {
        let history = self.cpu_history.entry(pid).or_default();
        history.push_back(cpu);
        if history.len() > CPU_HISTORY_LEN {
            history.pop_front();
        }
    }
}

pub fn create_row<'a>(process: &BrtProcess, cpu_history: &VecDeque<f64>) -> Row<'a> {
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
        Cell::new(cpu_graph(cpu_history)).style(special_style),
        Cell::new(format!("{:.2}", process.cpu)).style(special_style),
    ])
}

pub fn create_rows<'a>(
    processes: &Vec<BrtProcess>,
    cpu_history: &HashMap<i32, VecDeque<f64>>,
) -> Vec<Row<'a>> {
    let empty_history = VecDeque::new();
    let mut rows = Vec::new();
    for process in processes {
        let history = cpu_history.get(&process.pid).unwrap_or(&empty_history);
        let row = create_row(process, history);
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

// Matches tui.rs's `component_interval` cadence, which is how often
// Action::Update actually drains this thread's channel; scanning any
// faster than that just burns a core for samples nobody reads (issue #44).
const PROCESS_SCAN_INTERVAL: Duration = Duration::from_millis(200);

fn processes_thread(thread_tx: ThreadSender<Vec<BrtProcess>>) {
    loop {
        let processes = get_all_processes();
        if thread_tx.send(processes).is_err() {
            break;
        }
        thread::sleep(PROCESS_SCAN_INTERVAL);
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
            Action::Update(since) => {
                if let Some(rx) = self.rx.as_mut() {
                    let mut latest: Option<Vec<BrtProcess>> = None;
                    #[allow(clippy::while_let_loop)]
                    loop {
                        match rx.try_recv() {
                            Ok(v) => latest = Some(v),
                            Err(_) => break,
                        }
                    }
                    if let Some(mut processes) = latest {
                        // Known simplification: a pid reused within one tick window would
                        // be (wrongly) matched to the exited process's ticks.
                        let elapsed = Duration::from_millis(since as u64);
                        let ticks_per_second = procfs::ticks_per_second();
                        let previous_ticks: HashMap<i32, u64> = self
                            .processes
                            .iter()
                            .map(|process| (process.pid, process.cpu_ticks))
                            .collect();
                        for process in &mut processes {
                            if let Some(&previous) = previous_ticks.get(&process.pid) {
                                process.cpu = cpu_percentage(
                                    previous,
                                    process.cpu_ticks,
                                    elapsed,
                                    ticks_per_second,
                                );
                            }
                            self.record_cpu_history(process.pid, process.cpu);
                        }
                        let live_pids: HashSet<i32> =
                            processes.iter().map(|process| process.pid).collect();
                        self.cpu_history.retain(|pid, _| live_pids.contains(pid));
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
            Constraint::Length(1),                                   // brt and battery
            Constraint::Fill(frame.area().height.saturating_sub(2)), // the process table
            Constraint::Length(1),                                   // debug line
        ])
        .areas(area);

        // used by the PageUp and PageDown action
        self.height = layout.height.saturating_sub(4) as i64;

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
            Cell::new("Graph"),
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

        let rows = create_rows(&self.processes, &self.cpu_history);
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
            Constraint::Length(10),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn component_with_processes(count: usize, selected: usize) -> ProcessesComponent {
        let mut state = TableState::default();
        state.select(Some(selected));
        ProcessesComponent {
            processes: vec![BrtProcess::default(); count],
            state,
            ..Default::default()
        }
    }

    #[test]
    fn jump_on_empty_process_list_does_not_panic() {
        let mut component = ProcessesComponent::default();

        component.jump(1);

        assert_eq!(
            component.state.selected(),
            None,
            "no row should become selected when there are no processes"
        );
    }

    #[test]
    fn jump_forward_past_the_last_row_wraps_to_the_first() {
        let mut component = component_with_processes(3, 2);

        component.jump(1);

        assert_eq!(component.state.selected(), Some(0));
    }

    #[test]
    fn jump_backward_past_the_first_row_wraps_to_the_last() {
        let mut component = component_with_processes(3, 0);

        component.jump(-1);

        assert_eq!(component.state.selected(), Some(2));
    }

    #[test]
    fn cpu_percentage_is_zero_when_no_time_has_elapsed() {
        let percentage = cpu_percentage(0, 100, Duration::ZERO, 100);

        assert_eq!(percentage, 0.0);
    }

    #[test]
    fn cpu_percentage_is_zero_when_ticks_are_unchanged() {
        let percentage = cpu_percentage(100, 100, Duration::from_secs(1), 100);

        assert_eq!(percentage, 0.0);
    }

    #[test]
    fn cpu_percentage_of_a_fully_busy_core() {
        let percentage = cpu_percentage(0, 100, Duration::from_secs(1), 100);

        assert_eq!(percentage, 100.0);
    }

    #[test]
    fn cpu_percentage_does_not_panic_when_ticks_go_backward() {
        let percentage = cpu_percentage(100, 50, Duration::from_secs(1), 100);

        assert_eq!(percentage, 0.0);
    }

    #[test]
    fn cpu_percentage_reaches_100_for_a_fully_busy_core_despite_sub_tick_jitter() {
        // 203ms of wall-clock time, 20 whole ticks (10ms each) accumulated:
        // the extra 3ms haven't produced a 21st tick yet, but the core was
        // busy for the entire window. Before rounding elapsed to whole
        // ticks, this undershot to 20 / 100 / 0.203 * 100 ≈ 98.5%.
        let percentage = cpu_percentage(0, 20, Duration::from_millis(203), 100);

        assert_eq!(percentage, 100.0);
    }

    #[test]
    fn cpu_percentage_is_zero_when_elapsed_is_shorter_than_one_tick() {
        let percentage = cpu_percentage(0, 1, Duration::from_millis(3), 100);

        assert_eq!(percentage, 0.0);
    }

    #[test]
    fn cpu_graph_renders_a_full_pair_at_the_extremes() {
        let history = VecDeque::from([0.0, 100.0]);

        let graph = cpu_graph(&history);

        assert_eq!(graph, char::from_u32(0x28B8).unwrap().to_string());
    }

    #[test]
    fn cpu_graph_renders_mid_range_levels() {
        let history = VecDeque::from([25.0, 75.0]);

        let graph = cpu_graph(&history);

        assert_eq!(graph, char::from_u32(0x28F0).unwrap().to_string());
    }

    #[test]
    fn cpu_graph_renders_a_trailing_unpaired_sample_with_only_its_left_dots() {
        let history = VecDeque::from([50.0]);

        let graph = cpu_graph(&history);

        assert_eq!(graph, char::from_u32(0x2844).unwrap().to_string());
    }

    #[test]
    fn record_cpu_history_keeps_only_the_most_recent_samples() {
        let mut component = ProcessesComponent::default();

        for sample in 0..(CPU_HISTORY_LEN + 5) {
            component.record_cpu_history(1, sample as f64);
        }

        let history = &component.cpu_history[&1];
        assert_eq!(history.len(), CPU_HISTORY_LEN);
        assert_eq!(history.front(), Some(&5.0));
        assert_eq!(history.back(), Some(&(CPU_HISTORY_LEN as f64 + 4.0)));
    }

    #[test]
    fn action_update_computes_cpu_percentage_from_the_previous_sample() {
        let (tx, rx) = std_mpsc::channel();
        let ticks_per_second = procfs::ticks_per_second();
        let mut component = ProcessesComponent {
            rx: Some(rx),
            processes: vec![BrtProcess {
                pid: 1,
                cpu_ticks: 100,
                ..Default::default()
            }],
            ..Default::default()
        };

        tx.send(vec![BrtProcess {
            pid: 1,
            cpu_ticks: 100 + ticks_per_second,
            ..Default::default()
        }])
        .unwrap();
        component.update(Action::Update(1000)).unwrap();

        assert_eq!(component.processes[0].cpu, 100.0);
    }

    #[test]
    fn action_update_leaves_cpu_percentage_at_zero_for_a_newly_seen_process() {
        let (tx, rx) = std_mpsc::channel();
        let mut component = ProcessesComponent {
            rx: Some(rx),
            ..Default::default()
        };

        tx.send(vec![BrtProcess {
            pid: 42,
            cpu_ticks: 500,
            ..Default::default()
        }])
        .unwrap();
        component.update(Action::Update(1000)).unwrap();

        assert_eq!(component.processes[0].cpu, 0.0);
    }

    #[test]
    fn action_update_prunes_cpu_history_for_processes_that_have_exited() {
        let (tx, rx) = std_mpsc::channel();
        let mut component = ProcessesComponent {
            rx: Some(rx),
            processes: vec![BrtProcess {
                pid: 1,
                cpu_ticks: 100,
                ..Default::default()
            }],
            ..Default::default()
        };
        component.cpu_history.insert(1, VecDeque::from([10.0]));
        component.cpu_history.insert(2, VecDeque::from([20.0]));

        tx.send(vec![BrtProcess {
            pid: 1,
            cpu_ticks: 200,
            ..Default::default()
        }])
        .unwrap();
        component.update(Action::Update(1000)).unwrap();

        assert!(
            component.cpu_history.contains_key(&1),
            "a still-running pid's history should survive"
        );
        assert!(
            !component.cpu_history.contains_key(&2),
            "an exited pid's history should be pruned"
        );
    }
}
