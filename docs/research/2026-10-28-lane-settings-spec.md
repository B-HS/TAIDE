# lane settings 사양 (Rust-native 전환, wave 2)

> 작성일: 2026-10-28 · 대상: 기능 대응표(`docs/quality-assurance/2026-10-09-native-function-matrix.md`) settings 영역의 남은 39개 ID
> 운영 방식: `docs/memory/2026-10-28-native-lane-operations.md`. 이 문서는 lane 순서 1(사양) 산출물입니다. 구현 작업자는 이 문서와 여기서 가리키는 파일 구간만 읽습니다.
> 단계 0(영역 모듈 분리)이 진행 중이라 `application.rs`·`host.rs`(native-app), `commands.rs`·`shell.rs`(native-ui)는 줄 번호 없이 심볼 이름으로만 인용합니다. `docs/research/2026-10-28-native-lane-seams.md`는 작성 시점에 없었습니다. 그 문서가 생기면 아래 영역 이름(`domains/settings.rs`, `HostCommand::Settings`)은 그 문서를 따릅니다.
> 같은 wave·앞 wave의 lane 사양(`2026-10-28-lane-shell-spec.md`, `-explorer-search-spec.md`, `-git-spec.md`)과 겹치는 범위는 1.3절에 소유를 맞춰 두었습니다.

## 0. 약어·검사 표기·실행 명령

| 약어 | 절대 경로 |
| --- | --- |
| `TS` | `/Users/hyunseokbyun/development/TAIDE/src` |
| `TAURI` | `/Users/hyunseokbyun/development/TAIDE/src-tauri/src` |
| `CR` | `/Users/hyunseokbyun/development/TAIDE/crates` |
| `UI` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src` |
| `APP` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src` |
| `TERM` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-terminal/src` |
| `WEB` | `/Users/hyunseokbyun/development/TAIDE/native/taide-remote-web` (동결) |
| `LOC` | `/Users/hyunseokbyun/development/TAIDE/crates/taide-locale/resources/locales/{en,ko,ja}.json` (키는 평탄한 `"settings.xxx"` 형식) |

검사 표기

- `[U]` native-ui 하네스: `UI` 안 `#[cfg(test)]` 모듈. `egui::Context::run_ui(input, |ui| ..)`로 그리고 `settings_view::Output`의 `traces`·`tooltip_traces`로 위치·클릭을 단언합니다. 선례: `UI/theme-editor-tests.rs`, `APP/settings-view-tests.rs`(`context.run_ui`).
- `[A]` native-app lib 실제 App: `NativeApplication`을 `eframe::CreationContext::_new_kittest`로 만들고 `eframe::App::logic`/`ui`를 `eframe::Frame::_new_kittest()`로 돌립니다. 선례: `APP/application-project-rail-tests.rs`의 `Fixture`·`isolated_state`·`launch`, `APP/application-startup-tests.rs`, `APP/application-auxiliary-window-tests.rs`(보조 viewport), `APP/settings-integrations-tests.rs`(가짜 `SecretStore`).
- `[C]` crate 단위: `CR/<crate>` 또는 `APP`·`UI` 안의 순수 함수 테스트.

실행 명령(모두 `CARGO_TARGET_DIR=<worktree>/target-lane`)

