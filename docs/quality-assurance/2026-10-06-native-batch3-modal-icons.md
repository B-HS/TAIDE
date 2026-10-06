# native 전환 배치 3 단계 2 — 공용 modal 모듈과 아이콘 레지스트리

- 작성일: 2026-10-06
- 기준 커밋: 09ced3a4
- 단계: modal-icons (배치 3, 구현 단계 2/2)
- 시작 상태: 작업 트리에 단계 1(toast-feedback)의 미커밋 변경과, 이 단계가 한 번 중단되며 남긴 미커밋 변경(`modal.rs`, `glyph-icons.rs`, 세 화면의 이전, 팔레트 아이콘, `resources/icons/`)이 있었습니다. 그 변경을 읽고 이어서 작업했으며 되돌리지 않았습니다.
- 줄 번호는 이 단계 종료 시점의 파일 기준입니다. 경로는 `/Users/hyunseokbyun/development/TAIDE` 기준입니다.

## 1. 변경 파일 (이 단계)

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-ui/src/modal.rs` (신규) | 공용 modal 모듈: `Chrome`, `Layer`, `Transition`, `Presence`, `Composition`, `FocusReturn`, `FOCUS_FILTER`, `trap_focus`, `takes_escape`, `unmount`, `layer_id` |
| `native/taide-native-ui/src/modal-tests.rs` (신규) | 모듈 테스트 11건 |
| `native/taide-native-ui/src/lib.rs` | `pub mod modal` |
| `native/taide-native-ui/src/command-palette.rs`, `command-palette-tests.rs` | 공용 모듈 사용, 열림·닫힘 전환, reduced motion, 고정 아이콘 5종 배치, 테스트 5건 추가 |
| `native/taide-native-ui/src/keybinding-editor.rs` | `Layer`·`Composition`·`FocusReturn`·`FOCUS_FILTER` 사용 |
| `native/taide-native-ui/src/snippet-editor.rs` | `Layer`·`Presence`·`Composition`·`trap_focus`·`takes_escape`·`unmount` 사용, sizing pass 에서 버튼 불투명도 모션을 표본화하지 않게 수정(10.3) |
| `native/taide-native-ui/src/icons.rs` | 팔레트 아이콘 5종(`Search`·`Terminal`·`File`·`CornerDownLeft`·`Loader`), `glyphs` 하위 모듈 선언 |
| `native/taide-native-ui/src/glyph-icons.rs`, `glyph-icons-tests.rs` (신규) | 문제 패널 아이콘 레지스트리를 공용으로 이동, 폴더 아이콘 6종·`folder()`·`FileColor::Folder` 추가, 테스트 4건 |
| `native/taide-native-app/resources/icons/` (신규) | lucide-react 1.28.0 의 `search`·`corner-down-left`·`loader-circle`·`folder`·`folder-open`·`folder-code`·`box`·`flask-conical`·`git-fork` SVG 와 `LICENSE.txt` |
| `native/taide-native-app/src/problems-icons.rs` | 공용 레지스트리 재export 로 축소(테스트 빌드는 기존 white-box 테스트를 위해 같은 소스를 포함) |
| `native/taide-native-app/src/explorer.rs` | 행의 chevron 칸과 타입 아이콘, `Appearance`·`RowIcons`·`RowIcon`·`show_with_icons` |
| `native/taide-native-app/src/application.rs` | 탐색기 아이콘 배선, 팔레트 reduced motion 배선 |
| `native/taide-native-app/src/modal.rs` (신규), `lib.rs` | 앱 테스트 빌드가 `keybinding-editor.rs` 를 include 하므로 `crate::modal` 경로 제공(`#[cfg(test)]`) |
| `native/taide-native-app/src/keybinding-editor-tests.rs` | Escape 포커스 복귀 재현 테스트 1건 |
| `native/taide-native-app/tests/explorer.rs` | 행 아이콘 배치 테스트 1건(기존 20건은 수정하지 않음) |

### 범위 밖 최소 수정 2건

| 파일 | 사유 |
| --- | --- |
| `native/taide-native-app/src/presentation-refresh.rs` (+2줄) | 표면별 Appearance 를 한곳에서 만드는 기존 구조(`Appearances`)에 `explorer` 를 추가했습니다. 테마 변경 3경로가 모두 이 구조를 거치므로 따로 배선하면 누락 위험이 있습니다. |
| `native/taide-native-app/src/tooltip-controlled-tests.rs` (+3줄) | `explorer::Output` 을 구조체 리터럴로 만드는 기존 테스트입니다. 새 필드 3개의 빈 값을 추가했습니다. 단언은 바꾸지 않았습니다. |

수정하지 않은 것: `Cargo.toml`·`Cargo.lock`, taide-locale 카탈로그(새 문자열 없음), `taide-remote-web`, `shell.rs`, `problems.rs`, `delete_dialog.rs`, `close_dialog.rs`.

## 2. M1 — 공용 modal 모듈

### 2.1 TS 근거

- `src/shared/ui/dialog.tsx:29-40` overlay: `modal-scrim fixed inset-0`, `animate-in fade-in-0` / `animate-out fade-out-0` (duration 클래스 없음).
- `src/shared/ui/dialog.tsx:62-80` content: `rounded-lg border border-modal-border bg-modal-background p-6 duration-200`, `fade-in-0 zoom-in-95` / `fade-out-0 zoom-out-95`, `onEscapeKeyDown` 이 IME 조합 중 Escape 를 무시.
- `src/shared/ui/alert-dialog.tsx:19-50`: 같은 overlay·content 클래스.
- Radix Dialog 동작(열릴 때 포커스 이동, Tab 순환, Escape, 바깥 pointer down, 닫힐 때 trigger 복귀)은 위 두 파일이 감싸는 `radix-ui` `Dialog`·`AlertDialog` 의 기본 동작입니다.

### 2.2 모듈 구성 (`native/taide-native-ui/src/modal.rs`)

| 항목 | 줄 | 역할 |
| --- | --- | --- |
| `Chrome { background, border, shadow }` | 24 | 테두리 1px·radius 8·그림자(0,8,24)·scrim(shadow 색 × 0.5) |
| `Layer { id, chrome, padding, transition }::show` | 178-242 | `transition: None` 이면 `egui::Modal` 그대로, `Some` 이면 scrim·본문 불투명도·중심 기준 scale 을 직접 그림. `egui::ModalResponse` 반환 |
| `Transition`, `Presence` | 50, 71 | 열림·닫힘 전환 표본(3장) |
| `Composition::observe` | 118-142 | IME 조합 이벤트가 있는 프레임과 조합 중인 프레임의 키 처리를 막는 판정 |
| `FocusReturn::{capture, release, settle}` | 145-175 | 이전 포커스 기억, 닫는 프레임의 복귀 요청, 이후 프레임의 복귀 유지(4장) |
| `FOCUS_FILTER` | 9 | Tab·방향키·Escape 를 포커스 위젯이 직접 받게 하는 필터 |
| `trap_focus(context, order)` | 268 | Tab/Shift+Tab 을 소비해 주어진 순서 안에서 순환 |
| `takes_escape(context, response)` | 261 | 최상위 modal·popup 없음·최상위 dismissal layer 일 때만 Escape 소비 |
| `unmount(context, layer)` | 248 | layer transform·dismissal 등록·layer 안 포커스 해제 |

