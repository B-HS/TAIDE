# HANDOFF — 2026-08-06 세션 스냅샷

> 최종 갱신: 2026-08-06 / 대응 커밋: `718d64d` 이후 **전량 미커밋**(`git status` 로 확인)
> 이 문서는 세션 인수인계 **단일 진입점**이다. 새 세션은 이것부터 읽는다.

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용이다.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` 의 FR-A~J 전량 구현 → Phase 8 배포(서명·공증) |
| 현재 마일스톤 | **Phase 7.5** — 실사용 피드백 18건 반영 (Phase 8 게이트) |
| 직전 작업 | Phase 7.5 **문서화만 완료**. 코드는 손대지 않음 |

Phase 0~7 + 잔여 전량은 **구현·검증 완료**됐다.

## 3. 완료 / 진행 중 / 미착수

### 3.1 완료 (코드 반영됨)

| Phase | 산출물 |
|-------|--------|
| 0 스캐폴딩 | `package.json`·`vite.config.ts`·`eslint.config.js`·`tsconfig.json`·`src-tauri/` 골격 |
| 1 셸 | `domain/{project,layout,theme,settings}`, `widgets/{app-shell,app-sidebar,editor-area}` |
| 2 에디터·파일 | `domain/{file,tree}`, `infra/watcher.rs`, `shared/lib/monaco/`, `widgets/{explorer,editor-pane}` |
| 3 터미널 | `infra/pty.rs`, `domain/terminal`, `features/terminal`, `widgets/terminal-pane` |
| 4 Git | `domain/git`(+`watch.rs`), `widgets/git-panel`, `shared/lib/graph-lanes.ts` |
| 5 LSP | `infra/lsp_proc.rs`, `domain/lsp`, `shared/lib/lsp/`(client+어댑터 10종) |
| 6 에이전트 | `domain/agent`, `crates/taide-cli` |
| 7 검색·설정·플러그인 | `domain/{search,plugin}`, `widgets/{search-panel,settings-view,command-palette}` |

**현재 기준선**: 프론트 141 tests(15 파일) / Rust 183 tests(169 lib + 5 통합 + 9 CLI).
IPC 커맨드 **79종**(specta 77 + raw channel 2), 이벤트 **16종**.

### 3.2 진행 중

**없음.** Phase 7.5 는 문서화만 끝났고 구현은 시작하지 않았다.

### 3.3 미착수 — Phase 7.5 (정본: `docs/roadmap.md` Phase 7.5)

| 그룹 | 내용 | 관련 문서 |
|------|------|----------|
| **7.5-A** | 버그 3건 (codeview 테마·기동 흰화면·타이틀바 다크모드) | `theme-system.md` §7.2, `window-chrome.md` §1.2·§2 |
| 7.5-B | 기술 검토 2건 | **완료** |
| **7.5-G** | 터미널 잔상 수정 (원인 3건 특정 완료) | `terminal.md` §12 |
| 7.5-C | UI 일관성·확장성 (7·8·9·10·11·12·13번) | `tabs.md` §3.1·§4.4, `command-palette.md`, `settings-ui.md` |
| 7.5-D | 표시·꾸밈 (1·6·14·15·18번) | `theme-system.md` §7.1·§7.3, `explorer-sidebar.md` §5, `window-chrome.md` |
| 7.5-E | 미리보기 (3번) | `preview.md` |
| 7.5-F | remote-control | **구현 안 함** — 범위 재합의 필요 |

### 3.4 알려진 미검증 (KNOWN ISSUE — 코드는 있으나 확인 못 함)

- **Windows 코드는 컴파일조차 검증 못 했다.** `#[cfg(windows)]` 블록은 macOS 에서 타입체크가 스킵된다.
  대상: `domain/agent/commands.rs`(sysinfo 감지), `infra/pty.rs`(`foreground_pid`).
- **LSP 실환경 미검증** — 서버(vtsls 등) 설치 상태에서 completion/hover 가 실제로 뜨는지 확인 안 됨.
- **git gutter/blame·diff 의 시각적 렌더 미확인.**
- `ExternalOpenRequest.path` 가 단수라 `--wait` + 다중 파일은 첫 파일만 전달된다
  (Claude Code ctrl+g 는 항상 1개라 실사용엔 무관).

## 4. 의사결정 요약

상세는 `docs/acknowledge/` (결정 1건 = 파일 1개) 및 `docs/adr/0001~0011`.

### 4.1 채택 + 이유

