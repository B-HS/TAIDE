use std::sync::mpsc;

use eframe::egui;
use muda::accelerator::{Accelerator, CMD_OR_CTRL, Code};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use taide_model::error::{AppError, AppResult};

use crate::app_menu::{Bar, FILE_ID, Model, OPEN_RECENT_ID, Row, Shortcut, Standard, literal_text};

pub(crate) struct Native {
    _menu: Menu,
    file: Submenu,
    open_recent: Submenu,
}

pub(crate) fn install(
    context: egui::Context,
    model: &Model,
) -> AppResult<(Native, mpsc::Receiver<String>)> {
    let (sender, receiver) = mpsc::channel();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        if sender.send(event.id.0).is_ok() {
            context.request_repaint();
        }
    }));
    let menu = Menu::new();
    let mut submenus = Vec::new();
    for spec in model.bar() {
        let submenu = match spec.id {
            Some(id) => Submenu::with_id(id, literal_text(&spec.title), true),
            None => Submenu::new(literal_text(&spec.title), true),
        };
        append_rows(&submenu, &spec.rows, &mut submenus).map_err(menu_error)?;
        menu.append(&submenu).map_err(menu_error)?;
        submenus.push(submenu);
    }
    let find = |id: &str| {
        submenus
            .iter()
            .find(|submenu| submenu.id() == id)
            .cloned()
            .ok_or_else(|| AppError::Internal(format!("native app menu has no {id} submenu")))
    };
    let file = find(FILE_ID)?;
    let open_recent = find(OPEN_RECENT_ID)?;
    menu.init_for_nsapp();
    Ok((
        Native {
            _menu: menu,
            file,
            open_recent,
        },
        receiver,
    ))
}

impl Bar for Native {
    fn apply(&mut self, model: &Model) {
        self.file.set_text(literal_text(&model.file));
        self.open_recent.set_text(literal_text(&model.open_recent));
        while self.open_recent.remove_at(0).is_some() {}
        if let Err(error) = append_rows(&self.open_recent, &model.recent, &mut Vec::new()) {
            log::warn!("native recent menu refresh failed: {error}");
        }
    }
}

fn append_rows(menu: &Submenu, rows: &[Row], submenus: &mut Vec<Submenu>) -> muda::Result<()> {
    for row in rows {
        match row {
            Row::Item {
                id,
                text,
                enabled,
                shortcut,
            } => menu.append(&MenuItem::with_id(
                id.as_str(),
                literal_text(text),
                *enabled,
                shortcut.map(accelerator),
            ))?,
            Row::Separator => menu.append(&PredefinedMenuItem::separator())?,
            Row::Standard(standard) => menu.append(&standard_item(*standard))?,
            Row::Submenu { id, title, rows } => {
                let nested = Submenu::with_id(*id, literal_text(title), true);
                append_rows(&nested, rows, submenus)?;
                menu.append(&nested)?;
                submenus.push(nested);
            }
        }
    }
    Ok(())
}

fn standard_item(standard: Standard) -> PredefinedMenuItem {
    match standard {
        Standard::About => PredefinedMenuItem::about(None, None),
        Standard::Services => PredefinedMenuItem::services(None),
        Standard::Hide => PredefinedMenuItem::hide(None),
        Standard::HideOthers => PredefinedMenuItem::hide_others(None),
        Standard::ShowAll => PredefinedMenuItem::show_all(None),
        Standard::Minimize => PredefinedMenuItem::minimize(None),
        Standard::Maximize => PredefinedMenuItem::maximize(None),
        Standard::Fullscreen => PredefinedMenuItem::fullscreen(None),
    }
}

fn accelerator(shortcut: Shortcut) -> Accelerator {
    match shortcut {
        Shortcut::CommandQ => Accelerator::new(CMD_OR_CTRL, Code::KeyQ),
    }
}

fn menu_error(error: muda::Error) -> AppError {
    AppError::Internal(format!("native app menu build failed: {error}"))
}
