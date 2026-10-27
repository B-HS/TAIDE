# lane shell 사양 (Rust-native 전환, wave 1)

> 작성일: 2026-10-28 · 대상 lane: `shell` · 운영 기준: `docs/memory/2026-10-28-native-lane-operations.md`
> 이 문서만 읽고 구현을 시작할 수 있도록 ID마다 원본 TS 동작, native 현재 자산, 빠진 것, 설계, 자동 검사, 완료 기준을 적었습니다. 원본 TS는 다시 조사하지 않아도 되고, 구현 중에는 여기 적힌 native 심볼 주변만 읽으면 됩니다.

## 0. 읽는 법과 전제

- 경로는 저장소 기준 상대 경로입니다. lane worktree(`/Users/hyunseokbyun/development/TAIDE-lanes/shell`)에서도 같습니다.
- 구조 이동 중인 파일(`native/taide-native-app/src/application.rs`, `host.rs`, `native/taide-native-ui/src/commands.rs`, `shell.rs`)은 줄 번호 없이 심볼 이름으로만 적었습니다.
- 작성 시점의 실제 영역 이름(파일에서 확인)
    - 앱 영역 모듈: `native/taide-native-app/src/domains/shell.rs`(`application.rs`가 `#[path = "domains/mod.rs"] pub mod domains;`로 선언, 경로 `crate::application::domains::shell`). 지금 `shell`과 `commands` 두 영역만 옮겨졌습니다.
    - host 명령: `HostCommand::Shell(domains::shell::Command::…)`, 응답 `HostReply::Shell(domains::shell::Reply::…)`, 처리 `domains::shell::dispatch(worker, command)`(host의 직렬 worker에서 호출)
    - 응답 반영: `NativeApplication::apply_shell_reply`, intent 처리 `apply_shell_intent`, 영역 상태 `NativeApplication.shell_state: domains::shell::State`(`app_menu`, `auxiliary_windows`, `window_state`, `file_drop_colors`, `overlay_viewport` 등)
    - 셸 영역 helper(이미 있음): `close_project`, `is_any_window_focused`, `focused_viewport`, `overlay_viewport`, `show_auxiliary_windows`, `close_auxiliary_window`, `run_app_menu_action`
    - UI intent: `taide_native_ui::commands::ShellIntent`에 영역 하위 enum이 생기는 중입니다(지금 `ShellIntent::Commands(intents::commands::Intent)`만 있음). 셸 intent를 새로 만들면 `native/taide-native-ui/src/intents/shell.rs`에 둡니다.
    - `docs/research/2026-10-28-native-lane-seams.md`는 아직 없습니다. 생기면 아래 이름을 그 문서의 이름으로 바꿔 읽습니다.
- 새 앱 모듈은 기존 관례대로 kebab 파일 + `#[path]`로 `native/taide-native-app/src/lib.rs` 모듈 목록 끝(영역 블록)에 추가합니다.
- 원본 줄 번호는 이 문서를 쓰며 직접 확인한 것입니다. QA 근거는 파일명으로 적었습니다(모두 `docs/quality-assurance/` 아래).
- 기준: 원본 TS 화면·상태·상호작용과 같게 만들고 새 기능·디자인은 넣지 않습니다. 원본 결함은 재현하지 않고, 그렇게 한 곳은 근거를 적었습니다.
- 상태 정의는 기능 대응표(`2026-10-09-native-function-matrix.md`) 머리말을 따릅니다. complete는 구현 + 실제 앱 경로 + 해당 동작의 자동 검사이고, 픽셀·실제 OS 입력·실기(배치 31)는 포함하지 않습니다. 실기만 남은 행은 complete로 올리고 비고에 실기 항목을 남깁니다.

## 1. 요약

### 1.1 수와 상태

- 대상 15개: shell-3, shell-5, shell-9, shell-21, shell-22, shell-37, shell-59, shell-78, shell-90, shell-91, shell-92, shell-93, shell-94, shell-96, shell-98
- 대응표 현재 상태: partial 13, unwired 1(shell-90), missing 1(shell-96)
    - shell-96의 missing은 오래된 공통 문구입니다. 같은 기능인 editor-80(Emmet)은 complete이고 설정 토글까지 실제 App 검사가 있습니다. 이 lane은 보조 창 검사 1건과 재판정만 합니다.
    - shell-90~93의 비고는 공통 문구라 실제 남은 범위가 적혀 있지 않습니다. 2절에 실제 남은 범위를 적었습니다.
- 이 lane(wave 1)에서 끝낼 수 있는 ID: 11개(묶음 1·2). shell-91은 4절 결정 1의 결과에 따릅니다.
- 다른 lane이 끝난 뒤에만 남은 범위를 판정할 ID: shell-21, shell-59, shell-78 전체와 shell-5·shell-92·shell-93의 일부(묶음 3). 이 범위의 구현은 대부분 git·explorer-search lane 사양에 이미 들어 있어 shell 쪽은 연결 확인과 행 판정만 남습니다.

### 1.2 ID별 분류

| ID | 기능 | 표 상태 | 이 lane에서 할 일 | 다른 lane·외부 의존 | 묶음 |
| --- | --- | --- | --- | --- | --- |
| shell-93 | IPC 동기화 | partial | provider 이벤트 수신함, `SyncStateChanged` 표시 재로드, 앱 밖 경로 `ProjectClosed` 정리 | Git 이벤트 소비는 git lane(git-64) | 1(최종 판정 3) |
| shell-37 | 프로젝트 닫기 | partial | 닫힌 프로젝트의 tree·explorer 캐시 제거, 닫기 직전 최신 mirror, peek 문서 소유자 재해석 | explorer-search lane D6(탐색기 상태 키 변경)과 병합 조정 | 1 |
| shell-90 | 외부 열기(CLI, `--wait`) | unwired | 실행 인자, 단일 인스턴스 중계, 대기열 drain, wait marker 등록·해제, 안내 toast | 실제 `taide` CLI → native 연결은 shell-15 허락 | 1 |
| shell-91 | OS 알림 | partial | agent·명령·LSP 알림 생산, 포커스 게이트, 첫 전달 안내, 다른 lane용 알림 진입점 | terminal-agent-59~63, settings-23과 중복(결정 1) | 1 |
| shell-92 | IDE 동기화 | partial | save 요청 처리, 진단 push | diff 요청 → ClaudeDiff 탭은 git lane(git-49) | 1, 3 |
| shell-96 | Emmet provider | missing | 보조 창 검사 1건 + 재판정 | editor-80 근거 재사용 | 1 |
| shell-94 | 테마·로케일 오류 배너 | partial | 시작 시 로드 실패 대체(메인 결정), 배너를 drop overlay 위로 | 없음 | 2 |
| shell-22 | OS 파일 drop | partial | overlay·배너 층 순서(shell-94와 함께), 재판정 | 실기(배치 31) | 2 |
| shell-9 | 네이티브 앱 메뉴 | partial | 부팅 직후 `Open Recent` 초기값 | 실기 | 2 |
| shell-3 | 창 위치·크기 복원 | partial | macOS zoom 기록, 창 생성 뒤 치명 오류의 종료 코드 | 실기, 로거는 결정 3 | 2 |
| shell-98 | provider 계층 | partial | 보조 창 자체 toast | 실기 | 2 |
| shell-5 | 보조 창 셸 | partial | 보조 창 WKWebView 배치, 터미널 raw input, theme preview, 보조 창 탭 drag 중 메인 web view 숨김 | 보조 창 search·git 보기와 탐색기 상태 창별 분리는 explorer-search·git lane | 2, 3 |
| shell-21 | 전역 키 `git` | partial | 없음(git lane git-1이 `Run::ShowGit` 구현) | git lane | 3 |
| shell-59 | explorer 4보기 | partial | 없음 | explorer-search lane(검색 본문·⌘⇧F/⌘⇧H), git lane(Git 본문·⌃⇧G) | 3 |
| shell-78 | 탭 메뉴 | partial | 없음 | git lane(git-48 변경 열기, git-55 파일 이력) | 3 |

### 1.3 다른 lane 의존과 순서

#### 먼저 필요한 것

- wave 0 구조 이동: `domains/shell.rs`는 생겼습니다. 이 lane의 새 코드는 `domains/shell.rs`와 새 모듈에 넣고, 공용 파일(`application.rs`, `lib.rs` 모듈 목록, `host.rs`의 enum 루트)은 영역 블록 끝에 추가만 합니다.

#### 다른 lane 사양과 겹치는 지점(메인 조정 필요)

이미 작성된 다른 lane 사양(`2026-10-28-lane-{git,explorer-search,settings}-spec.md`)과 같은 코드를 만들 위험이 있는 곳입니다. 4절 결정 1이 이 표의 소유를 정합니다.

| 겹치는 것 | 이 문서 | 다른 사양의 계획 | 추천 처리 |
| --- | --- | --- | --- |
| OS 알림 생산자(agent·명령·LSP)와 첫 전달 안내 | shell-91(묶음 1, wave 1) | settings-23(settings lane S2, wave 2)이 `crates/taide-notification/src/producer.rs` + `domains::settings` 알림 상태로 같은 것을 계획. terminal-agent-59~63도 같은 행 | shell lane이 wave 1에 구현하고 settings S2는 검증·행 갱신만(결정 1 A) |
| 다른 lane이 부를 알림 진입점 | `NativeApplication::notify_event`(2.5) | explorer-search 사양 1.3은 "terminal-agent lane의 전송 포트"를 기다림. settings 사양은 `domains::settings::notify` | 진입점을 shell lane 하나로 정하고 두 사양의 의존 대상을 바꿈 |
| 테스트용 플랫폼 주입(`services.platform` 대체) | 2.5의 생성자 override | settings 사양 2.5가 `NativeApplication::new_with_overrides(.., Overrides { secret_store, settings_ports })` 계획 | shell lane이 `new_with_overrides`와 `Overrides { secret_store, platform }`을 먼저 만들고, settings lane은 필드만 추가 |
| `presentation_refresh::Changes::record`의 `SyncStateChanged` 조건 | shell-93(묶음 1) | settings S4(settings-46)가 같은 한 줄 계획 | shell lane이 wave 1에 넣고 settings S4는 확인만 |
| `PaintSink` 이벤트 기록 | `provider_events::Inbox` 필드 | settings 사양 2.7이 `domains::settings::Changes` 필드로 `AgentStateChanged`·`TerminalCommandFinished`·`LspInstallProgress`·`ProjectClosed`도 복제 | 결정 1 A면 settings `Changes`는 알림 이벤트를 빼고 `SyncStateChanged`·`RemoteStateChanged`만 기록 |
| 탐색기 상태 키 | shell-37이 닫힌 프로젝트의 `explorers` 항목 제거 | explorer-search D6이 `explorers`를 mount(`ShellSlotId`, 보조 창 mount) 키로 바꿈 | 둘 다 wave 1이라 병합 때 메인이 "그 프로젝트의 mount 전부 제거"로 맞춤. explorer-search가 `forget_project(project)` 같은 함수를 열면 shell은 그것을 부름 |
| 보조 창 탐색기 상태 창별 분리, 보조 창 Search 본문 | shell-5 남은 범위 | explorer-search-30·D6(wave 1) | explorer-search lane 소관. shell-5는 그 병합 뒤 판정 |
| IDE diff 요청 소비 | shell-92 남은 범위 | git-49(terminal-agent-68·69 정정 포함) | git lane 소관. shell-92는 그 병합 뒤 판정 |
| IDE save·진단 push | shell-92(묶음 1) | 대응표 terminal-agent-70(save)·73(진단)이 같은 기능. terminal-agent 사양은 아직 없음 | shell lane이 구현하고 terminal-agent lane 범위에서 제외(결정 1 A) |
| 외부 열기 | shell-90(묶음 1) | terminal-agent-57이 같은 기능 | 같음 |

#### 같은 wave의 파일 조율

- terminal-agent lane: 보조 창 터미널 raw input(shell-5)이 `terminal_surface.rs`의 raw input 상태를 공유 가능한 형태로 나눕니다. 그 lane이 같은 구조를 바꾸지 않도록 알립니다.
- editor+preview lane: 보조 창 WKWebView 배치(shell-5)가 `preview_web_view.rs`를 viewport별로 바꿉니다. IDE 진단 push(shell-92)는 `diagnostics.rs`를 읽기만 합니다.

#### 나중에 이어야 하는 것(묶음 3)

- git lane(wave 2): Git 패널 본체와 ⌃⇧G(git-1), 탭 메뉴 변경 열기(git-48)·파일 이력(git-55), ClaudeDiff 탭(git-49), Git 이벤트 소비(git-64)
- explorer-search lane(wave 1): 검색 패널 본체와 ⌘⇧F/⌘⇧H(31·37), 보조 창 탐색기 상태·검색 본문(30, D6)
- commands-ui lane(wave 3): 운영 문서 3절의 "앞 wave에서 진입만 남은 행"을 잇습니다. 묶음 3은 그 lane에 넘기는 판정 목록입니다.

## 2. ID별 사양

### 2.0 공통 설계(묶음 1·2가 함께 씀)

#### 2.0.1 provider 이벤트 수신함

