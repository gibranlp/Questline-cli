// ─────────────────────────────────────────────────────────────────────────────
// dashboard.rs — el centro de comando del héroe: campaña de hoy y estado del reino
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::{App, ModalType};
use crate::models::{Achievement, Statistics, Task, TaskPriority, User};
use crate::screens::intro::centered_rect;
use crate::services::bonsai::BonsaiGrid;
use crate::services::planner::{self, format_duration, DashboardPlan, ScoredTask};
use crate::theme::Theme;
use chrono::Timelike;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph},
    Frame,
};

fn greeting(username: &str) -> String {
    let hour = chrono::Local::now().hour();
    let salutation = match hour {
        5..=11 => "morning",
        12..=17 => "afternoon",
        _ => "evening",
    };
    format!("Good {}, {}.", salutation, username)
}

fn priority_label(priority: TaskPriority) -> (&'static str, Color) {
    match priority {
        TaskPriority::High => ("HIGH", Color::Rgb(239, 68, 68)),
        TaskPriority::Medium => ("MED", Color::Rgb(245, 158, 11)),
        TaskPriority::Low => ("LOW", Color::Rgb(107, 114, 128)),
    }
}

fn task_xp(priority: TaskPriority) -> i32 {
    match priority {
        TaskPriority::High => 50,
        _ => 25,
    }
}

fn render_progress_bar(filled: usize, total: usize, width: usize) -> String {
    if total == 0 {
        return "░".repeat(width);
    }
    let filled_count = ((filled as f64 / total as f64) * width as f64).round() as usize;
    let filled_count = filled_count.min(width);
    format!(
        "{}{}",
        "\u{2588}".repeat(filled_count),
        "\u{2591}".repeat(width - filled_count)
    )
}

fn workload_label(minutes: u32) -> (&'static str, Color) {
    match minutes {
        0..=90 => ("Light", Color::Rgb(34, 197, 94)),
        91..=300 => ("Balanced", Color::Rgb(245, 158, 11)),
        301..=480 => ("Heavy", Color::Rgb(249, 115, 22)),
        _ => ("Epic", Color::Rgb(239, 68, 68)),
    }
}

// ─── Columna izquierda: la campaña de hoy ────────────────────────────────────

fn draw_campaign_header(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
    plan: &DashboardPlan,
) {
    let user = app.user.as_ref().unwrap();
    let greeting_str = greeting(&user.username);

    let lines = vec![
        Line::from(vec![
            Span::styled(
                greeting_str,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   \"{}\"", plan.guidance),
                Style::default().fg(theme.muted),
            ),
        ]),
    ];

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Today's Campaign "),
    );
    f.render_widget(p, area);
}

fn draw_main_quest(
    f: &mut Frame,
    theme: &Theme,
    area: ratatui::layout::Rect,
    main: Option<&ScoredTask>,
) {
    let border_color = theme.primary;

    if let Some(sq) = main {
        let (prio_label, prio_color) = priority_label(sq.task.priority);
        let xp = task_xp(sq.task.priority);
        let progress_bar = render_progress_bar(sq.completed_steps, sq.total_steps, 10);
        let step_text = if sq.total_steps > 0 {
            format!(
                "[{}] {}/{} steps",
                progress_bar, sq.completed_steps, sq.total_steps
            )
        } else {
            "[No steps]".to_string()
        };

        let lines = vec![
            Line::from(vec![
                Span::styled(
                    format!("[{}]", prio_label),
                    Style::default().fg(prio_color).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    sq.project_name.as_str(),
                    Style::default().fg(theme.muted),
                ),
                Span::styled(
                    format!("  |  +{} XP  |  {}", xp, format_duration(sq.est_minutes)),
                    Style::default().fg(theme.muted),
                ),
            ]),
            Line::from(vec![Span::styled(
                sq.task.title.as_str(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                step_text,
                Style::default().fg(theme.success),
            )]),
            Line::from(vec![
                Span::styled(sq.reason, Style::default().fg(theme.muted)),
                Span::styled(
                    "   [o] Open in Workspace",
                    Style::default().fg(theme.disabled),
                ),
            ]),
        ];

        let p = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(
                    " Main Quest ",
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                )),
        );
        f.render_widget(p, area);
    } else {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No quest selected for today.",
                Style::default().fg(theme.muted),
            )),
            Line::from(Span::styled(
                "  Add tasks to your projects to begin the adventure.",
                Style::default().fg(theme.disabled),
            )),
        ];
        let p = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Main Quest "),
        );
        f.render_widget(p, area);
    }
}

