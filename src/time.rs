use crate::app::INTERVAL;
use chrono::{DateTime, Local};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::Widget;
use ratatui::widgets::Paragraph;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct TimeWidget {
    state: Arc<RwLock<TimeState>>,
}

#[derive(Debug, Clone)]
struct TimeState {
    time: DateTime<Local>,
}

impl Default for TimeState {
    fn default() -> Self {
        Self { time: Local::now() }
    }
}

impl TimeWidget {
    pub fn run(&self) {
        let this = self.clone(); // clone the widget to pass to the background task
        tokio::spawn(this.time());
    }
    async fn time(self) {
        let mut interval = tokio::time::interval(Duration::from_millis(INTERVAL));
        loop {
            let now = Local::now();
            self.on_load(&now);
            interval.tick().await;
        }
    }
    fn on_load(&self, time: &DateTime<Local>) {
        let mut state = self.state.write().unwrap();
        state.time = *time;
    }
}

impl Widget for &TimeWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let state = self.state.write().unwrap();
        let binding = state.time.format("%H:%M:%S%.3f").to_string();
        let p = Paragraph::new(binding.as_str());
        Widget::render(p, area, buf);
    }
}
