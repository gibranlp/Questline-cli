// ─────────────────────────────────────────────────────────────────────────────
// screens/hit_test.rs — where mouse clicks landed, remembered from the last
// frame draw() computed. Rects are recomputed every render and discarded, but
// a mouse event is handled on a *later* poll, so draw() stashes the Rects it
// used here instead of the click handler recomputing (and risking drifting
// out of sync with) each screen's layout logic.
// ─────────────────────────────────────────────────────────────────────────────
use ratatui::layout::Rect;

/// Per-screen hit regions, refreshed every frame by that screen's draw() call.
/// Phase 2 adds one field per list/menu screen as click-to-select gets built
/// out for it — see each screen's *HitRegions struct below.
#[derive(Debug, Clone, Default)]
pub struct HitRegions {
    pub editor: Option<EditorHitRegions>,
    pub archive: Option<ArchiveHitRegions>,
    pub gateway: Option<GatewayHitRegions>,
    pub great_chronicle: Option<GreatChronicleHitRegions>,
    pub onboarding: Option<OnboardingHitRegions>,
    pub legends: Option<LegendsHitRegions>,
    pub focus: Option<FocusHitRegions>,
    pub projects: Option<ProjectsHitRegions>,
    pub dashboard: Option<DashboardHitRegions>,
    pub soundscapes: Option<SoundscapesHitRegions>,
    pub library: Option<LibraryHitRegions>,
    pub settings: Option<SettingsHitRegions>,
    pub character: Option<CharacterHitRegions>,
    pub sync: Option<SyncHitRegions>,
    pub fellowship: Option<FellowshipHitRegions>,
    pub workspace: Option<WorkspaceHitRegions>,
    /// Deliberately NOT stashed by any draw() call, unlike every field
    /// above — modals have no single draw() that returns a HitRegions
    /// value (rendering is scattered across main.rs's inline per-variant
    /// blocks plus a handful of screen-local draw functions), and adding
    /// an out-parameter to thread through all of them was judged riskier
    /// than the alternative: App::compute_modal_hit_regions() recomputes
    /// this fresh from `modal_state`/`overlay_modal` on demand, using the
    /// exact same centered_rect/Layout calls each modal's own render code
    /// uses. This does carry the "drift out of sync" risk this module's
    /// own doc comment warns about — a modal's popup dimensions changing
    /// in its render code without the matching arm in
    /// compute_modal_hit_regions being updated to match.
    pub modal: Option<ModalHitRegions>,
}

/// Regions for whichever modal is currently open. See the `modal` field
/// comment on `HitRegions` for why this is computed on demand rather than
/// stashed from a render pass like every other field here.
#[derive(Debug, Clone)]
pub struct ModalHitRegions {
    /// The modal's outer popup bounds. A click outside this cancels the
    /// modal — synthesizes Esc, replicating whatever that modal's own Esc
    /// keybinding already does, side effects included (a few modals save
    /// their draft on Esc rather than discard it, e.g. NewProject/
    /// EditProject — this is existing behavior, not something new).
    pub popup_area: Rect,
    /// Set for confirm-style dialogs where the whole popup interior (minus
    /// any list) is one big "click to confirm" target — carries that
    /// dialog's own primary keybinding, which isn't always the same key
    /// across dialogs (mostly 'y'/'Y', but Enter for a few, and None for
    /// progress modals that aren't dismissible yet).
    pub confirm_key: Option<crossterm::event::KeyCode>,
    /// Set for dialogs upgraded to real, distinct clickable buttons instead
    /// of one whole-popup confirm zone — one entry per button, each
    /// carrying the key it mirrors. Takes priority over `confirm_key` when
    /// present: a click inside the popup that lands on a button fires that
    /// button's key; a click inside the popup that misses every button
    /// does nothing (unlike the single-zone `confirm_key` case, where any
    /// click inside confirms).
    pub buttons: Option<Vec<(Rect, crossterm::event::KeyCode)>>,
    /// Set for list-picker modals — clicking selects a row/item (mirrors
    /// Up/Down), never confirms/activates it.
    pub list: Option<ModalListRegion>,
    /// Set for multi-field forms — clicking a field jumps keyboard focus
    /// there (mirrors Tab), same select-only spirit as `list` above; it
    /// never types into or edits the field. Position in the Vec is that
    /// field's `focus_idx`.
    pub focus_fields: Option<Vec<Rect>>,
}

