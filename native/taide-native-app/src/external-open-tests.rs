use super::*;

fn projects(roots: &[&str]) -> Vec<(ProjectId, String)> {
    roots
        .iter()
        .map(|root| (ProjectId::new(), (*root).to_owned()))
        .collect()
}

fn arguments(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn 외부_열기_대상은_가장_긴_root를_고르고_같으면_먼저_나온_것이다() {
    let listed = projects(&["/work", "/work/app", "/work/app", "/work/app-other"]);
    assert_eq!(
        resolve_target("/work/app/src/main.rs", &listed, None),
        Some(listed[1].0.clone())
    );
    assert_eq!(
        resolve_target("/work/app-other/a.rs", &listed, None),
        Some(listed[3].0.clone())
    );
    assert_eq!(
        resolve_target("/work/app/../app-other/a.rs", &listed, None),
        Some(listed[3].0.clone())
    );
    assert_eq!(
        resolve_target("/work/./app/x", &listed, None),
        Some(listed[1].0.clone())
    );
    assert_eq!(
        resolve_target("/work/app", &listed, None),
        Some(listed[1].0.clone())
    );
}

#[test]
fn 외부_열기_대상은_root_밖이면_활성_프로젝트_없으면_첫_프로젝트다() {
    let listed = projects(&["/alpha", "/beta"]);
    assert_eq!(
        resolve_target("/tmp/claude-prompt.md", &listed, Some(&listed[1].0)),
        Some(listed[1].0.clone())
    );
    assert_eq!(
        resolve_target("/tmp/claude-prompt.md", &listed, Some(&ProjectId::new())),
        Some(listed[0].0.clone())
    );
    assert_eq!(
        resolve_target("/tmp/claude-prompt.md", &listed, None),
        Some(listed[0].0.clone())
    );
    assert_eq!(resolve_target("/tmp/claude-prompt.md", &[], None), None);
    assert!(!is_within_root("/alpha-other/file", "/alpha"));
    assert!(is_within_root("C:\\work\\app\\file", "C:/work/app"));
}

#[test]
fn wait_marker는_경로마다_여러개를_모으고_꺼내면_비운다() {
    let markers = WaitMarkers::default();
    markers.register("/a.md".into(), "first".into());
    markers.register("/a.md".into(), "second".into());
    markers.register("/b.md".into(), "other".into());
    assert_eq!(markers.take("/a.md"), vec!["first", "second"]);
    assert!(markers.take("/a.md").is_empty());
    assert_eq!(markers.take("/b.md"), vec!["other"]);
}

#[test]
fn 실행_인자는_data_dir_뒤의_경로와_wait_marker를_외부_열기_요청으로_읽는다() {
    let base = ["--data-dir", "/tmp/native-test-data"];
    let launch = crate::bootstrap::Launch::parse(arguments(&base)).unwrap();
    assert!(launch.external_open.is_none() && launch.relay_arguments.is_empty());
    let launch =
        crate::bootstrap::Launch::parse(arguments(&[base[0], base[1], "/work/notes.md"])).unwrap();
    assert_eq!(
        launch.external_open,
        Some(ExternalOpenRequest {
            path: "/work/notes.md".into(),
            wait_marker: None,
        })
    );
    let launch = crate::bootstrap::Launch::parse(arguments(&[
        base[0],
        base[1],
        "/work/notes.md",
        "--wait-marker",
        "/tmp/taide-wait-1",
    ]))
    .unwrap();
    assert_eq!(
        launch.external_open,
        Some(ExternalOpenRequest {
            path: "/work/notes.md".into(),
            wait_marker: Some("/tmp/taide-wait-1".into()),
        })
    );
    assert_eq!(launch.relay_arguments.len(), 3);
    assert_eq!(
        launch.config.data_dir,
        std::path::PathBuf::from("/tmp/native-test-data")
    );
    for invalid in [
        vec!["/work/notes.md", "--data-dir", "/tmp/data"],
        vec!["--data-dir", "relative", "/work/notes.md"],
    ] {
        assert!(crate::bootstrap::Launch::parse(arguments(&invalid)).is_err());
    }
}
