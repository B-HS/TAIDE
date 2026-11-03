# 11. 남은 작업 — editor·preview 영역

> 작성일: 2026-11-04 · 기준: 메인 작업 트리 `/Users/hyunseokbyun/development/TAIDE`(브랜치 `to_rust_native`, HEAD `0c52b438`), Git 영역은 병합 중인 worktree `/Users/hyunseokbyun/development/TAIDE-lanes/git`(브랜치 `lane/git`, HEAD `50892b59` 위에 메인을 병합하는 중. `git status`에서 `UU`인 파일은 `docs/quality-assurance/2026-10-09-native-function-matrix.md`, `native/taide-native-ui/src/command-registry.rs`, `native/taide-native-app/src/application-explorer-views-tests.rs`이며 세 파일 모두 충돌 표식(`<<<<<<<`)은 이미 없습니다).
> 독자: 이 저장소를 처음 보는 구현 작업자(사람 또는 AI). egui 0.36·Rust 2024 edition·이 저장소의 관례를 모른다고 가정했습니다.
> 모든 사실은 2026-11-04에 실제 파일을 열어 확인했고 "경로:줄" 또는 "경로 + 심볼"로 적었습니다. 줄 번호는 git lane 병합 뒤 크게 움직입니다(git lane은 `UI/editor_surface.rs`에 170줄, `APP/domains/editor.rs`에 git 연결을 더함). 줄 번호가 맞지 않으면 심볼 이름으로 다시 찾으십시오.
> 이 문서와 실제 코드가 다르면 실제 코드가 우선입니다. 사양 문서(`SPEC`)와 이 문서가 다르면 이 문서가 원본 파일을 다시 읽어 정정한 것입니다(정정 목록은 4절 1번).

---

## 0. 읽는 법

### 0.1 경로 약어

