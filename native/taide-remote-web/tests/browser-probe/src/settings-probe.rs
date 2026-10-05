use std::cell::{Cell, RefCell};
use std::rc::Rc;

use egui::{Context, RawInput, Ui};
use serde_json::json;
use taide_model::ids::{PaneId, ProjectId, TabId};
use taide_native_editor::store::EditorLimits;
use taide_native_ui::settings_owner::Owner;
use taide_remote_web::canvas::{BrowserCanvas, CanvasAction, CanvasContents, WebOptions};
use taide_remote_web::close::CloseState;
use taide_remote_web::{BrowserEditor, BrowserEvent, Delivery};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

const LIMIT: usize = 1;
const BYTE_LIMIT: usize = 128;
const APP_FILE_BYTE_LIMIT: usize = 65_536;
const BACKGROUND: &str = "#102030";
const FOREGROUND: &str = "#F4F4F4";
const BORDER: &str = "#8896A4";
const ACCENT: &str = "#5AA8FF";
const HOVER: &str = "#204060";

#[derive(Default)]
struct Observations {
    frames: u32,
    pumps: u32,
    mount: Option<u64>,
    generation: Option<u64>,
    themes: Vec<String>,
    locales: Vec<String>,
    catalog_errors: Vec<String>,
    failures: Vec<String>,
    theme_errors: Vec<String>,
    editing: Option<String>,
    pending_theme: bool,
    targets: std::collections::BTreeMap<String, [f32; 2]>,
    hit_targets: Vec<String>,
    tooltip_targets: std::collections::BTreeMap<String, [f32; 2]>,
    open_tooltips: Vec<String>,
    focused: Vec<String>,
    popups: Vec<serde_json::Value>,
    hex_input: Option<serde_json::Value>,
}

struct Contents {
    owner: Owner,
    observations: Rc<RefCell<Observations>>,
    mounted: Rc<Cell<bool>>,
    context: Rc<RefCell<Option<Context>>>,
}

impl CanvasContents for Contents {
    fn consume(&mut self, editor: &mut BrowserEditor, events: Vec<BrowserEvent>) {
        let mut observed = self.observations.borrow_mut();
        observed.pumps += 1;
        observed.pending_theme = editor.has_pending_theme_mutations();
        observed.theme_errors.extend(
            editor
                .take_theme_errors()
                .into_iter()
                .map(|error| error.to_string()),
        );
        for event in events {
            if let BrowserEvent::Frame(frame) = event
                && !matches!(frame, Delivery::Event { .. })
            {
                observed.failures.push(format!("unclaimed {frame:?}"));
            }
        }
    }

