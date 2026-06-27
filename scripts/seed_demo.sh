#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# seed_demo.sh — llena la base de datos con datos de demo para probar el app
# ─────────────────────────────────────────────────────────────────────────────
# Creates a demo SQLite database pre-loaded with a fictional user, projects, tasks,
# journal entries, focus sessions, achievements, and more — for taking screenshots.
#
# Uso:
#   ./scripts/seed_demo.sh
#   ./scripts/seed_demo.sh --path /custom/path/questline.db
#
# WARNING: This will overwrite any existing questline.db at the target path.

set -euo pipefail

# ── Parse args ───────────────────────────────────────────────────────────────
CUSTOM_PATH=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --path) CUSTOM_PATH="$2"; shift 2 ;;
        *) echo "Unknown argument: $1"; exit 1 ;;
    esac
done

# ── Resolve DB path ──────────────────────────────────────────────────────────
if [[ -n "$CUSTOM_PATH" ]]; then
    DB_PATH="$CUSTOM_PATH"
else
    OS="$(uname -s)"
    case "$OS" in
        Darwin)
            DB_PATH="$HOME/Library/Application Support/questline/questline.db"
            ;;
        Linux)
            DB_PATH="${XDG_CONFIG_HOME:-$HOME/.config}/questline/questline.db"
            ;;
        *)
            echo "Unsupported OS: $OS  Use --path to specify the DB path manually."
            exit 1
            ;;
    esac
fi

# ── Preflight ────────────────────────────────────────────────────────────────
if ! command -v sqlite3 &>/dev/null; then
    echo "Error: sqlite3 is not installed. Install it and re-run."
    exit 1
fi

mkdir -p "$(dirname "$DB_PATH")"

if [[ -f "$DB_PATH" ]]; then
    echo "⚠  Existing database found at:"
    echo "   $DB_PATH"
    echo ""
    read -rp "   Overwrite it with demo data? [y/N] " confirm
    [[ "${confirm,,}" == "y" ]] || { echo "Aborted."; exit 0; }
    rm -f "$DB_PATH"
fi

echo ""
echo "Creating demo database at:"
echo "  $DB_PATH"
echo ""

# ── IDs (pre-generated, stable) ──────────────────────────────────────────────
USER_ID="a1b2c3d4-0000-4000-8000-000000000001"

PRJ1="b0000001-0000-4000-8000-000000000001"  # Operation: Inbox Zero
PRJ2="b0000002-0000-4000-8000-000000000002"  # The Grand Refactor
PRJ3="b0000003-0000-4000-8000-000000000003"  # Learn Rust (For Real This Time)
PRJ4="b0000004-0000-4000-8000-000000000004"  # World Domination v3.1
PRJ5="b0000005-0000-4000-8000-000000000005"  # coffee.js  [completed]

T01="c0000001-0000-4000-8000-000000000001"
T02="c0000002-0000-4000-8000-000000000002"
T03="c0000003-0000-4000-8000-000000000003"
T04="c0000004-0000-4000-8000-000000000004"
T05="c0000005-0000-4000-8000-000000000005"
T06="c0000006-0000-4000-8000-000000000006"
T07="c0000007-0000-4000-8000-000000000007"
T08="c0000008-0000-4000-8000-000000000008"
T09="c0000009-0000-4000-8000-000000000009"
T10="c000000a-0000-4000-8000-000000000010"
T11="c000000b-0000-4000-8000-000000000011"
T12="c000000c-0000-4000-8000-000000000012"
T13="c000000d-0000-4000-8000-000000000013"
T14="c000000e-0000-4000-8000-000000000014"
T15="c000000f-0000-4000-8000-000000000015"
T16="c0000010-0000-4000-8000-000000000016"
T17="c0000011-0000-4000-8000-000000000017"
T18="c0000012-0000-4000-8000-000000000018"
T19="c0000013-0000-4000-8000-000000000019"
T20="c0000014-0000-4000-8000-000000000020"

NOTE1="d0000001-0000-4000-8000-000000000001"
NOTE2="d0000002-0000-4000-8000-000000000002"
NOTE3="d0000003-0000-4000-8000-000000000003"
NOTE4="d0000004-0000-4000-8000-000000000004"
NOTE5="d0000005-0000-4000-8000-000000000005"

JOURN1="e0000001-0000-4000-8000-000000000001"
JOURN2="e0000002-0000-4000-8000-000000000002"
JOURN3="e0000003-0000-4000-8000-000000000003"
JOURN4="e0000004-0000-4000-8000-000000000004"
JOURN5="e0000005-0000-4000-8000-000000000005"

FOCUS1="f0000001-0000-4000-8000-000000000001"
FOCUS2="f0000002-0000-4000-8000-000000000002"
FOCUS3="f0000003-0000-4000-8000-000000000003"
FOCUS4="f0000004-0000-4000-8000-000000000004"
FOCUS5="f0000005-0000-4000-8000-000000000005"
FOCUS6="f0000006-0000-4000-8000-000000000006"

RITUAL1="9a000001-0000-4000-8000-000000000001"
RITUAL2="9a000002-0000-4000-8000-000000000002"
RITUAL3="9a000003-0000-4000-8000-000000000003"

MILE1="9b000001-0000-4000-8000-000000000001"
MILE2="9b000002-0000-4000-8000-000000000002"
MILE3="9b000003-0000-4000-8000-000000000003"
MILE4="9b000004-0000-4000-8000-000000000004"
MILE5="9b000005-0000-4000-8000-000000000005"

ZEN_ID="aaaaaaaa-0000-4000-8000-000000000001"
STREAK_ID="bbbbbbbb-0000-4000-8000-000000000001"

DQUEST1="da000001-0000-4000-8000-000000000001"
DQUEST2="da000002-0000-4000-8000-000000000002"
DQUEST3="da000003-0000-4000-8000-000000000003"

CHRON1="gc000001-0000-4000-8000-000000000001"
CHRON2="gc000002-0000-4000-8000-000000000002"
CHRON3="gc000003-0000-4000-8000-000000000003"
CHRON4="gc000004-0000-4000-8000-000000000004"

CODEX1="9c000001-0000-4000-8000-000000000001"
CODEX2="9c000002-0000-4000-8000-000000000002"

# Subtasks
ST01="9d000001-0000-4000-8000-000000000001"
ST02="9d000002-0000-4000-8000-000000000002"
ST03="9d000003-0000-4000-8000-000000000003"
ST04="9d000004-0000-4000-8000-000000000004"
ST05="9d000005-0000-4000-8000-000000000005"

# Realm Activity Feed (global_chronicle — other heroes across the realm)
GC01="9e000001-0000-4000-8000-000000000001"
GC02="9e000002-0000-4000-8000-000000000002"
GC03="9e000003-0000-4000-8000-000000000003"
GC04="9e000004-0000-4000-8000-000000000004"
GC05="9e000005-0000-4000-8000-000000000005"
GC06="9e000006-0000-4000-8000-000000000006"
GC07="9e000007-0000-4000-8000-000000000007"
GC08="9e000008-0000-4000-8000-000000000008"
GC09="9e000009-0000-4000-8000-000000000009"
GC10="9e000010-0000-4000-8000-000000000010"
GC11="9e000011-0000-4000-8000-000000000011"
GC12="9e000012-0000-4000-8000-000000000012"
GC13="9e000013-0000-4000-8000-000000000013"
GC14="9e000014-0000-4000-8000-000000000014"
GC15="9e000015-0000-4000-8000-000000000015"
GC16="9e000016-0000-4000-8000-000000000016"
GC17="9e000017-0000-4000-8000-000000000017"
GC18="9e000018-0000-4000-8000-000000000018"
GC19="9e000019-0000-4000-8000-000000000019"
GC20="9e000020-0000-4000-8000-000000000020"

# ── Build and execute SQL ─────────────────────────────────────────────────────
sqlite3 "$DB_PATH" <<'ENDSQL'

