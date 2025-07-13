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

fn bar(percentage: f32) -> Vec<Span<'static>> {
    let block_0 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#d86453").unwrap()),
    );
    let block_1 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#d57b59").unwrap()),
    );
    let block_2 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#d19260").unwrap()),
    );
    let block_3 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#cea966").unwrap()),
    );
    let block_4 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#cbc06c").unwrap()),
    );
    let block_5 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#bac276").unwrap()),
    );
    let block_6 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#a9c47f").unwrap()),
    );
    let block_7 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#98c689").unwrap()),
    );
    let block_8 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#87c892").unwrap()),
    );
    let block_9 = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#77ca9b").unwrap()),
    );
    let blocks = vec![
        block_0, block_1, block_2, block_3, block_4, block_5, block_6, block_7, block_8, block_9,
    ];

    let style_empty = Span::styled(
        "■",
        Style::default().fg(Color::from_str("#404040").unwrap()),
    );
    let empty_bar = vec![style_empty; 10];

    let until = (percentage / 10.0) as usize;
    let filled = &blocks[..until].to_vec();
    let emptied = &empty_bar[until..10].to_vec();
    let mut bar = vec![];
    bar.append(&mut filled.clone());
    bar.append(&mut emptied.clone());
    bar
}

impl Component for BatteryComponent<'_> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
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
            _ => {}
        }
        Ok(None)
    }
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        frame.render_widget(Paragraph::new(self.line.clone()), area);
        Ok(())
    }
}
