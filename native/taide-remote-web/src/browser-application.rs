use std::cell::{Cell, RefCell};
use std::rc::Rc;

use taide_native_editor::store::EditorLimits;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::Window;

use crate::close::{CloseFailure, CloseState, drain_state};
use crate::files::FileEvent;
use crate::{BrowserEditor, BrowserEvent, EditorBrowserError};

type ConsumeEvents = Rc<dyn Fn(&mut BrowserEditor, Vec<BrowserEvent>)>;

#[derive(Debug)]
pub enum ApplicationError {
    Editor(EditorBrowserError),
    WindowUnavailable,
    SchedulerUnavailable,
    Busy,
    Disposed,
    Closing,
}

struct Inner {
    window: Window,
    editor: RefCell<Option<BrowserEditor>>,
    consume: ConsumeEvents,
    changed: Rc<dyn Fn()>,
    callback: RefCell<Option<Closure<dyn FnMut()>>>,
    scheduled: Cell<Option<i32>>,
    disposed: Cell<bool>,
    scheduler_failed: Cell<bool>,
    notify: Cell<bool>,
    close: RefCell<CloseState>,
}

impl Inner {
    fn schedule(&self) -> Result<(), ApplicationError> {
        if self.disposed.get() {
            return Err(ApplicationError::Disposed);
        }
        if self.scheduled.get().is_some() {
            return Ok(());
        }
        let callback = self
            .callback
            .try_borrow()
            .map_err(|_| ApplicationError::Busy)?;
        let callback = callback
            .as_ref()
            .ok_or(ApplicationError::SchedulerUnavailable)?;
        let id = self
            .window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                0,
            )
            .map_err(|_| ApplicationError::SchedulerUnavailable)?;
        self.scheduled.set(Some(id));
        self.scheduler_failed.set(false);
        Ok(())
    }

    fn wake(&self) {
        if self.disposed.get() {
            return;
        }
        self.notify.set(true);
        if self.schedule().is_err() {
            self.scheduler_failed.set(true);
            if matches!(*self.close.borrow(), CloseState::Pending) {
                *self.close.borrow_mut() = CloseState::Failed(CloseFailure::SchedulerUnavailable);
            }
            (self.changed)();
        }
    }

    fn dispose(&self) {
        self.disposed.set(true);
        if let Some(id) = self.scheduled.take() {
            self.window.clear_timeout_with_handle(id);
        }
        if let Ok(mut editor) = self.editor.try_borrow_mut()
            && let Some(editor) = editor.as_mut()
        {
            editor.dispose();
        }
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(id) = self.scheduled.take() {
            self.window.clear_timeout_with_handle(id);
        }
    }
}

pub struct BrowserApplication {
    inner: Rc<Inner>,
}

impl BrowserApplication {
    pub fn new(
        limits: EditorLimits,
        consume: ConsumeEvents,
        changed: Rc<dyn Fn()>,
    ) -> Result<Self, ApplicationError> {
        let inner = Rc::new(Inner {
            window: web_sys::window().ok_or(ApplicationError::WindowUnavailable)?,
            editor: RefCell::new(None),
            consume,
            changed,
            callback: RefCell::new(None),
            scheduled: Cell::new(None),
            disposed: Cell::new(false),
            scheduler_failed: Cell::new(false),
            notify: Cell::new(true),
            close: RefCell::new(CloseState::Open),
        });
        let owner = Rc::downgrade(&inner);
        let callback = Closure::new(move || {
            let Some(inner) = owner.upgrade() else {
                return;
            };
            inner.scheduled.set(None);
            if inner.disposed.get() {
                return;
            }
            let notify = inner.notify.replace(false);
            let Ok(mut borrowed) = inner.editor.try_borrow_mut() else {
                inner.wake();
                return;
            };
            if let Some(editor) = borrowed.as_mut() {
                if matches!(*inner.close.borrow(), CloseState::Pending) {
                    editor.flush_mirrors();
                }
                let events = editor.poll();
                observe_close_failure(&inner, editor);
                (inner.consume)(editor, events);
                observe_close_failure(&inner, editor);
                if matches!(*inner.close.borrow(), CloseState::Pending) {
                    let state = drain_state(
                        editor.files().has_pending_operations()
                            || editor.has_pending_preferences()
                            || editor.has_pending_theme_mutations()
                            || editor.has_pending_snippet_mutations()
                            || editor.has_pending_app_files(),
                        editor.dirty_flush_state(),
                        editor.mirror_flush_status(),
                    );
                    let settled = !matches!(state, CloseState::Pending);
                    *inner.close.borrow_mut() = state;
                    if settled {
                        inner.notify.set(true);
                    }
                    if matches!(*inner.close.borrow(), CloseState::Ready) {
                        inner.disposed.set(true);
                    }
                }
            }
            drop(borrowed);
            if inner.disposed.get() {
                inner.dispose();
                (inner.changed)();
                return;
            }
            if notify
                || !matches!(
                    *inner.close.borrow(),
                    CloseState::Open | CloseState::Pending
                )
            {
                (inner.changed)();
            }
        });
        *inner.callback.borrow_mut() = Some(callback);
        let owner = Rc::downgrade(&inner);
        let editor = BrowserEditor::new(
            limits,
            Rc::new(move || {
                if let Some(inner) = owner.upgrade() {
                    inner.wake();
                }
            }),
        )
        .map_err(ApplicationError::Editor)?;
        *inner.editor.borrow_mut() = Some(editor);
        let application = Self { inner };
        application.inner.schedule()?;
        Ok(application)
    }

