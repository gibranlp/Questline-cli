// ─────────────────────────────────────────────────────────────────────────────
// models/chapter.rs — el struct del capítulo para las quests cooperativas globales
// ─────────────────────────────────────────────────────────────────────────────
use serde::{Deserialize, Serialize};

pub struct ChapterObjectiveDef {
    pub id: &'static str,
    pub name: &'static str,
    pub target: u64,
}

pub struct Chapter {
    pub id: &'static str,
    pub title: &'static str,
    pub lore: &'static str,
    pub call_to_arms: &'static str,
    pub objectives: &'static [ChapterObjectiveDef],
    pub reward_lore_ids: &'static [&'static str],
    pub completion_text: &'static str,
    pub start_date: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterObjectiveProgress {
    pub id: String,
    pub name: String,
    pub current: u64,
    pub target: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterProgressData {
    pub chapter_id: String,
    pub objectives: Vec<ChapterObjectiveProgress>,
    pub completed: bool,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterHistoryEntry {
    pub chapter_id: String,
    pub title: String,
    pub completed_at: String,
    pub personal_contribution: u64,
}

static CHAPTER_ONE_OBJECTIVES: &[ChapterObjectiveDef] = &[
    ChapterObjectiveDef { id: "tasks_completed",     name: "Complete Quests",          target: 1000 },
    ChapterObjectiveDef { id: "subtasks_completed",  name: "Complete Steps",           target: 2000 },
    ChapterObjectiveDef { id: "scrolls_created",     name: "Create Scrolls",           target: 1000 },
    ChapterObjectiveDef { id: "focus_sessions",      name: "Complete Focus Sessions",  target: 500  },
    ChapterObjectiveDef { id: "tree_waterings",      name: "Water the Zen Tree",       target: 2000 },
    ChapterObjectiveDef { id: "rituals_completed",   name: "Complete Sidequests",      target: 300  },
    ChapterObjectiveDef { id: "reflections_written", name: "Write Reflections",        target: 750  },
];

static CHAPTER_ONE_REWARD_IDS: &[&str] = &["world_chapter_11", "memory_ch1_001"];

pub static CHAPTER_ONE: Chapter = Chapter {
    id: "chapter_one",
    title: "Chapter One: The Notification Swarm",
    lore: "Long before the Orders tracked every task and timed every session, a quieter world existed.\n\nNotifications were rare.\n\nMost were helpful.\n\nSome were urgent.\n\nThen something changed.\n\nNo one remembers exactly when the Swarm began.\n\nThe first signs were subtle.\n\nA single red circle appearing where there had been none.\n\nA banner arriving for a task that did not require one.\n\nA reminder about a reminder.\n\nThen the numbers grew.\n\nPings multiplied.\n\nBadges propagated.\n\nAlerts arrived to inform heroes that other alerts had arrived.\n\nThe Notification Sprites had been messengers once.\n\nHarmless creatures carrying messages across the Realm.\n\nBut something fed them.\n\nSomething nurtured their numbers beyond any natural limit.\n\nWithout intervention, the Swarm would consume all remaining attention in the Realm.\n\nThe Orders have convened.\n\nThe diagnosis is clear.\n\nThe remedy is simple.\n\nHeroes must begin working again.",
    call_to_arms: "The Orders call upon all heroes across the Realm.\n\nComplete your quests and their steps.\n\nHonor your focus sessions.\n\nNurture the Zen Tree.\n\nFulfill your sidequests.\n\nRecord your reflections.\n\nEvery act of meaningful progress weakens the Swarm.\n\nThe Realm is counting on you.",
    objectives: CHAPTER_ONE_OBJECTIVES,
    reward_lore_ids: CHAPTER_ONE_REWARD_IDS,
    completion_text: "The Notification Swarm has been dispersed. Heroes across the Realm completed the Chapter.",
    start_date: "2026-06-25",
};

pub static ALL_CHAPTERS: &[&Chapter] = &[&CHAPTER_ONE];

pub fn get_active_chapter() -> Option<&'static Chapter> {
    ALL_CHAPTERS.last().copied()
}
