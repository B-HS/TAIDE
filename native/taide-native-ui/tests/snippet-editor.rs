use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};
use taide_model::{
    error::AppError,
    ids::{PaneId, ProjectId, TabId},
    locale::ResolvedLocale,
    paths::AppPaths,
    settings::Settings,
    snippet::SnippetFile,
    theme::ResolvedTheme,
};
use taide_native_ui::{
    settings_controls::Section,
    settings_owner::Owner,
    settings_view::{Appearance, Output, Views},
    snippet_edit::{Kind, Outcome, Reply},
    snippet_editor::Notice,
};
use taide_runtime::{AppState, locale_actions, theme_actions};

const SCREEN: [f32; 2] = [1000.0, 900.0];
const FRAME_STEP: f64 = 0.1;
const SLOW_FRAME_STEP: f64 = 0.25;
const GEOMETRY_TOLERANCE: f32 = 0.01;
const DIALOG_VIEWPORTS: [(f32, f32); 3] = [(600.0, 568.0), (640.0, 512.0), (1000.0, 512.0)];
const RESPONSIVE_BREAKPOINT: f32 = 640.0;
const DIALOG_INSETS: f32 = 50.0;
const FOOTER_HEIGHT: f32 = 36.0;
const SMALL_DIALOG_WIDTH: f32 = 320.0;
const FILE_GAP: f32 = 4.0;
const FILE_PADDING: f32 = 12.0;
const FILE_CONTENT_WIDTH: f32 = 223.0;
const EDITOR_NON_CARD_WIDTH: f32 = 304.0;
const INPUT_RADIUS: u8 = 4;
const EMPTY_ADVICE_COUNT: usize = 2;
const CLOSE_OPACITY: f32 = 0.7;
const NARROW_VIEWPORT: f32 = 300.0;
const CLOSE_RING_WIDTH: f32 = 2.0;
const OUTLINE_SHADOW_BLUR: f32 = 2.0;
const OUTLINE_SHADOW_ALPHA: u8 = 13;
const GLOBAL_INPUT_HEIGHT: f32 = 30.0;
const GLOBAL_LINE_HEIGHT: f32 = 20.0;
const LABEL_INPUT_GAP: f32 = 4.0;
const FIELD_INPUT_INSET: f32 = 9.0;
const BUTTON_RING_WIDTH: f32 = 3.0;
const BUTTON_RING_OPACITY: f32 = 0.5;
const DESTRUCTIVE_RING_OPACITY: f32 = 0.2;
const BUTTON_HOVER_OPACITY: f32 = 0.9;
const BUTTON_MOTION_HALF: f64 = 0.075;
const BUTTON_MOTION_HALF_EASED: f32 = 0.77556133;
const DISABLED_OPACITY: f32 = 0.5;
const BUTTON_BORDER_WIDTH: f32 = 1.0;
const TRANSLUCENT_BORDER: &str = "#31324480";
const SEMIBOLD_TEST_FAMILY: &str = "taide-ui-semibold";
const DIALOG_TITLE_SIZE: f32 = 18.0;
const ALERT_TITLE_HEIGHT: f32 = 28.0;

struct Scene {
    context: egui::Context,
    views: Views,
    owner: Owner,
    appearance: Appearance,
    locale: ResolvedLocale,
    settings: Settings,
    time: f64,
    screen: [f32; 2],
    theme: ResolvedTheme,
    window_focused: bool,
    frame_step: f64,
}

impl Scene {
    fn new() -> Self {
        let state = AppState::new(AppPaths::new(
            std::env::temp_dir().join(format!("taide-m8-snippet-ui-{}", ProjectId::new())),
        ));
        let theme = theme_actions::theme_get(&state, "taide-dark".into()).unwrap();
        let context = egui::Context::default();
        context.enable_accesskit();
        context.set_os(egui::os::OperatingSystem::Mac);
        Self {
            context,
            views: Views::default(),
            owner: Owner {
                project: ProjectId::new(),
                pane: PaneId::new(),
                tab: TabId::new(),
            },
            appearance: Appearance::new(&theme).unwrap(),
            locale: locale_actions::locale_get(&state, "en".into()).unwrap(),
            settings: Settings::default(),
            time: 0.0,
            screen: SCREEN,
            theme,
            window_focused: true,
            frame_step: FRAME_STEP,
        }
    }

    fn frame(&mut self, events: Vec<Event>, enabled: bool) -> (Output, egui::FullOutput) {
        self.time += self.frame_step;
        for event in &events {
            if let Event::WindowFocused(focused) = event {
                self.window_focused = *focused;
            }
        }
        let mut output = Output::default();
        self.views.begin_frame();
        let mut drawing = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(self.screen[0], self.screen[1]),
                )),
                time: Some(self.time),
                focused: self.window_focused,
                events,
                ..Default::default()
            },
            |ui| {
                if !enabled {
                    ui.disable();
                }
                let mut pass = self.views.show(
                    ui,
                    self.owner.clone(),
                    &self.settings,
                    &self.locale,
                    &self.appearance,
                );
                output.snippets.append(&mut pass.snippets);
                output.snippet_notices.append(&mut pass.snippet_notices);
                output.folders.append(&mut pass.folders);
                output.traces = pass.traces;
                output.editor_traces = pass.editor_traces;
                output.snippet_interactions = pass.snippet_interactions;
                output.scroll = pass.scroll;
            },
        );
        self.views.finish_frame();
        drawing.textures_delta.clear();
        (output, drawing)
    }

    fn point(&self, output: &Output, field: &str, index: usize) -> Pos2 {
        let (_, id, rect) = output
            .editor_traces
            .iter()
            .filter(|(name, _, _)| name == field)
            .nth(index)
            .unwrap_or_else(|| panic!("missing {field}:{index}"));
        let point = rect.center();
        assert!(
            output.snippet_interactions.get(id).unwrap().contains(point),
            "clipped {field}"
        );
        point
    }

    fn enter(&mut self) -> taide_native_ui::snippet_edit::Request {
        assert_eq!(Section::BASIC[5], Section::Snippets);
        let (output, _) = self.frame(Vec::new(), true);
        let toc = output
            .traces
            .iter()
            .find(|trace| trace.field == Section::Snippets.title())
            .unwrap()
            .rect
            .center();
        self.frame(click(toc), true);
        assert_eq!(
            self.views.inspection()[&self.owner].active,
            Section::Snippets
        );
        self.time += f64::from(self.context.global_style().scroll_animation.duration.max);
        self.frame(Vec::new(), true);
        let (output, _) = self.frame(Vec::new(), true);
        let manage = output
            .traces
            .iter()
            .find(|trace| trace.field == "settings.snippetsManage")
            .unwrap();
        assert_eq!(manage.rect.height(), 32.0);
        let folder = output
            .traces
            .iter()
            .find(|trace| trace.field == "settings.snippetsOpenFolder")
            .unwrap();
        assert_eq!(folder.rect.height(), 24.0);
        assert!(
            manage.interact_rect.contains(manage.rect.center()),
            "manage {:?}, response {:?}, scroll {:?}",
            manage.rect,
            manage.interact_rect,
            output.scroll
        );
        self.frame(click(manage.rect.center()), true);
        assert!(self.views.inspection()[&self.owner].snippets.is_some());
        let (mut output, _) = self.frame(Vec::new(), true);
        assert_eq!(output.snippets.len(), 1);
        output.snippets.remove(0)
    }
}

