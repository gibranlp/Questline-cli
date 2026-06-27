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
            "  No active projects. Press [n] to create one.",
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
            .title(" Active Realms "),
    );
    f.render_widget(list_widget, body_chunks[0]);

    // panel derecho: si hay proyecto seleccionado muestra nombre, fecha y descripción; si no, placeholder
    // también formatea la fecha con timezone local — chrono haciendo su magia
    let details_border_style = Style::default().fg(theme.border);
    let details_p = if active_projects.is_empty() || selected_idx >= active_projects.len() {
        Paragraph::new("\n  Select a realm from the list to view chronicles.").block(
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

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Realm:     ", Style::default().fg(theme.muted)),
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
            Line::from(Span::styled(
                format!("  {}", desc),
                Style::default().fg(theme.text),
            )),
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled(
                "  Press [Enter] to open the War Room.",
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            )),
        ];

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
        Span::styled(" Projects |  ", Style::default().fg(accent_color)),
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
            desc,
            focus_idx,
        } => {
            draw_project_modal(f, " New Realm Quest ", name, desc, *focus_idx, theme);
        }
        ModalType::EditProject {
            name,
            desc,
            focus_idx,
            ..
        } => {
            draw_project_modal(f, " Edit Realm Quest ", name, desc, *focus_idx, theme);
        }
        _ => {}
    }
}

// el popup de new/edit: centrado al 60x40%, nombre arriba, descripción abajo y help al fondo
fn draw_project_modal(
    f: &mut Frame,
    title: &str,
    name: &str,
    desc: &str,
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

    // borde activo según focus_idx — 0 = nombre, 1 = descripción; muestra "_" si está vacío y enfocado
    let name_border_style = if focus_idx == 0 {
        Style::default()
            .fg(accent_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border)
    };
    let name_text = if name.is_empty() && focus_idx == 0 {
        "_"
    } else {
        name
    };
    let name_p = Paragraph::new(name_text).block(
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
    let desc_text = if desc.is_empty() && focus_idx == 1 {
        "_"
    } else {
        desc
    };
    let desc_p = Paragraph::new(desc_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(desc_border_style)
                .title(" Description "),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(desc_p, chunks[1]);

    // Dialog shortcuts guide
    let helper = Paragraph::new("Tab: switch fields | Enter: save | ESC: cancel")
        .style(Style::default().fg(theme.muted))
        .alignment(Alignment::Center);
    f.render_widget(helper, chunks[2]);
}