fn draw_next_quest(
    f: &mut Frame,
    theme: &Theme,
    area: ratatui::layout::Rect,
    next: Option<&ScoredTask>,
) {
    if let Some(sq) = next {
        let (prio_label, prio_color) = priority_label(sq.task.priority);
        let xp = task_xp(sq.task.priority);

        let lines = vec![
            Line::from(vec![Span::styled(
                sq.task.title.as_str(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    format!("[{}]", prio_label),
                    Style::default().fg(prio_color),
                ),
                Span::styled(
                    format!("  {}  |  +{} XP", sq.project_name, xp),
                    Style::default().fg(theme.muted),
                ),
            ]),
            Line::from(vec![Span::styled(
                sq.reason,
                Style::default().fg(theme.secondary),
            )]),
            Line::from(vec![Span::styled(
                format_duration(sq.est_minutes),
                Style::default().fg(theme.muted),
            )]),
        ];

        let p = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.secondary))
                .title(" Next Quest "),
        );
        f.render_widget(p, area);
    } else {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No further quests.",
                Style::default().fg(theme.muted),
            )),
        ];
        let p = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Next Quest "),
        );
        f.render_widget(p, area);
    }
}

fn draw_daily_quests(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
    focused: bool,
) {
    let daily_adventures = app.db.get_daily_adventures().unwrap_or_default();
    let comp_count = daily_adventures.iter().filter(|a| a.completed).count();
    let total = daily_adventures.len();

    let rituals = app.db.get_rituals().unwrap_or_default();
    let completed_rituals = app
        .db
        .get_ritual_history_for_date(chrono::Local::now().date_naive())
        .unwrap_or_default();

    let mut items: Vec<ListItem> = Vec::new();

    for a in &daily_adventures {
        let check = if a.completed { "[x]" } else { "[ ]" };
        let style = if a.completed {
            Style::default().fg(theme.success)
        } else {
            Style::default().fg(theme.text)
        };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("  {} ", check), style),
            Span::styled(
                format!("{} ({}/{})", a.title, a.current_count, a.target_count),
                style,
            ),
        ])));
    }

    if !rituals.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "  ── Sidequests ──────────────────",
            Style::default().fg(theme.border),
        ))));

        for (idx, r) in rituals.iter().enumerate() {
            let is_done = completed_rituals.contains(&r.id);
            let is_sel = idx == app.selected_ritual_idx && !app.dashboard_task_focus;
            let check = if is_done { "[x]" } else { "[ ]" };
            let cursor = if is_sel { "> " } else { "  " };

            let (cursor_style, text_style) = if is_sel {
                (
                    Style::default()
                        .fg(theme.primary)
                        .add_modifier(Modifier::BOLD),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                (
                    Style::default().fg(theme.muted),
                    Style::default().fg(theme.text),
                )
            };

            items.push(ListItem::new(Line::from(vec![
                Span::styled(cursor, cursor_style),
                Span::styled(
                    format!("{} ", check),
                    if is_done {
                        Style::default().fg(theme.success)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(r.name.as_str(), text_style),
                Span::styled(
                    format!(" (+{} XP)", r.reward_xp),
                    Style::default().fg(theme.muted),
                ),
            ])));
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Span::styled(
            "  No daily quests today.",
            Style::default().fg(theme.muted),
        )));
    }

    let border_color = if focused { theme.warning } else { theme.border };
    let hint = if focused { "[Tab] Quick Wins  [n] New  [Del] Remove" } else { "[Tab] to focus" };
    let title = format!(" Daily Quests ({}/{})  {} ", comp_count, total, hint);
    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(title.as_str()),
    );
    f.render_widget(list, area);
}

