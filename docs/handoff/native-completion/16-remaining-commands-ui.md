# 16 — commands-ui 영역 남은 작업 인계

> 작성일: 2026-11-04
> 기준 코드: 메인 작업 트리 `/Users/hyunseokbyun/development/TAIDE`(브랜치 `to_rust_native`, HEAD `0c52b438`, settings lane 병합 뒤, 코드 변경 없음). Git 영역은 worktree `/Users/hyunseokbyun/development/TAIDE-lanes/git`(브랜치 `lane/git`, HEAD `50892b59`)를 읽었습니다. 이 worktree는 `to_rust_native`(`0c52b438`)를 병합하는 중입니다(`.git/worktrees/git/MERGE_HEAD` = `0c52b438`). `git diff --name-only --diff-filter=U`가 8개 파일(기능표, `application-explorer-views-tests.rs`, `command-dispatch.rs`, `domains/explorer_search.rs`, `explorer.rs`, `navigation-icons.rs`, `shell_keymap.rs`, UI `command-registry.rs`)을 미병합으로 보이지만, 2026-11-04 기준 모든 파일에서 충돌 표식(`<<<<<<<`·`=======`·`>>>>>>>`)은 이미 지워졌고 스테이징만 남았습니다. 아래 `GITWT/` 줄 번호는 그 병합 중 작업 트리의 값이라 병합 커밋 뒤 조금 바뀔 수 있습니다. 반드시 함께 적은 심볼 이름으로 다시 찾습니다.
> 대상: 기능표 `docs/quality-assurance/2026-10-09-native-function-matrix.md`의 commands-ui 행 중 상태가 complete·not-applicable·frozen이 아닌 35행. 두 트리(메인, git worktree)의 commands-ui 행 상태는 같습니다(두 기능표에서 ID·상태만 뽑아 `diff`한 결과 차이 없음).
> 이 문서는 저장소를 처음 보는 작업자가 이 문서만 읽고 같은 품질로 구현하도록 쓴 인계서입니다.

## 0. 읽는 법

### 0.1 경로 약어

| 약어 | 실제 경로 |
| --- | --- |
| `TS/` | `/Users/hyunseokbyun/development/TAIDE/src/` (원본 React 앱, 동작의 기준) |
| `UI/` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src/` |
| `APP/` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src/` |
| `ED/` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-editor/src/` |
| `CR/` | `/Users/hyunseokbyun/development/TAIDE/crates/` |
| `RES/` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/resources/` (아이콘 SVG. `APP/resources`가 아닙니다) |
| `MON/` | `/Users/hyunseokbyun/development/TAIDE/node_modules/monaco-editor/esm/vs/editor/` (원본 편집기 동작 근거) |
| `GITWT/` | `/Users/hyunseokbyun/development/TAIDE-lanes/git/` (git lane worktree, 같은 하위 구조) |
| `QA/` | `/Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/` |

### 0.2 기준 문서

| 문서 | 용도 |
| --- | --- |
| `docs/research/2026-10-28-lane-commands-ui-spec.md` | commands-ui lane 사양(묶음 W1-A·W1-B·3~8). 설계 대부분의 출처. 실제 코드와 다른 점은 이 문서가 우선합니다(각 행 (c)에 적음) |
| `docs/research/2026-10-28-native-lane-seams.md` | 영역 모듈 구조, 공용 파일 영역 블록 규칙(1.4절), 영역 간 상태 접근(1.5절) |
| `docs/memory/2026-10-28-native-lane-operations.md` | lane 운영, wave 순서(commands-ui는 wave 3) |
| `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` | 사양 결정 채택(추천안 전부 + 예외 1건), 허락 보류 목록 |
| `docs/utils/2026-11-03-native-merge-gate.sh` | 병합 게이트 명령 |
| `docs/utils/2026-11-02-native-lane-workflow.js` | lane 작업자 규칙(셸·cargo·금지 사항) |
| `QA/2026-11-02-native-lane-terminal-agent-commands-ui-entry.md`, `…-commands-ui-evidence.md` | commands-ui W1-A·W1-B를 terminal-agent lane이 처리한 기록. 이 영역 검사 파일의 첫 사용 예 |

### 0.3 이 영역에 걸린 사용자 결정(전부 채택됨)

출처: 사양 4.1절, `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` "7개 사양의 4.1절 결정은 아래 예외를 빼고 모두 추천안을 채택".

| 결정 | 채택 내용 | 걸린 행 |
| --- | --- | --- |
| 1 | `window.reload` = 실행한 창의 UI만 다시 적재(일시 UI 닫기, 장애 경계 초기화, 표시·데이터 재조회, 문서·터미널·LSP·레이아웃 유지) | 1 |
| 2 | 원본에서 "활성 행이지만 실행해도 무동작"인 명령은 native에서 비활성 행으로 두고 비고에 차이를 적음 | 43(`goToLocation`, hover verbosity 2개) |
| 4 | Monaco quick input(`gotoLine`·`quickOutline`)은 기존 팔레트를 `:`·`@` 모드로 엶 | 43 |
| 5 | 대응표에 행이 없는 편집기 기능(Unicode 강조, URL 링크 감지, 독립 색 선택기, 구문 강조 복사, Paste As)은 편집기 영역 새 행으로 등록하고 43은 그 의존을 비고에 남긴 채 partial | 43 |
| 6 | 공용 위젯 행 완료 기준 = 원본 소비자가 실제로 쓰는 variant·state만 + native에 있는 소비 화면 전부 이전 | 67~79, 84 |
| 7 | 전역 글자 크기: egui `TextStyle::Body`만 원본 본문 13px | 86 |
| 8 | **예외로 뒤집힘**: CLI 설치의 관리자 대화상자 취소는 성공 문구를 보이지 않음(terminal-agent 결정 7 추천을 따름) | 33 |
| editor-preview 사양 결정 2 A | Unicode 강조·텍스트 링크 감지를 editor 새 행으로 추가하고 editor 영역이 구현 | 43 |

주의: 결정 5(commands-ui)와 editor-preview 결정 2 A는 같은 기능을 다룹니다. 구현 소유는 editor 영역입니다. 2026-11-04 현재 그 새 행은 기능표에 없습니다(editor 행은 editor-87이 마지막, `grep -n "| editor-88 " QA/2026-10-09-native-function-matrix.md` 결과 없음).

### 0.4 complete의 뜻

기능표 머리말과 사양 0절: complete = 원본 TS 화면·상태·상호작용 대응 구현 + 실제 앱 경로(`main.rs` → `NativeApplication` → 해당 경로) + 그 동작의 자동 검사. 픽셀 대조·실제 OS 입력·VoiceOver 같은 실기(배치 31)는 포함하지 않습니다. 원본 결함은 재현하지 않습니다.

---

## 1. 요약

### 1.1 남은 행 수

기능표 행 단위 집계(2026-11-04, 634행 `## commands-ui` 아래 표에서 `awk -F'|'`로 상태 열을 센 값): commands-ui 102행 = complete 63, partial 21, unwired 11, missing 3, not-applicable 4.

- **이 문서 대상 35행**
    - missing 3: 1, 82, 83
    - unwired 11: 14, 30, 31, 32, 33, 34, 36, 37, 38, 39, 62
    - partial 21: 43, 59, 61, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 79, 84, 85, 86, 87, 96, 98
- 사양(2026-10-28)의 대상 51행 중 16행은 이미 닫혔습니다: 11·12(explorer-search lane), 13·15·22·23·35·40·41(W1-A), 20·52·90·101(W1-B)는 complete, 21·54·91은 not-applicable.
- 기능표 상단 합계 표(20행 `| commands-ui | 102 | 50 | 25 | 22 | 4 | 1 | 0 |`)는 갱신되지 않은 옛 값입니다. lane 규칙상 상단 합계 표는 건드리지 않으므로 행 단위 값을 기준으로 씁니다.
- 기능표 비고의 많은 문구("레지스트리·공유 backend/계약 또는 모드 껍데기는 있으나…", "구현한 native 위젯/모달·메뉴·toast는 있으나…", "reload 실행·MRU·구역별 장애/상단 재시도 배너의 실제 native 사용자 경로는 없습니다")는 2026-10-06 감사 때의 공통 문구라 실제 남은 범위가 아닙니다. 실제 남은 범위는 3절 각 행 (c)입니다.

### 1.2 행 목록과 현재 분류

분류(2026-11-04 실제 코드 확인 결과):
- **근거**: 실행 경로·검사가 다른 영역 작업으로 이미 들어왔고 이 영역은 행 갱신(필요하면 검사 1건)만 남음.
- **git 병합 뒤**: git lane worktree에 실행 경로가 있고 `to_rust_native`에 병합되면 이 영역이 실제 팔레트 검사를 더해 닫음.
- **지금 구현**: 선행이 모두 끝나 지금 구현 가능.
- **의존 남음**: 이 영역 밖의 미완료 행이 있어 그 행이 끝나야 complete.

| ID | 기능 | 표 상태 | 분류 | 선행(상태) | 사양 묶음 | 이 문서 권장 순서 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `window.reload` | missing | 지금 구현 | 없음 | 7 | 7 |
| 14 | `view.git` | unwired | git 병합 뒤 | git-1(git lane complete) | 3 | 3 |
| 30 | `sync.uploadNow` | unwired | 지금 구현 | settings-46·47(complete, 병합됨) | 4 | 4 |
| 31 | `sync.downloadNow` | unwired | 지금 구현 | settings-46·47 | 4 | 4 |
| 32 | `cli.connectExternalEditor` | unwired | 근거 | terminal-agent-56 명령 부분(병합됨) | 4 | 0 |
| 33 | `cli.installShellCommand` | unwired | 근거 | 같음 | 4 | 0 |
| 34 | `cli.uninstallShellCommand` | unwired | 근거 | 같음 | 4 | 0 |
| 36 | `git.revertHead` | unwired | git 병합 뒤 | git-36(git lane complete) | 3 | 3 |
| 37 | `git.createTagOnHead` | unwired | git 병합 뒤 | git-36, git-1 | 3 | 3 |
| 38 | `git.toggleBlame` | unwired | git 병합 뒤 | git-57(git lane complete) | 3 | 3 |
| 39 | `git.openFileHistory` | unwired | git 병합 뒤 | git-55(git lane complete) | 3 | 3 |
| 43 | `monaco.*` 액션 실행 | partial | 지금 구현(일부) + 의존 남음 | editor 새 행(미등록), editor-57(partial), git-55·57·60 | 8 | 8 |
| 59 | 키맵 디스패치 대상 41개 | partial | git 병합 뒤 | git-1 | 3 | 3 |
| 61 | find/search/search-replace 키 | partial | 근거 | explorer-search-37(complete, 병합됨) | 3 | 0 |
| 62 | explorer/git 키 | unwired | git 병합 뒤 | git-1 | 3 | 3 |
| 67 | alert-dialog | partial | 지금 구현 | 묶음 5(button·tokens) | 6 | 6 |
| 68 | button | partial | 지금 구현 | ui-tokens | 5 | 5 |
| 69 | card | partial | 지금 구현 | ui-tokens | 5 | 5 |
| 70 | checkbox | partial | 지금 구현 | 소비 화면 2곳 병합됨 | 5 | 5 |
| 71 | command(cmdk 목록) | partial | 지금 구현 | 묶음 5 | 6 | 6 |
| 72 | context-menu | partial | 지금 구현 | 묶음 5 | 6 | 6 |
| 73 | dialog | partial | 지금 구현 | 묶음 5 | 6 | 6 |
| 74 | dropdown-menu | partial | 지금 구현 | 72 모듈 | 6 | 6 |
| 75 | popover | partial | 지금 구현 | 묶음 5 | 6 | 6 |
| 76 | progress | partial | 지금 구현 | settings-33(complete, 병합됨) | 5 | 5 |
| 77 | overlay scrollbar | partial | 지금 구현 | 없음 | 5 | 5 |
| 79 | switch | partial | 지금 구현 | 없음 | 5 | 5 |
| 82 | error-boundary | missing | 의존 남음(지금은 partial로 정정) | shell-12(partial) → wave 4 편집기·루트 경계 | 7 | 0(정정) + wave 4 |
| 83 | status-error-banner | missing | 근거 | shell-94(complete, 병합됨) | 7 | 0 |
| 84 | file-group-header | partial | 지금 구현 | 소비 화면 2곳 병합됨 | 5 | 5 |
| 85 | toast | partial | 지금 구현 | 30·31과 함께 | 4 | 4 |
| 86 | 테마 토큰 → 전역 스타일 | partial | 지금 구현 | 없음 | 7 | 5(토큰) + 7(Body 13px) |
| 87 | global.css 보조 | partial | git 병합 뒤 | git-56~63(git lane complete) | 7 | 7 |
| 96 | 비국제화 문자열 | partial | 지금 구현(일부) + 의존 남음 | 다른 영역 `status` 대입 0건(메인 통합) | 7 | 7 |
| 98 | lucide 아이콘 | partial | 지금 구현(일부) + 의존 남음 | git·preview·terminal 화면 아이콘 | 7 | 7 |

분류별 수: 근거 5(32, 33, 34, 61, 83) + 정정 1(82), git 병합 뒤 8(14, 36, 37, 38, 39, 59, 62, 87), 지금 구현 18(1, 30, 31, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 79, 84, 85, 86), 일부 지금 구현 + 의존 남음 3(43, 96, 98).

### 1.3 현재 구현 위치

| 역할 | crate·파일 | 핵심 심볼(메인 트리 줄) |
| --- | --- | --- |
| 명령 카탈로그(212개, `monaco.*` 157개) | `UI/keybinding-commands.json` | `"id"`, `"titleKey"`, `"categoryKey"`, `"keymapId"`, `"platform"` |
| 명령 등록·실행 경로·활성 조건 | `UI/command-registry.rs` | `EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE`(25, 21개), `PaletteEntry`(145), `Run`(152~198), `WindowMove`(202), `Execution`(214), `Enablement`(220), `CommandContext`(239), `Command::runnable`(293), `enablement()`(331), `execution()`(360), 마지막 `let run = match id {`(455), `line_command`(504), `cursor_command`(544), `fold_command`(573), `keymap_run`(601), `registry()`(812) |
| 기본 키맵(41개) | `UI/keymap-defaults.json`, `UI/keymap.rs` | `DEFAULTS`(11), `EDITOR_DEFAULTS`(12), 편집기 키 JSON `include_str!`(463~584), `Decision::Dispatch(id)` |
| 팔레트 | `UI/command-palette.rs`, `UI/command-palette-query.rs` | `Palette::{open(518), dismiss(501), is_open(389), inspection(510)}`, `Action::RunCommand(id)`, `navigation()`(1803), `entry_query`(query 81), `COMMAND/SYMBOL/LINE/WORKSPACE_SYMBOL_MODE_PREFIX`(query 4~7) |
| 팔레트 파일 색인 | `APP/command-palette.rs` | `FileIndexes::{observe, accept, invalidate(137), cancel_fetches, view}` |
| 명령 → intent | `APP/command-dispatch.rs` | `accepts`(62), `intent`(73), `intent_with_editor`(90) |
| `Run` → `ShellIntent` | `APP/shell_keymap.rs` | `supports`(61), `runs_without_focused_project`(65), `intent`(89) |
| commands 영역 intent | `UI/intents/commands.rs` | `Intent::{OpenPalette, OpenKeybindings, OpenTaskRunner}` |
| commands 영역 앱 처리 | `APP/domains/commands.rs` | `Command`(17), `Reply`(32), `State`(47), `dispatch`(56), `apply_commands_reply`(122), `apply_commands_intent`(147), `show_commands_overlays`(192), `show_palette`(241), 검사 연결(649~651) |
| CLI 명령(terminal-agent 영역 소유) | `APP/agent-cli.rs`, `APP/domains/terminal_agent.rs`, `UI/intents/terminal_agent.rs` | `Run::Cli(CliAction)`, `Intent::Cli`, `Command::Cli`(348), `finish_agent_cli`(467) |
| toast | `UI/toast.rs`, `APP/toast.rs`, `APP/toast-layers.rs` | UI `Kind`(74), `Action<A>`(96), `Options<A>`(101), `Toasts`(326), `Inspection`(349), `notify`(488), `warning/success/error/info`(515~527), `take_actions`(537), `ipc_error`(587), `describe_error`(1589); 앱 `toast::Action`(14), `ToastLayers`(창별 toaster) |
| modal 기반 | `UI/modal.rs` | `FOCUS_FILTER`(9), `SCRIM_OPACITY`(15), `Chrome`(24)·`Chrome::scrim`(44), `Transition`(50), `Composition`(118), `FocusReturn`(145), `Layer`(179), `unmount`(255), `takes_escape`(268), `trap_focus`(275) |
| 장애 경계 | `UI/error-boundary.rs` | `STATE_KEY`(11), `INJECTION_KEY`(32), `Region`(42), `show`(94), `show_instance`(107), `inject_panic`(356), `inject_instance_panic`(361) |
| 상단 오류 배너 | `APP/status-error-banner.rs` | `LOCALE_MESSAGE`(17), `startup`(53), `show`(104) |
| 테마 → egui visuals | `UI/presentation.rs`, `APP/presentation-refresh.rs` | `color`(59), `visuals`(143), `apply_visuals`(227), `Refresh::retry`(169), `Appearances::new`(230) |
| 아이콘 | `UI/icons.rs`, `UI/glyph-icons.rs`, `APP/navigation-icons.rs`, `RES/` | UI `Icon`(19), `Glyph`, 앱 `Icon`(13) |

현재 공용 위젯 모듈(`ui-button.rs`, `ui-dialog.rs` 등)은 하나도 없습니다(`ls UI/ | grep "^ui-"` 결과 없음). 같은 그림이 화면마다 따로 있습니다(3.5·3.6절).

### 1.4 다른 영역과의 의존·소유 경계

- **소유 원칙**(`docs/memory/2026-10-28-native-lane-operations.md` 2.2, seams 1.4): 이 영역은 `APP/domains/commands.rs`, `UI/intents/commands.rs`, 새 공용 UI 모듈(`UI/ui-*.rs`), `UI/toast.rs`·`UI/modal.rs`·`UI/error-boundary.rs`·`UI/presentation.rs`(commands-ui 근거 묶음 파일), `APP/application-commands-tests.rs`, commands QA만 고칩니다. 공용 파일(`APP/application.rs`, `APP/lib.rs`·`UI/lib.rs` 모듈 목록, `HostCommand`·`ShellIntent` 루트, locale 카탈로그)은 영역 블록 끝에 추가만 합니다. 공용 위젯 이전(3.5·3.6절)은 다른 영역 화면 파일을 고치므로 "그리기 함수만 교체, 동작 불변, 그 화면 기존 검사 전부 통과"가 조건입니다(사양 3절 주의 4).
- **git**: 14, 36~39, 59(git 부분), 62, 87(gutter·blame·충돌 클래스), 43의 `monaco.taide.{gitStageSelection, toggleBlame, openFileHistory}` 3개, 98의 git 화면 아이콘, 67·71~75의 git 소비 화면. git lane QA가 "commands-ui-14·62는 commands-ui lane이 행을 닫는다"(`GITWT/docs/quality-assurance/2026-11-02-native-lane-git-g1-base.md` 57행), "commands-ui-36~39·87은 wave 3 commands-ui lane이 이 근거로 닫는다(행 미변경)"(`…-g8-blame-conflicts.md` 102행)고 적었습니다.
- **settings**: 30·31의 동기화 실행 함수(`APP/domains/settings.rs` `execute_sections`의 `SyncUpload`·`SyncDownload` arm, `download_sync`), 76의 소비 화면(`UI/settings-sections-view.rs` `progress_line`), 70의 VSIX 후보 checkbox, 67·73의 `UI/settings-dialogs.rs`, 79의 스위치, 96의 status 라벨 결정(settings-78).
- **explorer-search**: 61(완료), 59의 `search`·`search-replace`(완료), 84·70의 검색 결과 그룹 머리(`APP/search-results.rs`), 74의 검색 이력 dropdown, 67의 바꾸기 확인(`APP/search-panel.rs` 1374).
- **terminal-agent**: 32~34 명령 구현 전부(이 영역은 행만 갱신), 72의 터미널 메뉴 typeahead(`APP/terminal_surface.rs`).
- **shell**: 83(shell-94 complete), 82(shell-12 partial), 1의 표시 재적재가 쓰는 배너, 1의 Welcome 최근 목록(`APP/welcome.rs`).
- **editor**: 43 전체(editor-34와 같은 기능), 43의 결정 5 항목은 editor 새 행.
- **wave 4(메인)**: 82의 편집기 영역·루트 장애 경계, 96의 다른 영역 `status` 대입 정리.

### 1.5 권장 구현 순서

순서는 의존 순서입니다. 각 묶음은 "실패 검사 → 구현 → 영역 검사 → 기능표 행 갱신 → QA 문서"로 끝냅니다. 사양 묶음 번호(3~8)를 그대로 쓰고, 근거만 남은 행은 "묶음 0"으로 앞에 모았습니다.

1. **묶음 0(근거 정정, 지금)**: 32, 33, 34(사양 묶음 4), 61(사양 묶음 3), 83(사양 묶음 7)을 기존 검사 근거로 complete, 82를 partial로 정정. 코드 변경 없음. 다른 묶음과 파일이 겹치지 않아 먼저 끝냅니다.
2. **묶음 3(git 병합 직후)**: 14, 36, 37, 38, 39, 62, 59. git lane이 `to_rust_native`에 병합된 HEAD에서 시작합니다. 87의 git 항목 확인도 여기서 할 수 있습니다. 사양 묶음 3의 11·12는 이미 complete입니다.
3. **묶음 4(지금)**: 30, 31, 85.
4. **묶음 5(지금)**: `UI/ui-tokens.rs`(86의 토큰 부분)를 먼저 만들고 68, 69, 79, 70, 76, 84, 77.
5. **묶음 6(묶음 5 뒤)**: 67, 73, 75, 71, 72, 74. alert·menu가 button·token을 쓰므로 5가 먼저입니다.
6. **묶음 7**: 1, 86(Body 13px), 87(scrim 단언), 96, 98. 82는 wave 4 편집기·루트 경계가 들어오면 complete.
7. **묶음 8(마지막, 작업량 큼)**: 43.

묶음 3은 git 병합을 기다리지만 4·5·6·7·8은 기다리지 않습니다. git 병합 전에 5·6을 하면 git 소비 화면(대화상자·메뉴)은 (c) 비고로 남기고, 병합 뒤 묶음 3에서 함께 확인합니다.

### 1.6 선행 행이 끝나면 할 수 있는 행

