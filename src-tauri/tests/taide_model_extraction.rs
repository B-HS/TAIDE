use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use serde_json::Value;
use taide_lib::error::{AppError, AppErrorKind, AppResult, LocalizedError};
use taide_lib::ids::{PaneId, ProjectGroupId, ProjectId, ShellSlotId, TabId};

const MANIFEST_SOURCE: &str = include_str!("fixtures/rust-native/ipc-contract-manifest.json");
const MODEL_ERROR_SOURCE: &str = include_str!("../../crates/taide-model/src/error.rs");
const MODEL_IDS_SOURCE: &str = include_str!("../../crates/taide-model/src/ids.rs");
const FACADE_ERROR_SOURCE: &str = include_str!("../src/error.rs");
const FACADE_IDS_SOURCE: &str = include_str!("../src/ids.rs");

const MODEL_ALLOWED_DEPENDENCIES: &[&str] = &["serde", "serde_json", "specta", "thiserror", "uuid"];

fn cargo_metadata() -> &'static Value {
    static METADATA: OnceLock<Value> = OnceLock::new();
    METADATA.get_or_init(|| {
        let output = Command::new(env!("CARGO"))
            .args(["metadata", "--no-deps", "--offline", "--format-version", "1"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("cargo metadata 를 실행하지 못했습니다");
        assert!(
            output.status.success(),
            "cargo metadata 가 실패했습니다 (exit {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("cargo metadata 출력이 유효한 JSON 이 아닙니다")
    })
}

fn metadata_package<'a>(metadata: &'a Value, package_name: &str) -> &'a Value {
    metadata["packages"]
        .as_array()
        .expect("cargo metadata 의 packages 가 배열이 아닙니다")
        .iter()
        .find(|package| package["name"].as_str() == Some(package_name))
        .unwrap_or_else(|| panic!("cargo metadata 에 {package_name} 패키지가 없습니다"))
}

fn metadata_dependency_names(metadata: &Value, package_name: &str) -> Vec<String> {
    let mut names: Vec<String> = metadata_package(metadata, package_name)["dependencies"]
        .as_array()
        .expect("cargo metadata 의 dependencies 가 배열이 아닙니다")
        .iter()
        .map(|dependency| dependency["name"].as_str().expect("의존성 name 이 문자열이 아닙니다").to_string())
        .collect();
    names.sort();
    names.dedup();
    names
}

fn metadata_dependency_requirements(metadata: &Value, package_name: &str, dependency_name: &str) -> Vec<String> {
    metadata_package(metadata, package_name)["dependencies"]
        .as_array()
        .expect("cargo metadata 의 dependencies 가 배열이 아닙니다")
        .iter()
        .filter(|dependency| dependency["name"].as_str() == Some(dependency_name))
        .map(|dependency| dependency["req"].as_str().expect("의존성 req 가 문자열이 아닙니다").to_string())
        .collect()
}

fn as_model_project_id(id: ProjectId) -> taide_model::ids::ProjectId {
    id
}

fn as_facade_project_id(id: taide_model::ids::ProjectId) -> ProjectId {
    id
}

fn as_model_error(error: AppError) -> taide_model::error::AppError {
    error
}

fn as_facade_error(error: taide_model::error::AppError) -> AppError {
    error
}

fn fallible(flag: bool) -> AppResult<u8> {
    if flag {
        Ok(7)
    } else {
        Err(AppError::Internal("boom".into()))
    }
}

#[test]
fn 아이디_타입은_모델과_파사드에서_동일하다() {
    let model_id = as_model_project_id(ProjectId::new());
    assert!(model_id.as_str().starts_with("prj-"));
    assert_eq!(as_facade_project_id(model_id.clone()), model_id);

    let _: taide_model::ids::PaneId = PaneId::new();
    let _: taide_model::ids::TabId = TabId::new();
    let _: taide_model::ids::ShellSlotId = ShellSlotId::new();
    let _: taide_model::ids::ProjectGroupId = ProjectGroupId::new();
}

#[test]
fn 에러_타입은_모델과_파사드에서_동일하다() {
    let error = AppError::localized(AppErrorKind::NotFound, "error.file.notFound", "file not found").with_arg("path", "/tmp/a");
    let model_error: taide_model::error::AppError = as_model_error(error);
    assert_eq!(model_error.kind(), AppErrorKind::NotFound);
    assert_eq!(as_facade_error(model_error).kind(), AppErrorKind::NotFound);

    let kind: taide_model::error::AppErrorKind = AppErrorKind::InvalidArgument;
    assert_eq!(kind, AppErrorKind::InvalidArgument);

    let localized = LocalizedError {
        kind: AppErrorKind::Forbidden,
        key: "error.forbidden".to_string(),
        args: BTreeMap::new(),
        fallback: "forbidden".to_string(),
    };
    let model_localized: taide_model::error::LocalizedError = localized;
    let facade_localized: LocalizedError = model_localized;
    assert_eq!(facade_localized.kind, AppErrorKind::Forbidden);

    let result: taide_model::error::AppResult<u8> = fallible(true);
    assert_eq!(result.expect("값이 있어야 합니다"), 7);

    let facade_result: AppResult<u8> = fallible(false);
    assert_eq!(facade_result.expect_err("에러여야 합니다").kind(), AppErrorKind::Internal);
}

