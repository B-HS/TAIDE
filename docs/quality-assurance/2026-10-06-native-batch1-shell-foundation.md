# Native 배치 1 단계 3 — 셸 기반·테마 Visuals·소규모 결함 수정 (2026-10-06)

상태: S1·S2·S3·S4·S5·S8 구현 완료, S6·S7 은 각각 절반만 구현(나머지 절반은 TS 소스에 해당 동작이 없어 구현하지 않음, 7절). 검증 계약 중 exit 0 이 아닌 것은 V5(전체 lib)와 V7 의 native-app `cargo fmt --check` 두 건이며 둘 다 이번 단계 변경이 원인이 아닙니다(6절).

이 단계를 시작한 시점의 작업 트리에는 이 단계 범위의 변경(아래 2절의 대부분)과 테스트가 이미 들어 있었고 QA 문서만 없었습니다. 이번 실행에서는 각 항목을 TS 소스로 다시 대조하고, 실패 상태로 남아 있던 탐색기 테스트의 근본 원인을 고치고, rustfmt 차이 2곳을 정리한 뒤 검증 계약을 처음부터 실행했습니다. 아래 "이번 실행에서 수정"이라고 적지 않은 변경은 인수한 상태 그대로이며 내용은 전부 다시 읽고 검증했습니다.

## 1. 기준 동작 확인 (TS·Tauri)

| 항목 | 기준 동작 | 근거 |
|---|---|---|
| S1 다크/라이트 판정 | `document.documentElement.dataset.appearance = theme.type` | `src/app/providers/theme-provider.tsx:49-50` |
| S1 기본 표면·글자·테두리 | body 배경 `--background`(=`app.background`), 글자 `--foreground`(=`app.foreground`), 모든 테두리 기본색 `--border`(=`app.border`) | `src/shared/styles/global.css:138-156, 298-315` |
| S1 버튼 | secondary: `bg-secondary text-secondary-foreground`(=`button.background`/`button.foreground`), hover 표면은 `button.hoverBackground`, 포커스 `focus-visible:border-ring`(=`app.focusBorder`), 모서리 `rounded-md`(=`--radius` 6px) | `src/shared/ui/button.tsx:8-15`, `src/shared/lib/theme-convert/component-contrast-pairs.ts:113-116`(buttonSecondary·buttonSecondaryHover 쌍), `global.css:156, 293-295` |
| S1 입력 | `bg-panel-input-background border-panel-input-border` | `src/features/settings/text-field.tsx:30` 외 24곳(동일 클래스) |
| S1 선택 항목 | `data-[selected=true]:bg-list-active-background data-[selected=true]:text-accent-foreground`(=`list.activeBackground`/`list.foreground`) | `src/shared/ui/command.tsx:112`, `global.css:150-151` |
| S1 메뉴·팝업 | `rounded-md border border-menu-border bg-menu-background shadow-overlay`, 항목 hover·열림 `bg-menu-item-hover` | `src/shared/ui/dropdown-menu.tsx:28, 56, 155` |
| S1 다이얼로그 | `rounded-lg`(8px) `shadow-overlay-lg` | `src/shared/ui/dialog.tsx:74` |
| S1 그림자 | `shadow-overlay: 0 2px 8px var(--taide-app-shadow)`, `shadow-overlay-lg: 0 8px 24px …` | `global.css:348-354` |
| S1 링크·흐린 글자·상태색 | 링크 `[&_a]:text-app-accent`, 흐린 글자 `--muted-foreground`(=`appSidebar.iconDefault`), 오류 `--destructive`(=`statusIndicator.error`), 경고 `statusIndicator.warning` | `global.css:149, 152`, `src/shared/lib/theme-convert/component-contrast-pairs.ts:83-100` |
| S2 창 | title "TAIDE", 1400x900, 최소 720x480, `titleBarStyle: "Overlay"`, `hiddenTitle: true` | `src-tauri/tauri.conf.json:13-26`, `src-tauri/src/lib.rs:765-782`(`from_config` 로 그대로 생성) |
| S2 타이틀바 | macOS 에서만 그림(`IS_MAC &&`), 전체가 drag region, traffic light 여백 78px | `src/widgets/app-shell/app-shell.tsx:178-182`, `src/features/window/title-bar.tsx:19-44`, `src/shared/constants/window-chrome.ts:1`, `src/shared/constants/platform.ts:1` |
| S2 drag region 동작 | 한 번 누르면 창 이동, 두 번 누르면 최대화 전환 | Tauri 2.11.5 `src/window/scripts/drag.js:103`(`e.detail === 2 ? 'internal_toggle_maximize' : 'start_dragging'`) |
| S3 Welcome 탭 | `kind === 'welcome'` 이면 창 종류와 무관하게 Welcome 화면 | `src/widgets/editor-area/pane-node-view.tsx:198-202` |
| S3 그릴 수 없는 탭 | 편집기 배경 위 가운데에 `text-sm opacity-60` 로 제목 표시 | `pane-node-view.tsx:255-266` |
| S4 빈 pane | 메인 창이고 `welcomeOnEmptyEditor`(기본 true)면 Welcome, 아니면 `text-sm opacity-40` 의 `editor.noFileOpen` | `pane-node-view.tsx:130, 267-274` |
| S5 rail 구역 | 그룹 순서대로 구역, 멤버는 열린 프로젝트 순서로 필터, 두 그룹이 주장하면 마지막 그룹에만, 나머지는 미분류 | `src/shared/lib/project-group.ts:42-52`, `project-group.test.ts:33-74` |
| S6 탭바 휠 | `scrollLeft += deltaY`(가로 휠은 브라우저 기본 스크롤) | `src/widgets/editor-area/pane-tab-bar.tsx:122-125, 261` |
| S7 선택 행 스크롤 | `rowVirtualizer.scrollToIndex(index)` 기본 정렬 `auto`: 아래로 벗어나면 행 아래를 뷰포트 아래에, 위로 벗어나면 행 위를 뷰포트 위에, 보이면 그대로 | `src/features/explorer/file-tree.tsx:130-139`, `node_modules/@tanstack/virtual-core/dist/esm/index.js:967-990` |
| S7 행 영역 | 행 div 하나가 `translateY(index*22)`·높이 22 로 클릭·hover·선택 배경을 모두 가짐, 글자는 `items-center` | `file-tree.tsx:361-368`, `src/features/explorer/file-tree-row.tsx:54-67` |
| S8 XLSX 날짜 셀 | `t="d"` 는 `cellDates` 없이 `datenum(parseDate(v, 1))` 숫자로 바뀜. 비유한 숫자는 `parseFloat` 결과 그대로, 화면은 `String(value)` | `node_modules/xlsx/xlsx.js:14958-14961, 3197-3203, 3247-3253`(0.18.5), `src/shared/lib/spreadsheet.ts:17, 34`, `src/features/preview/spreadsheet-preview.tsx:24` |