| 이 영역 행 | 선행 행 | 선행 상태(2026-11-04) | 선행이 끝나면 할 일 |
| --- | --- | --- | --- |
| 14, 62 | git-1 | git lane complete, 병합 중 | 실제 팔레트·⌃⇧G App 검사 추가, 행 갱신 |
| 36 | git-36 | git lane complete | 실제 팔레트 App 검사 추가, 행 갱신 |
| 37 | git-36, git-1 | git lane complete | 같음 |
| 38 | git-57 | git lane complete | 같음 |
| 39 | git-55 | git lane complete | 같음 |
| 59 | git-1(`git` 키), explorer-search-37(`search`·`search-replace`) | git lane complete / 병합됨 | `keymap-defaults.json` 41개 전수 단언 |
| 87 | git-56~63 | git lane complete | CSS 클래스 대응 표 근거 확인 + scrim 단언 |
| 43(`monaco.taide.*` 3개) | git-55, git-57, git-60 | git lane complete | 실행 확인(병합으로 이미 `Run::Git`) |
| 43(결정 5 항목 8개) | editor 새 행(Unicode 강조 4개, 링크 1개, 구문 강조 복사 1개, pasteAs 1개), editor-57 색 선택기 1개 | 새 행 미등록 / editor-57 partial | 그 행이 complete면 실행 연결 |
| 30, 31, 85 | settings-46, settings-47 | complete, 병합됨 | 지금 구현 |
| 32, 33, 34 | terminal-agent-56(명령 부분), settings-16 | 명령 부분 완료·병합됨 / complete | 지금 행 갱신 |
| 61 | explorer-search-37 | complete, 병합됨 | 지금 행 갱신 |
| 76 | settings-33 | complete, 병합됨 | 지금 구현 |
| 70, 84 | 검색 결과 그룹 머리(`APP/search-results.rs`), VSIX 후보 목록(`UI/settings-sections-view.rs` `checkbox`) | 병합됨 | 지금 구현 |
| 83 | shell-94 | complete, 병합됨 | 지금 행 갱신 |
| 82 | shell-12 | partial(편집기 영역·루트 경계 남음) | wave 4 뒤 complete |
| 96 | settings-78 결정 3 A와 각 영역 `status` 대입 정리 | settings 영역 0건, 나머지 메인 통합 대기 | 0건이 되면 complete |
| 67·71·72·73·74·75의 (c) | git 대화상자·메뉴·branch switcher | git lane 구현, 병합 중 | 병합 뒤 그 화면이 공용 모듈을 쓰는지 확인(안 쓰면 그리기 함수만 교체) |
| 98(일부) | git·preview·terminal 화면의 lucide 아이콘 | git lane이 SVG 11개 추가(병합 중), preview·terminal 일부 미추가 | 그 화면 소유 영역이 SVG 추가 뒤 대조 |

---

## 2. 영역 공통 지식

### 2.1 빌드 환경 기초(처음 보는 작업자용)

- 언어: Rust 2024 edition, `rust-version = "1.95"`(`native/taide-native-ui/Cargo.toml` 4~5행, 앱 crate도 같음). let-chain(`if let Some(x) = a && cond {}`)을 씁니다(`APP/domains/commands.rs` 263~269 예).
- UI 라이브러리: egui `=0.36.2`. 저장소는 crates.io egui 대신 fork를 씁니다: 두 crate 모두 `[patch.crates-io] egui = { path = "…/taide-native-app/vendor/egui-input" }`(`native/taide-native-ui/Cargo.toml` 53~54, 앱 `Cargo.toml` 112). eframe도 `experiments/native-shell-spike/vendor/eframe`의 fork입니다(앱 `Cargo.toml` 56). 그래서 egui 공식 문서에 없는 API가 있습니다. 예: `Context::keyboard_focus_before_events`, `Context::raw_event_index`, `Context::keyboard_focus_request_at`(터미널 메뉴 typeahead가 씀, `APP/terminal_surface.rs` 1378~1500). API를 쓰기 전에 `native/taide-native-app/vendor/egui-input/src/`에서 시그니처를 확인합니다. 예: 팝업 닫기 `egui::Popup::close_all(ctx)`(`vendor/egui-input/src/containers/popup.rs` 701), 팝업 상태는 viewport별(`vendor/egui-input/src/memory/mod.rs` 118 `popups: ViewportIdMap<OpenPopup>`).
- 각 native crate는 독립 workspace이고 각자 `Cargo.lock`이 있습니다. 새 의존성은 이 영역 어느 묶음에도 필요 없습니다(사양 3절). `serde_json`은 두 crate 모두 이미 의존합니다.
- feature: UI crate `default = ["native-host"]`, `inspection`(검사용 조회 API, `UI/Cargo.toml` 11~14). 앱 crate의 dev-dependency가 UI를 `features = ["inspection"]`로 켜므로(앱 `Cargo.toml` 106) 앱 검사에서 `palette.inspection()`·`toasts.inspection()`을 쓸 수 있습니다. 동결 crate `native/taide-remote-web`은 UI crate를 `default-features = false`로 씁니다. 그래서 native 전용 변형은 `#[cfg(feature = "native-host")]`로 막아야 remote-web의 컴파일 표면이 바뀌지 않습니다(`UI/command-registry.rs` `Run::OpenTaskRunner`·`Cli`·`OpenWelcomeTab` 선례). git lane은 `Run::ShowGit`·`Run::Git(GitRun)`을 cfg 없이 넣었습니다(`GITWT/native/taide-native-ui/src/command-registry.rs` 172~173). UI crate를 바꾸면 remote-web Wasm check를 반드시 돌립니다.
- 앱 crate의 `APP/command-registry.rs`는 `pub use taide_native_ui::command_registry::*;`, `APP/modal.rs`는 `pub use taide_native_ui::modal::*;`입니다. 반면 `APP/keymap.rs`·`APP/keybinding-editor.rs`·`APP/tooltips.rs` 등은 `cfg(test)`에서 `include!("../../taide-native-ui/src/<파일>.rs")`로 UI 소스를 복제 컴파일합니다(`APP/tooltips.rs` 1~14 예). 이 파일들의 앱 검사는 배포 코드와 다른 컴파일 단위를 검증한다는 점을 기억합니다.
- 한 프레임 흐름(`docs/architecture.md` 9.2): `background_tick`(호스트 응답 수신) → 키맵 → `shell.show` → 오버레이(팔레트·키바인딩 편집기·대화상자·toast) → `ShellMutation`·`HostCommand` 제출. 호스트는 worker 1개 직렬, 큐 용량 64(`APP/host.rs` 10 `HOST_COMMAND_CAPACITY`).

### 2.2 명령 실행 흐름(실제 심볼)

팔레트 경로:
1. `UI/command-palette.rs`가 행 선택 시 `Action::RunCommand(id)`를 냅니다.
2. `APP/domains/commands.rs` `show_palette`(393~509)가 편집기 전용 몇 개(`ai.inlineEdit`·inline 완성, `monaco.editor.action.showContextMenu`, smartSelect 2개, `monaco.codelens.showLensesInCurrentLine`, code action, rename, 포맷)를 직접 처리하고 나머지는 `self.commands_state.palette_commands.push(id)`(507)합니다.
3. `APP/application.rs` 2107~2118: overlay viewport의 pass에서만(`if context.viewport_id() == frame_pass.overlay`) `keymap_actions.append(&mut self.commands_state.palette_commands)` 뒤 `crate::command_dispatch::intent_with_editor(&action, &command_context, &snapshot, focused_view.as_ref(), keymap_editor_scope.is_some() || explorer_output.is_editor_widget_focused, &self.shell.scope)`.

키 경로: `taide_native_ui::keymap`이 `Decision::Dispatch(id)`(사용자 override 포함)를 내면 같은 `keymap_actions`에 쌓여 3과 합류합니다.

공통 이후:
4. `APP/command-dispatch.rs` `intent_with_editor`(90~110): `find`이고 편집기 focus가 없으면 `search`로 대체. `Run::EditDocument(edit)`이면 `ShellIntent::Editor(Intent::EditDocument{tab, edit})`. 그 밖은 `intent`(73~86) → `command_run(id, context)`(registry `Command::runnable`) → `crate::shell_keymap::intent(run, snapshot, scope)`.
5. `APP/shell_keymap.rs` `intent`(89~): 첫 match(98~151)는 프로젝트가 필요 없는 실행, `let (root, focused) = crate::symbol_sidebar::window_tree(project, layout, scope)?;`(154) 뒤 두 번째 match(155~232)는 focused pane이 필요한 실행, 그 뒤(233~)는 focused leaf의 탭이 필요한 실행입니다. git 병합 뒤 줄은 각각 98~157, 160, 161~291로 늘어납니다(`GITWT/…/shell_keymap.rs`).
6. `APP/application.rs` 2203~2218 intent 루프: `ShellIntent::Commands(intent) => self.apply_commands_intent(intent, &context)`, `ShellIntent::TerminalAgent(intent) => self.apply_terminal_agent_intent(intent)`, `ShellIntent::Git(intent) => self.apply_git_intent(intent)`(git 병합 뒤 `(intent, &context, &snapshot)`), `ShellIntent::Settings(intent) => self.apply_settings_intent(intent, &snapshot)`, 나머지 shell 변형은 `apply_shell_intent`.
7. 호스트가 필요하면 `self.submit(HostCommand::Commands(Command::X))` → `APP/host.rs` 330~331 `HostCommand::Commands(command) => crate::application::domains::commands::dispatch(worker, command).await` → `HostReply::Commands(Reply::Y)` → 다음 프레임 `poll()`(`APP/application.rs` 749, 758 `HostReply::Commands(reply) => self.apply_commands_reply(reply)`).
8. 호스트 오류 표시: 원본 `runCommandSafely`(`TS/shared/lib/command-registry.ts` 106~114)는 `toast.error(describeIpcError(error))`입니다. native 대응은 `self.toasts.ipc_error(&self.locale, &error, Instant::now())`(`APP/domains/commands.rs` 141 선례).

### 2.3 상태 위치

| 상태 | 위치 |
| --- | --- |
| 팔레트·키바인딩 편집기·파일 색인·팔레트 명령 큐·task runner 세대 | `NativeApplication.commands_state`(`APP/domains/commands.rs` `State` 47~54, 초기값 `APP/application.rs` 649~) |
| toast(창별) | `NativeApplication.toasts: ToastLayers`(`APP/toast-layers.rs`). `Deref`가 그 프레임 시작 때 OS focus를 가진 창의 toaster를 가리킵니다. 메인 창 전용은 `ToastLayers::main()`(29), 보조 창은 `auxiliary(viewport)`(34), 창 단위 제거 `retain`(47) |
| Settings section 데이터·포트 | `settings_state.settings_views.sections_data_mut()`, `settings_state.sections_ports: Arc<settings_ports::Ports>`(`APP/domains/settings.rs` 102, 제출 시 1491) |
| 동기화 상태 사건 기록 | `Events::record`(`APP/domains/settings.rs` 132, `AppEvent::SyncStateChanged` arm) |
| 표시(테마·로케일) 재적재 상태 | `settings_state.presentation_refresh: Refresh`(`APP/application.rs` 2040의 `.retry()` 호출) |
| CLI 설치 상태(명령 경로) | `terminal_agent_state.cli_status`(`APP/domains/terminal_agent.rs` 109) |
| 장애 경계 상태 | egui temp data, id `(STATE_KEY, region, instance, viewport_id)`(`UI/error-boundary.rs` 115) |
| tooltip 상태 | `UI/tooltips.rs` `Provider(Arc<Mutex<HashMap<ViewportId, Viewport>>>)`(86), viewport별 `open: Option<Id>`(100) |
| Welcome 최근 목록 캐시 | `shell_state.welcome: crate::welcome::State`(`APP/welcome.rs` 34, `loaded_for`가 snapshot 세대) |
| 상태 표시줄 문구 | `NativeApplication.status: Option<String>`. 비검사 앱 코드의 `status = Some(` 97곳(2026-11-04 메인: application.rs 21, domains/editor.rs 51, explorer_search.rs 8, commands.rs 5, terminal_agent.rs 5, shell.rs 4, preview.rs 2, git.rs 1). git worktree는 105곳(git.rs 8) |

다른 영역 상태는 읽기만 하고, 바꿔야 하면 그 영역의 `pub(crate)` 메서드를 부릅니다(seams 1.5). 메서드가 없으면 그 영역 파일에 최소 추가하고 QA에 사유를 적습니다.

### 2.4 새 명령을 붙이는 규칙(이 저장소 관례)

1. `UI/command-registry.rs`
    - `Run` 끝(197행 `MoveFocusedTab(WindowMove),` 뒤)에 변형을 추가하고 `#[cfg(feature = "native-host")]`로 막습니다. `Run`은 `Copy`라 `Clone`만인 타입을 담지 않습니다. 필요하면 `WindowMove`(202)처럼 작은 `Copy` enum을 새로 두고 그것도 같은 cfg로 막습니다.
    - `execution()`의 마지막 `let run = match id {`(455) 안, `_ =>`(485) 바로 앞에 `#[cfg(feature = "native-host")] "<id>" => Some(Run::X),`를 더합니다.
    - 활성 조건이 원본과 다르면 `enablement()`(331)를 고칩니다.
    - 검사: 실행 경로 표 검사 `실행경로는_native동작이_있는_명령과_keymap액션만_typed요청으로_구분한다`(1339)의 `filter_map` match(1354 선례 `Execution::Native(Run::OpenWelcomeTab | Run::MoveFocusedTab(_)) => None`)에 새 변형을 더합니다. 고정 길이 `NATIVE_COMMANDS: [(&str, Run); 38]`(985)에는 cfg 행을 넣을 수 없으므로 새 변형 전용 검사를 따로 둡니다(선례 `cli_세_명령은_mac_전용_셸_명령_범주로_native_host에서만_cli_실행경로를_갖는다`(1221), `welcome과_탭_창이동_명령은_native_host에서만_실행되고_메인창_이동은_보조창에서만_활성이다`(1283)).
2. intent가 영역 하위 enum이면 `UI/intents/<영역>.rs`에 변형을 추가합니다(`#[cfg(feature = "native-host")]`). commands 영역은 `UI/intents/commands.rs`.
3. `APP/shell_keymap.rs` `intent`: 프로젝트가 필요 없으면 첫 match(98~151), focused pane이 필요하면 두 번째 match(155~232), 활성 탭이 필요하면 233행 이후에 둡니다. 키 경로에서도 프로젝트 없이 실행돼야 하면 `runs_without_focused_project`(65)에 넣습니다(keymap id가 있는 명령만 해당).
4. 호스트가 필요하면 `APP/domains/commands.rs`의 `Command`/`Reply`에 변형을 더하고 `dispatch`·`apply_commands_reply`·`apply_commands_intent`에 처리를 둡니다.
5. 검사 목록 갱신: `APP/command-dispatch.rs` 검사의 "unavailable" 목록 두 곳(804~815 `명령_gate는_…`의 `accepts` 목록, 1055 `명령_실행은_…`의 `intent` 목록)과 무프로젝트 수락 목록(820~829).

예(실제 코드, `UI/command-registry.rs` 476~485):

```rust
        "monaco.taide.saveFile" => Some(Run::SaveActiveTab),
        #[cfg(feature = "native-host")]
        "view.welcome" => Some(Run::OpenWelcomeTab),
        #[cfg(feature = "native-host")]
        "tab.moveToNewWindow" => Some(Run::MoveFocusedTab(WindowMove::NewWindow)),
        #[cfg(feature = "native-host")]
        "tab.moveToMainWindow" => Some(Run::MoveFocusedTab(WindowMove::MainWindow)),
        _ => id
```

호스트를 오래 막는 작업(네트워크·blocking)은 worker 안에서 기다리지 않고 `services.tasks.spawn_transient(..)`로 넘긴 뒤 `None`을 돌려줍니다. 실제 선례(`APP/domains/terminal_agent.rs` 348~370):

```rust
        Command::Cli(action) => {
            let ports = integrations.agent_cli.clone();
            let tasks = services.tasks.clone();
            let sender = worker.sender.clone();
            let repaint = worker.repaint.clone();
            if services.tasks.spawn_transient(AGENT_CLI_TASK, async move {
                let result = tasks
                    .run_blocking_result(AGENT_CLI_TASK, move || {
                        crate::agent_cli::run(&ports, action)
                    })
                    .await;
                let reply = HostReply::TerminalAgent(Reply::Cli { action, result });
                if sender.send(reply).await.is_ok() {
                    repaint();
                }
            }) {
                None
            } else {
                Some(HostReply::TerminalAgent(Reply::Cli {
                    action,
                    result: Err(crate::host::stopped()),
                }))
            }
        }
```

### 2.5 공용 UI 토큰(67~79, 84, 86 공통)

원본 shadcn 토큰은 `TS/shared/styles/global.css` 138~155의 별칭입니다. native 테마 키 대응(사양 2.0.3, 키는 모두 `CR/taide-theme`의 필수 키이고 `UI/presentation.rs` `visuals`(143~225)가 대부분을 이미 씀):

| 원본 토큰 | native 테마 키 |
| --- | --- |
| background / foreground | `app.background` / `app.foreground` |
| card | `panel.background` |
| popover | `popover.background`, 테두리 `popover.border`, 구분선 `popover.separator` |
| primary / primary-foreground | `button.primaryBackground` / `button.primaryForeground` |
| muted / muted-foreground | `appSidebar.itemHover` / `appSidebar.iconDefault` |
| accent / accent-foreground | `list.hoverBackground` / `list.foreground` |
| destructive | `statusIndicator.error` |
| border / input / ring | `app.border` / `input.border` / `app.focusBorder` |
| 메뉴 | `menu.background`, `menu.border`, `menu.itemHover`, `menu.separator` |
| modal | `modal.background`, `modal.border`, `modal.itemHover`, `modal.separator` |
| 선택 행 | `list.activeBackground` + `app.accent` 1px 링 |
| scrollbar | `scrollbar.thumb`, `scrollbar.thumbHover`, `scrollbar.track` |
| radius | sm 4px, md 6px, lg 8px(`--radius` 0.375rem, `global.css` 156) |
| 그림자 | `shadow-overlay` 0 2px 8px(348), `shadow-overlay-lg` 0 8px 24px(352), 색 `app.shadow`. scrim = `app.shadow` 50%(356) |

설계(사양 2.0.3): 새 `UI/ui-tokens.rs`(`#[path = "ui-tokens.rs"] pub mod ui_tokens;`를 `UI/lib.rs`에 추가)에 `pub struct Tokens { … }`와 `impl Tokens { pub fn new(theme: &ResolvedTheme) -> AppResult<Self>; pub fn store(&self, ctx: &egui::Context); pub fn get(ctx: &egui::Context) -> Option<Self> }`. 값은 `presentation::color(theme, "<키>")?`(59)로 읽습니다. 앱은 테마가 바뀔 때 `APP/presentation-refresh.rs` `Appearances::new`(230)에서 함께 만들고, `presentation::apply_visuals`를 부르는 두 곳(`APP/application.rs` 395 시작 시, `APP/domains/settings.rs` 1263 `apply_presentation_appearances`)에서 `tokens.store(ctx)`를 부릅니다. 공용 위젯은 `Tokens::get(ui.ctx())`만 읽고, `None`(UI 단위 검사에서 store 전)이면 egui 현재 `Visuals` 값으로 그립니다(패닉 금지).

### 2.6 공용 위젯 행의 완료 기준(결정 6 A, 사양 2.0.4)

- (a) 원본 소비 화면이 실제로 쓰는 variant·state 조합을 native 공용 모듈 하나가 모두 그립니다. 원본에 소비자가 없는 variant(button `secondary`·`link`, size `lg`·`icon`·`icon-sm`·`icon-lg`, 메뉴 checkbox·radio 항목, `CommandDialog`, `CommandSeparator`, `scroll-area.tsx`)는 대상이 아닙니다.
- (b) native에 이미 있는 소비 화면은 모두 그 모듈을 씁니다(화면별 복제 구현 제거).
- (c) 다른 영역 소유 소비 화면은 그 영역이 병합된 상태에서 그 모듈을 쓰는지 확인합니다. 아직 화면이 없으면 행 비고에 남깁니다.
- 검사: 공용 모듈 단위 검사(egui context에서 그려진 shape의 색·크기, accesskit role·toggled·disabled), 이전한 소비 화면의 기존 App·UI 검사 통과 + 소비 화면마다 대표 단언 1개.

TS 소비자 목록은 `grep -rlE "@shared/ui/<이름>'" /Users/hyunseokbyun/development/TAIDE/src --include='*.tsx'`로 만듭니다(zsh에서는 `--include` 값을 따옴표로 감쌉니다. 각 행 (b)에 2026-11-04 결과를 적음). native 대응 파일은 그 TS 화면의 locale 키를 native에서 찾아(`grep -rn "<키>" native/taide-native-ui/src native/taide-native-app/src`) 확정합니다.

### 2.7 검사 하네스

#### 2.7.1 UI crate 단위 검사(egui context 직접 구동)

UI 모듈 끝에 `#[cfg(test)] #[path = "<모듈>-tests.rs"] mod tests;`를 두고(`UI/modal.rs` 314~316 선례), 검사 파일에서 `egui::Context::default()`와 `run_ui(RawInput{…}, |ui| …)`로 프레임을 돌립니다. 실제 예(`UI/modal-tests.rs` 62~ `frame`):

```rust
fn frame(&mut self, events: Vec<Event>, step: Step, transition: Option<Transition>) -> egui::FullOutput {
    self.time += FRAME_SECONDS;
    let mut output = self.context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(SCREEN[0], SCREEN[1]))),
            time: Some(self.time),
            focused: true,
            events,
            ..Default::default()
        },
        |ui| { /* 그릴 위젯 */ },
    );
    output.textures_delta.clear();
    output
}
```

- 그려진 결과는 `output.shapes`(예: `egui::Shape::Rect(rect)`의 `rect.fill`, `egui::Shape::Text(t)`의 `t.galley.text()`)로, 접근성은 `context.enable_accesskit()` 뒤 `output.platform_output.accesskit_update`의 `nodes`(`node.role()`, `node.label()`, `node.toggled()`, `node.is_disabled()`)로 단언합니다(`APP/application-project-rail-tests.rs` 30·43 `has_label`·`has_text` 선례).
- 실행: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --features native-host,inspection --lib --locked --offline --target-dir <T> -- <필터>`

#### 2.7.2 UI 셸 하네스(workbench)

`native/taide-native-ui/tests/workbench.rs`: 가짜 `Surfaces`가 `ShellSurfaces`·영역 surface trait을 구현하고 `NativeShell`을 그립니다. 테마 visuals 검사 선례 `테마_토큰은_원본_css변수의_egui_visuals로_변환되어_두_테마_슬롯에_적용된다`(1018), 장애 경계 검사 8건. 실행: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --features native-host,inspection --test workbench --locked --offline --target-dir <T> -- <필터>`.

#### 2.7.3 앱 lib 실제 App 검사

- fixture(모두 `APP/application-project-rail-tests.rs`, `pub(super)`): `Fixture { directory, application }`(15, Drop에서 `eframe::App::on_exit` 호출과 임시 폴더 삭제), `project_ref`(69), `isolated_state(data)`(80, IDE 서버·agent hooks 끔), `insert_project(state, id, root, name, root_missing)`(95), `launch(fixture, state) -> egui::Context`(120, `NativeApplication::new` + `context.enable_accesskit()`), `step(application, context, events) -> FullOutput`(142, `eframe::App::logic` + paint), `pointer(position, button)`(192).
- `APP/application-inlay-hints-tests.rs`: `paint(application, context, events)`(32, logic 없이 ui만), `key(egui::Key::X, egui::Modifiers::Y)`(49, 누름·뗌 두 이벤트).
- 팔레트 실행 helper `run_palette(application, context, "<명령 id>")`(`APP/application-selection-ranges-tests.rs` 74~114). 실제 코드:

```rust
pub(super) fn run_palette(application: &mut NativeApplication, context: &egui::Context, command: &str) {
    application.commands_state.palette.open(context, taide_native_ui::command_registry::PaletteEntry::Commands);
    paint(application, context, Vec::new());
    paint(application, context, Vec::new());
    assert!(application.commands_state.palette.inspection().has_input_focus);
    let label = crate::command_registry::registry().unwrap().command(command).unwrap().label(&application.locale);
    paint(application, context, vec![egui::Event::Text(label)]);
    assert!(application.commands_state.palette.inspection().rows.iter()
        .any(|row| row.key == command && row.is_enabled && row.is_selected), /* … */);
    paint(application, context, key(egui::Key::Enter, egui::Modifiers::NONE));
}
```

  행이 비활성이면 이 helper 안의 단언이 실패합니다. 비활성 검사는 `commands_state.palette.inspection().rows`를 직접 봅니다(`APP/application-commands-tests.rs` 563 `실제_app_팔레트_ime_디버그_복사는_비활성_행이라_클릭해도_실행되지_않는다` 선례).
- `APP/application-commands-tests.rs` 안의 helper: `fixture(name)`(34), `file_tab(root, name)`(44), `focus_single_slot`(60), `settle`(72, 4프레임), `step_until(application, context, phase, ready)`(78, 10초 deadline 루프), `pane_tabs`(99), `visible_toasts(application) -> Vec<(Kind, String)>`(109), `command_shift()`(300), `route_viewports(context) -> RoutedEvents`(309, 보조 viewport 입력 라우팅), `unfocused_main_step`(333), `auxiliary_until`(350), `run_auxiliary_palette(application, context, events, viewport, command)`(371, 보조 창에 ⌘⇧P → 글자 → Enter).
- 보조 창 선례: `실제_app_팔레트_탭_창이동은_새창과_메인으로_옮기고_빈_pane은_무시한다`(414).
- toast: `application.toasts.inspection()`(`Inspection { kind, title, description, action_label, is_dismissed, close, action }`, `UI/toast.rs` 349). action 버튼 클릭은 `inspection[i].action.unwrap().center()`에 `PointerMoved`·`PointerButton{pressed: true/false}`를 보냅니다(`APP/toast-tests.rs` 357~372 선례).
- 장애 경계 주입: `taide_native_ui::error_boundary::{inject_panic, inject_instance_panic}`(`APP/application-error-boundary-tests.rs`, `assert_fallback` 79).
- secret store가 필요하면 `NativeApplication::new_with_secret_store(&CreationContext, runtime, state, tasks, cli_path, Vec::new(), Some(SecretStoreState::with_store(Arc::new(<가짜 SecretStore>))))`(`APP/application.rs` 290, 실제 호출은 `APP/application-settings-sections-tests.rs` 126~138).
- git 저장소 fixture(git 병합 뒤): `crate::application::domains::git::fixture::{git, init, repository}`(`GITWT/native/taide-native-app/src/application-git-fixture.rs`, `domains/git.rs` 2989~2991 `#[cfg(test)] pub(crate) mod fixture;`). `repository(root, empty, &[("a.txt", "one\n")])`는 전역·시스템 git 설정과 hook을 끈 임시 저장소에 첫 commit을 만듭니다. git 검사 모듈(`domains::git::{base_tests, graph_tests, editor_tests, claude_history_tests}`)의 helper(`Workspace`, `show_git`, `until`)는 `mod base_tests;`가 비공개라 commands 검사에서 쓸 수 없습니다.
- commands 영역의 새 App 검사는 모두 `APP/application-commands-tests.rs` 한 파일에 둡니다. 연결은 이미 `APP/domains/commands.rs` 649~651 `#[cfg(all(test, unix))] #[path = "../application-commands-tests.rs"] mod tests;`입니다. 필터는 `domains::commands::tests`.
- 모듈 가시성 주의: `domains`는 `application`의 자식이라 `crate::application::project_rail_tests` 같은 `application` 직속 검사 모듈은 보이지만, `domains::settings`의 자식 검사 모듈(`settings_sections_tests::{Scene, Keychain}`(1647), `settings_sync_remote_tests::FakeGist`(1655))은 비공개 `mod`라 `domains::commands::tests`에서 보이지 않습니다. 30·31 검사는 3.3절의 방법을 씁니다.
- 실행: 먼저 `cargo build --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir <T>`(LSP 검사용 mock, 없으면 앱 lib 일부가 실패). 그다음 `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- <필터> --test-threads=2`.

`<T>`: 메인 트리는 `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`(병합 게이트와 같음), lane worktree는 `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`.

### 2.8 검증 게이트

묶음 끝마다:
1. 변경 crate `cargo check --quiet --all-targets`(UI는 `--features native-host,inspection`).
2. 관련 필터 검사(위 명령).
3. 바꾼 crate마다 `cargo fmt --manifest-path <crate>/Cargo.toml -- --check`.
4. UI crate를 바꿨으면 `cargo check --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline --target-dir <T>`.
5. `Cargo.toml`·`Cargo.lock` 변경 없음 확인(`git -C <트리> diff --stat -- '*Cargo.*'`).
6. 다른 영역 화면 파일을 고친 묶음(5·6)은 변경 crate 전체 `--no-fail-fast -- --test-threads=4`를 1회(사양 3절 주의 4). 병합 게이트는 `docs/utils/2026-11-03-native-merge-gate.sh <로그 폴더>`(lsp-mock 빌드 → 앱 전체 `--test-threads=4` → UI 전체 → editor·syntax·terminal → remote-web Wasm → 5개 crate fmt → 로그별 `test result: ok` 수와 FAILED 목록).

### 2.9 기능표·QA 갱신 규칙

- 기능표에서 이번 묶음 ID 행만 바꿉니다. 상단 합계 표는 건드리지 않습니다.
- 비고에는 실행 경로(심볼), 검사 이름, QA 문서 경로를 적습니다. 부분이면 partial과 정확한 남은 것을 적습니다.
- QA 문서: `QA/<날짜>-native-lane-commands-ui-<묶음키>.md`(대상 ID, 원본 근거, 사양과 다르게 한 점, 구현, 검증 명령과 실제 결과 수, 남은 범위). 형식 선례는 `QA/2026-11-02-native-lane-terminal-agent-commands-ui-entry.md`.

---

## 3. 행별 상세

각 행은 같은 형식입니다. (a) ID·기능·상태 (b) 원본 TS 동작 (c) native 현재 자산과 정확히 빠진 것 (d) 구현 방법 (e) 자동 검사 (f) 완료 판정 (g) 막는 요소.

### 3.1 묶음 0: 근거 정정만 남은 행

#### commands-ui-32 `cli.connectExternalEditor`, commands-ui-33 `cli.installShellCommand`, commands-ui-34 `cli.uninstallShellCommand`

- (a) 세 행 모두 unwired.
- (b) 원본 `TS/entities/agent/agent.commands.ts` 18~66. macOS에서만 등록(`IS_MAC`), 카테고리 `KEYMAP_CATEGORY.SHELL_COMMAND`, 활성 조건 없음, 키 없음.
    - connect(18~30): `getCliInstallStatus()` → `installed && !dangling`이면 그 상태, 아니면 `installCliCommand()` → 결과가 `installed && !dangling`이 아니면 `toast.error(t('settings.cliInstallFailed'))`, 맞으면 `toast.success(t('settings.cliExternalEditorConnected'))`. 예외 → `settings.cliInstallFailed`.
    - install(32~39): 오류 없으면 `settings.cliInstallSuccess`, 예외면 `settings.cliInstallFailed`. 원본은 관리자 대화상자를 취소해도 성공 문구를 보이지만, 결정 8이 뒤집혀 native는 취소에 성공 문구를 보이지 않습니다(0.3절).
    - uninstall(41~48): `settings.cliUninstallSuccess` / `settings.cliUninstallFailed`.
    - 제목 키: `keymap.cliConnectExternalEditor`, `keymap.cliInstall`, `keymap.cliUninstall`.
- (c) terminal-agent lane 묶음 3이 전부 구현·병합했습니다(`QA/2026-11-02-native-lane-terminal-agent-commands-run.md`, 기능표 terminal-agent-56 비고 "명령 부분 완료").
    - `UI/command-registry.rs` 462~473: 세 id → `Run::Cli(CliAction::{ConnectExternalEditor, Install, Uninstall})`(native-host).
    - `APP/shell_keymap.rs` 142~146: `Run::Cli(action)` → `ShellIntent::TerminalAgent(Intent::Cli(action))`(프로젝트 없이).
    - `APP/domains/terminal_agent.rs` 348~370(`Command::Cli`), 445(`Reply::Cli`), 458~466(`Intent::Cli` 제출과 실패), 467~(`finish_agent_cli`: toast와 `cli_status`).
    - 검사: 앱 `application::terminal_agent_tests::실제_app_cli_셸_명령은_팔레트에서_실행되어_원본_toast를_보이고_취소는_성공_문구를_보이지_않는다`(`APP/application-terminal-agent-tests.rs` 1513, 실제 팔레트, 모듈 연결 `#[cfg(all(test, target_os = "macos"))]` `APP/application.rs` 3001~3003), `agent_cli::tests` 6건, UI `command_registry::tests::cli_세_명령은_…`(1221), `command_dispatch` 검사의 `cli.*` 무프로젝트 수락(820~829).
    - 빠진 것: 기능표 행 갱신뿐입니다.
    - 차이 기록 대상(코드 변경 아님): (1) install 성공 판정이 원본(오류 없으면 성공)과 달리 설치 상태 확인 결과를 따름(결정 8 예외). (2) Settings CLI 행(settings-16)은 `settings_ports::CliPorts`(`APP/settings-ports.rs` 125), 명령은 `agent_cli::Ports`(`APP/host.rs` 56 `HostIntegrations.agent_cli`)로 설치 구현이 두 갈래입니다. 원본도 명령과 Settings 행이 같은 IPC를 따로 부르고 명령 뒤 query를 무효화하지 않으므로 화면 결과 차이는 없습니다(Settings 행은 60초 stale 뒤 재조회, `UI/settings-sections-data.rs` 22 `STALE_TIME`).
- (d) 코드 변경 없음. 기능표 세 행을 complete로, 비고에 위 경로·검사·QA 경로와 차이 (1)을 적습니다.
- (e) 근거 재실행(같은 HEAD에서 1회): 앱 `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir <T> -- agent_cli::tests terminal_agent_tests::실제_app_cli command_dispatch::tests --test-threads=2`, UI `… --features native-host,inspection --lib … -- command_registry::tests::cli`. 키 override 경로는 `command_dispatch::accepts`의 무프로젝트 수락 단언이 덮습니다.
- (f) 위 검사가 현재 HEAD에서 통과 → 세 행 complete.
- (g) 실제 `/usr/local/bin/taide` 설치·해제와 관리자 암호는 사용자 허락 항목입니다(lane 결정 보류 목록). 자동 검사는 가짜 osascript 포트까지만 합니다.

#### commands-ui-61 find / search / search-replace 키

- (a) partial. 비고 "editor find는 연결됐으나 프로젝트 search/search-replace는 없습니다"는 현재 사실과 다릅니다.
- (b) 원본 `TS/widgets/editor-area/editor-area.tsx` 153~160 `openFind`: `hasWidgetFocus()`인 편집기가 있으면 그 편집기의 `actions.find`, 없으면 `requestOpenSearchPanel()`. 162~167 `openGlobalSearch`: `hasTextFocus` 편집기의 비지 않은 한 줄 선택을 seed로. 292~299 `useGlobalKeymap`의 `find`·`search`·`search-replace`는 `whenFocused`(focused slot만).
- (c) explorer-search lane 묶음 C가 구현·병합했습니다(`QA/2026-11-02-native-lane-explorer-search-search-entry-editor.md`, explorer-search-37 complete).
    - `UI/command-registry.rs` 658~659 `"search" => Run::OpenSearch{replace: false}`, `"search-replace" => Run::OpenSearch{replace: true}`, 642~647 `find` → `FindCommand::Open`.
    - `APP/command-dispatch.rs` 62~66(`accepts`의 `find` 대체), 98~100(`intent_with_editor`의 편집기 focus 없음 → `search`).
    - `APP/shell_keymap.rs` 194~211 `OpenSearch`/`OpenAuxiliarySearch`.
    - 검사: `APP/application-search-panel-tests.rs` 940 `실제_app_명령_shift_f는_편집기의_한_줄_선택을_검색어로_넣고_전체_선택하며_바꾸기와_접힘과_범위와_명령_f를_처리한다`, 1289 `실제_app_보조_창의_명령_shift_f는_그_창_탐색기의_검색_보기만_연다`, `command_dispatch` 단위.
    - 빠진 것: 행 갱신뿐.
- (d) 코드 변경 없음. 행을 complete로, 비고에 explorer-search-37 근거와 위 검사 이름.
- (e) `… --lib -- search_panel_tests::실제_app_명령_shift_f search_panel_tests::실제_app_보조_창의_명령_shift_f command_dispatch::tests --test-threads=2`.
- (f) 통과 → complete.
- (g) 없음.

#### commands-ui-83 status-error-banner

- (a) missing. 비고 "…상단 재시도 배너의 실제 native 사용자 경로는 없습니다"는 사실과 다릅니다.
- (b) 원본 `TS/shared/ui/status-error-banner.tsx`(28줄): `position: fixed` 위쪽, 높이 32px(`STATUS_ERROR_BANNER_HEIGHT_PX`), 배경 `status-error` 15%, 글자 `status-error`, 좌우 12px·위아래 6px, 12px, `AlertTriangle` 14px, 밑줄 `<button>` 재시도(`retryLabel`). 소비자는 로케일 공급자(고정 영어 문구)와 테마 공급자(`theme.loadFailed`), 둘 다 실패면 테마 배너가 32px 아래.
- (c) shell-94 complete(병합됨): `APP/status-error-banner.rs`(`startup` 53, `show` 104, `LOCALE_MESSAGE` 17), `Refresh::retry`(`APP/presentation-refresh.rs` 169). 시작 시 로드 실패 대체(번들 `taide-dark`·`en`)와 OS 파일 drag overlay 아래 그리기도 완료(`QA/2026-11-02-native-lane-shell-windows-layers.md`). 검사: `APP/application-explorer-views-tests.rs` 314 `실제_app은_테마와_로케일_로드_실패를_상단_배너로_쌓고_재시도로_다시_불러온다`, 596 `실제_app은_시작_때_테마와_로케일_로드가_실패해도_번들_기본값으로_띄우고_배너를_보인다`, 653 `실제_app은_시작_때_사용자_테마를_그릴_수_없으면_번들_테마로_띄우고_테마_배너를_보인다`. 빠진 것: 행 갱신뿐.
- (d) 코드 변경 없음. 행을 complete로, 비고에 shell-94 근거.
- (e) `… --lib -- explorer_views_tests::실제_app은_테마와_로케일 explorer_views_tests::실제_app은_시작_때 --test-threads=2`.
- (f) 통과 → complete.
- (g) 배너 픽셀은 배치 31 실기.

#### commands-ui-82 error-boundary(정정만, complete는 wave 4)

- 이 행은 3.6절에 전체 형식으로 적었습니다. 묶음 0에서는 비고를 shell-12 근거로 바꾸고 상태를 missing → partial로 올리는 것만 합니다.

### 3.2 묶음 3: Git 병합 뒤 닫는 행

시작 조건: `git -C /Users/hyunseokbyun/development/TAIDE log --oneline -5`에 git lane 병합 커밋이 있고, `grep -n "ShowGit\|GitRun" /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src/command-registry.rs`가 결과를 냅니다. 아직이면 이 절은 시작하지 않습니다.

이 절의 공통 사실(2026-11-04 `GITWT` 검사 본문 확인): git lane의 명령 검사는 모두 실제 팔레트 UI를 거치지 않고 명령 채널에 직접 넣습니다(`application.commands_state.palette_commands.push("view.git".into())` — `GITWT/…/application-git-base-tests.rs` 311~315, `git.revertHead` — `application-git-graph-tests.rs` 792~795, `git.openFileHistory` — `application-git-claude-history-tests.rs` 484~487, 편집기 검사의 `run` helper — `application-git-editor-tests.rs` 195~201). 따라서 이 영역은 행마다 `run_palette`로 실제 팔레트 검사 1건을 더합니다(팔레트 행의 활성 여부와 Enter 선택까지 덮기 위해). 키(⌃⇧G)는 `show_git` helper(`application-git-base-tests.rs` 229~235)가 실제 키 이벤트를 보냅니다.

또한 git 병합은 `APP/command-dispatch.rs` 검사를 이미 바꿨습니다(`GITWT/…/command-dispatch.rs`): `KEYMAP_ACTIONS: [&str; 39]`(681, `"git"` 포함), `PROJECTLESS_KEYMAP_ACTIONS: [&str; 12]`(722, `"git"` 포함), unavailable 목록(806~815)에서 `"git"`·`"git.revertHead"` 제거, `for command in ["task.runTask", "git.revertHead", "git.createTagOnHead"]`(818)의 프로젝트 필요 단언 추가.

#### commands-ui-14 `view.git`

- (a) unwired.
- (b) 원본 `TS/shared/lib/command-catalog.ts` 94 `{ id: 'view.git', titleKey: 'git.title', categoryKey: KEYMAP_CATEGORY.VIEW, keymapId: 'git', run: () => requestShowExplorerView('git') }`. 키 `git`(⌃⇧G, `UI/keymap-defaults.json` 57). 활성 조건 없음. 프로젝트가 없으면 explorer panel이 없어 무동작. 제목 locale `git.title`("Git").
- (c) git lane(G1, `GITWT/docs/quality-assurance/2026-11-02-native-lane-git-g1-base.md`):
    - `GITWT/native/taide-native-ui/src/command-registry.rs`: `Run::ShowGit`(172), `"git" => Run::ShowGit`(677, `keymap_run`), `("view.git", Run::ShowGit)`(1017, `NATIVE_COMMANDS`).
    - `GITWT/native/taide-native-app/src/shell_keymap.rs` 72(`runs_without_focused_project`에 `ShowGit`), 200~211(`Run::ShowGit` → 메인 `intents::git::Intent::ShowView{project, slot: snapshot.shell.focused}`·보조 창 `ShowAuxiliaryView{project, slot}`). 프로젝트가 없으면 `project?`(158)에서 `None` → 무동작.
    - 검사: `GITWT/…/application-git-base-tests.rs` 257 `실제_app_git_키와_view_git_명령은_포커스_slot만_git_보기로_바꾸고_접힌_사이드바를_펼친다`(⌃⇧G 키 + `palette_commands.push("view.git")`), 327 `실제_app_보조_창의_git_키는_그_창의_탐색기를_열고_git_보기로_바꾼다`.
    - 빠진 것: 병합, 실제 팔레트 경로 검사, 행 갱신.
- (d) 코드 변경 없음(병합된 것 사용).
- (e) `APP/application-commands-tests.rs`에 1건: 기존 `실제_app_팔레트_view_explorer는_포커스_슬롯만_접힘을_풀고_파일_보기로_바꾼다`(120~195)를 그대로 본떠(두 프로젝트·두 slot·접힘·두 sidebar Outline) `run_palette(application, &context, "view.git")` → `step_until(.., |_| !collapsed(1))` → focused slot만 `View::Git`(`crate::symbol_sidebar::View`의 Git 변형, git 병합 뒤 존재 확인), 다른 slot은 `View::Outline`·접힘 유지. 추가로 프로젝트 없는 fixture에서 `view.git` 행이 활성이고 실행해도 레이아웃 수·toast 변화가 없음을 단언합니다. 실행: `… --lib -- domains::commands::tests base_tests --test-threads=2`.
- (f) 팔레트 경로 검사 통과 → complete.
- (g) git lane 병합(다른 작업자가 병합 중).

#### commands-ui-62 explorer / git 키

- (a) unwired.
- (b) 원본 explorer-panel-bridge: ⌘⇧E → `requestShowExplorerView('files')`, ⌃⇧G → `requestShowExplorerView('git')`. 둘 다 `whenFocused`(focused slot만).
- (c) explorer 키는 shell-21 근거(메인 기능표 shell-21 partial, git worktree 기능표 shell-21 complete — 남은 범위가 git 키뿐이어서 git lane이 갱신). git 키는 14와 같은 git lane 경로·검사(`application-git-base-tests.rs` 257의 `show_git`(⌃⇧G 실제 키)·327 보조 창). 빠진 것: 병합과 행 갱신.
- (d) 코드 변경 없음.
- (e) git lane 검사가 실제 키를 보내므로 그 검사 재실행을 근거로 씁니다(`… --lib -- base_tests::실제_app_git_키와 base_tests::실제_app_보조_창의_git_키 --test-threads=2`). 프로젝트 없음 → 키 수락(`runs_without_focused_project`)·무동작은 `command_dispatch` 검사의 `PROJECTLESS_KEYMAP_ACTIONS`가 덮습니다.
- (f) 통과 → complete.
- (g) git lane 병합.

#### commands-ui-36 `git.revertHead`

- (a) unwired.
- (b) 원본 `TS/entities/git/git.commands.ts` 14~22·32: 활성 = `isProjectActive`(29). `revertGitCommit({ projectId, rev: 'HEAD' })` → `outcome.conflicted`면 `toast.warning(t('git.revertConflict'))`, 아니면 `toast.success(t('git.revertSuccess'))`, 오류 `toast.error(describeIpcError(error))`. 제목 키 `git.revert`("Revert Commit").
- (c) git lane G6(`GITWT/docs/quality-assurance/2026-11-02-native-lane-git-g6-log-graph.md`): `GitRun::RevertHead`(`GITWT/…/command-registry.rs` 153, 474), 활성 `Enablement::ActiveProject`(362) → `GITWT/…/shell_keymap.rs` 143~147 `intents::git::Intent::RevertHead(project)` → `GITWT/…/domains/git.rs` 2102~2108 `HostCommand::Git(Command::Revert { project, rev: "HEAD", opens_conflict: false })`. 검사 `GITWT/…/application-git-graph-tests.rs` 756 `실제_app_팔레트_git_명령은_head를_revert하고_접힌_그래프에서도_포커스_slot에_head_태그_대화상자를_하나만_연다`(채널 주입). 빠진 것: 병합, 실제 팔레트 경로 검사, 행 갱신.
- (d) 코드 변경 없음.
- (e) `APP/application-commands-tests.rs`에 1건: `crate::application::domains::git::fixture::repository(&root, &empty, &[("a.txt", "one\n")])`로 임시 저장소를 만들고 두 번째 commit(`b.txt`)을 `fixture::git`으로 추가한 뒤 그 폴더를 프로젝트로 띄웁니다(`insert_project` + `focus_single_slot`). `run_palette(.., "git.revertHead")` → `step_until(.., |application| visible_toasts(application).contains(&(Kind::Success, <git.revertSuccess 문구>)))` → `fixture::git(&root, &empty, &["log", "-1", "--format=%s"])`가 `Revert "<두 번째 메시지>"`, `b.txt` 없음. 프로젝트가 없는 fixture에서는 행이 비활성(`inspection().rows`의 `is_enabled == false`). 실행: `… --lib -- domains::commands::tests::실제_app_팔레트_git --test-threads=2`. 검사는 임시 저장소만 씁니다.
- (f) 통과 → complete.
- (g) 병합. 실제 사용자 저장소에서 revert 실행(새 commit)은 사용자 허락 항목(사양 4.2-3).

