use super::{Duration, Instant};

use crate::css_motion::{cubic_bezier, ease};

const NORMAL: Duration = Duration::from_millis(400);
const COLLAPSED_EXIT: Duration = Duration::from_millis(500);
const COLLAPSED_EXIT_OPACITY: Duration = Duration::from_millis(200);
const CLOSE_OPACITY: Duration = Duration::from_millis(100);
const CLOSE_COLOR: Duration = Duration::from_millis(200);
const FOCUS_SHADOW: Duration = Duration::from_millis(200);
const SCALE_STEP: f32 = 0.05;
const EXIT_PERCENT: f32 = 0.4;
const EASE_OUT_X: [f64; 2] = [0.0, 0.58];
const EASE_OUT_Y: [f64; 2] = [0.0, 1.0];

pub(super) fn ease_out(progress: f32) -> f32 {
    cubic_bezier(progress, EASE_OUT_X, EASE_OUT_Y)
}

struct Transition<const N: usize> {
    from: [f32; N],
    to: [f32; N],
    reversing_start: [f32; N],
    shortening: f32,
    started: Instant,
    duration: Duration,
}

impl<const N: usize> Transition<N> {
    fn new(value: [f32; N], now: Instant) -> Self {
        Self {
            from: value,
            to: value,
            reversing_start: value,
            shortening: 1.0,
            started: now,
            duration: Duration::ZERO,
        }
    }

    fn active(&self, now: Instant) -> bool {
        self.from != self.to && now.saturating_duration_since(self.started) < self.duration
    }

    fn progress(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        ease(
            now.saturating_duration_since(self.started).as_secs_f32() / self.duration.as_secs_f32(),
        )
    }

    fn sample(&self, now: Instant) -> [f32; N] {
        if !self.active(now) {
            return self.to;
        }
        let progress = self.progress(now);
        std::array::from_fn(|index| {
            self.from[index] + (self.to[index] - self.from[index]) * progress
        })
    }

    fn retarget(&mut self, target: [f32; N], duration: Duration, now: Instant) {
        if target == self.to && !duration.is_zero() {
            return;
        }
        let current = self.sample(now);
        let reversed = self.active(now) && target == self.reversing_start;
        let shortening = if reversed {
            (self.progress(now) * self.shortening + 1.0 - self.shortening)
                .abs()
                .clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.reversing_start = if reversed { self.to } else { current };
        self.shortening = shortening;
        self.from = current;
        self.to = target;
        self.started = now;
        self.duration = duration.mul_f32(shortening);
    }
}

#[derive(Clone, Copy)]
pub(super) struct Style {
    pub index: usize,
    pub height: f32,
    pub front_height: f32,
    pub offset: f32,
    pub gap: f32,
    pub top: bool,
    pub expanded: bool,
    pub visible: bool,
    pub removed: bool,
    pub swiping: bool,
    pub swipe_out: bool,
    pub is_reduced_motion: bool,
}

impl Style {
    fn transform(&self, offset: f32) -> [f32; 3] {
        let lift = if self.top { 1.0 } else { -1.0 };
        if self.removed && !self.swipe_out {
            if self.index != 0 && !self.expanded {
                return [0.0, EXIT_PERCENT, 1.0];
            }
            let offset = if self.index == 0 { 0.0 } else { lift * offset };
            return [offset, -lift, 1.0];
        }
        if self.expanded {
            return [lift * offset, 0.0, 1.0];
        }
        if self.index != 0 {
            return [
                lift * self.gap * self.index as f32,
                0.0,
                -(1.0 + self.index as f32 * SCALE_STEP),
            ];
        }
        [0.0, 0.0, 1.0]
    }

    fn height(&self) -> f32 {
        if self.index == 0 || self.expanded {
            self.height
        } else {
            self.front_height
        }
    }

