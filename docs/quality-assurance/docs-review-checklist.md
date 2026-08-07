# 문서 정합성 검수 (2026-08-06)

> 문서화 세션 종료 전 교차 검토 결과. 항목별로 [x] = 해소됨 / [ ] = 구현 시 확인 필요.

## 1. 발견·해소한 불일치

- [x] `research/react-frontend-stack.md` 의 tauri-specta "1.0.2" 표기 — v1 용 stable 을 오인.
  정본은 `research/tauri-v2.md` §7(=2.0.0-rc.25). → `tech-stack.md` 정정 기록에 명시.
- [x] research 문서들의 zustand persist·localStorage 제안(레이아웃·테마·뷰 상태) —
  ADR-0004/0008 과 충돌. → ADR-0008 "주의" 절 + `tech-stack.md` 정정 기록으로 교정.
  FOUC 방지도 localStorage 대신 window backgroundColor + visible:false (`theme-system.md` §4.1).
- [x] `data-model.md` SessionState.window 필드 — ADR-0009(window-state 플러그인 위임)와 충돌 → 제거.
- [x] 그래프 레인 색 8색(theme 초안) vs 12색(Git Graph 참조) → 12색으로 통일.
- [x] `@theme` 직접 값으론 런타임 테마 전환 불가 → `theme-system.md` 에 `@theme inline` 2단 구조 명시.

## 2. 의도된 차이 (충돌 아님 — 근거 기록)

- [x] 터미널 flow control: `research/xterm-pty.md`(reader pause = 커널 backpressure) vs
  `research/performance-memory.md`(읽기 유지 + ring buffer 절단). **채택: reader pause**
  (xterm 공식 가이드의 pty.pause 와 동일 원리, 단순·확실). ring buffer 는 복원 용도로 별도 유지.
- [x] LSP 클라이언트: `research/monaco.md` 는 경로 B(monaco-languageclient) 권장, ADR-0007 은
  경로 C(자체 경량) — 번들·수명주기 통제 근거로 의도적 이탈. 사용자 확정 완료(§3).
- [x] git commit/push 를 git2 가 아닌 CLI 로 — hook 실행 요구 때문(ADR-0006). research 의
  git2 push 구현 예시는 백엔드 교체 대비 참고로 유지.

## 3. 사용자 결정 (2026-08-06 전건 확정 — 관련 문서 갱신 완료)

- [x] 파일 트리 = Rust 트리 + @tanstack/react-virtual → `explorer-sidebar.md` §2.1, `tech-stack.md`
- [x] LSP 클라이언트 = 자체 경량 클라이언트 → ADR-0007 승인, `lsp.md` §3
- [x] tauri-specta = RC 핀 고정 채택 → ADR-0011 승인, `tech-stack.md`
- [x] 커밋 대상 규칙 = staged 우선 + 없으면 스테이지 제안 → `git.md` §3
- [x] dnd-kit = 레거시 라인(core 6.3.1 + sortable 10) → `tech-stack.md`

## 4. 구현 시 실측 검증 필요 (research "미확인" 승계 — 중요한 것만)

- [x] Vite 8 에서 monaco 0.56 `?worker` + subpath exports **실동작 확인**(2026-08-06) —
  `worker: { format: 'es' }` + `optimizeDeps.exclude: ['monaco-editor']` 조합으로 빌드 시
  editor/ts/json/css/html worker 5종이 **개별 청크로 분리 생성**됨. 경로는 `esm/vs` 접두 없이
  `monaco-editor/editor/editor.worker.js?worker` (0.56 exports 맵 `"./*.js": "./esm/vs/*.js"` 기준).
  **함정**: `setup.ts` 를 아무도 import 하지 않으면 통째로 tree-shake 되어 `MonacoEnvironment` 가
  설정되지 않는다 → monaco 접근을 전부 `setup.ts` 의 재export 경유로 라우팅해야 한다.
