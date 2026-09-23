use taide_lib::domain::system::types::{AppDataPathKind, SystemUsage, SystemUsageProcess};
use taide_lib::domain::task::types::{Task, TaskSource};
use taide_model::system::{AppDataPathKind as ModelAppDataPathKind, SystemUsageProcess as ModelSystemUsageProcess};
use taide_model::task::Task as ModelTask;

#[test]
fn 작업_목록의_기존_wire와_공개_타입을_유지한다() {
    let wire = serde_json::json!({
        "label": "build", "command": "cargo build", "source": "cargo", "cwd": "/project"
    });
    let model: ModelTask = serde_json::from_value(wire.clone()).expect("기존 작업 항목");
    let facade: Task = model.clone();
    assert_eq!(facade.source, TaskSource::Cargo);
    assert_eq!(serde_json::to_value(model).expect("작업 직렬화"), wire);
    assert_eq!(serde_json::to_value(TaskSource::Npm).expect("npm 이름"), "npm");
}

#[test]
fn 시스템_사용량의_null과_프로세스_분류_wire를_유지한다() {
    let usage_wire = serde_json::json!({ "cpuPercent": null, "memoryBytes": 0.0 });
    let usage: taide_model::system::SystemUsage = serde_json::from_value(usage_wire.clone()).expect("기존 시스템 사용량");
    let facade_usage: SystemUsage = usage.clone();
    assert!(facade_usage.cpu_percent.is_none());
    assert_eq!(serde_json::to_value(usage).expect("시스템 사용량 직렬화"), usage_wire);

    let process_wire = serde_json::json!({
        "pid": 42, "kind": "lsp", "label": "rust-analyzer", "cpuPercent": null, "memoryBytes": 1.0
    });
    let process: ModelSystemUsageProcess = serde_json::from_value(process_wire.clone()).expect("기존 프로세스 사용량");
    let facade: SystemUsageProcess = process.clone();
    assert!(facade.cpu_percent.is_none());
    assert_eq!(serde_json::to_value(process).expect("프로세스 직렬화"), process_wire);
}

#[test]
fn 앱_데이터_경로_분류의_기존_wire와_타입을_유지한다() {
    let model: ModelAppDataPathKind = serde_json::from_str("\"snippets\"").expect("경로 분류");
    let facade: AppDataPathKind = model;
    assert_eq!(facade, AppDataPathKind::Snippets);
    assert_eq!(serde_json::to_string(&facade).expect("경로 분류 직렬화"), "\"snippets\"");
}
