# lane commands-ui 사양 (Rust-native 전환, wave 3 + wave 1 선행 묶음)

> 작성일: 2026-10-28 · 대상 lane: `commands-ui` · 운영 기준: `docs/memory/2026-10-28-native-lane-operations.md`
> 이 문서만 읽고 구현을 시작할 수 있도록 원본 TS 동작, native 현재 자산, 설계, 검사, 완료 기준을 ID마다 적었습니다. 원본 TS를 다시 조사할 필요는 없고, 구현 중에는 여기 적힌 native 심볼 주변만 읽으면 됩니다.

## 0. 읽는 법과 전제

- 경로 약어: `TS` = `src/`, `UI` = `native/taide-native-ui/src/`, `APP` = `native/taide-native-app/src/`, `CR` = `crates/`, `NM` = `node_modules/`. lane worktree(`/Users/hyunseokbyun/development/TAIDE-lanes/commands-ui`)에서도 같습니다.
- wave 0 구조 이동이 진행 중입니다. 이 문서를 쓸 때 이미 `APP/domains/commands.rs`(`Command`, `Reply`, `State`, `dispatch`, `NativeApplication::{apply_commands_reply, apply_commands_intent, tick_commands_files, show_commands_overlays, show_palette}`), `APP/domains/shell.rs`, `UI/intents/commands.rs`(`Intent::{OpenPalette, OpenKeybindings, OpenTaskRunner}`)가 생겼고 `HostCommand::{Shell, Commands}`·`HostReply::{Shell, Commands}`가 영역 하위 enum을 품습니다. 반면 `APP/shell_keymap.rs`는 아직 옛 `ShellIntent::OpenPalette` 등을 가리킵니다(이동 작업 중). 그래서 `APP/application.rs`, `APP/host.rs`, `UI/commands.rs`, `UI/shell.rs`, `APP/shell_keymap.rs`는 줄 번호 없이 심볼 이름으로만 적었습니다. `docs/research/2026-10-28-native-lane-seams.md`는 아직 없습니다. 생기면 아래 이름을 그 문서의 실제 이름으로 바꿔 읽습니다.
- 원본 줄 번호는 이 문서를 쓰며 직접 확인했습니다. QA 근거는 파일명만 적었습니다(모두 `docs/quality-assurance/` 아래).
- 기준: 원본 TS 화면·상태·상호작용과 같게 만들고, 새 기능·디자인은 넣지 않으며, 원본 결함은 재현하지 않습니다. complete = 구현 + 실제 앱 경로 + 해당 동작의 자동 검사입니다. 픽셀·실제 OS 입력·실기(배치 31)는 포함하지 않습니다.
- 대응표의 commands-ui 비고 대부분은 2026-10-06 감사 때의 공통 문구라 실제 남은 범위가 적혀 있지 않습니다(예: 101의 "67종"은 실제 56종, 13·35·40·41은 이미 동작). 2절에 실제 상태를 적었습니다.

## 1. 요약

### 1.1 수와 상태

- 대상 51개. 대응표 현재 상태: partial 25, unwired 22, missing 4(1, 54, 82, 83).
- 분류(2절 근거)
    - 이미 동작, 근거 갱신만 필요: 10개 — 13, 20, 35, 40, 41, 52, 54, 90, 91, 101
    - 지금 바로 연결 가능(기능 이미 있음): 19개 — 1, 15, 21, 22, 23, 43(실행 안 되는 25개 중 구현 대상), 67, 68, 69, 71, 72, 73, 74, 75, 77, 79, 86, 96, 98(일부)
    - 다른 lane 기능 완료 후 연결: 22개 — 11, 12, 14, 30, 31, 32, 33, 34, 36, 37, 38, 39, 59, 61, 62, 70, 76, 82, 83, 84, 85, 87
    - 공용 위젯 행(67~79, 84)은 "이미 있는 native 소비 화면"은 바로, 다른 lane 소비 화면은 의존이라 두 분류에 걸칩니다. 위 수는 1.2 표의 주 분류 기준입니다.
- 이 lane이 끝낼 수 있는 ID: 50개. 82(편집기·루트 장애 경계)는 wave 4 메인 작업(배치 31 "편집기 장애 경계 설계")이 끝나야 complete가 됩니다.

### 1.2 ID별 분류

분류: 근거 = 이미 동작·근거 갱신만, 바로 = 지금 바로 연결 가능, 의존 = 다른 lane 완료 후. 묶음은 3절.

| ID | 기능 | 표 상태 | 분류 | 이 lane이 할 일 | 먼저 필요한 것 | 묶음 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | window.reload | missing | 바로 | 창 재적재 명령(4절 결정 1) | 없음 | 7 |
| 11 | search.find | unwired | 의존 | 실행 확인·행 갱신 | explorer-search-37 | 3 |
| 12 | search.replace | unwired | 의존 | 같음 | explorer-search-37 | 3 |
| 13 | view.explorer | unwired | 근거 | 팔레트 App 검사 1건, 행 갱신 | 없음(shell-21에서 구현됨) | W1-A |
| 14 | view.git | unwired | 의존 | 실행 확인·행 갱신 | git-1 | 3 |
| 15 | view.welcome | unwired | 바로 | `Run` 추가 → 기존 `ShellIntent::OpenWelcome` | 없음 | W1-A |
| 20 | terminal.copyImeDebug | unwired | 근거 | 비활성 동등 검사, 행 갱신 | 없음 | W1-B |
| 21 | app.showPerfSnapshot | unwired | 바로 | 4절 결정 3에 따라 not-applicable 또는 로그 출력 | (B안이면 settings S1의 `perf::init`) | W1-B |
| 22 | tab.moveToNewWindow | unwired | 바로 | `Run` 추가 → 기존 `ShellIntent::MoveTabToWindow` | 없음(shell-7 경로) | W1-A |
| 23 | tab.moveToMainWindow | unwired | 바로 | 같음, 보조 창에서만 활성 | 없음 | W1-A |
| 30 | sync.uploadNow | unwired | 의존 | 명령 실행·토스트 | settings S4 호스트 동작 | 4 |
| 31 | sync.downloadNow | unwired | 의존 | 명령 실행·충돌 토스트+"Pull Remote" 동작 | settings S4 | 4 |
| 32 | cli.connectExternalEditor | unwired | 의존 | 상태→설치 조합·토스트 | settings S1 CLI 포트 | 4 |
| 33 | cli.installShellCommand | unwired | 의존 | 설치·토스트 | settings S1 | 4 |
| 34 | cli.uninstallShellCommand | unwired | 의존 | 해제·토스트 | settings S1 | 4 |
| 35 | ai.inlineEdit | unwired | 근거 | 행 갱신(App 검사 있음) | 없음 | W1-A |
| 36 | git.revertHead | unwired | 의존 | 실행 확인·행 갱신 | git-36 | 3 |
| 37 | git.createTagOnHead | unwired | 의존 | 같음 | git-36, git-1 | 3 |
| 38 | git.toggleBlame | unwired | 의존 | 같음 | git-57 | 3 |
| 39 | git.openFileHistory | unwired | 의존 | 같음 | git-55 | 3 |
| 40 | terminal.runSelectedText | unwired | 근거 | 행 갱신, 활성 조건 차이(4절 결정 2) | 없음 | W1-A |
| 41 | task.runTask | unwired | 근거 | 행 갱신(shell-24 근거) | 없음 | W1-A |
| 43 | monaco.* 액션 실행 | partial | 바로(일부) | 실행 안 되는 25개 처리(2.3의 commands-ui-43 표) | 3개는 git lane, 5종은 결정 5 | 8 |
| 52 | cmdk command-score | partial | 근거 | UI crate 단위 검사, 행 갱신 | 없음 | W1-B |
| 54 | 최근 항목(MRU) | missing | 근거 | not-applicable 재분류(원본 미구현) | 없음 | W1-B |
| 59 | 키맵 디스패치 대상 | partial | 의존 | 41개 전부 소비 검사로 닫기 | explorer-search-37, git-1 | 3 |
| 61 | find/search/search-replace 키 | partial | 의존 | 같음 | explorer-search-37 | 3 |
| 62 | explorer/git 키 | unwired | 의존 | 같음 | git-1 | 3 |
| 67 | alert-dialog | partial | 바로 | 공용 alert dialog + 기존 소비 화면 이전 | 다른 lane 소비 화면은 각 lane | 6 |
| 68 | button | partial | 바로 | 공용 button(4 variant × 4 size) + 이전 | 없음 | 5 |
| 69 | card | partial | 바로 | 공용 card + Welcome·Settings 이전 | 없음 | 5 |
| 70 | checkbox | partial | 의존 | 공용 checkbox | 소비 화면 2곳이 explorer-search·settings | 5 |
| 71 | command(cmdk 목록) | partial | 바로 | 공용 목록 규칙 + 팔레트·Picker 대조 | branch switcher는 git lane | 6 |
| 72 | context-menu | partial | 바로 | 공용 메뉴(항목·파괴·단축키·하위 메뉴·구분선·typeahead) + 이전 | git 메뉴는 git lane | 6 |
| 73 | dialog | partial | 바로 | 공용 dialog(닫기 X·바깥 클릭·IME Escape) + 이전 | git·settings 대화상자는 각 lane | 6 |
| 74 | dropdown-menu | partial | 바로 | 72와 같은 메뉴 모듈(Label 추가) + 이전 | 검색 이력은 explorer-search | 6 |
| 75 | popover | partial | 바로 | 공용 popover 틀 + Picker·색 선택기 이전 | branch switcher는 git lane | 6 |
| 76 | progress | partial | 의존 | 공용 progress | settings S2(LSP 설치 진행) | 5 |
| 77 | scroll-area/overlay scrollbar | partial | 바로 | 공용 overlay scrollbar + 이전 | 없음 | 5 |
| 79 | switch | partial | 바로 | 공용 switch + 이전 | settings 새 스위치는 settings lane | 5 |
| 82 | error-boundary | missing | 의존 | 근거 갱신(shell-12와 같음) | wave 4 편집기·루트 경계 | 7 |
| 83 | status-error-banner | missing | 의존 | 근거 갱신 | shell-94 | 7 |
| 84 | file-group-header | partial | 의존 | 공용 header + 문제 패널 이전 | explorer-search 검색 결과 | 5 |
| 85 | toast | partial | 의존 | 4종·설명·동작 근거, 동기화 충돌 동작 소비자 | 묶음 4(settings S4) | 4 |
| 86 | 테마 토큰 → 전역 스타일 | partial | 바로 | 공용 토큰 표, 본문 13px(결정 7) | 없음 | 7 |
| 87 | global.css 보조 | partial | 의존 | scrim·그림자·토스트 위치 근거, gutter·blame은 git | git-56~63 | 7 |
| 90 | use-reveal-window | partial | 근거 | 첫 프레임 표시 근거 검사 | 없음 | W1-B |
| 91 | use-tauri-event 등 hook | partial | 근거 | not-applicable 재분류(4절 결정 4) | 없음 | W1-B |
| 96 | 비국제화 문자열 | partial | 바로 | 문자열 감사 검사 + 결정 반영 | settings-78 결정 3 | 7 |
| 98 | lucide 아이콘 | partial | 바로(일부) | 아이콘 목록표 대조·누락 보충 | git·explorer-search 화면 | 7 |
| 101 | 프로젝트 아이콘 카탈로그 | partial | 근거 | 원본 목록 drift 검사, 행 갱신(56종) | 없음 | W1-B |

### 1.3 다른 lane 의존과 순서

- 먼저 필요한 것: wave 0 구조 이동. 이 lane의 새 코드는 `APP/domains/commands.rs`, `UI/intents/commands.rs`, 새 공용 UI 모듈, 새 검사 파일에 넣습니다. 공용 파일(`APP/application.rs`, `APP/lib.rs`·`UI/lib.rs` 모듈 목록, `HostCommand`/`ShellIntent` 루트, locale 카탈로그)은 영역 블록 끝에 추가만 합니다.
- wave 1 lane과 겹치는 파일(W1 묶음에만 해당)
    - explorer-search lane 묶음 C가 `UI/command-registry.rs`(Run·검사), `APP/command-dispatch.rs`(find 대체), `APP/shell_keymap.rs`를 고칩니다. W1 묶음은 같은 파일의 `Run` enum 끝, `execution()` match 끝, `shell_keymap::intent` match 끝, 검사 목록 끝에만 추가합니다. 병합 충돌은 메인이 풉니다.
    - shell lane은 `ShellIntent::MoveTabToWindow`·`OpenWelcome` 처리(`APP/domains/shell.rs`)를 바꾸지 않습니다(shell spec 2절). W1-A는 그 처리를 읽기만 합니다.
- wave 3에서 이을 것(이 lane의 묶음 3~8)
    - explorer-search(wave 1): 검색 패널과 ⌘⇧F/⌘⇧H·`search.find`/`search.replace`(explorer-search-37). 그 lane 사양 결정 D8이 A면 commands-ui-11·12 행은 그 lane이 갱신하고, 이 lane은 묶음 3에서 59·61의 전체 검사만 합니다.
    - git(wave 2): Git 보기·⌃⇧G·`view.git`(git-1), `git.revertHead`/`git.createTagOnHead`(git-36), 파일 이력(git-55), blame(git-57), gutter·충돌 장식(git-56~63, 87의 CSS 클래스 대응). git spec 1.2: "commands-ui-14·36~39·62는 이 lane이 실행 경로를 만든 뒤 그 lane(commands-ui)이 행만 닫습니다".
    - settings(wave 2): 동기화 업로드·다운로드 호스트 동작(S4), CLI 상태·설치·해제 포트(S1), LSP 설치 진행(S2, 76의 소비자), 공용 대화상자 `UI/settings-dialogs.rs`(S1), status 라벨 결정 3(96과 연결). settings spec 2K settings-67: "실제 팔레트·키 연결은 commands-ui-30~34가 이 호스트 동작을 부릅니다".
    - shell(wave 1): shell-94(시작 시 로드 실패 대체, 배너 순서)이 83의 근거입니다.
    - wave 4 메인: 편집기 영역·루트 장애 경계(82).
