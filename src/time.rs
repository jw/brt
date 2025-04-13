use ratatui::prelude::Widget;
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;
use ratatui::widgets::Paragraph;
use chrono::{DateTime, Utc};
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct TimeWidget {
    state: Arc<RwLock<TimeState>>,
}

#[derive(Debug, Clone)]
struct TimeState {
    time: DateTime<Utc>,
}

impl Default for TimeState {
    fn default() -> Self { Self { time: Utc::now() } }
}

impl TimeWidget {
    pub fn run(&self) {
        let this = self.clone(); // clone the widget to pass to the background task
        tokio::spawn(this.time());
    }
    async fn time(self) {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            let now = Utc::now();
            self.on_load(&now);
            interval.tick().await;
        }
    }
    fn on_load(&self, time: &DateTime<Utc>) {
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