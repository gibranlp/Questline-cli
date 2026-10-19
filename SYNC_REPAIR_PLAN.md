# Sync, profile transfer, and shutdown repair plan

Audit date: 2026-10-19. Scope: Rust CLI/TUI, browser client, both PHP APIs, local backups, profile import/export, and graceful quit. The audit below records the original findings. The implementation status and rollout requirements follow; changes are local and have not been deployed.

## Implementation status

Implemented locally:

- SQLite-consistent, validated backups and atomic JSON export; private file permissions; profile identity included in complete exports.
- Staged cloud and JSON restore, validation before activation, atomic data/identity installation, destination device preservation, and a concurrent-edit check under the activation write lock. A failed download leaves the active profile intact.
- Encrypted cloud backups stored independently of event history and bound to its generation, so a stale backup cannot resurrect state after a reset. Upload errors are reported accurately. An explicit reset requires the current server head; changed history generations stop stale clients from publishing.
- OS-level per-profile coordination of sync, backup upload, and restore. Manual and transfer HTTP work runs off the UI thread. Optional presence/activity refresh is separated from core sync completion and checks the active identity before writing.
- Exact encrypted outbox envelopes survive uncertain acknowledgments and restart. Delivery uses bounded batches and acknowledges accepted pushes independently of pull success. Normal manual sync sends deltas instead of re-publishing the complete profile.
- Pull application, deduplication, and cursors commit together. Per-event savepoints retain dependent events in a durable inbox so missing keys/parents do not block later pages. Invalid notes/tasks remain recoverable in quarantine; unsupported event types remain deferred.
- Deletion versions prevent stale resurrection. Reflections, traits, class quest progress, titles, relics, personal chronicle/lore, notifications, activities, reactions, preferences, and reward flags now have change capture, initial backfill, and sync handlers.
- Browser edits and immutable queue entries commit together in IndexedDB. Each destination is acknowledged separately, queues survive logout, and caches are isolated by account. CLI catchup runs automatically; stranded legacy web writes can be bridged back to the authoritative stream. An explicit reset stops that legacy bridge.
- Browser and CLI agree on streak aliases, note permissions, dated quests, focus session fields, and XP baselines for new snapshots. Browser reflections are synced; former local-only reflections are migrated for the original cache owner.
- Quit verifies pending uploads, download lag, and deferred events before reporting cloud success. The quit dialog also offers an explicit local-only exit with queued changes retained. Automatic polling cannot repeatedly award itself sync XP.
- No-op sync completions avoid broad UI reloads; the Sync screen counts its outbox through SQL instead of loading it every frame. HTTP connections are reused, client errors are not retried, failures back off, and sync duration is recorded in `last_sync_duration_ms`.

Still requires rollout/operational validation:

- Apply migrations 008 and 009 to the encrypted database, deploy the PHP API and browser together, and release the updated CLI. See `server/migrations/README.md`.
- Verify on two real devices with representative large profiles and measure latency/frame-time budgets. The audit's proposed performance targets are not production measurements.
- Backups currently retain the latest independent encrypted snapshot. Historical snapshot browsing/retention and bundling local external files are separate follow-up features; local music paths remain device-specific.
- Abrupt process termination/browser shutdown can interrupt in-memory drafts. Saved edits/outbox entries survive restart; this change does not promise network delivery during power loss or an OS force-kill.


## Findings

The code already has valuable protections: SQLite WAL, an incremental CLI outbox, encrypted and signed events, duplicate detection, conflict revisions, transactional JSON import with relationship validation, and quit tests. The problem is that the different sync and transfer paths do not share the same guarantees.

