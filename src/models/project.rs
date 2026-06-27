// ─────────────────────────────────────────────────────────────────────────────
// models/project.rs — el struct de proyecto (reino)
// ─────────────────────────────────────────────────────────────────────────────
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Model representing a project bucket in Questline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub archived: bool,
    pub completed: bool,
    pub owner_identity: Option<String>,
    pub owner_username: Option<String>,
    pub is_shared: bool,
}
