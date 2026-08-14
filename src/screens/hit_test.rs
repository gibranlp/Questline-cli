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

impl HitRegions {
    /// True if (col, row) — terminal screen coordinates from a MouseEvent — falls inside rect.
    pub fn contains(rect: Rect, col: u16, row: u16) -> bool {
        col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
    }
}