| 약어 | 절대 경로 |
| --- | --- |
| `TS` | `/Users/hyunseokbyun/development/TAIDE/src` (원본 Tauri+React 앱) |
| `TAURI` | `/Users/hyunseokbyun/development/TAIDE/src-tauri/src` |
| `CR` | `/Users/hyunseokbyun/development/TAIDE/crates` (Tauri·native 공용 코어, 루트 workspace 멤버) |
| `APP` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src` |
| `APPT` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/tests` |
| `UI` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src` |
| `UIT` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/tests` |
| `ED` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-editor/src` (egui 비의존 portable core, wasm32 컴파일 대상) |
| `EDT` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-editor/tests` |
| `SYN` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-syntax/src` (정규식·TextMate·Monaco 언어 구성) |
| `WEB` | `/Users/hyunseokbyun/development/TAIDE/native/taide-remote-web` (동결, 컴파일만 유지) |
| `MON` | `/Users/hyunseokbyun/development/TAIDE/node_modules/monaco-editor/esm/vs` (원본이 쓰는 Monaco 0.56) |
| `MEXT` | `/Users/hyunseokbyun/development/TAIDE/node_modules/monaco-editor/esm/external` (Monaco가 번들한 vscode-json/css/html-languageservice) |
| `LOC` | `/Users/hyunseokbyun/development/TAIDE/crates/taide-locale/resources/locales/{en,ko,ja}.json` |
| `MOCK` | `/Users/hyunseokbyun/development/TAIDE/experiments/lsp-coordinator-spike/src/bin/mock-server.rs` (앱 example 이름 `native-lsp-mock`) |
| `GIT` | `/Users/hyunseokbyun/development/TAIDE-lanes/git` (git lane worktree, 병합 중) |
| `QA` | `/Users/hyunseokbyun/development/TAIDE/docs/quality-assurance` |
| `MATRIX` | `QA/2026-10-09-native-function-matrix.md` (기능표) |
| `SPEC` | `/Users/hyunseokbyun/development/TAIDE/docs/research/2026-10-28-lane-editor-preview-spec.md` (editor·preview lane 사양) |
| `DEC` | `/Users/hyunseokbyun/development/TAIDE/docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` (lane 사양 결정) |
| `HO` | `/Users/hyunseokbyun/development/TAIDE/docs/handoff/native-completion` (이 문서 묶음) |

### 0.2 검사 표기

- `[C]` crate 단위 검사: `EDT/*.rs`(일반 `#[test]`), `SYN` 내부 `#[cfg(test)]`, 앱 crate의 순수 단위 검사(`APP/*-tests.rs` 중 `NativeApplication`을 띄우지 않는 것, 예: `APP/preview_web_cache.rs` 끝의 `mod tests`).
- `[U]` UI 하네스: `UIT/*.rs`. egui `Context`를 직접 만들어 `NativeEditor`·위젯을 한 프레임씩 돌리고 `FullOutput.shapes`·accesskit 노드로 단언합니다. `--features native-host,inspection`이 필요합니다.
- `[A]` 실제 App: `APP/application-*-tests.rs`. `NativeApplication`을 egui kittest 프레임(`eframe::Frame::_new_kittest()`)으로 돌리는 앱 lib 검사입니다(`--lib`). 진짜 창·GUI를 띄우지 않습니다.
- `[L]` 실제 LSP child: `MOCK`에 `--native-<기능>` 모드를 추가하고, 임시 bin 디렉터리에 서버 이름의 셸 스크립트(`exec '<mock 경로>' <모드>`)를 써서 `LspBridge::connect`로 실제 자식 프로세스를 띄웁니다.

### 0.3 실행 명령

메인 작업 트리에서는 병합 게이트와 같은 target을 씁니다. lane worktree에서 작업하면 `<ROOT>`를 그 worktree로, `<T>`를 그 lane의 target(`/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`)으로 바꿉니다.

- `<ROOT>` = `/Users/hyunseokbyun/development/TAIDE`
- `<T>` = `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`

1. LSP mock 준비(앱 lib의 LSP 검사 전 1회, target을 비웠으면 다시):
   `cargo build --quiet --manifest-path <ROOT>/native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir <T>`
2. 편집기 core: `cargo test --manifest-path <ROOT>/native/taide-native-editor/Cargo.toml --locked --offline --target-dir <T> --test <이름>`
3. 편집기 core wasm: `cargo check --quiet --manifest-path <ROOT>/native/taide-native-editor/Cargo.toml --lib --target wasm32-unknown-unknown --locked --offline --target-dir <T>`
4. 구문: `cargo test --manifest-path <ROOT>/native/taide-native-syntax/Cargo.toml --locked --offline --target-dir <T> --lib -- <필터>`
5. UI: `cargo test --manifest-path <ROOT>/native/taide-native-ui/Cargo.toml --features native-host,inspection --locked --offline --target-dir <T> --test <이름>`
6. 앱 lib: `cargo test --manifest-path <ROOT>/native/taide-native-app/Cargo.toml --locked --offline --target-dir <T> --lib -- <필터> --test-threads=2`
7. 앱 통합(필요한 대상만): `cargo test --manifest-path <ROOT>/native/taide-native-app/Cargo.toml --locked --offline --target-dir <T> --test <이름>`
8. 동결 미러(ED·UI·SYN의 공개 타입을 바꾼 묶음마다 1회, 두 명령 모두):
   `cargo check --quiet --manifest-path <ROOT>/native/taide-remote-web/Cargo.toml --tests --features inspection --locked --offline --target-dir <T>`
   `cargo check --quiet --manifest-path <ROOT>/native/taide-remote-web/Cargo.toml --lib --features canvas --target wasm32-unknown-unknown --locked --offline --target-dir <T>`
9. 공용 crate(`CR/taide-lsp` 등)를 바꾼 경우(editor-7이 해당): `HO/05-process-and-gates.md` 9.2절의 루트 workspace 명령 두 개(`cargo test --manifest-path <ROOT>/Cargo.toml -p <crate> --locked --offline --no-fail-fast`, `cargo check --manifest-path <ROOT>/Cargo.toml --workspace --locked --offline`).
10. 형식: 바꾼 crate마다 `cargo fmt --manifest-path <crate>/Cargo.toml -- --check`

모든 검사에서 금지: GUI 실행, 실제 clipboard·Keychain·OS 알림·Finder/외부 앱 열기, 실제 AI provider, 네트워크, 실제 사용자 데이터 경로, 휴지통을 쓰는 검사(`APPT/explorer-delete.rs`와 `HO/05-process-and-gates.md` 9.1절의 lib 검사 3개), 저장소 밖 탐색, git 상태 변경, 허락 없는 새 의존성·vendor 패치·`Cargo.toml`/`Cargo.lock` 변경, 동결 `WEB` 소스·Cargo 파일 변경. 근거: `SPEC` 40줄, `/Users/hyunseokbyun/development/TAIDE/docs/memory/2026-10-28-native-lane-operations.md` 60줄, `DEC` "보류" 표, `/Users/hyunseokbyun/development/TAIDE/docs/utils/2026-11-02-native-lane-workflow.js` 45~50줄.

병합 게이트 스크립트(`/Users/hyunseokbyun/development/TAIDE/docs/utils/2026-11-03-native-merge-gate.sh` 9줄)는 앱 crate를 `--skip` 없이 실행해 휴지통 검사가 포함됩니다. 이 영역 작업자는 그 스크립트를 직접 실행하지 말고 위 1~10번의 대상 지정 명령만 씁니다.

---

## 1. 요약

### 1.1 남은 행과 상태

대상은 `MATRIX`의 editor·preview 행 중 상태가 complete·not-applicable·frozen이 아닌 행입니다. 행 목록은 다음 명령으로 직접 만들었습니다. 기능표 머리말의 영역 합계 표(9~22줄)는 갱신이 밀려 실제 행 값과 다르므로(예: 합계 표는 editor partial 10·missing 7, 실제 행은 partial 7·missing 5) 쓰지 않았습니다.

```
grep -n -E '^\| (editor|preview)-' /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-10-09-native-function-matrix.md
```

결과를 `|`로 나눈 5번째 칸(상태)이 complete·not-applicable·frozen이 아닌 행이 아래 15행입니다. editor-73~76은 Git 영역이므로 `GIT/docs/quality-assurance/2026-10-09-native-function-matrix.md` 272~275줄도 함께 읽었습니다(git lane 기능표가 최신).

| ID | 기능 | 메인 기능표 상태(줄) | git lane 기능표 상태 | 남은 것 한 줄 | 막는 요소 |
| --- | --- | --- | --- | --- | --- |
| editor-7 | semantic token 강조 | missing(206) | missing | 전체(SDK 옵션 노출·요청·캐시·delta·refresh·설정·테마·표시) | 없음 |
| editor-15 | minimap과 토글 액션 | partial(214) | partial | section header(MARK·region), 접힌 영역 배경, 선택 텍스트 발생 강조의 미니맵 표시 | 선택 발생 표시는 3.16 C(본문 기능)가 먼저 |
| editor-16 | sticky scroll | partial(215) | partial | CSS 문서 심볼(파서 필요), TS/JS 내장 심볼 | editor-57 C1·C2, E6(사용자 허락) |
| editor-32 | formatOnType·formatOnPaste | partial(231) | partial | TS/JS 내장 붙여넣기·onType 포맷 | E6(사용자 허락) |
| editor-34 | Monaco 액션 팔레트·키바인딩 | partial(233) | partial | fontZoom·gotoLine·quickOutline·hideHover·hover verbosity·inPlaceReplace·서식 복사·pasteAs·tabFocusMode·openLink·unicodeHighlight 4개·toggleAlwaysShowToolbar 팔레트 실행·색 선택기 | openLink·unicodeHighlight는 3.16 A·B, 색 선택기는 3.16 D |
| editor-49 | 포맷 | partial(248) | partial | TS/JS 내장 포맷, `<style>` 안 CSS 120열 감싸기 | E6(사용자 허락) |
| editor-57 | Monaco 내장 언어 서비스 | partial(256) | partial | JSON hover 앱 소비·JSON 색, CSS 파서 이후 서비스 전부, HTML 완성·hover·하이라이트·링크·rename, TS/JS 전체 | E6 사용자 허락, 웹 자료 표 생성에 bun 실행 허락 |
| editor-72 | Markdown 미리보기 분할 | partial(271) | partial | 결정 7 A 목록 9개 + `kbd` 전용 `[A]` | 없음 |
| editor-73 | Git gutter·hunk 되돌리기·선택 stage | missing(272) | **complete** | 없음(git lane 병합으로 닫힘) | git lane 병합 |
| editor-74 | merge conflict 표시·해결 dialog | missing(273) | **complete** | 없음(git lane 병합으로 닫힘) | git lane 병합 |
| editor-75 | blame | missing(274) | **complete** | 없음(git lane 병합으로 닫힘) | git lane 병합 |
| editor-76 | diff 편집기 | missing(275) | partial | 문자 단위 강조, ±표시, overview ruler, inline 삭제 줄 구문 색·두 줄 번호 열, ClaudeDiff 거터 "Revert Block" | 소유 git lane(`HO/14-remaining-git.md` 3.3·3.5), git lane 병합 선행 |
| preview-5 | 외부 저장·이동·삭제 시 미리보기 갱신 | partial(435) | partial | HTML·PDF 외부 수정 `[A]`, 앱 안 rename 뒤 새 경로·override `[A]`, 앱 안 삭제 | 앱 안 삭제는 휴지통이라 허락 필요 |
| preview-14 | 비디오 재생 | partial(444) | partial | 실제 WKWebView에서 restyle·재생·seek·컨트롤·키보드/AX, `evaluate_script` 성공 판정 창 | GUI 실기 허락 |
| preview-15 | 오디오 재생 | partial(445) | partial | preview-14와 같음 | GUI 실기 허락 |

상태별 수:

- 메인 기능표 기준(지정된 기능표 경로): **15행 = missing 5(editor-7·73·74·75·76) + partial 10(editor-15·16·32·34·49·57·72, preview-5·14·15)**.
- git lane 병합 뒤 기준: **12행 = missing 1(editor-7) + partial 11**, 그리고 editor-73·74·75 3행은 complete가 됩니다.

이 영역에는 기능표에 행이 없지만 원본 Monaco 기본값으로 켜져 있는 기능이 있습니다. `DEC` "채택" 7줄(4.1절 결정은 추천안)과 `SPEC` 4.1 결정 2 A에 따라 "새 행(editor-88~)으로 추가하고 구현"합니다. 3.16절에서 같은 형식으로 다룹니다: A. Unicode 강조, B. 텍스트 링크 감지, C. 선택 텍스트 발생 강조, D. 색 장식·색 선택기. 이번 확인에서 사양 1.4 목록 밖의 기본 기능 3개(E. 선택 텍스트 끌어 옮기기, 읽기 전용 편집 시도 안내, 비정상 줄 끝 문자 확인 대화상자)를 더 찾았습니다. 이 셋은 결정 2의 범위가 아니므로 메인 확인 대상입니다(3.16 E).

### 1.2 현재 구현 위치

| 기능 | crate·파일 | 핵심 심볼(줄은 메인 HEAD 기준) |
| --- | --- | --- |
| 편집기 영역 진입점 | `APP/domains/editor.rs`(6,098줄) | `pub enum Command`(30), `pub enum Reply`(141), `dispatch`(225), `impl NativeApplication`(737: `apply_editor_reply` 738, `apply_editor_intent` 1397), 두 번째 `impl NativeApplication`(1845: `request_editor_formatting` 2170, `request_editor_input_formatting` 2244, `reconcile_lsp` 2349, `reconcile_builtin_json` 2523, `dispatch_completion` 2676, `poll_lsp` 2990, `reconcile_editor_features` 5077), `impl EditorSurfaces for AppSurfaces`(5313: `tab_content` 5353), `AppSurfaces::show_document`(5485) |
| 편집기 상태 | `APP/application.rs`의 `NativeApplication` 루트 필드(`editor_json` 162, `editor_highlights` 169, `editor_inlay_hints` 170, `editor_formatting` 192 등) | `APP/domains/editor.rs`에 `struct State`가 아직 없습니다(`grep -n "struct State" APP/domains/editor.rs` 결과 없음). seams 문서 0절 "editor의 상태·tick 이동(5.9절 2~4단계)만 남았다" |
| 편집기 표면 | `UI/editor_surface.rs`(4,517줄) | `EditorDisplayOptions`(108), `EditorPresentation`(155), `EditorTokens`(160), `EditorRequest`(181), `NativeEditor`(913), `show_request`(1280)…`show_request_with_code_lenses`(1428) |
| 행 배치·글자 | `UI/editor-geometry.rs`, `UI/editor-row-text.rs` | `RowLayout`(geometry 154), `RowLayout::row`(178: 본문 행의 `text.highlight` 192줄), `RowText`(row-text 60), `highlight`(384), `decorate`(442), `split_section`(464), `styled_layout_job`(483) |
| 행 그리기 | `UI/editor-paint.rs` | `row_background`(105: `heads_hidden_lines`면 `folded_background`), `folded_background`(181, `FOLD_BACKGROUND_OPACITY` 0.3) |
| 미니맵 | `UI/editor-minimap.rs` | `EditorMinimapColors`(33), `measure`(42, Normal 등급에서만), `ImageKey`(78), `MinimapFrame`(294, `marks` 306), `paint`(311), `render_image`(456), `paint_background`(574) |
| overview·미니맵 표시 | `UI/editor-overview.rs` | `ScrollMark`(32), `marks`(39: `DecorationKind::Overview`·진단·괄호에서 만듦) |
| 명령 표 | `UI/command-registry.rs`(2,118줄) | `EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE`(25), `DocumentEdit`(88), `PaletteEntry`(145), `Run`(152), `Execution`(214), `enablement`(331), `execution`(360), `keymap_run`(601) |
| Run → intent | `APP/shell_keymap.rs` | `intent`(89), `APP/command-dispatch.rs` 85줄이 팔레트 실행에서 호출 |
| 내장 언어 서비스 구조 | `APP/editor-json.rs`(282줄) | `WORKER_NAME`(18) `"native-json-structure"`, `MAX_ACTIVE`(19) 2, `Models`(51), `provides_symbols`(56), `symbol_documents`(96), `State::tick`(104), `build`(196) |
| 내장 포맷 | `APP/editor-formatting.rs` | `is_native_eligible`(25), `native_build`(31), `Kind`(47), `begin_native_save`(186), `tick_native_saves`(214), `tick_native`(294), 내장 onType 거절(472줄 `if native && matches!(kind, Kind::OnType { .. })`) |
| JSON 진단 | `APP/editor-json-validation.rs` | `State::tick`(190), 소비 `APP/domains/editor.rs` 2605~2646줄(`retain_builtin` 2611, `publish_builtin`) |
| JSON 완성 | `APP/editor-completion-json.rs` | `WORKER_NAME` `"native-json-completion"`(21), `ORDINAL`(26), `provider`(40), `applies`(49), `spawn`(55) |
| CSS·HTML 앱 공급 | `APP/editor-css.rs`(170줄), `APP/editor-html.rs`(125줄) | `dialect`, `folds`, `is_format_eligible`, `format`, `is_html`, `models`, `selection_ranges` |
| hover 공급 | `APP/editor-documentation.rs`(883줄) | `HoverGroup`(88), `State`(214), `Provider::available`(635), `version`(651), `request`(711), `open_link`(848) |
| 하이라이트 | `APP/editor-highlights.rs`(969줄) | `MAX_HIGHLIGHT_UTF16_LENGTH`(39)·`MAX_HIGHLIGHT_LINES`(40), `Colors`(160~201, `minimap` 200), `is_highlight_model`(284), `textual`(289), `has_word_selection`(369), `Display`(408)·`paint`(414), `Consumer::submit`(821) |
| 내장 서비스 core | `ED/json-*.rs`, `ED/css-scanner.rs`·`css-folding.rs`·`css-format.rs`, `ED/html-*.rs`, `ED/color.rs`, `ED/default-colors.rs`(`document_colors` 271) | 각 파일의 `pub fn` |
| Markdown 미리보기 | `APP/editor-markdown.rs`(`PreviewProvider::open_link` 425), `APP/editor-markdown-html.rs`(`BLOCK_TAGS` 12, `REMOVED_TAGS` 38, `inlines` 84~182, `blocks` 279), `UI/editor-markup*.rs` | `ED/documentation.rs` `Block`(47), `Inline`(201), `Span`(220) |
| 미리보기 영역 | `APP/domains/preview.rs`(928줄) | `State`(59), `apply_preview_intent`(376), `reconcile_previews`(433), `tick_preview_media`(590), `show_preview_web_views`(626), `load_preview_requests`(678), `show_preview_tab`(754) |
| 미디어 WebView | `APP/preview_web_media.rs`, `APP/preview_web_cache.rs`, `APP/preview_web_view.rs` | `VIDEO_TEMPLATE`(20)·`AUDIO_TEMPLATE`(21)·`STYLES`(22), `style`(45), `restyle_script`(54), `Cache::restyle_media`(94), `release_restyled`(117), `source`(254), `Views::open_tabs`·`evaluate`(266~300) |
| diff 편집기(git lane) | `GIT/native/taide-native-ui/src/diff-view.rs`(1,028줄), `GIT/native/taide-native-app/src/domains/git.rs`(3,027줄), `git-diff.rs`, `git-claude-diff.rs`, `git-editor.rs`(`editor_layers` 58, `show_git_editor` 472) | `HO/14-remaining-git.md` 3.3·3.5 |
| LSP 다리 | `APP/lsp.rs`(3,203줄) | `Reply`(333: `CodeLenses` 338, `CodeLensRefresh` 350), `code_lens_providers`(929), `code_lenses`(937), `ready_feature_providers`(976), `initialize`(1281), `plans`(1310: `DocumentKey::File`만), `notice`(1680: `Refresh(RefreshKind::CodeLens)`만 1697줄, 나머지 1765줄 `Rejected(Unavailable)`) |
| LSP SDK | `CR/taide-lsp/src/native/feature.rs`, `session.rs`, `capabilities.rs`, `CR/taide-lsp/src/native.rs` | `SemanticTokensDeltaResult`(feature 88), typed 요청 표(203~204), `validate_projection`(307), `SessionSnapshot`(session 45), `publish`(session 599), `completion_options`(native.rs 224), `document_options`(native.rs 253) |

### 1.3 다른 영역과의 의존·소유 경계

| 필요한 것 | 소유 | 영향 행 | 처리 |
| --- | --- | --- | --- |
| `UI/diff-view.rs`, `domains/git.rs`, `git-claude-diff.rs`의 diff 표면 | git lane | editor-76 | git lane 병합 뒤 그 파일을 고칩니다. 구현 절차의 정본은 `HO/14-remaining-git.md` 3.3(git-40)·3.5(git-50). 이 영역 작업자는 행 판정만 함께 갱신합니다 |
| `DocumentKey::Virtual`(diff 가상 문서) | git lane(`GIT/native/taide-native-editor/src/document.rs` 17줄, `store.rs` 649·682줄) | 새 내장 서비스 전부, editor-7 | 병합 뒤 새 문서 선별 조건은 `DocumentKey::AppFile(_) \| DocumentKey::Virtual(_)`를 함께 제외합니다(4절 함정 4) |
| `show_document`의 git 연결(`self.git.editor_layers` 5878줄, `show_git_editor` 6103줄, git lane 기준) | git lane | editor-7·15·3.16 | 병합 뒤 `show_document`를 고칠 때 git 장식 레이어·영역 호출을 지우거나 순서를 바꾸지 않습니다 |
| `monaco.taide.gitStageSelection`·`toggleBlame`·`openFileHistory` 실행 | git lane(`GIT/native/taide-native-ui/src/command-registry.rs` 476~480줄) | editor-34 | 이미 연결되어 있습니다. 병합 뒤 editor-34 비고에서 git 3개를 빼면 됩니다 |
| `preview_web_view.rs` 구조(`Views.children` 64줄, `sync` 81줄) | shell lane(shell-5 보조 창 WKWebView) | preview-14/15 | 이 영역은 파일 끝 `impl Views` 블록에 메서드만 추가합니다(`SPEC` 2.0.1 마지막 문단) |
| 팔레트 열기 intent(`intents::commands::Intent::OpenPalette`) | commands-ui | editor-34 gotoLine·quickOutline | `PaletteEntry` 변형 추가만 하고 팔레트 화면은 바꾸지 않습니다 |
| `CR/taide-lsp`(Tauri 앱과 공유) | 공용 | editor-7 | `SessionSnapshot`에 필드를 추가하면 루트 workspace 검사(0.3의 9번)를 합니다 |
| 기능표 머리말 합계 표·5줄 범례 | 메인 | 모든 행 | 해당 행만 고치고 합계 표는 메인이 병합 때 다시 셉니다 |
| settings-28 비고(리거처·semantic 소비자) | settings | editor-7 | editor-7을 닫으면 settings-28 비고의 semantic 문장을 같은 근거로 바꾸도록 메인에 알립니다 |
| 외부 URL 열기 | shell/editor 공용 | 3.16 B | 새 Host 명령을 만들지 않고 기존 `APP/editor-documentation.rs` `open_link`(848줄) 경로를 재사용합니다 |
| TS/JS 내장 서비스 방식(결정 4) | 사용자 | editor-16·32·49·57 | 허락 전에는 E6을 구현하지 않습니다(3.7 E6 비교) |

### 1.4 권장 구현 순서

1. **git lane 병합 확인**: `git -C /Users/hyunseokbyun/development/TAIDE log --oneline -5`에 git lane 병합 커밋이 있는지 봅니다. 있으면 `MATRIX`의 editor-73~75가 complete인지, editor-76이 partial인지 확인하고, 내장 서비스 파일(`APP/editor-json.rs`·`editor-json-validation.rs`·`editor-css.rs`·`editor-html.rs`·`editor-completion-json.rs`·`editor-selection-ranges.rs`)의 문서 선별 조건에 `DocumentKey::Virtual(_)`가 있는지 `grep -n "Virtual" <파일>`로 확인합니다(GIT 작업 트리에는 모두 들어가 있습니다).
2. **editor-7 semantic token**: 다른 영역 선행이 없습니다. SDK(공용 crate)·ED·SYN·UI·APP을 모두 거치는 가장 큰 단일 묶음입니다.
3. **3.16 C 선택 텍스트 발생 강조 → editor-15**: 미니맵의 선택 발생 표시가 본문 기능을 전제로 합니다. section header·접힌 영역 배경은 독립이므로 C와 병렬 가능합니다.
4. **3.16 A Unicode 강조, 3.16 B 링크 감지**: editor-34의 `unicodeHighlight.*`·`openLink` 실행 대상이 생깁니다.
5. **editor-34**: 남은 액션 표(3.5).
6. **editor-57 J1(JSON hover 앱 소비) → C1(CSS 파서) → C2(심볼, editor-16의 CSS 부분 닫힘) → C3(선택 확장) → C4(탐색) → C5(진단) → C6(색) → 3.16 D(색 UI, J2 JSON 색 포함) → C7(완성·hover) → C8(`<style>` 120열, editor-49의 CSS 차이 닫힘) → H1~H4(HTML)**: 같은 공급 경로를 공유하므로 직렬입니다. C7·H4는 bun 허락이 필요합니다.
7. **editor-72**: preview 계열·Markdown 파일만 고치므로 6과 병렬 가능합니다.
8. **preview-5 잔여 `[A]`**.
9. **editor-76**: git lane 소유. git lane 작업자가 맡지 않으면 메인과 소유를 정한 뒤 이 영역 작업자가 병합 뒤 진행합니다.
10. **E6(TS/JS)**: 사용자 결정 4를 받은 뒤. 그 결과로 editor-16·32·49·57을 닫습니다.
11. **preview-14/15**: GUI 실기 허락 뒤.

---

## 2. 영역 공통 지식

### 2.1 crate 구조와 의존 규칙

- native crate는 루트 workspace(`<ROOT>/Cargo.toml` 3줄 `members`) 밖의 개별 workspace이며 각자 `Cargo.lock`이 있습니다. `CR/*`는 루트 workspace 멤버이고 Tauri 앱(`TAURI`)과 native가 함께 씁니다(`<ROOT>/src-tauri/Cargo.toml` 68줄 `taide-lsp = { path = "../crates/taide-lsp" }`).
- 의존 방향: `APP` → `UI` → `ED`, `APP` → `SYN` → `ED`. `UI`는 `SYN`에 의존하지 않습니다(`<ROOT>/native/taide-native-ui/Cargo.toml` `[dependencies]`에 `taide-native-syntax` 없음). `ED`의 의존은 `lsp-types =0.97.0`, `unicode-segmentation =1.13.3`, `ropey =1.6.1`, `taide-model`뿐입니다(`<ROOT>/native/taide-native-editor/Cargo.toml`). 정규식 crate(`regress =0.12.0`, `ferroni =1.8.1`)는 `SYN`에만 있습니다. 그래서 원본의 정규식은 (a) `ED`에서 손으로 상태 기계로 구현하거나(선례 `ED/css-folding.rs`의 `#region` 처리, `ED/default-colors.rs`의 색 정규식), (b) `ED`에 trait만 두고 `SYN`이 구현합니다(선례 `ED/find.rs`의 `FindPattern`·`FindPatternCompiler`와 `SYN`의 `MonacoFindPatternCompiler`, `ED/language-configuration.rs` 118줄 `LanguageRules`와 `SYN/language-configuration.rs`의 `MonacoLanguage` 구현).
- `UI`의 egui는 vendored `APP/../vendor/egui-input`으로 patch됩니다(`UI` Cargo.toml `[patch.crates-io] egui = { path = "../taide-native-app/vendor/egui-input" }`). egui 0.36에서 `Context::open_url`은 `OutputCommand::OpenUrl`을 `FullOutput.platform_output.commands`에 넣습니다(`APP/../vendor/egui-input/src/context.rs` 1903~1916줄).
- `EditorDisplayOptions`(`#[derive(Debug, Clone, Default, PartialEq)]`)의 literal은 모두 `..Default::default()`를 씁니다(`UI/presentation.rs` 263·298줄, `UIT/editor-minimap.rs` 114줄, `UIT/editor-sticky-scroll.rs` 134줄, `UIT/editor-display.rs` 104줄, `UIT/editor_surface.rs` 291·2749·3548·5293·5821줄의 10곳). 필드 추가가 literal을 깨지 않습니다. 반대로 `EditorTokens`(`Copy`, 빌림 3필드)는 literal이 5개 파일 12곳(`UIT/editor-sticky-scroll.rs`, `editor-display.rs`, `editor-minimap.rs`, `editor_surface.rs`, `APP/editor-syntax.rs`)이라 필드 추가를 피합니다. `EditorMinimapColors`는 literal이 2곳(`UI/presentation.rs` 381줄, `UIT/editor-minimap.rs` 117줄)입니다. `EditorAppearance`·`EditorPresentation`은 동결 crate 검사까지 쓰여 필드 추가를 피해 왔습니다(`/Users/hyunseokbyun/development/TAIDE/docs/acknowledge/2026-10-06-native-transition-decisions.md` "기각된 대안" 표).
- `WEB`은 `NativeEditor`, `EditorOutput`, `EditorAppearance`만 import합니다(`grep -rn "editor_surface::" WEB/src`: `files.rs` 17줄, `presentation.rs` 10줄, `browser-editor.rs` 14줄). `ShellIntent`·`ShellSurfaces`·`documentation::Block`은 쓰지 않습니다. 그래도 ED·UI 공개 타입을 바꾼 묶음은 0.3의 8번 두 명령으로 확인합니다.

### 2.2 앱 crate의 영역 구조와 흐름

영역 구조 규칙의 단일 출처는 `/Users/hyunseokbyun/development/TAIDE/docs/research/2026-10-28-native-lane-seams.md` 1절입니다. 요점과 이 영역에 해당하는 실제 위치를 옮깁니다.

1. `domains`는 `APP/application.rs` 안의 `#[path = "domains/mod.rs"] pub mod domains;`로 선언된 `application`의 자식 모듈입니다. 영역 파일은 `NativeApplication`의 비공개 필드·메서드와 `application.rs`의 비공개 항목(`AppSurfaces`, `WINDOW_LABEL`, `editor_error` 등)에 가시성 변경 없이 접근합니다(seams 1.1).
2. **호스트 요청**: 영역 코드가 `HostCommand::Editor(domains::editor::Command::X)`(또는 `HostCommand::Preview(domains::preview::Command::X)`)를 `self.commands`(표면 안에서는 `AppSurfaces.commands`)에 넣으면 프레임 끝에 `HostBridge`(worker 1개, 큐 용량 `HOST_COMMAND_CAPACITY` 64, `APP/host.rs` 10줄)로 제출됩니다. `APP/host.rs` `dispatch`의 맨 위 라우팅(333줄 `HostCommand::Editor(command) => …`)이 `domains::editor::dispatch(worker, command)`(225줄)로 보냅니다. 응답 `HostReply::Editor(Reply::Y)`는 다음 프레임 `poll()`에서 `apply_editor_reply`(738줄)로 옵니다(seams 1.2).
3. **화면 의도(intent)**: UI가 `ShellIntent::Editor(taide_native_ui::intents::editor::Intent::Z)`를 내면 `application.rs` 화면 pass의 intent 루프가 `apply_editor_intent(intent, &context, frame, &snapshot, &command_context, &keymap_documents)`(1397줄)로 보냅니다. 미리보기는 `intents::preview::Intent::ReopenWith` 하나이며 `apply_preview_intent`(`APP/domains/preview.rs` 376줄)가 받습니다(seams 1.3).
4. **편집기 명령(팔레트·키)**: 명령 id → `UI/command-registry.rs` `execution`(360줄)이 `Execution::Native(Run::…)` 또는 `Execution::Unavailable`. 팔레트 실행은 `APP/command-dispatch.rs` 85줄 `crate::shell_keymap::intent(command_run(id, context)?, snapshot, scope)` → `APP/shell_keymap.rs` `intent`(89줄)가 `Run`을 `ShellIntent`로 바꿉니다. 편집기 문서 대상 명령(`Run::EditDocument(DocumentEdit::…)`)은 탭 id와 함께 `self.document_edits`에 쌓였다가 `show_document`가 그 탭을 그릴 때 꺼냅니다(예: `APP/domains/editor.rs` 5622줄 `DocumentEdit::Problem`, 5646줄 `Highlight`, 5698줄 `Documentation`).
5. **표면**: 탭 본문은 `UI/shell.rs`가 `ShellSurfaces`(supertrait `EditorSurfaces + PreviewSurfaces + …`)의 `tab_content`를 불러 그립니다. 앱 구현은 `APP/domains/editor.rs` 5353줄 `fn tab_content` → 파일·untitled 탭이면 `AppSurfaces::show_document`(5485줄), 미리보기 탭이면 `AppSurfaces::show_preview_tab`(`APP/domains/preview.rs` 754줄). `AppSurfaces`는 `APP/application.rs` 2524줄 struct이고, 화면 pass 생성 위치가 1883~1884줄(`editor_highlights: &mut self.editor_highlights` 등)입니다.
6. **프레임 순서**: `APP/application.rs` `background_tick`(1071줄)에서 응답 수신 → … → `reconcile_builtin_json`(1147줄: 내장 JSON·CSS·HTML 구조와 JSON 진단) → `dispatch_completion`(1154줄) → `poll_lsp`(1161줄). 그 뒤 `frame_ui`(1383줄)가 `reconcile_editor_features`(1409줄)를 부르고 `WindowPass`를 만든 다음 창별 `show_window`(1686줄)에서 탭 본문을 그립니다.
7. **새 Host 명령은 최소화**합니다. LSP 기능은 `LspBridge`(`APP/lsp.rs`) 내부 worker 명령, 내장 서비스는 감독 worker(`self.services.tasks`의 `spawn_blocking_transient_handle`)로 처리합니다(`SPEC` 2.0.2).
8. **공용 파일**(`application.rs`, `lib.rs`, `host.rs`, UI `commands.rs`·`shell.rs`, `LOC`)은 자기 영역 블록 끝에 추가만 합니다(seams 1.4 표). 새 앱 상태는 원칙상 `domains/editor.rs`의 `State`에 둡니다. 지금은 `State`가 없으므로 처음 필요한 작업자가 다음을 함께 합니다.
   1. `APP/domains/editor.rs`의 `Reply` 정의 뒤(seams 1.1 항목 순서 3번 위치)에 `#[derive(Default)] pub(crate) struct State { … }`를 정의합니다.
   2. `APP/application.rs`의 `NativeApplication` struct에 `editor_state: domains::editor::State,` 한 줄(영역 상태 필드 블록, `preview_state` 151줄 근처), 생성자 `new_with_secret_store`(290줄)의 리터럴(552줄 `preview_state:` 근처)에 `editor_state: domains::editor::State::default(),` 한 줄을 추가합니다.
   3. 탭 본문 그리기에서 그 상태가 필요하면 `AppSurfaces`(2524줄)에 `editor_state: &'a mut domains::editor::State,` 필드를, 생성 위치(1883줄 근처)에 `editor_state: &mut self.editor_state,`를 추가합니다.

### 2.3 대표 흐름

**(a) LSP 기능 요청(코드 렌즈가 가장 가까운 선례).**

1. 공급자 조회: `LspBridge::code_lens_providers(project, snapshot)`(`APP/lsp.rs` 929줄) → `ready_feature_providers(project, snapshot, "textDocument/codeLens")`(976줄). Normal 등급이 아니면 빈 집합(982줄), 세션의 `document_versions`가 문서의 key·revision·언어·등급과 같아야 포함됩니다. LSP 세션은 `DocumentKey::File`만 만듭니다(`plans` 1310줄 `let DocumentKey::File(path) = &snapshot.key else { return Ok(Vec::new()); }`). Untitled·AppFile 문서는 LSP 기능이 없습니다.
2. 요청 제출: `LspBridge::code_lenses(request)`(937줄)가 `Command::CodeLensWork(Work::Provide(request))`를 LSP worker에 넣습니다.
3. worker 처리: `APP/lsp-editor-code-lenses.rs` `run`(197줄) → `provide`(83줄). 같은 프로젝트의 세션 중 문서가 있는 것을 모아 `session.order` 내림차순으로 정렬합니다(99줄 `candidates.sort_by_key(|(session, _)| std::cmp::Reverse(session.order))`). 문서 revision·언어가 요청과 다르면 `Failure::StaleRevision`(`document` 50~62줄), 세션 세대가 다르면 `Failure::StaleGeneration`(`provider` 67~80줄), 서버가 기능을 광고하지 않으면 건너뜀(104줄 `state.supports_document(&document.uri, "textDocument/codeLens")`), typed 요청은 113줄 `session.client.request_typed::<lsp_types::request::CodeLensRequest>(params, Some((document.uri.clone(), document.protocol_revision)))`.
4. 응답: `Reply::CodeLenses { request, result }`(`APP/lsp.rs` 338줄) → `poll_lsp`(`APP/domains/editor.rs` 2990줄 이후)가 상태(`editor_code_lenses`)에 반영합니다.
5. refresh: 서버의 `workspace/codeLens/refresh` 요청은 `notice`(`APP/lsp.rs` 1680줄)의 1697줄 분기가 `ServerReply::Acknowledged`로 답하고 `Reply::CodeLensRefresh { project, provider }`(350줄)를 돌려줍니다.
6. initialize capability는 `initialize`(1281줄)의 JSON에 직접 추가합니다(1293~1297줄의 `params["capabilities"]["textDocument"]["codeLens"] = json!({});` 형태).

**(b) 내장 언어 서비스 구조 공급(JSON·CSS·HTML 공용).**

1. `reconcile_builtin_json`(`APP/domains/editor.rs` 2523줄)이 `self.editor_json.tick(&self.store, &self.services.tasks, context)`를 부릅니다.
2. `APP/editor-json.rs` `State::tick`(104줄)은 `(provides_symbols(snapshot) || editor_css::dialect(snapshot).is_some())` && Normal 등급 && AppFile 아님 && view가 붙은 문서만 고르고(110~118줄), key·revision·언어·등급이 바뀐 문서마다 worker(`spawn_blocking_transient_handle(WORKER_NAME, …)` 173줄)에서 `build`(196줄)를 돌립니다. 동시 실행 상한 `MAX_ACTIVE` 2, 취소는 `Arc<AtomicBool>`(Entry가 drop될 때 true).
3. 결과 `Models { symbols: Option<Arc<DocumentSymbols>>, folds: Arc<SyntaxFolds> }`는 `editor_symbols.install_builtin`·`editor_folding.install_builtin`으로 들어갑니다(`APP/domains/editor.rs` 2648~2675줄). 고정 줄(sticky)·`@` 팔레트·접기가 이 모델을 그대로 씁니다.

**(c) 포맷.** 팔레트 `monaco.editor.action.formatDocument` → `Run::FormatEditor(command)`(`UI/command-registry.rs` 395줄) → `APP/domains/editor.rs` 3839줄 → `request_editor_formatting`(2170줄) → 외부 공급자가 없으면 `editor_formatting::State`의 내장 경로 → `native_build`(`APP/editor-formatting.rs` 31줄: CSS → HTML → JSON 순서) → `apply_edits`. 저장 포맷은 `begin_native_save`(186줄), 입력·붙여넣기 포맷은 `request_editor_input_formatting`(`APP/domains/editor.rs` 2244줄) → `editor_formatting.begin_automatic`. UI는 입력한 글자마다 `UI/editor-formatting-input.rs` `Input::typed`(20줄)를 만들고, 앱이 `lsp.automatic_formatting_providers`(`APP/lsp.rs` 875줄)로 공급자를 고릅니다.

**(d) 미리보기 갱신.** watcher 이벤트 `AppEvent::FsChanged` → `explorer_search_state.tree_changes.record(event)`가 미리보기 무효화도 기록(`APP/events.rs` `record_preview` 62줄, `take_previews` 100줄) → `reconcile_previews`(`APP/domains/preview.rs` 433줄)가 형식별 캐시를 `invalidate`/`invalidate_root`/`invalidate_all` → 다음 프레임 `show_preview_tab`(754줄)이 캐시 미스로 요청을 만들고 `load_preview_requests`(678줄)가 `HostCommand::Preview(…)`를 제출합니다.

**(e) 편집기 장식 레이어.** `show_document`는 찾기·스크롤·탐색 강조·문서 하이라이트 레이어를 한 목록으로 모아(5864줄 `let find_layers = find_decorations.iter().chain(scroll_decorations.iter()).chain(navigation_highlight…).chain(document_highlights.as_ref().map(|display| &display.layer)).collect::<Vec<_>>();`) `EditorRequest.decorations`(5955줄)로 넘깁니다. 레이어의 `DecorationKind::Inline`은 행 글자색·밑줄(`UI/editor-geometry.rs` 222~238줄), `DecorationKind::Overview { lane, color, minimap }`은 overview ruler와 미니맵 표시(`UI/editor-overview.rs` `marks` 39줄)가 됩니다. 테두리처럼 레이어로 표현할 수 없는 것은 편집기 출력 geometry 위에 직접 그립니다(5994줄 `if let Some(display) = &document_highlights { display.paint(ui, &output.geometry); }`, `APP/editor-highlights.rs` `Display::paint` 414줄).

### 2.4 테스트 하네스

**`[C]` ED 검사.** `EDT/<이름>.rs`에 일반 `#[test]`를 둡니다. 원본 대조 oracle은 `EDT/fixtures/<이름>-oracle.mjs`를 bun으로 설치된 `MEXT`·`MON`에 돌려 `<이름>-oracle.rs`를 생성하고 검사 파일에서 `include!("fixtures/<이름>-oracle.rs");`로 읽는 방식입니다(선례 `EDT/fixtures/json-format-oracle.mjs` 머리의 `import { format as rawFormat } from '../../../../node_modules/monaco-editor/esm/external/jsonc-parser/lib/esm/impl/format.js'`, `EDT/emmet-completion.rs` 33줄 `include!`). 2026-11-02 lane은 셸 규칙(`/Users/hyunseokbyun/development/TAIDE/docs/utils/2026-11-02-native-lane-workflow.js` 45줄)에 bun이 없어 손 계산 기대값만 썼습니다(4절 함정 2).

**`[U]` UI 하네스.** `UIT/editor-minimap.rs`가 전형입니다.

```rust
struct Fixture {
    context: Context,
    editor: NativeEditor,
    store: EditorStore,
    view: ViewId,
    presentation: EditorPresentation,
    tokens: Option<(u64, LineTokens, TokenStyleTable)>,
    screen: Vec2,
    max_texture_side: Option<usize>,
    foreign: Option<String>,
    decorations: Vec<taide_native_editor::decoration::DecorationLayer>,
}
```

`Fixture::admitted(text, FileSizeTier::Normal, false)`가 `EditorStore::new(EditorLimits { max_documents: 2, max_views: 2, max_undo_groups: HISTORY_LIMIT, max_document_bytes: BYTE_LIMIT })`와 view를 만들고, `presentation`은 `EditorPresentation { options: EditorDisplayOptions { minimap: true, minimap_colors: Some(EditorMinimapColors { … }), ..Default::default() } }`로 둡니다. 프레임마다 `RawInput`을 넣어 `editor.show_request(...)`를 부른 뒤 `FullOutput.shapes`(색·rect)나 텍스처를 단언합니다. 텍스처를 만드는 검사는 프레임 사이에 `output.textures_delta.clear()`가 없으면 panic합니다(4절 함정 9).

**`[A]` 실제 App 하네스.** 세 묶음이 있습니다.

1. 편집기 문서 fixture: `APP/application-inlay-hints-tests.rs`의 `pub(super)` 함수.
   - `Fixture`(12줄: `directory`, `path`, `peek_path`, `project`, `tab`, `context`, `application`), `Drop`이 `on_exit`와 임시 디렉터리 삭제를 합니다.
   - `paint(application, context, events)`(32줄): 1000×700 화면으로 `context.run_ui(…, |ui| eframe::App::ui(application, ui, &mut eframe::Frame::_new_kittest()))`를 돌리고 `textures_delta`를 비웁니다.
   - `key(key, modifiers)`(49줄), `wait(application, context, phase, ready)`(62줄, 10초 한도).
   - `fixture_with_provider(mode, is_suggest_preview_enabled, provider_ready)`(153줄): 확장자 `rs` 파일 하나와 `peek.rs`를 가진 임시 프로젝트를 만들고 `connect_lsp_mock(application, context, bin, mode)`(395줄)로 mock을 `rust-analyzer`라는 이름의 실행 파일로 연결합니다. mock 경로는 `std::env::current_exe()`의 두 단계 위 `examples/native-lsp-mock`(0.3의 1번이 만듦)입니다.
   - `fixture_with_language(text, extension)`(217줄): mock `--native-inlay-hints`, 원하는 확장자 파일, `open_with`를 Editor로 고정.
   - `body_view(application, path, tab)`(434줄), `text_contains(output, label)`(457줄).
   - 팔레트 실행은 `APP/application-selection-ranges-tests.rs` 74줄 `run_palette(application, context, id)`.

   실제 사용 예(`APP/application-css-tests.rs` 1~3줄 import와 106줄 이후 검사):

   ```rust
   use super::super::inlay_hints_tests::{body_view, fixture_with_language, paint, wait};
   use super::super::selection_ranges_tests::run_palette;

   let mut fixture = fixture_with_language(CSS, "css");
   let context = fixture.context.clone();
   let path = fixture.path.clone();
   let application = fixture.application.as_mut().unwrap();
   let view = body_view(application, &path, &fixture.tab);
   let document = application.store.views().get(view).unwrap().document;
   wait_folds(application, &context, document, &[(0, 2, None), (4, 6, Some("region"))]);
   let body = crate::editor_context_menu::body(&context, view).unwrap();
   context.memory_mut(|memory| memory.request_focus(body));
   run_palette(application, &context, "monaco.editor.action.formatDocument");
   ```

   등록: CSS·HTML 검사는 `APP/application-json-tests.rs` 2077~2080줄의 `#[path = "application-css-tests.rs"] mod css_tests;`·`html_tests`로 하위 모듈 등록, 그 밖의 `[A]` 파일은 `APP/application.rs` 2898~3010줄의 `#[cfg(all(test, unix))] #[path = "application-xxx-tests.rs"] mod xxx_tests;` 목록에 등록합니다(이 영역 파일은 `markdown_tests` 2940줄, `preview_tests` 2943줄 근처에 둡니다).
2. 셸·미리보기 fixture: `APP/application-project-rail-tests.rs`의 `pub(super)` 항목 `Fixture`(15줄), `project_ref`(69줄), `isolated_state(data)`(80줄), `insert_project`(95줄), `launch(fixture, state)`(120줄), `step(application, context, events)`(142줄), `node_rect`(153줄), `pointer(position, button)`(192줄), `click_label`(210줄). 미리보기 검사(`APP/application-preview-tests.rs`)는 1~3줄에서 이것들을 import하고 그 위에 `open(name, layout)`(88줄: 임시 디렉터리에 `data`·`alpha`를 만들고 원하는 pane 트리로 시작), `wait(application, context, phase, ready)`(134줄, 실패 시 accesskit label과 글자 shape를 모두 출력), `record_change(application, project, kind, path)`(171줄: 실제 watcher 대신 `tree_changes.record(&AppEvent::FsChanged { … from_app: false })`)를 둡니다.
3. 보조 창 fixture: `APP/application-auxiliary-window-tests.rs`(preview-3 검사 `실제_app_보조_창_파일_탭_메뉴의_다시열기는_편집기와_미리보기를_전환한다`, 탐색기 rename 입력 `focused_text_input` 832줄과 `egui::Event::Text(RENAMED_FILE.into())` 1016줄).

`[A]` 단언 대상: `application.store`(문서·view·undo), 각 상태 struct, `FullOutput.shapes`의 `egui::Shape::Text`(글자·색)·`Shape::Mesh`의 `texture_id`, `platform_output.accesskit_update`의 node label·role, `platform_output.commands`(`egui::OutputCommand::CopyText`·`OpenUrl`). 미리보기 검사는 accesskit label보다 본문 글자 shape와 `texture_id`로 판정해야 안정적입니다(`QA/2026-11-02-native-lane-preview-preview-markdown.md` 90줄).

**`[L]` LSP child.** `APP/lsp-editor-code-lenses-tests.rs`가 전형입니다. 서버 이름(`rust-analyzer` 등)의 셸 스크립트를 임시 bin 디렉터리에 쓰고 `LspBridge::connect(services, bin, repaint)`로 연결합니다. mock 모드는 `MOCK`의 `main`(2265줄) 안 `match args.first().map(String::as_str)`(2273줄) 블록에 추가합니다. 코드 렌즈 모드 예(2405~2430줄):

```rust
Some(
    mode @ ("--native-code-lenses"
    | "--native-code-lenses-null"
    | "--native-code-lenses-refresh"),
) => {
    server.should_track_saves = true;
    server.locations = Some("normal");
    server.code_lenses = Some(match mode {
        "--native-code-lenses-null" => "null",
        "--native-code-lenses-refresh" => "refresh",
        _ => "normal",
    });
}
```

mock은 initialize에서 클라이언트 capability를 확인하고 없으면 오류를 냅니다(635~648줄: `params["capabilities"]["textDocument"]["codeLens"]`가 객체가 아니거나 `params["capabilities"]["workspace"]["codeLens"]["refreshSupport"] != true`면 `invalid(...)`). 새 기능도 같은 방식으로 capability 수신을 검사합니다.

### 2.5 완료 판정과 문서 갱신

- `MATRIX` 머리말 5줄: complete = 구현 + 실제 앱 경로 + 해당 동작 묶음의 자동 검사 근거. 픽셀·OS 입력·접근성 실기·성능·출시 통과는 complete의 조건이 아닙니다.
- 원본 결함·내부 수치는 재현하지 않습니다(`/Users/hyunseokbyun/development/TAIDE/docs/acknowledge/2026-10-09-native-find-regex-decisions.md` "사용자 결정": "버그마저 똑같이 구현할필요는 없어"). 재현하지 않은 결함은 QA에 원본 근거와 함께 적습니다.
- 작은 결정은 원본에 가장 가까운 안으로 정하고 QA에 근거를 적습니다(`/Users/hyunseokbyun/development/TAIDE/docs/acknowledge/2026-10-26-native-autonomous-continuation.md` "적용").
- 행을 닫을 때 함께 갱신: `MATRIX` 해당 행의 상태·비고(합계 표는 건드리지 않음), 새 QA 문서(`QA/<날짜>-native-<영역>-<묶음>.md`: 대상 ID, 원본 근거, 구현, 실제 명령과 결과 수, 원본과 다른 점, 남은 범위). 같은 성공을 중복 합산하지 않고, 실행하지 않은 검사를 통과했다고 쓰지 않습니다.

---

## 3. 행별 상세

각 행은 (a) ID·기능명·현재 상태, (b) 원본 TS 동작, (c) native 현재 자산과 정확히 빠진 것, (d) 구현 방법, (e) 자동 검사, (f) 완료 판정, (g) 막는 요소 순서입니다. 원본 Monaco 위젯 안의 문구는 영어 고정입니다(원본이 Monaco nls를 설정하지 않음, `SPEC` 2.0.3).

### 3.1 editor-7 semantic token 강조

#### (a) 상태
editor-7 · semantic token 강조 · **missing**(`MATRIX` 206줄 비고: "해당 사용자 동작의 실제 native 편집기/앱 소비 경로가 없습니다").

#### (b) 원본 동작
- 공급자 등록: `TS/shared/lib/lsp/adapters/semantic-tokens.ts` `registerSemanticTokens`(184~247줄). 서버가 `semanticTokensProvider.full`과 `legend`를 광고할 때만 등록하고, 아니면 아무것도 하지 않습니다(185~188줄). 공급자는 언어마다 등록됩니다(`TS/entities/lsp/lsp-session-registry.ts` 25줄 import, `ensureLanguageRegistered`).
- delta: `full.delta === true`이고 이 문서의 캐시 `resultId`가 Monaco가 넘긴 `lastResultId`와 같을 때만 `textDocument/semanticTokens/full/delta`(`previousResultId`), 그 밖에는 `full`(213~217줄). delta 응답이 `edits`를 가지면 캐시한 원래 `data`에 **역순** splice로 적용합니다(`applySemanticTokensDeltaEdits` 85~86줄; 모든 edit의 `start`는 적용 전 배열 기준). delta 요청에 서버가 full 형태(`data`)로 답하면 그것을 그대로 씁니다(144~146줄).
- 다시 인코딩: `decodeSemanticTokensData`(20~32줄)로 절대 위치로 풀고, `buildSemanticTokensLegendMapping`(108~122줄)이 서버 type 이름을 `SEMANTIC_TOKEN_TYPE_MAP`(`TS/shared/lib/theme-convert/semantic-token-map.ts` 32~96줄, `lookupSemanticTokenTypeMapping`로 정확 일치 — `constructor` 같은 `Object.prototype` 이름은 버림)으로 21개 표적에 대응합니다. 대응 없는 type의 토큰은 버리고 다음 남은 토큰의 delta를 남은 토큰 기준으로 다시 계산합니다(`reencodeSemanticTokens` 46~70줄). modifier는 표준 10개(`SEMANTIC_TOKEN_MODIFIERS`)만 유지합니다.
- 결과 캐시: 응답에 `resultId`가 있으면 `{ resultId, serverData }`를 문서 URI별로 저장, 없으면 지웁니다(220~221줄). Monaco가 결과를 놓으면 `releaseDocumentSemanticTokens`(226~233줄)가 그 resultId의 캐시를 지웁니다.
- null·취소: 요청이 취소되었거나 응답이 null이면 `emptySemanticTokens()`(13줄, 빈 `data`)를 돌려줍니다 → 화면의 semantic 색이 사라집니다. 요청이 throw하면 Monaco가 기존 토큰을 유지하고, 그동안 내용이 바뀌었으면 다시 예약합니다(`MON/editor/contrib/semanticTokens/browser/documentSemanticTokens.js` 237~250줄).
- 여러 공급자: Monaco는 모든 공급자에 동시에 요청하고, 공급자 순서(등록 점수 → 나중 등록 우선)대로 첫 "토큰 있는 결과"를 씁니다. 앞 공급자가 throw하면 그 오류로 전체를 실패시킵니다(`MON/editor/contrib/semanticTokens/common/getSemanticTokens.js` 35~68줄). TAIDE 공급자는 null도 빈 토큰 객체로 바꿔 돌려주므로 사실상 순서상 첫 공급자의 결과가 쓰입니다.
- 설정: `settings.editorSemanticHighlighting ?? true`를 요청마다 읽고(`TS/widgets/editor-pane/use-lsp-session.ts` 47·175줄), 꺼져 있으면 캐시를 지우고 빈 토큰(`semantic-tokens.ts` 208~211줄). 설정이 바뀌면 `observeSemanticHighlightingSetting`(75~90줄)이 `triggerSemanticTokensRefresh(client)`로 즉시 다시 요청하게 합니다(128~129줄).
- refresh: 세션별 `workspace/semanticTokens/refresh` 처리기가 그 client의 공급자만 다시 요청합니다(`TS/entities/lsp/lsp-session-registry.ts` 349~357줄 → `triggerSemanticTokensRefresh` → 공급자 `onDidChange`).
- initialize: `workspace.semanticTokens.refreshSupport: true`(`TS/shared/lib/lsp/initialize-params.ts` 86줄), `textDocument.semanticTokens = { requests: { full: { delta: true } }, tokenTypes: [...SEMANTIC_TOKEN_TYPES], tokenModifiers: [...SEMANTIC_TOKEN_MODIFIERS], formats: ['relative'], overlappingTokenSupport: false, multilineTokenSupport: false, serverCancelSupport: false, augmentsSyntaxTokens: true }`(130~139줄).
- Monaco 쪽 활성: `TS/features/editor/code-editor.tsx` 200줄 `'semanticHighlighting.enabled': true`.
- 일정(`MON/editor/contrib/semanticTokens/browser/documentSemanticTokens.js`, 사양 정정 포함):
  - 편집기에 붙은 모델만 요청합니다(206~209줄 `isAttachedToEditor`).
  - 진행 중 요청은 1개입니다(196~200줄).
  - 최초 생성과 언어 변경은 0ms(163·156줄), 공급자의 `onDidChange`(=refresh·설정 변경)는 0ms(`handleProviderDidChange` 169~179줄, 진행 중이면 결과 적용 뒤 재예약).
  - 내용 변경·편집기 부착 변경·공급자 등록 변경·**테마 변경**은 적응형 지연(`REQUEST_MIN_DELAY` 300ms ~ `REQUEST_MAX_DELAY` 2000ms, 119~120·128줄)으로 예약합니다. 테마 변경은 먼저 토큰을 지웁니다(157~161줄). `SPEC` 2.2의 "테마 변경 시 즉시 재요청"은 실제 코드와 다릅니다.
  - 결과가 도착했을 때 그 사이 내용이 바뀌었으면 결과를 적용한 뒤 재예약합니다(`_setDocumentSemanticTokens` 260~266줄).
  - 편집 중에는 기존 semantic 토큰을 편집 위치에 맞춰 옮깁니다(`MON/editor/common/tokens/sparseMultilineTokens.js` `acceptDeleteRange` 285줄·`acceptInsertText` 423줄의 경우 분류: 토큰 앞/뒤/안쪽 삽입·삭제, 단어 문자 1개를 토큰 끝에 넣으면 토큰이 늘어남).
- 색과 글꼴(사양 정정): Monaco standalone의 `getTokenStyleMetadata`(`MON/editor/standalone/browser/standaloneThemeService.js` 149~161줄)는 `tokenTheme._match([type, ...modifiers].join('.'))`의 결과에서 foreground와 italic·bold·underline·strikethrough 네 값을 모두 boolean으로 돌려줍니다. `MON/editor/common/services/semanticTokensProviderStyling.js` 52~70줄은 값이 `undefined`가 아니면 `SEMANTIC_USE_*` 비트를 켜므로, **네 글꼴 속성과 전경색이 항상 TextMate 토큰 값을 덮습니다**. `SPEC` 2.2의 "규칙에 fontStyle이 없으면 TextMate 글꼴 스타일을 유지"는 설치된 Monaco 0.56 코드와 다릅니다. 규칙(`taideSemantic.<type>`)이 없거나 fontStyle이 없으면 trie의 기본 규칙(전경 = 편집기 전경, fontStyle 없음)이 적용됩니다. modifier가 붙은 문자열도 trie 접두 일치로 type 규칙을 받으므로 modifier는 표시 결과를 바꾸지 않습니다.
- 버리는 토큰(`semanticTokensProviderStyling.js` 145~180줄): 길이 0 이하, 같은 줄 앞 토큰의 끝보다 앞에서 시작하는 겹친 토큰.
- 사용자 문자열: 편집기 안에는 없습니다. 설정 화면 `settings.editorSemanticHighlighting`·`settings.editorSemanticHighlightingDescription`(`LOC` ko 728~729줄)은 이미 native 설정 UI에 연결되어 있습니다(`UI/settings-code-controls.rs` `CodeSwitch::EditorSemanticHighlighting`).

#### (c) native 현재 자산과 빠진 것
- 있는 것
  - SDK typed 요청: `"textDocument/semanticTokens/full" => request::SemanticTokensFullRequest`, `"textDocument/semanticTokens/full/delta" => request::SemanticTokensFullDeltaRequest => Option<SemanticTokensDeltaResult>`(`CR/taide-lsp/src/native/feature.rs` 203~204줄), `SemanticTokensDeltaResult { Tokens(lsp_types::SemanticTokens) | Delta(SemanticTokensDelta) }`·`SemanticTokensEdit { start, delete_count, data }`(88~108줄), 응답 투영 검증(`validate_projection` 307줄 이후: data 길이 5의 배수·u32 범위, edits의 start·deleteCount).
  - capability 판정: `CR/taide-lsp/src/native/capabilities.rs` 59~74줄(`full` 켜짐, `full.delta == true`), 동적 등록(`registration.rs` 181·341줄), `RefreshKind::SemanticTokens`(`protocol.rs` 272줄).
  - 테마 규칙: `SYN/token-theme.rs` `SEMANTIC_TOKEN_THEME_TARGETS`(109~131줄, 21개: namespace·type·class·enum·interface·parameter·variable·property·constant·function·method·keyword·comment·string·number·regexp·operator·decorator·storage·invalid·punctuation), `token_color_rules`(291줄)의 305줄 반복이 `taideSemantic.<표적>` 규칙을 만듭니다. `style_table`(163~185줄)이 `monaco.matched(scope)`로 `TokenStyle`을 만드는 방식이 semantic 표에도 그대로 쓰일 수 있습니다.
  - 설정 모델 `editor_semantic_highlighting`(`CR/taide-model/src/settings.rs` 328줄, 기본 `default_true()` 596줄).
- 빠진 것(재확인)
  - **SDK가 legend를 노출하지 않습니다**: `SessionSnapshot`(`CR/taide-lsp/src/native/session.rs` 45~61줄)에는 문서별 서명·완성·code action·onType 옵션만 있고 semantic token 옵션(legend·`full.delta`)이 없습니다. 원시 capability를 꺼내는 공개 API도 없습니다(`session.rs`의 `pub fn` 목록: `supports_document`·`snapshot`·`request_typed` 등). legend 없이는 응답의 type index를 해석할 수 없습니다.
  - `APP/lsp.rs` `initialize`(1281~1301줄)에 semanticTokens capability가 없습니다. `notice`(1680줄)는 `Refresh(RefreshKind::CodeLens)`만 처리하고 나머지 서버 요청은 1765줄에서 `ServerReply::Rejected(Rejection::Unavailable)`로 답합니다. 따라서 지금 서버는 semantic 요청을 받지 않고 refresh 요청은 거절됩니다.
  - `grep -rn -i "semantic" APP UI ED`의 결과는 설정 컨트롤과 무관한 이름뿐입니다. 요청·캐시·표시 코드가 없습니다.

#### (d) 구현 방법
1. **SDK 옵션 노출(`CR/taide-lsp`, 공용 crate)**
   1. `CR/taide-lsp/src/native.rs`의 `completion_options`(224줄) 옆에 `pub(crate) fn semantic_token_options(&self, uri: &str) -> Vec<lsp_types::SemanticTokensOptions> { self.document_options(uri, "semanticTokensProvider", "textDocument/semanticTokens/full") }`를 추가합니다. 정적 capability의 `semanticTokensProvider`는 `SemanticTokensOptions` 또는 `SemanticTokensRegistrationOptions` 형태인데, 후자의 추가 필드(`documentSelector`·`id`)는 serde가 무시하는지 `[C]`로 확인합니다(`lsp_types::SemanticTokensOptions`에 `deny_unknown_fields`가 없으면 그대로 역직렬화됨).
   2. `CR/taide-lsp/src/native/session.rs`의 `SessionSnapshot`(45줄)에 `pub document_semantic_token_options: Arc<BTreeMap<String, Vec<lsp_types::SemanticTokensOptions>>>`를 추가하고, 같은 파일의 초기화(230·545줄), `publish`(599줄 이후)의 계산·이전 값 재사용(620줄 근처)·`send_replace`(688줄 이후) 세 곳에 `completions`와 같은 방식으로 넣습니다.
   3. `SessionSnapshot`을 literal로 만드는 앱 검사 helper 두 곳(`APP/lsp-status.rs` 119줄, `APP/lsp-recovery.rs` 220줄)에 `document_semantic_token_options: Default::default(),`를 추가합니다.
   4. 0.3의 9번 루트 workspace 명령으로 `taide-lsp`와 Tauri 쪽 컴파일을 확인합니다.
2. **ED core `ED/semantic-tokens.rs`(새 파일, `ED/lib.rs` 모듈 목록의 알파벳 위치에 `#[path = "semantic-tokens.rs"] pub mod semantic_tokens;`)**: 정규식이 필요 없습니다.
   1. `pub enum Target`을 `SEMANTIC_TOKEN_THEME_TARGETS`와 같은 21개·같은 순서로 정의하고 `pub fn name(self) -> &'static str`.
   2. `pub fn target(server_type: &str) -> Option<Target>`: `semantic-token-map.ts` 32~96줄 표를 `match` 하나로 옮깁니다(정확 일치, 표에 없는 이름은 `None`).
   3. `pub fn apply_delta(base: &[u32], edits: &[SemanticTokensEdit]) -> Result<Vec<u32>, Failure>`: edits를 역순으로 splice합니다. `start + delete_count > base.len()`이면 `Err`(호출자는 캐시를 지우고 다음 요청을 full로 보냄).
   4. `pub fn decode(data: &[u32], legend: &[Option<Target>], snapshot: &DocumentSnapshot, limits: Limits) -> Result<Layer, Failure>`: 5개씩 읽어 줄·UTF-16 열을 byte 범위로 바꿉니다(위치 변환은 기존 `taide_native_editor::lsp::position_to_byte(&snapshot, Position::new(line, character))`). 대응 없는 type은 건너뛰되 줄·열 누적은 서버 스트림 기준으로 계속합니다. 길이 0·같은 줄 겹침은 버립니다(원본 145~180줄 규칙). 줄을 넘는 길이는 그 줄 끝에서 자릅니다(`multilineTokenSupport: false`). 토큰 수·작업 예산 상한(`Limits`)을 둡니다(선례: JSON 서비스들의 `Limits`).
   5. `pub struct Layer { pub document: DocumentId, pub revision: u64, pub spans: Vec<(Range<usize>, Target)> }`와 편집 추적 `pub fn accept(&mut self, changes: &ChangesSince<'_>)`: `MON/editor/common/tokens/sparseMultilineTokens.js` `acceptDeleteRange`(285줄)·`acceptInsertText`(423줄)의 경우 분류를 byte 범위로 이식합니다(단어 문자 1개를 토큰 끝에 넣으면 토큰을 늘리는 규칙 포함). 변경 열람은 `EditorStore::changes_since(document, revision)`(선례 `UI/editor_surface.rs` `tracked_layers` 225줄)입니다.
3. **SYN `SYN/token-theme.rs`(추가만)**: `pub fn semantic_style_table(&self) -> Result<[TokenStyle; 21], TokenThemeError>`. `style_table`의 `style` 클로저처럼 `monaco.matched(&format!("taideSemantic.{name}"))`로 foreground와 네 글꼴 비트를 모두 채웁니다((b)의 정정대로 "미지정" 표현은 필요 없습니다). `kind`는 `TokenKind::Other`로 둡니다.
4. **APP LSP(`APP/lsp.rs`)**
   1. `initialize`에 원본과 같은 두 capability를 추가합니다: `params["capabilities"]["textDocument"]["semanticTokens"] = json!({ "requests": { "full": { "delta": true } }, "tokenTypes": [...], "tokenModifiers": [...], "formats": ["relative"], "overlappingTokenSupport": false, "multilineTokenSupport": false, "serverCancelSupport": false, "augmentsSyntaxTokens": true });`, `params["capabilities"]["workspace"]["semanticTokens"] = json!({ "refreshSupport": true });`. 표준 type·modifier 목록은 `TS/shared/lib/lsp/protocol.ts`의 `SEMANTIC_TOKEN_TYPES`·`SEMANTIC_TOKEN_MODIFIERS`와 같은 순서로 `lsp_types::SemanticTokenType`·`SemanticTokenModifier` 상수로 만듭니다.
   2. `notice`의 1697줄 분기를 `RefreshKind::SemanticTokens`도 받게 넓히고 `Reply::SemanticTokensRefresh { project, provider }`를 추가합니다(코드 렌즈와 같은 `ProviderIdentity` 구성).
   3. `pub(crate) fn semantic_token_providers(&self, project, snapshot)` = `ready_feature_providers(project, snapshot, "textDocument/semanticTokens/full")`, `pub(crate) fn semantic_tokens(&self, request) -> AppResult<()>`.
   4. 새 파일 `APP/lsp-editor-semantic-tokens.rs`(`APP/lsp.rs` 상단 `#[path]` 목록 39~76줄에 알파벳 위치로 추가): `lsp-editor-code-lenses.rs`의 `document`·`provider`·`identity` 검사를 같은 방식으로 쓰고, 세션 순서는 `Reverse(session.order)`. legend는 `session.client.snapshot().document_semantic_token_options.get(&document.uri)`의 첫 옵션에서 읽고 `target()`으로 `Vec<Option<Target>>`을 만듭니다. 공급자별 문서 캐시 `HashMap<(ProviderIdentity, DocumentId), (String /* resultId */, Vec<u32>)>`를 worker 쪽에 둡니다. delta 조건은 `supports_document(uri, "textDocument/semanticTokens/full/delta")` && 캐시 있음. 응답은 `Reply::SemanticTokens { request, result: Result<Option<Payload>, Failure> }`(`Payload { provider, legend, data, result_id }`), `Ok(None)`이 null입니다. 세대가 바뀐 공급자의 캐시는 버립니다.
5. **APP 상태 `APP/editor-semantic-tokens.rs`(새 파일, `APP/lib.rs` 편집기 모듈 목록 정렬 위치)**: 상태는 2.2절 8번의 `domains::editor::State` 안에 `semantic: crate::editor_semantic_tokens::State`로 둡니다. 문서별 `Entry { revision, providers, in_flight: Option<Request>, layer: Option<Layer>, due: Instant, failed: bool }`.
   1. 대상: view가 붙은 문서, `DocumentKey::File`, Normal 등급(LSP 세션 조건과 같음). git lane 병합 뒤에는 `Virtual`이 자연히 빠집니다(File만 고르므로).
   2. 일정(원본 그대로): 최초·언어 변경·refresh·설정 켬은 즉시. 내용 변경·공급자 집합 변경·테마 변경은 300ms(원본 적응형 최소값. 최대 2000ms는 요청 시간 측정 기반이라 측정 없이 고정 최소값을 쓰고 QA에 적음). 진행 중 요청은 문서당 1개. 결과 도착 시 revision이 바뀌었으면 layer를 `accept`로 옮겨 적용한 뒤 재예약.
   3. 설정 꺼짐: 요청하지 않고 layer를 모두 지움. 다시 켜면 즉시 요청. 판정은 매 tick `self.services.state.settings.read().editor_semantic_highlighting`.
   4. 테마 변경: layer를 지우고 300ms 뒤 재요청. 감지는 `APP/editor-syntax.rs` `follow_theme`(653줄: 테마가 바뀌면 `TokenTheme::from_resolved` 666줄 → `pipeline.set_theme`)에서 같은 `TokenTheme`의 `semantic_style_table()`을 계산해 세대 번호와 함께 보관하는 메서드를 `editor_syntax` 상태에 추가하고, semantic 상태가 세대 변화를 봅니다.
   5. 오류: 기존 layer 유지, 진행 중 변경이 있었으면 재예약. null: layer 제거.
   6. 다중 공급자: 공급자 순서(세션 `order` 내림차순)대로 첫 성공 결과를 씁니다. 앞 공급자의 오류는 원본처럼 전체 실패(기존 layer 유지)로 처리합니다.
   7. tick은 `reconcile_editor_features`(`APP/domains/editor.rs` 5077줄) 안, 응답 반영은 `poll_lsp`(2990줄)의 `Reply` match에 추가합니다. 종료(`closing`·`is_shutting_down`) 때 모든 진행 중 요청을 취소하고 상태를 비웁니다.
6. **UI 표시(`UI/editor-row-text.rs`·`UI/editor-geometry.rs`·`UI/editor_surface.rs`·`UI/editor-minimap.rs`, 추가만)**
   1. `RowText::restyle(&mut self, chars: Range<usize>, foreground: Color32, font_style: RowFontStyle)`: `decorate`(442줄)처럼 `split_section`(464줄)으로 경계를 나눈 뒤 범위 안 section의 `foreground`와 `font_styles[index]`를 덮습니다.
   2. 표면 입력: `EditorDisplayOptions`에 `#[cfg(feature = "native-host")] pub semantic: Option<Arc<SemanticOverlay>>`를 추가합니다(`SemanticOverlay { document: DocumentId, revision: u64, style_generation: u64, spans: Vec<(Range<usize>, Target)>, styles: [TokenStyle; 21] }`). `EditorDisplayOptions`는 `PartialEq`를 파생하므로 `SemanticOverlay`의 `PartialEq`는 `document`·`revision`·`style_generation`만 비교하도록 직접 구현해 매 프레임 비교 비용을 막습니다. 앱 `show_document`(5560~5580줄의 `editor_presentation.options.*` 대입 근처)에서 현재 문서의 overlay를 넣습니다.
   3. 본문 행: `RowLayout`(`UI/editor-geometry.rs` 154줄)에 `#[cfg(feature = "native-host")] pub(crate) semantic: Option<&'a SemanticOverlay>`를 추가하고, `RowLayout::row`에서 `text.highlight(...)`(192줄) 직후·괄호 색(199줄) 앞에 해당 행 범위의 span마다 `restyle`을 부릅니다(괄호 색과 찾기·진단 장식이 semantic 위에 오도록). overlay는 `overlay.document == document.id && overlay.revision == document.revision`일 때만 씁니다. `RowLayout` literal은 `UI/editor_surface.rs` 1107·2740줄 두 곳이고, 고정 줄은 `UI/editor-sticky-scroll.rs` 521줄 `..*frame.rows`로 같은 값을 받습니다.
   4. peek(참조 미리보기 편집기)는 `APP/domains/editor.rs` 5634줄 `let preview_presentation = editor_presentation.clone();`으로 그려지므로 clone 뒤 `preview_presentation.options.semantic`을 peek 문서의 overlay(없으면 `None`)로 바꿉니다. 같은 `DocumentId`의 split mirror 뷰는 같은 overlay를 받습니다.
   5. 미니맵: `MinimapFrame`(294줄)에 `semantic: Option<&'a SemanticOverlay>`를 넣고 `render_image`(456줄)가 토큰 전경색을 고른 뒤 그 byte를 덮는 span이 있으면 semantic 전경색으로 바꿉니다. `ImageKey`(78줄)에 `semantic: Option<(u64, u64)>`(revision, style_generation)를 넣어 텍스처를 다시 만듭니다.

