# 04. 규칙과 관례 — Rust-native 전환 마무리 작업용

> 작성일: 2026-11-03 · 기준: 브랜치 `to_rust_native` HEAD `0c52b438`, Git 영역은 병합 중인 worktree `/Users/hyunseokbyun/development/TAIDE-lanes/git`(브랜치 `lane/git`, HEAD `50892b59`)
> 이 문서는 사용자 전역 규칙(`/Users/hyunseokbyun/.claude/convention/*.md`)과 이 저장소의 합의(`docs/acknowledge/`, `docs/memory/`, `docs/research/2026-10-28-native-lane-seams.md`, lane workflow script)를 Rust-native 작업에 맞게 모은 것입니다. 원문과 다르면 원문이 정본입니다. 원문 위치는 각 절 끝의 "근거"에 적었습니다.

## 0. 규칙의 우선순위

규칙끼리 부딪히면 아래 순서로 따릅니다(`/Users/hyunseokbyun/.claude/convention/index.md` "규칙 충돌 · 공백 시 판단 기준").

1. 시스템·실행환경 지시(권한, 도구 계약)
2. 사용자의 명시적 지시(현재 대화). 예: "커밋하지 마"가 있으면 자동 커밋 합의보다 우선합니다(`docs/agent-operations.md` 4절, `docs/feedback/2026-08-27-commit-despite-no-commit-directive.md`).
3. 프로젝트 합의: `docs/acknowledge/*.md`(사용자 결정), `docs/memory/*.md`(항상 적용하는 운영 지식), `docs/research/2026-10-28-native-lane-seams.md`(영역 경계 규칙)
4. 사용자 전역 컨벤션: `/Users/hyunseokbyun/.claude/convention/{index,ai-process,common,comments,security,git,desktop}.md`
5. 기존 코드 패턴

판단 기준:

- 전역 컨벤션의 `frontend.md`·`fsd.md`·`query.md`·`backend.md`는 TypeScript·React·Hono 전용이라 Rust-native 코드에는 적용하지 않습니다. `common.md`의 언어 무관 원칙(매직넘버, early return, 최소 변경, 네이밍 정확성, 불변성 우선)과 `comments.md`, `security.md`, `git.md`, `desktop.md`(렌더러·셸 분리, 권한 최소화)는 적용합니다.
- 저장소 루트에는 `AGENTS.md`·`CLAUDE.md`가 없습니다(`ls /Users/hyunseokbyun/development/TAIDE`로 확인). 프로젝트 규칙은 위 3번 문서들이 담당합니다.
- 상위 규칙을 따르느라 컨벤션과 다르게 했으면 그 사실을 사용자에게 알립니다.

## 1. 코드 규칙

### 1.1 주석 금지, 영어 doc comment만 허용

- 새 코드에 `//`, `/* */` 설명 주석을 쓰지 않습니다. 이름·구조·타입으로 설명합니다.
- 유일한 예외는 의미가 자명하지 않은 공개(또는 `pub(crate)`) 항목의 영어 `///` doc comment입니다. 저장소의 실제 형태:

```rust
/// One toaster per window, like `AppToaster` mounted in every window (`app-providers.tsx` 15-26).
/// Dereferencing reaches the layer of the window that had OS focus when the frame began, so a
/// plain `toasts.error(..)` for a user action lands in the window the user is acting in.
/// Providers only the main window mounts (external open, notifications, drops) use
/// [`ToastLayers::main`].
pub(crate) struct ToastLayers {
```

(출처: `native/taide-native-app/src/toast-layers.rs` 9-14, `ToastLayers`)

- `#[path = ...]`, `#[cfg(...)]` 같은 attribute는 주석이 아니므로 제한 대상이 아닙니다.
- 기존 주석: 요청 범위 밖의 기존 주석은 지우지 않습니다. 지금 고치는 코드에 붙은 주석은 이름·구조 개선으로 대체하고 제거합니다. 쓰지 않게 된 코드는 주석으로 보관하지 않고 삭제합니다.
- 코드에 남기고 싶은 설명(왜 이렇게 했는지, 원본과 다른 점, 트레이드오프)은 그 묶음의 QA 문서(`docs/quality-assurance/`)나 사양(`docs/research/`)에 적습니다.

근거: `comments.md` 1~3절, lane workflow `RULES`("코드 주석 새로 쓰지 않음(공개 API 영어 /// 만 예외)", `docs/utils/2026-11-02-native-lane-workflow.js`).

### 1.2 매직넘버는 의미 있는 const

- 시간·크기·임계값·반복 횟수 같은 리터럴은 파일 위쪽의 `UPPER_SNAKE_CASE` const로 둡니다. 0, 1, 빈 문자열처럼 자명한 값은 예외입니다.
- 테스트도 같습니다. 실제 예(`native/taide-native-app/src/application-commands-tests.rs` 상단):

