# Native 전환 감사 — 코드 편집기·LSP·스니펫·AI 편집 (2026-10-06)

읽기 전용 감사입니다. 빌드·테스트는 실행하지 않았고, 모든 판정은 실제 파일과 호출 체인 검색으로 내렸습니다. 경로는 저장소 루트(`/Users/hyunseokbyun/development/TAIDE`) 기준 상대 경로입니다.

## 1. 결론

- native 편집기(`native/taide-native-ui/src/editor_surface.rs`, 724줄)는 **단색 평문 편집 위젯**입니다. rope 문서 저장소·IME·저장·hot exit·충돌 배너·untitled·app-file·LSP 문서 동기화와 진단·저장 시 포맷은 실제 앱에 연결돼 있습니다.
- Monaco 가 제공하던 편집 기능(구문 강조, 찾기/바꾸기, 접기, minimap, word wrap, 다중 커서 조작, 편집 명령 약 150개, 컨텍스트 메뉴)과 LSP 상호작용 UI(자동완성, hover, signature help, 정의·참조 이동, rename, code action, inlay hint, semantic token, 진단 밑줄)는 **native 에 없습니다.**
- 스니펫 삽입 엔진(`native/taide-native-editor/src/snippet-*.rs`)과 LSP 기능 프로토콜 계약(`crates/taide-lsp/src/native/feature.rs`)은 구현돼 있으나 실제 앱 경로에서 호출되지 않습니다.
- 기능 87행 중 done 23, partial 11, unwired 6, missing 45, n/a 2 입니다.

## 2. 범위

### 2.1 읽은 TS 경로

- `src/features/editor/code-editor.tsx`(전체), `ai-inline-edit.ts`, `conflict-banner.tsx`, `editor-group-shortcut-actions.ts`, `markdown-preview.tsx`, `breadcrumb-segment.tsx`, `blame-footer-bar.tsx`
- `src/widgets/editor-pane/editor-pane.tsx`(전체), `untitled-pane.tsx`, `use-lsp-session.ts`, `use-editor-git-gutter-and-conflicts.ts`, `use-editor-blame.ts`, `breadcrumbs-bar.tsx`, `use-editor-*.ts`(검색 기반)
- `src/widgets/search-editor/search-editor-pane.tsx`, `src/widgets/app-file-pane/app-file-pane.tsx`(검색 기반)
- `src/entities/lsp/lsp-session-registry.ts`(등록부 중심), `src/entities/{editor,ai,snippet,app-file,file}`(목록·검색)
- Monaco 설정·확장: `src/shared/lib/monaco/setup.ts`, `monaco-actions.ts`, `src/shared/lib/shiki/lang-map.ts`, `src/shared/lib/lsp/adapters/*.ts`(등록 API 검색), `command-relay.ts`, `src/shared/lib/emmet-integration.ts`, `snippet-completion.ts`, `ai/inline-completion.ts`, `src/features/git/diff-view.tsx`, `src/widgets/claude-diff-pane/claude-diff-pane.tsx`
- inventory: `docs/quality-assurance/2026-09-28-ts-view-inventory.md`, `2026-09-29-ts-feature-inventory-a.md`, `-b.md`, `-c.md`, `2026-09-29-ts-provider-inventory.md`

### 2.2 읽은 native 경로

- `native/taide-native-ui/src/editor_surface.rs`(전체), `presentation.rs`, `document_admission.rs`, `conflict_banner.rs`, `snippet-completion.rs`·`snippet-catalog.rs`(호출부 검색)
- `native/taide-native-editor/src/{lib,editing,view,syntax,document}.rs`(전체), `store.rs`(공개 API·apply·undo), 스니펫 모듈(공개 API·호출부 검색)
- `native/taide-native-app/src/application.rs`(`tab_content` 4751-5128, `show_document` 5215-5396, `reconcile_lsp`·`poll_lsp` 2594-3158, `request_tab_save` 1775-1818), `lsp.rs`(1-350, 1180-1293), `host.rs`(명령 목록), `lib.rs`, `shell_keymap.rs`, `save.rs`·`persistence.rs`·`file_sync.rs`·`untitled.rs`·`missing_draft.rs`·`editor_reveal.rs`·`diagnostics.rs`·`lsp_workspace*.rs`·`app-file*.rs`(공개 API·호출부)
- `crates/taide-lsp/src/native/feature.rs`(전체), `native.rs`(공개 API), `crates/taide-model/src/layout.rs`(TabKind), `crates/taide-file/src/service.rs`(tier 규칙)

### 2.3 실제 앱 호출 체인 (확인됨)

`native/taide-native-app/src/main.rs` → `application.rs` `NativeApplication`(102) → `eframe::App::ui`(3618) → `AppSurfaces::tab_content`(4751) → `show_document`(5215) → `NativeEditor::show_with_input_route`(`editor_surface.rs` 167).

`tab_content` 가 처리하는 탭 종류는 Settings(4759), AppFile(4812), Terminal(4848), Untitled(4901), File(4916 이후)뿐입니다. 그 외(`Diff`, `ClaudeDiff`, `SearchEditor`, `Welcome`)는 4916-4923 에서 제목 라벨과 `"native tab surface is not connected"` 상태 문구만 표시합니다.

## 3. TS 가 의존하는 Monaco 기능 목록

