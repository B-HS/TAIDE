use super::project_rail_tests::{Fixture, insert_project, isolated_state, launch, project_ref};
use super::*;
use taide_model::layout::SplitDir;
use taide_model::project::ShellSlotTree;

const DEADLINE: Duration = Duration::from_secs(10);
const FRAME_SIZE: egui::Vec2 = egui::vec2(1000.0, 700.0);
const FRAME_SECONDS: f64 = 1.0 / 60.0;
const HALF: f32 = 50.0;
const SETTLE_FRAMES: usize = 5;
const BEFORE_DELAY_SECONDS: f64 = 0.3;
const AFTER_DELAY_SECONDS: f64 = 0.45;
const MOTION_SETTLE_SECONDS: f64 = 1.0;
const TOOLTIP_FONT_SIZE: f32 = 12.0;
const TOOLTIP_ICON_SIDE: f32 = 12.0;
const SECONDARY_OPACITY: f32 = 0.7;
const BANNER_STACK_OFFSET: f32 = 32.0;
const BANNER_BACKGROUND_OPACITY: f32 = 0.15;
const UNDERLINE_WIDTH: f32 = 1.0;
const ALPHA_FILE: &str = "readme.md";
const BETA_FILE: &str = "notes.txt";
const BROKEN_JSON: &str = "{";
const THEME_ID: &str = "synthetic-broken-theme";
const LOCALE_ID: &str = "xx";
const THEME_JSON: &str = r#"{"version":1,"id":"synthetic-broken-theme","name":"Synthetic","type":"dark","extends":"vscode-dark-modern"}"#;
const LOCALE_JSON: &str =
    r#"{"version":1,"id":"xx","name":"Synthetic","extends":"en","messages":{}}"#;
const TOAST_TEXT: &str = "synthetic toast";
const AWAY: egui::Pos2 = egui::pos2(900.0, 600.0);

struct Clock(f64);

impl Clock {
    fn frame(
        &mut self,
        application: &mut NativeApplication,
        context: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        self.0 += FRAME_SECONDS;
        eframe::App::logic(application, context, &mut eframe::Frame::_new_kittest());
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, FRAME_SIZE)),
                time: Some(self.0),
                events,
                ..Default::default()
            },
            |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()),
        );
        output.textures_delta.clear();
        output
    }

    fn until(
        &mut self,
        application: &mut NativeApplication,
        context: &egui::Context,
        phase: &str,
        ready: impl Fn(&mut NativeApplication, &egui::FullOutput) -> bool,
    ) -> egui::FullOutput {
        let deadline = Instant::now() + DEADLINE;
        loop {
            let output = self.frame(application, context, Vec::new());
            if ready(application, &output) {
                return output;
            }
            assert!(Instant::now() < deadline, "phase={phase}");
            std::thread::yield_now();
        }
    }

    fn click(
        &mut self,
        application: &mut NativeApplication,
        context: &egui::Context,
        position: egui::Pos2,
    ) {
        let button = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.frame(
            application,
            context,
            vec![egui::Event::PointerMoved(position), button(true)],
        );
        self.frame(application, context, vec![button(false)]);
    }
}

fn label(application: &NativeApplication, key: &str) -> String {
    presentation::message(&application.locale, key, &[])
}

