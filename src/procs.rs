use crate::app::INTERVAL;
use procfs::process::{all_processes, Process};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Line, Widget};
use ratatui::text::Span;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct ProcWidget {
    state: Arc<RwLock<ProcState>>,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct Proc {
    pid: i32,
    name: String,
    command: Vec<String>,
    threads: u32,
    user: String,
    mem: u64,
    history: String,
    cpu: f64,
}

impl From<Process> for Proc {
    fn from(p: Process) -> Self {
        Self {
            pid: p.pid,
            name: "name".to_string(),
            command: vec!["one".to_string(), "two".to_string()],
            threads: 0,
            user: "".to_string(),
            mem: 0,
            history: "...".to_string(),
            cpu: 0.0,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct ProcState {
    processes: Vec<Proc>,
}

impl ProcWidget {
    pub fn run(&self) {
        let this = self.clone(); // clone the widget to pass to the background task
        tokio::spawn(this.processes());
    }
    async fn processes(self) {
        let mut interval = tokio::time::interval(Duration::from_millis(INTERVAL));
        loop {
            let mut processes = vec![];
            for prc in all_processes().unwrap().flatten() {
                processes.push(Proc::from(prc));
            }
            self.on_load(processes);
            interval.tick().await;
        }
    }
    fn on_load(&self, processes: Vec<Proc>) {
        let mut state = self.state.write().unwrap();
        state.processes = processes;
    }
}

impl Widget for &ProcWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let state = self.state.write().unwrap();
        let span = Span::raw(format!("{} processes", state.processes.len()));
        let line = Line::from(span);
        Widget::render(line, area, buf);
    }
}
