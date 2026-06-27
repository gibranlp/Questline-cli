// ─────────────────────────────────────────────────────────────────────────────
// milestone_templates.rs — plantillas de milestones por tier: bronce, plata, oro y legendario
// ─────────────────────────────────────────────────────────────────────────────
/// Three-tier milestone template system for Questline.
///
/// Tiers: Initiate (1) → Veteran (2) → Legendary (3)
/// Each tier has predefined templates with specific requirements.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Initiate = 1,
    Veteran = 2,
    Legendary = 3,
}

impl Tier {
    pub fn name(self) -> &'static str {
        match self {
            Tier::Initiate => "Initiate",
            Tier::Veteran => "Veteran",
            Tier::Legendary => "Legendary",
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Tier::Initiate),
            2 => Some(Tier::Veteran),
            3 => Some(Tier::Legendary),
            _ => None,
        }
    }

    pub fn xp_range(self) -> &'static str {
        match self {
            Tier::Initiate => "150–250 XP",
            Tier::Veteran => "750–1000 XP",
            Tier::Legendary => "3000–5000 XP",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Tier::Initiate => "For those beginning their journey.",
            Tier::Veteran => "For seasoned adventurers with proven dedication.",
            Tier::Legendary => "For masters who have shaped their realm.",
        }
    }
}

/// A single requirement for a milestone template.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Req {
    ProjectAgeDays(i64),
    CompletedTasksInProject(i64),
    NotesInProject(i64),
    JournalEntriesInProject(i64),
    ActiveDaysInProject(i64),
    TotalCompletedTasks(i64),
    CurrentStreak(i64),
    FocusSessionsTotal(i64),
    DailyAdventuresCompleted(i64),
}

impl Req {
    pub fn label(self) -> String {
        match self {
            Req::ProjectAgeDays(n) => format!("Project Age: {} Day(s)", n),
            Req::CompletedTasksInProject(n) => format!("Tasks Completed (Project): {}", n),
            Req::NotesInProject(n) => format!("Notes Created (Project): {}", n),
            Req::JournalEntriesInProject(n) => format!("Chronicle Entries (Project): {}", n),
            Req::ActiveDaysInProject(n) => format!("Active Days (Project): {}", n),
            Req::TotalCompletedTasks(n) => format!("Total Quests Completed: {}", n),
            Req::CurrentStreak(n) => format!("Current Streak: {} Day(s)", n),
            Req::FocusSessionsTotal(n) => format!("Focus Sessions Total: {}", n),
            Req::DailyAdventuresCompleted(n) => format!("Daily Adventures Done: {}", n),
        }
    }

    pub fn target(self) -> i64 {
        match self {
            Req::ProjectAgeDays(n) => n,
            Req::CompletedTasksInProject(n) => n,
            Req::NotesInProject(n) => n,
            Req::JournalEntriesInProject(n) => n,
            Req::ActiveDaysInProject(n) => n,
            Req::TotalCompletedTasks(n) => n,
            Req::CurrentStreak(n) => n,
            Req::FocusSessionsTotal(n) => n,
            Req::DailyAdventuresCompleted(n) => n,
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Req::ProjectAgeDays(_) => "Project Age",
            Req::CompletedTasksInProject(_) => "Tasks Completed",
            Req::NotesInProject(_) => "Notes Created",
            Req::JournalEntriesInProject(_) => "Chronicle Entries",
            Req::ActiveDaysInProject(_) => "Active Days",
            Req::TotalCompletedTasks(_) => "Total Quests",
            Req::CurrentStreak(_) => "Current Streak",
            Req::FocusSessionsTotal(_) => "Focus Sessions",
            Req::DailyAdventuresCompleted(_) => "Daily Adventures",
        }
    }
}

/// A predefined milestone template with fixed requirements and rewards.
#[derive(Debug, Clone, Copy)]
pub struct MilestoneTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub tier: Tier,
    pub xp_reward: i32,
    pub requirements: &'static [Req],
    pub flavor_text: &'static str,
}

