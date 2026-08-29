# 기능 — 코드 에디터 (codeview)

> FR-D. Monaco 기반 에디터, git gutter, 인라인 blame, 키바인딩, 모델/상태 관리.
> API 확정 근거: `docs/research/monaco.md` (0.56 기준 — 경로·API 가 구버전 자료와 다름. 반드시 이 문서 기준).
> LSP 연결은 `lsp.md`, diff 뷰는 `git.md` §4.

## 1. 기본 구성

- `monaco-editor@0.56.0`. **`monaco-editor/esm/vs/...` 경로 금지** — 0.56 exports 맵 기준
  `monaco-editor/editor/editor.worker.js` 형태만 사용(research §1, 함정 1).
- worker 배선: 플러그인 없이 `?worker` 수동 배선 + `MonacoEnvironment.getWorker`.
  `vite.config.ts` 에 `worker: { format: 'es' }`, `optimizeDeps: { exclude: ['monaco-editor'] }`.
- **에디터 인스턴스는 pane(Leaf)당 1개, 탭 전환은 model 교체**(research §8 정석).
  탭마다 인스턴스 생성 금지.
- React 래퍼: `features/editor/code-editor.tsx` 는 컨테이너 div 마운트/`create`/`dispose` 만 하는
  순수 컴포넌트. 탭·git·LSP 배선은 `widgets/editor-pane` 담당.
- CSP: `style-src 'unsafe-inline'`, `worker-src 'self' blob:` 허용 필요(research 함정 12).
  초기 개발은 CSP 미설정으로 검증 후 조인다.

## 2. 모델 레지스트리 (누수 방지의 핵심)

- `entities/editor/model-registry.ts`: `URI(monaco.Uri.file(absPath)) → { model, viewState }` 단일 레지스트리.
  - 생성 전 `monaco.editor.getModel(uri)` 확인(중복 createModel 은 예외 — research 함정 6).
  - 탭 닫기 = `model.dispose()` (편집 중이 아니면). **열린 탭 외 모델 캐시는 두지 않는다**(단순·확실).
    실제 배선은 d-50 S8 에서 들어왔다(감사 §1-6): `useCloseTab` 이 어느 창·페인에도 그 파일 탭이
    남지 않았을 때만 `disposeModel(path)` 한다. 다시 열면 새 모델이라 **undo 스택은 초기화**된다.
  - 개명 추종(d-50 S8): `retargetModel(from, to)` 가 버퍼·언어·뷰 상태를 새 uri 의 모델로 옮기고,
    그 모델을 띄우던 에디터를 **폐기 전에** 새 모델로 갈아 끼운 뒤 옛 모델을 dispose 한다
    (uri 는 불변이라 in-place 이동이 불가능 — 그래서 여기서도 undo 스택은 초기화된다).
    확장자가 바뀐 개명은 새 경로 재조회 결과의 `languageId` 를 `applyModelLanguage` 로 반영한다.
  - DiffEditor 의 original 모델(`taide://git/...` 스킴)은 diff 탭 닫을 때 함께 dispose.
- viewState: 탭 전환 시 `saveViewState()` → 레지스트리에 보관, 복귀 시 `restoreViewState()` + `editor.focus()`
  (restore 는 포커스를 주지 않음 — 함정 7). viewState 는 직렬화 가능 → Rust layout 도메인에 미러해
  재시작 복원에 사용(`data-model.md` layout.json).

## 3. 파일 열기/저장 흐름

- 열기: Rust `file_open(path)` → 내용 + 감지 언어 id + 파일 크기. 대형 파일 판정은 Rust 가 선행:
  **2MB 또는 50,000줄 초과 시 "대형 파일 모드"** — research §9 의 축소 옵션 프리셋 적용
  (minimap·folding·bracketColorization off, `largeFileOptimizations: true` 유지) + LSP didOpen 미전송.
  20MB 초과는 열람 전용(편집 비활성) + 안내 배너. **50MB 초과(`REFUSED_FILE_BYTES`)는 내용을
  읽지 않고 refused 상태를 반환**하며, 바이너리 판정(`is_binary`)도 크기와 무관하게 즉시
  refused 다(`domain/file/service.rs`). **UTF-8 이 아닌 바이트가 섞인 파일**(EUC-KR 등)은 손실
  디코딩(U+FFFD)이 감지되어 크기와 무관하게 열람 전용 + `encodingLossy` 표식으로 열린다 — 되쓰면
  원본이 파손되므로 저장 경로가 막힌다(d-50 S2, `ipc-contract.md` file 절). **디스크에 쓰는 경로 3곳이
  모두 `readOnly` 를 거른다**: `handleSave`(⌘S·자동 저장), LSP `WorkspaceEdit` 의 파일 경로
  (`shared/lib/lsp/workspace-edit-applier.ts`), Claude Code 저장 요청(`ide:save-requested`). 미러 복원·
  백그라운드 모델 편집으로 read-only 탭이 dirty 가 될 수 있어 셋 다 필요하다. ⌘S 는 무음으로 무시하지
  않고 `editor.readOnlySaveBlocked` 토스트로 알린다(자동 저장은 조용히 건너뛴다).
- 저장: `⌘S` → 모델 내용 `file_save(path, content)`. 저장 완료 시 dirty 해제·버퍼 미러 삭제.
  포맷-온-세이브는 설정 옵션(LSP formatting 연동, 기본 off — 1차). 포맷 뒤에 붙는 on-save 정리
  (후행 공백 제거·마지막 줄바꿈)는 §16.
- **저장 정착은 pane 이 아니라 경로 단위**다(d-51 F1). 디스크 쓰기가 성공하면 `useSaveFile` 이
  `entities/editor/file-save-settle-registry.ts` 로 `(path, content)` 를 방송하고, 그 경로를 보는
  모든 `EditorPane` 이 dirty·`syncedContent`·hot-exit 미러(타이머·캐시 엔트리)를 함께 정착시킨다.
  ⌘S·자동 저장·`ide:save-requested`(Claude Code 외부 저장)·충돌 해소가 모두 이 경로를 탄다. 저장
  왕복 중 계속 타이핑한 pane 은 draft 가 디스크에 쓰인 내용과 달라 정착에서 제외되고 dirty 를
  유지한다. 이 방송이 없으면 스플릿의 반대쪽 pane 이 dirty 잔존 + 가짜 `changedOnDisk` 배너 +
  미러 부활을 일으켰다.
