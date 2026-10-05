use super::{Duration, Instant};

use egui::{Pos2, Vec2};

const THRESHOLD: f32 = 45.0;
const VELOCITY: f64 = 0.11;
const LOCK_DISTANCE: f32 = 1.0;
const DAMPING_DISTANCE: f32 = 20.0;
const DAMPING_BASE: f32 = 1.5;
const EXIT_DURATION: Duration = Duration::from_millis(200);

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

pub(super) struct Directions {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

pub(super) struct Capture {
    origin: Pos2,
    started: Instant,
    axis: Option<Axis>,
    pub amount: Vec2,
}

impl Capture {
    pub fn new(origin: Pos2, now: Instant) -> Self {
        Self {
            origin,
            started: now,
            axis: None,
            amount: Vec2::ZERO,
        }
    }

    pub fn update(&mut self, position: Pos2, highlighted: bool, directions: Directions) {
        let delta = position - self.origin;
        if highlighted || !delta.is_finite() {
            return;
        }
        let Some(axis) = self.axis else {
            if delta.x.abs() > LOCK_DISTANCE || delta.y.abs() > LOCK_DISTANCE {
                self.axis = Some(if delta.x.abs() > delta.y.abs() {
                    Axis::X
                } else {
                    Axis::Y
                });
            }
            return;
        };
        let (amount, negative, positive) = match axis {
            Axis::X => (delta.x, directions.left, directions.right),
            Axis::Y => (delta.y, directions.top, directions.bottom),
        };
        let amount = if !negative && !positive {
            0.0
        } else if (negative && amount < 0.0) || (positive && amount > 0.0) {
            amount
        } else {
            let damped = amount / (DAMPING_BASE + amount.abs() / DAMPING_DISTANCE);
            if damped.abs() < amount.abs() {
                damped
            } else {
                amount
            }
        };
        self.amount = match axis {
            Axis::X => Vec2::new(amount, 0.0),
            Axis::Y => Vec2::new(0.0, amount),
        };
    }

    pub fn release(&self, now: Instant) -> Option<Exit> {
        let axis = self.axis.unwrap_or(Axis::Y);
        let amount = match axis {
            Axis::X => self.amount.x,
            Axis::Y => self.amount.y,
        };
        let milliseconds = now.saturating_duration_since(self.started).as_millis() as f64;
        let should_exit =
            amount.abs() >= THRESHOLD || f64::from(amount.abs()) / milliseconds > VELOCITY;
        if !should_exit {
            return None;
        }
        Some(Exit {
            amount: self.amount,
            axis,
            positive: amount > 0.0,
            started: now,
        })
    }
}

pub(super) struct Exit {
    amount: Vec2,
    axis: Axis,
    positive: bool,
    started: Instant,
}

impl Exit {
    pub fn sample(
        &self,
        now: Instant,
        width: f32,
        height: f32,
        is_reduced_motion: bool,
    ) -> (Vec2, f32) {
        if is_reduced_motion {
            return (self.amount, 1.0);
        }
        let progress = super::motion::ease_out(
            now.saturating_duration_since(self.started).as_secs_f32() / EXIT_DURATION.as_secs_f32(),
        );
        let sign = if self.positive { 1.0 } else { -1.0 };
        let movement = match self.axis {
            Axis::X => Vec2::new(sign * width * progress, 0.0),
            Axis::Y => Vec2::new(0.0, sign * height * progress),
        };
        (self.amount + movement, 1.0 - progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directions() -> Directions {
        Directions {
            top: false,
            bottom: true,
            left: false,
            right: true,
        }
    }

    #[test]
    fn native_toast_swipe는_축잠금_감쇠_속도_선택과_종료를_보존한다() {
        let now = Instant::now();
        let mut capture = Capture::new(Pos2::ZERO, now);
        capture.update(Pos2::new(50.0, 1.0), false, directions());
        assert_eq!(capture.amount, Vec2::ZERO);
        capture.update(Pos2::new(50.0, 1.0), false, directions());
        assert_eq!(capture.amount, Vec2::new(50.0, 0.0));
        capture.update(Pos2::new(-50.0, 100.0), true, directions());
        assert_eq!(capture.amount, Vec2::new(50.0, 0.0));
        capture.update(Pos2::new(-50.0, 100.0), false, directions());
        assert_eq!(capture.amount, Vec2::new(-12.5, 0.0));
        assert!(capture.release(now + Duration::from_secs(1)).is_none());
        assert!(capture.release(now).is_some());
        capture.update(Pos2::new(44.0, 100.0), false, directions());
        assert!(capture.release(now + Duration::from_millis(400)).is_none());
        assert!(capture.release(now + Duration::from_millis(399)).is_some());
        capture.update(Pos2::new(45.0, 100.0), false, directions());
        let exit = capture.release(now + Duration::from_secs(1)).unwrap();
        let started = now + Duration::from_secs(1);
        assert_eq!(
            exit.sample(started, 356.0, 80.0, false),
            (Vec2::new(45.0, 0.0), 1.0)
        );
        let (amount, opacity) = exit.sample(started + EXIT_DURATION / 2, 356.0, 80.0, false);
        assert!(amount.x > 223.0 && amount.x < 401.0);
        assert!(opacity > 0.0 && opacity < 0.5);
        assert_eq!(
            exit.sample(started + EXIT_DURATION, 356.0, 80.0, false),
            (Vec2::new(401.0, 0.0), 0.0)
        );
        let mut vertical = Capture::new(Pos2::ZERO, now);
        vertical.update(Pos2::new(2.0, 2.0), false, directions());
        vertical.update(Pos2::new(100.0, 50.0), false, directions());
        assert_eq!(vertical.amount, Vec2::new(0.0, 50.0));
        let exit = vertical.release(now + Duration::from_secs(1)).unwrap();
        assert_eq!(
            exit.sample(
                now + Duration::from_secs(1) + EXIT_DURATION,
                356.0,
                80.0,
                false
            ),
            (Vec2::new(0.0, 130.0), 0.0)
        );
        let mut center = Capture::new(Pos2::ZERO, now);
        for _ in 0..2 {
            center.update(
                Pos2::new(100.0, 0.0),
                false,
                Directions {
                    top: true,
                    bottom: false,
                    left: false,
                    right: false,
                },
            );
        }
        assert_eq!(center.amount, Vec2::ZERO);
        assert!(center.release(now).is_none());
    }

    #[test]
    fn native_toast_reduced_motion_swipe는_삭제전까지_원래이동량을_유지한다() {
        let now = Instant::now();
        let mut capture = Capture::new(Pos2::ZERO, now);
        for _ in 0..2 {
            capture.update(Pos2::new(50.0, 0.0), false, directions());
        }
        let at = now + Duration::from_secs(1);
        let exit = capture.release(at).unwrap();
        for elapsed in [Duration::ZERO, EXIT_DURATION / 2, EXIT_DURATION] {
            assert_eq!(
                exit.sample(at + elapsed, 356.0, 80.0, true),
                (Vec2::new(50.0, 0.0), 1.0)
            );
        }
    }
}
