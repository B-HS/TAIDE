# d-56 — 터미널·PTY 계층 웨이브 1: 리플레이 flow control 제외 · 링 축출 정렬 · zsh OSC 7 · ~ 확장 · 링크 존재 검증 (2026-09-06)

> `docs/research/2026-09-06-terminal-agent-deep-dive.md` 종합 웨이브 1 중 터미널·PTY 계층의 Rust 접촉 5건(T2-F3 · T2-F8 · SI-5 · T5-10 · T5-01).
> **d-54(Rust 단일 에이전트) 완료 후** 착수한다. 실행 방식은 `docs/agent-operations.md` §2. 메인이 코드로 근거를 재확인했다(§0).

## 0. 근본 원인 (증거)

- **T2-F3 리플레이가 flow control 에 집계**: `pty_attach`(`domain/terminal/commands.rs`) 가 최대 2MB 스크롤백을 라이브와 같은 채널로 재생하고(`SessionOutput::attach`),
  FE 는 구분 없이 `pendingRef.current += data.byteLength`(`terminal-view.tsx` write) → `evaluateFlowControl`(`terminal-pane.tsx` `handleWriteBacklogChange`,
  HIGH_WATER 512KB) 이 pause 를 보낸다. 탭을 오갈 때마다 이미 지나간 출력 때문에 살아 있는 자식이 멈춘다.
- **T2-F8 링 축출이 시퀀스 한가운데**: `ScrollbackRing::append`(`domain/terminal/service.rs`) 가 넘치는 만큼을 바이트 단위로 `drain` 한다. 2MB 초과 뒤 재부착 첫
  화면은 임의 바이트에서 시작해 잘린 OSC/CSI 의 잔여가 텍스트로 찍힌다(`docs/features/terminal.md` §12.2-C 가 남은 한계로 기록).
- **SI-5 zsh OSC 7 이 `print -n`**: `infra/shell_integration.rs` `_taide_precmd` 가 `print -n "\e]7;$PWD\e\\"` 를 쓴다. zsh `print` 는 `-r` 없이 인자의 이스케이프를
  해석하므로 백슬래시가 든 디렉터리가 훼손된다. bash 쪽은 이미 `printf '\e]7;%s\e\\' "$PWD"` 다.
- **T5-10 `~user` 확장 오류**: `domain/terminal/service.rs::expand_home` 이 `strip_prefix('~')` 결과를 `$HOME` 뒤에 그대로 붙여 `~alice/notes.md` →
  `/Users/<me>alice/notes.md` 를 만든다.
- **T5-01 링크 존재 검증 부재**: `features/terminal/terminal-file-link.ts` 가 정규식 매치를 무조건 밑줄·포인터 링크로 만들고 존재 검증은 클릭 뒤 `resolve_terminal_path`
  에서만 한다. `v18.20.4`·`127.0.0.1:8080`·`0.123s`·`e.g` 에 밑줄이 그어지고 클릭하면 실패 토스트만 남는다.

## 1. 수정 방향

### 1.A T2-F3 — 리플레이 바이트를 flow control 집계에서 제외

- Rust: `pty_attach` 반환을 `u32` → `PtyAttachResult { subscription_id: u32, replay_bytes: u64 }`(specta Type) 로 넓힌다. `SessionOutput::attach` 가 재생한 두 조각의
  길이 합을 돌려준다. 리플레이가 라이브보다 먼저 도착하는 전제는 `SessionOutput` 락이 이미 보장한다(계약에 명시).
- FE: `entities/terminal/terminal.ipc.ts::attachPty` 가 새 반환을 그대로 노출. `terminal-session.tsx` 의 attach 콜백에 **리플레이 예산 카운터**를 두고, 예산이 남아 있는
  동안 도착한 바이트는 예산에서 차감하며 flow control 집계(`handleWriteBacklogChange` 로 가는 pendingBytes)에는 넣지 않는다. xterm `write` 자체와 perf 카운터는 그대로.
  순수 함수 `consumeReplayBudget(budget, chunkBytes) -> { countedBytes, remainingBudget }`(shared 승격 불필요 — widgets/terminal-pane 에 두고 테스트).
- 문서: `terminal.md` §2 flow control 문단·`ipc-contract.md` `pty_attach` 반환 계약 갱신. `docs/features/terminal.md` §3 재부착 리플레이 문단에 한 줄.

