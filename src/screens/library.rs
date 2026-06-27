// ─────────────────────────────────────────────────────────────────────────────
// screens/library.rs — la biblioteca de lore: fragmentos, memorias y piezas coleccionables
// ─────────────────────────────────────────────────────────────────────────────

use crate::theme::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

// Categorías de la biblioteca — cada una tiene su propio conjunto de entradas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryCategory {
    ClassQuests,
    ClassStories,
    WorldHistory,
    Achievements,
    MemoryFragments,
}

impl LibraryCategory {
    pub fn name(&self) -> &'static str {
        match self {
            LibraryCategory::ClassQuests => "Class Quests",
            LibraryCategory::ClassStories => "Class Stories",
            LibraryCategory::WorldHistory => "World History",
            LibraryCategory::Achievements => "Achievement Lore",
            LibraryCategory::MemoryFragments => "Memory Fragments",
        }
    }
}

// Draw principal — navegación de tres columnas: categoría → items → detalles
// Los parámetros son bastante gordos porque todo se pasa desde el app state
pub fn draw(
    f: &mut Frame,
    // Active column: 0 = Categories, 1 = Items, 2 = Details (scroll content)
    active_col: usize,
    selected_cat: usize,
    selected_item: usize,
    scroll_offset: u16,
    // Quests details: (class_name, unlock_level, name, desc, status, progress, target, reward)
    quests: &[(String, i32, String, String, String, i32, i32, String)],
    // Lore entries: (id, category, title, content, unlocked, unlocked_at)
    lore_entries: &[(String, String, String, String, bool, Option<String>)],
    user_class: &str,
    theme: &Theme,
) {
    let size = f.size();
    let accent_color = theme.primary;

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Library explorer
            Constraint::Length(3), // Instructions footer
        ])
        .split(size);

    // 1. Header
    let header_p = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            " THE LORE LIBRARY & QUEST ARCHIVE ",
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " - Discover the ancient chronicles and complete class quests.",
            Style::default().fg(theme.text),
        ),
        Span::styled(
            "  [G] → Hall of Legends",
            Style::default().fg(theme.muted),
        ),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border)),
    );
    f.render_widget(header_p, main_chunks[0]);

    // Split explorer into 3 columns
    let col_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(22), // Categories
            Constraint::Percentage(33), // Items
            Constraint::Percentage(45), // Details
        ])
        .split(main_chunks[1]);

    let categories = [
        LibraryCategory::ClassQuests,
        LibraryCategory::ClassStories,
        LibraryCategory::WorldHistory,
        LibraryCategory::Achievements,
        LibraryCategory::MemoryFragments,
    ];

    // Render Column 0: lista de categorías con highlight del activo
    let cat_items: Vec<ListItem> = categories
        .iter()
        .enumerate()
        .map(|(idx, cat)| {
            let is_selected = idx == selected_cat;
            let is_active = is_selected && active_col == 0;
            let style = if is_active {
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };
            let prefix = if is_selected { "> " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(accent_color)),
                Span::styled(cat.name(), style),
            ]))
        })
        .collect();

    let cat_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if active_col == 0 {
            accent_color
        } else {
            theme.muted
        }))
        .title(" Categories ");
    let cat_list = List::new(cat_items).block(cat_block);
    f.render_widget(cat_list, col_chunks[0]);

    // Filtro de lore por categoría — ClassStories tiene lógica especial por clase del usuario
    let cur_cat = categories[selected_cat];
    let cat_str = match cur_cat {
        LibraryCategory::ClassQuests => "",
        LibraryCategory::ClassStories => "Class",
        LibraryCategory::WorldHistory => "World",
        LibraryCategory::Achievements => "Achievement",
        LibraryCategory::MemoryFragments => "Memory",
    };

    // Convierte el nombre de clase a su key en los IDs del lore — qué rollo si agregan más clases
    let class_key = match user_class {
        "Code Warlock" => "warlock",
        "Task Paladin" => "paladin",
        "Mind Sage" => "sage",
        "Systems Architect" => "architect",
        "Time Chronomancer" => "chronomancer",
        "Arch Accountant" => "accountant",
        _ => "",
    };

    // ClassStories muestra lore compartido (six_orders, council) + lore específico de la clase
    let filtered_lore_entries: Vec<&(String, String, String, String, bool, Option<String>)> = if cur_cat == LibraryCategory::ClassStories {
        lore_entries
            .iter()
            .filter(|e| {
                e.1 == "Class"
                    && (e.0 == "class_six_orders"
                        || e.0 == "class_council_orders"
                        || (!class_key.is_empty() && e.0.starts_with(&format!("class_{}_", class_key))))
            })
            .collect()
    } else {
        lore_entries.iter().filter(|e| e.1 == cat_str).collect()
    };

    // Render Column 1: items de la categoría activa
    let mut items_lines: Vec<String> = Vec::new();
    let mut items_unlocked: Vec<bool> = Vec::new();

    match cur_cat {
        LibraryCategory::ClassQuests => {
            for q in quests {
                items_lines.push(format!("Lvl {} - {}", q.1, q.2));
                items_unlocked.push(q.4 != "Locked");
            }
        }
        _ => {
            for entry in &filtered_lore_entries {
                items_lines.push(entry.2.clone());
                items_unlocked.push(entry.4);
            }
        }
    }

    // IDs paralelos a items_lines — solo para MemoryFragments; el resto usa None
    let fragment_ids: Vec<Option<String>> = if cur_cat == LibraryCategory::MemoryFragments {
        filtered_lore_entries.iter().map(|e| Some(e.0.clone())).collect()
    } else {
        vec![None; items_lines.len()]
    };

    let item_items: Vec<ListItem> = if items_lines.is_empty() {
        vec![ListItem::new("  (No entries found)")]
    } else {
        items_lines
            .iter()
            .enumerate()
            .map(|(idx, title)| {
                let is_selected = idx == selected_item;
                let is_active = is_selected && active_col == 1;
                let unlocked = items_unlocked.get(idx).cloned().unwrap_or(true);

                // Colores de rareza solo para fragmentos — memory_999 es el legendario
                let frag_id = fragment_ids.get(idx).and_then(|o| o.as_deref());
                let rarity_color = frag_id.map(|id| match id {
                    "memory_999" => theme.warning,
                    "memory_077" | "memory_112" | "memory_144" | "memory_188" => Color::Cyan,
                    _ => theme.muted,
                });

                let base_color = rarity_color.unwrap_or(theme.muted);

                let style = if is_active {
                    Style::default()
                        .fg(accent_color)
                        .add_modifier(Modifier::BOLD)
                } else if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else if !unlocked {
                    Style::default().fg(theme.muted)
                } else {
                    Style::default().fg(base_color)
                };

                let prefix = if is_selected { "> " } else { "  " };
                let lock_icon = if !unlocked { "L " } else { "" };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, Style::default().fg(accent_color)),
                    Span::styled(lock_icon, Style::default().fg(theme.muted)),
                    Span::styled(title, style),
                ]))
            })
            .collect()
    };

    // El título de la columna de items muestra cuántos fragmentos encontró el user
    let found_count = if cur_cat == LibraryCategory::MemoryFragments {
        filtered_lore_entries.iter().filter(|e| e.4).count()
    } else {
        0
    };
    let items_title = if cur_cat == LibraryCategory::MemoryFragments {
        format!(" Memory Fragments | Found: {}/15 ", found_count)
    } else {
        format!(" {} ", cur_cat.name())
    };

    let item_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if active_col == 1 {
            accent_color
        } else {
            theme.muted
        }))
        .title(items_title);
    let item_list = List::new(item_items).block(item_block);
    f.render_widget(item_list, col_chunks[1]);

    // Render Column 2: panel de detalles — bifurca entre quests y lore general
    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.border))
        .title(" Chronicle Details ");

    if items_lines.is_empty()
        || selected_item >= items_lines.join("").len() && items_lines.is_empty()
    {
        let p = Paragraph::new("\n\n  No item selected.")
            .block(details_block)
            .alignment(Alignment::Center);
        f.render_widget(p, col_chunks[2]);
    } else if cur_cat == LibraryCategory::ClassQuests {
        let q_idx = selected_item.min(quests.len() - 1);
        let q = &quests[q_idx]; // (class_name, unlock_level, name, desc, status, progress, target, reward)
        let status_color = match q.4.as_str() {
            "Completed" => theme.success,
            "Active" => theme.warning,
            "Available" => Color::Cyan,
            _ => theme.muted,
        };

        // Progreso como ratio 0.0-1.0 para la gauge — clamp por si los datos están chistosos
        let progress_ratio = if q.6 > 0 {
            (q.5 as f64 / q.6 as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let detail_lines = vec![
            Line::from(vec![
                Span::styled(" QUEST: ", Style::default().fg(theme.muted)),
                Span::styled(
                    &q.2,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(" CLASS: ", Style::default().fg(theme.muted)),
                Span::styled(&q.0, Style::default().fg(accent_color)),
                Span::styled(
                    format!(" (Level {} Quest)", q.1),
                    Style::default().fg(theme.text),
                ),
            ]),
            Line::from(vec![
                Span::styled(" STATUS: ", Style::default().fg(theme.muted)),
                Span::styled(
                    &q.4,
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                " OBJECTIVE:",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  {}", q.3),
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " PROGRESS:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  {}/{} completed", q.5, q.6),
                Style::default().fg(Color::White),
            )),
        ];

        // Layout interno del panel: texto + gauge de progreso + acción disponible
        let detail_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Length(3), // Progress Gauge
                Constraint::Min(2),    // Actions instruction
            ])
            .split(details_block.inner(col_chunks[2]));

        f.render_widget(details_block, col_chunks[2]);

        let detail_p = Paragraph::new(detail_lines).wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(detail_p, detail_layout[0]);

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.border))
                    .title(" Quest Progression "),
            )
            .gauge_style(Style::default().fg(status_color).bg(Color::Rgb(30, 30, 30)))
            .ratio(progress_ratio)
            .label(format!("{:.0}%", progress_ratio * 100.0));
        f.render_widget(gauge, detail_layout[1]);

        // El CTA cambia según el estado — Available = empezar, Active = esperar o reclamar
        let mut actions = vec![Line::from("")];
        if q.4 == "Available" {
            actions.push(Line::from(Span::styled(
                " Press [Space] to Embark on this Quest! ",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if q.4 == "Active" {
            actions.push(Line::from(Span::styled(
                " Quest active. Fuel progress via productivity. ",
                Style::default().fg(Color::Cyan),
            )));
            if q.5 >= q.6 {
                actions.push(Line::from(Span::styled(
                    " Press [Space] to Claim Victory! ",
                    Style::default()
                        .fg(theme.success)
                        .add_modifier(Modifier::BOLD),
                )));
            }
        } else if q.4 == "Completed" {
            actions.push(Line::from(Span::styled(
                " Completed! Reward Unlocked:  ",
                Style::default().fg(theme.success),
            )));
            actions.push(Line::from(Span::styled(
                format!("     {}", q.7),
                Style::default()
                    .fg(theme.text)
                    .add_modifier(Modifier::ITALIC),
            )));
        } else {
            actions.push(Line::from(Span::styled(
                " Locked. Advance your class Level to unlock. ",
                Style::default().fg(theme.muted),
            )));
        }
        let action_p = Paragraph::new(actions).alignment(Alignment::Center);
        f.render_widget(action_p, detail_layout[2]);
    } else {
        // Lore general — muestra bloqueado o el contenido completo con scroll
        if filtered_lore_entries.is_empty() {
            let p = Paragraph::new("\n\n  No item selected.")
                .block(details_block)
                .alignment(Alignment::Center);
            f.render_widget(p, col_chunks[2]);
        } else {
            let e_idx = selected_item.min(filtered_lore_entries.len() - 1);
            let entry = filtered_lore_entries[e_idx]; // (id, category, title, content, unlocked, unlocked_at)

            let detail_inner = details_block.inner(col_chunks[2]);
            f.render_widget(details_block, col_chunks[2]);

        if !entry.4 {
            // Entrada bloqueada — ni modo, hay que ganársela primero
            let locked_text = vec![
                Line::from(""),
                Line::from(Span::styled(
                    " RECORD LOCKED ",
                    Style::default().fg(theme.danger).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "This chapter of lore remains hidden in the shadow of unfinished deeds.",
                    Style::default().fg(theme.text),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("Requirement: Unlock the milestone related to '{}'", entry.2),
                    Style::default().fg(theme.muted),
                )),
            ];
            let locked_p = Paragraph::new(locked_text)
                .alignment(Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true });
            f.render_widget(locked_p, detail_inner);
        } else {
            // Rareza solo aplica para MemoryFragments — el resto no tiene rarity label
            let frag_rarity = if cur_cat == LibraryCategory::MemoryFragments {
                match entry.0.as_str() {
                    "memory_999" => Some(("Legendary", theme.warning)),
                    "memory_077" | "memory_112" | "memory_144" | "memory_188" => {
                        Some(("Rare", Color::Cyan))
                    }
                    id if id.starts_with("memory_") => Some(("Common", Color::White)),
                    _ => None,
                }
            } else {
                None
            };

            let mut text = vec![
                Line::from(vec![
                    Span::styled(" TITLE: ", Style::default().fg(theme.muted)),
                    Span::styled(
                        &entry.2,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" TYPE:  ", Style::default().fg(theme.muted)),
                    Span::styled(cur_cat.name(), Style::default().fg(accent_color)),
                ]),
            ];
            if let Some((rarity_label, rarity_color)) = frag_rarity {
                text.push(Line::from(vec![
                    Span::styled(" RARITY:", Style::default().fg(theme.muted)),
                    Span::styled(
                        format!(" [ {} ]", rarity_label),
                        Style::default()
                            .fg(rarity_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
            text.push(Line::from(vec![
                Span::styled(" FOUND: ", Style::default().fg(theme.muted)),
                Span::styled(
                    entry.5.clone().unwrap_or_else(|| "Ancient Era".to_string()),
                    Style::default().fg(theme.text),
                ),
            ]));
            text.push(Line::from(""));
            // Divider distinto para fragmentos vs crónicas normales
            let divider_label = if cur_cat == LibraryCategory::MemoryFragments {
                "--- MEMORY FRAGMENT ---"
            } else {
                "--- CHRONICLE ENTRY ---"
            };
            text.push(Line::from(Span::styled(
                divider_label,
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )));
            text.push(Line::from(""));

            // El contenido puede ser multilinea — se preservan los saltos originales
            for line in entry.3.lines() {
                text.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(Color::White),
                )));
            }

            let p = Paragraph::new(text)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .scroll((scroll_offset, 0));
            f.render_widget(p, detail_inner);
        }
        }
    }

    // 3. Footer — las instrucciones cambian según la columna activa
    let inst_text = if active_col == 0 {
        "Press [Right Arrow] or [Tab] to browse entries | [Esc] back"
    } else if active_col == 1 {
        "Press [Left Arrow] to select categories | [Right Arrow] to view details | [Space] to act on quest"
    } else {
        "Press [Left Arrow] to return to entries | [Up/Down Arrow] to scroll content"
    };
    let footer_p = Paragraph::new(Span::styled(
        format!("  Keys: {}  ", inst_text),
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
