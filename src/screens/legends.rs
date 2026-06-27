// ─────────────────────────────────────────────────────────────────────────────
// screens/legends.rs — el salón de leyendas: reliquias y títulos legendarios
// ─────────────────────────────────────────────────────────────────────────────

use crate::models::Statistics;
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

// toda la pantalla: récords personales a la izq, inventario de reliquias a la der
// la tupla de relics es (id, nombre, desc, desbloqueada, fecha_unlock) — medio fea pero funciona
pub fn draw(
    f: &mut Frame,
    stats: &Statistics,
    selected_relic_idx: usize,
    // Relic status: (id, name, desc, unlocked, unlocked_at)
    relics: &[(String, String, String, bool, Option<String>)],
    theme: &Theme,
) {
    let size = f.size();
    let accent_color = theme.primary;

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Legends Room
            Constraint::Length(3), // Help Footer
        ])
        .split(size);

    // 1. Header
    let header_p = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "THE HALL OF LEGENDS",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " - A sanctuary displaying your legendary accomplishments and relics.",
            Style::default().fg(theme.text),
        ),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(header_p, main_chunks[0]);

    // Split room into left (records) and right (relics)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Personal Records & Streaks
            Constraint::Percentage(50), // Relics Inventory
        ])
        .split(main_chunks[1]);

    // el árbol zen tiene etapas según el tree_growth — de bellota hasta Árbol del Mundo, órale
    // 2a. Left Column: Personal Records & Streaks
    let tree_stage = match stats.tree_growth {
        0..=9 => "Acorn",
        10..=24 => "Entling",
        25..=49 => "Young Entling",
        50..=99 => "Grove Sapling",
        100..=199 => "Mallorn Tree",
        200..=299 => "Ancient Ent",
        _ => "World Tree",
    };

    let records_text = vec![
        Line::from(Span::styled(
            "COGNITIVE TRIUMPHS & RECORDS",
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Longest Streak:        ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} Days", stats.best_streak),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Total Focus Hours:     ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{:.1} Hours", stats.focus_hours),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Sessions Completed:    ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} Focus Sessions", stats.sessions_completed),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Tasks Completed:       ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} Tasks", stats.tasks_completed),
                Style::default().fg(theme.success),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Projects Completed:    ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} Projects", stats.projects_completed),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Milestones Reached:    ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} Milestones", stats.milestones_completed),
                Style::default().fg(Color::LightCyan),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Zen Tree Stature:      ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} ({} Growth)", tree_stage, stats.tree_growth),
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Sidequests Done:       ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} Sidequests", stats.rituals_completed),
                Style::default().fg(theme.text),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Cloud Achievements:    ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} Unlocked", stats.achievements_unlocked),
                Style::default().fg(theme.warning),
            ),
        ]),
    ];

    let records_p = Paragraph::new(records_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.border))
                .title(" Personal Records "),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(records_p, body_chunks[0]);

    // columna derecha: lista de reliquias arriba (altura fija 7) y detalle + ASCII abajo
    let relics_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Relics list
            Constraint::Min(5),    // Selected relic details & ASCII
        ])
        .split(body_chunks[1]);

    // construye los items: ">" si está seleccionado, "+" desbloqueado o "-" bloqueado
    let relic_items: Vec<ListItem> = relics
        .iter()
        .enumerate()
        .map(|(idx, relic)| {
            let is_selected = idx == selected_relic_idx;
            let prefix = if is_selected { "> " } else { "  " };
            let status = if relic.3 { "+  " } else { "-  " };
            let style = if is_selected {
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD)
            } else if !relic.3 {
                Style::default().fg(theme.muted)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(accent_color)),
                Span::styled(
                    status,
                    Style::default().fg(if relic.3 {
                        theme.warning
                    } else {
                        theme.muted
                    }),
                ),
                Span::styled(&relic.1, style),
            ]))
        })
        .collect();

    let relic_list = List::new(relic_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Cosmetical Relics Inventory "),
    );
    f.render_widget(relic_list, relics_layout[0]);

    // Selected Relic Detail (and ASCII drawing)
    let relic_idx = selected_relic_idx.min(relics.len() - 1);
    let relic = &relics[relic_idx]; // (id, name, desc, unlocked, unlocked_at)

    let relic_detail_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // ASCII drawing
            Constraint::Percentage(60), // Content
        ])
        .split(relics_layout[1]);

    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if relic.3 {
            theme.warning
        } else {
            theme.muted
        }))
        .title(" Relic Stats & History ");

    f.render_widget(details_block, relics_layout[1]);

    // ASCII art hardcodeado por id de reliquia — si está bloqueada solo muestra [LOCKED], sin drama
    let ascii_art = if !relic.3 {
        "\n\n  [LOCKED]"
    } else {
        match relic.0.as_str() {
            "ancient_quill" => {
                "
        /
       /
      /
     /
    /___
    \\___)
"
            }
            "crystal_compass" => {
                "
       .=.
      / | \\
     (  *  )
      \\ | /
       '='
"
            }
            "rune_tablet" => {
                "
     .---.
     | * |
     | # |
     '---'
"
            }
            "explorers_map" => {
                "
     .-----.
     | ~~~ |
     |  X  |
     '-----'
"
            }
            "clock_of_focus" => {
                "
      .---.
     / \\_/ \\
    |   |   |
     \\_____/
"
            }
            _ => "\n   [*]\n  ARTIFACT",
        }
    };

    let ascii_p = Paragraph::new(ascii_art)
        .alignment(Alignment::Center)
        .style(Style::default().fg(if relic.3 {
            theme.warning
        } else {
            theme.muted
        }));
    f.render_widget(ascii_p, relic_detail_chunks[0]);

    // si la reliquia está bloqueada muestra el hint; si no, el nombre + fecha de desbloqueo y descripción
    let detail_text = if !relic.3 {
        vec![
            Line::from(Span::styled(&relic.1, Style::default().fg(theme.danger).add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(Span::styled("This relic is locked. Reaching the corresponding productivity milestone will unlock it in your hall.", Style::default().fg(theme.text))),
            Line::from(""),
            Line::from(Span::styled(format!("Hint: {}", relic.2), Style::default().fg(theme.muted))),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                &relic.1,
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(vec![
                Span::styled("UNLOCKED: ", Style::default().fg(theme.muted)),
                Span::styled(
                    relic.4.clone().unwrap_or_else(|| "Ancient Era".to_string()),
                    Style::default().fg(theme.success),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(&relic.2, Style::default().fg(Color::White))),
        ]
    };

    let detail_p = Paragraph::new(detail_text).wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(detail_p, relic_detail_chunks[1]);

    // 3. Help Footer
    let footer_p = Paragraph::new(Span::styled(
        "  Use Up/Down Arrows to inspect different Relics in your inventory.  ",
        Style::default().fg(theme.muted),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    )
    .alignment(Alignment::Center);
    f.render_widget(footer_p, main_chunks[2]);
}
