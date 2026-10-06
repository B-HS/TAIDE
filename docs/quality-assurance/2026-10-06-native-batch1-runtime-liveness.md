# Native 배치 1 단계 2 — 터미널·에이전트 동작 결함 수정 (2026-10-06)

상태: T1~T5 구현을 마쳤고 V3·V4·V5·V6(cargo check, native-terminal fmt)는 exit 0 입니다. V2 와 V6 의 native-app fmt 검사는 이번 변경과 무관한 기존 상태 때문에 exit 0 을 만들 수 없었습니다(6절). GUI 는 실행하지 않았으므로 8절의 실기 확인이 남아 있습니다.

경로는 저장소 루트(`/Users/hyunseokbyun/development/TAIDE`) 기준입니다.

## 1. 기준 동작 확인 (TS·Tauri)

| 항목 | 기준 동작 | 근거 |
| --- | --- | --- |
| T1 흐름 제어 | 미처리 출력이 `HIGH_WATER_BYTES`(512 KiB)를 넘으면 PTY 읽기를 pause, `LOW_WATER_BYTES`(64 KiB) 아래로 내려가면 resume. 과부하는 오류가 아닙니다 | `src/shared/constants/terminal.ts:1-3`, `src/widgets/terminal-pane/terminal-flow-control.ts:10-14`, `src/widgets/terminal-pane/terminal-pane.tsx:127-131, 199-209`, `src/features/terminal/terminal-view.tsx:403-412`, `src/widgets/terminal-pane/terminal-session.tsx:215-218` |
| T1 pause 지점 | reader thread 가 다음 read 전에 gate 에서 대기하고, kill·child 종료는 gate 를 영구 개방 | `crates/taide-infra/src/pty.rs:299-322, 379-381, 664-666`, `crates/taide-terminal/src/store.rs:206-210`, `crates/taide-runtime/src/terminal_actions.rs:204-206` |
| T2 fit | `ResizeObserver` 가 포커스와 무관하게 fit 하고, 0px 또는 2열·1행 이하만 거절 | `src/features/terminal/terminal-view.tsx:88-99, 330, 336-341, 396`, `src/widgets/terminal-pane/terminal-session.tsx:195-199` |
| T3 오류 표시 | write·resize·pause 실패는 삼키고(`.catch(() => undefined)`), 링크·spawn 실패만 자동으로 사라지는 toast. 터미널 안에 남는 오류 문구는 없습니다 | `src/widgets/terminal-pane/terminal-session.tsx:106, 177-182, 187, 198, 212, 217, 220-222, 237` |
| T4 프로젝트 닫기 | 닫는 프로젝트의 PTY 를 `kill_project` 로 회수. Tauri 에는 native Hub 에 해당하는 별도 보관소가 없습니다 | `src-tauri/src/domain/terminal/capability.rs:12-24`, `crates/taide-terminal/src/store.rs:287-301`, `crates/taide-runtime/src/project_actions.rs:458-496` |
| T5 에이전트 폴링 | `TaskSupervisor` 의 `"agent-poll"` 작업이 Unix 500ms·Windows 2000ms 주기로 `poll_agents` 호출. 입력은 `TerminalStore::foreground_pids` 와 `detect_agents_for_pids_blocking` | `src-tauri/src/lib.rs:322-324, 1077-1089`, `src-tauri/src/domain/agent/commands.rs:300-317`, `crates/taide-agent/src/constants.rs:1-2`, `crates/taide-runtime/src/agent_actions.rs:135-174`, `crates/taide-runtime/src/agent_host.rs:83-98` |

T2 의 포커스 조건이 들어간 이유: `docs/bug/2026-10-03-native-terminal-*.md` 11건에는 resize 와 포커스를 묶은 결함 기록이 없습니다(`resize` 검색 결과는 delete-lines-history, hidden-wheel 의 다른 문맥뿐). 근거는 `docs/quality-assurance/2026-10-02-m8-native-terminal-surface.md:10` 의 "focused view만 resize를 요청합니다 … 여러 창의 단일 controlling-view 정책은 아직 미완료" 와 `2026-10-02-m8-native-terminal-settings.md:12` 의 "unfocused resize 동등성은 아직 미완료" 입니다. 즉 같은 세션을 여러 뷰가 볼 때의 임시 소유 규칙이었고, 되살리면 안 되는 별도 결함은 확인되지 않았습니다.

