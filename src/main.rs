use chrono::{DateTime, Utc};
use color_eyre::Result;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use app::App;

mod app;
mod battery;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::default().run(terminal).await;
    ratatui::restore();
    result
}


#[derive(Debug, Clone, Default)]
struct TimeWidget {
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
    fn run(&self) {
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

// type OctoPullRequest = octocrab::models::pulls::PullRequest;
//
// impl From<&OctoPullRequest> for PullRequest {
//     fn from(pr: &OctoPullRequest) -> Self {
//         Self {
//             id: pr.number.to_string(),
//             title: pr.title.as_ref().unwrap().to_string(),
//             url: pr
//                 .html_url
//                 .as_ref()
//                 .map(ToString::to_string)
//                 .unwrap_or_default(),
//         }
//     }
// }

impl Widget for &TimeWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let state = self.state.write().unwrap();
        let binding = state.time.format("%H:%M:%S%.3f").to_string();
        let p = Paragraph::new(binding.as_str());
        Widget::render(p, area, buf);
    }
}