- **hot-exit 미러(파일 탭)**: 모델 변경 debounce `HOT_EXIT_MIRROR_DEBOUNCE_MS`(500ms,
  `shared/constants/mirror.ts`) → `file_mirror_dirty(projectId, path, content, diskModifiedMs)`
  (`diskModifiedMs` = 열람 시점 `file.modifiedMs` baseline, ADR-0004 예외 경로, `data-model.md` §6).
  window `blur`·탭 언마운트 시 즉시 flush(`entities/editor/mirror-flush-registry.ts` 에 탭당 flush
  콜백 등록). 프로젝트 활성화 시 `file_list_mirrors(projectId)` 1회 조회 → path→entry 캐시
  (TanStack Query `staleTime: Infinity`, 매 `file_mirror_dirty` 성공마다 `setQueryData` 로 직접
  갱신 — 재조회 없이 캐시를 최신 상태로 유지). 탭 활성화 시 미러가 있으면
  `applyExternalContent` + `setDirty(true)` 로 lazy 복원. 이 복원은 같은 커밋에 예약된
  "디스크 내용 sync" microtask 와 경쟁하므로, sync 가드는 렌더 스냅샷이 아니라 **라이브 dirty
  신호**(`dirtyRef`)를 읽는다(`widgets/editor-pane/editor-draft-sync.ts` 의 `syncModelFromDisk`).
  또한 `applyExternalContent` 의 `setValue` 는 자기 변경 이벤트를 억제하므로(model-registry 의
  `isApplyingExternalContentTo`) 프로그램적 콘텐츠 적용이 키 입력으로 오인돼 dirty 를 세우거나
  미러를 덮어쓰지 않는다. Rust 가 `disk_modified_ms` 대비 현재
  디스크 mtime 으로 계산한 `conflict` 가 true 면 `ConflictBanner` 의 `mirrorRestoredConflict`
  variant(위험, 선택 강제), false 면 `mirrorRestored` variant(경미, 닫기만 가능)를 띄운다.
  "디스크 보기" 선택 시 `file_clear_mirror`. 탭 닫기 시 미러 삭제, 프로젝트 열기 시
  `file_prune_mirrors(projectId, keepPaths)` 로 열린 탭 외 미러 GC.
- **hot-exit 미러(untitled 탭)**: untitled 탭은 더 이상 휘발성이 아니다(레이아웃에 영속) —
  탭ID 를 키로 `file_mirror_untitled(projectId, tabId, content)` / `file_list_untitled_mirrors` /
  `file_clear_untitled_mirror` / `file_prune_untitled_mirrors(projectId, keepTabIds)`. 파일 탭과
  동일한 debounce·blur/언마운트 flush·`setQueryData` 캐시 동기화를 따르되 경로가 없으므로
  conflict 판정은 없다(항상 `mirrorRestored`류 복원 없이 조용히 적용).
- **종료 시 0-손실 플러시**: 창 X 버튼(`WindowEvent::CloseRequested`)을 Rust 가 가로채
  `prevent_close()` 후 `HotExitFlushRequested` 이벤트를 emit → 프론트(`HotExitFlushProvider`)가
  마운트된 모든 편집기 pane 의 flush 콜백을 `Promise.all` 로 실행·대기(각 콜백은 실제
  `file_mirror_dirty`/`file_mirror_untitled` IPC 완료까지 await)한 뒤 `file_flush_complete` 호출 →
  Rust 가 `app.exit(0)`. 프론트 무응답 시 `HOT_EXIT_FLUSH_TIMEOUT_MS`(2.5s, `constants.rs`) 타임아웃
  폴백이 강제 종료한다. 앱 메뉴 Quit(⌘Q)도 `NSApplication terminate:` 로 직행하는 대신 커스텀
  메뉴 아이템이 메인 윈도우의 `close()` 를 호출해 같은 경로를 탄다. 이 핸드셰이크는
  데스크톱 로컬 전용이며 원격 클라이언트에는 전파되지 않는다(원격 세션은 종료를 제어할 수 없음).
- 외부 변경: watcher 이벤트로 열린 파일이 바뀌면 — dirty 아니면 조용히 리로드(viewState 유지),
  dirty 면 충돌 배너(디스크 내용 보기 / 덮어쓰기 / 유지, `changedOnDisk` variant).

## 4. Git gutter (FR-D2)

- 데이터: git 도메인 `git_gutter(path)` — hunk 배열 `{ kind: added|modified|deleted, start, end }`
  (`diff_tree_to_workdir_with_index`, `context_lines(0)` — `docs/research/git2.md` §3).
- 렌더: `createDecorationsCollection()` (deltaDecorations 는 deprecated — 함정 4),
  `linesDecorationsClassName` + CSS(추가/수정 = 3px 바, 삭제 = 라인 경계 삼각형, VSCode 규칙),
  `overviewRuler` 동시 표시. 색은 `editorGutter.*` 테마 토큰.
- 갱신: 파일 저장·git status 변경 이벤트 시 재요청. 편집 중(미저장)엔 마지막 저장 기준 유지(1차.
  버퍼 기준 실시간 diff 는 2차).
- gutter 클릭(2차): `onMouseDown` + `GUTTER_LINE_DECORATIONS` 타겟 → inline change peek(view zone).

## 5. 인라인 blame (FR-D3)

- 표기 형식: GitLens 기본 포맷을 채택 — `${author, }${agoOrDate}${ • message|50?}` 렌더 결과
  `HS, 4 days ago • fix: ...`. 본인 커밋은 author 를 `You` 로 치환. 미커밋 라인은
  `You, now - Uncommitted changes` (research vscode-behaviors §7).
  - 작은 토큰 파서: `?`(optional)·`|50`(truncate)·패딩 3종만 지원(동 문서 적용 가이드 6).
