// ─────────────────────────────────────────────────────────────────────────────
// ui/mod.rs — utilidades de interfaz compartidas
// ─────────────────────────────────────────────────────────────────────────────

use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn progress_bar(filled: usize, width: usize) -> String {
    (0..width)
        .map(|i| if i < filled { '#' } else { '-' })
        .collect()
}

pub fn draw_hydration_reminder_modal(
    f: &mut Frame,
    area: Rect,
    glasses: i32,
    target: i32,
    _theme: &Theme,
) {
    let modal_area = centered_rect(40, 35, area);
    let hydration_bg = Color::Rgb(7, 25, 48);
    let hydration_border = Color::Rgb(56, 189, 248);
    let hydration_title = Color::Rgb(224, 242, 254);
    let hydration_text = Color::Rgb(186, 230, 253);
    let hydration_muted = Color::Rgb(125, 211, 252);

    f.render_widget(Clear, modal_area);
    f.render_widget(
        Block::default().style(Style::default().bg(hydration_bg)),
        modal_area,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(hydration_border).bg(hydration_bg))
        .title(Span::styled(
            " Hydration Reminder ",
            Style::default()
                .fg(hydration_title)
                .bg(hydration_bg)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(modal_area);
    f.render_widget(block, modal_area);
    let content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(inner);
    f.render_widget(Paragraph::new(""), content[0]);
    f.render_widget(
        Paragraph::new(Span::styled(
            "  Time to drink some water!",
            Style::default()
                .fg(hydration_title)
                .bg(hydration_bg)
                .add_modifier(Modifier::BOLD),
        )),
        content[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("  Today: {}/{} glasses", glasses, target),
            Style::default().fg(hydration_text).bg(hydration_bg),
        )),
        content[2],
    );
    let bar_w = inner.width.saturating_sub(4) as usize;
    let filled = if target > 0 {
        (glasses * bar_w as i32 / target).min(bar_w as i32) as usize
    } else {
        0
    };
    f.render_widget(
        Paragraph::new(Span::styled(
            format!("  [{}]", progress_bar(filled, bar_w)),
            Style::default().fg(hydration_border).bg(hydration_bg),
        )),
        content[3],
    );
    f.render_widget(Paragraph::new(""), content[4]);
    f.render_widget(
        Paragraph::new(Span::styled(
            " [d] Drink  [s] Snooze 15m  [x] Dismiss ",
            Style::default().fg(hydration_muted).bg(hydration_bg),
        ))
        .alignment(Alignment::Center),
        content[5],
    );
}
