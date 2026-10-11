use std::collections::HashMap;

use eframe::egui::{Context as EguiContext, Ui};
use taide_model::file::FileSizeTier;
use taide_model::ids::ProjectId;
use taide_model::locale::ResolvedLocale;
use taide_native_editor::document::EditorError;
use taide_native_editor::editing::ClipboardText;
use taide_native_editor::store::EditorStore;
use taide_native_editor::view::{ViewId, ViewKey};
use taide_native_ui::editor_context_menu::{self as menu, Action};
use taide_native_ui::keymap::catalog::{Overrides, Row};
use taide_native_ui::shell::WindowScope;
use tokio::sync::watch;
use uuid::Uuid;

pub(crate) struct Environment<'a> {
    pub(crate) project: Option<&'a ProjectId>,
    pub(crate) lsp: Option<&'a crate::lsp::LspBridge>,
    pub(crate) locale: Option<&'a ResolvedLocale>,
    pub(crate) overrides: Option<&'a str>,
    pub(crate) scope: &'a WindowScope,
    pub(crate) embedded: bool,
}

#[derive(Clone)]
pub struct Request {
    pub(crate) invocation: menu::Invocation,
    pub(crate) context: EguiContext,
    pub(crate) project: Option<ProjectId>,
    pub(crate) scope: WindowScope,
    pub(crate) owner_key: ViewKey,
}

pub struct TerminalRequest {
    pub(crate) target: Request,
    pub(crate) payload: String,
    pub(crate) title: String,
}

pub struct TerminalDelivery {
    pub(crate) project: ProjectId,
    scope: WindowScope,
    pane: taide_model::ids::PaneId,
    pub(crate) tab: taide_model::ids::TabId,
    pub(crate) payload: String,
}

impl TerminalDelivery {
    pub(crate) fn session(&self, state: &taide_runtime::AppState) -> Option<String> {
        if state.is_shutting_down() {
            return None;
        }
        let layouts = state.layouts.read();
        let layout = layouts.get(&self.project)?;
        let (root, _) = crate::symbol_sidebar::window_tree(&self.project, layout, &self.scope)?;
        let taide_model::layout::PaneNode::Leaf { tabs, .. } =
            taide_layout::service::find_leaf(root, &self.pane)?
        else {
            return None;
        };
        let tab = tabs.iter().find(|tab| tab.id == self.tab)?;
        let taide_model::layout::TabKind::Terminal { session_id, .. } = &tab.kind else {
            return None;
        };
        Some(session_id.clone())
    }
}

pub(crate) async fn terminal_target(
    services: &taide_runtime::AppServices,
    request: TerminalRequest,
) -> taide_model::error::AppResult<TerminalDelivery> {
    use taide_model::error::AppError;
    use taide_model::layout::{PaneNode, Tab, TabKind};
    let _guard = services.state.begin_mutation().await;
    if services.state.is_shutting_down() || !request.target.is_active(&services.state) {
        return Err(AppError::NotFound(
            "native editor terminal source is no longer active".into(),
        ));
    }
    let project = request
        .target
        .project
        .ok_or_else(|| AppError::NotFound("native editor terminal project is missing".into()))?;
    let pane = request.target.owner_key.pane;
    let mut layouts = services.state.layouts.read().clone();
    let layout = layouts
        .get_mut(&project)
        .ok_or_else(|| AppError::NotFound("native editor terminal layout is missing".into()))?;
    let existing = {
        let (root, _) = crate::symbol_sidebar::window_tree(&project, layout, &request.target.scope)
            .ok_or_else(|| AppError::NotFound("native editor terminal window is missing".into()))?;
        let Some(PaneNode::Leaf { tabs, active, .. }) =
            taide_layout::service::find_leaf(root, &pane)
        else {
            return Err(AppError::NotFound(
                "native editor terminal pane is missing".into(),
            ));
        };
        tabs.iter()
            .find(|tab| {
                Some(&tab.id) == active.as_ref() && matches!(tab.kind, TabKind::Terminal { .. })
            })
            .or_else(|| {
                tabs.iter()
                    .find(|tab| matches!(tab.kind, TabKind::Terminal { .. }))
            })
            .map(|tab| tab.id.clone())
    };
    let tab = if let Some(tab) = existing {
        taide_layout::service::activate_tab(layout, &tab)?;
        tab
    } else {
        let tab = Tab {
            id: taide_model::ids::TabId::new(),
            kind: TabKind::Terminal {
                session_id: String::new(),
                cwd: None,
            },
            title: request.title,
            pinned: false,
            preview: false,
            dirty: false,
            view_state: None,
        };
        taide_layout::service::open_tab(layout, &pane, tab, false)?
    };
    taide_runtime::layout_actions::finish_mutation(
        services.events.as_ref(),
        &services.state,
        &project,
        layout,
    );
    *services.state.layouts.write() = layouts;
    Ok(TerminalDelivery {
        project,
        scope: request.target.scope,
        pane,
        tab,
        payload: request.payload,
    })
}

