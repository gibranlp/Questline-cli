// ─────────────────────────────────────────────────────────────────────────────
// models/xp_event.rs — el struct que registra cada evento de XP ganado
// ─────────────────────────────────────────────────────────────────────────────
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Model representing a tracked progression event that rewards the player XP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XPEvent {
    pub id: Uuid,
    pub event_type: String,
    pub xp_gained: i32,
    pub timestamp: DateTime<Utc>,
}