## 2. 항목별 변경

### S1. 테마 토큰 → egui 전역 Visuals

- `native/taide-native-ui/src/presentation.rs`: `visuals(theme) -> AppResult<Visuals>`(순수 함수)와 `apply_visuals(context, visuals)` 추가. `apply_visuals` 는 `Context::all_styles_mut` 로 egui 의 dark·light 두 스타일 슬롯에 같은 값을 넣어 OS 테마 전환으로 값이 되돌아가지 않게 합니다.
- 매핑(왼쪽이 egui 필드): `dark_mode` ← `theme.theme_type`, `panel_fill`·`noninteractive.bg_fill` ← `app.background`, `noninteractive.fg_stroke` ← `app.foreground`, `noninteractive.bg_stroke` ← `app.border` 1px, `inactive` 배경 ← `button.background`, `hovered`·`active` 배경 ← `button.hoverBackground`, 세 상태 글자 ← `button.foreground`, `inactive`·`hovered` 테두리 ← `panel.inputBorder`, `active` 테두리 ← `app.focusBorder`, `open` 배경 ← `menu.itemHover`, 위젯 모서리 6, `extreme_bg_color`·`text_edit_bg_color` ← `panel.inputBackground`, `selection` ← `list.activeBackground` + `list.foreground` 1px, `hyperlink_color` ← `app.accent`, `weak_text_color` ← `appSidebar.iconDefault`, `warn_fg_color`·`error_fg_color` ← `statusIndicator.warning`·`error`, `window_fill`·`window_stroke` ← `menu.background`·`menu.border`, `window_corner_radius` 8, `menu_corner_radius` 6, `popup_shadow` ← (0,2) blur 8 `app.shadow`, `window_shadow` ← (0,8) blur 24 `app.shadow`. 토큰이 하나라도 없으면 오류를 반환합니다.
- `native/taide-native-app/src/presentation-refresh.rs`: `Appearances` 에 `visuals` 필드 추가, `Appearances::new` 에서 계산.
- `native/taide-native-app/src/application.rs`: presentation 을 갱신하는 세 지점 모두에서 `presentation::apply_visuals` 호출 — 생성자(시작), 테마 응답 처리(테마 변경), 테마 미리보기 갱신.
- `native/taide-native-app/src/presentation.rs`: `apply_visuals`·`visuals` 재노출. 이번 실행에서 수정: rustfmt 가 요구하는 `use` 순서로 정리.
- 브라우저 클라이언트: 함수가 `taide_native_ui::presentation` 에 있어 `taide-remote-web` 이 이미 쓰는 `shell_colors`·`editor_appearance` 와 같은 경로로 호출할 수 있습니다. `taide-remote-web` 쪽 호출 연결은 수정 범위 밖이라 하지 않았습니다(8절 3번).
- 기존 화면이 `ui.visuals_mut()` 로 개별 지정한 색(settings-view, snippet-editor, keybinding-editor, theme-editor 등)은 건드리지 않았습니다.

