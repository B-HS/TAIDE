use std::time::{Duration, Instant};

use super::{Conflicts, REPARSE_DELAY, Region, Resolution, parse, resolve};

const SINGLE: [&str; 8] = [
    "line1",
    "<<<<<<< HEAD",
    "ours A",
    "ours B",
    "=======",
    "theirs A",
    ">>>>>>> feature",
    "line8",
];
const TWO: [&str; 13] = [
    "a",
    "<<<<<<< HEAD",
    "ours1",
    "=======",
    "theirs1",
    ">>>>>>> feature",
    "b",
    "<<<<<<< HEAD",
    "ours2",
    "=======",
    "theirs2",
    ">>>>>>> feature",
    "c",
];
const DIFF3: [&str; 9] = [
    "x",
    "<<<<<<< HEAD",
    "ours",
    "||||||| merged common ancestors",
    "base",
    "=======",
    "theirs",
    ">>>>>>> feature",
    "y",
];

fn region(start: u32, base: Option<u32>, separator: u32, end: u32) -> Region {
    Region {
        start,
        base,
        separator,
        end,
    }
}

#[test]
fn 마커_파싱은_원본_conflict_marker_사례와_같다() {
    assert!(parse("a\nb\nc").is_empty());
    assert_eq!(parse(&SINGLE.join("\n")), [region(2, None, 5, 7)]);
    assert_eq!(
        parse(&TWO.join("\n")),
        [region(2, None, 4, 6), region(8, None, 10, 12)]
    );
    assert_eq!(parse(&DIFF3.join("\n")), [region(2, Some(4), 6, 8)]);
    assert!(parse(&["a", "<<<<<<< HEAD", "ours only, no end", "b"].join("\n")).is_empty());
    assert_eq!(
        parse(
            &[
                "<<<<<<< HEAD",
                "ours",
                "=======",
                "theirs",
                ">>>>>>> feature"
            ]
            .join("\r\n")
        ),
        [region(1, None, 3, 5)]
    );
    assert_eq!(
        parse(&["<<<<<<< a", "<<<<<<< b", "x", "=======", "y", ">>>>>>> c"].join("\n")),
        [region(2, None, 4, 6)],
        "새 시작 마커는 끝나지 않은 영역을 버립니다"
    );
    assert!(
        parse(&["<<<<<<< a", "x", "======= ", "y", ">>>>>>> c"].join("\n")).is_empty(),
        "구분선은 정확히 일곱 글자만"
    );
}

#[test]
fn 세_가지_해결_변환은_원본_사례와_같다() {
    let single = SINGLE.join("\n");
    let [first] = parse(&single)[..] else {
        panic!("한 영역")
    };
    assert_eq!(
        resolve(&single, first, Resolution::Current),
        ["line1", "ours A", "ours B", "line8"].join("\n")
    );
    assert_eq!(
        resolve(&single, first, Resolution::Incoming),
        ["line1", "theirs A", "line8"].join("\n")
    );
    assert_eq!(
        resolve(&single, first, Resolution::Both),
        ["line1", "ours A", "ours B", "theirs A", "line8"].join("\n")
    );
    let two = TWO.join("\n");
    assert_eq!(
        resolve(&two, parse(&two)[0], Resolution::Current),
        [
            "a",
            "ours1",
            "b",
            "<<<<<<< HEAD",
            "ours2",
            "=======",
            "theirs2",
            ">>>>>>> feature",
            "c"
        ]
        .join("\n")
    );
    let unterminated = "a\n<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> feature";
    assert_eq!(
        resolve(unterminated, parse(unterminated)[0], Resolution::Current),
        "a\nours"
    );
    let empty_ours = ["<<<<<<< HEAD", "=======", "theirs", ">>>>>>> feature"].join("\n");
    assert_eq!(
        resolve(&empty_ours, parse(&empty_ours)[0], Resolution::Current),
        ""
    );
    let diff3 = DIFF3.join("\n");
    assert_eq!(
        resolve(&diff3, parse(&diff3)[0], Resolution::Both),
        ["x", "ours", "theirs", "y"].join("\n")
    );
}

#[test]
fn 해결_중_문서가_짧아져_영역이_범위를_벗어나면_원문을_그대로_돌려준다() {
    let single = SINGLE.join("\n");
    let [stale] = parse(&single)[..] else {
        panic!("한 영역")
    };
    let shortened = ["line1", "line8"].join("\n");
    for resolution in [Resolution::Current, Resolution::Incoming, Resolution::Both] {
        assert_eq!(resolve(&shortened, stale, resolution), shortened);
    }
    let cut = SINGLE[..5].join("\n");
    assert_eq!(
        resolve(&cut, stale, Resolution::Incoming),
        cut,
        "끝 마커가 잘려도 원문"
    );
    assert_eq!(resolve("", stale, Resolution::Both), "");
}

#[test]
fn 충돌_파일은_처음_즉시_읽고_편집은_150ms_뒤에_다시_읽으며_충돌이_풀리면_잊는다() {
    let start = Instant::now();
    let mut conflicts = Conflicts::default();
    assert_eq!(conflicts.observe(false, 1, start), Ok(false));
    assert_eq!(conflicts.observe(true, 1, start), Ok(true));
    conflicts.parsed(1, &SINGLE.join("\n"));
    assert_eq!(conflicts.regions(), [region(2, None, 5, 7)]);
    assert_eq!(conflicts.region_at(2), Some(region(2, None, 5, 7)));
    assert_eq!(conflicts.region_at(3), None, "시작 줄만 누를 수 있습니다");
    assert_eq!(conflicts.observe(true, 1, start), Ok(false));
    assert_eq!(conflicts.observe(true, 2, start), Err(REPARSE_DELAY));
    let half = REPARSE_DELAY / 2;
    assert_eq!(conflicts.observe(true, 3, start + half), Err(REPARSE_DELAY));
    assert_eq!(
        conflicts.observe(true, 3, start + REPARSE_DELAY),
        Err(half),
        "마지막 변경부터 다시 셉니다"
    );
    assert_eq!(
        conflicts.observe(true, 3, start + half + REPARSE_DELAY),
        Ok(true)
    );
    conflicts.parsed(3, "resolved\n");
    assert!(conflicts.regions().is_empty());
    assert_eq!(conflicts.observe(false, 4, start), Ok(false));
    assert_eq!(
        conflicts.observe(true, 4, start + Duration::from_secs(1)),
        Ok(true),
        "다시 충돌이면 즉시 읽습니다"
    );
}
