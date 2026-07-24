// ─────────────────────────────────────────────────────────────────────────────
// services/lore_manager.rs — Descarga y cachea el lore desde questlinecli.com
//
// El lore ya no vive en el binario. Se descarga una vez por sesión y se persiste
// en disco (~/.questline/lore_cache.json). Si no hay red, se usa el caché local.
// Esto permite agregar nuevas entradas sin recompilar ni distribuir la app.
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const LORE_URL:   &str = "https://questlinecli.com/data/lore.json";
const QUESTS_URL: &str = "https://questlinecli.com/data/quests.json";

// Tiempo máximo de vida del caché en segundos (1 hora)
const CACHE_TTL_SECS: i64 = 3_600;

// ── Estructuras del JSON ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreUnlock {
    #[serde(rename = "type")]
    pub unlock_type:  String,
    pub level:        Option<i32>,
    pub class:        Option<String>,
    pub milestone_id: Option<String>,
    pub chapter_id:   Option<String>,
    pub display:      Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoreEntry {
    pub id:          String,
    pub category:    String,
    pub title:       String,
    pub content:     String,
    pub class_filter:Option<String>,
    pub unlock:      LoreUnlock,
    pub rarity:      Option<String>,
    pub sort_order:  i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    #[serde(rename = "type")]
    pub obj_type: String,
    pub target:   i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassQuest {
    pub class:          String,
    pub level:          i32,
    pub name:           String,
    pub description:    String,
    pub objective:      QuestObjective,
    pub lore_reward:    String,
    pub reward_lore_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoreFile {
    entries: Vec<LoreEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuestsFile {
    quests: Vec<ClassQuest>,
}

// Estructura que se guarda en disco como caché unificado
#[derive(Debug, Serialize, Deserialize)]
struct LoreCache {
    fetched_at: i64,  // unix timestamp
    lore:       Vec<LoreEntry>,
    quests:     Vec<ClassQuest>,
}

pub struct LoreManager {
    pub lore:   Vec<LoreEntry>,
    pub quests: Vec<ClassQuest>,
}

fn quest(
    class: &str,
    level: i32,
    name: &str,
    description: &str,
    target: i32,
    reward: &str,
) -> ClassQuest {
    let reward_lore_id = class_story_key(class)
        .map(|key| format!("class_{}_{}", key, level))
        .unwrap_or_else(|| format!("quest_story_{}_{}", class.to_lowercase().replace(' ', "_"), level));

    ClassQuest {
        class: class.to_string(),
        level,
        name: name.to_string(),
        description: description.to_string(),
        objective: QuestObjective {
            obj_type: "tracked_activity".to_string(),
            target,
        },
        lore_reward: reward.to_string(),
        reward_lore_id,
    }
}

fn class_story_key(class: &str) -> Option<&'static str> {
    match class {
        "Code Warlock" => Some("warlock"),
        "Task Paladin" => Some("paladin"),
        "Mind Sage" => Some("sage"),
        "Systems Architect" => Some("architect"),
        "Time Chronomancer" => Some("chronomancer"),
        "Arch Accountant" => Some("accountant"),
        _ => None,
    }
}

fn story(class: &str, level: i32, title: &str, content: String, sort_order: i32) -> LoreEntry {
    let class_key = class_story_key(class).unwrap_or("unknown");
    LoreEntry {
        id: format!("class_{}_{}", class_key, level),
        category: "Class".to_string(),
        title: title.to_string(),
        content,
        class_filter: Some(class.to_string()),
        unlock: LoreUnlock {
            unlock_type: "class_quest".to_string(),
            level: Some(level),
            class: Some(class.to_string()),
            milestone_id: None,
            chapter_id: None,
            display: Some(format!("Complete the Level {} {} class quest.", level, class)),
        },
        rarity: None,
        sort_order,
    }
}

// Catálogo base de quests de clase: diez pruebas por clase, una cada diez niveles.
fn built_in_class_quests() -> Vec<ClassQuest> {
    let specs: &[(&str, &[(&str, &str, &str)])] = &[
        ("Code Warlock", &[
            ("The Forgotten Compiler", "Complete 5 quests to align the compiler parameters and purge syntax anomalies.", "Class Quest XP and Warlock task-reward calibration."),
            ("The Log Lanterns", "Complete 5 daily adventures to light the logs before they become folklore.", "Permanent Warlock note rewards improve."),
            ("The Broken Daemon", "Complete 120 focus minutes to debug and stabilize the rogue background daemon.", "Permanent Warlock focus rewards improve."),
            ("The Patch Without a Ticket", "Log 8 glasses of water to survive the emergency patch window.", "Permanent Warlock task rewards improve."),
            ("The Library of Infinite Scripts", "Water The Evergrowth 3 times to grow script-bearing leaves.", "Permanent Warlock project rewards improve."),
            ("The Daemon of Refactoring", "Write 3 journal entries to record what changed before everyone forgets.", "Permanent Warlock note and journal rewards improve."),
            ("The Rollback Rite", "Complete 5 sidequests to clean the forgotten maintenance queue.", "Permanent Warlock passive rewards improve."),
            ("The Dependency Hex", "Complete 3 milestones to bind the dependency graph.", "Permanent Warlock milestone rewards improve."),
            ("The Production Window", "Complete 1 campaign before the deployment window closes.", "Permanent Warlock project rewards improve."),
            ("The Simulation Core", "Maintain streak progress 7 times to boot the final simulation engine.", "Unlocks the title Keeper of Working Systems and a permanent Warlock XP modifier."),
        ]),
        ("Task Paladin", &[
            ("The Mountain of Unfinished Things", "Complete 5 quests to clear the pass of procrastination monsters.", "Class Quest XP and Paladin task-reward calibration."),
            ("The Oath Renewed", "Complete 5 daily adventures without abandoning the day's promises.", "Permanent Paladin daily adventure rewards improve."),
            ("The Keeper of Deadlines", "Complete 120 focus minutes to reinforce the fortress deadlines.", "Permanent Paladin focus rewards improve."),
            ("The Hydrated Vigil", "Log 8 glasses of water to hold the wall without fading.", "Permanent Paladin task rewards improve."),
            ("The Final Checklist", "Water The Evergrowth 3 times to bless the roots of completion.", "Permanent Paladin tree and task rewards improve."),
            ("The Ledger of Finished Work", "Write 3 journal entries proving the work actually happened.", "Permanent Paladin journal rewards improve."),
            ("The Shield of Discipline", "Complete 5 sidequests to polish the shield against distraction.", "Permanent Paladin sidequest rewards improve."),
            ("The Banner of Milestones", "Complete 3 milestones to plant banners on real progress.", "Permanent Paladin milestone rewards improve."),
            ("The Last Gate", "Complete 1 campaign to prove the Order finishes the large thing too.", "Permanent Paladin project rewards improve."),
            ("The Citadel of Completion", "Maintain streak progress 7 times to defend the Citadel.", "Unlocks the title Avatar of Completion and a permanent Paladin XP modifier."),
        ]),
        ("Mind Sage", &[
            ("The Silent Archive", "Complete 5 quests to index the scrolls in the quiet archive.", "Class Quest XP and Sage task-reward calibration."),
            ("The Note That Became Useful", "Complete 5 daily adventures so knowledge leaves the shelf.", "Permanent Sage daily adventure rewards improve."),
            ("The Crystal of Reflection", "Complete 120 focus minutes to charge the crystal of inner sight.", "Permanent Sage focus rewards improve."),
            ("The Clear Water Index", "Log 8 glasses of water to keep the Archive's lamps bright.", "Permanent Sage note rewards improve."),
            ("The Hall of Thoughts", "Water The Evergrowth 3 times to nourish the branches of knowledge.", "Permanent Sage memory-fragment rewards improve."),
            ("The Reflection Chain", "Write 3 journal entries to link the day to its lesson.", "Permanent Sage journal rewards improve."),
            ("The Inbox Reckoning", "Complete 5 sidequests to clarify what the Archive was avoiding.", "Permanent Sage passive rewards improve."),
            ("The Cognitive Cartography", "Complete 3 milestones to map assumptions into action.", "Permanent Sage milestone rewards improve."),
            ("The Useful Connection", "Complete 1 campaign to prove the map helped the journey.", "Permanent Sage project rewards improve."),
            ("The Singularity of Mind", "Maintain streak progress 7 times until archive, task, and present agree.", "Unlocks the title Lantern of the Silent Archive and a permanent Sage XP modifier."),
        ]),
        ("Systems Architect", &[
            ("The Blueprint of Babel", "Complete 5 quests to lay down the base schema of construction.", "Class Quest XP and Architect task-reward calibration."),
            ("The Ownership Line", "Complete 5 daily adventures to make responsibility visible.", "Permanent Architect daily adventure rewards improve."),
            ("The Pillars of Order", "Complete 120 focus minutes to reinforce the support pillars.", "Permanent Architect focus rewards improve."),
            ("The Maintenance Reservoir", "Log 8 glasses of water to cool the machinery of order.", "Permanent Architect project rewards improve."),
            ("The Grand Engine", "Water The Evergrowth 3 times to feed the engine cooling system.", "Permanent Architect tree and project rewards improve."),
            ("The System Log", "Write 3 journal entries to document the operating model.", "Permanent Architect journal rewards improve."),
            ("The Friction Audit", "Complete 5 sidequests to remove drag instead of renaming it.", "Permanent Architect sidequest rewards improve."),
            ("The Dependency Map", "Complete 3 milestones to prove the sequence works.", "Permanent Architect milestone rewards improve."),
            ("The Modular Framework", "Complete 1 campaign to connect the system's components.", "Permanent Architect project rewards improve."),
            ("The Unified Schema", "Maintain streak progress 7 times until the system survives repetition.", "Unlocks the title Keeper of the Unified Schema and a permanent Architect XP modifier."),
        ]),
        ("Time Chronomancer", &[
            ("The Broken Hourglass", "Complete 5 quests to collect the scattered sands of time.", "Class Quest XP and Chronomancer task-reward calibration."),
            ("The Calendar Reckoning", "Complete 5 daily adventures to spend the day where it matters.", "Permanent Chronomancer daily adventure rewards improve."),
            ("The Sands of Yesterday", "Complete 120 focus minutes to spin the threads of memory.", "Permanent Chronomancer focus rewards improve."),
            ("The Clear Hour", "Log 8 glasses of water to keep the timeline from blurring.", "Permanent Chronomancer focus rewards improve."),
            ("The Infinite Loop", "Water The Evergrowth 3 times to grow temporal leaves.", "Permanent Chronomancer tree rewards improve."),
            ("The Time Ledger", "Write 3 journal entries to record where the hours went.", "Permanent Chronomancer journal rewards improve."),
            ("The Meeting That Ended", "Complete 5 sidequests to close loops before they multiply.", "Permanent Chronomancer sidequest rewards improve."),
            ("The Protected Hour", "Complete 3 milestones to protect work that required sequencing.", "Permanent Chronomancer milestone rewards improve."),
            ("The Temporal Shield", "Complete 1 campaign to defend the chosen hour from invasion.", "Permanent Chronomancer project rewards improve."),
            ("The Eternal Timeline", "Maintain streak progress 7 times to keep continuity alive.", "Unlocks the title Keeper of Reality and a permanent Chronomancer XP modifier."),
        ]),
        ("Arch Accountant", &[
            ("The Ledger of Fate", "Complete 5 quests to reconcile the local ledger entries.", "Class Quest XP and Accountant task-reward calibration."),
            ("The Receipt Hunt", "Complete 5 daily adventures so today's work has a receipt.", "Permanent Accountant daily adventure rewards improve."),
            ("The Golden Ratio", "Complete 120 focus minutes to calculate the perfect balance.", "Permanent Accountant focus rewards improve."),
            ("The Clear Ledger", "Log 8 glasses of water because even the body has accounts payable.", "Permanent Accountant all-source rewards improve."),
            ("The Final Balance", "Water The Evergrowth 3 times to secure the growth dividend.", "Permanent Accountant tree rewards improve."),
            ("The Daily Close", "Write 3 journal entries to close the books on the day.", "Permanent Accountant journal rewards improve."),
            ("The Subscription Exorcism", "Complete 5 sidequests to settle recurring obligations.", "Permanent Accountant sidequest rewards improve."),
            ("The Audit Trail", "Complete 3 milestones to prove the numbers became reality.", "Permanent Accountant milestone rewards improve."),
            ("The Compound Interest Vault", "Complete 1 campaign to deposit progress into the Vault.", "Permanent Accountant project rewards improve."),
            ("The Ledger of Eternity", "Maintain streak progress 7 times to balance the infinite ledger.", "Unlocks the title Keeper of the Eternal Ledger and a permanent Accountant XP modifier."),
        ]),
    ];

    let levels = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    let targets = [5, 5, 120, 8, 3, 3, 5, 3, 1, 7];
    let mut quests = Vec::new();
    for (class, entries) in specs {
        for (idx, (name, description, reward)) in entries.iter().enumerate() {
            quests.push(quest(class, levels[idx], name, description, targets[idx], reward));
        }
    }
    quests
}

fn class_story_titles(class: &str) -> &'static [&'static str; 10] {
    match class {
        "Code Warlock" => &[
            "The Forgotten Compiler",
            "The Log Lanterns",
            "The Broken Daemon",
            "The Patch Without a Ticket",
            "The Library of Infinite Scripts",
            "The Daemon of Refactoring",
            "The Rollback Rite",
            "The Dependency Hex",
            "The Production Window",
            "The Simulation Core",
        ],
        "Task Paladin" => &[
            "The Mountain of Unfinished Things",
            "The Oath Renewed",
            "The Keeper of Deadlines",
            "The Hydrated Vigil",
            "The Final Checklist",
            "The Ledger of Finished Work",
            "The Shield of Discipline",
            "The Banner of Milestones",
            "The Last Gate",
            "The Citadel of Completion",
        ],
        "Mind Sage" => &[
            "The Silent Archive",
            "The Note That Became Useful",
            "The Crystal of Reflection",
            "The Clear Water Index",
            "The Hall of Thoughts",
            "The Reflection Chain",
            "The Inbox Reckoning",
            "The Cognitive Cartography",
            "The Useful Connection",
            "The Singularity of Mind",
        ],
        "Systems Architect" => &[
            "The Blueprint of Babel",
            "The Ownership Line",
            "The Pillars of Order",
            "The Maintenance Reservoir",
            "The Grand Engine",
            "The System Log",
            "The Friction Audit",
            "The Dependency Map",
            "The Modular Framework",
            "The Unified Schema",
        ],
        "Time Chronomancer" => &[
            "The Broken Hourglass",
            "The Calendar Reckoning",
            "The Sands of Yesterday",
            "The Clear Hour",
            "The Infinite Loop",
            "The Time Ledger",
            "The Meeting That Ended",
            "The Protected Hour",
            "The Temporal Shield",
            "The Eternal Timeline",
        ],
        "Arch Accountant" => &[
            "The Ledger of Fate",
            "The Receipt Hunt",
            "The Golden Ratio",
            "The Clear Ledger",
            "The Final Balance",
            "The Daily Close",
            "The Subscription Exorcism",
            "The Audit Trail",
            "The Compound Interest Vault",
            "The Ledger of Eternity",
        ],
        _ => &["Unknown Story"; 10],
    }
}

fn class_story_voice(class: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    match class {
        "Code Warlock" => (
            "The Code Warlocks did not inherit power. They inherited systems that were already running and explanations that had expired.",
            "Their elders taught that a spell was only a command with consequences, and that logs were the confessions of tools that had obeyed too literally.",
            "The Order keeps its humor because terror without humor becomes management, and management near a terminal has caused enough damage.",
            "So the Warlock reads the old warning, changes one thing at a time, and listens for the machine to admit what everyone else had been pretending not to know.",
        ),
        "Task Paladin" => (
            "The Task Paladins began wherever work had become too familiar to see and too heavy to lift.",
            "Their doctrine remained plain: name the quest, do the quest, return tomorrow. Anything more ornate had to prove it could survive breakfast.",
            "They did not hate complexity. They hated the ceremony people built around refusing to move.",
            "So the Paladin raises the small clean blade of action and cuts one honest path through the waiting work.",
        ),
        "Mind Sage" => (
            "The Mind Sages learned early that memory preserved without use becomes a museum of future mistakes.",
            "Their libraries were not built to impress visitors. They were built so that pain could become instruction before it became tradition.",
            "They distrusted both clutter and emptiness, because each can hide the question that should have changed the day.",
            "So the Sage returns to the root, writes what happened, writes what it taught, and then accepts the rude necessity of doing something about it.",
        ),
        "Systems Architect" => (
            "The Systems Architects were summoned when good intentions had become hallways that led into one another forever.",
            "They believed that every structure teaches behavior, even the structures pretending to be temporary until the next planning cycle.",
            "Their craft was not control for its own sake. It was mercy for the future person who would otherwise inherit a maze with inspirational labels.",
            "So the Architect names the boundary, draws the dependency, removes the decorative confusion, and leaves a system that can be used twice.",
        ),
        "Time Chronomancer" => (
            "The Time Chronomancers discovered that lost hours rarely vanish. They are usually recruited by other people's urgency.",
            "They studied clocks, calendars, delay, regret, and the suspicious way tomorrow accepts assignments without attending the meeting.",
            "Their magic was not speed. Speed only helps when the road is correct and the traveler has not forgotten why they began.",
            "So the Chronomancer protects the chosen hour, closes the open loop, and makes the future pay rent before moving in.",
        ),
        "Arch Accountant" => (
            "The Arch Accountants understood that every promise enters the ledger, whether or not anyone admits an account has been opened.",
            "They did not worship numbers. They respected numbers because numbers, unlike speeches, eventually ask where the work went.",
            "Their humor was dry because wet humor smears ink, and the Realm had already lost too much truth to decorative reporting.",
            "So the Accountant reconciles the day, finds the missing receipt, and balances intention against evidence.",
        ),
        _ => ("The Order opened its book.", "The page waited.", "The work answered.", "The story continued."),
    }
}

fn class_story_stage(level: i32) -> &'static str {
    match level {
        10 => "At the first threshold, the apprentice discovers that the Order does not ask for a grand identity. It asks for proof that one real thing can be finished without being disguised as preparation.",
        20 => "At the second threshold, the lesson moves from effort to continuity. A single victory is useful, but repeated care is where the Realm begins to trust the hand that holds the tool.",
        30 => "At the third threshold, focus becomes a visible discipline. The old enemy does not always roar; sometimes it merely offers one more interruption with excellent manners.",
        40 => "At the fourth threshold, the body is admitted into the story. The Orders learned, after several embarrassing collapses, that heroic neglect is still neglect with better lighting.",
        50 => "At the fifth threshold, the Evergrowth answers. The work is no longer only personal. Roots remember what the calendar forgets, and leaves keep quiet records of steady attention.",
        60 => "At the sixth threshold, reflection is required. The day must be closed honestly, because unexamined progress can repeat mistakes with impressive confidence.",
        70 => "At the seventh threshold, the abandoned edges return. Small obligations, postponed too long, begin to rule from the margins until someone gives them a proper ending.",
        80 => "At the eighth threshold, milestones become gates instead of decorations. The apprentice learns that a marker is not a celebration of distance imagined, but evidence of ground actually crossed.",
        90 => "At the ninth threshold, the Order asks for a larger completion. Campaigns test whether discipline can survive scale, coordination, fatigue, and the charming lie that almost finished is finished enough.",
        100 => "At the final threshold, the class ceases to be a costume and becomes a title earned in public. The old trial ends, but its law remains in the hands, quieter and harder to lose.",
        _ => "The threshold was old, and the lesson waited patiently for someone tired enough to need it.",
    }
}