    fn show(&mut self, ui: &mut Ui, editor: &mut BrowserEditor) -> Option<CanvasAction> {
        *self.context.borrow_mut() = Some(ui.ctx().clone());
        self.observations.borrow_mut().frames += 1;
        self.observations.borrow_mut().popups.clear();
        let mut tooltip_traces = Vec::new();
        let ready = editor.workbench().snapshot().is_some()
            && editor.workbench().presentation().theme().is_some()
            && editor.workbench().presentation().locale().is_some();
        let app_file_owner = editor.workbench().snapshot().and_then(|snapshot| {
            let layout = snapshot.layouts.get(&self.owner.project)?;
            let taide_model::layout::PaneNode::Leaf { tabs, active, .. } =
                taide_model::layout::find_leaf(&layout.root, &self.owner.pane)?
            else {
                return None;
            };
            let tab = tabs.iter().find(|tab| Some(&tab.id) == active.as_ref())?;
            let taide_model::layout::TabKind::AppFile { target } = tab.kind else {
                return None;
            };
            Some(taide_remote_web::app_files::Owner {
                project: self.owner.project.clone(),
                key: taide_native_editor::view::ViewKey {
                    window: "settings-probe".into(),
                    pane: self.owner.pane.clone(),
                    tab: tab.id.clone(),
                },
                target,
            })
        });
        if ready
            && self.mounted.get()
            && let Some(owner) = app_file_owner.as_ref()
        {
            match editor.show_app_file(ui, owner.clone(), false) {
                Ok(Some(output)) => {
                    let center = output.response.rect.center();
                    let mut observed = self.observations.borrow_mut();
                    observed.targets = [("app-file-editor".into(), [center.x, center.y])]
                        .into_iter()
                        .collect();
                    observed
                        .failures
                        .extend(output.errors.into_iter().map(|error| format!("{error:?}")));
                }
                Ok(None) => (),
                Err(error) => self
                    .observations
                    .borrow_mut()
                    .failures
                    .push(format!("{error:?}")),
            }
        }
        if ready && self.mounted.get() && app_file_owner.is_none() {
            match editor.show_settings(ui, self.owner.clone()) {
                Ok(output) => {
                    let mut observed = self.observations.borrow_mut();
                    observed.popups = output.popup_traces.iter().map(|popup| {
                        json!({"field":popup.field,"open":popup.open,"opacity":popup.opacity,"scale":popup.scale,"active":popup.active})
                    }).collect();
                    let focused = ui.ctx().memory(|memory| memory.focused());
                    observed.focused = output
                        .editor_traces
                        .iter()
                        .filter_map(|(key, id, _)| (focused == Some(*id)).then_some(key.clone()))
                        .chain(output.traces.iter().filter_map(|trace| {
                            (focused == Some(trace.id)).then_some(trace.field.to_owned())
                        }))
                        .collect();
                    observed.hex_input = output.editor_traces.iter().find_map(|(key, id, rect)| {
                        if key != "picker-Colors-app.background-3" {
                            return None;
                        }
                        ui.ctx().read_response(*id).map(|response| {
                            json!({
                                "id":format!("{id:?}"),
                                "focused":format!("{focused:?}"),
                                "enabled":response.enabled(),
                                "hovered":response.hovered(),
                                "containsPointer":response.contains_pointer(),
                                "layer":format!("{:?}",response.layer_id),
                                "hitLayer":format!("{:?}",ui.ctx().layer_id_at(rect.center())),
                                "input":ui.input(|input|json!({"focused":input.focused,"pointer":format!("{:?}",input.pointer.hover_pos())})),
                            })
                        })
                    });
                    if let Some(error) = output.error.or(output.theme_error) {
                        observed.failures.push(format!("{error:?}"));
                    }
                    observed.targets = output
                        .traces
                        .iter()
                        .map(|trace| {
                            (
                                trace.field.into(),
                                [trace.rect.center().x, trace.rect.center().y],
                            )
                        })
                        .chain(output.editor_traces.iter().map(|(key, _, rect)| {
                            (key.clone(), [rect.center().x, rect.center().y])
                        }))
                        .collect();
                    observed.hit_targets = output
                        .traces
                        .iter()
                        .filter_map(|trace| {
                            ui.ctx()
                                .read_response(trace.id)
                                .filter(|response| response.contains_pointer())
                                .map(|_| trace.field.to_owned())
                        })
                        .chain(output.editor_traces.iter().filter_map(|(key, id, _)| {
                            ui.ctx()
                                .read_response(*id)
                                .filter(|response| response.contains_pointer())
                                .map(|_| key.clone())
                        }))
                        .collect();
                    observed.tooltip_targets = output.tooltip_traces.iter().fold(
                        std::collections::BTreeMap::new(),
                        |mut targets, (key, _, rect)| {
                            targets
                                .entry(key.clone())
                                .or_insert([rect.center().x, rect.center().y]);
                            targets
                        },
                    );
                    tooltip_traces = output.tooltip_traces;
                    if !output.changes.is_empty()
                        || !output.themes.is_empty()
                        || !output.folders.is_empty()
                        || output.open_settings_file
                    {
                        observed.failures.push("unexpected synthetic input".into());
                    }
                }
                Err(error) => self
                    .observations
                    .borrow_mut()
                    .failures
                    .push(format!("{error:?}")),
            }
        }
        let mut observed = self.observations.borrow_mut();
        let open = editor.tooltips().inspection_open(ui.ctx());
        observed.open_tooltips = tooltip_traces
            .iter()
            .filter_map(|(label, id, _)| {
                open.iter()
                    .any(|(open_id, _)| id == open_id)
                    .then_some(label.clone())
            })
            .collect();
        observed.mount = None;
        observed.generation = None;
        observed.themes.clear();
        observed.locales.clear();
        observed.catalog_errors.clear();
        observed.editing = None;
        if let Some(view) = editor.settings_views().inspection().get(&self.owner) {
            observed.mount = Some(view.mount);
            observed.generation = Some(view.generation);
            observed.editing = view
                .editor
                .as_ref()
                .and_then(|editor| editor.preview())
                .map(|theme| theme.id);
            if let Some(catalog) = &view.catalog {
                match &catalog.themes {
                    Ok(themes) => {
                        observed.themes = themes.iter().map(|theme| theme.id.clone()).collect()
                    }
                    Err(error) => observed.catalog_errors.push(format!("{error:?}")),
                }
                match &catalog.locales {
                    Ok(locales) => {
                        observed.locales = locales.iter().map(|locale| locale.id.clone()).collect()
                    }
                    Err(error) => observed.catalog_errors.push(format!("{error:?}")),
                }
            }
        }
        None
    }