/// Regions for the task calendar (`App::task_calendar`) — computed on
/// demand the same way `ModalHitRegions` is, for the same reason (no
/// single draw() to stash this from).
#[derive(Debug, Clone)]
pub struct CalendarHitRegions {
    pub popup_area: Rect,
    /// One entry per real (non-blank-padding) day cell in the 6x7 month
    /// grid: its Rect and the date it represents.
    pub days: Vec<(Rect, chrono::NaiveDate)>,
}

/// The About screen's one real click target — computed on demand the same
/// way `ModalHitRegions`/`CalendarHitRegions` are, since it's a single
/// `Block` title rather than a stashed draw() output.
#[derive(Debug, Clone)]
pub struct AboutHitRegions {
    pub report_button: Rect,
}

#[derive(Debug, Clone)]
pub enum ModalListRegion {
    /// A uniform vertical list, one item per screen row.
    /// `first_visible_index` accounts for ratatui's ListState auto-scroll
    /// on the few modals that use it — 0 for modals that render every item
    /// unconditionally (the common case here).
    Rows {
        area: Rect,
        count: usize,
        first_visible_index: usize,
    },
    /// Discrete, non-uniform item Rects — e.g. per-tier boxes in
    /// MilestoneTierSelect, or per-choice spans in ShareNote/
    /// JournalVisibility. Position in the Vec is the item's index.
    Items(Vec<Rect>),
}

/// The regions `screens::editor::draw`/`draw_in_area` rendered on the last frame.
#[derive(Debug, Clone, Copy)]
pub struct EditorHitRegions {
    /// Inner text area — borders already excluded.
    pub body: Rect,
    pub title: Rect,
    pub status: Rect,
    pub quick_note: bool,
}

/// `screens::archive::draw` — a single, unscrolled list (renders from row 0
/// every frame, so `list` need not account for a scroll offset).
#[derive(Debug, Clone, Copy)]
pub struct ArchiveHitRegions {
    /// Inner list area — borders already excluded.
    pub list: Rect,
    /// Archived-project count backing the list; a click past this many rows
    /// (or past the placeholder "no archived campaigns" row) is a no-op.
    pub item_count: usize,
}

/// `screens::gateway::draw` — two hand-placed option boxes, border included
/// (clicking anywhere in the box, border or text, should pick that option).
#[derive(Debug, Clone, Copy)]
pub struct GatewayHitRegions {
    pub option0: Rect,
    pub option1: Rect,
}

/// `screens::great_chronicle::draw` — the two body panels; clicking either
/// one just moves `chapter_panel_focused`, mirroring Left/Right.
#[derive(Debug, Clone, Copy)]
pub struct GreatChronicleHitRegions {
    pub feed: Rect,
    pub chapter_panel: Rect,
}

/// `screens::onboarding::draw` — the name field and the class list.
#[derive(Debug, Clone, Copy)]
pub struct OnboardingHitRegions {
    pub name_input: Rect,
    /// Inner list area — borders already excluded.
    pub class_list: Rect,
    pub class_count: usize,
}

/// `screens::legends::draw` — the relic list has a fixed `Constraint::Length(7)`
/// height and no scroll offset, so only the rows that actually fit are
/// clickable; `item_count` still caps at the real relic count underneath.
#[derive(Debug, Clone, Copy)]
pub struct LegendsHitRegions {
    /// Inner list area — borders already excluded.
    pub relic_list: Rect,
    pub item_count: usize,
}

