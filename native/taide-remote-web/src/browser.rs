use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use js_sys::{ArrayBuffer, Uint8Array};
use serde_json::Value;
use taide_remote_wire::REMOTE_LOGIN_PATH;
use taide_remote_wire::client::{AfterClose, Client, Connection, Delivery, InvokeError};
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{BinaryType, CloseEvent, Event, MessageEvent, Url, WebSocket, Window};

const SOCKET_PATH: &str = "/__taide/ws";
const ABNORMAL_CLOSE: u16 = 1006;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserError {
    WindowUnavailable,
    OriginUnavailable,
    UnsupportedProtocol,
    SocketUnavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BrowserEvent {
    Connected { recovered: bool },
    Frame(Delivery),
    Disconnected { rejected: Vec<u32> },
    AuthenticationRequired,
    NavigationFailed,
    ReconnectUnavailable,
}

struct Socket {
    web: WebSocket,
    handlers: Vec<Closure<dyn FnMut(Event)>>,
}

impl Drop for Socket {
    fn drop(&mut self) {
        self.web.set_onopen(None);
        self.web.set_onmessage(None);
        self.web.set_onclose(None);
        self.web.set_onerror(None);
        self.handlers.clear();
        let _ = self.web.close();
    }
}

struct Timer {
    window: Window,
    id: i32,
    _callback: Closure<dyn FnMut()>,
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.window.clear_timeout_with_handle(self.id);
    }
}

struct Inner {
    core: Client,
    window: Window,
    url: String,
    socket: Option<Socket>,
    timer: Option<Timer>,
    events: VecDeque<BrowserEvent>,
    wake: Rc<dyn Fn()>,
}

pub struct BrowserClient {
    inner: Rc<RefCell<Inner>>,
}

impl BrowserClient {
    pub fn new(wake: Rc<dyn Fn()>) -> Result<Self, BrowserError> {
        let window = web_sys::window().ok_or(BrowserError::WindowUnavailable)?;
        let location = window.location();
        let base = location
            .href()
            .map_err(|_| BrowserError::OriginUnavailable)?;
        let url =
            Url::new_with_base(SOCKET_PATH, &base).map_err(|_| BrowserError::OriginUnavailable)?;
        match url.protocol().as_str() {
            "http:" => url.set_protocol("ws:"),
            "https:" => url.set_protocol("wss:"),
            _ => return Err(BrowserError::UnsupportedProtocol),
        }
        let inner = Rc::new(RefCell::new(Inner {
            core: Client::new(),
            window,
            url: url.href(),
            socket: None,
            timer: None,
            events: VecDeque::new(),
            wake,
        }));
        let connection = inner.borrow().core.connection();
        connect(&inner, connection)?;
        Ok(Self { inner })
    }

    pub fn invoke(&self, command: &str, args: Value) -> Result<u32, InvokeError> {
        let (invocation, connection, web) = {
            let mut inner = self.inner.borrow_mut();
            let invocation = inner.core.invoke(command, args)?;
            (
                invocation,
                inner.core.connection(),
                inner.socket.as_ref().map(|socket| socket.web.clone()),
            )
        };
        if let Some(message) = invocation.send {
            let sent = web.is_some_and(|web| web.send_with_str(&message).is_ok());
            if !sent {
                close_connection(&self.inner, connection, ABNORMAL_CLOSE);
            }
        }
        Ok(invocation.seq)
    }

    pub fn next_event(&self) -> Option<BrowserEvent> {
        self.inner.borrow_mut().events.pop_front()
    }

    pub fn is_open(&self) -> bool {
        self.inner.borrow().core.is_open()
    }

    pub fn dispose(&self) {
        let rejected = {
            let mut inner = self.inner.borrow_mut();
            let rejected = inner.core.dispose();
            inner.timer = None;
            inner.socket = None;
            rejected
        };
        if !rejected.is_empty() {
            publish(&self.inner, BrowserEvent::Disconnected { rejected });
        }
    }
}

impl Drop for BrowserClient {
    fn drop(&mut self) {
        let mut inner = self.inner.borrow_mut();
        inner.core.dispose();
        inner.timer = None;
        inner.socket = None;
    }
}

fn publish(inner: &Rc<RefCell<Inner>>, event: BrowserEvent) {
    let wake = {
        let mut inner = inner.borrow_mut();
        inner.events.push_back(event);
        inner.wake.clone()
    };
    wake();
}

