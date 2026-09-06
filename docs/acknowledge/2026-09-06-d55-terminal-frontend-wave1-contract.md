# d-55 — 터미널·팔레트 프론트 웨이브 1: 링크 좌표 문법 · 빈 프롬프트 가짜 블록 · 퍼지 다중 토큰 (2026-09-06)

> `docs/research/2026-09-06-terminal-agent-deep-dive.md` 종합 웨이브 1 중 **Rust 를 건드리지 않는** 3건(T5-03 · SI-4 · W7-7)을 먼저 처리한다.
> d-54(Rust 단일 에이전트) 와 병행 가능하도록 범위를 프론트 파일로 한정한다. 실행 방식은 `docs/agent-operations.md` §2.

## 0. 근본 원인 (증거 — 메인이 코드로 재확인)

- **A. 링크 좌표 문법**: `src/shared/lib/terminal-link.ts` 의 `TERMINAL_LINK_PATTERN` 은 접미사가 `:L` 과 `:L:C` 둘뿐이다. tsc(`--pretty false`) 의
  `src/a.ts(12,3)`, python 트레이스백 `File "/abs/x.py", line 42`, GitHub 식 `#L42`, 범위 `:10-20` 은 좌표를 잃고 `line 1` 로 열린다.
  Rust `resolve_terminal_path` 는 `match.path`(접미사 제거된 경로)만 받으므로(`terminal-session.tsx` `handleOpenFileLink`) IPC 계약은 무관하다.
- **B. 빈 프롬프트 가짜 블록**: zsh 훅은 매 프롬프트마다 `133;D;$?` → `133;A` 를 찍으므로 아무것도 입력하지 않은 Enter·Ctrl-C 는 `C` 없이 `A→D` 만
  남는다. `applyOsc133Event` 는 `A` 에서 무조건 블록을 push 하고 `D` 에서 `exitCode` 가 null 이 아니면 `attachOsc133BlockTracker.applyDecoration`
  이 거터·오버뷰 룰러 장식을 그린다(`src/features/terminal/terminal-osc133.ts`). 빈 프롬프트마다 성공색 바가, Ctrl-C 로 비운 프롬프트에는 실패(130)
  빨간 바가 생기고, `MAX_TRACKED_COMMAND_BLOCKS`(500) 예산과 ⌘↑/⌘↓ 점프 대상에 섞인다. 단, macOS 기본 bash 3.2 는 PS0 이 없어 **정상 명령도
  `C` 가 없다**(`docs/features/terminal.md` §5) — 무조건 버리면 그 셸의 블록이 전부 사라진다.
- **C. 퍼지 다중 토큰**: `src/shared/lib/fuzzy-match.ts` 의 `fuzzyMatch` 는 공백을 일반 문자로 소비한다. 팔레트 파일 모드 대상은 프로젝트 상대 경로라
  공백이 없으므로 `search panel` 같은 입력은 항상 0건이다(`command-palette.tsx` `fuzzyFilter(searchTerm, filePaths, …)`).

## 1. 수정 방향

### 1.A 링크 좌표 문법 확장 — `src/shared/lib/terminal-link.ts` (+ `terminal-link.test.ts`)

- 접미사 문법을 한 곳(`TERMINAL_LINK_PATTERN` 의 접미사 부분 또는 `as const` 접미사 표)에 모으고 **가장 긴 성공 접미사**를 채택한다. 지원 형식:
  `:L` · `:L:C` · `:L:C:`(뒤 콜론은 링크 밖) · `(L,C)` · `(L)` · `#L<L>`(GitHub, `#L10-L20`·`#L10-20` 은 L=10) · `:L-M`(범위, L 채택) ·
  `", line N` / `', line N`(python — 닫는 따옴표 뒤 `, line N`). `TerminalLinkMatch.text`/`endIndex` 는 접미사를 포함해 밑줄이 `(12,3)`·`, line 42`
  까지 덮는다. `path` 는 접미사를 제외한 원문 그대로(변경 없음).
- 유지: 확장자 필수·경계 문자 규칙·URL 내부 비매칭(`http://x/y.ts` 에서 파일 링크 없음)·`v1.2.3` 에 좌표 없음. 이 회귀 테스트를 추가한다.
- `handleOpenFileLink`·`readTerminalRowColumns`·Rust 는 변경 없음.
- 문서: `docs/features/terminal.md` §6 표의 `path:line:col` 행을 지원 형식 목록으로 갱신.