| 구분 | 내용 | 근거 |
| --- | --- | --- |
| 편집기 생성 | `monaco.editor.create` (automaticLayout, largeFileOptimizations, semanticHighlighting) | `src/features/editor/code-editor.tsx:180-201` |
| 편집기 옵션 | folding, bracketPairColorization, minimap, stickyScroll, wordWrap, lineNumbers, tabSize, insertSpaces, detectIndentation, renderWhitespace, fontLigatures, cursorStyle, cursorBlinking, scrollBeyondLastLine, guides.bracketPairs, smoothScrolling, cursorSmoothCaretAnimation, suggest.preview, rulers, formatOnType, formatOnPaste, readOnly, inlineSuggest, fontFamily, fontSize | `code-editor.tsx:271-341` |
| 모델·뷰 상태 | 경로별 모델 공유, `saveViewState`/`restoreViewState`, 모델별 editorconfig 들여쓰기 | `code-editor.tsx:360-409`, `src/entities/editor/model-registry.ts` |
| 커스텀 액션 | `taide.saveFile`, `taide.toggleMinimap`, `taide.aiInlineEdit`(⌘I), `taide.gitStageSelection`, `taide.toggleBlame`, `taide.openFileHistory`, `taide.runSelectedTextInTerminal`, 그룹 단축키 | `code-editor.tsx:205-268`, `src/widgets/editor-pane/editor-pane.tsx:290-298`, `use-editor-git-gutter-and-conflicts.ts:292-308`, `use-editor-blame.ts:140` |
| 내장 액션 카탈로그 | 찾기·바꾸기, 줄 편집, 다중 커서, 접기, 포맷, 이동·peek, rename, quick fix 등 약 150개를 커맨드 팔레트·키바인딩 편집기에 노출 | `src/shared/lib/monaco/monaco-actions.ts:51-832`, `src/app/bootstrap-commands.ts:13` |
| 토크나이저 | Shiki(TextMate) 31개 언어 + 플러그인 언어 | `src/shared/lib/shiki/lang-map.ts:66-96`, `src/shared/lib/monaco/register-plugin-languages.ts` |
| 내장 언어 서비스 | JSON·CSS·HTML·TS worker, LSP 없을 때 TS/JS fallback | `src/shared/lib/monaco/setup.ts:10-22`, `builtin-typescript.ts:55-56` |
| LSP provider | completion, declaration, definition, documentHighlight, documentSymbol, foldingRange, formatting, hover, implementation, inlayHints, onTypeFormatting, rangeFormatting, references, rename, selectionRange, signatureHelp, typeDefinition, codeAction, codeLens, semanticTokens, diagnostics(`setModelMarkers`) | `src/entities/lsp/lsp-session-registry.ts:36-54, 673-681` |
| LSP 부가 | `workspace/applyEdit`, semanticTokens·codeLens refresh, executeCommand relay, `registerEditorOpener` | `lsp-session-registry.ts:291, 356, 369`, `src/shared/lib/lsp/command-relay.ts:73, 154-156`, `src/shared/lib/bridge/editor-opener-bridge.ts:40` |
| 장식 | Git gutter·conflict 배경, blame after-text, AI 편집 삭제 표시, diff hunk | `use-editor-git-gutter-and-conflicts.ts:211-226`, `use-editor-blame.ts:128-134`, `ai-inline-edit.ts:212` |
| 위젯 | content widget·view zone(AI 인라인 편집), inline completions provider | `ai-inline-edit.ts:137, 230`, `src/shared/lib/ai/inline-completion.ts:182` |
| 스니펫·Emmet | 사용자 스니펫 completion provider, emmet-monaco-es | `src/shared/lib/snippet-completion.ts:95`, `src/shared/lib/emmet-integration.ts:26-33` |
| diff 편집기 | `createDiffEditor` | `src/features/git/diff-view.tsx:46`, `src/widgets/claude-diff-pane/claude-diff-pane.tsx:58` |

## 4. 기능 대응표

effort: S(반나절 이하) / M(1~2일) / L(3~5일) / XL(1주 초과). native 근거의 `ES` 는 `native/taide-native-ui/src/editor_surface.rs`, `APP` 은 `native/taide-native-app/src/application.rs` 입니다.

### 4.1 표시

