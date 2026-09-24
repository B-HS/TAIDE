use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::domain::layout::types::ShellViewState;
use crate::error::{AppError, AppErrorKind, AppResult};
use crate::ids::{ProjectGroupId, ProjectId, ShellSlotId};
use crate::infra::clock::now_epoch_ms;
use crate::infra::home;
use crate::infra::persist;
use crate::paths::AppPaths;

use super::types::{
    CapabilityKind, ForgetRecentOutcome, Project, ProjectDisplay, ProjectDisplayPatch, ProjectGroup, ProjectRef, SessionShellState,
    SessionState, ShellSlotEdge, WindowChrome, WindowChromePatch,
};
use super::{groups, shell_slots};

const BACKUP_SUFFIX: &str = ".bak";
const PROJECTS_DIR_NAME: &str = "projects";
const DISPLAY_ICON_NAME_MAX_BYTES: usize = 64;
const DISPLAY_LABEL_MAX_CODEPOINTS: usize = 4;
/// The color tokens a project display may name, mirroring the theme system's `graph.lane1..lane12`
/// (`src/entities/theme/theme-tokens.ts`, `src/shared/styles/global.css`) — every bundled theme
/// already defines all twelve, so the sidebar can render one as `var(--taide-graph-laneN)` without
/// a new theme token. An explicit allow-list rather than a parsed `lane<N>` range: parsing would
/// also accept `lane01`/`lane+1`, which name no CSS variable.
const DISPLAY_COLOR_TOKENS: &[&str] = &[
    "lane1", "lane2", "lane3", "lane4", "lane5", "lane6", "lane7", "lane8", "lane9", "lane10", "lane11", "lane12",
];
/// Upper bound on a [`ProjectGroup`] name, measured in codepoints like
/// [`DISPLAY_LABEL_MAX_CODEPOINTS`]. A group name is a sidebar header, not prose — this keeps one
/// from pushing the project rail's layout around, and keeps `session.json` from growing a pasted
/// document.
const GROUP_NAME_MAX_CODEPOINTS: usize = 40;

pub use taide_model::project::ProjectOpenResult;

pub fn list_projects(session: &SessionState) -> Vec<ProjectRef> {
    session.projects.clone()
}

pub fn shell_state(session: &SessionState) -> SessionShellState {
    SessionShellState {
        tree: session.shell_slots.clone(),
        focused: session.focused_shell_slot.clone(),
        window_chrome: session.window_chrome,
    }
}

/// Re-derives `focused_shell_slot` and `active_project` from the slot tree, the single place those
/// three fields are reconciled after any structural change. The focus preference order is "keep the
/// current slot if it still exists → otherwise `successor`, the neighbour a caller picked out before
/// pruning → otherwise the slot showing the current active project → otherwise the first slot", so a
/// close that removed some *other* slot never moves the user's focus, while a close that removed the
/// focused one lands on the slot that was next to it.
///
/// `successor` is what closing the focused slot needs and nothing else supplies: its old id is gone
/// from the tree, no project points back at it, and the remaining fallback is the tree's *first*
/// slot — so closing the third of three slots used to jump focus (and, through it, the active
/// project, the native `File` menu and every ⌘P/⌘B target) all the way back to the first. Callers
/// compute it with [`shell_slots::successor_slot_after_prune`] before the prune and pass `None` when
/// the slot they removed was not the focused one; a candidate the prune also swallowed is ignored.
///
/// `active_project` is assigned from the focused slot rather than the other way round — that is the
/// d-62 redefinition of the field (see [`SessionState::active_project`]), and it is what keeps
/// `project_get_active`, the native `File` menu and the frontend's `activeProjectQueryOptions`
/// correct without any of them knowing that slots exist.
fn reconcile_focus(session: &mut SessionState, successor: Option<ShellSlotId>) {
    let Some(tree) = session.shell_slots.as_ref() else {
        session.focused_shell_slot = None;
        session.active_project = None;
        return;
    };

    let focused = session
        .focused_shell_slot
        .as_ref()
        .filter(|slot| shell_slots::contains_slot(tree, slot))
        .cloned()
        .or_else(|| successor.filter(|slot| shell_slots::contains_slot(tree, slot)))
        .or_else(|| {
            session
                .active_project
                .as_ref()
                .and_then(|project_id| shell_slots::slot_of_project(tree, project_id))
        })
        .or_else(|| shell_slots::first_slot(tree));

    session.active_project = focused.as_ref().and_then(|slot| shell_slots::project_of_slot(tree, slot));
    session.focused_shell_slot = focused;
}

/// The neighbour that inherits focus when the focused slot is about to be pruned, read while the
/// tree still records where that slot sat — [`shell_slots::successor_slot_after_prune`]'s "before
/// the prune" precondition. `focus_is_doomed` is the caller's answer to "is the focused slot one of
/// the ones this removal takes": a slot-id match for [`close_shell_slot`], a project match for
/// [`close_project`], which can take several slots at once. `false` yields `None`, leaving
/// [`reconcile_focus`]'s first branch to keep focus exactly where it is.
fn focus_successor(session: &SessionState, focus_is_doomed: bool) -> Option<ShellSlotId> {
    if !focus_is_doomed {
        return None;
    }
    let tree = session.shell_slots.as_ref()?;
    let focused = session.focused_shell_slot.as_ref()?;
    shell_slots::successor_slot_after_prune(tree, focused)
}

/// Brings the slot tree back into agreement with `session.projects`, then with itself. Runs on boot
/// restore (contract §0.1 S-1) and after every close, so three invariants always hold before the
/// tree is handed to a window: no leaf names a project the session does not list, a session with at
/// least one open project always has at least one slot (a session written before d-62 has no tree
/// at all — this is where it becomes one), and `focused_shell_slot`/`active_project` name a slot
/// that exists.
///
/// `successor` is forwarded to [`reconcile_focus`] — see there. Boot restore passes `None`; only a
/// close that took the focused slot with it has a neighbour to nominate.
pub fn normalize_shell_slots(session: &mut SessionState, successor: Option<ShellSlotId>) {
    let open: HashSet<ProjectId> = session.projects.iter().map(|reference| reference.id.clone()).collect();
    session.shell_slots = session
        .shell_slots
        .take()
        .and_then(|tree| shell_slots::retain_projects(tree, &open));

    if session.shell_slots.is_none() {
        let fallback = session
            .active_project
            .clone()
            .filter(|project_id| open.contains(project_id))
            .or_else(|| session.projects.last().map(|reference| reference.id.clone()));
        session.shell_slots = fallback.map(shell_slots::leaf);
    }

    reconcile_focus(session, successor);
}

/// The slot meaning of "activate this project" (contract §0.1 S-5): a project already on screen
/// just takes focus, and one that is not replaces whatever the focused slot was showing — which is
/// exactly the pre-d-62 single-slot behaviour when there is only one slot.
fn place_project_in_focused_slot(session: &mut SessionState, project_id: &ProjectId) {
    let Some(tree) = session.shell_slots.as_mut() else {
        let materialized = shell_slots::leaf(project_id.clone());
        session.focused_shell_slot = shell_slots::first_slot(&materialized);
        session.shell_slots = Some(materialized);
        session.active_project = Some(project_id.clone());
        return;
    };

    if let Some(slot) = shell_slots::slot_of_project(tree, project_id) {
        session.focused_shell_slot = Some(slot);
        session.active_project = Some(project_id.clone());
        return;
    }

    let target = session
        .focused_shell_slot
        .clone()
        .filter(|slot| shell_slots::contains_slot(tree, slot))
        .or_else(|| shell_slots::first_slot(tree));

    if let Some(slot) = target {
        shell_slots::set_slot_project(tree, &slot, project_id.clone());
        session.focused_shell_slot = Some(slot);
    }
    session.active_project = Some(project_id.clone());
}

fn project_already_in_slot(project_id: &ProjectId) -> AppError {
    AppError::localized(
        AppErrorKind::InvalidArgument,
        "error.shellSlot.projectAlreadyInSlot",
        format!("project is already shown in another shell slot: {project_id}"),
    )
}

/// Server-side half of contract §0.1 S-3: the drop zones refuse a duplicate for UX, but the rule
/// itself lives here, so a remote session or a command palette entry cannot put the same project in
/// two slots either (its hot-exit mirror and pane tree are single-owner state — contract §1.E).
/// Also validates that `target_slot` exists at all, *before* the caller opens anything, so a bad
/// request never leaves a half-opened project behind.
///
/// `project_id` is `None` when the caller named a path that is not open yet: a project that is not
/// open cannot be in any slot, so only the target check applies.
pub fn ensure_slot_placement_allowed(
    session: &SessionState,
    project_id: Option<&ProjectId>,
    target_slot: &ShellSlotId,
    edge: ShellSlotEdge,
) -> AppResult<()> {
    let tree = session
        .shell_slots
        .as_ref()
        .ok_or_else(|| AppError::NotFound(format!("shell slot not found: {target_slot}")))?;
    if !shell_slots::contains_slot(tree, target_slot) {
        return Err(AppError::NotFound(format!("shell slot not found: {target_slot}")));
    }

    let Some(project_id) = project_id else {
        return Ok(());
    };

    let occupied = shell_slots::leaves(tree)
        .into_iter()
        .any(|(slot_id, occupant)| &occupant == project_id && (edge != ShellSlotEdge::Replace || &slot_id != target_slot));
    if occupied {
        return Err(project_already_in_slot(project_id));
    }

    Ok(())
}

/// Puts an already-open project into the slot tree at `target_slot`, either replacing that slot's
/// project or splitting it along `edge`, and focuses the result. Assumes
/// [`ensure_slot_placement_allowed`] already passed for the same arguments.
///
/// Placing a project is an activation, so it stamps `last_opened_at` and persists the record the
/// same way [`activate_project`] and [`focus_shell_slot`] do: the slot this creates is the one the
/// user is now looking at, and without the stamp "open to the right" would leave the project
/// sitting at the bottom of the Welcome screen's recency ordering (contract §1.1) while it is on
/// screen. When the caller opened the project in this same mutation, [`open_project`] has already
/// stamped it once — restamping is what keeps the two entry points indistinguishable afterwards.
pub fn place_project_in_slot(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    project_id: &ProjectId,
    target_slot: &ShellSlotId,
    edge: ShellSlotEdge,
) -> AppResult<ShellSlotId> {
    let tree = session
        .shell_slots
        .as_mut()
        .ok_or_else(|| AppError::NotFound(format!("shell slot not found: {target_slot}")))?;

    let slot = if edge == ShellSlotEdge::Replace {
        if !shell_slots::set_slot_project(tree, target_slot, project_id.clone()) {
            return Err(AppError::NotFound(format!("shell slot not found: {target_slot}")));
        }
        target_slot.clone()
    } else {
        shell_slots::split_slot(tree, target_slot, edge, project_id)?
            .ok_or_else(|| AppError::NotFound(format!("shell slot not found: {target_slot}")))?
    };

    session.focused_shell_slot = Some(slot.clone());
    session.active_project = Some(project_id.clone());
    if let Some(project) = projects.get_mut(project_id) {
        project.last_opened_at = now_epoch_ms();
        save_project(paths, project)?;
    }
    save_session(paths, session)?;
    Ok(slot)
}

/// Removes one slot while leaving its project open — the slot header's close button, as opposed to
/// closing the project itself. The last slot is refused rather than silently emptying the window:
/// "at least one slot while at least one project is open" is the invariant
/// [`normalize_shell_slots`] maintains everywhere else.
///
/// Closing the *focused* slot hands focus to its on-screen neighbour, nominated before the prune
/// erases the adjacency — see [`reconcile_focus`]. Closing any other slot leaves focus untouched.
pub fn close_shell_slot(paths: &AppPaths, session: &mut SessionState, slot_id: &ShellSlotId) -> AppResult<()> {
    let tree = session
        .shell_slots
        .as_ref()
        .ok_or_else(|| AppError::NotFound(format!("shell slot not found: {slot_id}")))?;
    if !shell_slots::contains_slot(tree, slot_id) {
        return Err(AppError::NotFound(format!("shell slot not found: {slot_id}")));
    }
    if shell_slots::leaf_count(tree) <= 1 {
        return Err(AppError::InvalidArgument("cannot close the last shell slot".to_string()));
    }

    let successor = focus_successor(session, session.focused_shell_slot.as_ref() == Some(slot_id));
    session.shell_slots = session.shell_slots.take().and_then(|tree| shell_slots::remove_slot(tree, slot_id));
    reconcile_focus(session, successor);
    save_session(paths, session)
}

/// Focuses one slot and makes its project the active one, stamping `last_opened_at` exactly like
/// [`activate_project`] — moving the focus between slots *is* an activation as far as the Welcome
/// screen's recency ordering is concerned.
pub fn focus_shell_slot(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    slot_id: &ShellSlotId,
) -> AppResult<()> {
    let project_id = session
        .shell_slots
        .as_ref()
        .and_then(|tree| shell_slots::project_of_slot(tree, slot_id))
        .ok_or_else(|| AppError::NotFound(format!("shell slot not found: {slot_id}")))?;

    session.focused_shell_slot = Some(slot_id.clone());
    session.active_project = Some(project_id.clone());
    if let Some(project) = projects.get_mut(&project_id) {
        project.last_opened_at = now_epoch_ms();
        save_project(paths, project)?;
    }
    save_session(paths, session)
}

