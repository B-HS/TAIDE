# d-54 — 에이전트 활동 감지 개편: 세션 신호(출력·타이틀·다이얼로그·인밴드 훅) (2026-09-06)

> 사용자 보고: "Claude Code 의 상태 감지가 늦고, `Do you want to proceed?` 가 떠 있는데도 초록색 빈 원(유휴)
> 배지가 그대로다." 계측·바이너리 분석·실기 탐침으로 근본 원인을 확정하고 감지 계층을 재설계한다.
> 실행 방식은 `docs/agent-operations.md` §2 (구현 wf opus·xhigh → 렌즈 검토 sonnet·xhigh → 테스트 wf fable·medium).

## 0. 근본 원인 (증거)

### 0.1 현행 판정 구조 (`src-tauri/src/domain/agent`)

- `poll_agents`(500ms) 가 pty 전경 pgid 리더 pid 를 `ps -o pid=,state=,comm=,args=` 로 해석하고, `is_probe_active` 는
  **`state == 'R'`(CPU 실행 중) 만** 활동으로 본다(`service.rs:325-336`, `classify_activity` 히스테리시스 2s/6s).
- `AwaitingInput` 은 **hooks 브리지가 전담**한다(`docs/features/agent-integration.md` §7.2). hooks 는 `agentHooksEnabled`
  기본 **off** 의 opt-in 이고, 이 저장소의 `.claude/settings.local.json` 에도 TAIDE 마커가 0건이다 → 사용자가 본 배지는
  휴리스틱 단독 결과다. 휴리스틱에서 `AwaitingInput` 은 **구조적으로 불가능**하다.
- Claude Code 는 API 응답 대기·권한 대기·유휴 모두 프로세스 상태가 `S`(sleep) 이므로 6초 뒤 `Idle` 로 떨어진다. "늦다"
  가 아니라 "구분할 수 없다" 가 정확한 진단이다.
- hooks 를 켜도 두 결함이 남는다. ① Claude 의 `Notification(permission_prompt)` 은 **6초 타이머 뒤** 발화한다(바이너리
  2.1.263 확인: `setTimeout(..., fVe=6000)` — 6초 안에 답하면 알림 자체가 없다). ② override 키가 `(ProjectId, agent_name)`
  이라 같은 프로젝트의 두 세션이 서로를 덮는다(`docs/backlog.md` "hooks override 세션 단위 정밀화").

### 0.2 Claude Code 2.1.263 이 TAIDE PTY 에 실제로 내보내는 것 (expect 탐침, `TERM_PROGRAM=TAIDE`)

| 관측 | 사실 |
|------|------|
| 알림 채널 | `preferredNotifChannel=auto` 는 `TERM_PROGRAM` 이 Apple_Terminal/iTerm.app/kitty/ghostty 일 때만 BEL/OSC 9/99/777 을 낸다. `TAIDE` 는 `no_method_available` → **인밴드 알림 0건** |
| 타이틀(OSC 0) | 시작 `✳ Claude Code` → 작업 중 `◐ <요약>` / `◑ <요약>` 교대(약 1~2s) → 턴 종료 `✳ <요약>`. **권한 다이얼로그 동안은 마지막 ◐/◑ 값에서 동결**(240s 무갱신) |
| 출력 흐름 | 작업 중 100~250ms 간격 스피너 프레임 + 1s 경과 카운터. **권한 다이얼로그 동안에도 600ms 마다 48바이트(⏺ 점멸) 출력이 계속된다**. 유휴(프롬프트) 상태는 291s 동안 출력 0 |
| 다이얼로그 렌더 | 텍스트를 **단어 단위로 `CSI n G`(열 이동)를 끼워** 출력한다: `Do\e[5Gyou\e[9Gwant\e[14Gto\e[17Gproceed?`. 원시 바이트 문구 매칭은 불가능하고, CSI 를 걷어내고 열 이동을 공백으로 치환한 정규화 텍스트에서만 `Do you want to proceed?` 가 복원된다 |
| 다이얼로그 본문 | `Bash command` / `<명령>` / `<설명>` / `Do you want to proceed?` / `❯ 1. Yes` / `2. Yes, and always allow …` / `3. Yes, and switch to auto mode …` / `4. No` / `Esc to cancel · Tab to amend` |
| 기타 다이얼로그 문구(바이너리) | 계획 승인 `Would you like to proceed?`, 선택 다이얼로그 푸터 `Esc to cancel`(+`Enter to confirm`), 신뢰 다이얼로그 `Yes, I trust this folder`. 작업 중 푸터는 소문자 `esc to interrupt` 라 구분된다 |
| 훅 출력 | 2.1.141+ 는 hook JSON 출력의 `terminalSequence` 필드(OSC 0/1/2/9/99/777·BEL 허용 목록)를 Claude 가 자기 터미널에 대신 써 준다. `PermissionRequest` 훅은 다이얼로그 표시 시점에 **즉시** 발화하고 exit 0·결정 없음이면 흐름을 바꾸지 않는다 |

결론: (a) 훅 없이도 **타이틀 글리프 + 정규화 텍스트의 다이얼로그 시그니처 + 실질 출력 흐름**으로 Working/AwaitingInput/Idle 을 판정할 수
있고, (b) 훅은 **세션의 PTY 안으로 이벤트를 넣는 인밴드 방식**(OSC 777, urxvt 계열 공개 규격)으로 바꾸면 HTTP 서버·포트·토큰·cwd
매칭·프로젝트 단위 override 가 전부 사라지며 즉시성(PermissionRequest)도 얻는다.

## 1. 수정 방향

### 1.1 계층 (우선순위 위→아래)

1. **세션 이벤트(인밴드 훅)** — `\e]777;notify;taide-agent;{"v":1,"agent":"claude","event":"<e>"}\a` 를 pty 리더가 파싱. 이벤트 어휘:
   `permission_request`(PermissionRequest·Notification/permission_prompt) → 차단 래치 ON / `question_asked`(Notification/elicitation_dialog) → 래치 ON /
   `tool_complete`(PostToolUse) → 래치 OFF / `stop`·`stop_failure` → 래치 OFF + 유휴 힌트 / `idle_prompt` → 무시(상태 불변).
2. **타이틀 글리프(OSC 0/2)** — 첫 글자 `◐`/`◑` → Working 증거(TITLE_WORKING_FRESH_MS 동안 유효), `✳` → 유휴 힌트(단독으로 Idle 을 강제하지
   않는다 — tmux 아래에서는 정적 `✳` 타이틀이라 출력 흐름이 이겨야 한다).
3. **다이얼로그 시그니처(정규화 텍스트)** — 에이전트가 감지된 세션에서 새로 도착한 정규화 텍스트(직전 꼬리 TEXT_OVERLAP_BYTES 와 이어 붙임)에
   시그니처가 나타나면 차단 래치 ON. Claude 표: `Do you want to proceed?` · `Would you like to proceed?` · `Esc to cancel`(대소문자 정확 일치 —
   `esc to interrupt` 는 매치 금지 테스트). 표는 에이전트별 `&[&str]` 상수로 두어 확장 가능하되 **이번 배치는 Claude 만**(codex·gemini 문구는
   미검증이라 넣지 않는다 — 추측 금지).
4. **실질 출력 흐름** — 청크의 정규화 텍스트에서 공백·점멸/스피너 글리프(`⏺ ✶ ✻ ✽ ✢ ✳ ◐ ◑ ·`)를 뺀 문자가 SUBSTANTIVE_OUTPUT_MIN_CHARS 이상이면
   "실질 출력". 직전 사용자 입력(pty_write) 후 ECHO_SUPPRESS_MS 안의 출력은 에코로 보고 제외한다. 실질 출력이 ACTIVITY_WORKING_HOLD_MS 안에 있으면
   Working 증거. **실질 출력은 래치를 해제한다**(다이얼로그 대기 중에는 ⏺ 점멸만 흐르므로, 실질 출력 재개 = 다이얼로그가 화면에서 사라짐).
5. **사용자 입력**(pty_write 바이트 도착) — 차단 래치 OFF(사용자가 그 터미널을 보고 있다), 에코 억제 창 시작.
6. (하위 호환) Codex·Gemini 의 HTTP command hook 프로젝트 override — 세션 신호가 없을 때만 참고. Claude 의 HTTP 항목 수신은 이번 릴리스까지 유지하되
   설치는 더 이상 만들지 않는다.