## 2. 항목별 변경

### T1. 출력 과부하 backpressure

- `native/taide-native-app/src/terminal_frames.rs`
    - `HIGH_WATER_BYTES`(512 KiB), `LOW_WATER_BYTES`(64 KiB): TS 상수와 같은 이름·값입니다.
    - `Watermarks::new`: 고수위는 `min(HIGH_WATER_BYTES, 한도/2)`, 저수위는 `min(LOW_WATER_BYTES, 고수위/8)` 입니다. 앱 한도(4 MiB)에서는 TS 값 그대로 512 KiB / 64 KiB 가 됩니다. 개수 한도는 native 큐에만 있는 제약이라 TS 상수가 없으므로 같은 비율(한도/2, 그 1/8)로 유도했습니다(앱 한도 64 → 32 / 4).
    - `Backlog`, `BacklogLease`: 큐에 들어간 프레임의 무게와 개수를 세고, `Delivery` 가 drop 될 때 반납합니다.
    - `Shared::pause_above_high_water`, `Shared::release`, `Shared::resume`: 고수위를 넘으면 `flow(true)`, 저수위 아래로 내려가면 `flow(false)` 를 한 번씩 호출합니다(`evaluateFlowControl` 과 같은 전이). 전이와 호출을 같은 mutex 안에서 처리해 pause·resume 순서가 뒤바뀌지 않습니다.
    - `Shared::fail`: 수신자 소멸·순서 오류 등 실제 실패에서는 보류를 풀어 reader 가 실패 경로로 빠져나가게 합니다. 실패 자체는 기존과 같이 `Closed`·`Sequence` 등으로 남고 세션을 종료시킵니다.
    - `channel_with_flow(limits, flow)`: 흐름 제어 port 를 받는 생성자입니다. 기존 `channel(limits)` 과 `Sender::submit` 의 서명·오류 종류는 그대로입니다.
- `native/taide-native-terminal/src/session.rs`
    - `OutputGate`(`set_paused`, `retire`, `wait_while_paused`): `pty.rs` 의 `PauseGate` 와 같은 계약(pause, 영구 개방, 대기)입니다.
    - `SharedTerminal::set_output_paused`: 공개 진입점입니다.
    - `SharedTerminal::feed_with_delivery`: PTY callback 이 chunk 를 코어에 넣기 전에 gate 에서 기다립니다. publication lock 을 잡기 전에 기다리므로 actor 의 `flush_sync_if_due`, resize, UI 의 색 설정이 막히지 않습니다.
    - `State::fail`: 모든 실패 경로에서 gate 를 영구 개방합니다(`DeliveryGuard`, `fail_and_stop`, resize·parser 실패 포함).
    - 같은 함수가 PTY batch 를 코어의 feed 한도(`TerminalCore::feed_limit`, 64 KiB) 단위로 나눠 넣습니다. `pty.rs` 의 batch 는 64 KiB 에 도달한 뒤 flush 하므로 최대 128 KiB 미만까지 커질 수 있는데, 코어는 64 KiB 초과 입력을 거절하고 세션이 `Failed(Parser)` 로 죽었습니다. 과부하에서 세션이 죽는 두 번째 경로라 함께 고쳤습니다(7절 1번).
- `native/taide-native-terminal/src/lib.rs`: `TerminalCore::feed_limit`.
- `native/taide-native-app/src/terminal_host.rs`: `spawn_display_with_initial_sink` 가 `channel_with_flow` 로 큐를 만들고 port 를 `SharedTerminal::set_output_paused` 에 연결합니다.

`PtySession::set_paused` 를 직접 쓰지 않은 이유는 세 가지입니다.

