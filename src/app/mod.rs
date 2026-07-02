// ─────────────────────────────────────────────────────────────────────────────
// app/mod.rs — el estado global de la app: datos cargados, pantalla activa y modales
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Timelike, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use rusqlite::params;
use std::cell::Cell;
use std::path::Path;
use uuid::Uuid;

use crate::database::Database;
use crate::milestone_templates::{self, ProjectStats};
use crate::models::{
    ClassType, DailyAdventure, DailyQuest, DailyReflection, FocusSession, JournalEntry, Milestone,
    Note, Project, RecurrenceType, Ritual, Task, TaskPriority, User, ZenTree,
};
use crate::screens::editor::EditorState;
use crate::screens::onboarding::OnboardingFocus;
use crate::screens::ActiveScreen;
use crate::services::{ThemeService, XPService};
use crate::theme::ThemeChoice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueDateType {
    None,
    Today,
    Tomorrow,
    InDays,
    Specific,
}

impl DueDateType {
    pub fn name(&self) -> &'static str {
        match self {
            DueDateType::None => "No Due Date",
            DueDateType::Today => "Today",
            DueDateType::Tomorrow => "Tomorrow",
            DueDateType::InDays => "In Days...",
            DueDateType::Specific => "Specific Date...",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchResultType {
    Project,
    Task,
    Note,
    JournalEntry,
    Achievement,
    Lore,
    ChronicleEntry,
}

impl SearchResultType {
    pub fn label(&self) -> &'static str {
        match self {
            SearchResultType::Project => "Project",
            SearchResultType::Task => "Task",
            SearchResultType::Note => "Note",
            SearchResultType::JournalEntry => "Journal",
            SearchResultType::Achievement => "Achievement",
            SearchResultType::Lore => "Lore",
            SearchResultType::ChronicleEntry => "Chronicle",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub result_type: SearchResultType,
    pub title: String,
    pub details: String,
    pub project_id: Option<uuid::Uuid>,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAction {
    pub name: &'static str,
    pub description: &'static str,
    pub shortcut: &'static str,
    pub id: &'static str,
}

// Todos los modales posibles de la app — hay más de 20, cada uno con su propio estado interno
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalType {
    // Sin modal activo — el estado base
    None,
    // Modal de capítulo completado — aparece cuando el capítulo cooperativo llega al 100%
    ChapterComplete,
    NewProject {
        name: String,
        desc: String,
        focus_idx: usize,
    },
    EditProject {
        id: Uuid,
        name: String,
        desc: String,
        focus_idx: usize,
    },
    NewTask {
        title: String,
        desc: String,
        desc_cursor: usize,
        priority: TaskPriority,
        due_date_type: DueDateType,
        due_date_val: String,
        focus_idx: usize,
        parent_task_id: Option<Uuid>,
        recurrence: Option<RecurrenceType>,
    },
    NewCodex {
        name: String,
    },
    RenameCodex {
        codex_id: Uuid,
        name: String,
    },
    EditTask {
        id: Uuid,
        title: String,
        desc: String,
        desc_cursor: usize,
        priority: TaskPriority,
        due_date_type: DueDateType,
        due_date_val: String,
        focus_idx: usize,
        step_selected_idx: usize,
        // true si esta tarea es un paso hijo de otra — cambia cómo se guarda
        is_step: bool,
        recurrence: Option<RecurrenceType>,
    },
    NewJournalEntry {
        content: String,
    },
    // Stage 4 Modal Additions
    CustomFocusDuration {
        input: String,
    },
    // Selector de tier de milestone — Bronze/Silver/Gold/etc antes de elegir template
    MilestoneTierSelect {
        project_id: Uuid,
        selected_idx: usize,
    },
    MilestoneTemplateSelect {
        project_id: Uuid,
        tier: u8,
        selected_idx: usize,
    },
    // Selector de especialización — se muestra una sola vez al subir cierto nivel
    SpecializationSelect {
        choices: Vec<String>,
        selected_idx: usize,
    },
    DailyReflection {
        what_went_well: String,
        what_can_improve: String,
        focus_idx: usize,
    },
    NewRitual {
        name: String,
        desc: String,
        frequency_idx: usize,
        reward_xp: String,
        focus_idx: usize,
    },
    EditServerUrl {
        input: String,
    },
    // Exporta la llave privada en base64 para transferirla a otro dispositivo — órale, con cuidado
    ExportProfile {
        transfer_code: String,
    },
    // Importa una identidad desde un transfer code — restaura al héroe en una máquina nueva
    RestoreIdentity {
        input: String,
    },
    LocalMusicFolder {
        input: String,
        suggestions: Vec<String>,
        selected: usize,
    },
    InviteMember {
        identity: String,
        username: String,
        role_idx: usize,
        project_idx: usize,
        focus_idx: usize,
    },
    ShareNote {
        note_id: Uuid,
        permission_idx: usize,
    },
    AssignTask {
        task_id: Uuid,
        selected_member_idx: usize,
    },
    JournalVisibility {
        entry_id: Uuid,
        visibility_idx: usize,
    },
    ProjectSharing {
        project_id: Uuid,
    },
    SearchMessages {
        query: String,
    },
    PostMessage {
        content: String,
    },
    AddReaction {
        message_id: Uuid,
    },
    // Selector de tema visual — cambia los colores de toda la UI al vuelo
    ThemeSelect {
        choices: Vec<String>,
        selected_idx: usize,
    },
    // Modal de celebración con ASCII art — para los level-ups y logros épicos
    Celebration {
        title: String,
        details: String,
        ascii_art: String,
    },
    // Búsqueda global: proyectos, tareas, notas, journal, logros y más en un solo lugar
    SearchEverywhere {
        query: String,
        selected_idx: usize,
        results: Vec<SearchResult>,
    },
    // Command palette style — type to filter actions, like VS Code's Ctrl+P but más chido
    CommandPalette {
        query: String,
        selected_idx: usize,
        actions: Vec<CommandAction>,
    },
    SelectProjectForAction {
        action_id: &'static str,
        selected_idx: usize,
    },
    KeyboardHelp,
    QuitConfirm {
        quote: String,
    },
    ConfirmArchiveProject {
        project_id: Uuid,
        project_name: String,
    },
    ConfirmDeleteProject {
        project_id: Uuid,
        project_name: String,
    },
    ConfirmDeleteCodex {
        codex_id: Uuid,
        codex_name: String,
    },
    RefileScroll {
        note_id: Uuid,
        selected_idx: usize,
    },
    UpdateAvailable {
        latest_version: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationKind {
    Info,    // azul  — informativo, todo bien
    Warning, // amarillo — advertencia, algo que el héroe debe saber
    Swarm,   // rojo  — los Notification Sprites atacan
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub kind:    NotificationKind,
    pub title:   String,
    pub unlocked_at: std::time::Instant,
}

impl Notification {
    pub fn info(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), kind: NotificationKind::Info, title: "Realm Bulletin".to_string(), unlocked_at: std::time::Instant::now() }
    }
    pub fn warning(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), kind: NotificationKind::Warning, title: "Realm Warning".to_string(), unlocked_at: std::time::Instant::now() }
    }
    pub fn swarm(msg: impl Into<String>, title: impl Into<String>) -> Self {
        Self { message: msg.into(), kind: NotificationKind::Swarm, title: title.into(), unlocked_at: std::time::Instant::now() }
    }
}

#[derive(Debug, Clone)]
pub struct FragmentAlert {
    pub number: String,
    pub rarity: String,
    pub attribution: String,
    pub shown_at: std::time::Instant,
}

// Pools de mensajes para los Notification Sprites — satíricos, en-lore, y a veces profundos

const SPRITE_MSG_USELESS: &[&str] = &[
    "A notification has arrived to inform you that another notification may arrive soon.",
    "A notification was successfully notified.",
    "Reminder: there are reminders waiting to remind you about other reminders.",
    "The Notification Swarm appreciates your continued attention.",
    "An update regarding a previous update has been scheduled.",
    "A banner would like to discuss another banner.",
    "The unread counter feels lonely.",
    "Several notifications are currently notifying one another.",
];
const SPRITE_MSG_SATIRE: &[&str] = &[
    "Someone somewhere organized their folders. You may now continue.",
    "A meeting has been scheduled to discuss a future meeting.",
    "The Scope Dragon has requested one more feature.",
    "Your task list has noticed you looking at it.",
    "An unread task has achieved self-awareness.",
    "Future You has requested immediate assistance.",
    "The Great Backlog likes where this is going.",
    "Tomorrow has submitted another extension request.",
    "The deadline remains unconvinced.",
];
const SPRITE_MSG_CLASS_LORE: &[&str] = &[
    "A Task Paladin completed something they started. The Realm is confused.",
    "A Code Warlock opened another terminal.",
    "A Mind Sage indexed an additional thought.",
    "A Chronomancer located seven extra minutes.",
    "An Arch Accountant balanced a ledger. Reality stabilizes.",
    "A Systems Architect created a folder for future folders.",
    "A Mind Sage has categorized a thought under \"Probably Important.\"",
    "An Architect has introduced a framework for organizing frameworks.",
];
const SPRITE_MSG_SPEAKING: &[&str] = &[
    "Hello. We noticed you were focusing. We have come to help.",
    "You appeared productive. We filed a report.",
    "Please stop completing tasks. You are weakening us.",
    "The Swarm respectfully requests procrastination.",
    "This notification could have been a completed task.",
    "The Swarm grows concerned. Progress has been detected.",
    "We are trying our best to distract you.",
    "Please return to scrolling.",
    "You were almost finished. We could not allow that.",
];
const SPRITE_MSG_TASK_COMPLETE: &[&str] = &[
    "The Swarm loses influence.",
    "Several Notification Sprites flee the area.",
    "Persistence deals critical damage.",
    "A task was completed. The Realm approves.",
    "The Great Backlog disliked that.",
    "Productivity has been reported to the authorities.",
    "The Swarm is visibly uncomfortable.",
    "Progress detected. Dispatching fewer Sprites.",
];
const SPRITE_MSG_RARE_CHRONICLE: &[&str] = &[
    "The Chronicle is watching. Keep going.",
    "One day this task will be a memory. Today it is simply work.",
    "Small steps built every legend recorded in the Chronicle.",
    "No great hero completed everything at once.",
    "The horizon remains distant because you returned today.",
    "Discipline rarely feels dramatic while it is happening.",
    "Most victories are invisible until they are complete.",
    "Every completed quest weakens something unseen.",
];

// cada pool de Sprites tiene su título absurdo — el título combina con el tono del mensaje
fn pick_sprite_message() -> (String, &'static str) {
    use rand::seq::SliceRandom;
    use rand::Rng;
    let mut rng = rand::thread_rng();
    if rng.r#gen::<f64>() < 0.008 {
        let msg = SPRITE_MSG_RARE_CHRONICLE.choose(&mut rng).copied().unwrap_or("").to_string();
        return (msg, "Final Reminder (Probably)");
    }
    let pools: &[(&[&str], &str)] = &[
        (SPRITE_MSG_USELESS,    "Notification Notification"),
        (SPRITE_MSG_SATIRE,     "Important Notification"),
        (SPRITE_MSG_CLASS_LORE, "Follow-up Notification"),
        (SPRITE_MSG_SPEAKING,   "Extremely Important Notification"),
    ];
    let (pool, title) = pools.choose(&mut rng).copied().unwrap_or((SPRITE_MSG_SPEAKING, "Important Notification"));
    (pool.choose(&mut rng).copied().unwrap_or("").to_string(), title)
}

fn pick_task_completion_sprite_message() -> (String, &'static str) {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let msg = SPRITE_MSG_TASK_COMPLETE.choose(&mut rng).copied().unwrap_or("").to_string();
    (msg, "Reminder About Previous Notification")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportType {
    Bug,
    Feature,
    Feedback,
}

impl ReportType {
    pub fn label(&self) -> &'static str {
        match self {
            ReportType::Bug => "Bug Report",
            ReportType::Feature => "Feature Request",
            ReportType::Feedback => "General Feedback",
        }
    }
    pub fn next(self) -> Self {
        match self {
            ReportType::Bug => ReportType::Feature,
            ReportType::Feature => ReportType::Feedback,
            ReportType::Feedback => ReportType::Bug,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            ReportType::Bug => ReportType::Feedback,
            ReportType::Feature => ReportType::Bug,
            ReportType::Feedback => ReportType::Feature,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BugReportModal {
    pub report_type: ReportType,
    pub description: String,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActiveFocusSession {
    pub start_time: DateTime<Utc>,
    pub duration_mins: i32,
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub soundscape: String,
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub x: u16,
    pub y: f32,
    pub speed: f32,
    pub symbol: char,
    pub color: ratatui::style::Color,
}

pub struct BackgroundSyncResult {
    pub pushed: usize,
    pub pulled: usize,
    pub conflicts: Vec<String>,
    pub error: Option<String>,
}

pub struct ChatPollResult {
    pub project_id: String,
    pub new_message_count: usize,
    pub last_timestamp: Option<String>,
    pub error: Option<String>,
}

// El estado central de toda la app — todo pasa por aquí, desde la DB hasta los modales
pub struct App {
    // Base de datos SQLite — la fuente de verdad de toda la información del héroe
    pub db: Database,
    // El usuario actual — None si no ha hecho onboarding todavía
    pub user: Option<User>,
    pub active_screen: ActiveScreen,

    // Onboarding Form States — campos del formulario de creación de héroe
    pub onboarding_username: String,
    pub onboarding_class_idx: usize,
    pub onboarding_focus: OnboardingFocus,
    pub onboarding_classes: Vec<ClassType>,
    pub onboarding_error: Option<String>,

    // Services
    pub theme_service: ThemeService,

    // Navigation and database states
    pub active_tab_idx: usize,
    // Quote del dashboard — se selecciona al iniciar y queda fija durante la sesión
    pub quote: String,
    pub quote_author: String,
    pub class_quote: Option<String>,
    pub class_quote_author: Option<String>,

    pub daily_quests: Vec<DailyQuest>,
    pub tasks_due_today: Vec<Task>,
    pub projects: Vec<Project>,
    pub should_quit: bool,

    // Stage 2 Core Workspace variables
    pub selected_project_idx: usize,
    pub active_project_id: Option<Uuid>,
    pub workspace_tab_idx: usize,
    pub workspace_sidebar_focused: bool,
    pub workspace_help_open: bool,
    pub selected_task_idx: usize,
    pub selected_note_idx: usize,
    pub selected_journal_idx: usize,
    pub selected_archive_idx: usize,
    pub selected_reflection_idx: usize,
    pub character_focus: usize, // 0 = Adventure Log Book, 1 = Reflection Entries list, 2 = Reflection Detail scroll
    pub reflection_detail_scroll: usize,

    // modal_state es el modal principal, overlay_modal va encima de él — se apilan
    pub modal_state: ModalType,
    pub overlay_modal: ModalType,
    pub editor_state: Option<EditorState>,

    // Dashboard quest panel navigation
    pub dashboard_task_focus: bool,
    pub selected_dashboard_task_idx: usize,

    // Local Search and Filters
    pub searching: bool,
    pub search_query: String,
    pub task_filter: String,
    pub task_sort: String,

    // Notifications overlay
    pub notifications: Vec<Notification>,
    pub fragment_notification: Option<FragmentAlert>,

    // Stage 4 Configuration variables
    pub active_focus_session: Option<ActiveFocusSession>,
    pub selected_focus_duration_idx: usize,
    pub selected_focus_project_idx: usize,
    pub selected_focus_task_idx: usize,
    pub selected_focus_field_idx: usize, // 0 = Duration, 1 = Project, 2 = Task, 3 = Soundscape

    pub selected_ritual_idx: usize,
    pub selected_milestone_idx: usize,

    // Soundscapes fields
    pub audio_player: crate::audio::AudioPlayer,
    pub selected_soundscape_idx: usize,
    pub selected_focus_soundscape_idx: usize, // 0 = None (Silent), 1..=SOUNDSCAPES.len() = specific soundscape

    // Stage 5A Sync & Identity Fields — cripto y sincronización con el servidor
    pub identity: crate::services::identity::Identity,
    // device_id es un UUID único por máquina — se genera una vez y se guarda en la DB
    pub device_id: String,

    pub server_url: String,
    pub auto_sync: bool,
    pub sync_status_msg: String,
    pub sync_conflicts: Vec<String>,
    pub sync_failure_count: u32,
    pub config: crate::services::Config,
    pub last_auto_sync: std::time::Instant,
    pub last_sync_status_time: Option<std::time::Instant>,
    // Registra cuándo fue la última mutación para decidir si hay que hacer auto-sync
    pub last_mutation: Option<std::time::Instant>,
    pub last_sync_warlock_xp: i32,

    // Stage 5B Fellowship & Collaboration Fields
    pub selected_fellowship_project_idx: usize,

    pub fellowship_search_query: String,
    pub selected_fellowship_tab: usize, // 0 = Projects/Chronicle, 1 = Invitations, 2 = Online Companions, 3 = Recent Activity, 4 = Search Messages
    pub selected_invitation_idx: usize,
    pub selected_notification_idx: usize,
    pub fellowship_chat_input: String,
    pub fellowship_selected_msg_idx: usize, // usize::MAX = input focused (bottom)
    pub fellowship_focus_left: bool,        // true = left project list has focus
    pub fellowship_composing: bool,         // true = in compose mode (like vim insert)
    pub about_scroll: u16,
    // Cell<u16> para poder mutar desde un contexto inmutable durante el render
    pub about_content_lines: Cell<u16>,
    pub terminal_height: u16,
    pub about_fact_seed: u64,
    pub bug_report_modal: Option<BugReportModal>,
    pub fellowship_search_results: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    )>,

    // Stage 6 Living World Fields
    pub selected_chronicle_idx: usize,
    pub selected_library_cat_idx: usize,
    pub selected_library_item_idx: usize,
    pub library_active_col: usize,
    pub library_scroll_offset: u16,
    pub selected_relic_idx: usize,
    pub ambient_effects_enabled: bool,
    pub active_ambient_effect: usize,
    pub ambient_particles: Vec<Particle>,
    pub ambient_particles_ticks_remaining: usize,
    pub corrupted_backups_found: Vec<String>,
    pub quit_confirm_ticks: usize,
    pub intro_ticks: usize,
    pub music_scroll_ticks: usize,

    // El hilo de fondo escribe aquí cuando termina de checar la versión más reciente
    pub update_check: std::sync::Arc<std::sync::Mutex<Option<String>>>,
    pub update_check_done: bool,
    pub run_installer_on_exit: bool,

    // v1.0.5 Steps & Codices
    pub codices: Vec<crate::models::Codex>,
    pub viewing_step_for_task: Option<Uuid>,
    // Flat selection index for notes tab (navigates headers + notes)
    pub selected_notes_flat_idx: usize,

    // Cachés de performance — se llenan en reload_data() para no golpear la DB en cada frame
    pub all_tasks: Vec<Task>,
    pub all_notes: Vec<Note>,
    pub all_journals: Vec<JournalEntry>,

    // El sync corre en un hilo aparte — este Arc/Mutex permite pasarle el resultado al hilo principal
    pub sync_in_progress: bool,
    pub sync_result: std::sync::Arc<std::sync::Mutex<Option<BackgroundSyncResult>>>,

    // Chat polling state — fast poll active only when fellowship chat tab is visible
    pub chat_poll_active: bool,
    pub chat_rx: Option<std::sync::mpsc::Receiver<ChatPollResult>>,
    pub last_chat_poll: std::time::Instant,
    pub last_chat_timestamp: std::collections::HashMap<String, String>,

    // Great Chronicle — feed global de logros de la comunidad de héroes
    pub great_chronicle_entries: Vec<crate::models::GlobalChronicleEntry>,
    pub great_chronicle_scroll: usize,

    // Living Chapters — sistema cooperativo global, todos contribuyen al mismo capítulo
    pub chapter_progress: Option<crate::models::chapter::ChapterProgressData>,
    pub chapter_panel_scroll: usize,
    pub chapter_tab: usize,           // 0 = Active Chapter, 1 = Chapter History
    pub chapter_panel_focused: bool,  // false = left feed, true = right chapter panel
    pub chapter_history: Vec<crate::models::chapter::ChapterHistoryEntry>,
    pub chapter_completion_seen: bool,
    pub chapter_progress_refreshed: std::sync::Arc<std::sync::atomic::AtomicBool>,

    // Notification Sprites — los personajes satiricos del Capítulo Uno
    pub sprite_notifications_shown_this_session: u32,
    pub last_sprite_notification_time: Option<std::time::Instant>,
    pub last_sprite_check_time: Option<std::time::Instant>,

    // Prologue — pantallas de historia con efecto typewriter que se muestran después del login
    pub prologue_page: u8,           // 0 = The Story So Far, 1 = Chapter One
    pub prologue_line_idx: usize,    // which line is currently being typed
    pub prologue_char_in_line: usize,// chars revealed in that line
    pub prologue_next_is_onboarding: bool, // where to go when prologue finishes
    pub prologue_delay_ticks: usize, // ticks to wait before typewriter starts (audio warm-up)
    pub prologue_skip_checked: bool, // "don't show again" checkbox state
}

pub fn extract_url(content: &str) -> Option<&str> {
    content.split_whitespace().find(|w| w.starts_with("http://") || w.starts_with("https://"))
}

// Arma la lista plana para el panel de tareas del dashboard — padres ordenados por fecha, cada uno seguido de sus pasos
fn dashboard_flat_items(all_tasks: &[Task]) -> Vec<(bool, Uuid, Task)> {
    let mut parents: Vec<&Task> = all_tasks
        .iter()
        .filter(|t| !t.completed && t.parent_task_id.is_none())
        .collect();
    parents.sort_by(|a, b| match (a.due_date, b.due_date) {
        (Some(d1), Some(d2)) => d1.cmp(&d2),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => b.created_at.cmp(&a.created_at),
    });
    let mut flat = Vec::new();
    for parent in parents {
        flat.push((false, parent.id, parent.clone()));
        let mut steps: Vec<&Task> = all_tasks
            .iter()
            .filter(|t| t.parent_task_id == Some(parent.id) && !t.completed)
            .collect();
        steps.sort_by_key(|s| s.created_at);
        for step in steps {
            flat.push((true, parent.id, step.clone()));
        }
    }
    flat
}

impl App {
    pub fn choose_dynamic_quote(
        user: &Option<User>,
        _db: &Database,
    ) -> (String, String, Option<(String, String)>) {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();

        let questline_pool = vec![
            (
                "The productivity app that finally admitted motivation was never coming.",
                "Questline",
            ),
            (
                "Turning obvious life advice into a role-playing game since yesterday.",
                "Questline",
            ),
            (
                "The world's most elaborate reminder to finish your tasks.",
                "Questline",
            ),
            (
                "Because apparently crossing things off a list releases chemicals.",
                "Questline",
            ),
            (
                "All the discipline. None of the enlightenment.",
                "Questline",
            ),
            (
                "A sophisticated system for repeatedly doing things you already knew you should do.",
                "Questline",
            ),
            (
                "Making productivity feel important enough to finally do it.",
                "Questline",
            ),
            (
                "A highly advanced system for avoiding the consequences of avoiding things.",
                "Questline",
            ),
            (
                "Proof that adding experience points makes humans do almost anything.",
                "Questline",
            ),
            (
                "If discipline were easy, this app wouldn't exist.",
                "Questline",
            ),
            (
                "Finally, a place where checking a box counts as heroism.",
                "Questline",
            ),
            (
                "Making responsible decisions look far more epic than they actually are.",
                "Questline",
            ),
            (
                "Because 'just do it' lacked proper worldbuilding.",
                "Questline",
            ),
            (
                "A game about productivity. Unfortunately, the productivity part is real.",
                "Questline",
            ),
            (
                "Helping ambitious people procrastinate less creatively.",
                "Questline",
            ),
            (
                "You are now entering a high-fantasy interpretation of personal responsibility.",
                "Questline",
            ),
            (
                "Making 'I'll do it tomorrow' a hostile creature since Version 1.0.",
                "Questline",
            ),
            (
                "Converting anxiety into progress since the First Cursor.",
                "Questline",
            ),
            (
                "Warning: may cause accidental self-improvement.",
                "Questline",
            ),
            (
                "The sacred ritual of pretending a task manager will solve your life.",
                "Questline",
            ),
            (
                "Built upon ancient productivity wisdom and modern panic.",
                "Questline",
            ),
            (
                "The path to greatness is surprisingly administrative.",
                "Questline",
            ),
            (
                "Not guaranteed to increase productivity. Guaranteed to make it look cooler.",
                "Questline",
            ),
            (
                "The closest thing to a save point in real life.",
                "Questline",
            ),
            (
                "Professional-grade procrastination resistance.",
                "Questline",
            ),
            (
                "Every legend starts the same way: with someone reluctantly opening their laptop.",
                "Questline",
            ),
            (
                "The reward for completing your tasks: more tasks.",
                "Questline",
            ),
            (
                "The fantasy RPG where the dragon is your inbox.",
                "Questline",
            ),
            (
                "The only RPG where the final boss is your own calendar.",
                "Questline",
            ),
            (
                "Thousands of years of human progress culminated in this: a better to-do list.",
                "Questline",
            ),
            (
                "The road to greatness begins with answering that email.",
                "Questline",
            ),
            (
                "The hero's journey, but with spreadsheets.",
                "Questline",
            ),
            (
                "No chosen one. No destiny. Just consistent effort and decent note-taking.",
                "Questline",
            ),
            (
                "The adventure begins where procrastination ends.",
                "Questline",
            ),
            (
                "Powered entirely by hope, caffeine, and unchecked task creation.",
                "Questline",
            ),
            (
                "Every day is a side quest until the deadline arrives.",
                "Questline",
            ),
            (
                "One small step for productivity. One giant leap away from doom-scrolling.",
                "Questline",
            ),
            (
                "The real treasure was the completed tasks we made along the way.",
                "Questline",
            ),
            (
                "Come for the quests. Stay because the backlog is gaining strength.",
                "Questline",
            ),
            (
                "The Great Backlog fears only two things: consistency and accidentally deleting the database.",
                "Questline",
            ),
            (
                "Your ancestors crossed oceans. You answered three emails.",
                "Questline",
            ),
            (
                "Remember: the backlog is doing pushups while you read this.",
                "Questline",
            ),
            (
                "You could be working right now.",
                "Questline",
            ),
            (
                "Eventually the tasks finish themselves. This is not that strategy.",
                "Questline",
            ),
            (
                "Achievement unlocked: Doing the thing.",
                "Questline",
            ),
            (
                "Leveling up your character because improving yourself sounded like too much work.",
                "Questline",
            ),
            (
                "The ancient art of doing your tasks before they become lore.",
                "Questline",
            ),
            (
                "Every task a quest. Every project an adventure. Every deadline a boss fight.",
                "Questline",
            ),
            (
                "Slayer of Scope Dragons. Defender of Deadlines. Destroyer of 'I'll do it tomorrow'.",
                "Questline",
            ),
            (
                "Where unfinished tasks become monsters and crossing them off becomes heroism.",
                "Questline",
            ),
            (
                "Saving the realm from chaos, one checkbox at a time.",
                "Questline",
            ),
            (
                "An RPG for the greatest battle of all: opening the document and actually starting.",
                "Questline",
            ),
            (
                "Every free trial eventually becomes a subscription.",
                "Auditor Prime",
            ),
            (
                "The budget was balanced. The consequences were not.",
                "Ledgermaster Vex",
            ),
            (
                "A coin saved is a coin someone forgot to expense.",
                "The Keeper of Receipts",
            ),
            (
                "Nothing terrifies mortals more than an unexpected audit.",
                "Accountant Emeritus #4",
            ),
            (
                "The difference between a plan and a disaster is usually a spreadsheet.",
                "Master of Spreadsheets",
            ),
            (
                "Reality compiled successfully. Nobody knows why.",
                "Archmage Segfault",
            ),
            (
                "I should really document this.",
                "The Nameless Warlock",
            ),
            (
                "The bug was removed. Three more attended the funeral.",
                "Daemon Lord Null",
            ),
            (
                "There is no temporary solution older than production code.",
                "The Ancient Maintainer",
            ),
            (
                "If it works, do not touch it. If it doesn't work, nobody knows why.",
                "Warlock of Forty-Seven Tabs",
            ),
            (
                "The answer was in your notes all along.",
                "Archivist Thorn",
            ),
            (
                "I took a note about this somewhere.",
                "The Sage of Infinite Notes",
            ),
            (
                "Knowledge is power. Searchability is greater power.",
                "Keeper of the Living Archive",
            ),
            (
                "I forgot where I wrote that, but I definitely wrote it.",
                "The Forgotten Rememberer",
            ),
            (
                "Every great idea begins as a note titled 'Untitled'.",
                "Master Linkwright",
            ),
            (
                "The hardest boss fight remains: getting started.",
                "Sir Checklist the Relentless",
            ),
            (
                "Motivation is a visitor. Discipline pays rent.",
                "Brother Completion",
            ),
            (
                "A completed task weighs nothing.",
                "Dame Productivity",
            ),
            (
                "The first checkbox is always the strongest.",
                "Paladin of the First Task",
            ),
            (
                "A task delayed gathers experience points.",
                "The Last Finisher",
            ),
            (
                "Perfect systems do not exist. Good systems survive users.",
                "The Architect of Folders",
            ),
            (
                "Every folder eventually becomes archaeology.",
                "Framework Sage Varon",
            ),
            (
                "Order is simply organized panic.",
                "Lord Structure",
            ),
            (
                "A good workflow survives contact with reality.",
                "Builder of Systems",
            ),
            (
                "Nothing is more permanent than a temporary process.",
                "The Refactor King",
            ),
            (
                "Tomorrow has an excellent reputation it does not deserve.",
                "The Keeper of Tuesdays",
            ),
            (
                "The meeting consumed two hours and produced three meetings.",
                "Master of Lost Afternoons",
            ),
            (
                "A calendar is merely a collection of future regrets.",
                "Chronomancer Vale",
            ),
            (
                "The deadline was visible from the beginning.",
                "The Calendar Oracle",
            ),
            (
                "Time management mostly consists of admitting what will not get done.",
                "The Last Hourglass",
            ),
            (
                "I can absolutely handle that tomorrow.",
                "Past You",
            ),
            (
                "I inherited this disaster.",
                "Future You",
            ),
            (
                "Keep postponing things. You're doing great.",
                "The Great Backlog",
            ),
            (
                "Just one more feature.",
                "An Anonymous Scope Dragon",
            ),
            (
                "This could have been an email.",
                "A Concerned Deadline Wraith",
            ),
            (
                "You clicked me. That's on you.",
                "Meeting Mimic #27",
            ),
            (
                "Your task list is delicious.",
                "The Great Backlog",
            ),
            (
                "I was promised fewer meetings.",
                "The Last Remaining QA Tester",
            ),
            (
                "You said five minutes three hours ago.",
                "The Coffee Machine",
            ),
            (
                "The roots grow. The tasks remain.",
                "The Zen Tree",
            ),
            (
                "I support your goals, but your calendar concerns me.",
                "A Notification Sprite",
            ),
            (
                "The project expanded to fill the available enthusiasm.",
                "An Overworked Project Manager",
            ),
            (
                "This seemed easier in planning.",
                "Grand Planner Elric",
            ),
            (
                "The roadmap was clear until reality joined the meeting.",
                "Lord Structure",
            ),
            (
                "The backlog remembers every promise.",
                "The Great Backlog",
            ),
            (
                "You do not lack time. You lack willingness to start.",
                "The Seventh Minute",
            ),
            (
                "Heroism is mostly administration with better marketing.",
                "Sir Checklist the Relentless",
            ),
            (
                "Every legend begins with a task nobody wanted to do.",
                "Brother Completion",
            ),
            (
                "The road to greatness is surprisingly administrative.",
                "Dame Productivity",
            ),
            (
                "A legendary journey through the dangerous realm of basic responsibility.",
                "The Last Finisher",
            ),
            (
                "The dragon was never the problem. The paperwork was.",
                "Paladin of the First Task",
            ),
            (
                "Thousands of years of civilization culminated in another status meeting.",
                "Master of Lost Afternoons",
            ),
            (
                "The universe rewards consistency far more often than brilliance.",
                "The Architect of Folders",
            ),
            (
                "You are one completed task away from feeling slightly better.",
                "The Zen Tree",
            ),
            (
                "Progress is simply stubbornness with a better public image.",
                "Archivist Thorn",
            ),
            (
                "The first step is rarely difficult. The first click is.",
                "Future You",
            ),
            (
                "No chosen one. No destiny. Just consistent effort and decent note-taking.",
                "The Chronicle",
            ),
            (
                "The Chronicle records persistence. The Chronicle also records excuses.",
                "The Chronicle",
            ),
            (
                "Remember: the backlog is doing pushups while you read this.",
                "An Anonymous Scope Dragon",
            ),
            (
                "The fantasy RPG where the dragon is your inbox.",
                "A Concerned Deadline Wraith",
            ),
            (
                "Because 'just do it' lacked proper worldbuilding.",
                "The Nameless Warlock",
            ),
            (
                "Your ancestors crossed oceans. You answered three emails.",
                "Future You",
            ),
            (
                "Achievement unlocked: Doing the thing.",
                "System Notification",
            ),
            (
                "Warning: may cause accidental self-improvement.",
                "System Notification",
            ),
            (
                "The productivity app that finally admitted motivation was never coming.",
                "Brother Completion",
            ),
            (
                "A sophisticated system for repeatedly doing things you already knew you should do.",
                "The Architect of Folders",
            ),
            (
                "The reward for completing your tasks: more tasks.",
                "The Great Backlog",
            ),
            (
                "Eventually the tasks finish themselves. This is not that strategy.",
                "Auditor Prime",
            ),
            (
                "Come for the quests. Stay because the backlog is gaining strength.",
                "An Anonymous Scope Dragon",
            ),
            (
                "Not guaranteed to increase productivity. Guaranteed to make it look cooler.",
                "Warlock of Forty-Seven Tabs",
            ),
            (
                "Every day is a side quest until the deadline arrives.",
                "The Keeper of Tuesdays",
            ),
            (
                "The only RPG where the final boss is your own calendar.",
                "The Calendar Oracle",
            ),
            (
                "The hero's journey, but with spreadsheets.",
                "Master of Spreadsheets",
            ),
            (
                "Saving the realm from chaos, one checkbox at a time.",
                "Sir Checklist the Relentless",
            ),
            (
                "Motivation is temporary. The Chronicle is forever.",
                "The Chronicle",
            ),
            (
                "The path to greatness is surprisingly administrative.",
                "Grand Planner Elric",
            ),
            (
                "You could be working right now.",
                "The Coffee Machine",
            ),
            (
                "We both know why you opened the app.",
                "Future You",
            ),
            (
                "The task has not become easier since yesterday.",
                "The Great Backlog",
            ),
            (
                "You seek wisdom. The task seeks completion.",
                "The Zen Tree",
            ),
            (
                "Another day, another opportunity to stop making excuses.",
                "Brother Completion",
            ),
        ];

        let questline_choice = questline_pool
            .choose(&mut rng)
            .unwrap_or(&questline_pool[0]);
        let q_quote = questline_choice.0.to_string();
        let q_author = questline_choice.1.to_string();

        let mut class_opt = None;

        if let Some(u) = user {
            let mut class_pool = Vec::new();
            match u.class {
                ClassType::CodeWarlock => {
                    class_pool.push((
                        "The spell worked. Nobody knows why. Preserve the artifact.",
                        "Code Warlock",
                    ));
                    class_pool.push((
                        "A true warlock commits first and understands later.",
                        "Code Warlock",
                    ));
                    class_pool.push((
                        "There are two kinds of bugs: known bugs and future surprises.",
                        "Code Warlock",
                    ));
                    class_pool.push((
                        "The code compiles. Ask no further questions.",
                        "Code Warlock",
                    ));
                    class_pool.push((
                        "Every script is temporary. Some merely survive longer.",
                        "Code Warlock",
                    ));
                    class_pool.push((
                        "Magic is just automation with better marketing.",
                        "Code Warlock",
                    ));
                    class_pool.push((
                        "The terminal rewards courage and punishes typos.",
                        "Code Warlock",
                    ));
                    class_pool.push((
                        "If the solution feels cursed, it is probably production-ready.",
                        "Code Warlock",
                    ));
                    class_pool.push((
                        "The strongest enchantment is copy-paste followed by confidence.",
                        "Code Warlock",
                    ));
                    class_pool.push((
                        "Ancient prophecies speak of documentation. None have seen it.",
                        "Code Warlock",
                    ));
                }
                ClassType::TaskPaladin => {
                    class_pool.push((
                        "Smite one task and the others begin to panic.",
                        "Task Paladin",
                    ));
                    class_pool.push((
                        "Discipline is motivation wearing plate armor.",
                        "Task Paladin",
                    ));
                    class_pool.push((
                        "The sacred checklist remembers what mortals forget.",
                        "Task Paladin",
                    ));
                    class_pool.push((
                        "A task delayed today returns tomorrow with reinforcements.",
                        "Task Paladin",
                    ));
                    class_pool.push((
                        "The enemy is not difficulty. The enemy is 'later.'",
                        "Task Paladin",
                    ));
                    class_pool.push((
                        "Your future self has filed several complaints.",
                        "Task Paladin",
                    ));
                    class_pool.push(("Completion is the purest form of magic.", "Task Paladin"));
                    class_pool.push((
                        "The to-do list grows stronger when ignored.",
                        "Task Paladin",
                    ));
                    class_pool.push(("A paladin's greatest weapon is starting.", "Task Paladin"));
                    class_pool.push((
                        "Victory is often just fifteen uninterrupted minutes.",
                        "Task Paladin",
                    ));
                }
                ClassType::MindSage => {
                    class_pool.push((
                        "Every note is a future memory refusing to die.",
                        "Mind Sage",
                    ));
                    class_pool.push((
                        "Ideas travel in packs. Catch one and the others appear.",
                        "Mind Sage",
                    ));
                    class_pool.push(("The mind is a forest. Organize the trails.", "Mind Sage"));
                    class_pool.push((
                        "Confusion is simply knowledge waiting for better labels.",
                        "Mind Sage",
                    ));
                    class_pool.push((
                        "A connected thought is worth three forgotten insights.",
                        "Mind Sage",
                    ));
                    class_pool.push((
                        "Wisdom is knowing which rabbit holes charge rent.",
                        "Mind Sage",
                    ));
                    class_pool.push((
                        "Most breakthroughs arrive while pretending not to think.",
                        "Mind Sage",
                    ));
                    class_pool.push((
                        "The answer was always there. It was hiding behind bad organization.",
                        "Mind Sage",
                    ));
                    class_pool.push((
                        "Knowledge grows best when linked to other knowledge.",
                        "Mind Sage",
                    ));
                    class_pool.push((
                        "Every great insight begins as a weird note nobody understands.",
                        "Mind Sage",
                    ));
                }
                ClassType::SystemsArchitect => {
                    class_pool.push((
                        "If you cannot find it in thirty seconds, chaos has won.",
                        "Systems Architect",
                    ));
                    class_pool.push((
                        "Order is simply procrastination that became useful.",
                        "Systems Architect",
                    ));
                    class_pool.push((
                        "A folder without purpose is merely decorative.",
                        "Systems Architect",
                    ));
                    class_pool.push((
                        "The universe is just a very large filing system.",
                        "Systems Architect",
                    ));
                    class_pool.push(("Structure first. Regret less.", "Systems Architect"));
                    class_pool.push((
                        "Every problem becomes smaller after proper categorization.",
                        "Systems Architect",
                    ));
                    class_pool.push((
                        "The difference between chaos and strategy is a naming convention.",
                        "Systems Architect",
                    ));
                    class_pool.push((
                        "A system that depends on memory is a trap.",
                        "Systems Architect",
                    ));
                    class_pool.push((
                        "When everything has a place, panic has nowhere to live.",
                        "Systems Architect",
                    ));
                    class_pool.push((
                        "Good architecture is invisible. Bad architecture has meetings.",
                        "Systems Architect",
                    ));
                }
                ClassType::TimeChronomancer => {
                    class_pool.push(("Time flies. Most people help push.", "Time Chronomancer"));
                    class_pool.push((
                        "The clock is undefeated, but you can still outsmart it.",
                        "Time Chronomancer",
                    ));
                    class_pool.push(("Lost hours rarely send postcards.", "Time Chronomancer"));
                    class_pool.push((
                        "Every minute has a job. Most just never receive one.",
                        "Time Chronomancer",
                    ));
                    class_pool.push((
                        "The future arrives surprisingly on schedule.",
                        "Time Chronomancer",
                    ));
                    class_pool.push((
                        "A calendar is merely a battle plan against entropy.",
                        "Time Chronomancer",
                    ));
                    class_pool.push(("You do not find time. You capture it.", "Time Chronomancer"));
                    class_pool.push((
                        "Hours are gold. Notifications are goblins.",
                        "Time Chronomancer",
                    ));
                    class_pool.push((
                        "The secret to more time is fewer distractions pretending to be important.",
                        "Time Chronomancer",
                    ));
                    class_pool.push((
                        "Even time respects those who use it wisely.",
                        "Time Chronomancer",
                    ));
                }
                ClassType::ArchAccountant => {
                    class_pool.push(("A balanced ledger is a silent song of order in a world of spendthrift chaos.", "Arch Accountant"));
                    class_pool.push((
                        "Nothing is spent without cost. Balance your assets, secure your focus.",
                        "Arch Accountant",
                    ));
                }
            }

            if let Some(choice) = class_pool.choose(&mut rng) {
                class_opt = Some((choice.0.to_string(), choice.1.to_string()));
            }
        }

        (q_quote, q_author, class_opt)
    }

    // Initializer.
    // Inicializa toda la app: DB, identidad, config, quote, audio, sync — pues aquí empieza todo
    pub fn new(db_path: &Path) -> Result<Self> {
        let db = Database::new(db_path)?;
        let mut user = db.get_user()?;

        let existing_user_id = user.as_ref().map(|u| u.id);

        // Revisa si ya existe identity.key antes de generarlo — si existe sin usuario local,
        // es un héroe en máquina nueva que copió su llave para restaurar desde la nube
        let _identity_key_existed = {
            let storage_dir = crate::storage::get_storage_dir().unwrap_or_default();
            storage_dir.join("identity.key").exists()
        };

        let identity = crate::services::identity::Identity::load_or_create(existing_user_id)?;

        // Genera un device_id único si no existe — se persiste en la DB para identificar la máquina
        let device_id = match db.get_setting("device_id")? {
            Some(id) => id,
            None => {
                let id = Uuid::new_v4().to_string();
                db.set_setting("device_id", &id)?;
                id
            }
        };
        let device_name = crate::services::identity::get_local_device_name();
        db.register_device(&device_id, &device_name)?;

        // La config tiene prioridad sobre los settings de la DB — archivo > DB
        let config = crate::services::config::Config::load().unwrap_or_default();
        let server_url = if config.sync_enabled {
            config.server_url.clone()
        } else {
            db.get_setting("server_url")?
                .unwrap_or_else(|| "http://localhost:8080".to_string())
        };
        let auto_sync = if config.sync_enabled {
            config.auto_sync
        } else {
            db.get_setting("auto_sync")?
                .map(|s| s == "true")
                .unwrap_or(false)
        };

        // Recuperación automática en dispositivo nuevo — jala el backup de la nube para que no llegue al onboarding
        #[cfg(not(test))]
        let should_recover = _identity_key_existed && user.is_none() && config.sync_enabled;
        #[cfg(test)]
        let should_recover = false;

        if should_recover {
            let client = crate::services::api_client::ApiClient::new(
                &server_url,
                identity.clone(),
                &device_id,
            );
            if let Ok(json) = client.send_request("GET", "recovery/latest", "") {
                if !json.trim().is_empty() {
                    use base64::{engine::general_purpose::STANDARD, Engine as _};
                    let decoded = STANDARD.decode(json.trim())
                        .ok()
                        .and_then(|b| String::from_utf8(b).ok())
                        .unwrap_or(json);
                    if db.import_from_json(&decoded).is_ok() {
                        user = db.get_user()?;
                    }
                }
            }
        }

        #[cfg(not(test))]
        let should_register_device = config.sync_enabled;
        #[cfg(test)]
        let should_register_device = false;

        if should_register_device {
            let client = crate::services::api_client::ApiClient::new(
                &server_url,
                identity.clone(),
                &device_id,
            );
            let d_name = device_name.clone();
            let u_name = user.as_ref().map(|u| u.username.clone()).unwrap_or_default();
            let _ = std::thread::spawn(move || {
                let body = serde_json::json!({
                    "device_name": d_name,
                    "username": u_name
                }).to_string();
                let _ = client.send_request("POST", "devices/register", &body);
            });
        }

        let mut theme_service = ThemeService::new();
        let active_screen = ActiveScreen::Intro;

        if let Some(ref u) = user {
            theme_service.set_class(u.class);
        }

        let (quote, quote_author, class_quote_opt) = Self::choose_dynamic_quote(&user, &db);
        let class_quote = class_quote_opt.as_ref().map(|q| q.0.clone());
        let class_quote_author = class_quote_opt.as_ref().map(|q| q.1.clone());

        let onboarding_classes = vec![
            ClassType::ArchAccountant,
            ClassType::CodeWarlock,
            ClassType::MindSage,
            ClassType::SystemsArchitect,
            ClassType::TaskPaladin,
            ClassType::TimeChronomancer,
        ];

        let mut app = Self {
            db,
            user,
            active_screen,
            onboarding_username: String::new(),
            onboarding_class_idx: 0,
            onboarding_focus: OnboardingFocus::NameInput,
            onboarding_classes,
            onboarding_error: None,
            theme_service,
            active_tab_idx: 0,
            quote,
            quote_author,
            class_quote,
            class_quote_author,

            daily_quests: Vec::new(),
            tasks_due_today: Vec::new(),
            projects: Vec::new(),
            should_quit: false,

            // Stage 2 setup
            selected_project_idx: 0,
            active_project_id: None,
            workspace_tab_idx: 0,
            workspace_sidebar_focused: true,
            workspace_help_open: false,
            selected_task_idx: 0,
            selected_note_idx: 0,
            selected_journal_idx: 0,
            selected_archive_idx: 0,
            selected_reflection_idx: 0,
            character_focus: 0,
            reflection_detail_scroll: 0,
            modal_state: ModalType::None,
            overlay_modal: ModalType::None,
            editor_state: None,
            dashboard_task_focus: false,
            selected_dashboard_task_idx: 0,
            searching: false,
            search_query: String::new(),
            task_filter: "All".to_string(),
            task_sort: "CreatedDate".to_string(),

            notifications: Vec::new(),
            fragment_notification: None,

            // Stage 4 setup
            active_focus_session: None,
            selected_focus_duration_idx: 0,
            selected_focus_project_idx: 0,
            selected_focus_task_idx: 0,
            selected_focus_field_idx: 0,
            selected_ritual_idx: 0,
            selected_milestone_idx: 0,

            // Soundscape setup
            audio_player: crate::audio::AudioPlayer::new(),
            selected_soundscape_idx: 0,
            selected_focus_soundscape_idx: 0,

            identity,
            device_id,

            server_url,
            auto_sync,
            sync_status_msg: "Idle".to_string(),
            sync_conflicts: Vec::new(),
            sync_failure_count: 0,
            config,
            last_auto_sync: std::time::Instant::now(),
            last_sync_status_time: None,
            last_mutation: None,
            last_sync_warlock_xp: 0,

            // Stage 5B Fellowship & Collaboration Fields
            selected_fellowship_project_idx: 0,

            fellowship_search_query: String::new(),
            selected_fellowship_tab: 0,
            selected_invitation_idx: 0,
            selected_notification_idx: 0,
            fellowship_search_results: Vec::new(),
            fellowship_chat_input: String::new(),
            fellowship_selected_msg_idx: usize::MAX,
            fellowship_focus_left: false,
            fellowship_composing: false,
            about_scroll: 0,
            about_content_lines: Cell::new(0),
            terminal_height: 40,
            about_fact_seed: 0,
            bug_report_modal: None,

            // Stage 6 initializations
            selected_chronicle_idx: 0,
            selected_library_cat_idx: 0,
            selected_library_item_idx: 0,
            library_active_col: 0,
            library_scroll_offset: 0,
            selected_relic_idx: 0,
            ambient_effects_enabled: true,
            active_ambient_effect: 1,
            ambient_particles: Vec::new(),
            ambient_particles_ticks_remaining: 0,
            corrupted_backups_found: Vec::new(),
            quit_confirm_ticks: 0,
            intro_ticks: 0,
            music_scroll_ticks: 0,
            update_check: std::sync::Arc::new(std::sync::Mutex::new(None)),
            update_check_done: false,
            run_installer_on_exit: false,
            codices: Vec::new(),
            viewing_step_for_task: None,
            selected_notes_flat_idx: 0,
            all_tasks: Vec::new(),
            all_notes: Vec::new(),
            all_journals: Vec::new(),
            sync_in_progress: false,
            sync_result: std::sync::Arc::new(std::sync::Mutex::new(None)),
            chat_poll_active: false,
            chat_rx: None,
            last_chat_poll: std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(60))
                .unwrap_or(std::time::Instant::now()),
            last_chat_timestamp: std::collections::HashMap::new(),
            great_chronicle_entries: Vec::new(),
            great_chronicle_scroll: 0,
            chapter_progress: None,
            chapter_panel_scroll: 0,
            chapter_tab: 0,
            chapter_panel_focused: false,
            chapter_history: Vec::new(),
            chapter_completion_seen: false,
            chapter_progress_refreshed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            sprite_notifications_shown_this_session: 0,
            last_sprite_notification_time: None,
            last_sprite_check_time: None,
            prologue_page: 0,
            prologue_line_idx: 0,
            prologue_char_in_line: 0,
            prologue_next_is_onboarding: false,
            prologue_delay_ticks: 0,
            prologue_skip_checked: false,
        };

        app.reload_data()?;
        if app.user.is_some() {
            app.check_new_day()?;
        }

        // Checa la versión en un hilo aparte para no bloquear el arranque — el resultado llega después
        {
            let channel = app.update_check.clone();
            let _ = std::thread::spawn(move || {
                fn is_newer(current: &str, latest: &str) -> bool {
                    let parse = |v: &str| -> (u32, u32, u32) {
                        let mut it = v.split('.');
                        let major = it.next().and_then(|p| p.parse().ok()).unwrap_or(0);
                        let minor = it.next().and_then(|p| p.parse().ok()).unwrap_or(0);
                        let patch = it.next().and_then(|p| p.parse().ok()).unwrap_or(0);
                        (major, minor, patch)
                    };
                    parse(latest) > parse(current)
                }
                let current = env!("CARGO_PKG_VERSION");
                // Consulta el API de GitHub para saber si hay una versión nueva — tag_name viene como "v1.0.6"
                let result = ureq::AgentBuilder::new()
                    .timeout(std::time::Duration::from_secs(8))
                    .build()
                    .get("https://api.github.com/repos/gibranlp/Questline-cli/releases/latest")
                    .set("User-Agent", "questline-cli")
                    .call();
                let new_ver = match result {
                    Ok(resp) => resp
                        .into_string()
                        .ok()
                        .and_then(|b| serde_json::from_str::<serde_json::Value>(&b).ok())
                        .and_then(|j| j["tag_name"].as_str().map(|s| s.trim_start_matches('v').to_string()))
                        .filter(|v| is_newer(current, v))
                        .unwrap_or_default(),
                    Err(_) => String::new(),
                };
                if let Ok(mut guard) = channel.lock() {
                    *guard = Some(new_ver);
                }
            });
        }

        // Restaura el volumen y la fuente de música del arranque anterior para que siga sonando
        if let Ok(Some(vol_str)) = app.db.get_setting("last_music_volume") {
            if let Ok(vol) = vol_str.parse::<f32>() {
                app.audio_player.set_volume(vol);
            }
        }
        if let Ok(Some(last_source)) = app.db.get_setting("last_music_source") {
            if !last_source.is_empty() && last_source != "Silent" && last_source != "None" {
                if let Some(idx) = crate::audio::SOUNDSCAPES.iter().position(|s| s.name == last_source) {
                    app.selected_soundscape_idx = idx;
                }
                app.audio_player.init_soundscape(&last_source);
            }
        }
        // Precarga la carpeta de música local para que las teclas Next/Prev funcionen sin tocar la DB
        if let Ok(Some(folder)) = app.db.get_setting("local_music_folder") {
            if !folder.trim().is_empty() {
                app.audio_player.set_local_music_folder(&folder);
            }
        }

        Ok(app)
    }

    // Simula eventos de Fellowship para pruebas offline — no manches, no llamar en producción
    pub fn simulate_fellowship_sync(&self) -> Result<()> {
        let my_identity = self.identity.public_key.clone();
        let my_username = self
            .user
            .as_ref()
            .map(|u| u.username.clone())
            .unwrap_or_else(|| "Gibranlp".to_string());

        // 1. Update presence
        self.db.update_presence(
            "alex_key",
            "Alex",
            true,
            "Just now",
            Some("Fellowship Adventure"),
            "Visible",
        )?;
        self.db.update_presence(
            "fiona_key",
            "Fiona",
            true,
            "2 mins ago",
            Some("Zen Garden Maintenance"),
            "Visible",
        )?;
        self.db
            .update_presence("diana_key", "Diana", false, "2 hours ago", None, "Offline")?;

        // 2. Generate simulated invitation if none exists
        let invites = self.db.get_invitations()?;
        if invites.is_empty() {
            let sim_proj_id = "fellowship_adv_proj_id";
            self.db.create_invitation(
                sim_proj_id,
                "Fellowship Adventure",
                "alex_key",
                "Alex",
                &my_identity,
                "Companion",
            )?;
            self.db.create_notification(
                "invitation",
                "Fellowship Invitation",
                "Alex has invited you to join Fellowship Adventure as a Companion.",
                Some(sim_proj_id),
            )?;
        }

        // 3. Populate project data if accepted
        let projects = self.db.get_projects()?;
        if let Some(proj) = projects
            .iter()
            .find(|p| p.id.to_string() == "fellowship_adv_proj_id")
        {
            let msgs = self.db.get_chronicle_messages("fellowship_adv_proj_id")?;
            if msgs.is_empty() {
                self.db.add_chronicle_message(
                    "fellowship_adv_proj_id",
                    "alex_key",
                    "Alex",
                    "Greetings companions! Ready for our quest? ⚔️",
                    "text",
                )?;
                self.db.add_chronicle_message(
                    "fellowship_adv_proj_id",
                    "system",
                    "System",
                    "Alex invited Gibranlp to the project.",
                    "system",
                )?;
                self.db.log_activity(
                    Some("fellowship_adv_proj_id"),
                    "member_joined",
                    "Gibranlp joined the fellowship.",
                    &my_identity,
                    &my_username,
                )?;
                self.db.log_activity(
                    Some("fellowship_adv_proj_id"),
                    "note_created",
                    "Alex created shared note: Fellowship Codex.",
                    "alex_key",
                    "Alex",
                )?;
            }

            // Alex reaction / reply simulation
            let user_msg_count = msgs.iter().filter(|m| m.2 == my_identity).count();
            let alex_reply_count = msgs
                .iter()
                .filter(|m| m.2 == "alex_key" && m.4.contains("Outstanding"))
                .count();
            if user_msg_count > alex_reply_count {
                self.db.add_chronicle_message(
                    "fellowship_adv_proj_id",
                    "alex_key",
                    "Alex",
                    "Outstanding work! Let's keep pushing!",
                    "text",
                )?;
                if let Some(last_user_msg) = msgs.iter().rfind(|m| m.2 == my_identity) {
                    self.db
                        .add_message_reaction(&last_user_msg.0, "alex_key", "⚔️")?;
                }
            }

            // Tasks seed
            let tasks = self.db.get_tasks()?;
            let proj_tasks: Vec<_> = tasks
                .into_iter()
                .filter(|t| {
                    t.project_id.map(|pid| pid.to_string())
                        == Some("fellowship_adv_proj_id".to_string())
                })
                .collect();
            if proj_tasks.is_empty() {
                let t1 = Task {
                    id: Uuid::new_v4(),
                    project_id: Some(proj.id),
                    title: "Design Fellowship Database Schema".to_string(),
                    description: Some("Alex handles database schemas".to_string()),
                    due_date: None,
                    completed: true,
                    priority: crate::models::TaskPriority::High,
                    created_at: Utc::now() - chrono::Duration::hours(2),
                    updated_at: Utc::now(),
                    owner_identity: Some("alex_key".to_string()),
                    owner_username: Some("Alex".to_string()),
                    parent_task_id: None,
                    xp_awarded: true,
                    recurrence: None,
                };
                self.db.insert_task(&t1)?;
                self.db
                    .assign_task(&t1.id.to_string(), "alex_key", "Alex")?;
                self.db
                    .assign_task(&t1.id.to_string(), &my_identity, &my_username)?;

                let t2 = Task {
                    id: Uuid::new_v4(),
                    project_id: Some(proj.id),
                    title: "Build Fellowship TUI Dashboard".to_string(),
                    description: Some("Implement Fellowship Tab 9".to_string()),
                    due_date: Some(Utc::now() + chrono::Duration::days(3)),
                    completed: false,
                    priority: crate::models::TaskPriority::Medium,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    owner_identity: Some("alex_key".to_string()),
                    owner_username: Some("Alex".to_string()),
                    parent_task_id: None,
                    xp_awarded: false,
                    recurrence: None,
                };
                self.db.insert_task(&t2)?;
                self.db
                    .assign_task(&t2.id.to_string(), &my_identity, &my_username)?;
            }
        }

        Ok(())
    }

    // Reloads all states from the SQLite database.
    pub fn reload_data(&mut self) -> Result<()> {
        if self.user.is_some() {
            self.user = self.db.get_user()?;
            if let Some(ref u) = self.user {
                self.theme_service.set_class(u.class);
            }

            // Load theme choice
            if let Some(theme_choice_str) = self.db.get_setting("equipped_theme")? {
                let choice = match theme_choice_str.as_str() {
                    "Forest" => ThemeChoice::Forest,
                    "AncientLibrary" => ThemeChoice::AncientLibrary,
                    "MountainFortress" => ThemeChoice::MountainFortress,
                    "ArcaneWorkshop" => ThemeChoice::ArcaneWorkshop,
                    "OceanTemple" => ThemeChoice::OceanTemple,
                    _ => ThemeChoice::ClassDefault,
                };
                self.theme_service.set_theme_choice(choice);
            }

            self.ambient_effects_enabled = self
                .db
                .get_setting("ambient_effects_enabled")?
                .and_then(|s| s.parse::<bool>().ok())
                .unwrap_or(true);

            self.active_ambient_effect = self
                .db
                .get_setting("active_ambient_effect")?
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);

            self.projects = self.db.get_projects()?;
            self.projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

            // Load all data into caches — used by renderer and key handlers instead of per-frame/per-key DB queries
            self.all_tasks = self.db.get_tasks()?;
            self.all_notes = self.db.get_notes().unwrap_or_default();
            self.all_journals = self.db.get_journal_entries().unwrap_or_default();
            let today = Utc::now().date_naive();
            self.tasks_due_today = self.all_tasks.iter().filter(|t| !t.completed).cloned().collect();

            // Load daily quests for today
            self.daily_quests = self.db.get_daily_quests_for_date(today)?;

            // Load codices for active project
            if let Some(pid) = self.active_project_id {
                self.codices = self.db.get_codices_for_project(pid).unwrap_or_default();
            } else {
                self.codices.clear();
            }
        }
        Ok(())
    }

    // Main entry router for key events.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return Ok(());
        }

        let modal_handled = self.handle_rpg_modal_key(key)?;
        if modal_handled {
            return Ok(());
        }

        // Bug report modal intercepts all keys when open
        if self.bug_report_modal.is_some() {
            return self.handle_bug_report_key(key);
        }

        // Fellowship chronicle chat intercept — handles input/browse before general keys
        if self.active_screen == ActiveScreen::Fellowship
            && self.selected_fellowship_tab == 0
            && self.modal_state == ModalType::None
        {
            if self.handle_fellowship_chat_key(key)? {
                return Ok(());
            }
        }

        let in_text_entry = self.searching
            || self.modal_state != ModalType::None
            || self.active_screen == ActiveScreen::Editor
            || self.active_screen == ActiveScreen::Onboarding;

        // Global Modals Triggers
        if (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('p'))
            || (key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('k'))
            || key.code == KeyCode::F(1)
            || (key.code == KeyCode::Char(':') && !in_text_entry)
        {
            let actions = self.get_available_command_actions("");
            self.modal_state = ModalType::CommandPalette {
                query: String::new(),
                selected_idx: 0,
                actions,
            };
            return Ok(());
        }

        // Global sync shortcut — works on any screen except the editor (which uses Ctrl+S to save notes)
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('s')
            && self.active_screen != ActiveScreen::Editor
        {
            if self.config.sync_enabled {
                self.start_forced_sync();
            } else {
                let _ = self.trigger_sync();
            }
            return Ok(());
        }

        // Global quick-create shortcuts — open project picker then task/note form
        if !in_text_entry && key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('t') => {
                    if self.active_project_id.is_some() {
                        self.active_screen = ActiveScreen::Workspace;
                        self.workspace_tab_idx = 0;
                        self.modal_state = ModalType::NewTask {
                            title: String::new(),
                            desc: String::new(),
                            desc_cursor: 0,
                            priority: crate::models::task::TaskPriority::Medium,
                            due_date_type: DueDateType::InDays,
                            due_date_val: "1".to_string(),
                            focus_idx: 0,
                            parent_task_id: None,
                            recurrence: None,
                        };
                    } else {
                        self.modal_state = ModalType::SelectProjectForAction {
                            action_id: "create_task",
                            selected_idx: 0,
                        };
                    }
                    return Ok(());
                }
                KeyCode::Char('n') => {
                    if self.active_project_id.is_some() {
                        self.active_screen = ActiveScreen::Workspace;
                        self.workspace_tab_idx = 1;
                        self.modal_state = ModalType::NewJournalEntry {
                            content: String::new(),
                        };
                    } else {
                        self.modal_state = ModalType::SelectProjectForAction {
                            action_id: "create_note",
                            selected_idx: 0,
                        };
                    }
                    return Ok(());
                }
                _ => {}
            }
        }

        if key.code == KeyCode::Char('?') {
            if !in_text_entry && self.active_screen != ActiveScreen::Workspace {
                self.active_screen = ActiveScreen::About;
                self.active_tab_idx = 13;
                self.about_scroll = 0;
                use rand::Rng;
                self.about_fact_seed = rand::thread_rng().r#gen();
                return Ok(());
            }
        }

        // Global Audio Hotkeys: only active when not in text-entry modes
        let in_text_entry = self.searching
            || self.modal_state != ModalType::None
            || self.active_screen == ActiveScreen::Editor
            || self.active_screen == ActiveScreen::Onboarding;

        if !in_text_entry {
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    let quotes = [
                        "The chronicle closes for now, but the story continues tomorrow.",
                        "Your quests rest. The realm will await your return.",
                        "Another page has been written in your legend.",
                        "The campfire burns low as today's adventures come to an end.",
                        "Even heroes must rest between journeys.",
                        "Your companions remain watchful until you return.",
                        "The road continues beyond the horizon.",
                        "Today's victories have been recorded in the Chronicle.",
                        "Your Zen Tree sways gently in the evening breeze.",
                        "The realm grows stronger through every task completed.",
                        "No quest is too small when pursued with purpose.",
                        "The archives will safeguard your progress.",
                        "Another day of adventure becomes another chapter of wisdom.",
                        "Rest now, traveler. Greater quests await.",
                        "The Fellowship remembers your contributions.",
                        "Every completed task leaves a mark upon the world.",
                        "Your tools are set aside, but your journey is far from over.",
                        "The path of progress is walked one step at a time.",
                        "Your legend grows, even in moments of rest.",
                        "The realm sleeps, but does not forget.",
                        "The stars keep watch over unfinished quests.",
                        "The ink has dried on today's chapter.",
                        "Tomorrow's challenges are already gathering beyond the hills.",
                        "The road behind you is proof of how far you've come.",
                        "Every milestone was once a distant goal.",
                        "Your efforts echo throughout the realm.",
                        "The lantern remains lit for your return.",
                        "A well-earned rest is part of every great journey.",
                        "The pages of the Chronicle turn ever forward.",
                        "The work of today becomes the wisdom of tomorrow.",
                        "The tree grows not only through action, but through patience.",
                        "The realm remembers those who return.",
                        "Your story advances with every small victory.",
                        "Even the longest quest is completed one step at a time.",
                        "The winds carry word of your accomplishments.",
                        "The next chapter awaits whenever you are ready.",
                        "Progress has been secured. Adventure will continue.",
                        "The day's burdens have become tomorrow's experience.",
                        "The journey pauses, but never truly ends.",
                        "The Chronicle records not perfection, but persistence.",
                        "You arrived with intentions. You depart with progress.",
                        "The greatest quests are rarely completed in a single day.",
                        "One day you will look back upon these pages and see how far you've traveled.",
                        "The realm is better than it was when you arrived today.",
                        "The quiet moments between adventures are part of the adventure itself.",
                        "Every return begins with a farewell.",
                        "The road ahead remains open.",
                        "Leave camp with pride. Return with purpose.",
                        "The adventure continues when next you open the Chronicle.",
                    ];
                    use rand::seq::SliceRandom;
                    let mut rng = rand::thread_rng();
                    let chosen_quote = quotes.choose(&mut rng).copied().unwrap_or("Even heroes must rest between journeys.").to_string();
                    self.modal_state = ModalType::QuitConfirm { quote: chosen_quote };
                    return Ok(());
                }
                KeyCode::Char('m') => {
                    // Only switch to Soundscapes if not on Workspace milestones tab (uses 'm' for New Milestone)
                    // and not on Fellowship screen (uses 'm' for New Message)
                    if !(self.active_screen == ActiveScreen::Workspace
                        && self.workspace_tab_idx == 3)
                        && self.active_screen != ActiveScreen::Fellowship
                    {
                        self.active_screen = ActiveScreen::Soundscapes;
                        self.active_tab_idx = 7;
                        return Ok(());
                    }
                }
                KeyCode::Char('p') => {
                    // Only pause audio if not on Fellowship screen (uses 'p' for companions tab)
                    if self.active_screen != ActiveScreen::Fellowship {
                        self.audio_player.pause();
                        return Ok(());
                    }
                }
                KeyCode::Char('s') => {
                    // Only stop audio if not on Character screen (specialization select)
                    // and not on Workspace tasks tab (sort tasks)
                    let is_character_spec = self.active_screen == ActiveScreen::Character;
                    let is_task_sort = self.active_screen == ActiveScreen::Workspace
                        && self.workspace_tab_idx == 0;
                    if !is_character_spec && !is_task_sort {
                        self.audio_player.stop();
                        let _ = self.db.set_setting("last_music_source", "Silent");
                        return Ok(());
                    }
                }
                KeyCode::Char('n') => {
                    // Only cycle soundscape if not on Dashboard, Projects, Workspace, or Fellowship
                    if self.active_screen != ActiveScreen::Dashboard
                        && self.active_screen != ActiveScreen::Projects
                        && self.active_screen != ActiveScreen::Workspace
                        && self.active_screen != ActiveScreen::Fellowship
                    {
                        use crate::audio::SOUNDSCAPES;
                        let current = self.audio_player.get_state().current_soundscape;
                        let idx = SOUNDSCAPES
                            .iter()
                            .position(|s| s.name == current || (s.name == "Music For Programming" && (current.starts_with("MFP:") || current.to_lowercase().contains("music for programming") || current.to_lowercase().contains("music_for_programming"))))
                            .unwrap_or(SOUNDSCAPES.len() - 1);
                        let next_idx = (idx + 1) % SOUNDSCAPES.len();
                        self.audio_player.play(SOUNDSCAPES[next_idx].name);
                        return Ok(());
                    }
                }
                KeyCode::Char('+') => {
                    self.audio_player.volume_up();
                    let vol = self.audio_player.get_state().volume;
                    let _ = self.db.set_setting("last_music_volume", &vol.to_string());
                    return Ok(());
                }
                KeyCode::Char('*') => {
                    self.audio_player.set_volume(0.5);
                    let _ = self.db.set_setting("last_music_volume", "0.5");
                    return Ok(());
                }
                KeyCode::Char('-') => {
                    self.audio_player.volume_down();
                    let vol = self.audio_player.get_state().volume;
                    let _ = self.db.set_setting("last_music_volume", &vol.to_string());
                    return Ok(());
                }
                KeyCode::Char('f') => {
                    if self.active_screen == ActiveScreen::Soundscapes && self.modal_state == ModalType::None {
                        let current_val = self.db.get_setting("local_music_folder").unwrap_or_default().unwrap_or_default();
                        let sugs = App::path_suggestions(&current_val);
                        self.modal_state = ModalType::LocalMusicFolder { input: current_val, suggestions: sugs, selected: 0 };
                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        // If focus session is active, lock input to focus screen
        if self.active_focus_session.is_some() {
            self.handle_focus_screen_key(key)?;
            return Ok(());
        }

        match self.active_screen {
            ActiveScreen::Intro => {
                let skip = self.db.get_setting("prologue_skip")
                    .ok().flatten()
                    .map(|v| v == "1")
                    .unwrap_or(false);
                self.prologue_next_is_onboarding = self.user.is_none();
                if skip {
                    if self.prologue_next_is_onboarding {
                        self.active_screen = ActiveScreen::Onboarding;
                    } else {
                        self.active_screen = ActiveScreen::Dashboard;
                        self.active_tab_idx = 0;
                    }
                } else {
                    self.prologue_page = 0;
                    self.prologue_line_idx = 0;
                    self.prologue_char_in_line = 0;
                    self.prologue_delay_ticks = 20;
                    self.prologue_skip_checked = false;
                    self.audio_player.play_cinematic();
                    self.active_screen = ActiveScreen::Prologue;
                }
            }
            ActiveScreen::Prologue => {
                use crate::screens::prologue::page_lines;
                let total = page_lines(self.prologue_page).len();
                let page_done = self.prologue_line_idx >= total;

                // On the final page (chapter one done): wait for Space/Enter,
                // allow 'x' to toggle the checkbox, ignore all other keys.
                if self.prologue_page == 1 && page_done {
                    match key.code {
                        KeyCode::Char('x') => {
                            self.prologue_skip_checked = !self.prologue_skip_checked;
                        }
                        KeyCode::Char(' ') | KeyCode::Enter => {
                            self.audio_player.stop_cinematic();
                            // Always write the current checkbox state — ensures unchecking
                            // from the Tab 8 replay actually clears the skip setting.
                            let _ = self.db.set_setting(
                                "prologue_skip",
                                if self.prologue_skip_checked { "1" } else { "0" },
                            );
                            if self.prologue_next_is_onboarding {
                                self.active_screen = ActiveScreen::Onboarding;
                            } else {
                                self.active_screen = ActiveScreen::Dashboard;
                                self.active_tab_idx = 0;
                            }
                        }
                        _ => {} // all other keys do nothing when waiting at the end
                    }
                    return Ok(());
                }

                // During typing or between pages: any key cancels delay and skips/advances.
                self.prologue_delay_ticks = 0;
                if self.prologue_line_idx < total {
                    // Skip — jump to end of this page immediately
                    self.prologue_line_idx = total;
                    self.prologue_char_in_line = 0;
                } else if self.prologue_page == 0 {
                    // Advance to Chapter One page
                    self.prologue_page = 1;
                    self.prologue_line_idx = 0;
                    self.prologue_char_in_line = 0;
                }
            }
            ActiveScreen::Onboarding => {
                self.handle_onboarding_key(key)?;
            }
            ActiveScreen::Editor => {
                self.handle_editor_key(key)?;
            }
            ActiveScreen::Workspace => {
                self.handle_workspace_key(key)?;
            }
            ActiveScreen::Focus => {
                self.handle_focus_screen_key(key)?;
            }
            _ => {
                self.handle_top_screen_key(key)?;
            }
        }

        Ok(())
    }

    fn handle_rpg_modal_key(&mut self, key: KeyEvent) -> Result<bool> {
        // Overlay modal (step creation on top of EditTask) takes priority over the base modal.
        if self.overlay_modal != ModalType::None {
            if let Some(project_id) = self.active_project_id {
                if let ModalType::NewTask {
                    ref title, ref desc, desc_cursor, priority, due_date_type,
                    ref due_date_val, focus_idx, parent_task_id, recurrence,
                } = self.overlay_modal.clone()
                {
                    // Swap: modal_state ← overlay, overlay ← base EditTask
                    std::mem::swap(&mut self.modal_state, &mut self.overlay_modal);
                    self.handle_task_modal_key(
                        key, project_id, None,
                        title.clone(), desc.clone(), desc_cursor, priority, due_date_type,
                        due_date_val.clone(), focus_idx, parent_task_id, 0, true, recurrence,
                    )?;
                    // Swap back: modal_state restored to EditTask, overlay = new/None
                    std::mem::swap(&mut self.modal_state, &mut self.overlay_modal);
                }
            }
            return Ok(true);
        }

        match self.modal_state {
            ModalType::None => Ok(false),
            ModalType::QuitConfirm { .. } => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.should_quit = true;
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::UpdateAvailable { .. } => {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.run_installer_on_exit = true;
                        self.should_quit = true;
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.update_check_done = true;
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::ConfirmArchiveProject { project_id, .. } => {
                let pid = project_id;
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        // Perform the archive
                        if let Some(pos) = self.projects.iter().position(|p| p.id == pid) {
                            let mut p = self.projects[pos].clone();
                            p.archived = true;
                            self.db.update_project(&p)?;
                            self.mark_dirty();
                            self.grow_tree(20)?;
                            self.check_action_achievements()?;
                            self.apply_class_passive("project_archive", 0)?;
                            self.selected_project_idx = 0;
                            self.reload_data()?;
                        }
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::ConfirmDeleteProject { project_id, .. } => {
                let pid = project_id;
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.db.delete_project_permanently(pid)?;
                        self.mark_dirty();
                        self.selected_archive_idx = 0;
                        self.reload_data()?;
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::ConfirmDeleteCodex { codex_id, .. } => {
                let cid = codex_id;
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        self.db.delete_codex(cid)?;
                        self.mark_dirty();
                        self.selected_notes_flat_idx = 0;
                        self.selected_note_idx = 0;
                        self.reload_data()?;
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::RefileScroll { note_id, selected_idx } => {
                let nid = note_id;
                let mut sel = selected_idx;
                let total = self.codices.len() + 1; // 0 = Ungrouped, 1..=n = codices
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        sel = if sel > 0 { sel - 1 } else { total - 1 };
                        self.modal_state = ModalType::RefileScroll { note_id: nid, selected_idx: sel };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        sel = (sel + 1) % total;
                        self.modal_state = ModalType::RefileScroll { note_id: nid, selected_idx: sel };
                    }
                    KeyCode::Enter => {
                        if let Some(mut note) = self.db.get_note_by_id(nid).ok() {
                            note.codex_id = if sel == 0 {
                                None
                            } else {
                                self.codices.get(sel - 1).map(|c| c.id)
                            };
                            note.updated_at = chrono::Utc::now();
                            self.db.update_note(&note)?;
                            self.mark_dirty();
                            self.selected_notes_flat_idx = 0;
                            self.reload_data()?;
                        }
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::CustomFocusDuration { ref input } => {
                let mut val = input.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char(c) => {
                        if c.is_ascii_digit() && val.len() < 4 {
                            val.push(c);
                            self.modal_state = ModalType::CustomFocusDuration { input: val };
                        }
                    }
                    KeyCode::Backspace => {
                        val.pop();
                        self.modal_state = ModalType::CustomFocusDuration { input: val };
                    }
                    KeyCode::Enter => {
                        if let Ok(mins) = val.parse::<i32>() {
                            if mins > 0 {
                                // Start focus session configuration with custom minutes
                                self.modal_state = ModalType::None;
                                self.active_screen = ActiveScreen::Focus;
                                // Automatically choose the project/task dial or custom
                                // Start custom focus
                                self.start_focus_session(mins, None, None)?;
                            }
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::EditServerUrl { ref input } => {
                let mut val = input.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char(c) => {
                        val.push(c);
                        self.modal_state = ModalType::EditServerUrl { input: val };
                    }
                    KeyCode::Backspace => {
                        val.pop();
                        self.modal_state = ModalType::EditServerUrl { input: val };
                    }
                    KeyCode::Enter => {
                        self.server_url = val.clone();
                        let _ = self.db.set_setting("server_url", &val);
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::ExportProfile { .. } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char('c') => {
                        if let ModalType::ExportProfile { ref transfer_code } = self.modal_state.clone() {
                            match crate::services::identity::copy_to_clipboard(transfer_code) {
                                Ok(_) => {
                                    self.sync_status_msg = "Transfer Code copied to clipboard!".to_string();
                                    self.notifications.push(Notification::info("Transfer Code copied!".to_string()));
                                }
                                Err(e) => {
                                    self.sync_status_msg = format!("Copy Failed: {}", e);
                                }
                            }
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::RestoreIdentity { ref input } => {
                let mut val = input.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char(c) => {
                        val.push(c);
                        self.modal_state = ModalType::RestoreIdentity { input: val };
                    }
                    KeyCode::Backspace => {
                        val.pop();
                        self.modal_state = ModalType::RestoreIdentity { input: val };
                    }
                    KeyCode::Enter => {
                        use base64::{engine::general_purpose::STANDARD, Engine as _};
                        let trimmed = val.trim().to_string();
                        match STANDARD.decode(&trimmed) {
                            Ok(secret_bytes) if secret_bytes.len() == 32 => {
                                use ed25519_dalek::{SigningKey, VerifyingKey};
                                let secret_arr: [u8; 32] = secret_bytes.try_into().unwrap();
                                let signing_key = SigningKey::from_bytes(&secret_arr);
                                let verifying_key: VerifyingKey = signing_key.verifying_key();
                                let secret_hex: String = secret_arr.iter().map(|b| format!("{:02x}", b)).collect();
                                let public_hex: String = verifying_key.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();

                                let storage_dir = crate::storage::get_storage_dir()?;
                                let db_path = storage_dir.join("questline.db");
                                let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
                                let backup_path = storage_dir.join(format!("questline_backup_prerestore_{}.db", ts));
                                let _ = std::fs::copy(&db_path, &backup_path);

                                let new_identity = crate::services::identity::Identity {
                                    user_uuid: self.identity.user_uuid,
                                    public_key: public_hex,
                                    secret_key: secret_hex,
                                    created_at: self.identity.created_at.clone(),
                                };
                                let key_path = storage_dir.join("identity.key");
                                if let Ok(json_str) = serde_json::to_string_pretty(&new_identity) {
                                    let _ = std::fs::write(&key_path, json_str);
                                }
                                self.identity = new_identity;
                                self.modal_state = ModalType::None;
                                self.sync_status_msg = "Identity restored! Syncing now...".to_string();
                                self.notifications.push(Notification::info("Identity Restored from Transfer Code!".to_string()));
                                let _ = self.trigger_sync();
                                self.reload_data()?;
                            }
                            Ok(_) => {
                                self.sync_status_msg = "Invalid transfer code: wrong key length".to_string();
                            }
                            Err(_) => {
                                self.sync_status_msg = "Invalid transfer code: not valid base64".to_string();
                            }
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::LocalMusicFolder { ref input, ref suggestions, ref selected } => {
                let mut val = input.clone();
                let mut sel = *selected;
                let mut sugs = suggestions.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char(c) => {
                        val.push(c);
                        sugs = App::path_suggestions(&val);
                        sel = 0;
                        self.modal_state = ModalType::LocalMusicFolder { input: val, suggestions: sugs, selected: sel };
                    }
                    KeyCode::Backspace => {
                        val.pop();
                        sugs = App::path_suggestions(&val);
                        sel = 0;
                        self.modal_state = ModalType::LocalMusicFolder { input: val, suggestions: sugs, selected: sel };
                    }
                    KeyCode::Tab | KeyCode::Down => {
                        if !sugs.is_empty() {
                            sel = (sel + 1) % sugs.len();
                            val = format!("{}/", sugs[sel]);
                            sugs = App::path_suggestions(&val);
                            sel = 0;
                            self.modal_state = ModalType::LocalMusicFolder { input: val, suggestions: sugs, selected: sel };
                        }
                    }
                    KeyCode::BackTab | KeyCode::Up => {
                        if !sugs.is_empty() {
                            sel = if sel == 0 { sugs.len() - 1 } else { sel - 1 };
                            val = format!("{}/", sugs[sel]);
                            sugs = App::path_suggestions(&val);
                            sel = 0;
                            self.modal_state = ModalType::LocalMusicFolder { input: val, suggestions: sugs, selected: sel };
                        }
                    }
                    KeyCode::Enter => {
                        let _ = self.db.set_setting("local_music_folder", &val);
                        self.audio_player.set_local_music_folder(&val);
                        self.modal_state = ModalType::None;
                        let current_sc = self.audio_player.get_state().current_soundscape;
                        if current_sc == "Local Folder" || current_sc.starts_with("Local:") || current_sc.starts_with("Local Music:") {
                            self.audio_player.play("Local Folder");
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::DailyReflection {
                ref what_went_well,
                ref what_can_improve,
                focus_idx,
            } => {
                let mut well = what_went_well.clone();
                let mut improve = what_can_improve.clone();
                let mut idx = focus_idx;

                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Tab => {
                        idx = (idx + 1) % 2;
                        self.modal_state = ModalType::DailyReflection {
                            what_went_well: well,
                            what_can_improve: improve,
                            focus_idx: idx,
                        };
                    }
                    KeyCode::BackTab => {
                        idx = if idx > 0 { idx - 1 } else { 1 };
                        self.modal_state = ModalType::DailyReflection {
                            what_went_well: well,
                            what_can_improve: improve,
                            focus_idx: idx,
                        };
                    }
                    KeyCode::Char(c) => {
                        if idx == 0 {
                            if well.len() < 120 {
                                well.push(c);
                            }
                        } else {
                            if improve.len() < 120 {
                                improve.push(c);
                            }
                        }
                        self.modal_state = ModalType::DailyReflection {
                            what_went_well: well,
                            what_can_improve: improve,
                            focus_idx: idx,
                        };
                    }
                    KeyCode::Backspace => {
                        if idx == 0 {
                            well.pop();
                        } else {
                            improve.pop();
                        }
                        self.modal_state = ModalType::DailyReflection {
                            what_went_well: well,
                            what_can_improve: improve,
                            focus_idx: idx,
                        };
                    }
                    KeyCode::Enter => {
                        if idx == 0 {
                            idx = 1;
                            self.modal_state = ModalType::DailyReflection {
                                what_went_well: well,
                                what_can_improve: improve,
                                focus_idx: idx,
                            };
                        } else {
                            let ref_obj = DailyReflection {
                                created_date: chrono::Local::now().date_naive(),
                                what_went_well: well.trim().to_string(),
                                what_can_improve: improve.trim().to_string(),
                            };
                            self.db.insert_reflection(&ref_obj)?;
                            self.mark_dirty();
                            self.push_great_chronicle_async("ReflectionWritten", "wrote a daily reflection.", true);
                            self.grant_xp("Daily Reflection Logged", 25)?;
                            self.complete_productive_action()?;
                            self.reload_data()?;
                            self.modal_state = ModalType::None;
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::SpecializationSelect {
                ref choices,
                selected_idx,
            } => {
                let mut sel = selected_idx;
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Up | KeyCode::Left => {
                        sel = if sel > 0 { sel - 1 } else { choices.len() - 1 };
                        self.modal_state = ModalType::SpecializationSelect {
                            choices: choices.clone(),
                            selected_idx: sel,
                        };
                    }
                    KeyCode::Down | KeyCode::Right => {
                        sel = (sel + 1) % choices.len();
                        self.modal_state = ModalType::SpecializationSelect {
                            choices: choices.clone(),
                            selected_idx: sel,
                        };
                    }
                    KeyCode::Enter => {
                        if let Some(ref mut u) = self.user {
                            let selected_spec = choices[sel].clone();
                            u.specialization = Some(selected_spec.clone());
                            self.db.update_user(u)?;

                            // Write chronicle entry for specialization choice
                            let day_number = (Utc::now() - u.created_at).num_days() as i32 + 1;
                            self.db.add_chronicle_entry(
                                day_number,
                                &format!(
                                    "Embraced the calling of the specialization subclass: {}.",
                                    selected_spec
                                ),
                            )?;

                            // Unlock custom lore entry for subclass
                            let spec_desc = u
                                .class
                                .specializations()
                                .iter()
                                .find(|s| s.0 == selected_spec)
                                .map(|s| s.1)
                                .unwrap_or("");
                            let entry_id = format!(
                                "spec_lore_{}",
                                selected_spec.replace(" ", "_").to_lowercase()
                            );
                            self.db.insert_custom_lore_entry(
                                &entry_id,
                                "Specialization",
                                &selected_spec,
                                &format!(
                                    "The legend of the {}.\nFocus and path: {}",
                                    selected_spec, spec_desc
                                ),
                                true,
                            )?;

                            self.notifications.push(Notification::info(format!("Specialization Unlocked: {}!", selected_spec)));
                        }
                        self.mark_dirty();
                        self.reload_data()?;
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::NewRitual {
                ref name,
                ref desc,
                frequency_idx,
                ref reward_xp,
                focus_idx,
            } => {
                let mut r_name = name.clone();
                let mut r_desc = desc.clone();
                let mut freq_idx = frequency_idx;
                let mut xp_str = reward_xp.clone();
                let mut idx = focus_idx;

                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Tab => {
                        idx = (idx + 1) % 4;
                        self.modal_state = ModalType::NewRitual {
                            name: r_name,
                            desc: r_desc,
                            frequency_idx: freq_idx,
                            reward_xp: xp_str,
                            focus_idx: idx,
                        };
                    }
                    KeyCode::BackTab => {
                        idx = if idx > 0 { idx - 1 } else { 3 };
                        self.modal_state = ModalType::NewRitual {
                            name: r_name,
                            desc: r_desc,
                            frequency_idx: freq_idx,
                            reward_xp: xp_str,
                            focus_idx: idx,
                        };
                    }
                    KeyCode::Left | KeyCode::Right => {
                        if idx == 2 {
                            if key.code == KeyCode::Left {
                                freq_idx = if freq_idx > 0 { freq_idx - 1 } else { 4 };
                            } else {
                                freq_idx = (freq_idx + 1) % 5;
                            }
                            self.modal_state = ModalType::NewRitual {
                                name: r_name,
                                desc: r_desc,
                                frequency_idx: freq_idx,
                                reward_xp: xp_str,
                                focus_idx: idx,
                            };
                        }
                    }
                    KeyCode::Char(c) => {
                        if idx == 0 {
                            if r_name.len() < 30 {
                                r_name.push(c);
                            }
                        } else if idx == 1 {
                            if r_desc.len() < 80 {
                                r_desc.push(c);
                            }
                        } else if idx == 2 {
                            if c == ' ' {
                                freq_idx = (freq_idx + 1) % 5;
                            }
                        } else if idx == 3 && c.is_ascii_digit() && xp_str.len() < 4 {
                            xp_str.push(c);
                        }
                        self.modal_state = ModalType::NewRitual {
                            name: r_name,
                            desc: r_desc,
                            frequency_idx: freq_idx,
                            reward_xp: xp_str,
                            focus_idx: idx,
                        };
                    }
                    KeyCode::Backspace => {
                        if idx == 0 {
                            r_name.pop();
                        } else if idx == 1 {
                            r_desc.pop();
                        } else if idx == 3 {
                            xp_str.pop();
                        }
                        self.modal_state = ModalType::NewRitual {
                            name: r_name,
                            desc: r_desc,
                            frequency_idx: freq_idx,
                            reward_xp: xp_str,
                            focus_idx: idx,
                        };
                    }
                    KeyCode::Enter => {
                        if idx < 3 {
                            idx += 1;
                            self.modal_state = ModalType::NewRitual {
                                name: r_name,
                                desc: r_desc,
                                frequency_idx: freq_idx,
                                reward_xp: xp_str,
                                focus_idx: idx,
                            };
                        } else {
                            if !r_name.trim().is_empty() {
                                let freqs = ["Daily", "Weekdays", "Weekly", "Monthly", "Custom"];
                                let freq = freqs[freq_idx].to_string();
                                let xp_val = xp_str.parse::<i32>().unwrap_or(20);
                                if xp_val > 100 {
                                    self.notifications.push(Notification::warning("Sidequest reward cannot exceed 100 XP!"));
                                } else {
                                    let rit = Ritual {
                                        id: Uuid::new_v4().to_string(),
                                        name: r_name.trim().to_string(),
                                        description: if r_desc.trim().is_empty() {
                                            None
                                        } else {
                                            Some(r_desc.trim().to_string())
                                        },
                                        frequency: freq,
                                        reward_xp: xp_val,
                                        created_at: Utc::now(),
                                    };
                                    self.db.insert_ritual(&rit)?;
                                    self.mark_dirty();
                                    self.reload_data()?;
                                    self.modal_state = ModalType::None;
                                }
                            }
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::MilestoneTierSelect {
                project_id,
                selected_idx,
            } => {
                let mut idx = selected_idx;
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if idx > 0 {
                            idx -= 1;
                        }
                        self.modal_state = ModalType::MilestoneTierSelect {
                            project_id,
                            selected_idx: idx,
                        };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if idx < 2 {
                            idx += 1;
                        }
                        self.modal_state = ModalType::MilestoneTierSelect {
                            project_id,
                            selected_idx: idx,
                        };
                    }
                    KeyCode::Enter => {
                        let tier: u8 = (idx as u8) + 1; // 1=Initiate, 2=Veteran, 3=Legendary
                        self.modal_state = ModalType::MilestoneTemplateSelect {
                            project_id,
                            tier,
                            selected_idx: 0,
                        };
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::MilestoneTemplateSelect {
                project_id,
                tier,
                selected_idx,
            } => {
                use crate::milestone_templates::{Tier, templates_for_tier, get_template_by_id};
                let tier_enum = Tier::from_u8(tier).unwrap_or(Tier::Initiate);
                let templates: Vec<&'static crate::milestone_templates::MilestoneTemplate> =
                    templates_for_tier(tier_enum).collect();
                let count = templates.len();
                let mut idx = selected_idx;

                match key.code {
                    KeyCode::Esc => {
                        // Go back to tier select
                        self.modal_state = ModalType::MilestoneTierSelect {
                            project_id,
                            selected_idx: (tier as usize).saturating_sub(1),
                        };
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if idx > 0 {
                            idx -= 1;
                        }
                        self.modal_state = ModalType::MilestoneTemplateSelect {
                            project_id,
                            tier,
                            selected_idx: idx,
                        };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if count > 0 && idx < count - 1 {
                            idx += 1;
                        }
                        self.modal_state = ModalType::MilestoneTemplateSelect {
                            project_id,
                            tier,
                            selected_idx: idx,
                        };
                    }
                    KeyCode::Enter => {
                        if !templates.is_empty() {
                            let tmpl = templates[idx.min(templates.len() - 1)];
                            let mil = Milestone {
                                id: Uuid::new_v4(),
                                project_id,
                                name: tmpl.name.to_string(),
                                description: Some(tmpl.description.to_string()),
                                completed: false,
                                xp_reward: tmpl.xp_reward,
                                created_at: Utc::now(),
                                tier: tmpl.tier as u8,
                                template_id: tmpl.id.to_string(),
                            };
                            self.db.insert_milestone(&mil)?;
                            self.mark_dirty();
                            self.reload_data()?;
                            self.modal_state = ModalType::None;
                            self.notifications.push(Notification::info(format!(
                                    "Milestone unlocked: {}! Complete requirements to claim {} XP.",
                                    tmpl.name, tmpl.xp_reward
                                )));
                        }
                    }
                    _ => {}
                }
                // suppress unused warning for get_template_by_id in this block
                let _ = get_template_by_id;
                Ok(true)
            }
            ModalType::InviteMember {
                ref identity,
                ref username,
                role_idx,
                project_idx,
                focus_idx,
            } => {
                let mut id_str = identity.clone();
                let mut name_str = username.clone();
                let mut r_idx = role_idx;
                let mut p_idx = project_idx;
                let mut f_idx = focus_idx;

                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Tab => {
                        if f_idx == 1 && id_str.len() == 64 && name_str.is_empty() && self.config.sync_enabled {
                            let client = crate::services::api_client::ApiClient::new(
                                &self.server_url,
                                self.identity.clone(),
                                &self.device_id,
                            );
                            if let Some(found) = client.lookup_username(&id_str) {
                                name_str = found;
                            }
                        }
                        f_idx = (f_idx + 1) % 4;
                        self.modal_state = ModalType::InviteMember {
                            identity: id_str,
                            username: name_str,
                            role_idx: r_idx,
                            project_idx: p_idx,
                            focus_idx: f_idx,
                        };
                    }
                    KeyCode::BackTab => {
                        if f_idx == 1 && id_str.len() == 64 && name_str.is_empty() && self.config.sync_enabled {
                            let client = crate::services::api_client::ApiClient::new(
                                &self.server_url,
                                self.identity.clone(),
                                &self.device_id,
                            );
                            if let Some(found) = client.lookup_username(&id_str) {
                                name_str = found;
                            }
                        }
                        f_idx = if f_idx > 0 { f_idx - 1 } else { 3 };
                        self.modal_state = ModalType::InviteMember {
                            identity: id_str,
                            username: name_str,
                            role_idx: r_idx,
                            project_idx: p_idx,
                            focus_idx: f_idx,
                        };
                    }
                    KeyCode::Left => {
                        if f_idx == 0 {
                            let active_projects: Vec<_> = self.projects.iter().filter(|p| !p.archived).collect();
                            if !active_projects.is_empty() {
                                p_idx = if p_idx > 0 { p_idx - 1 } else { active_projects.len() - 1 };
                            }
                            self.modal_state = ModalType::InviteMember {
                                identity: id_str,
                                username: name_str,
                                role_idx: r_idx,
                                project_idx: p_idx,
                                focus_idx: f_idx,
                            };
                        } else if f_idx == 3 {
                            r_idx = if r_idx > 0 { r_idx - 1 } else { 3 };
                            self.modal_state = ModalType::InviteMember {
                                identity: id_str,
                                username: name_str,
                                role_idx: r_idx,
                                project_idx: p_idx,
                                focus_idx: f_idx,
                            };
                        }
                    }
                    KeyCode::Right => {
                        if f_idx == 0 {
                            let active_projects: Vec<_> = self.projects.iter().filter(|p| !p.archived).collect();
                            if !active_projects.is_empty() {
                                p_idx = (p_idx + 1) % active_projects.len();
                            }
                            self.modal_state = ModalType::InviteMember {
                                identity: id_str,
                                username: name_str,
                                role_idx: r_idx,
                                project_idx: p_idx,
                                focus_idx: f_idx,
                            };
                        } else if f_idx == 3 {
                            r_idx = (r_idx + 1) % 4;
                            self.modal_state = ModalType::InviteMember {
                                identity: id_str,
                                username: name_str,
                                role_idx: r_idx,
                                project_idx: p_idx,
                                focus_idx: f_idx,
                            };
                        }
                    }
                    KeyCode::Char(c) => {
                        if f_idx == 1 {
                            if id_str.len() < 64 {
                                id_str.push(c);
                            }
                            // Auto-fill companion name when key is complete
                            if id_str.len() == 64 && name_str.is_empty() && self.config.sync_enabled {
                                let client = crate::services::api_client::ApiClient::new(
                                    &self.server_url,
                                    self.identity.clone(),
                                    &self.device_id,
                                );
                                if let Some(found) = client.lookup_username(&id_str) {
                                    name_str = found;
                                }
                            }
                        } else if f_idx == 2 && name_str.len() < 24 {
                            name_str.push(c);
                        }
                        self.modal_state = ModalType::InviteMember {
                            identity: id_str,
                            username: name_str,
                            role_idx: r_idx,
                            project_idx: p_idx,
                            focus_idx: f_idx,
                        };
                    }
                    KeyCode::Backspace => {
                        if f_idx == 1 {
                            id_str.pop();
                        } else if f_idx == 2 {
                            name_str.pop();
                        }
                        self.modal_state = ModalType::InviteMember {
                            identity: id_str,
                            username: name_str,
                            role_idx: r_idx,
                            project_idx: p_idx,
                            focus_idx: f_idx,
                        };
                    }
                    KeyCode::Enter => {
                        if f_idx < 3 {
                            if f_idx == 1 && id_str.len() == 64 && name_str.is_empty() && self.config.sync_enabled {
                                let client = crate::services::api_client::ApiClient::new(
                                    &self.server_url,
                                    self.identity.clone(),
                                    &self.device_id,
                                );
                                if let Some(found) = client.lookup_username(&id_str) {
                                    name_str = found;
                                }
                            }
                            f_idx += 1;
                            self.modal_state = ModalType::InviteMember {
                                identity: id_str,
                                username: name_str,
                                role_idx: r_idx,
                                project_idx: p_idx,
                                focus_idx: f_idx,
                            };
                        } else {
                            // Enforce member invitation
                            let active_projects: Vec<_> =
                                self.projects.iter().filter(|p| !p.archived).collect();
                            if !active_projects.is_empty() && p_idx < active_projects.len() {
                                let proj = active_projects[p_idx];

                                // Ensure project is shared when inviting someone
                                if !proj.is_shared {
                                    let mut updated = proj.clone();
                                    updated.is_shared = true;
                                    self.db.update_project(&updated)?;

                                    // Add current user as owner
                                    self.db.add_project_member(
                                        &proj.id.to_string(),
                                        &self.identity.public_key,
                                        &self.user.as_ref().unwrap().username,
                                        "Owner",
                                    )?;
                                }

                                let roles = ["Owner", "Steward", "Companion", "Observer"];
                                let selected_role = roles[r_idx];

                                self.db.add_project_member(
                                    &proj.id.to_string(),
                                    &id_str,
                                    &name_str,
                                    selected_role,
                                )?;
                                if self.config.sync_enabled {
                                    let client = crate::services::api_client::ApiClient::new(
                                        &self.server_url,
                                        self.identity.clone(),
                                        &self.device_id,
                                    );
                                    let body = serde_json::json!({
                                        "project_id": proj.id.to_string(),
                                        "project_name": proj.name.clone(),
                                        "invitee_identity": id_str.clone(),
                                        "role": selected_role.to_string()
                                    })
                                    .to_string();
                                    match client.send_request("POST", "invite", &body) {
                                        Ok(_) => {
                                            self.notifications.push(Notification::info(format!("Invitation sent to {}!", name_str)));
                                        }
                                        Err(e) => {
                                            self.notifications.push(Notification::warning(format!("Failed to send invitation: {}", e)));
                                        }
                                    }
                                } else {
                                    self.notifications.push(Notification::info("Local activity logged. Enable sync to transmit invitation.".to_string()));
                                }
                                self.db.log_activity(
                                    Some(&proj.id.to_string()),
                                    "member_invited",
                                    &format!(
                                        "Invited {} to join the fellowship as a {}.",
                                        name_str, selected_role
                                    ),
                                    &self.identity.public_key,
                                    &self.user.as_ref().unwrap().username,
                                )?;

                                // Mentor achievement progress
                                let mentor_count = self
                                    .db
                                    .get_setting("mentor_invite_count")?
                                    .and_then(|s| s.parse::<i32>().ok())
                                    .unwrap_or(0)
                                    + 1;
                                self.db.set_setting(
                                    "mentor_invite_count",
                                    &mentor_count.to_string(),
                                )?;
                                if mentor_count >= 10 {
                                    self.db.conn.execute("UPDATE achievements SET unlocked_at = ?1 WHERE id = 'mentor' AND unlocked_at IS NULL", params![Utc::now().to_rfc3339()])?;
                                }
                            } else {
                                self.notifications.push(Notification::warning("No project selected for invitation!"));
                            }
                            self.mark_dirty();
                            self.modal_state = ModalType::None;
                            self.reload_data()?;
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::PostMessage { ref content } => {
                let mut val = content.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char(c) => {
                        if val.len() < 120 {
                            val.push(c);
                        }
                        self.modal_state = ModalType::PostMessage { content: val };
                    }
                    KeyCode::Backspace => {
                        val.pop();
                        self.modal_state = ModalType::PostMessage { content: val };
                    }
                    KeyCode::Enter => {
                        if !val.trim().is_empty() {
                            let shared_projects: Vec<_> =
                                self.projects.iter().filter(|p| p.is_shared).collect();
                            if !shared_projects.is_empty()
                                && self.selected_fellowship_project_idx < shared_projects.len()
                            {
                                let proj = shared_projects[self.selected_fellowship_project_idx];
                                let msg_id = self.db.add_chronicle_message(
                                    &proj.id.to_string(),
                                    &self.identity.public_key,
                                    &self.user.as_ref().unwrap().username,
                                    val.trim(),
                                    "text",
                                )?;
                                if self.config.sync_enabled {
                                    let client = crate::services::api_client::ApiClient::new(
                                        &self.server_url,
                                        self.identity.clone(),
                                        &self.device_id,
                                    );
                                    let body = serde_json::json!({
                                        "id": msg_id,
                                        "project_id": proj.id.to_string(),
                                        "content": val.trim().to_string(),
                                        "message_type": "text"
                                    })
                                    .to_string();
                                    let _ = std::thread::spawn(move || {
                                        let _ =
                                            client.send_request("POST", "chronicle/message", &body);
                                    });
                                }

                                self.db.log_activity(
                                    Some(&proj.id.to_string()),
                                    "message_posted",
                                    &format!("Posted a chronicle message: {}", val.trim()),
                                    &self.identity.public_key,
                                    &self.user.as_ref().unwrap().username,
                                )?;

                                // Chronicler of Fellowship achievement progress
                                let total_msgs = self.db.conn.query_row(
                                    "SELECT COUNT(*) FROM chronicle_messages WHERE sender_identity = ?1",
                                    params![self.identity.public_key],
                                    |row| row.get::<_, i32>(0)
                                )?;
                                if total_msgs >= 100 {
                                    self.db.conn.execute("UPDATE achievements SET unlocked_at = ?1 WHERE id = 'chronicler_fellowship' AND unlocked_at IS NULL", params![Utc::now().to_rfc3339()])?;
                                }
                            }
                        }
                        self.modal_state = ModalType::None;
                        self.reload_data()?;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::AddReaction { ref message_id } => {
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char(c) => {
                        let emoji = match c {
                            '1' => Some("👍"),
                            '2' => Some("⚔️"),
                            '3' => Some("🔥"),
                            '4' => Some("🌱"),
                            '5' => Some("🎉"),
                            '6' => Some("📜"),
                            _ => None,
                        };

                        if let Some(em) = emoji {
                            let msg_to_react = if message_id.is_nil() {
                                let shared_projects: Vec<_> =
                                    self.projects.iter().filter(|p| p.is_shared).collect();
                                if !shared_projects.is_empty()
                                    && self.selected_fellowship_project_idx < shared_projects.len()
                                {
                                    let proj =
                                        shared_projects[self.selected_fellowship_project_idx];
                                    let msgs = self
                                        .db
                                        .get_chronicle_messages(&proj.id.to_string())
                                        .unwrap_or_default();
                                    msgs.last().map(|m| m.0.clone()).unwrap_or_default()
                                } else {
                                    String::new()
                                }
                            } else {
                                message_id.to_string()
                            };

                            if !msg_to_react.is_empty() {
                                self.db.add_message_reaction(
                                    &msg_to_react,
                                    &self.identity.public_key,
                                    em,
                                )?;
                            }
                            self.modal_state = ModalType::None;
                            self.reload_data()?;
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::ProjectSharing { project_id } => {
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char('s') => {
                        if let Some(proj) = self.projects.iter().find(|p| p.id == project_id) {
                            let mut updated = proj.clone();
                            updated.is_shared = !proj.is_shared;
                            self.db.update_project(&updated)?;
                            self.mark_dirty();

                            // If sharing is enabled, automatically add the user as owner
                            if updated.is_shared {
                                self.db.add_project_member(
                                    &project_id.to_string(),
                                    &self.identity.public_key,
                                    &self.user.as_ref().unwrap().username,
                                    "Owner",
                                )?;
                            }
                        }
                        self.modal_state = ModalType::None;
                        self.reload_data()?;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::SearchMessages { ref query } => {
                let mut val = query.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Char(c) => {
                        if val.len() < 40 {
                            val.push(c);
                        }
                        self.modal_state = ModalType::SearchMessages { query: val };
                    }
                    KeyCode::Backspace => {
                        val.pop();
                        self.modal_state = ModalType::SearchMessages { query: val };
                    }
                    KeyCode::Enter => {
                        self.fellowship_search_query = val.clone();
                        self.fellowship_search_results =
                            self.db.search_chronicle_messages(&val).unwrap_or_default();
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::ShareNote {
                note_id,
                permission_idx,
            } => {
                let mut idx = permission_idx;
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Left => {
                        idx = if idx > 0 { idx - 1 } else { 2 };
                        self.modal_state = ModalType::ShareNote {
                            note_id,
                            permission_idx: idx,
                        };
                    }
                    KeyCode::Right => {
                        idx = (idx + 1) % 3;
                        self.modal_state = ModalType::ShareNote {
                            note_id,
                            permission_idx: idx,
                        };
                    }
                    KeyCode::Enter => {
                        if let Ok(mut note) = self.db.get_note_by_id(note_id) {
                            let permissions = ["read_only", "editable", "collaborative"];
                            note.sharing_permission = permissions[idx].to_string();
                            self.db.update_note(&note)?;
                            self.mark_dirty();
                        }
                        self.modal_state = ModalType::None;
                        self.reload_data()?;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::AssignTask {
                task_id,
                selected_member_idx,
            } => {
                let mut sel = selected_member_idx;
                match self.db.get_task_by_id(task_id) { Ok(task) => {
                    if let Some(proj_id) = task.project_id {
                        let members = self
                            .db
                            .get_project_members(&proj_id.to_string())
                            .unwrap_or_default();
                        if members.is_empty() {
                            self.modal_state = ModalType::None;
                            return Ok(true);
                        }
                        match key.code {
                            KeyCode::Esc => {
                                self.modal_state = ModalType::None;
                            }
                            KeyCode::Up => {
                                sel = if sel > 0 { sel - 1 } else { members.len() - 1 };
                                self.modal_state = ModalType::AssignTask {
                                    task_id,
                                    selected_member_idx: sel,
                                };
                            }
                            KeyCode::Down => {
                                sel = (sel + 1) % members.len();
                                self.modal_state = ModalType::AssignTask {
                                    task_id,
                                    selected_member_idx: sel,
                                };
                            }
                            KeyCode::Enter => {
                                let member = &members[sel];
                                let existing = self
                                    .db
                                    .get_task_assignments(&task_id.to_string())
                                    .unwrap_or_default();
                                if existing.iter().any(|a| a.0 == member.0) {
                                    self.db.conn.execute("DELETE FROM task_assignments WHERE task_id = ?1 AND user_identity = ?2", params![task_id.to_string(), member.0])?;
                                } else {
                                    self.db.assign_task(
                                        &task_id.to_string(),
                                        &member.0,
                                        &member.1,
                                    )?;
                                    if member.0 != self.identity.public_key {
                                        self.db.create_notification(
                                            "task_assignment",
                                            "Task Assigned",
                                            &format!(
                                                "You have been assigned to task: {}",
                                                task.title
                                            ),
                                            Some(&proj_id.to_string()),
                                        )?;
                                    }
                                }
                                self.modal_state = ModalType::None;
                                self.reload_data()?;
                            }
                            _ => {}
                        }
                    } else {
                        self.modal_state = ModalType::None;
                    }
                } _ => {
                    self.modal_state = ModalType::None;
                }}
                Ok(true)
            }
            ModalType::JournalVisibility {
                entry_id,
                visibility_idx,
            } => {
                let mut idx = visibility_idx;
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Left => {
                        idx = if idx > 0 { idx - 1 } else { 2 };
                        self.modal_state = ModalType::JournalVisibility {
                            entry_id,
                            visibility_idx: idx,
                        };
                    }
                    KeyCode::Right => {
                        idx = (idx + 1) % 3;
                        self.modal_state = ModalType::JournalVisibility {
                            entry_id,
                            visibility_idx: idx,
                        };
                    }
                    KeyCode::Enter => {
                        let vis_options = ["Private", "Project Visible", "Fellowship Visible"];
                        let selected_vis = vis_options[idx].to_string();
                        self.db.conn.execute(
                            "UPDATE journal_entries SET visibility = ?1 WHERE id = ?2",
                            params![selected_vis, entry_id.to_string()],
                        )?;
                        self.modal_state = ModalType::None;
                        self.reload_data()?;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::ThemeSelect {
                ref choices,
                selected_idx,
            } => {
                let mut sel = selected_idx;
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Up | KeyCode::Left => {
                        sel = if sel > 0 { sel - 1 } else { choices.len() - 1 };
                        self.modal_state = ModalType::ThemeSelect {
                            choices: choices.clone(),
                            selected_idx: sel,
                        };
                    }
                    KeyCode::Down | KeyCode::Right => {
                        sel = (sel + 1) % choices.len();
                        self.modal_state = ModalType::ThemeSelect {
                            choices: choices.clone(),
                            selected_idx: sel,
                        };
                    }
                    KeyCode::Enter => {
                        let selected_choice = &choices[sel];
                        self.db.set_setting("equipped_theme", selected_choice)?;

                        let choice = match selected_choice.as_str() {
                            "Forest" => ThemeChoice::Forest,
                            "AncientLibrary" => ThemeChoice::AncientLibrary,
                            "MountainFortress" => ThemeChoice::MountainFortress,
                            "ArcaneWorkshop" => ThemeChoice::ArcaneWorkshop,
                            "OceanTemple" => ThemeChoice::OceanTemple,
                            _ => ThemeChoice::ClassDefault,
                        };
                        self.theme_service.set_theme_choice(choice);

                        self.notifications.push(Notification::info(format!("Theme Equipped: {}!", selected_choice)));
                        self.modal_state = ModalType::None;
                        self.reload_data()?;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::ChapterComplete => {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::Celebration { .. } => {
                match key.code {
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::SearchEverywhere {
                ref query,
                selected_idx,
                ref results,
            } => {
                let mut q = query.clone();
                let mut sel = selected_idx;
                let mut res = results.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Up => {
                        if !res.is_empty() {
                            sel = if sel > 0 { sel - 1 } else { res.len() - 1 };
                            self.modal_state = ModalType::SearchEverywhere {
                                query: q,
                                selected_idx: sel,
                                results: res,
                            };
                        }
                    }
                    KeyCode::Down => {
                        if !res.is_empty() {
                            sel = (sel + 1) % res.len();
                            self.modal_state = ModalType::SearchEverywhere {
                                query: q,
                                selected_idx: sel,
                                results: res,
                            };
                        }
                    }
                    KeyCode::Char(c) => {
                        q.push(c);
                        res = self.perform_unified_search(&q);
                        self.modal_state = ModalType::SearchEverywhere {
                            query: q,
                            selected_idx: 0,
                            results: res,
                        };
                    }
                    KeyCode::Backspace => {
                        q.pop();
                        res = self.perform_unified_search(&q);
                        self.modal_state = ModalType::SearchEverywhere {
                            query: q,
                            selected_idx: 0,
                            results: res,
                        };
                    }
                    KeyCode::Enter if !res.is_empty() && sel < res.len() => {
                        let selected_item = res[sel].clone();
                        self.navigate_to_search_result(&selected_item)?;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::CommandPalette {
                ref query,
                selected_idx,
                ref actions,
            } => {
                let mut q = query.clone();
                let mut sel = selected_idx;
                let mut act = actions.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Up => {
                        if !act.is_empty() {
                            sel = if sel > 0 { sel - 1 } else { act.len() - 1 };
                            self.modal_state = ModalType::CommandPalette {
                                query: q,
                                selected_idx: sel,
                                actions: act,
                            };
                        }
                    }
                    KeyCode::Down => {
                        if !act.is_empty() {
                            sel = (sel + 1) % act.len();
                            self.modal_state = ModalType::CommandPalette {
                                query: q,
                                selected_idx: sel,
                                actions: act,
                            };
                        }
                    }
                    KeyCode::Char(c) => {
                        q.push(c);
                        act = self.get_available_command_actions(&q);
                        self.modal_state = ModalType::CommandPalette {
                            query: q,
                            selected_idx: 0,
                            actions: act,
                        };
                    }
                    KeyCode::Backspace => {
                        q.pop();
                        act = self.get_available_command_actions(&q);
                        self.modal_state = ModalType::CommandPalette {
                            query: q,
                            selected_idx: 0,
                            actions: act,
                        };
                    }
                    KeyCode::Enter if !act.is_empty() && sel < act.len() => {
                        let action = act[sel].clone();
                        self.execute_command_action(action.id)?;
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::SelectProjectForAction { action_id, selected_idx } => {
                let projects: Vec<&Project> = self.projects.iter()
                    .filter(|p| !p.archived && !p.completed)
                    .collect();
                let mut sel = selected_idx;
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        sel = if sel > 0 { sel - 1 } else { projects.len().saturating_sub(1) };
                        self.modal_state = ModalType::SelectProjectForAction { action_id, selected_idx: sel };
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !projects.is_empty() {
                            sel = (sel + 1) % projects.len();
                        }
                        self.modal_state = ModalType::SelectProjectForAction { action_id, selected_idx: sel };
                    }
                    KeyCode::Enter if !projects.is_empty() => {
                        let project = projects[sel];
                        self.active_project_id = Some(project.id);
                        self.active_screen = ActiveScreen::Workspace;
                        self.reload_data()?;
                        if action_id == "create_task" {
                            self.workspace_tab_idx = 0;
                            self.modal_state = ModalType::NewTask {
                                title: String::new(),
                                desc: String::new(),
                                desc_cursor: 0,
                                priority: crate::models::task::TaskPriority::Medium,
                                due_date_type: crate::app::DueDateType::InDays,
                                due_date_val: "1".to_string(),
                                focus_idx: 0,
                                parent_task_id: None,
                                recurrence: None,
                            };
                        } else {
                            self.workspace_tab_idx = 1;
                            self.modal_state = ModalType::NewJournalEntry {
                                content: String::new(),
                            };
                        }
                    }
                    _ => {}
                }
                Ok(true)
            }
            ModalType::KeyboardHelp => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') => {
                        self.modal_state = ModalType::None;
                    }
                    _ => {}
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    // Onboarding Key handling.
    fn handle_onboarding_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.onboarding_focus {
            OnboardingFocus::NameInput => match key.code {
                KeyCode::Char(c) => {
                    if self.onboarding_username.len() < 24 {
                        self.onboarding_username.push(c);
                        self.onboarding_error = None;
                    }
                }
                KeyCode::Backspace => {
                    self.onboarding_username.pop();
                    self.onboarding_error = None;
                }
                KeyCode::Tab | KeyCode::Enter => {
                    if self.onboarding_username.trim().is_empty() {
                        self.onboarding_error = Some("Name cannot be empty.".to_string());
                    } else {
                        self.onboarding_error = None;
                        self.onboarding_focus = OnboardingFocus::ClassSelect;
                    }
                }
                _ => {}
            },
            OnboardingFocus::ClassSelect => match key.code {
                KeyCode::Up => {
                    if self.onboarding_class_idx > 0 {
                        self.onboarding_class_idx -= 1;
                    } else {
                        self.onboarding_class_idx = self.onboarding_classes.len() - 1;
                    }
                }
                KeyCode::Down => {
                    if self.onboarding_class_idx < self.onboarding_classes.len() - 1 {
                        self.onboarding_class_idx += 1;
                    } else {
                        self.onboarding_class_idx = 0;
                    }
                }
                KeyCode::Tab => {
                    self.onboarding_focus = OnboardingFocus::NameInput;
                }
                KeyCode::Enter => {
                    self.complete_onboarding()?;
                }
                _ => {}
            },
        }
        Ok(())
    }

    // Complete onboarding identity generation.
    fn complete_onboarding(&mut self) -> Result<()> {
        let username = self.onboarding_username.trim().to_string();

        if username.is_empty() {
            self.onboarding_error = Some("Name cannot be empty.".to_string());
            return Ok(());
        }

        let known_names = self.db.get_all_known_usernames().unwrap_or_default();
        let name_taken = known_names
            .iter()
            .any(|n| n.to_lowercase() == username.to_lowercase());
        if name_taken {
            use rand::Rng;
            let suffix: u8 = rand::thread_rng().gen_range(10..=99);
            let suggestion = format!("{}{}", username, suffix);
            self.onboarding_error = Some(format!(
                "'{}' is already taken. How about '{}'?",
                username, suggestion
            ));
            self.onboarding_username = suggestion;
            return Ok(());
        }

        let selected_class = self.onboarding_classes[self.onboarding_class_idx];

        let new_user = User {
            id: self.identity.user_uuid,
            username: username.clone(),
            class: selected_class,
            level: 1,
            xp: 0,
            created_at: Utc::now(),
            specialization: None,
        };

        self.db.insert_user(&new_user)?;
        self.mark_dirty();
        self.user = Some(new_user);
        self.theme_service.set_class(selected_class);

        // Grant starting XP
        let xp_service = XPService::new(&self.db);
        if let Some(ref mut u) = self.user {
            xp_service.grant_xp(u, "Onboarding Completed", 50)?;
        }

        self.reload_data()?;
        self.active_screen = ActiveScreen::Dashboard;
        self.active_tab_idx = 0;

        Ok(())
    }

    // Trigger sync service execution.
    pub fn trigger_sync(&mut self) -> Result<()> {
        self.sync_status_msg = "Syncing...".to_string();

        let server_url_opt = if self.config.sync_enabled {
            Some(self.server_url.as_str())
        } else {
            None
        };

        let sync_engine = match crate::services::sync_engine::SyncEngine::new(
            &self.db,
            &self.identity,
            &self.device_id,
            server_url_opt,
        ) {
            Ok(se) => se,
            Err(e) => {
                self.sync_status_msg = format!("Sync init failed: {}", e);
                return Err(e);
            }
        };

        match sync_engine.sync() {
            Ok((pushed, pulled, conflicts)) => {
                let now_str = chrono::Utc::now().to_rfc3339();
                let _ = self.db.set_setting("last_sync", &now_str);

                if self.config.sync_enabled {
                    let client = crate::services::api_client::ApiClient::new(
                        &self.server_url,
                        self.identity.clone(),
                        &self.device_id,
                    );

                    // 1. Pull pending invitations from server
                    if let Ok(resp_str) = client.send_request("GET", "pending", "") {
                        if let Ok(server_invites) =
                            serde_json::from_str::<serde_json::Value>(&resp_str)
                        {
                            if let Some(arr) = server_invites.as_array() {
                                for inv_val in arr {
                                    // check mapping fields
                                    let id = inv_val["id"].as_str().unwrap_or_default().to_string();
                                    let project_id = inv_val["project_id"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string();
                                    let project_name = inv_val["project_name"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string();
                                    let inviter_identity = inv_val["inviter_identity"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string();
                                    let inviter_username = inv_val["inviter_username"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string();
                                    let invitee_identity = inv_val["invitee_identity"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string();
                                    let role =
                                        inv_val["role"].as_str().unwrap_or_default().to_string();
                                    let status =
                                        inv_val["status"].as_str().unwrap_or("Pending").to_string();
                                    let created_at = inv_val["created_at"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string();

                                    let _ = self.db.conn.execute(
                                        "INSERT OR IGNORE INTO invitations (id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                        rusqlite::params![id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at]
                                    );
                                }
                            }
                        }
                    }

                    // 2. Fetch chronicle messages for each shared project
                    if let Ok(projs) = self.db.get_projects() {
                        let shared_projs: Vec<_> =
                            projs.into_iter().filter(|p| p.is_shared).collect();
                        for p in shared_projs {
                            let path = format!("chronicle/messages?project_id={}", p.id);
                            if let Ok(resp_str) = client.send_request("GET", &path, "") {
                                if let Ok(server_msgs) =
                                    serde_json::from_str::<serde_json::Value>(&resp_str)
                                {
                                    if let Some(arr) = server_msgs.as_array() {
                                        for msg_val in arr {
                                            let id = msg_val["id"]
                                                .as_str()
                                                .unwrap_or_default()
                                                .to_string();
                                            let project_id = msg_val["project_id"]
                                                .as_str()
                                                .unwrap_or_default()
                                                .to_string();
                                            let sender_identity = msg_val["sender_identity"]
                                                .as_str()
                                                .unwrap_or_default()
                                                .to_string();
                                            let sender_username = msg_val["sender_username"]
                                                .as_str()
                                                .unwrap_or_default()
                                                .to_string();
                                            let content = msg_val["content"]
                                                .as_str()
                                                .unwrap_or_default()
                                                .to_string();
                                            let message_type = msg_val["message_type"]
                                                .as_str()
                                                .unwrap_or("text")
                                                .to_string();
                                            let timestamp = msg_val["timestamp"]
                                                .as_str()
                                                .unwrap_or_default()
                                                .to_string();

                                            let _ = self.db.conn.execute(
                                                "INSERT OR IGNORE INTO chronicle_messages (id, project_id, sender_identity, sender_username, content, message_type, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                                                rusqlite::params![id, project_id, sender_identity, sender_username, content, message_type, timestamp]
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // 3. Fetch companion presence from shared projects
                    let _ = self.refresh_companions(&client);

                    // 4. Submit chapter contribution increments
                    self.submit_chapter_contribution(&client);

                    // 5. Refresh active chapter progress
                    self.refresh_chapter_progress_sync(&client);
                } else {
                    let _ = self.simulate_fellowship_sync();
                }

                let sync_count = self
                    .db
                    .get_setting("sync_count")?
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                self.db
                    .set_setting("sync_count", &(sync_count + 1).to_string())?;

                if !conflicts.is_empty() {
                    let conflict_count = self
                        .db
                        .get_setting("conflict_count")?
                        .and_then(|s| s.parse::<i32>().ok())
                        .unwrap_or(0);
                    self.db.set_setting(
                        "conflict_count",
                        &(conflict_count + conflicts.len() as i32).to_string(),
                    )?;
                    self.sync_conflicts = conflicts;
                }

                self.last_sync_warlock_xp = 0;
                self.sync_status_msg = format!("↑{} pushed  ↓{} pulled", pushed, pulled);
                self.last_sync_status_time = Some(std::time::Instant::now());
                self.apply_class_passive("sync_complete", 0)?;
            }
            Err(e) => {
                self.sync_status_msg = format!("Sync failed: {}", e);
                self.last_sync_status_time = Some(std::time::Instant::now());
            }
        }

        self.reload_data()?;
        Ok(())
    }

    fn fellowship_current_messages(&self) -> Vec<(String, String, String, String, String, String, String)> {
        let shared: Vec<_> = self.projects.iter().filter(|p| p.is_shared).collect();
        if shared.is_empty() || self.selected_fellowship_project_idx >= shared.len() {
            return vec![];
        }
        let proj = shared[self.selected_fellowship_project_idx];
        self.db.get_chronicle_messages(&proj.id.to_string()).unwrap_or_default()
    }

    fn send_fellowship_message(&mut self) -> Result<()> {
        let content = self.fellowship_chat_input.trim().to_string();
        if content.is_empty() { return Ok(()); }

        let shared: Vec<_> = self.projects.iter().filter(|p| p.is_shared).collect();
        if shared.is_empty() || self.selected_fellowship_project_idx >= shared.len() {
            return Ok(());
        }
        let proj = shared[self.selected_fellowship_project_idx].clone();
        let my_name = self.user.as_ref().map(|u| u.username.clone()).unwrap_or_default();

        let msg_id = self.db.add_chronicle_message(
            &proj.id.to_string(),
            &self.identity.public_key,
            &my_name,
            &content,
            "text",
        )?;

        if self.config.sync_enabled {
            let client = crate::services::api_client::ApiClient::new(
                &self.server_url, self.identity.clone(), &self.device_id,
            );
            let body = serde_json::json!({
                "id": msg_id,
                "project_id": proj.id.to_string(),
                "content": content,
                "message_type": "text"
            }).to_string();
            let _ = client.send_request("POST", "chronicle/message", &body);
        }

        self.fellowship_chat_input.clear();
        self.fellowship_selected_msg_idx = usize::MAX;
        self.fellowship_composing = false;
        self.reload_data()?;
        Ok(())
    }

    // Returns true if the key was fully handled (caller should return early).
    fn handle_fellowship_chat_key(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Left panel focused: Enter/→ switches to right panel, everything else falls through
        if self.fellowship_focus_left {
            match key.code {
                KeyCode::Enter | KeyCode::Right => {
                    self.fellowship_focus_left = false;
                    return Ok(true);
                }
                _ => return Ok(false),
            }
        }

        let browsing = self.fellowship_selected_msg_idx != usize::MAX;

        // Browse mode: arrow navigation + react/copy — always active regardless of compose state
        match key.code {
            KeyCode::Up => {
                let msgs = self.fellowship_current_messages();
                if msgs.is_empty() { return Ok(false); }
                self.fellowship_selected_msg_idx = if browsing && self.fellowship_selected_msg_idx > 0 {
                    self.fellowship_selected_msg_idx - 1
                } else {
                    msgs.len() - 1
                };
                return Ok(true);
            }
            KeyCode::Down => {
                if browsing {
                    let msgs = self.fellowship_current_messages();
                    if self.fellowship_selected_msg_idx + 1 >= msgs.len() {
                        self.fellowship_selected_msg_idx = usize::MAX;
                    } else {
                        self.fellowship_selected_msg_idx += 1;
                    }
                    return Ok(true);
                }
            }
            KeyCode::Char('r') if browsing => {
                let msgs = self.fellowship_current_messages();
                if self.fellowship_selected_msg_idx < msgs.len() {
                    let msg_id = msgs[self.fellowship_selected_msg_idx].0.clone();
                    self.modal_state = ModalType::AddReaction {
                        message_id: Uuid::parse_str(&msg_id).unwrap_or(Uuid::nil()),
                    };
                }
                return Ok(true);
            }
            KeyCode::Char('c') if browsing => {
                let msgs = self.fellowship_current_messages();
                if self.fellowship_selected_msg_idx < msgs.len() {
                    let content = &msgs[self.fellowship_selected_msg_idx].4;
                    let to_copy = extract_url(content).unwrap_or(content.as_str()).to_string();
                    let _ = crate::services::identity::copy_to_clipboard(&to_copy);
                    self.notifications.push(Notification::info("Copied to clipboard.".to_string()));
                }
                return Ok(true);
            }
            KeyCode::Esc if browsing => {
                self.fellowship_selected_msg_idx = usize::MAX;
                return Ok(true);
            }
            _ => {}
        }

        // Compose mode: all keys go to input
        if self.fellowship_composing {
            match key.code {
                KeyCode::Enter => {
                    if !self.fellowship_chat_input.is_empty() {
                        self.send_fellowship_message()?;
                    }
                    return Ok(true);
                }
                KeyCode::Esc => {
                    self.fellowship_chat_input.clear();
                    self.fellowship_composing = false;
                    return Ok(true);
                }
                KeyCode::Backspace => {
                    self.fellowship_chat_input.pop();
                    return Ok(true);
                }
                KeyCode::Left => {
                    // Allow leaving compose mode if input is empty
                    if self.fellowship_chat_input.is_empty() {
                        self.fellowship_composing = false;
                        self.fellowship_focus_left = true;
                    }
                    return Ok(true);
                }
                KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.fellowship_chat_input.push(c);
                    return Ok(true);
                }
                _ => return Ok(true), // swallow unhandled keys in compose mode
            }
        }

        // Command mode (not composing, not browsing): Enter starts composing, other keys fall through
        if let KeyCode::Enter = key.code {
            self.fellowship_composing = true;
            return Ok(true);
        }
        if let KeyCode::Left = key.code {
            self.fellowship_focus_left = true;
            return Ok(true);
        }
        if let KeyCode::Esc = key.code {
            self.fellowship_focus_left = true;
            return Ok(true);
        }

        // All other keys fall through to main handler (shortcuts work in command mode)
        Ok(false)
    }

    pub fn refresh_companions(&self, client: &crate::services::api_client::ApiClient) -> Result<()> {
        if let Ok(resp_str) = client.send_request("GET", "project/companions", "") {
            if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&resp_str) {
                if let Some(companions) = arr.as_array() {
                    for comp in companions {
                        let identity = comp["user_identity"].as_str().unwrap_or_default();
                        let username = comp["user_username"].as_str().unwrap_or_default();
                        if identity.is_empty() || username.is_empty() {
                            continue;
                        }
                        let last_seen_raw = comp["last_seen"].as_str().unwrap_or_default();
                        let (is_online, last_seen_display) = Self::parse_companion_last_seen(last_seen_raw);
                        let _ = self.db.update_presence(
                            identity,
                            username,
                            is_online,
                            &last_seen_display,
                            None,
                            if is_online { "Visible" } else { "Offline" },
                        );
                        // Also fix stale username in local project_members
                        let _ = self.db.conn.execute(
                            "UPDATE project_members SET user_username = ?1 WHERE user_identity = ?2 AND user_username = 'Accepted Companion'",
                            rusqlite::params![username, identity],
                        );
                    }
                }
            }
        }
        Ok(())
    }

    // Markdown Editor Key handling.
    fn handle_editor_key(&mut self, key: KeyEvent) -> Result<()> {
        let state = if let Some(ref mut s) = self.editor_state {
            s
        } else {
            self.active_screen = ActiveScreen::Workspace;
            return Ok(());
        };

        // Save command: Ctrl+S
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            let note_title = if state.title.trim().is_empty() {
                "Untitled Scroll".to_string()
            } else {
                state.title.trim().to_string()
            };
            let content = state.get_content();
            let word_count = content.len();

            let is_new_note = state.note_id.is_none();
            let xp_service = XPService::new(&self.db);
            if let Some(note_id) = state.note_id {
                // Editing note
                let mut note = Note {
                    id: note_id,
                    project_id: Some(state.project_id),
                    title: note_title,
                    markdown_content: content,
                    created_at: Utc::now(), // will keep original below
                    updated_at: Utc::now(),
                    sharing_permission: "collaborative".to_string(),
                    codex_id: None,
                };
                // Fetch original created_at and codex_id if exists
                if let Ok(notes) = self.db.get_notes() {
                    if let Some(orig) = notes.iter().find(|n| n.id == note_id) {
                        note.created_at = orig.created_at;
                        note.sharing_permission = orig.sharing_permission.clone();
                        note.codex_id = orig.codex_id;
                    }
                }
                self.db.update_note(&note)?;

                if let Some(ref mut u) = self.user {
                    let mut xp_gain = 2; // Edit Note = +2 XP
                    if word_count > 500 {
                        xp_gain += 10; // Large Note bonus = +10 XP
                    }
                    let leveled_up = xp_service.grant_xp(u, "Edit Scroll Note", xp_gain)?;
                    if leveled_up {
                        self.notifications.push(Notification::info(format!("LEVEL UP! You reached Level {}!", u.level)));
                    }
                }
            } else {
                // New note
                let note = Note {
                    id: Uuid::new_v4(),
                    project_id: Some(state.project_id),
                    title: note_title,
                    markdown_content: content,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    sharing_permission: "collaborative".to_string(),
                    codex_id: state.codex_id,
                };
                self.db.insert_note(&note)?;
                self.push_great_chronicle_async("ScrollCreated", "wrote a scroll.", true);

                if let Some(ref mut u) = self.user {
                    let mut xp_gain = 5; // Create Note = +5 XP
                    if word_count > 500 {
                        xp_gain += 10; // Large Note bonus = +10 XP
                    }
                    let leveled_up = xp_service.grant_xp(u, "Write New Scroll Note", xp_gain)?;
                    if leveled_up {
                        self.notifications.push(Notification::info(format!("LEVEL UP! You reached Level {}!", u.level)));
                    }
                }

                // Stage 3 Integration:
                self.complete_productive_action()?;
                self.update_daily_adventure_progress("write_note", 1)?;
                self.check_action_achievements()?;
            }

            self.mark_dirty();
            let note_trigger = if is_new_note { "note_create" } else { "note_edit" };
            self.apply_class_passive(note_trigger, word_count)?;

            self.reload_data()?;
            self.editor_state = None;
            self.active_screen = ActiveScreen::Workspace;
            self.workspace_tab_idx = 1; // Return to Notes Tab
            return Ok(());
        }

        // Cancel command: ESC
        if key.code == KeyCode::Esc {
            self.editor_state = None;
            self.active_screen = ActiveScreen::Workspace;
            return Ok(());
        }

        // Typing and navigation in editor
        match key.code {
            KeyCode::Up => state.move_up(),
            KeyCode::Down => state.move_down(),
            KeyCode::Left => state.move_left(),
            KeyCode::Right => state.move_right(),
            KeyCode::BackTab => state.editing_title = !state.editing_title,
            KeyCode::Char(c) => state.insert_char(c),
            KeyCode::Backspace => state.handle_backspace(),
            KeyCode::Delete => state.handle_delete(),
            KeyCode::Enter => state.handle_enter(),
            KeyCode::Tab => state.handle_tab(),
            _ => {}
        }

        Ok(())
    }

    // Top Level Screens Key handling.
    fn handle_top_screen_key(&mut self, key: KeyEvent) -> Result<()> {
        // GreatChronicle: Esc unfocuses right panel (or falls through to Dashboard navigation)
        if self.active_screen == ActiveScreen::GreatChronicle
            && key.code == KeyCode::Esc
            && self.chapter_panel_focused
        {
            self.chapter_panel_focused = false;
            return Ok(());
        }

        // GreatChronicle: Left arrow returns focus to the left panel
        if self.active_screen == ActiveScreen::GreatChronicle
            && key.code == KeyCode::Left
            && self.chapter_panel_focused
        {
            self.chapter_panel_focused = false;
            return Ok(());
        }

        // GreatChronicle: Right arrow focuses the right chapter panel
        if self.active_screen == ActiveScreen::GreatChronicle
            && key.code == KeyCode::Right
            && !self.chapter_panel_focused
        {
            self.chapter_panel_focused = true;
            return Ok(());
        }

        if self.active_screen == ActiveScreen::SyncSettings {
            match key.code {
                KeyCode::Char('s') | KeyCode::Enter => {
                    if self.config.sync_enabled {
                        self.start_forced_sync();
                    } else {
                        let _ = self.trigger_sync();
                    }
                    return Ok(());
                }
                KeyCode::Char('a') => {
                    self.auto_sync = !self.auto_sync;
                    let _ = self
                        .db
                        .set_setting("auto_sync", if self.auto_sync { "true" } else { "false" });
                    self.sync_status_msg = format!(
                        "Auto Sync {}",
                        if self.auto_sync {
                            "Enabled"
                        } else {
                            "Disabled"
                        }
                    );
                    return Ok(());
                }
                KeyCode::Char('u') => {
                    self.modal_state = ModalType::EditServerUrl {
                        input: self.server_url.clone(),
                    };
                    return Ok(());
                }
                KeyCode::Char('b') => {
                    if self.config.sync_enabled {
                        self.sync_status_msg = "Creating Backup...".to_string();
                        let json = self.db.export_to_json()?;
                        let client = crate::services::api_client::ApiClient::new(
                            &self.server_url,
                            self.identity.clone(),
                            &self.device_id,
                        );
                        match client.send_request("POST", "recovery", &json) {
                            Ok(_) => {
                                self.sync_status_msg = "Cloud Backup Successful!".to_string();
                                self.notifications.push(Notification::info("Cloud Backup Saved!".to_string()));
                            }
                            Err(e) => {
                                self.sync_status_msg = format!("Backup Failed: {}", e);
                            }
                        }
                    } else {
                        self.sync_status_msg = "Backup requires Cloud Sync enabled".to_string();
                    }
                    return Ok(());
                }
                KeyCode::Char('r') => {
                    if self.config.sync_enabled {
                        self.sync_status_msg = "Restoring Backup...".to_string();
                        let client = crate::services::api_client::ApiClient::new(
                            &self.server_url,
                            self.identity.clone(),
                            &self.device_id,
                        );
                        match client.send_request("GET", "recovery/latest", "") {
                            Ok(json) => {
                                if !json.trim().is_empty() {
                                    let decoded_json = if let Ok(decoded_bytes) = {
                                        use base64::{
                                            engine::general_purpose::STANDARD, Engine as _,
                                        };
                                        STANDARD.decode(json.trim())
                                    } {
                                        String::from_utf8(decoded_bytes)
                                            .unwrap_or_else(|_| json.clone())
                                    } else {
                                        json.clone()
                                    };

                                    match self.db.import_from_json(&decoded_json) { Ok(_) => {
                                        self.sync_status_msg =
                                            "Cloud Restore Complete! Reloading...".to_string();
                                        self.reload_data()?;
                                        self.notifications.push(Notification::info("Database Restored from Cloud!".to_string()));
                                    } _ => {
                                        self.sync_status_msg =
                                            "Restore Failed: Invalid data structure".to_string();
                                    }}
                                } else {
                                    self.sync_status_msg =
                                        "Restore Failed: Empty backup content".to_string();
                                }
                            }
                            Err(e) => {
                                self.sync_status_msg = format!("Restore Failed: {}", e);
                            }
                        }
                    } else {
                        self.sync_status_msg = "Restore requires Cloud Sync enabled".to_string();
                    }
                    return Ok(());
                }
                KeyCode::Char('c') => {
                    match crate::services::identity::copy_to_clipboard(&self.identity.public_key) {
                        Ok(_) => {
                            self.sync_status_msg = "Copied Share Key to Clipboard!".to_string();
                            self.notifications.push(Notification::info("Share Key copied to clipboard!".to_string()));
                        }
                        Err(e) => {
                            self.sync_status_msg = format!("Copy Failed: {}", e);
                        }
                    }
                    return Ok(());
                }
                KeyCode::Char('e') => {
                    use base64::{engine::general_purpose::STANDARD, Engine as _};
                    let storage_dir = crate::storage::get_storage_dir()?;
                    let db_path = storage_dir.join("questline.db");
                    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
                    let backup_path = storage_dir.join(format!("questline_backup_{}.db", ts));
                    match std::fs::copy(&db_path, &backup_path) {
                        Ok(_) => {
                            let secret_bytes: Vec<u8> = self.identity.secret_key
                                .as_bytes()
                                .chunks(2)
                                .filter_map(|c| {
                                    let s = std::str::from_utf8(c).ok()?;
                                    u8::from_str_radix(s, 16).ok()
                                })
                                .collect();
                            let transfer_code = STANDARD.encode(&secret_bytes);
                            self.sync_status_msg = format!("Profile exported! Backup saved to {}", backup_path.display());
                            self.modal_state = ModalType::ExportProfile { transfer_code };
                        }
                        Err(e) => {
                            self.sync_status_msg = format!("Export Failed: {}", e);
                        }
                    }
                    return Ok(());
                }
                KeyCode::Char('i') => {
                    self.modal_state = ModalType::RestoreIdentity { input: String::new() };
                    return Ok(());
                }
                _ => {}
            }
        }

        // If modal dialog is active on Projects screen
        if self.active_screen == ActiveScreen::Projects && self.modal_state != ModalType::None {
            self.handle_project_modal_key(key)?;
            return Ok(());
        }

        match key.code {
            // Numeric tab shortcuts (only outside Workspace so they don't clash
            // with the workspace's own 1-4 sub-tab keys)
            KeyCode::Char('1') if self.active_screen != ActiveScreen::Workspace => {
                self.active_screen = ActiveScreen::Dashboard;
                self.active_tab_idx = 0;
            }
            KeyCode::Char('2') if self.active_screen != ActiveScreen::Workspace => {
                self.active_screen = ActiveScreen::Projects;
                self.active_tab_idx = 1;
            }
            KeyCode::Char('3') if self.active_screen != ActiveScreen::Workspace => {
                self.active_screen = ActiveScreen::Character;
                self.active_tab_idx = 2;
            }
            KeyCode::Char('4') if self.active_screen != ActiveScreen::Workspace => {
                self.active_screen = ActiveScreen::Library;
                self.active_tab_idx = 4;
            }
            KeyCode::Char('5') if self.active_screen != ActiveScreen::Workspace => {
                self.active_screen = ActiveScreen::Soundscapes;
                self.active_tab_idx = 7;
            }
            KeyCode::Char('6') if self.active_screen != ActiveScreen::Workspace => {
                self.active_screen = ActiveScreen::SyncSettings;
                self.active_tab_idx = 12;
            }
            KeyCode::Char('7') if self.active_screen != ActiveScreen::Workspace => {
                self.active_screen = ActiveScreen::Fellowship;
                self.active_tab_idx = 8;
                self.pull_invitations_async();
            }
            KeyCode::Char('8') if self.active_screen != ActiveScreen::Workspace => {
                self.active_screen = ActiveScreen::GreatChronicle;
                self.active_tab_idx = 14;
                self.great_chronicle_scroll = 0;
                self.chapter_panel_scroll = 0;
                self.chapter_panel_focused = true;
                self.great_chronicle_entries =
                    self.db.get_global_chronicle_entries().unwrap_or_default();
                self.load_chapter_progress_from_cache();
                self.pull_great_chronicle_async();
                self.pull_chapter_progress_async();
            }
            KeyCode::Char('D') => {
                self.active_screen = ActiveScreen::Dashboard;
                self.active_tab_idx = 0;
            }
            KeyCode::Char('d') => {
                if self.active_screen == ActiveScreen::Projects {
                    let active: Vec<&Project> =
                        self.projects.iter().filter(|p| !p.archived).collect();
                    if !active.is_empty() && self.selected_project_idx < active.len() {
                        let p = active[self.selected_project_idx];
                        self.modal_state = ModalType::ConfirmArchiveProject {
                            project_id: p.id,
                            project_name: p.name.clone(),
                        };
                    }
                } else if self.active_screen == ActiveScreen::Fellowship
                    && self.selected_fellowship_tab == 1
                {
                    let invites = self.db.get_invitations().unwrap_or_default();
                    if !invites.is_empty() && self.selected_invitation_idx < invites.len() {
                        let invite = &invites[self.selected_invitation_idx];
                        if invite.7 == "Pending" {
                            self.db.update_invitation_status(&invite.0, "Declined")?;
                            if self.config.sync_enabled {
                                let client = crate::services::api_client::ApiClient::new(
                                    &self.server_url,
                                    self.identity.clone(),
                                    &self.device_id,
                                );
                                let invite_id_clone = invite.0.clone();
                                let _ = std::thread::spawn(move || {
                                    let body = serde_json::json!({ "invite_id": invite_id_clone })
                                        .to_string();
                                    let _ = client.send_request("POST", "decline", &body);
                                });
                            }
                            self.notifications.push(Notification::info(format!("Declined invitation to '{}'", invite.2)));
                            self.reload_data()?;
                        }
                    }
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                if self.active_screen == ActiveScreen::GreatChronicle {
                    self.config.chronicle_share_level =
                        match self.config.chronicle_share_level.as_str() {
                            "none" => "everything".to_string(),
                            "everything" => "none".to_string(),
                            _ => "everything".to_string(),
                        };
                    let _ = self.config.save();
                } else if self.active_screen == ActiveScreen::Fellowship {
                    self.selected_fellowship_tab = 2;
                } else {
                    self.active_screen = ActiveScreen::Projects;
                    self.active_tab_idx = 1;
                }
            }
            KeyCode::Char('H') => {
                self.active_screen = ActiveScreen::Character;
                self.active_tab_idx = 2;
            }
            KeyCode::Char('F') if self.active_screen == ActiveScreen::Projects => {
                self.active_screen = ActiveScreen::Focus;
                self.active_tab_idx = 6;
            }
            KeyCode::Char('F') => {
                self.active_screen = ActiveScreen::Fellowship;
                self.active_tab_idx = 8;
                self.pull_invitations_async();
            }

            KeyCode::Char('S') if self.active_screen == ActiveScreen::Projects => {
                self.active_screen = ActiveScreen::Fellowship;
                self.active_tab_idx = 8;
                self.pull_invitations_async();
            }
            KeyCode::Char('A') if self.active_screen == ActiveScreen::Projects => {
                self.active_screen = ActiveScreen::Archive;
                self.active_tab_idx = 10;
            }
            KeyCode::Char('S') => {
                self.active_screen = ActiveScreen::SyncSettings;
                self.active_tab_idx = 12;
            }

            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.active_screen = ActiveScreen::Library;
                self.active_tab_idx = 4;
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                self.active_screen = ActiveScreen::GreatChronicle;
                self.active_tab_idx = 14;
                self.great_chronicle_scroll = 0;
                self.chapter_panel_scroll = 0;
                self.chapter_panel_focused = true;
                self.great_chronicle_entries =
                    self.db.get_global_chronicle_entries().unwrap_or_default();
                self.load_chapter_progress_from_cache();
                self.pull_great_chronicle_async();
                self.pull_chapter_progress_async();
            }
            KeyCode::Char('w') => {
                if self.active_screen == ActiveScreen::Dashboard {
                    self.water_tree()?;
                }
            }
            KeyCode::Char('e') | KeyCode::Char('E')
                if self.active_screen == ActiveScreen::Dashboard
                    || key.code == KeyCode::Char('E') =>
            {
                if self.active_screen == ActiveScreen::Dashboard {
                    // Cycle active ambient effect
                    self.active_ambient_effect = (self.active_ambient_effect + 1) % 6;
                    self.db.set_setting(
                        "active_ambient_effect",
                        &self.active_ambient_effect.to_string(),
                    )?;
                    self.trigger_ambient_particles();
                    self.notifications.push(Notification::info(format!(
                            "Ambient Effect: {}",
                            match self.active_ambient_effect {
                                0 => "Off",
                                1 => "Falling Leaves",
                                2 => "Stars",
                                3 => "Rain",
                                4 => "Snow",
                                5 => "Glowing Runes",
                                _ => "Off",
                            }
                        )));
                }
            }
            KeyCode::Left => {
                if self.active_screen == ActiveScreen::Dashboard {
                    self.dashboard_task_focus = false;
                } else if self.active_screen == ActiveScreen::Library && self.library_active_col > 0 {
                    self.library_active_col -= 1;
                    self.library_scroll_offset = 0;
                } else if self.active_screen == ActiveScreen::Character {
                    if self.character_focus > 0 {
                        self.character_focus -= 1;
                    }
                }
            }
            KeyCode::Right => {
                if self.active_screen == ActiveScreen::Dashboard {
                    self.dashboard_task_focus = true;
                } else if self.active_screen == ActiveScreen::Library && self.library_active_col < 2 {
                    self.library_active_col += 1;
                    self.library_scroll_offset = 0;
                } else if self.active_screen == ActiveScreen::Character {
                    let has_reflections = !self.db.get_reflections().unwrap_or_default().is_empty();
                    if has_reflections && self.character_focus < 2 {
                        self.character_focus += 1;
                    }
                }
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                if self.active_screen == ActiveScreen::Character {
                    let achievements = self.db.get_achievements()?;
                    let mut choices = vec!["Class Default".to_string()];
                    let has_forest = achievements
                        .iter()
                        .any(|a| a.id == "ancient_gardener" && a.unlocked_at.is_some());
                    let has_library = achievements.iter().any(|a| {
                        (a.id == "scholar" || a.id == "chronicler") && a.unlocked_at.is_some()
                    });
                    let has_fortress = achievements.iter().any(|a| {
                        (a.id == "hundred_day_journey" || a.id == "deep_worker")
                            && a.unlocked_at.is_some()
                    });
                    let has_workshop = achievements
                        .iter()
                        .any(|a| a.id == "master_concentration" && a.unlocked_at.is_some())
                        || self
                            .user
                            .as_ref()
                            .map(|u| u.specialization.is_some())
                            .unwrap_or(false);
                    let has_temple = achievements.iter().any(|a| {
                        (a.id == "alliance_builder" || a.id == "quest_together")
                            && a.unlocked_at.is_some()
                    });
                    if has_forest {
                        choices.push("Forest".to_string());
                    }
                    if has_library {
                        choices.push("AncientLibrary".to_string());
                    }
                    if has_fortress {
                        choices.push("MountainFortress".to_string());
                    }
                    if has_workshop {
                        choices.push("ArcaneWorkshop".to_string());
                    }
                    if has_temple {
                        choices.push("OceanTemple".to_string());
                    }
                    self.modal_state = ModalType::ThemeSelect {
                        choices,
                        selected_idx: 0,
                    };
                }
            }
            KeyCode::Char('s') => {
                if self.active_screen == ActiveScreen::Character {
                    if let Some(ref u) = self.user {
                        if u.level >= 10 && u.specialization.is_none() {
                            let specs = u.class.specializations();
                            let choices: Vec<String> =
                                specs.iter().map(|s| s.0.to_string()).collect();
                            self.modal_state = ModalType::SpecializationSelect {
                                choices,
                                selected_idx: 0,
                            };
                        }
                    }
                }
            }
            KeyCode::Tab if self.active_screen == ActiveScreen::GreatChronicle => {
                if self.chapter_panel_focused {
                    // Cycle chapter tab (Active / History) when right panel is focused
                    self.chapter_tab = (self.chapter_tab + 1) % 2;
                    self.chapter_panel_scroll = 0;
                } else {
                    // Switch focus to right chapter panel
                    self.chapter_panel_focused = true;
                }
            }
            KeyCode::Tab => {
                self.active_tab_idx = (self.active_tab_idx + 1) % 15;
                while self.active_tab_idx == 3
                    || self.active_tab_idx == 5
                    || self.active_tab_idx == 6
                    || self.active_tab_idx == 8
                    || self.active_tab_idx == 9
                    || self.active_tab_idx == 10
                    || self.active_tab_idx == 11
                    || self.active_tab_idx == 14
                {
                    self.active_tab_idx = (self.active_tab_idx + 1) % 15;
                }
                self.sync_screen_tab();
            }
            KeyCode::BackTab => {
                if self.active_tab_idx > 0 {
                    self.active_tab_idx -= 1;
                } else {
                    self.active_tab_idx = 14;
                }
                while self.active_tab_idx == 3
                    || self.active_tab_idx == 5
                    || self.active_tab_idx == 6
                    || self.active_tab_idx == 8
                    || self.active_tab_idx == 9
                    || self.active_tab_idx == 10
                    || self.active_tab_idx == 11
                    || self.active_tab_idx == 14
                {
                    if self.active_tab_idx > 0 {
                        self.active_tab_idx -= 1;
                    } else {
                        self.active_tab_idx = 14;
                    }
                }
                self.sync_screen_tab();
            }
            // Screen specific arrows and edits
            KeyCode::Up => {
                if self.active_screen == ActiveScreen::Character {
                    if self.character_focus == 0 {
                        let entries = self.db.get_chronicle_entries().unwrap_or_default();
                        if !entries.is_empty() {
                            if self.selected_chronicle_idx > 0 {
                                self.selected_chronicle_idx -= 1;
                            } else {
                                self.selected_chronicle_idx = entries.len() - 1;
                            }
                        }
                    } else if self.character_focus == 1 {
                        let reflections = self.db.get_reflections().unwrap_or_default();
                        if !reflections.is_empty() {
                            if self.selected_reflection_idx > 0 {
                                self.selected_reflection_idx -= 1;
                            } else {
                                self.selected_reflection_idx = reflections.len() - 1;
                            }
                            self.reflection_detail_scroll = 0;
                        }
                    } else if self.character_focus == 2 {
                        if self.reflection_detail_scroll > 0 {
                            self.reflection_detail_scroll -= 1;
                        }
                    }
                } else if self.active_screen == ActiveScreen::Projects {
                    let active_len = self.projects.iter().filter(|p| !p.archived).count();
                    if active_len > 0 {
                        if self.selected_project_idx > 0 {
                            self.selected_project_idx -= 1;
                        } else {
                            self.selected_project_idx = active_len - 1;
                        }
                    }
                } else if self.active_screen == ActiveScreen::Archive {
                    let archive_len = self.projects.iter().filter(|p| p.archived).count();
                    if archive_len > 0 {
                        if self.selected_archive_idx > 0 {
                            self.selected_archive_idx -= 1;
                        } else {
                            self.selected_archive_idx = archive_len - 1;
                        }
                    }
                } else if self.active_screen == ActiveScreen::Dashboard {
                    if self.dashboard_task_focus {
                        let tasks = self.db.get_tasks().unwrap_or_default();
                        let flat = dashboard_flat_items(&tasks);
                        if !flat.is_empty() {
                            self.selected_dashboard_task_idx = if self.selected_dashboard_task_idx > 0 {
                                self.selected_dashboard_task_idx - 1
                            } else {
                                flat.len() - 1
                            };
                        }
                    } else { match self.db.get_rituals() { Ok(rituals) => {
                        if !rituals.is_empty() {
                            self.selected_ritual_idx = if self.selected_ritual_idx > 0 {
                                self.selected_ritual_idx - 1
                            } else {
                                rituals.len() - 1
                            };
                        }
                    } _ => {}}}
                } else if self.active_screen == ActiveScreen::About {
                    self.about_scroll = self.about_scroll.saturating_sub(1);
                } else if self.active_screen == ActiveScreen::GreatChronicle {
                    if self.chapter_panel_focused {
                        self.chapter_panel_scroll = self.chapter_panel_scroll.saturating_sub(3);
                    } else {
                        self.great_chronicle_scroll = self.great_chronicle_scroll.saturating_sub(3);
                    }
                } else if self.active_screen == ActiveScreen::Soundscapes {
                    use crate::audio::SOUNDSCAPES;
                    if self.selected_soundscape_idx > 0 {
                        self.selected_soundscape_idx -= 1;
                    } else {
                        self.selected_soundscape_idx = SOUNDSCAPES.len() - 1;
                    }
                } else if self.active_screen == ActiveScreen::Fellowship {
                    if self.selected_fellowship_tab == 1 {
                        let invites = self.db.get_invitations().unwrap_or_default();
                        if !invites.is_empty() {
                            self.selected_invitation_idx = if self.selected_invitation_idx > 0 {
                                self.selected_invitation_idx - 1
                            } else {
                                invites.len() - 1
                            };
                        }
                    } else {
                        let shared_projects: Vec<_> =
                            self.projects.iter().filter(|p| p.is_shared).collect();
                        if !shared_projects.is_empty() {
                            let new_idx = if self.selected_fellowship_project_idx > 0 {
                                self.selected_fellowship_project_idx - 1
                            } else {
                                shared_projects.len() - 1
                            };
                            if new_idx != self.selected_fellowship_project_idx {
                                self.fellowship_selected_msg_idx = usize::MAX;
                                self.fellowship_chat_input.clear();
                                self.fellowship_composing = false;
                            }
                            self.selected_fellowship_project_idx = new_idx;
                        } else if self.selected_fellowship_tab == 0 {
                            let notifications = self.db.get_notifications().unwrap_or_default();
                            if !notifications.is_empty() {
                                self.selected_notification_idx = if self.selected_notification_idx > 0 {
                                    self.selected_notification_idx - 1
                                } else {
                                    notifications.len() - 1
                                };
                            }
                        }
                    }
                } else if self.active_screen == ActiveScreen::Library {
                    if self.library_active_col == 0 {
                        self.selected_library_cat_idx = if self.selected_library_cat_idx > 0 {
                            self.selected_library_cat_idx - 1
                        } else {
                            4
                        };
                        self.selected_library_item_idx = 0;
                        self.library_scroll_offset = 0;
                    } else if self.library_active_col == 1 {
                        let count = match self.selected_library_cat_idx {
                            0 => {
                                let class_name =
                                    self.user.as_ref().map(|u| u.class.name()).unwrap_or("");
                                self.db
                                    .get_class_quests(class_name)
                                    .map(|q| q.len())
                                    .unwrap_or(0)
                            }
                            1 => self
                                .db
                                .get_lore_entries()
                                .map(|e| e.iter().filter(|x| x.1 == "Class").count())
                                .unwrap_or(0),
                            2 => self
                                .db
                                .get_lore_entries()
                                .map(|e| e.iter().filter(|x| x.1 == "World").count())
                                .unwrap_or(0),
                            3 => self
                                .db
                                .get_lore_entries()
                                .map(|e| e.iter().filter(|x| x.1 == "Achievement").count())
                                .unwrap_or(0),
                            4 => self
                                .db
                                .get_lore_entries()
                                .map(|e| e.iter().filter(|x| x.1 == "Memory").count())
                                .unwrap_or(0),
                            _ => 0,
                        };
                        if count > 0 {
                            self.selected_library_item_idx = if self.selected_library_item_idx > 0 {
                                self.selected_library_item_idx - 1
                            } else {
                                count - 1
                            };
                        }
                        self.library_scroll_offset = 0;
                    } else if self.library_active_col == 2 && self.library_scroll_offset > 0 {
                        self.library_scroll_offset -= 1;
                    }
                } else if self.active_screen == ActiveScreen::Legends {
                    let relics = self.db.get_relics().unwrap_or_default();
                    if !relics.is_empty() {
                        if self.selected_relic_idx > 0 {
                            self.selected_relic_idx -= 1;
                        } else {
                            self.selected_relic_idx = relics.len() - 1;
                        }
                    }
                }
            }
            KeyCode::Down => {
                if self.active_screen == ActiveScreen::Character {
                    if self.character_focus == 0 {
                        let entries = self.db.get_chronicle_entries().unwrap_or_default();
                        if !entries.is_empty() {
                            self.selected_chronicle_idx =
                                (self.selected_chronicle_idx + 1) % entries.len();
                        }
                    } else if self.character_focus == 1 {
                        let reflections = self.db.get_reflections().unwrap_or_default();
                        if !reflections.is_empty() {
                            self.selected_reflection_idx =
                                (self.selected_reflection_idx + 1) % reflections.len();
                            self.reflection_detail_scroll = 0;
                        }
                    } else if self.character_focus == 2 {
                        self.reflection_detail_scroll += 1;
                    }
                } else if self.active_screen == ActiveScreen::Projects {
                    let active_len = self.projects.iter().filter(|p| !p.archived).count();
                    if active_len > 0 {
                        if self.selected_project_idx < active_len - 1 {
                            self.selected_project_idx += 1;
                        } else {
                            self.selected_project_idx = 0;
                        }
                    }
                } else if self.active_screen == ActiveScreen::Archive {
                    let archive_len = self.projects.iter().filter(|p| p.archived).count();
                    if archive_len > 0 {
                        if self.selected_archive_idx < archive_len - 1 {
                            self.selected_archive_idx += 1;
                        } else {
                            self.selected_archive_idx = 0;
                        }
                    }
                } else if self.active_screen == ActiveScreen::Dashboard {
                    if self.dashboard_task_focus {
                        let tasks = self.db.get_tasks().unwrap_or_default();
                        let flat = dashboard_flat_items(&tasks);
                        if !flat.is_empty() {
                            self.selected_dashboard_task_idx =
                                (self.selected_dashboard_task_idx + 1) % flat.len();
                        }
                    } else { match self.db.get_rituals() { Ok(rituals) => {
                        if !rituals.is_empty() {
                            self.selected_ritual_idx =
                                (self.selected_ritual_idx + 1) % rituals.len();
                        }
                    } _ => {}}}
                } else if self.active_screen == ActiveScreen::About {
                    let content = self.about_content_lines.get();
                    let visible = self.terminal_height.saturating_sub(5);
                    let max_scroll = content.saturating_sub(visible);
                    self.about_scroll = self.about_scroll.saturating_add(2).min(max_scroll);
                } else if self.active_screen == ActiveScreen::GreatChronicle {
                    if self.chapter_panel_focused {
                        self.chapter_panel_scroll = self.chapter_panel_scroll.saturating_add(3);
                    } else {
                        self.great_chronicle_scroll = self.great_chronicle_scroll.saturating_add(3);
                    }
                } else if self.active_screen == ActiveScreen::Soundscapes {
                    use crate::audio::SOUNDSCAPES;
                    self.selected_soundscape_idx =
                        (self.selected_soundscape_idx + 1) % SOUNDSCAPES.len();
                } else if self.active_screen == ActiveScreen::Fellowship {
                    if self.selected_fellowship_tab == 1 {
                        let invites = self.db.get_invitations().unwrap_or_default();
                        if !invites.is_empty() {
                            self.selected_invitation_idx =
                                (self.selected_invitation_idx + 1) % invites.len();
                        }
                    } else {
                        let shared_projects: Vec<_> =
                            self.projects.iter().filter(|p| p.is_shared).collect();
                        if !shared_projects.is_empty() {
                            let new_idx = (self.selected_fellowship_project_idx + 1) % shared_projects.len();
                            if new_idx != self.selected_fellowship_project_idx {
                                self.fellowship_selected_msg_idx = usize::MAX;
                                self.fellowship_chat_input.clear();
                                self.fellowship_composing = false;
                            }
                            self.selected_fellowship_project_idx = new_idx;
                        } else if self.selected_fellowship_tab == 0 {
                            let notifications = self.db.get_notifications().unwrap_or_default();
                            if !notifications.is_empty() {
                                self.selected_notification_idx =
                                    (self.selected_notification_idx + 1) % notifications.len();
                            }
                        }
                    }
                } else if self.active_screen == ActiveScreen::Library {
                    if self.library_active_col == 0 {
                        self.selected_library_cat_idx = (self.selected_library_cat_idx + 1) % 5;
                        self.selected_library_item_idx = 0;
                        self.library_scroll_offset = 0;
                    } else if self.library_active_col == 1 {
                        let count = match self.selected_library_cat_idx {
                            0 => {
                                let class_name =
                                    self.user.as_ref().map(|u| u.class.name()).unwrap_or("");
                                self.db
                                    .get_class_quests(class_name)
                                    .map(|q| q.len())
                                    .unwrap_or(0)
                            }
                            1 => self
                                .db
                                .get_lore_entries()
                                .map(|e| e.iter().filter(|x| x.1 == "Class").count())
                                .unwrap_or(0),
                            2 => self
                                .db
                                .get_lore_entries()
                                .map(|e| e.iter().filter(|x| x.1 == "World").count())
                                .unwrap_or(0),
                            3 => self
                                .db
                                .get_lore_entries()
                                .map(|e| e.iter().filter(|x| x.1 == "Achievement").count())
                                .unwrap_or(0),
                            4 => self
                                .db
                                .get_lore_entries()
                                .map(|e| e.iter().filter(|x| x.1 == "Memory").count())
                                .unwrap_or(0),
                            _ => 0,
                        };
                        if count > 0 {
                            self.selected_library_item_idx =
                                (self.selected_library_item_idx + 1) % count;
                        }
                        self.library_scroll_offset = 0;
                    } else if self.library_active_col == 2 {
                        self.library_scroll_offset = self.library_scroll_offset.saturating_add(1);
                    }
                } else if self.active_screen == ActiveScreen::Legends {
                    let relics = self.db.get_relics().unwrap_or_default();
                    if !relics.is_empty() {
                        self.selected_relic_idx = (self.selected_relic_idx + 1) % relics.len();
                    }
                }
            }
            KeyCode::Enter => {
                if self.active_screen == ActiveScreen::Projects {
                    let active: Vec<&Project> =
                        self.projects.iter().filter(|p| !p.archived).collect();
                    if !active.is_empty() && self.selected_project_idx < active.len() {
                        self.active_project_id = Some(active[self.selected_project_idx].id);
                        self.active_screen = ActiveScreen::Workspace;
                        self.workspace_tab_idx = 0;
                        self.workspace_sidebar_focused = true;
                        self.selected_task_idx = 0;
                        self.selected_note_idx = 0;
                        self.selected_notes_flat_idx = 0;
                        self.selected_journal_idx = 0;
                        self.reload_data()?;
                    }
                } else if self.active_screen == ActiveScreen::Dashboard {
                    if self.dashboard_task_focus {
                        // Navigate to the selected dashboard task (or its parent if a step) in its project workspace
                        let tasks = self.db.get_tasks().unwrap_or_default();
                        let flat = dashboard_flat_items(&tasks);
                        let sel = self.selected_dashboard_task_idx.min(flat.len().saturating_sub(1));
                        if let Some((is_step, parent_id, task)) = flat.get(sel).cloned() {
                            // If on a step, navigate to the parent task instead
                            let nav_task_id = if is_step { parent_id } else { task.id };
                            let nav_project_id = if is_step {
                                tasks.iter().find(|t| t.id == parent_id).and_then(|t| t.project_id)
                            } else {
                                task.project_id
                            };
                            if let Some(p_id) = nav_project_id {
                                self.active_project_id = Some(p_id);
                                self.active_screen = ActiveScreen::Workspace;
                                self.workspace_tab_idx = 0;
                                self.workspace_sidebar_focused = false;
                                self.task_filter = "All".to_string();
                                let proj_tasks = self.db.get_tasks().unwrap_or_default();
                                let mut proj_tasks_sorted: Vec<&Task> = proj_tasks.iter()
                                    .filter(|t| t.project_id == Some(p_id) && t.parent_task_id.is_none())
                                    .collect();
                                proj_tasks_sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
                                self.selected_task_idx = proj_tasks_sorted.iter().position(|t| t.id == nav_task_id).unwrap_or(0);
                                self.dashboard_task_focus = false;
                                self.reload_data()?;
                            }
                        }
                    } else { match self.db.get_rituals() { Ok(rituals) => {
                        if !rituals.is_empty() && self.selected_ritual_idx < rituals.len() {
                            let r_id = rituals[self.selected_ritual_idx].id.clone();
                            self.complete_ritual(&r_id)?;
                        }
                    } _ => {}}}
                } else if self.active_screen == ActiveScreen::Soundscapes {
                    use crate::audio::SOUNDSCAPES;
                    let s_name = SOUNDSCAPES[self.selected_soundscape_idx].name;
                    if s_name == "Local Folder" {
                        let folder = self.db.get_setting("local_music_folder").unwrap_or_default().unwrap_or_default();
                        if folder.trim().is_empty() {
                            self.modal_state = ModalType::LocalMusicFolder { input: String::new(), suggestions: vec![], selected: 0 };
                            return Ok(());
                        }
                    }
                    let _ = self.db.set_setting("last_music_source", s_name);
                    self.audio_player.play(s_name);
                } else if self.active_screen == ActiveScreen::Fellowship {
                    if self.selected_fellowship_tab == 1 {
                        // Accept invitation
                        let invites = self.db.get_invitations().unwrap_or_default();
                        if !invites.is_empty() && self.selected_invitation_idx < invites.len() {
                            let invite = &invites[self.selected_invitation_idx];
                            if invite.7 == "Pending" {
                                self.db.update_invitation_status(&invite.0, "Accepted")?;
                                if self.config.sync_enabled {
                                    let client = crate::services::api_client::ApiClient::new(
                                        &self.server_url,
                                        self.identity.clone(),
                                        &self.device_id,
                                    );
                                    let invite_id_clone = invite.0.clone();
                                    let my_username = self.user.as_ref().map(|u| u.username.clone()).unwrap_or_default();
                                    let _ = std::thread::spawn(move || {
                                        let body = serde_json::json!({
                                            "invite_id": invite_id_clone,
                                            "username": my_username
                                        }).to_string();
                                        let _ = client.send_request("POST", "accept", &body);
                                    });
                                }
                                // Insert the project locally

                                if let Ok(proj_uuid) = Uuid::parse_str(&invite.1) {
                                    let new_proj = Project {
                                        id: proj_uuid,
                                        name: invite.2.clone(),
                                        description: Some("Fellowship shared project".to_string()),
                                        created_at: Utc::now(),
                                        archived: false,
                                        completed: false,
                                        owner_identity: Some(invite.3.clone()),
                                        owner_username: Some(invite.4.clone()),
                                        is_shared: true,
                                    };
                                    let _ = self.db.insert_project(&new_proj);

                                    // Add inviter as a member too
                                    let _ = self.db.add_project_member(
                                        &invite.1, &invite.3, &invite.4, "Owner",
                                    );
                                    // Add myself as member
                                    let _ = self.db.add_project_member(
                                        &invite.1,
                                        &self.identity.public_key,
                                        &self.user.as_ref().unwrap().username,
                                        &invite.6,
                                    );
                                }

                                // Unlock "First Companion" achievement
                                let _ = self.db.conn.execute("UPDATE achievements SET unlocked_at = ?1 WHERE id = 'first_companion' AND unlocked_at IS NULL", params![Utc::now().to_rfc3339()]);

                                self.notifications.push(Notification::info(format!("Accepted invitation to '{}'", invite.2)));

                                // Check "Alliance Builder" achievement progress
                                let shared_projs_count =
                                    self.projects.iter().filter(|p| p.is_shared).count() + 1;
                                if shared_projs_count >= 25 {
                                    let _ = self.db.conn.execute("UPDATE achievements SET unlocked_at = ?1 WHERE id = 'alliance_builder' AND unlocked_at IS NULL", params![Utc::now().to_rfc3339()]);
                                }

                                self.mark_dirty();
                                self.reload_data()?;
                            }
                        }
                    }
                } else if self.active_screen == ActiveScreen::Fellowship
                    && self.selected_fellowship_tab == 0
                    && self.projects.iter().filter(|p| p.is_shared).count() == 0
                {
                    let notifications = self.db.get_notifications().unwrap_or_default();
                    if !notifications.is_empty()
                        && self.selected_notification_idx < notifications.len()
                    {
                        let notif = &notifications[self.selected_notification_idx];
                        self.db.mark_notification_read(&notif.0)?;
                        self.reload_data()?;
                    }
                }
            }
            KeyCode::Char(' ') => {
                if self.active_screen == ActiveScreen::Dashboard {
                    if self.dashboard_task_focus {
                        let all_tasks = self.db.get_tasks().unwrap_or_default();
                        let flat = dashboard_flat_items(&all_tasks);
                        let sel = self.selected_dashboard_task_idx.min(flat.len().saturating_sub(1));
                        if let Some((is_step, _parent_id, task)) = flat.get(sel).cloned() {
                            if is_step {
                                if task.completed {
                                    // Reabrir paso — XP ya cobrado
                                    let mut t = task;
                                    t.completed = false;
                                    self.db.update_task(&t)?;
                                    self.mark_dirty();
                                    self.notifications.push(Notification::info("Trial reopened. XP already claimed — face it again with fresh resolve.".to_string()));
                                    self.reload_data()?;
                                } else {
                                    let already_awarded = task.xp_awarded;
                                    let mut t = task;
                                    t.completed = true;
                                    t.xp_awarded = true;
                                    self.db.update_task(&t)?;
                                    self.mark_dirty();
                                    self.audio_player.play_task_complete();
                                    if !already_awarded {
                                        let is_high = t.priority == TaskPriority::High;
                                        let xp = if is_high { 50 } else { 25 };
                                        let label = if is_high { "Resolve Hero Step Quest" } else { "Complete Step Quest" };
                                        self.grant_xp(label, xp)?;
                                        self.apply_class_passive("task_complete", 0)?;
                                    }
                                    self.trigger_ambient_particles();
                                    self.complete_productive_action()?;
                                    self.grow_tree(2)?;
                                    self.update_daily_adventure_progress("complete_tasks", 1)?;
                                    self.check_action_achievements()?;
                                    let new_flat = dashboard_flat_items(&self.db.get_tasks().unwrap_or_default());
                                    if self.selected_dashboard_task_idx >= new_flat.len() && !new_flat.is_empty() {
                                        self.selected_dashboard_task_idx = new_flat.len() - 1;
                                    }
                                    self.reload_data()?;
                                }
                            } else {
                                if task.completed {
                                    // Reabrir quest — los pasos también se reabren
                                    let steps: Vec<Task> = all_tasks.iter()
                                        .filter(|t| t.parent_task_id == Some(task.id))
                                        .cloned()
                                        .collect();
                                    for step in &steps {
                                        if step.completed {
                                            let mut s = step.clone();
                                            s.completed = false;
                                            self.db.update_task(&s)?;
                                        }
                                    }
                                    let mut t = task;
                                    t.completed = false;
                                    self.db.update_task(&t)?;
                                    self.mark_dirty();
                                    self.notifications.push(Notification::info("Quest unsealed. XP already claimed — the path forward is yours to walk again.".to_string()));
                                    let new_flat = dashboard_flat_items(&self.db.get_tasks().unwrap_or_default());
                                    if self.selected_dashboard_task_idx >= new_flat.len() && !new_flat.is_empty() {
                                        self.selected_dashboard_task_idx = new_flat.len() - 1;
                                    }
                                    self.reload_data()?;
                                } else {
                                    let incomplete_steps: Vec<Task> = all_tasks.iter()
                                        .filter(|t| t.parent_task_id == Some(task.id) && !t.completed)
                                        .cloned()
                                        .collect();
                                    if !incomplete_steps.is_empty() {
                                        // Bloquear hasta que todos los pasos estén cerrados
                                        let n = incomplete_steps.len();
                                        let msg = if n == 1 {
                                            "One trial remains unsealed. Face it before this quest can be closed.".to_string()
                                        } else {
                                            format!("{} trials remain unsealed. Resolve them before this quest can be claimed.", n)
                                        };
                                        self.notifications.push(Notification::warning(msg));
                                    } else {
                                        let already_awarded = task.xp_awarded;
                                        let total_steps = all_tasks.iter()
                                            .filter(|t| t.parent_task_id == Some(task.id))
                                            .count();
                                        let mut t = task;
                                        t.completed = true;
                                        t.xp_awarded = true;
                                        self.db.update_task(&t)?;
                                        self.mark_dirty();
                                        self.audio_player.play_task_complete();
                                        if !already_awarded {
                                            let is_high = t.priority == TaskPriority::High;
                                            let val = if is_high { 50 } else { 25 };
                                            let title = if is_high { "Resolve Hero Task Quest" } else { "Complete Task Quest" };
                                            self.grant_xp(title, val)?;
                                            let passive_trigger = if is_high { "high_priority_task" } else { "task_complete" };
                                            self.apply_class_passive(passive_trigger, 0)?;
                                        }
                                        self.trigger_ambient_particles();
                                        self.increment_quest_progress(10, 1)?;
                                        let frag_trigger = if t.priority == TaskPriority::High { "high_priority_task" } else { "task" };
                                        self.simulate_memory_fragment_unlock(frag_trigger)?;
                                        self.complete_productive_action()?;
                                        let growth = if t.priority == TaskPriority::High { 4 } else { 2 };
                                        self.grow_tree(growth)?;
                                        self.update_daily_adventure_progress("complete_tasks", 1)?;
                                        if t.priority == TaskPriority::High {
                                            self.update_daily_adventure_progress("complete_high_priority_task", 1)?;
                                        }
                                        self.check_action_achievements()?;
                                        let chronicle_desc = if total_steps > 0 {
                                            format!("completed 1 quest with {} step{}.", total_steps, if total_steps == 1 { "" } else { "s" })
                                        } else {
                                            "completed 1 quest.".to_string()
                                        };
                                        self.push_great_chronicle_async("QuestComplete", &chronicle_desc, true);
                                        self.maybe_spawn_task_completion_sprite();
                                        // Tarea recurrente — genera la siguiente ocurrencia automáticamente
                                        if let Some(recurrence) = t.recurrence {
                                            let next_due = Self::advance_recurrence_date(t.due_date, recurrence);
                                            let next_task = Task {
                                                id: Uuid::new_v4(),
                                                project_id: t.project_id,
                                                title: t.title.clone(),
                                                description: t.description.clone(),
                                                due_date: Some(next_due),
                                                completed: false,
                                                priority: t.priority,
                                                created_at: Utc::now(),
                                                updated_at: Utc::now(),
                                                owner_identity: t.owner_identity.clone(),
                                                owner_username: t.owner_username.clone(),
                                                parent_task_id: None,
                                                xp_awarded: false,
                                                recurrence: Some(recurrence),
                                            };
                                            let _ = self.db.insert_task(&next_task);
                                        }
                                        let new_flat = dashboard_flat_items(&self.db.get_tasks().unwrap_or_default());
                                        if self.selected_dashboard_task_idx >= new_flat.len() && !new_flat.is_empty() {
                                            self.selected_dashboard_task_idx = new_flat.len() - 1;
                                        }
                                        self.reload_data()?;
                                    }
                                }
                            }
                        }
                    } else { match self.db.get_rituals() { Ok(rituals) => {
                        if !rituals.is_empty() && self.selected_ritual_idx < rituals.len() {
                            let r_id = rituals[self.selected_ritual_idx].id.clone();
                            self.complete_ritual(&r_id)?;
                        }
                    } _ => {}}}
                } else if self.active_screen == ActiveScreen::Library {
                    self.handle_library_action()?;
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X')
                if self.active_screen == ActiveScreen::GreatChronicle =>
            {
                // Re-open the prologue (Story So Far → Chapter One) from Tab 8.
                // Reflect the current saved "don't show again" state so the checkbox
                // appears as the user last left it.
                self.prologue_skip_checked = self.db.get_setting("prologue_skip")
                    .ok().flatten()
                    .map(|v| v == "1")
                    .unwrap_or(false);
                self.prologue_page = 0;
                self.prologue_line_idx = 0;
                self.prologue_char_in_line = 0;
                self.prologue_delay_ticks = 20;
                self.prologue_next_is_onboarding = false;
                self.audio_player.play_cinematic();
                self.active_screen = ActiveScreen::Prologue;
            }
            KeyCode::Char('r') | KeyCode::Char('R')
                if self.active_screen == ActiveScreen::GreatChronicle =>
            {
                self.great_chronicle_scroll = 0;
                self.chapter_panel_scroll = 0;
                // Synchronous pull: fetch from server and store before reloading the list
                if self.config.sync_enabled {
                    let client = crate::services::api_client::ApiClient::new(
                        &self.server_url,
                        self.identity.clone(),
                        &self.device_id,
                    );
                    if let Ok(resp) = client.send_request("GET", "global_chronicle", "") {
                        if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&resp) {
                            if let Some(entries) = arr.as_array() {
                                for e in entries {
                                    let entry = crate::models::GlobalChronicleEntry {
                                        id: e["id"].as_str().unwrap_or_default().to_string(),
                                        hero_name: e["hero_name"].as_str().unwrap_or_default().to_string(),
                                        event_type: e["event_type"].as_str().unwrap_or_default().to_string(),
                                        description: e["description"].as_str().unwrap_or_default().to_string(),
                                        timestamp: e["timestamp"].as_str().unwrap_or_default().to_string(),
                                    };
                                    if !entry.id.is_empty() {
                                        let _ = self.db.upsert_chronicle_entry(&entry);
                                    }
                                }
                            }
                        }
                    }
                }
                self.great_chronicle_entries =
                    self.db.get_global_chronicle_entries().unwrap_or_default();
                self.pull_chapter_progress_async();
                self.notifications.push(Notification::info(format!("Realm Activity refreshed — {} entries.", self.great_chronicle_entries.len())));
            }
            KeyCode::Char('r') => {
                if self.active_screen == ActiveScreen::Fellowship
                    && self.selected_fellowship_tab == 2
                    && self.config.sync_enabled
                {
                    let client = crate::services::api_client::ApiClient::new(
                        &self.server_url,
                        self.identity.clone(),
                        &self.device_id,
                    );
                    let _ = self.refresh_companions(&client);
                    self.notifications.push(Notification::info("Companion presence refreshed.".to_string()));
                } else if self.active_screen == ActiveScreen::Dashboard {
                    // Guard: only allow one reflection per day
                    let today = chrono::Local::now().date_naive();
                    let already_reflected = self
                        .db
                        .get_reflection_for_date(today)
                        .unwrap_or(None)
                        .is_some();
                    if already_reflected {
                        self.notifications.push(Notification::warning("You have already reflected today. Come back tomorrow."));
                    } else {
                        self.modal_state = ModalType::DailyReflection {
                            what_went_well: String::new(),
                            what_can_improve: String::new(),
                            focus_idx: 0,
                        };
                    }
                } else if self.active_screen == ActiveScreen::Archive {
                    let archived: Vec<&Project> =
                        self.projects.iter().filter(|p| p.archived).collect();
                    if !archived.is_empty() && self.selected_archive_idx < archived.len() {
                        let mut p = archived[self.selected_archive_idx].clone();
                        p.archived = false;
                        self.db.update_project(&p)?;
                        self.mark_dirty();
                        self.apply_class_passive("project_restore", 0)?;
                        self.selected_archive_idx = 0;
                        self.reload_data()?;
                    }
                } else if self.active_screen == ActiveScreen::Fellowship
                    && self.selected_fellowship_tab == 0
                {
                    self.modal_state = ModalType::AddReaction {
                        message_id: Uuid::nil(),
                    };
                } else if self.active_screen == ActiveScreen::About {
                    self.bug_report_modal = Some(BugReportModal {
                        report_type: ReportType::Bug,
                        description: String::new(),
                        status: None,
                    });
                }
            }
            KeyCode::Char('n') => {
                if self.active_screen == ActiveScreen::Projects {
                    self.modal_state = ModalType::NewProject {
                        name: String::new(),
                        desc: String::new(),
                        focus_idx: 0,
                    };
                } else if self.active_screen == ActiveScreen::Dashboard {
                    self.modal_state = ModalType::NewRitual {
                        name: String::new(),
                        desc: String::new(),
                        frequency_idx: 0,
                        reward_xp: "20".to_string(),
                        focus_idx: 0,
                    };
                }
            }
            KeyCode::Char('e') => {
                if self.active_screen == ActiveScreen::Projects {
                    let active: Vec<&Project> =
                        self.projects.iter().filter(|p| !p.archived).collect();
                    if !active.is_empty() && self.selected_project_idx < active.len() {
                        let p = active[self.selected_project_idx];
                        self.modal_state = ModalType::EditProject {
                            id: p.id,
                            name: p.name.clone(),
                            desc: p.description.clone().unwrap_or_default(),
                            focus_idx: 0,
                        };
                    }
                }
            }
            KeyCode::Char('c') => {
                if self.active_screen == ActiveScreen::Fellowship {
                    self.selected_fellowship_tab = 0;
                    self.fellowship_focus_left = false;
                    self.fellowship_composing = false;
                }
            }
            KeyCode::Char('i') => {
                if self.active_screen == ActiveScreen::Fellowship {
                    self.selected_fellowship_tab = 1;
                }
            }
            KeyCode::Char('a') => {
                if self.active_screen == ActiveScreen::Fellowship {
                    if self.selected_fellowship_tab == 0 && self.projects.iter().filter(|p| p.is_shared).count() == 0 {
                        self.db.mark_all_notifications_read()?;
                        self.reload_data()?;
                    } else {
                        self.selected_fellowship_tab = 3;
                    }
                }
            }
            KeyCode::Char('/') => {
                if self.active_screen == ActiveScreen::Fellowship {
                    self.selected_fellowship_tab = 4;
                    self.modal_state = ModalType::SearchMessages {
                        query: String::new(),
                    };
                }
            }
            KeyCode::Char('m') => {
                // Fellowship tab 0 'm' is handled by handle_fellowship_chat_key (inline input).
                // Nothing to do here for that case.
            }
            KeyCode::Char('v') => {
                if self.active_screen == ActiveScreen::Fellowship
                    && self.selected_fellowship_tab == 0
                {
                    let active_projects: Vec<_> = self.projects.iter().filter(|p| !p.archived).collect();
                    let shared_projects: Vec<_> = self.projects.iter().filter(|p| p.is_shared).collect();

                    let mut default_proj_idx = 0;
                    if !shared_projects.is_empty() && self.selected_fellowship_project_idx < shared_projects.len() {
                        let selected_shared_id = shared_projects[self.selected_fellowship_project_idx].id;
                        if let Some(pos) = active_projects.iter().position(|p| p.id == selected_shared_id) {
                            default_proj_idx = pos;
                        }
                    }

                    self.modal_state = ModalType::InviteMember {
                        identity: String::new(),
                        username: String::new(),
                        role_idx: 0,
                        project_idx: default_proj_idx,
                        focus_idx: 0,
                    };
                }
            }
            KeyCode::Char('j') => {
                if self.active_screen == ActiveScreen::Fellowship {
                    let shared_projects: Vec<_> =
                        self.projects.iter().filter(|p| p.is_shared).collect();
                    if !shared_projects.is_empty()
                        && self.selected_fellowship_project_idx < shared_projects.len()
                    {
                        let p = shared_projects[self.selected_fellowship_project_idx];
                        self.modal_state = ModalType::ProjectSharing { project_id: p.id };
                    }
                } else if self.active_screen == ActiveScreen::Projects {
                    let active: Vec<&Project> =
                        self.projects.iter().filter(|p| !p.archived).collect();
                    if !active.is_empty() && self.selected_project_idx < active.len() {
                        let p = active[self.selected_project_idx];
                        self.modal_state = ModalType::ProjectSharing { project_id: p.id };
                    }
                }
            }

            KeyCode::Delete => {
                if self.active_screen == ActiveScreen::Archive {
                    // Permanently Slay Project — show confirmation first
                    let archived: Vec<&Project> =
                        self.projects.iter().filter(|p| p.archived).collect();
                    if !archived.is_empty() && self.selected_archive_idx < archived.len() {
                        let p = archived[self.selected_archive_idx];
                        self.modal_state = ModalType::ConfirmDeleteProject {
                            project_id: p.id,
                            project_name: p.name.clone(),
                        };
                    }
                } else if self.active_screen == ActiveScreen::Dashboard {
                    // Delete selected ritual
                    if let Ok(rituals) = self.db.get_rituals() {
                        if rituals.len() <= 1 {
                            self.notifications.push(Notification::warning("You must maintain at least one active sidequest!"));
                        } else if self.selected_ritual_idx < rituals.len() {
                            let ritual_name = rituals[self.selected_ritual_idx].name.clone();
                            let r_id = rituals[self.selected_ritual_idx].id.clone();
                            self.db.delete_ritual(&r_id)?;
                            self.mark_dirty();
                            self.selected_ritual_idx = self.selected_ritual_idx.saturating_sub(1);
                            self.notifications.push(Notification::info(format!("Sidequest '{}' removed.", ritual_name)));
                            self.reload_data()?;
                        }
                    }
                }
            }
            KeyCode::Esc => {
                // Exit to Dashboard
                self.active_screen = ActiveScreen::Dashboard;
                self.active_tab_idx = 0;
            }
            _ => {}
        }
        Ok(())
    }

    // Handles Project edit/new modals.
    fn handle_project_modal_key(&mut self, key: KeyEvent) -> Result<()> {
        let (mut name, mut desc, mut focus_idx, is_edit, p_id) = match self.modal_state {
            ModalType::NewProject {
                ref name,
                ref desc,
                focus_idx,
            } => (name.clone(), desc.clone(), focus_idx, false, None),
            ModalType::EditProject {
                id,
                ref name,
                ref desc,
                focus_idx,
            } => (name.clone(), desc.clone(), focus_idx, true, Some(id)),
            _ => return Ok(()),
        };

        match key.code {
            KeyCode::Esc => {
                self.modal_state = ModalType::None;
            }
            KeyCode::Tab => {
                focus_idx = (focus_idx + 1) % 2;
                self.modal_state = if is_edit {
                    ModalType::EditProject {
                        id: p_id.unwrap(),
                        name,
                        desc,
                        focus_idx,
                    }
                } else {
                    ModalType::NewProject {
                        name,
                        desc,
                        focus_idx,
                    }
                };
            }
            KeyCode::BackTab => {
                focus_idx = if focus_idx > 0 { 0 } else { 1 };
                self.modal_state = if is_edit {
                    ModalType::EditProject {
                        id: p_id.unwrap(),
                        name,
                        desc,
                        focus_idx,
                    }
                } else {
                    ModalType::NewProject {
                        name,
                        desc,
                        focus_idx,
                    }
                };
            }
            KeyCode::Char(c) => {
                if focus_idx == 0 {
                    if name.len() < 30 {
                        name.push(c);
                    }
                } else {
                    if desc.len() < 100 {
                        desc.push(c);
                    }
                }
                self.modal_state = if is_edit {
                    ModalType::EditProject {
                        id: p_id.unwrap(),
                        name,
                        desc,
                        focus_idx,
                    }
                } else {
                    ModalType::NewProject {
                        name,
                        desc,
                        focus_idx,
                    }
                };
            }
            KeyCode::Backspace => {
                if focus_idx == 0 {
                    name.pop();
                } else {
                    desc.pop();
                }
                self.modal_state = if is_edit {
                    ModalType::EditProject {
                        id: p_id.unwrap(),
                        name,
                        desc,
                        focus_idx,
                    }
                } else {
                    ModalType::NewProject {
                        name,
                        desc,
                        focus_idx,
                    }
                };
            }
            KeyCode::Enter if !name.trim().is_empty() => {
                let project_desc = if desc.trim().is_empty() {
                    None
                } else {
                    Some(desc.trim().to_string())
                };
                if let Some(id) = p_id {
                    if let Some(mut existing) = self.projects.iter().find(|x| x.id == id).cloned() {
                        existing.name = name.trim().to_string();
                        existing.description = project_desc;
                        self.db.update_project(&existing)?;
                        self.mark_dirty();
                    }
                } else {
                    let p = Project {
                        id: Uuid::new_v4(),
                        name: name.trim().to_string(),
                        description: project_desc,
                        created_at: Utc::now(),
                        archived: false,
                        completed: false,
                        owner_identity: Some(self.identity.public_key.clone()),
                        owner_username: Some(
                            self.user
                                .as_ref()
                                .map(|u| u.username.clone())
                                .unwrap_or_else(|| "Gibranlp".to_string()),
                        ),
                        is_shared: false,
                    };
                    self.db.insert_project(&p)?;
                    self.mark_dirty();
                    self.apply_class_passive("project_create", 0)?;
                }
                self.reload_data()?;
                self.modal_state = ModalType::None;
            }
            _ => {}
        }
        Ok(())
    }

    // Builds a flat navigation list for the notes tab.
    // Each item: (codex_id Option<Uuid>, note_index Option<usize>)
    // (Some, None) = codex header  |  (_, Some) = note  |  (None, None) = divider
    fn build_notes_flat(notes: &[Note], codices: &[crate::models::Codex], project_id: Uuid) -> Vec<(Option<Uuid>, Option<usize>)> {
        let mut flat: Vec<(Option<Uuid>, Option<usize>)> = Vec::new();
        let proj_notes: Vec<(usize, &Note)> = notes.iter().enumerate()
            .filter(|(_, n)| n.project_id == Some(project_id))
            .collect();

        // codices already arrive sorted by name (DB ORDER BY LOWER(name))
        for codex in codices {
            flat.push((Some(codex.id), None));
            let mut codex_notes: Vec<(usize, &Note)> = proj_notes.iter()
                .filter(|(_, n)| n.codex_id == Some(codex.id))
                .map(|(i, n)| (*i, *n))
                .collect();
            codex_notes.sort_by(|(_, a), (_, b)| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
            for (idx, _) in codex_notes {
                flat.push((Some(codex.id), Some(idx)));
            }
        }

        let grouped_note_indices: std::collections::HashSet<usize> = proj_notes.iter()
            .filter(|(_, n)| n.codex_id.is_some() && codices.iter().any(|c| Some(c.id) == n.codex_id))
            .map(|(i, _)| *i)
            .collect();
        let mut ungrouped: Vec<(usize, &Note)> = proj_notes.iter()
            .filter(|(i, _)| !grouped_note_indices.contains(i))
            .map(|(i, n)| (*i, *n))
            .collect();
        ungrouped.sort_by(|(_, a), (_, b)| a.title.to_lowercase().cmp(&b.title.to_lowercase()));

        if !codices.is_empty() && !ungrouped.is_empty() {
            flat.push((None, None)); // divider
        }
        for (idx, _) in ungrouped {
            flat.push((None, Some(idx)));
        }
        flat
    }

    // Handles active Project Workspace key events.
    fn handle_workspace_key(&mut self, key: KeyEvent) -> Result<()> {
        let p_id = if let Some(id) = self.active_project_id {
            id
        } else {
            self.active_screen = ActiveScreen::Projects;
            return Ok(());
        };

        // Si el codex de atajos está abierto, solo ESC o ? lo cierra — nada más pasa
        if self.workspace_help_open {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('?')) {
                self.workspace_help_open = false;
            }
            return Ok(());
        }

        // If local search is active
        if self.searching {
            match key.code {
                KeyCode::Esc => {
                    self.searching = false;
                    self.search_query.clear();
                }
                KeyCode::Enter => {
                    self.searching = false;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Char(c) if self.search_query.len() < 30 => {
                    self.search_query.push(c);
                }
                _ => {}
            }
            return Ok(());
        }

        // If a task or journal modal is active
        if self.modal_state != ModalType::None {
            self.handle_workspace_modal_key(key, p_id)?;
            return Ok(());
        }

        let all_tasks = self.all_tasks.clone();
        // When viewing steps (→ drill-down), proj_tasks = steps of that task only.
        // In the main view, proj_tasks is a flat list: sorted parents + their incomplete steps inline.
        let proj_tasks: Vec<Task> = if let Some(parent_id) = self.viewing_step_for_task {
            let mut steps: Vec<Task> = all_tasks
                .iter()
                .filter(|t| t.parent_task_id == Some(parent_id))
                .cloned()
                .collect();
            // Open steps first, then by the same sort the user chose for parent tasks
            steps.sort_by(|a, b| {
                a.completed.cmp(&b.completed).then_with(|| match self.task_sort.as_str() {
                    "DueDate" => match (a.due_date, b.due_date) {
                        (Some(d1), Some(d2)) => d1.cmp(&d2),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.created_at.cmp(&b.created_at),
                    },
                    "Priority" => b.priority.cmp(&a.priority),
                    "Alphabetical" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                    _ => b.created_at.cmp(&a.created_at),
                })
            });
            steps
        } else {
            // Collect and sort parent tasks: open first, then by the chosen sort key
            let mut parents: Vec<Task> = all_tasks
                .iter()
                .filter(|t| t.project_id == Some(p_id) && t.parent_task_id.is_none())
                .filter(|t| match self.task_filter.as_str() {
                    "Incomplete" => !t.completed,
                    "Completed" => t.completed,
                    _ => true,
                })
                .filter(|t| {
                    if !self.search_query.is_empty() {
                        t.title
                            .to_lowercase()
                            .contains(&self.search_query.to_lowercase())
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();

            match self.task_sort.as_str() {
                "DueDate" => parents.sort_by(|a, b| {
                    a.completed.cmp(&b.completed).then_with(|| match (a.due_date, b.due_date) {
                        (Some(d1), Some(d2)) => d1.cmp(&d2),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.created_at.cmp(&b.created_at),
                    })
                }),
                "Priority" => parents.sort_by(|a, b| {
                    a.completed.cmp(&b.completed)
                        .then_with(|| b.priority.cmp(&a.priority))
                }),
                "Alphabetical" => parents.sort_by(|a, b| {
                    a.completed.cmp(&b.completed)
                        .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                }),
                _ => parents.sort_by(|a, b| {
                    a.completed.cmp(&b.completed)
                        .then_with(|| b.created_at.cmp(&a.created_at))
                }),
            }

            // Build flat list: each parent followed by its incomplete steps
            let mut flat = Vec::new();
            for parent in parents {
                flat.push(parent.clone());
                if !parent.completed {
                    let mut steps: Vec<Task> = all_tasks
                        .iter()
                        .filter(|t| t.parent_task_id == Some(parent.id) && !t.completed)
                        .cloned()
                        .collect();
                    steps.sort_by(|a, b| match self.task_sort.as_str() {
                        "DueDate" => match (a.due_date, b.due_date) {
                            (Some(d1), Some(d2)) => d1.cmp(&d2),
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (None, None) => a.created_at.cmp(&b.created_at),
                        },
                        "Priority" => b.priority.cmp(&a.priority),
                        "Alphabetical" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                        _ => b.created_at.cmp(&a.created_at),
                    });
                    flat.extend(steps);
                }
            }
            flat
        };

        // Evita que el índice quede fuera de rango cuando se completan pasos y la lista se encoge
        if !proj_tasks.is_empty() && self.selected_task_idx >= proj_tasks.len() {
            self.selected_task_idx = proj_tasks.len() - 1;
        }

        let proj_notes: Vec<Note> = self.all_notes
            .iter()
            .filter(|n| n.project_id == Some(p_id))
            .filter(|n| {
                if !self.search_query.is_empty() {
                    n.title
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        let proj_journals: Vec<JournalEntry> = self.all_journals
            .iter()
            .filter(|j| j.project_id == p_id)
            .filter(|j| {
                if !self.search_query.is_empty() {
                    j.content
                        .to_lowercase()
                        .contains(&self.search_query.to_lowercase())
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        match key.code {
            KeyCode::Esc => {
                if self.viewing_step_for_task.is_some() {
                    // Exit step view back to quest list
                    self.viewing_step_for_task = None;
                    self.selected_task_idx = 0;
                } else {
                    // Exit workspace to Projects
                    self.active_project_id = None;
                    self.active_screen = ActiveScreen::Projects;
                    self.workspace_sidebar_focused = false;
                    self.reload_data()?;
                }
            }
            KeyCode::Char('/') => {
                self.searching = true;
                self.search_query.clear();
            }
            KeyCode::Char('1') => { self.workspace_tab_idx = 0; self.workspace_sidebar_focused = false; }
            KeyCode::Char('2') => { self.workspace_tab_idx = 1; self.workspace_sidebar_focused = false; }
            KeyCode::Char('3') => { self.workspace_tab_idx = 2; self.workspace_sidebar_focused = false; }
            KeyCode::Char('4') => { self.workspace_tab_idx = 3; self.workspace_sidebar_focused = false; }
            KeyCode::Tab => { self.workspace_tab_idx = (self.workspace_tab_idx + 1) % 4; self.workspace_sidebar_focused = false; }
            KeyCode::BackTab => {
                self.workspace_tab_idx = if self.workspace_tab_idx > 0 { self.workspace_tab_idx - 1 } else { 3 };
                self.workspace_sidebar_focused = false;
            }
            // Left: in step view → go back; in content → focus sidebar
            KeyCode::Left if !self.workspace_sidebar_focused && self.viewing_step_for_task.is_some() && self.workspace_tab_idx == 0 => {
                self.viewing_step_for_task = None;
                self.selected_task_idx = 0;
            }
            KeyCode::Left if !self.workspace_sidebar_focused => {
                self.workspace_sidebar_focused = true;
            }
            // Right: in quests tab → drill into steps (parent tasks only); in sidebar → return to content
            KeyCode::Right if !self.workspace_sidebar_focused && self.viewing_step_for_task.is_none() && self.workspace_tab_idx == 0 => {
                if !proj_tasks.is_empty() && self.selected_task_idx < proj_tasks.len() {
                    let task = &proj_tasks[self.selected_task_idx];
                    // Only enter step view for parent tasks, not for inline step items
                    if task.parent_task_id.is_none() {
                        let task_id = task.id;
                        self.viewing_step_for_task = Some(task_id);
                        self.selected_task_idx = 0;
                    }
                }
            }
            KeyCode::Right if self.workspace_sidebar_focused => {
                self.workspace_sidebar_focused = false;
            }
            // Up/Down: switch tabs in sidebar mode, navigate items in content mode
            KeyCode::Up if self.workspace_sidebar_focused => {
                self.workspace_tab_idx = if self.workspace_tab_idx > 0 { self.workspace_tab_idx - 1 } else { 3 };
            }
            KeyCode::Down if self.workspace_sidebar_focused => {
                self.workspace_tab_idx = (self.workspace_tab_idx + 1) % 4;
            }
            // Navigation inside Workspace active lists
            KeyCode::Up => match self.workspace_tab_idx {
                0 => {
                    if !proj_tasks.is_empty() {
                        if self.selected_task_idx > 0 {
                            self.selected_task_idx -= 1;
                        } else {
                            self.selected_task_idx = proj_tasks.len() - 1;
                        }
                    }
                }
                1 => {
                    let flat = Self::build_notes_flat(&proj_notes, &self.codices, p_id);
                    if !flat.is_empty() {
                        let len = flat.len();
                        let mut idx = if self.selected_notes_flat_idx > 0 {
                            self.selected_notes_flat_idx - 1
                        } else {
                            len - 1
                        };
                        // skip dividers (None, None)
                        while flat[idx] == (None, None) {
                            idx = if idx > 0 { idx - 1 } else { len - 1 };
                        }
                        self.selected_notes_flat_idx = idx;
                        if let Some(note_idx) = flat[idx].1 {
                            self.selected_note_idx = note_idx;
                        }
                    }
                }
                2 => {
                    if !proj_journals.is_empty() {
                        if self.selected_journal_idx > 0 {
                            self.selected_journal_idx -= 1;
                        } else {
                            self.selected_journal_idx = proj_journals.len() - 1;
                        }
                    }
                }
                3 => {
                    if let Ok(milestones) = self.db.get_milestones_for_project(p_id) {
                        if !milestones.is_empty() {
                            self.selected_milestone_idx = if self.selected_milestone_idx > 0 {
                                self.selected_milestone_idx - 1
                            } else {
                                milestones.len() - 1
                            };
                        }
                    }
                }
                _ => {}
            },
            KeyCode::Down => match self.workspace_tab_idx {
                0 => {
                    if !proj_tasks.is_empty() {
                        if self.selected_task_idx < proj_tasks.len() - 1 {
                            self.selected_task_idx += 1;
                        } else {
                            self.selected_task_idx = 0;
                        }
                    }
                }
                1 => {
                    let flat = Self::build_notes_flat(&proj_notes, &self.codices, p_id);
                    if !flat.is_empty() {
                        let len = flat.len();
                        let mut idx = (self.selected_notes_flat_idx + 1) % len;
                        while flat[idx] == (None, None) {
                            idx = (idx + 1) % len;
                        }
                        self.selected_notes_flat_idx = idx;
                        if let Some(note_idx) = flat[idx].1 {
                            self.selected_note_idx = note_idx;
                        }
                    }
                }
                2 => {
                    if !proj_journals.is_empty() {
                        if self.selected_journal_idx < proj_journals.len() - 1 {
                            self.selected_journal_idx += 1;
                        } else {
                            self.selected_journal_idx = 0;
                        }
                    }
                }
                3 => {
                    if let Ok(milestones) = self.db.get_milestones_for_project(p_id) {
                        if !milestones.is_empty() {
                            self.selected_milestone_idx =
                                (self.selected_milestone_idx + 1) % milestones.len();
                        }
                    }
                }
                _ => {}
            },
            // Space: Complete / Reopen Tasks & Steps, Toggle Milestones
            KeyCode::Char(' ') => {
                if self.workspace_tab_idx == 0
                    && !proj_tasks.is_empty()
                    && self.selected_task_idx < proj_tasks.len()
                {
                    let task = proj_tasks[self.selected_task_idx].clone();
                    let is_step = task.parent_task_id.is_some();

                    if is_step {
                        if task.completed {
                            // Reabrir paso — el XP ya fue cobrado, no se devuelve
                            let mut t = task.clone();
                            t.completed = false;
                            self.db.update_task(&t)?;
                            self.mark_dirty();
                            self.notifications.push(Notification::info("Trial reopened. XP already claimed — face it again with fresh resolve.".to_string()));
                            self.reload_data()?;
                        } else {
                            let mut t = task.clone();
                            t.completed = true;
                            t.xp_awarded = true;
                            self.db.update_task(&t)?;
                            self.mark_dirty();
                            self.audio_player.play_task_complete();
                            if !task.xp_awarded {
                                let is_high = t.priority == TaskPriority::High;
                                let xp = if is_high { 50 } else { 25 };
                                let label = if is_high { "Resolve Hero Step Quest" } else { "Complete Step Quest" };
                                self.grant_xp(label, xp)?;
                                self.apply_class_passive("task_complete", 0)?;
                            }
                            self.trigger_ambient_particles();
                            self.complete_productive_action()?;
                            self.grow_tree(2)?;
                            self.update_daily_adventure_progress("complete_tasks", 1)?;
                            self.check_action_achievements()?;
                            self.reload_data()?;
                        }
                    } else {
                        if task.completed {
                            // Reabrir quest — los pasos también se reabren
                            let steps: Vec<Task> = self.all_tasks.iter()
                                .filter(|t| t.parent_task_id == Some(task.id))
                                .cloned()
                                .collect();
                            for step in &steps {
                                if step.completed {
                                    let mut s = step.clone();
                                    s.completed = false;
                                    self.db.update_task(&s)?;
                                }
                            }
                            let mut t = task.clone();
                            t.completed = false;
                            self.db.update_task(&t)?;
                            self.mark_dirty();
                            self.notifications.push(Notification::info("Quest unsealed. XP already claimed — the path forward is yours to walk again.".to_string()));
                            self.reload_data()?;
                        } else {
                            let incomplete_steps: Vec<Task> = self.all_tasks.iter()
                                .filter(|t| t.parent_task_id == Some(task.id) && !t.completed)
                                .cloned()
                                .collect();
                            if !incomplete_steps.is_empty() {
                                // Bloquear hasta que todos los pasos estén cerrados
                                let n = incomplete_steps.len();
                                let msg = if n == 1 {
                                    "One trial remains unsealed. Face it before this quest can be closed.".to_string()
                                } else {
                                    format!("{} trials remain unsealed. Resolve them before this quest can be claimed.", n)
                                };
                                self.notifications.push(Notification::warning(msg));
                            } else {
                                let total_steps = self.all_tasks.iter()
                                    .filter(|t| t.parent_task_id == Some(task.id))
                                    .count();
                                let mut t = task.clone();
                                t.completed = true;
                                t.xp_awarded = true;
                                self.db.update_task(&t)?;
                                self.mark_dirty();
                                self.audio_player.play_task_complete();
                                if !task.xp_awarded {
                                    let is_high = t.priority == TaskPriority::High;
                                    let val = if is_high { 50 } else { 25 };
                                    let title = if is_high { "Resolve Hero Task Quest" } else { "Complete Task Quest" };
                                    self.grant_xp(title, val)?;
                                    let passive_trigger = if is_high { "high_priority_task" } else { "task_complete" };
                                    self.apply_class_passive(passive_trigger, 0)?;
                                }
                                self.trigger_ambient_particles();
                                self.increment_quest_progress(10, 1)?;
                                let frag_trigger = if task.priority == TaskPriority::High { "high_priority_task" } else { "task" };
                                self.simulate_memory_fragment_unlock(frag_trigger)?;
                                self.complete_productive_action()?;
                                let growth = if task.priority == TaskPriority::High { 4 } else { 2 };
                                self.grow_tree(growth)?;
                                self.update_daily_adventure_progress("complete_tasks", 1)?;
                                if task.priority == TaskPriority::High {
                                    self.update_daily_adventure_progress("complete_high_priority_task", 1)?;
                                }
                                self.check_action_achievements()?;
                                let chronicle_desc = if total_steps > 0 {
                                    format!("completed 1 quest with {} step{}.", total_steps, if total_steps == 1 { "" } else { "s" })
                                } else {
                                    "completed 1 quest.".to_string()
                                };
                                self.push_great_chronicle_async("QuestComplete", &chronicle_desc, false);
                                self.maybe_spawn_task_completion_sprite();
                                // Tarea recurrente — genera la siguiente ocurrencia automáticamente
                                if let Some(recurrence) = t.recurrence {
                                    let next_due = Self::advance_recurrence_date(t.due_date, recurrence);
                                    let next_task = Task {
                                        id: Uuid::new_v4(),
                                        project_id: t.project_id,
                                        title: t.title.clone(),
                                        description: t.description.clone(),
                                        due_date: Some(next_due),
                                        completed: false,
                                        priority: t.priority,
                                        created_at: Utc::now(),
                                        updated_at: Utc::now(),
                                        owner_identity: t.owner_identity.clone(),
                                        owner_username: t.owner_username.clone(),
                                        parent_task_id: None,
                                        xp_awarded: false,
                                        recurrence: Some(recurrence),
                                    };
                                    let _ = self.db.insert_task(&next_task);
                                    let recur_label = recurrence.name();
                                    self.notifications.push(Notification::info(format!("Quest recurring! Next {} occurrence queued for {}.", recur_label, next_due.format("%Y-%m-%d"))));
                                }
                                self.reload_data()?;
                            }
                        }
                    }
                } else if self.workspace_tab_idx == 3 {
                    if let Ok(milestones) = self.db.get_milestones_for_project(p_id) {
                        if !milestones.is_empty() && self.selected_milestone_idx < milestones.len()
                        {
                            let m_id = milestones[self.selected_milestone_idx].id;
                            self.toggle_milestone(m_id)?;
                        }
                    }
                }
            }
            // d: New Codex in Notes tab
            KeyCode::Char('d') if self.workspace_tab_idx == 1 => {
                self.modal_state = ModalType::NewCodex { name: String::new() };
            }
            // r: Refile selected scroll to a different codex
            KeyCode::Char('r') if self.workspace_tab_idx == 1 => {
                let flat = Self::build_notes_flat(&proj_notes, &self.codices, p_id);
                if let Some((_, Some(note_idx))) = flat.get(self.selected_notes_flat_idx) {
                    if *note_idx < proj_notes.len() {
                        let note_id = proj_notes[*note_idx].id;
                        let current_codex_id = proj_notes[*note_idx].codex_id;
                        // Pre-select the current codex in the picker (0 = Ungrouped, 1..=n = codices)
                        let selected_idx = current_codex_id
                            .and_then(|cid| self.codices.iter().position(|c| c.id == cid))
                            .map(|pos| pos + 1)
                            .unwrap_or(0);
                        self.modal_state = ModalType::RefileScroll { note_id, selected_idx };
                    }
                }
            }
            // +: Add Step to selected quest (from quest list, no need to enter step view)
            KeyCode::Char('+') if self.workspace_tab_idx == 0 && self.viewing_step_for_task.is_none() => {
                if !proj_tasks.is_empty() && self.selected_task_idx < proj_tasks.len() {
                    let task = &proj_tasks[self.selected_task_idx];
                    // If on an inline step, add to its parent; otherwise add to the selected parent
                    let parent_id = task.parent_task_id.unwrap_or(task.id);
                    self.modal_state = ModalType::NewTask {
                        title: String::new(),
                        desc: String::new(),
                        desc_cursor: 0,
                        priority: TaskPriority::Medium,
                        due_date_type: DueDateType::None,
                        due_date_val: String::new(),
                        focus_idx: 0,
                        parent_task_id: Some(parent_id),
                        recurrence: None,
                    };
                }
            }
            // n/m: New Note / Task / Milestone / Journal
            KeyCode::Char('n') | KeyCode::Char('m') => {
                if self.workspace_tab_idx == 0 && key.code == KeyCode::Char('n') {
                    // When in step view, create a step under the viewed task
                    let parent_id = self.viewing_step_for_task;
                    self.modal_state = ModalType::NewTask {
                        title: String::new(),
                        desc: String::new(),
                        desc_cursor: 0,
                        priority: TaskPriority::Medium,
                        due_date_type: DueDateType::None,
                        due_date_val: String::new(),
                        focus_idx: 0,
                        parent_task_id: parent_id,
                        recurrence: None,
                    };
                } else if self.workspace_tab_idx == 1 && key.code == KeyCode::Char('n') {
                    // If selected flat item is a codex header, create note inside that codex
                    let flat = Self::build_notes_flat(&proj_notes, &self.codices, p_id);
                    let codex_id = flat.get(self.selected_notes_flat_idx)
                        .and_then(|(cid, _)| *cid);
                    let mut state = EditorState::new(p_id, None, String::new(), String::new());
                    state.codex_id = codex_id;
                    self.editor_state = Some(state);
                    self.active_screen = ActiveScreen::Editor;
                } else if self.workspace_tab_idx == 2 && key.code == KeyCode::Char('n') {
                    self.modal_state = ModalType::NewJournalEntry {
                        content: String::new(),
                    };
                } else if self.workspace_tab_idx == 3 {
                    self.modal_state = ModalType::MilestoneTierSelect {
                        project_id: p_id,
                        selected_idx: 0,
                    };
                }
            }
            // Enter or e: Edit Note / Task
            KeyCode::Enter | KeyCode::Char('e') => {
                if self.workspace_tab_idx == 0
                    && !proj_tasks.is_empty()
                    && self.selected_task_idx < proj_tasks.len()
                {
                    let t = &proj_tasks[self.selected_task_idx];
                    if t.completed {
                        // Completed tasks cannot be edited
                        return Ok(());
                    }
                    let (due_type, due_val) = match t.due_date {
                        None => (DueDateType::None, String::new()),
                        Some(d) => {
                            let today = Utc::now().date_naive();
                            let d_naive = d.date_naive();
                            if d_naive == today {
                                (DueDateType::Today, String::new())
                            } else if d_naive == today + chrono::Duration::days(1) {
                                (DueDateType::Tomorrow, String::new())
                            } else {
                                (DueDateType::Specific, d.format("%Y-%m-%d").to_string())
                            }
                        }
                    };
                    let t_desc = t.description.clone().unwrap_or_default();
                    let t_desc_len = t_desc.len();
                    self.modal_state = ModalType::EditTask {
                        id: t.id,
                        title: t.title.clone(),
                        desc: t_desc,
                        desc_cursor: t_desc_len,
                        priority: t.priority,
                        due_date_type: due_type,
                        due_date_val: due_val,
                        focus_idx: 0,
                        step_selected_idx: 0,
                        is_step: t.parent_task_id.is_some(),
                        recurrence: t.recurrence,
                    };
                } else if self.workspace_tab_idx == 1 {
                    let flat = Self::build_notes_flat(&proj_notes, &self.codices, p_id);
                    match flat.get(self.selected_notes_flat_idx) {
                        Some((_, Some(note_idx))) if *note_idx < proj_notes.len() => {
                            let n = &proj_notes[*note_idx];
                            let mut state = EditorState::new(p_id, Some(n.id), n.title.clone(), n.markdown_content.clone());
                            state.codex_id = n.codex_id;
                            self.editor_state = Some(state);
                            self.active_screen = ActiveScreen::Editor;
                        }
                        Some((Some(codex_id), None)) => {
                            let cid = *codex_id;
                            let current_name = self.codices.iter()
                                .find(|c| c.id == cid)
                                .map(|c| c.name.clone())
                                .unwrap_or_default();
                            self.modal_state = ModalType::RenameCodex {
                                codex_id: cid,
                                name: current_name,
                            };
                        }
                        _ => {}
                    }
                }
            }
            // Delete: Slay Note / Task / Codex / Milestone
            KeyCode::Delete => {
                if self.workspace_tab_idx == 0
                    && !proj_tasks.is_empty()
                    && self.selected_task_idx < proj_tasks.len()
                {
                    let t = &proj_tasks[self.selected_task_idx];
                    self.db.delete_task(t.id)?;
                    self.mark_dirty();
                    self.selected_task_idx = 0;
                    self.reload_data()?;
                } else if self.workspace_tab_idx == 1 {
                    let flat = Self::build_notes_flat(&proj_notes, &self.codices, p_id);
                    match flat.get(self.selected_notes_flat_idx) {
                        Some((_, Some(note_idx))) if *note_idx < proj_notes.len() => {
                            let n = &proj_notes[*note_idx];
                            self.db.delete_note(n.id)?;
                            self.mark_dirty();
                            self.selected_notes_flat_idx = 0;
                            self.selected_note_idx = 0;
                            self.reload_data()?;
                        }
                        Some((Some(codex_id), None)) => {
                            let cid = *codex_id;
                            let name = self.codices.iter()
                                .find(|c| c.id == cid)
                                .map(|c| c.name.clone())
                                .unwrap_or_default();
                            self.modal_state = ModalType::ConfirmDeleteCodex {
                                codex_id: cid,
                                codex_name: name,
                            };
                        }
                        _ => {}
                    }
                } else if self.workspace_tab_idx == 3 {
                    if let Ok(milestones) = self.db.get_milestones_for_project(p_id) {
                        if !milestones.is_empty() && self.selected_milestone_idx < milestones.len()
                        {
                            let m_id = milestones[self.selected_milestone_idx].id;
                            self.db.delete_milestone(m_id)?;
                            self.mark_dirty();
                            self.selected_milestone_idx = 0;
                            self.reload_data()?;
                        }
                    }
                }
            }
            // j: New journal entry
            KeyCode::Char('j') => {
                if self.workspace_tab_idx == 2 {
                    self.modal_state = ModalType::NewJournalEntry {
                        content: String::new(),
                    };
                }
            }
            // f: filter tasks
            KeyCode::Char('f') => {
                if self.workspace_tab_idx == 0 {
                    self.task_filter = match self.task_filter.as_str() {
                        "All" => "Incomplete".to_string(),
                        "Incomplete" => "Completed".to_string(),
                        _ => "All".to_string(),
                    };
                    self.selected_task_idx = 0;
                }
            }
            // s: sort tasks or share note
            KeyCode::Char('s') => {
                if self.workspace_tab_idx == 0 {
                    self.task_sort = match self.task_sort.as_str() {
                        "CreatedDate" => "DueDate".to_string(),
                        "DueDate" => "Priority".to_string(),
                        "Priority" => "Alphabetical".to_string(),
                        _ => "CreatedDate".to_string(),
                    };
                    self.selected_task_idx = 0;
                } else if self.workspace_tab_idx == 1 && !proj_notes.is_empty() {
                    let flat = Self::build_notes_flat(&proj_notes, &self.codices, p_id);
                    if let Some((_, Some(note_idx))) = flat.get(self.selected_notes_flat_idx) {
                        if *note_idx < proj_notes.len() {
                            let n = &proj_notes[*note_idx];
                            self.modal_state = ModalType::ShareNote {
                                note_id: n.id,
                                permission_idx: 0,
                            };
                        }
                    }
                }
            }
            // a: Assign task (shared projects only)
            KeyCode::Char('a') => {
                let is_shared = self
                    .projects
                    .iter()
                    .find(|p| p.id == p_id)
                    .map(|p| p.is_shared)
                    .unwrap_or(false);
                if is_shared
                    && self.workspace_tab_idx == 0
                    && !proj_tasks.is_empty()
                    && self.selected_task_idx < proj_tasks.len()
                {
                    let t = &proj_tasks[self.selected_task_idx];
                    self.modal_state = ModalType::AssignTask {
                        task_id: t.id,
                        selected_member_idx: 0,
                    };
                }
            }
            // v: journal visibility
            KeyCode::Char('v') => {
                if self.workspace_tab_idx == 2
                    && !proj_journals.is_empty()
                    && self.selected_journal_idx < proj_journals.len()
                {
                    let j = &proj_journals[self.selected_journal_idx];
                    self.modal_state = ModalType::JournalVisibility {
                        entry_id: j.id,
                        visibility_idx: 0,
                    };
                }
            }
            // c: complete project
            KeyCode::Char('c') if self.workspace_tab_idx == 3 => {
                self.complete_project(p_id)?;
                self.active_project_id = None;
                self.active_screen = ActiveScreen::Projects;
                return Ok(());
            }
            KeyCode::Char('?') => {
                self.workspace_help_open = true;
            }
            _ => {}
        }
        Ok(())
    }

    // Handles Task and Journal entry dialog popups keys.
    fn handle_workspace_modal_key(&mut self, key: KeyEvent, project_id: Uuid) -> Result<()> {
        match self.modal_state {
            ModalType::NewTask {
                ref title,
                ref desc,
                desc_cursor,
                priority,
                due_date_type,
                ref due_date_val,
                focus_idx,
                parent_task_id,
                recurrence,
            } => {
                let is_step = parent_task_id.is_some();
                self.handle_task_modal_key(
                    key,
                    project_id,
                    None,
                    title.clone(),
                    desc.clone(),
                    desc_cursor,
                    priority,
                    due_date_type,
                    due_date_val.clone(),
                    focus_idx,
                    parent_task_id,
                    0,
                    is_step,
                    recurrence,
                )?;
            }
            ModalType::EditTask {
                id,
                ref title,
                ref desc,
                desc_cursor,
                priority,
                due_date_type,
                ref due_date_val,
                focus_idx,
                step_selected_idx,
                is_step,
                recurrence,
            } => {
                // índice del bloque de steps — depende de si hay campo de valor de fecha
                let has_due_value = matches!(due_date_type, DueDateType::InDays | DueDateType::Specific);
                let steps_focus = if has_due_value { 6usize } else { 5usize };

                // Handle step-section keys when focus is on the steps list
                if focus_idx == steps_focus && !is_step {
                    let title_c = title.clone();
                    let desc_c = desc.clone();
                    let due_val_c = due_date_val.clone();
                    let steps: Vec<Task> = self.db.get_tasks()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|t| t.parent_task_id == Some(id))
                        .collect();
                    match key.code {
                        KeyCode::Up => {
                            let new_idx = if step_selected_idx > 0 { step_selected_idx - 1 } else { 0 };
                            self.modal_state = ModalType::EditTask {
                                id, title: title_c, desc: desc_c, desc_cursor, priority, due_date_type,
                                due_date_val: due_val_c, focus_idx: steps_focus, step_selected_idx: new_idx, is_step, recurrence,
                            };
                            return Ok(());
                        }
                        KeyCode::Down => {
                            let max = steps.len().saturating_sub(1);
                            let new_idx = (step_selected_idx + 1).min(max);
                            self.modal_state = ModalType::EditTask {
                                id, title: title_c, desc: desc_c, desc_cursor, priority, due_date_type,
                                due_date_val: due_val_c, focus_idx: steps_focus, step_selected_idx: new_idx, is_step, recurrence,
                            };
                            return Ok(());
                        }
                        KeyCode::Char(' ') => {
                            if let Some(step) = steps.get(step_selected_idx) {
                                if !step.completed {
                                    let mut s = step.clone();
                                    s.completed = true;
                                    self.db.update_task(&s)?;
                                    self.mark_dirty();
                                    self.audio_player.play_task_complete();
                                    let is_high = s.priority == TaskPriority::High;
                                    let xp = if is_high { 50 } else { 25 };
                                    let label = if is_high { "Resolve Hero Step Quest" } else { "Complete Step Quest" };
                                    self.grant_xp(label, xp)?;
                                    self.apply_class_passive("task_complete", 0)?;
                                    self.trigger_ambient_particles();
                                    self.complete_productive_action()?;
                                    self.grow_tree(2)?;
                                    self.update_daily_adventure_progress("complete_tasks", 1)?;
                                    self.check_action_achievements()?;
                                    self.reload_data()?;
                                }
                            }
                            self.modal_state = ModalType::EditTask {
                                id, title: title_c, desc: desc_c, desc_cursor, priority, due_date_type,
                                due_date_val: due_val_c, focus_idx: steps_focus, step_selected_idx, is_step, recurrence,
                            };
                            return Ok(());
                        }
                        KeyCode::Delete => {
                            if let Some(step) = steps.get(step_selected_idx) {
                                self.db.delete_task(step.id)?;
                                self.mark_dirty();
                                self.reload_data()?;
                            }
                            let new_idx = if step_selected_idx > 0 { step_selected_idx - 1 } else { 0 };
                            self.modal_state = ModalType::EditTask {
                                id, title: title_c, desc: desc_c, desc_cursor, priority, due_date_type,
                                due_date_val: due_val_c, focus_idx: steps_focus, step_selected_idx: new_idx, is_step, recurrence,
                            };
                            return Ok(());
                        }
                        KeyCode::Char('n') | KeyCode::Char('+') => {
                            // Open step creation on top of this EditTask (overlay)
                            self.overlay_modal = ModalType::NewTask {
                                title: String::new(),
                                desc: String::new(),
                                desc_cursor: 0,
                                priority: TaskPriority::Medium,
                                due_date_type: DueDateType::None,
                                due_date_val: String::new(),
                                focus_idx: 0,
                                parent_task_id: Some(id),
                                recurrence: None,
                            };
                            return Ok(());
                        }
                        KeyCode::Char('e') => {
                            if let Some(step) = steps.get(step_selected_idx) {
                                if !step.completed {
                                    let today = Utc::now().date_naive();
                                    let (due_type, due_val) = match step.due_date {
                                        None => (DueDateType::None, String::new()),
                                        Some(d) => {
                                            let d_naive = d.date_naive();
                                            if d_naive == today {
                                                (DueDateType::Today, String::new())
                                            } else if d_naive == today + chrono::Duration::days(1) {
                                                (DueDateType::Tomorrow, String::new())
                                            } else {
                                                (DueDateType::Specific, d.format("%Y-%m-%d").to_string())
                                            }
                                        }
                                    };
                                    let s_desc = step.description.clone().unwrap_or_default();
                                    let s_desc_len = s_desc.len();
                                    self.modal_state = ModalType::EditTask {
                                        id: step.id,
                                        title: step.title.clone(),
                                        desc: s_desc,
                                        desc_cursor: s_desc_len,
                                        priority: step.priority,
                                        due_date_type: due_type,
                                        due_date_val: due_val,
                                        focus_idx: 0,
                                        step_selected_idx: 0,
                                        is_step: true,
                                        recurrence: None,
                                    };
                                }
                            }
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                self.handle_task_modal_key(
                    key,
                    project_id,
                    Some(id),
                    title.clone(),
                    desc.clone(),
                    desc_cursor,
                    priority,
                    due_date_type,
                    due_date_val.clone(),
                    focus_idx,
                    None,
                    step_selected_idx,
                    is_step,
                    recurrence,
                )?;
            }
            ModalType::NewJournalEntry { ref content } => {
                self.handle_journal_modal_key(key, project_id, content.clone())?;
            }
            ModalType::NewCodex { ref name } => {
                let name = name.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Enter if !name.trim().is_empty() => {
                        use crate::models::Codex;
                        let codex = Codex {
                            id: Uuid::new_v4(),
                            project_id,
                            name: name.trim().to_string(),
                            created_at: Utc::now(),
                        };
                        self.modal_state = ModalType::None;
                        self.db.insert_codex(&codex)?;
                        self.mark_dirty();
                        self.reload_data()?;
                        let _ = self.check_action_achievements();
                    }
                    KeyCode::Backspace => {
                        let mut n = name.clone();
                        n.pop();
                        self.modal_state = ModalType::NewCodex { name: n };
                    }
                    KeyCode::Char(c) => {
                        if name.len() < 40 {
                            let mut n = name.clone();
                            n.push(c);
                            self.modal_state = ModalType::NewCodex { name: n };
                        }
                    }
                    _ => {}
                }
            }
            ModalType::RenameCodex { codex_id, ref name } => {
                let cid = codex_id;
                let name = name.clone();
                match key.code {
                    KeyCode::Esc => {
                        self.modal_state = ModalType::None;
                    }
                    KeyCode::Enter if !name.trim().is_empty() => {
                        self.modal_state = ModalType::None;
                        self.db.update_codex_name(cid, name.trim())?;
                        self.mark_dirty();
                        self.reload_data()?;
                    }
                    KeyCode::Backspace => {
                        let mut n = name.clone();
                        n.pop();
                        self.modal_state = ModalType::RenameCodex { codex_id: cid, name: n };
                    }
                    KeyCode::Char(c) => {
                        if name.len() < 40 {
                            let mut n = name.clone();
                            n.push(c);
                            self.modal_state = ModalType::RenameCodex { codex_id: cid, name: n };
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    // Task Dialog keys.
    fn handle_task_modal_key(
        &mut self,
        key: KeyEvent,
        project_id: Uuid,
        task_id: Option<Uuid>,
        mut title: String,
        mut desc: String,
        mut desc_cursor: usize,
        mut priority: TaskPriority,
        mut due_date_type: DueDateType,
        mut due_date_val: String,
        mut focus_idx: usize,
        parent_task_id: Option<Uuid>,
        step_selected_idx: usize,
        is_step: bool,
        mut recurrence: Option<RecurrenceType>,
    ) -> Result<()> {
        // Recurrencia disponible solo para tareas padre (no pasos ni subtareas)
        let show_recurrence = !is_step && parent_task_id.is_none();
        let has_due_value = matches!(due_date_type, DueDateType::InDays | DueDateType::Specific);
        // índice dinámico del campo de recurrencia — viene después del campo de valor de fecha si existe
        let recurrence_focus: usize = if has_due_value { 5 } else { 4 };
        let steps_focus: usize = recurrence_focus + 1;

        let has_steps_section = task_id.is_some() && !is_step;
        let max_fields = if has_steps_section {
            steps_focus
        } else if show_recurrence {
            recurrence_focus
        } else if has_due_value {
            4
        } else {
            3
        };
        let next_focus = |idx: usize| -> usize {
            (idx + 1) % (max_fields + 1)
        };
        let prev_focus = |idx: usize| -> usize {
            if idx > 0 { idx - 1 } else { max_fields }
        };
        match key.code {
            KeyCode::Esc => {
                self.modal_state = ModalType::None;
            }
            KeyCode::Tab => {
                focus_idx = next_focus(focus_idx);
                self.update_task_modal_state(
                    task_id, title, desc, desc_cursor, priority, due_date_type,
                    due_date_val, focus_idx, parent_task_id, step_selected_idx, is_step, recurrence,
                );
            }
            KeyCode::BackTab => {
                focus_idx = prev_focus(focus_idx);
                self.update_task_modal_state(
                    task_id, title, desc, desc_cursor, priority, due_date_type,
                    due_date_val, focus_idx, parent_task_id, step_selected_idx, is_step, recurrence,
                );
            }
            KeyCode::Left => {
                if focus_idx == 1 {
                    // Move cursor left in description
                    if desc_cursor > 0 {
                        desc_cursor -= 1;
                        while desc_cursor > 0 && !desc.is_char_boundary(desc_cursor) {
                            desc_cursor -= 1;
                        }
                    }
                } else if show_recurrence && focus_idx == recurrence_focus {
                    recurrence = match recurrence {
                        None => Some(RecurrenceType::Yearly),
                        Some(RecurrenceType::Daily) => None,
                        Some(RecurrenceType::Weekly) => Some(RecurrenceType::Daily),
                        Some(RecurrenceType::Monthly) => Some(RecurrenceType::Weekly),
                        Some(RecurrenceType::Yearly) => Some(RecurrenceType::Monthly),
                    };
                } else {
                    match focus_idx {
                        2 => {
                            priority = match priority {
                                TaskPriority::Low => TaskPriority::High,
                                TaskPriority::Medium => TaskPriority::Low,
                                TaskPriority::High => TaskPriority::Medium,
                            };
                        }
                        3 => {
                            due_date_type = match due_date_type {
                                DueDateType::None => DueDateType::Specific,
                                DueDateType::Today => DueDateType::None,
                                DueDateType::Tomorrow => DueDateType::Today,
                                DueDateType::InDays => DueDateType::Tomorrow,
                                DueDateType::Specific => DueDateType::InDays,
                            };
                        }
                        _ => {}
                    }
                }
                self.update_task_modal_state(
                    task_id, title, desc, desc_cursor, priority, due_date_type,
                    due_date_val, focus_idx, parent_task_id, step_selected_idx, is_step, recurrence,
                );
            }
            KeyCode::Right => {
                if focus_idx == 1 {
                    // Move cursor right in description
                    if desc_cursor < desc.len() {
                        desc_cursor += 1;
                        while desc_cursor < desc.len() && !desc.is_char_boundary(desc_cursor) {
                            desc_cursor += 1;
                        }
                    }
                } else if show_recurrence && focus_idx == recurrence_focus {
                    recurrence = match recurrence {
                        None => Some(RecurrenceType::Daily),
                        Some(RecurrenceType::Daily) => Some(RecurrenceType::Weekly),
                        Some(RecurrenceType::Weekly) => Some(RecurrenceType::Monthly),
                        Some(RecurrenceType::Monthly) => Some(RecurrenceType::Yearly),
                        Some(RecurrenceType::Yearly) => None,
                    };
                } else {
                    match focus_idx {
                        2 => {
                            priority = match priority {
                                TaskPriority::Low => TaskPriority::Medium,
                                TaskPriority::Medium => TaskPriority::High,
                                TaskPriority::High => TaskPriority::Low,
                            };
                        }
                        3 => {
                            due_date_type = match due_date_type {
                                DueDateType::None => DueDateType::Today,
                                DueDateType::Today => DueDateType::Tomorrow,
                                DueDateType::Tomorrow => DueDateType::InDays,
                                DueDateType::InDays => DueDateType::Specific,
                                DueDateType::Specific => DueDateType::None,
                            };
                        }
                        _ => {}
                    }
                }
                self.update_task_modal_state(
                    task_id, title, desc, desc_cursor, priority, due_date_type,
                    due_date_val, focus_idx, parent_task_id, step_selected_idx, is_step, recurrence,
                );
            }
            KeyCode::Up => {
                if focus_idx == 1 {
                    // Move cursor up one line in description
                    let before = &desc[..desc_cursor];
                    let col = before.rfind('\n').map(|i| desc_cursor - i - 1).unwrap_or(desc_cursor);
                    if let Some(prev_nl) = before.rfind('\n') {
                        let prev_line_end = prev_nl;
                        let prev_line_start = desc[..prev_nl].rfind('\n').map(|i| i + 1).unwrap_or(0);
                        let prev_line_len = prev_line_end - prev_line_start;
                        desc_cursor = prev_line_start + col.min(prev_line_len);
                    }
                    self.update_task_modal_state(
                        task_id, title, desc, desc_cursor, priority, due_date_type,
                        due_date_val, focus_idx, parent_task_id, step_selected_idx, is_step, recurrence,
                    );
                }
                // For other fields: Up does nothing (Tab handles cycling)
            }
            KeyCode::Down => {
                if focus_idx == 1 {
                    // Move cursor down one line in description
                    let before = &desc[..desc_cursor];
                    let col = before.rfind('\n').map(|i| desc_cursor - i - 1).unwrap_or(desc_cursor);
                    if let Some(next_nl) = desc[desc_cursor..].find('\n') {
                        let next_line_start = desc_cursor + next_nl + 1;
                        let next_line_end = desc[next_line_start..].find('\n')
                            .map(|i| next_line_start + i)
                            .unwrap_or(desc.len());
                        let next_line_len = next_line_end - next_line_start;
                        desc_cursor = next_line_start + col.min(next_line_len);
                    }
                    self.update_task_modal_state(
                        task_id, title, desc, desc_cursor, priority, due_date_type,
                        due_date_val, focus_idx, parent_task_id, step_selected_idx, is_step, recurrence,
                    );
                }
                // For other fields: Down does nothing
            }
            KeyCode::Char(c) => {
                match focus_idx {
                    0 => {
                        if title.len() < 100 {
                            title.push(c);
                        }
                    }
                    1 => {
                        if desc.len() < 500 {
                            let cursor = desc_cursor.min(desc.len());
                            desc.insert(cursor, c);
                            desc_cursor = cursor + c.len_utf8();
                        }
                    }
                    2 => {
                        if c == ' ' {
                            priority = match priority {
                                TaskPriority::Low => TaskPriority::Medium,
                                TaskPriority::Medium => TaskPriority::High,
                                TaskPriority::High => TaskPriority::Low,
                            };
                        }
                    }
                    3 => {
                        if c == ' ' {
                            due_date_type = match due_date_type {
                                DueDateType::None => DueDateType::Today,
                                DueDateType::Today => DueDateType::Tomorrow,
                                DueDateType::Tomorrow => DueDateType::InDays,
                                DueDateType::InDays => DueDateType::Specific,
                                DueDateType::Specific => DueDateType::None,
                            };
                        }
                    }
                    4 if due_date_val.len() < 20 && !show_recurrence => {
                        due_date_val.push(c);
                    }
                    4 if due_date_val.len() < 20 && show_recurrence && has_due_value => {
                        due_date_val.push(c);
                    }
                    _ if show_recurrence && focus_idx == recurrence_focus && c == ' ' => {
                        recurrence = match recurrence {
                            None => Some(RecurrenceType::Daily),
                            Some(RecurrenceType::Daily) => Some(RecurrenceType::Weekly),
                            Some(RecurrenceType::Weekly) => Some(RecurrenceType::Monthly),
                            Some(RecurrenceType::Monthly) => Some(RecurrenceType::Yearly),
                            Some(RecurrenceType::Yearly) => None,
                        };
                    }
                    _ => {}
                }
                self.update_task_modal_state(
                    task_id, title, desc, desc_cursor, priority, due_date_type,
                    due_date_val, focus_idx, parent_task_id, step_selected_idx, is_step, recurrence,
                );
            }
            KeyCode::Backspace => {
                match focus_idx {
                    0 => { title.pop(); }
                    1 => {
                        if desc_cursor > 0 {
                            let cursor = desc_cursor.min(desc.len());
                            // find start of previous char
                            let prev = desc[..cursor].char_indices().next_back().map(|(i, _)| i).unwrap_or(0);
                            desc.remove(prev);
                            desc_cursor = prev;
                        }
                    }
                    4 => { due_date_val.pop(); }
                    _ => {}
                }
                self.update_task_modal_state(
                    task_id, title, desc, desc_cursor, priority, due_date_type,
                    due_date_val, focus_idx, parent_task_id, step_selected_idx, is_step, recurrence,
                );
            }
            KeyCode::Enter => {
                if focus_idx == 1 {
                    if desc.len() < 500 {
                        let cursor = desc_cursor.min(desc.len());
                        desc.insert(cursor, '\n');
                        desc_cursor = cursor + 1;
                        self.update_task_modal_state(
                            task_id, title, desc, desc_cursor, priority, due_date_type,
                            due_date_val, focus_idx, parent_task_id, step_selected_idx, is_step, recurrence,
                        );
                    }
                } else if !title.trim().is_empty() {
                    let task_desc = if desc.trim().is_empty() {
                        None
                    } else {
                        Some(desc.trim().to_string())
                    };
                    let due_str = match due_date_type {
                        DueDateType::None => String::new(),
                        DueDateType::Today => "today".to_string(),
                        DueDateType::Tomorrow => "tomorrow".to_string(),
                        DueDateType::InDays => format!("in {} days", due_date_val.trim()),
                        DueDateType::Specific => due_date_val.trim().to_string(),
                    };
                    let due_parsed = self.parse_due_date_input(&due_str);

                    if due_date_type != DueDateType::None && due_parsed.is_none() {
                        // Do not save, wait for valid input
                        return Ok(());
                    }

                    let xp_service = XPService::new(&self.db);
                    if let Some(id) = task_id {
                        let mut t = Task {
                            id,
                            project_id: Some(project_id),
                            title: title.trim().to_string(),
                            description: task_desc,
                            due_date: due_parsed,
                            completed: false, // kept
                            priority,
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                            owner_identity: Some(self.identity.public_key.clone()),
                            owner_username: Some(
                                self.user
                                    .as_ref()
                                    .map(|u| u.username.clone())
                                    .unwrap_or_else(|| "Gibranlp".to_string()),
                            ),
                            parent_task_id: None,
                            xp_awarded: false,
                            recurrence,
                        };
                        // Retrieve original status
                        if let Ok(ts) = self.db.get_tasks() {
                            if let Some(orig) = ts.iter().find(|o| o.id == id) {
                                t.completed = orig.completed;
                                t.created_at = orig.created_at;
                                t.owner_identity = orig.owner_identity.clone();
                                t.owner_username = orig.owner_username.clone();
                                t.parent_task_id = orig.parent_task_id;
                                t.xp_awarded = orig.xp_awarded;
                            }
                        }
                        self.db.update_task(&t)?;
                        self.mark_dirty();
                    } else {
                        // Create Task (or Step when parent_task_id is set)
                        let parent_id = if let ModalType::NewTask { parent_task_id, .. } = self.modal_state {
                            parent_task_id
                        } else {
                            None
                        };
                        let is_step = parent_id.is_some();
                        let t = Task {
                            id: Uuid::new_v4(),
                            project_id: Some(project_id),
                            title: title.trim().to_string(),
                            description: task_desc,
                            due_date: due_parsed,
                            completed: false,
                            priority,
                            created_at: Utc::now(),
                            updated_at: Utc::now(),
                            owner_identity: Some(self.identity.public_key.clone()),
                            owner_username: Some(
                                self.user
                                    .as_ref()
                                    .map(|u| u.username.clone())
                                    .unwrap_or_else(|| "Gibranlp".to_string()),
                            ),
                            parent_task_id: parent_id,
                            xp_awarded: false,
                            recurrence: if is_step { None } else { recurrence },
                        };
                        self.db.insert_task(&t)?;
                        self.audio_player.play_task_creation();

                        // Grant +5 XP on Create Task / Step
                        let xp_label = if is_step { "Spawn Step Quest" } else { "Spawn Task Quest" };
                        if let Some(ref mut u) = self.user {
                            xp_service.grant_xp(u, xp_label, 5)?;
                        }
                    }

                    self.mark_dirty();
                    self.reload_data()?;
                    // If we just created a step, reopen a fresh step form so the user
                    // can keep adding steps without navigating back to the task.
                    if is_step && task_id.is_none() {
                        self.modal_state = ModalType::NewTask {
                            title: String::new(),
                            desc: String::new(),
                            desc_cursor: 0,
                            priority: TaskPriority::Medium,
                            due_date_type: DueDateType::None,
                            due_date_val: String::new(),
                            focus_idx: 0,
                            parent_task_id,
                            recurrence: None,
                        };
                    } else {
                        self.modal_state = ModalType::None;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    // Modal state updating helper.
    fn update_task_modal_state(
        &mut self,
        id: Option<Uuid>,
        title: String,
        desc: String,
        desc_cursor: usize,
        priority: TaskPriority,
        due_date_type: DueDateType,
        due_date_val: String,
        focus_idx: usize,
        parent_task_id: Option<Uuid>,
        step_selected_idx: usize,
        is_step: bool,
        recurrence: Option<RecurrenceType>,
    ) {
        self.modal_state = if let Some(t_id) = id {
            ModalType::EditTask {
                id: t_id,
                title,
                desc,
                desc_cursor,
                priority,
                due_date_type,
                due_date_val,
                focus_idx,
                step_selected_idx,
                is_step,
                recurrence,
            }
        } else {
            ModalType::NewTask {
                title,
                desc,
                desc_cursor,
                priority,
                due_date_type,
                due_date_val,
                focus_idx,
                parent_task_id,
                recurrence,
            }
        };
    }

    // Journal dialog keys.
    fn handle_journal_modal_key(
        &mut self,
        key: KeyEvent,
        project_id: Uuid,
        mut content: String,
    ) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.modal_state = ModalType::None;
            }
            KeyCode::Char(c) => {
                if content.len() < 120 {
                    content.push(c);
                }
                self.modal_state = ModalType::NewJournalEntry { content };
            }
            KeyCode::Backspace => {
                content.pop();
                self.modal_state = ModalType::NewJournalEntry { content };
            }
            KeyCode::Enter if !content.trim().is_empty() => {
                let author = self.user.as_ref().map(|u| u.username.clone()).unwrap_or_default();
                let entry = JournalEntry {
                    id: Uuid::new_v4(),
                    project_id,
                    entry_date: Utc::now().date_naive(),
                    content: content.trim().to_string(),
                    created_at: Utc::now(),
                    visibility: "Private".to_string(),
                    author_username: author,
                };
                self.db.insert_journal_entry(&entry)?;
                self.mark_dirty();
                self.apply_class_passive("journal_create", 0)?;

                // Stage 3 Integration:
                self.complete_productive_action()?;
                self.update_daily_adventure_progress("write_journal", 1)?;
                self.check_action_achievements()?;

                self.reload_data()?;
                self.modal_state = ModalType::None;
            }
            _ => {}
        }
        Ok(())
    }

    fn sync_screen_tab(&mut self) {
        self.active_screen = match self.active_tab_idx {
            0 => ActiveScreen::Dashboard,
            1 => ActiveScreen::Projects,
            2 => ActiveScreen::Character,
            3 => ActiveScreen::Character,
            4 => ActiveScreen::Library,
            5 => ActiveScreen::Legends,
            6 => ActiveScreen::Focus,
            7 => ActiveScreen::Soundscapes,
            8 => {
                self.pull_invitations_async();
                ActiveScreen::Fellowship
            }
            9 => ActiveScreen::Dashboard,
            10 => ActiveScreen::Archive,
            11 => ActiveScreen::Dashboard,
            12 => ActiveScreen::SyncSettings,
            13 => {
                self.about_scroll = 0;
                use rand::Rng;
                self.about_fact_seed = rand::thread_rng().r#gen();
                ActiveScreen::About
            }
            14 => {
                self.great_chronicle_scroll = 0;
                self.chapter_panel_scroll = 0;
                self.chapter_panel_focused = false;
                self.great_chronicle_entries =
                    self.db.get_global_chronicle_entries().unwrap_or_default();
                self.pull_great_chronicle_async();
                self.pull_chapter_progress_async();
                ActiveScreen::GreatChronicle
            }
            _ => ActiveScreen::Dashboard,
        };
        self.active_project_id = None; // Reset workspace focus
    }

    // Calcula la siguiente fecha de una tarea recurrente — avanza por el período correcto
    fn advance_recurrence_date(due_date: Option<DateTime<Utc>>, recurrence: RecurrenceType) -> DateTime<Utc> {
        use chrono::Months;
        let base = due_date.unwrap_or_else(Utc::now);
        match recurrence {
            RecurrenceType::Daily => base + chrono::Duration::days(1),
            RecurrenceType::Weekly => base + chrono::Duration::days(7),
            RecurrenceType::Monthly => base.checked_add_months(Months::new(1)).unwrap_or(base + chrono::Duration::days(30)),
            RecurrenceType::Yearly => base.checked_add_months(Months::new(12)).unwrap_or(base + chrono::Duration::days(365)),
        }
    }

    // Helper function to parse due date inputs, including today, tomorrow, and 'in X days'.
    fn parse_due_date_input(&self, input: &str) -> Option<DateTime<Utc>> {
        let s = input.trim().to_lowercase();
        if s.is_empty() {
            return None;
        }

        if s == "today" {
            let today = Utc::now().date_naive();
            Some(DateTime::<Utc>::from_naive_utc_and_offset(
                today.and_hms_opt(12, 0, 0).unwrap(),
                Utc,
            ))
        } else if s == "tomorrow" {
            let tomorrow = Utc::now().date_naive() + chrono::Duration::days(1);
            Some(DateTime::<Utc>::from_naive_utc_and_offset(
                tomorrow.and_hms_opt(12, 0, 0).unwrap(),
                Utc,
            ))
        } else if s.starts_with("in ") && s.ends_with(" days") {
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.len() == 3 {
                if let Ok(days) = parts[1].parse::<i64>() {
                    let target = Utc::now().date_naive() + chrono::Duration::days(days);
                    return Some(DateTime::<Utc>::from_naive_utc_and_offset(
                        target.and_hms_opt(12, 0, 0).unwrap(),
                        Utc,
                    ));
                }
            }
            None
        } else {
            // Fallback to strict YYYY-MM-DD format
            if let Ok(naive) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
                Some(DateTime::<Utc>::from_naive_utc_and_offset(
                    naive.and_hms_opt(12, 0, 0).unwrap(),
                    Utc,
                ))
            } else {
                None
            }
        }
    }

    // RPG Progression System helpers

    pub fn check_new_day(&mut self) -> Result<()> {
        let today = chrono::Local::now().date_naive();

        let mut streak = self.db.get_streak()?;

        let existing_adventures = self.db.get_daily_adventures()?;
        let needs_regeneration =
            existing_adventures.is_empty() || existing_adventures[0].created_date != today;

        if needs_regeneration {
            self.db.clear_daily_adventures()?;
            let new_quests = DailyAdventure::generate_daily_quests(today);
            for q in new_quests {
                self.db.insert_daily_adventure(&q)?;
            }

            let mut tree = self.db.get_zen_tree()?;
            tree.water_today = 0;
            tree.last_watered = None;

            if let Some(last_day) = streak.last_active_day {
                if today - last_day > chrono::Duration::days(1) {
                    let missed_days = (today - last_day).num_days() - 1;
                    if missed_days > 0 {
                        tree.health = (tree.health - missed_days as i32).max(10);
                        self.notifications.push(Notification::info(format!(
                                "You missed {} days. Tree health declined to {}%",
                                missed_days, tree.health
                            )));
                    }
                    streak.current_streak = 0;
                    self.db.update_streak(&streak)?;
                }
            }
            self.db.update_zen_tree(&tree)?;
        }

        Ok(())
    }

    pub fn is_watering_allowed_at(&self, now_local: chrono::DateTime<chrono::Local>) -> Result<Result<(), String>> {
        let tree = self.db.get_zen_tree()?;
        if tree.water_today >= 2 {
            return Ok(Err("Your tree is already fully watered for today!".to_string()));
        }
        
        let today = now_local.date_naive();
        let current_hour = now_local.hour();
        let is_morning = current_hour < 12;
        
        if tree.water_today > 0 {
            if let Some(last_utc) = tree.last_watered {
                let last_local = last_utc.with_timezone(&chrono::Local);
                if last_local.date_naive() == today {
                    let last_hour = last_local.hour();
                    let last_was_morning = last_hour < 12;

                    if is_morning && last_was_morning {
                        return Ok(Err("You have already watered the tree this morning. Please wait until afternoon!".to_string()));
                    }

                    if !is_morning && !last_was_morning {
                        return Ok(Err("You have already watered the tree this afternoon. Please wait until tomorrow morning!".to_string()));
                    }
                }
            }
        }
        Ok(Ok(()))
    }

    pub fn is_watering_allowed(&self) -> Result<Result<(), String>> {
        self.is_watering_allowed_at(chrono::Local::now())
    }

    pub fn water_tree_at(&mut self, now_local: chrono::DateTime<chrono::Local>) -> Result<()> {
        match self.is_watering_allowed_at(now_local)? {
            Ok(_) => {},
            Err(msg) => {
                self.notifications.push(Notification::info(msg));
                return Ok(());
            }
        }

        let mut tree = self.db.get_zen_tree()?;
        // Apply seasonal tree growth bonus (Festival of Growth in Spring)
        let amount = if crate::models::Season::current() == crate::models::Season::Spring {
            2
        } else {
            1
        };
        tree.water_today += 1;
        tree.growth += amount;
        tree.last_watered = Some(now_local.with_timezone(&Utc));

        self.notifications.push(Notification::info(format!(
                "Tree Watered! Growth +{} (Today: {}/2)",
                amount, tree.water_today
            )));

        self.db.update_zen_tree(&tree)?;
        let _ = self.db.increment_tree_waterings();
        self.push_great_chronicle_async("TreeWatering", "watered the Zen Tree.", true);

        self.update_daily_adventure_progress("water_tree", 1)?;

        // Stage 6 quest progress increment
        self.increment_quest_progress(50, 1)?;
        self.simulate_memory_fragment_unlock("zen_water")?;

        self.check_tree_evolution(&mut tree)?;
        self.reload_data()?;
        Ok(())
    }

    pub fn water_tree(&mut self) -> Result<()> {
        self.water_tree_at(chrono::Local::now())
    }

    pub fn path_suggestions(input: &str) -> Vec<String> {
        let expanded = if input.starts_with('~') {
            let home = std::env::var("HOME").unwrap_or_default();
            input.replacen('~', &home, 1)
        } else {
            input.to_string()
        };

        let path = std::path::Path::new(&expanded);
        let (dir, prefix): (std::path::PathBuf, &str) =
            if expanded.ends_with(std::path::MAIN_SEPARATOR) || expanded.is_empty() {
                (path.to_path_buf(), "")
            } else {
                let parent = path.parent().unwrap_or(std::path::Path::new("/"));
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                (parent.to_path_buf(), name)
            };

        let Ok(entries) = std::fs::read_dir(&dir) else {
            return vec![];
        };

        let mut results: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| !n.starts_with('.') && n.to_lowercase().starts_with(&prefix.to_lowercase()))
                    .unwrap_or(false)
            })
            .map(|e| {
                let p = e.path().to_string_lossy().to_string();
                if input.starts_with('~') {
                    let home = std::env::var("HOME").unwrap_or_default();
                    format!("~{}", &p[home.len()..])
                } else {
                    p
                }
            })
            .collect();

        results.sort();
        results.truncate(8);
        results
    }

    pub fn grow_tree(&mut self, amount: i32) -> Result<()> {
        let mut tree = self.db.get_zen_tree()?;
        let mut final_amount = amount;
        if crate::models::Season::current() == crate::models::Season::Spring {
            final_amount = (final_amount as f64 * 1.2).round() as i32;
            if final_amount == amount && amount > 0 {
                final_amount += 1;
            }
        }
        tree.growth += final_amount;
        self.db.update_zen_tree(&tree)?;

        self.notifications.push(Notification::info(format!("Zen Tree Growth +{}!", final_amount)));

        self.check_tree_evolution(&mut tree)?;
        self.reload_data()?;
        Ok(())
    }

    fn check_tree_evolution(&mut self, tree: &mut ZenTree) -> Result<()> {
        let old_stage = tree.stage;
        let new_stage = match tree.growth {
            0..10 => 1,
            10..25 => 2,
            25..50 => 3,
            50..100 => 4,
            100..200 => 5,
            200..350 => 6,
            _ => 7,
        };

        if new_stage > old_stage {
            tree.stage = new_stage;
            self.db.update_zen_tree(tree)?;

            // Write chronicle entry for tree evolution
            if let Some(ref u) = self.user {
                let day_number = (Utc::now() - u.created_at).num_days() as i32 + 1;
                self.db.add_chronicle_entry(
                    day_number,
                    &format!(
                        "Zen Tree evolved to Stage {}: {}.",
                        new_stage,
                        tree.stage_name()
                    ),
                )?;
            }
            self.push_great_chronicle_async(
                "ZenTree",
                &format!("reached Zen Tree Stage {}: {}.", new_stage, tree.stage_name()),
                true,
            );

            self.notifications.push(Notification::info(format!(
                    "Zen Tree Evolved! Stage {}: {}",
                    new_stage,
                    tree.stage_name()
                )));

            if new_stage == 5 {
                self.unlock_achievement("ancient_gardener")?;
            }

            if new_stage == 7 {
                self.trigger_celebration(
                    "SACRED WORLD TREE EVOLUTION!",
                    "Your Zen Tree has grown to Stage 7: The World Tree!\nIts canopy now encompasses the entire workspace, feeding on your focus.",
                    "TREE"
                );
            }
        }
        Ok(())
    }

    pub fn update_daily_adventure_progress(&mut self, quest_type: &str, amount: i32) -> Result<()> {
        let mut advs = self.db.get_daily_adventures()?;
        let mut completed_any = false;
        let was_all_completed = advs.iter().all(|a| a.completed);

        for adv in &mut advs {
            if adv.quest_type == quest_type && !adv.completed {
                adv.current_count = (adv.current_count + amount).min(adv.target_count);
                if adv.current_count >= adv.target_count {
                    adv.completed = true;
                    completed_any = true;

                    self.notifications.push(Notification::info(format!(
                            "Daily Adventure Complete: {}! (+75 XP, +5 Growth)",
                            adv.title
                        )));

                    if let Some(ref mut u) = self.user {
                        let xp_service = XPService::new(&self.db);
                        let leveled_up =
                            xp_service.grant_xp(u, &format!("Daily Quest: {}", adv.title), 75)?;
                        if leveled_up {
                            self.notifications.push(Notification::info(format!("LEVEL UP! You reached Level {}!", u.level)));
                        }
                    }

                    let mut tree = self.db.get_zen_tree()?;
                    tree.growth += 5;
                    self.check_tree_evolution(&mut tree)?;
                    self.db.update_zen_tree(&tree)?;
                    self.simulate_memory_fragment_unlock("daily_adventure")?;
                    self.apply_class_passive("daily_adventure_complete", 0)?;
                }
                self.db.update_daily_adventure(adv)?;
            }
        }

        if completed_any {
            let new_advs = self.db.get_daily_adventures()?;
            let is_all_completed = new_advs.iter().all(|a| a.completed);
            if is_all_completed && !was_all_completed {
                self.notifications.push(Notification::info("Quest Chain Completed! (+150 XP Bonus)".to_string()));

                if let Some(ref mut u) = self.user {
                    let xp_service = XPService::new(&self.db);
                    let leveled_up = xp_service.grant_xp(u, "Quest Chain Completed", 150)?;
                    if leveled_up {
                        self.notifications.push(Notification::info(format!("LEVEL UP! You reached Level {}!", u.level)));
                    }
                }
                self.apply_class_passive("daily_adventure_chain", 0)?;
            }
        }

        self.reload_data()?;
        Ok(())
    }

    pub fn complete_productive_action(&mut self) -> Result<()> {
        let mut streak = self.db.get_streak()?;
        let today = chrono::Local::now().date_naive();
        let mut increased = false;

        match streak.last_active_day {
            Some(last_day) => {
                if last_day == today {
                    // Already active today, do nothing.
                } else if last_day == today - chrono::Duration::days(1) {
                    streak.current_streak += 1;
                    if streak.current_streak > streak.best_streak {
                        streak.best_streak = streak.current_streak;
                    }
                    streak.last_active_day = Some(today);
                    self.db.update_streak(&streak)?;
                    increased = true;
                    self.apply_class_passive("streak_maintain", 0)?;

                    self.notifications.push(Notification::info(format!("Streak increased to {} days!", streak.current_streak)));

                    if streak.current_streak % 7 == 0 {
                        self.notifications.push(Notification::info(format!(
                                "{}-Day Streak Reward! (+100 XP)",
                                streak.current_streak
                            )));
                        self.push_great_chronicle_async(
                            "Streak",
                            &format!("achieved a {}-day streak.", streak.current_streak),
                            true,
                        );
                        if let Some(ref mut u) = self.user {
                            let xp_service = XPService::new(&self.db);
                            let leveled_up = xp_service.grant_xp(u, "Streak Bonus XP", 100)?;
                            if leveled_up {
                                self.notifications.push(Notification::info(format!("LEVEL UP! You reached Level {}!", u.level)));
                            }
                        }
                    }

                    if streak.current_streak == 100 {
                        self.unlock_achievement("hundred_day_journey")?;
                    }
                } else {
                    streak.current_streak = 1;
                    streak.last_active_day = Some(today);
                    self.db.update_streak(&streak)?;
                    increased = true;

                    self.notifications.push(Notification::info("Starting a new streak! Day 1".to_string()));
                }
            }
            None => {
                streak.current_streak = 1;
                streak.best_streak = 1;
                streak.last_active_day = Some(today);
                self.db.update_streak(&streak)?;
                increased = true;

                self.notifications.push(Notification::info("First action! Day 1 streak started".to_string()));
            }
        }

        if increased {
            self.check_action_achievements()?;
        }
        self.reload_data()?;
        Ok(())
    }

    pub fn grant_xp(&mut self, event_type: &str, xp_gained: i32) -> Result<()> {
        let is_focus = event_type.contains("Focus");
        let is_task = event_type.contains("Task") || event_type.contains("Hero");
        let is_journal = event_type.contains("Journal");

        let mut final_xp = xp_gained;

        // Seasonal event bonuses (+20% XP)
        let season = crate::models::Season::current();
        if season == crate::models::Season::Summer && is_focus {
            final_xp = (final_xp as f64 * 1.2).round() as i32;
        } else if season == crate::models::Season::Autumn && is_task {
            final_xp = (final_xp as f64 * 1.2).round() as i32;
        } else if season == crate::models::Season::Winter && is_journal {
            final_xp = (final_xp as f64 * 1.2).round() as i32;
        }

        let mut level_up_info = None;
        if let Some(ref mut u) = self.user {
            let xp_service = XPService::new(&self.db);
            let leveled_up = xp_service.grant_xp(u, event_type, final_xp)?;

            if leveled_up {
                level_up_info = Some((
                    u.level,
                    (Utc::now() - u.created_at).num_days() as i32 + 1,
                    u.title().to_string(),
                    u.class.name().to_string(),
                ));
            }
        }

        if let Some((new_level, day_number, title, class_name)) = level_up_info {
            self.db.add_chronicle_entry(
                day_number,
                &format!("Reached Level {}! Claimed Title: {}.", new_level, title),
            )?;
            self.push_great_chronicle_async(
                "LevelUp",
                &format!("reached Level {}.", new_level),
                true,
            );

            self.notifications.push(Notification::info(format!("LEVEL UP! You reached Level {}!", new_level)));

            self.simulate_memory_fragment_unlock("level_up")?;

            // Activate quests up to new level
            self.db
                .activate_class_quests_up_to_level(&class_name, new_level)?;

            // Trigger major milestone celebrations
            if new_level == 25 || new_level == 50 || new_level == 75 || new_level == 100 {
                self.trigger_celebration(
                    &format!("REACHED LEVEL {}!", new_level),
                    &format!("Your dedication has elevated you to a new height of power.\nYou are now recognized as a level {} {}!", new_level, class_name),
                    "LEVEL",
                );
            }
        }

        self.reload_data()?;
        Ok(())
    }

    pub fn trigger_celebration(&mut self, title: &str, details: &str, art_type: &str) {
        let ascii_art = match art_type {
            "LEVEL" => {
                "
      /\\
     /  \\
    /____\\
   |      |
   | LEVEL|
   |  UP! |
   |______|
"
            }
            "TREE" => {
                "
     .ooo.
   .oQOOPQo.
  .oQOQOQOQo.
 .oQOQOQOQOQo.
   ||| |||
   ||| |||
"
            }
            "STREAK" => {
                "
    (  )   (  )
     \\  \\ /  /
      \\  V  /
       )   (
      /     \\
     (_______)
"
            }
            _ => {
                "
      * * * *
     * QUEST *
    * SUCCESS *
      * * * *
"
            }
        };

        self.modal_state = ModalType::Celebration {
            title: title.to_string(),
            details: details.to_string(),
            ascii_art: ascii_art.to_string(),
        };
    }

    pub fn increment_quest_progress(&mut self, quest_level: i32, amount: i32) -> Result<()> {
        if let Some(ref u) = self.user {
            let class_name = u.class.name();
            let quests = self.db.get_class_quests(class_name)?;
            if let Some(q) = quests.iter().find(|q| q.1 == quest_level) {
                // (class_name, unlock_level, name, desc, status, progress, target, reward)
                if q.4 == "Active" {
                    let new_progress = q.5 + amount;
                    self.db
                        .update_class_quest_progress(class_name, quest_level, new_progress)?;

                    if new_progress >= q.6 {
                        self.db.complete_class_quest(class_name, quest_level)?;

                        let day_number = (Utc::now() - u.created_at).num_days() as i32 + 1;
                        self.db.add_chronicle_entry(
                            day_number,
                            &format!(
                                "Accomplished the legendary quest '{}' and unlocked: {}",
                                q.2, q.7
                            ),
                        )?;

                        // Grant Quest Completion XP (100 * quest_level)
                        self.grant_xp("Quest Completed Bonus", 100 * quest_level)?;

                        // Insert unlocked custom quest lore entry into the Library
                        let entry_id = format!("quest_lore_{}", quest_level);
                        self.db.insert_custom_lore_entry(
                            &entry_id,
                            "Questline",
                            &q.2,
                            &format!(
                                "The saga of the level {} quest: '{}'.\nReward lore unlocked: {}",
                                quest_level, q.2, q.7
                            ),
                            true,
                        )?;

                        self.notifications.push(Notification::info(format!("QUEST SUCCESS: {} completed!", q.2)));
                        self.push_great_chronicle_async(
                            "ClassQuest",
                            "accomplished a class quest.",
                            true,
                        );

                        self.trigger_celebration(
                            &format!("QUEST ACCOMPLISHED: {}", q.2),
                            &format!(
                                "Victory! You completed the quest for level {}!\nReward: {}",
                                quest_level, q.7
                            ),
                            "QUEST",
                        );
                    }
                }
            }
        }
        Ok(())
    }

    pub fn check_stage6_unlocks(&mut self) -> Result<()> {
        let stats = self.db.get_statistics()?;
        let streak_obj = self.db.get_streak()?;
        let tree = self.db.get_zen_tree()?;

        let user_ref = match self.user.as_ref() {
            Some(u) => u,
            None => return Ok(()),
        };
        let day_number = (Utc::now() - user_ref.created_at).num_days() as i32 + 1;

        // --- RELICS UNLOCK CHECKS ---
        let achievements = self.db.get_achievements()?;
        let scholar_unlocked = achievements
            .iter()
            .any(|a| a.id == "scholar" && a.unlocked_at.is_some());
        if scholar_unlocked && self.db.unlock_relic("ancient_quill")? {
            self.db.add_chronicle_entry(day_number, "Acquired legendary relic: the Ancient Quill. It writes with invisible ink that glows only under moonlight.")?;
            self.notifications.push(Notification::info("Relic Unlocked: Ancient Quill!".to_string()));
            self.push_great_chronicle_async("Relic", "unlocked the Ancient Quill.", true);
        }

        let proj_master_unlocked = achievements
            .iter()
            .any(|a| a.id == "project_master" && a.unlocked_at.is_some());
        if proj_master_unlocked && self.db.unlock_relic("crystal_compass")? {
            self.db.add_chronicle_entry(day_number, "Found legendary relic: the Crystal Compass. Its needle points toward the nearest unfinished task.")?;
            self.notifications.push(Notification::info("Relic Unlocked: Crystal Compass!".to_string()));
            self.push_great_chronicle_async("Relic", "unlocked the Crystal Compass.", true);
        }

        if user_ref.level >= 50 && self.db.unlock_relic("rune_tablet")? {
            self.db.add_chronicle_entry(day_number, "Discovered legendary relic: the Rune Tablet. An ancient stone slab pulsing with tree energy.")?;
            self.notifications.push(Notification::info("Relic Unlocked: Rune Tablet!".to_string()));
            self.push_great_chronicle_async("Relic", "unlocked the Rune Tablet.", true);
        }

        if streak_obj.best_streak >= 30 && self.db.unlock_relic("explorers_map")? {
            self.db.add_chronicle_entry(day_number, "Retrieved legendary relic: the Explorer's Map. A parchment depicting shifting paths.")?;
            self.notifications.push(Notification::info("Relic Unlocked: Explorer's Map!".to_string()));
            self.push_great_chronicle_async("Relic", "unlocked the Explorer's Map.", true);
        }

        if stats.sessions_completed >= 50 && self.db.unlock_relic("clock_of_focus")? {
            self.db.add_chronicle_entry(day_number, "Secured legendary relic: the Clock of Focus. A watch that ticks slower when you concentrate.")?;
            self.notifications.push(Notification::info("Relic Unlocked: Clock of Focus!".to_string()));
            self.push_great_chronicle_async("Relic", "unlocked the Clock of Focus.", true);
        }

        // --- LEGENDARY TITLES UNLOCK CHECKS ---
        if streak_obj.best_streak >= 30 && self.db.unlock_legendary_title("relentless")? {
            self.db
                .add_chronicle_entry(day_number, "Earned Legendary Title: The Relentless.")?;
            self.notifications.push(Notification::info("Legendary Title Unlocked: The Relentless!".to_string()));
            self.push_great_chronicle_async("Legend", "entered the Hall of Legends as The Relentless.", true);
        }

        let chronicler_unlocked = achievements
            .iter()
            .any(|a| a.id == "chronicler" && a.unlocked_at.is_some());
        if (scholar_unlocked || chronicler_unlocked)
            && self.db.unlock_legendary_title("archivist")?
        {
            self.db
                .add_chronicle_entry(day_number, "Earned Legendary Title: The Archivist.")?;
            self.notifications.push(Notification::info("Legendary Title Unlocked: The Archivist!".to_string()));
            self.push_great_chronicle_async("Legend", "entered the Hall of Legends as The Archivist.", true);
        }

        let deep_worker_unlocked = achievements
            .iter()
            .any(|a| a.id == "deep_worker" && a.unlocked_at.is_some());
        if (deep_worker_unlocked || stats.sessions_completed >= 100)
            && self.db.unlock_legendary_title("focused")?
        {
            self.db
                .add_chronicle_entry(day_number, "Earned Legendary Title: The Focused.")?;
            self.notifications.push(Notification::info("Legendary Title Unlocked: The Focused!".to_string()));
            self.push_great_chronicle_async("Legend", "entered the Hall of Legends as The Focused.", true);
        }

        let master_atmosphere = achievements
            .iter()
            .any(|a| a.id == "master_atmosphere" && a.unlocked_at.is_some());
        if master_atmosphere && self.db.unlock_legendary_title("master_seasons")? {
            self.db
                .add_chronicle_entry(day_number, "Earned Legendary Title: Master of Seasons.")?;
            self.notifications.push(Notification::info("Legendary Title Unlocked: Master of Seasons!".to_string()));
            self.push_great_chronicle_async("Legend", "entered the Hall of Legends as Master of Seasons.", true);
        }

        let ancient_gardener = achievements
            .iter()
            .any(|a| a.id == "ancient_gardener" && a.unlocked_at.is_some());
        if (ancient_gardener || tree.stage >= 5)
            && self.db.unlock_legendary_title("ancient_gardener")?
        {
            self.db
                .add_chronicle_entry(day_number, "Earned Legendary Title: The Ancient Gardener.")?;
            self.notifications.push(Notification::info("Legendary Title Unlocked: The Ancient Gardener!".to_string()));
            self.push_great_chronicle_async("Legend", "entered the Hall of Legends as The Ancient Gardener.", true);
        }

        let entries = self.db.get_chronicle_entries()?;
        if entries.len() >= 50 && self.db.unlock_legendary_title("keeper_chronicles")? {
            self.db
                .add_chronicle_entry(day_number, "Earned Legendary Title: Keeper of Chronicles.")?;
            self.notifications.push(Notification::info("Legendary Title Unlocked: Keeper of Chronicles!".to_string()));
            self.push_great_chronicle_async("Legend", "entered the Hall of Legends as Keeper of Chronicles.", true);
        }

        // --- LORE LIBRARY UNLOCK CHECKS ---
        let base_class_key = match user_ref.class {
            ClassType::CodeWarlock => "warlock",
            ClassType::TaskPaladin => "paladin",
            ClassType::MindSage => "sage",
            ClassType::SystemsArchitect => "architect",
            ClassType::TimeChronomancer => "chronomancer",
            ClassType::ArchAccountant => "accountant",
        };

        for lvl in &[5, 15, 20, 30] {
            if user_ref.level >= *lvl {
                let class_key = format!("class_{}_{}", base_class_key, lvl);
                let notif_key = format!("lore_notified_{}", class_key);
                if self.db.get_setting(&notif_key)?.is_none() {
                    if self.db.unlock_lore_entry(&class_key)? {
                        let _ = self.db.set_setting(&notif_key, "1");
                        self.notifications.push(Notification::info(format!("New Class Lore Unlocked in Library! (Level {})", lvl)));
                        self.push_great_chronicle_async(
                            "ClassStory",
                            &format!("unlocked a class story at Level {}.", lvl),
                            true,
                        );
                    }
                }
            }
        }

        if user_ref.level >= 40 {
            if self.db.get_setting("lore_notified_class_council_orders")?.is_none()
                && self.db.unlock_lore_entry("class_council_orders")?
            {
                let _ = self.db.set_setting("lore_notified_class_council_orders", "1");
                self.notifications.push(Notification::info("The Council of Orders Unlocked in Library!".to_string()));
            }
        }

        for i in 1..=10 {
            let req_level = i * 10;
            if user_ref.level >= req_level {
                let world_key = format!("world_chapter_{}", i);
                let notif_key = format!("lore_notified_{}", world_key);
                if self.db.get_setting(&notif_key)?.is_none() {
                    if self.db.unlock_lore_entry(&world_key)? {
                        let _ = self.db.set_setting(&notif_key, "1");
                        self.notifications.push(Notification::info(format!("World History Chapter {} Unlocked in Library!", i)));
                        self.push_great_chronicle_async(
                            "WorldLore",
                            &format!("uncovered World History Chapter {}.", i),
                            true,
                        );
                    }
                }
            }
        }

        Ok(())
    }

    pub fn handle_library_action(&mut self) -> Result<()> {
        if self.selected_library_cat_idx != 0 {
            return Ok(());
        }
        let user_ref = match self.user.as_ref() {
            Some(u) => u,
            None => return Ok(()),
        };
        let class_name = user_ref.class.name();
        let quests = self.db.get_class_quests(class_name)?;
        if self.selected_library_item_idx >= quests.len() {
            return Ok(());
        }

        let q = &quests[self.selected_library_item_idx];
        let quest_name = q.2.clone();
        let status = q.4.clone();
        let progress = q.5;
        let target = q.6;
        let lore_reward = q.7.clone();
        let unlock_level = q.1;

        let day_number = (Utc::now() - user_ref.created_at).num_days() as i32 + 1;

        if status == "Available" {
            self.db.start_class_quest(class_name, unlock_level)?;
            self.db.add_chronicle_entry(
                day_number,
                &format!(
                    "Embarked on Class Quest: '{}' for class {}",
                    quest_name, class_name
                ),
            )?;
            self.notifications.push(Notification::info(format!("Embarked on quest: {}!", quest_name)));
            self.reload_data()?;
        } else if status == "Active" && progress >= target {
            self.db.complete_class_quest(class_name, unlock_level)?;
            self.grant_xp("Completed Class Quest", 200)?;
            self.db.add_chronicle_entry(
                day_number,
                &format!(
                    "Completed Class Quest: '{}' - Unlocked: {}",
                    quest_name, lore_reward
                ),
            )?;
            self.db.insert_custom_lore_entry(
                &format!("quest_story_{}_{}", class_name, unlock_level),
                "Class",
                &quest_name,
                &lore_reward,
                true,
            )?;

            self.trigger_celebration(
                &format!("VICTORY: {}!", quest_name),
                &format!(
                    "You have completed the trial for your class. Reward Unlocked:\n{}",
                    lore_reward
                ),
                "VICTORY",
            );

            self.notifications.push(Notification::info(format!("Quest Completed: {}!", quest_name)));
            self.check_action_achievements()?;
            self.reload_data()?;
        }
        Ok(())
    }

    pub fn perform_unified_search(&self, query: &str) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }
        let q = query.to_lowercase();
        let mut results = Vec::new();

        // 1. Search Projects
        if let Ok(projects) = self.db.get_projects() {
            for p in projects {
                if p.name.to_lowercase().contains(&q)
                    || p.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&q))
                        .unwrap_or(false)
                {
                    results.push(SearchResult {
                        result_type: SearchResultType::Project,
                        title: p.name.clone(),
                        details: p
                            .description
                            .clone()
                            .unwrap_or_else(|| "No description".to_string()),
                        project_id: Some(p.id),
                        item_id: p.id.to_string(),
                    });
                }
            }
        }

        // 2. Search Tasks
        if let Ok(tasks) = self.db.get_tasks() {
            for t in tasks {
                if t.title.to_lowercase().contains(&q)
                    || t.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&q))
                        .unwrap_or(false)
                {
                    results.push(SearchResult {
                        result_type: SearchResultType::Task,
                        title: t.title.clone(),
                        details: format!("Task (Priority: {})", t.priority.name()),
                        project_id: t.project_id,
                        item_id: t.id.to_string(),
                    });
                }
            }
        }

        // 3. Search Notes
        if let Ok(projects) = self.db.get_projects() {
            for p in &projects {
                if let Ok(notes) = self.db.get_notes_for_project(p.id) {
                    for n in notes {
                        if n.title.to_lowercase().contains(&q)
                            || n.markdown_content.to_lowercase().contains(&q)
                        {
                            results.push(SearchResult {
                                result_type: SearchResultType::Note,
                                title: n.title.clone(),
                                details: format!("Note in project '{}'", p.name),
                                project_id: Some(p.id),
                                item_id: n.id.to_string(),
                            });
                        }
                    }
                }
            }
        }

        // 4. Search Journal Entries
        if let Ok(projects) = self.db.get_projects() {
            for p in &projects {
                if let Ok(journal) = self.db.get_journal_entries_for_project(p.id) {
                    for j in journal {
                        if j.content.to_lowercase().contains(&q) {
                            results.push(SearchResult {
                                result_type: SearchResultType::JournalEntry,
                                title: format!("Journal - {}", j.entry_date),
                                details: j.content.chars().take(60).collect(),
                                project_id: Some(p.id),
                                item_id: j.id.to_string(),
                            });
                        }
                    }
                }
            }
        }

        // 5. Search Achievements
        if let Ok(achievements) = self.db.get_achievements() {
            for a in achievements {
                if a.name.to_lowercase().contains(&q) || a.description.to_lowercase().contains(&q) {
                    let status = if a.unlocked_at.is_some() {
                        "Unlocked"
                    } else {
                        "Locked"
                    };
                    results.push(SearchResult {
                        result_type: SearchResultType::Achievement,
                        title: format!("Achievement: {}", a.name),
                        details: format!("{} ({})", a.description, status),
                        project_id: None,
                        item_id: a.id.clone(),
                    });
                }
            }
        }

        // 6. Search Lore
        if let Ok(lore) = self.db.get_lore_entries() {
            for l in lore {
                if l.2.to_lowercase().contains(&q) || l.3.to_lowercase().contains(&q) {
                    results.push(SearchResult {
                        result_type: SearchResultType::Lore,
                        title: format!("Lore: {}", l.2),
                        details: format!(
                            "Category: {} - {}",
                            l.1,
                            if l.4 { "Unlocked" } else { "Locked" }
                        ),
                        project_id: None,
                        item_id: l.0.clone(),
                    });
                }
            }
        }

        // 7. Search Chronicle Entries
        if let Ok(entries) = self.db.get_chronicle_entries() {
            for e in entries {
                if e.2.to_lowercase().contains(&q) {
                    results.push(SearchResult {
                        result_type: SearchResultType::ChronicleEntry,
                        title: format!("Chronicle Day {}", e.1),
                        details: e.2.clone(),
                        project_id: None,
                        item_id: e.0.clone(),
                    });
                }
            }
        }

        results.truncate(15);
        results
    }

    pub fn navigate_to_search_result(&mut self, result: &SearchResult) -> Result<()> {
        self.modal_state = ModalType::None;
        match result.result_type {
            SearchResultType::Project => {
                if let Some(id) = result.project_id {
                    self.active_project_id = Some(id);
                    self.active_screen = ActiveScreen::Workspace;
                    self.workspace_tab_idx = 0;
                }
            }
            SearchResultType::Task => {
                if let Some(p_id) = result.project_id {
                    self.active_project_id = Some(p_id);
                    self.active_screen = ActiveScreen::Workspace;
                    self.workspace_tab_idx = 0;
                    let tasks = self.db.get_tasks_for_project(p_id).unwrap_or_default();
                    if let Ok(task_uuid) = uuid::Uuid::parse_str(&result.item_id) {
                        if let Some(pos) = tasks.iter().position(|t| t.id == task_uuid) {
                            self.selected_task_idx = pos;
                        }
                    }
                }
            }
            SearchResultType::Note => {
                if let Some(p_id) = result.project_id {
                    self.active_project_id = Some(p_id);
                    self.active_screen = ActiveScreen::Workspace;
                    self.workspace_tab_idx = 1;
                    let notes = self.db.get_notes_for_project(p_id).unwrap_or_default();
                    if let Ok(note_uuid) = uuid::Uuid::parse_str(&result.item_id) {
                        if let Some(pos) = notes.iter().position(|n| n.id == note_uuid) {
                            self.selected_note_idx = pos;
                        }
                    }
                }
            }
            SearchResultType::JournalEntry => {
                if let Some(p_id) = result.project_id {
                    self.active_project_id = Some(p_id);
                    self.active_screen = ActiveScreen::Workspace;
                    self.workspace_tab_idx = 2;
                    let journal = self
                        .db
                        .get_journal_entries_for_project(p_id)
                        .unwrap_or_default();
                    if let Ok(journal_uuid) = uuid::Uuid::parse_str(&result.item_id) {
                        if let Some(pos) = journal.iter().position(|j| j.id == journal_uuid) {
                            self.selected_journal_idx = pos;
                        }
                    }
                }
            }
            SearchResultType::Achievement => {
                self.active_screen = ActiveScreen::Character;
                self.active_tab_idx = 2;
            }
            SearchResultType::Lore => {
                self.active_screen = ActiveScreen::Library;
                self.active_tab_idx = 4;
            }
            SearchResultType::ChronicleEntry => {
                self.active_screen = ActiveScreen::Character;
                self.active_tab_idx = 2;
                if let Ok(entries) = self.db.get_chronicle_entries() {
                    if let Some(pos) = entries.iter().position(|e| e.0 == result.item_id) {
                        self.selected_chronicle_idx = pos;
                    }
                }
            }
        }
        if self.active_screen == ActiveScreen::Workspace {
            self.reload_data()?;
        }
        Ok(())
    }

fn parse_companion_last_seen(last_seen_raw: &str) -> (bool, String) {
    if last_seen_raw.is_empty() {
        return (false, "Never".to_string());
    }
    // MySQL TIMESTAMP format: "YYYY-MM-DD HH:MM:SS"
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(last_seen_raw, "%Y-%m-%d %H:%M:%S") {
        let dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc);
        let diff = chrono::Utc::now().signed_duration_since(dt);
        let online = diff.num_minutes() < 15;
        let display = if diff.num_minutes() < 1 {
            "Just now".to_string()
        } else if diff.num_hours() < 1 {
            format!("{} min ago", diff.num_minutes())
        } else if diff.num_days() < 1 {
            format!("{} hr ago", diff.num_hours())
        } else {
            format!("{} days ago", diff.num_days())
        };
        (online, display)
    } else {
        (false, last_seen_raw.to_string())
    }
}

fn fuzzy_match(query: &str, target: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let q_lower = query.to_lowercase();
    let t_lower = target.to_lowercase();

    // Check if the query is a substring
    if let Some(idx) = t_lower.find(&q_lower) {
        let mut score = 1000 - (idx as i32);
        if idx == 0 {
            score += 500; // starts with query bonus
        }
        return Some(score);
    }

    // Fuzzy sequence matching
    let mut q_chars = q_lower.chars().peekable();
    let mut score = 0;
    let mut last_match_idx: Option<usize> = None;

    for (idx, t_char) in t_lower.chars().enumerate() {
        if let Some(&q_char) = q_chars.peek() {
            if t_char == q_char {
                q_chars.next();
                
                // Add points for matching
                score += 10;

                // Consecutive match bonus
                if let Some(prev) = last_match_idx {
                    if idx == prev + 1 {
                        score += 30;
                    } else {
                        score -= (idx - prev) as i32; // gap penalty
                    }
                } else {
                    // First character match penalty based on how far it is from the start
                    score -= idx as i32;
                }

                // Word boundary bonus: if target character is preceded by a space/delimiter
                if idx == 0 || t_lower.chars().nth(idx - 1).map(|c| !c.is_alphanumeric()).unwrap_or(false) {
                    score += 50;
                }

                last_match_idx = Some(idx);
            }
        } else {
            break;
        }
    }

    if q_chars.peek().is_none() {
        Some(score)
    } else {
        None
    }
}

    pub fn get_available_command_actions(&self, filter: &str) -> Vec<CommandAction> {
        let all_actions = vec![
            CommandAction {
                name: "Open Dashboard",
                description: "Navigate to your Dashboard",
                shortcut: "D / 1",
                id: "open_dashboard",
            },
            CommandAction {
                name: "Open Projects",
                description: "Navigate to Projects list",
                shortcut: "P / 2",
                id: "open_projects",
            },
            CommandAction {
                name: "Open Character",
                description: "Navigate to Character screen",
                shortcut: "H / 3",
                id: "open_character",
            },

            CommandAction {
                name: "Open Focus",
                description: "Navigate to Focus screen",
                shortcut: "F (from Projects)",
                id: "open_focus",
            },
            CommandAction {
                name: "Open Sync",
                description: "Navigate to Sync Settings",
                shortcut: "S / 6",
                id: "open_sync",
            },
            CommandAction {
                name: "Open Statistics",
                description: "Navigate to Statistics screen",
                shortcut: "G",
                id: "open_stats",
            },
            CommandAction {
                name: "Open Fellowship",
                description: "Navigate to Fellowship screen",
                shortcut: "S (from Projects)",
                id: "open_fellowship",
            },
            CommandAction {
                name: "Open Library",
                description: "Navigate to Library screen",
                shortcut: "L / 4",
                id: "open_library",
            },
            CommandAction {
                name: "Open Archive",
                description: "Navigate to Archive screen",
                shortcut: "A (from Projects)",
                id: "open_archive",
            },
            CommandAction {
                name: "Search Everywhere",
                description: "Search across quests, notes, and achievements",
                shortcut: "/",
                id: "search_integration",
            },
            CommandAction {
                name: "Create Task",
                description: "Create a new task, with project picker if needed",
                shortcut: "Ctrl+T",
                id: "create_task",
            },
            CommandAction {
                name: "Create Note",
                description: "Create a new note, with project picker if needed",
                shortcut: "Ctrl+N",
                id: "create_note",
            },
            CommandAction {
                name: "Start Focus Session",
                description: "Begin a pomodoro focus session to gain XP",
                shortcut: "F (from Projects)",
                id: "start_focus",
            },
            CommandAction {
                name: "Open Project",
                description: "Browse and open your active projects list",
                shortcut: "2",
                id: "open_project",
            },
            CommandAction {
                name: "Water Tree",
                description: "Nurture your Zen Tree with a drop of water",
                shortcut: "w (Dashboard)",
                id: "water_tree",
            },
            CommandAction {
                name: "Sync Devices",
                description: "Force trigger peer-to-peer cloud synchronization",
                shortcut: "Ctrl+S",
                id: "sync",
            },
            CommandAction {
                name: "View Achievements",
                description: "Check all unlocked titles and stats",
                shortcut: "H / 3",
                id: "view_achievements",
            },
            CommandAction {
                name: "Open Music",
                description: "Navigate to Soundscapes & music player",
                shortcut: "M / 5",
                id: "open_music",
            },
        ];

        if filter.is_empty() {
            all_actions
        } else {
            let mut scored_actions: Vec<(i32, CommandAction)> = all_actions
                .into_iter()
                .filter_map(|action| {
                    let name_score = Self::fuzzy_match(filter, action.name);
                    let desc_score = Self::fuzzy_match(filter, action.description);
                    if let Some(ns) = name_score {
                        let mut score = ns + 100;
                        if action.id.starts_with("open_") {
                            score += 1000;
                        }
                        Some((score, action))
                    } else if let Some(ds) = desc_score {
                        let mut score = ds;
                        if action.id.starts_with("open_") {
                            score += 1000;
                        }
                        Some((score, action))
                    } else {
                        None
                    }
                })
                .collect();
            scored_actions.sort_by(|a, b| b.0.cmp(&a.0));
            scored_actions.into_iter().map(|(_, action)| action).collect()
        }
    }

    pub fn execute_command_action(&mut self, action_id: &str) -> Result<()> {
        self.modal_state = ModalType::None;
        match action_id {
            "open_dashboard" => {
                self.active_screen = ActiveScreen::Dashboard;
                self.active_tab_idx = 0;
            }
            "open_projects" => {
                self.active_screen = ActiveScreen::Projects;
                self.active_tab_idx = 1;
            }
            "open_character" => {
                self.active_screen = ActiveScreen::Character;
                self.active_tab_idx = 2;
            }

            "open_focus" => {
                self.active_screen = ActiveScreen::Focus;
                self.active_tab_idx = 6;
            }
            "open_sync" => {
                self.active_screen = ActiveScreen::SyncSettings;
                self.active_tab_idx = 12;
            }
            "open_stats" => {
                self.active_screen = ActiveScreen::Legends;
                self.active_tab_idx = 5;
            }
            "open_fellowship" => {
                self.active_screen = ActiveScreen::Fellowship;
                self.active_tab_idx = 8;
                self.pull_invitations_async();
            }
            "open_library" => {
                self.active_screen = ActiveScreen::Library;
                self.active_tab_idx = 4;
            }
            "open_archive" => {
                self.active_screen = ActiveScreen::Archive;
                self.active_tab_idx = 10;
            }
            "search_integration" => {
                self.modal_state = ModalType::SearchEverywhere {
                    query: String::new(),
                    selected_idx: 0,
                    results: Vec::new(),
                };
            }
            "create_task" => {
                if self.active_project_id.is_some() {
                    self.active_screen = ActiveScreen::Workspace;
                    self.workspace_tab_idx = 0;
                    self.modal_state = ModalType::NewTask {
                        title: String::new(),
                        desc: String::new(),
                        desc_cursor: 0,
                        priority: TaskPriority::Medium,
                        due_date_type: DueDateType::InDays,
                        due_date_val: "1".to_string(),
                        focus_idx: 0,
                        parent_task_id: None,
                        recurrence: None,
                    };
                } else {
                    self.modal_state = ModalType::SelectProjectForAction {
                        action_id: "create_task",
                        selected_idx: 0,
                    };
                }
            }
            "create_note" => {
                if self.active_project_id.is_some() {
                    self.active_screen = ActiveScreen::Workspace;
                    self.workspace_tab_idx = 1;
                    self.modal_state = ModalType::NewJournalEntry {
                        content: String::new(),
                    };
                } else {
                    self.modal_state = ModalType::SelectProjectForAction {
                        action_id: "create_note",
                        selected_idx: 0,
                    };
                }
            }
            "start_focus" => {
                self.active_screen = ActiveScreen::Focus;
            }
            "open_project" => {
                self.active_screen = ActiveScreen::Projects;
                self.active_tab_idx = 1;
            }
            "water_tree" => {
                self.active_screen = ActiveScreen::Dashboard;
                self.active_tab_idx = 0;
                self.water_tree()?;
            }
            "sync" => {
                self.active_screen = ActiveScreen::SyncSettings;
                self.active_tab_idx = 12;
                if self.config.sync_enabled {
                    self.start_forced_sync();
                } else {
                    let _ = self.trigger_sync();
                }
            }
            "view_achievements" => {
                self.active_screen = ActiveScreen::Character;
                self.active_tab_idx = 2;
            }
            "open_music" => {
                self.active_screen = ActiveScreen::Soundscapes;
                self.active_tab_idx = 7;
            }
            _ => {}
        }
        Ok(())
    }

    // Applies class-specific passive XP bonus for a given trigger.
    // word_count is used for note-length checks; pass 0 for non-note triggers.
    pub fn apply_class_passive(&mut self, trigger: &str, word_count: usize) -> Result<()> {
        let class = match self.user.as_ref() {
            Some(u) => u.class,
            None => return Ok(()),
        };

        let (bonus_xp, label): (i32, &str) = match (class, trigger) {
            // Task Paladin
            (ClassType::TaskPaladin, "task_complete") => (5, "Passive: Paladin Task Zeal"),
            (ClassType::TaskPaladin, "high_priority_task") => (10, "Passive: Paladin Priority Strike"),
            (ClassType::TaskPaladin, "daily_adventure_chain") => (15, "Passive: Paladin Oath Fulfilled"),

            // Code Warlock
            (ClassType::CodeWarlock, "note_create") => (5, "Passive: Warlock Scroll Conjured"),
            (ClassType::CodeWarlock, "note_edit") => (5, "Passive: Warlock Arcane Edit"),
            (ClassType::CodeWarlock, "project_create") => (15, "Passive: Warlock System Summoned"),
            (ClassType::CodeWarlock, "sync_complete") => (2, "Passive: Warlock Daemon Sync"),

            // Mind Sage
            (ClassType::MindSage, "note_create") if word_count > 500 => (10, "Passive: Sage Deep Knowledge"),
            (ClassType::MindSage, "note_edit") if word_count > 500 => (10, "Passive: Sage Deep Knowledge"),
            (ClassType::MindSage, "journal_create") => (5, "Passive: Sage Wisdom Recorded"),
            (ClassType::MindSage, "memory_fragment") => (5, "Passive: Sage Fragment Insight"),

            // Systems Architect
            (ClassType::SystemsArchitect, "project_create") => (10, "Passive: Architect Blueprint Drawn"),
            (ClassType::SystemsArchitect, "project_archive") => (15, "Passive: Architect Order Sealed"),
            (ClassType::SystemsArchitect, "project_restore") => (5, "Passive: Architect System Restored"),

            // Time Chronomancer
            (ClassType::TimeChronomancer, "focus_session") => (10, "Passive: Chronomancer Hour Mastered"),
            (ClassType::TimeChronomancer, "focus_pomodoro") => (25, "Passive: Chronomancer Cycle Complete"),
            (ClassType::TimeChronomancer, "daily_adventure_complete") => (5, "Passive: Chronomancer Time Invested"),

            // Arch Accountant (event-specific; global bonus is in xp.rs)
            (ClassType::ArchAccountant, "daily_adventure_chain") => (10, "Passive: Accountant Full Ledger"),
            (ClassType::ArchAccountant, "streak_maintain") => (5, "Passive: Accountant Compound Returns"),

            _ => return Ok(()),
        };

        if let Some(ref mut u) = self.user {
            let xp_service = XPService::new(&self.db);
            xp_service.grant_xp(u, label, bonus_xp)?;
        }

        // Warlock Daemon Sync XP is shown in the sync footer instead of a notification
        if class == ClassType::CodeWarlock && trigger == "sync_complete" {
            self.last_sync_warlock_xp = bonus_xp;
            return Ok(());
        }

        self.notifications.push(Notification::info(format!("{} +{} XP", label.trim_start_matches("Passive: "), bonus_xp)));

        Ok(())
    }

    pub fn simulate_memory_fragment_unlock(&mut self, trigger: &str) -> Result<()> {
        let multiplier = if self.user.as_ref().map(|u| u.class == ClassType::MindSage).unwrap_or(false) {
            1.1
        } else {
            1.0
        };
        if let Some((id, title)) = self.db.discover_memory_fragment(trigger, multiplier)? {
            let number = id
                .strip_prefix("memory_")
                .map(|n| format!("#{}", n))
                .unwrap_or_else(|| "???".to_string());
            let rarity = match id.as_str() {
                "memory_999" => "Legendary",
                "memory_077" | "memory_112" | "memory_144" | "memory_188" => "Rare",
                _ => "Common",
            };
            let attribution = title
                .splitn(2, " \u{2014} ")
                .nth(1)
                .unwrap_or(&title)
                .to_string();

            self.fragment_notification = Some(FragmentAlert {
                number,
                rarity: rarity.to_string(),
                attribution,
                shown_at: std::time::Instant::now(),
            });

            if let Some(ref u) = self.user {
                let day_number = (Utc::now() - u.created_at).num_days() as i32 + 1;
                self.db.add_chronicle_entry(
                    day_number,
                    &format!(
                        "Memory Fragment {} discovered — a forgotten echo recorded in the Chronicle.",
                        id
                    ),
                )?;
            }
            self.push_great_chronicle_async(
                "MemoryFragment",
                &format!("recovered a {} Memory Fragment.", rarity),
                true,
            );
            self.apply_class_passive("memory_fragment", 0)?;
        }
        Ok(())
    }

    pub fn trigger_ambient_particles(&mut self) {
        self.ambient_particles_ticks_remaining = 60; // 3 seconds at 50ms per tick
    }

    pub fn tick_prologue(&mut self) {
        if self.active_screen != ActiveScreen::Prologue {
            return;
        }
        // Audio warm-up delay: hold the typewriter until the sink is ready
        if self.prologue_delay_ticks > 0 {
            self.prologue_delay_ticks -= 1;
            return;
        }
        use crate::screens::prologue::{page_lines, LineKind};
        let lines = page_lines(self.prologue_page);
        let total = lines.len();

        if self.prologue_line_idx >= total {
            return; // page complete, waiting for Space
        }

        let sl = &lines[self.prologue_line_idx];

        // Instant lines (empty, separator) — advance without typing
        if sl.instant || sl.text.is_empty() {
            self.prologue_line_idx += 1;
            self.prologue_char_in_line = 0;
            return;
        }

        // Dramatic lines type slightly slower for effect
        let chars_per_tick: usize = match sl.kind {
            LineKind::Dramatic => 1,
            _ => 2,
        };

        let line_len = sl.text.chars().count();
        self.prologue_char_in_line = (self.prologue_char_in_line + chars_per_tick).min(line_len);

        if self.prologue_char_in_line >= line_len {
            self.prologue_line_idx += 1;
            self.prologue_char_in_line = 0;
        }
    }

    pub fn tick_particles(&mut self) {
        if !self.ambient_effects_enabled || self.active_ambient_effect == 0 {
            self.ambient_particles.clear();
            return;
        }

        let should_spawn = self.ambient_particles_ticks_remaining > 0;
        if self.ambient_particles_ticks_remaining > 0 {
            self.ambient_particles_ticks_remaining -= 1;
        }

        use rand::prelude::SliceRandom;
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let max_particles = match self.active_ambient_effect {
            3 => 40,
            _ => 20,
        };

        if should_spawn && self.ambient_particles.len() < max_particles && rng.gen_bool(0.3) {
            let symbol = match self.active_ambient_effect {
                1 => *['*', 'o', '~', 's'].choose(&mut rng).unwrap_or(&'*'),
                2 => *['.', '*', '+'].choose(&mut rng).unwrap_or(&'.'),
                3 => '|',
                4 => *['.', '*', '+'].choose(&mut rng).unwrap_or(&'.'),
                5 => *['A', '7', '@', '#', '!', 'Z', '$', '%']
                    .choose(&mut rng)
                    .unwrap_or(&'*'),
                _ => '*',
            };

            let color = match self.active_ambient_effect {
                1 => *[
                    ratatui::style::Color::Rgb(249, 115, 22),
                    ratatui::style::Color::Rgb(163, 230, 53),
                    ratatui::style::Color::Rgb(217, 119, 6),
                ]
                .choose(&mut rng)
                .unwrap_or(&ratatui::style::Color::Green),
                2 => ratatui::style::Color::Yellow,
                3 => ratatui::style::Color::Rgb(14, 165, 233),
                4 => ratatui::style::Color::White,
                5 => ratatui::style::Color::Rgb(168, 85, 247),
                _ => ratatui::style::Color::White,
            };

            let speed = match self.active_ambient_effect {
                1 => rng.gen_range(0.1..0.25),
                2 => rng.gen_range(0.01..0.03),
                3 => rng.gen_range(0.8..1.3),
                4 => rng.gen_range(0.08..0.18),
                5 => rng.gen_range(0.15..0.35),
                _ => 0.1,
            };

            self.ambient_particles.push(Particle {
                x: rng.gen_range(0..180),
                y: 0.0,
                speed,
                symbol,
                color,
            });
        }

        let mut active_particles = Vec::new();
        for mut p in self.ambient_particles.drain(..) {
            p.y += p.speed;

            if self.active_ambient_effect == 1 || self.active_ambient_effect == 4 {
                let drift = rng.gen_range(-1..=1);
                p.x = (p.x as i32 + drift).max(0) as u16;
            }

            if p.y < 45.0 {
                active_particles.push(p);
            }
        }
        self.ambient_particles = active_particles;
    }

    fn check_action_achievements(&mut self) -> Result<()> {
        let stats = self.db.get_statistics()?;
        if stats.tasks_completed >= 1 {
            self.unlock_achievement("first_quest")?;
        }
        if stats.notes_created >= 25 {
            self.unlock_achievement("scholar")?;
        }
        if stats.journal_entries >= 50 {
            self.unlock_achievement("chronicler")?;
        }
        if stats.projects_created >= 10 {
            self.unlock_achievement("project_master")?;
        }

        let silent_count = self.db.count_focus_sessions_with_soundscape(&["Silent"])?;
        if silent_count >= 25 {
            self.unlock_achievement("silent_monk")?;
        }
        let forest_count = self
            .db
            .count_focus_sessions_with_soundscape(&["Forest Sounds"])?;
        if forest_count >= 50 {
            self.unlock_achievement("forest_wanderer")?;
        }
        let rain_count = self
            .db
            .count_focus_sessions_with_soundscape(&["Rain Sounds"])?;
        if rain_count >= 50 {
            self.unlock_achievement("rain_listener")?;
        }
        let unique_soundscapes = self.db.count_unique_soundscapes_used()?;
        if unique_soundscapes >= 8 {
            self.unlock_achievement("master_atmosphere")?;
        }

        // Stage 6 milestone check: 1000 tasks
        if stats.tasks_completed >= 1000 {
            let key = "celebrated_1000_tasks";
            if self.db.get_setting(key)?.is_none() {
                self.db.set_setting(key, "true")?;
                self.trigger_celebration(
                    "1,000 TASKS PURIFIED!",
                    "Your relentless crusade against procrastination has resolved 1,000 quests.\nThe realm is forever in your debt.",
                    "SUCCESS"
                );
            }
        }

        // Codex (note folder) achievements
        let codex_count = self.db.count_codices().unwrap_or(0);
        if codex_count >= 3 {
            self.unlock_achievement("archivist")?;
        }
        if codex_count >= 10 {
            self.unlock_achievement("grand_archivist")?;
        }

        self.check_stage6_unlocks()?;
        Ok(())
    }

    pub fn unlock_achievement(&mut self, id: &str) -> Result<()> {
        let achievements = self.db.get_achievements()?;
        if let Some(ach) = achievements.iter().find(|a| a.id == id) {
            if ach.unlocked_at.is_none() {
                self.db.unlock_achievement(id)?;
                let user_ref = self.user.as_ref().unwrap();
                let day_number = (Utc::now() - user_ref.created_at).num_days() as i32 + 1;
                self.db.add_chronicle_entry(
                    day_number,
                    &format!("Unlocked Achievement: {} - {}", ach.name, ach.description),
                )?;

                self.notifications.push(Notification::info(format!("ACHIEVEMENT UNLOCKED: {} ({})", ach.name, ach.description)));
                self.push_great_chronicle_async(
                    "Achievement",
                    "unlocked an achievement.",
                    true,
                );
            }
        }
        Ok(())
    }

    pub fn tick_auto_sync(&mut self) -> Result<()> {
        if !self.auto_sync {
            return Ok(());
        }

        // Apply result from completed background sync
        if self.sync_in_progress {
            let result = self.sync_result.try_lock().ok().and_then(|mut g| g.take());
            if let Some(bg) = result {
                self.sync_in_progress = false;
                match bg.error {
                    Some(e) => {
                        self.sync_failure_count = self.sync_failure_count.saturating_add(1);
                        self.sync_status_msg = format!("Sync failed: {}", e);
                        self.last_sync_status_time = Some(std::time::Instant::now());
                        let _ = self.reload_data();
                    }
                    None => {
                        self.sync_failure_count = 0;
                        self.sync_conflicts = bg.conflicts;
                        self.last_sync_warlock_xp = 0;
                        self.sync_status_msg = format!("↑{} pushed  ↓{} pulled", bg.pushed, bg.pulled);
                        self.last_sync_status_time = Some(std::time::Instant::now());
                        self.apply_class_passive("sync_complete", 0)?;
                        self.reload_data()?;
                        self.load_chapter_progress_from_cache();
                        self.great_chronicle_entries =
                            self.db.get_global_chronicle_entries().unwrap_or_default();
                    }
                }
            }
            return Ok(());
        }

        let debounce = std::time::Duration::from_secs(2);
        let interval = std::time::Duration::from_secs(30);
        let mutation_ready = self.last_mutation
            .map(|t| t.elapsed() >= debounce)
            .unwrap_or(false);

        // Limpieza diaria del sync_log — borramos entradas synced=1 de más de 30 días
        let last_cleanup = self.db.get_setting("last_sync_cleanup").ok().flatten()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.with_timezone(&chrono::Utc));
        let cleanup_due = last_cleanup.map(|d| (chrono::Utc::now() - d).num_seconds() > 86400).unwrap_or(true);
        if cleanup_due {
            let _ = self.db.cleanup_old_sync_logs(30);
            let _ = self.db.set_setting("last_sync_cleanup", &chrono::Utc::now().to_rfc3339());
        }

        if mutation_ready || self.last_auto_sync.elapsed() >= interval {
            self.last_mutation = None;
            self.last_auto_sync = std::time::Instant::now();
            if self.config.sync_enabled {
                // Network sync: spawn background thread so la UI se queda responsive
                self.start_background_sync();
            } else {
                // Local-only sync (FileCloudProvider): fast, safe to run on main thread
                let _ = self.trigger_sync();
            }
        }
        Ok(())
    }

    pub fn tick_chat_poll(&mut self) -> Result<()> {
        // Only run when the fellowship chat tab is visible and cloud sync is enabled
        if self.active_screen != ActiveScreen::Fellowship
            || self.selected_fellowship_tab != 0
            || !self.config.sync_enabled
        {
            return Ok(());
        }

        // Drain completed poll result
        if self.chat_poll_active {
            let result = self.chat_rx.as_ref().and_then(|rx| rx.try_recv().ok());
            if let Some(poll) = result {
                self.chat_poll_active = false;
                if poll.error.is_none() {
                    if let Some(ts) = poll.last_timestamp {
                        self.last_chat_timestamp.insert(poll.project_id.clone(), ts);
                    }
                    if poll.new_message_count > 0 {
                        self.reload_data()?;
                    }
                }
            }
            return Ok(());
        }

        // Poll every 4 seconds
        if self.last_chat_poll.elapsed() < std::time::Duration::from_millis(4000) {
            return Ok(());
        }

        let shared: Vec<_> = self.projects.iter().filter(|p| p.is_shared).collect();
        if shared.is_empty() || self.selected_fellowship_project_idx >= shared.len() {
            return Ok(());
        }
        let proj_id = shared[self.selected_fellowship_project_idx].id.to_string();

        // Get the `since` timestamp — from our map, or seed from the DB max
        let since = match self.last_chat_timestamp.get(&proj_id) {
            Some(ts) => ts.clone(),
            None => {
                let max_ts: String = self.db.conn.query_row(
                    "SELECT COALESCE(MAX(timestamp), '') FROM chronicle_messages WHERE project_id = ?1",
                    rusqlite::params![proj_id],
                    |r| r.get(0),
                ).unwrap_or_default();
                if !max_ts.is_empty() {
                    self.last_chat_timestamp.insert(proj_id.clone(), max_ts.clone());
                }
                max_ts
            }
        };

        let (tx, rx) = std::sync::mpsc::channel();
        self.chat_rx = Some(rx);
        self.chat_poll_active = true;
        self.last_chat_poll = std::time::Instant::now();

        let identity = self.identity.clone();
        let device_id = self.device_id.clone();
        let server_url = self.server_url.clone();
        let proj_id_thread = proj_id.clone();
        let since_thread = since.clone();

        let _ = std::thread::spawn(move || {
            let make_result = || -> anyhow::Result<ChatPollResult> {
                let client = crate::services::api_client::ApiClient::new(
                    &server_url, identity, &device_id,
                );
                let storage_dir = crate::storage::get_storage_dir()?;
                let db = crate::database::Database::new(&storage_dir.join("questline.db"))?;
                let _ = db.conn.execute_batch("PRAGMA busy_timeout = 1000;");

                // Fetch new messages (only those after `since`)
                let since_encoded = since_thread.replace('+', "%2B");
                let path = if since_thread.is_empty() {
                    format!("chronicle/messages?project_id={}", proj_id_thread)
                } else {
                    format!("chronicle/messages?project_id={}&since={}", proj_id_thread, since_encoded)
                };

                let mut new_count = 0usize;
                let mut last_ts: Option<String> = None;

                if let Ok(resp) = client.send_request("GET", &path, "") {
                    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&resp) {
                        if let Some(msgs) = arr.as_array() {
                            new_count = msgs.len();
                            for msg in msgs {
                                let id = msg["id"].as_str().unwrap_or_default().to_string();
                                let pid = msg["project_id"].as_str().unwrap_or_default().to_string();
                                let sender_identity = msg["sender_identity"].as_str().unwrap_or_default().to_string();
                                let sender_username = msg["sender_username"].as_str().unwrap_or_default().to_string();
                                let content = msg["content"].as_str().unwrap_or_default().to_string();
                                let message_type = msg["message_type"].as_str().unwrap_or("text").to_string();
                                let timestamp = msg["timestamp"].as_str().unwrap_or_default().to_string();
                                if !timestamp.is_empty() {
                                    last_ts = Some(timestamp.clone());
                                }
                                let _ = db.conn.execute(
                                    "INSERT OR IGNORE INTO chronicle_messages (id, project_id, sender_identity, sender_username, content, message_type, timestamp) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                                    rusqlite::params![id, pid, sender_identity, sender_username, content, message_type, timestamp],
                                );
                            }
                        }
                    }
                }

                // Fetch real-time presence for this project
                let presence_path = format!("chronicle/presence?project_id={}", proj_id_thread);
                if let Ok(resp) = client.send_request("GET", &presence_path, "") {
                    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&resp) {
                        if let Some(members) = arr.as_array() {
                            for m in members {
                                let identity_key = m["user_identity"].as_str().unwrap_or_default();
                                let username = m["user_username"].as_str().unwrap_or_default();
                                let last_seen_raw = m["last_seen"].as_str().unwrap_or_default();
                                let is_online = m["is_online"].as_i64().map(|n| n != 0).unwrap_or(false);
                                let display = if is_online {
                                    "Just now".to_string()
                                } else if last_seen_raw.is_empty() {
                                    "Never".to_string()
                                } else {
                                    last_seen_raw.to_string()
                                };
                                if !identity_key.is_empty() {
                                    let _ = db.update_presence(
                                        identity_key, username, is_online, &display,
                                        Some(&proj_id_thread),
                                        if is_online { "Visible" } else { "Offline" },
                                    );
                                }
                            }
                        }
                    }
                }

                Ok(ChatPollResult {
                    project_id: proj_id_thread,
                    new_message_count: new_count,
                    last_timestamp: last_ts,
                    error: None,
                })
            };

            let result = make_result().unwrap_or_else(|e| ChatPollResult {
                project_id: proj_id.clone(),
                new_message_count: 0,
                last_timestamp: None,
                error: Some(e.to_string()),
            });
            let _ = tx.send(result);
        });

        Ok(())
    }

    fn start_background_sync(&mut self) {
        self.do_background_sync(false);
    }

    fn start_forced_sync(&mut self) {
        if self.sync_in_progress {
            self.sync_status_msg = "Sync already in progress...".to_string();
            self.last_sync_status_time = Some(std::time::Instant::now());
            return;
        }
        self.sync_status_msg = "Syncing...".to_string();
        self.do_background_sync(true);
    }

    fn do_background_sync(&mut self, include_contributions: bool) {
        if self.sync_in_progress {
            return;
        }
        self.sync_in_progress = true;
        self.sync_status_msg = "Syncing...".to_string();

        let result_slot = std::sync::Arc::clone(&self.sync_result);
        let identity = self.identity.clone();
        let device_id = self.device_id.clone();
        let server_url = self.server_url.clone();

        let _ = std::thread::spawn(move || {
            let outcome: Result<(usize, usize, Vec<String>)> = (|| {
                let storage_dir = crate::storage::get_storage_dir()?;
                let db_path = storage_dir.join("questline.db");
                let db = crate::database::Database::new(&db_path)?;
                // WAL mode ya permite concurrencia — pero damos 5s por si acaso hay contención
                let _ = db.conn.execute_batch("PRAGMA busy_timeout = 5000;");

                let sync_engine = crate::services::sync_engine::SyncEngine::new(
                    &db, &identity, &device_id, Some(server_url.as_str()),
                )?;
                let (pushed, pulled, conflicts) = sync_engine.sync()?;

                let now_str = chrono::Utc::now().to_rfc3339();
                let _ = db.set_setting("last_sync", &now_str);

                let client = crate::services::api_client::ApiClient::new(
                    &server_url, identity.clone(), &device_id,
                );

                // Pull pending invitations
                if let Ok(resp) = client.send_request("GET", "pending", "") {
                    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&resp) {
                        if let Some(list) = arr.as_array() {
                            for inv in list {
                                let id = inv["id"].as_str().unwrap_or_default().to_string();
                                let project_id = inv["project_id"].as_str().unwrap_or_default().to_string();
                                let project_name = inv["project_name"].as_str().unwrap_or_default().to_string();
                                let inviter_identity = inv["inviter_identity"].as_str().unwrap_or_default().to_string();
                                let inviter_username = inv["inviter_username"].as_str().unwrap_or_default().to_string();
                                let invitee_identity = inv["invitee_identity"].as_str().unwrap_or_default().to_string();
                                let role = inv["role"].as_str().unwrap_or_default().to_string();
                                let status = inv["status"].as_str().unwrap_or("Pending").to_string();
                                let created_at = inv["created_at"].as_str().unwrap_or_default().to_string();
                                let _ = db.conn.execute(
                                    "INSERT OR IGNORE INTO invitations (id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                                    rusqlite::params![id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at],
                                );
                            }
                        }
                    }
                }

                // Pull chronicle messages for shared projects
                if let Ok(projs) = db.get_projects() {
                    for p in projs.into_iter().filter(|p| p.is_shared) {
                        let path = format!("chronicle/messages?project_id={}", p.id);
                        if let Ok(resp) = client.send_request("GET", &path, "") {
                            if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&resp) {
                                if let Some(msgs) = arr.as_array() {
                                    for msg in msgs {
                                        let id = msg["id"].as_str().unwrap_or_default().to_string();
                                        let proj_id = msg["project_id"].as_str().unwrap_or_default().to_string();
                                        let sender_identity = msg["sender_identity"].as_str().unwrap_or_default().to_string();
                                        let sender_username = msg["sender_username"].as_str().unwrap_or_default().to_string();
                                        let content = msg["content"].as_str().unwrap_or_default().to_string();
                                        let message_type = msg["message_type"].as_str().unwrap_or("text").to_string();
                                        let timestamp = msg["timestamp"].as_str().unwrap_or_default().to_string();
                                        let _ = db.conn.execute(
                                            "INSERT OR IGNORE INTO chronicle_messages (id, project_id, sender_identity, sender_username, content, message_type, timestamp) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                                            rusqlite::params![id, proj_id, sender_identity, sender_username, content, message_type, timestamp],
                                        );
                                    }
                                }
                            }
                        }
                    }
                }

                // Refresh companion presence
                if let Ok(resp) = client.send_request("GET", "project/companions", "") {
                    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&resp) {
                        if let Some(companions) = arr.as_array() {
                            for comp in companions {
                                let identity_key = comp["user_identity"].as_str().unwrap_or_default();
                                let username = comp["user_username"].as_str().unwrap_or_default();
                                if identity_key.is_empty() || username.is_empty() { continue; }
                                let last_seen_raw = comp["last_seen"].as_str().unwrap_or_default();
                                let (is_online, last_seen_display) = App::parse_companion_last_seen(last_seen_raw);
                                let _ = db.update_presence(identity_key, username, is_online, &last_seen_display, None, if is_online { "Visible" } else { "Offline" });
                                let _ = db.conn.execute(
                                    "UPDATE project_members SET user_username = ?1 WHERE user_identity = ?2 AND user_username = 'Accepted Companion'",
                                    rusqlite::params![username, identity_key],
                                );
                            }
                        }
                    }
                }

                // Pull Realm Activity (global chronicle) entries from server into local DB
                if let Ok(resp) = client.send_request("GET", "global_chronicle", "") {
                    if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&resp) {
                        if let Some(entries) = arr.as_array() {
                            for e in entries {
                                let id = e["id"].as_str().unwrap_or_default();
                                let hero = e["hero_name"].as_str().unwrap_or_default();
                                let etype = e["event_type"].as_str().unwrap_or_default();
                                let desc = e["description"].as_str().unwrap_or_default();
                                let ts = e["timestamp"].as_str().unwrap_or_default();
                                if !id.is_empty() {
                                    let _ = db.conn.execute(
                                        "INSERT OR IGNORE INTO global_chronicle (id, hero_name, event_type, description, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
                                        rusqlite::params![id, hero, etype, desc, ts],
                                    );
                                }
                            }
                        }
                    }
                }

                // Submit chapter contribution increments (only on forced/manual sync)
                if include_contributions {
                    if let Some(chapter) = crate::models::chapter::get_active_chapter() {
                        // Only count tasks owned by this user — prevents shared-project
                        // pulls (other users' completed tasks) from inflating the delta.
                        if let Ok(snapshot) = db.get_contribution_snapshot(&identity.public_key) {
                            let last_sent = db.get_last_sent_contributions(chapter.id).unwrap_or_default();
                            if last_sent.is_empty() {
                                // Fresh baseline: record current totals without sending so
                                // only future actions contribute to the chapter objectives.
                                let _ = db.save_sent_contributions(chapter.id, &snapshot);
                            } else {
                                let mut increments = std::collections::HashMap::new();
                                for (key, &current) in &snapshot {
                                    let prev = last_sent.get(key).copied().unwrap_or(0);
                                    if current > prev {
                                        increments.insert(key.clone(), current - prev);
                                    }
                                }
                                if !increments.is_empty() {
                                    // Save the new baseline FIRST (anti-cheat: if anything
                                    // fails after this point the same delta won't replay).
                                    // Only send if the save succeeded — better to miss one
                                    // contribution than to double-count on every retry.
                                    if db.save_sent_contributions(chapter.id, &snapshot).is_ok() {
                                        let _ = client.submit_chapter_contribution(chapter.id, &increments);
                                    }
                                }
                            }
                        }
                        // Cache fresh chapter progress for the UI to pick up after sync
                        if let Ok(val) = client.fetch_chapter_progress(chapter.id) {
                            let json = serde_json::to_string(&val).unwrap_or_default();
                            let _ = db.set_setting("chapter_progress_cache", &json);
                        }
                    }
                }

                // Update sync and conflict counters
                let sync_count = db.get_setting("sync_count")?.and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                let _ = db.set_setting("sync_count", &(sync_count + 1).to_string());
                if !conflicts.is_empty() {
                    let conflict_count = db.get_setting("conflict_count")?.and_then(|s| s.parse::<i32>().ok()).unwrap_or(0);
                    let _ = db.set_setting("conflict_count", &(conflict_count + conflicts.len() as i32).to_string());
                }

                Ok((pushed, pulled, conflicts))
            })();

            let bg = match outcome {
                Ok((pushed, pulled, conflicts)) => BackgroundSyncResult { pushed, pulled, conflicts, error: None },
                Err(e) => BackgroundSyncResult { pushed: 0, pulled: 0, conflicts: Vec::new(), error: Some(e.to_string()) },
            };
            if let Ok(mut guard) = result_slot.lock() {
                *guard = Some(bg);
            }
        });
    }

    pub fn mark_dirty(&mut self) {
        self.last_mutation = Some(std::time::Instant::now());
    }

    // ── Notification Sprite helpers ──────────────────────────────────────────

    /// Average chapter-one objective completion (0.0 = none, 1.0 = all done).
    fn chapter_one_progress(&self) -> f64 {
        match &self.chapter_progress {
            Some(data) if data.completed => 1.0,
            Some(data) if !data.objectives.is_empty() => {
                let sum: f64 = data.objectives.iter()
                    .map(|o| (o.current as f64 / o.target as f64).min(1.0))
                    .sum();
                sum / data.objectives.len() as f64
            }
            _ => 0.0,
        }
    }

    fn can_spawn_sprite_notification(&self) -> bool {
        if matches!(
            self.active_screen,
            ActiveScreen::Intro | ActiveScreen::Prologue | ActiveScreen::Onboarding | ActiveScreen::Editor
        ) { return false; }
        if self.modal_state != ModalType::None { return false; }
        if self.active_focus_session.is_some() { return false; }
        if self.sync_in_progress { return false; }
        true
    }

    /// Called after a task is marked complete — occasionally spawns a Swarm response.
    pub fn maybe_spawn_task_completion_sprite(&mut self) {
        if !self.can_spawn_sprite_notification() { return; }
        if self.sprite_notifications_shown_this_session >= 5 { return; }
        let progress = self.chapter_one_progress();
        if progress >= 1.0 { return; }
        // 15% chance, tapering to ~5% as the Swarm weakens
        let chance = 0.15 * (1.0 - progress * 0.7);
        use rand::Rng;
        if rand::thread_rng().r#gen::<f64>() > chance { return; }
        let (msg, title) = pick_task_completion_sprite_message();
        self.notifications.push(Notification::swarm(msg, title));
        self.sprite_notifications_shown_this_session += 1;
        self.last_sprite_notification_time = Some(std::time::Instant::now());
    }

    /// Background idle-spawn tick — checked once per minute, very low probability.
    pub fn tick_sprite_notifications(&mut self) {
        if let Some(t) = self.last_sprite_check_time {
            if t.elapsed().as_secs() < 60 { return; }
        }
        self.last_sprite_check_time = Some(std::time::Instant::now());

        if !self.can_spawn_sprite_notification() { return; }
        if self.sprite_notifications_shown_this_session >= 5 { return; }

        let progress = self.chapter_one_progress();
        if progress >= 1.0 { return; }

        // Minimum cooldown between Sprite notifications (scales up as Swarm weakens)
        let min_cooldown: u64 = if progress >= 0.90 { 3600 }
            else if progress >= 0.75 { 1800 }
            else if progress >= 0.50 { 900 }
            else if progress >= 0.25 { 600 }
            else { 300 };
        if let Some(t) = self.last_sprite_notification_time {
            if t.elapsed().as_secs() < min_cooldown { return; }
        }

        // Per-minute spawn probability (decreases as chapter progresses)
        let spawn_chance: f64 = if progress >= 0.90 { 0.015 }
            else if progress >= 0.75 { 0.04 }
            else if progress >= 0.50 { 0.07 }
            else if progress >= 0.25 { 0.12 }
            else { 0.20 };
        use rand::Rng;
        if rand::thread_rng().r#gen::<f64>() > spawn_chance { return; }

        let (msg, title) = pick_sprite_message();
        self.notifications.push(Notification::swarm(msg, title));
        self.sprite_notifications_shown_this_session += 1;
        self.last_sprite_notification_time = Some(std::time::Instant::now());
    }

    // Show the update modal as soon as the background check finishes and the screen is ready.
    pub fn tick_update_check(&mut self) {
        if self.update_check_done { return; }
        if let Ok(guard) = self.update_check.try_lock() {
            if let Some(ref ver) = *guard {
                if ver.is_empty() {
                    self.update_check_done = true;
                    return;
                }
                if self.modal_state == ModalType::None
                    && self.active_screen != ActiveScreen::Intro
                    && self.active_screen != ActiveScreen::Onboarding
                {
                    self.modal_state = ModalType::UpdateAvailable {
                        latest_version: ver.clone(),
                    };
                    self.update_check_done = true;
                }
            }
        }
    }

    pub fn tick_chapter_progress(&mut self) {
        if self.chapter_progress_refreshed.swap(false, std::sync::atomic::Ordering::Relaxed) {
            self.load_chapter_progress_from_cache();
        }
    }

    pub fn pull_great_chronicle_async(&self) {
        if !self.config.sync_enabled {
            return;
        }
        let client = crate::services::api_client::ApiClient::new(
            &self.server_url,
            self.identity.clone(),
            &self.device_id,
        );
        if let Ok(storage_dir) = crate::storage::get_storage_dir() {
            let db_path = storage_dir.join("questline.db");
            let _ = std::thread::spawn(move || {
                if let Ok(resp) = client.send_request("GET", "global_chronicle", "") {
                    if let Ok(arr) =
                        serde_json::from_str::<serde_json::Value>(&resp)
                    {
                        if let Some(entries) = arr.as_array() {
                            if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                                for e in entries {
                                    let id = e["id"].as_str().unwrap_or_default();
                                    let hero = e["hero_name"].as_str().unwrap_or_default();
                                    let etype = e["event_type"].as_str().unwrap_or_default();
                                    let desc = e["description"].as_str().unwrap_or_default();
                                    let ts = e["timestamp"].as_str().unwrap_or_default();
                                    if !id.is_empty() {
                                        let _ = conn.execute(
                                            "INSERT OR IGNORE INTO global_chronicle (id, hero_name, event_type, description, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
                                            rusqlite::params![id, hero, etype, desc, ts],
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    }

    // Fetches chapter progress from the server asynchronously and caches it in the settings table.
    pub fn pull_chapter_progress_async(&self) {
        if !self.config.sync_enabled {
            return;
        }
        let client = crate::services::api_client::ApiClient::new(
            &self.server_url,
            self.identity.clone(),
            &self.device_id,
        );
        if let Ok(storage_dir) = crate::storage::get_storage_dir() {
            let db_path = storage_dir.join("questline.db");
            let flag = std::sync::Arc::clone(&self.chapter_progress_refreshed);
            let _ = std::thread::spawn(move || {
                if let Some(chapter) = crate::models::chapter::get_active_chapter() {
                    if let Ok(val) = client.fetch_chapter_progress(chapter.id) {
                        let json = serde_json::to_string(&val).unwrap_or_default();
                        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                            let _ = conn.execute(
                                "INSERT INTO settings (key, value) VALUES ('chapter_progress_cache', ?1)
                                 ON CONFLICT(key) DO UPDATE SET value = ?1",
                                rusqlite::params![json],
                            );
                            flag.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    if let Ok(hist) = client.fetch_chapter_history() {
                        let json = serde_json::to_string(&hist).unwrap_or_default();
                        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                            let _ = conn.execute(
                                "INSERT INTO settings (key, value) VALUES ('chapter_history_cache', ?1)
                                 ON CONFLICT(key) DO UPDATE SET value = ?1",
                                rusqlite::params![json],
                            );
                        }
                    }
                }
            });
        }
    }

    // Loads cached chapter progress from settings into app state.
    pub fn load_chapter_progress_from_cache(&mut self) {
        if let Ok(Some(json)) = self.db.get_setting("chapter_progress_cache") {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json) {
                let chapter_id = val["chapter_id"].as_str().unwrap_or("").to_string();
                let completed = val["completed"].as_bool().unwrap_or(false);
                let completed_at = val["completed_at"].as_str().map(|s| s.to_string());

                let mut objectives = Vec::new();
                if let Some(chapter) = crate::models::chapter::get_active_chapter() {
                    for obj_def in chapter.objectives {
                        let current = val["objectives"]
                            .as_array()
                            .and_then(|arr| arr.iter().find(|o| o["type"].as_str() == Some(obj_def.id)))
                            .and_then(|o| o["current"].as_u64())
                            .unwrap_or(0);
                        let target = val["objectives"]
                            .as_array()
                            .and_then(|arr| arr.iter().find(|o| o["type"].as_str() == Some(obj_def.id)))
                            .and_then(|o| o["target"].as_u64())
                            .unwrap_or(obj_def.target);
                        objectives.push(crate::models::chapter::ChapterObjectiveProgress {
                            id: obj_def.id.to_string(),
                            name: obj_def.name.to_string(),
                            current,
                            target,
                        });
                    }
                }

                // Check if chapter newly completed — unlock rewards
                let was_completed = self.chapter_completion_seen;
                if completed && !was_completed {
                    self.chapter_completion_seen = true;
                    if let Some(chapter) = crate::models::chapter::get_active_chapter() {
                        for &lore_id in chapter.reward_lore_ids {
                            let _ = self.db.unlock_lore_entry(lore_id);
                        }
                        let ts = chrono::Utc::now().to_rfc3339();
                        let entry_id = uuid::Uuid::new_v4().to_string();
                        let _ = self.db.conn.execute(
                            "INSERT OR IGNORE INTO global_chronicle (id, hero_name, event_type, description, timestamp) VALUES (?1, 'The Realm', 'ChapterComplete', ?2, ?3)",
                            rusqlite::params![entry_id, chapter.completion_text, ts],
                        );
                        // Show the chapter complete modal once — persisted so it survives restarts
                        let modal_key = format!("chapter_complete_modal_shown_{}", chapter.id);
                        let already_shown = self.db.get_setting(&modal_key).ok().flatten().is_some();
                        if !already_shown {
                            let _ = self.db.set_setting(&modal_key, "1");
                            self.modal_state = ModalType::ChapterComplete;
                        }

                        // Grant 5 000 XP once to every user who contributed to the chapter
                        let xp_key = format!("chapter_xp_rewarded_{}", chapter.id);
                        let xp_already_given = self.db.get_setting(&xp_key).ok().flatten().is_some();
                        if !xp_already_given {
                            let contributed = self.db.conn.query_row(
                                "SELECT COUNT(*) FROM chapter_contribution_log WHERE chapter_id = ?1 AND last_sent_total > 0",
                                rusqlite::params![chapter.id],
                                |row| row.get::<_, i64>(0),
                            ).unwrap_or(0) > 0;
                            if contributed {
                                let _ = self.db.set_setting(&xp_key, "1");
                                let _ = self.grant_xp("Chapter Complete: The Notification Swarm", 5000);
                            }
                        }
                    }
                }

                self.chapter_progress = Some(crate::models::chapter::ChapterProgressData {
                    chapter_id,
                    objectives,
                    completed,
                    completed_at,
                });
            }
        }

        if let Ok(Some(json)) = self.db.get_setting("chapter_history_cache") {
            if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&json) {
                if let Some(items) = arr.as_array() {
                    self.chapter_history = items.iter().filter_map(|item| {
                        Some(crate::models::chapter::ChapterHistoryEntry {
                            chapter_id: item["chapter_id"].as_str()?.to_string(),
                            title: item["title"].as_str().unwrap_or("").to_string(),
                            completed_at: item["completed_at"].as_str().unwrap_or("").to_string(),
                            personal_contribution: item["personal_contribution"].as_u64().unwrap_or(0),
                        })
                    }).collect();
                }
            }
        }
    }

    pub fn submit_chapter_contribution(&mut self, client: &crate::services::api_client::ApiClient) {
        if let Some(chapter) = crate::models::chapter::get_active_chapter() {
            let owner_key = self.identity.public_key.clone();
            let snapshot = match self.db.get_contribution_snapshot(&owner_key) {
                Ok(s) => s,
                Err(_) => return,
            };

            // Resync local baseline from server's confirmed per-user totals.
            // This corrects over-advanced baselines caused by the old save-before-send pattern
            // (where the baseline was saved even when the network call failed, silently losing
            // contributions). For each objective the server has a confirmed record, we trust
            // that number; for objectives with no server record we keep the local baseline to
            // avoid re-contributing something that may already be reflected in the global total.
            if let Ok(confirmed) = client.fetch_my_chapter_contributions(chapter.id) {
                let local_baseline = self.db.get_last_sent_contributions(chapter.id).unwrap_or_default();
                let all_objectives = [
                    "tasks_completed", "subtasks_completed", "focus_sessions",
                    "tree_waterings", "rituals_completed", "reflections_written", "scrolls_created",
                ];
                let mut corrected: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
                for &obj in &all_objectives {
                    let server_val = confirmed.get(obj).copied();
                    let local_val = local_baseline.get(obj).copied().unwrap_or(0);
                    corrected.insert(obj.to_string(), server_val.unwrap_or(local_val));
                }
                let _ = self.db.save_sent_contributions(chapter.id, &corrected);
            }

            let last_sent = self.db.get_last_sent_contributions(chapter.id).unwrap_or_default();

            if last_sent.is_empty() {
                let _ = self.db.save_sent_contributions(chapter.id, &snapshot);
                return;
            }

            let mut increments = std::collections::HashMap::new();
            for (key, &current) in &snapshot {
                let prev = last_sent.get(key).copied().unwrap_or(0);
                if current > prev {
                    increments.insert(key.clone(), current - prev);
                }
            }
            if !increments.is_empty() {
                if client.submit_chapter_contribution(chapter.id, &increments).is_ok() {
                    let _ = self.db.save_sent_contributions(chapter.id, &snapshot);
                }
            }
        }
    }

    // Fetches chapter progress synchronously and updates app state (used after sync).
    pub fn refresh_chapter_progress_sync(&mut self, client: &crate::services::api_client::ApiClient) {
        if let Some(chapter) = crate::models::chapter::get_active_chapter() {
            if let Ok(val) = client.fetch_chapter_progress(chapter.id) {
                let json = serde_json::to_string(&val).unwrap_or_default();
                let _ = self.db.set_setting("chapter_progress_cache", &json);
                self.load_chapter_progress_from_cache();
            }
        }
    }

    pub fn push_great_chronicle_async(
        &self,
        event_type: &str,
        description: &str,
        show_name: bool,
    ) {
        if !self.config.sync_enabled {
            return;
        }
        if self.config.chronicle_share_level == "none" {
            return;
        }
        // Named events require a username; anonymous events use "The Realm" as
        // a sentinel so the server accepts the entry but the feed hides the name.
        let hero_name = if show_name {
            match &self.user {
                Some(u) if !u.username.is_empty() => u.username.clone(),
                _ => return,
            }
        } else {
            "The Realm".to_string()
        };
        let client = crate::services::api_client::ApiClient::new(
            &self.server_url,
            self.identity.clone(),
            &self.device_id,
        );
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();
        let body = format!(
            r#"{{"id":"{id}","hero_name":"{hero_name}","event_type":"{event_type}","description":"{description}","timestamp":"{timestamp}"}}"#,
        );
        let _ = std::thread::spawn(move || {
            let _ = client.send_request("POST", "global_chronicle", &body);
        });
    }

    pub fn pull_invitations_async(&self) {
        if self.config.sync_enabled {
            let client = crate::services::api_client::ApiClient::new(
                &self.server_url,
                self.identity.clone(),
                &self.device_id,
            );
            if let Ok(storage_dir) = crate::storage::get_storage_dir() {
                let db_path = storage_dir.join("questline.db");
                let _ = std::thread::spawn(move || {
                    if let Ok(resp_str) = client.send_request("GET", "pending", "") {
                        if let Ok(server_invites) = serde_json::from_str::<serde_json::Value>(&resp_str) {
                            if let Some(arr) = server_invites.as_array() {
                                if let Ok(conn) = rusqlite::Connection::open(&db_path) {
                                    for inv_val in arr {
                                        let id = inv_val["id"].as_str().unwrap_or_default().to_string();
                                        let project_id = inv_val["project_id"].as_str().unwrap_or_default().to_string();
                                        let project_name = inv_val["project_name"].as_str().unwrap_or_default().to_string();
                                        let inviter_identity = inv_val["inviter_identity"].as_str().unwrap_or_default().to_string();
                                        let inviter_username = inv_val["inviter_username"].as_str().unwrap_or_default().to_string();
                                        let invitee_identity = inv_val["invitee_identity"].as_str().unwrap_or_default().to_string();
                                        let role = inv_val["role"].as_str().unwrap_or_default().to_string();
                                        let status = inv_val["status"].as_str().unwrap_or_default().to_string();
                                        let created_at = inv_val["created_at"].as_str().unwrap_or_default().to_string();

                                        let _ = conn.execute(
                                            "INSERT OR IGNORE INTO invitations (id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                                            rusqlite::params![id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at]
                                        );
                                    }
                                }
                            }
                        }
                    }
                });
            }
        }
    }

    pub fn tick_focus_session(&mut self) -> Result<()> {
        if let Some(ref active) = self.active_focus_session {
            let total_seconds = (active.duration_mins * 60) as i64;
            let elapsed_seconds = (Utc::now() - active.start_time).num_seconds();
            let remaining = total_seconds - elapsed_seconds;
            if remaining <= 0 {
                self.audio_player.play_task_complete();
                // Focus session completes!
                let duration = active.duration_mins;
                let active_soundscape = active.soundscape.clone();
                let project_id = active.project_id;
                let task_id = active.task_id;
                self.active_focus_session = None;

                let mut xp = match duration {
                    15 => 10,
                    25 => 20,
                    45 => 35,
                    60 => 50,
                    90 => 80,
                    _ => (duration as f64 * 0.8).round() as i32,
                };
                if active_soundscape == "Ocean Waves" {
                    xp += 5;
                }

                self.grant_xp("Focus Session Complete", xp)?;
                let focus_trigger = if duration == 25 { "focus_pomodoro" } else { "focus_session" };
                self.apply_class_passive(focus_trigger, 0)?;
                self.increment_quest_progress(25, duration)?;
                self.simulate_memory_fragment_unlock("focus_session")?;

                let final_duration = if active_soundscape == "White Noise" {
                    (duration as f64 * 1.1).round() as i32
                } else {
                    duration
                };

                // Insert log to focus_sessions
                let sess = FocusSession {
                    id: Uuid::new_v4(),
                    project_id,
                    task_id,
                    duration_mins: final_duration,
                    xp_gained: xp,
                    completed_at: Utc::now(),
                    soundscape: active_soundscape.clone(),
                };
                self.db.insert_focus_session(&sess)?;
                self.mark_dirty();
                self.push_great_chronicle_async(
                    "FocusSession",
                    &format!("honored a {}-minute focus session.", final_duration),
                    true,
                );

                self.notifications.push(Notification::info(format!("Focus Completed! Gained +{} XP", xp)));

                // Apply active soundscape bonuses
                if active_soundscape == "Forest Sounds" {
                    self.grow_tree(1)?;
                } else if active_soundscape == "Rain Sounds" {
                    if self.is_watering_allowed()?.is_ok() {
                        let mut tree = self.db.get_zen_tree()?;
                        tree.water_today += 1;
                        tree.growth += 1;
                        tree.last_watered = Some(Utc::now());
                        self.db.update_zen_tree(&tree)?;
                        self.notifications.push(Notification::info(format!(
                                "Rain Sounds Auto-Watered! Growth +1 (Today: {}/2)",
                                tree.water_today
                            )));
                        self.update_daily_adventure_progress("water_tree", 1)?;
                        self.check_tree_evolution(&mut tree)?;
                    } else {
                        self.notifications.push(Notification::warning("Rain Sounds: Tree is already watered for this time period."));
                    }
                }

                // Unlock Focus Achievements
                let stats = self.db.get_statistics()?;
                if stats.sessions_completed >= 1 {
                    self.unlock_achievement("first_focus")?;
                }
                if stats.sessions_completed >= 100 {
                    self.unlock_achievement("deep_worker")?;
                }
                if stats.sessions_completed >= 500 {
                    self.unlock_achievement("master_concentration")?;
                }
                if duration >= 90 {
                    self.unlock_achievement("ninety_minute_sage")?;
                }
                self.check_action_achievements()?;

                // Perform productive actions triggers
                self.complete_productive_action()?;
                self.check_traits()?;
                self.reload_data()?;
            }
        }
        Ok(())
    }

    pub fn start_focus_session(
        &mut self,
        duration_mins: i32,
        project_id: Option<Uuid>,
        task_id: Option<Uuid>,
    ) -> Result<()> {
        let soundscape = {
            use crate::audio::SOUNDSCAPES;
            match self.selected_focus_soundscape_idx {
                0 => "None".to_string(),
                idx if idx <= SOUNDSCAPES.len() => SOUNDSCAPES[idx - 1].name.to_string(),
                _ => "None".to_string(),
            }
        };

        let active = ActiveFocusSession {
            start_time: Utc::now(),
            duration_mins,
            project_id,
            task_id,
            soundscape: soundscape.clone(),
        };
        self.active_focus_session = Some(active);
        self.active_screen = ActiveScreen::Focus;

        if soundscape != "None" {
            self.audio_player.play(&soundscape);
        } else {
            self.audio_player.stop();
        }

        Ok(())
    }

    pub fn handle_focus_screen_key(&mut self, key: KeyEvent) -> Result<()> {
        if self.active_focus_session.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.active_focus_session = None;
                    self.notifications.push(Notification::warning("Focus Session Cancelled"));
                }
                _ => {}
            }
            return Ok(());
        }

        let active_projects: Vec<Project> = self
            .projects
            .iter()
            .filter(|p| !p.completed && !p.archived)
            .cloned()
            .collect();

        let mut active_tasks: Vec<Task> = Vec::new();
        if self.selected_focus_project_idx > 0
            && self.selected_focus_project_idx <= active_projects.len()
        {
            let selected_p_id = active_projects[self.selected_focus_project_idx - 1].id;
            active_tasks = self.all_tasks.iter()
                .filter(|t| t.project_id == Some(selected_p_id) && !t.completed)
                .cloned()
                .collect();
        }

        match key.code {
            KeyCode::Left | KeyCode::Char('h') => {
                self.selected_focus_field_idx = if self.selected_focus_field_idx > 0 {
                    self.selected_focus_field_idx - 1
                } else {
                    3
                };
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.selected_focus_field_idx = (self.selected_focus_field_idx + 1) % 4;
            }
            KeyCode::Up | KeyCode::Char('k') => match self.selected_focus_field_idx {
                0 => {
                    self.selected_focus_duration_idx = if self.selected_focus_duration_idx > 0 {
                        self.selected_focus_duration_idx - 1
                    } else {
                        5
                    };
                }
                1 => {
                    let max_proj_idx = active_projects.len();
                    self.selected_focus_project_idx = if self.selected_focus_project_idx > 0 {
                        self.selected_focus_project_idx - 1
                    } else {
                        max_proj_idx
                    };
                    self.selected_focus_task_idx = 0;
                }
                2 => {
                    let max_task_idx = active_tasks.len();
                    self.selected_focus_task_idx = if self.selected_focus_task_idx > 0 {
                        self.selected_focus_task_idx - 1
                    } else {
                        max_task_idx
                    };
                }
                3 => {
                    use crate::audio::SOUNDSCAPES;
                    let total = SOUNDSCAPES.len() + 1;
                    self.selected_focus_soundscape_idx = if self.selected_focus_soundscape_idx == 0 {
                        total - 1
                    } else {
                        self.selected_focus_soundscape_idx - 1
                    };
                }
                _ => {}
            },
            KeyCode::Down | KeyCode::Char('j') => match self.selected_focus_field_idx {
                0 => {
                    self.selected_focus_duration_idx = (self.selected_focus_duration_idx + 1) % 6;
                }
                1 => {
                    let max_proj_idx = active_projects.len();
                    self.selected_focus_project_idx =
                        (self.selected_focus_project_idx + 1) % (max_proj_idx + 1);
                    self.selected_focus_task_idx = 0;
                }
                2 => {
                    let max_task_idx = active_tasks.len();
                    self.selected_focus_task_idx =
                        (self.selected_focus_task_idx + 1) % (max_task_idx + 1);
                }
                3 => {
                    use crate::audio::SOUNDSCAPES;
                    self.selected_focus_soundscape_idx =
                        (self.selected_focus_soundscape_idx + 1) % (SOUNDSCAPES.len() + 1);
                }
                _ => {}
            },
            KeyCode::Enter => {
                let duration_mins = match self.selected_focus_duration_idx {
                    0 => 15,
                    1 => 25,
                    2 => 45,
                    3 => 60,
                    4 => 90,
                    5 => -1,
                    _ => 25,
                };

                if duration_mins == -1 {
                    self.modal_state = ModalType::CustomFocusDuration {
                        input: String::new(),
                    };
                } else {
                    let project_id = if self.selected_focus_project_idx > 0
                        && self.selected_focus_project_idx <= active_projects.len()
                    {
                        Some(active_projects[self.selected_focus_project_idx - 1].id)
                    } else {
                        None
                    };

                    let task_id = if self.selected_focus_task_idx > 0
                        && self.selected_focus_task_idx <= active_tasks.len()
                    {
                        Some(active_tasks[self.selected_focus_task_idx - 1].id)
                    } else {
                        None
                    };

                    // If Local Folder is selected but no folder is configured, prompt first
                    use crate::audio::SOUNDSCAPES;
                    let is_local_folder = self.selected_focus_soundscape_idx > 0
                        && SOUNDSCAPES[self.selected_focus_soundscape_idx - 1].name == "Local Folder";
                    if is_local_folder {
                        let folder = self.db.get_setting("local_music_folder").unwrap_or_default().unwrap_or_default();
                        if folder.trim().is_empty() {
                            self.modal_state = ModalType::LocalMusicFolder { input: String::new(), suggestions: vec![], selected: 0 };
                            return Ok(());
                        }
                    }

                    self.start_focus_session(duration_mins, project_id, task_id)?;
                }
            }
            _ => {
                self.handle_top_screen_key(key)?;
            }
        }
        Ok(())
    }

    pub fn check_traits(&mut self) -> Result<()> {
        let stats = self.db.get_statistics()?;
        if stats.tasks_completed >= 100 {
            self.unlock_trait("task_slayer")?;
        }
        if stats.notes_created >= 100 {
            self.unlock_trait("scholar")?;
        }
        if stats.journal_entries >= 100 {
            self.unlock_trait("historian")?;
        }
        let tree = self.db.get_zen_tree()?;
        if tree.stage >= 5 {
            self.unlock_trait("gardener")?;
        }
        if stats.sessions_completed >= 100 {
            self.unlock_trait("focused_mind")?;
        }
        Ok(())
    }

    pub fn unlock_trait(&mut self, trait_id: &str) -> Result<()> {
        let unlocked = self.db.get_unlocked_traits()?;
        if !unlocked.contains(&trait_id.to_string()) {
            self.db.unlock_trait(trait_id)?;
            let display_name = match trait_id {
                "task_slayer" => "Task Slayer",
                "scholar" => "Scholar",
                "historian" => "Historian",
                "gardener" => "Gardener",
                "focused_mind" => "Focused Mind",
                _ => trait_id,
            };
            self.notifications.push(Notification::info(format!("Trait Unlocked: {}!", display_name)));
        }
        Ok(())
    }

    pub fn complete_ritual(&mut self, ritual_id: &str) -> Result<()> {
        let date = chrono::Local::now().date_naive();
        let completed_today = self.db.get_ritual_history_for_date(date)?;
        if !completed_today.contains(&ritual_id.to_string()) {
            self.audio_player.play_task_complete();
            self.db.complete_ritual(ritual_id, date)?;
            self.mark_dirty();
            self.trigger_ambient_particles();

            let rituals = self.db.get_rituals()?;
            if let Some(r) = rituals.iter().find(|rit| rit.id == ritual_id) {
                let xp = r.reward_xp;
                let ritual_name = r.name.clone();
                self.grant_xp("Sidequest Completed", xp)?;
                self.push_great_chronicle_async(
                    "SidequestComplete",
                    "fulfilled a sidequest.",
                    true,
                );
                self.notifications.push(Notification::info(format!("Sidequest Completed: {}! (+{} XP)", ritual_name, xp)));
            }

            self.complete_productive_action()?;
            self.check_traits()?;
            self.reload_data()?;
        }
        Ok(())
    }

    pub fn toggle_milestone(&mut self, milestone_id: Uuid) -> Result<()> {
        let p_id = self
            .active_project_id
            .ok_or_else(|| anyhow::anyhow!("No active project"))?;
        let mut milestones = self.db.get_milestones_for_project(p_id)?;
        if let Some(m) = milestones.iter_mut().find(|ms| ms.id == milestone_id) {
            if m.completed {
                self.notifications.push(Notification::warning("Milestone already achieved — it cannot be undone."));
                return Ok(());
            }

            // Determine whether to use template-based or legacy checks
            let requirements_met = if m.template_id.is_empty() {
                // Legacy milestone: hardcoded checks (3 days, 3 tasks, 1 journal)
                let age_days = (Utc::now() - m.created_at).num_days();
                if age_days < 3 {
                    let days_left = 3 - age_days;
                    self.notifications.push(Notification::info(format!(
                            "Milestone too new — wait {} more day(s) to complete it.",
                            days_left
                        )));
                    return Ok(());
                }
                let completed_tasks = self.all_tasks.iter()
                    .filter(|t| t.project_id == Some(p_id) && t.completed)
                    .count();
                if completed_tasks < 3 {
                    self.notifications.push(Notification::info(format!(
                            "Need {} more completed quest(s) in this project first.",
                            3 - completed_tasks
                        )));
                    return Ok(());
                }
                let all_journals = self.db.get_journal_entries().unwrap_or_default();
                let project_journals = all_journals
                    .iter()
                    .filter(|j| j.project_id == p_id)
                    .count();
                if project_journals < 1 {
                    self.notifications.push(Notification::warning("Write at least 1 Chronicle entry for this project first."));
                    return Ok(());
                }
                true
            } else {
                // Template-based milestone: check all template requirements
                match milestone_templates::get_template_by_id(&m.template_id) {
                    None => {
                        // Template not found — treat as legacy (allow completion)
                        true
                    }
                    Some(tmpl) => {
                        let project_created_at = self
                            .projects
                            .iter()
                            .find(|p| p.id == p_id)
                            .map(|p| p.created_at)
                            .unwrap_or_else(Utc::now);
                        let stats = build_project_stats(p_id, project_created_at, &self.db);
                        let progress =
                            milestone_templates::compute_progress(tmpl.requirements, &stats);
                        let unmet: Vec<_> = progress.iter().filter(|r| !r.met).collect();
                        if !unmet.is_empty() {
                            // Report first unmet requirement
                            let r = &unmet[0];
                            self.notifications.push(Notification::info(format!(
                                    "Requirement not met: {} ({}/{})",
                                    r.label, r.current, r.target
                                )));
                            return Ok(());
                        }
                        true
                    }
                }
            };

            if requirements_met {
                m.completed = true;
                self.db.update_milestone(m)?;
                self.mark_dirty();

                let milestone_xp = m.xp_reward;
                let milestone_name = m.name.clone();
                self.grant_xp(&format!("Milestone Met: {}", milestone_name), milestone_xp)?;
                self.trigger_ambient_particles();

                if let Some(ref u) = self.user {
                    let day_number = (Utc::now() - u.created_at).num_days() as i32 + 1;
                    self.db.add_chronicle_entry(
                        day_number,
                        &format!("Completed Milestone: {}.", milestone_name),
                    )?;
                }

                self.notifications.push(Notification::info(format!(
                        "Milestone Achieved: {}! (+{} XP)",
                        milestone_name, milestone_xp
                    )));

                self.grow_tree(5)?;
                self.complete_productive_action()?;
                self.check_traits()?;
                if !m.template_id.is_empty() {
                    let lore_id = format!("milestone_{}", m.template_id);
                    self.unlock_achievement(&lore_id)?;
                    let _ = self.db.unlock_lore_entry(&lore_id);
                }
                self.reload_data()?;
            }
        }
        Ok(())
    }

    pub fn complete_project(&mut self, project_id: Uuid) -> Result<()> {
        if let Some(mut existing) = self.projects.iter().find(|p| p.id == project_id).cloned() {
            if !existing.completed {
                existing.completed = true;
                existing.archived = true;
                self.db.update_project(&existing)?;
                self.mark_dirty();
                self.audio_player.play_task_complete();
                self.trigger_ambient_particles();

                let proj_name = existing.name.clone();
                self.grant_xp(&format!("Complete Project: {}", proj_name), 200)?;

                // Stage 6: increment Level 75 quest progress
                self.increment_quest_progress(75, 1)?;

                if let Some(ref u) = self.user {
                    let day_number = (Utc::now() - u.created_at).num_days() as i32 + 1;
                    self.db.add_chronicle_entry(
                        day_number,
                        &format!("Completed Project: {}.", proj_name),
                    )?;
                }

                self.trigger_celebration(
                    &format!("PROJECT COMPLETED: {}", proj_name),
                    &format!("You have completed the grand project '{}'!\nOrder has been restored to this workspace.", proj_name),
                    "SUCCESS"
                );

                self.grow_tree(20)?;

                self.notifications.push(Notification::info(format!(
                        "Project Completed: {}! (+200 XP, +20 Growth)",
                        existing.name
                    )));

                self.check_action_achievements()?;
                self.check_traits()?;
                self.simulate_memory_fragment_unlock("project_complete")?;
                self.reload_data()?;
            }
        }
        Ok(())
    }

    fn handle_bug_report_key(&mut self, key: KeyEvent) -> Result<()> {
        let modal = match self.bug_report_modal.as_mut() {
            Some(m) => m,
            None => return Ok(()),
        };

        // If already showing a result status, any key closes the modal
        if modal.status.is_some() {
            self.bug_report_modal = None;
            return Ok(());
        }

        match key.code {
            KeyCode::Esc => {
                self.bug_report_modal = None;
            }
            KeyCode::Left => {
                let modal = self.bug_report_modal.as_mut().unwrap();
                modal.report_type = modal.report_type.prev();
            }
            KeyCode::Right | KeyCode::Tab => {
                let modal = self.bug_report_modal.as_mut().unwrap();
                modal.report_type = modal.report_type.next();
            }
            KeyCode::Backspace => {
                let modal = self.bug_report_modal.as_mut().unwrap();
                modal.description.pop();
            }
            KeyCode::Enter => {
                let modal = self.bug_report_modal.as_mut().unwrap();
                modal.description.push('\n');
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.submit_bug_report()?;
            }
            KeyCode::Char(c) => {
                let modal = self.bug_report_modal.as_mut().unwrap();
                modal.description.push(c);
            }
            _ => {}
        }
        Ok(())
    }

    fn submit_bug_report(&mut self) -> Result<()> {
        let modal = match self.bug_report_modal.as_ref() {
            Some(m) => m.clone(),
            None => return Ok(()),
        };

        if modal.description.trim().is_empty() {
            if let Some(m) = self.bug_report_modal.as_mut() {
                m.status = Some("Description cannot be empty.".to_string());
            }
            return Ok(());
        }

        if !self.config.sync_enabled {
            if let Some(m) = self.bug_report_modal.as_mut() {
                m.status = Some("Sync must be enabled to send reports. Enable it in [S] Sync.".to_string());
            }
            return Ok(());
        }

        let (username, class_name, level) = if let Some(ref u) = self.user {
            (u.username.clone(), u.class.name().to_string(), u.level)
        } else {
            ("Unknown".to_string(), "None".to_string(), 0)
        };

        let version = env!("CARGO_PKG_VERSION");
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let term = std::env::var("TERM").unwrap_or_else(|_| "unknown".to_string());
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_else(|_| "unknown".to_string());

        let payload = serde_json::json!({
            "report_type": modal.report_type.label(),
            "description": modal.description.trim(),
            "version": version,
            "os": os,
            "arch": arch,
            "term": term,
            "term_program": term_program,
            "username": username,
            "class": class_name,
            "level": level,
        });

        let client = crate::services::api_client::ApiClient::new(
            &self.server_url,
            self.identity.clone(),
            &self.device_id,
        );

        match client.send_request("POST", "report", &payload.to_string()) {
            Ok(_) => {
                if let Some(m) = self.bug_report_modal.as_mut() {
                    m.status = Some("Report sent. Thank you, hero.".to_string());
                }
            }
            Err(e) => {
                if let Some(m) = self.bug_report_modal.as_mut() {
                    m.status = Some(format!("Failed to send: {e}"));
                }
            }
        }

        Ok(())
    }
}

/// Build a `ProjectStats` snapshot for the given project, used by template-based milestone checks.
fn build_project_stats(
    p_id: Uuid,
    project_created_at: DateTime<Utc>,
    db: &Database,
) -> ProjectStats {
    let all_tasks = db.get_tasks().unwrap_or_default();
    let all_notes = db.get_notes().unwrap_or_default();
    let all_journals = db.get_journal_entries().unwrap_or_default();
    let streak = db.get_streak().unwrap_or(crate::models::Streak {
        id: String::new(),
        current_streak: 0,
        best_streak: 0,
        last_active_day: None,
    });

    ProjectStats {
        project_age_days: (Utc::now() - project_created_at).num_days(),
        completed_tasks_in_project: all_tasks
            .iter()
            .filter(|t| t.project_id == Some(p_id) && t.completed && t.parent_task_id.is_none())
            .count() as i64,
        notes_in_project: all_notes
            .iter()
            .filter(|n| n.project_id == Some(p_id))
            .count() as i64,
        journal_entries_in_project: all_journals
            .iter()
            .filter(|j| j.project_id == p_id)
            .count() as i64,
        active_days_in_project: db.get_active_days_for_project(p_id).unwrap_or(0),
        total_completed_tasks: all_tasks.iter().filter(|t| t.completed && t.parent_task_id.is_none()).count() as i64,
        current_streak: streak.current_streak as i64,
        focus_sessions_total: db.get_focus_sessions().unwrap_or_default().len() as i64,
        daily_adventures_completed: db.get_daily_adventures_completed_count().unwrap_or(0),
    }
}

#[cfg(test)]
mod app_tests {
    use super::*;
    use crate::models::Season;
    use std::path::Path;

    #[test]
    fn test_parse_due_date_input() {
        let db_file = Path::new("test_questline_date.db");
        let _ = std::fs::remove_file(db_file);
        let app = App::new(db_file).unwrap();

        let parsed_today = app.parse_due_date_input("today").unwrap();
        assert_eq!(parsed_today.date_naive(), Utc::now().date_naive());

        let parsed_tomorrow = app.parse_due_date_input("tomorrow").unwrap();
        assert_eq!(
            parsed_tomorrow.date_naive(),
            Utc::now().date_naive() + chrono::Duration::days(1)
        );

        let parsed_in_5_days = app.parse_due_date_input("in 5 days").unwrap();
        assert_eq!(
            parsed_in_5_days.date_naive(),
            Utc::now().date_naive() + chrono::Duration::days(5)
        );

        let parsed_iso = app.parse_due_date_input("2026-06-25").unwrap();
        assert_eq!(
            parsed_iso.date_naive(),
            NaiveDate::from_ymd_opt(2026, 6, 25).unwrap()
        );

        assert!(app.parse_due_date_input("invalid").is_none());

        // Clean up
        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_zen_tree_growth_and_watering() {
        let db_file = Path::new("test_questline_tree.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Check initial state
        let tree = app.db.get_zen_tree().unwrap();
        assert_eq!(tree.growth, 0);
        assert_eq!(tree.stage, 1);
        assert_eq!(tree.water_today, 0);

        // Water tree once (morning)
        let morning_time = chrono::Local::now()
            .with_time(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap())
            .unwrap();
        app.water_tree_at(morning_time).unwrap();
        let tree = app.db.get_zen_tree().unwrap();
        assert_eq!(tree.water_today, 1);
        assert_eq!(tree.growth, 1);

        // Water tree twice (afternoon)
        let afternoon_time = chrono::Local::now()
            .with_time(chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap())
            .unwrap();
        app.water_tree_at(afternoon_time).unwrap();
        let tree = app.db.get_zen_tree().unwrap();
        assert_eq!(tree.water_today, 2);
        assert_eq!(tree.growth, 2);

        // Water tree thrice (afternoon again, should fail/block)
        let afternoon_time_2 = chrono::Local::now()
            .with_time(chrono::NaiveTime::from_hms_opt(16, 0, 0).unwrap())
            .unwrap();
        app.water_tree_at(afternoon_time_2).unwrap();
        let tree = app.db.get_zen_tree().unwrap();
        assert_eq!(tree.water_today, 2);
        assert_eq!(tree.growth, 2);

        // Grow tree manually to test evolution
        app.grow_tree(8).unwrap(); // total 10 growth
        let tree = app.db.get_zen_tree().unwrap();
        assert_eq!(tree.growth, 10);
        assert_eq!(tree.stage, 2); // evolved to sprout

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_streak_actions() {
        let db_file = Path::new("test_questline_streak.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Initial streak is 0
        let streak = app.db.get_streak().unwrap();
        assert_eq!(streak.current_streak, 0);

        // Perform productive action
        app.complete_productive_action().unwrap();
        let streak = app.db.get_streak().unwrap();
        assert_eq!(streak.current_streak, 1);
        assert_eq!(streak.best_streak, 1);
        assert!(streak.last_active_day.is_some());

        // Perform another productive action today (should not increase streak)
        app.complete_productive_action().unwrap();
        let streak = app.db.get_streak().unwrap();
        assert_eq!(streak.current_streak, 1);

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_specialization_xp_multiplier() {
        let db_file = Path::new("test_questline_spec.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Seed user
        let new_user = User {
            id: Uuid::new_v4(),
            username: "Test User".to_string(),
            class: ClassType::CodeWarlock,
            level: 1,
            xp: 0,
            created_at: Utc::now(),
            specialization: None,
        };
        app.db.insert_user(&new_user).unwrap();
        app.user = Some(new_user);
        app.reload_data().unwrap();

        // 1. Initially user is Level 1, no specialization
        let mut user = app.db.get_user().unwrap().unwrap();
        assert_eq!(user.level, 1);
        assert!(user.specialization.is_none());

        // Set user level to 10
        user.level = 10;
        app.db.update_user(&user).unwrap();
        app.reload_data().unwrap();

        // Check it loaded
        let user = app.user.as_ref().unwrap();
        assert_eq!(user.level, 10);

        // 2. Select specialization
        // For Code Warlock, "Bug Hunter" is valid (+10% XP from Task Completion)
        let mut user_mod = app.user.clone().unwrap();
        user_mod.specialization = Some("Bug Hunter".to_string());
        app.db.update_user(&user_mod).unwrap();
        app.reload_data().unwrap();

        // Verify specialization updated
        assert_eq!(
            app.user.as_ref().unwrap().specialization.as_deref(),
            Some("Bug Hunter")
        );

        // 3. Complete a task and check XP gain
        // Base task completion is 25 XP. With +10% it should be round(25 * 1.1) = 28 XP.
        let task_id = Uuid::new_v4();
        let proj = Project {
            id: Uuid::new_v4(),
            name: "Test Project".to_string(),
            description: None,
            archived: false,
            completed: false,
            created_at: Utc::now(),
            owner_identity: None,
            owner_username: None,
            is_shared: false,
        };
        app.db.insert_project(&proj).unwrap();
        let t = Task {
            id: task_id,
            project_id: Some(proj.id),
            title: "Task Quest".to_string(),
            description: None,
            priority: TaskPriority::Medium,
            due_date: None,
            completed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            owner_identity: None,
            owner_username: None,
            parent_task_id: None,
        };
        app.db.insert_task(&t).unwrap();
        app.active_project_id = Some(proj.id);
        app.selected_task_idx = 0;
        app.workspace_tab_idx = 0;
        app.active_screen = ActiveScreen::Workspace;
        app.reload_data().unwrap();

        // Set key code space to toggle completion
        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()))
            .unwrap();

        // Check user XP. Level 10 has xp = 0.
        // Granting 28 XP should make user.xp = 28.
        let user_after = app.db.get_user().unwrap().unwrap();
        assert_eq!(user_after.xp, 28);

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_focus_session_xp_rewards() {
        let db_file = Path::new("test_questline_focus.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Seed user
        let new_user = User {
            id: Uuid::new_v4(),
            username: "Test User".to_string(),
            class: ClassType::CodeWarlock,
            level: 1,
            xp: 0,
            created_at: Utc::now(),
            specialization: None,
        };
        app.db.insert_user(&new_user).unwrap();
        app.user = Some(new_user);
        app.reload_data().unwrap();

        let season = Season::current();
        let multiplier: f64 = if season == Season::Summer { 1.2 } else { 1.0 };
        let xp_15 = (10.0 * multiplier).round() as i32;
        let xp_25 = (20.0 * multiplier).round() as i32;
        let xp_100 = (80.0 * multiplier).round() as i32;

        // 1. Verify 15m focus session yields 10 XP (modified by season)
        app.active_focus_session = Some(ActiveFocusSession {
            start_time: Utc::now() - chrono::Duration::seconds(15 * 60 + 2), // 15 mins ago
            duration_mins: 15,
            project_id: None,
            task_id: None,
            soundscape: "Silent".to_string(),
        });
        app.tick_focus_session().unwrap();
        assert!(app.active_focus_session.is_none());
        assert_eq!(app.db.get_user().unwrap().unwrap().xp, xp_15);

        // 2. Verify 25m focus session yields 20 XP (modified by season)
        app.active_focus_session = Some(ActiveFocusSession {
            start_time: Utc::now() - chrono::Duration::seconds(25 * 60 + 2), // 25 mins ago
            duration_mins: 25,
            project_id: None,
            task_id: None,
            soundscape: "Silent".to_string(),
        });
        app.tick_focus_session().unwrap();
        // Since XP accumulates
        assert_eq!(app.db.get_user().unwrap().unwrap().xp, xp_15 + xp_25);

        // 3. Verify custom 100m focus session yields (100 * 0.8) = 80 XP (modified by season)
        app.active_focus_session = Some(ActiveFocusSession {
            start_time: Utc::now() - chrono::Duration::seconds(100 * 60 + 2),
            duration_mins: 100,
            project_id: None,
            task_id: None,
            soundscape: "Silent".to_string(),
        });
        app.tick_focus_session().unwrap();
        assert_eq!(
            app.db.get_user().unwrap().unwrap().xp,
            xp_15 + xp_25 + xp_100
        );

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_focus_session_soundscape_bonuses() {
        let db_file = Path::new("test_questline_soundscape_bonuses.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Seed user
        let new_user = User {
            id: Uuid::new_v4(),
            username: "Audio Monk".to_string(),
            class: ClassType::CodeWarlock,
            level: 1,
            xp: 0,
            created_at: Utc::now(),
            specialization: None,
        };
        app.db.insert_user(&new_user).unwrap();
        app.user = Some(new_user);
        app.reload_data().unwrap();

        // Ensure ZenTree exists in DB
        let initial_tree = app.db.get_zen_tree().unwrap();
        assert_eq!(initial_tree.growth, 0);

        // 1. Forest Sounds: Grow Zen Tree (+1 extra growth)
        app.active_focus_session = Some(ActiveFocusSession {
            start_time: Utc::now() - chrono::Duration::seconds(15 * 60 + 2),
            duration_mins: 15,
            project_id: None,
            task_id: None,
            soundscape: "Forest Sounds".to_string(),
        });
        app.tick_focus_session().unwrap();
        let tree = app.db.get_zen_tree().unwrap();
        assert_eq!(tree.growth, 1);

        // 2. Rain Sounds: Auto-waters Tree (+1 water count & +1 growth)
        app.active_focus_session = Some(ActiveFocusSession {
            start_time: Utc::now() - chrono::Duration::seconds(15 * 60 + 2),
            duration_mins: 15,
            project_id: None,
            task_id: None,
            soundscape: "Rain Sounds".to_string(),
        });
        app.tick_focus_session().unwrap();
        let tree = app.db.get_zen_tree().unwrap();
        assert_eq!(tree.water_today, 1);
        assert_eq!(tree.growth, 2);

        // 3. Ocean Waves: Gained normal 10 XP + 5 extra focus XP = 15 XP (modified by season)
        let current_xp = app.db.get_user().unwrap().unwrap().xp;
        app.active_focus_session = Some(ActiveFocusSession {
            start_time: Utc::now() - chrono::Duration::seconds(15 * 60 + 2),
            duration_mins: 15,
            project_id: None,
            task_id: None,
            soundscape: "Ocean Waves".to_string(),
        });
        app.tick_focus_session().unwrap();
        let after_xp = app.db.get_user().unwrap().unwrap().xp;
        let season = Season::current();
        let multiplier: f64 = if season == Season::Summer { 1.2 } else { 1.0 };
        let expected_ocean_waves_gain = ((10.0 + 5.0) * multiplier).round() as i32;
        assert_eq!(after_xp - current_xp, expected_ocean_waves_gain);

        // 4. White Noise: Pad duration by 10%
        app.active_focus_session = Some(ActiveFocusSession {
            start_time: Utc::now() - chrono::Duration::seconds(20 * 60 + 2),
            duration_mins: 20,
            project_id: None,
            task_id: None,
            soundscape: "White Noise".to_string(),
        });
        app.tick_focus_session().unwrap();
        let sessions = app.db.get_focus_sessions().unwrap();
        let white_noise_sess = sessions
            .iter()
            .find(|s| s.soundscape == "White Noise")
            .unwrap();
        // 20 * 1.1 = 22 mins
        assert_eq!(white_noise_sess.duration_mins, 22);

        // 5. Test Atmosphere Master Achievement (Use all 8 soundscapes)
        // We have used Forest Sounds, Rain Sounds, Ocean Waves, White Noise. Let's seed the rest.
        let soundscapes_list = ["LoFi Radio", "Ambient Radio", "Brown Noise", "Silent"];
        for sc in soundscapes_list {
            app.active_focus_session = Some(ActiveFocusSession {
                start_time: Utc::now() - chrono::Duration::seconds(15 * 60 + 2),
                duration_mins: 15,
                project_id: None,
                task_id: None,
                soundscape: sc.to_string(),
            });
            app.tick_focus_session().unwrap();
        }

        let achievements = app.db.get_achievements().unwrap();
        let master_ach = achievements
            .iter()
            .find(|a| a.id == "master_atmosphere")
            .unwrap();
        assert!(master_ach.unlocked_at.is_some());

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_milestone_completion() {
        let db_file = Path::new("test_questline_milestone.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Seed user
        let new_user = User {
            id: Uuid::new_v4(),
            username: "Test User".to_string(),
            class: ClassType::CodeWarlock,
            level: 1,
            xp: 0,
            created_at: Utc::now(),
            specialization: None,
        };
        app.db.insert_user(&new_user).unwrap();
        app.user = Some(new_user);
        app.reload_data().unwrap();

        let proj = Project {
            id: Uuid::new_v4(),
            name: "Milestone Project".to_string(),
            description: None,
            archived: false,
            completed: false,
            created_at: Utc::now(),
            owner_identity: None,
            owner_username: None,
            is_shared: false,
        };
        app.db.insert_project(&proj).unwrap();
        app.active_project_id = Some(proj.id);

        let mil = Milestone {
            id: Uuid::new_v4(),
            project_id: proj.id,
            name: "Deploy Alpha".to_string(),
            description: None,
            completed: false,
            xp_reward: 100,
            // Use a very old created_at so legacy age check passes
            created_at: Utc::now() - chrono::Duration::days(10),
            tier: 0,
            template_id: String::new(), // legacy milestone — uses hardcoded checks
        };
        app.db.insert_milestone(&mil).unwrap();
        app.reload_data().unwrap();

        // Zen Tree health/growth before
        let tree_before = app.db.get_zen_tree().unwrap();
        assert_eq!(tree_before.growth, 0);

        // Complete milestone (legacy path; requirements won't block since we seed tasks/journals below)
        // For the test we rely on legacy check: 3 days old (pass), 3 completed tasks, 1 journal
        // We need to seed those too for the test to complete the milestone.
        // Actually the test just calls toggle and expects completion — we need to seed data.
        // Seed 3 completed tasks and 1 journal for the project.
        use crate::models::Task;
        for i in 0..3 {
            let t = Task {
                id: Uuid::new_v4(),
                project_id: Some(proj.id),
                title: format!("Task {}", i),
                description: None,
                completed: true,
                priority: crate::models::TaskPriority::Medium,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                due_date: None,
                owner_identity: None,
                owner_username: None,
                parent_task_id: None,
            };
            app.db.insert_task(&t).unwrap();
        }
        use crate::models::JournalEntry;
        let je = JournalEntry {
            id: Uuid::new_v4(),
            project_id: proj.id,
            content: "Journal entry".to_string(),
            entry_date: chrono::Utc::now().date_naive(),
            created_at: Utc::now(),
            visibility: "Private".to_string(),
            author_username: String::new(),
        };
        app.db.insert_journal_entry(&je).unwrap();
        app.reload_data().unwrap();

        app.toggle_milestone(mil.id).unwrap();

        // 1. Check milestone is completed
        let mil_after = app.db.get_milestones_for_project(proj.id).unwrap();
        assert!(mil_after[0].completed);

        // 2. Check user has XP rewarded (xp_reward = 100 from template)
        let user = app.db.get_user().unwrap().unwrap();
        assert_eq!(user.xp, 100);

        // 3. Check Zen Tree growth increased by 5
        let tree_after = app.db.get_zen_tree().unwrap();
        assert_eq!(tree_after.growth, 5);

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_ritual_completion_and_streak() {
        let db_file = Path::new("test_questline_ritual.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Seed user
        let new_user = User {
            id: Uuid::new_v4(),
            username: "Test User".to_string(),
            class: ClassType::CodeWarlock,
            level: 1,
            xp: 0,
            created_at: Utc::now(),
            specialization: None,
        };
        app.db.insert_user(&new_user).unwrap();
        app.user = Some(new_user);
        app.reload_data().unwrap();

        let rit = Ritual {
            id: "drink-water".to_string(),
            name: "Drink 2L Water".to_string(),
            description: Some("Stay hydrated during focus".to_string()),
            frequency: "Daily".to_string(),
            reward_xp: 30,
            created_at: Utc::now(),
        };
        app.db.insert_ritual(&rit).unwrap();
        app.reload_data().unwrap();

        // Completion today
        app.complete_ritual("drink-water").unwrap();

        // 1. Verify XP was granted
        let user = app.db.get_user().unwrap().unwrap();
        assert_eq!(user.xp, 30);

        // 2. Verify duplicate completion today is blocked
        app.complete_ritual("drink-water").unwrap();
        // XP should still be 30
        let user2 = app.db.get_user().unwrap().unwrap();
        assert_eq!(user2.xp, 30);

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_shortcut_conflict_resolution() {
        let db_file = Path::new("test_questline_shortcuts.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Seed user & project
        let new_user = User {
            id: Uuid::new_v4(),
            username: "Test User".to_string(),
            class: ClassType::CodeWarlock,
            level: 1,
            xp: 0,
            created_at: Utc::now(),
            specialization: None,
        };
        app.db.insert_user(&new_user).unwrap();
        app.user = Some(new_user);

        let proj = Project {
            id: Uuid::new_v4(),
            name: "Shortcut Test Project".to_string(),
            description: None,
            archived: false,
            completed: false,
            created_at: Utc::now(),
            owner_identity: None,
            owner_username: None,
            is_shared: false,
        };
        app.db.insert_project(&proj).unwrap();
        app.reload_data().unwrap();

        // Test 1: On Projects Screen, 'n' opens New Project Modal
        app.active_screen = ActiveScreen::Projects;
        app.modal_state = ModalType::None;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()))
            .unwrap();
        match app.modal_state {
            ModalType::NewProject { .. } => {}
            _ => panic!("Expected ModalType::NewProject"),
        }

        // Test 2: In Workspace Tab 0 (Tasks), 'n' opens New Task Modal
        app.active_screen = ActiveScreen::Workspace;
        app.active_project_id = Some(proj.id);
        app.workspace_tab_idx = 0;
        app.modal_state = ModalType::None;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()))
            .unwrap();
        match app.modal_state {
            ModalType::NewTask { .. } => {}
            _ => panic!("Expected ModalType::NewTask"),
        }

        // Test 3: In Workspace Tab 2 (Journal), 'n' opens New Journal Entry Modal
        app.workspace_tab_idx = 2;
        app.modal_state = ModalType::None;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()))
            .unwrap();
        match app.modal_state {
            ModalType::NewJournalEntry { .. } => {}
            _ => panic!("Expected ModalType::NewJournalEntry"),
        }

        // Test 4: In Workspace Tab 3 (Milestones), 'm' opens Milestone Tier Select Modal
        app.workspace_tab_idx = 3;
        app.modal_state = ModalType::None;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::empty()))
            .unwrap();
        match app.modal_state {
            ModalType::MilestoneTierSelect { .. } => {}
            _ => panic!("Expected ModalType::MilestoneTierSelect"),
        }

        // Test 5: In Workspace Tab 0 (Tasks), 's' sorts tasks
        app.workspace_tab_idx = 0;
        app.modal_state = ModalType::None;
        app.task_sort = "CreatedDate".to_string();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.task_sort, "DueDate");

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_ritual_modal_navigation_and_spaces() {
        let db_file = Path::new("test_questline_ritual_modal.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // 1. Open NewRitual modal
        app.modal_state = ModalType::NewRitual {
            name: String::new(),
            desc: String::new(),
            frequency_idx: 0,
            reward_xp: "20".to_string(),
            focus_idx: 0,
        };

        // 2. Type characters including space on focus_idx 0 (name)
        app.handle_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::empty()))
            .unwrap();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()))
            .unwrap();
        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()))
            .unwrap();
        app.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty()))
            .unwrap();

        match app.modal_state {
            ModalType::NewRitual {
                ref name,
                focus_idx,
                ..
            } => {
                assert_eq!(name, "Ha b");
                assert_eq!(focus_idx, 0);
            }
            _ => panic!("Expected ModalType::NewRitual"),
        }

        // 3. Cycle using Tab
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()))
            .unwrap();
        match app.modal_state {
            ModalType::NewRitual { focus_idx, .. } => {
                assert_eq!(focus_idx, 1);
            }
            _ => panic!("Expected ModalType::NewRitual"),
        }

        // 4. Cycle backward using BackTab (Shift+Tab)
        app.handle_key_event(KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()))
            .unwrap();
        match app.modal_state {
            ModalType::NewRitual { focus_idx, .. } => {
                assert_eq!(focus_idx, 0);
            }
            _ => panic!("Expected ModalType::NewRitual"),
        }

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_global_search_and_palette_shortcuts() {
        let db_file = Path::new("test_questline_global_shortcuts.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // 1. Press Ctrl+P globally to open CommandPalette modal
        app.modal_state = ModalType::None;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL))
            .unwrap();
        match app.modal_state {
            ModalType::CommandPalette { .. } => {}
            _ => panic!("Expected ModalType::CommandPalette"),
        }

        // 2. Escape to close
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.modal_state, ModalType::None);

        // 2b. Press : globally to open CommandPalette modal
        app.handle_key_event(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::empty()))
            .unwrap();
        match app.modal_state {
            ModalType::CommandPalette { .. } => {}
            _ => panic!("Expected ModalType::CommandPalette via ':'"),
        }

        // Escape to close
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.modal_state, ModalType::None);

        // 3. Press Ctrl+K to open CommandPalette
        app.handle_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL))
            .unwrap();
        match app.modal_state {
            ModalType::CommandPalette { .. } => {}
            _ => panic!("Expected ModalType::CommandPalette"),
        }

        // 4. Escape to close
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.modal_state, ModalType::None);

        // 5. Press F1 to open CommandPalette
        app.handle_key_event(KeyEvent::new(KeyCode::F(1), KeyModifiers::empty()))
            .unwrap();
        match app.modal_state {
            ModalType::CommandPalette { .. } => {}
            _ => panic!("Expected ModalType::CommandPalette"),
        }

        // 6. Escape to close
        app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.modal_state, ModalType::None);

        // Test fuzzy matching
        let actions = app.get_available_command_actions("proj");
        assert!(!actions.is_empty());
        assert_eq!(actions[0].name, "Open Projects");

        let actions = app.get_available_command_actions("sync");
        assert!(!actions.is_empty());
        assert_eq!(actions[0].name, "Open Sync");

        let actions = app.get_available_command_actions("char");
        assert!(!actions.is_empty());
        assert_eq!(actions[0].name, "Open Character");

        // 7. Press ? to open About screen (since we are not in text entry)
        app.handle_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::About);
        assert_eq!(app.active_tab_idx, 13);

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_choose_dynamic_quotes_isolation() {
        let db_file = Path::new("test_questline_quotes.db");
        let _ = std::fs::remove_file(db_file);
        let app = App::new(db_file).unwrap();

        // 1. First time Questline is opened, user is None -> shows a random quote with its author
        let (quote, author, class_opt) = App::choose_dynamic_quote(&None, &app.db);
        assert!(!author.is_empty());
        assert!(!quote.is_empty());
        assert!(class_opt.is_none());

        // 2. User exists and class is selected -> shows both a random quote and class quote
        let user = User {
            id: uuid::Uuid::new_v4(),
            username: "Tester".to_string(),
            class: ClassType::ArchAccountant,
            level: 1,
            xp: 0,
            created_at: chrono::Utc::now(),
            specialization: None,
        };
        let (quote_q, author_q, class_opt_2) = App::choose_dynamic_quote(&Some(user), &app.db);
        assert!(!author_q.is_empty());
        assert!(!quote_q.is_empty());
        assert!(class_opt_2.is_some());
        let class_vals = class_opt_2.unwrap();
        assert_eq!(class_vals.1, "Arch Accountant");
        assert!(!class_vals.0.is_empty());

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_ritual_constraints_validation() {
        let db_file = Path::new("test_questline_ritual_constraints.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // 1. Setup mock user so we can access dashboard screen
        let user = User {
            id: uuid::Uuid::new_v4(),
            username: "Ritual Guy".to_string(),
            class: ClassType::TaskPaladin,
            level: 1,
            xp: 0,
            created_at: chrono::Utc::now(),
            specialization: None,
        };
        app.db.insert_user(&user).unwrap();
        app.user = Some(user);
        app.active_screen = ActiveScreen::Dashboard;

        // 2. Initial state: verify default db has at least 1 ritual
        let rituals_init = app.db.get_rituals().unwrap();
        assert!(!rituals_init.is_empty());

        // Delete all but 1 ritual so we can test the last-ritual deletion guard
        while app.db.get_rituals().unwrap().len() > 1 {
            let list = app.db.get_rituals().unwrap();
            app.db.delete_ritual(&list[0].id).unwrap();
        }

        let rituals_init = app.db.get_rituals().unwrap();
        assert_eq!(rituals_init.len(), 1);

        // 3. Try to delete the last ritual using Delete key
        app.selected_ritual_idx = 0;
        app.notifications.clear();
        app.handle_key_event(KeyEvent::new(KeyCode::Delete, KeyModifiers::empty()))
            .unwrap();

        // Check deletion is rejected: database still has the ritual and warning notification is shown
        let rituals_after = app.db.get_rituals().unwrap();
        assert_eq!(rituals_after.len(), rituals_init.len());
        assert!(!app.notifications.is_empty());
        assert!(app
            .notifications
            .last()
            .unwrap()
            .message
            .contains("at least one"));

        // 4. Try to create a ritual with 150 XP (over 100 XP cheating limit)
        app.modal_state = ModalType::NewRitual {
            name: "Super Cheat Ritual".to_string(),
            desc: "Trying to earn lots of XP".to_string(),
            frequency_idx: 0,
            reward_xp: "150".to_string(),
            focus_idx: 3, // focus is on XP input field
        };

        // Submit via Enter key
        app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()))
            .unwrap();

        // Check creation is rejected: modal is still open, database count is same, warning is posted
        match app.modal_state {
            ModalType::NewRitual { .. } => {}
            _ => panic!("Expected modal to remain open because of invalid XP value!"),
        }
        let rituals_after_creation = app.db.get_rituals().unwrap();
        assert_eq!(rituals_after_creation.len(), rituals_init.len());
        assert!(app
            .notifications
            .last()
            .unwrap()
            .message
            .contains("cannot exceed 100 XP"));

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_particle_trigger_system() {
        let db_file = Path::new("test_questline_particles.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Initially no ticks remaining and no particles
        assert_eq!(app.ambient_particles_ticks_remaining, 0);
        assert!(app.ambient_particles.is_empty());

        // Triggering sets ticks remaining
        app.trigger_ambient_particles();
        assert_eq!(app.ambient_particles_ticks_remaining, 60);

        // Ticking down decrements ticks remaining
        app.tick_particles();
        assert_eq!(app.ambient_particles_ticks_remaining, 59);

        // Verify ticking down to 0
        for _ in 0..59 {
            app.tick_particles();
        }
        assert_eq!(app.ambient_particles_ticks_remaining, 0);

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_tab_switching_keys() {
        let db_file = Path::new("test_questline_tabs.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // Start on Dashboard
        app.active_screen = ActiveScreen::Dashboard;

        // Key '1' -> Dashboard
        app.handle_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Dashboard);
        assert_eq!(app.active_tab_idx, 0);

        // Key '2' -> Projects
        app.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Projects);
        assert_eq!(app.active_tab_idx, 1);

        // Key '3' -> Character
        app.handle_key_event(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Character);
        assert_eq!(app.active_tab_idx, 2);

        // Key '4' -> Library
        app.handle_key_event(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Library);
        assert_eq!(app.active_tab_idx, 4);

        // Key '5' -> Soundscapes
        app.handle_key_event(KeyEvent::new(KeyCode::Char('5'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Soundscapes);
        assert_eq!(app.active_tab_idx, 7);

        // Key '6' -> SyncSettings
        app.handle_key_event(KeyEvent::new(KeyCode::Char('6'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::SyncSettings);
        assert_eq!(app.active_tab_idx, 12);

        // Key '7' -> Fellowship
        app.handle_key_event(KeyEvent::new(KeyCode::Char('7'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Fellowship);
        assert_eq!(app.active_tab_idx, 8);

        // Key '8' -> GreatChronicle
        app.handle_key_event(KeyEvent::new(KeyCode::Char('8'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::GreatChronicle);
        assert_eq!(app.active_tab_idx, 14);

        // Key '9' -> should NOT switch screens
        app.handle_key_event(KeyEvent::new(KeyCode::Char('9'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::GreatChronicle);

        // Key '0' -> should NOT switch screens
        app.handle_key_event(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::GreatChronicle);

        // Key 'Y' -> should NOT switch screens
        app.handle_key_event(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::GreatChronicle);

        // Key 'S' -> SyncSettings
        app.handle_key_event(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::SyncSettings);
        assert_eq!(app.active_tab_idx, 12);

        // Key 'c' on SyncSettings -> should copy key and NOT switch screens
        app.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::SyncSettings);
        assert_eq!(app.active_tab_idx, 12);

        // Key 'A' on Dashboard -> should NOT switch screens
        app.active_screen = ActiveScreen::Dashboard;
        app.active_tab_idx = 0;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Dashboard);
        assert_eq!(app.active_tab_idx, 0);

        // Key 'A' on Projects -> should switch to Archive
        app.active_screen = ActiveScreen::Projects;
        app.active_tab_idx = 1;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Archive);
        assert_eq!(app.active_tab_idx, 10);

        // Key 'F' on Dashboard -> should switch to Fellowship
        app.active_screen = ActiveScreen::Dashboard;
        app.active_tab_idx = 0;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Fellowship);
        assert_eq!(app.active_tab_idx, 8);

        // Key 'F' on Projects -> should switch to Focus
        app.active_screen = ActiveScreen::Projects;
        app.active_tab_idx = 1;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Focus);
        assert_eq!(app.active_tab_idx, 6);

        // Key 'S' on Dashboard -> should switch to SyncSettings
        app.active_screen = ActiveScreen::Dashboard;
        app.active_tab_idx = 0;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::SyncSettings);
        assert_eq!(app.active_tab_idx, 12);

        // Key 'S' on Projects -> should switch to Fellowship
        app.active_screen = ActiveScreen::Projects;
        app.active_tab_idx = 1;
        app.handle_key_event(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.active_screen, ActiveScreen::Fellowship);
        assert_eq!(app.active_tab_idx, 8);

        let _ = std::fs::remove_file(db_file);
    }

    #[test]
    fn test_quit_confirmation_modal() {
        let db_file = Path::new("test_questline_quit.db");
        let _ = std::fs::remove_file(db_file);
        let mut app = App::new(db_file).unwrap();

        // 1. Pressing 'q' opens the confirmation modal with a non-empty quote
        assert_eq!(app.modal_state, ModalType::None);
        assert!(!app.should_quit);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()))
            .unwrap();

        match &app.modal_state {
            ModalType::QuitConfirm { quote } => {
                assert!(!quote.is_empty());
            }
            _ => panic!("Expected ModalType::QuitConfirm modal state"),
        }
        assert!(!app.should_quit);

        // 2. Pressing 'n' inside QuitConfirm modal closes the modal and keeps should_quit = false
        app.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.modal_state, ModalType::None);
        assert!(!app.should_quit);

        // 3. Pressing 'Q' also opens it
        app.handle_key_event(KeyEvent::new(KeyCode::Char('Q'), KeyModifiers::empty()))
            .unwrap();
        match &app.modal_state {
            ModalType::QuitConfirm { quote } => {
                assert!(!quote.is_empty());
            }
            _ => panic!("Expected ModalType::QuitConfirm modal state"),
        }

        // 4. Pressing 'y' confirms quit
        app.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()))
            .unwrap();
        assert_eq!(app.modal_state, ModalType::None);
        assert!(app.should_quit);

        let _ = std::fs::remove_file(db_file);
    }
}