/// `screens::focus::draw_config_screen` — the four picker cards (Duration /
/// Project / Task / Soundscape), border included, in field-index order.
#[derive(Debug, Clone, Copy)]
pub struct FocusHitRegions {
    pub cards: [Rect; 4],
}

/// What clicking a given rendered row of `screens::projects::draw`'s list
/// should do — mirrors the "All" pinned entry vs. a real campaign row.
#[derive(Debug, Clone, Copy)]
pub enum ProjectsRowTarget {
    All,
    Project(usize),
}

/// `screens::projects::draw` — the campaign list interleaves a pinned "All
/// Campaigns" row and a non-selectable "◇ Shared Campaigns" separator
/// wherever the shared/unshared boundary falls, so — unlike Archive/Legends
/// — the rendered row index isn't the project index. `row_targets` is one
/// entry per rendered row (`None` for the separator/placeholder rows),
/// built by the exact same loop draw() uses to lay out `list_items`.
#[derive(Debug, Clone)]
pub struct ProjectsHitRegions {
    /// Inner list area — borders already excluded.
    pub list: Rect,
    pub row_targets: Vec<Option<ProjectsRowTarget>>,
}

/// `screens::dashboard::default_layout::draw` — the Command Center list
/// interleaves non-selectable separator rows ("-- Quick Wins --" etc.)
/// between the real Main/Next/Quick-Win/Sidequest/Daily entries, so
/// `row_targets` maps each *rendered* row to the logical action index
/// `dashboard_command_targets()` uses (`None` for a separator/placeholder
/// row). The list is `ListState`-driven and auto-scrolls to keep the
/// keyboard selection visible, so `visible_start` records
/// `ListState::offset()` *after* that frame's render — the true first
/// visible logical row — rather than trying to recompute ratatui's
/// scroll-into-view algorithm ourselves.
#[derive(Debug, Clone)]
pub struct DefaultHitRegions {
    /// Inner list area — borders already excluded.
    pub list: Rect,
    pub row_targets: Vec<Option<usize>>,
    pub visible_start: usize,
}

/// `screens::dashboard::journey_map::draw` — same "one clickable list,
/// possibly with non-selectable rows" shape as `DefaultHitRegions` (the
/// trail's waypoint rows map to the same `dashboard_command_targets()`
/// action-index space as every other dashboard layout), just a distinct
/// type so each layout's hit-region shape is free to diverge later.
#[derive(Debug, Clone)]
pub struct JourneyMapHitRegions {
    pub list: Rect,
    pub row_targets: Vec<Option<usize>>,
    pub visible_start: usize,
}

/// `screens::dashboard::todays_agenda::draw` — the urgency-banded list,
/// same shape as `DefaultHitRegions` for the same reason.
#[derive(Debug, Clone)]
pub struct TodaysAgendaHitRegions {
    pub list: Rect,
    pub row_targets: Vec<Option<usize>>,
    pub visible_start: usize,
}

/// `screens::dashboard::deadline_timeline::draw` — the selected day's detail
/// list below the calendar strip. Which *day* is selected is tracked as a
/// plain `App` field (`selected_timeline_day_idx`), not part of this hit-test
/// struct, since day selection is mostly a keyboard (Left/Right) affordance;
/// `list`/`row_targets`/`visible_start` cover that day's task rows exactly
/// like every other dashboard layout's list.
#[derive(Debug, Clone)]
pub struct DeadlineTimelineHitRegions {
    pub list: Rect,
    pub row_targets: Vec<Option<usize>>,
    pub visible_start: usize,
}

/// One dashboard layout's hit regions — exactly one variant is ever active
/// at a time (mirrors `App.dashboard_layout`), so this is an enum rather
/// than a struct of `Option`s: the "only one can be populated" invariant is
/// a compiler fact instead of a convention every reader has to remember.
#[derive(Debug, Clone)]
pub enum DashboardHitRegions {
    Default(DefaultHitRegions),
    JourneyMap(JourneyMapHitRegions),
    TodaysAgenda(TodaysAgendaHitRegions),
    DeadlineTimeline(DeadlineTimelineHitRegions),
}

