# PROCESS — TAIDE 작업 상태

> 기준 문서: `~/.claude/convention/*.md`(전 컨벤션), `docs/acknowledge/`(결정), 이 문서(체크리스트).
> 구현 순서 정본은 `docs/roadmap.md`, 버전 정본은 `docs/tech-stack.md`, API 정본은 `docs/research/*.md`.

## 완료: 문서화 (2026-08-06)

- [x] a~o 전건 완료 — PRD·아키텍처·tech-stack·ADR 0001~0011·IPC 계약·data-model·theme-system·
      features 9건·roadmap·정합성 검수 (커밋 `718d64d`, `d1e8f80`)

## 진행 중: Phase 0 — 스캐폴딩 (2026-08-06)

- [x] 0-1. Tauri 2 + Vite 8 + React 19 프로젝트 생성 (bun, `tech-stack.md` 버전 고정)
- [x] 0-2. React Compiler 설정 + **적용 실측 검증** (`useMemoCache` ×7 · `compiler-runtime` ×3 확인)
- [x] 0-3. Tailwind 4.3 + shadcn 배선 (`components.json` → `@shared/ui`, button 으로 동작 확인)
- [x] 0-4. FSD 디렉토리 골격 + path alias (`@app`/`@widgets`/`@features`/`@entities`/`@shared`)
- [x] 0-5. Rust 골격 (`state.rs`/`error.rs`/`events.rs`/`domain/app`/`infra/persist`)
- [x] 0-6. tauri-specta 배선 + `bindings.ts` 생성 파이프라인 (`cargo test` 로 재생성 — 결정론적)
- [x] 0-7. `useTauriEvent` 훅, QueryClient 기본값(`networkMode:'always'` 등), 에러 토스트(sonner)
- [x] 0-8. 검증 스크립트 `bun run verify` (typecheck→lint→format→test→cargo fmt→clippy→cargo test)
- [x] 0-9. capabilities 최소 구성 + CSP (Monaco 요구 `style-src 'unsafe-inline'`·`worker-src blob:` 반영)
- [x] 0-10. **FSD 의존 방향 ESLint 가드** (로드맵 외 추가 — 실제 위반 1건을 잡아 도입)
- [x] 0-11. 실기기 기동 확인 — 사용자가 창 렌더 확인(`TAIDE / macos v0.1.0` 표시).
      Rust→Vite→React→specta bindings→invoke→응답 전 구간 동작 확정.

### Phase 0 에서 확정한 것 (문서 기재값과 다른 부분)

| 항목 | 문서/템플릿 값 | 실제 채택 | 사유 |
|------|---------------|----------|------|
| TypeScript | "최신" | **5.9.3** | typescript-eslint 8.66 peer 가 `<6.1.0` — TS7 이면 `.tsx` 린트가 통째로 죽음 |
| `build.minify` | `'esbuild'` (Tauri 템플릿) | **`'oxc'`** | Vite 8 은 esbuild 를 번들하지 않음 (빌드 실패로 확인) |
| react-hooks flat config | `configs['recommended-latest']` | **`configs.flat['recommended-latest']`** | 전자는 legacy 형식이라 ESLint 10 flat config 에서 거부 |
| React Compiler 실행기 | — | **Babel 유지** | oxc 네이티브 포트는 Rolldown 이 되돌림(바이너리 +5.1MB·성능). 실측 비용 +0.1s/90모듈 |

## 진행 중: Phase 1 — 셸(프로젝트·레이아웃·탭·테마·영속화)

### Rust 코어

- [x] 1-1. 공유 계약 작성 (main 직접): `ids.rs`·`paths.rs`·`state.rs`·`events.rs` +
      각 도메인 `types.rs`(layout/project/theme/settings) — **IPC 계약이라 에이전트 수정 금지**
- [x] 1-2. layout 도메인 — PaneNode 트리 변형·정규화·preview/pin·closed 스택 (테스트 15건)
- [x] 1-3. project 도메인 — open/close/activate/reorder·세션 복원·파손 파일 bak 정책 (테스트 8건)
- [x] 1-4. theme + settings 도메인 — 내장 dark/light 전량 토큰·`$` palette 해석·부분 patch
      (테스트 12건 — 토큰 전량 존재·ANSI 16색 존재를 CI 로 검증)
- [x] 1-5. lib.rs 배선 (main 직접) — 커맨드 21종 등록·이벤트 발행·세션/레이아웃 복원·bindings 재생성.
      Rust 총 44 테스트 통과, clippy `-D warnings` 무경고.

#### 1-5 에서 함께 처리할 것 (통합 시 내가 판단한 사항)

- **동시 mutation lost-update 방어**: 현재 mutation command 는 `read().clone()` → service → `write()` 대입
  패턴이라(락 안 IO 금지 규율 준수) 두 mutation 이 겹치면 갱신이 유실된다.
  → `AppState` 에 mutation 직렬화 guard(`tokio::sync::Mutex<()>`)를 두고 mutation command 가 획득한다.
- **영속화 시점 — 스펙 대비 의도적 편차**: `data-model.md` §4 는 debounce 저장이지만,
  project mutation(open/close/activate/reorder)은 **빈도가 낮고 유실 비용이 커서 동기 저장을 유지**한다.
  debounce 는 빈도가 높은 **layout·dirty 버퍼에만** 적용한다(Phase 1-5·Phase 2).

### 프론트엔드

- [x] 1-6. shadcn 프리미티브 (context-menu/tooltip/scroll-area/dialog/alert-dialog/separator/dropdown-menu)
      — **resizable 은 제외**: shadcn 판이 react-resizable-panels 구 API(`PanelGroup`) 기반이라 v4 와 불일치
- [x] 1-7. 테마 토큰→CSS 변수 변환기 (`shared/lib/theme-variables.ts`, 테스트 4건)
- [x] 1-8. entities (project/layout/theme/settings) — ipc + queryOptions + `IpcSyncProvider` 이벤트 invalidate.
      `useTauriEvent` 는 **생성 바인딩의 이벤트 객체를 받도록 시그니처 변경**(문자열 이름 대신) —
      ADR-0011 타입 안전성을 구독까지 확장. 핸들러는 React 19.2 `useEffectEvent` 로 고정해
      매 렌더 재구독을 막았다.
- [x] 1-9. 앱 사이드바 위젯 (프로젝트 목록·툴팁·context menu·세로 DND·폴더 열기)
- [x] 1-10. 탭 바 + pane 트리 렌더 (v4 `Group`/`Separator`, `onLayoutChanged` + `isUserInteraction` 로 드래그 종료 시 1회만 IPC)
- [x] 1-11. 탭 DND (단일 DndContext + DragOverlay + pointerWithin, 5분할 드롭존, pinned 구역 클램프)
- [x] 1-12. 테마 프로바이더 (CSS 변수 주입 + 테마 로드 완료 후 `show()` — FOUC 방지)
- [x] 1-13. 빈 상태(웰컴) 화면
- [x] 1-14a. 복원 E2E **통합 테스트** (`src-tauri/tests/session_restore.rs`, 4건) —
      재시작 시 프로젝트·탭 구성·스플릿 구조·focused pane 복원 / root 부재 시 `root_missing` /
      활성 프로젝트 닫기 시 승계 / 레이아웃 파일 부재 시 기본 레이아웃.