fn draw_quick_wins(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
    quick_wins: &[Task],
) {
    let focused = app.dashboard_task_focus;
    let border_color = if focused { theme.primary } else { theme.border };

    let items: Vec<ListItem> = if quick_wins.is_empty() {
        vec![ListItem::new(Span::styled(
            "  No quick wins available.",
            Style::default().fg(theme.muted),
        ))]
    } else {
        quick_wins
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let is_sel = focused && i == app.selected_dashboard_task_idx;
                let cursor = if is_sel { "> " } else { "  " };
                let (cursor_style, text_style) = if is_sel {
                    (
                        Style::default()
                            .fg(theme.primary)
                            .add_modifier(Modifier::BOLD),
                        Style::default()
                            .fg(Color::Black)
                            .bg(theme.selection)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    (
                        Style::default().fg(theme.muted),
                        Style::default().fg(theme.text),
                    )
                };
                let (prio_label, prio_color) = priority_label(t.priority);
                ListItem::new(Line::from(vec![
                    Span::styled(cursor, cursor_style),
                    Span::styled(
                        format!("[{}] ", prio_label),
                        Style::default().fg(prio_color),
                    ),
                    Span::styled(t.title.as_str(), text_style),
                ]))
            })
            .collect()
    };

    let hint = if focused {
        "[Space] Done  [Enter] Open  [Tab] Sidequests"
    } else {
        "[Tab] to focus"
    };

    let mut state = ListState::default();
    if focused && !quick_wins.is_empty() {
        state.select(Some(
            app.selected_dashboard_task_idx
                .min(quick_wins.len().saturating_sub(1)),
        ));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(format!(" Quick Wins ({})  {} ", quick_wins.len(), hint)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(theme.selection)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_workload(
    f: &mut Frame,
    theme: &Theme,
    area: ratatui::layout::Rect,
    total_quests: usize,
    estimated_minutes: u32,
) {
    let (label, label_color) = workload_label(estimated_minutes);
    let cap_minutes = 480u32;
    let ratio = (estimated_minutes.min(cap_minutes) as f64 / cap_minutes as f64).clamp(0.0, 1.0);
    let bar_width = area.width.saturating_sub(4) as usize;
    let filled = (ratio * bar_width as f64).round() as usize;
    let bar = format!(
        "[{}{}]",
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(bar_width.saturating_sub(filled))
    );

    let lines = vec![
        Line::from(vec![
            Span::styled(
                format!("  {} quests  |  ", total_quests),
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                label,
                Style::default()
                    .fg(label_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!("  {}", bar),
            Style::default().fg(label_color),
        )),
        Line::from(Span::styled(
            format!("  Est. remaining: {}", format_duration(estimated_minutes)),
            Style::default().fg(theme.muted),
        )),
    ];

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Today's Workload "),
    );
    f.render_widget(p, area);
}

// ─── Columna derecha: héroe y reino ──────────────────────────────────────────

fn draw_hero_panel(
    f: &mut Frame,
    theme: &Theme,
    area: ratatui::layout::Rect,
    user: &User,
) {
    let next_level_xp = User::xp_for_next_level(user.level);
    let ratio = if next_level_xp > 0 {
        (user.xp as f64 / next_level_xp as f64).clamp(0.0, 1.0)
    } else {
        1.0
    };

    // Poder actual desbloqueado y el siguiente objetivo del árbol de progresión
    let powers = user.class.powers();
    let current_power = powers.iter()
        .rev()
        .find(|(lvl, _, _)| *lvl <= user.level)
        .map(|(_, name, _)| *name)
        .unwrap_or("");
    let next_power = powers.iter()
        .find(|(lvl, _, _)| *lvl > user.level)
        .map(|(lvl, name, _)| (*lvl, *name));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(" Adventurer ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let info_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(inner);

    let mut progression_spans = vec![
        Span::styled("→ ", Style::default().fg(theme.muted)),
        Span::styled(current_power, Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
    ];
    if let Some((next_lvl, next_name)) = next_power {
        progression_spans.push(Span::styled(
            format!("  ⟶  {} ({})", next_name, next_lvl),
            Style::default().fg(theme.muted),
        ));
    }

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(&user.username, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("  ", Style::default()),
            Span::styled(user.class.name(), Style::default().fg(theme.primary).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(user.title(), Style::default().fg(theme.warning)),
            Span::styled(
                format!("   Lv {}", user.level),
                Style::default().fg(theme.muted),
            ),
        ]),
        Line::from(progression_spans),
    ]);
    f.render_widget(info, info_rows[0]);

    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(theme.primary).bg(Color::Rgb(30, 30, 30)))
        .label(format!(
            "{} / {} XP  ({:.0}%)",
            user.xp,
            next_level_xp,
            ratio * 100.0
        ))
        .ratio(ratio);
    f.render_widget(gauge, info_rows[1]);
}

fn draw_evergrowth_panel(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
) {
    let zen_tree = app.db.get_zen_tree().unwrap();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.success))
        .title(" Evergrowth  [w] Water ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.height < 4 || inner.width < 6 {
        return;
    }

    // Divide el área en: cabecera de estadísticas (3 filas) + árbol (resto)
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(inner);

    // ── Estadísticas compactas en la cabecera ───────────────────────────────
    let last_watered = match zen_tree.last_watered {
        Some(dt) => dt.with_timezone(&chrono::Local).format("%H:%M").to_string(),
        None => "Never".to_string(),
    };
    let bar = |ratio: f64, width: usize| -> String {
        let filled = (ratio * width as f64).round() as usize;
        format!(
            "{}{}",
            "\u{2588}".repeat(filled),
            "\u{2591}".repeat(width - filled)
        )
    };
    let growth_ratio = ((zen_tree.growth % 100) as f64 / 100.0).clamp(0.0, 1.0);
    let health_ratio = (zen_tree.health as f64 / 100.0).clamp(0.0, 1.0);
    let health_color = if zen_tree.health >= 70 {
        theme.success
    } else if zen_tree.health >= 40 {
        theme.warning
    } else {
        theme.danger
    };

    let stats = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" Stage: ", Style::default().fg(theme.muted)),
            Span::styled(
                zen_tree.stage_name(),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Grw:", Style::default().fg(theme.muted)),
            Span::styled(
                format!("[{}]{}", bar(growth_ratio, 6), zen_tree.growth),
                Style::default().fg(theme.success),
            ),
            Span::styled(" Hp:", Style::default().fg(theme.muted)),
            Span::styled(
                format!("[{}]{}%", bar(health_ratio, 6), zen_tree.health),
                Style::default().fg(health_color),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Water: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}/2 today", zen_tree.water_today),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("  Last: ", Style::default().fg(theme.muted)),
            Span::styled(last_watered, Style::default().fg(theme.text)),
        ]),
    ])
    .alignment(Alignment::Left);
    f.render_widget(stats, sections[0]);

    // ── Estado de animación: crece lentamente de etapa 1 a la actual, luego espera ──
    // 160 ticks/etapa = 8 segundos por transición; 24 000 ticks = 20 minutos en la etapa final
    const STAGE_TICKS: usize = 160;
    const HOLD_TICKS: usize = 24_000;
    let current_stage = zen_tree.stage.max(1) as usize;
    let grow_ticks = current_stage * STAGE_TICKS;
    let cycle_len = grow_ticks + HOLD_TICKS;
    let pos = app.music_scroll_ticks % cycle_len;
    let animated_stage = if pos >= grow_ticks {
        current_stage as i32
    } else {
        (pos / STAGE_TICKS + 1).min(current_stage) as i32
    };

    // ── Árbol procedural — crece desde el fondo del área ────────────────────
    let tree_area = sections[1];
    if tree_area.height > 0 && tree_area.width > 0 {
        let grid = BonsaiGrid::generate(
            tree_area.height as usize,
            tree_area.width as usize,
            zen_tree.growth as u64,
            animated_stage,
            zen_tree.health,
        );
        let tree_para = Paragraph::new(grid.into_lines());
        f.render_widget(tree_para, tree_area);
    }
}

