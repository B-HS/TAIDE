use std::rc::Rc;

use serde_json::Value;
use taide_native_ui::commands::ShellMutation;
use taide_native_ui::snapshot::ShellSnapshot;

use crate::shell::{ShellState, mutation_call};
use crate::{BrowserClient, BrowserError, BrowserEvent, Delivery, InvokeError};

pub struct BrowserShell {
    client: BrowserClient,
    state: ShellState,
    connected: bool,
}

impl BrowserShell {
    pub fn new(wake: Rc<dyn Fn()>) -> Result<Self, BrowserError> {
        Ok(Self {
            client: BrowserClient::new(wake)?,
            state: ShellState::default(),
            connected: false,
        })
    }

    pub fn snapshot(&self) -> Option<&ShellSnapshot> {
        self.state.snapshot()
    }

    pub fn state(&self) -> &ShellState {
        &self.state
    }

    pub(crate) fn settings_updated(
        &mut self,
        settings: taide_model::settings::Settings,
        generation: u64,
    ) {
        self.state.settings_updated(settings, generation);
    }

    pub fn invoke(&self, command: &str, args: Value) -> Result<u32, InvokeError> {
        self.client.invoke(command, args)
    }

    pub(crate) fn layout_updated(&mut self, opened: crate::app_file_opens::Opened) {
        self.state.layout_updated(opened.project, opened.layout);
    }

    pub fn is_connected(&self) -> bool {
        self.connected && self.client.is_open()
    }

    pub fn mutate(&self, mutation: &ShellMutation) -> Result<u32, InvokeError> {
        let call = mutation_call(mutation);
        self.invoke(call.command, call.args)
    }

    pub fn refresh(&mut self) {
        self.state.refresh();
    }

    pub fn poll(&mut self) -> Vec<BrowserEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.client.next_event() {
            match &event {
                BrowserEvent::Connected { .. } => {
                    self.connected = true;
                    self.state.refresh();
                }
                BrowserEvent::Disconnected { .. } | BrowserEvent::AuthenticationRequired => {
                    self.connected = false;
                    self.state.disconnected();
                }
                BrowserEvent::Frame(Delivery::Response { seq, result }) => {
                    if self.state.response(*seq, result) {
                        continue;
                    }
                }
                BrowserEvent::Frame(Delivery::Event { event, payload }) => {
                    self.state.event(event, payload);
                }
                _ => {}
            }
            events.push(event);
        }
        if self.connected {
            for read in self.state.next_reads() {
                if !self.client.is_open() {
                    self.connected = false;
                    self.state.disconnected();
                    break;
                }
                let call = read.call();
                match self.client.invoke(call.command, call.args) {
                    Ok(seq) => self.state.sent(read, seq),
                    Err(error) => self.state.invocation_failed(read, error),
                }
            }
        }
        events
    }

    pub fn dispose(&mut self) {
        self.connected = false;
        self.state.disconnected();
        self.client.dispose();
    }
}
