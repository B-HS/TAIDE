use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

use parking_lot::Mutex;
use taide_infra::pty::PtySession;
use taide_model::error::{AppError, AppResult};
use taide_model::ids::ProjectId;
use taide_model::terminal::{PtyAttachResult, TerminalSession};

use crate::metadata::TerminalSessionMetadata;
use crate::session::TerminalSessionOutput;

/// Couples one PTY resource with its metadata and output stream.
pub struct TerminalSessionEntry {
    pty: PtySession,
    metadata: Arc<TerminalSessionMetadata>,
    output: Arc<TerminalSessionOutput>,
}

impl TerminalSessionEntry {
    pub fn new(pty: PtySession, metadata: Arc<TerminalSessionMetadata>, output: Arc<TerminalSessionOutput>) -> Self {
        Self { pty, metadata, output }
    }
}

/// Serializes terminal session lookup, resource operations, and project-scoped cleanup.
#[derive(Default)]
pub struct TerminalStore(Mutex<HashMap<String, TerminalSessionEntry>>);

impl TerminalStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, session_id: String, entry: TerminalSessionEntry) {
        self.0.lock().insert(session_id, entry);
    }

    pub fn cwd(&self, session_id: &str) -> Option<String> {
        self.0.lock().get(session_id).map(|entry| entry.metadata.cwd())
    }

    pub fn update_cwd(&self, session_id: &str, cwd: String) -> bool {
        self.0.lock().get(session_id).is_some_and(|entry| entry.metadata.update_cwd(cwd))
    }

    pub fn writer_handle(&self, session_id: &str) -> AppResult<Arc<Mutex<Box<dyn Write + Send>>>> {
        let sessions = self.0.lock();
        Ok(find_entry(&sessions, session_id)?.pty.writer_handle())
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> AppResult<()> {
        let sessions = self.0.lock();
        find_entry(&sessions, session_id)?.pty.resize(cols, rows)
    }

    pub fn kill(&self, session_id: &str) -> AppResult<()> {
        let removed = self.0.lock().remove(session_id);
        match removed {
            Some(entry) => entry.pty.kill(),
            None => Err(AppError::NotFound(format!("terminal session not found: {session_id}"))),
        }
    }

    pub fn set_paused(&self, session_id: &str, paused: bool) -> AppResult<()> {
        let sessions = self.0.lock();
        find_entry(&sessions, session_id)?.pty.set_paused(paused);
        Ok(())
    }

    pub fn attach(&self, session_id: &str, sink: impl Fn(&[u8]) -> bool + Send + Sync + 'static) -> AppResult<PtyAttachResult> {
        let sessions = self.0.lock();
        Ok(find_entry(&sessions, session_id)?.output.attach(sink))
    }

    pub fn detach(&self, session_id: &str, subscription_id: u32) -> AppResult<()> {
        let sessions = self.0.lock();
        if let Some(entry) = sessions.get(session_id) {
            entry.output.detach(subscription_id);
        }
        Ok(())
    }

    pub fn sessions_for_project(&self, project_id: &ProjectId) -> Vec<TerminalSession> {
        self.0
            .lock()
            .iter()
            .filter(|(_, entry)| entry.metadata.project_id() == project_id)
            .map(|(session_id, entry)| entry.metadata.snapshot(session_id))
            .collect()
    }

    pub fn foreground_pids(&self, project_id: &ProjectId) -> Vec<(String, u32)> {
        self.0
            .lock()
            .iter()
            .filter(|(_, entry)| entry.metadata.project_id() == project_id)
            .filter_map(|(session_id, entry)| entry.pty.foreground_pid().map(|pid| (session_id.clone(), pid)))
            .collect()
    }

    pub fn kill_all(&self) {
        for entry in self.0.lock().values() {
            let _ = entry.pty.kill();
        }
    }

    pub fn kill_project(&self, project_id: &ProjectId) {
        let mut sessions = self.0.lock();
        let session_ids: Vec<String> =
            sessions.iter().filter(|(_, entry)| entry.metadata.project_id() == project_id).map(|(session_id, _)| session_id.clone()).collect();

        for session_id in session_ids {
            if let Some(entry) = sessions.remove(&session_id) {
                let _ = entry.pty.kill();
            }
        }
    }

    pub fn kill_session(&self, session_id: &str) {
        if let Some(entry) = self.0.lock().remove(session_id) {
            let _ = entry.pty.kill();
        }
    }
}

fn find_entry<'a>(sessions: &'a HashMap<String, TerminalSessionEntry>, session_id: &str) -> AppResult<&'a TerminalSessionEntry> {
    sessions.get(session_id).ok_or_else(|| AppError::NotFound(format!("terminal session not found: {session_id}")))
}