fn word_wrap(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn achievement_progress(
    id: &str,
    stats: &Statistics,
    streak_days: i32,
    zen_stage: i32,
    silent: i32,
    forest: i32,
    rain: i32,
    unique_sc: i32,
    codices: i32,
) -> Option<(i32, i32, &'static str)> {
    match id {
        "first_quest"          => Some((stats.tasks_completed.min(1), 1, "task completed")),
        "scholar"              => Some((stats.notes_created, 25, "notes created")),
        "chronicler"           => Some((stats.journal_entries, 50, "journal entries")),
        "project_master"       => Some((stats.projects_completed, 10, "projects completed")),
        "ancient_gardener"     => Some((zen_stage, 5, "tree stages grown")),
        "hundred_day_journey"  => Some((streak_days, 100, "day streak")),
        "first_focus"          => Some((stats.sessions_completed.min(1), 1, "focus session")),
        "deep_worker"          => Some((stats.sessions_completed, 100, "focus sessions")),
        "master_concentration" => Some((stats.sessions_completed, 500, "focus sessions")),
        "silent_monk"          => Some((silent, 25, "silent sessions")),
        "forest_wanderer"      => Some((forest, 50, "forest sessions")),
        "rain_listener"        => Some((rain, 50, "rain sessions")),
        "master_atmosphere"    => Some((unique_sc, 8, "soundscapes used")),
        "archivist"            => Some((codices, 3, "codices")),
        "grand_archivist"      => Some((codices, 10, "codices")),
        _                      => None,
    }
}

fn draw_streaks_panel(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
) {
    let streak = app.db.get_streak().unwrap();
    let achievements = app.db.get_achievements().unwrap_or_default();
    let unlocked = achievements.iter().filter(|a| a.unlocked_at.is_some()).count();

    let stats = app.db.get_statistics().unwrap();
    let zen_stage = app.db.get_zen_tree().map(|t| t.stage).unwrap_or(0);
    let silent_count = app.db.count_focus_sessions_with_soundscape(&["Silent"]).unwrap_or(0);
    let forest_count = app.db.count_focus_sessions_with_soundscape(&["Forest Sounds"]).unwrap_or(0);
    let rain_count = app.db.count_focus_sessions_with_soundscape(&["Rain Sounds"]).unwrap_or(0);
    let unique_sc = app.db.count_unique_soundscapes_used().unwrap_or(0);
    let codex_count = app.db.count_codices().unwrap_or(0);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(249, 115, 22)))
        .title(format!(" Streaks & Achievements ({}/{})", unlocked, achievements.len()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(2)])
        .split(inner);

    let streak_info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" Current: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} days", streak.current_streak),
                Style::default()
                    .fg(Color::Rgb(249, 115, 22))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Best: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{} days", streak.best_streak),
                Style::default().fg(theme.warning),
            ),
        ]),
    ]);
    f.render_widget(streak_info, rows[0]);

    // 1 most-recently unlocked + 2 closest to completion
    let progress_ratio = |a: &Achievement| -> f64 {
        achievement_progress(
            &a.id, &stats, streak.current_streak, zen_stage,
            silent_count, forest_count, rain_count, unique_sc, codex_count,
        )
        .map(|(cur, tgt, _)| if tgt > 0 { cur as f64 / tgt as f64 } else { 0.0 })
        .unwrap_or(0.0)
    };

    let mut unlocked_sorted: Vec<&Achievement> =
        achievements.iter().filter(|a| a.unlocked_at.is_some()).collect();
    unlocked_sorted.sort_by(|a, b| b.unlocked_at.cmp(&a.unlocked_at));

    let mut locked_sorted: Vec<&Achievement> =
        achievements.iter().filter(|a| a.unlocked_at.is_none()).collect();
    locked_sorted.sort_by(|a, b| {
        progress_ratio(b)
            .partial_cmp(&progress_ratio(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let display: Vec<&Achievement> = unlocked_sorted
        .into_iter()
        .take(1)
        .chain(locked_sorted.into_iter().take(2))
        .collect();

    // "      " prefix = 6 chars, 2 for border
    let desc_width = area.width.saturating_sub(8) as usize;

    let make_desc_items = |text: &str, color: Color| -> Vec<ListItem<'static>> {
        word_wrap(text, desc_width)
            .into_iter()
            .map(|line| {
                ListItem::new(Line::from(vec![
                    Span::raw("      "),
                    Span::styled(line, Style::default().fg(color)),
                ]))
            })
            .collect()
    };

    let ach_items: Vec<ListItem> = if achievements.is_empty() {
        vec![ListItem::new(Span::styled(
            " No achievements recorded.",
            Style::default().fg(theme.muted),
        ))]
    } else {
        display
            .iter()
            .flat_map(|a| {
                if a.unlocked_at.is_some() {
                    let mut items = vec![ListItem::new(Line::from(vec![
                        Span::styled(" [+] ", Style::default().fg(theme.success)),
                        Span::styled(a.name.clone(), Style::default().fg(theme.text).add_modifier(Modifier::BOLD)),
                    ]))];
                    items.extend(make_desc_items(&a.description, theme.success));
                    items
                } else {
                    let desc = achievement_progress(
                        &a.id, &stats, streak.current_streak, zen_stage,
                        silent_count, forest_count, rain_count, unique_sc, codex_count,
                    )
                    .map(|(cur, tgt, unit)| format!("{} / {} {}", cur, tgt, unit))
                    .unwrap_or_else(|| a.description.clone());
                    let mut items = vec![ListItem::new(Line::from(vec![
                        Span::styled(" [ ] ", Style::default().fg(theme.disabled)),
                        Span::styled(a.name.clone(), Style::default().fg(theme.muted).add_modifier(Modifier::BOLD)),
                    ]))];
                    items.extend(make_desc_items(&desc, theme.disabled));
                    items
                }
            })
            .collect()
    };
    f.render_widget(List::new(ach_items), rows[1]);
}

fn draw_focus_panel(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
) {
    let stats = app.db.get_statistics().unwrap();
    let fav = app
        .db
        .get_favorite_soundscape()
        .unwrap_or_else(|_| "None".to_string());

    let lines = vec![
        Line::from(vec![
            Span::styled(" Sessions: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", stats.sessions_completed),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   Hours: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{:.1} hr", stats.focus_hours),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" Favorite: ", Style::default().fg(theme.muted)),
            Span::styled(fav, Style::default().fg(theme.warning)),
        ]),
    ];

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.focus_timer))
            .title(" Deep Work "),
    );
    f.render_widget(p, area);
}

