use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::app::{App, Tab};
use crate::views;

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Check minimum size (80x24)
    if size.width < 80 || size.height < 24 {
        let warning = Paragraph::new("Terminal window too small (Minimum: 80x24). Please resize.")
            .style(Style::default().fg(Color::Red))
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(warning, size);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Content
                Constraint::Length(3), // Footer
            ]
            .as_ref(),
        )
        .split(size);

    draw_tabs(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles = vec![
        Line::from(" [1] Dashboard "),
        Line::from(" [2] Sweeper Inbox "),
        Line::from(" [3] Sentinel "),
        Line::from(" [4] Search "),
        Line::from(" [5] Ask "),
    ];
    let active_index = match app.current_tab {
        Tab::Dashboard => 0,
        Tab::SweeperInbox => 1,
        Tab::SentinelInbox => 2,
        Tab::SemanticSearch => 3,
        Tab::Ask => 4,
    };

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(app.theme.safe)
                .add_modifier(Modifier::BOLD),
        )
        .select(active_index);
    f.render_widget(tabs, area);
}

fn draw_content(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default().borders(Borders::ALL);
    match app.current_tab {
        Tab::Dashboard => {
            let text = if app.dashboard.active_projects == 0 && app.dashboard.idle_projects == 0 {
                "No projects indexed yet — run sprawl profile-machine to get started"
            } else {
                "Dashboard content goes here..."
            };
            let p = Paragraph::new(text).block(block);
            f.render_widget(p, area);
        }
        Tab::SweeperInbox => {
            if app.sweeper.items.is_empty() {
                let p = Paragraph::new("Inbox zero!").block(block);
                f.render_widget(p, area);
            } else {
                let items: Vec<ratatui::widgets::ListItem> = app
                    .sweeper
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, item)| {
                        let content = Line::from(Span::raw(format!("  {}  ", item)));
                        let mut style = Style::default();
                        if i == app.sweeper.selected_index {
                            style = style.bg(app.theme.info).fg(Color::Black);
                        }
                        ratatui::widgets::ListItem::new(content).style(style)
                    })
                    .collect();

                let list = ratatui::widgets::List::new(items).block(block);
                f.render_widget(list, area);
            }
        }
        Tab::SentinelInbox => {
            views::sentinel::draw(f, app, area);
        }
        Tab::SemanticSearch => {
            views::search::draw(f, app, area);
        }
        Tab::Ask => {
            views::ask::draw(f, app, area);
        }
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let footer_text = match app.current_tab {
        Tab::Dashboard => " [q] Quit  [Tab] Next ".to_string(),
        Tab::SweeperInbox => " [X] Nuke  [A] Archive  [S] Snooze  [q] Quit ".to_string(),
        Tab::SentinelInbox => {
            if app.sentinel.batch_running {
                " [W] Batch Classify (Running...) ".to_string()
            } else {
                " [k] Known secret  [n] Noise  [W] Batch Classify ".to_string()
            }
        },
        Tab::SemanticSearch => " [Esc] Cancel search ".to_string(),
        Tab::Ask => " [Esc] Cancel ask ".to_string(),
    };

    let p = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(app.theme.info));
    f.render_widget(p, area);
}
