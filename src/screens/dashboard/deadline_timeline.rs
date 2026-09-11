// ─────────────────────────────────────────────────────────────────────────────
// dashboard/deadline_timeline.rs — a calendar-strip summary of when today's
// actionable quests (main/next/quick-wins — the same set every layout shares)
// fall due, plus the same action list sorted strictly by deadline instead of
// Default's fixed grouping or Today's Agenda's named bands.
//
// Scope note: the strip is a decorative density summary built from the
// planner's picks, not every task in `app.all_tasks`, and there is no
// keyboard day-column picker yet — Left/Right keep their existing Dashboard
// behavior. A future pass could wire day selection into a dedicated
// App field if that turns out to be worth the risk of touching the shared
// Left/Right key arms; this first version keeps the strip purely visual so it
// doesn't need to.
// ─────────────────────────────────────────────────────────────────────────────

use super::default_layout;
use super::{format_due_date, priority_label, short_text, sidequest_rank, task_energy_tag};
use crate::app::App;
use crate::models::Task;
use crate::screens::hit_test::DeadlineTimelineHitRegions;
use crate::services::planner::DashboardPlan;
use crate::theme::Theme;
use chrono::{Local, NaiveDate};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

/// How many individual upcoming days get their own column, after the pinned
/// Overdue column and before the trailing Beyond column: Today + this many
/// more days.
const FUTURE_DAYS: i64 = 6;

struct TimelineTaskItem<'a> {
    action_idx: usize,
    task: &'a Task,
    project_name: String,
}

fn column_index(task: &Task, today: NaiveDate) -> usize {
    match task.due_date {
        None => FUTURE_DAYS as usize + 2,
        Some(d) => {
            let due = d.with_timezone(&Local).date_naive();
            let days = (due - today).num_days();
            if days < 0 {
                0
            } else if days <= FUTURE_DAYS {
                1 + days as usize
            } else {
                FUTURE_DAYS as usize + 2
            }
        }
    }
}

fn column_label(idx: usize, today: NaiveDate) -> String {
    if idx == 0 {
        "OVERDUE".to_string()
    } else if idx == FUTURE_DAYS as usize + 2 {
        "BEYOND".to_string()
    } else if idx == 1 {
        "TODAY".to_string()
    } else {
        let day = today + chrono::Duration::days((idx - 1) as i64);
        day.format("%a %d").to_string().to_uppercase()
    }
}

pub(super) fn draw(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    plan: &DashboardPlan,
) -> DeadlineTimelineHitRegions {
    let today = chrono::Local::now().date_naive();
    let reflected_today = app
        .db
        .get_reflection_for_date(today)
        .unwrap_or(None)
        .is_some();

    let mut action_idx = 0usize;
    let mut items: Vec<TimelineTaskItem> = Vec::new();
    if let Some(main) = plan.main_quest.as_ref() {
        items.push(TimelineTaskItem { action_idx, task: &main.task, project_name: main.project_name.clone() });
        action_idx += 1;
    }
    if let Some(next) = plan.next_quest.as_ref() {
        items.push(TimelineTaskItem { action_idx, task: &next.task, project_name: next.project_name.clone() });
        action_idx += 1;
    }
    for task in &plan.quick_wins {
        let project_name = app
            .projects
            .iter()
            .find(|p| Some(p.id) == task.project_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "General".to_string());
        items.push(TimelineTaskItem { action_idx, task, project_name });
        action_idx += 1;
    }

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    default_layout::draw_campaign_header(f, app, theme, outer[0], plan);
    draw_calendar_strip(f, theme, outer[1], &items, today);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(78), Constraint::Percentage(22)])
        .split(outer[2]);

    let regions = draw_deadline_list(f, app, theme, body[0], &items, action_idx);
    // Tree hidden per user preference; Reflection stays.
    default_layout::draw_side_rail(f, app, theme, body[1], reflected_today, false, true);

    regions
}