#### commands-ui-37 `git.createTagOnHead`

- (a) unwired.
- (b) 원본 `git.commands.ts` 24~27·33: 활성 = `isProjectActive`. `requestShowExplorerView('git')` + `requestOpenCreateTagDialog({ target: 'HEAD' })`. 제목 키 `git.createTag`("Create Tag"). git lane 결정: 원본 결함은 재현하지 않음.
- (c) `GitRun::CreateTagOnHead`(`GITWT/…/command-registry.rs` 475) → 메인 `Intent::CreateTagOnHead{project, slot}`/보조 창 `CreateAuxiliaryTagOnHead`(`GITWT/…/shell_keymap.rs` 212~224) → `domains/git.rs` 2117~2128: `ShowView` 적용 뒤 `request_tag(&project, &slot, "HEAD")`. 검사는 36과 같은 756행 검사(채널 주입). 빠진 것: 병합, 실제 팔레트 경로 검사, 행 갱신.
- (d) 코드 변경 없음.
- (e) 36의 검사에 이어 같은 fixture에서 `run_palette(.., "git.createTagOnHead")` → focused slot sidebar가 `View::Git`이고 태그 대화상자 제목(`git.createTag` 문구)을 담은 accesskit 노드가 1개(`application-project-rail-tests.rs`의 `has_label` 방식). 다시 실행해도 대화상자는 1개(git lane 검사 이름의 "하나만"과 같은 단언).
- (f) 통과 → complete.
- (g) 병합.

#### commands-ui-38 `git.toggleBlame`

- (a) unwired.
- (b) 원본 `git.commands.ts` 34~39: 활성 조건 없음(항상 활성), `requestEditorPaneCommand({ type: 'run-monaco-action', actionId: TOGGLE_BLAME_MONACO_ACTION_ID })` = focused 편집기에서 `taide.toggleBlame` 실행. 편집기가 없으면 무동작. 제목 키 `git.toggleBlame`.
- (c) git lane G8(`GITWT/docs/quality-assurance/2026-11-02-native-lane-git-g8-blame-conflicts.md`): `"git.toggleBlame" | "monaco.taide.toggleBlame" => Some(Run::Git(GitRun::ToggleBlame))`(`GITWT/…/command-registry.rs` 479), `shell_keymap` 237~252(focused pane의 활성 탭이 `TabKind::File`이 아니면 `None` → 무동작, 맞으면 `Intent::ToggleBlame{pane, tab}`) → `GITWT/…/git-editor.rs` 151 → `toggle_git_blame`(248, `git_state.editors.of_tab(pane, tab)`의 `blame.toggle_overlay(line_count)`). 활성 조건: `git.toggleBlame`은 `Always`(원본과 같음, 검사 1984 `("git.toggleBlame", Enablement::Always, [true; 5])`), `monaco.taide.toggleBlame`은 `SupportedEditorAction`. 검사 `GITWT/…/application-git-editor-tests.rs` 270 `실제_app_파일_편집기는_gutter_막대와_blame_footer_overlay를_원본대로_그린다`(채널 주입 `run(.., "git.toggleBlame")` 340). 빠진 것: 병합, 실제 팔레트 경로 검사, 행 갱신.
- 결정 2와의 관계: 편집기가 없을 때 원본은 활성 행·무동작입니다. 결정 2 A가 나열한 대상은 40·43뿐이고 git lane은 원본대로 `Always`로 두었습니다. 그대로 두고 비고에 "편집기 없음 → 무동작(원본과 같음)"을 적습니다.
- (d) 코드 변경 없음.
- (e) `APP/application-commands-tests.rs`에 1건: 36의 저장소 fixture에서 `a.txt` 파일 탭을 연 상태(`file_tab`이 아니라 저장소 안 파일 경로로 `Tab` 생성)로 띄우고 `run_palette(.., "git.toggleBlame")` → `step_until(.., |application| application.git_state.editors.of_tab(&pane, &tab).is_some_and(|editor| editor.blame.overlay().is_some()))`(`GITWT/…/git-blame.rs` 155 `overlay()`, `git-gutter.rs` 247 `of_tab`; `of_tab`은 `&mut self`라 클로저가 아니라 루프 안에서 `application`을 가변으로 빌려 확인). 빈 pane 상태에서 실행 → host 명령·toast 없음(`visible_toasts` 비어 있음).
- (f) 통과 → complete.
- (g) 병합.

#### commands-ui-39 `git.openFileHistory`

- (a) unwired.
- (b) 원본 `git.commands.ts` 40~45: 활성 조건 없음, focused 편집기에서 `taide.openFileHistory`. 편집기가 없으면 무동작. 제목 키 `git.fileHistory`("File History").
- (c) `"git.openFileHistory" | "monaco.taide.openFileHistory" => Some(Run::Git(GitRun::OpenFileHistory))`(`GITWT/…/command-registry.rs` 476~478), `shell_keymap` 226~236(활성 탭이 `TabKind::File{path}`일 때만 `Intent::OpenFileHistory{project, path}`) → `domains/git.rs` 2244~2248 `git_state.histories.open(context.viewport_id(), project, path)`. 검사 `GITWT/…/application-git-claude-history-tests.rs` 466 `실제_app_파일_이력은_명령_탭_메뉴로_열리고_항목을_inline_diff로_보여준다`(채널 주입, 편집기 없음 → `histories.get(window).is_none()` "편집기가 없으면 무동작" 단언 포함). 빠진 것: 병합, 실제 팔레트 경로 검사, 행 갱신.
- (d) 코드 변경 없음.
- (e) 38과 같은 fixture에서 `run_palette(.., "git.openFileHistory")` → `application.git_state.histories.get(egui::ViewportId::ROOT).is_some()`. 편집기 없는 상태에서 실행 → `None` 유지.
- (f) 통과 → complete.
- (g) 병합.

#### commands-ui-59 디스패치 대상(키맵 41개)

- (a) partial. 기능표 이름의 "31개"는 옛 값이고 실제 키맵 엔트리는 41개입니다(`grep -c '"id"' UI/keymap-defaults.json` = 41, 원본 `TS/shared/lib/keymap/keymap.ts` 97~243).
- (b) 원본: 41개 엔트리마다 `useGlobalKeymap` 소비자가 있습니다(편집기 영역·셸·터미널·탐색기).
- (c) 현재 `to_rust_native`: `keymap_run`(`UI/command-registry.rs` 601~687)이 38개를 받고(`find`는 642~647에서 편집기 찾기), 터미널 점프 2개(`terminal-jump-to-previous-command`, `terminal-jump-to-next-command`)는 `APP/terminal_surface.rs` 3668~3673·4380~4395가 처리합니다(검사는 같은 파일 5268~의 검사 모듈, override JSON 사례 5469 등). 남은 것은 `git` 1개(git 병합 뒤 `Run::ShowGit`). 병합된 `GITWT/…/command-dispatch.rs`는 `KEYMAP_ACTIONS` 39개로 이미 바꿨습니다(3.2절 공통 사실).
- (d) git 병합 뒤:
    1. `APP/command-dispatch.rs` 검사 목록이 git 병합 상태(39·12개, unavailable에서 `"git"` 제거)인지 확인만 합니다.
    2. 같은 검사 모듈에 전수 단언 1건 추가: `UI/keymap-defaults.json`(`include_str!("../../taide-native-ui/src/keymap-defaults.json")`, 경로는 `APP/` 기준이며 `APP/keymap.rs` 1~20의 `include!` 경로와 같은 방식)을 `serde_json::from_str::<serde_json::Value>`로 파싱해 모든 `id`가 `accepts(id, true, &CommandContext::default())`이거나 터미널 점프 2개 중 하나임을 단언하고, 목록 수가 41임도 단언합니다. 검사 이름 예: `키맵_기본값의_모든_id는_실행경로가_있거나_터미널_점프다`.
- (e) `… --lib -- command_dispatch::tests --test-threads=2`.
- (f) 전수 단언 통과 → complete.
- (g) git 병합.

### 3.3 묶음 4: settings 소유 동작을 부르는 명령

#### commands-ui-30 `sync.uploadNow`, commands-ui-31 `sync.downloadNow`

- (a) 두 행 unwired.
- (b) 원본 `TS/entities/sync/sync.commands.ts` 7~43:
    - 업로드(7~14): `uploadSync()` 성공 → `toast.success(t('settings.syncUploadSuccess'))`("Uploaded to GitHub Gist"), 실패 → `toast.error(t('settings.syncUploadFailed'))`("Upload failed", 설명 없음).
    - 다운로드(24~38) `runSyncDownload(force)`: 결과 `kind === 'conflict'` → `toast.warning(t('settings.syncConflictTitle'), { description: t('settings.syncConflictDescription'), action: { label: t('settings.syncConflictPullRemote'), onClick: () => void runSyncDownload(true) } })`. 그 밖 성공 → `toast.success(t('settings.syncDownloadSuccess'))`. 예외 → `toast.error(t('settings.syncDownloadFailed'))`. `force` true는 conflict를 내지 않습니다(16~23 JSDoc).
    - 등록(40~43): `{ id: 'sync.uploadNow', titleKey: 'settings.syncUploadNow', categoryKey: SYNC }`, `{ id: 'sync.downloadNow', titleKey: 'settings.syncDownloadNow', … run: () => runSyncDownload(false) }`. 활성 조건 없음(연결 안 됐으면 백엔드 오류 → 실패 toast).
    - Settings 화면의 같은 동작(settings-46·47)은 충돌 때 대화상자, 명령은 toast입니다.
    - locale 키 9개 모두 `CR/taide-locale/resources/locales/en.json`에 있습니다.
- (c) native 현재:
    - 카탈로그 행만, `Execution::Unavailable`. `APP/command-dispatch.rs` 검사 unavailable 목록 812(`"sync.uploadNow"`)와 1055.
    - 실행 함수: `APP/domains/settings.rs` `execute_sections`(470)의 `SyncUpload` arm(666~683: `services.tasks.run_nonabortable_result("native-settings-sync-upload", sync_actions::sync_upload(&worker.state, worker.secrets.0.as_ref(), || gist, worker.events.as_ref()))`), `SyncDownload` arm(684~690) → private `download_sync(services, ports, reconcile, force) -> AppResult<SyncDownloadResult>`(878~913: `prepare_sync_download` 뒤 `run_nonabortable_result`로 `apply_sync_download` + `settings_actions::apply_and_broadcast` + reconcile).
    - 결과 타입: `sync_upload`는 `AppResult<SyncStatus>`(`CR/taide-runtime/src/sync_actions.rs` 186~191), 다운로드는 `AppResult<SyncDownloadResult>`, `SyncDownloadResult::{Applied { status }, Conflict { remote_updated_at }}`(`CR/taide-model/src/sync.rs` 46~51).
    - 이 경로를 그대로 재사용할 수 없는 이유: Settings section 요청(`sections::Request::Act(Action::SyncUpload)`)의 응답은 `UI/settings-sections-data.rs` 1041~(`Reply::SyncUploaded`, `SyncDownloaded`)이 받아 Busy 상태·`Notice` toast·충돌 owner 대화상자를 처리합니다. 명령은 대화상자가 아니라 toast여야 하고 Settings 탭이 없을 수도 있습니다.
    - 포트: `settings_state.sections_ports: Arc<settings_ports::Ports>`(`APP/domains/settings.rs` 102, `gist: Gist` 필드는 `APP/settings-ports.rs` 183~193). 상태 반영: `sync_upload`·`apply_sync_download`가 `AppEvent::SyncStateChanged`를 발행하고 settings `Events::record`(`APP/domains/settings.rs` 132)가 Settings section 상태를 갱신하므로 명령 뒤 Settings 화면도 맞춰집니다.
    - toast 동작 버튼: UI `Toasts::notify(kind, title, Options{description, action: Some(Action{label, id})}, now)`(`UI/toast.rs` 488), 클릭 결과 `take_actions`(537), 앱 `toast::Action`(`APP/toast.rs` 14~24, 현재 `RetryCreate`·`RetryRename`), 소비 `run_toast_actions`(`APP/application.rs` 878~916, 2307에서 호출).
- (d) 구현 순서:
    1. `APP/domains/settings.rs`(settings 영역 파일, 최소 수정): `download_sync`(878)를 `pub(crate)`로 열고, `SyncUpload` arm 본문을 `pub(crate) async fn upload_sync(services: &Arc<AppServices>, ports: &crate::settings_ports::Ports) -> AppResult<SyncStatus>`로 꺼내 arm은 그 함수를 부르게 합니다. 동작은 바꾸지 않습니다. QA에 "다른 영역 파일 최소 수정"으로 사유를 적습니다.
    2. `UI/command-registry.rs`: `#[cfg(feature = "native-host")] Sync(SyncRun)` 변형과 `#[cfg(feature = "native-host")] #[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum SyncRun { Upload, Download }`. `execution()`에 `"sync.uploadNow" => Some(Run::Sync(SyncRun::Upload))`, `"sync.downloadNow" => Some(Run::Sync(SyncRun::Download))`. 활성 `Always`(기본값, 변경 없음).
    3. `UI/intents/commands.rs`: force를 담을 수 있게 `#[cfg(feature = "native-host")] SyncUpload`, `#[cfg(feature = "native-host")] SyncDownload { force: bool }` 두 변형을 둡니다.
    4. `APP/shell_keymap.rs` 첫 match에 `Run::Sync(SyncRun::Upload) => return Some(ShellIntent::Commands(intents::commands::Intent::SyncUpload))`, `Run::Sync(SyncRun::Download) => return Some(ShellIntent::Commands(intents::commands::Intent::SyncDownload { force: false }))`.
    5. `APP/domains/commands.rs`:
        - `Command::SyncUpload { ports: Arc<crate::settings_ports::Ports> }`, `Command::SyncDownload { force: bool, ports: Arc<crate::settings_ports::Ports> }`. 포트는 intent 처리 시 `self.settings_state.sections_ports.clone()`(검사가 가짜 gist를 주입하는 곳과 같음).
        - `Reply::SyncUploaded(AppResult<SyncStatus>)`, `Reply::SyncDownloaded { force: bool, result: AppResult<SyncDownloadResult> }`.
        - `dispatch`: 네트워크 작업이라 host worker를 막지 않도록 2.4절 `Command::Cli` 선례처럼 `services.tasks.spawn_transient("native-commands-sync", async move { … sender.send(HostReply::Commands(reply)).await; repaint(); })`로 돌리고 `None`을 돌려줍니다. spawn이 거절되면 `Err(crate::host::stopped())`를 담은 Reply를 바로 돌려줍니다. reconcile은 `worker.integrations.reconcile.clone()`.
        - `apply_commands_intent`: 제출(`self.submit(..)`)이 `false`면 같은 실패 toast(`settings.syncUploadFailed`/`settings.syncDownloadFailed`)를 바로 보입니다.
        - `apply_commands_reply`: 업로드 Ok → `self.toasts.success(presentation::message(&self.locale, "settings.syncUploadSuccess", &[]), now)`, Err → `error("settings.syncUploadFailed")`. 다운로드 `Ok(SyncDownloadResult::Conflict{..}) if !force` → `self.toasts.notify(Kind::Warning, message("settings.syncConflictTitle"), Options { description: Some(message("settings.syncConflictDescription")), action: Some(taide_native_ui::toast::Action { label: message("settings.syncConflictPullRemote"), id: crate::toast::Action::SyncPullRemote }) }, now)`. `Ok(Applied{..})` 또는 force의 `Ok(Conflict{..})` → `settings.syncDownloadSuccess`. Err → `settings.syncDownloadFailed`.
    6. `APP/toast.rs` `Action`에 `SyncPullRemote` 추가. `run_toast_actions`(`APP/application.rs` 878)는 `context`를 받지 않으므로 `APP/domains/commands.rs`에 `pub(crate) fn submit_sync_download(&mut self, force: bool)`(제출 + 실패 toast)를 두고, `apply_commands_intent`의 `SyncDownload`와 `run_toast_actions`의 새 arm `crate::toast::Action::SyncPullRemote => self.submit_sync_download(true)`가 모두 그것을 부르게 합니다(`application.rs`는 match arm 추가만). 기존 `run_toast_actions`가 먼저 `flush_dirty()`를 부르는 구조(880)는 그대로 둡니다.
    7. 검사 목록 갱신: `APP/command-dispatch.rs` 812·1055에서 `"sync.uploadNow"`를 빼되 단언 의도(실행 경로 없는 명령 거절)를 지키도록 아직 실행 경로가 없는 다른 id(예: `"window.reload"`가 묶음 7 전까지 남음, 그 뒤에는 `"unknown.future"`만)로 둡니다. 무프로젝트 수락 목록에 두 id 추가. UI 실행 경로 표 검사(1354)에 `Execution::Native(Run::Sync(_)) => None` arm.
- (e) 검사:
    - UI `--lib`: `sync_두_명령은_native_host에서만_실행경로를_갖고_항상_활성이다`(native-host 아닌 빌드에서는 `Unavailable`; `--no-default-features --features inspection`로도 1회).
    - 앱 `application-commands-tests.rs`(가짜 gist·가짜 SecretStore):
        - fixture: `domains::settings`의 `Keychain`·`FakeGist`는 이 모듈에서 보이지 않으므로(2.7.3) 같은 파일 안에 작은 가짜를 둡니다. `struct MemorySecrets(Mutex<HashMap<SecretAccount, String>>)`로 `taide_infra::secret::SecretStore`(`set`·`get` 등, 선례 `APP/application-settings-sections-tests.rs` 544~ `Keychain`)를 구현하고 `SecretAccount::GithubSync`에 PAT를 넣고, `struct FixtureGist`로 `crate::settings_ports::GistClient`(`APP/settings-ports.rs` 32, `discover`·`create`·`update`·`fetch`)를 구현합니다(선례 `APP/application-settings-sync-remote-tests.rs` 33~84 `FakeGist`: `updated_at`·`fetched`를 `Mutex<Result<..>>`로 두고 `uploads: AtomicUsize`로 업로드 수를 셈). 앱은 `NativeApplication::new_with_secret_store`로 띄우고 `application.settings_state.sections_ports = Arc::new(Ports { gist: Gist(Arc::new(gist)), ..Default::default() })`(선례 `Scene::ports` 160~162).
        - 설정: `sync_gist_id = Some("fixture-gist")`, `sync_last_synced_at = Some("2026-10-01T00:00:00Z")`, gist fetch가 `"2026-10-04T00:00:00Z"`를 돌려주면 충돌이 납니다(선례 `실제_sync_충돌은_로컬_유지로_한번_업로드하고_원격_당김으로_한번_강제_다운로드한다` 230~237의 설정과 `payload(font)` 13~31).
        - 단언: (1) `run_palette(.., "sync.uploadNow")` → success toast `settings.syncUploadSuccess`, gist `update` 1회. (2) gist update가 Err → error toast `settings.syncUploadFailed`, description 없음. (3) `sync.downloadNow`(충돌) → warning toast 제목 `settings.syncConflictTitle`, `description` = `settings.syncConflictDescription`, `action_label` = `settings.syncConflictPullRemote`, 설정 미변경. (4) 그 toast의 action 클릭(2.7.3 방법) → force 다운로드 1회, success `settings.syncDownloadSuccess`, `editor_font_size`가 원격 값. (5) 원격이 같은 시각(충돌 아님) → 바로 success. (6) PAT 없음(미연결) → error `settings.syncDownloadFailed`. (7) Settings 탭이 없어도 동작(위 fixture는 Settings 탭 없이 띄움).
        - 키 override: `settings.keymap_overrides`에 `sync.uploadNow`로 가는 키를 넣고 그 키를 보내 같은 success toast(형식은 `APP/keymap-tests.rs`의 override 검사 참고).
    - 실행: `… --lib -- domains::commands::tests::실제_app_sync --test-threads=2`, `… --lib -- command_dispatch::tests settings_sync_remote_tests --test-threads=2`(settings 최소 수정 회귀).
- (f) 위 검사 통과 → 30·31 complete.
- (g) 실제 GitHub gist·Keychain PAT는 사용자 허락 항목. 자동 검사는 가짜 gist·메모리 SecretStore만.

#### commands-ui-85 toast(sonner)

- (a) partial.
- (b) 원본 sonner `toast.success/error/warning/info(title, { description?, action?: { label, onClick } })`. `toast.loading`·`promise`·`dismiss`는 사용처 0건(사양 2.4). action 사용처는 동기화 충돌(`sync.commands.ts` 28~31)과 탐색기 생성·이름 바꾸기 재시도(`TS/widgets/explorer/use-explorer-entry-crud.ts` 127·174) 둘뿐.
- (c) `UI/toast.rs`: `Kind{Warning, Error, Success, Info}`(74), `Options{description, action}`(101), 위치·swipe·motion·focus, 동작 버튼 검사(`Probe` 3125, "Pull Remote" 사례 3157~3196). 아이콘 `RES/toasts/{error,info,success,warning}.svg`. 앱 소비: 탐색기 재시도(`APP/toast.rs` `Action::RetryCreate/RetryRename`), info(`open_project_first` 35), description(AI 편집 실패). 창별 toaster(`APP/toast-layers.rs`). 빠진 것: 원본의 마지막 action 소비자(동기화 충돌)와 4종 × 설명 × 동작 조합의 한 곳 단언.
- (d) 30·31 (d) 5~6이 마지막 소비자입니다. 추가 구현 없음.
- (e) UI `toast` 검사 모듈(1611~)에 1건: 4개 `Kind` × (설명 있음/없음) × (동작 있음/없음) 16조합을 `notify`로 넣고 각 `inspection()`의 `kind`·`description`·`action_label`과, 동작이 있을 때만 `action` hit rect가 있음을 단언합니다. 실행 `… --features native-host,inspection --lib -- toast::tests`.
- (f) 30·31 complete + 조합 검사 통과 → complete. 영역별 toast 문구는 각 영역 행입니다.
- (g) 없음.

### 3.4 묶음 5: 기본 컨트롤(토큰 먼저)

공통 첫 단계: 2.5절 `UI/ui-tokens.rs`를 만들고 `Tokens::new`가 모든 번들 테마에서 성공하는지 단언합니다(번들 테마 목록은 `taide_runtime::theme_actions::theme_list(&state)`(`CR/taide-runtime/src/theme_actions.rs` 8)로 얻습니다. 사양의 "36개"는 옛 값이고 `CR/taide-theme/resources/themes/`에는 47개 파일이 있으므로 수를 고정하지 않습니다).

#### commands-ui-68 button(variant 6종 × size 8종)

