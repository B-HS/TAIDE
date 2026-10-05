use std::cell::Cell;
use std::rc::Rc;

use serde_json::{Value, json};
use taide_model::ids::{PaneId, TabId};
use taide_model::settings::Settings;
use taide_native_editor::document::DiskChoice;
use taide_native_editor::editing::replace_selections;
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::ViewKey;
use taide_remote_web::files::{ChoiceRequest, FileEvent, FileState};
use taide_remote_web::{BrowserEditor, BrowserEvent, Delivery, ResponsePayload};
use wasm_bindgen::prelude::*;

const PATH: &str = r"C:\synthetic\file.rs";
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 128;

#[wasm_bindgen]
pub struct FileProbe {
    editor: BrowserEditor,
    context: egui::Context,
    first: ViewKey,
    second: ViewKey,
    wakes: Rc<Cell<u32>>,
    recoveries: u32,
    loaded: u32,
    saves: Vec<bool>,
    save_failures: Vec<String>,
    responses: Vec<u32>,
    input: Vec<egui::Event>,
    pending_choice: Option<(u32, ChoiceRequest)>,
    choices: Vec<bool>,
    choice_failures: Vec<String>,
    dirty_flushes: u32,
    dirty_failures: Vec<String>,
    restored: u32,
    mirror_restore_failures: Vec<String>,
    mirror_failures: Vec<String>,
}