```rust
const DEADLINE: Duration = Duration::from_secs(10);
const SETTLE_FRAMES: usize = 4;
const HALF: f32 = 50.0;
const AUXILIARY_SCREEN: [f32; 2] = [1000.0, 700.0];
```

- 원본 TS의 상수를 옮길 때는 값과 의미를 그대로 쓰고, 어느 원본 상수인지 사양·QA 문서에 대응표로 남깁니다(예: `docs/research/2026-10-28-lane-git-spec.md` 2.0.8 "원본 상수 대응").

근거: `common.md` 4.1절, lane workflow `RULES`.

### 1.3 우회 금지, 근본 원인 수정

- 검사기를 끄는 attribute(`#[allow(...)]` 등)를 새로 넣지 않습니다. 현재 native crate 3개(app·ui·editor `src`)에서 `#[allow(` 는 `native/taide-native-editor/src/json-schema-document.rs` 60줄(`#[allow(clippy::too_many_arguments)]`, `diagnostics`) 한 곳뿐입니다(`rg -c "#\[allow\("`로 확인). 범위 밖이면 지우지도 않습니다.
- 테스트 기대값을 바꿔 실패를 통과시키지 않습니다. 앱이 틀렸으면 앱을 고칩니다. 원본 TS 동작 확인 결과 기대값 자체가 틀렸다면 원본 근거(경로:줄)를 QA 문서에 적고 고칩니다.
- 실패를 재현하려고 구현을 임시로 끄거나 되돌리는 편집을 하지 않습니다(자동 모드에서 거부되고 원복 실수 위험이 있음, `docs/memory/2026-10-07-native-transition-operations.md` 3절).
- 흔들리는 검사를 대기 시간 늘리기·단언 약화로 덮지 않습니다. 원인이 검사 설계(테스트 기대값 변경 필요)이면 범위 밖으로 기록만 합니다. 선례: `docs/research/2026-10-28-native-lane-seams.md` 3절의 보조 창 layout revision 경쟁 3건.
- 같은 가정에 기반한 수정이 세 번 연속 실패하면 멈추고, 틀렸을 수 있는 가정과 원인 가설을 기록한 뒤 질문합니다.
- 원본 TS의 결함·내부 수치는 재현하지 않습니다(사용자 결정 2026-10-09). 원본과 다르게 둔 점은 QA 문서의 "원본과 다르게 둔 점(근거)" 절에 적습니다.

근거: `ai-process.md` 6.2·8절, `docs/acknowledge/2026-10-09-native-full-resume.md`, `docs/acknowledge/2026-10-09-native-find-regex-decisions.md`.

### 1.4 최소 변경과 기존 스타일

- 요청 범위 밖의 리팩터링·리네임·재포맷을 하지 않습니다. 범위 밖 수정이 꼭 필요하면 최소로 하고 QA 문서에 사유를 적습니다.
- 내 변경으로 쓰이지 않게 된 import·함수·필드는 함께 지웁니다. 범위 밖의 기존 dead code는 보고만 합니다.
- 포맷은 rustfmt 기본값입니다(native crate에 `rustfmt.toml`이 없음). 변경한 crate마다 `cargo fmt --manifest-path <절대경로>/Cargo.toml -- --check`가 통과해야 합니다. app crate는 wave 0에서 기존 fmt 차이를 정리했으므로(`186c2d5a style(native): 앱 크레이트 기존 rustfmt 차이 정리`) `cargo fmt`를 돌려도 내 코드만 바뀌어야 합니다. 다른 파일이 바뀌면 되돌리고 원인을 보고합니다.
- 기존 코드가 쓰는 관례를 따릅니다.
    - Rust 2024 edition(`native/taide-native-app/Cargo.toml`의 `edition = "2024"`, `rust-version = "1.95"`). `if let A && let B` 형태의 let chain을 씁니다(예: `native/taide-native-app/src/application.rs` 2130-2140, `consume_key(..., egui::Key::N)` 주변).
    - 오류 타입은 `taide_model::error::{AppError, AppResult}`입니다.
    - crate 내부 공개는 `pub(crate)`, 영역 메서드 시그니처에 쓰이는 `application.rs` 타입도 `pub(crate)`로 올립니다(seams 1.1절).
    - 외부 효과(클립보드, 플랫폼, 비밀 저장소)는 `Arc<dyn ...>` 포트로 주입합니다(2절 표).
- UI는 egui 0.36.2(`native/taide-native-ui/Cargo.toml` `egui = { version = "=0.36.2" }`, 앱은 vendored `vendor/egui-input` patch와 `experiments/native-shell-spike/vendor/eframe`)입니다. egui는 즉시 모드 GUI라 매 프레임 `Context::run_ui`가 화면 전체를 다시 그리고, 위젯 상태는 앱 구조체와 `egui::Memory`에 둡니다.