| # | 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 텍스트 렌더링(보이는 행만) | `code-editor.tsx:180` | partial | ES 297-318, 415 | 행 단위 가상화만 있음. 긴 줄 가로 가상화 없음(행 전체를 매 프레임 `to_string` 후 layout, ES 305-307). 스크롤바 없음 | M |
| 2 | 편집기 글꼴 패밀리 | `code-editor.tsx:335-337` | missing | `native/taide-native-ui/src/presentation.rs:111` | `FontId::monospace` 고정. `editor_font_family` 는 설정 화면(`settings-code-controls.rs:292`)에서만 읽음 | M |
| 3 | 글꼴 크기·줄 높이 | `code-editor.tsx:339-341` | done | `presentation.rs:31-40, 109-112`, APP 5179-5187 | — | — |
| 4 | 리거처 | `code-editor.tsx:290` | missing | — | egui 텍스트 셰이핑에 리거처 경로 없음 | L |
| 5 | 구문 강조(31개 언어 + 플러그인) | `shiki/lang-map.ts:66-96` | missing | `native/taide-native-editor/src/syntax.rs:2-7` | 토크나이저·색 렌더 모두 없음. `SyntaxSnapshot` 은 Other/Comment/String/Regex 4종 모델뿐이고 `install_syntax`(`store.rs:230`)는 호출부가 없음. galley 를 전경색 단색으로 그림(ES 306-307) | XL |
| 6 | 편집기 기본 색(배경·전경·선택·커서·줄 강조·줄 번호) | `src/shared/lib/monaco/theme.ts` | done | `presentation.rs:105-127` | — | — |
| 7 | semantic token 강조 | `lsp/adapters/semantic-tokens.ts:236` | missing | `crates/taide-lsp/src/native/feature.rs:133-134`(계약만) | 요청·디코딩·렌더 없음 | L |
| 8 | 줄 번호·gutter | `code-editor.tsx:285` | done | ES 416-427, 669-689 | — | — |
| 9 | 현재 줄 강조 | Monaco 기본 | done | ES 362-371 | — | — |
| 10 | 커서 스타일·깜빡임·부드러운 캐럿 | `code-editor.tsx:291-296` | missing | ES 397-404 | 1px 세로선 고정, 깜빡임 없음 | S |
| 11 | 공백 문자 표시 | `code-editor.tsx:289` | missing | — | 설정 컨트롤만 존재 | M |
| 12 | 괄호 쌍 색·가이드 | `code-editor.tsx:271, 294` | missing | — | 설정 컨트롤만 존재 | L |
| 13 | 괄호 매칭 강조·괄호로 이동 | `monaco-actions.ts:343, 543` | missing | — | — | M |
| 14 | rulers | `code-editor.tsx:298` | missing | — | — | S |
| 15 | minimap 과 토글 액션 | `code-editor.tsx:246-252, 275` | missing | — | 설정 컨트롤만 존재 | L |
| 16 | sticky scroll | `code-editor.tsx:279` | missing | — | 설정 컨트롤만 존재 | L |
| 17 | word wrap | `code-editor.tsx:284` | missing | ES 306(`layout_no_wrap`) | 표시 줄 모델이 없음 | L |
| 18 | 스크롤 | `code-editor.tsx:293, 295` | partial | ES 269-287 | 휠 스크롤만. 스크롤바, scrollBeyondLastLine, smoothScrolling, PageUp/Down 없음. 가로 스크롤 상한 없음(ES 275) | M |
| 19 | 대형 파일 tier | `editor-pane.tsx:422`, `code-editor.tsx:271-276` | partial | APP 5241-5253, `native/taide-native-app/src/lsp.rs:614` | read-only 안내와 LSP 제외는 있음. 긴 줄 성능 대책 없음(1번 행) | M |
| 20 | 접기(들여쓰기·LSP folding range, 접기 명령 20여 개) | `code-editor.tsx:271`, `monaco-actions.ts:747-826`, `lsp/adapters/folding-range.ts:30` | unwired | `native/taide-native-editor/src/view.rs:120`, `store.rs:1195-1212` | `ViewState.folds` 필드와 편집 시 매핑만 있음. 접기를 만들거나 그리는 코드 없음 | L |

### 4.2 편집

| # | 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- | --- |
| 21 | 문자 입력·삭제·Enter·Tab | Monaco 기본 | partial | ES 499-501, 611-630 | Enter 자동 들여쓰기 없음. Shift+Tab 내어쓰기 없음(ES 628). 선택 영역 Tab 은 줄 들여쓰기가 아니라 치환 | M |
| 22 | 커서 이동 | Monaco 기본 | partial | ES 571-609, `editing.rs:114-174` | 화살표·Home/End·문서 처음/끝만. alt/ctrl 조합은 전부 미처리(ES 591-596)라 단어 이동, ⌘←/→, ⌥⌫, ⌘⌫ 없음. PageUp/Down 없음. 세로 이동 시 goal column 미보존(`editing.rs:135, 150-155`) | M |
| 23 | 마우스 선택 | Monaco 기본 | partial | ES 319-354 | 클릭·드래그·shift 클릭만. 더블 클릭 단어, 트리플 클릭 줄, 드래그 자동 스크롤, 줄 번호 클릭 없음 | M |
| 24 | 다중 커서·컬럼 선택 | `monaco-actions.ts:91-110, 306-318` | unwired | `view.rs:25-28`, `editing.rs:228-294` | `SelectionSet` 과 다중 편집 적용은 있으나 다중 선택을 만드는 입력 경로가 없음(ES 339-342 는 항상 단일 선택) | L |
| 25 | undo/redo | Monaco 기본, `monaco-actions.ts:79-80` | partial | ES 573-582, `store.rs:1217-1275` | 동작은 하지만 키 입력이 한 글자씩 undo 됨(6.2 참조). cursorUndo 없음 | S |
| 26 | 클립보드 | Monaco 기본 | partial | ES 486-498, 247-249 | 선택이 없을 때 줄 복사·잘라내기 없음, 오히려 클립보드를 빈 문자열로 덮어씀(6.1). 다중 커서 붙여넣기 분배 없음 | S |
| 27 | IME 조합 | Monaco 기본 | done | ES 503-564, 429-461, `native/taide-native-ui/tests/editor_surface.rs:332` | preedit 는 뒤 글자를 밀지 않고 덮어 그림(ES 440-442) | — |
| 28 | 찾기/바꾸기 위젯 | `monaco-actions.ts:66-72, 409-461, 609` | missing | — | `keymap-defaults.json:33` 에 `find` 기본 키만 있고 `shell_keymap.rs:9-39` 지원 목록에 없음 | L |
| 29 | 줄 이동·복제·삭제·주석 토글·join·sort·대소문자 변환 등 편집 명령 | `monaco-actions.ts:81-176, 268-341, 385-392, 481-506, 535, 591-694` | missing | — | 명령 구현 0개 | XL |
| 30 | 들여쓰기 설정(tabSize·insertSpaces·editorconfig) | `code-editor.tsx:286-288, 405-409` | partial | APP 5288-5296, `native/taide-native-editor/src/indent.rs:9` | detectIndentation 없음. 들여쓰기 변환·indent/outdent 명령 없음 | M |
| 31 | 자동 닫힘 괄호·따옴표·감싸기 | Monaco 기본 | missing | — | — | M |
| 32 | formatOnType·formatOnPaste | `code-editor.tsx:299-300`, `lsp/adapters/formatting.ts:53` | missing | `feature.rs:125`(계약만) | — | M |
| 33 | 컨텍스트 메뉴 | `code-editor.tsx:249`, `editor-pane.tsx:293` | missing | — | 보조 클릭 처리 없음 | M |
| 34 | Monaco 액션의 팔레트·키바인딩 노출 | `src/shared/lib/monaco/monaco-action-commands.ts:60` | unwired | `native/taide-native-ui/src/keybinding-commands.json`(Monaco 액션 id 274행) | 키바인딩 편집기 목록에는 나오나 실행할 편집기 명령이 없음 | 29번에 종속 |
| 35 | 편집기 포커스 상태의 그룹·탭 단축키 | `editor-group-shortcut-actions.ts:28` | done | APP 5308-5336, `shell_keymap.rs:9-39` | — | — |
| 36 | 읽기 전용 | `code-editor.tsx:323`, `editor-pane.tsx:535-539` | done | `store.rs:1109-1111`, APP 5241-5253 | 읽기 전용 문서에 입력하면 상태줄에 `native editor: ReadOnly` 가 표시됨(APP 5387-5389) | — |