### 1.B 빈 프롬프트 가짜 블록 제거 — `src/features/terminal/terminal-osc133.ts` (+ 테스트)

- `Osc133BlockTrackerState` 에 `hasSeenOutputStart: boolean` 래치를 추가한다(`C` 를 한 번이라도 보면 true, 세션 수명 동안 유지).
- `D` 처리: `hasSeenOutputStart && currentBlock.outputStartMarker === null` 이면 그 블록은 빈 프롬프트다 — end 마커를 등록하지 않고 블록을 배열에서
  제거하며 `currentBlockIndex: null`, `changedBlock: null`, 추가로 `discardedBlock` 을 돌려준다. 트래커는 `discardedBlock.startMarker.dispose()` 로
  xterm 마커를 회수한다(기존 `onDispose → pruneBlockByStartMarker` 경로가 그대로 뒷정리; 재진입 안전 확인).
- `C` 를 본 적 없는 세션(bash 3.2·미지 셸)은 현행 유지(관대) — 세션 첫 명령 이전의 빈 Enter 1회는 블록으로 남을 수 있음을 문서에 적는다.
- 테스트: `C` 를 본 세션의 `A→D` 는 버려진다 / 본 적 없으면 유지 / 버린 뒤 `currentBlockIndex` null / 점프 후보에서 제외 / 기존 테스트 전부 유지.
- 문서: `docs/features/terminal.md` §5 에 규칙 한 줄.

### 1.C 퍼지 다중 토큰 — `src/shared/lib/fuzzy-match.ts` (+ 테스트)

- `fuzzyMatch` 의 단일 토큰 계약은 그대로 둔다. `fuzzyFilter` 에서 질의를 공백으로 분해(trim·빈 토큰 제거·`MAX_FUZZY_QUERY_TOKENS`(8) 초과분 무시)하고,
  0~1 토큰이면 기존 경로, 2개 이상이면 **모든 토큰이 각각 매치**할 때만 통과·점수는 합·`indices` 는 합집합(정렬·중복 제거).
- 소비처(`command-palette.tsx`·`keybindings-editor.tsx`)는 변경 없음. `command-palette-workspace-symbol-group.tsx` 의 직접 `fuzzyMatch` 호출은 범위 밖.
- 테스트: 토큰 각각 매치 통과 / 순서 무관 / 하나라도 실패하면 탈락 / 점수 합 / 인덱스 합집합 / 앞뒤·연속 공백 무시 / 토큰 상한 / 단일 토큰 결과 불변.
- 문서: `docs/features/command-palette.md` 퍼지 절에 한 줄.

### 1.D 범위 외

T5-01(존재 검증 resolver, Rust)·T5-02·T5-04~T5-10(T5-10 은 Rust `expand_home`)·W7-6·W7-8(bench 선행)·SI-5 등 웨이브 1 의 Rust 접촉 항목은 d-54 완료 후 별도 계약.

## 2. 실행 계획

- 구현 wf(opus·xhigh): A → B 순차(둘 다 `terminal.md` 를 편집), C 병렬. 각 에이전트 종료 조건: `bun run typecheck`·`bun run lint`·`bun run format:check`·
  `bun test <해당 테스트 파일>` exit 0, 계약 §3 기록.
- 검토·테스트는 d-54 와 같은 렌즈 wf 에 합류. 메인 2차 검증은 `bun run verify` 전체.

## 3. 기록

