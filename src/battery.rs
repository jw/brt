use battery::units::ratio::percent;
use battery::units::{Energy, Ratio, Time};
use battery::State;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Line, Widget};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use std::error::Error;
use std::fmt;
use std::str::FromStr;
use std::string::ToString;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct BatteryWidget {
    state: Arc<RwLock<BatteryState>>,
}

#[derive(Debug, Default, Clone, Copy)]
struct BatteryState {
    state_of_charge: Ratio,
    time_to_empty: Option<Time>,
    time_to_full: Option<Time>,
    energy: Energy,
    state: State,
}

impl fmt::Display for BatteryState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.state)
    }
}

impl BatteryWidget {
    pub fn run(&self) -> color_eyre::Result<(), Box<dyn Error>> {
        let this = self.clone();
        tokio::spawn(this.battery());
        Ok(())
    }

    async fn battery(self) {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            let mut state = BatteryState::default();
            {
                let manager = battery::Manager::new().expect("Failed to init battery manager");
                {
                    for (_, maybe_battery) in manager.batteries().expect("No battery found").enumerate() {
                        let battery = maybe_battery.expect("No battery found");
                        state.state_of_charge = battery.state_of_charge();
                        state.time_to_empty = battery.time_to_empty();
                        state.time_to_full = battery.time_to_full();
                        state.energy = battery.energy();
                        state.state = battery.state();
                    }
                    self.on_load(&state);
                }
            }
            interval.tick().await;

        }
    }

    fn on_load(&self, battery_state: &BatteryState) {
        let mut state = self.state.write().unwrap();
        state.state = battery_state.state;
        state.state_of_charge = battery_state.state_of_charge;

    }

    pub fn scroll_down(&self) {
    }

    pub fn scroll_up(&self) {
    }
}

static BATTERY_STATE_SYMBOL_UNKNOWN: &str = "?";
static BATTERY_STATE_SYMBOL: &[(State, &str)] = &[
        (State::Charging, "▲"),
        (State::Discharging, "▼"),
        (State::Full, "●"),
        (State::Empty, "○"),
        (State::Unknown, BATTERY_STATE_SYMBOL_UNKNOWN),
    ];

fn get_state_symbol(s: State) -> String {
    if let Some((_, symbol)) = BATTERY_STATE_SYMBOL.iter().find(|(state, _)| s == *state) {
        return symbol.to_string();
    };
    BATTERY_STATE_SYMBOL_UNKNOWN.to_string()
}

impl Widget for &BatteryWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let state = self.state.write().unwrap();
        let percentage = state.state_of_charge.get::<percent>() as i32;
        let line = line(&state.state, &percentage);
        Widget::render(line, area, buf);
    }
}

fn line<'a>(state: &'a State, percentage: &'a i32) -> Line<'a> {
    let bat = Span::raw(format!(
        "BAT{} {}% ",
        get_state_symbol(*state),
        percentage,
    ));
    let mut bar = bar(&percentage);
    let mut parts = vec![bat];
    parts.append(&mut bar);
    Line::from(parts)
}

fn bar(percentage: &i32) -> Vec<Span> {
    let block_0 = Span::styled("■", Style::default().fg(Color::from_str("#d86453").unwrap()));
    let block_1 = Span::styled("■", Style::default().fg(Color::from_str("#d57b59").unwrap()));
    let block_2 = Span::styled("■", Style::default().fg(Color::from_str("#d19260").unwrap()));
    let block_3 = Span::styled("■", Style::default().fg(Color::from_str("#cea966").unwrap()));
    let block_4 = Span::styled("■", Style::default().fg(Color::from_str("#cbc06c").unwrap()));
    let block_5 = Span::styled("■", Style::default().fg(Color::from_str("#bac276").unwrap()));
    let block_6 = Span::styled("■", Style::default().fg(Color::from_str("#a9c47f").unwrap()));
    let block_7 = Span::styled("■", Style::default().fg(Color::from_str("#98c689").unwrap()));
    let block_8 = Span::styled("■", Style::default().fg(Color::from_str("#87c892").unwrap()));
    let block_9 = Span::styled("■", Style::default().fg(Color::from_str("#77ca9b").unwrap()));
    let blocks = vec![block_0, block_1, block_2, block_3, block_4, block_5, block_6, block_7, block_8, block_9];

    let style_empty = Span::styled("■", Style::default().fg(Color::from_str("#404040").unwrap()));
    let empty_bar = vec![style_empty; 10];

    let until = (percentage / 10) as usize;
    let filled = &blocks[..until].to_vec();
    let emptied = &empty_bar[until..10].to_vec();
    let mut bar = vec![];
    bar.append(&mut filled.clone());
    bar.append(&mut emptied.clone());
    bar
}
