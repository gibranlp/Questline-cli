// ─────────────────────────────────────────────────────────────────────────────
// models/note.rs — el struct de nota, nada complicado
// ─────────────────────────────────────────────────────────────────────────────
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Model representing a markdown note, optionally bound to a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub markdown_content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default = "default_sharing_permission")]
    pub sharing_permission: String, // "read_only", "editable", "collaborative"
    #[serde(default)]
    pub codex_id: Option<Uuid>,
    #[serde(default)]
    pub owner_identity: Option<String>,
}

fn default_sharing_permission() -> String { "collaborative".into() }
