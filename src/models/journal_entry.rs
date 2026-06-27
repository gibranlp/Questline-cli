// ─────────────────────────────────────────────────────────────────────────────
// models/journal_entry.rs — el struct de entrada de journal con visibilidad
// ─────────────────────────────────────────────────────────────────────────────
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Model representing a project daily work log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub id: Uuid,
    pub project_id: Uuid,
    pub entry_date: NaiveDate,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub visibility: String, // "Private", "Project Visible", "Fellowship Visible"
    pub author_username: String,
}
