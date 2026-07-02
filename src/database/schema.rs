// ─────────────────────────────────────────────────────────────────────────────
// database/schema.rs — el esquema completo de la base de datos SQLite, todas las tablas
// ─────────────────────────────────────────────────────────────────────────────

// Aquí viven todas las CREATE TABLE del juego — si algo no existe aquí, no existe en ningún lado
pub const CREATE_TABLES_SQL: &str = "

-- Tablas del héroe: usuario, configuración y proyectos
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    class TEXT NOT NULL,
    level INTEGER NOT NULL DEFAULT 1,
    xp INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0,
    owner_identity TEXT,
    owner_username TEXT,
    is_shared INTEGER NOT NULL DEFAULT 0
);

-- Tareas y subtareas — el corazón de todo el rollo de productividad
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    title TEXT NOT NULL,
    description TEXT,
    due_date TEXT,
    completed INTEGER NOT NULL DEFAULT 0,
    priority TEXT NOT NULL DEFAULT 'Medium',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT '',
    owner_identity TEXT,
    owner_username TEXT,
    parent_task_id TEXT,
    xp_awarded INTEGER NOT NULL DEFAULT 0,
    recurrence TEXT,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL,
    FOREIGN KEY(parent_task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Codices y notas — el sistema de conocimiento del héroe
CREATE TABLE IF NOT EXISTS codices (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    title TEXT NOT NULL,
    markdown_content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    sharing_permission TEXT NOT NULL DEFAULT 'collaborative',
    codex_id TEXT,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL,
    FOREIGN KEY(codex_id) REFERENCES codices(id) ON DELETE SET NULL
);

-- Misiones diarias, XP y progresión del personaje
CREATE TABLE IF NOT EXISTS daily_quests (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    completed INTEGER NOT NULL DEFAULT 0,
    due_date TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS xp_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    xp_gained INTEGER NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS journal_entries (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    entry_date TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'Private',
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- El árbol zen — ojo, solo hay uno por usuario, no se crean varios
CREATE TABLE IF NOT EXISTS zen_tree (
    id TEXT PRIMARY KEY,
    growth INTEGER NOT NULL DEFAULT 0,
    health INTEGER NOT NULL DEFAULT 100,
    stage INTEGER NOT NULL DEFAULT 1,
    last_watered TEXT,
    water_today INTEGER NOT NULL DEFAULT 0,
    total_waterings INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS daily_adventures (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    quest_type TEXT NOT NULL,
    target_count INTEGER NOT NULL,
    current_count INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0,
    created_date TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS streaks (
    id TEXT PRIMARY KEY,
    current_streak INTEGER NOT NULL DEFAULT 0,
    best_streak INTEGER NOT NULL DEFAULT 0,
    last_active_day TEXT
);

CREATE TABLE IF NOT EXISTS achievements (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    unlocked_at TEXT
);

-- Sesiones de enfoque y rituales — la parte de hábitos del juego
CREATE TABLE IF NOT EXISTS focus_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    task_id TEXT,
    duration_mins INTEGER NOT NULL,
    xp_gained INTEGER NOT NULL,
    completed_at TEXT NOT NULL,
    soundscape TEXT NOT NULL DEFAULT 'Silent',
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS rituals (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    frequency TEXT NOT NULL,
    reward_xp INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ritual_history (
    ritual_id TEXT NOT NULL,
    completed_date TEXT NOT NULL,
    PRIMARY KEY(ritual_id, completed_date),
    FOREIGN KEY(ritual_id) REFERENCES rituals(id) ON DELETE CASCADE
);

-- Rasgos, hitos y reflexiones — el meta-RPG sobre el meta-RPG
CREATE TABLE IF NOT EXISTS traits (
    id TEXT PRIMARY KEY,
    unlocked_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS milestones (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    completed INTEGER NOT NULL DEFAULT 0,
    xp_reward INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS reflections (
    created_date TEXT PRIMARY KEY,
    what_went_well TEXT NOT NULL,
    what_can_improve TEXT NOT NULL
);

-- Tablas de sync — rastrea qué se mandó al servidor y qué no
CREATE TABLE IF NOT EXISTS sync_log (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    synced INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_sync_log_synced ON sync_log(synced);

CREATE TABLE IF NOT EXISTS revisions (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    revision_number INTEGER NOT NULL,
    content TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY,
    device_name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_sync TEXT
);

-- Tablas colaborativas — para cuando el juego se pone multijugador
CREATE TABLE IF NOT EXISTS project_members (
    project_id TEXT,
    user_identity TEXT,
    user_username TEXT NOT NULL,
    role TEXT NOT NULL,
    PRIMARY KEY (project_id, user_identity),
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS invitations (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    project_name TEXT NOT NULL,
    inviter_identity TEXT NOT NULL,
    inviter_username TEXT NOT NULL,
    invitee_identity TEXT NOT NULL,
    role TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Pending',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chronicle_messages (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    sender_identity TEXT NOT NULL,
    sender_username TEXT NOT NULL,
    content TEXT NOT NULL,
    message_type TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_reactions (
    message_id TEXT,
    user_identity TEXT,
    emoji TEXT,
    PRIMARY KEY (message_id, user_identity, emoji),
    FOREIGN KEY(message_id) REFERENCES chronicle_messages(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS activity_log (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    event_type TEXT NOT NULL,
    description TEXT NOT NULL,
    user_identity TEXT NOT NULL,
    user_username TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    notification_type TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    target_id TEXT,
    read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_assignments (
    task_id TEXT,
    user_identity TEXT,
    user_username TEXT NOT NULL,
    PRIMARY KEY (task_id, user_identity),
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS presence (
    user_identity TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    online INTEGER NOT NULL DEFAULT 0,
    last_seen TEXT NOT NULL,
    current_project TEXT,
    privacy_status TEXT NOT NULL DEFAULT 'Visible'
);

-- Crónicas — la historia del héroe y los eventos globales del realm
CREATE TABLE IF NOT EXISTS great_chronicle (
    id TEXT PRIMARY KEY,
    day_number INTEGER NOT NULL,
    entry_text TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS global_chronicle (
    id TEXT PRIMARY KEY,
    hero_name TEXT NOT NULL,
    event_type TEXT NOT NULL,
    description TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

-- Contenido de lore y clases — reliquias, títulos, historia del mundo
CREATE TABLE IF NOT EXISTS class_quests (
    class_name TEXT NOT NULL,
    unlock_level INTEGER NOT NULL,
    quest_name TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL,
    progress INTEGER NOT NULL DEFAULT 0,
    target INTEGER NOT NULL DEFAULT 1,
    lore_reward TEXT NOT NULL,
    PRIMARY KEY(class_name, unlock_level)
);

CREATE TABLE IF NOT EXISTS legendary_titles (
    title_id TEXT PRIMARY KEY,
    title_name TEXT NOT NULL,
    description TEXT NOT NULL,
    unlocked INTEGER NOT NULL DEFAULT 0,
    equipped INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS relics (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    unlocked INTEGER NOT NULL DEFAULT 0,
    unlocked_at TEXT
);

CREATE TABLE IF NOT EXISTS companion_lore (
    id TEXT PRIMARY KEY,
    story_text TEXT NOT NULL,
    timestamp TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS lore_library (
    id TEXT PRIMARY KEY,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    unlocked INTEGER NOT NULL DEFAULT 0,
    unlocked_at TEXT
);

-- Living Chapter tracking — cuánto ha aportado cada héroe al capítulo global
CREATE TABLE IF NOT EXISTS chapter_contribution_log (
    chapter_id TEXT NOT NULL,
    objective_type TEXT NOT NULL,
    last_sent_total INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (chapter_id, objective_type)
);

-- Índices — sin estos las queries se ven bonitas pero tardan un chingo
CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id);
CREATE INDEX IF NOT EXISTS idx_tasks_completed ON tasks(completed);
CREATE INDEX IF NOT EXISTS idx_notes_project_id ON notes(project_id);
CREATE INDEX IF NOT EXISTS idx_journal_entries_project_id ON journal_entries(project_id);
CREATE INDEX IF NOT EXISTS idx_focus_sessions_project_id ON focus_sessions(project_id);
CREATE INDEX IF NOT EXISTS idx_focus_sessions_task_id ON focus_sessions(task_id);
CREATE INDEX IF NOT EXISTS idx_milestones_project_id ON milestones(project_id);
CREATE INDEX IF NOT EXISTS idx_chronicle_messages_project_id ON chronicle_messages(project_id);
CREATE INDEX IF NOT EXISTS idx_great_chronicle_day ON great_chronicle(day_number);
CREATE INDEX IF NOT EXISTS idx_lore_library_category ON lore_library(category);
";