impl DashboardHitRegions {
    /// Inner list area for whichever layout is active — borders already
    /// excluded, same convention as every variant's own `list` field.
    pub fn list(&self) -> Rect {
        match self {
            Self::Default(r) => r.list,
            Self::JourneyMap(r) => r.list,
            Self::TodaysAgenda(r) => r.list,
            Self::DeadlineTimeline(r) => r.list,
        }
    }

    pub fn row_targets(&self) -> &[Option<usize>] {
        match self {
            Self::Default(r) => &r.row_targets,
            Self::JourneyMap(r) => &r.row_targets,
            Self::TodaysAgenda(r) => &r.row_targets,
            Self::DeadlineTimeline(r) => &r.row_targets,
        }
    }

    pub fn visible_start(&self) -> usize {
        match self {
            Self::Default(r) => r.visible_start,
            Self::JourneyMap(r) => r.visible_start,
            Self::TodaysAgenda(r) => r.visible_start,
            Self::DeadlineTimeline(r) => r.visible_start,
        }
    }
}

/// `screens::soundscapes::draw_local_files_panel` — the nested track list
/// that only appears under the Local Folder source, once a folder with at
/// least one supported file is configured.
#[derive(Debug, Clone, Copy)]
pub struct LocalTracksHitRegions {
    /// Inner paragraph area — borders already excluded. Line 0 of the
    /// paragraph's text starts at this Rect's top edge (no scroll offset).
    pub area: Rect,
    /// Line index (within the paragraph) of the "Random shuffle" row —
    /// `selected_local_track_idx == 0`.
    pub row_start: usize,
    /// Real track rows rendered right after row_start, capped to whatever
    /// fit (mirroring the "...and N more" fallback text below them).
    pub track_count: usize,
}

/// `screens::soundscapes::draw` — the source list has fixed 4-line-tall
/// rows and, like Archive/Legends, renders from item 0 with no scroll
/// offset. `local_tracks` is only Some while the Local Folder source is
/// selected, a folder is configured, and at least one track was found.
#[derive(Debug, Clone, Copy)]
pub struct SoundscapesHitRegions {
    /// Inner list area — borders already excluded.
    pub source_list: Rect,
    pub item_count: usize,
    pub local_tracks: Option<LocalTracksHitRegions>,
}

/// `screens::library::draw` — three columns (Categories / Items / Details).
/// Categories is a fixed 6-row list with no scroll; Items is manually
/// scrolled at draw time (`.skip(start_idx).take(visible_item_rows)` plus a
/// separate `Scrollbar`, not a `ListState`), so `item_start_idx` is the
/// piece a click needs to convert an on-screen row back to an item index.
/// `detail_panel` has no sub-selection — a click there just moves focus.
#[derive(Debug, Clone, Copy)]
pub struct LibraryHitRegions {
    /// Inner category list area — borders already excluded.
    pub cat_list: Rect,
    pub cat_count: usize,
    /// Inner item list area — borders already excluded.
    pub item_list: Rect,
    pub item_start_idx: usize,
    pub item_count: usize,
    /// Full details column, border included.
    pub detail_panel: Rect,
}

/// `screens::settings::draw` — a plain theme `List` (no `.block()`, so no
/// border inset) plus two hand-built `Paragraph`s (Alerts & Audio rows,
/// `selected_settings_focus_idx` 1-5; Oath Calendar rows, 6-15) where each
/// row is one `Line` at a fixed offset from `settings_row()` — no wrapping
/// in practice, though a narrow enough terminal could still wrap a row and
/// throw this off (pre-existing risk, not something Phase 2 fixes). Click
/// only moves focus here, same as Projects/Dashboard/Library — several of
/// these rows are live toggles (notifications, weekday oaths) and a
/// misclick flipping one is worse than requiring the existing Enter/Space
/// keys to commit.
#[derive(Debug, Clone, Copy)]
pub struct SettingsHitRegions {
    /// theme_cols[0] itself — the List has no block/border to inset for.
    pub theme_list: Rect,
    pub theme_count: usize,
    /// Inner Alerts & Audio area — borders already excluded. Row 0 is
    /// focus_idx 1.
    pub alerts_panel: Rect,
    pub alerts_row_count: usize,
    /// Inner Oath Calendar area — borders already excluded. Row 0 is
    /// focus_idx 6.
    pub oath_panel: Rect,
    pub oath_row_count: usize,
}

