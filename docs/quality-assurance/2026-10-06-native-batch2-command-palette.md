# Native 전환 배치 2 단계 3/3 — 커맨드 팔레트(명령·파일·줄 이동 모드)

작성일 2026-10-06. 기준 커밋 `0973248b` 위의 미커밋 작업 트리(앞 두 단계 test-health · command-registry 변경 포함)에서 진행했습니다. GUI 는 실행하지 않았고, 아래 결과는 전부 이 문서에 적은 명령의 실제 출력입니다.

## 0. 결론

- 검증 계약 V1~V5 의 명령은 모두 exit 0 입니다. native-ui lib 테스트는 57 → 87(신규 30), native-app lib 테스트는 329 → 336(신규 7), host 통합 테스트는 7 → 8, editor-reveal 통합 테스트는 1 → 2 이며 실패 0 입니다.
- ⌘P · ⌘⇧P · ⌘T 와 `file.quickOpen` 명령이 팔레트를 엽니다. 파일 모드는 `search_list_files` 로 받은 목록을 fuzzy 로 걸러 preview 탭으로 열고, 명령 모드는 레지스트리 실행 경로(`command_dispatch::intent`)로 실행하며, 줄 이동 모드는 활성 파일 탭에 reveal 을 대기시킵니다.
- 심볼(`@`) · 워크스페이스 심볼(`#`) 모드는 모드 인식과 빈 상태 표시까지만 구현했고 LSP 조회는 연결하지 않았습니다(P6 지시).
- 아이콘 5종, 열림·닫힘 애니메이션, 실제 화면 외형 확인은 남아 있습니다. 6장에 적었습니다.

## 1. 항목별 변경

### P1 순수 로직

| 파일 | 내용 |
|---|---|
| `native/taide-native-ui/src/fuzzy-match.rs` (신규, `fuzzy_match`) | `fuzzy_match(query, target)`, `Matcher::new(locale)` · `Matcher::filter(query, items, label_of, limit)`, `highlight_segments(text, indices)`. 후보 타입 `T` 에 독립적이고 `Ranked<T> { item, label, matched }` 를 돌려줍니다. 인덱스는 TS 와 같은 UTF-16 코드유닛 오프셋입니다 |
| `native/taide-native-ui/src/command-palette-query.rs` (신규, `command_palette_query`) | `Mode`, `parse`, `command_mode_query`, `entry_query`, `parse_line_target`, `LineTarget::label`, 접두사 상수 |
| `native/taide-native-ui/src/command-palette-file-match.rs` (신규, `command_palette_file_match`) | `split_for_display`, `relative_path`(TS `toRelativePath`), `file_name`(TS `fileNameOf`) |

- 기존 두 구현(`command-score.rs`, `keybinding-search.rs`)은 바꾸지 않았습니다. `fuzzy-match.rs` 는 JS 공백 판정 `keybinding_search::js_whitespace` 한 함수만 가져다 씁니다. `keybinding-search.rs` 의 `fuzzy_match` · `Search::filter` 와 본문이 겹치므로, 이후 그쪽이 공용 모듈을 호출하도록 바꾸면 중복이 없어집니다.
- `Matcher::filter` 의 `limit` 은 TS 의 `fuzzyFilter(...).slice(0, FILE_RESULT_LIMIT)` 를 한 함수로 합친 것입니다. 빈 질의는 정렬이 없어 앞 `limit` 개만 라벨을 만들고, 비어 있지 않으면 정렬 뒤 자릅니다. 결과는 slice 와 같습니다.
- `flattenDocumentSymbols` 는 이식하지 않았습니다. native 에 monaco `DocumentSymbol` 에 대응하는 타입과 조회 경로가 없습니다(P6).
- `highlight_segments` 는 코드포인트 단위로 자릅니다. TS 는 코드유닛 단위라 서로게이트 쌍의 한쪽만 인덱스에 있으면 문자 중간에서 자르지만, `fuzzyMatch` 는 항상 두 코드유닛을 함께 넣으므로 실제 입력에서는 같습니다.

### P2 팔레트 셸

`native/taide-native-ui/src/command-palette.rs` (신규, `command_palette`)

| 대상 | 내용 |
|---|---|
| `Palette::open(context, entry)` | 닫혀 있으면 이전 포커스를 기억하고 연 뒤 진입 질의(`""` · `">"` · `"#"`)를 넣습니다. 열려 있으면 질의만 바꿉니다(TS `openPalette`) |
| `Palette::show(context, scope, enabled)` | `egui::Modal`(scrim `app.shadow` 50%, 가운데 정렬) 안에 입력 행과 목록을 그리고 `Output { action }` 을 돌려줍니다 |
| `Scope` / `FileIndex` | 프레임마다 앱이 넘기는 값: locale, `CommandContext`, keymap override JSON, 파일 목록(루트 · 경로 · revision · pending · refreshing), 활성 파일 경로 |
| `Action` | `RunCommand(id)` · `OpenFile(절대 경로)` · `RevealLine(LineTarget)` |
| `Appearance::new(theme)` | 테마 키 11개 |
| `Inspection` | test · `inspection` feature 에서만 컴파일되는 프레임 기록 |