- 운영 문서 3절은 wave 3 commands-ui lane에 "앞 wave에서 진입만 남은 행"과 shell-89도 줍니다. 그 행들(shell spec 묶음 3: shell-21·59·78·89의 남은 부분 등)은 이 문서의 대상 ID 밖입니다. 4.1 결정 9를 봅니다.

### 1.4 wave 1에 넣을 수 있는 선행 묶음

다른 lane 기능 없이 지금 끝낼 수 있고, wave 1 lane의 소유 파일(편집기 표면, 탐색기, 셸 화면, 터미널)을 고치지 않는 것만 모았습니다. 공용 위젯 행(67~79)은 기술적으로는 지금도 가능하지만 소비 화면 파일이 wave 1·2 lane 소유라 wave 3로 둡니다.

- W1-A 명령 진입(7개): 13, 15, 22, 23, 35, 40, 41
- W1-B 근거·재분류(7개): 20, 21, 52, 54, 90, 91, 101
- 고치는 파일: `UI/command-registry.rs`, `APP/shell_keymap.rs`, `APP/command-dispatch.rs`(검사 목록), `UI/command-score.rs`(검사 모듈), `UI/glyph-icons-tests.rs`, 새 `APP/application-commands-tests.rs`(`domains/commands.rs`의 `#[cfg(test)] #[path]`), 대응표·QA 문서. 새 의존성 없음.

## 2. ID별 사양

### 2.0 공통 설계

#### 2.0.1 명령 실행 경로(현재)

- 등록: `UI/keybinding-commands.json`(212개, `monaco.*` 157개) → `UI/command-registry.rs` `Registry::load` → `Command { id, title_key, category_key, keymap_id, is_mac_only, execution, enablement }`. `execution(id, keymap_id)`가 `Execution::Native(Run)` 또는 `Unavailable`, `enablement(id)`가 원본 `isEnabled`를 옮긴 `Enablement`(`Always`, `Never`, `WebviewDiagnostics`, `MainWindow`, `AuxiliaryWindow`, `ActiveProject`, `ActiveEditor`, `SupportedEditorAction`)입니다. `Command::runnable(context)`이 둘을 합칩니다(원본 `isCommandRunnable`, `TS/shared/lib/command-registry.ts:89`).
- 팔레트: `UI/command-palette.rs`가 `Action::RunCommand(id)`를 내고, `APP/domains/commands.rs` `show_palette`가 편집기 전용 몇 개(ai.inlineEdit, 문맥 메뉴, 스마트 선택, code lens, code action, rename, 포맷)를 직접 처리한 뒤 나머지를 `commands_state.palette_commands`에 넣습니다.
- 키: `taide_native_ui::keymap`이 `Decision::Dispatch(id)`를 내면(사용자 override의 명령 전용 행 포함, commands-ui-63) `keymap_actions`에 쌓입니다. 앱 프레임에서 둘을 합쳐 `APP/command-dispatch.rs` `intent_with_editor(id, context, snapshot, focused_view, scope)` → `intent` → `shell_keymap::intent(run, snapshot, scope)`가 `ShellIntent`를 만듭니다.
- 원본의 `runCommandSafely`(`TS/shared/lib/command-registry.ts:106-114`): `run`이 던지거나 reject하면 `toast.error(describeIpcError(error))`. native는 호스트 응답 `Err`를 `self.toasts.ipc_error(..)`로 보입니다(`APP/domains/commands.rs` `apply_commands_reply`의 선례).

#### 2.0.2 새 명령을 붙이는 규칙

1. `UI/command-registry.rs`: `Run`에 변형을 추가하고 `#[cfg(feature = "native-host")]`로 막습니다. 동결된 `taide-remote-web`은 `taide-native-ui`를 `default-features = false`로 쓰므로, 막아 두면 그쪽 컴파일 표면이 바뀌지 않습니다(`OpenTaskRunner` 선례). `execution()`의 마지막 `let run = match id { .. }` 블록 끝에 `"<id>" => Some(Run::..)`을 더합니다. 활성 조건이 원본과 다르면 `enablement()`를 고칩니다.
2. `APP/shell_keymap.rs` `intent`: 프로젝트가 필요 없는 실행은 첫 match에, 포커스 pane이 필요한 실행은 `(root, focused)` 뒤에 둡니다. 결과는 기존 `ShellIntent` 또는 `ShellIntent::Commands(UI/intents/commands.rs Intent::..)`입니다.
3. 호스트가 필요하면 `APP/domains/commands.rs`의 `Command`/`Reply`에 변형을 더하고 `dispatch`·`apply_commands_reply`·`apply_commands_intent`에 처리를 둡니다.
4. 검사 목록 갱신: `UI/command-registry.rs` 검사의 `NATIVE_COMMANDS`(실행 경로 표), `APP/command-dispatch.rs` 검사의 "unavailable" 목록.

#### 2.0.3 공용 UI 토큰(67~79, 84, 86 공통)

원본 shadcn 토큰은 `TS/shared/styles/global.css:138-155`의 별칭입니다. native 테마 키로 옮기면 아래와 같습니다(모두 `CR/taide-theme`의 필수 키이고 `UI/snippet-editor.rs` `Appearance::new`가 이미 대부분을 씁니다).

| 원본 토큰 | native 테마 키 |
| --- | --- |
| background / foreground | `app.background` / `app.foreground` |
| card / card-foreground | `panel.background` / `app.foreground` |
| popover / popover-foreground | `popover.background` / `app.foreground` |
| primary / primary-foreground | `button.primaryBackground` / `button.primaryForeground` |
| secondary / secondary-foreground | `button.background` / `button.foreground`(원본 소비자 없음) |
| muted / muted-foreground | `appSidebar.itemHover` / `appSidebar.iconDefault` |
| accent / accent-foreground | `list.hoverBackground` / `list.foreground` |
| destructive | `statusIndicator.error` |
| border / input / ring | `app.border` / `input.border` / `app.focusBorder` |
| 메뉴 | `menu.background`, `menu.border`, `menu.itemHover`, `menu.separator` |
| popover 테두리·구분선 | `popover.border`, `popover.separator` |
| modal | `modal.background`, `modal.border`, `modal.itemHover`, `modal.separator` |
| 선택 행 | `list.activeBackground` + `app.accent` 링 |
| scrollbar | `scrollbar.thumb`, `scrollbar.thumbHover`, `scrollbar.track` |
| radius | sm 4px, md 6px, lg 8px(`--radius` 6px) |
| 그림자 | `shadow-overlay` 0 2px 8px, `shadow-overlay-lg` 0 8px 24px, 색 `app.shadow`. scrim = `app.shadow` 50%(global.css 349-359) |

- 새 모듈 `UI/ui-tokens.rs`(`crate::ui_tokens`)에 `Tokens::new(&ResolvedTheme) -> AppResult<Tokens>`를 두고, 앱은 테마가 바뀔 때마다 `APP/presentation-refresh.rs`가 만드는 표시 묶음에 함께 만들어 egui context data에 넣습니다(`Tokens::store(ctx)`, `Tokens::get(ctx)`). 프리미티브는 이 값만 읽습니다. 기존 `Appearance`류는 이 값에서 만들도록 바꿀 수 있지만, 바꾸는 범위는 이전하는 소비 화면으로 한정합니다.

#### 2.0.4 공용 위젯 행의 완료 기준(4절 결정 6 추천안 A)

- (a) 원본 소비 화면이 실제로 쓰는 variant·state 조합을 native 공용 모듈 하나가 모두 그립니다. 원본에 소비자가 없는 variant(button `secondary`·`link`, size `lg`·`icon`·`icon-sm`·`icon-lg`, 메뉴 checkbox·radio 항목, `CommandDialog`, `CommandSeparator`)는 대상이 아닙니다.
- (b) native에 이미 있는 소비 화면은 모두 그 모듈을 씁니다(화면별 복제 구현 제거).
- (c) 다른 lane 소유 소비 화면은 그 lane이 병합된 상태에서 그 모듈을 쓰는지만 확인합니다. 아직 화면이 없으면 행 비고에 남깁니다.
- 검사: 공용 모듈 단위 검사(egui context에서 그려진 shape의 색·크기, accesskit role·toggled·disabled), 이전한 소비 화면의 기존 App·UI 검사 통과 + 소비 화면마다 대표 단언 1개.

#### 2.0.5 검사 공통

- 모든 명령은 `CARGO_TARGET_DIR=<worktree>/target-lane`로 실행합니다.
    - UI lib: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib -- <필터>`
    - UI 하네스: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test workbench -- <필터>`
    - 앱 lib: `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- <필터> --test-threads=4`
    - crate 단위: `cargo test --manifest-path crates/<crate>/Cargo.toml -- <필터>`
    - 동결 remote-web(UI crate를 바꾼 묶음만): `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline`
    - 형식: 바꾼 crate마다 `cargo fmt --manifest-path <crate>/Cargo.toml -- --check`
- 실제 App 검사는 `APP/application-project-rail-tests.rs`의 `Fixture`·`isolated_state`·`launch`, 팔레트 실행은 `APP/application-selection-ranges-tests.rs`의 `run_palette`(application-ai-edit-tests.rs 104가 쓰는 helper), 보조 창은 `APP/application-auxiliary-window-tests.rs`의 viewport renderer를 씁니다.
- 새 App 검사는 `APP/application-commands-tests.rs` 하나에 모으고 `APP/domains/commands.rs` 끝의 `#[cfg(test)] #[path = "../application-commands-tests.rs"] mod tests;`로 연결합니다(공용 `application.rs` 검사 목록을 늘리지 않음). 새 test target은 만들지 않습니다(디스크 비용).
- 금지: 실제 클립보드·Keychain·네트워크·`/usr/local/bin` 쓰기·OS 알림·GUI 실행·합성 OS 입력. 모두 기존 주입 포트(`ClipboardWriter`, settings `Ports`, 가짜 gist)를 씁니다.
- 검사 이름은 기존처럼 한국어로 씁니다.

---

### 2.1 명령 카탈로그 진입

#### commands-ui-1 window.reload (missing, 바로, 묶음 7)

- 원본: `TS/shared/lib/command-catalog.ts:20` `{ id: 'window.reload', titleKey: 'app.reloadWindow', categoryKey: WINDOW, run: () => window.location.reload() }`. 활성 조건 없음(메인·보조 창 모두 활성), 키 없음. 실행한 창의 webview만 다시 읽습니다. Rust 상태(레이아웃, pty, LSP 세션, hot-exit mirror)는 유지되고 화면이 백엔드에서 다시 조회됩니다(`docs/features/terminal.md` 18·51, `lsp.md` 254, `tabs.md` 251). 대기 중 확인 대화상자·팔레트·메뉴·toast 같은 일시 UI는 사라지고, 장애 경계 fallback도 새로 mount됩니다. wait marker는 `sessionStorage`라 남습니다(`TS/entities/agent/agent-wait-marker-registry.ts:70-85`).
- native 현재: 카탈로그 행만 있고 `Execution::Unavailable`이라 팔레트에서 비활성 행입니다. 검사 `APP/command-dispatch.rs` "unavailable" 목록에 `window.reload`가 있습니다.
- 빠진 것: 실행 정의. native는 UI와 Rust 상태가 한 프로세스라 "webview 다시 읽기"에 대응하는 동작을 정해야 합니다(4절 결정 1).
- 설계(결정 1 추천안 A)
    - `Run::ReloadWindow`(native-host), 활성 `Always`. `shell_keymap::intent` 첫 match에서 `ShellIntent::Commands(Intent::ReloadWindow)`.
    - `apply_commands_intent(Intent::ReloadWindow, context)`가 `context.viewport_id()` 창에 대해 다음을 순서대로 합니다.
        1. 일시 UI 닫기: 팔레트 `commands_state.palette.dismiss`, 키바인딩 편집기 닫기, 열린 egui popup 전부 닫기, tooltip provider 초기화. 대기 중 확인(`pending_tab_close`, `pending_entry_delete`, `pending_disk_choice`)은 "취소"로 처리합니다(원본은 대화상자가 사라지고 탭·항목이 그대로 남음).
        2. 장애 경계 초기화: `UI/error-boundary.rs`에 `pub fn reset_viewport(ctx: &egui::Context, viewport: ViewportId)`를 추가해 그 viewport의 `STATE_KEY` 상태를 모두 지웁니다(다음 pass에 새 generation으로 그림).
        3. 표시 재적재: 테마·로케일·설정을 다시 읽습니다(`APP/presentation-refresh.rs` `Refresh::retry`와 같은 경로를 강제). 실패하면 shell-94 배너가 뜹니다.
        4. 데이터 다시 조회: 팔레트 파일 색인 전부 무효화(`commands_state.palette_files.invalidate`), 그 창 프로젝트들의 explorer tree 재조회, Settings 카탈로그·자원(`Views::invalidate_catalog`, `refresh_resources`), Welcome 최근 목록, 제목 branch(`title_branch::invalidate`), 미리보기 cache(`invalidate_all` 계열).
        5. 유지: 편집 문서·undo·view 상태, 터미널 세션과 scrollback, LSP 세션, 레이아웃, 창 위치·크기.
- 검사(App): 문서를 dirty로 만들고 팔레트 열기 + 상태 표시줄 panic 주입(`error_boundary::inject_panic`) → 팔레트로 `window.reload` 실행 → 팔레트 닫힘, fallback이 사라지고 정상 상태 표시줄, presentation 재적재 요청 1회, 다음 팔레트 열기에서 파일 색인 재요청, 문서 dirty·본문 유지, 터미널 세션 id 유지. 보조 창에서 실행하면 메인 창 팔레트·fallback은 그대로.
- 완료 판정: 위 검사 통과. 결정 1이 B면 행을 not-applicable로 바꾸고 비고에 "웹 view 다시 읽기 전용, native에는 대응 대상 없음"을 적습니다.

#### commands-ui-13 view.explorer (unwired, 근거, W1-A)

