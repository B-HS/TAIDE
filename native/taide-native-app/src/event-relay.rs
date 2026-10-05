use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use serde_json::json;
use taide_git::store::GitStore;
use taide_model::app_event::AppEvent;
use taide_remote::store::RemoteStore;
use taide_runtime::EventSink;

pub struct GitEvents {
    store: GitStore,
    enabled: AtomicBool,
}

impl GitEvents {
    pub fn new(store: GitStore) -> Self {
        Self {
            store,
            enabled: AtomicBool::new(false),
        }
    }

    pub fn activate(&self) {
        self.enabled.store(true, Ordering::Release);
    }

    fn record(&self, event: &AppEvent) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        match event {
            AppEvent::FsChanged { project_id, .. }
            | AppEvent::GitStatusChanged { project_id }
            | AppEvent::GitRefsChanged { project_id } => self.store.invalidate_status(project_id),
            _ => {}
        }
    }
}

pub struct Relay {
    forward: Arc<dyn EventSink>,
    remote: RemoteStore,
    git: Arc<GitEvents>,
}

impl Relay {
    pub fn new(forward: Arc<dyn EventSink>, remote: RemoteStore, git: Arc<GitEvents>) -> Self {
        Self {
            forward,
            remote,
            git,
        }
    }
}

impl EventSink for Relay {
    fn publish(&self, event: AppEvent) {
        self.git.record(&event);
        if self.remote.has_event_subscribers()
            && let Some(frame) = frame(&event)
        {
            self.remote.broadcast_event(frame);
        }
        self.forward.publish(event);
    }
}

fn frame(event: &AppEvent) -> Option<String> {
    let (name, payload) = match event {
        AppEvent::ProjectOpened { project } => ("project:opened", json!({"project": project})),
        AppEvent::ProjectClosed { project_id } => {
            ("project:closed", json!({"projectId": project_id}))
        }
        AppEvent::ProjectActivated { project_id } => {
            ("project:activated", json!({"projectId": project_id}))
        }
        AppEvent::ProjectListChanged { projects } => {
            ("project:list-changed", json!({"projects": projects}))
        }
        AppEvent::ProjectGroupsChanged { groups } => {
            ("project:groups-changed", json!({"groups": groups}))
        }
        AppEvent::ProjectRecentCleared {
            removed,
            skipped_with_drafts,
        } => (
            "project:recent-cleared",
            json!({"removed": removed, "skippedWithDrafts": skipped_with_drafts}),
        ),
        AppEvent::SessionShellSlotsChanged { tree, focused } => (
            "session:shell-slots-changed",
            json!({"tree": tree, "focused": focused}),
        ),
        AppEvent::WindowChromeChanged { chrome } => {
            ("session:window-chrome-changed", json!({"chrome": chrome}))
        }
        AppEvent::LayoutChanged {
            project_id,
            revision,
        } => (
            "layout:changed",
            json!({"projectId": project_id, "revision": revision}),
        ),
        AppEvent::ThemeChanged { theme_id } => ("theme:changed", json!({"themeId": theme_id})),
        AppEvent::FsChanged { project_id, change } => (
            "fs:changed",
            json!({"projectId": project_id, "change": change}),
        ),
        AppEvent::FsRescanRequired { project_id } => {
            ("fs:rescan-required", json!({"projectId": project_id}))
        }
        AppEvent::TerminalSpawned {
            session_id,
            project_id,
            cwd,
            shell,
        } => (
            "terminal:spawned",
            json!({"sessionId": session_id, "projectId": project_id, "cwd": cwd, "shell": shell}),
        ),
        AppEvent::TerminalExited { session_id, code } => (
            "terminal:exited",
            json!({"sessionId": session_id, "code": code}),
        ),
        AppEvent::TerminalCwdChanged { session_id, cwd } => (
            "terminal:cwd-changed",
            json!({"sessionId": session_id, "cwd": cwd}),
        ),
        AppEvent::TerminalCommandFinished {
            session_id,
            cwd,
            exit_code,
            duration_ms,
        } => (
            "terminal:command-finished",
            json!({"sessionId": session_id, "cwd": cwd, "exitCode": exit_code, "durationMs": duration_ms}),
        ),
        AppEvent::GitStatusChanged { project_id } => {
            ("git:status-changed", json!({"projectId": project_id}))
        }
        AppEvent::GitRefsChanged { project_id } => {
            ("git:refs-changed", json!({"projectId": project_id}))
        }
        AppEvent::LspSessionStatusChanged {
            session_id,
            status,
            last_error,
            generation,
        } => (
            "lsp:session-status-changed",
            json!({"sessionId": session_id, "status": status, "lastError": last_error, "generation": generation}),
        ),
        AppEvent::LspInstallProgress {
            server_id,
            phase,
            received_bytes,
            total_bytes,
            message,
        } => (
            "lsp:install-progress",
            json!({"serverId": server_id, "phase": phase, "receivedBytes": received_bytes, "totalBytes": total_bytes, "message": message}),
        ),
        AppEvent::AgentStateChanged { project_id, agents } => (
            "agent:state-changed",
            json!({"projectId": project_id, "agents": agents}),
        ),
        AppEvent::IdeStatusChanged { status } => ("ide:status-changed", json!({"status": status})),
        AppEvent::IdeDiffRequested {
            request_id,
            project_id,
            old_path,
            new_path,
            new_contents,
            tab_name,
        } => (
            "ide:diff-requested",
            json!({"requestId": request_id, "projectId": project_id, "oldPath": old_path, "newPath": new_path, "newContents": new_contents, "tabName": tab_name}),
        ),
        AppEvent::IdeSaveRequested {
            request_id,
            project_id,
            path,
        } => (
            "ide:save-requested",
            json!({"requestId": request_id, "projectId": project_id, "path": path}),
        ),
        AppEvent::IdeCloseTabRequested {
            tab_name,
            request_id,
        } => (
            "ide:close-tab-requested",
            json!({"tabName": tab_name, "requestId": request_id}),
        ),
        AppEvent::SyncStateChanged { status } => ("sync:state-changed", json!({"status": status})),
        AppEvent::RemoteStateChanged { status } => {
            ("remote:state-changed", json!({"status": status}))
        }
        AppEvent::SettingsChanged { settings } => {
            ("settings:changed", json!({"settings": settings}))
        }
        AppEvent::AgentExternalOpen { .. } | AppEvent::HotExitFlushRequested { .. } => return None,
    };
    Some(json!({ "t": "event", "event": name, "payload": payload.to_string() }).to_string())
}

#[cfg(test)]
#[path = "event-relay-tests.rs"]
mod tests;
