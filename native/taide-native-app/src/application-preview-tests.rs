use super::project_rail_tests::{
    Fixture, insert_project, isolated_state, launch, node_rect, pointer, project_ref, step,
};
use super::*;
use taide_model::app_event::AppEvent;
use taide_model::file::{FsChange, FsChangeKind};
use taide_model::layout::{PaneNode, SplitDir};
use taide_model::project::ShellSlotTree;

const DEADLINE: Duration = Duration::from_secs(10);
const TAB_BAR_BOTTOM: f32 =
    taide_native_ui::shell::TITLE_HEIGHT + taide_native_ui::shell::TAB_HEIGHT;
const FIRST_SIZE: [u32; 2] = [2, 1];
const NEXT_SIZE: [u32; 2] = [3, 1];
const PANE_SHARE: f32 = 100.0 / 3.0;
const IMAGE_FILE: &str = "pixels.png";
const SHEET_FILE: &str = "table.csv";
const FIRST_CELL: &str = "first-cell";
const NEXT_CELL: &str = "next-cell";

fn png(size: [u32; 2]) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::RgbaImage::from_pixel(size[0], size[1], image::Rgba([255, 0, 0, 255]))
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

fn file_tab(path: &std::path::Path) -> Tab {
    Tab {
        id: TabId::new(),
        kind: TabKind::File {
            path: path.to_str().unwrap().into(),
        },
        title: path.file_name().unwrap().to_str().unwrap().into(),
        pinned: false,
        preview: false,
        dirty: false,
        view_state: None,
    }
}

fn leaf(tabs: Vec<Tab>) -> PaneNode {
    PaneNode::Leaf {
        id: PaneId::new(),
        active: tabs.first().map(|tab| tab.id.clone()),
        tabs,
    }
}

fn tab_bar() -> egui::Rect {
    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(f32::INFINITY, TAB_BAR_BOTTOM))
}

fn body() -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(0.0, TAB_BAR_BOTTOM),
        egui::pos2(f32::INFINITY, f32::INFINITY),
    )
}

fn body_texts(output: &egui::FullOutput, text: &str) -> usize {
    output
        .shapes
        .iter()
        .filter(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(shape)
                if shape.galley.text() == text && shape.pos.y >= TAB_BAR_BOTTOM)
        })
        .count()
}

fn shows_text(output: &egui::FullOutput, text: &str) -> bool {
    body_texts(output, text) > 0
}

fn texture_meshes(output: &egui::FullOutput, texture: egui::TextureId) -> usize {
    output
        .shapes
        .iter()
        .filter(|clipped| clipped.shape.texture_id() == texture)
        .count()
}