1. `PauseGate` 는 세션당 bool 하나이고 원격 클라이언트의 `pty_set_paused` 가 같은 값을 씁니다. TS 뷰는 attach 때마다 `setPaused(false)` 를 보내므로(`terminal-pane.tsx:199-209`) 원격 접속이 native 의 pause 를 풀어 버립니다. native 큐는 xterm 버퍼와 달리 한도가 있어 이 경우 다시 세션이 죽습니다.
2. `PauseGate` 는 reader 의 다음 read 에서만 적용되고 flusher thread 가 한 batch 를 더 넘길 수 있어 한도 직전에서 정확하지 않습니다. callback 안에서 기다리면 보류 뒤 출력 프레임이 0개입니다.
3. `PtySession` 은 `TerminalStore` 소유라 store 등록 전 구간에는 닿을 수 없습니다.

원격의 `pty_set_paused` 경로는 건드리지 않았고, 두 gate 가 모두 열려 있어야 출력이 흐릅니다.

### T2. 포커스와 무관한 resize

- `native/taide-native-app/src/terminal_surface.rs`
    - `Views::is_shown_elsewhere`: 같은 세션을 그리고 있는 다른 뷰가 있는지 봅니다. 뷰가 마지막으로 그려진 pass 번호(`View::processed_pass`)를 해당 viewport 의 `cumulative_pass_nr_for` 와 비교합니다. 탭을 다른 pane·창으로 옮긴 뒤 남은 뷰 상태는 그려지지 않으므로 제외됩니다. `cumulative_frame_nr_for` 는 viewport 가 없으면 debug 빌드에서 panic 하므로 쓰지 않았습니다.
    - `show_with_keymap`: 조건을 `running && (포커스 || 다른 뷰 없음) && ui.is_enabled()` 로 바꿨습니다. 단일 뷰는 포커스 없이 측정 크기를 요청하고, 같은 세션을 여러 뷰가 그리면 기존 규칙대로 포커스 뷰만 요청합니다. 측정 불가 크기 거절(`measured`, 3열·2행 미만)과 세션당 요청 1건 제한(`resizing`)은 그대로입니다.

### T3. 지워지지 않는 오류 문구

- `native/taide-native-app/src/terminal_surface.rs`
    - `submit_input`: 포커스 보고가 아닌 입력이 큐에 받아들여지면 `view.error` 를 지웁니다.
    - `Outbox::poll`: write receipt 가 성공하면 `outbox.error` 를 지우고 실패하면 새 오류로 바꿉니다.
    - `show_with_keymap`: `outbox.error` 를 `view.error` 에 복사해 고정하던 코드를 없애고, 그릴 때 `view.error` 가 없으면 `outbox.error` 를 봅니다.
- 해제 기준을 "입력이 다시 받아들여짐"으로 둔 이유: 같은 프레임에서 뒤따르는 복사·전체 선택 같은 로컬 동작까지 해제 조건에 넣으면 방금 난 오류가 한 번도 그려지지 않습니다(`tests/terminal-host.rs` `headless_input_budget은_…` 가 이 동작을 고정하고 있습니다). toast 이전은 하지 않았습니다.

### T4. 프로젝트 닫기 시 Hub 엔트리 정리

- `native/taide-native-app/src/terminal_host.rs` `Hub::discard_project`: 해당 프로젝트의 엔트리를 꺼내 lock 밖에서 drop 합니다. `Hub::discard` 와 같은 `Entry::drop` 경로(writer 종료, store kill)를 그대로 씁니다.
- `native/taide-native-app/src/projects.rs` `NativeProjects::with_terminals`, `detach_all`: Hub 가 연결돼 있으면 `kill_project` 뒤에 `discard_project` 를 호출합니다.
- `native/taide-native-app/src/application-ports.rs` `Ports::build`: 프로젝트 lifecycle factory 가 터미널 Hub 를 넘깁니다.
- `host.rs` 의 `OpenProject`·`RestoreWatchers` 는 범위 밖이라 `NativeProjects::new` 그대로입니다. 이 경로의 `detach_all` 은 방금 연 프로젝트의 attach 실패 rollback 뿐이라 터미널이 없습니다.

### T5. 에이전트 감지 폴링

