// ─────────────────────────────────────────────────────────────────────────────
// models/task.rs — el struct de tarea y su prioridad
// ─────────────────────────────────────────────────────────────────────────────
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestStatus {
    Backlog,
    Ready,
    InProgress,
    Blocked,
    Review,
    Done,
}

impl QuestStatus {
    pub const ACTIVE: [Self; 5] = [
        Self::Backlog,
        Self::Ready,
        Self::InProgress,
        Self::Blocked,
        Self::Review,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Backlog => "Backlog",
            Self::Ready => "Ready",
            Self::InProgress => "InProgress",
            Self::Blocked => "Blocked",
            Self::Review => "Review",
            Self::Done => "Done",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Backlog => "Awaiting the Council",
            Self::Ready => "Ready for Adventure",
            Self::InProgress => "Quest Underway",
            Self::Blocked => "Path Obstructed",
            Self::Review => "Awaiting Judgment",
            Self::Done => "Conquered",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "Ready" => Self::Ready,
            "InProgress" => Self::InProgress,
            "Blocked" => Self::Blocked,
            "Review" => Self::Review,
            "Done" => Self::Done,
            _ => Self::Backlog,
        }
    }

    pub fn next_active(self) -> Self {
        let index = Self::ACTIVE
            .iter()
            .position(|status| *status == self)
            .unwrap_or(0);
        Self::ACTIVE[(index + 1) % Self::ACTIVE.len()]
    }

    pub fn previous_active(self) -> Self {
        let index = Self::ACTIVE
            .iter()
            .position(|status| *status == self)
            .unwrap_or(0);
        Self::ACTIVE[(index + Self::ACTIVE.len() - 1) % Self::ACTIVE.len()]
    }
}

#[cfg(test)]
mod quest_status_tests {
    use super::QuestStatus;

    #[test]
    fn quest_stances_cycle_without_claiming_done() {
        let mut status = QuestStatus::Backlog;
        let expected = [
            QuestStatus::Ready,
            QuestStatus::InProgress,
            QuestStatus::Blocked,
            QuestStatus::Review,
            QuestStatus::Backlog,
        ];
        for next in expected {
            status = status.next_active();
            assert_eq!(status, next);
        }
        assert_ne!(status, QuestStatus::Done);
    }

    #[test]
    fn quest_stance_wire_names_are_stable() {
        assert_eq!(QuestStatus::from_str("InProgress").name(), "InProgress");
        assert_eq!(QuestStatus::from_str("unknown"), QuestStatus::Backlog);
        assert_eq!(QuestStatus::Done.display_name(), "Conquered");
    }

    #[test]
    fn quest_stances_can_move_backward() {
        assert_eq!(
            QuestStatus::Blocked.previous_active(),
            QuestStatus::InProgress
        );
        assert_eq!(QuestStatus::Backlog.previous_active(), QuestStatus::Review);
    }
}

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

// Model representing a task, optionally bound to a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub due_date: Option<DateTime<Utc>>,
    // Retired scheduling field retained only for older sync payloads/backups.
    #[serde(default)]
    pub set_date: Option<DateTime<Utc>>,
    pub completed: bool,
    pub priority: TaskPriority,
    pub created_at: DateTime<Utc>,
    // updated_at — crítico para resolver conflictos entre dispositivos con Latest Edit Wins
    #[serde(default = "crate::models::default_sync_timestamp")]
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

impl Task {
    /// Converts the retired Set Date field into the canonical Due Date.
    /// `set_date` remains serialized temporarily for compatibility with older clients.
    pub fn normalize_schedule(&mut self) {
        if self.due_date.is_none() {
            self.due_date = self.set_date;
        }
        self.set_date = None;
    }
}
