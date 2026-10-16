use eframe::egui;
use taide_model::locale::ResolvedLocale;

const DIALOG_ID: &str = "native-open-project-by-path";
const INPUT_ID: &str = "native-open-project-by-path-input";

/// `open-project-by-path-dialog.tsx`: the typed path is trimmed and handed to the project open
/// command as is; the input is cleared on every open.
#[derive(Default)]
pub(crate) struct OpenPathDialog {
    is_open: bool,
    path: String,
    is_pending: bool,
    focus: bool,
}

impl OpenPathDialog {
    pub(crate) fn open(&mut self) {
        self.is_open = true;
        self.path.clear();
        self.is_pending = false;
        self.focus = true;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.is_open
    }

    /// Settles a submitted open: success closes the dialog, failure keeps the typed path.
    pub(crate) fn finish(&mut self, is_success: bool) {
        self.is_pending = false;
        if is_success {
            self.is_open = false;
        }
    }

    /// Returns the trimmed path to open when the user confirms.
    pub(crate) fn show(
        &mut self,
        context: &egui::Context,
        locale: &ResolvedLocale,
    ) -> Option<String> {
        if !self.is_open {
            return None;
        }
        let text = |key| crate::presentation::message(locale, key, &[]);
        let mut confirmed = false;
        let mut cancelled = false;
        let trimmed = self.path.trim().to_owned();
        let can_confirm = !trimmed.is_empty() && !self.is_pending;
        let response = egui::Modal::new(egui::Id::new(DIALOG_ID)).show(context, |ui| {
            ui.heading(text("sidebar.openByPathTitle"));
            let input = ui.add(
                egui::TextEdit::singleline(&mut self.path)
                    .id(egui::Id::new(INPUT_ID))
                    .hint_text(text("sidebar.openByPathPlaceholder"))
                    .desired_width(f32::INFINITY),
            );
            if std::mem::take(&mut self.focus) {
                input.request_focus();
            }
            if input.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                confirmed = true;
                // A single-line edit gives up focus on Enter; the original input keeps it.
                self.focus = true;
            }
            ui.horizontal(|ui| {
                if ui.button(text("common.cancel")).clicked() {
                    cancelled = true;
                }
                if ui
                    .add_enabled(can_confirm, egui::Button::new(text("app.openProject")))
                    .clicked()
                {
                    confirmed = true;
                }
            });
        });
        if cancelled || response.should_close() {
            self.is_open = false;
            return None;
        }
        let trimmed = self.path.trim().to_owned();
        if confirmed && !trimmed.is_empty() && !self.is_pending {
            self.is_pending = true;
            return Some(trimmed);
        }
        None
    }
}

#[cfg(test)]
#[path = "open-path-dialog-tests.rs"]
mod tests;