- `native/taide-native-app/src/application-ports.rs`
    - `AGENT_POLL_TASK`(`"agent-poll"`), `AGENT_POLL_INTERVAL`(`AGENT_POLL_UNIX_MS`·`AGENT_POLL_WINDOWS_MS`).
    - `start_agent_poll`: `tokio::time::interval` 로 `agent_actions::poll_agents` 를 호출합니다. 인자는 Tauri 와 1:1 입니다(state, agents, agent_hooks, events, tasks, `services.terminal.foreground_pids`, `agent_host::detect_agents_for_pids_blocking`).
    - `Ports::start`: 서비스 reconcile 뒤 폴링을 등록합니다. 이름 있는 작업이라 다시 호출해도 중복 등록되지 않습니다.
    - `AgentPollEvents`: `Arc<dyn EventSink>` 를 `&impl EventSink` 로 넘기기 위한 어댑터입니다. `agent-hooks.rs` 의 `HookEvents` 와 같은 모양이지만 그 파일이 범위 밖이라 공용화하지 않았습니다(7절 5번).
- 종료 회수: `drain_services` → `ExitDrain::wait_for_direct_exit` 의 `tasks.stop_all()` 이 작업을 abort 하고 완료를 기다립니다. `application::` 테스트 3건(실제 App 생성자·exit 포함)에서 종료 뒤 `tracked_count() == 0` 을 확인했습니다.
- `crates/taide-runtime` 은 수정하지 않았습니다.

## 3. 추가·수정한 테스트

| 항목 | 파일 | 테스트 |
| --- | --- | --- |
| T1 | `native/taide-native-app/tests/terminal-frames.rs` | 신규 `수위_신호는_high에서_보류하고_low에서_재개하며_닫힘은_보류를_푼다`(개수 수위, TS 상수 수위, 닫힘) |
| T1 | 같은 파일 | 기존 `실제_pty의_포화는_native_core를_실패로_닫고_…` 를 `실제_pty의_포화는_출력을_보류하고_소비_뒤_재개해_끝까지_전달한다` 로 교체. 기존 테스트는 포화 시 `Failed(Delivery)` 를 기대해 결함을 고정하고 있었습니다 |
| T1 | 같은 파일 | 신규 `실제_pty의_과부하_출력은_high_water에서_보류되고_유실없이_끝까지_전달된다`(앱 한도, 약 1 MiB 출력), `보류된_실제_pty는_수신자_소멸로_풀려_실패로_닫히고_worker를_회수한다` |
| T1 | `native/taide-native-terminal/tests/session.rs` | 신규 `출력_보류는_pty를_실패시키지_않고_…`, `보류된_출력은_실패_중단으로_풀려_…` (미실행, 6절 1번) |
| T1 | `native/taide-native-terminal/src/session.rs` | 신규 `feed_한도를_넘는_pty_batch는_한도_단위로_나눠_순서대로_제출한다` (미실행, 6절 1번) |
| T1 | `native/taide-native-terminal/tests/fixtures/session.rs` | `TAIDE_NATIVE_FIXTURE_FLOOD=1` 이면 60,000행을 한 번에 씁니다. 기본 동작은 그대로입니다 |
| T2 | `native/taide-native-app/tests/terminal-host.rs` | 신규 `포커스없는_단일_view는_측정_크기로_resize를_요청하고_공유_view는_focus_owner를_유지한다` |
| T3 | `native/taide-native-app/src/terminal-input-tests.rs` | 신규 `입력_오류는_성공한_후속_입력과_write_뒤에_해제된다` |
| T4 | `native/taide-native-app/tests/projects.rs` | 신규 `project_닫기는_해당_project의_terminal_hub_entry와_admission만_회수한다` |
| T5 | `native/taide-native-app/src/application-ports-tests.rs` | 신규 `start는_agent_poll을_등록해_foreground_pid와_probe로_상태를_발행하고_종료에서_회수한다`. 기존 테스트의 start 직후 `tracked_count` 기대값을 0 에서 1(agent-poll)로 변경, 한도 literal 을 `terminal_limits()` 로 추출 |

범위 목록에 없던 테스트 파일 수정: `terminal-input-tests.rs`(`terminal_surface.rs` 의 lib 테스트 모듈), `application-ports-tests.rs`(V1 지정 방식), `tests/fixtures/session.rs`(두 크레이트가 공유하는 PTY fixture).

