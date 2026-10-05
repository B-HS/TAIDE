use std::cell::{Cell, RefCell};
use std::rc::Rc;

use serde_json::json;
use taide_model::ids::{PaneId, TabId};
use taide_model::settings::Settings;
use taide_native_editor::document::DiskChoice;
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::ViewKey;
use taide_native_ui::settings_controls::{Change, Switch};
use taide_remote_web::close::CloseState;
use taide_remote_web::files::{FileEvent, FileState};
use taide_remote_web::{ApplicationError, BrowserApplication, BrowserEvent, Delivery};
use wasm_bindgen::prelude::*;

const PATH: &str = r"C:\synthetic\file.rs";
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 128;

#[derive(Default)]
struct Observations {
    pumps: u32,
    loaded: u32,
    recoveries: u32,
    failures: Vec<String>,
    responses: Vec<String>,
    preferences: Vec<String>,
    settings_events: u32,
}

#[wasm_bindgen]
pub struct ApplicationProbe {
    application: BrowserApplication,
    observations: Rc<RefCell<Observations>>,
    changes: Rc<Cell<u32>>,
    key: ViewKey,
    disposed: bool,
    context: egui::Context,
}

#[wasm_bindgen]
impl ApplicationProbe {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let observations = Rc::new(RefCell::new(Observations::default()));
        let observed = Rc::clone(&observations);
        let changes = Rc::new(Cell::new(0u32));
        let changed = Rc::clone(&changes);
        let application = BrowserApplication::new(
            EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            },
            Rc::new(move |editor, events| {
                let mut observed = observed.borrow_mut();
                observed.pumps += 1;
                for event in events {
                    match event {
                        BrowserEvent::Connected { recovered: true } => observed.recoveries += 1,
                        BrowserEvent::Frame(Delivery::Event { event, .. })
                            if event == "settings:changed" =>
                        {
                            observed.settings_events += 1
                        }
                        BrowserEvent::Frame(frame) => observed.responses.push(format!("{frame:?}")),
                        _ => {}
                    }
                }
                for event in editor.take_file_events() {
                    match event {
                        FileEvent::Loaded { .. } => observed.loaded += 1,
                        FileEvent::SaveFinished {
                            result: Err(error), ..
                        } => observed.failures.push(format!("{error:?}")),
                        _ => {}
                    }
                }
                observed.failures.extend(
                    editor
                        .take_mirror_failures()
                        .into_iter()
                        .map(|error| format!("{error:?}")),
                );
                for finished in editor.take_preference_results() {
                    let result = match finished.result {
                        Ok(_) => "ok".into(),
                        Err(error) => format!("{error:?}"),
                    };
                    observed
                        .preferences
                        .push(format!("{:?}:{result}", finished.change));
                }
            }),
            Rc::new(move || changed.set(changed.get().saturating_add(1))),
        )
        .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        let key = ViewKey {
            window: "remote".into(),
            pane: PaneId("synthetic-pane".into()),
            tab: TabId("synthetic".into()),
        };
        application
            .update(|editor| editor.files_mut().bind(key.clone(), PATH))
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        Ok(Self {
            application,
            observations,
            changes,
            key,
            disposed: false,
            context: egui::Context::default(),
        })
    }

    pub fn default_settings() -> String {
        serde_json::to_string(&Settings::default()).expect("serializable synthetic settings")
    }

    pub fn snapshot(&self) -> Result<String, JsValue> {
        let observations = self.observations.borrow();
        let close = self.application.close_state();
        if self.disposed || matches!(close, CloseState::Ready) {
            return Ok(json!({"disposed":true,"close":format!("{close:?}"),"pumps":observations.pumps,"changes":self.changes.get(),"failures":observations.failures,"responses":observations.responses,"preferences":observations.preferences,"settingsEvents":observations.settings_events}).to_string());
        }
        self.application.read(|editor| {
            let snapshot = match editor.files().state(PATH) {
                FileState::Ready(document) => editor.files().store().documents().snapshot(document).ok(),
                _ => None,
            };
            json!({
                "disposed":false,"ready":snapshot.is_some(),"text":snapshot.as_ref().map(|snapshot|snapshot.rope.to_string()),
                "close":format!("{close:?}"),
                "pendingOperations":editor.files().has_pending_operations(),"dirtyReady":matches!(editor.dirty_flush_state(),taide_remote_web::dirty::FlushState::Ready),
                "dirty":snapshot.as_ref().map(|snapshot|snapshot.dirty),"connected":editor.workbench().is_connected(),
                "scopeReady":editor.workbench().snapshot().is_some(),"mirrorReady":editor.workbench().snapshot().and_then(|snapshot|snapshot.projects.first()).is_some_and(|project|editor.mirrors().mirrors(&project.id).is_some()),
                "mirrorStatus":format!("{:?}",editor.mirror_flush_status()),"timerArmed":editor.mirror_timer_armed(),
                "pumps":observations.pumps,"changes":self.changes.get(),"recoveries":observations.recoveries,
                "loaded":observations.loaded,"failures":observations.failures,"responses":observations.responses,
                "schedulerFailed":self.application.scheduler_failed()
                ,"preferences":observations.preferences,"settingsEvents":observations.settings_events,"pendingPreferences":editor.has_pending_preferences(),
                "showSystemUsage":editor.workbench().shell().state().settings().map(|settings|settings.show_system_usage),
                "language":editor.workbench().shell().state().settings().map(|settings|&settings.language),
                "localeId":editor.workbench().presentation().locale().map(|locale|&locale.id)
            }).to_string()
        }).map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn bind_scope(&self) -> Result<(), JsValue> {
        self.application
            .update(|editor| {
                let project = editor
                    .workbench()
                    .snapshot()
                    .and_then(|snapshot| snapshot.projects.first())
                    .cloned()
                    .ok_or_else(|| JsValue::from_str("Synthetic project is missing"))?;
                editor
                    .bind_project_file(self.key.clone(), PATH, &project)
                    .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
                Ok(())
            })
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?
    }

    pub fn edit(&self) -> Result<(), JsValue> {
        self.application
            .update(|editor| {
                let mut error = None;
                for events in [Vec::new(), vec![egui::Event::Text("typed ".into())]] {
                    let mut output = self.context.run_ui(
                        egui::RawInput {
                            events,
                            ..Default::default()
                        },
                        |ui| {
                            if let Err(failure) = editor.show_file(ui, &self.key, true) {
                                error = Some(failure);
                            }
                        },
                    );
                    output.textures_delta.clear();
                }
                match error {
                    Some(error) => Err(JsValue::from_str(&format!("{error:?}"))),
                    None => Ok(()),
                }
            })
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?
    }

    pub fn flush(&self) -> Result<(), JsValue> {
        self.application
            .update(|editor| editor.flush_mirrors())
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn save(&self) -> Result<(), JsValue> {
        self.application
            .update(|editor| {
                let view = editor
                    .files()
                    .view(&self.key)
                    .ok_or_else(|| JsValue::from_str("Synthetic view is missing"))?;
                editor
                    .save_prepared(view)
                    .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
                Ok(())
            })
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?
    }

    pub fn choose(&self) -> Result<(), JsValue> {
        self.application
            .update(|editor| {
                let view = editor
                    .files()
                    .view(&self.key)
                    .ok_or_else(|| JsValue::from_str("Synthetic view is missing"))?;
                editor
                    .request_disk_choice(view, DiskChoice::KeepMine)
                    .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
                Ok(())
            })
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?
    }

    pub fn change_preference(&self, name: &str) -> Result<(), JsValue> {
        self.application
            .update(|editor| {
                let change = match name {
                    "switch" => {
                        let settings = editor
                            .workbench()
                            .shell()
                            .state()
                            .settings()
                            .ok_or_else(|| JsValue::from_str("Synthetic settings not ready"))?;
                        Change::Switch(Switch::ShowSystemUsage, !settings.show_system_usage)
                    }
                    "language" => Change::Language("ja".into()),
                    "theme" => Change::Theme("synthetic-next".into()),
                    "failed-theme" => Change::Theme("synthetic-failed".into()),
                    _ => return Err(JsValue::from_str("Invalid synthetic setting")),
                };
                editor
                    .change_preference(change)
                    .map(|_| ())
                    .map_err(|error| JsValue::from_str(&format!("{error:?}")))
            })
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?
    }

    pub fn cut_connection(&self) -> Result<(), JsValue> {
        self.application
            .update(|editor| {
                editor
                    .workbench()
                    .invoke("synthetic_cut_connection", serde_json::Value::Null)
                    .map(|_| ())
                    .map_err(|error| JsValue::from_str(&format!("{error:?}")))
            })
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?
    }

    pub fn request_close(&self) -> Result<(), JsValue> {
        self.application
            .request_close()
            .map(|_| ())
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn cancel_close(&self) -> Result<(), JsValue> {
        self.application
            .cancel_close()
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn retry_persistence(&self) -> Result<(), JsValue> {
        self.application
            .update(|editor| {
                editor.retry_dirty();
                editor.retry_mirror_writes()
            })
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn update_is_closing(&self) -> bool {
        matches!(
            self.application.update(|_| ()),
            Err(ApplicationError::Closing)
        )
    }

    pub fn nested_update_is_busy(&self) -> Result<bool, JsValue> {
        self.application
            .read(|_| matches!(self.application.update(|_| ()), Err(ApplicationError::Busy)))
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn dispose(&mut self) {
        self.application.dispose();
        self.disposed = true;
    }
}