### 2.3 화면별 적용과 보존한 고유 동작

| 화면 | 공용 모듈로 옮긴 것 | 화면에 남긴 것(고유 동작) |
| --- | --- | --- |
| 커맨드 팔레트 | `Layer`(padding 0), `Presence`, `Composition`(raw 이벤트), `FocusReturn`, `FOCUS_FILTER` | 탐색 키 처리(`take_navigation`), 입력에 매 프레임 포커스 유지, 동작으로 닫힐 때 포커스를 되돌리지 않는 분기 |
| 키바인딩 편집기 | `Layer`(전환 없음), `Composition`(raw 이벤트), `FocusReturn`, `FOCUS_FILTER` | 가상화된 행을 대상으로 하는 Tab 순환(`tab_targets`), 캡처 중 키 가로채기, Escape 가 캡처 취소와 닫기를 구분하는 분기 |
| 스니펫 편집기 | `Layer`, `Presence`, `Composition`(가공 이벤트), `trap_focus`, `takes_escape`, `unmount`, `FOCUS_FILTER` | 다이얼로그 종류별 초기 포커스, New 다이얼로그만 바깥 클릭으로 닫힘 |

- 세 화면의 기존 테스트는 수정하지 않았습니다(추가만 했습니다). 앱 lib 344건·native-ui lib 111건·`snippet-editor` 18건이 통과합니다(9장).
- 초기 포커스 이동은 화면마다 대상과 조건이 달라(입력·검색·취소 버튼, sizing pass 제외 조건) 모듈로 옮기지 않았습니다. 공통 부분인 `FOCUS_FILTER` 만 공유합니다.
- 스니펫 편집기는 native 에서 원래 닫힐 때 trigger 로 포커스를 되돌리지 않았고 이번에도 추가하지 않았습니다(기존 테스트가 다루지 않는 새 동작이라 11장에 남깁니다).

### 2.4 옮기지 않은 다이얼로그와 차이

| 파일 | 현재 구성 | 공용 모듈과의 차이 | 옮기지 않은 이유 |
| --- | --- | --- | --- |
| `native/taide-native-app/src/delete_dialog.rs` | `egui::Modal::new` 기본 frame·backdrop, 취소 버튼 초기 포커스, `is_top_modal && !any_popup_open` 일 때 Escape 소비 | 테마 chrome 없음, dismissal layer 확인 없음, IME 판정 없음, 포커스 복귀 없음, 전환 없음 | 복제된 구성이 아니라 egui 기본 modal 입니다. 옮기면 색·테두리·Escape 조건이 바뀌고 `tests/explorer-delete.rs` 가 보는 버튼 배치에 영향이 갑니다. |
| `native/taide-native-app/src/close_dialog.rs` | `egui::Modal::new` 기본, `response.should_close()` 로 취소 | 위와 같음. busy 중 spinner | 같은 이유. TS `close-dirty-tab-dialog.tsx`(AlertDialog) 재현은 별도 작업입니다. |
| `native/taide-native-app/src/system-usage-view.rs:312`, `native/taide-native-ui/src/theme-editor.rs:584` | `egui::Modal::new` 기본 | 위와 같음 | 수정 범위 밖이고 복제된 구성이 아닙니다. |

## 3. M3 — 열림·닫힘 전환

- TS 값: 본문 `duration-200` + `fade-in-0`(opacity 0→1) + `zoom-in-95`(scale 0.95→1), 닫힘은 반대(`dialog.tsx:74`). overlay 는 duration 클래스가 없어 `node_modules/tw-animate-css/dist/tw-animate.css` 의 `--animate-in: enter var(--tw-animation-duration,var(--tw-duration,.15s)) var(--tw-ease,ease)` 기본값 150ms 를 씁니다. easing 은 둘 다 CSS `ease`.
- 구현: 스니펫 편집기에 있던 Dialog Presence(본문 `Motion::with_duration(.., 0.2)`, scrim `Motion::new` 150ms, `css_motion::ease`)를 `modal::Presence` 로 옮겨 팔레트와 공유합니다. `Presence::sample` 이 `Transition { opacity, scale, scrim_opacity, is_active }` 를 돌려주고 `Layer::show` 가 적용합니다.
- 팔레트(`command-palette.rs` `sync_presence`, `show`, `close`): 닫히면 200ms 동안 layer 를 유지하되 입력을 받지 않고, 끝나면 `modal::unmount` 합니다. (리뷰 후속 12.1: 이 200ms 동안은 egui modal layer 로 등록하지 않습니다.) 닫힘 전환 중 다시 열면 열림 전환을 처음부터 시작합니다. 닫는 순간 질의를 비우는 것은 TS `command-palette.tsx:153-156`(`if (!next) setQuery('')`)과 같습니다.
- reduced motion: `Presence::set_reduced_motion(true)` 면 전환 없이 즉시 나타나고 사라집니다. 앱은 `application.rs` `show_palette` 에서 `motion_preference.current()` 를 팔레트에 전달합니다(toast 와 같은 출처). 스니펫 편집기는 기존에도 reduced motion 을 받지 않았고 이번에 배선하지 않았습니다(11장).
- 키바인딩 편집기는 `transition: None` 입니다(작업 지시가 팔레트 적용만 요구). TS 는 같은 Dialog 라 전환이 있습니다(11장).

## 4. M2 — 키바인딩 편집기 Escape 포커스 복귀

- 결론: 문제가 실제로 있습니다. 원인은 egui 0.36.2 입니다.
  - `egui-0.36.2/src/memory/mod.rs:640`: 최상위 modal layer 는 프레임이 끝날 때 다음 프레임용으로 넘어갑니다(`top_modal_layer = top_modal_layer_current_frame.take()`). 닫은 다음 프레임에도 직전 modal 이 최상위로 남습니다.
  - `egui-0.36.2/src/context.rs:1256-1277`: 위젯을 만들 때 `allows_interaction(layer)` 가 거짓이면 `surrender_focus(id)` 를 부릅니다. modal 아래 layer 의 위젯은 닫은 다음 프레임에 포커스를 내놓습니다.
  - 기준 커밋의 편집기 `close` 는 닫는 프레임에 `request_focus(previous)` 한 번만 했습니다.
