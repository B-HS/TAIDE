#[cfg(not(test))]
pub(crate) use taide_native_ui::icons::glyphs::{
    FileColor, Glyph, Icons, SEVERITIES, file, folder, tab,
};

#[cfg(test)]
pub(crate) use compatibility::{FileColor, Glyph, Icons, SEVERITIES, file, folder, tab};
#[cfg(test)]
use eframe::egui;

#[cfg(test)]
mod compatibility {
    include!("../../taide-native-ui/src/glyph-icons.rs");

    mod tests {
        include!("problems-icons-tests.rs");
    }
}
