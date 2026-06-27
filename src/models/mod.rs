// ─────────────────────────────────────────────────────────────────────────────
// models/mod.rs — re-exporta todos los modelos de datos para usarlos fácil
// ─────────────────────────────────────────────────────────────────────────────
pub mod chapter;
pub mod codex;
pub mod daily_quest;
pub mod global_chronicle;
pub mod journal_entry;
pub mod note;
pub mod project;
pub mod rpg;
pub mod task;
pub mod user;
pub mod xp_event;

pub use codex::Codex;
pub use global_chronicle::GlobalChronicleEntry;
pub use daily_quest::DailyQuest;
pub use journal_entry::JournalEntry;
pub use note::Note;
pub use project::Project;
pub use rpg::{
    Achievement, DailyAdventure, DailyReflection, FocusSession, Milestone, Ritual, Season,
    Statistics, Streak, ZenTree,
};
pub use task::{Task, TaskPriority};
pub use user::{ClassType, User};
pub use xp_event::XPEvent;
