# 로드맵 · 전체 구현 체크리스트

> 구현 순서의 정본. 각 항목의 상세 스펙은 `docs/features/*.md`, 검증 항목은
> `docs/quality-assurance/`. 구현 세션에서는 이 체크리스트를 `docs/PROCESS.md` 로 복사·세분화해
> 진행 상태를 기록한다. 각 Phase 는 "동작하는 상태"로 끝난다(수직 슬라이스).

## Phase 0 — 스캐폴딩

- [ ] Tauri 2 + Vite 8 + React 19 프로젝트 생성 (bun, `tech-stack.md` 버전 고정)
- [ ] React Compiler 설정 + 적용 검증 (DevTools `Memo` 배지 / 산출물 `compiler-runtime` grep — research 함정)
- [ ] Tailwind 4.3 + shadcn init (`shared/ui`, components.json aliases — FSD)
- [ ] FSD 디렉토리 골격 (`app/widgets/features/entities/shared`) + path alias
- [ ] Rust 골격 (`state.rs`/`error.rs`/`events.rs`/`domain/`/`infra/` — `architecture.md` §2)
- [ ] tauri-specta 배선 + `bindings.ts` 생성 파이프라인 (ADR-0011 확정 시)
- [ ] `useTauriEvent` 훅, QueryClient 기본값(networkMode 등 — ADR-0008), 에러 토스트(sonner)
- [ ] 검증 스크립트: `typecheck`·`test`·`lint`(eslint react-hooks) + `cargo clippy`
- [ ] capabilities 최소 구성 + CSP 계획 (Monaco 요구 반영 — `ipc-contract.md` §4)

## Phase 1 — 셸: 프로젝트·레이아웃·탭·테마·영속화

- [ ] persist 인프라 (원자적 쓰기 + version 마이그레이션 — ADR-0009, `data-model.md`)
- [ ] project 도메인 (open/close/activate/reorder + 세션 복원) — `layout-shell.md`
- [ ] 앱 사이드바 (프로젝트 아이콘·상태 아이콘·context menu·DND 정렬)
- [ ] layout 도메인 (PaneNode 트리 + 전 mutation + revision 이벤트) — `tabs.md`
- [ ] 탭 바 UI (preview/pin/dirty·context menu·키보드) + react-resizable-panels 스플릿
- [ ] 탭 DND (재정렬 + 5분할 드롭 존 스플릿) — dnd-kit
- [ ] 테마 시스템 (스키마·내장 dark/light·CSS 변수 주입·`@theme inline` 매핑·FOUC 방지) — `theme-system.md`
- [ ] window-state 플러그인 + 빈 상태(웰컴) 화면
- [ ] 복원 E2E: 재시작 시 프로젝트·탭·스플릿·활성 상태 복원 (성공 기준 3)

## Phase 2 — 에디터·파일

- [ ] file 도메인 (open/save/create/rename/delete + 크기 정책 판정 + dirty 미러) — `editor.md` §3
- [ ] watcher (notify + debouncer, 무시 목록, echo 플래그, 배치 emit)
- [ ] Monaco 셋업 (0.56 경로·worker 배선·CSP) + 모델 레지스트리 + viewState 복원
- [ ] 파일 트리 (트리 구조 결정안 적용, lazy load, git 상태색, context menu, typeahead) — `explorer-sidebar.md`
- [ ] 키바인딩 3계층 (Monaco 보강 + 앱 전역 keymap 모듈 + WebView 기본 동작 차단)
- [ ] Monaco 테마 파생 (defineTheme hex 변환) + 내장 TS worker (tsconfig 주입)
- [ ] 대형 파일 4단계 정책 (2/20/50MB — 상수 Rust·TS 동기)
- [ ] 외부 변경·충돌 처리 (리로드/충돌 배너)

## Phase 3 — 터미널

- [ ] pty 인프라 (spawn/write/resize/kill, Raw Channel + 4ms 배칭) — `terminal.md`
- [ ] xterm 위젯 (addon 셋·WebGL contextLoss·ResizeObserver·fit)
- [ ] flow control (watermark + pause) + ring buffer 복원(`pty_attach`)
- [ ] 기본 셸·셸 프로필 열거 + 프로젝트 기본 터미널 탭
- [ ] 폰트 크기 단축키 + 설정 연동
- [ ] 파일 경로 LinkProvider + cmd+click → 탭 열기 (`resolve_terminal_path`)
- [ ] 터미널 검색·복사/붙여넣기·테마(ANSI) 연동
- [ ] 탭 닫기 확인·exit 표시·앱 종료 시 전 세션 정리