fn draw_calendar_strip(f: &mut Frame, theme: &Theme, area: Rect, items: &[TimelineTaskItem], today: NaiveDate) {
    let num_cols = FUTURE_DAYS as usize + 3; // Overdue + Today..+FUTURE_DAYS + Beyond
    let mut counts = vec![0usize; num_cols];
    let mut worst_priority = vec![None::<crate::models::TaskPriority>; num_cols];
    for item in items {
        let col = column_index(item.task, today);
        counts[col] += 1;
        worst_priority[col] = Some(match worst_priority[col] {
            Some(existing) if existing >= item.task.priority => existing,
            _ => item.task.priority,
        });
    }
    let worst_priority_color: Vec<Option<Color>> = worst_priority
        .iter()
        .map(|p| p.map(|p| priority_label(p).1))
        .collect();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![Constraint::Ratio(1, num_cols as u32); num_cols])
        .split(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.border))
                .title(" Upcoming Deadlines (today's picks) ")
                .inner(area),
        );
    f.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Upcoming Deadlines (today's picks) "),
        area,
    );

    for (idx, col_area) in cols.iter().enumerate() {
        let count = counts[idx];
        let bar_color = worst_priority_color[idx].unwrap_or(theme.muted);
        let bar = if count == 0 {
            "·".to_string()
        } else {
            "▓".repeat(count.min(4))
        };
        let label_color = if idx == 0 && count > 0 { theme.danger } else { theme.muted };
        let lines = vec![
            Line::from(Span::styled(column_label(idx, today), Style::default().fg(label_color))),
            Line::from(vec![
                Span::styled(bar, Style::default().fg(bar_color)),
                Span::styled(format!(" {}", count), Style::default().fg(theme.text)),
            ]),
        ];
        f.render_widget(Paragraph::new(lines), *col_area);
    }
}

fn draw_deadline_list(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    items: &[TimelineTaskItem],
    mut action_idx: usize,
) -> DeadlineTimelineHitRegions {
    let mut sorted: Vec<&TimelineTaskItem> = items.iter().collect();
    sorted.sort_by_key(|i| (i.task.due_date.is_none(), i.task.due_date));

    let mut rows: Vec<ListItem> = Vec::new();
    let mut row_targets: Vec<Option<usize>> = Vec::new();
    let mut selected_visual_idx = None;

    for item in sorted {
        let (prio_label, prio_color) = priority_label(item.task.priority);
        let (energy_label, energy_color) = task_energy_tag(item.task);
        if item.action_idx == app.selected_dashboard_task_idx {
            selected_visual_idx = Some(rows.len());
        }
        let due_str = format_due_date(item.task.due_date);
        rows.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{:>11} ", due_str), Style::default().fg(theme.focus_timer)),
            Span::styled(format!("{} ", short_text(&item.project_name, 14)), Style::default().fg(theme.muted)),
            Span::styled(format!("[{}] ", prio_label), Style::default().fg(prio_color)),
            Span::styled(item.task.title.as_str(), Style::default().fg(theme.text)),
            Span::styled(format!(" [{}]", energy_label), Style::default().fg(energy_color)),
        ])));
        row_targets.push(Some(item.action_idx));
    }

    let push_separator = |rows: &mut Vec<ListItem>, row_targets: &mut Vec<Option<usize>>, label: &str, color: Color| {
        if !rows.is_empty() {
            rows.push(ListItem::new(Line::from("")));
            row_targets.push(None);
        }
        rows.push(ListItem::new(Line::from(Span::styled(format!("  -- {} --", label), Style::default().fg(color)))));
        row_targets.push(None);
    };

    push_separator(&mut rows, &mut row_targets, "Sidequests (due today)", theme.secondary);
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
            Span::styled(format!(" ({}/{}) +{} XP ", count, target, ritual.reward_xp), Style::default().fg(theme.muted)),
            Span::styled(format!("{}d", streak), Style::default().fg(rank.map_or(theme.muted, |(_, color)| color))),
        ];
        if let Some((rank_name, rank_color)) = rank {
            spans.push(Span::styled(format!(" {}", rank_name), Style::default().fg(rank_color)));
        }
        rows.push(ListItem::new(Line::from(spans)));
        row_targets.push(Some(this_idx));
    }

    push_separator(&mut rows, &mut row_targets, "Daily (due today)", theme.warning);
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
            Span::styled(format!(" ({}/{})", adventure.current_count, adventure.target_count), Style::default().fg(theme.muted)),
        ])));
        row_targets.push(Some(this_idx));
    }

    if rows.is_empty() {
        rows.push(ListItem::new(Span::styled(
            "  No dated quests in range.",
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
        .title(" Sorted by Deadline ");
    let list_inner = list_block.inner(area);
    let list = List::new(rows).block(list_block).highlight_style(
        Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, area, &mut state);
    let visible_start = state.offset();

    DeadlineTimelineHitRegions {
        list: list_inner,
        row_targets,
        visible_start,
    }
}
