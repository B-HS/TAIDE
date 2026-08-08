# 2026-08-08 — AI 상태 배지 반영 지연 분석·개선 결정 사항

> QA 피드백: "claude 의 상태 반영 뱃지가 반영은 되는데 조금 늦게 반영된다"
> 대상: `src-tauri/src/domain/agent/`, `src-tauri/src/lib.rs`(폴링 틱 호출부만), `src/entities/agent/`

## 1. 지연 경로 전수 분석

| 구간 | 코드 위치 | 즉시성 |
|------|-----------|--------|
| (a) 감지 폴링 주기 | `domain/agent/types.rs` `AGENT_POLL_UNIX_MS`(기존 1000ms) / `AGENT_POLL_WINDOWS_MS`(2000ms) | 폴링 틱마다 재평가 |
| (a) 활동 판정 히스테리시스 | `domain/agent/types.rs` `ACTIVITY_WORKING_HOLD_MS`(2000ms) / `ACTIVITY_IDLE_QUIET_MS`(6000ms), `service::classify_activity` | 의도된 디바운스 |
| (b) hooks 이벤트 → emit | `domain/agent/hooks.rs` `handle_connection` → `apply_hook_payload` → `AgentStateChanged::emit` | **폴링과 무관하게 즉시 emit** (HTTP 수신 직후 동기 처리, 확인 완료 — 코드 변경 없음) |
| (c) 프론트 구독 → 배지 리렌더 | `src/entities/agent/agent.query.ts` `useAgentStateSync`(Tauri event → `setQueryData`), `src/app/query-client.ts`(refetchInterval 없음, staleTime 60s는 이벤트 경로에 영향 없음) | **이벤트 즉시 반영, 폴링/네트워크 왕복 없음** (확인 완료 — 코드 변경 없음) |
| (d) 프로세스 스캔 자체 | `domain/agent/commands.rs` `detect_agents_for_pids`(unix `ps` subprocess, windows `sysinfo::System::new_all()`) | 기존에 `poll_agents`(lib.rs 틱 루프)에서 **tokio 워커 스레드를 블로킹**하며 동기 호출됨 — `agent_list` 커맨드는 이미 `spawn_blocking`으로 격리되어 있었으나 틱 루프는 아니었음(비대칭) |

**핵심 발견**: (b)·(c)는 이미 즉시성이 보장되어 있어 변경 불필요. 지연의 실질적 원인은 (a)의 폴링 틱 자체(작동 시작은 최대 1폴링 틱, 종료는 `ACTIVITY_IDLE_QUIET_MS + 최대 1폴링 틱`)와, (d)가 틱 타이머와 같은 스레드풀을 공유해 틱 자체를 지연시킬 수 있던 구조적 리스크였다.

## 2. 적용한 개선

### 2.1 `detect_agents_for_pids_blocking` 신설 — 틱 루프의 스레드풀 점유 제거

`domain/agent/commands.rs`에 `detect_agents_for_pids_blocking`(spawn_blocking 래퍼)을 추가하고, `agent_list` 커맨드와 `lib.rs`의 `poll_agents` 양쪽에서 재사용한다(2회 이상 사용 → 공통화, common.md §3.3). 기존에는 `agent_list`만 `spawn_blocking`을 썼고 `poll_agents`(1초/2초마다 도는 틱)는 동기 호출이라 `ps` 서브프로세스 spawn(측정: 이 개발 머신에서 `ps -o comm=,state=,args=` 1회 약 1.5~2ms) 동안 tokio 워커 스레드를 점유했다. 터미널 수가 늘거나 시스템이 바쁠 때 이 점유가 hooks HTTP 서버의 accept 루프 등 같은 런타임의 다른 작업까지 지연시킬 수 있었다. `poll_agents`를 `async fn`으로 바꾸고 `.await`로 spawn_blocking에 위임해 이 리스크를 제거했다(`lib.rs`는 틱 루프 호출부 두 줄만 최소 수정 — 로직 자체는 owned 파일인 `domain/agent/commands.rs`에 있음).

### 2.2 `AGENT_POLL_UNIX_MS` 1000ms → 500ms (근거 기록 — 상수 변경 필수 근거)

