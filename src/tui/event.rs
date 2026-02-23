use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers, MouseEventKind};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use crate::network::NetworkEvent;

use super::app::App;
use super::layout;

pub async fn run(
    terminal: &mut DefaultTerminal,
    app: &mut App,
    mut net_rx: mpsc::Receiver<NetworkEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_stream = EventStream::new();

    while !app.should_quit {
        terminal.draw(|frame| layout::render(frame, app))?;

        tokio::select! {
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => match (key.modifiers, key.code) {
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
                    Some(Ok(Event::Mouse(mouse))) => match mouse.kind {
                        MouseEventKind::ScrollUp => app.scroll_up(),
                        MouseEventKind::ScrollDown => app.scroll_down(),
                        _ => {}
                    },
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
            maybe_net = net_rx.recv() => {
                if let Some(event) = maybe_net {
                    app.handle_network_event(event);
                }
            }
        }
    }

    Ok(())
}