pub fn set_shell_slot_sizes(paths: &AppPaths, session: &mut SessionState, path: &[u32], sizes: Vec<f32>) -> AppResult<()> {
    let tree = session
        .shell_slots
        .as_mut()
        .ok_or_else(|| AppError::NotFound("shell slot tree is empty".to_string()))?;
    shell_slots::set_sizes(tree, path, sizes)?;
    save_session(paths, session)
}

pub fn set_window_chrome(paths: &AppPaths, session: &mut SessionState, patch: &WindowChromePatch) -> AppResult<WindowChrome> {
    if let Some(zen) = patch.zen {
        session.window_chrome.zen = zen;
    }
    if let Some(sidebar_rail_collapsed) = patch.sidebar_rail_collapsed {
        session.window_chrome.sidebar_rail_collapsed = sidebar_rail_collapsed;
    }
    save_session(paths, session)?;
    Ok(session.window_chrome)
}

/// One-time boot promotion of the two window-chrome axes off the per-project
/// `ProjectLayout::shell_view` and onto `SessionState::window_chrome` (contract §0.1 S-6). The
/// source values are **cleared** on every layout as part of the promotion, and the ids of the
/// layouts that changed are returned so the caller can persist them: without that clearing the
/// promotion would re-fire on a later boot (the trigger is "session chrome is still at its default",
/// which a user turning Zen back off restores), silently resurrecting a stale Zen from a layout file
/// nothing reads any more.
///
/// The value promoted is the active project's, falling back to the first listed project that has
/// either axis set — the same "what was the user actually looking at" ordering
/// `projects_pending_watcher_restore` uses.
pub fn promote_legacy_window_chrome(session: &mut SessionState, shell_views: &mut HashMap<ProjectId, ShellViewState>) -> Vec<ProjectId> {
    if session.window_chrome != WindowChrome::default() {
        return Vec::new();
    }

    let has_legacy_value = |view: &ShellViewState| view.zen || view.sidebar_collapsed;
    let ordered: Vec<ProjectId> = session
        .active_project
        .iter()
        .cloned()
        .chain(
            session
                .projects
                .iter()
                .map(|reference| reference.id.clone())
                .filter(|id| Some(id) != session.active_project.as_ref()),
        )
        .collect();

    let source = ordered
        .into_iter()
        .find(|id| shell_views.get(id).is_some_and(has_legacy_value))
        .and_then(|id| shell_views.get(&id).copied());
    let Some(source) = source else {
        return Vec::new();
    };

    session.window_chrome = WindowChrome {
        zen: source.zen,
        sidebar_rail_collapsed: source.sidebar_collapsed,
    };

    let mut cleared = Vec::new();
    for (project_id, view) in shell_views.iter_mut() {
        if !has_legacy_value(view) {
            continue;
        }
        view.zen = false;
        view.sidebar_collapsed = false;
        cleared.push(project_id.clone());
    }
    cleared.sort();
    cleared
}

pub fn get_project(projects: &HashMap<ProjectId, Project>, project_id: &ProjectId) -> AppResult<Project> {
    projects
        .get(project_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))
}

/// Resolves the path a caller handed [`open_project`] into one the filesystem can answer for: a
/// leading `~` is a shell convention `canonicalize` knows nothing about, so a path the user typed
/// by hand ("Open by path…", the sidebar's + menu) would otherwise fail as a literal directory
/// named `~`. Shares [`home::expand_home`] with the terminal's own path-link resolver rather than
/// re-deriving the rule — see that function for why `~user` is deliberately left alone.
fn resolve_open_root(root: &Path) -> PathBuf {
    match root.to_str() {
        Some(text) => PathBuf::from(home::expand_home_from_env(text)),
        None => root.to_path_buf(),
    }
}

/// Which already-open project (if any) the path `root` names, resolved the same way
/// [`open_project`] resolves it. `project_open_in_slot` needs this *before* it opens anything: the
/// duplicate-slot rule ([`ensure_slot_placement_allowed`]) has to be able to reject a drop without
/// leaving a freshly opened project stranded, and a path that resolves to nothing open yet cannot
/// be a duplicate by definition. A path that does not canonicalize answers `None` and the real,
/// localized error comes from [`open_project`] a moment later.
pub fn find_open_project_by_root(projects: &HashMap<ProjectId, Project>, root: &Path) -> Option<ProjectId> {
    let canonical = std::fs::canonicalize(resolve_open_root(root)).ok()?;
    let root_str = canonical.to_string_lossy().to_string();
    projects
        .values()
        .find(|project| project.root == root_str)
        .map(|project| project.id.clone())
}

/// Opens (or re-activates) the project at `root`. `detect_capabilities` is called once with the
/// canonicalized root on a fresh open and its result is recorded verbatim as
/// `Project.capabilities` — `project_open` injects the capability registry's `detected_kinds`, so
/// the registry is the single source of that field and `GitWatcherCapability::attach`'s
/// `contains(Git)` gate is the one place the git decision is made on the open path (no second
/// filesystem probe here).
///
/// A missing path is reported as a localized `error.project.pathNotFound` rather than the bare
/// `std::io::Error` string `AppError::from` would produce: this is the one project entry point a
/// user can reach by typing a path, so "No such file or directory (os error 2)" is the error text
/// they would actually see.
///
/// The re-activation branch (`already_open`) clears `root_missing` and writes the whole `ProjectRef`
/// mirror through: everything above it — `canonicalize`, the `is_dir` check, the `read_dir` probe —
/// has just proven the root is readable, so a flag left over from a boot where the folder was
/// missing (an unplugged drive, restored by `restore_session`) would otherwise stay stuck for the
/// rest of the session and keep the sidebar drawing a warning on a project that is demonstrably
/// fine. Re-attaching that project's watchers still requires a real close/re-open — the boot-time
/// exclusion in `commands::projects_pending_watcher_restore` is deliberate — which is what the
/// slot header's "다시 열기" action does.
///
/// `activate` (contract §0.1 S-2) decides whether the newly opened project also becomes the focused
/// slot's project: `project_open` passes `true` (its long-standing behaviour), while
/// `project_open_in_slot` passes `false` because it places the project in a slot of its own right
/// afterwards, and the future group-open queue will pass `false` for every member but the first. A
/// non-activating open still records the project in the session, stamps `last_opened_at` and
/// persists — it just leaves the focus, the slot tree and `ProjectActivated` alone.
pub fn open_project(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    root: &Path,
    activate: bool,
    detect_capabilities: impl FnOnce(&Path) -> Vec<CapabilityKind>,
) -> AppResult<ProjectOpenResult> {
    let requested = resolve_open_root(root);
    let canonical = std::fs::canonicalize(&requested).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => AppError::localized(
            AppErrorKind::NotFound,
            "error.project.pathNotFound",
            format!("path not found: {}", requested.display()),
        )
        .with_arg("path", requested.display()),
        _ => AppError::from(error),
    })?;
    let metadata = std::fs::metadata(&canonical)?;
    if !metadata.is_dir() {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.project.pathNotDirectory",
            format!("path is not a directory: {}", canonical.display()),
        )
        .with_arg("path", canonical.display()));
    }
    std::fs::read_dir(&canonical)?;

    let root_str = canonical.to_string_lossy().to_string();

    if let Some(existing) = projects.values().find(|project| project.root == root_str) {
        let mut existing = existing.clone();
        existing.last_opened_at = now_epoch_ms();
        existing.root_missing = false;
        projects.insert(existing.id.clone(), existing.clone());
        upsert_project_ref(session, &existing);
        if activate {
            place_project_in_focused_slot(session, &existing.id);
        }
        save_project(paths, &existing)?;
        save_session(paths, session)?;
        return Ok(ProjectOpenResult {
            project: existing,
            already_open: true,
        });
    }

    let history = find_existing_project_record(paths, &root_str)?;
    let id = history.as_ref().map(|project| project.id.clone()).unwrap_or_else(ProjectId::new);
    let display = history.map(|project| project.display).unwrap_or_default();

    let name = canonical
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| root_str.clone());

    let capabilities = detect_capabilities(&canonical);

    let project = Project {
        id: id.clone(),
        root: root_str,
        name,
        capabilities,
        root_missing: false,
        last_opened_at: now_epoch_ms(),
        display,
    };

    projects.insert(id, project.clone());
    upsert_project_ref(session, &project);
    if activate {
        place_project_in_focused_slot(session, &project.id);
    }

    save_project(paths, &project)?;
    save_session(paths, session)?;

    Ok(ProjectOpenResult {
        project,
        already_open: false,
    })
}

/// Removes the project from the session **and** from the slot tree in the same mutation, so a close
/// can never leave a slot pointing at a project that is gone (contract §0.1 S-1). The tree collapse
/// and the focus/active recomputation are [`normalize_shell_slots`]'s, which is also what
/// re-materializes a single slot when the last slot's project was the one closed but other projects
/// are still open.
///
/// This is the higher-traffic half of the focus succession [`close_shell_slot`] also performs: when
/// the focused slot is one of the ones this project occupied, focus goes to its on-screen neighbour
/// rather than to the tree's first slot — see [`reconcile_focus`].
pub fn close_project(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    project_id: &ProjectId,
) -> AppResult<()> {
    let focused_project = session
        .shell_slots
        .as_ref()
        .zip(session.focused_shell_slot.as_ref())
        .and_then(|(tree, slot)| shell_slots::project_of_slot(tree, slot));
    let successor = focus_successor(session, focused_project.as_ref() == Some(project_id));

    projects.remove(project_id);
    session.projects.retain(|reference| &reference.id != project_id);
    session.shell_slots = session
        .shell_slots
        .take()
        .and_then(|tree| shell_slots::remove_project(tree, project_id));
    if session.active_project.as_ref() == Some(project_id) {
        session.active_project = session.projects.last().map(|reference| reference.id.clone());
    }
    normalize_shell_slots(session, successor);
    save_session(paths, session)
}

/// Activates `project_id` (must already be open in `session`) and, when its `Project` record is
/// present in `projects`, stamps `last_opened_at` and persists it — mirrors `open_project`'s
/// re-open branch so a mere pane-switch back to an already-open project also counts toward the
/// Welcome screen's "recent" ordering (contract §1.1), not only a fresh `project_open` call.
///
/// Since d-62 this also carries the slot meaning of an activation — see
/// [`place_project_in_focused_slot`]: a sidebar click on a project that is already on screen moves
/// the focus to its slot instead of stealing the focused slot away from another project.
pub fn activate_project(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    project_id: &ProjectId,
) -> AppResult<()> {
    if !session.projects.iter().any(|reference| &reference.id == project_id) {
        return Err(AppError::NotFound(format!("project not open: {project_id}")));
    }
    place_project_in_focused_slot(session, project_id);
    if let Some(project) = projects.get_mut(project_id) {
        project.last_opened_at = now_epoch_ms();
        save_project(paths, project)?;
    }
    save_session(paths, session)
}

pub fn reorder_projects(paths: &AppPaths, session: &mut SessionState, ids: &[ProjectId]) -> AppResult<()> {
    let original = std::mem::take(&mut session.projects);
    let mut used = vec![false; original.len()];
    let mut reordered = Vec::with_capacity(original.len());

    for id in ids {
        if let Some(position) = original.iter().position(|reference| &reference.id == id) {
            if !used[position] {
                used[position] = true;
                reordered.push(original[position].clone());
            }
        }
    }

    for (index, reference) in original.into_iter().enumerate() {
        if !used[index] {
            reordered.push(reference);
        }
    }

    session.projects = reordered;
    save_session(paths, session)
}

fn display_invalid(field: &str) -> AppError {
    AppError::localized(
        AppErrorKind::InvalidArgument,
        "error.project.displayInvalid",
        format!("invalid project display value for field: {field}"),
    )
}

/// A curated lucide icon name (`src/shared/icons/project-icon-registry.ts`). Only the shape is
/// checked here — an unknown-but-well-formed name is accepted and the frontend registry falls back
/// to the folder icon, so the desktop never has to be rebuilt in lockstep with the registry.
fn sanitize_display_icon(value: &str) -> AppResult<Option<String>> {
    if value.is_empty() {
        return Ok(None);
    }
    let is_icon_name = value.len() <= DISPLAY_ICON_NAME_MAX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    is_icon_name.then(|| Some(value.to_string())).ok_or_else(|| display_invalid("icon"))
}

/// A short text label drawn inside the 40px sidebar button. Control characters are dropped and the
/// result trimmed before the length is measured, so a paste carrying a newline or a stray tab is
/// corrected rather than rejected; a value that is empty once cleaned clears the label, matching
/// the empty-string clear convention instead of persisting an invisible label.
fn sanitize_display_label(value: &str) -> AppResult<Option<String>> {
    let cleaned: String = value.chars().filter(|character| !character.is_control()).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > DISPLAY_LABEL_MAX_CODEPOINTS {
        return Err(display_invalid("label"));
    }
    Ok(Some(trimmed.to_string()))
}

fn sanitize_display_color(value: &str) -> AppResult<Option<String>> {
    if value.is_empty() {
        return Ok(None);
    }
    DISPLAY_COLOR_TOKENS
        .contains(&value)
        .then(|| Some(value.to_string()))
        .ok_or_else(|| display_invalid("color"))
}

