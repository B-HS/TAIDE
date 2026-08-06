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
- dirty 미러: 모델 변경 debounce 1~2s → `file_mirror_dirty(path, content)` (ADR-0004 예외 경로,
  `data-model.md` §6).
- 외부 변경: watcher 이벤트로 열린 파일이 바뀌면 — dirty 아니면 조용히 리로드(viewState 유지),
  dirty 면 충돌 배너(디스크 내용 보기 / 덮어쓰기 / 유지).

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
- semantic highlighting: `'semanticHighlighting.enabled': true` + LSP semantic tokens(`lsp.md`).

## 8. 내장 TS/JS 지원 (LSP 이전 베이스라인)

- 1차 기동 시 TS/JS 는 내장 ts worker(TS 5.9.3)로 시작: `setEagerModelSync(true)`,
  Rust 가 프로젝트 `tsconfig.json` 을 읽어 `setCompilerOptions` 주입(paths·jsx 등).
- 한계(연구 확정): node_modules 타입 해석·프로젝트 전역 참조 불가 → LSP 전환 시
  `setModeConfiguration` 으로 내장 기능을 끄고 일원화(`lsp.md` 참조).

## 9. 수명주기 · 누수 방지

- pane unmount: `editor.dispose()` (model 은 레지스트리 소유 — dispose 하지 않음).
- 탭 닫기: viewState 저장 → model dispose (§2).
- decoration collection 은 에디터 dispose 로 함께 정리되지만, 파일 전환 시 `clear()` 명시 호출.
- blame·gutter 요청은 최신 요청만 유효(경쟁 응답 무시 — 요청 시퀀스 토큰).

## 10. 범위 (1차/2차)

| 1차 | 2차 |
|-----|-----|
| Monaco 마운트·모델 레지스트리·viewState 복원·저장/dirty·대형 파일 모드·git gutter·인라인 blame·키바인딩 3계층·테마 연동·내장 TS | gutter 클릭 peek, blame hover 상세, 실시간(버퍼) gutter diff, 포맷-온-세이브 기본화, 파일 퀵오픈(⌘P) 고도화(심볼 검색), minimap git 표시 |
