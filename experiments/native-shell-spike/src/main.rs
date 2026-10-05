use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Instant;

use eframe::egui;
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use taide_native_shell_spike::{Locale, Pane, ShellFixture, TREE_ROW_COUNT};

#[cfg(target_os = "macos")]
#[path = "native-context-menu.rs"]
mod native_context_menu;

const MAIN_SIZE: [f32; 2] = [1100.0, 760.0];
const AUXILIARY_SIZE: [f32; 2] = [760.0, 560.0];
const TREE_WIDTH: f32 = 220.0;
const TREE_ROW_HEIGHT: f32 = 22.0;
const EDITOR_ROW_COUNT: usize = 12;
const MILLISECONDS_PER_SECOND: f64 = 1_000.0;
const FONT_PATHS: &[&str] = &[
    "/System/Library/Fonts/AppleSDGothicNeo.ttc",
    "/System/Library/Fonts/Hiragino Sans GB.ttc",
];

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(MAIN_SIZE),
        ..Default::default()
    };
    eframe::run_native(
        Pane::Main.window_title(),
        options,
        Box::new(|context| Ok(Box::new(ShellSpike::new(context)?))),
    )
}

struct ShellSpike {
    fixture: ShellFixture,
    is_auxiliary_open: bool,
    menu: Menu,
    menu_events: mpsc::Receiver<MenuEvent>,
    dialog_item: MenuItem,
    auxiliary_item: MenuItem,
    probes: HashMap<Pane, WindowProbe>,
    dialog_count: usize,
    status: String,
    focus_tab: Option<(usize, Pane)>,
}

#[derive(Default)]
struct WindowProbe {
    observed_frame: Option<u64>,
    ime_preedit_count: usize,
    ime_commit_count: usize,
    drop_count: usize,
    native_context_count: usize,
    native_context_selected_count: usize,
    last_context_frame: Option<u64>,
    visible_rows: usize,
    ui_time_ms: f64,
}