    fn show_close(&mut self, ui: &mut Ui, state: &CloseState) -> Option<CanvasAction> {
        ui.label(format!("{state:?}"));
        None
    }

    fn raw_input(&mut self, _: &Context, _: &mut RawInput) {}
}

#[wasm_bindgen]
pub struct SettingsProbe {
    canvas: BrowserCanvas,
    observations: Rc<RefCell<Observations>>,
    mounted: Rc<Cell<bool>>,
    context: Rc<RefCell<Option<Context>>>,
}

#[wasm_bindgen]
impl SettingsProbe {
    pub async fn start(
        canvas: HtmlCanvasElement,
        app_files: bool,
    ) -> Result<SettingsProbe, JsValue> {
        let observations = Rc::new(RefCell::new(Observations::default()));
        let mounted = Rc::new(Cell::new(true));
        let context = Rc::new(RefCell::new(None));
        let failures = Rc::clone(&observations);
        let canvas = BrowserCanvas::start(
            canvas,
            WebOptions::default(),
            EditorLimits {
                max_documents: LIMIT,
                max_views: LIMIT,
                max_undo_groups: LIMIT,
                max_document_bytes: if app_files {
                    APP_FILE_BYTE_LIMIT
                } else {
                    BYTE_LIMIT
                },
            },
            Contents {
                owner: Owner {
                    project: ProjectId("prj-synthetic".into()),
                    pane: PaneId("synthetic-pane".into()),
                    tab: TabId("synthetic-settings".into()),
                },
                observations: Rc::clone(&observations),
                mounted: Rc::clone(&mounted),
                context: Rc::clone(&context),
            },
            Rc::new(move |error| failures.borrow_mut().failures.push(format!("{error:?}"))),
        )
        .await?;
        Ok(Self {
            canvas,
            observations,
            mounted,
            context,
        })
    }