/// Merges one display axis under the settings domain's clearable-string convention (see
/// [`ProjectDisplayPatch`]): an omitted axis keeps `existing` untouched without being re-validated,
/// and a present one goes through `sanitize`, which decides between clearing the axis and replacing
/// it. Rejecting a bad value here (rather than silently dropping it) is what keeps a remote or
/// programmatic caller from persisting a value the sidebar could not render.
fn merge_display_axis(
    patch_value: Option<&String>,
    existing: Option<&String>,
    sanitize: impl FnOnce(&str) -> AppResult<Option<String>>,
) -> AppResult<Option<String>> {
    match patch_value {
        None => Ok(existing.cloned()),
        Some(value) => sanitize(value),
    }
}

fn merge_display(existing: &ProjectDisplay, patch: &ProjectDisplayPatch) -> AppResult<ProjectDisplay> {
    Ok(ProjectDisplay {
        icon: merge_display_axis(patch.icon.as_ref(), existing.icon.as_ref(), sanitize_display_icon)?,
        label: merge_display_axis(patch.label.as_ref(), existing.label.as_ref(), sanitize_display_label)?,
        color: merge_display_axis(patch.color.as_ref(), existing.color.as_ref(), sanitize_display_color)?,
    })
}

/// Applies `patch` to an open project's sidebar presentation and persists the result to both
/// owners: `projects/<id>/project.json` (the source of truth) and `session.json`'s `ProjectRef`
/// mirror, through the same [`upsert_project_ref`] that already mirrors `root`/`name` — writing
/// only one of the two would leave the sidebar (which reads `project_list`) and the project record
/// disagreeing until the next open. The whole patch is validated before anything is assigned, so a
/// rejected axis leaves the project exactly as it was rather than persisting a half-applied change.
pub fn set_project_display(
    paths: &AppPaths,
    session: &mut SessionState,
    projects: &mut HashMap<ProjectId, Project>,
    project_id: &ProjectId,
    patch: &ProjectDisplayPatch,
) -> AppResult<()> {
    let project = projects
        .get_mut(project_id)
        .ok_or_else(|| AppError::NotFound(format!("project not open: {project_id}")))?;

    let merged = merge_display(&project.display, patch)?;
    project.display = merged;
    let updated = project.clone();

    upsert_project_ref(session, &updated);
    save_project(paths, &updated)?;
    save_session(paths, session)
}

fn group_invalid(field: &str) -> AppError {
    AppError::localized(
        AppErrorKind::InvalidArgument,
        "error.projectGroup.invalid",
        format!("invalid project group value for field: {field}"),
    )
}

fn group_not_found(group_id: &ProjectGroupId) -> AppError {
    AppError::NotFound(format!("project group not found: {group_id}"))
}

/// A sidebar header string: control characters are dropped and the result trimmed before the length
/// is measured (so a pasted newline is corrected rather than rejected), and a name that is empty
/// once cleaned is refused — unlike a display label, an unnamed group has nothing to click on.
fn sanitize_group_name(value: &str) -> AppResult<String> {
    let cleaned: String = value.chars().filter(|character| !character.is_control()).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() || trimmed.chars().count() > GROUP_NAME_MAX_CODEPOINTS {
        return Err(group_invalid("name"));
    }
    Ok(trimmed.to_string())
}

fn sanitize_group_color(value: Option<&str>) -> AppResult<Option<String>> {
    match value {
        None => Ok(None),
        Some(token) => sanitize_display_color(token).map_err(|_| group_invalid("color")),
    }
}

/// Every project this desktop has a persisted record for — the membership vocabulary. Groups
/// organize what the user *can* open, so a closed project is a perfectly good member (its
/// `projects/<id>/` directory is still there); only an id with no record at all is refused, which is
/// also why [`forget_recent_projects`] is the one thing that evicts a member.
fn known_project_ids(paths: &AppPaths) -> AppResult<HashSet<ProjectId>> {
    Ok(iter_project_ids(paths)?.into_iter().collect())
}

/// Dedupes `members` and refuses any id with no persisted project record. Refusing rather than
/// silently dropping is what keeps a stale client list from quietly shrinking a group the user
/// still sees — the caller gets an error and re-reads, instead of a half-applied write.
fn normalize_group_members(paths: &AppPaths, members: Vec<ProjectId>) -> AppResult<Vec<ProjectId>> {
    let known = known_project_ids(paths)?;
    let deduped = groups::dedupe_members(members);
    if let Some(unknown) = deduped.iter().find(|project_id| !known.contains(project_id)) {
        return Err(AppError::localized(
            AppErrorKind::InvalidArgument,
            "error.projectGroup.memberUnknown",
            format!("project group member has no persisted record: {unknown}"),
        ));
    }
    Ok(deduped)
}

pub fn list_groups(session: &SessionState) -> Vec<ProjectGroup> {
    session.groups.clone()
}

/// The root `project_group_open` should open a member at, or `None` when the member cannot be
/// opened: its record is gone, unreadable, or names a root that is no longer a directory. Read-only
/// on purpose — this runs for every member of a group before anything is opened, so it uses the same
/// never-writes path [`list_recent_projects`] does rather than `load_project`'s quarantining one.
pub fn group_member_root(paths: &AppPaths, project_id: &ProjectId) -> Option<String> {
    let project = try_load_project_readonly(paths, project_id)?;
    Path::new(&project.root).is_dir().then_some(project.root)
}

/// The members of one group, in the user's own order — the queue `project_group_open` walks.
pub fn group_members(session: &SessionState, group_id: &ProjectGroupId) -> AppResult<Vec<ProjectId>> {
    groups::find(&session.groups, group_id)
        .map(|group| group.members.clone())
        .ok_or_else(|| group_not_found(group_id))
}

pub fn create_group(
    paths: &AppPaths,
    session: &mut SessionState,
    name: &str,
    color: Option<&str>,
    members: Option<Vec<ProjectId>>,
) -> AppResult<ProjectGroup> {
    let group = ProjectGroup {
        id: ProjectGroupId::new(),
        name: sanitize_group_name(name)?,
        color: sanitize_group_color(color)?,
        members: normalize_group_members(paths, members.unwrap_or_default())?,
        collapsed: false,
    };

    groups::claim_members(&mut session.groups, &group.id, &group.members);
    session.groups.push(group.clone());
    save_session(paths, session)?;
    Ok(group)
}

pub fn rename_group(paths: &AppPaths, session: &mut SessionState, group_id: &ProjectGroupId, name: &str) -> AppResult<()> {
    let name = sanitize_group_name(name)?;
    let group = groups::find_mut(&mut session.groups, group_id).ok_or_else(|| group_not_found(group_id))?;
    group.name = name;
    save_session(paths, session)
}

pub fn set_group_color(paths: &AppPaths, session: &mut SessionState, group_id: &ProjectGroupId, color: Option<&str>) -> AppResult<()> {
    let color = sanitize_group_color(color)?;
    let group = groups::find_mut(&mut session.groups, group_id).ok_or_else(|| group_not_found(group_id))?;
    group.color = color;
    save_session(paths, session)
}

pub fn set_group_collapsed(paths: &AppPaths, session: &mut SessionState, group_id: &ProjectGroupId, collapsed: bool) -> AppResult<()> {
    let group = groups::find_mut(&mut session.groups, group_id).ok_or_else(|| group_not_found(group_id))?;
    group.collapsed = collapsed;
    save_session(paths, session)
}

/// Deletes the group itself and nothing else: its members stay open, stay in `session.projects`, and
/// keep their own records. A group is an organization of projects, never their owner.
pub fn delete_group(paths: &AppPaths, session: &mut SessionState, group_id: &ProjectGroupId) -> AppResult<()> {
    if groups::find(&session.groups, group_id).is_none() {
        return Err(group_not_found(group_id));
    }
    session.groups.retain(|group| &group.id != group_id);
    save_session(paths, session)
}

/// Replaces one group's membership wholesale, taking each named project away from whatever other
/// group held it (the one-group-per-project rule on [`ProjectGroup`]). The whole list is validated
/// before anything is assigned, so a rejected member leaves every group exactly as it was.
pub fn set_group_members(
    paths: &AppPaths,
    session: &mut SessionState,
    group_id: &ProjectGroupId,
    members: Vec<ProjectId>,
) -> AppResult<()> {
    if groups::find(&session.groups, group_id).is_none() {
        return Err(group_not_found(group_id));
    }
    let members = normalize_group_members(paths, members)?;

    groups::claim_members(&mut session.groups, group_id, &members);
    if let Some(group) = groups::find_mut(&mut session.groups, group_id) {
        group.members = members;
    }
    save_session(paths, session)
}

pub fn reorder_groups(paths: &AppPaths, session: &mut SessionState, ids: &[ProjectGroupId]) -> AppResult<()> {
    session.groups = groups::reorder(std::mem::take(&mut session.groups), ids);
    save_session(paths, session)
}

pub fn migrate_session(value: serde_json::Value) -> AppResult<SessionState> {
    let session: SessionState = serde_json::from_value(value)?;
    Ok(session)
}

pub fn load_session(paths: &AppPaths) -> AppResult<(SessionState, Vec<String>)> {
    let path = paths.session_file();
    let raw = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((SessionState::default(), Vec::new()));
        }
        Err(error) => return Err(error.into()),
    };

    let parsed = serde_json::from_slice::<serde_json::Value>(&raw)
        .map_err(AppError::from)
        .and_then(migrate_session);

    match parsed {
        Ok(session) => Ok((session, Vec::new())),
        Err(_) => {
            backup_corrupted(&path)?;
            Ok((
                SessionState::default(),
                vec![format!("손상된 세션 파일을 백업하고 기본값으로 초기화했습니다: {}", path.display())],
            ))
        }
    }
}

pub fn save_session(paths: &AppPaths, session: &SessionState) -> AppResult<()> {
    persist::write_json(&paths.session_file(), session)
}

pub fn load_project(paths: &AppPaths, id: &ProjectId) -> AppResult<(Option<Project>, Vec<String>)> {
    let path = paths.project_file(id);
    let raw = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((None, Vec::new())),
        Err(error) => return Err(error.into()),
    };

    match serde_json::from_slice::<Project>(&raw) {
        Ok(project) => Ok((Some(project), Vec::new())),
        Err(_) => {
            backup_corrupted(&path)?;
            Ok((None, vec![format!("손상된 프로젝트 파일을 백업했습니다: {}", path.display())]))
        }
    }
}

pub fn save_project(paths: &AppPaths, project: &Project) -> AppResult<()> {
    persist::write_json(&paths.project_file(&project.id), project)
}

