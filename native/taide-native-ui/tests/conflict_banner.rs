use egui::{Color32, Context, Event, Modifiers, PointerButton, RawInput, Rect, pos2, vec2};
use taide_model::locale::ResolvedLocale;
use taide_native_ui::conflict_banner::{
    self, BannerAction, BannerAppearance, BannerOutput, BannerVariant,
};

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 200.0;

fn frame(context: &Context, variant: BannerVariant, events: Vec<Event>) -> BannerOutput {
    let locale = ResolvedLocale {
        id: "en".into(),
        name: "English".into(),
        messages: [
            ("editor.changedOnDisk", "Changed on disk"),
            ("editor.mirrorRestored", "Draft restored"),
            (
                "editor.mirrorRestoredConflict",
                "Restored draft conflicts with disk",
            ),
            ("editor.keepMine", "Keep mine"),
            ("editor.viewDiskContent", "View disk content"),
            ("common.close", "Close"),
        ]
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect(),
        warnings: Vec::new(),
    };
    let appearance = BannerAppearance {
        error: Color32::RED,
        warning: Color32::YELLOW,
    };
    let mut result = None;
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(WIDTH, HEIGHT))),
            events,
            ..Default::default()
        },
        |ui| result = Some(conflict_banner::show(ui, &locale, &appearance, variant)),
    );
    assert!(!output.shapes.is_empty());
    output.textures_delta.clear();
    result.unwrap()
}

#[test]
fn 원본_세_배너는_정확한_선택지만_제공하고_실제_클릭을_반환한다() {
    for variant in [
        BannerVariant::MirrorRestored,
        BannerVariant::ChangedOnDisk,
        BannerVariant::MirrorRestoredConflict,
    ] {
        let context = Context::default();
        let output = frame(&context, variant, Vec::new());
        let expected = if variant == BannerVariant::MirrorRestored {
            vec![BannerAction::Dismiss]
        } else {
            vec![BannerAction::KeepMine, BannerAction::ViewDisk]
        };
        assert_eq!(
            output
                .actions
                .iter()
                .map(|(action, _)| *action)
                .collect::<Vec<_>>(),
            expected
        );
        assert!(output.action.is_none());
        for (action, rect) in output.actions {
            assert!(output.rect.contains_rect(rect));
            let position = rect.center();
            frame(
                &context,
                variant,
                vec![
                    Event::PointerMoved(position),
                    Event::PointerButton {
                        pos: position,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::default(),
                    },
                ],
            );
            let clicked = frame(
                &context,
                variant,
                vec![Event::PointerButton {
                    pos: position,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::default(),
                }],
            );
            assert_eq!(clicked.action, Some(action));
        }
    }
}