fn click(position: Pos2) -> Vec<Event> {
    vec![
        Event::PointerMoved(position),
        Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]
}

fn key(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn file(name: &str, content: &str) -> SnippetFile {
    SnippetFile {
        file_name: name.into(),
        snippets: serde_json::from_str(content).unwrap(),
    }
}

#[test]
fn snippet_ghost_back은_hover_이탈_후에도_배경을_끝까지_페이드한다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(Vec::new()))
    }));
    let (output, _) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    let target = scene.point(&output, "back", 0);
    let rect = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "back")
        .unwrap()
        .2;
    let fill = |drawing: &egui::FullOutput| {
        drawing
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape) if shape.rect == rect && shape.blur_width == 0.0 => {
                    Some(shape.fill)
                }
                _ => None,
            })
            .unwrap_or(egui::Color32::TRANSPARENT)
    };
    let hover = taide_native_ui::presentation::color(&scene.theme, "list.hoverBackground").unwrap();
    let (_, drawing) = scene.frame(vec![Event::PointerMoved(target)], true);
    assert_eq!(fill(&drawing), egui::Color32::TRANSPARENT);
    scene.frame(Vec::new(), true);
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(fill(&drawing), hover);
    let (_, drawing) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    assert_eq!(fill(&drawing), hover);
    let (_, drawing) = scene.frame(Vec::new(), true);
    let middle = fill(&drawing);
    assert!(middle.a() > 0 && middle.a() < hover.a());
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(fill(&drawing), egui::Color32::TRANSPARENT);
}

#[test]
fn snippet_button_hover는_실제_도형에서_즉시_점프하지_않고_150ms에_완료된다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file("rust.json", "{}")]))
    }));
    let (output, _) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    let target = scene.point(&output, "new-file", 0);
    let rect = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "new-file")
        .unwrap()
        .2;
    let fill = |drawing: &egui::FullOutput, rect: Rect| {
        drawing
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.rect == rect
                        && shape.blur_width == 0.0
                        && shape.fill != egui::Color32::TRANSPARENT =>
                {
                    Some(shape.fill)
                }
                _ => None,
            })
            .unwrap()
    };
    let background = taide_native_ui::presentation::color(&scene.theme, "app.background").unwrap();
    let hover = taide_native_ui::presentation::color(&scene.theme, "list.hoverBackground").unwrap();
    assert_ne!(background, hover);
    let (_, drawing) = scene.frame(vec![Event::PointerMoved(target)], true);
    assert_eq!(fill(&drawing, rect), background);
    let (_, drawing) = scene.frame(Vec::new(), true);
    let middle = fill(&drawing, rect);
    assert_ne!(middle, background);
    assert_ne!(middle, hover);
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(fill(&drawing, rect), hover);
    let (_, drawing) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    assert_eq!(fill(&drawing, rect), hover);
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_ne!(fill(&drawing, rect), background);
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(fill(&drawing, rect), background);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "delete-file", 0)), true);
    scene.time += SLOW_FRAME_STEP;
    let (output, _) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    let target = scene.point(&output, "dialog-confirm", 0);
    let rect = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "dialog-confirm")
        .unwrap()
        .2;
    let error =
        taide_native_ui::presentation::color(&scene.theme, "statusIndicator.error").unwrap();
    let (_, drawing) = scene.frame(vec![Event::PointerMoved(target)], true);
    assert_eq!(fill(&drawing, rect), error);
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_ne!(fill(&drawing, rect), error);
    scene.frame(Vec::new(), true);
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(
        fill(&drawing, rect),
        error.gamma_multiply(BUTTON_HOVER_OPACITY)
    );
}

#[test]
fn snippet_button은_포커스_링과_border를_150ms에_전환하고_입력_modality를_유지한다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file("rust.json", "{}")]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "delete-file", 0)), true);
    scene.time += SLOW_FRAME_STEP;
    let (output, drawing) = scene.frame(Vec::new(), true);
    let rect = |output: &Output, name: &str| {
        output
            .editor_traces
            .iter()
            .find(|(field, _, _)| field == name)
            .unwrap()
            .2
    };
    let ring = |drawing: &egui::FullOutput, rect: Rect, color: egui::Color32| {
        drawing.shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::Shape::Rect(shape) if shape.rect == rect && shape.stroke.width == BUTTON_RING_WIDTH && shape.stroke.color == color && shape.stroke_kind == egui::StrokeKind::Outside)
    })
    };
    let normal = taide_native_ui::presentation::color(&scene.theme, "app.focusBorder")
        .unwrap()
        .gamma_multiply(BUTTON_RING_OPACITY);
    let destructive = taide_native_ui::presentation::color(&scene.theme, "statusIndicator.error")
        .unwrap()
        .gamma_multiply(DESTRUCTIVE_RING_OPACITY);
    let cancel = rect(&output, "dialog-cancel");
    assert!(!ring(&drawing, cancel, normal));
    scene.frame(vec![key(Key::Tab, Modifiers::NONE)], true);
    let (output, drawing) = scene.frame(Vec::new(), true);
    let confirm = rect(&output, "dialog-confirm");
    assert!(!ring(&drawing, confirm, destructive));
    scene.frame_step = BUTTON_MOTION_HALF;
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert!(drawing.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Rect(shape) if shape.rect == confirm && shape.stroke_kind == egui::StrokeKind::Outside && (shape.stroke.width - BUTTON_RING_WIDTH * BUTTON_MOTION_HALF_EASED).abs() < GEOMETRY_TOLERANCE && shape.stroke.color == destructive.gamma_multiply(BUTTON_MOTION_HALF_EASED))));
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert!(
        ring(&drawing, confirm, destructive),
        "missing destructive keyboard ring"
    );
    scene.frame(vec![key(Key::Tab, Modifiers::SHIFT)], true);
    let (output, _) = scene.frame(Vec::new(), true);
    let cancel = rect(&output, "dialog-cancel");
    let (_, drawing) = scene.frame(Vec::new(), true);
    let border = taide_native_ui::presentation::color(&scene.theme, "app.border").unwrap();
    let focus = taide_native_ui::presentation::color(&scene.theme, "app.focusBorder").unwrap();
    let channels = std::array::from_fn::<_, 4, _>(|index| {
        (f32::from(border.to_array()[index]) * (1.0 - BUTTON_MOTION_HALF_EASED)
            + f32::from(focus.to_array()[index]) * BUTTON_MOTION_HALF_EASED)
            .round() as u8
    });
    let [red, green, blue, alpha] = channels;
    let middle_border = egui::Color32::from_rgba_premultiplied(red, green, blue, alpha);
    assert!(drawing.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Rect(shape) if shape.rect == cancel && shape.stroke_kind == egui::StrokeKind::Inside && shape.stroke.width == 1.0 && shape.stroke.color == middle_border)));
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert!(ring(&drawing, cancel, normal));
    let (_, drawing) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    assert!(ring(&drawing, cancel, normal));
    let (_, drawing) = scene.frame(vec![Event::WindowFocused(false)], true);
    assert!(ring(&drawing, cancel, normal));
    scene.time += SLOW_FRAME_STEP;
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert!(!ring(&drawing, cancel, normal));
    scene.frame(vec![Event::WindowFocused(true)], true);
    scene.time += SLOW_FRAME_STEP;
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert!(ring(&drawing, cancel, normal));
    scene.frame(Vec::new(), false);
    scene.time += SLOW_FRAME_STEP;
    let (_, drawing) = scene.frame(Vec::new(), false);
    assert!(!ring(&drawing, cancel, normal));
}

