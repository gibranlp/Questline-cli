// ─────────────────────────────────────────────────────────────────────────────
// database/mod.rs — la capa de acceso a datos, aquí vive toda la magia del SQLite
// ─────────────────────────────────────────────────────────────────────────────

pub mod schema;

use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

use crate::models::{
    Achievement, ClassType, Codex, DailyAdventure, DailyQuest, DailyReflection, FocusSession,
    GlobalChronicleEntry, JournalEntry, Milestone, Note, Project, RecurrenceType, Ritual,
    Statistics, Streak, Task, TaskPriority, User, XPEvent, ZenTree,
};

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;

        // WAL mode: permite lecturas concurrentes mientras el sync thread escribe — sin esto la UI puede congelarse
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        // Sin foreign keys el castillo de datos se cae — además le damos 5s para reintentar en caso de lock
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")?;

        conn.execute_batch(schema::CREATE_TABLES_SQL)?;

        // Migraciones por columna — ALTER TABLE si el campo no existe todavía (upgrades de versiones viejas)
        let has_task_updated_at: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('tasks') WHERE name='updated_at'",
            [],
            |row| { let cnt: i32 = row.get(0)?; Ok(cnt > 0) },
        )?;
        if !has_task_updated_at {
            conn.execute("ALTER TABLE tasks ADD COLUMN updated_at TEXT NOT NULL DEFAULT '';", [])?;
            // Rellenar updated_at con created_at para tareas existentes — sin esto quedan vacías
            conn.execute("UPDATE tasks SET updated_at = created_at WHERE updated_at = '';", [])?;
        }

        let has_priority_col: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('tasks') WHERE name='priority'",
            [],
            |row| {
                let cnt: i32 = row.get(0)?;
                Ok(cnt > 0)
            },
        )?;
        if !has_priority_col {
            conn.execute(
                "ALTER TABLE tasks ADD COLUMN priority TEXT NOT NULL DEFAULT 'Medium';",
                [],
            )?;
        }

        // Especialización del héroe — se agregó después del lanzamiento inicial
        let has_specialization_col: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('users') WHERE name='specialization'",
            [],
            |row| {
                let cnt: i32 = row.get(0)?;
                Ok(cnt > 0)
            },
        )?;
        if !has_specialization_col {
            conn.execute("ALTER TABLE users ADD COLUMN specialization TEXT;", [])?;
        }

        // Los proyectos ahora pueden completarse, no nomás archivarse
        let has_completed_col: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('projects') WHERE name='completed'",
            [],
            |row| {
                let cnt: i32 = row.get(0)?;
                Ok(cnt > 0)
            },
        )?;
        if !has_completed_col {
            conn.execute(
                "ALTER TABLE projects ADD COLUMN completed INTEGER NOT NULL DEFAULT 0;",
                [],
            )?;
        }

        // El ambiente sonoro del focus llegó tarde — para usuarios viejos se pone Silent por defecto
        let has_soundscape_col: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('focus_sessions') WHERE name='soundscape'",
            [],
            |row| {
                let cnt: i32 = row.get(0)?;
                Ok(cnt > 0)
            },
        )?;
        if !has_soundscape_col {
            conn.execute(
                "ALTER TABLE focus_sessions ADD COLUMN soundscape TEXT NOT NULL DEFAULT 'Silent';",
                [],
            )?;
        }

        let has_focus_owner_col: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('focus_sessions') WHERE name='owner_identity'",
            [],
            |row| { let cnt: i32 = row.get(0)?; Ok(cnt > 0) },
        )?;
        if !has_focus_owner_col {
            conn.execute("ALTER TABLE focus_sessions ADD COLUMN owner_identity TEXT;", [])?;
        }

        // Migraciones del modo colaborativo (Stage 5B) — dueño e identidad por entidad
        let has_project_owner_identity: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('projects') WHERE name='owner_identity'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_project_owner_identity {
            conn.execute("ALTER TABLE projects ADD COLUMN owner_identity TEXT;", [])?;
            conn.execute("ALTER TABLE projects ADD COLUMN owner_username TEXT;", [])?;
            conn.execute(
                "ALTER TABLE projects ADD COLUMN is_shared INTEGER NOT NULL DEFAULT 0;",
                [],
            )?;
        }

        let has_task_owner_identity: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('tasks') WHERE name='owner_identity'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_task_owner_identity {
            conn.execute("ALTER TABLE tasks ADD COLUMN owner_identity TEXT;", [])?;
            conn.execute("ALTER TABLE tasks ADD COLUMN owner_username TEXT;", [])?;
        }

        let has_note_sharing_permission: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('notes') WHERE name='sharing_permission'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_note_sharing_permission {
            conn.execute("ALTER TABLE notes ADD COLUMN sharing_permission TEXT NOT NULL DEFAULT 'collaborative';", [])?;
        }

        let has_journal_visibility: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('journal_entries') WHERE name='visibility'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_journal_visibility {
            conn.execute("ALTER TABLE journal_entries ADD COLUMN visibility TEXT NOT NULL DEFAULT 'Private';", [])?;
        }

        let has_journal_author: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('journal_entries') WHERE name='author_username'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_journal_author {
            conn.execute("ALTER TABLE journal_entries ADD COLUMN author_username TEXT NOT NULL DEFAULT '';", [])?;
        }

        let has_milestone_created_at: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('milestones') WHERE name='created_at'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_milestone_created_at {
            conn.execute("ALTER TABLE milestones ADD COLUMN created_at TEXT NOT NULL DEFAULT '2000-01-01T00:00:00+00:00';", [])?;
        }

        let has_tier: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('milestones') WHERE name='tier'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_tier {
            conn.execute(
                "ALTER TABLE milestones ADD COLUMN tier INTEGER NOT NULL DEFAULT 0;",
                [],
            )?;
        }

        let has_template_id: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('milestones') WHERE name='template_id'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_template_id {
            conn.execute(
                "ALTER TABLE milestones ADD COLUMN template_id TEXT NOT NULL DEFAULT '';",
                [],
            )?;
        }

        // v1.0.5: subtareas — sin esto las tareas son todas huérfanas de jerarquía
        let has_parent_task_id: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('tasks') WHERE name='parent_task_id'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_parent_task_id {
            conn.execute("ALTER TABLE tasks ADD COLUMN parent_task_id TEXT;", [])?;
        }
        // CUIDADO: el índice debe crearse después de que la columna ya exista — no mover antes del bloque de migración
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_tasks_parent_task_id ON tasks(parent_task_id);",
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS codices (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_codices_project_id ON codices(project_id);",
        )?;

        let has_codex_id: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('notes') WHERE name='codex_id'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_codex_id {
            conn.execute("ALTER TABLE notes ADD COLUMN codex_id TEXT;", [])?;
        }
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_notes_codex_id ON notes(codex_id);",
        )?;

        let has_note_owner_identity: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('notes') WHERE name='owner_identity'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_note_owner_identity {
            conn.execute("ALTER TABLE notes ADD COLUMN owner_identity TEXT;", [])?;
        }

        // v1.0.6: contador acumulado de riegos — necesario para calcular contribuciones al capítulo global
        let has_total_waterings: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('zen_tree') WHERE name='total_waterings'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_total_waterings {
            conn.execute(
                "ALTER TABLE zen_tree ADD COLUMN total_waterings INTEGER NOT NULL DEFAULT 0;",
                [],
            )?;
        }

        // Tabla nueva para rastrear qué tanto ya mandamos al servidor del capítulo — evita doble-conteo
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS chapter_contribution_log (
                chapter_id TEXT NOT NULL,
                objective_type TEXT NOT NULL,
                last_sent_total INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (chapter_id, objective_type)
            );",
        )?;

        let has_chronicle_hero_class: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('global_chronicle') WHERE name='hero_class'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_chronicle_hero_class {
            conn.execute("ALTER TABLE global_chronicle ADD COLUMN hero_class TEXT;", [])?;
        }

        // v1.0.7: xp_awarded — evita que reabrir y re-completar una tarea dé XP extra
        let has_xp_awarded: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('tasks') WHERE name='xp_awarded'",
            [],
            |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_xp_awarded {
            conn.execute(
                "ALTER TABLE tasks ADD COLUMN xp_awarded INTEGER NOT NULL DEFAULT 0;",
                [],
            )?;
            // Las tareas ya completadas antes de esta versión ya dieron XP — se marca como cobrado
            conn.execute(
                "UPDATE tasks SET xp_awarded = 1 WHERE completed = 1;",
                [],
            )?;
        }

        // v1.0.8: recurrence — tareas que renacen solas al completarse
        let has_recurrence: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('tasks') WHERE name='recurrence'",
            [], |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_recurrence {
            conn.execute("ALTER TABLE tasks ADD COLUMN recurrence TEXT;", [])?;
        }

        // v1.0.9: nested codices — un codex puede vivir dentro de otro
        let has_parent_codex: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('codices') WHERE name='parent_codex_id'",
            [], |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_parent_codex {
            conn.execute("ALTER TABLE codices ADD COLUMN parent_codex_id TEXT;", [])?;
        }

        let has_codex_collapsed: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('codices') WHERE name='collapsed'",
            [], |row| row.get::<_, i32>(0).map(|c| c > 0),
        )?;
        if !has_codex_collapsed {
            conn.execute("ALTER TABLE codices ADD COLUMN collapsed INTEGER NOT NULL DEFAULT 0;", [])?;
        }

        for (id, name, desc) in Achievement::static_list() {
            conn.execute(
                "INSERT OR IGNORE INTO achievements (id, name, description, unlocked_at) VALUES (?1, ?2, ?3, NULL)",
                params![id, name, desc],
            )?;
        }

        let count_rituals: i32 =
            conn.query_row("SELECT count(*) FROM rituals", [], |row| row.get(0))?;
        if count_rituals == 0 {
            let default_rituals = vec![
                (
                    "morning_planning",
                    "Morning Planning",
                    "Plan out your day's quests.",
                    "Daily",
                    25,
                ),
                (
                    "workout",
                    "Workout",
                    "Physical conditioning for the trials ahead.",
                    "Daily",
                    40,
                ),
                (
                    "read_pages",
                    "Read 10 Pages",
                    "Absorb ancient knowledge from scrolls.",
                    "Daily",
                    30,
                ),
                (
                    "review_tasks",
                    "Review Tasks",
                    "Audit and clean your to-do lists.",
                    "Daily",
                    20,
                ),
                (
                    "meditate",
                    "Meditate",
                    "Calm the mind-sage within.",
                    "Daily",
                    25,
                ),
            ];
            for (id, name, desc, freq, xp) in default_rituals {
                conn.execute(
                    "INSERT INTO rituals (id, name, description, frequency, reward_xp, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![id, name, desc, freq, xp, Utc::now().to_rfc3339()],
                )?;
            }
        }

        let count_tree: i32 =
            conn.query_row("SELECT count(*) FROM zen_tree", [], |row| row.get(0))?;
        if count_tree == 0 {
            let tree_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO zen_tree (id, growth, health, stage, last_watered, water_today) VALUES (?1, 0, 100, 1, NULL, 0)",
                params![tree_id],
            )?;
        }

        let count_streaks: i32 =
            conn.query_row("SELECT count(*) FROM streaks", [], |row| row.get(0))?;
        if count_streaks == 0 {
            conn.execute(
                "INSERT INTO streaks (id, current_streak, best_streak, last_active_day) VALUES ('streak_id', 0, 0, NULL)",
                [],
            )?;
        }

        let count_titles: i32 =
            conn.query_row("SELECT count(*) FROM legendary_titles", [], |row| {
                row.get(0)
            })?;
        if count_titles == 0 {
            let default_titles = vec![
                (
                    "relentless",
                    "The Relentless",
                    "Achieve a 30-day streak of questing.",
                ),
                (
                    "archivist",
                    "The Archivist",
                    "Amass 50 journal entries or scrolls of thought.",
                ),
                (
                    "focused",
                    "The Focused",
                    "Complete 100 deep work focus sessions.",
                ),
                (
                    "master_seasons",
                    "Master of Seasons",
                    "Experience the focus rituals of all four seasons.",
                ),
                (
                    "ancient_gardener",
                    "The Ancient Gardener",
                    "Nurture your Zen Tree to Stage 5.",
                ),
                (
                    "keeper_chronicles",
                    "Keeper of Chronicles",
                    "Record 50 entries in the Great Chronicle.",
                ),
            ];
            for (id, name, desc) in default_titles {
                conn.execute(
                    "INSERT INTO legendary_titles (title_id, title_name, description, unlocked, equipped) VALUES (?1, ?2, ?3, 0, 0)",
                    params![id, name, desc],
                )?;
            }
        }

        let count_relics: i32 =
            conn.query_row("SELECT count(*) FROM relics", [], |row| row.get(0))?;
        if count_relics == 0 {
            let default_relics = vec![
                ("ancient_quill", "Ancient Quill", "A feather plucked from an owl of the high canopy. It writes with invisible ink that glows only under moonlight. (Unlocked by Scholar achievement)"),
                ("crystal_compass", "Crystal Compass", "Its needle does not point north, but toward the nearest unfinished task. (Unlocked by Project Master achievement)"),
                ("rune_tablet", "Rune Tablet", "An ancient stone slab inscribed with glowing symbols that pulse in harmony with your tree. (Unlocked at Level 50)"),
                ("explorers_map", "Explorer's Map", "A dusty parchment depicting shifting landscapes that update as your streak grows. (Unlocked by 30-day streak)"),
                ("clock_of_focus", "Clock of Focus", "A pocket watch that ticks slower when you are concentrated, expanding time itself. (Unlocked by 50 focus sessions)"),
            ];
            for (id, name, desc) in default_relics {
                conn.execute(
                    "INSERT INTO relics (id, name, description, unlocked, unlocked_at) VALUES (?1, ?2, ?3, 0, NULL)",
                    params![id, name, desc],
                )?;
            }
        }

        Ok(Self { conn })
    }

    // ── Métodos públicos para sembrar lore desde el LoreManager ──────────────
    //
    // Se usan INSERT OR IGNORE para no pisar el estado de desbloqueo del usuario.
    // Se llaman desde app/mod.rs después de que LoreManager descargó los datos.

    pub fn seed_lore_entries(&self, entries: &[crate::services::lore_manager::LoreEntry]) -> Result<()> {
        // Recolecta los IDs que vienen del servidor para detectar eliminaciones
        let remote_ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();

        for e in entries {
            let starts_unlocked = if e.category == "Memory" || e.unlock.unlock_type == "discovery" {
                0i32
            } else if e.unlock.unlock_type == "free" {
                1i32
            } else {
                0i32
            };

            // Inserta si no existe — preserva unlocked/unlocked_at del usuario
            self.conn.execute(
                "INSERT OR IGNORE INTO lore_library (id, category, title, content, unlocked, unlocked_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
                rusqlite::params![e.id, e.category, e.title, e.content, starts_unlocked],
            )?;

            // Actualiza título y contenido en filas existentes — el admin puede editar el texto
            // sin tocar unlocked ni unlocked_at, que son progreso del usuario
            self.conn.execute(
                "UPDATE lore_library SET category = ?2, title = ?3, content = ?4 WHERE id = ?1",
                rusqlite::params![e.id, e.category, e.title, e.content],
            )?;
        }

        // Elimina entradas que el admin borró del JSON — si el ID ya no está en el servidor, no debe estar aquí
        // Solo borra las que aún están bloqueadas; si el usuario ya la desbloqueó se conserva como recuerdo
        if !remote_ids.is_empty() {
            let placeholders = remote_ids.iter().enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "DELETE FROM lore_library WHERE id NOT IN ({}) AND unlocked = 0",
                placeholders
            );
            let mut stmt = self.conn.prepare(&sql)?;
            for (i, id) in remote_ids.iter().enumerate() {
                stmt.raw_bind_parameter(i + 1, id)?;
            }
            stmt.raw_execute()?;
        }

        Ok(())
    }

    pub fn seed_class_quests(&self, quests: &[crate::services::lore_manager::ClassQuest]) -> Result<()> {
        let remote_keys: Vec<(String, i32)> = quests.iter()
            .map(|q| (q.class.clone(), q.level))
            .collect();

        for q in quests {
            // Inserta si no existe — preserva status y progress del usuario
            self.conn.execute(
                "INSERT OR IGNORE INTO class_quests (class_name, unlock_level, quest_name, description, status, progress, target, lore_reward) VALUES (?1, ?2, ?3, ?4, 'Locked', 0, ?5, ?6)",
                rusqlite::params![q.class, q.level, q.name, q.description, q.objective.target, q.lore_reward],
            )?;

            // Actualiza nombre, descripción y target si el admin los editó
            // No toca status ni progress — eso es progreso del usuario
            self.conn.execute(
                "UPDATE class_quests SET quest_name = ?3, description = ?4, target = ?5, lore_reward = ?6 WHERE class_name = ?1 AND unlock_level = ?2",
                rusqlite::params![q.class, q.level, q.name, q.description, q.objective.target, q.lore_reward],
            )?;
        }

        // Elimina quests que el admin borró — solo las que no tienen progreso iniciado
        for q_existing in self.conn.prepare(
            "SELECT class_name, unlock_level FROM class_quests WHERE status = 'Locked'"
        )?.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?.flatten() {
            if !remote_keys.iter().any(|(c, l)| c == &q_existing.0 && *l == q_existing.1) {
                self.conn.execute(
                    "DELETE FROM class_quests WHERE class_name = ?1 AND unlock_level = ?2 AND status = 'Locked'",
                    rusqlite::params![q_existing.0, q_existing.1],
                )?;
            }
        }

        Ok(())
    }

    pub fn get_user(&self) -> Result<Option<User>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, username, class, level, xp, created_at, specialization FROM users LIMIT 1",
        )?;
        let user_opt = stmt
            .query_row([], |row| {
                let id_str: String = row.get(0)?;
                let username: String = row.get(1)?;
                let class_str: String = row.get(2)?;
                let level: i32 = row.get(3)?;
                let xp: i32 = row.get(4)?;
                let created_str: String = row.get(5)?;
                let specialization: Option<String> = row.get(6)?;

                let id =
                    Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
                let class =
                    ClassType::from_str(&class_str).ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                let created_at = DateTime::parse_from_rfc3339(&created_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

                Ok(User {
                    id,
                    username,
                    class,
                    level,
                    xp,
                    created_at,
                    specialization,
                })
            })
            .optional()?;
        Ok(user_opt)
    }

    pub fn insert_user(&self, user: &User) -> Result<()> {
        self.conn.execute(
            "INSERT INTO users (id, username, class, level, xp, created_at, specialization) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                user.id.to_string(),
                user.username,
                user.class.name(),
                user.level,
                user.xp,
                user.created_at.to_rfc3339(),
                user.specialization
            ],
        )?;
        let _ = self.log_change("user", &user.id.to_string(), "create");
        Ok(())
    }

    pub fn update_user(&self, user: &User) -> Result<()> {
        self.conn.execute(
            "UPDATE users SET username = ?1, class = ?2, level = ?3, xp = ?4, specialization = ?5 WHERE id = ?6",
            params![
                user.username,
                user.class.name(),
                user.level,
                user.xp,
                user.specialization,
                user.id.to_string()
            ],
        )?;
        let _ = self.log_change("user", &user.id.to_string(), "update");
        Ok(())
    }

    pub fn insert_project(&self, project: &Project) -> Result<()> {
        self.conn.execute(
            "INSERT INTO projects (id, name, description, created_at, archived, completed, owner_identity, owner_username, is_shared) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                project.id.to_string(),
                project.name,
                project.description,
                project.created_at.to_rfc3339(),
                if project.archived { 1 } else { 0 },
                if project.completed { 1 } else { 0 },
                project.owner_identity,
                project.owner_username,
                if project.is_shared { 1 } else { 0 }
            ],
        )?;
        let _ = self.log_change("project", &project.id.to_string(), "create");
        Ok(())
    }

    pub fn get_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare("SELECT id, name, description, created_at, archived, completed, owner_identity, owner_username, is_shared FROM projects")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let name: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let created_str: String = row.get(3)?;
            let archived_int: i32 = row.get(4)?;
            let completed_int: i32 = row.get(5)?;
            let owner_identity: Option<String> = row.get(6)?;
            let owner_username: Option<String> = row.get(7)?;
            let is_shared_int: i32 = row.get(8)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

            Ok(Project {
                id,
                name,
                description,
                created_at,
                archived: archived_int != 0,
                completed: completed_int != 0,
                owner_identity,
                owner_username,
                is_shared: is_shared_int != 0,
            })
        })?;

        let mut projects = Vec::new();
        for r in rows {
            projects.push(r?);
        }
        Ok(projects)
    }

    pub fn update_project(&self, project: &Project) -> Result<()> {
        self.conn.execute(
            "UPDATE projects SET name = ?1, description = ?2, archived = ?3, completed = ?4, owner_identity = ?5, owner_username = ?6, is_shared = ?7 WHERE id = ?8",
            params![
                project.name,
                project.description,
                if project.archived { 1 } else { 0 },
                if project.completed { 1 } else { 0 },
                project.owner_identity,
                project.owner_username,
                if project.is_shared { 1 } else { 0 },
                project.id.to_string()
            ],
        )?;
        Ok(())
    }

    pub fn delete_project_permanently(&self, id: Uuid) -> Result<()> {
        // Tombstone antes de borrar — los otros dispositivos necesitan saber que este proyecto ya no existe
        let _ = self.log_change("project", &id.to_string(), "delete");
        self.conn.execute(
            "DELETE FROM projects WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    pub fn insert_task(&self, task: &Task) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tasks (id, project_id, title, description, due_date, completed, priority, created_at, updated_at, owner_identity, owner_username, parent_task_id, xp_awarded, recurrence) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                task.id.to_string(),
                task.project_id.map(|id| id.to_string()),
                task.title,
                task.description,
                task.due_date.map(|d| d.to_rfc3339()),
                if task.completed { 1 } else { 0 },
                task.priority.name(),
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.owner_identity,
                task.owner_username,
                task.parent_task_id.map(|id| id.to_string()),
                if task.xp_awarded { 1 } else { 0 },
                task.recurrence.map(|r| r.name()),
            ],
        )?;
        let _ = self.log_change("task", &task.id.to_string(), "create");
        if let Ok(content_json) = serde_json::to_string(task) {
            let _ = self.create_revision("task", &task.id.to_string(), &content_json);
        }
        Ok(())
    }

    pub fn get_tasks(&self) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, title, description, due_date, completed, priority, created_at, updated_at, owner_identity, owner_username, parent_task_id, xp_awarded, recurrence FROM tasks")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: Option<String> = row.get(1)?;
            let title: String = row.get(2)?;
            let description: Option<String> = row.get(3)?;
            let due_str: Option<String> = row.get(4)?;
            let completed_int: i32 = row.get(5)?;
            let priority_str: String = row.get(6)?;
            let created_str: String = row.get(7)?;
            let updated_str: String = row.get(8)?;
            let owner_identity: Option<String> = row.get(9)?;
            let owner_username: Option<String> = row.get(10)?;
            let parent_task_id_str: Option<String> = row.get(11)?;
            let xp_awarded_int: i32 = row.get(12)?;
            let recurrence_str: Option<String> = row.get(13)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let project_id = match project_id_str {
                Some(p) => {
                    Some(Uuid::parse_str(&p).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?)
                }
                None => None,
            };
            let due_date = match due_str {
                Some(d) => Some(
                    DateTime::parse_from_rfc3339(&d)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?,
                ),
                None => None,
            };
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(created_at);
            let priority = TaskPriority::from_str(&priority_str);
            let parent_task_id = match parent_task_id_str {
                Some(p) => Some(Uuid::parse_str(&p).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };

            Ok(Task {
                id,
                project_id,
                title,
                description,
                due_date,
                completed: completed_int != 0,
                priority,
                created_at,
                updated_at,
                owner_identity,
                owner_username,
                parent_task_id,
                xp_awarded: xp_awarded_int != 0,
                recurrence: recurrence_str.as_deref().and_then(RecurrenceType::from_str),
            })
        })?;

        let mut tasks = Vec::new();
        for r in rows {
            tasks.push(r?);
        }
        Ok(tasks)
    }

    pub fn get_tasks_for_project(&self, project_id: Uuid) -> Result<Vec<Task>> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, title, description, due_date, completed, priority, created_at, updated_at, owner_identity, owner_username, parent_task_id, xp_awarded, recurrence FROM tasks WHERE project_id = ?1")?;
        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: Option<String> = row.get(1)?;
            let title: String = row.get(2)?;
            let description: Option<String> = row.get(3)?;
            let due_str: Option<String> = row.get(4)?;
            let completed_int: i32 = row.get(5)?;
            let priority_str: String = row.get(6)?;
            let created_str: String = row.get(7)?;
            let updated_str: String = row.get(8)?;
            let owner_identity: Option<String> = row.get(9)?;
            let owner_username: Option<String> = row.get(10)?;
            let parent_task_id_str: Option<String> = row.get(11)?;
            let xp_awarded_int: i32 = row.get(12)?;
            let recurrence_str: Option<String> = row.get(13)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let project_id = match project_id_str {
                Some(p) => Some(Uuid::parse_str(&p).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };
            let due_date = match due_str {
                Some(d) => Some(DateTime::parse_from_rfc3339(&d).map(|dt| dt.with_timezone(&Utc)).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };
            let created_at = DateTime::parse_from_rfc3339(&created_str).map(|dt| dt.with_timezone(&Utc)).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_str).map(|dt| dt.with_timezone(&Utc)).unwrap_or(created_at);
            let priority = TaskPriority::from_str(&priority_str);
            let parent_task_id = match parent_task_id_str {
                Some(p) => Some(Uuid::parse_str(&p).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };
            Ok(Task { id, project_id, title, description, due_date, completed: completed_int != 0, priority, created_at, updated_at, owner_identity, owner_username, parent_task_id, xp_awarded: xp_awarded_int != 0, recurrence: recurrence_str.as_deref().and_then(RecurrenceType::from_str) })
        })?;
        let mut tasks = Vec::new();
        for r in rows { tasks.push(r?); }
        Ok(tasks)
    }


    pub fn update_task(&self, task: &Task) -> Result<()> {
        let old_task = self.get_task_by_id(task.id).ok();
        let was_completed = old_task.map(|t| t.completed).unwrap_or(false);

        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE tasks SET project_id = ?1, title = ?2, description = ?3, due_date = ?4, completed = ?5, priority = ?6, updated_at = ?7, owner_identity = ?8, owner_username = ?9, parent_task_id = ?10, xp_awarded = ?11, recurrence = ?12 WHERE id = ?13",
            params![
                task.project_id.map(|id| id.to_string()),
                task.title,
                task.description,
                task.due_date.map(|d| d.to_rfc3339()),
                if task.completed { 1 } else { 0 },
                task.priority.name(),
                now,
                task.owner_identity,
                task.owner_username,
                task.parent_task_id.map(|id| id.to_string()),
                if task.xp_awarded { 1 } else { 0 },
                task.recurrence.map(|r| r.name()),
                task.id.to_string()
            ],
        )?;

        // si pasó de incompleta a completa, el log va como "complete" — el sync server distingue ambos eventos
        let op = if task.completed && !was_completed {
            "complete"
        } else {
            "update"
        };
        let _ = self.log_change("task", &task.id.to_string(), op);
        if let Ok(content_json) = serde_json::to_string(task) {
            let _ = self.create_revision("task", &task.id.to_string(), &content_json);
        }
        Ok(())
    }

    pub fn delete_task(&self, id: Uuid) -> Result<()> {
        // Tombstone — sin esto la tarea resucita en el próximo pull desde otro dispositivo
        let _ = self.log_change("task", &id.to_string(), "delete");
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }

    pub fn insert_note(&self, note: &Note) -> Result<()> {
        self.conn.execute(
            "INSERT INTO notes (id, project_id, title, markdown_content, created_at, updated_at, sharing_permission, codex_id, owner_identity) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                note.id.to_string(),
                note.project_id.map(|id| id.to_string()),
                note.title,
                note.markdown_content,
                note.created_at.to_rfc3339(),
                note.updated_at.to_rfc3339(),
                note.sharing_permission,
                note.codex_id.map(|id| id.to_string()),
                note.owner_identity.as_deref()
            ],
        )?;
        let _ = self.log_change("note", &note.id.to_string(), "create");
        if let Ok(content_json) = serde_json::to_string(note) {
            let _ = self.create_revision("note", &note.id.to_string(), &content_json);
        }
        Ok(())
    }

    pub fn get_notes(&self) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, title, markdown_content, created_at, updated_at, sharing_permission, codex_id, owner_identity FROM notes")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: Option<String> = row.get(1)?;
            let title: String = row.get(2)?;
            let content: String = row.get(3)?;
            let created_str: String = row.get(4)?;
            let updated_str: String = row.get(5)?;
            let sharing_permission: String = row.get(6)?;
            let codex_id_str: Option<String> = row.get(7)?;
            let owner_identity: Option<String> = row.get(8)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let project_id = match project_id_str {
                Some(p) => {
                    Some(Uuid::parse_str(&p).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?)
                }
                None => None,
            };
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let codex_id = match codex_id_str {
                Some(c) => Some(Uuid::parse_str(&c).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };

            Ok(Note {
                id,
                project_id,
                title,
                markdown_content: content,
                created_at,
                updated_at,
                sharing_permission,
                codex_id,
                owner_identity,
            })
        })?;

        let mut notes = Vec::new();
        for r in rows {
            notes.push(r?);
        }
        Ok(notes)
    }

    pub fn get_notes_for_project(&self, project_id: Uuid) -> Result<Vec<Note>> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, title, markdown_content, created_at, updated_at, sharing_permission, codex_id, owner_identity FROM notes WHERE project_id = ?1")?;
        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: Option<String> = row.get(1)?;
            let title: String = row.get(2)?;
            let content: String = row.get(3)?;
            let created_str: String = row.get(4)?;
            let updated_str: String = row.get(5)?;
            let sharing_permission: String = row.get(6)?;
            let codex_id_str: Option<String> = row.get(7)?;
            let owner_identity: Option<String> = row.get(8)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let project_id = match project_id_str {
                Some(p) => Some(Uuid::parse_str(&p).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };
            let created_at = DateTime::parse_from_rfc3339(&created_str).map(|dt| dt.with_timezone(&Utc)).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_str).map(|dt| dt.with_timezone(&Utc)).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let codex_id = match codex_id_str {
                Some(c) => Some(Uuid::parse_str(&c).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };
            Ok(Note { id, project_id, title, markdown_content: content, created_at, updated_at, sharing_permission, codex_id, owner_identity })
        })?;
        let mut notes = Vec::new();
        for r in rows { notes.push(r?); }
        Ok(notes)
    }

    pub fn update_note(&self, note: &Note) -> Result<()> {
        self.conn.execute(
            "UPDATE notes SET project_id = ?1, title = ?2, markdown_content = ?3, updated_at = ?4, sharing_permission = ?5, codex_id = ?6 WHERE id = ?7",
            params![
                note.project_id.map(|id| id.to_string()),
                note.title,
                note.markdown_content,
                note.updated_at.to_rfc3339(),
                note.sharing_permission,
                note.codex_id.map(|id| id.to_string()),
                note.id.to_string()
            ],
        )?;
        let _ = self.log_change("note", &note.id.to_string(), "update");
        if let Ok(content_json) = serde_json::to_string(note) {
            let _ = self.create_revision("note", &note.id.to_string(), &content_json);
        }
        Ok(())
    }

    pub fn delete_note(&self, id: Uuid) -> Result<()> {
        self.conn
            .execute("DELETE FROM notes WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }

    pub fn get_daily_quests_for_date(&self, date: NaiveDate) -> Result<Vec<DailyQuest>> {
        let mut stmt = self.conn.prepare("SELECT id, title, description, completed, due_date FROM daily_quests WHERE due_date = ?1")?;
        let rows = stmt.query_map([date.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let title: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let completed_int: i32 = row.get(3)?;
            let due_str: String = row.get(4)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let due_date = NaiveDate::parse_from_str(&due_str, "%Y-%m-%d")
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

            Ok(DailyQuest {
                id,
                title,
                description,
                completed: completed_int != 0,
                due_date,
            })
        })?;

        let mut quests = Vec::new();
        for r in rows {
            quests.push(r?);
        }
        Ok(quests)
    }

    pub fn insert_xp_event(&self, event: &XPEvent) -> Result<()> {
        self.conn.execute(
            "INSERT INTO xp_events (id, event_type, xp_gained, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![
                event.id.to_string(),
                event.event_type,
                event.xp_gained,
                event.timestamp.to_rfc3339()
            ],
        )?;
        Ok(())
    }

    pub fn insert_journal_entry(&self, entry: &JournalEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO journal_entries (id, project_id, entry_date, content, created_at, visibility, author_username) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                entry.id.to_string(),
                entry.project_id.to_string(),
                entry.entry_date.to_string(),
                entry.content,
                entry.created_at.to_rfc3339(),
                entry.visibility,
                entry.author_username
            ],
        )?;
        let _ = self.log_change("journal_entry", &entry.id.to_string(), "create");
        Ok(())
    }

    pub fn get_journal_entries(&self) -> Result<Vec<JournalEntry>> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, entry_date, content, created_at, visibility, author_username FROM journal_entries ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: String = row.get(1)?;
            let date_str: String = row.get(2)?;
            let content: String = row.get(3)?;
            let created_str: String = row.get(4)?;
            let visibility: String = row.get(5)?;
            let author_username: String = row.get(6)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let project_id = Uuid::parse_str(&project_id_str)
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let entry_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

            Ok(JournalEntry {
                id,
                project_id,
                entry_date,
                content,
                created_at,
                visibility,
                author_username,
            })
        })?;

        let mut entries = Vec::new();
        for r in rows {
            entries.push(r?);
        }
        Ok(entries)
    }

    pub fn get_journal_entries_for_project(&self, project_id: Uuid) -> Result<Vec<JournalEntry>> {
        let entries = self.get_journal_entries()?;
        let filtered = entries
            .into_iter()
            .filter(|e| e.project_id == project_id)
            .collect();
        Ok(filtered)
    }

    pub fn get_zen_tree(&self) -> Result<ZenTree> {
        let mut stmt = self.conn.prepare(
            "SELECT id, growth, health, stage, last_watered, water_today, COALESCE(total_waterings, 0) FROM zen_tree LIMIT 1",
        )?;
        let tree = stmt.query_row([], |row| {
            let id_str: String = row.get(0)?;
            let growth: i32 = row.get(1)?;
            let health: i32 = row.get(2)?;
            let stage: i32 = row.get(3)?;
            let last_watered_str: Option<String> = row.get(4)?;
            let water_today: i32 = row.get(5)?;
            let total_waterings: i32 = row.get(6)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let last_watered = last_watered_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });

            Ok(ZenTree {
                id,
                growth,
                health,
                stage,
                last_watered,
                water_today,
                total_waterings,
            })
        })?;
        Ok(tree)
    }

    pub fn update_zen_tree(&self, tree: &ZenTree) -> Result<()> {
        self.conn.execute(
            "UPDATE zen_tree SET growth = ?1, health = ?2, stage = ?3, last_watered = ?4, water_today = ?5, total_waterings = ?6 WHERE id = ?7",
            params![
                tree.growth,
                tree.health,
                tree.stage,
                tree.last_watered.map(|dt| dt.to_rfc3339()),
                tree.water_today,
                tree.total_waterings,
                tree.id.to_string(),
            ],
        )?;
        let _ = self.log_change("zen_tree", &tree.id.to_string(), "update");
        Ok(())
    }

    pub fn get_streak(&self) -> Result<Streak> {
        let mut stmt = self.conn.prepare(
            "SELECT id, current_streak, best_streak, last_active_day FROM streaks LIMIT 1",
        )?;
        let streak = stmt.query_row([], |row| {
            let id: String = row.get(0)?;
            let current_streak: i32 = row.get(1)?;
            let best_streak: i32 = row.get(2)?;
            let last_active_day_str: Option<String> = row.get(3)?;

            let last_active_day =
                last_active_day_str.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok());

            Ok(Streak {
                id,
                current_streak,
                best_streak,
                last_active_day,
            })
        })?;
        Ok(streak)
    }

    pub fn update_streak(&self, streak: &Streak) -> Result<()> {
        self.conn.execute(
            "UPDATE streaks SET current_streak = ?1, best_streak = ?2, last_active_day = ?3 WHERE id = ?4",
            params![
                streak.current_streak,
                streak.best_streak,
                streak.last_active_day.map(|d| d.format("%Y-%m-%d").to_string()),
                streak.id,
            ],
        )?;
        Ok(())
    }

    pub fn get_achievements(&self) -> Result<Vec<Achievement>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, description, unlocked_at FROM achievements")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let description: String = row.get(2)?;
            let unlocked_str: Option<String> = row.get(3)?;

            let unlocked_at = unlocked_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });

            Ok(Achievement {
                id,
                name,
                description,
                unlocked_at,
            })
        })?;

        let mut achievements = Vec::new();
        for r in rows {
            achievements.push(r?);
        }
        Ok(achievements)
    }

    pub fn unlock_achievement(&self, id: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE achievements SET unlocked_at = ?1 WHERE id = ?2 AND unlocked_at IS NULL",
            params![now_str, id],
        )?;
        let _ = self.log_change("achievement", id, "unlock");
        Ok(())
    }

    pub fn get_daily_adventures(&self) -> Result<Vec<DailyAdventure>> {
        let mut stmt = self.conn.prepare("SELECT id, title, quest_type, target_count, current_count, completed, created_date FROM daily_adventures")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let title: String = row.get(1)?;
            let quest_type: String = row.get(2)?;
            let target_count: i32 = row.get(3)?;
            let current_count: i32 = row.get(4)?;
            let completed_int: i32 = row.get(5)?;
            let created_date_str: String = row.get(6)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let created_date = NaiveDate::parse_from_str(&created_date_str, "%Y-%m-%d")
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

            Ok(DailyAdventure {
                id,
                title,
                quest_type,
                target_count,
                current_count,
                completed: completed_int != 0,
                created_date,
            })
        })?;

        let mut adventures = Vec::new();
        for r in rows {
            adventures.push(r?);
        }
        Ok(adventures)
    }

    pub fn insert_daily_adventure(&self, adv: &DailyAdventure) -> Result<()> {
        self.conn.execute(
            "INSERT INTO daily_adventures (id, title, quest_type, target_count, current_count, completed, created_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                adv.id.to_string(),
                adv.title,
                adv.quest_type,
                adv.target_count,
                adv.current_count,
                if adv.completed { 1 } else { 0 },
                adv.created_date.format("%Y-%m-%d").to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn update_daily_adventure(&self, adv: &DailyAdventure) -> Result<()> {
        self.conn.execute(
            "UPDATE daily_adventures SET current_count = ?1, completed = ?2 WHERE id = ?3",
            params![
                adv.current_count,
                if adv.completed { 1 } else { 0 },
                adv.id.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn clear_daily_adventures(&self) -> Result<()> {
        self.conn.execute("DELETE FROM daily_adventures", [])?;
        Ok(())
    }

    pub fn get_statistics(&self) -> Result<Statistics> {
        let tasks_completed: i32 = self.conn.query_row(
            "SELECT count(*) FROM tasks WHERE completed = 1",
            [],
            |row| row.get(0),
        )?;
        let notes_created: i32 = self
            .conn
            .query_row("SELECT count(*) FROM notes", [], |row| row.get(0))?;
        let journal_entries: i32 =
            self.conn
                .query_row("SELECT count(*) FROM journal_entries", [], |row| row.get(0))?;
        let projects_created: i32 =
            self.conn
                .query_row("SELECT count(*) FROM projects", [], |row| row.get(0))?;

        let streak = self.get_streak().unwrap_or(Streak {
            id: "streak_id".to_string(),
            current_streak: 0,
            best_streak: 0,
            last_active_day: None,
        });

        let tree = self.get_zen_tree().unwrap_or(ZenTree {
            id: Uuid::nil(),
            growth: 0,
            health: 100,
            stage: 1,
            last_watered: None,
            water_today: 0,
            total_waterings: 0,
        });

        let achievements_unlocked: i32 = self.conn.query_row(
            "SELECT count(*) FROM achievements WHERE unlocked_at IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        let total_xp_earned: i32 = self.conn.query_row(
            "SELECT COALESCE(SUM(xp_gained), 0) FROM xp_events",
            [],
            |row| row.get(0),
        )?;

        let focus_hours: f64 = self.conn.query_row(
            "SELECT COALESCE(SUM(duration_mins), 0) / 60.0 FROM focus_sessions",
            [],
            |row| row.get(0),
        )?;
        let sessions_completed: i32 =
            self.conn
                .query_row("SELECT count(*) FROM focus_sessions", [], |row| row.get(0))?;
        let rituals_completed: i32 =
            self.conn
                .query_row("SELECT count(*) FROM ritual_history", [], |row| row.get(0))?;
        let projects_completed: i32 = self.conn.query_row(
            "SELECT count(*) FROM projects WHERE completed = 1",
            [],
            |row| row.get(0),
        )?;
        let milestones_completed: i32 = self.conn.query_row(
            "SELECT count(*) FROM milestones WHERE completed = 1",
            [],
            |row| row.get(0),
        )?;

        let most_productive_day: String = self.conn.query_row(
            "SELECT date(timestamp) as d, SUM(xp_gained) as total FROM xp_events GROUP BY d ORDER BY total DESC LIMIT 1",
            [],
            |row| row.get(0)
        ).optional()?.unwrap_or_else(|| "None".to_string());

        let days_elapsed = match self.conn
                .query_row("SELECT created_at FROM users LIMIT 1", [], |row| {
                    row.get::<_, String>(0)
                }) { Ok(created_str) => {
            if let Ok(created_at) = DateTime::parse_from_rfc3339(&created_str) {
                let diff = Utc::now()
                    .signed_duration_since(created_at.with_timezone(&Utc))
                    .num_days();
                diff.max(1)
            } else {
                1
            }
        } _ => {
            1
        }};

        let avg_tasks_per_day = tasks_completed as f64 / days_elapsed as f64;
        let avg_xp_per_day = total_xp_earned as f64 / days_elapsed as f64;

        let sync_count: i32 = self
            .get_setting("sync_count")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let backup_count: i32 = self
            .get_setting("backup_count")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let conflict_count: i32 = self
            .get_setting("conflict_count")?
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let last_restore: String = self
            .get_setting("last_restore")?
            .unwrap_or_else(|| "Never".to_string());
        let devices_connected = self.get_devices().map(|d| d.len()).unwrap_or(0) as i32;
        let active_devices = self.count_active_devices(5).unwrap_or(0) as i32;

        Ok(Statistics {
            tasks_completed,
            notes_created,
            journal_entries,
            projects_created,
            current_streak: streak.current_streak,
            best_streak: streak.best_streak,
            tree_growth: tree.growth,
            achievements_unlocked,
            total_xp_earned,
            focus_hours,
            sessions_completed,
            rituals_completed,
            projects_completed,
            milestones_completed,
            most_productive_day,
            avg_tasks_per_day,
            avg_xp_per_day,
            sync_count,
            backup_count,
            devices_connected,
            active_devices,
            last_restore,
            conflict_count,
        })
    }

    pub fn insert_focus_session(&self, sess: &FocusSession) -> Result<()> {
        self.conn.execute(
            "INSERT INTO focus_sessions (id, project_id, task_id, duration_mins, xp_gained, completed_at, soundscape, owner_identity) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                sess.id.to_string(),
                sess.project_id.map(|u| u.to_string()),
                sess.task_id.map(|u| u.to_string()),
                sess.duration_mins,
                sess.xp_gained,
                sess.completed_at.to_rfc3339(),
                sess.soundscape,
                sess.owner_identity,
            ],
        )?;
        let _ = self.log_change("focus_session", &sess.id.to_string(), "insert");
        Ok(())
    }

    pub fn get_focus_sessions(&self) -> Result<Vec<FocusSession>> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, task_id, duration_mins, xp_gained, completed_at, soundscape, owner_identity FROM focus_sessions")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let proj_str: Option<String> = row.get(1)?;
            let task_str: Option<String> = row.get(2)?;
            let duration_mins: i32 = row.get(3)?;
            let xp_gained: i32 = row.get(4)?;
            let completed_str: String = row.get(5)?;
            let soundscape: String = row.get(6)?;
            let owner_identity: Option<String> = row.get(7)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let project_id = proj_str.and_then(|s| Uuid::parse_str(&s).ok());
            let task_id = task_str.and_then(|s| Uuid::parse_str(&s).ok());
            let completed_at = DateTime::parse_from_rfc3339(&completed_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

            Ok(FocusSession {
                id,
                project_id,
                task_id,
                duration_mins,
                xp_gained,
                completed_at,
                soundscape,
                owner_identity,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_last_soundscape_used(&self) -> Result<String> {
        let res: Option<String> = self
            .conn
            .query_row(
                "SELECT soundscape FROM focus_sessions ORDER BY completed_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(res.unwrap_or_else(|| "None".to_string()))
    }

    pub fn get_favorite_soundscape(&self) -> Result<String> {
        let res: Option<String> = self.conn.query_row(
            "SELECT soundscape FROM focus_sessions GROUP BY soundscape ORDER BY count(*) DESC LIMIT 1",
            [],
            |row| row.get(0)
        ).optional()?;
        Ok(res.unwrap_or_else(|| "None".to_string()))
    }

    pub fn get_most_productive_soundscape(&self) -> Result<String> {
        let res: Option<String> = self.conn.query_row(
            "SELECT soundscape FROM focus_sessions GROUP BY soundscape ORDER BY SUM(xp_gained) DESC LIMIT 1",
            [],
            |row| row.get(0)
        ).optional()?;
        Ok(res.unwrap_or_else(|| "None".to_string()))
    }

    pub fn count_focus_sessions_with_soundscape(&self, s_names: &[&str]) -> Result<i32> {
        if s_names.is_empty() {
            return Ok(0);
        }
        let placeholders = s_names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT count(*) FROM focus_sessions WHERE soundscape IN ({})",
            placeholders
        );
        let mut stmt = self.conn.prepare(&query)?;

        let params = rusqlite::params_from_iter(s_names.iter().map(|s| s.to_string()));
        let count: i32 = stmt.query_row(params, |row| row.get(0))?;
        Ok(count)
    }

    pub fn count_unique_soundscapes_used(&self) -> Result<i32> {
        let count: i32 = self.conn.query_row(
            "SELECT count(distinct soundscape) FROM focus_sessions WHERE soundscape IN ('LoFi Radio', 'Ambient Radio', 'Forest Sounds', 'Rain Sounds', 'Ocean Waves', 'White Noise', 'Brown Noise', 'Silent')",
            [],
            |row| row.get(0)
        )?;
        Ok(count)
    }

    pub fn get_max_focus_session_duration(&self) -> Result<i32> {
        let max: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(duration_mins), 0) FROM focus_sessions",
            [],
            |row| row.get(0),
        )?;
        Ok(max)
    }

    pub fn insert_ritual(&self, r: &Ritual) -> Result<()> {
        self.conn.execute(
            "INSERT INTO rituals (id, name, description, frequency, reward_xp, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![r.id, r.name, r.description, r.frequency, r.reward_xp, r.created_at.to_rfc3339()],
        )?;
        let _ = self.log_change("ritual", &r.id, "create");
        Ok(())
    }

    pub fn get_rituals(&self) -> Result<Vec<Ritual>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, frequency, reward_xp, created_at FROM rituals",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let frequency: String = row.get(3)?;
            let reward_xp: i32 = row.get(4)?;
            let created_str: String = row.get(5)?;

            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

            Ok(Ritual {
                id,
                name,
                description,
                frequency,
                reward_xp,
                created_at,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn delete_ritual(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM rituals WHERE id = ?1", params![id])?;
        let _ = self.log_change("ritual", id, "delete");
        Ok(())
    }

    pub fn complete_ritual(&self, ritual_id: &str, date: NaiveDate) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO ritual_history (ritual_id, completed_date) VALUES (?1, ?2)",
            params![ritual_id, date.to_string()],
        )?;
        let _ = self.log_change("ritual_history", &format!("{}__{}", ritual_id, date), "create");
        Ok(())
    }

    pub fn get_ritual_history_for_date(&self, date: NaiveDate) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT ritual_id FROM ritual_history WHERE completed_date = ?1")?;
        let rows = stmt.query_map(params![date.to_string()], |row| row.get::<_, String>(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn insert_milestone(&self, m: &Milestone) -> Result<()> {
        self.conn.execute(
            "INSERT INTO milestones (id, project_id, name, description, completed, xp_reward, created_at, tier, template_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                m.id.to_string(),
                m.project_id.to_string(),
                m.name,
                m.description,
                if m.completed { 1 } else { 0 },
                m.xp_reward,
                m.created_at.to_rfc3339(),
                m.tier as i32,
                m.template_id,
            ],
        )?;
        let _ = self.log_change("milestone", &m.id.to_string(), "create");
        Ok(())
    }

    pub fn get_milestones_for_project(&self, project_id: Uuid) -> Result<Vec<Milestone>> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, name, description, completed, xp_reward, created_at, tier, template_id FROM milestones WHERE project_id = ?1")?;
        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let proj_str: String = row.get(1)?;
            let name: String = row.get(2)?;
            let description: Option<String> = row.get(3)?;
            let completed_int: i32 = row.get(4)?;
            let xp_reward: i32 = row.get(5)?;
            let created_str: String = row.get(6)?;
            let tier_int: i32 = row.get(7).unwrap_or(0);
            let template_id: String = row.get(8).unwrap_or_default();

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let project_id =
                Uuid::parse_str(&proj_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(Milestone {
                id,
                project_id,
                name,
                description,
                completed: completed_int != 0,
                xp_reward,
                created_at,
                tier: tier_int.clamp(0, 255) as u8,
                template_id,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_milestone(&self, m: &Milestone) -> Result<()> {
        self.conn.execute(
            "UPDATE milestones SET name = ?1, description = ?2, completed = ?3, xp_reward = ?4, tier = ?5, template_id = ?6 WHERE id = ?7",
            params![
                m.name,
                m.description,
                if m.completed { 1 } else { 0 },
                m.xp_reward,
                m.tier as i32,
                m.template_id,
                m.id.to_string(),
            ],
        )?;
        let _ = self.log_change("milestone", &m.id.to_string(), "update");
        Ok(())
    }

    pub fn delete_milestone(&self, id: Uuid) -> Result<()> {
        self.conn.execute(
            "DELETE FROM milestones WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    pub fn get_active_days_for_project(&self, project_id: Uuid) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM (
                SELECT date(entry_date) as d FROM journal_entries WHERE project_id = ?1
                UNION
                SELECT date(created_at) as d FROM tasks WHERE project_id = ?1
            )",
            params![project_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn get_daily_adventures_completed_count(&self) -> Result<i64> {
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM xp_events WHERE event_type LIKE 'Daily Quest:%'",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn get_unlocked_traits(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT id FROM traits")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn unlock_trait(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO traits (id, unlocked_at) VALUES (?1, ?2)",
            params![id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn insert_reflection(&self, ref_obj: &DailyReflection) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO reflections (created_date, what_went_well, what_can_improve) VALUES (?1, ?2, ?3)",
            params![ref_obj.created_date.to_string(), ref_obj.what_went_well, ref_obj.what_can_improve],
        )?;
        Ok(())
    }

    pub fn get_reflection_for_date(&self, date: NaiveDate) -> Result<Option<DailyReflection>> {
        let mut stmt = self.conn.prepare("SELECT created_date, what_went_well, what_can_improve FROM reflections WHERE created_date = ?1")?;
        let ref_opt = stmt
            .query_row(params![date.to_string()], |row| {
                let date_str: String = row.get(0)?;
                let what_went_well: String = row.get(1)?;
                let what_can_improve: String = row.get(2)?;
                let created_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
                Ok(DailyReflection {
                    created_date,
                    what_went_well,
                    what_can_improve,
                })
            })
            .optional()?;
        Ok(ref_opt)
    }

    pub fn get_reflections(&self) -> Result<Vec<DailyReflection>> {
        let mut stmt = self.conn.prepare("SELECT created_date, what_went_well, what_can_improve FROM reflections ORDER BY created_date DESC")?;
        let rows = stmt.query_map([], |row| {
            let date_str: String = row.get(0)?;
            let what_went_well: String = row.get(1)?;
            let what_can_improve: String = row.get(2)?;
            let created_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            Ok(DailyReflection {
                created_date,
                what_went_well,
                what_can_improve,
            })
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_most_productive_project(&self) -> Result<String> {
        let mut stmt = self.conn.prepare(
            "
            SELECT p.name, COUNT(t.id) as completed_count
            FROM projects p
            JOIN tasks t ON p.id = t.project_id
            WHERE t.completed = 1
            GROUP BY p.id
            ORDER BY completed_count DESC
            LIMIT 1
        ",
        )?;
        let name_opt = stmt
            .query_row([], |row| row.get::<_, String>(0))
            .optional()?;
        Ok(name_opt.unwrap_or_else(|| "None yet".to_string()))
    }

    pub fn get_xp_history(&self) -> Result<Vec<XPEvent>> {
        let mut stmt = self.conn.prepare("SELECT id, event_type, xp_gained, timestamp FROM xp_events ORDER BY timestamp DESC LIMIT 5")?;
        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let event_type: String = row.get(1)?;
            let xp_gained: i32 = row.get(2)?;
            let timestamp_str: String = row.get(3)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;

            Ok(XPEvent {
                id,
                event_type,
                xp_gained,
                timestamp,
            })
        })?;

        let mut events = Vec::new();
        for r in rows {
            events.push(r?);
        }
        Ok(events)
    }

    pub fn get_task_by_id(&self, id: Uuid) -> Result<Task> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, title, description, due_date, completed, priority, created_at, updated_at, owner_identity, owner_username, parent_task_id, xp_awarded, recurrence FROM tasks WHERE id = ?1")?;
        let task = stmt.query_row(params![id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: Option<String> = row.get(1)?;
            let title: String = row.get(2)?;
            let description: Option<String> = row.get(3)?;
            let due_str: Option<String> = row.get(4)?;
            let completed_int: i32 = row.get(5)?;
            let priority_str: String = row.get(6)?;
            let created_str: String = row.get(7)?;
            let updated_str: String = row.get(8)?;
            let owner_identity: Option<String> = row.get(9)?;
            let owner_username: Option<String> = row.get(10)?;
            let parent_task_id_str: Option<String> = row.get(11)?;
            let xp_awarded_int: i32 = row.get(12)?;
            let recurrence_str: Option<String> = row.get(13)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let project_id = match project_id_str {
                Some(p) => {
                    Some(Uuid::parse_str(&p).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?)
                }
                None => None,
            };
            let due_date = match due_str {
                Some(d) => Some(
                    DateTime::parse_from_rfc3339(&d)
                        .map(|dt| dt.with_timezone(&Utc))
                        .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?,
                ),
                None => None,
            };
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(created_at);
            let priority = TaskPriority::from_str(&priority_str);
            let parent_task_id = match parent_task_id_str {
                Some(p) => Some(Uuid::parse_str(&p).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };

            Ok(Task {
                id,
                project_id,
                title,
                description,
                due_date,
                completed: completed_int != 0,
                priority,
                created_at,
                updated_at,
                owner_identity,
                owner_username,
                parent_task_id,
                xp_awarded: xp_awarded_int != 0,
                recurrence: recurrence_str.as_deref().and_then(RecurrenceType::from_str),
            })
        })?;
        Ok(task)
    }

    pub fn get_note_by_id(&self, id: Uuid) -> Result<Note> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, title, markdown_content, created_at, updated_at, sharing_permission, codex_id, owner_identity FROM notes WHERE id = ?1")?;
        let note = stmt.query_row(params![id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let project_id_str: Option<String> = row.get(1)?;
            let title: String = row.get(2)?;
            let content: String = row.get(3)?;
            let created_str: String = row.get(4)?;
            let updated_str: String = row.get(5)?;
            let sharing_permission: String = row.get(6)?;
            let codex_id_str: Option<String> = row.get(7)?;
            let owner_identity: Option<String> = row.get(8)?;

            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let project_id = match project_id_str {
                Some(p) => {
                    Some(Uuid::parse_str(&p).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?)
                }
                None => None,
            };
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let updated_at = DateTime::parse_from_rfc3339(&updated_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let codex_id = match codex_id_str {
                Some(c) => Some(Uuid::parse_str(&c).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };

            Ok(Note {
                id,
                project_id,
                title,
                markdown_content: content,
                created_at,
                updated_at,
                sharing_permission,
                codex_id,
                owner_identity,
            })
        })?;
        Ok(note)
    }

    pub fn insert_codex(&self, codex: &Codex) -> Result<()> {
        self.conn.execute(
            "INSERT INTO codices (id, project_id, name, created_at, parent_codex_id, collapsed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                codex.id.to_string(),
                codex.project_id.to_string(),
                codex.name,
                codex.created_at.to_rfc3339(),
                codex.parent_codex_id.map(|id| id.to_string()),
                codex.collapsed as i32,
            ],
        )?;
        let _ = self.log_change("codex", &codex.id.to_string(), "create");
        if let Ok(json) = serde_json::to_string(codex) {
            let _ = self.create_revision("codex", &codex.id.to_string(), &json);
        }
        Ok(())
    }

    pub fn get_codices_for_project(&self, project_id: Uuid) -> Result<Vec<Codex>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, name, created_at, parent_codex_id, collapsed FROM codices WHERE project_id = ?1 ORDER BY LOWER(name) ASC"
        )?;
        let rows = stmt.query_map(params![project_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let pid_str: String = row.get(1)?;
            let name: String = row.get(2)?;
            let created_str: String = row.get(3)?;
            let parent_str: Option<String> = row.get(4)?;
            let collapsed: i32 = row.get(5).unwrap_or(0);
            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let pid = Uuid::parse_str(&pid_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let parent_codex_id = match parent_str {
                Some(s) => Some(Uuid::parse_str(&s).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };
            Ok(Codex { id, project_id: pid, name, created_at, parent_codex_id, collapsed: collapsed != 0 })
        })?;
        let mut list = Vec::new();
        for r in rows { list.push(r?); }
        Ok(list)
    }

    pub fn get_codex_by_id(&self, id: &str) -> Result<Codex> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, name, created_at, parent_codex_id, collapsed FROM codices WHERE id = ?1"
        )?;
        let codex = stmt.query_row(params![id], |row| {
            let id_str: String = row.get(0)?;
            let pid_str: String = row.get(1)?;
            let name: String = row.get(2)?;
            let created_str: String = row.get(3)?;
            let parent_str: Option<String> = row.get(4)?;
            let collapsed: i32 = row.get(5).unwrap_or(0);
            let id = Uuid::parse_str(&id_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let pid = Uuid::parse_str(&pid_str).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            let parent_codex_id = match parent_str {
                Some(s) => Some(Uuid::parse_str(&s).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?),
                None => None,
            };
            Ok(Codex { id, project_id: pid, name, created_at, parent_codex_id, collapsed: collapsed != 0 })
        })?;
        Ok(codex)
    }

    pub fn set_codex_collapsed(&self, id: Uuid, collapsed: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE codices SET collapsed = ?1 WHERE id = ?2",
            params![collapsed as i32, id.to_string()],
        )?;
        let _ = self.log_change("codex", &id.to_string(), "update");
        Ok(())
    }

    pub fn update_codex_name(&self, id: Uuid, name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE codices SET name = ?1 WHERE id = ?2",
            params![name, id.to_string()],
        )?;
        let _ = self.log_change("codex", &id.to_string(), "update");
        Ok(())
    }

    pub fn update_codex_parent(&self, id: Uuid, parent_codex_id: Option<Uuid>) -> Result<()> {
        self.conn.execute(
            "UPDATE codices SET parent_codex_id = ?1 WHERE id = ?2",
            params![parent_codex_id.map(|p| p.to_string()), id.to_string()],
        )?;
        let _ = self.log_change("codex", &id.to_string(), "update");
        Ok(())
    }

    pub fn delete_codex(&self, id: Uuid) -> Result<()> {
        // los sub-codices huérfanos suben a raíz en vez de borrarse — el usuario no pierde jerarquía de golpe
        self.conn.execute(
            "UPDATE codices SET parent_codex_id = NULL WHERE parent_codex_id = ?1",
            params![id.to_string()],
        )?;
        // el ON DELETE SET NULL del schema ya desagrupa las notas — no hace falta UPDATE manual aquí
        self.conn.execute(
            "DELETE FROM codices WHERE id = ?1",
            params![id.to_string()],
        )?;
        let _ = self.log_change("codex", &id.to_string(), "delete");
        Ok(())
    }

    pub fn count_codices(&self) -> Result<i32> {
        let count: i32 = self.conn.query_row(
            "SELECT count(*) FROM codices",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    // synced=0 al insertar — el motor de sync sólo toma los pendientes; no tocar este default
    pub fn log_change(&self, entity_type: &str, entity_id: &str, operation: &str) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO sync_log (id, entity_type, entity_id, operation, timestamp, synced) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![id, entity_type, entity_id, operation, timestamp],
        )?;
        Ok(())
    }

    // el número de revisión se calcula como MAX+1 por entidad, no global — dos entidades distintas pueden tener rev 1
    pub fn create_revision(&self, entity_type: &str, entity_id: &str, content: &str) -> Result<()> {
        let next_rev: i32 = self.conn.query_row(
            "SELECT COALESCE(MAX(revision_number), 0) + 1 FROM revisions WHERE entity_type = ?1 AND entity_id = ?2",
            params![entity_type, entity_id],
            |row| row.get(0),
        )?;
        let id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO revisions (id, entity_type, entity_id, revision_number, content, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, entity_type, entity_id, next_rev, content, timestamp],
        )?;
        Ok(())
    }

    pub fn get_revisions(
        &self,
        entity_type: &str,
        entity_id: &str,
    ) -> Result<Vec<(i32, String, String)>> {
        let mut stmt = self.conn.prepare("SELECT revision_number, content, timestamp FROM revisions WHERE entity_type = ?1 AND entity_id = ?2 ORDER BY revision_number DESC")?;
        let rows = stmt.query_map(params![entity_type, entity_id], |row| {
            let num: i32 = row.get(0)?;
            let content: String = row.get(1)?;
            let ts: String = row.get(2)?;
            Ok((num, content, ts))
        })?;
        let mut revs = Vec::new();
        for r in rows {
            revs.push(r?);
        }
        Ok(revs)
    }

    pub fn register_device(&self, device_id: &str, device_name: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO devices (device_id, device_name, created_at, last_sync) VALUES (?1, ?2, ?3, NULL)",
            params![device_id, device_name, now_str],
        )?;
        Ok(())
    }

    pub fn get_devices(&self) -> Result<Vec<(String, String, String, Option<String>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT device_id, device_name, created_at, last_sync FROM devices")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let created: String = row.get(2)?;
            let last: Option<String> = row.get(3)?;
            Ok((id, name, created, last))
        })?;
        let mut dev = Vec::new();
        for r in rows {
            dev.push(r?);
        }
        Ok(dev)
    }

    pub fn update_device_sync_time(&self, device_id: &str) -> Result<()> {
        let now_str = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE devices SET last_sync = ?1 WHERE device_id = ?2",
            params![now_str, device_id],
        )?;
        Ok(())
    }

    /// Aplica un registro de dispositivo recibido desde otro nodo — upsert seguro.
    pub fn upsert_remote_device(&self, device_id: &str, device_name: &str, last_sync: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO devices (device_id, device_name, created_at, last_sync)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(device_id) DO UPDATE SET device_name = excluded.device_name, last_sync = excluded.last_sync",
            params![device_id, device_name, Utc::now().to_rfc3339(), last_sync],
        )?;
        Ok(())
    }

    /// Cuenta cuántos dispositivos han sincronizado en los últimos N minutos (activos ahora mismo).
    pub fn count_active_devices(&self, within_minutes: i64) -> Result<usize> {
        let cutoff = (Utc::now() - chrono::Duration::minutes(within_minutes)).to_rfc3339();
        let count: i64 = self.conn.query_row(
            "SELECT count(*) FROM devices WHERE last_sync > ?1",
            params![cutoff],
            |r| r.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn get_pending_sync_logs(&self) -> Result<Vec<(String, String, String, String, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, entity_type, entity_id, operation, timestamp FROM sync_log WHERE synced = 0")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let et: String = row.get(1)?;
            let ei: String = row.get(2)?;
            let op: String = row.get(3)?;
            let ts: String = row.get(4)?;
            Ok((id, et, ei, op, ts))
        })?;
        let mut logs = Vec::new();
        for r in rows {
            logs.push(r?);
        }
        Ok(logs)
    }

    // el IN dinámico con placeholders numerados es necesario porque rusqlite no acepta slices directamente en execute
    pub fn mark_sync_logs_synced(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let placeholders = ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect::<Vec<_>>().join(",");
        let sql = format!("UPDATE sync_log SET synced = 1 WHERE id IN ({})", placeholders);
        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        stmt.execute(params.as_slice())?;
        Ok(())
    }

    /// Carga todos los IDs de eventos remotos ya procesados en un HashSet para dedup O(1) durante el pull.
    pub fn load_processed_remote_ids(&self) -> Result<std::collections::HashSet<String>> {
        let mut stmt = self.conn.prepare("SELECT id FROM processed_remote_events")?;
        let ids = stmt.query_map([], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(ids)
    }

    /// Registra los IDs de eventos remotos recién aplicados para no repetirlos en futuros syncs.
    pub fn mark_remote_events_processed(&self, ids: &[String]) -> Result<()> {
        if ids.is_empty() { return Ok(()); }
        let now = Utc::now().to_rfc3339();
        for id in ids {
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO processed_remote_events (id, processed_at) VALUES (?1, ?2)",
                params![id, now],
            );
        }
        Ok(())
    }

    /// Limpia entradas antiguas de processed_remote_events para no crecer indefinidamente.
    pub fn cleanup_processed_remote_events(&self, days: i64) -> Result<()> {
        let cutoff = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        let _ = self.conn.execute(
            "DELETE FROM processed_remote_events WHERE processed_at < ?1",
            params![cutoff],
        );
        Ok(())
    }

    pub fn cleanup_old_sync_logs(&self, days: i64) -> Result<usize> {
        let cutoff = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        let deleted = self.conn.execute(
            "DELETE FROM sync_log WHERE synced = 1 AND timestamp < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }

    pub fn count_prunable_tasks(&self, days: i64) -> Result<usize> {
        let cutoff = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE completed = 1 AND (
                (updated_at != '' AND updated_at < ?1) OR
                (updated_at = '' AND created_at < ?1)
            )",
            params![cutoff],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn prune_completed_tasks(&self, days: i64) -> Result<usize> {
        let cutoff = (Utc::now() - chrono::Duration::days(days)).to_rfc3339();

        // CUIDADO: los tombstones van antes del DELETE — si se hace al revés, otros dispositivos resucitan la tarea en el siguiente pull
        let mut stmt = self.conn.prepare(
            "SELECT id FROM tasks WHERE completed = 1 AND (
                (updated_at != '' AND updated_at < ?1) OR
                (updated_at = '' AND created_at < ?1)
            )",
        )?;
        let ids: Vec<String> = stmt
            .query_map(params![cutoff], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        let count = ids.len();
        for id in &ids {
            let _ = self.log_change("task", id, "delete");
        }

        // el CASCADE del schema borra las subtareas automáticamente al borrar la tarea raíz
        self.conn.execute(
            "DELETE FROM tasks WHERE completed = 1 AND parent_task_id IS NULL AND (
                (updated_at != '' AND updated_at < ?1) OR
                (updated_at = '' AND created_at < ?1)
            )",
            params![cutoff],
        )?;
        // subtareas completadas cuyo padre sobrevivió la poda — hay que borrarlas por separado
        self.conn.execute(
            "DELETE FROM tasks WHERE completed = 1 AND parent_task_id IS NOT NULL AND (
                (updated_at != '' AND updated_at < ?1) OR
                (updated_at = '' AND created_at < ?1)
            )",
            params![cutoff],
        )?;

        Ok(count)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let val_opt = stmt
            .query_row(params![key], |row| row.get::<_, String>(0))
            .optional()?;
        Ok(val_opt)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn export_to_json(&self) -> Result<String> {
        let mut map = serde_json::Map::new();
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "app_name".to_string(),
            serde_json::Value::String("Questline".to_string()),
        );
        metadata.insert(
            "version".to_string(),
            serde_json::Value::String(env!("CARGO_PKG_VERSION").to_string()),
        );
        metadata.insert(
            "export_date".to_string(),
            serde_json::Value::String(Utc::now().to_rfc3339()),
        );

        if let Ok(Some(u)) = self.get_user() {
            let mut profile = serde_json::Map::new();
            profile.insert(
                "username".to_string(),
                serde_json::Value::String(u.username),
            );
            profile.insert(
                "class".to_string(),
                serde_json::Value::String(u.class.name().to_string()),
            );
            profile.insert(
                "level".to_string(),
                serde_json::Value::Number(u.level.into()),
            );
            profile.insert("xp".to_string(), serde_json::Value::Number(u.xp.into()));
            profile.insert(
                "specialization".to_string(),
                match u.specialization {
                    Some(s) => serde_json::Value::String(s),
                    None => serde_json::Value::Null,
                },
            );
            metadata.insert(
                "character_profile".to_string(),
                serde_json::Value::Object(profile),
            );
        }
        map.insert("_metadata".to_string(), serde_json::Value::Object(metadata));

        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )?;
        let tables: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        for table in tables {
            let mut rows_list = Vec::new();
            let mut row_stmt = self.conn.prepare(&format!("SELECT * FROM {}", table))?;
            let col_names: Vec<String> = row_stmt
                .column_names()
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            let col_count = row_stmt.column_count();

            let mut rows = row_stmt.query([])?;
            while let Some(row) = rows.next()? {
                let mut row_map = serde_json::Map::new();
                for i in 0..col_count {
                    let val_ref = row.get_ref(i)?;
                    let val = match val_ref {
                        rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                        rusqlite::types::ValueRef::Integer(n) => {
                            serde_json::Value::Number(n.into())
                        }
                        rusqlite::types::ValueRef::Real(f) => {
                            if let Some(num) = serde_json::Number::from_f64(f) {
                                serde_json::Value::Number(num)
                            } else {
                                serde_json::Value::Null
                            }
                        }
                        rusqlite::types::ValueRef::Text(t) => {
                            let s = String::from_utf8_lossy(t).into_owned();
                            serde_json::Value::String(s)
                        }
                        rusqlite::types::ValueRef::Blob(b) => {
                            let hex_str: String = b.iter().map(|x| format!("{:02x}", x)).collect();
                            serde_json::Value::String(hex_str)
                        }
                    };
                    row_map.insert(col_names[i].clone(), val);
                }
                rows_list.push(serde_json::Value::Object(row_map));
            }
            map.insert(table, serde_json::Value::Array(rows_list));
        }

        Ok(serde_json::to_string_pretty(&serde_json::Value::Object(
            map,
        ))?)
    }

    /// Importa datos desde un JSON de export hacia el esquema actual.
    /// Las columnas del JSON que no existen en el esquema actual se ignoran — esto permite migrar
    /// de un schema viejo a uno nuevo sin romper el import cuando se renombran o eliminan columnas.
    /// Las tablas del JSON que no existen en el schema nuevo también se saltan silenciosamente.
    pub fn import_from_json(&self, json_str: &str) -> Result<()> {
        let value: serde_json::Value = serde_json::from_str(json_str)?;
        let map = value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid export format: root must be an object"))?;

        // FK off para importar sin importar el orden de tablas — se reactiva al final pase lo que pase
        self.conn.execute("PRAGMA foreign_keys = OFF;", [])?;

        let res = (|| -> Result<()> {
            for (table_name, rows_val) in map {
                if table_name.starts_with('_') {
                    continue;
                }
                if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return Err(anyhow::anyhow!("Invalid table name: {}", table_name));
                }

                // Si la tabla ya no existe en el nuevo schema, se salta sin error
                let table_exists: bool = self.conn.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    params![table_name],
                    |r| r.get::<_, i32>(0).map(|c| c > 0),
                )?;
                if !table_exists {
                    continue;
                }

                // Obtiene las columnas que realmente existen en la tabla del schema actual
                let mut col_stmt = self.conn.prepare(
                    &format!("SELECT name FROM pragma_table_info('{}')", table_name),
                )?;
                let existing_cols: std::collections::HashSet<String> = col_stmt
                    .query_map([], |r| r.get::<_, String>(0))?
                    .filter_map(|r| r.ok())
                    .collect();

                self.conn
                    .execute(&format!("DELETE FROM {}", table_name), [])?;

                let rows = rows_val
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Table data must be an array"))?;
                for row_val in rows {
                    let row_obj = row_val
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("Row data must be an object"))?;
                    let mut cols = Vec::new();
                    let mut placeholders = Vec::new();
                    let mut vals: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

                    for (col, val) in row_obj {
                        // Columnas del export que ya no existen en el schema nuevo se ignoran
                        if !existing_cols.contains(col) {
                            continue;
                        }
                        cols.push(col.clone());
                        placeholders.push(format!("?{}", cols.len()));

                        let to_sql: Box<dyn rusqlite::ToSql> = match val {
                            serde_json::Value::Null => Box::new(rusqlite::types::Null),
                            serde_json::Value::Bool(b) => {
                                let v: bool = *b;
                                Box::new(if v { 1 } else { 0 })
                            }
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    Box::new(i)
                                } else if let Some(f) = n.as_f64() {
                                    Box::new(f)
                                } else {
                                    Box::new(rusqlite::types::Null)
                                }
                            }
                            serde_json::Value::String(s) => Box::new(s.clone()),
                            serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
                                Box::new(val.to_string())
                            }
                        };
                        vals.push(to_sql);
                    }

                    if cols.is_empty() {
                        continue;
                    }

                    let sql = format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        table_name,
                        cols.join(", "),
                        placeholders.join(", ")
                    );

                    let params: Vec<&dyn rusqlite::ToSql> =
                        vals.iter().map(|v| v.as_ref()).collect();
                    self.conn.execute(&sql, params.as_slice())?;
                }
            }
            Ok(())
        })();

        self.conn.execute("PRAGMA foreign_keys = ON;", [])?;
        res
    }

    pub fn get_recent_revisions(&self) -> Result<Vec<(String, String, String, i32, String)>> {
        let mut stmt = self.conn.prepare("SELECT entity_id, entity_type, content, revision_number, timestamp FROM revisions ORDER BY timestamp DESC LIMIT 10")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let et: String = row.get(1)?;
            let content: String = row.get(2)?;
            let num: i32 = row.get(3)?;
            let ts: String = row.get(4)?;
            Ok((id, et, content, num, ts))
        })?;
        let mut revs = Vec::new();
        for r in rows {
            revs.push(r?);
        }
        Ok(revs)
    }

    pub fn add_project_member(
        &self,
        project_id: &str,
        identity: &str,
        username: &str,
        role: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO project_members (project_id, user_identity, user_username, role) VALUES (?1, ?2, ?3, ?4)",
            params![project_id, identity, username, role],
        )?;
        let compound_id = format!("{}__{}", project_id, identity);
        let _ = self.log_change("project_member", &compound_id, "add");
        Ok(())
    }

    /// Retorna todos los miembros de un proyecto con su estado de presencia actual.
    /// El campo `is_online` se deriva: online=1 y last_seen dentro de los últimos 10 minutos.
    pub fn get_presence_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<(String, String, String, bool, String, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT pm.user_identity, pm.user_username, pm.role,
                    COALESCE(p.online, 0), COALESCE(p.last_seen, ''), p.current_project
             FROM project_members pm
             LEFT JOIN presence p ON p.user_identity = pm.user_identity
             WHERE pm.project_id = ?1
             ORDER BY COALESCE(p.online, 0) DESC, pm.user_username ASC",
        )?;
        let cutoff = (Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
        let rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            let (identity, username, role, online_int, last_seen, current_proj) = r?;
            let is_online = if online_int != 0 {
                // Validamos con el timestamp — si no es parseable asumimos que viene del server y está bien
                let stale = chrono::DateTime::parse_from_rfc3339(&last_seen)
                    .map(|dt| dt.to_rfc3339() < cutoff)
                    .unwrap_or(false);
                !stale
            } else {
                false
            };
            list.push((identity, username, role, is_online, last_seen, current_proj));
        }
        Ok(list)
    }

    /// Marca como offline a todos los usuarios cuyo last_seen sea más viejo que el cutoff.
    pub fn mark_stale_presence_offline(&self, older_than_minutes: i64) -> Result<()> {
        let cutoff = (Utc::now() - chrono::Duration::minutes(older_than_minutes)).to_rfc3339();
        let _ = self.conn.execute(
            "UPDATE presence SET online = 0 WHERE online = 1 AND last_seen < ?1 AND length(last_seen) > 10",
            params![cutoff],
        );
        Ok(())
    }

    pub fn get_project_members(&self, project_id: &str) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT user_identity, user_username, role FROM project_members WHERE project_id = ?1",
        )?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_member_role(&self, project_id: &str, identity: &str) -> Result<Option<String>> {
        let role: Option<String> = self
            .conn
            .query_row(
                "SELECT role FROM project_members WHERE project_id = ?1 AND user_identity = ?2",
                params![project_id, identity],
                |row| row.get(0),
            )
            .optional()?;
        Ok(role)
    }

    pub fn remove_project_member(&self, project_id: &str, identity: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM project_members WHERE project_id = ?1 AND user_identity = ?2",
            params![project_id, identity],
        )?;
        Ok(())
    }

    pub fn create_invitation(
        &self,
        project_id: &str,
        project_name: &str,
        inviter_id: &str,
        inviter_name: &str,
        invitee_id: &str,
        role: &str,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO invitations (id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'Pending', ?8)",
            params![id, project_id, project_name, inviter_id, inviter_name, invitee_id, role, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_invitations(
        &self,
    ) -> Result<
        Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        )>,
    > {
        let mut stmt = self.conn.prepare("SELECT id, project_id, project_name, inviter_identity, inviter_username, invitee_identity, role, status, created_at FROM invitations ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_invitation_status(&self, invite_id: &str, status: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE invitations SET status = ?1 WHERE id = ?2",
            params![status, invite_id],
        )?;
        Ok(())
    }

    pub fn add_chronicle_message(
        &self,
        project_id: &str,
        sender_identity: &str,
        sender_username: &str,
        content: &str,
        msg_type: &str,
    ) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let ts = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO chronicle_messages (id, project_id, sender_identity, sender_username, content, message_type, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, project_id, sender_identity, sender_username, content, msg_type, ts],
        )?;
        let _ = self.log_change("chronicle_message", &id, "create");
        Ok(id)
    }

    pub fn get_chronicle_messages(
        &self,
        project_id: &str,
    ) -> Result<Vec<(String, String, String, String, String, String, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, project_id, sender_identity, sender_username, content, message_type, timestamp FROM chronicle_messages WHERE project_id = ?1 ORDER BY timestamp ASC")?;
        let rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn search_chronicle_messages(
        &self,
        query: &str,
    ) -> Result<
        Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
        )>,
    > {
        let query_param = format!("%{}%", query);
        let mut stmt = self.conn.prepare("SELECT m.id, m.project_id, m.sender_identity, m.sender_username, m.content, m.message_type, m.timestamp, p.name FROM chronicle_messages m JOIN projects p ON m.project_id = p.id WHERE m.content LIKE ?1 OR m.sender_username LIKE ?1 ORDER BY m.timestamp DESC")?;
        let rows = stmt.query_map(params![query_param], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn add_message_reaction(
        &self,
        message_id: &str,
        user_identity: &str,
        emoji: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO message_reactions (message_id, user_identity, emoji) VALUES (?1, ?2, ?3)",
            params![message_id, user_identity, emoji],
        )?;
        Ok(())
    }

    pub fn get_message_reactions(&self, message_id: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT user_identity, emoji FROM message_reactions WHERE message_id = ?1")?;
        let rows = stmt.query_map(params![message_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn log_activity(
        &self,
        project_id: Option<&str>,
        event_type: &str,
        description: &str,
        user_identity: &str,
        user_username: &str,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO activity_log (id, project_id, event_type, description, user_identity, user_username, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, project_id, event_type, description, user_identity, user_username, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_recent_activities(
        &self,
        limit: i32,
    ) -> Result<
        Vec<(
            String,
            Option<String>,
            String,
            String,
            String,
            String,
            String,
        )>,
    > {
        let mut stmt = self.conn.prepare("SELECT id, project_id, event_type, description, user_identity, user_username, timestamp FROM activity_log ORDER BY timestamp DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_activity_log_for_project(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<(String, Option<String>, String, String, String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, event_type, description, user_identity, user_username, timestamp FROM activity_log WHERE project_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![project_id, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn create_notification(
        &self,
        notif_type: &str,
        title: &str,
        content: &str,
        target_id: Option<&str>,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO notifications (id, notification_type, title, content, target_id, read, created_at) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
            params![id, notif_type, title, content, target_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_notifications(
        &self,
    ) -> Result<Vec<(String, String, String, String, Option<String>, bool, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, notification_type, title, content, target_id, read, created_at FROM notifications ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], |row| {
            let read_int: i32 = row.get(5)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                read_int != 0,
                row.get::<_, String>(6)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn mark_notification_read(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE notifications SET read = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn mark_all_notifications_read(&self) -> Result<()> {
        self.conn.execute("UPDATE notifications SET read = 1", [])?;
        Ok(())
    }

    pub fn get_unread_notifications_count(&self) -> Result<i32> {
        let count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM notifications WHERE read = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn assign_task(
        &self,
        task_id: &str,
        user_identity: &str,
        user_username: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO task_assignments (task_id, user_identity, user_username) VALUES (?1, ?2, ?3)",
            params![task_id, user_identity, user_username],
        )?;
        let compound_id = format!("{}__{}", task_id, user_identity);
        let _ = self.log_change("task_assignment", &compound_id, "assign");
        Ok(())
    }

    pub fn get_task_assignments(&self, task_id: &str) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT user_identity, user_username FROM task_assignments WHERE task_id = ?1",
        )?;
        let rows = stmt.query_map(params![task_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn clear_task_assignments(&self, task_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM task_assignments WHERE task_id = ?1",
            params![task_id],
        )?;
        Ok(())
    }

    pub fn update_presence(
        &self,
        user_identity: &str,
        username: &str,
        online: bool,
        last_seen: &str,
        current_project: Option<&str>,
        privacy: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO presence (user_identity, username, online, last_seen, current_project, privacy_status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                user_identity,
                username,
                if online { 1 } else { 0 },
                last_seen,
                current_project,
                privacy
            ],
        )?;
        Ok(())
    }

    pub fn get_presence_list(
        &self,
    ) -> Result<Vec<(String, String, bool, String, Option<String>, String)>> {
        let mut stmt = self.conn.prepare("SELECT user_identity, username, online, last_seen, current_project, privacy_status FROM presence ORDER BY username ASC")?;
        let rows = stmt.query_map([], |row| {
            let online_int: i32 = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                online_int != 0,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn add_chronicle_entry(&self, day_number: i32, text: &str) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO great_chronicle (id, day_number, entry_text, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![id, day_number, text, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_chronicle_entries(&self) -> Result<Vec<(String, i32, String, String)>> {
        let mut stmt = self.conn.prepare("SELECT id, day_number, entry_text, timestamp FROM great_chronicle ORDER BY timestamp DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_class_quests(
        &self,
        class_name: &str,
    ) -> Result<Vec<(String, i32, String, String, String, i32, i32, String)>> {
        let mut stmt = self.conn.prepare("SELECT class_name, unlock_level, quest_name, description, status, progress, target, lore_reward FROM class_quests WHERE class_name = ?1 ORDER BY unlock_level ASC")?;
        let rows = stmt.query_map(params![class_name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, i32>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_class_quest_progress(
        &self,
        class_name: &str,
        unlock_level: i32,
        progress: i32,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE class_quests SET progress = ?1 WHERE class_name = ?2 AND unlock_level = ?3",
            params![progress, class_name, unlock_level],
        )?;
        Ok(())
    }

    pub fn complete_class_quest(&self, class_name: &str, unlock_level: i32) -> Result<()> {
        self.conn.execute(
            "UPDATE class_quests SET status = 'Completed' WHERE class_name = ?1 AND unlock_level = ?2",
            params![class_name, unlock_level],
        )?;
        Ok(())
    }

    pub fn start_class_quest(&self, class_name: &str, unlock_level: i32) -> Result<()> {
        self.conn.execute(
            "UPDATE class_quests SET status = 'Active' WHERE class_name = ?1 AND unlock_level = ?2",
            params![class_name, unlock_level],
        )?;
        Ok(())
    }

    pub fn activate_class_quests_up_to_level(&self, class_name: &str, level: i32) -> Result<()> {
        self.conn.execute(
            "UPDATE class_quests SET status = 'Available' WHERE class_name = ?1 AND unlock_level <= ?2 AND status = 'Locked'",
            params![class_name, level],
        )?;
        Ok(())
    }

    pub fn get_legendary_titles(&self) -> Result<Vec<(String, String, String, bool, bool)>> {
        let mut stmt = self.conn.prepare("SELECT title_id, title_name, description, unlocked, equipped FROM legendary_titles ORDER BY title_name ASC")?;
        let rows = stmt.query_map([], |row| {
            let unlocked_int: i32 = row.get(3)?;
            let equipped_int: i32 = row.get(4)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                unlocked_int != 0,
                equipped_int != 0,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn unlock_legendary_title(&self, title_id: &str) -> Result<bool> {
        let already_unlocked: Option<i32> = self
            .conn
            .query_row(
                "SELECT unlocked FROM legendary_titles WHERE title_id = ?1",
                params![title_id],
                |row| row.get(0),
            )
            .optional()?;

        match already_unlocked {
            None => Ok(false),
            Some(1) => Ok(false),
            Some(_) => {
                let changed = self.conn.execute(
                    "UPDATE legendary_titles SET unlocked = 1 WHERE title_id = ?1 AND unlocked = 0",
                    params![title_id],
                )?;
                Ok(changed > 0)
            }
        }
    }

    pub fn equip_legendary_title(&self, title_id: Option<&str>) -> Result<()> {
        self.conn
            .execute("UPDATE legendary_titles SET equipped = 0", [])?;
        if let Some(tid) = title_id {
            self.conn.execute(
                "UPDATE legendary_titles SET equipped = 1 WHERE title_id = ?1 AND unlocked = 1",
                params![tid],
            )?;
        }
        Ok(())
    }

    pub fn get_equipped_legendary_title(&self) -> Result<Option<String>> {
        let title_name: Option<String> = self
            .conn
            .query_row(
                "SELECT title_name FROM legendary_titles WHERE equipped = 1 LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(title_name)
    }

    pub fn get_relics(&self) -> Result<Vec<(String, String, String, bool, Option<String>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, unlocked, unlocked_at FROM relics ORDER BY name ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let unlocked_int: i32 = row.get(3)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                unlocked_int != 0,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn unlock_relic(&self, id: &str) -> Result<bool> {
        let already_unlocked: Option<i32> = self
            .conn
            .query_row(
                "SELECT unlocked FROM relics WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;

        match already_unlocked {
            None => Ok(false),
            Some(1) => Ok(false),
            Some(_) => {
                let changed = self.conn.execute(
                    "UPDATE relics SET unlocked = 1, unlocked_at = ?2 WHERE id = ?1 AND unlocked = 0",
                    params![id, Utc::now().to_rfc3339()],
                )?;
                Ok(changed > 0)
            }
        }
    }

    pub fn add_companion_lore(&self, text: &str) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO companion_lore (id, story_text, timestamp) VALUES (?1, ?2, ?3)",
            params![id, text, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn get_companion_lore(&self) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, story_text, timestamp FROM companion_lore ORDER BY timestamp DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_lore_entries(
        &self,
    ) -> Result<Vec<(String, String, String, String, bool, Option<String>)>> {
        let mut stmt = self.conn.prepare("SELECT id, category, title, content, unlocked, unlocked_at FROM lore_library ORDER BY title ASC")?;
        let rows = stmt.query_map([], |row| {
            let unlocked_int: i32 = row.get(4)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                unlocked_int != 0,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }

        // orden custom: por categoría primero; World/Class se ordenan por número del ID, no por título — cambiarlo rompe la narrativa
        list.sort_by(|a, b| {
            if a.1 != b.1 {
                a.1.cmp(&b.1)
            } else if a.1 == "World" {
                let a_num = a.0.strip_prefix("world_chapter_")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(999);
                let b_num = b.0.strip_prefix("world_chapter_")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(999);
                a_num.cmp(&b_num)
            } else if a.1 == "Class" {
                // Ordena por clase primero (shared al inicio y al final), luego por nivel
                let class_sort_key = |id: &str| -> (usize, usize) {
                    if id == "class_six_orders" { return (0, 0); }
                    if id == "class_council_orders" { return (99, 0); }
                    let class_order = if id.starts_with("class_warlock_") { 1 }
                        else if id.starts_with("class_paladin_") { 2 }
                        else if id.starts_with("class_sage_") { 3 }
                        else if id.starts_with("class_architect_") { 4 }
                        else if id.starts_with("class_chronomancer_") { 5 }
                        else if id.starts_with("class_accountant_") { 6 }
                        else { 50 };
                    let level = id.split('_').last()
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0);
                    (class_order, level)
                };
                class_sort_key(&a.0).cmp(&class_sort_key(&b.0))
            } else {
                let title_cmp = a.2.cmp(&b.2);
                if title_cmp == std::cmp::Ordering::Equal {
                    a.0.cmp(&b.0)
                } else {
                    title_cmp
                }
            }
        });

        Ok(list)
    }

    pub fn unlock_lore_entry(&self, id: &str) -> Result<bool> {
        // el WHERE unlocked = 0 hace esto atómico — elimina la carrera SELECT→UPDATE y retorna 0 si ya estaba desbloqueado
        let changed = self.conn.execute(
            "UPDATE lore_library SET unlocked = 1, unlocked_at = ?2 WHERE id = ?1 AND unlocked = 0",
            params![id, Utc::now().to_rfc3339()],
        )?;
        if changed > 0 {
            let _ = self.log_change("lore_unlock", id, "unlock");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn unlock_lore_by_title(&self, title: &str) -> Result<bool> {
        let row_opt: Option<(String, i32)> = self
            .conn
            .query_row(
                "SELECT id, unlocked FROM lore_library WHERE title = ?1 LIMIT 1",
                params![title],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        if let Some((id, unlocked)) = row_opt {
            if unlocked == 0 {
                self.conn.execute(
                    "UPDATE lore_library SET unlocked = 1, unlocked_at = ?2 WHERE id = ?1",
                    params![id, Utc::now().to_rfc3339()],
                )?;
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    }

    pub fn insert_custom_lore_entry(
        &self,
        id: &str,
        category: &str,
        title: &str,
        content: &str,
        unlocked: bool,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO lore_library (id, category, title, content, unlocked, unlocked_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                category,
                title,
                content,
                if unlocked { 1 } else { 0 },
                if unlocked { Some(Utc::now().to_rfc3339()) } else { None }
            ],
        )?;
        Ok(())
    }

    pub fn discover_memory_fragment(&self, trigger: &str, chance_multiplier: f64) -> Result<Option<(String, String)>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let base_common: f64 = match trigger {
            "task" => 0.05,
            "high_priority_task" => 0.10,
            "project_complete" => 0.25,
            "level_up" => 0.15,
            "daily_adventure" => 0.10,
            "zen_water" => 0.02,
            _ => 0.05,
        };
        let common_chance = (base_common * chance_multiplier).min(1.0);
        let rare_chance = (0.01 * chance_multiplier).min(1.0);
        let legendary_chance = (0.001 * chance_multiplier).min(1.0);

        // se evalúa de mayor a menor rareza para que legendary no quede eclipsado por common al mismo tiempo
        let rarity = if rng.gen_bool(legendary_chance) {
            "legendary"
        } else if rng.gen_bool(rare_chance) {
            "rare"
        } else if rng.gen_bool(common_chance) {
            "common"
        } else {
            return Ok(None);
        };

        let candidates: &[&str] = match rarity {
            "legendary" => &["memory_999"],
            "rare" => &["memory_077", "memory_112", "memory_144", "memory_188"],
            _ => &[
                "memory_001", "memory_002", "memory_003", "memory_004", "memory_005",
                "memory_006", "memory_007", "memory_008", "memory_009", "memory_010",
            ],
        };

        let mut undiscovered: Vec<(String, String)> = Vec::new();
        for &id in candidates {
            let row: Option<(String, i32)> = self
                .conn
                .query_row(
                    "SELECT title, unlocked FROM lore_library WHERE id = ?1",
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()?;
            if let Some((title, unlocked)) = row {
                if unlocked == 0 {
                    undiscovered.push((id.to_string(), title));
                }
            }
        }

        if undiscovered.is_empty() {
            return Ok(None);
        }

        let idx = rng.gen_range(0..undiscovered.len());
        let (id, title) = undiscovered.swap_remove(idx);
        self.conn.execute(
            "UPDATE lore_library SET unlocked = 1, unlocked_at = ?2 WHERE id = ?1",
            params![id, Utc::now().to_rfc3339()],
        )?;
        Ok(Some((id, title)))
    }

    pub fn verify_data_integrity(&self) -> Result<Vec<String>> {
        let mut reports = Vec::new();

        // las tareas huérfanas se desconectan (project_id = NULL) en vez de borrarse — el usuario no pierde el trabajo
        let orphaned_tasks: i32 = self.conn.query_row(
            "SELECT count(*) FROM tasks WHERE project_id IS NOT NULL AND project_id NOT IN (SELECT id FROM projects)",
            [],
            |row| row.get(0)
        )?;
        if orphaned_tasks > 0 {
            self.conn.execute(
                "UPDATE tasks SET project_id = NULL WHERE project_id IS NOT NULL AND project_id NOT IN (SELECT id FROM projects)",
                []
            )?;
            reports.push(format!(
                "Repaired {} orphaned task project references.",
                orphaned_tasks
            ));
        }

        let orphaned_notes: i32 = self.conn.query_row(
            "SELECT count(*) FROM notes WHERE project_id IS NOT NULL AND project_id NOT IN (SELECT id FROM projects)",
            [],
            |row| row.get(0)
        )?;
        if orphaned_notes > 0 {
            self.conn.execute(
                "UPDATE notes SET project_id = NULL WHERE project_id IS NOT NULL AND project_id NOT IN (SELECT id FROM projects)",
                []
            )?;
            reports.push(format!(
                "Repaired {} orphaned note project references.",
                orphaned_notes
            ));
        }

        // los milestones huérfanos sí se borran (a diferencia de tareas/notas) — sin proyecto no tienen contexto y el schema lo requiere
        let orphaned_milestones: i32 = self.conn.query_row(
            "SELECT count(*) FROM milestones WHERE project_id NOT IN (SELECT id FROM projects)",
            [],
            |row| row.get(0),
        )?;
        if orphaned_milestones > 0 {
            self.conn.execute(
                "DELETE FROM milestones WHERE project_id NOT IN (SELECT id FROM projects)",
                [],
            )?;
            reports.push(format!(
                "Cleaned up {} orphaned milestones with missing projects.",
                orphaned_milestones
            ));
        }

        let count_streaks: i32 =
            self.conn
                .query_row("SELECT count(*) FROM streaks", [], |row| row.get(0))?;
        if count_streaks == 0 {
            self.conn.execute(
                "INSERT INTO streaks (id, current_streak, best_streak, last_active_day) VALUES ('streak_id', 0, 0, NULL)",
                [],
            )?;
            reports.push("Initialized missing streak records.".to_string());
        }

        let count_tree: i32 = self
            .conn
            .query_row("SELECT count(*) FROM zen_tree", [], |row| row.get(0))?;
        if count_tree == 0 {
            let tree_id = Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO zen_tree (id, growth, health, stage, last_watered, water_today) VALUES (?1, 0, 100, 1, NULL, 0)",
                params![tree_id],
            )?;
            reports.push("Initialized missing Zen Tree records.".to_string());
        }

        Ok(reports)
    }

    pub fn verify_db_backup(backup_path: &Path) -> Result<bool> {
        let conn = Connection::open(backup_path)?;
        let integrity: String = conn.query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;

        let mut stmt = conn.prepare("PRAGMA foreign_key_check;")?;
        let foreign_keys: Vec<String> = stmt
            .query_map([], |row| {
                let table: String = row.get(0)?;
                let rowid: i64 = row.get(1)?;
                let parent: String = row.get(2)?;
                Ok(format!(
                    "Table {} (rowid {}) has invalid reference to {}",
                    table, rowid, parent
                ))
            })?
            .filter_map(Result::ok)
            .collect();

        Ok(integrity == "ok" && foreign_keys.is_empty())
    }

    pub fn get_all_known_usernames(&self) -> Result<Vec<String>> {
        let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();

        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT user_username FROM project_members WHERE user_username IS NOT NULL AND user_username != ''",
        )?;
        for row in stmt.query_map([], |r| r.get::<_, String>(0))?.filter_map(Result::ok) {
            names.insert(row.to_lowercase());
        }

        let mut stmt2 = self.conn.prepare(
            "SELECT DISTINCT hero_name FROM global_chronicle WHERE hero_name IS NOT NULL AND hero_name != ''",
        )?;
        for row in stmt2.query_map([], |r| r.get::<_, String>(0))?.filter_map(Result::ok) {
            names.insert(row.to_lowercase());
        }

        let mut stmt3 = self.conn.prepare(
            "SELECT DISTINCT username FROM users WHERE username IS NOT NULL AND username != ''",
        )?;
        for row in stmt3.query_map([], |r| r.get::<_, String>(0))?.filter_map(Result::ok) {
            names.insert(row.to_lowercase());
        }

        Ok(names.into_iter().collect())
    }

    pub fn upsert_global_chronicle_entry(
        &self,
        id: &str,
        hero_name: &str,
        event_type: &str,
        description: &str,
        timestamp: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO global_chronicle (id, hero_name, event_type, description, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, hero_name, event_type, description, timestamp],
        )?;
        Ok(())
    }

    pub fn upsert_chronicle_entry(&self, e: &GlobalChronicleEntry) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO global_chronicle (id, hero_name, event_type, description, timestamp, hero_class) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![e.id, e.hero_name, e.event_type, e.description, e.timestamp, e.hero_class],
        )?;
        // si la fila llegó primero desde otro dispositivo sin hero_class, la completamos aquí — no sobreescribimos datos ya presentes
        if e.hero_class.is_some() {
            self.conn.execute(
                "UPDATE global_chronicle SET hero_class = ?1 WHERE id = ?2 AND hero_class IS NULL",
                rusqlite::params![e.hero_class, e.id],
            )?;
        }
        Ok(())
    }

    pub fn get_global_chronicle_entries(&self) -> Result<Vec<GlobalChronicleEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, hero_name, event_type, description, timestamp, hero_class FROM global_chronicle ORDER BY timestamp DESC LIMIT 500",
        )?;
        let entries = stmt
            .query_map([], |row| {
                Ok(GlobalChronicleEntry {
                    id: row.get(0)?,
                    hero_name: row.get(1)?,
                    event_type: row.get(2)?,
                    description: row.get(3)?,
                    timestamp: row.get(4)?,
                    hero_class: row.get(5)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(entries)
    }

    // el owner_identity filtra las tareas para que los pulls de proyectos compartidos no inflen el delta del capítulo
    pub fn get_contribution_snapshot(&self, owner_identity: &str) -> Result<std::collections::HashMap<String, u64>> {
        let mut map = std::collections::HashMap::new();

        let tasks: u64 = self.conn.query_row(
            // sólo tareas raíz propias — las subtareas y las de otros colaboradores no cuentan para el capítulo
            "SELECT COUNT(*) FROM tasks WHERE completed = 1 AND parent_task_id IS NULL
             AND (owner_identity IS NULL OR owner_identity = ?1)",
            params![owner_identity],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as u64;
        map.insert("tasks_completed".to_string(), tasks);

        let subtasks: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tasks WHERE completed = 1 AND parent_task_id IS NOT NULL
             AND (owner_identity IS NULL OR owner_identity = ?1)",
            params![owner_identity],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as u64;
        map.insert("subtasks_completed".to_string(), subtasks);

        let focus: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM focus_sessions WHERE owner_identity = ?1 OR owner_identity IS NULL",
            params![owner_identity],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as u64;
        map.insert("focus_sessions".to_string(), focus);

        let waterings: u64 = self.conn.query_row(
            "SELECT COALESCE(total_waterings, 0) FROM zen_tree LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as u64;
        map.insert("tree_waterings".to_string(), waterings);

        let rituals: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM ritual_history",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as u64;
        map.insert("rituals_completed".to_string(), rituals);

        let reflections: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM reflections",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as u64;
        map.insert("reflections_written".to_string(), reflections);

        let scrolls: u64 = self.conn.query_row(
            "SELECT COUNT(*) FROM notes",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) as u64;
        map.insert("scrolls_created".to_string(), scrolls);

        Ok(map)
    }

    // sin este log estaríamos mandando el total acumulado en cada sync en vez del delta — doble-conteo en el servidor
    pub fn get_last_sent_contributions(&self, chapter_id: &str) -> Result<std::collections::HashMap<String, u64>> {
        let mut stmt = self.conn.prepare(
            "SELECT objective_type, last_sent_total FROM chapter_contribution_log WHERE chapter_id = ?1",
        )?;
        let entries = stmt.query_map(params![chapter_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
        })?
        .filter_map(Result::ok)
        .collect::<std::collections::HashMap<_, _>>();
        Ok(entries)
    }

    pub fn save_sent_contributions(&self, chapter_id: &str, totals: &std::collections::HashMap<String, u64>) -> Result<()> {
        for (obj_type, &total) in totals {
            // INSERT OR REPLACE en vez de ON CONFLICT DO UPDATE — este último requiere SQLite ≥ 3.24
            // y en algunas builds de rusqlite falla silenciosamente, lo que causaría que el delta
            // se re-mandara en cada sync e inflara los contadores del capítulo
            self.conn.execute(
                "INSERT OR REPLACE INTO chapter_contribution_log (chapter_id, objective_type, last_sent_total)
                 VALUES (?1, ?2, ?3)",
                params![chapter_id, obj_type, total as i64],
            )?;
        }
        Ok(())
    }

}
