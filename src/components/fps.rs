use super::Component;
use color_eyre::Result;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Span,
    widgets::Paragraph,
    Frame,
};
use std::time::Instant;

use crate::action::Action;

#[derive(Debug, Clone, PartialEq)]
pub struct FpsCounter<'a> {
    last_tick_update: Instant,
    tick_count: u32,
    ticks_per_second: f64,

    last_frame_update: Instant,
    frame_count: u32,
    frames_per_second: f64,
    pub widget: Paragraph<'a>,
}

impl Default for FpsCounter<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl FpsCounter<'_> {
    pub fn new() -> Self {
        Self {
            last_tick_update: Instant::now(),
            tick_count: 0,
            ticks_per_second: 0.0,
            last_frame_update: Instant::now(),
            frame_count: 0,
            frames_per_second: 0.0,
            widget: Default::default(),
        }
    }

    fn app_tick(&mut self) -> Result<()> {
        self.tick_count += 1;
        let now = Instant::now();
        let elapsed = (now - self.last_tick_update).as_secs_f64();
        if elapsed >= 1.0 {
            self.ticks_per_second = self.tick_count as f64 / elapsed;
            self.last_tick_update = now;
            self.tick_count = 0;
        }
        Ok(())
    }

    fn render_tick(&mut self) -> Result<()> {
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = (now - self.last_frame_update).as_secs_f64();
        if elapsed >= 1.0 {
            self.frames_per_second = self.frame_count as f64 / elapsed;
            self.last_frame_update = now;
            self.frame_count = 0;
        }
        Ok(())
    }
}

fn widget<'a>(fps: &mut FpsCounter) -> Paragraph<'a> {
    let message = format!(
        "{:.2} ticks/sec, {:.2} FPS",
        fps.ticks_per_second, fps.frames_per_second
    );
    let span = Span::styled(message, Style::new().dim());
    let paragraph = Paragraph::new(span).right_aligned();
    paragraph
}

impl Component for FpsCounter<'_> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => self.app_tick()?,
            Action::Render => self.render_tick()?,
            _ => {}
        };
        self.widget = widget(self);
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [_, bottom] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
        frame.render_widget(self.widget.clone(), bottom);
        Ok(())
    }
}