#[test]
fn snippet_outline은_반투명_border를_한_번만_그리고_modal중에도_삭제_opacity를_전환한다() {
    let mut scene = Scene::new();
    scene
        .theme
        .colors
        .insert("app.border".into(), TRANSLUCENT_BORDER.into());
    scene.appearance = Appearance::new(&scene.theme).unwrap();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file("rust.json", "{}")]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, drawing) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    let rect = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "delete-file")
        .unwrap()
        .2;
    let borders = |drawing: &egui::FullOutput| {
        drawing
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.rect == rect
                        && shape.stroke.width == BUTTON_BORDER_WIDTH
                        && shape.stroke.color.a() > 0 =>
                {
                    Some(shape.stroke.color)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let shadow = |drawing: &egui::FullOutput| {
        drawing
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.blur_width == OUTLINE_SHADOW_BLUR
                        && shape.rect.min.distance(rect.min + egui::vec2(0.0, 1.0))
                            < GEOMETRY_TOLERANCE =>
                {
                    Some(shape.fill)
                }
                _ => None,
            })
            .unwrap()
    };
    let border = taide_native_ui::presentation::color(&scene.theme, "app.border").unwrap();
    assert_eq!(borders(&drawing), vec![border]);
    scene.frame(click(scene.point(&output, "delete-file", 0)), true);
    scene.time += SLOW_FRAME_STEP;
    let (output, _) = scene.frame(Vec::new(), true);
    let (mut output, _) = scene.frame(click(scene.point(&output, "dialog-confirm", 0)), true);
    assert_eq!(output.snippets.len(), 1);
    assert!(matches!(
        output.snippets.remove(0).kind(),
        Kind::Delete { .. }
    ));
    let (output, drawing) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    assert!(
        output
            .editor_traces
            .iter()
            .any(|(name, _, _)| name == "dialog-content")
    );
    assert_eq!(borders(&drawing), vec![border]);
    scene.frame_step = BUTTON_MOTION_HALF;
    let (output, drawing) = scene.frame(Vec::new(), true);
    assert!(
        output
            .editor_traces
            .iter()
            .any(|(name, _, _)| name == "dialog-content")
    );
    let opacity = 1.0 - (1.0 - DISABLED_OPACITY) * BUTTON_MOTION_HALF_EASED;
    assert_eq!(borders(&drawing), vec![border.gamma_multiply(opacity)]);
    assert_eq!(
        shadow(&drawing),
        egui::Color32::from_black_alpha(OUTLINE_SHADOW_ALPHA).gamma_multiply(opacity)
    );
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(
        borders(&drawing),
        vec![border.gamma_multiply(DISABLED_OPACITY)]
    );
    assert_eq!(
        shadow(&drawing),
        egui::Color32::from_black_alpha(OUTLINE_SHADOW_ALPHA).gamma_multiply(DISABLED_OPACITY)
    );
}

#[test]
fn snippet_save는_즉시_입력을_차단하며_버튼과_문자_투명도는_150ms로_복구된다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file("rust.json", "{}")]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, _) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    let (_, save_id, rect) = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "save")
        .unwrap()
        .clone();
    scene
        .context
        .memory_mut(|memory| memory.request_focus(save_id));
    let (mut output, _) = scene.frame(vec![key(Key::Enter, Modifiers::NONE)], true);
    assert_eq!(output.snippets.len(), 1);
    let request = output.snippets.remove(0);
    assert!(matches!(request.kind(), Kind::Save { create: false, .. }));
    let fill = |drawing: &egui::FullOutput| {
        drawing
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if shape.rect == rect && shape.fill != egui::Color32::TRANSPARENT =>
                {
                    Some(shape.fill)
                }
                _ => None,
            })
            .unwrap()
    };
    let text_color = |drawing: &egui::FullOutput| {
        drawing
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(shape)
                    if rect.contains(shape.pos) && shape.galley.text() == "Save" =>
                {
                    Some(shape.galley.rows[0].visuals.mesh.vertices[0].color)
                }
                _ => None,
            })
            .unwrap()
    };
    let background =
        taide_native_ui::presentation::color(&scene.theme, "button.primaryBackground").unwrap();
    let foreground =
        taide_native_ui::presentation::color(&scene.theme, "button.primaryForeground").unwrap();
    let (output, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(
        output
            .editor_traces
            .iter()
            .find(|(name, _, _)| name == "save")
            .unwrap()
            .1,
        save_id
    );
    assert!(!output.snippet_interactions.contains_key(&save_id));
    assert_eq!(fill(&drawing), background);
    scene.frame_step = BUTTON_MOTION_HALF;
    let (_, drawing) = scene.frame(Vec::new(), true);
    let opacity = 1.0 - (1.0 - DISABLED_OPACITY) * BUTTON_MOTION_HALF_EASED;
    assert_eq!(fill(&drawing), background.gamma_multiply(opacity));
    assert_eq!(text_color(&drawing), foreground.gamma_multiply(opacity));
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(fill(&drawing), background.gamma_multiply(DISABLED_OPACITY));
    assert_eq!(
        text_color(&drawing),
        foreground.gamma_multiply(DISABLED_OPACITY)
    );
    let (output, _) = scene.frame(click(rect.center()), true);
    assert!(output.snippets.is_empty());
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Saved(file("rust.json", "{}")))
    }));
    let (output, drawing) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    assert!(output.snippet_interactions.contains_key(&save_id));
    assert_eq!(fill(&drawing), background.gamma_multiply(DISABLED_OPACITY));
    scene.frame(Vec::new(), true);
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(fill(&drawing), background);
    assert_eq!(text_color(&drawing), foreground);
}