### 1.B T2-F8 — 링 축출을 개행 경계로 정렬

- `ScrollbackRing::append`: overflow 뒤 축출 위치를 **overflow 이후 첫 `\n` 다음**까지 전진시킨다. 개행이 `MAX_EVICTION_SCAN_BYTES`(64KB) 안에 없으면 원래 위치에서
  자른다(TUI 처럼 `\n` 이 드문 출력에서 링이 비는 것을 막는다). capacity 를 최대 한 줄만큼 덜 지키는 것이 비용이다.
- 재부착 리플레이 앞에 SGR 리셋 프리앰블 `\x1b[0m` 1회를 보내고(잘린 SGR 상속 방지), 그 바이트는 §1.A 의 `replay_bytes` 에 포함한다.
- 테스트: 축출 경계가 `\n` 직후인지 / 개행 없는 긴 청크는 원 위치 절단 / capacity 초과분 상한 / 기존 링 테스트 유지.

### 1.C SI-5 — zsh OSC 7 보고를 `printf` 로

- `ZSH_SCRIPT_TEMPLATE` 의 `print -n "\e]7;$PWD\e\\"` → `printf '\e]7;%s\e\\' "$PWD"`. zsh `printf` 는 빌트인이라 "훅이 외부 바이너리를 부르지 않는다" 성질 유지.
- 테스트: 스크립트 조립 단언(백슬래시·`%`·공백 포함 경로가 그대로 실린 시퀀스) + 스캐너(`infra/terminal_scan.rs`, d-54 산출) 라운드트립.

### 1.D T5-10 — `expand_home` 정정

- `~` 단독과 `~/` 로 시작할 때만 `$HOME` 치환. `~user` 형태와 `HOME` 미설정은 원문 유지(해석 실패 → "열리지 않음" 으로 안전 수렴). 테스트 3종.

### 1.E T5-01 — 링크 존재 검증 배치 resolver

- Rust: 커맨드 `terminal_resolve_link_candidates(cwd: String, candidates: Vec<String>) -> Vec<Option<String>>`(입력 순서 보존, 상한 `MAX_LINK_CANDIDATES_PER_ROW`=16
  초과분은 None). 내부는 후보마다 기존 `guard_terminal_path` 재사용 — 루트 밖·부재를 동일하게 `None` 으로 접어 현행 보안 정책(정보 비노출) 유지. 원격 미러 허용 분류는
  `resolve_terminal_path` 와 동일(원격 dispatch 표 갱신).
- FE: `createTerminalFileLinkProvider` 를 비동기화 — 정규식 매치 → 행 텍스트+cwd 키 FIFO 캐시(`LINK_RESOLVE_CACHE_MAX_ROWS`=256) 조회 → 없으면 배치 resolver 호출 →
  존재하는 후보만 `ILink` 로 콜백. 해석된 절대 경로를 매치에 실어 활성화 시 두 번째 IPC 없이 `requestOpenFileFromEditor` 로 간다(`handleOpenFileLink` 단순화).
  cwd 는 `terminal-session.tsx` 가 getter 로 넘긴다. xterm 은 `provideLinks` 지연 콜백을 허용하고 행 단위로 재질의한다(안전).
- 테스트: Rust(순서 보존·상한·루트 밖 None) / TS(캐시 히트·미스·후보 0·resolver 실패 시 링크 없음).
- 문서: `terminal.md` §6 첫 문단("정규식 매치 → 존재 검증 후에만 링크") · `ipc-contract.md` 커맨드 표 · 원격 dispatch 허용 목록 문서.

### 1.F 범위 외

T5-02(확장자 필수 해제)·T5-04(git a/ b/)·T5-07(공백 경로)는 T5-01 위에 웨이브 2 로. T2-F1/F2/F4/F5/F7/F9/F10/F11 은 nix 의존 결정 등 미결 후.

## 2. 실행 계획

- 구현 wf(opus·xhigh): Rust 단일 에이전트 R(1.B → 1.C → 1.D → 1.A Rust → 1.E Rust, 각 단계 cargo fmt/clippy/test) → TS 에이전트 F(1.A FE → 1.E FE, bindings 재생성 확인·
  typecheck/lint/format/test). 문서는 F 가 마지막에.
