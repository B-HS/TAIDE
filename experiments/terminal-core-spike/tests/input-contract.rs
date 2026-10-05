use taide_terminal_core_spike::AlacrittyProbe;
use taide_terminal_core_spike::input::{InputAction, InputError, Key, Modifiers, NativeInput};

const MAX_INPUT_BYTES: usize = 64 * 1024;
const FUNCTION_TILDE_CODES: &[u8] = &[15, 17, 18, 19, 20, 21, 23, 24];
const FIRST_TILDE_FUNCTION: u8 = 5;
const UTF8_TEXT: &str = "한글 日本語 中文 e\u{301} 𐐀";

fn send(probe: &AlacrittyProbe, key: Key, modifiers: Modifiers) -> InputAction {
    probe
        .encode_input(NativeInput::Key { key, modifiers }, MAX_INPUT_BYTES)
        .unwrap()
}

fn written(text: &str) -> InputAction {
    InputAction::Write(text.as_bytes().to_vec())
}

#[test]
fn live_mode_input_contract는_키_붙여넣기_focus_확정_text와_상한을_보존한다() {
    let mut probe = AlacrittyProbe::default();
    let plain = Modifiers::default();
    let shift = Modifiers {
        shift: true,
        ..plain
    };
    let alt = Modifiers { alt: true, ..plain };
    let control = Modifiers {
        control: true,
        ..plain
    };
    let command = Modifiers {
        command: true,
        ..plain
    };
    for (key, suffix) in [
        (Key::ArrowUp, 'A'),
        (Key::ArrowDown, 'B'),
        (Key::ArrowRight, 'C'),
        (Key::ArrowLeft, 'D'),
        (Key::Home, 'H'),
        (Key::End, 'F'),
    ] {
        assert_eq!(send(&probe, key, plain), written(&format!("\x1b[{suffix}")));
    }
    probe.advance(b"\x1b[?1h");
    for (key, suffix) in [
        (Key::ArrowUp, 'A'),
        (Key::ArrowDown, 'B'),
        (Key::ArrowRight, 'C'),
        (Key::ArrowLeft, 'D'),
        (Key::Home, 'H'),
        (Key::End, 'F'),
    ] {
        assert_eq!(send(&probe, key, plain), written(&format!("\x1bO{suffix}")));
        assert_eq!(
            send(&probe, key, shift),
            written(&format!("\x1b[1;2{suffix}"))
        );
        assert_eq!(
            send(&probe, key, alt),
            written(&format!("\x1b[1;3{suffix}"))
        );
        assert_eq!(
            send(&probe, key, control),
            written(&format!("\x1b[1;5{suffix}"))
        );
    }
    probe.advance(b"\x1b[?1l");
    assert_eq!(send(&probe, Key::ArrowUp, plain), written("\x1b[A"));
    assert_eq!(send(&probe, Key::ArrowUp, command), InputAction::Ignore);
    for (number, suffix) in ['P', 'Q', 'R', 'S'].into_iter().enumerate() {
        let number = u8::try_from(number + 1).unwrap();
        assert_eq!(
            send(&probe, Key::Function(number), plain),
            written(&format!("\x1bO{suffix}"))
        );
        assert_eq!(
            send(&probe, Key::Function(number), control),
            written(&format!("\x1b[1;5{suffix}"))
        );
    }
    for (index, code) in FUNCTION_TILDE_CODES.iter().enumerate() {
        let number = u8::try_from(index).unwrap() + FIRST_TILDE_FUNCTION;
        assert_eq!(
            send(&probe, Key::Function(number), plain),
            written(&format!("\x1b[{code}~"))
        );
        assert_eq!(
            send(&probe, Key::Function(number), alt),
            written(&format!("\x1b[{code};3~"))
        );
    }
    assert_eq!(
        probe.encode_input(
            NativeInput::Key {
                key: Key::Function(0),
                modifiers: plain
            },
            MAX_INPUT_BYTES
        ),
        Err(InputError::UnsupportedKey)
    );
    assert_eq!(send(&probe, Key::PageUp, shift), InputAction::PageUp);
    assert_eq!(send(&probe, Key::PageDown, shift), InputAction::PageDown);
    assert_eq!(send(&probe, Key::PageUp, control), written("\x1b[5;5~"));
    assert_eq!(send(&probe, Key::PageDown, alt), written("\x1b[6~"));
    assert_eq!(
        send(&probe, Key::Insert, shift),
        InputAction::ClipboardShortcut
    );
    assert_eq!(send(&probe, Key::Delete, control), written("\x1b[3;5~"));
    assert_eq!(send(&probe, Key::Tab, shift), written("\x1b[Z"));
    assert_eq!(send(&probe, Key::Tab, plain), written("\t"));
    assert_eq!(send(&probe, Key::Backspace, alt), written("\x1b\x7f"));
    assert_eq!(send(&probe, Key::Backspace, control), written("\x08"));
    assert_eq!(send(&probe, Key::Enter, plain), written("\r"));
    assert_eq!(send(&probe, Key::Enter, shift), written("\n"));
    assert_eq!(send(&probe, Key::Enter, alt), written("\x1b\r"));
    assert_eq!(send(&probe, Key::Escape, alt), written("\x1b\x1b"));
    for (character, expected) in [
        ('c', "\x03"),
        (' ', "\0"),
        ('[', "\x1b"),
        ('\\', "\x1c"),
        (']', "\x1d"),
        ('3', "\x1b"),
        ('8', "\x7f"),
    ] {
        assert_eq!(
            send(&probe, Key::Character(character), control),
            written(expected)
        );
    }
    assert_eq!(
        send(&probe, Key::Character('a'), command),
        InputAction::SelectAll
    );
    assert_eq!(
        send(&probe, Key::Character('c'), command),
        InputAction::Ignore
    );
    assert_eq!(send(&probe, Key::Character('x'), alt), written("\x1bx"));
    assert_eq!(
        probe.encode_input(NativeInput::Preedit(UTF8_TEXT), MAX_INPUT_BYTES),
        Ok(InputAction::Ignore)
    );
    assert_eq!(
        probe.encode_input(NativeInput::CommittedText(UTF8_TEXT), MAX_INPUT_BYTES),
        Ok(written(UTF8_TEXT))
    );
    let paste = "한\r\n日\n中\re\u{301}𐐀";
    let normalized = "한\r日\r中\re\u{301}𐐀";
    assert_eq!(
        probe.encode_input(NativeInput::Paste(paste), MAX_INPUT_BYTES),
        Ok(written(normalized))
    );
    assert_eq!(
        probe.encode_input(NativeInput::Focus(true), MAX_INPUT_BYTES),
        Ok(InputAction::Ignore)
    );
    probe.advance(b"\x1b[?2004h\x1b[?1004h");
    let bracketed = format!("\x1b[200~{normalized}\x1b[201~");
    assert_eq!(
        probe.encode_input(NativeInput::Paste(paste), MAX_INPUT_BYTES),
        Ok(written(&bracketed))
    );
    assert_eq!(
        probe.encode_input(NativeInput::Paste(paste), bracketed.len()),
        Ok(written(&bracketed))
    );
    assert_eq!(
        probe.encode_input(NativeInput::Paste(paste), bracketed.len() - 1),
        Err(InputError::Capacity)
    );
    assert_eq!(
        probe.encode_input(NativeInput::CommittedText("한"), 1),
        Err(InputError::Capacity)
    );
    assert_eq!(
        probe.encode_input(NativeInput::Focus(true), MAX_INPUT_BYTES),
        Ok(written("\x1b[I"))
    );
    assert_eq!(
        probe.encode_input(NativeInput::Focus(false), MAX_INPUT_BYTES),
        Ok(written("\x1b[O"))
    );
    probe.advance(b"\x1b[?2004l\x1b[?1004l");
    assert_eq!(
        probe.encode_input(NativeInput::Paste(paste), MAX_INPUT_BYTES),
        Ok(written(normalized))
    );
    assert_eq!(
        probe.encode_input(NativeInput::Focus(false), MAX_INPUT_BYTES),
        Ok(InputAction::Ignore)
    );
}