#[test]
fn snippet_dialog_제목은_등록된_semibold_family와_미등록_fallback을_선택한다() {
    for registered in [true, false] {
        let mut scene = Scene::new();
        let family = egui::FontFamily::Name(SEMIBOLD_TEST_FAMILY.into());
        let expected = if registered {
            let mut fonts = egui::FontDefinitions::default();
            fonts.families.insert(
                family.clone(),
                fonts.families[&egui::FontFamily::Proportional].clone(),
            );
            scene.context.set_fonts(fonts);
            family
        } else {
            egui::FontFamily::Proportional
        };
        let title = |scene: &Scene, drawing: &egui::FullOutput, key: &str, line_height: f32| {
            let text = &scene.locale.messages[key];
            let galley = drawing
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(shape) if shape.galley.job.text == *text => {
                        Some(&shape.galley)
                    }
                    _ => None,
                })
                .unwrap();
            let format = &galley.job.sections[0].format;
            assert_eq!(
                format.font_id.family, expected,
                "{key}, registered={registered}"
            );
            assert_eq!(format.font_id.size, DIALOG_TITLE_SIZE);
            assert_eq!(format.line_height, Some(line_height));
            assert_eq!(
                format.color,
                taide_native_ui::presentation::color(&scene.theme, "app.foreground").unwrap()
            );
        };
        let request = scene.enter();
        assert!(scene.views.accept_snippet(Reply {
            request,
            result: Ok(Outcome::Listed(vec![file(
                "rust.json",
                r#"{"Entry":{"prefix":"entry","body":"line"}}"#
            )]))
        }));
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "new-file", 0)), true);
        scene.time += SLOW_FRAME_STEP;
        let (_, drawing) = scene.frame(Vec::new(), true);
        title(
            &scene,
            &drawing,
            "snippetEditor.newFileDialogTitle",
            DIALOG_TITLE_SIZE,
        );
        scene.frame(vec![key(Key::Escape, Modifiers::NONE)], true);
        scene.time += SLOW_FRAME_STEP;
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "delete-file", 0)), true);
        scene.time += SLOW_FRAME_STEP;
        let (output, drawing) = scene.frame(Vec::new(), true);
        title(
            &scene,
            &drawing,
            "snippetEditor.deleteFileConfirmTitle",
            ALERT_TITLE_HEIGHT,
        );
        scene.frame(click(scene.point(&output, "dialog-cancel", 0)), true);
        scene.time += SLOW_FRAME_STEP;
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "delete-entry", 0)), true);
        scene.time += SLOW_FRAME_STEP;
        let (output, drawing) = scene.frame(Vec::new(), true);
        title(
            &scene,
            &drawing,
            "snippetEditor.deleteConfirmTitle",
            ALERT_TITLE_HEIGHT,
        );
        scene.frame(click(scene.point(&output, "dialog-cancel", 0)), true);
        scene.time += SLOW_FRAME_STEP;
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "add-entry", 0)), true);
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "back", 0)), true);
        scene.time += SLOW_FRAME_STEP;
        let (_, drawing) = scene.frame(Vec::new(), true);
        title(
            &scene,
            &drawing,
            "common.unsavedChangesTitle",
            ALERT_TITLE_HEIGHT,
        );
    }
}

#[test]
fn snippet_dialog_제목은_sdk_강조색_대신_실제_theme_전경색을_사용한다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(Vec::new()))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "new-file", 0)), true);
    scene.time += SLOW_FRAME_STEP;
    let (_, drawing) = scene.frame(Vec::new(), true);
    let title = scene
        .locale
        .messages
        .get("snippetEditor.newFileDialogTitle")
        .unwrap();
    let color = drawing
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == *title => {
                Some(shape.galley.job.sections[0].format.color)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(
        color,
        taide_native_ui::presentation::color(&scene.theme, "app.foreground").unwrap()
    );
}

#[test]
fn snippet_이름_label과_input은_원본_세로_배치와_좌측_inset을_사용한다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file(
            "rust.json",
            r#"{"Entry":{"prefix":"entry","body":"line"}}"#
        )]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, drawing) = scene.frame(Vec::new(), true);
    let text = scene
        .locale
        .messages
        .get("snippetEditor.nameLabel")
        .unwrap();
    let label = drawing
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(shape) if shape.galley.job.text == *text => {
                Some(Rect::from_min_size(shape.pos, shape.galley.size()))
            }
            _ => None,
        })
        .unwrap();
    let input = output
        .editor_traces
        .iter()
        .find(|(field, _, _)| field == "name")
        .unwrap()
        .2;
    assert!(
        input.top() >= label.bottom() + LABEL_INPUT_GAP,
        "label {label:?}, input {input:?}"
    );
    assert!(
        (input.left() - label.left() - FIELD_INPUT_INSET).abs() < GEOMETRY_TOLERANCE,
        "label {label:?}, input {input:?}"
    );
}

#[test]
fn snippet_응답_알림은_편집기_렌더_없이도_한_번만_배출된다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file("rust.json", "{}")]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    let (mut output, _) = scene.frame(click(scene.point(&output, "save", 0)), true);
    let request = output.snippets.remove(0);
    assert!(
        scene
            .views
            .accept_snippet(request.failed(AppError::InvalidArgument("synthetic refusal".into())))
    );
    let notices = scene.views.take_snippet_notices();
    assert!(
        matches!(&notices[..], [Notice::SaveFailed { create: false, error: AppError::InvalidArgument(message) }] if message == "synthetic refusal")
    );
    assert!(scene.views.take_snippet_notices().is_empty());
    let (output, _) = scene.frame(Vec::new(), false);
    assert!(output.snippet_notices.is_empty());
}

