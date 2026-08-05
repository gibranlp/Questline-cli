# Questline Teamworking Plan

## Development strategy decision: CLI first, web app on standby

**Decision recorded 2026-08-01:** Questline will complete and polish the CLI
teamworking experience before continuing feature development in the web app.

The CLI is the primary product, the clearest expression of Questline's identity,
and the best place to settle collaboration behavior. Building the same evolving
feature simultaneously in Rust and Svelte would divide attention, duplicate UI
work, and make compatibility decisions prematurely. The collaboration model
should first become complete, enjoyable, secure, and stable in the CLI; the web
app can then implement a proven contract instead of co-designing it in parallel.

### What “web app on standby” means

- Preserve the current web Phase One prototype and its documentation.
- Do not remove the assignment store, encrypted outbox, My Quests card,
  assignment UI, notification prototype, or collaboration contract.
- Do not add new web collaboration features until the CLI release gates below
  are satisfied.
- Web maintenance may continue only for critical security issues, data-loss
  defects, broken builds, or compatibility required by a CLI schema change.
- Every new CLI collaboration entity must still have a documented, versioned,
  backward-compatible sync contract so future web work remains practical.
- Avoid designing CLI behavior around current web UI limitations.

### Conditions for resuming web development

Resume the web roadmap after the CLI can demonstrate all of the following with
two real Fellowship members:

1. Companion Key invitation, acceptance, removal, and key rotation are reliable.
2. Roles and permissions are consistently enforced.
3. Quest assignment and unassignment work across devices and offline recovery.
4. Quest statuses, comments, mentions, and notifications have stable contracts.
5. My Quests and the team Campaign workflow are useful in daily operation.
6. Mixed-version sync behavior is tested and migrations are documented.
7. The CLI team workflow has completed a focused usability and security pass.

When work resumes, the web app should pursue feature parity in vertical slices,
using the CLI contracts as the source of truth.

### Current web prototype status

The initial web assignment slice remains useful as a compatibility experiment,
but it is not the active Phase One implementation target. Its outstanding live
two-account test, mobile visual QA, full notification inbox, exact task deep
links, and stronger assignment authorization are deferred until web work resumes.

From this point forward, references to **Phase One** in the active roadmap mean
the **CLI Phase One** unless a section explicitly says “web app.”

### CLI Phase One progress

Companion Key experience completed on 2026-08-01:

- [x] Adopted **Companion Key** as the CLI product term for the public identity
  exchanged during Fellowship invitations.
- [x] Kept the existing 64-character public identity and encrypted invitation
  protocol unchanged.
- [x] Added reusable normalization and validation for plain, spaced, dashed,
  colon-separated, and uppercase key representations.
- [x] Rejects malformed keys, incorrect lengths, and self-invitations before
  project sharing or encrypted invitation work begins.
- [x] Formats keys in readable eight-character groups.
- [x] Shows a deterministic short fingerprint for out-of-band verification in
  Sync Settings and the invitation modal.
- [x] Explicitly distinguishes the public Companion Key from the private
  Transfer Code.
- [x] Updated clipboard language from “Share Key” to “Companion Key.”
- [x] Added focused normalization, formatting, fingerprint, paste, and invalid
  input tests.
- [ ] Add QR display/scanning later as a progressive enhancement; it is not
  required for the keyboard-first CLI invitation release.

My Quests assignment view completed on 2026-08-01:

- [x] Added a Fellowship **My Quests** tab using the existing encrypted
  `task_assignment` edges.
- [x] Added the `[y]` keyboard shortcut, selection controls, and Campaign open
  action.
- [x] Orders active work before completed work, then by priority and due date.
- [x] Shows Campaign, priority, due date, and legacy completion state without
  changing the established task sync payload.
- [x] Keeps multiple assignments independent and compatible with older clients.
- [x] Added the separate backward-compatible `task_status` entity without
  changing the legacy Quest payload.
