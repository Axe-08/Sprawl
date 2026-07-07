use crate::theme::ThemeColors;

#[derive(PartialEq)]
pub enum Tab {
    Dashboard,
    SweeperInbox,
    SentinelInbox,
    SemanticSearch,
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

pub struct SentinelInboxState {
    // Scaffold for M17
}

pub struct SearchState {
    // Scaffold for M17
}

pub struct App {
    pub current_tab: Tab,
    pub dashboard: DashboardState,
    pub sweeper: SweeperInboxState,
    pub sentinel: SentinelInboxState,
    pub search: SearchState,
    pub theme: ThemeColors,
    pub should_quit: bool,
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
            sentinel: SentinelInboxState {},
            search: SearchState {},
            theme: ThemeColors::from_terminal(),
            should_quit: false,
        }
    }

    pub fn next_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Dashboard => Tab::SweeperInbox,
            Tab::SweeperInbox => Tab::SentinelInbox,
            Tab::SentinelInbox => Tab::SemanticSearch,
            Tab::SemanticSearch => Tab::Dashboard,
        };
    }

    pub fn previous_tab(&mut self) {
        self.current_tab = match self.current_tab {
            Tab::Dashboard => Tab::SemanticSearch,
            Tab::SweeperInbox => Tab::Dashboard,
            Tab::SentinelInbox => Tab::SweeperInbox,
            Tab::SemanticSearch => Tab::SentinelInbox,
        };
    }
}