/// All 9 predefined milestone templates across 3 tiers.
pub const TEMPLATES: &[MilestoneTemplate] = &[
    // ─── Tier 1: Initiate ────────────────────────────────────────────────────
    MilestoneTemplate {
        id: "first_quest",
        name: "First Quest",
        description: "Take the first steps into your project.",
        tier: Tier::Initiate,
        xp_reward: 150,
        requirements: &[
            Req::ProjectAgeDays(1),
            Req::CompletedTasksInProject(1),
            Req::NotesInProject(1),
        ],
        flavor_text: "Every legend begins somewhere.",
    },
    MilestoneTemplate {
        id: "chronicle_keeper",
        name: "Chronicle Keeper",
        description: "Establish a record of your adventures.",
        tier: Tier::Initiate,
        xp_reward: 200,
        requirements: &[
            Req::JournalEntriesInProject(1),
            Req::CompletedTasksInProject(3),
            Req::ActiveDaysInProject(2),
        ],
        flavor_text: "History is written by those who show up.",
    },
    MilestoneTemplate {
        id: "focused_adventurer",
        name: "Focused Adventurer",
        description: "Demonstrate consistent focus in your work.",
        tier: Tier::Initiate,
        xp_reward: 250,
        requirements: &[
            Req::FocusSessionsTotal(3),
            Req::ActiveDaysInProject(2),
        ],
        flavor_text: "Clarity comes with consistent effort.",
    },
    // ─── Tier 2: Veteran ─────────────────────────────────────────────────────
    MilestoneTemplate {
        id: "realm_builder",
        name: "Realm Builder",
        description: "Prove your dedication by building out your project realm.",
        tier: Tier::Veteran,
        xp_reward: 750,
        requirements: &[
            Req::ProjectAgeDays(7),
            Req::CompletedTasksInProject(10),
            Req::NotesInProject(3),
            Req::ActiveDaysInProject(5),
        ],
        flavor_text: "A realm worth building takes time.",
    },
    MilestoneTemplate {
        id: "keeper_of_chronicle",
        name: "Keeper of the Chronicle",
        description: "Document your journey with dedication and depth.",
        tier: Tier::Veteran,
        xp_reward: 800,
        requirements: &[
            Req::ProjectAgeDays(7),
            Req::JournalEntriesInProject(5),
            Req::CompletedTasksInProject(15),
            Req::ActiveDaysInProject(7),
        ],
        flavor_text: "The chronicle awaits further deeds.",
    },
    MilestoneTemplate {
        id: "steady_hero",
        name: "Steady Hero",
        description: "Maintain discipline across your entire adventure.",
        tier: Tier::Veteran,
        xp_reward: 1000,
        requirements: &[
            Req::CurrentStreak(7),
            Req::TotalCompletedTasks(20),
            Req::DailyAdventuresCompleted(10),
        ],
        flavor_text: "Heroes are forged through daily discipline.",
    },
    // ─── Tier 3: Legendary ───────────────────────────────────────────────────
    MilestoneTemplate {
        id: "master_of_realms",
        name: "Master of Realms",
        description: "Achieve true mastery over your project's domain.",
        tier: Tier::Legendary,
        xp_reward: 3000,
        requirements: &[
            Req::ProjectAgeDays(30),
            Req::CompletedTasksInProject(50),
            Req::NotesInProject(20),
            Req::ActiveDaysInProject(20),
        ],
        flavor_text: "Mastery is the sum of ten thousand small victories.",
    },
    MilestoneTemplate {
        id: "legend_of_chronicle",
        name: "Legend of the Chronicle",
        description: "Become a legend through unwavering documentation.",
        tier: Tier::Legendary,
        xp_reward: 4000,
        requirements: &[
            Req::ProjectAgeDays(30),
            Req::JournalEntriesInProject(25),
            Req::TotalCompletedTasks(100),
            Req::ActiveDaysInProject(30),
        ],
        flavor_text: "Your story is now legend.",
    },
    MilestoneTemplate {
        id: "avatar_of_completion",
        name: "Avatar of Completion",
        description: "Transcend the mortal limits of productivity.",
        tier: Tier::Legendary,
        xp_reward: 5000,
        requirements: &[
            Req::TotalCompletedTasks(100),
            Req::DailyAdventuresCompleted(25),
            Req::CurrentStreak(30),
        ],
        flavor_text: "You have become the hero the world needed.",
    },
];

/// Live statistics for a project, used to evaluate requirements.
pub struct ProjectStats {
    pub project_age_days: i64,
    pub completed_tasks_in_project: i64,
    pub notes_in_project: i64,
    pub journal_entries_in_project: i64,
    pub active_days_in_project: i64,
    pub total_completed_tasks: i64,
    pub current_streak: i64,
    pub focus_sessions_total: i64,
    pub daily_adventures_completed: i64,
}

/// Progress toward a single requirement.
pub struct ReqProgress {
    pub label: String,
    pub current: i64,
    pub target: i64,
    pub met: bool,
}

/// Compute progress for every requirement in a template against the given stats.
pub fn compute_progress(requirements: &[Req], stats: &ProjectStats) -> Vec<ReqProgress> {
    requirements
        .iter()
        .map(|&req| {
            let current = match req {
                Req::ProjectAgeDays(_) => stats.project_age_days,
                Req::CompletedTasksInProject(_) => stats.completed_tasks_in_project,
                Req::NotesInProject(_) => stats.notes_in_project,
                Req::JournalEntriesInProject(_) => stats.journal_entries_in_project,
                Req::ActiveDaysInProject(_) => stats.active_days_in_project,
                Req::TotalCompletedTasks(_) => stats.total_completed_tasks,
                Req::CurrentStreak(_) => stats.current_streak,
                Req::FocusSessionsTotal(_) => stats.focus_sessions_total,
                Req::DailyAdventuresCompleted(_) => stats.daily_adventures_completed,
            };
            let target = req.target();
            ReqProgress {
                label: req.label(),
                current,
                target,
                met: current >= target,
            }
        })
        .collect()
}

/// Returns an iterator over all templates for the given tier.
pub fn templates_for_tier(tier: Tier) -> impl Iterator<Item = &'static MilestoneTemplate> {
    TEMPLATES.iter().filter(move |t| t.tier == tier)
}

/// Look up a template by its string id.
pub fn get_template_by_id(id: &str) -> Option<&'static MilestoneTemplate> {
    TEMPLATES.iter().find(|t| t.id == id)
}