- 재현 테스트
  - `modal-tests.rs` `닫는_프레임의_포커스_요청만으로는_다음_프레임에_배경_위젯이_포커스를_내놓는다`: 닫는 프레임의 요청만 하고 다음 프레임을 그리면 포커스가 `None` 이 됩니다(원인 고정).
  - `keybinding-editor-tests.rs` `keybinding_escape로_닫힌_뒤_이어지는_프레임에도_이전_포커스가_유지된다`: 배경 위젯을 매 프레임 다시 그리면서 Escape 로 닫은 뒤 3프레임 동안 이전 포커스가 유지되는지 봅니다.
  - 구현을 끄고 실패를 재현하는 편집은 금지라 기준 구현에서의 실패 실행은 하지 않았습니다. 위 첫 테스트가 같은 조건의 실패를 고정합니다.
- 수정(한 곳): `FocusReturn::settle(context, is_layer_mounted)`. 닫힌 뒤 프레임(전환 중 포함)에 복귀 요청을 반복하고 layer 가 사라진 첫 프레임에 마지막으로 요청한 뒤 끝냅니다. 팔레트의 "한 프레임 미룸" 구현도 이것으로 바꿨습니다. (리뷰 후속 12.1 에서 닫힌 다음 프레임에 한 번만 요청하는 `settle(context)` 로 바뀌었습니다.)
- 전제: 화면의 `show` 가 배경 위젯보다 뒤에 호출돼야 합니다. 앱은 `shell.show`(`application.rs:4130` 부근) 뒤에 `keybindings.show`·`show_palette` 를 부르므로 만족합니다.

## 5. M4 — 아이콘 레지스트리

- `icons.rs` `Icon` 에 5종 추가. 크기는 TS 클래스 그대로입니다: `Search`·`Terminal`·`File`·`CornerDownLeft` 16px(`size-4`), `Loader` 12px(`size-3`).
- SVG 는 `node_modules/lucide-react/dist/esm/icons/{search,corner-down-left,loader-circle}.mjs`(v1.28.0)의 `__iconNode` 와 대조했습니다. `Terminal`·`File` 은 기존 `resources/problems/` 파일을 재사용합니다. `Loader2` 는 lucide 의 `loader-circle` 별칭입니다.
- 팔레트 배치(TS 근거 → native)

| 위치 | TS | native (`command-palette.rs`) |
| --- | --- | --- |
| 입력 왼쪽 | `src/shared/ui/command.tsx:50-51` `px-3 gap-2`, `SearchIcon size-4 opacity-50` | `INPUT_PADDING_X` 12, `INPUT_ICON_GAP` 8, `INPUT_ICON_OPACITY` 0.5 |
| 항목 왼쪽 | `command.tsx:112` `gap-2 px-2`, svg `size-4 text-muted-foreground` | `ITEM_PADDING_X` 8, `ITEM_GAP` 8, muted 색 |
| 명령 | `src/features/command-palette/command-palette-commands-group.tsx:34` `Terminal` | `Icon::Terminal` |
| 줄 이동 | `command-palette-line-group.tsx:18` `CornerDownLeft` | `Icon::CornerDownLeft` |
| 파일 | `command-palette-files-group.tsx:48` 고정 `File`(타입 아이콘 아님) | `Icon::File` |
| 갱신 중 | `command-palette-files-group.tsx:34-38` `gap-1.5`, `gap-1`, `Loader2 size-3 animate-spin` | `HEADING_GAP` 6, `REFRESHING_GAP` 4, 1초에 한 바퀴 등속 회전 |

- 문제 패널 아이콘: `problems-icons.rs` 의 `Glyph`·`FileColor`·`file()`·`Icons` 를 `native/taide-native-ui/src/glyph-icons.rs`(`taide_native_ui::icons::glyphs`)로 옮겼습니다. 리소스 위치(`resources/problems/`), tint, (글리프, 픽셀 크기)별 텍스처 캐시, 배율 변경 시 캐시 비움은 그대로입니다. 텍스처 이름과 오류 문구의 "Problems" 는 "glyph" 로 바뀌었습니다(테스트가 보지 않는 내부 식별자).
- 앱 `problems-icons.rs` 는 재export 입니다. 테스트 빌드에서는 `keybinding-editor.rs` 와 같은 방식으로 공용 소스를 `include!` 해 기존 white-box 테스트 `problems-icons-tests.rs`(비공개 필드 `trees`·`textures`·`SOURCES` 사용)를 한 글자도 바꾸지 않고 통과시킵니다. `problems.rs` 는 수정하지 않았습니다.

## 6. M5 — 파일·폴더 타입 아이콘

- 매핑 근거: `src/shared/lib/file-icon.ts`(`SPECIAL_FILE_NAME_ICON`, readme/license/licence/.env 접두사, `EXTENSION_ICON`, `SPECIAL_FOLDER_NAME_ICON`, 기본 folder/folder-open), `src/shared/lib/file-extension.ts`(마지막 점이 맨 앞이면 확장자 없음), `src/shared/icons/file-icon-registry.ts`(이름 → lucide 컴포넌트 28종).
- 구현: `glyph-icons.rs` `file(name)`, `folder(name, is_expanded)`, `FileColor::theme_key`.
- 색: TS 색 클래스 9종 → 테마 키. `text-status-{info,warning,error,success}` → `statusIndicator.*`, `text-git-{renamed,conflicted,staged}` → `git.*`, `text-app-sidebar-icon-default` → `appSidebar.iconDefault`, `text-explorer-folder-icon` → `explorer.folderIcon`. 모든 builtin 테마에 두 키가 있음을 확인했습니다.
- 테스트(`glyph-icons-tests.rs`): `file-icon.test.ts` 의 파일 사례 46건과 폴더 사례 전부, `file-icon-registry.test.tsx` 의 대표 이름 16+6건과 "이름마다 서로 다른 글리프"(28종을 14px 로 래스터화해 픽셀이 모두 다름), 색 9종의 테마 키.
- 자산 범위: TS 가 쓰는 28종 전부를 옮겼습니다.

| 출처 | 글리프 |
| --- | --- |
| 기존 `resources/problems/` (22종) | file, file-code, component, file-json, book-text, palette, globe, cog, coffee, terminal, file-cog, lock, image, file-text, file-archive, package, settings-2, git-branch, container, book-marked, scale, key |
| 신규 `resources/icons/` (6종) | folder, folder-open, folder-code, box, flask-conical, git-fork |

