use std::cell::RefCell;
use std::rc::Rc;

use egui::{Context, RawInput, Ui};
use taide_native_editor::store::EditorLimits;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use crate::close::CloseState;
use crate::{ApplicationError, BrowserApplication, BrowserEditor, BrowserEvent};

pub use eframe::WebOptions;

pub enum CanvasAction {
    RequestClose,
    CancelClose,
}

pub trait CanvasContents {
    fn consume(&mut self, editor: &mut BrowserEditor, events: Vec<BrowserEvent>);
    fn show(&mut self, ui: &mut Ui, editor: &mut BrowserEditor) -> Option<CanvasAction>;
    fn show_close(&mut self, ui: &mut Ui, state: &CloseState) -> Option<CanvasAction>;
    fn raw_input(&mut self, context: &Context, input: &mut RawInput);
}

pub struct BrowserCanvas {
    runner: Rc<eframe::WebRunner>,
    application: Rc<RefCell<Option<Rc<BrowserApplication>>>>,
}

struct CanvasApp<C> {
    application: Rc<BrowserApplication>,
    contents: Rc<RefCell<C>>,
    failure: Rc<dyn Fn(ApplicationError)>,
}

impl<C: CanvasContents> eframe::App for CanvasApp<C> {
    fn raw_input_hook(&mut self, context: &Context, input: &mut RawInput) {
        if matches!(self.application.close_state(), CloseState::Open) {
            if self
                .application
                .read(|editor| editor.keybindings_open())
                .is_ok_and(|open| !open)
            {
                self.contents.borrow_mut().raw_input(context, input);
            }
        }
    }

    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        let state = self.application.close_state();
        let contents = &self.contents;
        let action = match state {
            CloseState::Open => match self.application.update(|editor| {
                editor.begin_ui_frame(ui.ctx());
                let action = ui
                    .add_enabled_ui(!editor.keybindings_open(), |ui| {
                        contents.borrow_mut().show(ui, editor)
                    })
                    .inner;
                editor.finish_ui_frame(ui.ctx());
                action
            }) {
                Ok(action) => action,
                Err(error) => {
                    (self.failure)(error);
                    return;
                }
            },
            _ => contents.borrow_mut().show_close(ui, &state),
        };
        if let Err(error) = self.application.show_feedback(ui.ctx()) {
            (self.failure)(error);
        }
        let result = match action {
            Some(CanvasAction::RequestClose) => self.application.request_close().map(|_| ()),
            Some(CanvasAction::CancelClose) => self.application.cancel_close(),
            None => return,
        };
        if let Err(error) = result {
            (self.failure)(error);
        }
    }

    fn persist_egui_memory(&self) -> bool {
        false
    }
}

impl BrowserCanvas {
    pub async fn start<C: CanvasContents + 'static>(
        canvas: HtmlCanvasElement,
        options: eframe::WebOptions,
        limits: EditorLimits,
        contents: C,
        failure: Rc<dyn Fn(ApplicationError)>,
    ) -> Result<Self, JsValue> {
        let runner = Rc::new(eframe::WebRunner::new());
        let application = Rc::new(RefCell::new(None));
        let contents = Rc::new(RefCell::new(contents));
        let owner = Rc::clone(&application);
        let callback_owner = Rc::downgrade(&application);
        let callback_runner = Rc::downgrade(&runner);
        let host = Self {
            runner,
            application,
        };
        let result = host
            .runner
            .start(
                canvas,
                options,
                Box::new(move |creation| {
                    let repaint = creation.egui_ctx.clone();
                    let consumed = Rc::clone(&contents);
                    let application = Rc::new(
                        BrowserApplication::new(
                            limits,
                            Rc::new(move |editor, events| {
                                consumed.borrow_mut().consume(editor, events)
                            }),
                            Rc::new(move || {
                                if callback_owner.upgrade().is_some_and(|owner| {
                                    owner.borrow().as_ref().is_some_and(
                                        |application: &Rc<BrowserApplication>| {
                                            matches!(application.close_state(), CloseState::Ready)
                                        },
                                    )
                                }) {
                                    if let Some(runner) = callback_runner.upgrade() {
                                        runner.destroy();
                                    }
                                    return;
                                }
                                repaint.request_repaint();
                            }),
                        )
                        .map_err(|error| format!("{error:?}"))?,
                    );
                    *owner.borrow_mut() = Some(Rc::clone(&application));
                    Ok(Box::new(CanvasApp {
                        application,
                        contents,
                        failure,
                    }))
                }),
            )
            .await;
        if let Err(error) = result {
            host.abort();
            return Err(error);
        }
        Ok(host)
    }

    pub fn request_close(&self) -> Result<CloseState, ApplicationError> {
        let application = self.application.borrow();
        application
            .as_ref()
            .ok_or(ApplicationError::Disposed)?
            .request_close()
    }

    pub fn cancel_close(&self) -> Result<(), ApplicationError> {
        let application = self.application.borrow();
        application
            .as_ref()
            .ok_or(ApplicationError::Disposed)?
            .cancel_close()
    }

    pub fn close_state(&self) -> Option<CloseState> {
        self.application
            .borrow()
            .as_ref()
            .map(|application| application.close_state())
    }

    pub fn read<T>(&self, read: impl FnOnce(&BrowserEditor) -> T) -> Result<T, ApplicationError> {
        let application = self
            .application
            .borrow()
            .clone()
            .ok_or(ApplicationError::Disposed)?;
        application.read(read)
    }

    pub fn has_panicked(&self) -> bool {
        self.runner.has_panicked()
    }

    pub fn abort(&self) {
        self.runner.destroy();
        if let Some(application) = self.application.borrow_mut().take() {
            application.dispose();
        }
    }
}

impl Drop for BrowserCanvas {
    fn drop(&mut self) {
        self.abort();
    }
}