- 앱 lib: `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline -- <필터> --test-threads=4`
- UI lib: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --locked --offline -- <필터>`
- 공유 crate: 루트 workspace에서 `cargo test -p <crate> --locked --offline`. 공유 crate를 바꾼 묶음은 `cargo check -p taide --locked --offline`(src-tauri 컴파일 확인, src-tauri 소스는 바꾸지 않음)
- 터미널 crate: `cargo test --manifest-path native/taide-native-terminal/Cargo.toml --locked --offline -- <필터>`
- 동결 미러: `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline`(UI crate나 `taide-model`을 바꾼 묶음마다)
- 형식: 바꾼 crate마다 `cargo fmt --manifest-path <crate>/Cargo.toml -- --check`

금지(모든 검사): 실제 OS 알림 전송, 시스템 설정 앱 열기, 실제 클립보드, Keychain, `/usr/local/bin` 쓰기와 관리자 프롬프트, 실제 사용자 홈(`~/.codex` 등)·실제 프로젝트 파일 쓰기, 네트워크(GitHub·Codex·ollama·LSP 다운로드), 실제 파일 선택 창, GUI 실행. 모두 2.7의 주입 포트로 가짜를 씁니다(4.2 허락 목록).

## 1. 요약

### 1.1 수와 상태

| 현재 판정 | 수 | ID |
| --- | --- | --- |
| unwired | 28 | 16, 17, 18, 21, 22, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54 |
| partial | 10 | 3, 5, 23, 24, 28, 30, 67, 78, 80, 83 |
| missing | 1 | 82 |
| 합계 | 39 | |

일의 성격으로 다시 나누면 다음과 같습니다.

| 성격 | ID | 수 |
| --- | --- | --- |
| 새 화면·동작 구현 | 16, 17, 18, 21, 22, 32~47, 49~54, 82 | 28 |
| 다른 lane 산출물 위에 연결·확인 | 3, 5, 23, 48, 80 | 5 |
| 기존 소비자 증거 검사와 작은 보완 | 24, 28, 30, 67, 83 | 5 |
| 사용자 결정 반영 | 78 | 1 |
| 합계 | | 39 |

### 1.2 핵심 판단

1. 원본 Settings는 14 section(13 + 조건부 Performance)입니다(`TS/widgets/settings-view/settings-view.tsx:61-92,263`). native는 8개(`Section::BASIC`, `UI/settings-controls.rs:30`)만 있습니다. 빠진 LSP·AI·Plugins·Sync·Remote·Performance section과 Interface의 CLI·hooks·IDE 행, Notifications의 버튼 2개·DEV 안내가 unwired·missing의 대부분입니다.
2. 백엔드는 거의 공유 crate에 있습니다: `CR/taide-runtime/src/{agent_hook_actions,agent_host,notification_actions,lsp_actions,ai_actions,plugin_actions,vsix_actions,sync_actions,remote_actions}.rs`. 원격 dispatch(`APP/remote-agents.rs`, `remote-ai.rs`, `remote-lsp.rs`, `remote-plugins.rs`, `remote-sync.rs`)가 같은 함수를 이미 부르므로 조립 방식을 그대로 본뜹니다. 빠진 것은 로컬 UI, 호스트 명령, 응답 소비자, 테스트 주입 포트입니다.
3. 새로 이식할 큰 덩어리는 VSIX 테마 변환(`TS/shared/lib/theme-convert/*` 비테스트 약 2,570줄 + `TS/shared/lib/color.ts` 199줄) 하나입니다(settings-42). `CR/taide-theme/src/service.rs` 테스트 모듈이 대비 계산 일부(`hex_to_rgb`, `relative_luminance`, `contrast_ratio`, `delta_e76`, `foreground_contrast_ratio` 등)와 쌍 표를 이미 미러하고 있어 재사용합니다.
4. OS 알림 생산자(agent·명령·LSP)와 첫 전달 안내, `SyncStateChanged` 뒤 테마·로케일 재로드는 wave 1 shell lane이 만듭니다(shell spec 2.1, 2.5). settings lane은 그 포트를 재사용하고(22) 행을 확인·갱신합니다(23, 48). 같은 기능을 두 번 만들지 않습니다.
5. `WEB`(원격 미러, 동결)는 `taide_native_ui::settings_view::Views::show`와 `settings_controls::Change`를 그대로 씁니다(`WEB/src/browser-editor.rs`, `WEB/src/preferences.rs`). UI crate를 `default-features = false`(= `native-host` 없음)로 쓰므로 새 UI 모듈은 `taide-model`·`egui`만으로 컴파일돼야 합니다. 기존 공개 시그니처는 바꾸지 않고, 새 section은 native 앱이 호스트 컨텍스트를 넘길 때만 그립니다(4.1 결정 1).
6. 새 외부 의존성은 필요 없습니다. 날짜 표시는 앱 crate의 기존 `chrono`·`sys-locale`, 파일 선택은 기존 `rfd`, 변환 이식은 `serde_json`만 씁니다(`taide-theme`의 `regex`는 dev-dependency라 쓰지 않습니다).

### 1.3 다른 lane 의존

| 의존 | 산출물 | 대상 ID | 처리 |
| --- | --- | --- | --- |
| 단계 0(필수) | `APP/domains/` 영역 모듈, `HostCommand::<영역>(domains::<영역>::Command)`·`HostReply::<영역>(..Reply)` 패턴(현재 `domains/commands.rs`, `domains/shell.rs`가 선례) | 전부 | lane 시작 조건. settings 기존 명령(`ReadSettingsCatalog`, `ReadSettingsResource`, `UpdateSettings`, `OpenSettings`, `OpenSettingsFolder`, `ThemeEdit`, `SnippetEdit`, `ReadSnippetCatalog` 등)이 `domains::settings`로 옮겨졌으면 그 enum 끝에 추가, 아니면 새 파일을 만들고 루트에 `Settings` 변형을 추가합니다 |
| shell(wave 1) shell-91 | `crate::native_notifications`(평가·본문·자르기·경과 문구), `ShellCommand::Notify(native_notifications::Request)`·`NotificationSent`, 주입 가능한 알림 platform 포트, 첫 전달 안내 | 22, 23 | 22는 같은 포트로 테스트 알림을 보냅니다(2B). 23의 agent·명령·LSP 범주는 shell-91 근거로 행 갱신만 합니다 |
| shell(wave 1) shell-93 | `presentation_refresh::Changes::record`가 `SyncStateChanged`에서 `revision`·`themes`를 올림 | 48 | 테마·로케일 재로드와 Settings 카탈로그 무효화(`Views::observe_theme_revision`)는 shell 산출물로 충족됩니다. settings lane은 동기화 상태 캐시 교체와 App 검사만 합니다 |
| shell(wave 1) shell-5 | 보조 창 수명(이동·렌더·닫힘 복귀) | 5 | App 검사만 추가합니다 |
| explorer-search(wave 1) | explorer-search-25(자동 reveal), -34(searchOnType·debounce), -33(검색 치환 알림) | 80, 23 | 병합 상태에서 App 증거 검사 1개씩 추가하고 행 갱신. 미병합이면 비고에 의존으로 남깁니다 |
| terminal-agent(wave 1) | 터미널 hub·세션 파일 정리(사양 미작성 상태) | 30 | settings lane이 `TERM`·`terminal_host.rs`에 scrollback 변경 API를 "추가만" 합니다(4.1 결정 7) |
| editor+preview(wave 1) | semantic tokens 소비자(사양 미작성 상태) | 28 | 없으면 의존으로 남깁니다 |
| git(wave 2, 동시) | git-7 push/pull OS 알림, git-49 IDE diff 자동 열기(`ideAutoOpenDiff`), diff 설정 2종 소비 | 23, 80, 28 | git spec 결정 1 A안대로 git lane 소유. settings lane은 손대지 않고, 메인 통합 뒤 settings 행 비고를 갱신합니다 |
| commands-ui(wave 3) | `sync.*`, `cli.*` 팔레트 명령(commands-ui-30~34)과 남은 명령 소비자 | 67 | settings lane은 override 동등성 검사만. 도메인 명령(`Command::CliInstall` 등)을 `pub(crate)`로 두어 재사용하게 합니다 |
| 사용자 결정 | status 라벨 잔여 처리(2026-10-06부터 미결, `docs/acknowledge/2026-10-06-native-transition-decisions.md` 61줄) | 78 | 4.1 결정 3 |

### 1.4 lane 소유 파일

- 새로 만듦(소유): `APP/domains/settings.rs`(단계 0이 이미 만들었으면 확장), `APP/settings-ports.rs`, `APP/settings-sync-time.rs`, `UI/settings-sections-data.rs`, `UI/settings-sections-view.rs`, `UI/settings-dialogs.rs`, `CR/taide-theme/src/convert/*`, 각 검사 파일, `docs/quality-assurance/<날짜>-native-settings-lane.md`.
- 고침(소유): `UI/settings-controls.rs`, `UI/settings-view.rs`, `UI/settings-code-view.rs`(도우미 가시성만), `UI/icons.rs`, `UI/theme-draft.rs`(`unique_id` 일반화), `APP/app-file-views.rs`, `CR/taide-runtime/src/{agent_host,app_actions,vsix_actions}.rs`, `CR/taide-theme/src/lib.rs`, `CR/taide-model/src/remote.rs`, `CR/taide-remote/src/types.rs`(재수출), `CR/taide-settings/src/service.rs`(옮긴 함수 호출).
- 추가만(공용): `APP/application.rs`(`PaintSink` 필드 1개와 `record` 1줄, 응답 match 1줄, Settings `tab_content` 분기의 호출 교체, 생성자 overrides), `APP/host.rs`(루트 변형·dispatch 1줄), `APP/main.rs`(`perf::init` 1줄), `APP/file-dialogs.rs`(열기 picker 타입·함수), `APP/lib.rs`·`UI/lib.rs`(모듈 목록), `APP/resources/icons/*.svg`, `TERM/lib.rs`·`TERM/session.rs`·`APP/terminal_host.rs`(scrollback API, 결정 7).
- 건드리지 않음: `TAURI/**`, `WEB/**`(lockfile 포함), `LOC`(필요 키는 모두 있음), `APP/presentation-refresh.rs`(shell lane 소유).

## 2. 공통 설계 (모든 묶음이 공유)

### 2.1 영역 경계

- `APP/domains/settings.rs`는 단계 0 선례(`APP/domains/commands.rs`)와 같은 꼴입니다: `pub enum Command`, `pub enum Reply`, `pub(crate) struct State`, `pub(crate) async fn dispatch(worker: &HostWorker<'_>, command: Command) -> Option<HostReply>`, `impl NativeApplication { pub(crate) fn apply_settings_reply(&mut self, context, reply) .. }`. 루트는 `HostCommand::Settings(domains::settings::Command)`·`HostReply::Settings(domains::settings::Reply)`이고 `host.rs` dispatch와 `application.rs` 응답 match에 한 줄씩 추가합니다.
- `HostWorker`가 주는 것: `services`, `clipboard: &ClipboardWriter`, `sender`, `repaint`, `integrations: &HostIntegrations`(`reconcile: Option<remote_preferences::Reconcile>` 포함). 설정 패치는 기존 `crate::host::update_control_settings(services, reconcile, patch)`를 씁니다(`domains::commands`의 `SetKeymapOverrides`가 선례).
- 실행 규칙: 짧은 동기 작업은 `services.tasks.run_blocking_result`, 설치·네트워크·쓰기는 `run_nonabortable_result`(선례 `APP/remote-sync.rs` `sync_upload`·`sync_download` 분기). 호스트 큐가 가득 차 제출이 실패하면 기존 패턴처럼 그 행동의 pending을 풀고 오류 toast를 냅니다.

### 2.2 UI 진입점과 미러 게이트

- `UI/settings-view.rs`의 `Views::show(ui, owner, settings, locale, appearance) -> Output`(433줄)은 시그니처를 유지합니다(`WEB` 호출). 새 진입 `Views::show_with_host(ui, owner, settings, locale, appearance, host: settings_sections_view::Host<'_>) -> Output`을 추가하고 `show`는 "호스트 없음"으로 위임합니다.
- `Host<'_>`는 native 앱이 프레임마다 채웁니다: `data: &mut settings_sections_data::Data`(2.4), `projects: &[ProjectRef]`(hooks 목록, `ShellSnapshot::projects` = 원본 `projectListQueryOptions`), `is_macos: bool`(`cfg!(target_os = "macos")`, 테스트에서 바꿀 수 있게 값으로), `is_dev_build: bool`(`cfg!(debug_assertions)`), `perf_enabled: bool`(2.7 `perf` 레지스트리의 `is_enabled()`), `remote_password_min_len: usize`(2C의 51), `last_synced_display: Option<String>`(46, 앱이 형식화).
- 호스트가 없으면(미러) 화면은 지금과 같습니다. 있으면 Interface 행 3개 삽입, Notifications 버튼 묶음·DEV 안내, 새 section 6개를 그립니다.
- `WEB` 무변경 확인: UI crate를 바꾸는 묶음마다 동결 미러 Wasm check. `Change`에 변형을 추가해도 `WEB`은 `Change::Theme`·`KeymapOverrides`만 개별 매칭하므로 컴파일에 영향이 없습니다(`WEB/src/preferences.rs:12`, `browser-editor.rs:229`).

### 2.3 Section 확장

- `UI/settings-controls.rs` `Section`에 `Lsp, Ai, Plugins, Sync, Remote, Performance`를 추가합니다. `Section::BASIC`(8개)은 유지하고 `Section::HOSTED: [Self; 5]`(Lsp, Ai, Plugins, Sync, Remote)를 추가합니다. Performance는 `host.perf_enabled`일 때만 TOC 끝과 카드 끝에 붙습니다(`settings-view.tsx:190-193,263`).
- 제목 키(`settings-view.tsx:87-92`): Lsp `settings.lspStatus`, Ai `settings.aiSectionTitle`, Plugins `settings.plugins`, Sync `settings.syncSectionTitle`, Remote `remote.title`, Performance `settings.performance`.
- 카드 설명: `Section::description() -> Option<&'static str>`를 추가합니다. Keymap `settings.keymapDescription`, Lsp `settings.lspDescription`, Remote `remote.description`, Performance `settings.performanceDescription`. 지금 `View::show`의 `if section == Section::Keymap` 분기를 이 일반 규칙으로 바꿉니다(사용처 4곳). 원본 카드: `TS/features/settings/settings-section.tsx:10-18`(CardTitle `text-sm`, CardDescription muted, 본문 gap 12px).
- TOC와 카드 루프는 같은 `sections(host: Option<&Host>) -> Vec<Section>`을 돕니다(지금은 둘 다 `Section::BASIC`).

### 2.4 데이터 캐시 `UI/settings-sections-data.rs`

- 이름 주의: 앱 crate에 이미 `crate::settings_integrations::Integrations`(토글 재조정, `APP/settings-integrations.rs`)가 있어 UI 새 모듈은 `settings_sections_data`로 이름을 나눕니다.
- 앱이 `Data` 하나를 소유하고 모든 Settings owner(메인·보조 창)가 공유합니다. 원본은 창마다 QueryClient가 따로지만 native는 프로세스가 하나라 공유가 상위 호환입니다.
- 조회 `Query`와 신선도(원본 기본 `staleTime` 60초, `gcTime` 600초, `retry` 0, 창 focus 재조회 없음: `TS/app/query-client.ts:4-15`). 기존 `UI/settings-resources.rs`의 `Cache`(`observe`/`unobserve`, `RESOURCE_GC_TIME` 600초)를 같은 방식으로 확장합니다. "mount"는 새 owner View 생성 또는 테마·스니펫 편집기에서 돌아온 순간(원본 section 재mount)입니다.

| Query | 원본 | 신선도·무효화 |
| --- | --- | --- |
| `CliStatus` | `agent.query.ts:23` | 60초. 설치·해제 응답 상태로 교체(`agent.query.ts:23-35`) |
| `HooksStatus { project, agent }` | `agent.query.ts:39-54`, user 범위는 project `""`(37) | 60초. 설치·해제 응답으로 교체 |
| `LspServers` | `lsp.query.ts:11` `staleTime: Infinity` | 부팅당 1회. 설치 성공 때만 무효화 |
| `AiTokens` | `ai.query.ts:8` | 60초. 토큰 저장·해제 성공 시 무효화 |
| `AiModels(provider)` | `ai.query.ts:10-16` `staleTime: Infinity`, configured일 때만 | 토큰 저장·해제 성공 시 그 provider 무효화. oMLX base URL 변경 시 `AiTokens`·`AiModels(Omlx)` 무효화(원본 누락 보정, 36) |
| `Plugins` | `plugin.query.ts:10` | 60초. reload·uninstall 응답 목록으로 교체, install·VSIX 문법 성공 시 재조회 |
| `SyncStatus` | `sync.query.ts:5` | 60초. connect·disconnect·upload 응답, download `applied` 응답, `SyncStateChanged`로 교체 |
| `RemoteStatus` | `remote.query.ts:9-10` `refetchInterval` 5초 | 관찰 중 5초 재조회. `RemoteStateChanged`로 교체, 비밀번호 설정·해제 성공 시 무효화, `remoteAccessEnabled` 변경(settings revision) 시 무효화(`ipc-sync-provider.tsx:418-419`) |

- 각 항목은 `Loading | Ready(T, at) | Failed(AppError, at)`입니다. "조회 중"(원본 `isPending`)은 `Loading`이고, 실패 값은 다음 mount에 다시 조회합니다.
- 행동 `Action`(UI → 호스트, 1회성): `CliInstall`, `CliUninstall`, `HooksInstall { project, agent }`, `HooksUninstall { project, agent }`, `LspInstall(id)`, `LspCancel(id)`, `AiSetToken { provider, token: Secret }`, `AiClearToken(provider)`, `PluginReload`, `PluginUninstall { id }`, `PickFile(PickPurpose)`, `PluginInstall(PathBuf)`, `VsixExtract(PathBuf)`, `VsixSaveThemes(Vec<Theme>)`, `VsixImportGrammars(PathBuf)`, `SyncConnect(Secret)`, `SyncDisconnect`, `SyncUpload`, `SyncDownload { force }`, `RemoteIssueLink`, `RemoteRevoke`, `RemoteSetPassword(Secret)`, `RemoteClearPassword`, `PerfRead`, `PerfReset`, `SendTestNotification`, `OpenNotificationSettings`, `CopyInstallHint(String)`.
- pending: 원본은 행·버튼마다 mutation 인스턴스가 있어 `isPending`이 그 단위입니다. `Data`는 행동 식별 키(예: `HooksInstall(project, agent)`, `AiSetToken(provider)`, `Sync(Upload)`)별 pending 집합을 두고 응답 때 지웁니다. AI 토큰 저장 pending은 원본처럼 "마지막으로 보낸 provider"에만 걸립니다(`settings-ai-section.tsx:43,71,79,86`).
- 설정 패치: 새 값은 `UI/settings-controls.rs` `Change`(493줄)에 `Change::Hosted(HostedChange)` 변형을 추가해 기존 `HostCommand::UpdateSettings` → `update_control_settings` → `Integrations::reconcile`(`APP/settings-integrations.rs`) 경로를 그대로 탑니다. `HostedChange`: `AgentHooks(bool)`, `IdeIntegration(bool)`, `AiProvider(AiProviderId)`(패치 `{ai_provider, ai_model: ""}`), `AiModel { provider, model }`, `AiAutoTab { provider, enabled }`, `AiOmlxBaseUrl(String)`(trim), `RemoteAccess(bool)`, `RemotePasswordOnly(bool)`, `RemoteAllowedHosts(Vec<String>)`. `Switch::ALL`에는 넣지 않습니다(미러 화면 불변).
- 원본은 섹션이 사라진 뒤 끝난 mutation의 호출별 콜백(toast)을 버립니다(TanStack v5 `mutate` 콜백 규칙). 결과를 사용자가 못 보는 결함이라 재현하지 않고 응답 toast는 항상 냅니다.

### 2.5 owner 상태와 mount 규칙

- `UI/settings-view.rs`의 retained `View`에 `host: settings_sections_view::OwnerState` 한 필드를 추가합니다.
- 컨테이너 수준(원본 `SettingsView` 상태, 편집기 전환 중에도 유지): 활성 TOC(기존), `issued_remote_url`(53), `sync_conflict_open`(47).
- section 수준(원본 section 컴포넌트 상태, 테마·스니펫 편집기가 열리면 사라짐): 입력 draft(AI 토큰 2개·oMLX 키·oMLX URL·Sync PAT·원격 비밀번호·허용 호스트), hooks 동의 대화상자 대상, 플러그인 제거 대상, VSIX 대화상자, Performance 읽기 결과, 폰트 picker 필터(기존). `Views::show`에서 `output.open`·`open_snippets`로 편집기가 열릴 때 `OwnerState::reset_section_state()`를 부릅니다.
- owner가 사라지면(`finish_frame`의 `retain(seen)`) 모두 사라지고, 다시 보이면 새 mount입니다(원본 컴포넌트 재mount와 같음).

### 2.6 공통 UI 규칙

- toast: `Toasts::{success, error, warning, info}`, `Toasts::ipc_error`, `toast::describe_error`(`UI/toast.rs` 515~587, 1589). 원본 `describeIpcError(error) || t(fallback)`은 `describe_error`가 빈 문자열이면 fallback 키로 바꿉니다.
- 대화상자: 새 `UI/settings-dialogs.rs`에 두 종류를 둡니다. 구현은 `UI/modal.rs`(`Layer`, `trap_focus`, `FocusReturn`, `Composition`, `takes_escape`)와 `UI/snippet-editor.rs`·`APP/delete_dialog.rs`의 선례를 따릅니다.
    - `AlertDialog`(원본 Radix AlertDialog): 제목, 설명, 버튼들. 바깥 클릭으로 닫히지 않고 Escape만 취소입니다. 확인 버튼(Radix `AlertDialog.Action`)은 누르는 즉시 대화상자를 닫고 동작을 시작합니다(원본 `onClick`이 `preventDefault`를 하지 않음). 크기 `sm`(max-w xs, 버튼 2열 격자)과 기본(max-w lg) 두 가지. 사용처: hooks 동의(17, 기본), 플러그인 제거(44, sm), VSIX 덮어쓰기 확인(42, sm), 동기화 충돌(47, sm).
    - `Dialog`(원본 Radix Dialog): 오른쪽 위 X 닫기 버튼(`common.close` 접근 이름), 바깥 클릭·Escape 닫기, IME 조합 중 Escape는 닫지 않음(`TS/shared/ui/dialog.tsx:42-71`). 사용처: VSIX 가져오기(42).
- 비밀값: `Secret(String)` newtype(`Debug`는 `Secret(<redacted>)`). 입력 draft는 제출 즉시 비웁니다(원본 `setToken('')`). 로그에 남기지 않습니다(`security.md` §6).
- 아이콘: 기존 `UI/icons.rs` `Icon::{Copy, Pencil, Trash, FolderOpen, Loader, Warning, Close}`를 쓰고, 기존 리소스 `APP/resources/problems/circle-check.svg`·`circle-x.svg`를 `Icon::CircleCheck`·`Icon::CircleX`로 등록합니다. 새 SVG 4개(`external-link`, `bell`, `import`, `refresh-cw`)는 `/Users/hyunseokbyun/development/TAIDE/node_modules/lucide-react/dist/esm/icons/<이름>.mjs`의 path 데이터로 만들어 `APP/resources/icons/`에 넣고 같은 폴더 `LICENSE.txt`를 따릅니다(의존성 추가 아님).
- 색 토큰(원본 Tailwind → native 테마 키, 모두 `CR/taide-theme/src/service.rs` 필수 키): `text-app-sidebar-icon-agent-running` → `appSidebar.iconAgentRunning`, `text-app-sidebar-badge` → `appSidebar.badge`, `text-status-warning|error|success` → `statusIndicator.warning|error|success`, `bg-panel-input-background`·`border-panel-input-border` → `panel.inputBackground`·`panel.inputBorder`, `border-modal-separator` → `modal.separator`, muted 글자 → 기존 `Appearance::muted`.
- 입력: 비밀번호형은 `egui::TextEdit::singleline(..).password(true)`. 원본 `TextField`는 blur 때 commit합니다(`TS/features/settings/text-field.tsx:22-37`). 기존 `UI/settings-code-view.rs` `text_field`(372줄)의 그리기·`TextDraft` 동기화 부분을 라벨·placeholder·저장값을 인자로 받는 `pub(crate)` 도우미로 나눠 재사용합니다(`Text` enum에는 변형을 추가하지 않습니다).
- 선택 상자: `UI/settings-code-view.rs` `Picker`(540줄, `pub(crate)`)를 재사용합니다. 원본 `OptionPicker`는 값이 목록에 없으면 placeholder를 muted로 보입니다(`TS/features/settings/option-picker.tsx:19-56`). 트리거 폭 128px, ArrowDown으로 열림.
- app-file 열기: `APP/app-file-views.rs` `settings_command(project, layout, scope)`(317줄)를 `open_command(project, layout, scope, target: AppFileTarget, title: String)`로 일반화합니다(settings.json과 프롬프트 3종). 보조 창이면 그 창의 focused pane, 아니면 `layout.focused_pane`(원본 `TS/entities/layout/layout.query.ts:111-125` `useOpenAppFileTab`, preview false, 실패 시 `toast.error(describeIpcError)`).

### 2.7 테스트 주입 포트 `APP/settings-ports.rs`

- `Ports`의 기본값은 실제 구현, 검사는 가짜를 넣습니다. `NativeApplication`에 `new_with_overrides(.., Overrides { secret_store, settings_ports })`를 추가하고 기존 `new_with_secret_store`는 이것으로 위임합니다(`Arc::get_mut(&mut assembly.services)`로 `secrets`를 바꾸는 기존 방식과 같음).
    - `agents: Arc<crate::remote_agents::Ports>`: 기존 구조(`cli_status`, `resolve_home`, `project_emitter`, `cli_target`)를 그대로 재사용합니다(`APP/remote-agents.rs:35-73`).
    - `cli: CliPorts { install, uninstall }`: 기본은 16의 runtime 함수 + `/usr/bin/osascript` 실행기와 `std::env::current_exe()`.
    - `open_notification_settings: Arc<dyn Fn() -> AppResult<()> + Send + Sync>`: 기본은 macOS에서 `/usr/bin/open -- x-apple.systempreferences:com.apple.Notifications-Settings.extension`(앱 상수 `MACOS_NOTIFICATION_SETTINGS_URL`, 원본 `TAURI/constants.rs:24`), 그 밖 OS는 `AppError::localized(InvalidArgument, "error.notification.settingsUnsupported", ..)`(`TAURI/domain/notification/commands.rs:53-67`). `NativePlatform::open_url`은 `platform_url`이 http(s)·file만 허용하므로(`APP/bootstrap.rs`) 넓히지 않습니다.
    - 알림 전송: shell-91이 만든 주입 platform을 그대로 씁니다(2B의 22). 별도 포트를 만들지 않습니다.
    - `gist: Gist`: `SyncGistPort`는 `impl Future`라 객체화되지 않습니다. `enum Gist { Http, #[cfg(test)] Fake(Arc<FakeGist>) }`에 `SyncGistPort`를 구현하고 `Http`는 `taide_runtime::sync_gist_http::SyncGistHttpPort`를 감쌉니다(`APP/remote-sync.rs` `Ports<G>`는 제네릭이라 그대로 쓸 수 없음).
    - `pick_file: OpenPicker`(`APP/file-dialogs.rs`에 추가): `Arc<dyn Fn(&eframe::Frame, OpenRequest) -> Option<PathBuf> + Send + Sync>`, 기본 `rfd::FileDialog::new().set_parent(frame).add_filter(name, exts).set_title(title).pick_file()`. 기존 `SavePicker`·`save_picker` 필드와 같은 꼴입니다.
    - `perf: &'static PerfRegistry`: 기본 `taide_infra::perf::global()`, 검사는 `Box::leak(Box::new(PerfRegistry::new()))`에 `set_enabled(true)`.
    - `path_var: Arc<dyn Fn() -> OsString>`: LSP 감지 PATH(기본 `std::env::var_os("PATH")`, 선례 `APP/remote-lsp.rs` `Ports`).
    - `lsp_install: Arc<dyn Fn(Arc<AppServices>, LspServerId) -> BoxFuture<'static, AppResult<()>>>`: 기본은 `lsp_actions::lsp_install(services.events, &state, &services.lsp_install, &tasks, id)`.
- 클립보드는 기존 호스트 `ClipboardWriter` 주입을 씁니다(`write_clipboard(services, clipboard, task, text)`, `APP/host.rs`).

### 2.8 이벤트 수신

- shell lane의 provider 수신함(shell spec 2.0.1)은 LSP `Done`/`Failed`만 보관하고 Sync·Remote를 보관하지 않습니다. settings는 자기 기록기를 둡니다: `APP/application.rs` `PaintSink`에 `settings: Arc<domains::settings::Events>` 필드와 `publish`의 `self.settings.record(&event)` 1줄.
- `Events`가 보관하는 것(모두 "마지막 값만", 상한 불필요): `LspInstallProgress`(서버별 마지막 진행, 모든 phase), `SyncStateChanged { status }`(마지막 상태), `RemoteStateChanged { status }`(마지막 상태).
- 소비: `NativeApplication::background_tick`에서 host 응답 처리 뒤 `domains::settings::consume_events`가 `Data`를 갱신합니다(33, 46, 48, 50). Settings가 닫혀 있어도 갱신합니다(원본은 컨테이너 mount 중에만 진행을 받지만 표시 차이가 없는 상위 호환).
- `remoteAccessEnabled` 변경 감지는 `presentation_changes.settings_revision()`이 바뀐 프레임에 설정값을 이전 값과 비교해 `RemoteStatus`를 무효화합니다.

## 2A. Interface section 행 (16, 17, 18)

원본 순서(`TS/widgets/settings-view/settings-interface-section.tsx:36-137`): toast 위치, 분할 두께, 시스템 사용량, 미니맵, 상태 배지, **CLI 행(67)**, **hooks 토글과 목록(68-76)**, **IDE 통합(77-82)**, IDE diff 자동 열기, 미리보기 탭, 탐색기 reveal, 환영 화면, zen 2종, 검색 그룹.
native 현재(`UI/settings-view.rs` `View::interface`, 1514줄): 위치, 두께, 그리고 `Switch::ALL` 중 Interface 항목(SearchOnType 제외)을 한 번에 순회합니다. 호스트가 있을 때 `Switch::AgentStatusBadge` 다음에 세 행을 끼우도록 순회를 둘로 나눕니다.

### settings-16 Interface: agent CLI(`taide`) 설치/해제 행 (unwired)

- 원본: `TS/widgets/settings-view/agent-cli-status-row.tsx:9-53`
    - macOS가 아니면 행 전체를 그리지 않습니다(16).
    - 왼쪽 세로 묶음(gap 2px): 제목 `keymap.category.shellCommand`(foreground). 조회가 끝났고 `installed`이면 아래 줄: `dangling`이면 `settings.cliStatusDangling`(warning), 아니면 `settings.cliStatusInstalled`(muted)(31-38).
    - 오른쪽(gap 6px, outline xs): `!status?.installed || dangling`이면 `settings.cliInstallButton`, `status?.installed`이면 `settings.cliUninstallButton`(40-49). 조회 중에는 `status`가 없어 설치 버튼만 보입니다. 각 버튼은 자기 동작 진행 중 disabled.
    - 결과 toast: 설치 성공 `settings.cliInstallSuccess`/실패 `settings.cliInstallFailed`, 해제 성공 `settings.cliUninstallSuccess`/실패 `settings.cliUninstallFailed`(18-27, 실패는 describe 없이 고정 문구). 성공 응답 상태로 캐시를 덮습니다.
    - 백엔드(`TAURI/domain/agent/commands.rs`): 상태 = `cli_install_status("/usr/local/bin/taide")`. 설치: 실행 파일이 `.app` 번들 안이 아니면 "only available in the installed TAIDE.app" 오류, 이미 같은 대상을 가리키면 osascript 생략, 아니면 관리자 권한 osascript `ln -sf`; 사용자 취소는 오류가 아님. 해제: 번들 게이트 → 없으면 현재 상태 반환 → TAIDE 소유 symlink가 아니면 거절 → `remove_file`, 권한 거부면 osascript `rm`.
- native 자산: `CR/taide-runtime/src/agent_host.rs:100` `cli_install_status(target)`, `CR/taide-agent/src/service.rs` `resolve_cli_install_target`(195), `build_cli_install_apple_script`(215), `build_cli_uninstall_apple_script`(229), `build_osascript_args`(239), `is_osascript_user_cancelled`(246), `is_cli_symlink_owned`(252), `APP/agent-hooks.rs:24` `TAIDE_CLI_TARGET_PATH`, `remote_agents::Ports::cli_status`.
- 빠진 것: 행 UI, 설치·해제 실행, 결과 toast.
- 설계:
    1. `CR/taide-runtime/src/agent_host.rs`에 `cli_install(exe: &Path, target: &Path, run_osascript: impl FnOnce(&str) -> AppResult<()>) -> AppResult<CliInstallStatus>`와 `cli_uninstall(exe, target, run_osascript)`을 추가해 Tauri 명령의 macOS 분기를 그대로 옮깁니다(Tauri 파일은 수정하지 않음). 함수 안에 `cfg`를 두지 않고 비 macOS 거절은 앱 포트 기본값에서 원본 문구로 합니다.
    2. `domains::settings`: `Command::Query(Query::CliStatus)` → `ports.agents.cli_status`, `Command::Act(Action::CliInstall|CliUninstall)` → `ports.cli.install|uninstall`(`run_nonabortable_result`).
    3. `UI/settings-sections-view.rs` `cli_row`: 위 분기·버튼·색. text-xs, gap 12px.
- 검사: `[C]` runtime(임시 디렉터리의 target·가짜 exe 경로·기록 실행기): 번들 밖 → 원본 오류, 같은 대상 → 실행기 미호출, 취소(실행기가 Ok) → 상태 반환, 해제 소유 아님 → 거절, 없음 → 현재 상태. `[U]` 상태 4종(조회 중, 미설치, 설치됨, dangling)의 문구·색·버튼 노출과 disabled, `is_macos=false`면 행 없음. `[A]` 가짜 `CliPorts`로 버튼 → 호스트 동작 1회 → 성공/실패 toast 문구, 응답 상태로 버튼 전환.
- 완료 판정: 위 검사 통과. 실제 `/usr/local/bin` 쓰기·관리자 프롬프트는 4.2 허락 항목.

### settings-17 Interface: agent hooks 토글, 에이전트 5종, 프로젝트별 설치/해제, 동의 대화상자, CLI 누락 경고 (unwired)

- 원본:
    - 토글: `TS/features/settings/agent-hooks-toggle.tsx:12-20`(라벨 `settings.agentHooks`, 힌트 `settings.agentHooksHint` muted, 기본 off, `settings-interface-section.tsx:69-74`). 켜져 있을 때만 목록(75). 목록 데이터는 프로젝트 목록 전체(`projectListQueryOptions`, 31).
    - 목록 `TS/widgets/settings-view/agent-hooks-project-list.tsx:29-35,106-116`(gap 12px): claude(project 범위), codex·gemini·opencode·pi(user 범위), 라벨 `settings.agentHooksAgent{Claude,Codex,Gemini,Opencode,Pi}`.
    - claude 영역(42-59): 라벨 muted, 프로젝트 0개면 `app.noProjectOpen`(muted), 있으면 프로젝트별 행(gap 6px).
    - 프로젝트 행 `TS/widgets/settings-view/agent-hooks-project-row.tsx:41-77`: 테두리 rounded-sm px 12 py 6 text-xs, 프로젝트 이름(truncate). 조회 중 `settings.loading`(muted). 설치됨이면 `agent.hooksInstalled`(agent-running) + outline xs `agent.hooksUninstall`(해제 중 disabled). 아니면 outline xs `agent.hooksInstall`(설치 중 disabled) → 동의 대화상자(기본 크기, 제목 `agent.hooksConsentTitle`, 설명 `agent.hooksConsentDescription`, `common.cancel`, 확인 `agent.hooksInstall`) → 확인 즉시 닫힘과 설치 요청(36-39). 오류는 `toast.error(describeIpcError)`.
    - user 범위 행(66-100): 조회 키 project `""`. 힌트 세로 3줄: `settings.agentHooksUserLevelDescription`(muted), `settings.agentHooksUserLevelWarning`(warning), CLI 누락이면 `settings.agentHooksCliMissing`(warning). `requires = status.requiresTaideCli`, `isCliMissing = requires && !cliPending && !cli.installed`, `isCliBlocking = requires && (cliPending || (isCliMissing && !status.installed))`(74-76). 스위치 checked=`status.installed`, disabled=`!status || 설치 중 || 해제 중 || isCliBlocking`(95-96). 켜기 → 설치, 끄기 → 해제, 오류 toast(78-83).
    - 백엔드 정책 `CR/taide-runtime/src/agent_hook_actions.rs`: 상태(44-78), 설치(81-153: 설정 off면 "agent hooks are disabled in settings" 거절, project 범위는 열린 프로젝트 root 필요, user 범위 command-hook은 CLI 필요 "taide CLI is not installed yet"), 해제(156~).
- native 자산: 위 runtime 함수, `APP/agent-hooks.rs`(`ensure_started` 36, `apply_toggle` 74, `resolve_claude_hook_emitter` 120), `APP/settings-integrations.rs`(토글 변경 시 `agent_hooks::apply_toggle` 재조정 연결됨), `APP/remote-agents.rs` dispatch(`agent_hooks_status`·`install`·`uninstall` 조립, 136-175).
- 빠진 것: 토글·목록·행·동의 대화상자 UI, 로컬 조회·설치·해제.
- 설계: `Query::HooksStatus { project, agent }` → `agent_hook_actions::agent_hooks_status(&state, &tasks, project, agent, || (ports.agents.resolve_home)())`. `Action::HooksInstall` → `agent_hooks_install(.., AgentHookInstallPorts::new(|| (agents.resolve_home)(), || (agents.project_emitter)(services.clone()), || (agents.cli_status)().installed, || agent_hooks::ensure_started(services.clone()), &agents.cli_target))`(`remote-agents.rs`와 같은 조립). `Action::HooksUninstall` → `agent_hooks_uninstall`. 토글은 `HostedChange::AgentHooks(bool)`. 동의 대화상자 대상은 `OwnerState::hooks_consent: Option<ProjectId>`(section 수준).
- 검사: `[U]` 토글 off면 목록 없음, on이면 5종 순서·라벨, 프로젝트 0개 문구, 행 3상태, user 행 disabled 4경우(조회 전, CLI 조회 중, CLI 없음+미설치, CLI 없음+설치됨은 해제 가능), 대화상자 열기·취소·Escape·확인 즉시 닫힘·바깥 클릭 무시. `[A]` 임시 프로젝트 + 가짜 home(`ports.agents.resolve_home`)으로 claude 설치 → 임시 프로젝트 `.claude/settings.local.json`에 TAIDE 항목, 해제로 제거; codex 설치 + CLI 없음 → 원본 오류 문구 toast; 토글 off 상태 설치 → 거절 문구 toast.
- 완료 판정: 위 검사 통과. 실제 사용자 홈·실제 프로젝트 파일 쓰기는 4.2 허락 항목.

### settings-18 Interface: IDE 통합 스위치(설명 포함) (unwired)

- 원본: `settings-interface-section.tsx:77-82`. 라벨 `settings.ideIntegration`, 설명 `settings.ideIntegrationHint`, 기본 true, 패치 `ideIntegrationEnabled`. hooks 블록 다음, `ideAutoOpenDiff` 앞.
- native 자산: `APP/settings-integrations.rs` `Integrations::reconcile`(설정 변경 시 `ide_server::apply_toggle`), `APP/ide-server.rs`.
- 빠진 것: 스위치 UI와 패치.
- 설계: `HostedChange::IdeIntegration(bool)`. 기존 `switch` 렌더(`UI/settings-view.rs` 1746줄)를 라벨·설명·값·변경을 인자로 받는 내부 함수로 일반화해 같은 모양으로 그립니다.
- 검사: `[U]` 위치(배지 → CLI → hooks → IDE → diff 자동 열기)와 설명 문구. `[A]` 클릭 → `settings.json`의 `ideIntegrationEnabled` 변경 → 주입한 가짜 IDE 토글이 (current, updated)로 1회 호출(선례 `APP/settings-integrations-tests.rs`).
- 완료 판정: 위 검사 통과.

## 2B. Notifications (21, 22, 82, 23)

원본 `TS/widgets/settings-view/settings-notification-section.tsx:65-127`: 스위치 9개(settings-20 완료) 다음 가로 묶음(gap 8px)에 버튼 2개, 그 아래 DEV 안내.

### settings-21 Notifications: 시스템 알림 설정 열기 버튼 (unwired)

- 원본: 같은 파일 46, 115-119. 첫 버튼(outline xs, ExternalLink 14px + `settings.notificationsOpenSystemSettings`). 실패는 `toast.error(describeIpcError)`. 항상 보입니다(차단 감지가 불가능, 원본 주석 14-22).
- native 자산: 없음.
- 설계: `Action::OpenNotificationSettings` → `ports.open_notification_settings`(2.7, `run_blocking_result`). 오류는 `Toasts::ipc_error`.
- 검사: `[U]` 버튼 위치·아이콘·문구. `[A]` 가짜 opener 1회 호출, opener 오류 → 오류 toast. `[C]` 비 macOS 기본 구현이 `error.notification.settingsUnsupported` 키 오류를 냄(`cfg` 분기 함수를 OS 인자로 나눠 검사).
- 완료 판정: 위 검사 통과. 실제 시스템 설정 앱 열기는 4.2 허락 항목.

### settings-22 Notifications: 시험 알림 보내기와 suppressed 사유 toast (unwired)

- 원본: 같은 파일 23-35, 48-63, 120-123. 두 번째 버튼(Bell + `settings.notificationsSendTest`). 범주 `error`, 제목 `t('settings.notificationsSendTest')`, 본문 `t('notification.enableHint')`. `sendNativeNotification`을 직접 부르므로(`TS/entities/notification/notification.ipc.ts`) 자르기·창 종류 검사·첫 전달 안내가 없습니다. 결과: suppressed면 사유별 `toast.warning` — `notificationsDisabled`→`settings.notificationsEnabled`, `categoryDisabled`→`settings.notificationsError`, `windowFocused`→`settings.notificationsOnlyWhenUnfocused`; delivered면 `toast.success(t('notification.enableHint'))`; 오류면 `toast.error(describeIpcError)`.
- native 자산: `CR/taide-runtime/src/notification_actions.rs:12` `notification_notify(state, platform, category, title, body, has_focused_window)`(자격 증명 마스킹), `CR/taide-notification/src/service.rs:17` `decide_delivery`(마스터 → 범주 → 포커스 순), shell-91의 주입 platform과 "아무 창 포커스" 계산.
- 빠진 것: 버튼, 호스트 명령, 결과 toast.
- 설계: `Action::SendTestNotification` → UI 스레드가 제출 시점에 `any_window_focused`를 계산(shell-91과 같은 함수: 메인 + 열린 보조 viewport의 `input.viewport().focused == Some(true)`) → `Command::SendTestNotification { title, body, any_window_focused }` → 호스트가 shell-91과 같은 주입 platform으로 `notification_notify(&state, platform, NotificationCategory::Error, ..)`를 `run_blocking_result`에서 실행 → `Reply::TestNotification(AppResult<NotificationDelivery>)`. shell의 `ShellCommand::Notify`를 쓰지 않는 이유: 그 응답은 첫 전달 안내를 띄우는데 원본 시험 알림은 안내 경로를 타지 않습니다. 사유 → 라벨 키는 `const fn suppression_label(reason) -> &'static str`.
- 검사: `[C]` 사유 → 라벨 키 표. `[A]` 기록 platform 주입: (a) 기본 설정 + 메인 viewport focused → warning `settings.notificationsOnlyWhenUnfocused`, platform 미호출 (b) 마스터 off → `settings.notificationsEnabled` (c) `notifyError` off → `settings.notificationsError` (d) 포커스 없음 → success `notification.enableHint`, platform 1회(제목·본문 확인), 첫 전달 info toast 없음 (e) platform 오류 → 오류 toast.
- 완료 판정: 위 검사 통과. 실제 OS 알림 표시는 4.2 허락 항목.
- 선행 미충족 시: shell-91이 병합되지 않았으면 주입 platform이 없으니 멈추고 메인에 보고합니다(settings lane이 알림 포트를 따로 만들지 않음).

### settings-82 `settingsNotificationsDevHint`(DEV 빌드 안내) (missing)

- 원본: 같은 파일 125. `import.meta.env.DEV`일 때 버튼 묶음 아래 muted text-xs `settings.notificationsDevHint`.
- 설계: `host.is_dev_build`(`cfg!(debug_assertions)`)일 때 같은 위치에 muted 문구. 문구는 원문 그대로입니다(4.1 결정 2).
- 검사: `[U]` dev=true면 문구, false면 없음, 위치가 버튼 묶음 아래.
- 완료 판정: 위 검사 통과.

### settings-23 OS 알림 발송(agent 완료, 입력 대기, task 완료, git, 검색 치환, LSP 설치, 오류) (partial)

- 원본 요약(전체 규칙은 shell spec 2.5에 있음): agent·명령·LSP 생산자와 첫 전달 안내 `TS/app/providers/native-notification-provider.tsx:120-210`, 공통 전송 `TS/entities/notification/notify.ts:37-50`(주 창·비미러만, 200 코드포인트 자르기), git push/pull `TS/entities/git/git.query.ts:190-236`(범주 `gitRemote`/`error`), 검색 치환 `TS/widgets/search-panel/search-panel-container.tsx:131-143`(범주 `searchReplace`).
- 소유(1.3): agent·명령·LSP·첫 전달 안내 = shell-91(wave 1), git = git-7(git lane), 검색 치환 = explorer-search-33. "오류" 범주는 위 생산자들의 실패 분기입니다.
- settings lane이 하는 것: 구현하지 않습니다. 병합된 근거(shell `native-notifications-tests.rs`·`application-notifications-tests.rs`, git·explorer-search 검사)를 대응표 비고에 연결하고, 범주 7종이 각 설정 스위치로 억제되는지 한 App 검사(`application-settings-notifications-tests.rs`)로 묶어 확인합니다: 기록 platform + 포커스 없음에서 `TerminalCommandFinished`(15초, exit 1)·`LspInstallProgress(Done)` 발행 → 각각 1회, 해당 스위치 off → 0회.
- 완료 판정: shell-91·git-7·explorer-search-33이 모두 병합·검증되면 complete. 하나라도 빠지면 partial 유지, 비고에 남은 범주와 소유 lane을 적습니다. 실제 OS 표시는 4.2 허락 항목.

## 2C. LSP section (32, 33, 34)

### settings-32 LSP section: 서버 목록, 설치 여부, 경로, 버전, experimental 배지, 힌트 (unwired)

- 원본: `TS/widgets/settings-view/settings-lsp-section.tsx:16-46`(카드 제목 `settings.lspStatus`, 설명 `settings.lspDescription`, 조회 중 muted `settings.loading`), `TS/features/settings/lsp-server-status-list.tsx:104-187`.
    - 목록 gap 6px. 행(테두리 rounded-md px 12 py 8 text-xs, 세로 gap 4px): 설치됨이면 CircleCheck 14px(agent-running) 아니면 CircleX(badge), 이름(medium, truncate), id가 `expert`면 experimental 배지(`settings.lspExperimental`, warning 글자·warning 40% 테두리·rounded·px 4·10px), 오른쪽 상태 문구(설치됨 agent-running, 아니면 badge): `settings.lspInstalled` 또는 버전이 있으면 `"{문구} (v{버전})"`, 미설치 `settings.lspNotInstalled`(124-143).
    - `resolvedPath`가 있으면 들여쓴(22px) muted monospace 줄, break-all(145).
    - 상태 힌트(미설치이고 진행·실패가 아닐 때, 104-117, warning): download이고 `downloadAvailable != true` → `settings.lspChecksumPending`, toolchain이고 `toolchainAvailable != true` → `settings.lspToolchainMissing`, sdk-detect이고 `sdkAvailable != true` → `settings.lspSdkMissing`.
    - 설치 힌트(119-120, 152-170): 미설치·비활성이고 `installHint`가 있을 때, sdk-detect면 muted 평문, 그 외면 평문 + 복사 버튼(34).
    - 목록은 부팅당 1회 감지(2.4).
- native 자산: `CR/taide-lsp/src/service.rs:195` `detect_servers(lsp_dir, path_var)`, `APP/remote-lsp.rs`(같은 호출), `CR/taide-model/src/lsp.rs:228` `LspServerDetection`.
- 빠진 것: section UI, 로컬 조회.
- 설계: `Query::LspServers` → `run_blocking_result`에서 `taide_lsp::service::detect_servers(&state.paths.lsp_dir(), &(ports.path_var)())`. 렌더 `settings_sections_view::lsp_section`.
- 검사: `[U]` 고정 감지 목록으로 행 구성(아이콘·배지·버전 문구·경로·힌트 3종·sdk 평문). `[A]` 임시 데이터 디렉터리와 빈 PATH로 실제 감지 → 행 수가 manifest 서버 수와 같고 모두 미설치.
- 완료 판정: 위 검사 통과.

### settings-33 LSP: 설치, 취소, 진행률(바이트), 실패 메시지, 설치 이벤트 동기화 (unwired)

- 원본: `lsp-server-status-list.tsx:13-102`, `settings-lsp-section.tsx:24-31`, `TS/entities/lsp/lsp.query.ts:29-63`, `settings-view.tsx:129`.
    - 진행은 서버별 마지막 `lsp:install-progress`. 활성 = 진행이 있고 `done`·`failed`가 아님.
    - 행 오른쪽 버튼(이미 설치됨이면 없음, 76-102): 활성이면 ghost xs `settings.lspCancel` → 취소(오류 toast). download면 outline xs `settings.lspInstall`(`downloadAvailable != true`면 disabled). toolchain이고 `toolchainAvailable == true`면 `"{toolchainTool} {settings.lspInstall}"`. 그 외 없음.
    - 진행 줄(활성 또는 실패, 들여씀 22px): 실패면 `message ?? settings.lspInstallFailed`(error). 아니면 회전 Loader 12px + `settings.lspInstalling`(muted), `downloading`이고 total이 있으면 monospace `"{받은} / {전체}"`와 높이 4px 진행 막대(37-58). 바이트 형식(16-21): 1024 미만 `"{n}B"`, 1 MiB 미만 `"{n/1024 소수1}KB"`, 이상 `"{n/1048576 소수1}MB"`.
    - 설치 요청 실패: 오류 키 `error.lsp.installCancelled`면 조용히, 아니면 `toast.error(describeIpcError)`(24-30). 실패 시 그 서버 진행이 아직 끝나지 않았으면 진행 항목을 지웁니다(`lsp.query.ts:52-59`). 성공 시 서버 목록 재감지(51).
- native 자산: `CR/taide-runtime/src/lsp_actions.rs:291` `lsp_install_cancel`, 297 `lsp_install`(download/toolchain/sdk 분기, 중복 `error.lsp.installAlreadyRunning`, sdk `error.lsp.sdkDetectOnly`), `services.lsp_install`, `AppEvent::LspInstallProgress`.
- 빠진 것: 버튼·진행 UI, 로컬 설치·취소, 이벤트 소비.
- 설계: `Action::LspInstall(id)` → `run_nonabortable_result`에서 `(ports.lsp_install)(services, id)`. `Action::LspCancel(id)` → `lsp_install_cancel(&services.lsp_install, id)`. 진행 맵은 `Data::lsp_progress`(2.8로 갱신). 회전은 `motion_preference` 규칙을 따르고 진행 중 repaint 요청.
- 검사: `[C]` 바이트 형식 경계(1023, 1024, 1048575, 1048576). `[U]` 진행 상태별 렌더(다운로드 막대 폭 비율, 검증·압축 해제 중 바이트 없음, 실패 메시지·대체 문구), 버튼 분기 5종. `[A]` 가짜 설치 포트가 진행 이벤트 2개 발행 후 성공 → 진행 줄 → 사라짐 → 재감지 1회; 실패(취소 키) → toast 없음; 실패(기타) → 오류 toast + 진행 항목 제거; sdk-detect 서버에 실제 `lsp_install` → `error.lsp.sdkDetectOnly` toast(네트워크 없음).
- 완료 판정: 위 검사 통과. 실제 다운로드 설치는 4.2 허락 항목.

### settings-34 LSP: 설치 명령 복사, 툴체인/SDK/체크섬 힌트 (unwired)

- 원본: `lsp-server-status-list.tsx:63-70,104-170`. 복사 버튼(ghost icon-xs, Copy 12px, 접근 이름·tooltip `settings.lspCopyCommand`, tooltip 아래쪽) → 클립보드 쓰기 → 성공 `settings.lspCommandCopied`, 실패 `settings.lspCommandCopyFailed`. 힌트 3종은 32.
- native 자산: `write_clipboard`(`APP/host.rs`), 공통 tooltip(`UI/tooltip-trigger.rs` `Trigger { response, label, align, focus_target }`).
- 빠진 것: 버튼, 결과 toast(기존 `domains::shell::Reply::CopiedText`는 탐색기 문구).
- 설계: `Action::CopyInstallHint(text)` → `write_clipboard(services, worker.clipboard, "native-settings-lsp-copy", text)` → `Reply::LspHintCopied(AppResult<()>)`. tooltip은 `output.tooltips`에 `RectAlign::BOTTOM`.
- 검사: `[U]` 버튼 노출 조건(미설치·비활성·sdk-detect 아님·힌트 있음), tooltip 문구. `[A]` 가짜 클립보드: 기록된 텍스트와 성공 toast, 실패 시 실패 toast.
- 완료 판정: 위 검사 통과. 실제 클립보드는 4.2 허락 항목.

## 2D. AI section (35, 36, 37, 38, 39)

원본 전체: `TS/widgets/settings-view/settings-ai-section.tsx:41-133`. 카드 제목 `settings.aiSectionTitle`. 기본 provider `ollamaCloud`(20). 순서: 토큰 행 목록(gap 6px: ollamaCloud, codex, oMLX) → provider 선택 → 모델 영역 → auto-tab 토글 → 프롬프트 3행. 토큰 상태는 `ai_token_status`(oMLX는 base URL이 있으면 configured, `CR/taide-ai/src/service.rs:44-50`).

### settings-35 AI: provider 토큰 행(ollamaCloud, codex 경고) 저장/해제/상태 (unwired)

- 원본: `TS/features/settings/ai-provider-token-row.tsx:17-61`, 섹션 57-82.
    - 행(테두리 rounded-md px 12 py 8 text-xs, gap 6px): CircleCheck/CircleX 14px, 라벨(medium truncate: `settings.aiProviderOllamaCloud`, `settings.aiProviderCodex`), 오른쪽 `settings.aiTokenSaved`(agent-running) 또는 `settings.aiTokenNotSet`(badge). codex만 들여쓴 warning 줄 `settings.aiCodexUnofficialWarning`.
    - 들여쓴 입력 줄: password 입력(placeholder `settings.aiTokenPlaceholder`) + outline xs `settings.aiTokenSave`(trim이 비었거나 그 provider 저장 중이면 disabled) + 설정됨이면 ghost xs `settings.aiTokenClear`. 저장 시 trim, 빈 값이면 무시, 입력 비움.
    - 오류: 저장 `describe || settings.aiTokenSaveFailed`, 해제 `describe || settings.aiTokenClearFailed`(57-60). 성공 toast 없음. 성공 시 2.4 무효화.
- native 자산: `CR/taide-runtime/src/ai_actions.rs:30` `ai_token_status`, 36 `ai_set_token`(codex는 whoami 네트워크 검증, `CR/taide-ai/src/service.rs:52-100`), 42 `ai_clear_token`; `APP/remote-ai.rs`.
- 빠진 것: 행 UI, 로컬 저장·해제.
- 설계: `Query::AiTokens`, `Action::AiSetToken { provider, token: Secret }`(`run_nonabortable_result`), `Action::AiClearToken(provider)`.
- 검사: `[U]` 두 행 상태별 렌더·disabled·draft 비움. `[A]` 가짜 `SecretStore`(`SecretStoreState::with_store`)로 ollamaCloud 저장 → Saved, 해제 → NotSet, store 오류 → 원본 문구 toast. codex 저장은 네트워크라 검사에서 부르지 않습니다.
- 완료 판정: 위 검사 통과. 실제 Keychain·Codex 검증은 4.2 허락 항목.

### settings-36 AI: oMLX base URL, API key (unwired)

- 원본: `TS/features/settings/ai-omlx-row.tsx:15-51`, 섹션 61, 83-89. 같은 테두리 행 안에 텍스트 필드(라벨 `settings.aiOmlxBaseUrlLabel`, placeholder `settings.aiOmlxBaseUrlPlaceholder`, 입력 폭 224px, blur commit → trim 값으로 `aiOmlxBaseUrl` 패치)와 password 입력(placeholder `settings.aiOmlxApiKeyOptional`) + 저장(빈 값·저장 중 disabled) + 해제(항상 표시, ghost).
- native 자산: 35와 같음, 정규화 `CR/taide-settings/src/service.rs` `sanitize_optional_url`.
- 빠진 것: 행 UI, 패치, 키 저장·해제.
- 설계: `HostedChange::AiOmlxBaseUrl(String)`, 키는 `Action::AiSetToken { provider: Omlx }`. base URL 패치 응답 뒤 `AiTokens`·`AiModels(Omlx)`를 무효화합니다(원본은 무효화하지 않아 최대 60초 늦게 반영되는 결함, 재현하지 않음).
- 검사: `[U]` 행 구성. `[A]` base URL 입력 후 포커스 이동 → `settings.json` 값 trim 반영 → oMLX configured → 모델 조회 1회.
- 완료 판정: 위 검사 통과.

### settings-37 AI: provider 선택, 모델 목록 로딩/오류/빈 상태, 모델 선택 (unwired)

- 원본: 섹션 47-53, 91-113. provider 선택(라벨 `settings.aiProviderLabel`, 옵션 `settings.aiProvider{OllamaCloud,Codex,Omlx}`) → 패치 `{aiProvider, aiModel: ''}`. 선택 provider가 configured일 때만 모델 조회. 오류면 oMLX `settings.aiOmlxConnectFailed`, 그 외 `settings.aiModelLoadFailed`(error). 성공·빈 목록이면 muted `settings.aiModelSelectPlaceholder`. 목록이 있으면 모델 선택(라벨 `settings.aiModelLabel`, 옵션 `displayName ?? modelId`, placeholder `settings.aiModelSelectPlaceholder`) → 패치 `{aiProvider: 선택값, aiModel: id}`. 조회 중·미설정이면 아무것도 그리지 않습니다.
- native 자산: `ai_actions::ai_list_models`(47), 원격 검사의 loopback oMLX 모의 서버(`APP/remote-ai-tests.rs`).
- 빠진 것: 선택 UI, 모델 조회·표시.
- 설계: `Query::AiModels(provider)`; `HostedChange::AiProvider(id)`, `HostedChange::AiModel { provider, model }`. 선택 상자는 `Picker` 재사용.
- 검사: `[U]` 상태별(미설정 없음, 조회 중 없음, 오류 문구 2종, 빈 목록 문구, 목록 선택 상자와 placeholder). `[A]` loopback oMLX 모의 서버로 모델 2개 → 선택 → `settings.json`의 provider·model, provider 변경 → model `""`.
- 완료 판정: 위 검사 통과.

### settings-38 AI: auto-tab 토글(provider 미설정 시 비활성) (unwired)

- 원본: `TS/features/settings/ai-auto-tab-toggle.tsx:11-25`, 섹션 114-118. 라벨 `settings.aiAutoTabToggle`, 힌트(muted)는 비활성이면 `settings.aiAutoTabProviderRequired`, 아니면 `settings.aiAutoTabToggleHint`. disabled = 선택 provider 미설정. 변경 → 패치 `{aiProvider: 선택값, aiAutoTabEnabled}`.
- native 자산: 소비자 `APP/editor-ai-completion.rs`(`ai_auto_tab_enabled`, `ai_provider`, `ai_model`).
- 빠진 것: 토글 UI와 패치.
- 설계: `HostedChange::AiAutoTab { provider, enabled }`.
- 검사: `[U]` disabled·힌트 전환. `[A]` 클릭 → 설정 반영 → 편집기 자동완성 요청 경로가 켜짐(`APP/application-ai-completion-tests.rs` 픽스처 재사용).
- 완료 판정: 위 검사 통과.

### settings-39 AI: 프롬프트 파일 열기 3종(app file 탭) (unwired)

- 원본: 섹션 28-32, 63, 119-130. 위 구분선(border-t, pt 12) 아래 3행(gap 6px, text-xs): 라벨 `prompts.autoTabTitle|inlineEditTitle|commitMessageTitle`, 오른쪽 Pencil 14px 아이콘 버튼(접근 이름·tooltip `prompts.editEntry`) → 그 Settings 탭을 가진 프로젝트에 `AppFile { Prompt { id } }` 탭을 제목 `t(labelKey)`로 엽니다(2.6). 실패는 `toast.error(describeIpcError)`.
- native 자산: `CR/taide-model/src/app.rs:59-86` `PromptTemplateId`·`AppFileTarget::Prompt`, `HostCommand::OpenAppFile { project, pane, target, title }`, `APP/app-file.rs`(대상 무관 읽기), `APP/app-file-views.rs` `settings_command`.
- 빠진 것: 행 UI, 프롬프트 대상 열기.
- 설계: 2.6 `open_command`. `Output`에 `open_app_file: Option<(AppFileTarget, String)>`를 추가하고 `open_settings_file`과 같은 자리(`tab_content`의 Settings 분기)에서 소비합니다.
- 검사: `[U]` 3행·tooltip. `[A]` 클릭 → 같은 프로젝트 focused pane에 AppFile 탭(제목 지역화), 내용이 기본 프롬프트(`taide_app::service::read_app_file`).
- 완료 판정: 위 검사 통과.

## 2E. Plugins section (40, 41, 42, 43, 44)

원본 전체: `TS/widgets/settings-view/settings-plugins-section.tsx:17-27`(카드 제목 `settings.plugins`, lazy는 not-applicable 행 81), `TS/widgets/plugin-manager/plugin-manager.tsx:26-82`(gap 12px: 목록 → 버튼 묶음(gap 8px, wrap) 순서 zip 설치, VSIX 가져오기, 새로고침, 폴더 열기 → 제거 대화상자 → VSIX 대화상자).

### settings-40 Plugins: 플러그인 목록(이름, 버전, 활성, 오류 코드 7종) (unwired)

- 원본: `TS/features/plugin/plugin-list-body.tsx:8-61`. 조회 중 muted `settings.loading`. 빈 목록: 점선 테두리 rounded-md px 12 py 16 muted 상자에 `settings.pluginsEmpty`, `settings.pluginsManifestHint` 두 줄. 행(테두리 rounded-md px 12 py 8 text-xs): 이름(medium truncate), 버전(muted monospace), `settings.pluginEnabled`(agent-running)/`settings.pluginDisabled`(muted), 오른쪽 Trash 아이콘 버튼(접근 이름·tooltip `settings.pluginUninstallButton`, 44). 오류 코드가 있으면 아래 error 줄 `settings.pluginError.{parseFailed,idMismatch,versionMismatch,pathEscape,grammarMissing,grammarInvalid,grammarConflict}`.
- native 자산: `CR/taide-runtime/src/plugin_actions.rs:11` `plugin_list`, 16 `plugin_reload`, `CR/taide-model/src/plugin.rs:68-86`, `APP/remote-plugins.rs`. 문법 반영은 `APP/editor-syntax.rs:376` `follow_plugins`가 `PluginStore` 변경을 따라갑니다(원본 `applyPluginList`의 language 등록·shiki 재초기화, `plugin.query.ts:19-23`).
- 빠진 것: 목록 UI, 로컬 조회.
- 설계: `Query::Plugins` → `plugin_actions::plugin_list(&state, &services.plugin)`.
- 검사: `[U]` 3상태와 오류 코드 7종 문구. `[A]` 임시 plugins 디렉터리에 정상 1개·manifest 오류 1개 → 행 2개, 오류 문구.
- 완료 판정: 위 검사 통과.

### settings-41 Plugins: .zip 설치(파일 선택, 성공/실패 toast) (unwired)

- 원본: `TS/widgets/plugin-manager/plugin-install-button.tsx:26-46`. 버튼(outline xs, Import + `settings.pluginInstallButton`, 진행 중 disabled) → 파일 선택(필터 이름 `Plugin Archive`, 확장자 `zip`, 제목 `settings.pluginInstallDialogTitle`, 단일) → 취소면 아무것도 안 함 → `plugin_install(path)` → 성공 `settings.pluginInstallSuccess` + 목록 재조회, 실패 `describe || settings.pluginInstallFailed`.
- native 자산: `plugin_actions::plugin_install`(24, 디렉터리·zip), `rfd`.
- 빠진 것: 버튼, 파일 선택, 실행.
- 설계: UI `Action::PickFile(PickPurpose::PluginArchive)` → 앱이 `App::ui`(frame이 있는 곳)에서 `ports.pick_file(frame, OpenRequest { filter_name: "Plugin Archive", extensions: ["zip"], title })` → 경로가 있으면 `Command::PluginInstall(path)`(`run_nonabortable_result`). 선택 창 동안 pending 표시.
- 검사: `[A]` 가짜 picker가 검사용 zip 경로 반환 → 설치 성공 toast·목록 갱신; `None` → 호스트 명령 없음; 잘못된 zip → 원본 오류 문구 toast.
- 완료 판정: 위 검사 통과. 실제 파일 선택 창은 4.2 허락 항목.

### settings-42 Plugins: VSIX 가져오기 대화상자(테마 후보 선택, 중복 덮어쓰기 확인, 대비 실패 표시, 문법 가져오기 3상태) (unwired)

- 원본:
    - 시작 `plugin-manager.tsx:39-48`: 버튼(Import + `settings.pluginImportVsixButton`, 추출 중 disabled) → 파일 선택(필터 `VSIX`/`vsix`, 제목 `settings.pluginImportVsixDialogTitle`) → `vsix_extract_themes(path)` → 대화상자, 실패 `toast.error(describe)`.
    - 후보 생성 `TS/shared/lib/vsix-theme-import.ts:43-119`: id 기반 = `slugifyThemeId(publisher-name)`, 비면 `slugifyThemeId(displayName)`, 비면 `imported-theme`; 테마가 여러 개면 `-{slugify(label) 또는 theme}`; 같은 VSIX 안 중복이면 `-2`, `-3`…; uiTheme `vs`·`hc-light` → light, `vs-dark`·`hc-black`·기타 → dark; include chain을 base부터 JSONC 파싱(실패 → `parse`), 변환 결과 `status != ok` → `incomplete`, `outputColorErrors` 있음 → `contrast`; 경고 수 = safe default + ANSI 대체 + repair + 상태 구별 오류; `idCollides` = 기존 테마 id에 있음; 테마 `{version 1, id, name: label, type, palette {}, colors, syntax, terminal, tokenColors, author: publisher || null, license null, source: "displayName version" || null}`; key `"{index}-{label}"`.
    - 대화상자 `TS/widgets/plugin-manager/vsix-import-dialog.tsx:52-211`(Dialog, 2.6): 제목 `settings.pluginImportVsixButton`, 설명 displayName. 테마 영역 제목 muted medium `settings.pluginImportVsixThemesSection`; 후보 없으면 `settings.pluginImportVsixNoThemes`; 있으면 최대 높이 256 스크롤 목록, 행(테두리 rounded-sm px 8 py 6)마다 체크박스(실패면 disabled + 행 50% 투명), 라벨(truncate), 아래 줄 `settings.themeDark|themeLight`, 실패가 아니면 경고 수(TriangleAlert 12px + 수, warning), 중복이면 `settings.themeImportDuplicate{name: id}`(warning), 실패면 오른쪽 `settings.themeImport{ThemeParseFailure,ThemeIncomplete,ThemeContrastFailure}`(error). 처음 선택 = 실패 아닌 전부. 오른쪽 기본 xs `common.save`(선택 0개나 저장 중 disabled).
    - 저장(77-108): 선택 중 중복이 있으면 확인 AlertDialog sm(제목 `common.confirm`, 설명 = 중복 문구들을 `" / "`로 연결, `common.cancel`/`common.confirm`). 순서대로 저장, 중복이면 `generateUniqueThemeId(name, 사용 중 id)`(slug + 무작위 접미사, `TS/shared/lib/theme-draft.ts:64-76`)와 이름 `themeEditor.duplicateNameTemplate{name}`로 복사본, 아니면 원래 id. 끝나면 `settings.themeImportSuccess{count, extension}`, 첫 오류에서 멈추고 `toast.error(describe)`.
    - 문법 영역(위 구분선 `modal.separator`, pt 12, 제목 `settings.pluginImportVsixGrammarsSection`) `TS/features/plugin/vsix-import-grammars-section.tsx:21-41`: 성공이면 CircleCheck 14px + `settings.pluginImportVsixSuccess`(success), 실패면 Rust 오류 문구 그대로(error, toast 없음), 아니면 오른쪽 outline xs `settings.pluginImportVsixButton`(진행 중 disabled) → `vsix_import_plugin(path)` → 성공 toast `settings.pluginImportVsixSuccess` + 플러그인 목록 재조회.
    - 하단 outline `common.close`. 오른쪽 위 X 닫기.
- native 자산: `CR/taide-runtime/src/vsix_actions.rs:12` `vsix_extract_themes`, 17 `vsix_import_plugin(state, tasks, commit_staged_import, path)`, `plugin_actions::commit_staged_vsix_plugin`(71), `CR/taide-model/src/vsix.rs`, `theme_actions::theme_save`, `UI/theme-draft.rs:325` `unique_id(name, themes)`(같은 slug 규칙 + 무작위 접미사), `CR/taide-theme/src/service.rs` 테스트 모듈의 대비·색 함수와 쌍 표 drift 검사.
- 빠진 것: 테마 변환(이식), 후보 생성, 대화상자 2개, 저장·문법 가져오기 실행.
- 설계:
    1. 변환 이식 `CR/taide-theme/src/convert/`(순수, 새 의존성 없음, `taide-theme/src/lib.rs`에 `pub mod convert`): `color.rs`(← `color.ts` 199), `jsonc.rs`(70; 주석 제거 + 끝 쉼표 제거를 문자열 인식 수동 스캔으로. 원본 정규식은 문자열 안의 `,}`도 지우는 결함이라 재현하지 않음), `types.rs`(54), `vocabulary.rs`(173; 키 목록은 `service::required_color_keys`·`required_syntax_keys`·`required_terminal_keys`와 drift 검사), `ansi_palette.rs`(62), `mapping.rs`(378), `merge.rs`(43), `resolve.rs`(colors 83 + syntax 32 + terminal 12), `semantic.rs`(123 중 변환이 쓰는 부분), `pairs.rs`(component 192 + state 355), `state.rs`(273), `contrast.rs`(595), `completeness.rs`(13), `mod.rs`(`convert_vscode_theme(raw_chain: &[serde_json::Value], theme_type) -> ConversionResult`, ← `convert.ts` 116). `service.rs` 테스트 모듈의 색·대비 도우미는 `convert::color`·`convert::contrast`의 공개 함수로 옮기고 테스트가 그것을 부르게 합니다(같은 값, 중복 제거).
    2. 후보 생성: `CR/taide-runtime/src/vsix_actions.rs`에 `vsix_theme_candidates(result: &VsixThemeExtractionResult, existing_ids: &[String]) -> Vec<VsixThemeCandidate>`(위 규칙; slug는 TS `slugifyThemeId`와 같은 규칙을 이 crate에 둡니다). `VsixThemeCandidate`는 `taide-model::vsix`에 둡니다(UI가 그림).
    3. 호스트: `Command::VsixExtract(path)` → `vsix_extract_themes` + `theme_actions::theme_list`의 id로 후보 생성(블로킹) → `Reply::VsixExtracted(AppResult<(PathBuf, String extension, Vec<VsixThemeCandidate>)>)`. `Command::VsixSaveThemes(Vec<Theme>)` → 순서대로 `theme_save`(첫 오류 중단) → 성공 시 `Views::invalidate_catalog`. `Command::VsixImportGrammars(path)` → `vsix_import_plugin(&state, &tasks, |dir, id| plugin_actions::commit_staged_vsix_plugin(&state, &services.plugin, dir, id), path)`.
    4. UI: section 수준 `vsix: Option<VsixDialog { path, extension, candidates, selected, confirm_open, saving, grammars: Idle|Importing|Imported|Failed(String) }>`. 복사본 id·이름은 저장 시점에 UI가 계산합니다: `UI/theme-draft.rs` `unique_id`를 `pub(crate) fn unique_theme_id(name, used: &HashSet<String>)`로 일반화(기존 호출은 테마 목록 id로 집합을 만들어 위임)하고 사용 중 집합에 하나씩 넣습니다.
- 검사: `[C]` 이식: `TS/shared/lib/theme-convert/{convert,contrast,state-distinctness,jsonc,mapping-tables}.test.ts`와 `TS/shared/lib/vsix-theme-import.test.ts`의 사례를 Rust로 옮겨 같은 기대값(번들 테마 `CR/taide-theme/resources/themes/*.json`은 이미 TAIDE 형식이라 변환 입력으로 쓰지 않습니다). `[U]` 대화상자: 후보 없음, 실패 행 disabled·투명, 경고 수, 중복 문구, 확인 대화상자 설명 연결, 문법 3상태, X·바깥 클릭·Escape 닫기, 포커스 트랩. `[A]` 검사용 작은 `.vsix`(테마 2개 + 문법 1개, `APP/fixtures/`)를 가짜 picker로 → 저장 → 테마 목록 2개 증가, 같은 VSIX 재가져오기 후 저장 → 확인 대화상자 → 복사본 이름이 `themeEditor.duplicateNameTemplate`(en `"{{name}} copy"`) 형식이고 id가 기존과 다름; 문법 가져오기 → 플러그인 1개 증가, 재가져오기 → 실패 문구가 Rust 오류 그대로.
- 완료 판정: 위 검사 통과.

### settings-43 Plugins: 새로고침, 플러그인 폴더 열기 (unwired)

- 원본: `plugin-manager.tsx:36-37,60-67`. 새로고침(outline xs, RefreshCw + `settings.pluginsReload`, 진행 중 disabled + 아이콘 회전 `animate-spin` 1초 선형) → `plugin_reload` → 목록 교체, 실패 `toast.error(describe)`. 폴더 열기(FolderOpen + `settings.pluginsOpenFolder`) → `system_open_app_data_path('plugins')`, 실패 오류 toast.
- native 자산: `plugin_actions::plugin_reload`, 기존 `Output.folders`와 `HostCommand::OpenSettingsFolder(AppDataPathKind::Plugins)`.
- 빠진 것: 두 버튼과 회전 표시.
- 설계: `Action::PluginReload`; 폴더는 `output.folders.push(AppDataPathKind::Plugins)`. 회전은 `Icon::RefreshCw`를 1회전/초로 그리고 진행 중 repaint 요청, reduced motion이면 정지(기존 `motion_preference` 규칙).
- 검사: `[U]` disabled·회전 각 변화. `[A]` 새 플러그인 디렉터리 추가 → 새로고침 → 목록 반영; 기록 platform으로 폴더 열기 경로 = 앱 데이터 plugins.
- 완료 판정: 위 검사 통과.

### settings-44 Plugins: 제거 확인 대화상자 (unwired)

- 원본: `TS/widgets/plugin-manager/plugin-uninstall-dialog.tsx:23-54`. AlertDialog sm: 제목 `settings.pluginUninstallConfirmTitle{name}`, 설명 `settings.pluginUninstallConfirmDescription`, `common.cancel`, 파괴 버튼 `settings.pluginUninstallButton`. 확인은 누르는 즉시 대화상자가 닫히고(Radix Action) 제거가 진행됩니다. 성공 → `settings.pluginUninstallSuccess` + 목록 교체, 실패 → `describe || settings.pluginUninstallFailed`.
- native 자산: `plugin_actions::plugin_uninstall`(55).
- 빠진 것: 대화상자와 실행.
- 설계: 2.6 `AlertDialog`, section 수준 `uninstall: Option<(id, name)>`, `Action::PluginUninstall { id }`.
- 검사: `[U]` 열기·Escape·바깥 클릭 무시·포커스 복귀(Trash 버튼)·파괴 버튼 색·확인 즉시 닫힘. `[A]` 확인 → 디렉터리 삭제·목록 갱신·성공 toast; 없는 id → 실패 문구 toast.
- 완료 판정: 위 검사 통과.

## 2F. Sync section (45, 46, 47, 48)

원본: `TS/widgets/settings-view/settings-sync-section.tsx:28-100`, `TS/features/settings/sync-section.tsx:23-104`, `TS/entities/sync/sync.query.ts:5-39`. 카드 제목 `settings.syncSectionTitle`. 상태 조회는 연결돼 있으면 gist를 가져와 `remoteNewer`를 계산합니다(`CR/taide-runtime/src/sync_actions.rs:104-124`).

### settings-45 Sync: GitHub PAT 연결/해제 (unwired)

- 원본: `sync-section.tsx:8,38-66,97-100`, 섹션 46-52. 미연결(조회 중 포함): muted `settings.syncDescription`, muted `settings.syncTokenScopeHint` + 선택 가능한 URL `https://github.com/settings/tokens/new?scopes=gist&description=TAIDE%20Sync`(foreground), password 입력(placeholder `settings.syncTokenPlaceholder`) + outline xs `settings.syncConnect`(빈 값·연결 중 disabled). 연결 실패 `describe || settings.syncConnectFailed`, 성공 toast 없음. 연결됨: 하단 오른쪽 ghost xs CircleX + `settings.syncDisconnect`(진행 중 disabled) → 성공 `settings.syncDisconnected`, 실패 `describe || settings.syncDisconnectFailed`. 응답 상태로 캐시 교체.
- native 자산: `sync_actions::sync_connect`(126), `sync_disconnect`(168), `SyncGistHttpPort`(`CR/taide-runtime/src/sync_gist_http.rs`), `APP/remote-sync.rs`(원격은 status/upload/download만).
- 빠진 것: UI, 로컬 연결·해제.
- 설계: `Action::SyncConnect(Secret)`, `Action::SyncDisconnect`; gist는 2.7 `Gist`.
- 검사: `[U]` 미연결/연결 화면 전환, URL 텍스트 선택 가능, disabled. `[A]` 가짜 gist + 가짜 secret: 연결 → connected·gist id 반영; discover 오류 → 원본 문구 toast; 해제 → 성공 toast·미연결 화면.
- 완료 판정: 위 검사 통과. 실제 GitHub·Keychain은 4.2 허락 항목.

### settings-46 Sync: 상태(gist id, 마지막 동기화, remoteNewer 배지), 업로드, 다운로드 (unwired)

- 원본: `sync-section.tsx:68-96`, 섹션 53-62. CircleCheck(agent-running) + medium 제목, `remoteNewer`면 오른쪽 배지 `settings.syncRemoteNewerBadge`(warning 15% 배경, warning 글자, rounded-sm px 6 py 2). 들여쓴 행 `settings.syncGistIdLabel`(muted) – 설정 `syncGistId` 또는 `settings.syncGistIdNone`; 행 `settings.syncLastSyncedLabel` – `new Date(lastSyncedAt).toLocaleString()` 또는 `settings.syncLastSyncedNever`. warning `settings.syncSecretGistWarning`. 버튼 outline xs `settings.syncUploadNow`(성공 `settings.syncUploadSuccess`, 실패 고정 `settings.syncUploadFailed`), `settings.syncDownloadNow`(applied → `settings.syncDownloadSuccess`, conflict → 47, 실패 고정 `settings.syncDownloadFailed`).
- `toLocaleString()`은 앱 언어가 아니라 WebView 기본 locale(= 시스템 선호 언어·지역)과 로컬 시간대를 씁니다.
- native 자산: `sync_actions::sync_status`(104), `sync_upload`(186), `prepare_sync_download`(254) + `apply_sync_download`(280)(원격 조립 `APP/remote-sync.rs`의 `sync_download` 분기: apply는 `settings_actions::apply_and_broadcast` + reconcile).
- 빠진 것: UI, 로컬 실행, 날짜 표시.
- 설계: `Query::SyncStatus`, `Action::SyncUpload`, `Action::SyncDownload { force }`. 날짜는 UI crate에 `chrono`가 없으므로 앱이 `APP/settings-sync-time.rs` `format_last_synced(iso, system_locale) -> Option<String>`(chrono `DateTime::parse_from_rfc3339` → `Local`)으로 만들어 `Host::last_synced_display`로 넘깁니다. 형식은 4.1 결정 5.
- 검사: `[C]` 날짜 형식 ko-KR·ja-JP·en-US·기타(en-US 대체) 각 1사례, 잘못된 문자열 → `None`(원문 표시). `[U]` 배지 유무, 없음 문구 2종. `[A]` 가짜 gist: 업로드 → 성공 toast·마지막 동기화 갱신; 업로드 오류 → 고정 실패 문구; 다운로드 applied → 성공 toast + 설정 반영.
- 완료 판정: 위 검사 통과.

### settings-47 Sync: 충돌 대화상자(로컬 유지/원격 당김) (unwired)

- 원본: `TS/features/settings/sync-conflict-dialog.tsx:20-41`, 섹션 58-73. 다운로드(force false)가 conflict면 열림. AlertDialog sm: 제목 `settings.syncConflictTitle`, 설명 `settings.syncConflictDescription`, 버튼 outline `settings.syncConflictKeepLocal`(닫고 업로드, 업로드 toast 규칙 46), 기본 `settings.syncConflictPullRemote`(닫고 force 다운로드, 성공 `settings.syncDownloadSuccess`/실패 `settings.syncDownloadFailed`). 취소 버튼 없음, Escape는 취소, 바깥 클릭 무시. 열림 상태는 컨테이너 수준(2.5).
- native 자산: 46과 같음.
- 빠진 것: 대화상자.
- 설계: 2.6 `AlertDialog`(취소 버튼 없는 변형), `OwnerState::sync_conflict_open`.
- 검사: `[U]` 버튼 2개 순서, Escape 취소, 포커스 트랩, 테마 편집기 전환 뒤 돌아와도 열려 있음. `[A]` 가짜 gist의 원격이 더 최신 → 다운로드 → 대화상자 → 로컬 유지 → 업로드 1회; 다시 → 원격 당김 → force 다운로드 1회·성공 toast.
- 완료 판정: 위 검사 통과.

### settings-48 Sync: 설정 변경 후 동기화 데이터 반영 (unwired)

- 원본: `TS/app/providers/ipc-sync-provider.tsx:482-487`(`sync:state-changed` → 상태 캐시 교체 + 설정·테마·로케일 무효화), 413-428(`settings:changed` → 설정 교체, 테마·로케일·원격 상태·settings.json app file 무효화).
- native 자산: `apply_and_broadcast`가 `SettingsChanged` 발행, shell-93이 `presentation_refresh::Changes::record`에 `SyncStateChanged`를 추가(테마 revision 증가 → 앱 presentation 재적재 + `Views::observe_theme_revision`(`APP/application.rs`)으로 Settings 카탈로그 무효화), `app_file_views.observe_settings_revision`.
- 빠진 것: `SyncStateChanged` → `Data::sync` 교체(2.8). 나머지는 shell-93 산출물.
- 검사: `[A]` 가짜 gist 다운로드가 새 사용자 테마 파일과 그 테마를 고른 설정을 적용 → 이후 프레임에 Settings 테마 목록에 새 테마, 앱 presentation이 새 테마, 열린 settings.json 탭 내용 갱신, Sync 행의 마지막 동기화 값 갱신.
- 완료 판정: 위 검사 통과(shell-93 병합 뒤). shell-93 미병합이면 동기화 상태 반영만 증명하고 partial로 둡니다.

## 2G. Remote section (49, 50, 51, 52, 53)

원본: `TS/widgets/settings-view/settings-remote-section.tsx:24-90`, `TS/features/settings/remote-section.tsx:29-103`, `TS/entities/remote/remote.query.ts:7-37`. 카드 제목 `remote.title`, 설명 `remote.description`. 본문 순서(gap 8px, text-xs): 토글 → 상태 줄 → 보안 경고 → 비밀번호 행 → password-only 토글 → 허용 호스트 → 발급 URL → 버튼 줄. native 서버는 loopback에만 bind합니다(`APP/remote-http.rs`).

### settings-49 Remote: 서버 활성 토글(설명) (unwired)

- 원본: `remote-section.tsx:53-59`, `settings-remote-section.tsx:41-44`. 라벨 `remote.enableToggle`, 설명 muted `remote.enableToggleHint`, 스위치. 변경 시 발급 URL을 지우고 `remoteAccessEnabled` 패치.
- native 자산: `APP/settings-integrations.rs` remote 재조정(자산 준비 → `remote_http::apply_toggle`), `remote_http::start`/`stop`이 `RemoteStateChanged` 발행.
- 빠진 것: 스위치 UI와 발급 URL 초기화.
- 설계: `HostedChange::RemoteAccess(bool)`; `OwnerState::issued_remote_url = None`.
- 검사: `[U]` 위치·문구. `[A]` 가짜 remote 토글 주입(`Integrations::with_assets` 검사 방식) → 1회 호출, 발급 URL 사라짐.
- 완료 판정: 위 검사 통과.

### settings-50 Remote: 실행 상태, 포트, 클라이언트 수, 보안 경고 (unwired)

- 원본: `remote-section.tsx:60-73`. 실행 중이면 CircleCheck(agent-running) + `remote.statusRunning{port}`, 오른쪽 muted `remote.clientCountLabel{count}`; 아니면 CircleX(muted) + `remote.statusStopped`. 아래 warning `remote.securityWarning`. 상태는 5초 주기 + `remote:state-changed` 즉시.
- native 자산: `remote_actions::remote_status`(11), `RemoteStateChanged`.
- 빠진 것: UI, 조회·이벤트 소비.
- 설계: `Query::RemoteStatus`(관찰 중 5초), 2.8 이벤트로 즉시 교체.
- 검사: `[U]` 두 상태. `[A]` `RemoteStateChanged` 발행 → 이후 프레임에 포트·수; 5초 경과(주입 clock) → 재조회 요청.
- 완료 판정: 위 검사 통과.

### settings-51 Remote: 비밀번호 설정/해제, 최소 길이, password-only 로그인 토글 (unwired)

- 원본: `TS/features/settings/remote-password-row.tsx:17-66`, `remote-section.tsx:74-89`, `remote.query.ts:16-37`. 행(테두리 rounded-md px 12 py 8): CircleCheck/CircleX, medium `remote.passwordLabel`, 오른쪽 `remote.passwordConfigured`(agent-running)/`remote.passwordNotConfigured`(badge); 미설정이면 들여쓴 warning `remote.passwordHint`; trim 길이 1~7이면 warning `remote.passwordTooShort{min: 8}`; password 입력(placeholder `remote.passwordPlaceholder`) + outline `remote.passwordSet`(빈 값·짧음·저장 중 disabled) + 설정됨이면 ghost `remote.passwordClear`. 저장 중 = 설정 또는 해제 진행. 설정 성공·해제 성공 → 상태 무효화 + `remote.passwordSessionsRevoked`; 설정 실패 → 고정 `remote.passwordSaveFailed`; 해제 실패는 toast 없음. 설정됨일 때만 password-only 토글(라벨 `remote.passwordOnlyToggle`, 설명 `remote.passwordOnlyToggleDescription`) → `remotePasswordOnlyLogin` 패치.
- native 자산: `remote_actions::remote_set_password`(37, 길이 검증·해시·세션 폐기), `remote_clear_password`(53), `CR/taide-remote/src/types.rs:13` `REMOTE_PASSWORD_MIN_LEN = 8`(Rust는 `chars().count()`, TS는 UTF-16 길이).
- 빠진 것: UI, 실행.
- 설계: `Action::RemoteSetPassword(Secret)`, `Action::RemoteClearPassword`, `HostedChange::RemotePasswordOnly(bool)`. 최소 길이 상수를 `taide-model::remote`로 옮기고 `taide-remote::types`가 재수출합니다(`ALLOWED_HOST_WILDCARD_PREFIX`와 같은 방식). UI는 `taide_model::remote::REMOTE_PASSWORD_MIN_LEN`을 읽고 길이는 `chars().count()`로 셉니다(서버 판정과 같게).
- 검사: `[U]` 길이 경고 경계(7/8), disabled, 토글 노출 조건. `[A]` 가짜 secret: 설정 → 세션 폐기 toast·configured; 해제 → 같은 toast; 토글 → 설정 반영.
- 완료 판정: 위 검사 통과. 실제 Keychain은 4.2 허락 항목.

### settings-52 Remote: 허용 호스트 목록(추가/삭제, RFC 1035 검증, 중복) (unwired)

- 원본: `TS/features/settings/remote-allowed-hosts-row.tsx:15-113`. 라벨 `remote.allowedHostsLabel`, 설명 muted `remote.allowedHostsDescription`. 목록이 있으면 행(테두리 rounded-sm px 8 py 4, monospace truncate, X 아이콘 버튼 접근 이름 `remote.allowedHostsRemove{host}`, 설정 저장 중 disabled). 입력은 trim+소문자. 유효성(29-51): 라벨 1~63자, 앞뒤 `-` 금지, `[a-z0-9-]`; 전체 1~253자; `*.` 접두면 나머지가 `.`을 포함하고 같은 규칙. 무효면 warning `remote.allowedHostsInvalid`. 중복(대소문자 무시)이면 추가 불가(문구 없음). 입력(placeholder `remote.allowedHostsPlaceholder`, IME 조합 중 Enter 제외 Enter로 추가) + outline `remote.allowedHostsAdd`(추가 불가·저장 중 disabled). 변경은 `remoteAllowedHosts` 패치(실패는 설정 저장 공통 toast `settings.saveFailed`, 중복 toast 없음).
- native 자산: `CR/taide-settings/src/service.rs:243-282` `is_valid_dns_label`, `is_valid_allowed_host`(비공개), `CR/taide-model/src/remote.rs:4` `ALLOWED_HOST_WILDCARD_PREFIX`.
- 빠진 것: UI, 패치.
- 설계: `is_valid_allowed_host`(+ `is_valid_dns_label`, 길이 상수)를 순수 함수로 `taide-model::remote`에 옮기고 `taide-settings`가 그것을 부릅니다(동작 동일). IME 조합 중 Enter 무시는 `UI/modal.rs` `Composition`. `HostedChange::RemoteAllowedHosts(Vec<String>)`. "저장 중"은 `UpdateSettings` 진행 여부입니다(원본 `isUpdatingSettings`).
- 검사: `[C]` `TS/features/settings/remote-allowed-hosts-row.test.ts`의 사례를 `taide-model`로 옮김, `taide-settings` 기존 sanitize 검사 유지. `[U]` 무효 문구, 중복 시 disabled, Enter 추가, 조합 중 Enter 무시, 삭제. `[A]` 추가·삭제가 `settings.json`에 반영.
- 완료 판정: 위 검사 통과.

### settings-53 Remote: 접속 링크 발급과 클립보드 복사, 세션 폐기 (unwired)

- 원본: `settings-remote-section.tsx:45-56`, `remote-section.tsx:91-100`. 발급 URL이 있으면 선택 가능한 foreground 줄(break-all). 버튼 outline xs `remote.issueLink`(실행 중 아니거나 발급 중 disabled) → 성공 시 URL 저장 + 클립보드 쓰기 → 성공이면 `remote.linkCopied`(실패는 조용히); 발급 실패 고정 `remote.startFailed`. 오른쪽 ghost xs CircleX + `remote.revokeSessions`(실행 중 아니거나 진행 중 disabled) → 성공 `remote.sessionsRevoked`(실패 toast 없음). 발급 URL은 컨테이너 수준.
- native 자산: `remote_actions::remote_issue_link`(16), `remote_revoke_sessions`(31), `write_clipboard`.
- 빠진 것: UI, 실행.
- 설계: `Action::RemoteIssueLink` → 호스트가 발급 후 같은 작업에서 `write_clipboard`까지 하고 `Reply::RemoteLinkIssued(AppResult<(RemoteLinkInfo, AppResult<()>)>)`로 돌려줍니다(복사 실패는 toast 없음). `Action::RemoteRevoke`.
- 검사: `[U]` disabled 조건, URL 줄. `[A]` 실제 loopback 서버(가짜 자산) 시작 상태에서 발급 → URL 형식·가짜 클립보드 기록·toast; 클립보드 실패 → toast 없음; 폐기 → toast.
- 완료 판정: 위 검사 통과. 실제 클립보드는 4.2 허락 항목.

## 2H. Performance section (54)

### settings-54 Performance section(개발 게이트): Rust perf snapshot 읽기/초기화 (unwired)

- 원본: `TS/widgets/settings-view/settings-performance-section.tsx:19-98`, `settings-view.tsx:115,142-153,190-193,263`. 표시 조건: native perf 게이트 on이고 미러가 아님. 카드 제목 `settings.performance`, 설명 `settings.performanceDescription`. 버튼 outline xs `settings.performanceRead`, `settings.performanceReset`(진행 중 둘 다 disabled). 읽기: 프레임 probe, native snapshot, 프런트엔드 보고, Monaco 모델 수·query cache 수를 각각 `<h3>`(`settings.performanceFrontend|Native|Memory`) + pretty JSON. 초기화: 모두 비우고 레지스트리 reset. 오류 `toast.error(describe)`.
- native 자산: `CR/taide-infra/src/perf.rs`(`init` 373, `global` 367, `PerfRegistry::reset` 316, 게이트 `TAIDE_PERF` + debug 기본 on), `CR/taide-app/src/service.rs:88` `perf_snapshot(registry)`. native는 `perf::init`을 부르지 않아 게이트가 항상 off입니다(`APP/main.rs`). Tauri는 `TAURI/lib.rs:790`에서 부릅니다.
- 빠진 것: 게이트 초기화, section, 읽기·초기화.
- 설계: `APP/main.rs`에서 `taide_infra::perf::init(std::iter::empty())`. 앱 crate가 `taide-app`에 의존하지 않으므로 `CR/taide-runtime/src/app_actions.rs`에 `perf_snapshot(registry) -> PerfSnapshot`·`perf_reset(registry)` 래퍼를 추가합니다(앱 manifest 불변). `Action::PerfRead|PerfReset` → `ports.perf`. 표시는 native 블록(`settings.performanceNative` + `serde_json::to_string_pretty`, monospace, 가로 스크롤)만이고 TS 실행 환경 전용 블록은 not-applicable로 기록합니다(4.1 결정 6).
- 검사: `[U]` 게이트 off면 TOC·카드 없음, on이면 마지막. `[A]` 주입 레지스트리(enabled)에 span 1회 기록 → 읽기 → JSON에 그 항목 count 1; 초기화 → 블록 사라짐·레지스트리 0.
- 완료 판정: 위 검사 통과.

## 2I. 화면 골격·소유권 (3, 5)

### settings-3 TOC 클릭 스크롤과 활성 표시 (partial)

- 원본: `settings-view.tsx:59-92,133-140,188-197`, `TS/features/settings/settings-toc.tsx`. 항목 13개 + 조건부 Performance. 클릭 → 활성 갱신 + 그 카드 top − 32px로 부드러운 스크롤. 활성은 클릭으로만 바뀝니다. TOC는 sticky(top 32).
- native 자산: `UI/settings-view.rs` `View::show`(TOC 루프, `scroll_to_rect`, sticky `toc_top`), `Section::BASIC`.
- 빠진 것: 6개 항목과 카드.
- 설계: 2.3 `sections(host)`.
- 검사: `[U]` 호스트 있음에서 TOC 13개(+Performance) 순서·라벨, 마지막 항목 클릭 → 그 카드가 viewport 위 32px; 호스트 없음이면 8개(미러 불변). `[A]` 실제 App에서 같은 확인.
- 완료 판정: 2B~2H의 section이 모두 들어온 뒤(S5) 위 검사 통과.

### settings-5 탭 소유권(보조창 포함 프로젝트별 Owner) (partial)

- 원본: `settings-view.tsx:98-106,125,131,233`. Settings 탭이 보조 창으로 옮겨지면 그 창은 자기 프로젝트로 고정됩니다. Settings에서 여는 새 탭(settings.json, 프롬프트 3종)은 전역 활성 프로젝트가 아니라 그 탭을 가진 프로젝트, 그 창의 focused pane에 열립니다.
- native 자산: `UI/settings-owner.rs`(`Owner { project, pane, tab }`), `APP/app-file-views.rs` `settings_command`(보조 창이면 그 창 pane, 다른 프로젝트면 `None`), `tab_content`의 Settings 분기, `APP/settings-view-tests.rs` `native_settings_open_host는_선택pane와_중복_aux_거절을_보존한다`(693).
- 빠진 것: 실제 보조 창 수명(옮김 → 렌더 → 그 창에서 열기 → 닫힘 복귀)의 App 증거, 프롬프트 열기 경로(39).
- 설계: 2.6 `open_command`. 보조 창 닫힘 뒤 그 owner의 View는 `finish_frame`의 `retain(seen)`으로 사라지고 메인에서 새 mount(활성 TOC·발급 URL·충돌 대화상자 초기화)가 됩니다.
- 검사: `[A]` `APP/application-auxiliary-window-tests.rs`의 viewport 도우미로: 프로젝트 2개, 메인 활성 B, Settings 탭(프로젝트 A)을 새 보조 창으로 이동 → 보조 viewport에서 settings.json 클릭 → A의 보조 창 pane에 열림(B 아님) → AI 프롬프트 클릭도 같은 pane → 보조 창 닫기 → 탭 메인 복귀 → TOC 활성이 Appearance.
- 완료 판정: 위 검사 통과(shell-5 병합 뒤).

## 2J. 코드 설정 소비 확인 (24, 28, 30, 80)

### settings-24 Editor: 글꼴 크기, 글꼴 family picker(모노스페이스 필터, 검색, 시스템 기본, 미리보기) (partial)

- 원본: `TS/widgets/settings-view/settings-editor-section.tsx:64-80`, `TS/features/settings/font-picker.tsx`. 크기 숫자 필드(6~48). 목록 조회 중 `settings.loading`. picker: 오른쪽 위 `settings.fontFamilyMonospaceOnly` 스위치(기본 on, picker 인스턴스 상태), trigger(전체 폭, 값은 그 글꼴로, 없으면 `settings.fontFamilySystemDefault`, ArrowDown으로 열림), popup: 검색(`settings.fontFamilySearchPlaceholder`), 결과 없음 `settings.fontFamilyNoResults`, 첫 항목 시스템 기본(패치 `''`), 각 글꼴은 자기 글꼴로 미리보기, 선택 체크.
- native 자산: `UI/settings-code-view.rs` `FontPicker`(1235, `monospace_only` 1237·1297, 시스템 기본 1312, 필터 1319), 검색·결과 없음(855, 951), `UI/font-preview.rs`, `APP/font-preview.rs`, `APP/editor-fonts.rs` `Families`.
- 빠진 것: 실제 App에서 필터·검색·시스템 기본·적용 증거.
- 설계: 코드 변경 없음이 기본입니다. 검사에서 차이가 나오면 그 차이만 고칩니다.
- 검사: `[A]` 고정 글꼴 목록(가짜 `settings_resources::Reply` Fonts: monospace 2, proportional 1)으로 Editor picker: 필터 on → 2개 + 시스템 기본, off → 3개; 검색어 → 1개; 선택 → `editorFontFamily` 저장 → `editor_fonts::Families` 재적재 요청; 시스템 기본 → `""` 저장·trigger 문구.
- 완료 판정: 위 검사 통과.

### settings-30 Terminal section(글꼴 크기/family, shell, shell profile, scrollback+힌트, cursor, blink) (partial)

- 원본: `TS/widgets/settings-view/settings-terminal-section.tsx`. 크기(6~48), family picker(24와 같음), shell 텍스트 필드(placeholder `/bin/zsh`, trim), shell profile 목록(조회 중 `settings.loading`), scrollback(100~100,000, 힌트 `settings.terminalScrollbackHint`), cursor `bar|block|underline`, blink 스위치. 실행 중 터미널에 즉시 반영: 크기·family(재 fit), scrollback, cursorStyle, cursorBlink(`TS/features/terminal/terminal-view.tsx:217-257`). shell은 새 터미널부터.
- native 자산: `UI/settings-code-view.rs`(전 컨트롤), `APP/terminal_settings.rs` `cursor`, `APP/terminal_host.rs` `Hub::configure_cursors`(프레임마다, `TERM/session.rs` `Session::configure_cursor` 255), `terminal_appearance.set_font_size`, `APP/terminal_fonts.rs`, 새 세션 scrollback(`APP/terminal_tabs.rs:424`), `TERM/lib.rs` `TerminalCore::new(size, history, limits)`(`Config { scrolling_history }`, `admit` 예산 검사).
- 빠진 것: 실행 중 터미널 scrollback 변경(생성 시에만 반영), 전체 App 증거.
- 설계(4.1 결정 7 A안): `TERM/lib.rs`에 `TerminalCore::set_history(lines) -> AppResult<bool>`(`admit(size, lines, limits)` 재검사 후 vendored alacritty `Term::set_options(Config { scrolling_history: lines, osc52: Disabled, .. })`, `vendor/alacritty-terminal/src/term/mod.rs:552`), `TERM/session.rs`에 `Session::configure_history(lines)`(`configure_cursor`와 같은 잠금·Failed 처리·같은 값이면 false), `APP/terminal_host.rs`에 `Hub::configure_history()`(설정값 읽어 모든 세션). 앱 프레임에서 `configure_cursors` 옆에서 부릅니다.
- 검사: `[C]` `set_history`: 줄이면 오래된 줄 잘림, 늘리면 유지, 예산 초과는 오류. `[A]` `/bin/cat` 셸 터미널 1개(`APP/application-context-menu-tests.rs` 방식)로 scrollback·cursor·blink·크기 변경 → 실행 중 세션의 history·cursor·글자 크기 반영; shell 변경 → 기존 세션 불변, 새 세션만.
- 완료 판정: 위 검사 통과.

### settings-28 위 표시·입력 옵션의 에디터 반영 (partial)

- 원본 소비처: `TS/shared/lib/code-editor-settings.ts:43-65`(Monaco 옵션), `TS/widgets/editor-pane/use-lsp-session.ts`(semantic), `TS/shared/lib/diff-view-settings.ts:13-14`(diff 2종), `TS/app/providers/emmet-provider.tsx`(emmet).
- native 소비(이번 조사 grep, 테스트·설정 컨트롤 제외): `UI/presentation.rs` 246-299(line numbers, word wrap, suggest preview, render whitespace, rulers, cursor style/blinking/smooth caret, scroll beyond last line, smooth scrolling, bracket 2종, sticky scroll, minimap), `APP/presentation-refresh.rs`(detect indentation 20, word wrap, minimap), `APP/editor-formatting.rs:60-61`(format on type/paste), `APP/editor-completion-provider.rs`(emmet), `NativeApplication`(code lens, sticky scroll, minimap).
- 소비자 없음: `editor_font_ligatures`(egui 한계로 기록 중, `docs/acknowledge/2026-10-06-native-transition-decisions.md` 61줄), `editor_semantic_highlighting`(native LSP semantic tokens 없음, editor lane), `editor_diff_hide_unchanged_regions`·`editor_diff_show_moves`(diff 보기 없음, git lane, git spec 490-494).
- 설계: 새 소비자를 만들지 않습니다. 있는 소비자마다 "설정 토글 → 소비 변화" App 검사를 한 파일(`application-settings-consumers-tests.rs`)에 모읍니다.
- 검사: `[A]` 필드마다 1사례(예: wordWrap → 같은 문서 표시 줄 수 변화, emmet off → 완성 목록에서 emmet 항목 사라짐, formatOnPaste off → 붙여넣기 포맷 요청 없음, rulers → 눈금 위치, lineNumbers off → gutter 숫자 없음).
- 완료 판정: 있는 소비자 검사 통과 + 비고에 ligature(한계), semantic(editor lane), diff 2종(git lane). 세 의존이 닫히면 complete.

### settings-80 Interface 설정값의 소비처 (partial)

- 원본 소비처: 탐색기 reveal `TS/widgets/explorer/use-explorer-auto-reveal.ts`, 환영 화면 `TS/widgets/editor-area/pane-node-view.tsx`(메인 창만), 상태 배지 `TS/widgets/app-sidebar/app-sidebar.tsx`, IDE diff 자동 열기 `TS/app/providers/ide-sync-provider.tsx:49-67`(off면 즉시 rejected), searchOnType·debounce `TS/widgets/search-panel/search-panel-container.tsx:184-203`, minimap `code-editor-settings.ts`.
- native 소비(grep): 환영 화면 `UI/snapshot.rs:33`·`UI/shell.rs`, 상태 배지 `NativeApplication`(`agent_status_badge_enabled`), minimap(28과 같음). 없음: `explorer_auto_reveal`(explorer-search-25), `search_on_type`·debounce(explorer-search-34), `ide_auto_open_diff`(git-49; 지금 `APP/ide-tools.rs`가 이벤트를 내고 응답을 기다리다 시간 초과로 rejected).
- 설계: 새 소비자를 만들지 않습니다. 있는 3개에 App 증거 검사를 추가하고, 없는 3개는 해당 lane 병합 상태에서 App 검사 1개씩 추가해 닫습니다(git-49는 같은 wave라 메인 통합 단계).
- 검사: `[A]` 환영 화면 off → 빈 편집기 영역에 환영 없음; 배지 off → 레일 배지 없음; minimap off → 미니맵 영역 없음. 병합된 경우: reveal off → 활성 탭 바꿔도 탐색기 선택 불변; searchOnType off → 입력만으로 검색 요청 없음, debounce 값만큼 지연.
- 완료 판정: 있는 3개 통과 + 남은 의존을 비고에. 의존이 닫히면 complete.

## 2K. 키·명령·오류 표시 (67, 78, 83)

### settings-67 편집한 키가 실제 동작에 반영 (partial)

- 원본: `TS/shared/lib/keymap/keymap.ts`(override 병합), 명령 실행기. 카탈로그의 모든 명령이 실행 경로를 갖고 override는 같은 경로로 실행됩니다.
- native 자산: `UI/keymap.rs`, `UI/keybinding-catalog.rs`, `UI/keybinding-commands.json`, `APP/shell_keymap.rs`, `APP/command-dispatch.rs`(`accepts` 60, `intent` 70; 실행 경로 없는 명령 거절), override 실행은 지원 명령에 연결됨.
- 빠진 것: 실행 경로가 없는 명령(`search.*`, `git.*`, `sync.*`, `cli.*`, 일부 `monaco.*` 등)의 소비자. 각 영역 lane과 commands-ui lane(commands-ui-30~34 등, wave 3) 몫입니다.
- 설계: settings lane이 하는 것은 (1) override 동등성 검사: 카탈로그의 모든 명령 id 중 `accepts`가 참인 것은 기본 키 대신 새 키로 override했을 때 새 키가 같은 intent를 내고 기본 키는 더 이상 내지 않는다는 것을 전수 검사, (2) 실행 경로 없는 명령 id 목록을 검사 출력으로 남겨 비고에 붙임, (3) `domains::settings`의 `CliInstall`·`CliUninstall`·`SyncUpload`·`SyncDownload` 명령을 `pub(crate)`로 두어 commands-ui가 재사용(팔레트 문구 규칙은 `TS/entities/sync/sync.commands.ts:7-43`, `TS/entities/agent/agent.commands.ts:18-66`; 구현은 commands-ui lane).
- 검사: `[A]` 위 (1)·(2).
- 완료 판정: 이 lane에서는 partial 유지(비고에 남은 명령 수와 소유 행). commands-ui lane이 마지막 소비자를 붙일 때 complete.

### settings-78 오류 표시 경로(TS는 `toast.error`) (partial)

- 원본: 대부분 `toast.error(describeIpcError(error))`, 설정 저장 실패는 `settings.saveFailed` + 설명(`TS/entities/settings/settings.query.ts`), 테마·로케일 실패는 배너, 트리 실패는 인라인.
- native 자산: `UI/toast.rs`(`ipc_error`, `settings_failed`, `describe_error`), `APP/status-error-banner.rs`(shell-94), `NativeApplication`의 `status: Option<String>`과 상태바 라벨. 이번 grep으로 `status = Some(` 대입이 `application.rs` 90곳, `domains/commands.rs` 5곳, `domains/shell.rs` 2곳(다른 변수 포함 가능). 분류 기록 `docs/quality-assurance/2026-10-06-native-batch3-toast-feedback.md` 3.3.
- 빠진 것: 남은 status 대입 지점의 처리 방향(사용자 결정 대기).
- 설계: 이 lane의 새 화면은 모두 원본 toast 표를 따르고 status 라벨을 쓰지 않습니다. 남은 지점은 4.1 결정 3에 따라 일괄 처리합니다(A안: TS 대응이 있는 지점은 그 표면, 대응이 없는 native 내부 오류는 `log::warn!`만 남기고 상태바 라벨과 `status` 필드 제거). 다른 lane 파일의 지점은 그 lane 병합 뒤 메인 통합 단계에서 처리합니다.
- 검사: `[A]` 결정 반영 후 `status = Some(` 대입 0건(또는 B·C 기준)과 대표 경로 3개(호스트 실패, 저장 실패, 터미널 오류)의 표시 확인.
- 완료 판정: 결정 반영과 위 검사 통과.

### settings-83 Monaco 키코드 바인딩 가능 여부와 Dialog 포커스 트랩 (partial)

- 원본: `TS/widgets/keybindings-editor/keybindings-editor.tsx:100-104`(monaco 명령이면 `resolveMonacoKeyCode(key)`가 있어야 함, 아니면 `settings.keymapKeyNotBindable` 경고), 키 표 `TS/shared/lib/monaco/monaco-keybinding.ts:19-94`. Radix Dialog 포커스 트랩.
- native 자산: `UI/keymap.rs:997` `editor_key`(같은 집합: 영숫자, `` ;=,-./`[\]' `` 와 공백, 이름 키 16개 + F1~F12), `UI/keybinding-capture.rs`(단일·chord 경고 146~240), `UI/modal.rs` `trap_focus`·`FocusReturn`.
- 빠진 것: 키 표 drift 방지, 이 lane 새 대화상자들의 트랩 증거. "모든 Monaco 동작"은 editor lane, 실기 포커스·접근성은 사용자 실기입니다.
- 설계: `editor_key` 집합과 원본 `MONACO_KEY_CODE` 키 집합이 같은지 `include_str!("../../../src/shared/lib/monaco/monaco-keybinding.ts")`로 읽어 비교하는 검사(선례 `CR/taide-theme/src/service.rs` `테마_토큰_목록은_rust와_theme_tokens_ts에서_일치한다`). 2.6 대화상자 모듈 1곳에서 트랩·복귀·Escape를 구현하고 5종(hooks 동의, 플러그인 제거, VSIX, VSIX 덮어쓰기, 동기화 충돌) 모두 같은 검사.
- 검사: `[C]` drift 검사. `[U]` 대화상자 5종 각각 Tab/Shift+Tab 순환이 밖으로 나가지 않음, Escape(VSIX는 IME 조합 중 제외), 포커스가 여는 버튼으로 복귀.
- 완료 판정: 위 검사 통과(실기 접근성은 4.2).

## 3. 작업 묶음 계획

순서는 의존 순서입니다. 각 묶음은 실패 검사 → 구현 → 영역 검사 → 대응표 행 갱신 → lane QA 기록으로 끝냅니다. 같은 영역 파일(`domains/settings.rs`, `settings-sections-*.rs`)을 고치므로 묶음은 직렬입니다. 새 외부 의존성은 어느 묶음에도 필요 없습니다.

| 묶음 | ID(수) | 내용 | 수정 예상 파일 | 새 의존성 |
| --- | --- | --- | --- | --- |
| S1 골격·Interface·알림 버튼·Performance | 16, 17, 18, 21, 22, 82, 54 (7) | 2.1~2.8 골격 전부(호스트 진입점, Section 확장, `Data`, `OwnerState`, 대화상자 모듈, `Ports`·`new_with_overrides`, 아이콘, `PaintSink` 기록기), CLI 행, hooks, IDE 스위치, 알림 버튼 2개·DEV 안내, Performance | UI: `settings-controls.rs`, `settings-view.rs`, 새 `settings-sections-data.rs`·`settings-sections-view.rs`·`settings-dialogs.rs`, `settings-code-view.rs`(도우미 가시성), `icons.rs`, `lib.rs`. APP: `domains/settings.rs`, 새 `settings-ports.rs`, `application.rs`·`host.rs`(추가만), `main.rs`(`perf::init`), `resources/icons/`(svg 4). CR: `taide-runtime/src/agent_host.rs`, `taide-runtime/src/app_actions.rs`. 검사: `UI/settings-sections-view-tests.rs`, `APP/application-settings-sections-tests.rs`, `CR/taide-runtime` 단위 | 없음 |
| S2 LSP·AI | 32, 33, 34, 35, 36, 37, 38 (7) | LSP 목록·설치·취소·진행·복사, AI 토큰·oMLX·provider/model·auto-tab | UI: `settings-sections-*.rs`. APP: `domains/settings.rs`, `settings-ports.rs`. 검사 파일 | 없음 |
| S3 Plugins·VSIX·프롬프트·소유권 | 39, 40, 41, 42, 43, 44, 5 (7) | 테마 변환 이식(가장 큼), VSIX 후보·대화상자, 플러그인 목록·설치·새로고침·폴더·제거, 프롬프트 열기·`open_command`, 보조 창 소유권 App 검사 | CR: `taide-theme/src/convert/*`(새, 약 2.8천 줄), `taide-theme/src/lib.rs`·`service.rs`(테스트 도우미 이동), `taide-runtime/src/vsix_actions.rs`, `taide-model/src/vsix.rs`(후보 타입). UI: `settings-sections-*.rs`, `settings-dialogs.rs`, `theme-draft.rs`. APP: `domains/settings.rs`, `app-file-views.rs`, `file-dialogs.rs`, `fixtures/`(검사용 `.vsix`·zip). 검사 파일 | 없음(JSONC·색 계산 수동 구현, `regex` 미사용) |
| S4 Sync·Remote | 45, 46, 47, 48, 49, 50, 51, 52, 53 (9) | 연결·상태·업로드·다운로드·충돌, `SyncStateChanged` 반영, 원격 토글·상태·비밀번호·허용 호스트·링크·폐기 | UI: `settings-sections-*.rs`. APP: `domains/settings.rs`, `settings-ports.rs`(`Gist`), 새 `settings-sync-time.rs`. CR: `taide-model/src/remote.rs`(검증·최소 길이 이동), `taide-remote/src/types.rs`(재수출), `taide-settings/src/service.rs`(호출 교체). 검사 파일 | 없음(날짜는 기존 `chrono`·`sys-locale`) |
| S5 마감·교차 확인 | 3, 23, 24, 28, 30, 67, 78, 80, 83 (9) | TOC 최종, 알림 범주 묶음 검사·행 갱신, 글꼴·터미널 App 증거와 실행 중 scrollback, 소비자 증거, override 동등성, status 라벨 결정 반영, 키 표 drift·대화상자 트랩 | TERM: `lib.rs`·`session.rs`(결정 7 A). APP: `terminal_host.rs`(추가만), `application.rs`(status, 결정 3), `domains/settings.rs`. UI: `keymap.rs`(drift 검사). 검사: `application-settings-notifications-tests.rs`, `application-settings-consumers-tests.rs` | 없음 |

묶음별 주의

1. S1은 토대라 먼저 단독으로 끝냅니다. S2·S3·S4는 내용상 독립이지만 lane 규칙상 직렬입니다(권장 순서 S2 → S3 → S4; S3 변환 이식이 가장 커서 늦게 막히지 않도록 S2 다음).
2. S3은 "변환 이식 → 후보 생성 → UI" 순으로 중간 단계마다 검사를 통과시킵니다(묶음 완료는 셋 다 끝난 뒤). 변환 이식만으로 묶음 시간이 넘치면 메인에 알리고 S3a(42 이식만)·S3b(나머지)로 나눕니다.
3. S5의 23·28·67·80·78은 다른 lane 병합 상태와 사용자 결정에 묶여 partial로 남을 수 있습니다. 그 경우 비고에 남은 의존을 정확히 적고 끝냅니다(과대 판정 금지).
4. 공통 게이트(묶음마다): 변경 crate 전체 테스트 대상(`--no-fail-fast -- --test-threads=4`), crate별 `cargo fmt --check`, UI crate·`taide-model`을 바꾼 묶음은 동결 미러 Wasm check, 공유 crate를 바꾼 묶음(S1, S3, S4)은 `cargo check -p taide`, lockfile 무변경 확인(`git diff --stat -- '**/Cargo.lock'` 0).
5. 선행 미충족 처리: 22(shell-91 포트), 48(shell-93), 5(shell-5)는 wave 1 병합이 전제입니다. 병합되지 않았으면 그 ID만 건너뛰고 비고에 적은 뒤 메인에 보고합니다(대체 구현을 만들지 않음).

