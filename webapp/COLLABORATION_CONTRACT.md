# Questline Collaboration Contract v1

This contract records the Phase One wire format shared by the CLI and web app.
All entities below are project-scoped and must use `project-v1` encryption with
the active opaque Fellowship `routing_id`.

## Task assignment

- Entity type: `task_assignment`
- Entity ID: `<task_id>__<lowercase_user_identity>`
- Add operation: `upsert` (the CLI's legacy `assign` operation is also accepted)
- Remove operation: `delete`

Payload:

```json
{
  "task_id": "uuid",
  "project_id": "uuid",
  "user_identity": "64-character public identity key",
  "user_username": "Display name",
  "assigned_by_identity": "64-character public identity key",
  "assigned_by_username": "Display name",
  "assigned_at": "RFC 3339 timestamp"
}
```

The first four properties are canonical and compatible with the current CLI.
Actor and timestamp properties are optional for older clients. One assignment
entity represents one task/member edge, allowing multiple companions to be
assigned without whole-task last-write-wins conflicts.

## Quest status

- Entity type: `task_status`
- Entity ID: `<task_id>`
- Conflict rule: newest `updated_at` wins
- Stable values: `Backlog`, `Ready`, `InProgress`, `Blocked`, `Review`, `Done`

`Done` is compatibility-derived from the legacy task `completed` flag. The
separate entity stores active stances; Space remains the canonical completion
and XP action.

## Quest Council comment and mention

- Entity type: `task_comment`
- Entity ID: comment UUID
- Conflict rule: newest `updated_at` wins per comment
- Mutable only by the original `author_identity`
- The decrypted author identity must equal the durable event signer

The payload contains task and project IDs, author identity/name, plain-text
content, stable `mentioned_identities`, timestamps, and edited/deleted state.
Withdrawals retain a tombstone so they cannot reappear after synchronization.

## Quest dependency (contract v2)

- Entity type: `task_dependency`
- Entity ID: `<task_id>__<depends_on_task_id>`
- Add operation: `upsert`
- Remove operation: `delete`
- Scope: two parent Quests in the same Campaign
- Conflict rule: independent set membership per compound edge

Payload:

```json
{
  "task_id": "uuid",
  "depends_on_task_id": "uuid",
  "project_id": "uuid",
  "created_by_identity": "64-character Companion Key",
  "created_by_username": "Display name",
  "created_at": "RFC 3339 timestamp"
}
```

Clients reject self-dependencies, cross-Campaign edges, Trial dependencies, and
edges that would create a direct or transitive cycle. A Quest is dependency-
blocked while any referenced blocker remains incomplete. This derived state
does not overwrite its manually selected Quest Stance, preserving old-client
compatibility and the existing status conflict rule.

## Recipient notification

Assignment notifications are derived client-side after decrypting an incoming
assignment. They are not uploaded as plaintext server notifications.

- ID: `fellowship:<source_type>:<durable_sync_event_id>`
- Type: `task_assignment`
- Target: assigned task ID and project ID
- Dedupe: insert by deterministic ID

The encrypted sync event ID is stable across server fan-out, retries, and the
recipient's devices. Notification text may include decrypted task data only in
the local cache/UI. Read state uses an account-encrypted `notification_state`
entity containing only the opaque deterministic ID, boolean state, and
`updated_at`; decrypted notice content is never uploaded.

## Permission matrix for this slice

| Action | Owner | Steward | Companion | Observer |
|---|---:|---:|---:|---:|
| View assignments | Yes | Yes | Yes | Yes |
| Assign/unassign | Yes | Yes | No | No |
| Receive assignment | Yes | Yes | Yes | No |
| Change Quest stance | Yes | Yes | Assigned only | No |
| Quest Council comment | Yes | Yes | Yes | No |

The server enforces Owner/Steward-only `task_assignment` and `project_member`
events using visible signed event metadata without decrypting their payloads.
Companions may write ordinary work entities; Observers may only write Chronicle
messages. Clients additionally enforce assigned-Companion stance changes.

## Treasury permission matrix

| Action | Owner | Steward | Companion | Observer |
|---|---:|---:|---:|---:|
| View treasury | Yes | Yes | Yes | Yes |
| Record entry | Yes | Yes | Yes | No |
| Edit/delete own Planned entry | Yes | Yes | Yes | No |
| Edit/delete any entry | Yes | Yes | No | No |
| Approve entry | Yes | Yes | No | No |
| Settle payment | Yes | Yes | No | No |
| Set overall/category budgets | Yes | Yes | No | No |
| Manage ledger categories | Yes | Yes | No | No |
| Switch campaign currency | Yes | No | No | No |
| Set Quest estimated/actual cost | Yes | Yes | Yes | No |
| Set Quest billable amount and payment status | Yes | Yes | No | No |

Separation of duties: whoever records a cost does not approve or settle it. A
Companion owns a `ledger_entry` only while it is `Planned`; once it is approved,
paid, or cancelled, only the Owner or a Steward may change it. Authorship lives
in `ledger_entries.created_by_identity` (the author's Companion Key), is set at
creation, and is immutable on update — editing another member's entry never
transfers ownership. Events from clients predating this field arrive without it
and are treated as unknown authorship, so only an Owner or Steward may alter
them. The `status` field in the entry form is not a bypass: persisting a
non-`Planned` status requires the approve/settle right.

The server cannot decrypt entry payloads, so it enforces only what the envelope
metadata exposes — an Observer never writes to a shared route. The rest of this
matrix is enforced by clients, and the CLI additionally refuses to *queue* a
write it knows the server would reject, because a 403 rolls back the whole push
batch and would stall that account's entire sync.

## Compatibility

- Existing CLI clients already read and write the four canonical assignment
  fields and compound entity ID.
- Unknown optional fields must be ignored.
- Deletes are resolved from the compound entity ID and do not require payload
  decryption by legacy clients.
- Assignment changes do not grant XP.

## Offline delivery

Signed encrypted Fellowship events are placed in the web IndexedDB project
outbox before upload. Successful upload removes the event. Background sync
retries queued events in insertion order before pulling remote changes. Event
IDs remain unchanged so server insertion and notification creation are
idempotent.
