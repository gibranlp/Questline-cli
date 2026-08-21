// ─────────────────────────────────────────────────────────────────────────────
// dashboard/todays_agenda.rs — the same actionable items as every other
// dashboard layout (main/next/quick-win quests, sidequests, dailies — the
// exact set and order `App::dashboard_command_targets()` builds), regrouped
// by due-date urgency band instead of the fixed Main/Next/Quick-Win/Sidequest
// /Daily grouping Default uses. Side panels are kept, just reused as-is in a
// narrower right-hand rail rather than redrawn from scratch.
// ─────────────────────────────────────────────────────────────────────────────

use super::default_layout;
use super::{priority_label, short_text, sidequest_rank, task_energy_tag, workload_label};
use crate::app::App;
use crate::models::Task;
use crate::screens::hit_test::TodaysAgendaHitRegions;
use crate::services::planner::{DashboardPlan, format_duration};
use crate::theme::Theme;
use chrono::Local;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

enum TaskKind {
    Main,
    Next,
    QuickWin,
}

struct AgendaTaskItem<'a> {
    action_idx: usize,
    kind: TaskKind,
    task: &'a Task,
    project_name: String,
}

const BAND_LABELS: [&str; 5] = [
    "Overdue",
    "Due Today",
    "Due Tomorrow",
    "This Week",
    "Later / No Date",
];

fn band_for(task: &Task, today: chrono::NaiveDate) -> usize {
    match task.due_date {
        None => 4,
        Some(d) => {
            let due = d.with_timezone(&Local).date_naive();
            let days = (due - today).num_days();
            if days < 0 {
                0
            } else if days == 0 {
                1
            } else if days == 1 {
                2
            } else if days <= 7 {
                3
            } else {
                4
            }
        }
    }
}

pub(super) fn draw(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    plan: &DashboardPlan,
) -> TodaysAgendaHitRegions {
    let today = chrono::Local::now().date_naive();
    let reflected_today = app
        .db
        .get_reflection_for_date(today)
        .unwrap_or(None)
        .is_some();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // ── Suggested Flow strip — a derived estimate, not a commitment: Task has
    // no time-of-day field, only a date-level due_date, so this is the
    // closest a "today's agenda" screen gets to hour labels without inventing
    // schedule data.
    let (workload_str, workload_color) = workload_label(plan.estimated_minutes);
    let flow_line = Line::from(vec![
        Span::styled(
            format!(" {} ", Local::now().format("%H:%M")),
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("Suggested Flow — ", Style::default().fg(theme.muted)),
        Span::styled(
            format!("{} workload", workload_str),
            Style::default().fg(workload_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "  ({}, {} quests today)",
                format_duration(plan.estimated_minutes),
                plan.total_quest_count
            ),
            Style::default().fg(theme.muted),
        ),
    ]);
    let flow = Paragraph::new(vec![flow_line]).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Today's Agenda "),
    );
    f.render_widget(flow, outer[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(78), Constraint::Percentage(22)])
        .split(outer[1]);

    let regions = draw_agenda_list(f, app, theme, body[0], today, plan);
    // Tree and Reflection hidden in this layout per user preference.
    default_layout::draw_side_rail(f, app, theme, body[1], reflected_today, false, false);

    regions
}