- 원본 provider는 Tauri 이벤트를 직접 구독합니다. native에서는 모든 `AppEvent`(`crates/taide-model/src/app_event.rs`)가 `application.rs`의 `PaintSink::publish`를 지납니다. 지금 기록하는 것은 `events::TreeChanges`, `command_palette::FileIndexChanges`, `presentation_refresh::Changes`, `title_branch::Changes`, `app_menu::Changes`뿐이고, 그 뒤 `request_repaint`합니다.
- 새 모듈 `native/taide-native-app/src/provider-events.rs`(`crate::provider_events`)
    - `Inbox`: `Mutex` 안의 종류별 대기열과 플래그. `PaintSink`에 `providers: Arc<provider_events::Inbox>` 필드를 추가하고 `publish`에서 `record(&event)`를 부릅니다. 생성자 `new_with_secret_store`의 다른 `Changes`와 같은 방식으로 만들어 앱 필드에도 보관합니다.
    - 기록 대상과 보관 규칙

| AppEvent | 쓰는 ID | 보관 |
| --- | --- | --- |
| `AgentStateChanged { project_id, agents }` | shell-91 | 프로젝트마다 마지막 것만(로스터 snapshot이라 덮어써도 됨). 단, 같은 tick 안에 working → idle이 연달아 오면 앞 것을 잃으므로 순서 대기열로 두고 상한을 적용합니다 |
| `TerminalCommandFinished { session_id, cwd, exit_code, duration_ms }` | shell-91 | 순서 대기열, 상한 `MAX_PROVIDER_EVENTS`(예: 256). 넘치면 가장 오래된 것을 버리고 `log::warn` |
| `LspInstallProgress { server_id, phase, message, .. }` | shell-91 | `Done`/`Failed`만, 상한 같음 |
| `AgentExternalOpen { .. }` | shell-90 | 플래그만(drain이 `AgentStore` 대기열을 직접 읽음) |
| `IdeSaveRequested { request_id, project_id, path }` | shell-92 | 상한 같음. 버려진 요청은 `ide-tools.rs` `save_document`가 `IDE_SAVE_TIMEOUT`(5초) 뒤 `false`로 끝냅니다(지금 소비자 없음과 같은 결과) |
| `ProjectClosed { project_id }` | shell-93, shell-37, shell-91 | 집합 |
| `SyncStateChanged { .. }` | shell-93 | 수신함이 아니라 `presentation_refresh::Changes`에서 처리(2.1) |

- 처리 위치: `domains/shell.rs`에 `NativeApplication::consume_provider_events(&mut self, context, now: Instant)`를 두고, `background_tick`에서 host 응답 처리(`self.poll(context)`) 바로 뒤에 부릅니다. host 응답을 먼저 처리해야 `Reply::ProjectClosed` 앞의 저장·mirror 응답 순서가 지켜집니다. `now`를 인자로 받아 검사가 시각을 정할 수 있게 합니다.
- 종료 중(`closing.is_some()`, `is_exit_ready`, `services.state.is_shutting_down()`)에는 소비하지 않고 비웁니다.

#### 2.0.2 새 host 명령(`domains::shell::Command` 끝에 추가)

| 명령 | 처리 | 응답(`domains::shell::Reply` 끝에 추가) |
| --- | --- | --- |
| `ProcessExternalOpens` | shell-90 drain(2.4) | `ExternalOpensProcessed(Vec<external_open::Outcome>)` |
| `Notify(native_notifications::Request)` | shell-91 게이트·전송(`tasks.run_blocking_result`) | `NotificationSent(AppResult<NotificationDelivery>)` |
| `ResolveIdeSave { request_id, saved }` | `taide_runtime::ide_actions::ide_resolve_save(&services.ide, request_id, saved)` | 없음(오류는 `log::warn`) |
| `PublishIdeDiagnostics { project, items }` | `ide_actions::ide_publish_diagnostics(&services.ide, project, items)` | 없음 |

- 기존 명령 재사용: 저장(`HostCommand::Save { path, snapshot }`, editor 영역 이동 뒤 이름은 seams 문서), `HostCommand::CloseTab`(marker 해제 훅), `HostCommand::SetDirty`.

#### 2.0.3 테스트용 생성자 override

- 지금 App 검사가 바꿀 수 있는 것은 비밀 저장소뿐입니다(`new_with_secret_store`가 `Arc::get_mut(&mut assembly.services)`로 `secrets` 교체). `bootstrap::NativePlatform::send_notification`은 실제 `/usr/bin/osascript`를 실행하므로 알림 검사에 그대로 쓰면 안 됩니다.
- `NativeApplication::new_with_overrides(context, runtime, state, tasks, helper, warnings, Overrides { secret_store, platform: Option<Arc<dyn PlatformServices>> })`를 추가하고 `new_with_secret_store`는 이것으로 위임합니다. `platform`이 있으면 같은 `Arc::get_mut` 지점에서 `services.platform = PlatformServicesState(platform)`로 바꿉니다. settings 사양 2.5의 `Overrides`와 같은 구조체이므로 settings lane은 필드만 추가합니다(1.3 표).

#### 2.0.4 검사 공통