- 원본: `command-catalog.ts:87-93` `keymapId: 'explorer'`(⌘⇧E), 실행 `requestShowExplorerView('files')`, 활성 조건 없음.
- native 현재: `Run::ShowExplorer`(`keymap_run("explorer")`) → `shell_keymap::intent` → `ShellIntent::ShowExplorer{project, slot}`/`ShowAuxiliaryExplorer`. UI 검사 `실행경로는_native동작이_있는_명령과_keymap액션만_typed요청으로_구분한다`가 `view.explorer` runnable을 단언합니다. 키 경로 App 검사는 shell-21 근거(2026-10-28-native-shell-keymap-dialogs-slot-header.md)입니다.
- 빠진 것: 대응표 갱신. 팔레트 경로 App 검사가 없습니다.
- 설계: 코드 변경 없음.
- 검사(App): 메인 2 slot, focused slot의 sidebar를 Outline 보기로 둔 뒤 팔레트 `view.explorer` → 그 slot만 Files 보기·접힘 해제.
- 완료 판정: complete.

#### commands-ui-15 view.welcome (unwired, 바로, W1-A)

- 원본
    - `command-catalog.ts:95` `run: (context) => context.openWelcomeTab()`, 활성 조건 없음, 키 없음.
    - `TS/widgets/command-palette/command-palette.tsx:202-214` `openWelcomeTab`: 프로젝트가 없으면 `toast.info(t('app.openProjectFirst'))`. 있으면 `openTab({ kind: welcome, title: WELCOME_TAB_TITLE, target: currentWindowFocusedPane(layout), preview: false })`, 실패는 `toast.error(describeIpcError)`. 제목은 번역하지 않은 상수입니다(196-201 주석). 이미 그 pane에 Welcome 탭이 있으면 runtime이 새로 만들지 않고 활성화합니다(`CR/taide-layout/src/service.rs:397-415` `open_tab`의 `is_dedupable`).
- native 현재: 탭 바 메뉴의 `tab.openWelcome`이 `ShellIntent::OpenWelcome{project, pane}`(UI `shell.rs` `tab_bar_menu`) → `APP/domains/shell.rs` `Command::OpenWelcome` → `layout_actions::layout_open_tab(.., TabKind::Welcome, WELCOME_TAB_TITLE, Some(pane), false)`, 실패는 `HostReply::Failed`. 프로젝트 없음 알림은 `ShellIntent::ShowOpenProjectNotice` → `notify_open_project_first`(info toast `app.openProjectFirst`).
- 빠진 것: 명령의 실행 경로.
- 설계: `Run::OpenWelcomeTab`(native-host), 활성 `Always`, `"view.welcome" => Some(Run::OpenWelcomeTab)`. `shell_keymap::intent`: 첫 match에서 `project`가 없으면 `ShellIntent::ShowOpenProjectNotice`. `(root, focused)` 뒤에서 `ShellIntent::OpenWelcome{ project, pane: focused.clone() }`. 보조 창은 `window_tree`가 그 창 트리를 주므로 그 창 focused pane입니다(원본 `currentWindowFocusedPane`).
- 검사
    - UI: `NATIVE_COMMANDS`에 `("view.welcome", Run::OpenWelcomeTab)` 추가.
    - 앱 단위(`shell_keymap` 검사): 프로젝트 없음 → `ShowOpenProjectNotice`, 분할 레이아웃 → focused pane 지정, 보조 창 → 그 창 pane.
    - App: 팔레트 `view.welcome` → focused pane에 `TabKind::Welcome`·제목 "Welcome" 활성. 다시 실행 → 탭 수 그대로. 프로젝트 0개 → info toast `app.openProjectFirst`.
- 완료 판정: complete.

#### commands-ui-20 terminal.copyImeDebug (unwired, 근거, W1-B)

- 원본: `command-catalog.ts:131-139` `isEnabled: isImeDebugEnabled`. `TS/shared/lib/ime-debug.ts:18-30`의 플래그는 `setImeDebugEnabled`로만 켜지는데 비검사 코드의 호출이 0건입니다(`rg "setImeDebugEnabled\(" src` 결과 없음). 그래서 원본에서도 항상 비활성 행입니다.
- native 현재: `Enablement::WebviewDiagnostics`(항상 거짓). UI 검사 `enabled_판정은_원본_조건과_실행경로_유무를_함께_적용한다`가 단언합니다.
- 설계: 코드 변경 없음. 대응표에 "원본과 같은 비활성 행, 원본에도 켜는 경로 없음"을 적습니다.
- 검사: 기존 검사 + 팔레트 UI 검사에서 행이 비활성으로 보이는지 1건(`UI/command-palette-tests.rs`).
- 완료 판정: complete.

#### commands-ui-21 app.showPerfSnapshot (unwired, 바로, W1-B)

- 원본: `command-catalog.ts:140-158`. `titleDefaultValue: 'Show Performance Snapshot'`, `isEnabled: isPerfEnabled`(`TS/shared/lib/perf-mark.ts:74-87`: `TAIDE_PERF` 값이 있으면 그것, 없으면 개발 빌드면 켜짐). 실행은 webview 개발자 console에 프런트엔드 계측표만 출력합니다(`perf-mark.ts:219-224`, 주석 143-153: Rust 쪽 snapshot은 출력하지 않음).
- native 현재: `Enablement::WebviewDiagnostics`. native에는 별도 프런트엔드 계측 레지스트리가 없고 Rust `taide_infra::perf::global()`만 있습니다(`CR/taide-infra/src/perf.rs` `PerfRegistry`). 성능 수치 화면은 settings-54(Performance section)입니다.
- 설계: 4절 결정 3. 추천 A는 not-applicable(개발자 console 출력 전용 계측 도구, commands-ui-55와 같은 성격). B면 `Enablement::PerfGate`(`perf::global().is_enabled()`)를 새로 두고 실행 시 `log::info!`로 span·counter·command 표를 출력합니다(settings S1의 `perf::init`이 먼저 필요).
- 검사: A면 대응표만. B면 앱 단위 검사(주입 레지스트리 활성 → 행 활성, 실행 → 로그 기록 포트에 표 1회).
- 완료 판정: A not-applicable / B complete.

#### commands-ui-22 tab.moveToNewWindow, commands-ui-23 tab.moveToMainWindow (unwired, 바로, W1-A)

- 원본
    - `command-catalog.ts:159-164`: `requestEditorPaneCommand({ type: 'move-focused-tab-to-window', target: { kind: 'newAuxiliary' } })`, 활성 조건 없음(메인·보조 창 모두).
    - `command-catalog.ts:165-179`: target `main`, `isEnabled: getWindowContext().kind === 'auxiliary'`.
    - 받는 쪽 `TS/widgets/editor-area/editor-area.tsx:139-144` `moveFocusedTabToWindow`: focused slot의 editor area만(323-325 `isFocused`), 그 창 pane 트리의 focused leaf에 활성 탭이 없으면 아무것도 하지 않음, 있으면 `moveTabToWindow({ tabId: leaf.active, target })`, 실패는 `toast.error(describeIpcError)`.
- native 현재: 탭 메뉴 경로가 complete(shell-7). `ShellIntent::MoveTabToWindow{tab, target}` → `APP/domains/shell.rs` `apply_shell_intent`(새 창이면 `auxiliary_windows.reserve()` label 예약, 제출 실패 시 해제) → `Command::MoveTabToWindow` → runtime `layout_move_tab_to_window` → `Reply::TabMovedToWindow`(새 창 채택, 실패 toast, 빈 보조 창 정리). `enablement("tab.moveToMainWindow") = AuxiliaryWindow`는 이미 있습니다.
- 빠진 것: 두 명령의 `Run`과 intent.
- 설계
    - `UI/command-registry.rs`: `pub enum WindowMove { NewWindow, MainWindow }`(Copy), `Run::MoveFocusedTab(WindowMove)`(native-host). `"tab.moveToNewWindow" => MoveFocusedTab(NewWindow)`, `"tab.moveToMainWindow" => MoveFocusedTab(MainWindow)`. `Run`이 `Copy`라 `TabWindowTarget`(Clone만)을 직접 담지 않습니다.
    - `shell_keymap::intent`: leaf를 찾은 뒤 `active?`로 `ShellIntent::MoveTabToWindow{ tab: active.clone(), target: NewAuxiliary | Main }`. 활성 탭이 없으면 `None`(원본도 무동작·무알림).
- 검사
    - UI: `NATIVE_COMMANDS`에 두 행, 메인 창 context에서 `tab.moveToMainWindow` 비활성·보조 창에서 활성.
    - App(`application-auxiliary-window-tests.rs` 방식): 메인 창 팔레트 `tab.moveToNewWindow` → 새 보조 창에 탭 이동·창 채택. 그 보조 창 팔레트 `tab.moveToMainWindow` → 메인 focused pane으로 복귀, 빈 보조 창 닫힘. 탭 없는 pane → host 명령 없음.
- 완료 판정: 두 행 complete(OS focus 실기는 배치 31).

### 2.2 다른 영역 명령

#### commands-ui-11 search.find, commands-ui-12 search.replace (unwired, 의존, 묶음 3)

- 원본: `command-catalog.ts:79-86`. `search.find` = `requestOpenSearchPanel()`(seed 없음), `search.replace` = `requestOpenSearchPanel({ openReplace: true })`, 키 `search`(⌘⇧F)·`search-replace`(⌘⇧H). 키 경로는 editor area의 `whenFocused`(`editor-area.tsx:293-298`): ⌘⇧F는 텍스트 focus 편집기의 비지 않은 한 줄 선택을 seed로 넣습니다(163-168). 프로젝트가 없으면 editor area가 없어 키를 소비하지 않습니다.
- native 현재: 카탈로그 행만, `keymap_run`에 없음.
- 의존: explorer-search-37(그 lane 사양 "explorer-search-37", 묶음 C). 그 lane이 `Run`·intent·검사를 만듭니다.
- 이 lane이 할 일: 병합 상태에서 팔레트 두 명령이 seed 없이 검색 보기를 열고(12는 바꾸기 열림) 보조 창에서는 그 창 탐색기에 열리는지 App 검사 1건. 그 lane 결정 D8이 A면 이미 갱신된 행을 확인만 합니다.
- 완료 판정: 검사 통과 → complete.

#### commands-ui-14 view.git (unwired, 의존, 묶음 3)

- 원본: `command-catalog.ts:94` `keymapId: 'git'`(⌃⇧G), `requestShowExplorerView('git')`.
- 의존: git-1(git spec "git-1": `Run::ShowGit` 또는 `ShowExplorer` 보기 인자 일반화, `runs_without_focused_project`에 넣지 않음).
- 이 lane: 팔레트 경로 App 검사 1건(focused slot만 Git 보기·접힘 해제), 행 갱신.

#### commands-ui-30 sync.uploadNow, commands-ui-31 sync.downloadNow (unwired, 의존, 묶음 4)

- 원본(`TS/entities/sync/sync.commands.ts:7-43`)
    - 업로드: `uploadSync()` 성공 → `toast.success(settings.syncUploadSuccess)`, 실패 → `toast.error(settings.syncUploadFailed)`(설명 없음).
    - 다운로드(force false): 결과 `conflict` → `toast.warning(settings.syncConflictTitle, { description: settings.syncConflictDescription, action: { label: settings.syncConflictPullRemote, onClick: () => runSyncDownload(true) } })`. `applied` → `toast.success(settings.syncDownloadSuccess)`. 실패 → `toast.error(settings.syncDownloadFailed)`. force true는 conflict를 내지 않습니다(16-23 주석).
    - 활성 조건 없음(연결되지 않았으면 백엔드 오류 → 실패 toast). 카테고리 `KEYMAP_CATEGORY.SYNC`.
    - Settings 화면의 같은 동작(settings-46/47)은 충돌 때 toast가 아니라 대화상자를 엽니다. 명령은 toast입니다.
- native 현재: runtime `CR/taide-runtime/src/sync_actions.rs` `sync_upload`(186), `prepare_sync_download`(254), `apply_sync_download`(280, 결과 `SyncDownloadResult::{Applied{status}, Conflict{remote_updated_at}}`, `CR/taide-model/src/sync.rs:46-50`). 원격 경로 조립 선례 `APP/remote-sync.rs:49-99`(`run_nonabortable_result`, `settings_actions::apply_and_broadcast` + reconcile). toast 동작 버튼은 `UI/toast.rs` `Options{description, action: Some(Action{label, id})}`와 `Toasts::take_actions`, 앱 `APP/toast.rs` `Action` enum, 소비 `NativeApplication::run_toast_actions`(application.rs)가 있습니다.
- 의존: settings S4가 `domains::settings`에 동기화 호스트 동작(업로드, 다운로드{force}), 가짜 gist 포트(`Ports.gist`), `SyncStateChanged` 소비를 만듭니다.
- 설계
    - `Run::SyncUpload`, `Run::SyncDownload`(native-host, 활성 `Always`) → `ShellIntent::Commands(Intent::Sync(SyncRequest::Upload | Download{force: false}))`.
    - `domains::commands::Command::Sync(SyncRequest)`: settings lane이 만든 동기화 실행 함수(포트 포함)를 그대로 부르고 `Reply::SyncFinished{ request, result }`로 돌려줍니다. 실행 함수가 settings 영역 dispatch 안에 묶여 있으면 그 파일에 `pub(crate)` 함수 하나만 꺼내는 최소 수정을 합니다(영역 블록 끝 추가).
    - `apply_commands_reply`: 위 원본 문구 표 그대로. 충돌 toast는 `toasts.notify(Kind::Warning, title, Options{ description: Some(..), action: Some(Action{ label: settings.syncConflictPullRemote, id: toast::Action::SyncPullRemote }) })`.
    - `APP/toast.rs` `Action::SyncPullRemote` 추가, `run_toast_actions`에서 `Intent::Sync(Download{force: true})`와 같은 제출. 원본은 dirty flush를 하지 않지만 기존 `run_toast_actions`가 먼저 `flush_dirty`를 부르는 구조이므로 그 동작은 그대로 둡니다(파일 저장과 무관한 설정 동기화라 결과 차이 없음).
- 검사(App, 가짜 gist·가짜 secret): 업로드 성공/실패 문구, 다운로드 applied → 성공 문구·설정 반영, 원격이 더 새로움 → warning·설명·동작 버튼 → 버튼 클릭 → force 다운로드 1회·성공 문구, 미연결 → 실패 문구. 키 override(`sync.uploadNow`에 키 지정, 기존 `APP/keymap-tests.rs` 90 형태) → 같은 실행.
- 완료 판정: 통과 → 두 행 complete. 실제 GitHub·Keychain은 4.2 허락 항목.