### 4.3 LSP

| # | 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- | --- |
| 37 | 세션 수명(시작·루트·idle·crash 복구·상태 표시) | `use-lsp-session.ts:163-193`, `lsp-session-registry.ts:783-917` | done | `lsp.rs:39-49, 296-334`, APP 5141-5146 | — | — |
| 38 | 문서 동기화(open/change/close/save) | `lsp-session-registry.ts:688-701` | done | APP 2594-2649, `lsp.rs:336-349, 492`, `crates/taide-lsp/src/native.rs:296-381` | 보이는 활성 탭만 동기화(TS 와 동일) | — |
| 39 | 진단 수신 → problems·상태바 | `lsp/adapters/diagnostics.ts:97` | done | APP 3062-3081, 5132-5140, `diagnostics.rs:78` | — | — |
| 40 | 진단 밑줄·hover 메시지·다음/이전 문제 이동 | `diagnostics.ts:97`, `monaco-actions.ts:349-368` | missing | — | 편집기 표면에 장식 계층 없음 | M |
| 41 | 자동완성 UI | `lsp/adapters/completion.ts:62`, `code-editor.tsx:297` | missing | `feature.rs:124`(계약만) | 요청·팝업·resolve·삽입 없음 | XL |
| 42 | hover | `lsp/adapters/hover.ts:17` | missing | `feature.rs:108`(계약만), `lsp-diagnostics-tests.rs:725`(테스트만) | — | L |
| 43 | signature help | `lsp/adapters/signature-help.ts:14` | missing | `feature.rs:126`(계약만) | — | M |
| 44 | 정의·선언·타입 정의·구현 이동 | `lsp/adapters/definition.ts:49`, `declaration.ts:7`, `type-definition.ts:7`, `implementation.ts:7` | missing | `feature.rs:109, 120-122`(계약만) | — | L |
| 45 | 참조 찾기·peek | `lsp/adapters/references.ts:12`, `peek-model-preload.ts:92` | missing | `feature.rs:110`(계약만) | — | L |
| 46 | rename | `lsp/adapters/rename.ts:17` | missing | `feature.rs:111, 130`(계약만) | 입력 위젯 없음. WorkspaceEdit 적용부(`lsp_workspace.rs:22`)는 재사용 가능 | M |
| 47 | code action·quick fix·refactor | `lsp/adapters/code-action.ts:232` | missing | — | 수동 호출 UI 없음 | L |
| 48 | 저장 시 fixAll·organizeImports | `editor-pane.tsx:114-115` | done | APP 1399-1402, `lsp.rs:1186-1293` | — | — |
| 49 | 포맷 | `lsp/adapters/formatting.ts:14, 34`, `editor-pane.tsx:141` | partial | APP 3082-3155, `lsp.rs:496` | 저장 시 포맷만 있음. 수동 문서·선택 포맷 명령 없음. JSON·CSS·HTML·TS 내장 포매터 대응 없음 | M |
| 50 | inlay hint | `lsp/adapters/inlay-hints.ts:23` | missing | `feature.rs:114`(계약만) | — | L |
| 51 | document highlight | `lsp/adapters/document-highlight.ts:26` | missing | `feature.rs:116`(계약만) | — | M |
| 52 | selection range(스마트 선택) | `lsp/adapters/selection-range.ts:25` | missing | `feature.rs:117`(계약만) | — | M |
| 53 | code lens | `lsp/adapters/code-lens.ts:105` | missing | `feature.rs:128, 132`(계약만) | — | L |
| 54 | document symbol(quick outline·breadcrumb 공급) | `lsp/adapters/document-symbol.ts:77` | missing | `feature.rs:115`(계약만) | outline 패널 자체는 다른 영역 | M |
| 55 | workspace/applyEdit 와 파일 작업 연동 | `lsp-session-registry.ts:291` | done | APP 3031-3059, `lsp_workspace.rs:22`, `workspace_rename.rs`, `workspace_delete.rs` | — | — |
| 56 | executeCommand relay(rust-analyzer showReferences 등) | `command-relay.ts:73, 154-156` | missing | `lsp.rs:1246`(저장 액션의 command 실행만) | — | M |
| 57 | Monaco 내장 언어 서비스(JSON·CSS·HTML·TS) | `monaco/setup.ts:10-22`, `builtin-typescript.ts:55-56` | missing | — | LSP 서버가 없으면 진단·완성·포맷이 전혀 없음 | 서버 번들 결정 필요 |
| 58 | 파일 간 이동 opener | `bridge/editor-opener-bridge.ts:40` | missing | — | 44번에 종속. reveal 큐(`editor_reveal.rs:31`)는 재사용 가능 | S |

