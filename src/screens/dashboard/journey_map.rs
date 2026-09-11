// ─────────────────────────────────────────────────────────────────────────────
// dashboard/journey_map.rs — the dashboard as a trail: a decorative banner
// shows START → quick-win spurs → "you are here" (today's main quest) → next
// quest → FINISH, and the actual actionable list below it (same action-index
// space as every other layout) carries waypoint-flavored icons instead of
// Default's MAIN/NEXT/[ ] tags. Side panels are the shared rail from
// default_layout, unchanged.
// ─────────────────────────────────────────────────────────────────────────────

use super::default_layout;
use super::{priority_label, short_text, sidequest_rank, task_energy_tag, workload_label};
use crate::app::{App, DashboardCommandTarget};
use crate::screens::hit_test::JourneyMapHitRegions;
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

pub(super) fn draw(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    plan: &DashboardPlan,
) -> JourneyMapHitRegions {
    let today = chrono::Local::now().date_naive();
    let reflected_today = app
        .db
        .get_reflection_for_date(today)
        .unwrap_or(None)
        .is_some();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    default_layout::draw_campaign_header(f, app, theme, outer[0], plan);
    draw_trail_banner(f, theme, outer[1], plan);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(76), Constraint::Percentage(24)])
        .split(outer[2]);

    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(6)])
        .split(body[0]);

    let regions = draw_waypoint_list(f, app, theme, left_rows[0], plan);
    draw_waypoint_detail(f, app, theme, left_rows[1]);
    // Tree hidden per user preference; Reflection stays.
    default_layout::draw_side_rail(f, app, theme, body[1], reflected_today, false, true);

    regions
}

/// A hand-built, decorative-only (no hit-testing) ASCII trail summarizing
/// today's plan — START, one spur per quick win, "you are here" at the main
/// quest, next quest ahead, and a FINISH marker with whatever doesn't fit.
fn draw_trail_banner(f: &mut Frame, theme: &Theme, area: Rect, plan: &DashboardPlan) {
    let has_any = plan.main_quest.is_some() || plan.next_quest.is_some() || !plan.quick_wins.is_empty();

    let line1 = if !has_any {
        Line::from(vec![
            Span::styled("★ START ", Style::default().fg(theme.muted)),
            Span::styled(
                "──── Camp cleared. No quests today. ────",
                Style::default().fg(theme.success),
            ),
            Span::styled("◇ FINISH", Style::default().fg(theme.muted)),
        ])
    } else {
        let mut spans = vec![Span::styled("★ START ", Style::default().fg(theme.muted))];
        for _ in 0..plan.quick_wins.len().min(5) {
            spans.push(Span::styled("──●", Style::default().fg(theme.primary)));
        }
        if let Some(main) = plan.main_quest.as_ref() {
            spans.push(Span::styled("──▲ ", Style::default().fg(theme.warning)));
            spans.push(Span::styled(
                format!("YOU ARE HERE: {}", short_text(&main.task.title, 26)),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            ));
        }
        if let Some(next) = plan.next_quest.as_ref() {
            spans.push(Span::styled("  ──○ ", Style::default().fg(theme.focus_timer)));
            spans.push(Span::styled(
                format!("next: {}", short_text(&next.task.title, 20)),
                Style::default().fg(theme.text),
            ));
        }
        spans.push(Span::styled("  ── ⋮ ──◇ FINISH", Style::default().fg(theme.muted)));
        Line::from(spans)
    };

    let shown = plan.quick_wins.len()
        + plan.main_quest.is_some() as usize
        + plan.next_quest.is_some() as usize;
    let remaining = plan.total_quest_count.saturating_sub(shown);
    let (workload_str, workload_color) = workload_label(plan.estimated_minutes);
    let line2 = Line::from(vec![
        Span::styled(
            format!("  {} more quest(s) beyond today's picks  ·  ", remaining),
            Style::default().fg(theme.muted),
        ),
        Span::styled(
            format!("{} · {} workload", format_duration(plan.estimated_minutes), workload_str),
            Style::default().fg(workload_color),
        ),
    ]);

    let p = Paragraph::new(vec![line1, line2]).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" The Trail "),
    );
    f.render_widget(p, area);
}