| Priority | Finding and evidence | Consequence |
| --- | --- | --- |
| P0 | TUI profile export and pre-restore backup use `std::fs::copy` on the live SQLite database (`src/app/mod.rs`, export near 12358 and restore near 8161/10071). CLI `backup` does the same (`src/main.rs:850`). WAL is enabled in `Database::new`. | Recent committed changes can be absent from the exported backup. Reproduced with an isolated WAL database: live row count 1, raw copy 0, SQLite backup API 1. |
| P0 | Profile export and cloud backup call `replace_with_pending_snapshot`; the server snapshot endpoint deletes the account's existing event history (`server/api/index.php:1267`). These workers run separately from `sync_in_progress`. | An export can replace remote changes this device has not pulled. Sync, backup, reset, and restore can race. Missing local entities and historical deletes are especially dangerous. |
| P0 | Cloud backup returns `Ok("Cloud Backup Successful! ... warning")` after snapshot failure; export likewise says cloud backup was saved after an upload error (`src/app/mod.rs:12281`, `12401`). | A user can transfer devices believing a failed upload succeeded. |
| P0 | Transfer-code restore overwrites identity before successful recovery and calls ordinary bidirectional `engine.sync()` on the existing database, synchronously in key handling (`src/app/mod.rs:8118`, `10011`). It does not establish a fresh account cursor/cache there. | UI freezes; old-account data, pending mutations, and cursor state can contaminate restore. A failed restore leaves the identity changed. |
| P1 | Browser account writes have no durable outbox; only project writes are queued (`webapp/src/lib/sync.js:234`). A failed second dual-push is silently ignored. | Offline changes can remain local forever; a web-only accepted write may never reach the CLI. |
| P1 | Browser background sync relies on webapp polling and manual CLI catchup (`webapp/src/lib/sync.js:480`). Encrypted server push does not invoke the legacy webhook dispatcher (`server/api/index.php:1090`); webhook ingest writes legacy `sync_events`, while encrypted pulls read `sync_v2_events` (`webapp/api/index.php:145`, `411`). | The automatic CLI-to-browser bridge described by comments is incomplete for v2. Project pulls skip account events. |
| P1 | Reflections and trait unlocks write local tables without outbox events (`src/database/mod.rs:3401`, `3409`), and lack sync-engine handlers. Synced preferences are restricted to ten keys (`src/database/mod.rs:326`). | “Everything the user does syncs” is not currently true. Other features need a complete coverage inventory. |
| P1 | Browser recognizes `streaks`, while CLI emits `streak`; browser also lacks handlers for several CLI entities, including `xp_event`, `daily_adventure`, and synced settings (`webapp/src/lib/store.js`, `sync.js`). | Transport delivery does not ensure the same state or progress appears in both clients. |
| P1 | CLI pull application contains ignored database errors and records an event as processed afterward. Entity application, dedup, and cursor advancement have no encompassing transaction (`src/services/sync_engine.rs:1651–3526`). Browser swallows cache-write errors and advances cursors; missing project keys return false but callers advance anyway. | Events can be permanently skipped after an application/cache failure or delayed key delivery. |
| P1 | JSON import clears `last_pull_seq`, but the active engine reads `last_pull_seq_v2` (`src/database/mod.rs:4723`, `src/services/sync_engine.rs:1451`). It also restores device/server bookkeeping from generic table exports. | Restore can resume at an inappropriate cursor. Backup data and replication bookkeeping need separate rules. |
| P1 | Quit checks pending changes only when following an already-running sync. A directly started final pass can exit after an `Ok` result without proving its outbox drained (`src/app/mod.rs:22446`, `23127`). The engine can leave deferred/unauthorized changes pending or stop at its page budget. | “Synchronized” on exit is stronger than what was verified. Existing tests establish basic wait/failure behavior, but not complete delivery. |
| P2 | Normal sync pulls before pushing, pushes an unbounded pending batch, then pulls again. Background completion also waits for invitations, presence, and global activity requests before publishing its result. HTTP requests allow 30 seconds per attempt and three attempts (`api_client.rs:9`, `114`). | Upload latency and apparent sync duration include remote backlog and unrelated network work. These are code-based latency contributors; production timings have not been measured. |
| P2 | The UI connection's SQLite busy timeout drops to 50 ms during sync; completion reloads broad application state. Some local ticks are suspended while sync runs (`src/app/mod.rs:23379`, `22437`; `src/main.rs:1029`). | Database contention can become user-visible save errors; reloads and suspended ticks can contribute to lag. Measure before choosing tuning values. |

## Required guarantees