pub fn restore_session(paths: &AppPaths) -> AppResult<(SessionState, Vec<Project>, Vec<String>)> {
    let (mut session, mut warnings) = load_session(paths)?;
    let mut projects = Vec::with_capacity(session.projects.len());

    for reference in &mut session.projects {
        let (loaded, load_warnings) = load_project(paths, &reference.id)?;
        warnings.extend(load_warnings);

        let mut project = loaded.unwrap_or_else(|| Project {
            id: reference.id.clone(),
            root: reference.root.clone(),
            name: reference.name.clone(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: reference.display.clone(),
        });

        project.root_missing = !Path::new(&project.root).is_dir();
        reference.root_missing = project.root_missing;
        projects.push(project);
    }

    normalize_shell_slots(&mut session, None);

    Ok((session, projects, warnings))
}

fn upsert_project_ref(session: &mut SessionState, project: &Project) {
    if let Some(existing) = session.projects.iter_mut().find(|reference| reference.id == project.id) {
        existing.root = project.root.clone();
        existing.name = project.name.clone();
        existing.display = project.display.clone();
        existing.root_missing = project.root_missing;
    } else {
        session.projects.push(ProjectRef {
            id: project.id.clone(),
            root: project.root.clone(),
            name: project.name.clone(),
            display: project.display.clone(),
            root_missing: project.root_missing,
        });
    }
}

/// Every project id with a persisted record under `paths.data_dir/projects/` — the full on-disk
/// history, independent of which projects (if any) are currently open in `SessionState`. Shared by
/// `find_existing_project_id` (id-reuse lookup by root) and `list_recent_projects` (the Welcome
/// screen's full history listing), so both read the exact same directory-listing rule.
fn iter_project_ids(paths: &AppPaths) -> AppResult<Vec<ProjectId>> {
    let projects_root = paths.data_dir.join(PROJECTS_DIR_NAME);
    let entries = match std::fs::read_dir(&projects_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut ids = Vec::new();
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        ids.push(ProjectId(entry.file_name().to_string_lossy().to_string()));
    }
    Ok(ids)
}

/// Read-only counterpart to `load_project`, for callers that must never touch the filesystem —
/// unlike `load_project` (used by `project_open`, which runs behind `begin_mutation` and may
/// legitimately quarantine a corrupted `project.json` by renaming it to `.bak`), this never
/// writes. A missing file, a read error (e.g. permission denied), or a parse failure are all
/// treated the same way: the record is unavailable, so the caller skips it. Used by
/// `list_recent_projects`, which runs with no mutation guard and must tolerate one damaged or
/// unreadable record without failing the whole listing or racing `project_open`'s own backup
/// rename of the same file.
fn try_load_project_readonly(paths: &AppPaths, id: &ProjectId) -> Option<Project> {
    let raw = std::fs::read(paths.project_file(id)).ok()?;
    serde_json::from_slice::<Project>(&raw).ok()
}

/// Every persisted project record, most-recently-opened first (`Project.last_opened_at`
/// descending) — the Welcome screen's "recent projects" source, distinct from `list_projects`
/// (which only returns the *currently open* session's `ProjectRef`s). `root_missing` is
/// recomputed against the live filesystem the same way `restore_session` does, so a project whose
/// folder moved or was deleted since it was last opened still lists (disabled, per contract §1.2)
/// instead of silently vanishing. A record that fails to read or parse is skipped rather than
/// failing the whole listing (`try_load_project_readonly`'s per-entry tolerance) — and, unlike
/// `load_project`, never backs it up to `.bak`, since this listing has no `begin_mutation` guard.
pub fn list_recent_projects(paths: &AppPaths) -> AppResult<Vec<Project>> {
    let mut projects = Vec::new();
    for id in iter_project_ids(paths)? {
        if let Some(mut project) = try_load_project_readonly(paths, &id) {
            project.root_missing = !Path::new(&project.root).is_dir();
            projects.push(project);
        }
    }

    projects.sort_by(|a, b| b.last_opened_at.total_cmp(&a.last_opened_at));
    Ok(projects)
}

/// Deletes the persisted record (`projects/<id>/`) of every project that is **not** currently open,
/// and answers how many were removed — what `File > Clear Recent` and the sidebar's equivalent
/// mean by "clear": the recent list is derived from those records ([`list_recent_projects`]), so
/// forgetting one is deleting its directory, there is no separate history file to truncate.
///
/// Open projects are kept because their record is live state, not history: `save_project` rewrites
/// it on every activation, `save_layout` writes the layout beside it, and `close_project`
/// deliberately leaves the directory behind so a re-open restores the same id, layout and display.
/// Deleting an open project's directory would strand all three.
///
/// A closed project whose hot-exit buffers still hold unsaved work is kept too, and counted into
/// [`ForgetRecentOutcome::skipped_with_drafts`] so the caller can say so. `projects/<id>/` is where
/// `domain::file`'s mirrors live (`AppPaths::buffers_dir`), and `close_project` deliberately leaves
/// them behind — re-opening the project restores the draft. "Clear Recent" reads as "forget this
/// entry", not "discard the work I have not saved yet", and the deletion is irreversible, so the
/// record outlives the clear until the draft is either restored or dropped.
///
/// A directory that fails to delete is logged and skipped rather than failing the whole call —
/// same per-entry tolerance as [`list_recent_projects`], since a half-cleared list is still a
/// better answer than an error that clears nothing.
///
/// Group membership is cleaned in the same call (contract §0.1 S-7): a forgotten project has no
/// record left to re-open, so leaving it listed under a group header would draw a member that
/// resolves to nothing. Only the ids this call actually deleted are evicted — a member whose record
/// was already missing for some other reason is left alone rather than silently pruned here, the
/// same restraint `normalize_shell_slots` shows by reconciling only at restore. The session is
/// rewritten only when a group really changed, so the common "nothing to forget" call still writes
/// nothing — and [`ForgetRecentOutcome::groups_changed`] hands that same answer back, so the caller
/// does not have to re-derive it to decide whether a group event is worth emitting.
pub fn forget_recent_projects(
    paths: &AppPaths,
    session: &mut SessionState,
    open_project_ids: &HashSet<ProjectId>,
) -> AppResult<ForgetRecentOutcome> {
    let mut forgotten: HashSet<ProjectId> = HashSet::new();
    let mut skipped_with_drafts: u32 = 0;
    for id in iter_project_ids(paths)? {
        if open_project_ids.contains(&id) {
            continue;
        }
        if has_hot_exit_drafts(paths, &id) {
            skipped_with_drafts += 1;
            continue;
        }
        match std::fs::remove_dir_all(paths.project_dir(&id)) {
            Ok(()) => {
                forgotten.insert(id);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => log::warn!("최근 프로젝트 레코드를 지우지 못했습니다 (projectId={id}): {error}"),
        }
    }

    let groups_changed = groups::forget_members(&mut session.groups, &forgotten);
    if groups_changed {
        save_session(paths, session)?;
    }
    Ok(ForgetRecentOutcome {
        removed: forgotten.len() as u32,
        skipped_with_drafts,
        groups_changed,
    })
}

/// Whether `project_id` still has any hot-exit mirror on disk — a path mirror
/// (`buffers/<hash>.json`) or an untitled one (`buffers/untitled/<tabId>.json`).
///
/// Asked as "is there any file under `buffers_dir`", deliberately not by re-deriving
/// `domain::file`'s mirror naming here: the only question that matters is whether deleting this
/// record would destroy work the user never saved, and anything the file domain wrote there is
/// exactly that. An unreadable directory answers `false` — the same per-entry tolerance the rest of
/// this listing shows, and a directory that cannot be read cannot be deleted either.
fn has_hot_exit_drafts(paths: &AppPaths, project_id: &ProjectId) -> bool {
    let mut pending = vec![paths.buffers_dir(project_id)];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            match entry.file_type() {
                Ok(file_type) if file_type.is_dir() => pending.push(entry.path()),
                Ok(_) => return true,
                Err(_) => continue,
            }
        }
    }
    false
}

/// The persisted record of a previously-opened project at `root`, if any — `open_project` reuses
/// both its id (so `projects/<id>/` and its layout survive a close/re-open cycle) and the parts of
/// it the user authored rather than the filesystem: currently `display`. Returning the whole
/// record instead of just the id is what keeps a re-open from silently resetting a project's
/// sidebar icon/label/color, since `open_project`'s fresh-open branch builds a brand-new `Project`
/// value even when it is reusing a known id.
fn find_existing_project_record(paths: &AppPaths, root: &str) -> AppResult<Option<Project>> {
    for id in iter_project_ids(paths)? {
        let (loaded, _warnings) = load_project(paths, &id)?;
        if let Some(project) = loaded {
            if project.root == root {
                return Ok(Some(project));
            }
        }
    }

    Ok(None)
}

