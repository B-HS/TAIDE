# M8 native 터미널 입력 재시도

## 대상·범위

`native/taide-native-app/src/{terminal_host,terminal_surface,application}.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/src/session.rs`, `tests/{session,fixtures/session}.rs`입니다. 기존 N4-B 입력 승인 API 뒤에 세션별 자동 재시도를 연결했습니다. 전체 터미널 입력·M8 완료는 아닙니다.

## 구현

- [x] `Session::queue_input`은 live core lock에서 한 번 인코딩한 바이트를 승인하거나 opaque `PendingInput`으로 반환합니다. `retry_input`은 세션 identity를 확인하고 원래 바이트를 다시 제출합니다. 재시도 시 mode를 다시 인코딩하지 않으며 core Running·epoch 상한을 다시 확인합니다.
- [x] `SharedTerminal::try_input`과 `try_write`는 같은 승인 경계를 사용합니다. 승인되지 않은 입력은 epoch와 agent 입력 기록을 갱신하지 않습니다. 승인된 user write는 한 번만 epoch를 갱신하고, 기존 Focus write의 epoch 제외 계약을 유지합니다. agent 기록은 core lock 해제 뒤입니다.
- [x] `Views`가 session ID별 `Outbox`를 공유합니다. 각 큐는 receipt+pending 합계 64개, pending Vec capacity+inline 크기 합계 `64 * (64KiB + size_of::<PendingInput>())`로 제한합니다. 기존 pending 뒤의 새 입력도 동일 큐에 넣어 여러 view가 같은 PTY에 쓰더라도 먼저 들어간 입력을 추월하지 않습니다.
- [x] `NativeApplication::background_tick`에서 숨겨진 view의 인코딩 완료 입력도 재전송하고, 남은 입력은 16ms repaint를 요청합니다. 사라진 Hub session·앱 종료는 대기를 회수합니다. receipt를 버리는 것은 이미 승인된 writer-owned 전송을 취소하지 않습니다.
- [x] 큐가 가득 차도 SelectAll·PageUp처럼 PTY 전송이 필요 없는 local action은 허용합니다. write는 반드시 슬롯·byte 상한을 통과해야 보유하며 초과 시 명시적 오류를 표시합니다.

`Tabs`를 `Hub` 인자에 전달한 E0308 compile 실패는 확인된 `Tabs::hub()` accessor로 수정했습니다. 예산 검사 전에 모든 입력을 거절해 local selection까지 막던 실패는 별도 최소 재현 후 수정했습니다. 새 의존성·검사 억제·unsafe는 없습니다.

후속 `2026-10-03-m8-native-terminal-focus-pressure.md`에서는 일반 64개 예산과 별도로 focus 전용 slot 하나를 추가했습니다. 해당 slot의 retained payload는 최대 min(64KiB, writer payload 상한)이며 PendingInput inline 크기가 추가됩니다. 위 최초 Outbox 예산만으로 이후 전체 retained 상태를 설명하지 않습니다.

## 실제 검사

공통 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 직렬 `cargo test|clippy --manifest-path <대상>/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

| 대상·추가 인자 | 실제 결과 |
| --- | --- |
| app `--test terminal-host mouse_surface` | 1 PASS, compile 12.13초 / suite 0.54초. writer count=1, 두 native view의 문자·mouse·wheel 7개 입력, 첫 승인 epoch+1→최종+7, UI 재표시 없는 flush, 실제 child literal 순서·exit 0·join·task 0 확인 |
| native terminal `--test session input_admission` | 1 PASS, compile 1.08초 / suite 0.00초. try_write 거절/승인·epoch·retire 후 callback 미호출 확인 |
| app `--lib mouse_queue` | 1 PASS, compile 4.96초 / suite 0.01초. 공유 Outbox receipt에 맞춘 raw packet 다음 frame·release 예약 유지 |
| app `--test terminal-host 활성_close` | 1 PASS, compile 2.94초 / suite 0.10초. deferred token 승인 전 epoch 불변·실제 close/join 뒤 retry 거절·epoch 불변 확인 |
| app `--test terminal-host headless_surface` | 1 PASS, compile 0.24초 / suite 0.21초. attach 전 입력·실패/재시작·IME 확정의 변경된 Outbox 연결 확인 |
| app `--test terminal-host headless_input_budget` | RED: local selection 차단, compile 3.18초 / suite 0.14초. 수정 후 1 PASS, compile 3.44초 / suite 0.34초. 64개 큐·초과 오류·포화 상태 SelectAll/CopyText·취소 후 추가 승인 없음·실제 join/task 0 확인 |
| native terminal strict `--lib --test session --bin native-session-fixture -- -D warnings` | exit 0, 0.63초 |
| app strict `--lib --test terminal-host -- -D warnings` | 최종 exit 0, 1.33초 |

app strict의 local action 수정 전 성공은 3.31초이며 영향받은 경로만 최종 재검사했습니다. authored 7파일 exact rustfmt와 추적 diff 검사 exit 0입니다. 기존 Wry 17 warnings는 authored strict 결과와 구분합니다. 동일 상태의 성공 검사는 반복하지 않았습니다.

## 남은 경계

- [ ] 이 checkpoint의 hidden 검사는 인코딩 완료 pending만 증명합니다. 후속 `2026-10-03-m8-native-terminal-hidden-input.md`에서 raw mouse release의 숨김·다른 view 순서를 연결했고 실제 PTY로 검증했습니다. 일반 captured-wheel·focus report·mode/rows/buffer epoch의 전체 전환은 남습니다.
- [ ] 다른 세션 token의 identity 거절은 구현했고 strict 검사에 포함했지만, 실제 두 세션 사이의 token 교차 제출 검사는 아직 없습니다. session 교체/다중 session 입력 경로 확장 시 해당 위험을 직접 검사합니다.
- [ ] 동일 event 일부 소비의 모호한 정렬·UI 주입·mode ABA·forced selection·OS/다중 창/DPR·context/search/link/project Hub는 미완료입니다.
- [ ] payload/inline의 큐 상한은 전체 aggregate allocator/RSS 상한의 증거가 아닙니다. query/input 공유 writer 압력·전체 VT/IME/AX·GUI/성능·213 view/full cutover/TS 제거도 남습니다.

보호 실기 bundle·사용자 데이터/clipboard·OS 설정·TS/root/MSRV를 유지했습니다. 실제 OS clipboard는 읽거나 쓰지 않았으며 headless CopyText 출력 명령만 검사했습니다. M8 상위 N1~N8은 0/8이며 전체 완료 뒤에만 commit·push합니다.
