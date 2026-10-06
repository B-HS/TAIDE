use egui::{Pos2, Rect, Vec2, pos2};

const PAGE_TOP_PADDING: f32 = 22.0;
const PAGE_BOTTOM_PADDING: f32 = 22.0;
const PAGE_LEFT_PADDING: f32 = 15.0;
const PAGE_RIGHT_PADDING: f32 = 15.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayPreference {
    Exact,
    Above,
    Below,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverlayBounds {
    Viewport(Rect),
    Page { editor: Rect, window: Rect },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayPlacement {
    pub position: Pos2,
    pub preference: OverlayPreference,
}

struct Candidates {
    fits_above: bool,
    fits_below: bool,
    left: f32,
}

impl OverlayBounds {
    fn candidates(self, anchor: Rect, size: Vec2) -> Candidates {
        let above_top = anchor.top() - size.y;
        let below_top = anchor.bottom();
        match self {
            Self::Viewport(viewport) => Candidates {
                fits_above: anchor.top() - viewport.top() >= size.y,
                fits_below: viewport.bottom() - below_top >= size.y,
                left: anchor
                    .left()
                    .min(viewport.right() - size.x)
                    .max(viewport.left()),
            },
            Self::Page { editor, window } => {
                let minimum = (window.left() + PAGE_LEFT_PADDING).max(editor.left() - size.x);
                let maximum = (editor.right() + size.x).min(window.right() - PAGE_RIGHT_PADDING);
                Candidates {
                    fits_above: above_top - window.top() >= PAGE_TOP_PADDING,
                    fits_below: below_top + size.y <= window.bottom() - PAGE_BOTTOM_PADDING,
                    left: anchor.left().min(maximum - size.x).max(minimum),
                }
            }
        }
    }
}

pub fn place_overlay(
    anchor: Rect,
    size: Vec2,
    preferences: &[OverlayPreference],
    bounds: OverlayBounds,
) -> Option<OverlayPlacement> {
    let candidates = bounds.candidates(anchor, size);
    let placed = |preference: &OverlayPreference| OverlayPlacement {
        position: match preference {
            OverlayPreference::Exact => anchor.left_top(),
            OverlayPreference::Above => pos2(candidates.left, anchor.top() - size.y),
            OverlayPreference::Below => pos2(candidates.left, anchor.bottom()),
        },
        preference: *preference,
    };
    preferences
        .iter()
        .find(|preference| match preference {
            OverlayPreference::Exact => true,
            OverlayPreference::Above => candidates.fits_above,
            OverlayPreference::Below => candidates.fits_below,
        })
        .or(preferences.first())
        .map(placed)
}
