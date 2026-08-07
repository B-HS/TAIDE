# HANDOFF — 2026-08-07 세션 스냅샷

> 최종 갱신: 2026-08-07 / 대응 커밋: **`bf0a713`** (`dev` 브랜치, origin 푸시 완료)
> 이 문서는 세션 인수인계 **단일 진입점**이다. 새 세션은 이것부터 읽는다.
> 수치는 이 문서 작성 시점에 **재실측**했다(§7).

## 1. 프로젝트 한 줄 정의

**TAIDE** — Tauri 2 + Rust 코어 + React 19 프론트로 만드는 **에이전트 친화 데스크톱 IDE**.
모든 도메인 상태를 Rust 가 소유하고(ADR-0004), view 는 표시 전용이다.

## 2. 현재 목표

| 층위 | 내용 |
|------|------|
| 최종 목표 | `docs/PRD.md` FR-A~J 전량 구현 → Phase 8 배포(서명·공증) |
| 직전 마일스톤 | **Phase 7.5 (실사용 피드백 19건) — 완료** |
| 직전 마일스톤 | **Phase 7.6 (IDE 핵심 루프 12건) — 완료** |
| 현재 상태 | **코드 구현은 전부 끝났고, 실제 화면 검증만 남았다** |

## 3. 완료 / 진행 중 / 미착수

### 3.1 완료 (커밋 반영됨)

| Phase | 커밋 | 산출물 |
|-------|------|--------|
| 0~7 + 7.5-A/G | `ee096ca` | 셸·에디터·파일·터미널·git·LSP·에이전트·검색/설정/플러그인 + 초기 버그 3건 + 터미널 잔상·한글입력 |
| 7.5-H 다국어 | `5942abb` | `domain/locale`, i18next 런타임 주입, 사용자 언어팩, UI 문자열 전량 치환 |
| 7.5-C UI 일관성 | `77369a7` | 커맨드 레지스트리·탭 메뉴·파일트리 툴바·설정 재구성·toast 9분할·리사이저·⌘W |
| 7.5-D 표시·꾸밈 | `0e2ac7a` | 폰트 선택·파일 아이콘·테마 편집기·타이틀바 중앙정보·footer |
| 7.5-E 미리보기 | `0154d04` | 이미지·비디오·오디오·PDF·HTML·xlsx·HWP·pptx(개요) |
| 7.6-A/B | `92c1506`, `bf0a713` | 찾기/바꾸기·전역치환·Problems·아웃라인·git 브랜치/stash/hunk·키맵·마크다운·터미널 다중세션·최근프로젝트·드래그&드롭 |

**사용자 요구 19건 최종 상태**: 완료 17건 / 보류 2건(5번 remote-control, 7번 새 창 규칙).

### 3.2 진행 중

**없음.** 모든 구현이 커밋·푸시됐다. `git status` 는 `Cargo.lock` 만 수정 상태다
(fontdb 추가분 — 다음 커밋에 함께 넣으면 된다).

### 3.3 미착수 — 우선순위는 §8

- **화면 검증** (가장 중요) — 아래 §8 1순위
- `docs/backlog.md` 의 3순위 후보 전량 (멀티 윈도우 포함)
- themes/locales 디렉토리 **watcher 핫리로드** — 둘 다 없다. 대칭적으로 함께 처리해야 한다
- 7.5-G **P0-3**(탭 전환 시 unmount 대신 숨김 유지) — A·B 수정으로 잔상이 사라져 보류 중

### 3.4 알려진 미검증 (KNOWN ISSUE — 코드는 있으나 확인 못 함)

- **Windows 코드는 컴파일조차 검증 못 했다.** `#[cfg(windows)]` 블록은 macOS 에서 타입체크가 스킵된다.
  대상: `domain/agent/commands.rs`, `infra/pty.rs`(`foreground_pid`).
- **LSP 실환경 미검증** — 서버 설치 상태에서 completion/hover 가 실제로 뜨는지 확인 안 됨.
- **Phase 7.5~7.6 의 거의 모든 UI 가 시각 미확인.** 아래 §8 참조.
- `applyMonacoTheme` 은 6자리 hex 가 아니면 예외를 던진다. 내장 테마는 안전하나
  사용자 테마가 잘못된 hex 를 담으면 앱이 죽는다. 방어 위치는 `theme-system.md` §6 이 정한
  **Rust 로더 검증**이고 아직 없다.
