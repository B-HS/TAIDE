use std::path::Path;

use taide_lib::domain::task::service as legacy_service;
use taide_model::task::{Task, TaskSource};

#[test]
fn 태스크_탐지는_독립_crate와_기존_경로에서_같은_명령을_만든다() {
    let extracted: fn(&Path) -> Vec<Task> = taide_task::service::detect_tasks;
    let legacy: fn(&Path) -> Vec<Task> = legacy_service::detect_tasks;
    assert!(std::ptr::fn_addr_eq(extracted, legacy));

    let root = std::env::temp_dir().join(format!("taide-task-boundary-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("fixture 디렉터리 생성");
    std::fs::write(root.join("package.json"), r#"{"scripts":{"build and test":"vitest run"}}"#).expect("package fixture 기록");
    std::fs::write(root.join("bun.lock"), "").expect("bun lock fixture 기록");

    let tasks = extracted(&root);
    assert_eq!(tasks, legacy(&root));
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].source, TaskSource::Npm);
    assert_eq!(tasks[0].command, "bun run 'build and test'");

    std::fs::remove_dir_all(root).expect("fixture 정리");
}