- [ ] 1-14b. 실기기 복원 확인 (사용자 피드백 루프)

#### 통합 테스트가 잡아낸 실제 버그 2건 (수정 완료)

1. **`project_open` 이 `active_project` 를 설정하지 않았다** — 프로젝트를 열어도 활성 프로젝트가
   `None` 이라 사이드바에는 뜨지만 아무것도 선택되지 않은 상태가 됐다. 신규/기존(중복 열기) 양쪽 모두
   활성화하도록 수정 (layout-shell.md §3 "중복 열기는 기존 프로젝트를 활성화한다").
2. **`close_project` 가 활성 프로젝트를 닫을 때 `None` 으로 비웠다** — 다른 프로젝트가 남아 있어도
   빈 상태가 됐다. 남은 프로젝트로 승계하고, 마지막 하나를 닫을 때만 `None` 이 되도록 수정
   (layout-shell.md §6 "마지막 프로젝트를 닫으면 빈 상태 화면").

두 건 다 도메인 단위 테스트는 통과하던 상태였다 — 도메인 경계를 넘는 통합 테스트에서만 드러났다.

3. **스플릿 `sizes` 단위 불일치 (Rust ↔ react-resizable-panels)** — Rust 는 스플릿을 `0.5`(분수)로
   seed 하는데 v4 `Layout` 은 `0..100` 이고 **숫자 `defaultSize` 는 px 로 해석**된다.
   그대로면 스플릿이 0.5px 로 그려지고 첫 드래그 후엔 100 스케일로 덮여 값이 뒤섞인다.
   → 퍼센트(0..100)로 통일 + 프론트는 `` `${size}%` `` 문자열 전달 + `sizes` 합이 100인지
   회귀 테스트 추가. 양쪽 단위 테스트로는 잡히지 않는 종류의 버그.

## 진행 중: Phase 2 — 에디터·파일 (2026-08-06)

- [x] 2-1. 공유 계약 (main 직접): `constants.rs`(무시목록·크기 4단계 임계값)·
      `domain/file/types.rs`·`domain/tree/types.rs`·`fs:changed`/`tree:changed` 이벤트
- [x] 2-2. file 도메인 + watcher 인프라 (테스트 22건) — 크기 4단계·언어 감지·경로 루트 검증·dirty 미러
- [x] 2-3. tree 도메인 (테스트 8건) — flatten·lazy load·페이지네이션·reveal·펼침 직렬화
- [x] 2-4. Monaco 통합 — worker 5종 청크 분리 **빌드 산출물로 실측 확인**, 모델 레지스트리, 테마 파생(monaco 비의존 순수함수+테스트), 순수 code-editor
- [x] 2-5. 파일 트리 위젯(가상스크롤·typeahead) + 전역 키맵 모듈(캡처 단계)
- [x] 2-6. lib.rs 배선 (main 직접) — 커맨드 총 37종 등록·프로젝트별 watcher 부착/해제·TreeStore 등록·bindings 재생성
- [x] 2-7a. 컨테이너 위젯 배선 (main 직접) — `explorer-container`(트리↔IPC), `editor-pane`(파일↔Monaco↔dirty 미러), 셸에 사이드바 스플릿 배치
- [x] 2-7b. `fs:changed` 수신 배선 — 변경 파일 쿼리 무효화 + **변경된 부모 디렉토리만** `tree_refresh`
      (explorer-sidebar §2.3 "루트 재스캔 금지"). 충돌 배너(dirty 상태 외부 변경)는 Phase 2 잔여로 남김
- [ ] 2-8. 실기기 확인 (사용자 피드백 루프)


## 진행 중: Phase 3 — 터미널 (2026-08-06)

- [x] 3-1. 공유 계약 (main 직접): `domain/terminal/types.rs`(ShellProfile·PtySpawnOptions·TerminalSession·
      배칭/링버퍼 상수) + `terminal:exited`/`terminal:cwd-changed` 이벤트
- [x] 3-2. pty 인프라 + terminal 도메인 (테스트 12건) — portable-pty 0.9 함정 대응
      (slave drop·take_writer 1회·clone_killer 분리·default_prog arg 금지), Condvar flow control,
      링버퍼, `/etc/shells` 파싱, `path:line:col` 해석
- [x] 3-3. xterm 위젯 (flow control 순수함수 8건·링크 매칭 13건 테스트) — WebGL contextLoss 재로드,
      ResizeObserver+rAF+NaN 가드, 바이트 그대로 `term.write`
- [x] 3-4. lib.rs 배선 (main 직접) — TerminalStore 등록, 앱 종료 시 전 세션 kill(`kill_all` 추가),
      **raw Channel 커맨드 분리 라우팅**(아래 참조)
- [x] 3-5. 컨테이너 배선 (main 직접) — `terminal-session`(spawn/attach/theme/flow control),
      pane 트리에 터미널 탭 마운트, `xterm-theme.ts`(테마 토큰→ANSI ITheme)
- [ ] 3-6. 실기기 확인 (사용자 피드백 루프)

### raw 바이트 Channel 은 tauri-specta 를 통과할 수 없다 (구조적 제약 — 우회 설계)

터미널 출력은 성능상 `InvokeResponseBody::Raw` 로 보내야 하는데(`Vec<u8>` 을 그대로 send 하면
JSON 숫자 배열이 된다 — research/xterm-pty.md), 이 타입은 tauri-specta 로 등록할 수 없다:

- `specta::Type` 을 `InvokeResponseBody` 에 구현 → **orphan rule 위반**(둘 다 외부 크레이트)
- 자체 newtype 에 `IpcResponse` 구현 → **coherence 위반**: tauri 에 `impl<T: Serialize> IpcResponse for T`
  블랭킷 impl 이 있어 특정 impl 을 추가할 수 없다

→ **채택**: `pty_spawn`·`pty_attach` 두 개만 `tauri::generate_handler!` 로 별도 등록하고,
`invoke_handler` 에서 **커맨드 이름으로 분기**해 specta 핸들러와 raw 핸들러를 나눈다.
이 둘의 TS 래퍼는 `entities/terminal/terminal.ipc.ts` 에 **수동 작성**(bindings 생성 대상 아님).
파라미터 타입 `PtySpawnOptions` 는 드리프트를 막기 위해 **`pty_default_options` query 를 새로 추가**해
specta 가 계속 생성하도록 했다(프론트가 셸·cwd 기본값을 추측하지 않아도 되므로 기능적으로도 필요).

## 진행 중: Phase 4 — Git (2026-08-06)

- [x] 4-1. 공유 계약 (main 직접): `domain/git/types.rs`(StatusRow 정규화·GutterHunk·BlameLine·
      LogEntry·DiffSides·CommitOptions) + `git:status-changed`/`git:refs-changed` 이벤트
