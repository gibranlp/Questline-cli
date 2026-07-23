pub const CREATE_TABLES_SQL: &str = "

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

CREATE TABLE IF NOT EXISTS codices (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    parent_codex_id TEXT,
    collapsed INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY(parent_codex_id) REFERENCES codices(id) ON DELETE SET NULL
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

-- singleton por usuario — si intentas insertar más de uno, el juego se rompe
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

CREATE TABLE IF NOT EXISTS focus_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    task_id TEXT,
    duration_mins INTEGER NOT NULL,
    xp_gained INTEGER NOT NULL,
    completed_at TEXT NOT NULL,
    soundscape TEXT NOT NULL DEFAULT 'Silent',
    owner_identity TEXT,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS rituals (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    frequency TEXT NOT NULL,
    reward_xp INTEGER NOT NULL,
    daily_target INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ritual_history (
    ritual_id TEXT NOT NULL,
    completed_date TEXT NOT NULL,
    completion_count INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY(ritual_id, completed_date),
    FOREIGN KEY(ritual_id) REFERENCES rituals(id) ON DELETE CASCADE
);

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
    timestamp TEXT NOT NULL,
    hero_class TEXT
);

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

-- guarda el último total enviado al servidor por capítulo/objetivo, para mandar solo el delta en el siguiente sync
CREATE TABLE IF NOT EXISTS chapter_contribution_log (
    chapter_id TEXT NOT NULL,
    objective_type TEXT NOT NULL,
    last_sent_total INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (chapter_id, objective_type)
);

-- evita reprocesar eventos remotos que ya fueron aplicados — sin esto, cada sync replays toda la historia del servidor
CREATE TABLE IF NOT EXISTS processed_remote_events (
    id TEXT PRIMARY KEY,
    processed_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS hydration_log (
    log_date TEXT PRIMARY KEY,
    count INTEGER NOT NULL DEFAULT 0,
    reward_given INTEGER NOT NULL DEFAULT 0
);

-- índices base — presentes desde el inicio
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

-- índices de query hotpaths — los que faltaban según patrones reales de acceso
CREATE INDEX IF NOT EXISTS idx_tasks_due_date ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_completed_due ON tasks(completed, due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at);
CREATE INDEX IF NOT EXISTS idx_xp_events_timestamp ON xp_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_sync_log_entity ON sync_log(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_journal_entries_date ON journal_entries(entry_date);
CREATE INDEX IF NOT EXISTS idx_chronicle_messages_timestamp ON chronicle_messages(timestamp);
CREATE INDEX IF NOT EXISTS idx_activity_log_project ON activity_log(project_id);
CREATE INDEX IF NOT EXISTS idx_activity_log_timestamp ON activity_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_notifications_read ON notifications(read);
CREATE INDEX IF NOT EXISTS idx_global_chronicle_timestamp ON global_chronicle(timestamp);
CREATE INDEX IF NOT EXISTS idx_notes_updated_at ON notes(updated_at);
CREATE INDEX IF NOT EXISTS idx_focus_sessions_completed_at ON focus_sessions(completed_at);
";