- `search_replace` 는 **에디터에 열린 dirty 버퍼와 화해하지 않는다** — 디스크를 직접 쓴다.

## 4. 의사결정 요약

상세는 `docs/acknowledge/` (결정 1건 = 파일 1개) 및 `docs/adr/0001~0011`.

### 4.1 채택 + 이유

| 결정 | 이유 |
|------|------|
| **TypeScript 5.9.3** (7.x 아님) | `typescript-eslint` peer 가 `<6.1.0`. TS7 이면 `.tsx` 린트가 통째로 죽는다 |
| **`build.minify: 'oxc'`** | Vite 8 은 esbuild 를 번들하지 않음 |
| **React Compiler 는 Babel 유지** | oxc 네이티브 포트는 Rolldown 이 되돌림 |
| **xterm.js 유지** | Safari 에서 같은 xterm 이 한글을 완벽 처리 — **xterm 무죄가 실측으로 증명됐다.** 웹뷰가 조합 이벤트를 안 주면 어떤 웹 터미널도 같다 |
| **`TabKind::Preview` 미신설** | 레이아웃은 영속 데이터라 스키마 확장이 마이그레이션을 부른다. "어떤 렌더러로 그릴지"는 view 판단 |
| **`theme_get_current(systemTheme)`** | 판단은 Rust, OS 값 감지는 view 가 센서. ADR-0004 는 "상태 소유"를 요구하지 "센서 소유"가 아니다 |
| **locale = 테마와 동일 구조** | 같은 문제(내장 + 사용자 파일 + `extends` 병합)를 두 번 풀지 않는다 |
| **flat dotted i18n key** | 사용자 언어팩이 **키 단위 부분 오버라이드**를 하기 쉽다 |
| **i18next 런타임 `addResourceBundle`** | 언어팩을 넣으면 재빌드 없이 적용 — 이 요구가 후보를 갈랐다 |
| **asset 스코프는 프로젝트 루트만 런타임 등록** | 비디오 스트리밍에 필요하되 임의 경로 노출은 막는다 |
| **자동 커밋·푸시 ON (`dev` 한정)** | 사용자 확정. `main` 머지는 지시 시에만 |

### 4.2 기각된 대안 (**같은 삽질 반복 금지**)

| 기각안 | 이유 |
|--------|------|
| TypeScript 7.0.2 | typescript-eslint 미지원 → `.tsx` 린트 전멸 |
| `Vec<u8>` 채널로 터미널 출력 | JSON 숫자 배열이 되어 성능 붕괴 |
| `InvokeResponseBody` 에 `specta::Type` 구현 | **orphan rule 위반** |
| 자체 newtype 에 `IpcResponse` 구현 | **coherence 위반** |
| `revision: u64` / `time_unix: i64` | specta 가 BigInt 계열 export 거부 → `u32`/`f64` |
| shadcn `resizable` 컴포넌트 | 구 `PanelGroup` API 기반이라 v4 와 불일치 |
| **Paraglide JS / LinguiJS** | 컴파일 타임 고정 → **사용자 언어팩을 런타임에 못 붙인다** |
| **터미널 라이브러리 교체** | Safari 실측으로 xterm 무죄 확인. 네이티브(Ghostty 등)는 웹뷰 임베드 불가이고 자체 한글 IME 이슈도 있다 |
| **Tauri/wry/tao 버전 업으로 IME 해결** | tao 0.36.0 / wry 0.56.0 를 받아 **diff 한 결과 IME 관련 변경 0건** |
| **`attachCustomKeyEventHandler` 로 조합 중 키 차단** | 가설 자체가 틀렸다. Safari 로그가 `keyCode: 229` 를 보여 전제가 반증됨 |
| `.xterm-helper-textarea` 크기 키우기 | 조합 이벤트는 여전히 미발생. IME 후보창 위치 목적으로만 유지 |
| **`setBackgroundColor` 로 타이틀바 색 변경** | JS 는 `{ color }`, Rust 는 `value` 를 기대 → `None` 으로 역직렬화돼 **config 값을 지운다**(업스트림 버그). macOS 타이틀바는 `NSAppearance` 가 지배 |
| pptx LibreOffice 폴백 | 사용자 제외 확정 |
| remote-control 픽셀 스트리밍 | 잠금 상태에서 동작 불가 |
| CLI bin 이름 `taide` | 앱 바이너리와 출력 파일명 충돌 |