/// `screens::character::draw` — three focus areas (`character_focus` 0/1/2).
/// The Adventure Log is the hard one: each entry is a hand word-wrapped,
/// *variable*-height `ListItem` inside a `ListState`-driven `List` that
/// auto-scrolls by whole items. `adventure_log_rows` is built right after
/// `render_stateful_widget` mutates the state (same `ListState::offset()`
/// readback as Dashboard) by walking forward from that entry, accumulating
/// each entry's real line count — one vec entry per rendered screen row, so
/// the click handler does a plain index instead of re-deriving the wrap.
/// Reflections is a plain single-line list; `reflection_detail` has no
/// sub-selection, a click there just moves focus.
#[derive(Debug, Clone)]
pub struct CharacterHitRegions {
    /// Inner Adventure Log area — borders already excluded.
    pub adventure_log: Rect,
    /// One entry per rendered screen row, top to bottom; empty if the log
    /// itself is empty (nothing real to select).
    pub adventure_log_rows: Vec<usize>,
    /// Inner Reflections list area. `None` when there are no reflections —
    /// that panel renders a hint message instead of the list/detail split.
    pub reflections_list: Option<Rect>,
    pub reflections_count: usize,
    /// Full reflection detail pane, border included — `None` alongside
    /// `reflections_list`.
    pub reflection_detail: Option<Rect>,
}

/// `screens::sync::draw` — unlike every other screen here, Sync has no
/// existing selection index to hook into: it's a wall of status/stat text
/// with single-letter-hotkey actions, most of which are read-only info (device
/// list, revision log, RPG stats) with no keyboard action of their own. Rather
/// than inventing click targets for all of it, only the three rows with an
/// obvious toggle/CTA affordance ("[s] toggle", "[a] toggle", the
/// "Press [Enter] to Sync Now" banner) get one — each one line tall, at a
/// fixed offset inside the left panel `Paragraph`. A click on one of these
/// activates it immediately (same as the keybinding), unlike Settings'
/// click-only-selects: these three are spaced apart with explicit inline
/// hints, not a dense field of similar-looking rows a stray click could hit
/// by mistake.
#[derive(Debug, Clone, Copy)]
pub struct SyncHitRegions {
    pub sync_now: Rect,
    pub cloud_sync_toggle: Rect,
    pub auto_sync_toggle: Rect,
}

/// `screens::fellowship::draw` — the tab bar is one `Paragraph` of
/// concatenated `Span`s, not a `Tabs` widget with per-tab `Rect`s, so each
/// tab's on-screen column range has to be computed by summing the
/// compile-time-constant label widths that come before it (done once at
/// draw time, same as everything else here). Each of the 8 tabs behind
/// `selected_fellowship_tab` renders a differently-shaped sub-list —
/// `sub_list` carries whichever one the active tab needs (see
/// `FellowshipSubList`); Activity and Treasury have no selection at all, so
/// both leave it `None`.
#[derive(Debug, Clone)]
pub struct FellowshipHitRegions {
    pub tabs: [Rect; 8],
    /// Shared Fellowship Campaigns list in the always-visible left panel —
    /// tab-independent, so it's separate from `sub_list` below.
    pub left_list: Option<FellowshipRowList>,
    /// Whichever clickable list the *active* tab renders, if any — Activity
    /// and Treasury have no selection to click into, so both stay None.
    pub sub_list: Option<FellowshipSubList>,
}