| 결정 | 이유 |
|------|------|
| **TypeScript 5.9.3** (7.0.2 아님) | `typescript-eslint` peer 가 `<6.1.0`. TS7 이면 `.tsx` 린트가 통째로 죽는다 |
| **`build.minify: 'oxc'`** | Vite 8 은 esbuild 를 번들하지 않음 — 템플릿의 `'esbuild'` 는 빌드 실패 |
| **React Compiler 는 Babel 유지** | oxc 네이티브 포트는 Rolldown 이 되돌림(바이너리 +5.1MB·성능). 실측 비용 +0.1s/90모듈 |
| **`pty_spawn`/`pty_attach` 만 specta 밖** | raw 바이트 Channel 은 specta 를 통과 못 함(§7.3) |
| **스플릿 `sizes` 는 퍼센트(0..100)** | react-resizable-panels v4 의 `Layout` 과 단위 통일 |
| **project mutation 은 동기 저장, layout 은 debounce** | 빈도·유실 비용이 다르다 (data-model §4 대비 의도적 편차) |
| **도메인 저장소는 `state.rs` 가 아니라 각 도메인 `commands.rs`** | 병렬 구현 시 `state.rs` 충돌 회피. 결과적으로 응집도 상승 |
| **탭 열기 = VSCode 규칙** | 사용자 확정. `⌘+클릭`=분할, 새 OS 창은 context menu 로만 |
| **HWP 는 `rhwp`** | 사용자 지정. 실사 확인: Rust+WASM, MIT, `@rhwp/core` 0.8.2, SVG 렌더 API |
| **footer 확대/축소 = 에디터·터미널 폰트만** | 사용자 확정 (앱 UI 배율 아님) |
| **xterm.js 유지** | 임베드 가능한 대체재 없음. VS Code 도 동일 라이브러리 |

### 4.2 기각된 대안 + 기각 이유 (**같은 삽질 반복 금지**)

| 기각안 | 이유 |
|--------|------|
| TypeScript 7.0.2 | typescript-eslint 미지원 → `.tsx` 린트 전멸 |
| `Vec<u8>` 채널로 터미널 출력 | JSON 숫자 배열이 되어 성능 붕괴 |
| `InvokeResponseBody` 에 `specta::Type` 구현 | **orphan rule 위반** (둘 다 외부 크레이트) |
| 자체 newtype 에 `IpcResponse` 구현 | **coherence 위반** — tauri 에 `impl<T: Serialize> IpcResponse for T` 블랭킷 impl 존재 |
| `revision: u64` / `time_unix: i64` | specta 가 BigInt 계열 export 거부 → `u32`/`f64` 로 |
| shadcn `resizable` 컴포넌트 | 구 `PanelGroup` API 기반이라 v4 와 불일치 |
| **remote-control 픽셀 스트리밍** | ScreenCaptureKit 이 활성 GUI 세션 요구 → **잠금 상태에서 탈락**. CRD 커튼모드도 Big Sur+ 중단 |
| **remote-control "잠자기 중 동작"** | OS 정책상 불가 — 뚜껑 닫힘·배터리·명시적 잠자기는 assertion 으로 못 막음 |
| xterm 대체(Ghostty/WezTerm/Alacritty) | 전부 네이티브 → 웹뷰 임베드 불가. `@wterm/ghostty` 는 0.3.2 로 시기상조 |
| `pptxgenjs` 로 pptx 렌더 | **생성기이지 렌더러가 아니다** |
| CLI bin 이름 `taide` | `src-tauri` 앱 바이너리와 **출력 파일명 충돌** — 실제로 앱을 덮어씀 |

## 5. 사용자 방향성 & 작업 규칙

### 5.1 답변·보고 스타일

- **한국어 + 존댓말.** 간결하게. 미사여구·자축 금지.
- **이모지·아스키아트 금지** (응답·코드·커밋 전부).
- **검증 안 된 "완벽/잘 됨" 단언 금지.** tsc·테스트 통과만으로 단정하지 않는다.
- 실수는 인정하고 보고한다. 에이전트 보고를 그대로 믿지 말고 **직접 검증** 후 채택한다.

### 5.2 코드 규칙 (ESLint 가 강제)

- arrow function 만, 반환 타입 명시 금지, `any`/`enum` 금지(`unknown` 은 경계에서만+즉시 좁힘)
- **코드 주석 금지**(JSDoc 만), 매직넘버 금지, 2회 이상일 때만 공통화
- **`useCallback`/`useMemo` 금지**(React Compiler 위임), `useEffect` 는 외부 동기화만
- **effect 안 동기 `setState` 금지** — `react-hooks/set-state-in-effect` 가 **에러**
- named export, 1파일 1컴포넌트, `FC<Props>`, 본문 순서 `useRef`→`useState`→함수→`useEffect`
- **FSD 의존 방향 위→아래만**, barrel(index.ts) 금지 — ESLint 가드 있음
- 색은 시맨틱 토큰만(raw hex 금지), 동적 className 은 `cn()`