1. Every acknowledged durable user change is committed locally together with a pending sync record. The UI distinguishes an editor draft from a saved change.
2. Retries survive process crashes and preserve event identity/content. Server acceptance with a lost response is safe to retry.
3. A remote event is durably applied, durably deferred, or explicitly quarantined before its cursor can advance. Unsupported types are visible rather than silently treated as success.
4. Devices converge after delivery stops, including deletes, offline edits, progress, and shared-project changes. Concurrent conflicts use the same documented rules in both clients.
5. Export captures a consistent portable profile. Import either activates a validated restored profile or leaves the current profile intact.
6. Normal sync and export preserve remote history. Cloud reset is a separate explicit operation with concurrency checks.
7. “Synced” means the relevant outbox is acknowledged and the reported pull target has been reached. Pending, deferred, failed, and quarantined counts remain visible.

## Implementation sequence

### 1. Make backups and transfer safe first

- Add one reusable SQLite backup service using `Connection::backup`, temporary files, integrity/relationship checks, and atomic rename. Resolve the active profile database from the connection. Use it for export, pre-import backup, CLI backup, and recovery.
- Save any open editor before profile export. Export JSON inside one consistent read transaction.
- Stop automatic remote history replacement from export/backup. Store cloud backups separately from the live sync stream, with a snapshot identifier, manifest, schema version, and checksum. Keep reset separate.
- Report local export success and cloud upload success independently. Propagate upload errors.
- Route sync, export upload, backup, restore, and reset through one coordinator per active profile; provide cross-process coordination for CLI commands and desktop helpers too.
- Stage transfer-code and file imports in an isolated database/profile. Cloud recovery is pull-only. Validate identity, keys, relationships, and content before activation; do not push the destination's previous data.
- Preserve destination device identity, reinitialize account/server-scoped replication metadata, and fix the v2 cursor reset. Make partial/older JSON import semantics explicit and validate required content.
- Define a portable profile archive containing database state, required identity/key material, preferences, and asset references or included assets. Maintain readers for existing JSON exports and transfer codes. A transfer code restores credentials; it is not a standalone data backup.

### 2. Establish complete mutation coverage

- Inventory every durable mutation in database services, TUI actions, shell API, desktop helpers, and browser routes. Track: entity, account/project scope, serializer, apply/delete handler, conflict rule, export/import coverage, and tests.
- Cover reflections, trait unlocks, progression/rewards, rituals/history, quests/steps/dependencies, journals/notes/codices, Atlas, treasury, hydration, notifications, assignments, and shared membership/key operations. Inspect remaining tables for missing user state.
- Separate account preferences from machine resources. Sync user choices such as theme/layout; retain device IDs, credentials for device services, local paths, and live playback state with the appropriate device. Document every intentional exception to “everything.”
- Give Rust and JavaScript a shared protocol contract with fixtures; reconcile `streak`/`streaks` and every supported event type. Derived views may differ, but unsupported data must remain recoverable.
- Commit entity changes and outbox entries atomically. Audit raw SQL and best-effort writes; use the outbox as scheduling truth rather than relying only on UI `mark_dirty()` calls.

### 3. Make delivery durable and convergent

- Extend the Rust outbox to persist the exact outbound envelope before first transmission; retain a stable event ID through retries. Coalesce only unsent redundant state updates, preserving deletes and append-only progress events.
- Add a transactional IndexedDB outbox for all browser mutations, atomically saved with local state. Partition storage and cursors by account/server. Persist retry state and ordered dependencies; repair UUID-based project outbox ordering.
- Add bounded event/byte batches, explicit acceptance acknowledgments, retry backoff with jitter, and structured error categories. Separate successful push acknowledgment from pull success.
- Apply each pull batch in a short local transaction with dedup and cursor updates, never holding a database transaction during HTTP. Persist deferred events when keys or parents are unavailable. Make quarantine inspectable and recoverable.
- Add durable deletion tombstones/version state so delayed offline upserts do not resurrect deleted content. Define deterministic tie handling and field/entity-specific merges; preserve note conflict revisions and additive XP behavior.
- Prefer one authoritative encrypted event service used by both clients. If the two databases must remain, use a durable server replication outbox with acknowledgments and automatic reconciliation in both directions. Client dual-push and legacy webhooks cannot be the delivery guarantee.
- Introduce account history generations and consistent snapshot-plus-tail bootstrap before pruning/replacing event history. Existing clients must detect a generation change and recover without replaying stale writes into a replacement snapshot.

