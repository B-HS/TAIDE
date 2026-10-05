#[cfg(not(test))]
pub use taide_native_ui::keybinding_editor::{Appearance, Editor, Output};

#[cfg(test)]
pub use compatibility::{Appearance, Editor, Output};
#[cfg(test)]
use eframe::egui;
#[cfg(test)]
use std::time::Instant;

#[cfg(test)]
mod compatibility {
    include!("../../taide-native-ui/src/keybinding-editor.rs");
    include!("keybinding-editor-tests.rs");

    mod tooltip_tests {
        include!("keybinding-tooltip-tests.rs");
    }
}