#### (e) 자동 검사
1. `[C]` SDK: `CR/taide-lsp`의 기존 capability 검사 옆에 정적 `semanticTokensProvider`와 동적 등록 옵션 각각에서 `semantic_token_options`가 legend·`full.delta`를 돌려주는지.
2. `[C]` `EDT/semantic-tokens.rs`(새 파일): 원본 `TS/shared/lib/lsp/adapters/semantic-tokens.test.ts`의 사례를 옮깁니다. 단언: decode 절대 위치, delta 역순 적용(서로 다른 start 두 개), 대응 없는 type 제거 뒤 다음 토큰 위치 유지, `constructor`·`toString` type 이름 제거, 길이 0·겹침 제거, 줄 끝 자르기, 비BMP 문자(UTF-16 2단위) 위치의 byte 변환, 토큰 수 상한 `Err`, `accept`의 앞 삽입 이동·토큰 안쪽 삽입 확장·토큰 끝 단어 문자 삽입 확장·삭제 경우(sparseMultilineTokens 표의 각 경우 1건).
3. `[C]` SYN: 21개 순서가 `SEMANTIC_TOKEN_THEME_TARGETS`와 같음, 규칙 없는 표적은 편집기 전경색·글꼴 없음, `bold italic` 규칙은 두 비트 모두.
4. `[L]` `MOCK`에 `--native-semantic-tokens`(full + delta + resultId), `-null`, `-error`, `-refresh`(첫 응답 뒤 `workspace/semanticTokens/refresh` 송신), `-unsupported`, `-full-only`(delta 미광고) 모드를 추가하고 initialize에서 두 capability 수신을 확인합니다(코드 렌즈 mock 635~648줄 방식). `APP/lsp-editor-semantic-tokens-tests.rs`(`APP/lsp.rs` 테스트 `#[path]` 목록 80~121줄에 등록): 첫 요청 full, 둘째 요청 delta와 `previousResultId`, refresh 요청 시 `Reply::SemanticTokensRefresh`, null·오류·취소.
5. `[A]` `APP/application-semantic-tokens-tests.rs`(새 파일, `APP/application.rs` 테스트 목록에 등록): `fixture_with_provider("--native-semantic-tokens", false, |application, project, document| …semantic 공급자 준비…)`로 `.rs` 파일을 열고 (1) 응답 뒤 해당 범위 글자 shape(`egui::Shape::Text`의 galley section 색)가 `semantic_style_table`의 색, (2) 토큰 앞 입력 직후(새 응답 전) 같은 글자가 이동한 위치에서 같은 색, (3) 설정 끔 → 다음 프레임 TextMate 색으로 복귀·요청 없음, 다시 켬 → 즉시 요청, (4) peek와 본문이 같은 색, (5) 테마 변경 → 색 제거 뒤 재요청, (6) 종료 뒤 `services.tasks` 활성 작업 0.
6. 실행: 0.3의 1·2·3·4·5·6·9번(필터 `semantic`), 동결 미러, fmt.

#### (f) 완료 판정
위 `[C]`·`[L]`·`[A]` 통과, `MATRIX` editor-7 complete와 비고(원본 일정 중 적응형 지연의 고정 최소값 사용을 차이로 기록), settings-28 비고 갱신 요청, QA에 (b)의 두 사양 정정(글꼴 네 속성 항상 덮음, 테마 변경 지연)을 원본 줄과 함께 기록.

#### (g) 막는 요소
없음. 공용 crate(`CR/taide-lsp`) 변경은 새 의존성이 아니므로 허락 대상이 아니지만 Tauri 앱과 공유하므로 루트 workspace 검사가 필수입니다. 실제 서버(rust-analyzer 등) 색 확인은 실기 항목이며 complete 조건이 아닙니다.

### 3.2 editor-15 minimap과 토글 액션

#### (a) 상태
editor-15 · minimap과 토글 액션 · **partial**(`MATRIX` 214줄: "남은 것은 section header(MARK·region)·접힌 영역 배경·선택 텍스트 발생 강조입니다").

#### (b) 원본 동작
- 표시: `TS/features/editor/code-editor.tsx` 275줄 `updateOptions({ minimap: { enabled: minimap && !largeFile } })`, 토글 액션 `taide.toggleMinimap`(100줄 `TOGGLE_MINIMAP_ACTION_ID`, 문구 `LOC` ko 80줄 `editor.toggleMinimap` "미니맵 전환"). 이미 native에 있습니다.
- section header 계산(`MON/editor/contrib/sectionHeaders/browser/sectionHeaders.js`):
  - 언어 구성에 주석 설정도 folding marker도 없으면 계산하지 않습니다(`createOptions` 78~95줄). 미니맵이 꺼져 있어도 계산은 하지만 그려지는 곳이 미니맵뿐입니다.
  - 일정: 모델·언어 변경 0ms, 내용 변경 250ms(`RunOnceScheduler(..., 250)` 73~75줄), 토큰 변경은 예약이 없을 때 1000ms(68~72줄). `model.isTooLargeForSyncing()`(50MB 초과)이면 계산하지 않습니다(104줄). TAIDE는 50MB 초과 파일을 열지 않으므로(`CR/taide-model/src/file.rs` 9줄 `REFUSED_FILE_BYTES` 50MiB) 사실상 항상 계산합니다.
  - region(`MON/editor/common/services/findSectionHeaders.js` `collectRegionHeaders` 30~51줄): 줄마다 folding marker 시작 정규식이 일치하면 `match[0].length`(일치 길이) 뒤 나머지를 `getHeaderText`로 처리합니다. 나머지가 비면(끝 열 ≤ 시작 열) 버립니다. `shouldBeInComments: false`.
  - MARK(`collectMarkHeaders` 52~118줄): 정규식 `\bMARK:\s*(?<separator>-?)\s*(?<label>.*)$`(`MON/editor/common/config/editorOptions.js` 1528줄), 플래그 `gdm`(`isMultilineRegexSource`가 거짓이라 `s` 없음). 100줄 묶음·5줄 겹침(`CHUNK_SIZE` 100, `MAX_SECTION_LINES` 5)으로 줄을 `\n`으로 이어 찾습니다. **`\s`는 `\n`도 포함하므로 `// MARK:`가 줄 끝에 있으면 label이 다음 비어 있지 않은 줄의 내용이 됩니다**(이 경우 범위가 두 줄). label은 `-` 제거나 trim을 하지 않습니다(그룹 값 그대로). label도 separator도 없으면 버리고, 앞에 넣은 header의 끝 줄보다 뒤에서 시작할 때만 추가합니다(110~113줄). `shouldBeInComments: true`.
  - 표시 직전 필터(`updateDecorations` 117~135줄): `shouldBeInComments`인 header는 범위 시작 위치의 표준 토큰 종류가 Comment이고 그 위치 언어가 모델 언어와 같을 때만 남습니다.
  - `getHeaderText`(`findSectionHeaders.js` 121~126줄): trim → `startsWith('-')`면 구분선 → 앞뒤 `-+` 제거.
  - 기본 옵션: `showRegionSectionHeaders`·`showMarkSectionHeaders` true, `sectionHeaderFontSize` 9, `sectionHeaderLetterSpacing` 1(`editorOptions.js` 1526~1530줄).