## 5. 사용자 방향성 & 작업 규칙

### 5.1 답변·보고 스타일

- **한국어 + 존댓말.** 간결하게. 미사여구·자축 금지.
- **이모지·아스키아트 금지** (응답·코드·커밋 전부).
- **검증 안 된 "완벽/잘 됨" 단언 금지.** tsc·테스트 통과만으로 단정하지 않는다.
- 실수는 인정하고 보고한다. **에이전트 보고를 그대로 믿지 말고 직접 검증 후 채택**한다.
  (이 세션에서 "완료"라고 보고된 stash UI 가 실제로는 없었다 — `grep` 으로 잡았다)
- **보고만 하고 턴을 끝내지 말 것.** goal 이 있으면 이어서 진행한다.

### 5.2 코드 규칙 (ESLint 가 강제)

- arrow function 만, 반환 타입 명시 금지, `any`/`enum` 금지(`unknown` 은 경계에서만+즉시 좁힘)
- **코드 주석 금지**(JSDoc 만), 매직넘버 금지, 2회 이상일 때만 공통화
- **`useCallback`/`useMemo` 금지**(React Compiler), `useEffect` 는 외부 동기화만
- **effect 안 동기 `setState` 금지** — `react-hooks/set-state-in-effect` 가 **에러**
- named export, **1파일 1컴포넌트**, `FC<Props>`, 본문 순서 `useRef`→`useState`→함수→`useEffect`
- **FSD 의존 위→아래만**, barrel(index.ts) 금지
- 색은 **시맨틱 토큰만**(raw hex 금지), 동적 className 은 `cn()`
- **사용자에게 보이는 문자열은 i18n 키.** 새 키는 `domain/locale/service.rs` 의 en/ko/ja **3곳 모두**
  + 네임스페이스 키 목록 배열에도 등록해야 파리티 테스트를 통과한다

### 5.3 금지 사항

- **`main` 에 직접 커밋 금지.** 작업은 `dev`. `git add -A` 금지, force push 금지,
  `Co-Authored-By`/Claude 트레일러 금지
- **`.env` 읽기·쓰기 금지**, 시크릿 하드코딩 금지
- **네이티브 UI 위젯 금지** — context menu·`<select>`·`confirm()`·`prompt()`·`<input type="color">`.
  `shared/ui` 자체 컴포넌트로 (사용자가 8·13번에서 반복 지적)
- **HACK·우회 금지** — 근본 원인 해결. `@ts-ignore`/`eslint-disable` 금지
- **새 패키지를 임의로 설치하지 말 것** — 필요하면 보고하고 확인받는다
- **에이전트 셸에서 앱을 띄우지 말 것** (§7 참조)

### 5.4 반복해서 지적받은 것

- **워크플로를 활용할 것** — 병렬화 가능한 작업은 subagent 로. 구현은 sonnet, 리서치·판단은 opus
- **문서를 제대로 갱신할 것** — 코드와 정합성 없는 문서는 거짓말이다
- **계약(types.rs·events.rs·lib.rs)은 메인이 먼저 확정**하고 에이전트는 service/commands 만 채운다.
  **배선은 항상 메인이 한다** (이 세션에서 git 브랜치 UI 를 배정 안 해 Rust 만 만들어진 사고가 있었다)

## 6. 미해결 질문 / 사용자 확인 필요

1. **remote-control 범위 재합의** — "잠자기 중 동작"이 OS 정책상 불가.
   "전원 연결 + 뚜껑 열림" 전제로 축소할지, Phase 8 이후로 미룰지 확인 필요.
2. **멀티 윈도우 착수 시점** — 요구 7번과 탭 메뉴 4항목이 여기 막혀 있다. 지금은 백로그.
3. **material-icon-theme 실제 SVG 도입 여부** — 현재는 분류만 참조해 lucide 로 매핑.
   진짜 SVG 는 라이선스·번들 크기 검토가 필요하다.
