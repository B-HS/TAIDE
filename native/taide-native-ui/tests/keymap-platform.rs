use egui::{Event, Key, Modifiers, RawInput, os::OperatingSystem};
use taide_native_ui::keymap::{Context, Decision, Route, Windows};

#[cfg(feature = "native-host")]
#[test]
fn 수동과_전체_접기_기본_chord는_플랫폼과_재지정과_ime를_따른다() {
    for os in [
        OperatingSystem::Mac,
        OperatingSystem::Windows,
        OperatingSystem::Nix,
    ] {
        let context = egui::Context::default();
        context.set_os(os);
        let modifiers = if os == OperatingSystem::Mac {
            Modifiers::MAC_CMD | Modifiers::COMMAND
        } else {
            Modifiers::CTRL | Modifiers::COMMAND
        };
        for (key, expected) in [
            (Key::Comma, "monaco.editor.createFoldingRangeFromSelection"),
            (Key::Period, "monaco.editor.removeManualFoldingRanges"),
            (Key::Num0, "monaco.editor.foldAll"),
        ] {
            let mut windows = Windows::default();
            editor_event(&mut windows, &context, Key::K, modifiers, None, false);
            let decisions = editor_event(&mut windows, &context, key, modifiers, None, false);
            assert!(
                decisions.contains(&Decision::ResolveChord(expected.into())),
                "{decisions:?}"
            );
        }
        let mut windows = Windows::default();
        editor_event(&mut windows, &context, Key::K, Modifiers::NONE, None, true);
        assert!(
            !editor_event(&mut windows, &context, Key::Comma, modifiers, None, false).contains(
                &Decision::ResolveChord("monaco.editor.createFoldingRangeFromSelection".into())
            )
        );
        let mut windows = Windows::default();
        let overrides = r#"[{"actionId":"monaco.editor.createFoldingRangeFromSelection","key":"r","mods":["mod"]}]"#;
        assert!(
            editor_event(
                &mut windows,
                &context,
                Key::R,
                modifiers,
                Some(overrides),
                false
            )
            .contains(&Decision::Dispatch(
                "monaco.editor.createFoldingRangeFromSelection".into()
            ))
        );
    }
}

#[cfg(feature = "native-host")]
#[test]
fn 찾기_기본_키와_재지정은_호스트_플랫폼과_활성_editor_경로를_따른다() {
    for (os, key, modifiers, expected) in [
        (
            OperatingSystem::Mac,
            Key::F3,
            Modifiers::NONE,
            "monaco.editor.action.nextMatchFindAction",
        ),
        (
            OperatingSystem::Mac,
            Key::G,
            Modifiers::MAC_CMD | Modifiers::COMMAND | Modifiers::SHIFT,
            "monaco.editor.action.previousMatchFindAction",
        ),
        (
            OperatingSystem::Mac,
            Key::F,
            Modifiers::MAC_CMD | Modifiers::COMMAND | Modifiers::ALT,
            "monaco.editor.action.startFindReplaceAction",
        ),
        (
            OperatingSystem::Windows,
            Key::H,
            Modifiers::CTRL | Modifiers::COMMAND,
            "monaco.editor.action.startFindReplaceAction",
        ),
        (
            OperatingSystem::Windows,
            Key::F3,
            Modifiers::CTRL | Modifiers::COMMAND,
            "monaco.editor.action.nextSelectionMatchFindAction",
        ),
    ] {
        let context = egui::Context::default();
        context.set_os(os);
        let mut windows = Windows::default();
        let decisions = editor_event(&mut windows, &context, key, modifiers, None, false);
        assert!(
            decisions.contains(&Decision::Dispatch(expected.into())),
            "{decisions:?}"
        );
    }
    let context = egui::Context::default();
    context.set_os(OperatingSystem::Mac);
    let mut windows = Windows::default();
    let overrides =
        r#"[{"actionId":"monaco.editor.action.nextMatchFindAction","key":"r","mods":["mod"]}]"#;
    let decisions = editor_event(
        &mut windows,
        &context,
        Key::R,
        Modifiers::MAC_CMD | Modifiers::COMMAND,
        Some(overrides),
        false,
    );
    assert!(decisions.contains(&Decision::Dispatch(
        "monaco.editor.action.nextMatchFindAction".into()
    )));
    let decisions = editor_event(
        &mut windows,
        &context,
        Key::F3,
        Modifiers::NONE,
        Some(overrides),
        false,
    );
    assert!(!decisions.contains(&Decision::Dispatch(
        "monaco.editor.action.nextMatchFindAction".into()
    )));
}