근거: `ai-process.md` 6.8절, lane workflow `RULES`("요청 밖 리팩터링·리네임·재포맷 금지").

### 1.5 테스트 이름은 한국어

- 테스트 함수 이름은 한국어 snake_case 식별자입니다. Rust는 유니코드 식별자를 허용합니다.
- 실제 `NativeApplication`을 띄워 프레임을 돌리는 검사는 `실제_app_` 또는 `실제_` 접두사를 씁니다.

```rust
#[test]
fn 상호작용_층은_포커스된_보조_창이고_주_창_층과_닫힌_창은_분리된다() {
```

(`native/taide-native-app/src/toast-layers-tests.rs` 10-11줄)

```rust
#[test]
fn 실제_app_팔레트_view_welcome은_포커스_pane에_한번만_열고_재실행은_활성화한다() {
```

(`native/taide-native-app/src/application-commands-tests.rs`)

- 이름은 "조건 + 기대 동작"을 문장으로 씁니다. 길어도 됩니다(`ai-process.md` 6.4절).

### 1.6 사용자 문자열은 locale 키

- 화면 문자열을 하드코딩하지 않습니다. 원본 TS와 같은 키로 `taide_native_ui::presentation::message`를 부릅니다.

```rust
let notice = presentation::message(&application.locale, "app.openProjectFirst", &[]);
```

(`native/taide-native-app/src/application-commands-tests.rs`; 함수 정의 `native/taide-native-ui/src/presentation.rs` 538줄 `pub fn message(locale: &ResolvedLocale, key: &str, arguments: &[(&str, &str)]) -> String`)

- 카탈로그는 Tauri 앱과 공유하는 crate의 `crates/taide-locale/resources/locales/{en,ko,ja}.json`(평면 JSON, 키 정렬)이고, 원본 TS가 쓰는 키와 같은 키 체계입니다(예: `app.openProjectFirst`가 `src/widgets/app-shell/app-shell.tsx`, `src/app/providers/agent-external-open-provider.tsx`에서도 쓰임). 그래서 원본에 있는 키를 먼저 찾아(`rg -n "<키>" /Users/hyunseokbyun/development/TAIDE/src`) 재사용하고, 새 키가 꼭 필요하면 세 언어 파일에 같은 키를 정렬 위치에 함께 추가합니다. 공유 crate이므로 기존 키의 값을 바꾸면 Tauri 앱 화면도 함께 바뀐다는 점을 확인하고 QA에 적습니다.
- 키 접두사와 소유 영역(seams 1.4절): `project.*`·`projectGroup.*`·`welcome.*`·`zen.*`·`window.*`·`systemUsage.*` → shell, `palette.*`·`keybindings.*`·`task*` → commands, `editor.*` → editor, `explorer.*`·`search.*`·`problems.*`·`outline.*` → explorer_search, `git.*` → git, `terminal.*`·`agent.*`·`ide.*` → terminal_agent, `settings.*`·`theme.*`·`snippet*` → settings, `preview.*` → preview. 목록에 없는 접두사를 처음 쓰는 lane은 seams 문서 1.4절에 한 줄 추가합니다.
- 예외: 찾기 위젯의 정규식 엔진 오류 문장은 영어 원문을 그대로 표시합니다(결정 2026-10-09, `docs/acknowledge/2026-10-09-native-find-regex-decisions.md` 6번).

### 1.7 파일 이름은 kebab-case `.rs` + `#[path]`

- 새 파일은 kebab-case 이름으로 만들고, 모듈 선언에 `#[path]`를 붙입니다. `lib.rs` 모듈 목록은 알파벳 순(rustfmt `reorder_modules`)입니다.

```rust
#[path = "agent-cli.rs"]
mod agent_cli;
#[path = "agent-hooks.rs"]
pub mod agent_hooks;
#[cfg(target_os = "macos")]
#[path = "app-menu-bar.rs"]
mod app_menu_bar;
pub mod application;
```

(`native/taide-native-app/src/lib.rs` 1-16)

- 기존 snake_case 파일(`application.rs`, `host.rs`, `explorer.rs`, `terminal_surface.rs`, `explorer_clipboard.rs`, `close_dialog.rs` 등)은 이름을 바꾸지 않습니다(최소 변경).
- 통합 테스트는 `native/taide-native-app/tests/<kebab-case>.rs`이고 `cargo test --test terminal-host`처럼 파일 이름으로 고릅니다.
- 영역 모듈은 `native/taide-native-app/src/domains/<영역>.rs`(영역 이름은 snake_case: `explorer_search.rs`, `terminal_agent.rs`)입니다.

### 1.8 테스트 파일 분리 `*-tests.rs`

- 모듈 단위 검사는 같은 이름 + `-tests.rs` 파일에 두고, 대상 파일 끝에서 선언합니다.

```rust
#[cfg(test)]
#[path = "toast-layers-tests.rs"]
mod tests;
```

