use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use sprawl_sentinel::classify::SecretClassification;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Sentinel Inbox ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.info));

    // Stats
    let unreviewed = app.sentinel.items.iter().filter(|i| i.review.is_none()).count();
    let confirmed = app.sentinel.items.iter().filter(|i| matches!(i.review, Some(SecretClassification::KnownProvider(_)))).count();
    let discarded = app.sentinel.items.iter().filter(|i| matches!(i.review, Some(SecretClassification::FilteredNoise(_)))).count();

    let stats = Line::from(vec![
        Span::styled(format!(" [●] {} unreviewed  ", unreviewed), Style::default().fg(app.theme.needs_review)),
        Span::styled(format!("[✔] {} confirmed  ", confirmed), Style::default().fg(app.theme.destructive)),
        Span::styled(format!("[✕] {} discarded ", discarded), Style::default().fg(app.theme.dormant)),
    ]);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(block.inner(area));

    f.render_widget(block, area);
    f.render_widget(Paragraph::new(stats), chunks[0]);

    // Items
    let mut lines = Vec::new();
    let start_idx = app.sentinel.selected_index.saturating_sub(10); // Simple scrolling
    for (i, item) in app.sentinel.items.iter().enumerate().skip(start_idx) {
        let is_selected = i == app.sentinel.selected_index;
        let prefix = if is_selected {
            if item.expanded { "▼" } else { "▶" }
        } else {
            " "
        };

        let style = match item.review {
            None => Style::default().fg(app.theme.needs_review),
            Some(SecretClassification::KnownProvider(_)) => Style::default().fg(app.theme.destructive).add_modifier(Modifier::CROSSED_OUT),
            Some(SecretClassification::FilteredNoise(_)) => Style::default().fg(app.theme.dormant),
            Some(SecretClassification::Ambiguous) => Style::default().fg(app.theme.needs_review),
        };

        let status_str = match item.review {
            None => "[unreviewed]",
            Some(SecretClassification::KnownProvider(_)) => "[secret]",
            Some(SecretClassification::FilteredNoise(_)) => "[noise]",
            Some(SecretClassification::Ambiguous) => "[ambiguous]",
        };

        let line = Line::from(vec![
            Span::raw(format!(" {} ", prefix)),
            Span::styled(format!("{:<20} {:<15} (ambiguous) {:>20}", item.secret.filepath, item.secret.raw_value, status_str), style),
        ]);
        lines.push(line);

        if item.expanded && is_selected {
            lines.push(Line::from(format!("   Full path: {}", item.secret.filepath)));
            lines.push(Line::from(format!("   Raw value: {}", item.secret.raw_value)));
        }
    }

    f.render_widget(Paragraph::new(lines), chunks[1]);
}