- [x] Fellowship My Quests opens the exact Quest Ledger row, clearing stale
  filters and searches while preserving the user's chosen sort order.

Quest Stances completed on 2026-08-01:

- [x] Added six readable, Questline-native stances: **Awaiting the Council**,
  **Ready for Adventure**, **Quest Underway**, **Path Obstructed**, **Awaiting
  Judgment**, and **Conquered**.
- [x] Added `[g] Quest Stance` to the Campaign Quest Ledger and displayed the
  stance in both the Ledger and Fellowship My Quests view.
- [x] Kept completion backward compatible: legacy `completed` remains the
  canonical source of **Conquered**, while the new encrypted entity stores the
  five active stances independently.
- [x] Added last-write-wins encrypted sync, full-state recovery, revocation
  cleanup, actor identity, and timestamps for Quest Stances.
- [x] Restricted stance decrees to assigned Companions, Owners, and Stewards in
  the CLI; Observers remain read-only at the server boundary.
- [x] Added activity events and deduplicated **Council decree** notifications
  for remote stance changes affecting a Companion's assigned Quest.
- [x] Preserved the established XP rule: changing a stance grants no XP and
  Space remains the canonical completion action.

Council Notices and exact navigation completed on 2026-08-01:

- [x] Added a permanent `[b] Council` tab to Fellowship; notices no longer
  disappear after the first shared Campaign is created.
- [x] Added unread count, keyboard selection, read/unread toggle, and mark-all
  read controls using Questline-native Council language.
- [x] Made Quest-targeted notices open the exact Quest Ledger row and mark the
  notice read in one action.
- [x] Corrected My Quests keyboard behavior so arrows select and Enter opens;
  navigation no longer opens a Campaign accidentally.
- [x] Synchronize Council Notice read state across a Companion's devices using
  an account-encrypted opaque `notification_state` entity; decrypted notice
  content remains local.
- [x] Added All, Unread, and Mentions filters plus Quest unassignment,
  Chronicle mention, due-soon, and overdue notice types.

Quest Ledger teamwork filters completed on 2026-08-01:

- [x] Expanded `[f]` filtering with **MyQuests**, **Unassigned**, **Blocked**,
  **HighPriority**, and **DueSoon**, alongside All, Incomplete, and Completed.
- [x] Kept the renderer and keyboard selection on the same filtered ordering so
  actions always affect the Quest visibly selected by the Companion.
- [x] All filters operate from the offline local Fellowship cache.

Quest Council discussion foundation completed on 2026-08-01:

- [x] Added `task_comment` as a first-class, project-scoped encrypted entity;
  concurrent Council messages never overwrite the Quest or Chronicle chat.
- [x] Added offline creation, durable outbox sync, last-write-wins reconciliation,
  full-state recovery, deletion handling, and revoked-route cleanup.
- [x] Added `[c] convene` in the Quest Ledger with a keyboard-first Council
  composer and the three latest messages visible in Quest details.
- [x] Resolve valid `@username` references against active Campaign members and
  store their stable Companion Keys alongside the rendered message.
- [x] Added deduplicated **The Council calls your name** notices that open the
  exact mentioned Quest.
- [x] Defined Observer behavior as read-only for Quest Councils and enforced it
  both in the CLI and through the server's project-entity authorization list.
- [x] Added a migration/regression test proving stable mention identities survive
  local persistence; the complete library now passes 96 tests.
- [x] Added a full scrollable Council history, explicit Tab-selected Companion
  mentions, and author-only revise/withdraw controls with visible revised and
  withdrawn states.
- [x] Enforced authorship in both local mutations and encrypted sync: the
  decrypted author identity must match the durable event-signing Companion Key.

## Product direction

Questline will grow from a personal productivity RPG with sharing into an
offline-first, end-to-end encrypted workspace for small creative and technical
teams.

The product promise is:

> Plan real work, complete quests together, and advance a shared story without
> surrendering ownership of your data.

