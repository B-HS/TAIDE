use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use taide_model::ids::ProjectId;
use taide_model::terminal::TerminalSession;

/// Holds a terminal session's mutable directory and running state independently of its PTY resource.
pub struct TerminalSessionMetadata {
    project_id: ProjectId,
    cwd: Mutex<String>,
    shell: String,
    running: AtomicBool,
}

impl TerminalSessionMetadata {
    pub fn new(project_id: ProjectId, cwd: String, shell: String) -> Self {
        Self { project_id, cwd: Mutex::new(cwd), shell, running: AtomicBool::new(true) }
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn cwd(&self) -> String {
        self.cwd.lock().clone()
    }

    pub fn shell(&self) -> &str {
        &self.shell
    }

    pub fn update_cwd(&self, cwd: String) -> bool {
        let mut current = self.cwd.lock();
        if *current == cwd {
            return false;
        }
        *current = cwd;
        true
    }

    pub fn mark_exited(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn snapshot(&self, session_id: &str) -> TerminalSession {
        TerminalSession {
            id: session_id.to_string(),
            project_id: self.project_id.clone(),
            cwd: self.cwd(),
            shell: self.shell.clone(),
            running: self.running.load(Ordering::SeqCst),
        }
    }
}
