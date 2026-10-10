use egui::{Event, MouseWheelUnit, TouchPhase, Ui, Vec2};
use taide_native_editor::document::DocumentId;

const SMOOTH_SCROLL_TIME: f64 = 0.125;
const RESPONSE_ADVANCE: f64 = 0.01;
const LONG_SCROLL_VIEWPORTS: f32 = 2.5;
const LONG_SCROLL_STOP: f32 = 0.75;
const LONG_SCROLL_CUT: f32 = 0.33;
const CUBIC_EXPONENT: i32 = 3;
const PRECISE_WHEEL_DELTA: f32 = 8.0;

pub(crate) fn content_height(height: f32, viewport: f32, line_height: f32, beyond: bool) -> f32 {
    height
        + if beyond {
            (viewport - line_height).max(0.0)
        } else {
            0.0
        }
}

#[derive(Clone)]
struct Animation {
    from: f32,
    to: f32,
    started: f64,
}

impl Animation {
    fn sample(&self, time: f64, viewport: f32) -> (f32, bool) {
        let progress = ((time - self.started + RESPONSE_ADVANCE)
            / (SMOOTH_SCROLL_TIME + RESPONSE_ADVANCE))
            .clamp(0.0, 1.0) as f32;
        if progress >= 1.0 {
            return (self.to, false);
        }
        let interpolate = |from: f32, to: f32, t: f32| {
            from + (to - from) * (1.0 - (1.0 - t).powi(CUBIC_EXPONENT))
        };
        if (self.to - self.from).abs() > LONG_SCROLL_VIEWPORTS * viewport {
            let direction = (self.to - self.from).signum();
            let first = self.from + direction * LONG_SCROLL_STOP * viewport;
            let second = self.to - direction * LONG_SCROLL_STOP * viewport;
            if progress < LONG_SCROLL_CUT {
                return (
                    interpolate(self.from, first, progress / LONG_SCROLL_CUT),
                    true,
                );
            }
            return (
                interpolate(
                    second,
                    self.to,
                    (progress - LONG_SCROLL_CUT) / (1.0 - LONG_SCROLL_CUT),
                ),
                true,
            );
        }
        (interpolate(self.from, self.to, progress), true)
    }
}

#[derive(Clone, Default)]
pub(crate) struct Axis {
    animation: Option<Animation>,
}

impl Axis {
    pub(crate) fn is_animating(&self) -> bool {
        self.animation.is_some()
    }

    pub(crate) fn future(&self, current: f32) -> f32 {
        self.animation
            .as_ref()
            .map_or(current, |animation| animation.to)
    }

    pub(crate) fn sample(
        &mut self,
        ui: &Ui,
        current: f32,
        requested: Option<f32>,
        maximum: f32,
        viewport: f32,
        immediate: bool,
    ) -> f32 {
        let requested = requested.map(|target| target.clamp(0.0, maximum));
        if immediate {
            let target = requested.unwrap_or_else(|| self.future(current));
            self.animation = None;
            return target.clamp(0.0, maximum);
        }
        let time = ui.input(|input| input.time);
        if let Some(target) = requested
            && target != self.future(current)
        {
            self.animation = Some(Animation {
                from: current,
                to: target,
                started: time,
            });
        }
        let Some(animation) = &mut self.animation else {
            return current.clamp(0.0, maximum);
        };
        animation.to = animation.to.clamp(0.0, maximum);
        animation.from = animation.from.clamp(0.0, maximum);
        let (position, pending) = animation.sample(time, viewport);
        if pending {
            ui.ctx().request_repaint();
        } else {
            self.animation = None;
        }
        position.clamp(0.0, maximum)
    }
}

#[derive(Clone, Default)]
pub(crate) struct ScrollState {
    pub(crate) x: Axis,
    pub(crate) y: Axis,
    document: Option<DocumentId>,
    rendered: Option<Vec2>,
    pub(crate) reveal_from: Option<Vec2>,
    touchpad: bool,
}

impl ScrollState {
    pub(crate) fn wheel(&mut self, ui: &Ui) -> (Vec2, bool) {
        ui.input_mut(|input| {
            let mut precise = self.touchpad;
            for event in &input.raw.events {
                if let Event::MouseWheel {
                    unit, delta, phase, ..
                } = event
                {
                    if *phase == TouchPhase::Start {
                        self.touchpad = true;
                        precise = true;
                    }
                    if *unit == MouseWheelUnit::Point && delta.length() < PRECISE_WHEEL_DELTA {
                        precise = true;
                    }
                    if matches!(phase, TouchPhase::End | TouchPhase::Cancel) {
                        precise |= self.touchpad;
                        self.touchpad = false;
                    }
                }
            }
            (input.take_scroll_delta_immediate(), precise)
        })
    }

    pub(crate) fn begin(&mut self, document: DocumentId, current: Vec2) -> bool {
        let external = self.document != Some(document)
            || self.rendered.is_some_and(|rendered| rendered != current);
        if external {
            self.x = Axis::default();
            self.y = Axis::default();
            self.touchpad = false;
        }
        self.document = Some(document);
        external
    }

    pub(crate) fn rendered(&mut self, scroll: Vec2) {
        self.rendered = Some(scroll);
    }
}