fn text_shapes<'a>(
    output: &'a egui::FullOutput,
    text: &'a str,
) -> impl Iterator<Item = (usize, &'a egui::epaint::TextShape)> + 'a {
    output
        .shapes
        .iter()
        .enumerate()
        .filter_map(move |(index, clipped)| match &clipped.shape {
            egui::Shape::Text(shape) if shape.galley.text() == text => Some((index, shape)),
            _ => None,
        })
}

fn has_text(output: &egui::FullOutput, text: &str) -> bool {
    text_shapes(output, text).next().is_some()
}

fn text_rect(shape: &egui::epaint::TextShape) -> egui::Rect {
    shape.galley.rect.translate(shape.pos.to_vec2())
}

fn rect_of(node: &egui::accesskit::Node) -> Option<egui::Rect> {
    node.bounds().map(|bounds| {
        egui::Rect::from_min_max(
            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
        )
    })
}

fn nodes(
    output: &egui::FullOutput,
    role: egui::accesskit::Role,
) -> Vec<(egui::accesskit::NodeId, &egui::accesskit::Node)> {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .map(|update| {
            update
                .nodes
                .iter()
                .filter(|(_, node)| node.role() == role)
                .map(|(id, node)| (*id, node))
                .collect()
        })
        .unwrap_or_default()
}

fn node(
    output: &egui::FullOutput,
    role: egui::accesskit::Role,
    label: &str,
    within: egui::Rect,
) -> Option<(egui::accesskit::NodeId, egui::Rect, bool)> {
    nodes(output, role).into_iter().find_map(|(id, node)| {
        let rect = rect_of(node)?;
        (node.label() == Some(label) && within.contains_rect(rect))
            .then(|| (id, rect, node.is_selected() == Some(true)))
    })
}

fn labeled(
    output: &egui::FullOutput,
    label: &str,
    within: egui::Rect,
) -> Option<(egui::accesskit::NodeId, egui::Rect)> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .find_map(|(id, node)| {
            let rect = rect_of(node)?;
            (node.label() == Some(label) && within.contains_rect(rect)).then_some((*id, rect))
        })
}

fn left_half() -> egui::Rect {
    egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(FRAME_SIZE.x / 2.0, FRAME_SIZE.y),
    )
}

#[test]
fn 실제_app_사이드바는_네_보기를_slot별로_전환하고_검색과_git은_빈_본문만_보인다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-explorer-views-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    let roots = [
        fixture.directory.join("alpha"),
        fixture.directory.join("beta"),
    ];
    for directory in [&data, &roots[0], &roots[1]] {
        std::fs::create_dir_all(directory).unwrap();
    }
    std::fs::write(roots[0].join(ALPHA_FILE), "").unwrap();
    std::fs::write(roots[1].join(BETA_FILE), "").unwrap();
    let projects = [ProjectId::new(), ProjectId::new()];
    let slots = [ShellSlotId::new(), ShellSlotId::new()];
    let names = ["alpha", "beta"];
    let state = isolated_state(data);
    for ((project, root), name) in projects.iter().zip(&roots).zip(names) {
        insert_project(&state, project, root, name, false);
    }
    {
        let mut session = state.session.write();
        session.projects = projects
            .iter()
            .zip(&roots)
            .zip(names)
            .map(|((project, root), name)| project_ref(project, root, name))
            .collect();
        session.active_project = Some(projects[1].clone());
        session.shell_slots = Some(ShellSlotTree::Split {
            dir: SplitDir::Horizontal,
            children: projects
                .iter()
                .zip(&slots)
                .map(|(project, slot)| ShellSlotTree::Leaf {
                    slot_id: slot.clone(),
                    project_id: project.clone(),
                })
                .collect(),
            sizes: vec![HALF, HALF],
        });
        session.focused_shell_slot = Some(slots[1].clone());
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let mut clock = Clock(0.0);
    let output = clock.until(application, &context, "trees", |_, output| {
        has_text(output, ALPHA_FILE) && has_text(output, BETA_FILE)
    });
    let switcher = label(application, "explorer.sidebarSwitchLabel");
    let (_, header, _) = node(
        &output,
        egui::accesskit::Role::TabList,
        &switcher,
        left_half(),
    )
    .expect("the left slot's view switcher");
    let body = egui::Rect::from_min_max(
        egui::pos2(header.left(), header.bottom()),
        egui::pos2(
            header.right(),
            FRAME_SIZE.y - taide_native_ui::shell::STATUS_HEIGHT,
        ),
    );
    let view = |application: &mut NativeApplication, index: usize| {
        application
            .symbol_sidebars
            .entry(&projects[index], &slots[index])
            .view
    };
    for (key, expected) in [
        ("git.title", crate::symbol_sidebar::View::Git),
        ("search.title", crate::symbol_sidebar::View::Search),
    ] {
        let name = label(application, key);
        let output = clock.frame(application, &context, Vec::new());
        let (_, tab, _) = node(&output, egui::accesskit::Role::Tab, &name, header)
            .unwrap_or_else(|| panic!("{key}"));
        clock.click(application, &context, tab.center());
        let output = clock.until(application, &context, key, |application, _| {
            view(application, 0) == expected
        });
        assert!(!has_text(&output, ALPHA_FILE), "{key}");
        assert!(
            has_text(&output, BETA_FILE),
            "{key}: the other slot keeps its files"
        );
        let output = clock.frame(application, &context, Vec::new());
        assert!(
            node(&output, egui::accesskit::Role::Tab, &name, header).is_some_and(|tab| tab.2),
            "{key}"
        );
        let stray = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape) if body.intersects(text_rect(shape)) => {
                    Some((shape.galley.text().to_owned(), text_rect(shape), tab))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(stray.is_empty(), "{key}: no placeholder text {stray:?}");
        assert_eq!(view(application, 1), crate::symbol_sidebar::View::Files);
    }
    let explorer = label(application, "explorer.title");
    let output = clock.frame(application, &context, Vec::new());
    let (_, tab, _) = node(&output, egui::accesskit::Role::Tab, &explorer, header).unwrap();
    clock.click(application, &context, tab.center());
    clock.until(application, &context, "files again", |_, output| {
        has_text(output, ALPHA_FILE)
    });
    assert_eq!(view(application, 0), crate::symbol_sidebar::View::Files);
}

