use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use eframe::egui::{self, Ui};
use taide_model::ai::{AiCommitMessageRequest, AiTextResponse};
use taide_model::error::AppResult;
use taide_model::git::{
    CommitOptions, DiffMode, DiffSides, RevertOutcome, StagedDiffText, TagCreateOptions,
};
use taide_model::ide::IdeDiffOutcome;
use taide_model::ids::{PaneId, ProjectId, ShellSlotId, TabId};
use taide_model::layout::{Tab, TabKind};
use taide_native_editor::document::{DocumentId, DocumentKey, VirtualKey, VirtualSide};
use taide_native_editor::view::ViewKey;
use taide_native_ui::commands::ShellIntent;
use taide_native_ui::intents::git::{Intent, Section};
use taide_native_ui::surfaces::git::GitSurfaces;
use taide_runtime::{AppServices, git_actions};

use crate::application::{AppSurfaces, NativeApplication};
use crate::git_cache::{GitCache, Invalidation, Key, Request, Value};
use crate::git_scm_model::{Collapsed, CommitGate};
use crate::host::{HostCommand, HostReply, HostWorker, stopped};
use crate::presentation;

pub type InitFuture = Pin<Box<dyn Future<Output = AppResult<()>> + Send>>;
pub type InitPort = Arc<dyn Fn(Arc<AppServices>, ProjectId) -> InitFuture + Send + Sync>;

const LOG_PAGE: u32 = 100;
const HEAD: &str = "HEAD";
const VIRTUAL_SIDES: [VirtualSide; 2] = [VirtualSide::Original, VirtualSide::Modified];

/// The diff tab being drawn and the time its read last answered.
struct DiffTab<'t> {
    project: &'t ProjectId,
    pane: &'t PaneId,
    tab: &'t Tab,
    path: &'t str,
    staged: bool,
    compare_with: Option<&'t str>,
    is_commit: bool,
    fetched_at: Option<Instant>,
}

/// Whether a surface's read answered since its documents last took its texts, remembering this
/// answer as taken.
fn take_fresh(ui: &Ui, owner: &TabId, fetched_at: Option<Instant>) -> bool {
    let applied = egui::Id::new(("taide-diff-applied", owner));
    let fresh = ui.data(|data| data.get_temp::<Instant>(applied)) != fetched_at;
    if let Some(fetched_at) = fetched_at {
        ui.data_mut(|data| data.insert_temp(applied, fetched_at));
    }
    fresh
}

/// A diff read's answer, or the empty editor background while loading and the centered failure
/// text when it failed (`diff-pane.tsx`, `commit-file-diff.tsx`).
fn loaded_diff<'g>(
    ui: &Ui,
    git: &'g State,
    locale: &taide_model::locale::ResolvedLocale,
    project: &ProjectId,
    key: &Key,
    rect: egui::Rect,
    appearance: crate::git_diff::Appearance,
) -> Option<&'g crate::git_cache::Diff> {
    match git.cache.diff(project, key)? {
        Ok(diff) => Some(diff),
        Err(_) => {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                presentation::message(locale, "editor.diffLoadFailed", &[]),
                egui::FontId::proportional(crate::git_diff::FAILED_TEXT_SIZE),
                appearance.error,
            );
            None
        }
    }
}

