use std::sync::{Arc, Weak};

use egui::{Color32, Context, Id};

const DURATION: f64 = 0.15;
const TIMING_X: [f64; 2] = [0.4, 0.2];
const TIMING_Y: [f64; 2] = [0.0, 1.0];

#[derive(Clone)]
struct ActiveOwner(Id);

#[derive(Clone, Default)]
struct OwnedMotions(Vec<Id>);

pub(crate) struct Owner {
    context: Context,
    id: Id,
    _lifetime: Weak<()>,
}

impl Owner {
    pub(crate) fn new(context: Context, lifetime: &Arc<()>) -> Self {
        Self {
            context,
            id: Id::new(("snippet-button-color-owner", Arc::as_ptr(lifetime))),
            _lifetime: Arc::downgrade(lifetime),
        }
    }

    pub(crate) fn activate(&self) {
        let active = active_owner_id(&self.context);
        self.context.data_mut(|data| {
            data.insert_temp(active, ActiveOwner(self.id));
        });
    }

    pub(crate) fn belongs_to(&self, context: &Context) -> bool {
        self.context == *context
    }
}

impl Drop for Owner {
    fn drop(&mut self) {
        let active = active_owner_id(&self.context);
        self.context.data_mut(|data| {
            if let Some(motions) = data.remove_temp::<OwnedMotions>(self.id) {
                for id in motions.0 {
                    data.remove::<Motion>(id);
                }
            }
            if data
                .get_temp::<ActiveOwner>(active)
                .is_some_and(|owner| owner.0 == self.id)
            {
                data.remove::<ActiveOwner>(active);
            }
        });
    }
}

fn active_owner_id(context: &Context) -> Id {
    Id::new(("snippet-button-active-color-owner", context.viewport_id()))
}

#[derive(Clone)]
struct Motion {
    start: [f32; 4],
    end: [f32; 4],
    reversing_start: [f32; 4],
    shortening: f32,
    started: f64,
    duration: f64,
    frame: u64,
}

impl Motion {
    #[cfg(test)]
    fn new(target: Color32, time: f64, frame: u64) -> Self {
        let value = target.to_array().map(f32::from);
        Self::new_value(value, time, frame)
    }

    fn new_value(value: [f32; 4], time: f64, frame: u64) -> Self {
        Self {
            start: value,
            end: value,
            reversing_start: value,
            shortening: 1.0,
            started: time,
            duration: 0.0,
            frame,
        }
    }

    fn progress(&self, time: f64) -> f32 {
        if self.duration <= 0.0 {
            return 1.0;
        }
        ((time - self.started) / self.duration).clamp(0.0, 1.0) as f32
    }

    fn value(&self, time: f64) -> [f32; 4] {
        let eased = crate::css_motion::cubic_bezier(self.progress(time), TIMING_X, TIMING_Y);
        std::array::from_fn(|index| self.start[index] * (1.0 - eased) + self.end[index] * eased)
    }

    fn sample_value(&mut self, target: [f32; 4], time: f64, frame: u64) -> ([f32; 4], bool) {
        if frame.saturating_sub(self.frame) > 1 {
            *self = Self::new_value(target, time, frame);
        }
        self.frame = frame;
        if self.end != target {
            let progress = self.progress(time);
            let current = self.value(time);
            if progress < 1.0 && target == self.reversing_start {
                let eased = crate::css_motion::cubic_bezier(progress, TIMING_X, TIMING_Y);
                self.shortening = (eased * self.shortening + 1.0 - self.shortening)
                    .abs()
                    .clamp(0.0, 1.0);
                self.reversing_start = self.end;
            } else {
                self.shortening = 1.0;
                self.reversing_start = current;
            }
            self.start = current;
            self.end = target;
            self.started = time;
            self.duration = DURATION * f64::from(self.shortening);
        }
        (
            self.value(time),
            self.start != self.end && self.progress(time) < 1.0,
        )
    }

    #[cfg(test)]
    fn sample(&mut self, target: Color32, time: f64, frame: u64) -> (Color32, bool) {
        let (value, running) = self.sample_value(target.to_array().map(f32::from), time, frame);
        (color(value), running)
    }
}

pub(crate) fn animate(context: &Context, id: Id, target: Color32) -> Color32 {
    color(animate_value(context, id, target.to_array().map(f32::from)))
}

pub(crate) fn animate_amount(context: &Context, id: Id, target: f32) -> f32 {
    animate_value(context, id, [target, 0.0, 0.0, 0.0])[0]
}

