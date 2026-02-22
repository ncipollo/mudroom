use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::DefaultTerminal;

use super::app::App;
use super::layout;

pub fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while !app.should_quit {
        terminal.draw(|frame| layout::render(frame, app))?;

        match event::read()? {
            Event::Key(key) => match (key.modifiers, key.code) {
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    app.should_quit = true;
                }
                (_, KeyCode::Char(c)) => {
                    app.input.push(c);
                }
                (_, KeyCode::Backspace) => {
                    app.input.pop();
                }
                (_, KeyCode::Enter) => {
                    let msg = app.input.drain(..).collect();
                    app.messages.push(msg);
                    app.scroll_offset = 0;
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp => app.scroll_up(),
                MouseEventKind::ScrollDown => app.scroll_down(),
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}
