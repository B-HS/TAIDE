use egui::{Color32, Rect, vec2};

use super::*;
use crate::project_rail::RailColors;

const SCREEN: [f32; 2] = [1024.0, 900.0];
const FRAME_SECONDS: f64 = 1.0 / 60.0;
const COLOR: Color32 = Color32::GRAY;
const LANE_RED: u8 = 240;
const SETTLE_FRAMES: usize = 10;
const OUTSIDE: egui::Pos2 = egui::pos2(4.0, 4.0);

fn colors() -> ShellColors {
    ShellColors {
        background: COLOR,
        foreground: COLOR,
        sidebar: COLOR,
        muted: COLOR,
        border: COLOR,
        focus_border: COLOR,
        tab_border: COLOR,
        active_tab: COLOR,
        inactive_tab: COLOR,
        active_indicator: COLOR,
        editor_background: COLOR,
        editor_foreground: COLOR,
        drop_target: COLOR,
        rail: RailColors {
            item_hover: COLOR,
            item_active: COLOR,
            accent: COLOR,
            agent_working: COLOR,
            agent_awaiting: COLOR,
            agent_idle: COLOR,
            agent_unknown: COLOR,
            lanes: std::array::from_fn(|lane| Color32::from_rgb(LANE_RED, lane as u8, 0)),
        },
    }
}

fn text_with_args(key: &str, arguments: &[(&str, &str)]) -> String {
    if arguments.is_empty() {
        return key.to_owned();
    }
    format!("{key} {arguments:?}")
}

#[derive(Default)]
struct Harness {
    context: egui::Context,
    dialog: ProjectGroupDialog,
    delete: ProjectGroupDeleteDialog,
    time: f64,
}

impl Harness {
    fn new() -> Self {
        let harness = Self::default();
        harness.context.enable_accesskit();
        harness
    }

    fn frame(
        &mut self,
        events: Vec<egui::Event>,
    ) -> (Option<Submission>, Option<ProjectGroupId>, egui::FullOutput) {
        self.time += FRAME_SECONDS;
        let colors = colors();
        let (dialog, delete) = (&mut self.dialog, &mut self.delete);
        let mut submitted = None;
        let mut deleted = None;
        let mut output = self.context.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(SCREEN[0], SCREEN[1]),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| {
                submitted = dialog.show(ui.ctx(), &colors, &|key| key.to_owned());
                deleted = delete.show(ui.ctx(), &text_with_args);
            },
        );
        output.textures_delta.clear();
        (submitted, deleted, output)
    }

    /// Repaints until the open modal keeps the same rect.
    fn settle(&mut self, id: &str) -> egui::FullOutput {
        let mut previous = None;
        for _ in 0..SETTLE_FRAMES {
            let (_, _, output) = self.frame(Vec::new());
            let rect = self.context.memory(|memory| memory.area_rect(Id::new(id)));
            if rect.is_some() && rect == previous {
                return output;
            }
            previous = rect;
        }
        panic!("{id} layout did not settle");
    }

    fn click(
        &mut self,
        id: &str,
        label: &str,
    ) -> (Option<Submission>, Option<ProjectGroupId>, egui::FullOutput) {
        let output = self.settle(id);
        let (rect, _, _) = node(&output, label).unwrap_or_else(|| panic!("{label}"));
        self.frame(click(rect.center()))
    }
}

fn node(
    output: &egui::FullOutput,
    label: &str,
) -> Option<(Rect, Option<egui::accesskit::Toggled>, bool)> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .find_map(|(_, node)| {
            (node.label() == Some(label)).then(|| {
                node.bounds().map(|bounds| {
                    (
                        Rect::from_min_max(
                            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
                            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
                        ),
                        node.toggled(),
                        node.is_disabled(),
                    )
                })
            })?
        })
}

fn is_focused(output: &egui::FullOutput, label: &str) -> bool {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .is_some_and(|update| {
            update
                .nodes
                .iter()
                .any(|(id, node)| *id == update.focus && node.label() == Some(label))
        })
}

fn has_text(output: &egui::FullOutput, needle: &str) -> bool {
    output.shapes.iter().any(
        |clipped| matches!(&clipped.shape, egui::Shape::Text(text) if text.galley.text() == needle),
    )
}

fn click(position: egui::Pos2) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        })
        .fold(
            vec![egui::Event::PointerMoved(position)],
            |mut events, event| {
                events.push(event);
                events
            },
        )
}