    pub fn default_theme() -> String {
        let colors = taide_native_ui::theme_editor_tokens::COLORS
            .iter()
            .flat_map(|(namespace, keys)| {
                keys.iter().map(move |key| {
                    let lower = key.to_lowercase();
                    let value = if lower.ends_with("foreground") || *key == "iconDefault" {
                        FOREGROUND
                    } else if lower.contains("border") {
                        BORDER
                    } else if lower.contains("accent") {
                        ACCENT
                    } else if lower.contains("hover") || lower.contains("active") {
                        HOVER
                    } else {
                        BACKGROUND
                    };
                    (format!("{namespace}.{key}"), value)
                })
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let syntax = taide_native_ui::theme_editor_tokens::SYNTAX
            .iter()
            .map(|key| (*key, json!({"fg":FOREGROUND,"bold":false,"italic":false})))
            .collect::<std::collections::BTreeMap<_, _>>();
        let terminal = taide_native_ui::theme_editor_tokens::TERMINAL
            .iter()
            .map(|key| {
                (
                    *key,
                    if *key == "background" {
                        BACKGROUND
                    } else {
                        FOREGROUND
                    },
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        json!({"id":"taide-dark","name":"Synthetic","type":"dark","colors":colors,"syntax":syntax,"terminal":terminal}).to_string()
    }

    pub fn set_mounted(&self, mounted: bool) {
        self.mounted.set(mounted);
        if let Some(context) = self.context.borrow().as_ref() {
            context.request_repaint();
        }
    }

    pub fn snapshot(&self) -> String {
        let observed = self.observations.borrow();
        let toasts = self.canvas.read(|editor| editor.toasts().inspection()).unwrap_or_default()
            .into_iter().map(|toast| {
                let close = toast.close.map(|rect| [rect.center().x, rect.center().y]);
                json!({"title":toast.title,"description":toast.description,"dismissed":toast.is_dismissed,"close":close})
            }).collect::<Vec<_>>();
        let reduced_motion = self
            .canvas
            .read(|editor| editor.toasts().inspection_reduced_motion())
            .unwrap_or(false);
        let preferences = self.canvas.read(|editor| {
            let follow_system = editor.workbench().shell().state().settings().map(|settings| settings.follow_system_theme);
            let results = editor.preference_results().iter().map(|finished| format!("{:?}", finished.result)).collect::<Vec<_>>();
            json!({"pending":editor.has_pending_preferences(),"followSystem":follow_system,"results":results})
        }).unwrap_or_else(|_| json!(null));
        let code = self
            .canvas
            .read(|editor| {
                let settings = editor.workbench().shell().state().settings();
                let resources = editor.settings_views().inspection().values().map(|view| {
                json!({"fonts":view.resources.fonts(),"shells":view.resources.shells()})
            }).collect::<Vec<_>>();
                json!({"settings":settings,"resources":resources,"hitTargets":observed.hit_targets,"popups":observed.popups})
            })
            .unwrap_or_else(|_| json!(null));
        let display_theme = self
            .canvas
            .read(|editor| {
                let background = editor
                    .theme_for_viewport(egui::ViewportId::ROOT)
                    .and_then(|theme| theme.colors.get("app.background"));
                let other = editor
                    .theme_for_viewport(egui::ViewportId::from_hash_of("synthetic-other-viewport"))
                    .and_then(|theme| theme.colors.get("app.background"));
                json!({"background":background,"other":other})
            })
            .unwrap_or_else(|_| json!(null));
        let close = self
            .canvas
            .close_state()
            .map_or_else(|| "Disposed".into(), |state| format!("{state:?}"));
        let keybindings = self.context.borrow().as_ref().and_then(|context| {
            self.canvas.read(|editor| {
                let inspection = editor.keybindings_inspection(context)?;
                let targets = inspection.targets.into_iter().map(|(label, rect)| (label, [rect.center().x, rect.center().y])).collect::<std::collections::BTreeMap<_, _>>();
                Some(json!({"open":inspection.open,"capturing":inspection.capturing,"query":inspection.query,"focused":inspection.focused,"targets":targets,"hitTargets":inspection.hit_targets,"overrides":editor.workbench().shell().state().settings().and_then(|settings| settings.keymap_overrides.as_deref())}))
            }).ok().flatten()
        });
        let app_file = self.canvas.read(|editor| {
            let store = editor.files().store();
            let document = store.documents().find(&taide_native_editor::document::DocumentKey::AppFile(taide_model::app::AppFileTarget::Settings))?;
            let snapshot = store.documents().snapshot(document).ok()?;
            Some(json!({"text":snapshot.rope.to_string(),"dirty":snapshot.dirty,"pending":editor.has_pending_app_files()}))
        }).ok().flatten();
        let snippets = self.canvas.read(|editor| {
            let views = editor.settings_views();
            let states = views.inspection().values().filter_map(|view| view.snippets.as_ref()).map(|editor| {
                let state = editor.state();
                let drafts = state.drafts().map(|drafts| drafts.iter().map(|draft| json!({"name":draft.name,"prefix":draft.prefix,"body":draft.body,"scope":draft.scope})).collect::<Vec<_>>());
                json!({"files":state.files(),"selected":state.selected_file_name(),"dirty":state.has_unsaved_changes(),"drafts":drafts,"newOpen":state.new_file.open,"option":state.new_file.option(),"globalName":state.new_file.global_name,"deleteOpen":state.delete_file_open,"discard":state.pending_discard().is_some()})
            }).collect::<Vec<_>>();
            json!({"pending":editor.has_pending_snippet_mutations(),"states":states,"catalog":views.snippet_catalog().files()})
        }).unwrap_or_else(|_| json!(null));
        json!({"frames":observed.frames,"pumps":observed.pumps,"mount":observed.mount,"generation":observed.generation,"themes":observed.themes,"locales":observed.locales,"catalogErrors":observed.catalog_errors,"failures":observed.failures,"panicked":self.canvas.has_panicked(),"close":close,"editing":observed.editing,"pendingTheme":observed.pending_theme,"themeErrors":observed.theme_errors,"targets":observed.targets,"tooltipTargets":observed.tooltip_targets,"openTooltips":observed.open_tooltips,"toasts":toasts,"reducedMotion":reduced_motion,"preferences":preferences,"displayTheme":display_theme,"focused":observed.focused,"hexInput":observed.hex_input,"keybindings":keybindings,"appFile":app_file,"code":code,"snippets":snippets}).to_string()
    }

    pub fn request_close(&self) -> Result<(), JsValue> {
        self.canvas
            .request_close()
            .map(|_| ())
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn cancel_close(&self) -> Result<(), JsValue> {
        self.canvas
            .cancel_close()
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }
}