#[wasm_bindgen]
impl FileProbe {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let wakes = Rc::new(Cell::new(0u32));
        let observed = Rc::clone(&wakes);
        let mut editor = BrowserEditor::new(
            EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            },
            Rc::new(move || observed.set(observed.get().saturating_add(1))),
        )
        .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        let first = ViewKey {
            window: "remote".into(),
            pane: PaneId("first".into()),
            tab: TabId("synthetic".into()),
        };
        let mut second = first.clone();
        second.pane = PaneId("second".into());
        for key in [&first, &second] {
            editor
                .files_mut()
                .bind(key.clone(), PATH)
                .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        }
        Ok(Self {
            editor,
            context: egui::Context::default(),
            first,
            second,
            wakes,
            recoveries: 0,
            loaded: 0,
            saves: Vec::new(),
            save_failures: Vec::new(),
            responses: Vec::new(),
            input: Vec::new(),
            pending_choice: None,
            choices: Vec::new(),
            choice_failures: Vec::new(),
            dirty_flushes: 0,
            dirty_failures: Vec::new(),
            restored: 0,
            mirror_restore_failures: Vec::new(),
            mirror_failures: Vec::new(),
        })
    }

    pub fn default_settings() -> String {
        serde_json::to_string(&Settings::default()).expect("serializable synthetic settings")
    }

    pub fn bind_project_scope(&mut self) -> Result<(), JsValue> {
        let project = self
            .editor
            .workbench()
            .snapshot()
            .and_then(|snapshot| snapshot.projects.first())
            .cloned()
            .ok_or_else(|| JsValue::from_str("Synthetic project snapshot is missing"))?;
        for key in [&self.first, &self.second] {
            self.editor
                .bind_project_file(key.clone(), PATH, &project)
                .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        }
        Ok(())
    }

    pub fn retry_mirrors(&mut self) -> Result<(), JsValue> {
        let project = self
            .editor
            .workbench()
            .snapshot()
            .and_then(|snapshot| snapshot.projects.first())
            .map(|project| project.id.clone())
            .ok_or_else(|| JsValue::from_str("Synthetic project snapshot is missing"))?;
        self.editor.retry_mirrors(&project);
        Ok(())
    }

    pub fn flush_mirrors(&mut self) {
        self.editor.flush_mirrors();
    }
    pub fn retry_mirror_writes(&mut self) {
        self.editor.retry_mirror_writes();
    }
    pub fn mirror_diagnostics(&self) -> String {
        json!({"wakes":self.wakes.get(),"timerArmed":self.editor.mirror_timer_armed(),"status":format!("{:?}",self.editor.mirror_flush_status())}).to_string()
    }

    pub fn edit(&mut self, text: &str) -> Result<(), JsValue> {
        let view = self
            .editor
            .files()
            .view(&self.first)
            .ok_or_else(|| JsValue::from_str("Synthetic view is missing"))?;
        replace_selections(self.editor.files_mut().store_mut(), view, text, None)
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        Ok(())
    }

    pub fn save(&mut self) -> Result<bool, JsValue> {
        let view = self
            .editor
            .files()
            .view(&self.first)
            .ok_or_else(|| JsValue::from_str("Synthetic view is missing"))?;
        self.editor
            .save_prepared(view)
            .map(|seq| seq.is_some())
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn retry(&mut self) {
        self.editor.files_mut().retry(PATH);
    }

    pub fn choose(&mut self, keep_mine: bool) -> Result<(), JsValue> {
        if self.pending_choice.is_some() {
            return Err(JsValue::from_str("Synthetic dirty flush is pending"));
        }
        let files = self.editor.files();
        let view = files
            .view(&self.first)
            .ok_or_else(|| JsValue::from_str("Synthetic view is missing"))?;
        let choice = if keep_mine {
            DiskChoice::KeepMine
        } else {
            DiskChoice::ViewDisk
        };
        let request = files
            .prepare_choice(view, choice)
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?
            .ok_or_else(|| JsValue::from_str("Synthetic choice is unavailable"))?;
        let dirty = files
            .store()
            .documents()
            .snapshot(request.document())
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?
            .dirty;
        let seq = self
            .editor
            .workbench()
            .invoke(
                "layout_set_dirty",
                json!({"tabId":self.first.tab,"dirty":dirty}),
            )
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        self.pending_choice = Some((seq, request));
        Ok(())
    }

    pub fn choose_owned(&mut self, keep_mine: bool) -> Result<bool, JsValue> {
        let view = self
            .editor
            .files()
            .view(&self.first)
            .ok_or_else(|| JsValue::from_str("Synthetic view is missing"))?;
        self.editor
            .request_disk_choice(
                view,
                if keep_mine {
                    DiskChoice::KeepMine
                } else {
                    DiskChoice::ViewDisk
                },
            )
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn text_input(&mut self, text: &str) {
        self.input.push(egui::Event::Text(text.into()));
    }

    pub fn retry_dirty(&mut self) {
        self.editor.retry_dirty();
    }

    pub fn edit_then_cut(&mut self) -> Result<u32, JsValue> {
        self.text_input("typed ");
        self.poll()?;
        self.cut()
    }

    pub fn tab(&mut self) {
        self.input.push(egui::Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::default(),
        });
    }

    pub fn cut(&self) -> Result<u32, JsValue> {
        self.editor
            .workbench()
            .invoke("synthetic_cut_connection", Value::Null)
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn unbind(&mut self, second: bool) -> Result<(), JsValue> {
        let key = if second { &self.second } else { &self.first };
        self.editor
            .unbind_file(key)
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn bind(&mut self) -> Result<(), JsValue> {
        self.editor
            .files_mut()
            .bind(self.first.clone(), PATH)
            .map(|_| ())
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn dispose(&mut self) {
        self.editor.dispose();
    }

    pub fn poll(&mut self) -> Result<String, JsValue> {
        for event in self.editor.poll() {
            match event {
                BrowserEvent::Connected { recovered: true } => self.recoveries += 1,
                BrowserEvent::Frame(Delivery::Response { seq, result }) => {
                    if self
                        .pending_choice
                        .as_ref()
                        .is_some_and(|(pending, _)| *pending == seq)
                    {
                        let (_, request) =
                            self.pending_choice.take().expect("checked pending choice");
                        if !matches!(result, Ok(ResponsePayload::Json(Value::Null))) {
                            return Err(JsValue::from_str("Synthetic dirty flush failed"));
                        }
                        self.dirty_flushes += 1;
                        self.editor
                            .choose_disk_after_dirty_flush(request)
                            .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
                    } else {
                        self.responses.push(seq);
                    }
                }
                _ => {}
            }
        }
        for error in self.editor.take_dirty_failures() {
            self.dirty_failures.push(format!("{error:?}"));
        }
        for error in self.editor.take_mirror_restore_failures() {
            self.mirror_restore_failures.push(format!("{error:?}"));
        }
        for error in self.editor.take_mirror_failures() {
            self.mirror_failures.push(format!("{error:?}"));
        }
        for event in self.editor.take_file_events() {
            match event {
                FileEvent::Restored { .. } => self.restored += 1,
                FileEvent::Loaded { .. } => self.loaded += 1,
                FileEvent::SaveFinished {
                    result: Ok(dirty), ..
                } => self.saves.push(dirty),
                FileEvent::SaveFinished {
                    result: Err(error), ..
                } => {
                    self.save_failures.push(format!("{error:?}"));
                }
                FileEvent::ChoiceFinished {
                    result: Ok(dirty), ..
                } => self.choices.push(dirty),
                FileEvent::ChoiceFinished {
                    result: Err(error), ..
                } => self.choice_failures.push(format!("{error:?}")),
                _ => {}
            }
        }
        let mut render_error = None;
        let editor = &mut self.editor;
        let first = &self.first;
        let mut output = self.context.run_ui(
            egui::RawInput {
                events: std::mem::take(&mut self.input),
                ..Default::default()
            },
            |ui| {
                if editor.files().view(first).is_some()
                    && let Err(error) = editor.show_file(ui, first, true)
                {
                    render_error = Some(error);
                }
            },
        );
        if let Some(error) = render_error {
            return Err(JsValue::from_str(&format!("{error:?}")));
        }
        let rendered = output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect::<Vec<_>>();
        output.textures_delta.clear();
        let files = self.editor.files();
        let snapshot = files.store().documents().snapshots().next();
        let project = self
            .editor
            .workbench()
            .snapshot()
            .and_then(|snapshot| snapshot.projects.first());
        let mirrors = project.and_then(|project| self.editor.mirrors().mirrors(&project.id));
        Ok(json!({
            "connected": self.editor.workbench().is_connected(),
            "ready": matches!(files.state(PATH), FileState::Ready(_)),
            "documents": files.store().documents().len(),
            "views": files.store().views().len(),
            "text": snapshot.as_ref().map(|snapshot| snapshot.rope.to_string()),
            "dirty": snapshot.as_ref().map(|snapshot| snapshot.dirty),
            "readOnly": snapshot.as_ref().map(|snapshot| snapshot.metadata.read_only),
            "conflict": snapshot.as_ref().map(|snapshot| files.store().has_disk_conflict(snapshot.id).expect("synthetic document exists")),
            "rendered": rendered,
            "loaded": self.loaded,
            "saves": self.saves,
            "saveFailures": self.save_failures,
            "choices": self.choices,
            "choiceFailures": self.choice_failures,
            "dirtyFlushes": self.dirty_flushes,
            "dirtyFailures": self.dirty_failures,
            "dirtyFlushed": matches!(self.editor.dirty_flush_state(), taide_remote_web::dirty::FlushState::Ready),
            "scopeReady": project.is_some(),
            "mirrorReady": mirrors.is_some(),
            "mirrorTexts": mirrors.map(|entries| entries.iter().map(|entry| entry.content.as_str()).collect::<Vec<_>>()),
            "restored": self.restored,
            "mirrorRestoreFailures": self.mirror_restore_failures,
            "mirrorFailures": self.mirror_failures,
            "mirrorStatus": format!("{:?}", self.editor.mirror_flush_status()),
            "mirrorTimerArmed": self.editor.mirror_timer_armed(),
            "recoveries": self.recoveries,
            "responses": self.responses,
            "wakes": self.wakes.get(),
        }).to_string())
    }
}
