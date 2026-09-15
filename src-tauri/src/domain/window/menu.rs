use tauri::menu::{IsMenuItem, MenuItemBuilder, PredefinedMenuItem, Submenu, SubmenuBuilder};
use tauri::{AppHandle, Manager, Wry};

use crate::constants::RECENT_PROJECT_MENU_LIMIT;
use crate::domain::locale::service as locale_service;
use crate::domain::project::service as project_service;
use crate::domain::project::types::Project;
use crate::ids::ProjectId;
use crate::state::AppState;

pub(crate) const MENU_ID_FILE: &str = "taide-file";
pub(crate) const MENU_ID_OPEN_RECENT: &str = "taide-open-recent";
pub(crate) const MENU_ID_CLEAR_RECENT: &str = "taide-clear-recent";

/// A custom (non-predefined) menu item id for the app menu's Quit entry — see
/// [`super::commands::request_quit`] for why this can't be `PredefinedMenuItem::quit`.
pub(crate) const MENU_ID_QUIT: &str = "taide-quit";

/// The disabled placeholder row shown when the recent list is empty. It has an id only because
/// every `muda` item does; nothing dispatches on it.
const MENU_ID_NO_RECENT_PROJECTS: &str = "taide-no-recent";

/// Prefix of a `File > Open Recent` row's menu id, followed by the project id verbatim —
/// [`parse_recent_menu_id`] is the only reader, and [`menu_action`] the only caller. A prefix
/// rather than a lookup table because menu items are rebuilt on every project change
/// ([`refresh_recent_menu`]) and a table would have to be kept in sync with them.
const RECENT_MENU_ID_PREFIX: &str = "taide-recent:";

/// What a click on one app-menu item means, resolved from the item's id alone. The assembly
/// (`lib.rs`'s `on_menu_event` → `dispatch_menu_action`) matches on this and calls each action's
/// owning domain, so the window domain never reaches into `project::commands` itself
/// (architecture.md §2 — cross-domain calls belong to the assembly; `tests/domain_boundaries.rs`
/// enforces it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MenuAction {
    /// Close the main window, which runs the hot-exit flush handshake — see
    /// [`super::commands::request_quit`].
    Quit,
    /// Reopen the project one `File > Open Recent` row names.
    OpenRecent(ProjectId),
    /// Forget every closed project's persisted record.
    ClearRecent,
    /// Every other id: the predefined items tauri handles itself, the submenu titles, and the
    /// disabled `No Recent Projects` placeholder.
    Ignored,
}

/// The action `menu_id` stands for. Pure — no `AppHandle`, no IO — so the whole id-to-action table
/// is testable without an event loop, and so the assembly decides *where* each action runs: the
/// menu-event handler is called on the main thread, and both resolving a recent row's root
/// ([`recent_project_root`], a disk read) and taking `project_open`'s mutation guard have to
/// happen off it.
pub(crate) fn menu_action(menu_id: &str) -> MenuAction {
    if menu_id == MENU_ID_QUIT {
        return MenuAction::Quit;
    }
    if menu_id == MENU_ID_CLEAR_RECENT {
        return MenuAction::ClearRecent;
    }
    match parse_recent_menu_id(menu_id) {
        Some(project_id) => MenuAction::OpenRecent(project_id),
        None => MenuAction::Ignored,
    }
}

/// One `File > Open Recent` row, resolved from a persisted [`Project`] record: what to draw and
/// whether it can be clicked. Split out from the `muda` item construction so the list's rules —
/// the newest-first cap, the display-label preference, the `root_missing` disable — are testable
/// without an event loop to build real menu items on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecentMenuEntry {
    pub id: ProjectId,
    pub label: String,
    pub enabled: bool,
}

