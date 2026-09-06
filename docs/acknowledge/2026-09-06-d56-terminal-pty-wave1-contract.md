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

- (대기)