#### commands-ui-32 cli.connectExternalEditor, 33 cli.installShellCommand, 34 cli.uninstallShellCommand (unwired, 의존, 묶음 4)

- 원본(`TS/entities/agent/agent.commands.ts:18-66`): macOS에서만 등록(`IS_MAC`), 카테고리 `KEYMAP_CATEGORY.SHELL_COMMAND`.
    - connect(18-30): 상태 조회 → `installed && !dangling`이면 그 상태, 아니면 설치 → 결과가 `installed && !dangling`이 아니면 `toast.error(settings.cliInstallFailed)`, 맞으면 `toast.success(settings.cliExternalEditorConnected)`. 예외 → `settings.cliInstallFailed`. 관리자 프롬프트 취소는 오류가 아니라 상태 반환이므로 실패 문구가 됩니다.
    - install(32-39): 설치 호출이 오류 없이 끝나면 `settings.cliInstallSuccess`, 예외면 `settings.cliInstallFailed`(취소해도 성공 문구 — 4절 결정 8).
    - uninstall(41-48): `settings.cliUninstallSuccess` / `settings.cliUninstallFailed`.
- native 현재: 카탈로그 `platform: mac` → `Command::is_registered(is_mac)`. 상태 `CR/taide-runtime/src/agent_host.rs` `cli_install_status`. 설치·해제 실행 함수는 settings S1이 `agent_host::cli_install/cli_uninstall`과 `Ports.cli{status, install, uninstall}`로 만듭니다(settings spec settings-16).
- 설계: `Run::Cli(CliCommand::{Connect, Install, Uninstall})`(native-host, 활성 `Always`) → `Intent::Cli(..)` → `Command::Cli(..)`가 settings `Ports.cli`를 부르고(`run_nonabortable_result`) `Reply::CliFinished{ command, result: AppResult<CliInstallStatus> }` → 위 문구. 성공 응답 상태로 settings 캐시(`Integrations`)도 덮어 Settings CLI 행이 같은 상태를 보이게 합니다(원본은 `getCliInstallStatus` query 무효화 없이 IPC 직접 호출이지만, native 캐시를 같은 응답으로 덮는 것은 화면 차이가 없음).
- 검사(App, 가짜 `CliPorts`): connect — 이미 설치됨 → 설치 미호출·연결 문구, 미설치 → 설치 1회·연결 문구, 설치 후에도 미설치(취소) → 실패 문구. install/uninstall 성공·실패 문구. 비 macOS 빌드에서 세 명령 미등록(기존 `is_registered` 검사 재사용).
- 완료 판정: 통과 → 세 행 complete. 실제 설치는 4.2 허락 항목.

#### commands-ui-35 ai.inlineEdit (unwired, 근거, W1-A)

- 원본: `TS/entities/ai/ai.commands.ts:6-14`, `isEnabled: activeEditorActionIds !== null`, 실행은 focused 편집기의 `taide.aiInlineEdit`.
- native 현재: `execution("ai.inlineEdit") = EditDocument(Completion(TriggerEdit))`, `enablement = ActiveEditor`(원본과 같음). 팔레트 경로는 `APP/domains/commands.rs` `show_palette`의 `id == "ai.inlineEdit"` 분기가 focused source에 `queue_command`. App 검사 `APP/application-ai-edit-tests.rs` 104(`run_palette(.., "ai.inlineEdit")`)와 editor-78 근거.
- 설계: 코드 변경 없음. 대응표 갱신(편집 기능 자체의 남은 부분은 editor-78 행).
- 완료 판정: complete.

#### commands-ui-36 git.revertHead, 37 git.createTagOnHead, 38 git.toggleBlame, 39 git.openFileHistory (unwired, 의존, 묶음 3)

- 원본(`TS/entities/git/git.commands.ts:14-46`)
    - 36: 활성 프로젝트 필요. `revertGitCommit({ projectId, rev: 'HEAD' })` → 충돌이면 `toast.warning(git.revertConflict)`, 아니면 `toast.success(git.revertSuccess)`, 오류 `toast.error(describeIpcError)`.
    - 37: 활성 프로젝트 필요. `requestShowExplorerView('git')` + `requestOpenCreateTagDialog({ target: 'HEAD' })`.
    - 38: 항상 활성, focused 편집기에 `taide.toggleBlame`. 39: 항상 활성, focused 편집기에 `taide.openFileHistory`(편집기가 없으면 무동작).
- native 현재: 36·37은 `enablement = ActiveProject`만 있고 실행 없음. 38·39는 `Always`·실행 없음. `monaco.taide.toggleBlame`·`monaco.taide.openFileHistory`도 실행 없음(43의 25개에 포함).
- 의존: git-36(두 명령), git-57(blame), git-55(파일 이력). git spec이 각 행의 "완료: commands-ui-xx 갱신 자료"를 남깁니다.
- 이 lane: 병합 상태에서 팔레트·키 override 경로 App 검사(36: 임시 저장소 HEAD revert 성공 문구, 37: Git 보기 + 태그 대화상자 대상 HEAD, 38·39: 편집기 focus 상태에서 실행, 편집기 없음 → 무동작). git lane 결정 5(createTagOnHead 원본 결함 미재현)를 따릅니다.
- 완료 판정: 의존 행 complete 후 complete.

#### commands-ui-40 terminal.runSelectedText (unwired, 근거, W1-A)

- 원본: `TS/entities/terminal/terminal.commands.ts:5-12`, 활성 조건 없음. 받는 쪽 `editor-area.tsx:283-288` `runSelectedTextInTerminal`: focused 편집기가 없으면 무동작, 있으면 선택 또는 현재 줄을 `runInTerminal(text, null)`(257-281: focused pane의 터미널 탭 재사용 또는 새 탭, `text + "\n"`).
- native 현재: `execution = EditDocument(RunSelectedText)`, `enablement = ActiveEditor`(native-host). editor-82 complete(메뉴·두 팔레트·재지정 키, App 검사 `APP/application-context-menu-tests.rs` 560·683·1477 등).
- 차이: 편집기가 없을 때 원본은 활성 행(무동작), native는 비활성 행. 4절 결정 2.
- 설계: 결정 2 A(추천)면 코드 변경 없음, 비고에 차이 기록. B면 `enablement`에서 이 id를 빼 `Always`로 두고(편집기가 없으면 `intent_with_editor`가 `focused_view?`로 None) 검사 단언을 바꿉니다.
- 완료 판정: complete.

#### commands-ui-41 task.runTask (unwired, 근거, W1-A)

- 원본: `TS/entities/task/task.commands.ts:7-15`, 활성 프로젝트 필요, `requestOpenTaskRunner()`.
- native 현재: shell-24 complete(`Run::OpenTaskRunner` → `Intent::OpenTaskRunner(project)` → `domains::commands` `DetectTasks`·`RunTask`, App 검사 `APP/application-shell-dialogs-tests.rs` 199-212, UI `command-palette-tests.rs` 472-475). terminal-agent-77도 같은 기능입니다.
- 설계: 코드 변경 없음. 이 행과 terminal-agent-77을 같은 근거로 갱신합니다(terminal-agent lane이 먼저 갱신했으면 확인만).
- 완료 판정: complete.

### 2.3 팔레트·키맵

#### commands-ui-52 cmdk command-score (partial, 근거, W1-B)

- 원본: cmdk 기본 필터 `commandScore(value, search, keywords)`(`NM/cmdk/dist/chunk-NZJY6EH4.mjs`). 소비 표면은 `shouldFilter`를 끄지 않고 `CommandInput`이 있는 곳뿐입니다: `TS/features/settings/font-picker.tsx`(글꼴 검색), `TS/features/project/project-display-dialog.tsx:112-136`(아이콘 검색). 팔레트·task runner·branch switcher는 `shouldFilter={false}`, language/option picker는 입력이 없습니다. 정렬은 점수 내림차순, 같은 점수는 원래 순서(안정 정렬), 점수 0은 숨김.
- native 현재: `UI/command-score.rs`(이식), 소비 `UI/settings-code-view.rs` Picker(검색 결과 `sort_by` 안정 정렬)와 `UI/project-display-dialog.rs` `icon_matches`. 점수표 검사는 동결 crate `native/taide-remote-web/tests/settings-resources.rs` 749-772에만 있고, 아이콘 검색 UI 검사는 `UI/project-display-dialog-tests.rs` 317.
- 빠진 것: UI crate 안의 점수표 검사, 같은 점수 순서 검사.
- 설계: `UI/command-score.rs`에 `#[cfg(test)] mod tests`로 remote-web 검사의 14개 사례 표를 옮기고(값은 원본 구현 기준), 같은 점수 두 항목의 순서 보존(`icon_matches`)과 빈 검색어 전체 표시를 단언합니다.
- 완료 판정: complete.

#### commands-ui-54 최근 항목(MRU) (missing, 근거, W1-B)

- 원본: `docs/features/command-palette.md:92` "MRU 는 미구현이다". TS 팔레트 코드에 MRU 없음(`rg -i "mru|recent" command-palette*` 0건).
- 설계: 대응표를 not-applicable로 바꾸고 비고에 근거 문서 줄을 적습니다.

#### commands-ui-59 디스패치 대상(키맵 41개) (partial, 의존, 묶음 3)

- 원본: `TS/shared/lib/keymap/keymap.ts:97-243` 41개 엔트리의 소비자(`useGlobalKeymap`).
- native 현재: `UI/keymap-defaults.json` 41개 중 35개가 `keymap_run`으로, `find`가 `editor.find`로, 터미널 점프 2개가 `APP/terminal_surface.rs`로 처리됩니다. 남은 3개: `search`, `search-replace`(explorer-search-37), `git`(git-1). `APP/command-dispatch.rs` 검사가 이 3개를 unavailable로 단언합니다.
- 설계: 의존 병합 뒤 그 검사의 `KEYMAP_ACTIONS`를 38개로 늘리고 "unavailable" 목록에서 3개를 지웁니다. 추가로 "keymap-defaults의 모든 id는 `accepts`가 참이거나 터미널 점프 2개 중 하나" 전수 단언을 넣습니다.
- 완료 판정: 전수 단언 통과 → complete.

#### commands-ui-61 find/search/search-replace 키 (partial, 의존, 묶음 3)

- 원본: `editor-area.tsx:154-161` `openFind`: 위젯 focus가 있는 편집기면 그 편집기 찾기, 없으면 `requestOpenSearchPanel()`. ⌘⇧F·⌘⇧H는 11·12.
- native 현재: `find`는 `editor.find`(ActiveEditor)일 때만 소비. 편집기 focus가 없을 때의 검색 패널 대체가 없습니다.
- 의존: explorer-search-37(그 사양 묶음 C "`command-dispatch.rs`(find 대체)").
- 이 lane: 병합 뒤 App 검사(편집기 focus 없음 + ⌘F → 검색 보기, 편집기 focus → 편집기 찾기)로 닫습니다.

#### commands-ui-62 explorer/git 키 (unwired, 의존, 묶음 3)

- explorer는 shell-21에서 완료. git은 git-1. 병합 뒤 ⌃⇧G App 검사로 닫습니다(프로젝트 없으면 키 미소비, git spec git-1 설계와 같음).

#### commands-ui-43 monaco.* 액션 실행 (partial, 바로(일부), 묶음 8)

- 원본: `TS/shared/lib/monaco/monaco-action-commands.ts:12-43`. 각 행은 focused 편집기에 `trigger`. 활성 조건: `MONACO_ACTION2_IDS`(12-34, 21개)는 항상 활성, 나머지는 편집기의 `getSupportedActions`에 있을 때만.
- native 현재: 157개 중 132개가 실행됩니다(`UI/command-registry.rs` `execution`의 `line_command`·`cursor_command`·`fold_command`·`IndentationCommand`·`HighlightCommand`, 편집기 crate의 `completion::Command`·`documentation::Command`·`symbol_locations::Command`·`problem_navigation::Command`·`formatting::Command`·`indent::Command`, UI `editor_find::FindCommand`·`editor_code_actions::Command`, 특수 id). editor-34가 같은 기능의 편집기 영역 행입니다.
- 실행되지 않는 25개와 처리

