# M8 원격 PTY·터미널 실제 backend

## 2026-10-05 renderer query 생성 경계

후속 provider 정본은 application-ports QA의 맨 위 절입니다. 현재 Effects는 AppResult<ObservePorts>이며 실제 Views palette/context 제공 경계와 미준비 spawn 거절을 신규1 PASS(8.74초/.04초)·최종 strict3.33초로 확인했습니다. 아래 query/dispatcher/actor/remote 성공 결과는 재사용했으며 최종 전체 App 성공으로 확대하지 않습니다.

`remote-query-owner` bug가 원본 source·실제 expected1/observed2 RED·타입 분리의 정본입니다. 원격 spawn은 ObservePorts, native spawn은 실제 query capability가 있는 EffectPorts를 사용합니다. 별도 geometry/color fake 공급자를 원격에 넣지 않았습니다.

1. 실제 remote query 검사1 PASS(compile12.04초/suite0.03초)를 재사용했습니다.
2. `--test terminal-dispatch -- --exact renderer_관찰은_모든_query를_쓰지_않고_metadata_timer_observer를_보존한다 dispatch는_cwd_명령시간_agent_관찰시각과_native_query_순서를_보존한다`: compile11.83초/suite0.00초·native1 PASS/새 renderer fixture1 FAIL입니다. 새 fixture가 기존 native와 달리 classify_session_state로 agent를 등록하지 않아 Unknown이었습니다. 등록만 정정하고 renderer 검사만 compile1.10초/suite0.00초·1 PASS입니다. 기대 query3종이 실제 outcome에 존재하는 것, 쓰기0·cwd/command5500ms·agent 관찰·Bell1·stream/title·update2·중복 revision 거절/task0을 확인했습니다. 성공한 native 검사는 반복하지 않았습니다.
3. 현재 lib binary에서 실제 remote raw/replay/write/resize/pause/detach/kill·env 대기/입력 취소·공유 Hub layout close 관련3 exact 검사: 3 PASS/suite0.05초입니다. 새 query 성공은 다시 실행하지 않았습니다.
4. `--test terminal-host -- --exact query_order는_기존_pending_뒤_새_문자_앞에_실제_응답을_배치한다 종료한_child의_마지막_query는_응답하지_않고_최종_grid를_보존한다`: compile5.89초/suite0.48초·2 PASS입니다. Native actor의 실제 응답 순서와 exited child의 응답 억제·final grid/lifetime을 확인했습니다.
5. 마지막 lib/bin/tests clippy `-- -D warnings`: exit0/3.95초·authored6 rustfmt/check·tracked diff/check exit0입니다. 기존 Wry17 dependency warnings는 authored strict와 구분합니다. Cargo는 locked/offline·CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo·target=/private/tmp/taide-m8-menu-build.j6Efnw에서 직렬이며 동일 성공 결과를 재사용했습니다.

후속47은1/4(25%)·전체362/433(83.60%, 비가중·공수비 아님)·최종0/8·전체 ETA 미확정·goal active입니다. 다음은 실제 Views palette/repaint 제공 경계·Rust public assets·App owner입니다. 원격/native 화면 사이의 전체 query 소유권 handoff와 native-origin remote 구독, 실제 App/start/Exit/GUI·N1~N8은 미완료입니다. 사용자 데이터/OS/보호 bundle/cache/제품TS/vendor/의존성/manifest/lock/MSRV/Git은 이 변경에서 불변입니다.

## 대상과 결과

대상은 `native/taide-native-app/src/remote-terminal.rs`, `remote-terminal-tests.rs`, `terminal_host.rs`, `terminal_writer.rs`, `lib.rs`입니다. 원본 허용 PTY·terminal12명령을 기존 root TerminalStore와 native Hub의 동일 PTY·Core·writer에 연결했습니다. 선행152+12=164/177 domain adapter·남은13이며 전체 M8 N1~N8은 미완료0/8·목표 active입니다.

## 구현 경계