### S2. 창 설정

- `native/taide-native-app/src/main.rs`: 제목 "TAIDE", 초기 1400x900, `with_min_inner_size([720, 480])`, `with_fullsize_content_view(true)`, `with_title_shown(false)`, `with_titlebar_shown(false)`. egui-winit 0.36.2 `src/lib.rs:2204-2207` 에서 이 세 옵션은 macOS 전용으로 `with_title_hidden`·`with_titlebar_transparent`·`with_fullsize_content_view` 에 대응하고 다른 플랫폼에서는 무시됩니다. traffic light 버튼은 `titlebar_buttons_shown` 기본값 true 로 유지됩니다.
- `native/taide-native-ui/src/shell.rs`: `NativeShell::has_title_bar` 추가. false 면 `title_bar` 가 아무것도 그리지 않고 본문이 창 위에서 시작합니다. true 면 타이틀 영역 전체를 drag region 으로 두고 drag 시작에 `ViewportCommand::StartDrag`, 두 번 누르면 `ViewportCommand::Maximized(!현재값)` 을 보냅니다. 제목 라벨은 `selectable(false)` 로 두어 drag 를 가로채지 않습니다.
- `native/taide-native-app/src/application.rs`: `has_title_bar: cfg!(target_os = "macos")`(TS 의 `IS_MAC` 분기).

### S3. 미지원 탭 종류 폴백

- `native/taide-native-ui/src/shell.rs` `pane_tree`: 활성 탭이 `TabKind::Welcome` 이면 기존 `welcome` 렌더를 재사용합니다(보조 창 포함, TS 와 동일).
- `native/taide-native-app/src/application.rs` `tab_content`: Diff·ClaudeDiff·SearchEditor 는 status 에 내부 영어 문구를 쓰지 않고, 편집기 배경 위 가운데에 14px·불투명도 0.6 으로 "탭 제목 + 줄바꿈 + `tab.contentUnavailable`" 을 표시합니다(TS 폴백의 `text-sm opacity-60` 과 같은 값). 상수 `UNAVAILABLE_TAB_FONT_SIZE`·`UNAVAILABLE_TAB_TEXT_OPACITY`.
- `crates/taide-locale`: 기존 키 중 가까운 것은 `editor.cannotOpen`("This file cannot be opened in the editor")과 `preview.notSupported`("Preview is not supported for this file")뿐인데 둘 다 "파일"을 전제해 diff·검색 편집기 탭에는 맞지 않습니다. 그래서 `tab.contentUnavailable` 을 `resources/locales/{en,ko,ja}.json` 과 `src/service.rs` 의 `MESSAGE_NAMESPACES`(`tab` 네임스페이스)에 추가했습니다. 문구: en "This tab can't be displayed yet", ko "이 탭은 아직 표시할 수 없습니다", ja "このタブはまだ表示できません".

### S4. welcomeOnEmptyEditor

- `native/taide-native-ui/src/snapshot.rs`: `ShellSnapshot::welcome_on_empty_editor` 추가, `read` 가 `settings.welcome_on_empty_editor` 를 채움.
- `native/taide-native-ui/src/shell.rs`: `PaneViewContext::welcome_on_empty` 로 전달. 메인 창은 설정값, 보조 창은 항상 false. 꺼져 있으면 `editor.background` 위에 14px·불투명도 0.4 의 `editor.noFileOpen`. `ShellColors` 에 `editor_background`·`editor_foreground` 추가(`presentation.rs` `shell_colors`).
- 범위 밖 최소 수정(컴파일 유지 목적, 각 1줄): `native/taide-remote-web/src/shell.rs`(브라우저 쪽 `ShellSnapshot` 생성에 필드 전달), `native/taide-native-app/tests/lsp.rs`(테스트의 `ShellSnapshot` 리터럴).