pub enum Command {
    GitBranch(ProjectId),
    Read(Request<Key>),
    Init {
        project: ProjectId,
        init: InitPort,
    },
    PatchSettings(taide_model::settings::SettingsPatch),
    Stage {
        project: ProjectId,
        paths: Vec<String>,
    },
    Unstage {
        project: ProjectId,
        paths: Vec<String>,
    },
    Discard {
        project: ProjectId,
        paths: Vec<String>,
    },
    OpenTab {
        project: ProjectId,
        kind: TabKind,
        title: String,
        target: Option<PaneId>,
        preview: bool,
    },
    BranchCheckout {
        project: ProjectId,
        name: String,
    },
    CheckoutRemote {
        project: ProjectId,
        remote_ref: String,
    },
    BranchCreate {
        project: ProjectId,
        name: String,
    },
    StashPush(ProjectId),
    StashApply {
        project: ProjectId,
        index: u32,
    },
    StashDrop {
        project: ProjectId,
        index: u32,
    },
    Pull(ProjectId),
    Push(ProjectId),
    Commit {
        project: ProjectId,
        message: String,
        stage_all: bool,
        window: String,
    },
    AiCommitMessage {
        project: ProjectId,
        request: AiCommitMessageRequest,
        port: crate::git_commit::AiPort,
    },
    AiCancel {
        owner: String,
        request_id: String,
    },
    StageHunk {
        project: ProjectId,
        path: String,
        start: u32,
        end: u32,
    },
    UnstageHunk {
        project: ProjectId,
        path: String,
        start: u32,
        end: u32,
    },
    Revert {
        project: ProjectId,
        rev: String,
        opens_conflict: bool,
    },
    TagCreate {
        project: ProjectId,
        slot: ShellSlotId,
        name: String,
        target: String,
        message: Option<String>,
    },
    TagDelete {
        project: ProjectId,
        name: String,
    },
    /// A Claude diff's line hunks for its current texts (monaco's default `ignoreTrimWhitespace`).
    Hunks {
        tab: TabId,
        revisions: crate::git_claude_diff::Revisions,
        original: String,
        modified: String,
    },
    /// `ide_resolve_diff`; `tab` is the pane's tab, `None` for the automatic reject.
    ResolveIdeDiff {
        tab: Option<TabId>,
        request_id: String,
        outcome: IdeDiffOutcome,
        content: Option<String>,
    },
    DiscardHunk {
        project: ProjectId,
        path: String,
        start: u32,
        end: u32,
    },
    StageLines {
        project: ProjectId,
        path: String,
        start: u32,
        end: u32,
    },
    /// `git_resolve_conflict` for a file whose markers are all resolved, then its disk read for the
    /// editor's save settle; `snapshot` is the resolved document.
    ResolveConflict {
        project: ProjectId,
        path: String,
        content: String,
        snapshot: taide_native_editor::store::SaveSnapshot,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MutationOp {
    Init,
    Stage,
    Unstage,
    Discard,
    SwitchBranch(String),
    StashPush,
    StashApply,
    StashDrop,
    Pull,
    Push,
    Commit { window: String },
    StageHunk,
    UnstageHunk,
    TagCreate { slot: ShellSlotId },
    TagDelete,
    DiscardHunk,
    StageLines,
}

pub enum Reply {
    GitBranch {
        project: ProjectId,
        branch: Option<String>,
    },
    Read {
        request: Request<Key>,
        result: AppResult<Value>,
    },
    Mutated {
        project: ProjectId,
        op: MutationOp,
        result: AppResult<()>,
    },
    AiCommitMessage {
        request_id: String,
        result: AppResult<(StagedDiffText, AiTextResponse)>,
    },
    Reverted {
        project: ProjectId,
        opens_conflict: bool,
        result: AppResult<RevertOutcome>,
    },
    Hunks {
        tab: TabId,
        revisions: crate::git_claude_diff::Revisions,
        result: AppResult<Vec<taide_native_ui::diff_view::Hunk>>,
    },
    IdeDiffResolved {
        tab: Option<TabId>,
        request_id: String,
        outcome: IdeDiffOutcome,
        result: AppResult<()>,
    },
    /// The resolution's outcome with the file's disk read for the save settle, when it succeeded.
    ConflictResolved {
        project: ProjectId,
        snapshot: taide_native_editor::store::SaveSnapshot,
        result: AppResult<Option<taide_model::file::OpenedFile>>,
    },
}

pub(crate) struct State {
    pub(crate) title_branch_changes: Arc<crate::title_branch::Changes>,
    pub(crate) title_branches: crate::title_branch::Branches,
    pub(crate) cache_changes: Arc<crate::git_cache::Changes>,
    pub(crate) cache: GitCache<Key, Value>,
    pub(crate) panels: crate::git_scm::Panels,
    pub(crate) appearance: Option<crate::git_scm::Appearance>,
    pub(crate) initializing: HashSet<ProjectId>,
    pub(crate) init: InitPort,
    pub(crate) stashing: HashMap<ProjectId, usize>,
    pub(crate) syncing: HashSet<ProjectId>,
    pub(crate) commit: crate::git_commit::State,
    pub(crate) diffs: crate::git_diff::Tabs,
    pub(crate) diff_appearance: Option<crate::git_diff::Appearance>,
    pub(crate) claude: crate::git_claude_diff::State,
    pub(crate) claude_appearance: Option<crate::git_claude_diff::Appearance>,
    pub(crate) histories: crate::git_history::Panels,
    pub(crate) history_appearance: Option<crate::git_history::Appearance>,
    pub(crate) editors: crate::git_gutter::Editors,
}

pub(crate) fn runtime_init() -> InitPort {
    Arc::new(|services, project| {
        Box::pin(async move {
            git_actions::git_init(
                services.events.as_ref(),
                git_actions::GitActionContext::new(&services.state, &services.git, &services.tasks),
                project,
            )
            .await
        })
    })
}

fn spawn_reply(
    worker: &HostWorker<'_>,
    name: &'static str,
    reply: impl Future<Output = HostReply> + Send + 'static,
    refused: HostReply,
) -> Option<HostReply> {
    let sender = worker.sender.clone();
    let repaint = worker.repaint.clone();
    if worker.services.tasks.spawn_transient(name, async move {
        if sender.send(reply.await).await.is_ok() {
            repaint();
        }
    }) {
        None
    } else {
        Some(refused)
    }
}

async fn read(services: &AppServices, request: &Request<Key>) -> AppResult<Value> {
    let context =
        git_actions::GitActionContext::new(&services.state, &services.git, &services.tasks);
    let project = request.project.clone();
    match request.key {
        Key::Status => git_actions::git_status(|| {}, context, project)
            .await
            .map(Value::Status),
        Key::Remotes => git_actions::git_remotes(context, project)
            .await
            .map(Value::Remotes),
        Key::Branches => git_actions::git_branches(context, project)
            .await
            .map(Value::Branches),
        Key::Stashes => git_actions::git_stash_list(context, project)
            .await
            .map(Value::Stashes),
        Key::Log => git_actions::git_log(context, project, 0, LOG_PAGE)
            .await
            .map(Value::Log),
        Key::Tags => git_actions::git_tags(context, project)
            .await
            .map(Value::Tags),
        Key::CommitFiles { ref rev } => {
            git_actions::git_commit_files(context, project, rev.clone())
                .await
                .map(Value::CommitFiles)
        }
        Key::Diff {
            ref path,
            staged,
            ref before_path,
        } => {
            let sides = git_actions::git_diff_file(
                context,
                || language_overlays(services),
                project,
                path.clone(),
                if staged {
                    DiffMode::IndexVsHead
                } else {
                    DiffMode::WorkdirVsIndex
                },
                before_path.clone(),
            )
            .await?;
            with_hunks(services, sides).await
        }
        Key::Compare {
            ref original,
            ref modified,
        } => {
            let open = |path: &String| {
                taide_runtime::file_actions::file_open(
                    &services.state,
                    &services.tasks,
                    path.clone(),
                    || language_overlays(services),
                )
            };
            let (original, modified) = (open(original).await?, open(modified).await?);
            with_hunks(
                services,
                DiffSides {
                    original: original.content,
                    modified: modified.content,
                    language_id: modified.language_id,
                },
            )
            .await
        }
        Key::CommitDiff {
            ref rev,
            ref parent,
            ref path,
            ref before_path,
        } => {
            let show = |rev: String, path: String| {
                let project = project.clone();
                async move {
                    match git_actions::git_show_file(context, project, rev, path).await {
                        Err(error)
                            if error.kind() == taide_model::error::AppErrorKind::NotFound =>
                        {
                            Ok(String::new())
                        }
                        result => result,
                    }
                }
            };
            let original = match parent {
                Some(parent) => show(parent.clone(), before_path.clone()).await?,
                None => String::new(),
            };
            let modified = show(rev.clone(), path.clone()).await?;
            with_hunks(
                services,
                DiffSides {
                    original,
                    modified,
                    language_id: taide_infra::language::language_id_for_path(
                        std::path::Path::new(path),
                        &language_overlays(services),
                    ),
                },
            )
            .await
        }
        Key::FileLog { ref path } => {
            git_actions::git_file_log(context, project, path.clone(), 0, LOG_PAGE)
                .await
                .map(Value::FileLog)
        }
        Key::File { ref path } => taide_runtime::file_actions::file_open(
            &services.state,
            &services.tasks,
            path.clone(),
            || language_overlays(services),
        )
        .await
        .map(Value::File),
        Key::Gutter { ref path } => git_actions::git_gutter(context, project, path.clone())
            .await
            .map(Value::Gutter),
        Key::CurrentUser => git_actions::git_current_user(context, project)
            .await
            .map(Value::CurrentUser),
        Key::BlameLine { ref path, line } => {
            git_actions::git_blame_range(context, project, path.clone(), line, line)
                .await
                .map(Value::Blame)
        }
        Key::BlameOverlay { ref path, lines } => {
            git_actions::git_blame_range(context, project, path.clone(), 1, lines)
                .await
                .map(Value::Blame)
        }
        Key::ConflictSides { ref path } => {
            let sides = git_actions::git_conflict_sides(context, project, path.clone()).await?;
            with_hunks(
                services,
                DiffSides {
                    original: sides.ours.unwrap_or_default(),
                    modified: sides.theirs.unwrap_or_default(),
                    language_id: taide_infra::language::language_id_for_path(
                        std::path::Path::new(path),
                        &language_overlays(services),
                    ),
                },
            )
            .await
        }
    }
}

fn language_overlays(services: &AppServices) -> Vec<taide_infra::language::LanguageOverlay> {
    let loaded =
        taide_plugin::service::ensure_loaded(&services.plugin, &services.state.paths.plugins_dir());
    taide_plugin::service::language_overlays(&loaded)
}

/// Adds the line hunks of a git diff surface (`ignoreTrimWhitespace: false`), computed off the
/// async workers.
async fn with_hunks(services: &AppServices, sides: DiffSides) -> AppResult<Value> {
    let (original, modified) = (sides.original.clone(), sides.modified.clone());
    let hunks = services
        .tasks
        .run_blocking_result("native-git-diff-hunks", move || {
            taide_git::service::diff_hunks(&original, &modified, false)
        })
        .await?;
    Ok(Value::Diff(crate::git_cache::Diff {
        sides,
        hunks: hunks.into_iter().map(view_hunk).collect(),
    }))
}

pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply> {
    let services = worker.services;
    let sender = worker.sender;
    let repaint = worker.repaint;
    match command {
        Command::GitBranch(project) => {
            let pending_services = services.clone();
            let sender = sender.clone();
            let repaint = repaint.clone();
            let pending_project = project.clone();
            if services
                .tasks
                .spawn_transient("native-title-branch", async move {
                    let status = git_actions::git_status(
                        || {},
                        git_actions::GitActionContext::new(
                            &pending_services.state,
                            &pending_services.git,
                            &pending_services.tasks,
                        ),
                        pending_project.clone(),
                    )
                    .await;
                    let reply = HostReply::Git(Reply::GitBranch {
                        project: pending_project,
                        branch: status.ok().and_then(|status| status.branch),
                    });
                    if sender.send(reply).await.is_ok() {
                        repaint();
                    }
                })
            {
                None
            } else {
                Some(HostReply::Git(Reply::GitBranch {
                    project,
                    branch: None,
                }))
            }
        }
        Command::Read(request) => {
            let pending_services = services.clone();
            let refused = HostReply::Git(Reply::Read {
                request: request.clone(),
                result: Err(stopped()),
            });
            spawn_reply(
                worker,
                "native-git-read",
                async move {
                    let result = read(&pending_services, &request).await;
                    HostReply::Git(Reply::Read { request, result })
                },
                refused,
            )
        }
        Command::Init { project, init } => {
            let refused = HostReply::Git(Reply::Mutated {
                project: project.clone(),
                op: MutationOp::Init,
                result: Err(stopped()),
            });
            let future = init(services.clone(), project.clone());
            spawn_reply(
                worker,
                "native-git-init",
                async move {
                    HostReply::Git(Reply::Mutated {
                        project,
                        op: MutationOp::Init,
                        result: future.await,
                    })
                },
                refused,
            )
        }
        Command::PatchSettings(patch) => {
            crate::host::update_control_settings(services, &worker.integrations.reconcile, patch)
                .await
        }
        Command::Stage { project, paths } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::Stage, async move {
                git_actions::git_stage(services.events.as_ref(), context(&services), target, paths)
                    .await
            })
        }
        Command::Unstage { project, paths } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::Unstage, async move {
                git_actions::git_unstage(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    paths,
                )
                .await
            })
        }
        Command::Discard { project, paths } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::Discard, async move {
                git_actions::git_discard(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    paths,
                )
                .await
            })
        }
        Command::OpenTab {
            project,
            kind,
            title,
            target,
            preview,
        } => taide_runtime::layout_actions::layout_open_tab(
            services.events.as_ref(),
            &services.state,
            project,
            kind,
            title,
            target,
            preview,
        )
        .await
        .err()
        .map(HostReply::Failed),
        Command::BranchCheckout { project, name } => {
            let services = services.clone();
            let target = project.clone();
            let op = MutationOp::SwitchBranch(name.clone());
            change(worker, project, op, async move {
                git_actions::git_branch_checkout(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    name,
                )
                .await
            })
        }
        Command::CheckoutRemote {
            project,
            remote_ref,
        } => {
            let services = services.clone();
            let target = project.clone();
            let op = MutationOp::SwitchBranch(remote_ref.clone());
            change(worker, project, op, async move {
                git_actions::git_checkout_remote_branch(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    remote_ref,
                )
                .await
            })
        }
        Command::BranchCreate { project, name } => {
            let services = services.clone();
            let target = project.clone();
            let op = MutationOp::SwitchBranch(name.clone());
            change(worker, project, op, async move {
                git_actions::git_branch_create(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    name,
                    true,
                )
                .await
            })
        }
        Command::StashPush(project) => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::StashPush, async move {
                git_actions::git_stash_push(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    None,
                )
                .await
            })
        }
        Command::StashApply { project, index } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::StashApply, async move {
                git_actions::git_stash_apply(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    index,
                )
                .await
            })
        }
        Command::StashDrop { project, index } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::StashDrop, async move {
                git_actions::git_stash_drop(context(&services), target, index).await
            })
        }
        Command::Pull(project) => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::Pull, async move {
                git_actions::git_pull(services.events.as_ref(), context(&services), target).await
            })
        }
        Command::Push(project) => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::Push, async move {
                git_actions::git_push(services.events.as_ref(), context(&services), target).await
            })
        }
        Command::Commit {
            project,
            message,
            stage_all,
            window,
        } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::Commit { window }, async move {
                git_actions::git_commit(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    message,
                    CommitOptions {
                        amend: false,
                        stage_all,
                    },
                )
                .await
                .map(|_| ())
            })
        }
        Command::AiCommitMessage {
            project,
            mut request,
            port,
        } => {
            let services = services.clone();
            let request_id = request.request_id.clone();
            let refused = HostReply::Git(Reply::AiCommitMessage {
                request_id: request_id.clone(),
                result: Err(stopped()),
            });
            spawn_reply(
                worker,
                "native-git-ai-commit-message",
                async move {
                    let result = async {
                        let diff =
                            git_actions::git_diff_staged_text(context(&services), project).await?;
                        request.diff_text = diff.diff_text.clone();
                        let response = port(services.clone(), request).await?;
                        Ok((diff, response))
                    }
                    .await;
                    HostReply::Git(Reply::AiCommitMessage { request_id, result })
                },
                refused,
            )
        }
        Command::AiCancel { owner, request_id } => {
            let _ = taide_runtime::ai_actions::ai_request_cancel(
                &services.ai_requests,
                owner,
                request_id,
            )
            .await;
            None
        }
        Command::StageHunk {
            project,
            path,
            start,
            end,
        } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::StageHunk, async move {
                git_actions::git_stage_hunk(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    path,
                    start,
                    end,
                )
                .await
            })
        }
        Command::UnstageHunk {
            project,
            path,
            start,
            end,
        } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::UnstageHunk, async move {
                git_actions::git_unstage_hunk(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    path,
                    start,
                    end,
                )
                .await
            })
        }
        Command::Revert {
            project,
            rev,
            opens_conflict,
        } => {
            let services = services.clone();
            let refused = HostReply::Git(Reply::Reverted {
                project: project.clone(),
                opens_conflict,
                result: Err(stopped()),
            });
            spawn_reply(
                worker,
                "native-git-revert",
                async move {
                    let result = git_actions::git_revert_commit(
                        services.events.as_ref(),
                        context(&services),
                        project.clone(),
                        rev,
                    )
                    .await;
                    HostReply::Git(Reply::Reverted {
                        project,
                        opens_conflict,
                        result,
                    })
                },
                refused,
            )
        }
        Command::TagCreate {
            project,
            slot,
            name,
            target,
            message,
        } => {
            let services = services.clone();
            let project_id = project.clone();
            change(
                worker,
                project,
                MutationOp::TagCreate { slot },
                async move {
                    git_actions::git_tag_create(
                        services.events.as_ref(),
                        context(&services),
                        project_id,
                        name,
                        target,
                        TagCreateOptions {
                            message,
                            annotated: true,
                        },
                    )
                    .await
                },
            )
        }
        Command::TagDelete { project, name } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::TagDelete, async move {
                git_actions::git_tag_delete(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    name,
                )
                .await
            })
        }
        Command::Hunks {
            tab,
            revisions,
            original,
            modified,
        } => {
            let services = services.clone();
            let refused = HostReply::Git(Reply::Hunks {
                tab: tab.clone(),
                revisions,
                result: Err(stopped()),
            });
            spawn_reply(
                worker,
                "native-git-claude-diff-hunks",
                async move {
                    let result = services
                        .tasks
                        .run_blocking_result("native-git-diff-hunks", move || {
                            taide_git::service::diff_hunks(&original, &modified, true)
                        })
                        .await
                        .map(|hunks| hunks.into_iter().map(view_hunk).collect());
                    HostReply::Git(Reply::Hunks {
                        tab,
                        revisions,
                        result,
                    })
                },
                refused,
            )
        }
        Command::ResolveIdeDiff {
            tab,
            request_id,
            outcome,
            content,
        } => {
            let services = services.clone();
            let refused = HostReply::Git(Reply::IdeDiffResolved {
                tab: tab.clone(),
                request_id: request_id.clone(),
                outcome,
                result: Err(stopped()),
            });
            spawn_reply(
                worker,
                "native-git-ide-resolve-diff",
                async move {
                    let result = taide_runtime::ide_actions::ide_resolve_diff(
                        &services.state,
                        &services.ide_save_file,
                        &services.ide,
                        request_id.clone(),
                        outcome,
                        content,
                    )
                    .await;
                    HostReply::Git(Reply::IdeDiffResolved {
                        tab,
                        request_id,
                        outcome,
                        result,
                    })
                },
                refused,
            )
        }
        Command::DiscardHunk {
            project,
            path,
            start,
            end,
        } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::DiscardHunk, async move {
                git_actions::git_discard_hunk(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    path,
                    start,
                    end,
                )
                .await
            })
        }
        Command::StageLines {
            project,
            path,
            start,
            end,
        } => {
            let services = services.clone();
            let target = project.clone();
            change(worker, project, MutationOp::StageLines, async move {
                git_actions::git_stage_lines(
                    services.events.as_ref(),
                    context(&services),
                    target,
                    path,
                    start,
                    end,
                )
                .await
            })
        }
        Command::ResolveConflict {
            project,
            path,
            content,
            snapshot,
        } => {
            let services = services.clone();
            let refused = HostReply::Git(Reply::ConflictResolved {
                project: project.clone(),
                snapshot: snapshot.clone(),
                result: Err(stopped()),
            });
            spawn_reply(
                worker,
                "native-git-resolve-conflict",
                async move {
                    let result = match git_actions::git_resolve_conflict(
                        services.events.as_ref(),
                        context(&services),
                        project.clone(),
                        path.clone(),
                        content,
                    )
                    .await
                    {
                        Ok(()) => Ok(taide_runtime::file_actions::file_open(
                            &services.state,
                            &services.tasks,
                            path,
                            || language_overlays(&services),
                        )
                        .await
                        .ok()),
                        Err(error) => Err(error),
                    };
                    HostReply::Git(Reply::ConflictResolved {
                        project,
                        snapshot,
                        result,
                    })
                },
                refused,
            )
        }
    }
}