- 필수 remaining JSON/channel/raw·바깥 with_policy·owner 강제·default-deny를 유지합니다. Hub, environment, native history, EffectPorts factory가 필수 typed Ports이며 production 기본/no-op 포트를 추가하지 않았습니다. fixture의 palette/geometry/event callback을 실제 App 조립으로 세지 않습니다.
- Hub의 기존 spawn 본문을 `spawn_with_initial_sink`로 이동하고 기존 UI spawn은 동일 경로로 위임합니다. 원본 terminal_actions::pty_spawn의 환경 await→mutation/project/shutdown gate→초기 onData 폐기→실제 child/등록/event 순서는 유지합니다. 초기 채널은 구독자가 아니며 attach만 raw bytes와 reset preamble/원본 raw ring replay·subscriptionId/replayBytes를 받습니다.
- raw ring은 원격 클라이언트의 전송 replay용입니다. 별도 native parser/TerminalStore/child를 생성하거나 native Core를 replay로 다시 파싱하지 않습니다. 같은 reader의 bytes callback이 root ring을 기록하고 같은 Core가 출력·프레임·effects를 처리합니다. PTY remount A/B 결정은 건드리지 않았습니다.
- write는 원본 agent input 신호를 session 조회 전에 기록합니다. raw 문자열에 native Paste/bracketed-paste/key 인코딩을 적용하지 않습니다. Session의 기존 Core input admission과 writer order를 사용하고, 전체 요청을 하나의 order로 유지하면서 기존 writer의 payload 예산 안에서 분할합니다. native 로컬 입력·query reply가 분할 사이를 추월하지 않습니다. 빈 입력도 flush 경로를 거칩니다.
- 입력 action 전체는 기존 TaskSupervisor::run_nonabortable_result로 추적합니다. 요청 waiter를 버려도 실제 전송 작업과 남은 분할이 계속되며 실제 completion까지 operation lease가 남습니다. 이미 시작된 blocking write는 기존 writer actor가 감독합니다. 새 원격 메시지/응답 byte cap은 넣지 않았으며 M7의 256-frame-only 정책과 단일 큰 메시지의 메모리 위험은 유지합니다.
- resize는 같은 Session::resize를 통해 OS PTY와 native Core geometry/frame을 함께 바꿉니다. kill은 root action 뒤 Hub Entry를 폐기해 writer/actor/child를 회수합니다. pause/detach/목록/path/link/default/shell은 실제 root action을 사용합니다. kill 응답은 원본처럼 실제 join까지 기다리는 응답으로 바꾸지 않았으며 root idle/감독 종료가 회수를 완료합니다.
- native Hub에 이미 있던 세션 한도·Core phase/geometry 검증·writer 예약 한도는 그대로 적용됩니다. 특히 종료 중 보유 중인 Session Arc는 한도를 유지하며 capacity 오류가 root shutdown gate보다 먼저 올 수 있습니다. 이를 원본 Tauri와 모든 오류가 동일하다고 주장하지 않습니다. 실제 production lifecycle/App assembly에서 일관된 세션 수명과 종료 입장을 확인해야 합니다.
- manifest/lock/dependency/MSRV/root/Tauri/제품TS·보호bundle·사용자 앱/설정 변경은 없습니다. 실행 child는 새 합성 project의 `/bin/cat`뿐이며 사용자 shell profile/명령을 실행하지 않았습니다.

## 검증

환경은 CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo, manifest native/taide-native-app/Cargo.toml, locked/offline, target experiments/native-shell-spike/target입니다. Cargo는 직렬로 실행했으며 기존 성공 결과를 반복하지 않았습니다.