### S5. 그룹 중복 표시

- `native/taide-native-ui/src/snapshot.rs`: `project_group_sections(projects, groups) -> ProjectGroupSections` 추가. 멤버 → 그룹 id 맵을 그룹 순서대로 덮어써 마지막 그룹이 소유하게 하고, 구역 멤버와 미분류는 열린 프로젝트 순서로 필터합니다(TS `resolveProjectGroupSections` 와 같은 절차).
- `native/taide-native-ui/src/shell.rs` `project_rail`: 위 결과로 그립니다.

### S6. 탭바

- `native/taide-native-ui/src/shell.rs`: 탭바 child ui 범위에서만 `always_scroll_the_only_direction = true`. egui `containers/scroll_area.rs:1224-1232` 에서 이 옵션은 한 방향만 켜진 ScrollArea 에 세로·가로 휠 변위의 합을 적용하므로 TS 의 `scrollLeft += deltaY` + 브라우저 기본 가로 스크롤과 같습니다.
- "활성 탭이 바뀌면 보이도록 스크롤"은 구현하지 않았습니다(7절 1번).

### S7. 탐색기

- `native/taide-native-app/src/explorer.rs`: `revealing_scroll_offset(current, viewport_height, index)` 추가. 행 아래가 뷰포트 아래를 넘으면 `row_bottom - viewport_height`, 행 위가 현재 오프셋보다 위면 `row_top`, 보이면 `None`(스크롤 안 함). `Explorer::scroll_offset` 에 매 프레임 ScrollArea 의 실제 오프셋을 기록해 다음 계산의 기준으로 씁니다. 선택 이동·경로 reveal·생성 입력 행이 모두 이 함수를 거칩니다.
- 이번 실행에서 수정(근본 원인): 행의 클릭·hover·선택 배경 영역(`row_rect`)이 `ui.next_widget_position()` 에서 시작했는데, 이 함수는 "다음 위젯의 중심"을 돌려주므로(egui `layout.rs:667-671`) 영역이 행 띠보다 11px 아래로 밀려 있었습니다. 첫 행 위에 11px 빈 띠가 있었고, 선택 행을 뷰포트 아래에 맞춰도 선택 배경의 아래 11px 이 잘렸습니다. `ui.max_rect().left_top()` 으로 바꿔 TS 처럼 행 띠와 영역을 일치시켰습니다.
- 위 수정으로 불필요해진 보정 제거: 빈 영역의 시작점을 "내용 아래와 각 행 영역 아래 중 큰 값"으로 구하던 fold 는 밀린 영역을 덮기 위한 것이어서 `content_bottom` 하나로 줄였습니다.
- "선택이 없을 때 typeahead 가 맨 위부터 검색"은 변경하지 않았습니다(7절 2번).

### S8. XLSX 셀 단위 변환

- `native/taide-native-app/src/preview_spreadsheet.rs`: `cell()` 이 더 이상 오류를 반환하지 않습니다. `DataRef::Float` 는 유한 여부와 무관하게 숫자, `DataRef::DateTimeIso` 는 `iso_date_serial` 로 1899-12-30 기준 일수(밀리초 정밀도), 해석할 수 없는 문자열은 NaN, `DataRef::DurationIso` 는 문자열 그대로입니다. `iso_date_instant` 는 RFC 3339(오프셋·Z 포함), `YYYY-MM-DD`(UTC 자정), 오프셋 없는 `YYYY-MM-DDTHH:MM:SS[.fff]`(로컬 시각) 순으로 해석합니다.
- 근거 계산: SheetJS 0.18.5 는 `d = new Date(v); d += tzOffset(d)` 뒤 `datenum` 에서 로컬 기준점에 같은 `tzOffset` 을 다시 더해 빼므로, 두 시점의 오프셋이 같으면 결과는 `(new Date(v) 의 UTC 시각 − 1899-12-30T00:00Z) / 86400000` 입니다. `new Date` 는 Z·오프셋이 있으면 그 순간, 날짜만 있으면 UTC 자정, 오프셋 없는 날짜-시각은 로컬 시각으로 해석합니다.
- calamine 0.36.1 의 xlsx 판독기는 `t="d"` 를 항상 `DateTimeIso` 로 돌려주고(`src/xlsx/cells_reader.rs:652`) `DurationIso` 는 ODS 에서만 만듭니다. 따라서 `DurationIso` 가지는 match 를 빠짐없이 채우기 위한 것이고 xlsx 경로로는 도달하지 않습니다.
- 표시는 기존 `Cell::display`(`ryu_js`)가 `NaN`·`Infinity`·`-Infinity` 를 JS `String(value)` 와 같은 문자열로 냅니다.

