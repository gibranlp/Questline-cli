# Questline CLI teamwork walkthrough

Use two real Questline profiles on separate devices or isolated terminal
profiles. Record the date, operators, client versions, and result of every
section. Do not mark the release gate complete until every critical flow passes
or has a documented disposition.

For same-device testing, open two terminals with different testing-only profile
names:

```sh
cargo run -- --profile owner
cargo run -- --profile companion
```

With an installed binary, use `questline --profile owner` and
`questline --profile companion`. Profile names are case-insensitive and may use
letters, numbers, `-`, and `_`. Each profile has an independent database,
identity key, and configuration beneath Questline's `profiles` storage folder.
Launching Questline without `--profile` continues to use the normal existing
data and never selects one of these test profiles.

## Run information

- Date: August 2
- Operator: Gibranlp
- Device A / account / version: 1.2.0
- Device B / account / version: 1.2.0
- API environment: v2
- Terminal sizes: 1920x1080

## 1. Companion trust ceremony

- [x] Both users can find their public Companion Key in Sync Settings.
- [x] Grouping and fingerprint are readable and clearly distinguished from the
      private Transfer Code.
- [x] Device A pastes Device B's grouped/uppercase key without manual cleanup.
- [x] A malformed key and self-invitation fail locally with useful wording.
- [x] The confirmation identifies the Campaign, role, fingerprint, and granted
      permissions.
- [x] Device B receives and accepts the invitation.

Notes:

## 2. Responsibility and offline recovery

- [x] Owner/Steward assigns two bearers to a Quest with `a`.
- [x] Device B sees the assignment after sync and can open the exact Quest from
      Fellowship My Quests with `y`, arrows, and Enter.
- [x] A Companion cannot administer assignment; an Owner/Steward can.
- [x] Device B goes offline, changes an assigned Quest stance with `g`, then
      reconnects and syncs exactly once.
- [x] Unassignment reaches Device B and produces one actionable Council Notice.

Notes:

## 3. Quest Council and notices

- [x] Device A opens the selected Quest Council with `c`, explicitly selects an
      `@mention`, and posts while offline.
- [x] Device B receives exactly one mention notice after Device A reconnects.
- [x] Enter on the notice opens the exact Quest and marks the notice read.
- [x] Read/unread state reaches the recipient's other device when available.
- [x] The author can revise with `Ctrl+E` and withdraw with `Ctrl+D`; the other
      member cannot.
- [x] Council history scrolls predictably and revised/withdrawn states are clear.

Notes:

## 4. Daily teamwork workflow

- [x] `K` toggles Ledger/Kanban and selection survives the toggle.
- [x] Kanban Left/Right changes non-empty stance columns; Up/Down changes cards.
- [x] `l`, arrows, and Space add/remove a blocker; self-links and cycles fail
      with understandable messages.
- [x] Completing the blocker produces one **The path has opened** notice for an
      assigned dependent Quest.
- [x] `B` opens Council Briefing; blocked, review, overdue, due-soon, unassigned,
      workload, presence, and recent activity are understandable without color.
- [x] Enter from each selected Briefing queue or Companion opens the expected
      filtered Ledger.
- [x] Global search opens the exact Quest Council message, Chronicle message, or
      Companion workload.
- [x] Command-palette teamwork actions route to the same destinations.

Notes:

## 5. Removal and rotation

- [x] Owner removes Device B and the confirmation clearly explains the impact.
- [ ] Device B retains a private local copy and sees that Fellowship access was
      removed.
- [x] Device B cannot write to the old route after removal.
- [x] Remaining members receive the rotated key/route and exchange later work.
- [x] The removed member receives no post-removal Quest or Council content.

Notes:

## Result

- [x] PASS — human two-device keyboard release gate is complete.
- [ ] FAIL — release-blocking findings are listed below.

Findings and disposition: