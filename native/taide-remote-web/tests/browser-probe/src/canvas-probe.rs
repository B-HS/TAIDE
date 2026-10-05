use std::cell::RefCell;
use std::rc::Rc;

use egui::{Context, RawInput, Ui};
use serde_json::json;
use taide_model::ids::{PaneId, TabId};
use taide_native_editor::store::EditorLimits;
use taide_native_editor::view::ViewKey;
use taide_remote_web::canvas::{BrowserCanvas, CanvasAction, CanvasContents, WebOptions};
use taide_remote_web::close::CloseState;
use taide_remote_web::{BrowserEditor, BrowserEvent};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

const PATH: &str = r"C:\synthetic\file.rs";
const DOCUMENT_LIMIT: usize = 4;
const VIEW_LIMIT: usize = 4;
const HISTORY_LIMIT: usize = 4;
const BYTE_LIMIT: usize = 128;

#[derive(Default)]
struct Observations {
    frames: u32,
    pumps: u32,
    text_inputs: usize,
    text: Option<String>,
    dirty: bool,
    bound: bool,
    failures: Vec<String>,
}

struct Contents {
    key: ViewKey,
    observations: Rc<RefCell<Observations>>,
}

impl CanvasContents for Contents {
    fn consume(&mut self, editor: &mut BrowserEditor, events: Vec<BrowserEvent>) {
        let mut observed = self.observations.borrow_mut();
        observed.pumps += 1;
        if !observed.bound {
            let project = editor
                .workbench()
                .snapshot()
                .and_then(|snapshot| snapshot.projects.first())
                .cloned();
            if let Some(project) = project {
                match editor.bind_project_file(self.key.clone(), PATH, &project) {
                    Ok(_) => observed.bound = true,
                    Err(error) => observed.failures.push(format!("{error:?}")),
                }
            }
        }
        for event in events {
            if let BrowserEvent::Frame(frame) = event {
                observed.failures.push(format!("Unclaimed {frame:?}"));
            }
        }
        for error in editor.take_dirty_failures() {
            observed.failures.push(format!("{error:?}"));
        }
        for error in editor.take_mirror_failures() {
            observed.failures.push(format!("{error:?}"));
        }
        for event in editor.take_file_events() {
            if let taide_remote_web::files::FileEvent::SaveFinished {
                result: Err(error), ..
            } = event
            {
                observed.failures.push(format!("{error:?}"));
            }
        }
        if let Some(view) = editor.files().view(&self.key)
            && let Some(view) = editor.files().store().views().get(view)
            && let Ok(snapshot) = editor.files().store().documents().snapshot(view.document)
        {
            observed.text = Some(snapshot.rope.to_string());
            observed.dirty = snapshot.dirty;
        }
    }

    fn show(&mut self, ui: &mut Ui, editor: &mut BrowserEditor) -> Option<CanvasAction> {
        self.observations.borrow_mut().frames += 1;
        if let Err(error) = editor.show_file(ui, &self.key, true) {
            self.observations
                .borrow_mut()
                .failures
                .push(format!("{error:?}"));
        }
        None
    }

    fn show_close(&mut self, ui: &mut Ui, state: &CloseState) -> Option<CanvasAction> {
        ui.label(format!("{state:?}"));
        None
    }

    fn raw_input(&mut self, _: &Context, input: &mut RawInput) {
        self.observations.borrow_mut().text_inputs += input
            .events
            .iter()
            .filter(|event| matches!(event, egui::Event::Text(_)))
            .count();
    }
}

#[wasm_bindgen]
pub struct CanvasProbe {
    canvas: BrowserCanvas,
    observations: Rc<RefCell<Observations>>,
}

#[wasm_bindgen]
impl CanvasProbe {
    pub async fn start(canvas: HtmlCanvasElement) -> Result<CanvasProbe, JsValue> {
        let observations = Rc::new(RefCell::new(Observations::default()));
        let failures = Rc::clone(&observations);
        let canvas = BrowserCanvas::start(
            canvas,
            WebOptions::default(),
            EditorLimits {
                max_documents: DOCUMENT_LIMIT,
                max_views: VIEW_LIMIT,
                max_undo_groups: HISTORY_LIMIT,
                max_document_bytes: BYTE_LIMIT,
            },
            Contents {
                key: ViewKey {
                    window: "remote".into(),
                    pane: PaneId("synthetic-pane".into()),
                    tab: TabId("synthetic".into()),
                },
                observations: Rc::clone(&observations),
            },
            Rc::new(move |error| failures.borrow_mut().failures.push(format!("{error:?}"))),
        )
        .await?;
        Ok(Self {
            canvas,
            observations,
        })
    }

    pub fn snapshot(&self) -> String {
        let observed = self.observations.borrow();
        let state = self
            .canvas
            .close_state()
            .map_or_else(|| "Disposed".into(), |state| format!("{state:?}"));
        json!({"frames":observed.frames,"pumps":observed.pumps,"textInputs":observed.text_inputs,"text":observed.text,"dirty":observed.dirty,"failures":observed.failures,"panicked":self.canvas.has_panicked(),"close":state}).to_string()
    }

    pub fn request_close(&self) -> Result<(), JsValue> {
        self.canvas
            .request_close()
            .map(|_| ())
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }
    pub fn abort(&self) {
        self.canvas.abort();
    }
}