- [x] 최초 `cargo test --lib remote_terminal::tests -- --test-threads=1`: compile9.16초/suite5.07초,1 PASS/3 FAIL입니다. actual path/default/link/profile 검사는 이 결과를 재사용합니다. 채널 누락은 localized InvalidArgument인데 fixture가 plain code를 기대했고, 종료 gate fixture가 Session Arc로 capacity를 유지했습니다. 나머지 출력 대기는 단계 정보가 없는 timeout이었습니다.
- [x] 실패3만 `--skip 실제_root_path`로 확인: compile4.48초/suite5.08초,2 PASS/1 FAIL입니다. 앞의 fixture2건만 정정하고 출력 대기에 진단 값을 추가했습니다. 실패는 `second` 채널의 after-detach 입력이었으며 앞선 1024자 한 줄이 macOS canonical PTY 버퍼를 채워 newline·후속 입력을 BEL로 거절한 raw 출력이 확인됐습니다. 제품 writer/parser/timeout 정책을 바꾸지 않았습니다.
- [x] 한 줄 fixture를 writer 예산512보다 큰768자로 정정하고 실패한 `remote_terminal::tests::실제_single_core`만 확인: compile2.23초/suite0.07초,1 PASS입니다. 경로 검사와 앞선2 성공은 반복하지 않았으며 신규 고유4검사 모두 PASS입니다.
- [x] catalog12/actual routing arm/allowlist·선행 domain 비중복·typed 인자/필수 채널 localized 오류·prefix·없는 session 오류/원본 detach no-op·JSON/raw remaining/owner·외부 app_exit 거절을 확인했습니다.
- [x] 실제 합성 파일·subdir·outside 경로의 root guard/link candidates와 shell override/default80x24/scrollback optional·없는 project·원본 profile DTO를 확인했습니다. shell profile 발견은 기존 executable metadata 확인이며 shell 실행이 아닙니다.
- [x] 실제 PTY의 초기 채널 Drop/미전송·write 뒤 Core input epoch·attach preamble/replay/live raw·빈 입력/분할 입력·같은 Core96x32 resize·pause/unpause 호출·detach 이후 다른 채널만 수신·목록 DTO·root kill/Hub 제거·child/actor 완료·failure 없음·root idle/task0을 확인했습니다. pause 동안 출력이 절대 발생하지 않는다는 시간 기반 주장은 하지 않습니다.
- [x] environment 대기 중 요청 취소 시 channel Drop/child 없음/task0·없는 project·native 로컬 deferred input 앞의 remote raw 대기·remote waiter 취소 뒤 실제 worker의 순서 보존 전송·shutdown 뒤 조회/default 허용과 실제 spawn Forbidden을 확인했습니다. 한도 입장과 종료 입장을 혼동하지 않도록 종료한 Session Arc/worker를 회수한 뒤 검사했습니다.
- [x] 공유 Writer::submit_wait 경로를 분리했으므로 기존 독립 회귀1건 `cargo test --test terminal-writer waiting_writer는_byte_대기_취소와_close에서_permit을_회수한다 -- --exact`를 확인했습니다. compile12.41초/suite0.00초,1 PASS입니다. byte 과대·permit 대기 취소·close/waiter·actual inflight 완료/task0을 확인했습니다. 다른 기존 writer/Hub 성공은 재사용했습니다.
- [x] 최종 `cargo clippy --lib --bin taide-native-app --tests -- -D warnings` exit0,14.38초·authored5 exactfmt exit0입니다. 기존 Wry dependency17경고는 별도이며 authored 검사 억제는 없습니다.

## 미완료와 다음 경계

- [ ] 남은13명령은 task/font/system4·AI6·sync3입니다. 전체 mandatory backend factory와 production App·Settings/assets/Exit·HostBridge 저장·views/TS 제거/Rust99%·배포/rollback을 조립해야 합니다.
- [ ] 생산용 terminal history/palette/current geometry·clipboard/권한/effects·로컬/원격 shared Hub 소유권·전체 Exit/auxiliary/closed project를 아직 App에서 조립하지 않았습니다. core phase/geometry/한도 오류의 native 계약과 원본 transport의 오류를 전체 lifecycle에서 정리해야 합니다.
- [ ] 기존 keybinding focus-scroll RED·PTY remount 선택·LSP 정상 종료 handshake/recovery·실기 GUI/AX/IME/성능은 기존 M8 gate입니다. 사용자 앱/입력기/VoiceOver 설정을 조작하지 않았고 전체 완료 뒤에만 commit/push합니다.

## 근거

원본 Tauri remote gateway·terminal commands·actual root terminal_actions/TerminalStore/TerminalSessionOutput·native Hub/SharedTerminal/Core input/resize/actor/Writer/EffectPorts·기존 actual `/bin/cat` fixture를 확인했습니다. 웹의 pinned Tokio docs 조회는 접근 실패였으며 설치된 official Tokio1.53.1 source의 JoinHandle Drop/detach와 Semaphore 공정 대기/취소 문서, 실제 root TaskSupervisor nonabortable operation 구현을 확인해 사용했습니다. 의존성이나 API를 추측으로 추가하지 않았습니다.