The RPG identity is a product advantage, not decoration. Team features should
feel native to Questline: projects remain Campaigns, tasks remain Quests, the
team remains a Fellowship, and useful team summaries become Council Briefings.
The fantasy language must never make ordinary actions difficult to understand.

## Foundational decision: shared-key invitations stay

Invitation by shared public identity key is part of Questline's foundation and
will remain the canonical trust ceremony.

We will improve its usability without silently replacing it with email-based
identity or weakening end-to-end encryption:

- A companion still joins by exchanging a public identity key.
- Private project data and project keys remain encrypted on the client.
- The invitation flow must clearly show what key is being shared and why.
- Add copy, paste, validation, QR display/scanning, and an optional formatted
  fingerprint to make key exchange easier.
- Never upload a private key or Transfer Code as part of an invitation.
- Preserve key rotation when a companion is removed.
- Any future invitation link may transport only public/opaque invitation data;
  it must not become a passwordless bypass around the shared-key ceremony.

Suggested presentation:

- Product term: **Companion Key**
- Technical help text: **Your public Questline identity key**
- Display: grouped characters with a short fingerprint for verification
- Primary actions: **Copy My Key**, **Paste Companion Key**, **Show QR**

## Existing foundations to reuse

Before adding schema, reuse and normalize what is already present:

- Tasks already contain `owner_identity` and `owner_username`.
- The local database already contains `task_assignments`.
- Notifications, activity logs, presence, project roles, Chronicle messages,
  message reactions, and encrypted Fellowship routing already exist.
- The CLI already renders task assignees, project activity, member presence,
  recurrence, search, and a task calendar.
- The web app already supports encrypted project sharing, invitations, project
  chat, membership removal, and project-key rotation.

Phase One should close the gap between CLI, sync, and web behavior before
creating overlapping concepts.

## Principles for implementation

1. **Encryption first.** New collaborative entities must use the existing
   project-scoped encrypted sync route.
2. **Offline first.** Mutations update the local store immediately, queue a sync
   event, and reconcile later. A network outage must not make basic task work
   unusable.
3. **Backward compatible.** Older clients must safely ignore new optional
   fields or entity types.
4. **One source of truth.** CLI and web must agree on entity names, status
   values, role permissions, timestamps, and conflict rules.
5. **Clear language.** Show the Questline term and the familiar work term where
   ambiguity is possible, for example `Blocked quest · task is waiting`.
6. **Keyboard usable.** All important Phase One flows must work without a
   pointer and have visible focus states.
7. **Accessible and responsive.** Team workflows must work at 360 px width,
   with reduced motion and sufficient contrast.
8. **No destructive migration.** New fields start nullable/defaulted and are
   backfilled only when behavior is verified.

# Phase One: Team Usability

## Current progress

First vertical slice started on 2026-08-01:

- [x] Reused the CLI `task_assignment` compound entity contract.
- [x] Added assignment state and IndexedDB persistence to the web app.
- [x] Added encrypted assignment/unassignment from shared Campaign Quest rows.
- [x] Added multiple-assignee identity chips and an Owner/Steward picker.
- [x] Added a cross-Campaign My Quests dashboard card.
- [x] Added deterministic recipient assignment notifications with Campaign
  navigation.
- [x] Added a durable encrypted-project outbox and background retry.
- [x] Added contract tests and a production web build check.
- [ ] Exercise the flow with two real accounts/devices against the API.
- [ ] Complete visual and 360 px browser QA (browser surface unavailable during
  the initial implementation session).
- [ ] Add a dedicated full notification inbox and exact task deep-link.
- [ ] Complete server-enforced Owner/Steward assignment authorization; see the
  opaque-payload limitation in `webapp/COLLABORATION_CONTRACT.md`.

## Goal

A Fellowship can share a Campaign, understand responsibility, discuss work in
context, notice changes that require action, and use the web app on a phone or
small screen—while retaining shared-key invitations and E2EE.

## Definition of done

Phase One is complete when two users on separate devices can:

1. Exchange and verify Companion Keys.
2. Share and accept an encrypted Campaign invitation.
3. Assign a Quest to one or more Campaign members.
4. Move it through a small, consistent set of statuses.
5. Discuss the Quest and mention another member.
6. Receive an actionable notification for assignment or mention.
7. Open a **My Quests** view containing their assigned work.
8. Complete the workflow from desktop or a 360 px-wide mobile viewport.
9. Go offline during the workflow and sync successfully after reconnecting.
10. Remove a member and verify that project-key rotation still prevents that
    member from receiving future project changes.

## Phase 1.0 — Contract and security baseline

Complete this before changing UI behavior.

### Work

- Document the canonical payloads for:
  - project member
  - task assignment
  - task status
  - task comment
  - mention
  - notification
- Decide whether `task_assignments` becomes a first-class sync entity or is
  represented as an array on each task. Prefer a first-class entity if multiple
  assignees are required; it avoids competing whole-task edits.
- Define stable task status values:
  - `Backlog`
  - `Ready`
  - `InProgress`
  - `Blocked`
  - `Review`
  - `Done`
- Treat the existing `completed` field as compatibility state:
  - `status = Done` implies `completed = true`.
  - every other status implies `completed = false`.
  - legacy completed tasks read as `Done`.
  - legacy incomplete tasks initially read as `Backlog`.
- Define permissions centrally:
  - Owner: membership, roles, project and work administration
  - Steward: create/edit/assign work and invite if policy permits
  - Companion: create/edit/comment/complete work
  - Observer: read and comment only, if commenting is explicitly allowed
- Verify that all new project collaboration payloads are encrypted before
  upload and routed using the current opaque project route.
- Write migration and mixed-version tests before rollout.

### Acceptance criteria

- A written entity contract exists in code or adjacent developer documentation.
- CLI, web, and server use identical status and role values.
- Invalid role operations are rejected server-side, not only hidden in UI.
- Security tests cover removed members, stale routes, replayed events, and
  unauthorized assignment/comment writes.

## Phase 1.1 — Companion Key invitation experience

### Work

- Rename the visible `64 hex identity` wording to `Companion Key`.
- Add a short explanation that the key is public and safe to share; explicitly
  distinguish it from the private Transfer Code.
- Add one-click copy for the current user's key.
- Accept pasted keys containing spaces or groups, normalize locally, and
  validate exact encoding/length before submission.
- Display a grouped, readable key and a short fingerprint.
- Add a confirmation step showing:
  - Campaign
  - invited Companion Key fingerprint
  - selected role
  - permissions granted by that role
- Add QR display and scanning as a progressive enhancement. The QR contains the
  public Companion Key only.
- Improve invitation pending, accepted, expired, revoked, and failed states.
- Preserve member removal and project-key rotation behavior.

### Acceptance criteria

- A user never needs to manually count or clean key characters.
- Malformed keys fail locally with a useful message.
- The UI never calls a public key a password, secret, or Transfer Code.
- Users can compare a short fingerprint out-of-band.
- Existing invitations remain compatible.
- E2EE and rotation security tests continue passing.

## Phase 1.2 — Assignees and Quest status

### Work

- Add `status` to the task contract with backward-compatible defaults.
- Sync task assignments as project-scoped encrypted entities.
- Add an assignee picker containing only active Campaign members.
- Show compact Companion identity chips on Quest rows.
- Add status control to task creation and editing.
- Add filters:
  - My Quests
  - Unassigned
  - Status
  - Priority
  - Due date
- Add a top-level **My Quests** view or dashboard section across Campaigns.
- When a Quest is assigned, reassigned, blocked, unblocked, or completed:
  - create an activity event
  - create relevant notifications
  - preserve the actor identity and timestamp
- Decide XP attribution explicitly. Recommended Phase One rule:
  - completion XP goes to the user who completes the Quest
  - assignment alone grants no XP
  - repeated close/reopen does not duplicate XP

### Acceptance criteria