- section header 그리기(`MON/editor/browser/viewParts/minimap/minimap.js` 1394~1458줄): 배경 높이 = 글꼴 크기×1.5, 배경색 = 미니맵 배경의 RGB에 불투명도 0.7, 글자색 = 테마 `editor.foreground`(`_getSectionHeaderColor` 97줄), 글꼴 `'500 ' + 9px + DEFAULT_FONT_FAMILY`(74·1407줄), 자간 1px(배율 미적용), 글자 기준선 y = 그 줄의 미니맵 y + 글꼴 크기, 배경 y = 기준선 − 글꼴 크기, 폭을 넘으면 `_fitSectionHeader`(1421~1442줄)가 가운데를 `…`로 줄임(공백 뒤에서 자르지 않음), 구분선이면 배경 y+2에 두께 0.4 선. 뷰포트 위쪽으로 글꼴 높이만큼 더 넓혀 찾습니다(`getSectionHeaderDecorationsInViewport` 843~847줄).
- 접힌 영역 배경(`MON/editor/contrib/folding/browser/foldingDecorations.js` 22·30·43~82줄): 접힌 범위(시작 줄)에 `editor.foldBackground` 색 배경(`foldingHighlight` 기본 true, `editorOptions.js` 3220줄). 기본색은 `editor.selectionBackground`의 불투명도 0.3이며, highlighted 변형이 `minimap: { color: foldBackground, position: Inline }`을 가집니다.
- 선택 텍스트 발생 강조의 미니맵 표시: 3.16 C의 본문 기능이 문서 하이라이트 공급자가 **없을 때만** 미니맵·overview 장식을 가집니다(`MON/editor/contrib/wordHighlighter/browser/highlightDecorations.js` 59~75·100~103줄: 공급자가 있으면 `_SELECTION_HIGHLIGHT_OPTIONS_NO_OVERVIEW`, overview·미니맵 없음).
- 사용자 문자열: `editor.toggleMinimap` 외에 없음(section header label은 문서 텍스트).

#### (c) native 현재 자산과 빠진 것
- 있는 것: 표시·설정 토글·찾기·진단·문서 하이라이트(`APP/editor-highlights.rs` 200줄 `minimap: value("minimap.selectionOccurrenceHighlight", selection)`)·선택 영역. 미니맵 `paint`(`UI/editor-minimap.rs` 311줄)가 `frame.marks`(`ScrollMark`)의 `minimap` 색으로 배경(396줄)·범위(408줄)를 그립니다. Normal 등급이 아니면 미니맵 자체를 만들지 않습니다(`measure` 50~55줄). 본문 접힌 배경은 `UI/editor-paint.rs` `row_background`(105줄)가 `row.heads_hidden_lines`(`UI/editor-geometry.rs` 351줄 `self.display.hidden_lines_at(segment.line + 1).is_some()`)일 때 `folded_background`(181줄, `appearance.selection.gamma_multiply(0.3)`)로 그립니다.
- 빠진 것(재확인): `grep -n -i "section" UI/editor-minimap.rs` 결과 없음. 미니맵에 접힌 줄 배경이 없습니다(`paint`가 `heads_hidden_lines`를 보지 않음). `ED`에 `section-headers.rs`가 없습니다. 선택 텍스트 발생 강조는 본문에도 없습니다(3.16 C).
- 사양 정정: `SPEC` 2.3은 region marker를 `ED/language-configuration.rs`에서 쓴다고 적었지만 marker 정규식은 `SYN`에 있습니다(`SYN/language-configuration.rs` `FoldMarkers` 167줄, `fold_marker` 750줄, `starts_marker_region` 772줄). `ED/language-configuration.rs`의 trait `LanguageRules`(118줄)에는 `fold_marker`(150줄)·`starts_marker_region`(152줄)·`comments`(129줄, 기본 `None`)가 있고 일치 끝 위치를 주는 메서드는 없습니다. `SPEC` 2.3의 "줄 단위 일치"는 MARK의 줄 넘김 동작과 다릅니다((b)).
- 테마 키 `editor.foldBackground`는 native 테마에 없습니다(`grep -rn foldBackground CR/taide-theme/src UI` 결과 없음). 본문과 같은 `selection × 0.3`이 원본 기본값과 같습니다.

#### (d) 구현 방법
1. **ED trait 확장(`ED/language-configuration.rs`, 추가만)**: `LanguageRules`에 기본 구현이 있는 `fn region_marker_length(&self, _line: &str) -> Option<usize> { None }`을 추가합니다(기본 구현이라 다른 구현자가 깨지지 않음). `SYN`의 `MonacoLanguage` 구현(`starts_marker_region` 772줄 옆)은 `self.fold_markers.as_ref()?.start.first_range(line).map(|range| range.len())`입니다(`SYN/js-regex.rs` 67줄 `first_range`). 원본은 `match[0].length`로 자르므로 `range.len()`을 씁니다. 내장 marker 시작 정규식은 모두 `^`로 시작해(`<ROOT>/native/taide-native-syntax/language-configurations/monaco.json` 130·263·401·534·738줄) `range.end`와 같습니다.
2. **ED core `ED/section-headers.rs`(새 파일)**
   1. `pub struct Header { pub line: usize, pub end_line: usize, pub start_byte: usize, pub text: String, pub has_separator: bool, pub should_be_in_comments: bool }`.
   2. `pub fn find(snapshot: &DocumentSnapshot, rules: &dyn LanguageRules, limits: Limits) -> Result<Vec<Header>, EditorError>`: 원본 순서대로 region(`rules.comments().is_some() || region 가능` 판정은 앱에서), 그 다음 MARK.
   3. region: 줄마다 `region_marker_length`; 나머지가 비면 버림; `getHeaderText` 이식(JS `String.prototype.trim`의 공백 집합은 `ED/json-parser.rs`의 ECMA 공백 helper `is_ecma_whitespace`를 재사용).
   4. MARK: 원본의 100줄 묶음·5줄 겹침 알고리즘을 그대로 이식합니다. 묶음 텍스트(줄을 `\n`으로 이음) 안에서 손으로 만든 매처로 `\bMARK:`를 찾습니다(JS `\b`는 ASCII 단어 문자 `[A-Za-z0-9_]` 경계). 그 뒤 JS `\s`(줄바꿈 포함)를 건너뛰고, 선택적 `-`, 다시 JS `\s`, 그 위치부터 다음 `\n`(또는 `\r`·U+2028·U+2029, JS `.`가 넘지 못하는 문자) 전까지가 label입니다. `$`(m 플래그)는 줄 끝에서 성립하므로 label 끝 = 줄 끝입니다. 일치 후 `lastIndex`를 일치 끝으로 옮기고, 앞 header의 `end_line` < 이번 시작 줄일 때만 추가합니다.
   5. 작업 예산(바이트·header 수)과 취소 플래그를 둡니다.
3. **APP 일정(`APP/editor-section-headers.rs` 새 파일, 상태는 `domains::editor::State`)**: view가 붙고 미니맵이 켜진 Normal 문서만(미니맵이 꺼지면 원본도 화면 결과가 없음), 언어 변경·최초 0ms, 내용 변경 250ms, 토큰 세대(`LineTokens::generation`, `ED/line-tokens.rs` 176줄) 변경은 예약이 없을 때 1000ms. 계산은 `spawn_blocking_transient_handle`. 언어 구성은 `crate::editor_syntax::language_rules(&snapshot.metadata.language_id)`(`APP/editor-syntax.rs` 190줄). 결과는 문서 revision과 함께 저장합니다.
4. **주석 필터**: 표시 직전에 `should_be_in_comments` header는 `start_byte` 위치의 토큰 종류가 `TokenKind::Comment`(`ED/syntax.rs` 6~11줄)인 것만 남깁니다. 토큰은 `EditorTokens.lines.spans(line)`·`styles.style(id).kind`(`SYN/token-theme.rs` 177줄 `standard_token_kind`)에서 읽습니다. 그 줄 토큰이 아직 없으면 숨기고, 토큰 세대가 바뀌면 다시 판정합니다(원본의 토큰 변경 1000ms 재계산과 같은 시점).
5. **UI 미니맵(`UI/editor-minimap.rs`, 추가만)**
   1. `EditorDisplayOptions`에 `#[cfg(feature = "native-host")] pub section_headers: Option<Arc<SectionHeaders>>`(`SectionHeaders { document, revision, items: Vec<(usize /* line */, String, bool /* separator */)> }`)를 추가하고, `MinimapFrame`에 `section_headers: &'a [(usize, String, bool)]`를 추가해 `paint`에 넘깁니다. revision이 문서와 다르면 비웁니다.
   2. 접힌 영역: `paint`에서 선택 배경(381~393줄) 다음, mark 배경(396줄) 앞에 `frame.layout.rows`의 각 행에 대해 `frame.display.hidden_lines_at(segment.line + 1).is_some()`(본문과 같은 판정)이면 그 행에 `paint_background(&painter, &frame, &mut highlighted, segment.bytes.clone(), frame.appearance.selection.gamma_multiply(0.3))`를 부릅니다. 새 색 필드는 필요 없습니다(원본 기본값 = 본문 접힌 배경 색).
   3. section header: (b)의 원본 그리기 규칙 그대로. 글꼴은 `FontId::proportional(9.0)`(배율 적용 전 point 단위), 굵기 500은 egui에 없으므로 기본 굵기로 그리고 QA에 차이를 기록합니다. 자간은 `LayoutJob`의 `TextFormat { extra_letter_spacing: 1.0, .. }`. 배경 `frame.colors.background`의 RGB에 α 0.7, 글자 `frame.appearance.foreground`, 구분선 `Stroke::new(0.4, foreground)`. 텍스처(`render_image`)가 아니라 텍스처 위 painter로 그리므로 `ImageKey`는 바꾸지 않습니다.
6. **선택 발생 강조의 미니맵 표시**: 3.16 C가 만든 장식 레이어의 `DecorationKind::Overview { lane: Center, color, minimap: Some(color) }`가 기존 `marks` → `paint` 경로로 그려집니다. 문서 하이라이트 공급자가 있으면 원본처럼 Overview 장식을 만들지 않습니다.

#### (e) 자동 검사
1. `[C]` `EDT/section-headers.rs`: `// MARK: - Foo`(구분선+label), `// MARK: Bar`, `// MARK: -`(구분선만), `// MARK:` 뒤 빈 줄·다음 줄 `fn x()`(label이 `fn x()`가 되고 범위가 두 줄), 단어 중간 `XMARK:`(버림), region `// #region Foo`·`// #region -`·`// #region`(버림), CRLF, 99·100·101줄 경계에 걸친 MARK가 한 번만 나옴. oracle은 bun 실행이 허락되면 `collectMarkHeaders`·`findSectionHeaders`를 직접 돌려 생성합니다.
2. `[U]` `UIT/editor-minimap.rs`에 두 검사: header 글자 shape(텍스트·y 위치·9px)와 배경 rect(높이 13.5, α 0.7), 구분선 선분; 접힌 시작 줄 행의 배경 rect 색 = selection×0.3에 `paint_background`의 감쇠(`HALF_OPACITY * CANVAS_OPACITY`)를 곱한 값.
3. `[A]` `APP/application-minimap-tests.rs`(새 파일): 실제 `.ts` 파일의 `// MARK: - Foo`와 `//#region Bar` → 미니맵에 두 글자 shape, 문자열 안의 `"MARK: X"`는 표시 안 됨(주석 아님), 편집 후 250ms 뒤 갱신, 접기 명령 뒤 미니맵 배경 rect, Large 등급 파일은 미니맵 자체 없음.
4. 실행: 0.3의 2·3·4·5·6번, 동결 미러(`EditorDisplayOptions` 필드 추가), fmt.

#### (f) 완료 판정
section header·접힌 영역 배경·(3.16 C 뒤) 선택 발생 미니맵 표시의 `[C]`·`[U]`·`[A]` 통과, `MATRIX` 비고의 남은 범위 삭제.

#### (g) 막는 요소
선택 발생 미니맵 표시는 3.16 C가 먼저입니다. 3.16 C를 결정 2에서 빼기로 메인이 정하면 editor-15는 그 사실을 비고에 적고 두 항목으로 판정합니다.

### 3.3 editor-16 sticky scroll

#### (a) 상태
editor-16 · sticky scroll · **partial**(`MATRIX` 215줄: "원본 1순위인 CSS 문서 심볼(파서 필요)·TS 내장 공급 … 이 남아 partial").

#### (b) 원본 동작
`TS/features/editor/code-editor.tsx` 279줄 `updateOptions({ stickyScroll: { enabled: stickyScroll } })`. Monaco 고정 줄 모델은 기본 `outlineModel`이며 문서 심볼(outline) → folding 공급자 → 들여쓰기 순으로 대체합니다(`MON/editor/contrib/stickyScroll/browser/stickyScrollModelProvider.js`). 그래서 언어별 내장 문서 심볼이 있으면 그 이름이 고정 줄에 나옵니다. 내장 심볼 공급: CSS/SCSS(`MON/languages/features/css/cssMode.js`의 `DocumentSymbolAdapter`), HTML(`htmlMode.js`), TS/JS(`MON/languages/features/typescript/tsMode.js` 63~65줄 `OutlineAdapter`). 사용자 문자열은 설정 `settings.editorStickyScroll`(`LOC` ko 732줄)뿐입니다.

#### (c) native 현재 자산과 빠진 것
고정 줄 UI·정의 이동·외부 LSP 심볼·구문 접기·들여쓰기 대체, JSON 구조, CSS/SCSS 내장 접기(2순위 재료), HTML 내장 심볼이 연결되어 있습니다(`ED/sticky-model.rs`, `UI/editor-sticky-scroll.rs`, `APP/editor-symbols.rs` `builtin_sticky` 132줄, `APP/editor-json.rs`). 빠진 것: CSS 문서 심볼(현재 `build`의 CSS 분기가 `symbols: None`, `APP/editor-json.rs` 197~201줄), TS/JS 내장 심볼(E6).

#### (d) 구현 방법
새 고정 줄 구현은 없습니다. editor-57 C2가 CSS 심볼을, E6이 TS/JS 심볼을 `Models.symbols`로 공급하면 기존 경로(`editor_symbols.install_builtin` → `builtin_sticky`)가 그대로 씁니다. C2에서 `provides_symbols`(`APP/editor-json.rs` 56줄)에 css/scss를 넣는 것을 잊으면 `symbol_documents()`가 CSS 문서를 빼 `retain_builtin`이 설치한 심볼을 다음 tick에 지웁니다(`APP/domains/editor.rs` 2651~2652줄).

#### (e) 자동 검사
언어별 `[A]` 1건: 파일 열기 → 고정 줄 켬 → 20줄 스크롤 → 고정 줄 텍스트가 선택자(CSS)·클래스/함수(TS) 이름. HTML 선례 `APP/application-html-tests.rs`의 `actual_app의_html은_요소_심볼로_고정줄과_접기를_공급한다`를 복제합니다.

#### (f) 완료 판정
CSS `[A]` + E6(또는 결정 4가 정한 축소 범위) `[A]`. 결정 4가 TS 내장을 제외하면 비고에 그 결정 문구를 적고 CSS 근거로 complete.

#### (g) 막는 요소
editor-57 C1·C2(선행), E6(사용자 허락).

### 3.4 editor-32 formatOnType·formatOnPaste

#### (a) 상태
editor-32 · formatOnType·formatOnPaste · **partial**(`MATRIX` 231줄: "TS 붙여넣기/onType(E6)이 남아 partial").

#### (b) 원본 동작
`TS/features/editor/code-editor.tsx` 299~300줄(`formatOnType`, `formatOnPaste` 설정값). 붙여넣기 포맷은 범위 포맷 공급자를 쓰고, 입력 포맷은 onType 공급자의 트리거 글자에서 동작합니다. 내장 공급자 중 onType이 있는 것은 TS/JS뿐입니다(`MON/languages/features/typescript/tsMode.js` 87~89줄 `FormatOnTypeAdapter`, `languageFeatures.js` 755줄, 트리거 `;` `}` `\n`). CSS·HTML·JSON은 onType 공급자가 없습니다. 설정 문구 `settings.editorFormatOnType`·`settings.editorFormatOnPaste`(`LOC` ko 718·716줄).

#### (c) native 현재 자산과 빠진 것
외부 LSP 입력/붙여넣기 포맷, CSS/SCSS·HTML 붙여넣기 포맷이 실제 App에서 확인되었습니다(`QA/2026-11-02-native-lane-editor-css-services.md` 59줄, `…-html-services.md` 65줄). 빠진 것은 TS/JS 내장 붙여넣기·onType(E6)뿐입니다. 현재 앱은 내장 onType을 명시적으로 거절합니다(`APP/editor-formatting.rs` 472줄 `if native && matches!(kind, Kind::OnType { .. }) { return Ok(None); }`).

#### (d) 구현 방법
E6의 범위 포맷·onType 어댑터를 `APP/editor-formatting.rs`의 `is_native_eligible`(25줄)·`native_build`(31줄)에 ts/js 분기로 추가합니다. 472줄 거절 조건을 "TS/JS 내장 공급자이고 입력 글자가 `;`·`}`·`\n` 중 하나일 때만 허용"으로 바꿉니다. 붙여넣기는 기존 `Kind::Ranges` 경로를 그대로 씁니다.

#### (e) 자동 검사
`[A]` `.ts` 파일에서 `if(a){b` 뒤 `}` 입력 → onType 포맷 결과·undo 1회, 붙여넣기 포맷 1건, `.json`에서 `}` 입력은 포맷 없음(원본처럼 onType 공급자 없음).

#### (f) 완료 판정
TS `[A]` 통과(또는 결정 4 축소 시 그 문구로 판정).

#### (g) 막는 요소
E6 사용자 허락.

### 3.5 editor-34 Monaco 액션의 팔레트·키바인딩 노출

#### (a) 상태
editor-34 · Monaco 액션 팔레트·키바인딩 · **partial**(`MATRIX` 233줄).

#### (b) 원본 동작
- 등록: `TS/shared/lib/monaco/monaco-action-commands.ts` 39~46줄. `MONACO_ACTIONS`(`TS/shared/lib/monaco/monaco-actions.ts`) 각 항목이 명령 `monaco.<actionId>`가 되고 실행은 `requestEditorPaneCommand({ type: 'run-monaco-action', actionId })`(41줄, 포커스된 편집기의 action `trigger`). `MONACO_ACTION2_IDS` 21개(12~34줄)는 `getSupportedActions()`에 나타나지 않아 항상 활성, 나머지는 `context.activeEditorActionIds`에 있을 때만 활성(42줄).
- 남은 액션별 원본 동작:

| 액션 id | 원본 동작 | 원본 파일 |
| --- | --- | --- |
| `editor.action.fontZoomIn`/`Out`/`Reset` | JS realm(창) 전역 zoom 수준 ±1/0, 범위 −5..20, 글꼴 크기·줄 높이 × (1 + 0.1 × 수준), 저장 안 됨 | `MON/editor/contrib/fontZoom/browser/fontZoom.js`, `MON/editor/common/config/editorZoom.js`(`setZoomLevel`의 `Math.min(Math.max(-5, zoomLevel), 20)`), `MON/editor/common/config/fontInfo.js` 34~36줄 |
| `editor.action.gotoLine`(⌃G) | standalone quick input `:` 줄/열 이동 | `MON/editor/standalone/browser/quickAccess/standaloneGotoLineQuickAccess.js` |
| `editor.action.quickOutline`(⇧⌘O) | quick input `@` 심볼 | `.../standaloneGotoSymbolQuickAccess.js` |
| `editor.action.hideHover` | 열린 hover 닫기 | `MON/editor/contrib/hover/browser/hoverActions.js` |
| `editor.action.increase/decreaseHoverVerbosityLevel` | 공급자가 verbosity를 지원할 때만 단계 변경 | 같은 파일 |
| `editor.action.inPlaceReplace.up/down`(⇧⌘, / ⇧⌘.) | 한 줄 선택만 허용. 선택이 비면 커서 뒤 1글자 범위를 선택 텍스트로 씀. worker가 (선택 텍스트, 커서 위치 단어) 순으로 `BasicInplaceReplace`를 시도(숫자는 소수 자릿수 유지 ±1, 0에서 down은 무동작; 값 집합 `true/false`·`True/False`·`Private/Public/Friend/ReadOnly/Partial/Protected/WriteOnly`·`public/protected/private` 순환). **커서 위치에 단어가 없으면 선택 텍스트가 있어도 무동작**. 편집은 undo 단위 1개, 바뀐 범위에 `editorBracketMatch.border` 2px 외곽선 350ms | `MON/editor/contrib/inPlaceReplace/browser/inPlaceReplace.js` 55~100줄, `MON/editor/common/services/editorWebWorker.js` 257~279줄, `MON/editor/common/languages/supports/inplaceReplaceSupport.js` 전체, CSS `inPlaceReplace.css` 7~9줄 |
| `editor.action.clipboardCopyWithSyntaxHighlightingAction` | 선택(없으면 줄)을 구문 색 HTML + 텍스트로 복사 | `MON/editor/contrib/clipboard/browser/clipboard.js` |
| `editor.action.pasteAs`·`pasteAsText` | 붙여넣기 편집 공급자 선택 / 일반 텍스트 붙여넣기 | `MON/editor/contrib/dropOrPasteInto/browser/defaultProviders.js` 55~82·216~222줄 |
| `editor.action.toggleTabFocusMode`(⌃⇧M) | 창 전역 토글, 켜지면 Tab/⇧Tab이 포커스 이동 | `MON/editor/contrib/toggleTabFocusMode/browser/toggleTabFocusMode.js` |
| `editor.action.openLink` | 각 선택 끝 위치의 감지된 링크 열기 | `MON/editor/contrib/links/browser/links.js` 348~372줄 |
| `editor.action.unicodeHighlight.*`(4) | Unicode 강조 설정 변경·제외 옵션 | `MON/editor/contrib/unicodeHighlighter/browser/unicodeHighlighter.js` 463~620줄 |
| `editor.action.inlineSuggest.toggleAlwaysShowToolbar` | toolbar 항상 표시 토글(설정 메모리 값). Action2라 편집기 조건 없음 | `MON/editor/contrib/inlineCompletions/browser/controller/commands.js` 313~334줄 |
| `editor.action.showOrFocusStandaloneColorPicker` | 커서 위치 색(없으면 기본) 선택기 표시/포커스 | `MON/editor/contrib/colorPicker/browser/` |

- 사용자 문자열: 명령 제목은 `keymap.monaco.<actionId>` 키와 `titleDefaultValue`(`UI/keybinding-commands.json`에 이미 있음, 예: 383·460·502·530·537·600·747·768·831·971줄). Monaco 위젯 안 문구는 영어 고정입니다.

#### (c) native 현재 자산과 빠진 것
- 있는 것: 명령 메타·키맵·활성 조건(`UI/command-registry.rs` `enablement` 331~358줄, `EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE` 25~47줄은 원본 `MONACO_ACTION2_IDS`와 같은 21개). 실행은 `execution`(360~501줄)이 지원 범주만 `Execution::Native(Run::…)`로, 나머지는 `Execution::Unavailable`로 둡니다. 현재 명령 표 검사에는 일부 id의 `Unavailable`을 단언하는 것이 있습니다(2010줄 근처 `assert_eq!(command.execution, Execution::Unavailable, "{id}")`). inline suggest toolbar 안의 Always Show Toolbar 토글은 editor-77로 구현되었습니다(`UI/editor-inline-suggest-toolbar.rs` `is_always_shown` 68줄·`toggle_always_shown` 76줄, 창별 egui 메모리).
- 빠진 것(재확인): `grep -rn -i -E "fontZoom|zoom_level|tab_focus_mode|inPlaceReplace|pasteAs|hideHover|HoverVerbosity|gotoLine|quickOutline" APP UI ED`의 결과는 `EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE`의 `toggleTabFocusMode` 문자열 하나뿐입니다. 팔레트에는 `@`(심볼)·`:`(줄) 모드가 이미 있지만(`UI/command-palette-query.rs` 5~6줄 `SYMBOL_MODE_PREFIX`·`LINE_MODE_PREFIX`), `PaletteEntry`(`UI/command-registry.rs` 145~149줄)는 `Files`·`Commands`·`WorkspaceSymbols`뿐이라 명령으로 그 모드를 열 수 없습니다(`entry_query` `UI/command-palette-query.rs` 81~87줄). `ED/documentation.rs` `Command`(27~30줄)에는 `ShowHover`·`Signature`만 있고 hover를 닫는 명령이 없습니다.
- git 3개(`monaco.taide.gitStageSelection`·`toggleBlame`·`openFileHistory`)는 `GIT`에서 연결되었습니다(1.3 표).

