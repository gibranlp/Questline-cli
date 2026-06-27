// ─────────────────────────────────────────────────────────────────────────────
// screens/great_chronicle.rs — el feed global de actividad del reino, chido para ver qué onda
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::App;
use crate::models::GlobalChronicleEntry;
use crate::models::chapter::{Chapter, ChapterProgressData, get_active_chapter};
use crate::theme::Theme;
use chrono::{DateTime, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

// convierte un timestamp ISO a texto legible tipo "3 minutes ago" — se usa en el feed y el historial
fn relative_time(ts: &str) -> String {
    let Ok(dt) = ts.parse::<DateTime<Utc>>() else {
        // si el timestamp no parsea pues lo dejamos como viene, qué rollo
        return ts.to_string();
    };
    let secs = (Utc::now() - dt).num_seconds().max(0);
    if secs < 60 {
        return "just now".to_string();
    }
    if secs < 3600 {
        let m = secs / 60;
        return format!("{} minute{} ago", m, if m == 1 { "" } else { "s" });
    }
    if secs < 86400 {
        let h = secs / 3600;
        return format!("{} hour{} ago", h, if h == 1 { "" } else { "s" });
    }
    let days = secs / 86400;
    if days == 1 {
        return "Yesterday".to_string();
    }
    if days < 7 {
        return format!("{} days ago", days);
    }
    if days < 30 {
        let w = days / 7;
        return format!("{} week{} ago", w, if w == 1 { "" } else { "s" });
    }
    let months = days / 30;
    if months < 12 {
        return format!("{} month{} ago", months, if months == 1 { "" } else { "s" });
    }
    let years = months / 12;
    format!("{} year{} ago", years, if years == 1 { "" } else { "s" })
}

// arma las líneas del feed — las entradas anónimas no muestran nombre de héroe, las demás sí
fn build_feed_lines<'a>(entries: &'a [GlobalChronicleEntry], theme: &'a Theme) -> Vec<Line<'a>> {
    if entries.is_empty() {
        return vec![
            Line::from(""),
            Line::from(Span::styled(
                "  The Great Chronicle is silent.",
                Style::default().fg(theme.muted),
            )),
            Line::from(Span::styled(
                "  The realm stirs. Activity will appear as heroes act.",
                Style::default().fg(theme.muted),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press [R] to refresh.",
                Style::default().fg(theme.muted),
            )),
        ];
    }

    let mut lines = Vec::new();
    for entry in entries {
        let icon = entry.icon();
        let age = relative_time(&entry.timestamp);

        // anónimo = solo descripción con color secundario; identificado = icon primario + nombre del héroe
        if entry.is_anonymous() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(icon, Style::default().fg(theme.secondary)),
                Span::raw("  "),
                Span::styled(entry.description.clone(), Style::default().fg(theme.text)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(icon, Style::default().fg(theme.primary)),
                Span::raw("  "),
                Span::styled(
                    entry.hero_name.clone(),
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(entry.description.clone(), Style::default().fg(theme.text)),
            ]));
        }
        lines.push(Line::from(vec![
            Span::raw("      "),
            Span::styled(age, Style::default().fg(theme.muted)),
        ]));
        lines.push(Line::from(""));
    }
    lines
}

// el texto de "Call to Arms" viene con saltos de línea — los parseamos y pintamos uno por uno
fn build_call_to_arms_lines<'a>(chapter: &'a Chapter, theme: &'a Theme) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Call to Arms",
        Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from("  "));
    for para in chapter.call_to_arms.split('\n') {
        if para.is_empty() {
            continue;
        }
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(para.to_string(), Style::default().fg(theme.text)),
        ]));
    }
    lines
}

// tarjeta de progreso general del capítulo — suma todos los objetivos y saca el % global
fn build_completion_card_lines<'a>(
    chapter: &'a Chapter,
    progress: Option<&'a ChapterProgressData>,
    theme: &'a Theme,
    width: u16,
) -> Vec<Line<'a>> {
    let mut total_current: u64 = 0;
    let mut total_target: u64 = 0;
    // recorremos todos los objetivos y acumulamos current/target — si no hay progress asumimos 0
    for obj in chapter.objectives {
        let cur = if let Some(prog) = progress {
            prog.objectives
                .iter()
                .find(|o| o.id == obj.id)
                .map(|o| o.current.min(obj.target))
                .unwrap_or(0)
        } else {
            0
        };
        total_current += cur;
        total_target += obj.target;
    }

    let ratio = if total_target > 0 {
        (total_current as f64 / total_target as f64).min(1.0)
    } else {
        0.0
    };
    let pct = (ratio * 100.0) as u32;

    // la barra ASCII con '=' y '-' — restamos 6 para los bordes y dejamos mínimo 8 chars
    let bar_width = (width as usize).saturating_sub(6).max(8);
    let filled = ((bar_width as f64) * ratio) as usize;
    let empty = bar_width.saturating_sub(filled);
    let bar = format!("[{}{}]", "=".repeat(filled), "-".repeat(empty));

    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Chapter Completion",
        Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(bar, Style::default().fg(theme.primary)),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{}%", pct),
            Style::default().fg(theme.primary).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            format!("{} / {}", total_current, total_target),
            Style::default().fg(theme.muted),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{} objectives", chapter.objectives.len()),
            Style::default().fg(theme.muted),
        ),
    ]));

    if progress.is_none() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Sync to load.",
            Style::default().fg(theme.muted),
        )));
    }

    lines
}