- [x] 4-2. git 도메인 (테스트 9건, 실제 temp 저장소 기반) — git2 읽기/stage + git CLI commit/push/pull
- [x] 4-3. SCM 패널 · 커밋 그래프(레인 배치 순수함수 5건 테스트) · diff 뷰(순수 컴포넌트)
- [x] 4-4a. lib.rs 배선 (커맨드 16종·GitStore·이벤트) + entities/git + GitPanelContainer 실배선
- [ ] 4-4b. gutter/blame 을 에디터에 연결 (데이터 API 는 완성, Monaco 데코레이션 주입 남음)
- [ ] 4-5. 실기기 확인

## 대기: Phase 5~7 공유 계약 (main 직접, 미리 확정)

- [x] `domain/lsp/types.rs` (LanguageServerSpec·LspServerDetection·LspSessionInfo·상태 enum)
      + `lsp:session-status-changed` 이벤트
- [x] `domain/search/types.rs` (SearchQuery·SearchMatch·상한 상수)
- [x] `domain/plugin/types.rs` (매니페스트 스키마 — plugins.md §2 정본)
- [x] `domain/agent/types.rs` (DetectedAgent·폴링 주기·감지 대상 목록) + `agent:state-changed` 이벤트

계약을 먼저 고정해 두면 각 Phase 의 병렬 에이전트가 서로의 타입을 기다리지 않아도 된다
(Phase 1~3 에서 검증된 방식 — 에이전트는 service/commands 만 채우고 배선은 메인이 한다).


## 진행 중: Phase 5 — LSP (2026-08-06)

- [x] 5-1. 공유 계약 (main 직접): `domain/lsp/types.rs` + `lsp:session-status-changed` 이벤트
- [x] 5-2. lsp 도메인 + JSON-RPC 프레이밍 인프라 — **프레이밍 6케이스 테스트**
      (헤더 부분수신·바디 부분수신·한 버퍼 다중 메시지·헤더 대소문자·Content-Type 혼재·왕복),
      루트 탐지 13건(Deno 회피·Cargo workspace 우선·rust-src 외부경로 등)
- [x] 5-3. 자체 경량 LSP 클라이언트 + Monaco 어댑터 10종 (completion/hover/definition/references/
      rename/formatting/signatureHelp/inlayHints/documentSymbol/diagnostics)
- [x] 5-4. lib.rs 배선 (커맨드 6종·LspStore·이벤트) + entities/lsp
- [ ] 5-5. 에디터에 LSP 클라이언트 연결 (어댑터 등록·didOpen/didChange 배선) — **Phase 5 잔여**
- [ ] 5-6. 실기기 확인 (사용자 피드백 루프)

### Phase 4·5 배선에서 고친 것

- **specta BigInt 재발**: `BlameLine.time_unix`/`LogEntry.time_unix` 가 `i64` 라 바인딩 생성 실패.
  `f64` 로 변경(JS number 가 어차피 f64, i32 로 낮추면 2038 문제). `Eq` derive 도 함께 제거.
- **위젯의 미러 타입 ↔ 생성 타입 불일치**: bindings 미생성 시점에 에이전트가 손으로 만든
  `GitStatusRow`/`GraphLogEntry` 가 실제 생성 타입과 어긋남(`origPath` 의 `undefined`,
  `timeUnix` 의 `null`). 미러 타입을 **생성 타입 재export 로 교체**하고 nullable 을 호출부에서 좁혔다.
- **`PluginContributions` 수동 Default** → clippy `derivable_impls` 지적, derive 로 교체.


## Phase 5 종료 시점 검증 결과 (2026-08-06)

전체 게이트 통과: `typecheck` / `eslint`(에러 0) / `prettier` / `bun test 110 pass` /
`cargo fmt` / `cargo clippy -D warnings` / `cargo test 105 + 5(통합) pass`.

남은 경고 1건은 `file-tree.tsx` 의 `@tanstack/react-virtual` React Compiler 비호환
(컴파일러가 안전하게 bail 한 것 — QA §4.2 에 기록, 조치 불필요).

### Phase 4·5 배선에서 추가로 고친 것

- **LSP 프로세스 누수**: `LspStore::kill_all()` 이 구현돼 있었지만 앱 종료 경로에서 호출되지 않아
  언어 서버 자식 프로세스가 잔존할 수 있었다. `RunEvent::Exit` 핸들러에 `TerminalStore` 와 함께 배선
  (lsp.md §5 · architecture §6.3 "자식 프로세스 종료까지 보장").

### Phase 5 이후로 넘긴 것 (에이전트 보고 + 내 확인)

- **에디터↔LSP 실배선**: 클라이언트·어댑터 10종·Rust 세션 관리는 완성됐고 테스트도 있지만,
  Monaco 에디터에 provider 를 실제 등록하고 didOpen/didChange 를 흘리는 배선은 남았다.
- **세션 공유(`didChangeWorkspaceFolders`)**: 현재는 `lsp_spawn` 마다 새 프로세스. vtsls/basedpyright/
  marksman 의 폴더 추가 방식(ADR-0007)은 미구현.
- **shutdown 핸드셰이크**: 응답 상관 없이 고정 2초 타임아웃 2단계(shutdown→exit→kill).
  요청/응답 id 추적이 view 책임이라는 원칙 때문에 Rust 단독으로는 응답을 못 기다린다.
- **git gutter/blame 의 Monaco 데코레이션 주입**: 데이터 API(`git_gutter`/`git_blame_range`)는 완성,
  에디터 표시는 남음.
- **diff 탭 마운트**: `DiffView` 순수 컴포넌트는 완성, `TabKind::Diff` 를 pane 에 렌더하는 배선은 남음.
- **git watcher 연동**: 외부(터미널 등)에서 워킹트리가 바뀔 때 status 자동 무효화 미배선.
  현재는 앱 내 mutation 직후에만 갱신된다.

## 완료: Phase 6 — 에이전트 연동 (2026-08-06)

- [x] 6-1. 공유 계약 (main 직접): `domain/agent/types.rs` + `agent:state-changed`/`agent:external-open`
- [x] 6-2. 에이전트 감지 — `process_group_leader()` → pid → `ps -o comm=,args=` 해석.
      **판정 로직 순수 함수 + 테스트 13건**(`claude` 직접 / `node .../claude` 런타임 경유 /
      Linux comm 15자 잘림 / 무관 프로세스)
- [x] 6-3. wait 마커 해제 — **경로 검증 순수 함수 + 테스트**(temp_dir 직속 + `taide-wait-` prefix,
      `../` 탈출 차단). 임의 경로 삭제를 구조적으로 막는다
- [x] 6-4. CLI 설치 상태 조회 + 사이드바 에이전트 배지 실데이터 연결
- [ ] 6-5. `taide` CLI 바이너리(`crates/taide-cli`) — **미착수**(별도 크레이트/workspace 구성 필요)
- [ ] 6-6. Windows 감지(sysinfo 프로세스 트리) — **스텁**(의존성 추가 필요)
- [ ] 6-7. IDE MCP 서버 / hooks·statusline 브리지 — 문서상 2·3차

