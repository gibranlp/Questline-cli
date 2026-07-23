// ─────────────────────────────────────────────────────────────────────────────
// screens/archive.rs — vista de proyectos archivados
// ─────────────────────────────────────────────────────────────────────────────
use crate::models::Project;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

// Renders the Archive Screen listing archived items from SQLite.
pub fn draw(f: &mut Frame, projects: &[Project], selected_idx: usize, theme: &Theme) {
    let size = f.size();
    let accent_color = theme.primary;

    // archived=true OR completed=true — cacha proyectos huérfanos (e.g. conquered en versión anterior y restaurados a medias)
    let archived_projects: Vec<&Project> = projects.iter().filter(|p| p.archived || p.completed).collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // List and details
            Constraint::Length(3), // Footer help
        ])
        .split(size);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[0]);

    // 1. Archived CampaignsList
    let list_items: Vec<ListItem> = if archived_projects.is_empty() {
        vec![ListItem::new("  No archived campaigns found in database.")]
    } else {
        archived_projects
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let style = if i == selected_idx {
                    Style::default()
                        .fg(Color::Black)
                        .bg(theme.selection)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                ListItem::new(format!("  {} ", p.name)).style(style)
            })
            .collect()
    };

    let list_widget = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Archived Campaigns"),
    );
    f.render_widget(list_widget, body_chunks[0]);

    // 2. Archived Details Panel
    let details_p = if archived_projects.is_empty() || selected_idx >= archived_projects.len() {
        Paragraph::new("\n  No archived campaign selected.").block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Archive Chronicle Details "),
        )
    } else {
        let p = archived_projects[selected_idx];
        let desc = p
            .description
            .as_deref()
            .unwrap_or("No description provided.");
        let date_str = p.created_at.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M:%S").to_string();

        let text = vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  Campaign:   ", Style::default().fg(theme.muted)),
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
                "  Press [r] to Restore or [Delete] to Slay permanently.",
                Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
            )),
        ];

        Paragraph::new(text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.danger))
                    .title(" Archive Chronicle Details "),
            )
            .wrap(ratatui::widgets::Wrap { trim: true })
    };
    f.render_widget(details_p, body_chunks[1]);

    // 3. Footer Help bar
    let footer_text = vec![Line::from(vec![
        Span::styled(" Archive |  ", Style::default().fg(accent_color)),
        Span::styled(
            "↑↓",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Navigate | ", Style::default().fg(theme.muted)),
        Span::styled(
            "r",
            Style::default()
                .fg(theme.success)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Restore Campaign | ", Style::default().fg(theme.muted)),
        Span::styled(
            "Delete",
            Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Delete Permanently | ",
            Style::default().fg(theme.muted),
        ),
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
}
