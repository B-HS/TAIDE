use std::collections::{HashMap, HashSet};

use taide_model::ids::ProjectId;
use taide_model::layout::ProjectLayout;
use taide_model::project::Project;

/// Every auxiliary editor window's OS-level Tauri label starts with this prefix, followed by a
/// Rust-assigned integer (`editor-1`, `editor-2`, ...). The prefix (not the whole label) is what
/// `capabilities/main.json`'s `editor-*` glob and `tauri-plugin-window-state`'s label-normalizing
/// hook match against — see `service::is_auxiliary_label`.
pub const AUXILIARY_WINDOW_LABEL_PREFIX: &str = "editor-";

/// Single `tauri-plugin-window-state` cache key every auxiliary window is normalized onto
/// (`service::normalize_window_state_label`), so short-lived, ever-incrementing `editor-<n>`
/// labels don't accumulate unboundedly in `.window-state.json`. Deliberately has no trailing
/// digit, so it can never collide with a real `editor-<n>` label.
pub const AUXILIARY_WINDOW_STATE_KEY: &str = "editor";

pub const AUXILIARY_WINDOW_DEFAULT_WIDTH: f64 = 1_000.0;

pub fn is_auxiliary_label(label: &str) -> bool {
    label.starts_with(AUXILIARY_WINDOW_LABEL_PREFIX)
}

/// Picks the lowest unused `editor-<n>` label given the labels of every window currently open.
/// Rust — not the caller — owns label issuance (contract §3.1): the frontend only supplies which
/// project/layout slot the new window represents, never the OS-level label itself, so two
/// concurrent open requests can never coin the same window label.
pub fn next_auxiliary_label<'a>(existing_labels: impl Iterator<Item = &'a str>) -> String {
    let used: HashSet<u32> = existing_labels
        .filter_map(|label| label.strip_prefix(AUXILIARY_WINDOW_LABEL_PREFIX))
        .filter_map(|suffix| suffix.parse::<u32>().ok())
        .collect();

    let mut candidate = 1u32;
    while used.contains(&candidate) {
        candidate += 1;
    }
    format!("{AUXILIARY_WINDOW_LABEL_PREFIX}{candidate}")
}

/// Collapses every auxiliary editor window label onto one shared `tauri-plugin-window-state`
/// cache key, passed to the plugin's `Builder::map_label` hook (`lib.rs`). Without this, each
/// ever-incrementing `editor-<n>` label would grow `.window-state.json` by one entry every time a
/// window opens and never shrink, since the plugin never learns a label is retired for good. The
/// main window's own label passes through untouched.
pub fn normalize_window_state_label(label: &str) -> &str {
    if is_auxiliary_label(label) {
        AUXILIARY_WINDOW_STATE_KEY
    } else {
        label
    }
}

/// The projects whose persisted auxiliary windows may be recreated at boot: every open project
/// except one whose root went missing while the app was closed. A `root_missing` project renders
/// nothing (its files are gone), so recreating its editor windows would put empty OS windows on
/// screen that the user never asked for and cannot fill.
///
/// Iteration order is the projects map's own — the caller spawns one independent task per window,
/// so restoration order carries no contract.
pub fn restorable_project_ids(projects: &HashMap<ProjectId, Project>) -> Vec<ProjectId> {
    projects.iter().filter(|(_, project)| !project.root_missing).map(|(project_id, _)| project_id.clone()).collect()
}