(`native/taide-native-app/src/toast-layers.rs` 96-98) 테스트 파일은 `use super::*;`로 비공개 항목에 접근합니다.

- 실제 App 검사는 `native/taide-native-app/src/application-<주제>-tests.rs`에 두고 `application.rs` 끝에서 선언합니다(`application.rs` 2899줄 부근).

```rust
#[path = "application-exit-tests.rs"]
mod exit_tests;
#[cfg(test)]
#[path = "application-code-lenses-tests.rs"]
mod code_lenses_tests;
#[cfg(all(test, unix))]
#[path = "application-inlay-hints-tests.rs"]
mod inlay_hints_tests;
```

- lane 작업에서 새로 만든 실제 App 검사 파일은 공용 `application.rs` 대신 자기 영역 파일 끝에 선언합니다(공용 파일 충돌 방지). 실제 예(`native/taide-native-app/src/domains/shell.rs` 1632-1643):

```rust
#[cfg(all(test, unix))]
#[path = "../application-ipc-sync-tests.rs"]
mod ipc_sync_tests;
#[cfg(all(test, unix))]
#[path = "../application-notifications-tests.rs"]
mod notifications_tests;
```

- 여러 검사 파일이 공유하는 도우미는 기존 것을 재사용합니다: `crate::application::project_rail_tests::{Fixture, insert_project, isolated_state, launch, step, project_ref}`(`application.rs` 2962줄 선언), `domains::shell::notifications_tests::{FakePlatform, launch_with_platform, frame, frame_until}`, `domains::shell::ipc_sync_tests::{open_project, open_project_with, edit}`.

### 1.9 영역 경계 규칙(병렬 lane의 충돌 방지)

`docs/research/2026-10-28-native-lane-seams.md` 1.1~1.5절이 정본입니다. 요지:

| 넣을 것 | 위치 |
| --- | --- |
| host 요청·응답 | `domains/<영역>.rs`의 `pub enum Command`·`pub enum Reply`. 루트 `HostCommand::<영역>(..)`·`HostReply::<영역>(..)` wrapper와 `host.rs` `dispatch` 라우팅은 손대지 않음 |
| 영역 상태 | `domains/<영역>.rs`의 `pub(crate) struct State`(필드 `NativeApplication.<영역>_state`). 루트 struct에 필드를 추가하지 않음 |
| worker 처리 | `domains/<영역>.rs`의 `pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply>` |
| 응답 반영·intent 처리·tick·show | `domains/<영역>.rs`의 `impl NativeApplication` 안 `apply_<영역>_reply`, `apply_<영역>_intent`, `tick_<영역>_*`, `show_<영역>_*` |
| UI intent | `native/taide-native-ui/src/intents/<영역>.rs`의 `pub enum Intent`(새 shell intent도 루트 `ShellIntent`가 아니라 `intents/shell.rs`) |
| 영역 surface 메서드 | `native/taide-native-ui/src/surfaces/<영역>.rs`의 `<영역>Surfaces` trait(supertrait 목록에 한 줄) |
| 공용 파일(`application.rs`, `lib.rs` 모듈 목록, locale 카탈로그) | 자기 영역 블록 끝에 추가만 |

다른 영역 상태는 읽기만 하고, 바꿔야 하면 소유 영역의 `pub(crate)` 메서드를 부릅니다(seams 1.5절).

### 1.10 동작 기준은 원본 TS

- 기준은 원본 TS 화면·상태·상호작용입니다: TS `src/`, Tauri `src-tauri/`, 공유 코어 `crates/`, 원본이 쓰던 Monaco·xterm·Shiki 소스(`node_modules/`, worktree에는 없으므로 메인 저장소의 것을 읽기 전용으로 읽음).
- 새 기능·새 디자인을 추가하지 않습니다(`docs/HANDOFF.md` 5절 1번, `docs/acknowledge/2026-10-09-native-full-resume.md`). 원본에 없는 동작이 필요하면 사용자 결정으로 올립니다.
- 원본 근거는 QA 문서에 `경로 줄-줄` 형태로 적습니다(예: `src/app/providers/native-notification-provider.tsx` 57-204).
- 기준 자료(oracle)를 구현 출력에 맞춰 만들지 않습니다. 원본 라이브러리를 직접 실행한 결과를 씁니다(예: `docs/utils/2026-10-10-monaco-*-oracle.js`, `bun <절대경로>` 한 줄 실행).

### 1.11 참고 출처 비표기

외부 오픈소스 제품을 설계 참고로 읽는 것은 허용되지만, 그 제품 이름·경로·식별자를 `docs/`·커밋 메시지·코드·릴리스 노트에 남기지 않습니다. 코드는 그대로 복사하지 않고 이 저장소 구조로 다시 씁니다. 공개 규격 이름(OSC 133 등)은 적어도 됩니다(사용자 지시 2026-09-06, 사용자 auto-memory `reference-source-attribution`).