| id(`monaco.` 생략) | 원본 기본 키 | 원본 동작 | 처리 |
| --- | --- | --- | --- |
| `editor.action.fontZoomIn` / `fontZoomOut` / `fontZoomReset` | 없음 | 창(webview) 전역 zoom level ±1, 범위 −5~20, 글꼴 크기·줄 높이 × (1 + 0.1 × level), 저장 안 함(`NM/monaco-editor/esm/vs/editor/common/config/editorZoom.js:16-17`, `config/fontInfo.js:34-36`, `contrib/fontZoom/browser/fontZoom.js:18,30,42`) | 구현: viewport별 `EditorZoom` 상태를 `APP`에 두고 편집기 appearance 계산(`taide_native_ui::presentation::update_editor_font_size` 호출부)에 곱함. 설정 글꼴 크기(⌘=/⌘-, `next_editor_font_size`)와는 별개 |
| `editor.action.inPlaceReplace.up` / `.down` | ⇧⌘, / ⇧⌘. | 선택(없으면 커서 단어)을 다음/이전 값으로: 숫자 증감(소수 자리 유지, 0에서 내림은 무동작), 값 집합 `true/false`, `True/False`, `Private/Public/Friend/ReadOnly/Partial/Protected/WriteOnly`, `public/protected/private` 순환(`NM/monaco-editor/esm/vs/editor/common/languages/supports/inplaceReplaceSupport.js`, 컨트롤러 `contrib/inPlaceReplace/browser/inPlaceReplace.js`) | 구현: 편집기 crate `line-commands.rs` 옆 새 `inplace-replace.rs`에 `BasicInplaceReplace` 이식, `LineCommand` 또는 별도 `DocumentEdit`로 실행. 바뀐 범위 짧은 강조(컨트롤러의 decoration)는 컨트롤러 파일을 읽어 같은 시간으로 |
| `editor.action.toggleTabFocusMode` | ⌃⇧M | Tab이 들여쓰기 대신 focus 이동, 창 전역 토글 | 구현: viewport별 플래그, `UI/editor_surface.rs`의 Tab 처리에서 플래그가 켜지면 egui focus 이동 |
| `editor.action.hideHover` | Escape(hover 표시 중) | 문서 hover 닫기 | 구현: `documentation::Command`에 `Hide` 추가(기존 hover 닫기 경로 재사용) |
| `editor.action.gotoLine` | ⌃G | 편집기 상단 quick input "줄:열로 이동" | 4절 결정 4(추천: 팔레트를 `:` 모드로 열기) |
| `editor.action.quickOutline` | ⇧⌘O | 편집기 상단 quick input 문서 심볼 | 결정 4(추천: 팔레트를 `@` 모드로 열기) |
| `editor.action.pasteAsText` | 없음 | 클립보드 평문 붙여넣기 | 구현: 클립보드 읽기 포트(주입 가능)를 두고 일반 붙여넣기 경로로 삽입 |
| `editor.action.pasteAs` | 없음 | 붙여넣기 종류 선택(standalone에서는 평문 한 종류) | 결정 5 |
| `editor.action.clipboardCopyWithSyntaxHighlightingAction` | 없음 | HTML+평문 복사 | 결정 5 |
| `editor.action.openLink` | 없음 | 커서 아래 URL 열기(편집기 링크 감지) | 결정 5(native 편집기에 URL 링크 감지가 없고 대응표에 행도 없음) |
| `editor.action.unicodeHighlight.*` 4개 | 없음 | 모호·보이지 않는 문자 강조 끄기·옵션 | 결정 5(monaco 기본값으로 강조가 켜져 있으나 native에 기능·행 없음) |
| `editor.action.showOrFocusStandaloneColorPicker` | 없음 | 커서 위치 색 선택기 위젯 | 결정 5 |
| `editor.action.inlineSuggest.toggleAlwaysShowToolbar` | 없음 | inline 제안 toolbar 항상 표시 토글 | editor-77(AI inline 완성)의 toolbar 범위. 그 행이 toolbar를 구현하면 연결, 아니면 비고 |
| `editor.action.goToLocation` | 없음 | 인자 없는 실행은 등록된 action이 없어 무동작(`contrib/gotoSymbol/browser/goToCommands.js:591-613`은 `goToLocations` 명령이 인자로 만드는 내부 객체) | 결정 2(무동작 활성 행 미재현): 비활성 유지, not-applicable 비고 |
| `editor.action.increaseHoverVerbosityLevel` / `decrease..` | 없음 | hover provider가 verbosity를 지원할 때만 효과. TAIDE hover는 LSP라 효과 없음 | 결정 2: 비활성 유지, 비고 |
| `taide.gitStageSelection` / `taide.toggleBlame` / `taide.openFileHistory` | 없음 | git 기능 | git lane(git-58~60 선택 stage, git-57, git-55) |

- 새 기본 키: 원본 monaco 기본 키가 있는 항목은 `UI/keymap.rs`가 읽는 편집기 기본 키 JSON에 플랫폼별로 추가합니다(새 파일 `UI/editor-commands-keymap-defaults.json`, 형식은 `editor-keymap-defaults.json`과 같음). 플랫폼별 정확한 키는 위 monaco 소스의 `kbOpts`에서 옮깁니다.
- 검사: 편집기 crate 단위(in-place replace 표: `5`→`6`, `1.5`→`1.6`, `0` 내림 무동작, `true`↔`false`, `private`→`public` 순환), UI 검사(zoom 곱, tab focus 모드에서 Tab이 focus 이동), App 검사(팔레트·기본 키 각 1건), 레지스트리 전수 검사("`Unavailable`인 `monaco.*` id 목록 = 이 표의 not-applicable·의존 목록"으로 단언해 누락 방지).
- 완료 판정: 구현 대상 통과 + 나머지가 not-applicable·의존 비고로 정리되면 complete. 결정 5가 A면 4종 의존이 남아 partial입니다.

### 2.4 공용 UI 프리미티브

모든 행은 2.0.3 토큰과 2.0.4 완료 기준을 따릅니다. 새 모듈은 UI crate의 kebab-case + `#[path]` 관례로 둡니다.

#### commands-ui-68 button (partial, 바로, 묶음 5)

- 원본: `TS/shared/ui/button.tsx:7-35`. 공통: inline-flex, gap 8px, radius 6px, 14px medium, 비활성 투명도 50%·클릭 불가, focus-visible 테두리 `ring` + 3px 링 `ring` 50%(destructive는 `destructive` 20%), 아이콘 16px.
    - 소비자가 쓰는 variant(`rg variant=` 집계): default(배경 primary, 글자 primary-foreground, hover primary 90%), destructive(배경 destructive, 글자 흰색, hover 90%), outline(1px `border`, 배경 background, shadow-xs, hover 배경 accent·글자 accent-foreground), ghost(투명, hover accent). `secondary`·`link`는 소비자 없음.
    - size: default 36px·좌우 16px(아이콘 있으면 12px), xs 24px·좌우 8px(6px)·12px 글자·gap 4px·아이콘 12px, sm 32px·좌우 12px(10px)·gap 6px, icon-xs 24×24·아이콘 12px. `lg`·`icon`·`icon-sm`·`icon-lg`는 소비자 없음.
- native 현재: 원본과 같은 그림은 `UI/snippet-editor.rs`의 `ButtonStyle{Primary, Destructive, Outline, Ghost}`, `button`, `paint_button_focus`(keyboard focus일 때만 링, `button_color_motion` 150ms)에만 있습니다. 나머지는 egui 기본 `Button`(전역 visuals: `button.background`)입니다. 예: `APP/close_dialog.rs`(세 버튼이 모두 egui 기본), `APP/delete_dialog.rs`, `APP/open_path_dialog.rs`, `UI/project-display-dialog.rs`, `UI/project-group-dialog.rs`, `UI/keybinding-editor.rs`, `UI/settings-view.rs` `outline_buttons`, `UI/theme-editor.rs`, `UI/error-boundary.rs`(재시도 outline xs 직접 그림).
- 설계: `UI/ui-button.rs`(`crate::ui_button`): `Variant{Default, Destructive, Outline, Ghost}`, `Size{Default, Xs, Sm, IconXs}`, `pub fn show(ui, Spec{ label, icon: Option<Glyph|Icon>, variant, size, enabled }) -> Response`. `snippet-editor.rs`의 그림·focus modality·motion 코드를 옮겨 오고 snippet editor는 이 모듈을 부르게 바꿉니다(화면 변화 없음). accesskit `Button`, 비활성은 `add_enabled`.
- 이전 대상(native에 있는 소비 화면): 위 목록 전부. 각 소비 화면의 원본 variant·size는 해당 TS 파일의 `<Button variant=.. size=..>`를 따릅니다(예: close-dirty-tab = 취소 outline, 버리기 destructive, 저장 default; 오류 경계 재시도 = outline xs).
- 검사: UI 단위 4×4 조합의 높이·채움색·테두리·글자 크기, hover 색, 비활성 50%, 키보드 focus 링만 보임(마우스 focus 없음). 이전한 화면의 기존 검사 + close dialog의 버리기 버튼 채움색이 `statusIndicator.error`라는 App 단언 1건.
- 완료 판정: (a)(b) 충족 → complete. 다른 lane 화면은 (c)로 확인.

#### commands-ui-69 card (partial, 바로, 묶음 5)

- 원본: `TS/shared/ui/card.tsx:5-46`. Card: 배경 `card`, 1px `border`, radius 8px, 위아래 20px, 세로 gap 16px, shadow-sm. Header: 좌우 20px, gap 6px. Title semibold leading 1. Description 12px muted. Content 좌우 20px. 소비자 2곳: `TS/features/settings/settings-section.tsx:11-16`(Title 14px, Content 세로 gap 12px, `scroll-mt-8`), `TS/features/welcome/welcome-screen.tsx:110-124`(Card 위아래 16px, Header·Content 좌우 16px, Title 12px + 아이콘 gap 6px, Content 2열 grid gap 16/6, 12px).
- native 현재: `UI/settings-view.rs`(`CARD_PADDING` 20, `CARD_GAP` 16, `CARD_RADIUS` 8, `CARD_DESCRIPTION_GAP` 6), Welcome(UI `shell.rs` welcome 렌더). `UI/theme-editor.rs` `card`는 원본 Card 소비자가 아닙니다(이전 대상 아님).
- 설계: `UI/ui-card.rs` `pub fn show(ui, tokens, Padding, header: impl FnOnce, content: impl FnOnce)`. settings view와 Welcome을 이 모듈로 바꿉니다. settings lane이 `Section::description()`을 일반화했으면 그 결과를 그대로 넘깁니다.
- 검사: UI 단위(채움·테두리·반경·여백), Settings 카드와 Welcome 카드 rect 여백 단언 각 1건.
- 완료 판정: complete.

#### commands-ui-70 checkbox (partial, 의존, 묶음 5)

- 원본: `TS/shared/ui/checkbox.tsx:7-21`. 14×14, radius 2px, 1px `input` 테두리, shadow-xs, 체크되면 배경·테두리 primary·글자 primary-foreground, 체크 아이콘 12px, 비활성 50%, focus 링 3px. 소비자: `TS/shared/ui/file-group-header.tsx:21`(검색 결과 파일 선택, explorer-search), `TS/widgets/plugin-manager/vsix-import-dialog.tsx`(테마 후보, settings S3).
- native 현재: 공용 checkbox 없음(`ui.checkbox` 사용처는 sticky scroll 문맥 메뉴뿐이며 원본 소비자 아님).
- 설계: `UI/ui-checkbox.rs` `pub fn show(ui, checked: &mut bool, enabled, accessible_label) -> Response`, accesskit `CheckBox` + toggled. 84의 공용 header와 settings S3 VSIX 목록이 씁니다. 두 lane이 자체 구현을 넣었으면 이 모듈로 바꿉니다.
- 검사: UI 단위(체크/해제/비활성 색, Space·클릭 토글), 두 소비 화면 단언 각 1건.
- 완료 판정: 두 소비 화면이 병합돼 있고 (b)(c) 충족 → complete.

#### commands-ui-79 switch (partial, 바로, 묶음 5)

- 원본: `TS/shared/ui/switch.tsx:6-21`. 32×18.4(1.15rem), 둥근 끝, 켜짐 primary·꺼짐 `input`, thumb 16px 배경색, 켜지면 14px 이동(`calc(100%-2px)`), 비활성 50%, focus 링 3px. 소비자: `TS/features/settings/switch-field.tsx`, `font-picker.tsx`(monospace만 보기), `remote-section.tsx`, `agent-hooks-toggle.tsx`, `ai-auto-tab-toggle.tsx`.
- native 현재: `UI/settings-view.rs` `switch_control`(원본 모양), `UI/settings-code-view.rs` 글꼴 Picker의 `settings.fontFamilyMonospaceOnly` 스위치. settings lane이 새 스위치(원격·hooks·AI)를 추가합니다.
- 설계: `UI/ui-switch.rs`로 `switch_control`의 그림·상호작용을 옮기고 settings-view·settings-code-view·settings lane 새 스위치가 쓰게 합니다. thumb 이동 transition은 원본 `transition-transform`(150ms 기본)을 `button_color_motion`과 같은 곡선으로.
- 검사: UI 단위(켜짐·꺼짐·비활성 rect·색, 클릭·Space 토글, accesskit `Switch` toggled), 기존 settings 검사.
- 완료 판정: complete(settings lane 새 스위치는 (c)).

#### commands-ui-76 progress (partial, 의존, 묶음 5)

- 원본: `TS/shared/ui/progress.tsx:6-19`. 트랙 primary 20%, 높이 6px, 둥근 끝, 표시줄 primary, `translateX(-(100 - value)%)`. 유일한 소비자 `TS/features/settings/lsp-server-status-list.tsx`(높이 4px로 덮어씀, settings-33).
- native 현재: 없음. settings S2가 LSP 진행 막대를 그립니다.
- 설계: `UI/ui-progress.rs` `pub fn show(ui, value: f32 /*0..=100*/, height)`, accesskit `ProgressIndicator` + numeric value. settings S2 구현을 이 모듈로 바꿉니다(없으면 그 화면이 이 모듈을 쓰게).
- 검사: UI 단위(0·37·100에서 표시줄 폭 비율, 트랙 색), settings LSP 진행 화면 단언 1건.
- 완료 판정: settings-33 병합 후 complete.

#### commands-ui-77 scroll-area / overlay-scrollbar (partial, 바로, 묶음 5)

- 원본
    - `TS/shared/scroll/scroll-container.tsx:17-60`: viewport는 기본 scrollbar 숨김, 방향별 overlay track. both면 세로 track 아래 10px, 가로 track 오른쪽 10px을 비웁니다.
    - `TS/shared/scroll/overlay-scrollbar.tsx:13-40`: track 10px(`w-2.5`/`h-2.5`), 배경 `scrollbar.track`, 기본 투명·스크롤 가능할 때만 포인터, 보일 때 불투명(200ms), thumb 둥근 끝 `scrollbar.thumb`, hover `scrollbar.thumbHover`.
    - `TS/shared/hooks/use-overlay-scrollbar.ts:25-213`: 스크롤하면 보이고 1000ms 뒤 숨김(`SCROLLBAR_AUTO_HIDE_MS`), track hover·thumb drag 중에는 숨기지 않음, thumb drag는 비례 이동, track 클릭은 thumb 가운데가 클릭 지점에 오도록 부드러운 스크롤.
    - `TS/shared/lib/scrollbar-metrics.ts:16-76`: 스크롤 가능 = 남는 길이 > 1px 그리고 track > 0, thumb = max(24px, client/scroll × track)를 track으로 제한, offset 비례, 퍼센트 반올림, drag·클릭 계산식.
    - `TS/shared/ui/scroll-area.tsx`(Radix)는 소비자가 없습니다(대상 아님).
    - 소비자(23곳): 시스템 사용량, 파일 이력, 테마 편집기, 검색 편집기, 키바인딩 편집기, 스니펫 편집기, 탭 바(가로), Git 패널·그래프, 문제 패널, 마크다운 미리보기, 탐색기 트리, 미리보기 5종, outline, Settings, 검색 패널·결과.
