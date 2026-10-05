use std::collections::{BTreeSet, VecDeque};

use serde_json::Value;

use crate::protocol::{decode_binary, BinaryFrame, RemoteRequest, TextFrame};
use crate::{REMOTE_RECONNECT_DELAY_MS, REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Connection(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Connecting,
    Open,
    Disconnected,
    Expired,
    Disposed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvokeError {
    Closed,
    SessionExpired,
    SequenceExhausted,
    Encoding(String),
}

pub struct Invocation {
    pub seq: u32,
    pub send: Option<String>,
}

pub struct Opened {
    pub queued: Vec<String>,
    pub recovered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterClose {
    ReconnectAfter(u32),
    Authenticate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closed {
    pub rejected: Vec<u32>,
    pub action: AfterClose,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponsePayload {
    Json(Value),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Delivery {
    Response {
        seq: u32,
        result: Result<ResponsePayload, Value>,
    },
    ChannelJson {
        channel_id: u32,
        index: u32,
        message: Value,
    },
    ChannelBinary {
        channel_id: u32,
        index: u32,
        bytes: Vec<u8>,
    },
    ChannelEnd {
        channel_id: u32,
        index: u32,
    },
    Event {
        event: String,
        payload: String,
    },
}

pub struct Client {
    connection: Connection,
    phase: Phase,
    next_seq: Option<u32>,
    needs_recovery: bool,
    pending: BTreeSet<u32>,
    outbox: VecDeque<String>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self {
            connection: Connection(1),
            phase: Phase::Connecting,
            next_seq: Some(1),
            needs_recovery: false,
            pending: BTreeSet::new(),
            outbox: VecDeque::new(),
        }
    }

    pub fn connection(&self) -> Connection {
        self.connection
    }

    pub fn is_open(&self) -> bool {
        self.phase == Phase::Open
    }

    pub fn reconnect(&mut self) -> Option<Connection> {
        if self.phase != Phase::Disconnected {
            return None;
        }
        self.connection = Connection(self.connection.0.checked_add(1)?);
        self.phase = Phase::Connecting;
        Some(self.connection)
    }

    pub fn invoke(&mut self, command: &str, args: Value) -> Result<Invocation, InvokeError> {
        match self.phase {
            Phase::Disposed => return Err(InvokeError::Closed),
            Phase::Expired => return Err(InvokeError::SessionExpired),
            _ => {}
        }
        let seq = self.next_seq.ok_or(InvokeError::SequenceExhausted)?;
        let message = serde_json::to_string(&RemoteRequest {
            seq,
            command: command.into(),
            args,
        })
        .map_err(|error| InvokeError::Encoding(error.to_string()))?;
        self.next_seq = seq.checked_add(1);
        self.pending.insert(seq);
        if self.phase == Phase::Open {
            return Ok(Invocation {
                seq,
                send: Some(message),
            });
        }
        self.outbox.push_back(message);
        Ok(Invocation { seq, send: None })
    }

    pub fn opened(&mut self, connection: Connection) -> Option<Opened> {
        if connection != self.connection || self.phase != Phase::Connecting {
            return None;
        }
        self.phase = Phase::Open;
        let recovered = std::mem::take(&mut self.needs_recovery);
        Some(Opened {
            queued: self.outbox.drain(..).collect(),
            recovered,
        })
    }

    pub fn closed(&mut self, connection: Connection, code: u16) -> Option<Closed> {
        if connection != self.connection || !matches!(self.phase, Phase::Connecting | Phase::Open) {
            return None;
        }
        let rejected = self.reject_all();
        self.needs_recovery = true;
        let action = if code == REMOTE_WS_CLOSE_CODE_SESSION_EXPIRED {
            self.phase = Phase::Expired;
            AfterClose::Authenticate
        } else {
            self.phase = Phase::Disconnected;
            AfterClose::ReconnectAfter(REMOTE_RECONNECT_DELAY_MS)
        };
        Some(Closed { rejected, action })
    }

    pub fn text(&mut self, connection: Connection, text: &str) -> Option<Delivery> {
        if !self.accepts(connection) {
            return None;
        }
        match serde_json::from_str::<TextFrame>(text).ok()? {
            TextFrame::Response { seq, ok, payload } => {
                if !self.pending.remove(&seq) {
                    return None;
                }
                let result = if ok {
                    Ok(ResponsePayload::Json(payload))
                } else {
                    Err(payload)
                };
                Some(Delivery::Response { seq, result })
            }
            TextFrame::Channel {
                channel_id,
                index,
                message,
            } => Some(Delivery::ChannelJson {
                channel_id,
                index,
                message,
            }),
            TextFrame::ChannelEnd { channel_id, index } => {
                Some(Delivery::ChannelEnd { channel_id, index })
            }
            TextFrame::Event { event, payload } => Some(Delivery::Event { event, payload }),
        }
    }

    pub fn binary(&mut self, connection: Connection, frame: &[u8]) -> Option<Delivery> {
        if !self.accepts(connection) {
            return None;
        }
        match decode_binary(frame)? {
            BinaryFrame::Response { seq, bytes } => {
                if !self.pending.remove(&seq) {
                    return None;
                }
                Some(Delivery::Response {
                    seq,
                    result: Ok(ResponsePayload::Binary(bytes.into())),
                })
            }
            BinaryFrame::Channel {
                channel_id,
                index,
                bytes,
            } => Some(Delivery::ChannelBinary {
                channel_id,
                index,
                bytes: bytes.into(),
            }),
        }
    }

    pub fn dispose(&mut self) -> Vec<u32> {
        self.phase = Phase::Disposed;
        self.needs_recovery = false;
        self.reject_all()
    }

    fn accepts(&self, connection: Connection) -> bool {
        self.connection == connection && self.phase == Phase::Open
    }

    fn reject_all(&mut self) -> Vec<u32> {
        self.outbox.clear();
        std::mem::take(&mut self.pending).into_iter().collect()
    }
}

#[cfg(test)]
#[path = "client-tests.rs"]
mod tests;
