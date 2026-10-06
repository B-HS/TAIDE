use super::*;
use egui::{Pos2, RawInput, Rect};

const SCREEN: [f32; 2] = [800.0, 600.0];
const FRAME_SECONDS: f64 = 0.05;
const CONTROL_SIDE: f32 = 8.0;
const SETTLED_FRAME_COUNT: usize = 3;
const PADDING: i8 = 24;
const ENTER_SCALE: f32 = 0.95;
const SCRIM_SECONDS: f64 = 0.15;
const TOLERANCE: f32 = 0.001;
const GEOMETRY_TOLERANCE: f32 = 0.01;
const BACKGROUND: &str = "synthetic-background-control";
const DIALOG: &str = "synthetic-dialog";
const CHROME: Chrome = Chrome {
    background: Color32::from_rgb(10, 20, 30),
    border: Color32::from_rgb(40, 50, 60),
    shadow: Color32::from_rgb(64, 128, 192),
};

#[derive(Clone, Copy, PartialEq)]
enum Step {
    Closed,
    ClosedWithoutSettling,
    Open,
    Closing { should_return: bool },
    Exiting,
}

struct Scene {
    context: egui::Context,
    time: f64,
    focus: FocusReturn,
    controls: Vec<Id>,
    should_focus_background: bool,
}

impl Scene {
    fn new() -> Self {
        Self {
            context: egui::Context::default(),
            time: 0.0,
            focus: FocusReturn::default(),
            controls: ["first", "second", "third"]
                .into_iter()
                .map(|name| Id::new(("synthetic-dialog-control", name)))
                .collect(),
            should_focus_background: false,
        }
    }

    fn layer(transition: Option<Transition>, is_modal: bool) -> Layer {
        Layer {
            id: Id::new(DIALOG),
            chrome: CHROME,
            padding: PADDING,
            transition,
            is_modal,
        }
    }

    fn frame(
        &mut self,
        events: Vec<Event>,
        step: Step,
        transition: Option<Transition>,
    ) -> egui::FullOutput {
        self.time += FRAME_SECONDS;
        let controls = self.controls.clone();
        let focus = &mut self.focus;
        let should_focus_background = std::mem::take(&mut self.should_focus_background);
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(SCREEN[0], SCREEN[1]),
                )),
                time: Some(self.time),
                focused: true,
                events,
                ..Default::default()
            },
            |ui| {
                let background = ui.interact(
                    Rect::from_min_size(Pos2::ZERO, egui::Vec2::splat(CONTROL_SIDE)),
                    Id::new(BACKGROUND),
                    Sense::focusable_noninteractive(),
                );
                if should_focus_background {
                    background.request_focus();
                }
                if step == Step::ClosedWithoutSettling {
                    return;
                }
                if step == Step::Closed {
                    focus.settle(ui.ctx());
                    return;
                }
                Self::layer(transition, step != Step::Exiting).show(ui.ctx(), |ui| {
                    if step == Step::Exiting {
                        ui.disable();
                    }
                    for control in &controls {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::Vec2::splat(CONTROL_SIDE), Sense::hover());
                        ui.interact(rect, *control, Sense::focusable_noninteractive());
                    }
                });
                match step {
                    Step::Open => trap_focus(ui.ctx(), &controls),
                    Step::Closing { should_return } => focus.release(ui.ctx(), should_return),
                    Step::Exiting => focus.settle(ui.ctx()),
                    Step::Closed | Step::ClosedWithoutSettling => (),
                }
            },
        );
        output.textures_delta.clear();
        output
    }

    fn focused(&self) -> Option<Id> {
        self.context.memory(|memory| memory.focused())
    }

    fn open(&mut self) {
        let background = Id::new(BACKGROUND);
        self.context
            .memory_mut(|memory| memory.request_focus(background));
        self.frame(Vec::new(), Step::Closed, None);
        assert_eq!(self.focused(), Some(background));
        self.focus.capture(&self.context);
        self.frame(Vec::new(), Step::Open, None);
        let first = self.controls[0];
        self.context
            .memory_mut(|memory| memory.request_focus(first));
        self.frame(Vec::new(), Step::Open, None);
        assert_eq!(self.focused(), Some(first));
    }
}