fn draw_agenda_list(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    today: chrono::NaiveDate,
    plan: &DashboardPlan,
) -> TodaysAgendaHitRegions {
    let mut action_idx = 0usize;
    let mut task_items: Vec<AgendaTaskItem> = Vec::new();

    if let Some(main) = plan.main_quest.as_ref() {
        task_items.push(AgendaTaskItem {
            action_idx,
            kind: TaskKind::Main,
            task: &main.task,
            project_name: main.project_name.clone(),
        });
        action_idx += 1;
    }
    if let Some(next) = plan.next_quest.as_ref() {
        task_items.push(AgendaTaskItem {
            action_idx,
            kind: TaskKind::Next,
            task: &next.task,
            project_name: next.project_name.clone(),
        });
        action_idx += 1;
    }
    for task in &plan.quick_wins {
        let project_name = app
            .projects
            .iter()
            .find(|p| Some(p.id) == task.project_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "General".to_string());
        task_items.push(AgendaTaskItem {
            action_idx,
            kind: TaskKind::QuickWin,
            task,
            project_name,
        });
        action_idx += 1;
    }

    let mut rows: Vec<ListItem> = Vec::new();
    let mut row_targets: Vec<Option<usize>> = Vec::new();
    let mut selected_visual_idx = None;

    let push_separator = |rows: &mut Vec<ListItem>, row_targets: &mut Vec<Option<usize>>, label: &str, color: Color| {
        if !rows.is_empty() {
            rows.push(ListItem::new(Line::from("")));
            row_targets.push(None);
        }
        rows.push(ListItem::new(Line::from(Span::styled(
            format!("  ▾ {}", label),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))));
        row_targets.push(None);
    };

    for band in 0..BAND_LABELS.len() {
        let band_items: Vec<&AgendaTaskItem> = task_items.iter().filter(|i| band_for(i.task, today) == band).collect();
        if band_items.is_empty() {
            continue;
        }
        let band_color = match band {
            0 => theme.danger,
            1 => theme.warning,
            2 => theme.focus_timer,
            3 => theme.primary,
            _ => theme.muted,
        };
        push_separator(&mut rows, &mut row_targets, BAND_LABELS[band], band_color);
        for item in band_items {
            let (prio_label, prio_color) = priority_label(item.task.priority);
            if item.action_idx == app.selected_dashboard_task_idx {
                selected_visual_idx = Some(rows.len());
            }
            let role_tag = match item.kind {
                TaskKind::Main => Span::styled("MAIN  ", Style::default().fg(theme.warning)),
                TaskKind::Next => Span::styled("NEXT  ", Style::default().fg(theme.focus_timer)),
                TaskKind::QuickWin => Span::styled("[ ]   ", Style::default().fg(theme.text)),
            };
            let (energy_label, energy_color) = task_energy_tag(item.task);
            rows.push(ListItem::new(Line::from(vec![
                role_tag,
                Span::styled(
                    format!("{} ", short_text(&item.project_name, 14)),
                    Style::default().fg(theme.muted),
                ),
                Span::styled(format!("[{}] ", prio_label), Style::default().fg(prio_color)),
                Span::styled(item.task.title.as_str(), Style::default().fg(theme.text)),
                Span::styled(format!(" [{}]", energy_label), Style::default().fg(energy_color)),
            ])));
            row_targets.push(Some(item.action_idx));
        }
    }

    push_separator(&mut rows, &mut row_targets, "Sidequests", theme.secondary);
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
        let this_idx = action_idx;
        action_idx += 1;
        let mut spans = vec![
            Span::styled(
                if done { "[x] " } else if count > 0 { "[~] " } else { "[ ] " },
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
            spans.push(Span::styled(format!(" {}", rank_name), Style::default().fg(rank_color)));
        }
        rows.push(ListItem::new(Line::from(spans)));
        row_targets.push(Some(this_idx));
    }

    push_separator(&mut rows, &mut row_targets, "Daily", theme.warning);
    for adventure in &app.stats_cache.todays_daily_adventures {
        if action_idx == app.selected_dashboard_task_idx {
            selected_visual_idx = Some(rows.len());
        }
        let this_idx = action_idx;
        action_idx += 1;
        rows.push(ListItem::new(Line::from(vec![
            Span::styled("DAILY ", Style::default().fg(theme.warning)),
            Span::styled(
                if adventure.completed { "[x] " } else { "[ ] " },
                Style::default().fg(if adventure.completed { theme.success } else { theme.text }),
            ),
            Span::styled(adventure.title.as_str(), Style::default().fg(theme.text)),
            Span::styled(
                format!(" ({}/{})", adventure.current_count, adventure.target_count),
                Style::default().fg(theme.muted),
            ),
        ])));
        row_targets.push(Some(this_idx));
    }

    if rows.is_empty() {
        rows.push(ListItem::new(Span::styled(
            "  Clear skies — nothing scheduled.",
            Style::default().fg(theme.muted),
        )));
        row_targets.push(None);
    }

    let mut state = ListState::default();
    state.select(Some(selected_visual_idx.unwrap_or(0).min(rows.len().saturating_sub(1))));

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(" Agenda — grouped by urgency ");
    let list_inner = list_block.inner(area);
    let list = List::new(rows).block(list_block).highlight_style(
        Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, area, &mut state);
    let visible_start = state.offset();

    TodaysAgendaHitRegions {
        list: list_inner,
        row_targets,
        visible_start,
    }
}

