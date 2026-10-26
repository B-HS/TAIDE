use super::*;
use crate::project_rail::RailColors;

const SCREEN: [f32; 2] = [1024.0, 900.0];
const FRAME_SECONDS: f64 = 1.0 / 60.0;
const COLOR: Color32 = Color32::GRAY;
const SIDEBAR: Color32 = Color32::from_rgb(11, 12, 13);
const MUTED: Color32 = Color32::from_rgb(21, 22, 23);
const LANE_RED: u8 = 240;
const LANE_THREE: usize = 2;
const SETTLE_FRAMES: usize = 10;

type Painted = Vec<(String, Rect, Color32)>;

fn colors() -> ShellColors {
    ShellColors {
        background: COLOR,
        foreground: COLOR,
        sidebar: SIDEBAR,
        muted: MUTED,
        border: COLOR,
        focus_border: COLOR,
        tab_border: COLOR,
        active_tab: COLOR,
        inactive_tab: COLOR,
        active_indicator: COLOR,
        editor_background: COLOR,
        editor_foreground: COLOR,
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

struct Harness {
    context: egui::Context,
    dialog: ProjectDisplayDialog,
    time: f64,
}

impl Harness {
    fn new() -> Self {
        let context = egui::Context::default();
        context.enable_accesskit();
        Self {
            context,
            dialog: ProjectDisplayDialog::default(),
            time: 0.0,
        }
    }

    fn frame(
        &mut self,
        events: Vec<egui::Event>,
    ) -> (
        Option<(ProjectId, ProjectDisplayPatch)>,
        egui::FullOutput,
        Painted,
    ) {
        self.time += FRAME_SECONDS;
        let colors = colors();
        let mut result = None;
        let mut painted = Vec::new();
        let dialog = &mut self.dialog;
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
                result = dialog.show(
                    ui.ctx(),
                    &colors,
                    &|key| key.to_owned(),
                    &mut |_, rect, icon, color| painted.push((format!("{icon:?}"), rect, color)),
                );
            },
        );
        output.textures_delta.clear();
        (result, output, painted)
    }

    /// Opens the dialog and lets the modal area finish its invisible sizing pass.
    fn open(&mut self, project: &ProjectRef) {
        self.dialog.open(project);
        let _ = self.settle();
    }

    /// Repaints until the centered modal area keeps the same rect, as a content height change
    /// re-centers it on the next frame.
    fn settle(&mut self) -> egui::FullOutput {
        let mut previous = None;
        for _ in 0..SETTLE_FRAMES {
            let (_, output, _) = self.frame(Vec::new());
            let rect = self
                .context
                .memory(|memory| memory.area_rect(Id::new(DIALOG_ID)));
            if rect.is_some() && rect == previous {
                return output;
            }
            previous = rect;
        }
        panic!("project display dialog layout did not settle");
    }

    fn click(&mut self, label: &str) -> Option<(ProjectId, ProjectDisplayPatch)> {
        let output = self.settle();
        let (rect, _, _) = node(&output, label).unwrap_or_else(|| panic!("{label}"));
        self.frame(click(rect.center())).0
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
            modifiers: egui::Modifiers::NONE,
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

fn project(display: ProjectDisplay) -> ProjectRef {
    ProjectRef {
        id: ProjectId::new(),
        root: "/synthetic/alpha".into(),
        name: "alpha".into(),
        display,
        root_missing: false,
    }
}

fn axes(patch: &ProjectDisplayPatch) -> [Option<&str>; 3] {
    [
        patch.icon.as_deref(),
        patch.label.as_deref(),
        patch.color.as_deref(),
    ]
}

#[test]
fn 열기는_저장된_표시로_필드를_다시_채우고_저장은_세_축과_대기_실패유지_성공닫힘을_지킨다() {
    let mut harness = Harness::new();
    let alpha = project(ProjectDisplay {
        icon: Some("rocket".into()),
        label: Some("  가나다라마".into()),
        color: Some("lane3".into()),
    });
    harness.open(&alpha);
    assert_eq!(harness.dialog.mode, DisplayMode::Label);
    assert_eq!(harness.dialog.label, "  가나");
    assert_eq!(harness.dialog.icon, "rocket");
    assert_eq!(harness.dialog.lane, Some(LANE_THREE));
    let (_, output, painted) = harness.frame(Vec::new());
    assert!(has_text(&output, "project.displayDialogTitle"));
    assert!(has_text(&output, "가나"));
    assert!(painted.is_empty(), "label preview draws text: {painted:?}");
    assert_eq!(
        node(&output, "project.displayModeLabel").unwrap().1,
        Some(egui::accesskit::Toggled::True)
    );
    assert_eq!(
        node(&output, "project.displayColor 3").unwrap().1,
        Some(egui::accesskit::Toggled::True)
    );
    let _ = harness.frame(Vec::new());
    assert!(
        harness
            .context
            .memory(|memory| memory.has_focus(Id::new(LABEL_ID)))
    );
    let (id, patch) = harness.click("common.save").expect("save submits");
    assert_eq!(id, alpha.id);
    assert_eq!(axes(&patch), [Some(""), Some("가나"), Some("lane3")]);
    let (_, output, _) = harness.frame(Vec::new());
    assert!(
        node(&output, "common.save").unwrap().2,
        "pending disables save"
    );
    assert!(harness.click("common.save").is_none());
    harness.dialog.finish(false);
    assert!(harness.dialog.is_open());
    assert_eq!(harness.dialog.label, "  가나");
    harness.dialog.finish(true);
    assert!(harness.dialog.is_open(), "only a pending save settles");
    assert!(harness.click("common.save").is_some());
    harness.dialog.finish(true);
    assert!(!harness.dialog.is_open());
    let (result, output, _) = harness.frame(Vec::new());
    assert!(result.is_none());
    assert!(!has_text(&output, "project.displayDialogTitle"));

    harness.open(&project(ProjectDisplay {
        icon: Some("constructor".into()),
        label: Some(" ".into()),
        color: Some("lane13".into()),
    }));
    assert_eq!(harness.dialog.mode, DisplayMode::Icon);
    assert_eq!(harness.dialog.icon, "folder");
    assert_eq!(harness.dialog.lane, None);
    let _ = harness.frame(Vec::new());
    let _ = harness.frame(key(Key::Escape));
    assert!(!harness.dialog.is_open());
}

#[test]
fn 라벨_모드는_제어문자와_4코드포인트를_자르고_빈_라벨이면_저장을_막으며_미리보기는_폴더로_돌아간다()
 {
    let mut harness = Harness::new();
    harness.open(&project(ProjectDisplay::default()));
    assert_eq!(harness.dialog.mode, DisplayMode::Default);
    let (_, output, painted) = harness.frame(Vec::new());
    assert!(!node(&output, "common.save").unwrap().2);
    assert!(
        painted
            .iter()
            .any(|(icon, rect, color)| icon == "Project(None)"
                && rect.width() == crate::project_rail::PROJECT_GLYPH_ICON_SIZE
                && *color == MUTED)
    );
    assert!(harness.click("project.displayModeLabel").is_none());
    let (_, output, painted) = harness.frame(Vec::new());
    assert!(node(&output, "common.save").unwrap().2, "empty label");
    assert!(
        painted.iter().any(|(icon, _, _)| icon == "Project(None)"),
        "empty label previews the folder"
    );
    assert!(
        harness
            .context
            .memory(|memory| memory.has_focus(Id::new(LABEL_ID)))
    );
    let _ = harness.frame(vec![egui::Event::Text("\u{7}A".into())]);
    assert_eq!(harness.dialog.label, "A");
    let _ = harness.frame(vec![egui::Event::Text(" B C".into())]);
    assert_eq!(harness.dialog.label, "A B ");
    let _ = harness.frame(vec![egui::Event::Text("D".into())]);
    assert_eq!(harness.dialog.label, "A B ");
    let (_, output, _) = harness.frame(Vec::new());
    assert!(has_text(&output, "A B"));
    assert!(!node(&output, "common.save").unwrap().2);
    let (_, patch) = harness.click("common.save").unwrap();
    assert_eq!(axes(&patch), [Some(""), Some("A B"), Some("")]);
    harness.dialog.finish(false);
    assert!(harness.click("project.displayModeDefault").is_none());
    let (_, patch) = harness.click("common.save").unwrap();
    assert_eq!(axes(&patch), [Some(""), Some(""), Some("")]);
}

#[test]
fn 아이콘_모드는_cmdk처럼_검색_정렬_첫항목_선택과_화살표_enter_클릭으로_고른다() {
    let mut harness = Harness::new();
    harness.open(&project(ProjectDisplay {
        icon: Some("rocket".into()),
        ..Default::default()
    }));
    let (_, output, painted) = harness.frame(Vec::new());
    let catalog = painted
        .iter()
        .filter(|(icon, rect, color)| {
            icon.starts_with("Project(Some") && rect.width() == ICON_GLYPH_SIZE && *color == COLOR
        })
        .count();
    assert_eq!(catalog, PROJECT_ICONS.len());
    assert_eq!(
        node(&output, "rocket").map(|(_, toggled, _)| toggled),
        Some(Some(egui::accesskit::Toggled::True))
    );
    assert_eq!(
        node(&output, "code").map(|(_, toggled, _)| toggled),
        Some(Some(egui::accesskit::Toggled::False))
    );
    assert_eq!(harness.dialog.cursor, 0, "cmdk selects the first item");
    let _ = harness.settle();
    let search = harness
        .context
        .read_response(Id::new(SEARCH_ID))
        .expect("icon search is rendered")
        .rect;
    let _ = harness.frame(click(search.center()));
    assert!(
        harness
            .context
            .memory(|memory| memory.has_focus(Id::new(SEARCH_ID)))
    );
    let _ = harness.frame(vec![egui::Event::Text("git".into())]);
    assert_eq!(icon_matches("git"), ["git-branch", "git-fork"]);
    assert_eq!(icon_matches("XML"), ["code-xml"]);
    assert_eq!(harness.dialog.cursor, 0);
    let _ = harness.frame(key(Key::ArrowDown));
    assert_eq!(harness.dialog.cursor, 1);
    let _ = harness.frame(key(Key::ArrowDown));
    assert_eq!(harness.dialog.cursor, 1, "cmdk does not loop by default");
    let _ = harness.frame(key(Key::Home));
    assert_eq!(harness.dialog.cursor, 0);
    let _ = harness.frame(key(Key::End));
    let _ = harness.frame(key(Key::Enter));
    assert_eq!(harness.dialog.icon, "git-fork");
    assert_eq!(harness.dialog.search, "git");
    assert!(harness.dialog.is_open(), "Enter chooses instead of saving");
    let _ = harness.frame(vec![egui::Event::Text("zzz".into())]);
    let (_, output, _) = harness.frame(Vec::new());
    assert!(has_text(&output, "palette.noResults"));
    let _ = harness.frame(key(Key::Enter));
    assert_eq!(harness.dialog.icon, "git-fork");
    assert!(harness.click("project.displayModeDefault").is_none());
    assert!(harness.click("project.displayModeIcon").is_none());
    assert!(
        harness.dialog.search.is_empty(),
        "remounted search is empty"
    );
    assert!(harness.click("globe").is_none());
    assert_eq!(harness.dialog.icon, "globe");
    assert!(harness.click("project.displayColor 12").is_none());
    let (_, output, painted) = harness.frame(Vec::new());
    assert_eq!(
        node(&output, "project.displayColor 12").unwrap().1,
        Some(egui::accesskit::Toggled::True)
    );
    let lane = Color32::from_rgb(LANE_RED, (PROJECT_COLOR_TOKENS.len() - 1) as u8, 0);
    assert!(
        painted
            .iter()
            .any(|(icon, rect, color)| icon == "Project(Some(\"globe\"))"
                && rect.width() == crate::project_rail::PROJECT_GLYPH_ICON_SIZE
                && *color == lane),
        "preview uses the chosen icon and lane: {painted:?}"
    );
    let (_, patch) = harness.click("common.save").unwrap();
    assert_eq!(axes(&patch), [Some("globe"), Some(""), Some("lane12")]);
    harness.dialog.finish(false);
    assert!(harness.click("project.displayColor 12").is_none());
    assert_eq!(harness.dialog.lane, None);
    assert!(harness.click("common.cancel").is_none());
    assert!(!harness.dialog.is_open());
}