fn key(key: Key) -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        })
        .collect()
}

fn group(name: &str, color: Option<&str>) -> ProjectGroup {
    ProjectGroup {
        id: ProjectGroupId::new(),
        name: name.into(),
        color: color.map(str::to_owned),
        members: Vec::new(),
        collapsed: false,
    }
}

#[test]
fn 그룹_이름은_입력중_제어문자만_빼고_확정때_trim한_뒤_40코드포인트로_자른다() {
    let long = "가".repeat(PROJECT_GROUP_NAME_MAX_CODEPOINTS + 5);
    assert_eq!(clamp_project_group_name(" a\u{7}b "), " ab ");
    assert_eq!(
        clamp_project_group_name(&long).chars().count(),
        PROJECT_GROUP_NAME_MAX_CODEPOINTS
    );
    assert_eq!(normalize_project_group_name("\n  팀 이름 \t"), "팀 이름");
    assert_eq!(normalize_project_group_name(" \u{1b} "), "");
    let padded = format!("  {long}");
    assert_eq!(
        normalize_project_group_name(&padded),
        "가".repeat(PROJECT_GROUP_NAME_MAX_CODEPOINTS)
    );
}

#[test]
fn 그룹_수정은_이름과_해석된_색이_바뀐_축만_순서대로_쓴다() {
    let stored = group("team", Some(" lane3 "));
    assert!(GroupEdit::new(&stored, "team", Some("lane3")).is_empty());
    assert_eq!(
        GroupEdit::new(&stored, "renamed", Some("lane3")),
        GroupEdit {
            name: Some("renamed".into()),
            color: None,
        }
    );
    assert_eq!(
        GroupEdit::new(&stored, "team", None),
        GroupEdit {
            name: None,
            color: Some(None),
        }
    );
    let invalid = group("team", Some("lane99"));
    assert!(GroupEdit::new(&invalid, "team", None).is_empty());
    assert_eq!(
        GroupEdit::new(&invalid, "team", Some("lane5")).color,
        Some(Some("lane5".into()))
    );
}

#[test]
fn 새_그룹_dialog는_빈칸으로_열려_이름_자동focus와_빈이름_비활성과_trim한_이름_색을_만든다() {
    let mut harness = Harness::new();
    let project = ProjectId::new();
    harness.dialog.create(project.clone());
    let output = harness.settle(DIALOG_ID);
    assert!(has_text(&output, "projectGroup.createTitle"));
    assert!(has_text(&output, "projectGroup.nameField"));
    assert!(has_text(&output, "projectGroup.color"));
    let (_, _, is_disabled) = node(&output, "projectGroup.create").unwrap();
    assert!(is_disabled, "an empty name cannot be created");
    let swatches: Vec<_> = (1..=PROJECT_COLOR_TOKENS.len())
        .map(|index| node(&output, &format!("projectGroup.color {index}")).unwrap())
        .collect();
    assert!(
        swatches
            .iter()
            .all(|(_, toggled, _)| *toggled == Some(egui::accesskit::Toggled::False))
    );
    let (submitted, _, _) = harness.frame(key(Key::Enter));
    assert!(submitted.is_none());

    harness.frame(vec![egui::Event::Text("   ".into())]);
    let output = harness.settle(DIALOG_ID);
    assert!(
        node(&output, "projectGroup.create").unwrap().2,
        "a blank name normalizes to nothing"
    );
    harness.frame(vec![egui::Event::Text("팀 a".into())]);
    let (submitted, _, _) = harness.frame(key(Key::Enter));
    assert_eq!(
        submitted,
        Some(Submission {
            target: Target::Create(project.clone()),
            name: "팀 a".into(),
            color: None,
        }),
        "Enter in the name confirms the trimmed name"
    );
    let output = harness.settle(DIALOG_ID);
    assert!(
        node(&output, "projectGroup.create").unwrap().2,
        "a pending write cannot be sent twice"
    );
    let (submitted, _, _) = harness.frame(key(Key::Enter));
    assert!(submitted.is_none());
    harness.dialog.finish(false);
    assert!(harness.dialog.is_open(), "a failed write keeps the dialog");
    harness.click(DIALOG_ID, "projectGroup.color 4");
    let output = harness.settle(DIALOG_ID);
    assert_eq!(
        node(&output, "projectGroup.color 4").unwrap().1,
        Some(egui::accesskit::Toggled::True)
    );
    let (submitted, _, _) = harness.click(DIALOG_ID, "projectGroup.create");
    assert_eq!(
        submitted,
        Some(Submission {
            target: Target::Create(project.clone()),
            name: "팀 a".into(),
            color: Some("lane4"),
        })
    );
    harness.dialog.finish(true);
    assert!(!harness.dialog.is_open());
    harness.dialog.finish(false);
    assert!(!harness.dialog.is_open());

    harness.dialog.create(project);
    let output = harness.settle(DIALOG_ID);
    assert!(node(&output, "projectGroup.create").unwrap().2);
    assert_eq!(
        node(&output, "projectGroup.color 4").unwrap().1,
        Some(egui::accesskit::Toggled::False),
        "every open starts from the given values"
    );
}