- 라이선스: lucide(ISC, 일부 Feather MIT). 기존 `native/taide-native-app/LICENSE-LUCIDE` 가 있고, 폴더별 고지 방식에 맞춰 `resources/icons/LICENSE.txt` 를 `resources/problems/LICENSE.txt` 와 같은 내용으로 두었습니다.

## 7. M6 — 탐색기 행

- TS 근거: `src/features/explorer/file-tree-row.tsx:21-23`(`ROW_INDENT_PX` 12, 아이콘 `size-3.5`, chevron `size-3`), `:60-76`(`paddingLeft: depth * 12`, `gap-1`, chevron 칸 `size-4` 이고 파일이면 `invisible`, 펼치면 `rotate-90`, 폴더는 `FolderTypeIcon(name, expanded)`, 파일은 `FileTypeIcon(name)`), `src/features/explorer/file-tree-draft-row.tsx:44-52`(빈 chevron 칸, 입력 중인 이름으로 아이콘 결정, 폴더는 `expanded=false`), `src/features/explorer/file-tree.tsx:16`(행 높이 22).
- 구현(`native/taide-native-app/src/explorer.rs`)
  - 배치: 들여쓰기 `depth × 12` → chevron 칸 16(안에 12px chevron, 폴더만) → 간격 4 → 타입 아이콘 14 → 간격 4 → 이름. `RowIcon::new`(행 rect 기준 계산), `of_row`, `of_draft`.
  - chevron 은 `Glyph::ChevronRight` 를 펼침 시 90도 회전해 그립니다. 색은 행 글자색(`ui.visuals().text_color()`, TS 의 currentColor)입니다.
  - 생성·이름 변경 입력 행도 같은 칸을 두고 입력 중인 이름으로 아이콘을 정합니다.
  - `Explorer::show` 시그니처는 그대로이고 배치만 계산합니다. `show_with_icons(.., RowIcons { glyphs, appearance })` 가 같은 경로로 실제 아이콘을 그립니다. 앱은 `show_with_icons` 를 씁니다.
  - `Output` 에 `icons`(경로별 `RowIcon`), `draft_icon`, `icon_error` 추가.
  - 행 높이 22, 전체 행 hit 영역, 선택·hover 배경, 키보드·typeahead 는 건드리지 않았습니다. 아이콘은 배경 뒤, 이름 앞에 그립니다.
- 테스트: `tests/explorer.rs` `탐색기_행은_들여쓰기_뒤에_chevron_칸과_타입_아이콘을_두고_이름을_그_뒤에_그린다` — 펼친 `src`(FolderCode·Info·90도), 깊이 1의 `main.rs`(Cog·Staged·chevron 없음), 접힌 `notes`(Folder·Folder·0도)의 칸 위치, 실제 테마로 그렸을 때 텍스처 4장, 생성 입력 행의 빈 chevron 칸.
- 팔레트 파일 행: TS 가 고정 `File` 아이콘을 쓰므로(5장 표) 타입 아이콘을 적용하지 않았습니다.
- 탭 아이콘: 이 단계에서는 적용하지 못했고(11장 1번) 리뷰 후속 12.2 에서 적용했습니다.

## 8. 실행 순서 요약

1. `git status`·`git diff --stat` 으로 중단된 이전 실행분을 확인하고 전부 읽음.
2. native-ui lib 전체 실행 → modal 테스트 6건 실패 확인 → 테스트 장면 수정(10.1).
3. M2 원인 고정 테스트 추가.
4. M6 테스트 작성 → 컴파일 실패 확인 → 구현 → 통과.
5. M4 앱 쪽 이전, M3 reduced motion 배선.
6. 검증 계약 실행(9장).

## 9. 실행한 검증

| 순서 | 명령 | exit | 결과 |
| --- | --- | --- | --- |
| 1 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --locked --offline --target-dir experiments/native-shell-spike/target` (이어받은 상태 그대로) | 101 | 104 passed, 6 failed, 0.46s. 실패는 모두 신규 `modal::tests`(10.1). 기존 팔레트 테스트는 전부 통과 |
| 2 | 같은 명령 + 필터 `modal` (테스트 장면 수정·M2 테스트 추가 뒤) | 0 | 11 passed, 100 filtered out, 0.01s |
| 3 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer --locked --offline --target-dir …` (M6 구현 전) | 101 | 컴파일 오류 6건(`Appearance`·`RowIcons`·`icons`·`draft_icon`·`show_with_icons` 없음). 실패하는 테스트를 먼저 확인. 앱 lib 은 M4 재export 상태로 컴파일됨 |
| 4 | `cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir … --message-format short` | 0 | 29.46s, 앱·native-ui 경고 없음(vendor `wry` 경고 17건은 기존) |
| 5 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer --test explorer-clipboard --test explorer-delete --locked --offline --target-dir …` | 0 | 21 / 5 / 4 passed, 0.13s / 0.05s / 3.58s |
| 6 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check`, `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 1 / 0 | 앱 4곳 차이 → 반영 뒤 0(10.2) |
| 7 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir …` | 101 | `tooltip-controlled-tests.rs:232` 의 `explorer::Output` 리터럴에 새 필드 누락(10.2) |
| 8 | 같은 명령(리터럴 보완 뒤) | 0 | 344 passed, 0 failed, 15.42s (단계 1 의 343 + 신규 1) |
| 9 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --locked --offline --target-dir …` | 0 | 111 passed, 0 failed, 0.43s |
| 10 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test snippet-editor --features inspection --locked --offline --target-dir …` | 101 | 17 passed, 1 failed: `snippet_button_hover는_실제_도형에서_즉시_점프하지_않고_150ms에_완료된다`(10.3) |
| 11 | 같은 명령(`add_button` 수정 뒤) | 0 | 18 passed, 0 failed, 0.38s |
| 12 | 순서 9 명령 재실행(`snippet-editor.rs` 수정 영향) | 0 | 111 passed, 0 failed, 0.46s |
| 13 | 순서 8 명령 재실행(같은 이유) | 0 | 344 passed, 0 failed, 13.85s |
| 14 | 순서 5 명령 재실행(포맷 반영·`snippet-editor.rs` 수정 뒤 최종 소스) | 0 | 21 / 5 / 4 passed, 0.13s / 0.06s / 3.58s |
| 15 | `cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir …` (최종 소스) | 0 | 2.71s, 앱·native-ui 경고 없음 |
| 16 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir …` | 0 | 7.42s, 경고 없음 |
| 17 | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check`, `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` (최종 소스) | 0 / 0 | 차이 없음 |