impl Request {
    pub(crate) fn matches(&self, store: &EditorStore) -> bool {
        let session = &self.invocation.session;
        session.matches(store, session.source, session.owner, session.viewport)
    }

    pub(crate) fn is_active(&self, state: &taide_runtime::AppState) -> bool {
        let Some(project) = &self.project else {
            return true;
        };
        state.layouts.read().get(project).is_some_and(|layout| {
            crate::symbol_sidebar::window_tree(project, layout, &self.scope).is_some_and(
                |(root, _)| {
                    taide_layout::service::find_leaf(root, &self.owner_key.pane).is_some_and(
                        |leaf| match leaf {
                            taide_model::layout::PaneNode::Leaf { active, .. } => {
                                active.as_ref() == Some(&self.owner_key.tab)
                            }
                            _ => false,
                        },
                    )
                },
            )
        })
    }
}

fn conditions(
    store: &EditorStore,
    source: ViewId,
    environment: &Environment<'_>,
) -> Result<menu::Context, EditorError> {
    let view = store.views().get(source).ok_or(EditorError::NotFound)?;
    let snapshot = store.documents().snapshot(view.document)?;
    let mut context = menu::Context {
        read_only: snapshot.metadata.read_only,
        embedded: environment.embedded,
        has_selection: view
            .selection
            .selections
            .iter()
            .any(|selection| selection.anchor != selection.head),
        ..Default::default()
    };
    if snapshot.metadata.tier != FileSizeTier::Normal || snapshot.metadata.lossy {
        return Ok(context);
    }
    let Some((project, lsp)) = environment.project.zip(environment.lsp) else {
        return Ok(context);
    };
    use taide_native_editor::symbol_locations::Kind;
    let location = |kind| !lsp.location_providers(project, &snapshot, kind).is_empty();
    context.definition = location(Kind::Definition);
    context.declaration = location(Kind::Declaration);
    context.type_definition = location(Kind::TypeDefinition);
    context.implementation = location(Kind::Implementation);
    context.references = location(Kind::References);
    context.rename = !lsp.rename_providers(project, &snapshot).is_empty();
    use taide_native_editor::formatting::Command as FormatCommand;
    context.document_formatting = !lsp
        .formatting_providers(project, &snapshot, FormatCommand::Document)
        .is_empty();
    context.range_formatting = !lsp
        .formatting_providers(project, &snapshot, FormatCommand::Selection)
        .is_empty();
    use taide_native_ui::editor_code_actions::Command;
    context.refactor = !lsp
        .code_action_providers(project, &snapshot, Command::Refactor)
        .is_empty();
    context.source_actions = !lsp
        .code_action_providers(project, &snapshot, Command::Source)
        .is_empty();
    Ok(context)
}

fn shortcut(rows: &[Row], action: Action, is_mac: bool) -> Option<String> {
    let id = action.id();
    let command = format!("monaco.{id}");
    if let Some(row) = rows.iter().find(|row| {
        row.id == id || row.id == command || row.command_id.as_deref() == Some(command.as_str())
    }) {
        return (!row.is_unassigned()).then(|| row.effective_binding().label(is_mac));
    }
    let label = match action {
        Action::Copy if is_mac => "⌘C",
        Action::Cut if is_mac => "⌘X",
        Action::Paste if is_mac => "⌘V",
        Action::Copy => "Ctrl+C",
        Action::Cut => "Ctrl+X",
        Action::Paste => "Ctrl+V",
        _ => return None,
    };
    Some(label.into())
}