fn claude_revisions(
    store: &taide_native_editor::store::EditorStore,
    tab: &TabId,
) -> Option<crate::git_claude_diff::Revisions> {
    let revision = |side| {
        let document = store.documents().find(&DocumentKey::Virtual(VirtualKey {
            owner: tab.clone(),
            side,
        }))?;
        store
            .documents()
            .snapshot(document)
            .ok()
            .map(|snapshot| snapshot.revision)
    };
    Some((
        revision(VirtualSide::Original)?,
        revision(VirtualSide::Modified)?,
    ))
}

/// How one side's virtual document follows its source text.
#[derive(Clone, Copy)]
enum SideText<'t> {
    /// Replaced by `text` when `fresh` (its read answered again).
    Read { text: &'t str, fresh: bool },
    /// Opened editable with `text`, then owned by the user's edits; only its language follows.
    Editable(&'t str),
}

/// What one diff surface draws: the documents' owner and view keys, texts and options.
struct Surface<'t> {
    owner: &'t TabId,
    pane: &'t PaneId,
    language_id: &'t str,
    sides: [SideText<'t>; 2],
    hunks: &'t [taide_native_ui::diff_view::Hunk],
    side_by_side: bool,
    stage_lanes: bool,
    hide_unchanged: bool,
}

/// The editor state a diff surface draws with.
struct Parts<'a> {
    store: &'a mut taide_native_editor::store::EditorStore,
    editor: &'a taide_native_ui::editor_surface::NativeEditor,
    syntax: &'a mut crate::editor_syntax::EditorSyntax,
    services: &'a AppServices,
    display: taide_native_ui::editor_display::EditorDisplayColors,
    brackets: taide_native_ui::editor_brackets::EditorBracketColors,
}

/// Opens or refreshes the surface's two virtual documents, attaches its views and draws the diff
/// widget (`DiffView`).
fn show_surface(
    ui: &mut Ui,
    parts: Parts<'_>,
    surface: Surface<'_>,
    colors: taide_native_ui::diff_view::Colors,
) -> AppResult<(taide_native_ui::diff_view::Output, [DocumentId; 2])> {
    let Parts {
        store,
        editor,
        syntax,
        services,
        display,
        brackets,
    } = parts;
    let editor_error = crate::application::editor_error;
    let mut views = Vec::with_capacity(VIRTUAL_SIDES.len());
    let mut documents = Vec::with_capacity(VIRTUAL_SIDES.len());
    for (side, text) in VIRTUAL_SIDES.into_iter().zip(surface.sides) {
        let key = VirtualKey {
            owner: surface.owner.clone(),
            side,
        };
        let existing = store.documents().find(&DocumentKey::Virtual(key.clone()));
        let document = match (existing, text) {
            (Some(document), SideText::Read { text, fresh: true }) => {
                store
                    .replace_virtual(document, text, surface.language_id)
                    .map_err(editor_error)?;
                document
            }
            (Some(document), SideText::Read { .. }) => document,
            (Some(document), SideText::Editable(_)) => {
                let snapshot = store.documents().snapshot(document).map_err(editor_error)?;
                if snapshot.metadata.language_id != surface.language_id {
                    store
                        .replace_virtual(document, &snapshot.rope.to_string(), surface.language_id)
                        .map_err(editor_error)?;
                }
                document
            }
            (None, SideText::Read { text, .. }) => store
                .open_virtual(key, text, surface.language_id.to_owned(), true)
                .map_err(editor_error)?,
            (None, SideText::Editable(text)) => store
                .open_virtual(key, text, surface.language_id.to_owned(), false)
                .map_err(editor_error)?,
        };
        let view = store
            .attach_view(
                ViewKey {
                    window: crate::git_diff::window(side).into(),
                    pane: surface.pane.clone(),
                    tab: surface.owner.clone(),
                },
                document,
            )
            .map_err(editor_error)?;
        documents.push(document);
        views.push(view);
    }
    let settings = services.state.settings.read();
    let mut editor_presentation = crate::presentation_refresh::editor_presentation(&settings);
    drop(settings);
    editor_presentation.options.colors = Some(display);
    editor_presentation.options.bracket_colors = Some(brackets);
    let output = taide_native_ui::diff_view::show(
        ui,
        store,
        editor,
        taide_native_ui::diff_view::Request {
            original: views[0],
            modified: views[1],
            hunks: surface.hunks,
            side_by_side: surface.side_by_side,
            stage_lanes: surface.stage_lanes,
            hide_unchanged: surface.hide_unchanged,
            presentation: &editor_presentation,
            colors,
        },
        &mut crate::git_diff::SyntaxTokens(syntax),
    )
    .map_err(editor_error)?;
    if let Some(original) = &output.original {
        syntax.show_lines(documents[0], original.rendered_lines.clone());
    }
    syntax.show_lines(documents[1], output.modified.rendered_lines.clone());
    Ok((output, [documents[0], documents[1]]))
}

fn view_hunk(hunk: taide_git::service::DiffHunk) -> taide_native_ui::diff_view::Hunk {
    taide_native_ui::diff_view::Hunk {
        old_start: hunk.old_start,
        old_lines: hunk.old_lines,
        new_start: hunk.new_start,
        new_lines: hunk.new_lines,
    }
}

/// `explorer-container.tsx` `compareWithSelected`: a preview file-against-file diff tab.
pub(crate) fn compare_tab(project: ProjectId, source: String, path: String) -> HostCommand {
    let name = |path: &str| {
        std::path::Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(path)
            .to_owned()
    };
    HostCommand::Git(Command::OpenTab {
        project,
        title: format!("{} vs {}", name(&source), name(&path)),
        kind: TabKind::Diff {
            path,
            staged: false,
            compare_with: Some(source),
            rev: None,
            parent_rev: None,
            before_path: None,
        },
        target: None,
        preview: true,
    })
}