- **비용 근거**: `ps -o comm=,state=,args= -p $$`를 20회 반복 실행해 측정한 결과 평균 1.5~2ms/회(이 macOS 개발 머신 기준). 터미널이 여러 개 열려 있어도(예: 5개) 틱당 총 10ms 내외로, 500ms 주기에서 CPU 비중은 무시할 수준이다.
- **2.1의 전제**: spawn_blocking으로 옮겨 틱 타이머 자체가 이 비용 때문에 지연되지 않으므로, 주기를 줄여도 실행기 경합 위험이 없다.
- **효과**: 휴리스틱 경로의 이론 지연 상한이 절반으로 준다(아래 3절).
- **windows는 변경하지 않음**: `AGENT_POLL_WINDOWS_MS`(2000ms)는 `sysinfo::System::new_all()`(전체 프로세스 열거)을 쓰는데, 이 비용을 이 환경에서 측정할 수 없어 "비용 근거 없이 줄이지 않는다" 원칙에 따라 보수적으로 유지했다. windows 환경에서 프로파일링 데이터가 확보되면 재검토 권장.
- **히스테리시스 상수(`ACTIVITY_WORKING_HOLD_MS`/`ACTIVITY_IDLE_QUIET_MS`)는 변경하지 않음**: A3 결정 문서(`docs/acknowledge/2026-08-07-a3-agent-activity-decisions.md`)가 밝힌 대로, CPU/state 신호는 "API 응답 대기 중 sleep"과 "진짜 idle"을 구분하지 못해 의도적으로 보수적인 디바운스를 둔 값이다. 실사용 텔레메트리 없이 이 값을 줄이면 false-idle 플리커 회귀 위험이 있어, 이번 변경 범위에서는 건드리지 않았다.

## 3. pty 출력 활동 기반 즉시 전이 — 검토 후 미구현 (근거)

작업 지시의 우선순위 2번("pty 출력 수신 시점에 상태 전이를 트리거하는 경로 검토")을 검토했다.

- **구조적 제약**: A3 결정 문서에서 이미 동일한 결론에 도달했다 — 이 신호를 구현하려면 `infra/pty.rs`(PtySession에 출력 타임스탬프 추가)뿐 아니라 `domain/terminal/commands.rs`(TerminalStore가 그 값을 agent 도메인에 노출하는 접근자 추가)까지 함께 수정해야 한다. `TerminalStore`의 내부 세션 맵은 private이라 agent 도메인이 우회할 방법이 없다. 이번 작업의 소유 파일 목록에도 `domain/terminal/`은 포함되어 있지 않다.
- **신규로 발견한 정확성 리스크**: pid 매칭 문제를 session_id 키로 우회하더라도, PTY가 방출하는 바이트를 그대로 "활동"으로 쓰면 **idle 상태에서도 스피너/경과시간 등을 다시 그리는 TUI(예: ink 기반 CLI)의 화면 재도장 자체가 계속 "최근 출력 있음"으로 잡혀 idle 전이를 영구히 막을 수 있다.** 이걸 안전하게 걸러내려면 ANSI 시퀀스를 인지한 "내용이 실제로 늘었는지" 판정이 필요한데, 이는 이번 범위를 크게 벗어난다.
- **결정**: 이번 웨이브에서는 구현하지 않는다. `domain/terminal`·`infra/pty.rs` 양쪽을 함께 소유하고 출력 diff 판정까지 설계하는 별도 웨이브로 남긴다(A3 문서의 권고와 동일한 결론 재확인).

## 4. 개선 후 이론 지연 상한 (전이별, unix 기준)

| 전이 | 경로 | 이전 | 이후 |
|------|------|------|------|
| Idle/Unknown → Working | 휴리스틱(`ps` R 상태) | 최대 1000ms | 최대 **500ms** |
| Idle/Unknown → Working | hook(`UserPromptSubmit`) | ~0ms (hooks 활성화·설치 시) | 변경 없음 |
| Working → Idle | 휴리스틱(`ACTIVITY_IDLE_QUIET_MS` + 폴링 틱) | 최대 6000+1000=7000ms | 최대 6000+**500**=**6500ms** |
| Working → Idle | hook(`Stop`) | ~0ms (hooks 활성화·설치 시) | 변경 없음 |
| → AwaitingInput | hook(`Notification`)에서만 발행, 휴리스틱은 이 상태를 만들지 않음 | ~0ms (hooks 활성화·설치 시), 미설치 시 구조적으로 노출 안 됨 | 변경 없음 |
| 틱 타이머 신뢰성 | `poll_agents`가 `ps` spawn 동안 tokio 워커 스레드를 블로킹 → 부하 시 틱 자체가 밀릴 위험(비결정적, 상한 없음) | 리스크 있음 | **제거**(spawn_blocking으로 격리) |

**참고**: `agent_hooks_enabled` 기본값은 `false`(`domain/settings/types.rs:167`, settings 도메인 — 이번 작업 범위 밖)라 대부분의 사용자는 휴리스틱 경로만 탄다. hooks를 기본 활성화하면 시작/종료 전이 모두 사실상 즉시(수 ms) 반영되지만, 이는 이번 작업의 소유 파일 밖(settings 도메인) 결정이라 변경하지 않았다 — 필요하면 별도 합의 후 진행.