#[test]
fn 실제_app은_테마와_로케일_로드_실패를_상단_배너로_쌓고_재시도로_다시_불러온다() {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-load-banner-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let context = launch(&mut fixture, isolated_state(data));
    let application = fixture.application.as_mut().unwrap();
    let mut clock = Clock(0.0);
    for _ in 0..SETTLE_FRAMES {
        clock.frame(application, &context, Vec::new());
    }
    let state = application.services.state.clone();
    let theme_file = state.paths.themes_dir().join(format!("{THEME_ID}.json"));
    let locale_file = state.paths.locales_dir().join(format!("{LOCALE_ID}.json"));
    for file in [&theme_file, &locale_file] {
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, BROKEN_JSON).unwrap();
    }
    {
        let mut settings = state.settings.write();
        settings.follow_system_theme = false;
        settings.theme_id = THEME_ID.into();
        settings.language = LOCALE_ID.into();
    }
    let theme_message = label(application, "theme.loadFailed");
    let locale_message = crate::status_error_banner::LOCALE_MESSAGE;
    let output = clock.until(application, &context, "both banners", |_, output| {
        has_text(output, locale_message) && has_text(output, &theme_message)
    });
    let error = presentation::color(&application.resolved_theme, "statusIndicator.error").unwrap();
    let mut backgrounds = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect)
                if rect.fill == error.gamma_multiply(BANNER_BACKGROUND_OPACITY) =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    backgrounds.sort_by(|left, right| left.top().total_cmp(&right.top()));
    assert_eq!(backgrounds.len(), 2, "{backgrounds:?}");
    assert_eq!(backgrounds[0].min, egui::Pos2::ZERO);
    assert_eq!(backgrounds[1].min, egui::pos2(0.0, BANNER_STACK_OFFSET));
    assert!(backgrounds.iter().all(|rect| rect.width() == FRAME_SIZE.x));
    let (_, locale_text) = text_shapes(&output, locale_message).next().unwrap();
    let (_, theme_text) = text_shapes(&output, &theme_message).next().unwrap();
    assert!(text_rect(locale_text).bottom() <= BANNER_STACK_OFFSET);
    assert!(text_rect(theme_text).top() >= BANNER_STACK_OFFSET);
    for text in [locale_text, theme_text] {
        assert_eq!(text.galley.job.sections[0].format.color, error);
        assert_eq!(
            text.galley.job.sections[0].format.font_id.size,
            TOOLTIP_FONT_SIZE
        );
    }
    let retry = label(application, "common.retry");
    let (_, retry_text) = text_shapes(&output, &retry).next().unwrap();
    assert_eq!(
        retry_text.galley.job.sections[0].format.underline,
        egui::Stroke::new(UNDERLINE_WIDTH, error)
    );
    let retries = |output: &egui::FullOutput| {
        nodes(output, egui::accesskit::Role::Button)
            .into_iter()
            .filter(|(_, node)| node.label() == Some(retry.as_str()))
            .filter_map(|(_, node)| rect_of(node))
            .collect::<Vec<_>>()
    };
    let buttons = retries(&output);
    assert_eq!(buttons.len(), 2, "{buttons:?}");
    assert!(
        application
            .status
            .as_deref()
            .is_none_or(|status| !status.contains(THEME_ID) && !status.contains(LOCALE_ID)),
        "the failures leave the status label: {:?}",
        application.status
    );

    std::fs::write(&theme_file, THEME_JSON).unwrap();
    taide_runtime::theme_actions::theme_get(&state, THEME_ID.into()).unwrap();
    for _ in 0..SETTLE_FRAMES {
        let output = clock.frame(application, &context, Vec::new());
        assert!(has_text(&output, &theme_message), "no reload without retry");
    }
    let theme_retry = buttons
        .iter()
        .find(|rect| rect.top() >= BANNER_STACK_OFFSET)
        .unwrap()
        .center();
    clock.click(application, &context, theme_retry);
    let output = clock.until(application, &context, "theme retried", |_, output| {
        !has_text(output, &theme_message)
    });
    assert!(has_text(&output, locale_message));
    let (_, locale_text) = text_shapes(&output, locale_message).next().unwrap();
    assert!(text_rect(locale_text).bottom() <= BANNER_STACK_OFFSET);

    std::fs::write(&locale_file, LOCALE_JSON).unwrap();
    taide_runtime::locale_actions::locale_get(&state, LOCALE_ID.into()).unwrap();
    let [locale_retry] = retries(&output)[..] else {
        panic!("only the locale banner is left")
    };
    clock.click(application, &context, locale_retry.center());
    clock.until(application, &context, "locale retried", |_, output| {
        !has_text(output, locale_message)
    });
    assert_eq!(application.locale.id, LOCALE_ID);
}