// una barra de progreso por cada objetivo del capítulo — scrolleable desde el panel derecho
fn build_objectives_lines<'a>(
    chapter: &'a Chapter,
    progress: Option<&'a ChapterProgressData>,
    theme: &'a Theme,
    panel_width: u16,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    lines.push(Line::from(""));

    let bar_width = (panel_width as usize).saturating_sub(6).max(10);

    for obj_def in chapter.objectives {
        // buscamos el objetivo en el progreso del usuario por ID — si no está, arrancamos en 0
        let (current, target) = if let Some(prog) = progress {
            prog.objectives
                .iter()
                .find(|o| o.id == obj_def.id)
                .map(|o| (o.current, o.target))
                .unwrap_or((0, obj_def.target))
        } else {
            (0, obj_def.target)
        };

        let ratio = if target > 0 {
            (current as f64 / target as f64).min(1.0)
        } else {
            0.0
        };
        let filled = ((bar_width as f64) * ratio) as usize;
        let empty = bar_width.saturating_sub(filled);
        let bar = format!("[{}{}]", "=".repeat(filled), "-".repeat(empty));

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                obj_def.name.to_string(),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(bar, Style::default().fg(theme.primary)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!("{} / {}", current, target),
                Style::default().fg(theme.muted),
            ),
        ]));
        lines.push(Line::from(""));
    }

    if let Some(prog) = progress {
        if prog.completed {
            lines.push(Line::from(Span::styled(
                "  Chapter Complete",
                Style::default().fg(theme.success).add_modifier(Modifier::BOLD),
            )));
            if let Some(ref ts) = prog.completed_at {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("Completed: {}", relative_time(ts)),
                        Style::default().fg(theme.muted),
                    ),
                ]));
            }
            lines.push(Line::from(""));
        }
    }

    if progress.is_none() {
        lines.push(Line::from(Span::styled(
            "  Sync to load chapter progress.",
            Style::default().fg(theme.muted),
        )));
    }

    lines
}

// historial de capítulos completados — muestra título, cuándo se completó y la contribución personal
fn build_history_lines<'a>(
    app: &'a App,
    theme: &'a Theme,
) -> Vec<Line<'a>> {
    if app.chapter_history.is_empty() {
        return vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No chapters completed yet.",
                Style::default().fg(theme.muted),
            )),
            Line::from(Span::styled(
                "  Complete Chapter One to begin the record.",
                Style::default().fg(theme.muted),
            )),
        ];
    }

    let mut lines = Vec::new();
    for entry in &app.chapter_history {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                entry.title.clone(),
                Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Status: ", Style::default().fg(theme.muted)),
            Span::styled("Completed", Style::default().fg(theme.success)),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Completed: ", Style::default().fg(theme.muted)),
            Span::styled(
                relative_time(&entry.completed_at),
                Style::default().fg(theme.text),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("Contribution: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} Actions", entry.personal_contribution),
                Style::default().fg(theme.text),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            "  ────────────────────────────────",
            Style::default().fg(theme.border),
        )));
    }
    lines
}

