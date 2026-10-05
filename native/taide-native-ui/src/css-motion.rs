const EASE_X: [f64; 2] = [0.25, 0.25];
const EASE_Y: [f64; 2] = [0.1, 1.0];
const BEZIER_ITERATIONS: usize = 48;

pub fn ease(progress: f32) -> f32 {
    cubic_bezier(progress, EASE_X, EASE_Y)
}

pub fn cubic_bezier(progress: f32, x: [f64; 2], y: [f64; 2]) -> f32 {
    if progress <= 0.0 || progress >= 1.0 {
        return progress.clamp(0.0, 1.0);
    }
    let coordinate = |t: f64, [first, second]: [f64; 2]| {
        let inverse = 1.0 - t;
        3.0 * inverse * inverse * t * first + 3.0 * inverse * t * t * second + t * t * t
    };
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..BEZIER_ITERATIONS {
        let t = (low + high) * 0.5;
        if coordinate(t, x) < f64::from(progress) {
            low = t;
        } else {
            high = t;
        }
    }
    coordinate((low + high) * 0.5, y) as f32
}
