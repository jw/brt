use std::error::Error;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::fmt;
use std::string::ToString;
use battery::State;
use battery::units::{Energy, Ratio, Time};
use ratatui::prelude::Widget;
use ratatui::layout::Rect;
use ratatui::buffer::Buffer;
use ratatui::widgets::Paragraph;

#[derive(Debug, Clone, Default)]
pub struct BatteryWidget {
    state: Arc<RwLock<BatteryState>>,
}

#[derive(Debug, Default, Clone, Copy)]
struct BatteryState {
    state_of_health: Ratio,
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
                        state.state_of_health = battery.state_of_health();
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
        state.state_of_health = battery_state.state_of_health;

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
        let p = Paragraph::new(format!("BAT{} {:?} ==== xyz", get_state_symbol(state.state), state.state_of_health));  // , state.state_of_health));
        Widget::render(p, area, buf);
    }
}