// función principal de la pantalla — header + body dividido, footer con controles
// el body se parte en feed (izquierda) y panel de capítulo (derecha, con su propio sistema de tabs)
pub fn draw(f: &mut ratatui::Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Body
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // el hint de sharing cambia de color según si el usuario está compartiendo o no
    let (share_hint, share_color) = match app.config.chronicle_share_level.as_str() {
        "none" => (" sharing: off ", theme.muted),
        _ => (" sharing: all ", Color::Rgb(249, 115, 22)),
    };
    let chapter_indicator = if let Some(ch) = get_active_chapter() {
        format!("  {}  ", ch.title)
    } else {
        "  No Active Chapter  ".to_string()
    };
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "  The Great Chronicle  ",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("|", Style::default().fg(theme.border)),
        Span::styled(
            format!("  {} entries", app.great_chronicle_entries.len()),
            Style::default().fg(theme.muted),
        ),
        Span::styled("  |", Style::default().fg(theme.border)),
        Span::styled(share_hint, Style::default().fg(share_color).add_modifier(Modifier::BOLD)),
        Span::styled("  |  ", Style::default().fg(theme.border)),
        Span::styled(chapter_indicator, Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD)),
        Span::styled("  |  ", Style::default().fg(theme.border)),
        Span::styled("[X]", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled(" story so far  ", Style::default().fg(theme.muted)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(header, chunks[0]);

    // Body: horizontal split — left feed / right chapter panel
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(55),
        ])
        .split(chunks[1]);

    // el borde activo se ilumina con primary — así el usuario sabe dónde está el focus
    let left_border_style = if app.chapter_panel_focused {
        Style::default().fg(theme.border)
    } else {
        Style::default().fg(theme.primary)
    };
    let right_border_style = if app.chapter_panel_focused {
        Style::default().fg(theme.primary)
    } else {
        Style::default().fg(theme.border)
    };

    // Left panel — feed de actividad global, scrolleable con up/down cuando el focus está aquí
    let feed_lines = build_feed_lines(&app.great_chronicle_entries, theme);
    let feed = Paragraph::new(feed_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(left_border_style)
                .title(Span::styled(
                    " Realm Activity ",
                    Style::default().fg(theme.primary),
                )),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.great_chronicle_scroll as u16, 0));
    f.render_widget(feed, body_chunks[0]);

    // panel derecho — tab 0 = capítulo activo (CTA + completion + objetivos), tab 1 = historial
    if app.chapter_tab == 1 {
        let history_lines = build_history_lines(app, theme);
        let history_panel = Paragraph::new(history_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(right_border_style)
                    .title(Span::styled(
                        " Chapter History ",
                        Style::default().fg(theme.secondary),
                    )),
            )
            .wrap(Wrap { trim: false })
            .scroll((app.chapter_panel_scroll as u16, 0));
        f.render_widget(history_panel, body_chunks[1]);
    } else if let Some(ch) = get_active_chapter() {
        // capítulo activo — la parte de arriba se divide en dos columnas: CTA a la izq, completion a la der
        let chapter_title_str = format!(" {} ", ch.title);

        let right_vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(42),
                Constraint::Percentage(58),
            ])
            .split(body_chunks[1]);

        let top_cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ])
            .split(right_vert[0]);

        // Call to Arms (top-left)
        let cta_lines = build_call_to_arms_lines(ch, theme);
        let cta_widget = Paragraph::new(cta_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(right_border_style)
                    .title(Span::styled(
                        chapter_title_str,
                        Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(cta_widget, top_cols[0]);

        // Completion Card (top-right)
        let completion_lines = build_completion_card_lines(
            ch,
            app.chapter_progress.as_ref(),
            theme,
            top_cols[1].width,
        );
        let completion_widget = Paragraph::new(completion_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(right_border_style)
                    .title(Span::styled(
                        " Progress ",
                        Style::default().fg(theme.primary),
                    )),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(completion_widget, top_cols[1]);

        // Objectives (bottom, scrollable)
        let obj_lines = build_objectives_lines(
            ch,
            app.chapter_progress.as_ref(),
            theme,
            right_vert[1].width,
        );
        let obj_widget = Paragraph::new(obj_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(right_border_style)
                    .title(Span::styled(
                        " Objectives ",
                        Style::default().fg(theme.primary),
                    )),
            )
            .wrap(Wrap { trim: false })
            .scroll((app.chapter_panel_scroll as u16, 0));
        f.render_widget(obj_widget, right_vert[1]);
    } else {
        // No active chapter
        let no_chapter_widget = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No chapter is currently active.",
                Style::default().fg(theme.muted),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(right_border_style)
                .title(Span::styled(
                    " No Active Chapter ",
                    Style::default().fg(theme.secondary).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
        f.render_widget(no_chapter_widget, body_chunks[1]);
    }

    // footer dinámico — los hints cambian según si el focus está en el feed o en el panel del capítulo
    let tab_hint = if app.chapter_panel_focused {
        if app.chapter_tab == 0 { "Active Chapter" } else { "Chapter History" }
    } else {
        ""
    };

    let mut footer_spans = vec![
        Span::styled("[↑↓]", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled(" scroll  ", Style::default().fg(theme.muted)),
    ];

    if app.chapter_panel_focused {
        footer_spans.push(Span::styled("[←]", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)));
        footer_spans.push(Span::styled(" feed  ", Style::default().fg(theme.muted)));
        footer_spans.push(Span::styled("[Tab]", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)));
        footer_spans.push(Span::styled(format!(" {} | history  ", tab_hint), Style::default().fg(theme.muted)));
    } else {
        footer_spans.push(Span::styled("[→]", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)));
        footer_spans.push(Span::styled(" chapter  ", Style::default().fg(theme.muted)));
    }

    footer_spans.extend_from_slice(&[
        Span::styled("[R]", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled(" refresh  ", Style::default().fg(theme.muted)),
        Span::styled("[P]", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled(" privacy  ", Style::default().fg(theme.muted)),
        Span::styled("[Esc]", Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        Span::styled(" back", Style::default().fg(theme.muted)),
    ]);

    let footer = Paragraph::new(Line::from(footer_spans))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border)),
        );
    f.render_widget(footer, chunks[2]);
}
