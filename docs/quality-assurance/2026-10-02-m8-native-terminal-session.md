# M8 실제 PTY와 단일 native Core 세션

## 대상·상태

`native/taide-native-terminal/src/session.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`, `tests/session.rs`, `tests/fixtures/session.rs`, `crates/taide-infra/src/pty.rs`입니다. 메인이 workflow·서브에이전트 없이 직접 구현했습니다. 실제 기존 infra PTY를 native SharedTerminal의 단일 Core에 연결했습니다. 아직 native app의 AppServices/session registry·TaskSupervisor composition·bounded writer/query/agent metadata consumer·terminal 화면은 연결 전이며 N4-B/N4/M8 전체는 미완료입니다.

## 단일 소유·snapshot/live

- SharedTerminal의 모든 clone은 같은 Arc/Mutex/Core를 참조합니다. snapshot callback은 그 잠금 안의 borrowed Core·revision·phase만 읽습니다. 새 view에 parser를 만들거나 truncated raw replay를 parse하지 않습니다. 출력은 같은 Core의 `advance_outcome`을 호출하고 단조 revision의 Frame으로 한 번 소비합니다.
- 하나의 SharedTerminal에는 PTY 한 개만 bind합니다. Core와 PTY의 초기 dimensions를 대조하고 중복 spawn은 OS 작업 전 거절합니다. 실패한 session의 정상 성공 재사용은 허용하지 않으며 새 세션을 만들어야 합니다. public Session resize·view별 scroll/selection·aggregate accounting은 아직 구현 전입니다.
- child exit callback은 `Draining(code)`만 기록합니다. 마지막 reader/flusher 출력은 이 상태에서도 받지만 새 입력은 거절합니다. `finish`는 자신에게 bind된 completion identity를 확인한 뒤 기존 `wait_for_completion`이 실제 worker join을 마칠 때까지 기다립니다. 그 후 마지막 synchronized bytes/text/effects를 flush하고 최종 Frame·Exited 상태를 한 번 확정합니다. 다른 PTY handle·중복 finish·종료 뒤 output/input은 거절합니다.
- deliver callback이 false를 반환하거나 unwind하면 RAII guard가 Failed(Delivery)와 명시적 stop을 요청합니다. parser/sequence 실패에서도 pending/Core를 retire하고 종료 성공으로 덮지 않습니다. 최초 실패 분류를 보존합니다. callback panic은 숨기거나 성공으로 변환하지 않으며 기존 join 오류가 보고됩니다.
- stop 요청은 기존 killer/pause 권한을 경유합니다. 새 `PtyStopHandle`은 completion owner에 대한 Weak이고 master/writer/worker의 강한 소유를 추가하지 않습니다. 실제 권한은 기존 kill에서 확인하며 소멸한 owner는 no-op입니다. callback이 handle bind보다 먼저 실패하면 StopGate의 요청 latch가 bind 직후 stop합니다. 고정 PID나 복제한 killer로 권한을 우회하지 않습니다.

Frame을 받는 실제 actor는 bounded queue에 admission하고 원래 순서로 metadata·query를 소비해야 합니다. 현재 deliver port는 동기 callback이며 IO/장시간 blocking/재진입을 피해야 합니다. 기존 PTY OutputBatch가 callback을 직렬 호출하지만 독립 caller가 직접 advance한 반환 Frame을 역순으로 소비하지 않는 책임은 caller에 있습니다. 기존 Core retained quota는 Core/Outcome 비용이며 Session wrapper·callback/외부 queue/OS resource의 전체 비용이나 peak/RSS cap이 아닙니다.

## 실제 검증

Cargo는 직렬이며 `--offline --target-dir experiments/native-shell-spike/target`입니다. 새 dev feature edge 해석을 위한 첫 테스트만 unlocked이고 이후 `--locked`를 사용했습니다. tokio는 기존 lock의 package를 재사용했습니다. 보호 사용자 `.app`은 빌드/조작하지 않았습니다. 자체 helper를 private 합성 PTY에서 실행했으며 `/bin/stty -echo -onlcr`는 그 helper의 private stdin PTY 설정만 바꿉니다. 사용자 OS 입력기·VoiceOver·clipboard·파일·실기 터미널 설정은 변경하지 않았습니다.

1. [x] 최초 compile에서 Frame을 pattern guard로 move한 E0507이 실패했습니다. 복제/Copy로 우회하지 않고 callback 본문의 조건으로 옮겼습니다. 해당 lib compile exit 0(0.47초)입니다.
2. [x] `cargo test --manifest-path native/taide-native-terminal/Cargo.toml … --test session -- --nocapture`: 실제 2 PASS, compile 1.40초/suite 0.34초. 동일 Core clone의 첫 borrowed Unicode snapshot·연속 revision·1,000행 전체 normalized 출력·중복 spawn 거절·실제 encoded input 수신·child exit 이후 실제 join·마지막 sync/title Frame·Exited content 보존·후속 입력/출력/finish 거절과, deliver false의 child stop/전체 worker 완료·Failed 상태 보존/부분 성공 거절을 확인했습니다. test signal의 `drop(Copy Result)` warning은 `let _`로 고쳤고 실행 성공을 반복하지 않았습니다.
3. [x] `cargo test … --test session callback_panic -- --nocapture`: 신규 1 PASS, compile 0.82초/suite 0.32초. synthetic callback panic이 실제로 출력됐고 stop guard가 child를 중단해 join은 3초 상한 안에 오류로 반환했습니다. `is_finished`는 오류를 성공 join으로 세지 않습니다. 살아 있는 다른 PTY의 completion 혼입을 거절하고 첫 Delivery 실패를 유지했으며 다른 활성 helper도 명시적 kill→own finish/join으로 회수했습니다. 관련 신규 경계만 실행하고 기존 2건을 반복하지 않았습니다.
4. [x] `cargo clippy --manifest-path native/taide-native-terminal/Cargo.toml … --lib --test session --bin native-session-fixture -- -D warnings`: exit 0(0.53초). 기존 infra API 변경에 대한 root `cargo clippy --manifest-path Cargo.toml -p taide-infra --lib … -- -D warnings`: exit 0(0.58초).
5. [x] native lib/session/test/helper·root infra PTY exact Rust fmt와 `git diff --check`: exit 0. 기존 helper/spike·product runtime/TS 성공은 재사용했으며 변경이 없는 전체 runtime 반복은 하지 않았습니다.

## 남은 제품 연결

- [ ] Native app의 단일 session registry·AppServices spawn lease/TaskSupervisor·등록 실패/닫힌 project/취소/종료 회수·metadata/agent observer·native viewport/tab owner.
- [ ] Queued+inflight count/byte quota를 가진 writer/query·입력 ack/geometry resize·sync deadline·호출 순서와 listener 분리. 지금의 finish sync는 실제 종료 경계이며 live sync deadline actor는 아직 없습니다.
- [ ] 실제 terminal placeholder의 borrowed renderer·colors/cursor/selection/search/link·view별 state/다중 창·IME/키패드/mouse·원본 전체 화면/AX·aggregate/CPU/peak/RSS·N1~N8/M8.

다음은 native app composition에서 session registry·bounded writer/query와 HostIntent/terminal surface를 연결합니다. 전체 M8 완료 전 commit·push하지 않습니다.