### 4.4 파일 수명

| # | 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- | --- |
| 59 | 파일 열기·로딩·오류 | `editor-pane.tsx:363-412` | partial | APP 5116-5127, `document_admission.rs:133` | 바이너리·초대형(refused) 전용 안내와 외부 열기 버튼 없음. 오류 문자열 라벨만 표시 | S |
| 60 | 저장(⌘S)·공백 정리·마지막 개행 | `use-editor-file-persistence.ts`, `monaco/on-save-cleanup.ts` | done | APP 1775-1818, 1479-1485, `save_cleanup.rs:20` | — | — |
| 61 | 자동 저장 | `editor-pane.tsx:140` | done | APP 2453-2488, 2511, `persistence.rs:90` | — | — |
| 62 | hot exit mirror·복원·종료 flush | `editor-pane.tsx:94`, `src/entities/editor/mirror-flush-registry.ts` | done | `persistence.rs:15, 125-160`, APP 2488-2560, 4376-4500 | — | — |
| 63 | 외부 변경·복원 충돌 배너 | `conflict-banner.tsx:40-59`, `editor-pane.tsx:540-547` | done | `conflict_banner.rs:14-92`, APP 5225-5240, 2336 | — | — |
| 64 | 삭제된 원본의 draft 구제 | `editor-pane.tsx:202-220, 382-395` | done | APP 5092-5115, 2204, `missing_draft.rs` | — | — |
| 65 | untitled 편집·Save As·mirror | `untitled-pane.tsx:76-191` | done | APP 4901-4914, 2110, `untitled.rs:50, 66` | — | — |
| 66 | app-file(settings.json 등) 편집 | `app-file-pane.tsx:57-171` | done | APP 4812-4846, 1781-1794, `app-file-views.rs`, `app-file-write.rs` | — | — |
| 67 | 뷰 상태(커서·스크롤) 저장과 재시작 복원 | `use-editor-view-state.ts`, `code-editor.tsx:365-370` | unwired | `native/taide-native-app/src/remote-layout.rs:144` | `layout_set_view_state` 는 원격 gateway 에서만 호출됨. native 편집기는 세션 내 `ViewState` 만 유지 | M |
| 68 | 줄·열 reveal | `editor-pane.tsx:271-274` | done | ES 82-134, APP 645, 5270-5280, `editor_reveal.rs:31-57` | — | — |
| 69 | 인코딩·EOL | `crates/taide-file/src/service.rs:96-115`(TS 도 UTF-8 한정) | done | `document.rs:45-105` | Enter 의 줄바꿈 종류를 메타데이터가 아닌 첫 줄로 추정(ES 618-625) | — |
| 70 | dirty 표시 연동 | `editor-pane.tsx:82` | done | APP 5361-5386 | — | — |

### 4.5 부가 기능