## 4. 사용자 결정과 허락

### 4.1 결정이 필요한 항목

1. 원격 미러(동결 `WEB`)의 새 section 표시(3, 16~22, 32~54) — **A(추천: 동결 유지, 미러에서는 원본도 이 동작들이 원격 정책으로 거절됨): native 앱에만 표시하고 미러는 지금 8 section 유지** / B: 미러에도 표시(`WEB` 소스 변경, 동결 해제)
2. DEV 안내 문구(82)와 알림 발신자 — **A(추천: 의존성 없음): 문구 원문 유지, native dev 빌드는 osascript라 발신자가 "Script Editor"로 보이는 차이를 QA에 기록** / B: native 전용 문구 추가(새 locale 키) / C: UNUserNotificationCenter 전환(`objc2-user-notifications` 새 의존성·서명 번들 필요, cutover 배치에서)
3. status 라벨 잔여 지점(78, 2026-10-06부터 미결) — **A(추천: 원본과 같은 표면): TS 대응이 있는 지점은 원본 표면(toast·배너·인라인), 대응이 없는 native 내부 오류는 `log::warn!`만 남기고 상태바 라벨 제거** / B: 전부 `toast.error(describe)` / C: 현행 유지
4. CLI 설치 대상(16, 17) — **A(추천: 원본과 동일): `/usr/local/bin/taide`, 소유 판정 동일. cutover 전 native 바이너리는 `.app` 번들 밖이라 설치가 원본 오류 문구로 실패하는 것이 정상 동작** / B: native 전용 이름
5. 마지막 동기화 시각 형식(46) — **A(추천: 새 의존성 없음): 시스템 locale(`sys-locale`)이 ko·ja·en이면 각 브라우저 Intl 기본 형식과 같은 패턴(`2026. 10. 28. 오후 2:05:09`, `2026/10/28 14:05:09`, `10/28/2026, 2:05:09 PM`)을 chrono `Local`로, 그 외 locale은 en-US 패턴** / B: ICU datetime으로 모든 locale 대응(새 의존성)
6. Performance의 TS 전용 블록(프런트엔드 계측·프레임 probe·Monaco 모델 수·query cache 수, 54) — **A(추천: 새 기능 추가 없음): native 지표 블록만 표시하고 나머지는 not-applicable 기록** / B: native 대응 지표를 새로 정의
7. 실행 중 터미널 scrollback 반영(30) — **A(추천: 원본과 동일): settings lane이 `TERM`·`terminal_host.rs`에 `set_history`/`configure_history`를 추가(wave 1 병합 뒤, 추가만)** / B: 새 터미널에만 반영(원본과 다름, 비고에 차이 기록)
8. 겹치는 범위 소유(5, 22, 23, 48, 80, 28) — **A(추천: shell·git spec의 결정 1 A와 일치): 알림 생산자·첫 전달 안내·Sync 테마 재로드·보조 창 수명은 shell lane, git 알림·IDE diff·diff 설정은 git lane, reveal·searchOnType·검색 치환 알림은 explorer-search lane이 만들고 settings lane은 재사용·확인·행 갱신만** / B: settings lane이 직접 구현(중복 위험)