-- ── Schema ──────────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY, username TEXT NOT NULL, class TEXT NOT NULL,
    level INTEGER NOT NULL DEFAULT 1, xp INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT,
    created_at TEXT NOT NULL, archived INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0, owner_identity TEXT,
    owner_username TEXT, is_shared INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY, project_id TEXT, title TEXT NOT NULL, description TEXT,
    due_date TEXT, completed INTEGER NOT NULL DEFAULT 0,
    priority TEXT NOT NULL DEFAULT 'Medium', created_at TEXT NOT NULL,
    owner_identity TEXT, owner_username TEXT, parent_task_id TEXT,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL,
    FOREIGN KEY(parent_task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS codices (
    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS notes (
    id TEXT PRIMARY KEY, project_id TEXT, title TEXT NOT NULL,
    markdown_content TEXT NOT NULL, created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    sharing_permission TEXT NOT NULL DEFAULT 'collaborative',
    codex_id TEXT,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL,
    FOREIGN KEY(codex_id) REFERENCES codices(id) ON DELETE SET NULL
);
CREATE TABLE IF NOT EXISTS daily_quests (
    id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT,
    completed INTEGER NOT NULL DEFAULT 0, due_date TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS xp_events (
    id TEXT PRIMARY KEY, event_type TEXT NOT NULL, xp_gained INTEGER NOT NULL,
    timestamp TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS journal_entries (
    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, entry_date TEXT NOT NULL,
    content TEXT NOT NULL, created_at TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'Private',
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS zen_tree (
    id TEXT PRIMARY KEY, growth INTEGER NOT NULL DEFAULT 0,
    health INTEGER NOT NULL DEFAULT 100, stage INTEGER NOT NULL DEFAULT 1,
    last_watered TEXT, water_today INTEGER NOT NULL DEFAULT 0,
    total_waterings INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS daily_adventures (
    id TEXT PRIMARY KEY, title TEXT NOT NULL, quest_type TEXT NOT NULL,
    target_count INTEGER NOT NULL, current_count INTEGER NOT NULL DEFAULT 0,
    completed INTEGER NOT NULL DEFAULT 0, created_date TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS streaks (
    id TEXT PRIMARY KEY, current_streak INTEGER NOT NULL DEFAULT 0,
    best_streak INTEGER NOT NULL DEFAULT 0, last_active_day TEXT
);
CREATE TABLE IF NOT EXISTS achievements (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL,
    unlocked_at TEXT
);
CREATE TABLE IF NOT EXISTS focus_sessions (
    id TEXT PRIMARY KEY, project_id TEXT, task_id TEXT,
    duration_mins INTEGER NOT NULL, xp_gained INTEGER NOT NULL,
    completed_at TEXT NOT NULL, soundscape TEXT NOT NULL DEFAULT 'Silent',
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL,
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE SET NULL
);
CREATE TABLE IF NOT EXISTS rituals (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT,
    frequency TEXT NOT NULL, reward_xp INTEGER NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS ritual_history (
    ritual_id TEXT NOT NULL, completed_date TEXT NOT NULL,
    PRIMARY KEY(ritual_id, completed_date),
    FOREIGN KEY(ritual_id) REFERENCES rituals(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS traits (id TEXT PRIMARY KEY, unlocked_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS milestones (
    id TEXT PRIMARY KEY, project_id TEXT NOT NULL, name TEXT NOT NULL,
    description TEXT, completed INTEGER NOT NULL DEFAULT 0,
    xp_reward INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS reflections (
    created_date TEXT PRIMARY KEY, what_went_well TEXT NOT NULL,
    what_can_improve TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sync_log (
    id TEXT PRIMARY KEY, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL,
    operation TEXT NOT NULL, timestamp TEXT NOT NULL, synced INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS revisions (
    id TEXT PRIMARY KEY, entity_type TEXT NOT NULL, entity_id TEXT NOT NULL,
    revision_number INTEGER NOT NULL, content TEXT NOT NULL, timestamp TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS devices (
    device_id TEXT PRIMARY KEY, device_name TEXT NOT NULL,
    created_at TEXT NOT NULL, last_sync TEXT
);
CREATE TABLE IF NOT EXISTS project_members (
    project_id TEXT, user_identity TEXT, user_username TEXT NOT NULL,
    role TEXT NOT NULL, PRIMARY KEY (project_id, user_identity),
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS invitations (
    id TEXT PRIMARY KEY, project_id TEXT, project_name TEXT NOT NULL,
    inviter_identity TEXT NOT NULL, inviter_username TEXT NOT NULL,
    invitee_identity TEXT NOT NULL, role TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'Pending', created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS chronicle_messages (
    id TEXT PRIMARY KEY, project_id TEXT, sender_identity TEXT NOT NULL,
    sender_username TEXT NOT NULL, content TEXT NOT NULL,
    message_type TEXT NOT NULL, timestamp TEXT NOT NULL,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS message_reactions (
    message_id TEXT, user_identity TEXT, emoji TEXT,
    PRIMARY KEY (message_id, user_identity, emoji),
    FOREIGN KEY(message_id) REFERENCES chronicle_messages(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS activity_log (
    id TEXT PRIMARY KEY, project_id TEXT, event_type TEXT NOT NULL,
    description TEXT NOT NULL, user_identity TEXT NOT NULL,
    user_username TEXT NOT NULL, timestamp TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY, notification_type TEXT NOT NULL, title TEXT NOT NULL,
    content TEXT NOT NULL, target_id TEXT, read INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS task_assignments (
    task_id TEXT, user_identity TEXT, user_username TEXT NOT NULL,
    PRIMARY KEY (task_id, user_identity),
    FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS presence (
    user_identity TEXT PRIMARY KEY, username TEXT NOT NULL,
    online INTEGER NOT NULL DEFAULT 0, last_seen TEXT NOT NULL,
    current_project TEXT, privacy_status TEXT NOT NULL DEFAULT 'Visible'
);
CREATE TABLE IF NOT EXISTS great_chronicle (
    id TEXT PRIMARY KEY, day_number INTEGER NOT NULL,
    entry_text TEXT NOT NULL, timestamp TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS global_chronicle (
    id TEXT PRIMARY KEY, hero_name TEXT NOT NULL,
    event_type TEXT NOT NULL, description TEXT NOT NULL,
    timestamp TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS chapter_contribution_log (
    chapter_id TEXT NOT NULL, objective_type TEXT NOT NULL,
    last_sent_total INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (chapter_id, objective_type)
);
CREATE TABLE IF NOT EXISTS class_quests (
    class_name TEXT NOT NULL, unlock_level INTEGER NOT NULL,
    quest_name TEXT NOT NULL, description TEXT NOT NULL, status TEXT NOT NULL,
    progress INTEGER NOT NULL DEFAULT 0, target INTEGER NOT NULL DEFAULT 1,
    lore_reward TEXT NOT NULL, PRIMARY KEY(class_name, unlock_level)
);
CREATE TABLE IF NOT EXISTS legendary_titles (
    title_id TEXT PRIMARY KEY, title_name TEXT NOT NULL,
    description TEXT NOT NULL, unlocked INTEGER NOT NULL DEFAULT 0,
    equipped INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS relics (
    id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL,
    unlocked INTEGER NOT NULL DEFAULT 0, unlocked_at TEXT
);
CREATE TABLE IF NOT EXISTS companion_lore (
    id TEXT PRIMARY KEY, story_text TEXT NOT NULL, timestamp TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS lore_library (
    id TEXT PRIMARY KEY, category TEXT NOT NULL, title TEXT NOT NULL,
    content TEXT NOT NULL, unlocked INTEGER NOT NULL DEFAULT 0, unlocked_at TEXT
);

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

ENDSQL

# Run the data inserts via here-doc with shell variable expansion
sqlite3 "$DB_PATH" <<ENDSQL

-- ── User ─────────────────────────────────────────────────────────────────────
-- Level 15 Code Warlock. XP needed for next level = 200 + (15*15*20) = 4700
-- We set xp=2800 so the bar is visibly more than half.
INSERT INTO users (id, username, class, level, xp, created_at)
VALUES (
    '$USER_ID', 'MerlinBytes', 'Code Warlock', 15, 2800,
    '2026-01-15T09:00:00+00:00'
);

ALTER TABLE users ADD COLUMN specialization TEXT;
UPDATE users SET specialization = 'Automation Mage' WHERE id = '$USER_ID';

-- ── Settings ─────────────────────────────────────────────────────────────────
INSERT INTO settings VALUES ('sync_count', '7');
INSERT INTO settings VALUES ('backup_count', '3');
INSERT INTO settings VALUES ('conflict_count', '0');
INSERT INTO settings VALUES ('last_restore', 'Never');
INSERT INTO settings VALUES ('specialization', 'Automation Mage');

-- ── Projects ─────────────────────────────────────────────────────────────────
INSERT INTO projects (id, name, description, created_at, archived, completed)
VALUES
    (
        '$PRJ1', 'Operation: Inbox Zero',
        'Declare war on 4,742 unread emails. I have a plan. Several plans, actually. One involves a ritual.',
        '2026-02-01T08:30:00+00:00', 0, 0
    ),
    (
        '$PRJ2', 'The Grand Refactor',
        'Everything is technical debt and nothing matters. This time we fix it for real.',
        '2026-02-20T14:00:00+00:00', 0, 0
    ),
    (
        '$PRJ3', 'Learn Rust (For Real This Time)',
        'Attempt #4. The borrow checker and I have reached a tentative ceasefire.',
        '2026-03-10T09:15:00+00:00', 0, 0
    ),
    (
        '$PRJ4', 'World Domination v3.1',
        'v1.0 had scalability issues. v2.0 was missing a database index. v3.1 is the one.',
        '2026-05-05T11:00:00+00:00', 0, 0
    ),
    (
        '$PRJ5', 'coffee.js',
        'A JavaScript library that makes a single cup of coffee worth 40 XP. Shipped.',
        '2026-01-18T10:00:00+00:00', 0, 1
    );

-- ── Tasks ────────────────────────────────────────────────────────────────────
-- Operation: Inbox Zero
INSERT INTO tasks (id, project_id, title, description, due_date, completed, priority, created_at)
VALUES
    ('$T01', '$PRJ1', 'Unsubscribe from at least 50 mailing lists', 'Minimum viable inbox. Focus on the ones that still send fax confirmations.', '2026-06-25T00:00:00+00:00', 1, 'High', '2026-02-01T09:00:00+00:00'),
    ('$T02', '$PRJ1', 'Archive everything before 2025', 'Just archive it. Do not read it. Do not make eye contact with it.', '2026-06-28T00:00:00+00:00', 1, 'Medium', '2026-02-01T09:05:00+00:00'),
    ('$T03', '$PRJ1', 'Set up filters for newsletters', 'Label: "Things I subscribed to drunk". Auto-archive: yes.', '2026-06-30T00:00:00+00:00', 0, 'Medium', '2026-02-03T10:00:00+00:00'),
    ('$T04', '$PRJ1', 'Respond to Kevin''s email from March', 'Kevin has followed up 6 times. Kevin is very persistent. Kevin deserves a response.', '2026-06-24T00:00:00+00:00', 0, 'High', '2026-02-05T11:00:00+00:00'),
    ('$T05', '$PRJ1', 'Delete the folder called "Miscellaneous (2)"', 'There are seven of these. Start with the oldest. Do not open any attachments.', NULL, 0, 'Low', '2026-02-10T08:30:00+00:00');

-- The Grand Refactor
INSERT INTO tasks (id, project_id, title, description, due_date, completed, priority, created_at)
VALUES
    ('$T06', '$PRJ2', 'Rename all variables from single letters', 'x, y, z, tmp, tmp2, tmp3, tmpFinal, tmpFinalActual — these need names.', '2026-07-01T00:00:00+00:00', 1, 'High', '2026-02-20T14:30:00+00:00'),
    ('$T07', '$PRJ2', 'Remove all commented-out code blocks', 'They are not documentation. They are archaeology. Remove them.', '2026-07-05T00:00:00+00:00', 1, 'Medium', '2026-02-21T09:00:00+00:00'),
    ('$T08', '$PRJ2', 'Split the 2,400-line utils.js file', 'A function called doEverything() is not an architecture.', '2026-07-10T00:00:00+00:00', 0, 'High', '2026-02-22T11:00:00+00:00'),
    ('$T09', '$PRJ2', 'Add tests to the untested 94% of the codebase', 'Baby steps. We start with the login button.', NULL, 0, 'High', '2026-02-25T15:00:00+00:00'),
    ('$T10', '$PRJ2', 'Document the mystery function called "fix2"', 'It was called "fix" but the original fix did not work. fix2 works. No one knows why.', NULL, 0, 'Medium', '2026-03-01T10:00:00+00:00');

-- Learn Rust
INSERT INTO tasks (id, project_id, title, description, due_date, completed, priority, created_at)
VALUES
    ('$T11', '$PRJ3', 'Understand the borrow checker without crying', 'A meditative exercise. The borrow checker is not your enemy. It is your disappointed parent.', '2026-07-01T00:00:00+00:00', 1, 'High', '2026-03-10T09:30:00+00:00'),
    ('$T12', '$PRJ3', 'Write a CLI tool that actually does something', 'Not hello world. Something real. Something with --help flags.', '2026-07-15T00:00:00+00:00', 1, 'Medium', '2026-03-12T10:00:00+00:00'),
    ('$T13', '$PRJ3', 'Read the async chapter without giving up', 'Attempt 3. This time with snacks.', '2026-07-20T00:00:00+00:00', 0, 'Medium', '2026-03-15T14:00:00+00:00'),
    ('$T14', '$PRJ3', 'Understand lifetimes well enough to explain them', 'Target audience: anyone who asks. Time limit: under 10 minutes.', NULL, 0, 'Low', '2026-03-20T09:00:00+00:00');

-- World Domination
INSERT INTO tasks (id, project_id, title, description, due_date, completed, priority, created_at)
VALUES
    ('$T15', '$PRJ4', 'Write the manifesto (draft 1)', 'Keep it under 5 pages. Use bullet points. Make it compelling.', '2026-06-30T00:00:00+00:00', 1, 'High', '2026-05-05T11:30:00+00:00'),
    ('$T16', '$PRJ4', 'Set up the secret Discord server', '12 members. All very serious. Pinned message: "No memes on Tuesdays."', '2026-07-05T00:00:00+00:00', 0, 'Medium', '2026-05-07T10:00:00+00:00'),
    ('$T17', '$PRJ4', 'Design the org chart', 'Current structure: 1 person. Target structure: also probably 1 person but with a fancier title.', NULL, 0, 'Low', '2026-05-10T15:00:00+00:00');

-- coffee.js (completed project, tasks also done)
INSERT INTO tasks (id, project_id, title, description, due_date, completed, priority, created_at)
VALUES
    ('$T18', '$PRJ5', 'Write the core brew() function', 'Takes options: strength, milk, regrets. Returns a Promise<Coffee>.', '2026-01-20T00:00:00+00:00', 1, 'High', '2026-01-18T10:30:00+00:00'),
    ('$T19', '$PRJ5', 'Publish to npm', 'npm publish --access public. And then immediately worry about it.', '2026-01-22T00:00:00+00:00', 1, 'Medium', '2026-01-19T14:00:00+00:00'),
    ('$T20', '$PRJ5', 'Write the README', 'Section 1: What is coffee.js? Section 2: Why? Section 3: FAQ (one question, no answers).', '2026-01-25T00:00:00+00:00', 1, 'Low', '2026-01-21T11:00:00+00:00');

-- ── Notes (Scrolls) ──────────────────────────────────────────────────────────
INSERT INTO notes (id, project_id, title, markdown_content, created_at, updated_at)
VALUES
    (
        '$NOTE1', '$PRJ2',
        'Architecture Decisions I Will Regret Later',
        '# Architecture Decisions I Will Regret Later

## Decision #1: Microservices
**Date:** February 2026
**Rationale:** It was in a blog post. The blog post had 4,000 likes.
**Current status:** We have 47 services. Three of them might be doing the same thing.

## Decision #2: NoSQL for Everything
**Rationale:** Joins are scary.
**Current status:** We now do joins manually in JavaScript. This is worse.

## Decision #3: The Monorepo
**Rationale:** One repo to rule them all.
**Current status:** git clone takes 14 minutes. Nobody clones twice.

## Lessons Learned
- Blog posts with likes are not architecture reviews.
- Joins were not the enemy. I was the enemy.
- A monorepo is just a big repo with delusions of grandeur.',
        '2026-03-01T10:00:00+00:00', '2026-03-15T16:00:00+00:00'
    ),
    (
        '$NOTE2', '$PRJ3',
        'The Borrow Checker: A Love Story',
        '# The Borrow Checker: A Love Story

## Chapter 1: Denial
I do not need the borrow checker. I am a senior engineer.
I have written JavaScript for 7 years.
I own two books about design patterns.

## Chapter 2: Anger
ERROR[E0502]: cannot borrow "data" as mutable because it is also borrowed as immutable.
I KNOW WHAT I AM DOING.
(I did not know what I was doing.)

## Chapter 3: Bargaining
What if I just clone() everything?
What if I use Rc<RefCell<Box<Arc<Mutex<T>>>>>?
What if I write this in Python instead?

## Chapter 4: Acceptance
The borrow checker was right.
The borrow checker was always right.
My code has not segfaulted once.
I owe the borrow checker an apology.',
        '2026-04-05T14:00:00+00:00', '2026-04-20T11:00:00+00:00'
    ),
    (
        '$NOTE3', '$PRJ1',
        'Email Triage System v2 (The One That Will Actually Work)',
        '# Email Triage System v2

## The Four Folders of Destiny
1. **URGENT** — things that require action within 24 hours
2. **WAITING** — things I have responded to and am now ignoring
3. **SOMEDAY** — things I intend to address in a parallel universe
4. **DELETE** — the folder that fixed 80% of the problem

## The Five-Email Rule
If a thread exceeds five emails, it should be a meeting.
If a meeting would make it worse, it should be a document.
If nobody reads the document, it was never important.

## Kevin
Kevin gets his own folder.
Kevin is persistent.
Kevin will be answered on Fridays.',
        '2026-02-15T09:00:00+00:00', '2026-06-01T10:00:00+00:00'
    ),
    (
        '$NOTE4', '$PRJ4',
        'Phase 1 Strategic Overview',
        '# World Domination v3.1 — Phase 1

## Core Thesis
The previous versions failed due to insufficient documentation and a lack of a proper CI/CD pipeline.
v3.1 addresses both.

## Key Milestones
- [ ] Establish presence in at least 3 timezones
- [ ] Achieve 100 newsletter subscribers (organic, not purchased)
- [x] Write the manifesto (first draft)
- [ ] Get a domain name that is not already taken

## Risk Register
| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Someone else does it first | Medium | Be faster |
| Running out of coffee | High | Never run out of coffee |
| Scope creep | Certain | Embrace it |

## Notes
This plan is confidential. Do not share in a public GitHub repo.',
        '2026-05-06T12:00:00+00:00', '2026-06-10T15:00:00+00:00'
    ),
    (
        '$NOTE5', '$PRJ3',
        'Things Rust Has Taught Me About Life',
        '# Things Rust Has Taught Me About Life

1. **Ownership matters.** You cannot give something you do not own.
2. **Borrowing has rules.** You can look. You can touch. But you cannot change without permission.
3. **Lifetimes are real.** References must not outlive the things they reference. This applies to jobs too.
4. **Panicking is not a strategy.** Use Result. Handle your errors gracefully.
5. **The compiler is smarter than you.** Not always. But often enough that you should listen.
6. **Zero-cost abstractions exist.** Elegance does not have to be slow.
7. **unsafe is a last resort.** For everything, not just code.

*Written during a 90-minute focus session at 2am. Accuracy not guaranteed.*',
        '2026-06-15T02:00:00+00:00', '2026-06-15T03:30:00+00:00'
    );

-- ── Journal Entries ──────────────────────────────────────────────────────────
INSERT INTO journal_entries (id, project_id, entry_date, content, created_at, visibility)
VALUES
    (
        '$JOURN1', '$PRJ2',
        '2026-06-10',
        'Started the Grand Refactor today. Opened the codebase. Immediately closed it. Made coffee. Opened it again. Found a function called doTheThingForReal(). It does not do the thing. It does not do a thing. I cannot tell what it is for. It has been there since 2019. The original author left the company in 2021. They left no comments. They left no documentation. They left only this function and a legacy of confusion. I have renamed it to mysterySandwich() temporarily. This is not an improvement.',
        '2026-06-10T17:30:00+00:00', 'Private'
    ),
    (
        '$JOURN2', '$PRJ3',
        '2026-06-14',
        'Had a breakthrough with Rust today. I finally understood why the borrow checker was yelling at me about lifetimes. It turns out I was trying to return a reference to a local variable. The local variable would be destroyed at the end of the function. The reference would then point to nothing. This is apparently bad. In JavaScript this would just return undefined and we would all pretend it was fine. Rust does not pretend. I respect this about Rust even when I do not enjoy it.',
        '2026-06-14T20:00:00+00:00', 'Private'
    ),
    (
        '$JOURN3', '$PRJ1',
        '2026-06-17',
        'Replied to Kevin. He replied back within four minutes. I did not know Kevin monitored his inbox that closely. Kevin had three follow-up questions. I have created a Kevin folder. I have created a Kevin filter. The filter marks Kevin emails as "High Priority" automatically because statistically they are. Kevin is now the most organized person in my inbox. Kevin did not ask for this honor. He has earned it through persistence.',
        '2026-06-17T19:00:00+00:00', 'Private'
    ),
    (
        '$JOURN4', '$PRJ4',
        '2026-06-20',
        'Progress update on World Domination v3.1: The manifesto first draft is done. It is 847 words. It is mostly coherent. Three people have read it. Two of them had questions. One of them said "bold vision" which could mean many things. I have scheduled a retrospective for next week. The domain worlddominationv31.com was taken. worlddominationv3point1.com was also taken. I have registered a different one. It is fine. Branding is a second-phase problem.',
        '2026-06-20T21:00:00+00:00', 'Private'
    ),
    (
        '$JOURN5', '$PRJ3',
        '2026-06-22',
        'Wrote my first useful CLI tool in Rust today. It reads a CSV and tells you which rows have missing fields. This sounds simple. It was not simple. It required understanding: File I/O, error propagation, the csv crate, clap for argument parsing, and a brief philosophical journey through what it means for a field to be "missing" versus "empty" versus "present but wrong". The tool works. It has --help flags. I am unreasonably proud of this. The borrow checker only yelled at me twice.',
        '2026-06-22T22:00:00+00:00', 'Private'
    );

-- ── Zen Tree (Stage 3 — Treekin) ─────────────────────────────────────────────
INSERT INTO zen_tree (id, growth, health, stage, last_watered, water_today, total_waterings)
VALUES ('$ZEN_ID', 72, 88, 3, '2026-06-23T08:00:00+00:00', 1, 284);

-- ── Streak ───────────────────────────────────────────────────────────────────
INSERT INTO streaks (id, current_streak, best_streak, last_active_day)
VALUES ('$STREAK_ID', 12, 28, '2026-06-23');

-- ── Achievements ─────────────────────────────────────────────────────────────
INSERT INTO achievements (id, name, description, unlocked_at)
VALUES
    ('first_quest', 'First Quest', 'Complete first task.', '2026-01-18T12:00:00+00:00'),
    ('first_focus', 'First Focus', 'Complete first focus session.', '2026-02-03T14:00:00+00:00'),
    ('scholar', 'Scholar', 'Create 25 notes.', NULL),
    ('chronicler', 'Chronicler', 'Create 50 journal entries.', NULL),
    ('project_master', 'Project Master', 'Complete 10 projects.', NULL),
    ('ancient_gardener', 'Ancient Gardener', 'Grow tree to Stage 5.', NULL),
    ('hundred_day_journey', 'Hundred Day Journey', 'Reach 100-day streak.', NULL),
    ('deep_worker', 'Deep Worker', 'Complete 100 focus sessions.', NULL),
    ('master_concentration', 'Master of Concentration', 'Complete 500 focus sessions.', NULL),
    ('ninety_minute_sage', '90 Minute Sage', 'Complete a 90-minute session.', '2026-06-15T03:30:00+00:00'),
    ('silent_monk', 'Silent Monk', 'Complete 25 focus sessions in silence.', NULL),
    ('forest_wanderer', 'Forest Wanderer', 'Complete 50 focus sessions with Forest Sounds.', NULL),
    ('rain_listener', 'Rain Listener', 'Complete 50 focus sessions with Rain Sounds.', NULL),
    ('master_atmosphere', 'Master of Atmosphere', 'Complete focus sessions with all 8 soundscapes.', NULL),
    ('first_companion', 'First Companion', 'Join first shared project.', NULL),
    ('quest_together', 'Quest Together', 'Complete project with another user.', NULL),
    ('chronicler_fellowship', 'Chronicler of Fellowship', 'Post 100 Chronicle messages.', NULL),
    ('mentor', 'Mentor', 'Invite 10 users.', NULL),
    ('alliance_builder', 'Alliance Builder', 'Participate in 25 shared projects.', NULL),
    ('milestone_first_quest', 'Reluctant Hero', 'You completed a task, wrote a note, and acknowledged the project existed for at least one day.', '2026-02-05T10:00:00+00:00'),
    ('milestone_chronicle_keeper', 'Amateur Historian', 'You showed up on two different days and wrote about it.', '2026-02-22T10:00:00+00:00'),
    ('milestone_focused_adventurer', 'Accidental Monk', 'Three focus sessions without rage-quitting.', '2026-03-05T10:00:00+00:00'),
    ('milestone_realm_builder', 'Management Material', 'Ten tasks completed.', NULL),
    ('milestone_keeper_of_chronicle', 'Unnecessary Biographer', 'Fifteen tasks. Five journal entries.', NULL),
    ('milestone_steady_hero', 'Creature of Habit', 'A seven-day streak. Twenty completed tasks.', NULL),
    ('milestone_master_of_realms', 'Probably Fine', 'Fifty tasks. Twenty notes. Twenty active days.', NULL),
    ('milestone_legend_of_chronicle', 'Unsolicited Archivist', 'One hundred tasks. Twenty-five journal entries.', NULL),
    ('milestone_avatar_of_completion', 'The Myth. The Legend. The Problem.', 'One hundred tasks. Twenty-five daily adventures. A thirty-day streak.', NULL);

-- ── Focus Sessions ───────────────────────────────────────────────────────────
INSERT INTO focus_sessions (id, project_id, task_id, duration_mins, xp_gained, completed_at, soundscape)
VALUES
    ('$FOCUS1', '$PRJ2', '$T06', 25, 30, '2026-06-18T10:30:00+00:00', 'LoFi Radio'),
    ('$FOCUS2', '$PRJ3', '$T11', 50, 65, '2026-06-18T16:00:00+00:00', 'Rain Sounds'),
    ('$FOCUS3', '$PRJ2', '$T07', 25, 30, '2026-06-19T09:30:00+00:00', 'LoFi Radio'),
    ('$FOCUS4', '$PRJ3', '$T12', 90, 120, '2026-06-20T22:00:00+00:00', 'Silent'),
    ('$FOCUS5', '$PRJ1', '$T01', 25, 30, '2026-06-21T11:00:00+00:00', 'Forest Sounds'),
    ('$FOCUS6', '$PRJ4', '$T15', 50, 65, '2026-06-22T20:00:00+00:00', 'Ambient Radio');

-- ── Daily Adventures (today) ──────────────────────────────────────────────────
INSERT INTO daily_adventures (id, title, quest_type, target_count, current_count, completed, created_date)
VALUES
    ('$DQUEST1', 'Complete 3 Tasks', 'complete_tasks', 3, 2, 0, '2026-06-23'),
    ('$DQUEST2', 'Write 1 Journal Entry', 'write_journal', 1, 0, 0, '2026-06-23'),
    ('$DQUEST3', 'Water Your Tree', 'water_tree', 1, 1, 1, '2026-06-23');

-- ── Rituals ──────────────────────────────────────────────────────────────────
INSERT INTO rituals (id, name, description, frequency, reward_xp, created_at)
VALUES
    ('$RITUAL1', 'Morning Terminal Boot', 'Open terminal. Check git status. Drink coffee. In that order.', 'Daily', 15, '2026-02-01T08:00:00+00:00'),
    ('$RITUAL2', 'Weekly Backlog Triage', 'Review all tasks. Move things. Delete things. Pretend you will do the low priority ones.', 'Weekly', 40, '2026-02-01T08:00:00+00:00'),
    ('$RITUAL3', 'Evening Code Review', 'Read what you wrote today as if you are someone who has never met you.', 'Daily', 20, '2026-03-01T08:00:00+00:00');

INSERT INTO ritual_history (ritual_id, completed_date) VALUES
    ('$RITUAL1', '2026-06-17'),
    ('$RITUAL1', '2026-06-18'),
    ('$RITUAL1', '2026-06-19'),
    ('$RITUAL1', '2026-06-20'),
    ('$RITUAL1', '2026-06-21'),
    ('$RITUAL1', '2026-06-22'),
    ('$RITUAL1', '2026-06-23'),
    ('$RITUAL2', '2026-06-16'),
    ('$RITUAL2', '2026-06-23'),
    ('$RITUAL3', '2026-06-20'),
    ('$RITUAL3', '2026-06-21'),
    ('$RITUAL3', '2026-06-22');

-- ── Milestones ───────────────────────────────────────────────────────────────
INSERT INTO milestones (id, project_id, name, description, completed, xp_reward)
VALUES
    ('$MILE1', '$PRJ1', 'Reach Inbox 500', 'From 4,742 to under 500. The first mountain.', 1, 50),
    ('$MILE2', '$PRJ1', 'Achieve Inbox Zero', 'The dream. The myth. The thing.', 0, 150),
    ('$MILE3', '$PRJ2', 'Delete All Dead Code', 'If it has been commented out for over a year, it is archaeology, not code.', 0, 75),
    ('$MILE4', '$PRJ3', 'Ship Something Written in Rust', 'Anything. A hello world with a --name flag counts.', 1, 100),
    ('$MILE5', '$PRJ4', 'Recruit 3 Lieutenants', 'People who respond to messages. People who attend meetings. People who are not Kevin.', 0, 80);

-- ── Reflections ──────────────────────────────────────────────────────────────
INSERT INTO reflections (created_date, what_went_well, what_can_improve)
VALUES
    ('2026-06-20', 'Completed two focus sessions. The Grand Refactor is actually moving forward. Renamed a truly absurd number of variables.', 'Spent 40 minutes renaming a variable and then renamed it back. Must trust the first instinct more.'),
    ('2026-06-21', 'The Kevin situation has been resolved diplomatically. Inbox is at 847 unread, down from 4,742.', 'Started four new browser tabs about Rust things I do not need yet. Close the tabs.'),
    ('2026-06-22', 'Shipped the CSV tool. It works. It has documentation. I am proud of this.', 'The documentation is one sentence. It should be more than one sentence. Two sentences minimum.');

-- ── XP Events ────────────────────────────────────────────────────────────────
INSERT INTO xp_events (id, event_type, xp_gained, timestamp)
VALUES
    ('xp0001-0000-4000-8000-000000000001', 'task_complete', 10, '2026-06-18T10:31:00+00:00'),
    ('xp0002-0000-4000-8000-000000000002', 'focus_session', 30, '2026-06-18T10:31:00+00:00'),
    ('xp0003-0000-4000-8000-000000000003', 'task_complete', 10, '2026-06-18T16:01:00+00:00'),
    ('xp0004-0000-4000-8000-000000000004', 'focus_session', 65, '2026-06-18T16:01:00+00:00'),
    ('xp0005-0000-4000-8000-000000000005', 'task_complete', 10, '2026-06-19T09:31:00+00:00'),
    ('xp0006-0000-4000-8000-000000000006', 'focus_session', 30, '2026-06-19T09:31:00+00:00'),
    ('xp0007-0000-4000-8000-000000000007', 'focus_session', 120, '2026-06-20T22:01:00+00:00'),
    ('xp0008-0000-4000-8000-000000000008', 'note_created', 15, '2026-06-20T22:30:00+00:00'),
    ('xp0009-0000-4000-8000-000000000009', 'focus_session', 30, '2026-06-21T11:01:00+00:00'),
    ('xp0010-0000-4000-8000-000000000010', 'milestone_complete', 100, '2026-06-21T12:00:00+00:00'),
    ('xp0011-0000-4000-8000-000000000011', 'focus_session', 65, '2026-06-22T20:01:00+00:00'),
    ('xp0012-0000-4000-8000-000000000012', 'ritual_complete', 20, '2026-06-22T21:00:00+00:00');

-- ── Codices ──────────────────────────────────────────────────────────────────
INSERT INTO codices (id, project_id, name, created_at)
VALUES
    ('$CODEX1', '$PRJ2', 'Architecture Notes', '2026-02-20T15:00:00+00:00'),
    ('$CODEX2', '$PRJ3', 'Rust Learning Log', '2026-03-10T10:00:00+00:00');

-- Link two notes to codices
UPDATE notes SET codex_id = '$CODEX1' WHERE id = '$NOTE1';
UPDATE notes SET codex_id = '$CODEX2' WHERE id = '$NOTE2';
UPDATE notes SET codex_id = '$CODEX2' WHERE id = '$NOTE5';

-- ── Subtasks ─────────────────────────────────────────────────────────────────
-- Steps under "Split the 2,400-line utils.js file"
INSERT INTO tasks (id, project_id, title, description, due_date, completed, priority, created_at, parent_task_id)
VALUES
    ('$ST01', '$PRJ2', 'Identify all exported functions', 'Map every public function — how many call each other.', NULL, 1, 'High', '2026-06-10T09:00:00+00:00', '$T08'),
    ('$ST02', '$PRJ2', 'Extract date utility functions', 'Move all date helpers to date-utils.js. There are eleven of them.', NULL, 1, 'High', '2026-06-11T09:00:00+00:00', '$T08'),
    ('$ST03', '$PRJ2', 'Extract string formatters', 'Move to string-utils.js. Do not rename them. Just move them.', NULL, 0, 'Medium', '2026-06-12T09:00:00+00:00', '$T08');

-- Steps under "Read the async chapter without giving up"
INSERT INTO tasks (id, project_id, title, description, due_date, completed, priority, created_at, parent_task_id)
VALUES
    ('$ST04', '$PRJ3', 'Read the futures chapter first', 'Async makes no sense without futures. Read that one first.', NULL, 1, 'Medium', '2026-06-13T09:00:00+00:00', '$T13'),
    ('$ST05', '$PRJ3', 'Write a basic async function', 'async fn fetch_something() -> Result<()>. It can fetch nothing. That is fine.', NULL, 0, 'Medium', '2026-06-14T09:00:00+00:00', '$T13');

-- ── Great Chronicle ──────────────────────────────────────────────────────────
INSERT INTO great_chronicle (id, day_number, entry_text, timestamp)
VALUES
    ('$CHRON1', 1, 'MerlinBytes began the Questline. The terminal was opened. The cursor blinked. History was waiting.', '2026-01-15T09:00:00+00:00'),
    ('$CHRON2', 38, 'Operation: Inbox Zero was launched. 4,742 emails stood between MerlinBytes and peace. The sorting began.', '2026-02-21T08:30:00+00:00'),
    ('$CHRON3', 102, 'The borrow checker was defeated in battle. A Rust CLI tool was compiled successfully on the first try. The second time had also been the first try in a different timeline.', '2026-04-26T15:00:00+00:00'),
    ('$CHRON4', 155, 'A 12-day streak was achieved. The Zen Tree reached Stage 3. coffee.js reached 47 weekly downloads. Progress was undeniable.', '2026-06-19T09:00:00+00:00'),
    ('gc000005-0000-4000-8000-000000000005', 160, 'Chapter One: The Notification Swarm is active. The Realm has been called to action. MerlinBytes answered.', '2026-06-25T08:00:00+00:00');

-- ── Class Quests (Code Warlock) ───────────────────────────────────────────────
INSERT INTO class_quests (class_name, unlock_level, quest_name, description, status, progress, target, lore_reward)
VALUES
    ('Code Warlock', 10, 'The Forgotten Compiler', 'Complete 5 tasks to align the compiler parameters and purge syntax anomalies.', 'Completed', 5, 5, 'Unlocks the lore of the Compiler Wizards.'),
    ('Code Warlock', 25, 'The Broken Daemon', 'Dedicate 60 minutes of deep focus to debug and stabilize the rogue background daemon.', 'In Progress', 30, 60, 'Unlocks the lore of the Background Daemons.'),
    ('Code Warlock', 50, 'The Library of Infinite Scripts', 'Water your Zen Tree 3 times to grow script-bearing leaves containing ancient functions.', 'Locked', 0, 3, 'Unlocks the lore of the Leaf Scripts.'),
    ('Code Warlock', 75, 'The Stack Overflow Sigil', 'Complete a project to craft the ultimate code architecture of the Keep.', 'Locked', 0, 1, 'Unlocks the lore of the Architecture Sigils.'),
    ('Code Warlock', 100, 'The Simulation Core', 'Maintain a 7-day streak to boot up the final cosmic simulation engine.', 'Locked', 0, 7, 'Unlocks the ultimate lore of the Simulation Core.');

-- ── Lore Library (a few unlocked entries) ────────────────────────────────────
INSERT INTO lore_library (id, category, title, content, unlocked, unlocked_at)
VALUES
    ('lore001', 'Class', 'The Terminal Covenant',
     'The Code Warlocks
"It worked on my machine."

No one knows exactly how the Code Warlocks began.
Their own records are incomplete.
Mostly because they forgot to back them up.
History records that this was a mistake.',
     1, '2026-01-15T09:01:00+00:00'),
    ('lore002', 'Class', 'The Great Forking',
     'The most famous event in Warlock history was The Great Forking.
A disagreement regarding indentation escalated into a civil war.
Entire repositories split apart.
To this day no one remembers the original argument.
Only that it was important.',
     1, '2026-02-01T10:00:00+00:00'),
    ('lore003', 'Class', 'Traditions',
     'Code Warlocks consume sacred caffeinated beverages before performing major rituals.
The stronger the coffee, the more powerful the magic.
This belief remains scientifically unchallenged.',
     1, '2026-03-10T09:00:00+00:00');

-- ── World Lore (chapters 1–10 unlocked, 11 locked — it is the Chapter One reward) ──
INSERT INTO lore_library (id, category, title, content, unlocked, unlocked_at)
VALUES
    ('world_chapter_1', 'World', 'Before the First Cursor',
     'Before there were projects, before there were tasks, before there were notes and chronicles, there was only the Void.

The Void was not empty.

It was filled with unfinished intentions.

Ideas that would someday be started.

Plans that would someday be organized.

Goals that would someday be pursued.

Yet none of them ever moved beyond possibility.

Nothing was recorded.

Nothing was completed.

Nothing endured.

This forgotten era became known as The Age of Intention.',
     1, '2026-01-15T09:01:00+00:00'),

    ('world_chapter_2', 'World', 'The Age of Open Tabs',
     'As civilization advanced, the people attempted to impose order upon their lives.

They created notes.

They created lists.

They created plans.

Unfortunately, they created them everywhere.

Entire kingdoms buried themselves beneath scattered notebooks, forgotten documents, abandoned projects, and browser windows numbering in the hundreds.

Scholars estimate that some individuals maintained over seventy active tabs simultaneously.

Few survived.

Historians still debate whether productivity truly existed during this period.',
     1, '2026-01-20T10:00:00+00:00'),

    ('world_chapter_3', 'World', 'The Rise of the Great Backlog',
     'Every unfinished task leaves behind a trace.

A postponed promise.

An ignored responsibility.

A project that would be completed "later."

For centuries these fragments accumulated.

Eventually they combined into something terrible.

The Great Backlog.

No one knows its true form.

Some describe a mountain.

Others a storm.

Still others claim it resembles an email inbox with twenty-seven thousand unread messages.

Whatever its nature, its influence spread throughout the world.

Projects stalled.

Deadlines collapsed.

Entire organizations vanished beneath the weight of unfinished work.',
     1, '2026-02-01T10:00:00+00:00'),

    ('world_chapter_4', 'World', 'The First Cursor',
     'During the darkest days of the Great Backlog, a lone traveler discovered a blinking light within the Void.

It appeared as a single cursor.

Patient.

Silent.

Waiting.

The traveler approached.

The cursor blinked.

The traveler blinked.

Neither moved for some time.

Eventually the traveler spoke the words:

"Hello World."

Light spread through the darkness.

Structure emerged.

Projects took form.

Notes gained permanence.

Tasks acquired purpose.

The First Cursor had awakened.',
     1, '2026-02-15T10:00:00+00:00'),

    ('world_chapter_5', 'World', 'The Founding of Questline',
     'In the years that followed, the surviving peoples gathered around the teachings of the First Cursor.

They learned that progress was not achieved through motivation.

Progress was achieved through repetition.

Through systems.

Through consistency.

Through showing up again tomorrow.

These teachings became known as the Questline.

Not because the path was easy.

But because every meaningful achievement was composed of smaller quests completed one after another.',
     1, '2026-03-01T10:00:00+00:00'),

    ('world_chapter_6', 'World', 'The Six Great Orders',
     'As Questline spread across the realm, different groups interpreted its teachings in different ways.

Some pursued structure.

Others sought discipline.

Some mastered knowledge.

Others mastered time itself.

From these philosophies emerged the Six Great Orders.

The Arch Accountants.

The Code Warlocks.

The Mind Sages.

The Task Paladins.

The Systems Architects.

The Time Chronomancers.

Though their methods differed, all sought the same goal:

To bring order to chaos and purpose to effort.',
     1, '2026-03-15T10:00:00+00:00'),

    ('world_chapter_7', 'World', 'The Era of Productivity',
     'For the first time in history, progress became measurable.

Projects were completed.

Goals were achieved.

Knowledge was preserved.

Entire cities prospered under the guidance of the Orders.

Yet success created new challenges.

Scope Dragons multiplied.

Meeting Mimics infiltrated institutions.

Deadline Wraiths appeared in increasing numbers.

The struggle against chaos had entered a new age.',
     1, '2026-04-01T10:00:00+00:00'),

    ('world_chapter_8', 'World', 'The Growth of the Zen Tree',
     'During this period, a mysterious sapling appeared near the center of the realm.

No one knows who planted it.

No one knows where it came from.

Attempts to accelerate its growth failed.

Attempts to manipulate it failed.

Attempts to place it in a productivity framework generated seventeen conflicting methodologies and three conference talks.

The Tree ignored them all.

It grew only through consistent effort.

A little each day.

Never quickly.

Never dramatically.

Yet never stopping.',
     1, '2026-04-15T10:00:00+00:00'),

    ('world_chapter_9', 'World', 'The Age of Chronicles',
     'The greatest weakness of mortals had always been memory.

Victories were forgotten.

Progress went unnoticed.

Growth became invisible.

To solve this, the Orders created the Chronicle.

A living record of journeys, achievements, failures, discoveries, and lessons learned.

The Chronicle does not celebrate perfection.

It celebrates persistence.

Every completed task.

Every finished project.

Every return after a difficult day.

All are recorded.',
     1, '2026-05-01T10:00:00+00:00'),

    ('world_chapter_10', 'World', 'The Present Age',
     'The realm now stands in an era unlike any before it.

The Great Backlog remains beyond the horizon.

Deadline Wraiths continue to roam.

Scope Dragons still tempt adventurers with promises of "just one more feature."

Yet the people endure.

Every day new travelers begin their journey.

Every day new quests are completed.

Every day another page is added to the Chronicle.

The story of Questline remains unfinished.

As all good stories should.',
     1, '2026-06-01T10:00:00+00:00'),

    -- Chapter One reward — locked until the Swarm is defeated
    ('world_chapter_11', 'World', 'The Fate of the Notification Sprites',
     'When the Swarm finally broke, no great battle was recorded.

No armies marched.

No ancient relic was activated.

No chosen hero stood atop a mountain and delivered a dramatic speech.

The Sprites simply began to vanish.

One by one.

Then hundreds at a time.

Then thousands.

Across the Realm, unfinished quests were completed.

Scrolls were written.

Reflections were recorded.

Focus sessions were honored.

The Swarm had always fed upon hesitation.

Every ignored task.

Every postponed intention.

Every promise made to "start tomorrow."

The Notification Sprites themselves were never evil.

Merely hungry.

Something else had nurtured the conditions that allowed the Swarm to grow.

Something patient.

Something ancient.

Something that preferred heroes distracted.

At the edge of recorded history, the Chronicle found references to a force long believed dormant.

A force known only as:

The Great Backlog.

The horizon darkened.

The Realm celebrated its victory.

But the Chronicle quietly turned to the next page.',
     0, NULL);

-- ── Memory Fragments ─────────────────────────────────────────────────────────
INSERT INTO lore_library (id, category, title, content, unlocked, unlocked_at)
VALUES
    ('memory_001', 'Memory', 'Fragment #001 — Sir Aldric the Hesitant',
     'Recovered from the Third Age of Checklists

"I kept postponing the task because I wanted to do it perfectly.

Three months later I realized doing it badly would have been sufficient."

— Sir Aldric the Hesitant
Task Paladin, Level 63

Status: Eventually completed.',
     1, '2026-02-10T11:00:00+00:00'),

    ('memory_002', 'Memory', 'Fragment #002 — Warlock Bryn of Forty-Seven Tabs',
     'Recovered from a damaged project archive

"Today I spent six hours building a system to save five minutes.

Tomorrow I shall spend another six improving it."

— Warlock Bryn of Forty-Seven Tabs

Status: Project still active.',
     1, '2026-03-22T14:00:00+00:00'),

    ('memory_003', 'Memory', 'Fragment #003 — Accountant General Harlan',
     'Recovered from the Great Backlog War

"The dragon was terrifying.

The budget meeting was worse."

— Accountant General Harlan

Status: Unknown.',
     1, '2026-04-08T09:00:00+00:00'),

    ('memory_004', 'Memory', 'Fragment #004 — Systems Architect Vexa',
     'Recovered from an abandoned notebook

"If you have reorganized the same folder four times,
you are no longer organizing.

You are decorating."

— Systems Architect Vexa

Status: Folder renamed twice more.',
     1, '2026-05-14T16:00:00+00:00'),

    ('memory_005', 'Memory', 'Fragment #005 — Chronomancer Elian',
     'Recovered from the Hall of Clocks

"The deadline was visible from the beginning.

I simply believed I was special."

— Chronomancer Elian

Status: Deadline victorious.',
     0, NULL),

    ('memory_006', 'Memory', 'Fragment #006 — Unknown Hero',
     'Recovered from a damaged coffee-stained scroll

"The task took twenty minutes.

Avoiding the task took three weeks."

— Unknown Hero

Status: Classic.',
     0, NULL),

    ('memory_007', 'Memory', 'Fragment #007 — Sage Corvin',
     'Recovered from the Archives of the Mind Sages

"I took excellent notes.

Unfortunately they were spread across seven notebooks,
three applications,
and one napkin."

— Sage Corvin

Status: Information technically preserved.',
     0, NULL),

    ('memory_008', 'Memory', 'Fragment #008 — Chronicle Entry 11,204',
     'Recovered from the Chronicle

"The hero requested motivation.

The Chronicle provided a deadline."

— Chronicle Entry 11,204',
     0, NULL),

    ('memory_009', 'Memory', 'Fragment #009 — Fellowship Log #382',
     'Recovered from an ancient project board

"Halfway through the project,
we realized nobody remembered why we started."

— Fellowship Log #382',
     0, NULL),

    ('memory_010', 'Memory', 'Fragment #010 — The Last Finisher',
     'Recovered from a sealed vault

"The greatest productivity technique I ever discovered
was beginning."

— The Last Finisher

Status: Confirmed.',
     0, NULL),

    ('memory_077', 'Memory', 'Fragment #077 — The Sixth Chronicle [Rare]',
     'Recovered from a forbidden archive

"The Great Backlog can never be destroyed.

Only managed."

— The Sixth Chronicle

[ Rare Fragment ]',
     0, NULL),

    ('memory_112', 'Memory', 'Fragment #112 — Rootkeeper Sol [Rare]',
     'Recovered from the roots of the Zen Tree

"Heroes ask how long the tree takes to grow.

The tree asks how long they plan to remain."

— Rootkeeper Sol

[ Rare Fragment ]',
     0, NULL),

    ('memory_144', 'Memory', 'Fragment #144 — Future You [Rare]',
     'Recovered from the Future

"Everything worked out.

Now stop worrying and finish the task."

— Future You

[ Rare Fragment ]',
     0, NULL),

    ('memory_188', 'Memory', 'Fragment #188 — Chronomancer Voss [Rare]',
     'Recovered from a corrupted timeline

"I finally reached Inbox Zero.

Nobody was there to witness it."

— Chronomancer Voss

Status: Scholars debate authenticity.

[ Rare Fragment ]',
     0, NULL),

    ('memory_999', 'Memory', 'Fragment #999 — Unknown [Legendary]',
     'Recovered from the deepest vault beneath the Chronicle

"There was never a chosen one.

There were only people who continued showing up.

Again.

And again.

And again."

— Unknown

The remainder of the fragment has been lost.

[ Legendary Fragment ]',
     0, NULL),

    -- Chapter One reward — locked until the Swarm chapter completes
    ('memory_ch1_001', 'Memory', 'Fragment #CH1-001 — The Last Quiet Morning [Chapter Reward]',
     'Recovered from the Early Chronicle

"I remember the morning after the Swarm vanished.

No pings.

No banners.

No red circles demanding attention.

For the first time in years, the Realm was silent.

The silence was unsettling.

Many heroes believed something was wrong.

Several Task Paladins spent hours refreshing things that no longer needed refreshing.

One Code Warlock claimed the silence was suspicious and restarted three perfectly functioning systems.

The Mind Sages simply smiled.

It took several days before the Realm remembered what silence felt like.

Most agreed it was pleasant.

A few admitted they missed the chaos.

The Chronicle records both opinions."

— Unknown Hero

[ Chapter One Reward Fragment ]',
     0, NULL);

-- ── Realm Activity Feed — global_chronicle ───────────────────────────────────
-- Simulates the live feed of other heroes contributing to Chapter One across the realm
INSERT INTO global_chronicle (id, hero_name, event_type, description, timestamp)
VALUES
    ('$GC01', 'Valdris',      'QuestComplete',     'Valdris completed "Debug the production daemon" and pushed the Swarm back.', '2026-06-25T06:14:00+00:00'),
    ('$GC02', 'Kessa',        'FocusSession',       'Kessa completed a 50-minute focus session. The Swarm flinched.', '2026-06-25T07:02:00+00:00'),
    ('$GC03', 'Thornbite',    'ScrollCreated',      'Thornbite wrote a new scroll: "On the Nature of Infinite Notifications".', '2026-06-25T08:30:00+00:00'),
    ('$GC04', 'Aelwyn',       'TreeWatering',       'Aelwyn watered the Zen Tree. Growth continued undisturbed.', '2026-06-25T09:15:00+00:00'),
    ('$GC05', 'Pip',          'SidequestComplete',  'Pip completed the Morning Ritual sidequest. Consistency noted.', '2026-06-25T10:00:00+00:00'),
    ('$GC06', 'The Realm',    'QuestComplete',      'The realm crossed 100 quests completed. The Swarm''s numbers are falling.', '2026-06-25T11:45:00+00:00'),
    ('$GC07', 'Valdris',      'ReflectionWritten',  'Valdris recorded a reflection. What went well: everything. What to improve: the definition of everything.', '2026-06-25T13:20:00+00:00'),
    ('$GC08', 'Sera',         'LevelUp',            'Sera reached Level 12. The Code Warlock''s power grows.', '2026-06-25T14:55:00+00:00'),
    ('$GC09', 'Orin',         'FocusSession',       'Orin ran a 90-minute deep work session. Soundscape: Rain. The Swarm lost ground.', '2026-06-25T16:30:00+00:00'),
    ('$GC10', 'Kessa',        'QuestComplete',      'Kessa finished "Migrate the old API endpoints." Three days ahead of schedule.', '2026-06-26T07:10:00+00:00'),
    ('$GC11', 'Thornbite',    'TreeWatering',       'Thornbite watered the Zen Tree for the 30th consecutive day.', '2026-06-26T08:00:00+00:00'),
    ('$GC12', 'Mira',         'ScrollCreated',      'Mira created a scroll: "System Design for People Who Hate Meetings".', '2026-06-26T09:45:00+00:00'),
    ('$GC13', 'The Realm',    'QuestComplete',      'The realm crossed 250 quests completed. Chapter One objectives are advancing.', '2026-06-26T11:00:00+00:00'),
    ('$GC14', 'Aelwyn',       'SidequestComplete',  'Aelwyn completed the Weekly Backlog Triage ritual.', '2026-06-26T13:30:00+00:00'),
    ('$GC15', 'Pip',          'ReflectionWritten',  'Pip wrote a reflection. Day 22 of the habit. The streak holds.', '2026-06-26T15:00:00+00:00'),
    ('$GC16', 'Sera',         'FocusSession',       'Sera completed a 25-minute focus session. Lo-Fi Radio. Back to back with another.', '2026-06-26T16:40:00+00:00'),
    ('$GC17', 'Orin',         'QuestComplete',      'Orin completed "Write the architecture document nobody asked for." It was necessary.', '2026-06-27T07:55:00+00:00'),
    ('$GC18', 'Mira',         'TreeWatering',       'Mira watered the Zen Tree. Stage 4. The canopy is forming.', '2026-06-27T09:00:00+00:00'),
    ('$GC19', 'The Realm',    'QuestComplete',      'The realm crossed 400 quests completed. The Notification Swarm is retreating.', '2026-06-27T10:30:00+00:00'),
    ('$GC20', 'Valdris',      'Milestone',          'Valdris completed the "Ship It" milestone. The project is live. The realm notes this.', '2026-06-27T11:15:00+00:00');

-- ── Chapter Contribution Baseline — chapter_contribution_log ─────────────────
-- Tracks what MerlinBytes has already sent to the server so contributions are not double-counted
INSERT INTO chapter_contribution_log (chapter_id, objective_type, last_sent_total)
VALUES
    ('chapter_one', 'tasks_completed',    11),
    ('chapter_one', 'subtasks_completed',  4),
    ('chapter_one', 'scrolls_created',     5),
    ('chapter_one', 'focus_sessions',      6),
    ('chapter_one', 'tree_waterings',    284),
    ('chapter_one', 'rituals_completed',  12),
    ('chapter_one', 'reflections_written', 3);

ENDSQL

echo ""
echo "Demo database created successfully!"
echo ""
echo "  User:   MerlinBytes  (Code Warlock, Level 15)"
echo "  Path:   $DB_PATH"
echo ""
echo "Projects:"
echo "  • Operation: Inbox Zero"
echo "  • The Grand Refactor        (codex: Architecture Notes)"
echo "  • Learn Rust (For Real This Time)  (codex: Rust Learning Log)"
echo "  • World Domination v3.1"
echo "  • coffee.js  [completed]"
echo ""
echo "Chapter One: The Notification Swarm"
echo "  • 20 realm activity entries in the Realm Feed"
echo "  • Contribution baseline seeded (tasks, focus, tree, rituals, reflections)"
echo "  • World lore: chapters 1-10 unlocked, chapter 11 locked (chapter reward)"
echo "  • Memory fragments: 4 common unlocked, rest locked"
echo ""
echo "Launch Questline to see your demo data."