### 1.2 판정 함수 (순수, 단위 테스트 대상) — `domain/agent/service.rs`

```
classify_session(signals, previous, now) -> AgentActivity
  blocked 래치 ON                                  → AwaitingInput
  타이틀 ◐/◑ 가 TITLE_WORKING_FRESH_MS 이내         → Working
  실질 출력이 ACTIVITY_WORKING_HOLD_MS 이내          → Working
  tool_complete 이벤트가 HOLD 이내                   → Working
  stop/stop_failure 이벤트 또는 타이틀 ✳ 힌트 뒤 HOLD 이상 조용함 → Idle
  조용함 ≥ ACTIVITY_IDLE_QUIET_MS                    → Idle
  신호 전무                                          → Unknown
  그 외                                              → previous (히스테리시스)
```

`AgentSessionSignals { last_substantive_output_at, last_input_at, title_glyph, title_seen_at, blocked: Option<BlockedSource{Event|Dialog}>,
last_event: Option<(AgentEvent, Instant)> }` 는 `AgentStore` 가 `session_id` 키로 보관한다. 갱신 지점: pty 리더 클로저(스캔 이벤트·정규화 텍스트),
`pty_write`(입력), 세션 종료(제거). `poll_agents` 는 기존 500ms 틱에서 `classify_session` 으로 `DetectedAgent.activity` 를 만들고 기존 `agents_changed`
diff 로 `agent:state-changed` 를 발행한다(IPC 계약 무변경). `ps -o state` 의 `R` 판정과 `activity_last_active`·`is_probe_active`·`classify_activity` 는 제거한다.

상수(`domain/agent/types.rs`, 근거는 이 문서 §0.2): `ACTIVITY_WORKING_HOLD_MS=2_000`(유지), `ACTIVITY_IDLE_QUIET_MS=4_000`(6_000→ 축소: 점멸 600ms·
스피너 ≤1s 보다 충분히 크고 유휴 진입이 빨라진다), `TITLE_WORKING_FRESH_MS=3_000`, `ECHO_SUPPRESS_MS=300`, `SUBSTANTIVE_OUTPUT_MIN_CHARS=2`,
`TEXT_OVERLAP_BYTES=128`, `AGENT_PROTOCOL_VERSION=1`, `AGENT_OSC_SENTINEL="taide-agent"`.

### 1.3 pty 출력 스캐너 통합 — `infra/terminal_scan.rs`(신설) — 리서치 후보 SCAN-1(T2-F6·ESC-2·ESC-5·AGT-6 병합)

- 세션당 `OutputScanner { carry: Vec<u8>, text_tail: String }` 하나를 `pty_spawn` 의 `on_data` 클로저가 소유한다(`command_started_at` 과 같은 캡처 수명).
  `scan(&mut self, chunk) -> ScanOutcome { events: Vec<ScanEvent>, text: String }`.
- **단일 패스**: `0x1b` 를 한 번만 찾아 `]`(OSC) 이면 숫자 ident 를 `;` 까지 읽고 `match` 로 분기 — `7` → `Cwd(String)`(현행 규칙: `file://` 스킵·BEL/ST 중
  먼저 오는 종결자), `133` → `CommandMarker`(C/D 만), `0|2` → `Title(String)`, `777` → `notify;<title>;<body…>`(body 는 `;` 재결합) 중 title 이
  `AGENT_OSC_SENTINEL` 인 것만 `AgentEvent`, `9` → `Notification9(body)`(첫 필드가 순수 숫자면 진행률 서브커맨드로 보고 무시). 그 외 ident 는 버린다.
- **경계 이월**: 종결자 없는 미완 시퀀스는 `carry` 에 남겨 다음 청크 앞에 붙인다. `carry.len() > MAX_OSC_PAYLOAD_BYTES(4096)` 이면 폐기(로그만).
- **하드닝**: 채택 문자열은 C0 제어문자(0x00-0x1F·0x7F) 제거 + 길이 상한(`MAX_TITLE_BYTES=512`, `MAX_AGENT_EVENT_BYTES=1024`) — 할당 전에 길이로 거절.
- **정규화 텍스트**(다이얼로그 시그니처·실질 출력용): OSC/DCS/APC/PM 문자열 제거, `CSI n G`·`CSI n C` → 공백 1개, `CSI … A/B/E/F/H/d/f` → 개행,
  나머지 CSI(SGR·K·J·?h/l 등)·`ESC ( ) = >` 제거, `\r` 제거, `\t` → 공백, 그 외 C0 제거, 연속 공백 1개로. 결과는 `ScanOutcome.text` 로 돌려주고
  스캐너는 마지막 `TEXT_OVERLAP_BYTES` 만 보관한다(대용량 출력에서 메모리 고정).
- 기존 `extract_latest_cwd`·`extract_command_markers` 는 스캐너로 대체하고 **그 44개 테스트는 의미를 유지한 채 스캐너 API 로 이식**한다(삭제 금지).
  추가 테스트: 두 청크에 걸쳐 잘린 `133;D` 가 다음 청크에서 방출 / carry 상한 초과 폐기 / 한 청크에 7·133·0·777·9 가 섞여도 순서대로 / 단어 단위 열 이동
  프레임(§0.2 실물)이 `Do you want to proceed?` 로 정규화 / `esc to interrupt` 는 시그니처가 아님 / 점멸 48바이트 청크는 실질 출력이 아님.
- `perf::CounterSlot` 에 스캔 이벤트 카운터 1개 추가(기존 `PtyOutputBytes` 옆).
- `docs/ipc-contract.md` 의 "청크 경계 미재조립은 의도된 한계" 서술을 정정한다.

### 1.4 Claude hooks 를 인밴드 command hook 으로 — `domain/agent/{service,hooks,commands}.rs`

- 설치 대상은 그대로 `.claude/settings.local.json`(프로젝트, opt-in 토글·동의 UI 유지). 항목은 `type:"command"` 이고 **정적 문자열 printf** 다(jq·CLI·서버 의존 0):

  | 훅 | matcher | event |
  |----|---------|-------|
  | `PermissionRequest` | — | `permission_request` |
  | `Notification` | `permission_prompt` | `permission_request` |
  | `Notification` | `elicitation_dialog` | `question_asked` |
  | `Notification` | `idle_prompt` | `idle_prompt` |
  | `PostToolUse` | — | `tool_complete` |
  | `Stop` | — | `stop` |
  | `StopFailure` | — | `stop_failure` |

  `UserPromptSubmit`·`SessionStart` 는 넣지 않는다(두 훅은 stdout 을 컨텍스트로도 해석하는 이벤트라 위험을 만들 이유가 없고, Working 은 타이틀·출력이 담당).
- 명령 형태(순수 함수 `build_claude_agent_hook_command(event, emitter) -> String`, 인용 규칙 단위 테스트):
  - `TerminalSequence` 방출: `if [ -n "$TAIDE_AGENT_PROTOCOL_VERSION" ]; then printf '%s' '{"terminalSequence":"]777;notify;taide-agent;{\"v\":1,\"agent\":\"claude\",\"event\":\"permission_request\"}"}'; fi`
  - `DevTty` 방출(구버전·버전 미상): `if [ -n "$TAIDE_AGENT_PROTOCOL_VERSION" ]; then printf '\033]777;notify;taide-agent;{"v":1,"agent":"claude","event":"permission_request"}\007' > /dev/tty 2>/dev/null; fi`
  - 항상 exit 0. env 게이트 덕에 iTerm 등 TAIDE 밖에서는 아무것도 내지 않는다(리서치 AGT-3).
- 방출 방식 선택: 설치·reconcile 시 `claude --version`(블로킹 풀, 3초 타임아웃, 앱 실행당 캐시)을 읽어 `>= 2.1.141` 이면 `TerminalSequence`, 그 외(미만·PATH 에
  없음·실패)는 `DevTty`. 비교는 순수 함수 + 상수 `CLAUDE_TERMINAL_SEQUENCE_MIN_VERSION`.