재현한 규칙과 근거

| 동작 | native | TS 근거 |
|---|---|---|
| 포커스 · 캐럿 | 열려 있는 동안 입력이 포커스를 갖고, 열 때와 질의를 코드로 바꿀 때 캐럿을 끝에 둡니다(선택 없음) | `command-palette.tsx:396-399,423-426`, `text-input-caret.ts` |
| 닫기 | Escape · 바깥 클릭은 닫고 이전 포커스를 되돌립니다. 항목 실행으로 닫히면 되돌리지 않습니다. 닫을 때 질의 · 선택을 비웁니다 | `command-palette.tsx:103,153-157,183-186,427-430` |
| 선택 | 질의가 바뀌거나 선택 항목이 목록에서 사라지면 첫 활성 항목을 고릅니다 | cmdk `index.mjs` 의 `W()`(search 변경 · item mount/unmount) |
| ↑/↓ | 활성 항목 사이만 이동하고 끝에서 멈춥니다(loop 없음). ⌘↑/⌘↓ 는 처음/끝, ⌥ 는 그룹 이동이며 그룹이 하나라 일반 이동과 같습니다 | cmdk `Q` · `X` · `re` · `ie` · `se` |
| Home/End | 처음/끝 활성 항목. 입력 캐럿은 움직이지 않습니다 | cmdk `case "Home"` · `"End"` (`preventDefault`) |
| ⌃N/⌃J, ⌃P/⌃K | 다음/이전(Shift 없이) | cmdk `vimBindings` 기본값 `true` |
| Enter | 선택 항목이 활성일 때만 실행 | cmdk `case "Enter"`, `CommandItem disabled` |
| PageUp/PageDown | 처리하지 않습니다 | cmdk 에 해당 분기가 없습니다. 6장 5번 |
| Tab | 포커스가 입력에 남습니다 | Radix `FocusScope`(탭 가능한 요소가 입력 하나) |
| IME | 조합 중(`Preedit` 비어 있지 않음)이거나 그 프레임에 IME 이벤트가 있으면 Enter · Escape · 탐색 키를 팔레트가 가로채지 않습니다 | cmdk `isComposing \|\| keyCode === 229`, `dialog.tsx:66-72` |
| 포인터 | 포인터가 움직여 활성 항목 위에 오면 선택(스크롤 없음), 클릭하면 실행. 비활성 항목은 반응 없음 | cmdk `onPointerMove` · `onClick`, `data-[disabled=true]:pointer-events-none` |
| 스크롤 | 목록 최대 300px, 스크롤바 숨김. 키보드로 선택이 바뀌면 위아래 4px 여유로 보이게 하고, 그룹 첫 항목이면 heading 까지 보이게 합니다 | `command.tsx:68`(`max-h-[300px] scroll-py-1 scrollbar-hidden`), cmdk `ne()` |
| 빈 상태 | 항목이 0개일 때 문구를 보이고 그룹 heading 은 그대로 둡니다 | cmdk `Empty`(`shouldFilter={false}` 에서 mount 된 항목 수), `Group` |
| 대상 프로젝트 | 메인 창은 포커스 slot 의 프로젝트(`CommandContext::active_project`) | `main-window-dialogs.tsx` |

크기 · 색(전부 TS 클래스에서 읽은 값)