/// A uniform-row-height list rendered inside a plain `Paragraph` (every
/// Fellowship sub-list except Chat is hand-built `Line`s, not a `List`
/// widget) — `first_row_offset` skips any header/summary lines before row 0
/// (e.g. Companions' "N online • M members" line), `row_height` is how many
/// screen rows each item occupies. Both counted once at draw time, same as
/// everything else here, so the click handler is just arithmetic.
#[derive(Debug, Clone, Copy)]
pub struct FellowshipRowList {
    pub area: Rect,
    pub first_row_offset: u16,
    pub row_height: u16,
    pub count: usize,
}

impl FellowshipRowList {
    /// (col, row) -> item index, or None if the click missed the area, fell
    /// in the leading header rows, landed between two items, or is past the
    /// last real item.
    pub fn row_index(&self, col: u16, row: u16) -> Option<usize> {
        if !HitRegions::contains(self.area, col, row) {
            return None;
        }
        let offset_row = (row - self.area.y).checked_sub(self.first_row_offset)?;
        let idx = (offset_row / self.row_height.max(1)) as usize;
        (idx < self.count).then_some(idx)
    }
}

/// The Chronicle chat transcript — variable-height messages in a plain
/// `Paragraph`, manually scrolled (no `ListState`). draw() already computes
/// `msg_start_lines` (each message's first line index) and `scroll` to
/// render it, so this just carries those out instead of the click handler
/// re-deriving them.
#[derive(Debug, Clone)]
pub struct FellowshipChatHitRegions {
    pub area: Rect,
    pub scroll: u16,
    pub msg_start_lines: Vec<u16>,
    pub message_count: usize,
}

#[derive(Debug, Clone)]
pub enum FellowshipSubList {
    Uniform(FellowshipRowList),
    Chat(FellowshipChatHitRegions),
}

/// `screens::project_workspace::draw` — the 5-item sidebar (Overview /
/// Tasks / Scrolls / Treasury / Chronicle) plus whichever of the 5 tabs'
/// own content lists is active. The sidebar is a plain `List`,
/// single-line rows, no scroll — same shape as Archive/Gateway — except
/// its *display* order isn't `workspace_tab_idx` order, so
/// `sidebar_tab_order[row]` is the real tab index for that row.
#[derive(Debug, Clone)]
pub struct WorkspaceHitRegions {
    /// Inner sidebar list area — borders already excluded.
    pub sidebar: Rect,
    pub sidebar_tab_order: [usize; 5],
    /// Overview tab's milestone list — one field per workspace tab, `None`
    /// unless that tab is the one currently rendered (and, for each, unless
    /// its list is non-empty — an empty-state placeholder row isn't real).
    pub milestones: Option<WorkspaceRowList>,
    pub treasury: Option<WorkspaceRowList>,
    pub journal: Option<WorkspaceRowList>,
    pub notes: Option<WorkspaceNotesHitRegions>,
    pub tasks: Option<WorkspaceRowList>,
    /// Tasks tab's Kanban board (`quest_board_open`) — a separate rendering
    /// mode from the list view `tasks` above (mutually exclusive: only one
    /// of the two is ever Some for tab 0), so it gets its own field rather
    /// than overloading that one.
    pub kanban: Option<WorkspaceKanbanHitRegions>,
    /// Tasks tab's right-hand "Quest Ledger" details pane — full pane,
    /// border included, same shape as `notes.preview`. Set whenever tab 0
    /// renders the list view (list and Kanban are mutually exclusive, but
    /// the ledger is only ever drawn alongside the list). A scroll or click
    /// there focuses it, same as the Notes preview, so its own long
    /// description/steps/comments can be scrolled independently of which
    /// quest is selected.
    pub ledger: Option<Rect>,
}

