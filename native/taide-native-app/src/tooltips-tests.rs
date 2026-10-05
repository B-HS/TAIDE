use super::*;

const SCREEN_SIZE: egui::Vec2 = egui::vec2(600.0, 400.0);
const FIRST_RECT: Rect = Rect::from_min_max(egui::pos2(80.0, 100.0), egui::pos2(120.0, 120.0));
const SECOND_RECT: Rect = Rect::from_min_max(egui::pos2(240.0, 100.0), egui::pos2(280.0, 120.0));
const BEFORE_DELAY: f64 = 0.399;
const AT_DELAY: f64 = 0.4;
const DURING_DELAY: f64 = 0.2;
const AT_CLOSE: f64 = 0.5;
const INSIDE_SKIP: f64 = 0.799;
const SECOND_CLOSE: f64 = 0.8;
const SKIP_EXPIRED: f64 = 1.1;
const NEW_DELAY: f64 = 1.5;

struct Rendered {
    open: [bool; 2],
    content: [Option<Rect>; 2],
    output: egui::FullOutput,
}

fn frame(
    context: &Context,
    provider: &Provider,
    time: f64,
    events: Vec<Event>,
    visible: [bool; 2],
) -> Rendered {
    frame_input(
        context,
        provider,
        egui::RawInput {
            time: Some(time),
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN_SIZE)),
            events,
            ..Default::default()
        },
        visible,
    )
}

fn frame_input(
    context: &Context,
    provider: &Provider,
    input: egui::RawInput,
    visible: [bool; 2],
) -> Rendered {
    let mut result = Rendered {
        open: [false; 2],
        content: [None; 2],
        output: egui::FullOutput::default(),
    };
    result.output = context.run_ui(input, |ui| {
        provider.begin_frame(ui.ctx());
        for (index, rect) in [FIRST_RECT, SECOND_RECT].into_iter().enumerate() {
            if !visible[index] {
                continue;
            }
            let response = ui.interact(
                rect,
                Id::new(("native-tooltip-test", index)),
                egui::Sense::click(),
            );
            result.open[index] = provider.is_open(&response);
            if result.open[index] {
                provider.track_layer(&response);
            }
            let mut tooltip = egui::Tooltip::for_widget(&response);
            tooltip.popup = tooltip
                .popup
                .open(result.open[index])
                .align(egui::RectAlign::BOTTOM);
            let shown = tooltip.show(|ui| {
                ui.label(format!("native tooltip {index}"));
            });
            result.content[index] = shown.map(|shown| shown.response.rect);
            provider.content(&response, result.content[index]);
        }
        provider.finish_frame(ui.ctx());
    });
    result.output.textures_delta.clear();
    result
}

fn moved(rect: Rect) -> Vec<Event> {
    vec![Event::PointerMoved(rect.center())]
}

fn escape() -> Vec<Event> {
    vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]
}

#[test]
fn tooltip_provider는_400ms와_300ms_skip을_공유하고_이동마다_deadline을_연장하지_않는다() {
    let context = Context::default();
    let provider = Provider::default();
    frame(&context, &provider, 0.0, Vec::new(), [true; 2]);
    assert_eq!(
        frame(&context, &provider, 0.0, moved(FIRST_RECT), [true; 2]).open,
        [false; 2]
    );
    assert_eq!(
        frame(
            &context,
            &provider,
            DURING_DELAY,
            moved(FIRST_RECT),
            [true; 2]
        )
        .open,
        [false; 2]
    );
    assert_eq!(
        frame(&context, &provider, BEFORE_DELAY, Vec::new(), [true; 2]).open,
        [false; 2]
    );
    assert_eq!(
        frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]).open,
        [true, false]
    );
    let opened = frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]);
    assert!(opened.content[0].is_some());
    assert!(opened.output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == "native tooltip 0")));
    assert_eq!(
        frame(&context, &provider, AT_CLOSE, escape(), [true; 2]).open,
        [false; 2]
    );
    assert!(!context.input(|input| input.key_pressed(egui::Key::Escape)));
    assert_eq!(
        frame(
            &context,
            &provider,
            INSIDE_SKIP,
            moved(SECOND_RECT),
            [true; 2]
        )
        .open,
        [false, true]
    );
    assert_eq!(
        frame(&context, &provider, SECOND_CLOSE, escape(), [true; 2]).open,
        [false; 2]
    );
    assert_eq!(
        frame(
            &context,
            &provider,
            SKIP_EXPIRED,
            moved(FIRST_RECT),
            [true; 2]
        )
        .open,
        [false; 2]
    );
    assert_eq!(
        frame(&context, &provider, NEW_DELAY, Vec::new(), [true; 2]).open,
        [true, false]
    );
}

