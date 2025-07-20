use super::Component;
use crate::action::Action;
use crate::components::fps::FpsCounter;
use crate::components::uptime::UptimeWidget;
use color_eyre::Result;
use ratatui::prelude::*;

#[derive(Default)]
pub struct Footer<'a> {
    fps: FpsCounter<'a>,
    up: UptimeWidget<'a>,
}

impl Component for Footer<'_> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                let _ = self.fps.update(Action::Tick);
            }
            Action::Render => {
                let _ = self.fps.update(Action::Render);
                let _ = self.up.update(Action::Render);
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

        let paragraph = self.up.widget.clone().left_aligned();
        frame.render_widget(paragraph, left);

        let paragraph = self.fps.widget.clone().right_aligned();
        frame.render_widget(paragraph, right);

        Ok(())
    }
}