- 마커: 기존 `HOOKS_URL_MARKER("taide=1")` 외에 `AGENT_OSC_SENTINEL("taide-agent")` 도 TAIDE 관리 항목으로 인식하도록 `is_taide_managed_entry`/
  `has_taide_marker_anywhere` 를 넓힌다. `reconcile_installed_hooks` 는 프로젝트 파일에 TAIDE 항목이 있는데 **현재 기대 명령 집합과 다르면**(구 http 항목 포함)
  제거 후 재주입한다(멱등). uninstall 은 두 마커 모두 제거.
- PTY env: `agent::commands::agent_protocol_env() -> Vec<(String,String)>` 가 `TAIDE_AGENT_PROTOCOL_VERSION=1`·`TAIDE_APP_VERSION=<cargo pkg version>` 을
  돌려주고 `lib.rs::pty_spawn_env_provider` 가 `editor_terminal_env` 뒤에 이어 붙인다. 원격 미러·다른 표면 무관(pty 스폰에만).
- HTTP 서버·`apply_hook_payload`·Codex/Gemini 사용자 레벨 command hook(`taide hook --url`)은 **변경하지 않는다**. `apply_hook_payload` 의 `agent=claude`
  분기는 이번 릴리스까지 유지(구버전 설치본 호환)하고 §4 에 제거 예정으로 남긴다.
- 프론트: 설치 UI·토글·로케일 키는 기존 그대로. 버전 미상으로 `DevTty` 가 선택돼도 사용자에게 알리지 않는다(동작 차이 없음).

### 1.5 프로세스 신원 해석 비용 — `poll_agents`

- `ps` 는 이제 **이름 해석에만** 쓴다. `AgentStore` 에 `pid -> Option<&'static str>` 캐시를 두고, 현재 전경 pid 집합에 없는 항목은 매 틱 축출한다.
  이미 캐시된 pid 는 `ps` 를 부르지 않으므로 fork 는 세션의 전경 pid 가 바뀐 틱에만 일어난다(백로그 M-3 잔여의 대부분 해소; sysinfo 교체(T6-F5)는 별도 결정).
- `DetectedAgentProbe.state` 와 `parse_ps_process_infos` 의 state 열은 제거(사용처 소멸). Windows 경로도 `windows_activity_state` 제거.

### 1.6 문서

- `docs/features/agent-integration.md` §1(감지 신호 4종·판정 함수·상수), §4(인밴드 훅 형식·env 게이트·버전 선택), §7.1·§7.2 재작성.
- `docs/features/terminal.md` §5 에 스캐너(단일 패스·이월·정규화 텍스트) 절 추가. `docs/ipc-contract.md` 청크 경계 서술 정정·`agent:state-changed` 발행
  근거 갱신. `docs/backlog.md` "hooks override 세션 단위 정밀화" 해결 처리, "poll_agents 의 ps fork" 잔여 축소 기록.
- `docs/bug/2026-09-06-agent-badge-idle-during-permission-prompt.md`(증상·원인·해결) — 메인이 작성.

### 1.7 범위 외 (후속 후보, `docs/research/2026-09-06-terminal-agent-deep-dive.md`)

Codex/Gemini 다이얼로그 시그니처·인밴드 전환(AGT-4 에이전트별 분기), 단계/차단 사유 노출(AGT-2·7·9), Ctrl-C 취소 유예창(AGT-8), tmux passthrough(AGT-5),
sysinfo 전환(T6-F5)·전경 그룹 확대(T6-F6), 훅 기본값 변경(제품 결정), OSC 9/777/BEL 을 OS 알림·탭 주의로 승격(ESC-1/3/6), 타이틀→탭 제목(ESC-10).

## 2. 실행 계획

- 구현 wf(opus·xhigh) — Rust 는 한 시점 한 에이전트. **A**(스캐너: §1.3, 리더 배선, 테스트 이식) → **B**(§1.2 신호·판정·§1.4 훅·§1.5 캐시·env, 테스트)
  → **C**(문서 §1.6). TS 변경은 없다(bindings 무변경 확인 — `bun run typecheck`).
