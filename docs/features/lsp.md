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

## 3. view — 클라이언트 (ADR-0007 결정 3 확정 후 본 절 갱신)

자체 경량 클라이언트(경로 C) 기준 명세:

- `shared/lib/lsp/client.ts`: 요청 id 관리·capabilities 협상·서버 capabilities 보관·
  didOpen/didChange(incremental)/didClose 동기화(대형 파일 모드 파일은 제외 — `editor.md` §3).
- 어댑터(기능별 1파일, Monaco provider 등록):
  completion(+resolve, snippet), hover, signatureHelp, definition/typeDefinition/implementation,
  references, documentHighlight, documentSymbol, rename(+prepare), formatting(document/range/onType),
  codeAction(+resolve), inlayHints, semanticTokens(full/range/delta), foldingRange, selectionRange,
  codeLens(2차), documentLink(2차).
- 진단: pull 지원 서버는 `textDocument/diagnostic` 폴링(변경 debounce), push 전용은
  `publishDiagnostics` 수신 → `monaco.editor.setModelMarkers`. **이중 소비 금지**(서버별 정책 고정).
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
| vtsls·rust-analyzer·basedpyright(+ruff)·marksman 기동/감지, 진단·completion·hover·definition·references·rename·formatting·signatureHelp·inlayHints·documentSymbol | semanticTokens·codeLens·codeAction 전체·call hierarchy·workspace symbol(⌘T)·TS7 백엔드 옵션·다운로드 매니저 |
