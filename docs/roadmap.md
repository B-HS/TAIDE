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

## Phase 7.5 — 실사용 피드백 반영 (Phase 8 전 필수)

> Phase 0~7 구현 후 사용자가 실기기에서 확인하며 제기한 18건.
> 결정·근거는 `docs/acknowledge/2026-08-06-phase75-decisions.md`.
> **버그(2·4·17)는 추측으로 고치지 말고 실제 렌더 값을 확인한 뒤 근본 원인을 잡는다.**

### 7.5-A 버그 수정 (먼저 — 나머지 작업의 전제)

- [x] codeview 에 테마 미반영 (2번) — `ThemeProvider` 에서 `applyMonacoTheme` 배선
- [x] 기동 시 흰 화면 깜빡임 (4번) — `index.html` 첫 페인트 바탕색 + `useLayoutEffect` 로 변수 주입
- [x] 타이틀바가 다크/라이트를 안 따름 (17번) — `setBackgroundColor` + capability 추가

### 7.5-B 기술 검토 — **완료 (2026-08-06)**

- [x] remote-control 실현 가능성 — `docs/research/remote-control.md` (374줄),
      결론 요약 `features/remote-control.md`
      → **잠자기 중 동작 불가**(뚜껑 닫힘·배터리). **잠금은 가능**.
      "화면 그대로"는 **픽셀 스트리밍이 아니라 상태 미러링**(ADR-0004 가 이점).
      **Phase 7.5 에서는 구현 안 함** — 사용자와 범위 재합의 필요.
- [x] 터미널 라이브러리 재평가 — `docs/research/terminal-reevaluation.md` (578줄),
      결론 요약 `features/terminal.md` §12
      → **xterm 유지**(대체재 없음, VS Code 도 동일 라이브러리 사용).
      잔상 원인은 **우리 코드 3건**으로 특정 → 아래 7.5-G

### 7.5-G 터미널 잔상 수정 (검토 결과 — 원인 특정 완료)

- [x] 수정 전 **원인 판별 체크리스트** 실행 — `terminal.md` §12.5
      → `tput cols`=80 으로 **B 확정**, 잔상 + 한글 한 템포 지연으로 **A 지목**
- [x] P0-1. reader 에 **타이머 flush** 도입 (`OutputBatch` + 5ms 틱 flusher 스레드) — `infra/pty.rs`
- [x] P0-2. **실측 cols/rows 로 spawn** — `TerminalView.onReady` → `TerminalSession` spawn + 후속 resize
- [x] P1. `pty_write`/`resize`/`set_paused` 를 전역 `begin_mutation` 락에서 **분리**
- [x] **한글(CJK) 입력 불가** — WKWebView 가 조합 이벤트를 발생시키지 않는 것이 근본 원인.
      `resolveImeInput` 어댑터로 해결. 정본 `docs/bug/2026-08-06-wkwebview-ime-composition.md`
- [x] 재시작 후 터미널 빈 화면 — 죽은 sessionId 재사용. 살아있는 세션 확인 후 없으면 새로 spawn
- [ ] P0-3. 탭 전환 시 unmount 대신 **숨김 유지**(replay 경로 제거) — `pane-node-view.tsx`
      (A·B 수정으로 잔상이 사라져 보류. 탭 전환 직후 재현되면 착수)
- [ ] Tauri/wry/tao 상위 버전에서 조합 이벤트가 고쳐지면 IME 어댑터 제거

### 7.5-H 다국어 (i18n) — en/ko/ja + 사용자 언어팩

> 결정: `docs/acknowledge/2026-08-06-i18n-and-session-findings.md` §1. 7.5-G 다음 순서.

- [x] `react-i18next` 도입 + 내장 en/ko/ja 번들 — 번들은 Rust 소유, 런타임 `addResourceBundle` 주입
- [x] `settings` 도메인에 `language` 필드(`system|en|ko|ja|커스텀`) — Rust 소유·영속화
- [x] Rust `locale_list()` / `locale_get(id)` / `locale_get_current(systemLanguage)` —
      `{app_data}/locales/*.json` 열거·로드 + `extends` 부분 오버라이드 + 시스템 언어 해석
- [x] 설정 UI 언어 선택(자체 컴포넌트) + 하드코딩 한국어 문자열 전량 치환 (잔여 0건)
- [ ] locales 디렉토리 watcher 핫리로드 (테마와 동일 구조 — 미착수)

### 7.5-C UI 일관성 · 확장성