- [ ] `typescript.setModeConfiguration` 전체 시그니처 (내장 TS 기능 끄기)
- [ ] Tauri 커스텀 프로토콜에서 module worker + CSP 조합
- [ ] Claude Code ctrl+g 의 에디터 결정 우선순위(#18990 open) — EDITOR 환경변수 기준으로만 설계
- [ ] IDE MCP 프로토콜의 비공식 부분(12개 도구·env 주입) — 스모크 테스트 필수
- [ ] `Channel` 인스턴스의 커맨드 간 재사용 가능 여부 (세션당 새 Channel 로 설계해 회피)
- [ ] basedpyright wheel 의 Node 런타임 동봉 여부 / marksman macOS universal2 여부
- [ ] Monaco 대형 파일 임계값이 0.56 번들에 그대로인지 (자체 측정으로 정책 확정)
- [ ] shadcn `add` 재실행 시 로컬 수정 덮어씀 — vendored 파일 수정 후 재실행 금지 규칙 준수

## 4.1 Phase 0~1 에서 실측으로 해소한 항목 (2026-08-06)

- [x] **React Compiler 실제 적용** — 빌드 산출물에 `useMemoCache` ×7 / `compiler-runtime` ×3 확인.
  (minify 빌드에서는 문자열이 사라지므로 `TAURI_ENV_DEBUG=1` 빌드로 확인해야 한다.)
  Babel 경유 비용은 90모듈 기준 **+0.1s** (1.60s vs 1.48s, 각 3회 측정).
- [x] **react-resizable-panels v4 API 확정** (`.d.ts` 직접 확인) — `Group`/`Panel`/`Separator`,
  `Layout = { [panelId: string]: number }`(flexGrow), `onLayoutChanged(layout, meta)` 는
  **포인터 릴리즈 후에만** 호출되고 `meta.isUserInteraction` 으로 사용자 조작 여부를 구분한다.
  → `tabs.md` §5 의 "드래그 종료 시 1회 `layout_resize`" 요구를 그대로 만족한다.
- [x] **Tauri `devCsp` 존재 확인** — dev 는 Vite 가 **인라인 module script** 를 주입하므로
  `script-src 'self'` 만으로는 막힌다. prod CSP 는 조인 채로 두고 dev 만 `'unsafe-inline'` + ws 허용.
- [x] **Vite 8 은 esbuild 를 번들하지 않는다** — Tauri 템플릿의 `minify: 'esbuild'` 는 빌드 실패.
  `'oxc'` 로 교체.
- [x] **shadcn `resizable` 은 사용 불가** — 구 `PanelGroup` API 기반이라 v4 와 불일치. 직접 사용한다.
- [ ] `tauri_runtime_wry` 가 기동 시 1회 남기는 `web content process terminated` 는 wry 가 웹뷰를
  재생성하며 복구한다(크래시 리포트 없음, IPC 왕복 정상). 재발·악화 시 재조사.

## 5. 요구사항 커버리지 (PRD FR ↔ features 문서)

- [x] FR-A(1~5) → layout-shell.md / FR-B(1~6) → tabs.md / FR-C(1~5) → explorer-sidebar.md
- [x] FR-D(1~5) → editor.md / FR-E(1~4) → lsp.md + plugins.md / FR-F(1~7) → git.md
- [x] FR-G(1~5) → terminal.md / FR-H(1~2) → agent-integration.md
- [x] FR-I(1~3) → theme-system.md / FR-J(1~2) → plugins.md + architecture.md §3
- [x] 원 프롬프트 요구 전수 대조: 폴더 프로젝트·테마 세분화·sidebar|content·멀티 프로젝트·
  탭 기본 2개·탭 DND·탭 복원·드래그 스플릿·좌측 사이드바(파일/검색/Git)·트리 클릭 열기·
  gutter git 표시·인라인 blame(`HS, 4 days ago, message`)·VSCode 단축키·LSP 5종+플러그인·
  remote 주소/changes/graph·changes 분류·diff split·context menu(되돌리기/add/이동)·
  commit/push GUI·사이드바 상태 아이콘·에이전트 감지·터미널 링크 cmd+click·ctrl+g 연동·
  사용자 기본 셸·글자 크기 단축키·iTerm 편의 기능·프로젝트 추상화(remote-control 대비)·
  Rust 소유 상태/누수 방지 — 전부 매핑 확인.

## 6. 컨벤션 준수 점검

- [x] 이모지·박스 다이어그램 없음(디렉토리 트리·화살표 텍스트는 컨벤션 문서 자체가 쓰는 형식)
- [x] FSD 배치 규칙 반영(architecture §5, 각 feature 문서), 코드 스니펫은 arrow function·타입 유도
- [x] 결정·합의는 acknowledge, 상태는 PROCESS.md, 검증은 quality-assurance 로 분류(ai-process §9)

## 4.2 Phase 2 에서 실측으로 확인한 항목 (2026-08-06)

- [x] **@tanstack/react-virtual 는 React Compiler 와 비호환** — `eslint-plugin-react-hooks@7` 이
  `Compilation Skipped: Use of incompatible library` 경고를 내고 해당 컴포넌트만 컴파일을 건너뛴다.
  버그가 아니라 컴파일러가 안전하게 bail 한 것이며, 그 컴포넌트는 일반 React 시맨틱으로 동작한다.
  (research/react-frontend-stack.md §7 의 "Compiler 호환 미확인" 항목에 대한 답)
  → 가상 스크롤 컴포넌트는 수동 최적화 여지를 남겨두되, 현재 규모에선 조치 불필요.
- [x] **notify-debouncer-full 0.7 의 `Drop`** 이 감시를 자동 중단한다 — `WatcherHandle` 이
  debouncer 를 소유하므로 프로젝트 닫기 시 핸들을 map 에서 제거하는 것만으로 감시가 끊긴다.

## 7. 실기기 수동 QA — 자동화 테스트에서 제외된 항목

> 실제 OS 리소스(사용자 휴지통 등)를 건드리는 동작은 `cargo test` 로 자동 검증하지 않는다
> (테스트 실행마다 실제 휴지통에 파일이 쌓이는 부작용 때문). 아래는 실기기에서 사람이 직접 확인한다.

- [ ] **파일/폴더 삭제 → 실제 휴지통 이동** (`domain/file/service.rs::delete_entry`) —
  탐색기에서 파일 삭제 후 macOS Finder 휴지통에 해당 파일이 보이는지, 원래 경로에서 사라졌는지 확인.
  폴더 삭제도 동일하게 확인(재귀 이동).
- [ ] 휴지통에서 복원 시 원래 경로로 되돌아오는지 확인 (OS 표준 동작 의존 — 앱이 관여하지 않음).