- (a) partial.
- (b) 원본 `TS/shared/ui/button.tsx` 7~35(cva). 공통: `inline-flex gap-2 rounded-md text-sm font-medium`, 비활성 `opacity-50`·`pointer-events-none`, focus-visible 테두리 `ring` + 3px 링 `ring/50`(destructive는 `ring-destructive/20`), 아이콘 16px.
    - variant(12~17): default(`bg-primary text-primary-foreground hover:bg-primary/90`), destructive(`bg-destructive text-white hover:bg-destructive/90`), outline(`border bg-background shadow-xs hover:bg-accent hover:text-accent-foreground`), ghost(`hover:bg-accent hover:text-accent-foreground`). `secondary`·`link`는 소비자 없음(2026-11-04 `variant='…'` 집계: outline 61, ghost 21, destructive 12, default 2).
    - size(20~27): default `h-9 px-4 py-2 has-[>svg]:px-3`, xs `h-6 gap-1 px-2 text-xs has-[>svg]:px-1.5`·아이콘 12px, sm `h-8 gap-1.5 px-3 has-[>svg]:px-2.5`, icon-xs `size-6`·아이콘 12px. `lg`·`icon`·`icon-sm`·`icon-lg`는 소비자 없음(`<Button … size=` 집계: xs 44, sm 13, icon-xs 2).
    - TS 소비자 46개 파일(`grep -rlE "@shared/ui/button'" TS --include='*.tsx'`): conflict-banner, git commit-box·create-tag-dialog, vsix-import-grammars-section, preview hwp·pdf·status·unsupported, open-project-by-path-dialog, project-display-dialog, project-group-dialog, settings(ai-omlx-row, ai-provider-token-row, font·language·option-picker, keybinding-row, lsp-server-status-list, remote-allowed-hosts-row, remote-password-row, remote-section, sync-section), welcome-screen, alert-dialog, dialog, error-boundary, claude-diff-pane, editor-pane, git-panel-container, keybindings-editor, plugin-install-button, plugin-manager, vsix-import-dialog, agent-cli-status-row, agent-hooks-project-row, settings-view의 appearance·keymap·language·notification·performance·snippets section과 settings-view, new-snippet-file-dialog, snippet-editor, terminal-session, theme-editor.
- (c) native 현재: 원본과 같은 그림은 `UI/snippet-editor.rs`의 `ButtonStyle{Primary, Outline, Ghost, Destructive}`(88), `paint_button_focus`(174, 키보드 focus일 때만 링), `button`(1452)과 `UI/settings-dialogs.rs`의 `Style`(104)·`button`(134, focus 링 없음)에 따로 있습니다. 나머지는 egui 기본 `Button`(전역 visuals `button.background`)입니다. egui 기본 또는 개별 그림을 쓰는 비검사 파일(2026-11-03 `grep -rlE "egui::Button::new|ui\.button\(|Button::new\(" UI APP` 결과): `APP/delete_dialog.rs`, `APP/editor-markdown.rs`, `APP/explorer.rs`, `APP/file-loading.rs`, `APP/open_path_dialog.rs`, `APP/search-panel.rs`, `APP/terminal_surface.rs`, `UI/conflict_banner.rs`, `UI/editor-ai-edit.rs`, `UI/editor-context-menu-widget.rs`, `UI/error-boundary.rs`, `UI/keybinding-editor.rs`, `UI/project-display-dialog.rs`, `UI/project-group-dialog.rs`, `UI/settings-dialogs.rs`, `UI/settings-sections-view.rs`, `UI/settings-view.rs`, `UI/shell.rs`, `UI/snippet-editor.rs`, `UI/theme-editor.rs`, `UI/toast.rs`, 그리고 `APP/close_dialog.rs`(`ui.button(` 줄바꿈 형태), `APP/preview_hwp_surface.rs`·`APP/preview_pdf_surface.rs`·`APP/status-editor.rs`의 지역 `fn button`. git lane은 `git-gutter.rs`·`git-scm.rs`·`git-graph.rs`. 이 중 원본에서 shadcn `Button`이 아닌 것(예: `status-error-banner`의 밑줄 `<button>`, `editor-context-menu-widget`의 Monaco 메뉴, toast 내부 버튼, `icon-button.tsx`를 쓰는 툴바)은 대상이 아닙니다. 각 파일이 대응하는 TS 화면이 `@shared/ui/button`을 쓰는지 2.6절 방법으로 확인한 목록을 QA에 남깁니다.
- (d)
    1. 새 `UI/ui-button.rs`(`#[path = "ui-button.rs"] pub mod ui_button;`): `pub enum Variant { Default, Destructive, Outline, Ghost }`, `pub enum Size { Default, Xs, Sm, IconXs }`, `pub struct Spec<'a> { pub label: Option<&'a str>, pub icon: Option<IconRef>, pub variant: Variant, pub size: Size, pub enabled: bool }`, `pub fn show(ui: &mut Ui, spec: Spec) -> Response`. 그림·focus modality·motion 코드는 `snippet-editor.rs`의 `button`·`paint_button_focus`를 옮깁니다. 비활성은 `ui.add_enabled_ui(false, …)` + 50% 투명도, accesskit role `Button`. 색은 `Tokens::get`.
    2. `snippet-editor.rs`·`settings-dialogs.rs`의 버튼을 이 모듈 호출로 바꿉니다(화면 변화 없음, 기존 검사 통과).
    3. 위 목록 중 원본이 shadcn Button인 native 화면을 차례로 바꿉니다. 각 화면의 variant·size는 해당 TS 파일의 `<Button variant=.. size=..>`를 따릅니다(예: close-dirty-tab = 취소 outline(AlertDialogCancel), 버리기 destructive, 저장 default(`TS/features/tab/close-dirty-tab-dialog.tsx` 47~51); 오류 경계 재시도 = outline xs(`TS/shared/ui/error-boundary.tsx` 78)).
- (e) UI 단위(`UI/ui-button-tests.rs`): 4 variant × 4 size의 rect 높이(36/24/32/24)·채움색·테두리·글자 크기(14/12/14/–), hover 채움(primary 90%, accent), 비활성 50%와 클릭 무시, 키보드 Tab focus일 때만 3px 링(포인터 클릭 focus에는 없음). App 1건: close dirty tab 대화상자의 버리기 버튼 채움색 = `statusIndicator.error`(dirty 탭 닫기 요청 뒤 `output.shapes`에서 버튼 rect 채움 확인). 실행 `… --features native-host,inspection --lib -- ui_button`.
- (f) 2.6절 (a)(b) 충족 → complete. 다른 영역 화면(git 등)은 (c)로 비고.
- (g) 없음. 실기 키보드 focus 링 픽셀은 배치 31.

#### commands-ui-69 card

- (a) partial.
- (b) 원본 `TS/shared/ui/card.tsx`(48줄): Card `bg-card text-card-foreground flex flex-col gap-4 rounded-lg border py-5 shadow-sm`(9), Header 좌우 20px·gap 6px, Title `leading-none font-semibold`(29), Description `text-muted-foreground text-xs`(33, 12px), Content `px-5`(41). 소비자 2곳: `TS/features/settings/settings-section.tsx` 11~16(Title 14px, Content 세로 gap 12px), `TS/features/welcome/welcome-screen.tsx` 110~124(Card 위아래 16px, Header·Content 좌우 16px, Title 12px + 아이콘 gap 6px, Content 2열 grid gap 16/6, 12px).
- (c) `UI/settings-view.rs`(`CARD_PADDING` 20, `CARD_GAP` 16, `CARD_RADIUS` 8, `CARD_DESCRIPTION_GAP` 6), Welcome 카드(UI `shell.rs` `welcome` 1951~). `UI/theme-editor.rs`의 `card`는 원본 Card 소비자가 아님(대상 아님). 빠진 것: 공용 모듈과 두 화면 이전.
- (d) `UI/ui-card.rs` `pub struct Padding { pub vertical: f32, pub horizontal: f32 }`, `pub fn show<R>(ui, padding, header: impl FnOnce(&mut Ui), content: impl FnOnce(&mut Ui) -> R) -> R`. Settings section 카드와 Welcome 카드를 이 모듈로 바꿉니다.
- (e) UI 단위(채움·테두리·반경·여백), workbench의 Welcome 검사와 settings 검사에 카드 rect 여백 단언 각 1건.
- (f) complete.
- (g) 없음.

#### commands-ui-79 switch

- (a) partial.
- (b) 원본 `TS/shared/ui/switch.tsx` 6~21: `h-[1.15rem] w-8`(18.4×32), 둥근 끝, 켜짐 `bg-primary`·꺼짐 `bg-input`, thumb `size-4`(16px) `bg-background`, 켜지면 `translate-x-[calc(100%-2px)]`(14px), `transition-transform`, 비활성 50%·`cursor-not-allowed`, focus-visible 3px 링. 소비자 5곳: `TS/features/settings/{remote-section, agent-hooks-toggle, ai-auto-tab-toggle, switch-field, font-picker}.tsx`.
- (c) `UI/settings-view.rs` `switch_lines`(1981~2067)가 원본 모양을 직접 그립니다(`SWITCH_WIDTH` 32(68), `SWITCH_HEIGHT` 18.4(69), thumb 원). 원본과 다른 점: thumb 이동 전환 없음, accesskit `WidgetType::Checkbox`(2044, Switch 역할 아님), focus 테두리 1px(2054~2061, 3px 링 아님). 같은 함수를 `UI/settings-sections-view.rs` 869(settings lane 스위치)가 부르고, 글꼴 Picker의 monospace 스위치(`UI/settings-code-view.rs` 1297~1325)도 있습니다.
- (d) `UI/ui-switch.rs` `pub fn show(ui, checked: bool, enabled: bool, accessible_label: &str) -> Response`(트랙·thumb 그림, `ui.ctx().animate_bool_with_time(id, checked, SWITCH_TRANSITION_SECONDS)`(vendored `vendor/egui-input/src/context.rs` 3852)로 thumb x를 0.15초 전환, 키보드 focus 3px 링). 0.15초는 Tailwind `transition-transform`의 기본 시간입니다(`node_modules/tailwindcss/theme.css` 492 `--default-transition-duration: 150ms`). accesskit 역할: vendored egui `WidgetType`(`vendor/egui-input/src/lib.rs` 623~)에는 Switch가 없으므로 `widget_info`로 `Checkbox`를 넣지 말고, `ui.ctx().accesskit_node_builder(response.id, |node| { node.set_role(egui::accesskit::Role::Switch); node.set_label(..); node.set_toggled(..) })`(context.rs 4322; 선례 `UI/error-boundary.rs` 245~249의 `Role::Alert`, accesskit 0.24.1)를 씁니다. `switch_lines`는 라벨 배치만 하고 컨트롤 그림은 이 모듈을 부르게 바꿉니다. Picker monospace 스위치도 바꿉니다.
- (e) UI 단위(켜짐·꺼짐·비활성 rect·색, 클릭·Space 토글, accesskit `Role::Switch`와 toggled, 전환 중간 프레임 thumb 위치가 두 끝 사이), 기존 settings 검사 통과.
- (f) complete(settings 스위치는 같은 함수 경유라 (b)로 함께 충족).
- (g) 없음.

#### commands-ui-70 checkbox

- (a) partial. 사양은 "의존"으로 분류했지만 두 소비 화면이 이미 병합됐습니다.
- (b) 원본 `TS/shared/ui/checkbox.tsx` 7~21: `size-3.5`(14px), `rounded-xs`(2px), 1px `border-input`, shadow-xs, 체크되면 배경·테두리 `primary`·글자 `primary-foreground`, `CheckIcon` `size-3`(12px), 비활성 50%, focus-visible 3px 링. 소비자 2곳: `TS/shared/ui/file-group-header.tsx` 21(검색 결과 파일 선택), `TS/widgets/plugin-manager/vsix-import-dialog.tsx`(테마 후보).
- (c) 두 화면에 각자 구현이 있습니다: `APP/search-results.rs` 314~363(그룹 머리 checkbox, accesskit 라벨 `search.selectFile`(324), `Icon::Check` 그림(348)), `UI/settings-sections-view.rs` `checkbox`(2001~). 빠진 것: 공용 모듈과 두 화면 이전, focus 링, 비활성 50% 일관성.
- (d) `UI/ui-checkbox.rs` `pub fn show(ui, checked: bool, enabled: bool, accessible_label: &str) -> Response`(accesskit `Checkbox` + toggled, 체크 그림은 lucide `check` path, Space·클릭 토글). 두 화면을 이 모듈로 바꿉니다. 검색 결과 쪽은 84의 공용 머리로 함께 옮겨집니다.
- (e) UI 단위(체크/해제/비활성 색·크기 14px·radius 2px, Space·클릭 토글, 라벨), 두 소비 화면 단언 각 1건(검색 결과 그룹 checkbox 클릭 → 선택 변경, VSIX 후보 토글).
- (f) 2.6절 (b)(c) 충족 → complete.
- (g) 없음.

#### commands-ui-76 progress

- (a) partial. 사양은 "settings S2 뒤"로 분류했고 settings-33이 complete가 됐습니다.
- (b) 원본 `TS/shared/ui/progress.tsx`(21줄): 트랙 `bg-primary/20 relative h-1.5 w-full overflow-hidden rounded-full`, 표시줄 `bg-primary`, `translateX(-(100 - value)%)`. 유일한 소비자 `TS/features/settings/lsp-server-status-list.tsx`(높이 4px로 덮어씀).
- (c) `UI/settings-sections-view.rs` `progress_line`(1250~1326)이 직접 그립니다(`PROGRESS_HEIGHT` 4.0(64), `PROGRESS_TRACK_OPACITY` 0.2(65), 비율 = received/total). accesskit 진행 정보 없음. 빠진 것: 공용 모듈, `ProgressIndicator` 역할과 수치.
- (d) `UI/ui-progress.rs` `pub fn show(ui, value: f32 /* 0..=100 */, height: f32)`(트랙·표시줄, accesskit: vendored `WidgetType::ProgressIndicator`(`vendor/egui-input/src/lib.rs` 623~ enum)로 `widget_info`를 넣고 수치는 `accesskit_node_builder`로 `set_numeric_value`·`set_min_numeric_value(0.0)`·`set_max_numeric_value(100.0)`). `progress_line`의 막대 그림을 이 함수 호출로 바꿉니다(높이 4).
- (e) UI 단위(0·37·100에서 표시줄 폭 비율, 트랙 색, accesskit numeric value), 기존 settings LSP 진행 검사(`UI/settings-sections-view-tests.rs` `progress` helper 278 사용 검사) 통과 + 막대 폭 단언 1건.
- (f) complete.
- (g) 없음.

#### commands-ui-84 file-group-header

- (a) partial.
- (b) 원본 `TS/shared/ui/file-group-header.tsx`(36줄) 19~36: 행 `text-xs`(12px), hover `bg-explorer-item-hover`, 선택 가능하면 `pl-2` + checkbox(70). 본문 `role=button tabIndex=0 aria-expanded`, Enter·Space 토글(`createActivationKeyDownHandler`), `py-0.5 pr-2`, 선택 가능하면 `pl-1` 아니면 `pl-2`, `gap-1`: `ChevronRight` 12px(펼치면 90도), `FileTypeIcon` 14px, 파일 이름 `font-medium` truncate, 경로 muted truncate, 오른쪽 `ml-auto` 개수 muted `tabular-nums`. 소비자: `TS/features/problems/problems-panel.tsx`(선택 없음), `TS/features/search/search-results-list.tsx`(선택 있음).
- (c) 두 화면에 각자 구현: `APP/problems.rs` `show_panel`(341) 안 633~686(chevron·파일 아이콘·개수·이름·경로), `APP/search-results.rs` 306~440(checkbox·chevron·파일 아이콘·이름·경로·개수, accesskit `set_expanded`(377)). 둘 다 APP crate이고 아이콘 그림은 APP의 `PaintIcon`(`APP/explorer.rs` 217)과 `problems_icons::file`을 씁니다.
- (d) 사양은 `UI/file-group-header.rs`를 제안했지만 두 소비자가 모두 APP crate이고 아이콘 경로(`PaintIcon`, `problems_icons`)가 APP에 있으므로 `APP/file-group-header.rs`(`#[path = "file-group-header.rs"] mod file_group_header;`를 `APP/lib.rs` 모듈 목록 끝에)에 둡니다(사양과 다른 점을 QA에 적음). `pub(crate) struct Header<'a> { path: &'a str, name: &'a str, count: usize, expanded: bool, selected: Option<bool>, select_label: Option<&'a str> }`, `pub(crate) fn show(ui, rect, id, header, icons, colors) -> AppResult<Output>`, `Output { toggled: bool, selection_toggled: bool }`. checkbox는 70의 `ui_checkbox::show`. 문제 패널과 검색 결과가 이 함수를 씁니다(두 곳 사용이라 공통화 기준 충족).
- (e) APP 단위 검사(`APP/file-group-header-tests.rs`: 요소 순서·간격 4px·chevron 회전·Enter/Space 토글·accesskit expanded·선택 가능 시 왼쪽 8px), 기존 `problems_tests`·`search_results_tests`·`search_panel_tests` 통과, 검색 결과 checkbox 단언 1건.
- (f) complete.
- (g) 없음.

#### commands-ui-77 scroll-area / overlay-scrollbar / scroll-container / use-overlay-scrollbar

- (a) partial.
- (b) 원본:
    - `TS/shared/scroll/scroll-container.tsx`(60줄): viewport 기본 scrollbar 숨김, 방향별 overlay track. both면 세로 track 아래 10px, 가로 track 오른쪽 10px 비움.
    - `TS/shared/scroll/overlay-scrollbar.tsx`(41줄): track 10px(`w-2.5`/`h-2.5`), 배경 `scrollbar.track`, 기본 투명·스크롤 가능할 때만 포인터, 보일 때 불투명(200ms), thumb 둥근 끝 `scrollbar.thumb`, hover `scrollbar.thumbHover`.
    - `TS/shared/hooks/use-overlay-scrollbar.ts`(213줄): 스크롤하면 보이고 `SCROLLBAR_AUTO_HIDE_MS` 뒤 숨김, track hover·thumb drag 중에는 유지, thumb drag 비례 이동, track 클릭은 thumb 가운데가 클릭 지점에 오도록 부드러운 스크롤.
    - 상수 `TS/shared/constants/scrollbar.ts`: `SCROLLBAR_AUTO_HIDE_MS = 1000`, `SCROLLBAR_MIN_THUMB_PX = 24`, `SCROLLBAR_SCROLL_EPSILON_PX = 1`, `SCROLLBAR_SCROLL_PERCENT_MAX = 100`.
    - `TS/shared/lib/scrollbar-metrics.ts`: `computeScrollbarThumbMetrics`(16), `computeScrollPercent`(29), `computeScrollOffsetForThumbDelta`(44), `computeScrollOffsetForTrackClick`(69). 스크롤 가능 = 남는 길이 > epsilon 그리고 track > 0, thumb = max(최소 24, client/scroll × track)를 track으로 제한, offset 비례. 원본 검사 `TS/shared/lib/scrollbar-metrics.test.ts`(125줄)와 `TS/shared/hooks/use-overlay-scrollbar.test.tsx`(147줄).
    - `TS/shared/ui/scroll-area.tsx`(Radix)는 소비자 없음(대상 아님).
    - 소비자 21개 파일(`grep -rlE "scroll-container'|use-overlay-scrollbar'|overlay-scrollbar'" TS`에서 `shared/scroll/` 2개 제외): markdown-preview, file-tree, outline-panel, preview(hwp·image·pdf·presentation·spreadsheet), problems-panel, search-panel, search-results-list, pane-tab-bar, file-history-panel, commit-graph, git-panel, keybindings-editor, search-editor-pane, settings-view, snippet-editor, system-usage-modal, theme-editor.
- (c) native 대부분은 egui `ScrollArea` 기본 scrollbar(전역 visuals). 편집기만 테마 scrollbar 색(`UI/presentation.rs` 321~322), 미리보기 일부 384~386. 팔레트 목록은 `ScrollBarVisibility::AlwaysHidden`(원본 cmdk `scrollbar-hidden`과 같음). 빠진 것: overlay track·thumb·자동 숨김·drag·track 클릭 규칙 전부.
- (d)
    1. `UI/ui-scroll.rs` `pub mod metrics`: 원본 함수 4개를 snake_case로 그대로 이식한 순수 함수(`thumb_metrics`, `scroll_percent`, `offset_for_thumb_delta`, `offset_for_track_click`)와 상수 4개.
    2. `pub fn show<R>(ui, id: Id, orientation: Orientation, add_contents: impl FnOnce(&mut Ui) -> R) -> egui::scroll_area::ScrollAreaOutput<R>`: egui `ScrollArea`를 `scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)`로 쓰고 `ScrollAreaOutput{state.offset, content_size, inner_rect}`로 overlay track·thumb를 그 위 layer에 그립니다. 보임 상태와 마지막 스크롤 시각은 egui temp data(`(id, "overlay-scrollbar")`), 1초 뒤 숨김은 `ctx.request_repaint_after`. thumb drag는 `ui.interact(thumb, id.with("thumb"), Sense::drag())`, track 클릭은 목표 offset을 `scroll_area::State::load/store`로 적용.
    3. 이전: 위 소비자 중 native에 있는 화면(시스템 사용량 `APP/system-usage-view.rs`, 테마·키바인딩·스니펫 편집기, 탭 바(가로), 문제 패널, 마크다운 미리보기, 탐색기 트리, 미리보기, outline, Settings, 검색 패널·결과·검색 편집기). 편집기 본문은 원본도 Monaco 자체 scrollbar라 대상이 아닙니다.
- (e) UI 단위: 계산식 표(원본 `scrollbar-metrics.test.ts` 사례를 그대로 옮김), 하네스 프레임 검사(스크롤 → 보임 → 1초 뒤 숨김(RawInput `time` 주입), hover 중 유지, thumb drag 비례, track 클릭 중심 이동, 스크롤 불가면 track이 포인터를 받지 않음). 이전한 화면의 기존 검사 통과.
- (f) complete(Git 화면 중 아직 병합 안 된 것은 (c)).
- (g) 없음. 작업량이 커서 묶음 5에서 가장 마지막에 둡니다.

### 3.5 묶음 6: 겹 위젯(묶음 5 뒤)

#### commands-ui-67 alert-dialog

- (a) partial.
- (b) 원본 `TS/shared/ui/alert-dialog.tsx`(Radix AlertDialog, 156줄):
    - 내용 46행: 가운데 고정, `max-w-[calc(100%-2rem)]`, `data-[size=default]:sm:max-w-lg`(512px), `data-[size=sm]:max-w-xs`(320px), 1px `modal-border`, `bg-modal-background`, `rounded-lg`(8px), `p-6`(24px), `gap-4`(16px), `shadow-overlay-lg`, 열림·닫힘 fade + zoom 95%(200ms).
    - Header 59행: 가운데 정렬(넓은 창 default size는 왼쪽), gap 6px. Title 85행 `text-lg font-semibold`(18px). Description 98행 14px muted.
    - Footer 72행: `flex-col-reverse gap-2`, sm 이상 `flex-row justify-end`, size sm이면 2열 grid. Cancel = Button outline, Action = Button default(소비자가 `variant='destructive'` 지정).
    - Radix 동작: 바깥 클릭으로 닫히지 않음, Escape는 닫힘(= 취소), 열리면 Cancel에 focus, focus 가둠, 닫히면 연 곳으로 focus 복귀.
    - TS 소비자 13곳과 native 대응(파일 이름·modal 사용 위치 대조):

