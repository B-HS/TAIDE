use taide_model::ids::ProjectId;
use taide_terminal::metadata::TerminalSessionMetadata;

#[test]
fn 세션_메타데이터는_동일한_cwd_보고를_무시하고_변경만_반영한다() {
    let project_id = ProjectId::from("prj-test".to_string());
    let metadata = TerminalSessionMetadata::new(project_id.clone(), "/project".to_string(), "zsh".to_string());

    assert_eq!(metadata.project_id(), &project_id);
    assert_eq!(metadata.cwd(), "/project");
    assert_eq!(metadata.shell(), "zsh");
    assert!(!metadata.update_cwd("/project".to_string()));
    assert!(metadata.update_cwd("/project/src".to_string()));
    assert!(!metadata.update_cwd("/project/src".to_string()));
    assert_eq!(metadata.cwd(), "/project/src");
}

#[test]
fn 세션_snapshot은_최신_cwd와_종료_상태를_반영한다() {
    let project_id = ProjectId::from("prj-test".to_string());
    let metadata = TerminalSessionMetadata::new(project_id.clone(), "/project".to_string(), "default".to_string());
    let initial = metadata.snapshot("term-test");

    assert_eq!(initial.id, "term-test");
    assert_eq!(initial.project_id, project_id);
    assert_eq!(initial.cwd, "/project");
    assert_eq!(initial.shell, "default");
    assert!(initial.running);

    metadata.update_cwd("/project/src".to_string());
    metadata.mark_exited();

    let exited = metadata.snapshot("term-test");
    assert_eq!(exited.cwd, "/project/src");
    assert!(!exited.running);
}