## Phase 4 — Git

- [ ] git capability (discover·status 정규화·watcher 연동 캐시) — `git.md`
- [ ] SCM 패널 (그룹·상태 문자·hover 액션·context menu·discard 휴지통)
- [ ] stage/unstage/discard (git2 — `force()`·`Index::write()` 함정 체크)
- [ ] commit/push/pull (git CLI 하이브리드 — ADR-0006, 에러 UI)
- [ ] diff 뷰 (Monaco DiffEditor, side-by-side/inline 토글, preview 탭 재사용)
- [ ] 에디터 gutter (hunk → decorations) + 인라인 blame (injected text + GitLens 포맷)
- [ ] 커밋 그래프 (revwalk 페이지네이션 + 프론트 레인 배치 + 12색 + ref 라벨)
- [ ] remote·ahead/behind·Sync 헤더

## Phase 5 — LSP

- [ ] lsp 도메인 (spawn/프레이밍/supervisor/Channel) — `lsp.md`
- [ ] 서버 감지 (로컬 우선 하이브리드 + 설치 안내 UI)
- [ ] 클라이언트 (ADR-0007 결정 3 확정안) — 어댑터: 진단→completion→hover→definition→references→rename→formatting→signatureHelp→inlayHints→documentSymbol 순
- [ ] vtsls (didChangeConfiguration 설정) + 내장 TS worker 핸드오프
- [ ] rust-analyzer (initializationOptions·cargo metadata 루트·rust-src 감지)
- [ ] basedpyright + ruff (venv 감지·이중 세션) / marksman (루트 마커 안내)
- [ ] positionEncoding·pull/push 진단 정책·세션 상태 UI

## Phase 6 — 에이전트 연동

- [ ] `taide` CLI + `--wait` 마커 방식 + single-instance 파일 열기 — `agent-integration.md`
- [ ] 탭 닫힘 → 마커 해제 + 안전망 (실패 즉시 삭제·종료 시 정리·힌트 UI)
- [ ] 에이전트 감지 (process_group_leader / sysinfo) → 사이드바·탭 아이콘
- [ ] CLI 설치 UX + EDITOR 설정 안내
- [ ] Claude Code ctrl+g 실기기 왕복 검증 (성공 기준 4 — tsc 로 못 잡음, 수동 QA)

## Phase 7 — 검색·설정·플러그인

- [ ] 전역 검색 (스트리밍·취소·결과 트리) — `explorer-sidebar.md` §3
- [ ] 설정 UI (테마·셸·키맵 오버라이드·LSP 상태·에이전트 목록)
- [ ] 플러그인 로더 (매니페스트 검증·LSP 기여·동의 UI) — `plugins.md`
- [ ] 커맨드 팔레트 (`⌘⇧P`) + 파일 퀵오픈 (`⌘P`)

## Phase 8 — 배포·확장 (2차) — **게이트: Phase 0~7 이 로컬에서 전부 테스트·확인 완료된 후에만 진행 (사용자 합의)**

- [ ] macOS 서명·공증, 업데이터 / Windows·Linux 빌드 검증 (NFR-6)
- [ ] IDE MCP 서버 (Claude Code 자동 연결 — agent-integration §3)
- [ ] OSC 133 명령 블록 UX, hooks/statusline 브리지
- [ ] git 2차 (브랜치 UI·커밋 상세·stash UI·충돌 해결)
- [ ] remote-control capability (프로젝트 추상화 위에 — 별도 기획)

## 상시 (전 Phase 공통)

- [ ] 각 기능의 "수명주기" 절 구현 확인 (구독 해제·dispose·프로세스 정리)
- [ ] 누수 회귀 수동 QA: 탭 50회 개폐 heap 비교·`yes` 10초 입력 반응·대형 리포 열기
  (`docs/quality-assurance/` 체크리스트)
- [ ] UI 는 라이트·다크 실렌더 확인 후 완료 보고 (개인 guideline §11)