- 최종 상태 근거: V2 는 순서 11·12, V3 는 순서 13, V4 는 순서 14, V5 는 순서 15~17 입니다.
- 순서 17 의 native-ui 포맷 검사는 순서 16 이 끝나기 전에 시작했습니다. `cargo fmt` 는 target 디렉터리를 쓰지 않아 결과에 영향은 없으나 "한 번에 하나" 규칙을 한 번 어겼습니다.
- 앱 lib 테스트 빌드의 경고 1건은 링커 stderr(`__eh_frame section too large`)이고 소스 경고는 없습니다.

- V1: 추출 전 기준은 직전 배치 결과(앱 lib 336, native-ui lib 87)와 단계 1 결과(343, 91)를 재사용했습니다. 이어받은 상태에서 기존 팔레트·키바인딩·스니펫 테스트를 다시 실행해 기대값 수정 없이 통과함을 확인했습니다(순서 1, 8, 11).
- 화면 외형은 GUI 를 실행하지 않아 확인하지 못했습니다(11장).

## 10. 실패했다가 고친 내역

### 10.1 신규 modal 테스트 6건 (이어받은 미완성 테스트)

- 원인: egui Area 의 첫 프레임은 sizing pass 라 내용이 보이지 않고 비활성입니다. 비활성 위젯은 만들어질 때 포커스를 내놓고(`context.rs:1274-1277`) 아무것도 그리지 않습니다. 테스트 장면이 첫 프레임에 포커스를 주고, 첫 프레임의 scrim 도형을 확인하고 있었습니다.
- 수정: 테스트 장면만 고쳤습니다. `Scene::open` 은 layer 를 한 프레임 그린 뒤 포커스를 주고, 전환 도형 테스트는 두 번째 프레임을 확인합니다. 구현과 단언 값은 바꾸지 않았습니다.

### 10.2 컴파일·포맷

- `application.rs` 편집 중 `shell: NativeShell` 의 공백이 빠졌고 rustfmt 가 잡았습니다. 손으로 쓴 줄바꿈 3곳도 rustfmt 결과에 맞췄습니다.
- `tooltip-controlled-tests.rs` 가 `explorer::Output` 을 리터럴로 만들어 새 필드가 필요했습니다(1장 범위 밖 수정).

### 10.3 `snippet_button_hover는_…_150ms에_완료된다` 실패

- 관찰값: 삭제 확인 다이얼로그의 확인 버튼 색이 `#F38BA8FF` 여야 하는 프레임에 `#E985A1F5`(약 0.962배).
- 원인: `snippet-editor.rs` `add_button` 이 버튼 불투명도를 150ms 모션(`snippet-button-opacity`)으로 표본화합니다. 다이얼로그가 처음 그려지는 프레임은 sizing pass 라 UI 가 비활성이고, 이때 비활성 불투명도 0.5 가 모션의 시작값으로 기록됩니다. 다음 프레임에 1.0 으로 전환을 시작해 100ms 뒤 프레임에서 `0.5 + 0.5 × ease(2/3) ≈ 0.962` 가 됩니다. 관찰값과 일치합니다.
- 이 단계의 추출과 무관하다고 판단한 근거: `add_button`·`button-color-motion.rs`·테스트 파일은 기준 커밋과 같고(`git diff` 없음, `git log -S"snippet-button-opacity"` 는 40057316), modal 을 그리는 순서와 값은 추출 전후가 같습니다. 이 테스트의 통과 기록은 `docs/quality-assurance/2026-10-06-m8-snippet-button-color-motion.md:25` 가 마지막이고, 그 뒤 불투명도 모션을 넣은 `2026-10-06-m8-snippet-button-state-motion.md` 는 다른 테스트만 필터로 실행했습니다. 기준 커밋에서의 실패를 직접 실행해 확인하지는 못했습니다(작업 트리를 되돌리는 명령 금지).
- 수정: sizing pass 에서는 모션을 표본화하지 않고 목표값을 그대로 씁니다(`if ui.is_sizing_pass() { target_opacity } else { animate_amount(..) }`). 첫 표시 프레임이 실제 상태에서 시작합니다. 테스트 기대값은 바꾸지 않았습니다. 수정 뒤 `snippet-editor` 18건 전부 통과합니다.

## 11. 남은 위험과 실기 확인

1. (리뷰 후속 12.2 에서 해소. 에이전트 터미널 탭만 12.4 에 남습니다.) 탭 아이콘 미적용. TS 는 `src/widgets/editor-area/pane-tab-bar.tsx:60-70` 에서 파일·untitled 탭에 `FileTypeIcon`, 그 밖에 `Terminal`·`Settings`·`FileDiff`·`FileSearch2`·`Sparkles` 를 쓰고 `src/features/tab/tab-item.tsx:44` 의 14px 칸에 둡니다. 적용하려면 (a) lucide `settings`·`file-diff`·`file-search-2`·`sparkles` 4종 추가, (b) `ShellSurfaces` 에 탭 아이콘 메서드 추가(`native/taide-native-ui/tests/workbench.rs` 의 구현체 영향), (c) 탭 폭 계산을 보는 기존 workbench 테스트 확인이 필요합니다. 이 단계에서는 검증 계약을 먼저 닫기 위해 하지 않았습니다.
2. 실기 확인 필요(GUI 미실행)
   - [ ] 팔레트 열림·닫힘 200ms fade + 95% zoom, scrim 150ms, reduced motion 설정 시 즉시 표시
   - [ ] 팔레트 입력의 Search 아이콘(50% 불투명), 항목 아이콘, 갱신 중 spinner 회전
   - [ ] 탐색기 행의 chevron 방향·회전, 14px 아이콘 선명도(1x·2x), 테마별 색, 선택·hover 배경 위 가독성
   - [ ] 생성·이름 변경 입력 행에서 이름을 입력할 때 아이콘이 바뀌는지
   - [ ] 키바인딩 편집기·팔레트를 Escape 로 닫은 뒤 포커스가 편집기·터미널로 돌아오는지
   - [ ] 스니펫 다이얼로그가 열릴 때 버튼이 반투명에서 시작하지 않는지(10.3)