| TS 소비자 | native | 소유 |
| --- | --- | --- |
| `features/tab/close-dirty-tab-dialog.tsx` | `APP/close_dialog.rs`(egui `Modal`, 27) | editor/shell |
| `features/explorer/entry-delete-dialog.tsx` | `APP/delete_dialog.rs`(egui `Modal`, 28) | explorer-search |
| `widgets/app-sidebar/sortable-project-group-header.tsx` | `UI/project-group-dialog.rs` `ProjectGroupDeleteDialog`(210) | shell |
| `widgets/snippet-editor/snippet-editor.tsx` | `UI/snippet-editor.rs` | settings |
| `widgets/theme-editor/theme-editor.tsx` | `UI/theme-editor.rs` | settings |
| `features/settings/sync-conflict-dialog.tsx`, `widgets/plugin-manager/plugin-uninstall-dialog.tsx`, `widgets/plugin-manager/vsix-import-dialog.tsx`(덮어쓰기), `widgets/settings-view/agent-hooks-project-row.tsx` | `UI/settings-dialogs.rs` `Alert`(87) | settings |
| `features/search/search-panel.tsx`(바꾸기 확인) | `APP/search-panel.rs` 1374(egui `Modal`) | explorer-search |
| `features/git/hunk-discard-dialog.tsx`, `features/git/conflict-resolution-dialog.tsx` | `GITWT/…/git-gutter.rs` 22 `alert`(egui `Modal`), `GITWT/…/git-editor.rs` 664 | git |
| `widgets/git-panel/git-panel.tsx` | `GITWT/…/git-scm.rs` 1579 | git |

- (c) 원본 규칙을 따르는 구현: `UI/settings-dialogs.rs`(settings-view의 자식 모듈 `#[path = "settings-dialogs.rs"]`(settings-view.rs 23), 항목이 `pub(super)`라 밖에서 못 씀. `Focus`(23), `Returning`(55), `settle`(61), `Size`(82), `Alert`(87), `Choice`(97), `Style`(104), `layer`(110), `content_width`(124, 320/512/뷰포트 −32), `button`(134), `header`(169), `focus_first`(192), `PADDING` 24, `CONTENT_GAP` 16, `ALERT_HEADER_GAP` 6, `modal::Layer` 기반), `ProjectGroupDeleteDialog`. 다른 점: `APP/close_dialog.rs`는 egui 기본 `Modal`이라 바깥 클릭으로 닫히고(85 `if !busy && response.should_close()` → Cancel), 버튼이 egui 기본색, busy면 spinner만(51). `APP/delete_dialog.rs`는 Cancel 첫 focus(47)·Escape 취소(77)는 맞지만 버튼 색·폭(`MAX_WIDTH` 512만, 6)·focus 가둠이 없습니다. git lane `alert`도 egui `Modal`입니다.
- (d)
    1. `UI/ui-dialog.rs`(`#[path = "ui-dialog.rs"] pub mod ui_dialog;`)에 `settings-dialogs.rs`의 `Focus`·`Returning`·`settle`·`Size`·`Choice`·`content_width`·`layer`·`header`·`focus_first`를 공개 API로 옮깁니다(`pub struct AlertDialog { title, description, buttons: Vec<(String, ui_button::Variant)>, size: Size }`, `pub fn show_alert(ctx, id, &mut Focus, &AlertDialog, tokens) -> Option<usize>`). 버튼은 68의 `ui_button`. 내부는 `modal::Layer`(scrim·전환), `modal::trap_focus`, `modal::Composition`, `modal::takes_escape`를 씁니다. 바깥 클릭은 무시합니다.
    2. `settings-dialogs.rs`는 이 모듈을 부르게 바꿉니다(settings 화면 변화 없음, 기존 settings 검사 통과).
    3. 이전: `close_dialog.rs`(바깥 클릭 무시로 바뀜, 원본과 같아짐. 원본 `close-dirty-tab-dialog.tsx`는 busy 상태가 없고 `dirtyTitles.length > 0` 동안 열려 있으므로, native busy 중에는 대화상자를 연 채 버튼만 비활성으로 둡니다), `delete_dialog.rs`, `ProjectGroupDeleteDialog`, snippet·theme 편집기 확인 대화상자, `search-panel.rs` 바꾸기 확인. git 소비자는 병합 뒤 (c)로 확인하고, egui `Modal` 그대로면 이 묶음에서 그리기 함수만 바꿉니다(영역 밖 최소 수정, QA 기록).
- (e) UI 단위(`UI/ui-dialog-tests.rs`, `modal-tests.rs` 하네스 방식): 바깥 클릭 무시, Escape = 취소, 첫 focus Cancel, Tab 순환이 대화상자 밖으로 안 나감, 닫힘 후 focus 복귀, sm 폭 320과 2열 footer, default 폭 512. App 1건(`application-commands-tests.rs`): dirty 탭 닫기 → 대화상자 → 바깥 클릭에도 열린 채, Escape → 탭 유지, 버리기 버튼 destructive 색.
- (f) 2.6절 (a)(b) 충족 → complete. git 소비자는 (c).
- (g) 없음. VoiceOver·키보드 실기는 배치 31.

#### commands-ui-73 dialog

- (a) partial.
- (b) 원본 `TS/shared/ui/dialog.tsx`(126줄): alert와 같은 틀(74행 `max-w-[calc(100%-2rem)]`, `p-6`, `gap-4`, `rounded-lg`, `shadow-overlay-lg`, fade+zoom 200ms)에 더해 바깥 클릭으로 닫힘, Escape 닫힘(단 IME 조합을 취소하는 Escape는 무시 — 42~72, `onEscapeKeyDown`이 `isImeCompositionKeydown(event)`면 무시(66~71)), 기본으로 오른쪽 위 16px(`top-4 right-4`, 82행)에 닫기 X(아이콘 16px, `opacity-70` hover 100%, 접근성 이름 `common.close`(84)). Header `flex flex-col gap-2`(93, 8px), Footer `flex-col-reverse gap-2 sm:flex-row sm:justify-end`(107).
    - TS 소비자 12곳(`grep -rlE "@shared/ui/dialog'" TS --include='*.tsx'`에서 `shared/ui/command.tsx` 제외): system-usage-modal, file-history-panel, keybindings-editor, task-runner-dialog, command-palette, vsix-import-dialog, conflict-compare-dialog, create-tag-dialog, open-project-by-path-dialog, project-group-dialog, project-display-dialog, new-snippet-file-dialog. X를 끄는 것은 팔레트·task runner·파일 이력(`showCloseButton={false}`).
- (c) native: X가 있는 것 = 키바인딩 편집기(`UI/keybinding-editor.rs` 1137~1174 `Icon::Close`), 시스템 사용량(`APP/system-usage-view.rs` 365 `common.close`), Settings VSIX 대화상자(`UI/settings-dialogs.rs` 356~380), 새 스니펫 파일(`UI/snippet-editor.rs` 1274~1296). X가 없는 것 = `UI/project-display-dialog.rs`, `UI/project-group-dialog.rs`(생성·편집), `APP/open_path_dialog.rs`(egui `Modal` 51). git: `GITWT/…/git-graph.rs` 572(태그), `git-history.rs` 163(파일 이력, X 없음이 원본), `git-editor.rs`(충돌 비교, X 있음). IME Escape 규칙은 `modal::Composition`(118)이 있지만 화면마다 적용 여부가 다릅니다.
- (d) `UI/ui-dialog.rs`에 `pub struct Dialog { title, description: Option<String>, show_close: bool, width: f32 }`, `pub fn show_dialog<R>(ctx, id, &mut Focus, &Dialog, tokens, add_contents) -> DialogOutput<R>`(바깥 클릭 닫힘, IME 조합 중 Escape 무시는 `Composition`, X 버튼 accesskit 이름 `common.close`). 이전: 위 X 없는 세 화면(X 추가)과 X 있는 화면 전부. 팔레트는 별도 구현이고 X가 없어 대상 확인만 합니다.
- (e) UI 단위(바깥 클릭 닫힘, IME preedit 중 Escape 무시(`modal-tests.rs`의 preedit 이벤트 방식), X 클릭 닫힘·accesskit 이름, focus 가둠·복귀). App: 프로젝트 표시 대화상자 X로 닫힘, 경로로 열기(`open_path_dialog`) 바깥 클릭 닫힘(기존 `APP/open-path-dialog-tests.rs` 확장).
- (f) complete(git 대화상자는 (c)).
- (g) 없음.

#### commands-ui-75 popover

- (a) partial.
- (b) 원본 `TS/shared/ui/popover.tsx`(50줄) 18~48: 기본 `w-72`(288px), `rounded-md`(6px), 1px `border-popover-border`, `bg-popover`, `p-4`(16px), `shadow-overlay`, `sideOffset` 4, `align='center'`, fade+zoom, 안에 cmdk 입력이 있으면 그 아래선 `popover-separator`, IME 조합 중 Escape 무시(28~33). 소비자 5곳: `features/git/branch-switcher.tsx`, `features/theme/color-picker.tsx`, `features/settings/{font,language,option}-picker.tsx`(각자 폭·안쪽 여백 덮어씀).
- (c) `UI/settings-code-view.rs` Picker(egui `Popup::from_response` 786~793, 구분선 894~901), `UI/theme-color-picker.rs`. 틀 색·반경·그림자·간격 규칙이 각자 있습니다.
- (d) `UI/ui-popover.rs` `pub fn frame(tokens) -> egui::Frame`(채움 `popover.background`, 테두리 `popover.border`, radius 6, 그림자 overlay, inner margin 16), `pub const SIDE_OFFSET: f32 = 4.0`, `pub fn takes_escape(ctx, composition: &Composition) -> bool`(IME 조합 중 false). 두 소비 화면의 popup 틀을 이 모듈로 바꿉니다.
- (e) UI 단위(틀 색·반경·그림자·4px 간격, IME 중 Escape 유지), Picker·색 선택기 기존 검사(`UI/theme-editor-tests.rs`, `native/taide-native-ui/tests/settings-code-controls.rs`) 통과.
- (f) complete(branch switcher는 (c)).
- (g) 없음.

#### commands-ui-71 command(cmdk 목록)

- (a) partial.
- (b) 원본 `TS/shared/ui/command.tsx`(124줄) 10~122: 입력 줄 높이 36px·아래 1px 테두리·왼쪽 검색 아이콘 16px 50%·placeholder muted. 목록 최대 300px, `scrollbar-hidden`. 빈 상태 위아래 24px 가운데 14px. 그룹 안쪽 4px, 머리 12px medium muted(좌우 8px, 위아래 6px). 항목: radius 4px, 좌우 8px·위아래 6px, 14px, gap 8px, 비활성 50%, 선택 행 = 배경 `list.activeBackground` + 글자 accent-foreground + 1px 안쪽 링 `app.accent`(95~106 주석: 두 단서 필수), 아이콘 16px muted. Shortcut: 오른쪽 12px muted `tracking-widest`. cmdk 기본: 방향키 끝에서 순환 안 함. TS 소비자 13곳 중 `loop` 속성을 켠 곳은 없습니다(`grep -rn "\bloop\b"` 결과 없음).
    - TS 소비자(`grep -rlE "@shared/ui/command'" TS --include='*.tsx'`): `widgets/command-palette/command-palette.tsx`와 `features/command-palette/` 그룹 파일 5개, `widgets/task-runner/task-runner-dialog.tsx`, `features/git/{branch-group,branch-switcher}.tsx`, `features/settings/{font,option,language}-picker.tsx`, `features/project/project-display-dialog.tsx`.
- (c) native: `UI/command-palette.rs`(commands-ui-53 complete, 키 처리 `navigation()` 1803~1825: ↑↓, ⌃N/⌃J·⌃P/⌃K, ⌘↑/⌘↓ = 처음/끝, Home/End, Enter, Escape), `UI/settings-code-view.rs` Picker, `UI/project-display-dialog.rs` 아이콘 grid. 같은 행 그림 규칙이 세 파일에 따로 있습니다. branch switcher는 git lane.
- (d) `UI/ui-command-list.rs`: `pub fn paint_item(ui, rect, selected: bool, enabled: bool, tokens)`, `pub fn group_heading(ui, text)`, `pub fn empty(ui, text)`, `pub fn shortcut(ui, rect, text)`, 그리고 팔레트의 `navigation()`을 옮긴 `pub fn navigation(key, modifiers) -> Option<Navigation>`(순환 없음). 세 화면이 이를 씁니다. 팔레트의 동작은 바뀌지 않아야 합니다(commands-ui-53 근거 유지).
- (e) UI 단위(선택 행 두 단서: 배경 + 1px 링, 비활성 50%, 빈 상태, 끝에서 순환 없음), 세 화면 기존 검사(`UI/command-palette-tests.rs`, `UI/project-display-dialog-tests.rs`, settings picker 검사) 통과.
- (f) complete(branch switcher는 (c)).
- (g) 없음.

#### commands-ui-72 context-menu, commands-ui-74 dropdown-menu

- (a) 두 행 partial.
- (b) 원본 `TS/shared/ui/context-menu.tsx`(191줄), `dropdown-menu.tsx`(194줄), 같은 규칙:
    - 내용: 최소 폭 128px, 안쪽 4px, 1px `menu.border`, 배경 `menu.background`, radius 6px, `shadow-overlay`(하위 메뉴 `-lg`), 넘치면 세로 스크롤(scrollbar 숨김), 방향별 2px slide + fade + zoom.
    - 항목: radius 4px, 좌우 8px·위아래 6px, 14px, gap 8px, focus(키·포인터) 배경 `menu.itemHover`, 비활성 50%, inset이면 왼쪽 32px, 아이콘 16px muted. destructive: 글자·아이콘 `statusIndicator.error`, focus 배경 그 색 10%.
    - 하위 메뉴 트리거: 열리면 배경 `menu.itemHover`, 오른쪽 chevron 16px. 구분선: 위아래 4px, 좌우 −4px, 1px `menu.separator`. Shortcut: 오른쪽 12px muted. Label(dropdown만): 좌우 8px·위아래 6px 14px medium.
    - Radix 동작: ↑↓ 이동(끝에서 순환 없음), Home/End, Enter·Space 실행, → 하위 메뉴 열기·← 닫기, Escape 닫기, 글자 typeahead(1초 누적).
    - TS 소비자: context-menu 8곳(`widgets/git-panel/git-panel.tsx`, `widgets/git-panel/commit-graph.tsx`, `widgets/app-sidebar/sortable-project-icon.tsx`, `widgets/app-sidebar/sortable-project-group-header.tsx`, `features/tab/tab-bar-context-menu.tsx`, `features/tab/tab-context-menu.tsx`, `features/explorer/file-tree-context-menu.tsx`, `features/terminal/terminal-context-menu.tsx`), dropdown 4곳(`features/editor/breadcrumb-segment.tsx`, `features/tab/tab-bar-add-menu.tsx`, `features/search/search-history-dropdown.tsx`, `features/project/sidebar-add-project-menu.tsx`).
    - 대상 아님: 편집기 우클릭 메뉴(`UI/editor-context-menu-widget.rs`)는 원본도 Monaco 자체 메뉴입니다.
- (c) native: `UI/shell.rs` `menu_item`(2077)·`enabled_menu_item`(2081)(rail·탭·탭 바·추가 메뉴, egui 기본 `Button`), `APP/terminal_surface.rs` `MenuSearch`(1313)·`menu_item`(1343, accesskit `MenuItem`)·`menu_separator`(1350)·`consume_menu_key`(1359)·`prepare_menu_search`(1378)·`navigate_menu`(1464)(Radix typeahead·방향키 이식, vendored egui 입력 API 사용), `APP/explorer.rs`(탐색기 메뉴), `APP/breadcrumb-menu.rs`, `APP/search-panel.rs`(검색 이력). git: `GITWT/…/git-scm.rs`, `git-graph.rs`. destructive 색·shortcut 칸·inset·하위 메뉴 chevron·typeahead가 화면마다 다르거나 없습니다.
- (d) `UI/ui-menu.rs`:
    - `pub fn frame(tokens, is_sub: bool) -> egui::Frame`, `pub struct Item<'a> { label, icon, shortcut: Option<&'a str>, variant: Variant /* Default | Destructive */, inset: bool, enabled: bool }`, `pub fn item(ui, Item) -> Response`(accesskit `MenuItem`), `pub fn separator(ui)`, `pub fn label(ui, text)`, `pub fn submenu(ui, label, add_contents)`.
    - 키보드: `terminal_surface.rs`의 `MenuSearch`·`prepare_menu_search`·`navigate_menu`·`consume_menu_key`를 이 모듈(UI crate)로 옮겨 모든 메뉴가 같은 typeahead·이동을 씁니다. 이 함수들은 vendored egui 전용 API를 쓰므로 UI crate도 같은 vendored egui(`[patch.crates-io]`)를 쓰는지 확인합니다(이미 같음, 2.1절). 동결 remote-web Wasm check로 UI crate 이동을 확인합니다. 터미널 메뉴는 이 모듈을 부르게 바꿉니다.
    - 이전: 위 native 소비자 전부.
- (e) UI 단위(destructive 색·focus 배경 10%, shortcut 오른쪽 정렬, 구분선 색, Label, 하위 메뉴 →/←, typeahead 1초 누적, 비활성 건너뜀, 끝에서 순환 없음). 각 메뉴의 기존 검사, 특히 `APP/terminal_surface.rs` 안의 터미널 메뉴 typeahead 검사(5268행 이후 `#[cfg(test)] mod tests`)가 옮긴 모듈로 그대로 통과해야 합니다.
- (f) 두 행 complete(Git 메뉴·검색 이력은 (c)).
- (g) 없음.

### 3.6 묶음 7: 전역·재적재·감사

#### commands-ui-1 `window.reload`

- (a) missing.
- (b) 원본 `TS/shared/lib/command-catalog.ts` 20 `{ id: 'window.reload', titleKey: 'app.reloadWindow', categoryKey: KEYMAP_CATEGORY.WINDOW, run: () => window.location.reload() }`. 활성 조건 없음(메인·보조 창 모두), 키 없음. 실행한 창의 webview만 다시 읽습니다. Rust 상태(레이아웃, pty, LSP 세션, hot-exit mirror)는 유지되고 화면이 백엔드에서 다시 조회됩니다(사양 2.1). 사라지는 것: React 상태인 확인 대화상자·팔레트·메뉴·popover·tooltip·toast(그 창 toaster), 장애 경계 fallback(새로 mount). 남는 것: wait marker(`sessionStorage`, `TS/entities/agent/agent-wait-marker-registry.ts` 70~85). 제목 locale 키 `app.reloadWindow`("Reload Window").
    - 결정 1 A와 원본의 차이: 원본은 Monaco model을 다시 만들므로 undo 기록이 사라지지만, 결정 1 A는 편집 문서·undo·view 상태를 유지합니다(사양 2.1 단계 5). QA에 결정 차이로 적습니다.
- (c) 카탈로그 행만 있고 `Execution::Unavailable`이라 팔레트에서 비활성 행입니다. `APP/command-dispatch.rs` 검사 unavailable 목록 809에 `"window.reload"`. 재사용할 자산:
    - 팔레트 닫기 `Palette::dismiss`(`UI/command-palette.rs` 501, native-host). 키바인딩 편집기는 닫기가 private `fn close(&mut self, context)`(`UI/keybinding-editor.rs` 558)뿐이라 공개 메서드가 필요합니다.
    - 대기 대화상자: `pending_tab_close`(close dirty, `APP/application.rs` 245), `explorer_search_state.pending_entry_delete`(삭제 확인, `APP/domains/explorer_search.rs` 163), `shell_state.system_usage.detail_open`(시스템 사용량, 닫기 선례 `APP/domains/shell.rs` 885). `pending_disk_choice`(`APP/application.rs` 226)는 대화상자가 아니라 디스크 선택 뒤 진행 중인 host 요청(`APP/domains/editor.rs` 4848 부근)이므로 건드리지 않습니다(사양 2.1 단계 1과 다름).
    - popup: `egui::Popup::close_all(ctx)`(viewport별 popup 상태라 현재 viewport만 닫힘).
    - tooltip: `UI/tooltips.rs` `Provider`(86)는 viewport별 `Viewport { open, widgets, layers, skip_until, is_delayed, … }`를 가집니다. 공개 초기화 API는 없습니다. `begin_frame`(895)의 "context가 바뀐 viewport" 분기(959~974: layers 해제 목록에 넣고 `widgets.clear()`, `open = None`, `skip_until = None`, `is_delayed = true`)가 원본 tooltip 재mount와 같은 초기화입니다.
    - toast: `ToastLayers`(`APP/toast-layers.rs`)는 창별 toaster라 실행한 창의 toaster만 비웁니다. UI `Toasts`(326)에는 전체 비우기 메서드가 없고, 앱 `retain`(47)은 창 단위 제거입니다.
    - 장애 경계 초기화 API는 없습니다(`UI/error-boundary.rs`에 `reset` 계열 없음).
    - 표시 재적재: `settings_state.presentation_refresh.retry()`(`APP/presentation-refresh.rs` 169)가 다음 프레임 재요청을 만듭니다(배너 재시도와 같은 경로).
    - 데이터 재조회: 팔레트 파일 색인 `commands_state.palette_files.invalidate(&project)`(`APP/command-palette.rs` 137), 탐색기 트리 `HostCommand::ExplorerSearch(Command::RefreshTree { project, dirs: None, origin: RefreshOrigin::Toolbar })`(툴바 새로고침과 같은 형태, `APP/domains/explorer_search.rs` 2192~2199), Settings 카탈로그·자원 `invalidate_catalog`·`refresh_resources`(`UI/settings-view.rs` 301·318), 제목 branch `git_state.title_branches.invalidate(projects)`(`APP/title-branch.rs` 52), 미리보기 `invalidate_all`(`APP/preview.rs` 226), Welcome 최근 목록(`APP/welcome.rs` `State`(34)는 `loaded_for`(snapshot 세대)가 같으면 다시 읽지 않으므로 `loaded_for = None`으로 되돌리는 메서드가 필요).
