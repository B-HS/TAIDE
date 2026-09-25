use std::time::{Duration, Instant};

use taide_infra::shell_integration::CommandMarker;
use taide_terminal::command_clock::{TerminalCommandClock, TimedCommand};

const ONE_HOUR_MS: u32 = 3_600_000;
const SHORT_COMMAND_MS: u32 = 250;

#[test]
fn 시작_마커가_없으면_종료를_발행하지_않는다() {
    let clock = TerminalCommandClock::new();
    let now = Instant::now();

    assert_eq!(clock.record(CommandMarker::Finished { exit_code: Some(0) }, now), None);
}

#[test]
fn 시작부터_종료까지의_경과와_종료_상태를_그대로_돌려준다() {
    for exit_code in [None, Some(0), Some(1), Some(130), Some(-1)] {
        let clock = TerminalCommandClock::new();
        let start = Instant::now();
        assert_eq!(clock.record(CommandMarker::OutputStart, start), None);

        assert_eq!(
            clock.record(
                CommandMarker::Finished { exit_code },
                start + Duration::from_millis(u64::from(ONE_HOUR_MS))
            ),
            Some(TimedCommand {
                exit_code,
                duration_ms: ONE_HOUR_MS,
            })
        );
    }
}

#[test]
fn 중복_시작은_최근_시각으로_교체하고_종료는_한_번만_소비한다() {
    let clock = TerminalCommandClock::new();
    let start = Instant::now();
    clock.record(CommandMarker::OutputStart, start);
    clock.record(CommandMarker::OutputStart, start + Duration::from_millis(u64::from(ONE_HOUR_MS)));

    assert_eq!(
        clock.record(
            CommandMarker::Finished { exit_code: None },
            start + Duration::from_millis(u64::from(ONE_HOUR_MS + SHORT_COMMAND_MS))
        ),
        Some(TimedCommand {
            exit_code: None,
            duration_ms: SHORT_COMMAND_MS,
        })
    );
    assert_eq!(
        clock.record(
            CommandMarker::Finished { exit_code: Some(0) },
            start + Duration::from_millis(u64::from(ONE_HOUR_MS + SHORT_COMMAND_MS))
        ),
        None
    );
}

#[test]
fn 같은_시계에서_연속_명령을_독립적으로_측정한다() {
    const FIRST_END_MS: u32 = 100;
    const SECOND_START_MS: u32 = 200;
    const SECOND_END_MS: u32 = 950;

    let clock = TerminalCommandClock::new();
    let start = Instant::now();
    clock.record(CommandMarker::OutputStart, start);
    let first = clock.record(
        CommandMarker::Finished { exit_code: Some(0) },
        start + Duration::from_millis(u64::from(FIRST_END_MS)),
    );
    clock.record(
        CommandMarker::OutputStart,
        start + Duration::from_millis(u64::from(SECOND_START_MS)),
    );
    let second = clock.record(
        CommandMarker::Finished { exit_code: Some(2) },
        start + Duration::from_millis(u64::from(SECOND_END_MS)),
    );

    assert_eq!(first.map(|timed| timed.duration_ms), Some(FIRST_END_MS));
    assert_eq!(
        second,
        Some(TimedCommand {
            exit_code: Some(2),
            duration_ms: SECOND_END_MS - SECOND_START_MS,
        })
    );
}

#[test]
fn 긴_명령의_경과는_u32_최댓값으로_포화된다() {
    let clock = TerminalCommandClock::new();
    let start = Instant::now();
    clock.record(CommandMarker::OutputStart, start);

    assert_eq!(
        clock.record(
            CommandMarker::Finished { exit_code: Some(0) },
            start + Duration::from_millis(u64::from(u32::MAX) + 1)
        ),
        Some(TimedCommand {
            exit_code: Some(0),
            duration_ms: u32::MAX,
        })
    );
}
