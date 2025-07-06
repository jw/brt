use super::Component;
use crate::action::Action;
use color_eyre::Result;
use procfs::process::{all_processes, Stat};
use ratatui::{prelude::*, widgets::*};
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
    // pub user: Option<User>,
    pub resident_memory: u64,
    pub cpus: VecDeque<f64>,
    // pub cpu_graph: String,
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
            resident_memory: 0,
            cpus: Default::default(),
            cpu: 0.0,
        }
    }
}

#[derive(Default)]
pub struct ProcessesComponent {
    processes: Vec<BrtProcess>,
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
        let processes = format!("processes: {}", self.processes.len());
        frame.render_widget(Paragraph::new(processes), area);
        Ok(())
    }
}
