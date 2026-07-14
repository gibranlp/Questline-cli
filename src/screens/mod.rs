// ─────────────────────────────────────────────────────────────────────────────
// screens/mod.rs — el enum ActiveScreen y re-exports de todas las pantallas
// ─────────────────────────────────────────────────────────────────────────────
pub mod about;
pub mod archive;
pub mod character;
pub mod dashboard;
pub mod editor;
pub mod fellowship;
pub mod focus;
pub mod gateway;
pub mod great_chronicle;
pub mod intro;
pub mod prologue;
pub mod legends;
pub mod library;
pub mod onboarding;
pub mod project_workspace;
pub mod projects;
pub mod restore;
pub mod soundscapes;
pub mod sync;

// Enum defining the active UI screen states in Questline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveScreen {
    Intro,
    Gateway,
    Restore,
    Prologue,
    Onboarding,
    Dashboard,
    Projects,
    Character,
    Library,
    Legends,
    Archive,
    Editor,
    Workspace,
    Focus,
    Soundscapes,
    SyncSettings,
    Fellowship,
    About,
    GreatChronicle,
}
