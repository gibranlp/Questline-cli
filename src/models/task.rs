// ─────────────────────────────────────────────────────────────────────────────
// models/task.rs — el struct de tarea y su prioridad
// ─────────────────────────────────────────────────────────────────────────────
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Frecuencia de recurrencia — qué tan seguido se reinicia la tarea al completarse
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecurrenceType {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrenceType {
    pub fn name(&self) -> &'static str {
        match self {
            RecurrenceType::Daily => "Daily",
            RecurrenceType::Weekly => "Weekly",
            RecurrenceType::Monthly => "Monthly",
            RecurrenceType::Yearly => "Yearly",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Daily" => Some(RecurrenceType::Daily),
            "Weekly" => Some(RecurrenceType::Weekly),
            "Monthly" => Some(RecurrenceType::Monthly),
            "Yearly" => Some(RecurrenceType::Yearly),
            _ => None,
        }
    }
}

// Enum representing the priority levels for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

impl TaskPriority {
    // Returns display name.
    pub fn name(&self) -> &'static str {
        match self {
            TaskPriority::Low => "Low",
            TaskPriority::Medium => "Medium",
            TaskPriority::High => "High",
        }
    }

    // Parses priority from a string representation.
    pub fn from_str(s: &str) -> Self {
        match s {
            "Low" | "low" => TaskPriority::Low,
            "High" | "high" => TaskPriority::High,
            _ => TaskPriority::Medium,
        }
    }
}

fn default_now() -> DateTime<Utc> {
    Utc::now()
}

// Model representing a task, optionally bound to a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    #[serde(default)]
    pub set_date: Option<DateTime<Utc>>,
    pub completed: bool,
    pub priority: TaskPriority,
    pub created_at: DateTime<Utc>,
    // updated_at — crítico para resolver conflictos entre dispositivos con Latest Edit Wins
    #[serde(default = "default_now")]
    pub updated_at: DateTime<Utc>,
    pub owner_identity: Option<String>,
    pub owner_username: Option<String>,
    #[serde(default)]
    pub parent_task_id: Option<Uuid>,
    // xp_awarded — bandera permanente: una vez otorgado el XP, ya no se repite aunque se reabra la tarea
    #[serde(default)]
    pub xp_awarded: bool,
    // recurrence — si está definida, al completarse se genera una nueva copia de la tarea con fecha avanzada
    #[serde(default)]
    pub recurrence: Option<RecurrenceType>,
}