- 검토·테스트 wf 는 d-54/d-55 와 합류 가능. 메인 2차 검증 `bun run verify` + `bunx vite build`.

## 3. 기록

### R 단계 — Rust 전량 (2026-09-07)

순서: §1.B → §1.C → §1.D → §1.A Rust → §1.E Rust. 각 단계 뒤 `cargo fmt --all` · `cargo fmt --all -- --check` ·
`cargo clippy --workspace --all-targets -- -D warnings` · `cargo test --workspace`.

**변경 파일**

- `src-tauri/src/domain/terminal/service.rs` — §1.B `ScrollbackRing::append` 개행 정렬 축출
  (`MAX_EVICTION_SCAN_BYTES` = 64KB, `line_aligned_eviction_end`), §1.D `expand_home`.
- `src-tauri/src/domain/terminal/types.rs` — §1.A `PtyAttachResult { subscriptionId, replayBytes }` 신설.
- `src-tauri/src/domain/terminal/commands.rs` — §1.B `SCROLLBACK_REPLAY_PREAMBLE`(`\x1b[0m`),
  §1.A `SessionOutput::attach` · `pty_attach` 반환 확장, §1.E `MAX_LINK_CANDIDATES_PER_ROW`(16) ·
  `resolve_link_candidates` · 커맨드 `terminal_resolve_link_candidates`.
- `src-tauri/src/infra/shell_integration.rs` — §1.C zsh OSC 7 보고를 `printf '\e]7;%s\e\\' "$PWD"` 로.
- `src-tauri/src/lib.rs` — `collect_commands!` 에 `terminal_resolve_link_candidates` 등록,
  `specta_builder` 에 `.typ::<PtyAttachResult>()` 추가(사유는 아래 이탈 2).
- `src-tauri/src/domain/remote/dispatch.rs` — 신규 커맨드를 `IMPLEMENTED_JSON_COMMANDS` ·
  `REMOTE_ALLOWED_COMMANDS` · `match` 팔에 `resolve_terminal_path` 와 같은 등급으로 추가, 허용 표 doc 의
  개수(158→159 / 157→158) 갱신.
- `src/shared/api/bindings.ts` — `cargo test` 재생성분(직접 편집 없음).

**테스트 순증 12** (`cargo test --workspace` 1503 passed / 0 failed)

- §1.B 4건: 개행 다음 바이트까지 전진 · 개행 직후 축출이 그 줄을 버리지 않음 · 개행 없는 출력은 원 위치 절단 ·
  상한 밖 개행까지는 전진하지 않음.
- §1.C 2건: zsh 스크립트 `printf` 조립 단언(`print -n` 잔존 금지 포함) · 백슬래시/`%`/공백 경로의 OSC 7 왕복
  (`infra::terminal_scan::scan_once`).
- §1.D 3건: `~`·`~/` 확장 · `~user` 원문 유지 · `HOME` 없음 원문 유지.
- §1.A 2건: 빈 링이면 프리앰블만 재생(+`replayBytes` 일치) · `replayBytes` 가 실제 전송 바이트와 일치(3개 크기).
- §1.E 3건: 입력 순서 보존 · 상한 초과분 `None` · 루트 밖 `None`.
- 기존 테스트 조정 5건(삭제 0): 프리앰블 프리픽스를 반영한 attach 계열 4건, `detach` 가 `subscription_id`
  필드를 쓰도록 1건. 링 회귀 테스트 `되감긴_링도_두_조각을_이어_붙이면_순서가_보존된다` 는 픽스처 바이트가
  `\n`(=10)과 겹쳐 새 축출 규칙과 상호작용하므로 `ROUND_MARK_BASE`(100) 오프셋을 주어 의도(되감김 순서
  보존)만 남겼다.

**검증**: `cargo fmt --all -- --check` OK · `cargo clippy --workspace --all-targets -- -D warnings` 경고 0 ·
`cargo test --workspace` 1503 passed · `bun run format:check` OK · `bun run lint` 오류 0(기존 경고 11, 무관 파일).

**bindings.ts 재생성 확인** (`git diff -- src/shared/api/bindings.ts`): `PtyAttachResult` 타입,
`terminalResolveLinkCandidates(cwd, candidates): (string | null)[]` 커맨드 2건 추가 — 그 외 변경 없음.

