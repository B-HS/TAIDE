use alacritty_terminal::term::TermMode;

const SHIFT_MODIFIER: u8 = 1;
const ALT_MODIFIER: u8 = 2;
const CONTROL_MODIFIER: u8 = 4;
const COMMAND_MODIFIER: u8 = 8;
const ASCII_CONTROL_OFFSET: u8 = 64;
const FUNCTION_PREFIX: &[char] = &['P', 'Q', 'R', 'S'];
const FUNCTION_TILDE_CODES: &[u8] = &[15, 17, 18, 19, 20, 21, 23, 24];
const FIRST_TILDE_FUNCTION: u8 = 5;
const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";
const MOUSE_SHIFT_MODIFIER: u8 = 4;
const MOUSE_ALT_MODIFIER: u8 = 8;
const MOUSE_CONTROL_MODIFIER: u8 = 16;
const MOUSE_MOTION_CODE: u8 = 32;
const MOUSE_WHEEL_CODE: u8 = 64;
const MOUSE_RELEASE_CODE: u8 = 3;
const MOUSE_DEFAULT_OFFSET: u8 = 32;
const MOUSE_DEFAULT_PREFIX: &[u8] = b"\x1b[M";
const MOUSE_DEFAULT_LENGTH: usize = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
    pub command: bool,
}