- 각 단계 종료 조건: `cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`cargo test --workspace` exit 0, 이 계약 §3 에 기록.
- 렌즈 검토 wf(sonnet·xhigh): ① 근본성(휴리스틱이 증상 덮기가 아닌지·상수 근거) ② 회귀(44개 이식 테스트·OSC 7/133 동작·pause/attach 경로·hooks 멱등)
  ③ 경계(신뢰할 수 없는 pty 출력·길이 상한·env 게이트·settings.local.json 최소 개입). major 는 적대적 검증.
- 테스트 wf(fable·medium): Rust 단위 보강 + e2e 시나리오 초안(수동 실기 체크리스트 `docs/quality-assurance/2026-09-06-agent-activity-qa.md`).
- 메인 2차 검증: `bun run verify` + `bunx vite build` + 탐침 스크립트로 실물 프레임 재생 테스트 확인 → 커밋(auto-commit 합의) → dev 푸시 → main ff.

## 3. 기록 (구현·검토·검증 — 실시간 동기)

### A 단계 — pty 출력 스캐너 통합 (§1.3) · 완료

변경 파일

- `src-tauri/src/infra/terminal_scan.rs` (신설) — `OutputScanner { carry, text_tail }` · `scan(&mut self, chunk) -> ScanOutcome`
  · `scan_once(bytes)`(무상태 편의 함수) · `ScanEvent{Cwd,CommandMarker,Title,AgentEvent,Notification9}`.
  단일 패스(ESC 1회 탐색 → `]`/`[`/`P X ^ _`/`( )` 분기), 경계 이월(미완 시퀀스를 `carry` 로), 정규화 텍스트
  (`CSI G/C`→공백, `CSI A/B/E/F/H/d/f`→개행, 그 외 CSI·OSC·DCS/APC/PM·`ESC ( ) = >` 제거, `\r` 제거, `\t`→공백,
  C0 제거, 연속 공백 1개). 상수: `MAX_OSC_PAYLOAD_BYTES=4096` · `MAX_TITLE_BYTES=512` · `MAX_AGENT_EVENT_BYTES=1024`
  · `TEXT_OVERLAP_BYTES=128` · `AGENT_OSC_SENTINEL="taide-agent"`.
- `src-tauri/src/infra/shell_integration.rs` — `extract_latest_cwd`·`extract_command_markers`·`earliest_terminator`
  ·`find_subslice`·`OSC7_PREFIX`·`OSC133_PREFIX`·`OSC_STRING_TERMINATOR`·`OSC_BEL`·`OSC7_FILE_URI_SCHEME` 제거(스캐너로 흡수).
  `CommandMarker` 와 셸 스크립트 조립은 그대로. 끊긴 rustdoc 링크 1건 갱신.
- `src-tauri/src/domain/terminal/commands.rs` — `pty_spawn` 의 `on_data` 가 `parking_lot::Mutex<OutputScanner>` 를
  `command_started_at` 과 같은 수명으로 캡처하고, 분기는 `dispatch_scan_outcome(app, session_id, started_at, &outcome)`
  한 곳으로 모았다. 이 단계에서는 `Cwd`(청크의 마지막 것만 — 기존 `extract_latest_cwd` 의미 보존) → `report_cwd_change`,
  `CommandMarker` → `report_command_marker` 만 처리하고 나머지 variant 는 명시적 no-op arm.
- `src-tauri/src/infra/perf.rs` — `CounterSlot::PtyScanEvents`(`pty.scan_events`) 추가(enum·`ALL`·`name()` 3곳 동기).
- `src-tauri/src/domain/agent/types.rs` — `AGENT_OSC_SENTINEL` 을 `pub use crate::infra::terminal_scan::AGENT_OSC_SENTINEL`
  로 재export(§3 이탈 1).
- `src-tauri/src/infra/mod.rs` — `pub mod terminal_scan`.

테스트 — `infra::terminal_scan::tests` 33개 (이식 18 + 신규 15). 워크스페이스 총 1,454개 통과.

- 이식(이름·단언 유지, 호출부만 스캐너 API 로): `extract_latest_cwd는_*` 8건, `extract_command_markers는_*` 10건.
- 신규: 청크 경계에서 잘린 `133;D` 의 다음 청크 방출 / 이월 상한 초과 폐기 후 복구 / 한 청크의 `7·133·0·777·9` 순서 보존 /
  실물 권한 다이얼로그 프레임(§0.2, `claude-raw3.log` 의 `Bash command` 주변 조각) 정규화 → `Do you want to proceed?`·
  `Esc to cancel` 복원 / 작업 중 푸터 `esc to interrupt` 는 다이얼로그 문구와 구별 / 48바이트 `⏺` 점멸 청크는 공백·글리프뿐 /
  `\x1b]0;◐ taide-probe-file 생성\x07` → `Title` / OSC 777 센티널·서브커맨드 불일치 무시 / 본문 `;` 재결합 /
  타이틀·에이전트 이벤트 길이 상한 거절 / 채택 문자열 C0 제거 / OSC 9 숫자 첫 필드 무시 / 정규화 규칙 / 텍스트 꼬리 이어붙임·상한.

검증 — `cargo fmt --all --check` exit 0 · `cargo clippy --workspace --all-targets -- -D warnings` 경고 0 ·
`cargo test --workspace` 1,454 passed / 0 failed · `bun run typecheck` exit 0(bindings 무변경).

이탈

1. **`AGENT_OSC_SENTINEL` 의 정의 위치** — §1.2 는 `domain/agent/types.rs` 에 두라고 했으나, `tests/domain_boundaries.rs` 의
   `infra는_화이트리스트_밖의_domain_참조를_가질_수_없다` 가 `infra → domain` 참조를 차단한다(architecture.md §2). 화이트리스트에
   등재하는 대신 그 가드가 지시하는 방향대로 **정의를 `infra::terminal_scan` 에 두고 `domain::agent::types` 가 `pub use` 로 재export**
   했다. 계약이 적은 경로(`domain::agent::types::AGENT_OSC_SENTINEL`)는 그대로 유효하므로 B 단계의 훅 빌더는 영향이 없다.
2. **`ScanOutcome` 에 필드 1개 추가** — §1.3 의 `{ events, text }` 에 `overlap: String` 을 더했다. §1.1.3 의 다이얼로그 시그니처는
   "직전 꼬리 + 이번 텍스트" 를 봐야 하고 §1.1.4 의 실질 출력은 "이번 청크 텍스트만" 을 세야 하는데, 둘을 하나의 `text` 로 합쳐
   돌려주면 꼬리(최대 128B)가 매 청크의 실질 출력으로 중복 계산돼 `SUBSTANTIVE_OUTPUT_MIN_CHARS=2` 를 항상 넘긴다. 두 대상 문자열을
   분리해 돌려주고 이어붙이기는 소비처(B)가 필요할 때만 하도록 했다.
3. **`Cwd` 이벤트의 소비 규칙** — 스캐너는 청크 안의 모든 OSC 7 을 순서대로 방출하지만(단일 패스 원칙), `dispatch_scan_outcome` 은
   `ScanOutcome::latest_cwd()` 로 **마지막 것만** 적용한다. 전부 순서대로 적용하면 한 청크에 두 번의 프롬프트 렌더가 실린 경우
   `TerminalCwdChanged` 가 중간값까지 발행돼 기존 동작이 바뀐다. 커맨드 마커보다 cwd 를 먼저 적용하는 순서도 종전과 같다.

미결 — 이 단계 범위 밖

- `docs/features/terminal.md` §5 스캐너 절 · `docs/ipc-contract.md` 의 "청크 경계 미재조립은 의도된 한계" 정정 ·
  `docs/debugging.md` 의 카운터 목록에 `pty.scan_events` 추가는 **C 단계(§1.6)** 에서 처리한다.
- `ScanEvent::{Title, AgentEvent, Notification9}` 과 `ScanOutcome::{text, overlap}` 은 아직 소비처가 없다(B 단계에서 배선).

### B 단계 — 세션 신호 판정 · 인밴드 훅 · pid 캐시 · env (§1.2·§1.4·§1.5) · 완료

변경 파일

- `src-tauri/src/domain/agent/types.rs` — 상수: `ACTIVITY_IDLE_QUIET_MS` 6_000→4_000, `TITLE_WORKING_FRESH_MS=3_000`,
  `ECHO_SUPPRESS_MS=300`, `SUBSTANTIVE_OUTPUT_MIN_CHARS=2`, `AGENT_PROTOCOL_VERSION=1`,
  `AGENT_PROTOCOL_VERSION_ENV_NAME`/`APP_VERSION_ENV_NAME`, `CLAUDE_TERMINAL_SEQUENCE_MIN_VERSION=(2,1,141)`,
  `CLAUDE_VERSION_TIMEOUT_SECONDS=3`, `HOOK_EVENT_STOP_FAILURE`, `NOTIFICATION_MATCHER_*` 3종,
  `CLAUDE_MANAGED_HOOK_EVENTS`(구 `MANAGED_HOOK_EVENTS` 대체 — 신규 5종 + 레거시 `UserPromptSubmit` 합집합),
  스캐너 재export 확대(`OSC_NOTIFY_IDENT`·`OSC_NOTIFY_SUBCOMMAND`·`TEXT_OVERLAP_BYTES`). `HOOK_HANDLER_TYPE_HTTP` 제거.
  `AgentActivity`·`DetectedAgent` IPC 표면 무변경.
- `src-tauri/src/domain/agent/service.rs` — 순수 판정 계층 신설: `AgentEvent`(+`wire_name`/`from_wire`) · `BlockedSource` ·
  `TitleGlyph` · `AgentSessionSignals` · `parse_agent_event_body` · `parse_title_glyph` · `CLAUDE_DIALOG_SIGNATURES` ·
  `find_dialog_signature` · `is_substantive_output` · `apply_scan_to_signals` · `note_input` · `classify_session`.
  훅 빌더: `HookEmitter` · `build_claude_agent_hook_command` · `claude_hook_entries`(7행 표) ·
  `inject_taide_claude_command_hook_entries` · `claude_hook_entries_match` · `installed_claude_entries` ·
  `parse_claude_version` · `supports_terminal_sequence`. `is_probe_active`·`classify_activity`·`inject_taide_hook_entries`
  ·`has_hook_entries_for_url`·`entry_has_url` 제거, `ProcessInfo.state`·`DetectedAgentProbe.state` 제거
  (`DetectedAgentProbe.name` 은 `&'static str` 로), 마커 인식은 `TAIDE_HOOK_MARKERS`(`taide=1`·`taide-agent`) 2종으로 확대.
- `src-tauri/src/domain/agent/commands.rs` — `AgentStore` 에 `signals`(세션별 신호)·`process_names`(pid→이름 캐시, unix) 추가,
  `compute_activity`·`prune_activity`·`activity_last_active` 제거. `classify_session_activity` · `record_scan` ·
  `record_input` · `prune_signals` · `unresolved_pids`/`remember_process_names`/`probes_for`/`retain_process_names`.
  `detect_agents_for_pids_blocking(agents, pids)` 가 캐시에 없는 pid 만 `ps -o pid=,comm=,args=` 로 조회하고,
  `resolve_activity` 는 `classify_session` 결과를 쓰되 신호 전무 + 비-claude 일 때만 hooks override 를 참고한다.
  `record_session_scan`/`record_session_input`(옵저버 진입점) · `agent_protocol_env` · `resolve_claude_hook_emitter`
  (`claude --version` blocking + 3초 타임아웃 + `OnceLock`) 추가. windows 의 `windows_activity_state` 제거.
  `agent_hooks_install` 의 Project 분기는 HTTP URL 대신 인밴드 command 항목을 쓴다.
- `src-tauri/src/domain/agent/hooks.rs` — `reconcile_claude_project_hooks`(신설) 가 이미 설치된 프로젝트만 모아
  기대 집합과 다르면 제거 후 재주입하고, HTTP 서버 기동은 codex·gemini 사용자 레벨 훅 쪽으로만 남았다.
  `apply_hook_payload`·`build_hook_url`·서버 자체는 무변경.
- `src-tauri/src/domain/terminal/commands.rs` — `PtySessionSignal{Output,Input}`·`PtySessionObserver(s)` 신설,
  `dispatch_scan_outcome` 끝과 `pty_write` 시작부에서 `notify_session_observers` 호출. `pty_write` 가 `app: AppHandle` 을 받는다
  (specta 는 `AppHandle` 을 인자로 내지 않으므로 `bindings.ts` 무변경).
