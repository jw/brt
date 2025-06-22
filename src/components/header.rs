use chrono::{DateTime, Local};
use color_eyre::Result;
use ratatui::text::ToSpan;
use ratatui::{prelude::*, widgets::*};

use super::Component;
use crate::action::Action;

#[derive(Default)]
pub struct Header {
    time: DateTime<Local>,
    _interval: u32,
}

impl Component for Header {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
                self.time = Local::now();
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [top, _] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
        let binding = self.time.format("%H:%M:%S%.3f");
        let paragraph = Paragraph::new(binding.to_span()).centered();
        frame.render_widget(paragraph, top);
        Ok(())
    }
}
