use crate::battery::BatteryWidget;
use crate::time::TimeWidget;
use crossterm::event::{Event, EventStream, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::{DefaultTerminal, Frame};
use std::time::Duration;
use tokio_stream::StreamExt;

#[derive(Debug, Default)]
pub struct App {
    should_quit: bool,
    battery_widget: BatteryWidget,
    time_widget: TimeWidget,
}

impl App {
    const FRAMES_PER_SECOND: f32 = 60.0;

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        let _ = self.battery_widget.run();
        self.time_widget.run();

        let period = Duration::from_secs_f32(1.0 / Self::FRAMES_PER_SECOND);
        let mut interval = tokio::time::interval(period);
        let mut events = EventStream::new();

        while !self.should_quit {
            tokio::select! {
                _ = interval.tick() => { terminal.draw(|frame| self.draw(frame))?; },
                Some(Ok(event)) = events.next() => self.handle_event(&event),
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.area());
        frame.render_widget(&self.battery_widget, layout[0]);
        frame.render_widget(&self.time_widget, layout[1]);
    }

    fn handle_event(&mut self, event: &Event) {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
                    KeyCode::Char('j') | KeyCode::Down => self.battery_widget.scroll_down(),
                    KeyCode::Char('k') | KeyCode::Up => self.battery_widget.scroll_up(),
                    _ => {}
                }
            }
        }
    }
}
