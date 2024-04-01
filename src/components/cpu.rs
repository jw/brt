use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;

use crate::action::Action;
use crate::components::Component;
use crate::tui::Frame;

#[derive(Debug)]
pub struct Cpu {
    state: bool,
}

impl Default for Battery {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self { state: false }
    }
}

impl Component for Cpu {
    fn init(&mut self) -> color_eyre::Result<()> {
        Ok(())
    }

    fn update(&mut self, _action: Action) -> color_eyre::Result<Option<Action>> {
        let _ = self.init();
        Ok(None)
    }

    fn draw(&mut self, f: &mut Frame<'_>, rect: Rect) -> color_eyre::Result<()> {
        let layout =
            Layout::new(Direction::Horizontal, vec![Constraint::Percentage(100)]).split(rect);
        let message = format!("{}", ".");
        let line = Line::from(status);
        f.render_widget(line, layout[0]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, prelude::*};

    #[test]
    fn test_cpu() {
        let mut cpu = Cpu::default();
        let _ = battery.init();
        let backend = TestBackend::new(40, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let _ = terminal.draw(|frame| {
            let _r = battery.draw(frame, Rect::new(3, 3, 10, 1));
            let b = frame.buffer_mut();
            println!("{:#?}", b);
        });
        assert_eq!(true, true)
    }
}
