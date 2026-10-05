use taide_model::settings::{Settings, TerminalCursorStyle};
use taide_native_terminal::{CursorShape, CursorStyle};

pub(crate) fn cursor(settings: &Settings) -> CursorStyle {
    CursorStyle {
        shape: match settings.terminal_cursor_style {
            TerminalCursorStyle::Bar => CursorShape::Beam,
            TerminalCursorStyle::Block => CursorShape::Block,
            TerminalCursorStyle::Underline => CursorShape::Underline,
        },
        blinking: settings.terminal_cursor_blink,
    }
}
