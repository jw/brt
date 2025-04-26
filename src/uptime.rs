use crate::app::INTERVAL;
use procfs::{FromRead, Uptime};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Line, Widget};
use ratatui::text::Span;
use std::fs::File;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct UptimeWidget {
    state: Arc<RwLock<UptimeState>>,
}

#[derive(Debug, Clone)]
struct UptimeState {
    uptime: Duration,
}

impl Default for UptimeState {
    fn default() -> Self {
        Self {
            uptime: Duration::new(0, 0),
        }
    }
}

impl UptimeWidget {
    pub fn run(&self) {
        let this = self.clone(); // clone the widget to pass to the background task
        tokio::spawn(this.time());
    }
    async fn time(self) {
        let mut interval = tokio::time::interval(Duration::from_millis(INTERVAL));
        loop {
            let f = File::open("/proc/uptime").expect("no /proc/uptime found");
            let uptime = Uptime::from_read(f);
            self.on_load(&uptime.unwrap().uptime_duration());
            interval.tick().await;
        }
    }
    fn on_load(&self, uptime: &Duration) {
        let mut state = self.state.write().unwrap();
        state.uptime = *uptime;
    }
}

impl Widget for &UptimeWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let state = self.state.write().unwrap();
        let span = Span::raw(format!("up {:?}", state.uptime));
        let line = Line::from(span);
        Widget::render(line, area, buf);
    }
}
