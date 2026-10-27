# lane editor-preview 사양 — 편집기·미리보기 남은 23행

> 작성일: 2026-10-28 · 기준 커밋 `72065136`(branch `to_rust_native`, 단계 0의 미커밋 작업 트리 관찰) · 운영 방식 `docs/memory/2026-10-28-native-lane-operations.md`
> 이 문서는 lane 순서 1(사양) 산출물입니다. 구현 작업자는 이 문서와 여기서 가리키는 파일 구간만 읽고 원본 TS를 처음부터 다시 조사하지 않습니다.
> 단계 0(영역 모듈 분리)이 진행 중이라 `application.rs`·`host.rs`(native-app), `commands.rs`·`shell.rs`(native-ui)는 줄 번호 없이 심볼 이름으로만 인용합니다. `docs/research/2026-10-28-native-lane-seams.md`는 작성 시점에 없었습니다. 생기면 그 문서의 이름(모듈·enum·trait)을 우선하고, 이 문서의 `domains/editor.rs`·`domains/preview.rs`·`HostCommand::Editor`·`HostCommand::Preview`는 그 이름으로 바꿔 읽습니다.
> 겹치는 lane 사양: `2026-10-28-lane-git-spec.md`(editor-73~76), `2026-10-28-lane-explorer-search-spec.md`(editor-81), `2026-10-28-lane-shell-spec.md`(shell-5의 `preview_web_view.rs`, shell-78), `2026-10-28-lane-settings-spec.md`(settings-28).

## 0. 약어·검사 표기·실행 명령