fn finish_stash(stashing: &mut HashMap<ProjectId, usize>, project: &ProjectId) {
    if let Some(count) = stashing.get_mut(project) {
        *count = count.saturating_sub(1);
        if *count == 0 {
            stashing.remove(project);
        }
    }
}

fn context(services: &AppServices) -> git_actions::GitActionContext<'_> {
    git_actions::GitActionContext::new(&services.state, &services.git, &services.tasks)
}

fn change(
    worker: &HostWorker<'_>,
    project: ProjectId,
    op: MutationOp,
    run: impl Future<Output = AppResult<()>> + Send + 'static,
) -> Option<HostReply> {
    let refused = HostReply::Git(Reply::Mutated {
        project: project.clone(),
        op: op.clone(),
        result: Err(stopped()),
    });
    spawn_reply(
        worker,
        "native-git-change",
        async move {
            HostReply::Git(Reply::Mutated {
                project,
                op,
                result: run.await,
            })
        },
        refused,
    )
}

impl NativeApplication {
    pub(crate) fn apply_git_reply(&mut self, context: &egui::Context, reply: Reply) {
        match reply {
            Reply::GitBranch { project, branch } => {
                self.git_state.title_branches.finish(&project, branch);
                context.request_repaint();
            }
            Reply::Read { request, result } => {
                self.git_state
                    .cache
                    .finish(&request, result, Instant::now());
                context.request_repaint();
            }
            Reply::Hunks {
                tab,
                revisions,
                result,
            } => {
                let current = claude_revisions(&self.store, &tab);
                self.git_state
                    .claude
                    .finish(&tab, revisions, result, current);
                context.request_repaint();
            }
            Reply::IdeDiffResolved {
                tab,
                request_id,
                outcome,
                result,
            } => {
                self.git_state.claude.finish_resolve(&request_id);
                let now = Instant::now();
                match (result, tab) {
                    (Ok(()), None) => {}
                    (Ok(()), Some(tab)) => {
                        self.git_state.claude.remove(&request_id);
                        let message = |key| presentation::message(&self.locale, key, &[]);
                        if outcome == IdeDiffOutcome::Saved {
                            self.toasts.success(message("ide.diffAccepted"), now);
                        } else {
                            self.toasts.info(message("ide.diffRejected"), now);
                        }
                        self.request_tab_close(tab);
                    }
                    (Err(error), _) => self.toasts.ipc_error(&self.locale, &error, now),
                }
                context.request_repaint();
            }
            Reply::Mutated {
                project,
                op: MutationOp::Init,
                result,
            } => {
                self.git_state.initializing.remove(&project);
                let now = Instant::now();
                match result {
                    Ok(()) => {
                        self.git_state
                            .cache
                            .invalidate(&project, &Invalidation::repository());
                        self.toasts.success(
                            presentation::message(&self.locale, "git.initSuccess", &[]),
                            now,
                        );
                    }
                    Err(_) => self.toasts.error(
                        presentation::message(&self.locale, "git.initFailed", &[]),
                        now,
                    ),
                }
                context.request_repaint();
            }
            Reply::Mutated {
                project,
                op:
                    MutationOp::Stage
                    | MutationOp::Unstage
                    | MutationOp::Discard
                    | MutationOp::StageHunk
                    | MutationOp::UnstageHunk
                    | MutationOp::DiscardHunk
                    | MutationOp::StageLines,
                result,
            } => {
                match result {
                    Ok(()) => self
                        .git_state
                        .cache
                        .invalidate(&project, &Invalidation::repository()),
                    Err(error) => self.toasts.ipc_error(&self.locale, &error, Instant::now()),
                }
                context.request_repaint();
            }
            Reply::Mutated {
                project,
                op: MutationOp::SwitchBranch(name),
                result,
            } => {
                let now = Instant::now();
                match result {
                    Ok(()) => {
                        self.git_state
                            .cache
                            .invalidate(&project, &Invalidation::repository());
                        self.toasts.success(
                            presentation::message(
                                &self.locale,
                                "git.branchSwitched",
                                &[("name", &name)],
                            ),
                            now,
                        );
                    }
                    Err(error) => self.toasts.ipc_error(&self.locale, &error, now),
                }
                context.request_repaint();
            }
            Reply::Mutated {
                project,
                op: op @ (MutationOp::StashPush | MutationOp::StashApply | MutationOp::StashDrop),
                result,
            } => {
                finish_stash(&mut self.git_state.stashing, &project);
                let now = Instant::now();
                match result {
                    Ok(()) => {
                        self.git_state
                            .cache
                            .invalidate(&project, &Invalidation::repository());
                        if op == MutationOp::StashPush {
                            self.toasts.success(
                                presentation::message(&self.locale, "git.stashPushed", &[]),
                                now,
                            );
                        }
                    }
                    Err(error) => self.toasts.ipc_error(&self.locale, &error, now),
                }
                context.request_repaint();
            }
            Reply::Mutated {
                project,
                op: MutationOp::Pull,
                result,
            } => {
                match result {
                    Ok(()) => {
                        self.git_state
                            .cache
                            .invalidate(&project, &Invalidation::repository());
                        if self.git_state.syncing.contains(&project)
                            && !self.submit(HostCommand::Git(Command::Push(project.clone())))
                        {
                            self.git_state.syncing.remove(&project);
                        }
                    }
                    Err(error) => {
                        self.git_state.syncing.remove(&project);
                        self.toasts.ipc_error(&self.locale, &error, Instant::now());
                    }
                }
                context.request_repaint();
            }
            Reply::Mutated {
                project,
                op: MutationOp::Push,
                result,
            } => {
                self.git_state.syncing.remove(&project);
                match result {
                    Ok(()) => self
                        .git_state
                        .cache
                        .invalidate(&project, &Invalidation::repository()),
                    Err(error) => self.toasts.ipc_error(&self.locale, &error, Instant::now()),
                }
                context.request_repaint();
            }
            Reply::Mutated {
                project,
                op: MutationOp::Commit { window },
                result,
            } => {
                self.git_state.commit.committing.remove(&project);
                match result {
                    Ok(()) => {
                        self.git_state
                            .cache
                            .invalidate(&project, &Invalidation::repository());
                        self.git_state
                            .commit
                            .write(&window, &project, String::new());
                    }
                    Err(error) => self.toasts.ipc_error(&self.locale, &error, Instant::now()),
                }
                context.request_repaint();
            }
            Reply::AiCommitMessage { request_id, result } => {
                self.finish_commit_message(&request_id, result);
                context.request_repaint();
            }
            Reply::Mutated {
                project,
                op: MutationOp::TagCreate { slot },
                result,
            } => {
                let now = Instant::now();
                self.git_state
                    .panels
                    .finish_tag(&project, &slot, result.is_ok());
                match result {
                    Ok(()) => {
                        self.git_state
                            .cache
                            .invalidate(&project, &Invalidation::repository());
                        self.toasts.success(
                            presentation::message(&self.locale, "git.tagCreated", &[]),
                            now,
                        );
                    }
                    Err(error) => self.toasts.ipc_error(&self.locale, &error, now),
                }
                context.request_repaint();
            }
            Reply::Mutated {
                project,
                op: MutationOp::TagDelete,
                result,
            } => {
                let now = Instant::now();
                match result {
                    Ok(()) => {
                        self.git_state
                            .cache
                            .invalidate(&project, &Invalidation::repository());
                        self.toasts.success(
                            presentation::message(&self.locale, "git.tagDeleted", &[]),
                            now,
                        );
                    }
                    Err(error) => self.toasts.ipc_error(&self.locale, &error, now),
                }
                context.request_repaint();
            }
            Reply::Reverted {
                project,
                opens_conflict,
                result,
            } => {
                self.finish_revert(project, opens_conflict, result);
                context.request_repaint();
            }
            Reply::ConflictResolved {
                project,
                snapshot,
                result,
            } => {
                self.finish_conflict_resolution(context, &project, snapshot, result);
                context.request_repaint();
            }
        }
    }

