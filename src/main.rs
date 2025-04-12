use battery::units::{Energy, Ratio, Time};
use battery::{Manager, State};
use chrono::{DateTime, Utc};
use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame};
use std::error::Error;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::default().run(terminal).await;
    ratatui::restore();
    result
}

#[derive(Debug, Default)]
struct App {
    should_quit: bool,
    battery_widget: BatteryWidget,
    time_widget: TimeWidget,
}

impl App {
    const FRAMES_PER_SECOND: f32 = 60.0;

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let _ = self.battery_widget.run();
        self.time_widget.run();

        let period = Duration::from_secs_f32(1.0 / Self::FRAMES_PER_SECOND);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while !self.should_quit {
            tokio::select! {
                _ = interval.tick() => { terminal.draw(|frame| self.draw(frame))?; },
                Some(Ok(event)) = events.next() => self.handle_event(&event),
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(frame.area());
        frame.render_widget(&self.battery_widget, layout[0]);
        frame.render_widget(&self.time_widget, layout[1]);
    }

    fn handle_event(&mut self, event: &Event) {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    KeyCode::Char('j') | KeyCode::Down => self.battery_widget.scroll_down(),
                    KeyCode::Char('k') | KeyCode::Up => self.battery_widget.scroll_up(),
                    _ => {}
                }
            }
        }
    }
}

/// A widget that displays a list of pull requests.
///
/// This is an async widget that fetches the battery information. It contains
/// an inner `Arc<RwLock<BatteryState>>` that holds the state of the widget. Cloning the
/// widget will clone the Arc, so you can pass it around to other threads, and this is used to spawn
/// a background task to fetch the pull requests.
#[derive(Debug, Clone, Default)]
struct BatteryWidget {
    // manager: Arc<RwLock<Manager>>,
    state: Arc<RwLock<BatteryState>>,
}

#[derive(Debug, Default)]
struct BatteryState {
    loading_state: LoadingState,
    battery: BrtBattery,
}

#[derive(Debug, Default, Clone, Copy)]
struct BrtBattery {
    state_of_health: Ratio,
    time_to_empty: Option<Time>,
    time_to_full: Option<Time>,
    energy: Energy,
    state: State,
}

impl fmt::Display for BrtBattery {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.state)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum LoadingState {
    #[default]
    Idle,
    Loading,
    Loaded,
    // Error(String),
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


impl BatteryWidget {
    /// Start fetching the pull requests in the background.
    ///
    /// This method spawns a background task that fetches the pull requests from the GitHub API.
    /// The result of the fetch is then passed to the `on_load` or `on_err` methods.
    fn run(&self) -> Result<(), Box<dyn Error>> {
        let this = self.clone(); // clone the widget to pass to the background task
        // let mut battery = Arc::new(Mutex::new(manager.batteries()?.next().ok_or("no battery found")));
        tokio::spawn(this.battery());
        Ok(())
    }

    async fn battery(self) {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            self.set_loading_state(LoadingState::Loading);
            let mut brt_battery = BrtBattery::default();
            {
                let manager = battery::Manager::new().expect("Failed to init battery manager");
                {
                    for (_, maybe_battery) in manager.batteries().expect("No battery found").enumerate() {
                        let battery = maybe_battery.expect("No battery found");
                        brt_battery.state_of_health = battery.state_of_health();
                        brt_battery.time_to_empty = battery.time_to_empty();
                        brt_battery.time_to_full = battery.time_to_full();
                        brt_battery.energy = battery.energy();
                        brt_battery.state = battery.state();
                    }
                    self.on_load(&brt_battery);
                }
            }
            interval.tick().await;

        }
    }

    fn on_load(&self, battery: &BrtBattery) {
        let mut state = self.state.write().unwrap();
        state.loading_state = LoadingState::Loaded;
        state.battery = *battery;
    }

    fn set_loading_state(&self, state: LoadingState) {
        self.state.write().unwrap().loading_state = state;
    }

    fn scroll_down(&self) {
    }

    fn scroll_up(&self) {
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

impl Widget for &BatteryWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let state = self.state.write().unwrap();
        let p = Paragraph::new(format!("Battery state: {}", state.battery));
        Widget::render(p, area, buf);
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

