use egui::{FontFamily, Ui};

pub const MEDIUM_FAMILY: &str = "taide-ui-medium";
pub const SEMIBOLD_FAMILY: &str = "taide-ui-semibold";
pub const EDITOR_FAMILY: &str = "taide-editor";
pub const EDITOR_BOLD_FAMILY: &str = "taide-editor-bold";
pub const CODICON_FAMILY: &str = "taide-monaco-codicons";

pub fn codicons(ui: &Ui) -> FontFamily {
    available(ui, CODICON_FAMILY)
}

pub fn medium(ui: &Ui) -> FontFamily {
    available(ui, MEDIUM_FAMILY)
}

pub fn semibold(ui: &Ui) -> FontFamily {
    available(ui, SEMIBOLD_FAMILY)
}

fn available(ui: &Ui, name: &str) -> FontFamily {
    let family = FontFamily::Name(name.into());
    if ui.fonts(|fonts| fonts.definitions().families.contains_key(&family)) {
        family
    } else {
        FontFamily::Proportional
    }
}
