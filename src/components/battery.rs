use chrono::{DateTime, Local};
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use color_eyre::Result;
use ratatui::text::ToText;
use tokio::sync::mpsc::UnboundedSender;
use crate::action::Action;
use crate::config::Config;
use super::Component;

#[derive(Debug, Clone, Default)]
pub struct TimeWidget {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    time: DateTime<Local>,
}

impl Component for TimeWidget {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

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
        frame.render_widget(Paragraph::new(self.time.format("%H:%M:%S%.3f").to_text()), area);
        Ok(())
    }

}
