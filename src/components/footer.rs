use super::Component;
use crate::action::Action;
use crate::components::fps::FpsCounter;
use color_eyre::Result;
use ratatui::text::ToSpan;
use ratatui::{prelude::*, widgets::*};

#[derive(Default)]
pub struct Footer<'a> {
    fps: FpsCounter<'a>,
    _up: Line<'a>,
}

impl Component for Footer<'_> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                let _ = self.fps.update(Action::Tick);
            }
            Action::Render => {
                let _ = self.fps.update(Action::Render);
            }
            _ => {}
        }
        Ok(None)
    }
    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [_, bottom] = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Min(0), Constraint::Length(1)])
            .areas(area);
        let [left, _, right] = Layout::horizontal(vec![
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(40),
        ])
        .areas(bottom);

        let paragraph = Paragraph::new("up".to_span()).left_aligned();
        frame.render_widget(paragraph, left);

        let paragraph = self.fps.widget.clone().right_aligned();
        frame.render_widget(paragraph, right);

        Ok(())
    }
}