- (d) 구현(결정 1 A):
    1. `UI/command-registry.rs`: `#[cfg(feature = "native-host")] ReloadWindow`를 `Run` 끝에, `execution()`에 `#[cfg(feature = "native-host")] "window.reload" => Some(Run::ReloadWindow),`. 활성 `Always`(기본).
    2. `UI/intents/commands.rs`: `#[cfg(feature = "native-host")] ReloadWindow`.
    3. `APP/shell_keymap.rs` 첫 match: `Run::ReloadWindow => return Some(ShellIntent::Commands(intents::commands::Intent::ReloadWindow)),`.
    4. `UI/error-boundary.rs`: 창 단위 초기화. viewport epoch 방식: `const EPOCH_KEY: &str = "native-error-boundary-epoch";`를 두고 `show_instance`의 상태 id(115)를 `egui::Id::new((STATE_KEY, region, instance, ui.ctx().viewport_id(), epoch))`로, 내용 `id_salt`(137)를 `(region, instance, epoch, state.generation)`으로 바꿉니다(`epoch`는 `ui.data(|d| d.get_temp::<u64>(Id::new((EPOCH_KEY, viewport_id)))).unwrap_or(0)`). `pub fn reset_viewport(ctx: &egui::Context, viewport: egui::ViewportId)`는 그 epoch를 1 올립니다. 실패 상태와 영역 내부 egui 상태(scroll·popup)가 모두 새로 시작합니다(원본 새 mount). `inject_panic`의 키(`INJECTION_KEY`)는 epoch와 무관하게 둡니다.
    5. `UI/keybinding-editor.rs`: `pub fn dismiss(&mut self, context: &egui::Context) { if self.open { self.close(context) } }`(필드 이름은 `is_open`(517)의 본문으로 확인).
    6. `UI/tooltips.rs`: `pub fn reset_viewport(&self, context: &Context)` — 그 viewport 항목에 대해 959~974 분기와 같은 초기화를 하고, 해제할 layer는 1007~1010처럼 `unregister_dismissal_layer`·`set_transform_layer(.., IDENTITY)`를 부릅니다. (`APP/tooltips.rs`는 test에서 UI 소스를 `include!`하므로 이 추가가 두 컴파일 단위에 모두 들어갑니다.)
    7. `UI/toast.rs`: `pub fn clear(&mut self)` — `entries`·`actions`·`bounds`·`hit_targets`를 비우고 `hovered`·`interacting`·`expanded`·`focus_within`을 false, `previous_focus`·`last_owned_focus`·`swipe_owner`를 None으로(원본 toaster 재mount). `assets`·`next_id`·`is_reduced_motion`·`position`·`mount`는 유지. `APP/toast-layers.rs`: `pub(crate) fn clear(&mut self, viewport: ViewportId)`(ROOT면 `main`, 아니면 그 `auxiliary` 항목의 `clear()`).
    8. `APP/welcome.rs`: `pub(crate) fn invalidate(&mut self) { self.loaded_for = None; }`(shell 영역 파일 최소 추가, QA 기록).
    9. `APP/domains/commands.rs` `apply_commands_intent`에 `Intent::ReloadWindow => self.reload_window(context)`와 `fn reload_window(&mut self, context: &egui::Context)`:
        1. `let viewport = context.viewport_id();`
        2. 일시 UI 닫기: 팔레트·키바인딩 편집기가 이 viewport의 overlay이면(`self.shell_state.overlay_viewport == viewport`) `palette.dismiss`·`keybindings.dismiss`, `egui::Popup::close_all(context)`, `self.tooltips.reset_viewport(context)`, 대기 대화상자 취소(`pending_tab_close`는 close 대화상자의 Cancel 처리와 같은 함수, `pending_entry_delete`는 Escape와 같은 처리 — 각 영역의 `pub(crate)` 메서드를 찾아 부르고 없으면 그 영역 파일에 최소 추가), 메인 창이면 `shell_state.system_usage.detail_open = false`(shell의 닫기 경로와 같게). git·settings 대화상자 상태(태그·hunk·충돌·sync 충돌 등)는 그 영역 상태라 병합 뒤 같은 방식으로 각 영역 닫기 메서드를 부릅니다.
        3. 장애 경계 초기화: `taide_native_ui::error_boundary::reset_viewport(context, viewport)`.
        4. 그 창 toaster 비우기: `self.toasts.clear(viewport)`.
        5. 표시 재적재: `self.settings_state.presentation_refresh.retry()`. 실패하면 기존 배너(shell-94)가 뜹니다.
        6. 데이터 재조회: 그 창의 프로젝트들(메인은 `snapshot.layouts`의 열린 프로젝트, 보조 창은 `WindowScope::Auxiliary{project}`)에 대해 (c)의 무효화·재조회를 부르고, 메인 창이면 `shell_state.welcome.invalidate()`.
        7. 유지: 편집 문서·undo·view 상태, 터미널 세션과 scrollback, LSP 세션, 레이아웃, 창 위치·크기는 손대지 않습니다.
    10. 검사 목록 갱신: `APP/command-dispatch.rs` 809에서 `"window.reload"` 제거, 무프로젝트 수락 목록에 추가. UI 실행 경로 표 검사에 `Execution::Native(Run::ReloadWindow) => None` arm.
- (e) 검사:
    - UI 단위: `error_boundary` 검사에 `reset_viewport는_그_창의_fallback만_지우고_다른_창은_유지한다`(두 viewport id로 `show_instance`를 그려 각각 panic 주입 → 한쪽 reset → 그쪽만 정상), `toast` 검사에 `clear는_항목과_동작을_비우고_위치와_motion_설정을_유지한다`, tooltip 검사에 `reset_viewport` 뒤 `inspection_open(context)`(542)이 비어 있음.
    - UI registry: `window.reload`가 native-host에서 `Run::ReloadWindow`·`Always`, 아니면 `Unavailable`.
    - App(`application-commands-tests.rs`): 문서를 dirty로 만들고, 상태 표시줄 panic 주입(`inject_panic(&context, Region::StatusBar)`) 뒤 fallback 확인, error toast 1개를 띄워 둔 상태에서 `run_palette(.., "window.reload")` → 팔레트 닫힘, fallback 사라지고 상태 표시줄 정상(`application-error-boundary-tests.rs` `assert_fallback`의 반대 단언), `visible_toasts` 비어 있음, 문서 dirty·본문 유지, 터미널 세션 id 유지. 이어서 팔레트를 Files 모드로 다시 열면 `commands_state.palette_files.view(Some(&project), None).is_refreshing`이 참(무효화 뒤 재요청; reload 없이 60초 안에 다시 열면 거짓인 대조군 포함). 표시 재적재는 `application-explorer-views-tests.rs` 314 검사 방식으로 테마 파일을 깨 배너를 띄운 뒤 파일을 고치고 `window.reload` → 재시도 버튼 없이 배너 사라짐. 보조 창에서 `run_auxiliary_palette`로 실행하면 메인 창 팔레트·fallback은 그대로.
    - 실행: `… --lib -- domains::commands::tests::실제_app_window_reload command_dispatch::tests --test-threads=2`, UI `--lib -- error_boundary command_registry toast::tests tooltips`, `--test workbench`(장애 경계 id 변경 회귀).
- (f) 위 검사 통과 → complete.
- (g) 없음. git·settings 대화상자 취소는 git 병합 뒤 확인합니다.

#### commands-ui-82 error-boundary(구역 단위 크래시 격리, 재시도, 포커스 이동)

- (a) missing(비고가 사실과 다름).
- (b) 원본 `TS/shared/ui/error-boundary.tsx`(126줄) 32~126: `getDerivedStateFromError`로 그 subtree만 fallback, `componentDidCatch`에서 `console.error`·`onCaught`, fallback `role='alert' tabIndex={-1}` 가운데 정렬(영역 라벨 `labelKey`, `errorBoundary.description`("Something went wrong."), 개발 빌드면 stack `pre` 10px 최대 128px, 재시도 Button outline xs `common.retry`(78~80)), fallback으로 바뀌면 그 컨테이너에 focus, 재시도는 새 mount. 경계 8곳(`QA/2026-10-27-native-error-isolation-file-drop.md` shell-12 절): 루트(`errorBoundary.app`), Welcome, Activity Bar, 상태 표시줄, 사이드바 패널(메인·보조), 편집기 영역(메인·보조, `errorBoundary.editorArea`).
- (c) shell-12(partial)와 같은 기능입니다. `UI/error-boundary.rs`: `Region{Welcome, ActivityBar, StatusBar, SidebarPanel}`(42~47), `show_instance`(107~, `catch_unwind`(183), `request_discard`(154), 직전 frame에 안 그려진 영역은 새 mount(`is_unmounted_while_hidden` 63), 사이드바 패널은 원본처럼 접어도 실패 유지), 검사 workbench 8건·App 4건(`APP/application-error-boundary-tests.rs`). 남은 것(기능표 shell-12 비고): 편집기 영역(메인·보조) 경계(편집기 render가 입력 처리 중 공유 `EditorStore`를 직접 바꿔 render 밖 트랜잭션 설계가 먼저 필요), 루트 `errorBoundary.app` 경계, 픽셀 대조. 경계 밖 panic은 winit `stop_app_on_panic`으로 앱 종료 후 hot-exit·세션 복원(shell-25).
- (d) 이 영역이 할 일:
    1. 묶음 0: 기능표 82 비고를 shell-12 근거 문장으로 바꾸고 상태를 partial로 올립니다(남은 것: 편집기 영역·루트 경계, wave 4).
    2. 묶음 7: 1의 `reset_viewport`가 이 경계를 씁니다(1의 검사가 경계 초기화도 덮음).
    3. wave 4(메인, 배치 31 "편집기 장애 경계 설계")에서 `Region::EditorArea`·`Region::App`이 추가되면 그 검사 근거로 complete.
- (e) 묶음 0은 기존 검사 재실행(`--test workbench -- 경계`, 앱 `--lib -- error_boundary_tests --test-threads=2`).
- (f) wave 4 뒤 shell-12와 함께 complete.
- (g) wave 4 편집기 입력 트랜잭션 구조 변경(메인 작업). 이 영역은 직접 구현하지 않습니다.

#### commands-ui-86 테마 토큰 → 전역 스타일(CSS 변수 `:root`, `@theme`)

- (a) partial.
- (b) 원본 `TS/shared/styles/global.css`(467줄): `:root` 토큰 6~157, `@theme inline` 159~296(shadcn 별칭 138~155 포함), base 298~346(`body` 13px(316), 시스템 글꼴 스택, 앱 배경·글자, `user-select: none`(318), webkit scrollbar 10px·thumb 둥근 끝, reveal 게이트(321)).
- (c) `UI/presentation.rs` `visuals(theme)`(143~225)가 전역 egui `Visuals`를 테마에 연결합니다(위젯 상태별 채움·테두리, `list.activeBackground` 선택, `app.accent` 링크, 입력 배경, warn/error 색, panel·window 채움, `menu.*`, 반경 6/8, 그림자 overlay·dialog). `apply_visuals`(227~229)가 모든 style에 적용합니다. 검사 workbench `테마_토큰은_원본_css변수의_egui_visuals로_변환되어_두_테마_슬롯에_적용된다`(1018). 글꼴 face는 `APP/ui-fonts.rs`. 빠진 것: 공용 토큰 묶음(2.5절)이 없고, 전역 본문 크기(`Style::text_styles`의 Body)가 egui 기본값으로 원본 13px과 다릅니다(`grep -rn "text_styles" UI APP` 결과 없음).
- (d)
    1. 묶음 5 첫 단계에서 `UI/ui-tokens.rs`(2.5절).
    2. 묶음 7: 결정 7 A — `UI/presentation.rs`에 `pub const BODY_FONT_SIZE: f32 = 13.0;`과 `pub fn apply_text_styles(context: &egui::Context)`(모든 style의 `text_styles`에서 `TextStyle::Body`를 `FontId::proportional(BODY_FONT_SIZE)`로)를 두고 `apply_visuals` 호출 두 곳(`APP/application.rs` 395, `APP/domains/settings.rs` 1263) 옆에서 부릅니다. 나머지 크기는 프리미티브가 명시합니다. `user-select: none`은 egui 기본(라벨 선택 불가)과 같습니다.
- (e) workbench 1018 검사에 `Tokens::new` 전 항목 값 단언 추가, UI 단위 `apply_text_styles` 뒤 `ctx.style().text_styles[&TextStyle::Body].size == 13.0`, 앱 presentation 검사에 모든 번들 테마(`theme_list`)에서 `Tokens::new` 성공.
- (f) complete.
- (g) 없음. 주의: Body 크기를 바꾸면 `TextStyle::Body`로 크기를 재는 화면(`UI/shell.rs` 841·852·2449, `UI/editor-completion.rs` 1199)의 rect 단언 검사가 바뀔 수 있습니다. 바뀐 값이 원본(13px)에 가까워졌는지 확인하고 검사 기대값을 고칠 때 근거를 QA에 적습니다(기대값만 맞추는 수정 금지).

#### commands-ui-87 global.css 보조(reveal 게이트, modal-scrim, shadow-overlay, gutter/blame 클래스, xterm 보정)

- (a) partial.
- (b) 원본 `global.css` 321~467과 native 대응:

| 원본 | native 대응 | 상태 |
| --- | --- | --- |
| reveal 게이트 `html[data-theme-ready][data-locale-ready] body`(321~323) | commands-ui-90(complete, 첫 프레임 테마·언어 검사) | 완료 |
| `shadow-overlay` 0 2px 8px(348~350), `shadow-overlay-lg` 0 8px 24px(352~354) | `UI/presentation.rs` 28~31 `OVERLAY_SHADOW_*`·`DIALOG_SHADOW_*`, 212~220 | 있음(workbench 1018이 값 단언) |
| `modal-scrim` 그림자색 50%(356~358) | `UI/modal.rs` 15 `SCRIM_OPACITY` 0.5, `Chrome::scrim`(44~45) | 있음(단언 추가 필요) |
| `scrollbar-hidden`(360~369) | 팔레트 `ScrollBarVisibility::AlwaysHidden`, 77 | 77 |
| `.taide-gutter-*`(371~408), `.taide-blame-overlay-text`(410~414), `.taide-conflict-*`(416~442), `.taide-diff-gutter-stage`(424~446) | git-56~63(편집기 gutter·blame·충돌, `GITWT/…/git-gutter.rs`·`git-blame.rs`·`git-conflict.rs`) | git lane complete, 병합 중 |
| `.xterm textarea`·`.xterm-viewport`(448~457), `.monaco-tl-twistie`(465~467) | 웹 구현 전용 | 비고에 not-applicable 항목으로 |
| `[data-sonner-toaster].taide-toaster-middle`(459~463) | `UI/toast.rs` 세로 Middle 위치 | 있음 |

- (c) 빠진 것: git 병합과 scrim 값 단언.
- (d) 코드 변경 없음. git 병합 뒤 표의 git 항목을 git-56~63 근거로 적습니다.
- (e) UI 단위 1건(`modal-tests.rs`): `Chrome::scrim(1.0)` = 그림자색의 `gamma_multiply(0.5)`. git 항목은 `GITWT/…/application-git-editor-tests.rs` 270·517 근거(병합 뒤 재실행 `… --lib -- editor_tests --test-threads=2`).
- (f) git-56~63 병합 + 단언 통과 → complete.
- (g) git 병합.

#### commands-ui-96 사용자에게 보이는 비국제화 문자열

- (a) partial.
- (b) 원본 기준: 화면 문구는 모두 locale 카탈로그(`CR/taide-locale/resources/locales/{en,ko,ja}.json`). 예외로 원본도 고정 영어: 로케일 로드 실패 배너 문구, Monaco hover "Loading...", 브랜드 "TAIDE". 백엔드 오류는 `describeIpcError`(`TS/shared/lib/ipc-error-message.ts` 26~29)라 Localized가 아니면 원문 메시지를 그대로 보입니다.
- (c) native:
    - UI 리터럴: 비검사 코드의 화면 리터럴은 `UI/shell.rs` 1964 `ui.heading("TAIDE")`(브랜드)와 `UI/editor-documentation.rs` 869 `ui.label("Loading...")`(원본 Monaco와 같음)뿐입니다(`grep -rnE '\.(label|button|heading|small_button|selectable_label)\(\s*"[A-Za-z]|RichText::new\(\s*"[A-Za-z]' UI APP` 결과 중 `#[cfg(test)]` 모듈 안 것 제외: `UI/toast.rs` 1611 이후, `APP/terminal_surface.rs` 5264 이후, `APP/zen.rs` 183 이후, `APP/ui-fonts.rs` 288 이후).
    - 오류 문구: `UI/toast.rs` `describe_error`(1589~1610)가 `Localized`가 아니면 원문을 그대로 돌려줍니다. native 전용 오류(`"native …"` 문자열로 만드는 곳 225개, `grep -rnE 'AppError::(Forbidden|Internal|InvalidArgument|NotFound|Io)\((format!\()?"native' UI APP | grep -v tests | wc -l`)가 toast·status로 가면 원본에 없는 영어 문구가 보입니다.
    - `status: Option<String>` 상태 표시줄 라벨 대입이 비검사 앱 코드 97곳(2.3절, git 병합 뒤 105곳). 이 영역 파일 `APP/domains/commands.rs`에 5곳(230, 335, 351, 388, 450).
- (d) settings-78 결정 3 A(채택, settings 영역은 0건으로 처리 완료)와 같은 규칙: TS 대응이 `runCommandSafely`·`toast.error(describeIpcError)`인 실패는 `toasts.ipc_error`, TS 대응이 없는 native 내부 실패(큐 가득 참, surface 미연결, registry 적재 실패 등)는 화면에 띄우지 않고 `log::warn!`.
    1. 이 영역 5곳을 위 규칙으로 바꿉니다: 230(키바인딩 편집기 `show` 오류 = native 내부) → `log::warn!`, 335(팔레트 `show` 오류 = native 내부) → `log::warn!`, 351(code lens 실행 실패, 원본은 Monaco command 실패 — 원본 `TS/shared/lib/lsp/command-relay.ts`의 실패 처리를 확인해 toast면 `toasts.ipc_error`, 무알림이면 `log::warn!`), 388(들여쓰기 적용 실패 = native 편집 오류) → `log::warn!`, 450(selection range 요청 실패) → `log::warn!`. 로그 문구에는 오류 종류(`error.kind()`)만 남기고 경로·본문은 넣지 않습니다(126 선례).
    2. 문자열 감사 검사: 앱 실행이 필요 없는 단위 검사(새 `APP/string-audit-tests.rs`를 `APP/lib.rs`에 `#[cfg(test)] #[path = "string-audit-tests.rs"] mod string_audit_tests;`로 연결)에서 `std::fs::read_dir`로 `UI`·`APP`의 비검사 `.rs`(파일명이 `-tests.rs`·`_tests.rs`로 끝나지 않는 것)를 읽고, 열 0의 `#[cfg(test)]` 또는 `#[cfg(all(test` 줄 다음에 `mod `가 오는 지점 이후를 버린 뒤, 위 정규식에 해당하는 리터럴이 허용 목록(`"TAIDE"`, `"Loading..."`)뿐인지 단언합니다. 경로는 `env!("CARGO_MANIFEST_DIR")` 기준(`APP` = `<manifest>/src`, `UI` = `<manifest>/../taide-native-ui/src`). 정규식 crate를 새로 넣지 않고 문자열 검색(`find(".label(\"")` 등)으로 구현합니다.
    3. 나머지 영역의 status 92곳(git 병합 뒤 100곳)·native 오류 문구는 각 영역 소유입니다. 기능표 settings-78 비고가 "메인 통합에서 같은 결정으로 처리, 0건이 되면 상태바 라벨과 `status` 필드 제거"라고 적었으므로, 96은 메인 통합 작업이 0건을 만든 뒤 complete로 하고 그 전에는 partial(남은 수 기록)입니다.
- (e) 감사 검사 통과, 대표 경로 1건(App: 호스트 큐를 가득 채운 상태(`HOST_COMMAND_CAPACITY` 64개 제출)에서 팔레트 Files 모드 → 화면 문구·status 없음).
- (f) 감사 검사 통과 + status 대입 0건 → complete.
- (g) 다른 영역 status 대입 정리(메인 통합).

#### commands-ui-98 앱 전반 lucide 아이콘(소스 약 91개 파일, 고유 약 100종)

- (a) partial.
- (b) 원본: `from 'lucide-react'`를 쓰는 비검사 `.ts`·`.tsx` 파일 91개(2026-11-04 `grep -rl "from 'lucide-react'" TS --include='*.ts' --include='*.tsx' | grep -v "\.test\." | wc -l`). 고유 import 이름은 158개(2026-11-03 집계, `XxxIcon` 별칭 형태와 프로젝트 아이콘 56종 포함). 기능표의 "약 100종"과 사양의 "약 120종"은 추정값입니다.
- (c) native: lucide SVG를 `RES/{icons, problems, keybindings, snippets, status, themes, project-icons, toasts}`에 두고 `UI/glyph-icons.rs` `Glyph`, `UI/icons.rs` `Icon`(19), `APP/navigation-icons.rs` `Icon`(13)이 래스터화합니다. 탐색기 툴바 4개는 lucide path를 직접 그립니다(`APP/explorer_toolbar.rs` `button` 20). 고지는 각 폴더 `LICENSE.txt`. `RES/icons/`에는 2026-11-04 기준 34개 SVG가 있고(bell, case-sensitive, check, external-link, eye-off, import, refresh-cw, regex, replace-all, whole-word 등 explorer-search·settings가 추가한 것 포함), ChevronDown SVG는 두 트리 모두 없습니다(`find /Users/hyunseokbyun/development/TAIDE/native /Users/hyunseokbyun/development/TAIDE-lanes/git/native -name "chevron-down*" -not -path "*/target*"` 결과 없음; 원본 사용처 `TS/features/settings/{language,option,font}-picker.tsx`, `TS/widgets/app-sidebar/sortable-project-group-header.tsx`). git worktree는 archive·arrow-down·arrow-left·arrow-up·minus·plus·rotate-ccw·tag·tags·trash-2·undo-2 11개를 추가했습니다(`ls GITWT/native/taide-native-app/resources/icons`).
    - 사양 2.6 표(사양 588~606행)의 영역별 미보유 아이콘: git(GitCompare, History, Archive, Tag, Tags, ArrowUp, ArrowDown, Undo2, Minus, RotateCcw, FileDiff), explorer-search(CaseSensitive, WholeWord, Regex, ReplaceAll, EyeOff, Clock, Scissors, ClipboardPaste, FilePlus, FolderPlus), shell 탭·창 메뉴(Columns2, AppWindow, ArrowLeftToLine, PanelLeft/Right/Top/Bottom, Pin, PinOff), 터미널 메뉴(ClipboardPaste, Eraser, SquareDashedMousePointer, SquareTerminal), 미리보기(ZoomIn, ZoomOut, ChevronLeft, Music, TableProperties, FileWarning, FileQuestion), settings(Bell, ExternalLink, Import, RefreshCw, Type), 공용(Check, ChevronDown). 이 표는 일부가 이미 들어와(위 목록) 구현 시 다시 대조해야 합니다.
- (d)
    1. 이 영역이 직접 더하는 것: 공용 ChevronDown(Picker trigger, 71·75)과 72·74 메뉴 모듈의 하위 메뉴 chevron. lucide `chevron-down.svg`를 `RES/icons/`에 더하고(같은 `LICENSE.txt`) `UI/icons.rs` `Icon`에 변형을 추가합니다. `lucide-static` 패키지는 저장소에 없으므로 path는 `node_modules/lucide-react/dist/esm/icons/chevron-down.mjs`의 `__iconNode`(`["path", { d: "m6 9 6 6 6-6" }]`, lucide-react v1.28.0)에서 옮기고, 외곽 속성은 기존 `RES/icons/check.svg`와 같게 씁니다(`<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>`).
    2. 원본 이름 목록과 native 보유 목록의 대조표를 QA에 만듭니다: 원본 이름 → kebab 파일명(lucide 규칙: `ChevronDown` → `chevron-down.svg`, `Loader2` → `loader-circle`처럼 별칭이 있는 것은 lucide 별칭표 확인) → `find /Users/hyunseokbyun/development/TAIDE/native -name "<파일명>" -not -path "*/target*"` 결과 → 소유 영역.
    3. 검사: `UI/glyph-icons-tests.rs`의 "등록된 SVG가 비지 않은 raster를 만든다" 검사를 `Glyph`뿐 아니라 `Icon`(UI)과 `navigation_icons::Icon`(APP, 앱 crate 검사로)까지 넓힙니다(기존 `rasters` helper 72 재사용).