| 약어 | 절대 경로 |
| --- | --- |
| `TS` | `/Users/hyunseokbyun/development/TAIDE/src` |
| `CR` | `/Users/hyunseokbyun/development/TAIDE/crates` |
| `APP` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src` |
| `UI` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/src` |
| `ED` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-editor/src` (portable core, wasm32 컴파일 대상) |
| `SYN` | `/Users/hyunseokbyun/development/TAIDE/native/taide-native-syntax/src` |
| `WEB` | `/Users/hyunseokbyun/development/TAIDE/native/taide-remote-web` (동결, 컴파일만 유지) |
| `MON` | `/Users/hyunseokbyun/development/TAIDE/node_modules/monaco-editor/esm/vs` (설치 Monaco 0.56) |
| `MEXT` | `/Users/hyunseokbyun/development/TAIDE/node_modules/monaco-editor/esm/external` (Monaco가 번들한 vscode-*-languageservice) |
| `LOC` | `/Users/hyunseokbyun/development/TAIDE/crates/taide-locale/resources/locales/{en,ko,ja}.json` |
| `MOCK` | `/Users/hyunseokbyun/development/TAIDE/experiments/lsp-coordinator-spike/src/bin/mock-server.rs` (앱 example `native-lsp-mock`, `--native-*` 모드) |

검사 표기

- `[C]` crate 단위: `ED`·`SYN`의 `tests/*.rs`, 원본 대조는 기존 oracle 방식(`native/taide-native-editor/tests/fixtures/<이름>-oracle.mjs`를 bun으로 설치된 `MEXT`·`MON`에 돌려 `<이름>-oracle.rs` 생성, Microsoft MIT 출처 주석은 fixture 머리에)을 그대로 씁니다. 선례 `json-format-oracle.mjs`/`.rs`.
- `[U]` native-ui 하네스: `native/taide-native-ui/tests/*.rs`(필요 시 `--features inspection`)의 egui 프레임 검사. 선례 `tests/editor-minimap.rs`, `tests/editor-markup-preview.rs`, `tests/editor-inlay-hints.rs`.
- `[A]` native-app lib 실제 App: `NativeApplication`을 kittest 프레임으로 돌리는 `APP/application-*-tests.rs`. 선례 `application-json-tests.rs`, `application-markdown-tests.rs`, `application-tab-menu-tests.rs`, `application-auxiliary-window-tests.rs`, `application-ai-completion-tests.rs`.
- `[L]` 실제 LSP child: `MOCK`에 새 `--native-<기능>` 모드를 더해 `APP/lsp-editor-*-tests.rs`에서 실행합니다. 선례 `lsp-editor-code-lenses-tests.rs`(가짜 서버 실행 파일을 임시 bin 디렉터리에 쓰고 `LspBridge::connect`).

실행 명령(모두 lane worktree에서 `CARGO_TARGET_DIR=<worktree>/target-lane`, `--locked --offline`)

- 편집기 core: `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test <이름>`, wasm 확인 `cargo check --manifest-path native/taide-native-editor/Cargo.toml --lib --target wasm32-unknown-unknown`
- 구문: `cargo test --manifest-path native/taide-native-syntax/Cargo.toml --test <이름>`
- UI: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features inspection --test <이름>`
- 앱 lib: `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- <필터> --test-threads=4`
- 동결 미러(ED·UI·SYN 공개 타입을 바꾼 묶음마다, 이전 배치와 같은 두 명령): `cargo check --manifest-path native/taide-remote-web/Cargo.toml --tests --features inspection`, `cargo check --manifest-path native/taide-remote-web/Cargo.toml --lib --features canvas --target wasm32-unknown-unknown`
- 형식: 바꾼 crate마다 `cargo fmt --manifest-path <crate>/Cargo.toml -- --check`

금지(모든 검사): GUI 실행, 실제 clipboard·Keychain·OS 알림·Finder/외부 앱 열기(주입 포트 사용), 실제 AI provider, 네트워크, 실제 사용자 데이터 경로, `tests/explorer-delete.rs` 실행, 저장소 밖 탐색, git 상태 변경, 허락 없는 새 의존성·vendor 패치·manifest/lock 변경.

---

## 1. 요약

### 1.1 수와 상태

| 현재 판정 | 수 | ID |
| --- | --: | --- |
| partial | 15 | editor-15, 16, 32, 34, 49, 57, 58, 72, 77, 78, preview-5, 6, 14, 15, 38 |
| missing | 8 | editor-4, 7, 73, 74, 75, 76, 81, preview-3 |
| 합계 | 23 | 배치 23 잔여 17(editor) + 배치 30 잔여 6(preview) |

일의 성격으로 다시 나누면 다음과 같습니다.

| 성격 | ID | 수 |
| --- | --- | --: |
| 이 lane이 새로 구현 | editor-4, 7, 15, 34, 72, preview-6, 14, 15 | 8 |
| 이 lane의 내장 언어 서비스 묶음(별도) | editor-57, 그리고 그 공급에 막힌 16, 32, 49 | 4 |
| 기존 구현의 증거 보강·작은 보완 후 판정 | editor-58, 77, 78, preview-3, 5 | 5 |
| 사용자 결정 반영 | preview-38 | 1 |
| 다른 lane에 위임(이 lane은 행을 함께 갱신만) | editor-73, 74, 75, 76(git lane), editor-81(explorer-search lane) | 5 |
| 합계 | | 23 |

### 1.2 배치 23·30 진행 기록과의 대조

`docs/PROCESS.md`의 `## 진행: 배치 23` 절과 배치 23 QA(`2026-10-10-native-batch23-editor.md`, `2026-10-14-native-batch23-builtin-language-services.md`, `2026-10-12-native-batch23-markdown.md`, `2026-10-13-native-batch23-ai.md`, `2026-10-16-native-json-schema-app.md`)를 대조한 결과입니다.

- 끝난 하위 작업(다시 하지 않음): b1·b2(고정 줄 정의 이동, editor-30 들여쓰기 전체), b3a~b3h(JSON scanner·AST·선택 확장·심볼·접기·포맷·구문/schema 진단·구조 공급·실제 App 진단/포맷/선택 확장), c1~c9(하이라이트·외부 포맷·입력/붙여넣기 포맷·rename·code action·inlay·code lens·selection range·컨텍스트 메뉴), d1~d5(터미널 실행·IDE 선택·뷰 상태·로딩 화면·삭제 draft), d6a·d6b·d6c1·d6c2·d6c3a~h4(Markdown 분할·수명·HTML 파서·표/목록/코드 박스 대부분), e1a~e1e·e2(AI 완성·AI 편집 전체 흐름), e3(Emmet), f1(프로젝트 열기 지연 조사, 배치 31 이관).
- 남은 것: b3의 JSON hover/completion/colors와 CSS·SCSS·HTML·TS 내장 서비스, c의 editor-32/34/49/57/58 잔여, d의 editor-72 잔여(d6c3h 부모)와 73~76·81, e1 체크 표시(하위 e1a~e1e 완료, 부모만 미체크), f(전체 게이트), g(완료 근거·선별 Git).
- PROCESS 배치 23 절의 `다음 행동:` 문장은 e3d 시점에 멈춰 있어 실제 상태와 다릅니다. 이 lane의 첫 묶음이 끝날 때 lane 기록으로 대체합니다(PROCESS 자체는 메인 소유).

### 1.3 대응표 정정 후보(구현 전에 근거와 함께 메인에 보고)

- **preview-3**: shell-78 작업(`2026-10-27-native-tab-icons-menu.md`)이 파일 탭 메뉴의 `tab.reopenEditorWith` 하위 메뉴(Editor/Preview → `open_with::Registry::set`)를 이미 연결했고 주 창 App 검사(`APP/application-tab-menu-tests.rs`의 reopen 사례)가 있습니다. missing은 낡은 판정입니다. 보조 창 검사 1건만 남습니다.
- **editor-58**: 원본 opener의 untitled 분기는 다른 모델로의 이동에서만 쓰입니다. 원본에서 untitled 대상이 생기는 경로는 문제 이동(`marker.nextInFiles`)뿐이고 이는 b3g2b가 실제 File/Untitled 키로 연결했습니다. 남은 것은 진입별 근거 정리와 창 범위 검사입니다.
- **editor-77·78**: 기능 흐름은 완료됐고 partial 사유는 전체 App 준비 실패(배치 31 환경 부채)와 실기입니다. 기능표 complete 정의(구현+실제 앱 경로+해당 동작 자동 검사, OS/출시 제외)에 따르면 판정 대상입니다. 다만 아래 1.4의 inline suggest hover toolbar·`⌘→` 다음 단어 수락이 원본에 있어 editor-77은 그 구현 뒤에 닫습니다.
- **editor-15**: 비고의 "SCM 표시 공급"은 원본에 없습니다. 원본 git gutter 장식(`TS/widgets/editor-pane/use-editor-git-gutter-and-conflicts.ts`)은 `minimap` 옵션을 주지 않으므로 Monaco 미니맵에 SCM 표시가 없습니다. 실제 남은 것은 2.3의 section header·접힌 영역 배경·선택 텍스트 발생 표시입니다.
- **editor-57**: 비고의 "언어 선택 UI"는 원본에 없습니다(`rg changeLanguage|languageMode|selectLanguage TS`: 내부 `setModelLanguage`뿐). 남은 범위에서 빼고 근거를 적습니다.
- **HTML 내장 서비스 표**(`2026-10-14-native-batch23-builtin-language-services.md` 9~14줄): HTML은 진단·colors 공급자를 등록하지 않습니다(`MON/languages/features/html/htmlMode.js`의 `registerProviders`에 diagnostics·colors 없음). 그 표의 "HTML diagnostics·colors"는 정정 대상입니다.

### 1.4 원본에 있으나 대응표에 행이 없는 기능(발견)

모두 설치 Monaco의 기본 동작이며 TAIDE가 끄지 않습니다(`TS/features/editor/code-editor.tsx:180-201`은 `semanticHighlighting.enabled`만 바꿈). editor-34의 액션 실행이 이 기능들을 전제로 하므로 4절 결정 2로 처리 범위를 정합니다.

| 기능 | 원본 근거 | 연결된 ID |
| --- | --- | --- |
| Unicode 강조(모호·보이지 않는 문자 상자, hover 안내, 끄기 액션) | `MON/editor/contrib/unicodeHighlighter/browser/unicodeHighlighter.js`, 기본값 `MON/editor/common/config/editorOptions.js:2074-2093`(ambiguous/invisible true, nonBasicASCII·includeComments는 `inUntrustedWorkspace`→standalone 신뢰 상태에서 false) | editor-34(`unicodeHighlight.*` 4개) |
| 텍스트 링크 감지(URL 밑줄, `⌘`+클릭, "Follow link" hover, `editor.action.openLink`) | `MON/editor/browser/services/editorWorkerService.js:69-78`(모든 언어 `$computeLinks`), `MON/editor/common/languages/linkComputer.js`, `MON/editor/contrib/links/browser/links.js` | editor-34(`openLink`), editor-57 HTML links |
| 색 장식·색 선택기(색 값 앞 견본, 클릭 시 선택기, 표현 순환) | `MON/editor/contrib/colorPicker/browser/*`, `colorDecorators` 기본 true. 공급자는 CSS/SCSS·JSON 내장과, 다른 공급자가 없는 모든 언어에 붙는 기본 공급자(`colorPicker/browser/defaultDocumentColorProvider.js:43-46`, 계산 `MON/editor/common/languages/defaultDocumentColorsComputer.js`, `defaultColorDecorators` 기본 `auto` `editorOptions.js:3471`) — 즉 원본은 .ts·.rs·.md 등의 `#ff0000`·`rgb()`에도 견본을 그립니다 | editor-34(`showOrFocusStandaloneColorPicker`), editor-57 |
| inline suggest hover toolbar(이전/개수/다음·Accept·Accept Word·Accept Line·Always Show Toolbar)와 `⌘→` 다음 단어 수락 | `MON/editor/contrib/inlineCompletions/browser/inlineCompletions.contribution.js:31`, `.../controller/commands.js:121-165, 313-330`, `showToolbar` 기본 `onHover`(`editorOptions.js:2210`) | editor-77, editor-34(`inlineSuggest.toggleAlwaysShowToolbar`) |
| 선택 텍스트 발생 강조(SelectionHighlighter) | `MON/editor/contrib/multicursor/browser/multicursor.js:688-`, `selectionHighlight` 기본 true | editor-15(미니맵 `minimapSelectionOccurrenceHighlight`) — native 본문에 이미 있는지 먼저 확인(아래 2.3) |

### 1.5 다른 lane 의존

| 필요한 것 | 제공 | 시점 | 영향 ID | 없을 때 |
| --- | --- | --- | --- | --- |
| `domains/editor.rs`·`domains/preview.rs`, `HostCommand::Editor/Preview` 하위 enum, `ShellIntent` 영역 하위 enum, 영역 trait | 단계 0(메인) | wave 0 | 전부 | 시작하지 않음 |
| 탭 내용 분기(`ShellSurfaces::tab_content` 구현)에서 `TabKind::SearchEditor`·`Diff`·`CommitDiff`·`ClaudeDiff`를 영역 렌더러로 넘기는 hook | 단계 0 권장. 단계 0이 못 하면 이 lane 묶음 E1의 첫 작업 | wave 1 시작 전 | editor-81(explorer-search), editor-76(git) | explorer-search lane 36번과 충돌. 4절 결정 5 |
| `preview_web_view.rs`의 viewport별 분리(보조 창 WKWebView) | shell lane(shell-5) | wave 1 동시 | preview-14, 15, 38 | 이 lane은 그 파일의 구조를 바꾸지 않고 파일 끝 영역 블록에 메서드만 추가. 다중 창 처리는 하지 않음 |
| 편집기 표면의 git 확장(`EditorOutput.lane_press`, filler zone, `DecorationKind::AfterText`, `DocumentKey::Diff`) | git lane(G5·G8) | wave 2 | editor-73~76 | 이 lane은 만들지 않음. git lane이 이 lane 병합 뒤 추가 |
| 검색 편집기 본문 | explorer-search lane(D4 A안) | wave 1 동시 | editor-81 | D4가 B안이면 이 lane 묶음 E2에 추가 |
| settings-28 소비자 목록(ligature·semantic) | 이 lane이 제공 | wave 1 | settings-28(wave 2) | settings lane이 이 lane 결과를 비고에 반영 |

---

## 2. ID별 사양

### 2.0 공통 설계

#### 2.0.1 파일 배치

새 파일은 native-app의 kebab-case + `#[path]` 관례를 따릅니다.

| 위치 | 내용 | ID |
| --- | --- | --- |
| `APP/domains/editor.rs` | 이 lane의 편집기 영역 진입점: 상태 묶음 호출, host 응답 반영, frame tick, 편집기 요청 조립에 새 layer 추가 | 공통 |
| `APP/domains/preview.rs` | 미리보기 영역 진입점: `reconcile_previews`, 미리보기 탭 내용 분기, 외부 열기 | preview 전부 |
| `ED/semantic-tokens.rs` | legend 대응·5-tuple decode/re-encode·delta 적용·편집 추적 sparse token | editor-7 |
| `ED/section-headers.rs` | `findSectionHeaders` 이식(MARK·region) | editor-15 |
| `ED/link-computer.rs` | Monaco `linkComputer.js` 이식 | editor-34(결정 2) |
| `ED/unicode-highlight.rs` | `UnicodeTextModelHighlighter` 이식 | editor-34(결정 2) |
| `ED/inplace-replace.rs` | `BasicInplaceReplace` 이식 | editor-34 |
| `ED/json-completion.rs`, `ED/json-hover.rs`, `ED/json-colors.rs` | JSON 잔여 | editor-57(E3) |
| `ED/css-*.rs`, `ED/html-*.rs`, `ED/web-data/*.rs`(생성) | CSS/SCSS·HTML 서비스 | editor-57(E4·E5) |
| `ED/color.rs` | 색 파싱·표현(rgb/hsl/hex) 공용 | editor-57, 34 |
| `SYN/token-theme.rs`(추가만) | `TokenTheme::semantic_style_table()` | editor-7 |
| `APP/lsp-editor-semantic-tokens.rs`, `APP/editor-semantic-tokens.rs` | typed 요청·캐시 / 문서별 상태·일정 | editor-7 |
| `APP/editor-css.rs`, `APP/editor-html.rs`, `APP/editor-colors.rs` | 내장 공급자(`editor-json.rs` 방식) | editor-57 |
| `UI/editor-row-text.rs`(추가만) | `RowText::restyle`(semantic 색·글꼴 스타일 덮어쓰기), 리거처 옵션 전달 | editor-4, 7 |
| `UI/editor-minimap.rs`(추가만) | section header 표시, 접힌 영역 배경 | editor-15 |
| `UI/editor-inline-suggest-toolbar.rs` | ghost hover toolbar | editor-77 |
| `UI/editor-color-decorators.rs` | 색 견본·선택기 hover(`UI/theme-color-picker.rs` 재사용) | editor-57, 34 |

공용 파일(`application.rs`, `lib.rs`의 모듈 목록, 공용 enum 루트, `LOC`)은 각 파일의 영역 블록 끝에 추가만 합니다. 이 lane이 손대지 않는 파일: `preview_web_view.rs`의 구조(shell lane), `explorer.rs`(explorer-search lane), `settings-controls.rs`·설정 화면(settings lane), git 확장 지점(git lane).

#### 2.0.2 Host 명령

- 새 Host 명령은 적습니다. LSP 요청은 기존 `LspBridge`(APP/lsp.rs) 경로, 내장 서비스는 기존 `editor-json.rs`처럼 감독 worker(`services.tasks`) 문서 작업, 외부 열기는 이미 있는 셸 명령(단계 0 뒤 `HostCommand::Shell(domains::shell::Command::OpenPath)`, 응답 `Reply::SystemFinished`에서 오류 toast)을 씁니다.
- 추가가 필요한 것: 서식 있는 복사(`clipboardCopyWithSyntaxHighlightingAction`)용 HTML+텍스트 clipboard 쓰기. 기존 `write_clipboard` 주입 포트에 html 변형을 더합니다(`HostCommand::Editor(EditorCommand::CopyRich { html, text })`). arboard 3.6.1의 HTML 쓰기 API 유무는 lane target의 `cargo doc -p arboard --no-deps` 생성물로 확인하고, 없으면 텍스트만 복사하고 4절 결정 3에 올립니다.

#### 2.0.3 표시 공통

- 편집기 위젯 안의 Monaco 문구는 원본처럼 영어 고정입니다(원본은 Monaco nls를 설정하지 않음: `rg monaco-editor/esm/nls TS` 결과 없음). 기존 native 편집기 위젯(code action 목록 등)의 영어 문구 처리 방식을 따릅니다.
- TAIDE 문구는 `LOC` 키를 씁니다. 이 lane에서 쓰는 기존 키: `preview.notSupported`, `preview.openExternally`, `tab.reopenEditorWith`, `explorer.openWithEditor`, `explorer.openWithPreview`, `editor.toggleMarkdownPreview`, `editor.toggleMinimap`, `settings.editorFontLigatures`, `settings.editorSemanticHighlighting`(모두 ko.json에 있음).

#### 2.0.4 원본 상수

| 원본 | 값 | 출처 |
| --- | --- | --- |
| semantic tokens 요청 지연 | 최소 300ms·최대 2000ms(적응형), 열기·언어 변경·공급자 변경·refresh는 0ms | `MON/editor/contrib/semanticTokens/browser/documentSemanticTokens.js:119-166` |
| section header 계산 지연 | 내용 변경 250ms, 토큰 변경 1000ms, 최초 0ms | `MON/editor/contrib/sectionHeaders/browser/sectionHeaders.js:65-77` |
| section header 표시 | 글꼴 9px·자간 1·굵기 500·배경 높이 글꼴×1.5, MARK 정규식 `\bMARK:\s*(?<separator>-?)\s*(?<label>.*)$`, 100줄 묶음·5줄 겹침 | `MON/editor/common/config/editorOptions.js:1515-1530`, `MON/editor/browser/viewParts/minimap/minimap.js:1394-1420`, `MON/editor/common/services/findSectionHeaders.js:8-10` |
| 글꼴 zoom | 수준 −5..20, 배율 1+0.1×수준(글꼴·줄 높이), 창(JS realm) 전역·비영속 | `MON/editor/common/config/editorZoom.js:7-23`, `.../fontInfo.js:34-36` |
| inline suggest toolbar | `showToolbar` 기본 `onHover` | `editorOptions.js:2210` |
| Markdown 미리보기 | 200ms 지연 갱신, `px-6 py-4 text-sm` 등 Tailwind 클래스 | `TS/features/editor/markdown-preview.tsx:5-22`, `TS/widgets/editor-pane/use-editor-markdown-preview.ts` |
| 미리보기 상태 화면 | 아이콘 40px(`size-10`), 간격 12px(`gap-3`), 여백 16px(`p-4`), 파일 이름 12px·70% 불투명·최대 448px | `TS/features/preview/unsupported-preview.tsx:15-24` |
| 오디오 | 아이콘 40px, 파일 이름 14px, controls 폭 100%·최대 448px, 간격 16px | `TS/features/preview/audio-preview.tsx:9-15` |

#### 2.0.5 검사 공통

- 원본 결함은 재현하지 않습니다. 재현하지 않기로 한 결함은 해당 QA에 원본 근거와 함께 적습니다.
- 성공한 같은 검사를 반복 합산하지 않고, 실패를 고친 뒤에는 영향 검사만 다시 돌립니다(운영 문서 4절).
- ED·UI·SYN의 공개 타입을 바꾸면 동결 미러 두 명령을 그 묶음에서 1회 확인합니다.

---

### 2.1 editor-4 리거처 (missing)

- 원본: `TS/features/editor/code-editor.tsx:282-321`의 `updateOptions({ fontLigatures, ... })`(290줄), 값은 `settings.editorFontLigatures`(`TS/shared/lib/code-editor-settings.ts:43-65`, 모델 기본 false `CR/taide-model/src/settings.rs:554`). Monaco `EditorFontLigatures`(`MON/editor/common/config/editorOptions.js:577-579`): false → CSS `font-feature-settings: "liga" off, "calt" off`, true → `"liga" on, "calt" on`. 편집기 본문 글자에만 적용됩니다(미니맵·터미널 아님). 설정 UI는 원본 `TS/widgets/settings-view/settings-editor-section.tsx`.
- native 현재: 설정 컨트롤과 patch(`UI/settings-code-controls.rs` `CodeSwitch::EditorFontLigatures`)만 있고 소비자가 없습니다. 본문 글자는 egui `LayoutJob`(`UI/editor-row-text.rs` `RowText::styled_layout_job`)으로 그리고, 사용 중인 epaint 0.36.2는 harfrust로 shaping합니다(`native/taide-native-ui/Cargo.lock` epaint 의존). 그래서 "egui 한계" 기록(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 미결 절)은 harfrust 이전 판단일 수 있습니다. 기본 글꼴(SF Mono/Menlo, `APP/editor-fonts.rs`)에는 리거처가 없지만 사용자가 Fira Code 등을 지정하면 차이가 보입니다.
- 빠진 것: 설정→레이아웃 소비, 기본 OFF 보장.
- 설계:
    1. 조사: lane target에서 `cargo doc --manifest-path native/taide-native-ui/Cargo.toml -p epaint --no-deps`로 만든 소스 페이지(`target-lane/doc/src/epaint/text/`)에서 harfrust 호출이 OpenType feature를 받는지, `TextFormat`·`FontTweak`·`FontData` 중 어디로 넘기는지 확인합니다. 저장소 밖 registry 소스를 직접 읽지 않습니다.
    2. API가 있으면: `UI/editor_surface.rs`의 `EditorDisplayOptions`에 `font_ligatures: bool`을 더하고, `RowText::styled_layout_job`이 그 값으로 `liga`·`calt`를 함께 켜고 끕니다. 본문·peek·고정 줄·inline hint·ghost 등 편집기 글자 경로 전부에 같은 옵션을 넘깁니다. galley 캐시 키에 포함합니다. 설정 변경은 기존 `APP/presentation-refresh.rs` 경로로 다음 프레임에 반영합니다.
    3. API가 없으면 4절 결정 1.
- 검사: `[U]` `tests/editor-display.rs`에 옵션별 LayoutJob(또는 shaping 결과 glyph 수) 단언. 저장소에 리거처 글꼴이 없어 실제 glyph 병합 비교는 4.2 허락(사용자 글꼴 실기) 항목입니다. `[A]` 설정 토글 뒤 편집기 옵션 반영.
- 완료: 결정 1 A면 `[U]`·`[A]` 통과로 complete, settings-28 비고의 ligature 항목을 같은 근거로 갱신. B면 근거와 함께 not-applicable로 정정.

### 2.2 editor-7 semantic token 강조 (missing)

- 원본:
    - 공급자: `TS/shared/lib/lsp/adapters/semantic-tokens.ts:184-247`. 서버가 `semanticTokensProvider.full`과 legend를 광고할 때만 등록. `full.delta === true`면 같은 문서의 캐시 `resultId`가 Monaco의 `lastResultId`와 같을 때 `textDocument/semanticTokens/full/delta`, 아니면 `full`. delta 편집은 역순 splice(85-86). 응답은 legend 대응으로 다시 인코딩(46-70, 108-122): 서버 type 이름을 `SEMANTIC_TOKEN_TYPE_MAP`(`TS/shared/lib/theme-convert/semantic-token-map.ts:32-96`, `Object.hasOwn` 정확 일치)으로 31개 중 21개 syntax token에 대응, 대응 없는 type은 버리고 다음 토큰의 delta를 다시 계산, modifier는 표준 10개(`TS/shared/lib/lsp/protocol.ts:245-256`)만 유지.
    - 설정: `settings.editorSemanticHighlighting ?? true`를 요청 때마다 읽고 꺼져 있으면 캐시를 지우고 빈 결과(208-211). 설정 변경은 `triggerSemanticTokensRefresh`로 즉시 재요청(`TS/widgets/editor-pane/use-lsp-session.ts:47, 75-90, 119-130`).
    - refresh: 세션별 `workspace/semanticTokens/refresh` 처리기가 그 client의 공급자만 다시 요청(`TS/entities/lsp/lsp-session-registry.ts:348-360`), 등록은 `ensureLanguageRegistered`(663-683).
    - initialize: `workspace.semanticTokens.refreshSupport: true`(`TS/shared/lib/lsp/initialize-params.ts:86`), `textDocument.semanticTokens = { requests: { full: { delta: true } }, tokenTypes: 표준 23, tokenModifiers: 표준 10, formats: ['relative'], overlappingTokenSupport: false, multilineTokenSupport: false, serverCancelSupport: false, augmentsSyntaxTokens: true }`(129-138).
    - 색: 테마에 `taideSemantic.<token>` 규칙(`TS/shared/lib/shiki/build-shiki-theme.ts:67, 91-99`), 색은 `syntax[token]`. Monaco는 `[type, ...modifiers].join('.')`를 접두 일치로 찾으므로 결국 type 규칙만 적용됩니다. 규칙에 fontStyle이 없으면 TextMate 글꼴 스타일을 유지하고 전경색만 바꿉니다.
    - Monaco 일정(`MON/editor/contrib/semanticTokens/browser/documentSemanticTokens.js:117-300`): 편집기에 붙은 모델만 요청, 진행 중 요청 1개, 진행 중 내용 변경은 결과 적용 뒤 재예약, null 결과는 토큰 제거, 오류는 기존 토큰 유지 후 변경이 있었으면 재예약, 테마 변경 시 토큰 제거 후 재요청, 공급자 여러 개면 순서대로 첫 non-null 결과. 편집 중에는 기존 토큰을 편집 위치에 맞춰 이동합니다.
- native 현재: SDK typed 요청 `SemanticTokensFullRequest`·`SemanticTokensFullDeltaRequest`·`SemanticTokensDeltaResult`(`CR/taide-lsp/src/native/feature.rs:88-105, 203-204, 311-324`), 동적 등록·capability(`registration.rs:176-181, 340-341`, `capabilities.rs:59-70`), `RefreshKind::SemanticTokens`(`protocol.rs:262-272`). 테마는 `taideSemantic.*` 규칙을 이미 만듭니다(`SYN/token-theme.rs` `SEMANTIC_TOKEN_THEME_TARGETS`·`token_color_rules`). 설정 모델(`CR/taide-model/src/settings.rs:596` 기본 true)과 토글 UI가 있습니다. 앱 `APP/lsp.rs`의 `initialize`에는 semanticTokens capability가 없고, `notice`는 `Refresh(RefreshKind::CodeLens)`만 처리합니다. 소비자가 없습니다.
- 설계:
    1. `ED/semantic-tokens.rs`: `TYPE_MAP`(원본 표 그대로), `Legend::new(server_types, server_modifiers)`, `decode`, `apply_delta`(역순), `reencode`. 결과를 문서 snapshot 기준으로 줄·UTF-16 열 → byte 범위로 바꿔 `SemanticLayer { revision, spans: Vec<(Range<usize>, target)> }`로 둡니다. 줄을 넘는 범위는 그 줄 끝에서 자릅니다(multilineTokenSupport false). 편집 이동은 기존 `ED/decoration.rs`의 ChangeSet 추적과 같은 방식(편집이 토큰 안을 지나면 그 토큰 제거)을 씁니다.
    2. `SYN/token-theme.rs`에 `semantic_style_table() -> [SemanticStyle; 21]`. `SemanticStyle { foreground, font_style: Option<FontStyleBits> }`로 "fontStyle 미지정"을 구분합니다. 지금 `MonacoTokenTheme::matched`가 미지정을 구분하지 못하면 그 구조체에 구분 필드를 추가합니다.
    3. `APP/lsp.rs`: initialize에 위 capability 두 개를 원본과 같은 값으로 추가. `notice`에서 `Refresh(RefreshKind::SemanticTokens)`를 `Acknowledged`로 응답하고 `Reply::SemanticTokensRefresh { project, provider }`.
    4. `APP/lsp-editor-semantic-tokens.rs`: 공급자별 문서 캐시 `{ result_id, server_data }`, delta 조건, null/오류 구분. `APP/editor-semantic-tokens.rs`: 문서별 일정(내용 변경 뒤 300ms, 열기·언어·공급자 변경·refresh·설정 변경은 즉시), 진행 중 1개, 화면에 붙은 문서만, Normal 등급만(LSP 계획이 Normal만 만듦), 테마 변경 시 제거 후 재요청, 설정 꺼짐이면 요청 없이 제거.
    5. `UI/editor-row-text.rs`에 `RowText::restyle(chars, foreground, font_style)`: `highlight()` 뒤, inline 장식(`decorate`) 앞에 적용. `UI/editor_surface.rs`의 편집기 요청에 줄별 semantic span을 inlay hint처럼 전달합니다. 같은 DocumentId의 본문·peek·mirror 뷰에 모두 적용합니다. 미니맵 토큰 색도 같은 layer를 씁니다(`UI/editor-minimap.rs`의 토큰 원천 확인).
- 검사: `[C]` 원본 `TS/shared/lib/lsp/adapters/semantic-tokens.test.ts` 사례 이식(decode/delta 역순/대응 없는 type 제거 뒤 delta 재계산/`constructor` 같은 이름 제거/modifier 대응), 편집 이동. `[C]` SYN style table 21개와 fontStyle 미지정. `[L]` `MOCK`에 `--native-semantic-tokens`(full, delta, null, error, refresh, unsupported) 모드 추가, 요청·delta·refresh·취소. `[A]` 실제 파일 열기 → 행 색 변경, 편집 중 이동, 설정 끔/켬, peek 같은 색, 테마 변경 재요청, 종료 tasks 0.
- 완료: 위 통과 + settings-28 비고의 semantic 항목 갱신.

### 2.3 editor-15 minimap과 토글 액션 (partial)

- 원본: 표시 `TS/features/editor/code-editor.tsx:275`(`minimap && !largeFile`), 토글 액션 246-252(`editor.toggleMinimap`, navigation 그룹). Monaco 미니맵에 그려지는 장식 공급원은 `rg -l "minimap:" MON/editor/contrib`로 확인한 다음뿐입니다: 찾기(`find/browser/findDecorations.js`), 단어 하이라이트·선택 발생(`wordHighlighter/browser/highlightDecorations.js:33-88`, 색 `minimapSelectionOccurrenceHighlight`), 접힌 영역 배경(`folding/browser/foldingDecorations.js:22, 30, 43-47, 61-65, 78-82`, 색 `editor.foldBackground`, `foldingHighlight` 기본 true일 때 highlighted 변형), section header(`sectionHeaders/browser/sectionHeaders.js`), 진단(marker). unicode 강조는 `minimap: null`.
- native 현재: 표시·설정 토글·찾기·진단·문서 하이라이트(`APP/editor-highlights.rs`의 `minimap` 색)·빈 줄 배경이 연결돼 있습니다(`UI/editor-minimap.rs`의 `frame.marks[].minimap`). section header와 접힌 영역 배경은 없습니다.
- 빠진 것과 설계:
    1. section header: `ED/section-headers.rs`에 `findSectionHeaders.js` 전체 이식. region은 언어 설정의 folding marker 시작 정규식(`ED/language-configuration.rs`의 marker)으로 줄 단위 일치, 텍스트는 trim·앞뒤 `-` 제거·앞 `-`면 구분선. MARK는 위 정규식을 100줄 묶음·5줄 겹침으로 찾고, `shouldBeInComments`는 그 위치의 표준 토큰이 Comment일 때만 남깁니다(기존 standard token Other 판정이 있는 Emmet 경로와 같은 토큰 원천). 무한 반복 정규식 가드(`regExpLeadsToEndlessLoop`)는 고정 정규식이라 필요 없지만 같은 의미의 빈 일치 전진을 둡니다. 계산은 지연 250ms/토큰 변경 1000ms, 큰 모델(`isTooLargeForSyncing`)은 계산하지 않음 → native Large 등급 제외. 표시는 `minimap.js:1394-1440`의 `_renderSectionHeaders`·`_fitSectionHeader`·`_renderSectionLabel`을 따라 배경 채움·글자·밑줄(구분선) 그리기.
    2. 접힌 영역 배경: 접힌 범위마다 `editor.foldBackground` 색으로 미니맵 줄 배경(`MinimapPosition.Inline`). 본문의 folded-background가 native에 이미 있는지 확인하고 같은 색 원천을 씁니다.
    3. 선택 텍스트 발생 강조: native 본문에 Monaco SelectionHighlighter(선택한 한 줄 텍스트의 다른 발생 강조)가 있는지 먼저 확인합니다. 있으면 그 표시의 미니맵 색만 더하고, 없으면 1.4의 발견 항목으로 4절 결정 2에 따릅니다.
- 검사: `[C]` `findSectionHeaders` oracle(MARK/구분선/빈 label/region/주석 밖 MARK 제거). `[U]` `tests/editor-minimap.rs`에 header 글자·배경 위치, 접힌 영역 배경. `[A]` 실제 TS/Rust 파일의 `// MARK: - Foo`와 `#region` 표시, 편집 250ms 뒤 갱신, Large 등급 미표시.
- 완료: 위 통과, 비고의 SCM 문구 정정.

### 2.4 editor-16 sticky scroll (partial)

- 원본: `code-editor.tsx:279`(`stickyScroll.enabled`). Monaco 기본 모델은 outline(문서 심볼) → folding provider → 들여쓰기 순입니다. 남은 것은 CSS/SCSS·HTML·TS/JS 내장 문서 심볼 공급뿐입니다(JSON은 b3f 완료).
- native 현재: 고정 줄 UI·정의 이동·JSON 구조 공급·외부 LSP 심볼·구문 접기·indent fallback 모두 연결(`ED/sticky-model.rs`, `UI/editor-sticky-scroll.rs`, `APP/editor-symbols.rs`, `APP/editor-json.rs`).
- 설계: 새 구현 없음. E4·E5·E6에서 각 언어 문서 심볼을 `editor-json.rs`의 구조 공급과 같은 경로로 공급하면 자동으로 쓰입니다.
- 검사: 각 언어 묶음의 `[A]` 1건씩(파일 열기 → 스크롤 → 고정 줄에 선택자/요소/클래스 이름).
- 완료: E6까지(또는 결정 4가 정한 TS 범위까지) 세 언어 고정 줄 근거.

### 2.5 editor-32 formatOnType·formatOnPaste (partial)

- 원본: `code-editor.tsx:299-300`, 외부 LSP 경로는 완료(c3). 내장 공급: TS/JS onType 트리거 `;`, `}`, `\n`(`MON/languages/features/typescript/languageFeatures.js:756-758`), 붙여넣기 포맷은 범위 포맷 공급자를 쓰므로 CSS/SCSS·HTML(html만)·TS/JS 범위 포맷이 대상입니다. JSON은 onType 없음, 범위 포맷은 완료(b3h2a).
- native 현재: `APP/editor-formatting.rs`의 입력/붙여넣기 포맷 상태와 공급자 순서(외부 우선, 내장 뒤 — b3h에서 정함).
- 설계: E4·E5·E6의 범위 포맷/onType 공급자를 이 상태의 공급자 목록에 더합니다.
- 완료: 세 언어 붙여넣기 포맷 `[A]` 각 1건 + TS onType `[A]` 1건.

### 2.6 editor-34 Monaco 액션의 팔레트·키바인딩 노출 (partial)

- 원본: `TS/shared/lib/monaco/monaco-action-commands.ts:39-46`(각 `MONACO_ACTIONS` 항목 → `requestEditorPaneCommand({ type: 'run-monaco-action' })`, Action2 21개는 항상 활성, 나머지는 `activeEditorActionIds` 포함 여부로 활성), 목록 `TS/shared/lib/monaco/monaco-actions.ts`(TAIDE 사용자 액션 7개 + Monaco 0.56 내장).
- native 현재: 메타·키맵·활성 조건은 연결(`UI/command-registry.rs` `enablement`, `EDITOR_ACTIONS_WITHOUT_SUPPORT_GATE`). 실행은 `execution`에서 지원 범주만 `Execution::Native`, 나머지는 `Execution::Unavailable`입니다.
- 정적 대조로 본 미실행 액션(구현 전 첫 작업으로 "모든 `monaco.*` 중 `Unavailable` 목록" 단위 검사를 만들어 확정):

| 액션 | 원본 동작 | 설계 |
| --- | --- | --- |
| `editor.action.fontZoomIn`·`Out`·`Reset` | 창 전역 zoom 수준 ±1/0(−5..20), 글꼴·줄 높이×(1+0.1×수준), 비영속(`MON/editor/contrib/fontZoom/browser/fontZoom.js`, `editorZoom.js`) | 창(viewport)별 `editor_zoom_level`을 `domains/editor.rs`에 두고 `EditorAppearance`의 글꼴 크기·줄 높이에 곱함. 상태바 글꼴 크기(editor-84)와 별개 |
| `editor.action.gotoLine`(⌃G) | standalone quick input `:` 줄/열 이동 | 기존 팔레트 `Mode::Line`(`UI/command-palette.rs`)을 `:`로 열기(결정 6) |
| `editor.action.quickOutline`(⇧⌘O) | standalone quick input `@` 심볼 | 기존 팔레트 `Mode::Symbol`을 `@`로 열기(결정 6) |
| `editor.action.hideHover` | 열린 hover 닫기 | 기존 hover 상태 닫기 |
| `increase/decreaseHoverVerbosityLevel` | 공급자가 verbosity를 지원할 때만 단계 변경, 아니면 무동작 | 현재 공급자(LSP·내장)는 미지원 → hover가 열렸을 때만 활성, 실행은 무동작(원본과 같은 결과). TS 내장(E6)에서 지원 여부 재확인 |
| `editor.action.inPlaceReplace.up/down`(⇧⌘, / ⇧⌘.) | 커서 단어를 값 집합의 이전/다음으로 교체, 숫자 ±1, 잠깐 강조(`MON/editor/contrib/inPlaceReplace/browser/inPlaceReplace.js`, `MON/editor/common/languages/supports/inplaceReplaceSupport.js`) | `ED/inplace-replace.rs` 이식 + 기존 편집 트랜잭션/undo, readonly 비활성 |
| `editor.action.clipboardCopyWithSyntaxHighlightingAction` | 선택(없으면 줄)을 구문 색 HTML+텍스트로 복사 | 2.0.2의 서식 있는 복사 |
| `editor.action.pasteAs`·`pasteAsText` | 붙여넣기 편집 공급자 선택(텍스트만이면 "Insert Plain Text" 하나) / 일반 텍스트 붙여넣기(`MON/editor/contrib/dropOrPasteInto/browser/defaultProviders.js:55-82, 216-222`) | 주입 clipboard의 형식으로 공급자 목록 계산, 하나면 바로 적용 |
| `editor.action.toggleTabFocusMode`(⌃⇧M) | 창 전역 토글, 켜지면 Tab/⇧Tab이 포커스 이동 | 창별 플래그, 켜졌을 때 편집기 Tab 입력을 egui 포커스 이동으로 |
| `editor.action.openLink` | 커서 위치 링크 열기 | 결정 2(링크 감지) |
| `editor.action.unicodeHighlight.*`(4) | hover 안의 제외 옵션 | 결정 2(Unicode 강조) |
| `editor.action.inlineSuggest.toggleAlwaysShowToolbar` | toolbar 항상 표시 토글(설정 서비스 메모리 값) | editor-77의 toolbar와 함께 |
| `editor.action.showOrFocusStandaloneColorPicker` | 커서 위치 색(없으면 기본) 선택기 표시/포커스 | E4 색 선택기 뒤 |
| `taide.gitStageSelection`·`taide.toggleBlame`·`taide.openFileHistory` | git 기능 | git lane(git-55·57·60). 이 lane은 손대지 않음 |

- 검사: `[U]` 목록 단위 검사(남은 `Unavailable`이 git 3개와 결정으로 보류한 항목뿐임), 각 액션 `[U]` 실행. `[A]` 팔레트와 재지정 키로 zoom·gotoLine·inPlaceReplace·tab focus 각 1건, readonly 비활성.
- 완료: git 3개를 뺀 모든 액션이 실행되고, 결정 2·E4에 걸린 항목은 그 구현 뒤 판정. git 3개는 git lane 완료 시 그 근거로 함께 닫습니다.

### 2.7 editor-49 포맷 (partial)

- 원본: `TS/shared/lib/lsp/adapters/formatting.ts:14, 34`, `TS/widgets/editor-pane/editor-pane.tsx:141`. 외부 LSP·JSON 내장은 완료. 남은 것: CSS/SCSS 문서·범위(`MEXT/vscode-css-languageservice/lib/esm/services/cssFormatter.js` + `beautify/beautify-css.js`, 옵션 `MON/languages/features/css/register.js:60-67`), HTML 문서·범위(`MEXT/vscode-html-languageservice/lib/esm/services/htmlFormatter.js` + `beautify/beautify-html.js`, `<style>`은 beautify-css, `<script>`는 무변경 mock `beautify/beautify.js`, 옵션 `MON/languages/features/html/register.js:31-44`), TS/JS 범위(문서 포맷도 전체 범위로 범위 공급자 사용).
- 설계: E4·E5·E6의 포맷 공급자를 `APP/editor-formatting.rs`·저장 포맷 참여자에 JSON과 같은 방식으로 연결. 편집기 탭/공백 값은 문서 옵션(editor-30)에서 가져옵니다.
- 완료: 세 언어 문서/범위/저장 포맷 `[A]`.

### 2.8 editor-57 Monaco 내장 언어 서비스 (partial) — 별도 묶음 E3~E6

- 원본 계약:
    - 활성: Monaco 내장 모드는 언어 ID 첫 사용 때 등록됩니다. TAIDE 언어 ID(`TS/shared/lib/shiki/lang-map.ts:3-35`) 중 내장 서비스가 붙는 것은 `json`(jsonc 아님), `css`, `scss`, `html`, `typescript`, `javascript`입니다. `less`·`handlebars`·`razor`는 TAIDE ID에 없고 `typescriptreact`·`javascriptreact`에는 내장 worker가 붙지 않습니다.
    - 공급자 목록: JSON(`MON/languages/features/json/jsonMode.js:44-115`: 문서/범위 포맷, 완성 트리거 `" ", ":", '"'`, hover, 심볼, 색, 접기, 선택 확장, 진단), CSS/SCSS(`css/cssMode.js:14-113`, 완성 트리거 `/ - :`, hover, 하이라이트, 정의, 참조, 심볼, rename, 색, 접기, 진단, 선택 확장, 문서/범위 포맷, lint 기본값 `css/register.js:37-59`), HTML(`html/htmlMode.js`의 `registerProviders`: 완성 트리거 `. : < " = /`, hover, 하이라이트, 링크, 심볼, rename, 접기, 선택 확장, html만 문서/범위 포맷. 진단·색 없음), TS/JS(`typescript/tsMode.js:30-132`: 완성 트리거 `.`, signature `( ,`, hover, 하이라이트, 정의, 참조, 심볼, rename, 범위 포맷, onType `; } \n`, code action, inlay hint, 진단).
    - 프로젝트 정책: TS/JS 진단은 syntax만(`TS/shared/lib/monaco/builtin-typescript.ts:38-57`). 외부 세션이 typescript·typescriptreact·javascript·javascriptreact를 소유하는 동안 TS 내장 서비스 전체 정지(참조 계수, `builtin-typescript-mode.ts:33-60, 83-88, 150-162`). 원본은 0.56에서 이미 등록된 공급자를 해제하지 못하는 한계를 문서에 적어 두었습니다(130-146). native는 이 한계를 재현하지 않고 세션 동안 TS 내장 공급자를 모두 제외합니다.
    - 외부 공급자와 공존: CSS/HTML/JSON 내장은 외부 세션이 있어도 정지하지 않습니다. 단일 결과 기능(포맷)은 외부 우선, 병합 기능(완성·hover·심볼·하이라이트)은 둘 다, 진단은 소유자별로 둘 다 표시합니다. 이 순서는 JSON(b3f·b3h)에서 정한 native 정책을 그대로 씁니다.
- 완료된 것: JSON의 구조·포맷·진단·schema 전체와 실제 App 공급(1.2).
- 남은 것과 묶음:
    - **E3 JSON 조회·완성**: `MEXT/vscode-json-languageservice/lib/esm/services/jsonCompletion.js`(schema 없는 속성/값 제안 포함, `$schema` 메타 schema는 b3e11 공급 재사용, resolve), `jsonHover.js`. 소유: `ED/json-completion.rs`·`ED/json-hover.rs`, 앱 공급은 `APP/editor-completion-provider.rs`(Emmet 공급 방식)와 hover 소비(`APP/editor-documentation.rs`). JSON 색(`jsonDocumentSymbols.findDocumentColors`)은 E4의 색 UI와 함께 연결합니다.
    - **E4 CSS/SCSS**: 파서(`MEXT/vscode-css-languageservice/lib/esm/parser/cssScanner.js`, `cssParser.js` 1794줄, `scssParser.js` 809줄, `cssNodes.js`), lint(`services/lint.js`·`lintRules.js`, 심각도는 위 기본값), 완성(`cssCompletion.js`·`scssCompletion.js` + `data/webCustomData.js`를 bun 스크립트로 Rust 표 생성), hover(`cssHover.js`·`selectorPrinting.js`), 탐색(`cssNavigation.js`·`scssNavigation.js`: 심볼·하이라이트·정의·참조·rename·색·색 표현), 접기(`cssFolding.js`), 선택 확장(`cssSelectionRange.js`), 포맷(`cssFormatter.js`·`beautify/beautify-css.js`). 색 장식·색 선택기 UI(1.4, `MON/editor/contrib/colorPicker/browser/`)를 여기서 만들고 JSON 색과 모든 언어의 기본 색 공급자(`defaultDocumentColorsComputer.js`, 다른 색 공급자가 없을 때만)도 연결합니다. 기본 공급자는 언어와 무관하므로 `ED/color.rs` 옆 `ED/default-colors.rs`로 둡니다.
    - **E5 HTML**: `MEXT/vscode-html-languageservice/lib/esm/parser/htmlScanner.js`(406줄)·`htmlParser.js`, 완성(`services/htmlCompletion.js` + `languageFacts/data/webCustomData.js` 표 생성), hover, 하이라이트(`htmlHighlighting.js`), 링크(`htmlLinks.js`, 1.4 링크 UI와 공유), 심볼, rename(`htmlRename.js`), 접기, 선택 확장, 포맷(`htmlFormatter.js`·`beautify/beautify-html.js` 3195줄, `<style>`은 E4 beautify-css 재사용).
    - **E6 TS/JS**: 4절 결정 4가 정한 방식으로 구현합니다. 원본 기능 목록은 위 TS/JS 행, 진단은 syntax만, 외부 세션 동안 정지.
- 공통 설계: 각 언어 서비스는 portable core(ED, wasm32 컴파일)와 앱 공급자(`APP/editor-css.rs`·`editor-html.rs`)로 나눕니다. 앱 공급자는 `APP/editor-json.rs`의 문서 작업·별도 generation·크기 등급(Normal만)·readonly·닫힘/취소/종료·오래된 결과 거절을 그대로 따릅니다. 원본 위치는 UTF-16 offset이므로 JSON처럼 byte/UTF-16 변환과 입력/출력/작업 예산을 둡니다. 원본 결함은 oracle에 raw로 남기고 교정 기대값을 따로 둡니다(JSON 선례).
- 검사: 기능마다 `[C]` oracle, 언어마다 `[A]`(파일·untitled·peek, 외부 공급자 공존, readonly, Large 제외, 닫힘 취소).
- 완료: E3~E6이 모두 닫히면 editor-57 complete, 같은 근거로 16·32·49를 닫습니다. 결정 4가 TS 범위를 줄이면 그 결정 문구를 비고에 적고 판정합니다.

### 2.9 editor-58 파일 간 이동 opener (partial)

- 원본: `TS/shared/lib/bridge/editor-opener-bridge.ts:39-61`. 같은 모델이면 false(Monaco가 그 편집기 안에서 처리). `untitled:`면 그 창 realm의 열린 편집기 중 같은 모델을 찾아 위치 설정·가운데 reveal·포커스, 없으면 false(무동작). `file:`이면 구독자에게 `{path, line, column}`을 보내고, 포커스된 창의 `editor-area`가 `openFileTab({ target: 포커스 pane, preview: true, reveal })`(`TS/widgets/editor-area/editor-area.tsx:341-353`). 진입: 정의/구현/타입 정의/선언/참조/peek/F8 계열이 다른 모델에 도착할 때.
- native 현재: 파일 정의 계열·LocationLink·peek 인계(`APP/symbol-location-host.rs`, `APP/editor-locations.rs`, c5c2·c9c2e2c 검사), 문제 이동의 File/Untitled 대상(`APP/editor-problems.rs`, b3g2b 검사).
- 빠진 것: 진입별 근거 정리, 보조 창이 포커스일 때 대상 창(원본은 포커스 창의 포커스 pane), untitled 대상이 다른 창에만 있을 때 무동작 단언.
- 설계: 새 기능 구현은 없을 것으로 봅니다. 아래 검사에서 실패하면 그 경로만 고칩니다. E6(TS 내장)이 정의를 lib 파일(`file:///lib.*.d.ts`)로 돌려주는 원본 결함(존재하지 않는 경로 열기 오류)은 재현하지 않습니다.
- 검사: `[A]` 보조 창 포커스에서 다른 파일 정의 이동 → 보조 창 포커스 pane에 preview 탭, `nextInFiles`로 같은 창 untitled 도착, 다른 창에만 있는 untitled는 무동작.
- 완료: 위 + 기존 근거 목록으로 complete.

### 2.10 editor-72 Markdown 미리보기 분할 (partial)

- 원본: 분할·토글(`TS/widgets/editor-pane/editor-pane.tsx:494-510, 548-564`, `editor.toggleMarkdownPreview`), 렌더 `marked` → `DOMPurify.sanitize`(`TS/shared/lib/markdown.ts:1-6`), 표시 클래스 `TS/features/editor/markdown-preview.tsx:5-37`(브라우저 기본 CSS + Tailwind 선택자, 세로·가로 overlay scrollbar).
- native 현재(d6a~d6c3h4): 분할·수명·readonly·두 뷰·독립 scroll·HTML 파서·안전 URL·이미지·표(병합·그룹·제목·셀 블록)·목록(type·역순·value·바깥 마커·작업 목록)·문단 간격·글꼴 상속·코드 요소 경계. 파일: `APP/editor-markdown.rs`, `editor-markdown-html.rs`, `editor-markdown-parse.rs`, `editor-markdown-table.rs`, `editor-markup.rs`, `UI/editor-markup-preview.rs`, `editor-markup-flow.rs`, `editor-markup-table.rs`, `editor-markup-aligned.rs`. 검사 `APP/application-markdown-tests.rs`, `editor-markdown-parse-tests.rs`, `UI/tests/editor-markup-preview.rs`.
- 남은 것(`2026-10-12-native-batch23-markdown.md` 237줄의 최신 잔여): 혼합 inline code의 줄 배치, 중첩 code 박스, `<br>`, 문단 안 이미지 경계, `kbd`/`samp`(브라우저 기본 monospace만, 박스 없음 — Tailwind는 `code`만 꾸밈), raw `input`(checkbox 외), 첫 블록 table 위 여백, 임의 CSS(`style` 속성)·상대 굵기(bolder)·counter, 열/행 스타일(`col`/`colgroup`), 접근성(표·목록 의미), SVG/MathML/audio/video/details/summary/이름 앵커.
- 설계: 4절 결정 7로 닫을 범위를 정합니다. 추천안 A의 구현 목록은 `br`, 혼합 inline code 줄 배치, 중첩 code, 문단 안 이미지, `kbd`/`samp` 글꼴, raw `input` 기본 모양(비활성), 첫 블록 여백, `details`/`summary`(접힘 토글), `a name`/`id` 앵커 이동, 표·목록 AccessKit 역할입니다. 기존 parser/모델(`Block`·`Span`·`TableLayout`)에 선택 필드를 더하는 방식은 이전 단계와 같습니다.
- 검사: 단계마다 `[C]` parser, `[U]` shape, `[A]` 1건(실제 파일·입력/undo·원문·작업 0). 브라우저 비교는 이전 단계처럼 설치된 브라우저 근거 표(사람이 만든 기대값)로 합니다.
- 완료: 결정 7이 정한 목록 통과 후 complete, 비대상 항목은 비고에 근거와 함께.

### 2.11 editor-73 Git gutter·hunk 되돌리기·선택 stage (missing) — git lane 위임

- 원본: `TS/widgets/editor-pane/use-editor-git-gutter-and-conflicts.ts:58, 224, 238, 292`.
- 처리: git lane의 git-58(gutter 막대), git-59(클릭 → hunk discard), git-60(우클릭 Stage Changes), git-43(diff 탭 gutter stage)이 구현합니다(`2026-10-28-lane-git-spec.md` 3절 G5·G8). 이 lane은 구현하지 않고 편집기 표면의 git 확장 지점도 만들지 않습니다.
- 완료: git lane 근거로 함께 갱신.

### 2.12 editor-74 merge conflict 표시·해결 dialog (missing) — git lane 위임

- 원본: `use-editor-git-gutter-and-conflicts.ts:211-219`, `editor-pane.tsx:520-533`. git-61(파싱·표시), git-62(해결 대화상자, 편집기 저장 정착 경로 사용), git-63(비교 대화상자)이 구현합니다(G8).

### 2.13 editor-75 blame (missing) — git lane 위임

- 원본: `TS/widgets/editor-pane/use-editor-blame.ts:54, 116-143`, `TS/features/editor/blame-footer-bar.tsx`. git-56(footer), git-57(overlay·`git.toggleBlame`), git-53~55(파일 이력)가 구현합니다(G7·G8).

### 2.14 editor-76 diff 편집기 (missing) — git lane 위임

- 원본: `TS/features/git/diff-view.tsx:46`, `TS/widgets/claude-diff-pane/claude-diff-pane.tsx:58`. git-40(diff 렌더), git-47(커밋 diff), git-50(Claude diff), git-63(충돌 비교)이 구현합니다(G5·G7·G8). diff 줄 계산·monaco 기본 장식 범위는 git 사양 4.1 결정 6·7입니다.

### 2.15 editor-77 AI 인라인 완성 (partial)

- 원본: `code-editor.tsx:323-333`(`inlineSuggest.enabled = aiAutoTabEnabled`, 공급자 획득), `TS/shared/lib/ai/inline-completion.ts:182`. Monaco 기본 UI: hover toolbar(`showToolbar: 'onHover'`, `inlineCompletions.contribution.js:31`의 hover participant, `hintsWidget/inlineCompletionsHintsWidget.js:115-200`: 이전·`n/m`·다음, primary Accept·Accept Word, secondary Accept Line·Always Show Toolbar), 키: `⌘→` 다음 단어 수락(조건: 쓰기 가능·ghost 표시·커서가 ghost 앞, `controller/commands.js:121-145`), Accept Line(키 없음, 146-165), 다음/이전 제안·숨김은 같은 파일의 Show Next/Previous·HideInlineCompletion.
- native 현재: 자동/명시 요청·문맥/캐시·credential·문서/뷰 수명·ghost/Tab/undo·IME/readonly/다중 커서·peek/split·일반 후보 합성 모두 완료(`APP/editor-ai-completion.rs`, `2026-10-13-native-batch23-ai.md`). 이전 작업은 toolbar를 "원본 미등록"으로 보고 만들지 않았으나 위 근거상 standalone에도 등록돼 있습니다.
- 빠진 것: hover toolbar, `⌘→` 다음 단어 수락, Accept Line, 다음/이전 제안(AI 공급자는 제안 1개라 비활성 표시).
- 설계: `ED/completion.rs`의 `Command`에 `AcceptNextWord`·`AcceptNextLine`·`ShowNext`·`ShowPrevious`를 추가하고 단어·줄 경계는 `MON/editor/contrib/inlineCompletions/browser/model/inlineCompletionsModel.js:887-940`의 `acceptNextWord`·`acceptNextLine` 규칙을 따릅니다. 부분 수락 뒤 남은 ghost는 기존 forward stability 경로(e1e)로 유지합니다. `UI/editor-inline-suggest-toolbar.rs`는 ghost 위 hover 때 표시, Always Show Toolbar는 창별 메모리 값(editor-34 액션과 공유).
- 검사: `[U]` toolbar 표시/클릭/개수 문구, `⌘→` 경계. `[A]` 주입 응답으로 `⌘→` 두 번 → 두 단어 삽입·나머지 ghost·undo 단위, toolbar Accept.
- 완료: 위 + 기존 근거. 전체 App 준비 실패는 배치 31 부채로 비고에 남기고 complete로 판정합니다.

### 2.16 editor-78 AI 인라인 편집 (partial)

- 원본: `TS/features/editor/ai-inline-edit.ts:86-349`. native는 e2·e2b·e2b2a~c로 요청·소비·source·표시·입력·추가 수명까지 완료했습니다.
- 빠진 것: 기능 차이는 확인되지 않았습니다. partial 사유는 전체 App 준비 실패와 실기입니다.
- 설계: 구현 없음. lane worktree에서 관련 `[A]`(`application-ai-edit-tests.rs`)를 1회 실행해 현재 HEAD 근거를 만듭니다.
- 완료: 그 통과로 complete 판정, 비고에 배치 31 부채 구분.

### 2.17 editor-81 search-editor 탭 (missing) — explorer-search lane 위임

- 원본: `TS/widgets/search-editor/search-editor-pane.tsx:65-195`(자세한 동작은 `2026-10-28-lane-explorer-search-spec.md` 467-488줄).
- 처리: explorer-search 사양 D4 A안(추천)은 그 lane이 검색 패널 모듈을 재사용해 구현합니다. 이 lane이 줄 것은 탭 내용 hook(1.5)뿐입니다. D4가 B안이면 이 lane 묶음 E2에 넣습니다.

### 2.18 preview-3 탭 우클릭 "다른 편집기로 다시 열기" (missing → 실제 구현됨)

- 원본: `TS/features/tab/tab-context-menu.tsx:58-59, 93, 150-158`(파일 탭이고 두 콜백이 있을 때 하위 메뉴 `tab.reopenEditorWith` → `explorer.openWithEditor`/`explorer.openWithPreview`), `TS/widgets/editor-area/pane-tab-bar.tsx:183, 226-227`(파일 이름의 `resolvePreviewKind`가 있을 때만, Editor → override `editor`, Preview → override 제거). 주 창·보조 창 공통.
- native 현재: `UI/shell.rs`의 파일 탭 메뉴가 `ShellSurfaces::can_reopen_with`일 때 같은 하위 메뉴와 `ShellIntent::ReopenWith { path, preview }`, 앱은 `open_with::Registry::set`(`APP/open_with.rs`, `Mode::Preview`는 override 제거 = 원본 null). 확장자 표는 `TS/shared/lib/preview-kind.ts:5-32`와 같습니다. 주 창 `[A]`가 있습니다.
- 빠진 것: 보조 창 근거.
- 검사: `[A]` 보조 창 파일 탭 메뉴 → Editor/Preview 전환(`application-auxiliary-window-tests.rs` fixture 재사용).
- 완료: 위 통과, shell-78 비고와 함께 갱신.

### 2.19 preview-5 외부 저장·이동·삭제 시 미리보기 갱신/무효화 (partial)

- 원본:
    - 외부 변경: `fs:changed` 경로의 `FILE.RAW`·`FILE.CONTENT`를 `fromApp`과 무관하게 무효화(`TS/app/providers/ipc-sync-provider.tsx:48-51, 489-497`) → 미리보기가 다시 읽음. PDF는 다시 읽으면 1쪽으로 돌아가고 배율은 유지(`TS/features/preview/pdf-preview.tsx:42-70`).
    - rescan: `FILE.ALL` 전체 무효화(249-274). 프로젝트 닫힘: 루트 아래 `FILE.RAW` 제거(120-134의 `isQueryKeyUnderProjectRoot`).
    - 앱 안 이름 바꾸기: 탭 경로 이동, override 이동, 새 경로 `FILE.RAW` 무효화(`TS/entities/layout/tab-path-change.ts:245-277`, `TS/entities/file/workspace-resource-operations.ts:64`).
    - 앱 안 삭제: 탭 닫기 후 경로별 상태 해제·캐시 제거(`tab-path-change.ts:145-166, 332-341`).
    - 외부 삭제: 탭은 닫지 않고 다시 읽기 실패 → `UnsupportedPreview`(preview-6).
- native 현재: `APP/events.rs`의 `TreeChanges::record_preview`(FsChanged 경로·FsRescanRequired 프로젝트·상한 512 넘으면 전체)와 앱의 `reconcile_previews`(모든 형식 캐시·실패/거절 표시·web 취소), 탭 닫힘 해제(`close` 처리의 `has_remaining_file`), 이름 바꾸기/삭제의 탭 경로 변경(`APP/workspace_rename.rs`, `APP/workspace_delete.rs`, `APP/explorer_delete.rs`), override 이동(`open_with::Registry::reconcile`). 근거 `native/taide-native-app/tests/preview-events.rs` 1건(이벤트 수집).
- 빠진 것: 형식별 실제 재표시·여러 뷰·이동/삭제 상호작용의 `[A]` 근거. 결함이 나오면 그 경로만 고칩니다.
- 검사: `[A]` 새 `application-preview-tests.rs`(domains/preview.rs에서 등록)에 소유 임시 프로젝트로 (1) PNG·CSV·HTML 각 외부 수정 → 새 내용 표시, (2) PDF 외부 수정 → 1쪽·배율 유지, (3) 두 pane에 같은 이미지 → 둘 다 갱신, (4) 앱 안 rename → 새 경로 표시·override 유지, (5) 앱 안 삭제(추적 파일, 휴지통이 아닌 경로만 — `tests/explorer-delete.rs` 금지 유지, 삭제 대신 주입 경로가 없으면 이 사례는 4.2 허락) → 탭 닫힘·캐시 해제, (6) 외부 삭제 → 읽기 실패 화면, (7) 프로젝트 닫힘 → 루트 캐시 제거. 이벤트는 실제 watcher 대신 `AppEvent::FsChanged`를 직접 기록합니다.
- 완료: 위 통과(5번이 허락 대기면 그 사실을 비고에 적고 판정 보류).

### 2.20 preview-6 외부 앱으로 열기 버튼과 UnsupportedPreview (partial)

- 원본: `TS/widgets/preview-pane/preview-pane.tsx:48-86`. 영상/오디오는 바이트를 읽지 않음. 바이트가 필요한 형식(image·pdf·spreadsheet·presentation·hwp·html)의 **읽기 실패**는 `UnsupportedPreview`(`TS/features/preview/unsupported-preview.tsx:11-27`: `FileQuestion` 40px, `preview.notSupported`, 파일 이름 12px·70%·말줄임·최대 448px, outline 작은 버튼 `ExternalLink` + `preview.openExternally`). 읽는 중이면 편집기 배경만(문구·spinner 없음). 외부 열기 실패는 오류 toast(`describeIpcError`). 문서 형식(pdf 등)의 해석 실패는 각 컴포넌트의 상태 화면(`TS/features/preview/preview-status.tsx`)과 외부 열기 버튼. 이미지 해석 실패는 브라우저 깨진 이미지(대체 문구 = 파일 이름).
- native 현재: web·pdf·spreadsheet·presentation·hwp는 `APP/preview_status.rs`의 `show`(읽기 실패 = 원본 화면 대응, 해석 실패 = 형식별 문구 + 외부 열기)를 씁니다. 이미지(`APP/preview.rs` `Cache`)는 읽기/해석 실패를 `Entry::Failed(AppError)` 하나로 합치고, 미리보기 탭 내용 분기에서 `ui.label` 세 줄(문구·제목·오류 문자열)과 기본 버튼을 그립니다. 같은 분기의 마지막 `native preview provider is not connected: {kind:?}` 문구는 모든 형식이 처리돼 도달하지 않지만 남아 있습니다. HTML·오디오·비디오의 준비 중 화면은 `preview_status::show`의 spinner + `common.loading`인데, 원본 HTML은 바이트를 읽는 동안 빈 편집기 배경이고(67줄) 오디오·비디오는 바로 요소를 그립니다. pdf·spreadsheet·presentation·hwp는 원본 컴포넌트도 spinner + `common.loading`(`TS/features/preview/pdf-preview.tsx:124-126`)이라 현행이 맞습니다.
- 설계: 이미지 `Entry::Failed`를 `Failure`(`Read`/`Decode`)로 바꾸고, 읽기 실패는 `preview_status::show`(읽기 실패 경로)로, 해석 실패는 파일 이름만 가운데 표시(브라우저 대체 문구 대응, 4절 결정 8). HTML·오디오·비디오의 WebView 준비 중에는 빈 편집기 배경만 그립니다(native는 WebView 준비 전 media 요소를 그릴 수단이 없음). 문서 형식의 loading 화면은 유지합니다. 도달하지 않는 디버그 문구는 제거합니다.
- 검사: `[U]` 이미지 읽기 실패 화면(아이콘·문구·이름·버튼 접근성 이름), 해석 실패 화면. `[A]` 읽을 수 없는 PNG(권한 0 파일) → 화면·버튼 → 주입 platform으로 OpenPath 호출·실패 toast.
- 완료: 위 통과.

### 2.21 preview-14 비디오 / preview-15 오디오 (partial)

- 원본: `TS/features/preview/video-preview.tsx:7-11`(`<video controls>` 가운데, 16px 여백, 최대 100%), `audio-preview.tsx:9-15`(Music 아이콘·파일 이름·`<audio controls>`). 소스는 `convertFileSrc(path)`(Range 지원). theme 변경은 CSS만 바뀌고 재생 위치·일시정지·볼륨이 유지됩니다.
- native 현재(`2026-10-02-m8-native-audio-video-webview.md`): `APP/preview_web_media.rs`(템플릿·DOM 주입), 승인 host·Range·캐시(`preview_web_host.rs`, `preview_web_cache.rs`, `preview_web_range.rs`), macOS child WebView(`preview_web_view.rs`). theme 변경은 media 문서를 무효화하고 다시 만들어 재생 상태가 사라집니다(QA 5항).
- 빠진 것: (1) theme 변경 때 재생 상태 유지, (2) 실제 재생·seek·컨트롤·키보드/AX 동작 근거.
- 설계: (1)은 4절 결정 9. 추천안은 WebView에 host 쪽 `evaluate_script`로 CSS 변수만 바꾸는 것입니다(페이지 JS는 계속 끔). 착수 전에 `APP/../vendor/wry-preview`의 WKWebView 설정이 `javaScriptEnabled`(앱 스크립트까지 막음)인지 `allowsContentJavaScript = false`(페이지만 막음)인지 확인합니다. 변경은 `preview_web_media.rs`·`preview_web_cache.rs`와 `preview_web_view.rs` 파일 끝의 새 메서드만으로 하고 shell lane의 viewport 분리를 건드리지 않습니다. (2)는 GUI가 필요해 4.2 허락 항목입니다.
- 검사: `[C]`·기존 `tests/preview-web-media.rs` 확장(theme 변경 시 media 문서 무효화 대신 갱신 요청 생성, 활성 token 유지). 실제 재생은 허락 뒤 실기 또는 합성 창 검사.
- 완료: (1) 통과 + (2) 허락된 근거. 허락이 없으면 partial 유지하고 사유를 적습니다.

### 2.22 preview-38 비macOS(Windows/Linux) 지원 (partial)

- 원본: Tauri는 3 OS 웹뷰를 쓰지만 원본 제품의 빌드·출시는 macOS뿐입니다(`.github/workflows/ci.yml:49-53`의 "the app targets macOS only" 주석과 `macos-latest` rust 작업, `release.yml`의 모든 작업 `macos-latest`). 미리보기의 OS별 경로는 원본에서도 출시 검증 대상이 아닙니다.
- native 현재: PDF(`APP/preview_pdf.rs` `decode`)는 비macOS에서 "not connected" 오류, AVIF·ICC(`APP/preview.rs`)는 macOS ImageIO 전용, WebView(`preview_web_view.rs`의 비macOS `sync`)는 placement가 있으면 오류, wry 의존은 macOS 대상에만 있습니다(`native/taide-native-app/Cargo.toml`의 `cfg(target_os = "macos")` 의존).
- 설계: 4절 결정 10. 추천 A는 not-applicable(원본 출시 대상 아님)로 정정하고, 비macOS 컴파일 경로의 오류가 패닉 없이 preview-6의 실패 화면으로 이어지는지만 코드 대조로 확인하는 것입니다. 비macOS 컴파일 확인(`rustup target` 추가)은 환경 변경이라 4.2 허락 항목입니다.
- 완료: 결정에 따릅니다.

---

## 3. 작업 묶음 계획

새 crate 의존성은 E6(결정 4)와 결정 1·9·10의 B안을 빼면 없습니다. 원본 자료(CSS/HTML data)는 bun 스크립트로 Rust 소스를 생성해 넣고 MIT 출처를 붙입니다. lockfile 변화가 없어야 합니다.

| 묶음 | ID (수) | 선행 | 수정 예상 파일 | 새 의존성 |
| --- | --- | --- | --- | --- |
| E1 편집기 표시·명령 | editor-58, 78, 77, 7, 15, 4, 34 (7) | 단계 0, 결정 1·2·6 | `ED/semantic-tokens.rs`·`section-headers.rs`·`inplace-replace.rs`·`completion.rs`(Command 추가)·`link-computer.rs`/`unicode-highlight.rs`(결정 2 A), `SYN/token-theme.rs`, `APP/lsp.rs`(initialize·refresh), `APP/lsp-editor-semantic-tokens.rs`·`editor-semantic-tokens.rs`·`editor-ai-completion.rs`, `APP/domains/editor.rs`, `UI/editor-row-text.rs`·`editor_surface.rs`(옵션·요청 필드 추가)·`editor-minimap.rs`·`editor-inline-suggest-toolbar.rs`·`command-registry.rs`, `UI/editor-keymap-defaults.json`(필요 시 `⌘→` 등), `MOCK`(semantic 모드), 검사·QA·대응표 | 없음(결정 1 B면 epaint vendor 패치) |
| E2 미리보기·Markdown | preview-3, 5, 6, 14, 15, 38, editor-72 (7) | 단계 0, 결정 7·8·9·10, shell lane과 `preview_web_view.rs` 조율 | `APP/domains/preview.rs`, `APP/preview.rs`, `preview_status.rs`, `preview_web_media.rs`, `preview_web_cache.rs`, `preview_web_view.rs`(파일 끝 메서드만), `APP/editor-markdown*.rs`, `UI/editor-markup*.rs`, 새 `APP/application-preview-tests.rs`, `application-auxiliary-window-tests.rs`(1건 추가), 검사·QA·대응표 | 없음 |
| E3 내장 서비스 1: JSON 조회·완성 | editor-57(부분 진척) | E1의 completion Command 정리 | `ED/json-completion.rs`·`json-hover.rs`, `APP/editor-json.rs`·`editor-completion-provider.rs`·hover 소비, oracle `.mjs/.rs` | 없음 |
| E4 내장 서비스 2: CSS/SCSS + 색 UI | editor-57, 16·32·49의 CSS 부분 | E3 | `ED/css-*.rs`, `ED/web-data/css-*.rs`(생성), `ED/color.rs`, `ED/json-colors.rs`, `APP/editor-css.rs`·`editor-colors.rs`, `UI/editor-color-decorators.rs`, `APP/editor-formatting.rs`·저장 포맷 참여자(공급자 추가만), oracle | 없음 |
| E5 내장 서비스 3: HTML | editor-57, 16·32·49의 HTML 부분 | E4(beautify-css·색 없음 확인), 결정 2(링크) | `ED/html-*.rs`, `ED/web-data/html-*.rs`(생성), `APP/editor-html.rs`, 링크 UI 공유, oracle | 없음 |
| E6 내장 서비스 4: TS/JS | editor-57, 16, 32, 49 마감 | E5, 결정 4 | 결정 4에 따름(`APP/editor-builtin-typescript.rs` 등) | 결정 4 |
| 위임 | editor-73, 74, 75, 76 → git lane G5·G7·G8 / editor-81 → explorer-search lane C | 각 lane | 이 lane은 수정 없음(1.5의 탭 내용 hook만 필요 시 E1 첫 작업) | — |

E1 안의 순서: 58(검사만) → 78(검사만) → 77(toolbar·부분 수락) → 7(semantic) → 15(minimap) → 4(리거처 조사·연결) → 34(액션 표, 77·결정 2 결과 사용). E2 안의 순서: preview-3 → 6 → 5 → 14·15 → 38 → editor-72. E1과 E2는 수정 파일이 겹치지 않아 하위 lane 병렬이 가능하지만 디스크 측정(운영 문서 2.2) 전에는 직렬을 기본으로 합니다. E3~E6은 E1 뒤에 직렬입니다(공급자 목록·completion 경로 공유).

묶음마다 끝에: 대응표 해당 행·QA 갱신, 바꾼 crate 전체 대상 `--no-fail-fast` 1회(앱은 `--test-threads=4`), 동결 미러 두 명령, fmt, 메인이 workflow 종료 뒤 선별 커밋.

---

## 4. 결정·허락

### 4.1 사용자 결정이 필요한 항목 (추천안 먼저)

1. **리거처(editor-4)에 epaint API가 없을 때** — **A. 2.1의 조사로 API가 없음을 확인하면 "egui 한계"를 근거와 함께 not-applicable로 정정(추천: 새 vendor 패치 없이 끝남)** / B. epaint를 vendor 패치해 feature 전달을 추가(허락 필요, 업스트림 갱신 부담) / C. 글꼴 데이터의 GSUB `liga`·`calt` feature를 잘라 낸 face를 따로 등록(우회에 가까워 비추천).
2. **원본 기본 기능이지만 대응표 행이 없는 것(1.4: Unicode 강조, 텍스트 링크 감지, 선택 텍스트 발생 강조가 없다면 그것까지)** — **A. 새 행(editor-88~)으로 추가하고 이 lane E1에서 구현(추천: editor-34의 액션이 이 기능 없이 실행 경로를 가질 수 없음)** / B. 새 행만 추가하고 구현은 배치 31 뒤로, editor-34는 그 액션들을 뺀 범위로 판정. 색 장식·색 선택기는 어느 쪽이든 E4에서 구현합니다.
3. **서식 있는 복사의 clipboard 형식** — **A. arboard에 HTML 쓰기가 있으면 HTML+텍스트, 없으면 텍스트만 복사하고 비고에 적음(추천)** / B. 플랫폼 pasteboard 직접 구현(새 코드량 큼).
4. **내장 언어 서비스 구현 방식(E4·E5·E6, editor-57)** — 원본은 Monaco 번들의 vscode-css/html-languageservice(`MEXT`)와 TypeScript 서비스(`MON/languages/features/typescript/lib/typescriptServices.js` 9MB, lib.d.ts 3MB)를 web worker에서 실행합니다. 서버 프로세스나 설치가 필요 없습니다.

    | 선택지 | CSS/SCSS (E4) | HTML (E5) | TS/JS (E6) |
    | --- | --- | --- | --- |
    | 기존 의존 재사용 | 앱 lockfile의 `cssparser`(dom_query 경유)는 오류 복구 위치·SCSS·lint·완성 자료가 원본과 다르고 portable core(`ED`, wasm)에 없어 서비스 근거로 부족 | `html5ever`(앱)는 원본 scanner의 오류 허용 offset·노드 범위를 주지 않음 | 해당 의존 없음 |
    | 순수 Rust 이식(JSON 방식, oracle 대조) | **추천**: 파서 2.6천 줄·서비스·beautify-css 1.7천 줄, 자료는 생성 표. 새 의존 없음 | **추천**: scanner/parser 570줄·서비스·beautify-html 3.2천 줄, 자료 생성 표. 새 의존 없음 | 파서 기반(syntax 진단·심볼·포맷·onType)만 가능. checker가 필요한 완성·hover·정의·참조·rename·code action·inlay는 원본과 같아질 수 없음 |
    | 외부 서버 | vscode-css-language-server 번들/실행 필요(Node 의존, 원본은 서버 없음) | 같음 | `vtsls` 등 기존 manifest 경로. 서버 미설치 사용자에게는 기능 부재 = 사실상 not-applicable |
    | 임베디드 JS 엔진(예: QuickJS 바인딩)에서 원본 번들 실행 | 가능하나 이식 대비 이득 작음 | 같음 | **추천**: 원본 TS 서비스를 그대로 실행하고 Monaco 어댑터(`typescript/languageFeatures.js`)만 Rust로 이식. checker 기능까지 원본과 같은 결과. 새 의존성·바이너리 크기·시작 시간을 재는 spike 1회 필요, 허락 필요 |

    추천 묶음: **CSS/SCSS·HTML = 순수 Rust 이식, TS/JS = 임베디드 JS 엔진(허락 뒤)**. TS/JS에서 새 의존을 허락하지 않으면 순수 Rust 범위로 줄이고 editor-57 비고에 checker 기능 부재를 적은 뒤 판정할지(B′) 함께 정해 주십시오.
5. **탭 내용 분기 hook의 소유** — **A. 단계 0에서 만들기(추천: wave 1의 explorer-search·이 lane 둘 다 필요)** / B. 이 lane E1의 첫 작업으로 만들고 explorer-search lane이 병합 뒤 사용.
6. **`gotoLine`(⌃G)·`quickOutline`(⇧⌘O)의 화면** — **A. 기존 팔레트의 `:`·`@` 모드로 열기(추천: 같은 자료·동작, 화면 중복 없음)** / B. Monaco standalone quick input을 편집기 안에 따로 구현.
7. **editor-72의 닫을 범위** — **A. 2.10의 목록(br·혼합/중첩 code·문단 안 이미지·kbd/samp·raw input·첫 블록 여백·details/summary·앵커·AccessKit)까지 구현하고 임의 CSS·SVG·MathML·미디어·counter는 비대상으로 근거와 함께 기록(추천)** / B. 정화된 HTML을 기존 승인 WebView로 그려 원본 엔진(WKWebView)과 같게(macOS 전용, 200ms 갱신 때 scroll 유지를 위해 host 스크립트 필요, 기존 native 렌더 폐기) / C. 현재 범위로 complete.
8. **이미지 해석 실패 화면(preview-6)** — **A. 파일 이름만 가운데 표시(추천: 원본 브라우저 대체 문구 대응)** / B. 읽기 실패와 같은 UnsupportedPreview(원본보다 친절하지만 화면이 다름).
9. **미디어 theme 변경 시 재생 유지(preview-14/15)** — **A. host `evaluate_script`로 CSS 변수만 갱신, 페이지 JS는 계속 끔(추천: 원본과 같은 상태 유지, `allowsContentJavaScript` 확인 전제)** / B. 활성 미디어는 다시 만들지 않고 색은 다음 열기에 반영 / C. 지금처럼 다시 만듦(재생 초기화, 원본과 다름을 비고에 기록).
10. **preview-38 비macOS** — **A. 원본 출시 대상이 macOS뿐이므로 not-applicable로 정정(추천)** / B. Windows/Linux 경로 구현(WebView2/WebKitGTK wry 기능, PDF 렌더러 의존 추가, 다른 OS CI — 허락 필요).

### 4.2 허락이 필요한 항목 (자동 검사에서는 하지 않음)

1. 새 의존성·vendor 패치: 결정 1 B(epaint 패치), 결정 4의 TS/JS 임베디드 JS 엔진(또는 순수 Rust 범위에서 새 parser crate를 쓰는 경우), 결정 10 B(비macOS 렌더러). manifest·lockfile 변경 포함.
2. GUI 실행: WKWebView 실제 재생·seek·컨트롤·키보드/AX(preview-14/15), Markdown·미리보기 픽셀 비교, 리거처 글꼴 실기(editor-4, 사용자 글꼴 Fira Code 등).
3. 환경 변경: 비macOS 컴파일 확인용 `rustup target add`(preview-38).
4. 사용자 휴지통을 쓰는 앱 안 삭제 경로(preview-5의 5번 사례) — 추적 파일·주입 경로가 없으면 실행하지 않습니다.
5. 실제 clipboard(서식 있는 복사·pasteAs)와 실제 외부 앱 열기(preview-6) — 주입 포트로만 검사합니다.
6. 실제 AI provider·Keychain(editor-77/78) — 주입 응답·메모리 SecretStore만 씁니다.
