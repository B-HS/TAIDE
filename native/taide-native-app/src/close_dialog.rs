use eframe::egui;
use taide_model::layout::Tab;
use taide_model::locale::ResolvedLocale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseChoice {
    Save,
    Discard,
    Cancel,
}

pub fn show(
    context: &egui::Context,
    locale: &ResolvedLocale,
    tab: &Tab,
    busy: bool,
) -> Option<CloseChoice> {
    show_titles(context, locale, std::slice::from_ref(&tab.title), busy)
}

pub fn show_titles(
    context: &egui::Context,
    locale: &ResolvedLocale,
    titles: &[String],
    busy: bool,
) -> Option<CloseChoice> {
    let response = egui::Modal::new(egui::Id::new("native-close-dirty-tab")).show(context, |ui| {
        ui.heading(crate::presentation::message(
            locale,
            "tab.confirmCloseDirtyTitle",
            &[],
        ));
        let description = if titles.len() == 1 {
            crate::presentation::message(
                locale,
                "tab.confirmCloseDirtyDescription",
                &[("title", &titles[0])],
            )
        } else {
            crate::presentation::message(
                locale,
                "tab.confirmCloseDirtyDescriptionMany",
                &[
                    ("count", &titles.len().to_string()),
                    ("titles", &titles.join(", ")),
                ],
            )
        };
        ui.label(description);
        if busy {
            ui.spinner();
            return None;
        }
        ui.horizontal(|ui| {
            if ui
                .button(crate::presentation::message(locale, "common.cancel", &[]))
                .clicked()
            {
                return Some(CloseChoice::Cancel);
            }
            if ui
                .button(crate::presentation::message(
                    locale,
                    "tab.confirmCloseDirtyDiscard",
                    &[],
                ))
                .clicked()
            {
                return Some(CloseChoice::Discard);
            }
            if ui
                .button(crate::presentation::message(
                    locale,
                    "tab.confirmCloseDirtySave",
                    &[],
                ))
                .clicked()
            {
                return Some(CloseChoice::Save);
            }
            None
        })
        .inner
    });
    if !busy && response.should_close() {
        return Some(CloseChoice::Cancel);
    }
    response.inner
}