fn connect(inner: &Rc<RefCell<Inner>>, connection: Connection) -> Result<(), BrowserError> {
    let web = WebSocket::new(&inner.borrow().url).map_err(|_| BrowserError::SocketUnavailable)?;
    web.set_binary_type(BinaryType::Arraybuffer);

    let weak = Rc::downgrade(inner);
    let opened = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Some(inner) = weak.upgrade() else { return };
        let (opened, web) = {
            let mut state = inner.borrow_mut();
            let Some(opened) = state.core.opened(connection) else {
                return;
            };
            (
                opened,
                state.socket.as_ref().map(|socket| socket.web.clone()),
            )
        };
        let Some(web) = web else {
            close_connection(&inner, connection, ABNORMAL_CLOSE);
            return;
        };
        for message in opened.queued {
            if web.send_with_str(&message).is_err() {
                close_connection(&inner, connection, ABNORMAL_CLOSE);
                return;
            }
        }
        publish(
            &inner,
            BrowserEvent::Connected {
                recovered: opened.recovered,
            },
        );
    });

    let weak = Rc::downgrade(inner);
    let message = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        let Some(inner) = weak.upgrade() else { return };
        let Ok(event) = event.dyn_into::<MessageEvent>() else {
            return;
        };
        let data = event.data();
        let delivery = if let Some(text) = data.as_string() {
            inner.borrow_mut().core.text(connection, &text)
        } else if let Ok(buffer) = data.dyn_into::<ArrayBuffer>() {
            let bytes = Uint8Array::new(&buffer).to_vec();
            inner.borrow_mut().core.binary(connection, &bytes)
        } else {
            None
        };
        if let Some(delivery) = delivery {
            publish(&inner, BrowserEvent::Frame(delivery));
        }
    });

    let weak = Rc::downgrade(inner);
    let closed = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        let Some(inner) = weak.upgrade() else { return };
        let Ok(event) = event.dyn_into::<CloseEvent>() else {
            return;
        };
        close_connection(&inner, connection, event.code());
    });

    let weak = Rc::downgrade(inner);
    let error = Closure::<dyn FnMut(Event)>::new(move |_| {
        let Some(inner) = weak.upgrade() else { return };
        let web = {
            let state = inner.borrow();
            if state.core.connection() != connection {
                return;
            }
            state.socket.as_ref().map(|socket| socket.web.clone())
        };
        if let Some(web) = web {
            let _ = web.close();
        }
    });

    web.set_onopen(Some(opened.as_ref().unchecked_ref()));
    web.set_onmessage(Some(message.as_ref().unchecked_ref()));
    web.set_onclose(Some(closed.as_ref().unchecked_ref()));
    web.set_onerror(Some(error.as_ref().unchecked_ref()));
    let socket = Socket {
        web,
        handlers: vec![opened, message, closed, error],
    };
    let old = inner.borrow_mut().socket.replace(socket);
    drop(old);
    Ok(())
}

fn close_connection(inner: &Rc<RefCell<Inner>>, connection: Connection, code: u16) {
    let (closed, window, web) = {
        let mut state = inner.borrow_mut();
        let Some(closed) = state.core.closed(connection, code) else {
            return;
        };
        (
            closed,
            state.window.clone(),
            state.socket.as_ref().map(|socket| socket.web.clone()),
        )
    };
    if let Some(web) = web {
        let _ = web.close();
    }
    let follow_up = match closed.action {
        AfterClose::Authenticate => {
            let navigation_failed = window.location().assign(REMOTE_LOGIN_PATH).is_err();
            Some((BrowserEvent::AuthenticationRequired, navigation_failed))
        }
        AfterClose::ReconnectAfter(delay) => {
            if schedule_reconnect(inner, delay).is_err() {
                inner.borrow_mut().core.dispose();
                Some((BrowserEvent::ReconnectUnavailable, false))
            } else {
                None
            }
        }
    };
    publish(
        inner,
        BrowserEvent::Disconnected {
            rejected: closed.rejected,
        },
    );
    if let Some((event, navigation_failed)) = follow_up {
        publish(inner, event);
        if navigation_failed {
            publish(inner, BrowserEvent::NavigationFailed);
        }
    }
}

fn schedule_reconnect(inner: &Rc<RefCell<Inner>>, delay: u32) -> Result<(), ()> {
    let window = inner.borrow().window.clone();
    let weak = Rc::downgrade(inner);
    let callback = Closure::<dyn FnMut()>::new(move || {
        let Some(inner) = weak.upgrade() else { return };
        let connection = inner.borrow_mut().core.reconnect();
        let Some(connection) = connection else { return };
        if connect(&inner, connection).is_err() {
            close_connection(&inner, connection, ABNORMAL_CLOSE);
        }
    });
    let id = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            i32::try_from(delay).map_err(|_| ())?,
        )
        .map_err(|_| ())?;
    let timer = Timer {
        window,
        id,
        _callback: callback,
    };
    let old = inner.borrow_mut().timer.replace(timer);
    drop(old);
    Ok(())
}