fn draw_reflection_panel(
    f: &mut Frame,
    theme: &Theme,
    area: ratatui::layout::Rect,
    reflected_today: bool,
) {
    let (text, border_color) = if reflected_today {
        (
            Line::from(Span::styled(
                "  Reflection recorded today.",
                Style::default().fg(theme.success),
            )),
            theme.success,
        )
    } else {
        (
            Line::from(vec![
                Span::styled(
                    "  [r] Record today's reflection",
                    Style::default().fg(theme.warning).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  +25 XP", Style::default().fg(theme.muted)),
            ]),
            theme.warning,
        )
    };

    let p = Paragraph::new(vec![text]).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(" Reflection "),
    );
    f.render_widget(p, area);
}

fn draw_fellowship_panel(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
) {
    let shared = app.projects.iter().filter(|p| p.is_shared).count();
    let pending = app
        .db
        .get_invitations()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| i.7 == "Pending")
        .count();

    let my_name = app.user.as_ref().map(|u| u.username.clone()).unwrap_or_default();
    let my_identity = app.identity.public_key.clone();
    let last_viewed = app.db.get_setting("last_viewed_fellowship").unwrap_or(None)
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    // Count unread messages and mentions from chronicle_messages
    let mut unread_count = 0;
    let mut mentions = 0;
    if let Ok(mut stmt) = app.db.conn.prepare("SELECT content, sender_identity FROM chronicle_messages WHERE timestamp > ?1 AND sender_identity != ?2") {
        if let Ok(mut rows) = stmt.query(rusqlite::params![last_viewed, my_identity]) {
            while let Ok(Some(row)) = rows.next() {
                let content: String = row.get(0).unwrap_or_default();
                unread_count += 1;
                if !my_name.is_empty() && content.to_lowercase().contains(&format!("@{}", my_name.to_lowercase())) {
                    mentions += 1;
                }
            }
        }
    }

    let border_color = if mentions > 0 {
        Color::Magenta
    } else if unread_count > 0 {
        Color::Cyan
    } else if pending > 0 {
        theme.warning
    } else {
        theme.border
    };

    let title = if mentions > 0 {
        format!(" Fellowship [Mentions: {}] 🔔 ", mentions)
    } else if unread_count > 0 {
        format!(" Fellowship [Unread: {}] ✉ ", unread_count)
    } else {
        " Fellowship ".to_string()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(" Shared: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", shared),
                Style::default().fg(Color::White),
            ),
            Span::styled("   Invites: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", pending),
                Style::default().fg(if pending > 0 { theme.warning } else { theme.disabled }),
            ),
            Span::styled("   Unread: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", unread_count),
                Style::default().fg(if unread_count > 0 { Color::Cyan } else { theme.disabled }),
            ),
            Span::styled("   Mentions: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", mentions),
                Style::default().fg(if mentions > 0 { Color::Magenta } else { theme.disabled }),
            ),
        ]),
    ];

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(title),
    );
    f.render_widget(p, area);
}

