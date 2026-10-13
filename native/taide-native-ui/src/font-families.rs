use egui::{FontFamily, Ui};

pub const MEDIUM_FAMILY: &str = "taide-ui-medium";
pub const SEMIBOLD_FAMILY: &str = "taide-ui-semibold";
pub const BOLD_FAMILY: &str = "taide-ui-bold";
pub const CODE_BOLD_FAMILY: &str = "taide-ui-code-bold";
pub const CODE_SEMIBOLD_FAMILY: &str = "taide-ui-code-semibold";
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

pub fn bold(ui: &Ui) -> FontFamily {
    available(ui, BOLD_FAMILY)
}

pub fn editor_bold(ui: &Ui) -> FontFamily {
    let family = FontFamily::Name(EDITOR_BOLD_FAMILY.into());
    if ui.fonts(|fonts| fonts.definitions().families.contains_key(&family)) {
        return family;
    }
    FontFamily::Monospace
}

pub fn code_bold(ui: &Ui) -> FontFamily {
    let family = FontFamily::Name(CODE_BOLD_FAMILY.into());
    if ui.fonts(|fonts| fonts.definitions().families.contains_key(&family)) {
        return family;
    }
    FontFamily::Monospace
}

pub fn code_semibold(ui: &Ui) -> FontFamily {
    let family = FontFamily::Name(CODE_SEMIBOLD_FAMILY.into());
    if ui.fonts(|fonts| fonts.definitions().families.contains_key(&family)) {
        return family;
    }
    FontFamily::Monospace
}

fn available(ui: &Ui, name: &str) -> FontFamily {
    let family = FontFamily::Name(name.into());
    if ui.fonts(|fonts| fonts.definitions().families.contains_key(&family)) {
        family
    } else {
        FontFamily::Proportional
    }
}