V1 이행 정도: 구현 전 실패를 실행으로 확인한 것은 T1 뿐입니다. 큐의 보류 신호를 임시로 끈 상태에서 `terminal-frames` 는 exit 101 이었고(수위 테스트 단언 실패, 과부하 테스트는 보류 신호를 60.13초 기다리다 실패), 원복 뒤 통과했습니다. T2~T5 테스트는 새 API 를 참조해 구현과 함께 작성했고, 구현을 임시로 끄는 방식의 실패 확인은 자동 승인 정책이 해당 편집을 거부해 중단했습니다. 따라서 T2~T5 의 "수정 전 실패" 실행 기록은 없습니다.

## 4. 실행한 명령과 결과

공통 접미사 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` 는 생략해 적습니다.

| 단계 | 명령 | exit | 결과 |
| --- | --- | --- | --- |
| V2 | `cargo test --manifest-path native/taide-native-terminal/Cargo.toml --test session` | 101 | 컴파일 전 중단: `cannot update the lock file … because --locked was passed`. 6절 1번 |
| V2 대안 | `cargo test --manifest-path native/taide-native-app/Cargo.toml -p taide-native-terminal --test session` | 101 | `cannot be tested because it requires dev-dependencies and is not a member of the workspace` |
| V3 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test terminal-frames` | 0 | 최초 build 1분 03초, 7 passed 0.82초. 최종 코드 재실행 7 passed 0.61초 |
| V4 | `cargo test --manifest-path native/taide-native-app/Cargo.toml --test terminal-host` | 0 | 최종 코드 52 passed 1.38초. 그 전 실행의 OS 수준 실패는 5절 1번 |
| V5 | `… --lib application_ports` | 0 | 2 passed 0.05초 (T5) |
| V5 | `… --lib terminal` | 0 | 49 passed 4.02초 (T3 포함, `terminal_surface`·`terminal_host`·`remote_terminal` 등) |
| V5 | `… --lib projects` | 0 | 6 passed 8.21초 |
| V5 | `… --lib application::` | 0 | 3 passed 3.52초 (실제 App 생성자·exit 에서 폴링 등록과 회수) |
| V6 | `cargo check --manifest-path native/taide-native-app/Cargo.toml` | 0 | 최초 7.46초, 최종 코드 재실행 exit 0. 경고는 vendored `wry-preview` 17건뿐 |
| V6 | `cargo fmt --manifest-path native/taide-native-terminal/Cargo.toml -- --check` | 0 | 차이 없음 |
| V6 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 1 | 남은 차이는 `experiments/lsp-coordinator-spike/src/bin/mock-server.rs` 1곳뿐. 6절 2번 |
| 추가 | `… --test projects -- project_닫기는` | 0 | 1 passed 0.02초 (T4) |
| 추가 | `… --test projects` | 101 | 4 passed, 1 failed. 실패는 기존 `미연결_hook는_열기_전_거절하고_…`. 6절 3번 |
| 추가 | `… --test terminal-dispatch` | 0 | 5 passed (프레임 큐 소비처 회귀) |
| 추가 | `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --lib` | 0 | 앱 코드 경고 0건(vendored `wry` 경고만) |
| 추가 | `cargo clippy … --test terminal-frames --test projects` | 0 | 대상 경고 0건 |

실행하지 않은 것: `native/taide-native-terminal` 의 모든 테스트와 clippy(lockfile), `terminal-host` 의 clippy, `taide-native-app` 의 나머지 lib·통합 테스트, GUI 실행.

## 5. 실패했다가 고친 내역

