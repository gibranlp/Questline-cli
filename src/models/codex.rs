// ─────────────────────────────────────────────────────────────────────────────
// models/codex.rs — el struct del codex, que agrupa notas
// ─────────────────────────────────────────────────────────────────────────────
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Codex {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}