#[test]
fn 그룹_수정_dialog는_저장값을_다시_채우고_색_토글과_코드포인트_상한과_취소_escape로_닫힌다() {
    let mut harness = Harness::new();
    let long = "가".repeat(PROJECT_GROUP_NAME_MAX_CODEPOINTS + 3);
    let stored = group(&long, Some("lane3"));
    harness.dialog.edit(&stored);
    let output = harness.settle(DIALOG_ID);
    assert!(has_text(&output, "projectGroup.editTitle"));
    assert_eq!(
        node(&output, "projectGroup.color 3").unwrap().1,
        Some(egui::accesskit::Toggled::True)
    );
    let (_, _, is_disabled) = node(&output, "common.save").unwrap();
    assert!(!is_disabled);
    harness.frame(vec![egui::Event::Text("나".into())]);
    harness.click(DIALOG_ID, "projectGroup.color 3");
    let (submitted, _, _) = harness.click(DIALOG_ID, "common.save");
    assert_eq!(
        submitted,
        Some(Submission {
            target: Target::Edit(stored.id.clone()),
            name: "가".repeat(PROJECT_GROUP_NAME_MAX_CODEPOINTS),
            color: None,
        }),
        "the stored name is clamped, extra input is refused and the pressed swatch toggles off"
    );
    harness.dialog.finish(true);

    harness.dialog.edit(&group("team", Some("lane99")));
    let output = harness.settle(DIALOG_ID);
    assert!((1..=PROJECT_COLOR_TOKENS.len()).all(|index| {
        node(&output, &format!("projectGroup.color {index}"))
            .unwrap()
            .1
            == Some(egui::accesskit::Toggled::False)
    }));
    harness.click(DIALOG_ID, "common.cancel");
    assert!(!harness.dialog.is_open());
    harness.dialog.edit(&stored);
    harness.settle(DIALOG_ID);
    harness.frame(key(Key::Escape));
    assert!(!harness.dialog.is_open());
}

#[test]
fn 그룹_삭제_확인은_취소에서_시작하고_바깥_클릭으로_닫히지_않으며_삭제나_취소나_escape로_닫힌다() {
    let mut harness = Harness::new();
    let stored = group("team", None);
    harness.delete.open(&stored);
    let output = harness.settle(DELETE_DIALOG_ID);
    assert!(has_text(
        &output,
        r#"projectGroup.deleteConfirmTitle [("name", "team")]"#
    ));
    assert!(has_text(&output, "projectGroup.deleteConfirmDescription"));
    let output = harness.settle(DELETE_DIALOG_ID);
    assert!(is_focused(&output, "common.cancel"));
    let (_, deleted, _) = harness.frame(click(OUTSIDE));
    assert!(deleted.is_none());
    assert!(harness.delete.is_open(), "an outside click keeps the alert");
    let (_, deleted, _) = harness.click(DELETE_DIALOG_ID, "common.cancel");
    assert!(deleted.is_none());
    assert!(!harness.delete.is_open());

    harness.delete.open(&stored);
    harness.settle(DELETE_DIALOG_ID);
    let (_, deleted, _) = harness.frame(key(Key::Escape));
    assert!(deleted.is_none());
    assert!(!harness.delete.is_open());

    harness.delete.open(&stored);
    assert_eq!(harness.delete.group(), Some(&stored.id));
    let (_, deleted, _) = harness.click(DELETE_DIALOG_ID, "projectGroup.delete");
    assert_eq!(deleted, Some(stored.id.clone()));
    assert!(!harness.delete.is_open());
}