## 3. 추가·수정한 테스트

`native/taide-native-ui/tests/workbench.rs`
- `테마_토큰은_원본_css변수의_egui_visuals로_변환되어_두_테마_슬롯에_적용된다`(S1): 내장 `taide-dark`·`taide-light` 를 runtime `theme_get` 으로 읽어 변환하고, 기대 색을 `global.css:7-135` 의 `:root` 값(다크)과 `crates/taide-theme/src/service.rs:456-587` 의 라이트 토큰(TS 가 CSS 변수로 쓰는 값)으로 고정. 모서리·그림자·테두리 두께, 두 스타일 슬롯 적용, 토큰 누락 시 오류까지 확인.
- `프로젝트_rail_구역은_열린_순서와_마지막_그룹_소속으로_한번씩만_배치한다`(S5): 두 그룹이 같은 프로젝트를 주장하는 경우, 닫힌 멤버, 그룹 순서 반전.
- `빈_pane과_welcome_탭은_설정과_창_범위에_따라_welcome과_파일없음_안내로_나뉜다`(S3·S4): 설정 on/off, Welcome 탭, 보조 창.
- `타이틀바는_mac_전용_drag_region이고_없으면_본문이_창_위에서_시작한다`(S2): 타이틀 높이만큼의 본문 이동, drag 시 `StartDrag`, 두 번 누름에 `Maximized(true)`, 타이틀바가 없을 때는 명령 없음.
- `탭바는_세로_휠을_가로_스크롤로_바꾼다`(S6): 탭 24개에서 세로 휠 뒤 첫 탭의 x 좌표 감소.
- 기존 fixture: 기본 레이아웃의 활성 탭이 Welcome 이라 표면 호출이 사라지므로 `surface_layout()` 로 Welcome 이 아닌 탭을 활성화. 이번 실행에서 수정: rustfmt 차이 1곳.

`native/taide-native-app/tests/explorer.rs`
- `탐색기_키보드_선택은_가시범위를_벗어날_때만_최소로_스크롤한다`(S7): 보이는 행으로 이동하면 목록이 움직이지 않고, 아래로 벗어나면 선택 행 아래가 뷰포트 아래에, 위로 벗어나면 선택 행 위가 목록 위에 맞음. 이번 실행에서 수정: 첫 행이 헤더(32px, `explorer-panel.tsx:203` 의 `h-8`) 바로 아래에서 시작한다는 단언(`SOURCE_HEADER_HEIGHT`) 추가, 아래 정렬 단언을 "뷰포트 아래 이하"에서 "뷰포트 아래와 일치"로 강화.

`native/taide-native-app/tests/preview-spreadsheet.rs`
- `xlsx_iso날짜와_비유한_숫자는_워크북을_실패시키지_않고_원본_라이브러리의_숫자로_표시한다`(S8): Z·날짜만·+09:00·해석 불가 `t="d"` 셀, `1e999`·`-1e999`·`NaN`, 같은 행의 공유 문자열 셀.

`native/taide-native-app/src/presentation-refresh.rs` 의 기존 테스트에 `visuals.panel_fill`·`dark_mode` 단언 2개 추가.

## 4. 실행한 명령과 결과

cargo 명령은 모두 `--locked --offline --target-dir experiments/native-shell-spike/target` 를 붙였습니다(V6 만 루트 target). 표에서는 생략합니다.

