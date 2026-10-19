# Portals and SpectrumOS Favorites

Portals are categorized web bookmarks owned by the local Questline profile. Open the **[0] Portals** tab in the TUI. Use Left/Right to select a category, Up/Down to select a link, Enter to open, `n` to add, `e` to edit or move, `d` to delete, `c` to create a category, `r` to rename it, `D` to delete an empty category, and `/` to search. Editors use Tab to change fields, Enter to save, and Escape to cancel. The Category field accepts an existing category name. Browser launch uses the system default.

For scripting, run `questline portals help`. For example:

```sh
questline portals category Research
questline portals add 'Rust documentation' 'https://doc.rust-lang.org/' Research
questline portals list Research
```

Titles, HTTP/HTTPS URLs, categories, moves, deletions and launch counts enter the encrypted account sync stream. They are private profile data and are not shared with Fellowships. Profile JSON exports, encrypted backups and profile recovery include this collection. Independent local edits to different links are retained. Edits to the same record use the existing deterministic last-write conflict rule; a stale open editor is rejected instead of replacing a newer saved version.

SpectrumOS Favorites reads and writes through `questline favorites-json`, a bounded JSON protocol over stdin/stdout. It does not write Questline SQLite tables directly. Initial connection imports the preexisting collection once. Stable migration IDs and baseline counters make repeat imports idempotent; known deleted rows are not resurrected. A bound cache from another profile is never imported. The private Favorites JSON file retains local view state, search and incomplete drafts and serves as a cache. It is not the authority for synced data. Browser associations and favicon caches stay local.

Visit totals combine monotonic counters per device. Imports establish a shared baseline rather than adding historical counts repeatedly. Sync replay cannot inflate counts. A category removed on another PC does not hide surviving links; they remain accessible in Uncategorized.

Enable Auto Sync on every PC using the same Questline identity/Transfer Code. The `spectrum-questline-sync.service` user service runs `questline sync-background` while the TUI is closed. It normally polls every five seconds, backs off to at most five minutes after failures, observes recovery holds and the Auto Sync toggle, and uses the same process lock as sync and profile transfers. The open Favorites widget refreshes every 2.5 seconds. Offline changes remain queued on disk. This is eventual synchronization; network delays and the polling intervals affect when another PC sees an edit.

On systems without the service, keep the TUI open with Auto Sync enabled or run `questline sync-background`. Profile restores initially leave Auto Sync disabled for review; enable it explicitly afterward. Upgrade all participating CLIs to a build supporting Portals before expecting them to receive this new entity type. The PHP API already accepts encrypted account events; no additional server migration or API deployment is needed for Portals.

Validation uses isolated profiles, simulated encrypted peers and the actual CLI/Quickshell bridge. No web app changes or public release are part of this feature.
