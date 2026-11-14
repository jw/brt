use super::Component;
use crate::action::Action;
use battery::units::power::watt;
use battery::units::ratio::percent;
use battery::units::time::second;
use battery::{Battery, State};
use color_eyre::Result;
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use std::str::FromStr;
use tracing::{error, warn};

#[derive(Debug, Default, Clone)]
pub struct BatteryComponent<'a> {
    pub line: Line<'a>,
    refresh_attempts: u32,
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

fn line<'a>(battery: Battery) -> Line<'a> {
    let percentage = battery.state_of_charge().get::<percent>();
    let bat = Span::raw(format!(
        "BAT{} {}% ",
        get_state_symbol(battery.state()),
        percentage
    ));
    let mut parts = vec![bat];

    let mut bar = bar(percentage);
    parts.append(&mut bar);

    if let Some(time_to_empty) = battery.time_to_empty() {
        let seconds_to_empty = time_to_empty.get::<second>() as i64;
        let (hours, minutes) = seconds_to_hours_minutes(seconds_to_empty);
        let time_to_empty = Span::raw(format!(" {hours:02}:{minutes:02}"));
        parts.push(time_to_empty);
    }

    if let Some(time_to_full) = battery.time_to_full() {
        let seconds_to_full = time_to_full.get::<second>() as i64;
        let (hours, minutes) = seconds_to_hours_minutes(seconds_to_full);
        let time_to_full = Span::raw(format!(" {hours:02}:{minutes:02}"));
        parts.push(time_to_full);
    }

    let energy_rate = Span::raw(format!(" {:.2}W", battery.energy_rate().get::<watt>()));
    parts.push(energy_rate);
    Line::from(parts)
}

fn seconds_to_hours_minutes(seconds: i64) -> (i64, i64) {
    let hours = seconds / 3600;
    let remaining_seconds = seconds % 3600;
    let minutes = remaining_seconds / 60;
    (hours, minutes)
}

const BAR_COLORS: [&str; 10] = [
    "#d86453", "#d57b59", "#d19260", "#cea966", "#cbc06c", "#bac276", "#a9c47f", "#98c689",
    "#87c892", "#77ca9b",
];
const BAR_EMPTY_COLOR: &str = "#404040";

fn bar(percentage: f32) -> Vec<Span<'static>> {
    let until = ((percentage / 10.0) as usize).min(10);
    BAR_COLORS
        .iter()
        .enumerate()
        .map(|(i, color)| {
            let color = if i < until { color } else { BAR_EMPTY_COLOR };
            Span::styled("■", Style::default().fg(Color::from_str(color).unwrap()))
        })
        .collect()
}

impl Component for BatteryComponent<'_> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                self.refresh_attempts += 1;
                let manager = battery::Manager::new()?;
                let battery = match manager.batteries()?.next() {
                    Some(Ok(battery)) => battery,
                    Some(Err(_)) => {
                        error!("Unable to access battery information");
                        self.line = Line::default();
                        return Ok(None);
                    }
                    None => {
                        warn!("Unable to find any batteries");
                        self.line = Line::default();
                        return Ok(None);
                    }
                };
                self.line = line(battery);
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            _ => {}
        }
        Ok(None)
    }
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        frame.render_widget(Paragraph::new(self.line.clone()), area);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refreshes_on_tick_rather_than_on_render() {
        let mut battery = BatteryComponent::default();

        battery.update(Action::Render).unwrap();
        assert_eq!(
            battery.refresh_attempts, 0,
            "Render should not query battery hardware"
        );

        battery.update(Action::Tick).unwrap();
        assert_eq!(
            battery.refresh_attempts, 1,
            "Tick should query battery hardware"
        );
    }
}
