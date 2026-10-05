use std::cell::Cell;
use std::rc::Rc;

use serde_json::json;
use taide_remote_web::{BrowserClient, BrowserEvent, Delivery, ResponsePayload};
use wasm_bindgen::prelude::*;

#[path = "shell-probe.rs"]
mod shell_probe;

#[path = "presentation-probe.rs"]
mod presentation_probe;

#[path = "file-probe.rs"]
mod file_probe;

#[path = "application-probe.rs"]
mod application_probe;

#[cfg(feature = "canvas")]
#[path = "canvas-probe.rs"]
mod canvas_probe;

#[cfg(feature = "canvas")]
#[path = "settings-probe.rs"]
mod settings_probe;

#[wasm_bindgen]
pub struct Probe {
    client: BrowserClient,
    wakes: Rc<Cell<u32>>,
}

#[wasm_bindgen]
impl Probe {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Self, JsValue> {
        let wakes = Rc::new(Cell::new(0u32));
        let observed = wakes.clone();
        let client = BrowserClient::new(Rc::new(move || {
            observed.set(observed.get().saturating_add(1));
        }))
        .map_err(|error| JsValue::from_str(&format!("{error:?}")))?;
        Ok(Self { client, wakes })
    }

    pub fn invoke(&self, command: &str, args: &str) -> Result<u32, JsValue> {
        let args = serde_json::from_str(args).map_err(|_| JsValue::from_str("Invalid JSON"))?;
        self.client
            .invoke(command, args)
            .map_err(|error| JsValue::from_str(&format!("{error:?}")))
    }

    pub fn wakes(&self) -> u32 {
        self.wakes.get()
    }

    pub fn dispose(&self) {
        self.client.dispose();
    }

    pub fn poll(&self) -> Option<String> {
        let event = match self.client.next_event()? {
            BrowserEvent::Connected { recovered } => {
                json!({"kind":"connected", "recovered":recovered})
            }
            BrowserEvent::Disconnected { rejected } => {
                json!({"kind":"disconnected", "rejected":rejected})
            }
            BrowserEvent::AuthenticationRequired => json!({"kind":"authenticate"}),
            BrowserEvent::NavigationFailed => json!({"kind":"navigation-failed"}),
            BrowserEvent::ReconnectUnavailable => json!({"kind":"reconnect-unavailable"}),
            BrowserEvent::Frame(frame) => match frame {
                Delivery::Response { seq, result } => match result {
                    Ok(ResponsePayload::Json(payload)) => {
                        json!({"kind":"response", "seq":seq, "payload":payload})
                    }
                    Ok(ResponsePayload::Binary(bytes)) => {
                        json!({"kind":"binary", "seq":seq, "payload":bytes})
                    }
                    Err(payload) => json!({"kind":"error", "seq":seq, "payload":payload}),
                },
                Delivery::ChannelJson {
                    channel_id,
                    index,
                    message,
                } => {
                    json!({"kind":"channel-json", "channelId":channel_id, "index":index, "payload":message})
                }
                Delivery::ChannelBinary {
                    channel_id,
                    index,
                    bytes,
                } => {
                    json!({"kind":"channel-binary", "channelId":channel_id, "index":index, "payload":bytes})
                }
                Delivery::ChannelEnd { channel_id, index } => {
                    json!({"kind":"channel-end", "channelId":channel_id, "index":index})
                }
                Delivery::Event { event, payload } => {
                    json!({"kind":"event", "event":event, "payload":payload})
                }
            },
        };
        Some(event.to_string())
    }
}