- Multiple assignees can be added and removed without overwriting unrelated
  task edits.
- Removed Campaign members disappear from future assignment choices while
  historical attribution remains readable.
- Legacy clients can still mark a Quest complete.
- Status and `completed` cannot drift into contradictory states.
- My Quests works offline from local encrypted/cache data.

## Phase 1.3 — Contextual Quest discussion and mentions

### Work

- Add a task detail drawer/page rather than expanding every capability inline.
- Include description, status, priority, due date, assignees, steps, activity,
  and discussion in that view.
- Implement task comments as a project-scoped encrypted entity. Do not overload
  general Chronicle chat unless the data contract cleanly supports a `task_id`.
- Support plain text first; add limited Markdown only if it can be rendered
  safely and consistently.
- Implement `@mention` selection from active project members rather than parsing
  arbitrary display text alone.
- Store stable mentioned identities in addition to rendered usernames.
- Permit comment edit/delete only by the author; show edited/deleted state.
- Link task activity into the Campaign Chronicle without duplicating content.

### Acceptance criteria

- Comments remain encrypted at rest on the server.
- Mentioning renamed users still targets the correct identity.
- Observer commenting behavior matches the defined permission matrix.
- Deleted comments do not reappear after sync.
- Concurrent comments do not overwrite one another.

## Phase 1.4 — Actionable notification inbox

### Work

- Add an inbox entry to web navigation with an unread badge.
- Support these initial notification types:
  - Fellowship invitation
  - Quest assigned
  - Quest unassigned
  - Mention in Quest comment
  - Mention in Chronicle message
  - Quest blocked/unblocked
  - Due soon/overdue
- Every notification should deep-link to the relevant Campaign, Quest, comment,
  or invitation.
- Add mark read, mark unread, mark all read, and filters for unread/mentions.
- Deduplicate notifications using a deterministic event/source identifier.
- Keep OS/browser notifications opt-in and secondary to the in-app inbox.
- Never place decrypted private task/comment content in a server-generated push
  payload. Use a generic prompt and decrypt after opening the app.

### Acceptance criteria

- An assignment and mention appear once, on all of the recipient's devices.
- Opening the notification goes to the exact relevant work item.
- Read state synchronizes without deleting notification history.
- Offline-generated actions create notifications after sync without duplicates.
- Push/OS previews do not leak encrypted project content.

## Phase 1.5 — Responsive team workspace

### Work

- Replace the permanently visible 300 px sidebar on small screens with a
  keyboard-accessible navigation drawer.
- Define breakpoints for:
  - mobile: 360–767 px
  - tablet: 768–1023 px
  - desktop: 1024 px and above
- Convert the three-column Fellowship layout into tabs or stacked panels on
  mobile.
- Make project tabs horizontally scrollable or replace them with a compact
  selector on small screens.
- Ensure modals become contained mobile sheets and do not exceed viewport
  height.
- Use at least 44 px touch targets for primary interactive controls.
- Add visible focus states, semantic labels, focus trapping, Escape handling,
  and reduced-motion behavior.
- Test long Campaign names, usernames, Companion Keys, comments, and translated
  date formats for overflow.

### Acceptance criteria

- No horizontal page overflow at 360 px.
- Navigation, invitation, assignment, commenting, notifications, and Quest
  completion work using keyboard only.
- The same workflows work with touch at 360 px and 768 px.
- Automated accessibility checks have no critical violations.
- Meaning is not communicated by color alone.

## Phase 1.6 — Stabilization and release

### Work

- Add unit tests for normalization, status compatibility, mention resolution,
  permissions, and notification deduplication.
- Add sync tests for two users, two devices per user, offline edits, conflicts,
  removal, and key rotation.
- Add browser workflow tests for the complete Definition of Done.
- Test mixed versions: new web with old CLI and new CLI with old web.
- Add a small opt-in onboarding checklist for the first shared Campaign.
- Update the README, website screenshots, manual, privacy/security explanation,
  and changelog.