- `src-tauri/src/domain/remote/dispatch.rs` — `pty_write` 호출에 `app.clone()` 추가(시그니처 동기).
- `src-tauri/src/lib.rs` — `pty_session_observers()` 조립·`app.manage`, `pty_spawn_env_provider` 에 `agent_protocol_env` 연결.
- `src-tauri/src/infra/terminal_scan.rs` — `OSC_NOTIFY_IDENT`·`OSC_NOTIFY_SUBCOMMAND` 를 `pub` 로(방출부·판독부 단일 정의).

테스트 — 신규 33개(제거 6개), 워크스페이스 총 1,511개 통과(lib 1,481).

- 판정: 스피너+출력→Working / 다이얼로그 시그니처→AwaitingInput 래치 / 점멸 청크 10회(6초)에도 래치 유지 /
  실질 출력 재개→해제·Working / 사용자 입력→해제 / 입력 후 299ms 출력은 에코(300ms 는 아님) / 타이틀 ✳+2s→Idle /
  ✳ 타이틀이어도 출력이 흐르면 Working(tmux) / 힌트 없이 4s→Idle·중간 구간은 previous / permission_request→래치 /
  tool_complete·stop→해제 / idle_prompt 무시 / 다른 agent 필드 무시 / `esc to interrupt` 비매치 / 청크 경계 문구 복원 /
  **이미 지나간 다이얼로그 꼬리는 재래치하지 않음** / 시그니처는 꼬리 상한보다 짧음 / 신호 전무→Unknown / 글리프·본문 파서 /
  **실물 pty 바이트**(다이얼로그 질문 행 + 48바이트 `⏺` 점멸 프레임)를 스캐너에 넣어 래치→유지까지 한 번에.
- 훅: 명령 문자열 2종 정확 일치 / 셸 인용 안전성 / 7행 표 / `sh -c` 실행 → stdout 이 유효 JSON 이고 `terminalSequence` 를
  스캐너가 `AgentEvent` 로 되읽음 / env 미설정 시 0바이트·exit 0(두 방출기) / 버전 파서·최소 버전 경계.
- 파일: 빈 설정 주입·사용자 항목 보존·2회 주입 멱등·방출기 전환 시 교체 / `hooks` 가 객체·배열이 아닌 파손 파일 /
  구버전 http 설치 인식→재주입 시 `UserPromptSubmit` 소멸 / 제거 시 두 마커 모두 정리(hooks.rs 포함).
- pid 캐시: 에이전트가 아니라는 답도 캐시되고 전경 집합 이탈 시 축출 / `ps` 파싱은 state 열 없이 동작.

검증 — `cargo fmt --all -- --check` exit 0 · `cargo clippy --workspace --all-targets -- -D warnings` 경고 0 ·
`cargo test --workspace` 1,511 passed / 0 failed · `bun run typecheck` exit 0(`bindings.ts` 무변경).

이탈

1. **터미널→에이전트 배선은 assembly-owned 옵저버로** — 계약 §1.2 는 "갱신 지점: pty 리더 클로저·`pty_write`" 만 정하고 방식은
   열어 두었다. `domain/terminal/commands.rs → agent::commands` 직접 호출은 `tests/domain_boundaries.rs` 의 도메인 간 참조
   가드에 걸리고(이미 있는 `agent → terminal::commands` 와 합쳐 순환이 된다), 화이트리스트 등재 대신 이 저장소가 같은 문제를
   푼 방식(`PtySpawnEnvProvider`·`SettingsToggleObservers`)을 따라 `PtySessionObservers` 를 두고 `lib.rs` 가 두 도메인을 잇는다.
2. **`AgentSessionSignals` 에 필드 2개 추가** — §1.2 스케치에 `agent_name: &'static str` 과 `idle_hint_at: Option<Instant>` 를
   더했다. 전자는 pty 리더 스레드에서 청크마다 에이전트별 표를 고르기 위한 O(1) 조회용(없으면 매 청크 `agents` 맵 전수 탐색),
   후자는 §1.2 의 "stop·stop_failure 또는 타이틀 ✳ 힌트 뒤 HOLD 이상 조용" 규칙을 표현하는 자리다. 후자에는 OSC 9 알림도
   기록한다(단계 지시의 "Notification9 는 유휴 힌트로만") — 힌트 단독으로는 절대 Idle 을 강제하지 않으므로 tmux 정적 타이틀과
   같은 안전성을 가진다.
3. **훅 명령 끝에 `exit 0` 을 붙인다** — §1.4 의 예시 문자열에는 없지만 같은 절이 "항상 exit 0" 을 요구한다. `> /dev/tty` 는
   제어 터미널이 없으면 리다이렉트 자체가 실패해 비영 종료가 되고, 그러면 Claude 트랜스크립트에 훅 실패로 뜬다.
4. **`terminalSequence` 봉투는 `serde_json` 으로 생성** — ESC·BEL 의 JSON 이스케이프와 내부 `\"` 를 손으로 적지 않는다. 인코더가
   Claude 의 디코더와 같은 규칙이라 이스케이프가 어긋날 수 없고, 결과에 작은따옴표가 없어 셸 단일 인용이 안전하다(테스트로 고정).
5. **claude 는 hooks project override 를 더 이상 참고하지 않는다** — 단계 지시대로 "신호 전무 + 비-claude" 에만 fallback 한다.
   구버전 HTTP 설치가 남아 있는 claude 세션은 `apply_hook_payload` 가 즉시 발행하는 상태 변경은 그대로 받지만, 다음 500ms 틱에
   세션 신호 판정으로 덮인다. reconcile 이 그런 프로젝트를 인밴드 항목으로 교체하므로 과도기만의 동작이다.
6. **`inject_taide_hook_entries`·`has_hook_entries_for_url` 제거** — claude 가 command 훅으로 옮겨가면서 프로덕션 사용처가
   0이 되었다(내 변경으로 생긴 죽은 코드). 구버전 http 설치 픽스처는 테스트에서 `json!` 로 직접 만든다.
7. **꼬리 결합 매칭은 "경계를 가로지르는 경우만"** — §1.1.3 은 "직전 꼬리와 이어 붙인 텍스트에 시그니처가 나타나면 래치" 라고
   적었지만, 그대로 구현하면 **사용자가 답한 뒤에도 래치가 다시 걸린다**: 꼬리(최대 128B)에는 방금 지나간 다이얼로그 문구가
   그대로 남아 있고, 점멸 청크(비실질 출력)가 흐르는 동안 꼬리가 밀려나가는 데 십수 초가 걸리므로 §1.1.4 의 "실질 출력은 래치를
   해제한다" 가 매 청크 무효화된다(= 고치려던 증상과 같은 화면). 그래서 각 시그니처마다 꼬리·머리를 그 시그니처 길이-1 로 잘라
   이어 붙여, **꼬리에 통째로 들어 있는 문구는 매치되지 않게** 했다. 잘린 문구 복원(계약의 의도)은 그대로 동작하며 두 방향 모두
   테스트로 고정했다.

미결 — 이 단계 범위 밖

- 문서(§1.6)는 C 단계: `features/agent-integration.md` §1·§4·§7 재작성, `features/terminal.md` §5 스캐너 절,
  `ipc-contract.md` 청크 경계 서술 정정, `debugging.md` 의 `pty.scan_events`, `backlog.md` 의 override·ps fork 항목 갱신.
- `AgentEvent::QuestionAsked`·`BlockedSource` 의 구분은 아직 UI 로 나가지 않는다(§1.7 의 AGT-2·7·9).
- `resolve_claude_hook_emitter` 는 앱 실행당 1회 캐시라 실행 중 Claude 를 업그레이드해도 재판정하지 않는다. `DevTty` 로 굳어도
  동작 차이는 없어(§1.4) 재프로브 트리거는 두지 않았다.

### C 단계 — 문서 갱신 (§1.6) · 완료

변경 파일 (문서 전용, 코드 변경 0)

