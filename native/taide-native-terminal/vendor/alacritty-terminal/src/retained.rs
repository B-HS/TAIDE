use taide_native_retained::{Error, RetainedBytes, Visitor};

use crate::event::Event;
use crate::term::{TermMode, cell::Flags};

macro_rules! scalar_flags {
    ($kind:ty, $bits:ty) => {
        impl RetainedBytes for $kind {
            fn has_owned_children() -> bool {
                false
            }

            fn visit<'value>(&'value self, _: &mut Visitor<'value, '_>) -> Result<(), Error> {
                let _: $bits = self.bits();
                Ok(())
            }
        }
    };
}

scalar_flags!(TermMode, u32);
scalar_flags!(Flags, u16);

impl RetainedBytes for Event {
    fn visit<'value>(&'value self, visitor: &mut Visitor<'value, '_>) -> Result<(), Error> {
        match self {
            Self::Title(text) | Self::PtyWrite(text) => visitor.push(text),
            Self::ClipboardStore(kind, text) => {
                visitor.push(kind)?;
                visitor.push(text)
            }
            Self::NativeColorRequest(index, query) => {
                visitor.push(index)?;
                visitor.push(query)
            }
            Self::NativeTextAreaSizeRequest(query) => visitor.push(query),
            Self::ClipboardLoad(..)
            | Self::ColorRequest(..)
            | Self::TextAreaSizeRequest(..)
            | Self::ChildExit(_) => Err(Error::Opaque),
            Self::MouseCursorDirty
            | Self::ResetTitle
            | Self::CursorBlinkingChange
            | Self::Wakeup
            | Self::Bell
            | Self::Exit => Ok(()),
        }
    }
}