fn draw_waypoint_list(
    f: &mut Frame,
    app: &App,
    theme: &Theme,
    area: Rect,
    plan: &DashboardPlan,
) -> JourneyMapHitRegions {
    let mut rows: Vec<ListItem> = Vec::new();
    let mut row_targets: Vec<Option<usize>> = Vec::new();
    let mut selected_visual_idx = None;
    let mut action_idx = 0usize;

    let push_separator = |rows: &mut Vec<ListItem>, row_targets: &mut Vec<Option<usize>>, label: &str, color: Color| {
        if !rows.is_empty() {
            rows.push(ListItem::new(Line::from("")));
            row_targets.push(None);
        }
        rows.push(ListItem::new(Line::from(Span::styled(
            format!("  ~ {} ~", label),
            Style::default().fg(color),
        ))));
        row_targets.push(None);
    };

    if let Some(main) = plan.main_quest.as_ref() {
        let (prio_label, prio_color) = priority_label(main.task.priority);
        if action_idx == app.selected_dashboard_task_idx {
            selected_visual_idx = Some(rows.len());
        }
        row_targets.push(Some(action_idx));
        action_idx += 1;
        rows.push(ListItem::new(Line::from(vec![
            Span::styled("▲ HERE ", Style::default().fg(theme.warning)),
            Span::styled(format!("{} ", short_text(&main.project_name, 14)), Style::default().fg(theme.muted)),
            Span::styled(format!("[{}] ", prio_label), Style::default().fg(prio_color)),
            Span::styled(main.task.title.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  {}", format_duration(main.est_minutes)), Style::default().fg(theme.muted)),
        ])));
    }

    if let Some(next) = plan.next_quest.as_ref() {
        let (prio_label, prio_color) = priority_label(next.task.priority);
        if action_idx == app.selected_dashboard_task_idx {
            selected_visual_idx = Some(rows.len());
        }
        row_targets.push(Some(action_idx));
        action_idx += 1;
        rows.push(ListItem::new(Line::from(vec![
            Span::styled("○ NEXT ", Style::default().fg(theme.focus_timer)),
            Span::styled(format!("{} ", short_text(&next.project_name, 14)), Style::default().fg(theme.muted)),
            Span::styled(format!("[{}] ", prio_label), Style::default().fg(prio_color)),
            Span::styled(next.task.title.as_str(), Style::default().fg(theme.text)),
            Span::styled(format!("  {}", format_duration(next.est_minutes)), Style::default().fg(theme.muted)),
        ])));
    }

    push_separator(&mut rows, &mut row_targets, "Side Spurs", theme.primary);
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
        row_targets.push(Some(action_idx));
        action_idx += 1;
        rows.push(ListItem::new(Line::from(vec![
            Span::styled("● ", Style::default().fg(theme.primary)),
            Span::styled(format!("{} ", short_text(project_name, 14)), Style::default().fg(theme.muted)),
            Span::styled(format!("[{}] ", prio_label), Style::default().fg(prio_color)),
            Span::styled(task.title.as_str(), Style::default().fg(theme.text)),
            Span::styled(format!(" [{}]", energy_label), Style::default().fg(energy_color)),
        ])));
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
        row_targets.push(Some(action_idx));
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
    }

    push_separator(&mut rows, &mut row_targets, "Daily", theme.warning);
    for adventure in &app.stats_cache.todays_daily_adventures {
        if action_idx == app.selected_dashboard_task_idx {
            selected_visual_idx = Some(rows.len());
        }
        row_targets.push(Some(action_idx));
        action_idx += 1;
        rows.push(ListItem::new(Line::from(vec![
            Span::styled("☀ ", Style::default().fg(theme.warning)),
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
    }

    if rows.is_empty() {
        rows.push(ListItem::new(Span::styled(
            "  No waypoints charted today.",
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
        .title(" Waypoints  Space Done | Enter Open ");
    let list_inner = list_block.inner(area);
    let list = List::new(rows).block(list_block).highlight_style(
        Style::default().fg(Color::Black).bg(theme.selection).add_modifier(Modifier::BOLD),
    );
    f.render_stateful_widget(list, area, &mut state);
    let visible_start = state.offset();

    JourneyMapHitRegions {
        list: list_inner,
        row_targets,
        visible_start,
    }
}

fn draw_waypoint_detail(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let mut lines = Vec::new();
    if let Some(DashboardCommandTarget::Task(task)) = app.selected_dashboard_command_target() {
        let (prio_label, prio_color) = priority_label(task.priority);
        let project_name = app
            .projects
            .iter()
            .find(|p| Some(p.id) == task.project_id)
            .map(|p| p.name.as_str())
            .unwrap_or("General");
        lines.push(Line::from(vec![
            Span::styled("Waypoint: ", Style::default().fg(theme.focus_timer).add_modifier(Modifier::BOLD)),
            Span::styled(format!("[{}] ", prio_label), Style::default().fg(prio_color)),
            Span::styled(short_text(&task.title, 30), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!(" ({})", short_text(project_name, 14)), Style::default().fg(theme.muted)),
        ]));
        let due_label = task
            .due_date
            .map(|d| d.with_timezone(&Local).format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "None".to_string());
        lines.push(Line::from(vec![
            Span::styled("  Due: ", Style::default().fg(theme.muted)),
            Span::styled(due_label, Style::default().fg(theme.text)),
        ]));
        let description = task
            .description
            .as_deref()
            .filter(|d| !d.trim().is_empty())
            .map(|d| short_text(d.trim(), 60))
            .unwrap_or_else(|| "No description.".to_string());
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(description, Style::default().fg(theme.text)),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            "  Select a waypoint to see its details.",
            Style::default().fg(theme.muted),
        )));
    }

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border))
            .title(" Waypoint Detail "),
    );
    f.render_widget(p, area);
}
