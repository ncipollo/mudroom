mod app;
mod event;
mod layout;

pub use app::App;

pub fn run_client() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let result = event::run(&mut terminal, &mut app);
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();
    result
}