#[test]
fn snippet_dialog의_원본_640px_분기와_작은_폐기_grid가_실제_렌더에_적용된다() {
    for (viewport, expected_width) in DIALOG_VIEWPORTS {
        let mut scene = Scene::new();
        let request = scene.enter();
        assert!(scene.views.accept_snippet(Reply {
            request,
            result: Ok(Outcome::Listed(vec![file("rust.json", "{}")]))
        }));
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "new-file", 0)), true);
        scene.screen[0] = viewport;
        scene.time += SLOW_FRAME_STEP;
        scene.frame(Vec::new(), true);
        let (output, _) = scene.frame(Vec::new(), true);
        let rect = |field: &str| {
            output
                .editor_traces
                .iter()
                .find(|(name, _, _)| name == field)
                .unwrap()
                .2
        };
        let content = rect("dialog-content");
        assert!(
            (content.width() - expected_width).abs() < GEOMETRY_TOLERANCE,
            "viewport {viewport}: {content:?}"
        );
        let confirm = rect("dialog-confirm");
        let cancel = rect("dialog-cancel");
        assert!((confirm.height() - FOOTER_HEIGHT).abs() < GEOMETRY_TOLERANCE);
        assert!((cancel.height() - FOOTER_HEIGHT).abs() < GEOMETRY_TOLERANCE);
        if viewport < RESPONSIVE_BREAKPOINT {
            assert!(confirm.bottom() < cancel.top());
            assert!((confirm.width() - expected_width + DIALOG_INSETS).abs() < GEOMETRY_TOLERANCE);
            assert!((cancel.width() - confirm.width()).abs() < GEOMETRY_TOLERANCE);
        } else {
            assert!((cancel.center().y - confirm.center().y).abs() < GEOMETRY_TOLERANCE);
            assert!(cancel.right() < confirm.left());
        }
        scene.frame(vec![key(Key::Escape, Modifiers::NONE)], true);
        scene.time += SLOW_FRAME_STEP;
        scene.frame(Vec::new(), true);
        scene.screen = SCREEN;
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "add-entry", 0)), true);
        let (output, _) = scene.frame(Vec::new(), true);
        scene.frame(click(scene.point(&output, "back", 0)), true);
        scene.screen[0] = viewport;
        scene.time += SLOW_FRAME_STEP;
        scene.frame(Vec::new(), true);
        let (output, _) = scene.frame(Vec::new(), true);
        let rect = |field: &str| {
            output
                .editor_traces
                .iter()
                .find(|(name, _, _)| name == field)
                .unwrap()
                .2
        };
        let content = rect("dialog-content");
        let cancel = rect("dialog-cancel");
        let confirm = rect("dialog-confirm");
        assert!((content.width() - SMALL_DIALOG_WIDTH).abs() < GEOMETRY_TOLERANCE);
        assert!((cancel.width() - confirm.width()).abs() < GEOMETRY_TOLERANCE);
        assert!(cancel.right() < confirm.left());
        assert!((cancel.center().y - confirm.center().y).abs() < GEOMETRY_TOLERANCE);
    }
}

#[test]
fn snippet_전역_이름은_원본_줄높이를_쓰고_alert_설명은_ax로_연결된다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file("rust.json", "{}")]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "new-file", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(
        click(scene.point(&output, "snippetEditor.newFileLanguagePlaceholder", 0)),
        true,
    );
    scene.frame(vec![key(Key::Home, Modifiers::NONE)], true);
    scene.frame(vec![key(Key::Enter, Modifiers::NONE)], true);
    scene.time += SLOW_FRAME_STEP;
    let (output, _) = scene.frame(Vec::new(), true);
    let (_, input_id, input) = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "new-global-name")
        .unwrap();
    assert_eq!(
        scene.context.memory(|memory| memory.focused()),
        Some(*input_id)
    );
    assert!(
        (input.height() - GLOBAL_INPUT_HEIGHT).abs() < GEOMETRY_TOLERANCE,
        "{input:?}"
    );
    scene.frame(vec![Event::Text("global".into())], true);
    assert_eq!(
        scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .new_file
            .global_name,
        "global"
    );
    let (_, drawing) = scene.frame(Vec::new(), true);
    let text = drawing
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text == "global" => Some(text),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        text.galley.job.sections[0].format.line_height,
        Some(GLOBAL_LINE_HEIGHT)
    );
    scene.frame(vec![key(Key::Escape, Modifiers::NONE)], true);
    scene.time += SLOW_FRAME_STEP;
    scene.frame(Vec::new(), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "delete-file", 0)), true);
    scene.time += SLOW_FRAME_STEP;
    let (output, drawing) = scene.frame(Vec::new(), true);
    let tree = drawing.platform_output.accesskit_update.as_ref().unwrap();
    let (_, dialog) = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == egui::accesskit::Role::AlertDialog)
        .unwrap();
    let (_, description_id, _) = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "dialog-description")
        .unwrap();
    assert_eq!(dialog.described_by(), &[description_id.accesskit_id()]);
    assert!(
        tree.nodes
            .iter()
            .any(|(id, _)| *id == description_id.accesskit_id())
    );
    assert!(!dialog.labelled_by().is_empty());
}

#[test]
fn snippet_close의_focus링_outline그림자와_좁은_폐기_dialog가_원본_치수로_렌더된다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file("rust.json", "{}")]))
    }));
    let (output, drawing) = scene.frame(Vec::new(), true);
    let rect = |output: &Output, field: &str| {
        output
            .editor_traces
            .iter()
            .find(|(name, _, _)| name == field)
            .unwrap()
            .2
    };
    let new = rect(&output, "new-file");
    assert!(drawing.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Rect(shape) =>
            shape.blur_width == OUTLINE_SHADOW_BLUR
                && shape.fill == egui::Color32::from_black_alpha(OUTLINE_SHADOW_ALPHA)
                && shape.rect.min.distance(new.min + egui::vec2(0.0, 1.0)) < GEOMETRY_TOLERANCE,
        _ => false,
    }));
    scene.frame(click(scene.point(&output, "new-file", 0)), true);
    scene.time += SLOW_FRAME_STEP;
    scene.frame(Vec::new(), true);
    scene.frame(vec![key(Key::Tab, Modifiers::SHIFT)], true);
    let (output, drawing) = scene.frame(Vec::new(), true);
    let close = rect(&output, "dialog-close");
    let focus = taide_native_ui::presentation::color(&scene.theme, "app.focusBorder").unwrap();
    assert!(drawing.shapes.iter().any(|clipped| match &clipped.shape {
        egui::Shape::Rect(shape) =>
            shape.stroke == egui::Stroke::new(CLOSE_RING_WIDTH, focus.gamma_multiply(CLOSE_OPACITY))
                && shape.stroke_kind == egui::StrokeKind::Outside
                && shape.rect.min.distance(close.expand(CLOSE_RING_WIDTH).min) < GEOMETRY_TOLERANCE,
        _ => false,
    }));
    scene.frame(vec![key(Key::Escape, Modifiers::NONE)], true);
    scene.time += SLOW_FRAME_STEP;
    scene.frame(Vec::new(), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "add-entry", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "back", 0)), true);
    scene.screen[0] = NARROW_VIEWPORT;
    scene.time += SLOW_FRAME_STEP;
    scene.frame(Vec::new(), true);
    let (output, _) = scene.frame(Vec::new(), true);
    let content = rect(&output, "dialog-content");
    assert!(
        (content.width() - NARROW_VIEWPORT).abs() < GEOMETRY_TOLERANCE,
        "{content:?}"
    );
}