#### (d) 구현 방법
1. **목록 고정 검사 먼저**: `UI/command-registry.rs` `mod tests`에 `registry().commands()` 중 `id.starts_with("monaco.")`이고 `execution == Execution::Unavailable`인 id를 모아 기대 목록과 `assert_eq!`하는 검사를 만듭니다. 처음에는 현재 실제 목록을 기대값으로 두고(실행해 얻은 목록을 QA에 기록), 아래 액션을 구현할 때마다 기대 목록에서 지웁니다. 마지막에는 결정으로 보류한 항목만 남아야 합니다. 이 검사가 바뀐 `execution`과 충돌하는 기존 `Unavailable` 단언(2010줄 근처)을 함께 고칩니다.
2. **fontZoom**: 창(viewport)별 zoom 수준 `HashMap<egui::ViewportId, i8>`(−5..=20)을 `domains::editor::State`에 둡니다. `Run`에 `EditorZoom(ZoomCommand)`를 추가하고 `execution`의 match에 3개 id를 연결, `APP/shell_keymap.rs` `intent`(89줄)에서 `ShellIntent::Editor(intents::editor::Intent::Zoom(..))`로 바꿔 `apply_editor_intent`가 수준을 바꿉니다. 표시: `show_document`의 `let editor = self.editor.with_indent(indent);`(5587줄 근처) 다음에 `editor.appearance.font.size`와 `editor.appearance.line_height`를 `× (1.0 + 0.1 × level)`로 바꾼 복사본을 씁니다(`EditorAppearance` literal 필드 추가 없음). 상태바 글꼴 크기(editor-84)와 별개이며 저장하지 않습니다.
3. **gotoLine·quickOutline(결정 6 A)**: `PaletteEntry`에 `Line`, `Symbol`을 추가하고 `entry_query`가 `LINE_MODE_PREFIX`·`SYMBOL_MODE_PREFIX`를 돌려주게 합니다. `execution`에서 `monaco.editor.action.gotoLine` → `Run::OpenPalette(PaletteEntry::Line)`, `monaco.editor.action.quickOutline` → `Run::OpenPalette(PaletteEntry::Symbol)`. `PaletteEntry`를 match하는 곳(`APP/command-palette.rs` 599~607줄, `APP/command-dispatch.rs` 677~679·1077줄, `UI/command-palette-query.rs` 180~182줄 검사)을 함께 고칩니다. `WEB`은 `PaletteEntry`를 쓰지 않습니다.
4. **hideHover**: `ED/documentation.rs` `Command`에 `HideHover`를 추가하고 `from_action("editor.action.hideHover")`가 그것을 돌려주게 합니다. 실행은 기존 `Run::EditDocument(DocumentEdit::Documentation(command))` 경로(`APP/domains/editor.rs` 5698줄에서 꺼냄)이고, UI 문서 위젯의 `command`(`UI/editor-documentation.rs` 197줄, `Command::ShowHover` 분기 211줄)에 `Command::HideHover` 분기를 추가해 `self.hover`를 비우고 `provider.close(view, Kind::Hover)`(trait 메서드 90줄)를 부릅니다. hover가 없으면 `Ok(false)`(무동작).
5. **hover verbosity**: 현재 공급자(LSP·JSON 내장) 모두 verbosity 미지원이므로 hover가 열렸을 때만 활성, 실행은 무동작(원본과 같은 결과). E6에서 TS 내장이 생기면 다시 확인합니다.
6. **inPlaceReplace**: `ED/inplace-replace.rs`(새 파일)에 `BasicInplaceReplace` 전체(숫자 규칙의 `Math.floor(n1 * precision)` 포함)를 이식합니다. 앱/UI 실행은 (b) 표의 worker 규칙을 따릅니다: 주 선택만, 여러 줄 선택이면 무동작, 빈 선택이면 커서 뒤 1글자, 커서 위치 단어는 그 언어의 word pattern(`LanguageRules::word_range`), 단어가 없으면 무동작. 편집은 기존 편집 트랜잭션(undo 단위 1개), readonly면 비활성. 바뀐 범위 강조는 350ms 수명의 장식(테두리이므로 2.3 (e)의 geometry 직접 그리기 방식, 색 `editorBracketMatch.border`)으로 그립니다.
7. **서식 있는 복사(결정 3 A)**: 기존 clipboard 쓰기 포트(`APP/host.rs`의 `write_clipboard`, shell `Command::CopyText` 경로 `APP/domains/shell.rs` 174~178줄)에 HTML 변형이 있는지 확인합니다. arboard의 HTML 쓰기 API 유무는 앱 `Cargo.lock`의 arboard 버전 문서로 확인하고(lane 규칙상 registry 소스 읽기는 허용, `docs/utils/2026-11-02-native-lane-workflow.js` 45줄), 없으면 텍스트만 복사하고 비고에 적습니다. HTML 형식은 원본 `clipboard.js`의 구문 색 직렬화를 따릅니다. 실제 clipboard 검사는 금지이므로 주입 포트로만 검사합니다.
8. **pasteAs·pasteAsText**: 주입 clipboard의 형식 목록으로 공급자 목록을 계산합니다. 텍스트만 있으면 "Insert Plain Text" 하나라 바로 적용합니다. `pasteAsText`는 붙여넣기 포맷(editor-32)을 건너뛴 일반 붙여넣기입니다.
9. **toggleTabFocusMode**: 창별 플래그(`HashMap<ViewportId, bool>`). 켜지면 편집기가 Tab/⇧Tab 키를 소비하지 않고 egui 포커스 이동으로 넘깁니다. 편집기 키 경로(`route_editor_keymap`, 본문 `body_input`)에서 분기합니다.
10. **openLink·unicodeHighlight 4개**: 3.16 B·A가 만든 상태의 명령으로 연결합니다.
11. **toggleAlwaysShowToolbar 팔레트 실행**: `Run`에 새 변형을 두고 `APP/shell_keymap.rs` `intent`에서 현재 창 viewport의 egui 메모리 값을 `taide_native_ui::editor_inline_suggest_toolbar::toggle_always_shown(context)`로 뒤집는 intent를 만듭니다(`QA/2026-11-02-native-lane-editor-display-commands.md` 59줄).
12. **showOrFocusStandaloneColorPicker**: 3.16 D 뒤.
13. Option 조합 키(⌥ 계열)를 새로 받으면 뒤따르는 `Event::Text`도 소비해야 합니다(4절 함정 12).

#### (e) 자동 검사
1. `[C]` UI `command-registry` 목록 고정 검사(1번).
2. `[C]` `EDT/inplace-replace.rs`: 숫자 증감(`1.5`→`1.6`, `0` down 무동작), 값 집합 순환(양방향·끝에서 처음), 선택 있음/없음.
3. `[U]` 기존 `UIT` 하네스에 zoom 배율 적용 뒤 줄 높이, Tab focus mode에서 Tab이 문서를 바꾸지 않음.
4. `[A]` 팔레트(`run_palette`)와 재지정 키로 각 1건: zoom 두 번 → 줄 높이 × 1.2, gotoLine → 팔레트 쿼리 `:`·숫자 입력 후 Enter → 커서 줄, quickOutline → `@`, inPlaceReplace → 문서 변경·undo 1회 복원, readonly 문서에서 inPlaceReplace 비활성(`is_runnable` 거짓), tabFocusMode → Tab이 포커스 이동, hideHover → 열린 hover 닫힘.
5. 실행: 0.3의 2·5·6번, fmt, `PaletteEntry`·`ED/documentation.rs` `Command`를 바꿨으므로 동결 미러.

#### (f) 완료 판정
목록 고정 검사의 남은 `Unavailable`이 결정으로 보류한 항목(예: arboard HTML 미지원 시의 서식 복사 HTML 부분)뿐이고 각 액션 `[A]` 통과. git 3개는 git lane 근거로 함께 닫습니다.

#### (g) 막는 요소
openLink·unicodeHighlight는 3.16 B·A, 색 선택기는 3.16 D가 먼저입니다. 실제 clipboard 검사는 금지라 주입 포트로만 검사합니다.

### 3.6 editor-49 포맷

#### (a) 상태
editor-49 · 포맷 · **partial**(`MATRIX` 248줄: "`<style>` 안 CSS 줄이 120열을 넘어도 native는 감싸지 않습니다 … TS(E6) 포맷이 남아 partial").

#### (b) 원본 동작
`TS/shared/lib/lsp/adapters/formatting.ts` 14·34줄, `TS/widgets/editor-pane/editor-pane.tsx` 141줄(저장 포맷). 내장: JSON·CSS/SCSS·HTML 문서/범위, TS/JS는 범위 공급자만 있어(`tsMode.js` 79~81줄 `registerDocumentRangeFormattingEditProvider`) 문서 포맷도 전체 범위로 범위 공급자를 씁니다. HTML 포맷은 `<style>` 내용을 beautify-css로 넘길 때 HTML의 `wrap_line_length` 120을 전달합니다(`MON/languages/features/html/register.js` 31~44줄 옵션, `MEXT/vscode-html-languageservice/lib/esm/beautify/beautify-html.js`의 css custom beautifier 호출).

#### (c) native 현재 자산과 빠진 것
외부 LSP·JSON·CSS/SCSS·HTML 포맷이 연결되어 있습니다. 빠진 것: TS/JS(E6), `<style>` 안 CSS 줄 120열 감싸기(editor-57 C8; 현재 `ED/css-format.rs`는 Monaco CSS 옵션의 감싸기 0 경로만 이식, `QA/2026-11-02-native-lane-editor-css-services.md` 33줄).

#### (d) 구현 방법
editor-57 C8과 E6. TS 포맷 옵션은 원본 `FormatHelper._convertOptions`(`MON/languages/features/typescript/languageFeatures.js` 697~720줄) 값을 그대로 씁니다.

#### (e) 자동 검사
`[C]` C8의 감싸기, `[A]` TS 문서·선택·저장 포맷 각 1건.

#### (f) 완료 판정
C8 + TS(또는 결정 4 축소 문구).

#### (g) 막는 요소
E6 사용자 허락.

### 3.7 editor-57 Monaco 내장 언어 서비스(JSON·CSS·HTML·TS)

#### (a) 상태
editor-57 · Monaco 내장 언어 서비스 · **partial**(`MATRIX` 256줄).

#### (b) 원본 동작(공통)
- 활성: Monaco 내장 모드는 언어 ID 첫 사용 때 등록됩니다. TAIDE 언어 ID(`TS/shared/lib/shiki/lang-map.ts` 3~35줄) 중 내장 서비스가 붙는 것은 `json`, `css`, `scss`, `html`, `typescript`, `javascript`뿐입니다. `typescriptreact`·`javascriptreact`에는 내장 worker가 붙지 않습니다(`TS/shared/lib/monaco/builtin-typescript-mode.ts`의 `DEFAULTS_KEY_BY_LANGUAGE_ID` 주석).
- 공급자 목록: JSON(`MON/languages/features/json/jsonMode.js` 44~115줄: 문서/범위 포맷, 완성 트리거 `" "`·`":"`·`'"'`, hover, 심볼, 색, 접기, 선택 확장, 진단), CSS/SCSS(`MON/languages/features/css/cssMode.js` 14~113줄: 완성 트리거 `/ - :`, hover, 하이라이트, 정의, 참조, 심볼, rename, 색, 접기, 진단, 선택 확장, 문서/범위 포맷, 기본 lint 심각도 `css/register.js` 37~59줄), HTML(`html/htmlMode.js` `registerProviders`: 완성 트리거 `. : < " = /`, hover, 하이라이트, 링크, 심볼, rename, 접기, 선택 확장, html만 문서/범위 포맷, **진단·색 없음**), TS/JS(`typescript/tsMode.js` 30~107줄: 완성 트리거 `.`, signature `(` `,`, hover, 하이라이트, 정의, 참조, 심볼, rename, 범위 포맷, onType `;` `}` `\n`, code action, inlay hint, 진단; **접기 공급자 없음**).
- 외부 공급자와 공존(`SPEC` 2.8): CSS/HTML/JSON 내장은 외부 세션이 있어도 정지하지 않고, 단일 결과(포맷)는 외부 우선, 병합 기능(완성·hover·심볼·하이라이트)은 둘 다, 진단은 소유자별로 둘 다. TS/JS 내장은 외부 세션이 typescript·typescriptreact(또는 javascript·javascriptreact)를 소유하는 동안 전체 정지(`builtin-typescript-mode.ts` `suspendBuiltinTypeScriptMode`, 서비스 단위 참조 계수). 원본은 0.56에서 이미 등록된 공급자를 해제하지 못하는 한계가 있으나(같은 파일 `suspendBuiltinTypeScriptMode` 주석) native는 이 한계를 재현하지 않고 세션 동안 모두 제외합니다.
- TS/JS 진단은 syntax만(`TS/shared/lib/monaco/builtin-typescript.ts` `BUILTIN_TYPESCRIPT_FALLBACK_DIAGNOSTICS_OPTIONS`: `noSemanticValidation: true`, `noSyntaxValidation: false`, `noSuggestionDiagnostics: true`). 세션이 소유하는 동안은 syntax 진단까지 끔(`BUILTIN_TYPESCRIPT_SUSPENDED_DIAGNOSTICS_OPTIONS`).
- 사용자 문자열: Monaco·language service 메시지는 영어 고정(원본이 nls·l10n을 설정하지 않음).

#### (c) native 현재 자산과 빠진 것
완료된 것(재작업 금지): JSON scanner·AST·심볼·접기·선택 확장·포맷·구문/schema 진단·완성 core와 앱 공급(`APP/editor-completion-json.rs`), JSON hover core(`ED/json-hover.rs` `hover` 113줄, `Hover { contents: String, range: Range<usize> }` 26줄), CSS/SCSS scanner·접기·포맷(문서·선택·붙여넣기·저장)의 앱 공급, Monaco 기본 색 공급자 계산과 색 형식(`ED/default-colors.rs` `document_colors` 271줄·`ED/color.rs`, 앱·화면 연결 없음), HTML scanner·parser·심볼·접기·선택 확장·포맷의 앱 공급. 근거: `QA/2026-11-02-native-lane-editor-json-services.md`, `…-css-services.md`, `…-html-services.md`.

"언어 선택 UI"는 원본에 없어 범위에서 뺐습니다(`SPEC` 1.3, `QA/…-json-services.md` 10줄).

남은 하위 묶음(이 절의 (d)~(f)는 하위 묶음별로 씁니다):

| 하위 | 내용 | native 현재 | 원본 근거 |
| --- | --- | --- | --- |
| J1 | JSON hover 앱 소비 | `APP/editor-documentation.rs` `Provider::available`(635줄)·`version`(651줄)·`request`(711줄)가 `lsp.documentation_providers`만 봐서 LSP가 없으면 hover 요청이 없음 | `MEXT/vscode-json-languageservice/lib/esm/services/jsonHover.js` |
| J2 | JSON 색(`findDocumentColors`, schema `format: color`) | 없음 | `jsonDocumentSymbols.js` `findDocumentColors` |
| J3 | JSON 전용 `[A]` 보강(untitled·peek·readonly·Large·외부 공존), oracle | 실제 App 검사는 파일 1건 | `QA/…-json-services.md` 64~70줄 |
| C1 | CSS 파서·노드 | 없음 | `MEXT/vscode-css-languageservice/lib/esm/parser/cssParser.js`(1,794줄)·`scssParser.js`(809줄)·`cssNodes.js` |
| C2 | CSS 문서 심볼(→ editor-16) | `build`의 CSS 분기가 `symbols: None`(`APP/editor-json.rs` 197~201줄) | `services/cssNavigation.js` |
| C3 | CSS 선택 확장 | 없음 | `services/cssSelectionRange.js` |
| C4 | CSS 하이라이트·정의·참조·rename | 없음, 앱에 내장 하이라이트·위치·rename 공급 경로도 없음 | `cssNavigation.js`, `scssNavigation.js` |
| C5 | CSS 진단(파서 오류 + lint) | 없음 | `services/lint.js`·`lintRules.js`, 심각도 `MON/languages/features/css/register.js` 37~59줄 |
| C6 | CSS 색과 색 표현 | 없음(기본 공급자 core만) | `cssNavigation.js` `findDocumentColors`·`getColorPresentations` |
| C7 | CSS 완성·hover | 없음 | `cssCompletion.js`·`scssCompletion.js`·`cssHover.js`·`selectorPrinting.js`·`data/webCustomData.js` |
| C8 | `<style>` 안 CSS의 120열 감싸기 | 없음(editor-49 비고의 차이) | `beautify-css.js`의 `wrap_line_length`(HTML 옵션 120이 전달됨) |
| H1 | HTML 하이라이트(시작·끝 태그 이름 쌍) | 없음 | `MEXT/vscode-html-languageservice/lib/esm/services/htmlHighlighting.js` |
| H2 | HTML 링크 | 없음 | `htmlLinks.js` — 3.16 B의 링크 UI와 공유 |
| H3 | HTML rename(태그 쌍) | 없음 | `htmlRename.js` |
| H4 | HTML 완성·hover | 없음 | `htmlCompletion.js`·`htmlHover.js`·`languageFacts/data/webCustomData.js` |
| E6 | TS/JS 전체 | 없음 | 아래 E6 절 |

#### (d) 구현 방법 — 공통 규칙
1. 각 언어 서비스는 portable core(`ED`, wasm32 컴파일)와 앱 공급(`APP/editor-css.rs`·`editor-html.rs`)으로 나눕니다.
2. 앱 공급은 `APP/editor-json.rs`의 문서 작업 규칙을 따릅니다: 별도 worker, revision이 바뀌면 이전 결과 거절, Normal 등급만, readonly(포맷 등 쓰기 기능만 거절), 닫힘·취소·종료 시 회수, AppFile과(git lane 병합 뒤) `Virtual` 제외.
3. 원본 위치는 UTF-16 offset이므로 byte↔UTF-16 변환과 입력·출력·작업 예산을 둡니다(선례: JSON 서비스들의 `Limits`).
4. 원본 결함은 oracle에 원본 값을 남기고 교정 기대값을 따로 둡니다(JSON 선례).
5. 새 crate 의존성 없음(결정 4: CSS/SCSS·HTML 순수 Rust 이식, `QA/…-css-services.md` 4줄).

#### (d)~(f) 하위 묶음별

**J1 JSON hover 앱 소비**
1. `APP/editor-documentation.rs`: JSON 전용 `ProviderIdentity`를 `APP/editor-completion-json.rs` `provider()`(40줄, `crate::diagnostics::Owner::new()`로 한 번 만든 고정 identity)와 같은 방식으로 만들고, 대상 판정은 `editor_completion_json::applies`(49줄)와 같은 조건(언어 `json`·Normal·AppFile 아님, 병합 뒤 `Virtual` 아님)으로 둡니다.
2. `Entry`(175줄)에 내장 대기 결과(`oneshot::Receiver<Result<Option<Hover>, EditorError>>`)를 추가하고 worker(`spawn_blocking_transient_handle("native-json-hover", …)`)에서 `json_hover::hover(&text, byte, environment, LIMITS)`를 돌립니다. `Environment`는 진단과 같은 schema 설정을 씁니다(JSON 완성이 `editor-json-validation.rs`의 상수·함수를 `pub(crate)`로 재사용한 것과 같은 방식, `QA/…-json-services.md` 30줄).
3. `available`·`version`·`request`가 "LSP 공급자 집합 ∪ {JSON 내장}"을 보게 합니다. `complete`는 LSP 완료 ∧ 내장 완료입니다.
4. 결과를 `HoverGroup { provider, part }`(88줄)로 넣습니다. ordinal은 LSP 뒤(원본에서 나중에 등록된 LSP가 앞)입니다. `contents`는 markdown 문자열 1개, `range`는 byte입니다.
5. `[A]` `APP/application-json-tests.rs`에 `$schema` 있는 파일의 키 위 hover → 문서 문구 표시, LSP mock 공존 시 두 그룹.

**J2 JSON 색**: C6과 3.16 D(색 UI)가 생긴 뒤 같은 공급 구조로 연결합니다. JSON 문서는 내장 색 공급자가 있으므로 기본 색 공급자를 쓰지 않습니다(`MON/editor/contrib/colorPicker/browser/color.js` 13~60줄: 다른 공급자가 있으면 기본 공급자 제외).

**J3 JSON 보강**: `[A]` untitled JSON 탭 완성, peek 안 JSON 완성, readonly에서 완성은 되고 포맷은 거절, Large 등급 제외, LSP mock과 공존. oracle은 bun 실행이 허락된 작업자가 `json-completion-oracle.mjs`·`json-hover-oracle.mjs`로 생성합니다(위험: patternProperties 정규식, tuple items, defaultSnippets 배열 깊이, propertyNames enum 설명이 손 계산 밖, `QA/…-json-services.md` 68줄).

**C1 CSS 파서**
1. `ED/css-parser.rs`·`ED/css-nodes.rs`(새 파일): `cssParser.js`·`scssParser.js`·`cssNodes.js`를 이식합니다. 기존 `ED/css-scanner.rs`(`Scanner::new(source, Dialect)`)를 씁니다. 노드는 arena(`Vec<Node>` + 부모·자식 index)로 두고, 원본의 노드 종류(`NodeType`)·오류 표식(`ParseError`, `Marker`)을 옮깁니다. 재귀는 명시 스택으로 바꾸고 깊이·노드·토큰 상한을 둡니다(HTML 파서 선례 `ED/html-parser.rs`).
2. `[C]` `EDT/css-parser.rs`: 규칙·선언·at-rule·SCSS 중첩·mixin·변수·보간 `#{}`·잘못된 입력 복구 위치. oracle은 bun 허락 시 `parseStylesheet` 결과의 노드 종류·offset 직렬화로 생성합니다.

**C2 CSS 문서 심볼(editor-16의 1순위 재료)**
1. `ED/css-symbols.rs`: Monaco CSS 워커(`MON/languages/features/css/cssWorker.js`)가 부르는 심볼 함수를 확인해 같은 것을 이식합니다(계층/평면 여부도 워커 호출로 결정).
2. `APP/editor-json.rs` `build`의 CSS 분기에서 `Models { symbols: Some(..), folds }`로 바꾸고 `provides_symbols`(56줄)에 css/scss를 추가합니다.
3. `[A]` `css_tests`에 고정 줄: 20줄 넘는 규칙 안으로 스크롤 → 고정 줄에 선택자 이름.

**C3 CSS 선택 확장**: `ED/css-selection.rs`로 `cssSelectionRange.js` 이식, `APP/editor-selection-ranges.rs`의 `is_builtin`(165줄)·`builtin_ranges`(169줄)에 css/scss 분기를 추가합니다(HTML 선례). `[A]` 스마트 선택 확장/축소.

**C4 CSS 탐색**: 하이라이트·정의·참조·rename은 앱에 "내장 공급자" 경로가 없습니다(JSON에도 없음).
1. 하이라이트: `APP/editor-highlights.rs`의 `Consumer::submit`(821줄)은 LSP 공급자가 없으면 단어 기준 텍스트 공급자 `textual`(289줄)로 대체합니다. 원본 Monaco는 언어 지정 공급자(내장)가 `*` 수준보다 앞이므로 css/scss/html이면 worker에서 내장 결과를 먼저 쓰도록 분기를 추가합니다. 구현 전에 `MON/editor/contrib/wordHighlighter/browser/wordHighlighter.js`의 공급자 그룹 처리를 읽어 "내장이 빈 배열을 주면 텍스트로 넘어가는지"를 확인하고 그대로 따릅니다.
2. 정의·참조: `APP/symbol-location-host.rs`·`editor-locations.rs`의 위치 공급 경로에 내장 결과를 합칩니다(같은 문서 안 위치만 나옴).
3. rename: `APP/editor-rename-controller.rs`에 내장 공급을 추가합니다(prepare 여부는 원본 `cssNavigation.js`를 확인해 따름).
4. `[C]` core 각 함수, `[A]` 변수 위 F2 → 같은 문서의 모든 참조 변경·undo 1회.

**C5 CSS 진단**: `ED/css-lint.rs`로 `lint.js`·`lintRules.js`를 이식하고 기본 심각도 표(`css/register.js` 37~59줄)를 상수로 둡니다. 파서 오류도 진단에 넣습니다(`validate: true`). 앱은 `APP/editor-json-validation.rs`처럼 500ms 지연 worker로 계산하고 `self.lsp_diagnostics.publish_builtin(&snapshot, markers, title)`로 게시합니다. 주의: `retain_builtin(&self.store, self.editor_json_validation.documents())`(`APP/domains/editor.rs` 2611~2612줄)는 넘긴 집합 밖의 내장 진단을 지우므로 CSS 검증 문서를 합친 집합을 넘깁니다(4절 함정 5). `[A]` 빈 규칙 경고·알 수 없는 속성 경고·`#ff` 길이 오류가 Problems·밑줄에 나타남.

**C6 CSS 색**: `findDocumentColors`·`getColorPresentations`를 `ED/css-colors.rs`로 이식합니다(`ED/color.rs`의 `Rgba`·`format_*` 재사용). 표시는 3.16 D.

**C7 CSS 완성·hover**
1. 자료 표 생성: bun 스크립트(`docs/utils/<날짜>-css-web-data.mjs`)로 `MEXT/vscode-css-languageservice/lib/esm/data/webCustomData.js`를 읽어 `ED/web-data/css-data.rs`(속성·at-rule·의사 클래스/요소의 이름·설명·restrictions·values·browsers)를 생성합니다. MIT 출처를 파일 머리에 넣고 `<ROOT>/THIRD_PARTY_LICENSES.md`를 확인합니다. 실행은 `bun <절대 경로>` 한 줄이며 lane workflow는 bun을 허용하지 않으므로 작업 지시에서 허용되는지 먼저 확인합니다.
2. `ED/css-completion.rs`·`ED/css-hover.rs` 이식, 완성 트리거 `/ - :`. 앱 공급은 JSON 완성 공급(`APP/editor-completion-json.rs`: 별도 worker·provider group으로 외부 LSP와 병합)을 복제해 `APP/editor-completion-css.rs`로 둡니다. hover는 J1 구조를 재사용합니다.
3. `[C]` 속성 이름·값 완성, `[A]` `.a{col` 뒤 Trigger → `color` 후보·수락.

**C8 `<style>` 120열**: `ED/css-format.rs`에 `wrap_line_length` 옵션 경로를 추가 이식하고 HTML 포맷이 `<style>` 내용을 넘길 때 120을 전달합니다(`ED/html-format.rs`의 css custom beautifier 호출 지점, `css_format::beautify`는 이미 `pub(crate)`). `[C]` 120열 넘는 선언 줄이 감싸짐, CSS 파일 자체 포맷은 여전히 감싸지 않음.

**H1~H3 HTML 하이라이트·링크·rename**: core는 `ED/html-parser.rs`의 `find_node_at`과 scanner로 이식합니다(`ED/html-highlighting.rs`, `ED/html-links.rs`, `ED/html-rename.rs`). 앱 공급 경로는 C4와 같은 구조를 공유합니다. 링크는 3.16 B의 링크 표시·열기에 공급자로 합칩니다(원본 `getLinks`는 모든 링크 공급자 결과를 합침). `[A]` `<div>` 시작 태그 위 커서 → 끝 태그 이름도 하이라이트, F2로 `div`→`section` 바꾸면 두 태그 모두 변경.

**H4 HTML 완성·hover**: C7과 같은 자료 생성(`languageFacts/data/webCustomData.js` → `ED/web-data/html-data.rs`). 자료 없이 되는 닫는 태그 완성(`</` → 열린 태그)도 같은 파일입니다. 트리거 `. : < " = /`.

#### E6 TS/JS 내장 서비스 — 방식 비교(사용자 허락 필요)

**원본이 하는 일.** Monaco가 `MON/languages/features/typescript/lib/typescriptServices.js`(9,075,748 byte, TypeScript 5.9.3 — `typescriptServicesMetadata.js` `typescriptVersion = "5.9.3"`)와 `lib/lib.js`(3,213,598 byte, `libFileMap["lib.d.ts"] = '…'` 형식의 lib 묶음)를 web worker에서 실행합니다. 두 파일 모두 ES 모듈(끝에 `export { … }`)입니다. worker의 LanguageServiceHost(`MON/languages/features/typescript/tsWorker.js` 17~135줄: `getCompilationSettings` 27, `getScriptFileNames` 36, `getScriptVersion` 51, `getScriptSnapshot` 82, `getDefaultLibFileName` 111)는 동기화된 모든 모델과 extraLibs를 script로 봅니다. 컴파일 옵션은 TS `{ allowNonTsExtensions: true, target: 99 }`, JS는 `allowJs: true` 추가(`register.js` 201·208줄), 기본 lib는 `lib.esnext.full.d.ts`입니다. 결과 변환은 `MON/languages/features/typescript/languageFeatures.js`의 어댑터(`DiagnosticsAdapter` 106, `SuggestAdapter` 297, `SignatureHelpAdapter` 427, `QuickInfoAdapter` 502, `DocumentHighlightAdapter` 530, `DefinitionAdapter` 554, `ReferenceAdapter` 589, `OutlineAdapter` 624, `FormatHelper._convertOptions` 698, `FormatAdapter` 724, `FormatOnTypeAdapter` 755, `CodeActionAdaptor` 778, `RenameAdapter` 840, `InlayHintsAdapter` 897줄)가 합니다. 서버 프로세스나 설치가 필요 없습니다.