## 2. 보호 규칙과 테스트 대체 방법

작업자(하위 에이전트)와 자동 검사는 아래 대상을 실제로 건드리지 않습니다. 실제 확인이 필요하면 사용자 허락 뒤 별도로 합니다(06 문서 "허락 대기").

| 보호 대상 | 금지 | 테스트에서 대신 쓰는 것(실제 심볼) |
| --- | --- | --- |
| GUI 실행 | 작업자는 앱 창을 띄우지 않음 | headless 실제 App: `eframe::CreationContext::_new_kittest(context)`로 `NativeApplication::new(..)`를 만들고 `eframe::App::logic` + `context.run_ui(..)`로 프레임을 돌림(`application-project-rail-tests.rs`의 `launch`·`step`). 화면 위치는 AccessKit 노드로 찾음(`node_rect`, `context.enable_accesskit()`) |
| 사용자 실제 데이터 | 실제 앱 데이터 경로·실제 프로젝트 파일에 쓰지 않음 | `isolated_state(<임시 디렉터리>/data)`(`AppState::new(AppPaths::new(data))`, IDE 서버·agent hooks 끔), 임시 디렉터리 `std::env::temp_dir().join(format!("taide-native-<이름>-{}", ProjectId::new()))`. 앱 실행은 `--data-dir` 필수, Keychain 서비스명은 격리용 `net.gumyo.taide.native-isolated`(`docs/architecture.md` 9.3절) |
| OS 설정·OS 알림·시스템 설정 앱·Finder | osascript·알림 전송·`open` 호출 없음 | `Overrides { platform: Some(Arc<FakePlatform>) }`(`application.rs` `Overrides`), `domains::shell::notifications_tests::FakePlatform`(`src/application-notifications-tests.rs` 26줄)이 `send_notification`을 기록하고 `open_path`·`reveal_item_in_dir`·`open_url`은 `AppError::Forbidden`, 진입은 `launch_with_platform` |
| 클립보드 | arboard 실제 클립보드 읽기·쓰기 없음 | `HostBridge::connect_with_clipboard`·`connect_with_clipboard_ports`(`host.rs`)에 가짜 `ClipboardWriter`(`Arc<dyn Fn(&str) -> AppResult<()> + Send + Sync>`)·`ClipboardReader`를 넣음. UI 검사는 실행 대신 `ShellIntent::CopyText(..)` 생성만 단언(git lane worktree의 `/Users/hyunseokbyun/development/TAIDE-lanes/git/docs/quality-assurance/2026-11-02-native-lane-git-g2-changes.md` 81줄) |
| Keychain | AI 토큰·GitHub PAT·원격 비밀번호를 실제 키체인에 쓰지 않음 | `Overrides.secret_store`에 `taide_infra::secret::test_support::InMemorySecretStore`(`crates/taide-infra/src/secret.rs` 153줄, `cfg(test)` 또는 feature `test-support`) |
| 휴지통 | 실제 휴지통으로 옮기는 검사 실행 금지 | 실행 금지 검사 3개: `remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다`(`src/remote-files-tests.rs` 393), `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다`(`tests/explorer-delete.rs` 401), `실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다`(`tests/lsp-workspace-worker.rs` 387). Git의 추적 안 된 파일 discard도 실검사 금지(추적 파일로만 검사) |
| 네트워크 | LSP 서버 다운로드, gist, 모델 목록, 실제 원격 push/pull 없음 | loopback 모의 서버 `tokio::net::TcpListener::bind("127.0.0.1:0")`(`src/editor-schema-transport-tests.rs` 293), LSP는 `native-lsp-mock` example 실행 파일, Git 원격은 임시 로컬 bare 저장소 |
| Git 사용자 설정 | 사용자 전역 설정을 바꾸지 않음 | 임시 저장소 + `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, `GIT_TERMINAL_PROMPT=0`, local config(`core.hooksPath`, `commit.gpgSign=false`, `credential.helper=""`, `protocol.allow=never` 등)(`src/remote-git-tests.rs` 150-200). git CLI가 사용자 전역 설정을 읽기 전용으로 참조하는 수준은 허용(결정 2026-10-28) |
| 합성 OS 입력 | CGEvent·AppleScript 키 입력·마우스 클릭 없음(사용자 다른 창으로 갈 수 있음) | 프로세스 안 egui 이벤트 주입은 허용: `step(application, &context, vec![egui::Event::PointerButton { .. }])`, `egui::Event::Text(..)`, `egui::Event::Key { .. }` |
| 실행 중인 사용자 TAIDE 앱 | `taide <인자>` CLI 실행 금지(인자를 파일 경로로 바꿔 실행 중 앱에 열기 요청을 보냄) | CLI 동작은 `crates/taide-cli/src/main.rs`를 읽어 파악(`docs/feedback/2026-09-16-taide-cli-probe-opens-file-in-app.md`). native 단일 인스턴스 중계는 격리 data dir과 직접 argv·소켓으로만 검사 |
| 시크릿 파일 | `.env`, `.env.*`, 키 파일, lockfile 토큰 읽기·출력 금지 | 없음. 2026-09-29 IDE lockfile 1개 예외는 그 검사 한정이었음(`docs/acknowledge/2026-09-29-m7-isolated-ide-lockfile-exception.md`) |
| 실기용 번들 | 지우지 않음 | `experiments/native-shell-spike/target/TAIDE M8 Egui Spike.app`, `TAIDE M8 Iced Spike.app`. `cargo clean --profile dev`는 `debug`만 지워 번들이 남음 |
| 저장소 밖 | 작업자는 저장소 밖을 탐색하지 않음 | 예외: cargo registry의 crate 소스 읽기, 메인 저장소 `node_modules` 읽기 |

주의: 메인 병합 게이트 스크립트 `docs/utils/2026-11-03-native-merge-gate.sh`는 앱 crate 전체 테스트를 `--skip` 없이 실행합니다. 위 휴지통 검사 3개가 포함되므로 그대로 실행하면 합성 파일(`delete-synthetic.txt` 등)이 사용자 휴지통으로 이동할 수 있습니다. 2026-10-10 배치 게이트는 세 검사를 `--skip`으로 제외했습니다(`docs/quality-assurance/2026-10-10-native-batch18-workspace-symbols.md` 51줄). 게이트 실행 전 처리 방법은 05 문서 9절에 적었습니다.

화면 캡처(메인만): 작업자는 GUI를 띄우지 않습니다. 메인이 창 캡처가 필요하면 `docs/memory/2026-10-07-native-transition-operations.md` 5절 절차(격리 data dir로 실행, `screencapture -x -o -l <창 번호>`, `kill -TERM`)만 쓰고 합성 입력은 쓰지 않습니다. 화면이 잠겨 있으면 빈 화면이 캡처됩니다.

근거: `docs/memory/2026-10-28-native-lane-operations.md` 4절, `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` 보류 표, `security.md` 1.1절, lane workflow `RULES`.

## 3. 동결 remote-web 규칙

`native/taide-remote-web`(브라우저 Wasm 원격 클라이언트)는 데스크톱 native 대응 완료 전까지 동결입니다(결정 2026-10-06).

1. `native/taide-remote-web`의 소스·`Cargo.toml`·`Cargo.lock`을 바꾸지 않습니다.
2. remote-web이 의존하는 공용 crate(`taide-native-ui`를 `default-features = false`로, `taide-native-editor`, `taide-model`, `taide-remote-wire`)를 바꾸면 컴파일만 유지되는지 확인합니다.

```
cargo check --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline --target-dir <target>
```

3. 그래서 지켜야 할 구조 규칙:
    - 정규식·구문 엔진 의존은 `native/taide-native-syntax`(앱만 의존)에만 둡니다. `taide-native-editor`나 `taide-native-ui`에 정규식 crate를 넣으면 remote-web의 의존 그래프와 lockfile이 바뀝니다(`docs/architecture.md` 9.1절, 결정 2026-10-06 기각 표).
    - UI crate의 새 모듈은 `native-host` feature 없이도 `taide-model`·`egui`만으로 컴파일돼야 합니다(`docs/research/2026-10-28-lane-settings-spec.md` 0절 5번).
    - `taide_native_ui::commands::ShellMutation`의 정의와 경로, remote-web이 import하는 UI 모듈 경로를 바꾸지 않습니다. 옮겨야 하면 옛 경로에 `pub use`를 남깁니다(seams 1.3절). remote-web은 `ShellIntent`·`ShellSurfaces`를 쓰지 않습니다(`rg -n "ShellIntent|ShellSurfaces" native/taide-remote-web/src` 결과 없음).
    - 구조체 literal 소비처를 깨는 필드 추가 대신 새 진입 함수를 둡니다(선례: `EditorAppearance`에 필드를 넣지 않고 `show_presented`·`show_tokenized`·`show_request` 추가, 결정 2026-10-06).
    - 설정 원격 미러는 기존 8 section만 표시하고 새 section은 native 앱에만 그립니다(settings 사양 결정 1 A).
4. 기능표의 remote-web 행은 `frozen`입니다.

## 4. 의존성 규칙

1. cargo는 항상 `--locked --offline`으로 실행합니다. `--offline`은 이미 내려받은 registry 캐시(`~/.cargo/registry`)만 씁니다.
2. 새 외부 crate는 네트워크 내려받기와 lockfile 변경을 부르므로 사용자 허락이 필요합니다(`docs/acknowledge/2026-10-26-native-autonomous-continuation.md` 보류 목록 "허용되지 않은 새 외부 의존성"). 작업자는 필요하면 구현하지 말고 보고합니다(lane workflow `RULES`).
3. 허락을 받은 crate는 정확한 버전으로 고정합니다(`regress = { version = "=0.12.0", features = ["utf16"] }`, `native/taide-native-syntax/Cargo.toml` 14줄). lockfile 변경은 허락받은 crate의 범위로 한정하고, remote-web lockfile은 바뀌면 안 됩니다.
4. 이미 lock에 있는 crate를 앱의 직접 의존으로 올리는 것은 사용자 결정으로 처리한 선례가 있습니다(`winit =0.30.13`, 단계 E 결정 2026-10-27).
5. 새 path 의존성으로 lockfile을 갱신해야 할 때만 `--locked`를 빼고 `--offline`으로 한 번 실행합니다(`docs/HANDOFF.md` 7절).
6. 의존성을 늘리기 전에 표준 라이브러리·기존 의존으로 되는지 확인합니다. 원본 자료(CSS/HTML data 등)는 bun 스크립트로 Rust 소스를 생성해 넣고 MIT 출처를 붙이는 방식이 기본입니다(`docs/research/2026-10-28-lane-editor-preview-spec.md` 3절).
7. 새 자산·문법을 넣으면 `THIRD_PARTY_LICENSES.md`를 갱신합니다. 생성·추출한 JSON 자산은 `.prettierignore`에 넣습니다(루트 `prettier --check .`가 `native/`를 포함함, memory 2026-10-07 6절).
8. vendored eframe 수정은 허용(shell 사양 결정 6 A, 선례 `render_immediate_viewport` drain 수정), epaint vendor 패치는 허용하지 않음(editor-preview 사양 결정 1 A).

지금까지 승인된 의존성 목록은 06 문서 3절에 있습니다.

## 5. Git 규칙

| 항목 | 규칙 |
| --- | --- |
| 브랜치 | native 작업은 `to_rust_native`(upstream `origin/to_rust_native`). lane 브랜치 `lane/<lane>`은 로컬 전용(upstream 없음, `git rev-parse --abbrev-ref lane/git@{u}` 실패로 확인). `main`·`dev`에 직접 커밋하지 않음 |
| 메시지 형식 | Conventional Commits, scope `native`, 설명은 한국어 명사형 종결. 이 브랜치의 실제 type: `feat`, `fix`, `docs`, `refactor`, `build`, `style`, `chore`, 병합 커밋은 `merge(native): <lane> lane 병합` |
| 예시(실제 로그) | `feat(native): 알림 진입점·외부 열기·단일 인스턴스·IDE 동기화와 창·표시 층 잔여 연결`, `docs(native): shell lane 검증 기록과 기능표 갱신`, `merge(native): shell lane 병합`, `docs(native): lane 5개 병합과 게이트 결과 기록`, `build(native): dev 프로필 debuginfo를 줄 정보로 축소` |
| AI 트레일러 | `Co-Authored-By`, `Generated with`, 세션 링크 등 금지. author는 사용자 단독 |
| 스테이징 | `git add -A`, `git add .` 금지. 경로를 명시해 선별 스테이징. 한 번에 한 커밋분만 스테이징 |
| 확인 | 커밋 전 `git status`와 `git diff --cached`로 의도하지 않은 파일이 없는지 확인 |
| 단위 | 독립적으로 되돌릴 수 있는 논리 단위. lane마다 코드+테스트(`feat`) 1개, QA+기능표(`docs`) 1개가 관례 |
| push | 일반 `git push`만. `--force`, `-f`, `--force-with-lease`, `--force-if-includes` 모두 금지 |
| 자동 커밋·푸시 | 저장소 설정 `llm-rules.auto-commit=true`, `llm-rules.auto-push=true`(`git config --get`으로 확인). 검증을 마친 논리 단위는 추가 승인 없이 커밋·푸시. 단 현재 대화에 "커밋하지 마" 같은 지시가 있으면 그것이 우선 |
| 작업자 | 하위 에이전트는 git 상태를 바꾸지 않음(`add`, `commit`, `stash`, `checkout`, `reset`, `restore`, `push`, `worktree` 금지). 커밋·병합·푸시는 메인만 |
| stash | 공유 작업 트리에서 `git stash` 금지(`docs/agent-operations.md` 4절) |
| 시크릿·산출물 | `.env`, 키 파일, `target/`·빌드 산출물을 커밋하지 않음 |

근거: `git.md` 1~6절, `docs/acknowledge/2026-08-06-git-workflow.md`, `docs/acknowledge/2026-09-23-rust-native-transition-contract.md` 2절, `git -C /Users/hyunseokbyun/development/TAIDE log --oneline`.

## 6. 응답 형식 규칙(사용자에게 답할 때)

1. 한국어 존댓말, 간결하게. 이모지·아스키아트(박스 그림) 금지.
2. 첫 줄은 바로 실행할 행동이나 직접적인 답입니다. 서론·계획 예고로 시작하지 않습니다.
3. 두 단계 이상은 번호 목록, 각 단계는 행동 하나입니다.
4. 매 응답에 현재 상태를 한 줄로 적습니다.
5. 끝은 2분 안에 할 수 있는 다음 행동 하나, 또는 확인 가능한 완료 결과입니다. 반복 요약·인사·추가 도움 제안을 붙이지 않습니다.
6. 화면 목록은 관련 항목을 묶어 그룹당 5개 이하로 씁니다.
7. 시간은 관찰 근거가 있을 때만 구체적으로 씁니다. 사용자가 소요 시간을 물으면 단계별 예상 시간 표로 답하고 진행 중인 작업은 멈추지 않습니다(소요 시간 질문은 중단 지시가 아님).
8. 장기 작업의 진행 보고에는 기능표 기준 진척(완료 행/기능 분모, %)과 남은 범위를 함께 적습니다(`docs/feedback/2026-10-28-native-serial-step-inefficiency.md`). 이 비율은 기능표 행 비율이며 배치 31~33의 실기·출시·전환 게이트를 포함한 전체 전환율이 아니라고 함께 밝힙니다(`docs/feedback/2026-10-10-native-progress-and-batch-boundaries.md`).
9. 사용자 결정이 필요하면 현재 작업의 결정만 한 번에 모아, 선택지와 추천안을 먼저 적어 "전부 추천안대로"로 답할 수 있게 묻습니다.
10. 오류는 원인·관찰값·해결(또는 다음 진단)을 담담하게 씁니다. 실행하지 않은 검사를 통과했다고 쓰지 않습니다.

근거: `ai-process.md` 0.1·3.1절, `docs/HANDOFF.md` 5절 7번, 사용자 auto-memory `session-duration-checkpoints`.

## 7. 문서 규칙

모든 작업의 입력·상태·결과는 `docs/` 아래 문서에 남깁니다. 파일 이름은 `YYYY-MM-DD-<주제>.md`(kebab-case)이고 문서는 한국어입니다.

| 위치 | 용도 | native 작업에서의 실제 쓰임 |
| --- | --- | --- |
| `docs/PROCESS.md` | 현재·누적 작업 체크리스트 | 맨 위 "진행: 배치 24" 절의 W0~W4 항목. 4150줄로 커서 처음 80줄만 읽음. 아카이브 규칙(300줄 초과 시 오래된 완료 절을 `docs/history/`로)은 아직 적용되지 않은 상태 |
| `docs/quality-assurance/` | 검증 기록, 기능표 | lane 묶음마다 `YYYY-MM-DD-native-lane-<lane>-<묶음키>.md`, 기능표 `2026-10-09-native-function-matrix.md`, 배치 계획 `2026-10-10-native-remaining-batch-plan.md`. 테스트 부채(재현 조건·생략 이유·남은 위험·실행 시점)도 여기 |
| `docs/acknowledge/` | 사용자 결정·합의 | 사용자가 답한 결정만 기록. 메인이 정한 계획을 사용자 결정으로 적지 않음(`docs/feedback/2026-10-10-native-progress-and-batch-boundaries.md`) |
| `docs/feedback/` | 지적받은 내용 | 1건 1파일, "무엇을 지적받았나 / 왜 틀렸나 / 어떻게 고치나 / 언제 적용하나" |
| `docs/memory/` | 항상 적용하는 운영 지식 | `2026-10-28-native-lane-operations.md`(현행 운영), `2026-10-07-native-transition-operations.md`(5·6절 유효) |
| `docs/research/` | 조사·사양 | lane 사양 7개 `2026-10-28-lane-<lane>-spec.md`, 영역 경계 `2026-10-28-native-lane-seams.md` |
| `docs/utils/` | 작업에 쓴 스크립트 | lane workflow `2026-11-02-native-lane-workflow.js`, 병합 게이트 `2026-11-03-native-merge-gate.sh`, oracle 스크립트 |
| `docs/history/`, `docs/bug/` | 시간순 이력, 버그(증상·원인·해결) | 필요할 때 |
| `docs/handoff/native-completion/` | 이 인수인계 묶음 | 이 문서 |

추가 규칙:

- 프로젝트 상태는 문서보다 실제 파일이 우선입니다. 문서와 파일이 다르면 파일을 믿고 문서를 고칩니다.
- 과거 QA 문서의 "통과" 기록을 현재 상태로 쓰지 않습니다. 인수 시점에 실제로 실패하던 사례가 있었습니다(memory 2026-10-07 6절).
- 문서 머리에 작성일·근거(lane, 사양, 결정 문서)를 적습니다(예: `docs/quality-assurance/2026-11-02-native-lane-shell-providers.md` 1-3줄).
- 기능표 상단 합계 표는 lane 작업자가 고치지 않습니다. 갱신 방법은 05 문서 10절입니다.