3. 키바인딩 편집기에는 열림·닫힘 전환이 없습니다(`transition: None`). TS 는 같은 Dialog 전환을 씁니다. 캡처·포커스 테스트가 첫 프레임 상태에 의존해 이번에는 넣지 않았습니다.
4. 스니펫 편집기: 닫힐 때 trigger 로의 포커스 복귀와 reduced motion 배선이 없습니다(기존 상태 유지).
5. `FocusReturn::settle` 은 화면의 `show` 가 배경 위젯보다 뒤에 호출된다는 전제에 의존합니다(4장).
6. 탐색기 행의 `pr-2`, git 상태 글자색·배지, 입력 칸 스타일은 옮기지 않았습니다(Git 데코레이션은 비목표).
7. 탐색기용 `Icons` 캐시와 문제 패널의 `Icons` 캐시는 별도 인스턴스입니다. 같은 글리프를 두 번 래스터화할 수 있습니다.
8. 앱 테스트 빌드는 `glyph-icons.rs` 를 `include!` 로 한 번 더 컴파일합니다(`keybinding-editor.rs` 와 같은 방식). `problems-icons-tests.rs` 를 native-ui 로 옮기면 없앨 수 있으나 파일 삭제가 필요해 하지 않았습니다. 탐색기도 앱 안에서는 `crate::problems_icons` 경로로 레지스트리를 씁니다(이름은 그대로 두었습니다).
9. `delete_dialog.rs`·`close_dialog.rs` 등 egui 기본 modal 4곳은 공용 모듈을 쓰지 않습니다(2.4).

## 12. 리뷰 후속

- 작성일: 2026-10-06. 리뷰어가 올린 차단 항목 2건을 실제 코드와 TS 근거로 다시 확인했고, 둘 다 옳은 지적이라 수정했습니다(반증 없음).
- 줄 번호는 이 절 작성 시점의 파일 기준입니다. egui 는 두 crate 모두 `native/taide-native-app/vendor/egui-input`(0.36.2 패치본)을 씁니다(`native/taide-native-ui/Cargo.toml:54`).

### 12.1 차단 항목 1 — 닫힘 전환 중인 팔레트가 뒤 화면의 포커스를 빼앗음 (수정)

확인한 사실

- egui: 프레임이 끝날 때 그 프레임의 modal layer 가 다음 프레임의 최상위로 넘어갑니다(`vendor/egui-input/src/memory/mod.rs:653`). 위젯을 만들 때 `allows_interaction(layer)` 가 거짓이면 `surrender_focus(id)` 를 부릅니다(`vendor/egui-input/src/context.rs:1360-1380`, `memory/mod.rs:992-1011`).
- 앱: 셸은 `!self.palette.is_open()` 이면 활성으로 그려집니다(`application.rs:4133`). 에디터는 `focus` 가 참인 프레임에 한 번만 `response.request_focus()` 를 부르고(`native/taide-native-ui/src/editor_surface.rs:361-362`), 같은 프레임에 `*self.focused` 가 갱신되어 다음 프레임부터 `focus` 가 거짓이 됩니다(`application.rs:5587-5602`, `5689-5691`). 터미널도 같습니다(`application.rs:5151-5184`).
- 기존 구현은 닫힘 전환 200ms 동안 매 프레임 `set_modal_layer` 를 불렀으므로, 닫힌 다음 프레임에 셸이 요청한 포커스가 그다음 프레임의 위젯 생성 때 사라졌습니다.
- TS 근거: Radix Dialog 는 exit 동안 Presence 로 본문을 남겨 두지만(`node_modules/@radix-ui/react-dialog/dist/index.mjs:134`) 포커스 가두기는 열려 있을 때만 켭니다(`:152` `trapFocus: context.open`, `:224` `trapped: trapFocus`). `src/shared/ui/dialog.tsx`·`alert-dialog.tsx` 가 이 컴포넌트를 그대로 감쌉니다.

재현

- `command-palette-tests.rs:1032` `동작으로_닫힌_뒤_뒤_화면_위젯이_요청한_포커스는_닫힘_전환이_끝난_뒤에도_유지된다` 를 먼저 추가했습니다. 명령을 Enter 로 실행해 닫은 뒤, 닫힘 전환의 첫 프레임에 뒤 화면 위젯이 `request_focus` 를 한 번 부르고 20ms 간격 4프레임과 전환 종료 뒤까지 포커스를 확인합니다.
- 수정 전 실행 결과: 두 번째 닫힘 프레임에서 `left: None, right: Some(Id::new("synthetic-previous-focus"))` 로 실패했습니다(12.3 순서 1).

