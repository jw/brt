use app::App;
use color_eyre::Result;

mod app;
mod battery;
mod procs;
mod time;
mod uptime;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::default().run(terminal).await;
    ratatui::restore();
    result
}
