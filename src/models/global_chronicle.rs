// ─────────────────────────────────────────────────────────────────────────────
// models/global_chronicle.rs — el struct de entrada del chronicle global del reino
// ─────────────────────────────────────────────────────────────────────────────
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalChronicleEntry {
    pub id: String,
    pub hero_name: String,
    pub event_type: String,
    pub description: String,
    pub timestamp: String,
}

impl GlobalChronicleEntry {
    pub fn is_anonymous(&self) -> bool {
        self.hero_name.is_empty() || self.hero_name == "The Realm"
    }

    pub fn icon(&self) -> &'static str {
        match self.event_type.as_str() {
            "LevelUp"           => "^",
            "RealmComplete"     => "[R]",
            "Milestone"         => "[M]",
            "Relic"             => "[+]",
            "Streak"            => "[~]",
            "Memory"            => "[#]",
            "MemoryFragment"    => "[#]",
            "Legend"            => "[L]",
            "DailyAdventure"    => "[D]",
            "ZenTree"           => "[T]",
            "TreeWatering"      => "[T]",
            "ChapterComplete"   => "[C]",
            "QuestComplete"     => "[Q]",
            "FocusSession"      => "[F]",
            "SidequestComplete" => "[*]",
            "ReflectionWritten" => "[W]",
            "ScrollCreated"     => "[S]",
            "ClassQuest"        => "[X]",
            "ClassStory"        => "[X]",
            "WorldLore"         => "[H]",
            "Achievement"       => "[A]",
            _                   => "o",
        }
    }
}
