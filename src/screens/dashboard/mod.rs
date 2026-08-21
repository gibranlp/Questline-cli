// ─────────────────────────────────────────────────────────────────────────────
// dashboard/mod.rs — el centro de comando del héroe: campaña de hoy y estado
// del reino. Selecciona entre varias presentaciones ("layouts") de los mismos
// datos (DashboardPlan + app.stats_cache), computados una sola vez por frame
// aquí y pasados por referencia a quien corresponda dibujar.
// ─────────────────────────────────────────────────────────────────────────────

mod default_layout;
mod deadline_timeline;
mod journey_map;
mod modals;
mod todays_agenda;

use crate::app::App;
use crate::models::{Task, TaskPriority};
use crate::screens::hit_test::DashboardHitRegions;
use crate::services::planner::{self, DashboardPlan};
use crate::theme::Theme;
use chrono::{Local, Timelike};
use ratatui::{Frame, layout::Rect, style::Color};

/// Which full-screen presentation of the dashboard's data is active.
/// `Default` is today's original dashboard, kept byte-for-byte unchanged.
/// Adding a new layout later is: one variant here, one entry in `ORDER`,
/// `key()`, and `from_key()`, one match arm in `draw()` below, and one new
/// module file — nothing else needs to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardLayout {
    Default,
    JourneyMap,
    TodaysAgenda,
    DeadlineTimeline,
}

impl DashboardLayout {
    pub const ORDER: [DashboardLayout; 4] = [
        DashboardLayout::Default,
        DashboardLayout::JourneyMap,
        DashboardLayout::TodaysAgenda,
        DashboardLayout::DeadlineTimeline,
    ];

    /// Round-robins to the next layout in `ORDER`, wrapping back to the
    /// first — the `m` key on the Dashboard screen drives this.
    pub fn next(self) -> Self {
        let idx = Self::ORDER.iter().position(|l| *l == self).unwrap_or(0);
        Self::ORDER[(idx + 1) % Self::ORDER.len()]
    }

    /// Stable string key persisted via `Database::set_setting("dashboard_layout", ...)`
    /// — mirrors `Theme::theme_key`/`choice_from_key` for the same purpose.
    pub fn key(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::JourneyMap => "JourneyMap",
            Self::TodaysAgenda => "TodaysAgenda",
            Self::DeadlineTimeline => "DeadlineTimeline",
        }
    }

    /// Reverse of `key()`. An unrecognized/missing key (fresh install, or a
    /// setting written by a future version with a layout we don't know)
    /// falls back to `Default`, never panics.
    pub fn from_key(key: &str) -> Self {
        match key {
            "JourneyMap" => Self::JourneyMap,
            "TodaysAgenda" => Self::TodaysAgenda,
            "DeadlineTimeline" => Self::DeadlineTimeline,
            _ => Self::Default,
        }
    }

    /// Short human label for help text / status lines.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::JourneyMap => "Journey Map",
            Self::TodaysAgenda => "Today's Agenda",
            Self::DeadlineTimeline => "Deadline Timeline",
        }
    }
}

// ─── Helpers shared across every layout ──────────────────────────────────────
// (moved here verbatim from the original single-layout dashboard.rs; `super::`
// from any layout module reaches these.)

pub(super) fn greeting(username: &str) -> String {
    let hour = chrono::Local::now().hour();
    let salutation = match hour {
        5..=11 => "morning",
        12..=17 => "afternoon",
        _ => "evening",
    };
    format!("Good {}, {}.", salutation, username)
}

pub(super) fn priority_label(priority: TaskPriority) -> (&'static str, Color) {
    match priority {
        TaskPriority::High => ("HIGH", Color::Rgb(239, 68, 68)),
        TaskPriority::Medium => ("MED", Color::Rgb(245, 158, 11)),
        TaskPriority::Low => ("LOW", Color::Rgb(107, 114, 128)),
    }
}

pub(super) fn render_progress_bar(filled: usize, total: usize, width: usize) -> String {
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

pub(super) fn workload_label(minutes: u32) -> (&'static str, Color) {
    match minutes {
        0..=90 => ("Light", Color::Rgb(34, 197, 94)),
        91..=300 => ("Balanced", Color::Rgb(245, 158, 11)),
        301..=480 => ("Heavy", Color::Rgb(249, 115, 22)),
        _ => ("Epic", Color::Rgb(239, 68, 68)),
    }
}

pub(super) fn sidequest_rank(streak: i32) -> Option<(&'static str, Color)> {
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

pub(super) fn task_energy_tag(task: &Task) -> (&'static str, Color) {
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

pub(super) fn short_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let short: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}...", short)
    } else {
        short
    }
}

pub(super) fn word_wrap(text: &str, max_width: usize) -> Vec<String> {
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

// ─── Dispatcher ───────────────────────────────────────────────────────────────

fn build_plan(app: &App) -> DashboardPlan {
    let today = chrono::Local::now().date_naive();
    let all_tasks = &app.all_tasks;
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

    planner::generate_plan(
        all_tasks,
        &app.projects,
        today,
        overdue_count,
        streak.current_streak,
        zen_tree.health,
        daily_completed,
        daily_total,
        app.quest_visibility_horizon_days(),
    )
}

pub fn draw(f: &mut Frame, app: &App, theme: &Theme, area: Rect) -> DashboardHitRegions {
    // Computed once per frame regardless of which layout is active — every
    // layout module reads this plan, none of them re-derive it.
    let plan = build_plan(app);

    let regions = match app.dashboard_layout {
        DashboardLayout::Default => {
            DashboardHitRegions::Default(default_layout::draw(f, app, theme, area, &plan))
        }
        DashboardLayout::JourneyMap => {
            DashboardHitRegions::JourneyMap(journey_map::draw(f, app, theme, area, &plan))
        }
        DashboardLayout::TodaysAgenda => {
            DashboardHitRegions::TodaysAgenda(todays_agenda::draw(f, app, theme, area, &plan))
        }
        DashboardLayout::DeadlineTimeline => DashboardHitRegions::DeadlineTimeline(
            deadline_timeline::draw(f, app, theme, area, &plan),
        ),
    };

    // Modals render once, outside the layout match, so every layout gets
    // correct modal rendering for free and a new layout can't forget it.
    modals::draw_modals(f, app, theme, area);

    regions
}
