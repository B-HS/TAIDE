use super::egui::{self, Pos2, Rect, Vec2};

pub(super) const ARROW_SIZE: f32 = 10.0;
pub(super) const ARROW_RADIUS: u8 = 2;
const BORDER: f32 = 1.0;
const HALF: f32 = 0.5;

#[derive(Clone, Copy)]
pub(super) struct Body {
    pub(super) bounds: Rect,
    pub(super) radius: f32,
}

impl Body {
    pub(super) fn contains(self, point: Pos2) -> bool {
        if !self.bounds.contains(point) {
            return false;
        }
        let radius = self
            .radius
            .min(self.bounds.width() * HALF)
            .min(self.bounds.height() * HALF);
        let center = self.bounds.shrink(radius);
        let nearest = center.clamp(point);
        point.distance_sq(nearest) <= radius * radius
    }
}

#[derive(Clone, Copy)]
pub(super) struct Arrow {
    pub(super) center: Pos2,
    clip: Rect,
    scaling: f32,
}

impl Arrow {
    pub(super) fn bounds(self) -> Rect {
        let half = ARROW_SIZE * HALF * self.scaling * std::f32::consts::SQRT_2;
        Rect::from_center_size(self.center, Vec2::splat(half + half)).intersect(self.clip)
    }

    pub(super) fn shape(self, fill: egui::Color32) -> egui::epaint::RectShape {
        let mut shape = egui::epaint::RectShape::filled(
            Rect::from_center_size(self.center, Vec2::splat(ARROW_SIZE)),
            ARROW_RADIUS,
            fill,
        )
        .with_angle(std::f32::consts::FRAC_PI_4);
        shape.round_to_pixels = Some(false);
        shape
    }

    pub(super) fn contains(self, point: Pos2) -> bool {
        if !self.clip.contains(point) {
            return false;
        }
        let delta = egui::emath::Rot2::from_angle(-std::f32::consts::FRAC_PI_4)
            * (point - self.center)
            / self.scaling;
        let half = ARROW_SIZE * HALF;
        if delta.x.abs() > half || delta.y.abs() > half {
            return false;
        }
        let radius = f32::from(ARROW_RADIUS);
        let corner = Vec2::new(delta.x.abs(), delta.y.abs()) - Vec2::splat(half - radius);
        corner.max(Vec2::ZERO).length_sq() <= radius * radius
    }

    pub(super) fn transformed(self, transform: egui::emath::TSTransform) -> Self {
        Self {
            center: transform * self.center,
            scaling: self.scaling * transform.scaling,
            ..self
        }
    }
}

pub(super) struct Placement {
    pub(super) position: Pos2,
    pub(super) align: egui::RectAlign,
    floating: Rect,
    clip: Rect,
    vertical: bool,
}

impl Placement {
    pub(super) fn new(
        anchor: Rect,
        size: Vec2,
        clip: Rect,
        align: egui::RectAlign,
        pixels_per_point: f32,
    ) -> Self {
        let (opposite, vertical) = if align == egui::RectAlign::TOP {
            (egui::RectAlign::BOTTOM, true)
        } else if align == egui::RectAlign::BOTTOM {
            (egui::RectAlign::TOP, true)
        } else if align == egui::RectAlign::LEFT {
            (egui::RectAlign::RIGHT, false)
        } else {
            (egui::RectAlign::LEFT, false)
        };
        let candidates = [align, opposite].map(|side| {
            let rect = side.align_rect(&anchor, size, ARROW_SIZE);
            let origin = if vertical {
                let shifted = clip.left().max(rect.left().min(clip.right() - size.x));
                let limited = (anchor.left() - size.x).max(shifted.min(anchor.right()));
                Pos2::new(limited, rect.top())
            } else {
                let shifted = clip.top().max(rect.top().min(clip.bottom() - size.y));
                let limited = (anchor.top() - size.y).max(shifted.min(anchor.bottom()));
                Pos2::new(rect.left(), limited)
            };
            let floating = Rect::from_min_size(origin, size);
            let overflow = if side == egui::RectAlign::TOP {
                clip.top() - floating.top()
            } else if side == egui::RectAlign::BOTTOM {
                floating.bottom() - clip.bottom()
            } else if side == egui::RectAlign::LEFT {
                clip.left() - floating.left()
            } else {
                floating.right() - clip.right()
            };
            (side, floating, overflow)
        });
        let (align, floating, _) = if candidates[0].2 <= 0.0 {
            candidates[0]
        } else if candidates[1].2 <= 0.0 || candidates[1].2 < candidates[0].2 {
            candidates[1]
        } else {
            candidates[0]
        };
        Self {
            position: Pos2::new(
                (floating.left() * pixels_per_point).round() / pixels_per_point,
                (floating.top() * pixels_per_point).round() / pixels_per_point,
            ),
            align,
            floating,
            clip,
            vertical,
        }
    }

    pub(super) fn arrow(&self, anchor: Rect, bounds: Rect) -> Option<Arrow> {
        let (extent, reference, center) = if self.vertical {
            (bounds.width(), anchor.center().x, self.floating.center().x)
        } else {
            (bounds.height(), anchor.center().y, self.floating.center().y)
        };
        let client = (extent - BORDER * 2.0).round();
        let desired = client * HALF + reference - center;
        let half = ARROW_SIZE * HALF;
        let offset = half.max(desired.min(client - half));
        if desired != offset {
            return None;
        }
        let center = if self.align == egui::RectAlign::TOP {
            Pos2::new(
                bounds.left() + offset,
                bounds.bottom() - f32::from(ARROW_RADIUS),
            )
        } else if self.align == egui::RectAlign::BOTTOM {
            Pos2::new(
                bounds.left() + offset,
                bounds.top() + f32::from(ARROW_RADIUS),
            )
        } else if self.align == egui::RectAlign::LEFT {
            Pos2::new(
                bounds.right() - f32::from(ARROW_RADIUS),
                bounds.top() + offset,
            )
        } else {
            Pos2::new(
                bounds.left() + f32::from(ARROW_RADIUS),
                bounds.top() + offset,
            )
        };
        Some(Arrow {
            center,
            clip: self.clip,
            scaling: 1.0,
        })
    }

    pub(super) fn origin(&self, bounds: Rect, arrow: Option<Arrow>) -> Pos2 {
        let gap = if arrow.is_some() { ARROW_SIZE } else { 0.0 };
        let center = arrow.map_or(bounds.center(), |arrow| arrow.center);
        if self.align == egui::RectAlign::TOP {
            return egui::pos2(center.x, bounds.bottom() + gap);
        }
        if self.align == egui::RectAlign::BOTTOM {
            return egui::pos2(center.x, bounds.top() - gap);
        }
        if self.align == egui::RectAlign::LEFT {
            return egui::pos2(bounds.right() + gap, center.y);
        }
        egui::pos2(bounds.left() - gap, center.y)
    }
}