#[test]
fn 아이디와_에러_wire_형태가_유지된다() {
    let id = TabId("tab-fixed".to_string());
    assert_eq!(serde_json::to_string(&id).expect("직렬화"), "\"tab-fixed\"");

    let forbidden = serde_json::to_value(AppError::Forbidden("outside root".into())).expect("직렬화");
    assert_eq!(forbidden["code"], "Forbidden");
    assert_eq!(forbidden["message"], "outside root");

    let localized =
        AppError::localized(AppErrorKind::InvalidArgument, "error.archive.openFailed", "could not open").with_arg("detail", "eof");
    let value = serde_json::to_value(&localized).expect("직렬화");
    assert_eq!(value["code"], "Localized");
    assert_eq!(value["message"]["kind"], "InvalidArgument");
    assert_eq!(value["message"]["key"], "error.archive.openFailed");
    assert_eq!(value["message"]["args"]["detail"], "eof");
    assert_eq!(value["message"]["fallback"], "could not open");
}

#[test]
fn 모델_crate는_tauri_의존을_가지지_않는다() {
    let metadata = cargo_metadata();
    let names = metadata_dependency_names(metadata, "taide-model");

    assert!(
        names.iter().any(|name| name == "specta"),
        "cargo metadata 에서 taide-model 의 specta 의존을 찾지 못했습니다: {names:#?}"
    );

    let unexpected: Vec<&String> = names
        .iter()
        .filter(|name| !MODEL_ALLOWED_DEPENDENCIES.contains(&name.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "taide-model 의존성 허용 목록 밖 항목이 있습니다 — dev/build/target/cfg 를 포함한 모든 kind 를 검사합니다:\n{unexpected:#?}"
    );

    let tauri_dependencies: Vec<&String> = names.iter().filter(|name| name.contains("tauri")).collect();
    assert!(
        tauri_dependencies.is_empty(),
        "taide-model 은 Tauri/tauri-specta 에 의존할 수 없습니다 (모든 kind 포함):\n{tauri_dependencies:#?}"
    );

    assert_eq!(
        metadata_dependency_requirements(metadata, "taide-model", "specta"),
        vec!["=2.0.0-rc.25".to_string()],
        "specta 버전 핀이 기존 `=2.0.0-rc.25` 와 다릅니다 — 파사드와 같은 Types 로 수출되어야 합니다"
    );
}

#[test]
fn metadata_수집은_일반과_build_의존_kind를_모두_담는다() {
    let names = metadata_dependency_names(cargo_metadata(), "taide");

    for expected in ["tauri", "tauri-build"] {
        assert!(
            names.contains(&expected.to_string()),
            "cargo metadata 수집이 {expected} 를 놓쳤습니다 — kind 수집이 비면 의존 gate 가 거짓 green 이 됩니다: {names:#?}"
        );
    }
}

#[test]
fn 모델_구현_파일과_ipc_manifest_원천이_실제_구현을_가리킨다() {
    assert!(
        MODEL_IDS_SOURCE.contains("macro_rules! string_id") && MODEL_IDS_SOURCE.contains("string_id!(ProjectId, \"prj\");"),
        "taide-model/ids.rs 가 실제 구현이 아니라 스텁입니다 — 스캔이 거짓 green 이 될 수 있습니다"
    );
    assert!(
        MODEL_ERROR_SOURCE.contains("#[serde(tag = \"code\", content = \"message\")]\npub enum AppError {")
            && MODEL_ERROR_SOURCE.contains("impl AppError {")
            && MODEL_ERROR_SOURCE.contains("pub type AppResult<T> = Result<T, AppError>;"),
        "taide-model/error.rs 가 실제 AppError 구현이 아니라 스텁입니다 — wire code 스캔이 거짓 green 이 될 수 있습니다"
    );

    assert!(
        FACADE_IDS_SOURCE.contains("pub use taide_model::ids") && FACADE_ERROR_SOURCE.contains("pub use taide_model::error"),
        "src-tauri 의 기존 모듈이 새 crate 재수출 파사드로 유지되지 않았습니다"
    );
    assert!(
        !FACADE_IDS_SOURCE.contains("macro_rules! string_id") && !FACADE_ERROR_SOURCE.contains("pub enum AppError {"),
        "파사드에 구현이 남아 있습니다 — 원천이 두 곳으로 갈라집니다"
    );

    let manifest: Value = serde_json::from_str(MANIFEST_SOURCE).expect("rust-native IPC 계약 manifest 를 읽지 못했습니다");
    let error_source_path = manifest["sources"]["error"]
        .as_str()
        .expect("manifest sources.error 는 문자열이어야 합니다");
    assert_eq!(error_source_path, "crates/taide-model/src/error.rs");

    let on_disk = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(error_source_path))
        .expect("manifest 가 가리키는 error 원천 파일을 읽지 못했습니다");
    assert_eq!(on_disk, MODEL_ERROR_SOURCE, "manifest 원천 경로와 include_str 원천이 다릅니다");
    assert!(
        on_disk.contains("#[serde(tag = \"code\", content = \"message\")]\npub enum AppError {") && on_disk.contains("impl AppError {"),
        "manifest 원천이 실제 AppError 구현 파일이 아닙니다 — Phase 0 스캔이 거짓 green 이 될 수 있습니다"
    );
}
