use crate::theme::ThemeColors;

#[derive(PartialEq)]
pub enum Tab {
    Dashboard,
    SweeperInbox,
    SentinelInbox,
    SemanticSearch,
    Ask,
}

pub struct DashboardState {
    pub disk_usage_mb: u64,
    pub active_projects: usize,
    pub idle_projects: usize,
}

pub struct SweeperInboxState {
    pub selected_index: usize,
    pub items: Vec<String>,
}

use sprawl_sentinel::llm::DiscoveredSecret;
use sprawl_sentinel::classify::SecretClassification;
use sprawl_archivist::SearchResult;
use uuid::Uuid;

pub enum AppEvent {
    BatchClassifyResult(Vec<(Uuid, SecretClassification)>),
    SearchResult(Vec<SearchResult>),
    SearchError(String),
    AskResult(String),
    AskError(String),
    SentinelInboxResult(Vec<DiscoveredSecret>),
}

pub struct InboxItem {
    pub secret: DiscoveredSecret,
    pub review: Option<SecretClassification>,
    pub expanded: bool,
}

pub struct SentinelInboxState {
    pub items: Vec<InboxItem>,
    pub selected_index: usize,
    pub batch_running: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SearchPhase { Input, Results }

pub struct SearchState {
    pub query: String,
    pub phase: SearchPhase,
    pub results: Vec<SearchResult>,
    pub selected_index: usize,
    pub is_searching: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AskPhase { Input, Waiting, Answered }

pub struct AskState {
    pub query: String,
    pub phase: AskPhase,
    pub answer: String,
}

pub struct App {
    pub current_tab: Tab,
    pub dashboard: DashboardState,
    pub sweeper: SweeperInboxState,
    pub sentinel: SentinelInboxState,
    pub search: SearchState,
    pub ask: AskState,
    pub theme: ThemeColors,
    pub should_quit: bool,
    pub input_mode: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            current_tab: Tab::Dashboard,
            dashboard: DashboardState {
                disk_usage_mb: 0,
                active_projects: 0,
                idle_projects: 0,
            },
            sweeper: SweeperInboxState {
                selected_index: 0,
                // Demo items injected at startup from sprawl-dev; production starts empty
                #[cfg(feature = "demo-data")]
                items: vec![
                    "~/Projects/old-api/node_modules   387MB   idle 45d   [X] Nuke  [A] Archive  [S] Snooze".into(),
                    "[!] Config Overridden by Project Local".into(),
                ],
                #[cfg(not(feature = "demo-data"))]
                items: Vec::new(),
            },
            sentinel: SentinelInboxState {
                #[cfg(feature = "demo-data")]
                items: vec![
                    InboxItem {
                        secret: DiscoveredSecret {
                            id: Uuid::new_v4(),
                            raw_value: "Vq1B9x...F5t".to_string(),
                            filepath: ".env:14".to_string(),
                        },
                        review: None,
                        expanded: false,
                    },
                    InboxItem {
                        secret: DiscoveredSecret {
                            id: Uuid::new_v4(),
                            raw_value: "aXk9Pm...4sR".to_string(),
                            filepath: ".env:27".to_string(),
                        },
                        review: None,
                        expanded: false,
                    },
                ],
                #[cfg(not(feature = "demo-data"))]
                items: Vec::new(),
                selected_index: 0,
                batch_running: false,
            },
            search: SearchState {
                query: String::new(),
                phase: SearchPhase::Input,
                results: Vec::new(),
                selected_index: 0,
                is_searching: false,
            },
            ask: AskState {
                query: String::new(),
                phase: AskPhase::Input,
                answer: String::new(),
            },
            theme: ThemeColors::from_terminal(),
            should_quit: false,
            input_mode: false,
        }
    }

    pub fn sentinel_accept_selected(&mut self) {
        if let Some(item) = self.sentinel.items.get_mut(self.sentinel.selected_index) {
            item.review = Some(SecretClassification::KnownProvider("User".to_string()));
            // Sync with keyring would happen here or via event loop
        }
    }

    pub fn sentinel_reject_selected(&mut self) {
        if let Some(item) = self.sentinel.items.get_mut(self.sentinel.selected_index) {
            item.review = Some(SecretClassification::FilteredNoise("User".to_string()));
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Dashboard => Tab::SweeperInbox,
            Tab::SweeperInbox => Tab::SentinelInbox,
            Tab::SentinelInbox => Tab::SemanticSearch,
            Tab::SemanticSearch => Tab::Ask,
            Tab::Ask => Tab::Dashboard,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Dashboard => Tab::SemanticSearch,
            Tab::SweeperInbox => Tab::Dashboard,
            Tab::SentinelInbox => Tab::SweeperInbox,
            Tab::SemanticSearch => Tab::SentinelInbox,
            Tab::Ask => Tab::SemanticSearch,
        };
    }
}