// ─── Función principal de renderizado ────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let user = app.user.as_ref().unwrap();
    let today = chrono::Local::now().date_naive();
    let all_tasks = &app.all_tasks;

    // Datos para el motor de planificación
    let daily_adventures = app.db.get_daily_adventures().unwrap_or_default();
    let streak = app.db.get_streak().unwrap();
    let zen_tree = app.db.get_zen_tree().unwrap();
    let overdue_count = all_tasks
        .iter()
        .filter(|t| {
            !t.completed
                && t.parent_task_id.is_none()
                && t.due_date
                    .map(|d| d.date_naive() < today)
                    .unwrap_or(false)
        })
        .count();
    let daily_completed = daily_adventures.iter().filter(|a| a.completed).count();
    let daily_total = daily_adventures.len();

    let plan = planner::generate_plan(
        all_tasks,
        &app.projects,
        today,
        overdue_count,
        streak.current_streak,
        zen_tree.health,
        daily_completed,
        daily_total,
    );

    let reflected_today = app
        .db
        .get_reflection_for_date(today)
        .unwrap_or(None)
        .is_some();

    // División principal: izquierda (30% héroe/reino) y derecha (70% campaña)
    let main_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    // ── Columna izquierda — árbol y logros ──────────────────────────────────
    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // héroe
            Constraint::Min(14),    // evergrowth — más espacio al quitar los paneles de abajo
            Constraint::Length(13), // rachas y logros
        ])
        .split(main_cols[0]);

    draw_hero_panel(f, theme, left_rows[0], user);
    draw_evergrowth_panel(f, app, theme, left_rows[1]);
    draw_streaks_panel(f, app, theme, left_rows[2]);

    // ── Columna derecha — campaña de hoy ────────────────────────────────────
    let right_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // encabezado de campaña
            Constraint::Length(7),  // quest principal
            Constraint::Min(8),     // siguiente quest + quests diarias
            Constraint::Length(9),  // victorias rápidas + carga de trabajo
            Constraint::Length(4),  // trabajo profundo + reflexión + compañerismo
        ])
        .split(main_cols[1]);

    draw_campaign_header(f, app, theme, right_rows[0], &plan);
    draw_main_quest(f, theme, right_rows[1], plan.main_quest.as_ref());

    // Fila de siguiente quest y quests diarias
    let mid_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(right_rows[2]);
    draw_next_quest(f, theme, mid_row[0], plan.next_quest.as_ref());
    draw_daily_quests(f, app, theme, mid_row[1], !app.dashboard_task_focus);

    // Fila de victorias rápidas y carga de trabajo
    let bottom_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(right_rows[3]);
    draw_quick_wins(f, app, theme, bottom_row[0], &plan.quick_wins);
    draw_workload(f, theme, bottom_row[1], plan.total_quest_count, plan.estimated_minutes);

    // Fila de trabajo profundo, reflexión y compañerismo
    let stats_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38), // trabajo profundo
            Constraint::Percentage(32), // reflexión
            Constraint::Percentage(30), // compañerismo
        ])
        .split(right_rows[4]);
    draw_focus_panel(f, app, theme, stats_row[0]);
    draw_reflection_panel(f, theme, stats_row[1], reflected_today);
    draw_fellowship_panel(f, app, theme, stats_row[2]);

    // ── Modales flotantes ────────────────────────────────────────────────────
    match &app.modal_state {
        ModalType::DailyReflection {
            what_went_well,
            what_can_improve,
            focus_idx,
        } => {
            let modal_area = centered_rect(55, 45, area);
            f.render_widget(Clear, modal_area);
            f.render_widget(
                Block::default().style(Style::default().bg(theme.background)),
                modal_area,
            );
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.warning))
                .title(Span::styled(
                    " Daily Reflection Journal ",
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ));
            let content = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(4),
                    Constraint::Length(4),
                    Constraint::Min(2),
                ])
                .split(block.inner(modal_area));
            f.render_widget(block, modal_area);

            let border_well = if *focus_idx == 0 {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.muted)
            };
            let border_improve = if *focus_idx == 1 {
                Style::default().fg(theme.primary)
            } else {
                Style::default().fg(theme.muted)
            };

            f.render_widget(
                Paragraph::new(format!(" > {}", what_went_well)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_well)
                        .title(" 1. What went well today? "),
                ),
                content[1],
            );
            f.render_widget(
                Paragraph::new(format!(" > {}", what_can_improve)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border_improve)
                        .title(" 2. What can be improved? "),
                ),
                content[2],
            );
            f.render_widget(
                Paragraph::new(Span::styled(
                    " [Tab] switch  |  [Enter] submit  |  [Esc] cancel ",
                    Style::default().fg(theme.muted),
                ))
                .alignment(Alignment::Center),
                content[3],
            );
        }
        ModalType::NewRitual {
            name,
            desc,
            frequency_idx,
            reward_xp,
            focus_idx,
        } => {
            let modal_area = centered_rect(55, 55, area);
            f.render_widget(Clear, modal_area);
            f.render_widget(
                Block::default().style(Style::default().bg(theme.background)),
                modal_area,
            );
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.warning))
                .title(Span::styled(
                    " New Sidequest (Habit) ",
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ));
            let content = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(2),
                ])
                .split(block.inner(modal_area));
            f.render_widget(block, modal_area);

            let border = |idx: usize| {
                if *focus_idx == idx {
                    Style::default().fg(theme.primary)
                } else {
                    Style::default().fg(theme.muted)
                }
            };
            let freqs = ["Daily", "Weekdays", "Weekly", "Monthly", "Custom"];
            let freq_str = format!("<  {}  >", freqs[*frequency_idx]);

            f.render_widget(
                Paragraph::new(format!(" > {}", name)).block(
                    Block::default().borders(Borders::ALL).border_style(border(0)).title(" 1. Name "),
                ),
                content[1],
            );
            f.render_widget(
                Paragraph::new(format!(" > {}", desc)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(1))
                        .title(" 2. Description (optional) "),
                ),
                content[2],
            );
            f.render_widget(
                Paragraph::new(freq_str)
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border(2))
                            .title(" 3. Frequency "),
                    ),
                content[3],
            );
            f.render_widget(
                Paragraph::new(format!(" > {}", reward_xp)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(3))
                        .title(" 4. XP Reward "),
                ),
                content[4],
            );
            f.render_widget(
                Paragraph::new(Span::styled(
                    " [Tab] switch  |  [<->] frequency  |  [Enter] create  |  [Esc] cancel ",
                    Style::default().fg(theme.muted),
                ))
                .alignment(Alignment::Center),
                content[5],
            );
        }
        _ => {}
    }
}