- 렌더(실구현은 두 갈래 — `use-editor-blame.ts`):
  - **커서 라인 blame = 하단 footer bar** — `blame-footer-bar.tsx` 의 textContent 를 직접 갱신
    (injected text 아님 — 리렌더 없이 텍스트만 교체).
  - **파일 전체 blame 오버레이(토글)** — `TOGGLE_BLAME_MONACO_ACTION_ID` 로 켜면
    `createDecorationsCollection` + injected text(`after`) 로 각 라인 끝에 렌더. `content` 는
    반드시 한 줄(개행 제거 — 함정 8). `editorBlame.*` 토큰.
- 데이터: `onDidChangeCursorPosition` debounce → `git_blame_range(path, from, to)`.
  Rust `blame_range` 는 매 호출 `repo.blame_file()` 직접 실행(캐시 없음)이며 **디스크 저장분
  기준**이다 — 미저장 버퍼 편집은 반영되지 않는다(초안의 `(path, HEAD oid)` 캐시·`blame_buffer`
  는 도입하지 않았다).
- hover(2차): blame 텍스트 위 hover 시 커밋 상세(메시지 전문·author·SHA·날짜).

## 6. 키바인딩 (FR-D4)

3계층 규칙 (research monaco §7 확정):

| 계층 | 대상 | 구현 |
|------|------|------|
| Monaco 내장 | 멀티커서·라인 조작·주석·찾기/바꾸기·go to def/refs·rename·포맷·quick fix 등 에디터 스코프 전부 | 그대로 사용 (VSCode 동일 키) |
| Monaco 보강 | 명령 팔레트 `⌘⇧P`(기본은 F1 뿐), `⌘S` 저장 등 | `addKeybindingRules` / `addAction` |
| 앱 전역 | `⌘P` 파일 퀵오픈, `⌘W` 탭 닫기, `⌘B` 사이드바, `⌘⇧F` 검색, `⌃Tab` 탭 전환, `⌘\` 분할, `⌘⇧E/G` 뷰 전환, `⌃\`` 터미널 | window keydown 캡처 단계 핸들러(단일 keymap 모듈) + 필요 시 Tauri 메뉴 accelerator. 에디터 내 충돌 키는 `{ keybinding, command: null }` 로 monaco 기본 해제 |

- 전체 키 목록 정본: `docs/research/vscode-behaviors.md` §8 표. 앱 전역 키맵은
  `shared/lib/keymap/keymap.ts` 한 곳에서 선언한다(설정 오버라이드 대비 데이터 구조로).
- WebView 기본 동작(`⌘P` 인쇄 등) preventDefault 필수(vscode-behaviors 함정 절).

## 7. 테마 연동

- `shared/lib/monaco/theme.ts`: 테마 토큰(`docs/theme-system.md` §4.2) → `defineTheme('taide', ...)` 변환.
  - **colors 값은 hex 만 허용**(CSS 변수 불가 — 잘못되면 조용히 빨강. research 함정 3).
    테마 JSON 이 hex 원본을 갖고 있으므로 CSS 변수 경유 없이 직접 변환한다.
  - 테마 전환 = 같은 이름 defineTheme 재호출 + `setTheme`.
- semantic highlighting: `'semanticHighlighting.enabled': true`(상시, create 옵션 — §8 참조) + LSP
  semantic tokens.

## 8. Semantic Tokens (Wave F — `docs/acknowledge/2026-08-15-wave-f-editor-presentation-contract.md`)

- **capability 선언**(`shared/lib/lsp/initialize-params.ts` `buildInitializeParams`):
  `textDocument.semanticTokens { requests: { full: { delta: true } }, tokenTypes: 표준 23종,
  tokenModifiers: 표준 10종, formats: ['relative'], augmentsSyntaxTokens: true }` +
  `workspace.semanticTokens.refreshSupport: true`. range 는 1차 제외(뷰포트 스코프 요청 미요청).
  상수(`SEMANTIC_TOKEN_TYPES`/`SEMANTIC_TOKEN_MODIFIERS`)는 `shared/lib/lsp/protocol.ts` 에
  `as const` + 유도로 정의.
- **어댑터(`shared/lib/lsp/adapters/semantic-tokens.ts`) — 재인코딩 전략**: monaco standalone 은
  `semanticHighlighting.enabled: true` 를 켜도 서버 legend 문자열을 직접 이해하지 못하고, 매칭
  실패 토큰은 기본 전경색(shiki 색 위)으로 **전면 덮어써버린다**(워시아웃) — 따라서 어댑터가 서버
  legend 를 `semantic-token-map.ts` 의 `SEMANTIC_TOKEN_TYPE_MAP` 으로 TAIDE 토큰명(`SYNTAX_TOKENS`
  31종)에 재매핑하고, **자체 monaco legend**(매핑된 토큰명 집합만)를 노출한다. 매핑되지 않는
  서버 토큰 타입은 스트림에서 **드롭**(shiki 색 보존) — modifier 는 표준 10종 이름으로 통과.
  legend 의 각 타입명은 `toSemanticTokenLegendScope`(`semantic-token-map.ts`)로 `taideSemantic.<token>`
  네임스페이스를 씌워 노출한다 — bare `SYNTAX_TOKENS` 이름(예: `'variable'`)은 다수 번들 테마의
  실제 TextMate scope 이기도 해서, 이 이름 그대로 semantic rule 을 추가하면 monaco 의 토큰 테마
  트라이(동일 token 문자열은 나중 index 가 이긴다)가 테마 자신의 rule 을 덮어써 **일반 구문
  강조 색까지 바뀌는 회귀**가 있었다(2026-08-15 검토에서 발견·수정). 네임스페이스는 어떤
  TextMate 문법도 절대 내지 않는 scope 라 이 충돌이 원천적으로 성립하지 않는다.