/// `screens::project_workspace::draw_quest_board` — 6 status columns (2 rows
/// of 3), each a `ListState`-scrolled list of cards, where a card spans one
/// header row plus one row per step. `columns[i]` corresponds to
/// `statuses[i]` in `[Backlog, Ready, InProgress, Blocked, Review, Done]`
/// order (row-major: row 0 is the first 3, row 1 the last 3) — the click
/// handler doesn't need to know the status itself, just that a row's
/// `task_idx` is an index into the *same* task slice `selected_task_idx`
/// already indexes, and its `step_idx` (if any) is a position into that
/// task's steps — matching `App.kanban_step_idx`'s meaning exactly, so a
/// click can just assign both fields straight through.
#[derive(Debug, Clone)]
pub struct WorkspaceKanbanHitRegions {
    pub columns: [WorkspaceKanbanColumn; 6],
}

/// One rendered row of a Kanban column: which task's card it belongs to, and
/// — for a step row nested under that card — which of the task's steps.
/// `None` for the card's own header row.
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceKanbanRow {
    pub task_idx: usize,
    pub step_idx: Option<usize>,
}

/// A `WorkspaceRowList`-shaped hit map, but one `WorkspaceKanbanRow` per
/// visible row instead of a plain `Option<usize>` — same "one vec entry per
/// rendered screen row" convention (see `WorkspaceRowList`), just carrying
/// enough to tell a card's header apart from one of its step rows.
#[derive(Debug, Clone)]
pub struct WorkspaceKanbanColumn {
    pub area: Rect,
    pub row_targets: Vec<Option<WorkspaceKanbanRow>>,
}

impl WorkspaceKanbanColumn {
    /// (col, row) -> the row clicked, or None if the click missed the area
    /// or landed past the last rendered row.
    pub fn row_at(&self, col: u16, row: u16) -> Option<WorkspaceKanbanRow> {
        if !HitRegions::contains(self.area, col, row) {
            return None;
        }
        let offset = (row - self.area.y) as usize;
        self.row_targets.get(offset).copied().flatten()
    }
}

/// A rendered row -> item-index map, built once at draw time — the same
/// idea as ProjectsHitRegions.row_targets (`None` for a non-selectable
/// divider/header-only row, e.g. Notes' "── Unassigned / Ungrouped ──").
/// Also covers variable-height rows (e.g. a milestone with unmet-requirement
/// sub-rows) the same way CharacterHitRegions.adventure_log_rows does: one
/// vec entry per rendered screen row, not per item.
#[derive(Debug, Clone)]
pub struct WorkspaceRowList {
    pub area: Rect,
    pub row_targets: Vec<Option<usize>>,
}

impl WorkspaceRowList {
    /// (col, row) -> item index, or None if the click missed the area,
    /// landed past the last rendered row, or hit a divider row.
    pub fn row_index(&self, col: u16, row: u16) -> Option<usize> {
        if !HitRegions::contains(self.area, col, row) {
            return None;
        }
        let offset = (row - self.area.y) as usize;
        self.row_targets.get(offset).copied().flatten()
    }
}

/// Notes/Scrolls tab — a list on the left and (when a note is selected and
/// visible) a preview pane on the right; a click on the preview just moves
/// focus there, mirroring Library/Character's detail panes — unless it lands
/// on a link, which opens it.
#[derive(Debug, Clone)]
pub struct WorkspaceNotesHitRegions {
    pub list: WorkspaceRowList,
    pub preview: Option<Rect>,
    /// One entry per on-screen run of a rendered `http(s)` link in the
    /// preview, with the URL to open. Built at draw time from the same
    /// wrap the preview rendered with (a link split across two visual rows
    /// contributes one entry per row), already offset by the preview's
    /// scroll — so anything off-screen simply isn't in here. Non-http
    /// schemes are deliberately absent: these URLs are handed to the OS
    /// opener, and a shared scroll shouldn't be able to launch `file:` or
    /// worse from a stray click.
    pub preview_links: Vec<(Rect, String)>,
}

impl HitRegions {
    /// True if (col, row) — terminal screen coordinates from a MouseEvent — falls inside rect.
    pub fn contains(rect: Rect, col: u16, row: u16) -> bool {
        col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
    }
}
