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

// Genera arte animado para reliquias sin depender de imagenes externas ni caracteres especiales.
fn relic_art_lines(id: &str, unlocked: bool, theme: &Theme, ticks: usize) -> Vec<Line<'static>> {
    let frame = (ticks / 5) % 4;
    let gold = Style::default().fg(Color::Rgb(255, 213, 92)).add_modifier(Modifier::BOLD);
    let amber = Style::default().fg(Color::Rgb(242, 156, 48)).add_modifier(Modifier::BOLD);
    let cyan = Style::default().fg(Color::Rgb(85, 214, 217)).add_modifier(Modifier::BOLD);
    let blue = Style::default().fg(Color::Rgb(82, 151, 219)).add_modifier(Modifier::BOLD);
    let green = Style::default().fg(Color::Rgb(84, 180, 92)).add_modifier(Modifier::BOLD);
    let purple = Style::default().fg(Color::Rgb(178, 121, 216)).add_modifier(Modifier::BOLD);
    let stone = Style::default().fg(Color::Rgb(135, 145, 154)).add_modifier(Modifier::BOLD);
    let ink = Style::default().fg(Color::Rgb(210, 220, 232));
    let muted = Style::default().fg(theme.muted);

    if !unlocked {
        let pulse = if frame % 2 == 0 { theme.danger } else { theme.muted };
        return vec![
            Line::from(""),
            Line::from(Span::styled("      .----.      ", Style::default().fg(pulse).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("     / ____ \\     ", muted)),
            Line::from(Span::styled("    | |    | |    ", muted)),
            Line::from(Span::styled("    | |LOCK| |    ", Style::default().fg(pulse).add_modifier(Modifier::BOLD))),
            Line::from(Span::styled("    | |____| |    ", muted)),
            Line::from(Span::styled("     \\______/     ", muted)),
            Line::from(""),
        ];
    }

    match id {
        "ancient_quill" => {
            let tip = if frame % 2 == 0 { gold } else { cyan };
            vec![
                Line::from(Span::styled("       *     .    ", gold)),
                Line::from(vec![Span::raw("          "), Span::styled("//", cyan)]),
                Line::from(vec![Span::raw("        "), Span::styled("//", cyan), Span::styled("/ ", ink), Span::styled("*", gold)]),
                Line::from(vec![Span::raw("      "), Span::styled("//", cyan), Span::styled("/___", ink)]),
                Line::from(vec![Span::raw("    "), Span::styled("//", cyan), Span::styled("/____/", ink)]),
                Line::from(vec![Span::raw("      "), Span::styled("\\___", tip), Span::styled(")", amber)]),
                Line::from(Span::styled("    ink remembers", purple)),
            ]
        }
        "crystal_compass" => {
            let needle = if frame < 2 { gold } else { cyan };
            vec![
                Line::from(Span::styled("      .  *  .     ", cyan)),
                Line::from(vec![Span::raw("        "), Span::styled(".=.", gold)]),
                Line::from(vec![Span::raw("      "), Span::styled("/ ", stone), Span::styled("|", needle), Span::styled(" \\", stone)]),
                Line::from(vec![Span::raw("     "), Span::styled("( --", blue), Span::styled("*", gold), Span::styled("-- )", blue)]),
                Line::from(vec![Span::raw("      "), Span::styled("\\ ", stone), Span::styled("|", needle), Span::styled(" /", stone)]),
                Line::from(vec![Span::raw("        "), Span::styled("'='", gold)]),
                Line::from(Span::styled("    nearest quest", green)),
            ]
        }
        "rune_tablet" => {
            let rune = if frame % 2 == 0 { green } else { gold };
            vec![
                Line::from(Span::styled("       .---.      ", stone)),
                Line::from(vec![Span::raw("      "), Span::styled("/____\\", stone)]),
                Line::from(vec![Span::raw("      "), Span::styled("| ", stone), Span::styled("*", rune), Span::styled("  |", stone)]),
                Line::from(vec![Span::raw("      "), Span::styled("| ", stone), Span::styled("#", cyan), Span::styled("  |", stone)]),
                Line::from(vec![Span::raw("      "), Span::styled("| ", stone), Span::styled("+", rune), Span::styled("  |", stone)]),
                Line::from(vec![Span::raw("      "), Span::styled("'----'", stone)]),
                Line::from(Span::styled("     roots hum", green)),
            ]
        }
        "explorers_map" => {
            let mark = if frame % 2 == 0 { Color::Rgb(218, 55, 42) } else { Color::Rgb(255, 213, 92) };
            vec![
                Line::from(Span::styled("     .       *    ", amber)),
                Line::from(vec![Span::raw("     "), Span::styled(".------.", gold)]),
                Line::from(vec![Span::raw("    "), Span::styled("/ ~  ~ /", ink)]),
                Line::from(vec![Span::raw("   "), Span::styled("/  ", ink), Span::styled("X", Style::default().fg(mark).add_modifier(Modifier::BOLD)), Span::styled("  /", ink)]),
                Line::from(vec![Span::raw("  "), Span::styled("/__~__/", gold)]),
                Line::from(Span::styled("    path shifts", cyan)),
            ]
        }
        "clock_of_focus" => {
            let hand = if frame % 2 == 0 { gold } else { cyan };
            vec![
                Line::from(Span::styled("       .---.      ", blue)),
                Line::from(vec![Span::raw("      "), Span::styled("/ .-. \\", stone)]),
                Line::from(vec![Span::raw("     "), Span::styled("|  ", stone), Span::styled("|", hand), Span::styled("  |", stone)]),
                Line::from(vec![Span::raw("     "), Span::styled("|  ", stone), Span::styled("+--", hand), Span::styled("|", stone)]),
                Line::from(vec![Span::raw("      "), Span::styled("\\___/", blue)]),
                Line::from(Span::styled("    time bends", purple)),
            ]
        }
        _ => vec![
            Line::from(Span::styled("      * * *      ", gold)),
            Line::from(Span::styled("     ARTIFACT    ", cyan)),
            Line::from(Span::styled("      * * *      ", gold)),
        ],
    }
}

// toda la pantalla: récords personales a la izq, inventario de reliquias a la der
// la tupla de relics es (id, nombre, desc, desbloqueada, fecha_unlock) — medio fea pero funciona
pub fn draw(
    f: &mut Frame,
    stats: &Statistics,
    selected_relic_idx: usize,
    // Relic status: (id, name, desc, unlocked, unlocked_at)
    relics: &[(String, String, String, bool, Option<String>)],
    theme: &Theme,
    animation_ticks: usize,
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
                "  Quests Completed:      ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} Tasks", stats.tasks_completed),
                Style::default().fg(theme.success),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "  Campaigns Completed:  ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{} Campaigns", stats.projects_completed),
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

    let ascii_p = Paragraph::new(relic_art_lines(&relic.0, relic.3, theme, animation_ticks))
        .alignment(Alignment::Center);
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
