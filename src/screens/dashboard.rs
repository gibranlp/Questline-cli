// ─────────────────────────────────────────────────────────────────────────────
// dashboard.rs — el centro de comando del héroe: campaña de hoy y estado del reino
// ─────────────────────────────────────────────────────────────────────────────

use crate::app::{App, ModalType};
use crate::models::{Achievement, Statistics, Task, TaskPriority, User};
use crate::screens::intro::centered_rect;
use crate::services::bonsai::BonsaiGrid;
use crate::services::planner::{self, DashboardPlan, format_duration};
use crate::theme::Theme;
use chrono::{Local, Timelike};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph},
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

fn sidequest_rank(streak: i32) -> Option<(&'static str, Color)> {
    match streak {
        s if s >= 90 => Some(("Ascendant Oath", Color::Yellow)),
        s if s >= 60 => Some(("Warlord Oath", Color::Rgb(245, 158, 11))),
        s if s >= 30 => Some(("Champion Oath", Color::Rgb(250, 204, 21))),
        s if s >= 15 => Some(("Devoted Oath", Color::Cyan)),
        s if s >= 7 => Some(("Seeker Oath", Color::Rgb(34, 197, 94))),
        s if s >= 3 => Some(("Initiate Oath", Color::Rgb(96, 165, 250))),
        _ => None,
    }
}

fn task_energy_tag(task: &Task) -> (&'static str, Color) {
    let title = task.title.to_lowercase();
    let desc = task.description.as_deref().unwrap_or("").to_lowercase();
    let text = format!("{} {}", title, desc);
    if text.contains("write")
        || text.contains("design")
        || text.contains("draft")
        || text.contains("create")
    {
        ("Creative", Color::Rgb(168, 85, 247))
    } else if text.contains("email")
        || text.contains("call")
        || text.contains("invoice")
        || text.contains("admin")
        || text.contains("reply")
    {
        ("Admin", Color::Rgb(96, 165, 250))
    } else if task.priority == TaskPriority::High {
        ("Deep Work", Color::Rgb(239, 68, 68))
    } else {
        ("Quick Win", Color::Rgb(34, 197, 94))
    }
}

