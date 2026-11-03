use super::*;

fn found(line: u32) -> SearchLineMatch {
    SearchLineMatch {
        line,
        column: 1,
        preview: "hello".into(),
        match_start: 0,
        match_end: 5,
        before: Vec::new(),
        after: Vec::new(),
    }
}

fn with_context(line: u32, before: &[&str], after: &[&str]) -> SearchLineMatch {
    SearchLineMatch {
        before: before.iter().map(|text| (*text).to_owned()).collect(),
        after: after.iter().map(|text| (*text).to_owned()).collect(),
        ..found(line)
    }
}

fn batch(path: &str, matches: Vec<SearchLineMatch>) -> SearchFileMatches {
    SearchFileMatches {
        path: path.into(),
        matches,
    }
}

fn group_all(batches: Vec<SearchFileMatches>) -> Accumulator {
    let mut accumulator = Accumulator::default();
    accumulator.append(batches);
    accumulator
}

fn texts(lines: &[String]) -> Vec<&str> {
    lines.iter().map(String::as_str).collect()
}

#[test]
fn 배치_하나가_그룹_하나가_된다() {
    let accumulator = group_all(vec![
        batch("src/a.ts", vec![found(1), found(2)]),
        batch("src/b.ts", vec![found(1)]),
    ]);
    let groups = accumulator.groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].path, "src/a.ts");
    assert_eq!(groups[0].matches.len(), 2);
    assert_eq!(groups[1].path, "src/b.ts");
    assert_eq!(groups[1].matches.len(), 1);
}