- native 현재: 대부분 egui `ScrollArea` 기본 scrollbar. 편집기만 테마 scrollbar 색을 씁니다(`UI/presentation.rs` `editor_display_colors`). 팔레트 목록은 `ScrollBarVisibility::AlwaysHidden`(원본 cmdk `scrollbar-hidden`과 같음).
- 설계: `UI/ui-scroll.rs`
    - `pub fn metrics(..)`: 위 계산식 4개를 그대로(`computeScrollbarThumbMetrics` 등) 옮긴 순수 함수.
    - `pub fn show<R>(ui, id, orientation, add_contents) -> ScrollAreaOutput<R>`: egui `ScrollArea`를 `AlwaysHidden`으로 쓰고 `ScrollAreaOutput{state.offset, content_size, inner_rect}`로 overlay track·thumb를 그 위에 그립니다. 보임 상태와 마지막 스크롤 시각은 egui temp data(`(id, "overlay-scrollbar")`), 1초 숨김은 `request_repaint_after`. thumb drag는 `interact(Sense::drag())`, track 클릭은 목표 offset으로 `state.offset`을 animate(egui `ScrollArea` 상태 `store`).
    - 이전: 위 소비자 중 native에 있는 화면(시스템 사용량, 테마·키바인딩·스니펫 편집기, 탭 바, 문제 패널, 마크다운 미리보기, 탐색기 트리, 미리보기, outline, Settings). 편집기 본문 scrollbar는 원본도 monaco 자체 scrollbar라 대상이 아닙니다.
- 검사: 앱 단위·UI 단위 계산식 표(원본 `TS/shared/lib/scrollbar-metrics.test.ts`가 있으면 그 사례를 옮김), UI 하네스(스크롤 → 보임 → 1초 뒤 숨김, hover 중 유지, thumb drag 비례, track 클릭 중심 이동, 스크롤 불가면 포인터 통과), 이전한 화면의 기존 검사.
- 완료 판정: complete(다른 lane 화면 Git·검색은 (c)).

#### commands-ui-84 file-group-header (partial, 의존, 묶음 5)

- 원본: `TS/shared/ui/file-group-header.tsx:19-36`. 행 12px, hover `explorer.itemHover`. 선택 가능하면 왼쪽 8px + checkbox(70). 본문 `role=button`, Enter·Space로 펼침 토글, `aria-expanded`, 위아래 2px·오른쪽 8px·왼쪽 4px(선택 가능)/8px, gap 4px: chevron 12px(펼치면 90도), 파일 형식 아이콘 14px, 파일 이름 medium, 경로 muted(줄임), 오른쪽 개수 muted tabular. 소비자: `TS/features/problems/problems-panel.tsx:100`(선택 없음), `TS/features/search/search-results-list.tsx:66`(선택 있음).
- native 현재: `APP/problems.rs` `show_panel`이 그룹 머리를 직접 그립니다(`Glyph::ChevronRight`, `group_label`, `group_path`). 검색 결과는 explorer-search lane이 만듭니다.
- 설계: `UI/file-group-header.rs` `pub fn show(ui, Header{ path, count, expanded, selected: Option<bool>, select_label }) -> Output{ toggled, selection_toggled }`. 문제 패널과 검색 결과가 이 모듈을 씁니다(두 곳 사용이라 공통화 기준 충족).
- 검사: UI 단위(요소 순서·간격·회전, Enter/Space, accesskit expanded), 문제 패널 기존 검사, 검색 결과 checkbox 단언 1건.
- 완료 판정: 검색 결과 병합 후 complete.

#### commands-ui-85 toast (partial, 의존, 묶음 4)

- 원본: sonner `toast.success/error/warning/info`, `description`, `action{label, onClick}`. `toast.loading`·`promise`·`dismiss`는 사용처 0건. action 사용처는 동기화 충돌(`sync.commands.ts:28-31`)과 탐색기 생성·이름 바꾸기 재시도(`TS/widgets/explorer/use-explorer-entry-crud.ts:127,174`) 둘뿐입니다.
- native 현재: `UI/toast.rs` `Kind{Warning, Error, Success, Info}`, `Options{description, action}`, 위치 9종(`Vertical::Middle` 포함), swipe·motion·focus, 동작 버튼 검사(`Probe::Retry`, "Pull Remote" 문구 사례 3157-3198). 앱 소비: 탐색기 재시도(`APP/toast.rs` `Action::RetryCreate/RetryRename`), info(`open_project_first`, 프로젝트 그룹 등), description(AI 편집 실패, application.rs 7029). 남은 원본 action 소비자는 동기화 충돌 하나입니다.
- 설계: 묶음 4의 `Action::SyncPullRemote`가 마지막 소비자입니다. 추가로 4종 × (설명 유무) × (동작 유무) 조합을 한 UI 단위 검사로 단언합니다(기존 개별 검사를 묶는 수준).
- 완료 판정: 30·31 통과 → complete. 영역별 toast 문구는 각 영역 행입니다.

#### commands-ui-67 alert-dialog (partial, 바로, 묶음 6)

- 원본: `TS/shared/ui/alert-dialog.tsx:19-141`(Radix AlertDialog)
    - scrim `modal-scrim`, 내용: 가운데, 폭 512px(`size='sm'`이면 320px), 1px `modal.border`, 배경 `modal.background`, radius 8px, 안쪽 24px, gap 16px, `shadow-overlay-lg`, 열림·닫힘 fade + zoom 95%(200ms).
    - Header: 가운데 정렬(넓은 창 default size는 왼쪽), gap 6px. Title 18px semibold. Description 14px muted.
    - Footer: 오른쪽 정렬 가로, gap 8px(`sm`이면 2열 grid). Cancel = Button outline, Action = Button default(소비자가 destructive 지정 가능).
    - Radix 동작: 바깥 클릭으로 닫히지 않음, Escape는 닫힘(onOpenChange false = 취소), 열리면 Cancel에 focus, focus 가둠, 닫히면 연 곳으로 focus 복귀.
    - 소비자 13곳: close-dirty-tab(`APP/close_dialog.rs`), entry-delete(`APP/delete_dialog.rs`), project group 삭제(`UI/project-group-dialog.rs` `ProjectGroupDeleteDialog`), snippet editor(`UI/snippet-editor.rs`), theme editor(`UI/theme-editor.rs`), sync 충돌·plugin 제거·VSIX 덮어쓰기·hooks 동의(settings lane `UI/settings-dialogs.rs`), 검색 바꾸기 확인(explorer-search), hunk 버리기·충돌 해결·Git 패널(git lane).
- native 현재: `ProjectGroupDeleteDialog`(project-group-dialog.rs 207 이후)가 원본 규칙(Cancel 시작, Escape 닫힘, 바깥 클릭 무시)을 따릅니다. `close_dialog.rs`는 egui 기본 `Modal`이라 바깥 클릭으로 닫히고(`should_close`), 버튼 색·focus가 원본과 다르며 "busy"면 spinner만 보입니다. `delete_dialog.rs`는 Cancel focus는 맞고 바깥 클릭 처리는 확인이 필요합니다.
- 설계: `UI/ui-dialog.rs`에 `AlertDialog`(제목, 설명, 버튼 목록 `[(label, ui_button::Variant)]`, size `Default|Sm`)를 둡니다. 내부는 `modal::Layer`(scrim·전환), `modal::trap_focus`, `modal::FocusReturn`, `modal::Composition`, `modal::takes_escape`를 씁니다. settings lane의 `settings-dialogs.rs` alert가 있으면 그것을 이 모듈로 옮기고 settings는 이 모듈을 부르게 합니다(settings spec 2.6이 "구현은 snippet-editor 대화상자를 따름"이라 같은 규칙). 이전: close_dialog, delete_dialog, ProjectGroupDeleteDialog, snippet·theme editor의 확인 대화상자.
- 검사: UI 단위(바깥 클릭 무시, Escape = 취소, 첫 focus Cancel, Tab 순환이 대화상자 밖으로 안 나감, 닫힘 후 focus 복귀, sm 폭과 2열 footer), App(close dirty tab: 바깥 클릭에도 열린 채, Escape → 취소, 버리기 destructive 색).
- 완료 판정: (a)(b) → complete. 다른 lane 소비자는 (c).

#### commands-ui-73 dialog (partial, 바로, 묶음 6)

- 원본: `TS/shared/ui/dialog.tsx:29-126`(Radix Dialog). alert와 같은 틀(폭 512px, 안쪽 24px, gap 16px, radius 8px, `shadow-overlay-lg`, fade+zoom 200ms)에 더해: 바깥 클릭으로 닫힘, Escape 닫힘(단 IME 조합 중 Escape는 무시, 46-58), 기본으로 오른쪽 위 16px에 닫기 X 버튼(16px 아이콘, 투명도 70%·hover 100%, 접근성 이름 `common.close`). Title 18px semibold, Description 14px muted, Header gap 8px, Footer 오른쪽 gap 8px.
    - 닫기 X를 끄는 소비자는 팔레트·task runner·파일 이력뿐(`showCloseButton={false}`). 나머지(시스템 사용량, 충돌 비교, 경로로 열기, 태그 만들기, 프로젝트 표시, 키바인딩 편집기, 프로젝트 그룹, VSIX, 새 스니펫 파일)는 X가 있습니다.
- native 현재: `modal::Layer` 기반 대화상자 다수. X 버튼은 키바인딩 편집기(`Icon::Close`)와 시스템 사용량(`common.close`)만 있고, `UI/project-display-dialog.rs`, `UI/project-group-dialog.rs`(생성·편집), `APP/open_path_dialog.rs`에는 없습니다.
- 설계: `UI/ui-dialog.rs` `Dialog{ title, description, show_close, width }` 틀 함수(내용 closure). 이전: 위 native 대화상자 전부, 새 스니펫 파일 대화상자(`UI/snippet-editor.rs`). 팔레트는 이미 별도 구현이고 X가 없어 대상만 확인합니다.
- 검사: UI 단위(바깥 클릭 닫힘, IME 조합 중 Escape 무시, X 클릭 닫힘·accesskit 이름, focus 가둠·복귀), App(프로젝트 표시 대화상자 X로 닫힘, 경로로 열기 바깥 클릭 닫힘).
- 완료 판정: complete(다른 lane 대화상자는 (c)).

#### commands-ui-75 popover (partial, 바로, 묶음 6)

- 원본: `TS/shared/ui/popover.tsx:20-48`. 기본 폭 288px, 1px `popover.border`, 배경 `popover.background`, radius 6px, 안쪽 16px, `shadow-overlay`, 기준에서 4px 떨어짐, 가운데 정렬, fade+zoom, 안에 cmdk 입력이 있으면 그 아래선 `popover.separator`, IME 조합 중 Escape 무시. 소비자: 색 선택기(`TS/features/theme/color-picker.tsx`), 글꼴·옵션·언어 Picker(`TS/features/settings/*-picker.tsx`, 각자 폭·안쪽 여백 덮어씀), branch switcher(git).
- native 현재: `UI/settings-code-view.rs` Picker(egui `Popup`), `UI/theme-color-picker.rs`.
- 설계: `UI/ui-popover.rs` `pub fn frame(tokens) -> egui::Frame`와 위치·IME Escape 규칙 함수. 두 소비 화면의 popup 틀을 이 모듈로 바꿉니다.
- 검사: UI 단위(틀 색·반경·그림자·4px 간격, IME 중 Escape 유지), Picker·색 선택기 기존 검사.
- 완료 판정: complete(branch switcher는 (c)).

#### commands-ui-71 command(cmdk 목록) (partial, 바로, 묶음 6)

- 원본: `TS/shared/ui/command.tsx:10-122`. 입력 줄 높이 36px·아래 1px 테두리·왼쪽 검색 아이콘 16px 50%·placeholder muted. 목록 최대 300px, scrollbar 숨김. 빈 상태 위아래 24px 가운데 14px. 그룹 안쪽 4px, 머리 12px medium muted(좌우 8px, 위아래 6px). 항목: radius 4px, 좌우 8px·위아래 6px, 14px, gap 8px, 비활성 50%, 선택 행 = 배경 `list.activeBackground` + 글자 accent-foreground + 1px 안쪽 링 `app.accent`(95-106 주석: 두 단서 필수), 아이콘 16px muted. Shortcut: 오른쪽 12px muted 넓은 자간. cmdk 기본: 방향키 순환 안 함(`loop` 미지정), 포인터가 지나가면 선택 이동.
- native 현재: `UI/command-palette.rs`(commands-ui-53 complete), `UI/settings-code-view.rs` Picker, `UI/project-display-dialog.rs` 아이콘 grid. 같은 규칙이 세 파일에 따로 있습니다.
- 설계: `UI/ui-command-list.rs`에 행 그림(`paint_item(ui, rect, selected, enabled)`), 그룹 머리, 빈 상태, shortcut 칸, 키보드 이동 규칙(순환 여부 인자)을 두고 세 소비 화면이 씁니다. 팔레트는 원본 팔레트가 `loop`를 켠 경우만 순환(commands-ui-53 근거 유지).
- 검사: UI 단위(선택 행 두 단서, 비활성, 빈 상태), 세 화면 기존 검사.
- 완료 판정: complete(branch switcher는 (c)).

#### commands-ui-72 context-menu, commands-ui-74 dropdown-menu (partial, 바로, 묶음 6)

- 원본: `TS/shared/ui/context-menu.tsx`, `dropdown-menu.tsx`(같은 규칙).
    - 내용: 최소 폭 128px, 안쪽 4px, 1px `menu.border`, 배경 `menu.background`, radius 6px, `shadow-overlay`(하위 메뉴는 `-lg`), 높이 넘치면 세로 스크롤(scrollbar 숨김), 방향별 2px slide + fade + zoom.
    - 항목: radius 4px, 좌우 8px·위아래 6px, 14px, gap 8px, focus(키·포인터) 배경 `menu.itemHover`, 비활성 50%, inset이면 왼쪽 32px, 아이콘 16px muted. destructive: 글자·아이콘 `statusIndicator.error`, focus 배경 그 색 10%.
    - 하위 메뉴 트리거: 열리면 배경 `menu.itemHover`, 오른쪽 chevron 16px. 구분선: 위아래 4px, 좌우 −4px, 1px `menu.separator`. Shortcut: 오른쪽 12px muted. Label(dropdown만 사용): 좌우 8px·위아래 6px 14px medium.
    - Radix 동작: ↑↓ 이동(끝에서 순환 없음), Home/End, Enter·Space 실행, → 하위 메뉴 열기·← 닫기, Escape 닫기, 글자 입력 typeahead(1초 누적).
    - 소비자가 쓰는 부분: Item(default·destructive), Separator, Sub, Shortcut(탐색기 메뉴), Label(rail "프로젝트 추가" 메뉴). checkbox·radio 항목은 미사용.
    - 소비자: context — rail 프로젝트·그룹 메뉴, 탭·탭 바 메뉴, 터미널 메뉴, 탐색기 메뉴, Git 패널·그래프(git lane). dropdown — rail 프로젝트 추가 메뉴, 탭 바 + 메뉴, breadcrumb 메뉴, 검색 이력(explorer-search).
