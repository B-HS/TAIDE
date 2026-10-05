# M8 native terminal의 bounded writer

> 2026-10-02 후속: 실제 session/surface 연결은 terminal-host·mouse-adapter QA에 있고, typed Pending byte 반환과 승인 후 input epoch/agent 갱신은 terminal-input-admission QA에 기록했습니다. 아래 본문은 최초 writer checkpoint 기록입니다. UI 자동 재시도 대기열은 미완료입니다.

## 대상·상태

`native/taide-native-app/src/terminal_writer.rs`, `src/lib.rs`, `tests/terminal-writer.rs`, app `Cargo.toml`·`Cargo.lock`입니다. 메인이 직접 기존 TaskSupervisor와 TerminalStore의 실제 writer handle에 직렬 actor를 연결했습니다. app의 직접 taide-terminal path edge만 추가했고 새 외부 dependency·root MSRV·제품 PTY 정책·보호 실기 bundle은 바꾸지 않았습니다. 단일 session registry·spawn composition·query/agent·입력/UI caller는 아직 연결 전이며 N4-B/N4/M8 전체는 미완료입니다.

## 소유·상한

- caller가 count/byte Limits를 선택합니다. 0·Semaphore 상한·u32 byte 표현 초과는 spawn 전 거절합니다. 데이터 Vec의 실제 capacity와 Envelope inline 크기를 byte 비용으로 측정하며 count와 byte permit은 queued와 실행 중 write 모두에서 보유합니다. quota 거절 시 부분 enqueue/write하지 않습니다.
- TaskSupervisor의 async actor가 순서대로 blocking write를 하나씩 감독합니다. 실제 TerminalStore writer lock에서 write_all·flush를 수행합니다. Receipt waiter 취소는 이미 수락한 데이터·write를 취소하지 않습니다. payload와 permit을 실제 write 완료 후 폐기하고 receipt를 보냅니다.
- sink 실패·panic·worker 시작 거절·close는 후속 수락을 닫고 pending receipt를 실패로 처리합니다. root stop이 async actor를 abort해도 이미 시작한 blocking write는 기존 감독자에 남습니다. write를 강제로 끊거나 종료 deadline을 보장하지 않습니다. stdout/PTY reader와 metadata observer의 수명 연결은 이후 composition의 책임입니다.
- byte 비용은 논리 envelope/payload quota입니다. semaphore/mpsc/receipt/task/sink closure의 allocator overhead·전역 aggregate·peak/RSS/OS buffer 상한이 아니며 제품 기본 quota도 아직 정하지 않았습니다.

## 실제 검증

모든 Cargo는 직렬·offline·`--target-dir experiments/native-shell-spike/target`입니다. 첫 path edge 해석의 check만 unlocked, 이후 locked입니다. `.app` 실행/교체·시스템 설정·사용자 파일·clipboard는 조작하지 않았습니다. 합성 memory sink와 Condvar/oneshot의 실제 started/release 신호만 사용했습니다.

1. [x] 최초 app `cargo check … --lib`: exit 0, 9.24초.
2. [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test terminal-writer -- --nocapture`: 3 PASS, compile 21.71초/suite 0.00초. 취소된 receipt의 write/순서·queued+inflight count·실제 Vec capacity quota·invalid limits, sink 실패/pending 거절/permit 회수, root stop 뒤 시작된 IO의 tracked_count 1 유지→실제 release/완료→tracked_count 0을 확인했습니다.
3. [x] `cargo clippy … --lib --test terminal-writer -- -D warnings`: exit 0, 7.24초. 기존 Wry dependency의 17 warnings가 출력됐으며 own 대상의 strict 성공을 dependency warning 0으로 확대하지 않습니다.
4. [x] authored writer/test exact `rustfmt --edition 2024 --check`·`git diff --check`: exit 0. 같은 상태의 runtime 성공은 반복하지 않았습니다.

## 남은 경계

- [ ] 실제 SharedTerminal/AppServices/session registry와 원본 spawn lease·metadata/agent·bounded query 수신 및 입력 notify 순서.
- [ ] 입력/query 공용 writer의 제품 Limits, 살아 있는 blocked PTY의 UI close/resize/종료 동작, 전역 memory admission·성능.
- [ ] 실제 terminal surface·다중 view·IME/AX·전체 parity 및 N1~N8. 전체 M8 완료 뒤에만 commit·일반 push합니다.
