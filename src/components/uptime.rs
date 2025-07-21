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

const MINUTE: u64 = 60;
const MINUTE_MOD: u64 = 60;
const HOUR: u64 = MINUTE * MINUTE_MOD;
const HOUR_MOD: u64 = 24;
const DAY: u64 = HOUR * HOUR_MOD;
const DAY_MOD: u64 = 7;
const WEEK: u64 = DAY * DAY_MOD;
const WEEK_MOD: u64 = 4;
const MONTH: u64 = WEEK * WEEK_MOD;

#[derive(Debug, Clone, Default)]
pub struct UptimeWidget<'a> {
    pub widget: Paragraph<'a>,
    duration: Duration,
}

fn as_pretty_uptime(duration: Duration) -> String {
    let months = duration.as_secs() / MONTH;
    let months_token = if months == 1 { "month" } else { "months" };
    let weeks = (duration.as_secs() / WEEK) % WEEK_MOD;
    let weeks_token = if weeks == 1 { "week" } else { "weeks" };
    let days = (duration.as_secs() / DAY) % DAY_MOD;
    let days_token = if days == 1 { "day" } else { "days" };
    let hours = (duration.as_secs() / HOUR) % HOUR_MOD;
    let hours_token = if hours == 1 { "hour" } else { "hours" };
    let minutes = (duration.as_secs() / MINUTE) % MINUTE_MOD;
    let minutes_token = if minutes == 1 { "minute" } else { "minutes" };
    let seconds = duration.as_secs() % 60;
    let seconds_token = if seconds == 1 { "second" } else { "seconds" };

    if months == 0 && weeks == 0 && days == 0 && hours == 0 && minutes == 0 {
        format!("{seconds} {seconds_token}")
    } else if months == 0 && weeks == 0 && days == 0 && hours == 0 {
        format!("{minutes} {minutes_token}, {seconds} {seconds_token}")
    } else if months == 0 && weeks == 0 && days == 0 {
        format!("{hours} {hours_token}, {minutes} {minutes_token}, {seconds} {seconds_token}")
    } else if months == 0 && weeks == 0 {
        format!("{days} {days_token}, {hours} {hours_token}, {minutes} {minutes_token}, {seconds} {seconds_token}")
    } else if months == 0 {
        format!("{weeks} {weeks_token}, {days} {days_token}, {hours} {hours_token}, {minutes} {minutes_token}, {seconds} {seconds_token}")
    } else {
        format!("{months} {months_token}, {weeks} {weeks_token}, {days} {days_token}, {hours} {hours_token}, {minutes} {minutes_token}, {seconds} {seconds_token}")
    }
}

fn widget<'a>(uptime: &mut UptimeWidget) -> Paragraph<'a> {
    let message = format!("up {}", as_pretty_uptime(uptime.duration));
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
                let f = File::open("/proc/uptime");
                if let Ok(f) = f {
                    let f = BufReader::new(f);
                    if let Ok(uptime) = Uptime::from_read(f) {
                        self.duration = Duration::new(uptime.uptime as u64, 0);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_pretty_uptime() {
        let result = as_pretty_uptime(Duration::new(5, 0));
        assert_eq!(result, "5 seconds");
        let result = as_pretty_uptime(Duration::new(1, 0));
        assert_eq!(result, "1 second");
        let result = as_pretty_uptime(Duration::new(181, 0));
        assert_eq!(result, "3 minutes, 1 second");
        let result = as_pretty_uptime(Duration::new(61, 0));
        assert_eq!(result, "1 minute, 1 second");
        let result = as_pretty_uptime(Duration::new(3 * MINUTE + 10, 0));
        assert_eq!(result, "3 minutes, 10 seconds");
        let result = as_pretty_uptime(Duration::new(3 * MINUTE + 1, 0));
        assert_eq!(result, "3 minutes, 1 second");
        let result = as_pretty_uptime(Duration::new(3 * MINUTE, 0));
        assert_eq!(result, "3 minutes, 0 seconds");
        let result = as_pretty_uptime(Duration::new(3 * HOUR, 0));
        assert_eq!(result, "3 hours, 0 minutes, 0 seconds");
        let result = as_pretty_uptime(Duration::new(3 * HOUR + 45 * MINUTE + 1, 0));
        assert_eq!(result, "3 hours, 45 minutes, 1 second");
        let result = as_pretty_uptime(Duration::new(10 * HOUR + MINUTE + 10, 0));
        assert_eq!(result, "10 hours, 1 minute, 10 seconds");
        let result = as_pretty_uptime(Duration::new(HOUR + 10, 0));
        assert_eq!(result, "1 hour, 0 minutes, 10 seconds");
        let result = as_pretty_uptime(Duration::new(DAY, 0));
        assert_eq!(result, "1 day, 0 hours, 0 minutes, 0 seconds");
        let result = as_pretty_uptime(Duration::new(DAY + 60, 0));
        assert_eq!(result, "1 day, 0 hours, 1 minute, 0 seconds");
        let result = as_pretty_uptime(Duration::new(7 * DAY, 0));
        assert_eq!(result, "1 week, 0 days, 0 hours, 0 minutes, 0 seconds");
        let result = as_pretty_uptime(Duration::new(2 * WEEK + DAY + 2 * HOUR + MINUTE + 10, 0));
        assert_eq!(result, "2 weeks, 1 day, 2 hours, 1 minute, 10 seconds");
        let result = as_pretty_uptime(Duration::new(7 * WEEK, 0));
        assert_eq!(
            result,
            "1 month, 3 weeks, 0 days, 0 hours, 0 minutes, 0 seconds",
        );
        let result = as_pretty_uptime(Duration::new(7 * WEEK + 2 * DAY + MINUTE + 30, 0));
        assert_eq!(
            result,
            "1 month, 3 weeks, 2 days, 0 hours, 1 minute, 30 seconds",
        );
    }
}