1. `terminal-host` 의 간헐적 `openpty` 실패. 전체 실행 6회 중 4회가 exit 101 이었고(신규 2건 포함 53건일 때 실패 3·성공 1, 최종 52건일 때 실패 1·성공 1), 실패는 매번 다른 기존 테스트의 `hub.spawn(…).unwrap()` 에서 `failed to openpty: Os { code: -6 }` 였습니다. 단언 실패는 한 번도 없었고, 실패한 테스트만 다시 돌리면 통과했습니다. -6 은 XNU 의 `EREDRIVEOPEN` 으로, 방금 닫힌 pty 번호의 slave 를 여는 순간과 겹칠 때 커널이 재시도해야 할 값을 그대로 돌려준 것으로 판단합니다. 신규 테스트를 뺀 기존 51건은 3회 모두 통과했지만 표본이 작아 신규 테스트가 빈도를 높였는지는 단정하지 못합니다. 조치: T4 테스트를 PTY 테스트가 하나뿐인 `tests/projects.rs` 로 옮기고, 닫은 직후 PTY 를 다시 여는 단계를 빼고 admission 을 "열려 있지 않은 프로젝트로 spawn 하면 `NotFound`(여유 있음) 또는 `InvalidArgument`(고갈)" 로 확인하도록 바꿨습니다.
2. rustfmt 차이 5곳(`application-ports.rs`, `terminal_surface.rs`, `tests/terminal-frames.rs`, `tests/projects.rs` 2곳)을 손으로 고쳤습니다. `cargo fmt` 를 실행하면 범위 밖 `mock-server.rs` 까지 바뀌므로 실행하지 않았습니다.
3. T3 해제 조건. 처음에는 로컬 동작 성공도 해제 조건으로 설계했다가, 같은 프레임의 전체 선택·복사가 방금 난 예산 초과 문구를 지워 기존 `headless_input_budget은_…` 가 깨지는 것을 코드 검토에서 확인하고 "입력이 큐에 받아들여짐"으로 좁혔습니다(실행 실패 전에 수정).
4. 과부하 테스트에서 feed 분할을 임시로 끈 실행은 통과했습니다(0.81초). debug 빌드에서는 파서가 느려 batch 가 64 KiB 를 넘지 않기 때문입니다. 즉 실제 PTY 테스트는 feed 분할을 검증하지 못하며, 그 검증은 미실행인 `session.rs` 단위 테스트뿐입니다.

## 6. 검증 계약 중 exit 0 을 만들지 못한 것 (모두 이번 변경 이전부터의 상태)

1. V2. `native/taide-native-terminal/Cargo.lock`(10월 2일)에 `taide-remote-wire` 가 없습니다. `crates/taide-model/Cargo.toml` 이 10월 5일에 이 의존성을 추가했고 `taide-native-app`·`taide-native-ui`·`taide-native-editor` 의 lockfile 에는 반영돼 있습니다. `--locked` 로는 이 크레이트의 어떤 cargo 명령도 시작되지 않고, Cargo.lock 수정은 금지라 손대지 않았습니다. `--locked` 없이 한 번 실행하면 path 크레이트 항목만 추가됩니다. 그 뒤 실행할 것: 위 V2 명령과 `cargo test --manifest-path native/taide-native-terminal/Cargo.toml --lib session`. 이 크레이트에 추가한 테스트 3건은 컴파일도 실행도 되지 않은 상태입니다(rustfmt 구문 검사만 통과). 같은 gate 동작은 `terminal-frames` 의 실제 PTY 테스트 3건이 app 크레이트를 통해 실행 검증했습니다.
2. V6 native-app fmt. `experiments/lsp-coordinator-spike/src/bin/mock-server.rs` 의 import 순서 1곳입니다. native-app 이 이 파일을 example 로 포함하면서 edition 2024 규칙으로 검사하기 때문이며, git 상 변경이 없는 파일이고 범위 밖이라 두었습니다. 변경한 파일의 차이는 0건입니다.
3. `tests/projects.rs` 의 `미연결_hook는_열기_전_거절하고_실패한_attach는_등록을_회수한다`(286행). hooks 가 켜진 프로젝트 열기가 실패하길 기대하지만 `docs/bug/2026-10-04-native-project-hooks-enabled-rejection.md` 와 `2026-10-04-native-project-optional-attachment.md` 에서 그 거절을 결함으로 보고 제거했습니다. 같은 계약은 `src/projects-tests.rs` 가 검증합니다. 검증 계약 밖의 묵은 테스트라 수정하지 않았습니다.

## 7. 메인 판단이 필요한 사항과 남은 위험