- [x] `⌘W` 가 앱을 종료하는 문제 — 커스텀 앱 메뉴로 Close Window 제거(Edit 메뉴는 유지) + `close-tab` 배선
- [x] 파일 트리 툴바 (19번) — 새 파일·새 폴더·새로고침·모두 접기
- [x] 탭 context menu 항목 확장 (9번) — 닫기류 5·복사류 2·탐색류 2·분할 4. 구현 불가 항목은 §7.5-C 미구현 표 참조
- [x] 커맨드 레지스트리 + `>` 접두 모드 (10번) — `shared/lib/command-registry.ts`, 테스트 21건
- [x] 설정 화면 재구성 — TOC + `max-w` 제거 + Card + 자체 Switch (13번)
- [x] toast 위치 9분할 설정 (11번) — sonner 6종 + 중간 행 3종 CSS 보정. Toaster 테마 추종도 함께 수정
- [x] 리사이저 두께 설정 + 히트영역 분리 (12번) — 히트 8px 고정 / 시각 두께는 설정값
- [ ] 탭 열기 규칙 — `⌘+클릭` 분할, Move/Copy into New Window (7번). 멀티 윈도우 미지원이라 보류

#### 7.5-C 미구현 (백엔드/기능 부재 — 가짜 UI 만들지 않음)

| 항목 | 필요한 것 |
|------|----------|
| File History | 파일 단위 git log 필터 (현재 프로젝트 전체 로그만) |
| Find File References | 탭 우클릭에서 LSP references 를 트리거하는 경로 |
| Move/Copy into New Window | 멀티 윈도우 자체 (capabilities 가 단일 `main`) |
| Reveal in Explorer View | 트리 확장·스크롤 상태 연동 |
| Keep Open | preview 해제 mutation (pin 과 다른 개념) |
| restart window | Tauri 커맨드 신설 필요 |

### 7.5-D 표시·꾸밈

- [x] 시스템 폰트 열거 + 폰트 선택 (1번) — Rust `font_list()`(fontdb) + 검색형 FontPicker + 폴백 체인
- [x] 테마 편집기 + 저장 (14번) — `theme_save`/`theme_delete` + 자체 HSV 색 피커 + 라이브 프리뷰
- [x] 파일 아이콘 세트 (6번) — material-icon-theme **분류만 참조**, lucide 매핑. SVG 도입은 별도 결정
- [x] 타이틀바 중앙 정보 + footer(폰트 크기 컨트롤) (15·18번)
- [x] `followSystemTheme` 백엔드 no-op 수정 — `theme_get_current(systemTheme)` + OS 테마 변경 구독
- [x] pty `LANG`/`LC_CTYPE` 누락 수정 (Finder 실행 시 non-UTF-8 방지)
- [ ] 테마 내보내기/가져오기 — `@tauri-apps/plugin-fs` 미설치라 보류
- [ ] 파일 타입 전용 색 토큰(`fileIcon.*`) — 현재 8색 토큰 재사용 중. 테마 스키마 확장 필요
- [ ] footer 커서 위치 표시 — 에디터가 값을 올려주는 구조 필요

### 7.5-E 미리보기

- [x] 이미지·비디오·오디오·PDF·HTML (3번) — 비디오·오디오는 asset 프로토콜 스트리밍,
      HTML 은 `allow-scripts` 없는 sandbox iframe
- [x] xlsx (SheetJS) · HWP/HWPX (`@rhwp/core`) (3번)
- [x] pptx — 개요 수준 + "레이아웃이 원본과 다를 수 있습니다" 안내
- [ ] pptx LibreOffice 감지 폴백 — 외부 바이너리 감지 Rust 커맨드 필요 (`soffice_detect`/`soffice_convert_to_pdf`)

> **설계 편차**: `preview.md` §1 은 `TabKind::Preview` 신설을 제안했으나 **만들지 않았다.**
> 레이아웃 스키마 변경은 영속 데이터 마이그레이션을 부르는데, "어떤 렌더러로 그릴지"는
> 도메인 상태가 아니라 view 판단이다. 기존 `TabKind::File` + `pane-node-view` 확장자 분기로 처리했고
> preview/pin 규칙도 그대로 유지된다.

### 7.5-F remote-control — **Phase 7.5 에서 구현하지 않음** (검토 결과)

- [x] 검토 완료 → 잠자기 중 동작이 OS 정책상 불가. 사용자 기대와 다르므로 **범위 재합의 필요**.
- [ ] **지금 지켜야 할 것 1건**: IPC 계약에 "전송 수단 = Tauri IPC" 가정을 **넣지 않는다**.
      (`features/remote-control.md` §2 — 소급하면 entities 전 도메인을 다시 만져야 함)
- [ ] 구현 자체는 Phase 8 이후 또는 별도 기획

## Phase 8 — 배포·확장 (2차) — **게이트: Phase 0~7.5 가 로컬에서 전부 테스트·확인 완료된 후에만 진행 (사용자 합의)**

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
