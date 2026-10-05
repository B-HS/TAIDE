#[cfg(not(test))]
pub use taide_native_ui::keymap::*;

#[cfg(test)]
pub use compatibility::*;
#[cfg(test)]
use eframe::egui;
#[cfg(test)]
use std::time::Instant;
#[cfg(test)]
use taide_native_ui::status_chord;

#[cfg(test)]
mod compatibility {
    include!("../../taide-native-ui/src/keymap.rs");
    include!("keymap-tests.rs");

    mod chord_status_tests {
        include!("keymap-chord-status-tests.rs");
    }
}