수정

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-ui/src/modal.rs:175-214` | `Layer` 에 `is_modal` 추가. 거짓이면 `set_modal_layer` 를 부르지 않고 `ModalResponse::is_top_modal` 을 거짓으로 돌려줍니다. 전환이 없는데 modal 도 아닌 조합은 `Transition::SETTLED` 로 같은 경로에서 그립니다(egui `Modal` 은 항상 modal 로 등록하므로). |
| `native/taide-native-ui/src/modal.rs:165-171` | `FocusReturn::settle(context)`. 닫힌 다음 프레임에 한 번 요청하고 끝냅니다. 닫는 프레임에 등록된 modal layer 가 다음 프레임 하나에만 남기 때문에 반복이 필요 없고, 반복하면 그 사이의 다른 포커스 요청을 덮어씁니다. 쓰이지 않게 된 `is_layer_mounted` 인자는 제거했습니다. |
| `native/taide-native-ui/src/command-palette.rs:389-395` | `is_modal: is_open`. 닫힘 전환 프레임은 modal 이 아닙니다. |
| `native/taide-native-ui/src/keybinding-editor.rs`, `snippet-editor.rs` | `is_modal: true` 로 기존 동작을 그대로 둡니다. |

- 뒤 화면의 포인터 차단은 유지됩니다. egui 의 hit test 는 modal layer 를 보지 않고 위 layer 의 위젯이 포인터 주변을 다 덮으면 그 아래 layer 를 제외합니다(`vendor/egui-input/src/hit_test.rs:66`, `:127-132`). `Layer::show` 가 닫힘 프레임에도 화면 전체 backdrop 을 `Sense::CLICK | Sense::DRAG` 로 등록합니다.
- dismissal layer 등록은 닫힘 프레임에도 그대로 둡니다. Radix 의 DismissableLayer 도 unmount 전까지 스택에 남습니다.

테스트 변경

- `command-palette-tests.rs` `닫힘_전환_동안_레이어는_남아_있지만_키를_받지_않고_200ms_뒤에_사라진다`: `top_modal_layer == Some(dialog_layer)` 단언을 제거했습니다(이 수정과 반대 내용). 포인터 차단 의도는 새 테스트 `닫힘_전환_동안_뒤_화면은_포인터_입력을_받지_않고_전환이_끝나면_다시_받는다`(`:1064`)로 옮겼습니다. 같은 테스트 안에 두지 않은 이유는 egui 가 다른 곳을 누르면 포커스 위젯의 포커스를 내려놓게 해(`vendor/egui-input/src/context.rs:1669-1676`) 기존 테스트의 포커스 복귀 단언과 충돌하기 때문입니다. 새 테스트는 닫힘 전환 안에서 뒤 화면 버튼을 클릭해 0회, 전환이 끝난 뒤 같은 클릭으로 1회를 확인합니다.
- 테스트 장면에 프레임 간격(`frame_step`), 프레임 안 한 번의 포커스 요청(`should_focus_previous`), 클릭 수를 세는 뒤 화면 버튼을 추가했습니다. 기존 18건의 단언은 위 1건 외에 바꾸지 않았습니다.
- `modal-tests.rs`: `닫힘_전환_중인_레이어는_modal로_등록되지_않아_뒤_화면_위젯이_요청한_포커스를_빼앗지_않는다`(`:262`) 추가. 전환이 있는 경우와 없는 경우 모두 확인합니다. 장면의 `Exiting` 단계는 `is_modal: false` 로 그리고 `settle` 호출에서 인자를 뺐습니다. 기존 11건의 단언은 바꾸지 않았습니다.

이 수정으로 달라진 이전 절의 내용: 2.2 표의 `FocusReturn::settle` 설명, 3장의 팔레트 닫힘 설명, 4장의 "복귀 요청을 반복" 문장.

### 12.2 차단 항목 2 — 탭 아이콘 (수정)

TS 근거와 native 대응

| TS (`src/widgets/editor-area/pane-tab-bar.tsx:60-70`) | native (`native/taide-native-ui/src/glyph-icons.rs:328-340` `tab`) |
| --- | --- |
| `file` → `FileTypeIcon(fileNameOf(path))` | `file(마지막 '/' 뒤)` 의 글리프와 색. `fileNameOf` 는 `src/shared/lib/relative-path.ts:6` |
| `untitled` → `FileTypeIcon('untitled')` | `file("untitled")` → `File` + `Neutral` |
| `terminal` → `Terminal` | `Glyph::Terminal`, 제목 글자색 |
| `settings`, `appFile` → `Settings` | `Glyph::Settings`, 제목 글자색 |
| `diff` → `FileDiff` | `Glyph::FileDiff`, 제목 글자색 |
| `searchEditor` → `FileSearch2` | `Glyph::FileSearchCorner`, 제목 글자색 |
| 그 밖(`claudeDiff`, `welcome`) → `Sparkles` | `Glyph::Sparkles`, 제목 글자색 |
| `terminal` + agent → `Sparkles` + 활동 색 | 미적용(12.4 1번) |

- 배치: `src/features/tab/tab-item.tsx:41` 의 `size-3.5`(14px) 칸과 `:51` 의 `gap-1.5`(6px). `shell.rs:841-867` 이 제목 앞에 14 + 6 을 비우고, 제목 버튼의 세로 중심에 맞춘 14px rect 를 `ShellSurfaces::tab_icon` 에 넘깁니다. 제목 버튼은 `frame(false)` 라 안쪽 여백이 없어(`vendor/egui-input/src/widgets/button.rs:354-358`) 글자가 아이콘 6px 뒤에서 시작합니다.
- 색: 색 클래스가 없는 아이콘은 TS 에서 currentColor(탭 글자색)입니다. native 제목은 egui 버튼이 그리므로 같은 규칙으로 색을 구해 넘깁니다. 선택된 버튼은 `selection.stroke.color`, 아니면 `override_text_color`(`vendor/egui-input/src/widget_style.rs:133-155`). 파일 아이콘 색은 탐색기와 같은 테마 키입니다.
- 픽셀 격자: 탭 폭이 글자 폭에 따라 정수가 아니어서 두 번째 탭부터 x 좌표가 소수가 됩니다(관찰값 980.6). 글리프는 텍스처를 mesh 로 그려 egui 의 rect 반올림을 받지 않으므로 아이콘 왼쪽 위를 `round_to_pixels` 로 맞췄습니다. egui 가 글자 위치도 같은 방식으로 반올림하므로 화면의 간격은 6px 로 유지됩니다.
- 앱 배선(`application.rs:5442-5450`): 탐색기와 같은 `Icons` 캐시와 `explorer::Appearance::file_color`(`explorer.rs:150`)를 씁니다. 매핑을 `glyph-icons.rs` 에 둔 이유는 앱의 테스트 빌드가 이 파일을 `include!` 로 따로 컴파일해(8번 위험) `Glyph`·`FileColor` 가 별개 타입이 되기 때문입니다. trait 이 글리프 타입을 주고받지 않도록 `tab_icon(ui, rect, tab, title_color)` 로 정했습니다.
- 자산: lucide-react 1.28.0 의 `settings`·`file-diff`·`file-search-corner`·`sparkles` 를 `resources/icons/` 에 추가했습니다. `FileSearch2` 는 1.28.0 에서 `file-search-corner` 의 별칭입니다(`node_modules/lucide-react/dist/esm/icons/file-search-2.mjs`). 네 파일의 주 path 를 `rg -c -F` 로 원본 `.mjs` 와 대조해 양쪽 모두 1건씩 일치함을 확인했습니다. 나머지 짧은 path·circle 은 원본을 보고 옮겼습니다.
- 탭 폭: 최소 96·최대 208 은 그대로이고, 제목이 쓸 수 있는 폭이 20px 줄었습니다. 탭 폭을 단언하는 기존 테스트는 없습니다. `tests/workbench.rs` 의 기존 11건은 수정 없이 통과합니다.
- 에이전트 활동 색 범위: TS 는 `projectAgentsQueryOptions(projectId)` 의 에이전트 목록에서 세션별 활동을 찾습니다(`pane-tab-bar.tsx:85, 104, 185`). native 의 `ShellSnapshot`·`shell.rs` 에는 에이전트 목록이 없고(`rg -i agent` 0건), 감사 문서가 이를 별도 항목으로 잡고 있습니다(`2026-10-06-native-audit-terminal-agent.md:95, 179`). 데이터가 없는 상태에서 표시만 만들 수 없어 이 단계에서는 터미널 탭을 모두 `Terminal` 로 그립니다.

테스트(구현 전에 작성)

- `glyph-icons-tests.rs:233` `탭_종류는_ts의_탭_아이콘과_같은_글리프와_색으로_풀린다`: TS 의 `kind` 태그 9종을 JSON 으로 만들어 12개 사례를 확인합니다.
- `glyph-icons-tests.rs:294` `탭_전용_글리프는_파일_타입_글리프와_구분되는_서로_다른_모양으로_그려진다`: 기존 28종과 새 4종을 14px 로 래스터화해 32개가 모두 다르고 비어 있지 않음을 확인합니다. 래스터화 코드를 `rasters` 함수로 옮겨 기존 테스트와 같이 씁니다(기존 단언은 그대로).
- `tests/workbench.rs:1080` `탭은_제목_앞에_14px_아이콘_칸을_두고_6px_뒤에_제목을_그린다`: 아이콘 크기 14, 픽셀 격자 정렬, 제목과의 간격 6, 세로 중심, 탭바 안 위치, 제목 글자색과 같은 색(활성·비활성이 서로 다름), 첫 탭의 왼쪽 끝, 탭 최소 폭.
- 구현 전 실행 결과: 컴파일 오류 11건(`tab`, `Glyph::Settings` 등 없음)과 `E0407 method tab_icon is not a member of trait ShellSurfaces`(12.3 순서 3, 4).
- 새 workbench 테스트는 처음에 첫 아이콘의 왼쪽이 본문 왼쪽과 정확히 같다고 가정해 실패했습니다(849.0 대 848.5). egui 가 위젯 위치를 픽셀에 맞추며 생긴 0.5px 이고, 이 관찰로 두 번째 탭의 소수 좌표를 확인해 픽셀 격자 맞춤을 넣었습니다. 허용 오차는 반 픽셀(`PIXEL_SNAP_TOLERANCE`)입니다.

범위 밖 수정: 없습니다. `tests/workbench.rs` 는 trait 구현체라 함께 고쳐야 하는 관련 테스트입니다. `taide-remote-web` 은 `ShellSurfaces` 를 구현하지 않아 수정이 필요 없었습니다.

### 12.3 실행한 검증

명령의 공통 인자 `--locked --offline --target-dir experiments/native-shell-spike/target` 은 줄였습니다. 한 번에 하나씩 실행했습니다.

| 순서 | 명령 | exit | 결과 |
| --- | --- | --- | --- |
| 1 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib -- command_palette::tests` (재현 테스트 추가 뒤, 수정 전) | 101 | 19 passed, 1 failed. 실패는 새 포커스 테스트(`left: None`) |
| 2 | 같은 명령 + `modal::tests` (항목 1 수정 뒤) | 0 | 32 passed, 82 filtered out, 0.28s |
| 3 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --test workbench -- glyph_tests 탭은_제목` (탭 아이콘 테스트 추가 뒤, 구현 전) | 101 | lib test 컴파일 오류 11건 |
| 4 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test workbench -- 탭은_제목` (구현 전) | 101 | `E0407` 1건 |
| 5 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --test workbench` (구현 뒤) | 101 | lib 116 passed. workbench 11 passed, 1 failed(새 테스트의 첫 탭 왼쪽 가정) |
| 6 | 순서 4 명령 (관찰값을 단언 메시지에 추가) | 101 | 아이콘 `[849.0 63.0]-[863.0 77.0]`, `[980.6 63.0]-[994.6 77.0]`, 본문 왼쪽 848.5 |
| 7 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test workbench` (픽셀 격자 맞춤 뒤) | 0 | 12 passed, 0.11s |
| 8 | `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check`, `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 0 / 0 | 차이 없음 |
| 9 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib` | 0 | 116 passed, 0 failed, 0.54s (이전 111 + 팔레트 2 + modal 1 + 글리프 2) |
| 10 | `cargo test --manifest-path native/taide-native-ui/Cargo.toml --test snippet-editor --features inspection` | 0 | 18 passed, 0 failed, 0.40s |
| 11 | `cargo check --manifest-path native/taide-native-app/Cargo.toml` | 0 | 19.49s. 앱·native-ui 경고 없음(vendor `wry` 17건은 기존) |
| 12 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` | 0 | 347 passed, 0 failed, 14.32s (단계 1 리뷰 후속 기록과 같은 수) |
| 13 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer --test explorer-clipboard --test explorer-delete` | 0 | 21 / 5 / 4 passed, 0.13s / 0.06s / 3.77s |
| 14 | `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | 0 | 1.32s, 경고 없음 |

- 최종 소스 기준 근거: V2 는 순서 9·10, V3 는 순서 12, V4 는 순서 13, V5 는 순서 8·11·14 입니다. 순서 8 뒤로 소스를 바꾸지 않았습니다. `tests/workbench.rs` 는 계약 밖이지만 수정 영향이 있어 순서 7 로 확인했습니다.
- GUI 는 실행하지 않았습니다.

### 12.4 남은 위험과 실기 확인

1. 에이전트가 감지된 터미널 탭의 `Sparkles` + 활동 색(`ICON_AGENT_ACTIVITY_CLASS`)과 `agent.sessionTooltip` 은 적용하지 않았습니다. 에이전트 목록을 스냅샷에 싣는 작업(`2026-10-06-native-audit-terminal-agent.md:179`)이 선행돼야 합니다. 이 이월은 사용자 확인이 필요합니다.
2. 실기 확인 필요(GUI 미실행)
   - [ ] 탭 아이콘의 종류별 모양과 색, 1x·2x 선명도, 활성·비활성 탭에서 제목과 같은 색인지
   - [ ] 팔레트에서 `:12` + Enter, 파일 선택 + Enter 뒤 에디터에 바로 입력이 들어가는지
   - [ ] 팔레트 닫힘 전환 200ms 동안 뒤 화면 클릭이 막히는지
3. 닫힘 전환 200ms 동안 hit test 로 판정하는 클릭·hover 는 backdrop 이 막지만, `layer_id_at` 으로 판정하는 입력(예: `ScrollArea` 의 휠, `vendor/egui-input/src/containers/scroll_area.rs:1219`, `context.rs:3664-3685`)은 팔레트 본문 rect 밖에서 뒤 화면에 전달됩니다. TS 는 exit 동안 overlay 가 150ms 남아 포인터 이벤트를 받습니다(`dialog.tsx:29-40`, Radix `:153` `disableOutsidePointerEvents: context.open`). 기준 커밋의 native 는 닫는 즉시 뒤 화면이 입력을 받았습니다.
4. 스니펫 다이얼로그는 닫힘 전환 200ms 동안 여전히 modal 로 등록됩니다(`is_modal: true`, 기준 동작). Radix 기준으로는 팔레트와 같아야 하지만 리뷰 범위 밖이라 바꾸지 않았습니다. 바꿀 때는 닫힘 프레임의 뒤 화면 상태를 보는 `tests/snippet-editor.rs:1337-1352` 에 미치는 영향을 먼저 확인해야 합니다(이번에는 확인하지 않았습니다).
5. 탭의 `px-3`, `border-r`, 제목과 닫기 버튼 사이 `gap-1.5`, 닫기 버튼의 `ml-auto` 는 기존과 같이 옮겨지지 않은 상태입니다(`2026-10-06-native-audit-shell.md` 73~87 항목). 아이콘은 탭 왼쪽 끝에서 시작합니다.
6. 5번 위험(`FocusReturn::settle` 이 배경 위젯보다 뒤에 호출돼야 함)은 그대로입니다.
