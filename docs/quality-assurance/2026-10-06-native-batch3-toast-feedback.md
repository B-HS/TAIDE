# native 전환 배치 3 단계 1 — toast 일반 API와 status 문자열의 toast 이전

- 작성일: 2026-10-06
- 기준 커밋: 09ced3a4 (작업 트리는 이 단계 시작 시 깨끗했습니다)
- 단계: toast-feedback (배치 3, 구현 단계 1/2)
- 줄 번호는 이 단계 종료 시점의 파일 기준입니다.

## 1. 변경 파일

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-ui/src/toast.rs` | Info 종류, 일반 API(`success`·`error`·`info`·`notify`), 액션 버튼, `is_showing`, `take_actions`, `describe_error` 공개, 신규 테스트 4건 |
| `native/taide-native-app/resources/toasts/info.svg` | sonner InfoIcon 경로(신규, 같은 폴더의 MIT LICENSE.md 적용 대상) |
| `native/taide-native-app/src/toast.rs` | 앱 액션 식별자 `Action`, `Toasts` 별칭, 안내·오류 변환 함수 6개 |
| `native/taide-native-app/src/toast-tests.rs` | 신규 테스트 6건 |
| `native/taide-native-app/src/application.rs` | status 대입 106곳 중 50곳 이전(toast 39, 로그 11), 보고 보조 메서드, toast 액션 처리 |
| `native/taide-native-app/src/explorer.rs` | `retry_create`·`retry_rename` |
| `native/taide-native-app/src/terminal_surface.rs` | 실패 화면 문구 로컬라이즈, debug 포맷 문구 제거, `view.error` 도색 제거, 테스트 1건 |

수정하지 않은 것: `Cargo.toml`·`Cargo.lock`, taide-locale 카탈로그(필요한 키가 모두 있었습니다), `taide-remote-web`, `host.rs`·`terminal_host.rs`·`projects.rs`(변경 불필요).

## 2. F1 — toast 일반 API

### 2.1 TS sonner 사용 전수 조사

검색 대상: `src` 의 테스트 제외 소스(`rg "toast\.(error|success|info|warning|message|promise|loading|dismiss|custom)\(|toast\("`).

| 기능 | TS 사용 | 근거 |
| --- | --- | --- |
| 종류 | error·success·info·warning 4종 | 62개 파일의 호출 전체 |
| description (문자열) | 사용 | `src/entities/settings/settings.query.ts:26`, `src/entities/sync/sync.commands.ts:29`, `src/widgets/git-panel/git-panel-container.tsx:193,196`, `src/features/editor/ai-inline-edit.ts:273` |
| description (JSX 여러 줄) | 1곳 | `src/widgets/search-panel/search-panel-container.tsx:145-156` |
| action `{ label, onClick }` | 3곳 | `src/entities/sync/sync.commands.ts:30`, `src/widgets/explorer/use-explorer-entry-crud.ts:127,174` |
| duration | 미사용(기본 4000ms) | `rg "duration: "` 결과에 toast 옵션 없음, `node_modules/sonner/dist/index.mjs` `TOAST_LIFETIME = 4000` |
| id 갱신·중복 제거, dismiss, promise, loading, custom, cancel | 미사용 | 위 검색 0건 |
| Toaster 설정 | `richColors`, `closeButton`, theme, position | `src/widgets/app-toaster/app-toaster.tsx:14-20` |

### 2.2 구현

- `Kind` 를 공개하고 `Info` 를 추가했습니다. 색은 `node_modules/sonner/dist/styles.css` 의 `--info-bg/border/text` 를 기존 상수와 같은 방식(HSL → sRGB 반올림)으로 옮겼습니다. light `(240,248,255)/(221,231,253)/(9,115,220)`, dark `(0,13,31)/(25,35,62)/(88,150,243)`.
- 아이콘은 sonner `InfoIcon` 경로를 `info.svg` 로 추가했습니다.
- `Toasts<A = NoAction>` 로 액션 식별자 타입을 받습니다. `Toasts::new()` 는 `Toasts<NoAction>` 전용으로 남겨 기존 호출부와 `taide-remote-web` 을 그대로 둡니다. 액션을 쓰는 호출부는 `Toasts::<A>::with_actions()` 를 씁니다.
- 일반 API: `success(title, now)`, `error(title, now)`, `info(title, now)`, 기존 `warning(title, now)`, 그리고 `notify(kind, title, Options { description, action }, now)`.
- 액션은 클로저 대신 `Action { label, id }` 의 `id` 를 보관하고, 버튼을 누르면 `take_actions()` 가 한 번만 돌려준 뒤 toast 를 닫습니다(sonner `onClick` 뒤 `deleteToast()` 와 같음: `index.mjs` 의 `data-action` 버튼 처리).
- 버튼 외형은 sonner `[data-button]` 규칙을 따릅니다: 높이 24, 좌우 padding 8, 글자 12px, radius 4, 오른쪽 정렬(`margin-left:auto`), 배경 `--normal-text`·글자 `--normal-bg`(light `(23,23,23)`/흰색, dark `(252,252,252)`/검정), focus ring `0 0 0 2px rgba(0,0,0,.4)`. 버튼이 있으면 본문 너비가 `gap 6 + 버튼 너비` 만큼 줄고, 카드 높이는 `max(본문, 24) + 34` 입니다.
- 버튼 위에서 누르면 swipe 를 시작하지 않습니다(sonner `onPointerDown` 의 `tagName === 'BUTTON'` 예외와 같음).
- `is_showing(kind, title)`: 닫히지 않은 같은 toast 가 있는지 답합니다. TS 에는 대응 기능이 없고, F3 의 일회성 요구를 앱 쪽에서 구현하기 위한 조회입니다.
- `describe_error` 를 공개해 인라인 오류·터미널 실패 화면이 toast 와 같은 문구를 쓰게 했습니다.
- `show` 의 반환형(`AppResult<()>`)은 바꾸지 않았습니다. 동결된 `taide-remote-web/src/browser-editor.rs` 가 반환형에 의존하므로, 액션 결과는 `take_actions()` 로 꺼냅니다.
- 지속 시간(4000ms)·종료 200ms·3장 쌓임·위치·모션·swipe 는 기존 구현 그대로입니다.

### 2.3 기존 테스트 기대값 변경 1건

`snippet_toast는_성공아이콘과_원본_검증_및_오류_메시지를_보존한다` 의 `assert_eq!(toasts.assets.textures.len(), 3)` 을 4로 바꿨습니다. 아이콘 수를 세는 단언이고, TS 는 4종(`toast.info` 13회 호출, sonner `InfoIcon`)을 쓰므로 3이라는 기대값이 TS 기준과 맞지 않았습니다. 그 밖의 기존 테스트 25건은 수정하지 않았습니다.

## 3. F2 — status 대입 분류표

이전 방식 표기: T-ERR = `toast.error(describeIpcError(error))` 와 같은 `Toasts::ipc_error`, ONCE = 같은 오류 toast 가 살아있는 동안 한 번만, LOG = 화면 표시 없이 `log::warn!`, KEEP = 근거 없음으로 기존 status 유지.

전체 106곳은 `self.status = Some(` 102곳에 여러 줄 대입 1곳, 지역 변수 `status` 대입 1곳, 클로저 안 `*status` 대입 1곳, 생성자 초기값 1곳을 더한 수입니다.

### 3.1 toast 로 옮긴 지점 (39곳)

| application.rs | 상황 | TS 근거 | 이전 방식 |
| --- | --- | --- | --- |
| 517 | shell mutation 비동기 오류(`controller.take_error`) | `src/widgets/app-shell/app-shell.tsx:89,95,102`, `src/widgets/app-shell/use-project-drag.ts:127` | T-ERR |
| 681 | 터미널 파일 링크·Problems 항목 열기 실패 | `src/entities/layout/layout.query.ts:270-273`(useOpenFileTab), `src/widgets/terminal-pane/terminal-session.tsx:224-231`, `src/widgets/editor-area/editor-area.tsx:348-351` | T-ERR (기존 `terminal.openLinkFailed` 문구는 TS 에서 URL 링크 전용이라 제거) |
| 716 | 터미널 attach(spawn) 실패 | `terminal-session.tsx:177-182` | T-ERR + 화면 내 실패 문구 |
| 1016 | untitled 저장(Save As) 실패 | `src/widgets/editor-pane/untitled-pane.tsx:133,135` | T-ERR |
| 1084 | 원본 없는 draft 저장 실패 | `src/widgets/editor-pane/editor-pane.tsx:217` | T-ERR |
| 1117, 1121, 1897 | 탭 닫기 실패(단일은 즉시, 일괄은 끝에서 첫 실패 1건) | `src/widgets/editor-area/use-request-close-tab.tsx:129-135` | T-ERR / 일괄은 `Toasts::error(첫 실패 문구)` |
| 1142 | 닫기 확인의 저장(mirror 탭) 실패 | `use-request-close-tab.tsx:152-158` | T-ERR |
| 1281 | 파일 저장 실패 | `src/widgets/editor-pane/use-editor-file-persistence.ts:460-463` | T-ERR |
| 1343 | 복사 실패 | `src/shared/lib/copy-text-to-clipboard.ts:27` | `toast.error(common.copyFailed)` |
| 1348 | 터미널 URL 링크 열기 실패 | `terminal-session.tsx:220-222` | `toast.error(terminal.openLinkFailed)` |
| 1357 | 시스템 열기·Finder 표시·브라우저 열기 실패 | `src/widgets/preview-pane/preview-pane.tsx:59`, `editor-pane.tsx:407`, `src/widgets/explorer/explorer-container.tsx:207-208` | T-ERR |
| 1375 | `HostReply::Failed`(host 명령 비동기 실패) | `src/shared/lib/command-registry.ts:106-114`, `layout.query.ts:124,161`, `explorer-container.tsx:100` | T-ERR |
| 1652, 3300 | 저장 준비 실패(편집기 스냅샷) | `use-editor-file-persistence.ts:460-463` | T-ERR, 자동 저장은 ONCE |
| 2687 | 자동 저장 스냅샷 실패 | 같은 근거 | ONCE |
| 1733 | host 제출 실패(닫는 중·큐 가득) | `command-registry.ts:106-114` (F3) | ONCE |
| 1758 | dirty flush 제출 실패 | 같은 근거 (F3) | ONCE |
| 1825 | pinned 탭 닫기 차단 | `src/widgets/editor-area/editor-area.tsx:133` | `toast.warning(tab.pinnedCloseBlocked)` |
| 1973 | 저장 요청 준비 실패 | `use-editor-file-persistence.ts:460-463` | T-ERR |
| 2228 | 닫기 확인의 저장 준비 실패, 읽기 전용 | 같은 근거, 읽기 전용은 `use-editor-file-persistence.ts:363-366` | T-ERR, 읽기 전용은 `editor.readOnlySaveBlocked` 키의 Localized 오류 |
| 2313, 2411 | Save As 대상이 프로젝트 밖 | `untitled-pane.tsx:133,135`, `editor-pane.tsx:217` | T-ERR |
| 2343 | untitled 저장 스냅샷 실패 | `untitled-pane.tsx:133,135` | T-ERR |
| 2837, 4367 | 탐색기 붙여넣기 실패 | `src/widgets/explorer/use-explorer-clipboard.ts:72-73` | T-ERR |
| 2869, 1466 | 탐색기 항목 생성 실패 | `src/widgets/explorer/use-explorer-entry-crud.ts:124-127` | `toast.error(message, { action: common.retry })` + 인라인 오류도 같은 문구 |
| 2880, 1477 | 탐색기 이름 변경 실패 | `use-explorer-entry-crud.ts:171-174` | 같은 방식 |
| 3127, 4569 | 탐색기 삭제 실패 | `use-explorer-entry-crud.ts:186-189` | T-ERR |
| 3515, 4393, 4427, 4442 | 프로젝트 없이 파일 열기·설정·터미널 요청 | `app-shell.tsx:92,116`, `src/widgets/command-palette/command-palette.tsx:189,203,217,340`, `layout.query.ts:150` | `toast.info(app.openProjectFirst)` |
| 4498 | shell mutation 동기 제출 실패(명령·UI 공통) | `app-shell.tsx:89-102`, `command-registry.ts:106-114` | T-ERR (직전 배치의 명령/비명령 분기 제거) |
| 4595 | Zen 해제 mutation 실패 | `app-shell.tsx:89-102` 의 shell mutation 패턴 | T-ERR |

### 3.2 화면 표시를 없애고 로그로 옮긴 지점 (11곳)

| application.rs | 상황 | TS 근거(화면 표시 없음) |
| --- | --- | --- |
| 264 | 시작 경고(상태 복원·글꼴) | `src-tauri/src/lib.rs:917-919` 가 `restore_state` 경고를 `log::warn!` 으로만 남김. 글꼴 대체는 TS 에 알림 없음 |
| 3644 | 터미널 글꼴 갱신 경고 | 같은 근거 |
| 695, 699 | 터미널 붙여넣기·클립보드 읽기 실패 | `src/widgets/terminal-pane/terminal-pane.tsx:163-168` `.catch(() => undefined)` |
| 722 | 터미널 크기 변경 실패 | `terminal-session.tsx:187,198` `.catch(() => undefined)` |
| 844, 2707 | draft mirror 실패 | `use-editor-file-persistence.ts:644`, `untitled-pane.tsx:180` `.catch(() => false)` |
| 1228 | 저장 후 LSP 알림 실패 | `use-editor-file-persistence.ts:458` `void notifyLspSessionsOfSave()` |
| 1620, 3267, 3272 | 저장 시 포맷·코드 액션 참여 실패 | `use-editor-file-persistence.ts:374,379` `.catch(() => undefined)` |

작업 지시는 시작 경고를 toast 로 옮기라고 했으나 TS 근거는 로그였습니다. TS 를 따라 로그로 옮겼습니다.

### 3.3 근거 없음으로 status 를 유지한 지점 (56곳)

| application.rs | 상황 | 유지 사유 |
| --- | --- | --- |
| 527 | app file 뷰와 편집기 저장소 정합 실패 | 편집기 저장소 내부 오류, TS 대응 없음 |
| 636, 640, 643 | 테마·로케일 적용 실패 | TS 는 toast 가 아니라 `StatusErrorBanner`+재시도(`src/app/providers/theme-provider.tsx:102-106`, `locale-provider.tsx:27-28`). native 에 배너가 없어 기존 유지 |
| 882 | 외부 변경 관찰 실패 | TS 대응 표시를 확인하지 못함 |
| 925, 2529 | 디스크 충돌 선택 실패 | TS 는 로컬 상태 연산이라 실패 경로 없음 |
| 1061 | draft 저장 후 문서 갱신 실패 | 편집기 저장소 내부 오류 |
| 1161, 1164, 1323, 1338 | 트리 동기화·조회·토글 실패 | TS 는 `rootUnavailable` 인라인 상태(`explorer-container.tsx:63,88`), `src/entities/tree` 에 toast 없음. native 에 인라인 상태가 없어 기존 유지 |
| 1275 | 닫기 저장 중 문서 변경 | native 전용 상태 |
| 2274, 2327, 2333, 2425, 2431, 4462, 4491 | 문서 미로드, 대상 draft 미저장, 경로 UTF-8 아님 | native 전용 가드 |
| 2768, 2779, 2802, 2806, 3214 | LSP 모델 동기화·브리지 실패 | TS LSP 계층의 표시 여부를 확인하지 못함 |
| 2980, 2983, 3101, 3172 | workspace 이름 변경·삭제의 문서 반영 실패·경고 | 편집기 저장소 내부 단계. 실패는 요청 완료 경로(2880, 3127)에서 toast 로도 보고됨 |
| 3209 | workspace edit 적용 실패 | TS `editor.workspaceEditApplyFailed`(`src/shared/lib/lsp/adapters/code-action.ts:176-179`)는 code action 한정이라 native 경로와 대응을 확정하지 못함 |
| 3504, 4538, 4901, 4925, 5161, 5424, 5439, 5486, 5685, 5690 | 팔레트·키바인딩·Problems·탐색기·터미널·상태바·문서 그리기 오류 | 렌더 단계 내부 오류, TS 대응 없음. 매 프레임 재발 가능 |
| 3605 | 테마 미리보기 적용 실패 | TS 대응을 확인하지 못함 |
| 3625, 3664, 4021, 4181, 4203, 4601, 5629 | 터미널 입력 flush·커서 설정·keymap 라우팅 실패 | native 전용 내부 오류 |
| 3647 | 터미널 글꼴 로드 실패 | native 전용 |
| 3739, 3757, 3778, 3805, 3812 | 종료 절차·브리지 재연결 실패 | TS 대응을 확인하지 못함 |
| 4631 | web preview 동기화 실패 | native 전용 |
| 4642 | toast 그리기 실패 | toast 자체 실패라 toast 로 보고할 수 없음 |

status 를 그리는 지점은 `AppSurfaces::status_bar` 의 `ui.label(status)` 한 곳(5441)이며, 위 56곳 때문에 남겼습니다.

## 4. F3 — 비동기 실패 경로

- `HostReply::Failed`(1375), `controller.take_error`(517), shell mutation 동기 실패(4498)를 `NativeApplication::report` → `Toasts::ipc_error` 로 보냅니다. 응답 큐에서 한 번 꺼낸 값만 처리하므로 응답당 toast 1개입니다.
- host 제출 실패(1733)와 dirty flush 실패(1758), 자동 저장 준비 실패(2687, 1652, 3300 의 자동 저장 분기)는 재시도마다 다시 도달할 수 있어 `report_once` → `crate::toast::error_once` 로 보냅니다. 같은 문구의 오류 toast 가 살아있는 동안에는 새로 만들지 않습니다.
- 일괄 탭 닫기는 실패마다 toast 를 만들지 않고 첫 실패 문구를 `batch.failure` 에 담아 끝에서 1개만 만듭니다(TS `failures[0]`).

## 5. F4 — 터미널 오류 표시

| 대상 | 이전 | 이후 | TS 근거 |
| --- | --- | --- | --- |
| attach 실패 화면 문구 | `error.to_string()` | `describe_error(locale, error)` | `terminal-session.tsx:87,388-396` (`useIpcErrorMessage(failure)`) |
| `Phase::Failed(failure)` | `"native terminal failed: {failure:?}"` | `terminal.processExited`(종료 코드 없음) | `terminal-session.tsx:381-384,399-407`, `src-tauri/src/domain/terminal/commands.rs:49-55` (세션이 끝나면 code 와 함께 `TerminalExited` 만 발행) |
| `view.error`·입력 큐 오류의 화면 도색 | 터미널 좌상단에 원문 도색 | 도색 제거, 문구가 바뀔 때만 `log::warn!` | `terminal-session.tsx:106,187,198,212,217` 의 `.catch(() => undefined)` |

`ended_message` 순수 함수로 분리해 테스트했습니다. `view.error` 값 자체는 그대로 유지합니다(기존 테스트가 상태를 확인).

## 6. F5 — 시작 경고와 안내

- 시작 경고: 3.2 의 264 참조.
- `app.openProjectFirst`: 3.1 의 4곳을 `toast.info` 로 옮겼습니다(팔레트 파일 열기, 설정 파일, 설정, `ShowOpenProjectNotice`).

## 7. 탐색기 재시도 액션

TS 의 재시도는 실패 당시의 대상과 이름으로 다시 제출합니다(`use-explorer-entry-crud.ts:127,174` 의 `commitDraft(trimmedName)`·`commitRename(trimmedName)`). native 는 실패한 요청을 `crate::toast::Action::RetryCreate`·`RetryRename` 에 싣고, 버튼을 누르면 `NativeApplication::run_toast_actions` 가 `Explorer::retry_create`·`retry_rename` 으로 이름을 다시 검증한 뒤 새 토큰으로 제출합니다. 진행 중인 요청이 있으면 제출하지 않습니다.

## 8. 실행한 검증

| 순서 | 명령 | exit | 결과 |
| --- | --- | --- | --- |
| 1 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib 일반_toast --locked --offline --target-dir experiments/native-shell-spike/target` (구현 전) | 101 | 컴파일 오류 24건(`Options`·`Action`·`Kind::Info`·`with_actions` 없음). 실패하는 테스트를 먼저 확인 |
| 2 | 같은 crate `--lib toast` | 0 | 29 passed, 0 failed, 0.03s (기존 25 + 신규 4) |
| 3 | `cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir …` | 0 | 45.42s, 앱 crate 경고 없음 |
| 4 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib toast --locked --offline --target-dir …` | 0 | 6 passed, 337 filtered out, 0.03s |
| 5 | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 1 → 0 | 1곳 차이 수정 뒤 통과 |
| 6 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 1 → 0 | `toast-tests.rs` 3곳 차이 수정 뒤 통과 |
| 7 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --locked --offline --target-dir …` | 0 | 91 passed, 0 failed, 0.38s |
| 8 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir …` | 0 | 343 passed, 0 failed, 15.14s |
| 9 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir …` | 0 | 14.68s |
| 10 | 순서 8 이후 `terminal_surface.rs`·`application.rs` 를 추가 수정한 뒤 두 crate 의 `cargo fmt … -- --check` | 0 | 차이 없음 |
| 11 | 추가 수정 뒤 `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir …` | 0 | 343 passed, 0 failed, 13.88s (기준 336 + 신규 7) |
| 12 | 추가 수정 뒤 `cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir … --message-format short` 와 `… --quiet` | 0 | 앱 crate 경고 없음. 출력 형식 플래그만 덧붙인 실행입니다 |

- 순서 9 뒤로 `taide-native-ui` 는 수정하지 않았으므로 순서 7·9 결과를 최종 상태 근거로 재사용했습니다.
- V5(taide-locale 테스트)는 카탈로그를 수정하지 않아 실행하지 않았습니다.
- 경로는 모두 `/Users/hyunseokbyun/development/TAIDE` 기준 절대 경로로 실행했습니다.

## 9. 실패했다가 고친 내역

1. 포맷 검사 2회 실패(순서 5, 6): 손으로 쓴 줄바꿈이 rustfmt 결과와 달랐습니다. 차이를 그대로 반영했습니다.
2. 신규 테스트 이름에 대문자가 들어가 `non_snake_case` 경고가 났습니다. 이름을 바꿨습니다.
3. `view.error.take()` 로 한 번만 로그를 남기려던 첫 구현은 오류가 매 프레임 다시 설정되면 로그가 프레임마다 쌓입니다. 값은 유지하고 문구가 바뀔 때만 로그를 남기도록 `logged_error` 를 두었습니다.
4. `match` 조건식의 `projects.read()` guard 가 살아있는 arm 에서 `&mut self` 메서드를 부르면 차용이 겹칩니다. 그 두 곳(2313, 2411)은 `self.toasts.ipc_error(&self.locale, …)` 를 직접 호출합니다.

## 10. 남은 위험과 실기 확인

- [ ] 액션 버튼 외형(위치·색·focus ring)과 여러 장이 쌓였을 때의 클릭 대상을 실제 화면에서 확인. 접힌 뒤쪽 toast 의 버튼은 상호작용하지 않게 했습니다.
- [ ] Info toast 의 색·아이콘을 light/dark 에서 sonner 와 비교.
- [ ] 탐색기 생성·이름 변경 실패 후 재시도 버튼의 실제 동작(권한 없는 폴더 등 합성 조건).
- [ ] 터미널 spawn 실패 시 toast 와 화면 내 문구가 같은 로컬라이즈 문구인지 확인. TS 의 실패 화면 글자색(`text-status-error`)은 native `status()` 가 foreground 색을 쓰므로 다릅니다(이번 범위 밖).
- [ ] 종료 중 host 제출 실패가 "native host is closing" 원문 toast 로 보일 수 있습니다. 문구 로컬라이즈는 별도 작업입니다.
- 터미널 복사 실패의 출처 구분은 11절(리뷰 후속 1)에서 해결했습니다.
- status 라벨은 3.3 의 56곳 때문에 남아 있고 소거 코드가 없는 점도 그대로입니다.
- 검색 치환 건너뜀 toast 의 여러 줄 description(JSX)은 호출부가 native 에 없어 문자열 한 덩어리 이상의 표현을 만들지 않았습니다.

## 11. 리뷰 후속

리뷰어가 올린 차단 항목 2건을 실제 코드와 TS 근거로 다시 확인했고, 두 건 모두 옳은 지적이어서 수정했습니다. 줄 번호는 이 절 작성 시점의 파일 기준이며, 1~10절의 줄 번호와 아래 "대체되는 기록"은 이 절이 우선합니다.

### 11.1 항목 1 — 터미널 복사 실패가 오류 toast 로 표시됨 (수정)

확인한 사실:

- TS 터미널 복사는 실패를 삼킵니다: `src/widgets/terminal-pane/terminal-pane.tsx:158-161` (`navigator.clipboard.writeText(selection).catch(() => undefined)`).
- TS 에서 `common.copyFailed` toast 를 만드는 곳은 `src/shared/lib/copy-text-to-clipboard.ts:27` 한 곳이며 터미널은 이 함수를 쓰지 않습니다.
- 수정 전 native 는 터미널 메뉴 복사(`terminal_surface.rs:1945`)와 탐색기 복사(`application.rs` 의 `explorer::Action::CopyText`)가 같은 `HostCommand::CopyText` → `HostReply::CopiedText` 를 써서, 실패가 출처와 무관하게 `common.copyFailed` toast 가 됐습니다.

수정:

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-app/src/host.rs` | `HostCommand::CopyTerminalSelection(String)`(90), `HostReply::TerminalSelectionCopied { result }`(274) 추가. 두 복사 명령이 같은 쓰기 절차를 쓰므로 `write_clipboard`(1490)로 묶고 작업 이름만 다르게 넘깁니다(`native-explorer-copy-path`, `native-terminal-copy-selection`) |
| `native/taide-native-app/src/terminal_surface.rs` | 메뉴 복사가 `HostCommand::CopyTerminalSelection` 을 보냅니다(1945) |
| `native/taide-native-app/src/toast.rs` | `CopyOrigin { Explorer, Terminal }`(29), `copy_finished`(54). 실패일 때 탐색기는 `copy_failed` toast, 터미널은 `log::warn!` 만 남깁니다 |
| `native/taide-native-app/src/application.rs` | `HostReply::CopiedText`(1346)는 `CopyOrigin::Explorer`, `HostReply::TerminalSelectionCopied`(1353)는 `CopyOrigin::Terminal` 로 `copy_finished` 를 호출합니다 |
| `native/taide-native-app/tests/terminal-host.rs` | 3313 의 기대 명령을 `HostCommand::CopyTerminalSelection` 으로 바꿨습니다. 지정 범위 밖 파일이지만 터미널 메뉴 복사가 보내는 명령을 단언하는 유일한 테스트라 명령 이름 변경을 따라가야 했습니다(1줄) |

테스트: `toast-tests.rs:189` `터미널_선택_복사_실패는_toast를_만들지_않고_탐색기_복사_실패만_toast가_된다`. 쓰기가 항상 실패하는 합성 클립보드를 꽂은 `HostBridge` 에 두 명령을 보내, 터미널 명령은 `TerminalSelectionCopied` 실패 응답이 되고 toast 가 0개인지, 탐색기 명령은 `CopiedText` 실패 응답이 되고 `common.copyFailed` 오류 toast 1개가 되는지 확인합니다. 성공 응답은 toast 를 만들지 않는 것도 함께 확인합니다.

### 11.2 항목 2 — 읽기 전용 문서의 명시 저장이 debug 문구 toast 로 표시됨 (수정)

확인한 사실:

- TS: `src/widgets/editor-pane/use-editor-file-persistence.ts:361-366`. `file?.readOnly` 이면 명시 저장(`reason === 'explicit'`)만 `toast.error(t('editor.readOnlySaveBlocked'))` 를 띄우고 자동 저장은 아무것도 표시하지 않습니다. 명시 저장은 Monaco save 액션(`src/features/editor/code-editor.tsx:208`)과 닫기 확인의 저장(`use-editor-file-persistence.ts:676`) 두 경로입니다.
- native: `crate::save::keymap_request`(`save.rs:35`)가 `EditorStore::save_snapshot`(`taide-native-editor/src/store.rs:1318`)의 `EditorError::ReadOnly` 를 그대로 돌려주고, 수정 전 `request_tab_save` 는 이를 `editor_error` 로 바꿔 `native editor: ReadOnly` toast 를 만들었습니다. `save::prepare`(`taide-native-editor/src/save-preparation.rs:25`)와 포맷 재개 뒤 `save_snapshot` 실패도 같은 변환을 거쳤습니다.
- 카탈로그 키 `editor.readOnlySaveBlocked` 는 `crates/taide-locale/resources/locales/{en,ko,ja}.json:74` 에 이미 있어 카탈로그는 수정하지 않았습니다.

수정:

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-app/src/toast.rs` | `save_error`(83): `EditorError::ReadOnly` 를 `AppError::localized(Forbidden, "editor.readOnlySaveBlocked", …)` 로 바꾸는 유일한 지점. `save_failed`(94): 명시 저장은 매번 오류 toast, 자동 저장은 `ReadOnly` 면 표시 없음·그 밖의 실패는 같은 toast 가 살아있는 동안 한 번 |
| `native/taide-native-app/src/application.rs` | `report_save_failure`(1417)가 `EditorError` 를 받아 `save_failed` 로 넘깁니다. 저장 준비 실패(1672), 키맵 저장 요청 실패(1993), untitled 저장 스냅샷 실패(2352), 자동 저장 스냅샷 실패(2696), 포맷 재개 뒤 스냅샷 실패(3309)가 모두 이 메서드를 거칩니다. 닫기 확인의 저장(2200-2202)은 `save_snapshot(id).map_err(crate::toast::save_error)` 로 같은 변환을 씁니다 |

닫기 확인 경로에 있던 `snapshot.metadata.read_only` 사전 검사는 `save_snapshot` 이 같은 조건에서 `ReadOnly` 를 돌려주므로 변환 지점을 하나로 모으면서 제거했습니다. 문서가 없을 때는 이전과 같이 `NotFound` 가 `native editor: NotFound` 로 보고됩니다(`store.rs:115-121`, `1312-1317`).

테스트:

- `toast-tests.rs:241` `읽기전용_파일의_명시_저장_요청은_로컬라이즈된_차단_문구_toast를_요청마다_발행한다`: 합성 읽기 전용 파일 문서를 `EditorStore` 에 열고 `keymap_request` 가 돌려준 오류를 `save_failed` 에 넘겨, 요청 2회에 `Not saved: this file is read-only` 오류 toast 2개가 생기는지 확인합니다.
- `toast-tests.rs:300` `읽기전용_자동_저장_실패는_toast를_만들지_않고_그밖의_자동_저장_실패는_한번만_발행한다`: 자동 저장의 `ReadOnly` 는 toast 0개, 그 밖의 자동 저장 실패는 2회 보고에 1개, 명시 저장 실패는 보고마다 1개인지 확인합니다.

### 11.3 대체되는 기록

| 위치 | 이전 기록 | 현재 |
| --- | --- | --- |
| 1절 "수정하지 않은 것" | `host.rs` 변경 불필요 | 11.1 에서 수정 |
| 3.1 의 1343 | 복사 실패는 모두 `toast.error(common.copyFailed)` | 탐색기 복사만 toast(1346), 터미널 복사는 로그(1353). 3.2 의 LOG 분류에 해당 |
| 3.1 의 1652, 3300, 2687, 1973, 2343 | `editor_error` 문구로 T-ERR 또는 ONCE | `save_failed` 정책. 읽기 전용은 명시 저장만 `editor.readOnlySaveBlocked` |
| 3.1 의 2228 | 닫기 확인 경로에서만 `editor.readOnlySaveBlocked` | 모든 명시 저장 경로가 `save_error` 한 곳을 사용 |

### 11.4 실행한 검증

| 순서 | 명령 | exit | 결과 |
| --- | --- | --- | --- |
| 1 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib toast --locked --offline --target-dir experiments/native-shell-spike/target` (구현 전, 신규 테스트 3건만 추가한 상태) | 101 | 컴파일 오류 13건. 전부 `HostCommand::CopyTerminalSelection`·`HostReply::TerminalSelectionCopied`·`CopyOrigin`·`copy_finished`·`save_failed`·`save_error` 가 없다는 오류입니다 |
| 2 | 같은 명령에 `--quiet` (구현 후) | 0 | 9 passed, 0 failed, 338 filtered out, 0.03s (기존 6 + 신규 3) |
| 3 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 1 → 0 | `toast-tests.rs` 1곳 차이 수정 뒤 통과 |
| 4 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir … --quiet` | 0 | 347 passed, 0 failed, 13.87s |
| 5 | `cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir … --quiet` | 0 | 앱 crate 경고 없음 |
| 6 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test terminal-host terminal_menu_actions --locked --offline --target-dir … --quiet` | 0 | 2 passed, 0 failed, 50 filtered out, 0.96s. 합성 fixture 바이너리(`native-terminal-queue-fixture`)를 셸로 쓰는 테스트이며 `assert!(copied)` 로 새 명령을 확인합니다 |

- 순서 1 은 런타임 단언 실패가 아니라 컴파일 실패입니다. 수정 전 동작은 위 "확인한 사실"의 코드 경로로 확인했고, 구현을 되돌려 런타임 실패를 다시 만드는 편집은 하지 않았습니다.
- 순서 4 의 347건 가운데 toast 필터 밖 338건은 8절 기록(337건)보다 1건 많습니다. 이 단계가 추가한 테스트는 toast 필터 안의 3건뿐입니다.
- `taide-native-ui`, `taide-native-editor`, taide-locale 카탈로그는 이 단계에서 수정하지 않았습니다. `taide-remote-web` 은 그 두 crate 에만 의존하므로(`native/taide-remote-web/Cargo.toml:28-29`) V4, V5, `taide-remote-web` check, `taide-native-ui` 포맷 검사는 다시 실행하지 않았습니다.
- 경로는 모두 `/Users/hyunseokbyun/development/TAIDE` 기준 절대 경로로 실행했습니다.

### 11.5 남은 위험

- [ ] 실제 화면에서 읽기 전용 파일 탭의 Cmd+S 가 로컬라이즈 문구 toast 1개를 띄우는지 확인.
- [ ] 실제 화면에서 터미널 메뉴 복사가 정상 동작하는지 확인(클립보드 실패 조건은 실기에서 만들기 어려워 합성 테스트로만 확인했습니다).
- `tests/explorer-system.rs` 는 `HostCommand::CopyText`·`HostReply::CopiedText` 를 그대로 쓰며 이 단계에서 계약이 바뀌지 않아 실행하지 않았습니다.
