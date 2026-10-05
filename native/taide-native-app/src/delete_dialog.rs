use eframe::egui::{self, Response};
use taide_model::locale::ResolvedLocale;

use crate::explorer_delete::Request;

const MAX_WIDTH: f32 = 512.0;

pub struct Confirmation {
    pub request: Request,
    focus_cancel: bool,
}

pub struct Output {
    pub choice: Option<bool>,
    pub cancel: Response,
    pub confirm: Response,
}

impl Confirmation {
    pub fn new(request: Request) -> Self {
        Self {
            request,
            focus_cancel: true,
        }
    }

    pub fn show(&mut self, context: &egui::Context, locale: &ResolvedLocale) -> Output {
        let response = egui::Modal::new(egui::Id::new("native-explorer-delete-confirmation")).show(
            context,
            |ui| {
                ui.set_max_width(MAX_WIDTH);
                ui.heading(crate::presentation::message(
                    locale,
                    "explorer.deleteConfirmTitle",
                    &[("name", &self.request.name)],
                ));
                ui.label(crate::presentation::message(
                    locale,
                    "explorer.deleteConfirmDescription",
                    &[("name", &self.request.name)],
                ));
                ui.horizontal(|ui| {
                    let cancel =
                        ui.button(crate::presentation::message(locale, "common.cancel", &[]));
                    if self.focus_cancel {
                        self.focus_cancel = false;
                        cancel.request_focus();
                    }
                    let confirm = ui.button(
                        egui::RichText::new(crate::presentation::message(
                            locale,
                            "explorer.delete",
                            &[],
                        ))
                        .color(ui.visuals().error_fg_color),
                    );
                    let choice = if cancel.clicked() {
                        Some(false)
                    } else if confirm.clicked() {
                        Some(true)
                    } else {
                        None
                    };
                    Output {
                        choice,
                        cancel,
                        confirm,
                    }
                })
                .inner
            },
        );
        let mut output = response.inner;
        if response.is_top_modal
            && !response.any_popup_open
            && context
                .input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            output.choice = Some(false);
        }
        output
    }
}