fn open(
    name: &str,
    layout: impl FnOnce(&std::path::Path) -> PaneNode,
) -> (Fixture, egui::Context, ProjectId) {
    let mut fixture = Fixture {
        directory: std::env::temp_dir().join(format!("taide-native-{name}-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let root = fixture.directory.join("alpha");
    for directory in [&data, &root] {
        std::fs::create_dir_all(directory).unwrap();
    }
    let root_pane = layout(&root);
    let project = ProjectId::new();
    let slot = ShellSlotId::new();
    let state = isolated_state(data);
    insert_project(&state, &project, &root, "alpha", false);
    {
        let mut layouts = state.layouts.write();
        let layout = layouts.get_mut(&project).unwrap();
        if let PaneNode::Leaf { id, .. } = &root_pane {
            layout.focused_pane = id.clone();
        }
        if let PaneNode::Split { children, .. } = &root_pane
            && let Some(PaneNode::Leaf { id, .. }) = children.first()
        {
            layout.focused_pane = id.clone();
        }
        layout.root = root_pane;
        layout.shell_view.sidebar_collapsed = true;
    }
    {
        let mut session = state.session.write();
        session.projects = vec![project_ref(&project, &root, "alpha")];
        session.active_project = Some(project.clone());
        session.shell_slots = Some(ShellSlotTree::Leaf {
            slot_id: slot.clone(),
            project_id: project.clone(),
        });
        session.focused_shell_slot = Some(slot);
    }
    let context = launch(&mut fixture, state);
    (fixture, context, project)
}

fn wait(
    application: &mut NativeApplication,
    context: &egui::Context,
    phase: &str,
    ready: impl Fn(&NativeApplication, &egui::FullOutput) -> bool,
) -> egui::FullOutput {
    let deadline = Instant::now() + DEADLINE;
    loop {
        let output = step(application, context, Vec::new());
        if ready(application, &output) {
            return output;
        }
        assert!(
            Instant::now() < deadline,
            "phase={phase} labels={:?} texts={:?}",
            output
                .platform_output
                .accesskit_update
                .as_ref()
                .map(|update| update
                    .nodes
                    .iter()
                    .filter_map(|(_, node)| node.label().map(str::to_owned))
                    .collect::<Vec<_>>()),
            output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        );
        std::thread::yield_now();
    }
}

fn record_change(
    application: &NativeApplication,
    project: &ProjectId,
    kind: FsChangeKind,
    path: &str,
) {
    application
        .explorer_search_state
        .tree_changes
        .record(&AppEvent::FsChanged {
            project_id: project.clone(),
            change: FsChange {
                kind,
                paths: vec![path.into()],
                from_app: false,
            },
        });
}

#[test]
fn 실제_app_이미지_미리보기는_읽기_실패와_해석_실패를_원본_화면으로_나누고_외부_열기_실패를_알린다()
{
    const BROKEN_FILE: &str = "broken.png";
    const OUTSIDE_FILE: &str = "outside.png";
    let (mut fixture, context, _) = open("preview-failure", |root| {
        std::fs::write(root.join(BROKEN_FILE), "synthetic-not-an-image").unwrap();
        let outside = root.parent().unwrap().join(OUTSIDE_FILE);
        std::fs::write(&outside, png(FIRST_SIZE)).unwrap();
        leaf(vec![file_tab(&root.join(BROKEN_FILE)), file_tab(&outside)])
    });
    let root = fixture.directory.join("alpha");
    let broken = root.join(BROKEN_FILE).to_str().unwrap().to_owned();
    let outside = fixture
        .directory
        .join(OUTSIDE_FILE)
        .to_str()
        .unwrap()
        .to_owned();
    let application = fixture.application.as_mut().unwrap();
    let message = |key: &str| presentation::message(&application.locale, key, &[]);
    let not_supported = message("preview.notSupported");
    let open_externally = message("preview.openExternally");

    let output = wait(
        application,
        &context,
        "decode failure",
        |application, output| {
            matches!(
                application.preview_state.previews.error(&broken),
                Some(crate::preview::Failure::Decode(_))
            ) && shows_text(output, BROKEN_FILE)
        },
    );
    assert!(!shows_text(&output, &not_supported));
    assert!(node_rect(&output, &open_externally, body()).is_none());

    let tab = node_rect(&output, OUTSIDE_FILE, tab_bar()).unwrap();
    step(
        application,
        &context,
        pointer(tab.center(), egui::PointerButton::Primary),
    );
    let output = wait(
        application,
        &context,
        "read failure",
        |application, output| {
            matches!(
                application.preview_state.previews.error(&outside),
                Some(crate::preview::Failure::Read(_))
            ) && node_rect(output, &open_externally, body()).is_some()
        },
    );
    assert!(shows_text(&output, &not_supported));
    assert!(shows_text(&output, OUTSIDE_FILE));

    let expected = taide_infra::root_guard::resolve_owning_project(
        &application.services.state.projects.read(),
        std::path::Path::new(&outside),
    )
    .unwrap_err();
    let expected = crate::toast::describe_error(&application.locale, &expected);
    let button = node_rect(&output, &open_externally, body()).unwrap();
    step(
        application,
        &context,
        pointer(button.center(), egui::PointerButton::Primary),
    );
    wait(
        application,
        &context,
        "open failure toast",
        |application, _| {
            application
                .toasts
                .is_showing(taide_native_ui::toast::Kind::Error, &expected)
        },
    );
}

#[test]
fn 실제_app_html과_오디오_미리보기는_준비되기_전에_로딩_표시_없이_빈_배경을_둔다() {
    const PAGE_FILE: &str = "page.html";
    const SONG_FILE: &str = "song.mp3";
    const PANE_HALF: f32 = 100.0 / 2.0;
    let (mut fixture, context, _) = open("preview-pending", |root| {
        std::fs::write(root.join(PAGE_FILE), "<p>synthetic</p>").unwrap();
        std::fs::write(root.join(SONG_FILE), "synthetic-audio").unwrap();
        PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![
                leaf(vec![file_tab(&root.join(PAGE_FILE))]),
                leaf(vec![file_tab(&root.join(SONG_FILE))]),
            ],
            sizes: vec![PANE_HALF, PANE_HALF],
        }
    });
    let application = fixture.application.as_mut().unwrap();
    let message = |key: &str| presentation::message(&application.locale, key, &[]);
    let loading = message("common.loading");
    let not_supported = message("preview.notSupported");

    let output = wait(application, &context, "web pending", |application, _| {
        application
            .preview_state
            .web_previews
            .active_request()
            .is_some()
    });
    assert!(!shows_text(&output, &loading));
    assert!(!shows_text(&output, &not_supported));
}

#[test]
fn 실제_app_미리보기는_외부_수정과_삭제와_프로젝트_닫힘을_여러_뷰에_반영한다() {
    let (mut fixture, context, project) = open("preview-refresh", |root| {
        std::fs::write(root.join(IMAGE_FILE), png(FIRST_SIZE)).unwrap();
        std::fs::write(root.join(SHEET_FILE), format!("name\n{FIRST_CELL}\n")).unwrap();
        PaneNode::Split {
            id: PaneId::new(),
            dir: SplitDir::Horizontal,
            children: vec![
                leaf(vec![file_tab(&root.join(IMAGE_FILE))]),
                leaf(vec![file_tab(&root.join(IMAGE_FILE))]),
                leaf(vec![file_tab(&root.join(SHEET_FILE))]),
            ],
            sizes: vec![PANE_SHARE, PANE_SHARE, PANE_SHARE],
        }
    });
    let root = fixture.directory.join("alpha");
    let image = root.join(IMAGE_FILE).to_str().unwrap().to_owned();
    let sheet = root.join(SHEET_FILE).to_str().unwrap().to_owned();
    let application = fixture.application.as_mut().unwrap();
    let texture = |application: &NativeApplication| {
        application
            .preview_state
            .previews
            .texture(&image)
            .map(|texture| (texture.id(), texture.size()))
    };
    let pixel_size = |size: [u32; 2]| size.map(|side| side as usize);

    wait(
        application,
        &context,
        "first contents",
        |application, output| {
            texture(application).is_some_and(|(id, size)| {
                size == pixel_size(FIRST_SIZE) && texture_meshes(output, id) == 2
            }) && shows_text(output, FIRST_CELL)
        },
    );

    std::fs::write(root.join(IMAGE_FILE), png(NEXT_SIZE)).unwrap();
    std::fs::write(root.join(SHEET_FILE), format!("name\n{NEXT_CELL}\n")).unwrap();
    record_change(application, &project, FsChangeKind::Modified, &image);
    record_change(application, &project, FsChangeKind::Modified, &sheet);
    wait(
        application,
        &context,
        "external changes",
        |application, output| {
            texture(application).is_some_and(|(id, size)| {
                size == pixel_size(NEXT_SIZE) && texture_meshes(output, id) == 2
            }) && shows_text(output, NEXT_CELL)
                && !shows_text(output, FIRST_CELL)
        },
    );

    std::fs::remove_file(root.join(IMAGE_FILE)).unwrap();
    record_change(application, &project, FsChangeKind::Removed, &image);
    let not_supported = presentation::message(&application.locale, "preview.notSupported", &[]);
    let output = wait(
        application,
        &context,
        "external delete",
        |application, output| {
            matches!(
                application.preview_state.previews.error(&image),
                Some(crate::preview::Failure::Read(_))
            ) && body_texts(output, &not_supported) == 2
        },
    );
    assert_eq!(body_texts(&output, IMAGE_FILE), 2);
    assert_eq!(
        crate::tabs::tabs_in(&application.services.state.layouts.read()[&project].root).len(),
        3
    );

    std::fs::write(root.join(IMAGE_FILE), png(FIRST_SIZE)).unwrap();
    record_change(application, &project, FsChangeKind::Created, &image);
    wait(application, &context, "restored", |application, _| {
        texture(application).is_some_and(|(_, size)| size == pixel_size(FIRST_SIZE))
    });
    let sheet_tab = crate::tabs::tabs_in(&application.services.state.layouts.read()[&project].root)
        .into_iter()
        .find(|tab| matches!(&tab.kind, TabKind::File { path } if path == &sheet))
        .map(|tab| tab.id.clone())
        .unwrap();
    assert!(
        application
            .preview_state
            .spreadsheet_previews
            .workbook(&sheet_tab)
            .is_some()
    );
    application.close_project(project.clone());
    wait(application, &context, "project closed", |application, _| {
        texture(application).is_none()
            && application.preview_state.previews.error(&image).is_none()
            && application
                .preview_state
                .spreadsheet_previews
                .workbook(&sheet_tab)
                .is_none()
            && !application
                .services
                .state
                .projects
                .read()
                .contains_key(&project)
    });
}