### 5.3 금지 사항

- **요청 전 commit/push 금지.** `git add -A` 금지, force push 금지, `Co-Authored-By` 트레일러 금지
- **`.env` 읽기·쓰기 금지**, 시크릿 하드코딩 금지
- **네이티브 UI 위젯 금지** — context menu·`<input>`·`<select>` 를 그대로 쓰지 않는다.
  전 OS 일관성을 위해 `shared/ui` 자체 컴포넌트로 (사용자가 8·13번에서 반복 지적)
- **HACK·우회 금지** — 근본 원인 해결. `@ts-ignore`/`eslint-disable` 로 검사기 끄기 금지

### 5.4 반복해서 지적받은 것

- **"goal 이 있는데 왜 멈추냐"** — 보고만 하고 턴을 끝내지 말 것. 작업을 이어서 진행한다.
- **워크플로를 활용할 것** — 병렬화 가능한 작업은 subagent 로. 구현은 sonnet, 리서치·판단은 opus.
- **문서를 제대로 갱신할 것** — 코드와 정합성 없는 문서는 거짓말이다.

## 6. 미해결 질문 / 사용자 확인 필요

1. **remote-control 범위 재합의 필요** — 사용자가 원한 "잠자기 중 동작"이 OS 정책상 불가.
   "전원 연결 + 뚜껑 열림/외부 디스플레이" 전제로 축소할지, Phase 8 이후로 미룰지 확인 필요.
2. **pptx 미리보기 수준** — 원본 충실 렌더러가 없다. 개요 렌더로 만족할지,
   LibreOffice 의존을 감수할지 확인 필요 (`preview.md` §3.1).
