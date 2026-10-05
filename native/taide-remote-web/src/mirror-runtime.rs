use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{Event, Performance, Window};

use crate::mirror_writes::MirrorError;

const MS_PER_SECOND: f64 = 1_000.0;
const BLUR_EVENT: &str = "blur";

struct Timer {
    window: Window,
    id: i32,
    deadline: Duration,
    fired: Rc<Cell<bool>>,
    _callback: Closure<dyn FnMut()>,
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.window.clear_timeout_with_handle(self.id);
    }
}

pub(crate) struct MirrorRuntime {
    window: Window,
    performance: Performance,
    wake: Rc<dyn Fn()>,
    timer: Option<Timer>,
    flush: Rc<Cell<bool>>,
    blur: Closure<dyn FnMut(Event)>,
}

impl MirrorRuntime {
    pub(crate) fn new(wake: Rc<dyn Fn()>) -> Result<Self, MirrorError> {
        let window = web_sys::window().ok_or(MirrorError::TimerUnavailable)?;
        let performance = window.performance().ok_or(MirrorError::TimerUnavailable)?;
        let flush = Rc::new(Cell::new(false));
        let requested = Rc::clone(&flush);
        let notify = Rc::clone(&wake);
        let blur = Closure::new(move |_: Event| {
            requested.set(true);
            notify();
        });
        window
            .add_event_listener_with_callback(BLUR_EVENT, blur.as_ref().unchecked_ref())
            .map_err(|_| MirrorError::TimerUnavailable)?;
        Ok(Self {
            window,
            performance,
            wake,
            timer: None,
            flush,
            blur,
        })
    }

    pub(crate) fn now(&self) -> Result<Duration, MirrorError> {
        Duration::try_from_secs_f64(self.performance.now() / MS_PER_SECOND)
            .map_err(|_| MirrorError::TimerUnavailable)
    }

    pub(crate) fn take_flush(&self) -> bool {
        self.flush.replace(false)
    }

    pub(crate) fn is_armed(&self) -> bool {
        self.timer.as_ref().is_some_and(|timer| !timer.fired.get())
    }

    pub(crate) fn arm(&mut self, deadline: Option<Duration>) -> Result<(), MirrorError> {
        let Some(deadline) = deadline else {
            self.timer = None;
            return Ok(());
        };
        if self
            .timer
            .as_ref()
            .is_some_and(|timer| timer.deadline == deadline && !timer.fired.get())
        {
            return Ok(());
        }
        self.timer = None;
        let remaining = deadline.saturating_sub(self.now()?);
        let delay = i32::try_from(remaining.as_millis().saturating_add(1)).unwrap_or(i32::MAX);
        let fired = Rc::new(Cell::new(false));
        let observed = Rc::clone(&fired);
        let wake = Rc::clone(&self.wake);
        let callback = Closure::new(move || {
            observed.set(true);
            wake();
        });
        let id = self
            .window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                delay,
            )
            .map_err(|_| MirrorError::TimerUnavailable)?;
        self.timer = Some(Timer {
            window: self.window.clone(),
            id,
            deadline,
            fired,
            _callback: callback,
        });
        Ok(())
    }
}

impl Drop for MirrorRuntime {
    fn drop(&mut self) {
        self.timer = None;
        let _ = self
            .window
            .remove_event_listener_with_callback(BLUR_EVENT, self.blur.as_ref().unchecked_ref());
    }
}
