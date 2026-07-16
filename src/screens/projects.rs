// ─────────────────────────────────────────────────────────────────────────────
// screens/projects.rs — la lista de reinos (proyectos) del usuario
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::ModalType;
use crate::models::Project;
use crate::screens::intro::centered_rect;
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

// pinta la pantalla de proyectos con lista + detalles + modales encima — recibe el area del layout padre
pub fn draw(
    f: &mut Frame,
    projects: &[Project],
    selected_idx: usize,
    modal: &ModalType,
    theme: &Theme,
    area: Rect,
) {
    let size = area;
    let accent_color = theme.primary;

    // filtra archivados aquí mismo — los proyectos archivados tienen su propia pantalla
    let active_projects: Vec<&Project> = projects.iter().filter(|p| !p.archived).collect();

    // Screen Layout splits: Main List/Details, Bottom keyboard shortcut guide
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // List and details
            Constraint::Length(3), // Footer help
        ])
        .split(size);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Left: list
            Constraint::Percentage(60), // Right: details
        ])
        .split(chunks[0]);

    // si no hay proyectos activos muestra un mensaje con la instrucción — no lo dejes en blanco cuate
    let list_items: Vec<ListItem> = if active_projects.is_empty() {
        vec![ListItem::new(
            "  No active campaigns. Press [n] to create one.",
        )]
    } else {
        active_projects
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let style = if i == selected_idx {
                    Style::default()
                        .fg(Color::Black)
                        .bg(theme.selection)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(format!("  {} ", p.name)).style(style)
            })
            .collect()
    };

    let list_border_style = Style::default().fg(theme.border);
    let list_widget = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(list_border_style)
            .title(" Active Campaigns"),
    );
    f.render_widget(list_widget, body_chunks[0]);

    // panel derecho: si hay proyecto seleccionado muestra nombre, fecha y descripción; si no, placeholder
    // también formatea la fecha con timezone local — chrono haciendo su magia
    let details_border_style = Style::default().fg(theme.border);
    let details_p = if active_projects.is_empty() || selected_idx >= active_projects.len() {
        Paragraph::new("\n  Select a campaign from the list to view chronicles.").block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(details_border_style)
                .title(" Chronicles Details "),
        )
    } else {
        let p = active_projects[selected_idx];
        let desc = p.description.as_deref().unwrap_or("No description.");
        let date_str = p.created_at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string();

        let mut text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Campaign:  ", Style::default().fg(theme.muted)),
                Span::styled(
                    &p.name,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Created At:  ", Style::default().fg(theme.muted)),
                Span::styled(date_str, Style::default().fg(theme.text)),
            ]),
            Line::from(""),
            Line::from("  Description:"),
        ];

        for line in desc.lines() {
            text.push(Line::from(Span::styled(
                format!("  {}", line),
                Style::default().fg(theme.text),
            )));
        }

        text.extend(vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  Press [Enter] to open the War Room.",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            )),
        ]);

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(accent_color))
                    .title(" Chronicle Details "),
            )
            .wrap(ratatui::widgets::Wrap { trim: true })
    };
    f.render_widget(details_p, body_chunks[1]);

    // 3. Footer Help bar
    let footer_text = vec![Line::from(vec![
        Span::styled(" Campaigns | ", Style::default().fg(accent_color)),
        Span::styled(
            "↑↓",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Navigate | ", Style::default().fg(theme.muted)),
        Span::styled(
            "n",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" New | ", Style::default().fg(theme.muted)),
        Span::styled(
            "e",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Edit | ", Style::default().fg(theme.muted)),
        Span::styled(
            "d",
            Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Archive | | ", Style::default().fg(theme.muted)),
        Span::styled(
            "A",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Archives | ", Style::default().fg(theme.muted)),
        Span::styled(
            "F",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Focus | ", Style::default().fg(theme.muted)),
        Span::styled(
            "S",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Share | ", Style::default().fg(theme.muted)),
        Span::styled("ESC", Style::default().fg(theme.text)),
        Span::styled(" Back ", Style::default().fg(theme.muted)),
    ])];
    let footer = Paragraph::new(footer_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(footer, chunks[1]);

    // los modales se renderizan encima de todo usando Clear — pues hay que limpiar antes de pintar
    match modal {
        ModalType::NewProject {
            name,
            name_cursor,
            desc,
            desc_cursor,
            focus_idx,
        } => {
            draw_project_modal(f, " New Campaign ", name, *name_cursor, desc, *desc_cursor, *focus_idx, theme);
        }
        ModalType::EditProject {
            name,
            name_cursor,
            desc,
            desc_cursor,
            focus_idx,
            ..
        } => {
            draw_project_modal(f, " Edit Campaign ", name, *name_cursor, desc, *desc_cursor, *focus_idx, theme);
        }
        _ => {}
    }
}

// el popup de new/edit: centrado al 60x40%, nombre arriba, descripción abajo y help al fondo
fn draw_project_modal(
    f: &mut Frame,
    title: &str,
    name: &str,
    name_cursor: usize,
    desc: &str,
    desc_cursor: usize,
    focus_idx: usize,
    theme: &Theme,
) {
    let area = centered_rect(60, 40, f.size());
    f.render_widget(Clear, area);
    f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);

    let accent_color = theme.primary;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Name input
            Constraint::Min(5),    // Description input
            Constraint::Length(2), // Help line
        ])
        .split(area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(accent_color))
        .title(Span::styled(
            title,
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(main_block, area);

    // borde activo según focus_idx — 0 = nombre, 1 = descripción
    let name_border_style = if focus_idx == 0 {
        Style::default()
            .fg(accent_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };

    let mut name_spans = Vec::new();
    if focus_idx == 0 {
        let s = name;
        let c_pos = name_cursor.min(s.len());
        let before = &s[..c_pos];
        let after = &s[c_pos..];
        name_spans.push(Span::styled(before, Style::default()));
        if let Some(first_char) = after.chars().next() {
            let char_len = first_char.len_utf8();
            name_spans.push(Span::styled(
                &after[..char_len],
                Style::default().add_modifier(Modifier::REVERSED),
            ));
            name_spans.push(Span::styled(&after[char_len..], Style::default()));
        } else {
            name_spans.push(Span::styled(" ", Style::default().add_modifier(Modifier::REVERSED)));
        }
    } else {
        name_spans.push(Span::styled(name, Style::default()));
    }

    let name_p = Paragraph::new(Line::from(name_spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(name_border_style)
            .title(" Quest Name "),
    );
    f.render_widget(name_p, chunks[0]);

    // Description field rendering
    let desc_border_style = if focus_idx == 1 {
        Style::default()
            .fg(accent_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };

    let desc_lines = desc_lines_with_cursor(desc, desc_cursor, focus_idx == 1);

    // no .wrap() here on purpose: ratatui's WordWrapper collapses a whitespace-only line to ""
    // regardless of the trim flag, and a freshly-started empty line (right after Enter) is
    // exactly that — just the cursor's single reversed space. The name field above has the
    // same no-wrap treatment and doesn't hit this.
    let desc_p = Paragraph::new(desc_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(desc_border_style)
            .title(" Description "),
    );
    f.render_widget(desc_p, chunks[1]);

    // Dialog shortcuts guide
    let helper = Paragraph::new("Tab: switch fields | Arrows/Home/End: move cursor | Enter: new line / save | ESC: save & close")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(helper, chunks[2]);
}

// arma el texto de la descripción como varias Line — respeta los \n reales en vez de meterlos
// en una sola Line (que ratatui no interpreta como salto de línea) y, si está enfocado, dibuja
// el cursor (carácter reversed) en la línea que le corresponde
fn desc_lines_with_cursor(desc: &str, cursor: usize, focused: bool) -> Vec<Line<'static>> {
    if !focused {
        return desc.split('\n').map(|l| Line::from(l.to_string())).collect();
    }

    let c_pos = cursor.min(desc.len());
    let before = &desc[..c_pos];
    let after = &desc[c_pos..];

    let mut lines: Vec<Line> = Vec::new();
    let mut before_parts: Vec<&str> = before.split('\n').collect();
    let last_before = before_parts.pop().unwrap_or("");
    for part in before_parts {
        lines.push(Line::from(part.to_string()));
    }

    let mut cur_line_spans: Vec<Span> = vec![Span::styled(last_before.to_string(), Style::default())];

    if let Some(first_char) = after.chars().next() {
        if first_char == '\n' {
            cur_line_spans.push(Span::styled(" ".to_string(), Style::default().add_modifier(Modifier::REVERSED)));
            lines.push(Line::from(cur_line_spans));
            let rest = &after[1..];
            for part in rest.split('\n') {
                lines.push(Line::from(part.to_string()));
            }
        } else {
            let char_len = first_char.len_utf8();
            cur_line_spans.push(Span::styled(
                first_char.to_string(),
                Style::default().add_modifier(Modifier::REVERSED),
            ));
            let rest = &after[char_len..];
            let mut rest_parts = rest.split('\n');
            if let Some(first_rest) = rest_parts.next() {
                cur_line_spans.push(Span::styled(first_rest.to_string(), Style::default()));
            }
            lines.push(Line::from(cur_line_spans));
            for part in rest_parts {
                lines.push(Line::from(part.to_string()));
            }
        }
    } else {
        cur_line_spans.push(Span::styled(" ".to_string(), Style::default().add_modifier(Modifier::REVERSED)));
        lines.push(Line::from(cur_line_spans));
    }

    lines
}