| # | 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- | --- |
| 71 | breadcrumbs bar(경로·심볼 dropdown) | `breadcrumbs-bar.tsx:62-194`, `breadcrumb-segment.tsx` | missing | — | — | L |
| 72 | Markdown 미리보기 분할 | `editor-pane.tsx:494-510, 548-564`, `markdown-preview.tsx` | missing | — | Markdown 렌더러 의존성 없음 | L |
| 73 | Git gutter·hunk 되돌리기·선택 stage | `use-editor-git-gutter-and-conflicts.ts:58, 224, 238, 292` | missing | `native/taide-native-app/src/remote-git.rs:20, 142`(원격 gateway 만) | 장식 계층·gutter 클릭 없음 | L |
| 74 | merge conflict 표시·해결 dialog | `use-editor-git-gutter-and-conflicts.ts:211-219`, `editor-pane.tsx:520-533` | missing | — | — | L |
| 75 | blame(줄 footer·overlay·토글·파일 이력) | `use-editor-blame.ts:54, 116-143`, `blame-footer-bar.tsx` | missing | `remote-git.rs:21, 145`(원격 gateway 만) | — | M |
| 76 | diff 편집기(Git diff·commit diff·Claude diff·conflict 비교) | `features/git/diff-view.tsx:46`, `claude-diff-pane.tsx:58` | missing | APP 4916-4923, `ide-tools.rs:144`(탭만 생성) | `TabKind::Diff`·`ClaudeDiff` 표면 없음 | XL |
| 77 | AI 인라인 완성(ghost text, Tab 수락) | `code-editor.tsx:327-333`, `ai/inline-completion.ts:182` | missing | `native/taide-native-app/src/remote-ai.rs`(원격 gateway 만), `crates/taide-ai`(백엔드) | 편집기 UI 없음 | L |
| 78 | AI 인라인 편집(⌘I, 미리보기·수락·거절·취소) | `ai-inline-edit.ts:86-349` | missing | 위와 동일 | 편집기 UI 없음 | L |
| 79 | 사용자 스니펫 완성·삽입(tabstop·choice·변수) | `snippet-completion.ts:95`, `src/app/bootstrap-snippets.ts` | unwired | `native/taide-native-editor/src/snippet-insertion.rs:31`, `snippet-session.rs:57-367`, `native/taide-native-ui/src/snippet-completion.rs` | 엔진은 있으나 `insert`·`Session`·`collect` 호출부가 테스트뿐(`snippet-host-tests.rs:291`, `native/taide-native-ui/tests/snippet-session.rs`). 완성 팝업(41번) 필요 | M(41번 이후) |
| 80 | Emmet | `emmet-integration.ts:26-33`, `src/app/providers/emmet-provider.tsx` | missing | `settings-code-controls.rs`(토글만) | — | L |
| 81 | search-editor 탭 | `search-editor-pane.tsx:65-195` | missing | APP 4916-4923 | `TabKind::SearchEditor` 표면 없음 | L |
| 82 | 선택 텍스트를 터미널에서 실행 | `editor-pane.tsx:288-300` | missing | — | — | S |
| 83 | IDE(Claude Code) 선택 동기화 | `use-editor-ide-selection.ts` | unwired | `native/taide-native-app/src/remote-ide.rs:45`, `ide-tools.rs:327` | `ide_set_selection` 을 native 편집기가 호출하지 않음. `getCurrentSelection` 은 갱신되지 않는 값을 반환 | S |
| 84 | 상태바 커서 위치·글꼴 크기 조절 | `src/widgets/window-chrome/status-bar-content.tsx` | done | APP 5170-5208, `status-editor.rs:100-290` | — | — |
| 85 | 플러그인 언어 등록 | `monaco/register-plugin-languages.ts` | missing | — | 5번에 종속 | 5번에 포함 |
| 86 | Monaco worker·automaticLayout·React fiber 보호 장치 | `monaco/setup.ts:10-18`, `code-editor.tsx:181, 230-241, 454-464`, `editor-pane.tsx:360-361` | n/a | — | 웹 런타임·React 수명 전용이라 native 대응 불필요 | — |
| 87 | 파일 열기 성능 측정 mark | `editor-pane.tsx:315-322` | n/a | — | 브라우저 Performance API 전용 계측 | — |

## 5. missing 판정에 사용한 검색어

검색 범위는 `native/taide-native-app/src`, `native/taide-native-ui/src`, `native/taide-native-editor/src`, `native/taide-native-retained/src`, `native/taide-remote-web/src` 이며 `target`·`vendor` 는 제외했습니다.

| 기능 | 검색어 | 결과 |
| --- | --- | --- |
| 구문 강조 | `syntect`, `tree_sitter`, `highlight`, `install_syntax`, `SyntaxSnapshot`, `TokenKind` | 스니펫·저장 정리용 4종 token 과 정의부만 |
| 찾기/바꾸기 | `find_widget`, `find_next`, `replace_all`, `actions.find`, `FindWidget` | 편집기 관련 없음(스프레드시트 미리보기의 문자열 치환만) |
| 접기·minimap·wrap·sticky | `fold_range`, `toggle_fold`, `minimap`, `word_wrap`, `sticky_scroll` | 설정 컨트롤(`settings-controls.rs`, `settings-code-controls.rs`)만 |
| 편집 명령 | `comment_line`, `toggle_comment`, `move_lines`, `duplicate_line`, `join_lines`, `editor.action.` | `keybinding-capture.rs` 와 카탈로그 json 만 |
| 다중 커서 | `multi_cursor`, `add_cursor`, `column_select`, `addSelectionToNextFindMatch` | 없음 |
| 괄호 | `bracket_match`, `auto_close`, `bracket_pair` | 테마 token 이름과 설정 컨트롤만 |
| LSP UI | `textDocument/hover`, `HoverRequest`, `completion_popup`, `signature_help`, `goto_definition`, `inlay`, `semantic_token`, `code_lens`, `quick_fix`, `peek` | `lsp.rs:1235` 의 CodeActionRequest(저장 액션)와 테스트만 |
| Git 장식 | `git_gutter`, `blame`, `hunk`, `ConflictRegion` | `remote-git.rs` gateway 와 테마 token 이름만 |
| diff·search-editor | `TabKind::Diff`, `ClaudeDiff`, `DiffView`, `SearchEditor`, `searchEditor` | `tabs.rs:128` 의 닫기 처리만 |
| AI·Emmet | `ai_inline`, `InlineEdit`, `inline_completion`, `ghost`, `emmet` | `remote-ai.rs` gateway 와 설정 컨트롤만 |
| breadcrumb·Markdown | `breadcrumb`, `markdown_preview`, `toggleMarkdownPreview`, `pulldown`, `comrak` | 없음 |
| 글꼴 패밀리·리거처 | `editor_font_family`, `FontFamily::Name`, `ligature` | 터미널·UI 글꼴과 설정 컨트롤만 |
| 뷰 상태 | `view_state`, `layout_set_view_state`, `SetViewState` | `remote-layout.rs:144` gateway 만 |