fn class_story_content(class: &str, level: i32, title: &str) -> String {
    let (opening, doctrine, warning, closing) = class_story_voice(class);
    let stage = class_story_stage(level);
    format!(
        "{opening}\n\nIn the records of the Six Orders, the tale called \"{title}\" is copied in a firm hand and surrounded by marginal notes from people who survived similar confidence. It is not presented as myth because myth is too comfortable. It is presented as a field report with candles, witnesses, and one sentence underlined by an archivist who clearly had opinions.\n\n{stage}\n\n{doctrine}\n\nThe tale says the apprentice did not win by becoming more dramatic. That would have pleased the bards and helped no one. The apprentice won by returning to the work after the first clean gesture had stopped feeling like destiny. The Realm noticed this. The Great Backlog noticed too, and shifted in the dark with the irritated patience of a thing that prefers people to confuse emotion with motion.\n\n{warning}\n\nWhen the quest was complete, the Order did not announce that the apprentice was finished. The Orders are old enough to know better. They opened a new page, wrote the date, named the proof, and left space beneath it for the next act.\n\n{closing}"
    )
}

// Semilla local de historias: una historia por cada quest de clase.
fn built_in_class_stories() -> Vec<LoreEntry> {
    let classes = [
        "Code Warlock",
        "Task Paladin",
        "Mind Sage",
        "Systems Architect",
        "Time Chronomancer",
        "Arch Accountant",
    ];
    let levels = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
    let mut stories = Vec::new();

    for (class_idx, class) in classes.iter().enumerate() {
        let titles = class_story_titles(class);
        for (idx, level) in levels.iter().enumerate() {
            let title = titles[idx];
            stories.push(story(
                class,
                *level,
                title,
                class_story_content(class, *level, title),
                1_000 + (class_idx as i32 * 100) + *level,
            ));
        }
    }

    stories
}