- 실제 App 검사는 `application-project-rail-tests.rs`의 `Fixture`, `isolated_state`, `insert_project`, `project_ref`, `launch`와 각 검사 파일의 frame helper(`application-file-drop-tests.rs`의 `run`/`frame_until`, `application-explorer-views-tests.rs`의 `Clock`)를 씁니다. 보조 창 검사는 `application-auxiliary-window-tests.rs`의 `install_viewports`·`Viewports::texts`·`step_until`을 재사용합니다.
- 명령(모두 `CARGO_TARGET_DIR=<worktree>/target-lane`)
    - 앱 lib 필터: `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- <필터> --test-threads=4`
    - UI 하네스: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features inspection --test workbench -- <필터>` 또는 `--lib`
    - 동결 remote-web: `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline`. vendored eframe을 바꾼 묶음은 `--features canvas`로도 확인합니다(eframe은 `canvas` 기능에서만 들어옴, `native/taide-remote-web/Cargo.toml` 11, 33).
    - 형식: 바꾼 crate의 `cargo fmt --manifest-path <crate>/Cargo.toml -- --check`
- 금지: 실제 OS 알림 전송(osascript), Finder/클립보드/휴지통, GUI 실행, 합성 OS 입력. 알림 검사는 2.0.3의 가짜 플랫폼을 씁니다.
- 검사 이름은 기존처럼 한국어로 씁니다.

---

### 2.1 shell-93 IPC 동기화

#### 원본 동작

- `src/app/providers/ipc-sync-provider.tsx` 290-618의 `IpcSyncProvider`는 주 창(`src/app/app.tsx` 94)과 보조 창(69)에 모두 mount됩니다. 이벤트별 처리와 native 대응은 아래와 같습니다.

| 이벤트(원본 줄) | 원본 처리 | native 대응 | 판정 |
| --- | --- | --- | --- |
| `projectListChanged`(305-307) | 프로젝트 목록·최근 재조회 | 모든 이벤트가 `NativeShellEvents::publish` → `ShellSnapshot::read` 재생성. Welcome 최근 목록은 snapshot 세대마다 재조회(`welcome.rs` `State::tick`), 앱 메뉴는 `app_menu::Changes` | 충족 |
| `projectOpened`(309-311) | `PROJECT.DETAIL` 재조회 | snapshot | 충족 |
| `projectClosed`(313-331) | open-with override 정리, 프로젝트 범위 query 제거(`PROJECT_SCOPED_KEYS`, `src/shared/constants/query-key.ts` 195-208), root 아래 파일 query 제거, layout revision 기억 제거, LSP 세션 flush | open-with는 `open_with.rs`가 닫힘을 처리. 문서 해제는 `Reply::ProjectClosed` → `release_project_documents`뿐 | 빠짐: 앱 밖 경로의 닫힘(remote `project_close`, `remote-projects.rs`)은 문서·캐시를 해제하지 않음 |
| `projectActivated`·`projectGroupsChanged`·`sessionShellSlotsChanged`·`sessionWindowChromeChanged`(333-360) | 재조회 | snapshot | 충족 |
| `layoutChanged`(362-383) | 같은 창 echo 생략, 보조 창이 사라지면 `FILE.MIRRORS` 재조회 | snapshot의 layouts 재읽기. native 문서 store는 모든 창이 공유해 창별 mirror 목록 캐시가 없음 | 해당 없음 |
| `themeChanged`(385-387) | `THEME.ALL` 재조회 | `presentation_refresh::Changes::record` | 충족 |
| `gitStatusChanged`·`gitRefsChanged`(399-411) | Git 캐시 무효화 | 제목줄 branch만 `title_branch::Changes` | git lane(git-64) |
| `settingsChanged`(413-428) | theme·locale·remote 상태 재조회, `SETTINGS.CURRENT`, settings.json AppFile 탭 재조회 | `presentation_refresh::Changes`, `status_editor`, `presentation`, `app_file_views::observe_settings_revision` | 충족(remote 상태 표면은 settings lane) |
| `terminalExited`·`terminalSpawned`(446-478) | 창별 세션 목록 캐시 | 터미널 hub 세션 metadata가 단일 출처 | 해당 없음 |
| `remoteStateChanged`(480) | remote 상태 캐시 | settings lane 표면 | settings lane |
| `syncStateChanged`(482-487) | `SYNC.STATUS` 설정, `SETTINGS.CURRENT`·`THEME.ALL`·`LOCALE.ALL` 무효화 | `presentation_refresh::Changes::record`가 이 이벤트를 세지 않음(`presentation-refresh.rs` `Changes::record`) | 빠짐(아래) |
| `fsChanged`(489-597) | 파일 내용·raw, Git status/gutter/diff, 빠른 열기 색인, tree | `events::TreeChanges`, `command_palette::FileIndexChanges`, 편집기 파일 관찰 | 충족(Git 제외) |
| `fsRescanRequired`(599-617) | 색인·Git·파일 무효화, root + 펼친 디렉터리 재읽기 | `TreeChanges::merge(None)` → host tree 재읽기 | 충족 |
| `useLspSessionsQueryInvalidationSync`(294) | LSP 세션 상태 | editor 영역 LSP 상태 표면 | editor lane |
| `useRecentProjectsClearedNotice`(295, `src/entities/project/project.query.ts` 92-96) | `skippedWithDrafts > 0`이면 `project.clearRecentKeptDrafts` info toast | `app_menu::Changes`의 kept drafts → toast | 충족 |
| `subscribeRemoteConnection`(297-303) | remote mirror 재연결 시 전체 무효화 | remote mirror 전용 | 해당 없음 |

- `SyncStateChanged`가 왜 필요한가: `crates/taide-runtime/src/sync_actions.rs` 280-338 `apply_sync_download`는 `apply_settings`(native는 `settings_actions::apply_and_broadcast`, `SettingsChanged` 발행)를 먼저 하고, 그 뒤에 `apply_theme_entries`·`apply_locale_entries`로 테마·로케일 파일을 쓰고, 마지막에 `SyncStateChanged`를 냅니다. 그래서 `SettingsChanged` 때의 재로드는 옛 파일을 읽을 수 있고, 현재 테마·로케일 파일만 바뀐 경우는 다시 읽을 계기가 없습니다.

#### native 현재 자산

- `PaintSink`, `events::TreeChanges`, `presentation_refresh::Changes`(`record`·`revision`·`theme_revision`·`settings_revision`), `presentation_refresh::Refresh`(`next`가 입력·revision이 같으면 재요청하지 않음), `app_menu::Changes`, `title_branch::Changes`, `NativeApplication::release_project_documents`

#### 빠진 것

1. `SyncStateChanged` 뒤 테마·로케일 재로드
2. 앱이 보내지 않은 `ProjectClosed`(remote 등) 뒤의 문서·프로젝트 캐시 정리
3. shell-90·91·92가 쓸 provider 이벤트 수신함(2.0.1)
4. Git 이벤트 소비(git lane)

#### 구현 설계

- `presentation_refresh::Changes::record`: `AppEvent::SyncStateChanged { .. }`에서 `revision`과 `themes`를 1 올립니다. `settings`(AppFile 탭 재조회용)는 올리지 않습니다. 설정 적용 자체가 `SettingsChanged`를 내므로 원본의 `SETTINGS.CURRENT` 무효화는 이미 덮입니다. 로케일 목록(`LOCALE.ALL`) 카운터는 표면이 생기는 settings lane이 필요하면 추가합니다.
- `ProjectClosed`: 수신함에 기록하고, `consume_provider_events`가 `services.state.projects`에 없는 프로젝트마다 `forget_closed_project(project)`(2.2)를 부릅니다. `Reply::ProjectClosed` 처리도 같은 함수를 부르도록 바꿉니다. 이 함수는 여러 번 불려도 결과가 같아야 합니다(rail 닫기는 응답과 이벤트 두 번 옵니다).

#### 자동 검사

- crate 단위(`presentation-refresh.rs` tests): `SyncStateChanged` 기록 뒤 `revision()`·`theme_revision()`이 오르고 `settings_revision()`은 그대로입니다.
- 실제 App
    - 현재 사용자 테마 파일(`themes_dir()/<id>.json`)의 `app.background`를 바꾸고 `services.events.publish(AppEvent::SyncStateChanged { status })` → frame 반복 → 셸 배경색이 새 값입니다.
    - 프로젝트를 열고 dirty untitled를 만든 뒤 host 명령을 거치지 않고 `project_actions::project_close(..)`를 직접 호출(remote 경로와 같음) → frame 반복 → store에 그 문서가 없고 `drafts()`에도 없으며 `trees`·`explorers`에 그 프로젝트 항목이 없습니다.

#### 완료 판정

- 빠진 것 1~3과 위 검사가 통과하면 비고를 갱신합니다. 상태는 Git 이벤트 소비(git-64)가 검증될 때까지 partial로 두고 묶음 3에서 최종 판정합니다.

---

### 2.2 shell-37 프로젝트 닫기

#### 원본 동작

- rail 메뉴 `project.close`(`src/widgets/app-sidebar/sortable-project-icon.tsx` 170) → `useCloseProject`(`src/entities/project/project.query.ts` 98-104) → runtime `project_close`(`crates/taide-runtime/src/project_actions.rs` 462-500)
    - `await_project_flush` → 세션·프로젝트 목록에서 제거 → `detach_all`(native `projects.rs`: layout, watcher, git watcher, 터미널, git store, tree store 제거) → `ProjectClosed`·`ProjectActivated`·slot·목록 이벤트
- 닫기 전 flush는 `{ project }` 범위입니다(`src/app/providers/hot-exit-flush-provider.tsx` 46-48, 60). 그 프로젝트의 최신 본문을 mirror에 씁니다.
- 닫힘 뒤 정리는 shell-93 표의 `projectClosed`입니다. `TREE.ROWS`도 프로젝트 범위 query라 지워지고, explorer의 선택·inline rename·create 입력은 패널 unmount로 사라집니다. 다시 열면 새 tree와 빈 선택으로 시작합니다.
- 실패는 mutation 오류 toast입니다.

#### native 현재 자산

- `NativeApplication::close_project`(domains/shell.rs: 문서마다 `persistence.mirror_job` → `HostCommand::MirrorDraft` → `Command::CloseProject`), host `CloseProject` → `Reply::ProjectClosed` → `release_project_documents`(남은 탭이 보는 문서는 소유자를 `root_guard::resolve_owning_project`로 다시 정함), `shutdown`의 닫힌 프로젝트 draft 건너뛰기. 근거 `2026-10-26-native-project-rail-menu.md`.

#### 빠진 것(같은 QA의 "과대 판정"·테스트 부채 1, 3)

1. 닫힌 프로젝트의 앱 캐시(`trees`, `explorers`, `tree_loading`, 그 프로젝트의 `symbol_sidebars` 항목, 그 프로젝트를 가리키는 `pending_entry_delete`)가 남아 다시 열 때 이전 행·선택이 보일 수 있습니다.
2. mirror가 전송 중일 때의 추가 수정은 닫기 전에 쓰이지 않습니다(`Persistence::mirror_job`이 in-flight 중 새 job을 만들지 않음).
3. 닫힌 프로젝트 소속으로 남은 dirty peek 문서(`peek_models`)의 debounce mirror가 닫힌 프로젝트로 갑니다.

- 테스트 부채 4(다른 프로젝트 탭이 보는 문서의 소유자 재해석)는 이번 범위가 아닙니다. 프로젝트 사이 탭 이동·중첩 프로젝트 열기 경로가 생길 때 다룹니다. QA에 그대로 남깁니다.

#### 구현 설계

- `domains/shell.rs`의 `NativeApplication::forget_closed_project(&mut self, project: &ProjectId)`
    1. `release_project_documents(project)`(기존)
    2. 프로젝트 단위 앱 캐시에서 그 프로젝트를 지웁니다. 최소 `trees`, `explorers`, `tree_loading`, `symbol_sidebars`의 그 프로젝트 항목, `pending_entry_delete`(요청 프로젝트가 같을 때). 구현 때 `rg "ProjectId" native/taide-native-app/src/application.rs`로 `HashMap<ProjectId, _>`·`HashSet<ProjectId>` 필드를 전수 확인하고, 지우지 않은 필드는 이유를 QA에 적습니다. explorer-search D6 병합 뒤에는 그 프로젝트의 mount 전부를 지웁니다(1.3 표).
    3. peek 문서: `peek_models`에서 소유 프로젝트가 닫힌 프로젝트인 문서를 `taide_infra::root_guard::resolve_owning_project(&services.state.projects.read(), path)`로 다시 정합니다(없으면 `None` → mirror 대상 아님). `release_project_documents`의 남은 탭 분기와 같은 방식입니다.
- 닫기 직전 최신 mirror: `persistence.rs`에 `closing_mirror_job(snapshot, project)`를 추가합니다. in-flight가 있어도 그 문서의 현재 revision이 in-flight revision보다 크면 job을 돌려줍니다. `close_project`는 `mirror_job` 대신 이 함수를 씁니다. host 큐가 직렬이라 이 job은 in-flight 뒤, `CloseProject` 앞에 처리됩니다. 문서는 `ProjectClosed` 때 해제되므로 늦게 온 `DraftMirrored` 응답이 해제된 문서를 가리켜도 무시되는지 확인합니다(필요하면 그 분기만 고칩니다).

#### 자동 검사

- crate 단위(`persistence` tests): in-flight 중 더 새 revision이면 `closing_mirror_job`이 job을 주고, 같거나 낮으면 `None`입니다.
- 실제 App(`application-project-rail-tests.rs`에 추가)
    - 프로젝트를 열고 디렉터리를 펼쳐 파일을 선택 → rail 메뉴 닫기 → 같은 root를 `Command::OpenProjectByPath`로 다시 열기 → 첫 tree 페이지가 다시 조회되고(펼침 없음) 선택이 없습니다.
    - 문서를 고친 뒤 `persistence.mirror_started`로 in-flight를 만든 상태에서 더 고치고 닫기 → 디스크 mirror(`taide_file::service`의 mirror 읽기)가 마지막 본문입니다.
    - 다른 프로젝트 파일을 peek로 열어 고친 상태에서 그 프로젝트를 닫으면 peek 문서의 소유자가 `None`이 되고 닫힌 프로젝트로 mirror 명령이 나가지 않습니다. 구성이 어려우면 소유자 재해석 helper의 단위 검사로 대신하고 QA에 이유를 적습니다.

#### 완료 판정

- 위 검사와 기존 rail·exit 검사 통과 → complete. 비고에서 "남은 것" 문구를 지우고 테스트 부채 4만 남깁니다.

---

### 2.3 shell-96 Emmet provider

#### 원본 동작

- `src/app/providers/emmet-provider.tsx` 13-23: `settings.emmetEnabled`(기본 true)가 켜져 있을 때만 `enableEmmet(monaco)`를 등록하고 꺼지면 해제합니다.
- `src/shared/lib/emmet-integration.ts` 13-33: `tokenizer: 'standard'`, HTML 계열 `html`·`heex`, CSS 계열 `css`·`scss`, JSX 계열 `javascriptreact`·`typescriptreact`
- 주 창(`src/app/app.tsx` 102)·보조 창(74) 모두 mount

#### native 현재 자산

- `editor-completion-emmet.rs`, `editor-completion-provider.rs`의 Emmet 공급(`settings.emmet_enabled`), 설정 스위치 `Switch::EmmetEnabled`
- 실제 App 검사 `application-emmet-tests.rs`: html(설정 변경 포함), css·scss, heex, jsx, tsx, 다른 파일 html peek. `actual_emmet(original, expected, extension, language)` helper. 대응표 editor-80 complete.

#### 빠진 것

- 보조 창 편집기에서의 실제 App 검사 1건. 완성 공급자는 창 사이 공유라 동작 차이는 예상하지 않습니다.

#### 구현 설계

- 코드 변경 없음이 기대값입니다. 검사가 실패하면 보조 창 범위(`active_window_scope`) 판정만 고칩니다.

#### 자동 검사

- 실제 App(`application-emmet-tests.rs` 또는 `application-auxiliary-window-tests.rs`): 보조 창에 html 파일 탭을 복원해 열고 `ul>li` 축약을 공급·수락 → 펼친 결과, undo 1회로 복귀.

#### 완료 판정

- 통과 → complete. 비고에 editor-80 근거와 이 검사를 적습니다.

---

### 2.4 shell-90 외부 열기(agent CLI)

#### 원본 동작

- 진입
    - 두 번째 실행: `tauri_plugin_single_instance`(`src-tauri/src/lib.rs` 821-830)가 실행 중인 인스턴스의 메인 창을 `show()`·`set_focus()`하고, argv를 `parse_cli_payload`로 읽어 요청이 있으면 `queue_external_open` 후 `AgentExternalOpen` 이벤트를 냅니다. 요청이 없어도 창은 앞으로 옵니다.
    - 첫 실행: `queue_cold_start_external_open`(`src-tauri/src/lib.rs` 982, `src-tauri/src/domain/agent/commands.rs` 339-345)이 대기열에만 넣고 화면 mount 때 drain합니다.
    - `parse_cli_payload`(`crates/taide-agent/src/service.rs` 256-270): argv[0]을 건너뛰고 `--wait-marker <m>`을 읽으며, 그 밖의 첫 인자 하나만 경로입니다.
    - `queue_external_open`(`commands.rs` 326-333): marker가 있으면 `AgentStore::register_wait_marker`, 경로를 `AppState::authorize_cli_opened_path`로 허용(OS 임시 폴더의 Claude Code Ctrl+G 파일), `AgentStore::push_pending_external_open`
    - CLI(`crates/taide-cli/src/main.rs` 246-316): 파일을 절대 경로로 바꾸고, `--wait`이면 임시 폴더에 `taide-wait-<uuid>` marker 파일을 만든 뒤 앱을 `<files> --wait-marker <marker>`로 띄우고 marker가 지워질 때까지(또는 timeout) 기다립니다.
- 처리(`src/app/providers/agent-external-open-provider.tsx`, 주 창만 mount `src/app/app.tsx` 97)
    - drain(86-91, 111, 113-116): 이벤트 payload가 아니라 `agent_pending_external_opens`(대기열 비우기)로 처리합니다. mount 때 1회, 이벤트마다 1회. 실패는 삼키고 대기열은 다음 drain에서 다시 처리됩니다.
    - mount 때 `clearStaleWaitMarkersOnStartup`(`src/entities/agent/agent-wait-marker-registry.ts` 80-85): 이전 실행이 남긴 localStorage 경로→marker 지도를 비웁니다(파일은 지우지 않음).
    - 요청마다(36-78)
        1. `openProject(path)` 성공 → 프로젝트 목록 무효화, marker 해제, 끝
        2. 실패 → 프로젝트 목록과 활성 프로젝트로 대상 선택(`src/entities/agent/external-open-target.ts` 15-23)
            - root가 경로를 포함하는 프로젝트 중 root 문자열이 가장 긴 것. 길이가 같으면 먼저 나온 것(`best.root.length >= project.root.length ? best : project`)
            - 포함 판정은 `isWithinRoot`(`src/shared/lib/path-root.ts` 15-40): 두 경로를 `\`→`/`, `.` 제거, `..`는 앞 조각 제거로 정규화한 뒤 같거나 `root + '/'`로 시작
            - 없으면 활성 프로젝트, 그것도 없으면 첫 프로젝트
        3. 대상 없음 → `app.openProjectFirst` info toast, marker 해제
        4. `activateProject(target)` → `openFileTab({ projectId, path, target: null, preview: !waitMarker })`
            - 성공이고 marker가 있으면 `registerWaitMarker(path, marker)` + `app.externalEditorTabHint` info toast
            - 실패 → marker 해제(toast 없음)
        5. 목록 조회·활성화 실패 → `toast.error(describeIpcError)`, marker 해제
    - `--wait` 요청은 preview가 아닌 탭으로 엽니다. preview 교체는 닫기가 아니라서 marker가 해제되지 않기 때문입니다(27-35).
- marker 해제(`src/entities/layout/tab-path-change.ts` 145-152 `releaseClosedFileTabPath`): 닫은 뒤의 그 프로젝트 layout(보조 창 포함 `collectAllPaneTabs`)에 같은 경로 문자열의 파일 탭이 없을 때만 그 경로의 marker를 모두 꺼내 `agent_release_marker`합니다. `useCloseTab`과 삭제 추종(`followDeletedPathInTabs`)이 부릅니다. 앱 종료 때 남은 marker 파일은 runtime이 지웁니다.
- runtime: `agent_actions::agent_release_marker(state, agents, tasks, marker)`(`crates/taide-runtime/src/agent_actions.rs` 110-127, 임시 폴더·`taide-wait-` 접두사 검증 후 파일 삭제), `agent_pending_external_opens`(130-132), 종료 `cleanup_all_wait_markers`(14)
- 문구: `app.openProjectFirst`, `app.externalEditorTabHint`(`crates/taide-locale/resources/locales/en.json` 35, 26). 카탈로그 추가 없음.

#### native 현재 자산

- `AgentStore`(`crates/taide-agent/src/store.rs` 153-175): `register_wait_marker`, `forget_wait_marker`, `take_all_markers`, `push_pending_external_open`, `drain_pending_external_opens`. 종료 때 `application.rs` `drain_services`가 `cleanup_all_wait_markers`
- `AppState::authorize_cli_opened_path`(`crates/taide-runtime/src/state.rs` 148)
- 열기 순서 선례: `domains/shell.rs`의 `open_dropped_paths`(`validate_open_root` → `integrations.flush_view_states` → `NativeProjects::open`, 실패면 `host::open_file_tab(services, project, None, path, preview)`)
- `crate::tabs::close`가 닫은 탭과 같은 canonical 경로가 모든 프로젝트 layout에 남았는지(`ClosedNativeTab.has_remaining_file`)를 계산
- 원격 fanout 제외: `event-relay.rs`(`AgentExternalOpen`은 `None`)
- 없음: 실행 인자의 경로(`bootstrap::LaunchConfig::parse`가 `--data-dir <dir>` 뒤 인자를 "unexpected native launch argument"로 거부), 단일 인스턴스, drain 소비자, 경로별 marker 등록·해제

#### 구현 설계

- 새 모듈 `native/taide-native-app/src/external-open.rs`(`crate::external_open`)
    - `queue(services, request)`: 원본 `queue_external_open`과 같은 세 단계
    - `resolve_target(path, projects: &[(ProjectId, String)], active: Option<&ProjectId>) -> Option<ProjectId>`: 위 원본 규칙 그대로(정규화, 형제 접두사 `/a/root-other`는 `/a/root` 안이 아님, 같은 길이는 먼저 나온 것)
    - `WaitMarkers`: `Mutex<HashMap<String, Vec<String>>>`. `register(path, marker)`, `take(path) -> Vec<String>`. 키는 탭에 저장된 경로 문자열 그대로입니다(`layout_open_tab`은 경로를 바꾸지 않음). `Arc`로 host 통합(`HostIntegrations`)과 IDE `LayoutActions`에 나눠 줍니다.
    - 원본의 시작 시 지도 비우기는 native에서 할 일이 없습니다. 지도는 프로세스 메모리라 실행마다 비어 있습니다.
    - `Outcome`: `ProjectOpened`, `Opened`, `WaitRegistered`, `OpenProjectFirst`, `Failed(AppError)`
- 실행 인자: `LaunchConfig`에 `external_open: Option<ExternalOpenRequest>`를 추가합니다. `--data-dir <dir>` 뒤의 나머지 인자를 `parse_cli_payload(&["taide-native".into(), 나머지…])`로 읽습니다. 기존 오류(없는 data dir, 상대 경로, root, 그리고 `--data-dir`가 첫 인자가 아님)는 그대로입니다. 미리보기 helper 플래그 분기(`main.rs`의 `preview_web_helper::FLAG`)는 건드리지 않습니다.
- 단일 인스턴스: 새 모듈 `single-instance.rs`
    - 소켓 경로: `std::env::temp_dir().join(format!("taide-native-{:016x}.sock", fnv1a(data_dir 바이트)))`. data dir 안에 두지 않는 이유는 macOS `sun_path` 104바이트 제한입니다. 해시는 파일 안의 작은 FNV-1a 함수입니다(의존성 없음).
    - `main.rs`에서 `LaunchConfig::parse` 직후, `restore` 전에 `Claim::acquire(path)`
        - 연결 성공 → 두 번째 인스턴스: argv를 JSON 한 줄로 보내고 1바이트 ack(`RELAY_ACK_TIMEOUT`)를 받으면 창을 만들지 않고 `Ok(())`로 끝
        - 연결 거부(파일만 남음) → 소켓 파일 삭제 후 bind, 권한 0600
        - bind 실패 → 지금처럼 단독 실행하고 `log::warn`(중계만 꺼짐)
    - 첫 인스턴스는 std `UnixListener`를 앱 생성 인자로 넘기고, 서비스가 생긴 뒤 `tasks.spawn_transient`로 tokio `UnixListener` accept loop를 띄웁니다(tokio `net` 기능은 이미 켜져 있음).
        - 연결마다 최대 `MAX_RELAY_BYTES`(예: 64 KiB), `RELAY_READ_TIMEOUT`(예: 2초)로 한 줄을 읽습니다. 넘거나 JSON 문자열 배열이 아니면 닫고 무시합니다.
        - 요청이 있으면 `external_open::queue` 후 `services.events.publish(AppEvent::AgentExternalOpen { request })`
        - 요청 유무와 관계없이 수신함에 "메인 창 앞으로" 플래그를 세우고 ack를 보냅니다.
        - `state.is_shutting_down()`이면 loop를 끝냅니다. 종료 drain이 이 task를 기다리지 않도록 transient로 둡니다. 소켓 파일은 첫 인스턴스 종료 때(`Claim` drop) 지웁니다.
- 첫 실행 요청: 생성자에서 `external_open::queue`를 부르고 수신함 플래그를 세웁니다(원본 mount drain과 같은 첫 tick 처리).
- drain: `consume_provider_events`가 플래그를 보면 `Command::ProcessExternalOpens`를 보냅니다(종료 중 제외, 제출 실패면 플래그 유지). host는 `services.agents.drain_pending_external_opens()`를 순서대로 처리합니다.
    1. `validate_open_root` → `flush_view_states` → `NativeProjects::open(path)` 성공 → marker 해제 → `ProjectOpened`
    2. 실패 → `services.state.projects`와 세션의 활성 프로젝트(없으면 세션 프로젝트 순서의 첫 열린 프로젝트)로 `resolve_target`
    3. 없음 → marker 해제 → `OpenProjectFirst`
    4. `flush_view_states` → `project_actions::project_activate(events, state, target)` → `open_file_tab(services, target, None, path, preview = marker 없음)`
        - 성공 + marker → host 안에서 `WaitMarkers::register(path, marker)` 뒤 `WaitRegistered`(응답 전에 등록해야 사용자가 응답 처리 전에 탭을 닫아도 해제됩니다)
        - 성공 + marker 없음 → `Opened`
        - 실패 → marker 해제 → `Failed(error)`
    - marker 해제는 `agent_actions::agent_release_marker`이고 실패는 `log::warn`만 합니다(원본 `.catch(() => undefined)`).
- 응답 처리(`Reply::ExternalOpensProcessed`): `OpenProjectFirst` → `toast::open_project_first`, `Failed` → `report`(IPC 오류 toast), `WaitRegistered` → `app.externalEditorTabHint` info toast. 모두 메인 창 층입니다(2.11 `ToastTarget::Main`).
- 메인 창 앞으로: 플래그가 있으면 `context.send_viewport_cmd_to(ViewportId::ROOT, ViewportCommand::Visible(true))`와 `ViewportCommand::Focus`
- marker 해제 지점. 조건은 원본처럼 "닫은 탭의 그 프로젝트 layout(메인·보조 트리)에 같은 경로 문자열의 파일 탭이 없음"입니다. 기존 `has_remaining_file`은 모든 프로젝트·canonical 비교라 범위가 넓으므로, 해제 판정용 helper `project_has_file_tab(layout, path)`를 `tabs.rs`에 둡니다.
    - host `CloseTab` 처리: `tabs::close` 결과가 `TabKind::File { path }`이고 helper가 거짓
    - IDE `LayoutActions::close_tab`(`ide-tools.rs`의 클로저): 닫은 뒤 helper가 거짓
    - 삭제 추종: `explorer_delete`·`workspace_delete`의 탭 닫기 뒤 helper가 거짓인 경로
    - 프로젝트 닫기는 원본처럼 해제하지 않습니다(종료 cleanup이 처리).
- 원본과 다르게 두는 점: 원본은 IDE closeTab 경로에서 marker를 해제하지 않아 CLI가 timeout까지 기다립니다. 결함이라 재현하지 않고 모든 닫기 경로에서 해제합니다.

#### 자동 검사

- crate 단위(`external-open-tests.rs`): `resolve_target`(가장 긴 root, 같은 길이 첫 항목, 형제 접두사 제외, `..` 정규화, 활성 → 첫 프로젝트 대체, 없음), `WaitMarkers`(한 경로 여러 marker, take 뒤 비움), `LaunchConfig::parse`(경로만, 경로+marker, data dir만, 기존 오류 유지)
- crate 단위(`single-instance-tests.rs`, 임시 폴더 소켓): 첫 claim이 Primary, 두 번째가 argv를 전달하고 ack 수신, 남은 소켓 파일 교체, 상한 초과 거부
- 실제 App(`application-external-open-tests.rs`). 요청은 `external_open::queue` + `services.events.publish(AppEvent::AgentExternalOpen { .. })`로 넣고, marker 파일은 `std::env::temp_dir()`의 `taide-wait-<uuid>`로 만들고 검사 끝에 지웁니다.
    1. 프로젝트 폴더 경로 → 프로젝트가 열리고 탭 없음, marker 파일 삭제
    2. 열린 프로젝트 안 파일 + marker → 그 프로젝트 포커스 pane에 preview 아닌 탭, hint toast, marker 파일 유지 → `HostCommand::CloseTab` → marker 파일 삭제
    3. 같은 파일을 분할해 탭 두 개 → 하나 닫으면 유지, 둘 다 닫으면 삭제
    4. 열린 프로젝트 없음 → `app.openProjectFirst` toast, marker 삭제
    5. 모든 root 밖 임시 파일, marker 없음 → 활성 프로젝트에 preview 탭
    6. 없는 파일 → 오류 toast, marker 삭제
    7. 생성 인자로 넘긴 첫 실행 요청이 첫 tick에 처리됨

#### 완료 판정

- 위 검사 통과 → complete. 비고에 남길 것: 실제 `taide` CLI(지금 `/Applications/TAIDE.app` 또는 `TAIDE_APP_PATH`를 `--data-dir` 없이 실행)를 native에 잇는 것은 실제 사용자 데이터 경로(shell-15) 허락 뒤 실기. terminal-agent-57은 결정 1에 따라 같은 근거로 갱신합니다.

---

### 2.5 shell-91 OS 알림

#### 원본 동작

- `src/app/providers/native-notification-provider.tsx`(주 창만, `src/app/app.tsx` 104)
    - agent(143-161): 프로젝트별 "session id → working 시작 시각" 지도를 `evaluateAgentCompletions`(`src/shared/lib/native-notification-gate.ts` 68-96)로 다시 만듭니다.
        - `working`이면 기존 시작 시각을 유지하거나 지금으로 둡니다.
        - 완료 활동(`idle`·`awaitingInput`)이고 시작 시각이 있으며 경과가 `AGENT_COMPLETION_NOTIFY_MIN_WORKING_MS`(10초, `src/shared/constants/notification.ts` 9) 이상이면 완료입니다.
        - 로스터에서 사라진 agent와 그 밖 활동은 알리지 않고 지도에서 빠집니다.
        - 범주: `awaitingInput` → `agentAwaitingInput`, 그 밖 → `agentCompleted`
        - 제목(`src/shared/lib/notification-text.ts` 42-50): 프로젝트 표시 override label, 없으면 프로젝트 이름. 프로젝트 정보가 없으면 두 범주 모두 `notification.agentCompleted`
        - 본문(provider 57-64): `tab` = 그 세션 터미널 탭 제목이 공백이 아니면 `" · " + 제목`, 아니면 빈 문자열(`notificationBodyFragment` 27, 보조 창 트리 포함 `findTerminalTabTitle` 58-62)
            - `awaitingInput`: `blockedReason`이 `permission` → `notification.agentPermissionBody`, `question` → `notification.agentQuestionBody`, 그 밖·없음 → `notification.agentAwaitingInputBody`(인자 `agent`, `tab`)
            - 완료: `notification.agentFinishedBody`(인자 `agent`, `duration`, `tab`)
    - `projectClosed`(163-165): 그 프로젝트 지도를 지웁니다.
    - 명령 완료(178-194): `durationMs >= 10000`일 때만
        - 성공 = exit code 없음 또는 0. 범주 성공 `taskCompleted`, 실패 `error`
        - 소유 프로젝트·탭 제목은 열린 프로젝트들의 layout에서 그 세션 터미널 탭을 찾습니다(`findTerminalSessionOwner` 73-79)
        - 제목: 프로젝트 제목, 없으면 `notification.taskCompletedSucceeded`/`notification.taskCompletedFailed`
        - 본문: `[탭 제목 ?? cwd ?? terminal.title, exit code가 있으면 notification.exitCode(code), 경과]`를 공백 항목을 빼고 `" · "`로 잇습니다(`buildTaskCompletionBody` 73-78, `joinNotificationBody` 19). exit code 0도 넣습니다.
    - LSP 설치(196-204): `done`/`failed`만. 범주 `lspInstall`/`error`, 제목 `notification.lspInstallSucceeded`/`notification.lspInstallFailed`, 본문 `message ?? serverId`
    - 첫 전달 안내(137-141, 206): 세션에 처음 `delivered`가 되면 `notification.enableHint` info toast 1회. 신호는 `notifyNative`를 지나는 모든 알림(다른 화면의 git push·검색 치환 포함)에서 옵니다(`src/entities/notification/notify.ts` 16-18, 45).
- 경과 문구 `formatDurationShort`(`src/shared/lib/format-duration.ts` 21-31): 초 = floor(ms/1000). 시간이 있으면 `common.durationHoursMinutes`(`hours`, `minutes`), 분이 있으면 `common.durationMinutesSeconds`(`minutes`, `seconds`), 아니면 `common.durationSeconds`(`seconds`)
- 전송(`notify.ts` 37-50): 메인 창·비미러에서만, 제목·본문을 코드 포인트 200개로 자르고 넘치면 199개 + `…`(`truncateNotificationText`, gate.ts 119-123). Rust `notification_notify`가 설정 게이트(마스터 → 범주 → "포커스 없을 때만"과 아무 창 포커스, `crates/taide-notification/src/service.rs` `decide_delivery`)와 비밀 가림(`crates/taide-runtime/src/notification_actions.rs` 8-32)을 거칩니다. 실패는 삼킵니다.
- 문구 키는 모두 en.json에 있습니다(463-478, 51-53, `terminal.title`). 카탈로그 추가 없음.

#### native 현재 자산

- 생산자: agent 폴링(`application-ports.rs`, `AgentStateChanged`), 명령 완료(`terminal_dispatch.rs`, `TerminalCommandFinished`), LSP 설치(runtime `lsp_install_actions`, `LspInstallProgress`)
- 전송: `bootstrap::NativePlatform::send_notification`(macOS osascript), runtime `notification_actions::notification_notify(state, platform, category, title, body, has_focused_window)`
- 포커스 판정: `NativeApplication::is_any_window_focused(context)`(domains/shell.rs, 메인 `raw.focused` 또는 보조 viewport 중 하나)
- 프로젝트 표시 라벨: `taide_native_ui::project_rail::resolve_project_display`
- 없음: 소비자, 시간 지도, 본문 조립, 경과 문구, 잘라내기, 첫 전달 안내, 다른 lane용 진입점

#### 구현 설계(결정 1 A 기준)

- 새 모듈 `native/taide-native-app/src/native-notifications.rs`(`crate::native_notifications`). 순수 함수는 원본 테스트(`src/shared/lib/native-notification-gate.test.ts`, `src/shared/lib/notification-text.test.ts`)의 사례를 옮길 수 있게 시각·문구를 인자로 받습니다.
    - 상수: `AGENT_COMPLETION_NOTIFY_MIN_WORKING`(10초), `TASK_COMPLETION_NOTIFY_MIN_DURATION_MS`(10000), `NOTIFICATION_TEXT_MAX_CODE_POINTS`(200)
    - `Tracker { working_since: HashMap<ProjectId, HashMap<String, Instant>>, has_announced_delivery: bool }`
    - `evaluate(previous: &HashMap<String, Instant>, agents: &[DetectedAgent], now: Instant) -> (HashMap<String, Instant>, Vec<Completion>)`
    - `agent_body`, `task_body`, `format_duration(locale, ms)`, `truncate(text, max)`, `join_body(parts)`, `body_fragment(text)`, `project_title(projects, project, fallback)`, `terminal_tab_title(layout, session)`(main 트리 + `auxiliary_windows`), `session_owner(projects, layouts, session)`
    - `Request { category: NotificationCategory, title: String, body: String, any_window_focused: bool }`
- 다른 lane용 진입점: `domains/shell.rs`의 `NativeApplication::notify_event(&mut self, context, category, project: Option<&ProjectId>, fallback_title_key: &str, body: String)`. 제목 규칙(표시 label → 이름 → 대체 키), 잘라내기, 포커스 계산, `Command::Notify` 제출을 한 곳에서 합니다. git push/pull(git lane)과 검색 치환(explorer-search lane)이 이것을 부릅니다.
- 소비: `consume_provider_events`가 수신함의 agent·명령·LSP·닫힘을 처리하고 `notify_event`를 부릅니다. 프로젝트·layout은 `self.controller.snapshot()`의 `projects`·`layouts`를 씁니다.
- host `Notify`: `tasks.run_blocking_result`에서 `notification_notify(&state, &*services.platform.0, category, &title, &body, || any_window_focused)`(osascript가 blocking). 검사는 2.0.3 override로 가짜 플랫폼을 넣습니다.
- 응답 `NotificationSent(Ok(NotificationDelivery::Delivered))`이고 아직 안내하지 않았으면 `notification.enableHint` info toast(메인 창 층) 1회. 오류는 삼키고 `log::warn`만 합니다.

#### 자동 검사

- crate 단위(`native-notifications-tests.rs`)
    - 평가: 10초 경계(9.999초 미알림, 10초 알림), working 유지 중 시작 시각 보존, 사라진 agent·그 밖 활동 미알림, 프로젝트별 분리
    - 본문: 이유 3종과 없음, 탭 꼬리 유무(공백 제목은 꼬리 없음), 완료 경과
    - 경과: 59초, 60초, 3599초, 3600초
    - 자르기: 코드 포인트 기준(이모지·한글), 200 이하 그대로, 201은 199 + `…`
    - 명령 본문: exit code 없음 생략, 0 포함, 탭 → cwd → `terminal.title` 대체
    - 소유: 보조 창 트리의 터미널 탭도 찾음
- 실제 App(`application-notifications-tests.rs`, 가짜 플랫폼)
    - `TerminalCommandFinished { duration_ms: 12000, exit_code: Some(1) }` 발행 → `error` 범주, 프로젝트 제목, 본문 순서
    - `LspInstallProgress { phase: Done }` → `lspInstall`. `Downloading`은 호출 없음
    - 설정 "포커스 없을 때만" + 메인 viewport focused → 플랫폼 호출 없음. 포커스 없음 → 호출
    - 첫 전달 뒤 `notification.enableHint` toast 1회, 두 번째 전달 뒤 추가 toast 없음
    - agent 경로는 `consume_provider_events`에 명시 시각을 넘겨 working → idle(11초) 1건

#### 완료 판정

- 위 검사 통과 → complete. 비고에 남길 것: 실제 OS 알림 표시·osascript 경로는 실기(배치 31), Windows/Linux 전송은 릴리스 대상 밖. 결정 1 결과에 따라 terminal-agent-59~63과 settings-23(agent·task·LSP 범주)을 같은 근거로 갱신합니다. git remote·검색 치환 범주는 각 lane이 `notify_event`를 부른 뒤 그 lane이 갱신합니다.

---

### 2.6 shell-92 IDE 동기화

#### 원본 동작

- `src/app/providers/ide-sync-provider.tsx`(주 창만, `src/app/app.tsx` 98)
    - `ideDiffRequested`(49-68): 대기 diff 등록(oldPath, newContents, tabName). `ideAutoOpenDiff`가 꺼져 있으면 등록을 지우고 `rejected`로 응답. 켜져 있으면 그 프로젝트에 `claudeDiff` 탭(`{ requestId, path: newPath }`, 제목 tabName, preview false, target null). 오류는 IPC 오류 toast
    - `ideSaveRequested`(70-129)
        - 그 프로젝트의 모든 트리(보조 창 포함)에서 같은 경로 파일 탭을 찾습니다.
        - 없거나 dirty가 아니면 `saved: true`
        - 모델이 없으면 `false`
        - readOnly(인코딩 손실 강제 포함)면 `false`
        - 그 밖에는 모델 본문을 `file_save`로 저장, mirror 목록 무효화, `setTabDirty(false)`, `true`. 실패는 `false`
        - 원본은 보조 창 탭을 realm 제약으로 저장하지 못해 `false`를 답합니다(76-80 주석).
    - `ideCloseTabRequested`(131-133): 대기 diff 등록 제거
    - 상태 동기화(135)
    - 진단 push(137-158): 포커스 slot 프로젝트가 있고 IDE가 실행 중일 때, monaco 표식·포커스 프로젝트·실행 상태가 바뀔 때마다 300ms(`IDE_DIAGNOSTICS_PUSH_DEBOUNCE_MS`) 뒤 모든 표식을 `{ path, severity, startLine, startCharacter, endLine, endCharacter, message, source }`(LSP 0 기준)로 `ide_publish_diagnostics(projectId, items)`. 표식이 없어도 빈 목록을 보냅니다. 실패는 삼킵니다.

#### native 현재 자산

- IDE 서버·도구: `ide-server.rs`, `ide-tools.rs`(`open_diff`가 `IdeDiffRequested`, `save_document`가 canonical 경로로 `IdeSaveRequested` 발행 후 `IDE_SAVE_TIMEOUT` 대기, `close_tab`이 `IdeCloseTabRequested`)
- 상태: `status-ide.rs`. 대응표 terminal-agent-65 complete
- 저장: `store.save_snapshot(document)` → `HostCommand::Save { path, snapshot }` → `HostReply::Saved { snapshot, result }` 처리(dirty 정리 포함)
- 진단: `diagnostics::Store`(`batches()`, `Batch { key, diagnostics: Vec<Arc<lsp Diagnostic>> }`, `diagnostics::resource(&key) -> Option<String>`, 변경 표지 `revision() -> &Arc<()>`; 바뀔 때마다 새 `Arc`라 `Arc::ptr_eq`로 비교, 선례 `problems.rs`·`editor-problems.rs`). JSON 내장 진단 포함
- 모델: `taide_model::ide::IdeDiagnostic { path, severity: IdeDiagnosticSeverity, start_line, start_character, end_line, end_character, message, source }`
- 탭 닫힘의 diff 거절: `tabs::close`가 `ClaudeDiff` 탭이면 `take_pending_diff` → `TabClosed`
- 없음: save·diff·close 요청 소비자, 진단 push

#### 구현 설계(이 lane 범위: save, 진단)

- 새 모듈 `native/taide-native-app/src/ide-sync.rs`(`crate::ide_sync`)
- save 요청: `consume_provider_events`에서
    1. `services.state.layouts`의 그 프로젝트 layout 모든 트리(`taide_layout::service::all_roots` + `tabs::tabs_in`)에서 같은 경로 문자열의 파일 탭을 찾습니다. 없으면 `ResolveIdeSave { saved: true }`
    2. `files`에서 그 경로의 문서를 찾지 못하면 `false`
    3. 문서가 dirty가 아니면 `true`. dirty 판단은 문서 store snapshot을 씁니다(layout의 `dirty`는 `SetDirty`로 뒤따라 반영되어 한 frame 늦을 수 있음)
    4. 문서 metadata `read_only`(인코딩 손실 강제 포함)면 `false`
    5. `store.save_snapshot(document)` → 저장 명령 제출, `ide_saves: HashMap<DocumentId, Vec<String>>`에 요청 id 기록. 제출 실패면 `false`
    6. `HostReply::Saved` 처리 끝에서 그 문서의 대기 요청을 꺼내 결과가 Ok면 `true`, Err면 `false`
    - format-on-save는 거치지 않습니다(원본 `useSaveFile` 직접 저장).
    - 보조 창 탭도 같은 store라 저장합니다. 원본의 `false`는 realm 제약이라 재현하지 않습니다.
- 진단 push
    - 매 tick 서명 `(diagnostics.revision()의 Arc 동일성, 포커스 slot 프로젝트, services.ide.is_running())`을 비교합니다. 바뀌면 마감 = 지금 + `IDE_DIAGNOSTICS_PUSH_DEBOUNCE`(300ms)
    - 마감이 지나고 프로젝트가 있고 실행 중이면 모든 batch를 `IdeDiagnostic`으로 바꿔 `PublishIdeDiagnostics { project, items }`를 보냅니다. 경로는 `diagnostics::resource`(없으면 건너뜀), 범위는 LSP 위치 그대로, severity는 1→Error, 2→Warning, 3→Info, 4→Hint, 없음→Error(native `diagnostics::severity`와 같은 기본값; 원본 monaco 표식은 항상 severity가 있음), `source` 그대로
    - 마감 대기 중에는 `request_repaint_after`로 깨웁니다.
- diff·close 요청은 git lane git-49가 구현합니다(대기 diff 등록, 자동 열기 꺼짐 거절, `TabKind::ClaudeDiff` 탭 열기, close 요청 등록 제거). 그 lane이 이 모듈에 붙이도록 `ide_sync`에 빈 자리 없이 공개 함수만 둡니다.

#### 자동 검사

- 실제 App(`application-ide-sync-tests.rs`). 대기 save는 `services.ide.insert_pending_save_owned(request_id, PendingSave { responder })`로 만들고 `AppEvent::IdeSaveRequested`를 발행해 responder 값을 확인합니다.
    - dirty 파일: `true`, 디스크 본문 = 편집 본문, 탭 clean
    - 깨끗한 파일: `true`, 디스크 쓰기 없음(수정 시각 불변)
    - 인코딩 손실 파일: `false`, 디스크 불변
    - 열린 탭 없음: `true`
    - 보조 창 탭 dirty: `true`, 저장됨
- 실제 App: IDE 실행 상태를 만들고(`application-ide-selection-tests.rs`의 방식) 스키마 오류가 있는 JSON 파일을 열기 → 300ms 뒤 `services.ide.diagnostics(None)`에 그 경로·0 기준 범위·severity·source 항목. 포커스 slot 프로젝트를 바꾸면 다시 push. 실행 중이 아니면 push 없음

#### 완료 판정

- save·진단 검사 통과 후 비고 갱신, 상태는 partial 유지(diff 요청이 묶음 3). 결정 1에 따라 terminal-agent-70·73을 같은 근거로 갱신합니다.

---

### 2.7 shell-94 테마·로케일 적용과 로드 오류 배너

#### 원본 동작

- `src/app/providers/theme-provider.tsx` 23-24, 100-112: 테마 query 실패면 `theme.loadFailed` + `common.retry` 배너(문구는 defaultValue `Failed to load the theme`/`Retry`), 재시도는 테마 `refetch`. 로케일도 실패면 32px 아래(`stackOffsetPx`)
- `src/app/providers/locale-provider.tsx` 8, 25-32: 로케일 실패 배너는 고정 영어 문구(`Failed to load the app language pack. Some labels may show as raw keys.`), 맨 위
- 시작 때 실패
    - 테마는 `src/shared/styles/global.css` 6-7의 기본 변수(`--taide-app-background: #1e1e2e` 등)로 그립니다. 이 값은 번들 `taide-dark`(`crates/taide-theme/src/service.rs` 318 `app.background`)와 같습니다.
    - 로케일은 i18next가 빈 리소스·`en`으로 시작해(`src/shared/i18n/i18n.ts` 7-15) 키가 그대로 보입니다.
