use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};

const SUPPORTED_SCHEMA_VERSION: u32 = 1;
const MANIFEST_ORDERING: &str = "source-declaration-order";
const BINDINGS_HASH_ALGORITHM: &str = "sha256";
const BINDINGS_PATH: &str = "src/shared/api/bindings.ts";
const IPC_REGISTRATION_PATH: &str = "src-tauri/src/lib.rs";
const EVENTS_PATH: &str = "src-tauri/src/events.rs";
const ERROR_PATH: &str = "src-tauri/src/error.rs";
const REMOTE_DISPATCH_PATH: &str = "src-tauri/src/domain/remote/dispatch.rs";

const IPC_REGISTRATION_SOURCE: &str = include_str!("../src/lib.rs");
const EVENTS_SOURCE: &str = include_str!("../src/events.rs");
const ERROR_SOURCE: &str = include_str!("../src/error.rs");
const REMOTE_DISPATCH_SOURCE: &str = include_str!("../src/domain/remote/dispatch.rs");
const BINDINGS_SOURCE: &str = include_str!("../../src/shared/api/bindings.ts");

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IpcContractManifest {
    schema_version: u32,
    ordering: String,
    sources: ManifestSources,
    generated_bindings: ManifestBindings,
    specta_commands: Vec<String>,
    raw_channel_commands: Vec<String>,
    events: Vec<ManifestEvent>,
    error_wire_codes: Vec<String>,
    remote_policy: ManifestRemotePolicy,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestSources {
    ipc_registration: String,
    events: String,
    error: String,
    remote_dispatch: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestBindings {
    path: String,
    algorithm: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestEvent {
    rust_type: String,
    wire_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestRemotePolicy {
    policies: Vec<String>,
    unclassified_policy: String,
    implemented_json_commands: Vec<String>,
    raw_dispatch_commands: Vec<String>,
    allowed_commands: Vec<String>,
    denied_commands: Vec<ManifestDeniedCommand>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ManifestDeniedCommand {
    command: String,
    policy: String,
}

fn manifest() -> IpcContractManifest {
    serde_json::from_str(include_str!("fixtures/rust-native/ipc-contract-manifest.json"))
        .expect("rust-native IPC 계약 manifest 를 읽지 못했습니다")
}

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn between<'a>(source: &'a str, start_marker: &str, end_marker: &str) -> &'a str {
    let start = source
        .find(start_marker)
        .unwrap_or_else(|| panic!("시작 마커를 찾을 수 없습니다: {start_marker}"))
        + start_marker.len();
    let end = source[start..]
        .find(end_marker)
        .unwrap_or_else(|| panic!("종료 마커를 찾을 수 없습니다: {end_marker}"));
    &source[start..start + end]
}

fn quoted_names(block: &str) -> Vec<String> {
    let pattern = Regex::new(r#""([^"]+)""#).expect("유효한 정규식");
    pattern.captures_iter(block).map(|capture| capture[1].to_string()).collect()
}

fn command_path_names(block: &str) -> Vec<String> {
    let pattern = Regex::new(r"::commands::([a-z_0-9]+)").expect("유효한 정규식");
    pattern.captures_iter(block).map(|capture| capture[1].to_string()).collect()
}

fn specta_command_names(source: &str) -> Vec<String> {
    let stripped = strip_line_comments(source);
    command_path_names(between(&stripped, "collect_commands![", "]"))
}

fn raw_channel_command_names(source: &str) -> Vec<String> {
    let stripped = strip_line_comments(source);
    quoted_names(between(&stripped, "RAW_CHANNEL_COMMANDS: &[&str] = &[", "]"))
}

fn raw_channel_handler_names(source: &str) -> Vec<String> {
    let stripped = strip_line_comments(source);
    command_path_names(between(&stripped, "tauri::generate_handler![", "]"))
}

fn registered_event_types(source: &str) -> Vec<String> {
    let stripped = strip_line_comments(source);
    between(&stripped, "collect_events![", "]")
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn declared_events(source: &str) -> Vec<(String, String)> {
    let pattern = Regex::new(r#"#\[tauri_specta\(event_name = "([^"]+)"\)\]\s*pub struct ([A-Za-z][A-Za-z0-9]*)"#).expect("유효한 정규식");
    let stripped = strip_line_comments(source);
    pattern
        .captures_iter(&stripped)
        .map(|capture| (capture[2].to_string(), capture[1].to_string()))
        .collect()
}

fn app_error_wire_codes(source: &str) -> Vec<String> {
    let stripped = strip_line_comments(source);
    assert!(
        stripped.contains("#[serde(tag = \"code\", content = \"message\")]\npub enum AppError {"),
        "AppError 의 serde tag 계약이 바뀌었습니다 — wire code 는 변형 이름이라는 전제가 깨집니다"
    );
    let body = between(&stripped, "pub enum AppError {", "\n}");
    let variants = body
        .lines()
        .filter(|line| !line.trim_start().starts_with("#["))
        .collect::<Vec<_>>()
        .join("\n");
    let pattern = Regex::new(r"(?m)^[ \t]+([A-Z][A-Za-z0-9]*)[ \t]*[\(,]").expect("유효한 정규식");
    pattern.captures_iter(&variants).map(|capture| capture[1].to_string()).collect()
}

fn denial_policy_names(source: &str) -> Vec<String> {
    let stripped = strip_line_comments(source);
    let body = between(&stripped, "enum RemoteDenialPolicy {", "\n}");
    let pattern = Regex::new(r"(?m)^\s+([A-Z][A-Za-z0-9]*),\s*$").expect("유효한 정규식");
    pattern.captures_iter(body).map(|capture| capture[1].to_string()).collect()
}

fn implemented_json_commands(source: &str) -> Vec<String> {
    let stripped = strip_line_comments(source);
    quoted_names(between(&stripped, "IMPLEMENTED_JSON_COMMANDS: &[&str] = &[", "];"))
}

fn allowed_remote_commands(source: &str) -> Vec<String> {
    let stripped = strip_line_comments(source);
    quoted_names(between(&stripped, "REMOTE_ALLOWED_COMMANDS: &[&str] = &[", "];"))
}

fn denied_remote_commands(source: &str) -> Vec<(String, String)> {
    let stripped = strip_line_comments(source);
    let block = between(&stripped, "REMOTE_DENIED_COMMANDS: &[RemoteDeniedCommandEntry] = &[", "];");
    let pattern = Regex::new(r#"\("([^"]+)",\s*RemoteDenialPolicy::([A-Za-z0-9_]+)\)"#).expect("유효한 정규식");
    pattern
        .captures_iter(block)
        .map(|capture| (capture[1].to_string(), capture[2].to_string()))
        .collect()
}

fn raw_dispatch_arms(source: &str) -> Vec<String> {
    let stripped = strip_line_comments(source);
    let block = between(&stripped, "pub async fn dispatch_raw(", "#[cfg(test)]");
    let pattern = Regex::new(r#"(?m)^        "([a-zA-Z0-9_]+)" => "#).expect("유효한 정규식");
    pattern.captures_iter(block).map(|capture| capture[1].to_string()).collect()
}

fn bound_invoke_names(source: &str) -> Vec<String> {
    let pattern = Regex::new(r#"__TAURI_INVOKE\(\s*"([a-zA-Z0-9_]+)""#).expect("유효한 정규식");
    pattern.captures_iter(source).map(|capture| capture[1].to_string()).collect()
}

fn bound_event_wire_names(source: &str) -> Vec<String> {
    let block = between(source, "export const events = {", "};");
    let pattern = Regex::new(r#"makeEvent<[A-Za-z0-9_]+>\("([a-zA-Z0-9:_-]+)"\)"#).expect("유효한 정규식");
    pattern.captures_iter(block).map(|capture| capture[1].to_string()).collect()
}

fn duplicates(names: &[String]) -> Vec<String> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for name in names {
        *counts.entry(name.as_str()).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(name, _)| name.to_string())
        .collect()
}

fn assert_no_duplicates(label: &str, names: &[String]) {
    let found = duplicates(names);
    assert!(
        found.is_empty(),
        "{label} 에 중복 항목이 있습니다 — 같은 이름이 두 번 등록되면 집합 비교가 그 중복을 숨깁니다:\n{found:#?}"
    );
}

fn assert_contains_exactly(label: &str, recorded: &[String], scanned: &[String]) {
    assert!(
        !recorded.is_empty(),
        "{label}: manifest 목록이 비어 있습니다 — 스키마가 원천을 잃었습니다"
    );
    assert!(
        !scanned.is_empty(),
        "{label}: 원천 스캔 결과가 비어 있습니다 — 마커나 정규식이 원천 형식과 어긋났습니다"
    );
    assert_no_duplicates(&format!("{label} manifest"), recorded);
    assert_no_duplicates(&format!("{label} 원천"), scanned);

    let recorded_set: BTreeSet<&str> = recorded.iter().map(String::as_str).collect();
    let scanned_set: BTreeSet<&str> = scanned.iter().map(String::as_str).collect();

    let missing: Vec<&&str> = scanned_set.difference(&recorded_set).collect();
    assert!(
        missing.is_empty(),
        "{label}: 원천에 있고 manifest 에 없는 항목(신규 누락)이 있습니다 — 원천 등록과 manifest 를 함께 갱신하십시오:\n{missing:#?}"
    );

    let stale: Vec<&&str> = recorded_set.difference(&scanned_set).collect();
    assert!(
        stale.is_empty(),
        "{label}: manifest 에 있고 원천에 없는 항목(stale)이 있습니다 — 폐기된 항목이 manifest 에 남아 있습니다:\n{stale:#?}"
    );
}

fn assert_list_matches(label: &str, recorded: &[String], scanned: &[String]) {
    assert_contains_exactly(label, recorded, scanned);
    assert_eq!(
        recorded, scanned,
        "{label}: 집합은 같지만 선언 순서가 다릅니다 — manifest 는 {MANIFEST_ORDERING} 를 그대로 기록해야 합니다"
    );
}

fn assert_denied_matches(recorded: &[ManifestDeniedCommand], scanned: &[(String, String)]) {
    let recorded_pairs: Vec<(String, String)> = recorded.iter().map(|entry| (entry.command.clone(), entry.policy.clone())).collect();

    assert!(
        !recorded_pairs.is_empty() && !scanned.is_empty(),
        "deniedCommands: manifest 또는 원천 거부 테이블이 비어 있습니다 — 기본 거부 정책이 사라졌는지 확인하십시오"
    );
    assert_no_duplicates(
        "deniedCommands manifest",
        &recorded.iter().map(|entry| entry.command.clone()).collect::<Vec<_>>(),
    );
    assert_no_duplicates(
        "deniedCommands 원천",
        &scanned.iter().map(|(command, _)| command.clone()).collect::<Vec<_>>(),
    );

    let missing: Vec<&(String, String)> = scanned.iter().filter(|pair| !recorded_pairs.contains(pair)).collect();
    assert!(
        missing.is_empty(),
        "deniedCommands: 원천에 있고 manifest 에 없는 거부 항목(신규 누락)이 있습니다:\n{missing:#?}"
    );

    let stale: Vec<&(String, String)> = recorded_pairs.iter().filter(|pair| !scanned.contains(pair)).collect();
    assert!(
        stale.is_empty(),
        "deniedCommands: manifest 에 있고 원천에 없는 거부 항목(stale)이 있습니다:\n{stale:#?}"
    );

    assert_eq!(
        recorded_pairs,
        scanned.to_vec(),
        "deniedCommands: 집합은 같지만 선언 순서가 다릅니다 — manifest 는 {MANIFEST_ORDERING} 를 그대로 기록해야 합니다"
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[test]
fn manifest_스키마와_원천_경로가_계약과_일치한다() {
    let manifest = manifest();

    assert_eq!(
        manifest.schema_version, SUPPORTED_SCHEMA_VERSION,
        "manifest schemaVersion 이 지원 버전과 다릅니다 — 스키마 변경은 이 테스트의 명시적 갱신을 거쳐야 합니다"
    );
    assert_eq!(manifest.ordering, MANIFEST_ORDERING, "manifest ordering 선언이 계약과 다릅니다");
    assert_eq!(manifest.sources.ipc_registration, IPC_REGISTRATION_PATH);
    assert_eq!(manifest.sources.events, EVENTS_PATH);
    assert_eq!(manifest.sources.error, ERROR_PATH);
    assert_eq!(manifest.sources.remote_dispatch, REMOTE_DISPATCH_PATH);
    assert_eq!(manifest.generated_bindings.path, BINDINGS_PATH);
    assert_eq!(manifest.generated_bindings.algorithm, BINDINGS_HASH_ALGORITHM);
}

#[test]
fn specta_command_집합이_manifest와_양방향_일치한다() {
    let manifest = manifest();

    assert_list_matches(
        "spectaCommands",
        &manifest.specta_commands,
        &specta_command_names(IPC_REGISTRATION_SOURCE),
    );

    let overlap: Vec<&String> = manifest
        .raw_channel_commands
        .iter()
        .filter(|name| manifest.specta_commands.contains(name))
        .collect();
    assert!(
        overlap.is_empty(),
        "raw channel 커맨드가 Specta command 와 겹칩니다 — raw handler 와 Specta handler 라우팅이 갈라집니다:\n{overlap:#?}"
    );
}

#[test]
fn raw_channel_command_집합이_manifest와_등록_핸들러에_일치한다() {
    let manifest = manifest();

    assert_list_matches(
        "rawChannelCommands",
        &manifest.raw_channel_commands,
        &raw_channel_command_names(IPC_REGISTRATION_SOURCE),
    );
    assert_contains_exactly(
        "rawChannelCommands vs generate_handler",
        &manifest.raw_channel_commands,
        &raw_channel_handler_names(IPC_REGISTRATION_SOURCE),
    );
}

#[test]
fn event_타입과_wire_이름이_manifest와_양방향_일치한다() {
    let manifest = manifest();
    let declared = declared_events(EVENTS_SOURCE);

    let manifest_types: Vec<String> = manifest.events.iter().map(|event| event.rust_type.clone()).collect();
    let manifest_wires: Vec<String> = manifest.events.iter().map(|event| event.wire_name.clone()).collect();
    let declared_types: Vec<String> = declared.iter().map(|(rust_type, _)| rust_type.clone()).collect();
    let declared_wires: Vec<String> = declared.iter().map(|(_, wire_name)| wire_name.clone()).collect();

    assert_list_matches("event rustType", &manifest_types, &declared_types);
    assert_list_matches("event wireName", &manifest_wires, &declared_wires);

    assert_contains_exactly("collect_events!", &manifest_types, &registered_event_types(IPC_REGISTRATION_SOURCE));
    assert_contains_exactly("bindings events", &manifest_wires, &bound_event_wire_names(BINDINGS_SOURCE));
}

#[test]
fn apperror_wire_code_집합이_manifest와_양방향_일치한다() {
    let manifest = manifest();

    assert_list_matches("errorWireCodes", &manifest.error_wire_codes, &app_error_wire_codes(ERROR_SOURCE));
}

#[test]
fn remote_허용_거부_테이블이_전체_커맨드를_교집합_없이_정확히_분할한다() {
    let manifest = manifest();
    let remote = &manifest.remote_policy;

    assert_list_matches("remote policies", &remote.policies, &denial_policy_names(REMOTE_DISPATCH_SOURCE));
    assert_list_matches(
        "implementedJsonCommands",
        &remote.implemented_json_commands,
        &implemented_json_commands(REMOTE_DISPATCH_SOURCE),
    );
    assert_list_matches(
        "allowedCommands",
        &remote.allowed_commands,
        &allowed_remote_commands(REMOTE_DISPATCH_SOURCE),
    );
    assert_list_matches(
        "rawDispatchCommands",
        &remote.raw_dispatch_commands,
        &raw_dispatch_arms(REMOTE_DISPATCH_SOURCE),
    );
    assert_denied_matches(&remote.denied_commands, &denied_remote_commands(REMOTE_DISPATCH_SOURCE));

    assert_contains_exactly(
        "implementedJsonCommands vs spectaCommands",
        &remote.implemented_json_commands,
        &manifest.specta_commands,
    );

    let policies: BTreeSet<&str> = remote.policies.iter().map(String::as_str).collect();
    assert!(
        policies.contains(remote.unclassified_policy.as_str()),
        "unclassifiedPolicy({}) 가 거부 정책 목록에 없습니다 — 기본 거부 fallback 이 사라졌습니다",
        remote.unclassified_policy
    );
    let unknown_policies: Vec<&str> = remote
        .denied_commands
        .iter()
        .map(|entry| entry.policy.as_str())
        .filter(|policy| !policies.contains(policy))
        .collect();
    assert!(
        unknown_policies.is_empty(),
        "거부 항목이 참조하는 정책 이름이 policies 목록에 없습니다:\n{unknown_policies:#?}"
    );

    let universe: BTreeSet<&str> = manifest
        .specta_commands
        .iter()
        .chain(manifest.raw_channel_commands.iter())
        .map(String::as_str)
        .collect();
    let allowed: BTreeSet<&str> = remote.allowed_commands.iter().map(String::as_str).collect();
    let denied: BTreeSet<&str> = remote.denied_commands.iter().map(|entry| entry.command.as_str()).collect();

    let overlap: Vec<&&str> = allowed.intersection(&denied).collect();
    assert!(
        overlap.is_empty(),
        "허용·거부 테이블에 동시에 분류된 커맨드가 있습니다 — 허용이 거부를 덮어씁니다:\n{overlap:#?}"
    );

    let classified: BTreeSet<&str> = allowed.union(&denied).copied().collect();
    let unclassified: Vec<&&str> = universe.difference(&classified).collect();
    assert!(
        unclassified.is_empty(),
        "전체 IPC 커맨드 중 허용·거부 어느 테이블에도 분류되지 않은 항목이 있습니다 — 기본 거부 계약 위반:\n{unclassified:#?}"
    );

    let unknown: Vec<&&str> = classified.difference(&universe).collect();
    assert!(
        unknown.is_empty(),
        "허용·거부 테이블에 IPC 커맨드 집합에 없는 이름이 있습니다 — 유령 분류가 남았습니다:\n{unknown:#?}"
    );

    let raw: BTreeSet<&str> = manifest.raw_channel_commands.iter().map(String::as_str).collect();
    let stray_raw: Vec<&String> = remote
        .raw_dispatch_commands
        .iter()
        .filter(|name| !raw.contains(name.as_str()))
        .collect();
    assert!(
        stray_raw.is_empty(),
        "rawDispatchCommands 는 rawChannelCommands 의 부분집합이어야 합니다:\n{stray_raw:#?}"
    );
}

#[test]
fn bindings_해시와_command_이름이_현재_파일과_일치한다() {
    let manifest = manifest();

    let digest = sha256_hex(BINDINGS_SOURCE.as_bytes());
    assert_eq!(
        manifest.generated_bindings.sha256, digest,
        "bindings.ts 전체 바이트 SHA-256 이 manifest 와 다릅니다 — command 입력·출력 DTO 나 event payload 가 드리프트했습니다"
    );

    let lowercase_hex = manifest.generated_bindings.sha256.len() == 64
        && manifest
            .generated_bindings
            .sha256
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'));
    assert!(lowercase_hex, "manifest 의 sha256 값이 소문자 16진수 64자리가 아닙니다");

    assert_contains_exactly("bindings invoke", &manifest.specta_commands, &bound_invoke_names(BINDINGS_SOURCE));

    let manifest_wires: Vec<String> = manifest.events.iter().map(|event| event.wire_name.clone()).collect();
    assert_contains_exactly("bindings event wire", &manifest_wires, &bound_event_wire_names(BINDINGS_SOURCE));
}