- native 현재: `UI/shell.rs` `menu_item`/`enabled_menu_item`(egui 기본 Button, 전역 visuals의 `menu.itemHover`), `APP/terminal_surface.rs`의 `menu_item`·`menu_separator`·`prepare_menu_search`·`navigate_menu`(Radix typeahead·방향키 이식), `APP/explorer.rs`, `APP/breadcrumb-menu.rs`. destructive 색·shortcut 칸·inset·하위 메뉴 chevron 규칙이 화면마다 다르거나 없습니다.
- 설계: `UI/ui-menu.rs`
    - `pub fn frame(tokens, is_sub)`, `pub fn item(ui, Item{ label, icon, shortcut, variant: Default|Destructive, inset, enabled }) -> Response`(accesskit `MenuItem`), `pub fn separator(ui)`, `pub fn label(ui, text)`, `pub fn submenu(ui, label, add_contents)`.
    - 키보드: `terminal_surface.rs`의 `MenuSearch`·`prepare_menu_search`·`navigate_menu`·`consume_menu_key`를 이 모듈로 옮겨 모든 메뉴가 같은 typeahead·이동을 씁니다(터미널 메뉴는 이 모듈을 부르도록 바꿈).
    - 이전: 위 native 소비자 전부.
- 검사: UI 단위(destructive 색·focus 배경, shortcut 오른쪽 정렬, 구분선 색, Label, 하위 메뉴 →/←, typeahead, 비활성 건너뜀), 각 메뉴의 기존 검사(터미널 메뉴 typeahead 검사는 옮긴 모듈로 그대로 통과해야 함).
- 완료 판정: 두 행 complete(Git 메뉴·검색 이력은 (c)).

### 2.5 장애·배너

#### commands-ui-82 error-boundary (missing, 의존, 묶음 7)

- 원본: `TS/shared/ui/error-boundary.tsx:32-126`, 경계 8곳(`docs/quality-assurance/2026-10-27-native-error-isolation-file-drop.md` shell-12 절).
- native 현재: shell-12와 같은 기능입니다. `UI/error-boundary.rs` 공용 경계와 Welcome·Activity Bar·상태 표시줄·사이드바 패널(메인 slot·보조 창) 경계가 구현·검사됐습니다(2026-10-27-native-error-boundary-foundation.md, -panels.md). 남은 것: 편집기 영역(메인·보조 창) 경계, 루트 `errorBoundary.app` 경계, 픽셀 대조.
- 의존: 운영 문서 3절 wave 4(배치 31 "편집기 장애 경계 설계").
- 이 lane: 대응표 비고를 shell-12 근거와 같게 정정(지금 비고 "구역별 장애 … 경로 없음"은 사실과 다름)하고 partial로 올립니다. 1(window.reload)의 `reset_viewport`가 이 경계를 씁니다.
- 완료 판정: wave 4 뒤 shell-12와 함께 complete.

#### commands-ui-83 status-error-banner (missing, 의존, 묶음 7)

- 원본: `TS/shared/ui/status-error-banner.tsx:9-27`. 고정 위쪽, 높이 32px(`STATUS_ERROR_BANNER_HEIGHT_PX`), 배경 error 15%, 글자 error, 좌우 12px·위아래 6px, 12px, 경고 아이콘 14px, 밑줄 재시도 버튼. 소비자: 로케일(고정 영어 문구)·테마(`theme.loadFailed`) 공급자, 둘 다 실패하면 32px 아래로 쌓음.
- native 현재: `APP/status-error-banner.rs`(shell-94: 실행 중 refresh 실패 배너, 쌓임, 재시도). 남은 것(shell spec shell-94): 시작 시 로드 실패 대체, OS 파일 drop overlay보다 위에 그리기.
- 의존: shell lane 묶음 2(shell-94).
- 이 lane: 병합 뒤 shell-94 근거로 이 행을 갱신합니다.
- 완료 판정: shell-94 complete와 함께 complete.

### 2.6 스타일·hook·i18n·아이콘

#### commands-ui-86 테마 토큰 → 전역 스타일 (partial, 바로, 묶음 7)

- 원본: `TS/shared/styles/global.css:6-297`(`:root` 토큰, shadcn 별칭 138-155, `@theme inline`), base 304-345(본문 13px, 시스템 글꼴, 앱 배경·글자, 선택 금지, webkit scrollbar 10px·thumb 둥근 끝).
- native 현재: `UI/presentation.rs` `visuals(theme)`가 전역 egui `Visuals`를 테마에 연결합니다(위젯 상태별 채움·테두리, `list.activeBackground` 선택, `app.accent` 링크, 입력 배경, warn/error 색, panel·window 채움, `menu.*`, 반경 6/8, 그림자 overlay·dialog). `apply_visuals`가 모든 style에 적용합니다. 글꼴 face는 `APP/ui-fonts.rs`. 전역 글자 크기(`Style::text_styles`)는 egui 기본(Body 12.5)입니다.
- 빠진 것: 공용 토큰 묶음(2.0.3)이 없고, 본문 기본 크기가 원본 13px과 다릅니다.
- 설계: 2.0.3 `UI/ui-tokens.rs`. 결정 7 추천 A: `Style::text_styles`의 Body만 13px로(원본 body 기본), 나머지는 프리미티브가 명시 크기를 씁니다. 원본 `user-select: none`은 egui 기본(라벨 선택 불가)과 같습니다.
- 검사: 앱 presentation 검사(`APP/presentation-refresh.rs` 기존 검사 형태)에 토큰 전 항목·Body 13px 단언, 36개 번들 테마 전부에서 `Tokens::new` 성공.
- 완료 판정: complete.

#### commands-ui-87 global.css 보조 (partial, 의존, 묶음 7)

- 원본 `global.css:321-467`와 native 대응

| 원본 | 대응 | 상태 |
| --- | --- | --- |
| reveal 게이트 `html[data-theme-ready][data-locale-ready] body`(321-323) | 90 | 근거 |
| `shadow-overlay`(0 2px 8px), `shadow-overlay-lg`(0 8px 24px), `modal-scrim`(그림자색 50%) | `UI/presentation.rs` `OVERLAY_SHADOW_*`·`DIALOG_SHADOW_*`, `UI/modal.rs` `Chrome::scrim`(`SCRIM_OPACITY` 0.5) | 있음 |
| `scrollbar-hidden` | `ScrollBarVisibility::AlwaysHidden`(팔레트), 77 | 77 |
| `.taide-gutter-*`, `.taide-blame-overlay-text`, `.taide-conflict-*`, `.taide-diff-gutter-stage` | git-56~63(편집기 gutter·blame·충돌) | git lane |
| `.xterm` 보정, `.monaco-tl-twistie` | 웹 구현 전용 | not-applicable 항목 |
| `[data-sonner-toaster].taide-toaster-middle` | `UI/toast.rs` `Vertical::Middle` | 있음 |

- 이 lane: git lane 병합 뒤 표의 git 항목 근거를 확인하고, scrim·그림자 값 단언(UI 단위 1건)을 더해 행을 닫습니다.
- 완료 판정: git-56~63 complete 후 complete.

#### commands-ui-90 use-reveal-window (partial, 근거, W1-B)

- 원본: `TS/shared/hooks/use-reveal-window.ts:10-33`, `TS/app/providers/theme-provider.tsx:63,98`. 창은 숨긴 채 만들고, 테마·로케일 조회가 끝나면(실패해도) `show()`. CSS 게이트로 본문도 그때 보입니다. 목적: 테마 없는 배경색 깜빡임 방지.
- native 현재: `NativeApplication::new`가 첫 프레임 전에 테마·로케일·설정을 동기로 적재합니다(실패 대체는 shell-94). vendored eframe은 창을 숨긴 채 만들고 첫 프레임을 그린 뒤 보입니다: `experiments/native-shell-spike/vendor/eframe/src/native/wgpu_integration.rs:1113`(`with_visible(false)`), `epi_integration.rs:407-412`(`post_rendering` 첫 프레임 `set_visible(true)`). 보조 창 viewport도 `with_visible(false)`로 만듭니다(`wgpu_integration.rs:1060`).
- 설계: 코드 변경 없음. 검사 1건 추가.
- 검사(App): `launch` 직후 첫 프레임의 `ctx.style().visuals.panel_fill`이 선택 테마의 `app.background`이고 locale이 설정 언어임을 단언(egui 기본 색이 한 번도 그려지지 않음). OS 창 표시 시점은 실기(배치 31).
- 완료 판정: complete.

#### commands-ui-91 use-tauri-event / use-object-url / use-monaco-markers / use-remote-connection-revision (partial, 근거, W1-B)

- 원본: `TS/shared/hooks/use-tauri-event.ts`(Tauri 이벤트 구독 수명), `use-object-url.ts`(Blob URL 생성·해제), `use-monaco-markers.ts`(monaco marker 집계·참조 수 구독), `use-remote-connection-revision.ts`(원격 웹 연결 세대).
- native 대응: 이벤트 → `AppEvent` sink(`PaintSink`, shell-93), Blob URL → 미리보기 texture cache(preview 행들), marker → `APP/diagnostics.rs` `counts()`·문제 패널(editor-39), 원격 연결 → 동결 remote-web.
- 설계: 4절 결정 4 추천 A: 구현 기술 hook이라 not-applicable로 재분류하고 비고에 위 대응 행을 적습니다.

#### commands-ui-96 사용자에게 보이는 비국제화 문자열 (partial, 바로, 묶음 7)

- 원본 기준: 화면 문구는 모두 locale 카탈로그(`CR/taide-locale/resources/locales/{en,ko,ja}.json`). 예외로 원본도 고정 영어인 것: 로케일 로드 실패 배너 문구, monaco hover "Loading...", 브랜드 "TAIDE". 백엔드 오류는 `describeIpcError`(`TS/shared/lib/ipc-error-message.ts:26-29`)라 Localized가 아니면 원문 메시지를 그대로 보입니다.
- native 현재
    - UI 리터럴: 비검사 코드의 화면 리터럴은 `UI/shell.rs` "TAIDE"(브랜드)와 `UI/editor-documentation.rs` "Loading..."(원본 monaco와 같음)뿐입니다(`rg "\.(label|button|heading)\(\s*\"[A-Za-z]"` 결과, 나머지는 검사 코드).
    - 오류 문구: native 전용 오류 약 200곳(`AppError::Forbidden("native … host is disconnected or full")` 등)이 `toast::describe_error`로 원문 영어가 됩니다. 원본에 대응 상황이 있는 것(IPC 실패)은 원본도 원문이지만, native 내부 상태(호스트 큐 가득 참, surface 미연결 등)는 원본에 없는 문구입니다. 또 `status: Option<String>` 상태 표시줄 라벨이 남아 있습니다(settings-78).
- 의존·결정: settings spec 4.1 결정 3(status 라벨 잔여 처리)과 같은 결정을 따릅니다. 추천 A면 native 내부 오류는 화면에 띄우지 않고 `log::warn!`만 남깁니다.
- 설계: 앱 crate 검사 모듈에 문자열 감사 검사를 둡니다. `std::fs`로 `APP`·`UI`의 비검사 `.rs`를 읽어 `.label("`, `.button("`, `.heading("`, `RichText::new("`의 영문 리터럴을 찾고 허용 목록(위 2개)과 비교합니다. 오류 문구는 결정 반영 뒤 `toasts.ipc_error`로 가는 native 전용 오류 생성 지점을 Localized(기존 키) 또는 로그로 바꿉니다.
- 검사: 감사 검사 통과, 대표 경로(호스트 큐 가득 참 → 화면 문구 없음·로그 1건).
- 완료 판정: 결정 반영 + 검사 통과 → complete.

#### commands-ui-98 앱 전반 lucide 아이콘 (partial, 바로(일부), 묶음 7)

- 원본: `from 'lucide-react'` 사용 약 120종(프로젝트 아이콘 56종 포함). 화면별 목록은 아래 표.
- native 현재: lucide SVG를 `APP/resources/{icons, problems, keybindings, snippets, status, themes, project-icons}`에 두고 `UI/glyph-icons.rs` `Glyph`(파일·폴더·프로젝트·탭·문제 등), `UI/icons.rs` `Icon`, `APP/navigation-icons.rs` `Icon`이 래스터화합니다. 탐색기 툴바 4개는 lucide path를 직접 그립니다(`APP/explorer_toolbar.rs` `button`). 고지는 각 폴더 `LICENSE.txt`.
- 아이콘별 대응(native에 SVG·path가 없는 것만, 소유 영역 기준)

| 소유 | 원본 아이콘(사용 화면) | 상태 |
| --- | --- | --- |
| git lane | GitCompare, History, Archive, Tag, Tags, ArrowUp, ArrowDown, Undo2, Minus(stage), RotateCcw(revert), FileDiff(일부) | 그 lane 화면과 함께 |
| explorer-search lane | CaseSensitive, WholeWord, Regex, ReplaceAll, EyeOff, Clock(이력), FileSearch2(=file-search-corner 있음), Scissors·ClipboardPaste·FilePlus·FolderPlus(탐색기 메뉴) | 그 lane 화면과 함께 |
| shell(탭·창 메뉴) | Columns2, AppWindow, ArrowLeftToLine, PanelLeft/Right/Top/Bottom, Pin, PinOff, FolderTree(있음), X(있음) | 메뉴 아이콘 표시 여부 확인 필요(원본 `TS/features/tab/tab-context-menu.tsx` 등) |
| 터미널 메뉴 | ClipboardPaste, Copy(있음), Eraser, SquareDashedMousePointer, SquareTerminal, Trash2(있음) | terminal-agent 영역 |
| 미리보기 | ZoomIn, ZoomOut, ChevronLeft, Music, TableProperties, FileWarning, FileQuestion | preview 영역 |
| settings | Bell, ExternalLink, Import, RefreshCw, Type, CircleX(있음) | settings spec 2.6(4종 SVG 추가 계획) |
| 공용 | Check, ChevronDown, ChevronRight(있음), Search(있음), Loader2(=loader-circle 있음), TriangleAlert(있음), Info(있음) | Check·ChevronDown은 이 lane(71·75의 Picker 선택 표시·trigger) |