#[test]
fn 공용_route는_컴파일_target이_아닌_실제_host의_command_modifier를_사용한다() {
    for (os, modifiers, opposite) in [
        (
            OperatingSystem::Mac,
            Modifiers::MAC_CMD | Modifiers::COMMAND,
            Modifiers::CTRL | Modifiers::COMMAND,
        ),
        (
            OperatingSystem::Windows,
            Modifiers::CTRL | Modifiers::COMMAND,
            Modifiers::MAC_CMD | Modifiers::COMMAND,
        ),
    ] {
        let context = egui::Context::default();
        context.set_os(os);
        let mut windows = Windows::default();
        for (modifiers, expected) in [
            (modifiers, Decision::Dispatch("quick-open".into())),
            (opposite, Decision::None),
        ] {
            let event = Event::Key {
                key: Key::P,
                physical_key: Some(Key::P),
                pressed: true,
                repeat: false,
                modifiers,
            };
            let mut output = context.run_ui(
                RawInput {
                    events: vec![event.clone()],
                    ..Default::default()
                },
                |ui| {
                    let mut observed = None;
                    let handled = windows
                        .route(
                            Route {
                                context: ui.ctx(),
                                event: &event,
                                index: 0,
                                scope: Context::default(),
                                composing: false,
                                overrides: None,
                            },
                            |decision| {
                                observed = Some(decision.clone());
                                matches!(decision, Decision::Dispatch(_))
                            },
                        )
                        .unwrap();
                    assert_eq!(observed, Some(expected.clone()));
                    assert_eq!(handled, matches!(expected, Decision::Dispatch(_)));
                },
            );
            output.textures_delta.clear();
        }
    }
}

fn editor_event(
    windows: &mut Windows,
    context: &egui::Context,
    key: Key,
    modifiers: Modifiers,
    overrides: Option<&str>,
    composing: bool,
) -> Vec<Decision> {
    let event = Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    };
    let mut decisions = Vec::new();
    let mut output = context.run_ui(
        RawInput {
            events: vec![event.clone()],
            ..Default::default()
        },
        |ui| {
            windows
                .route(
                    Route {
                        context: ui.ctx(),
                        event: &event,
                        index: 0,
                        scope: Context {
                            editor: true,
                            terminal: false,
                        },
                        composing,
                        overrides,
                    },
                    |decision| {
                        if matches!(
                            decision,
                            Decision::Dispatch(_)
                                | Decision::ResolveChord(_)
                                | Decision::EnterChord
                                | Decision::NoMatch
                        ) {
                            decisions.push(decision.clone());
                            return true;
                        }
                        false
                    },
                )
                .unwrap();
        },
    );
    output.textures_delta.clear();
    decisions
}

#[cfg(feature = "native-host")]
#[test]
fn 위치_이동_기본키는_os와_옆열기_chord와_재지정과_ime를_따른다() {
    for os in [
        OperatingSystem::Mac,
        OperatingSystem::Windows,
        OperatingSystem::Nix,
    ] {
        let context = egui::Context::default();
        context.set_os(os);
        let command = if os == OperatingSystem::Mac {
            Modifiers::MAC_CMD | Modifiers::COMMAND
        } else {
            Modifiers::CTRL | Modifiers::COMMAND
        };
        for (key, modifiers, expected) in [
            (
                Key::F12,
                Modifiers::NONE,
                "monaco.editor.action.revealDefinition",
            ),
            (
                Key::F12,
                Modifiers::SHIFT,
                "monaco.editor.action.goToReferences",
            ),
            (Key::F12, command, "monaco.editor.action.goToImplementation"),
            (
                Key::F12,
                command | Modifiers::SHIFT,
                "monaco.editor.action.peekImplementation",
            ),
            (
                if os == OperatingSystem::Nix {
                    Key::F10
                } else {
                    Key::F12
                },
                if os == OperatingSystem::Nix {
                    command | Modifiers::SHIFT
                } else {
                    Modifiers::ALT
                },
                "monaco.editor.action.peekDefinition",
            ),
        ] {
            let mut windows = Windows::default();
            assert!(
                editor_event(&mut windows, &context, key, modifiers, None, false)
                    .contains(&Decision::Dispatch(expected.into()))
            );
            assert_eq!(
                editor_event(
                    &mut Windows::default(),
                    &context,
                    key,
                    modifiers,
                    None,
                    true
                )
                .contains(&Decision::Dispatch(expected.into())),
                modifiers.mac_cmd || modifiers.ctrl
            );
        }
        let mut windows = Windows::default();
        assert!(
            editor_event(&mut windows, &context, Key::K, command, None, false)
                .contains(&Decision::EnterChord)
        );
        assert!(
            editor_event(
                &mut windows,
                &context,
                Key::F12,
                Modifiers::NONE,
                None,
                false
            )
            .contains(&Decision::ResolveChord(
                "monaco.editor.action.revealDefinitionAside".into()
            ))
        );
        let overrides =
            r#"[{"actionId":"monaco.editor.action.revealDefinition","key":"r","mods":["mod"]}]"#;
        let mut windows = Windows::default();
        assert!(
            editor_event(
                &mut windows,
                &context,
                Key::R,
                command,
                Some(overrides),
                false
            )
            .contains(&Decision::Dispatch(
                "monaco.editor.action.revealDefinition".into()
            ))
        );
        assert!(
            !editor_event(
                &mut windows,
                &context,
                Key::F12,
                Modifiers::NONE,
                Some(overrides),
                false
            )
            .contains(&Decision::Dispatch(
                "monaco.editor.action.revealDefinition".into()
            ))
        );
    }
}