| 단계 | 명령 | exit | 결과 |
|---|---|---|---|
| V2 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test workbench` | 0 | build 1.93초, 11 passed 0.11초(S1·S2·S3·S4·S5·S6 테스트 포함). 포맷 정리 뒤 재실행 11 passed 0.13초 |
| V3 최초 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer` | 101 | 19 passed, 1 failed: `탐색기_키보드_선택은_…`(`list_top=43, end_aligned=189`). 5절 1번 |
| V3 | 같은 명령 | 0 | 행 영역 수정 뒤 build 8.30초, 20 passed 0.14초. 최종 코드 재실행 20 passed 0.13초 |
| V4 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test preview-spreadsheet` | 0 | 2 passed 0.02초. 최종 코드 재실행 2 passed 0.02초 |
| V5 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` | 101 | 322건 중 313 passed, 9 failed, 12.92초. 6절 1번 |
| V5 분류 | `… --lib -- problems::tests status_ide::tests` | 101 | 20건 중 16 passed, 4 failed 0.08초(tooltip 4건, 격리해도 실패) |
| V5 분류 | `… --lib -- projects::tests remote_dispatch::tests remote_projects::tests remote_assets::prepare_tests` | 101 | 11건 중 10 passed, 1 failed 8.85초(시간 초과 4건은 격리 시 통과, `remote_assets` 1건은 격리해도 실패) |
| V6 | `cargo test --manifest-path Cargo.toml -p taide-locale` | 0 | build 4분 35초, 20 passed 0.06초 |
| V7 | `cargo check --manifest-path native/taide-native-app/Cargo.toml` | 0 | 경고는 vendored `wry-preview` 17건뿐 |
| V7 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | 0 | 출력 없음 |
| V7 | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 0 | 최초 exit 1(`tests/workbench.rs` 1곳) → 손으로 정리 뒤 exit 0 |
| V7 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 1 | 최초 2곳(`src/presentation.rs`, `mock-server.rs`) → 앞의 것만 정리. 남은 차이는 범위 밖 `experiments/lsp-coordinator-spike/src/bin/mock-server.rs` 1곳. 6절 2번 |
| 추가 | `cargo test … --test explorer-delete --test explorer-clipboard --test paste-shortcuts` | 0 | 3개 대상 합계 11 passed(행 영역 수정의 직접 영향 범위) |
| 추가 | `cargo test … --test lsp --no-run` | 0 | `ShellSnapshot` 필드 추가 뒤 컴파일 확인 |
| 추가 | `cargo clippy --manifest-path native/taide-native-ui/Cargo.toml --lib --test workbench` | 0 | 경고 8건 모두 기존 파일(`lib.rs`, `settings-code-view.rs`, `settings-view.rs`, `snippet-editor.rs`). 변경 파일 0건 |
| 추가 | `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --lib --bin taide-native-app --test explorer --test preview-spreadsheet` | 0 | 앱 코드 경고 0건(vendored `wry` 경고만) |

## 5. 실패했다가 고친 내역

1. V3 `탐색기_키보드_선택은_가시범위를_벗어날_때만_최소로_스크롤한다`. 관찰값 `list_top=43, end_aligned=189`. 헤더가 32px 인데 첫 행 영역이 43 에서 시작했고, 선택 행 영역의 아래가 211 로 뷰포트 아래(200)를 11px 넘었습니다. 원인은 스크롤 계산이 아니라 행 영역의 기준점이었습니다(2절 S7). `ui.horizontal` 안에서 `set_min_height(22)` 뒤의 `next_widget_position()` 은 행 띠의 세로 중심(위 + 11)을 돌려줍니다. 기준점을 행 띠의 왼쪽 위로 바꾼 뒤 `list_top=32`, 아래 정렬이 뷰포트 아래와 일치했습니다. 스크롤 계산 쪽에 11px 을 더하는 식의 보정은 하지 않았습니다.
2. rustfmt 차이 2곳(`native/taide-native-app/src/presentation.rs` 의 `use` 순서, `native/taide-native-ui/tests/workbench.rs` 의 `assert_eq!` 줄바꿈)을 손으로 정리했습니다. `cargo fmt` 를 실행하면 범위 밖 `mock-server.rs` 까지 바뀌므로 실행하지 않았습니다.

## 6. 검증 계약 중 exit 0 을 만들지 못한 것

