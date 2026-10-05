use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::process::ExitStatus;
use std::sync::Arc;

use crate::term::ClipboardType;
use crate::vte::ansi::Rgb;

/// Terminal event.
///
/// These events instruct the UI over changes that can't be handled by the terminal emulation layer
/// itself.
#[derive(Clone)]
pub enum Event {
    /// Grid has changed possibly requiring a mouse cursor shape change.
    MouseCursorDirty,

    /// Window title change.
    Title(String),

    /// Reset to the default window title.
    ResetTitle,

    /// Request to store a text string in the clipboard.
    ClipboardStore(ClipboardType, String),

    /// Request to write the contents of the clipboard to the PTY.
    ///
    /// The attached function is a formatter which will correctly transform the clipboard content
    /// into the expected escape sequence format.
    ClipboardLoad(ClipboardType, Arc<dyn Fn(&str) -> String + Sync + Send + 'static>),

    /// Request to write the RGB value of a color to the PTY.
    ///
    /// The attached function is a formatter which will correctly transform the RGB color into the
    /// expected escape sequence format.
    ColorRequest(usize, Arc<dyn Fn(Rgb) -> String + Sync + Send + 'static>),

    #[cfg(feature = "native-retained")]
    NativeColorRequest(usize, ColorQuery),

    /// Write some text to the PTY.
    PtyWrite(String),

    /// Request to write the text area size.
    TextAreaSizeRequest(Arc<dyn Fn(WindowSize) -> String + Sync + Send + 'static>),

    #[cfg(feature = "native-retained")]
    NativeTextAreaSizeRequest(TextAreaSizeQuery),

    /// Cursor blinking state has changed.
    CursorBlinkingChange,

    /// New terminal content available.
    Wakeup,

    /// Terminal bell ring.
    Bell,

    /// Shutdown request.
    Exit,

    /// Child process exited.
    ChildExit(ExitStatus),
}

impl Debug for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Event::ClipboardStore(ty, _) => write!(f, "ClipboardStore({ty:?})"),
            Event::ClipboardLoad(ty, _) => write!(f, "ClipboardLoad({ty:?})"),
            Event::TextAreaSizeRequest(_) => write!(f, "TextAreaSizeRequest"),
            Event::ColorRequest(index, _) => write!(f, "ColorRequest({index})"),
            #[cfg(feature = "native-retained")]
            Event::NativeColorRequest(index, _) => write!(f, "NativeColorRequest({index})"),
            #[cfg(feature = "native-retained")]
            Event::NativeTextAreaSizeRequest(_) => write!(f, "NativeTextAreaSizeRequest"),
            Event::PtyWrite(_) => write!(f, "PtyWrite"),
            Event::Title(_) => write!(f, "Title"),
            Event::CursorBlinkingChange => write!(f, "CursorBlinkingChange"),
            Event::MouseCursorDirty => write!(f, "MouseCursorDirty"),
            Event::ResetTitle => write!(f, "ResetTitle"),
            Event::Wakeup => write!(f, "Wakeup"),
            Event::Bell => write!(f, "Bell"),
            Event::Exit => write!(f, "Exit"),
            Event::ChildExit(status) => write!(f, "ChildExit({status:?})"),
        }
    }
}

#[cfg(feature = "native-retained")]
#[derive(Clone, taide_native_retained::RetainedBytes)]
pub struct ColorQuery {
    pub prefix: String,
    pub terminator: String,
    pub override_color: Option<Rgb>,
}

#[cfg(feature = "native-retained")]
impl ColorQuery {
    pub fn reply(&self, color: Rgb) -> String {
        format!(
            "\x1b]{};rgb:{1:02x}{1:02x}/{2:02x}{2:02x}/{3:02x}{3:02x}{4}",
            self.prefix, color.r, color.g, color.b, self.terminator
        )
    }
}

#[cfg(feature = "native-retained")]
#[derive(Clone, taide_native_retained::RetainedBytes)]
pub struct TextAreaSizeQuery;

#[cfg(feature = "native-retained")]
impl TextAreaSizeQuery {
    pub fn reply(&self, window_size: WindowSize) -> String {
        let height = u32::from(window_size.num_lines) * u32::from(window_size.cell_height);
        let width = u32::from(window_size.num_cols) * u32::from(window_size.cell_width);
        format!("\x1b[4;{height};{width}t")
    }
}

/// Byte sequences are sent to a `Notify` in response to some events.
pub trait Notify {
    /// Notify that an escape sequence should be written to the PTY.
    ///
    /// TODO this needs to be able to error somehow.
    fn notify<B: Into<Cow<'static, [u8]>>>(&self, _: B);
}

#[derive(Copy, Clone, Debug)]
pub struct WindowSize {
    pub num_lines: u16,
    pub num_cols: u16,
    pub cell_width: u16,
    pub cell_height: u16,
}

/// Types that are interested in when the display is resized.
pub trait OnResize {
    fn on_resize(&mut self, window_size: WindowSize);
}

/// Event Loop for notifying the renderer about terminal events.
pub trait EventListener {
    fn send_event(&self, _event: Event) {}
}

/// Null sink for events.
#[cfg_attr(feature = "native-retained", derive(taide_native_retained::RetainedBytes))]
pub struct VoidListener;

impl EventListener for VoidListener {}