/// Flattens `(project, persisted auxiliary window)` into the `(project id, window slot)` pairs
/// `commands::restore_auxiliary_windows` recreates. A project id with no persisted layout at all
/// contributes nothing; one whose layout records no auxiliary window contributes nothing either.
///
/// Split out of the command so the plan is testable without an `AppHandle`, and kept as two
/// functions rather than one taking both maps so the caller keeps reading `projects` and `layouts`
/// through separate, non-overlapping read guards.
pub fn plan_auxiliary_window_restorations(project_ids: &[ProjectId], layouts: &HashMap<ProjectId, ProjectLayout>) -> Vec<(ProjectId, u32)> {
    project_ids
        .iter()
        .filter_map(|project_id| layouts.get(project_id).map(|layout| (project_id, layout)))
        .flat_map(|(project_id, layout)| layout.auxiliary_windows.iter().map(move |window| (project_id.clone(), window.slot)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_접두사가_붙은_라벨만_보조_창으로_인식한다() {
        assert!(is_auxiliary_label("editor-1"));
        assert!(is_auxiliary_label("editor-42"));
        assert!(!is_auxiliary_label("main"));
        assert!(!is_auxiliary_label("editor"));
    }

    #[test]
    fn 기존_라벨이_없으면_editor_1_을_할당한다() {
        assert_eq!(next_auxiliary_label(std::iter::empty()), "editor-1");
    }

    #[test]
    fn 메인_창_라벨은_번호_할당에_영향을_주지_않는다() {
        assert_eq!(next_auxiliary_label(["main"].into_iter()), "editor-1");
    }

    #[test]
    fn 사용중인_가장_작은_번호_다음을_할당한다() {
        assert_eq!(next_auxiliary_label(["main", "editor-1", "editor-2"].into_iter()), "editor-3");
    }

    #[test]
    fn 중간_번호가_닫혀_비면_그_번호를_재사용한다() {
        assert_eq!(next_auxiliary_label(["main", "editor-1", "editor-3"].into_iter()), "editor-2");
    }

    #[test]
    fn 보조_창_라벨은_공유_상태_키로_정규화된다() {
        assert_eq!(normalize_window_state_label("editor-1"), "editor");
        assert_eq!(normalize_window_state_label("editor-7"), "editor");
    }

    #[test]
    fn 메인_창_라벨은_정규화되지_않고_그대로다() {
        assert_eq!(normalize_window_state_label("main"), "main");
    }

    fn project_id(name: &str) -> ProjectId {
        ProjectId::from(format!("prj-{name}"))
    }

    fn project(name: &str, root_missing: bool) -> (ProjectId, Project) {
        let id = project_id(name);
        (
            id.clone(),
            Project {
                id,
                root: format!("/repo/{name}"),
                name: name.to_string(),
                capabilities: Vec::new(),
                root_missing,
                last_opened_at: 0.0,
                display: taide_model::project::ProjectDisplay::default(),
            },
        )
    }

    fn layout_with_slots(slots: &[u32]) -> ProjectLayout {
        let mut layout = taide_layout::service::default_layout();
        layout.auxiliary_windows = slots
            .iter()
            .map(|slot| taide_model::layout::AuxWindowLayout { slot: *slot, root: layout.root.clone(), focused_pane: layout.focused_pane.clone() })
            .collect();
        layout
    }

    fn sorted(mut pairs: Vec<(ProjectId, u32)>) -> Vec<(String, u32)> {
        pairs.sort();
        pairs.into_iter().map(|(id, slot)| (id.as_str().to_string(), slot)).collect()
    }

    #[test]
    fn 루트가_사라진_프로젝트는_복원_대상에서_빠진다() {
        let projects = HashMap::from([project("alive", false), project("gone", true)]);

        let mut ids: Vec<String> = restorable_project_ids(&projects).into_iter().map(|id| id.as_str().to_string()).collect();
        ids.sort();

        assert_eq!(ids, vec!["prj-alive"]);
    }

    #[test]
    fn 보조_창_복원_계획은_프로젝트별_슬롯을_모두_펼친다() {
        let project_ids = vec![project_id("a"), project_id("b")];
        let layouts = HashMap::from([(project_id("a"), layout_with_slots(&[1, 2])), (project_id("b"), layout_with_slots(&[5]))]);

        assert_eq!(
            sorted(plan_auxiliary_window_restorations(&project_ids, &layouts)),
            vec![("prj-a".to_string(), 1), ("prj-a".to_string(), 2), ("prj-b".to_string(), 5)]
        );
    }

    #[test]
    fn 레이아웃이_없거나_보조_창이_없으면_복원할_것이_없다() {
        let project_ids = vec![project_id("no-layout"), project_id("no-windows")];
        let layouts = HashMap::from([(project_id("no-windows"), layout_with_slots(&[]))]);

        assert!(plan_auxiliary_window_restorations(&project_ids, &layouts).is_empty());
    }

    /// A layout can outlive the project id it belongs to only in the persisted file; the plan is
    /// driven by the *project* list, so a leftover layout entry never resurrects a window.
    #[test]
    fn 목록에_없는_프로젝트의_레이아웃은_복원되지_않는다() {
        let layouts = HashMap::from([(project_id("closed"), layout_with_slots(&[1]))]);

        assert!(plan_auxiliary_window_restorations(&[], &layouts).is_empty());
    }

    #[test]
    fn 한_프로젝트_안에서는_기록된_슬롯_순서를_유지한다() {
        let project_ids = vec![project_id("a")];
        let layouts = HashMap::from([(project_id("a"), layout_with_slots(&[3, 1, 2]))]);

        let slots: Vec<u32> = plan_auxiliary_window_restorations(&project_ids, &layouts).into_iter().map(|(_, slot)| slot).collect();

        assert_eq!(slots, vec![3, 1, 2]);
    }
}
