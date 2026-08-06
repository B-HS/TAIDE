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

- [ ] Vite 8 에서 monaco 0.56 `?worker` + subpath exports 실동작 (실패 시 optimizeDeps.exclude 우선)
- [ ] `typescript.setModeConfiguration` 전체 시그니처 (내장 TS 기능 끄기)
- [ ] Tauri 커스텀 프로토콜에서 module worker + CSP 조합
- [ ] Claude Code ctrl+g 의 에디터 결정 우선순위(#18990 open) — EDITOR 환경변수 기준으로만 설계
- [ ] IDE MCP 프로토콜의 비공식 부분(12개 도구·env 주입) — 스모크 테스트 필수
- [ ] `Channel` 인스턴스의 커맨드 간 재사용 가능 여부 (세션당 새 Channel 로 설계해 회피)
- [ ] basedpyright wheel 의 Node 런타임 동봉 여부 / marksman macOS universal2 여부
- [ ] Monaco 대형 파일 임계값이 0.56 번들에 그대로인지 (자체 측정으로 정책 확정)
- [ ] shadcn `add` 재실행 시 로컬 수정 덮어씀 — vendored 파일 수정 후 재실행 금지 규칙 준수

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