- `docs/features/agent-integration.md`
  - **§1 전면 재작성** — "신원(§1.1) / 활동 신호 4종(§1.2) / `classify_session` 우선순위(§1.3) /
    상수 표(§1.4)" 로 나눴다. `ps` 가 이름 해석 전용이 되고 pid 캐시로 fork 가 줄었다는 것,
    `state` 열 제거의 이유(모든 대기가 `S`), 폴링 주기 정정(낡은 "unix 1s" → `AGENT_POLL_UNIX_MS`
    500ms), 신호별 효과 표, 다이얼로그 시그니처가 정규화 텍스트에서만 복원되는 이유,
    대소문자 정확 일치, 꼬리 결합이 경계를 가로지르는 경우로 좁혀진 이유, 에코 억제,
    "실질 출력이 래치를 먼저 푼다" 순서, 상수 9개의 값·근거를 실물 심볼명으로 적었다.
  - **§4 전면 재작성** — 제목을 "hooks 브리지"로 바꾸고 §4.1(인밴드 command hook·7행 표·
    `UserPromptSubmit`/`SessionStart` 제외 이유·`PermissionRequest` 즉시성) / §4.2(명령 형태 —
    env 게이트·정적 printf·항상 `exit 0`·봉투 인코딩) / §4.3(방출기 선택·마커 2종·멱등 재조정) /
    §4.4(codex·gemini HTTP 유지, claude 는 override 불참조, 수신 분기 제거 예정) /
    §4.5(statusline 미구현) 으로 나눴다. 두 방출기의 **실제 설치 명령 문자열**을 그대로 실었다.
  - **§7.1 재작성**(세 상수의 의미 — hold/quiet 의 새 근거, `HOOK_OVERRIDE_STALE_MS` 는 이제
    codex·gemini 전용), **§7.2 재작성**(제목을 "세션 신호의 수집"으로 — 옵저버가 리더 스레드에서
    동기로 도는 제약, 신호 레코드 생성 시점의 공백, `agent_name` 을 레코드에 든 이유, 꼬리 결합을
    좁힌 근거, `now: Instant` 주입), **§7.3 갱신**(제거된 `inject_taide_hook_entries` →
    `inject_taide_claude_command_hook_entries`, 마커 2종).
  - 낡은 서술 "`AwaitingInput` 판정은 hooks 브리지가 전담한다" 는 §7.2 에서 제거됐다.
- `docs/features/terminal.md`
  - **§5.2 신설** — "pty 출력 스캐너 `infra/terminal_scan.rs`": 소유(세션당 1개·`on_data` 캡처),
    단일 패스 분기표, 종결자는 BEL/ST 중 이른 쪽인 이유, 경계 이월과 4096바이트 상한, 할당 전
    길이 거절(512/1024)·C0 제거, 정규화 텍스트 규칙 전부, 꼬리 128바이트를 `text` 와 분리해
    돌려주는 이유, `dispatch_scan_outcome` 의 소비 순서, `pty.scan_events` 계측.
  - §5 의 "청크 경계 재조립은 2차" 서술과 §9 의 `extract_latest_cwd`·`extract_command_markers`
    참조를 현행 스캐너로 정정, §11 범위표에서 "OSC7 cwd 추적 고도화(청크 경계 재조립 등)" 를
    2차에서 완료 칸으로 옮겼다.
- `docs/ipc-contract.md`
  - terminal 절: **"청크 경계에서 잘린 시퀀스는 감지되지 않을 뿐 — 의도된 한계" 를 정정**했다
    (d-54 로 재조립됨, 1회성 이벤트는 자가 치유가 없다는 근거 포함). 한 청크의 OSC 7 은 마지막
    것만 적용한다는 규칙도 명시. `terminal:command-finished` 의 스캔 주체도 갱신.
  - agent 절: 폴링 주기 정정(500ms)과 **`agent:state-changed` 의 `activity` 판정 근거** 항목을
    신설했다 — 타입·이벤트·발행 조건은 무변경이고 달라진 것은 근거와 "hooks 없이도
    `AwaitingInput` 이 나온다" 는 점임을 명시.
- `docs/debugging.md` — `counters` 표에 `pty.scan_events` 행 추가(A 단계 미결 이관분).
- `docs/backlog.md` — "hooks override 세션 단위 정밀화" 행을 취소선 + **해결(d-54)** 로,
  "`poll_agents` 의 `ps` fork 제거" 행에 pid 캐시로 축소된 현황과 "회귀 위험 근거였던 `state` 판정이
  사라졌다" 를 기록(표 형식 유지).

검증 — `cargo fmt --all -- --check` exit 0 · `cargo clippy --workspace --all-targets -- -D warnings`
경고 0 · `cargo test --workspace` **1,511 passed / 0 failed**(lib 1,481 + 통합 30, C 단계에서 코드
변경이 없어 B 단계와 동일) · `bunx prettier --check` 로 편집한 md 5개 통과.

이탈

1. **§1.6 이 지정한 3개 파일 외에 2개를 더 손댔다** — `docs/debugging.md`(A 단계가 C 로 넘긴
   `pty.scan_events` 카운터)와 `docs/features/agent-integration.md` §7.3(제거된
   `inject_taide_hook_entries` 를 가리키고 있었다). 둘 다 이 배치의 변경으로 **사실과 어긋나게 된**
   서술이라 같은 갱신 단위로 처리했다.
2. **§1.6 이 "§7.1·§7.2" 만 적었으나 §1 의 폴링 주기 서술도 정정**했다(문서 "unix 1s" ↔ 코드
   `AGENT_POLL_UNIX_MS = 500`). 이 배치가 만든 불일치는 아니지만 §1 을 재작성하며 남겨 둘 수 없었다.
3. **훅 명령의 제어 바이트는 `<ESC>`·`<BEL>` 표기로 적었다.** 문서 편집 파이프라인이 JSON 유니코드 이스케이프
   같은 표기를 실제 제어 바이트로 바꿔 넣어(md 파일에 보이지 않는 0x1b·0x07 이 들어감) 그대로는
   실을 수 없었다. 대신 바로 아래에 "실제 파일에는 `printf` 용 백슬래시 표기 또는 JSON 유니코드
   이스케이프 6글자가 들어간다" 를 명시해 의미 손실을 막았다. 이 사고를 되돌리며
   `git checkout HEAD -- docs/features/agent-integration.md` 를 1회 실행했다(단계 지시의 "파일 수정은
   Edit/Write 로만" 에 대한 예외 — 내가 만든 오염을 원본으로 되돌리기 위한 복원이고, 이후 내용은
   전부 Edit 로 다시 작성했다).
4. **`docs/research/2026-09-06-terminal-agent-deep-dive.md` 는 손대지 않았다** — 그 문서는 배치
   착수 시점의 조사 기록이라 사후 수정 대상이 아니다(현행 사실은 위 정본 문서들이 말한다).
   `ipc-contract.md` 의 d-50 배치 기록(§"에이전트 폴링(§2 M-3)")도 같은 이유로 유지했다.

미결 — 이 단계 범위 밖

- `docs/quality-assurance/2026-09-06-agent-activity-qa.md`(수동 실기 체크리스트)는 테스트 wf 몫이다.
- 프론트 문서(`layout-shell.md` §2.2 배지)는 표시 규칙이 그대로라 갱신 대상이 아니다.

### §3 총괄 (A·B·C)

- **테스트 순증 +42** — A 단계 +15(이식 18 은 이름·단언을 유지한 이동이라 순증이 아니다),
  B 단계 +27(신규 33 · 제거 6), C 단계 0. 워크스페이스 총 **1,511 passed / 0 failed**(lib 1,481).
- **상수 최종값** — `ACTIVITY_WORKING_HOLD_MS=2_000` · `ACTIVITY_IDLE_QUIET_MS=4_000`(6_000 에서 축소)
  · `TITLE_WORKING_FRESH_MS=3_000` · `ECHO_SUPPRESS_MS=300` · `SUBSTANTIVE_OUTPUT_MIN_CHARS=2`
  · `TEXT_OVERLAP_BYTES=128` · `MAX_OSC_PAYLOAD_BYTES=4096` · `MAX_TITLE_BYTES=512`
  · `MAX_AGENT_EVENT_BYTES=1024` · `AGENT_PROTOCOL_VERSION=1` · `AGENT_OSC_SENTINEL="taide-agent"`
  · `CLAUDE_TERMINAL_SEQUENCE_MIN_VERSION=(2,1,141)` · `CLAUDE_VERSION_TIMEOUT_SECONDS=3`
  · `HOOK_OVERRIDE_STALE_MS=900_000`(codex·gemini 전용으로 축소) · `AGENT_POLL_UNIX_MS=500`(유지).
