use super::*;

const OWNER: &str = "native-main";

fn query(text: &str) -> SearchQuery {
    serde_json::from_value(serde_json::json!({ "text": text })).unwrap()
}

fn session(tab: &TabId) -> Session {
    Session {
        owner: OWNER.into(),
        id: tab.to_string(),
    }
}

fn mount(editors: &mut Editors, tab: &TabId, project: &ProjectId) -> (u64, bool) {
    let mounted = editors.mount(tab, project, || session(tab), &query("needle"));
    (mounted.nonce, mounted.is_created)
}

fn shown(tabs: &[&TabId]) -> HashSet<TabId> {
    tabs.iter().map(|tab| (*tab).clone()).collect()
}

#[test]
fn 저장한_적_없는_탭은_탭의_질의로_새로_만들고_첫_마운트로_센다() {
    let mut editors = Editors::default();
    let tab = TabId::new();
    assert!(editors.get_mut(&tab).is_none());
    assert_eq!(mount(&mut editors, &tab, &ProjectId::new()), (1, true));
}

#[test]
fn 탭을_떠났다가_돌아오면_입력과_결과를_그대로_돌려주고_다시_포커스한다() {
    let mut editors = Editors::default();
    let (tab, project) = (TabId::new(), ProjectId::new());
    mount(&mut editors, &tab, &project);
    assert_eq!(mount(&mut editors, &tab, &project), (1, false));
    let streams = crate::search_stream::Streams::default();
    let run = editors
        .get_mut(&tab)
        .unwrap()
        .begin_unsubmitted_run(&streams);
    assert_eq!(editors.unmount_hidden(&shown(&[])), vec![run.session]);
    assert_eq!(
        editors.get_mut(&tab).unwrap().status(),
        crate::search_panel::Status::Completed
    );
    assert!(editors.unmount_hidden(&shown(&[])).is_empty());
    assert_eq!(mount(&mut editors, &tab, &project), (2, false));
    assert_eq!(editors.get_mut(&tab).unwrap().generation(), run.generation);
}

#[test]
fn 다른_프로젝트의_항목은_돌려주지_않고_버린다() {
    let mut editors = Editors::default();
    let tab = TabId::new();
    mount(&mut editors, &tab, &ProjectId::new());
    editors.unmount_hidden(&shown(&[]));
    assert_eq!(mount(&mut editors, &tab, &ProjectId::new()), (1, true));
}

#[test]
fn 상한을_넘으면_가장_오래_떠난_탭부터_버린다() {
    let mut editors = Editors::default();
    let project = ProjectId::new();
    let tabs = (0..=MEMORY_LIMIT).map(|_| TabId::new()).collect::<Vec<_>>();
    for tab in &tabs {
        mount(&mut editors, tab, &project);
        editors.unmount_hidden(&shown(&[]));
    }
    assert!(editors.get_mut(&tabs[0]).is_none());
    assert!(tabs[1..].iter().all(|tab| editors.get_mut(tab).is_some()));
}

#[test]
fn 다시_떠난_탭은_최신으로_취급되어_상한에서_먼저_밀려나지_않는다() {
    let mut editors = Editors::default();
    let project = ProjectId::new();
    let tabs = (0..MEMORY_LIMIT).map(|_| TabId::new()).collect::<Vec<_>>();
    for tab in &tabs {
        mount(&mut editors, tab, &project);
        editors.unmount_hidden(&shown(&[]));
    }
    mount(&mut editors, &tabs[0], &project);
    editors.unmount_hidden(&shown(&[]));
    let newest = TabId::new();
    mount(&mut editors, &newest, &project);
    editors.unmount_hidden(&shown(&[]));
    assert!(editors.get_mut(&tabs[0]).is_some());
    assert!(editors.get_mut(&tabs[1]).is_none());
    assert!(editors.get_mut(&newest).is_some());
}

#[test]
fn 보이는_탭은_떠나지_않고_실행을_유지한다() {
    let mut editors = Editors::default();
    let (tab, project) = (TabId::new(), ProjectId::new());
    mount(&mut editors, &tab, &project);
    let streams = crate::search_stream::Streams::default();
    editors
        .get_mut(&tab)
        .unwrap()
        .begin_unsubmitted_run(&streams);
    assert!(editors.unmount_hidden(&shown(&[&tab])).is_empty());
    assert_eq!(
        editors.get_mut(&tab).unwrap().status(),
        crate::search_panel::Status::Running
    );
    assert_eq!(mount(&mut editors, &tab, &project), (1, false));
}
