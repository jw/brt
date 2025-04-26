use crate::app::INTERVAL;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Line, Widget};
use ratatui::text::Span;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct DebugWidget {
    state: Arc<RwLock<DebugState>>,
}

#[derive(Debug, Default, Clone)]
struct DebugState {
    interval_as_millis: u128,
}

impl DebugWidget {
    pub fn run(&self) {
        let this = self.clone(); // clone the widget to pass to the background task
        tokio::spawn(this.debug());
    }
    async fn debug(self) {
        let mut interval = tokio::time::interval(Duration::from_millis(INTERVAL));
        loop {
            // TODO(jw): Add framerate
            self.on_load(interval.period());
            interval.tick().await;
        }
    }
    fn on_load(&self, interval: Duration) {
        let mut state = self.state.write().unwrap();
        state.interval_as_millis = interval.as_millis();
    }
}

impl Widget for &DebugWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let state = self.state.write().unwrap();
        let span = Span::raw(format!("debug: interval: {}ms", state.interval_as_millis));
        let line = Line::from(span);
        Widget::render(line, area, buf);
    }
}