- Roll out behind a collaboration schema/capability version if mixed-client
  behavior cannot be guaranteed.

### Release gates

- No known plaintext leak of project-scoped content.
- No known authorization bypass for Observer/Companion/Steward roles.
- No duplicated XP or notifications in retry/offline tests.
- No loss of legacy task completion or invitation data.
- Key removal/rotation security suite passes.
- Mobile Definition of Done passes on Chromium, Firefox, and WebKit-equivalent
  coverage where available.

## CLI Phase One release-gate status — 2026-08-01

Completed locally and against the configured live API:

- [x] 98 Rust tests, including status compatibility, stable mentions,
  author-only Council mutations, deterministic notices, encrypted notice-state
  application, revocation, and key rotation.
- [x] 10 browser cryptographic/contract tests and production web build retained
  as compatibility guards while web product work remains on standby.
- [x] Live ephemeral-account checks for durable signatures, replay rejection,
  ciphertext/identity forgery, duplicate events, invitation concurrency,
  removal, stale-route rejection, rotation, and post-removal isolation.
- [x] README, website manual, changelog, collaboration contract, first-Campaign
  Field Guide notice, and release checklist updated.

Final release verification:

- [x] Deployed the updated API authorization rule restricting
  `task_assignment` and `project_member` events to Owners and Stewards.
- [x] Re-ran `scripts/test_sync_v2_security.mjs` after deployment: Companion
  work writes passed, Companion assignment administration returned 403, Owner
  assignment administration passed, and the complete rotation suite remained
  green.
- [ ] Complete one human two-device keyboard walkthrough. Automated ephemeral
  accounts cover the transport/security flow but cannot judge terminal focus,
  wording, or operator comprehension.
- [ ] PHP syntax/fixture/retention scripts require a PHP runtime and isolated
  database credentials; PHP is not installed in the current workspace runtime.

The CLI Phase One engineering implementation and automated release gates are
complete. The remaining human walkthrough and PHP infrastructure audits are
release-operations evidence, not unfinished product behavior. Web
mobile/responsive work remains explicitly deferred until the CLI release is
accepted.

# Proposed implementation sequence

Use small vertical slices rather than building all backend work and all UI work
separately.

1. **Contracts and tests:** status, assignment, comment, mention, notification,
   permission matrix, and encrypted routing.
2. **Companion Key UX:** terminology, formatting, validation, copy, fingerprint,
   confirmation; QR can follow after the base flow is stable.
3. **Single-assignment slice:** assign one member, sync, render, notify, and open
   from My Quests.
4. **Multiple-assignment hardening:** first-class assignment events and
   concurrent edits.
5. **Status slice:** status editing, compatibility with `completed`, filters,
   and activity events.
6. **Comment slice:** task detail, encrypted comments, then identity-based
   mentions.
7. **Inbox slice:** assignment and mention notifications first; other types
   follow the same contract.
8. **Responsive pass:** implement continuously, then run a dedicated mobile and
   accessibility hardening pass.
9. **Mixed-client and security release gate.**

# First development milestone

The first milestone is intentionally narrow and demonstrable:

> Two existing Fellowship members can assign a shared Quest to one Companion;
> the assignment syncs encrypted, appears in My Quests, and produces one
> actionable notification.

Tasks:

- Audit current sync serialization for `task_assignments`, notifications, and
  activity logs.
- Write the collaboration entity contract and role matrix.
- Choose the canonical assignment representation.
- Add failing round-trip and permission tests.
- Implement assignment sync and local-store application.
- Add the assignee selector and identity chip to the web workspace.
- Add the first My Quests dashboard section.
- Emit/deduplicate the assignment notification.
- Verify offline creation, retry, multi-device receipt, and member removal.

This milestone should be completed before Kanban, calendar, dependencies,
integrations, or additional progression systems.

# Phase Two preview: Daily team workflow

Phase Two begins only after Phase One release gates pass:

- [x] List and calendar views already available in the CLI.
- [x] Added the first keyboard-first Kanban slice, grouped by the six canonical
  Quest Stances and backed entirely by the existing offline encrypted status
  contract.
- [x] Hardened Kanban navigation: Left/Right move between non-empty stance
  columns, Up/Down move within a stance, and the selected Quest survives
  Ledger/Board toggles.
- [x] Defined the additive encrypted `task_dependency` edge contract with
  same-Campaign validation, parent-Quest validation, self/cycle rejection,
  independent add/remove sync events, and derived unresolved-blocker state.
- [x] Added a keyboard dependency manager (`l`, arrows, Space), unresolved
  blocker details in the Ledger, and blocker markers on Kanban cards; Observers
  may inspect links but cannot mutate them.
- [x] Added deterministic **The path has opened** Council Notices for assigned
  dependent Quests and Campaign activity when a blocker is completed.
- [x] Added the first offline Council Briefing dashboard with actionable
  blocked, review, overdue, due-soon, and unassigned queues, Fellowship workload,
  recent Campaign activity, and exact filtered-Ledger navigation.
- [x] Embedded live/recent Companion presence in the Council Briefing, including
  role, open workload, current-Campaign presence, human-readable freshness, and
  navigation to that Companion's assigned Quests.
- [x] Added context-aware command-palette actions for Council Briefing, Kanban,
  My Quests, blocked work, review queues, and the selected Quest Council, all
  routed through existing exact-navigation and permission logic.
- [x] Expanded local global search across active Quest Council messages,
  Campaign Chronicle messages, and stable Companion identities, excluding
  withdrawn Council content and opening the exact discussion or workload.
- [x] Added workload and review visibility: AVAILABLE/BALANCED/OVERLOADED bands,
  open/high/blocked/overdue counts per Companion, an explicit Awaiting Judgment
  panel, assignee visibility, Owner/Steward reviewer guidance, and unassigned
  review warnings that do not rely on color alone.

## Phase Two stabilization

- [x] Added cross-device regression coverage proving a remote blocker
  completion produces one deterministic dependency-resolution Council Notice.
- [x] Added narrow-terminal render coverage for Kanban and Council Briefing.
- [x] Extended the live sync security fixture to require Companion
  `task_dependency` writes while retaining Owner/Steward-only administration.
- [x] Deployed the server's `task_dependency` project-scope allowlist and ran
  the updated live two-account security fixture: Companion dependency writes
  passed, Companion assignment administration remained 403, Owner assignment
  administration passed, and removal/rotation/stale-route isolation stayed green.
- [ ] Complete a human keyboard walkthrough of Kanban, dependency management,
  Council Briefing, teamwork search, and exact navigation.

# Phase Three: Adoption and growth

Phase Three started on 2026-08-03. Adoption features remain CLI-first and must
preserve the offline, encrypted collaboration contracts established in Phases
One and Two.

## Phase 3.1 — Campaign templates and imports

- [x] Defined a reusable Campaign blueprint contract containing only Campaign,
  Quest, priority, description, and step structure—never identity, assignment,
  completion, Fellowship, or encryption-key state.
- [x] Added keyboard-first built-in template selection from Campaigns with
  preview, explicit focus, cancel, and create actions.
- [x] Added Software Release, Content Sprint, and Event Launch starter
  blueprints using clear work language within Questline's Campaign vocabulary.
- [x] Instantiate the Campaign, Quests, steps, revisions, and sync outbox events
  in one database transaction so a failed template cannot leave partial work.
- [x] Create templates as private Campaigns owned by the current profile; users
  may edit or explicitly share them through the established Fellowship flow.
- [x] Avoid duplicate-name confusion by suffixing repeated template Campaigns.
- [x] Defined a versioned portable JSON blueprint format using the same validated
  identity-free contract.
- [x] Added safe file import with size/depth/count limits, schema validation,
  unknown-version rejection, and an exact pre-creation preview.
- [x] Added template export for one local Campaign while excluding private notes,
  comments, assignments, activity, completion history, and encryption state.