/// The first `limit` of `projects` — which `project::service::list_recent_projects` already orders
/// most-recently-opened first — as menu rows. A project whose folder is gone stays listed but
/// disabled, the same treatment the Welcome screen's recent list gives it: silently dropping it
/// would leave the user wondering where the project went.
pub(crate) fn recent_menu_entries(projects: Vec<Project>, limit: usize) -> Vec<RecentMenuEntry> {
    projects
        .into_iter()
        .take(limit)
        .map(|project| RecentMenuEntry {
            label: recent_menu_label(&project),
            enabled: !project.root_missing,
            id: project.id,
        })
        .collect()
}

/// A project's own short display label when it has one, else its folder name — the same preference
/// `resolveProjectDisplay` applies wherever a project is named as plain text
/// (`src/shared/lib/notification-text.ts`), so a project the user renamed in the sidebar reads the
/// same way in the menu.
fn recent_menu_label(project: &Project) -> String {
    project
        .display
        .label
        .as_deref()
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .unwrap_or(project.name.as_str())
        .to_string()
}

pub(crate) fn recent_menu_item_id(project_id: &ProjectId) -> String {
    format!("{RECENT_MENU_ID_PREFIX}{project_id}")
}

/// The project a `File > Open Recent` row names, or `None` for every other menu id. Project ids
/// carry no `:` (`ids::string_id!` builds them as `prj-<uuid>`), so the prefix split is
/// unambiguous.
pub(crate) fn parse_recent_menu_id(menu_id: &str) -> Option<ProjectId> {
    menu_id
        .strip_prefix(RECENT_MENU_ID_PREFIX)
        .filter(|rest| !rest.is_empty())
        .map(|rest| ProjectId(rest.to_string()))
}

/// A menu label out of the bundled catalog for the language the user picked, falling back to the
/// key's English text and finally to `fallback` — a menu with a missing label would otherwise draw
/// an empty row the user cannot identify. See
/// [`locale_service::builtin_locale_for_language`] for why `"system"` reads as English here.
fn menu_label(app: &AppHandle, key: &str, fallback: &str) -> String {
    let language = app.state::<AppState>().settings.read().language.clone();
    let locale_id = locale_service::builtin_locale_for_language(&language);
    locale_service::lookup_builtin_message(locale_id, key)
        .or_else(|| locale_service::lookup_builtin_message(locale_service::BUILTIN_EN_ID, key))
        .unwrap_or_else(|| fallback.to_string())
}

fn recent_entries_now(app: &AppHandle) -> Vec<RecentMenuEntry> {
    let state = app.state::<AppState>();
    match project_service::list_recent_projects(&state.paths) {
        Ok(projects) => recent_menu_entries(projects, RECENT_PROJECT_MENU_LIMIT),
        Err(error) => {
            log::warn!("최근 프로젝트 목록을 읽지 못해 메뉴를 비웁니다: {error}");
            Vec::new()
        }
    }
}

/// Builds the rows that live inside `Open Recent`: the recent projects, a separator, then
/// `Clear Recent` (disabled when there is nothing to clear). Returned as owned `muda` items
/// because both the initial build and every rebuild append the same list.
fn build_open_recent_items(app: &AppHandle, entries: &[RecentMenuEntry]) -> tauri::Result<Vec<Box<dyn IsMenuItem<Wry>>>> {
    let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::with_capacity(entries.len() + 2);

    if entries.is_empty() {
        items.push(Box::new(
            MenuItemBuilder::with_id(
                MENU_ID_NO_RECENT_PROJECTS,
                menu_label(app, "menu.noRecentProjects", "No Recent Projects"),
            )
            .enabled(false)
            .build(app)?,
        ));
    }
    for entry in entries {
        items.push(Box::new(
            MenuItemBuilder::with_id(recent_menu_item_id(&entry.id), &entry.label)
                .enabled(entry.enabled)
                .build(app)?,
        ));
    }

    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    items.push(Box::new(
        MenuItemBuilder::with_id(MENU_ID_CLEAR_RECENT, menu_label(app, "menu.clearRecent", "Clear Recent"))
            .enabled(!entries.is_empty())
            .build(app)?,
    ));

    Ok(items)
}