3. **macOS 방화벽 접근 요청 "2.1.233"** — 사용자가 문의했으나 어떤 앱인지 미확정.
   확인 결과 TAIDE 바이너리는 소켓을 열지 않고 Vite 는 루프백 전용이었다.

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS (Darwin 24.6.0, arm64). **Windows/Linux 미검증** |
| 패키지 매니저 | **bun** 1.3.0 |
| 워크스페이스 | Cargo workspace — `src-tauri` + `crates/taide-cli` |
| 실행 | `bun run tauri dev` (Vite 5173 루프백 + Tauri 앱) |
| 검증 | `bun run verify` = typecheck → lint → format:check → bun test → cargo fmt/clippy/test (workspace) |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` |
| 커밋 상태 | **이번 세션 변경분 전량 미커밋** (사용자가 커밋을 요청하지 않음) |

**허용 경고 2건**: `@tanstack/react-virtual` 의 React Compiler 비호환
(`file-tree.tsx`·`commit-graph.tsx`). 컴파일러가 안전하게 bail 한 것 — 조치 불필요.

## 8. 다음 세션 TODO (우선순위 순)

### 1순위 — 7.5-A 버그 3건 (나머지 작업의 전제)

**1번은 이 문서 작성 중 원인이 확정됐다.** 2·3번은 아직 증상 보고 상태이므로 실측 후 고친다.

1. **codeview 테마 미반영** — **원인 확정됨. 조사 불필요, 바로 고치면 된다.**
   `applyMonacoTheme` 을 **아무도 호출하지 않는다**(`grep -rn "applyMonacoTheme" src/` 결과 없음).
   변환 로직은 정상 — `src/shared/lib/monaco/theme.ts:88` 이 `base` 를 올바르게 파생한다.
   → **배선만 하면 된다**: `src/app/providers/theme-provider.tsx` 에서 테마 로드/변경 시 호출.
   monaco 는 `@shared/lib/monaco/setup` 재export 경유(직접 import 금지).
   테마 **전환** 시에는 `defineTheme` 재호출이 필요하다(`setTheme` 만으로는 이전 정의가 남음).
   **완료 조건**: 다크/라이트 전환 시 에디터 배경·구문색이 함께 바뀌는 것을 눈으로 확인.
2. **기동 시 흰 화면** — `src/app/providers/theme-provider.tsx`, `index.html`
   `ThemeProvider` 가 `isFetched`(데이터 도착)로 reveal 하는데 **DOM 반영 시점이 아니다**.
   `index.html` 에 첫 페인트용 바탕색 인라인 `<style>` 추가(정적 기본색은 ADR-0004 위반 아님).
   **완료 조건**: 기동 시 흰 깜빡임이 사라진 것을 눈으로 확인.
3. **타이틀바 다크/라이트 미반영** — `src-tauri/tauri.conf.json`, 테마 적용 흐름
   `backgroundColor` 가 **정적 값**이라 윈도우 배경이 다크 고정. 테마 전환 시
   `window.set_background_color()` 갱신 추가 (`window-chrome.md` §1.2).

### 2순위 — 7.5-G 터미널 잔상

**먼저 `terminal.md` §12.5 판별 체크리스트 실행** (원인 3개가 증상은 같고 고칠 곳이 다르다):
`tput cols` → 80 이면 B / 잔상 상태에서 키 1회로 사라지면 A / 탭 전환 직후에만이면 C.

- **P0-1** `src-tauri/src/infra/pty.rs` — reader 에 **타이머 flush**(별도 flusher 스레드 5ms 틱).
  현재 flush 판정이 `read()` 직후에만 있어 버스트 끝 소량 청크가 갇힌다.
- **P0-2** `src-tauri/src/domain/terminal/commands.rs` + `src/widgets/terminal-pane/terminal-session.tsx`
  — 실측 cols/rows 로 spawn (현재 `DEFAULT_TERMINAL_COLS/ROWS = 80/24` 고정)
- **P0-3** `src/widgets/editor-area/pane-node-view.tsx` — 탭 전환 시 unmount 대신 숨김 유지
- **P1** `pty_write`/`resize`/`set_paused` 를 전역 `begin_mutation` 락에서 분리
  (`src-tauri/src/state.rs` — 현재 키 입력이 파일 저장·git 뒤에 줄 선다)

### 3순위 이후

`docs/roadmap.md` Phase 7.5 의 C → D → E 순서.
7.5-F(remote-control)는 **구현하지 않는다** — §6.1 확인 후 결정.

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 세션 인수인계 단일 진입점 |
| `docs/PROCESS.md` | Phase 0~7.5 작업 이력·체크리스트 (시간순 기록) |
| `docs/roadmap.md` | **구현 순서의 정본**. Phase 0~8 + Phase 7.5 A~G |
| `docs/PRD.md` | 요구사항 FR-A~J |
| `docs/architecture.md` | 시스템 구조·Rust/프론트 디렉토리·IPC 경계·수명주기 규칙 |
| `docs/tech-stack.md` | 버전 정본 + **구현하며 확정·추가된 것**(하단) |
| `docs/ipc-contract.md` | command/event 계약 정본 |
| `docs/data-model.md` | 영속 타입·저장 전략·마이그레이션 |
| `docs/theme-system.md` | 테마 토큰·파생 규칙 + §7 Phase 7.5 확장(폰트·편집기·버그) |
| `docs/adr/0001~0011` | 아키텍처 결정 기록 |
| `docs/acknowledge/*.md` | 사용자와의 결정·합의 (결정 1건 = 파일 1개) |
| `docs/features/layout-shell.md` | 앱 셸·멀티 프로젝트 |
| `docs/features/tabs.md` | 탭·스플릿 + §3.1 context menu + §4.4 새 창 규칙 |
| `docs/features/editor.md` | Monaco·모델 레지스트리·gutter·blame·키바인딩 |
| `docs/features/explorer-sidebar.md` | 파일 트리·검색 + §5 파일 아이콘 |
| `docs/features/terminal.md` | pty·xterm + **§12 재평가 결과(잔상 원인 3건)** |
| `docs/features/git.md` | SCM 패널·diff·그래프 |
| `docs/features/lsp.md` | LSP 세션·클라이언트·어댑터 |
| `docs/features/agent-integration.md` | 에이전트 감지·`taide` CLI·IDE MCP(2차) |
| `docs/features/plugins.md` | 플러그인 매니페스트 |
| `docs/features/preview.md` | **신규** 미리보기(이미지/PDF/비디오/xlsx/pptx/HWP) |
| `docs/features/window-chrome.md` | **신규** 타이틀바·footer·기동 흰화면·다크모드 |
| `docs/features/command-palette.md` | **신규** `>` 접두 모드 + 커맨드 레지스트리 |
| `docs/features/settings-ui.md` | **신규** 설정 레이아웃·toast 9분할·리사이저 |
| `docs/features/remote-control.md` | **신규** 검토 결론 요약 + 지금 지킬 계약 원칙 |
| `docs/research/*.md` | 기술 조사 원문 (1차 출처 링크·미확인 표기 포함) |
| `docs/quality-assurance/docs-review-checklist.md` | 문서 정합성·실측 확인 항목 |

## 10. 복기 신뢰도

- **높음**: Phase 0~7.5 의 결정·기각안·코드 구조 — 전부 이 세션에서 직접 수행·검증했다.
- **중간**: 각 Phase 의 세부 테스트 케이스 목록 — 서브에이전트 보고에 의존한 부분이 있다.
  숫자(테스트 수·커맨드 수)는 이 문서 작성 시 **재실측**했다.
- **낮음 없음** — 대화 전체가 단일 세션이고 중간 요약 손실이 없었다.