fn short_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let short: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", short)
    } else {
        short
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
    let guidance = format!("   \"{}\"", plan.guidance);
    let local_time = format!(" {}", Local::now().format("%H:%M:%S"));
    let inner_width = area.width.saturating_sub(2) as usize;
    let used_width = greeting_str.chars().count() + guidance.chars().count() + local_time.len();
    let gap = if inner_width > used_width {
        " ".repeat(inner_width - used_width)
    } else {
        " ".to_string()
    };

    let lines = vec![Line::from(vec![
        Span::styled(
            greeting_str,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(guidance, Style::default().fg(theme.muted)),
        Span::raw(gap),
        Span::styled(
            local_time,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Today's Campaign "),
    );
    f.render_widget(p, area);
}

fn draw_today_command_center(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
    all_tasks: &[Task],
    today: chrono::NaiveDate,
    plan: &DashboardPlan,
) {
    let (_, label_color) = workload_label(plan.estimated_minutes);
    let overdue = all_tasks
        .iter()
        .filter(|t| {
            !t.completed
                && t.parent_task_id.is_none()
                && t.due_date
                    .map(|d| d.with_timezone(&Local).date_naive() < today)
                    .unwrap_or(false)
        })
        .count();
    let due_today = all_tasks
        .iter()
        .filter(|t| {
            !t.completed
                && t.parent_task_id.is_none()
                && t.due_date
                    .map(|d| d.with_timezone(&Local).date_naive() == today)
                    .unwrap_or(false)
        })
        .count();
    let high = all_tasks
        .iter()
        .filter(|t| !t.completed && t.parent_task_id.is_none() && t.priority == TaskPriority::High)
        .count();
    let mut rows: Vec<ListItem> = Vec::new();
    let mut selected_visual_idx = None;
    let mut action_idx = 0usize;
    let push_separator = |rows: &mut Vec<ListItem>, label: &'static str, color: Color| {
        if !rows.is_empty() {
            rows.push(ListItem::new(Line::from("")));
            rows.push(ListItem::new(Line::from(Span::styled(
                format!("  -- {} --", label),
                Style::default().fg(color),
            ))));
        }
    };

    if let Some(main) = plan.main_quest.as_ref() {
        let (prio_label, prio_color) = priority_label(main.task.priority);
        if action_idx == app.selected_dashboard_task_idx {
            selected_visual_idx = Some(rows.len());
        }
        action_idx += 1;
        rows.push(ListItem::new(Line::from(vec![
            Span::styled("MAIN  ", Style::default().fg(theme.warning)),
            Span::styled(
                format!("{} ", short_text(&main.project_name, 14)),
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("[{}] ", prio_label),
                Style::default().fg(prio_color),
            ),
            Span::styled(
                main.task.title.as_str(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}", format_duration(main.est_minutes)),
                Style::default().fg(theme.muted),
            ),
        ])));
    }

    push_separator(&mut rows, "Quick Wins", theme.primary);
    for task in &plan.quick_wins {
        let (prio_label, prio_color) = priority_label(task.priority);
        let project_name = app
            .projects
            .iter()
            .find(|p| Some(p.id) == task.project_id)
            .map(|p| p.name.as_str())
            .unwrap_or("General");
        let (energy_label, energy_color) = task_energy_tag(task);
        if action_idx == app.selected_dashboard_task_idx {
            selected_visual_idx = Some(rows.len());
        }
        action_idx += 1;
        rows.push(ListItem::new(Line::from(vec![
            Span::styled("[ ] ", Style::default().fg(theme.text)),
            Span::styled(
                format!("{} ", short_text(project_name, 14)),
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("[{}] ", prio_label),
                Style::default().fg(prio_color),
            ),
            Span::styled(task.title.as_str(), Style::default().fg(theme.text)),
            Span::styled(
                format!(" [{}]", energy_label),
                Style::default().fg(energy_color),
            ),
        ])));
    }

    push_separator(&mut rows, "Sidequests", theme.secondary);
    for ritual in &app.stats_cache.rituals {
        let (count, target) = app
            .stats_cache
            .ritual_day_counts
            .get(&ritual.id)
            .copied()
            .unwrap_or((0, ritual.daily_target));
        let done = count >= target;
        let streak = *app.stats_cache.ritual_streaks.get(&ritual.id).unwrap_or(&0);
        let rank = sidequest_rank(streak);
        if action_idx == app.selected_dashboard_task_idx {
            selected_visual_idx = Some(rows.len());
        }
        action_idx += 1;
        let mut spans = vec![
            Span::styled(
                if done {
                    "[x] "
                } else if count > 0 {
                    "[~] "
                } else {
                    "[ ] "
                },
                Style::default().fg(if done { theme.success } else { theme.text }),
            ),
            Span::styled(ritual.name.as_str(), Style::default().fg(theme.text)),
            Span::styled(
                format!(" ({}/{}) +{} XP ", count, target, ritual.reward_xp),
                Style::default().fg(theme.muted),
            ),
            Span::styled(
                format!("{}d", streak),
                Style::default().fg(rank.map_or(theme.muted, |(_, color)| color)),
            ),
        ];
        if let Some((rank_name, rank_color)) = rank {
            spans.push(Span::styled(
                format!(" {}", rank_name),
                Style::default().fg(rank_color),
            ));
        }
        rows.push(ListItem::new(Line::from(spans)));
    }

    push_separator(&mut rows, "Daily", theme.warning);
    for adventure in &app.stats_cache.todays_daily_adventures {
        if action_idx == app.selected_dashboard_task_idx {
            selected_visual_idx = Some(rows.len());
        }
        action_idx += 1;
        rows.push(ListItem::new(Line::from(vec![
            Span::styled("DAILY ", Style::default().fg(theme.warning)),
            Span::styled(
                if adventure.completed { "[x] " } else { "[ ] " },
                Style::default().fg(if adventure.completed {
                    theme.success
                } else {
                    theme.text
                }),
            ),
            Span::styled(adventure.title.as_str(), Style::default().fg(theme.text)),
            Span::styled(
                format!(" ({}/{})", adventure.current_count, adventure.target_count),
                Style::default().fg(theme.muted),
            ),
        ])));
    }

    if rows.is_empty() {
        rows.push(ListItem::new(Span::styled(
            "  The command board is clear.",
            Style::default().fg(theme.muted),
        )));
    }

    let mut state = ListState::default();
    if !rows.is_empty() {
        state.select(Some(
            selected_visual_idx
                .unwrap_or(0)
                .min(rows.len().saturating_sub(1)),
        ));
    }

    let title = format!(
        "Command Center  Due {} Overdue {} High {} L: {}  Space Done | Enter Open | z Tomorrow | Z Week ",
        due_today,
        overdue,
        high,
        format_duration(plan.estimated_minutes)
    );
    let list = List::new(rows)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(label_color))
                .title(title.as_str()),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(theme.selection)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_campaign_intel(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: ratatui::layout::Rect,
    all_tasks: &[Task],
    today: chrono::NaiveDate,
) {
    let mut lines = Vec::new();

    lines.push(Line::from(Span::styled(
        "Upcoming Threats",
        Style::default()
            .fg(theme.warning)
            .add_modifier(Modifier::BOLD),
    )));
    let mut threats: Vec<&Task> = all_tasks
        .iter()
        .filter(|t| !t.completed && t.parent_task_id.is_none() && t.due_date.is_some())
        .collect();
    threats.sort_by(|a, b| a.due_date.cmp(&b.due_date));
    if threats.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No dated threats.",
            Style::default().fg(theme.muted),
        )));
    } else {
        for task in threats.into_iter().take(3) {
            let due = task
                .due_date
                .map(|d| d.with_timezone(&Local).date_naive())
                .unwrap_or(today);
            let label = if due < today {
                format!("{}d late", (today - due).num_days())
            } else if due == today {
                "today".to_string()
            } else if due == today + chrono::Duration::days(1) {
                "tomorrow".to_string()
            } else {
                format!("{}d", (due - today).num_days())
            };
            let days_until = (due - today).num_days();
            let color = if due <= today + chrono::Duration::days(1) {
                theme.danger
            } else if days_until <= 3 {
                theme.warning
            } else {
                theme.success
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:>8} ", label), Style::default().fg(color)),
                Span::styled(short_text(&task.title, 24), Style::default().fg(theme.text)),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Active Milestones",
        Style::default()
            .fg(theme.secondary)
            .add_modifier(Modifier::BOLD),
    )));
    let milestones = &app.stats_cache.active_milestones;
    if milestones.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No active milestones.",
            Style::default().fg(theme.muted),
        )));
    } else {
        for intel in milestones.iter().take(2) {
            let milestone = &intel.milestone;
            let tier = match milestone.tier {
                3 => "LEG",
                2 => "VET",
                1 => "INI",
                _ => "ARC",
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  [{}] ", tier),
                    Style::default().fg(theme.secondary),
                ),
                Span::styled(
                    short_text(&milestone.name, 24),
                    Style::default().fg(theme.text),
                ),
                Span::styled(
                    format!(
                        " +{} {}",
                        milestone.xp_reward,
                        short_text(&intel.project_name, 12)
                    ),
                    Style::default().fg(theme.muted),
                ),
            ]));
            for requirement in &intel.progress {
                let marker = if requirement.met { "[x]" } else { "[ ]" };
                let color = if requirement.met {
                    theme.success
                } else {
                    theme.warning
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("    {} ", marker), Style::default().fg(color)),
                    Span::styled(
                        short_text(&requirement.label, 30),
                        Style::default().fg(theme.muted),
                    ),
                    Span::styled(
                        format!(" {}/{}", requirement.current, requirement.target),
                        Style::default().fg(color),
                    ),
                ]));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Recent Scrolls",
        Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
    )));
    let mut notes = app.all_notes.clone();
    notes.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    if notes.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No scrolls yet.",
            Style::default().fg(theme.muted),
        )));
    } else {
        for note in notes.iter().take(3) {
            let project_name = app
                .projects
                .iter()
                .find(|p| Some(p.id) == note.project_id)
                .map(|p| p.name.as_str())
                .unwrap_or("General");
            lines.push(Line::from(vec![
                Span::styled(
                    format!(
                        "  {} ",
                        note.updated_at.with_timezone(&Local).format("%m-%d")
                    ),
                    Style::default().fg(theme.muted),
                ),
                Span::styled(short_text(&note.title, 20), Style::default().fg(theme.text)),
                Span::styled(
                    format!(" ({})", short_text(project_name, 12)),
                    Style::default().fg(theme.muted),
                ),
            ]));
        }
    }

    let panel = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Campaign Intel "),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(panel, area);
}

