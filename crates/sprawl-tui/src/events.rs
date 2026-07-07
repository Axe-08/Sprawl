use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

use crate::app::{App, Tab};

pub fn handle_events(app: &mut App) -> std::io::Result<()> {
    // 0% CPU idle -> block indefinitely until event occurs
    if event::poll(Duration::from_secs(60))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Tab => app.next_tab(),
                    KeyCode::BackTab => app.previous_tab(),
                    KeyCode::Char('1') => app.current_tab = Tab::Dashboard,
                    KeyCode::Char('2') => app.current_tab = Tab::SweeperInbox,
                    KeyCode::Char('3') => app.current_tab = Tab::SentinelInbox,
                    KeyCode::Char('4') => app.current_tab = Tab::SemanticSearch,
                    
                    // Basic navigation
                    KeyCode::Down if app.current_tab == Tab::SweeperInbox => {
                        if app.sweeper.selected_index + 1 < app.sweeper.items.len() {
                            app.sweeper.selected_index += 1;
                        }
                    }
                    KeyCode::Up if app.current_tab == Tab::SweeperInbox => {
                        if app.sweeper.selected_index > 0 {
                            app.sweeper.selected_index -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Up => {}
                    _ => {}
                }
            }
        } else if let Event::Mouse(mouse_event) = event::read()? {
            match mouse_event.kind {
                event::MouseEventKind::ScrollDown if app.current_tab == Tab::SweeperInbox => {
                    if app.sweeper.selected_index + 1 < app.sweeper.items.len() {
                        app.sweeper.selected_index += 1;
                    }
                }
                event::MouseEventKind::ScrollUp if app.current_tab == Tab::SweeperInbox => {
                    if app.sweeper.selected_index > 0 {
                        app.sweeper.selected_index -= 1;
                    }
                }
                event::MouseEventKind::ScrollDown | event::MouseEventKind::ScrollUp => {}
                _ => {} // Ignore clicks
            }
        }
    }
    Ok(())
}