### 4. Improve responsiveness and shutdown

- Use the same worker/coordinator for manual, automatic, restore, and quit operations. Keep all HTTP off the UI thread. Publish core sync completion before optional presence/activity refreshes.
- Schedule push promptly after a committed mutation with a short debounce and a maximum wait during continuous editing. Poll pulls independently; wake on reconnect and browser visibility. Measure before adding streaming transport.
- Measure serialization, encryption, request, server commit, apply, lock wait, and UI reload times. Optimize repeated queries and refresh changed views instead of reloading everything.
- Replace quit booleans with an explicit flow: save drafts → capture local mutation generation → drain through that generation → record pull status → verify local flush → exit. If shutdown produces more durable changes, include them before declaring success.
- On timeout/offline failure offer retry or explicit exit with local changes queued. Never call queued changes synchronized. Check SQLite checkpoint result rows; executing the PRAGMA alone does not establish that checkpointing completed.
- Browser correctness must come from persisting each edit before close; unload-time HTTP is only best effort. Cover terminal signals and OS close with local durability and next-launch retries.

### 5. Prove and roll out the repair

- Build a deterministic two-device harness plus PHP integration tests using disposable databases and a controllable transport. Keep existing encryption/signature and shared-project authorization tests.
- Test offline edits/restarts; accepted push with lost acknowledgment; failed second mirror delivery; interrupted pull transactions; delayed keys/parents; duplicate/out-of-order events; edit/delete races; clock skew; independent XP; and a backlog larger than one sync budget.
- Round-trip every persistent feature through profile export/import, including committed WAL changes, older formats, a non-default profile, malformed data, failed identity writes, and failure before activation.
- Test quit during edits, an in-flight sync, backup, restore, retry delay, pending deferred events, and local database contention. Assert that UI input stays responsive and no false success is shown.
- Establish latency and frame-time baselines on small and large profiles. Proposed healthy-network target: outbound acknowledgment within 2 seconds of the last saved edit; remote visibility within 5 seconds for active clients; no UI-thread network calls. Agree final budgets after baseline measurements.
- Ship additive database/protocol changes first, compatible clients second, and migrate/backfill existing profiles only after verified backups. Enable the new path gradually; retain rollback and recovery tools. Remove old paths after convergence checks pass.

## Validation performed during this audit

- `cargo test --lib -- --test-threads=2`: **421 passed, 0 failed, 1 ignored**.
- `npm test` in `webapp`: **2 test files passed** (crypto and collaboration). These do not exercise the browser sync lifecycle.
- Isolated SQLite WAL reproduction confirmed raw file copying misses committed changes while the backup API preserves them.
- No production account requests, live data changes, deployment, or sync repair implementation performed. Server conclusions describe the checked-out PHP code; deployed versions and production latency still need verification.

Recommended delivery order: backup/restore safety → coverage and protocol contract → durable delivery and server convergence → responsiveness and verified quit → fault testing and staged rollout. Each implementation change should include its relevant regression tests before proceeding.


## Implementation validation (2026-10-19)

- Rust library: **436 passed**, zero failed, two ignored in the normal suite (the new network transfer test is run separately).
- Rust CLI parsing: **3 passed**. The final normal suite used an isolated `QUESTLINE_STORAGE_DIR`.
- Browser: **21 passed**, including actual IndexedDB transactions with `fake-indexeddb`.
- PHP/MariaDB: **38 API checks passed** against disposable localhost services, including signatures, replay, Fellowship roles/revocation, independent backups, reset preconditions, and generation guards.
- Rust encrypted cloud transfer integration: **passed separately**, including recovery after reset with a stale backup.
- CLI smoke test: **passed** for identity/data export/import, replacement of target-only state, device/cursor preservation rules, WAL-safe backup, and 0600 export permissions.
- Browser production build, PHP syntax checks, and `git diff --check`: **passed**. The browser build retains existing unrelated accessibility/CSS warnings.
- Production services, the installed CLI, and live account data were not deployed or migrated. Real-device performance validation remains part of rollout.

## Deployment and Portals follow-up — 2026-10-19