**이탈 3건**

1. **`replayBytes` 를 `u64` → `u32`**: 계약 §1.A 는 `u64` 였으나 `specta-typescript` 가 BigInt 계열(u64 포함)
   export 를 거부해 바인딩 생성 테스트가 실패한다. 한 번의 리플레이는 `DEFAULT_SCROLLBACK_BYTES`(2MB) +
   4바이트 프리앰블로 상한이 잡히므로 `u32` 로 충분하다. 누산은 `usize` 로 하고 반환 시
   `u32::try_from(..).unwrap_or(u32::MAX)`(기존 `duration_ms` 와 같은 포화 변환).
2. **`specta_builder` 에 `.typ::<PtyAttachResult>()` 추가**: `pty_attach` 는 `Channel` 인자 때문에
   `collect_commands!` 가 아니라 raw 핸들러(`RAW_CHANNEL_COMMANDS`)에 등록되어 있어, 그대로 두면 새 타입이
   bindings.ts 에 나오지 않는다. FE 가 타입을 손으로 다시 적지 않도록 명시 등록했다(커맨드 표면은 불변).
3. **§1.B 개행 정렬을 "청크가 capacity 이상" 분기에는 적용하지 않음**: 그 분기는 append 전에 꼬리를 잘라
   `overflow` 가 0 이므로 정렬 지점이 없다. pty 읽기 청크는 64KB 라 2MB capacity 에서 실제로 도달하지 않는
   경로이며, 기존 테스트(`용량보다_큰_청크는_꼬리만_남긴다`)의 계약도 그대로 유지된다.

**F 단계로 넘기는 사실**

- `bun run typecheck` 는 **통과한다**. `attachPty` 가 `invokeRaw<number>`(손으로 적은 제네릭)라 반환 확장이
  타입 오류로 드러나지 않기 때문이며, 현재 `terminal-session.tsx` `handleAttachData` 는 객체를
  `detachPty(sessionId, subscriptionId: number)` 에 그대로 넘기는 **런타임 파손 상태**다. F 가 §1.A FE 에서
  `attachPty` 를 `PtyAttachResult` 로 바꾸며 함께 고쳐야 한다.
- 문서(`terminal.md` §2·§3·§6, `ipc-contract.md` terminal 절·원격 허용 목록)는 계약 §2 대로 F 가 마지막에.

### F 단계 — 프론트 + 문서 (2026-09-07)

순서: §1.A FE → §1.E FE → 문서. Rust(`src-tauri`)는 읽기만 했고 수정하지 않았다.

**변경 파일**

- `src/entities/terminal/terminal.ipc.ts` — §1.A `attachPty` 를 `invokeRaw<PtyAttachResult>` 로,
  §1.E `resolveTerminalLinkCandidates` 신설, 호출자가 사라진 `resolveTerminalPath` 래퍼 제거
  (아래 결정 3).
- `src/widgets/terminal-pane/terminal-replay-budget.ts`(신규) — 순수 함수 `consumeReplayBudget`.
- `src/widgets/terminal-pane/terminal-replay-budget.test.ts`(신규) — 7건.
- `src/widgets/terminal-pane/terminal-session.tsx` — §1.A `handleAttachData` 리플레이 예산·큐,
  §1.E `handleGetCwd`/`handleResolveFileLinkCandidates` 신설과 `handleOpenFileLink` 단순화.
- `src/widgets/terminal-pane/terminal-pane.tsx` — `attachData` 콜백 2인자화, 링크 resolver 2개 prop
  중계(기존 ref 동기화 패턴 그대로).
- `src/features/terminal/terminal-view.tsx` — `TerminalAttachHandle.write(data, backlogBytes)`,
  provider 를 deps 객체로 생성, `getCwd`/`resolveFileLinkCandidates` prop 추가.
- `src/features/terminal/terminal-file-link.ts` — provider 비동기화·행 단위 배치 해석·(cwd, 행 텍스트)
  FIFO 캐시(`LINK_RESOLVE_CACHE_MAX_ROWS`=256)·`ResolvedTerminalLinkMatch`.
