// ─────────────────────────────────────────────────────────────────────────────
// dashboard.rs — la pantalla principal del héroe, aquí va todo el desmadre
// ─────────────────────────────────────────────────────────────────────────────
use crate::app::{App, ModalType};
use crate::models::{Project, Task, User};
use crate::screens::intro::centered_rect;
use crate::theme::Theme;
use chrono::Utc;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph},
    Frame,
};

fn get_project_name(project_id: Option<uuid::Uuid>, projects: &[Project]) -> String {
    if let Some(pid) = project_id {
        projects
            .iter()
            .find(|p| p.id == pid)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "General".to_string())
    } else {
        "General".to_string()
    }
}

// La función más importante del archivo — dibuja TODO el dashboard de un jalón
// si esto tarda, el usuario lo va a sentir, ojo con las queries al DB
pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let size = area;
    let accent_color = theme.primary;
    let user = app.user.as_ref().unwrap(); // unwrap seguro: si no hay user, ni entramos aquí

    // Layout adaptivo — si la terminal es chica apretamos filas, si es grande damos más espacio
    let constraints = if size.height < 42 {
        vec![
            Constraint::Length(4),      // Row 1: Profile + XP Gauge
            Constraint::Percentage(35), // Row 2: Tree + Adventures + Streak & Trophies
            Constraint::Length(7),      // Row 3: Rituals + Focus stats & reflection box
            Constraint::Min(4),         // Row 4: Tasks + Summary
        ]
    } else {
        vec![
            Constraint::Length(5),      // Row 1: Profile + XP Gauge
            Constraint::Percentage(42), // Row 2: Tree + Adventures + Streak & Trophies
            Constraint::Length(9),      // Row 3: Rituals + Focus stats & reflection box
            Constraint::Min(6),         // Row 4: Tasks + Summary
        ]
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(size);

    // ── ROW 1: Perfil del aventurero y su barra de XP ──
    let profile_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(chunks[0]);

    let mut profile_text = vec![
        Line::from(vec![
            Span::styled("Character: ", Style::default().fg(theme.muted)),
            Span::styled(
                &user.username,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Class:     ", Style::default().fg(theme.muted)),
            Span::styled(
                user.class.name(),
                Style::default()
                    .fg(accent_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Title:     ", Style::default().fg(theme.muted)),
            Span::styled(user.title(), Style::default().fg(theme.warning)),
        ]),
    ];
    // Órale — si hay backups corruptos le avisamos al usuario bien visible
    if !app.corrupted_backups_found.is_empty() {
        profile_text.push(Line::from(vec![Span::styled(
            "[!] Corrupted Backup!",
            Style::default()
                .fg(Color::LightRed)
                .add_modifier(Modifier::BOLD),
        )]));
    }
    let profile_p = Paragraph::new(profile_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Adventurer "),
    );
    f.render_widget(profile_p, profile_layout[0]);

    // Calcula el ratio de XP para la barra — clamp evita que se pase de 1.0 si algo sale mal
    let current_xp = user.xp;
    let next_level_xp = User::xp_for_next_level(user.level);
    let ratio = if next_level_xp > 0 {
        (current_xp as f64 / next_level_xp as f64).clamp(0.0, 1.0)
    } else {
        1.0 // ya llegó al tope, qué chido
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(format!(" Level {} Progression ", user.level)),
        )
        .gauge_style(Style::default().fg(accent_color).bg(Color::Rgb(30, 30, 30)))
        .label(format!(
            "XP: {} / {} ({:.0}%)",
            current_xp,
            next_level_xp,
            ratio * 100.0
        ))
        .ratio(ratio);
    f.render_widget(gauge, profile_layout[1]);

    // ── ROW 2: El árbol zen, las quests del día y las rachas del héroe ──
    let hub_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(chunks[1]);

    // 2a. El arbolito zen — refleja el progreso acumulado del usuario, cuídalo!
    let zen_tree = app.db.get_zen_tree().unwrap();
    let tree_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.success))
        .title(" Little Ent  ");
    let tree_inner = tree_block.inner(hub_layout[0]);
    f.render_widget(tree_block, hub_layout[0]);

    let tree_sub = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(tree_inner);

    let tree_ascii = Paragraph::new(zen_tree.ascii_art())
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.success));
    f.render_widget(tree_ascii, tree_sub[0]);

    let last_watered_str = match zen_tree.last_watered {
        Some(dt) => dt.with_timezone(&chrono::Local).format("%H:%M").to_string(),
        None => "Never".to_string(),
    };

    // growth es acumulativo así que usamos módulo 100 para el progreso del stage actual
    let growth_ratio = ((zen_tree.growth % 100) as f64 / 100.0).clamp(0.0, 1.0);
    let health_ratio = (zen_tree.health as f64 / 100.0).clamp(0.0, 1.0);

    // Closure para dibujar barras de texto con bloques — se reutiliza para growth y health
    let render_bar = |ratio: f64, width: usize| -> String {
        let filled = (ratio * width as f64).round() as usize;
        format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
    };

    let growth_bar = render_bar(growth_ratio, 8);
    let health_bar = render_bar(health_ratio, 8);

    let tree_status = vec![
        Line::from(vec![
            Span::styled(" Stage:  ", Style::default().fg(theme.muted)),
            Span::styled(
                zen_tree.stage_name(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Growth: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} pts ", zen_tree.growth),
                Style::default().fg(theme.success),
            ),
            Span::styled(
                format!("[{}]", growth_bar),
                Style::default().fg(theme.success),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Health: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}% ", zen_tree.health),
                Style::default().fg(accent_color),
            ),
            Span::styled(format!("[{}]", health_bar), Style::default().fg(accent_color)),
        ]),
        Line::from(vec![
            Span::styled(" Water:  ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}/2 watered", zen_tree.water_today),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Last:   ", Style::default().fg(theme.muted)),
            Span::styled(last_watered_str, Style::default().fg(theme.text)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            " [w] Water Tree",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
    ];
    let tree_status_p = Paragraph::new(tree_status);
    f.render_widget(tree_status_p, tree_sub[1]);

    // 2b. Las quests del día — el engine las genera automáticamente cada mañana
    let daily_adventures = app.db.get_daily_adventures().unwrap_or_default();
    let comp_count = daily_adventures.iter().filter(|a| a.completed).count();
    let adv_items: Vec<ListItem> = if daily_adventures.is_empty() {
        vec![ListItem::new("  No adventures today.")]
    } else {
        daily_adventures
            .iter()
            .map(|a| {
                let status = if a.completed { "[x]" } else { "[ ]" };
                ListItem::new(format!(
                    "  {} {} ({}/{})",
                    status, a.title, a.current_count, a.target_count
                ))
            })
            .collect()
    };
    let adv_list = List::new(adv_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.warning))
            .title(format!(" Daily Quests ({}/3) ", comp_count)),
    );
    f.render_widget(adv_list, hub_layout[1]);

    // 2c. Rachas y trofeos — solo mostramos los primeros 4 achievements para no saturar
    let streak_obj = app.db.get_streak().unwrap();
    let achievements = app.db.get_achievements().unwrap_or_default();
    let achievements_unlocked = achievements
        .iter()
        .filter(|a| a.unlocked_at.is_some())
        .count();
    let ach_items: Vec<ListItem> = if achievements.is_empty() {
        vec![ListItem::new("  No achievements recorded.")]
    } else {
        achievements
            .iter()
            .take(4)
            .map(|a| {
                if a.unlocked_at.is_some() {
                    ListItem::new(format!("  [Unlocked] {}", a.name))
                } else {
                    ListItem::new(format!("  [Locked] {}", a.name))
                }
            })
            .collect()
    };

    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(249, 115, 22)))
        .title(" Trophies & Streaks ");
    let right_inner = right_block.inner(hub_layout[2]);
    f.render_widget(right_block, hub_layout[2]);

    let right_sub = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(4)])
        .split(right_inner);

    let streak_info = vec![
        Line::from(vec![
            Span::styled("  Current Streak: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} Days", streak_obj.current_streak),
                Style::default()
                    .fg(Color::Rgb(249, 115, 22))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Best Streak:    ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} Days", streak_obj.best_streak),
                Style::default().fg(theme.warning),
            ),
        ]),
    ];
    let streak_info_p = Paragraph::new(streak_info);
    f.render_widget(streak_info_p, right_sub[0]);

    let ach_list = List::new(ach_items).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.border))
            .title(format!(" Achievements ({}/14) ", achievements_unlocked)),
    );
    f.render_widget(ach_list, right_sub[1]);

    // ── ROW 3: Sidequests diarias, stats de deep work y sincronía en la nube ──
    let row3_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(chunks[2]);

    // 3a. Lista de sidequests (hábitos) — el historial es por fecha, no por ID global
    let rituals = app.db.get_rituals().unwrap_or_default();
    let completed_rituals = app
        .db
        .get_ritual_history_for_date(chrono::Local::now().date_naive())
        .unwrap_or_default();

    let ritual_items: Vec<ListItem> = if rituals.is_empty() {
        vec![ListItem::new("  No sidequests yet, add one | [n] new ")]
    } else {
        rituals
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                let is_completed = completed_rituals.contains(&r.id);
                let check = if is_completed { "[x]" } else { "[ ]" };
                // El cursor visual — el ">" se mueve con las flechas del teclado
                let highlight = if idx == app.selected_ritual_idx {
                    "> "
                } else {
                    "  "
                };
                let style = if idx == app.selected_ritual_idx {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.text)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        highlight,
                        Style::default()
                            .fg(accent_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{} ", check), Style::default()),
                    Span::styled(&r.name, style),
                    Span::styled(
                        format!(" ({}, +{} XP)", r.frequency, r.reward_xp),
                        Style::default().fg(theme.muted),
                    ),
                ]))
            })
            .collect()
    };

    let rituals_list = List::new(ritual_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(" Sidequests — [Space] Done | [n] New | [Delete] "),
    );
    let mut rituals_state = ListState::default();
    if !rituals.is_empty() {
        rituals_state.select(Some(app.selected_ritual_idx));
    }
    f.render_stateful_widget(rituals_list, row3_layout[0], &mut rituals_state);

    // 3b. Resumen de focus sessions y el recordatorio de reflexión diaria
    let stats = app.db.get_statistics().unwrap();
    let reflected_today = app
        .db
        .get_reflection_for_date(chrono::Local::now().date_naive())
        .unwrap_or(None)
        .is_some();

    let focus_and_ref_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Focus summary metrics (expanded for soundscapes)
            Constraint::Length(4), // Reflection status box
        ])
        .split(row3_layout[1]);

    // Tres queries para soundscapes: último usado, favorito y más productivo
    let last_soundscape = app
        .db
        .get_last_soundscape_used()
        .unwrap_or_else(|_| "None".to_string());
    let fav_soundscape = app
        .db
        .get_favorite_soundscape()
        .unwrap_or_else(|_| "None".to_string());
    let prod_soundscape = app
        .db
        .get_most_productive_soundscape()
        .unwrap_or_else(|_| "None".to_string());

    let focus_summary_text = vec![
        Line::from(vec![
            Span::styled("  Focus Sessions: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} completed", stats.sessions_completed),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  |  Deep Work Hours: ",
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{:.1} hrs", stats.focus_hours),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Last Used: ", Style::default().fg(theme.muted)),
            Span::styled(last_soundscape, Style::default().fg(Color::White)),
            Span::styled(" | Fav: ", Style::default().fg(theme.muted)),
            Span::styled(fav_soundscape, Style::default().fg(theme.warning)),
            Span::styled(" | Productive: ", Style::default().fg(theme.muted)),
            Span::styled(prod_soundscape, Style::default().fg(theme.success)),
        ]),
    ];
    let focus_summary_p = Paragraph::new(focus_summary_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Deep Work Summary "),
    );
    f.render_widget(focus_summary_p, focus_and_ref_layout[0]);

    // El box cambia de color y mensaje dependiendo si ya reflejó hoy o no
    let (ref_text, ref_block_style) = if reflected_today {
        (
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Daily Reflection recorded. You checked in on your growth today!",
                    Style::default().fg(theme.success),
                )),
            ],
            Style::default().fg(theme.success),
        )
    } else {
        (
            vec![
                Line::from(Span::styled(
                    " Press [r] to record your reflection today ",
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(" +25 XP ", Style::default().fg(Color::White))),
            ],
            Style::default().fg(theme.warning),
        )
    };
    let ref_p = Paragraph::new(ref_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(ref_block_style)
            .title(" Daily Reflection Status "),
    );
    f.render_widget(ref_p, focus_and_ref_layout[1]);

    // 3c. Estado de la sincronía con el servidor — parsea el RFC3339 y lo convierte a tiempo relativo
    let last_sync_raw = app
        .db
        .get_setting("last_sync")
        .unwrap_or(None)
        .unwrap_or_else(|| "Never".to_string());
    // Convierte timestamp crudo a string legible: "Just now", "5 mins ago", etc.
    let last_sync_formatted = if last_sync_raw == "Never" {
        "Never".to_string()
    } else if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&last_sync_raw) {
        let diff = chrono::Utc::now().signed_duration_since(dt.with_timezone(&chrono::Utc));
        if diff.num_seconds() < 60 {
            "Just now".to_string()
        } else if diff.num_minutes() < 60 {
            format!("{} mins ago", diff.num_minutes())
        } else if diff.num_hours() < 24 {
            format!("{} hours ago", diff.num_hours())
        } else {
            format!("{} days ago", diff.num_days())
        }
    } else {
        last_sync_raw
    };

    // pending_changes en amarillo si hay algo por sincronizar, verde si todo está al día
    let dev_count = app.db.get_devices().unwrap_or_default().len();
    let pending_changes = app.db.get_pending_sync_logs().map(|l| l.len()).unwrap_or(0);

    let chronicle_text = vec![
        Line::from(vec![
            Span::styled("  Last Sync: ", Style::default().fg(theme.muted)),
            Span::styled(
                last_sync_formatted,
                Style::default()
                    .fg(theme.success)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Devices:   ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} connected", dev_count),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Pending:   ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} changes", pending_changes),
                Style::default()
                    .fg(if pending_changes > 0 {
                        theme.warning
                    } else {
                        theme.success
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    let chronicle_p = Paragraph::new(chronicle_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(accent_color))
            .title(" Cloud Chronicle "),
    );
    f.render_widget(chronicle_p, row3_layout[2]);

    // ── ROW 4: Tasks activas, proyectos compartidos y actividad del gremio ──
    let bottom_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Left: Active & Assigned Quests
            Constraint::Percentage(30), // Middle: Shared Projects & Invites
            Constraint::Percentage(35), // Right: Activity & Mentions
        ])
        .split(chunks[3]);

    // 4a. Lista jerárquica de tasks — padres primero, luego sus steps incompletos
    // No manches, el sort por due_date con None al final requiere ese match tan verbose
    let all_tasks = &app.all_tasks;
    let today = Utc::now().date_naive();

    // Construye lista plana: parent task seguida de sus steps, ordenada por fecha límite
    let mut parents: Vec<&Task> = all_tasks
        .iter()
        .filter(|t| !t.completed && t.parent_task_id.is_none())
        .collect();
    parents.sort_by(|a, b| match (a.due_date, b.due_date) {
        (Some(d1), Some(d2)) => d1.cmp(&d2),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => b.created_at.cmp(&a.created_at),
    });
    // (is_step, task)
    let mut flat: Vec<(bool, &Task)> = Vec::new();
    for parent in &parents {
        flat.push((false, parent));
        let mut steps: Vec<&Task> = all_tasks
            .iter()
            .filter(|t| t.parent_task_id == Some(parent.id) && !t.completed)
            .collect();
        steps.sort_by_key(|s| s.created_at);
        for step in steps {
            flat.push((true, step));
        }
    }

    let parent_count = parents.len();
    // saturating_sub para no entrar en pánico si flat está vacío
    let sel_idx = app.selected_dashboard_task_idx.min(flat.len().saturating_sub(1));

    let left_items: Vec<ListItem> = if flat.is_empty() {
        vec![ListItem::new("  No active quests.")]
    } else {
        flat.iter().enumerate().map(|(i, (is_step, t))| {
            let is_sel = app.dashboard_task_focus && i == sel_idx;
            if *is_step {
                let (prefix_style, title_style) = if is_sel {
                    (
                        Style::default().fg(accent_color).add_modifier(Modifier::BOLD),
                        Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD),
                    )
                } else {
                    (
                        Style::default().fg(theme.secondary),
                        Style::default().fg(theme.muted),
                    )
                };
                let prefix = if is_sel { "     > o  " } else { "       o  " };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, prefix_style),
                    Span::styled(&t.title, title_style),
                ]))
            } else {
                let proj_name = get_project_name(t.project_id, &app.projects);
                // La etiqueta de fecha: rojo si ya venció, neutral si es hoy o futuro
                let due_label = match t.due_date {
                    Some(d) => {
                        let d_naive = d.date_naive();
                        if d_naive < today {
                            format!(" !! OVERDUE ({})", d_naive)
                        } else if d_naive == today {
                            " - Today".to_string()
                        } else {
                            format!(" - {}", d_naive)
                        }
                    }
                    None => String::new(),
                };
                let is_overdue = t.due_date.map(|d| d.date_naive() < today).unwrap_or(false);
                let step_count = all_tasks
                    .iter()
                    .filter(|s| s.parent_task_id == Some(t.id) && !s.completed)
                    .count();

                let (sel_prefix, title_style) = if is_sel {
                    (
                        "> ",
                        Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD),
                    )
                } else {
                    ("  ", Style::default().fg(Color::White))
                };
                let mut spans = vec![
                    Span::styled(sel_prefix, Style::default().fg(accent_color).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("[{}] ", proj_name), Style::default().fg(theme.muted)),
                    Span::styled(&t.title, title_style),
                ];
                if step_count > 0 {
                    spans.push(Span::styled(
                        format!(" ({} steps)", step_count),
                        Style::default().fg(theme.secondary),
                    ));
                }
                spans.push(Span::styled(
                    due_label,
                    Style::default().fg(if is_overdue { theme.danger } else { theme.muted }),
                ));
                ListItem::new(Line::from(spans))
            }
        }).collect()
    };

    let quest_border = if app.dashboard_task_focus {
        Style::default().fg(accent_color)
    } else {
        Style::default().fg(theme.muted)
    };
    let quest_title = if app.dashboard_task_focus {
        format!(" Active Quests ({}) — [Space] Complete | [Enter] Open ", parent_count)
    } else {
        format!(" Active & Upcoming Quests ({}) ", parent_count)
    };

    let mut left_state = ListState::default();
    if app.dashboard_task_focus && !flat.is_empty() {
        left_state.select(Some(sel_idx));
    }
    let left_list = List::new(left_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(quest_border)
                .title(quest_title),
        )
        .highlight_style(Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(left_list, bottom_layout[0], &mut left_state);

    // 4b. Proyectos compartidos e invitaciones — el campo i.7 es el status, ojo con el índice
    let mut middle_lines = vec![Line::from("")];
    let shared_projects: Vec<_> = app.projects.iter().filter(|p| p.is_shared).collect();
    middle_lines.push(Line::from(vec![
        Span::styled("   Shared Projects: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} active", shared_projects.len()),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    for p in shared_projects.iter().take(2) {
        middle_lines.push(Line::from(vec![
            Span::styled("     • ", Style::default().fg(accent_color)),
            Span::styled(&p.name, Style::default().fg(theme.text)),
        ]));
    }

    let pending_invites: Vec<_> = app
        .db
        .get_invitations()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| i.7 == "Pending")
        .collect();
    middle_lines.push(Line::from(""));
    middle_lines.push(Line::from(vec![
        Span::styled("   Pending Invites: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} pending", pending_invites.len()),
            Style::default()
                .fg(if pending_invites.is_empty() {
                    theme.disabled
                } else {
                    theme.warning
                })
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    for i in pending_invites.iter().take(2) {
        middle_lines.push(Line::from(vec![
            Span::styled("     ⚔️ ", Style::default().fg(theme.warning)),
            Span::styled(format!("from {}", i.4), Style::default().fg(theme.text)),
        ]));
    }

    let middle_p = Paragraph::new(middle_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Shared Fellowship "),
    );
    f.render_widget(middle_p, bottom_layout[1]);

    // 4c. Feed del gremio — menciones sin leer y actividad reciente de compañeros
    let mut right_lines = vec![Line::from("")];
    let unread_mentions: Vec<_> = app
        .db
        .get_notifications()
        .unwrap_or_default()
        .into_iter()
        .filter(|n| n.1 == "mention" && !n.5)
        .collect();
    right_lines.push(Line::from(vec![
        Span::styled("   Unread Mentions: ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} mentions", unread_mentions.len()),
            Style::default()
                .fg(if unread_mentions.is_empty() {
                    theme.disabled
                } else {
                    Color::Magenta
                })
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    for m in unread_mentions.iter().take(2) {
        right_lines.push(Line::from(vec![
            Span::styled("     @ ", Style::default().fg(Color::Magenta)),
            Span::styled(
                format!("{}: {}", m.2, m.3),
                Style::default().fg(theme.text),
            ),
        ]));
    }

    let activities = app.db.get_recent_activities(3).unwrap_or_default();
    right_lines.push(Line::from(""));
    right_lines.push(Line::from(vec![Span::styled(
        "   Fellowship Activity: ",
        Style::default().fg(theme.muted),
    )]));
    for act in activities.iter().take(2) {
        right_lines.push(Line::from(vec![
            Span::styled("     • ", Style::default().fg(Color::LightCyan)),
            Span::styled(
                format!("{}: {}", act.5, act.3),
                Style::default().fg(theme.text),
            ),
        ]));
    }

    let right_p = Paragraph::new(right_lines)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Activity & Mentions "),
        );
    f.render_widget(right_p, bottom_layout[2]);

    // ── Modals flotantes — se renderizan encima de todo lo demás ──
    match &app.modal_state {
        // Modal de reflexión diaria — dos campos de texto con Tab para cambiar entre ellos
        ModalType::DailyReflection {
            what_went_well,
            what_can_improve,
            focus_idx,
        } => {
            let area = centered_rect(55, 45, size);
            f.render_widget(Clear, area);
            f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.warning))
                .title(Span::styled(
                    " Daily Reflection Journal ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));

            let focus_well = *focus_idx == 0;
            let focus_improve = *focus_idx == 1;

            let border_well = if focus_well {
                Style::default().fg(accent_color)
            } else {
                Style::default().fg(theme.muted)
            };
            let border_improve = if focus_improve {
                Style::default().fg(accent_color)
            } else {
                Style::default().fg(theme.muted)
            };

            let text_well = format!(" > {}", what_went_well);
            let text_improve = format!(" > {}", what_can_improve);

            let content_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Spacer
                    Constraint::Length(4), // What went well
                    Constraint::Length(4), // What can improve
                    Constraint::Min(2),    // Instruction footer
                ])
                .split(block.inner(area));

            f.render_widget(block, area);

            let well_p = Paragraph::new(text_well).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_well)
                    .title(" 1. What went well today? "),
            );
            f.render_widget(well_p, content_layout[1]);

            let improve_p = Paragraph::new(text_improve).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_improve)
                    .title(" 2. What can be improved? "),
            );
            f.render_widget(improve_p, content_layout[2]);

            let help_text = vec![Line::from(Span::styled(
                " [Tab] switch field  |  [Enter] submit reflection  |  [Esc] cancel ",
                Style::default().fg(theme.muted),
            ))];
            let help_p = Paragraph::new(help_text).alignment(Alignment::Center);
            f.render_widget(help_p, content_layout[3]);
        }
        // Modal para crear un nuevo sidequest — 4 campos, el de frecuencia es un selector con flechas
        ModalType::NewRitual {
            name,
            desc,
            frequency_idx,
            reward_xp,
            focus_idx,
        } => {
            let area = centered_rect(55, 55, size);
            f.render_widget(Clear, area);
            f.render_widget(Block::default().style(Style::default().bg(theme.background)), area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.warning))
                .title(Span::styled(
                    " New Sidequest (Habit) ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));

            let focus_name = *focus_idx == 0;
            let focus_desc = *focus_idx == 1;
            let focus_freq = *focus_idx == 2;
            let focus_xp = *focus_idx == 3;

            let border_name = if focus_name {
                Style::default().fg(accent_color)
            } else {
                Style::default().fg(theme.muted)
            };
            let border_desc = if focus_desc {
                Style::default().fg(accent_color)
            } else {
                Style::default().fg(theme.muted)
            };
            let border_freq = if focus_freq {
                Style::default().fg(accent_color)
            } else {
                Style::default().fg(theme.muted)
            };
            let border_xp = if focus_xp {
                Style::default().fg(accent_color)
            } else {
                Style::default().fg(theme.muted)
            };

            // El selector de frecuencia — flechas izq/der o Space para ciclar entre opciones
            let freqs = ["Daily", "Weekdays", "Weekly", "Monthly", "Custom"];
            let freq_str = format!("<  {}  >", freqs[*frequency_idx]);

            let content_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Spacer
                    Constraint::Length(3), // Name
                    Constraint::Length(3), // Desc
                    Constraint::Length(3), // Freq
                    Constraint::Length(3), // XP
                    Constraint::Min(2),    // Instruction footer
                ])
                .split(block.inner(area));

            f.render_widget(block, area);

            let name_p = Paragraph::new(format!(" > {}", name)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_name)
                    .title(" 1. Sidequest Name "),
            );
            f.render_widget(name_p, content_layout[1]);

            let desc_p = Paragraph::new(format!(" > {}", desc)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_desc)
                    .title(" 2. Description (optional) "),
            );
            f.render_widget(desc_p, content_layout[2]);

            let freq_p = Paragraph::new(freq_str)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_freq)
                        .title(" 3. Frequency "),
                )
                .alignment(Alignment::Center);
            f.render_widget(freq_p, content_layout[3]);

            let xp_p = Paragraph::new(format!(" > {}", reward_xp)).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_xp)
                    .title(" 4. XP Reward "),
            );
            f.render_widget(xp_p, content_layout[4]);

            let help_text = vec![
                Line::from(Span::styled(" [Tab] switch field  |  [<- ->/Space] cycle frequency  |  [Enter] create  |  [Esc] cancel ", Style::default().fg(theme.muted)))
            ];
            let help_p = Paragraph::new(help_text).alignment(Alignment::Center);
            f.render_widget(help_p, content_layout[5]);
        }
        _ => {}
    }
}
