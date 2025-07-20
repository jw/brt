use super::Component;
use crate::action::Action;
use color_eyre::Result;
use procfs::{FromRead, Uptime};
use ratatui::layout::Rect;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;
use std::fs::File;
use std::io::BufReader;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct UptimeWidget<'a> {
    pub widget: Paragraph<'a>,
    uptime: Duration,
}

fn widget<'a>(uptime: &mut UptimeWidget) -> Paragraph<'a> {
    let message = format!("up {} seconds", uptime.uptime.as_secs_f64());
    let span = Span::styled(message, Style::new().dim());
    let paragraph = Paragraph::new(span).right_aligned();
    paragraph
}

impl Component for UptimeWidget<'_> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                // add any logic here that should run on every tick
            }
            Action::Render => {
                // add any logic here that should run on every render
                let f = File::open("/proc/uptime");
                if let Ok(f) = f {
                    let f = BufReader::new(f);
                    if let Ok(uptime) = Uptime::from_read(f) {
                        self.uptime = Duration::new(uptime.uptime as u64, 0);
                        self.widget = widget(self);
                    };
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        frame.render_widget(self.widget.clone(), area);
        Ok(())
    }
}
