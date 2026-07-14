use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenTree {
    pub id: Uuid,
    pub growth: i32,
    pub health: i32,
    pub stage: i32,
    pub last_watered: Option<DateTime<Utc>>,
    pub water_today: i32,
    pub total_waterings: i32,
}

impl ZenTree {
    pub fn stage_name(&self) -> &'static str {
        match self.stage {
            1 => "Acorn",
            2 => "Sprout",
            3 => "Young Entling",
            4 => "Grove Guardian",
            5 => "Ancient Ent",
            6 => "Evergrowth Elder",
            _ => "Evergrowth Tree",
        }
    }

    pub fn ascii_art(&self) -> &'static str {
        Self::ascii_art_at_stage(self.stage)
    }

    pub fn ascii_art_at_stage(stage: i32) -> &'static str {
        match stage {
            1 => "    .\n   ( )\n",
            2 => "    .\n   \\|/\n    |\n    '\n",
            3 => "    ,\n   \\|/\n  --|--\n    |\n   / \\\n",
            4 => "    .^.\n   /^^\\  \n  <^^^^>\n    ||\n    ||\n   /__\\\n",
            5 => "     .-^^^-.\n   .^^^^^^^^^.\n  <^^^^^^^^^^^>\n <^^^^^^^^^^^^^>\n      ||||\n      ||||\n     /||||\\\n",
            6 => "        .-^^^^^-.\n     .^^^^^^^^^^^^.\n   .^^^^^^^^^^^^^^^^.\n  <^^^^^^^^^^^^^^^^^^>\n <^^^^^^^^^^^^^^^^^^^^>\n<^^^^^^^^^^^^^^^^^^^^^^>\n        ||||||\n        ||||||\n      __||||||__\n",
            _ => "          .-^^^^^^^^^-.\n      .^^^^^^^^^^^^^^^^^^.\n    .^^^^^^^^^^^^^^^^^^^^^^.\n  .^^^^^^^^^^^^^^^^^^^^^^^^^^.\n <^^^^^^^^^^^^^^^^^^^^^^^^^^^^>\n<^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^>\n <^^^^^^^^^^^^^^^^^^^^^^^^^^^^>\n   \\^^^^^^^^^^^^^^^^^^^^^^^^/\n        ||||||||||||\n        ||||||||||||\n     ___||||||||||||___\n",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyAdventure {
    pub id: Uuid,
    pub title: String,
    pub quest_type: String, // valores posibles: "complete_tasks", "write_note", "write_journal", "water_tree", "complete_high_priority_task"
    pub target_count: i32,
    pub current_count: i32,
    pub completed: bool,
    pub created_date: NaiveDate,
}

impl DailyAdventure {
    pub fn generate_daily_quests(today: NaiveDate) -> Vec<Self> {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();

        let mut pool = vec![
            ("Complete 3 Tasks", "complete_tasks", 3),
            ("Write 1 Scroll", "write_note", 1),
            ("Write 1 Journal Entry", "write_journal", 1),
            ("Water Your Tree", "water_tree", 1),
            (
                "Complete 1 High Priority Quest",
                "complete_high_priority_task",
                1,
            ),
        ];

        pool.shuffle(&mut rng);
        pool.into_iter()
            .take(3)
            .map(|(title, quest_type, target_count)| DailyAdventure {
                id: Uuid::new_v4(),
                title: title.to_string(),
                quest_type: quest_type.to_string(),
                target_count,
                current_count: 0,
                completed: false,
                created_date: today,
            })
            .collect()
    }
}

// Este struct fue creciendo por etapas del roadmap — los campos de Stage 4 y 5A se agregaron después, no son legacy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub tasks_completed: i32,
    pub notes_created: i32,
    pub journal_entries: i32,
    pub projects_created: i32,
    pub current_streak: i32,
    pub best_streak: i32,
    pub tree_growth: i32,
    pub achievements_unlocked: i32,
    pub total_xp_earned: i32,
    pub focus_hours: f64,
    pub sessions_completed: i32,
    pub rituals_completed: i32,
    pub projects_completed: i32,
    pub milestones_completed: i32,
    pub most_productive_day: String,
    pub avg_tasks_per_day: f64,
    pub avg_xp_per_day: f64,

    pub sync_count: i32,
    pub backup_count: i32,
    pub devices_connected: i32,
    pub active_devices: i32,
    pub last_restore: String,
    pub conflict_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Streak {
    pub id: String,
    pub current_streak: i32,
    pub best_streak: i32,
    pub last_active_day: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub unlocked_at: Option<DateTime<Utc>>,
}

impl Achievement {
    pub fn static_list() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![
            ("first_quest", "First Quest", "Complete first task."),
            ("scholar", "Scholar", "Create 25 notes."),
            ("chronicler", "Chronicler", "Create 50 journal entries."),
            ("project_master", "Project Master", "Complete 10 projects."),
            (
                "ancient_gardener",
                "Ancient Gardener",
                "Grow tree to Stage 5.",
            ),
            (
                "hundred_day_journey",
                "Hundred Day Journey",
                "Reach 100-day streak.",
            ),
            (
                "first_focus",
                "First Focus",
                "Complete first focus session.",
            ),
            ("deep_worker", "Deep Worker", "Complete 100 focus sessions."),
            (
                "master_concentration",
                "Master of Concentration",
                "Complete 500 focus sessions.",
            ),
            (
                "ninety_minute_sage",
                "90 Minute Sage",
                "Complete a 90-minute session.",
            ),
            (
                "silent_monk",
                "Silent Monk",
                "Complete 25 focus sessions in silence.",
            ),
            (
                "forest_wanderer",
                "Forest Wanderer",
                "Complete 50 focus sessions with Forest Sounds.",
            ),
            (
                "rain_listener",
                "Rain Listener",
                "Complete 50 focus sessions with Rain Sounds.",
            ),
            (
                "master_atmosphere",
                "Master of Atmosphere",
                "Complete focus sessions with all 8 soundscapes.",
            ),
            (
                "first_companion",
                "First Companion",
                "Join first shared project.",
            ),
            (
                "quest_together",
                "Quest Together",
                "Complete project with another user.",
            ),
            (
                "chronicler_fellowship",
                "Chronicler of Fellowship",
                "Post 100 Chronicle messages.",
            ),
            ("mentor", "Mentor", "Invite 10 users."),
            (
                "alliance_builder",
                "Alliance Builder",
                "Participate in 25 shared projects.",
            ),
            (
                "milestone_first_quest",
                "Reluctant Hero",
                "You completed a task, wrote a note, and acknowledged the project existed for at least one day. The bar was on the floor. You found it.",
            ),
            (
                "milestone_chronicle_keeper",
                "Amateur Historian",
                "You showed up on two different days and wrote about it. Future archaeologists will be politely unimpressed. History is indeed written by those who show up — just not always read by anyone.",
            ),
            (
                "milestone_focused_adventurer",
                "Accidental Monk",
                "Three focus sessions without rage-quitting. Your attention span has outlasted most relationships and several national governments. This was not the plan. It worked anyway.",
            ),
            (
                "milestone_realm_builder",
                "Management Material",
                "Ten tasks completed. Seven days of project age. Five days of actual presence. You have officially done more structured follow-through than most people accomplish in a calendar quarter. This is now a personality trait. Accept it.",
            ),
            (
                "milestone_keeper_of_chronicle",
                "Unnecessary Biographer",
                "Fifteen tasks. Five journal entries. Seven active days over at least a week. You have documented a project that probably no one asked about with the dedication of someone who absolutely did not need to ask. Thorough. Possibly alarming.",
            ),
            (
                "milestone_steady_hero",
                "Creature of Habit",
                "A seven-day streak. Twenty completed tasks. Ten daily adventures. You are either genuinely productive or you have confused discipline for a coping mechanism. Both explanations are statistically consistent with the evidence.",
            ),
            (
                "milestone_master_of_realms",
                "Probably Fine",
                "Fifty tasks. Twenty notes. Twenty active days. Thirty days of project age. You have built something large enough that you can no longer remember where it started. The paperwork is, however, immaculate. Carry on.",
            ),
            (
                "milestone_legend_of_chronicle",
                "Unsolicited Archivist",
                "One hundred tasks. Twenty-five journal entries. Thirty active days on a project at least thirty days old. You have documented your productivity so thoroughly that your documentation now has its own subtext. Future scholars will cite you. They will not know why.",
            ),
            (
                "milestone_avatar_of_completion",
                "The Myth. The Legend. The Problem.",
                "One hundred tasks. Twenty-five daily adventures. A thirty-day streak. This is either exceptional discipline or a warning sign. Either way, the backlog has stopped trying to intimidate you. It now simply respects you from a cautious distance.",
            ),
            (
                "archivist",
                "Archivist",
                "Create 3 codices to organize your scrolls.",
            ),
            (
                "grand_archivist",
                "Grand Archivist",
                "Create 10 codices. Your lore is immaculately filed.",
            ),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSession {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub duration_mins: i32,
    pub xp_gained: i32,
    pub completed_at: DateTime<Utc>,
    pub soundscape: String,
    pub owner_identity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ritual {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub frequency: String,
    pub reward_xp: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub completed: bool,
    pub xp_reward: i32,
    pub created_at: DateTime<Utc>,
    // 0 = legacy/custom, 1 = Initiate, 2 = Veteran, 3 = Legendary — no hay un enum porque se serializa directo a SQLite
    pub tier: u8,
    // vacío ("") cuando el milestone es legacy o fue creado a mano, no desde un template
    pub template_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReflection {
    pub created_date: NaiveDate,
    pub what_went_well: String,
    pub what_can_improve: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn current() -> Self {
        use chrono::Datelike;
        let month = chrono::Local::now().month();
        match month {
            3..=5 => Season::Spring,
            6..=8 => Season::Summer,
            9..=11 => Season::Autumn,
            _ => Season::Winter,
        }
    }
}