## 완료: Phase 7 — 검색·설정·플러그인·팔레트 (2026-08-06)

- [x] 7-1. 전역 검색 — 자체 병렬 스캔(외부 크레이트 없이), 무시목록 공유, 바이너리 스킵,
      `AtomicBool` 취소, Channel 스트리밍. **매칭·프리뷰·glob 순수 함수 + 테스트**
- [x] 7-2. 설정 UI — 테마/폰트/셸/LSP 감지 상태/플러그인 목록
- [x] 7-3. 플러그인 로더 — 매니페스트 검증, **경로 탈출(`..`) 차단 순수 함수 + 테스트**,
      실패한 플러그인만 비활성(나머지는 계속 로드), LSP 기여는 자동 기동 안 함(ADR-0010)
- [x] 7-4. 커맨드 팔레트(`⌘⇧P`) + 파일 퀵오픈(`⌘P`) — **fuzzy 매칭 순수 함수 + 테스트**
- [x] 7-5. lib.rs 배선 (커맨드 총 60종·SearchStore/PluginStore/AgentStore·이벤트) + entities 4종
- [ ] 7-6. regex 검색 — **미지원**(regex 크레이트 미추가, 명시적 에러 반환)

## 완료: Phase 4·5 잔여 배선 (2026-08-06)