- **delta 처리**: 모델별 캐시(서버 `resultId` + 서버 원본 `data`)를 보관한다.
  `textDocument/semanticTokens/full/delta` 가 `SemanticTokensDelta`(splice edits)로 응답하면
  캐시된 원본 데이터에 적용한 뒤 **전체를 재인코딩**해 monaco 에는 항상 full `SemanticTokens` 로
  반환한다(재인코딩은 드롭/재배치 때문에 부분 적용이 불가 — delta 의 이득은 서버 재계산·와이어
  절감으로만 실현된다). `applySemanticTokensDeltaEdits` 는 `edits` 를 **역순으로** 적용한다 — 각
  `edit.start` 는 원본(이전 결과) 배열의 인덱스를 가리키므로(monaco 의 참조 구현
  `documentSemanticTokens.js` 와 동일한 계약), 정순으로 순차 적용하면 앞선 edit 이 배열 길이를
  바꿔 이후 edit 의 `start` 가 엉뚱한 위치를 가리키게 된다. `releaseDocumentSemanticTokens`/모델
  dispose 시 캐시 해제.
- **refresh**: `workspace/semanticTokens/refresh` 는 **세션 스코프**로
  `client.registerRequestHandler`(`workspace/applyEdit` 와 동일 패턴, `lsp-session-registry.ts`
  `createSession`)에 등록 — 그 세션의 어댑터 인스턴스만 `onDidChange` 를 발화한다(전 세션 폭풍
  방지). `registerSemanticTokens` 가 내부적으로 관리하는 리스너 집합에
  `triggerSemanticTokensRefresh(client)` 가 연결한다.
- **활성화**: `code-editor.tsx` 의 `monaco.editor.create` 옵션에 `'semanticHighlighting.enabled':
  true` 상시 고정. 설정 `editorSemanticHighlighting`(기본 **true**)은 어댑터의 **요청 시점 게터
  게이트**(`isCodeLensEnabled` 선례)로 구현 — off 면 빈 결과 반환, 재등록 없음. 토글 시점에는
  `use-lsp-session.ts`의 `attachLspSession`이 `QUERY_KEY.SETTINGS.CURRENT` 쿼리 캐시 변경을
  구독해 값이 실제로 바뀐 순간에만 `triggerSemanticTokensRefresh(client)`를 호출 — 그 세션이
  등록한 어댑터의 `onDidChange`가 즉시 발화해 다음 편집을 기다리지 않고 재계산된다. large-file
  tier 는 LSP 미부착이라 자연 비활성.
- **테마 매핑**(`shared/lib/shiki/build-shiki-theme.ts`): `buildShikiTheme` 의 `tokenColors`
  배열 **끝에** `SEMANTIC_TOKEN_TYPE_MAP` 매핑 대상 토큰명을 **`taideSemantic.` 네임스페이스
  scope**(bare 토큰명이 아님)로 하는 rule 을 기존 syntax 색으로 append — 신규 색 도입 없음.
  네임스페이스를 씌우는 이유는 위 어댑터 설명대로 실제 TextMate scope 와의 충돌(및 그로 인한
  일반 구문 색 변경)을 원천 차단하기 위함이다. 폴백 테마(`shared/lib/monaco/theme.ts`)는 이미
  토큰명 rule 이라 무변경.
- **서버별 지원**:

  | 서버 | full | delta | range | 비고 |
  |------|:----:|:-----:|:-----:|------|
  | rust-analyzer | O | O | O(서버 지원, 클라이언트가 요청하지 않음 — 1차 제외) | legend 비표준 ~30종, `SEMANTIC_TOKEN_TYPE_MAP` 으로 재매핑 |
  | gopls | `initializationOptions: { "semanticTokens": true }` 로 활성화(기본 false) | - | - | `resources/lsp-servers.json` 매니페스트 값 — 코드 변경 없이 명세로 활성화(Wave A 의 `initialization_options` 필드 재사용) |
  | vtsls | O | X(delta 미지원) | O(클라이언트 미요청) | TAIDE 의 TS/JS 서버 |

  gopls 는 `ConfigurationSupported` 면 provider 를 광고하되 설정이 꺼져 있으면 요청마다 빈 결과를
  반환할 수 있다("조용한 0토큰") — `initializationOptions` 는 gopls 소스(`options.Set`)가 정식
  설정 경로로 확정 반영하는 것으로 확인했으나, TAIDE 의 `workspace/configuration` null 응답이
  이 값을 되돌리지 않는지는 실기 확인 대상(검토 웨이브).

## 9. Format on Type / Paste (Wave F)

- 어댑터 2종 신설(`shared/lib/lsp/adapters/formatting.ts`, `registerFormatting` 의 형제 함수):
  `registerRangeFormatting`(`textDocument/rangeFormatting`) · `registerOnTypeFormatting`
  (`textDocument/onTypeFormatting`). 셋 다 `toMonacoTextEdits` 변환·`CancellationToken` 처리를
  공유한다.
- **formatOnPaste 는 monaco 내부적으로 `documentRangeFormattingEditProvider` 등록이 필수
  게이트**다(문서 전체 포매팅으로의 합성 폴백이 없다) — `registerRangeFormatting` 이 곧 그 전제를
  충족시킨다.
- **formatOnType** 트리거 문자는 서버가 선언한 그대로 사용한다:
  `autoFormatTriggerCharacters = [capability.firstTriggerCharacter,
  ...(capability.moreTriggerCharacter ?? [])]` — 클라이언트가 임의로 화이트리스트를 좁히지 않는다.
- 설정: `editorFormatOnType`·`editorFormatOnPaste`(둘 다 기본 **false**, VS Code 동일) —
  `code-editor.tsx` 의 기존 "묶음" `updateOptions` effect 로 배선. `CodeEditorProps` 의 다른 boolean
  들과 마찬가지로 **필수 prop**(옵션 기본값 없음) — untitled 탭(`untitled-pane.tsx`)은 LSP 세션이
  붙지 않으므로 항상 `false` 를 명시 전달한다.