fn backup_corrupted(path: &Path) -> AppResult<()> {
    let backup_path = PathBuf::from(format!("{}{}", path.display(), BACKUP_SUFFIX));
    std::fs::rename(path, backup_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::project::types::ShellSlotTree;

    fn temp_paths() -> AppPaths {
        let dir = std::env::temp_dir().join(format!("taide-project-test-{}", uuid::Uuid::new_v4()));
        AppPaths::new(dir)
    }

    fn cleanup(paths: &AppPaths) {
        std::fs::remove_dir_all(&paths.data_dir).ok();
    }

    fn detect_terminal_only(_root: &Path) -> Vec<CapabilityKind> {
        vec![CapabilityKind::Terminal]
    }

    #[test]
    fn 열기_경로의_물결은_홈으로_확장되고_다른_형태는_그대로다() {
        let home = home::home_dir_env();
        let expanded_tilde = match &home {
            Some(home) => PathBuf::from(home.clone()),
            None => PathBuf::from("~"),
        };
        let expanded_subpath = match &home {
            Some(home) => PathBuf::from(format!("{home}/workspace/demo")),
            None => PathBuf::from("~/workspace/demo"),
        };

        assert_eq!(resolve_open_root(Path::new("~")), expanded_tilde);
        assert_eq!(resolve_open_root(Path::new("~/workspace/demo")), expanded_subpath);
        assert_eq!(resolve_open_root(Path::new("~alice/demo")), PathBuf::from("~alice/demo"));
        assert_eq!(resolve_open_root(Path::new("/abs/demo")), PathBuf::from("/abs/demo"));
    }

    #[test]
    fn 최근_기록_삭제는_열려있지_않은_프로젝트_레코드만_지운다() {
        let paths = temp_paths();
        let open = Project {
            id: ProjectId::new(),
            root: "/tmp/open".to_string(),
            name: "open".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 2.0,
            display: ProjectDisplay::default(),
        };
        let closed = Project {
            id: ProjectId::new(),
            root: "/tmp/closed".to_string(),
            name: "closed".to_string(),
            last_opened_at: 1.0,
            ..open.clone()
        };
        save_project(&paths, &open).expect("열린 프로젝트 저장");
        save_project(&paths, &closed).expect("닫힌 프로젝트 저장");

        let open_ids: HashSet<ProjectId> = [open.id.clone()].into_iter().collect();
        let mut session = SessionState::default();
        let outcome = forget_recent_projects(&paths, &mut session, &open_ids).expect("최근 기록 삭제");

        assert_eq!(outcome.removed, 1);
        assert!(
            !outcome.groups_changed,
            "그룹이 없으면 레코드를 지워도 그룹 변경으로 보고하면 안 됩니다 — 호출부가 groups-changed 를 발행합니다"
        );
        assert!(paths.project_dir(&open.id).exists());
        assert!(!paths.project_dir(&closed.id).exists());
        assert_eq!(
            list_recent_projects(&paths)
                .expect("목록")
                .into_iter()
                .map(|p| p.id)
                .collect::<Vec<_>>(),
            vec![open.id]
        );

        cleanup(&paths);
    }

    /// "최근 항목 지우기" 는 목록에서 빼는 조작으로 읽히는데, 닫힌 프로젝트의 디렉터리에는
    /// hot-exit 미러가 함께 산다(`close_project` 가 의도적으로 남긴다). 되돌릴 수 없는 삭제이므로
    /// 초안이 남아 있는 레코드는 건너뛰고 그 개수를 보고한다.
    #[test]
    fn 최근_기록_삭제는_미저장_초안이_있는_프로젝트를_건너뛴다() {
        let paths = temp_paths();
        let with_draft = Project {
            id: ProjectId::new(),
            root: "/tmp/with-draft".to_string(),
            name: "with-draft".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 2.0,
            display: ProjectDisplay::default(),
        };
        let clean = Project {
            id: ProjectId::new(),
            root: "/tmp/clean".to_string(),
            name: "clean".to_string(),
            last_opened_at: 1.0,
            ..with_draft.clone()
        };
        save_project(&paths, &with_draft).expect("초안 프로젝트 저장");
        save_project(&paths, &clean).expect("깨끗한 프로젝트 저장");
        let buffers = paths.buffers_dir(&with_draft.id);
        std::fs::create_dir_all(&buffers).expect("버퍼 디렉터리");
        std::fs::write(buffers.join("deadbeefdeadbeef.json"), b"{}").expect("경로 미러");

        let mut session = SessionState::default();
        let outcome = forget_recent_projects(&paths, &mut session, &HashSet::new()).expect("최근 기록 삭제");

        assert_eq!(outcome.removed, 1);
        assert_eq!(outcome.skipped_with_drafts, 1);
        assert!(paths.buffers_dir(&with_draft.id).exists(), "초안이 있는 레코드는 살아남아야 한다");
        assert!(!paths.project_dir(&clean.id).exists());

        cleanup(&paths);
    }

    /// untitled 미러는 `buffers/untitled/` 하위에 있어, 경로 미러와 똑같이 보호 대상이다.
    #[test]
    fn 최근_기록_삭제는_untitled_초안만_있어도_건너뛴다() {
        let paths = temp_paths();
        let project = Project {
            id: ProjectId::new(),
            root: "/tmp/untitled-only".to_string(),
            name: "untitled-only".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 1.0,
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &project).expect("프로젝트 저장");
        let untitled = paths.buffers_dir(&project.id).join("untitled");
        std::fs::create_dir_all(&untitled).expect("untitled 디렉터리");
        std::fs::write(untitled.join("tab-1.json"), b"{}").expect("untitled 미러");

        let mut session = SessionState::default();
        let outcome = forget_recent_projects(&paths, &mut session, &HashSet::new()).expect("최근 기록 삭제");

        assert_eq!(outcome.removed, 0);
        assert_eq!(outcome.skipped_with_drafts, 1);
        assert!(paths.project_dir(&project.id).exists());

        cleanup(&paths);
    }

    /// 비어 있는 버퍼 디렉터리는 초안이 아니다 — 미러를 다 정리한 프로젝트까지 남기면 목록이
    /// 영영 비지 않는다.
    #[test]
    fn 최근_기록_삭제는_빈_버퍼_디렉터리를_초안으로_보지_않는다() {
        let paths = temp_paths();
        let project = Project {
            id: ProjectId::new(),
            root: "/tmp/empty-buffers".to_string(),
            name: "empty-buffers".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 1.0,
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &project).expect("프로젝트 저장");
        std::fs::create_dir_all(paths.buffers_dir(&project.id).join("untitled")).expect("빈 버퍼 디렉터리");

        let mut session = SessionState::default();
        let outcome = forget_recent_projects(&paths, &mut session, &HashSet::new()).expect("최근 기록 삭제");

        assert_eq!(outcome.removed, 1);
        assert_eq!(outcome.skipped_with_drafts, 0);
        assert!(!paths.project_dir(&project.id).exists());

        cleanup(&paths);
    }

    #[test]
    fn 최근_기록_삭제는_지울_레코드가_없으면_0을_돌려준다() {
        let paths = temp_paths();

        let mut session = SessionState::default();
        let outcome = forget_recent_projects(&paths, &mut session, &HashSet::new()).expect("최근 기록 삭제");

        assert_eq!(outcome.removed, 0);
        assert!(!outcome.groups_changed);

        cleanup(&paths);
    }

    #[test]
    fn 존재하지_않는_경로를_열면_로케일_키가_붙은_notfound_다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let missing = paths.data_dir.join("does-not-exist");

        let error = open_project(&paths, &mut session, &mut projects, &missing, true, detect_terminal_only).expect_err("열기 실패");

        match error {
            AppError::Localized(localized) => {
                assert_eq!(localized.kind, AppErrorKind::NotFound);
                assert_eq!(localized.key, "error.project.pathNotFound");
                assert_eq!(
                    localized.args.get("path").map(String::as_str),
                    Some(missing.display().to_string().as_str())
                );
            }
            other => panic!("로케일 키가 붙은 NotFound 여야 합니다: {other:?}"),
        }

        cleanup(&paths);
    }

    #[test]
    fn 동일_root_재열기는_기존_프로젝트를_반환한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let first = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("open");
        assert!(!first.already_open);

        let second = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("open again");
        assert!(second.already_open);
        assert_eq!(first.project.id, second.project.id);
        assert_eq!(projects.len(), 1);
        assert_eq!(session.projects.len(), 1);

        cleanup(&paths);
    }

    #[test]
    fn 이력이_있는_root는_기존_id를_재사용한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();
        let canonical_root = std::fs::canonicalize(&project_root).unwrap().to_string_lossy().to_string();

        let previous_id = ProjectId::new();
        let history = Project {
            id: previous_id.clone(),
            root: canonical_root,
            name: "workspace".to_string(),
            capabilities: vec![CapabilityKind::Terminal],
            root_missing: false,
            last_opened_at: 1_000.0,
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &history).expect("seed history");

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let opened = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("open");
        assert_eq!(opened.project.id, previous_id);
        assert!(!opened.already_open);

        cleanup(&paths);
    }

    /// Locks in the precondition `commands::project_open`'s `ProjectActivated` emit relies on
    /// (`docs/acknowledge/2026-08-25-d42-e2e-defects-contract.md` §3, item c): `open_project` sets
    /// `session.active_project` on *every* path, not only a first-time open, so the command must
    /// fan that activation out unconditionally rather than only inside its `!already_open` branch.
    #[test]
    fn open_project은_already_open_여부와_무관하게_session_active_project를_대상_프로젝트로_설정한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let first = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("open");
        assert!(!first.already_open);
        assert_eq!(session.active_project, Some(first.project.id.clone()));

        session.active_project = None;

        let second = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("re-open");
        assert!(second.already_open);
        assert_eq!(session.active_project, Some(second.project.id));

        cleanup(&paths);
    }

    #[test]
    fn capabilities는_주입된_검출_결과를_그대로_기록한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("repo");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let opened = open_project(&paths, &mut session, &mut projects, &project_root, true, |_root| {
            vec![CapabilityKind::Git, CapabilityKind::Terminal]
        })
        .expect("open");

        assert_eq!(opened.project.capabilities, vec![CapabilityKind::Git, CapabilityKind::Terminal]);

        cleanup(&paths);
    }

    #[test]
    fn reorder는_누락된_id를_뒤에_보존한다() {
        let paths = temp_paths();
        let mut session = SessionState {
            projects: vec![
                ProjectRef {
                    id: ProjectId("prj-a".to_string()),
                    root: "/a".to_string(),
                    name: "a".to_string(),
                    display: ProjectDisplay::default(),
                    root_missing: false,
                },
                ProjectRef {
                    id: ProjectId("prj-b".to_string()),
                    root: "/b".to_string(),
                    name: "b".to_string(),
                    display: ProjectDisplay::default(),
                    root_missing: false,
                },
                ProjectRef {
                    id: ProjectId("prj-c".to_string()),
                    root: "/c".to_string(),
                    name: "c".to_string(),
                    display: ProjectDisplay::default(),
                    root_missing: false,
                },
            ],
            ..SessionState::default()
        };

        reorder_projects(
            &paths,
            &mut session,
            &[ProjectId("prj-c".to_string()), ProjectId("prj-missing".to_string())],
        )
        .expect("reorder");

        let ids: Vec<_> = session.projects.iter().map(|reference| reference.id.as_str().to_string()).collect();
        assert_eq!(ids, vec!["prj-c", "prj-a", "prj-b"]);

        cleanup(&paths);
    }

    #[test]
    fn 파손된_session_json은_백업되고_기본값을_반환한다() {
        let paths = temp_paths();
        std::fs::create_dir_all(&paths.data_dir).unwrap();
        std::fs::write(paths.session_file(), b"not json").unwrap();

        let (session, warnings) = load_session(&paths).expect("load");

        assert_eq!(session, SessionState::default());
        assert_eq!(warnings.len(), 1);

        let backup_path = PathBuf::from(format!("{}{}", paths.session_file().display(), BACKUP_SUFFIX));
        assert!(backup_path.exists());
        assert!(!paths.session_file().exists());

        cleanup(&paths);
    }

    #[test]
    fn 루트가_없으면_root_missing이_표시된다() {
        let paths = temp_paths();
        let missing_root = paths.data_dir.join("gone");

        let id = ProjectId::new();
        let project = Project {
            id: id.clone(),
            root: missing_root.to_string_lossy().to_string(),
            name: "gone".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &project).expect("save project");

        let mut session = SessionState::default();
        session.projects.push(ProjectRef {
            id: id.clone(),
            root: project.root.clone(),
            name: project.name.clone(),
            display: ProjectDisplay::default(),
            root_missing: false,
        });
        save_session(&paths, &session).expect("save session");

        let (restored_session, projects, warnings) = restore_session(&paths).expect("restore");

        assert_eq!(projects.len(), 1);
        assert!(projects[0].root_missing);
        assert!(
            restored_session.projects[0].root_missing,
            "사이드바·슬롯 헤더는 ProjectRef 만 읽으므로 미러가 없으면 루트 부재를 그릴 수 없다"
        );
        assert!(warnings.is_empty());

        cleanup(&paths);
    }

    /// 드라이브가 돌아온 뒤 같은 루트를 다시 열면 `already_open` 분기를 타는데, 여기서
    /// 재계산하지 않으면 부팅 때 세운 플래그가 세션 내내 남아 멀쩡한 프로젝트에 경고가 붙는다.
    #[test]
    fn 이미_열린_프로젝트를_다시_열면_root_missing_미러가_해제된다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let opened = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("open");
        projects.get_mut(&opened.project.id).expect("프로젝트").root_missing = true;
        session.projects[0].root_missing = true;

        let reopened = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("reopen");

        assert!(reopened.already_open);
        assert!(!reopened.project.root_missing);
        assert!(!projects[&opened.project.id].root_missing);
        assert!(!session.projects[0].root_missing);

        cleanup(&paths);
    }

    #[test]
    fn close_project은_세션과_활성프로젝트에서_제거한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let opened = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("open");
        session.active_project = Some(opened.project.id.clone());

        close_project(&paths, &mut session, &mut projects, &opened.project.id).expect("close");

        assert!(projects.is_empty());
        assert!(session.projects.is_empty());
        assert_eq!(session.active_project, None);

        cleanup(&paths);
    }

    #[test]
    fn activate_project은_세션에_없는_id면_실패한다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let result = activate_project(&paths, &mut session, &mut projects, &ProjectId::new());

        assert!(result.is_err());

        cleanup(&paths);
    }

    #[test]
    fn open_project은_재열기_시에도_last_opened_at을_갱신한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();

        let first = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("open");
        let second = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("open again");

        assert!(second.already_open);
        assert!(
            second.project.last_opened_at >= first.project.last_opened_at,
            "재열기는 last_opened_at 을 뒤로 미루거나 최소한 유지해야 한다"
        );
        let persisted = projects.get(&second.project.id).expect("in-memory project");
        assert_eq!(
            persisted.last_opened_at, second.project.last_opened_at,
            "재열기 결과가 in-memory 맵에도 반영돼야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn activate_project은_last_opened_at을_갱신하고_영속화한다() {
        let paths = temp_paths();
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();

        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let opened = open_project(&paths, &mut session, &mut projects, &project_root, true, detect_terminal_only).expect("open");

        let before = projects
            .get(&opened.project.id)
            .expect("open 직후 in-memory project")
            .last_opened_at;
        projects.get_mut(&opened.project.id).expect("mutate before activate").last_opened_at = 0.0;

        activate_project(&paths, &mut session, &mut projects, &opened.project.id).expect("activate");

        let after_memory = projects
            .get(&opened.project.id)
            .expect("activate 후 in-memory project")
            .last_opened_at;
        assert!(after_memory > 0.0, "activate 는 last_opened_at 을 다시 채워야 한다");
        assert!(after_memory >= before, "activate 이후 시각이 최초 open 시각보다 과거일 수 없다");

        let (persisted, _warnings) = load_project(&paths, &opened.project.id).expect("load persisted project");
        assert_eq!(
            persisted.expect("project.json 이 존재해야 한다").last_opened_at,
            after_memory,
            "activate 의 갱신은 디스크에도 저장돼야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn list_recent_projects은_last_opened_at_내림차순으로_정렬한다() {
        let paths = temp_paths();

        let older = Project {
            id: ProjectId("prj-older".to_string()),
            root: "/older".to_string(),
            name: "older".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 1_000.0,
            display: ProjectDisplay::default(),
        };
        let newer = Project {
            id: ProjectId("prj-newer".to_string()),
            root: "/newer".to_string(),
            name: "newer".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 2_000.0,
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &older).expect("seed older");
        save_project(&paths, &newer).expect("seed newer");

        let recent = list_recent_projects(&paths).expect("list recent");

        let ids: Vec<_> = recent.iter().map(|project| project.id.as_str().to_string()).collect();
        assert_eq!(ids, vec!["prj-newer", "prj-older"]);

        cleanup(&paths);
    }

    #[test]
    fn list_recent_projects은_세션에_없는_기록도_포함하고_root_missing을_표시한다() {
        let paths = temp_paths();
        let missing_root = paths.data_dir.join("gone-forever");

        let closed = Project {
            id: ProjectId::new(),
            root: missing_root.to_string_lossy().to_string(),
            name: "gone-forever".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 500.0,
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &closed).expect("seed closed history");

        let recent = list_recent_projects(&paths).expect("list recent");

        assert_eq!(recent.len(), 1);
        assert!(
            recent[0].root_missing,
            "세션에 없어도 디스크 기록이면 목록에 포함되고 root_missing 이 표시돼야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn list_recent_projects은_손상된_기록을_건너뛰고_bak으로_rename하지_않는다() {
        let paths = temp_paths();

        let healthy = Project {
            id: ProjectId("prj-healthy".to_string()),
            root: "/healthy".to_string(),
            name: "healthy".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 1_000.0,
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &healthy).expect("seed healthy");

        let corrupted_dir = paths.data_dir.join("projects").join("prj-corrupted");
        std::fs::create_dir_all(&corrupted_dir).unwrap();
        let corrupted_path = corrupted_dir.join("project.json");
        std::fs::write(&corrupted_path, b"not json").unwrap();

        let recent = list_recent_projects(&paths).expect("list recent");

        assert_eq!(
            recent.iter().map(|project| project.id.as_str()).collect::<Vec<_>>(),
            vec!["prj-healthy"],
            "손상된 기록은 건너뛰고 나머지는 목록에 남아야 한다"
        );
        assert!(
            corrupted_path.exists(),
            "읽기 전용 조회는 손상된 project.json 을 .bak 으로 rename 해서는 안 된다"
        );

        cleanup(&paths);
    }

    #[test]
    fn last_opened_at이_없는_구버전_project_json도_기본값으로_파싱된다() {
        let paths = temp_paths();
        std::fs::create_dir_all(paths.data_dir.join("projects").join("prj-legacy")).unwrap();
        let legacy_json = serde_json::json!({
            "id": "prj-legacy",
            "root": "/legacy",
            "name": "legacy",
        });
        std::fs::write(
            paths.data_dir.join("projects").join("prj-legacy").join("project.json"),
            serde_json::to_vec(&legacy_json).unwrap(),
        )
        .unwrap();

        let (loaded, warnings) = load_project(&paths, &ProjectId("prj-legacy".to_string())).expect("load legacy project");

        let project = loaded.expect("구버전 필드 누락도 파싱에 성공해야 한다");
        assert_eq!(project.last_opened_at, 0.0);
        assert!(warnings.is_empty());

        cleanup(&paths);
    }

    fn open_workspace(paths: &AppPaths, session: &mut SessionState, projects: &mut HashMap<ProjectId, Project>) -> ProjectId {
        let project_root = paths.data_dir.join("workspace");
        std::fs::create_dir_all(&project_root).unwrap();
        open_project(paths, session, projects, &project_root, true, detect_terminal_only)
            .expect("open")
            .project
            .id
    }

    fn patch_of(icon: Option<&str>, label: Option<&str>, color: Option<&str>) -> ProjectDisplayPatch {
        ProjectDisplayPatch {
            icon: icon.map(str::to_string),
            label: label.map(str::to_string),
            color: color.map(str::to_string),
        }
    }

    #[test]
    fn merge_display은_아이콘_이름_규격만_통과시킨다() {
        let empty = ProjectDisplay::default();
        let too_long = "a".repeat(DISPLAY_ICON_NAME_MAX_BYTES + 1);

        assert_eq!(
            merge_display(&empty, &patch_of(Some("square-code-2"), None, None))
                .expect("허용 형식")
                .icon,
            Some("square-code-2".to_string())
        );

        for rejected in ["Rocket", "rocket!", "아이콘", "ro ket", too_long.as_str()] {
            assert_eq!(
                merge_display(&empty, &patch_of(Some(rejected), None, None))
                    .expect_err("규격 밖 아이콘 이름은 거부돼야 한다")
                    .kind(),
                AppErrorKind::InvalidArgument,
                "거부 대상: {rejected}"
            );
        }
    }

    #[test]
    fn merge_display은_아이콘_이름_최대_바이트_경계를_허용한다() {
        let empty = ProjectDisplay::default();
        let at_limit = "a".repeat(DISPLAY_ICON_NAME_MAX_BYTES);

        assert_eq!(
            merge_display(&empty, &patch_of(Some(at_limit.as_str()), None, None))
                .expect("상한과 같은 길이는 허용")
                .icon,
            Some(at_limit)
        );
    }

    #[test]
    fn merge_display은_라벨_길이를_바이트가_아니라_코드포인트로_센다() {
        let empty = ProjectDisplay::default();
        let four_emoji = "🚀🚀🚀🚀";
        assert!(
            four_emoji.len() > DISPLAY_LABEL_MAX_CODEPOINTS,
            "테스트 전제: UTF-8 바이트 수가 상한을 넘어야 한다"
        );

        assert_eq!(
            merge_display(&empty, &patch_of(None, Some(four_emoji), None))
                .expect("코드포인트 4개는 허용")
                .label,
            Some(four_emoji.to_string())
        );
    }

    #[test]
    fn merge_display은_제어문자만_있는_라벨을_해제로_해석한다() {
        let existing = ProjectDisplay {
            icon: None,
            label: Some("TA".to_string()),
            color: None,
        };

        assert_eq!(
            merge_display(&existing, &patch_of(None, Some("\n\t\u{1b}"), None))
                .expect("정리 후 빈 라벨")
                .label,
            None
        );
    }

    #[test]
    fn merge_display은_세_축을_한_번에_해제한다() {
        let existing = ProjectDisplay {
            icon: Some("rocket".to_string()),
            label: Some("TA".to_string()),
            color: Some("lane5".to_string()),
        };

        assert_eq!(
            merge_display(&existing, &patch_of(Some(""), Some(""), Some(""))).expect("전부 해제"),
            ProjectDisplay::default(),
            "빈 문자열 3개는 기본 표시로 되돌려야 한다"
        );
    }

    #[test]
    fn merge_display은_라벨을_정리한_뒤_길이를_판정한다() {
        let empty = ProjectDisplay::default();

        assert_eq!(
            merge_display(&empty, &patch_of(None, Some("  가나\t다라  "), None))
                .expect("정리 후 4자")
                .label,
            Some("가나다라".to_string()),
            "제어문자 제거·trim 이후의 코드포인트 수로 판정해야 한다"
        );
        assert_eq!(
            merge_display(&empty, &patch_of(None, Some("   "), None))
                .expect("공백만 남는 라벨")
                .label,
            None,
            "정리 후 비는 라벨은 거부가 아니라 해제로 해석한다"
        );
        assert_eq!(
            merge_display(&empty, &patch_of(None, Some("가나다라마"), None))
                .expect_err("최대 코드포인트 초과는 거부돼야 한다")
                .kind(),
            AppErrorKind::InvalidArgument
        );
    }

    #[test]
    fn merge_display은_lane_색_토큰_목록만_통과시킨다() {
        let empty = ProjectDisplay::default();

        for accepted in ["lane1", "lane12"] {
            assert_eq!(
                merge_display(&empty, &patch_of(None, None, Some(accepted)))
                    .expect("허용 토큰")
                    .color,
                Some(accepted.to_string())
            );
        }

        for rejected in ["lane0", "lane13", "lane01", "lane+1", "red", "--taide-graph-lane1"] {
            assert_eq!(
                merge_display(&empty, &patch_of(None, None, Some(rejected)))
                    .expect_err("목록 밖 색 토큰은 거부돼야 한다")
                    .kind(),
                AppErrorKind::InvalidArgument,
                "거부 대상: {rejected}"
            );
        }
    }

    #[test]
    fn merge_display의_빈_문자열은_해제하고_미지정_축은_유지한다() {
        let existing = ProjectDisplay {
            icon: Some("rocket".to_string()),
            label: Some("TA".to_string()),
            color: Some("lane5".to_string()),
        };

        let cleared = merge_display(&existing, &patch_of(Some(""), None, None)).expect("아이콘만 해제");
        assert_eq!(cleared.icon, None);
        assert_eq!(cleared.label, existing.label);
        assert_eq!(cleared.color, existing.color);

        assert_eq!(
            merge_display(&existing, &ProjectDisplayPatch::default()).expect("빈 패치"),
            existing,
            "미지정 패치는 기존 값을 그대로 유지해야 한다"
        );
    }

    #[test]
    fn set_project_display은_project_json과_session_json에_같은_값을_남긴다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);

        set_project_display(
            &paths,
            &mut session,
            &mut projects,
            &id,
            &patch_of(Some("database"), Some(" TA "), Some("lane7")),
        )
        .expect("set display");

        let expected = ProjectDisplay {
            icon: Some("database".to_string()),
            label: Some("TA".to_string()),
            color: Some("lane7".to_string()),
        };
        assert_eq!(projects.get(&id).expect("in-memory project").display, expected);

        let (persisted, _warnings) = load_project(&paths, &id).expect("load project.json");
        assert_eq!(persisted.expect("project.json").display, expected);

        let (reloaded, _warnings) = load_session(&paths).expect("load session.json");
        assert_eq!(
            reloaded
                .projects
                .iter()
                .find(|reference| reference.id == id)
                .expect("session ref")
                .display,
            expected,
            "session.json 의 ProjectRef 미러도 재로드 후 같은 값이어야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn set_project_display은_거부된_패치를_부분_적용하지_않는다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);

        set_project_display(&paths, &mut session, &mut projects, &id, &patch_of(Some("rocket"), None, None)).expect("set display");

        let error = set_project_display(
            &paths,
            &mut session,
            &mut projects,
            &id,
            &patch_of(Some("database"), Some("다섯글자라벨"), None),
        )
        .expect_err("라벨이 규격 밖이면 전체가 거부돼야 한다");

        assert_eq!(error.kind(), AppErrorKind::InvalidArgument);
        assert_eq!(
            projects.get(&id).expect("in-memory project").display,
            ProjectDisplay {
                icon: Some("rocket".to_string()),
                label: None,
                color: None,
            },
            "거부된 패치는 같은 호출의 유효한 축도 적용하지 않아야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn set_project_display은_열려_있지_않은_프로젝트면_notfound_이고_아무것도_저장하지_않는다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let missing = ProjectId("prj-missing".to_string());

        let error = set_project_display(&paths, &mut session, &mut projects, &missing, &patch_of(Some("rocket"), None, None))
            .expect_err("열려 있지 않은 프로젝트");

        assert_eq!(error.kind(), AppErrorKind::NotFound);
        assert!(session.projects.is_empty(), "세션 미러가 생기면 안 된다");
        assert!(!paths.session_file().exists(), "session.json 을 쓰면 안 된다");

        cleanup(&paths);
    }

    #[test]
    fn set_project_display은_해제_패치를_양쪽_파일에_미러한다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);

        set_project_display(
            &paths,
            &mut session,
            &mut projects,
            &id,
            &patch_of(Some("rocket"), Some("TA"), Some("lane2")),
        )
        .expect("set display");
        set_project_display(&paths, &mut session, &mut projects, &id, &patch_of(Some(""), Some(""), Some(""))).expect("clear display");

        let (persisted, _warnings) = load_project(&paths, &id).expect("load project.json");
        assert_eq!(persisted.expect("project.json").display, ProjectDisplay::default());

        let (reloaded, _warnings) = load_session(&paths).expect("load session.json");
        assert_eq!(
            reloaded
                .projects
                .iter()
                .find(|reference| reference.id == id)
                .expect("session ref")
                .display,
            ProjectDisplay::default(),
            "해제도 저장과 같은 경로로 session.json 에 미러돼야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn set_project_display은_root와_name_미러를_건드리지_않는다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);
        let before = session
            .projects
            .iter()
            .find(|reference| reference.id == id)
            .expect("session ref")
            .clone();

        set_project_display(&paths, &mut session, &mut projects, &id, &patch_of(None, Some("TA"), None)).expect("set display");

        let after = session.projects.iter().find(|reference| reference.id == id).expect("session ref");
        assert_eq!(after.root, before.root);
        assert_eq!(after.name, before.name);
        assert_eq!(session.projects.len(), 1, "미러 갱신이 항목을 늘리면 안 된다");

        cleanup(&paths);
    }

    #[test]
    fn 닫았다_다시_연_프로젝트는_표시_설정을_유지한다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let mut projects = HashMap::new();
        let id = open_workspace(&paths, &mut session, &mut projects);

        set_project_display(
            &paths,
            &mut session,
            &mut projects,
            &id,
            &patch_of(Some("database"), None, Some("lane3")),
        )
        .expect("set display");
        close_project(&paths, &mut session, &mut projects, &id).expect("close");

        let reopened = open_workspace(&paths, &mut session, &mut projects);

        assert_eq!(reopened, id, "같은 루트는 기존 id 를 재사용해야 한다");
        assert_eq!(
            projects.get(&id).expect("재열기한 프로젝트").display,
            ProjectDisplay {
                icon: Some("database".to_string()),
                label: None,
                color: Some("lane3".to_string()),
            },
            "재열기가 사용자 지정 표시를 초기화해서는 안 된다"
        );
        assert_eq!(
            session
                .projects
                .iter()
                .find(|reference| reference.id == id)
                .expect("session ref")
                .display,
            projects.get(&id).expect("재열기한 프로젝트").display,
            "재열기 후에도 session.json 미러가 project.json 과 같아야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn display_필드가_없는_구버전_기록은_기본_표시로_파싱된다() {
        let paths = temp_paths();
        std::fs::create_dir_all(paths.data_dir.join("projects").join("prj-legacy")).unwrap();
        std::fs::write(
            paths.data_dir.join("projects").join("prj-legacy").join("project.json"),
            serde_json::to_vec(&serde_json::json!({ "id": "prj-legacy", "root": "/legacy", "name": "legacy" })).unwrap(),
        )
        .unwrap();
        std::fs::write(
            paths.session_file(),
            serde_json::to_vec(&serde_json::json!({
                "version": crate::domain::project::types::SESSION_SCHEMA_VERSION,
                "projects": [{ "id": "prj-legacy", "root": "/legacy", "name": "legacy" }],
                "activeProject": null,
            }))
            .unwrap(),
        )
        .unwrap();

        let (loaded, project_warnings) = load_project(&paths, &ProjectId("prj-legacy".to_string())).expect("load legacy project");
        assert_eq!(loaded.expect("project.json").display, ProjectDisplay::default());
        assert!(project_warnings.is_empty());

        let (session, session_warnings) = load_session(&paths).expect("load legacy session");
        assert!(session_warnings.is_empty(), "display 가 없다고 세션이 손상 처리되면 안 된다");
        assert_eq!(session.projects[0].display, ProjectDisplay::default());

        cleanup(&paths);
    }

    fn 세션_레퍼런스(id: &str) -> ProjectRef {
        ProjectRef {
            id: ProjectId(id.to_string()),
            root: format!("/tmp/{id}"),
            name: id.to_string(),
            display: ProjectDisplay::default(),
            root_missing: false,
        }
    }

    fn 슬롯_세션(ids: &[&str]) -> SessionState {
        let mut session = SessionState {
            projects: ids.iter().map(|id| 세션_레퍼런스(id)).collect(),
            active_project: ids.first().map(|id| ProjectId((*id).to_string())),
            ..SessionState::default()
        };
        normalize_shell_slots(&mut session, None);
        session
    }

    fn 슬롯_목록(session: &SessionState) -> Vec<(ShellSlotId, ProjectId)> {
        session.shell_slots.as_ref().map(shell_slots::leaves).unwrap_or_default()
    }

    #[test]
    fn 슬롯_트리가_없던_세션은_활성_프로젝트_하나짜리_리프로_해석된다() {
        let session = 슬롯_세션(&["a", "b"]);

        let slots = 슬롯_목록(&session);
        assert_eq!(slots.len(), 1, "구버전 세션은 슬롯 하나로 시작한다");
        assert_eq!(slots[0].1, ProjectId("a".to_string()));
        assert_eq!(session.focused_shell_slot, Some(slots[0].0.clone()));
        assert_eq!(session.active_project, Some(ProjectId("a".to_string())));
    }

    #[test]
    fn 열린_프로젝트가_없으면_슬롯도_포커스도_활성도_없다() {
        let session = 슬롯_세션(&[]);

        assert!(session.shell_slots.is_none());
        assert!(session.focused_shell_slot.is_none());
        assert!(session.active_project.is_none());
    }

    #[test]
    fn 복원_정규화는_세션에_없는_리프를_지우고_남은_트리로_포커스를_다시_잡는다() {
        let mut session = 슬롯_세션(&["a"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        session.projects.push(세션_레퍼런스("b"));
        let tree = session.shell_slots.as_mut().expect("트리");
        let slot_b = shell_slots::split_slot(tree, &slot_a, ShellSlotEdge::Right, &ProjectId("b".to_string()))
            .expect("분할")
            .expect("b 슬롯");
        session.focused_shell_slot = Some(slot_b);

        session.projects.retain(|reference| reference.id != ProjectId("b".to_string()));
        normalize_shell_slots(&mut session, None);

        assert_eq!(슬롯_목록(&session), vec![(slot_a.clone(), ProjectId("a".to_string()))]);
        assert_eq!(session.focused_shell_slot, Some(slot_a));
        assert_eq!(session.active_project, Some(ProjectId("a".to_string())));
    }

    #[test]
    fn 이미_슬롯에_있는_프로젝트의_활성화는_그_슬롯으로_포커스만_옮긴다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a", "b"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let tree = session.shell_slots.as_mut().expect("트리");
        let slot_b = shell_slots::split_slot(tree, &slot_a, ShellSlotEdge::Right, &ProjectId("b".to_string()))
            .expect("분할")
            .expect("b 슬롯");
        let mut projects = HashMap::new();

        activate_project(&paths, &mut session, &mut projects, &ProjectId("b".to_string())).expect("활성화");

        assert_eq!(session.focused_shell_slot, Some(slot_b));
        assert_eq!(session.active_project, Some(ProjectId("b".to_string())));
        assert_eq!(슬롯_목록(&session).len(), 2, "이미 떠 있는 프로젝트는 슬롯을 늘리지 않는다");

        cleanup(&paths);
    }

    #[test]
    fn 슬롯에_없는_프로젝트의_활성화는_포커스_슬롯의_프로젝트를_교체한다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a", "b"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let mut projects = HashMap::new();

        activate_project(&paths, &mut session, &mut projects, &ProjectId("b".to_string())).expect("활성화");

        assert_eq!(슬롯_목록(&session), vec![(slot_a.clone(), ProjectId("b".to_string()))]);
        assert_eq!(session.focused_shell_slot, Some(slot_a));

        cleanup(&paths);
    }

    #[test]
    fn 열려있지_않은_프로젝트의_활성화는_거부되고_슬롯도_그대로다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a"]);
        let before = 슬롯_목록(&session);
        let mut projects = HashMap::new();

        let error = activate_project(&paths, &mut session, &mut projects, &ProjectId("zzz".to_string())).expect_err("거부");

        assert!(matches!(error, AppError::NotFound(_)));
        assert_eq!(슬롯_목록(&session), before);

        cleanup(&paths);
    }

    #[test]
    fn 프로젝트를_닫으면_같은_호출에서_슬롯이_축약되고_포커스가_재계산된다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a", "b"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let tree = session.shell_slots.as_mut().expect("트리");
        let slot_b = shell_slots::split_slot(tree, &slot_a, ShellSlotEdge::Right, &ProjectId("b".to_string()))
            .expect("분할")
            .expect("b 슬롯");
        session.focused_shell_slot = Some(slot_b);
        session.active_project = Some(ProjectId("b".to_string()));
        let mut projects = HashMap::new();

        close_project(&paths, &mut session, &mut projects, &ProjectId("b".to_string())).expect("닫기");

        assert_eq!(슬롯_목록(&session), vec![(slot_a.clone(), ProjectId("a".to_string()))]);
        assert_eq!(session.focused_shell_slot, Some(slot_a));
        assert_eq!(session.active_project, Some(ProjectId("a".to_string())));

        cleanup(&paths);
    }

    #[test]
    fn 슬롯에_없는_프로젝트를_닫아도_포커스는_움직이지_않는다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a", "b"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let mut projects = HashMap::new();

        close_project(&paths, &mut session, &mut projects, &ProjectId("b".to_string())).expect("닫기");

        assert_eq!(슬롯_목록(&session), vec![(slot_a.clone(), ProjectId("a".to_string()))]);
        assert_eq!(session.focused_shell_slot, Some(slot_a));

        cleanup(&paths);
    }

    /// 화면상 `[A | B | C]` — 슬롯 분할은 항상 이진이라 트리는 `Split(A, Split(B, C))` 가 된다.
    fn 삼분할_슬롯_세션() -> (SessionState, Vec<ShellSlotId>) {
        let mut session = 슬롯_세션(&["a", "b", "c"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let tree = session.shell_slots.as_mut().expect("트리");
        let slot_b = shell_slots::split_slot(tree, &slot_a, ShellSlotEdge::Right, &ProjectId("b".to_string()))
            .expect("첫 분할")
            .expect("b 슬롯");
        let slot_c = shell_slots::split_slot(tree, &slot_b, ShellSlotEdge::Right, &ProjectId("c".to_string()))
            .expect("둘째 분할")
            .expect("c 슬롯");
        (session, vec![slot_a, slot_b, slot_c])
    }

    /// 포커스 슬롯이 사라지면 남는 후보는 "첫 슬롯" 뿐이라, 셋 중 마지막을 닫을 때 포커스가
    /// 화면 반대편 끝으로 튀었다(활성 프로젝트·네이티브 File 메뉴·⌘P 대상까지 함께).
    #[test]
    fn 포커스한_슬롯을_닫으면_이웃_슬롯이_포커스를_승계한다() {
        let paths = temp_paths();
        let (mut session, slots) = 삼분할_슬롯_세션();
        session.focused_shell_slot = Some(slots[2].clone());
        session.active_project = Some(ProjectId("c".to_string()));

        close_shell_slot(&paths, &mut session, &slots[2]).expect("슬롯 닫기");

        assert_eq!(session.focused_shell_slot, Some(slots[1].clone()));
        assert_eq!(session.active_project, Some(ProjectId("b".to_string())));

        cleanup(&paths);
    }

    #[test]
    fn 포커스한_첫_슬롯을_닫으면_다음_슬롯이_포커스를_승계한다() {
        let paths = temp_paths();
        let (mut session, slots) = 삼분할_슬롯_세션();
        session.focused_shell_slot = Some(slots[0].clone());
        session.active_project = Some(ProjectId("a".to_string()));

        close_shell_slot(&paths, &mut session, &slots[0]).expect("슬롯 닫기");

        assert_eq!(session.focused_shell_slot, Some(slots[1].clone()));
        assert_eq!(session.active_project, Some(ProjectId("b".to_string())));

        cleanup(&paths);
    }

    #[test]
    fn 포커스하지_않은_슬롯을_닫으면_포커스는_그대로다() {
        let paths = temp_paths();
        let (mut session, slots) = 삼분할_슬롯_세션();
        session.focused_shell_slot = Some(slots[0].clone());
        session.active_project = Some(ProjectId("a".to_string()));

        close_shell_slot(&paths, &mut session, &slots[2]).expect("슬롯 닫기");

        assert_eq!(session.focused_shell_slot, Some(slots[0].clone()));
        assert_eq!(session.active_project, Some(ProjectId("a".to_string())));

        cleanup(&paths);
    }

    /// 같은 승계를 `close_project` 경로도 타야 한다 — 실사용 빈도는 이쪽이 더 높다. 사이드바 순서
    /// (`session.projects`)를 슬롯 배치와 다르게 둬야 옛 폴백(활성 프로젝트를 목록 맨 뒤로 되돌린
    /// 뒤 그 슬롯을 잡는 ②분기)이 이웃과 갈린다.
    #[test]
    fn 포커스한_슬롯의_프로젝트를_닫아도_이웃_슬롯이_포커스를_승계한다() {
        let paths = temp_paths();
        let (mut session, slots) = 삼분할_슬롯_세션();
        session.projects = ["b", "c", "a"].into_iter().map(세션_레퍼런스).collect();
        session.focused_shell_slot = Some(slots[2].clone());
        session.active_project = Some(ProjectId("c".to_string()));
        let mut projects = HashMap::new();

        close_project(&paths, &mut session, &mut projects, &ProjectId("c".to_string())).expect("프로젝트 닫기");

        assert_eq!(session.focused_shell_slot, Some(slots[1].clone()));
        assert_eq!(session.active_project, Some(ProjectId("b".to_string())));

        cleanup(&paths);
    }

    #[test]
    fn 마지막_슬롯은_닫을_수_없다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();

        let error = close_shell_slot(&paths, &mut session, &slot_a).expect_err("거부");

        assert!(matches!(error, AppError::InvalidArgument(_)));
        assert_eq!(슬롯_목록(&session).len(), 1);

        cleanup(&paths);
    }

    #[test]
    fn 슬롯만_닫으면_프로젝트는_열린_채로_남는다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a", "b"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let tree = session.shell_slots.as_mut().expect("트리");
        let slot_b = shell_slots::split_slot(tree, &slot_a, ShellSlotEdge::Right, &ProjectId("b".to_string()))
            .expect("분할")
            .expect("b 슬롯");

        close_shell_slot(&paths, &mut session, &slot_b).expect("슬롯 닫기");

        assert_eq!(슬롯_목록(&session), vec![(slot_a, ProjectId("a".to_string()))]);
        assert_eq!(session.projects.len(), 2, "슬롯만 닫았으므로 프로젝트는 둘 다 열려 있다");

        cleanup(&paths);
    }

    #[test]
    fn 같은_프로젝트를_다른_슬롯에_또_두는_요청은_거부된다() {
        let mut session = 슬롯_세션(&["a", "b"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let tree = session.shell_slots.as_mut().expect("트리");
        let slot_b = shell_slots::split_slot(tree, &slot_a, ShellSlotEdge::Right, &ProjectId("b".to_string()))
            .expect("분할")
            .expect("b 슬롯");

        let error =
            ensure_slot_placement_allowed(&session, Some(&ProjectId("a".to_string())), &slot_b, ShellSlotEdge::Replace).expect_err("거부");

        let AppError::Localized(localized) = &error else {
            panic!("로케일 키가 있는 에러여야 한다");
        };
        assert_eq!(localized.key, "error.shellSlot.projectAlreadyInSlot");
        assert_eq!(localized.kind, AppErrorKind::InvalidArgument);
    }

    #[test]
    fn 대상_슬롯을_같은_프로젝트로_교체하는_것은_허용되고_분할은_거부된다() {
        let session = 슬롯_세션(&["a"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let a = ProjectId("a".to_string());

        assert!(ensure_slot_placement_allowed(&session, Some(&a), &slot_a, ShellSlotEdge::Replace).is_ok());
        assert!(
            ensure_slot_placement_allowed(&session, Some(&a), &slot_a, ShellSlotEdge::Right).is_err(),
            "분할은 같은 프로젝트를 두 슬롯에 만드는 셈이라 거부돼야 한다"
        );
    }

    #[test]
    fn 없는_대상_슬롯은_아무것도_열기_전에_거부된다() {
        let session = 슬롯_세션(&["a"]);

        let error = ensure_slot_placement_allowed(&session, None, &ShellSlotId("shellslot-missing".to_string()), ShellSlotEdge::Right)
            .expect_err("거부");

        assert!(matches!(error, AppError::NotFound(_)));
    }

    #[test]
    fn 슬롯에_두면_새_슬롯이_포커스와_활성_프로젝트가_된다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a", "b"]);
        let mut projects = HashMap::new();
        let slot_a = 슬롯_목록(&session)[0].0.clone();

        let new_slot = place_project_in_slot(
            &paths,
            &mut session,
            &mut projects,
            &ProjectId("b".to_string()),
            &slot_a,
            ShellSlotEdge::Bottom,
        )
        .expect("배치");

        assert_eq!(슬롯_목록(&session).len(), 2);
        assert_eq!(session.focused_shell_slot, Some(new_slot.clone()));
        assert_eq!(session.active_project, Some(ProjectId("b".to_string())));
        assert_ne!(new_slot, slot_a);

        cleanup(&paths);
    }

    #[test]
    fn 슬롯_배치는_이미_열린_프로젝트의_last_opened_at을_갱신하고_영속화한다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a", "b"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let b = ProjectId("b".to_string());
        let mut projects = HashMap::from([(
            b.clone(),
            Project {
                id: b.clone(),
                root: "/tmp/b".to_string(),
                name: "b".to_string(),
                capabilities: Vec::new(),
                root_missing: false,
                last_opened_at: 0.0,
                display: ProjectDisplay::default(),
            },
        )]);

        place_project_in_slot(&paths, &mut session, &mut projects, &b, &slot_a, ShellSlotEdge::Bottom).expect("배치");

        let stamped = projects.get(&b).expect("배치 후 in-memory project").last_opened_at;
        assert!(stamped > 0.0, "슬롯 배치도 activate 처럼 recency 를 찍어야 한다");

        let (persisted, _warnings) = load_project(&paths, &b).expect("project.json 적재");
        assert_eq!(
            persisted.expect("project.json 이 존재해야 한다").last_opened_at,
            stamped,
            "슬롯 배치의 갱신은 디스크에도 저장돼야 한다"
        );

        cleanup(&paths);
    }

    #[test]
    fn 슬롯_크기는_인덱스_경로로_저장된다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a", "b"]);
        let slot_a = 슬롯_목록(&session)[0].0.clone();
        let tree = session.shell_slots.as_mut().expect("트리");
        shell_slots::split_slot(tree, &slot_a, ShellSlotEdge::Right, &ProjectId("b".to_string()))
            .expect("분할")
            .expect("b 슬롯");

        set_shell_slot_sizes(&paths, &mut session, &[], vec![25.0, 75.0]).expect("크기 저장");

        let (reloaded, warnings) = load_session(&paths).expect("세션 재적재");
        assert!(warnings.is_empty());
        let Some(ShellSlotTree::Split { sizes, .. }) = reloaded.shell_slots else {
            panic!("루트는 split 이어야 한다");
        };
        assert_eq!(sizes, vec![25.0, 75.0]);

        cleanup(&paths);
    }

    #[test]
    fn 창_크롬은_활성_프로젝트의_구_shell_view에서_한_번만_승격된다() {
        let mut session = 슬롯_세션(&["a", "b"]);
        let mut shell_views: HashMap<ProjectId, ShellViewState> = HashMap::from([
            (
                ProjectId("a".to_string()),
                ShellViewState {
                    zen: true,
                    sidebar_collapsed: false,
                },
            ),
            (
                ProjectId("b".to_string()),
                ShellViewState {
                    zen: false,
                    sidebar_collapsed: true,
                },
            ),
        ]);

        let promoted = promote_legacy_window_chrome(&mut session, &mut shell_views);

        assert_eq!(
            session.window_chrome,
            WindowChrome {
                zen: true,
                sidebar_rail_collapsed: false
            }
        );
        assert_eq!(promoted, vec![ProjectId("a".to_string()), ProjectId("b".to_string())]);
        assert!(shell_views.values().all(|view| !view.zen && !view.sidebar_collapsed));

        let again = promote_legacy_window_chrome(&mut session, &mut shell_views);
        assert!(again.is_empty(), "원본이 비워졌으므로 다음 부팅에 다시 승격되지 않는다");
    }

    #[test]
    fn 창_크롬에_이미_값이_있으면_승격하지_않는다() {
        let mut session = 슬롯_세션(&["a"]);
        session.window_chrome = WindowChrome {
            zen: false,
            sidebar_rail_collapsed: true,
        };
        let mut shell_views: HashMap<ProjectId, ShellViewState> = HashMap::from([(
            ProjectId("a".to_string()),
            ShellViewState {
                zen: true,
                sidebar_collapsed: false,
            },
        )]);

        let promoted = promote_legacy_window_chrome(&mut session, &mut shell_views);

        assert!(promoted.is_empty());
        assert!(!session.window_chrome.zen);
        assert!(shell_views[&ProjectId("a".to_string())].zen, "원본도 건드리지 않는다");
    }

    #[test]
    fn 승격할_구_값이_없으면_아무것도_하지_않는다() {
        let mut session = 슬롯_세션(&["a"]);
        let mut shell_views: HashMap<ProjectId, ShellViewState> = HashMap::from([(ProjectId("a".to_string()), ShellViewState::default())]);

        assert!(promote_legacy_window_chrome(&mut session, &mut shell_views).is_empty());
        assert_eq!(session.window_chrome, WindowChrome::default());
    }

    #[test]
    fn 창_크롬_패치는_지정한_축만_바꾼다() {
        let paths = temp_paths();
        let mut session = 슬롯_세션(&["a"]);

        let chrome = set_window_chrome(
            &paths,
            &mut session,
            &WindowChromePatch {
                zen: Some(true),
                sidebar_rail_collapsed: None,
            },
        )
        .expect("패치");

        assert!(chrome.zen);
        assert!(!chrome.sidebar_rail_collapsed);

        cleanup(&paths);
    }
    fn seed_project_record(paths: &AppPaths, name: &str) -> ProjectId {
        let project = Project {
            id: ProjectId::new(),
            root: format!("/tmp/{name}"),
            name: name.to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        };
        save_project(paths, &project).expect("프로젝트 레코드 저장");
        project.id
    }

    #[test]
    fn 그룹_생성은_이름과_색과_멤버를_저장하고_세션_왕복에서_살아남는다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let first = seed_project_record(&paths, "alpha");
        let second = seed_project_record(&paths, "beta");

        let created = create_group(
            &paths,
            &mut session,
            "  백엔드  ",
            Some("lane3"),
            Some(vec![first.clone(), second.clone(), first.clone()]),
        )
        .expect("그룹 생성");

        assert_eq!(created.name, "백엔드", "이름은 트림돼 저장된다");
        assert_eq!(created.color.as_deref(), Some("lane3"));
        assert_eq!(created.members, vec![first.clone(), second.clone()], "중복 멤버는 제거된다");
        assert!(!created.collapsed);

        let (restored, _) = load_session(&paths).expect("세션 로드");
        assert_eq!(restored.groups, vec![created], "그룹은 session.json 왕복에서 그대로다");

        cleanup(&paths);
    }

    #[test]
    fn 구버전_세션에는_그룹이_없고_기본값으로_읽힌다() {
        let session: SessionState = serde_json::from_str(r#"{"version":1,"projects":[],"activeProject":null}"#).expect("구버전 세션");

        assert!(session.groups.is_empty(), "groups 가 없던 세션도 마이그레이션 없이 읽혀야 합니다");
    }

    #[test]
    fn 그룹_이름과_색은_규격을_벗어나면_거부된다() {
        let paths = temp_paths();
        let mut session = SessionState::default();

        let blank = create_group(&paths, &mut session, "   ", None, None).expect_err("빈 이름은 거부");
        let AppError::Localized(localized) = &blank else {
            panic!("로케일 키가 있는 에러여야 한다");
        };
        assert_eq!(localized.key, "error.projectGroup.invalid");
        assert_eq!(localized.kind, AppErrorKind::InvalidArgument);

        let long = "가".repeat(GROUP_NAME_MAX_CODEPOINTS + 1);
        assert!(
            create_group(&paths, &mut session, &long, None, None).is_err(),
            "상한 초과 이름은 거부"
        );
        assert!(
            create_group(&paths, &mut session, "그룹", Some("crimson"), None).is_err(),
            "팔레트 밖 색은 거부"
        );
        assert!(session.groups.is_empty(), "거부된 생성은 아무것도 남기지 않는다");

        cleanup(&paths);
    }

    #[test]
    fn 그룹_멤버는_레코드가_없는_아이디를_거부한다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let known = seed_project_record(&paths, "alpha");
        let group = create_group(&paths, &mut session, "그룹", None, Some(vec![known.clone()])).expect("그룹 생성");

        let error = set_group_members(
            &paths,
            &mut session,
            &group.id,
            vec![known.clone(), ProjectId::from("prj-ghost".to_string())],
        )
        .expect_err("없는 프로젝트는 멤버가 될 수 없다");

        let AppError::Localized(localized) = &error else {
            panic!("로케일 키가 있는 에러여야 한다");
        };
        assert_eq!(localized.key, "error.projectGroup.memberUnknown");
        assert_eq!(localized.kind, AppErrorKind::InvalidArgument);
        assert_eq!(
            session.groups[0].members,
            vec![known],
            "거부된 멤버 변경은 기존 멤버십을 그대로 둔다"
        );

        cleanup(&paths);
    }

    #[test]
    fn 프로젝트는_한_그룹에만_속한다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let shared = seed_project_record(&paths, "shared");
        let old = create_group(&paths, &mut session, "이전", None, Some(vec![shared.clone()])).expect("이전 그룹");
        let new = create_group(&paths, &mut session, "새", None, None).expect("새 그룹");

        set_group_members(&paths, &mut session, &new.id, vec![shared.clone()]).expect("멤버 이동");

        assert!(
            groups::find(&session.groups, &old.id).expect("이전 그룹").members.is_empty(),
            "이전 그룹에서 빠져야 합니다"
        );
        assert_eq!(groups::find(&session.groups, &new.id).expect("새 그룹").members, vec![shared]);

        cleanup(&paths);
    }

    #[test]
    fn 그룹_이름_색_접힘_변경은_영속화된다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let group = create_group(&paths, &mut session, "처음", Some("lane1"), None).expect("그룹 생성");

        rename_group(&paths, &mut session, &group.id, "나중").expect("이름 변경");
        set_group_color(&paths, &mut session, &group.id, None).expect("색 해제");
        set_group_collapsed(&paths, &mut session, &group.id, true).expect("접기");

        let (restored, _) = load_session(&paths).expect("세션 로드");
        let stored = &restored.groups[0];
        assert_eq!(stored.name, "나중");
        assert_eq!(stored.color, None);
        assert!(stored.collapsed);

        cleanup(&paths);
    }

    #[test]
    fn 없는_그룹을_바꾸거나_지우면_notfound_다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let missing = ProjectGroupId::from("group-missing".to_string());

        assert_eq!(
            rename_group(&paths, &mut session, &missing, "이름").expect_err("없는 그룹").kind(),
            AppErrorKind::NotFound
        );
        assert_eq!(
            delete_group(&paths, &mut session, &missing).expect_err("없는 그룹").kind(),
            AppErrorKind::NotFound
        );
        assert_eq!(
            group_members(&session, &missing).expect_err("없는 그룹").kind(),
            AppErrorKind::NotFound
        );

        cleanup(&paths);
    }

    #[test]
    fn 그룹_삭제는_멤버_프로젝트를_건드리지_않는다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let member = seed_project_record(&paths, "member");
        let group = create_group(&paths, &mut session, "그룹", None, Some(vec![member.clone()])).expect("그룹 생성");

        delete_group(&paths, &mut session, &group.id).expect("그룹 삭제");

        assert!(session.groups.is_empty());
        assert!(paths.project_dir(&member).exists(), "그룹을 지워도 프로젝트 레코드는 남는다");

        cleanup(&paths);
    }

    #[test]
    fn 그룹_재정렬은_지정_순서를_따르고_누락분을_뒤에_붙인다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let first = create_group(&paths, &mut session, "첫", None, None).expect("첫 그룹");
        let second = create_group(&paths, &mut session, "둘", None, None).expect("둘 그룹");
        let third = create_group(&paths, &mut session, "셋", None, None).expect("셋 그룹");

        reorder_groups(&paths, &mut session, &[third.id.clone(), first.id.clone()]).expect("재정렬");

        let (restored, _) = load_session(&paths).expect("세션 로드");
        assert_eq!(
            restored.groups.into_iter().map(|group| group.id).collect::<Vec<_>>(),
            vec![third.id, first.id, second.id]
        );

        cleanup(&paths);
    }

    #[test]
    fn 최근_기록_삭제는_그룹_멤버십도_정리하고_저장한다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let kept = seed_project_record(&paths, "kept");
        let forgotten = seed_project_record(&paths, "forgotten");
        let group = create_group(&paths, &mut session, "그룹", None, Some(vec![kept.clone(), forgotten.clone()])).expect("그룹 생성");

        let open_ids: HashSet<ProjectId> = [kept.clone()].into_iter().collect();
        let outcome = forget_recent_projects(&paths, &mut session, &open_ids).expect("최근 기록 삭제");

        assert_eq!(outcome.removed, 1);
        assert!(outcome.groups_changed, "멤버십이 정리됐으면 그룹 변경으로 보고해야 합니다");
        assert_eq!(session.groups[0].members, vec![kept.clone()]);

        let (restored, _) = load_session(&paths).expect("세션 로드");
        assert_eq!(
            restored.groups[0].members,
            vec![kept],
            "정리된 멤버십이 session.json 에도 저장돼야 합니다"
        );
        assert_eq!(restored.groups[0].id, group.id);

        cleanup(&paths);
    }

    /// 계약 §3 B-1 — `project_forget_recent` 는 이 답으로 `ProjectGroupsChanged` 발행 여부를 가른다.
    /// 그룹이 있어도 지워진 프로젝트가 아무 그룹의 멤버가 아니면 사이드바에 바뀐 것이 없으므로
    /// 변경 없음으로 보고해야 한다(그러면 세션 재저장도 없다).
    #[test]
    fn 최근_기록_삭제가_어느_그룹의_멤버도_건드리지_않으면_변경_없음이다() {
        let paths = temp_paths();
        let mut session = SessionState::default();
        let member = seed_project_record(&paths, "member");
        let outsider = seed_project_record(&paths, "outsider");
        create_group(&paths, &mut session, "그룹", None, Some(vec![member.clone()])).expect("그룹 생성");

        let open_ids: HashSet<ProjectId> = [member.clone()].into_iter().collect();
        let outcome = forget_recent_projects(&paths, &mut session, &open_ids).expect("최근 기록 삭제");

        assert_eq!(outcome.removed, 1, "그룹 밖 레코드는 지워져야 합니다");
        assert!(
            !outcome.groups_changed,
            "지운 프로젝트가 어느 그룹에도 없으면 그룹 변경으로 보고하면 안 됩니다"
        );
        assert!(!paths.project_dir(&outsider).exists());
        assert_eq!(session.groups[0].members, vec![member]);

        cleanup(&paths);
    }

    #[test]
    fn 그룹_멤버_루트는_레코드와_실재하는_디렉토리일_때만_해소된다() {
        let paths = temp_paths();
        let workspace = std::env::temp_dir().join(format!("taide-group-root-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&workspace).expect("작업 디렉토리 생성");

        let present = Project {
            id: ProjectId::new(),
            root: workspace.to_string_lossy().to_string(),
            name: "present".to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at: 0.0,
            display: ProjectDisplay::default(),
        };
        save_project(&paths, &present).expect("프로젝트 저장");
        let gone = seed_project_record(&paths, "gone-from-disk");

        assert_eq!(group_member_root(&paths, &present.id), Some(present.root.clone()));
        assert_eq!(group_member_root(&paths, &gone), None, "루트가 없으면 열 수 없다");
        assert_eq!(
            group_member_root(&paths, &ProjectId::from("prj-ghost".to_string())),
            None,
            "레코드가 없으면 열 수 없다"
        );

        std::fs::remove_dir_all(&workspace).ok();
        cleanup(&paths);
    }
}