- 실행 중 실패는 이전 데이터를 유지하고 배너만 보입니다.
- 배너는 `src/shared/ui/status-error-banner.tsx` 9, 18-28(`fixed inset-x-0 z-50`)이고 drop overlay(`z-50`, 앱 셸 안)보다 위입니다(`2026-10-28-native-explorer-views-theme-providers.md`).
- 메인 결정: 시작 때 실패하면 번들 기본 테마(`taide-dark`)와 기본 로케일(`en`)로 계속 띄우고 원본처럼 배너를 보입니다. 로케일은 원본의 "키 노출" 대신 번들 `en`을 씁니다.

#### native 현재 자산

- 실행 중 실패: `HostReply::Presentation` 처리(`presentation_failures` 갱신), `status_error_banner::{Failures, show}`(`egui::Area`, `Order::Middle`, id `(LAYER, kind)`), `presentation_refresh::Refresh::retry`. 근거 `2026-10-28-native-explorer-views-theme-providers.md`
- 시작 경로: `NativeApplication::new_with_secret_store`가 `Inputs::load` 결과를 `resolved.theme?`, `resolved.locale?`로 받고 이어서 `Appearances::new(&theme, ..)?`라, 하나라도 실패하면 앱 생성 실패입니다.
- 번들 로더: `taide_runtime::theme_actions::theme_get(state, BUILTIN_DARK_ID.into())`(builtin은 디스크를 읽지 않음, `taide_theme::service::BUILTIN_DARK_ID`, `load_theme` 996), `locale_actions::locale_get(state, BUILTIN_EN_ID.into())`(`taide_locale::service::BUILTIN_EN_ID`, `load_locale` 1456)