impl ShellSpike {
    fn new(context: &eframe::CreationContext<'_>) -> Result<Self, muda::Error> {
        let mut fonts = egui::FontDefinitions::default();
        for path in FONT_PATHS {
            if let Ok(bytes) = std::fs::read(path) {
                fonts.font_data.insert(
                    path.to_string(),
                    std::sync::Arc::new(egui::FontData::from_owned(bytes)),
                );
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts
                        .families
                        .entry(family)
                        .or_default()
                        .push(path.to_string());
                }
            }
        }
        context.egui_ctx.set_fonts(fonts);
        let menu = Menu::new();
        let application = Submenu::new("TAIDE M8 spike", true);
        application.append(&PredefinedMenuItem::quit(None))?;
        let file = Submenu::new("File", true);
        let dialog_item = MenuItem::with_id("choose-file", "Choose file", true, None);
        let auxiliary_item = MenuItem::with_id("auxiliary", "Auxiliary window", true, None);
        file.append_items(&[&dialog_item, &auxiliary_item])?;
        let windows = Submenu::new("Window", true);
        windows.append_items(&[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::close_window(None),
        ])?;
        menu.append_items(&[&application, &file, &windows])?;
        #[cfg(target_os = "macos")]
        {
            menu.init_for_nsapp();
            windows.set_as_windows_menu_for_nsapp();
        }
        let (sender, receiver) = mpsc::channel();
        let repaint_context = context.egui_ctx.clone();
        MenuEvent::set_event_handler(Some(move |event| {
            if sender.send(event).is_ok() {
                repaint_context.request_repaint();
            }
        }));
        Ok(Self {
            fixture: ShellFixture::default(),
            is_auxiliary_open: true,
            menu,
            menu_events: receiver,
            dialog_item,
            auxiliary_item,
            probes: HashMap::new(),
            dialog_count: 0,
            status: "Isolated synthetic fixture; no project data writes".to_string(),
            focus_tab: None,
        })
    }

    fn choose_file(&mut self) {
        if rfd::FileDialog::new()
            .set_title("TAIDE M8 isolated probe")
            .pick_file()
            .is_some()
        {
            self.dialog_count += 1;
            self.status = "Native file dialog selection received; file was not read".to_string();
        }
    }

    fn close_auxiliary(&mut self) -> bool {
        let has_auxiliary_tabs = self.fixture.tabs.contains(&Pane::Auxiliary);
        let has_auxiliary_focus = self
            .focus_tab
            .is_some_and(|(_, pane)| pane == Pane::Auxiliary);
        let was_open = self.is_auxiliary_open;
        self.is_auxiliary_open = false;
        self.fixture.return_auxiliary_tabs();
        if let Some((tab, Pane::Auxiliary)) = self.focus_tab {
            self.focus_tab = Some((tab, Pane::Main));
        }
        was_open || has_auxiliary_tabs || has_auxiliary_focus
    }

    fn show_shell(&mut self, root_ui: &mut egui::Ui, pane: Pane) {
        let started = Instant::now();
        let context = root_ui.ctx().clone();
        let mut probe = self.probes.remove(&pane).unwrap_or_default();
        let frame = context.cumulative_frame_nr();
        if probe.observed_frame != Some(frame) {
            probe.observed_frame = Some(frame);
            context.input(|input| {
                for event in &input.events {
                    match event {
                        egui::Event::Ime(egui::ImeEvent::Preedit { .. }) => {
                            probe.ime_preedit_count += 1
                        }
                        egui::Event::Ime(egui::ImeEvent::Commit(_)) => probe.ime_commit_count += 1,
                        _ => {}
                    }
                }
                if !input.raw.dropped_files.is_empty() {
                    probe.drop_count += input.raw.dropped_files.len();
                    self.status = "External file drop received; files were not read".to_string();
                }
            });
        }
        egui::Panel::top("toolbar").show(root_ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("TAIDE M8 / egui");
                ui.checkbox(
                    &mut self.is_auxiliary_open,
                    self.fixture.locale.label("auxiliary"),
                );
                if ui.button(self.fixture.locale.label("dialog")).clicked() {
                    self.choose_file();
                }
                ui.checkbox(
                    &mut self.fixture.is_dark,
                    self.fixture.locale.label("theme"),
                );
                egui::ComboBox::from_id_salt("locale")
                    .selected_text(format!("{:?}", self.fixture.locale))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.fixture.locale, Locale::English, "English");
                        ui.selectable_value(&mut self.fixture.locale, Locale::Korean, "한국어");
                        ui.selectable_value(&mut self.fixture.locale, Locale::Japanese, "日本語");
                    });
            });
        });
        if self.fixture.is_dark {
            context.set_visuals(egui::Visuals::dark());
        } else {
            context.set_visuals(egui::Visuals::light());
        }
        egui::Panel::bottom("status").show(root_ui, |ui| {
            ui.label(&self.status);
            ui.label(format!(
                "IME preedit {} / commit {} | drops {} | dialogs {} | native menus {} / selected {} | visible rows {} / {} | UI {:.2}ms (not input latency)",
                probe.ime_preedit_count, probe.ime_commit_count, probe.drop_count,
                self.dialog_count, probe.native_context_count, probe.native_context_selected_count,
                probe.visible_rows, TREE_ROW_COUNT, probe.ui_time_ms,
            ));
        });
        egui::Panel::left("tree")
            .default_size(TREE_WIDTH)
            .show(root_ui, |ui| {
                ui.heading(self.fixture.locale.label("tree"));
                egui::ScrollArea::vertical().show_rows(
                    ui,
                    TREE_ROW_HEIGHT,
                    TREE_ROW_COUNT,
                    |ui, rows| {
                        probe.visible_rows = rows.len();
                        for row in rows {
                            ui.push_id(row, |ui| {
                                if ui
                                    .selectable_label(
                                        self.fixture.selected_row == Some(row),
                                        format!("fixture-{row:05}.rs"),
                                    )
                                    .clicked()
                                {
                                    self.fixture.selected_row = Some(row);
                                }
                            });
                        }
                    },
                );
            });
        egui::CentralPanel::default().show(root_ui, |ui| {
            let response = ui
                .scope(|ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("tabs")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for tab in 0..self.fixture.tabs.len() {
                                    if self.fixture.tabs[tab] != pane {
                                        continue;
                                    }
                                    let selected = match pane {
                                        Pane::Main => self.fixture.active_main,
                                        Pane::Auxiliary => self.fixture.active_auxiliary,
                                    };
                                    let button = egui::Button::new(format!("Tab {tab:02}"))
                                        .selected(selected == Some(tab))
                                        .sense(egui::Sense::click_and_drag());
                                    let response =
                                        ui.push_id(("tab", pane, tab), |ui| ui.add(button)).inner;
                                    if self.focus_tab == Some((tab, pane)) {
                                        response.request_focus();
                                        response.scroll_to_me(Some(egui::Align::Center));
                                        context.send_viewport_cmd(egui::ViewportCommand::Focus);
                                        self.focus_tab = None;
                                    }
                                    response.dnd_set_drag_payload(tab);
                                    if response.clicked() {
                                        match pane {
                                            Pane::Main => self.fixture.active_main = Some(tab),
                                            Pane::Auxiliary => {
                                                self.fixture.active_auxiliary = Some(tab)
                                            }
                                        }
                                    }
                                    #[cfg(target_os = "macos")]
                                    let is_keyboard_request =
                                        native_context_menu::keyboard_request_for_response(
                                            ui, &response,
                                        );
                                    #[cfg(target_os = "macos")]
                                    if (response.secondary_clicked() || is_keyboard_request)
                                        && probe.last_context_frame != Some(frame)
                                    {
                                        probe.last_context_frame = Some(frame);
                                        let position = if is_keyboard_request {
                                            native_context_menu::keyboard_position(
                                                response.rect,
                                                ui.clip_rect().intersect(context.viewport_rect()),
                                                context.zoom_factor(),
                                            )
                                            .map(Some)
                                        } else {
                                            Ok(None)
                                        };
                                        match position.and_then(|position| {
                                            native_context_menu::show_tab_menu(
                                                pane,
                                                self.fixture.locale.label("move-tab"),
                                                position,
                                            )
                                        }) {
                                            Ok(is_selected) => {
                                                probe.native_context_count += 1;
                                                let focus = self.fixture.finish_context_menu(
                                                    tab,
                                                    pane,
                                                    is_selected,
                                                );
                                                if focus.is_some_and(|(_, destination)| {
                                                    destination != pane
                                                }) {
                                                    probe.native_context_selected_count += 1;
                                                    self.is_auxiliary_open = true;
                                                    self.focus_tab = focus;
                                                    context.request_repaint();
                                                    self.status =
                                                        "Native context menu moved a synthetic tab"
                                                            .into();
                                                } else if focus.is_some() {
                                                    response.request_focus();
                                                }
                                            }
                                            Err(error) => self.status = error.into(),
                                        }
                                    }
                                    #[cfg(not(target_os = "macos"))]
                                    response.context_menu(|ui| {
                                        if ui
                                            .button(self.fixture.locale.label("move-tab"))
                                            .clicked()
                                        {
                                            if self.fixture.move_tab_from(tab, pane) {
                                                self.is_auxiliary_open = true;
                                            }
                                            ui.close();
                                        }
                                    });
                                }
                            });
                        });
                    ui.separator();
                    ui.label("Drop a tab here to move it between native windows");
                    let label = ui.label(self.fixture.locale.label("input"));
                    ui.add(
                        egui::TextEdit::multiline(&mut self.fixture.input)
                            .id_salt(("input", pane))
                            .desired_rows(EDITOR_ROW_COUNT)
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(label.id);
                    if ui.button("Copy fixture text").clicked() {
                        context.copy_text(self.fixture.input.clone());
                    }
                    ui.label("CJK: 한글 日本語 中文 | emoji and bidi are separate editor gates");
                    ui.allocate_space(ui.available_size());
                })
                .response;
            if let Some(tab) = response.dnd_release_payload::<usize>() {
                self.fixture.move_tab(*tab, pane);
            }
        });
        probe.ui_time_ms = started.elapsed().as_secs_f64() * MILLISECONDS_PER_SECOND;
        self.probes.insert(pane, probe);
    }
}