#[test]
fn tooltip_provider는_내용으로의_포인터_이동을_유지하고_외부_이동과_unmount에_회수한다() {
    const IN_GAP: f32 = 2.0;
    const OUTSIDE: f32 = 40.0;
    const AFTER_OPEN: f64 = 0.41;
    const AFTER_TRANSIT: f64 = 0.42;
    let context = Context::default();
    let provider = Provider::default();
    frame(&context, &provider, 0.0, Vec::new(), [true; 2]);
    frame(&context, &provider, 0.0, moved(FIRST_RECT), [true; 2]);
    frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]);
    let content = frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]).content[0].unwrap();
    let gap = FIRST_RECT.center_bottom() + egui::vec2(0.0, IN_GAP);
    assert!(content.top() > gap.y);
    assert_eq!(
        frame(
            &context,
            &provider,
            AFTER_OPEN,
            vec![Event::PointerMoved(gap)],
            [true; 2]
        )
        .open,
        [true, false]
    );
    assert_eq!(
        frame(&context, &provider, AFTER_OPEN, moved(content), [true; 2]).open,
        [true, false]
    );
    let outside = content.left_top() - egui::vec2(OUTSIDE, OUTSIDE);
    assert_eq!(
        frame(
            &context,
            &provider,
            AFTER_TRANSIT,
            vec![Event::PointerMoved(outside)],
            [true; 2]
        )
        .open,
        [false; 2]
    );
    frame(&context, &provider, AT_CLOSE, moved(FIRST_RECT), [true; 2]);
    frame(&context, &provider, AT_CLOSE, Vec::new(), [true; 2]);
    assert_eq!(
        provider.0.lock().unwrap()[&ViewportId::ROOT].widgets.len(),
        2
    );
    frame(&context, &provider, AT_CLOSE, Vec::new(), [false; 2]);
    let viewports = provider.0.lock().unwrap();
    let viewport = &viewports[&ViewportId::ROOT];
    assert!(viewport.open.is_none());
    assert!(viewport.widgets.is_empty());
}

#[test]
fn tooltip_provider는_짝없는_pointer_up을_click으로_오인하지_않고_실제_down에_닫힌다() {
    let context = Context::default();
    let provider = Provider::default();
    frame(&context, &provider, 0.0, Vec::new(), [true; 2]);
    frame(&context, &provider, 0.0, moved(FIRST_RECT), [true; 2]);
    frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]);
    frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]);
    let button = |pressed| Event::PointerButton {
        pos: FIRST_RECT.center(),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    assert_eq!(
        frame(
            &context,
            &provider,
            AT_CLOSE,
            vec![button(false)],
            [true; 2]
        )
        .open,
        [true, false]
    );
    assert_eq!(
        frame(&context, &provider, AT_CLOSE, vec![button(true)], [true; 2]).open,
        [false; 2]
    );
    assert_eq!(
        frame(&context, &provider, AT_CLOSE, Vec::new(), [true; 2]).open,
        [false; 2]
    );
    assert_eq!(
        frame(
            &context,
            &provider,
            AT_CLOSE,
            vec![button(false)],
            [true; 2]
        )
        .open,
        [false; 2]
    );
}