/// The app menu's `File` submenu — currently `Open Recent ▸` alone. Built here rather than inline
/// in `commands::build_app_menu` because [`refresh_recent_menu`] has to rebuild the same rows on
/// every project change, and the two must not drift.
pub(crate) fn build_file_submenu(app: &AppHandle) -> tauri::Result<Submenu<Wry>> {
    let entries = recent_entries_now(app);
    let items = build_open_recent_items(app, &entries)?;

    let mut open_recent = SubmenuBuilder::with_id(app, MENU_ID_OPEN_RECENT, menu_label(app, "menu.openRecent", "Open Recent"));
    for item in &items {
        open_recent = open_recent.item(item.as_ref());
    }

    SubmenuBuilder::with_id(app, MENU_ID_FILE, menu_label(app, "menu.file", "File"))
        .item(&open_recent.build()?)
        .build()
}

/// Rebuilds `File > Open Recent` in place against the current on-disk project history — called by
/// `lib.rs`'s listener for every event that can change that history (`project:list-changed`,
/// `project:activated`), so the native menu never shows a list the app has already moved past.
///
/// The on-disk read happens here, on the caller's own (blocking) thread; every menu mutation is
/// then handed to the main thread as **one** closure. Each `Submenu` call would marshal itself
/// there individually (`tauri::menu::run_item_main_thread!`), which is safe but not atomic: two
/// refreshes racing — a rapid open/close pair, or the open's own pair of events — could interleave
/// their removals and appends and leave the menu holding a spliced mix of both lists. One closure
/// makes each rebuild all-or-nothing relative to the other.
///
/// Failures are logged, never propagated: a stale menu is a cosmetic problem, and the command that
/// triggered the refresh has already succeeded by the time this runs.
pub fn refresh_recent_menu(app: &AppHandle) {
    let entries = recent_entries_now(app);
    let handle = app.clone();

    if let Err(error) = app.run_on_main_thread(move || replace_open_recent_items(&handle, &entries)) {
        log::warn!("최근 항목 메뉴 갱신을 메인 스레드로 보내지 못했습니다: {error}");
    }
}

fn menu_submenu(parent: &Submenu<Wry>, id: &str) -> Option<Submenu<Wry>> {
    parent.get(id).and_then(|item| item.as_submenu().cloned())
}

/// The main-thread half of [`refresh_recent_menu`]: find `Open Recent`, drop every row it holds,
/// append the freshly read ones. Logs rather than returns its failures because
/// `run_on_main_thread` takes a closure that cannot report one back.
fn replace_open_recent_items(app: &AppHandle, entries: &[RecentMenuEntry]) {
    let Some(open_recent) = app
        .menu()
        .and_then(|menu| menu.get(MENU_ID_FILE))
        .and_then(|file| file.as_submenu().cloned())
        .and_then(|file| menu_submenu(&file, MENU_ID_OPEN_RECENT))
    else {
        log::warn!("앱 메뉴에서 Open Recent 서브메뉴를 찾지 못했습니다");
        return;
    };

    if let Err(error) = rebuild_open_recent_items(app, &open_recent, entries) {
        log::warn!("최근 항목 메뉴 갱신에 실패했습니다: {error}");
    }
}

fn rebuild_open_recent_items(app: &AppHandle, open_recent: &Submenu<Wry>, entries: &[RecentMenuEntry]) -> tauri::Result<()> {
    while open_recent.remove_at(0)?.is_some() {}

    for item in build_open_recent_items(app, entries)? {
        open_recent.append(item.as_ref())?;
    }
    Ok(())
}