#### 빠진 것

1. 시작 때 테마·로케일 로드 실패(또는 사용자 테마의 `Appearances::new` 실패)의 대체와 배너
2. drop overlay가 배너 위에 그려지는 순서

#### 구현 설계

- 생성자
    - 테마: `resolved.theme.and_then(|theme| Appearances::new(&theme, &settings).map(|a| (theme, a)))`가 Err면 `log::warn` 후 `theme_get(&state, BUILTIN_DARK_ID.into())`와 그 `Appearances`를 씁니다. 대체까지 실패(번들 결함)하면 지금처럼 생성 오류입니다. `theme_failed = true`
    - 로케일: Err면 `locale_get(&state, BUILTIN_EN_ID.into())`, `locale_failed = true`
    - `presentation_failures: Failures { theme: theme_failed, locale: locale_failed }`
    - `Refresh::new(inputs, revision)`는 그대로입니다. 배너 재시도 → `retry()` → 같은 입력 재요청 → 성공하면 기존 `HostReply::Presentation` 경로가 적용하고 배너를 지웁니다.
- 배너를 overlay 위로(shell-22와 함께)
    - 원인: `file_drop::paint_overlay`(`file-drop.rs` 118-131)는 `ui.new_child(UiBuilder::new().layer_id(LayerId::new(Order::Middle, OVERLAY_LAYER)))`로 영역(Area) 목록에 없는 층에 그립니다. vendored egui(`native/taide-native-app/vendor/egui-input/src/layers.rs` 213-260 `GraphicLayers::drain`)는 같은 Order 안에서 영역 순서 목록에 있는 층을 먼저, 목록에 없는 층을 나중에 그리므로 overlay가 배너(Area) 위입니다.
    - 설계: overlay를 그린 frame에 보이는 배너마다 `context.set_sublayer(overlay_layer, banner_layer)`를 부릅니다. `Areas::set_sublayer`(`vendor/egui-input/src/memory/mod.rs` 1382-1398)는 두 층을 순서 목록에 넣고, `end_pass`(1427-1462)가 자식 층을 부모 바로 뒤로 옮깁니다. `Context::end_pass`가 이 정렬(`context.rs` 2918)을 `graphics.drain`(2966)보다 먼저 하므로 같은 frame에 반영됩니다. sublayer는 frame마다 비워지므로 overlay가 없는 frame에는 아무 영향이 없습니다.
    - `move_to_top`만으로는 부족합니다. `end_pass`의 정렬은 `(order, wants_to_be_on_top)` 기준 안정 정렬이라, 둘 다 올리면 순서 목록에서의 기존 상대 위치가 남고 overlay가 처음 목록에 들어갈 때 끝에 붙어 배너 위가 됩니다.
    - 목표 순서(아래 → 위): 셸 본문 < overlay < 배너 < Foreground(대화상자·메뉴) < toast
    - 위치: overlay를 그리는 곳(메인 `show_window`)에서 배너 id(`status_error_banner`의 `(LAYER, kind)`)를 알 수 있도록 `status_error_banner`에 `layer_ids(failures) -> impl Iterator<Item = LayerId>`를 둡니다.