    fn content_opacity(&self) -> f32 {
        f32::from(u8::from(self.index == 0 || self.expanded))
    }
}

pub(super) struct Sample {
    pub height: f32,
    pub translate: f32,
    pub translate_x: f32,
    pub scale: f32,
    pub opacity: f32,
    pub content_opacity: f32,
    pub close_opacity: f32,
    pub active: bool,
}

pub(super) struct Motion {
    transform: Transition<4>,
    height: Transition<1>,
    opacity: Transition<1>,
    content_opacity: Transition<1>,
    close_opacity: Transition<1>,
    last_offset: f32,
    removed_offset: Option<f32>,
    last_style: Style,
    close_colors: Option<[Transition<3>; 2]>,
    focus_shadow: Transition<1>,
}

impl Motion {
    pub fn new(style: &Style, now: Instant) -> Self {
        let entering = if style.top { -1.0 } else { 1.0 };
        let transform = if style.index != 0 && !style.expanded {
            style.transform(style.offset)
        } else {
            [0.0, entering, 1.0]
        };
        Self {
            transform: Transition::new([0.0, transform[0], transform[1], transform[2]], now),
            height: Transition::new([style.height()], now),
            opacity: Transition::new([0.0], now),
            content_opacity: Transition::new([style.content_opacity()], now),
            close_opacity: Transition::new([style.content_opacity()], now),
            last_offset: style.offset,
            removed_offset: None,
            last_style: *style,
            close_colors: None,
            focus_shadow: Transition::new([0.0], now),
        }
    }

    pub fn sample(&mut self, style: &Style, now: Instant) -> Sample {
        if style.removed {
            self.removed_offset.get_or_insert(self.last_offset);
        } else {
            self.last_offset = style.offset;
        }
        let collapsed_exit =
            style.removed && !style.swipe_out && style.index != 0 && !style.expanded;
        let transform = style.transform(self.removed_offset.unwrap_or(style.offset));
        self.transform.retarget(
            [0.0, transform[0], transform[1], transform[2]],
            if style.swiping || style.is_reduced_motion {
                Duration::ZERO
            } else if collapsed_exit {
                COLLAPSED_EXIT
            } else {
                NORMAL
            },
            now,
        );
        self.height.retarget(
            [style.height()],
            if style.is_reduced_motion
                || style.swiping
                || (!style.expanded && style.index == 0)
                || (!self.last_style.expanded && self.last_style.index == 0)
                || collapsed_exit
            {
                Duration::ZERO
            } else {
                NORMAL
            },
            now,
        );
        self.opacity.retarget(
            [f32::from(u8::from(
                style.visible && (!style.removed || style.swipe_out),
            ))],
            if style.swiping || style.is_reduced_motion {
                Duration::ZERO
            } else if collapsed_exit {
                COLLAPSED_EXIT_OPACITY
            } else {
                NORMAL
            },
            now,
        );
        self.content_opacity.retarget(
            [style.content_opacity()],
            if style.is_reduced_motion {
                Duration::ZERO
            } else {
                NORMAL
            },
            now,
        );
        self.close_opacity.retarget(
            [style.content_opacity()],
            if style.is_reduced_motion {
                Duration::ZERO
            } else {
                CLOSE_OPACITY
            },
            now,
        );
        let height = self.height.sample(now)[0];
        let [translate_x, translate, percent, scale] = self.transform.sample(now);
        self.last_style = *style;
        Sample {
            height,
            translate: translate + percent * height,
            translate_x,
            scale,
            opacity: self.opacity.sample(now)[0],
            content_opacity: self.content_opacity.sample(now)[0],
            close_opacity: self.close_opacity.sample(now)[0],
            active: self.transform.active(now)
                || self.height.active(now)
                || self.opacity.active(now)
                || self.content_opacity.active(now)
                || self.close_opacity.active(now),
        }
    }

    pub fn dismiss(&mut self, now: Instant) {
        self.sample(
            &Style {
                removed: true,
                ..self.last_style
            },
            now,
        );
    }

    pub fn return_from_swipe(&mut self, amount: [f32; 2], now: Instant) {
        let [x, y, percent, scale] = self.transform.sample(now);
        self.transform = Transition::new(
            [x + scale * amount[0], y + scale * amount[1], percent, scale],
            now,
        );
    }