impl eframe::App for ShellSpike {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(event) = self.menu_events.try_recv() {
            if event.id == self.dialog_item.id() {
                self.choose_file();
            }
            if event.id == self.auxiliary_item.id() {
                self.is_auxiliary_open = true;
            }
        }
        self.show_shell(ui, Pane::Main);
        if !self.is_auxiliary_open {
            if self.close_auxiliary() {
                ui.ctx().request_repaint();
            }
            return;
        }
        ui.ctx().show_viewport_immediate(
            egui::ViewportId::from_hash_of("auxiliary"),
            egui::ViewportBuilder::default()
                .with_title(Pane::Auxiliary.window_title())
                .with_inner_size(AUXILIARY_SIZE),
            |ui, class| {
                if class == egui::ViewportClass::EmbeddedWindow {
                    self.status =
                        "FAIL: auxiliary viewport is embedded, not a native window".to_string();
                    return;
                }
                self.show_shell(ui, Pane::Auxiliary);
                if ui.input(|input| input.viewport().close_requested()) && self.close_auxiliary() {
                    ui.ctx().request_repaint();
                }
            },
        );
    }
}

impl Drop for ShellSpike {
    fn drop(&mut self) {
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
        #[cfg(target_os = "macos")]
        self.menu.remove_for_nsapp();
    }
}