- **이탈 15건** — A 3(스캐너 상수 위치·`ScanOutcome.overlap` 추가·`Cwd` 는 마지막 것만 적용),
  B 7(assembly-owned 옵저버·`AgentSessionSignals` 필드 2개·`exit 0`·`serde_json` 봉투·claude 의
  override 불참조·죽은 코드 제거·꼬리 결합 축소), C 4(위 1~4). 계약의 의도를 바꾼 것은 B-7 하나이며
  (그대로 구현하면 고치려던 증상이 재현된다), 나머지는 표현·배치 수준의 이탈이다.
- **계약 범위 밖으로 남긴 것** — §1.7 의 후속 후보 전부, `AgentEvent::QuestionAsked`·
  `BlockedSource` 의 UI 노출, codex·gemini 다이얼로그 시그니처·인밴드 전환, `sysinfo` 교체.

### D 단계 — 렌즈 검토 반영 (§2 의 ①②③ 렌즈) · 완료

발견 11건(major 1 · minor 6 · info 4). 확정 major 1건은 전부 수용했고, minor 는 계약·컨벤션에
부합하는 5건을 수용(2건은 문서만), 1건을 기각했다.

| id | severity | 확정 | 처리 |
|----|----------|------|------|
| `f1-echo-window-suppresses-dialog-signature` | major | 확정(2인 반박 실패) | **수용** — 에코 억제를 실질 출력 판정에만 적용 |
| `R4-substantive-output-clears-event-latch-same-chunk` | minor | — | **수용** — 같은 청크의 이벤트 래치는 실질 출력이 지우지 못한다 |
| `R3-signals-agent-name-pinned-on-first-detection` | minor | — | **수용** — 세션의 에이전트가 바뀌면 신호 레코드를 새로 만든다 |
| `f2-stale-agent-name-on-same-tick-agent-handoff` | info | — | **수용** — R3 과 동일 원인·동일 수정으로 해소 |
| `R1-dead-code-clear-project-override` | minor | — | **수용** — 호출부 0인 `clear_project_override` 삭제 |
| `F2-broadened-marker-collision-risk` | minor | — | **수용** — 소유 판정 마커를 `AGENT_OSC_MARKER`(`notify;taide-agent;`)로 좁힘 |
| `f3-doc-cold-start-gap-claim-vs-trust-dialog` | minor | — | **부분 수용** — 코드 변경 없이 §7.2 의 단정을 미확인으로 정정 |
| `F1-osc777-forgery-unauthenticated` | minor | — | **부분 수용** — 수용된 위험임을 문서에 명시, nonce 는 §4 후속 결정 |
| `R2-pid-name-cache-never-revalidates-while-continuously-foreground` | minor | — | **기각**(사유 아래) + §4 후속 결정 |
| `F3-claude-version-probe-thread-leak` | info | — | 기록만 |
| `F4-unicode-format-chars-not-stripped` | info | — | 기록만 |

변경 파일

- `src-tauri/src/domain/agent/service.rs` — `apply_scan_to_signals` 의 순서를 "한 청크 안에서는
  차단 증거가 이긴다" 로 정리했다. ① 에코 가드(`is_echo`)는 실질 출력 분기 안으로만 들어가고
  다이얼로그 시그니처 검사는 창과 무관하게 항상 돈다(f1). ② `apply_agent_event` 가
  `Option<bool>`(적용 여부 / 차단 여부)을 돌려주고, **그 청크의 이벤트가 방금 세운 래치는** 실질
  출력이 지우지 않는다(R4). ③ `TAIDE_HOOK_MARKERS` 의 인밴드 마커를 `AGENT_OSC_SENTINEL` 에서
  `AGENT_OSC_MARKER` 로 교체(F2).
- `src-tauri/src/infra/terminal_scan.rs` — `AGENT_OSC_MARKER`(`notify;taide-agent;`) 추가.
  `const` 는 결합이 불가능해 리터럴로 두고, 구성요소(`OSC_NOTIFY_SUBCOMMAND`·`AGENT_OSC_SENTINEL`)
  와의 일치를 테스트로 고정했다.
- `src-tauri/src/domain/agent/types.rs` — `AGENT_OSC_MARKER` 재export.
- `src-tauri/src/domain/agent/commands.rs` — `classify_session_activity` 가 레코드의 `agent_name`
  과 이번 틱에 감지된 이름이 다르면 레코드를 새로 만든다(R3·f2). `clear_project_override` 삭제(R1).

테스트 — 신규 6개, 의미 변경 1개. 워크스페이스 총 **1,517 passed / 0 failed**(lib 1,487).

- `입력_직후의_출력은_에코로_보고_무시한다` → `입력_직후의_출력은_에코로_보고_실질_출력에_세지_않는다`
  로 **의미를 바꿨다**. 옛 테스트는 에코 창 안의 다이얼로그 문구가 무시되는 것을 "의도" 로 고정하고
  있었는데, 그것이 f1 이 지적한 실패 모드 자체다. 지금은 비-시그니처 텍스트로 에코 창의 본래 목적
  (사용자가 친 글자를 에이전트 출력으로 세지 않는다)만 검증한다.
- 신규: 답변 직후 에코 창 안에 그려진 다이얼로그도 래치하고 이후 점멸 10회에도 유지된다(f1 재현) /
  같은 청크의 `permission_request` 는 실질 출력에 지워지지 않는다(R4) / 같은 청크의 `tool_complete`
  뒤에 온 다이얼로그는 다시 래치한다(R4 의 반대 방향) / 센티널을 경로 문자열로만 담은 사용자 hook 은
  TAIDE 소유가 아니다(F2) / 마커는 서브커맨드+센티널의 조합이다(F2 단일 출처) / 세션의 에이전트가
  바뀌면 이전 에이전트의 래치를 버린다(R3).

검증 — `cargo fmt --all -- --check` exit 0 · `cargo clippy --workspace --all-targets -- -D warnings`
경고 0 · `cargo test --workspace` 1,517 passed / 0 failed · `bun run typecheck` exit 0.

문서

- `docs/features/agent-integration.md` — §1.2 의 에코 억제 항목을 "실질 출력에만 적용" 으로 정정하고
  한 청크 안의 우선순위(차단 증거 우선)를 새 항목으로 적었다. §1.4 상수 표에 `AGENT_OSC_MARKER` 행
  추가. §4.1 에 **인밴드 채널이 인증되지 않는다는 수용된 위험**을 명시(F1). §4.3 의 마커 서술을
  새 마커와 그 이유(사용자 항목 오판 방지)로 교체. §7.2 의 콜드스타트 공백 서술에서 "다이얼로그가
  뜨는 시점과 겹치지 않는다" 는 단정을 지우고 신뢰 다이얼로그가 미확인임을 적었다(f3). 같은 절에
  에이전트 교체 시 레코드 재생성(R3)도 기록.
- `docs/features/terminal.md`·`docs/ipc-contract.md` — 이번 수정으로 어긋난 서술이 없어 무변경
  (스캐너 분기·`agent:state-changed` 판정 근거는 그대로다).

기각 — `R2-pid-name-cache-never-revalidates-while-continuously-foreground`

- 지적 자체(같은 pid 위에서 `exec` 로 프로세스 이미지가 바뀌면 캐시가 영원히 낡는다)는 사실이다.
  그러나 §1.5 는 축출 규칙을 "현재 전경 pid 집합에 없는 항목은 매 틱 축출" 로 **명시**했고 B 단계
  테스트가 음성 캐시를 그 규칙대로 고정했다. 제안된 해법은 둘 다 계약 밖이다 — `(pid, 시작시각)`
  키는 캐시 히트마다 `ps` 를 다시 불러 캐시의 목적을 되돌리고, TTL 은 §3 총괄의 상수 표에 없는
  새 튜닝값을 도입한다.
- 영향도 국지적이다. 재현에는 사용자가 `exec`(또는 그런 런처)를 쓰는 것이 전제이고, 회복 경로
  (탭을 닫았다 여는 것)가 있으며, 발견자 스스로 confidence 를 low 로 적었다. 상수·프로브 정책
  변경은 사용자 결정으로 §4 에 남긴다.

기록만 (info)