fn color(value: [f32; 4]) -> Color32 {
    let [red, green, blue, alpha] = value.map(|value| value.round() as u8);
    Color32::from_rgba_premultiplied(red, green, blue, alpha)
}

fn animate_value(context: &Context, id: Id, target: [f32; 4]) -> [f32; 4] {
    let time = context.input(|input| input.time);
    let frame = context.cumulative_frame_nr();
    let active = active_owner_id(context);
    let (color, running) = context.data_mut(|data| {
        let Some(owner) = data.get_temp::<ActiveOwner>(active) else {
            return (target, false);
        };
        let id = id.with(owner.0);
        let motions = data.get_temp_mut_or_default::<OwnedMotions>(owner.0);
        if !motions.0.contains(&id) {
            motions.0.push(id);
        }
        let motion =
            data.get_temp_mut_or_insert_with(id, || Motion::new_value(target, time, frame));
        motion.sample_value(target, time, frame)
    });
    if running {
        context.request_repaint();
    }
    color
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALF: f64 = DURATION / 2.0;
    const HALF_EASED: f32 = 0.77556133;
    const EPSILON: f32 = 0.00001;

    #[test]
    fn button_전환은_편집기_owner별로_격리되고_drop에서_회수된다() {
        let context = Context::default();
        let id = Id::new("same-button");
        let first = Arc::new(());
        let second = Arc::new(());
        let owner = Owner::new(context.clone(), &first);
        let next = Owner::new(context.clone(), &second);
        let mut colors = Vec::new();
        let mut frame = |time, target, owner: &Owner| {
            let mut output = context.run_ui(
                egui::RawInput {
                    time: Some(time),
                    ..Default::default()
                },
                |_| {
                    owner.activate();
                    colors.push(animate(&context, id, target));
                },
            );
            output.textures_delta.clear();
        };
        let _ = frame(0.0, Color32::BLACK, &owner);
        let _ = frame(HALF, Color32::WHITE, &owner);
        let _ = frame(DURATION, Color32::WHITE, &owner);
        let _ = frame(DURATION + HALF, Color32::WHITE, &next);
        assert_eq!(colors[0], Color32::BLACK);
        assert_eq!(colors[1], Color32::BLACK);
        assert_ne!(colors[2], Color32::WHITE);
        assert_eq!(colors[3], Color32::WHITE);
        let first_id = id.with(owner.id);
        let next_id = id.with(next.id);
        assert!(context.data(|data| data.get_temp::<Motion>(first_id).is_some()));
        drop(owner);
        assert!(context.data(|data| data.get_temp::<Motion>(first_id).is_none()));
        assert!(context.data(|data| data.get_temp::<Motion>(next_id).is_some()));
        drop(next);
        assert!(context.data(|data| data.get_temp::<Motion>(next_id).is_none()));
    }

    #[test]
    fn button_색상은_150ms_곡선과_연속_반전_축소_및_재마운트를_따른다() {
        let start = Color32::from_rgb(0, 0, 0);
        let end = Color32::from_rgb(200, 100, 50);
        let mut motion = Motion::new(start, 0.0, 0);
        assert_eq!(motion.sample(start, 0.0, 0), (start, false));
        assert_eq!(motion.sample(end, 0.0, 0), (start, true));
        let (middle, running) = motion.sample(end, HALF, 1);
        assert_eq!(middle, Color32::from_rgb(155, 78, 39));
        assert!(running);
        assert_eq!(motion.sample(start, HALF, 1), (middle, true));
        assert!((motion.shortening - HALF_EASED).abs() < EPSILON);
        let reverse_half = HALF + motion.duration / 2.0;
        let reversing = motion.sample(start, reverse_half, 2).0;
        assert_eq!(motion.sample(end, reverse_half, 2), (reversing, true));
        let double_shortening = HALF_EASED * HALF_EASED + 1.0 - HALF_EASED;
        assert!((motion.shortening - double_shortening).abs() < EPSILON);
        assert_eq!(motion.sample(end, reverse_half + DURATION, 3), (end, false));
        let transparent = Color32::from_rgba_unmultiplied(200, 100, 50, 0);
        assert_eq!(
            motion.sample(transparent, reverse_half + DURATION, 3),
            (end, true)
        );
        let (fading, running) = motion.sample(transparent, reverse_half + DURATION + HALF, 4);
        assert_eq!(fading, Color32::from_rgba_premultiplied(45, 22, 11, 57));
        assert!(running);
        assert_eq!(
            motion.sample(start, reverse_half + DURATION + HALF, 6),
            (start, false)
        );
    }
}
