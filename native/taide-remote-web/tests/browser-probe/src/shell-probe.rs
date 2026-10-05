use std::cell::Cell;
use std::rc::Rc;

use serde_json::{Value, json};
use taide_model::ids::{ProjectId, TabId};
use taide_model::settings::Settings;
use taide_native_ui::commands::ShellMutation;
use taide_native_ui::snapshot::active_tab;
use taide_remote_web::{BrowserEvent, BrowserShell, Delivery};
use wasm_bindgen::prelude::*;

const SYNTHETIC_TAB: &str = "tab-synthetic";

#[wasm_bindgen]
pub struct ShellProbe {
    shell: BrowserShell,
    wakes: Rc<Cell<u32>>,
    uuid: String,
    recoveries: u32,
    responses: Vec<u32>,
}

#[wasm_bindgen]
impl ShellProbe {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let wakes = Rc::new(Cell::new(0u32));
        let observed = wakes.clone();
        let shell = BrowserShell::new(Rc::new(move || {
            observed.set(observed.get().saturating_add(1));
        }))
        .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        Ok(Self {
            shell,
            wakes,
            uuid: ProjectId::new().as_str().into(),
            recoveries: 0,
            responses: Vec::new(),
        })
    }

    pub fn default_settings() -> String {
        serde_json::to_string(&Settings::default()).expect("serializable synthetic settings")
    }

    pub fn pin(&self) -> Result<u32, JsValue> {
        self.shell
            .mutate(&ShellMutation::PinTab {
                tab: TabId(SYNTHETIC_TAB.into()),
                pinned: true,
            })
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn refresh(&mut self) {
        self.shell.refresh();
    }

    pub fn cut(&self) -> Result<u32, JsValue> {
        self.shell
            .invoke("synthetic_cut_connection", Value::Null)
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn poll(&mut self) -> String {
        for event in self.shell.poll() {
            match event {
                BrowserEvent::Connected { recovered: true } => {
                    self.recoveries = self.recoveries.saturating_add(1);
                }
                BrowserEvent::Frame(Delivery::Response { seq, .. }) => self.responses.push(seq),
                _ => {}
            }
        }
        let snapshot = self.shell.snapshot();
        let projects = snapshot.map(|snapshot| {
            snapshot
                .projects
                .iter()
                .map(|project| project.id.as_str())
                .collect::<Vec<_>>()
        });
        let focused = snapshot.and_then(|snapshot| snapshot.focused_project());
        let focused_layout = focused.and_then(|id| snapshot?.layouts.get(id));
        let focused_tab =
            focused_layout.and_then(|layout| active_tab(&layout.root, &layout.focused_pane));
        json!({
            "connected": self.shell.is_connected(),
            "ready": snapshot.is_some(),
            "projects": projects,
            "layoutCount": snapshot.map(|snapshot| snapshot.layouts.len()),
            "focused": focused,
            "revision": focused_layout.map(|layout| layout.revision),
            "pinned": focused_tab.map(|tab| tab.pinned),
            "hideStatus": snapshot.map(|snapshot| snapshot.hide_status_in_zen),
            "failures": self.shell.state().failures().len(),
            "recoveries": self.recoveries,
            "responses": self.responses,
            "wakes": self.wakes.get(),
            "uuid": self.uuid,
        })
        .to_string()
    }
}
