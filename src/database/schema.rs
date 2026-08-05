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
    updated_at TEXT NOT NULL DEFAULT '',
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
    set_date TEXT,
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
    updated_at TEXT NOT NULL DEFAULT '',
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
    updated_at TEXT NOT NULL DEFAULT '',
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
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS ritual_history (
    ritual_id TEXT NOT NULL,
    completed_date TEXT NOT NULL,
    completion_count INTEGER NOT NULL DEFAULT 1,
    completed_at TEXT,
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
    updated_at TEXT NOT NULL DEFAULT '',
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

CREATE TABLE IF NOT EXISTS project_encryption_keys (
    project_id TEXT PRIMARY KEY,
    routing_id TEXT UNIQUE NOT NULL,
    key_hex TEXT NOT NULL,
    key_version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Kept independently of projects so encrypted tombstones can still be routed after
-- the local entity (or the whole project) has been deleted.
CREATE TABLE IF NOT EXISTS project_encryption_key_history (
    routing_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    key_hex TEXT NOT NULL,
    key_version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS revoked_project_routes (
    routing_id TEXT PRIMARY KEY,
    replacement_routing_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    revoked_at TEXT NOT NULL
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
    created_at TEXT NOT NULL,
    routing_id TEXT,
    inviter_encryption_key TEXT,
    key_nonce TEXT,
    key_ciphertext TEXT,
    project_name_nonce TEXT,
    project_name_ciphertext TEXT,
    project_id_nonce TEXT,
    project_id_ciphertext TEXT
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

CREATE TABLE IF NOT EXISTS notification_states (
    notification_id TEXT PRIMARY KEY,
    read INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS task_assignments (
    task_id TEXT,
    user_identity TEXT,
    user_username TEXT NOT NULL,
    PRIMARY KEY (task_id, user_identity),
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS task_statuses (
    task_id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Backlog',
    changed_by_identity TEXT NOT NULL,
    changed_by_username TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS task_comments (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    author_identity TEXT NOT NULL,
    author_username TEXT NOT NULL,
    content TEXT NOT NULL,
    mentioned_identities TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    edited_at TEXT,
    deleted_at TEXT,
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS task_dependencies (
    task_id TEXT NOT NULL,
    depends_on_task_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    created_by_identity TEXT NOT NULL,
    created_by_username TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (task_id, depends_on_task_id),
    CHECK (task_id != depends_on_task_id),
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY(depends_on_task_id) REFERENCES tasks(id) ON DELETE CASCADE
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
    unlock_type TEXT NOT NULL DEFAULT '',
    unlock_chapter_id TEXT,
    unlock_display TEXT,
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
    reward_given INTEGER NOT NULL DEFAULT 0,
    last_drink_at TEXT
);

CREATE TABLE IF NOT EXISTS campaign_treasury (
    campaign_id TEXT PRIMARY KEY,
    overall_budget_minor INTEGER NOT NULL DEFAULT 0 CHECK(overall_budget_minor >= 0),
    currency_code TEXT NOT NULL DEFAULT 'USD',
    large_expense_threshold_minor INTEGER NOT NULL DEFAULT 100000,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(campaign_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ledger_categories (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    name TEXT NOT NULL COLLATE NOCASE,
    is_default INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(campaign_id, name),
    FOREIGN KEY(campaign_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS category_budgets (
    category_id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    amount_minor INTEGER NOT NULL DEFAULT 0 CHECK(amount_minor >= 0),
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(category_id) REFERENCES ledger_categories(id) ON DELETE CASCADE,
    FOREIGN KEY(campaign_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ledger_entries (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    entry_type TEXT NOT NULL CHECK(entry_type IN ('Income', 'Expense', 'Transfer', 'Adjustment')),
    category_id TEXT NOT NULL,
    amount_minor INTEGER NOT NULL CHECK(amount_minor >= 0),
    currency_code TEXT NOT NULL DEFAULT 'USD',
    status TEXT NOT NULL CHECK(status IN ('Planned', 'Approved', 'Paid', 'Cancelled')),
    due_date TEXT,
    payment_date TEXT,
    vendor_source TEXT,
    related_task_id TEXT,
    notes TEXT,
    attachment_ref TEXT,
    recurrence TEXT NOT NULL DEFAULT 'None',
    custom_recurrence TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    -- Companion Key de quien registró el movimiento: los Companions solo pueden
    -- editar o borrar los suyos, y la Fellowship necesita saber quién lo asentó.
    created_by_identity TEXT,
    FOREIGN KEY(campaign_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY(category_id) REFERENCES ledger_categories(id) ON DELETE RESTRICT,
    FOREIGN KEY(related_task_id) REFERENCES tasks(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS task_financials (
    task_id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    estimated_cost_minor INTEGER,
    actual_cost_minor INTEGER,
    billable_amount_minor INTEGER,
    payment_status TEXT,
    currency_code TEXT NOT NULL DEFAULT 'USD',
    version INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY(campaign_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS treasury_history (
    id TEXT PRIMARY KEY,
    campaign_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    snapshot TEXT NOT NULL,
    version INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(campaign_id) REFERENCES projects(id) ON DELETE CASCADE
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
CREATE INDEX IF NOT EXISTS idx_task_statuses_project ON task_statuses(project_id);
CREATE INDEX IF NOT EXISTS idx_task_comments_task ON task_comments(task_id, created_at);
CREATE INDEX IF NOT EXISTS idx_task_comments_project ON task_comments(project_id);
CREATE INDEX IF NOT EXISTS idx_task_dependencies_project ON task_dependencies(project_id);
CREATE INDEX IF NOT EXISTS idx_task_dependencies_blocker ON task_dependencies(depends_on_task_id);
CREATE INDEX IF NOT EXISTS idx_global_chronicle_timestamp ON global_chronicle(timestamp);
CREATE INDEX IF NOT EXISTS idx_notes_updated_at ON notes(updated_at);
CREATE INDEX IF NOT EXISTS idx_focus_sessions_completed_at ON focus_sessions(completed_at);
CREATE INDEX IF NOT EXISTS idx_treasury_campaign ON campaign_treasury(campaign_id);
CREATE INDEX IF NOT EXISTS idx_ledger_campaign_created ON ledger_entries(campaign_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ledger_campaign_due ON ledger_entries(campaign_id, due_date);
CREATE INDEX IF NOT EXISTS idx_ledger_campaign_payment ON ledger_entries(campaign_id, payment_date);
CREATE INDEX IF NOT EXISTS idx_ledger_campaign_status ON ledger_entries(campaign_id, status);
CREATE INDEX IF NOT EXISTS idx_ledger_campaign_category ON ledger_entries(campaign_id, category_id);
CREATE INDEX IF NOT EXISTS idx_ledger_task ON ledger_entries(related_task_id);
CREATE INDEX IF NOT EXISTS idx_ledger_vendor ON ledger_entries(campaign_id, vendor_source);
CREATE INDEX IF NOT EXISTS idx_ledger_amount ON ledger_entries(campaign_id, amount_minor);
CREATE INDEX IF NOT EXISTS idx_ledger_search ON ledger_entries(campaign_id, title, vendor_source);
CREATE INDEX IF NOT EXISTS idx_categories_campaign ON ledger_categories(campaign_id, name);
CREATE INDEX IF NOT EXISTS idx_category_budgets_campaign ON category_budgets(campaign_id);
CREATE INDEX IF NOT EXISTS idx_task_financials_campaign ON task_financials(campaign_id);
CREATE INDEX IF NOT EXISTS idx_treasury_history_entity ON treasury_history(entity_type, entity_id, version);
CREATE INDEX IF NOT EXISTS idx_treasury_history_campaign ON treasury_history(campaign_id, created_at DESC);
";
