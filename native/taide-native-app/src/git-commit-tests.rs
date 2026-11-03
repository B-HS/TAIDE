use std::collections::BTreeMap;

use taide_model::git::{LogEntry, StagedDiffText};
use taide_model::ids::{ProjectId, ShellSlotId};
use taide_model::locale::ResolvedLocale;

use super::{DRAFT_LIMIT, Drafts, Request, State, notices, recent_commits, sanitize};

const MAIN: &str = "native-main";
const AUXILIARY: &str = "native-auxiliary-1";

#[test]
fn 초안은_빈_문자열이면_지우고_다시_쓰면_최신이_되며_여덟_프로젝트만_남긴다() {
    let mut drafts = Drafts::default();
    let projects: Vec<_> = (0..=DRAFT_LIMIT).map(|_| ProjectId::new()).collect();
    assert_eq!(drafts.read(&projects[0]), "");
    for (index, project) in projects.iter().take(DRAFT_LIMIT).enumerate() {
        drafts.write(project, format!("message {index}"));
    }
    drafts.write(&projects[0], "rewritten".into());
    drafts.write(&projects[DRAFT_LIMIT], "newest".into());
    assert_eq!(
        drafts.read(&projects[0]),
        "rewritten",
        "a rewrite is recent"
    );
    assert_eq!(drafts.read(&projects[1]), "", "the oldest is evicted");
    assert_eq!(drafts.read(&projects[2]), "message 2");
    assert_eq!(drafts.read(&projects[DRAFT_LIMIT]), "newest");

    drafts.write(&projects[2], String::new());
    assert_eq!(drafts.read(&projects[2]), "");
    let survivor = ProjectId::new();
    drafts.write(&survivor, "kept".into());
    assert_eq!(
        drafts.read(&projects[3]),
        "message 3",
        "an emptied draft frees its place instead of evicting another"
    );
}

#[test]
fn 창마다_초안이_따로이고_닫힌_보조_창의_초안은_버린다() {
    let mut state = State::default();
    let project = ProjectId::new();
    let (main_slot, auxiliary_slot) = (ShellSlotId::new(), ShellSlotId::new());
    state.set_windows(
        MAIN,
        [(auxiliary_slot.clone(), AUXILIARY.to_owned())].into_iter(),
    );
    assert_eq!(state.window(&main_slot, MAIN), MAIN);
    assert_eq!(state.window(&auxiliary_slot, MAIN), AUXILIARY);
    state.write(MAIN, &project, "main draft".into());
    state.write(AUXILIARY, &project, "auxiliary draft".into());
    assert_eq!(state.draft(MAIN, &project), "main draft");
    assert_eq!(state.draft(AUXILIARY, &project), "auxiliary draft");

    state.set_windows(MAIN, std::iter::empty());
    assert_eq!(state.window(&auxiliary_slot, MAIN), MAIN);
    assert_eq!(state.draft(MAIN, &project), "main draft");
    assert_eq!(state.draft(AUXILIARY, &project), "");
}

#[test]
fn 생성_요청은_slot마다_하나이고_보이지_않는_패널의_요청을_버린다() {
    let mut state = State::default();
    let (alpha, beta) = (ProjectId::new(), ProjectId::new());
    let (first, second) = (ShellSlotId::new(), ShellSlotId::new());
    let request = |project: &ProjectId, id: &str| Request {
        project: project.clone(),
        id: id.into(),
        owner: MAIN.into(),
    };
    state.begin(first.clone(), request(&alpha, "one"));
    state.begin(second.clone(), request(&beta, "two"));
    assert!(state.is_generating(&alpha, &first));
    assert!(!state.is_generating(&beta, &first));
    assert!(state.finish("missing").is_none());

    let abandoned = state.abandon(|project, slot| *project == beta && *slot == second);
    assert_eq!(
        abandoned
            .iter()
            .map(|request| request.id.as_str())
            .collect::<Vec<_>>(),
        ["one"]
    );
    assert!(!state.is_generating(&alpha, &first));
    let (slot, finished) = state.finish("two").unwrap();
    assert_eq!((slot, finished.project), (second.clone(), beta.clone()));
    assert!(state.finish("two").is_none());

    state.begin(second.clone(), request(&beta, "three"));
    assert_eq!(
        state.take(&second).map(|request| request.id),
        Some("three".into())
    );
    assert!(state.take(&second).is_none());
}

#[test]
fn 응답_정리는_코드펜스와_바깥_따옴표_한_쌍을_지우고_앞뒤_공백을_자른다() {
    for (raw, expected) in [
        ("  feat: add thing \n", "feat: add thing"),
        ("```\nfeat: fenced\n```", "feat: fenced"),
        ("```text\n\"feat: both\"\n```", "feat: both"),
        ("\"feat: double\"", "feat: double"),
        ("'feat: single'", "feat: single"),
        ("`feat: backtick`", "feat: backtick"),
        ("\u{201C} feat: curly \u{201D}", "feat: curly"),
        ("\u{2018}feat: single curly\u{2019}", "feat: single curly"),
        ("\"\"feat: only one pair\"\"", "\"feat: only one pair\""),
        ("\"unbalanced", "\"unbalanced"),
        ("\"", "\""),
        ("\"\"", ""),
        ("```\n\n```", ""),
        ("   ", ""),
    ] {
        assert_eq!(sanitize(raw), expected, "{raw:?}");
    }
}

#[test]
fn 최근_커밋_요약은_앞_스무_개의_짧은_해시와_요약을_줄로_잇는다() {
    let entry = |index: usize| LogEntry {
        id: format!("{index:040x}"),
        parents: Vec::new(),
        summary: format!("summary {index}"),
        author: "Synthetic".into(),
        time_unix: 0.0,
        refs: Vec::new(),
    };
    let log: Vec<_> = (0..25).map(entry).collect();
    let summary = recent_commits(&log);
    let lines: Vec<_> = summary.lines().collect();
    assert_eq!(lines.len(), 20);
    assert_eq!(lines[0], "0000000 summary 0");
    assert_eq!(lines[19], "0000000 summary 19");
    assert_eq!(recent_commits(&[]), "");
    let short = LogEntry {
        id: "abc".into(),
        ..entry(1)
    };
    assert_eq!(recent_commits(&[short]), "abc summary 1");
}

#[test]
fn 생성_알림_설명은_대체_잘림_제외_순서로_가운뎃점을_잇는다() {
    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        warnings: Vec::new(),
        messages: BTreeMap::from([
            (
                "git.commitMessageUsedUnstaged".to_owned(),
                "unstaged".to_owned(),
            ),
            (
                "git.commitMessageDiffTruncated".to_owned(),
                "truncated".to_owned(),
            ),
            (
                "git.commitMessageFilesSkipped".to_owned(),
                "{{count}} skipped".to_owned(),
            ),
        ]),
    };
    let diff = |used_fallback, truncated, skipped: &[&str]| StagedDiffText {
        diff_text: String::new(),
        truncated,
        skipped_files: skipped.iter().map(|path| (*path).to_owned()).collect(),
        used_fallback,
    };
    assert_eq!(notices(&locale, &diff(false, false, &[])), None);
    assert_eq!(
        notices(&locale, &diff(true, true, &["a", "b"])).as_deref(),
        Some("unstaged · truncated · 2 skipped")
    );
    assert_eq!(
        notices(&locale, &diff(false, true, &[])).as_deref(),
        Some("truncated")
    );
}