#[test]
fn tooltip_touch_hover는_열지_않고_폐기된_지연은_재마운트에_이어지지_않는다() {
    let context = Context::default();
    let provider = Provider::default();
    frame(&context, &provider, 0.0, Vec::new(), [true; 2]);
    let mut events = moved(FIRST_RECT);
    events.push(Event::Touch {
        device_id: egui::TouchDeviceId(0),
        id: egui::TouchId(0),
        phase: egui::TouchPhase::Start,
        pos: FIRST_RECT.center(),
        force: None,
    });
    assert_eq!(
        frame(&context, &provider, 0.0, events, [true; 2]).open,
        [false; 2]
    );
    assert_eq!(
        frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]).open,
        [false; 2]
    );
    frame(
        &context,
        &provider,
        AT_DELAY,
        vec![
            Event::Touch {
                device_id: egui::TouchDeviceId(0),
                id: egui::TouchId(0),
                phase: egui::TouchPhase::End,
                pos: FIRST_RECT.center(),
                force: None,
            },
            Event::PointerGone,
        ],
        [true; 2],
    );
    assert_eq!(
        frame(&context, &provider, AT_DELAY, moved(FIRST_RECT), [true; 2]).open,
        [false; 2]
    );
    frame(&context, &provider, AT_DELAY, Vec::new(), [false; 2]);
    assert!(
        provider.0.lock().unwrap()[&ViewportId::ROOT]
            .widgets
            .is_empty()
    );
    frame(&context, &provider, SECOND_CLOSE, Vec::new(), [true; 2]);
    assert_eq!(
        frame(
            &context,
            &provider,
            SECOND_CLOSE,
            moved(FIRST_RECT),
            [true; 2]
        )
        .open,
        [false; 2]
    );
    assert_eq!(
        frame(&context, &provider, SKIP_EXPIRED, Vec::new(), [true; 2]).open,
        [false; 2]
    );
    const REMOUNT_DEADLINE: f64 = 1.21;
    assert_eq!(
        frame(&context, &provider, REMOUNT_DEADLINE, Vec::new(), [true; 2]).open,
        [true, false]
    );
}

#[test]
fn tooltip_창별_지연과_skip은_격리되고_창제거와_context_교체에_회수된다() {
    let context = Context::default();
    let provider = Provider::default();
    let root = ViewportId::ROOT;
    let child = ViewportId::from_hash_of("native-tooltip-aux");
    let run = |context: &Context, viewport, live: &[ViewportId], time, events| {
        frame_input(
            context,
            &provider,
            egui::RawInput {
                viewport_id: viewport,
                viewports: live
                    .iter()
                    .map(|id| {
                        (
                            *id,
                            egui::ViewportInfo {
                                parent: (*id != root).then_some(root),
                                ..Default::default()
                            },
                        )
                    })
                    .collect(),
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN_SIZE)),
                events,
                ..Default::default()
            },
            [true; 2],
        )
    };
    run(&context, root, &[root, child], 0.0, Vec::new());
    run(&context, child, &[root, child], 0.0, Vec::new());
    run(&context, root, &[root, child], 0.0, moved(FIRST_RECT));
    assert_eq!(
        run(&context, child, &[root, child], AT_DELAY, moved(FIRST_RECT)).open,
        [false; 2]
    );
    assert_eq!(
        run(&context, root, &[root, child], AT_DELAY, Vec::new()).open,
        [true, false]
    );
    run(&context, root, &[root, child], AT_CLOSE, escape());
    assert_eq!(
        run(&context, child, &[root, child], AT_CLOSE, Vec::new()).open,
        [false; 2]
    );
    run(&context, root, &[root], AT_CLOSE, Vec::new());
    assert!(!provider.0.lock().unwrap().contains_key(&child));
    run(&context, child, &[root, child], SECOND_CLOSE, Vec::new());
    assert_eq!(
        run(
            &context,
            child,
            &[root, child],
            SECOND_CLOSE,
            moved(FIRST_RECT)
        )
        .open,
        [false; 2]
    );
    let replacement = Context::default();
    run(&replacement, root, &[root], SECOND_CLOSE, Vec::new());
    assert_eq!(
        run(&replacement, root, &[root], SECOND_CLOSE, moved(FIRST_RECT)).open,
        [false; 2]
    );
    assert_eq!(provider.0.lock().unwrap().len(), 1);
    assert_eq!(
        run(&replacement, root, &[root], SKIP_EXPIRED, Vec::new()).open,
        [false; 2]
    );
    const REPLACEMENT_DEADLINE: f64 = 1.21;
    assert_eq!(
        run(
            &replacement,
            root,
            &[root],
            REPLACEMENT_DEADLINE,
            Vec::new()
        )
        .open,
        [true, false]
    );
}

