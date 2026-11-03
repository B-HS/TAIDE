# explorer-search lane 묶음 C: 검색 진입·교체·편집기·메뉴 (search-entry-editor)

> 작성일 2026-11-03 · lane `explorer-search` 3/3 · worktree `TAIDE-lanes/explorer-search`(브랜치 `lane/explorer-search`)
> 기준: 사양 `docs/research/2026-10-28-lane-explorer-search-spec.md` 2.3절, 결정 D2·D3·D4·D8·D10 추천안(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`), 구조 `docs/research/2026-10-28-native-lane-seams.md`

## 대상 ID와 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| explorer-search-33 | partial | 바꾸기 전체 연결. 남은 것은 OS 알림 한 줄(shell lane 진입점 병합 뒤) |
| explorer-search-36 | complete | 검색 편집기 열기 버튼 + 탭 본문(D4 A) |
| explorer-search-37 | complete | ⌘⇧F/⌘⇧H/⌘F 대체, 선택 seed, focus·전체 선택, openReplace, 보조 창 |
| explorer-search-20 | partial | 메뉴 항목·구분선·동작·host 탭 열기 검사 통과. App 행 우클릭 → 메뉴 구간 자동 검사 미완 |
| explorer-search-21 | partial | 메뉴 항목·동작·App 범위 검색 통과. 같은 App 우클릭 구간 미완 |
| explorer-search-16 | partial | 삭제 뒤 부모 갱신 실패 toast 통과. 실제 휴지통 검사는 허락 P1 대기(미실행) |
| editor-81 | complete | = 36의 탭 본문 |
| commands-ui-11·12 | complete | = 37의 팔레트 실행(D8 A) |

대응표 함께 갱신: explorer-search-22·23 lane 표기를 panel-git으로(D3·D2 A, 사양 2.3), shell-89를 partial(searchEditor 부분 연결). settings-23은 33의 알림이 아직 없어 바꾸지 않았습니다.

## 이어받은 상태

앞 작업자가 이 묶음 코드(바꾸기, 검색 편집기, `Run::OpenSearch`, intent, find 대체, 메뉴 2항목, `RefreshOrigin::Delete`)를 넣어 두었으나 lib test가 컴파일되지 않았습니다.

- `command-dispatch.rs` 검사가 비공개 `application::WINDOW_LABEL`을 참조 → 같은 파일의 다른 검사 관례(`"main"` view key)로 수정.
- `search-panel-tests.rs`가 바뀐 `start_run(query, record, live)` 시그니처로 호출 → 같은 의미의 `submit`으로 수정.
- `command-dispatch.rs`의 보조 창 `search-replace` 단언이 보조 창 layout 없는 snapshot으로 검사해 실패 → 그 단언에만 보조 창 layout을 넣은 snapshot을 써서 시나리오를 바로잡음(기대값 그대로).

## 원본 근거

사양 2.3절의 ID별 "원본 동작"을 따랐습니다. 직접 다시 확인한 구간: `src/widgets/search-panel/search-panel-container.tsx:118-176`, `src/entities/search/replace-skip-report.ts`, `src/widgets/explorer/explorer-path.ts`(`resolveTargetDir`), `src/widgets/explorer/explorer-container.tsx:150-220`, 단위 검사 `search-query.test.ts`, `replace-skip-report.test.ts`, `search-editor-context-lines.test.ts`, `search-panel-bridge.test.ts`.

## 구현 (묶음 C 전체)

- `search-panel.rs`: `is_same_query`(contextLines 제외), `skip_report`·`skip_reason_key`·`SKIP_LIST_LIMIT`(5), `clamp_context_lines`(0~10), `Surface::{Sidebar, Editor}`, 바꾸기 줄(입력·모두 바꾸기·질의 불일치 안내)·파일별 체크(새 검색에도 유지)·확인 모달(취소 첫 focus, Escape, 확정 시 표시 결과 질의 + 체크 파일), `replace_finished`(성공이면 같은 질의 재검색, 이력 없음), 편집기 표면(바탕 `editor.background`, 좌우 12·상하 8, 제외 glob + 컨텍스트 줄 입력, 바꾸기·범위 칩·입력 검색 없음, 0건 `searchEditor.noResults`, 안내는 항상 `search.pressEnterHint`).
- `search-editor.rs`: 탭별 본문 16개 기억, 다른 프로젝트면 버림, 처음 mount만 자동 실행 표시, 화면에서 내려간 탭은 실행 취소·completed·최신 기억으로 이동.
- `domains/explorer_search.rs`: host `ReplaceSearch`/`SearchReplaced`(transient), `OpenSearchEditor`·`OpenTerminalAt`(`layout_open_tab`, preview 아님, 그 창 pane), `SearchTarget::{Sidebar, Editor}`로 실행 응답 분기, `apply_search_replaced`(요약 success toast, 건너뜀 warning, 재검색), intent `OpenSearch`/`OpenAuxiliarySearch`(Files 보기 → 접힘 해제 → `request_search`), ⌘⇧F seed(텍스트 focus 본문의 한 줄 primary 선택), `show_search_editor_tab`(탭 내용 분기 hook), 메뉴 동작 `OpenInTerminal`(폴더 자기·파일 부모·없으면 루트)·`FindInFolder`, 삭제 뒤 `RefreshTree{origin: Delete}` 실패 toast.
- `explorer.rs`: 행 메뉴 "터미널에서 열기"(행 묶음)·"폴더에서 찾기"(폴더만), 원본 구분선 구조(파일 묶음은 앞 구분선 하나, 행 묶음은 항상 구분선으로 시작). 항목 아이콘은 D10 A(글자만, 19의 기존 차이).
- `command-dispatch.rs`: `find`는 편집기(본문·찾기 위젯) focus가 없으면 `search`로 대체. `shell_keymap.rs`: `Run::OpenSearch` 처리. UI `command-registry.rs`: `Run::OpenSearch{replace}`와 keymap·팔레트 연결, `intents/explorer_search.rs`: `OpenSearch`/`OpenAuxiliarySearch`.

## 이번 작업자가 더한 것

- 단위 검사(`search-panel-tests.rs`): 질의 정규화·비교 8사례(정규화는 serde 기본값 = backend 계약으로 옮김), 건너뜀 보고 5사례, 컨텍스트 줄 5사례, 요청 기본값 2사례, 바꾸기 UI(활성 조건·안내·체크·확인 문구·Escape·취소·확정 내용·바꾸는 중 비활성·재검색), 검색 편집기 열기 버튼(빈 검색어·범위 포함), 편집기 본문(컨텍스트 줄 입력, 바꾸기·범위 칩·입력 검색 없음, Enter 이력·범위 유지, 0건 문구 2곳).
- 실제 App 검사(`application-search-panel-tests.rs` 4건, `application-explorer-search-tests.rs` 2건), 탐색기 메뉴 검사(`tests/explorer.rs` 1건).
- 검사용 읽기 접근자 2개(`#[cfg(test)]`): `symbol_sidebar::Views::search_request`, `search_editor::Editors::get`. 테스트 helper 공개 범위 조정(`focused_text_input`, `launch_project` 등 `pub(super)`).

## 검증 (실제 명령과 결과)

모든 cargo 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/explorer-search`를 붙였습니다.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets` | 처음 2 오류(위 "이어받은 상태") → 수정 뒤 성공, 기존 경고만 |
| `cargo test … --lib -- search_panel::tests search_editor::tests --test-threads=2` | 52 통과 |
| `cargo test … --lib -- <새 App 검사 6건> --test-threads=2` | 1차 4 통과·2 실패(바꾸기: 검사 fixture 본문 오기, 메뉴: App 우클릭 메뉴 미열림) → 바꾸기 기대 본문을 fixture에서 계산하도록 수정 |
| `cargo test … --test explorer -- 행_메뉴는` | 1 통과 |
| 영역 회귀 `cargo test … --lib -- search explorer symbol_ navigation command_dispatch shell_keymap application::tab_menu_tests application::auxiliary_window_tests application::explorer_views_tests application::error_boundary_tests application::context_menu_tests toast tooltip_controlled breadcrumb --test-threads=2` | 1차 237 통과·2 실패(dispatch 보조 창 fixture, App 입력 클릭이 같은 이름 "Search" 사이드바 탭을 집음) → 수정. 2차 238 통과·1 실패(바꾸기 전환 버튼 클릭을 Search 탭 tooltip이 가로챔, 부하 시만) → 포인터를 먼저 옮겨 tooltip이 닫힌 뒤 클릭 |
| `cargo test … --lib -- application::search_panel_tests application::explorer_search_tests command_dispatch --test-threads=2` | 25 통과(수정 영향 검사 재실행) |
| `cargo test … --test explorer -- --test-threads=2` | 22 통과 |
| `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib -- command_registry` | 15 통과 |
| `cargo check … native/taide-native-ui … --features native-host,inspection --all-targets` | 성공(기존 `tests/workbench.rs:1880` 경고만) |
| `cargo check … native/taide-remote-web … --target wasm32-unknown-unknown --lib` | 성공(동결 crate 기존 경고만) |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 처음 diff(이 lane 파일만) → `cargo fmt` 적용, `git status`로 lane 밖 파일 변경 없음 확인 → 통과 |
| `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 통과 |

### ID별 자동 검사

- 33: App `실제_app_모두_바꾸기는_확인한_파일만_바꾸고_…`(검색 4건/3파일 → 바꾸기 열기 → 교체 입력 → b.txt 체크 해제 → 확인 문구 2파일/2건 → Escape 무변화 → 확정 → a.txt만 변경, b.txt·비UTF-8 파일 보존, success `search.replaceDone{1,1}`, warning `search.replaceSkipped{1}` 설명에 파일명과 `notUtf8` 사유, 재검색 3건/2파일). 단위 바꾸기 UI·비교·건너뜀 보고.
- 36·editor-81: App `실제_app_검색_편집기_탭은_패널_입력으로_열려_…`(탭 kind·제목·preview 아님, 자동 실행 결과, 사이드바 패널은 실행 안 함, 컨텍스트 줄 2 → 컨텍스트 줄 표시와 이력 기록, 일치 열기 → 같은 pane preview, 탭 복귀 뒤 generation·컨텍스트 줄 유지, 빈 질의 탭 자동 실행 없음). 단위 기억 6·컨텍스트 줄 5·본문 UI.
- 37·commands-ui-11·12: App `실제_app_명령_shift_f는_…`(편집기 한 줄 선택 → ⌘⇧F → Search 보기·질의 "needle one"·전체 선택 [0,10], 여러 줄 선택 → seed 없음, ⌘⇧H → 바꾸기 열림, 범위 요청 뒤 접힌 사이드바에서 ⌘⇧F → 펼침·범위 해제, 편집기 focus 없는 ⌘F → 검색 패널, 편집기 focus ⌘F → 보기 유지), App `실제_app_보조_창의_명령_shift_f는_…`(그 창 탐색기만 Search). 단위 `command_registry`(keymap·팔레트 runnable), `command_dispatch`(`search`/`search.find`/`search-replace`/`search.replace`/`find` 대체, 보조 창).
- 20·21: `tests/explorer.rs` `행_메뉴는_원본_구분선_구조로_…`(파일·html·폴더 메뉴의 항목 순서, 구분선 수, `OpenInTerminal(row)`, 폴더만 `FindInFolder("node_modules")`). App `실제_app_터미널에서_열기는_…`(host `OpenTerminalAt` → 고정 Terminal 탭 cwd = 폴더, 제목 `terminal.title`, 활성; `scopeDir` 요청 → 범위 칩, `node_modules` 안 1건만, 칩 해제 뒤 재검색은 기본 제외로 note.txt만).
- 16: App `실제_app_삭제_뒤_부모_갱신이_실패하면_오류_toast로_알린다`(루트 제거 뒤 `ExplorerDeleteFinished{Ok}` 주입 → Error toast 증가).
- 실제 앱 경로: App 검사는 `launch`(실제 `NativeApplication::new`) → `eframe::App::logic` + 렌더 → 키 라우팅(`route_window_keys` → `command_dispatch::intent_with_editor`) → intent 적용 → 사이드바 region·탭 내용 분기 → host 작업자(`dispatch` → `search_actions`·`layout_actions`)를 거칩니다.

## 사양과 다르게 한 것·판단 (근거)

- 33 OS 알림: 이 worktree에 shell lane 진입점(`NativeApplication::notify_event`, shell 사양 2.5)이 없습니다. 존재하지 않는 함수를 부르면 컴파일되지 않고, 대신 아무것도 하지 않는 같은 이름의 대체 함수를 두면 동작 없는 우회 코드가 되므로 호출을 넣지 않았습니다. 병합 뒤 연결 위치: `domains/explorer_search.rs` `apply_search_replaced`의 성공 분기, `self.toasts.success(summary, now)` 바로 뒤에 `self.notify_event(context, NotificationCategory::SearchReplace, Some(&project), "notification.searchReplaceDone", summary.clone())` 한 줄(요약 문자열은 toast와 같은 것).
- 33 건너뜀 fixture: 사양은 binary·NUL fixture였지만 backend 검색이 binary를 결과에서 빼므로 패널 경로로는 binary가 교체 대상이 될 수 없습니다. 검색 결과에는 나오고 교체에서 건너뛰는 비UTF-8 파일로 같은 경로(`skipped` → warning)를 검사했습니다.
- 질의 정규화 2사례: Rust `SearchQuery`는 항상 모든 필드를 가지므로 원본 `normalizeSearchQuery`를 serde 기본값(`{"text": …}` 역직렬화)으로 옮겨 검사했습니다.
- 20 cwd: 메뉴 동작은 그 frame의 page 행에서 나오므로 원본 `resolveTargetDir`의 "page에 없는 행 → 루트" 분기는 부모 경로가 없을 때 루트로 대체하는 형태만 남습니다.

## 남은 범위·테스트 부채

- 33: OS 알림 한 줄(병합 뒤 연결, 위 위치). 실제 알림 표시는 P2(실기).
- 20·21: 실제 App에서 탐색기 행 우클릭 → 메뉴 → `finish_explorer` 구간. App 하네스(`step`/`pointer`)로 행을 우클릭하면 메뉴가 열리지 않았습니다(`egui::Popup::is_any_open` false, 같은 위치 좌클릭은 미리보기를 엶). 같은 `Explorer`를 직접 그리는 `tests/explorer.rs`에서는 열립니다. 원인(앱 입력 경로 또는 하네스 차이)은 이 묶음에서 확인하지 못했습니다. 메뉴·동작·host·결과 화면은 각각 검사했지만 구간을 잇는 App 검사가 없어 partial로 두었습니다.
- 16: `tests/explorer-delete.rs` 실제 휴지통 검사 미실행(허락 P1).
- explorer-search-38: 묶음 B에서 33으로 미뤘던 교체 host 명령과 소비자가 이번에 연결·검사되었습니다. 이 묶음 대상 ID가 아니라 대응표 행은 바꾸지 않았습니다(메인 판단).
- 테스트 부채(직접 다루지 않음): 검색 편집기 탭 기억의 "화면에 보이는 탭" 판정은 layout의 각 leaf 활성 탭 기준이라, 프로젝트가 어느 slot에도 표시되지 않는 동안에도 활성 탭은 mount로 남습니다(원본은 실제 unmount). 실행이 필요해지는 시점: 프로젝트 전환 중 검색 편집기 실행 취소 관련 결함 보고 때.
- 관찰(다른 영역): App 검사에서 Search 사이드바 탭(`search.title`)과 검색 입력 placeholder(`search.placeholder`)가 영어 locale에서 같은 이름 "Search"라 접근성 이름으로 찾을 때 role을 함께 봐야 합니다(검사는 role로 구분).

## 리뷰 반영 수정 (2026-11-03)

### 차단: explorer-search-37 찾기 위젯·AI 편집 입력 focus의 ⌘F (실제 결함, 수정)

- 확인: `application.rs`의 keymap 결과 루프가 `intent_with_editor(.., keymap_editor_scope.is_some(), ..)`로 넘기고, `keymap_editor_scope`는 `domains/editor.rs`에서 본문 `has_focus()`와 `output.focus_ids`(AI 편집 컨트롤은 `ai_control_focused`로 명시 제외)로만 켜집니다. 찾기 위젯 입력 focus(`editor_find_widget::show`의 `focused`)는 이 값을 켜지 않았습니다. 그래서 ⌘F로 찾기를 연 뒤 찾기 입력에서 ⌘F를 다시 누르면 `route_find_keymap`이 올린 `find`가 `OpenSearch`가 되어 사이드바가 Search 보기로 바뀌었습니다. 원본 `editor-area.tsx:146-161`은 `hasWidgetFocus`(편집기 container와 그 overlay·content widget 전체)로 판정하고, AI 편집 입력도 Monaco content widget(`ai-inline-edit.ts:137`)이라 같은 범위입니다.
- 재현: App 검사 `실제_app_명령_shift_f는_…`에 "찾기 입력 focus → ⌘F → 보기 Files 유지, 찾기 위젯 표시·입력 focus 유지" 단계를 추가 → 수정 전 실패(`left: Search, right: Files`, 1143행). 새 App 검사 `실제_app_ai_편집_입력에_focus가_있을_때_명령_f는_편집기_찾기를_연다`(⌘I로 AI 편집 입력 focus → ⌘F → 그 view 찾기 위젯 표시) → AI 분기만 뺀 상태에서 "editor find" 단계 시간 초과로 실패 확인.
- 수정(근본): 편집기 widget focus를 keymap 범위와 분리한 별도 값으로 올립니다. `keymap_editor_scope`를 켜는 방안은 창 단위 keymap 판정(`route_window_keys`의 `editor` 문맥)까지 바꿔 찾기 입력·AI 입력에서 편집기 전용 단축키가 창 수준에서 발화할 수 있어 택하지 않았습니다.
  - `domains/explorer_search.rs` `FrameOutput.is_editor_widget_focused` 추가(공용 파일 규칙: 새 요청은 자기 FrameOutput 필드).
  - `domains/editor.rs`: `find_output.focused || ai_control_focused`이면 그 값을 켬(편집기 영역 파일이지만 focus를 아는 곳이 여기뿐이라 3줄 추가, 사유 기록).
  - `application.rs`: `intent_with_editor` 인자를 `keymap_editor_scope.is_some() || explorer_output.is_editor_widget_focused`로.
  - 본문 focus 외의 다른 overlay(이름 바꾸기 입력 등)는 이미 `focus_ids`로 `keymap_editor_scope`를 켭니다(`editor-rename.rs:243`).

### 비차단 처리

- 검색 이력 저장 실패: `RecordRecentSearches`가 `settings_update` 호출을 복제하고 오류를 버렸습니다. 원본 `useAddRecentSearch`는 `useUpdateSettings`(onError `toastSettingsSaveFailure`) + `onSettled` 정리라 실패 toast가 뜹니다. `crate::host::update_control_settings`를 재사용하고, 실패 응답(`HostReply::SettingsFailed` → `settings.saveFailed` toast)은 작업자 sender로 먼저 보낸 뒤 `RecentSearchesSaved`로 대기 목록을 정리하도록 바꿨습니다. 새 App 검사 `실제_app_검색_이력_저장이_실패하면_…`(설정 파일 자리를 디렉터리로 바꿔 저장 실패 → Error toast, 대기 목록 비움, 설정값 불변). 수정 전 실패는 실행하지 않았습니다(이전 코드는 `let _ =`로 오류를 버려 toast 경로가 없음).
- 늦은 바꾸기 응답(33·38): 원본은 mutate 단위 콜백이라 패널 unmount 뒤 toast·재검색이 없습니다. native는 같은 mount에 새로 만든 패널에도 적용했습니다. `search_panel::Panel::is_replacing()`을 두고 `apply_search_replaced`가 바꾸기를 시작한 패널에만 적용하도록 했습니다. 새 App 검사 `실제_app_바꾸기를_시작하지_않은_새_패널은_늦게_온_바꾸기_응답을_무시한다` → 수정 전 실패(success toast 표시) 확인 뒤 통과.
- 아이콘 출처: 새 lucide SVG 7개(`case-sensitive`·`check`·`eye-off`·`history`·`regex`·`replace-all`·`whole-word`)를 `THIRD_PARTY_LICENSES.md`에 기존 690-699행 관례로 추가(`lucide-react` 1.28.0, 같은 `resources/icons/LICENSE.txt`). `regex.svg` geometry가 `node_modules/lucide-react/dist/esm/icons/regex.mjs`와 같음을 확인.
- 다시 열기 버튼 연속 클릭: 원본 `explorer-container.tsx:249-253`도 클릭마다 `closeProject(onError: notifyError)`라 두 번째 닫기 실패 toast가 같은 흐름입니다. 원본과 같아 바꾸지 않았습니다.
- 대응표: 37 비고(위 수정·검사), shell-59 남은 범위에서 ⌘⇧F/⌘⇧H 제거, 25 근거 범위 명시, 36·editor-81 한계 명시, 38 complete(교체 host 명령·소비자·건너뜀 보고가 33 App 검사로 연결·검사됨, 늦은 응답 무시 포함).

### 검증 (리뷰 반영)

| 명령 | 결과 |
| --- | --- |
| `cargo test … --lib 실제_app_명령_shift_f는_편집기의 -- --test-threads=2` | 수정 전 1 실패(재현) → 수정 뒤 1 통과 |
| `cargo test … --lib 실제_app_ai_편집_입력에_focus가 -- --test-threads=2` | AI 분기 제외 상태 1 실패(재현) → 복원 뒤 통과(아래 회귀에 포함) |
| `cargo test … --lib 실제_app_바꾸기를_시작하지_않은 -- --test-threads=2` | 수정 전 1 실패(재현) → 수정 뒤 통과(아래 회귀에 포함) |
| `cargo test … --lib 실제_app_검색_이력_저장이_실패하면 -- --test-threads=2` | 1 통과 |
| `cargo test … --lib -- --test-threads=2 search_panel_tests actual_app의_문서_선택_peek와 actual_app의_아웃라인_workspace` | 12 통과(⌘F가 있는 `application::startup_tests` 2건 포함) |
| 영역 회귀 `cargo test … --lib -- --test-threads=2 search explorer symbol_ navigation command_dispatch shell_keymap application::tab_menu_tests application::auxiliary_window_tests application::explorer_views_tests application::error_boundary_tests application::context_menu_tests toast tooltip_controlled breadcrumb application::ai_edit_tests actual_app의_문서_선택_peek와 actual_app의_아웃라인_workspace` | 251 통과 |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets` | 성공(기존 경고만: `zen.rs` unused mut, `editor-code-actions.rs` `is_applying`, inlay 검사 `fixture_with_emmet_peek`, vendor wry) |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 통과 |

UI crate는 이번 수정에서 바꾸지 않아 UI·wasm 검사는 재실행하지 않았습니다(앞 검증 재사용).

### 남은 범위 (리뷰 반영 뒤)

- 20·21: App 하네스 행 우클릭 메뉴 미열림 원인 미확인(이번에도 조사하지 않음). partial 유지.
- `reconnect_explorer_search`는 `tree_loading`만 비웁니다. host 재연결 시 유실된 `SearchFinished`·`SearchReplaced`·`RecentSearchesSaved` 때문에 패널 Running·`is_replacing`·이력 대기 목록이 남을 수 있습니다. 정리하려면 사이드바·검색 편집기 패널 전체 순회 API가 필요해 이번에 하지 않았습니다(드문 경로, 후속).
- `search-stream.rs` `Streams` 맵은 검색 편집기 탭 id별 항목을 탭 닫힘·기억 제거 때 지우지 않습니다(항목 크기 작음, 후속).
- 병합 충돌 면적(메인 참고): `command-dispatch.rs` `KEYMAP_ACTIONS`·unavailable 목록, `command-registry.rs` `NATIVE_KEYMAP_COUNT`·`NATIVE_COMMANDS`·pending 목록, `Run::OpenSearch` enum 중간 삽입, `intent_with_editor` 인자, 공개 필드 추가(`explorer::Output.reopen`, `RowIcons.decorations`, `CreateRequest.mount`, `RenameRequest.mount`, 이번 `FrameOutput.is_editor_widget_focused`).
