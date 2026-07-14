// ─────────────────────────────────────────────────────────────────────────────
// screens/gateway.rs — la puerta del Realm: ¿héroe nuevo o exiliado que regresa?
// ─────────────────────────────────────────────────────────────────────────────

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

const PALETTE: &[Color] = &[
    Color::Rgb(168,  85, 247),
    Color::Rgb(249, 115,  22),
    Color::Rgb(6,  182, 212),
    Color::Rgb(245, 158,  11),
    Color::Rgb(255, 105, 180),
];

/// Dibuja la pantalla de selección inicial — nuevo aventurero o retorno del exilio.
/// Único punto de entrada al Realm para quienes no tienen usuario registrado.
pub fn draw(f: &mut Frame, selected_idx: usize, ticks: usize, theme: &Theme) {
    let size = f.size();
    let accent = PALETTE[(ticks / 8) % PALETTE.len()];
    let muted  = Color::Rgb(70, 70, 70);
    let dim    = Color::Rgb(50, 50, 50);
    let body   = Color::Rgb(195, 195, 195);
    let ghost  = Color::Rgb(110, 110, 110);

    f.render_widget(
        Block::default().style(Style::default().bg(theme.background)),
        size,
    );

    // borde exterior doble — enmarca la decisión más importante del aventurero
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent));
    let inner = outer.inner(size);
    f.render_widget(outer, size);

    // layout vertical: top padding + header + sep + opción1 + gap + opción2 + disclaimer + footer
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(8),
            Constraint::Length(1),
            Constraint::Length(8),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(inner);

    let center = |area: Rect| {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(15),
                Constraint::Percentage(70),
                Constraint::Percentage(15),
            ])
            .split(area)[1]
    };

    // ── Header ────────────────────────────────────────────────────────────────
    let header = vec![
        Line::from(Span::styled(
            "T H E   G A T E S   O F   T H E   R E A L M",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "The Chronicle demands your classification before granting passage.",
            Style::default().fg(ghost).add_modifier(Modifier::ITALIC),
        )),
        Line::from(Span::styled(
            "Those who remain undecided are automatically enrolled in Advanced Tax Optimization.",
            Style::default().fg(dim).add_modifier(Modifier::ITALIC),
        )),
    ];
    f.render_widget(
        Paragraph::new(header).alignment(Alignment::Center).wrap(Wrap { trim: true }),
        center(layout[1]),
    );

    // ── Separador ─────────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(64),
            Style::default().fg(dim),
        ))).alignment(Alignment::Center),
        center(layout[2]),
    );

    // ── Opción 0 — Nuevo Aventurero ───────────────────────────────────────────
    let sel0 = selected_idx == 0;
    let (border0, btype0, title0) = if sel0 {
        (Style::default().fg(accent).add_modifier(Modifier::BOLD), BorderType::Double, " ▸  NEW ADVENTURER  ◂ ")
    } else {
        (Style::default().fg(muted), BorderType::Plain, "   New Adventurer   ")
    };

    let opt0 = vec![
        Line::from(""),
        Line::from(Span::styled(
            "I have arrived unburdened by history, glory, or any completed tasks whatsoever.",
            Style::default().fg(if sel0 { body } else { muted }),
        )),
        Line::from(Span::styled(
            "The Orders shall evaluate my calling. I accept that the bar is refreshingly low.",
            Style::default().fg(if sel0 { ghost } else { dim }),
        )),
        Line::from(""),
    ];
    f.render_widget(
        Paragraph::new(opt0)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(btype0)
                    .border_style(border0)
                    .title(Span::styled(title0, Style::default().fg(if sel0 { accent } else { muted }).add_modifier(Modifier::BOLD))),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        center(layout[3]),
    );

    // ── Opción 1 — Continuar la Aventura ──────────────────────────────────────
    let sel1 = selected_idx == 1;
    let (border1, btype1, title1) = if sel1 {
        (Style::default().fg(accent).add_modifier(Modifier::BOLD), BorderType::Double, " ▸  CONTINUE THE ADVENTURE  ◂ ")
    } else {
        (Style::default().fg(muted), BorderType::Plain, "   Continue the Adventure   ")
    };

    let opt1 = vec![
        Line::from(""),
        Line::from(Span::styled(
            "I am a refugee from another device. I have suffered elsewhere and wish to continue.",
            Style::default().fg(if sel1 { body } else { muted }),
        )),
        Line::from(Span::styled(
            "I carry a Transfer Code. My chronicle awaits restoration. Please. I beg of thee.",
            Style::default().fg(if sel1 { ghost } else { dim }),
        )),
        Line::from(""),
    ];
    f.render_widget(
        Paragraph::new(opt1)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(btype1)
                    .border_style(border1)
                    .title(Span::styled(title1, Style::default().fg(if sel1 { accent } else { muted }).add_modifier(Modifier::BOLD))),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        center(layout[5]),
    );

    // ── Disclaimer ────────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "The Chronicle bears no responsibility for decisions made under pressure, confusion, or hubris.",
            Style::default().fg(dim).add_modifier(Modifier::ITALIC),
        ))).alignment(Alignment::Center).wrap(Wrap { trim: true }),
        center(layout[6]),
    );

    // ── Footer ────────────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[ ↑ / ↓ ]", Style::default().fg(ghost)),
            Span::styled("  navigate    ", Style::default().fg(dim)),
            Span::styled("[ Enter ]", Style::default().fg(ghost)),
            Span::styled("  confirm your fate", Style::default().fg(dim)),
        ])).alignment(Alignment::Center),
        center(layout[7]),
    );
}