    pub fn read<T>(&self, read: impl FnOnce(&BrowserEditor) -> T) -> Result<T, ApplicationError> {
        if self.inner.disposed.get() {
            return Err(ApplicationError::Disposed);
        }
        let borrowed = self
            .inner
            .editor
            .try_borrow()
            .map_err(|_| ApplicationError::Busy)?;
        let editor = borrowed.as_ref().ok_or(ApplicationError::Disposed)?;
        let result = read(editor);
        drop(borrowed);
        if self.inner.disposed.get() {
            self.inner.dispose();
        }
        Ok(result)
    }

    pub fn update<T>(
        &self,
        update: impl FnOnce(&mut BrowserEditor) -> T,
    ) -> Result<T, ApplicationError> {
        if self.inner.disposed.get() {
            return Err(ApplicationError::Disposed);
        }
        if !matches!(*self.inner.close.borrow(), CloseState::Open) {
            return Err(ApplicationError::Closing);
        }
        let mut borrowed = self
            .inner
            .editor
            .try_borrow_mut()
            .map_err(|_| ApplicationError::Busy)?;
        let editor = borrowed.as_mut().ok_or(ApplicationError::Disposed)?;
        let result = update(editor);
        drop(borrowed);
        if self.inner.disposed.get() {
            self.inner.dispose();
            return Ok(result);
        }
        if self.inner.schedule().is_err() {
            self.inner.scheduler_failed.set(true);
        }
        Ok(result)
    }

    pub fn retry_pump(&self) -> Result<(), ApplicationError> {
        self.inner.schedule()
    }

    pub(crate) fn show_feedback(&self, context: &egui::Context) -> Result<(), ApplicationError> {
        if self.inner.disposed.get() {
            return Ok(());
        }
        let mut borrowed = self
            .inner
            .editor
            .try_borrow_mut()
            .map_err(|_| ApplicationError::Busy)?;
        let editor = borrowed.as_mut().ok_or(ApplicationError::Disposed)?;
        editor
            .show_feedback(
                context,
                matches!(*self.inner.close.borrow(), CloseState::Open),
            )
            .map_err(ApplicationError::Editor)
    }

    pub fn scheduler_failed(&self) -> bool {
        self.inner.scheduler_failed.get()
    }

    pub fn request_close(&self) -> Result<CloseState, ApplicationError> {
        if self.inner.disposed.get() {
            return Ok(self.close_state());
        }
        if !matches!(*self.inner.close.borrow(), CloseState::Open) {
            return Ok(self.close_state());
        }
        if self.inner.editor.try_borrow_mut().is_err() {
            return Err(ApplicationError::Busy);
        }
        *self.inner.close.borrow_mut() = CloseState::Pending;
        self.inner.notify.set(true);
        if self.inner.schedule().is_err() {
            self.inner.scheduler_failed.set(true);
            *self.inner.close.borrow_mut() = CloseState::Failed(CloseFailure::SchedulerUnavailable);
            (self.inner.changed)();
        }
        Ok(self.close_state())
    }

    pub fn cancel_close(&self) -> Result<(), ApplicationError> {
        if self.inner.disposed.get() {
            return Err(ApplicationError::Disposed);
        }
        *self.inner.close.borrow_mut() = CloseState::Open;
        self.inner.wake();
        Ok(())
    }

    pub fn close_state(&self) -> CloseState {
        self.inner.close.borrow().clone()
    }

    pub fn dispose(&self) {
        self.inner.dispose();
    }
}

fn observe_close_failure(inner: &Inner, editor: &mut BrowserEditor) {
    let preference_failures = editor.take_preference_close_failures();
    let app_file_failures = editor.take_app_file_close_failures();
    let snippet_failures = editor.take_snippet_failures();
    if !matches!(*inner.close.borrow(), CloseState::Pending) {
        return;
    }
    if let Some(error) = editor.theme_failures().first() {
        *inner.close.borrow_mut() = CloseState::Failed(CloseFailure::Theme(error.clone()));
        inner.notify.set(true);
        return;
    }
    if let Some(error) = snippet_failures.first() {
        *inner.close.borrow_mut() = CloseState::Failed(CloseFailure::Snippet(error.clone()));
        inner.notify.set(true);
        return;
    }
    if let Some(error) = preference_failures.first() {
        *inner.close.borrow_mut() = CloseState::Failed(CloseFailure::Preference(error.clone()));
        inner.notify.set(true);
        return;
    }
    if let Some(error) = app_file_failures.first() {
        *inner.close.borrow_mut() = CloseState::Failed(CloseFailure::AppFile(error.clone()));
        inner.notify.set(true);
        return;
    }
    for event in editor.file_events() {
        let error = match event {
            FileEvent::SaveFinished {
                result: Err(error), ..
            }
            | FileEvent::ChoiceFinished {
                result: Err(error), ..
            } => error,
            _ => continue,
        };
        *inner.close.borrow_mut() = CloseState::Failed(CloseFailure::File(error.clone()));
        inner.notify.set(true);
        return;
    }
}

impl Drop for BrowserApplication {
    fn drop(&mut self) {
        self.inner.dispose();
    }
}