| 요소 | 값 | 근거 |
|---|---|---|
| 너비 | 뷰포트 640px 이상 512px, 미만은 뷰포트 − 32px | `dialog.tsx:74` `max-w-[calc(100%-2rem)] sm:max-w-lg` |
| 테두리 · 모서리 · 그림자 | 1px `modal.border`, 8px, `0 8px 24px app.shadow` | `rounded-lg border border-modal-border shadow-overlay-lg`, `global.css:156,293-295,352-354` |
| 배경 | 바깥 `modal.background`, 안쪽 `panel.background`(모서리 6px) | `dialog.tsx:74`, `command.tsx:14`, `command-palette.tsx:431` |
| 입력 행 | 높이 36px, 좌우 12px, 아래 1px `app.border`, 14px | `command.tsx:50,55` `h-9 px-3 border-b text-sm` |
| 빈 문구 | 위아래 24px, 가운데, 14px/20px | `command.tsx:75` `py-6 text-center text-sm` |
| 그룹 | 4px. heading 좌우 8px · 위아래 6px · 12px/16px · medium · muted | `command.tsx:83` |
| 항목 | 좌우 8px · 위아래 6px · 간격 8px · 모서리 4px · 14px/20px | `command.tsx:112` |
| 선택 항목 | 배경 `list.activeBackground`, 글자 `list.foreground`, 안쪽 1px `app.accent` | `command.tsx:112`, `global.css:151,164,266` |
| 비활성 항목 | 전체 불투명도 50% | `data-[disabled=true]:opacity-50` |
| 강조 | `panel.matchHighlight`, semibold | `highlighted-text.tsx:8` |
| 단축키 | 오른쪽 정렬, 12px, 자간 0.1em(1.2px), muted | `command.tsx:121` `ml-auto text-xs tracking-widest` |
| muted | `appSidebar.iconDefault` | `global.css:149,285` |

기존 다이얼로그 방식 재사용: `snippet-editor.rs` 의 Dialog · `keybinding-editor.rs` 와 같은 구성요소를 썼습니다(`egui::Modal` 레이어, scrim `app.shadow × 0.5`, `EventFilter` 로 포커스 고정, `request_focus_with_filter`, IME 조합 판정). 공용 함수가 없어 호출로 재사용하지는 못했습니다.

### P3 명령 모드

- `Palette::command_items`: `registry().commands()` 중 플랫폼에 등록된 전체를 `Command::label` 로 fuzzy 필터합니다. 빈 검색어는 등록 순서 그대로입니다.
- TS 는 비활성 명령을 숨기지 않고 흐리게(선택 · 실행 불가) 보입니다(`command-palette-commands-group.tsx:31-33`, `command-palette.tsx:307-312`). native 도 전부 보이고 `Command::is_runnable(context)` 가 거짓인 행을 비활성으로 둡니다.
- 단축키 표시는 `keymap::catalog::rows` 에서 행 id(`keymapId ?? id`)로 찾아, 키가 있으면 `Binding::label`, 없고 `defaultBindingLabel` 이 있으면 그 문자열을 씁니다(`command-palette-commands-group.tsx:30,38-41`). override JSON 이 바뀔 때만 다시 만듭니다.
- 실행: 팔레트는 `Action::RunCommand(id)` 만 내고, 앱이 다음 프레임의 키맵 액션 목록에 id 를 넣어 `command_dispatch::intent` 로 해석합니다. 키맵과 같은 경로라 실패 보고(toast)도 같습니다.
- `file.quickOpen` 은 실행 경로가 `Run::OpenPalette(Files)` 라 팔레트가 닫히지 않고 질의만 비웁니다(TS `switchToFileSearchMode`, `command-palette.tsx:252,336`).

### P4 파일 모드

| 파일 | 내용 |
|---|---|
| `native/taide-native-app/src/host.rs` | `HostCommand::ListProjectFiles(project)` → `HostReply::ProjectFiles { project, result }`(`search_actions::search_list_files`), `HostCommand::OpenPaletteFile { project, pane, path }` → `HostReply::PaletteFileOpened { project, result }`. 기존 `OpenFileTab` 분기의 본문을 `open_file_tab` 으로 옮겨 둘이 함께 씁니다(동작 변화 없음) |
| `native/taide-native-app/src/command-palette.rs` (신규) | `FileIndexes`(프로젝트별 목록 캐시), `FileIndexChanges`(이벤트 sink 에서 무효화 대상 기록), `file_tab_pane` |
| `native/taide-native-app/src/application.rs` | `show_palette`, `HostReply` 두 분기, `PaintSink::publish` 의 `file_indexes.record`, `background_tick` 의 무효화 · 닫힌 프로젝트 정리 · 호스트 재연결 시 `cancel_fetches` |

캐시 · 재조회 규칙(`FileIndexes::observe`)

| 규칙 | native | TS 근거 |
|---|---|---|
| 조회 조건 | 팔레트가 열려 있고 파일 모드이며 프로젝트가 있을 때만 | `command-palette.tsx:129-132` `enabled: open && mode === 'files' && !!projectId` |
| 신선도 | 조건이 켜지는 순간 목록이 없거나 무효화됐거나 60초가 지났으면 조회 | `app/query-client.ts` `staleTime 60_000`, `docs/features/command-palette.md` §3.1 |
| 열려 있는 동안 | 무효화됐을 때만 다시 조회. 조회 중 무효화는 응답 뒤 한 번 더 | 같은 문서 §3.1 |
| 무효화 | `FsChanged` 의 kind 가 modified 가 아닐 때, `FsRescanRequired`, 팔레트 파일 열기가 `NotFound` 로 실패했을 때 | `ipc-sync-provider.tsx:266-270,551`, `layout.query.ts:270-271` |
| 실패 | 목록 없이 실패하면 "결과 없음", 이전 목록이 있으면 유지. 다시 열 때 재시도 | TanStack Query 기본(`retry: 0`, 오류 시 이전 data 유지) |
| 보관 | 팔레트의 대상 프로젝트가 아니게 된 목록은 10분 뒤 버리고, 닫힌 프로젝트는 즉시 버립니다 | `gcTime 10 * 60_000` |