fn tab(is_backward: bool) -> Event {
    Event::Key {
        key: Key::Tab,
        physical_key: Some(Key::Tab),
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            shift: is_backward,
            ..Default::default()
        },
    }
}

fn preedit(text: &str) -> Event {
    Event::Ime(egui::ImeEvent::Preedit {
        text: text.into(),
        active_range_chars: None,
    })
}

fn rects(output: &egui::FullOutput) -> Vec<&egui::epaint::RectShape> {
    let mut pending = output
        .shapes
        .iter()
        .map(|clipped| &clipped.shape)
        .collect::<Vec<_>>();
    let mut rects = Vec::new();
    while let Some(shape) = pending.pop() {
        match shape {
            egui::Shape::Rect(rect) => rects.push(rect),
            egui::Shape::Vec(children) => pending.extend(children),
            _ => (),
        }
    }
    rects
}

fn covers_screen_with(output: &egui::FullOutput, fill: Color32) -> bool {
    let screen = egui::vec2(SCREEN[0], SCREEN[1]);
    rects(output)
        .iter()
        .any(|rect| (rect.rect.size() - screen).length() < GEOMETRY_TOLERANCE && rect.fill == fill)
}

#[test]
fn 닫는_프레임의_포커스_요청만으로는_다음_프레임에_배경_위젯이_포커스를_내놓는다() {
    let mut scene = Scene::new();
    scene.open();
    scene.frame(
        Vec::new(),
        Step::Closing {
            should_return: true,
        },
        None,
    );
    assert_eq!(scene.focused(), Some(Id::new(BACKGROUND)));
    scene.frame(Vec::new(), Step::ClosedWithoutSettling, None);
    assert_eq!(
        scene.focused(),
        None,
        "직전 프레임의 modal 층이 아직 최상위라 그 아래 위젯은 포커스를 내놓는다"
    );
}

#[test]
fn 닫는_프레임에_되돌린_포커스는_배경_위젯이_다시_그려져도_유지된다() {
    let mut scene = Scene::new();
    scene.open();
    scene.frame(
        Vec::new(),
        Step::Closing {
            should_return: true,
        },
        None,
    );
    for _ in 0..SETTLED_FRAME_COUNT {
        assert_eq!(scene.focused(), Some(Id::new(BACKGROUND)));
        scene.frame(Vec::new(), Step::Closed, None);
    }
    assert_eq!(scene.focused(), Some(Id::new(BACKGROUND)));
}

#[test]
fn 동작으로_닫힌_레이어는_이전_포커스로_돌아가지_않는다() {
    let mut scene = Scene::new();
    scene.open();
    scene.frame(
        Vec::new(),
        Step::Closing {
            should_return: false,
        },
        None,
    );
    for _ in 0..SETTLED_FRAME_COUNT {
        scene.frame(Vec::new(), Step::Closed, None);
    }
    assert_eq!(scene.focused(), None);
}

#[test]
fn 닫힘_전환_동안_레이어가_남아_있어도_포커스_복귀는_끝까지_유지된다() {
    let mut scene = Scene::new();
    scene.open();
    scene.frame(
        Vec::new(),
        Step::Closing {
            should_return: true,
        },
        Some(Transition::SETTLED),
    );
    for _ in 0..SETTLED_FRAME_COUNT {
        scene.frame(Vec::new(), Step::Exiting, Some(Transition::SETTLED));
        assert_eq!(scene.focused(), Some(Id::new(BACKGROUND)));
    }
    for _ in 0..SETTLED_FRAME_COUNT {
        scene.frame(Vec::new(), Step::Closed, None);
        assert_eq!(scene.focused(), Some(Id::new(BACKGROUND)));
    }
}