- 설계: 이 lane이 직접 더하는 것은 공용 행(Check, ChevronDown)과 72·74 메뉴 모듈이 그리는 하위 메뉴 chevron뿐입니다. 나머지는 각 영역 화면이 생길 때 그 lane이 `APP/resources/icons/`에 lucide SVG(같은 `LICENSE.txt`)를 더합니다. 이 lane은 묶음 7에서 표를 다시 대조하고, `UI/glyph-icons-tests.rs`에 "등록된 모든 SVG가 비지 않은 raster를 만든다" 검사를 `Icon`·`navigation_icons::Icon`까지 넓힙니다.
- 완료 판정: native에 있는 모든 화면의 아이콘이 lucide 형상(SVG 또는 path 이식)이고, 화면이 없는 행만 비고로 남으면 complete. git·검색 화면이 아직이면 partial.

#### commands-ui-101 프로젝트 아이콘 카탈로그 (partial, 근거, W1-B)

- 원본: `TS/shared/icons/project-icon-registry.ts:74-131` `PROJECT_ICON_NAMES` 56종(대응표의 "67종"은 오기), 기본 `folder`, 모르는 id는 `folder`로. 선택 grid는 `TS/features/project/project-display-dialog.tsx:110-136`(8열, cmdk 검색, 선택 링).
- native 현재: `UI/glyph-icons.rs` `PROJECT_ICONS: [(&str, Glyph); 56]`, `project()` 대체 규칙, 검사 `glyph-icons-tests.rs` `프로젝트_아이콘_카탈로그는_원본_순서의_서로_다른_글리프이고_모르는_id는_폴더로_풀린다`(첫·끝 id만 확인), grid 검사 `UI/project-display-dialog-tests.rs` 317.
- 빠진 것: 전체 순서 대조.
- 설계: 같은 검사 파일에 `include_str!("../../../src/shared/icons/project-icon-registry.ts")`로 `PROJECT_ICON_NAMES = [` 와 `] as const` 사이의 따옴표 이름을 읽어 native id 순서와 같은지 단언합니다(선례 `UI/theme-editor-tests.rs` 275의 TS 파일 include).
- 완료 판정: complete. 비고의 "67종"을 "56종"으로 정정합니다.

## 3. 작업 묶음 계획

순서는 의존 순서입니다. 각 묶음은 실패 검사 → 구현 → 영역 검사 → 대응표 행 갱신으로 끝냅니다. 새 외부 의존성은 어느 묶음에도 필요하지 않습니다(결정 5에서 HTML 복사를 고르면 기존 `arboard`의 HTML 쓰기를 쓰므로 새 crate 없음).

| 묶음 | 시기 | ID | 내용 | 수정 예상 파일 | 새 의존성 |
| --- | --- | --- | --- | --- | --- |
| W1-A 명령 진입 | wave 1(선행) | 13, 15, 22, 23, 35, 40, 41 (7) | `Run::{OpenWelcomeTab, MoveFocusedTab(WindowMove)}`와 intent, 팔레트 App 검사 4건, 근거 행 4개 | UI `command-registry.rs`, APP `shell_keymap.rs`, `command-dispatch.rs`(검사 목록), `domains/commands.rs`(검사 연결 1줄), 새 `application-commands-tests.rs`, 대응표 | 없음 |
| W1-B 근거·재분류 | wave 1(선행) | 20, 21, 52, 54, 90, 91, 101 (7) | command-score 단위 검사, 프로젝트 아이콘 drift 검사, 첫 프레임 표시 검사, 재분류 3건(54·91·결정 3의 21) | UI `command-score.rs`, `glyph-icons-tests.rs`, `command-palette-tests.rs`, `application-commands-tests.rs`, 대응표 | 없음 |
| 3 다른 영역 진입 닫기 | wave 3 첫 묶음 | 11, 12, 14, 36, 37, 38, 39, 59, 61, 62 (10) | 병합된 explorer-search·git 실행 경로의 팔레트·키 App 검사, 키맵 41개 전수 단언, 행 갱신 | `command-dispatch.rs`(검사), `application-commands-tests.rs`, 대응표. 빠진 `Run`이 있으면 `command-registry.rs`·`shell_keymap.rs` | 없음 |
| 4 settings 소유 명령 | wave 3 | 30, 31, 32, 33, 34, 85 (6) | `Run::{SyncUpload, SyncDownload, Cli(..)}`, `Command::{Sync, Cli}`, 응답 toast, `toast::Action::SyncPullRemote` | UI `command-registry.rs`, `intents/commands.rs`, APP `shell_keymap.rs`, `domains/commands.rs`, `toast.rs`, `application.rs` `run_toast_actions`(추가만), settings 영역 파일에서 실행 함수 공개(최소), 검사 | 없음 |
| 5 기본 컨트롤 | wave 3 | 68, 69, 70, 79, 76, 84, 77 (7) | `ui-tokens`, `ui-button`, `ui-card`, `ui-checkbox`, `ui-switch`, `ui-progress`, `file-group-header`, `ui-scroll`과 소비 화면 이전 | UI 새 모듈 8개 + `lib.rs`, `snippet-editor.rs`, `settings-view.rs`, `settings-code-view.rs`, `shell.rs`(Welcome), `error-boundary.rs`, `keybinding-editor.rs`, `theme-editor.rs`, `project-*-dialog.rs`. APP `close_dialog.rs`, `delete_dialog.rs`, `open_path_dialog.rs`, `problems.rs`, `presentation-refresh.rs`, 스크롤 소비 화면들, 검색 결과·LSP 진행(병합 상태에 따라) | 없음 |
| 6 겹 위젯 | wave 3 | 67, 73, 75, 71, 72, 74 (6) | `ui-dialog`(alert·dialog), `ui-popover`, `ui-command-list`, `ui-menu`(typeahead 이전 포함)과 소비 화면 이전 | UI 새 모듈 4개, `command-palette.rs`, `settings-code-view.rs`, `theme-color-picker.rs`, `project-display-dialog.rs`, `project-group-dialog.rs`, `snippet-editor.rs`, `theme-editor.rs`, `shell.rs`(메뉴), settings lane `settings-dialogs.rs`. APP `close_dialog.rs`, `delete_dialog.rs`, `open_path_dialog.rs`, `terminal_surface.rs`(메뉴 함수 이동), `explorer.rs`, `breadcrumb-menu.rs`, `system-usage-view.rs` | 없음 |
| 7 전역·재적재·감사 | wave 3 | 1, 82, 83, 86, 87, 96, 98 (7) | 창 재적재, 장애 경계 `reset_viewport`, 근거 갱신 2건, 토큰·Body 13px, 문자열 감사, 아이콘 대조 | UI `error-boundary.rs`, `presentation.rs`, `glyph-icons-tests.rs`, `intents/commands.rs`. APP `domains/commands.rs`, `shell_keymap.rs`, `presentation-refresh.rs`, 오류 생성 지점(결정 반영분), 검사 | 없음 |
| 8 편집기 액션 | wave 3 마지막 | 43 (1, 작업량 큼) | 25개 처리: 구현 8개 이상(zoom 3, in-place 2, tab focus, hide hover, pasteAsText) + 결정 4·5 반영, 기본 키 JSON | 편집기 crate 새 `inplace-replace.rs`·`documentation.rs`, UI `command-registry.rs`, `editor_surface.rs`(Tab focus), `keymap.rs`·새 `editor-commands-keymap-defaults.json`, APP 편집기 appearance 호출부, 검사 | 없음 |

묶음별 주의

1. W1-A·W1-B는 wave 1의 다른 lane과 동시에 돕니다. `command-registry.rs`·`shell_keymap.rs`·`command-dispatch.rs`는 explorer-search lane 묶음 C도 고치므로 match·검사 목록 끝에만 추가합니다. W1 두 묶음은 서로 독립이라 어느 순서든 됩니다.
2. 묶음 3은 wave 1·2 병합 상태를 그대로 확인하는 묶음입니다. 의존 lane이 그 행을 partial로 남겼으면 이 lane도 같은 의존을 비고에 남기고 넘어갑니다.
3. 묶음 5가 6보다 먼저입니다(alert dialog·menu가 button·token을 씀).
4. 묶음 5·6은 다른 영역 화면 파일을 많이 고칩니다. 화면 동작은 바꾸지 않고 그리기 함수만 공용 모듈로 바꾸며, 그 화면의 기존 검사가 모두 통과해야 합니다. 변경 crate 전체 검사(`--no-fail-fast -- --test-threads=4`)를 묶음 끝에 1회 돌립니다.
5. 묶음 8은 1개 ID지만 25개 액션이라 작업량은 다른 묶음과 비슷합니다. editor+preview lane이 wave 1에서 editor-34를 이미 진행했으면 그 결과를 확인하고 남은 것만 합니다.
6. 모든 묶음 공통 게이트: 변경 crate 테스트, crate별 fmt, UI crate를 바꾼 묶음은 remote-web Wasm check, lockfile 무변경.

## 4. 사용자 결정과 허락

### 4.1 결정이 필요한 항목

| # | 결정 지점 | 선택지 | 영향 ID |
| --- | --- | --- | --- |
| 1 | `window.reload`의 native 의미 | **A(추천): 실행한 창의 UI만 다시 적재(일시 UI 닫기, 장애 경계 초기화, 테마·로케일·설정·데이터 재조회, 문서·터미널·LSP·레이아웃 유지, 2.1 설계)** / B: not-applicable(웹 view 다시 읽기 전용) | 1 |
| 2 | 원본에서 "활성으로 보이지만 실행해도 아무 일도 없는" 명령 행 | **A(추천): native는 비활성 행으로 두고 비고에 차이 기록(무동작 재현 안 함)** / B: 원본처럼 활성 행 + 무동작 | 40(편집기 없을 때), 43(goToLocation, hover verbosity 2개) |
| 3 | `app.showPerfSnapshot` | **A(추천): not-applicable(개발자 console 전용 계측, commands-ui-55와 같은 성격)** / B: native perf 게이트일 때 활성, 실행 시 native 계측표를 로그로 출력 | 21 |
| 4 | Monaco quick input 대체(`gotoLine` ⌃G, `quickOutline` ⇧⌘O)와 구현 기술 hook 행 | **A(추천): 기존 native 팔레트를 `:`·`@` 모드로 열기 / hook 행 91은 not-applicable** / B: 편집기 상단 quick input 위젯 이식, 91은 partial 유지 | 43, 91 |
| 5 | 대응표에 행이 없는 편집기 기본 기능(유니코드 문자 강조, URL 링크 감지·열기, 독립 색 선택기, 구문 강조 복사, Paste As) | **A(추천): 편집기 영역에 새 행으로 등록하고 43은 그 의존을 비고에 남긴 채 partial, 구현 배치는 따로 정함** / B: 이 lane 묶음 8에서 함께 구현(묶음 8 규모 약 2배) / C: not-applicable로 기록 | 43 |
| 6 | 공용 위젯 행(67~79, 84)의 완료 기준 | **A(추천): 원본 소비자가 쓰는 variant·state만, 기존 native 소비 화면 전부 이전(2.0.4)** / B: 원본 컴포넌트의 모든 variant(미사용 포함) | 67~79, 84 |
| 7 | 전역 글자 크기 | **A(추천): Body만 원본 본문 13px으로, 나머지는 프리미티브가 명시** / B: 현행 egui 기본 유지 | 86 |
| 8 | `cli.installShellCommand`가 관리자 프롬프트 취소에도 성공 문구를 보이는 원본 동작 | **A(추천): 원본대로(설치 호출이 오류 없이 끝나면 성공 문구, Settings CLI 행과 같은 규칙)** / B: 결과 상태를 확인해 미설치면 실패 문구(connect 명령과 같은 규칙) | 33 |
| 9 | 운영 문서 3절이 이 lane에 준 "앞 wave에서 진입만 남은 행"(shell spec 묶음 3: shell-21·59·78·89의 남은 부분 등) | **A(추천): wave 3 시작 전에 메인이 병합 상태 기준으로 그 행 목록을 확정해 이 문서에 보충 절로 추가** / B: 이 lane 묶음 3에 그대로 합침(묶음 3이 10개를 넘음) | 범위 밖 행 |

모두 추천안이면 "전부 추천안대로"로 답하시면 됩니다. W1 묶음에 걸리는 것은 2(40)와 3(21)뿐이고, 나머지는 wave 3 착수 전에 답이 있으면 됩니다.

### 4.2 허락이 필요한 항목(자동 검사에서 하지 않는 것)

1. 실제 GitHub gist 연결·업로드·다운로드와 Keychain의 PAT(30, 31).
2. 실제 CLI 설치·해제: `/usr/local/bin/taide` 쓰기와 관리자 암호 프롬프트. 설치된 Tauri 앱의 symlink를 바꿀 수 있습니다(32~34).
3. 실제 저장소에서 `git.revertHead` 실행(새 commit 생성). 자동 검사는 임시 저장소만 씁니다(36).
4. 실제 클립보드 읽기·쓰기(43의 pasteAsText, 결정 5 B의 구문 강조 복사).
5. 실기: OS 창 표시 시점(90), 보조 창 OS focus(22, 23), 메뉴·대화상자 키보드·VoiceOver(67~75), 픽셀 대조(배치 31).

### 4.3 범위 밖 관찰(대상 ID 아님, 보고만)

- commands-ui-10 `editor.find`: 원본은 `isEnabled: alwaysDisabled`(`command-catalog.ts:71-78`, 팔레트 행 비활성, ⌘F 키만 동작)인데 native는 `enablement`가 native-host에서 `ActiveEditor`라 편집기가 있으면 팔레트 행이 활성입니다. 이미 complete 행이라 이 lane은 고치지 않습니다. 필요하면 별도 지시를 주십시오.