// ─── Columna derecha: héroe y reino ──────────────────────────────────────────

fn draw_hero_panel(f: &mut Frame, theme: &Theme, area: ratatui::layout::Rect, user: &User) {
    let next_level_xp = User::xp_for_next_level(user.level);
    let ratio = if next_level_xp > 0 {
        (user.xp as f64 / next_level_xp as f64).clamp(0.0, 1.0)
    } else {
        1.0
    };

    // Poder actual desbloqueado y el siguiente objetivo del árbol de progresión
    let powers = user.class.powers();
    let current_power = powers
        .iter()
        .rev()
        .find(|(lvl, _, _)| *lvl <= user.level)
        .map(|(_, name, _)| *name)
        .unwrap_or("");
    let next_power = powers
        .iter()
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
        Span::styled(
            current_power,
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some((next_lvl, next_name)) = next_power {
        progression_spans.push(Span::styled(
            format!("  ⟶  {} ({})", next_name, next_lvl),
            Style::default().fg(theme.muted),
        ));
    }

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                &user.username,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(
                user.class.name(),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
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
        .gauge_style(
            Style::default()
                .fg(theme.primary)
                .bg(Color::Rgb(30, 30, 30)),
        )
        .label(format!(
            "{} / {} XP  ({:.0}%)",
            user.xp,
            next_level_xp,
            ratio * 100.0
        ))
        .ratio(ratio);
    f.render_widget(gauge, info_rows[1]);
}

fn draw_evergrowth_panel(f: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let zen_tree = &app.stats_cache.zen_tree;

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
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
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
        "first_quest" => Some((stats.tasks_completed.min(1), 1, "task completed")),
        "scholar" => Some((stats.notes_created, 25, "notes created")),
        "chronicler" => Some((stats.journal_entries, 50, "journal entries")),
        "project_master" => Some((stats.projects_completed, 10, "projects completed")),
        "ancient_gardener" => Some((zen_stage, 5, "tree stages grown")),
        "hundred_day_journey" => Some((streak_days, 100, "day streak")),
        "first_focus" => Some((stats.sessions_completed.min(1), 1, "focus session")),
        "deep_worker" => Some((stats.sessions_completed, 100, "focus sessions")),
        "master_concentration" => Some((stats.sessions_completed, 500, "focus sessions")),
        "silent_monk" => Some((silent, 25, "silent sessions")),
        "forest_wanderer" => Some((forest, 50, "forest sessions")),
        "rain_listener" => Some((rain, 50, "rain sessions")),
        "master_atmosphere" => Some((unique_sc, 8, "soundscapes used")),
        "archivist" => Some((codices, 3, "codices")),
        "grand_archivist" => Some((codices, 10, "codices")),
        _ => None,
    }
}

fn achievement_detail(id: &str, app: &App, max_width: usize) -> Option<String> {
    match id {
        "master_atmosphere" | "atmosphere_collector" | "atmosphere_explorer" => {
            let used = app.db.get_unique_soundscapes_used().unwrap_or_default();
            let missing: Vec<&str> = crate::audio::soundscapes::ATMOSPHERE_ACHIEVEMENT_SOUNDSCAPES
                .iter()
                .copied()
                .filter(|name| !used.iter().any(|u| u == name))
                .collect();
            if missing.is_empty() {
                None
            } else {
                let list = missing.join(", ");
                Some(format!(
                    "Missing: {}",
                    short_text(&list, max_width.saturating_sub(9))
                ))
            }
        }
        _ => None,
    }
}

fn draw_streaks_panel(f: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let streak = &app.stats_cache.streak;
    let achievements = &app.stats_cache.achievements;
    let unlocked = achievements
        .iter()
        .filter(|a| a.unlocked_at.is_some())
        .count();

    let stats = &app.stats_cache.statistics;
    let zen_stage = app.stats_cache.zen_tree.stage;
    let silent_count = app.stats_cache.silent_sessions;
    let forest_count = app.stats_cache.forest_sessions;
    let rain_count = app.stats_cache.rain_sessions;
    let unique_sc = app.stats_cache.unique_soundscapes;
    let codex_count = app.db.count_codices().unwrap_or(0);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Rgb(249, 115, 22)))
        .title(format!(
            " Streaks & Achievements ({}/{})",
            unlocked,
            achievements.len()
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(2)])
        .split(inner);

    let streak_info = Paragraph::new(vec![Line::from(vec![
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
    ])]);
    f.render_widget(streak_info, rows[0]);

    // 1 most-recently unlocked + 2 closest to completion
    let progress_ratio = |a: &Achievement| -> f64 {
        achievement_progress(
            &a.id,
            &stats,
            streak.current_streak,
            zen_stage,
            silent_count,
            forest_count,
            rain_count,
            unique_sc,
            codex_count,
        )
        .map(|(cur, tgt, _)| {
            if tgt > 0 {
                cur as f64 / tgt as f64
            } else {
                0.0
            }
        })
        .unwrap_or(0.0)
    };

    let mut unlocked_sorted: Vec<&Achievement> = achievements
        .iter()
        .filter(|a| a.unlocked_at.is_some())
        .collect();
    unlocked_sorted.sort_by(|a, b| b.unlocked_at.cmp(&a.unlocked_at));

    let mut locked_sorted: Vec<&Achievement> = achievements
        .iter()
        .filter(|a| a.unlocked_at.is_none())
        .collect();
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
                        Span::styled(
                            a.name.clone(),
                            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
                        ),
                    ]))];
                    items.extend(make_desc_items(&a.description, theme.success));
                    items
                } else {
                    let desc = achievement_progress(
                        &a.id,
                        &stats,
                        streak.current_streak,
                        zen_stage,
                        silent_count,
                        forest_count,
                        rain_count,
                        unique_sc,
                        codex_count,
                    )
                    .map(|(cur, tgt, unit)| format!("{} / {} {}", cur, tgt, unit))
                    .unwrap_or_else(|| a.description.clone());
                    let mut items = vec![ListItem::new(Line::from(vec![
                        Span::styled(" [ ] ", Style::default().fg(theme.disabled)),
                        Span::styled(
                            a.name.clone(),
                            Style::default()
                                .fg(theme.muted)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]))];
                    items.extend(make_desc_items(&desc, theme.disabled));
                    if let Some(detail) = achievement_detail(&a.id, app, desc_width) {
                        items.extend(make_desc_items(&detail, theme.warning));
                    }
                    items
                }
            })
            .collect()
    };
    f.render_widget(List::new(ach_items), rows[1]);
}