- Applied and verified migrations 008 and 009 in the production encrypted database `gibranlp_QuestlineE` via SSH to `gibranlp.dev`. Deployed the PHP API atomically with a private rollback copy, remote PHP syntax validation and checksum verification. HTTPS authentication rejection verified.
- The owner paused web app work in favor of the local CLI and SpectrumOS integration. No web app deployment or public CLI release was performed.
- Added Portals to the local profile: native `[0]` TUI management, `questline portals` commands, account-encrypted categories/links/deletions, monotonic per-device visit counts, profile export/recovery coverage, and a JSON bridge for SpectrumOS Favorites.
- Added the desktop `spectrum-questline-sync.service`, respecting Auto Sync, recovery holds, profile/process locks and offline backoff. Favorites refreshes while open without replacing local drafts; stale edits are rejected.
- Installed the optimized CLI and SpectrumOS integration on this PC. All 143 existing links, 36 categories (including Uncategorized), and historical counts were preserved. Backups are under `/home/gibranlp/.local/state/spectrumos/questline-portals-backups/20261019T144059Z`; the database backup passed integrity verification. Installed shell files and binary match maintained sources/build.
- Live sync completed with no queued changes, deferred events, reported lag, or background error. SpectrumShell stayed active and loaded the updated configuration cleanly. A second physical PC still needs the compatible CLI and the same identity with Auto Sync enabled; multi-device behavior was validated using simulated encrypted peers, not two physical PCs.
- Final validation: 449 library tests and 3 CLI tests passed (2 separate existing/network tests ignored by the normal suite); 7 isolated CLI/Quickshell bridge tests passed, including actual autosave queue/remote-refresh/conflict behavior; the 43 SpectrumOS Favorites backend and 17 rendered widget checks passed. The optimized CLI build, Python compilation, shell syntax checks and `git diff --check` passed. The earlier separate encrypted cloud-transfer integration and API security checks remain documented above.

See [Portals and SpectrumOS Favorites](docs/portals.md) for controls, migration and operational details.

## SpectrumOS Sync/profile controls — 2026-10-19

Installed a Sync tab in the Questline widget, backed by the new stdin JSON-lines `sync-json` interface. It exposes local Cloud/Auto Sync preferences, incremental Sync Now, delivery/service/device status, profile JSON export and private Transfer Code display/copy after successful cloud backup, consistent local SQLite plus optional encrypted cloud backup, staged file/cloud/identity restoration, and confirmed 30-day private task pruning.

Restores and pruning require an in-panel preview/confirmation. Restore activation retains a verified pre-restore database copy and the destination device identity, keeps the local Cloud Sync preference, and leaves Auto Sync off for review. Every activation rotates a local shell profile epoch. Old widget mutations are rejected, old drafts are archived before rebinding (including terminal-initiated restores), archive failures preserve/block those drafts, and reminder caches reload the active profile. Transfer Codes are sent over stdin and stay out of draft persistence. Pruning now preserves shared campaigns and branches with recent or incomplete steps while recording tombstones for every removed task/step. A running TUI polls the same per-PC sync flags; its configured server URL remains correct when Cloud Sync starts disabled.

Validation: 457 Rust library tests plus 3 CLI tests passed, with the normal suite's 2 ignored tests left separate; the encrypted transfer integration passed separately. All 10 new integration checks passed against isolated profiles and an actual disposable PHP/MariaDB API, including export/Transfer Code cloud recovery, current-identity cloud restore preview/cancel, a simulated second PC, manual sync with Auto Sync off, file restore/cancel/concurrent-edit rejection, pruning, draft preservation and rendered controls. The 61 related SpectrumOS backend checks and 20 rendered workspace checks passed. Python/shell syntax checks, optimized build and `git diff --check` passed. Disposable containers/network were removed.

Local deployment backup: `/home/gibranlp/.local/state/spectrumos/questline-sync-controls-backups/20261019T152321Z`. Its database integrity check passed. Installed shell files and CLI match source/build, SpectrumShell loaded cleanly, and shell/sync/reminder services are active. The live profile has no queued changes or recorded background sync error; existing Favorites were preserved. No live profile export, restore or pruning was invoked during deployment verification. No web deployment, public release, package publication or ISO build was performed.

See [the Sync widget guide](docs/sync-widget.md).
