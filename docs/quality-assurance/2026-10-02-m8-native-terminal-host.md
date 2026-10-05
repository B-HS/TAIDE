# M8 native terminal Hub와 실제 root 수명 연결

## 대상·상태

`native/taide-native-app/src/{terminal_host,terminal_frames,terminal_writer,terminal_dispatch,lib}.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/src/session.rs`와 합성 session helper입니다. 실제 AppServices spawn lease·TerminalStore·TaskSupervisor·PTY writer/completion을 단일 Core/Frame/Dispatcher에 연결했습니다. NativeApplication/HostIntent/terminal renderer·제품 palette/geometry/env composition은 다음 단계이며 N4-B/N4/M8 전체 완료가 아닙니다.

## 실제 소유·정책

- Hub admission은 env/factory 전에 확보하고 Session·실제 read/exit callback이 같은 permit을 보유합니다. borrowed snapshot만 공개하므로 독립 Core handle이 quota 밖으로 탈출하지 않습니다. retained view가 남으면 Hub에서 닫힌 session도 permit을 유지합니다. 제품 limit 값·전역 합산 memory 정책은 아직 확정하지 않았습니다.
- 기존 `terminal_actions::pty_spawn`의 project 재검증·mutation guard·spawn lease·store 삽입·Spawned event를 그대로 사용합니다. 원본 raw output append/broadcast도 유지하되 native state는 같은 PTY chunk의 단일 parser에서만 갱신합니다. truncated remote replay를 새 Core의 완전한 복원으로 사용하지 않습니다.
- Pending RAII의 취소 latch와 blocking factory의 prepared slot이 같은 mutex를 사용합니다. late factory는 만든 실제 PTY를 retire합니다. actor Drop은 Core fail/stop·writer close·원본 store kill/remove·waiter notify를 수행합니다. 실제 worker handle은 registry Entry가 소유해 Session/actor cycle을 만들지 않습니다.
- 정상 종료는 own PTY completion join·남은 accepted Delivery drain·마지막 sync Frame 소비·Exit event·writer actor join 순서입니다. `Hub.close`는 원본 PTY stop/remove 뒤 dispatch와 실제 actor JoinHandle을 모두 기다립니다. 입력은 actual live mode encoder·기존 agent record_input→bounded writer 순서이며 Local action도 caller에 반환합니다.
- exit callback은 Core 실패 후에도 실제 exit code와 metadata 종료를 기록합니다. binding 전 실패/poison도 late actual handle의 stop에 연결합니다. 종료 후 query만 생략하는 수정은 별도 bug 문서에 RED/PASS를 기록했습니다. OS kill 실패/막힌 blocking IO의 bounded shutdown을 보장한다고 주장하지 않습니다.

## 검증 결과

Cargo는 직렬·`--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 합성 helper/AppState만 사용하며 보호 `.app`·OS 설정·사용자 파일·clipboard·keychain은 건드리지 않았습니다.

1. [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml … --test terminal-host -- --nocapture`: 최초 3 PASS(compile 5.73초/suite 0.34초). actual root spawn/same Arc·1,000행/raw attach·final sync/title·input receipt·정상 final grid·Spawned/Exit 1회·join/tracked 0, root stop/actor drop/store 제거·actual exit report, env-await 취소/닫힌 project/admission 회수·invalid limit preflight를 확인했습니다. 최초 compile의 없는 wait_for_completion 메서드는 실제 wait_for_idle로 수정했고 closure metadata 추론은 원본 Arc 경계 타입으로 수정했습니다.
2. [x] 새 활성 close 검사만 실행: 1 PASS(compile 3.90초/suite 0.42초). 실제 actor/PTY join·Local Preedit·retained view의 final grid와 quota 유지·view drop 뒤 quota 회수를 확인했습니다. 이전 3건 runtime은 반복하지 않았습니다.
3. [x] 새 종료 query 검사 RED 뒤 해당 1건만 수정 검증: 1 PASS(compile 4.38초/suite 0.34초). 실제 실패/원인/수정은 `docs/bug/2026-10-02-native-terminal-exited-query.md`입니다.
4. [x] 최종 `cargo clippy --manifest-path native/taide-native-app/Cargo.toml … --lib --test terminal-host --test terminal-dispatch -- -D warnings`: exit 0(1.98초). dependency Wry의 기존 17 warnings는 남아 있습니다. exact authored rustfmt와 `git diff --check`: exit 0.

## 미검증·다음 경계

- 취소 runtime은 env-await 이전 OS 미생성 경계입니다. 이미 시작한 blocking native factory의 취소는 RAII/source 및 기존 root spawn 근거이며 이 신규 runtime에서 직접 재현했다고 쓰지 않습니다.
- Session/wrapper/raw ring/queue/writer/allocator/TaskSupervisor 전역 합산·peak/RSS/visitor CPU·제품 session 상한은 별도입니다. 논리 queue/Core 상한만으로 총 memory를 보장하지 않습니다.
- NativeApplication의 단일 Hub 공유·실제 NewTerminal/layout/close·원본 세 가지 env provider·resize/live sync deadline·다중 view/selection/search/link/mouse/IME·실제 painter/AX/전체 N1~N8 gate는 미완료입니다. 기존 성공은 재사용하고 전체 M8 완료 뒤만 commit/push합니다.
