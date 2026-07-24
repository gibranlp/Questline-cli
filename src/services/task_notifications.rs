use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::database::Database;
use crate::models::{Task, TaskPriority};
use crate::services::notifications::NotificationIcon;

const KEY_ENABLED: &str = "task_notifications_enabled";
const KEY_DUE_ENABLED: &str = "task_due_notifications_enabled";
const KEY_OVERDUE_ENABLED: &str = "task_overdue_notifications_enabled";
const KEY_SUMMARY_ENABLED: &str = "task_daily_summary_notifications_enabled";
const KEY_IDLE_ENABLED: &str = "task_idle_notifications_enabled";
const KEY_FELLOWSHIP_ENABLED: &str = "task_fellowship_notifications_enabled";
const KEY_LAST_PRODUCTIVE_AT: &str = "task_last_productive_at";
const IDLE_AFTER_HOURS: i64 = 4;

#[derive(Debug, Clone)]
pub struct TaskNotificationEvent {
    pub title: String,
    pub message: String,
    pub urgent: bool,
    pub icon: NotificationIcon,
}

#[derive(Debug, Clone, Copy)]
struct TaskNotificationSettings {
    enabled: bool,
    due: bool,
    overdue: bool,
    summary: bool,
    idle: bool,
    fellowship: bool,
}

impl TaskNotificationSettings {
    fn load(db: &Database) -> Self {
        Self {
            enabled: setting_bool(db, KEY_ENABLED, true),
            due: setting_bool(db, KEY_DUE_ENABLED, true),
            overdue: setting_bool(db, KEY_OVERDUE_ENABLED, true),
            summary: setting_bool(db, KEY_SUMMARY_ENABLED, true),
            idle: setting_bool(db, KEY_IDLE_ENABLED, true),
            fellowship: setting_bool(db, KEY_FELLOWSHIP_ENABLED, true),
        }
    }
}

pub fn task_notifications_enabled(db: &Database) -> bool {
    setting_bool(db, KEY_ENABLED, true)
}

pub fn set_task_notifications_enabled(db: &Database, enabled: bool) -> Result<()> {
    db.set_setting(KEY_ENABLED, if enabled { "true" } else { "false" })
}

pub fn record_productive_action(db: &Database, now: DateTime<Utc>) {
    let _ = db.set_setting(KEY_LAST_PRODUCTIVE_AT, &now.to_rfc3339());
}

/// Evalúa recordatorios de tareas y registra cada emisión antes de devolver eventos al llamador.
pub fn collect_task_notifications(
    db: &Database,
    tasks: &[Task],
    now: DateTime<Utc>,
) -> Result<Vec<TaskNotificationEvent>> {
    let settings = TaskNotificationSettings::load(db);
    if !settings.enabled {
        return Ok(Vec::new());
    }

    let mut events = Vec::new();
    let today = now.date_naive();

    if settings.summary {
        if let Some(event) = daily_summary_event(db, tasks, today)? {
            events.push(event);
        }
    }

    if settings.due || settings.overdue {
        for task in tasks.iter().filter(|t| !t.completed && t.parent_task_id.is_none()) {
            if task.recurrence.is_some() {
                continue;
            }
            if let Some(due) = task.due_date {
                if settings.overdue {
                    if let Some(event) = overdue_event(db, task, due, now, today)? {
                        events.push(event);
                        continue;
                    }
                }
                if settings.due {
                    if let Some(event) = due_event(db, task, due, now)? {
                        events.push(event);
                    }
                }
            }
        }
    }

    if settings.idle {
        if let Some(event) = idle_event(db, tasks, now, today)? {
            events.push(event);
        }
    }

    if settings.fellowship {
        events.extend(fellowship_events(db)?);
    }

    Ok(events)
}

