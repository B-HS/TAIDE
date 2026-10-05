#[cfg(not(test))]
pub use taide_native_ui::keybinding_search::*;

#[cfg(test)]
include!("../../taide-native-ui/src/keybinding-search.rs");
