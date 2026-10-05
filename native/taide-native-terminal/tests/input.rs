use taide_native_terminal::{
    Effect, Limits, Mode, Size, TerminalCore, TerminalEvent,
    input::{
        InputAction, InputError, Key, Modifiers, MouseAction, MouseButton, MouseInput, NativeInput,
    },
};

const COLUMNS: u16 = 20;
const ROWS: u16 = 4;
const HISTORY: usize = 8;
const CAPACITY: usize = 64 * 1024;
const COMMITTED: &str = "한글 日本語 中文 e\u{301} 𐐀";
const MOUSE_COLUMNS: u16 = 240;
const DEFAULT_LAST_COLUMN: u16 = 222;

fn written(text: &str) -> InputAction {
    InputAction::Write(text.as_bytes().to_vec())
}

#[test]
fn mouse_protocol은_xterm의_x10_비활성화와_무시된_utf8_모드를_보존한다() {
    let mut core = TerminalCore::new(
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Limits::default(),
    )
    .unwrap();
    let modified = Modifiers {
        shift: true,
        alt: true,
        control: true,
        command: true,
    };
    let mouse = |action| {
        NativeInput::Mouse(MouseInput {
            column: 0,
            row: 0,
            pixels: None,
            action,
            modifiers: modified,
        })
    };
    core.advance(b"\x1b[?9h").unwrap();
    assert_eq!(
        core.encode_input(mouse(MouseAction::Press(MouseButton::Left)), CAPACITY),
        Ok(InputAction::Write(b"\x1b[M !!".to_vec()))
    );
    for action in [
        MouseAction::Release(MouseButton::Left),
        MouseAction::Move(Some(MouseButton::Left)),
        MouseAction::WheelUp,
    ] {
        assert_eq!(
            core.encode_input(mouse(action), CAPACITY),
            Ok(InputAction::Ignore)
        );
    }
    core.advance(b"\x1b[?1003h\x1b[?1000l").unwrap();
    assert_eq!(
        core.encode_input(mouse(MouseAction::Press(MouseButton::Left)), CAPACITY),
        Ok(InputAction::Ignore)
    );
    core.advance(b"\x1b[?1000h\x1b[?1006h\x1b[?1005h\x1b[?1015h")
        .unwrap();
    assert_eq!(
        core.encode_input(mouse(MouseAction::Press(MouseButton::Left)), CAPACITY),
        Ok(written("\x1b[<28;1;1M"))
    );
    core.advance(b"\x1b[?1016h").unwrap();
    assert!(!core.mode().unwrap().contains(Mode::SGR_MOUSE));
    let pixel = MouseInput {
        column: 0,
        row: 0,
        pixels: Some((0, 0)),
        action: MouseAction::Press(MouseButton::Left),
        modifiers: modified,
    };
    assert_eq!(
        core.encode_input(NativeInput::Mouse(pixel), CAPACITY),
        Ok(written("\x1b[<28;0;0M"))
    );
    assert_eq!(
        core.encode_input(mouse(MouseAction::Press(MouseButton::Left)), CAPACITY),
        Err(InputError::InvalidCoordinates)
    );
    assert_eq!(
        core.encode_input(
            NativeInput::Mouse(MouseInput {
                pixels: Some((1234, 5678)),
                action: MouseAction::Release(MouseButton::Middle),
                ..pixel
            }),
            CAPACITY
        ),
        Ok(written("\x1b[<29;1234;5678m"))
    );
    let replies = core
        .advance(b"\x1b[?9$p\x1b[?1000$p\x1b[?1005$p\x1b[?1006$p\x1b[?1015$p\x1b[?1016$p")
        .unwrap()
        .into_iter()
        .filter_map(|effect| match effect {
            Effect::Terminal(TerminalEvent::PtyWrite(reply)) => Some(reply),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        replies,
        [
            "\x1b[?9;2$y",
            "\x1b[?1000;1$y",
            "\x1b[?1005;4$y",
            "\x1b[?1006;2$y",
            "\x1b[?1015;4$y",
            "\x1b[?1016;1$y",
        ]
    );
    core.advance(b"\x1b[?1006l").unwrap();
    assert!(!core.mode().unwrap().contains(Mode::SGR_PIXEL_MOUSE));
    assert_eq!(
        core.encode_input(mouse(MouseAction::Press(MouseButton::Left)), CAPACITY),
        Ok(InputAction::Write(b"\x1b[M<!!".to_vec()))
    );
    core.advance(b"\x1b[?1006h\x1b[?1016l").unwrap();
    assert!(!core.mode().unwrap().contains(Mode::SGR_MOUSE));
    core.advance(b"\x1b[?1006h\x1b[?9h").unwrap();
    assert_eq!(
        core.encode_input(mouse(MouseAction::Press(MouseButton::Left)), CAPACITY),
        Ok(written("\x1b[<0;1;1M"))
    );
    core.advance(b"\x1b[?1002l").unwrap();
    assert_eq!(
        core.encode_input(mouse(MouseAction::Press(MouseButton::Left)), CAPACITY),
        Ok(InputAction::Ignore)
    );
}

#[test]
fn mouse_encoder는_actual_live_tracking_binary_sgr_modifier와_상한을_보존한다() {
    let mut core = TerminalCore::new(
        Size {
            columns: MOUSE_COLUMNS,
            rows: ROWS,
        },
        HISTORY,
        Limits::default(),
    )
    .unwrap();
    let input = |column, row, action, modifiers| {
        NativeInput::Mouse(MouseInput {
            column,
            row,
            pixels: None,
            action,
            modifiers,
        })
    };
    let plain = Modifiers::default();
    let left = MouseButton::Left;
    assert_eq!(
        core.encode_input(input(0, 0, MouseAction::Press(left), plain), 0),
        Ok(InputAction::Ignore)
    );
    core.advance(b"\x1b[?1000h").unwrap();
    assert_eq!(
        core.encode_input(input(0, 0, MouseAction::Press(left), plain), CAPACITY),
        Ok(InputAction::Write(b"\x1b[M !!".to_vec()))
    );
    assert_eq!(
        core.encode_input(input(0, 0, MouseAction::Release(left), plain), CAPACITY),
        Ok(InputAction::Write(b"\x1b[M#!!".to_vec()))
    );
    assert_eq!(
        core.encode_input(input(0, 0, MouseAction::Move(Some(left)), plain), CAPACITY),
        Ok(InputAction::Ignore)
    );
    assert_eq!(
        core.encode_input(
            input(DEFAULT_LAST_COLUMN, 0, MouseAction::Press(left), plain),
            CAPACITY
        ),
        Ok(InputAction::Write(b"\x1b[M \xff!".to_vec()))
    );
    assert_eq!(
        core.encode_input(
            input(DEFAULT_LAST_COLUMN + 1, 0, MouseAction::Press(left), plain),
            CAPACITY
        ),
        Ok(InputAction::Ignore)
    );
    for action in [MouseAction::WheelUp, MouseAction::WheelDown] {
        let expected = if action == MouseAction::WheelUp {
            b"\x1b[M`!!"
        } else {
            b"\x1b[Ma!!"
        };
        assert_eq!(
            core.encode_input(input(0, 0, action, plain), expected.len()),
            Ok(InputAction::Write(expected.to_vec()))
        );
        assert_eq!(
            core.encode_input(input(0, 0, action, plain), expected.len() - 1),
            Err(InputError::Capacity)
        );
    }
    core.advance(b"\x1b[?1002h\x1b[?1006h").unwrap();
    let modified = Modifiers {
        shift: true,
        alt: true,
        control: true,
        command: true,
    };
    for (action, expected) in [
        (MouseAction::Press(MouseButton::Middle), "\x1b[<29;3;4M"),
        (MouseAction::Release(MouseButton::Right), "\x1b[<30;3;4m"),
        (MouseAction::Move(Some(left)), "\x1b[<60;3;4M"),
        (MouseAction::WheelUp, "\x1b[<92;3;4M"),
        (MouseAction::WheelDown, "\x1b[<93;3;4M"),
    ] {
        assert_eq!(
            core.encode_input(input(2, ROWS - 1, action, modified), expected.len()),
            Ok(written(expected))
        );
        assert_eq!(
            core.encode_input(input(2, ROWS - 1, action, modified), expected.len() - 1),
            Err(InputError::Capacity)
        );
    }
    assert_eq!(
        core.encode_input(input(0, 0, MouseAction::Move(None), plain), CAPACITY),
        Ok(InputAction::Ignore)
    );
    assert_eq!(
        core.encode_input(
            input(MOUSE_COLUMNS - 1, ROWS - 1, MouseAction::Press(left), plain),
            CAPACITY
        ),
        Ok(written("\x1b[<0;240;4M"))
    );
    core.advance(b"\x1b[?1003h").unwrap();
    assert_eq!(
        core.encode_input(input(2, 1, MouseAction::Move(None), plain), CAPACITY),
        Ok(written("\x1b[<35;3;2M"))
    );
    for (column, row) in [(MOUSE_COLUMNS, 0), (0, ROWS), (u16::MAX, u16::MAX)] {
        assert_eq!(
            core.encode_input(
                input(column, row, MouseAction::Press(left), plain),
                CAPACITY
            ),
            Ok(InputAction::Ignore)
        );
    }
    core.advance(b"\x1b[?1006l").unwrap();
    assert_eq!(
        core.encode_input(input(2, 1, MouseAction::Move(None), plain), CAPACITY),
        Ok(InputAction::Write(b"\x1b[MC#\"".to_vec()))
    );
    core.advance(b"\x1b[?1005h").unwrap();
    assert_eq!(
        core.encode_input(input(2, 1, MouseAction::Press(left), plain), CAPACITY),
        Ok(InputAction::Write(b"\x1b[M #\"".to_vec()))
    );
    core.advance(b"\x1b[?1005l\x1b[?1003l").unwrap();
    assert_eq!(
        core.encode_input(input(2, 1, MouseAction::Press(left), plain), CAPACITY),
        Ok(InputAction::Ignore)
    );
}

#[test]
fn actual_core의_live_mode_입력은_paste_focus_utf8와_retire를_보존한다() {
    let size = Size {
        columns: COLUMNS,
        rows: ROWS,
    };
    let mut core = TerminalCore::new(size, HISTORY, Limits::default()).unwrap();
    let plain = Modifiers::default();
    let arrow = NativeInput::Key {
        key: Key::ArrowUp,
        modifiers: plain,
    };
    assert_eq!(core.encode_input(arrow, CAPACITY), Ok(written("\x1b[A")));
    core.advance(b"\x1b[?1h\x1b[?2004h\x1b[?1004h").unwrap();
    assert_eq!(
        core.encode_input(
            NativeInput::Key {
                key: Key::ArrowUp,
                modifiers: plain
            },
            CAPACITY
        ),
        Ok(written("\x1bOA"))
    );
    assert_eq!(
        core.encode_input(
            NativeInput::Key {
                key: Key::ArrowUp,
                modifiers: Modifiers {
                    control: true,
                    ..plain
                }
            },
            CAPACITY
        ),
        Ok(written("\x1b[1;5A"))
    );
    assert_eq!(
        core.encode_input(
            NativeInput::Key {
                key: Key::Enter,
                modifiers: Modifiers {
                    shift: true,
                    ..plain
                }
            },
            CAPACITY
        ),
        Ok(written("\n"))
    );
    assert_eq!(
        core.encode_input(NativeInput::Preedit(COMMITTED), CAPACITY),
        Ok(InputAction::Ignore)
    );
    assert_eq!(
        core.encode_input(NativeInput::CommittedText(COMMITTED), COMMITTED.len()),
        Ok(written(COMMITTED))
    );
    assert_eq!(
        core.encode_input(NativeInput::CommittedText(COMMITTED), COMMITTED.len() - 1),
        Err(InputError::Capacity)
    );
    let pasted = "한\r\n日\n中\re\u{301}𐐀";
    let expected = "\x1b[200~한\r日\r中\re\u{301}𐐀\x1b[201~";
    assert_eq!(
        core.encode_input(NativeInput::Paste(pasted), expected.len()),
        Ok(written(expected))
    );
    assert_eq!(
        core.encode_input(NativeInput::Paste(pasted), expected.len() - 1),
        Err(InputError::Capacity)
    );
    assert_eq!(
        core.encode_input(NativeInput::Focus(true), CAPACITY),
        Ok(written("\x1b[I"))
    );
    assert_eq!(
        core.encode_input(NativeInput::Focus(false), CAPACITY),
        Ok(written("\x1b[O"))
    );
    assert_eq!(
        core.encode_input(
            NativeInput::Key {
                key: Key::Character('a'),
                modifiers: Modifiers {
                    command: true,
                    ..plain
                }
            },
            CAPACITY
        ),
        Ok(InputAction::SelectAll)
    );
    assert_eq!(
        core.encode_input(
            NativeInput::Key {
                key: Key::PageUp,
                modifiers: Modifiers {
                    shift: true,
                    ..plain
                }
            },
            CAPACITY
        ),
        Ok(InputAction::PageUp)
    );
    core.advance(b"\x1b[?1l\x1b[?2004l\x1b[?1004l").unwrap();
    assert_eq!(
        core.encode_input(
            NativeInput::Key {
                key: Key::ArrowUp,
                modifiers: plain
            },
            CAPACITY
        ),
        Ok(written("\x1b[A"))
    );
    assert_eq!(
        core.encode_input(NativeInput::Paste(pasted), CAPACITY),
        Ok(written("한\r日\r中\re\u{301}𐐀"))
    );
    assert_eq!(
        core.encode_input(NativeInput::Focus(false), CAPACITY),
        Ok(InputAction::Ignore)
    );
    let mut retired = TerminalCore::new(
        size,
        HISTORY,
        Limits {
            effect_count: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(retired.advance(b"\x07\x07").is_err());
    assert_eq!(
        retired.encode_input(NativeInput::CommittedText(COMMITTED), CAPACITY),
        Err(InputError::Retired)
    );
    assert_eq!(
        retired.encode_input(NativeInput::Preedit(COMMITTED), CAPACITY),
        Err(InputError::Retired)
    );
}