- `src/features/terminal/terminal-file-link.test.ts` — 새 계약에 맞춰 하네스 교체 + 7건 추가.
- `docs/features/terminal.md` §2(리플레이 제외)·§3(개행 정렬 축출·SGR 프리앰블)·§6(존재 검증 후 링크)·
  §9(IPC 목록)·§12.2-C(남은 한계 해소 반영).
- `docs/ipc-contract.md` — 실측 헤더(186/189/161/28), terminal 절의 `pty_attach` 반환 확장과
  `terminal_resolve_link_candidates` 계약, 원격 dispatch 정책의 d-56 분류 기록.

**테스트 순증 14** (`bun test` 2333 passed / 0 failed, 233 파일)

- §1.A 7건: 예산 0·예산 초과·정확 소진·경계 초과분만 집계·0바이트 청크·음수 예산·연속 청크 누적.
- §1.E 7건: cwd 미확정 시 resolver 미호출 · 행 후보 일괄 전달 · 일부 미해석 시 나머지만 링크 ·
  전부 미해석이면 링크 없음(`v18.20.4`) · 캐시 히트로 resolver 1회 · cwd 변경 시 재해석 ·
  resolver 실패 시 링크 없음 + 실패 비캐시 재시도.
- 기존 테스트 조정 8건(삭제 0): `terminal-file-link.test.ts` 의 provider 테스트 전부가 새 하네스
  (deps 객체 · 비동기 resolver)를 쓰도록 바뀌었고, `activate` 테스트는 `resolvedPath` 단언을 더했다.
  `readTerminalRowColumns` 3건은 손대지 않았다.

**검증**: `bun run typecheck` OK · `bun run lint` 오류 0(경고 11 — 전부 이번 변경과 무관한 기존
`react-hooks/incompatible-library`) · `bun run format:check` OK · `bun test` 2333 passed ·
`cargo test --workspace` exit 0(재실행 후 `git diff -- src/shared/api/bindings.ts` 는 R 단계와 동일한
22줄 추가 그대로 — F 단계가 바인딩을 흔들지 않았다).

**구현 결정 3건**

1. **리플레이 청크를 attach 결과가 올 때까지 큐에 담는다**: 계약 §1.A 는 "예산 카운터"만 적었지만,
   리플레이는 `pty_attach` 커맨드 **안에서** 채널로 나가므로 예산을 실은 결과보다 먼저 도착할 수
   있다. 예산을 모르는 채 흘리면 리플레이 전체가 집계돼 고치려던 pause 가 그대로 남는다. 그래서
   `handleAttachData` 는 결과 도착 전 청크를 큐에 쌓았다가 도착 즉시 순서대로 흘린다(왕복 1회 지연,
   이미 지나간 스크롤백이라 체감 없음). 구독 해제가 먼저 오면 큐는 그대로 버려진다.
2. **`TerminalAttachHandle.write` 에 `backlogBytes` 2번째 인자**: 계약이 제시한 "플래그" 대신 바이트
   수를 넘긴다. 예산 경계를 걸친 청크(리플레이 꼬리 + 라이브 머리)를 통째로 넣거나 빼지 않고 넘친
   만큼만 집계하려면 boolean 으로는 표현할 수 없다. 호출부는 `terminal-pane.tsx` 한 곳뿐이라
   기본값 없이 필수 인자로 뒀다.
3. **`resolveTerminalPath` FE 래퍼 제거**: §1.E 로 클릭 시점 해석이 사라져 호출자가 0이 됐다(죽은
   코드 제거 규칙). Rust 커맨드 `resolve_terminal_path` 와 원격 분류는 그대로 두었고, "프론트
   호출자 0" 이라는 사실을 `terminal.md` §9 · `ipc-contract.md` terminal 절에 명시했다.

### 총괄 (R + F)

계약 §1.A~§1.E 5건을 전부 구현했다. §1.F(웨이브 2 이월 항목)는 손대지 않았다.

| 항목 | 상태 | 근거 |
|------|------|------|
| §1.A 리플레이 flow control 제외 | 완료 | `PtyAttachResult.replayBytes` → `consumeReplayBudget` → `write(data, backlogBytes)` |
| §1.B 링 축출 개행 정렬 + SGR 프리앰블 | 완료 | `ScrollbackRing::append` · `SCROLLBACK_REPLAY_PREAMBLE` |
| §1.C zsh OSC 7 `printf` | 완료 | `ZSH_SCRIPT_TEMPLATE` + 스캐너 왕복 테스트 |
| §1.D `expand_home` 정정 | 완료 | `~`·`~/` 만 치환, `~user`·HOME 미설정 원문 유지 |
| §1.E 링크 존재 검증 | 완료 | `terminal_resolve_link_candidates` + provider 비동기화·행 캐시 |