4. **파일 타입 전용 색 토큰(`fileIcon.*`) 추가 여부** — 현재 기존 8색 토큰을 재사용해
   ts/css/md 가 같은 색이다. 테마 스키마 확장이 필요하다.
5. **테마 내보내기/가져오기** — `@tauri-apps/plugin-fs` 미설치. 설치 여부 확인 필요.

## 7. 환경 & 전제

| 항목 | 값 |
|------|-----|
| 플랫폼 | macOS (Darwin 24.6.0, arm64). **Windows/Linux 미검증** |
| 패키지 매니저 | **bun** 1.3.0 |
| 리모트 | `origin` = `https://github.com/B-HS/TAIDE.git` (비공개). `main`=prod, `dev`=개발 |
| 워크스페이스 | Cargo workspace — `src-tauri` + `crates/taide-cli` |
| 실행 | `bun run tauri dev` |
| 검증 | `bun run verify` = typecheck → lint → format:check → bun test → cargo fmt/clippy/test |
| 앱 데이터 | `~/Library/Application Support/dev.taide.app/` |

**현재 기준선 (2026-08-07 재실측)**

- 프론트 **279 tests** (28 파일) / Rust **226 tests** (212 lib + 5 통합 + 9 CLI)
- IPC 커맨드 **94종** (specta 91 + raw 3: `pty_spawn`·`pty_attach`·`file_read_raw`), 이벤트 **16종**
- Rust 도메인 16개, 프론트 위젯 17개
- lint: 0 errors / **허용 경고 2건** — `@tanstack/react-virtual` 의 React Compiler 비호환
  (`file-tree.tsx`·`commit-graph.tsx`). 컴파일러가 안전하게 bail 한 것 — 조치 불필요

### 앱 실행은 사용자가 직접 한다 (실패 후 확정)

에이전트 셸에서 앱을 띄웠다가 **두 번 오진**했다.

1. **샌드박스 셸**에서 띄운 인스턴스는 웹뷰가 IPC 부트스트랩을 못 받아 파일트리가 비고
   테마가 적용되지 않았다. 이것을 코드 회귀로 오인했다.
2. **macOS TCC 권한은 부모 프로세스에 귀속**된다 → 에이전트 셸에서 띄우면 보호 폴더 접근이
   `Operation not permitted (os error 1)` 로 막힌다.

부수 주의: 진단용으로 Chrome 을 `localhost:5173` 에 붙이면 **vite 클라이언트 로그가 앱 로그에 섞인다**
(Tauri 런타임이 없어 `__TAURI_INTERNALS__` undefined 에러가 대량 발생). 진단 후 탭을 반드시 닫는다.

## 8. 다음 세션 TODO (우선순위 순)

### 1순위 — 화면 검증 (다른 모든 작업의 전제)

**코드는 전부 구현됐고 `bun run verify` 는 통과하지만, Phase 7.5~7.6 의 UI 대부분이 눈으로 확인되지 않았다.**
사용자가 `bun run tauri dev` 로 직접 띄우고 아래를 확인해야 한다.

| 확인 대상 | 관련 파일 |
|-----------|-----------|
| 언어 전환(en/ko/ja) 즉시 반영 | `src/app/providers/locale-provider.tsx`, `src/features/settings/language-picker.tsx` |
| `⌘W` 탭 닫기(앱 종료 아님) | `src-tauri/src/lib.rs` `build_app_menu`, `src/widgets/editor-area/editor-area.tsx` |
| 팔레트 `>` 커맨드 모드 | `src/shared/lib/command-registry.ts`, `src/widgets/command-palette/command-palette.tsx` |
| 설정 화면(TOC·Card·Switch) | `src/widgets/settings-view/settings-view.tsx` |
| toast 9분할 / 리사이저 두께 | `src/widgets/app-toaster/app-toaster.tsx`, `src/features/split/pane-separator.tsx` |
| 폰트 선택 / 파일 아이콘 / 테마 편집기 | `src/features/settings/font-picker.tsx`, `src/shared/lib/file-icon.ts`, `src/widgets/theme-editor/theme-editor.tsx` |
| 타이틀바 중앙 정보 / footer | `src/features/window/title-bar.tsx`, `src/features/window/status-bar.tsx` |
| 미리보기 8종 | `src/widgets/preview-pane/preview-pane.tsx` |
| **찾기/바꾸기·전역 치환** (파일을 실제로 바꾼다 — 테스트 디렉토리에서) | `src/widgets/search-panel/search-panel-container.tsx` |
| **hunk 되돌리기** (파일을 실제로 바꾼다) | `src/widgets/editor-pane/editor-pane.tsx`, `src/features/git/hunk-discard-dialog.tsx` |
| git 브랜치 전환 / stash | `src/features/git/branch-switcher.tsx`, `src/features/git/stash-list.tsx` |
| Problems / 아웃라인 패널 | `src/widgets/problems-panel/`, `src/widgets/outline-panel/` |