1. V5 전체 lib 9건 실패. Visuals 변경이 원인인 실패는 없습니다. 아래 세 부류이고 모두 수정 범위 밖 파일이라 고치지 않았습니다.
   - tooltip 4건(격리해도 실패): `problems::tests::problems_tooltip은_ax_role과_trigger_description을_열린_동안만_연결한다`, `problems::tests::problems_tooltip은_테마색과_원본_글꼴_여백_테두리를_렌더한다`, `problems::tests::problems_버튼은_원본_모서리와_높이_툴팁방향을_렌더한다`, `status_ide::tests::ide_tooltip은_dark_light_원본_테마와_12px_글꼴을_사용한다`. 네 테스트는 `egui::Context::default()` 를 직접 만들어 `apply_visuals` 를 거치지 않습니다. `tooltips::Provider` 는 tooltip 을 대기열에 넣고 `finish_frame` 에서만 그리는데(`native/taide-native-ui/src/tooltips.rs:540-557, 955-994`) 이 네 테스트는 `begin_frame`·`finish_frame` 을 부르지 않습니다(같은 파일의 통과하는 테스트는 부릅니다, 예 `problems-tests.rs:286-291`). `tooltips.rs`·`problems-tests.rs`·`status-ide.rs` 는 git 상 변경이 없고 같은 체크포인트 커밋(40057316)에 들어 있어 이번 배치 이전부터의 실패로 판단합니다.
   - `remote_assets::prepare_tests::production_startup과_settings는_자산_준비_실패시_listen하지_않고_성공후에만_시작한다`(격리해도 실패, `remote-assets-prepare-tests.rs:175` `tracked_count()` 가 0 이어야 하는데 1). 배치 1 단계 2 가 `application-ports.rs` 의 `Ports::start` 끝에 `start_agent_poll` 을 추가해 추적 작업이 1개 생깁니다. 단계 2 의 QA 는 `--lib application_ports` 등 일부만 실행해 이 테스트를 돌리지 않았습니다. 테스트가 고정한 "자산 준비 실패 시 추적 작업 0" 이 여전히 옳은 계약인지(폴링을 자산 준비와 무관하게 시작해도 되는지)는 단계 2 의 설계 판단이라 이 단계에서 정하지 않았습니다.
   - 시간 초과 4건(`Elapsed`, 전체 병렬 실행에서만 실패, 격리 실행에서는 통과): `projects::tests::원본처럼_watcher와_lockfile_준비_실패는_프로젝트_commit을_거절하지_않는다`, `remote_dispatch::tests::실제_project_file_raw_search_channel과_설정_sync는_동일_services와_reconcile을_사용한다`, `remote_dispatch::tests::실제_pty_spawn_attach와_layout_close는_공유_hub_core_child를_회수한다`, `remote_projects::tests::열기3진입점의_취소와_감독종료는_승인된_작업을_남기고_닫기_flush는_취소된다`. 부하 의존으로 보이며 원인은 조사하지 않았습니다.
2. V7 native-app `cargo fmt --check`. 남은 차이는 `experiments/lsp-coordinator-spike/src/bin/mock-server.rs` 의 import 순서 1곳입니다. native-app 이 이 파일을 `[[example]]` 로 포함해 edition 2024 규칙으로 검사하기 때문이고, git 상 변경이 없는 범위 밖 파일이라 두었습니다(단계 2 의 QA 6절 2번과 같은 상태). 이 단계에서 변경한 파일의 차이는 0건입니다.

## 7. 지시와 TS 소스가 달라 구현하지 않은 것 (메인 판단 필요)

1. S6 "활성 탭이 바뀌면 보이도록 스크롤". TS 에 이 동작이 없습니다. `src/widgets/editor-area` 와 `src/features/tab` 전체에서 스크롤을 건드리는 코드는 `pane-tab-bar.tsx:124` 의 `scrollLeft += deltaY` 하나뿐이고, `src` 전체에 `scrollIntoView` 호출이 0건입니다. 탭은 `tabIndex` 만 바뀌고 `.focus()` 를 받지 않아 브라우저의 포커스 스크롤도 일어나지 않습니다. 감사 문서(`2026-10-06-native-audit-shell.md` 2.7절 표의 83번)가 든 근거 줄(122-125, 287)도 휠 처리와 overlay scrollbar 입니다. "TS 에 없는 동작을 만들지 않는다"는 규칙을 따라 구현하지 않았습니다. 원본보다 나은 동작으로 추가하기로 결정하면 `ScrollArea` 안에서 활성 탭 response 에 `scroll_to_me(None)` 을 활성 탭 id 가 바뀐 프레임에만 부르는 방식으로 넣을 수 있습니다.
2. S7 "선택이 없을 때 typeahead 가 맨 위부터 검색". TS 의 실제 동작은 "무시"입니다. `file-tree.tsx:231` 의 `if (selectedIndex < 0) return` 이 254행의 typeahead 분기보다 앞에 있어, 선택이 없으면 `handleTypeahead` 에 도달하지 않습니다. 감사 문서가 근거로 든 171행의 `selectedIndex < 0 ? -1 : selectedIndex - 1` 은 그 guard 때문에 실행되지 않는 가지입니다. native 는 이미 같은 동작이고 기존 테스트(`tests/explorer.rs` `탐색기_선택검색_타이핑은_…` 의 `explorer.selected = None` 구간)가 이를 고정하고 있어 변경하지 않았습니다.

## 8. 남은 위험과 테스트 부채

