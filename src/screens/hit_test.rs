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

/// `screens::dashboard::draw_today_command_center` — the Command Center list
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
pub struct DashboardHitRegions {
    /// Inner list area — borders already excluded.
    pub list: Rect,
    pub row_targets: Vec<Option<usize>>,
    pub visible_start: usize,
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

impl HitRegions {
    /// True if (col, row) — terminal screen coordinates from a MouseEvent — falls inside rect.
    pub fn contains(rect: Rect, col: u16, row: u16) -> bool {
        col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
    }
}
