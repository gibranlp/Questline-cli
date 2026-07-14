// ─────────────────────────────────────────────────────────────────────────────
// screens/restore.rs — portal de continuidad: pega tu código o llora en silencio
// ─────────────────────────────────────────────────────────────────────────────

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

/// Dibuja la pantalla de restauración de identidad para aventureros que regresan del exilio.
/// El input acepta el Transfer Code generado en el dispositivo original vía Sync → Export Profile.
pub fn draw(f: &mut Frame, input: &str, error: Option<&str>, ticks: usize, theme: &Theme) {
    let size = f.size();
    let accent = Color::Rgb(6, 182, 212);
    let gold   = Color::Rgb(245, 158, 11);
    let danger = Color::Rgb(210, 65, 65);
    let ghost  = Color::Rgb(110, 110, 110);
    let dim    = Color::Rgb(55, 55, 55);

    let show_cursor = (ticks / 10) % 2 == 0;

    f.render_widget(
        Block::default().style(Style::default().bg(theme.background)),
        size,
    );

    // borde exterior doble en cyan — distingue visualmente este portal del resto de pantallas
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent));
    let inner = outer.inner(size);
    f.render_widget(outer, size);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Percentage(12),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(inner);

    // ── Título ────────────────────────────────────────────────────────────────
    let title_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(10), Constraint::Percentage(80), Constraint::Percentage(10)])
        .split(layout[1])[1];

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "P O R T A L   O F   C O N T I N U I T Y",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "The Scribes of the Sync Realm demand proof of identity before restoring your chronicle.",
                Style::default().fg(ghost).add_modifier(Modifier::ITALIC),
            )),
            Line::from(Span::styled(
                "Your Transfer Code was forged on your original device via  [ Sync → Export Profile ].",
                Style::default().fg(dim),
            )),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        title_area,
    );

    // ── Separador ─────────────────────────────────────────────────────────────
    let sep_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(10), Constraint::Percentage(80), Constraint::Percentage(10)])
        .split(layout[2])[1];
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(64),
            Style::default().fg(dim),
        ))).alignment(Alignment::Center),
        sep_area,
    );

    // ── Advertencia sagrada ───────────────────────────────────────────────────
    let warn_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(8), Constraint::Percentage(84), Constraint::Percentage(8)])
        .split(layout[3])[1];
    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "⚠  This code contains your private signing key — a sacred relic of the Realm.",
                Style::default().fg(gold).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "   Do not share it with anyone. Not even suspiciously helpful strangers offering free loot.",
                Style::default().fg(Color::Rgb(100, 80, 40)),
            )),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true }),
        warn_area,
    );

    // ── Campo de entrada ──────────────────────────────────────────────────────
    let input_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(5), Constraint::Percentage(90), Constraint::Percentage(5)])
        .split(layout[5])[1];

    let cursor = if show_cursor { "▌" } else { " " };
    let border_color = if error.is_some() { danger } else { accent };

    f.render_widget(
        Paragraph::new(format!("  {}{}", input, cursor))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(Span::styled(
                        " Transfer Code — Paste it here ",
                        Style::default().fg(ghost),
                    )),
            )
            .style(Style::default().fg(Color::Rgb(80, 220, 235))),
        input_area,
    );

    // ── Error ─────────────────────────────────────────────────────────────────
    if let Some(err) = error {
        let err_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(5), Constraint::Percentage(90), Constraint::Percentage(5)])
            .split(layout[6])[1];
        f.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    err,
                    Style::default().fg(danger).add_modifier(Modifier::BOLD),
                )),
            ])
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
            err_area,
        );
    }

    // ── Footer ────────────────────────────────────────────────────────────────
    let footer_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(15), Constraint::Percentage(70), Constraint::Percentage(15)])
        .split(layout[7])[1];
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[ Enter ]", Style::default().fg(ghost)),
            Span::styled("  Invoke Restoration    ·    ", Style::default().fg(dim)),
            Span::styled("[ Esc ]", Style::default().fg(ghost)),
            Span::styled("  Return to the Gates", Style::default().fg(dim)),
        ])).alignment(Alignment::Center),
        footer_area,
    );
}