#[test]
fn 닫힘_전환_중인_레이어는_modal로_등록되지_않아_뒤_화면_위젯이_요청한_포커스를_빼앗지_않는다() {
    for transition in [Some(Transition::SETTLED), None] {
        let mut scene = Scene::new();
        scene.open();
        let layer = Scene::layer(transition, true).layer_id();
        let top_modal_layer =
            |scene: &Scene| scene.context.memory(|memory| memory.top_modal_layer());
        scene.frame(
            Vec::new(),
            Step::Closing {
                should_return: false,
            },
            transition,
        );
        assert_eq!(top_modal_layer(&scene), Some(layer));

        scene.should_focus_background = true;
        for _ in 0..SETTLED_FRAME_COUNT {
            scene.frame(Vec::new(), Step::Exiting, transition);
            assert_eq!(top_modal_layer(&scene), None);
            assert_eq!(scene.focused(), Some(Id::new(BACKGROUND)));
        }
        scene.frame(Vec::new(), Step::Closed, None);
        assert_eq!(scene.focused(), Some(Id::new(BACKGROUND)));
    }
}

#[test]
fn 닫힌_직후_다시_열면_처음_포커스를_복귀_대상으로_이어받는다() {
    let mut scene = Scene::new();
    scene.open();
    scene.frame(
        Vec::new(),
        Step::Closing {
            should_return: true,
        },
        None,
    );
    let first = scene.controls[0];
    scene
        .context
        .memory_mut(|memory| memory.request_focus(first));
    scene.focus.capture(&scene.context);
    scene.frame(Vec::new(), Step::Open, None);
    scene.frame(
        Vec::new(),
        Step::Closing {
            should_return: true,
        },
        None,
    );
    scene.frame(Vec::new(), Step::Closed, None);
    scene.frame(Vec::new(), Step::Closed, None);
    assert_eq!(scene.focused(), Some(Id::new(BACKGROUND)));
}

#[test]
fn tab은_레이어_안의_순서를_양방향으로_순환한다() {
    let mut scene = Scene::new();
    scene.open();
    let [first, second, third] = [scene.controls[0], scene.controls[1], scene.controls[2]];
    for (is_backward, expected) in [
        (false, second),
        (false, third),
        (false, first),
        (true, third),
        (true, second),
    ] {
        scene.frame(vec![tab(is_backward)], Step::Open, None);
        assert_eq!(scene.focused(), Some(expected));
    }
}

#[test]
fn 열림_닫힘_전환은_200ms_본문과_150ms_scrim을_css_ease로_표본화한다() {
    let mut presence = Presence::enter(0.0);
    let entering = presence.sample(0.0);
    assert_eq!(entering.opacity, 0.0);
    assert!((entering.scale - ENTER_SCALE).abs() < TOLERANCE);
    assert_eq!(entering.scrim_opacity, 0.0);
    assert!(entering.is_active);

    let halfway = presence.sample(CONTENT_TRANSITION_SECONDS / 2.0);
    let eased = crate::css_motion::ease(0.5);
    assert!((halfway.opacity - eased).abs() < TOLERANCE);
    assert!((halfway.scale - (ENTER_SCALE + (1.0 - ENTER_SCALE) * eased)).abs() < TOLERANCE);
    assert!(
        (halfway.scrim_opacity
            - crate::css_motion::ease((CONTENT_TRANSITION_SECONDS / 2.0 / SCRIM_SECONDS) as f32))
        .abs()
            < TOLERANCE
    );

    let scrim_settled = presence.sample(SCRIM_SECONDS);
    assert_eq!(scrim_settled.scrim_opacity, 1.0);
    assert!(scrim_settled.opacity < 1.0 && scrim_settled.is_active);
    assert_eq!(
        presence.sample(CONTENT_TRANSITION_SECONDS),
        Transition::SETTLED
    );

    let closed_at = 1.0;
    presence.target(false, closed_at);
    assert!(presence.is_present(closed_at));
    let leaving = presence.sample(closed_at + CONTENT_TRANSITION_SECONDS / 2.0);
    assert!((leaving.opacity - (1.0 - eased)).abs() < TOLERANCE);
    assert!((leaving.scale - (1.0 - (1.0 - ENTER_SCALE) * eased)).abs() < TOLERANCE);
    assert!(presence.is_present(closed_at + CONTENT_TRANSITION_SECONDS - FRAME_SECONDS));
    assert!(!presence.is_present(closed_at + CONTENT_TRANSITION_SECONDS));
    let left = presence.sample(closed_at + CONTENT_TRANSITION_SECONDS);
    assert_eq!(left.opacity, 0.0);
    assert!((left.scale - ENTER_SCALE).abs() < TOLERANCE);
}

