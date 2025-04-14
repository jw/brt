use std::error::Error;
use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::fmt;
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

#[derive(Debug, Default)]
struct BatteryState {
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

impl BatteryWidget {
    pub fn run(&self) -> color_eyre::Result<(), Box<dyn Error>> {
        let this = self.clone();
        tokio::spawn(this.battery());
        Ok(())
    }

    async fn battery(self) {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
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
        state.battery = *battery;
    }

    pub fn scroll_down(&self) {
    }

    pub fn scroll_up(&self) {
    }
}

impl Widget for &BatteryWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let state = self.state.write().unwrap();
        let p = Paragraph::new(format!("Battery state: {}", state.battery));
        Widget::render(p, area, buf);
    }
}