모두 추천안이면 "전부 추천안대로"로 답하시면 됩니다. 3은 다른 lane 파일에도 걸리므로 S5 착수 전에, 8은 lane 지시 작성 전에 답이 필요합니다. 나머지는 추천안으로 진행해도 이후 바꾸기 쉽습니다.

### 4.2 허락이 필요한 항목(자동 검사·작업자가 하지 않는 것)

작업자 자동 검사는 모두 가짜 포트를 쓰고 아래를 하지 않습니다. 실기 확인은 사용자 허락 뒤 별도(배치 31 등)로 진행합니다.

1. 실제 OS 알림 표시와 macOS 알림 권한 상태(21, 22, 23).
2. 시스템 설정 앱의 알림 패널 열기(21).
3. 실제 CLI 설치·해제: `/usr/local/bin` 쓰기와 관리자 암호 프롬프트(16). 설치된 Tauri 앱의 symlink를 바꿀 수 있습니다(결정 4).
4. Keychain 쓰기: AI 토큰, GitHub PAT, 원격 비밀번호 해시. native는 격리 서비스명(`net.gumyo.taide.native-isolated`, `APP/bootstrap.rs`)을 쓰지만 사용자 키체인입니다(35, 36, 45, 51).
5. 사용자 데이터 쓰기: 사용자 홈의 에이전트 hooks 파일(`~/.codex` 등)과 실제 프로젝트의 `.claude/settings.local.json`(17). 토글 재조정도 같은 파일을 씁니다(이미 연결된 경로).
6. 네트워크: LSP 서버 실제 다운로드(33), GitHub gist 연결·업로드·다운로드(45~48), Codex 토큰 검증과 ollamaCloud 모델 목록(35, 37).
7. 실제 클립보드 쓰기(34, 53)와 실제 파일 선택 창(41, 42).
8. 원격 접속 서버 실기 기동(loopback이라 외부 노출은 없음, 49~53).
9. 실기 포커스·VoiceOver 접근성 확인(83).