#[test]
fn reduced_motion은_전환_없이_즉시_나타나고_사라진다() {
    let mut presence = Presence::enter(0.0);
    presence.set_reduced_motion(true);
    assert_eq!(presence.sample(0.0), Transition::SETTLED);
    assert!(presence.is_present(0.0));
    presence.target(false, FRAME_SECONDS);
    assert!(!presence.is_present(FRAME_SECONDS));
}

#[test]
fn ime_조합은_조합_이벤트가_있는_프레임과_조합_중인_프레임의_키_처리를_막는다() {
    let mut composition = Composition::default();
    assert!(!composition.observe(&[tab(false)]));
    assert!(composition.observe(&[preedit("가")]));
    assert!(composition.observe(&[]), "조합 중에는 계속 막는다");
    assert!(
        composition.observe(&[preedit("")]),
        "조합을 취소하는 프레임의 키도 막는다"
    );
    assert!(!composition.observe(&[]));
    composition.observe(&[preedit("나")]);
    assert!(
        composition.observe(&[Event::Ime(egui::ImeEvent::Commit("나".into()))]),
        "조합을 확정하는 프레임의 키도 막는다"
    );
    assert!(!composition.observe(&[]));
    composition.observe(&[preedit("다")]);
    composition.reset();
    assert!(!composition.observe(&[]));
}

#[test]
fn 전환_중인_레이어는_화면_전체_scrim과_중심_기준_확대를_그리고_해제하면_흔적을_남기지_않는다() {
    let mut scene = Scene::new();
    let layer = Scene::layer(None, true).layer_id();
    let sample = Presence::enter(0.0).sample(FRAME_SECONDS);
    assert!(sample.scale < 1.0 && sample.scrim_opacity > 0.0 && sample.scrim_opacity < 1.0);
    scene.frame(Vec::new(), Step::Open, Some(sample));
    let entering = scene.frame(Vec::new(), Step::Open, Some(sample));
    let transform = scene.context.layer_transform_to_global(layer).unwrap();
    assert!((transform.scaling - sample.scale).abs() < TOLERANCE);
    let center = egui::pos2(SCREEN[0] / 2.0, SCREEN[1] / 2.0);
    assert!((transform * center).distance(center) < GEOMETRY_TOLERANCE);
    assert!(covers_screen_with(
        &entering,
        CHROME.scrim(sample.scrim_opacity)
    ));
    assert!(scene.context.dismissal_layers().contains(&Id::new(DIALOG)));
    assert_eq!(
        scene.context.memory(|memory| memory.top_modal_layer()),
        Some(layer)
    );

    let settled = scene.frame(Vec::new(), Step::Open, Some(Transition::SETTLED));
    assert!(scene.context.layer_transform_to_global(layer).is_none());
    assert!(covers_screen_with(&settled, CHROME.scrim(1.0)));

    scene.frame(Vec::new(), Step::Open, Some(sample));
    unmount(&scene.context, layer);
    assert!(scene.context.layer_transform_to_global(layer).is_none());
    assert!(!scene.context.dismissal_layers().contains(&Id::new(DIALOG)));
}

#[test]
fn 전환이_없는_레이어는_테마_scrim과_그림자_테두리를_그대로_그린다() {
    let mut scene = Scene::new();
    scene.frame(Vec::new(), Step::Open, None);
    scene.time += 1.0;
    let output = scene.frame(Vec::new(), Step::Open, None);
    assert!(covers_screen_with(&output, CHROME.scrim(1.0)));
    let rects = rects(&output);
    assert!(
        rects
            .iter()
            .any(|rect| rect.blur_width == f32::from(SHADOW_BLUR) && rect.fill == CHROME.shadow)
    );
    assert!(rects.iter().any(|rect| {
        rect.fill == CHROME.background
            && rect.stroke == Stroke::new(BORDER_WIDTH, CHROME.border)
            && rect.corner_radius == egui::CornerRadius::same(CORNER_RADIUS)
    }));
}