impl Modifiers {
    fn mask(self) -> u8 {
        (u8::from(self.shift) * SHIFT_MODIFIER)
            | (u8::from(self.alt) * ALT_MODIFIER)
            | (u8::from(self.control) * CONTROL_MODIFIER)
            | (u8::from(self.command) * COMMAND_MODIFIER)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Key {
    ArrowUp,
    ArrowDown,
    ArrowRight,
    ArrowLeft,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
    Tab,
    Backspace,
    Enter,
    Escape,
    Function(u8),
    Character(char),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
}

impl MouseButton {
    fn code(self) -> u8 {
        match self {
            Self::Left => 0,
            Self::Middle => 1,
            Self::Right => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAction {
    Press(MouseButton),
    Release(MouseButton),
    Move(Option<MouseButton>),
    WheelUp,
    WheelDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseInput {
    pub column: u16,
    pub row: u16,
    pub pixels: Option<(u32, u32)>,
    pub action: MouseAction,
    pub modifiers: Modifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseProtocol {
    None,
    X10,
    Vt200,
    Drag,
    Any,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEncoding {
    Default,
    Sgr,
    SgrPixels,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseMode {
    pub protocol: MouseProtocol,
    pub encoding: MouseEncoding,
}

impl MouseMode {
    pub fn from_mode(mode: TermMode) -> Self {
        let protocol = if mode.contains(TermMode::MOUSE_MOTION) {
            MouseProtocol::Any
        } else if mode.contains(TermMode::MOUSE_DRAG) {
            MouseProtocol::Drag
        } else if mode.contains(TermMode::MOUSE_REPORT_CLICK) {
            MouseProtocol::Vt200
        } else {
            MouseProtocol::None
        };
        let encoding = if mode.contains(TermMode::SGR_MOUSE) {
            MouseEncoding::Sgr
        } else {
            MouseEncoding::Default
        };
        Self { protocol, encoding }
    }
}

pub enum NativeInput<'a> {
    Key { key: Key, modifiers: Modifiers },
    Mouse(MouseInput),
    Preedit(&'a str),
    CommittedText(&'a str),
    Paste(&'a str),
    Focus(bool),
}

#[derive(Debug, PartialEq, Eq)]
pub enum InputAction {
    Write(Vec<u8>),
    PageUp,
    PageDown,
    SelectAll,
    ClipboardShortcut,
    Ignore,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InputError {
    Retired,
    Capacity,
    UnsupportedKey,
    UnsupportedMode,
    InvalidCoordinates,
}

fn write(text: &str, capacity: usize) -> Result<InputAction, InputError> {
    if text.len() > capacity {
        return Err(InputError::Capacity);
    }
    Ok(InputAction::Write(text.as_bytes().to_vec()))
}

fn cursor(suffix: char, mask: u8, application: bool) -> String {
    if mask != 0 {
        return format!("\x1b[1;{}{suffix}", mask + 1);
    }
    if application {
        return format!("\x1bO{suffix}");
    }
    format!("\x1b[{suffix}")
}

fn tilde(code: u8, mask: u8) -> String {
    if mask == 0 {
        return format!("\x1b[{code}~");
    }
    format!("\x1b[{code};{}~", mask + 1)
}

fn character(character: char, modifiers: Modifiers) -> Option<String> {
    if modifiers.command {
        return None;
    }
    if modifiers.alt {
        let character = if modifiers.control && character.is_ascii_alphabetic() {
            let byte = u8::try_from(u32::from(character.to_ascii_uppercase())).ok()?;
            char::from(byte - ASCII_CONTROL_OFFSET)
        } else if modifiers.control && character == ' ' {
            '\0'
        } else {
            character
        };
        return Some(format!("\x1b{character}"));
    }
    if !modifiers.control {
        return Some(character.to_string());
    }
    if character == '_' {
        return Some("\x1f".into());
    }
    if character == '@' {
        return Some("\0".into());
    }
    if modifiers.shift {
        return None;
    }
    let character = match character {
        'a'..='z' | 'A'..='Z' => {
            let byte = u8::try_from(u32::from(character.to_ascii_uppercase())).ok()?;
            char::from(byte - ASCII_CONTROL_OFFSET)
        }
        ' ' => '\0',
        '3' | '[' => '\x1b',
        '4' | '\\' => '\x1c',
        '5' | ']' => '\x1d',
        '6' => '\x1e',
        '7' => '\x1f',
        '8' => '\x7f',
        _ => return None,
    };
    Some(character.to_string())
}

pub fn encode_mouse(
    mode: MouseMode,
    input: MouseInput,
    capacity: usize,
) -> Result<InputAction, InputError> {
    if mode.protocol == MouseProtocol::None {
        return Ok(InputAction::Ignore);
    }
    if mode.protocol == MouseProtocol::X10 && !matches!(input.action, MouseAction::Press(_)) {
        return Ok(InputAction::Ignore);
    }
    if let MouseAction::Move(button) = input.action
        && mode.protocol != MouseProtocol::Any
        && (mode.protocol != MouseProtocol::Drag || button.is_none())
    {
        return Ok(InputAction::Ignore);
    }
    let is_sgr = mode.encoding != MouseEncoding::Default;
    let modifiers = if mode.protocol == MouseProtocol::X10 {
        Modifiers::default()
    } else {
        input.modifiers
    };
    let code = match input.action {
        MouseAction::Press(button) => button.code(),
        MouseAction::Release(button) if is_sgr => button.code(),
        MouseAction::Release(_) => MOUSE_RELEASE_CODE,
        MouseAction::Move(button) => {
            button.map_or(MOUSE_RELEASE_CODE, MouseButton::code) | MOUSE_MOTION_CODE
        }
        MouseAction::WheelUp => MOUSE_WHEEL_CODE,
        MouseAction::WheelDown => MOUSE_WHEEL_CODE | 1,
    } | (u8::from(modifiers.shift) * MOUSE_SHIFT_MODIFIER)
        | (u8::from(modifiers.alt) * MOUSE_ALT_MODIFIER)
        | (u8::from(modifiers.control) * MOUSE_CONTROL_MODIFIER);
    let (column, row) = if mode.encoding == MouseEncoding::SgrPixels {
        input.pixels.ok_or(InputError::InvalidCoordinates)?
    } else {
        (u32::from(input.column) + 1, u32::from(input.row) + 1)
    };
    if is_sgr {
        let suffix = if matches!(input.action, MouseAction::Release(_)) {
            'm'
        } else {
            'M'
        };
        return write(&format!("\x1b[<{code};{column};{row}{suffix}"), capacity);
    }
    let (Ok(column), Ok(row)) = (
        u8::try_from(column + u32::from(MOUSE_DEFAULT_OFFSET)),
        u8::try_from(row + u32::from(MOUSE_DEFAULT_OFFSET)),
    ) else {
        return Ok(InputAction::Ignore);
    };
    if capacity < MOUSE_DEFAULT_LENGTH {
        return Err(InputError::Capacity);
    }
    let mut bytes = Vec::with_capacity(MOUSE_DEFAULT_LENGTH);
    bytes.extend_from_slice(MOUSE_DEFAULT_PREFIX);
    bytes.extend_from_slice(&[code + MOUSE_DEFAULT_OFFSET, column, row]);
    Ok(InputAction::Write(bytes))
}

pub fn encode_input(
    mode: TermMode,
    input: NativeInput<'_>,
    capacity: usize,
) -> Result<InputAction, InputError> {
    match input {
        NativeInput::Mouse(input) => {
            if mode.contains(TermMode::UTF8_MOUSE) {
                return Err(InputError::UnsupportedMode);
            }
            encode_mouse(MouseMode::from_mode(mode), input, capacity)
        }
        NativeInput::Preedit(_) => Ok(InputAction::Ignore),
        NativeInput::CommittedText(text) => write(text, capacity),
        NativeInput::Paste(text) => {
            let normalized_len = text.len()
                - text
                    .as_bytes()
                    .windows(2)
                    .filter(|pair| *pair == b"\r\n")
                    .count();
            let is_bracketed = mode.contains(TermMode::BRACKETED_PASTE);
            let decoration = if is_bracketed {
                PASTE_START.len() + PASTE_END.len()
            } else {
                0
            };
            let length = normalized_len
                .checked_add(decoration)
                .ok_or(InputError::Capacity)?;
            if length > capacity {
                return Err(InputError::Capacity);
            }
            let mut bytes = Vec::with_capacity(length);
            if is_bracketed {
                bytes.extend_from_slice(PASTE_START);
            }
            let mut previous = None;
            for byte in text.bytes() {
                if byte != b'\n' || previous != Some(b'\r') {
                    bytes.push(if byte == b'\n' { b'\r' } else { byte });
                }
                previous = Some(byte);
            }
            if is_bracketed {
                bytes.extend_from_slice(PASTE_END);
            }
            Ok(InputAction::Write(bytes))
        }
        NativeInput::Focus(is_focused) => {
            if !mode.contains(TermMode::FOCUS_IN_OUT) {
                return Ok(InputAction::Ignore);
            }
            write(if is_focused { "\x1b[I" } else { "\x1b[O" }, capacity)
        }
        NativeInput::Key { key, modifiers } => {
            if mode.intersects(TermMode::KITTY_KEYBOARD_PROTOCOL) {
                return Err(InputError::UnsupportedMode);
            }
            let mask = modifiers.mask();
            let application = mode.contains(TermMode::APP_CURSOR);
            let text = match key {
                Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight
                    if modifiers.command =>
                {
                    return Ok(InputAction::Ignore);
                }
                Key::ArrowUp => cursor('A', mask, application),
                Key::ArrowDown => cursor('B', mask, application),
                Key::ArrowRight => cursor('C', mask, application),
                Key::ArrowLeft => cursor('D', mask, application),
                Key::Home => cursor('H', mask, application),
                Key::End => cursor('F', mask, application),
                Key::Function(number) => {
                    if let Some(suffix) = number
                        .checked_sub(1)
                        .and_then(|index| FUNCTION_PREFIX.get(usize::from(index)))
                    {
                        cursor(*suffix, mask, true)
                    } else {
                        let code = number
                            .checked_sub(FIRST_TILDE_FUNCTION)
                            .and_then(|index| FUNCTION_TILDE_CODES.get(usize::from(index)))
                            .ok_or(InputError::UnsupportedKey)?;
                        tilde(*code, mask)
                    }
                }
                Key::Insert if modifiers.shift || modifiers.control => {
                    return Ok(InputAction::ClipboardShortcut);
                }
                Key::Insert => tilde(2, 0),
                Key::Delete => tilde(3, mask),
                Key::PageUp if modifiers.shift => return Ok(InputAction::PageUp),
                Key::PageDown if modifiers.shift => return Ok(InputAction::PageDown),
                Key::PageUp => tilde(5, if modifiers.control { mask } else { 0 }),
                Key::PageDown => tilde(6, if modifiers.control { mask } else { 0 }),
                Key::Tab if modifiers.shift => "\x1b[Z".into(),
                Key::Tab => "\t".into(),
                Key::Backspace => {
                    let byte = if modifiers.control { '\x08' } else { '\x7f' };
                    if modifiers.alt {
                        format!("\x1b{byte}")
                    } else {
                        byte.into()
                    }
                }
                Key::Enter if mask == SHIFT_MODIFIER => "\n".into(),
                Key::Enter if modifiers.alt => "\x1b\r".into(),
                Key::Enter => "\r".into(),
                Key::Escape if modifiers.alt => "\x1b\x1b".into(),
                Key::Escape => "\x1b".into(),
                Key::Character(character)
                    if mask == COMMAND_MODIFIER && character.eq_ignore_ascii_case(&'a') =>
                {
                    return Ok(InputAction::SelectAll);
                }
                Key::Character(value) => {
                    let Some(text) = character(value, modifiers) else {
                        return Ok(InputAction::Ignore);
                    };
                    text
                }
            };
            write(&text, capacity)
        }
    }
}