- `F3-claude-version-probe-thread-leak` — `claude --version` 프로브의 3초 타임아웃은 대기만 끊고
  자식 프로세스·블로킹 스레드를 죽이지 않는다. PATH 의 `claude` 가 끝나지 않는 프로그램일 때
  스레드 1개가 남는다. 앱 실행당 1회(`OnceLock`)라 누수 상한이 1이고, 고치려면 `Child` 핸들 유지 +
  `kill` 로 프로브 구조를 바꿔야 해 이번 배치의 수정 범위 밖으로 둔다.
- `F4-unicode-format-chars-not-stripped` — 정규화 텍스트는 C0·DEL 만 제거하고 유니코드 포맷
  문자(U+202E·제로폭 등)는 통과시킨다. 현재 소비처가 배지 판정(부분 문자열 매치)뿐이라 관측 가능한
  영향이 없다. `ScanOutcome.text`/`overlap` 을 로그·UI 로 내보내는 소비처가 생기면 그 작업의 선행
  조건으로 `strip_control` 을 유니코드 포맷 범주까지 넓힌다.

### E 단계 — 테스트 (§2 테스트 wf: 리플레이 회귀 + 수동 QA) · 완료

변경 파일

- `src-tauri/src/domain/agent/service.rs` — 기존 `tests` 모듈 안에 `scenario_timeline` 서브모듈 신설(프로덕션 코드 무변경).
  실물 pty 바이트 픽스처 10종(`RAW_TITLE_START/WORKING_LEFT/WORKING_RIGHT/IDLE_END` · `RAW_SPINNER_GLYPH_FRAME` ·
  `RAW_SPINNER_COUNTER_FRAME` · `RAW_INPUT_ECHO_FRAME` · `RAW_DIALOG_QUESTION` · `RAW_BLINK_FRAME` · `RAW_TURN_DONE_LINE`)을
  `claude-raw2.log`(작업→유휴 턴)·`claude-raw3.log`(권한 다이얼로그)에서 바이트 리터럴로 옮겼다(각 ≤ 200B, 세션 URL 등
  식별 조각 없음). `Replay { base, scanner, signals, activity }` 가 가상 시계(`Instant + Duration`)로 스캐너 → `apply_scan_to_signals`
  → `classify_session` 을 순서대로 돌리며 이전 판정을 `previous` 로 되먹인다.
- `docs/quality-assurance/2026-09-06-agent-activity-qa.md` (신설) — §1 픽스처 / §2 hooks 없는 휴리스틱 단독(다이얼로그 1초 안
  노란 마름모 → 답하면 파란 점 → 초록 빈 원, 다른 프로젝트 탭에서 배지만 보기, 연쇄 승인, 타이핑 무영향, `esc to interrupt`,
  Ctrl-C 4초) / §3 hooks 켠 프로젝트(7행·명령 형태·멱등·즉시성·트랜스크립트 무경고·제거) / §4 환경 게이트(iTerm 외부 터미널,
  env 확인, tmux, codex·gemini 는 Working/Idle 만 — 다이얼로그 미지원 명시, 두 세션 독립, 탭 닫기) / §5 탐침 스크립트
  (`probe-claude-permission.exp` 인자·산출물·확인 명령, 프로젝트 밖 임시 디렉터리 실행) / §6 종료 조건.

테스트 — 신규 3개. 워크스페이스 총 **1,520 passed / 0 failed**(lib 1,490 + 통합 30).

- `실물_시나리오_타임라인을_재생하면_각_단계의_배지가_계약대로_판정된다` — (a) `✳ Claude Code` 만: 100ms 는 `Unknown`(previous),
  HOLD 뒤 `Idle` / (b) `note_input` 뒤 299ms 에코 프레임: `last_substantive_output_at == None`, 배지 `Idle` 유지 / (c) `◐` 타이틀 +
  120ms × 10 스피너(카운터·글리프 교대) → `Working` / (d) 열 이동 다이얼로그 프레임 → 즉시 `AwaitingInput` / (e) 48바이트 `⏺` 점멸
  600ms × 10 = 6s(≥ `ACTIVITY_IDLE_QUIET_MS` 단언) 동안 매 틱 `AwaitingInput`, 6.5s 에도 유지 / (f) 키 입력 → 래치 해제, `◑` + 카운터
  프레임 → `Working` / (g) `✳` 타이틀 + 완료 행 뒤 HOLD-1ms 는 `Working`, HOLD 에 `Idle` / (h) OSC 777 `permission_request` 와 카운터
  프레임을 **한 청크**로 → `BlockedSource::Event` 유지(R4)·`AwaitingInput`, 점멸 후에도 유지, `tool_complete` → 해제·`Working` /
  (i) `note_input` 뒤 299ms 새 다이얼로그 프레임 → `AwaitingInput`(f1 회귀).
- `실물_스피너_글리프_프레임만으로는_실질_출력이_아니다` — 글리프 1자 프레임은 비실질, `(3s · thinking with xhigh effort)` 카운터
  프레임은 실질(§0.2 의 "스피너 프레임 + 1s 경과 카운터" 를 분리해 고정).
- `실물_타이틀_시퀀스는_시작_작업_종료_글리프로_해석된다` — 실물 OSC 0 4종이 `Idle → Working → Working → Idle` 글리프로 읽힌다.

검증 — `cargo fmt --all -- --check` exit 0 · `cargo clippy --workspace --all-targets -- -D warnings` 경고 0 ·
`cargo test --workspace` 1,520 passed / 0 failed · `bunx prettier --check` QA 문서 통과.

이탈

1. **픽스처를 별도 상수로 재선언했다** — 기존 `실물_pty_바이트로_래치가_걸리고_점멸_동안_유지된다` 의 `RAW_DIALOG_QUESTION`·
   `RAW_BLINK_FRAME` 은 그 테스트 함수 안의 지역 상수라 재사용이 불가능했다. 기존 테스트를 고쳐 끌어올리는 대신(최소 변경)
   같은 바이트를 `scenario_timeline` 모듈 상수로 다시 적었다. 정규화 텍스트 픽스처(`DIALOG_TEXT`·`BLINK_TEXT`)와 헬퍼
   (`at`·`claude_signals`)는 부모 `tests` 모듈 것을 그대로 쓴다.
2. **(c) 의 "스피너 프레임 = 실질 출력" 은 절반만 사실이다** — 실측 로그에서 글리프 1자만 바꾸는 프레임(`✶`·`✻`…)은
   `SUBSTANTIVE_OUTPUT_MIN_CHARS=2` 미만이라 비실질이고, 실질 출력은 1초 경과 카운터 행(`(3s · thinking …)`)에서 나온다.
   시나리오는 두 프레임을 교대로 넣어 실물 흐름을 재현했고, 두 번째 테스트가 이 구분을 따로 고정했다. 타이틀 `◐/◑` 가
   함께 오므로 판정 결과(`Working`)는 계약 그대로다.
3. **가시성 문제 없음** — 필요한 항목(`agent_osc_sequence`·`claude_agent_event_payload`·`OutputScanner`)은 모두 테스트에서 이미
   닿는 경로였다. 프로덕션 코드는 1바이트도 바꾸지 않았다.

## 4. 후속 · 미결 결정 (사용자)

1. hooks 기본값 — 현행 opt-in 유지(추천: 서드파티 파일 개입은 동의가 전제) / 첫 에이전트 감지 시 1회 제안 토스트 / 기본 on.
2. Claude HTTP 훅 수신 분기 제거 시점 — 다음 릴리스(추천) / 즉시.
3. Codex·Gemini 인밴드 전환(`taide hook --tty`) 착수 여부 — 리서치 AGT-4 의존.
4. 인밴드 이벤트 인증(D 단계 F1) — 현행 무인증 유지(추천: pty 출력 위조는 OSC 133 과 같은 등급의
   기존 수용 위험이고, 효과가 배지·알림에 그친다) / 세션별 nonce 를 `agent_protocol_env()` 로 발급해
   훅 페이로드에 싣고 수신 시 대조(와이어 포맷 `v` 증가).
5. pid 이름 캐시 재검증(D 단계 R2) — 현행 유지(추천: `exec` 로 같은 pid 위에서 이미지가 바뀌는
   경우에만 낡고, 탭 재열기로 회복된다) / 재검증 TTL 상수 도입 / 캐시 키에 프로세스 시작시각 포함.
