use crossterm::event::{Event, KeyCode, KeyEventKind};
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{App, AppEvent, SearchPhase, Tab};

pub fn handle_crossterm_event(
    app: &mut App,
    event: Event,
    tx: UnboundedSender<AppEvent>,
) -> std::io::Result<()> {
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Press {
            if app.input_mode {
                if app.current_tab == Tab::SemanticSearch && app.search.phase == SearchPhase::Input
                {
                    match key.code {
                        KeyCode::Esc => {
                            app.input_mode = false;
                            app.search.query.clear();
                        }
                        KeyCode::Enter => {
                            app.input_mode = false;
                            app.search.phase = SearchPhase::Results;
                            app.search.is_searching = true;

                            let tx_clone = tx.clone();
                            let _query = app.search.query.clone();

                            tokio::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(300)).await;

                                let mock_result = sprawl_archivist::SearchResult {
                                    project_id: "demo-project".to_string(),
                                    file_path: "src/main.rs".to_string(),
                                    start_line: 1,
                                    end_line: 8,
                                    similarity_score: 0.95,
                                    chunk_text: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
                                };
                                let _ = tx_clone.send(AppEvent::SearchResult(vec![mock_result]));
                            });
                        }
                        KeyCode::Char(c) => {
                            app.search.query.push(c);
                        }
                        KeyCode::Backspace => {
                            app.search.query.pop();
                        }
                        _ => {}
                    }
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Tab => {
                        app.next_tab();
                        if app.current_tab == Tab::SemanticSearch && app.search.phase == SearchPhase::Input {
                            app.input_mode = true;
                        }
                    }
                    KeyCode::BackTab => {
                        app.previous_tab();
                        if app.current_tab == Tab::SemanticSearch && app.search.phase == SearchPhase::Input {
                            app.input_mode = true;
                        }
                    }
                    KeyCode::Char('1') => app.current_tab = Tab::Dashboard,
                    KeyCode::Char('2') => app.current_tab = Tab::SweeperInbox,
                    KeyCode::Char('3') => app.current_tab = Tab::SentinelInbox,
                    KeyCode::Char('4') => {
                        app.current_tab = Tab::SemanticSearch;
                        if app.search.phase == SearchPhase::Input {
                            app.input_mode = true;
                        }
                    }
                    KeyCode::Esc => {
                        if app.current_tab == Tab::SemanticSearch {
                            app.search.phase = SearchPhase::Input;
                            app.search.query.clear();
                            app.search.results.clear();
                            app.input_mode = true;
                        }
                    }

                    // Sentinel specific bindings
                    KeyCode::Down if app.current_tab == Tab::SentinelInbox => {
                        if app.sentinel.selected_index + 1 < app.sentinel.items.len() {
                            app.sentinel.selected_index += 1;
                        }
                    }
                    KeyCode::Up if app.current_tab == Tab::SentinelInbox => {
                        if app.sentinel.selected_index > 0 {
                            app.sentinel.selected_index -= 1;
                        }
                    }
                    KeyCode::Char('k') if app.current_tab == Tab::SentinelInbox => {
                        app.sentinel_accept_selected();
                    }
                    KeyCode::Char('n') if app.current_tab == Tab::SentinelInbox => {
                        app.sentinel_reject_selected();
                    }
                    KeyCode::Enter if app.current_tab == Tab::SentinelInbox => {
                        if let Some(item) = app.sentinel.items.get_mut(app.sentinel.selected_index)
                        {
                            item.expanded = !item.expanded;
                        }
                    }
                    KeyCode::Char('W') if app.current_tab == Tab::SentinelInbox => {
                        if !app.sentinel.batch_running {
                            app.sentinel.batch_running = true;
                            let items = app
                                .sentinel
                                .items
                                .iter()
                                .filter(|i| i.review.is_none())
                                .map(|i| i.secret.clone())
                                .collect::<Vec<_>>();

                            let tx_clone = tx.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                                let mut results = Vec::new();
                                for s in items {
                                    results.push((
                                        s.id,
                                        sprawl_sentinel::classify::SecretClassification::FilteredNoise("mock".to_string()),
                                    ));
                                }
                                let _ = tx_clone.send(AppEvent::BatchClassifyResult(results));
                            });
                        }
                    }

                    // Sweeper specific bindings
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

                    // Semantic Search Results bindings
                    KeyCode::Down
                        if app.current_tab == Tab::SemanticSearch
                            && app.search.phase == SearchPhase::Results =>
                    {
                        if app.search.selected_index + 1 < app.search.results.len() {
                            app.search.selected_index += 1;
                        }
                    }
                    KeyCode::Up
                        if app.current_tab == Tab::SemanticSearch
                            && app.search.phase == SearchPhase::Results =>
                    {
                        if app.search.selected_index > 0 {
                            app.search.selected_index -= 1;
                        }
                    }

                    _ => {}
                }
            }
        }
    } else if let Event::Mouse(mouse_event) = event {
        match mouse_event.kind {
            crossterm::event::MouseEventKind::ScrollDown => {
                if app.current_tab == Tab::SweeperInbox {
                    if app.sweeper.selected_index + 1 < app.sweeper.items.len() {
                        app.sweeper.selected_index += 1;
                    }
                } else if app.current_tab == Tab::SentinelInbox {
                    if app.sentinel.selected_index + 1 < app.sentinel.items.len() {
                        app.sentinel.selected_index += 1;
                    }
                } else if app.current_tab == Tab::SemanticSearch
                    && app.search.phase == SearchPhase::Results
                {
                    if app.search.selected_index + 1 < app.search.results.len() {
                        app.search.selected_index += 1;
                    }
                }
            }
            crossterm::event::MouseEventKind::ScrollUp => {
                if app.current_tab == Tab::SweeperInbox {
                    if app.sweeper.selected_index > 0 {
                        app.sweeper.selected_index -= 1;
                    }
                } else if app.current_tab == Tab::SentinelInbox {
                    if app.sentinel.selected_index > 0 {
                        app.sentinel.selected_index -= 1;
                    }
                } else if app.current_tab == Tab::SemanticSearch
                    && app.search.phase == SearchPhase::Results
                {
                    if app.search.selected_index > 0 {
                        app.search.selected_index -= 1;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn handle_app_event(app: &mut App, event: AppEvent) {
    match event {
        AppEvent::BatchClassifyResult(results) => {
            app.sentinel.batch_running = false;
            for (id, classification) in results {
                if let Some(item) = app.sentinel.items.iter_mut().find(|i| i.secret.id == id) {
                    item.review = Some(classification);
                }
            }
        }
        AppEvent::SearchResult(results) => {
            app.search.is_searching = false;
            app.search.results = results;
            app.search.selected_index = 0;
            app.search.phase = SearchPhase::Results;
        }
        AppEvent::SearchError(_) => {
            app.search.is_searching = false;
        }
    }
}
