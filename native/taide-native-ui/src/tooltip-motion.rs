use super::egui::{self, Pos2, Vec2, emath::TSTransform};

const DURATION: f64 = 0.15;
const SCALE_DELTA: f32 = 0.05;
const SLIDE: f32 = 8.0;

#[derive(Clone, Copy)]
pub(crate) struct Motion {
    open: bool,
    started: f64,
    duration: f64,
}

pub(crate) struct Sample {
    pub(crate) opacity: f32,
    pub(crate) scale: f32,
    pub(crate) translate: Vec2,
    pub(crate) active: bool,
}

impl Motion {
    pub(crate) fn new(open: bool, now: f64) -> Self {
        Self::with_duration(open, now, DURATION)
    }

    pub(crate) fn with_duration(open: bool, now: f64, duration: f64) -> Self {
        Self {
            open,
            started: now,
            duration,
        }
    }

    pub(crate) fn target(&mut self, open: bool, now: f64) {
        if self.open != open {
            self.open = open;
            self.started = now;
        }
    }

    pub(crate) fn is_open(self) -> bool {
        self.open
    }

    pub(crate) fn is_present(self, now: f64) -> bool {
        self.open || now < self.started + self.duration
    }

    pub(crate) fn sample(self, now: f64, side: egui::RectAlign) -> Sample {
        let offset = if !self.open {
            Vec2::ZERO
        } else if side == egui::RectAlign::TOP {
            egui::vec2(0.0, SLIDE)
        } else if side == egui::RectAlign::BOTTOM {
            egui::vec2(0.0, -SLIDE)
        } else if side == egui::RectAlign::LEFT {
            egui::vec2(SLIDE, 0.0)
        } else {
            egui::vec2(-SLIDE, 0.0)
        };
        self.sample_with_offset(now, offset)
    }

    pub(crate) fn sample_popup(self, now: f64) -> Sample {
        self.sample_with_offset(now, Vec2::ZERO)
    }

    fn sample_with_offset(self, now: f64, offset: Vec2) -> Sample {
        let progress = crate::css_motion::ease(((now - self.started) / self.duration) as f32);
        let opacity = if self.open { progress } else { 1.0 - progress };
        let delta = if self.open { 1.0 - progress } else { progress };
        Sample {
            opacity,
            scale: 1.0 - SCALE_DELTA * delta,
            translate: offset * delta,
            active: now < self.started + self.duration,
        }
    }
}

impl Sample {
    pub(crate) fn transform(&self, origin: Pos2) -> TSTransform {
        TSTransform::new(
            origin.to_vec2() * (1.0 - self.scale) + self.translate,
            self.scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMES: [f64; 4] = [0.0, 0.03, 0.075, 0.149];

    #[test]
    fn popup은_같은_css_전이에서_위치이동없이_불투명도와_크기만_변경한다() {
        for open in [false, true] {
            let motion = Motion::new(open, 0.0);
            for time in TIMES {
                let tooltip = motion.sample(time, egui::RectAlign::BOTTOM);
                let popup = motion.sample_popup(time);
                assert_eq!(popup.opacity, tooltip.opacity);
                assert_eq!(popup.scale, tooltip.scale);
                assert_eq!(popup.active, tooltip.active);
                assert_eq!(popup.translate, Vec2::ZERO);
            }
        }
    }
}
