// ─────────────────────────────────────────────────────────────────────────────
// screens/hit_test.rs — where mouse clicks landed, remembered from the last
// frame draw() computed. Rects are recomputed every render and discarded, but
// a mouse event is handled on a *later* poll, so draw() stashes the Rects it
// used here instead of the click handler recomputing (and risking drifting
// out of sync with) each screen's layout logic.
// ─────────────────────────────────────────────────────────────────────────────
use ratatui::layout::Rect;

/// Per-screen hit regions, refreshed every frame by that screen's draw() call.
/// Only the editor is populated in this pass — add one field per screen as
/// click-to-select gets built out for it.
#[derive(Debug, Clone, Default)]
pub struct HitRegions {
    pub editor: Option<EditorHitRegions>,
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

impl HitRegions {
    /// True if (col, row) — terminal screen coordinates from a MouseEvent — falls inside rect.
    pub fn contains(rect: Rect, col: u16, row: u16) -> bool {
        col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
    }
}
