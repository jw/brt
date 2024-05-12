use battery as battery_model;
use battery::State;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;

use crate::action::Action;
use crate::components::Component;
use crate::tui::Frame;

#[derive(Debug)]
pub struct Battery {
    battery: Option<battery_model::Battery>,
}

impl Default for Battery {
    fn default() -> Self {
        Self::new()
    }
}

impl Battery {
    pub fn new() -> Self {
        Self { battery: None }
    }
}

impl Component for Battery {
    fn init(&mut self) -> color_eyre::Result<()> {
        let batteries = battery_model::Manager::new().unwrap().batteries();
        if batteries.is_ok() {
            let b = batteries.unwrap().next().unwrap().unwrap();
            self.battery = Some(b);
        }
        Ok(())
    }

    fn update(&mut self, _action: Action) -> color_eyre::Result<Option<Action>> {
        let _ = self.init();
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> color_eyre::Result<()> {
        let layout =
            Layout::new(Direction::Horizontal, vec![Constraint::Percentage(100)]).split(rect);
        let mut state = "○";
        if self.battery.is_some() {
            state = match self.battery.as_mut().unwrap().state() {
                State::Charging => "▲",
                State::Discharging => "▼",
                State::Full => "■",
                State::Unknown => "○",
                State::Empty => "○",
                _ => "○",
            };
        }
        let soc = self.battery.as_mut().unwrap().state_of_charge().value * 100.0;
        let percentage = format!("{}%", soc as u32);
        let status = format!("{}{} {}", "BAT", state, percentage);
        let line = Line::from(status);
        f.render_widget(line, layout[0]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::info;
    use ratatui::{backend::TestBackend, prelude::*};

    #[test]
    fn test_battery() {
        let mut battery = Battery::default();
        let _ = battery.init();
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let _ = terminal.draw(|frame| {
            let _r = battery.draw(frame, Rect::new(3, 3, 10, 1));
            let b = frame.buffer_mut();
            info!("{:#?}", b);
        });
        assert_eq!(true, true)
    }
}