표시 · 열기

- 매칭 대상은 프로젝트 루트 기준 상대 경로이고(`relative_path`), 결과 상한은 200개입니다(`FILE_RESULT_LIMIT`, `command-palette.tsx:65,305-306`).
- 행은 파일명 + 상위 경로 2줄, 둘 다 말줄임. 루트 직속 파일은 한 줄(`command-palette-files-group.tsx:45-58`).
- 목록이 아직 없으면 "Loading...", 조회 중에는 heading 옆에 "Refreshing"(`command-palette.tsx:324`, `files-group.tsx:34-42`). TS 와 같이 첫 조회 중에도 "Refreshing" 이 함께 보입니다(`isFetching`).
- 프로젝트가 없으면 heading "Files" 와 "No results" 만 보입니다(`resolveEmptyStateMessage`).
- 선택하면 `OpenPaletteFile` 을 preview=true 로 제출합니다(`openFile`, `preview: true`). 대상 pane 은 그 파일이 이미 열린 pane(포커스 pane 우선, 다음 트리 순서), 없으면 서버의 포커스 pane 폴백입니다(`layout.query.ts:258`, `pane-tree.ts:107-112`).
- 열기 실패는 toast(`Toasts::ipc_error`)로 보고합니다(`layout.query.ts:272`).
- 걸러진 결과는 (검색어, 루트, revision) 이 같으면 다시 계산하지 않습니다.

### P5 줄 이동 모드

- `Palette::listing` 의 `Mode::Line`: `parse_line_target` 이 성공하고 활성 파일이 있을 때만 heading 없는 그룹에 항목 하나(`42` 또는 `42:10`)를 둡니다(`command-palette-line-group.tsx:13-21`).
- 활성 파일이 없으면 `palette.noActiveFile`, 있고 입력이 유효하지 않으면 `palette.noResults`(`command-palette.tsx:319-323`).
- 활성 파일은 포커스 탭이 `TabKind::File` 일 때만입니다(`pane-tree.ts:102-105`, `command-palette.tsx:138`).
- 실행: `editor_reveal::Reveals::queue_position(Target, Position, layouts, now)` 로 기존 reveal 대기열에 넣습니다. 기존 `queue(&OpenedFileLink, ..)` 는 이 함수를 호출하도록 바꿨습니다. 다음 프레임 `show_document` 가 소비하면서 편집기에 포커스를 줍니다(기존 동작).

### P6 심볼 · 워크스페이스 심볼 모드

- `@` : heading `palette.symbols`, 항목 없음. 활성 파일이 없으면 `palette.noActiveFile`, 있으면 `palette.noResults`.
- `#` : heading `palette.workspaceSymbols`, 항목 없음. 프로젝트가 없으면 `app.openProjectFirst`, 있으면 `palette.noResults`.
- TS 에서 provider 가 없을 때 도달하는 최종 상태입니다(`use-document-symbol-loader.ts`, `use-workspace-symbol-search.ts`, `command-palette.tsx:318-331`). 조회 전의 일시적인 `common.loading`(심볼 로딩 중, 워크스페이스 심볼 200ms debounce)은 조회 자체가 없어 표시하지 않습니다.
- LSP `documentSymbol` · `workspace/symbol` 호출은 연결하지 않았습니다.

### P7 진입 연결

| 파일 | 내용 |
|---|---|
| `native/taide-native-ui/src/command-registry.rs` | `PaletteEntry { Files, Commands, WorkspaceSymbols }`, `Run::OpenPalette(PaletteEntry)`, `keymap_run` 에 `quick-open` · `command-palette` · `workspace-symbol` 추가. `file.quickOpen` 은 keymapId 가 `quick-open` 이라 자동으로 `Execution::Native` 가 됩니다 |
| `native/taide-native-ui/src/commands.rs` | `ShellIntent::OpenPalette(PaletteEntry)` |
| `native/taide-native-app/src/shell_keymap.rs` | `intent` 가 `Run::OpenPalette` 를 `ShellIntent::OpenPalette` 로, `runs_without_focused_project` 에 포함(프로젝트 없이도 열립니다) |
| `native/taide-native-app/src/application.rs` | intent 분기 `ShellIntent::OpenPalette(entry) => self.palette.open(..)` |