fn draw_focus_panel(f: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let stats = &app.stats_cache.statistics;
    let fav = &app.stats_cache.favorite_soundscape;

    let lines = vec![
        Line::from(vec![
            Span::styled(" Sessions: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", stats.sessions_completed),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
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
                    Style::default()
                        .fg(theme.warning)
                        .add_modifier(Modifier::BOLD),
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

fn draw_fellowship_panel(f: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let shared = app.projects.iter().filter(|p| p.is_shared).count();
    let pending = app
        .db
        .get_invitations()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| i.7 == "Pending")
        .count();

    let my_name = app
        .user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_default();
    let my_identity = app.identity.public_key.clone();
    let last_viewed = app
        .db
        .get_setting("last_viewed_fellowship")
        .unwrap_or(None)
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());

    // Chronicle chat predates Council Notices, so combine both sources. Quest
    // Council mentions are identity-backed notices rather than Chronicle text.
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
    if let Ok((notice_unread, council_mentions)) = app.db.conn.query_row(
        "SELECT
             SUM(CASE WHEN read = 0 AND notification_type != 'chronicle_mention' THEN 1 ELSE 0 END),
             SUM(CASE WHEN read = 0 AND notification_type = 'mention' THEN 1 ELSE 0 END)
         FROM notifications",
        [],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or(0),
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
            ))
        },
    ) {
        unread_count += notice_unread as usize;
        mentions += council_mentions as usize;
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

    let title = format!(
        " Fellowship [Unread: {} · Mentions: {}]{} ",
        unread_count,
        mentions,
        if mentions > 0 {
            " 🔔"
        } else if unread_count > 0 {
            " ✉"
        } else {
            ""
        }
    );

    let lines = vec![
        Line::from(vec![
            Span::styled(" Shared: ", Style::default().fg(theme.muted)),
            Span::styled(format!("{}", shared), Style::default().fg(Color::White)),
            Span::styled("   Invites: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", pending),
                Style::default().fg(if pending > 0 {
                    theme.warning
                } else {
                    theme.disabled
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("   Unread: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", unread_count),
                Style::default().fg(if unread_count > 0 {
                    Color::Cyan
                } else {
                    theme.disabled
                }),
            ),
            Span::styled("   Mentions: ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}", mentions),
                Style::default().fg(if mentions > 0 {
                    Color::Magenta
                } else {
                    theme.disabled
                }),
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

// ─── Hydration widget ─────────────────────────────────────────────────────────

fn draw_hydration_widget(f: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    if !app.hydration_enabled {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "The Well awaits,",
                Style::default().fg(theme.muted),
            )),
            Line::from(Span::styled(
                "Adventurer.",
                Style::default().fg(theme.muted),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("[h]", Style::default().fg(theme.primary)),
                Span::styled(" Awaken it", Style::default().fg(theme.muted)),
            ]),
        ];
        let p = Paragraph::new(lines).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.muted))
                .title(Span::styled(
                    " Hydration ",
                    Style::default().fg(theme.muted),
                )),
        );
        f.render_widget(p, area);
        return;
    }

    let glasses = app.hydration_glasses;
    let target = app.hydration_target;
    let bar_width = (area.width as usize).saturating_sub(4).min(20);
    let filled = if target > 0 {
        (glasses * bar_width as i32 / target).min(bar_width as i32) as usize
    } else {
        0
    };
    let progress = render_progress_bar(filled, bar_width, bar_width);

    let next_str = if let Some(at) = app.hydration_next_reminder_at {
        let secs = at
            .saturating_duration_since(std::time::Instant::now())
            .as_secs();
        if secs == 0 {
            "Now".to_string()
        } else {
            format!("{}m {:02}s", secs / 60, secs % 60)
        }
    } else if !app.hydration_is_active_at_hour(chrono::Local::now().hour()) {
        format!("Paused to {:02}:00", app.hydration_active_from)
    } else {
        "Arming...".to_string()
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("Next  ", Style::default().fg(theme.muted)),
            Span::styled(
                next_str,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Today ", Style::default().fg(theme.muted)),
            Span::styled(
                format!("{}/{}", glasses, target),
                Style::default()
                    .fg(if glasses >= target {
                        theme.success
                    } else {
                        Color::White
                    })
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            format!("[{}]", progress),
            Style::default().fg(if glasses >= target {
                theme.success
            } else {
                theme.primary
            }),
        )]),
        Line::from(vec![
            Span::styled("[d]", Style::default().fg(theme.primary)),
            Span::styled(" Drink  ", Style::default().fg(theme.muted)),
            Span::styled("[h]", Style::default().fg(theme.primary)),
            Span::styled(" Config", Style::default().fg(theme.muted)),
        ]),
    ];

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.secondary))
            .title(Span::styled(
                " Hydration ",
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(p, area);
}

// ─── Función principal de renderizado ────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: ratatui::layout::Rect) {
    let user = app.user.as_ref().unwrap();
    let today = chrono::Local::now().date_naive();
    let all_tasks = &app.all_tasks;

    // Datos para el motor de planificación
    let streak = &app.stats_cache.streak;
    let zen_tree = &app.stats_cache.zen_tree;
    let overdue_count = all_tasks
        .iter()
        .filter(|t| {
            !t.completed
                && t.parent_task_id.is_none()
                && t.due_date
                    .map(|d| d.with_timezone(&Local).date_naive() < today)
                    .unwrap_or(false)
        })
        .count();
    let daily_completed = app.stats_cache.todays_daily_adventures_completed;
    let daily_total = app.stats_cache.todays_daily_adventures_total;

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
            Constraint::Length(3), // encabezado de campaña
            Constraint::Min(18),   // centro de mando + hidratación
            Constraint::Length(4), // trabajo profundo + reflexión + compañerismo
        ])
        .split(main_cols[1]);

    draw_campaign_header(f, app, theme, right_rows[0], &plan);

    let command_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(right_rows[1]);
    draw_today_command_center(f, app, theme, command_row[0], all_tasks, today, &plan);

    let intel_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Min(10)])
        .split(command_row[1]);
    draw_hydration_widget(f, app, theme, intel_rows[0]);
    draw_campaign_intel(f, app, theme, intel_rows[1], all_tasks, today);

    // Fila de trabajo profundo, reflexión y compañerismo
    let stats_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38), // trabajo profundo
            Constraint::Percentage(32), // reflexión
            Constraint::Percentage(30), // compañerismo
        ])
        .split(right_rows[2]);
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
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
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
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
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
            let freqs = [
                "Daily", "2x Daily", "3x Daily", "5x Daily", "Weekdays", "Weekly", "Monthly",
            ];
            let freq_str = format!("<  {}  >", freqs[*frequency_idx]);

            f.render_widget(
                Paragraph::new(format!(" > {}", name)).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(0))
                        .title(" 1. Name "),
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
                Paragraph::new(freq_str).alignment(Alignment::Center).block(
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
        ModalType::HydrationReminder => {
            let modal_area = centered_rect(40, 35, area);
            let hydration_bg = Color::Rgb(7, 25, 48);
            let hydration_border = Color::Rgb(56, 189, 248);
            let hydration_title = Color::Rgb(224, 242, 254);
            let hydration_text = Color::Rgb(186, 230, 253);
            let hydration_muted = Color::Rgb(125, 211, 252);
            f.render_widget(Clear, modal_area);
            f.render_widget(
                Block::default().style(Style::default().bg(hydration_bg)),
                modal_area,
            );
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(hydration_border).bg(hydration_bg))
                .title(Span::styled(
                    " Hydration Reminder ",
                    Style::default()
                        .fg(hydration_title)
                        .bg(hydration_bg)
                        .add_modifier(Modifier::BOLD),
                ));
            let inner = block.inner(modal_area);
            f.render_widget(block, modal_area);
            let content = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(2),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(inner);
            f.render_widget(Paragraph::new(""), content[0]);
            f.render_widget(
                Paragraph::new(Span::styled(
                    "  Time to drink some water!",
                    Style::default()
                        .fg(hydration_title)
                        .bg(hydration_bg)
                        .add_modifier(Modifier::BOLD),
                )),
                content[1],
            );
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!(
                        "  Today: {}/{} glasses",
                        app.hydration_glasses, app.hydration_target
                    ),
                    Style::default().fg(hydration_text).bg(hydration_bg),
                )),
                content[2],
            );
            let bar_w = inner.width.saturating_sub(4) as usize;
            let filled = if app.hydration_target > 0 {
                (app.hydration_glasses * bar_w as i32 / app.hydration_target).min(bar_w as i32)
                    as usize
            } else {
                0
            };
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!("  [{}]", render_progress_bar(filled, bar_w, bar_w)),
                    Style::default().fg(hydration_border).bg(hydration_bg),
                )),
                content[3],
            );
            f.render_widget(Paragraph::new(""), content[4]);
            f.render_widget(
                Paragraph::new(Span::styled(
                    " [d] Drink  [s] Snooze 15m  [x] Dismiss ",
                    Style::default().fg(hydration_muted).bg(hydration_bg),
                ))
                .alignment(Alignment::Center),
                content[5],
            );
        }
        ModalType::HydrationSettings {
            interval_idx,
            from_hour,
            to_hour,
            target,
            pause_focus,
            focus_idx,
        } => {
            let modal_area = centered_rect(52, 55, area);
            f.render_widget(Clear, modal_area);
            f.render_widget(
                Block::default().style(Style::default().bg(theme.background)),
                modal_area,
            );
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(theme.secondary))
                .title(Span::styled(
                    " Hydration Settings ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ));
            let content = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(3), // interval
                    Constraint::Length(3), // from
                    Constraint::Length(3), // to
                    Constraint::Length(3), // target
                    Constraint::Length(3), // pause focus
                    Constraint::Min(1),
                    Constraint::Length(1),
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

            let intervals = [30i32, 45, 60, 90, 120];
            let interval_str = format!("<  {} min  >", intervals[*interval_idx]);
            f.render_widget(
                Paragraph::new(interval_str)
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border(0))
                            .title(" 1. Reminder Interval "),
                    ),
                content[1],
            );
            f.render_widget(
                Paragraph::new(format!("<  {:02}:00  >", from_hour))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border(1))
                            .title(" 2. Active From (hour) "),
                    ),
                content[2],
            );
            f.render_widget(
                Paragraph::new(format!("<  {:02}:00  >", to_hour))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border(2))
                            .title(" 3. Active To (hour) "),
                    ),
                content[3],
            );
            f.render_widget(
                Paragraph::new(format!("<  {}  glasses  >", target))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(border(3))
                            .title(" 4. Daily Target "),
                    ),
                content[4],
            );
            let pause_str = if *pause_focus {
                "[x] Pause during focus sessions"
            } else {
                "[ ] Pause during focus sessions"
            };
            f.render_widget(
                Paragraph::new(pause_str).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(border(4))
                        .title(" 5. Focus Pause "),
                ),
                content[5],
            );
            f.render_widget(
                Paragraph::new(Span::styled(
                    " [Tab] switch  |  [<->] adjust  |  [Enter] save  |  [x] Disable  |  [Esc] cancel ",
                    Style::default().fg(theme.muted),
                ))
                .alignment(Alignment::Center),
                content[7],
            );
        }
        _ => {}
    }
}