### 2순위 — 검증에서 나온 버그 수정

없으면 3순위로.

### 3순위 — 남은 구현

1. **themes/locales watcher 핫리로드** — 둘 다 없다. `infra/watcher.rs` 재사용, 대칭 처리
2. **`applyMonacoTheme` 방어** — Rust 테마 로더에서 hex 검증(`theme-system.md` §6).
   현재는 잘못된 hex 하나로 앱이 죽는다
3. **`search_replace` ↔ dirty 버퍼 화해** — 디스크를 직접 써서 열린 편집 내용과 어긋날 수 있다
4. `docs/backlog.md` 재검토 — 멀티 윈도우 포함
5. 7.5-G **P0-3**(탭 전환 replay 제거) — 재현되면 착수

### 4순위 — Phase 8 게이트

`docs/roadmap.md` Phase 8. **Phase 0~7.6 이 로컬에서 전부 테스트·확인 완료된 후에만** 진행(사용자 합의).

## 9. 문서 지도

| 문서 | 내용 |
|------|------|
| `docs/HANDOFF.md` | **이 문서** — 세션 인수인계 단일 진입점 |
| `docs/PROCESS.md` | Phase 0~7.6 작업 이력·체크리스트 (시간순). **기준선 수치는 이 HANDOFF §7 이 정본** |
| `docs/roadmap.md` | **구현 순서의 정본**. Phase 0~8 + 7.5 A~H + 7.6 A~B |
| `docs/backlog.md` | **3순위 후보** — 1·2순위 완료 후 재검토. 멀티 윈도우 포함 |
| `docs/PRD.md` | 요구사항 FR-A~J |
| `docs/architecture.md` | 시스템 구조·Rust/프론트 디렉토리·IPC 경계·수명주기 |
| `docs/tech-stack.md` | 버전 정본 + Phase 7.5~7.6 추가 의존성과 선정/기각 이유 |
| `docs/ipc-contract.md` | command/event 계약 정본 + raw 커맨드 3종 + hunk 좌표 계약 |
| `docs/data-model.md` | 영속 타입·저장 전략·마이그레이션 |
| `docs/theme-system.md` | 테마 토큰·파생 규칙 + §7 Phase 7.5 확장 |
| `docs/adr/0001~0011` | 아키텍처 결정 기록 |
| `docs/acknowledge/*.md` | 사용자와의 결정·합의 (결정 1건 = 파일 1개) |
| `docs/bug/2026-08-06-wkwebview-ime-composition.md` | **WKWebView 조합 이벤트 부재** — 원인·실측 비교·반증된 가설 4건 |
| `docs/features/*.md` | 기능별 스펙 (layout-shell·tabs·editor·explorer-sidebar·terminal·git·lsp·agent-integration·plugins·preview·window-chrome·command-palette·settings-ui·remote-control) |
| `docs/research/*.md` | 기술 조사 원문 (1차 출처 링크·미확인 표기 포함) |
| `docs/quality-assurance/docs-review-checklist.md` | 문서 정합성·실측 확인 항목 |

## 10. 복기 신뢰도

- **높음**: Phase 7.5~7.6 의 결정·기각안·코드 구조 — 이 세션에서 직접 수행·검증했다.
  §7 의 수치는 이 문서 작성 시점에 **재실측**했다.
- **중간**: 서브에이전트가 구현한 세부 로직(테마 편집기 내부·마크다운 렌더러·키맵 오버라이드 UI 등).
  타입체크·테스트·사용처 grep 으로는 확인했으나 **동작은 미확인**이다.
- **낮음**: 없음 — 대화 전체가 단일 세션이고 중간 요약 손실이 없었다.