## 6. 잘못 구현됐거나 보강이 필요한 native 코드

1. **선택이 없을 때 복사·잘라내기가 클립보드를 지웁니다.** `Event::Copy`·`Event::Cut` 가 빈 선택에서도 `output.copied = Some("")` 을 만들고(ES 486-494) 그대로 `copy_text` 합니다(ES 247-249). Monaco 는 현재 줄을 복사합니다.
2. **undo 가 키 입력 한 글자 단위입니다.** 모든 편집이 `UndoGroup(document.revision)` 을 쓰는데(`native/taide-native-editor/src/editing.rs:281`, ES 711) revision 은 편집마다 1씩 늘어나므로 병합 조건 `last.group == entry.group`(`store.rs:1163-1168`)이 연속 입력에서 성립하지 않습니다.
3. **macOS 표준 이동·삭제 키가 동작하지 않습니다.** `modifiers.command` 분기는 A/Z/Y/Home/End 만 처리하고(ES 571-590), alt·ctrl·mac_cmd 조합은 전부 `Ok(false)` 로 흘립니다(ES 591-596). ⌘←/→/↑/↓, ⌥←/→, ⌥⌫, ⌘⌫ 가 없어 Home/End 키가 없는 키보드에서는 줄 처음·끝으로 갈 수 없습니다.
4. **Enter 가 들여쓰기를 유지하지 않고 Shift+Tab 이 없습니다.** Enter 는 줄바꿈 문자만 넣고(ES 617-627), Tab 은 `!modifiers.shift` 일 때만 처리하며 선택 영역을 들여쓰기 문자열로 치환합니다(ES 628-630).
5. **스크롤바가 없고 가로 스크롤 상한이 없습니다.** 휠 delta 만 반영하며(ES 269-287) `scroll.x` 는 `max(0.0)` 만 적용합니다(ES 275).
6. **세로 이동이 goal column 을 보존하지 않습니다.** 매번 현재 열을 다시 계산하므로 짧은 줄을 지나면 열이 줄어든 채 유지됩니다(`editing.rs:135, 150-155`). 열 단위도 char 수라 탭·전각 문자에서 시각 열과 어긋납니다.
7. **긴 줄 성능 대책이 없습니다.** 보이는 행마다 줄 전체를 `to_string` 한 뒤 `layout_no_wrap` 합니다(ES 302-307). minified 파일 한 줄이 그대로 매 프레임 layout 대상이 됩니다.
8. **편집기 키맵이 터미널 모듈에 묶여 있습니다.** `show_document` 가 `terminal_views.route_keymap` 으로 키를 라우팅합니다(APP 5310-5329, `terminal_surface.rs:3107`). 편집기 명령 계층을 추가하려면 키맵 소유자를 분리해야 합니다.
9. **키바인딩 카탈로그와 실행 계층이 어긋납니다.** `keybinding-commands.json` 은 Monaco 액션 id 를 274행 나열하지만 `shell_keymap.rs:9-39` 가 지원하는 액션은 셸 동작 20여 개뿐입니다. 사용자가 편집기 액션을 재바인딩해도 아무 일도 일어나지 않습니다.
10. **확장 지점이 없는 표면 구조입니다.** `EditorAppearance`(ES 26-38)와 `show_with_input_route` 는 장식(밑줄·gutter 표시·after-text)·오버레이 위젯·표시 줄 매핑을 받을 인자가 없습니다. 구문 강조, 진단 밑줄, Git gutter, inlay hint, AI ghost text, 접기, wrap 이 모두 이 계층을 요구합니다.
11. **Enter 의 줄바꿈 종류를 첫 줄로 추정합니다.** `DocumentMetadata.line_ending`(`document.rs:89`)이 있는데도 첫 줄 끝 문자를 검사합니다(ES 618-625). 첫 줄에 개행이 없는 CRLF 문서에서 LF 가 섞입니다.
12. **refused 파일과 일반 열기 오류가 구분되지 않습니다.** `EditorError::Refused` 가 일반 오류로 변환돼(`document_admission.rs:133`) 문자열 라벨만 나옵니다(APP 5116-5119).
13. **`install_syntax`·`ViewState.folds` 가 호출부 없는 모델입니다.** `store.rs:230-279`, `view.rs:120`. 토크나이저·접기 구현 방향이 정해지기 전까지 죽은 경로입니다.

## 7. 실제 앱 연결이 끊긴 지점