#[test]
fn snippet_close의_150ms_투명도는_링까지_적용되고_trash_hover는_즉시_적용된다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file(
            "rust.json",
            r#"{"Entry":{"prefix":"entry","body":"line"}}"#
        )]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "new-file", 0)), true);
    scene.time += SLOW_FRAME_STEP;
    scene.frame(Vec::new(), true);
    let (output, drawing) = scene.frame(Vec::new(), true);
    let (_, _, close) = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "dialog-close")
        .unwrap();
    let close = *close;
    let tint = |drawing: &egui::FullOutput, rect: Rect| {
        drawing
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh)
                    if rect
                        .expand(GEOMETRY_TOLERANCE)
                        .contains_rect(mesh.calc_bounds())
                        && !mesh.vertices.is_empty() =>
                {
                    Some(mesh.vertices[0].color)
                }
                egui::Shape::Rect(shape)
                    if shape.brush.is_some()
                        && rect.expand(GEOMETRY_TOLERANCE).contains_rect(shape.rect) =>
                {
                    Some(shape.fill)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing icon {rect:?}"))
    };
    let foreground = taide_native_ui::presentation::color(&scene.theme, "app.foreground").unwrap();
    assert_eq!(
        tint(&drawing, close),
        foreground.gamma_multiply(CLOSE_OPACITY)
    );
    scene.frame(vec![key(Key::Tab, Modifiers::SHIFT)], true);
    let (_, drawing) = scene.frame(Vec::new(), true);
    let focus = taide_native_ui::presentation::color(&scene.theme, "app.focusBorder").unwrap();
    let offset = taide_native_ui::presentation::color(&scene.theme, "app.background").unwrap();
    let ring = |drawing: &egui::FullOutput, rect: Rect, color: egui::Color32| {
        drawing.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Rect(shape) if shape.rect == rect && shape.stroke == egui::Stroke::new(CLOSE_RING_WIDTH, color) && shape.stroke_kind == egui::StrokeKind::Outside))
    };
    assert!(ring(&drawing, close, offset.gamma_multiply(CLOSE_OPACITY)));
    assert!(ring(
        &drawing,
        close.expand(CLOSE_RING_WIDTH),
        focus.gamma_multiply(CLOSE_OPACITY)
    ));
    let (_, drawing) = scene.frame(vec![Event::PointerMoved(close.center())], true);
    assert_eq!(
        tint(&drawing, close),
        foreground.gamma_multiply(CLOSE_OPACITY)
    );
    scene.frame_step = BUTTON_MOTION_HALF;
    let (_, drawing) = scene.frame(Vec::new(), true);
    let middle = CLOSE_OPACITY + (1.0 - CLOSE_OPACITY) * BUTTON_MOTION_HALF_EASED;
    assert_eq!(tint(&drawing, close), foreground.gamma_multiply(middle));
    assert!(ring(
        &drawing,
        close.expand(CLOSE_RING_WIDTH),
        focus.gamma_multiply(middle)
    ));
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(tint(&drawing, close), foreground);
    let (_, drawing) = scene.frame(vec![Event::PointerMoved(Pos2::ZERO)], true);
    assert_eq!(tint(&drawing, close), foreground);
    let (_, drawing) = scene.frame(Vec::new(), true);
    let middle = 1.0 - (1.0 - CLOSE_OPACITY) * BUTTON_MOTION_HALF_EASED;
    assert_eq!(tint(&drawing, close), foreground.gamma_multiply(middle));
    assert!(ring(&drawing, close, offset.gamma_multiply(middle)));
    let (_, drawing) = scene.frame(Vec::new(), true);
    assert_eq!(
        tint(&drawing, close),
        foreground.gamma_multiply(CLOSE_OPACITY)
    );
    scene.frame(vec![key(Key::Escape, Modifiers::NONE)], true);
    scene.time += SLOW_FRAME_STEP;
    scene.frame(Vec::new(), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, drawing) = scene.frame(Vec::new(), true);
    let (_, _, trash) = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "delete-entry")
        .unwrap();
    let trash = *trash;
    assert_eq!(
        tint(&drawing, trash),
        taide_native_ui::presentation::color(&scene.theme, "appSidebar.iconDefault").unwrap()
    );
    let (_, drawing) = scene.frame(vec![Event::PointerMoved(trash.center())], true);
    assert_eq!(
        tint(&drawing, trash),
        taide_native_ui::presentation::color(&scene.theme, "statusIndicator.error").unwrap()
    );
}

#[test]
fn snippet_목록의_원본_제목_빈안내_간격_좌측정렬과_입력_반경이_렌더된다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(Vec::new()))
    }));
    scene.frame(Vec::new(), true);
    let (output, drawing) = scene.frame(Vec::new(), true);
    let no_files = scene.locale.messages.get("snippetEditor.noFiles").unwrap();
    assert_eq!(drawing.shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == *no_files)).count(), EMPTY_ADVICE_COUNT);
    scene.frame(click(scene.point(&output, "back", 0)), true);
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![
            file(
                "rust.json",
                r#"{"Entry":{"prefix":"entry","body":"line","description":"desc"}}"#
            ),
            file("other.code-snippets", "{}")
        ]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, drawing) = scene.frame(Vec::new(), true);
    let rect = |field: &str| {
        output
            .editor_traces
            .iter()
            .find(|(name, _, _)| name == field)
            .unwrap()
            .2
    };
    let first = rect("file:rust.json");
    let second = rect("file:other.code-snippets");
    assert!((second.top() - first.bottom() - FILE_GAP).abs() < GEOMETRY_TOLERANCE);
    assert!(
        (first.width() - FILE_CONTENT_WIDTH).abs() < GEOMETRY_TOLERANCE,
        "{first:?}"
    );
    let card = rect("entry-card");
    let container = rect("editor-container");
    assert!(
        (card.width() - container.width() + EDITOR_NON_CARD_WIDTH).abs() < GEOMETRY_TOLERANCE,
        "card {card:?}, container {container:?}"
    );
    let title = scene
        .locale
        .messages
        .get("snippetEditor.snippetListTitle")
        .unwrap();
    assert!(drawing.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == *title)
    ));
    let text = drawing
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text)
                if text.galley.job.text == "rust.json" && first.contains(text.pos) =>
            {
                Some(text)
            }
            _ => None,
        })
        .unwrap();
    assert!(
        (text.pos.x - first.left() - FILE_PADDING).abs() < GEOMETRY_TOLERANCE,
        "{text:?}"
    );
    let name = rect("name");
    assert!(drawing.shapes.iter().any(|shape| match &shape.shape {
        egui::Shape::Rect(shape) =>
            shape.rect.contains_rect(name)
                && shape.corner_radius == egui::CornerRadius::same(INPUT_RADIUS)
                && shape.stroke.width == 1.0,
        _ => false,
    }));
}