#[test]
fn 실제_app_rail_tooltip은_공통_채널로_400ms_지연과_focus로_열고_다줄_아이콘_외형과_toast_아래_층을_지킨다()
 {
    let mut fixture = Fixture {
        directory: std::env::temp_dir()
            .join(format!("taide-native-shell-tooltip-{}", ProjectId::new())),
        application: None,
    };
    let data = fixture.directory.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let root = fixture.directory.join("alpha-gone");
    let project = ProjectId::new();
    let state = isolated_state(data);
    insert_project(&state, &project, &root, "alpha", true);
    {
        let mut session = state.session.write();
        let mut entry = project_ref(&project, &root, "alpha");
        entry.root_missing = true;
        session.projects = vec![entry];
        session.active_project = Some(project.clone());
    }
    let context = launch(&mut fixture, state);
    let application = fixture.application.as_mut().unwrap();
    let mut clock = Clock(0.0);
    let rail = egui::Rect::from_min_max(
        egui::Pos2::ZERO,
        egui::pos2(taide_native_ui::shell::PROJECT_RAIL_WIDTH, FRAME_SIZE.y),
    );
    let output = clock.until(application, &context, "rail", |_, output| {
        labeled(output, "alpha", rail).is_some()
    });
    let (button_node, button) = labeled(&output, "alpha", rail).unwrap();
    application
        .toasts
        .warning(TOAST_TEXT.into(), Instant::now());
    clock.until(application, &context, "toast", |_, output| {
        has_text(output, TOAST_TEXT)
    });
    let warning = label(application, "app.recentProjectRootMissing");
    let root_text = root.to_str().unwrap().to_owned();
    let expected_label = format!("alpha\n{warning}\n{root_text}");
    let tooltip = |output: &egui::FullOutput| {
        nodes(output, egui::accesskit::Role::Tooltip)
            .into_iter()
            .find_map(|(_, node)| {
                (node.label() == Some(expected_label.as_str())).then(|| rect_of(node))
            })
            .flatten()
    };

    clock.frame(
        application,
        &context,
        vec![egui::Event::PointerMoved(button.center())],
    );
    let start = clock.0;
    clock.0 = start + BEFORE_DELAY_SECONDS;
    let output = clock.frame(application, &context, Vec::new());
    assert!(tooltip(&output).is_none(), "the tooltip waits 400ms");
    clock.0 = start + AFTER_DELAY_SECONDS;
    let output = clock.frame(application, &context, Vec::new());
    assert!(tooltip(&output).is_some(), "the tooltip opens after 400ms");
    clock.0 += MOTION_SETTLE_SECONDS;
    let output = clock.frame(application, &context, Vec::new());
    let bounds = tooltip(&output).expect("the tooltip stays open");
    assert!(bounds.left() >= button.right(), "side right: {bounds:?}");
    let theme = &application.resolved_theme;
    let color = |key| presentation::color(theme, key).unwrap();
    assert!(output.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Rect(rect) if rect.fill == color("tooltip.background")
            && rect.stroke.color == color("tooltip.border")
            && rect.rect.contains_rect(bounds.shrink(1.0)))));
    let foreground = color("app.foreground");
    let (warning_index, warning_text) = text_shapes(&output, &warning)
        .find(|(_, shape)| bounds.contains_rect(text_rect(shape)))
        .expect("the warning row");
    let (_, root_line) = text_shapes(&output, &root_text)
        .find(|(_, shape)| bounds.contains_rect(text_rect(shape)))
        .expect("the root row");
    for (text, expected) in [
        (warning_text, foreground),
        (root_line, foreground.gamma_multiply(SECONDARY_OPACITY)),
    ] {
        let format = &text.galley.job.sections[0].format;
        assert_eq!(format.font_id.size, TOOLTIP_FONT_SIZE);
        assert_eq!(format.color, expected);
    }
    let warning_rect = text_rect(warning_text);
    assert!(text_rect(root_line).top() >= warning_rect.bottom());
    assert!(
        output.shapes.iter().any(|clipped| matches!(&clipped.shape,
        egui::Shape::Mesh(mesh) if {
            let icon = mesh.calc_bounds();
            icon.size() == egui::vec2(TOOLTIP_ICON_SIDE, TOOLTIP_ICON_SIDE)
                && icon.right() < warning_rect.left()
                && bounds.contains_rect(icon)
        })),
        "the warning row leads with its 12px glyph"
    );
    let (toast_index, _) = text_shapes(&output, TOAST_TEXT).next().unwrap();
    assert!(
        toast_index > warning_index,
        "the toast layer paints over the tooltip"
    );

    clock.frame(application, &context, vec![egui::Event::PointerMoved(AWAY)]);
    clock.0 += MOTION_SETTLE_SECONDS;
    let output = clock.frame(application, &context, Vec::new());
    assert!(tooltip(&output).is_none());
    clock.frame(
        application,
        &context,
        vec![egui::Event::AccessKitActionRequest(
            egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: button_node,
                data: None,
            },
        )],
    );
    let output = clock.frame(application, &context, Vec::new());
    assert!(
        tooltip(&output).is_some(),
        "keyboard focus opens it without the delay"
    );
}
