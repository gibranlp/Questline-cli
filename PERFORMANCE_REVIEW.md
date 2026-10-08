# Questline responsiveness review

Reviewed task creation/completion, cache refreshes, frame ticks, audio/notification dispatch, database access, and graceful shutdown.

## Findings and changes

- Every statistics refresh previously rebuilt milestone statistics for every active campaign, even campaigns without unfinished milestones. Each rebuild deserialized all tasks, notes, journals, and focus sessions. Refreshes now skip campaigns without unfinished milestones and use SQL counts for milestone progress. Focus-session totals reuse the statistics count.
- Daily-adventure updates reloaded all UI data even when no unfinished adventure matched the action. Those calls now return without a reload when nothing changes.
- Cloud shutdown waited for invitations, companion presence, the Realm feed, and chapter refreshes after durable state synchronization. Exit-triggered synchronization now skips these optional refreshes. Normal sync still performs them.
- Quitting during an existing sync previously always scheduled another pass. A second pass now runs only if unsynced changes or a restore hold remain. Successful shutdown skips UI cache rebuilding. Local saves, required synchronization, sync-failure handling, and the final SQLite checkpoint remain in place.
- API requests had no explicit overall timeout. Each attempt is now limited to 30 seconds; existing retry rules remain unchanged.

## Local measurements

The ignored `profile_task_actions_with_many_campaigns` test creates a temporary database with 20 campaigns and 2,000 tasks, then exercises the actual task-creation and completion handlers. No live user data or cloud services are used. The fixture has no unfinished milestones and uses a debug test build.

| Operation | Before | After |
| --- | ---: | ---: |
| Full data reload | 243 ms | 14 ms |
| Create task | 244 ms | 15 ms |
| Complete task | 1,225 ms | 62 ms |
| Local-only quit | 0.87 ms | 0.81 ms |

These single-run measurements identify the local bottleneck; they are not a guarantee for every profile. Cloud exit still depends on the required sync and network availability. A sync already in progress may finish its current work before exiting.

Repeat the profile with:

```sh
cargo test --lib profile_task_actions_with_many_campaigns -- --ignored --nocapture
```

Regression coverage includes milestone counts, task completion and rewards, creation, private/shared Treasury sync, and graceful-quit success/failure paths.