#[test]
fn 편집기_기본_키는_실제_os의_줄_복사와_주석과_구두점_규칙을_사용한다() {
    for os in [
        OperatingSystem::Mac,
        OperatingSystem::Windows,
        OperatingSystem::Nix,
    ] {
        let context = egui::Context::default();
        context.set_os(os);
        let mut windows = Windows::default();
        let command = if os == OperatingSystem::Mac {
            Modifiers::MAC_CMD | Modifiers::COMMAND
        } else {
            Modifiers::CTRL | Modifiers::COMMAND
        };
        let copy = if os == OperatingSystem::Nix {
            command | Modifiers::SHIFT | Modifiers::ALT
        } else {
            Modifiers::SHIFT | Modifiers::ALT
        };
        assert_eq!(
            editor_event(&mut windows, &context, Key::ArrowUp, copy, None, false),
            [Decision::Dispatch(
                "monaco.editor.action.copyLinesUpAction".into()
            )]
        );
        let comment = if os == OperatingSystem::Nix {
            command | Modifiers::SHIFT
        } else {
            Modifiers::ALT | Modifiers::SHIFT
        };
        assert_eq!(
            editor_event(&mut windows, &context, Key::A, comment, None, false),
            [Decision::Dispatch(
                "monaco.editor.action.blockComment".into()
            )]
        );
        assert_eq!(
            editor_event(
                &mut windows,
                &context,
                Key::OpenBracket,
                command,
                None,
                false
            ),
            [Decision::Dispatch(
                "monaco.editor.action.outdentLines".into()
            )]
        );
    }
}

#[test]
fn 주석_chord와_재지정과_해제는_편집기_경로에서_처리한다() {
    let context = egui::Context::default();
    context.set_os(OperatingSystem::Mac);
    let mut windows = Windows::default();
    let command = Modifiers::MAC_CMD | Modifiers::COMMAND;
    assert_eq!(
        editor_event(&mut windows, &context, Key::K, command, None, false),
        [Decision::EnterChord]
    );
    assert_eq!(
        editor_event(&mut windows, &context, Key::C, command, None, false),
        [Decision::ResolveChord(
            "monaco.editor.action.addCommentLine".into()
        )]
    );
    let rebound = r#"[{"actionId":"monaco.editor.action.commentLine","key":"j","mods":["mod"],"chord":{"key":"r","mods":["mod"]}},{"actionId":"monaco.editor.action.copyLinesUpAction","key":"","mods":[]}]"#;
    assert!(
        editor_event(
            &mut windows,
            &context,
            Key::Slash,
            command,
            Some(rebound),
            false
        )
        .is_empty()
    );
    assert!(
        editor_event(
            &mut windows,
            &context,
            Key::ArrowUp,
            Modifiers::SHIFT | Modifiers::ALT,
            Some(rebound),
            false
        )
        .is_empty()
    );
    assert_eq!(
        editor_event(
            &mut windows,
            &context,
            Key::J,
            command,
            Some(rebound),
            false
        ),
        [Decision::EnterChord]
    );
    assert_eq!(
        editor_event(
            &mut windows,
            &context,
            Key::R,
            command,
            Some(rebound),
            false
        ),
        [Decision::ResolveChord(
            "monaco.editor.action.commentLine".into()
        )]
    );
    assert!(
        editor_event(
            &mut windows,
            &context,
            Key::A,
            Modifiers::SHIFT | Modifiers::ALT,
            None,
            true
        )
        .is_empty()
    );
}

#[test]
fn 커서_추가와_선택_키는_원본_플랫폼_분기와_chord를_사용한다() {
    for os in [
        OperatingSystem::Mac,
        OperatingSystem::Windows,
        OperatingSystem::Nix,
    ] {
        let context = egui::Context::default();
        context.set_os(os);
        let mut windows = Windows::default();
        let command = if os == OperatingSystem::Mac {
            Modifiers::MAC_CMD | Modifiers::COMMAND
        } else {
            Modifiers::CTRL | Modifiers::COMMAND
        };
        let add = if os == OperatingSystem::Nix {
            Modifiers::SHIFT | Modifiers::ALT
        } else {
            command | Modifiers::ALT
        };
        assert_eq!(
            editor_event(&mut windows, &context, Key::ArrowDown, add, None, false),
            [Decision::Dispatch(
                "monaco.editor.action.insertCursorBelow".into()
            )]
        );
        assert_eq!(
            editor_event(&mut windows, &context, Key::D, command, None, false),
            [Decision::Dispatch(
                "monaco.editor.action.addSelectionToNextFindMatch".into()
            )]
        );
        assert_eq!(
            editor_event(&mut windows, &context, Key::U, command, None, false),
            [Decision::Dispatch("monaco.cursorUndo".into())]
        );
        assert_eq!(
            editor_event(&mut windows, &context, Key::K, command, None, false),
            [Decision::EnterChord]
        );
        assert_eq!(
            editor_event(&mut windows, &context, Key::B, command, None, false),
            [Decision::ResolveChord(
                "monaco.editor.action.setSelectionAnchor".into()
            )]
        );
    }
}