- (e) `… --features native-host,inspection --lib -- glyph`, 앱 `--lib -- navigation_icons`.
- (f) native에 있는 모든 화면의 아이콘이 lucide 형상(SVG 또는 path 이식)이고, 화면이 없는 행만 비고로 남으면 complete. git·preview·terminal 화면 아이콘이 아직이면 partial(남은 아이콘 목록을 비고에).
- (g) git 병합, preview·terminal-agent 영역의 아이콘 추가.

### 3.7 묶음 8: 편집기 액션

#### commands-ui-43 `monaco.*` 액션 실행(약 157종)

- (a) partial.
- (b) 원본 `TS/shared/lib/monaco/monaco-action-commands.ts` 12~45: `MONACO_ACTIONS`의 각 항목 → `requestEditorPaneCommand({ type: 'run-monaco-action', actionId })`(focused 편집기의 `trigger`). 활성: `MONACO_ACTION2_IDS`(12~34, 21개)는 항상 활성, 나머지는 `context.activeEditorActionIds?.has(actionId)`. 제목 키 `keymap.monaco.<actionId>`, 기본값 `entry.defaultLabel`. TAIDE 사용자 액션 7개는 `registerTaideCustomActionCommands`(48~)가 bare id로도 등록합니다.
- (c) native 현재(2026-11-04 실제 코드 대조): 카탈로그 `monaco.*` 157개(`grep -c '"id": "monaco\.' UI/keybinding-commands.json` = 157) 중 실행 경로가 없는(`Execution::Unavailable`) 것은 아래 25개입니다(각 id가 `UI`·`ED`·`APP`의 비검사 코드에 실행 경로로 나오지 않음을 `grep -rln`으로 확인; `toggleTabFocusMode`·`unicodeHighlight`·`showOrFocusStandaloneColorPicker`·`toggleAlwaysShowToolbar`·`goToLocation`은 `EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE`(25~47)에만 나옴). 나머지 132개는 `UI/command-registry.rs` `execution()`(360~502)의 `line_command`(504)·`cursor_command`(544)·`fold_command`(573)·`IndentationCommand`·`HighlightCommand`, 편집기 crate `completion::Command`·`documentation::Command`(`ED/documentation.rs` 27~39, `ShowHover`·parameter hints)·`symbol_locations::Command`·`problem_navigation::Command`·`formatting::Command`·`indent::Command`, UI `editor_find::FindCommand`·`editor_code_actions::Command`, 특수 id(`showContextMenu`, `codelens.showLensesInCurrentLine`, `rename`, `toggleStickyScroll`, `taide.toggleMinimap`, `taide.saveFile`, `taide.runSelectedTextInTerminal`, `ai.inlineEdit`/`taide.aiInlineEdit`)로 실행됩니다. editor-34(partial)가 같은 기능의 편집기 영역 행이고, 기능표 editor-34 비고가 같은 남은 표를 적었습니다. 둘 중 먼저 하는 쪽이 다른 행 근거를 함께 적습니다.

| id(`monaco.` 생략) | 원본 기본 키 | 원본 동작 | 처리 |
| --- | --- | --- | --- |
| `editor.action.fontZoomIn` / `fontZoomOut` / `fontZoomReset` | 없음 | 창 전역 zoom level ±1/0, 범위 −5~20(`MON/common/config/editorZoom.js` `setZoomLevel` 16~17), 글꼴 크기·줄 높이 × (1 + 0.1 × level)(`MON/common/config/fontInfo.js`), 비영속(`MON/contrib/fontZoom/browser/fontZoom.js`) | 구현: viewport별 zoom level을 앱 상태로 두고 편집기 appearance 계산(`taide_native_ui::presentation::update_editor_font_size` 호출부)에 곱함. 설정 글꼴 크기(⌘=/⌘-, `next_editor_font_size`)와 별개 |
| `editor.action.inPlaceReplace.up` / `.down` | ⇧⌘, / ⇧⌘.(`MON/contrib/inPlaceReplace/browser/inPlaceReplace.js` 113~115·134~136, precondition `writable`) | 선택(없으면 커서 단어)을 다음/이전 값으로: 숫자 증감(소수 자리 유지), 값 집합 `true/false`, `True/False`, `Private/Public/Friend/ReadOnly/Partial/Protected/WriteOnly`, `public/protected/private` 순환(`MON/common/languages/supports/inplaceReplaceSupport.js`) | 구현: `ED/inplace-replace.rs` 새 파일에 `BasicInplaceReplace` 이식, 기존 편집 트랜잭션·undo로 실행, readonly 비활성, 바뀐 범위 짧은 강조는 컨트롤러의 decoration 시간 그대로 |
| `editor.action.toggleTabFocusMode` | ⌃⇧M | Tab이 들여쓰기 대신 focus 이동, 창 전역 토글 | 구현: viewport별 플래그, `UI/editor_surface.rs` Tab 처리에서 켜지면 egui focus 이동 |
| `editor.action.hideHover` | Escape(hover 표시 중) | 문서 hover 닫기 | 구현: `ED/documentation.rs` `Command`(27)에 `Hide` 추가(기존 hover 닫기 경로 재사용), `from_action`(33)에 id 연결 |
| `editor.action.gotoLine` | ⌃G | quick input "줄:열로 이동" | 결정 4 A: 팔레트를 `:` 모드로. `UI/command-registry.rs` `PaletteEntry`(145, 현재 `Files`·`Commands`·`WorkspaceSymbols`)에 `Line` 변형 추가, `entry_query`(`UI/command-palette-query.rs` 81)가 `LINE_MODE_PREFIX`(6, `":"`)를 돌려주게 |
| `editor.action.quickOutline` | ⇧⌘O | quick input 문서 심볼 | 결정 4 A: `PaletteEntry::Symbol`(`SYMBOL_MODE_PREFIX` `"@"`, 5) |
| `editor.action.pasteAsText` | 없음 | 클립보드 평문 붙여넣기 | 구현: 편집기 우클릭 메뉴 Paste와 같은 경로를 씀(`APP/domains/editor.rs` `run_editor_context_menu`(3716)의 `Action::Paste` → `ClipboardOperation::Paste` → `HostCommand::Editor(Command::EditorClipboard(..))`(259~) → worker의 `read_clipboard`(`APP/host.rs` 13 `ClipboardReader`, 62)). 요청 생성은 `APP/editor-context-menu.rs` `request_palette`(412)와 `Clipboard::begin`(487)을 읽고 같은 방식으로 만듦. TAIDE에는 평문 외 붙여넣기 공급자가 없어 결과는 일반 붙여넣기와 같음 |
| `editor.action.pasteAs` | 없음 | 붙여넣기 종류 선택(텍스트만이면 "Insert Plain Text" 하나) | 결정 5: 편집기 새 행 의존 |
| `editor.action.clipboardCopyWithSyntaxHighlightingAction` | 없음 | HTML+평문 복사 | 결정 5 |
| `editor.action.openLink` | 없음 | 커서 아래 URL 열기(링크 감지) | 결정 5(editor 새 행 "텍스트 링크 감지") |
| `editor.action.unicodeHighlight.*` 4개 | 없음 | 모호·보이지 않는 문자 강조 끄기·옵션 | 결정 5(editor 새 행 "Unicode 강조") |
| `editor.action.showOrFocusStandaloneColorPicker` | 없음 | 커서 위치 색 선택기 | 결정 5, editor-57 E4 색 선택기 UI(기능표 editor-57 비고 "선택기 UI(… 화면·앱 연결 없음)") |
| `editor.action.inlineSuggest.toggleAlwaysShowToolbar` | 없음 | inline 제안 toolbar 항상 표시 토글 | toolbar는 editor-77로 구현됨(`UI/editor-inline-suggest-toolbar.rs` 76 `toggle_always_shown(context)`). 팔레트 실행 경로만 남음: 원본 Action2라 편집기 조건 없이 항상 활성 → `show_palette`에서 `toggle_always_shown(context)` 호출(기능표 editor-34 비고 "팔레트 실행 경로는 남았습니다") |
| `editor.action.goToLocation` | 없음 | 인자 없는 실행은 무동작(`MON/contrib/gotoSymbol/browser/goToCommands.js` 591~613) | 결정 2 A: 비활성 유지, 비고 |
| `editor.action.increaseHoverVerbosityLevel` / `decrease..` | 없음 | 공급자가 verbosity를 지원할 때만 효과, TAIDE hover는 LSP라 무효과 | 결정 2 A: 비활성 유지, 비고 |
| `taide.gitStageSelection` / `taide.toggleBlame` / `taide.openFileHistory` | 없음 | git 기능 | git 병합으로 `Run::Git(GitRun::{StageSelection, ToggleBlame, OpenFileHistory})`(`GITWT/…/command-registry.rs` 476~480). 병합 뒤 실행 확인만 |

- 활성 조건 주의: `goToLocation`·`toggleAlwaysShowToolbar`·`showOrFocusStandaloneColorPicker`·`toggleTabFocusMode`·`unicodeHighlight.*`는 원본 Action2(항상 활성)라 `EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE`(`UI/command-registry.rs` 25~47)에 있습니다. 실행 경로가 없으면 `runnable`이 `None`이라 팔레트에서 비활성으로 보입니다. 실행을 연결하면 자동으로 활성이 됩니다. 결정 2 A의 무동작 항목(`goToLocation`)은 실행을 연결하지 않아 비활성으로 남깁니다.
- (d) 구현 순서:
    1. 실패 검사 먼저: UI `command_registry` 검사에 "`Unavailable`인 `monaco.*` id 목록 = 이 표에서 '결정'·'의존'으로 남기는 id 집합"을 단언하는 전수 검사(현재 실행되지 않는 25개 목록으로 시작, git 병합 뒤 22개, 구현할 때마다 줄임). 이름 예: `실행되지_않는_monaco_액션은_결정과_의존으로_남긴_목록뿐이다`.
    2. 구현 대상 9개: fontZoom 3, inPlaceReplace 2, toggleTabFocusMode, hideHover, pasteAsText, toggleAlwaysShowToolbar 팔레트 경로. 결정 4 A 2개: gotoLine, quickOutline.
    3. 새 기본 키: 원본 Monaco 기본 키가 있는 항목(inPlaceReplace ⇧⌘,/⇧⌘., toggleTabFocusMode ⌃⇧M, gotoLine ⌃G, quickOutline ⇧⌘O, hideHover Escape)은 `UI/keymap.rs`가 읽는 편집기 기본 키 JSON에 플랫폼별로 추가합니다. 기존 형식은 `UI/editor-documentation-keymap-defaults.json`(`{ "id": "monaco.editor.action.showHover", "key": "k", "mods": ["mod"], "chord": { "key": "i", "mods": ["mod"] } }`) 등 `UI/editor-*-keymap-defaults.json` 10개. 새 파일 `UI/editor-commands-keymap-defaults.json`을 만들면 `UI/keymap.rs`의 `include_str!` 목록(463~584)에 같은 방식으로 추가합니다. 플랫폼별 정확한 키는 각 Monaco 소스의 `kbOpts`에서 옮깁니다(`2048 /* KeyMod.CtrlCmd */` = mod).
    4. 결정 5 항목 7개(pasteAs, 구문 강조 복사, openLink, unicodeHighlight 4)와 색 선택기 1개: 편집기 영역 새 행이 생기고 complete가 되면 실행을 연결합니다. 그 전에는 43을 partial로 두고 비고에 의존 행을 적습니다. 새 행 번호가 아직 없으므로 editor 영역 담당에게 행 등록을 먼저 요청합니다(결정 5 A의 "편집기 영역에 새 행으로 등록").
    5. 결정 2 A 항목 3개(goToLocation, hover verbosity 2): 비활성 유지, 비고.
- (e) 편집기 crate 단위(`ED/inplace-replace.rs` 검사: `5`→`6`, `1.5`→`1.6`, `true`↔`false`, `private`→`public` 순환, `Private`→`Public`; 기대값은 `MON/common/languages/supports/inplaceReplaceSupport.js`를 읽어 정함), UI 단위(zoom 곱으로 글꼴·줄 높이 변화, tab focus 모드에서 Tab이 focus 이동, hideHover로 hover 닫힘), App(`application-commands-tests.rs`: 팔레트와 기본 키 각 1건 — zoom, gotoLine(팔레트가 `:` 모드로 열림, `inspection()`의 query 확인), inPlaceReplace, tab focus, readonly 문서에서 inPlaceReplace 비활성), 레지스트리 전수 검사. 실행: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-editor/Cargo.toml --lib --locked --offline --target-dir <T> -- inplace`, UI·앱 필터는 2.7절.
- (f) 구현 대상 11개 통과 + 나머지가 결정·의존 비고로 정리되면, 결정 5 의존이 남으므로 partial(남은 의존 행 명시). 결정 5 의존 행이 모두 complete가 되고 연결되면 complete.
- (g) editor 영역 새 행(미등록)과 editor-57 색 선택기 UI. 실제 클립보드 읽기(pasteAsText 실기)는 사용자 허락 항목이고 자동 검사는 주입 포트(`HostBridge::connect_with_clipboard`류, `APP/host.rs` 95)만.

---

## 4. 이 영역에서 반복된 함정과 주의

QA 문서의 실패 기록·흔들림·리뷰 차단에서 추렸습니다.

1. **기능표 비고를 현재 상태로 믿지 않습니다.** commands-ui 비고 대부분은 2026-10-06 감사 공통 문구입니다(사양 0절). 82의 "구역별 장애 … 경로 없음", 83의 "상단 재시도 배너 … 없음", 61의 "search/search-replace는 없습니다"는 모두 이미 사실과 다릅니다. 반드시 실제 코드로 다시 확인합니다. 과거 QA의 "통과" 기록도 현재 HEAD에서 다시 돌리기 전에는 근거로 쓰지 않습니다(`docs/memory/2026-10-07-native-transition-operations.md` 6절).
2. **사양 문서도 낡을 수 있습니다.** 이 문서를 쓰며 확인한 차이: 테마 수(36 → 47 파일), 아이콘 수(약 120 → import 이름 158), 아이콘 경로(`APP/resources` → `native/taide-native-app/resources`), 32~34·61·83·70·76·84의 선행 완료, 84 모듈 위치(UI → APP), 1의 `pending_disk_choice`(대화상자 아님), 33의 결정 8 뒤집힘, 77의 상수 수(3 → 4). 사양과 실제 코드가 다르면 실제 코드를 따르고 QA에 차이를 적습니다(lane workflow 규칙).
3. **다른 영역 검사의 "명령" 근거가 실제 팔레트 경로가 아닐 수 있습니다.** git lane의 `view.git`·`git.revertHead`·`git.createTagOnHead`·`git.toggleBlame`·`git.openFileHistory` 검사는 모두 `palette_commands.push(..)`로 채널에 직접 넣습니다(3.2절). 이 방식은 팔레트 행의 활성 여부·label 검색·Enter 선택을 덮지 않으므로 이 영역 행을 닫을 때는 `run_palette`/`run_auxiliary_palette` 검사를 따로 둡니다.
4. **앱 lib 병렬 실행 흔들림.** 실제 App 검사를 많이 동시에 돌리면 host 큐(64)가 차서 `native host command queue is full`로 실패하거나 OS watcher 60초 대기가 시간 초과됩니다(`QA/2026-10-27-native-error-boundary-foundation.md` 리뷰 수정 검증 절, `QA/2026-10-28-native-shell-keymap-dialogs-slot-header.md` 134~136행 "60초 OS watcher 대기 시간 초과 … 단독 재실행은 1통과"). 필터 실행은 `--test-threads=2`, 시간 초과 실패는 단독으로 1회 재실행해 흔들림과 결함을 구분합니다.
5. **알려진 기존 실패.** `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`는 시스템 글꼴 fallback 체인 차이로 단독 재실행도 실패하는 기존 실패입니다(seams 0절, `docs/PROCESS.md` W0·W1). `CR/taide-runtime/tests/agent_probe.rs` 소스 문자열 단언 1건도 `81e96198`부터의 기존 실패입니다. 이 영역 변경의 실패로 세지 않습니다.
6. **LSP mock 선행 빌드.** 캐시를 비운 뒤 앱 lib 검사 다수가 실패한 사례는 `native-lsp-mock` 예제가 없어서였습니다. 앱 lib 검사 전에 `--example native-lsp-mock` 빌드를 먼저 합니다(병합 게이트 첫 줄과 같음).
7. **`cfg(feature = "native-host")` 누락.** 새 `Run`·`Intent` 변형이나 그것이 담는 타입(`WindowMove`, `SyncRun`)을 막지 않으면 동결 `taide-remote-web`의 컴파일 표면이 바뀝니다. UI crate를 바꾼 묶음은 remote-web Wasm check를 반드시 돌립니다. 반대로 `NATIVE_COMMANDS` 같은 고정 길이 const 표에는 cfg 행을 넣을 수 없어 전용 검사로 대신합니다(`QA/2026-11-02-native-lane-terminal-agent-commands-ui-entry.md` "사양과 다르게 한 점").
8. **검사 모듈 연결 조건.** App 검사 helper(`project_rail_tests`·`selection_ranges_tests`·`inlay_hints_tests`)가 `#[cfg(all(test, unix))]`라 새 검사 연결도 같은 조건이어야 합니다. macOS 키 배치(`MAC_CMD`)나 macOS 전용 동작(CLI 설치)을 가정하면 `#[cfg(all(test, target_os = "macos"))]`로 연결합니다(terminal-agent 리뷰 수정). 검사 전용 함수(`HostBridge::connect_with_agent_cli`)도 같은 조건으로 맞춰 다른 플랫폼에서 쓰이지 않는 함수 경고를 남기지 않습니다.
9. **다른 영역 검사 모듈은 보이지 않습니다.** `domains::settings`의 자식 검사 모듈(`Scene`, `Keychain`, `FakeGist`)과 `domains::git`의 검사 모듈(`Workspace`, `show_git`)은 `domains::commands::tests`에서 쓸 수 없습니다(2.7.3). 그 모듈을 공개하려면 다른 영역 파일을 바꿔야 하므로, 작은 가짜를 같은 파일에 둡니다. 예외로 git 저장소 fixture는 `pub(crate) mod fixture`라 바로 씁니다.
10. **discard pass와 intent 수집.** panic이 난 프레임은 `request_discard`로 같은 프레임에 pass가 두 번 돌고 입력 event는 첫 pass에만 들어갑니다(vendored `context.rs` `run_dyn`). 하네스가 마지막 pass의 출력만 보면 단언이 아무것도 검증하지 못합니다. 모든 pass의 출력을 모아 단언하고, panic 없는 2 pass 대조군을 둡니다(`QA/2026-10-27-native-error-boundary-foundation.md` 차단 1). 1(`window.reload`)의 장애 경계 검사에 그대로 적용됩니다.
11. **상태 수명.** 영역이 숨었다 다시 보이면 새 mount인지(Welcome·Activity Bar·상태 표시줄) 실패 유지인지(사이드바 패널) 원본 규칙이 다릅니다(차단 2). `reset_viewport`를 넣을 때 이 frame 기반 규칙(`is_unmounted_while_hidden`, `State.frame`)을 깨지 않게 epoch를 id에만 더합니다.
12. **같은 프레임 순서.** 팔레트 명령은 overlay viewport의 pass에서만 `keymap_actions`에 합류합니다(`APP/application.rs` 2107). 보조 창 검사는 overlay viewport가 그 창이 되도록 그 viewport에 ⌘⇧P를 보내야 합니다(`run_auxiliary_palette`가 `assert_eq!(application.shell_state.overlay_viewport, viewport)`로 확인).
13. **원본 결함 재현 금지와 결정 우선.** 원본의 무동작 활성 행은 결정 2 A대로 비활성으로 둡니다(43의 3개, 40은 이미 처리). 단, 결정 2 A가 나열하지 않은 행(38·39 편집기 없음)은 원본과 같은 `Always`로 두고 비고에 적습니다. CLI 취소 성공 문구는 결정 8 예외로 보이지 않습니다.
14. **구현과 검사를 같은 편집에서 쓸 때.** 여러 QA가 "구현 전 실패 실행을 따로 하지 않았다"고 적었습니다(구현을 되돌려 재현하는 편집은 작업 규칙상 금지). 대신 새 검사의 기대값을 원본(TS 값표, 원본 문구)에서 가져오고, 바뀐 기존 단언(예: unavailable 목록)이 이전 동작의 기대값이었음을 QA에 적습니다(commands-ui-entry QA "남은 범위" 선례).
15. **검사 쪽 결함이 첫 실패의 흔한 원인.** 같은 색의 다른 사각형을 세기, 다른 pass의 기록을 두 번 세기, 한 프레임 일찍 단언하기 같은 검사 결함이 첫 실행 실패의 다수였습니다(git g8 QA, error boundary panels QA). 조건 대기는 `step_until`·`auxiliary_until`처럼 deadline 루프로 씁니다.
16. **공용 위젯 이전은 화면 동작을 바꾸지 않습니다.** 그리기 함수만 바꾸고 그 화면의 기존 검사가 모두 통과해야 합니다. 원본과 다른 기존 동작을 바로잡는 경우(예: close dirty 대화상자의 바깥 클릭 닫힘)는 원본 근거와 함께 QA에 따로 적습니다. 묶음 끝에 변경 crate 전체 `--no-fail-fast -- --test-threads=4`를 1회 돌립니다.
17. **레거시 TS 파일을 읽는 검사.** `UI/glyph-icons-tests.rs` 343은 `include_str!("../../../src/shared/icons/project-icon-registry.ts")`로 원본 TS 파일을 읽습니다(commands-ui-evidence QA 리뷰 수정 절). 59의 전수 검사처럼 원본 파일을 `include_str!`로 읽는 검사를 더하면 `src/` 정리 작업이 이 검사를 함께 옮겨야 한다는 사실을 QA에 남깁니다. 59는 native 쪽 `UI/keymap-defaults.json`을 읽으므로 해당 없음.
18. **사용자 허락 없이 하지 않는 것.** 실제 GitHub gist·Keychain PAT(30·31), `/usr/local/bin/taide` 설치·해제(32~34), 실제 저장소 revert(36), 실제 클립보드(43 pasteAsText), GUI 실행·합성 OS 입력·OS 알림·휴지통(`native/taide-native-app/tests/explorer-delete.rs` 실행 금지), 실기(OS 창 focus, VoiceOver, 픽셀). 검사는 임시 디렉터리·주입 포트·가짜 platform만 씁니다.
19. **git worktree는 다른 작업자가 병합 중입니다.** 그 worktree를 고치지 않습니다. 묶음 3은 병합이 `to_rust_native`에 들어온 뒤 메인 트리 또는 새 lane worktree에서 시작합니다. 병합 커밋 뒤에는 이 문서의 `GITWT/` 줄 번호를 심볼 이름으로 다시 찾습니다.
