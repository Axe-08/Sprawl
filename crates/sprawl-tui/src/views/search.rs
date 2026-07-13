use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::{App, SearchPhase};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Semantic Search ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.info));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(block.inner(area));

    f.render_widget(block, area);

    // Query Input
    let mut query_spans = vec![Span::raw(" Query: "), Span::styled(&app.search.query, Style::default().fg(app.theme.safe))];
    if app.search.phase == SearchPhase::Input {
        query_spans.push(Span::styled("█", Style::default().add_modifier(Modifier::SLOW_BLINK)));
    }
    let query_p = Paragraph::new(Line::from(query_spans)).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(query_p, chunks[0]);

    // Results
    if app.search.is_searching {
        f.render_widget(Paragraph::new(" Searching..."), chunks[1]);
    } else if app.search.phase == SearchPhase::Results {
        let mut lines = Vec::new();
        let start_idx = app.search.selected_index.saturating_sub(5);
        for (i, result) in app.search.results.iter().enumerate().skip(start_idx) {
            let is_selected = i == app.search.selected_index;
            let style = if is_selected {
                Style::default().fg(app.theme.safe).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Reset)
            };

            lines.push(Line::from(Span::styled(format!(" {:.2}  {}:{}-{}", result.similarity_score, result.file_path, result.start_line, result.end_line), style)));
            
            for line in result.chunk_text.lines() {
                lines.push(Line::from(Span::styled(format!(" ┆  {}", line), Style::default().fg(app.theme.dormant))));
            }
        }
        f.render_widget(Paragraph::new(lines), chunks[1]);
    }
}
