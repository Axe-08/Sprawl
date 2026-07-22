use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AskPhase};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" RAG (Ask) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.info));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(block.inner(area));

    f.render_widget(block, area);

    // Query Input
    let mut query_spans = vec![Span::raw(" Query: "), Span::styled(&app.ask.query, Style::default().fg(app.theme.safe))];
    if app.ask.phase == AskPhase::Input {
        query_spans.push(Span::styled("█", Style::default().add_modifier(Modifier::SLOW_BLINK)));
    }
    let query_p = Paragraph::new(Line::from(query_spans)).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(query_p, chunks[0]);

    // Results
    if app.ask.phase == AskPhase::Waiting {
        f.render_widget(Paragraph::new(" Loading context and generating answer... (This may take a few seconds)").style(Style::default().fg(app.theme.needs_review)), chunks[1]);
    } else if app.ask.phase == AskPhase::Answered {
        let answer_p = Paragraph::new(app.ask.answer.as_str())
            .wrap(Wrap { trim: false });
        f.render_widget(answer_p, chunks[1]);
    }
}