#### 자동 검사

- 실제 App(`application-explorer-views-tests.rs`의 배너 검사 옆)
    - 실행 전에 `themes_dir()`·`locales_dir()`에 깨진 JSON을 쓰고 설정(`follow_system_theme=false`, `theme_id`, `language`)을 맞춘 `isolated_state`로 `launch` → 생성 성공, 첫 frame에 배너 2개(로케일 y=0, 테마 y=32), 셸 배경 `#1e1e2e`, 테마 배너 문구 = en 카탈로그 `theme.loadFailed`
    - 파일을 고친 뒤 재시도 클릭 → 배너 사라짐, 사용자 테마 색 적용
    - 사용자 테마 JSON은 열리지만 `Appearances::new`가 실패하는 경우도 대체 + 테마 배너(실패를 만드는 입력은 `presentation-refresh.rs`의 `Appearances::new` 오류 분기를 보고 고릅니다)
- 실제 App(`application-file-drop-tests.rs`): 두 실패 + hovered 파일 상태에서 `FullOutput.shapes` 안의 overlay 배경 사각형 순번 < 배너 배경 사각형 순번이고, overlay는 셸 본문보다 뒤(위)입니다.

#### 완료 판정

- 위 검사 통과 → complete. 픽셀은 배치 31

---

### 2.8 shell-22 OS 파일/폴더 drop

#### 원본 동작

- `src/widgets/app-shell/app-shell.tsx` 99-135: 메인 창만 drag 이벤트를 구독합니다. enter/over에 overlay, leave/drop에 해제. drop 경로마다 `openProject` 시도 → 성공하면 대상 변경, 실패하면 대상 없음 → `app.openProjectFirst`, 대상 있음 → preview 파일 탭(오류 toast)
- overlay `src/features/window/drag-drop-overlay.tsx` 9-15: 앱 셸 전체, `z-50`, 85% 배경, 4px 점선 accent, `FolderInput`, `app.dropToOpen`. Zen 안내(`z-20`)와 Monaco popup보다 위, 대화상자·toast보다 아래

#### native 현재 자산과 남은 범위

- 구현·검사 완료: `file-drop.rs`(`is_hovering`, `take_dropped`, `paint_overlay`, `WebDrags`), `domains/shell.rs` `open_dropped_paths`, WKWebView drag handler. 근거 `2026-10-27-native-error-isolation-file-drop.md`, `2026-10-27-native-error-boundary-panels.md`
- 남은 것
    - 배너와의 순서: shell-94 설계로 해결
    - Zen 안내·편집기 popup이 overlay 위인 순서 차이: 앞 QA가 근거를 적고 고른 의도된 차이입니다(overlay를 Foreground로 올리면 대화상자·메뉴 위로 올라가는 다른 차이가 생김). 결정 5
    - 실기(배치 31): overlay가 WKWebView를 숨긴 뒤 AppKit drag 대상 전환, drag 중 repaint, 실제 Finder drag, 점선 픽셀

#### 구현 설계

- shell-94의 sublayer 지정 외 코드 변경 없음. 기존 drop 검사(`application-file-drop-tests.rs` 2건, web drag 1건)가 그대로 통과해야 합니다.

#### 자동 검사·완료 판정

- shell-94 순서 검사 + 기존 drop 검사 통과 → complete. 비고에 실기 항목과 결정 5의 결과를 적습니다.

---

### 2.9 shell-9 네이티브 앱 메뉴

#### 원본 동작

- `src-tauri/src/domain/window/menu.rs` 169-181(`build_file_submenu`)은 메뉴를 만들 때 최근 프로젝트를 바로 읽어(`recent_entries_now`) `Open Recent` 행을 채웁니다. 그 뒤 변경은 `refresh_recent_menu`(196-203)입니다. 처리기는 `src-tauri/src/lib.rs` 398-425

#### native 현재 자산과 남은 범위

- 구현·검사 완료: `app-menu.rs`(`Model::new(locale, &[Project])`, `State`, `Changes`), `app-menu-bar.rs`, `NativeApplication::with_app_menu`(macOS), `Command::OpenRecentProject`·`ClearRecentProjects`. 근거 `2026-10-27-native-app-menu.md`
- 남은 것
    - 부팅 직후 첫 조회 전 `Open Recent`가 비어 있음: `with_app_menu`가 `Model { recent: Vec::new(), ..Model::new(&self.locale, &[]) }`로 설치
    - 실기(배치 31): NSMenu 생성·클릭·⌘Q·Hide·Services·Window, 보조 창 Cut/Copy/Paste vendored 경로
    - 기록된 차이 유지: 편집 항목 단축키 표시 없음(egui 키 입력 보존)

#### 구현 설계

- `pub(crate) fn initial_app_menu_model(&self) -> app_menu::Model`을 둡니다. `self.runtime.block_on(project_actions::project_list_recent(&self.services.state))` 결과로 `app_menu::Model::new(&self.locale, &recent)`를 만들고, 실패면 지금처럼 빈 목록입니다. `with_app_menu`는 이 모델로 설치합니다(macOS 전용 설치는 그대로).
- 첫 tick 재조회가 같은 결과면 다시 그리지 않는 기존 모델 비교(`app_menu::State`)가 그대로 동작해야 합니다. 필요하면 `State`의 `recent`도 같은 값으로 채웁니다.

#### 자동 검사

- 실제 App(`application-app-menu-tests.rs`): 최근 프로젝트가 있는 data dir로 launch → `initial_app_menu_model()`의 최근 행이 `project_list_recent` 상위 10개와 같고 `Clear Recent`가 활성

#### 완료 판정

- 통과 → complete. 비고에 실기 항목과 단축키 표시 차이를 남깁니다.

---

### 2.10 shell-3 창 위치·크기 복원

#### 원본 동작

- `src-tauri/src/lib.rs` 856-870: `tauri_plugin_window_state`, 보조 창 제외(`with_filter`), 저장 사각형이 연결된 모니터와 겹칠 때만 위치 적용
- 로거: `tauri_plugin_log::Builder::new()`(lib.rs 834-837). 기본 출력 대상은 저장소 밖 crate라 확인하지 않았습니다.

#### native 현재 자산과 남은 범위

- 구현·검사 완료: `window-state.rs`(`Tracker::observe(info, zoom_factor)`, `MonitorProbe`, `main_window_hook`), `main.rs`의 `eframe::create_native` + `event_loop.run_app(&mut MonitorProbe::new(..))`, `on_exit`의 저장. 근거 `2026-10-27-native-window-session-restore.md`, `2026-10-27-native-window-position-mirror-prune.md`
- 남은 것
    1. macOS 실행 중 zoom(최대화)을 egui-winit 0.36.2가 갱신하지 않아 zoom된 크기·위치를 일반 값으로 기록
    2. 창 생성 뒤 eframe 치명 오류가 호출자에게 오지 않아 종료 코드 0(`main.rs`는 생성 결과 `startup`만 봄)
    3. 시작 실패 원인 문구가 로그로만 남고 출력되지 않음(로거 없음) → 결정 3
    4. 실기(배치 31): 실제 위치 복원·모니터 분리·배율 혼합

#### 구현 설계

- zoom
    - `Tracker::observe(info, zoom_factor, is_zoomed: Option<bool>)`: `maximized = info.maximized == Some(true) || is_zoomed == Some(true)`. 나머지 규칙(최대화·전체 화면 중 직전 일반 크기·위치 유지)은 그대로입니다.
    - macOS에서 `is_zoomed`는 ROOT 패스의 `eframe::Frame`(vendored `epi.rs` 711 `impl HasWindowHandle for Frame`)에서 AppKit 핸들의 `ns_view`를 얻어 `objc2::msg_send![view, window]` → `msg_send![window, isZoomed]`로 읽습니다. `objc2 =0.6.4`는 이미 macOS 의존입니다. `raw_window_handle` 타입은 `wry::raw_window_handle` 재수출을 씁니다(`raw-window-handle 0.6.2` lock 고정). 핸들을 못 얻으면 `None`입니다.
    - 다른 OS는 `None`(egui 값 그대로)입니다.
- 종료 코드
    - vendored eframe(`experiments/native-shell-spike/vendor/eframe/src/native/run.rs`): `WinitAppWrapper`의 `return_result`(83, 92, 164, 394)를 `Rc<RefCell<Result<(), Error>>>`로 바꿔 `EframeWinitApplication`(492)이 같은 셀을 들게 하고 `pub fn take_result(&mut self) -> crate::Result<()>`를 엽니다. 기존 `run_native` 경로는 같은 셀에서 꺼냅니다.
    - `window_state::MonitorProbe`에 안쪽 앱의 `take_result`를 넘기는 함수를 두고, `main.rs`는 `run_app` 뒤 생성 결과 다음에 이 결과를 `?`로 돌려줍니다.
    - vendored eframe 수정은 결정 6을 따릅니다.

#### 자동 검사

- crate 단위(`window-state-tests.rs`): `is_zoomed: Some(true)`면 `maximized`가 기록되고 직전 일반 크기·위치가 유지됨, `None`이면 기존 동작
- vendored eframe: 결과 셀 공유를 작은 단위 검사로 확인합니다(이벤트 루프 없이 셀에 오류를 넣고 `take_result`가 돌려줌). 실제 오류 경로는 실기입니다.
- 실제 App: 기존 `application-window-state-tests.rs`가 그대로 통과(`observe` 인자 추가 반영)

#### 완료 판정

- 1·2 구현과 검사 통과, 결정 3 반영 → complete. 비고에 실기 항목

---

### 2.11 shell-98 앱 provider 계층(보조 창 toast)

#### 원본 동작

- `src/app/providers/app-providers.tsx` 15-26: `AppToaster`가 `AppProviders` 안에 있고, `AppProviders`는 주 창(`src/app/app.tsx` 92)·보조 창(67) 둘 다 mount됩니다. 그래서 toast는 `toast()`를 부른 realm, 곧 그 동작을 한 창에 뜹니다.
- 주 창 전용 provider(외부 열기, IDE, 알림)가 내는 toast는 항상 주 창입니다.