- [x] diff 탭 마운트(`diff-pane`), `⌥\` side-by-side/inline 토글
- [x] git gutter 데코레이션(`createDecorationsCollection`) + 인라인 blame(injected text,
      300ms debounce, **포맷 순수 함수 + 테스트 15건**)
- [x] LSP 클라이언트 ↔ 에디터 실연결 — 첫 didOpen 시점 lazy 기동, 어댑터 10종 등록,
      `(projectId, serverId)` ref-count 세션 공유, unmount 시 dispose, 대형 파일 스킵,
      서버 미설치 시 조용히 스킵
- [x] `git:status-changed`/`git:refs-changed` 구독 → gutter/diff 자동 갱신

## Phase 7 종료 검증 (2026-08-06)

전체 게이트 통과: `typecheck` / `eslint`(에러 0) / `prettier` / `bun test 135 pass` /
`cargo fmt` / `cargo clippy -D warnings` / `cargo test 139 + 5(통합) pass`.
남은 경고 1건은 `@tanstack/react-virtual` 의 React Compiler 비호환(QA §4.2 기록, 조치 불필요).

## 진행 중: Phase 8 전 잔여 정리 (2026-08-06)

메인이 먼저 처리한 선행 작업:
- [x] **Cargo workspace 구성** — 루트 `Cargo.toml` 에 `members = ["src-tauri", "crates/taide-cli"]`.
      release 프로파일을 워크스페이스 루트로 이동(멤버에 두면 무시된다).
- [x] 의존성 추가 — `sysinfo 0.39`(Windows 감지) · `regex 1`(정규식 검색) · `trash 5`(휴지통 discard)
- [x] 검증 스크립트를 워크스페이스 기준으로 변경
      (`cargo fmt --all` / `clippy --workspace` / `test --workspace`)

에이전트 병렬 작업 4건:
- [ ] R-1. `taide` CLI 바이너리 — `--wait` 마커 방식, 조기 exit 금지(R1~R9), 앱 측 마커 등록/정리
- [ ] R-2. Rust 갭 — Windows 에이전트 감지(sysinfo) · regex 검색 · untracked discard 휴지통 ·
      git watcher 무효화 분류(`.git/index`→status, `refs/**`→refs, `objects/**` 무시)
- [ ] R-3. LSP 갭 — `find_root` 를 실제 사용(`lsp_resolve_root` 커맨드) · ruff 병행 세션
      (진단 owner 를 서버별 분리해 덮어쓰기 방지) · `didChangeWorkspaceFolders` 세션 공유
- [ ] R-4. 프론트 갭 — 외부 변경 충돌 배너(dirty 시) · 커밋 그래프 가상 스크롤 ·
      blame "You" 치환(`git_current_user` 추가) · 터미널 sessionId 를 layout 탭에 영속

### 메인이 배선할 것 (에이전트가 못 건드리는 `lib.rs`)

- 에이전트 감지 **폴링 태스크** — `TerminalStore::foreground_pids` → 감지 → `AgentStore::diff` →
  변경 시에만 `AgentStateChanged` emit (unix 1s / windows 2s)
- single-instance 콜백 → `parse_cli_payload` → `AgentExternalOpen` emit
- 앱 종료 시 미해결 wait 마커 일괄 삭제
- git watcher 콜백에 `classify_git_change` 연결

## 완료: Phase 8 전 잔여 전량 (2026-08-06)

- [x] R-1. `taide` CLI (`crates/taide-cli`, 테스트 9건) — `--wait` 마커 + 300ms 폴링 블록,
      절대경로 정규화, detach spawn, **stdout 무오염·exit code 실측 검증 완료**
- [x] R-2. Rust 갭 — Windows 에이전트 감지(sysinfo 후손 트리) · regex 검색(잘못된 정규식은 패닉 아닌
      `InvalidArgument`) · untracked discard 휴지통(macOS 는 `NsFileManager` — 기본 Finder 방식은
      Apple Event 권한이 필요해 실패) · `classify_git_change` 순수 함수
- [x] R-3. LSP 갭 — `lsp_resolve_root` 커맨드 추가 후 프론트가 루트 결정에 사용,
      ruff 병행 세션(진단 owner 를 `lsp-{serverId}` 로 분리해 마커 덮어쓰기 방지),
      `didChangeWorkspaceFolders` 세션 공유 + ref-count
- [x] R-4. 프론트 갭 — 외부 변경 충돌 배너 · 커밋 그래프 가상 스크롤 · blame "You" 치환
      (`git_current_user` 추가) · 터미널 sessionId 를 layout 탭에 영속(`layout_set_terminal_session`)

### 메인이 직접 처리한 배선·수정

- **`taide` 바이너리 이름 충돌 (실제 사고)** — `src-tauri` 패키지(`taide`)와 `taide-cli` 의
  `[[bin]] name = "taide"` 가 **같은 `target/debug/taide` 로 출력**되어 CLI 가 GUI 앱 바이너리를
  덮어썼다(48MB → 684KB). Cargo 도 "향후 hard error" 경고.
  → CLI bin 을 `taide-cli` 로 분리. 설치 시 `/usr/local/bin/taide` 로 심링크하는 정책은 그대로.
- 커맨드 3종 등록 누락 보완: `layout_set_terminal_session` · `git_current_user` · `lsp_resolve_root`
  (등록 후 프론트의 수동 `invoke` 우회 래퍼를 생성 바인딩으로 교체)
- **에이전트 감지 폴링 태스크** — unix 1s / windows 2s, `AgentStore::diff` 로 **상태 변화 시에만** emit
- **single-instance argv 수신** → `parse_cli_payload` → `AgentExternalOpen` emit + 마커 안전망 등록
- **앱 종료 시 미해결 wait 마커 일괄 삭제** (`cleanup_all_wait_markers`)
- **git watcher 부착** — `.git` 은 무시목록에 있어 기존 워처가 못 봤다. `.git` 전용 워처를 따로 달고
  `classify_git_change` 로 status/refs 를 분류해 이벤트 발행. 프로젝트 close 시 함께 해제.
- 워크스페이스 전환 부수효과: `target/` 위치 변경으로 ESLint 가 Rust 빌드 산출물을 린트 →
  `eslint.config.js`/`.prettierignore`/`.gitignore` 에 `target/**` 추가

### 최종 검증 (2026-08-06)

`typecheck` ✓ / `eslint`(에러 0, 경고 2) ✓ / `prettier` ✓ / `bun test 141 pass` ✓ /
`cargo fmt --all` ✓ / `cargo clippy --workspace -D warnings` ✓ /
`cargo test --workspace` — 169 + 5(통합) + 9(CLI) pass ✓

경고 2건은 `@tanstack/react-virtual` 의 React Compiler 비호환(file-tree·commit-graph).
컴파일러가 안전하게 bail 한 것으로 QA §4.2 에 기록됨 — 조치 불필요.


## 계획: Phase 7.5 — 실사용 피드백 18건 (2026-08-06)

문서화 완료. 순서·상세는 `docs/roadmap.md` Phase 7.5, 결정은
`docs/acknowledge/2026-08-06-phase75-decisions.md`.

### 작성·갱신한 문서

| 문서 | 내용 |
|------|------|
| `acknowledge/2026-08-06-phase75-decisions.md` (신규) | 18건 전량 + 사용자 확인 3건 + 반복 원칙 3가지 |
| `features/preview.md` (신규) | 이미지/비디오/PDF/HTML/xlsx/pptx/HWP 미리보기 |
| `features/window-chrome.md` (신규) | 타이틀바·footer·기동 흰화면·다크모드 미반영 |
| `features/command-palette.md` (신규) | `>` 접두 모드 + **커맨드 레지스트리**(확장성) |
| `features/settings-ui.md` (신규) | 설정 레이아웃·자체 컴포넌트·toast 9분할·리사이저 |
| `features/tabs.md` (갱신) | §3.1 context menu 전량(view component), §4.4 새 창 규칙 |
| `theme-system.md` (갱신) | §7 폰트 커스텀·codeview 테마 버그·테마 편집기·윈도우 배경색 |
| `features/explorer-sidebar.md` (갱신) | §5 파일 아이콘 세트 |
| `roadmap.md` (갱신) | Phase 7.5 A~F + Phase 8 게이트를 7.5 포함으로 수정 |

### 사용자 확인을 받은 결정 3건

1. **탭 열기 = VSCode 규칙** — 단일=preview / 더블·편집=고정 / `⌘+클릭`=분할 /
   새 OS 창은 context menu 로만
2. **미리보기 = 오피스 전량 + HWP** — HWP 는 `rhwp` 사용
   (실사 확인: Rust+WASM, MIT, npm `@rhwp/core` 0.8.2, SVG 렌더 API)
3. **footer 확대/축소 = 에디터·터미널 폰트만** (앱 UI 배율 아님)

### 문서 작성 중 확인한 사실 (추측 아님)

- `@rhwp/core` 0.8.2 / `@rhwp/editor` 0.8.2 · `pdfjs-dist` 6.2.108 · `xlsx` 0.18.5 — npm 실재 확인
- **`pptxgenjs`(4.0.1)는 생성기이지 렌더러가 아니다** — pptx 원본 충실 렌더러는 확인 실패(미확인).
  → `preview.md` §3.1 에 한계를 명시하고 개요 렌더 + LibreOffice 폴백으로 설계
- **sonner 는 `middle-*` 위치를 지원하지 않는다**(6종만) → 9분할 중 3종은 컨테이너 CSS 필요

### 기술 검토 2건 — 완료 (2026-08-06)

- [x] `docs/research/remote-control.md` (374줄) → 요약 `features/remote-control.md`
- [x] `docs/research/terminal-reevaluation.md` (578줄) → 요약 `features/terminal.md` §12

**remote-control**: 잠자기 중 동작은 **OS 정책상 불가**(뚜껑 닫힘·배터리·명시적 잠자기).
잠금은 가능. "화면 그대로"는 픽셀 스트리밍이 아니라 **상태 미러링**이 답이고 ADR-0004 가 이점.
→ **Phase 7.5 에서 구현하지 않는다.** 사용자 기대와 제약이 다르므로 범위 재합의가 먼저.
단 **IPC 계약에 "전송 = Tauri IPC" 가정을 넣지 않는 것**만은 지금부터 지킨다.

**터미널**: **xterm 유지**(임베드 가능한 대체재 없음, VS Code 도 `@xterm/xterm@^6.1.0-beta.292` 사용).
자동완성 잔상의 원인을 **우리 코드 3건**으로 특정 → roadmap 7.5-G.

### Phase 7.5 남은 작업 (구현 미착수 — 별도 세션에서 진행)

문서화만 완료했고 코드는 손대지 않았다. 순서·상세는 `docs/roadmap.md` Phase 7.5.

| 그룹 | 내용 | 상태 |
|------|------|------|
| 7.5-A | 버그 3건 (codeview 테마·기동 흰화면·타이틀바 다크모드) | 미착수 |
| 7.5-B | 기술 검토 2건 | **완료** |
| 7.5-C | UI 일관성·확장성 (탭 메뉴·새창 규칙·커맨드 레지스트리·설정·toast·리사이저) | 미착수 |
| 7.5-D | 표시·꾸밈 (폰트·테마 편집기·파일 아이콘·헤더/footer) | 미착수 |
| 7.5-E | 미리보기 (이미지/PDF/비디오/HTML/xlsx/pptx/HWP) | 미착수 |
| 7.5-F | remote-control | **구현 안 함** (범위 재합의 필요) |
| 7.5-G | 터미널 잔상 수정 (원인 3건 특정 완료) | 미착수 |

현재 코드 기준선: 프론트 141 tests / Rust 169 + 5(통합) + 9(CLI) tests, 전체 게이트 통과.

## 진행 중: Phase 7.5 구현 (2026-08-06, 2번째 세션)

> 기준: `docs/roadmap.md` Phase 7.5. 결정은 `docs/acknowledge/2026-08-06-phase75-decisions.md`,
> 추가 결정은 `docs/acknowledge/2026-08-06-i18n-and-session-findings.md`.

### 7.5-A 버그 3건 — 코드 반영 완료, 시각 확인 대기

- [x] a. codeview 테마 미반영 (2번) — `ThemeProvider` 에서 `applyMonacoTheme(theme, monaco.editor)` 호출.
      monaco 는 `@shared/lib/monaco/setup` 재export 경유. `applyMonacoTheme` 이 매번 `defineTheme`+`setTheme`
      을 부르므로 테마 전환 시 재정의도 함께 처리된다.
- [x] b. 기동 시 흰 화면 (4번) — 실제 원인은 reveal 타이밍이 아니라 **웹뷰 기본 흰 배경**이었다.
      `body` 배경이 `global.css`(JS 번들 주입)에만 있어 첫 페인트에 없었고, 헤더만 색이 맞았던 것은
      창 `backgroundColor` 가 비쳐 보인 것. → `index.html` 인라인 `<style>` 로 첫 페인트 바탕색 확보 +
      변수 주입을 `useLayoutEffect` 로 옮겨 reveal 보다 먼저 DOM 에 반영되게 했다.
- [x] c. 타이틀바 다크/라이트 (17번) — 테마 적용 시 `getCurrentWindow().setBackgroundColor(app.background)`.
      `capabilities/main.json` 에 `core:window:allow-set-background-color` 추가.

파일: `src/app/providers/theme-provider.tsx`, `src/shared/lib/window-background.ts`(신규),
`src/shared/constants/theme.ts`(신규), `index.html`, `src-tauri/capabilities/main.json`.

### 7.5-G 터미널 — 원인 판별 후 A·B·P1 수정 완료

판별 체크리스트(`terminal.md` §12.5) 실행 결과:

- `tput cols` → **80** → **원인 B 확정**
- 잔상 + **한글 입력 한 템포 지연** → **원인 A 지목**(버스트 끝 소량 청크가 갇힘)

- [x] P0-1. reader 타이머 flush — `OutputBatch` 구조체 + 5ms 틱 flusher 스레드.
      `infra/pty.rs`, 상수 `OUTPUT_FLUSH_TICK_MS`(`domain/terminal/types.rs`). 테스트 4건 추가.
- [x] P0-2. 실측 cols/rows 로 spawn — `TerminalView.onReady(cols, rows)` 신설 →
      `TerminalPane`(sessionId nullable) → `TerminalSession` 이 실측값으로 spawn + 비동기 spawn 중
      크기가 바뀌었으면 후속 resize. Rust 기본값은 fallback 으로만 남는다.
- [x] P1. `pty_write`/`pty_resize`/`pty_set_paused` 에서 `begin_mutation` 제거 —
      셋 다 `TerminalStore` 와 pty 내부 락만 건드리고 영속 상태를 바꾸지 않는다.
      키 입력이 파일 저장·git 뒤에 줄 서던 문제 해소. `bindings.ts` 변경 없음(State 는 JS 인자가 아님).
- [ ] P0-3. 탭 전환 시 숨김 유지(원인 C) — **A·B 실사용 확인 후 판단**. 아직 미착수.

사용자 실측 결과: `tput cols` 창 폭에 맞음(B 해결), 잔상 사라짐(A 해결).
CJK 입력 끊김은 P1 적용본으로 재확인 대기.

### 부수 발견 (별건)

- [x] **새 터미널을 여는 경로가 없었다** — 기본 레이아웃의 Terminal 탭 1개뿐이고 닫으면 복구 불가.
      `toggle-terminal` 은 keymap 선언만 있고 핸들러가 없었다.
      → keymap `new-terminal`(`⌃⇧\``) 추가 + 커맨드 팔레트/전역 단축키 배선.
- [ ] **`⌘W` 가 앱을 종료한다** — IDE 라면 탭 닫기여야 한다. 7.5-C 로.
- [ ] **타이틀바 중앙 정보(프로젝트·파일·브랜치)는 미구현** — 7.5-D(18번). 버그 아님.
- [ ] `applyMonacoTheme` 은 6자리 hex 가 아니면 예외를 던진다. 사용자 테마(7.5-D §7.3) 도입 전
      Rust 로더 검증(`theme-system.md` §6)이 선행돼야 한다.

### 세션 운영 교훈 (중요)

- **에이전트 셸에서 앱을 띄우면 안 된다.** 두 번 오진했다.
  1. 샌드박스 셸에서 띄운 인스턴스는 웹뷰가 IPC 부트스트랩을 못 받아 파일트리 빈 상태 + 테마 미적용이 됐다.
     코드 회귀로 오인했다.
  2. macOS **TCC 권한은 부모 프로세스에 귀속**된다 → 에이전트 셸에서 띄우면 보호 폴더 접근이
     `Operation not permitted (os error 1)` 로 막힌다.
  → **앱 실행은 사용자가 직접 한다.** 에이전트는 코드·검증까지만.
- 진단용으로 Chrome 을 `localhost:5173` 에 붙이면 **vite 클라이언트 로그가 섞여** 앱 콘솔로 오인된다.
  (Tauri 런타임이 없어 `__TAURI_INTERNALS__` undefined 에러가 대량 발생) 진단 후 반드시 탭을 닫는다.

### 7.5-G 후속 — 한글(CJK) 입력 불가 (2026-08-06, 같은 세션)

잔상(A·B) 수정 후 남은 **한글 입력 불가**를 별건으로 추적해 해결했다.
근본 원인·실측·시도 이력 정본: `docs/bug/2026-08-06-wkwebview-ime-composition.md`.

- [x] Safari 격리 테스트로 xterm 무죄 입증 — 같은 WebKit 엔진에서 조합이 정상 동작
- [x] 앱 내 계측으로 **WKWebView 가 조합 이벤트를 전혀 발생시키지 않음**을 확정
      (`document` 캡처 리스너로 커맨드 팔레트까지 확인 → 앱 전역 문제)
- [x] `resolveImeInput` 어댑터 + 테스트 5건 — `insertReplacementText` 를 백스페이스+재전송으로 번역
- [x] 임시 계측 전량 제거
- [ ] Tauri/wry/tao 상위 버전에서 수정되면 어댑터 제거 (조사 진행 중)

**부수 수정**
- [x] 재시작 후 터미널 빈 화면 — 죽은 `sessionId` 를 재사용하던 문제.
      `terminalSessionsQueryOptions` 로 살아있는 세션을 확인 후 없으면 새로 spawn
      (`terminal.md` §3 이 정한 동작이 미구현이었음)
- [x] 타이틀바가 IDE 테마색이 아니던 문제 — `titleBarStyle` `Transparent`→`Overlay`
      (`Transparent` 는 `fullsize_content_view(false)` 라 웹뷰가 그 영역을 안 그림) +
      신호등 겹침 방지용 `features/window/title-bar.tsx` 스트립
- [x] **`setBackgroundColor` 호출 제거** — `@tauri-apps/api` 가 `{ color }` 로 보내는데
      Rust 커맨드는 `value` 를 기대해(`tauri-2.11.5 window/plugin.rs` setter 매크로)
      `None` 으로 역직렬화되고 `NSWindow.setBackgroundColor(nil)` 로 config 값을 지운다.
      **내가 추가한 이 호출이 타이틀바를 밝게 만든 원인이었다.** 업스트림 버그
- [x] 새 터미널 커맨드(`⌃⇧\``) — 열 방법이 아예 없었음

**미해결로 넘김**
- [ ] `followSystemTheme` 이 백엔드 no-op — `theme_get_current` 가 `settings.theme_id` 만 읽는다
      (`domain/theme/commands.rs:19-23`). 설정 UI 에 체크박스는 있으나 동작 안 함 → 7.5-D
- [ ] `⌘W` 가 앱을 종료 → 7.5-C

### 7.5-H 다국어 — 구현 완료 (2026-08-06)

- [x] Rust `domain/locale` — `LocalePack`/`LocaleSummary`/`ResolvedLocale`(flat dotted key),
      내장 en/ko/ja 137키, `extends` 부분 오버라이드, `{app_data}/locales/*.json` 사용자 팩 열거,
      `resolve_language(system|명시)` — 테스트 8건
- [x] `lib.rs` 커맨드 3종 등록 + `bindings.ts` 재생성
- [x] 프론트 `shared/i18n/i18n.ts` — i18next 26.3.6 / react-i18next 17.0.11.
      `keySeparator: false` 로 flat 키 사용, `addResourceBundle` 로 런타임 주입
      (**언어팩 요구를 만족시키는 핵심** — 컴파일타임 방식은 이래서 배제)
- [x] `entities/locale` + `LocaleProvider` 배선, `settings.language` + 언어 선택 UI(네이티브 select 미사용)
- [x] UI 하드코딩 문자열 전량 치환 — 22개 파일, `grep [가-힣]` 잔여 0건.
      `keymap.ts` 의 `description` 은 `descriptionKey` 로 바꿔 렌더 시점에 번역
- [x] **보간 문법 정정** — Rust 메시지가 `{count}` 였는데 i18next 기본은 `{{count}}` 라
      치환이 안 됐다. Rust 쪽을 표준으로 맞춤
- [ ] locales watcher 핫리로드 (미착수)

**남은 검증**: 실제 앱에서 언어 전환이 즉시 반영되는지 눈으로 확인 필요.

### 7.5-C UI 일관성·확장성 — 구현 완료 (2026-08-06)

- [x] **`⌘W` 앱 종료** — 원인은 커스텀 메뉴 부재. Tauri 가 만든 macOS 기본 메뉴의 "Close Window"(⌘W)가
      키를 선점하고 있었다. `lib.rs` 에 앱 메뉴를 직접 구성해 그 항목만 제거.
      **Edit 메뉴(undo/redo/cut/copy/paste/select_all)는 반드시 유지** — 빼면 macOS 에서 ⌘C/⌘V 가 죽는다.
      프론트는 `EditorArea` 가 `close-tab` 키맵으로 포커스 pane 의 활성 탭을 닫는다.
- [x] **커맨드 레지스트리 + `>` 모드** — `shared/lib/command-registry.ts`(순수, React/IPC 미import),
      테스트 21건. `⌘P`=파일 모드, `⌘⇧P`=커맨드 모드(`>` 프리필). 실행 구현이 없는 항목은
      disabled 로 표시(가짜 toast 제거).
- [x] **탭 context menu 확장** — Close/Others/ToRight/Saved/All, Copy Path/Relative Path,
      Reveal in Finder/Open Changes, Split 4방향. 기존 버그도 발견·수정(`onCloseOthers` 에
      `tab.closeAll` 라벨이 잘못 붙어 있었음).
- [x] **파일 트리 툴바** — 새 파일·새 폴더·새로고침·모두 접기. 이름 입력은 네이티브 prompt 대신 Dialog.
- [x] **설정 화면 재구성** — 좌측 sticky TOC + Card + `max-w` 제거 + native checkbox → 자체 Switch.
- [x] **toast 9분할** — sonner 는 6종만 지원(`top/bottom` × `left/center/right`)이라
      중간 행은 `top-*` 앵커 + CSS 수직 보정. 순수 함수로 분리하고 테스트 4건.
      **부수 수정**: `Toaster theme='dark'` 하드코딩이라 라이트 테마에서 토스트만 다크였던 것을 테마 추종으로.
- [x] **리사이저 두께** — `PaneSeparator` 로 히트영역(8px 고정)과 시각 두께(설정값) 분리. VSCode 방식.
- [x] `emptyPatch` 를 `entities/settings` 의 `emptySettingsPatch()` 로 이전 — 설정 필드가 늘 때마다
      호출부가 깨지던 것을 한 곳으로 모음.

**남은 검증**: 실제 앱에서 ⌘W·toast 위치·리사이저·설정 화면·팔레트 `>` 모드를 눈으로 확인 필요.

### 7.5-D 진행 (2026-08-06) — 계약 확정분

메인이 먼저 확정한 것(에이전트는 이 위에서만 작업):

- [x] Rust `domain/font` — `fontdb` 0.24 로 시스템 폰트 열거. `FontFamily { name, monospaced }`,
      `font_list()` 커맨드. 가족명 중복 제거 + 정렬. 테스트 2건
- [x] `settings.editorFontFamily` / `terminalFontFamily` / `uiFontFamily` 필드
- [x] `theme_save(theme)` / `theme_delete(themeId)` 커맨드 — 내장 테마 덮어쓰기 차단,
      경로 구분자(`/ \ .`) 포함 id 거부(경로 탈출 방지). 테스트 3건
- [x] **`followSystemTheme` no-op 수정** — `theme_get_current(systemTheme)` 로 시그니처 변경.
      판단은 Rust(`builtin_id_for_system`), 시스템 값 감지는 view 가 센서 역할(locale 과 동일 패턴).
      프론트는 `prefers-color-scheme` 구독으로 OS 테마 변경 시 즉시 무효화
- [x] **pty 로케일 누락 수정** — `LANG`/`LC_CTYPE` 를 UTF-8 로케일로 설정.
      Finder 실행 릴리스 빌드에서 pty 가 non-UTF-8 이 되어 한글이 깨지는 것을 막는다.
      기존 환경변수에 UTF-8 이 있으면 승계, 없으면 `en_US.UTF-8`

미착수로 남은 것: themes/locales 디렉토리 watcher 핫리로드(둘 다 없음 — 대칭적으로 함께 처리 필요).

### 7.5-D 표시·꾸밈 — 구현 완료 (2026-08-06)

- [x] **시스템 폰트 선택(1번)** — Rust `font_list()`(fontdb 0.24), 검색형 `FontPicker`(cmdk 재사용,
      monospace 필터 기본 on, 항목별 미리보기). `buildMonospaceFontStack` 으로 **폴백 체인 강제**.
      Monaco 는 `updateOptions`, xterm 은 `options.fontFamily` + **`fit()` 재호출**(안 하면 커서 밀림)
- [x] **파일 아이콘(6번)** — `resolveFileIcon`/`resolveFolderIcon` 순수 함수 + 테스트 18건.
      탐색기 행과 탭 아이콘 양쪽에 적용
- [x] **테마 편집기(14번)** — 자체 HSV 색 피커(native color input 미사용), 라이브 프리뷰,
      `extends` 로 **바뀐 토큰만** 저장. `theme-draft.ts`/`color.ts` 순수 로직 + 테스트 30건
- [x] **타이틀바 중앙 정보(18번)** — 활성 탭 · 프로젝트명 · git 브랜치. 폭이 좁아지면
      브랜치→프로젝트명 순으로 생략. `data-tauri-drag-region` 을 자식마다 부여(상속 안 됨)
- [x] **footer(15번)** — 상태바 + 에디터/터미널 폰트 크기 컨트롤(앱 UI 배율 아님)

**메인이 직접 처리한 것**
- `shared/ui/**` 가 eslint `ignores`(shadcn 전용)라 에이전트가 거기 만든 자체 아이콘 파일이
  **lint 검사를 빠져나갔다.** `src/shared/icons/` 로 이동해 검사 대상에 포함시켰다.

**남은 결정**
- material-icon-theme 실제 SVG 도입 — 라이선스·번들 크기 검토 필요. 현재는 분류만 참조
- 파일 타입 전용 색 토큰 부재 — 기존 8색 토큰 재사용이라 ts/css/md 가 같은 색
- 테마 내보내기/가져오기 — `@tauri-apps/plugin-fs` 미설치

### 7.5-E 미리보기 — 구현 완료 (2026-08-06)

**메인이 확정한 계약**
- `file_read_raw(path)` — raw 바이트를 `ArrayBuffer` 로 반환하는 커맨드. specta 를 안 거치므로
  `RAW_CHANNEL_COMMANDS` 에 등록하고 `invoke<ArrayBuffer>('file_read_raw', { path })` 로 호출한다.
  20MB(`READ_ONLY_FILE_BYTES`) 상한은 Rust 가 강제
- **asset 프로토콜 활성화** — 비디오·오디오 스트리밍용(전체 메모리 적재 금지).
  `protocol-asset` 피처 + CSP `media-src`/`frame-src` 추가.
  **스코프는 `[]` 로 두고 열린 프로젝트 루트만 런타임 등록**(`allow_asset_access`) — 임의 경로 노출 차단
- **`TabKind::Preview` 를 만들지 않기로 결정** — 위 roadmap 주석 참조

**구현**
- [x] 이미지(SVG 는 `<img>` 로만 — 인라인 시 스크립트 실행 위험) / 비디오 / 오디오 / HTML
- [x] PDF(pdfjs-dist 6.2.108, worker 는 Monaco 와 동일 `?worker` 패턴, unmount 시 `destroy()`)
- [x] xlsx(SheetJS 0.18.5) — 순수 변환 함수 + 테스트 5건, 행 수 상한 + "일부만 표시" 안내
- [x] HWP/HWPX(`@rhwp/core` 0.8.2)
- [x] pptx 개요 — 한계를 UI 에 명시
- [x] 미지원·실패 시 "외부 앱에서 열기"

**메인이 직접 처리한 것**
- 에이전트가 만든 PDF/xlsx/HWP/pptx 컴포넌트가 **`PreviewPane` 에 배선되지 않은 채 남아 있었다.**
  4종을 dispatch 에 연결(objectUrl 이 필요한 것과 ArrayBuffer 를 그대로 넘기는 것을 분리)
- HTML iframe 이 blob URL 을 여는데 **CSP 에 `frame-src` 가 없었다** — 추가

**남은 것**: LibreOffice 폴백(Rust 커맨드 필요), 실제 파일로 각 미리보기 렌더 확인(눈으로 검증 필요)

### Phase 7.6 IDE 핵심 루프 — 구현 (2026-08-06)

**메인이 확정한 계약**: `GitBranch` / `GitStashEntry` / `SearchReplaceResult` 타입,
Rust 커맨드 10종 `lib.rs` 등록 + bindings 재생성.

**Rust**
- [x] git 브랜치·stash·hunk — `CheckoutBuilder::safe()` 로 커밋 안 된 변경을 덮어쓰지 않고,
      충돌 시 명확한 에러로 떨어뜨린다. `branch_delete` 는 현재 브랜치 거부 + force 없으면 미머지 거부.
      `discard_hunk` 는 `gutter()` 와 **같은 diff 옵션·같은 hunk 경계 계산**을 공유하도록
      헬퍼를 추출해, 프론트가 받은 좌표와 어긋날 수 없게 했다. 테스트 17건
- [x] `search_replace` — 매칭 로직을 `search()` 와 공유(규칙이 갈리면 안 된다),
      `persist::write_atomic` 로 원자적 쓰기, 바이너리·비UTF8 skip, 줄 종결자(LF/CRLF) 보존.
      실패해도 실제 변경분을 정확히 반환. 테스트 7건

**프론트**
- [x] `⌘F` 스마트 분기 — 에디터 포커스면 Monaco find, 아니면 전역 검색.
      `⇧⌘F`(전역)는 문서화된 값이라 건드리지 않고 `find` 액션을 신설
- [x] 전역 치환 UI — 파일 단위 선택 + 확인 다이얼로그(되돌릴 수 없으므로 필수)
- [x] Problems 패널 / 아웃라인 패널 / 키맵 오버라이드 UI / 저장 시 포맷·자동 저장
- [x] 마크다운 미리보기 / 터미널 다중 세션 / 최근 프로젝트 / 드래그&드롭

**메인이 직접 채운 것**
- **워크플로 설계 누락**: git 브랜치 UI 를 프론트 작업에 배정하지 않아 Rust 만 만들어졌다.
  `BranchSwitcher`(popover + cmdk 검색, 로컬/원격 그룹, 없는 이름이면 생성 제안)를 직접 만들고
  git 패널 헤더의 브랜치 이름을 이것으로 교체
- 전역 치환이 `replaceNotWired` toast 로 막혀 있던 것을 실제 커맨드에 배선
- i18n 키 11종 추가(3언어) — 키 목록 배열에도 등록해야 파리티 테스트를 통과한다

**남은 것**: git stash UI, hunk 되돌리기 UI(커맨드·훅은 준비됨), 그리고 **전 기능 시각 확인**

### Phase 7.6 마무리 (2026-08-07)

- [x] **git stash UI** — git 패널 상단에 stash 섹션. 목록 + 스태시(변경이 있을 때만 활성) + 적용 + 삭제.
      `ResourceGroupHeader` 의 기존 action prop 형태(`actionLabel`/`actionIcon`/`onAction`)를 그대로 따랐다
- [x] **hunk 단위 되돌리기 UI** — Monaco `onMouseDown` 에서 `GUTTER_LINE_DECORATIONS` 타겟을 잡아
      클릭한 줄이 속한 hunk 를 찾고, **확인 다이얼로그**를 거쳐 `git_discard_hunk` 를 호출한다.
      되돌리면 복구가 불가능하므로 확인 없이 실행하지 않는다
- [x] i18n 키 5종 추가(3언어) + 네임스페이스 키 목록 등록

**Phase 7.6 구현 완료.** 프론트 279 tests / Rust 218 tests, lint 0 errors, 한글 리터럴 잔여 0건.
**전 기능 시각 확인이 남아 있다** — 이번 Phase 에서만 찾기/바꾸기·Problems·아웃라인·브랜치 전환·
stash·hunk 되돌리기·키맵 설정·마크다운·드래그&드롭이 추가됐다.