- **§1.A 링크 좌표 문법 확장 완료 (2026-09-06)**
  - 변경 파일: `src/shared/lib/terminal-link.ts`(경로+좌표 일체형 `TERMINAL_LINK_PATTERN` 을 경로 전용 `TERMINAL_LINK_PATH_PATTERN` 과 접미사 표
    `TERMINAL_LINK_SUFFIX_PATTERNS`(6규칙)로 분리 · `matchTerminalLinkSuffix` 가 경로 끝에 표 전체를 시도해 **가장 긴 성공 접미사**를 채택),
    `src/shared/lib/terminal-link.test.ts`(테스트 13개 추가 → 25 pass), `docs/features/terminal.md` §6 표 행 + 아래 2 bullet.
  - 지원 형식: `:L` · `:L:C` · `:L:C:`(뒤 콜론은 링크 밖) · `:L-M`(시작 줄) · `(L,C)` · `(L)` · `#L<L>`/`#L10-L20`/`#L10-20` · `", line N`/`', line N`.
    접미사 캡처 규약은 그룹 1 = line, 선택적 그룹 2 = column, 나머지 비캡처 — 규칙이 늘어도 소비 코드는 그대로다.
  - `TerminalLinkMatch` 타입·`path`(접미사 제외 원문)·`text`/`endIndex`(접미사 포함) 계약 불변. 매치 후 `lastIndex` 를 접미사 끝으로 밀어 접미사 내부
    재스캔을 막는다(경로 최소 3자라 진행 보장 — 기존 zero-length 가드는 도달 불가라 제거).
  - 회귀 테스트 추가: URL 내부 비매칭(`http://x/y.ts` · `https://github.com/.../a.ts#L42`) · `v1.2.3` 좌표 없음 · 확장자/경계 문자 규칙.
  - `terminal-file-link.ts`·`terminal-session.tsx` `handleOpenFileLink`·`readTerminalRowColumns`·Rust 무변경, 신규 의존성 0.
  - 검증: `bun run typecheck` 0 · `bun run lint` 0(경고 11건은 기존 `react-hooks/incompatible-library`) · `bun run format:check` 0 ·
    `bun test src/shared/lib/terminal-link.test.ts` 25 pass 0 fail · 참고 `bun test src/features/terminal/terminal-file-link.test.ts` 11 pass 0 fail.
  - 이탈: 없음.
- **§1.B 빈 프롬프트 가짜 블록 제거 완료 (2026-09-06)**
  - 변경 파일: `src/features/terminal/terminal-osc133.ts`(`Osc133BlockTrackerState.hasSeenOutputStart` 래치 + `Osc133EventTransition.discardedBlock` 추가,
    `D` 분기에서 래치 ON & `outputStartMarker === null` 이면 end 마커를 등록하지 않고 블록을 배열에서 제거·`currentBlockIndex: null`·`changedBlock: null`,
    트래커 `handleOscData` 가 `disposeBlockMarkers(discardedBlock)` 로 마커 회수), `src/features/terminal/terminal-osc133.test.ts`(테스트 5개 추가 → 39 pass),
    `docs/features/terminal.md` §5 파싱 절에 규칙 bullet 1개.
  - 래치는 **`C` 가 열린 블록에 실제로 반영된 경로에서만** true 로 켠다 — 열린 블록 없는 stray `C` 와 `registerMarker` 실패 `C` 는 기존 no-op(상태 동일 참조)을
    유지해야 하고(기존 테스트 2건이 참조 동일성을 단언), 마커 없이 래치만 켜면 그 블록의 `D` 가 실제 명령을 빈 프롬프트로 오인해 버리기 때문이다(관대 쪽으로 실패).
  - `pruneDisposedBlocks` 는 `...state` 스프레드로 래치를 보존한다(스크롤백 정리 후에도 엄격 모드 유지). `INITIAL_OSC133_BLOCK_TRACKER_STATE` 에 `hasSeenOutputStart: false`
    가 추가되면서 기존 `pruneDisposedBlocks` 테스트 5건의 상태 리터럴에 같은 필드 한 줄씩만 더했다(단언·의미 불변).
  - 재진입: `discardedBlock.startMarker.dispose()` → `A` 에서 걸어 둔 `onDispose` → `pruneBlockByStartMarker` → `pruneDisposedBlocks` 는 이미 배열에서 빠진 블록이라
    무해하다(데코레이션은 애초에 생성되지 않음). 트래커 테스트가 dispose 리스너를 실제로 발화시키는 fake 마커로 이 경로를 통과시킨다.
  - 테스트 5개: `C` 를 본 세션의 빈 `A→D` 폐기(`currentBlockIndex` null·`changedBlock` null·`discardedBlock` 반환) / 폐기 시 end 마커 미등록 /
    `C` 를 본 적 없는 세션은 유지 / 래치가 prune 후에도 유지 / 트래커에서 점프 후보 제외 + `startMarker` 회수. 기존 34개 전부 통과.
  - `attachOsc133BlockTracker` 시그니처·`terminal-view.tsx`·`terminal-session.tsx`·Rust 무변경, 신규 의존성 0.
  - 검증: `bun run typecheck` 0 · `bun run lint` 0(경고 11건은 기존 `react-hooks/incompatible-library`) · `bun run format:check` 0 ·
    `bun test src/features/terminal/terminal-osc133.test.ts` 39 pass 0 fail · 참고 `bun test src/features/terminal/` 72 pass 0 fail.
  - 이탈: 없음(위 래치 게이팅 범위는 기존 no-op 계약 유지를 위한 해석).