#### native 현재 자산

- `crate::toast::Toasts` 하나를 `NativeApplication::finish_frame`에서 ROOT로만 그립니다(`self.toasts.show(..)`). `background_tick`이 위치·감속·tick을 맡습니다. 호출부는 `application.rs`와 `domains/{shell,commands}.rs`에 30여 곳(작성 시점)
- tooltip provider, toast 층 순서는 완료(`2026-10-28-native-explorer-views-theme-providers.md`)

#### 빠진 것

- 보조 창 자체 toast

#### 구현 설계(결정 2 추천안 A)

- 새 모듈 `toast-layers.rs`: `ToastLayers { main: Toasts, auxiliary: HashMap<ViewportId, Toasts> }`
    - `for_target(ToastTarget) -> &mut Toasts`
    - `ToastTarget::Main`: 주 창 전용 provider toast(shell-90 응답, shell-91 안내, drop 알림, 앱 메뉴 kept drafts, 프로젝트 그룹 결과)
    - `ToastTarget::Interaction`: 사용자 동작의 결과. 만드는 순간 OS focus를 가진 창(`focused_viewport` 규칙, 없으면 ROOT)
- `report`와 `toast::*` helper 호출부를 `for_target`으로 바꿉니다. 기본은 `Interaction`이고 위 `Main` 목록만 명시합니다.
- 보조 창 패스(`show_auxiliary_windows`의 viewport 콜백) 끝에서 그 창의 층을 `show`하고, `take_actions`도 층마다 처리합니다. `toast_interaction_enabled`는 그 창의 모달 상태로 판정합니다. 위치·감속·tick은 `background_tick`에서 모든 층에 같게 적용합니다.
- 보조 창을 닫으면(`close_auxiliary_window`) 그 층을 버립니다(원본은 창과 함께 toaster가 사라짐).

#### 자동 검사

- 실제 App(`application-auxiliary-window-tests.rs`)
    - 보조 창에 OS focus를 준 상태에서 그 창의 동작으로 IPC 오류를 냄(예: 보조 창 탭 메뉴로 없는 파일 열기) → toast 문구가 보조 viewport 출력에만 있고 ROOT 출력에는 없음
    - `ToastTarget::Main` toast(예: 외부 열기 hint)는 보조 창이 focus여도 ROOT에만 있음
    - 보조 창 닫기 뒤 그 층의 toast가 사라짐
- 기존 toast 검사(`toast-tests.rs`, tooltip 위 층 검사) 통과

#### 완료 판정

- 통과 → complete. 픽셀·실제 지연은 배치 31

---

### 2.12 shell-5 보조 창 셸

#### 원본 동작