1. egui `Visuals` 는 TS 가 따로 쓰는 토큰 여러 개를 한 필드로 받습니다. 내장 두 테마에서는 값이 같아 차이가 없지만 다른 테마에서는 다음이 원본과 달라질 수 있고, 해당 화면이 Frame·색을 직접 지정해야 정확해집니다.
   - `window_fill`·`window_stroke`: 메뉴·팝업·tooltip·다이얼로그가 공유. `menu.*` 를 넣었으므로 `modal.*`·`popover.*`·`tooltip.*` 가 다른 테마에서는 egui 기본 Window·Popup 이 원본과 다릅니다.
   - `selection.stroke`: 선택 항목 글자색과 포커스된 `TextEdit` 테두리가 공유. `list.foreground` 를 넣었으므로 개별 지정이 없는 입력(탐색기 이름 입력 등)의 포커스 테두리는 원본의 `app.focusBorder` 가 아닙니다.
   - `widgets.inactive.bg_stroke`: 입력 테두리와 버튼 테두리가 공유. 원본 secondary 버튼에는 테두리가 없습니다.
   - `widgets.hovered` 배경: 버튼 hover 와 메뉴 항목 hover 가 공유. `button.hoverBackground` 를 넣었으므로 `menu.itemHover` 가 다른 테마에서는 메뉴 항목 hover 가 다릅니다.
   - 입력 안 글자 선택 배경: 원본은 시스템 강조색, native 는 `list.activeBackground`.
2. 탐색기 행은 선택·hover 배경을 전역 `selection.bg_fill`·`widgets.hovered.weak_bg_fill` 에서 읽습니다(`explorer.rs` 의 기존 코드). 원본은 `explorer.itemSelected`·`itemFocused`·`itemHover` 입니다. 이번 범위가 아니라 두었습니다.
3. `taide-remote-web` 은 아직 `apply_visuals` 를 호출하지 않아 브라우저 클라이언트는 egui 기본 외형입니다.
4. XLSX 테스트 부채(지금 실행하지 않음). 재현 조건과 남은 위험:
   - 초 없는 `YYYY-MM-DDTHH:MM`, 공백 구분 `YYYY-MM-DD HH:MM:SS` 등 JS `Date` 가 받아 주는 변형은 native 에서 NaN 이 됩니다. Excel 이 쓰는 형식이 아니라 생략했습니다.
   - 오프셋 없는 날짜-시각이 로컬 DST 전환으로 존재하지 않는 시각이면 native 는 NaN, JS 는 앞으로 민 시각입니다.
   - DST 전환 직전 몇 시간 안의 시각, 1899년의 분 미만 지역 오프셋을 쓰는 시간대에서는 SheetJS 쪽 두 오프셋이 달라져 1시간 또는 수십 초 차이가 날 수 있습니다. WebKit 에서 원본을 실행해 비교해야 확정됩니다.
   - 실행이 필요해지는 시점: 실제 사용자 파일에서 `t="d"` 셀 표시 차이가 보고될 때.
5. 이번 실행 중 cargo 레지스트리 위치를 찾느라 홈 디렉터리 전체에 파일 이름 검색(`find`, 깊이 6)을 한 번 실행했습니다. 내용은 읽지 않았고 OS 가 보호 폴더 접근을 거부했습니다. 이후 검색은 저장소와 cargo 디렉터리로 한정했습니다.

## 9. 실기 확인이 필요한 것

GUI 를 실행하지 않았으므로 아래는 코드와 합성 프레임 테스트로만 확인했습니다.

- [ ] macOS: OS 타이틀과 자체 타이틀바가 겹치지 않고, traffic light 가 78px 여백 안에 들어오며, 타이틀 영역 drag 로 창이 움직이고 두 번 누르면 최대화가 전환되는지.
- [ ] 창이 1400x900 으로 열리고 720x480 아래로 줄어들지 않는지.
- [ ] 내장 다크·라이트와 다른 테마 2~3종에서 버튼·입력·메뉴·tooltip·다이얼로그 외형, 테마 전환 직후 반영.
- [ ] 탐색기: 선택·hover 배경이 글자와 같은 줄에 맞는지, 첫 행 위 빈 띠가 없어졌는지, 방향키로 화면 안에서 이동할 때 목록이 움직이지 않는지.
- [ ] 탭이 많을 때 마우스 세로 휠로 탭바가 가로로 움직이는지(트랙패드 가로 스크롤 포함).
- [ ] `welcomeOnEmptyEditor` 를 끄면 빈 pane 에 "No file is open" 계열 문구가 보이는지, diff 탭을 열었을 때 안내 문구가 보이고 상태바에 오류가 남지 않는지.