/// The root path of the persisted project record `project_id` names, read back at click time
/// rather than baked into the menu item: a menu built minutes ago can name a project whose record
/// has since been cleared, and re-reading is what turns that into a logged no-op instead of an
/// open call against a stale path.
pub(crate) fn recent_project_root(app: &AppHandle, project_id: &ProjectId) -> Option<String> {
    let state = app.state::<AppState>();
    project_service::list_recent_projects(&state.paths)
        .ok()?
        .into_iter()
        .find(|project| &project.id == project_id)
        .map(|project| project.root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::project::types::ProjectDisplay;

    fn 프로젝트(name: &str, last_opened_at: f64) -> Project {
        Project {
            id: ProjectId::new(),
            root: format!("/tmp/{name}"),
            name: name.to_string(),
            capabilities: Vec::new(),
            root_missing: false,
            last_opened_at,
            display: ProjectDisplay::default(),
        }
    }

    #[test]
    fn 최근_항목은_상한까지만_원래_순서로_남는다() {
        let projects: Vec<Project> = (0..(RECENT_PROJECT_MENU_LIMIT + 5))
            .map(|index| 프로젝트(&format!("p{index}"), index as f64))
            .collect();
        let expected: Vec<String> = projects
            .iter()
            .take(RECENT_PROJECT_MENU_LIMIT)
            .map(|project| project.name.clone())
            .collect();

        let entries = recent_menu_entries(projects, RECENT_PROJECT_MENU_LIMIT);

        assert_eq!(entries.len(), RECENT_PROJECT_MENU_LIMIT);
        assert_eq!(entries.iter().map(|entry| entry.label.clone()).collect::<Vec<_>>(), expected);
    }

    #[test]
    fn 표시_라벨이_있으면_라벨을_없으면_폴더명을_쓴다() {
        let mut labeled = 프로젝트("long-folder-name", 1.0);
        labeled.display.label = Some("TA".to_string());
        let mut blank = 프로젝트("plain", 2.0);
        blank.display.label = Some("   ".to_string());

        let entries = recent_menu_entries(vec![labeled, blank], RECENT_PROJECT_MENU_LIMIT);

        assert_eq!(entries[0].label, "TA");
        assert_eq!(entries[1].label, "plain");
    }

    #[test]
    fn 폴더가_사라진_프로젝트는_목록에_남되_비활성이다() {
        let mut missing = 프로젝트("gone", 1.0);
        missing.root_missing = true;

        let entries = recent_menu_entries(vec![missing, 프로젝트("here", 2.0)], RECENT_PROJECT_MENU_LIMIT);

        assert_eq!(entries.len(), 2);
        assert!(!entries[0].enabled);
        assert!(entries[1].enabled);
    }

    #[test]
    fn 최근_항목_메뉴_id는_왕복한다() {
        let project_id = ProjectId::new();

        let menu_id = recent_menu_item_id(&project_id);

        assert_eq!(parse_recent_menu_id(&menu_id), Some(project_id));
    }

    #[test]
    fn 최근_항목이_아닌_메뉴_id는_파싱되지_않는다() {
        assert_eq!(parse_recent_menu_id(MENU_ID_CLEAR_RECENT), None);
        assert_eq!(parse_recent_menu_id(MENU_ID_OPEN_RECENT), None);
        assert_eq!(parse_recent_menu_id(RECENT_MENU_ID_PREFIX), None);
    }

    #[test]
    fn 메뉴_id는_각자의_행동으로_해석된다() {
        let project_id = ProjectId::new();

        assert_eq!(menu_action(MENU_ID_QUIT), MenuAction::Quit);
        assert_eq!(menu_action(MENU_ID_CLEAR_RECENT), MenuAction::ClearRecent);
        assert_eq!(menu_action(&recent_menu_item_id(&project_id)), MenuAction::OpenRecent(project_id));
    }

    /// Everything the assembly must not act on: the submenu titles, the disabled placeholder row,
    /// and the predefined items tauri handles itself (which reach the handler as their own ids).
    #[test]
    fn 처리_대상이_아닌_메뉴_id는_무시된다() {
        assert_eq!(menu_action(MENU_ID_FILE), MenuAction::Ignored);
        assert_eq!(menu_action(MENU_ID_OPEN_RECENT), MenuAction::Ignored);
        assert_eq!(menu_action(MENU_ID_NO_RECENT_PROJECTS), MenuAction::Ignored);
        assert_eq!(menu_action(""), MenuAction::Ignored);
    }
}