| 지점 | 있는 것 | 끊긴 곳 |
| --- | --- | --- |
| 스니펫 삽입 | `snippet-insertion.rs:31 insert`, `snippet-session.rs:57 Session`(tabstop 이동·choice·변환), `snippet-completion.rs collect`, 카탈로그 로딩(APP 552, `settings-view.rs:351-412`) | `insert`·`Session::new`·`collect` 의 비테스트 호출부 없음. 편집기에 완성 팝업이 없어 진입점이 없음 |
| LSP 기능 요청 | `crates/taide-lsp/src/native/feature.rs:107-135` 의 27개 method 계약, `SessionClient::request_typed` | `LspBridge` 명령은 Sync·Format·Saved·파일 작업뿐(`lsp.rs:182-227`). hover 등은 테스트(`lsp-diagnostics-tests.rs:725`)에서만 호출 |
| 구문 token | `syntax.rs`, `store.rs:216-279` | `install_syntax` 호출부 없음, 렌더러가 token 을 읽지 않음 |
| 접기 | `view.rs:120`, `store.rs:1195-1212` | 접기 생성·렌더·명령 없음 |
| 다중 선택 | `view.rs:25-28`, `editing.rs:228-294` | 표면이 항상 단일 선택을 기록(ES 339-342) |
| 뷰 상태 영속 | `layout_actions::layout_set_view_state` | native 편집기 호출 없음(`remote-layout.rs:144` 만) |
| IDE 선택·진단 게시 | `ide_actions::ide_set_selection`, `ide_publish_diagnostics` | `remote-ide.rs:45-51` gateway 에서만 호출 |
| Git gutter·blame | `remote-git.rs:142-146` | 브라우저 원격 gateway 전용. native 편집기 표면 없음 |
| AI | `crates/taide-ai`, `remote-ai.rs` | native 편집기 UI 없음 |
| Diff·ClaudeDiff·SearchEditor 탭 | `ide-tools.rs:144 open_diff` 가 탭을 만듦 | `tab_content` 에 표면 없음(APP 4916-4923) |
| 키바인딩 카탈로그 | `keybinding-commands.json`(Monaco 액션 274행) | 실행 계층 없음 |

## 8. 권장 구현 순서

1. **기반 결함 수정(S~M, 선행 조건 없음)** — 6장의 1~6, 11, 12 번. 클립보드, undo 병합, macOS 이동 키, 자동 들여쓰기·Shift+Tab, 스크롤바, goal column, 더블·트리플 클릭.
2. **편집기 명령 계층(L)** — 액션 id → 편집 함수 레지스트리를 `taide-native-editor` 에 두고, 키맵 소유자를 `terminal_surface` 에서 분리(6장 8번)한 뒤 `keybinding-commands.json`·커맨드 팔레트·컨텍스트 메뉴에 연결합니다. 29·33·34번과 이후 모든 명령형 기능의 전제입니다.
3. **표시 계층 재설계(XL)** — 문서 줄 ↔ 표시 줄 매핑(wrap·fold), 장식 API(인라인 색·밑줄·gutter 표시·after-text·줄 사이 블록), 오버레이 위젯 앵커를 `editor_surface.rs` 에 도입합니다. 5, 7, 11~17, 20, 40, 50, 73~78번이 전부 여기에 의존합니다.
4. **구문 강조(XL, 3번 이후)** — 토크나이저 선택(TextMate 문법 엔진 또는 tree-sitter)은 새 의존성이므로 사용자 결정이 필요합니다. 31개 언어와 플러그인 언어, 테마 token 색 매핑을 포함합니다.
5. **찾기/바꾸기와 다중 커서(L, 2·3번 이후)** — 28, 24번.
6. **LSP 상호작용(XL, 2·3번 이후)** — `LspBridge` 에 범용 기능 요청 경로를 추가한 뒤 완성 팝업(41) → 스니펫 연결(79) → hover(42) → signature help(43) → 정의·참조(44·45·58) → rename(46) → code action(47) → 진단 밑줄(40) → inlay·semantic token·document highlight·code lens(50·7·51·53) 순서입니다.
7. **Git 장식과 diff(XL, 3번 이후)** — gutter·blame·conflict(73~75) 다음 diff 편집기(76). diff 는 두 표면의 스크롤 동기화와 줄 사이 블록이 필요합니다.
8. **AI(L, 3·6번 이후)** — ghost text(77), 인라인 편집 위젯(78).
9. **나머지(각 S~L)** — 글꼴 패밀리(2), 뷰 상태 영속(67), IDE 선택 동기화(83), refused 화면(59), 터미널 실행(82), breadcrumbs(71, 54번 이후), Markdown 미리보기(72), search-editor(81), Emmet(80), 내장 언어 서비스 대체 방안(57).

## 9. 확인하지 못한 것

- 빌드·테스트·실행을 하지 않았으므로 done 판정은 "코드가 존재하고 호출 체인으로 도달한다"는 의미입니다. 실제 화면 동작과 시각 일치는 검증하지 않았습니다.
- `use-editor-file-persistence.ts`(718줄), `lsp-session-registry.ts`(937줄), LSP adapter 개별 파일은 전체를 읽지 않고 등록 API 와 핵심 분기만 확인했습니다. 저장 경합·crash 복구의 세부 동등성은 판정 대상에서 제외했습니다.
- `keymap-defaults.json` 의 `find` 같은 미지원 액션 키가 편집기에서 삼켜지는지 통과되는지는 `keymaps.route` 내부(`terminal_surface.rs:3107` 이후)를 끝까지 따라가지 않아 미확인입니다.
- egui 기본 monospace 글꼴의 한글·CJK glyph 범위와 실제 표시 품질은 확인하지 않았습니다.
- `native/taide-remote-web` 은 같은 `NativeEditor` 를 쓰는 것만 확인했고(`browser-editor.rs:1106`) 브라우저 쪽 추가 기능은 깊게 보지 않았습니다.
- effort 는 코드 규모와 의존 관계로 추정한 값이며 3번(표시 계층) 설계 방식에 따라 크게 달라집니다.
- outline·problems 패널, 탭 바, 설정의 스니펫·테마 편집기, 미리보기 표면은 다른 영역 감사 대상이라 편집기와 맞닿는 부분만 적었습니다.