#[test]
fn 그룹은_배치가_도착한_순서를_유지한다() {
    let accumulator = group_all(vec![
        batch("src/z.ts", vec![found(1)]),
        batch("src/a.ts", vec![found(1)]),
    ]);
    let paths = accumulator
        .groups()
        .iter()
        .map(|group| group.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(paths, ["src/z.ts", "src/a.ts"]);
}

#[test]
fn 같은_경로가_두_번_도착하면_새_그룹을_만들지_않고_기존_그룹에_이어붙인다() {
    let mut accumulator = Accumulator::default();
    accumulator.append(vec![batch("src/a.ts", vec![found(1)])]);
    accumulator.append(vec![batch("src/a.ts", vec![found(5)])]);
    assert_eq!(accumulator.groups().len(), 1);
    let lines = accumulator.groups()[0]
        .matches
        .iter()
        .map(|entry| entry.line)
        .collect::<Vec<_>>();
    assert_eq!(lines, [1, 5]);
}

#[test]
fn 여러_번_append_해도_이전_배치의_그룹은_그대로_남는다() {
    let mut accumulator = Accumulator::default();
    accumulator.append(vec![batch("src/a.ts", vec![found(1)])]);
    let first = accumulator.groups()[0].clone();
    accumulator.append(vec![batch("src/b.ts", vec![found(1)])]);
    assert_eq!(accumulator.groups()[0], first);
    assert_eq!(accumulator.groups()[1].path, "src/b.ts");
}

#[test]
fn 누적_매치_수를_세어둔다() {
    let mut accumulator = Accumulator::default();
    accumulator.append(vec![batch("src/a.ts", vec![found(1), found(2)])]);
    accumulator.append(vec![batch("src/b.ts", vec![found(1)])]);
    assert_eq!(accumulator.total(), 3);
}

#[test]
fn 컨텍스트_줄을_그대로_보존한다() {
    let accumulator = group_all(vec![batch(
        "src/a.ts",
        vec![with_context(1, &["line before"], &["line after"])],
    )]);
    let entry = &accumulator.groups()[0].matches[0];
    assert_eq!(texts(&entry.before), ["line before"]);
    assert_eq!(texts(&entry.after), ["line after"]);
}

#[test]
fn 빈_배치_목록이면_기존_그룹을_그대로_둔다() {
    let mut accumulator = Accumulator::default();
    accumulator.append(Vec::new());
    assert!(accumulator.groups().is_empty());
    assert_eq!(accumulator.total(), 0);
}

#[test]
fn 인접한_매치의_컨텍스트_줄이_겹치면_겹치는_부분을_제거한다() {
    let accumulator = group_all(vec![batch(
        "src/a.ts",
        vec![
            with_context(
                10,
                &["line 8", "line 9"],
                &["line 11", "line 12", "line 13"],
            ),
            with_context(12, &["line 10", "line 11"], &["line 13", "line 14"]),
        ],
    )]);
    let matches = &accumulator.groups()[0].matches;
    assert_eq!(texts(&matches[0].before), ["line 8", "line 9"]);
    assert_eq!(texts(&matches[0].after), ["line 11"]);
    assert!(matches[1].before.is_empty());
    assert_eq!(texts(&matches[1].after), ["line 13", "line 14"]);
}

#[test]
fn 같은_줄에_여러_매치가_있으면_컨텍스트_줄을_한_번만_표시한다() {
    let accumulator = group_all(vec![batch(
        "src/a.ts",
        vec![
            with_context(5, &["line 4"], &["line 6"]),
            SearchLineMatch {
                column: 10,
                ..with_context(5, &["line 4"], &["line 6"])
            },
        ],
    )]);
    let matches = &accumulator.groups()[0].matches;
    assert_eq!(texts(&matches[0].before), ["line 4"]);
    assert!(matches[0].after.is_empty());
    assert!(matches[1].before.is_empty());
    assert_eq!(texts(&matches[1].after), ["line 6"]);
}

#[test]
fn 겹치지_않는_매치의_컨텍스트_줄은_그대로_보존한다() {
    let accumulator = group_all(vec![batch(
        "src/a.ts",
        vec![
            with_context(5, &["line 3", "line 4"], &["line 6", "line 7"]),
            with_context(20, &["line 18", "line 19"], &["line 21", "line 22"]),
        ],
    )]);
    let matches = &accumulator.groups()[0].matches;
    assert_eq!(texts(&matches[0].before), ["line 3", "line 4"]);
    assert_eq!(texts(&matches[0].after), ["line 6", "line 7"]);
    assert_eq!(texts(&matches[1].before), ["line 18", "line 19"]);
    assert_eq!(texts(&matches[1].after), ["line 21", "line 22"]);
}

#[test]
fn 이어붙인_그룹도_컨텍스트_중복_제거를_다시_적용한다() {
    let mut accumulator = Accumulator::default();
    accumulator.append(vec![batch(
        "src/a.ts",
        vec![with_context(10, &[], &["line 11", "line 12"])],
    )]);
    accumulator.append(vec![batch(
        "src/a.ts",
        vec![with_context(12, &["line 10", "line 11"], &[])],
    )]);
    let matches = &accumulator.groups()[0].matches;
    assert_eq!(texts(&matches[0].after), ["line 11"]);
    assert!(matches[1].before.is_empty());
}

fn path_group(path: &str, lines: &[u32]) -> Group {
    Group {
        path: path.into(),
        matches: lines.iter().map(|line| found(*line)).collect(),
    }
}

#[test]
fn 그룹_헤더와_매치_행을_하나의_목록으로_펼친다() {
    let groups = [
        path_group("src/a.ts", &[1, 2]),
        path_group("src/b.ts", &[7]),
    ];
    let rows = rows(&groups, &HashSet::new());
    assert_eq!(
        rows,
        [
            Row::Group {
                group: 0,
                collapsed: false
            },
            Row::Match { group: 0, index: 0 },
            Row::Match { group: 0, index: 1 },
            Row::Group {
                group: 1,
                collapsed: false
            },
            Row::Match { group: 1, index: 0 },
        ]
    );
}

#[test]
fn 접힌_그룹은_헤더만_남기고_매치_행을_내보내지_않는다() {
    let groups = [
        path_group("src/a.ts", &[1, 2]),
        path_group("src/b.ts", &[7]),
    ];
    let rows = rows(&groups, &HashSet::from(["src/a.ts".to_owned()]));
    assert_eq!(
        rows,
        [
            Row::Group {
                group: 0,
                collapsed: true
            },
            Row::Group {
                group: 1,
                collapsed: false
            },
            Row::Match { group: 1, index: 0 },
        ]
    );
}

#[test]
fn 행_id_는_경로_기준이라_파일_도착_순서가_달라도_안정적이다() {
    let ascending = [path_group("src/a.ts", &[1]), path_group("src/b.ts", &[1])];
    let reversed = [path_group("src/b.ts", &[1]), path_group("src/a.ts", &[1])];
    let ids = |groups: &[Group]| {
        rows(groups, &HashSet::new())
            .into_iter()
            .map(|row| row_id(groups, row))
            .collect::<HashSet<_>>()
    };
    assert_eq!(ids(&ascending), ids(&reversed));
}

#[test]
fn 같은_파일의_서로_다른_매치는_서로_다른_id_를_가진다() {
    let groups = [path_group("src/a.ts", &[1, 2, 3])];
    let rows = rows(&groups, &HashSet::new());
    let ids = rows
        .iter()
        .map(|row| row_id(&groups, *row))
        .collect::<HashSet<_>>();
    assert_eq!(ids.len(), rows.len());
}

#[test]
fn 컨텍스트_줄이_많은_매치는_더_높게_잡는다() {
    let plain = [path_group("src/a.ts", &[1])];
    let context = [Group {
        path: "src/b.ts".into(),
        matches: vec![with_context(5, &["a", "b"], &["c"])],
    }];
    let row = Row::Match { group: 0, index: 0 };
    assert!(estimate_height(&context, row) > estimate_height(&plain, row));
}

#[test]
fn 그룹_헤더_높이는_매치_개수와_무관하다() {
    let one = [path_group("src/a.ts", &[1])];
    let many = [path_group("src/b.ts", &[1, 2, 3, 4])];
    let row = Row::Group {
        group: 0,
        collapsed: false,
    };
    assert_eq!(estimate_height(&one, row), estimate_height(&many, row));
}

fn numbered_batches(count: usize, offset: usize) -> Vec<SearchFileMatches> {
    (0..count)
        .map(|index| {
            batch(
                &format!("/repo/src/file-{}.ts", offset + index),
                vec![found(1)],
            )
        })
        .collect()
}

#[test]
fn 새_경로_배치는_배치마다_그룹과_색인_하나씩만_더한다() {
    let mut accumulator = Accumulator::default();
    accumulator.append(numbered_batches(100, 0));
    assert_eq!(accumulator.groups().len(), 100);
    assert_eq!(accumulator.index_by_path.len(), 100);
    accumulator.append(numbered_batches(100, 100));
    assert_eq!(accumulator.groups().len(), 200);
    assert_eq!(accumulator.index_by_path.len(), 200);
    assert!(
        accumulator
            .groups()
            .iter()
            .enumerate()
            .all(|(index, group)| accumulator.index_by_path[&group.path] == index)
    );
}

#[test]
fn 같은_경로가_다시_오면_색인을_다시_쓰지_않고_그룹_수도_그대로다() {
    let mut accumulator = Accumulator::default();
    accumulator.append(numbered_batches(10, 0));
    accumulator.append(numbered_batches(3, 0));
    assert_eq!(accumulator.groups().len(), 10);
    assert_eq!(accumulator.index_by_path.len(), 10);
    assert_eq!(accumulator.groups()[0].matches.len(), 2);
    assert_eq!(accumulator.total(), 13);
}

#[test]
fn 빈_배치_목록은_색인을_건드리지_않는다() {
    let mut accumulator = Accumulator::default();
    accumulator.append(numbered_batches(2, 0));
    accumulator.append(Vec::new());
    assert_eq!(accumulator.index_by_path.len(), 2);
    assert_eq!(accumulator.total(), 2);
}

#[test]
fn utf16_오프셋은_bmp_밖_문자를_두_단위로_세고_서로게이트_안쪽은_다음_경계로_올린다() {
    let preview = "a😀b한c";
    assert_eq!(utf16_byte_offset(preview, 0), 0);
    assert_eq!(utf16_byte_offset(preview, 1), 1);
    assert_eq!(utf16_byte_offset(preview, 2), 5);
    assert_eq!(utf16_byte_offset(preview, 3), 5);
    assert_eq!(utf16_byte_offset(preview, 4), 6);
    assert_eq!(utf16_byte_offset(preview, 5), 9);
    assert_eq!(utf16_byte_offset(preview, 6), 10);
    assert_eq!(utf16_byte_offset(preview, 99), preview.len());
    assert_eq!(
        &preview[utf16_byte_offset(preview, 3)..utf16_byte_offset(preview, 5)],
        "b한"
    );
}

#[test]
fn 미리보기_공백은_한_칸으로_줄고_줄_앞_공백은_사라진다() {
    assert_eq!(
        collapse_whitespace(["    let  ", "x", "\t= 1"]),
        ["let ".to_owned(), "x".to_owned(), " = 1".to_owned()]
    );
}