#[test]
fn tooltip_content의_pointer_down은_유지하고_외부_down은_닫힌다() {
    let context = Context::default();
    let provider = Provider::default();
    frame(&context, &provider, 0.0, Vec::new(), [true; 2]);
    frame(&context, &provider, 0.0, moved(FIRST_RECT), [true; 2]);
    frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]);
    let content = frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]).content[0].unwrap();
    frame(&context, &provider, AT_DELAY, moved(content), [true; 2]);
    let press = |pos, pressed| Event::PointerButton {
        pos,
        pressed,
        button: egui::PointerButton::Primary,
        modifiers: egui::Modifiers::NONE,
    };
    assert_eq!(
        frame(
            &context,
            &provider,
            AT_CLOSE,
            vec![press(content.center(), true)],
            [true; 2]
        )
        .open,
        [true, false]
    );
    frame(
        &context,
        &provider,
        AT_CLOSE,
        vec![press(content.center(), false)],
        [true; 2],
    );
    assert_eq!(
        frame(
            &context,
            &provider,
            AT_CLOSE,
            vec![press(SECOND_RECT.center(), true)],
            [true; 2]
        )
        .open,
        [false; 2]
    );
}

#[test]
fn tooltip_keyboard_space는_짝있는_해제만_click으로_닫힌다() {
    let context = Context::default();
    let provider = Provider::default();
    frame(&context, &provider, 0.0, Vec::new(), [true; 2]);
    context.memory_mut(|memory| memory.request_focus(Id::new(("native-tooltip-test", 0usize))));
    assert_eq!(
        frame(&context, &provider, 0.0, Vec::new(), [true; 2]).open,
        [true, false]
    );
    let space = |pressed, repeat| Event::Key {
        key: egui::Key::Space,
        pressed,
        repeat,
        physical_key: None,
        modifiers: egui::Modifiers::NONE,
    };
    assert_eq!(
        frame(
            &context,
            &provider,
            0.0,
            vec![space(false, false)],
            [true; 2]
        )
        .open,
        [true, false]
    );
    assert_eq!(
        frame(
            &context,
            &provider,
            0.0,
            vec![space(true, false)],
            [true; 2]
        )
        .open,
        [true, false]
    );
    assert_eq!(
        frame(&context, &provider, 0.0, vec![space(true, true)], [true; 2]).open,
        [true, false]
    );
    assert_eq!(
        frame(
            &context,
            &provider,
            0.0,
            vec![space(false, false)],
            [true; 2]
        )
        .open,
        [false; 2]
    );
    assert_eq!(
        frame(&context, &provider, AT_DELAY, Vec::new(), [true; 2]).open,
        [false; 2]
    );
}

#[test]
fn tooltip_pointer_down_ref는_focus_왕복에도_up까지_유지된다() {
    let context = Context::default();
    let provider = Provider::default();
    let focus = |index: usize| {
        context.memory_mut(|memory| memory.request_focus(Id::new(("native-tooltip-test", index))))
    };
    let button = |pressed| Event::PointerButton {
        pos: FIRST_RECT.center(),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&context, &provider, 0.0, Vec::new(), [true; 2]);
    frame(
        &context,
        &provider,
        0.0,
        vec![Event::PointerMoved(FIRST_RECT.center()), button(true)],
        [true; 2],
    );
    focus(1);
    assert_eq!(
        frame(&context, &provider, 0.0, Vec::new(), [true; 2]).open,
        [false, true]
    );
    focus(0);
    assert_eq!(
        frame(&context, &provider, 0.0, Vec::new(), [true; 2]).open,
        [false; 2]
    );
    frame(&context, &provider, 0.0, vec![button(false)], [true; 2]);
    focus(1);
    assert_eq!(
        frame(&context, &provider, 0.0, Vec::new(), [true; 2]).open,
        [false, true]
    );
    focus(0);
    assert_eq!(
        frame(&context, &provider, 0.0, Vec::new(), [true; 2]).open,
        [true, false]
    );
}