1. 과제 문구에 없던 feed 분할을 T1 에 포함했습니다. 빼려면 `feed_with_delivery` 의 `chunks` 를 없애면 되지만, 그 경우 release 빌드에서 16 MiB/s 이상의 출력(`yes`, 큰 파일 `cat`)이 `Failed(Parser)` 로 세션을 죽일 수 있습니다. 이 경로는 코드 근거의 판단이며 실행으로 재현하지는 못했습니다.
2. 프레임 큐의 hard 한도 초과(`Error::Capacity`)는 여전히 치명입니다. 앱 한도에서는 보류가 512 KiB·32개에서 걸리고, 프레임 하나의 무게는 코어 한도(feed 64 KiB, 효과 256개·64 KiB)상 수백 KiB 수준으로 추정되므로 출력으로는 4 MiB·64개에 닿지 않습니다. 닿는 경우는 계약 위반(비정상적으로 작은 한도 등)뿐이라고 판단하지만, 프레임 최대 무게를 실측하지는 않았습니다.
3. 코어의 chunk 당 효과 예산(256개)은 그대로입니다. BEL·OSC 가 많은 바이너리를 `cat` 하면 `terminal effect budget overflow` 로 세션이 죽을 수 있습니다. backpressure 와 다른 원인이라 이번에 고치지 않았습니다.
4. 보류 중 교착 가능성. child 가 질의(DA, 색, 크기)를 쏟아내면서 응답을 읽지 않으면 actor 가 응답 write 에서 막히고 큐가 비워지지 않아 보류가 풀리지 않습니다. 이전에는 같은 상황에서 큐 초과로 세션이 죽었습니다. 종료·탭 닫기는 actor abort 와 수신자 drop 으로 빠져나옵니다.
5. `AgentPollEvents` 와 `agent-hooks.rs` 의 `HookEvents` 가 같은 어댑터입니다. `taide-runtime` 에 `impl<T: EventSink + ?Sized> EventSink for Arc<T>` 를 두면 둘 다 없앨 수 있습니다(범위 밖).
6. 에이전트 상태는 `AgentStateChanged` 로 발행되지만 native UI 소비자는 아직 없습니다(다음 배치). 폴링이 채운 `AgentStore` 덕분에 훅 payload 가 더는 버려지지 않습니다. 열린 프로젝트마다 500ms 주기로 foreground pid 를 읽고, 처음 보는 pid 가 있을 때만 `ps` 를 실행합니다(Tauri 와 동일).
7. native 화면의 프로젝트 닫기 경로는 원격 dispatch(`project_close`)뿐입니다. T4 는 그 경로와 lifecycle port 를 고쳤고, native UI 에 닫기가 생기면 같은 factory(`with_terminals`)를 써야 합니다.
8. `terminal-host` 의 `openpty` 간헐 실패(5절 1번)는 남아 있습니다. `crates/taide-infra/src/pty.rs` 에서 이 오류를 재시도하면 사라지겠지만 범위 밖이고, 실제 사용자도 터미널을 닫는 순간 다른 터미널을 열면 드물게 만날 수 있습니다.
9. T2 는 같은 세션을 두 뷰가 그리기 시작하는 첫 프레임에 먼저 그려진 뷰가 한 번 크기를 요청합니다. 다음 프레임부터 포커스 뷰 규칙이 적용됩니다.

## 8. 실기 확인이 필요한 것

- [ ] release 빌드에서 `yes`, `cat <수십 MiB 파일>`, `find /` 실행 중 세션이 "native terminal failed" 로 바뀌지 않는지, Ctrl+C 가 바로 듣는지, 출력이 끝난 뒤 스크롤이 정상인지
- [ ] 원격 웹 클라이언트를 같은 세션에 붙였다 뗀 뒤에도 과부하 출력이 멈추거나 죽지 않는지
- [ ] 에디터에 포커스를 둔 채 분할선을 끌거나 글꼴 크기를 바꾸면 터미널 열·행이 바로 맞춰지는지(`stty size`), TUI(vim, Claude Code) 화면이 깨지지 않는지
- [ ] 보조 창으로 터미널 탭을 옮긴 뒤 원래 창·새 창 모두에서 resize 가 따라오는지
- [ ] 입력 예산 초과 등으로 좌상단에 오류 문구가 뜬 뒤 다음 입력에서 사라지는지
- [ ] 원격 클라이언트로 프로젝트를 닫은 뒤 터미널 64개 한도가 줄어들지 않는지
- [ ] 터미널에서 `claude` 등을 실행했을 때 원격 클라이언트의 에이전트 배지가 바뀌는지, 앱 종료가 지연되지 않는지