#[test]
fn snippet_dialog의_확대전이중_scrim_도형과_clip은_전체_viewport를_덮는다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(Vec::new()))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "new-file", 0)), true);
    let (_, drawing) = scene.frame(Vec::new(), true);
    let screen = Rect::from_min_size(Pos2::ZERO, egui::vec2(SCREEN[0], SCREEN[1]));
    assert!(drawing.shapes.iter().any(|clipped| {
        let egui::Shape::Rect(shape) = &clipped.shape else {
            return false;
        };
        shape.fill.a() > 0
            && shape.fill.a() < u8::MAX
            && shape.rect.min.distance(screen.min) < GEOMETRY_TOLERANCE
            && shape.rect.max.distance(screen.max) < GEOMETRY_TOLERANCE
            && clipped
                .clip_rect
                .expand(GEOMETRY_TOLERANCE)
                .contains_rect(screen)
    }));
}

#[test]
fn snippet_dialog은_200ms_presence_전체_scrim_tab순환_alert취소_포커스와_drop을_보존한다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(vec![file(
            "rust.json",
            r#"{"Synthetic":{"prefix":"s","body":"body"}}"#
        )]))
    }));
    let geometry = |output: &Output, field: &str| {
        output
            .editor_traces
            .iter()
            .find(|(name, _, _)| name == field)
            .map(|(_, id, rect)| (*id, *rect))
            .unwrap_or_else(|| panic!("missing {field}"))
    };
    let (output, _) = scene.frame(Vec::new(), true);
    let (output, _) = scene.frame(click(scene.point(&output, "new-file", 0)), true);
    let (modal_id, _) = geometry(&output, "dialog-content");
    let layer = egui::LayerId::new(egui::Order::Foreground, modal_id);
    let transform = scene.context.layer_transform_to_global(layer).unwrap();
    assert!((transform.scaling - 0.95).abs() < GEOMETRY_TOLERANCE);
    let (_, backdrop) = geometry(&output, "dialog-backdrop");
    assert!(backdrop.min.distance(Pos2::ZERO) < GEOMETRY_TOLERANCE);
    assert!(backdrop.max.distance(egui::pos2(SCREEN[0], SCREEN[1])) < GEOMETRY_TOLERANCE);
    let (output, _) = scene.frame(Vec::new(), true);
    let (picker, _) = geometry(&output, "snippetEditor.newFileLanguagePlaceholder");
    let (cancel, _) = geometry(&output, "dialog-cancel");
    let (confirm, _) = geometry(&output, "dialog-confirm");
    let (close, _) = geometry(&output, "dialog-close");
    assert!(!output.snippet_interactions.contains_key(&confirm));
    assert_eq!(
        scene.context.memory(|memory| memory.focused()),
        Some(picker)
    );
    scene.frame(vec![key(Key::Tab, Modifiers::NONE)], true);
    assert_eq!(
        scene.context.memory(|memory| memory.focused()),
        Some(cancel)
    );
    scene.frame(vec![key(Key::Tab, Modifiers::NONE)], true);
    assert_eq!(scene.context.memory(|memory| memory.focused()), Some(close));
    scene.frame(vec![key(Key::Tab, Modifiers::NONE)], true);
    assert_eq!(
        scene.context.memory(|memory| memory.focused()),
        Some(picker)
    );
    scene.frame(vec![key(Key::Tab, Modifiers::SHIFT)], true);
    assert_eq!(scene.context.memory(|memory| memory.focused()), Some(close));
    let (output, _) = scene.frame(vec![key(Key::Escape, Modifiers::NONE)], true);
    assert!(output.snippets.is_empty());
    assert!(
        !scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .new_file
            .open
    );
    let (output, _) = scene.frame(Vec::new(), true);
    assert!(scene.context.layer_transform_to_global(layer).is_some());
    assert!(
        output
            .editor_traces
            .iter()
            .any(|(name, _, _)| name == "dialog-content")
    );
    assert!(!output.snippet_interactions.contains_key(&cancel));
    assert!(
        !output
            .editor_traces
            .iter()
            .filter(|(name, _, _)| name == "new-file")
            .any(|(_, id, _)| output.snippet_interactions.contains_key(id))
    );
    scene.time += SLOW_FRAME_STEP;
    let (output, _) = scene.frame(Vec::new(), true);
    assert!(scene.context.layer_transform_to_global(layer).is_none());
    assert!(!scene.context.dismissal_layers().contains(&modal_id));
    assert!(
        !output
            .editor_traces
            .iter()
            .any(|(name, _, _)| name == "dialog-content")
    );
    scene.frame(Vec::new(), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "file:rust.json", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "delete-file", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    let (cancel, _) = geometry(&output, "dialog-cancel");
    assert_eq!(
        scene.context.memory(|memory| memory.focused()),
        Some(cancel)
    );
    let (modal_id, _) = geometry(&output, "dialog-content");
    let layer = egui::LayerId::new(egui::Order::Foreground, modal_id);
    let (output, _) = scene.frame(click(egui::pos2(1.0, 1.0)), true);
    assert!(output.snippets.is_empty());
    assert!(
        scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .delete_file_open
    );
    scene.views.clear();
    assert!(scene.context.layer_transform_to_global(layer).is_none());
    assert!(!scene.context.dismissal_layers().contains(&modal_id));
}