    /// `commit-graph.tsx` `handleRevert` and `git.revertHead`: a clean revert says so, a conflicted
    /// one warns and, from the graph, opens the first conflicted file.
    fn finish_revert(
        &mut self,
        project: ProjectId,
        opens_conflict: bool,
        result: AppResult<RevertOutcome>,
    ) {
        let now = Instant::now();
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(error) => {
                self.toasts.ipc_error(&self.locale, &error, now);
                return;
            }
        };
        self.git_state
            .cache
            .invalidate(&project, &Invalidation::repository());
        if !outcome.conflicted {
            self.toasts.success(
                presentation::message(&self.locale, "git.revertSuccess", &[]),
                now,
            );
            return;
        }
        self.toasts.warning(
            presentation::message(&self.locale, "git.revertConflict", &[]),
            now,
        );
        if let Some(path) = outcome
            .conflicted_abs_paths
            .into_iter()
            .next()
            .filter(|_| opens_conflict)
        {
            self.submit(HostCommand::Editor(
                crate::domains::editor::Command::OpenFileTab {
                    project,
                    pane: None,
                    path,
                    preview: false,
                },
            ));
        }
    }

    fn commit_window(&self, slot: &ShellSlotId) -> String {
        self.git_state
            .commit
            .window(slot, crate::application::WINDOW_LABEL)
            .to_owned()
    }

    fn commit_gate(&self, project: &ProjectId) -> CommitGate {
        crate::git_scm_model::commit_gate(
            self.git_state
                .cache
                .status_result(project)
                .and_then(Result::ok)
                .map_or(&[][..], |status| status.rows.as_slice()),
        )
    }

    /// `git-panel-container.tsx` `handleCommit`: commits the slot window's draft, staging
    /// everything first when the confirmation asked to.
    fn commit(&mut self, project: &ProjectId, slot: &ShellSlotId, stage_all: bool) {
        let window = self.commit_window(slot);
        let message = self.git_state.commit.draft(&window, project).to_owned();
        if self.git_state.commit.committing.insert(project.clone())
            && !self.submit(HostCommand::Git(Command::Commit {
                project: project.clone(),
                message,
                stage_all,
                window,
            }))
        {
            self.git_state.commit.committing.remove(project);
        }
    }

    /// `handleGenerateCommitMessage`: a click while generating cancels, otherwise it asks for a
    /// message from the staged diff and the newest commits' summaries.
    fn toggle_commit_message(&mut self, project: ProjectId, slot: ShellSlotId) {
        if let Some(request) = self.git_state.commit.take(&slot) {
            self.submit(HostCommand::Git(Command::AiCancel {
                owner: request.owner,
                request_id: request.id,
            }));
            if request.project == project {
                return;
            }
        }
        let owner = self.commit_window(&slot);
        let id = uuid::Uuid::new_v4().to_string();
        let request = AiCommitMessageRequest {
            request_id: id.clone(),
            owner: owner.clone(),
            provider: None,
            model: None,
            diff_text: String::new(),
            recent_commits: crate::git_commit::recent_commits(self.git_state.cache.log(&project)),
        };
        if self.submit(HostCommand::Git(Command::AiCommitMessage {
            project: project.clone(),
            request,
            port: self.git_state.commit.ai.clone(),
        })) {
            self.git_state
                .commit
                .begin(slot, crate::git_commit::Request { project, id, owner });
        }
    }

    fn finish_commit_message(
        &mut self,
        request_id: &str,
        result: AppResult<(StagedDiffText, AiTextResponse)>,
    ) {
        let Some((slot, request)) = self.git_state.commit.finish(request_id) else {
            return;
        };
        if self
            .git_state
            .panels
            .panel(&request.project, &slot)
            .is_none()
        {
            return;
        }
        let now = Instant::now();
        let (diff, text) = match result {
            Ok((
                diff,
                AiTextResponse {
                    text: Some(text), ..
                },
            )) => (diff, text),
            Ok(_) => return,
            Err(error) => {
                self.toasts.notify(
                    taide_native_ui::toast::Kind::Error,
                    presentation::message(&self.locale, "git.generateCommitMessageFailed", &[]),
                    taide_native_ui::toast::Options {
                        description: Some(crate::toast::describe_error(&self.locale, &error)),
                        ..Default::default()
                    },
                    now,
                );
                return;
            }
        };
        let message = crate::git_commit::sanitize(&text);
        if message.is_empty() {
            self.toasts.error(
                presentation::message(&self.locale, "git.commitMessageEmptyResponse", &[]),
                now,
            );
            return;
        }
        self.git_state
            .commit
            .write(&request.owner, &request.project, message);
        self.toasts.notify(
            taide_native_ui::toast::Kind::Success,
            presentation::message(&self.locale, "git.commitMessageGenerated", &[]),
            taide_native_ui::toast::Options {
                description: crate::git_commit::notices(&self.locale, &diff),
                ..Default::default()
            },
            now,
        );
    }

    pub(crate) fn apply_git_intent(
        &mut self,
        intent: Intent,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
    ) {
        match intent {
            Intent::ShowView { project, slot } => {
                self.apply_explorer_search_intent(
                    taide_native_ui::intents::explorer_search::Intent::ShowExplorer {
                        project: project.clone(),
                        slot: slot.clone(),
                    },
                    context,
                    snapshot,
                );
                self.explorer_search_state
                    .symbol_sidebars
                    .entry(&project, &slot)
                    .view = crate::symbol_sidebar::View::Git;
            }
            Intent::ShowAuxiliaryView { project, slot } => {
                self.apply_explorer_search_intent(
                    taide_native_ui::intents::explorer_search::Intent::ShowAuxiliaryExplorer {
                        project: project.clone(),
                        slot,
                    },
                    context,
                    snapshot,
                );
                if let Some(explorer) = self
                    .shell_state
                    .auxiliary_windows
                    .find(&project, slot)
                    .map(|window| window.explorer.clone())
                {
                    self.explorer_search_state
                        .symbol_sidebars
                        .entry(&project, &explorer)
                        .view = crate::symbol_sidebar::View::Git;
                }
            }
            Intent::Shown { project, slot } => {
                let settings = &self.services.state.settings;
                self.git_state.panels.shown(&project, &slot, || {
                    Collapsed::from_ids(&settings.read().git_sections_collapsed)
                });
                let now = Instant::now();
                for key in [
                    Key::Status,
                    Key::Remotes,
                    Key::Branches,
                    Key::Stashes,
                    Key::Log,
                ] {
                    self.git_state.cache.observe(&project, key, now);
                }
                let cache = &mut self.git_state.cache;
                let log = cache.log(&project);
                let is_graph_shown = !log.is_empty()
                    && self
                        .git_state
                        .panels
                        .collapsed(&project, &slot)
                        .is_some_and(|collapsed| !collapsed.is_collapsed(Section::Graph));
                if is_graph_shown {
                    let selected = self
                        .git_state
                        .panels
                        .panel(&project, &slot)
                        .and_then(|panel| panel.selected_commit())
                        .filter(|selected| log.iter().any(|commit| commit.id == *selected))
                        .map(|rev| Key::CommitFiles {
                            rev: rev.to_owned(),
                        });
                    for key in std::iter::once(Key::Tags).chain(selected) {
                        cache.observe(&project, key, now);
                    }
                }
            }
            Intent::ToggleSection {
                project,
                slot,
                section,
            } => {
                let settings = &self.services.state.settings;
                let collapsed = self.git_state.panels.toggle(&project, &slot, section, || {
                    Collapsed::from_ids(&settings.read().git_sections_collapsed)
                });
                self.submit(HostCommand::Git(Command::PatchSettings(
                    taide_model::settings::SettingsPatch {
                        git_sections_collapsed: Some(collapsed.ids()),
                        ..Default::default()
                    },
                )));
                context.request_repaint();
            }
            Intent::Init(project) => {
                if self.git_state.initializing.insert(project.clone())
                    && !self.submit(HostCommand::Git(Command::Init {
                        project: project.clone(),
                        init: self.git_state.init.clone(),
                    }))
                {
                    self.git_state.initializing.remove(&project);
                }
                context.request_repaint();
            }
            Intent::Stage { project, paths } => {
                self.submit(HostCommand::Git(Command::Stage { project, paths }));
            }
            Intent::Unstage { project, paths } => {
                self.submit(HostCommand::Git(Command::Unstage { project, paths }));
            }
            Intent::RequestDiscard {
                project,
                slot,
                paths,
            } => {
                self.git_state
                    .panels
                    .request_discard(&project, &slot, paths);
                context.request_repaint();
            }
            Intent::ResolveDiscard {
                project,
                slot,
                confirmed,
            } => {
                if let Some(paths) = self.git_state.panels.take_discard(&project, &slot)
                    && confirmed
                {
                    self.submit(HostCommand::Git(Command::Discard { project, paths }));
                }
                context.request_repaint();
            }
            Intent::OpenFile { project, path } => {
                self.submit(HostCommand::Editor(
                    crate::domains::editor::Command::OpenFileTab {
                        project,
                        pane: None,
                        path,
                        preview: false,
                    },
                ));
            }
            Intent::OpenChanges {
                project,
                path,
                staged,
                before_path,
            } => {
                let title = format!(
                    "{} (diff)",
                    std::path::Path::new(&path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(&path)
                );
                self.submit(HostCommand::Git(Command::OpenTab {
                    project,
                    kind: TabKind::Diff {
                        path,
                        staged,
                        compare_with: None,
                        rev: None,
                        parent_rev: None,
                        before_path,
                    },
                    title,
                    target: None,
                    preview: false,
                }));
            }
            Intent::Select {
                project,
                slot,
                selected,
                anchor,
            } => {
                self.git_state
                    .panels
                    .select(&project, &slot, selected, anchor);
                context.request_repaint();
            }
            Intent::RowMenu {
                project,
                slot,
                target,
            } => {
                self.git_state.panels.open_menu(&project, &slot, target);
                context.request_repaint();
            }
            Intent::CheckoutBranch { project, name } => {
                self.submit(HostCommand::Git(Command::BranchCheckout { project, name }));
            }
            Intent::CheckoutRemoteBranch {
                project,
                remote_ref,
            } => {
                self.submit(HostCommand::Git(Command::CheckoutRemote {
                    project,
                    remote_ref,
                }));
            }
            Intent::CreateBranch { project, name } => {
                self.submit(HostCommand::Git(Command::BranchCreate { project, name }));
            }
            Intent::Sync(project) => {
                if self.git_state.syncing.insert(project.clone())
                    && !self.submit(HostCommand::Git(Command::Pull(project.clone())))
                {
                    self.git_state.syncing.remove(&project);
                }
                context.request_repaint();
            }
            Intent::StashPush(project) => {
                self.submit_stash(project.clone(), Command::StashPush(project));
                context.request_repaint();
            }
            Intent::StashApply { project, index } => {
                self.submit_stash(project.clone(), Command::StashApply { project, index });
                context.request_repaint();
            }
            Intent::StashDrop { project, index } => {
                self.submit_stash(project.clone(), Command::StashDrop { project, index });
                context.request_repaint();
            }
            Intent::SetCommitMessage {
                project,
                slot,
                message,
            } => {
                let window = self.commit_window(&slot);
                self.git_state.commit.write(&window, &project, message);
                context.request_repaint();
            }
            Intent::RequestCommit { project, slot } => {
                match self.commit_gate(&project) {
                    CommitGate::BlockedByConflicts => {}
                    CommitGate::ConfirmStageAll => {
                        self.git_state.panels.request_stage_all(&project, &slot)
                    }
                    CommitGate::Commit => self.commit(&project, &slot, false),
                }
                context.request_repaint();
            }
            Intent::ResolveStageAll {
                project,
                slot,
                confirmed,
            } => {
                if self.git_state.panels.take_stage_all(&project, &slot) && confirmed {
                    match self.commit_gate(&project) {
                        CommitGate::BlockedByConflicts => {}
                        gate => self.commit(&project, &slot, gate == CommitGate::ConfirmStageAll),
                    }
                }
                context.request_repaint();
            }
            Intent::GenerateCommitMessage { project, slot } => {
                self.toggle_commit_message(project, slot);
                context.request_repaint();
            }
            Intent::DiffShown {
                project,
                tab,
                path,
                staged,
                compare_with,
                before_path,
                rev,
                parent_rev,
            } => {
                self.git_state.diffs.shown(tab);
                let now = Instant::now();
                let key = match (rev, compare_with) {
                    (Some(rev), _) => Key::CommitDiff {
                        rev,
                        parent: parent_rev,
                        before_path: before_path.unwrap_or_else(|| path.clone()),
                        path,
                    },
                    (None, Some(original)) => Key::Compare {
                        original,
                        modified: path,
                    },
                    (None, None) => {
                        self.git_state.cache.observe(&project, Key::Status, now);
                        Key::Diff {
                            path,
                            staged,
                            before_path,
                        }
                    }
                };
                self.git_state.cache.observe(&project, key, now);
            }
            Intent::ToggleDiffSideBySide(tab) => {
                self.git_state.diffs.toggle(&tab);
                context.request_repaint();
            }
            Intent::StageHunk {
                project,
                path,
                start,
                end,
                staged,
            } => {
                self.submit(HostCommand::Git(if staged {
                    Command::UnstageHunk {
                        project,
                        path,
                        start,
                        end,
                    }
                } else {
                    Command::StageHunk {
                        project,
                        path,
                        start,
                        end,
                    }
                }));
            }
            Intent::OpenTabChanges {
                project,
                path,
                title,
                window,
            } => {
                let target = snapshot
                    .layouts
                    .get(&project)
                    .and_then(|layout| match window {
                        None => Some(layout.focused_pane.clone()),
                        Some(slot) => layout
                            .auxiliary_windows
                            .iter()
                            .find(|window| window.slot == slot)
                            .map(|window| window.focused_pane.clone()),
                    });
                self.submit(HostCommand::Git(Command::OpenTab {
                    project,
                    kind: TabKind::Diff {
                        path,
                        staged: false,
                        compare_with: None,
                        rev: None,
                        parent_rev: None,
                        before_path: None,
                    },
                    title,
                    target,
                    preview: true,
                }));
            }
            Intent::SelectCommit {
                project,
                slot,
                commit,
            } => {
                self.git_state.panels.select_commit(&project, &slot, commit);
                context.request_repaint();
            }
            Intent::GraphMenu {
                project,
                slot,
                commit,
            } => {
                self.git_state
                    .panels
                    .open_graph_menu(&project, &slot, commit);
                context.request_repaint();
            }
            Intent::Revert { project, rev } => {
                self.submit(HostCommand::Git(Command::Revert {
                    project,
                    rev,
                    opens_conflict: true,
                }));
            }
            Intent::RevertHead(project) => {
                self.submit(HostCommand::Git(Command::Revert {
                    project,
                    rev: HEAD.to_owned(),
                    opens_conflict: false,
                }));
            }
            Intent::RequestCreateTag {
                project,
                slot,
                target,
            } => {
                self.request_tag(&project, &slot, target);
                context.request_repaint();
            }
            Intent::CreateTagOnHead { project, slot } => {
                self.apply_git_intent(
                    Intent::ShowView {
                        project: project.clone(),
                        slot: slot.clone(),
                    },
                    context,
                    snapshot,
                );
                self.request_tag(&project, &slot, HEAD.to_owned());
                context.request_repaint();
            }
            Intent::CreateAuxiliaryTagOnHead { project, slot } => {
                self.apply_git_intent(
                    Intent::ShowAuxiliaryView {
                        project: project.clone(),
                        slot,
                    },
                    context,
                    snapshot,
                );
                if let Some(explorer) = self
                    .shell_state
                    .auxiliary_windows
                    .find(&project, slot)
                    .map(|window| window.explorer.clone())
                {
                    self.request_tag(&project, &explorer, HEAD.to_owned());
                }
                context.request_repaint();
            }
            Intent::CloseTagDialog { project, slot } => {
                self.git_state.panels.close_tag(&project, &slot);
                context.request_repaint();
            }
            Intent::CreateTag {
                project,
                slot,
                name,
                message,
            } => {
                if let Some(target) = self.git_state.panels.begin_tag(&project, &slot)
                    && !self.submit(HostCommand::Git(Command::TagCreate {
                        project: project.clone(),
                        slot: slot.clone(),
                        name,
                        target,
                        message: (!message.is_empty()).then_some(message),
                    }))
                {
                    self.git_state.panels.finish_tag(&project, &slot, false);
                }
                context.request_repaint();
            }
            Intent::DeleteTag { project, name } => {
                self.submit(HostCommand::Git(Command::TagDelete { project, name }));
            }
            Intent::ResizeGraph {
                project,
                slot,
                height,
            } => {
                if let Some(collapsed) =
                    self.git_state
                        .panels
                        .resize_graph(&project, &slot, height, Instant::now())
                {
                    self.submit(HostCommand::Git(Command::PatchSettings(
                        taide_model::settings::SettingsPatch {
                            git_sections_collapsed: Some(collapsed.ids()),
                            ..Default::default()
                        },
                    )));
                }
                context.request_repaint();
            }
            Intent::OpenCommitFile {
                project,
                kind,
                title,
            } => {
                self.submit(HostCommand::Git(Command::OpenTab {
                    project,
                    kind,
                    title,
                    target: None,
                    preview: true,
                }));
            }
            Intent::ClaudeDiffShown {
                project,
                tab,
                request_id,
                path,
                revisions,
            } => {
                let now = Instant::now();
                self.git_state.diffs.shown(tab.clone());
                self.git_state
                    .cache
                    .observe(&project, Key::File { path }, now);
                self.git_state
                    .claude
                    .shown(&request_id, tab, revisions, now);
            }
            Intent::ResolveClaudeDiff {
                tab,
                request_id,
                content,
            } => {
                if !self.git_state.claude.begin_resolve(&request_id) {
                    return;
                }
                let outcome = match content {
                    Some(_) => IdeDiffOutcome::Saved,
                    None => IdeDiffOutcome::Rejected,
                };
                if !self.submit(HostCommand::Git(Command::ResolveIdeDiff {
                    tab: Some(tab),
                    request_id: request_id.clone(),
                    outcome,
                    content,
                })) {
                    self.git_state.claude.finish_resolve(&request_id);
                }
                context.request_repaint();
            }
            Intent::OpenFileHistory { project, path } => {
                self.git_state
                    .histories
                    .open(context.viewport_id(), project, path);
                context.request_repaint();
            }
            intent @ (Intent::EditorShown { .. }
            | Intent::EditorLanePress { .. }
            | Intent::ResolveHunkDiscard { .. }
            | Intent::ResolveConflict { .. }
            | Intent::CloseConflictCompare { .. }
            | Intent::ToggleBlame { .. }
            | Intent::StageSelection { .. }) => self.apply_git_editor_intent(intent, context),
        }
    }

    /// `ide-sync-provider.tsx`: records each IDE diff request and opens its tab, or rejects it at
    /// once when automatic diff tabs are off; a close-tab notice drops its request.
    fn consume_ide_diff_events(&mut self) {
        for event in self.git_state.cache_changes.ide.take() {
            match event {
                crate::git_claude_diff::Event::Requested {
                    request_id,
                    project,
                    path,
                    contents,
                    title,
                } => {
                    self.git_state.claude.insert(request_id.clone(), contents);
                    if !self.services.state.settings.read().ide_auto_open_diff {
                        self.git_state.claude.remove(&request_id);
                        self.submit(HostCommand::Git(Command::ResolveIdeDiff {
                            tab: None,
                            request_id,
                            outcome: IdeDiffOutcome::Rejected,
                            content: None,
                        }));
                        continue;
                    }
                    self.submit(HostCommand::Git(Command::OpenTab {
                        project,
                        kind: TabKind::ClaudeDiff { request_id, path },
                        title,
                        target: None,
                        preview: false,
                    }));
                }
                crate::git_claude_diff::Event::Closed(request_id) => {
                    self.git_state.claude.remove(&request_id)
                }
            }
        }
    }

    fn tick_claude_diffs(
        &mut self,
        context: &egui::Context,
        shown: &HashSet<TabId>,
        open_projects: &HashSet<ProjectId>,
    ) {
        self.consume_ide_diff_events();
        let open_tabs = self
            .services
            .state
            .layouts
            .read()
            .values()
            .flat_map(taide_layout::service::all_roots)
            .flat_map(taide_layout::service::collect_claude_diff_tab_ids)
            .collect::<HashSet<_>>();
        self.git_state.claude.retain(shown, &open_tabs);
        let windows = std::iter::once(egui::ViewportId::ROOT)
            .chain(
                self.shell_state
                    .auxiliary_windows
                    .windows()
                    .iter()
                    .map(|window| window.viewport),
            )
            .collect::<HashSet<_>>();
        self.git_state.histories.retain(&windows, open_projects);
        let now = Instant::now();
        for (tab, revisions) in self.git_state.claude.due(now) {
            let texts = VIRTUAL_SIDES.map(|side| {
                self.store
                    .documents()
                    .find(&DocumentKey::Virtual(VirtualKey {
                        owner: tab.clone(),
                        side,
                    }))
                    .and_then(|document| self.store.documents().snapshot(document).ok())
                    .filter(|_| claude_revisions(&self.store, &tab) == Some(revisions))
                    .map(|snapshot| snapshot.rope.to_string())
            });
            let [Some(original), Some(modified)] = texts else {
                self.git_state.claude.cancel(&tab);
                continue;
            };
            if !self.submit(HostCommand::Git(Command::Hunks {
                tab: tab.clone(),
                revisions,
                original,
                modified,
            })) {
                self.git_state.claude.cancel(&tab);
                context.request_repaint();
            }
        }
        if let Some(wait) = self.git_state.claude.next_due(now) {
            context.request_repaint_after(wait);
        }
    }

    /// The drawn window's `FileHistoryPanel`: a right docked dialog with the file's commits, or one
    /// commit's inline file diff in their place.
    pub(crate) fn show_git_file_history(&mut self, context: &egui::Context) {
        let window = context.viewport_id();
        let (Some(panel), Some(appearance), Some(diff_appearance)) = (
            self.git_state.histories.get(window),
            self.git_state.history_appearance,
            self.git_state.diff_appearance,
        ) else {
            return;
        };
        let now = Instant::now();
        let (project, owner, pane) = (
            panel.project.clone(),
            panel.owner.clone(),
            panel.pane.clone(),
        );
        let log_key = Key::FileLog {
            path: panel.path.clone(),
        };
        let diff_key = panel.diff_key();
        self.git_state.cache.observe(&project, log_key, now);
        if let Some(key) = &diff_key {
            self.git_state.cache.observe(&project, key.clone(), now);
            self.git_state.diffs.shown(owner.clone());
        }
        let git = &self.git_state;
        let Some(panel) = git.histories.get(window) else {
            return;
        };
        let view = crate::git_history::View {
            panel,
            entries: git.cache.file_log(&project, &panel.path),
            appearance,
            locale: &self.locale,
            tooltips: &self.tooltips,
            tooltip_appearance: &self.tooltip_appearance,
            now_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0.0, |elapsed| elapsed.as_secs_f64()),
        };
        let icons = &mut self.explorer_search_state.navigation_icons;
        let store = &mut self.store;
        let parts = Parts {
            store,
            editor: &self.editor,
            syntax: &mut self.editor_syntax,
            services: &self.services,
            display: self.editor_display_colors,
            brackets: self.editor_bracket_colors,
        };
        let hide_unchanged = self
            .services
            .state
            .settings
            .read()
            .editor_diff_hide_unchanged_regions;
        let content = context.content_rect();
        let size = egui::vec2(
            crate::git_history::PANEL_WIDTH.min(content.width()),
            content.height(),
        );
        let id = egui::Id::new(("taide-file-history", window));
        let modal = crate::git_history::modal(id, context, appearance);
        let response = modal.show(
            context,
            |ui| -> AppResult<Option<crate::git_history::Action>> {
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let (body, action) = crate::git_history::header(ui, rect, &view, icons)?;
                let Some(key) = &diff_key else {
                    return Ok(crate::git_history::list(ui, body, &view).or(action));
                };
                let Some(diff) =
                    loaded_diff(ui, git, &self.locale, &project, key, body, diff_appearance)
                else {
                    return Ok(action);
                };
                let fresh = take_fresh(ui, &owner, git.cache.fetched_at(&project, key));
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
                child.set_clip_rect(body.intersect(ui.clip_rect()));
                show_surface(
                    &mut child,
                    parts,
                    Surface {
                        owner: &owner,
                        pane: &pane,
                        language_id: &diff.sides.language_id,
                        sides: [
                            SideText::Read {
                                text: &diff.sides.original,
                                fresh,
                            },
                            SideText::Read {
                                text: &diff.sides.modified,
                                fresh,
                            },
                        ],
                        hunks: &diff.hunks,
                        side_by_side: false,
                        stage_lanes: false,
                        hide_unchanged,
                    },
                    diff_appearance.colors,
                )?;
                Ok(action)
            },
        );
        let is_closing = response.backdrop_response.clicked()
            || taide_native_ui::modal::takes_escape(context, &response);
        let action = match response.inner {
            Ok(action) => action,
            Err(error) => {
                self.status = Some(error.to_string());
                None
            }
        };
        match (is_closing, action) {
            (true, _) => self
                .git_state
                .histories
                .apply(window, crate::git_history::Action::Close),
            (false, Some(action)) => self.git_state.histories.apply(window, action),
            (false, None) => return,
        }
        context.request_repaint();
    }

    fn request_tag(&mut self, project: &ProjectId, slot: &ShellSlotId, target: String) {
        let settings = &self.services.state.settings;
        self.git_state
            .panels
            .request_tag(project, slot, target, || {
                Collapsed::from_ids(&settings.read().git_sections_collapsed)
            });
    }

    fn submit_stash(&mut self, project: ProjectId, command: Command) {
        *self.git_state.stashing.entry(project.clone()).or_default() += 1;
        if !self.submit(HostCommand::Git(command)) {
            finish_stash(&mut self.git_state.stashing, &project);
        }
    }

    pub(crate) fn reconnect_git(&mut self) {
        self.git_state.title_branches.cancel_fetches();
        self.git_state.cache.cancel_fetches();
        self.git_state.initializing.clear();
        self.git_state.stashing.clear();
        self.git_state.syncing.clear();
        self.git_state.commit.committing.clear();
        self.git_state.commit.clear_requests();
        self.git_state.panels.cancel_pending_tags();
        self.git_state.claude.reconnect();
    }

    pub(crate) fn tick_git_title_branches(
        &mut self,
        context: &egui::Context,
        snapshot: &taide_native_ui::snapshot::ShellSnapshot,
        open_projects: &HashSet<ProjectId>,
    ) {
        self.git_state
            .title_branches
            .invalidate(self.git_state.title_branch_changes.take());
        self.git_state.title_branches.retain(open_projects);
        if self.shell.has_title_bar {
            let shown = snapshot.focused_project().cloned().into_iter().chain(
                self.shell_state
                    .auxiliary_windows
                    .windows()
                    .iter()
                    .map(|window| window.project.clone()),
            );
            for project in self.git_state.title_branches.requests(shown) {
                if !self.submit(HostCommand::Git(Command::GitBranch(project.clone()))) {
                    self.git_state.title_branches.retry(&project);
                    context.request_repaint();
                }
            }
        }
        self.tick_git_cache(context, open_projects);
    }

    fn tick_git_cache(&mut self, context: &egui::Context, open_projects: &HashSet<ProjectId>) {
        for (project, invalidation) in self.git_state.cache_changes.take() {
            self.git_state.cache.invalidate(&project, &invalidation);
        }
        self.git_state.cache.retain(open_projects);
        self.git_state
            .initializing
            .retain(|project| open_projects.contains(project));
        self.git_state
            .stashing
            .retain(|project, _| open_projects.contains(project));
        self.git_state
            .syncing
            .retain(|project| open_projects.contains(project));
        self.git_state.panels.finish_frame();
        let now = Instant::now();
        for size in self.git_state.panels.take_due_resizes(now) {
            self.submit(HostCommand::Git(Command::PatchSettings(
                taide_model::settings::SettingsPatch {
                    git_graph_panel_size_px: Some(size),
                    ..Default::default()
                },
            )));
        }
        if let Some(wait) = self.git_state.panels.next_resize(now) {
            context.request_repaint_after(wait);
        }
        self.git_state.editors.finish_frame();
        if self.git_state.editors.appearance.is_none() || !self.git_state.editors.is_empty() {
            let theme = self
                .settings_state
                .preview_theme
                .as_ref()
                .unwrap_or(&self.settings_state.resolved_theme);
            match crate::git_gutter::Appearance::new(theme) {
                Ok(appearance) => self.git_state.editors.appearance = Some(appearance),
                Err(error) => self.status = Some(error.to_string()),
            }
        }
        let shown_diffs = self.git_state.diffs.finish_frame();
        if let Err(error) = crate::git_diff::release_hidden(&mut self.store, &shown_diffs) {
            self.status = Some(crate::application::editor_error(error).to_string());
        }
        if self.git_state.diff_appearance.is_none()
            || !shown_diffs.is_empty()
            || !self.git_state.histories.is_empty()
        {
            let theme = self
                .settings_state
                .preview_theme
                .as_ref()
                .unwrap_or(&self.settings_state.resolved_theme);
            match crate::git_diff::Appearance::new(theme)
                .and_then(|diff| Ok((diff, crate::git_claude_diff::Appearance::new(theme)?)))
                .and_then(|(diff, claude)| {
                    Ok((diff, claude, crate::git_history::Appearance::new(theme)?))
                }) {
                Ok((diff, claude, history)) => {
                    self.git_state.diff_appearance = Some(diff);
                    self.git_state.claude_appearance = Some(claude);
                    self.git_state.history_appearance = Some(history);
                }
                Err(error) => self.status = Some(error.to_string()),
            }
        }
        self.tick_claude_diffs(context, &shown_diffs, open_projects);
        self.git_state
            .commit
            .committing
            .retain(|project| open_projects.contains(project));
        self.git_state.commit.set_windows(
            crate::application::WINDOW_LABEL,
            self.shell_state
                .auxiliary_windows
                .windows()
                .iter()
                .map(|window| (window.explorer.clone(), window.label.clone())),
        );
        let panels = &self.git_state.panels;
        for request in self
            .git_state
            .commit
            .abandon(|project, slot| panels.panel(project, slot).is_some())
        {
            self.submit(HostCommand::Git(Command::AiCancel {
                owner: request.owner,
                request_id: request.id,
            }));
        }
        if self.git_state.appearance.is_none() || !self.git_state.panels.is_empty() {
            let theme = self
                .settings_state
                .preview_theme
                .as_ref()
                .unwrap_or(&self.settings_state.resolved_theme);
            match crate::git_scm::Appearance::new(theme) {
                Ok(appearance) => self.git_state.appearance = Some(appearance),
                Err(error) => self.status = Some(error.to_string()),
            }
        }
        for request in self.git_state.cache.requests(Instant::now()) {
            if !self.submit(HostCommand::Git(Command::Read(request.clone()))) {
                context.request_repaint();
                break;
            }
            self.git_state.cache.begin(&request);
        }
    }
}

