use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::database::Database;
use crate::models::BudgetWarningLevel;
use crate::services::notifications::NotificationIcon;
use crate::services::treasury::TreasuryService;

#[derive(Debug, Clone)]
pub struct TreasuryNotificationEvent {
    pub title: String,
    pub message: String,
    pub urgent: bool,
    pub icon: NotificationIcon,
}

/// Evalúa vencimientos y presupuestos, y registra cada alerta con una clave idempotente.
pub fn collect_treasury_notifications(
    db: &Database,
    now: DateTime<Utc>,
) -> Result<Vec<TreasuryNotificationEvent>> {
    let service = TreasuryService::new(db);
    let mut events = Vec::new();
    for campaign in db
        .get_projects()?
        .into_iter()
        .filter(|campaign| !campaign.archived)
    {
        service.ensure_campaign(campaign.id)?;
        for entry in service.upcoming_payments(campaign.id, now, 7)? {
            let Some(due) = entry.due_date else { continue };
            let event = TreasuryNotificationEvent {
                title: "Upcoming payment due".to_string(),
                message: format!("{} is due on {}.", entry.title, due.format("%Y-%m-%d")),
                urgent: false,
                icon: NotificationIcon::Info,
            };
            if insert_once(
                db,
                &format!("treasury:due:{}:{}", entry.id, due.date_naive()),
                "treasury_due",
                &event,
                &entry.id.to_string(),
            )? {
                events.push(event);
            }
        }
        for entry in service.overdue_payments(campaign.id, now)? {
            let event = TreasuryNotificationEvent {
                title: "Expense overdue".to_string(),
                message: format!("{} has not been paid.", entry.title),
                urgent: true,
                icon: NotificationIcon::Warning,
            };
            if insert_once(
                db,
                &format!("treasury:overdue:{}:{}", entry.id, now.date_naive()),
                "treasury_overdue",
                &event,
                &entry.id.to_string(),
            )? {
                events.push(event);
            }
        }
        for category in service.calculate_category_totals(campaign.id)? {
            let Some(budget) = category.budget_minor else {
                continue;
            };
            let usage = TreasuryService::budget_usage(budget, category.spending_minor);
            let threshold = match usage.warning {
                BudgetWarningLevel::EightyPercent => Some("80"),
                BudgetWarningLevel::NinetyPercent => Some("90"),
                BudgetWarningLevel::Exhausted => Some("100"),
                BudgetWarningLevel::Exceeded => Some("exceeded"),
                BudgetWarningLevel::Healthy => None,
            };
            let Some(threshold) = threshold else { continue };
            let event = TreasuryNotificationEvent {
                title: "Category budget alert".to_string(),
                message: format!(
                    "{} budget reached {}% or more.",
                    category.category.name, threshold
                ),
                urgent: matches!(
                    usage.warning,
                    BudgetWarningLevel::Exhausted | BudgetWarningLevel::Exceeded
                ),
                icon: NotificationIcon::Warning,
            };
            if insert_once(
                db,
                &format!("treasury:category:{}:{}", category.category.id, threshold),
                "treasury_category_budget",
                &event,
                &category.category.id.to_string(),
            )? {
                events.push(event);
            }
        }
    }
    Ok(events)
}

fn insert_once(
    db: &Database,
    id: &str,
    kind: &str,
    event: &TreasuryNotificationEvent,
    target_id: &str,
) -> Result<bool> {
    let existed = db.conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM notifications WHERE id=?1)",
        rusqlite::params![id],
        |row| row.get::<_, bool>(0),
    )?;
    if existed {
        return Ok(false);
    }
    db.create_notification_once(id, kind, &event.title, &event.message, Some(target_id))?;
    Ok(true)
}