fn label(environment: &Environment<'_>, action: Action, fallback: &str) -> String {
    let key = match action {
        Action::ToggleMinimap => "editor.toggleMinimap",
        Action::RunSelectedText => "terminal.runSelectedText",
        _ => return fallback.into(),
    };
    environment
        .locale
        .and_then(|locale| locale.messages.get(key))
        .map_or_else(|| fallback.into(), Clone::clone)
}

pub(crate) fn show(
    ui: &mut Ui,
    store: &mut EditorStore,
    request: menu::Request<'_>,
    environment: Environment<'_>,
    commands: &mut Vec<crate::host::HostCommand>,
) -> Result<menu::Output, EditorError> {
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            eframe::egui::Id::new(("native-editor-context-menu-body", request.source)),
            request.response.id,
        );
    });
    let owner = request.owner;
    let is_mac = ui.ctx().os().is_mac();
    let rows =
        taide_native_ui::keymap::catalog::rows(&Overrides::parse(environment.overrides), is_mac)
            .map_err(|_| EditorError::InvalidBoundary)?;
    let output = menu::show(ui, store, request, |store, source| {
        let Ok(context) = conditions(store, source, &environment) else {
            return Vec::new();
        };
        menu::groups(context, "Peek".into(), |action, fallback| {
            (
                label(&environment, action, fallback),
                shortcut(&rows, action, is_mac),
            )
        })
    })?;
    if let Some(invocation) = &output.invocation {
        let owner_key = store
            .views()
            .get(owner)
            .ok_or(EditorError::NotFound)?
            .key
            .clone();
        commands.push(crate::host::HostCommand::EditorContextMenu(Request {
            invocation: invocation.clone(),
            context: ui.ctx().clone(),
            project: environment.project.cloned(),
            scope: environment.scope.clone(),
            owner_key,
        }));
    }
    Ok(output)
}

#[cfg(test)]
pub(crate) fn body(context: &EguiContext, source: ViewId) -> Option<eframe::egui::Id> {
    context.data(|data| {
        data.get_temp(eframe::egui::Id::new((
            "native-editor-context-menu-body",
            source,
        )))
    })
}

#[derive(Clone)]
pub(crate) enum ClipboardOperation {
    Copy(ClipboardText),
    Cut(ClipboardText),
    Paste,
}

#[derive(Clone)]
pub struct ClipboardRequest {
    pub(crate) target: Request,
    pub(crate) operation: ClipboardOperation,
    token: Uuid,
    cancelled: watch::Receiver<bool>,
}

impl ClipboardRequest {
    pub(crate) fn is_cancelled(&self) -> bool {
        *self.cancelled.borrow() || self.cancelled.has_changed().is_err()
    }
}

struct Pending {
    request: ClipboardRequest,
    cancel: watch::Sender<bool>,
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.cancel.send_replace(true);
    }
}

#[derive(Default)]
pub(crate) struct Clipboard {
    pending: HashMap<ViewId, Pending>,
}

impl Clipboard {
    #[cfg(test)]
    pub(crate) fn has_pending(&self, source: ViewId) -> bool {
        self.pending.contains_key(&source)
    }

    pub(crate) fn begin(
        &mut self,
        target: Request,
        operation: ClipboardOperation,
    ) -> ClipboardRequest {
        let source = target.invocation.session.source;
        let (cancel, cancelled) = watch::channel(false);
        let request = ClipboardRequest {
            target,
            operation,
            token: Uuid::new_v4(),
            cancelled,
        };
        self.pending.insert(
            source,
            Pending {
                request: request.clone(),
                cancel,
            },
        );
        request
    }

    pub(crate) fn finish(&mut self, request: &ClipboardRequest, store: &EditorStore) -> bool {
        let source = request.target.invocation.session.source;
        let current = !request.is_cancelled()
            && request.target.matches(store)
            && self
                .pending
                .get(&source)
                .is_some_and(|pending| pending.request.token == request.token);
        if self
            .pending
            .get(&source)
            .is_some_and(|pending| pending.request.token == request.token)
        {
            self.pending.remove(&source);
        }
        current
    }

    pub(crate) fn reconcile(&mut self, store: &EditorStore, state: &taide_runtime::AppState) {
        self.pending.retain(|_, pending| {
            !state.is_shutting_down()
                && pending.request.target.matches(store)
                && pending.request.target.is_active(state)
        });
    }

    pub(crate) fn clear(&mut self) {
        self.pending.clear();
    }
}
