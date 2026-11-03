# native lane settings 묶음 S1: Interface 행·알림 버튼·Performance (s1-interface-performance)

> 작성일: 2026-11-03 · worktree `TAIDE-lanes/settings`(브랜치 `lane/settings`) · 사양 `docs/research/2026-10-28-lane-settings-spec.md` 2.1~2.8, 2A(16~18), 2B(21·22·82), 2H(54), 3절 S1
> 결정: 사양 4.1 전부 추천안(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`), CLI 관리자 대화상자 취소 시 성공 문구 없음(같은 문서 13줄). 앞 묶음 S2·S3·S4의 미커밋 변경 위에서 이어 작업했습니다(공통 골격 대부분은 S2에서 이미 만들어짐).

## 1. 대상 ID와 판정

| ID | 내용 | 판정 |
| --- | --- | --- |
| settings-16 | Interface `taide` CLI 설치/해제 행(macOS) | complete(실제 `/usr/local/bin`·관리자 암호는 4.2 허락 항목) |
| settings-17 | Interface agent hooks 토글·에이전트 5종·프로젝트별 설치/해제·동의 대화상자·CLI 누락 경고 | complete(실제 사용자 홈·실제 프로젝트 쓰기는 4.2) |
| settings-18 | Interface IDE 통합 스위치(설명 포함) | complete |
| settings-21 | Notifications 시스템 알림 설정 열기 버튼 | complete(실제 시스템 설정 앱은 4.2) |
| settings-22 | Notifications 시험 알림과 suppressed 사유 toast | partial(전달 성공·플랫폼 오류 App 검사는 shell-91 플랫폼 override 병합 뒤, 6절) |
| settings-82 | `settings.notificationsDevHint`(DEV 빌드 안내) | complete |
| settings-54 | Performance section(개발 게이트) 읽기/초기화 | complete(TS 전용 블록은 결정 6에 따라 not-applicable) |

## 2. 원본 동작 근거

- 사양 2A·2B·2H의 원본 인용을 따릅니다. 이번에 다시 확인한 원본: `src/widgets/settings-view/{settings-interface-section,agent-cli-status-row,agent-hooks-project-list,agent-hooks-project-row,settings-notification-section,settings-performance-section}.tsx`, `src/features/settings/{agent-hooks-toggle,switch-field}.tsx`, `src/entities/agent/agent.query.ts`(CLI·hooks 성공 응답으로 캐시 교체, user 범위 project `""`), `src-tauri/src/domain/agent/commands.rs:118-246`(CLI 설치·해제 정책과 오류 문구), `src-tauri/src/domain/notification/commands.rs:40-67`, `src-tauri/src/domain/app/commands.rs:20-40`.
- 순서: 배지 스위치 → CLI 행 → hooks 토글(+목록) → IDE 통합 → diff 자동 열기(`settings-interface-section.tsx:61-87`). 알림 버튼 2개는 스위치 9개 아래 가로 묶음(gap 8px), DEV 안내는 그 아래(115-125). Performance는 게이트가 켜졌을 때 TOC·카드 끝.

## 3. 구현

### 3.1 공유 crate (`crates/taide-runtime`)

- `agent_host.rs`: `AdminPrompt { Completed, Cancelled }`, `CliCommandOutcome { status, is_cancelled }`, `cli_install(exe, source, run_script)`, `cli_uninstall(exe, source, run_script)`, `run_admin_osascript(program, script)`. Tauri 명령의 macOS 분기를 옮겼습니다(번들 게이트 문구, 같은 대상이면 스크립트 생략, 없으면 현재 상태, TAIDE 소유 symlink가 아니면 거절, `PermissionDenied`면 관리자 `rm`). 사용자 취소는 오류가 아니라 `is_cancelled`로 돌려줍니다. Tauri 파일은 바꾸지 않았습니다.
- `app_actions.rs`: `perf_snapshot(registry)`, `perf_reset(registry)`(앱 manifest 불변).

### 3.2 UI crate (`native/taide-native-ui`)

- `settings-controls.rs`: `Section::Performance`(제목 `settings.performance`, 설명 `settings.performanceDescription`), `HostedChange::{AgentHooks, IdeIntegration}`(기존 `UpdateSettings` → `update_control_settings` → reconcile 경로).
- `settings-sections-data.rs`: `HookTarget { project, agent }`, `HookChange`, `Query::{CliStatus, HooksStatus(HookTarget)}`(60초), `Busy::{CliInstall, CliUninstall}`, 대상·종류별 hooks 진행 맵, `Action::{CliInstall, CliUninstall, Hooks, OpenNotificationSettings, SendTestNotification, PerfRead, PerfReset}`, 응답 소비(CLI 응답 상태로 교체·취소 무음·실패 고정 문구, hooks 성공 응답으로 그 대상만 교체·실패 `describe`, 시험 알림 사유 → `suppression_label` warning·전달 success `notification.enableHint`), `Notice::Warning`, 앱이 매 프레임 넣는 `observe_window_focus`·`observe_perf_gate`, `perf_json`(`JSON.stringify(x, null, 2)`와 같은 들여쓰기·정수 표기).
- `settings-sections-view.rs`: `Host`에 `projects`·`is_macos`·`is_dev_build`·`is_perf_enabled`. `interface`(CLI 행, hooks 토글과 목록, claude 프로젝트 행·동의 AlertDialog 기본 크기, user 행, IDE 스위치), `notifications`(ExternalLink·Bell outline xs 버튼, DEV 안내), `performance`(읽기·초기화, `settings.performanceNative` 제목 + monospace JSON 가로 스크롤). `OwnerState`에 section 수준 `hooks_consent`, `perf_ticket`·`perf_snapshot`·`is_perf_busy`.
- `settings-view.rs`: `Views::show_with_host`가 `Host<'_>`를 받고, Interface는 배지 스위치 다음에 호스트 행을 끼우며, Notifications 끝에 버튼 묶음, `Section::Performance`는 게이트가 켜질 때 끝에 붙습니다. 기존 `switch_control`을 `switch_lines(label, hints: [(문구, 색)], checked, enabled)`로 일반화해 위임합니다(기존 호출 동작 동일, 비활성은 Radix처럼 스위치만 50% 흐림).
- `icons.rs`: `Icon::{ExternalLink, Bell}`(lucide path로 만든 `resources/icons/{external-link,bell}.svg`, `LICENSE.txt` 따름). `Braces`·`Hash` 색인을 23·24로 이동.

### 3.3 앱 (`native/taide-native-app`)

- `settings-ports.rs`: `Ports::{agents: Arc<remote_agents::Ports>, cli: CliPorts, open_notification_settings, perf: &'static PerfRegistry}`. `CliPorts` 기본값은 macOS에서 `current_exe()` + `/usr/local/bin/taide` + `/usr/bin/osascript`, 그 밖 OS는 Tauri 비 macOS 명령의 문구로 거절. `open_notification_settings(is_macos, open)`는 macOS면 `x-apple.systempreferences:com.apple.Notifications-Settings.extension`를 `/usr/bin/open --`로 열고, 그 밖은 `error.notification.settingsUnsupported` 지역화 오류.
- `domains/settings.rs`: `execute_sections`에 CLI 상태(`run_blocking_result`)·설치·해제, hooks 상태·설치·해제(`remote-agents.rs`와 같은 `AgentHookInstallPorts` 조립), 알림 설정 열기, 시험 알림(`notification_notify(Error, services.platform)`, 제출 프레임의 `is_any_window_focused`), perf 읽기·초기화. `tick_settings_reads`가 포커스와 perf 게이트를 `Data`에 넣고, `show_settings_tab`이 `Host { projects: open_projects, is_macos: cfg!(target_os = "macos"), is_dev_build: cfg!(debug_assertions), is_perf_enabled }`를 넘깁니다. `Notice::Warning` → `toasts.warning`.
- `main.rs`(공용, 1줄): `taide_infra::perf::init(std::iter::empty())`(HTML 도우미 분기 뒤).
- `application.rs`·`host.rs`는 고치지 않았습니다.

## 4. 사양·원본과 다른 점(근거)

1. 포커스와 perf 게이트: 사양 2.2는 `Host`가 `perf_enabled`를 직접 받는 설계지만, `AppSurfaces`에 포트 필드를 늘리지 않으려고(공용 파일) 앱이 `tick_settings_reads`에서 `Data`에 매 프레임 기록하고 `show_settings_tab`이 그 값을 `Host::is_perf_enabled`로 넘깁니다. 시험 알림의 `any_window_focused`도 같은 프레임 값입니다.
2. CLI 관리자 대화상자 취소: 원본은 성공 문구를 보이지만 결정(acknowledge 13줄)대로 상태만 교체하고 문구를 내지 않습니다.
3. osascript 경로: 원본은 PATH의 `osascript`, native는 `/usr/bin/osascript`(사양 2.7). 알림 설정은 `NativePlatform::open_url`이 http(s)·file만 허용하므로 넓히지 않고 `/usr/bin/open --`을 직접 씁니다(사양 2.7).
4. CLI 상태 조회는 macOS가 아니어도 합니다. 원본도 `useQuery`가 `IS_MAC` 반환보다 앞에 있고 user 행이 같은 조회를 씁니다.
5. 동의 대화상자는 토글이 꺼지거나 그 프로젝트가 목록에서 빠지면 닫힙니다(원본 행 unmount와 같음).
6. user 행 비활성은 스위치만 흐리고 라벨·힌트 색(경고 포함)은 유지합니다(Radix `Switch disabled`). S2의 auto-tab은 기존처럼 `add_enabled_ui`로 감쌉니다.
7. Performance 결과는 section 수준 상태라 테마·스니펫 편집기로 바뀌면 사라지고, 그 사이 도착한 응답은 ticket이 달라 버립니다(원본 section unmount).
8. Performance 숫자 표기: Rust `f64` 1.0을 원본 화면처럼 `1`로 보이게 serde_json 형식기의 `write_f64`만 바꿨습니다(안전 정수 범위). 필드 순서는 직렬화 순서 그대로입니다.
9. DEV 안내(결정 2 A): 문구는 원문("Terminal's name and icon")을 유지합니다. native dev 빌드는 `osascript display notification`으로 보내므로 macOS가 발신자를 Script Editor로 보이는 차이가 있습니다(실기 확인은 4.2).
10. 시험 알림 플랫폼: 별도 포트를 만들지 않고 shell-91 설계와 같은 `services.platform`(`notification_notify`)을 씁니다. 이 worktree에는 shell-91의 생성자 플랫폼 override가 없어 전달 성공·플랫폼 오류를 실제 App에서 가짜 플랫폼으로 검사하지 못했습니다(6절).
11. 검사 고정: (a) hooks App 검사는 프로젝트를 `root_missing`으로 넣어(`Scene::launch_with(.., false)`) 시작 watcher 복원이 그 프로젝트를 붙이지 않게 했습니다. 붙이면 `agent_hooks::reconcile_installed`가 실제 `HOME`으로 돌고, 검사에서 토글을 켠 뒤 실행되면 사용자 홈 hooks를 고칠 수 있기 때문입니다(세션 `ProjectRef`는 그대로라 화면은 같음). (b) 이 기계에 실제 `/usr/local/bin/taide`가 있어 새 CLI 행의 `Installed` 문구가 S2 LSP App 검사 2개의 문구 수·대기 조건과 겹쳤습니다. 두 검사에 없는 경로를 보는 `missing_cli_agents()` 포트를 넣어 고쳤고, 새 S1 App 검사도 모두 가짜 CLI 상태를 씁니다. 그 밖의 기존 App 검사는 기본 포트로 실제 `/usr/local/bin/taide`의 symlink 메타데이터를 읽기만 합니다(쓰기 없음).
12. 실패 검사 선행: 구현과 검사를 같은 단계에서 작성했고, 구현 전 실패 실행은 하지 않았습니다. 실행으로 잡은 검사 결함 2건(UI 검사의 `Codex` 문구가 AI section과 겹침, App 검사에서 CLI가 필요한 에이전트를 codex로 잘못 가정 — 실제는 gemini가 HTTP 전달이라 CLI 필요)은 검사를 고쳤습니다.

## 5. 검증

실행 명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/settings`입니다.

| 범위 | 명령 | 결과 |
| --- | --- | --- |
| [C] runtime CLI | `cargo test --manifest-path Cargo.toml -p taide-runtime --lib -- agent_host app_actions` | 7 passed(새 5: 번들 밖 거절, 같은 대상 생략·취소·성공·실패, 없음·소유 아님·직접 삭제, 권한 거부 → 관리자 `rm`·취소, `/usr/bin/true`·`/usr/bin/false` 실행기) |
| src-tauri | `cargo check --manifest-path Cargo.toml -p taide` | 통과 |
| [C][U] UI sections | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection -- settings_sections sections_view` | 47 passed(새 data 5, view 7) |
| UI lib 전체 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --features native-host,inspection` | 214 passed(아이콘 전수 래스터·`switch_control` 위임 회귀 포함, 형식 정리 뒤 재실행) |
| UI 통합 | `... --features native-host,inspection --test settings-code-controls --test snippet-editor` | 1 + 18 passed |
| [A][C] 앱 settings 영역 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- settings_ theme_ snippet_host_tests app_file_views remote_preferences presentation remote_agents remote_sync remote_http remote_dispatch agent_hooks --test-threads=2` | 74 passed(새 App 5·`settings_ports` 1 포함, 형식 정리 뒤 재실행) |
| 앱 통합 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test settings-controls` | 2 passed |
| 컴파일 | `cargo check --all-targets`(UI `--features native-host,inspection`, 앱 기본) | 통과, 경고는 기존 것뿐(`tests/workbench.rs` unused mut, vendor wry, `is_applying`, `zen.rs`, `fixture_with_emmet_peek`) |
| 동결 미러 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 통과, 경고는 기존 `browser-editor.rs`·`browser-application.rs` |
| 형식 | `cargo fmt -- --check`(UI, 앱), `cargo fmt --manifest-path Cargo.toml -p taide-runtime -- --check` | 차이 없음 |
| lockfile | `git status --short` | `Cargo.toml`·`Cargo.lock` 변경 없음 |

### 5.1 이어받은 실행의 재검증 (2026-11-03)

같은 묶음을 다시 배정받아 미커밋 상태를 그대로 이어받았습니다. 코드는 바꾸지 않았고, 현재 파일 상태에서 위 표의 명령을 다시 실행했습니다(모두 `--locked --offline --target-dir .../targets/settings`).

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --all-targets`(UI `--features native-host,inspection`, 앱, `-p taide-runtime`) | 통과, 경고는 위 표의 기존 것뿐 |
| `cargo check --quiet -p taide` | 통과 |
| `cargo test -p taide-runtime --lib -- agent_host app_actions` | 7 passed |
| `cargo test`(UI `--lib --features native-host,inspection`, 필터 없음) | 214 passed |
| `cargo test`(UI `--test settings-code-controls --test snippet-editor`) | 1 + 18 passed |
| `cargo test`(앱 `--lib -- settings_ theme_ snippet_host_tests app_file_views remote_preferences presentation remote_agents remote_sync remote_http remote_dispatch agent_hooks --test-threads=2`) | 74 passed(새 S1 App 5 포함), 흔들림 재실행 없음 |
| `cargo test`(앱 `--test settings-controls`) | 2 passed |
| `cargo fmt -- --check`(UI, 앱, `-p taide-runtime`) | 차이 없음 |
| `cargo check`(`taide-remote-web` wasm32 `--lib`) | 통과, 기존 `browser-editor.rs`·`browser-application.rs` 경고뿐 |

- 새 코드의 일반 주석·`#[allow]`·`todo!` 없음(추가 줄과 새 파일 grep). `Cargo.toml`·`Cargo.lock` 변경 없음.
- shell-91 플랫폼 override(`native-notifications.rs` 등)는 여전히 이 worktree에 없어 settings-22는 partial을 유지합니다(6절).

- [C] `settings-sections-data-tests.rs`: CLI 진행·취소 무음·응답 상태 교체·실패 고정 문구 4종, hooks 대상별 조회·진행·그 대상만 교체·실패 `describe`, 시험 알림 포커스 값 전달·사유 3종 → 스위치 라벨·전달·오류, 알림 설정 열기 오류, perf JSON 원본 형식·요청자 ticket·초기화·실패, 제출 실패 응답 9종.
- [C] `settings-ports-tests.rs`: macOS면 원본 URL 1회, 그 밖 `error.notification.settingsUnsupported`(InvalidArgument), opener 오류 전달.
- [U] `settings-sections-view-tests.rs`: 미러에는 새 행 없음·비 macOS는 CLI 행 없음·배지 → CLI → hooks → IDE → diff 순서·IDE 패치, CLI 4상태 문구·색·버튼(끊어진 링크는 설치가 해제 왼쪽)·진행 중 비활성·요청, hooks 토글 off 목록 없음·on 5종 순서·`app.noProjectOpen`·user 4종 조회, 프로젝트 행 조회 중(+2)·설치됨 색·해제 진행 비활성·동의 대화상자(첫 포커스 취소, 취소가 확인 왼쪽, 바깥 클릭 무시, Escape 무요청·여는 버튼으로 포커스 복귀, 확인 즉시 닫힘·그 프로젝트 설치 요청·설치 중 다시 열리지 않음, 토글 off로 닫힘), user 행 비활성 4경우·경고 색·설치됨이면 해제 가능·요청, 알림 버튼 위치·DEV 안내 색·위치·요청과 포커스 값, Performance 게이트 off 없음·on 마지막(Remote 다음)·진행 중 비활성·JSON·초기화.
- [A] `application-settings-interface-tests.rs`(실제 `NativeApplication`, 경로 main.rs → `NativeApplication` → `show_window` → `show_settings_tab` → `Views::show_with_host` → `sections_view::{interface, notifications, performance}` → `load_settings_requests` → `domains::settings::dispatch`):
    1. 가짜 `CliPorts`: 취소 → toast 없음, 설치 실패 → `settings.cliInstallFailed`, 성공 → `settings.cliInstallSuccess`·해제 버튼·`Installed`, 해제 실패 → `settings.cliUninstallFailed`, 성공 → `settings.cliUninstallSuccess`·설치 버튼.
    2. 가짜 reconcile bridge·가짜 home·가짜 emitter·CLI 없음: 토글 클릭 → settings.json `agentHooksEnabled: true`·reconcile `(false, true)` 1회·gemini CLI 누락 경고 1개, claude 행 설치 → 동의 확인 → 임시 프로젝트 `.claude/settings.local.json`에 TAIDE 항목 → 해제로 제거, codex 스위치 → 가짜 home의 codex hooks 파일에 marker → 해제로 제거, gemini 설치 요청 → `taide CLI is not installed yet` toast, 토글 off → `(true, false)`·목록 사라짐 → claude 설치 요청 → `agent hooks are disabled in settings` toast·파일 불변.
    3. IDE 스위치 클릭 → settings.json `ideIntegrationEnabled` 반전·reconcile (현재, 변경) 1회·hooks 재조정 없음.
    4. 가짜 opener 1회 호출, 두 번째 오류 → 그 문구 toast. 시험 알림: 메인 viewport 포커스(검사 전 단언) → warning `settings.notificationsOnlyWhenUnfocused`, 마스터 off → `settings.notificationsEnabled`, `notifyError` off → `settings.notificationsError`. 세 경우 모두 결정 함수가 플랫폼 호출 전에 억제합니다.
    5. 기본 전역 레지스트리(게이트 off)에서는 Performance 없음 → 주입 레지스트리(enabled, span 1회) → TOC·카드 끝 → 읽기 → JSON에 그 항목 `"count": 1` → 초기화 → 블록 사라짐·레지스트리 0.

## 6. 남은 범위

- settings-22: 전달 성공(`notification.enableHint` success, 플랫폼 1회·제목·본문, 첫 전달 info 없음)과 플랫폼 오류 toast의 App 검사는 shell-91의 생성자 플랫폼 override(shell spec 2.0.3)가 이 worktree에 없어 병합 뒤 연결·검사합니다. 코드 경로(`notification_notify(Error, services.platform)`)와 응답 → 알림 매핑([C])은 끝났습니다.
- 4.2 허락 항목은 하지 않았습니다: 실제 `/usr/local/bin/taide` 설치·해제와 관리자 암호, 실제 사용자 홈 hooks·실제 프로젝트 파일, 실제 OS 알림과 시스템 설정 앱, 실기 포커스·접근성.
- settings-54의 TS 실행 환경 전용 블록(프런트엔드 계측·프레임 probe·Monaco 모델 수·query cache 수)은 결정 6에 따라 not-applicable입니다.
- 대화상자 등장·퇴장 애니메이션은 S3·S4와 같이 넣지 않았습니다.