- **§1.C 퍼지 다중 토큰 완료 (2026-09-06)**
  - 변경 파일: `src/shared/lib/fuzzy-match.ts`(`MAX_FUZZY_QUERY_TOKENS`=8 · `QUERY_TOKEN_SEPARATOR_PATTERN` · `splitQueryIntoTokens` ·
    `matchQueryTokens` 추가, `fuzzyFilter` 가 토큰 분해 경로를 쓰도록 교체 — `fuzzyMatch` 단일 토큰 계약·`fuzzyFilter` 시그니처·정렬 규칙 불변),
    `src/shared/lib/fuzzy-match.test.ts`(테스트 8종 추가), `docs/features/command-palette.md` §3 첫 항목 아래 규칙 1항 추가.
  - 라벨은 후보당 정확히 1회만 읽는다(토큰 반복은 이미 읽은 라벨 문자열 위에서 돈다) — `perf-budget.test.ts` 의 `fuzzyFilter 연산 예산` 유지.
  - 소비처(`command-palette.tsx`·`keybindings-editor.tsx`·`task-runner-dialog.tsx`) 무변경, 신규 의존성 0, Rust 무접촉.
  - 검증: `bun run typecheck` 0 · `bun run lint` 0(경고 11건은 기존 `react-hooks/incompatible-library`) · `bun run format:check` 0 ·
    `bun test src/shared/lib/fuzzy-match.test.ts src/shared/lib/perf-budget.test.ts` 43 pass 0 fail.
  - 이탈: 없음. 단 파생 동작 1건 — 공백뿐인 질의(`'   '`)는 토큰 0개로 축약돼 빈 질의와 동일하게 전체를 원 순서로 반환한다(변경 전에는 공백을 문자로
    찾아 0건). 계약 §1.C 테스트 목록에 없던 케이스라 아래 검토에서 확정하고 `docs/features/command-palette.md` §3 에 명시했다.

### 3.1 검토·수정 (2026-09-06)

리뷰 렌즈(근본성·계약 준수 / 회귀·경계)의 발견 3건. 확정 major 0건.

| id | severity | 확정 | 처리 |
|----|----------|------|------|
| L1-01 (`terminal-link.ts:12` 비공개 상수 JSDoc) | minor | 확정 | **수용** — `TERMINAL_LINK_SUFFIX_PATTERNS`(비 export)에 붙어 있던 13행 JSDoc 삭제. 같은 내용이 이미 `docs/features/terminal.md` §6 에 있고, comments.md §2.1 은 JSDoc 을 공개 API·재사용 유틸로 한정한다. 대신 export 되는 `findTerminalLinkMatches` 에 3줄 요약(가장 긴 접미사 채택 · `path` 는 접미사 제외 · `text`/`endIndex` 는 포함 · 문법 목록은 문서 참조)만 남겼다. 런타임 동작·테스트 불변. |
| fuzzy-multi-token-score-inflation (`fuzzy-match.ts:79` 겹침 토큰 점수 중복 가산) | minor | 확정(현상) | **코드 변경 기각 · 문서화로 수용** — 계약 §1.C 가 "점수는 합"을 명시하고 §1.C 테스트("점수 합")가 이를 단언하므로, 겹침 감점 로직은 계약 이탈이다. 통과·탈락 판정과 강조 인덱스(합집합)는 영향 없고 후보 간 순위만 흔들릴 수 있는 범위라, 트레이드오프를 `docs/features/command-palette.md` §3 에 한 항목으로 남기고 규칙은 유지했다. 변경이 필요하면 계약 개정 후 별도 작업. |
| L1-02 (공백뿐인 질의 → 전체 반환) | info | 확정 | **기록** — 위 §1.C "이탈" 줄에 파생 동작으로 명시하고 `docs/features/command-palette.md` §3 문구를 "단일 토큰은 기존 동작 / 빈 질의·공백뿐인 질의는 전체 원 순서"로 갱신. 코드 변경 없음(빈 질의와 동일 취급이 사용자 관점에서 합리적). |

- 변경 파일: `src/shared/lib/terminal-link.ts`(JSDoc 이동), `docs/features/command-palette.md`(§3 2항), 본 계약 §3.
- 검증: `bun run typecheck` 0 · `bun run lint` 0(경고 11건은 기존 `react-hooks/incompatible-library`) · `bun run format:check` 0 ·
  `bun test src/shared/lib/terminal-link.test.ts src/shared/lib/fuzzy-match.test.ts` 63 pass 0 fail.
