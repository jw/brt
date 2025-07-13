use chrono::{DateTime, Local};
use color_eyre::Result;
use ratatui::text::ToSpan;
use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::action::Action;
use crate::components::battery::BatteryComponent;

#[derive(Default)]
pub struct Header<'a> {
    time: DateTime<Local>,
    battery: BatteryComponent<'a>,
    _interval: u32,
}

impl Component for Header<'_> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                let _ = self.battery.update(Action::Render);
                self.time = Local::now();
            }
            _ => {}
        }
        Ok(None)
    }
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [left, center, right] = Layout::horizontal(vec![
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(40),
        ])
        .areas(area);

        let binding = format!("brt {}", env!("CARGO_PKG_VERSION"));
        let paragraph = Paragraph::new(binding.to_span()).left_aligned();
        frame.render_widget(paragraph, left);

        let binding = self.time.format("%H:%M:%S%.3f");
        let paragraph = Paragraph::new(binding.to_span()).centered();
        frame.render_widget(paragraph, center);

        let paragraph = Paragraph::new(self.battery.line.clone()).right_aligned();
        frame.render_widget(paragraph, right);

        Ok(())
    }
}