- **서버별 지원**:

  | 서버 | rangeFormatting | onTypeFormatting | 트리거 문자 |
  |------|:---:|:---:|------|
  | rust-analyzer | O | O | `.` `=` `<` `>` `{` `(` `\|` `+` |
  | vtsls | O(추정) | O(추정) | `;` `}` `\n`(VS Code TS 확장 래핑 구조 근거 — 서버가 실제 선언한 값을 그대로 사용하므로 추정이 틀려도 동작에는 영향 없음) |
  | gopls | X | X | 둘 다 미선언 — gopls 는 포매팅을 `gofmt`/`goimports` 외부 도구에 위임하는 설계 |

## 10. 사용자 스니펫 (Wave F)

- **저장소(Rust, `domain/snippet`)**: `paths.rs` 의 `snippets_dir()`(테마 디렉토리 선례) 아래
  `<languageId>.json`(언어 전용) / `*.code-snippets`(전역, `scope` 필드로 언어 필터) 파일을 VS
  Code 호환 포맷으로 저장한다. 3커맨드 — `snippet_list`(스캔, 파싱 실패 파일은 스킵 +
  `log::warn!`) · `snippet_save(fileName, content)`(파일명 `/`·`\`·`..` 거부 + 확장자
  화이트리스트 `.json`/`.code-snippets`, 리스트 스캔과 동일 predicate 공유 — "저장했지만 목록에
  안 보이는 파일" 방지 + `content` JSON 스키마 검증 후 원문 그대로 저장) ·
  `snippet_delete(fileName)`. 원격 dispatch 는 `theme_save` 계열과 동일하게 허용.
- **파일 포맷**: 키 = 스니펫 이름, 값 = `{ prefix: string | string[], body: string | string[],
  description?: string | string[], scope?: string }`(`scope` 는 `.code-snippets` 전용, 콤마
  구분 언어 목록 — 비어 있으면 전체 언어). `isFileTemplate`·`include`/`exclude` 는 1차 무시.
  `prefix`·`description`의 `string[]` 은 의미가 다르다 — `prefix` 배열은 복수 트리거(콤마로
  구분해 편집), `description` 배열은 VS Code 와 동일하게 여러 줄(개행으로 합쳐 편집)이다.
  편집 UI(`snippet-entry-editor.tsx`)는 description 을 단일 행 입력으로만 받으므로 저장 시
  콤마로 분해하지 않고 입력한 문자열 그대로 쓴다(2026-08-15 검토에서 콤마 분해 회귀 수정).
- **프론트 completion**(`shared/lib/snippet-completion.ts` `registerSnippetCompletions`):
  `TAIDE_LANGUAGE_IDS` **각각에 개별** `registerCompletionItemProvider` 를 등록한다 — `'*'`
  단일 selector 는 monaco 가 낮은 우선순위 그룹으로 밀어 넣어 LSP 가 붙은 언어에서 영구
  미노출되므로 금지. `kind: Snippet`·`insertTextRules: InsertAsSnippet`·`body.join('\n')`.
  `shared` 는 `entities` 를 import 할 수 없으므로(fsd.md §2) 스니펫 목록은 주입 게터
  (`getSnippetFiles: () => readonly SnippetFile[]`)로 받는다(`workspace-edit-applier.ts` 의
  로컬 래퍼 선례와 동일한 이유).
- **캐시·부트스트랩**(`app/bootstrap-snippets.ts`): 앱 시작 시 `QUERY_KEY.SNIPPET.LIST` 에
  대해 앱 수명 동안 유지되는 `QueryObserver` 를 구독한 뒤 `registerSnippetCompletions` 를 1회
  등록한다 — 설정 화면을 한 번도 열지 않은 세션에서도 첫 입력부터 스니펫이 보이게 하기 위함.
  옵저버 없는 `prefetchQuery` 만으로는 전역 `gcTime`(10분) 경과 후 옵저버 0개인 캐시가 GC 되어
  이후 completion 이 영구히 빈 목록이 되는 회귀가 있었다(2026-08-15 검토에서 발견·수정) —
  옵저버를 유지하면 GC 되지 않을 뿐 아니라, `entities/snippet`(`snippet.query.ts`)의 저장/삭제
  mutation 이 성공 시 호출하는 `QUERY_KEY.SNIPPET.ALL` 의 `invalidateQueries`(기본
  `refetchType: 'active'`)도 실제 재조회로 이어진다. provider 의 게터는 매 completion 요청마다
  캐시를 다시 읽으므로 재등록 없이 곧바로 반영된다.
- **플러그인 언어**(d-51 F6 · 감사 §4-B D6): provider 가 언어 id 별이라 부팅 시 고정 목록만 등록하면
  플러그인이 기여한 언어에는 스니펫이 전혀 뜨지 않았다. `registerPluginLanguages` 가 새 언어 id 를
  `registerSnippetCompletionsForLanguages` 로 넘겨 같은 provider 를 붙인다(`plugins.md` §6.4).
  단 새 스니펫 **파일 생성 다이얼로그**의 언어 목록은 여전히 `TAIDE_LANGUAGE_IDS` 고정이다.
- **편집 UI**: 설정 SNIPPETS 섹션에 "Manage Snippets"(→ `widgets/snippet-editor`, ThemeEditor
  형 — 파일 선택·prefix/body/description/scope 폼·저장/삭제 `AlertDialog`) ·
  "Open Snippets Folder"(`systemOpenAppDataPath('snippets')`) 버튼.
- **저장·이탈 규약**(d-51 F6 · 감사 §4-B D6): 이름·prefix·body 중 하나라도 빈 항목은
  `draftsToSnippetContent` 가 **버린다.** 그래서 일부만 채운 행이 있으면 저장을 진행하지 않고
  `snippetEditor.incompleteEntryError` 로 막는다(완전히 빈 행은 잃을 것이 없으므로 그대로 버린다).
  저장하지 않은 편집을 들고 다른 파일을 고르거나 설정으로 돌아가면 확인 다이얼로그를 띄운다
  (`hasUnsavedSnippetDraftChanges` — 직렬화되지 않는 미완성 행도 "변경" 으로 센다).
- 1차 보류: 스니펫 파일 워처(외부 편집 즉시 반영 없음, 명시 저장 경로의 invalidate 만),
  gist 동기화(sync 는 `settings.json` 한정).

## 11. Emmet (Wave F)

- `emmet-monaco-es@5.7.0`(신규 승인 — `docs/tech-stack.md`, `THIRD_PARTY_LICENSES.md`) 로 HTML/
  CSS/JSX 계열에 Emmet 약어 확장을 등록한다: `emmetHTML(['html', 'heex'])` ·
  `emmetCSS(['css', 'scss'])` · `emmetJSX(['javascriptreact', 'typescriptreact'])`
  (`shared/lib/emmet-integration.ts`). `heex` 는 shiki 문법이 `html` 그래머를 재사용하므로 HTML
  계열에 포함.
- **`{ tokenizer: 'standard' }` 필수** — TAIDE 는 토큰화를 `@shikijs/monaco`(shiki) 로만 하고
  monaco 내장 Monarch 토크나이저를 쓰지 않는데, 라이브러리 기본값(`'monarch'`)은 Monarch 내부
  상태를 리플렉션해 읽으므로 shiki 모델에서는 조용히 동작하지 않는다. `'standard'` 는
  `model.tokenization.getLineTokens`/`getStandardTokenType` 비공개 API 를 사용한다(계약 문서
  기록 — `monaco-command-service-deep-import.md` 선례 범주).
- 설정 `emmetEnabled`(기본 **true**) 토글은 `app/providers/emmet-provider.tsx` 가
  `useEffect`(외부 시스템 동기화)로 켜질 때 `enableEmmet(monaco)` 를 호출하고 꺼질 때 그 반환
  dispose 로 세 언어군의 등록을 모두 해제한다.
- Tab 키를 직접 가로채 확장하는 방식은 채택하지 않았다(Tab 이 들여쓰기·suggest 수락·스니펫
  탭스톱 이동과 3중으로 경합) — Emmet 확장 후보는 completion 항목으로 노출되어 다른 completion
  과 동일하게 Tab/Enter 로 선택한다.
- **알려진 한계**: `emmet-monaco-es` 내부의 `triggerCharacters` 조회 테이블(`LANGUAGE_MODES`)은
  `html`/`css`/`scss`/`javascript`/`typescript` 키만 갖고 있고, TAIDE 가 등록하는
  `javascriptreact`/`typescriptreact`/`heex` 에 대한 별칭이 없다(라이브러리 소스 확인,
  2026-08-15 검토). 그 결과 이 세 언어의 Emmet completion provider 는 `triggerCharacters` 없이
  등록되어, `}`/`>`/`*`/`$`/`]`/`/`처럼 단어 문자가 아닌 문자로 끝나는 약어는 자동으로 제안
  목록이 뜨지 않는다(수동 `⌃Space`는 정상 동작 — quickSuggestions 로 문자 입력 중에는 계속
  뜬다). 이 조회 테이블·`doComplete`/`isValidLocationForEmmetAbbreviation` 는 패키지가 공개
  export 하지 않아, 고치려면 (a) 업스트림에 별칭 추가를 요청하거나 (b) TAIDE 가 패키지 내부
  경로로 딥임포트해 자체 트리거를 얹는 새 아키텍처가 필요하다 — 둘 다 이번 Emmet 도입 계약
  (§3.4, `{tokenizer:'standard'}` + monaco 비공개 API 사용만 승인)의 범위 밖이라 별도 계약
  검토 없이는 보류한다.

## 12. 내장 TS/JS 지원 (LSP 이전 베이스라인)

- 1차 기동 시 TS/JS 는 내장 ts worker(TS 5.9.3)로 시작: `setEagerModelSync(true)`,
  Rust 가 프로젝트 `tsconfig.json` 을 읽어 `setCompilerOptions` 주입(paths·jsx 등).
- 한계(연구 확정): node_modules 타입 해석·프로젝트 전역 참조 불가 → LSP 전환 시
  `setModeConfiguration` 으로 내장 기능을 끄고 일원화(`lsp.md` 참조).

## 13. 수명주기 · 누수 방지

- pane unmount: `editor.dispose()` (model 은 레지스트리 소유 — dispose 하지 않음).
- 탭 닫기: viewState 저장 → model dispose (§2).
- decoration collection 은 에디터 dispose 로 함께 정리되지만, 파일 전환 시 `clear()` 명시 호출.
- blame·gutter 요청은 최신 요청만 유효(경쟁 응답 무시 — 요청 시퀀스 토큰).
- semantic tokens 어댑터는 세션 dispose 시 캐시(`cacheByModelUri`)를 전부 비우고 refresh 리스너
  구독을 해제한다(§8). Emmet 은 `emmetEnabled` 토글·`EmmetProvider` 언마운트 시 세 언어군의
  monaco 등록을 일괄 해제한다(§11).

## 14. 범위 (1차/2차)

| 1차 | 2차 |
|-----|-----|
| Monaco 마운트·모델 레지스트리·viewState 복원·저장/dirty·대형 파일 모드·git gutter·인라인 blame·키바인딩 3계층·테마 연동·내장 TS·**semantic tokens(full+delta)**·**format on type/paste**·**사용자 스니펫(CRUD+completion+편집 UI)**·**Emmet(HTML/CSS/JSX)** | gutter 클릭 peek, blame hover 상세, 실시간(버퍼) gutter diff, 포맷-온-세이브 기본화, 파일 퀵오픈(⌘P) 고도화(심볼 검색), minimap git 표시, semantic tokens range(뷰포트) provider, 스니펫 파일 워처·gist 동기화·`isFileTemplate`/`include`·`exclude`, gopls 서버별 설정 맵(`workspace/configuration`) |

## 15. 표시 옵션 5건 (d-53 U1 — `docs/acknowledge/2026-08-29-d53-ux-batch-contract.md` §1 U1)

감사 §5 "저비용 즉효" ①~⑤. 전부 **VS Code 파리티로 기본 off/빈 값**이고, 설정 화면 Editor 섹션에
토글·입력으로 노출된다. 설정 → monaco 옵션 매핑은 다른 에디터 설정과 같은 경로
(`shared/lib/code-editor-settings.ts` 의 `resolveCodeEditorSettingsProps` → `CodeEditorProps` →
`code-editor.tsx` 의 묶음 `updateOptions` effect)를 그대로 확장했다.

| 설정 | monaco 옵션 | 비고 |
|------|-------------|------|
| `editorBracketPairGuides` | `guides: { bracketPairs }` | `editorBracketPairColorization`(`bracketPairColorization.enabled`)과 **별개 옵션**이다 — 색은 그대로 두고 가이드선만 켠다 |
| `editorSmoothScrolling` | `smoothScrolling` | |
| `editorCursorSmoothCaretAnimation` | `cursorSmoothCaretAnimation` | monaco 는 `'off'｜'explicit'｜'on'` 3값이지만 설정은 **토글**로 좁혀 `'on'`/`'off'` 만 쓴다(중간값 하나를 위해 3지 선택 UI 를 두지 않는다) |
| `editorSuggestPreview` | `suggest: { preview }` | 자동완성 인라인 고스트 미리보기 |
| `editorRulers` | `rulers` | 콤마 구분 열 번호 입력 — 아래 참조 |

- **`guides`·`suggest` 는 객체 옵션**이라 `updateOptions` 에 부분 객체를 넘기면 monaco 가 그 객체를
  **하위 필드 단위로 재귀 병합**한다(`EditorOptionsUtil.applyUpdate` → 각 옵션의 `applyUpdate`).
  즉 이전에 설정해 둔 하위 필드는 보존되고, **한 번도 설정한 적 없는** 하위 필드만 monaco 기본값으로
  검증된다(`guides.indentation` 등). TAIDE 는 그 하위 필드를 다른 곳에서 건드리지 않으므로 부분
  전달이 안전하다 — 그래도 같은 객체의 다른 하위 옵션을 설정으로 노출한다면 **한 번의 호출에 함께**
  넘기는 편이 낫다(호출이 갈리면 어느 값이 언제 반영되는지가 effect 의존성 순서에 걸린다).
- **`editorRulers` 입력 형태**: 기존 설정 표면에 목록 입력은 `remoteAllowedHosts` 의 추가/삭제 행
  하나뿐인데, 열 번호는 짧고 여러 개를 한 번에 고치는 값이라 **`TextField` + 콤마 구분 문자열**을
  택했다(`shellOverride`·`aiOmlxBaseUrl` 과 같은 blur 커밋 컴포넌트). 파싱·표기는
  `shared/lib/editor-rulers.ts`(`parseEditorRulers`/`formatEditorRulers`)가 담당하고, 정규화 규칙
  (1~1000 범위 밖 버림 · 중복 제거 · 오름차순 · 16개 상한)은 백엔드
  `settings::service::sanitize_editor_rulers` 와 **양쪽 거울**이다. 정렬까지 하는 이유는 화면이
  저장값을 그대로 다시 그리기 때문 — 상세는 `ipc-contract.md` d-53 U1 절.
- diff 에디터 쪽 2건(`editorDiffHideUnchangedRegions`·`editorDiffShowMoves`)은 `git.md` §4.

## 16. on-save 정리 2건 (d-53 U2 — `docs/acknowledge/2026-08-29-d53-ux-batch-contract.md` §1 U2)

감사 §5 "중간 규모"의 trim/final-newline on-save. 둘 다 **VS Code 파리티로 기본 off**
(`files.trimTrailingWhitespace`·`files.insertFinalNewline`)이고, 설정 화면 Editor 섹션의 on-save
묶음(포맷·Organize Imports·Fix All) 바로 뒤에 토글로 붙는다.

| 설정 | 동작 | monaco 액션 |
|------|------|-------------|
| `trimTrailingWhitespaceOnSave` | 모든 줄의 후행 공백 제거 | `editor.action.trimTrailingWhitespace` |
| `insertFinalNewlineOnSave` | 마지막 줄이 비어 있지 않으면 줄바꿈 1개 추가 | `editor.action.insertFinalNewLine` |

- **적용 지점은 저장 파이프라인 한 곳**이다 — `widgets/editor-pane/use-editor-file-persistence.ts`
  의 `handleSave`. ⌘S·자동 저장·format-on-save가 전부 이 함수를 지나므로, 트리거별로 정리를 따로
  달지 않는다. 순서는 **Code Actions on Save(명시 저장만) → format-on-save → 후행 공백 제거 →
  마지막 줄바꿈 → 디스크 쓰기**다. 포맷 뒤에 두는 이유는 포맷터가 만들어 낸 후행 공백까지 정리하기
  위해서고, 공백 제거를 줄바꿈보다 앞에 두는 이유는 "공백만 있는 마지막 줄"을 먼저 비워야 그 뒤에
  줄바꿈이 덧붙지 않기 때문이다.
- **커서·선택 보존**: 두 단계 모두 monaco 내장 액션을 그대로 재사용한다
  (`shared/lib/monaco/on-save-cleanup.ts` 의 `runOnSaveCleanup`, 액션 id 는 키맵 카탈로그
  `shared/lib/monaco/monaco-actions.ts` 가 단일 출처 — 수동 실행 행과 같은 구현이다). 두 액션은
  `executeCommands` 경로로 편집하고 그 커맨드가 선택을 추적하므로, 정리 후에도 캐럿·멀티커서가
  제자리에 남는다. 별도 trim 구현을 두면 이 보존 로직을 다시 쓰는 셈이 된다.
- **자동 저장은 커서가 있는 줄을 건너뛴다**: 자동 저장일 때만 trim 액션에 `{ reason: 'auto-save' }`
  를 넘긴다(VS Code 의 save participant `isAutoSaved` 와 같은 규약). 타이머가 타이핑 도중에 돌면서
  방금 친 들여쓰기를 지우는 것을 막기 위해서다. 명시적 ⌘S 는 인자 없이 실행해 커서 줄까지 정리한다.
- **dirty·미러·정착과의 관계**: 정리 편집은 모델을 거치므로 `onDidChangeContent` → `onChange` →
  `handleChange` 로 draft 가 갱신된 뒤 `readDraft()` 가 최종 내용을 읽는다. `savingRef` 가 이미 서
  있어 이 재진입이 자동 저장을 다시 무장시키지 않고(포맷 단계와 동일), 저장 성공 시
  `settleDraftToDiskContent` 가 미러 타이머·에폭·`syncedContent` 를 정리한다(§3 저장 정착).
- **실패는 삼킨다**: 액션이 등록돼 있지 않거나 실패해도 저장 자체는 진행한다(포맷 단계와 같은 규약).
  read-only 파일은 애초에 `handleSave` 앞단에서 걸러지고, 두 액션의 monaco 자체 precondition 도
  `writable` 이라 이중으로 막힌다.
- **적용 범위 밖**: Claude Code 의 외부 저장 요청(`ide:save-requested`)과 충돌 해소 쓰기는 모델만
  들고 에디터 인스턴스(커서)를 전제하지 않아 이 정리를 태우지 않는다 — 계약 §5 이월.
  **untitled 탭의 Save As** 도 마찬가지다(`untitled-pane.tsx` 의 `handleSaveAs` 는 `useSaveFile` 을
  직접 부른다) — untitled 은 원래 format-on-save 도 타지 않고 `.editorconfig` 대상도 아니라 정리
  대상이 되는 파일 규약 자체가 없다.

## 17. EditorConfig (d-53 U3 — `docs/acknowledge/2026-08-29-d53-ux-batch-contract.md` §1 U3)

감사 §5 "중간 규모"의 EditorConfig. **기본 off** (`editorConfigEnabled`, 설정 화면 Editor 섹션의
on-save 묶음 바로 뒤 토글) — VS Code 도 코어에 EditorConfig 를 갖지 않고 확장으로 제공하므로 off 가
파리티 기본값이고, 꺼져 있으면 `file_open` 이 체인을 걷지 않아 비용이 0이다.

| `.editorconfig` 프로퍼티 | 적용 대상 |
|---|---|
| `indent_style` | 모델 `insertSpaces` (`space`→true, `tab`→false) |
| `indent_size` · `tab_width` | 모델 `tabSize` — `indent_style = tab` 이면 `tab_width` 우선, 아니면 `indent_size` 우선(없는 쪽은 서로 폴백) |
| `insert_final_newline` | 저장 시 마지막 줄바꿈(§16) — 전역 설정보다 우선 |
| `trim_trailing_whitespace` | 저장 시 후행 공백 제거(§16) — 전역 설정보다 우선 |

`charset` · `end_of_line` 은 다루지 않는다(계약 §5 이월).

- **해석은 Rust**(`domain::file::editorconfig`)가 하고 결과는 `OpenedFile.editorConfig` 에 실려
  온다. 별도 커맨드를 두지 않은 이유는 **파일 열기와 같은 왕복**이어야 에디터가 첫 렌더부터 올바른
  들여쓰기로 모델을 붙이기 때문이다 — 쿼리를 하나 더 두면 틀린 들여쓰기로 붙었다가 한 틱 뒤에
  교정되는 깜빡임이 생긴다. 체인 규칙·글롭 문법·상한은 `ipc-contract.md` d-53 U3 절이 정본이다.
- **들여쓰기는 모델에 건다.** `tabSize`/`insertSpaces`/`detectIndentation` 은 monaco 의 **전역**
  에디터 옵션(`IGlobalEditorOptions`)이라 `editor.updateOptions` 로 보내면
  `StandaloneConfigurationService` → `ModelService._updateModelOptions` 를 거쳐 **열려 있는 모든
  모델**에 걸린다. 파일별 값을 그 경로로 보내면 스플릿한 두 페인이 서로의 들여쓰기를 덮어쓴다.
  그래서 전역 경로에는 기존대로 설정값만 흐르고, editorconfig 오버라이드는
  `CodeEditor` 의 `[editorConfigTabSize, editorConfigInsertSpaces]` effect 가
  `ITextModel.updateOptions` 로 **그 페인의 모델에만** 얹는다.
- **오버라이드가 없으면 모델을 건드리지 않는다.** `.editorconfig` 가 없는 파일(또는 설정 off)의
  동작은 이전과 완전히 같다 — `editorDetectIndentation` 추정 경로가 그대로 유효하다.
- **한쪽 축만 지정해도 된다.** `indent_style` 만 있는 설정은 `insertSpaces` 만 뒤집고 탭 폭은
  추정/설정값을 그대로 둔다(`updateOptions` 는 넘기지 않은 필드를 유지한다).
- **수동 들여쓰기 변경은 유지된다.** 키맵의 `editor.action.indentUsingSpaces`/`indentUsingTabs`/
  `detectIndentation` 으로 사용자가 바꾼 값은 effect 의 의존성이 바뀌지 않는 한 다시 덮어쓰지
  않는다(VS Code 의 EditorConfig 확장과 같은 감각).
- **반영 시점은 파일 열기다.** `.editorconfig` 를 고쳐도 이미 열린 탭에는 반영되지 않는다 — 다음에
  그 파일을 열 때부터다(계약 §5 이월). 단 **오버라이드를 지우는 방향**(`.editorconfig` 에서 그 섹션을
  삭제, `editorConfigEnabled` 를 off)은 재시작 전까지 반영되지 않는다: 오버라이드가 없으면 모델을
  건드리지 않는 규약(위)이 곧 "되돌리지도 않는다"는 뜻이고, 실파일 모델은 탭을 닫아도 dispose 되지
  않아(`model-registry.ts`) 다시 연 탭이 같은 모델을 재사용하기 때문이다 — 계약 §5.
- **다른 페인이 전역 옵션을 뒤집으면 오버라이드가 한 번 풀릴 수 있다.** monaco 의 모델 생성 옵션
  하나(`bracketPairColorization`)가 페인별 `largeFile` 을 타고 전역으로 나가므로, 옆 페인이 대형
  파일을 여닫으면 `ModelService` 가 모든 모델의 들여쓰기를 전역값으로 다시 계산한다. 그 페인의 prop
  은 하나도 바뀌지 않아 재확정 effect 도 돌지 않는다 — 계약 §5.