계약 §1.A 의 `replay_bytes: u64` 는 `u32` 로 좁혔고(R 이탈 1), §1.A FE 의 "예산 차감"은 큐를 하나
더 두는 형태가 됐다(F 결정 1). 그 외 설계 이탈은 없다. 메인 2차 검증(`bun run verify` +
`bunx vite build`)은 계약 §2 대로 남아 있다.

### 검토·수정 (2026-09-07)

리뷰 결과 확정 major 0건 · 반박된 major 0건 · minor 1건 · info 4건.

| id | severity | 확정 | 처리 |
|----|----------|------|------|
| d56-1 | minor | 확정(부분) | **부분 수용** — 링크 후보 캐시가 부재(`null`)를 영구 보존한다는 사실은 맞다. 캐시 동작은 유지하고(사유 아래) 트레이드오프를 `terminal.md` §6 · provider JSDoc 에 명시했다. 회귀 테스트 1건 추가(부재 응답도 캐시). |
| d56-2 | info | 기록 | 문서가 번호를 매긴 이탈/결정은 R 3건 + F 3건 = 6건이 전부다. 지시문의 "7건"에 해당하는 7번째 미기록 이탈은 코드 대조에서도 나오지 않았다. 이번 절이 d56-1 을 7번째 항목으로 흡수한다. |
| d56-3 | info | 기록 | 같은 (cwd, 행) 의 응답 도착 전 재-hover 는 in-flight 캐시가 없어 resolver 를 중복 호출한다. 결과는 동일하고(같은 값을 덮어쓴다) IPC 1회가 낭비될 뿐이라 웨이브 2 후보로 남긴다. |
| d56-4 | info | 기록 | `line_aligned_eviction_end` 의 스캔 비용은 "한 번의 append 가 pty 읽기 청크(64KB) 를 넘지 않는다"는 호출부 전제에 기댄다. 전제는 R 이탈 3 에만 서술돼 있으므로, 리더 버퍼 크기를 키우는 변경이 오면 이 함수를 함께 점검한다. |
| F1 | info | 기록 | 캐시 키 구분자 `'\n'` 가 cwd 에도 나타날 수 있어 이론상 충돌 가능. cwd 에 개행이 든 디렉터리를 전제해야 하고 결과는 링크 오탐/누락뿐이라 현행 유지. |

**d56-1 을 코드로 고치지 않은 사유.** 부재만 캐시에서 빼면(포지티브 전용 캐시) 후보가 전부 파일이
아닌 행 — 버전 문자열·`host:port` 로그처럼 포인터가 가장 자주 지나는 행 — 이 hover 마다 IPC 와
파일시스템 stat 을 다시 낸다. 캐시를 둔 목적(§1.E "같은 행을 다시 hover 해도 IPC 가 나가지 않는다")이
정확히 그 경우를 위한 것이라 역전이다. TTL·버전 무효화는 "새 셸 명령 실행" 같은 신호를 provider
(`features`)까지 새 prop 으로 끌어와야 해 웨이브 1 minor 의 범위를 넘는다. 그래서 동작은 유지하고
① `docs/features/terminal.md` §6 에 트레이드오프 문단, ② `terminal-file-link.ts` provider JSDoc 에
같은 내용, ③ `terminal-file-link.test.ts` 에 "부재 응답도 캐시한다" 테스트로 의도를 고정했다.
무효화 신호 도입은 d56-3 과 함께 웨이브 2 에서 다룬다.

**변경 파일**: `src/features/terminal/terminal-file-link.ts`(JSDoc) ·
`src/features/terminal/terminal-file-link.test.ts`(테스트 순증 1) · `docs/features/terminal.md` §6 ·
이 문서.

**검증**: `cargo fmt --all -- --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`cargo test --workspace` · `bun run typecheck` · `bun run lint` · `bun run format:check` · `bun test`
전부 exit 0.