fn due_event(
    db: &Database,
    task: &Task,
    due: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<Option<TaskNotificationEvent>> {
    let windows: &[(&str, i64)] = if task.priority == TaskPriority::High {
        &[("24h", 24), ("2h", 2)]
    } else {
        &[("24h", 24), ("1h", 1)]
    };

    for (label, hours) in windows {
        let threshold = due - chrono::Duration::hours(*hours);
        if now >= threshold && now < due {
            let key = task_key("due_before", task.id, label);
            if mark_once(db, &key)? {
                return Ok(Some(TaskNotificationEvent {
                    title: "Quest approaching".to_string(),
                    message: format!("{} is due in {}.", task.title, label),
                    urgent: task.priority == TaskPriority::High,
                    icon: if task.priority == TaskPriority::High {
                        NotificationIcon::TaskHighPriority
                    } else {
                        NotificationIcon::TaskDue
                    },
                }));
            }
        }
    }

    if now >= due && now < due + chrono::Duration::minutes(30) {
        let key = task_key("due_now", task.id, "once");
        if mark_once(db, &key)? {
            return Ok(Some(TaskNotificationEvent {
                title: "Quest due".to_string(),
                message: format!("{} is due now.", task.title),
                urgent: task.priority == TaskPriority::High,
                icon: if task.priority == TaskPriority::High {
                    NotificationIcon::TaskHighPriority
                } else {
                    NotificationIcon::TaskDue
                },
            }));
        }
    }

    Ok(None)
}

fn overdue_event(
    db: &Database,
    task: &Task,
    due: DateTime<Utc>,
    now: DateTime<Utc>,
    today: NaiveDate,
) -> Result<Option<TaskNotificationEvent>> {
    if now < due + chrono::Duration::minutes(30) {
        return Ok(None);
    }

    let first_key = task_key("overdue_first", task.id, "once");
    if let Some(sent_at) = db.get_setting(&first_key)? {
        if DateTime::parse_from_rfc3339(&sent_at)
            .map(|d| d.with_timezone(&Utc).date_naive() == today)
            .unwrap_or(false)
        {
            return Ok(None);
        }
    } else {
        db.set_setting(&first_key, &now.to_rfc3339())?;
        return Ok(Some(TaskNotificationEvent {
            title: "Quest overdue".to_string(),
            message: format!("{} is overdue.", task.title),
            urgent: task.priority == TaskPriority::High,
            icon: NotificationIcon::TaskOverdue,
        }));
    }

    let daily_key = task_key("overdue_daily", task.id, &today.format("%Y-%m-%d").to_string());
    if mark_once(db, &daily_key)? {
        return Ok(Some(TaskNotificationEvent {
            title: "Overdue quest".to_string(),
            message: format!("{} still needs your attention.", task.title),
            urgent: false,
            icon: NotificationIcon::TaskOverdue,
        }));
    }

    Ok(None)
}

fn daily_summary_event(
    db: &Database,
    tasks: &[Task],
    today: NaiveDate,
) -> Result<Option<TaskNotificationEvent>> {
    let key = format!("task_notify:summary:{}", today.format("%Y-%m-%d"));
    if !mark_once(db, &key)? {
        return Ok(None);
    }

    let count = due_today_count(tasks, today);
    if count == 0 {
        return Ok(None);
    }

    Ok(Some(TaskNotificationEvent {
        title: "Today's quests".to_string(),
        message: format!("{} quest{} due today.", count, if count == 1 { "" } else { "s" }),
        urgent: false,
        icon: NotificationIcon::TaskDailySummary,
    }))
}

fn idle_event(
    db: &Database,
    tasks: &[Task],
    now: DateTime<Utc>,
    today: NaiveDate,
) -> Result<Option<TaskNotificationEvent>> {
    let count = due_today_count(tasks, today);
    if count == 0 {
        return Ok(None);
    }

    let Some(last) = db
        .get_setting(KEY_LAST_PRODUCTIVE_AT)?
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc))
    else {
        record_productive_action(db, now);
        return Ok(None);
    };

    if now - last < chrono::Duration::hours(IDLE_AFTER_HOURS) {
        return Ok(None);
    }

    let key = format!("task_notify:idle:{}", today.format("%Y-%m-%d"));
    if mark_once(db, &key)? {
        return Ok(Some(TaskNotificationEvent {
            title: "Questline check-in".to_string(),
            message: "A quest due today is still waiting.".to_string(),
            urgent: false,
            icon: NotificationIcon::TaskIdle,
        }));
    }

    Ok(None)
}

fn fellowship_events(db: &Database) -> Result<Vec<TaskNotificationEvent>> {
    let mut events = Vec::new();
    for (id, kind, title, content, _, read, _) in db.get_notifications()? {
        if read || !kind.starts_with("task_") {
            continue;
        }
        let key = format!("task_notify:fellowship_seen:{}", id);
        if mark_once(db, &key)? {
            events.push(TaskNotificationEvent {
                title,
                message: content,
                urgent: kind == "task_assignment",
                icon: NotificationIcon::Fellowship,
            });
        }
        if events.len() >= 3 {
            break;
        }
    }
    Ok(events)
}

fn due_today_count(tasks: &[Task], today: NaiveDate) -> usize {
    tasks
        .iter()
        .filter(|t| {
            !t.completed
                && t.parent_task_id.is_none()
                && t.due_date.map(|d| d.date_naive() == today).unwrap_or(false)
        })
        .count()
}

fn task_key(kind: &str, task_id: Uuid, suffix: &str) -> String {
    format!("task_notify:{}:{}:{}", kind, task_id, suffix)
}

fn mark_once(db: &Database, key: &str) -> Result<bool> {
    if db.get_setting(key)?.is_some() {
        return Ok(false);
    }
    db.set_setting(key, &Utc::now().to_rfc3339())?;
    Ok(true)
}

fn setting_bool(db: &Database, key: &str, default: bool) -> bool {
    db.get_setting(key)
        .ok()
        .flatten()
        .map(|s| s == "true")
        .unwrap_or(default)
}
