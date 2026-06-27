// ─────────────────────────────────────────────────────────────────────────────
// models/daily_quest.rs — el struct de quest diaria
// ─────────────────────────────────────────────────────────────────────────────
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Model representing a daily quest that refreshes daily.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyQuest {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub due_date: NaiveDate,
}