- [x] Added malformed, oversized, unknown-field/version, control-character,
  privacy-retention, reset-state, parser, and partial-write regression coverage
  before enabling third-party templates.

## Phase 3.2 — Privacy-safe calendar import

- [x] Added local `.ics` import without calendar credentials, remote API calls,
  telemetry, background polling, or uploaded source files.
- [x] Added preview-first CLI behavior; no Quest is created until the user
  repeats the command with `--confirm`.
- [x] Show the exact destination, resulting UTC due times, new/duplicate counts,
  and every Quest title before import. Shared Campaign destinations display an
  explicit Fellowship-sync warning.
- [x] Parse a bounded iCalendar VEVENT subset with folded lines, escaped text,
  date/date-time values, cancellation capture, required UID/SUMMARY/date,
  and strict file/event/text limits.
- [x] Derive stable per-Campaign Quest IDs from event UIDs so repeated imports
  are idempotent across devices without storing calendar credentials or source
  metadata in Quest descriptions.
- [x] Insert all new calendar Quests and sync outbox events atomically as normal
  medium-priority Quests with no completion, XP, recurrence, or assignment state.
- [x] Added IANA TZID timezone resolution with daylight-saving rules. Unknown
  zones and ambiguous/nonexistent local times fail validation instead of being
  silently guessed; floating times without TZID remain explicitly previewed as
  UTC.
- [x] Added opt-in `--reconcile` behavior for existing event IDs. It updates
  only title, description, and due time, preserves priority/completion/XP and
  assignment state, and marks cancellations visibly without deleting or
  completing the Quest. Plain `--confirm` remains create-only.
- [x] Completed the calendar token-storage threat-model review in
  `CALENDAR_SUBSCRIPTION_THREAT_MODEL.md`. It requires an OS credential vault,
  PKCE, provider-fixed HTTPS endpoints, read-only scopes, manual preview-first
  refresh, redacted diagnostics, and explicit export/sync isolation. The review
  found that SQLite settings and ordinary config files are unsafe for tokens
  because full exports and database backups preserve their contents.
- [x] Added a cross-platform OS credential-vault boundary backed by macOS
  Keychain, Windows Credential Manager, and Unix Secret Service. It has no
  plaintext fallback, uses stable opaque references scoped by profile/provider/
  account/credential kind, keeps access and refresh tokens separate, wipes
  retrieved byte buffers on drop, makes deletion idempotent, and redacts native
  vault diagnostics.
- [ ] Prove provider tokens, private feed URLs, authorization codes, and PKCE
  material cannot enter SQLite, config, logs, exports, backups, or sync payloads.
- [ ] Add authenticated, least-privilege calendar subscriptions only after the
  credential-vault and export-isolation gate passes.

## Later Phase Three slices

- GitHub/GitLab and calendar integrations
- Shareable, opt-in adventure recaps
- Guided team onboarding
- Privacy-respecting activation and retention measurement
- Sustainable team subscription model

# Phase Four preview: Visual polish

- Unified icon and component system
- More tactile Quest and milestone interactions
- Professional/low-fantasy density option
- Animation, accessibility, and keyboard refinement
- Updated product screenshots and invitation landing experience

# Decisions still to record during Phase 1.0

These should be resolved from security and existing behavior, not left as
implicit UI choices:

- Can Stewards invite or only Owners?
- Can Observers comment, or are they strictly read-only?
- Are multiple assignees required in the first release or enabled immediately
  after a single-assignee vertical slice?
- Does a blocked Quest require a reason?
- Who may change assignment and status?
- How are comments retained after a member is removed?
- Which collaborative entities contribute to Living Chapters, if any?
- What conflict rule applies to status changes made offline by two users?

The default recommendation is: Owners and Stewards may assign; Companions may
change the status of Quests assigned to them; Observers are read-only; comments
remain as immutable historical attribution after member removal; and assignment
or chat activity does not grant XP.