**native 현재.** 외부 LSP `vtsls`가 설치되어 있으면 TS/JS는 이미 LSP 경로로 동작합니다(`CR/taide-lsp/resources/lsp-servers.json` 3~17줄: `languageIds` typescript·typescriptreact·javascript·javascriptreact, 설치 안내 `npm install -g @vtsls/language-server`). vtsls가 없는 사용자는 TS/JS 완성·hover·진단·포맷이 없습니다. 원본은 vtsls가 없어도 내장으로 동작합니다. 앱 `Cargo.lock`에는 JS 엔진 crate가 없습니다(`javascriptcore-rs` 2946줄은 wry의 Linux(WebKitGTK) 전용 의존으로 macOS에서 컴파일되지 않음, `APP/../vendor/wry-preview/Cargo.toml` 215줄 `[target.'cfg(any(target_os = "linux", …))'.dependencies.javascriptcore-rs]`).

**허락 상태.** `DEC` "보류" 표 첫 행: "editor-57 TS/JS 내장 서비스의 임베디드 JS 엔진 — 새 대형 의존성". 사양 결정 4의 추천은 "TS/JS = 임베디드 JS 엔진(허락 뒤)"입니다(`SPEC` 412~414줄).

| 방식 | 원본 동등성 | 새 의존·허락 | 구현 방법 요약 | 위험 |
| --- | --- | --- | --- | --- |
| A1 임베디드 QuickJS(C 소스를 빌드하는 binding crate, 예: `rquickjs`) | 원본 서비스 그대로 실행 → checker 기능 포함 동일 | 새 crate와 빌드 시 C 컴파일, JS 자산 약 12MB 반입(TypeScript Apache-2.0·Monaco MIT 고지) | 공통 A 절차 | 인터프리터라 V8 대비 느림(큰 파일 완성 지연), 시작 때 9MB 해석 비용(바이트코드 사전 컴파일 기능이 있으면 완화). crate 버전·라이선스·빌드 방식은 spike에서 crate 문서로 확인 |
| A2 임베디드 Boa(순수 Rust 엔진 crate) | 엔진이 TS 번들을 실행할 수 있으면 동일 | 새 대형 crate | 공통 A | ECMAScript 적합성·성능이 TS 번들 실행에 충분한지 불확실 → spike로 먼저 확인 |
| A3 V8 binding(`v8`/`deno_core` 계열) | 동일, JIT로 가장 빠름 | 새 crate. 기본 빌드가 사전 빌드 정적 라이브러리를 내려받는 방식이면 `--offline` 규칙과 충돌(저장소 밖 공급 필요, spike에서 확인) | 공통 A | 빌드 재현성·바이너리 크기 수십 MB 증가 |
| A4 macOS `JavaScriptCore.framework`(시스템 framework, C API FFI) | 동일, JIT | crate 추가 없이 `#[link(name = "JavaScriptCore", kind = "framework")]`와 `extern "C"` 선언(`JSGlobalContextCreate`, `JSEvaluateScript`, `JSStringCreateWithUTF8CString`, `JSObjectMakeFunctionWithCallback`, `JSValueToStringCopy` 등). 시스템 framework 링크라 역시 "임베디드 JS 엔진" 허락 대상 | 공통 A, 엔진 계층만 FFI | macOS 전용(원본 출시 대상이 macOS뿐이라는 preview-38 결정 10 A와는 맞음), `unsafe` FFI 코드량, 전용 스레드에서 컨텍스트 하나 |
| A5 숨은 WKWebView에서 원본 worker 실행 | 동일 | 새 crate 없음(vendored wry). 단 페이지 JS를 켠 WebView를 하나 더 둠 | 숨은 child WebView에 `ts.worker.js`와 번들을 적재하고 `evaluate_script_with_callback`(`APP/../vendor/wry-preview/src/lib.rs` 2015줄, 결과는 JSON 직렬화 문자열로 콜백)으로 요청/응답 | 부모 창(view) 필요, 콜백이 주 스레드, 미리보기 WebView의 페이지 JS 차단 정책(`vendor/wry-preview/src/wkwebview/mod.rs` 349~355줄 `setAllowsContentJavaScript(false)`)과 다른 보안 경계, 수명·크래시 처리 복잡. 보안 결정이 따로 필요 |
| B 순수 Rust(파서 기반만) | 부분: syntax 진단·심볼·포맷·onType만. 완성·hover·정의·참조·rename·code action·inlay·signature·하이라이트는 checker가 필요해 원본과 같아질 수 없음 | TS 파서를 직접 이식하면 의존 없음(수만 줄). 기존 Rust 파서 crate(oxc·swc·biome 계열)를 쓰면 새 의존 허락 필요, 오류 메시지·코드가 TS와 다름 | 파서 → `getNavigationTree` 규칙 이식(심볼), TS formatting 규칙 이식(포맷·onType) | 포맷 결과를 원본과 맞추려면 TS formatter 규칙 전체 이식 필요 |
| C 외부 tsserver/LSP만 | vtsls가 있으면 LSP 기준 동작, 없으면 기능 없음 | 없음(이미 있음) | 구현 없음. editor-57 비고에 "TS/JS 내장 미제공, vtsls 경로로 대체"를 적고 판정 | 원본 동작(설치 없이 동작)과 다름. Node 번들을 앱이 실행하는 변형은 Node 설치 의존이라 원본보다 요구가 큼 |

**공통 A 절차(엔진 선택과 무관)**
1. **spike(허락 직후 1회)**: 후보 엔진으로 `typescriptServices.js`를 적재해 (1) 바이너리 증가량, (2) 차가운 시작 시간(번들 해석), (3) 메모리, (4) 2,000줄 `.ts` 파일의 `getCompletionsAtPosition` 지연, (5) `lib.esnext.full.d.ts`와 그 참조 lib 적재 시간을 잽니다. ES 모듈 형식(끝의 `export {}`)을 엔진이 모듈로 적재할 수 있는지, 없으면 끝 줄을 전역 대입으로 바꾸는 생성 단계가 필요한지 확인합니다. 결과를 `docs/research/<날짜>-native-ts-engine-spike.md`에 적고 엔진을 정합니다.
2. **자산 반입**: `typescriptServices.js`와 `lib.js`(또는 필요한 lib 파일만)를 `native/taide-native-app/resources/typescript/`로 복사하고 라이선스(TypeScript Apache-2.0, Monaco MIT)를 `<ROOT>/THIRD_PARTY_LICENSES.md`에 추가합니다. 저장소 루트 prettier 대상에서 빼야 하면 `.prettierignore`에 경로를 추가합니다.
3. **JS 호스트 shim**: `tsWorker.js`의 `TypeScriptWorker`를 거의 그대로 옮긴 작은 JS 파일(같은 MIT 고지)을 두고, Rust가 문서 열기·변경·닫기를 `updateModel(uri, version, text)`로 알리면 shim이 `getScriptFileNames`·`getScriptVersion`·`getScriptSnapshot`에 반영합니다. 컴파일 옵션은 원본 값 그대로, 진단은 syntax만(`getSyntacticDiagnostics`).
4. **엔진 스레드**: 엔진 컨텍스트 하나를 전용 스레드에 두고 `std::sync::mpsc` 요청 큐로 감쌉니다(`services.tasks`가 감독). 요청은 문서 revision·세대를 들고 가며, 응답 시 현재와 다르면 버립니다. 취소는 세대 증가로 처리합니다(엔진 안 실행 중단은 엔진별 interrupt 기능이 있으면 사용).
5. **어댑터 이식**: `languageFeatures.js`의 각 Adapter를 `APP/editor-builtin-typescript.rs`로 옮겨 native 타입(완성 `Candidates`, hover markdown, `DocumentHighlight`, 위치, `DocumentSymbols`, `WorkspaceEdit`, 포맷 `TextEdit`, code action, inlay hint, 진단 marker)으로 바꿉니다. 포맷 옵션은 `FormatHelper._convertOptions` 값 그대로.
6. **공급 경로**: 완성은 JSON 내장처럼 별도 provider group, hover는 J1 구조, 포맷·onType은 `APP/editor-formatting.rs` `native_build`에 ts/js 분기와 472줄 거절 조건 완화(3.4), 심볼은 `editor_json::build`와 같은 구조 공급(sticky·`@` 팔레트), 진단은 C5처럼 `publish_builtin`.
7. **정지 정책**: LSP 세션이 `typescript`/`typescriptreact`를 소유하면 TypeScript 서비스 전체, `javascript`/`javascriptreact`를 소유하면 JavaScript 서비스 전체를 제외합니다(원본 `DEFAULTS_KEY_BY_LANGUAGE_ID` 대응, 참조 계수는 `APP/lsp.rs`의 세션 목록에서 매 tick 계산). 원본의 "이미 등록된 공급자 해제 불가" 한계는 재현하지 않습니다.
8. **원본 결함 비재현**: 정의가 `file:///lib.*.d.ts`로 가면 원본은 존재하지 않는 경로 열기 오류가 납니다(`SPEC` 2.9, `QA/2026-11-02-native-lane-editor-display-commands.md` 26줄). native는 lib 파일 정의를 열지 않거나 읽기 전용 가상 문서로 엽니다(작은 결정, QA에 기록).

**추천**: 사양 결정 4의 추천(TS/JS = 임베디드 JS 엔진)을 유지합니다. 원본 동등성과 성능을 함께 보면 A4(JavaScriptCore, 새 crate 없음·JIT·macOS 전용 출시와 일치)와 A1(QuickJS, 이식성·단순성)을 spike로 비교하는 것을 권장합니다. 사용자가 새 의존·플랫폼 framework 링크를 모두 거절하면 B′(순수 Rust 범위로 축소하고 checker 기능 부재를 비고에 적은 뒤 판정) 또는 C 중 하나를 다시 묻습니다(`HO/30-pending-permissions.md`의 질문 형식을 따름).

#### (e) 자동 검사(공통)
기능마다 `[C]`(가능하면 oracle), 언어마다 `[A]`: 파일·untitled·peek, 외부 공급자 공존, readonly, Large 제외, 닫힘 취소, 종료 뒤 작업 0. 실행: 0.3의 2·3·6번(필터 `json_tests css_tests html_tests editor_json editor_formatting editor_selection_ranges editor_documentation`), 동결 미러(ED 타입 변경 시), fmt.

#### (f) 완료 판정
J1~J3, C1~C8, H1~H4, E6(또는 결정 4가 정한 축소 범위)이 모두 닫히면 editor-57 complete. 같은 근거로 editor-16·32·49를 닫습니다.

#### (g) 막는 요소
- E6: 새 의존·플랫폼 framework 사용자 허락(`DEC` 보류 표 첫 행). 허락 전에는 E6을 구현하지 않습니다.
- C7·H4: bun 실행 허락(자료 표 생성). 허락이 없으면 그 하위 묶음만 보류하고 나머지를 진행합니다.

### 3.8 editor-72 Markdown 미리보기 분할

#### (a) 상태
editor-72 · Markdown 미리보기 분할 · **partial**(`MATRIX` 271줄: "결정 7 A 목록의 남은 것: `br` 줄 배치 검증, 혼합 inline code 줄 배치, 중첩 code, 문단 안 이미지 경계, raw `input` 기본 모양(비활성), 첫 블록 여백, `details`/`summary` 접힘, `a name`/`id` 앵커 이동, 표·목록 AccessKit 역할").

#### (b) 원본 동작
- 분할·토글: `TS/widgets/editor-pane/editor-pane.tsx` 494~510·548~564줄(`showPreviewPanel = isMarkdown && showMarkdownPreview` 494줄, `<MarkdownPreview html={renderMarkdownToSafeHtml(...)} />` 505줄), 명령 `editor.toggleMarkdownPreview`(`LOC` ko 78줄), 오류 문구 `editor.markdownPreviewError`(79줄). 지연 갱신은 `TS/widgets/editor-pane/use-editor-markdown-preview.ts`(`previewTimeoutRef`).
- 렌더: `TS/shared/lib/markdown.ts` `marked(source)` → `DOMPurify.sanitize`. DOMPurify 기본 허용 태그에 `details`·`summary`·`input`이 들어 있습니다(`<ROOT>/node_modules/dompurify/dist/purify.es.mjs`).
- 표시: `TS/features/editor/markdown-preview.tsx` 5~22줄의 Tailwind 클래스. 컨테이너 `px-6 py-4 text-sm`(좌우 24px·상하 16px·14px 글자), `[&_code]:rounded bg-app-sidebar-item-hover px-1 py-0.5`, `[&_p]:my-2 leading-relaxed`, `[&_h1]:mt-4 mb-2 text-2xl font-semibold`, `[&_img]:max-w-full`, `[&_li]:ml-5`, `[&_table]:border-collapse`, `[&_td]/[&_th]:border px-2 py-1` 등. Tailwind v4 preflight(`TS/shared/styles/global.css` 1줄 `@import 'tailwindcss'`)가 적용되어 브라우저 기본 margin이 0입니다.
- 링크: 미리보기 안 `<a>` 클릭은 `ExternalLinkProvider`(`TS/app/providers/external-link-provider.tsx`)가 다른 origin의 http(s)만 OS 브라우저로 보내고, 같은 origin·상대 경로·`#fragment`는 webview 기본 동작(같은 문서 안 이동 = 해당 `id`/`name` 요소로 스크롤)에 맡깁니다(`TS/shared/lib/external-anchor.ts`). 실패 toast `common.openExternalLinkFailed`(`LOC` ko 55줄).
- 원본 엔진은 macOS Tauri의 WKWebView(WebKit)입니다.

#### (c) native 현재 자산과 빠진 것
분할·수명·readonly·두 뷰·독립 scroll·HTML 파서(`dom_query`, `APP/editor-markdown-html.rs` 1줄)·안전 URL(`target` 51~62줄: http·https·file, 링크만 mailto, 이미지만 `data:image/`)·이미지·표·목록·문단 간격·코드 요소 경계·`kbd`/`samp`(monospace만)가 연결되어 있습니다(`QA/2026-11-02-native-lane-preview-preview-markdown.md` 69~81줄). 결정 7 A 목록 중 남은 것과 현재 코드:

| 항목 | 원본 결과(WebKit) | native 현재(실제 코드) |
| --- | --- | --- |
| 1 `br` 줄 배치 | 줄 바꿈 | parser는 `Inline::Break`를 냄(`APP/editor-markdown-html.rs` 137~140줄). 미리보기 배치 검사 없음 |
| 2 혼합 inline code 줄 배치 | code 요소가 줄을 넘으면 조각마다 배경, 좌우 padding·둥근 모서리는 요소 시작·끝 조각에만(CSS 기본 `box-decoration-break: slice`) | 같은 요소 span을 한 Frame으로 묶는 구조(`QA/2026-10-12-native-batch23-markdown.md` 227줄 이후). 줄 넘김 조각 처리 미검증 |
| 3 중첩 code 박스 | `<code>a<code>b</code></code>`면 바깥 박스 안에 안쪽 박스 | `inlines`가 `code`를 만날 때마다 새 식별자로 덮어써(118~122줄) 안쪽 텍스트는 안쪽 식별자만 가짐 → 인접 박스로 그려짐 |
| 4 문단 안 이미지 경계 | inline 이미지가 글자 baseline에 붙고 `max-w-full` | `Inline::Image`(142~167줄). 경계 검사 없음 |
| 5 raw `input` | 체크박스 외 입력(텍스트·버튼 등)이 브라우저 기본 모양(결정 7 A: 비활성 모양) | 작업 목록 checkbox 외에는 버림(`"input" => return,` 141줄) |
| 6 첫 블록 위 여백 | 컨테이너 padding 16px + 첫 블록 자기 margin(padding이 있어 margin 상쇄 없음): `p` 8px, `h1`·`h2` 16px, `h3` 12px, `pre` 8px, `hr` 16px, `table`·`ul`·`ol`·`blockquote` 0 | 첫 table 위 여백 불일치가 기록됨(`MATRIX` 271줄 "첫 table") |
| 7 `details`/`summary` | 접힌 상태로 summary와 펼침 표식, 클릭으로 토글, `open` 속성이면 펼침 | `details`·`summary`가 `BLOCK_TAGS`(12~37줄)에 없어 inline으로 흘러 summary와 본문 글자가 한 문단으로 이어짐. `Block`에 해당 변형 없음(`ED/documentation.rs` 47~72줄) |
| 8 `a name`/`id` 앵커 이동 | `#frag` 클릭 시 미리보기가 그 요소로 스크롤 | `#frag`는 base URL과 합쳐 `file://…#frag`가 되고 `open_link`(`APP/editor-documentation.rs` 848~879줄)의 `"file"` 분기에서 같은 파일을 여는 명령(`Command::OpenDocumentationFile`)이 됨 → 스크롤하지 않음 |
| 9 표·목록 AccessKit 역할 | 브라우저가 table/row/cell, list/listitem 역할을 노출 | 역할 지정 없음 |
| 10 `kbd` 전용 `[A]` | — | `[C]`(`editor-markdown-parse-tests.rs` `kbd와_samp는_monospace지만_code_요소_박스가_아니다`)·`[U]`만 있음 |

비대상(결정 7 A, 비고에 근거와 함께 유지): 임의 CSS(`style` 속성)·상대 굵기(bolder)·counter·`col`/`colgroup` 스타일·SVG·MathML·audio/video.

#### (d) 구현 방법
공통: 기존 parser/모델(`Block`·`Inline`·`Span`·`TableLayout`)에 선택 필드·변형을 더하는 방식(이전 단계와 같음). `Block`·`Inline`은 ED 공개 타입이므로 바꾼 묶음마다 동결 미러 두 명령을 실행합니다. `Block`을 match하는 파일은 `grep -rln "Block::Rule" <ROOT>/native/taide-native-ui <ROOT>/native/taide-native-app <ROOT>/native/taide-native-editor`의 결과(`UI/editor-markup-preview.rs`·`editor-markup-flow.rs`·`editor-markup.rs`·`editor-markup-table.rs`, `APP/editor-documentation-images.rs`·`editor-markdown-html.rs`·`editor-markup.rs`·`editor-markup-tests.rs`·`editor-markup-autolinks.rs`)입니다.
1. `br`: `[U]`로 현재 배치를 먼저 관찰하고 한 문단의 두 줄로 그려지는지 확인합니다. 틀리면 `UI/editor-markup-flow.rs`의 inline 흐름에서 `Inline::Break`를 강제 줄바꿈으로 처리합니다.
2. 혼합 code 줄 배치: 같은 `code_element`의 연속 span이 줄을 넘을 때 줄마다 배경 rect를 따로 그리고, 왼쪽 padding·둥근 모서리는 첫 조각, 오른쪽은 마지막 조각에만 적용합니다(egui `CornerRadius`의 모서리별 반지름).
3. 중첩 code: `Span`에 `code_parent: Option<usize>`를 추가해 바깥 박스를 그 안쪽 span들까지 감싸 그립니다(`code_element`는 유지). 브라우저 결과(바깥 padding 4px 안에 안쪽 padding 4px)를 기대값 표로 만듭니다.
4. 문단 안 이미지: 이미지 높이가 줄 높이보다 크면 그 줄 높이를 이미지 높이로 늘리고 이미지 아래 변을 글자 baseline에 맞춥니다(WebKit 기본 `vertical-align: baseline`).
5. raw `input`: `Inline`에 `Control { kind: InputKind, checked: bool, value: String }`(새 변형)을 추가하고 UI는 비활성 모양(회색 테두리 상자, checkbox/radio는 기본 크기)으로 그립니다. 크기·색 기대값은 WebKit(Safari) 기준으로 만듭니다(4절 함정 10).
6. 첫 블록 여백: 블록 사이 간격 계산에서 첫 블록 위에는 "컨테이너 padding + 그 블록 자기 margin"만 더하도록 고칩니다(위 표 값). `[U]`로 첫 블록이 table·p·h1인 세 경우의 y 위치를 단언합니다.
7. `details`/`summary`: `BLOCK_TAGS`에 `details`를 추가하고 `Block::Details { summary: Vec<Inline>, body: Vec<Block>, open: bool }`를 추가합니다. 펼침 상태는 미리보기 view별 egui 메모리에 두되, 원본은 갱신 때 `dangerouslySetInnerHTML`로 DOM을 통째로 바꿔 펼침 상태가 `open` 속성 값으로 초기화되므로 native도 문서 내용이 바뀌면 초기화합니다. summary 앞에 펼침 표식(WebKit 기본 삼각형)을 그리고 클릭으로 토글합니다.
8. 앵커: parser가 `id`/`name` 속성을 가진 요소의 블록 위치를 `anchors: HashMap<String, usize>`로 기록합니다. `PreviewProvider::open_link`(`APP/editor-markdown.rs` 425줄)에서 대상 URL이 현재 문서 URL과 같고 fragment가 있으면 `open_link` 대신 미리보기 scroll을 그 블록 위치로 옮깁니다. 없는 fragment면 무동작(브라우저와 같음).
9. AccessKit: 표 요소에 `Role::Table`·`Row`·`Cell`/`ColumnHeader`, 목록에 `Role::List`·`ListItem`을 지정합니다. 기존 방법은 `ui.ctx().accesskit_node_builder(id, |node| node.set_role(egui::accesskit::Role::List))`(`UI/snippet-editor.rs` 735~758줄)입니다.
10. `kbd` `[A]`: `APP/application-markdown-tests.rs`에 `<kbd>Ctrl</kbd>` 문서 → 미리보기에 배경 rect 없음·monospace 글자.

#### (e) 자동 검사
항목마다 `[C]` parser(`APP/editor-markdown-parse-tests.rs`), `[U]` shape(`UIT/editor-markup-preview.rs`), 묶음마다 `[A]` 1건(`APP/application-markdown-tests.rs`: 실제 파일·입력/undo·원문 보존·작업 0). 브라우저 기대값은 사람이 만든 근거 표로 두되 WebKit(Safari) 기준으로 측정합니다(이전 근거는 headless Chrome, `QA/2026-10-12-native-batch23-markdown.md` 143·171·227줄). 실행: 0.3의 5·6번(`--lib -- editor_markdown application::markdown_tests`), 동결 미러(ED 타입 변경 시), fmt.

#### (f) 완료 판정
위 10개 통과, 비대상은 비고에 근거와 함께 유지, `MATRIX` complete.

#### (g) 막는 요소
없음. 픽셀 비교 실기는 complete 조건이 아닙니다. Safari 기대값 측정은 사람이 하거나 허락된 방식으로만 합니다(GUI 실행 금지).

### 3.9 editor-73 Git gutter·hunk 되돌리기·선택 stage

#### (a) 상태
editor-73 · Git gutter·hunk 되돌리기·선택 stage · 메인 **missing**(`MATRIX` 272줄), git lane **complete**(`GIT/docs/quality-assurance/2026-10-09-native-function-matrix.md` 272줄: "git lane G8: 파일 편집기 gutter 막대(git-58), lane 누름 hunk discard 대화상자(git-59), 편집기 메뉴·keybinding 선택/커서 hunk stage(git-60)를 실제 App 편집기 그리기 경로(`domains/editor.rs` `show_document` → `State::editor_layers`·`show_git_editor`)로 연결").

#### (b) 원본 동작
`TS/widgets/editor-pane/use-editor-git-gutter-and-conflicts.ts` 58·224·238·292줄(gutter 장식·hunk discard·선택 stage). 상세는 `HO/14-remaining-git.md`의 git-58~60 절과 `GIT/docs/quality-assurance/2026-11-02-native-lane-git-g8-blame-conflicts.md`.

#### (c) native 현재 자산과 빠진 것
git lane 작업 트리에 구현되어 있습니다(`GIT/native/taide-native-app/src/git-editor.rs` `editor_layers` 58줄, `show_git_editor` 472줄, `GIT/native/taide-native-app/src/domains/editor.rs` 5878·6103줄 호출). 메인에는 아직 없습니다(병합 전). 이 영역에서 빠진 구현은 없습니다.

#### (d) 구현 방법
구현하지 않습니다. git lane 병합 뒤 메인 `MATRIX` 272줄이 complete인지 확인하고, 아니면 git lane 기능표의 문장과 근거 문서로 갱신합니다.

#### (e) 자동 검사
git lane의 `[A]`(`GIT/native/taide-native-app/src/application-git-*-tests.rs`)가 병합 게이트에서 통과하는지 메인이 확인합니다. 이 영역 작업자는 병합 뒤 `show_document`를 고칠 때 0.3의 6번으로 `application::git_` 필터를 함께 돌려 회귀가 없는지 봅니다.

#### (f) 완료 판정
git lane 병합 + 병합 게이트 통과 → complete.

#### (g) 막는 요소
git lane 병합(메인 작업). 병합 전에 이 행을 다시 구현하지 마십시오.

### 3.10 editor-74 merge conflict 표시·해결 dialog

#### (a) 상태
editor-74 · merge conflict 표시·해결 dialog · 메인 **missing**(`MATRIX` 273줄), git lane **complete**(git lane 기능표 273줄: 충돌 영역 배경·시작 줄 lane(git-61), 해결 대화상자와 저장 정착(git-62), ours↔theirs 비교 대화상자(git-63), "해결 대화상자가 열린 동안 문서가 짧아진 경우의 패닉을 고치고 `[C]`·`[A]` 검사를 더했습니다").

#### (b) 원본 동작
`use-editor-git-gutter-and-conflicts.ts` 211~219줄, `TS/widgets/editor-pane/editor-pane.tsx` 520~533줄. 상세는 `HO/14-remaining-git.md` git-61~63 절.

#### (c)~(g)
editor-73과 같습니다. 근거 문서는 `GIT/docs/quality-assurance/2026-11-02-native-lane-git-g8-blame-conflicts.md`(6.1절 패닉 수정 포함)입니다.

### 3.11 editor-75 blame

#### (a) 상태
editor-75 · blame(줄 footer·overlay·토글·파일 이력) · 메인 **missing**(`MATRIX` 274줄), git lane **complete**(git lane 기능표 274줄: 커서 줄 footer(git-56), overlay·`git.toggleBlame`·편집기 메뉴 토글(git-57), 편집기 메뉴 File History(git-55 남은 범위), 파일 이력 패널은 G7(git-53·54)).

#### (b) 원본 동작
`TS/widgets/editor-pane/use-editor-blame.ts` 54·116~143줄, `TS/features/editor/blame-footer-bar.tsx`. 상세는 `HO/14-remaining-git.md` git-53~57 절.

#### (c)~(g)
editor-73과 같습니다. 근거 문서는 `GIT/docs/quality-assurance/2026-11-02-native-lane-git-g8-blame-conflicts.md`·`…-g7-commit-claude-diff.md`입니다. editor-34의 `monaco.taide.toggleBlame`·`openFileHistory`도 같은 근거로 닫힙니다(1.3 표).

### 3.12 editor-76 diff 편집기

#### (a) 상태
editor-76 · diff 편집기(Git diff·commit diff·Claude diff·conflict 비교) · 메인 **missing**(`MATRIX` 275줄), git lane **partial**(git lane 기능표 275줄: "남은 것: git-40의 문자 단위 강조·±·overview ruler와 git-50 ClaudeDiff revert 화살표(결정 4 A)").

