use std::cell::Cell;
use std::rc::Rc;

use serde_json::{Value, json};
use taide_model::settings::Settings;
use taide_remote_web::presentation::Read;
use taide_remote_web::{BrowserEvent, BrowserWorkbench, Delivery};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct PresentationProbe {
    workbench: BrowserWorkbench,
    wakes: Rc<Cell<u32>>,
    recoveries: u32,
    responses: Vec<u32>,
    themes: Vec<String>,
}

#[wasm_bindgen]
impl PresentationProbe {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let wakes = Rc::new(Cell::new(0u32));
        let observed = Rc::clone(&wakes);
        let workbench = BrowserWorkbench::new(Rc::new(move || {
            observed.set(observed.get().saturating_add(1));
        }))
        .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        Ok(Self {
            workbench,
            wakes,
            recoveries: 0,
            responses: Vec::new(),
            themes: Vec::new(),
        })
    }

    pub fn default_settings() -> String {
        serde_json::to_string(&Settings::default()).expect("serializable synthetic settings")
    }

    pub fn retry_locale(&mut self) {
        self.workbench.retry_presentation(Read::Locale);
    }

    pub fn cut(&self) -> Result<u32, JsValue> {
        self.workbench
            .invoke("synthetic_cut_connection", Value::Null)
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn dispose(&mut self) {
        self.workbench.dispose();
    }

    pub fn poll(&mut self) -> String {
        for event in self.workbench.poll() {
            match event {
                BrowserEvent::Connected { recovered: true } => {
                    self.recoveries = self.recoveries.saturating_add(1);
                }
                BrowserEvent::Frame(Delivery::Response { seq, .. }) => self.responses.push(seq),
                _ => {}
            }
        }
        let state = self.workbench.presentation();
        let theme = state.theme().map(|theme| theme.id.as_str());
        if let Some(theme) = theme
            && self.themes.last().is_none_or(|previous| previous != theme)
        {
            self.themes.push(theme.into());
        }
        let color = state
            .shell_colors()
            .map(|colors| colors.map(|colors| colors.background.to_array()))
            .transpose()
            .expect("synthetic theme colors are valid");
        let editor = self
            .workbench
            .shell()
            .state()
            .settings()
            .and_then(|settings| state.editor_appearance(settings))
            .transpose()
            .expect("synthetic editor appearance is valid");
        json!({
            "connected": self.workbench.is_connected(),
            "ready": self.workbench.snapshot().is_some() && state.theme().is_some() && state.locale().is_some(),
            "systemTheme": state.inputs().theme,
            "systemLanguage": state.inputs().language,
            "theme": theme,
            "locale": state.locale().map(|locale| locale.id.as_str()),
            "message": state.message("hello", &[("name", "Rust")]),
            "background": color,
            "editorFont": editor.map(|editor| editor.font.size),
            "failures": state.failures().len(),
            "refreshing": state.is_refreshing(),
            "recoveries": self.recoveries,
            "responses": self.responses,
            "themes": self.themes,
            "wakes": self.wakes.get(),
        })
        .to_string()
    }
}
