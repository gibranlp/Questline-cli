// ─────────────────────────────────────────────────────────────────────────────
// screens/character.rs — el perfil del héroe: stats, logros y reflexiones
// ─────────────────────────────────────────────────────────────────────────────

use crate::models::{DailyReflection, User, XPEvent};
use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph},
    Frame,
};

// función monstruo — recibe un chingo de parámetros para pintar todo el perfil del héroe
// izquierda: ficha del personaje, XP history, adventure log, gauge de progreso
// derecha: árbol de poderes de clase y el historial de reflexiones diarias
pub fn draw(
    f: &mut Frame,
    user: &User,
    achievements_count: i32,
    achievements_total: usize,
    tree_stage: &str,
    tree_growth: i32,
    tree_health: i32,
    streak_curr: i32,
    streak_best: i32,
    xp_history: &[XPEvent],
    most_productive_project: &str,
    reflections: &[DailyReflection],
    selected_reflection_idx: usize,
    modal: &crate::app::ModalType,
    devices: &[(String, String, String, Option<String>)],
    chronicle_entries: &[(String, i32, String, String)],
    selected_chronicle_idx: usize,
    character_focus: usize,
    reflection_detail_scroll: usize,
    theme: &Theme,
    area: ratatui::layout::Rect,
) {
    let size = area;
    let accent_color = theme.primary;

    // 62/38 — la hoja del personaje necesita más espacio que el árbol de poderes
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(size);

    // Left Panel Stack: Profile, Progression details, XP History, Adventure Log Book, XP Progress Bar
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(15), // Profile Info (expanded for devices list)
            Constraint::Length(6),  // RPG Progression summary
            Constraint::Length(7),  // XP History Summary
            Constraint::Min(7),     // Adventure Log Book (Up/Down to scroll)
            Constraint::Length(3),  // Progress Bar
        ])
        .split(chunks[0]);

    // 1. Profile Details — jalamos el nombre de todos los devices para mostrarlo en una línea
    let created_date_str = user.created_at.format("%Y-%m-%d").to_string();
    let device_names = devices
        .iter()
        .map(|d| d.1.clone())
        .collect::<Vec<String>>()
        .join(", ");
    let profile_text = vec![
        Line::from(vec![
            Span::styled("  NAME:    ", Style::default().fg(theme.muted)),
            Span::styled(
                &user.username,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  CLASS:   ", Style::default().fg(theme.muted)),
            Span::styled(
                user.class.name(),
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  TITLE:   ", Style::default().fg(theme.muted)),
            Span::styled(
                user.title(),
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  LEVEL:   ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", user.level),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        // la especialización se desbloquea al nivel 10 — antes sale "Locked", después te deja elegir
        Line::from(vec![
            Span::styled("  SPECIAL: ", Style::default().fg(theme.muted)),
            if let Some(ref spec) = user.specialization {
                Span::styled(
                    spec,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else if user.level >= 10 {
                Span::styled(
                    "Press [s] to Choose Specialization!",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    "Locked (Unlocks at Lvl 10)",
                    Style::default()
                        .fg(theme.muted),
                )
            },
        ]),
        Line::from(vec![
            Span::styled("  CREATED: ", Style::default().fg(theme.muted)),
            Span::styled(created_date_str, Style::default().fg(theme.text)),
        ]),
        Line::from(vec![
            Span::styled("  DEVICES: ", Style::default().fg(theme.muted)),
            Span::styled(device_names, Style::default().fg(Color::LightCyan)),
        ]),
        Line::from(""),
        Line::from("  MOTTO:"),
        Line::from(Span::styled(
            format!("  \"{}\"", user.class.flavor()),
            Style::default()
                .fg(accent_color)
                .add_modifier(Modifier::ITALIC),
        )),
        Line::from("  DESCRIPTION:"),
        Line::from(Span::styled(
            format!("  {}", user.class.description()),
            Style::default().fg(theme.text),
        )),
    ];

    let specs_sub = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(left_chunks[0]);

    let profile_p = Paragraph::new(profile_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Character Specs "),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(profile_p, specs_sub[0]);

    let mut passive_lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Always Active:",
            Style::default()
                .fg(theme.muted)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];
    for passive in user.class.passive_description().split("  |  ") {
        passive_lines.push(Line::from(vec![
            Span::styled("  ✦ ", Style::default().fg(accent_color)),
            Span::styled(
                passive.trim().to_string(),
                Style::default().fg(theme.text),
            ),
        ]));
    }
    let passives_p = Paragraph::new(passive_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Class Passives "),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(passives_p, specs_sub[1]);

    // 2. Progression Summary
    let rpg_summary = vec![
        Line::from(vec![
            Span::styled("  ACHIEVEMENTS: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} / {} Unlocked", achievements_count, achievements_total),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ZEN TREE:     ", Style::default().fg(theme.muted)),
            Span::styled(
                format!(
                    "{} ({} Growth, {}% Health)",
                    tree_stage, tree_growth, tree_health
                ),
                Style::default().fg(theme.success),
            ),
        ]),
        Line::from(vec![
            Span::styled("  STREAK:       ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} Days Active (Best: {} Days)", streak_curr, streak_best),
                Style::default().fg(Color::Rgb(249, 115, 22)),
            ),
        ]),
        Line::from(vec![
            Span::styled("  BEST REALM:   ", Style::default().fg(theme.muted)),
            Span::styled(
                most_productive_project,
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let rpg_p = Paragraph::new(rpg_summary).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Progression Summary "),
    );
    f.render_widget(rpg_p, left_chunks[1]);

    // 3. XP History
    let xp_items: Vec<ListItem> = if xp_history.is_empty() {
        vec![ListItem::new("  No XP events recorded.")]
    } else {
        xp_history
            .iter()
            .map(|e| ListItem::new(format!("  +{} XP — {}", e.xp_gained, e.event_type)))
            .collect()
    };
    let xp_list = List::new(xp_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Recent XP History "),
    );
    f.render_widget(xp_list, left_chunks[2]);

    // el log del adventure book con word-wrap manual — no hay widget nativo que lo haga bien aquí
    let log_items: Vec<ListItem> = if chronicle_entries.is_empty() {
        vec![ListItem::new(
            "  The chronicle is empty. Embark on quests and focus to write your history.",
        )]
    } else {
        // restamos 30 para dejar espacio al cursor + [Day NNN] + timestamp al inicio de línea
        let max_width = (left_chunks[3].width as usize).saturating_sub(30);
        chronicle_entries
            .iter()
            .enumerate()
            .map(|(idx, (_, day, text, timestamp))| {
                let is_selected = idx == selected_chronicle_idx;
                let cursor = if is_selected { "> " } else { "  " };

                let time_str = if timestamp.len() >= 10 {
                    &timestamp[0..10]
                } else {
                    timestamp
                };

                let item_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };

                // word wrap a mano — si la línea ya no aguanta más palabras, abrimos una nueva
                let mut lines = Vec::new();
                let words: Vec<&str> = text.split_whitespace().collect();
                let mut current_line = String::new();
                for word in words {
                    if current_line.is_empty() {
                        current_line = word.to_string();
                    } else if current_line.len() + 1 + word.len() <= max_width {
                        current_line.push(' ');
                        current_line.push_str(word);
                    } else {
                        lines.push(current_line);
                        current_line = word.to_string();
                    }
                }
                if !current_line.is_empty() {
                    lines.push(current_line);
                }

                let mut list_lines = Vec::new();
                if lines.is_empty() {
                    list_lines.push(Line::from(vec![
                        Span::styled(
                            cursor,
                            Style::default()
                                .fg(accent_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("[Day {:>3}] ", day),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("({}) ", time_str),
                            Style::default().fg(theme.muted),
                        ),
                        Span::styled("", item_style),
                    ]));
                } else {
                    list_lines.push(Line::from(vec![
                        Span::styled(
                            cursor,
                            Style::default()
                                .fg(accent_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("[Day {:>3}] ", day),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("({}) ", time_str),
                            Style::default().fg(theme.muted),
                        ),
                        Span::styled(lines[0].clone(), item_style),
                    ]));
                    // líneas de continuación llevan padding para alinearse con el texto de la primera
                    for line in &lines[1..] {
                        list_lines.push(Line::from(vec![
                            Span::styled("                          ", Style::default()),
                            Span::styled(line.clone(), item_style),
                        ]));
                    }
                }
                ListItem::new(list_lines)
            })
            .collect()
    };

    // el borde cambia de color según en qué panel está el focus — 0=log, 1=lista reflexiones, 2=detalle
    let log_border_style = if character_focus == 0 {
        Style::default().fg(accent_color)
    } else {
        Style::default().fg(theme.muted)
    };
    let log_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(log_border_style)
        .title(" Adventure Log Book (Up/Down to scroll) ");

    let log_list = List::new(log_items)
        .block(log_block)
        .highlight_style(Style::default().bg(Style::default().fg.unwrap_or(Color::Reset)));

    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(selected_chronicle_idx));
    f.render_stateful_widget(log_list, left_chunks[3], &mut state);

    // 4. Progress Gauge — clampeamos el ratio para que nunca pase de 1.0 aunque el XP sea raro
    let current_xp = user.xp;
    let next_level_xp = User::xp_for_next_level(user.level);
    let ratio = if next_level_xp > 0 {
        (current_xp as f64 / next_level_xp as f64).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" XP Progression "),
        )
        .gauge_style(Style::default().fg(accent_color).bg(Color::Rgb(30, 30, 30)))
        .label(format!(
            "{}/{} XP ({:.0}%)",
            current_xp,
            next_level_xp,
            ratio * 100.0
        ))
        .ratio(ratio);
    f.render_widget(gauge, left_chunks[4]);

    // árbol de poderes — los desbloqueados brillan con accent_color, los locked se ven apagados
    let powers = user.class.powers();
    let power_items: Vec<ListItem> = powers
        .into_iter()
        .map(|(lvl, name, _desc)| {
            let is_unlocked = user.level >= lvl;
            if is_unlocked {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  ✦ Lvl {:>2}  ", lvl),
                        Style::default().fg(theme.muted),
                    ),
                    Span::styled(
                        name,
                        Style::default()
                            .fg(accent_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  ○ Lvl {:>2}  ", lvl),
                        Style::default().fg(theme.muted),
                    ),
                    Span::styled(
                        name,
                        Style::default().fg(theme.muted),
                    ),
                ]))
            }
        })
        .collect();

    // Split right panel: Top (Progression Tree), Bottom (Daily Reflections History)
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    let powers_list = List::new(power_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(format!(" {} Class Progression Tree ", user.class.name())),
    );
    f.render_widget(powers_list, right_chunks[0]);

    // sección de reflexiones diarias — si está vacía muestra hint de cómo escribir una
    let ref_block_title = Span::styled(
        " Daily Reflections History ",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    let ref_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(ref_block_title);

    if reflections.is_empty() {
        let no_ref = Paragraph::new(
            "\n\n  No daily reflections recorded yet.\n  Press [r] on the Dashboard to write one.",
        )
        .block(ref_block)
        .alignment(Alignment::Center);
        f.render_widget(no_ref, right_chunks[1]);
    } else {
        let list_border_style = if character_focus == 1 {
            Style::default().fg(accent_color)
        } else {
            Style::default().fg(theme.muted)
        };
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(list_border_style)
            .title(" Entries ");

        let detail_border_style = if character_focus == 2 {
            Style::default().fg(accent_color)
        } else {
            Style::default().fg(theme.muted)
        };
        let detail_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(detail_border_style)
            .title(" Reflection Detail ");

        // protección para que el índice seleccionado no se pase del final si se borraron entradas
        let sel_idx = selected_reflection_idx.min(reflections.len() - 1);

        let list_items: Vec<ListItem> = reflections
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let date_str = r.created_date.format("%Y-%m-%d").to_string();
                let style = if i == sel_idx {
                    Style::default()
                        .fg(accent_color)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                let prefix = if i == sel_idx { "> " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(date_str, style),
                ]))
            })
            .collect();

        let ref_list = List::new(list_items)
            .block(list_block)
            .highlight_style(Style::default().bg(Style::default().fg.unwrap_or(Color::Reset)));

        let selected_ref = &reflections[sel_idx];
        let detail_text = vec![
            Line::from(vec![
                Span::styled(" DATE: ", Style::default().fg(theme.muted)),
                Span::styled(
                    selected_ref
                        .created_date
                        .format("%A, %B %e, %Y")
                        .to_string(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " WHAT WENT WELL:",
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  {}", selected_ref.what_went_well),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " WHAT CAN IMPROVE:",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  {}", selected_ref.what_can_improve),
                Style::default().fg(Color::White),
            )),
        ];

        // el detalle de la reflexión es scrolleable — focus 2 lo controla con up/down
        let scroll_y = reflection_detail_scroll as u16;
        let ref_detail = Paragraph::new(detail_text)
            .block(detail_block)
            .scroll((scroll_y, 0))
            .wrap(ratatui::widgets::Wrap { trim: true });

        let ref_sub_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
            .split(right_chunks[1]);

        let mut list_state = ratatui::widgets::ListState::default();
        list_state.select(Some(sel_idx));
        f.render_stateful_widget(ref_list, ref_sub_chunks[0], &mut list_state);
        f.render_widget(ref_detail, ref_sub_chunks[1]);
    }

    // modal de especialización — solo aparece cuando el héroe llega a nivel 10, órale
    if let crate::app::ModalType::SpecializationSelect {
        choices,
        selected_idx,
    } = modal
    {
        let area = crate::screens::intro::centered_rect(50, 30, size);
        f.render_widget(Clear, area);
        f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(theme.warning))
            .title(Span::styled(
                " Unlock Class Specialization ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ));

        let mut items = vec![
            Line::from(""),
            Line::from(" Choose your subclass specialization path:"),
            Line::from(""),
        ];

        // buscamos el detalle de cada spec en la lista de la clase — si no lo encuentra queda vacío
        let class_specs = user.class.specializations();

        for (idx, choice) in choices.iter().enumerate() {
            let detail = class_specs
                .iter()
                .find(|s| s.0 == choice)
                .map(|s| s.1)
                .unwrap_or("");
            if idx == *selected_idx {
                items.push(Line::from(vec![
                    Span::styled(
                        "> ",
                        Style::default()
                            .fg(accent_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{:<22}", choice),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!(" ({})", detail), Style::default().fg(theme.warning)),
                ]));
            } else {
                items.push(Line::from(vec![
                    Span::styled("    ", Style::default().fg(theme.muted)),
                    Span::styled(format!("{:<22}", choice), Style::default().fg(theme.text)),
                    Span::styled(
                        format!(" ({})", detail),
                        Style::default().fg(theme.muted),
                    ),
                ]));
            }
        }

        items.push(Line::from(""));
        items.push(Line::from(Span::styled(
            " Up/Down to navigate  |  [Enter] Confirm Specialization ",
            Style::default().fg(theme.muted),
        )));

        let p = Paragraph::new(items)
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(p, area);
    }
}