#### (b) 원본 동작
- `TS/features/git/diff-view.tsx` 26~83줄: monaco diff editor, `readOnly`, `ignoreTrimWhitespace: false`, `renderSideBySide`, `hideUnchangedRegions`. ClaudeDiff(`TS/widgets/claude-diff-pane/claude-diff-pane.tsx` 58줄)는 옵션을 주지 않아 Monaco 기본 `ignoreTrimWhitespace: true`, 수정 쪽 편집 가능.
- Monaco 0.56 diff 기본값(`MON/editor/common/config/diffEditor.js` 5~36줄): `renderSideBySide: true`, `renderMarginRevertIcon: true`, `renderGutterMenu: true`, `renderIndicators: true`, `renderOverviewRuler: true`, `diffAlgorithm: 'advanced'`, `renderSideBySideInlineBreakpoint: 900`.
- 줄·문자 장식(`MON/editor/browser/widget/diffEditor/components/diffEditorDecorations.js` 30~74줄): 변경 줄 배경, 한쪽이 비면 전체 줄 문자 강조, 양쪽이 있으면 `innerChanges`(문자 단위 범위)에 문자 배경.
- ±표시(`.../registrations.contribution.js` 25~40줄), overview ruler(`.../features/overviewRulerFeature.js`), 문자 단위 계산(advanced 알고리즘 `MON/editor/common/diff/defaultLinesDiffComputer/defaultLinesDiffComputer.js`의 `refineDiff`).
- **revert(사양 정정)**: Monaco 0.56은 `renderGutterMenu`가 참이라 옛 여백 화살표가 그려지지 않고, 두 편집기 사이 거터의 hunk toolbar "Revert Block"(side-by-side `arrowRight`, inline `discard`)과 선택이 있으면 "Revert Selection"이 **수정 쪽이 쓰기 가능할 때만** 나타납니다(`diffEditorOptions.js` 42~56줄, `features/gutterFeature.js`, `diffEditor.contribution.js` 41~63줄). 그래서 읽기 전용 Git diff에는 거터가 없고 ClaudeDiff에만 있습니다. 상세 수치(거터 35px, 버튼 20×34px, hover 표시 규칙)는 `HO/14-remaining-git.md` 3.5절 737~743줄에 정리되어 있습니다.
- 사용자 문자열: Monaco 영어 고정("Revert Block", "Revert Selection").

#### (c) native 현재 자산과 빠진 것
git lane이 공용 diff 위젯 `GIT/native/taide-native-ui/src/diff-view.rs`로 Git diff 탭(G5), commit diff(git-47), Claude diff(git-50), 충돌 비교(git-63)를 그립니다: side-by-side·inline·900px 자동 inline·채움 빗금·줄 배경·스크롤 동기화·분할선 끌기·미변경 영역 숨김(`GIT/docs/quality-assurance/2026-11-02-native-lane-git-g5-diff.md`). 편집기 표면에는 `SurfaceExtras`(`ExtraZone`·`hidden_lines`)와 `show_request_with_extras`가 추가되었습니다(git lane 커밋 `5195d415`의 `UI/editor_surface.rs` 변경). 빠진 것(git lane 기능표 git-40·git-50 비고): 문자 단위 강조, ±표시, overview ruler, inline 삭제 줄의 구문 색, inline 원본·수정 두 줄 번호 열, ClaudeDiff 거터 Revert Block·Revert Selection. 줄 계산은 git 사양 결정 7 A로 libgit2 hunk입니다.

#### (d) 구현 방법(git lane 병합 뒤, 소유자는 git lane)
정본 절차는 `HO/14-remaining-git.md`의 다음 절입니다. 이 문서에 따로 쓰면 두 문서가 어긋나므로 순서만 옮깁니다.
1. 3.3.1: Monaco `refineDiff`를 `taide-native-editor`로 이식하고 원본 oracle로 검증합니다(git-40 문자 강조, inline 삭제 줄 문자 강조, git-50 선택 revert, git-42 이동 블록 판정이 공유).
2. 3.3.2~3.3.7: 문자 강조 → ± 표시와 gutter 배경 → inline 원본 줄 번호 열·삭제 줄 구문 색 → hunk 안 정렬 → overview ruler.
3. 3.5: ClaudeDiff revert gutter(`Request.revert_blocks`·`is_diff_current`, `Output.revert`, 앱의 되돌리기 편집과 200ms 재계산).

#### (e) 자동 검사
`HO/14-remaining-git.md` 3.3·3.5의 `[C]`(`refineDiff` oracle)·`[U]`(`GIT/native/taide-native-ui/tests/diff-view.rs`)·`[A]`(`application-git-claude-history-tests.rs` 등)를 따릅니다.

#### (f) 완료 판정
git-40·git-50의 남은 범위가 닫히면 editor-76 complete(git lane 근거로 함께 갱신, 같은 비고 문장을 쓰도록 `HO/14-remaining-git.md` 142줄 "git 행을 닫으면 함께 갱신할 다른 영역 행").

#### (g) 막는 요소
git lane 병합(선행)과 소유 lane 조정. 이 영역 작업자가 맡으려면 메인과 소유를 먼저 정합니다.

### 3.13 preview-5 외부 저장·이동·삭제 시 미리보기 갱신/무효화

#### (a) 상태
preview-5 · 외부 저장·이동·삭제 시 미리보기 갱신/무효화 · **partial**(`MATRIX` 435줄: "남은 것: HTML·PDF 외부 수정 `[A]`(WebView/PDFKit 실기 의존), 앱 안 rename 뒤 새 경로 표시·override 유지 `[A]`, 앱 안 삭제(휴지통 경로, 허락 항목)").

#### (b) 원본 동작
- 외부 변경: `fs:changed`의 경로에 해당하는 `FILE.RAW`·`FILE.CONTENT`를 `fromApp`과 무관하게 무효화합니다(`TS/app/providers/ipc-sync-provider.tsx` 489~496줄 `invalidateQueries({ queryKey: QUERY_KEY.FILE.ALL, predicate: … isFilePathQueryForChangedPaths … })`, 판정 함수 26~51줄) → 미리보기가 다시 읽습니다.
- PDF 재읽기: 1쪽과 **초기 배율**로 돌아갑니다(`TS/features/preview/pdf-preview.tsx` load effect 70~71줄 `setCurrentPage(1)`·`setScale(PDF_INITIAL_SCALE)`, `PDF_INITIAL_SCALE = 1` 22줄). `SPEC` 2.19의 "배율 유지"는 틀렸고 preview QA 43줄이 정정했습니다.
- rescan: `FILE.ALL` 전체 무효화(252~269줄). 프로젝트 닫힘: 루트 아래 query 제거(326~327줄 `removeQueries({ predicate: … isQueryKeyUnderProjectRoot(query.queryKey, project.root) })`).
- 앱 안 이름 바꾸기: 탭 경로 이동, override 이동, 새 경로 `FILE.RAW` 무효화(`TS/entities/layout/tab-path-change.ts` `followRenamedPathInTabs` 316줄 이후, `reconcileRenamedLayoutPaths` 296줄).
- 앱 안 삭제: 탭 닫기 후 경로별 상태 해제·캐시 제거(`releaseClosedFileTabPath` 145~165줄, `followDeletedPathInTabs` 332줄).
- 외부 삭제: 탭은 닫지 않고 다시 읽기 실패 → `UnsupportedPreview`(문구 `preview.notSupported`, 버튼 `preview.openExternally`, `LOC` ko 505~506줄).

#### (c) native 현재 자산과 빠진 것
- 있는 것: `APP/events.rs` `record_preview`(62줄) → `reconcile_previews`(`APP/domains/preview.rs` 433~541줄: 형식별 `invalidate`/`invalidate_root`/`invalidate_all`, 실패·거절 표시 정리, `open_with.reconcile`, 탭에 없는 경로 캐시 정리, web 요청 취소), override 이동(`APP/open_with.rs` `Registry::reconcile` 91줄: 같은 탭 id의 경로가 바뀌면 `self.set(from, Mode::Preview); self.set(to, Mode::Editor);` 122~123줄), PDF 무효화 시 page·zoom 초기화(`APP/preview_pdf.rs` `invalidate` 210~213줄 `*view = View::new(view.path.clone())` = 원본과 같음). `[A]` `실제_app_미리보기는_외부_수정과_삭제와_프로젝트_닫힘을_여러_뷰에_반영한다`(`APP/application-preview-tests.rs` 307줄)가 PNG·CSV 외부 수정, 두 pane 공유, 외부 삭제, 재생성, 프로젝트 닫힘을 확인했습니다.
- 빠진 것: HTML·PDF 외부 수정 `[A]`, 앱 안 rename 뒤 새 경로 표시·override 유지 `[A]`, 앱 안 삭제.

#### (d) 구현 방법
결함이 나오지 않으면 코드 변경은 없습니다. 검사만 추가합니다.
1. PDF: `APPT/preview-pdf.rs`(`#![cfg(target_os = "macos")]`)가 합성 PDF로 CoreGraphics 디코드를 headless로 검사하므로 앱 lib에서도 PDF 디코드가 동작합니다. 합성 PDF 생성 함수 `pdf(pages: &[(&str, &str)]) -> Vec<u8>`(45~85줄)는 통합 검사 파일이라 lib 검사에서 import할 수 없으므로 `APP/application-preview-tests.rs`에 같은 helper를 복제합니다. 단언은 화면 픽셀 대신 상태로 합니다: `application.preview_state.pdf_previews.total_pages(&tab)`(236줄)·`selection(&tab)`(232줄)·`texture(&tab)`(459줄).
2. HTML: WebView는 lib 검사에서 만들어지지 않으므로 `preview_state.web_previews.source(path)`(`APP/preview_web_cache.rs` 254줄)의 URL이 외부 수정 뒤 무효화되어 새로 발행되는지로 단언합니다. lib 검사에서 source가 준비되지 않으면(`preview_state.web_bridge`가 `None`이거나 helper 실행 파일이 없음) 그 사실을 QA에 적고 통합 검사(`APPT/preview-web-*.rs`) 수준에서 HTTP 응답 바이트가 새 내용인지로 대체합니다.
3. 앱 안 rename: 탭 메뉴 "이름 바꾸기"(`tab.rename`)로 탐색기 inline rename을 시작하는 흐름이 `APP/application-tab-menu-tests.rs` 258~271줄에 있습니다(draft 생성까지). 같은 흐름으로 `egui::Event::Text(새 이름)`(보조 창 선례 `APP/application-auxiliary-window-tests.rs` 1016줄)과 `Key::Enter`를 보내 실제 임시 파일 rename(소유 임시 프로젝트 안, 휴지통 미사용)을 끝낸 뒤 (1) 탭 `TabKind::File { path }`가 새 경로, (2) 새 경로 이미지 texture가 그려짐, (3) Editor로 설정해 둔 다른 PNG 탭의 override가 새 경로에서도 `Surface::Editor`를 단언합니다.
4. 앱 안 삭제: native 삭제 경로(`APP/workspace_delete.rs`, `APP/explorer_delete.rs`)는 `CR/taide-file/src/service.rs` 191~208줄의 `trash::delete`로 휴지통을 씁니다. 주입 경로가 없으므로 허락 전에는 검사하지 않습니다.

#### (e) 자동 검사
`APP/application-preview-tests.rs`에 세 검사: `실제_app_pdf_미리보기는_외부_수정_뒤_새_쪽수와_첫쪽_초기배율로_다시_읽는다`, `실제_app_html_미리보기는_외부_수정_뒤_새_문서를_발행한다`, `실제_app_앱_안_이름바꾸기는_미리보기_경로와_override를_옮긴다`. 이벤트는 `record_change(application, &project, FsChangeKind::Modified, path)`. 실행: `--lib -- application::preview_tests --test-threads=2`.

#### (f) 완료 판정
세 검사 통과 + 앱 안 삭제는 허락 대기 사실을 비고에 적고 판정 보류(또는 허락 뒤 검사 통과로 complete).

#### (g) 막는 요소
앱 안 삭제는 사용자 휴지통이라 허락 필요(`SPEC` 4.2-4, `DEC` 보류 표 "실제 휴지통 사용 검사"). HTML은 lib 검사에서 WebView가 없어 상태 단언으로 대체합니다.

### 3.14 preview-14 비디오 재생

#### (a) 상태
preview-14 · 비디오 mp4/webm/mov/m4v 재생(컨트롤·seek) · **partial**(`MATRIX` 444줄: "실제 WKWebView 적용·재생·seek·컨트롤·키보드/AX는 GUI 허락 항목이라 partial").

#### (b) 원본 동작
`TS/features/preview/video-preview.tsx` 7~11줄: `bg-editor-background flex h-full w-full items-center justify-center p-4` 안의 `<video src={src} controls className='max-h-full max-w-full' />`. 소스 `convertFileSrc(path)`(asset protocol, Range 지원). theme 변경은 CSS만 바뀌어 재생 위치·일시정지·볼륨이 유지됩니다. 바이트를 미리 읽지 않고 바로 요소를 그립니다. 사용자 문자열 없음.

#### (c) native 현재 자산과 빠진 것
- 있는 것: media 문서 템플릿(`APP/preview_web_media.rs` 20줄 `VIDEO_TEMPLATE`, 22줄 `STYLES`의 `.video video{max-height:100%;max-width:100%}`), 승인 host·Range·캐시, macOS child WebView(`APP/preview_web_view.rs`). theme 변경 시 `tick_preview_media`(`APP/domains/preview.rs` 590~624줄)가 열린 오디오·비디오 탭 view에 `restyle_script`(54줄: `document.querySelector('style').textContent=<새 style JSON 문자열>;`)를 `Views::evaluate`(`APP/preview_web_view.rs` 280~292줄)로 보내고 닿은 탭의 경로만 `restyle_media`(`APP/preview_web_cache.rs` 94줄)로 유지합니다. `show_preview_web_views`(626줄)는 sync 전에 `release_restyled`(117줄)를 부릅니다. 페이지 JS는 꺼져 있고(`vendor/wry-preview/src/wkwebview/mod.rs` 349~355줄 `setAllowsContentJavaScript(false)`) 앱 스크립트(`evaluateJavaScript`)는 막히지 않습니다(preview QA 55줄).
- 빠진 것: 실제 WKWebView에서 restyle 적용·재생 유지·seek·컨트롤·키보드/AX 근거(GUI 필요). `[A]`도 WebView가 있어야 해서 없습니다. 자동 근거는 `[C]` 두 개(`preview_web_media.rs` 199줄 `web_media_restyle_script는_문서와_같은_style만_새_색으로_바꾼다`, `preview_web_cache.rs` 379줄 `web_cache_restyle은_새_색을_받은_view만_유지하고_새_view나_닫힘이면_다시_읽는다`)뿐이고 `tick_preview_media` → `Views::evaluate` → WKWebView 경로는 자동으로 실행된 적이 없습니다(preview QA 109줄).
- 알려진 창: `Views::evaluate`는 `child.view.evaluate_script(script).is_ok()`(289줄)를 "적용됨"으로 봅니다. 이 호출은 JS 실행 결과를 기다리지 않습니다(`vendor/wry-preview/src/lib.rs` 2005~2009줄 `eval(js, None)`). WebView가 막 만들어져 문서가 commit되기 전에 theme가 바뀌면 `querySelector('style')`이 null이라 스크립트가 실패해도 restyled로 남아 그 view가 닫힐 때까지 옛 색이 보일 수 있습니다(preview QA 110줄).
- 원본과 다른 점(이미 기록됨): 같은 경로가 두 pane에 있고 한쪽이 새로 view를 만들 때 문서 교체로 다른 쪽 재생 위치도 처음으로 돌아갑니다(preview QA 133줄). cache 구조를 탭 단위로 바꿔야 해서 남아 있습니다.

#### (d) 구현 방법
1. 알려진 창 수정(실기와 함께 확정): `APP/preview_web_view.rs` 파일 끝 `impl Views` 블록에 `evaluate_checked(&self, tabs, script, results: Arc<Mutex<Vec<(TabId, bool)>>>)`를 추가하고 `child.view.evaluate_script_with_callback(script, move |value| …)`(lib.rs 2015줄)를 씁니다. 스크립트는 `(function(){var s=document.querySelector('style');if(!s)return 'missing';s.textContent=…;return 'ok';})()`이고, 콜백 인자는 결과의 JSON 직렬화 문자열(`"\"ok\""`)입니다. `restyle_media`는 "대기" 상태로 두고 다음 프레임 `tick_preview_media`에서 결과가 `"ok"`인 탭만 유지·나머지는 무효화합니다. 콜백은 주 스레드에서 오므로 `Arc<Mutex<…>>` 수신함으로 충분합니다.
2. 탭 단위 재생 유지(선택, 메인 결정): `preview_web_cache`의 문서 발행 단위를 경로에서 탭으로 바꾸면 위 "원본과 다른 점"이 사라집니다. 구조 변경이 커서 메인 확인 뒤 진행합니다.
3. 실기(허락 뒤): `/Users/hyunseokbyun/development/TAIDE/docs/memory/2026-10-07-native-transition-operations.md` 5절의 방법(합성 데이터 디렉터리로 앱 실행, `screencapture -l <CGWindowID>`로 창만 캡처)으로 확인합니다. 확인 항목: 재생 중 테마 전환 → 재생 위치 유지·색 변경, seek·볼륨·전체화면 컨트롤, 키보드 Space 재생/정지, VoiceOver로 컨트롤 이름. 합성 키·마우스 입력은 WKWebView에 닿지 않고 금지이므로 조작은 사용자가 하거나 사용자가 허락한 방식으로만 합니다.

#### (e) 자동 검사
기존 `[C]` 두 개 유지. 1번을 구현하면 `APP/preview_web_cache.rs` `mod tests`에 "콜백 결과 `missing`이면 무효화, `ok`면 유지, 결과가 오기 전 닫힘이면 무효화"를 추가합니다. `tick_preview_media` → WKWebView 경로는 자동 검사로 실행할 수 없습니다(GUI). 실행: `--lib -- preview_web_cache preview_web_media preview_web_view --test-threads=2`, `--test preview-web-media`.

#### (f) 완료 판정
3번 실기 근거(허락된 방식) + 1번 `[C]`. 허락이 없으면 partial을 유지하고 사유를 비고에 둡니다.

#### (g) 막는 요소
GUI 실기 허락(`SPEC` 4.2-2, `DEC` 보류 표 마지막 행 "실기 … 배치 31"). 보조 창 WKWebView는 shell-5(shell lane)와 함께 다룹니다.

### 3.15 preview-15 오디오 재생

#### (a) 상태
preview-15 · 오디오 mp3/wav/flac/m4a/ogg(아이콘·파일명·컨트롤) · **partial**(`MATRIX` 445줄).

#### (b) 원본 동작
`TS/features/preview/audio-preview.tsx` 9~15줄: 세로 배치·간격 16px·여백 16px, Music 아이콘 40px(`text-app-sidebar-icon-default`), 파일 이름 14px·최대 448px·말줄임, `<audio controls>` 폭 100%·최대 448px. theme 변경 동작은 비디오와 같습니다. 사용자 문자열 없음(파일 이름).

#### (c) native 현재 자산과 빠진 것
`APP/preview_web_media.rs` 21줄 `AUDIO_TEMPLATE`(Lucide music 경로 SVG)·22줄 `STYLES`(`.audio{flex-direction:column;gap:16px}`, `.audio svg{width:40px;height:40px}`, `.audio span{font-size:14px;line-height:20px;max-width:448px;…text-overflow:ellipsis}`, `.audio audio{width:100%;max-width:448px}`)와 `style`(45줄)의 `.audio svg{color:<muted>}`가 원본 수치와 같습니다. restyle 경로·빠진 것·알려진 창은 preview-14와 같습니다.

#### (d)~(g)
preview-14와 같은 구현·검사·판정·막는 요소입니다. 실기 항목에 "파일 이름 말줄임과 아이콘 색이 테마 전환 뒤 바뀜"을 추가합니다.

### 3.16 기능표에 행이 없는 원본 기본 기능

`SPEC` 1.4와 결정 2 A(`DEC` "채택" 7줄: 4.1 결정 추천안 채택)에 따라 A~D는 새 행으로 추가하고 구현합니다. 2026-11-04 현재 기능표에 editor-88 이후 행이 없습니다(`grep -n -E "editor-(8[89]|9[0-9])" MATRIX` 결과 없음). 행 번호는 메인이 기능표에 추가할 때 정합니다(아래 제안 번호는 가칭이며 css-services QA 90줄이 번호 충돌을 피하려 색 장식 행을 만들지 않은 사정과 같음).

원본이 이 기능들을 끄지 않는다는 근거: `TS/features/editor/code-editor.tsx`의 `monaco.editor.create` 옵션(180~201줄)과 `updateOptions` 호출들(271~320줄)에 `unicodeHighlight`·`links`·`selectionHighlight`·`colorDecorators`·`dragAndDrop`·`readOnlyMessage`·`unusualLineTerminators`가 없습니다(`grep -rn -E "unicodeHighlight|colorDecorators|selectionHighlight|links:|dragAndDrop|readOnlyMessage|unusualLineTerminators" TS --include='*.ts' --include='*.tsx'`의 결과는 `monaco-action-commands.ts`·`monaco-actions.ts`의 액션 id뿐). 원본이 바꾸는 큰 파일 관련 옵션은 `folding`·`bracketPairColorization`·`minimap`뿐입니다(271·275줄).

#### A. Unicode 강조

**(a)** 행 없음(제안: editor-88 Unicode 강조) · missing.

**(b) 원본 동작**
- 기본값(`MON/editor/common/config/editorOptions.js` 2074~2093줄): `nonBasicASCII: 'inUntrustedWorkspace'`, `invisibleCharacters: true`, `ambiguousCharacters: true`, `includeComments: 'inUntrustedWorkspace'`, `includeStrings: true`, `allowedCharacters: {}`, `allowedLocales: { _os: true, _vscode: true }`. standalone의 작업 공간 신뢰는 항상 참(`MON/editor/standalone/browser/standaloneServices.js` 622줄 `isWorkspaceTrusted()`)이므로 `resolveOptions`(`MON/editor/contrib/unicodeHighlighter/browser/unicodeHighlighter.js` 177~187줄) 결과는 nonBasicASCII **false**, includeComments **false**, includeStrings true입니다.
- locale: `_os`는 `Intl.NumberFormat().resolvedOptions().locale`(148~151줄), `_vscode`는 Monaco nls 언어(설정이 없으면 `'en'`). 혼동 문자 표 선택(`MON/base/common/strings.js` 758~800줄): 표 키(`_common`, `_default`, `cs`, `de`, `es`, `fr`, `it`, `ja`, `ko`, `pl`, `pt-BR`, `qps-ploc`, `ru`, `tr`, `zh-hans`, `zh-hant`, 756줄)와 **정확히 일치**하는 locale만 남기고, 남은 것이 없으면 `_default`, 남은 locale 표들의 교집합을 `_common`에 합칩니다. 따라서 `ko-KR`·`en`이면 `_common` + `_default`입니다.
- 보이지 않는 문자 집합은 모든 locale 표의 합집합입니다(`InvisibleCharacters.getData` 828~832줄: `new Set([...Object.values(getRawData())].flat())`). 공백·`\n`·`\t`는 제외(`isAllowedInvisibleCharacter`).
- 계산(`MON/editor/common/services/unicodeTextModelHighlighter.js`): 후보 code point(보이지 않는 문자 ∪ 혼동 문자 − 허용 문자)를 줄마다 찾고 서로게이트 쌍 전체로 범위를 넓힙니다. 그 문자를 포함한 단어(Monaco 기본 단어 정규식 `DEFAULT_WORD_REGEXP`)가 ASCII를 포함하지 않고 "혼동·보이지 않는 문자가 아닌 비ASCII 글자"를 포함하면 강조하지 않습니다(`shouldHighlightNonBasicASCII`, 예: 전부 한글인 단어). 결과 최대 1,000개(`MAX_RESULT_LENGTH`), 넘으면 `hasMore`.
- 일정(`DocumentUnicodeHighlighter` 188~240줄): 내용 변경 250ms 뒤 worker 계산, 모델에 비ASCII가 없으면 지움, 결과 도착 시 버전이 바뀌었으면 버림. `hasMore`면 장식을 그리지 않고 배너를 띄웁니다(45~96줄: 세 종류 중 가장 많은 것 기준 "This document contains many non-basic ASCII/ambiguous/invisible unicode characters", 버튼은 해당 Disable 액션의 짧은 이름, 닫으면 같은 모델에서 다시 띄우지 않음). 모델이 worker 동기화 한도(50MB)를 넘으면 보이는 영역만 계산(`ViewportUnicodeHighlighter`)하지만 TAIDE는 50MB 초과 파일을 열지 않으므로 Large·ReadOnly 등급 문서도 문서 전체를 계산합니다.
- 표시(`Decorations` 436~460줄, `unicodeHighlighter.css`): `border: 1px solid editorUnicodeHighlight.border`(기본 `editorWarning.foreground`) + `background-color: editorUnicodeHighlight.background`(기본 `editorWarning.background`)(`MON/editor/common/core/editorColorRegistry.js` 92~93줄), 빈 범위도 표시, 미니맵·overview 없음. `includeComments`가 거짓이면 주석 토큰 안의 강조는 숨김(`hideInCommentTokens`).
- hover(341~410줄): "The character {0} could be confused with the ASCII character {1}, which is more common in source code." / "…with the character {1}…" / "The character {0} is invisible." / "The character {0} is not a basic ASCII character." + "Adjust settings" 링크(→ 제외 옵션). 영어 고정.
- 액션(463~620줄): Disable highlighting in comments/strings, ambiguous/invisible/non-basic ASCII 끄기(설정 서비스 USER 값 — standalone에서는 창 realm 메모리, 비영속), Show Exclude Options(quick pick: 그 문자 제외 / 해당 locale 허용).

**(c) native 현재**: 없음(`grep -rn -i unicode APP/editor-*.rs UI/editor-*.rs`에 해당 기능 없음). 액션 4개는 `Execution::Unavailable`이고 `EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE`에 들어 있어 항상 활성으로 보입니다(25~47줄).

**(d) 구현 방법**
1. 자료: `strings.js`의 `AmbiguousCharacters`(756줄 JSON)·`InvisibleCharacters`(827줄 JSON)를 Rust 표(`ED/unicode-data.rs`, MIT 출처)로 옮깁니다. bun이 허락되지 않으면 Write로 직접 작성할 수 있으나 표가 크므로 생성 스크립트를 권장합니다.
2. ED `ED/unicode-highlight.rs`: `compute(snapshot, options, word_at: &dyn Fn(&str, usize) -> Option<Range<usize>>, limits) -> Result<Highlights, EditorError>`(정규식 없이 code point 집합 검사), `reason(ch, options)`(hover용). 단어 판정 함수는 앱이 주입합니다: `crate::editor_syntax::language_rules("plaintext")`의 `LanguageRules::word_range`가 Monaco 기본 단어 정규식으로 컴파일되어 있습니다(`QA/2026-11-02-native-lane-editor-display-commands.md` 113줄). 원본의 서로게이트 확장 조건(`endIndex + 1 < lineLength`)의 경계 결함은 재현하지 않습니다.
3. APP `APP/editor-unicode-highlight.rs`: 문서별 250ms 일정·worker, 창 realm별 옵션 상태(`domains::editor::State`의 `HashMap<ViewportId, Options>`), 배너 상태(문서별 닫힘 기억). `_os` locale은 앱이 이미 쓰는 `sys_locale::get_locale()`(`APP/application.rs` 364·502줄)로 얻고, `_vscode`는 `"en"`입니다.
4. 표시: 테두리가 필요하므로 `DecorationKind::Inline`(테두리 없음)으로는 부족합니다. 배경은 `DecorationLayer`의 `Inline { background }`, 테두리는 문서 하이라이트처럼 편집기 출력 geometry 위에 직접 그립니다(2.3 (e), `APP/editor-highlights.rs` `Display::paint` 414~431줄의 `geometry.range_rects(bytes)` + `rect_stroke`). 주석 숨김은 해당 byte의 토큰 종류 `TokenKind::Comment`로 판정합니다.
5. hover: 기존 hover 공급(`APP/editor-documentation.rs`)에 내장 그룹을 추가(J1과 같은 구조).
6. 배너: 편집기 위 배너(`UI/conflict_banner.rs` 형태 참고)로 원본 문구·버튼.
7. 액션 4개: editor-34의 실행 경로에 연결합니다.