impl GitSurfaces for AppSurfaces<'_> {
    fn branch(&self, project: &ProjectId) -> Option<&str> {
        self.title_branches.branch(project)
    }
    fn title_branch_icon(&mut self, ui: &Ui, rect: egui::Rect, color: egui::Color32) {
        if let Err(error) = self.explorer_icons.paint(
            ui,
            rect,
            crate::problems_icons::Glyph::GitBranch,
            color,
            0.0,
        ) {
            *self.status = Some(error.to_string());
        }
    }
}

impl AppSurfaces<'_> {
    /// `pane-node-view.tsx` diff branch: a working tree/staged diff or a file compare
    /// (`DiffPane`), or one file of a commit (`CommitFileDiff`, side-by-side without Alt+\).
    pub(crate) fn show_diff_tab(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        pane: &PaneId,
        tab: &Tab,
        intents: &mut Vec<ShellIntent>,
    ) {
        let TabKind::Diff {
            path,
            staged,
            compare_with,
            rev,
            parent_rev,
            before_path,
        } = &tab.kind
        else {
            self.show_unavailable_tab(ui, tab);
            return;
        };
        intents.push(ShellIntent::Git(Intent::DiffShown {
            project: project.clone(),
            tab: tab.id.clone(),
            path: path.clone(),
            staged: *staged,
            compare_with: compare_with.clone(),
            before_path: before_path.clone(),
            rev: rev.clone(),
            parent_rev: parent_rev.clone(),
        }));
        let Some(appearance) = self.git.diff_appearance else {
            return;
        };
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, appearance.background);
        let key = match (rev, compare_with) {
            (Some(rev), _) => Key::CommitDiff {
                rev: rev.clone(),
                parent: parent_rev.clone(),
                path: path.clone(),
                before_path: before_path.clone().unwrap_or_else(|| path.clone()),
            },
            (None, Some(original)) => Key::Compare {
                original: original.clone(),
                modified: path.clone(),
            },
            (None, None) => Key::Diff {
                path: path.clone(),
                staged: *staged,
                before_path: before_path.clone(),
            },
        };
        let Some(diff) = loaded_diff(ui, self.git, self.locale, project, &key, rect, appearance)
        else {
            return;
        };
        if rev.is_none()
            && Some(pane) == self.target
            && crate::git_diff::take_side_by_side_toggle(ui)
        {
            intents.push(ShellIntent::Git(Intent::ToggleDiffSideBySide(
                tab.id.clone(),
            )));
        }
        let shown = DiffTab {
            project,
            pane,
            tab,
            path,
            staged: *staged,
            compare_with: compare_with.as_deref(),
            is_commit: rev.is_some(),
            fetched_at: self.git.cache.fetched_at(project, &key),
        };
        if let Err(error) = self.show_diff(ui, shown, diff, appearance, intents) {
            *self.status = Some(error.to_string());
        }
    }

    fn show_diff(
        &mut self,
        ui: &mut Ui,
        shown: DiffTab<'_>,
        diff: &crate::git_cache::Diff,
        appearance: crate::git_diff::Appearance,
        intents: &mut Vec<ShellIntent>,
    ) -> AppResult<()> {
        let git = self.git;
        let fresh = take_fresh(ui, &shown.tab.id, shown.fetched_at);
        let rows = git
            .cache
            .status_result(shown.project)
            .and_then(Result::ok)
            .map_or(&[][..], |status| status.rows.as_slice());
        let hide_unchanged = self
            .services
            .state
            .settings
            .read()
            .editor_diff_hide_unchanged_regions;
        let (output, _) = show_surface(
            ui,
            Parts {
                store: self.store,
                editor: self.editor,
                syntax: self.editor_syntax,
                services: self.services,
                display: self.editor_display_colors,
                brackets: self.editor_bracket_colors,
            },
            Surface {
                owner: &shown.tab.id,
                pane: shown.pane,
                language_id: &diff.sides.language_id,
                sides: [
                    SideText::Read {
                        text: &diff.sides.original,
                        fresh,
                    },
                    SideText::Read {
                        text: &diff.sides.modified,
                        fresh,
                    },
                ],
                hunks: &diff.hunks,
                side_by_side: shown.is_commit || git.diffs.is_side_by_side(&shown.tab.id),
                stage_lanes: !shown.is_commit
                    && crate::git_diff::is_stageable(shown.compare_with, shown.path, rows),
                hide_unchanged,
            },
            appearance.colors,
        )?;
        if let Some(range) = output.lane_press {
            intents.push(ShellIntent::Git(Intent::StageHunk {
                project: shown.project.clone(),
                path: shown.path.to_owned(),
                start: *range.start(),
                end: *range.end(),
                staged: shown.staged,
            }));
        }
        Ok(())
    }

    /// `ClaudeDiffPane`: the accept/reject bar over the file on disk (read-only) against the
    /// requested text (editable), its lines recomputed after edits.
    pub(crate) fn show_claude_diff_tab(
        &mut self,
        ui: &mut Ui,
        project: &ProjectId,
        pane: &PaneId,
        tab: &Tab,
        intents: &mut Vec<ShellIntent>,
    ) {
        let TabKind::ClaudeDiff { request_id, path } = &tab.kind else {
            self.show_unavailable_tab(ui, tab);
            return;
        };
        let git = self.git;
        let (Some(appearance), Some(bar)) = (git.diff_appearance, git.claude_appearance) else {
            return;
        };
        let rect = ui.available_rect_before_wrap();
        ui.painter().rect_filled(rect, 0.0, appearance.background);
        let bar_rect = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width(), crate::git_claude_diff::BAR_HEIGHT),
        );
        let labels = ["ide.rejectChanges", "ide.acceptChanges"]
            .map(|key| presentation::message(self.locale, key, &[]));
        let choice = crate::git_claude_diff::bar(
            ui,
            bar_rect,
            bar,
            [&labels[0], &labels[1]],
            git.claude.is_resolving(request_id),
        );
        let file_key = Key::File { path: path.clone() };
        let file = git.cache.file(project, path);
        let fresh = take_fresh(ui, &tab.id, git.cache.fetched_at(project, &file_key));
        let mut body = ui.new_child(egui::UiBuilder::new().max_rect(egui::Rect::from_min_max(
            egui::pos2(rect.left(), bar_rect.bottom()),
            rect.max,
        )));
        let shown = show_surface(
            &mut body,
            Parts {
                store: self.store,
                editor: self.editor,
                syntax: self.editor_syntax,
                services: self.services,
                display: self.editor_display_colors,
                brackets: self.editor_bracket_colors,
            },
            Surface {
                owner: &tab.id,
                pane,
                language_id: file.map_or(crate::git_claude_diff::FALLBACK_LANGUAGE_ID, |file| {
                    file.language_id.as_str()
                }),
                sides: [
                    SideText::Read {
                        text: file.map_or("", |file| file.content.as_str()),
                        fresh,
                    },
                    SideText::Editable(git.claude.contents(request_id)),
                ],
                hunks: git.claude.hunks(&tab.id),
                side_by_side: true,
                stage_lanes: false,
                hide_unchanged: false,
            },
            appearance.colors,
        );
        let modified = match shown {
            Ok((_, [_, modified])) => modified,
            Err(error) => {
                *self.status = Some(error.to_string());
                return;
            }
        };
        if let Some(revisions) = claude_revisions(self.store, &tab.id) {
            intents.push(ShellIntent::Git(Intent::ClaudeDiffShown {
                project: project.clone(),
                tab: tab.id.clone(),
                request_id: request_id.clone(),
                path: path.clone(),
                revisions,
            }));
        }
        if let Some(choice) = choice {
            intents.push(ShellIntent::Git(Intent::ResolveClaudeDiff {
                tab: tab.id.clone(),
                request_id: request_id.clone(),
                content: match choice {
                    crate::git_claude_diff::Choice::Reject => None,
                    crate::git_claude_diff::Choice::Accept => {
                        Some(self.store.documents().snapshot(modified).map_or_else(
                            |_| git.claude.contents(request_id).to_owned(),
                            |snapshot| snapshot.rope.to_string(),
                        ))
                    }
                },
            }));
        }
    }

    pub(crate) fn show_git_view(
        &self,
        ui: &mut Ui,
        project: &ProjectId,
        slot: &ShellSlotId,
        icons: &mut crate::navigation_icons::Deferred,
        errors: &mut Vec<String>,
        intents: &mut Vec<ShellIntent>,
    ) {
        intents.push(ShellIntent::Git(Intent::Shown {
            project: project.clone(),
            slot: slot.clone(),
        }));
        let Some(appearance) = self.git.appearance.as_ref() else {
            return;
        };
        let panel = self.git.panels.panel(project, slot);
        let settings = self.services.state.settings.read();
        let graph = crate::git_scm::Graph {
            log: self.git.cache.log(project),
            tags: self.git.cache.tags(project),
            commit_files: panel
                .and_then(|panel| panel.selected_commit())
                .and_then(|rev| self.git.cache.commit_files(project, rev)),
            size: settings.git_graph_panel_size_px as f32,
            resizer_thickness: settings.resizer_thickness as f32,
            now_unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0.0, |elapsed| elapsed.as_secs_f64()),
        };
        drop(settings);
        let view = crate::git_scm::View {
            project,
            slot,
            status: self.git.cache.status_result(project),
            collapsed: self.git.panels.collapsed(project, slot).unwrap_or_else(|| {
                Collapsed::from_ids(&self.services.state.settings.read().git_sections_collapsed)
            }),
            panel,
            is_initializing: self.git.initializing.contains(project),
            remotes: self.git.cache.remotes(project),
            branches: self.git.cache.branches(project),
            stashes: self.git.cache.stashes(project),
            is_stashing: self.git.stashing.contains_key(project),
            is_syncing: self.git.syncing.contains(project),
            commit: crate::git_scm::Commit {
                message: self.git.commit.draft(
                    self.git
                        .commit
                        .window(slot, crate::application::WINDOW_LABEL),
                    project,
                ),
                is_committing: self.git.commit.committing.contains(project),
                is_generating: self.git.commit.is_generating(project, slot),
            },
            graph,
            appearance,
            locale: self.locale,
            tooltips: self.tooltips,
            tooltip_appearance: self.tooltip_appearance,
        };
        if let Err(error) = crate::git_scm::show(ui, &view, icons, intents) {
            errors.push(error.to_string());
        }
    }
}

#[cfg(test)]
#[path = "../application-git-fixture.rs"]
pub(crate) mod fixture;

#[cfg(test)]
#[path = "../application-git-base-tests.rs"]
mod base_tests;

#[cfg(test)]
#[path = "../application-git-changes-tests.rs"]
mod changes_tests;

#[cfg(test)]
#[path = "../application-git-header-tests.rs"]
mod header_tests;

#[cfg(test)]
#[path = "../application-git-commit-tests.rs"]
mod commit_tests;

#[cfg(test)]
#[path = "../application-git-graph-tests.rs"]
mod graph_tests;

#[cfg(test)]
#[path = "../application-git-diff-tests.rs"]
mod diff_tests;

#[cfg(test)]
#[path = "../application-git-claude-history-tests.rs"]
mod claude_history_tests;

#[path = "../git-editor.rs"]
mod editor;
pub(crate) use editor::EditorFrame;

#[cfg(test)]
#[path = "../application-git-editor-tests.rs"]
mod editor_tests;