fn merge_with_built_in_lore(mut lore: Vec<LoreEntry>) -> Vec<LoreEntry> {
    let built_ins = built_in_class_stories();
    let built_in_ids: std::collections::HashSet<String> =
        built_ins.iter().map(|entry| entry.id.clone()).collect();
    lore.retain(|entry| !built_in_ids.contains(&entry.id));
    lore.extend(built_ins);
    lore
}

fn merge_with_built_in_quests(mut quests: Vec<ClassQuest>) -> Vec<ClassQuest> {
    let built_ins = built_in_class_quests();
    let built_in_classes: std::collections::HashSet<String> =
        built_ins.iter().map(|q| q.class.clone()).collect();
    quests.retain(|q| {
        !built_in_classes.contains(&q.class)
    });
    quests.extend(built_ins);
    quests
}

impl LoreManager {
    // Ruta del caché en disco — mismo directorio que la DB (~/.config/questline/)
    fn cache_path() -> PathBuf {
        crate::storage::get_storage_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("lore_cache.json")
    }

    // Lee el caché local; devuelve None si está expirado o no existe
    fn load_cache() -> Option<LoreCache> {
        let path = Self::cache_path();
        let data = std::fs::read_to_string(&path).ok()?;
        let cache: LoreCache = serde_json::from_str(&data).ok()?;

        let age = Utc::now().timestamp() - cache.fetched_at;
        if age > CACHE_TTL_SECS { return None; }

        Some(cache)
    }