**(e) 자동 검사**: `[C]` 혼동 문자(키릴 `а` U+0430), 보이지 않는 문자(U+200B), 허용 단어(전부 비ASCII인 한글 단어 안의 U+3000 등), `ko-KR` locale이 `_default`로 가는지, 1,000개 상한·`hasMore`. `[A]` 실제 파일에 U+200B → 테두리 rect, 주석 안은 숨김, hover 문구, Disable 액션 뒤 사라짐.

**(f) 완료 판정**: 새 행 complete(구현·실제 앱·검사).

**(g) 막는 요소**: 표 생성에 bun을 쓰려면 허락 필요(손 작성은 가능).

#### B. 텍스트 링크 감지

**(a)** 행 없음(제안: editor-89 텍스트 링크 감지) · missing.

**(b) 원본 동작**
- 공급: `MON/editor/browser/services/editorWorkerService.js` 69~78줄이 모든 언어(`'*'`)에 기본 링크 공급자를 등록하고 worker에서 `computeLinks`(`MON/editor/common/languages/linkComputer.js`)를 돌립니다. 상태 기계는 `http://`, `https://`, `file://`(대소문자 무시)만 링크 시작으로 인정합니다(`getStateMachine` 59~87줄). 강제 종료 문자 `' \t<>\'"、。｡､，．：；‘〈「『〔（［｛｢｣｝］）〕』」〉’｀～…|'`, 끝에 올 수 없는 문자 `.,;:`(`getClassifier` 89~103줄), 앞 글자가 `(`/`[`/`{`이고 링크 끝이 짝 괄호면 그 괄호 제외(`_createLink` 105~138줄). 모델이 너무 크면(`canSyncModel` 거짓) 빈 결과. HTML은 `htmlLinks.js` 공급자도 있으며 결과가 합쳐집니다.
- 일정(`MON/editor/contrib/links/browser/links.js` 36~130줄): 처음·모델·언어·공급자 변경 즉시, 내용 변경은 적응형(최소 1000ms·최대 4000ms, 49~50줄), `editor.links` 기본 true.
- 표시(`links.css`): 감지된 링크는 항상 밑줄. 수정 키(macOS ⌘)를 누른 채 hover하면 활성 링크가 `editorLink.activeForeground` 색과 손가락 커서.
- hover 문구(`getHoverMessage` 315~346줄): "Follow link (cmd + click)"(macOS) / "(ctrl + click)", `command:` URL이면 "Execute command".
- 열기(`openLinkOccurrence` 193~230줄): 수정 키 + 클릭 또는 `editor.action.openLink` → Monaco `OpenerService`(`MON/editor/browser/services/openerService.js`): http(s)는 `windowOpenNoOpener` → Tauri `on_new_window` → `open_new_window_externally`(`TAURI/platform/navigation_guard.rs` 50~68줄: `validate_external_url` 검증 뒤 OS 브라우저). `file://`는 `EditorOpener`(77~97줄, fragment의 줄/열 선택 추출) → `openCodeEditor` → TAIDE 편집기 opener(`TS/shared/lib/bridge/editor-opener-bridge.ts` 40~61줄)가 그 파일 탭을 엽니다. 실패 시 "Failed to open this link because it is not well-formed: {0}" 등 경고(224~227줄).

**(c) native 현재**: 편집기 텍스트 링크 감지 없음(`grep -rln -i "linkComputer\|link_computer" <ROOT>/native <ROOT>/crates` 결과 없음). Markdown 미리보기용 GFM autolink(`APP/editor-markup-autolinks.rs`)는 다른 기능입니다. hover 문서의 링크 열기는 `APP/editor-documentation.rs` `open_link`(848~879줄: http·https·mailto → `ui.ctx().open_url(OpenUrl::new_tab(..))`, file → `HostCommand::Editor(Command::OpenDocumentationFile(request))`)가 있습니다.

**(d) 구현 방법**
1. ED `ED/link-computer.rs`: `linkComputer.js`의 상태 기계·문자 분류를 그대로 이식합니다(정규식 없음). 위치는 원본처럼 줄·UTF-16 열을 계산한 뒤 byte로 바꿉니다.
2. APP `APP/editor-links.rs`: 문서별 일정(초기 즉시, 내용 변경 1000ms 고정 — 적응형 최대값은 측정 근거가 없으므로 QA에 기록), 결과 링크 목록, HTML 내장 링크(editor-57 H2) 합치기. 원본은 등급과 무관하게 계산합니다(50MB 이하).
3. 표시: 밑줄은 `DecorationKind::Inline(InlineStyle { underline: Some(Underline { kind: Straight, color }), .. })`(`ED/decoration.rs` 20~24줄). 활성 링크(수정 키 + hover)는 정의 이동 hover 밑줄(`UI/editor-definition-link.rs`)과 같은 입력 판정을 재사용하고 색만 `editorLink.activeForeground`.
4. 열기: `crate::editor_documentation::open_link(ui, target, project, owner, viewport, commands)`를 재사용합니다. http(s)는 `taide_infra::external_url::validate_external_url`(`CR/taide-infra/src/external_url.rs` 38줄, 앱 의존 `<ROOT>/native/taide-native-app/Cargo.toml` 75줄)로 먼저 검증해 원본 Tauri 검증과 같은 거절을 하고, 통과하면 `ctx.open_url`로 보냅니다(egui `OutputCommand::OpenUrl`이라 kittest에서는 실제 브라우저가 열리지 않고 `FullOutput.platform_output.commands`로 단언 가능). `file://`는 기존 `OpenDocumentationFile` 경로로 그 파일 탭을 엽니다.
5. 정의 이동 제스처와의 공존: 원본 `MON/editor/contrib/gotoSymbol/browser/link/goToDefinitionAtPosition.js`가 링크 위에서 어떻게 동작하는지 먼저 읽고(두 기여가 독립으로 반응하는지) 같은 결과가 되게 합니다.
6. `editor.action.openLink`: 각 선택 끝 위치의 링크를 엽니다.

**(e) 자동 검사**: `[C]` `linkComputer` 사례(괄호로 감싼 URL, 따옴표 안, 끝 마침표, `[` 안, 대문자 `HTTPS://`, `mailto:`는 링크 아님). `[U]` 밑줄 rect. `[A]` 주석 안 `https://example.com` → 밑줄, ⌘+클릭 → `platform_output.commands`에 `egui::OutputCommand::OpenUrl`(URL 일치) 1회, `file://<임시 파일>` → 그 파일 탭 열림.

**(f) 완료 판정**: 새 행 complete.

**(g) 막는 요소**: 없음(실제 외부 열기는 하지 않고 출력 명령으로만 검사).

#### C. 선택 텍스트 발생 강조(SelectionHighlighter)

**(a)** 행 없음(제안: editor-90 선택 텍스트 발생 강조) · missing.

**(b) 원본 동작**(`MON/editor/contrib/multicursor/browser/multicursor.js` 688~900줄)
- 옵션(`editorOptions.js` 3353~3355줄): `selectionHighlight` 기본 true, `selectionHighlightMaxLength` 200(0은 무제한), `selectionHighlightMultiline` false.
- 일정(713~765줄): 비어 있지 않은 선택으로 바뀌면 즉시 갱신, 빈 선택으로의 명시적 이동이면 지우고 300ms 뒤 갱신, 내용 변경 300ms, 찾기 상태 변경 즉시.
- 조건(`_createState` 772~838줄): 여러 줄 선택이면 안 함, 다중 선택은 모두 같은 텍스트일 때만, 선택이 비어 있으면 안 함(단어 하이라이트에 맡김), 공백·탭만이면 안 함, 길이가 200을 넘으면 안 함, 찾기 위젯이 열려 있고 같은 검색어·옵션이면 안 함.
- 검색: 찾기 상태의 대소문자 구분·단어 단위(`MultiCursorSession.create` 230~270줄: 선택이 있으면 찾기 위젯의 `matchCase`·`wholeWord`)로 문서 전체 일치, 선택과 겹치는 일치는 제외(856~886줄). `model.isTooLargeForTokenization()`(20MB 초과 또는 30만 줄 초과)이면 안 함(852줄).
- 표시: 배경 `editor.selectionHighlightBackground`의 불투명도 0.5(`highlightDecorations.js` 104~109줄). 문서 하이라이트 공급자가 **없으면** overview(Center lane, `editorOverviewRuler.selectionHighlightForeground`)와 미니맵(`minimap.selectionOccurrenceHighlight`)도 표시, 있으면 배경만(59~75·100~103줄).

**(c) native 현재**: 색 값은 이미 계산합니다(`APP/editor-highlights.rs` 167~173줄 `editor.selectionHighlightBackground`·`editorOverviewRuler.selectionHighlightForeground`, 200줄 미니맵). 문서 크기 한도도 원본과 같은 값이 있습니다(`MAX_HIGHLIGHT_UTF16_LENGTH` 20Mi·`MAX_HIGHLIGHT_LINES` 30만, `is_highlight_model` 284줄). 그러나 선택 텍스트로 다른 발생을 찾는 경로는 없습니다: 문서 하이라이트 요청은 `has_word_selection`(369줄)으로 선택이 커서 단어 안에 있을 때만 단어 기준으로 동작합니다.

**(d) 구현 방법**: `APP/editor-selection-highlight.rs`(새 파일, 상태는 `domains::editor::State`): 매 프레임 view 선택과 찾기 상태를 보고 위 조건을 검사합니다. 일치 계산은 기존 찾기 모델(`FindQuery::new(&text, FindOptions { match_case, whole_word, .. }, &taide_native_syntax::MonacoFindPatternCompiler)` + `find_matches(&snapshot.rope, &[], limit, Some(timeout))`, 선례 `APP/editor-highlights.rs` `textual` 307~322줄)을 리터럴로 쓰고 시간 상한을 둡니다. 결과는 `DecorationLayer`(배경 `Inline { background }`) + 공급자가 없을 때만 `Overview { lane: Center, color, minimap: Some(color) }`로 만들어 `show_document`의 `find_layers` 체인(`APP/domains/editor.rs` 5864줄)에 추가합니다. 공급자 유무는 `lsp.highlight_providers(project, snapshot)`(`APP/lsp.rs` 906줄)가 비었는지로 판정합니다.

**(e) 자동 검사**: `[A]` `foo bar foo bar`에서 `foo b` 선택 → 다른 발생 1개 배경 rect·(공급자 없음) 미니맵 mark, 여러 줄 선택이면 없음, 공백만 선택이면 없음, 201자 선택이면 없음, 찾기 위젯이 같은 검색어면 없음, 하이라이트 mock 공급자가 있으면 미니맵 mark 없음.

**(f) 완료 판정**: 새 행 complete, editor-15의 미니맵 항목 근거 공유.

**(g) 막는 요소**: 없음.

#### D. 색 장식·색 선택기

**(a)** 행 없음(제안: editor-91 색 장식·색 선택기) · missing. editor-57 C6·J2와 editor-34 `showOrFocusStandaloneColorPicker`가 이것에 의존합니다.

**(b) 원본 동작**
- 공급자: CSS/SCSS·JSON 내장, 그리고 다른 색 공급자가 없는 모든 언어에 기본 공급자(`MON/editor/contrib/colorPicker/browser/defaultDocumentColorProvider.js` 46줄 `colorProvider.register('*', …)`, 계산 `MON/editor/common/languages/defaultDocumentColorsComputer.js`, `defaultColorDecorators` 기본 `auto` — `editorOptions.js` 3471줄). 그래서 원본은 `.ts`·`.rs`·`.md`의 `#ff0000`·`rgb()`에도 견본을 그립니다. 다른 공급자가 있으면 기본 공급자 제외(`color.js` 13~60줄).
- 표시: 색 값 앞 견본(`colorDecorators` 기본 true 3140줄, 최대 `colorDecoratorsLimit` 500개 3149줄). 견본 클릭/hover(`colorDecoratorsActivatedOn` 기본 `clickAndHover` 3141줄) → 색 선택기 hover(표현 순환 rgb·hsl·hex, 원래 색 복원, SV·색상·불투명도 띠), 선택하면 표현 텍스트로 편집.

**(c) native 현재**: core만 완료(`ED/color.rs`, `ED/default-colors.rs` `document_colors` 271줄). 앱 문서별 계산·표시·선택기 없음. 설정 화면용 색 선택기 `UI/theme-color-picker.rs`가 있습니다.

**(d) 구현 방법**(`QA/2026-11-02-native-lane-editor-css-services.md` 90줄 "남은 범위" 2번을 따름)
1. 앱 문서별 색 계산 상태(Normal 등급, 공급자 규칙: CSS/SCSS/JSON은 내장 공급자, 그 밖의 언어는 기본 공급자).
2. ED 표시 맵의 주입 항목(견본 폭)과 UI 행 배치·그리기(`UI/editor-color-decorators.rs` 신설, 기존 inlay hint 주입 경로 `ED/display-map.rs` 473줄 `set_inlay_hints`와 `UI/editor_surface.rs`의 `show_request_with_inlay_hints`(1392줄) 참고).
3. 견본 클릭/hover → 선택기(`UI/theme-color-picker.rs` 재사용), 표현 편집 적용(undo 1단위), readonly면 표시만.
4. editor-34 `showOrFocusStandaloneColorPicker` 연결.

**(e) 자동 검사**: `[U]` 견본 rect 색·위치, 500개 상한. `[A]` `.css`의 `color: #ff0000;` → 견본, 클릭 → 선택기, 표현 순환 클릭 → 텍스트가 `rgb(255, 0, 0)`으로 바뀜·undo, `.ts` 문자열 안 `#00ff00`에도 견본(기본 공급자).

**(f) 완료 판정**: 새 행 complete, editor-57 C6·J2의 표시 근거 공유.

**(g) 막는 요소**: CSS 색 공급(C6)은 CSS 파서(C1) 뒤. 기본 공급자 부분은 지금 바로 가능합니다.

#### E. 사양 1.4 목록 밖의 추가 발견(메인 확인 필요)

결정 2 A의 대상은 `SPEC` 1.4의 목록입니다. 아래 셋은 이번 확인에서 새로 찾은 원본 기본 기능이며 native에 없습니다. 새 행 추가 여부를 메인이 정해야 합니다(작은 결정이 아니라 범위 추가이므로 ai-process 3절 "멈춤" 대상).

| 기능 | 원본 근거 | native 현재 | 구현 요지 |
| --- | --- | --- | --- |
| 선택 텍스트 끌어 옮기기(drag and drop) | `dragAndDrop` 기본 true(`editorOptions.js` 3181줄), `MON/editor/contrib/dnd/browser/dnd.js` 75~180줄: 선택 안에서 누르고 끌면 삽입 위치 표식(`dnd-target`)과 커서, 놓으면 이동(macOS ⌥를 누르면 복사, `TRIGGER_KEY_VALUE` 25줄), `dragAndDropCommand.js`, undo 단위 1개 | 없음: `UI/editor-pointer.rs` `PointerSelection::update`(169줄)는 선택 안을 눌러도 새 선택을 시작함 | `PointerSelection`에 "선택 안에서 시작한 끌기" 상태를 추가하고 놓을 때 이동/복사 편집을 트랜잭션 1개로 적용. readonly·column 선택 모드면 비활성(원본 50·63줄) |
| 읽기 전용 편집 시도 안내 | `MON/editor/contrib/readOnlyMessage/browser/contribution.js` 1~36줄: 읽기 전용 편집기에서 입력하면 커서 위치에 "Cannot edit in read-only editor" 말풍선(MessageController). TAIDE는 `readOnly`만 넘기고 `readOnlyMessage`를 설정하지 않음 | 없음(`grep -rn "Cannot edit" <ROOT>/native` 결과 없음). 읽기 전용 배너(editor-36)는 있음 | 편집 거절(`EditorError::ReadOnly`)이 난 입력 이벤트에서 커서 위치 말풍선을 잠시 표시 |
| 비정상 줄 끝 문자 확인 | `unusualLineTerminators` 기본 `'prompt'`(`editorOptions.js` 3405줄), `MON/editor/contrib/unusualLineTerminators/browser/unusualLineTerminators.js` 55~95줄: 문서에 LS(U+2028)·PS(U+2029)가 있으면 확인 대화상자("Unusual Line Terminators" / "Remove Unusual Line Terminators" / "Ignore"), standalone 대화상자는 `window.confirm`(`standaloneServices.js` 165~177줄) | 없음 | 원본 화면(WKWebView의 `window.confirm` 표시 여부)을 실기로 먼저 확인해야 함. 표시되면 같은 문구의 확인 대화상자와 제거 편집 |

---

## 4. 이 영역에서 반복된 함정과 주의

1. **사양 문서와 실제 원본의 차이(이 문서에서 정정한 것)**: 사양을 그대로 믿지 말고 원본 파일을 열어 확인하십시오.
   - `SPEC` 2.2 semantic "fontStyle 미지정이면 TextMate 글꼴 유지" → 실제로는 네 글꼴 속성을 항상 덮음(3.1 (b)).
   - `SPEC` 2.2·2.0.4 semantic "테마 변경 시 즉시 재요청" → 실제로는 토큰 제거 뒤 적응형 지연(3.1 (b)).
   - `SPEC` 2.3 section header "줄 단위 일치" → MARK의 `\s`가 줄바꿈을 넘음(3.2 (b)). region marker는 `ED`가 아니라 `SYN`에 있음.
   - `SPEC` 2.19 PDF "배율 유지" → 실제로는 초기 배율(preview QA 43줄).
   - git 사양의 "revert 화살표" → Monaco 0.56 기본은 거터 메뉴 "Revert Block"(3.12 (b)).
   - 기능표 머리말 합계 표는 행 값과 다름(1.1).
2. **oracle 부채(bun 금지)**: 2026-11-02 lane workflow 셸 규칙(`docs/utils/2026-11-02-native-lane-workflow.js` 45줄)에 bun이 없어 JSON 완성·hover, CSS 포맷·접기, 기본 색, HTML 포맷·접기·선택 확장의 기대값을 손으로 계산했습니다. 손 계산 밖 분기(patternProperties 정규식, beautify의 `@`·`insideScssMap`·`grid-template-areas`, 속성 감싸기 정렬, `<!-- beautify ignore -->`)는 원본 대조가 없습니다(`QA/…-json-services.md` 68줄, `…-css-services.md` 91줄, `…-html-services.md` 96줄). bun 한 줄 실행이 허락되면 oracle을 먼저 만드십시오.
3. **앱 lib 병렬 흔들림**: 실제 App 검사를 많이 동시에 돌리면 host 큐(64) 포화·종료 대기 시간 초과로 실패합니다. `--test-threads=2`(lane)로 제한하고, 실패는 `--exact` 단독 재실행으로 구분합니다(`HO/05-process-and-gates.md` 9.3절 명령). 기존 실패: `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`(시스템 글꼴 fallback 체인 단언), 루트 workspace의 `taide-runtime/tests/agent_probe.rs` 소스 문자열 단언 1건(`81e96198`부터, `docs/PROCESS.md` W1 settings 병합 줄). 이 둘을 자기 작업의 실패로 오인하지 마십시오.
4. **diff 가상 문서와 내장 서비스**: git lane이 `DocumentKey::Virtual`을 추가했고, 내장 JSON 서비스가 `.json` diff 가상 문서를 처리해 `InvalidIdentity` 오류 toast가 반복되는 결함이 있었습니다(`GIT/docs/quality-assurance/2026-11-02-native-lane-git-g5-diff.md` 6.1). 부정 검사(`!matches!(snapshot.key, DocumentKey::AppFile(_))`)는 컴파일되지만 가상 문서를 흘려보냅니다. 새 내장 서비스(CSS 진단·Unicode·링크·TS 등)의 문서 선별 조건에는 `DocumentKey::AppFile(_) | DocumentKey::Virtual(_)`를 넣으십시오(`HO/05-process-and-gates.md` 228~229줄).
5. **내장 진단 게시 집합**: `self.lsp_diagnostics.retain_builtin(&self.store, self.editor_json_validation.documents())`(`APP/domains/editor.rs` 2611~2612줄)는 넘긴 집합 밖의 내장 진단을 지웁니다. CSS·TS 진단을 추가하면 문서 집합을 합쳐 넘기십시오. 같은 이유로 심볼은 `retain_builtin(&symbol_documents)`(2652줄)가 `provides_symbols`로 거르므로 새 언어의 심볼은 `provides_symbols`에 함께 넣어야 합니다.
6. **mock 실행 파일**: target을 비우면 앱 lib의 LSP 검사 10건 안팎이 `native-lsp-mock` 실행 파일이 없어 실패합니다(`connect_lsp_mock`의 `assert!(mock.is_file())`). 0.3의 1번을 먼저 실행합니다.
7. **실패 먼저 확인**: lane QA들은 새 `[A]`를 연결 전에 실행해 의도한 실패(대기 시간 초과 등)를 확인하고, 제품 코드를 잠시 되돌려 새 검사가 실패하는지 본 뒤 복구했습니다(preview QA 143~158줄). 같은 방식을 따르되, 되돌린 코드가 남지 않았는지 `git -C <ROOT> diff --stat`로 줄 수를 대조하십시오.
8. **판정 방식**: 미리보기 `[A]`는 accesskit label보다 본문 글자 shape와 `Shape::Mesh`의 `texture_id`로 판정해야 안정적입니다. 보조 창 검사는 view 생성 전에 조회하면 실패하므로 생성 대기를 넣습니다. 팔레트 첫 행이 기대 명령이 아닐 수 있으므로 키 경로(F12 등)를 직접 쓰는 편이 안전합니다(`QA/…-display-commands.md` 70줄).
9. **UI 하네스의 텍스처**: 텍스처를 만드는 `[U]`는 프레임 사이에 `textures_delta.clear()`가 없으면 panic합니다(preview QA 94줄).
10. **브라우저 기준**: 원본은 macOS WKWebView(WebKit)입니다. 배치 23 Markdown 비교는 headless Chrome으로 했습니다(`QA/2026-10-12-native-batch23-markdown.md` 143·171·227줄). input·details 같은 기본 모양은 엔진마다 다르므로 WebKit(Safari) 기준으로 기대값을 만드십시오.
11. **이미지 Decode 범위**: `Failure::Decode`에는 깨진 이미지뿐 아니라 자원 한도 초과(파일 크기 예산, 64MiB 픽셀 예산, 애니메이션 예산, 텍스처 캐시 가득)도 들어가 파일 이름만 보입니다(preview QA 111줄). 6000×4000 JPEG처럼 원본(브라우저)은 정상 표시하는 경우가 있습니다. 여러 decoder에 한도 오류 분류가 필요해 메인 결정으로 넘어가 있습니다. 미리보기를 고칠 때 이 결정이 났는지 먼저 확인하십시오.
12. **Alt 조합 글자**: macOS에서 Option 키 조합은 키 이벤트 뒤에 별도 `Event::Text`(예: `‘`)가 옵니다. 키를 소비하면 다음 index의 Text도 소비해야 합니다(`UI/editor-completion.rs` `State.inline_claimed_text`, `QA/…-display-commands.md` 95~107줄). 새 단축키(inPlaceReplace ⇧⌘, 등은 ⌘ 조합이라 해당 없음, ⌥ 계열 추가 시 해당)도 확인하십시오.
13. **`evaluate_script` 성공 판정**: wry `evaluate_script`의 `Ok`는 JS 실행 결과가 아닙니다(3.14 (c)). 결과가 필요하면 `evaluate_script_with_callback`을 쓰고 콜백 값이 JSON 직렬화 문자열임에 주의하십시오.
14. **경로 정규화**: macOS 임시 디렉터리는 `/var`와 `/private/var`가 섞여 탐색기 행 경로·git 경로 비교가 실패합니다. fixture들은 `canonicalize()`한 경로를 씁니다(`APP/application-inlay-hints-tests.rs` 291~293줄). 단언도 canonicalize한 경로로 하십시오.
15. **공용 파일 충돌**: `application.rs`의 테스트 모듈 등록, `lib.rs` 모듈 목록, 기능표 상단 합계·범례, `application-json-tests.rs` 하위 모듈 등록은 여러 lane이 같은 줄을 고쳐 병합 충돌이 납니다. 모듈 목록은 알파벳 위치에 한 줄, 합계 표는 건드리지 말고 메인에 맡기십시오(seams 1.4).
16. **`include!` 복제 컴파일**: `APP`의 일부 파일(예: `APP/keymap.rs`)은 `cfg(test)`에서 UI 소스를 `include!`로 복제 컴파일합니다(`/Users/hyunseokbyun/development/TAIDE/docs/memory/2026-10-07-native-transition-operations.md` 6절). UI의 키맵·명령 코드를 바꾼 뒤 앱 검사가 다른 컴파일 단위를 검증할 수 있으니 UI·앱 검사를 모두 돌리십시오.
17. **휴지통 실행 사고**: `APPT/explorer-delete.rs`는 실제 휴지통에 파일을 넣습니다. 과거에 작업자가 두 번 실행해 사용자 휴지통에 합성 파일이 들어갔습니다(`docs/PROCESS.md` 배치 24 D). 병합 게이트 스크립트도 휴지통 검사를 포함하므로(`HO/05-process-and-gates.md` 9.1) 이 영역 작업자는 대상 지정 명령만 쓰십시오.
18. **외부 crate API 확인 절차**: 사양은 `cargo doc` 산출물로 확인하라고 했지만(`SPEC` 2.1·2.0.2), lane workflow 규칙은 cargo registry의 crate 소스 읽기를 예외로 허용합니다(`docs/utils/2026-11-02-native-lane-workflow.js` 45줄). editor-4 조사가 registry 소스를 인용한 절차 차이를 QA에 기록했습니다(`QA/…-display-commands.md` 129줄). 작업 지시가 허용하는 쪽을 먼저 확인하고 QA에 어떤 방법을 썼는지 적으십시오.
19. **완료 판정 과대**: 리뷰는 "검사 함수 수", "자동 근거의 한계(실행되지 않은 경로)"를 과대 판정으로 지적했습니다(preview QA 120~128줄). 기능표 비고에는 실제로 실행된 검사만 적고, 실행되지 않은 경로(예: `tick_preview_media` → WKWebView)는 남은 범위로 명시하십시오.
20. **peek·mirror 표시 일치**: 편집기 표시 옵션을 문서별로 넣을 때 peek는 `editor_presentation.clone()`(`APP/domains/editor.rs` 5634줄)으로 그려지므로 본문 문서의 값을 그대로 받습니다. semantic·section header처럼 문서별 값은 문서 id와 revision을 함께 들고 UI에서 대조하고, peek에는 peek 문서의 값을 따로 넣으십시오(3.1 (d) 6-4).
21. **LSP 기능은 File 문서만**: `APP/lsp.rs` `plans`(1310줄)가 `DocumentKey::File`만 세션을 만들므로 Untitled·AppFile·Virtual 문서에는 LSP 공급자가 없습니다. LSP 기반 새 기능의 대상 조건을 Untitled까지 넓히지 마십시오.