- 근거: `command-palette.tsx:235-241`(키맵 3종), `command-catalog.ts:49-55`(`file.quickOpen`).
- 열려 있는 동안 ⌘P · ⌘⇧P · ⌘T 를 다시 누르면 모드만 바뀝니다. 전역 키맵은 TS 와 같이 팔레트가 열려 있어도 동작합니다(`use-global-keymap.ts` 에 다이얼로그 가드가 없습니다).
- 키 입력 격리: 팔레트가 열려 있는 동안 셸을 비활성으로 그리고(`add_enabled_ui`, 키바인딩 편집기와 같은 방식), 팔레트가 처리한 키는 `input.events` 에서 제거합니다. zen Escape, 웹 미리보기, toast 상호작용, 하드코딩 ⌘N 도 팔레트가 열려 있으면 동작하지 않게 했습니다.

이전 단계 기대값을 바꾼 테스트(요구사항이 바뀐 부분만)

- `command-registry.rs`: `NATIVE_COMMANDS` 34 → 35(`file.quickOpen`), `NATIVE_KEYMAP_COUNT` 31 → 34, 미연결 목록에서 3개 제거 후 `Run::OpenPalette` 단언 추가
- `command-dispatch.rs`: `KEYMAP_ACTIONS` 31 → 34, `PROJECTLESS_KEYMAP_ACTIONS` 7 → 10, 미연결 목록에서 3개 제거, `ShellIntent::OpenPalette` 단언 추가

현재 레지스트리: Native 35, Unavailable 177. 키맵 액션 41개 중 `Run` 34, 터미널 뷰 직접 처리 2, 실행 대상 없음 5(find, search, search-replace, explorer, git).

## 2. 범위 밖 수정

| 파일 | 내용 | 사유 |
|---|---|---|
| `native/taide-native-app/src/presentation-refresh.rs` | `Appearances` 에 `palette` 필드와 생성 1줄 | 테마 교체가 "전부 성공하거나 전부 유지"로 묶여 있어, 팔레트 색만 따로 계산하면 그 원자성이 깨집니다 |
| `native/taide-native-ui/src/commands.rs` | `ShellIntent::OpenPalette` 1개 | 키맵 · 명령이 같은 intent 경로로 팔레트를 열려면 intent 종류가 필요합니다 |
| `native/taide-native-app/tests/host.rs`, `tests/editor-reveal.rs` | 테스트 1개씩 추가 | host 명령과 reveal 함수 추가의 검증 |

## 3. 추가한 테스트

native-ui (30개)

- `fuzzy_match::tests` 9개: TS `fuzzy-match.test.ts` 의 사례를 describe 단위로 묶어 옮겼고 단언 메시지에 원본 테스트 이름을 적었습니다. `limit` 테스트 1개는 신규입니다.
- `command_palette_query::tests` 4개: `command-palette-query.test.ts` 의 `parsePaletteQuery` · `buildCommandModeQuery` · `parseLineModeTarget`. 정규식 `^(\d+)(?::(\d+))?$` 에서 따라 나오는 거절 사례(`1:`, `:1`, `+1`, `1.5`, 전각 숫자, `1 2`, `-1`)를 더했습니다.
- `command_palette_file_match::tests` 4개: `command-palette-file-match.test.ts`, `relative-path.test.ts`
- `command_palette::tests` 13개(egui `Context` 를 직접 구동): 열기 · 포커스 · 캐럿 · 반응형 너비, 파일 모드(2줄 · 로딩 · 갱신 · 상한 200 · 목록 높이 300), 명령 모드(전체 행 · 비활성 · 단축키 · override 반영), 줄 이동 · 심볼 빈 상태, 탐색 키, 스크롤 가시화, Enter 실행과 포커스 미복귀, Escape · 바깥 클릭과 포커스 복귀, IME, 포인터, 키 소비, 비활성 상태, 테마 키

native-app lib (7개, `command_palette::tests`)

- `FileIndexes` 4개(조회 시점 · 60초 · 무효화 · 실패 · 프로젝트 전환 · 10분 보관), `FileIndexChanges` 1개, `file_tab_pane` 1개
- `팔레트_키는_창_route에서_소비되어_…`: `Views::route_window_keys` → `command_dispatch::intent` → `Palette::open/show` → `FileIndexes` → `Action::OpenFile` 을 한 흐름으로 확인합니다.

native-app 통합