#[test]
fn 실제_settings_takeover의_입력_저장_거절_재시도_초안_삭제와_unmount가_연속으로_작동한다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert_eq!(request.kind(), &Kind::List);
    let original = file("rust.json", r#"{"A":{"prefix":"a","body":"one"}}"#);
    assert!(scene.views.accept_snippet(Reply {
        request: request.clone(),
        result: Ok(Outcome::Listed(vec![original.clone()]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    let position = scene.point(&output, "file:rust.json", 0);
    scene.frame(click(position), true);
    let (output, drawing) = scene.frame(Vec::new(), true);
    assert!(
        output
            .editor_traces
            .iter()
            .any(|(name, _, _)| name == "body")
    );
    assert!(
        !output
            .editor_traces
            .iter()
            .any(|(name, _, _)| name == "scope")
    );
    let tree = drawing.platform_output.accesskit_update.as_ref().unwrap();
    for field in ["name", "prefix", "body", "description"] {
        let (_, id, _) = output
            .editor_traces
            .iter()
            .find(|(name, _, _)| name == field)
            .unwrap();
        let (_, node) = tree
            .nodes
            .iter()
            .find(|(node_id, _)| *node_id == id.accesskit_id())
            .unwrap();
        assert!(!node.labelled_by().is_empty());
    }
    let body_id = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "body")
        .unwrap()
        .1;
    scene.frame(click(scene.point(&output, "body", 0)), true);
    scene.frame(
        vec![
            key(Key::A, Modifiers::MAC_CMD | Modifiers::COMMAND),
            Event::Text("typed\n$0".into()),
        ],
        true,
    );
    assert_eq!(
        scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .drafts()
            .unwrap()[0]
            .body,
        "typed\n$0"
    );
    let (output, _) = scene.frame(Vec::new(), true);
    assert_eq!(
        output
            .editor_traces
            .iter()
            .find(|(name, _, _)| name == "body")
            .unwrap()
            .1,
        body_id
    );
    let (mut output, _) = scene.frame(click(scene.point(&output, "save", 0)), true);
    let failed = output.snippets.remove(0);
    let Kind::Save {
        file_name,
        content,
        create,
    } = failed.kind()
    else {
        panic!("expected save");
    };
    assert_eq!(file_name, "rust.json");
    assert!(!create);
    let canonical = file(file_name, content);
    assert!(
        scene.views.accept_snippet(
            failed
                .clone()
                .failed(AppError::Internal("synthetic refusal".into()))
        )
    );
    assert!(
        !scene.views.accept_snippet(
            failed
                .clone()
                .failed(AppError::Internal("duplicate reply".into()))
        )
    );
    let (output, _) = scene.frame(Vec::new(), true);
    assert!(matches!(
        output.snippet_notices.as_slice(),
        [Notice::SaveFailed { create: false, .. }]
    ));
    assert!(
        scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .has_unsaved_changes()
    );
    let (mut output, _) = scene.frame(click(scene.point(&output, "save", 0)), true);
    let saved = output.snippets.remove(0);
    assert!(!saved.same_request(&failed));
    assert!(scene.views.accept_snippet(Reply {
        request: saved.clone(),
        result: Ok(Outcome::Saved(canonical.clone()))
    }));
    let (mut output, _) = scene.frame(Vec::new(), true);
    assert!(matches!(output.snippet_notices.as_slice(), [Notice::Saved]));
    let refreshed = output.snippets.remove(0);
    assert!(!scene.views.accept_snippet(Reply {
        request: request.clone(),
        result: Ok(Outcome::Listed(vec![original]))
    }));
    assert!(scene.views.accept_snippet(Reply {
        request: refreshed,
        result: Ok(Outcome::Listed(vec![canonical]))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    assert!(
        !scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .has_unsaved_changes()
    );
    let (output, _) = scene.frame(click(scene.point(&output, "add-entry", 0)), true);
    scene.frame(click(scene.point(&output, "name", 1)), true);
    scene.frame(vec![Event::Text("incomplete".into())], true);
    let (output, _) = scene.frame(Vec::new(), true);
    let (output, _) = scene.frame(click(scene.point(&output, "save", 0)), true);
    assert!(output.snippets.is_empty());
    assert!(matches!(
        output.snippet_notices.as_slice(),
        [Notice::Incomplete(1)]
    ));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "delete-entry", 1)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "dialog-confirm", 0)), true);
    assert_eq!(
        scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .drafts()
            .unwrap()
            .len(),
        1
    );
    scene.time += SLOW_FRAME_STEP;
    scene.frame(Vec::new(), true);
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "back", 0)), true);
    assert!(scene.views.inspection()[&scene.owner].snippets.is_none());
    assert!(!request.is_active());
    assert!(!saved.is_active());
    assert!(
        !scene
            .views
            .accept_snippet(saved.failed(AppError::Internal("late reply".into())))
    );
}

#[test]
fn 새_파일_dialog의_실제_picker_전역_입력_ime_escape와_disabled가_검증된다() {
    let mut scene = Scene::new();
    let request = scene.enter();
    assert!(scene.views.accept_snippet(Reply {
        request,
        result: Ok(Outcome::Listed(Vec::new()))
    }));
    let (output, _) = scene.frame(Vec::new(), true);
    scene.frame(click(scene.point(&output, "new-file", 0)), true);
    let (output, _) = scene.frame(Vec::new(), true);
    let language = scene.point(&output, "snippetEditor.newFileLanguagePlaceholder", 0);
    scene.frame(click(language), true);
    scene.frame(vec![key(Key::Home, Modifiers::NONE)], true);
    scene.frame(vec![key(Key::Enter, Modifiers::NONE)], true);
    scene.time += SLOW_FRAME_STEP;
    let (output, _) = scene.frame(Vec::new(), true);
    assert!(
        scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .new_file
            .is_global()
    );
    assert!(
        output
            .editor_traces
            .iter()
            .any(|(name, _, _)| name == "new-global-name")
    );
    scene.frame(vec![Event::Text("global".into())], true);
    assert_eq!(
        scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .new_file
            .file_name(),
        "global.code-snippets"
    );
    scene.frame(
        vec![
            Event::Ime(egui::ImeEvent::Preedit {
                text: "한".into(),
                active_range_chars: None,
            }),
            key(Key::Escape, Modifiers::NONE),
        ],
        true,
    );
    assert!(
        scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .new_file
            .open
    );
    scene.frame(vec![Event::Ime(egui::ImeEvent::Commit("한".into()))], true);
    let (output, _) = scene.frame(Vec::new(), false);
    let confirm = output
        .editor_traces
        .iter()
        .find(|(name, _, _)| name == "dialog-confirm")
        .unwrap()
        .2
        .center();
    let (output, _) = scene.frame(click(confirm), false);
    assert!(output.snippets.is_empty());
    assert!(
        scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .new_file
            .open
    );
    let (output, _) = scene.frame(Vec::new(), true);
    let (mut output, _) = scene.frame(click(scene.point(&output, "dialog-confirm", 0)), true);
    let create = output.snippets.remove(0);
    let Kind::Save {
        file_name,
        content,
        create: is_new,
    } = create.kind()
    else {
        panic!("expected create");
    };
    assert!(is_new);
    assert!(file_name.ends_with(".code-snippets"));
    assert_eq!(content, "{}");
    assert!(
        !scene.views.inspection()[&scene.owner]
            .snippets
            .as_ref()
            .unwrap()
            .state()
            .new_file
            .open
    );
}