    pub fn close_colors(&mut self, targets: [[f32; 3]; 2], now: Instant) -> ([[f32; 3]; 2], bool) {
        let colors = self
            .close_colors
            .get_or_insert_with(|| targets.map(|color| Transition::new(color, now)));
        for (transition, target) in colors.iter_mut().zip(targets) {
            transition.retarget(
                target,
                if self.last_style.is_reduced_motion {
                    Duration::ZERO
                } else {
                    CLOSE_COLOR
                },
                now,
            );
        }
        (
            [colors[0].sample(now), colors[1].sample(now)],
            colors.iter().any(|color| color.active(now)),
        )
    }

    pub fn focus_shadow(&mut self, focused: bool, now: Instant) -> (f32, bool) {
        self.focus_shadow.retarget(
            [f32::from(u8::from(focused))],
            if self.last_style.swiping || self.last_style.is_reduced_motion {
                Duration::ZERO
            } else {
                FOCUS_SHADOW
            },
            now,
        );
        (
            self.focus_shadow.sample(now)[0],
            self.focus_shadow.active(now),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_style() -> Style {
        Style {
            index: 0,
            height: 80.0,
            front_height: 60.0,
            offset: 0.0,
            gap: 14.0,
            top: false,
            expanded: false,
            visible: true,
            removed: false,
            swiping: false,
            swipe_out: false,
            is_reduced_motion: false,
        }
    }

    #[test]
    fn native_toast_reduced_motion은_모든전환을_즉시적용하고_복귀를_보존한다() {
        let now = Instant::now();
        let mut style = create_style();
        let mut motion = Motion::new(&style, now);
        assert_eq!(motion.sample(&style, now).opacity, 0.0);
        style.is_reduced_motion = true;
        let now = now + Duration::from_millis(100);
        let sample = motion.sample(&style, now);
        assert_eq!(sample.opacity, 1.0);
        assert_eq!(sample.translate, 0.0);
        assert_eq!(sample.scale, 1.0);
        assert!(!sample.active);
        assert_eq!(motion.focus_shadow(true, now), (1.0, false));
        let initial_colors = [[0.0; 3]; 2];
        let final_colors = [[255.0; 3]; 2];
        motion.close_colors(initial_colors, now);
        assert_eq!(
            motion.close_colors(final_colors, now),
            (final_colors, false)
        );
        style.index = 1;
        style.offset = 100.0;
        let sample = motion.sample(&style, now);
        assert_eq!(sample.height, style.front_height);
        assert_eq!(sample.scale, -(1.0 + SCALE_STEP));
        assert_eq!(sample.content_opacity, 0.0);
        assert_eq!(sample.close_opacity, 0.0);
        assert!(!sample.active);
        style.expanded = true;
        let sample = motion.sample(&style, now);
        assert_eq!(sample.height, style.height);
        assert_eq!(sample.translate, -style.offset);
        assert_eq!(sample.scale, 1.0);
        assert_eq!(sample.content_opacity, 1.0);
        assert_eq!(sample.close_opacity, 1.0);
        assert!(!sample.active);
        style.is_reduced_motion = false;
        assert!(!motion.sample(&style, now).active);
        style.removed = true;
        let sample = motion.sample(&style, now);
        assert_eq!(sample.opacity, 1.0);
        assert!(sample.active);
        style.is_reduced_motion = true;
        let sample = motion.sample(&style, now);
        assert_eq!(sample.opacity, 0.0);
        assert!(!sample.active);
    }

    #[test]
    fn native_toast_shadow_swipe는_transition_none을_적용한다() {
        let now = Instant::now();
        let mut style = create_style();
        let mut motion = Motion::new(&style, now);
        assert_eq!(motion.focus_shadow(true, now), (0.0, true));
        style.swiping = true;
        motion.sample(&style, now);
        assert_eq!(motion.focus_shadow(true, now), (1.0, false));
        assert_eq!(motion.focus_shadow(false, now), (0.0, false));
        style.swiping = false;
        motion.sample(&style, now);
        assert_eq!(motion.focus_shadow(true, now), (0.0, true));
        assert_eq!(motion.focus_shadow(true, now + FOCUS_SHADOW), (1.0, false));
    }

    #[test]
    fn native_toast_motion은_css_ease와_중간반전_수명을_보존한다() {
        let now = Instant::now();
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert!((ease(0.3125) - 0.5375).abs() < f32::EPSILON);
        let mut transition = Transition::new([0.0], now);
        transition.retarget([1.0], NORMAL, now);
        let midway = now + Duration::from_millis(125);
        let before = transition.sample(midway);
        assert!((before[0] - 0.5375).abs() < f32::EPSILON);
        transition.retarget([0.0], NORMAL, midway);
        assert_eq!(transition.sample(midway), before);
        const CLOCK_TOLERANCE: Duration = Duration::from_nanos(100);
        assert!(transition.duration.abs_diff(Duration::from_millis(215)) < CLOCK_TOLERANCE);
        let reversal = midway + Duration::from_millis(50);
        let before = transition.sample(reversal);
        transition.retarget([1.0], NORMAL, reversal);
        assert_eq!(transition.sample(reversal), before);
        assert!(transition.duration < NORMAL);
        let end = reversal + transition.duration;
        assert_eq!(transition.sample(end), [1.0]);
        assert!(!transition.active(end));
        let started = transition.started;
        transition.retarget([1.0], NORMAL, end);
        assert_eq!(transition.started, started);
    }

    #[test]
    fn native_toast_motion은_입장_확장_자식투명도와_종료분기를_보존한다() {
        let now = Instant::now();
        for top in [false, true] {
            let mut style = Style {
                top,
                ..create_style()
            };
            let mut motion = Motion::new(&style, now);
            let initial = motion.sample(&style, now);
            assert_eq!(initial.opacity, 0.0);
            assert_eq!(initial.translate, if top { -80.0 } else { 80.0 });
            assert!(initial.active);
            let end = now + NORMAL;
            let entered = motion.sample(&style, end);
            assert_eq!(entered.opacity, 1.0);
            assert_eq!(entered.translate, 0.0);
            assert!(!entered.active);
            style.removed = true;
            let exit = motion.sample(&style, end);
            assert_eq!(exit.translate, 0.0);
            let exit = motion.sample(&style, end + Duration::from_millis(200));
            assert!(exit.opacity > 0.0 && exit.opacity < 1.0);
            assert!(if top {
                exit.translate < 0.0
            } else {
                exit.translate > 0.0
            });
        }
        let mut style = Style {
            index: 1,
            offset: 94.0,
            ..create_style()
        };
        let mut motion = Motion::new(&style, now);
        motion.sample(&style, now);
        let end = now + NORMAL;
        let collapsed = motion.sample(&style, end);
        assert_eq!(collapsed.scale, -1.05);
        assert_eq!(collapsed.translate, -14.0);
        assert_eq!(collapsed.height, 60.0);
        assert_eq!(collapsed.content_opacity, 0.0);
        style.expanded = true;
        let start = motion.sample(&style, end);
        assert_eq!(start.scale, collapsed.scale);
        let partially = motion.sample(&style, end + CLOSE_OPACITY);
        assert_eq!(partially.close_opacity, 1.0);
        assert!(partially.content_opacity > 0.0 && partially.content_opacity < 1.0);
        let expanded = motion.sample(&style, end + NORMAL);
        assert_eq!(expanded.translate, -94.0);
        assert_eq!(expanded.scale, 1.0);
        assert_eq!(expanded.height, 80.0);
        assert_eq!(expanded.content_opacity, 1.0);
        style.removed = true;
        style.offset = 200.0;
        motion.sample(&style, end + NORMAL);
        let removed = motion.sample(&style, end + NORMAL * 2);
        assert_eq!(removed.translate, -94.0 + 80.0);
        assert_eq!(removed.opacity, 0.0);
        let mut style = Style {
            index: 1,
            offset: 94.0,
            ..create_style()
        };
        let mut motion = Motion::new(&style, now);
        motion.sample(&style, now);
        motion.sample(&style, end);
        style.removed = true;
        motion.sample(&style, end);
        let removed = motion.sample(&style, end + COLLAPSED_EXIT_OPACITY);
        assert_eq!(removed.opacity, 0.0);
        assert!(removed.active);
        assert_ne!(removed.translate, 60.0 * EXIT_PERCENT);
        let removed = motion.sample(&style, end + COLLAPSED_EXIT);
        assert_eq!(removed.translate, 60.0 * EXIT_PERCENT);
        assert_eq!(removed.scale, 1.0);
    }
}