- `tests/host.rs` `실제_host는_프로젝트_파일_목록과_팔레트_파일_열기의_결과를_…`: 실제 디렉터리 순회, 닫힌 프로젝트 `NotFound`, 없는 파일 `NotFound` 와 레이아웃 불변, 목록의 경로 그대로 preview 탭 열기
- `tests/editor-reveal.rs` `이미_열린_파일_탭의_줄_이동은_…`

## 4. 실행한 명령과 결과

공통 인자 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` 과 저장소 절대 경로 접두사는 생략해 적었습니다.

| 순서 | 명령 | 결과 |
|---|---|---|
| 1 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib -- fuzzy_match:: command_palette_query:: command_palette_file_match::` (본문이 `unimplemented!` 인 상태) | exit 101. 0 passed, 17 failed |
| 2 | 같은 명령 + `command_registry::` (구현 뒤) | exit 0. 21 passed, 0.03s |
| 3 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib -- command_palette::` | exit 101. 컴파일 오류 2건 |
| 4 | 같은 명령 | exit 101. 5 passed, 7 failed |
| 5 | 같은 명령 | exit 101. 10 passed, 2 failed |
| 6 | 같은 명령 | exit 0. 12 passed, 0.27s |
| 7 | `cargo check --manifest-path native/taide-native-app/Cargo.toml` | exit 0 |
| 8 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- command_palette:: command_dispatch::` | exit 0. 10 passed, 0.01s |
| 9 | 6번과 같은 명령(키 소비 테스트 추가 뒤) | exit 0. 13 passed, 0.27s |
| 10 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- command_palette::` | exit 0. 7 passed, 0.14s |
| 11 | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | exit 1. 이번 단계 신규 파일만 차이 |
| 12 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | exit 1. 이번 단계에 추가한 부분만 차이 |
| 13 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` (V3, 전체 1회) | exit 0. 336 passed, 0 failed, 13.26s |
| 14 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test host` (V4) | exit 0. 8 passed, 0.07s |
| 15 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test editor-reveal` | exit 0. 2 passed |
| 16 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib` | exit 0. 87 passed, 0.38s |
| 17 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test workbench --test controller --test keymap-platform` | exit 0. 11 · 2 · 1 passed |
| 18 | `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml` (계약 외) | exit 0. 경고 8건은 모두 기존 파일(`lib.rs:1`, `settings-code-view.rs`, `settings-view.rs`, `snippet-editor.rs`) |
| 19 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml` (V5) | exit 0 |
| 20 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` (V5) | exit 0 |
| 21 | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` (V5) | exit 0 |
| 22 | `cargo check --manifest-path native/taide-native-app/Cargo.toml` (V5, 최종 트리) | exit 0. 경고는 vendor `wry` 17건뿐(기존) |

- 11 · 12번 뒤 `cargo fmt --manifest-path …` 로 형식을 맞췄고 13번부터는 그 뒤의 트리입니다. `git status` 로 포맷이 다른 파일을 건드리지 않은 것을 확인했습니다.
- 13 ~ 22번 사이에 소스는 바뀌지 않았습니다.

## 5. 실패했다가 고친 내역

1. 3번: `RowInspection` 에 `Default` 를 derive 했는데 `egui::Rect` 가 `Default` 가 아니었고, 테스트가 `CCursor::index` 를 `usize` 와 비교했습니다. derive 를 빼고 `CharIndex` 로 비교했습니다.
2. 4번 교착 3건: `Palette::close` 가 `memory_mut` 클로저 안에서 `context.viewport_id()` 를 불러 같은 락을 다시 잡았습니다. id 를 클로저 밖에서 먼저 계산합니다.
3. 4번 포커스 복귀 1건: 닫는 프레임에 `request_focus(previous)` 를 하면 다음 프레임에 포커스가 사라졌습니다. egui 는 직전 프레임의 최상위 modal 레이어보다 아래에 있는 위젯이 만들어질 때 그 위젯의 포커스를 놓게 합니다(`context.rs` `create_widget`, `memory/mod.rs` `top_modal_layer`). 복귀를 한 프레임 미뤄 팔레트가 그려지지 않은 첫 프레임에 요청하도록 바꿨고(`restorable_focus`), 테스트는 복귀 뒤 한 프레임 더 유지되는지까지 확인합니다. 닫힌 직후 다시 열면 원래 포커스를 이어받습니다.
4. 4 · 5번 테스트 쪽 오류 4건(구현은 그대로):
   - 명령 모드의 placeholder 를 파일 모드 문구로 기대했습니다. TS `PALETTE_PLACEHOLDER_KEY[mode]` 대로 `palette.commandPlaceholder` 가 맞습니다.
   - `app.openSettingsFile` 의 영어 제목을 추측해 적었습니다. locale 카탈로그 값 `Open settings.json` 으로 고쳤습니다.
   - 포인터 테스트가 `file.quickOpen` 행을 클릭 대상으로 골라 팔레트가 닫히지 않았습니다(올바른 동작). 대상에서 제외했습니다.
   - IME 테스트: 조합 중인 글자는 egui `TextEdit` 가 질의에 넣으므로(브라우저 입력과 같습니다) 명령 목록이 비었고, 조합 중에 보낸 ↓ 가 `TextEdit` 의 조합 범위를 옮겨 확정 글자가 두 번 들어갔습니다. 한글 파일명 fixture 로 바꾸고, 조합 중 탐색 키 검증은 확정이 없는 별도 구간으로 나눴습니다.

테스트 선작성: P1 은 본문을 `unimplemented!` 로 둔 채 실패를 확인한 뒤(1번) 구현했습니다. P2 ~ P5 의 화면 테스트와 앱 쪽 테스트는 모듈과 같은 편집에서 함께 썼고 구현 없는 상태로 따로 돌려 보지 않았습니다.

## 6. 남은 것

미구현 · 결정 필요

1. 아이콘 5종이 없습니다. TS 팔레트는 파일 타입 아이콘이 아니라 lucide 고정 아이콘을 씁니다: 입력의 `Search`(16px, 불투명도 50%), 명령 행 `Terminal`, 파일 행 `File`, 줄 이동 행 `CornerDownLeft`, 갱신 중 `Loader2`(12px, 회전). `native-ui/src/icons.rs` 의 고정 레지스트리에 추가해야 하는데 수정 범위 밖이라 넣지 않았습니다. `resources/problems/{terminal,file}.svg` 는 이미 있고 나머지 3개의 path 는 `node_modules/lucide-react/dist/esm/icons/{search,corner-down-left,loader-circle}.mjs` 에 있습니다. 지금은 아이콘 자리(16px + 간격 8px) 없이 글자가 왼쪽 여백에서 시작하므로, 아이콘을 넣으면 행 안쪽 배치가 24px 밀립니다.
2. 심볼(`@`) · 워크스페이스 심볼(`#`) 조회(P6). 다음 배치(LSP 화면) 범위입니다.
3. 열림 · 닫힘 애니메이션이 없습니다. TS 는 200ms fade + zoom 95%(`dialog.tsx:33,74`)이고 native 는 egui `Area` 기본 fade-in 만 있습니다. `snippet-editor.rs` 의 Dialog 는 같은 모션을 구현해 두었으나 그 모듈 안에 묶여 있습니다.
4. 실행 경로가 없는 명령 177개가 흐리게 보입니다. TS 에서는 대부분 활성 행입니다. 해당 기능이 native 에 생기면 `command-registry.rs` 의 `execution` 한 곳에서 풀립니다.
5. PageUp/PageDown: 작업 지시의 탐색 키 목록에 있었으나 cmdk 1.1.1 에 해당 분기가 없어 구현하지 않았습니다. 필요하면 동작을 정해 주셔야 합니다.
6. 배경 흐림: 팔레트가 열려 있는 동안 셸을 `add_enabled_ui(false)` 로 그려 egui 가 배경 전체를 반투명하게 만듭니다. TS 는 scrim 만 덮습니다. 키바인딩 편집기가 이미 같은 방식이라 맞췄습니다. 셸을 켜 둔 채 modal 레이어와 포커스만으로 입력을 막는 방법도 있으나, 편집기 · 터미널의 입력 소유 판정(vendored egui 의 `keyboard_input_route`)을 GUI 없이 확인할 수 없어 택하지 않았습니다.
7. 프로젝트 없이 파일을 여는 경우의 안내는 toast 가 아니라 `status` 문자열입니다(기존 `ShowOpenProjectNotice` 와 같은 처리, native toast 에 info 종류가 없습니다). 현재 화면에서는 프로젝트가 없으면 파일 행이 없어 도달하지 않습니다.

