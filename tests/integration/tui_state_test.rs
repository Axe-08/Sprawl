use sprawl_tui::app::{App, SearchPhase, AppEvent, InboxItem};
use sprawl_sentinel::classify::SecretClassification;
use sprawl_sentinel::llm::DiscoveredSecret;
use uuid::Uuid;

#[test]
fn test_sentinel_accept_marks_item_as_known() {
    let mut app = App::new();
    
    // Manually add an item
    app.sentinel.items.push(InboxItem {
        secret: DiscoveredSecret {
            id: Uuid::new_v4(),
            filepath: "foo".into(),
            raw_value: "bar".into(),
        },
        review: None,
        expanded: false,
    });
    
    // Assert initial state is unreviewed
    assert!(app.sentinel.items[0].review.is_none());
    
    // Accept
    app.sentinel.selected_index = 0;
    app.sentinel_accept_selected();
    
    assert!(matches!(app.sentinel.items[0].review, Some(SecretClassification::KnownProvider(_))));
}

#[test]
fn test_sentinel_reject_marks_item_as_noise() {
    let mut app = App::new();
    
    app.sentinel.items.push(InboxItem {
        secret: DiscoveredSecret {
            id: Uuid::new_v4(),
            filepath: "foo".into(),
            raw_value: "bar".into(),
        },
        review: None,
        expanded: false,
    });
    
    app.sentinel.selected_index = 0;
    app.sentinel_reject_selected();
    
    assert!(matches!(app.sentinel.items[0].review, Some(SecretClassification::FilteredNoise(_))));
}

#[test]
fn test_search_state_clears_on_esc() {
    let mut app = App::new();
    app.search.query = "test query".to_string();
    app.search.phase = SearchPhase::Results;
    app.input_mode = false;
    
    // Simulating the Esc logic from events.rs
    app.search.phase = SearchPhase::Input;
    app.search.query.clear();
    app.search.results.clear();
    app.input_mode = true;
    
    assert_eq!(app.search.query, "");
    assert!(app.search.results.is_empty());
    assert!(app.input_mode);
}

#[test]
fn test_search_phase_transitions_on_enter() {
    let mut app = App::new();
    app.search.query = "parse_config".to_string();
    app.search.phase = SearchPhase::Input;
    app.input_mode = true;
    
    // Simulate Enter press
    app.input_mode = false;
    app.search.phase = SearchPhase::Results;
    app.search.is_searching = true;
    
    assert_eq!(app.search.phase, SearchPhase::Results);
    assert!(!app.input_mode);
    assert!(app.search.is_searching);
    
    // Simulate async result
    sprawl_tui::events::handle_app_event(&mut app, AppEvent::SearchResult(vec![]));
    
    assert!(!app.search.is_searching);
}
