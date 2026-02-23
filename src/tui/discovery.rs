use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures_util::StreamExt;
use ratatui::DefaultTerminal;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};

use crate::network::discovery::DiscoveredServer;
use crate::network::discovery::client::discover;

const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub struct DiscoveryApp {
    servers: Vec<DiscoveredServer>,
    cursor: usize,
    discovering: bool,
    spinner_tick: u8,
    should_quit: bool,
    selected_url: Option<String>,
}

impl DiscoveryApp {
    fn new() -> Self {
        Self {
            servers: Vec::new(),
            cursor: 0,
            discovering: true,
            spinner_tick: 0,
            should_quit: false,
            selected_url: None,
        }
    }

    fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    fn cursor_down(&mut self) {
        if !self.servers.is_empty() && self.cursor < self.servers.len() - 1 {
            self.cursor += 1;
        }
    }

    fn select(&mut self) {
        if let Some(server) = self.servers.get(self.cursor) {
            self.selected_url = Some(server.url());
            self.should_quit = true;
        }
    }
}

fn render(frame: &mut ratatui::Frame, app: &DiscoveryApp) {
    let areas = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .split(frame.area());

    // Title bar
    let title =
        Paragraph::new("mudroom - Server Discovery").block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, areas[0]);

    // Server list
    let items: Vec<ListItem> = app
        .servers
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == app.cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            ListItem::new(Span::styled(format!("{} — {}", s.name, s.url()), style))
        })
        .collect();

    let list = List::new(items).block(Block::default().title("Servers").borders(Borders::ALL));
    frame.render_widget(list, areas[1]);

    // Status line
    let status_text = if app.discovering {
        let spinner = SPINNER_FRAMES[app.spinner_tick as usize % SPINNER_FRAMES.len()];
        format!("Discovering... {spinner}")
    } else {
        format!("{} server(s) found", app.servers.len())
    };
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(status, areas[2]);

    // Help footer
    let help = Paragraph::new("↑/↓ navigate  |  Enter: connect  |  Ctrl+C: quit")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, areas[3]);
}

pub async fn run(
    terminal: &mut DefaultTerminal,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut app = DiscoveryApp::new();
    let mut event_stream = EventStream::new();
    let mut ticker = interval(Duration::from_millis(100));

    let (discovery_tx, mut discovery_rx) = mpsc::channel::<DiscoveredServer>(32);

    tokio::spawn(async move {
        if let Ok(servers) = discover(3000).await {
            for s in servers {
                if discovery_tx.send(s).await.is_err() {
                    break;
                }
            }
        }
    });

    while !app.should_quit {
        terminal.draw(|frame| render(frame, &app))?;

        tokio::select! {
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key))) => match (key.modifiers, key.code) {
                        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                            app.should_quit = true;
                        }
                        (_, KeyCode::Up) => app.cursor_up(),
                        (_, KeyCode::Char('k')) => app.cursor_up(),
                        (_, KeyCode::Down) => app.cursor_down(),
                        (_, KeyCode::Char('j')) => app.cursor_down(),
                        (_, KeyCode::Enter) => app.select(),
                        _ => {}
                    },
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
            maybe_server = discovery_rx.recv() => {
                match maybe_server {
                    Some(server) => app.servers.push(server),
                    None => app.discovering = false,
                }
            }
            _ = ticker.tick() => {
                app.spinner_tick = app.spinner_tick.wrapping_add(1);
                if !app.discovering {
                    // Still tick for spinner until channel closes
                }
            }
        }
    }

    Ok(app.selected_url)
}