남은 위험

8. 파일 목록 조회가 단일 host worker 에서 순서대로 실행됩니다. 큰 프로젝트를 순회하는 동안 뒤에 온 host 명령(저장 등)이 기다립니다. TS 는 IPC 가 병렬입니다.
9. 인앱 파일 생성 · 이름 변경 · 삭제 직후의 무효화가 워처 에코(디바운스 300ms)에만 달려 있습니다. TS 는 mutation 성공 시점에도 무효화합니다. 그 사이의 낡은 행은 열기 선검증(`NotFound` toast)과 `NotFound` 무효화가 막습니다.
10. 키바인딩 편집기의 포커스 복귀에도 5장 3번과 같은 원인이 있을 수 있습니다(닫는 프레임에 `request_focus`). 범위 밖이라 손대지 않았습니다.
11. 팔레트 입력에서 IME 조합 중인 키가 전역 키맵으로 전달될 때의 가드가 없습니다(`route_window_keys` 의 `composing` 은 편집기 조합만 봅니다). macOS 는 IME 가 소비한 키를 앱에 넘기지 않아 실제로 나타나는지 확인이 필요합니다. 키바인딩 편집기 검색창도 같은 조건입니다.
12. 줄 번호가 1e21 이상이면 표시가 다릅니다(JS 는 `1e+21`, Rust 는 전체 자릿수).
13. 목록 높이가 바뀌는 프레임에 다이얼로그 세로 위치가 한 프레임 늦게 따라옵니다(egui `Area` 가 직전 프레임 크기로 가운데를 잡습니다).

