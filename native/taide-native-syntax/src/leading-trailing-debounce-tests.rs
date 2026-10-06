use std::time::{Duration, Instant};

use super::LeadingTrailingDebounce;

const DELAY: Duration = Duration::from_millis(150);
const SHORT_STEP: Duration = Duration::from_millis(100);

#[test]
fn 쉬고_있을_때의_호출은_바로_실행하고_밀린_실행을_남기지_않는다() {
    let start = Instant::now();
    let mut debounce = LeadingTrailingDebounce::new(DELAY);
    assert_eq!(debounce.trailing_delay(start), None);
    assert!(debounce.trigger(start));
    assert_eq!(debounce.trailing_delay(start), None);
    assert!(!debounce.poll(start + SHORT_STEP));
    assert!(!debounce.poll(start + DELAY));
    assert!(debounce.trigger(start + DELAY));
}

#[test]
fn 대기_중의_호출은_대기_시간을_다시_시작하고_끝난_뒤_한_번만_실행한다() {
    let start = Instant::now();
    let mut debounce = LeadingTrailingDebounce::new(DELAY);
    assert!(debounce.trigger(start));
    assert!(!debounce.trigger(start + SHORT_STEP));
    assert_eq!(debounce.trailing_delay(start + SHORT_STEP), Some(DELAY));
    assert!(!debounce.trigger(start + SHORT_STEP * 2));
    assert!(!debounce.poll(start + DELAY));
    assert!(!debounce.poll(start + SHORT_STEP * 3));
    assert_eq!(
        debounce.trailing_delay(start + SHORT_STEP * 3),
        Some(DELAY - SHORT_STEP)
    );
    assert_eq!(
        debounce.trailing_delay(start + SHORT_STEP * 2 + DELAY * 2),
        Some(Duration::ZERO)
    );
    assert!(debounce.poll(start + SHORT_STEP * 2 + DELAY));
    assert_eq!(
        debounce.trailing_delay(start + SHORT_STEP * 2 + DELAY),
        None
    );
    assert!(!debounce.poll(start + SHORT_STEP * 2 + DELAY));
    assert!(debounce.trigger(start + SHORT_STEP * 2 + DELAY));
}

#[test]
fn 대기_시간이_지난_뒤에_온_호출은_밀린_실행을_합쳐_바로_실행한다() {
    let start = Instant::now();
    let mut debounce = LeadingTrailingDebounce::new(DELAY);
    assert!(debounce.trigger(start));
    assert!(!debounce.trigger(start + SHORT_STEP));
    assert!(debounce.trigger(start + SHORT_STEP + DELAY));
    assert_eq!(debounce.trailing_delay(start + SHORT_STEP + DELAY), None);
    assert!(!debounce.poll(start + SHORT_STEP + DELAY * 2));
}
