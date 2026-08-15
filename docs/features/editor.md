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
  - DiffEditor 의 original 모델(`taide://git/...` 스킴)은 diff 탭 닫을 때 함께 dispose.
- viewState: 탭 전환 시 `saveViewState()` → 레지스트리에 보관, 복귀 시 `restoreViewState()` + `editor.focus()`
  (restore 는 포커스를 주지 않음 — 함정 7). viewState 는 직렬화 가능 → Rust layout 도메인에 미러해
  재시작 복원에 사용(`data-model.md` layout.json).

## 3. 파일 열기/저장 흐름

- 열기: Rust `file_open(path)` → 내용 + 감지 언어 id + 파일 크기. 대형 파일 판정은 Rust 가 선행:
  **2MB 또는 50,000줄 초과 시 "대형 파일 모드"** — research §9 의 축소 옵션 프리셋 적용
  (minimap·folding·bracketColorization off, `largeFileOptimizations: true` 유지) + LSP didOpen 미전송.
  20MB 초과는 열람 전용(편집 비활성) + 안내 배너.
- 저장: `⌘S` → 모델 내용 `file_save(path, content)`. 저장 완료 시 dirty 해제·버퍼 미러 삭제.
  포맷-온-세이브는 설정 옵션(LSP formatting 연동, 기본 off — 1차).
- **hot-exit 미러(파일 탭)**: 모델 변경 debounce `HOT_EXIT_MIRROR_DEBOUNCE_MS`(500ms,
  `shared/constants/mirror.ts`) → `file_mirror_dirty(projectId, path, content, diskModifiedMs)`
  (`diskModifiedMs` = 열람 시점 `file.modifiedMs` baseline, ADR-0004 예외 경로, `data-model.md` §6).
  window `blur`·탭 언마운트 시 즉시 flush(`entities/editor/mirror-flush-registry.ts` 에 탭당 flush
  콜백 등록). 프로젝트 활성화 시 `file_list_mirrors(projectId)` 1회 조회 → path→entry 캐시
  (TanStack Query `staleTime: Infinity`, 매 `file_mirror_dirty` 성공마다 `setQueryData` 로 직접
  갱신 — 재조회 없이 캐시를 최신 상태로 유지). 탭 활성화 시 미러가 있으면
  `applyExternalContent` + `setDirty(true)` 로 lazy 복원, Rust 가 `disk_modified_ms` 대비 현재
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
- 렌더: **injected text(`after`)** 방식(research monaco §5(a)) — 커서 라인 끝에 이탤릭 흐린 텍스트.
  `content` 는 반드시 한 줄(개행 제거 — 함정 8). `editorBlame.*` 토큰.
- 데이터: `onDidChangeCursorPosition` debounce 300ms → `git_blame_range(path, line, line)`.
  Rust 쪽 `(path, HEAD oid)` 캐시 + `blame_buffer` 로 미저장 편집 반영(`docs/research/git2.md` §4).
- hover(2차): blame 텍스트 위 hover 시 커밋 상세(메시지 전문·author·SHA·날짜).

## 6. 키바인딩 (FR-D4)

3계층 규칙 (research monaco §7 확정):

| 계층 | 대상 | 구현 |
|------|------|------|
| Monaco 내장 | 멀티커서·라인 조작·주석·찾기/바꾸기·go to def/refs·rename·포맷·quick fix 등 에디터 스코프 전부 | 그대로 사용 (VSCode 동일 키) |
| Monaco 보강 | 명령 팔레트 `⌘⇧P`(기본은 F1 뿐), `⌘S` 저장 등 | `addKeybindingRules` / `addAction` |
| 앱 전역 | `⌘P` 파일 퀵오픈, `⌘W` 탭 닫기, `⌘B` 사이드바, `⌘⇧F` 검색, `⌃Tab` 탭 전환, `⌘\` 분할, `⌘⇧E/G` 뷰 전환, `⌃\`` 터미널 | window keydown 캡처 단계 핸들러(단일 keymap 모듈) + 필요 시 Tauri 메뉴 accelerator. 에디터 내 충돌 키는 `{ keybinding, command: null }` 로 monaco 기본 해제 |

- 전체 키 목록 정본: `docs/research/vscode-behaviors.md` §8 표. 앱 전역 키맵은
  `shared/lib/keymap.ts` 한 곳에서 선언한다(설정 오버라이드 대비 데이터 구조로).
- WebView 기본 동작(`⌘P` 인쇄 등) preventDefault 필수(vscode-behaviors 함정 절).

## 7. 테마 연동

- `shared/lib/monaco/theme.ts`: 테마 토큰(`docs/theme-system.md` §4.2) → `defineTheme('taide', ...)` 변환.
  - **colors 값은 hex 만 허용**(CSS 변수 불가 — 잘못되면 조용히 빨강. research 함정 3).
    테마 JSON 이 hex 원본을 갖고 있으므로 CSS 변수 경유 없이 직접 변환한다.
  - 테마 전환 = 같은 이름 defineTheme 재호출 + `setTheme`.
- semantic highlighting: `'semanticHighlighting.enabled': true`(상시, create 옵션 — §8 참조) + LSP
  semantic tokens.

## 8. Semantic Tokens (Wave F — `docs/acknowledge/2026-08-15-wave-f-editor-presentation-contract.md`)

- **capability 선언**(`lsp-session-registry.ts` `buildInitializeParams`):
  `textDocument.semanticTokens { requests: { full: { delta: true } }, tokenTypes: 표준 23종,
  tokenModifiers: 표준 10종, formats: ['relative'], augmentsSyntaxTokens: true }` +
  `workspace.semanticTokens.refreshSupport: true`. range 는 1차 제외(뷰포트 스코프 요청 미요청).
  상수(`SEMANTIC_TOKEN_TYPES`/`SEMANTIC_TOKEN_MODIFIERS`)는 `shared/lib/lsp/protocol.ts` 에
  `as const` + 유도로 정의.
- **어댑터(`shared/lib/lsp/adapters/semantic-tokens.ts`) — 재인코딩 전략**: monaco standalone 은
  `semanticHighlighting.enabled: true` 를 켜도 서버 legend 문자열을 직접 이해하지 못하고, 매칭
  실패 토큰은 기본 전경색(shiki 색 위)으로 **전면 덮어써버린다**(워시아웃) — 따라서 어댑터가 서버
  legend 를 `mapping-tables.ts` 의 `SEMANTIC_TOKEN_TYPE_MAP` 으로 TAIDE 토큰명(`SYNTAX_TOKENS`
  31종)에 재매핑하고, **자체 monaco legend**(매핑된 토큰명 집합만)를 노출한다. 매핑되지 않는
  서버 토큰 타입은 스트림에서 **드롭**(shiki 색 보존) — modifier 는 표준 10종 이름으로 통과.
  legend 의 각 타입명은 `toSemanticTokenLegendScope`(`mapping-tables.ts`)로 `taideSemantic.<token>`
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
- **편집 UI**: 설정 SNIPPETS 섹션에 "Manage Snippets"(→ `widgets/snippet-editor`, ThemeEditor
  형 — 파일 선택·prefix/body/description/scope 폼·저장/삭제 `AlertDialog`) ·
  "Open Snippets Folder"(`systemOpenAppDataPath('snippets')`) 버튼.
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