실기 확인 필요

- [ ] ⌘P · ⌘⇧P · ⌘T 로 열리고, 열린 상태에서 다시 누르면 모드만 바뀌는지
- [ ] 외형: 너비 512px, 입력 행 36px, 항목 32px/48px, 목록 300px 에서 스크롤, 선택 행 배경 + 링, 강조 색 · 굵기, 단축키 자간, 한글 글꼴
- [ ] 배경 흐림 정도(6번)가 받아들일 만한지
- [ ] 편집기 포커스에서 열고 Escape 로 닫으면 편집기 캐럿으로, 터미널 포커스에서 열고 닫으면 터미널로 돌아가는지
- [ ] 파일을 열면 새 preview 탭의 편집기에 포커스가 가는지, 이미 다른 pane 에 열린 파일은 그 pane 에서 활성화되는지
- [ ] `:120`, `:120:8` 로 커서가 이동하고 화면 가운데로 reveal 되는지. 줄 수를 넘는 값의 동작
- [ ] 한글 IME: 조합 중 Enter 가 글자만 확정하고 항목을 실행하지 않는지, 조합 중 Escape 가 팔레트를 닫지 않는지
- [ ] 큰 프로젝트(수만 파일)에서 첫 "Loading..." 시간과 타이핑 응답, 조회 중 저장 지연(8번)
- [ ] 파일을 만들거나 지운 뒤 ⌘P 를 열었을 때 "Refreshing" 표시와 목록 갱신
- [ ] 명령 실행: `App: Settings`, `View: Toggle Sidebar`, `Tab: Reopen Closed Tab`, `File: Quick Open`(닫히지 않고 파일 모드로), 읽기 전용 문서에서 `monaco.deleteAllLeft` 비활성
- [ ] 팔레트가 열린 동안 편집기 · 터미널에 글자가 들어가지 않는지, 웹 미리보기 탭이 팔레트 위에 그려지지 않는지
- [ ] 키바인딩 편집기 위에서 ⌘P 를 눌렀을 때 팔레트가 위에 뜨고 닫으면 편집기 검색창으로 돌아가는지

검증하지 못한 것

- TS 쪽 bun 테스트와 `tools/keybinding-catalog/export.ts --check`: bun 실행이 금지돼 돌리지 않았습니다. `keybinding-commands.json` · `keymap-defaults.json` · fixture 는 바꾸지 않았습니다.
- native-app 대상 clippy 는 실행하지 않았습니다.
- `taide-remote-web` 은 호스트 타깃 `cargo check` 만 했습니다.

## 7. 테스트 부채

- [ ] `NativeApplication::show_palette` 와 `ui()` 안의 배선(셸 비활성, intent → `palette.open`, 다음 프레임 명령 실행, `OpenPaletteFile` 제출, reveal 대기, `HostReply` 두 분기). 재현 조건: 앱에서 ⌘P → 파일 선택, ⌘⇧P → 명령 실행, `:줄`. 생략 이유: `NativeApplication` 을 띄우는 테스트 하네스가 없습니다. 남은 위험: 구성 함수들은 각각 테스트가 있고 흐름은 `팔레트_키는_창_route에서_…` 가 같은 순서로 재현하지만, 앱 구조체 안의 연결은 컴파일만 확인했습니다. 필요 시점: 실기 확인, 또는 앱 하네스가 생길 때.
- [ ] `background_tick` 의 무효화 전달(`FileIndexChanges::take` → `FileIndexes::invalidate`)과 닫힌 프로젝트 정리. 같은 이유.
- [ ] 줄 번호 1e21 이상 표기(6장 12번). 생략 이유: 실제 문서에서 나올 수 없는 값입니다. 필요 시점: JS 숫자 표기 이식이 다른 곳에서 필요해질 때.
