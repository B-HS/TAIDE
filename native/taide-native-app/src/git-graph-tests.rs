use std::collections::HashMap;

use eframe::egui::pos2;
use taide_model::git::{CommitFile, GitChangeKind, LogEntry, TagInfo};
use taide_model::layout::TabKind;

use super::{
    Edge, Mark, Node, compute_lanes, marks, max_lane, relative_time, short_hash, tags_targeting,
};

const NOW: f64 = 1_700_000_000.0;
const MINUTE: f64 = 60.0;
const HOUR: f64 = 60.0 * MINUTE;
const DAY: f64 = 24.0 * HOUR;
const MANY_PARENTS: usize = 13;

fn commit(id: &str, parents: &[&str]) -> LogEntry {
    LogEntry {
        id: id.into(),
        parents: parents.iter().map(|parent| (*parent).to_owned()).collect(),
        summary: format!("summary {id}"),
        author: "Synthetic".into(),
        time_unix: NOW,
        refs: Vec::new(),
    }
}

fn by_id<'a>(nodes: &'a [Node<'a>]) -> HashMap<&'a str, &'a Node<'a>> {
    nodes.iter().map(|node| (node.id, node)).collect()
}

#[test]
fn 선형_히스토리는_모두_레인_0을_차지한다() {
    let commits = [
        commit("c3", &["c2"]),
        commit("c2", &["c1"]),
        commit("c1", &[]),
    ];
    let nodes = compute_lanes(&commits);
    assert_eq!(
        nodes.iter().map(|node| node.lane).collect::<Vec<_>>(),
        [0, 0, 0]
    );
}

#[test]
fn 분기가_생기면_새_브랜치_팁은_새_레인을_받는다() {
    let commits = [
        commit("c2", &["c1"]),
        commit("b1", &["c1"]),
        commit("c1", &[]),
    ];
    let nodes = compute_lanes(&commits);
    let nodes = by_id(&nodes);
    assert_eq!(nodes["c2"].lane, 0);
    assert_eq!(nodes["b1"].lane, 1);
    assert!(!nodes["b1"].continues_from_above);
    assert_eq!(nodes["c1"].lane, 0);
}

#[test]
fn 머지_커밋의_두번째_부모는_새_레인을_열고_합류하면_레인이_회수된다() {
    let commits = [
        commit("m", &["c2", "b1"]),
        commit("c2", &["c1"]),
        commit("b1", &["c1"]),
        commit("c1", &[]),
    ];
    let nodes = compute_lanes(&commits);
    let by = by_id(&nodes);
    assert_eq!(by["m"].lane, 0);
    assert_eq!(
        by["m"].edges,
        [
            Edge {
                to_lane: 0,
                parent: "c2"
            },
            Edge {
                to_lane: 1,
                parent: "b1"
            },
        ]
    );
    assert_eq!(by["b1"].lane, 1);
    assert_eq!(
        by["b1"].edges,
        [Edge {
            to_lane: 0,
            parent: "c1"
        }]
    );
    assert_eq!(by["c1"].lane, 0);
    assert!(by["c1"].passthrough.is_empty());
}

#[test]
fn 레인_수가_12를_넘어도_레인_번호를_계속_부여한다() {
    let parents: Vec<String> = (0..MANY_PARENTS).map(|index| format!("p{index}")).collect();
    let parent_ids: Vec<&str> = parents.iter().map(String::as_str).collect();
    let commits: Vec<_> = std::iter::once(commit("m", &parent_ids))
        .chain(parent_ids.iter().map(|id| commit(id, &[])))
        .collect();
    let nodes = compute_lanes(&commits);
    let by = by_id(&nodes);
    assert_eq!(by["p12"].lane, 12);
    assert_eq!(by["p11"].lane, 11);
    assert_eq!(max_lane(&nodes), 12);
}

#[test]
fn 레인이_회수된_뒤에는_재사용된다() {
    let commits = [
        commit("m", &["c2", "b1"]),
        commit("c2", &["c1"]),
        commit("b1", &["c1"]),
        commit("c1", &["c0"]),
        commit("d1", &["c0"]),
        commit("c0", &[]),
    ];
    let nodes = compute_lanes(&commits);
    assert_eq!(by_id(&nodes)["d1"].lane, 1);
}

#[test]
fn 행_그림은_지나가는_선_위_절반_간선_곡선_노드_순서로_레인_좌표에_그린다() {
    let commits = [
        commit("m", &["c2", "b1"]),
        commit("c2", &["c1"]),
        commit("b1", &["c1"]),
        commit("c1", &[]),
    ];
    let nodes = compute_lanes(&commits);
    let origin = pos2(8.0, 100.0);
    assert_eq!(
        marks(&nodes[0], origin),
        [
            Mark::Line {
                from: pos2(15.0, 112.0),
                to: pos2(15.0, 124.0),
                lane: 0
            },
            Mark::Curve {
                points: [
                    pos2(15.0, 112.0),
                    pos2(15.0, 124.0),
                    pos2(29.0, 112.0),
                    pos2(29.0, 124.0)
                ],
                lane: 0
            },
            Mark::Node {
                center: pos2(15.0, 112.0),
                lane: 0
            },
        ]
    );
    assert_eq!(
        marks(&nodes[1], origin),
        [
            Mark::Line {
                from: pos2(29.0, 100.0),
                to: pos2(29.0, 124.0),
                lane: 1
            },
            Mark::Line {
                from: pos2(15.0, 100.0),
                to: pos2(15.0, 112.0),
                lane: 0
            },
            Mark::Line {
                from: pos2(15.0, 112.0),
                to: pos2(15.0, 124.0),
                lane: 0
            },
            Mark::Node {
                center: pos2(15.0, 112.0),
                lane: 0
            },
        ]
    );
    assert_eq!(
        marks(&nodes[2], origin)[2],
        Mark::Curve {
            points: [
                pos2(29.0, 112.0),
                pos2(29.0, 124.0),
                pos2(15.0, 112.0),
                pos2(15.0, 124.0)
            ],
            lane: 1
        }
    );
    assert_eq!(
        marks(&nodes[3], origin),
        [
            Mark::Line {
                from: pos2(15.0, 100.0),
                to: pos2(15.0, 112.0),
                lane: 0
            },
            Mark::Node {
                center: pos2(15.0, 112.0),
                lane: 0
            },
        ]
    );
}

#[test]
fn 상대_시간은_일_시간_분_방금_순으로_고르고_미래는_방금이다() {
    assert_eq!(relative_time(NOW, NOW), ("git.timeJustNow", None));
    assert_eq!(
        relative_time(NOW, NOW - MINUTE + 1.0),
        ("git.timeJustNow", None)
    );
    assert_eq!(
        relative_time(NOW, NOW - 5.0 * MINUTE),
        ("git.timeMinutesAgo", Some(5))
    );
    assert_eq!(
        relative_time(NOW, NOW - 2.0 * HOUR - MINUTE),
        ("git.timeHoursAgo", Some(2))
    );
    assert_eq!(
        relative_time(NOW, NOW - 3.0 * DAY - HOUR),
        ("git.timeDaysAgo", Some(3))
    );
    assert_eq!(relative_time(NOW, NOW + DAY), ("git.timeJustNow", None));
    assert_eq!(
        relative_time(NOW + 0.9, NOW - MINUTE),
        ("git.timeMinutesAgo", Some(1))
    );
}

#[test]
fn 태그_대상_필터와_짧은_해시는_원본_규칙을_따른다() {
    let tag = |name: &str, target: &str| TagInfo {
        name: name.into(),
        target: target.into(),
        message: None,
        annotated: false,
    };
    let tags = [tag("v1", "aaa"), tag("v2", "bbb"), tag("v3", "aaa")];
    assert_eq!(
        tags_targeting(&tags, "aaa")
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>(),
        ["v1", "v3"]
    );
    assert_eq!(tags_targeting(&tags, "ccc").count(), 0);
    assert_eq!(short_hash("0123456789abcdef"), "0123456");
    assert_eq!(short_hash("abc"), "abc");
}

#[test]
fn 상세_파일은_절대경로_commit_diff_탭과_이름_해시_제목을_만든다() {
    let mut entry = commit("0123456789abcdef", &["fedcba9876543210", "1111111"]);
    let file = |kind, orig: Option<&str>| CommitFile {
        path: "docs/new.txt".into(),
        abs_path: "/repo/docs/new.txt".into(),
        orig_path: orig.map(|_| "old.txt".to_owned()),
        orig_abs_path: orig.map(str::to_owned),
        kind,
    };
    let diff = |before: &str, parent: Option<&str>| TabKind::Diff {
        path: "/repo/docs/new.txt".into(),
        staged: false,
        compare_with: None,
        rev: Some("0123456789abcdef".into()),
        parent_rev: parent.map(str::to_owned),
        before_path: Some(before.into()),
    };
    assert_eq!(
        crate::git_commit_detail::file_tab(
            &entry,
            &file(GitChangeKind::Renamed, Some("/repo/old.txt"))
        ),
        (
            diff("/repo/old.txt", Some("fedcba9876543210")),
            "new.txt @ 0123456".to_owned()
        )
    );
    assert_eq!(
        crate::git_commit_detail::file_tab(&entry, &file(GitChangeKind::Renamed, None)).0,
        diff("/repo/docs/new.txt", Some("fedcba9876543210"))
    );
    entry.parents.clear();
    assert_eq!(
        crate::git_commit_detail::file_tab(
            &entry,
            &file(GitChangeKind::Modified, Some("/repo/old.txt"))
        )
        .0,
        diff("/repo/docs/new.txt", None)
    );
}
