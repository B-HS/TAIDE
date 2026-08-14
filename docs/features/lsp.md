# 기능 — LSP

> FR-E. 서버 수명주기(Rust)·에디터 연결(view)·언어별 설정. 결정 근거: ADR-0007,
> `docs/research/lsp-servers.md`(서버별 capabilities·함정 표가 구현 체크리스트),
> `docs/research/monaco.md` §10(transport 구현 예시).

## 1. Rust — lsp 도메인

- **세션**: `LspSession { spec, folders, child, stdin, readerTask, channel, status }`.
  세션 키 = (서버 id × 폴더 집합) — vtsls·basedpyright·marksman 은 프로젝트 추가 시
  `workspace/didChangeWorkspaceFolders` 로 기존 세션에 폴더를 붙이고,
  rust-analyzer(TS7 옵션 포함)는 워크스페이스 루트당 새 세션(ADR-0007).
- **프레이밍**: `Content-Length` 헤더 파싱/작성은 Rust 만 담당. view 와는 파싱된 JSON 객체를
  Channel(세션당 1개)로 주고받는다. `Content-Type` 헤더는 필수 파싱하지 않음(research 함정).
- **supervisor**: 프로세스 비정상 종료 시 백오프 재기동(연속 3회 실패 시 중지 + UI 알림).
  vtsls OOM 대비 `typescript.tsserver.maxTsServerMemory` 설정 노출.
- **루트 탐지**:
  - JS/TS: `package.json`/`tsconfig.json` 상향 탐색. `deno.json(c)`/`deno.lock` 이 더 가까우면 미기동
    (Deno 프로젝트 오탑재 방지 — research 함정).
  - Rust: `cargo metadata --no-deps` 의 `workspace_root`(가까운 Cargo.toml 사용 금지 —
    멤버 크레이트마다 서버가 뜨는 사고). 의존성 소스(`~/.cargo/registry` 등)로 점프한 파일은
    기존 세션에 붙인다.
  - Python: `pyproject.toml`/`.venv` 탐지 + 인터프리터(`python.pythonPath`) 자동 감지(venv 우선).
  - Markdown: 프로젝트 루트. `.git`/`.marksman.toml` 부재 시 크로스파일 기능 없음 안내(빈
    `.marksman.toml` 생성 제안).
- **서버 실행 감지**(ADR-0007 배포 전략): 프로젝트 로컬(`node_modules/.bin`, `.venv/bin`) →
  PATH/rustup(`rust-analyzer` 는 인자 없이 stdio — `--stdio` 플래그 없음) → TAIDE 관리 디렉토리 →
  다운로드 제안. 감지 결과·버전은 설정 UI 에 표시.
- **initialize 정본** (서버별 — research §1~5 의 JSON 예시를 그대로 사용):
  - 공통: `clientInfo: { name: "TAIDE" }`, `workspaceFolders`, **`general.positionEncoding` 협상**
    (`["utf-16"]` 기본 — Rust 쪽 UTF-8 인덱스와의 변환 유틸을 한 곳에 둔다).
  - vtsls: 설정은 `workspace/didChangeConfiguration` + `workspace/configuration` 응답으로
    (`typescript.*`/`vtsls.*`), inlay hints·fuzzy match 옵션 포함.
  - rust-analyzer: **설정 전부 `initializationOptions`**(prefix 제거 트리). `rust-src` 미설치 감지 안내.
    `experimental.serverStatusNotification` 로 인덱싱 상태 표시.
  - basedpyright: `typeCheckingMode: "standard"` 기본·`diagnosticMode: "openFilesOnly"`,
    프로젝트에 `pyrightconfig.json`/`pyproject.toml` 설정이 있으면 미덮어씀. `ruff server` 병행 세션.
  - marksman: initializationOptions 없음.

## 2. IPC

- mutation: `lsp_spawn(spec, folders) → sessionId`, `lsp_send(sessionId, message)`,
  `lsp_stop(sessionId)`, `lsp_restart(sessionId)`
- query: `lsp_sessions(projectId)` (상태·서버 버전·감지 경로)
- Channel: 세션당 1개 — 서버→클라 JSON-RPC 메시지 스트림
- event: `lsp:session-status-changed` (starting/running/crashed/stopped — UI 상태 표시용)

## 3. view — 클라이언트 (Wave A 로 갱신 — `docs/acknowledge/2026-08-14-wave-a-lsp-intelligence-contract.md`)

자체 경량 클라이언트(경로 C) 기준 명세:

- `shared/lib/lsp/client.ts`: 요청 id 관리·capabilities 협상·서버 capabilities 보관·
  didOpen/didChange(incremental)/didClose/**didSave**(저장 성공 시 실전송) 동기화(대형 파일 모드
  파일은 제외 — `editor.md` §3). `handleMessage` 는 응답·알림뿐 아니라 **서버→클라 요청**도
  라우팅한다(`isJsonRpcRequest` 분기 → `server-request-handler-registry.ts` 조회 → 결과/에러를
  JSON-RPC 로 응답, 미등록 메서드는 `-32601 MethodNotFound`).
- `shared/lib/lsp/lsp-session-registry.ts`(정확히는 `widgets/editor-pane/`) 의
  `buildInitializeParams` 가 initialize SSOT — LSP 3.17 고정, codeAction/codeLens/foldingRange/
  implementation/typeDefinition/declaration/workspace.applyEdit·executeCommand·workspaceEdit
  (documentChanges+resourceOperations)·codeLens.refreshSupport 를 전 선언. 매니페스트의
  `LspServerDetection.initializationOptions` 를 그대로 `initialize` 파라미터에 실어 보낸다.
- 서버→클라 요청 핸들러(`server-request-handler-registry.ts`, 앱 전역·세션 비종속):
  `workspace/configuration`(null 채움 배열 — 서버별 매핑은 backlog), `client/registerCapability`
  (빈 결과), `window/workDoneProgress/create`(빈 결과), `workspace/codeLens/refresh`·
  `workspace/inlayHint/refresh`(구독자 발화 — **앱 전역 리스너**라 한 세션의 refresh 가 동일
  언어의 다른 세션 구독자도 함께 깨움, 과다 재요청만 유발). `workspace/applyEdit` 는
  `workspace-edit-apply-handler.ts` 가 별도 등록(부트스트랩, 아래).
- 어댑터(기능별 1파일, `shared/lib/lsp/adapters/`, Monaco provider 등록):
  completion(+resolve, snippet), hover, signatureHelp, definition, implementation, typeDefinition,
  declaration(4종은 `definition.ts` 의 `createLocationRequestAdapter` 팩토리 공용 — capability
  술어·LSP 메서드·monaco 등록 함수만 주입), references, documentHighlight, documentSymbol,
  rename(+prepare, WorkspaceEdit 적용기로 통일 — 미열린 파일 rename 가능), formatting
  (document/range), **codeAction**(+resolve, on-save 겸용), **codeLens**(+resolve,
  `editorCodeLensEnabled` 설정 게이트), inlayHints, foldingRange(0-based↔1-based 변환,
  `FOLDING_RANGE_CLIENT_LIMIT`=5000 클라 측 절삭), selectionRange. semanticTokens·documentLink 는
  2차 잔존. `LANGUAGE_ADAPTER_REGISTRARS` 배열에 표준 3-인자(`monaco, client, languageId`) 어댑터가
  모두 등록되고, `serverId` 가 추가로 필요한 diagnostics·codeAction 과 `isCodeLensEnabled` 게터가
  필요한 codeLens 는 `ensureLanguageRegistered` 가 별도로 호출한다.
- **WorkspaceEdit 적용기**(`workspace-edit-applier.ts`): `changes`/`documentChanges`
  (TextDocumentEdit·CreateFile·RenameFile·DeleteFile)를 배열 순서대로 적용. 열린 모델은
  `pushEditOperations`, 미열린 파일은 file IPC 로 읽기→치환→저장. monaco 표준
  `StandaloneBulkEditService` 를 우회한다(텍스트 편집 외·미열린 모델에 throw 하는 한계 회피).
  Code Action 의 `resolveCodeAction` 훅과 rename 어댑터가 이 적용기를 공유한다.
- **Code Actions on Save**(`editor-pane.tsx` `handleSave`): 설정 `fixAllOnSave`/`organizeImportsOnSave`
  (boolean, 기본 false) 가 켜져 있으면 명시적 저장(⌘S, autoSave 제외)에서 `source.fixAll` →
  `source.organizeImports` 순으로 kind 별 요청 후 적용, 이어서 formatOnSave, 저장, `didSave` 전송.
  `CODE_ACTIONS_ON_SAVE_TIMEOUT_MS`(5초) 상한 — 초과 시 대기만 중단하고 `editor.codeActionsOnSaveSkipped`
  토스트.
- **command 중계**(`command-relay.ts`): 세션이 뜰 때 `executeCommandProvider.commands` 를
  `registerSessionExecuteCommands` 로 monaco 전역 커맨드로 일괄 등록(세션 dispose 시 해제) —
  실행 시 `workspace/executeCommand` 로 서버에 중계하며 그 요청 promise 를 그대로 반환한다(fire-and-forget
  금지 — on-save 의 "edit 적용 성공 후에만 command 실행" 순서가 이 await 에 의존한다). 클라이언트 전용
  네비게이션 커맨드(`editor.action.showReferences`/`rust-analyzer.showReferences`/
  `rust-analyzer.gotoLocation`)는 앱 부트스트랩(`app/bootstrap-lsp.ts`)에서 1회
  `registerLspClientNavigationCommands` 로 등록 — CodeLens 클릭이 이 두 계열을 소비한다. rust-analyzer
  는 `experimental.commands.commands` 에 그 2종을 선언하지 않으면 HasImpls/HasReferences/run/debug
  CodeLens 를 아예 생성하지 않으므로(자체 클라 커맨드 광고 여부로 렌즈 생성을 게이트) `initialize` 의
  `capabilities.experimental` 에 이 선언이 필수다. 다인자 커맨드 실행에는 공개 monaco.d.ts API 가 없어
  `ICommandService` 를 딥 임포트한다 — 근거는
  `docs/acknowledge/2026-08-14-monaco-command-service-deep-import.md`.
- **cross-file 오프너**(`shared/lib/editor-opener-bridge.ts`): `monaco.editor.registerEditorOpener`
  를 부트스트랩에서 1회 등록. 같은 모델 이동은 monaco 기본 처리에 위임(false 반환), untitled 는
  이미 열린 에디터로 직접 reveal, file scheme 은 `requestOpenFileFromEditor` 로 pub/sub emit —
  `widgets/editor-area/editor-area.tsx` 가 구독해 `requestReveal` + `openTab(preview)` 실행. 이
  오프너 없이는 F12/⌘F12 cross-file·Peek·F8(파일 간) 이동이 무음 실패한다.
- **Peek 미리보기 모델 선생성**(`peek-model-preload.ts`): definition 계열·references 어댑터가
  위치 반환 직전 대상 파일의 monaco 모델을 `PEEK_MODEL_PRELOAD_LIMIT`(8, distinct 상한)까지
  선생성(file IPC 읽기, `normal` tier 만 대상, 미부착 시 지연 dispose). `entities/editor/
  model-registry.ts` 의 `getOrCreateModel` 이 이렇게 선생성된 orphan 모델을 나중에 실제 탭이
  열릴 때 등록소로 흡수한다(중복 생성 예외 방지).
- **CancellationToken**: 전 어댑터가 요청 완료 후 토큰을 확인해 취소된 응답을 폐기한다. 서버로의
  실제 중단 신호(`$/cancelRequest`)는 없음 — 클라이언트 측 "결과 폐기" 수준(client.ts 에 취소
  전달 경로 추가는 backlog).
- 진단: pull 지원 서버는 `textDocument/diagnostic` 폴링(변경 debounce), push 전용은
  `publishDiagnostics` 수신 → `monaco.editor.setModelMarkers`. **이중 소비 금지**(서버별 정책 고정).
  원본(`code`/`data`/`source`)은 `diagnostics.ts` 의 사이드 맵(`${owner}::${uri}` 키)에 별도
  보관되어 codeAction 의 `context.diagnostics` 가 손실 없이 참조한다.
- 서버 capabilities 에 없는 기능은 어댑터 미등록(research 의 capabilities 비교표가 기준 —
  예: marksman 은 inlay hints·formatting 없음, basedpyright 는 folding 없음 등).
- executeCommand: basedpyright `basedpyright.organizeimports` 처럼 **광고되지 않는 private 커맨드**가
  있으므로 화이트리스트 검증으로 차단하지 않는다.

## 4. 에디터 연동 규칙

- TS/JS: LSP 세션이 뜨면 내장 ts worker 기능을 `setModeConfiguration` 으로 끄고 일원화
  (`editor.md` §8). LSP 실패/미설치 시 내장 worker 가 fallback.
- 파일 열기 → 해당 언어 세션 lazy 기동(프로젝트 capability attach 시점이 아니라 첫 didOpen 시점).
- 파일 rename: marksman 은 didRename 미지원 → didClose(구)+didOpen(신) 전송(research 함정).
- 상태 표시: 상태바(또는 탭 영역)에 서버 상태·인덱싱 진행 표시. 크래시 시 재시작 버튼.

## 5. 수명주기 · 누수 방지

- 세션 소유는 Rust. 프로젝트 close(capability detach) 시: 공유 세션이면
  `didChangeWorkspaceFolders(removed)`, 마지막 폴더면 `shutdown`→`exit`→프로세스 종료 확인(타임아웃
  후 kill). reader task 는 CancellationToken 으로 중단.
- view 클라이언트는 세션 Channel 콜백·Monaco provider disposable 을 모두 보관했다가
  세션 종료/프로젝트 전환 시 dispose.
- view reload: Rust 세션은 유지 — view 는 `lsp_sessions` 재조회 후 새 Channel 로 재연결
  (재연결 시 열린 문서 didOpen 재전송).

## 6. 플러그인 확장 (FR-E3)

- 플러그인 매니페스트의 `lsp` 기여(`features/plugins.md`)가 `LanguageServerSpec` 으로 변환되어
  내장 4종과 동일한 경로로 등록된다. 내장/플러그인 차이는 spec 출처뿐.

## 7. 범위

| 1차 | 2차 |
|-----|-----|
| vtsls·rust-analyzer·basedpyright(+ruff)·marksman 기동/감지, 진단·completion·hover·definition/implementation/typeDefinition/declaration·references·rename·formatting·signatureHelp·inlayHints·documentSymbol·selectionRange·codeAction(+resolve, on-save 포함)·codeLens·foldingRange | semanticTokens·documentLink·call hierarchy·workspace symbol(⌘T)·TS7 백엔드 옵션·다운로드 매니저·codeLens runSingle/debugSingle·workspace/configuration 서버별 매핑 |