    // Persiste el caché en disco — falla silenciosamente
    fn save_cache(lore: &[LoreEntry], quests: &[ClassQuest]) {
        let cache = LoreCache {
            fetched_at: Utc::now().timestamp(),
            lore:       lore.to_vec(),
            quests:     quests.to_vec(),
        };
        let path = Self::cache_path();
        if let Ok(json) = serde_json::to_string(&cache) {
            let _ = std::fs::write(&path, json);
        }
    }

    // Descarga un JSON desde la URL dada — timeout de 5 segundos para no bloquear el arranque
    fn fetch_json(url: &str) -> Result<String> {
        let resp = ureq::get(url)
            .timeout(std::time::Duration::from_secs(5))
            .call()
            .context("HTTP request failed")?;
        resp.into_string().context("Failed to read response body")
    }

    // Intenta descargar lore y quests; si falla devuelve los valores del caché o vacío
    pub fn load() -> Self {
        // Primero intenta usar el caché vigente para no bloquear el arranque
        if let Some(cache) = Self::load_cache() {
            // Lanza una descarga en background para refrescar el caché (fire-and-forget)
            std::thread::spawn(|| {
                let _ = Self::fetch_and_save();
            });
            return Self {
                lore: merge_with_built_in_lore(cache.lore),
                quests: merge_with_built_in_quests(cache.quests),
            };
        }

        // Sin caché válido: descarga bloqueante (solo ocurre la primera vez)
        match Self::fetch_and_save() {
            Ok((lore, quests)) => Self {
                lore: merge_with_built_in_lore(lore),
                quests: merge_with_built_in_quests(quests),
            },
            Err(_) => {
                Self {
                    lore: built_in_class_stories(),
                    quests: built_in_class_quests(),
                }
            }
        }
    }

    // Descarga ambos archivos, guarda el caché y devuelve los datos
    fn fetch_and_save() -> Result<(Vec<LoreEntry>, Vec<ClassQuest>)> {
        let lore_json   = Self::fetch_json(LORE_URL)?;
        let quests_json = Self::fetch_json(QUESTS_URL)?;

        let lore_file:   LoreFile   = serde_json::from_str(&lore_json).context("Invalid lore.json")?;
        let quests_file: QuestsFile = serde_json::from_str(&quests_json).context("Invalid quests.json")?;

        Self::save_cache(&lore_file.entries, &quests_file.quests);

        Ok((lore_file.entries, quests_file.quests))
    }
}