- `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 73-146: 제목줄(mac), view-local 탐색기 패널(240px, 최소 180px, 최대 40%), `EditorArea`. 트리가 비거나 layout 오류면 창을 닫습니다.
    - 키맵 89-100: `toggle-sidebar`, `explorer` → files, `git` → git
    - 102-108: 검색 패널·보기 전환·reveal·rename 요청이 접힌 패널을 펼침
- 보조 창의 `EditorArea`는 주 창과 같은 탭 콘텐츠를 그립니다. HTML·오디오·비디오 미리보기와 터미널(xterm 마우스 보고 포함)이 그 창 안에서 동작합니다.
- theme preview는 realm마다입니다(`src/entities/theme/theme.query.ts` 65-67의 모듈 store, `theme-provider.tsx` 30 `previewTheme ?? resolvedTheme`). 보조 창 설정 탭의 preview는 그 창에만 적용됩니다.
- `TaskRunnerDialog`·`CommandPalette`도 보조 창에 mount(`src/app/app.tsx` 77-78)

#### native 현재 자산

- 근거 `2026-10-27-native-auxiliary-window.md`, `2026-10-27-native-auxiliary-window-moves.md`: `auxiliary-windows.rs`(복원 계획은 이제 `taide_window::service::{restorable_project_ids, plan_auxiliary_window_restorations, next_auxiliary_label}`을 직접 씀), `show_auxiliary_windows`·`show_window`·`close_auxiliary_window`, `ShellIntent::ToggleAuxiliaryExplorer`, 창별 키맵·LSP·reveal/rename
- `preview_web_view::Views::sync(frame: &eframe::Frame, context, live, placements, enabled)`는 메인 `eframe::Frame`에만 붙음(ROOT 패스의 `web_placements`만 사용)
- vendored eframe `render_immediate_viewport`(`experiments/native-shell-spike/vendor/eframe/src/native/wgpu_integration.rs` 1139-1215)는 `App::raw_input_hook`을 부르지 않음. 창(`Arc<winit::Window>`)은 그 함수 안에만 있음
- `sync_theme_preview`는 `context.viewport_id()`(ROOT 패스) 기준 preview만 적용
- 메인 WKWebView의 탭 drag 비활성 판정은 ROOT 패스의 `tab_drag::is_dragging`만 봄(보조 창에서 진행 중인 drag 미반영, 같은 QA 남은 범위)

#### 이 lane 범위(묶음 2)

1. 보조 창 WKWebView 배치
    - vendored eframe: `render_immediate_viewport`가 `viewport_ui_cb` 호출 동안 그 창을 thread-local에 두고 `pub fn eframe::immediate_viewport_window() -> Option<Arc<winit::window::Window>>`로 읽게 합니다(콜백이 끝나면 비움, 재진입 시 이전 값 복원). 비 wasm 전용(`cfg`)으로 둡니다.
    - 앱: `preview_web_view::Views`를 viewport별로 나누고 `sync`가 `&impl HasWindowHandle`을 받게 일반화합니다. 보조 창 패스 끝에서 그 창의 placement와 창 핸들로 `sync`합니다. 활성 조건은 메인 `finish_frame`의 `web_enabled`와 같은 항목을 그 viewport 기준으로 계산합니다(그 창의 모달·popup·탭 drag·프로젝트 drag 중 숨김).
    - 원본 보조 창은 OS 파일 drop을 무시하므로 보조 창 web view의 drag 처리기는 항상 `false`(기본 처리)를 돌려줍니다.
    - 창을 닫으면 그 viewport의 web view를 지웁니다.
2. 탭 drag 중 web view 숨김을 모든 창 기준으로: 어느 창에서든 탭 drag가 진행 중이면 모든 창의 web view를 숨깁니다(원본은 drag 중 iframe 위에 pointer를 가로채는 층이 창마다 있음. 탭 drag는 창 사이를 넘을 수 있으므로 전 창 기준이 맞음).
3. 보조 창 터미널 raw input(마우스 보고)
    - vendored eframe: `SharedState`에 `immediate_raw_input_hook: Option<Box<dyn FnMut(&egui::Context, &mut egui::RawInput)>>`와 등록 함수 `eframe::set_immediate_raw_input_hook`을 두고, `take_egui_input` 직후 부릅니다.
    - 앱: hook이 앱 `&mut self` 없이 돌 수 있도록 `terminal_surface::Views`의 raw input 상태를 `Rc<RefCell<..>>` 공유로 나누고, 메인 `raw_input_hook`(`terminal_views.raw_input`)과 같은 함수를 부릅니다. `app_menu.inject`(보조 창)는 지금처럼 viewport 콜백 안에 둡니다. replay 변형(`raw_input_hook_with_replay`)이 보조 창에서 필요한지는 vendored egui-input의 replay 경로를 보고 정하고, 필요 없으면 이유를 QA에 적습니다.
    - `terminal_surface.rs` 소유는 terminal-agent lane과 맞춥니다(1.3).
4. 보조 창 theme preview
    - `HostReply::Presentation` 처리와 `sync_theme_preview`에 같은 "Appearances 적용" 블록이 두 번 있으므로 `apply_appearances(&mut self, context, appearances)`로 공통화합니다(2회 이상 규칙).
    - 메인 `Appearances`를 보관하고, 보조 창 패스 시작에 `settings_views.preview(window.viewport, state)`가 있으면 그 preview의 `Appearances`(viewport·preview 값으로 캐시)를 적용, 패스 끝에 메인 값으로 되돌립니다. visuals와 터미널 palette(`terminal_views.set_palette`)는 context 전역이라 되돌림이 필요합니다.

#### 묶음 3으로 넘기는 범위

- 보조 창 탐색기의 search·git 보기 전환과 `git` 키: explorer-search-30(검색 본문), git-1(Git 본문·`ShowAuxiliaryExplorerView`)
- 탐색기 선택·inline rename·create 입력의 창별 분리: explorer-search D6(mount별 상태). 이전 초안의 "shell lane wave 3에서 처리"는 이 사양으로 대체됩니다.
- 보조 창 `TaskRunnerDialog`(terminal-agent-76, shell-24)
- 파일 선택 dialog(rfd) 부모 창: 원본 근거가 저장소 밖 crate라 허락 4

#### 자동 검사

- 1: 보조 창 HTML 탭의 placement가 그 viewport로 기록되고 메인 placement에는 없음(실제 App, web view 생성은 GUI라 placement 계산까지). web view 생성·표시는 실기
- 2: 보조 창에서 탭 drag 시작 상태를 만들면 메인 web view 활성 값이 거짓(실제 App, `web_enabled` 계산 결과)
- 3: 앱 쪽 hook 함수에 보조 창 viewport의 합성 `RawInput`(마우스 누름/이동)을 넣으면 그 창 터미널 view가 마우스 보고 패킷을 만들고 해당 이벤트를 걸러냄(앱 lib). vendored 호출 경로는 실기
- 4: 보조 창 설정 탭에서 theme preview를 켬 → 보조 viewport 출력의 셸 배경 = preview 색, 같은 frame의 ROOT 출력 = 확정 테마 색, preview를 끄면 보조 창도 복귀(실제 App)
- vendored eframe을 바꿨으므로 동결 remote-web Wasm check(`--features canvas` 포함) 필수

#### 완료 판정

- 묶음 2 뒤 비고 갱신, 상태 partial 유지(묶음 3 범위). 묶음 3 범위가 각 lane에서 검증되면 complete

---

### 2.13 shell-21 전역 키맵 `git`(묶음 3)

- 원본: `src/widgets/app-shell/app-shell.tsx` 143-147, `auxiliary-window-shell.tsx` 96-100. 기본 키 ⌃⇧G, 명령 `view.git`(`src/shared/lib/command-catalog.ts` 92-94). 받는 쪽은 포커스 slot의 sidebar를 Git 보기로 바꾸고 접힌 패널을 펼칩니다. 보조 창은 자기 패널
- native 자산: `toggle-sidebar`·`explorer` 연결 완료(`Run::ShowExplorer` → `ShellIntent::ShowExplorer`/`ShowAuxiliaryExplorer`, 근거 `2026-10-28-native-shell-keymap-dialogs-slot-header.md`). `git`은 키 미소비·명령 비활성(`command-dispatch.rs` 검사가 unavailable로 단언)
- 구현은 git lane 사양 git-1에 있습니다: `Run::ShowGit`(`"git"`, `"view.git"`) → `ShellIntent::ShowExplorerView { project, slot, view: Git }`, 보조 창은 `ShowAuxiliaryExplorerView`. 프로젝트가 없으면 키를 소비하지 않습니다(`PROJECTLESS_KEYMAP_ACTIONS`에 `git` 없음, 원본은 explorer 패널이 없어 요청이 무시됨). 이 문서의 이전 초안에 있던 "프로젝트가 없어도 소비"는 틀린 기술이라 git 사양을 따릅니다.
- shell 쪽 할 일: git lane 병합 뒤 기존 `shell_keymap` App 검사 방식(메인 2 slot, 보조 창)으로 확인된 것을 보고 행을 갱신합니다.
- 완료: git-1 병합·검증 뒤 complete. commands-ui-14와 같은 근거

### 2.14 shell-59 explorer 4보기(묶음 3)

- 원본: `src/widgets/explorer/explorer-panel.tsx` 24-31(순서), 131-173(검색 요청·보기 요청·reveal/rename, 포커스 slot만), 177-200(전환 줄), 202-260(본체), `explorer-container.tsx` 58(mount마다 files)
- native 자산: 전환 줄·slot별 보존·tooltip 완료(근거 `2026-10-28-native-explorer-views-theme-providers.md`). Search·Git 본문은 빈 상태
- 구현: 검색 본문과 ⌘⇧F/⌘⇧H는 explorer-search lane(31·37), Git 본문과 ⌃⇧G는 git lane(git-1)
- 완료: 두 lane 병합 뒤 complete. explorer-search-1·commands-ui-11, git-1과 같은 근거

### 2.15 shell-78 탭 context menu 남은 항목(묶음 3)

- 원본: `src/features/tab/tab-context-menu.tsx` 121-148, `src/widgets/editor-area/pane-tab-bar.tsx` 208-224
    - `tab.openChanges`: 파일 탭이면 `{ kind: 'diff', path, staged: false }` 탭, 제목 `"<탭 제목> (diff)"`, 현재 창 포커스 pane, preview true. 실패는 IPC 오류 toast
    - `git.fileHistory`: `requestOpenFileHistory(path)`(파일 이력 패널 bridge)
- native 자산: 나머지 메뉴 항목 완료(근거 `2026-10-27-native-tab-icons-menu.md`, `2026-10-27-native-auxiliary-window-moves.md`). 두 항목은 소비자가 없어 숨김
- 구현: git lane git-48(변경 열기, diff 탭 본문 포함)과 git-55(파일 이력 패널)
- 완료: 두 항목 연결과 메뉴 검사(git lane) 뒤 complete

### 2.16 shell-92·shell-93·shell-5 묶음 3 잔여

- shell-92 diff 요청: git-49가 `ide_sync`에 대기 diff 등록, `ide_auto_open_diff` 꺼짐 거절(`ide_actions::ide_resolve_diff(rejected)`), `TabKind::ClaudeDiff { request_id, path: new_path }` 탭(제목 tab_name, preview false, pane 없음), 오류 toast, `IdeCloseTabRequested` 등록 제거를 붙입니다. 탭 닫힘의 `TabClosed` 응답은 이미 `tabs::close`에 있습니다. 병합 뒤 shell-92를 complete로 판정합니다.
- shell-93: git-64의 Git 이벤트 소비(상태·gutter·diff 캐시 무효화)가 검증되면 complete
- shell-5: explorer-search-30·D6, git-1(보조 창 Git 보기), terminal-agent-76(보조 창 Task Runner)이 검증되면 complete

## 3. 작업 묶음 계획

### 3.1 묶음 1: provider 소비자(6 ID, wave 1, 먼저)

- ID: shell-93, shell-37, shell-90, shell-91, shell-92(save·진단), shell-96
- 순서(묶음 안 의존)
    1. shell-93: 수신함(`provider-events.rs`), `PaintSink` 필드, `SyncStateChanged` 한 줄, `forget_closed_project` 골격, `consume_provider_events` 호출 지점
    2. shell-37: `forget_closed_project` 완성, `persistence::closing_mirror_job`, peek 재해석
    3. shell-90: 수신함 플래그, host drain, `WaitMarkers`·해제 helper, 실행 인자, 단일 인스턴스
    4. shell-91: 생성자 override(2.0.3), `native-notifications.rs`, `notify_event`, `Command::Notify`(결정 1이 A가 아니면 이 단계를 빼고 5 ID)
    5. shell-92: `ide-sync.rs` save·진단
    6. shell-96: 보조 창 Emmet 검사, 재판정
- 수정 예상 파일
    - 새 앱 모듈: `provider-events.rs`, `external-open.rs`, `external-open-tests.rs`, `single-instance.rs`, `single-instance-tests.rs`, `native-notifications.rs`, `native-notifications-tests.rs`, `ide-sync.rs`
    - 새 App 검사: `application-external-open-tests.rs`, `application-notifications-tests.rs`, `application-ide-sync-tests.rs`
    - 변경(영역): `domains/shell.rs`(새 `Command`·`Reply`, 소비·응답 처리, `forget_closed_project`, `notify_event`), `persistence.rs`, `presentation-refresh.rs`(한 조건), `tabs.rs`(`project_has_file_tab`), `ide-tools.rs`(close 훅), `explorer_delete`·`workspace_delete` 탭 닫기 뒤 훅, `bootstrap.rs`(`LaunchConfig`), `main.rs`(claim·listener 전달)
    - 변경(공용, 추가만): `application.rs`(`PaintSink` 필드, `Overrides`/`new_with_overrides`, 생성 인자, `background_tick` 호출 한 줄, `Saved` 처리 끝 훅), `lib.rs`(모듈 목록), `host.rs`(`HostIntegrations`에 `WaitMarkers` 필드)
    - 검사 추가: `application-project-rail-tests.rs`, `application-emmet-tests.rs` 또는 `application-auxiliary-window-tests.rs`
    - 문서: QA `docs/quality-assurance/<날짜>-native-shell-providers.md`, 대응표 shell-37·90·91·92·93·96 행(결정 1에 따라 terminal-agent-57·59~63·70·73, settings-23 부분)
- 새 의존성: 없음. tokio `net`(UnixListener), `serde_json`, `uuid`, `taide-agent`, `taide-runtime`, `taide-ide`(`taide_ide::store::PendingSave`), `taide-model`이 이미 앱 의존입니다. `crates/*` 공유 코어는 바꾸지 않습니다.
- 동결 remote-web: native-ui와 vendored eframe을 바꾸지 않으므로 Wasm check는 기존 결과 재사용

### 3.2 묶음 2: 창·viewport·표시 층(6 ID, wave 1, 묶음 1 뒤)

- ID: shell-94, shell-22, shell-9, shell-3, shell-98, shell-5(보조 창 표시 4건)
- 순서
    1. shell-94: 시작 대체, 배너 sublayer
    2. shell-22: 재판정(shell-94 검사에 포함)
    3. shell-9: 초기 최근 목록
    4. shell-3: zoom, 종료 코드(vendored eframe `run.rs`)
    5. shell-98: toast 층. 묶음 1이 만든 toast 호출부를 `ToastTarget::Main`으로 지정
    6. shell-5: vendored eframe 창 핸들·raw input hook, 보조 창 WKWebView, 전 창 탭 drag 숨김, theme preview
- 수정 예상 파일
    - 앱 변경: `application.rs`(생성자 대체, `apply_appearances` 공통화, 보조 창 패스 끝 처리, `finish_frame`의 web 활성 판정), `status-error-banner.rs`, `file-drop.rs`(또는 overlay를 그리는 `show_window` 부분), `app-menu.rs`, `window-state.rs`·`window-state-tests.rs`, `main.rs`, `preview_web_view.rs`, `terminal_surface.rs`(raw input 공유 상태, terminal-agent lane과 조율), `toast.rs`, 새 `toast-layers.rs`, `domains/shell.rs`
    - vendored: `experiments/native-shell-spike/vendor/eframe/src/native/run.rs`, `src/native/wgpu_integration.rs`, `src/lib.rs`(공개 함수) — 결정 6
    - 검사: `application-explorer-views-tests.rs`, `application-file-drop-tests.rs`, `application-app-menu-tests.rs`, `application-auxiliary-window-tests.rs`, `application-window-state-tests.rs`
- 새 의존성: 없음. zoom은 `objc2 =0.6.4` `msg_send!`와 `wry::raw_window_handle` 재수출을 씁니다.
    - 대안(비추천): `objc2-app-kit`에 `NSWindow` feature 추가. 새 crate는 없지만 Cargo.toml feature가 바뀌고 lock 영향 확인이 필요합니다.
    - 로거는 결정 3. 추천안은 의존성 없는 최소 `log::Log` 구현입니다.
- 동결 remote-web Wasm check 필수(vendored eframe을 remote-web `canvas` 기능이 씀)

### 3.3 묶음 3: 다른 lane 뒤 판정(6 ID 범위, wave 3)

- ID: shell-21, shell-59, shell-78, shell-5(잔여), shell-92(diff), shell-93(최종 판정)
- 선행: git lane(git-1, git-48, git-49, git-55, git-64), explorer-search lane(31·37, 30·D6), terminal-agent lane(terminal-agent-76)
- 구현은 각 lane 사양에 이미 있으므로 이 묶음은 병합 결과 확인, 빠진 연결(보조 창 경로 등) 보완, 대응표 행 판정입니다. 운영 문서 3절에 따라 wave 3 commands-ui lane의 "진입만 남은 행"으로 넘기고, 2.13~2.16과 2.12의 "묶음 3으로 넘기는 범위"가 그 입력입니다.
- 수정 예상 파일: 보완이 필요할 때만 `domains/shell.rs`, `shell_keymap.rs`, `command-dispatch.rs`, native-ui 셸 영역 모듈(탭 메뉴·sidebar 보기), 검사 파일. 새 의존성 없음 예상

## 4. 결정과 허락이 필요한 항목

### 4.1 사용자(메인) 결정

1. 중복 기능의 소유(1.3 표)
    - 대상: shell-90 = terminal-agent-57 / shell-91 = terminal-agent-59~63 = settings-23(agent·task·LSP 범주, settings S2) / shell-92 save = terminal-agent-70, 진단 = terminal-agent-73, diff = terminal-agent-68·69 = git-49 / 알림 진입점(explorer-search 사양이 terminal-agent-63에 기대함) / `SyncStateChanged` 재로드 = settings S4 한 줄 / 테스트 플랫폼 override = settings 2.5 `Overrides`
    - A안(추천): provider 소비자와 알림 진입점은 shell lane이 묶음 1(wave 1)에서 구현합니다. 순수 알림 로직은 앱 모듈 `native-notifications.rs`에 두고(Cargo·crates 변경 없음), settings 사양 2B settings-23 설계 1의 `crates/taide-notification/src/producer.rs`는 이 모듈로 바꿉니다. terminal-agent lane은 57·59~63·70·73을 범위에서 빼고, settings lane S2는 settings-23을 검증·행 갱신만 하며, explorer-search 사양 1.3의 알림 포트 제공자를 shell lane `notify_event`로 바꿉니다. diff는 git-49 그대로입니다. 이유: 원본에서도 앱 provider 한 곳이 소유하고, wave 1 두 lane과 wave 2 한 lane이 같은 코드를 만들 위험을 없애며, wave 1의 explorer-search·wave 2의 git이 부를 진입점이 먼저 생깁니다.
    - B안: 알림은 settings lane(wave 2)이 settings 사양대로 소유. shell-91은 묶음 1에서 빠지고(5 ID), explorer-search의 검색 치환 알림 연결은 wave 2 뒤로 밀립니다.
    - C안: terminal-agent lane이 57·59~63·70·73을 소유하고 shell lane은 shell-90~92를 재판정만. settings 사양과의 겹침은 따로 정리해야 합니다.
2. 보조 창 toast가 뜰 창(shell-98)
    - A안(추천): 사용자 동작 결과 toast는 만드는 순간 OS focus를 가진 창, 주 창 전용 provider toast는 주 창. 기존 모달 창 규칙(`overlay_viewport`)과 같고 단순합니다. 응답 전에 사용자가 창을 바꾸면 바꾼 창에 뜨는 작은 차이가 있습니다.
    - B안: host 명령마다 출처 창을 기록해 응답 toast를 그 창에 띄움(원본에 더 가깝지만 명령·응답 전체에 출처 필드 추가)
3. 로거(shell-3)
    - A안(추천): 의존성 없는 최소 `log::Log`(stderr, `Warn` 이상)를 `main.rs`에 설치합니다. 시작 실패 원인이 보입니다. 앱 전체 경고가 stderr에 처음 나오는 동작 변화가 있습니다.
    - B안: 그대로 둠(원인 문구 없음, 종료 코드만)
    - C안: `env_logger` 추가(새 의존성)
    - 파일 로그(원본 로그 폴더)는 실제 사용자 경로라 shell-15와 함께 다룹니다.
4. 이전 초안의 "보조 창 복원 규칙 복제 유지 대 `taide-window` 의존" 결정은 이미 해소됐습니다. `taide-window`는 앱 의존이고(`native/taide-native-app/Cargo.toml`, 커밋 a03166b3) `auxiliary-windows.rs`가 원본 함수를 직접 부릅니다. `2026-10-27-native-auxiliary-window.md` 171의 열린 항목만 갱신하면 됩니다(결정 불필요).
5. drop overlay와 Zen 안내·편집기 popup의 그리기 순서(shell-22)
    - A안(추천): 기존 QA가 근거를 적은 의도된 차이로 유지하고, 배치 31 실기에서 보이면 다시 봅니다. overlay를 Foreground로 올리면 대화상자·메뉴 위로 올라가는 더 큰 차이가 생깁니다.
    - B안: Zen 안내·popup을 overlay의 부모 층 아래로 고정하는 층 재설계(sublayer 관계를 frame마다 지정)
6. vendored eframe 수정 범위(shell-3 종료 코드, shell-5 창 핸들·raw input hook)
    - A안(추천): 허용합니다. `render_immediate_viewport` drain 수정 선례가 있고, 동결 remote-web은 `--features canvas` Wasm check로 확인합니다.
    - B안: 금지. shell-3 종료 코드, 보조 창 WKWebView, 보조 창 터미널 마우스 보고가 보류되어 shell-3·shell-5가 partial로 남습니다.

- 이전 초안의 "탐색기 상태 창별 분리를 어느 lane이 하나" 결정은 explorer-search 사양 D6(그 lane의 결정)으로 넘어갔습니다.

### 4.2 허락이 필요한 항목

1. 실제 `taide` CLI → native 연결(shell-90): CLI는 `--data-dir` 없이 `/Applications/TAIDE.app` 또는 `TAIDE_APP_PATH`를 실행합니다. native에 잇는 것은 실제 사용자 데이터 경로(shell-15) 허락 뒤입니다. 그 전에는 격리 data dir과 직접 argv·소켓 중계로만 검사합니다.
2. 실제 OS 알림 전송(shell-91): 자동 검사는 가짜 플랫폼을 넣고 osascript를 부르지 않습니다. 실제 알림 표시와 macOS 알림 권한 확인은 사용자 실기(배치 31)입니다.
3. 단일 인스턴스 소켓(shell-90): 사용자 임시 폴더(`std::env::temp_dir()`)에 `taide-native-<hash>.sock`을 만들고 지웁니다. 사용자 데이터·OS 설정은 아니지만 새 OS 자원이라 적어 둡니다. 다른 위치를 원하면 결정이 필요합니다.
4. 저장소 밖 근거 확인(shell-5 파일 선택 dialog 부모 창): 원본 `tauri-plugin-dialog`가 부모 창을 호출 창으로 정하는지는 저장소 밖 crate 소스에 있습니다. 작업자 규칙상 읽을 수 없어 확인 허락이나 사용자 확인이 필요합니다. 확인 전에는 지금(메인 창 부모)을 유지합니다.
5. 실기(배치 31): shell-3 창 위치·zoom·모니터, shell-9 NSMenu, shell-22 실제 Finder drag·WKWebView 대상 전환, shell-5 보조 창 WKWebView·터미널 마우스·OS 창 focus, shell-94·98 픽셀과 tooltip 지연
