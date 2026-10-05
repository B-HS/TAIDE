# M8 native 입력 승인과 재시도 API

## 대상·범위

`native/taide-native-app/src/{terminal_writer,terminal_host}.rs`, `tests/{terminal-writer,terminal-host}.rs`, `native/taide-native-terminal/src/session.rs`, `tests/session.rs`입니다. 기존 N4-B 입력 큐의 승인 경계입니다. 이 checkpoint 뒤의 UI 자동 재시도 연결과 실제 결과는 `2026-10-03-m8-native-terminal-input-retry.md`가 정본입니다.

## 구현

- [x] `Writer::try_submit`은 `Submission::Accepted(Receipt)` 또는 `Submission::Pending(Vec<u8>)`를 반환합니다. 일시적으로 count/byte permit이 부족하면 원래 바이트를 caller에게 돌려줍니다. 단일 payload 자체가 전체 byte quota를 넘거나 writer가 닫혔으면 오류입니다. `submit`은 기존 거절 계약을 유지하는 wrapper입니다.
- [x] queue/inflight의 실제 Vec capacity+Envelope charge·count permit·취소된 receipt의 수락한 write·close/failure·TaskSupervisor 수명을 유지했습니다. Pending을 반환하는 경로에서 부분 확보한 permit은 scope 종료 시 반납합니다.
- [x] `SharedTerminal::try_input`은 live mode encoding과 동기 delivery callback을 같은 core lock에서 수행하고 callback이 승인한 user write만 input epoch에 반영합니다. 외부 delivery 실패와 input encoding 실패를 구분합니다. epoch 상한은 delivery 전에 검사하며 legacy `encode_input`은 같은 경계에 승인 callback을 전달합니다.
- [x] `Session::input`은 이 경계에서 기존 writer를 호출하고 실제 수락 후에만 agent 입력 기록을 갱신합니다. agent store 호출은 core lock 해제 뒤입니다. 이전에는 writer가 거절해도 epoch와 agent 입력 시각이 먼저 바뀌었습니다.

Rust Result에 없는 `is_err_or` 호출로 한 번 E0599 compile 실패가 있었습니다. 설치된 표준 Result의 `is_ok_and` 부정 조건으로 바꿨고 generic panic callback의 반환형을 명시했습니다. 검사 억제·새 dependency·unsafe는 추가하지 않았습니다.

## 실제 검사

Cargo는 기존 CARGO_HOME·직렬 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.

| 검사 | 결과 |
| --- | --- |
| app `--test terminal-writer` | 기존 3건 PASS, compile 6.35초 / suite 0.00초. blocked sink의 두 permit이 찬 동안 third byte를 Pending으로 보존한 뒤 기존 두 write 완료 후 정확히 세 번째로 재전송했습니다. 실패·root stop 회수도 확인했습니다. |
| native terminal `--test session input_admission` | 신규 1 PASS, compile 1.12초 / suite 0.00초. callback 거절 시 epoch 불변, 승인 시 한 번 증가, Focus write/encoding 거절 시 불변입니다. |
| app `--test terminal-host 활성_close` | 확장된 기존 1 PASS, compile 8.66초 / suite 0.43초. 실제 session의 writer byte quota가 거절한 입력에서 epoch 불변·local preedit·실제 close/join·보유 view admission 수명을 확인했습니다. |
| native terminal strict `--lib --test session -- -D warnings` | exit 0, 0.57초 |
| app strict `--lib --test terminal-host --test terminal-writer -- -D warnings` | exit 0, 1.62초 |

authored 6파일 exact rustfmt·추적 diff 검사는 exit 0입니다. 기존 Wry 17 warnings를 authored 검사 실패/성공과 구분합니다. 같은 입력의 PTY mouse/wheel 성공은 재사용했습니다.

## 남은 연결

- [x] 후속 Session→Views 공유 Outbox·byte/count 상한·hidden background 재시도를 연결했습니다. Writer 호환 wrapper가 아니라 typed `try_submit` 결과를 사용하며 actual count=1 PTY의 두 view·7개 혼합 입력 순서/epoch를 확인했습니다. 상세 범위와 한계는 input-retry QA에 있습니다.
- [ ] 인코딩 완료 pending의 Running/retire·승인 epoch·close/hidden/cancel은 후속 검사에서 확인했습니다. 아직 승인되지 않은 raw mouse/release의 숨김·focus/epoch 수명과 모든 교차 세션/실기 gate는 남습니다.
- [ ] 동일 event 일부 소비의 모호한 raw 정렬·UI 주입·mode ABA/forced selection·실제 OS/다중 창/IME/AX·전체 N4/M8는 미완료입니다.

보호 실기 bundle·사용자 데이터/clipboard·OS 설정·TS/root/MSRV는 유지했습니다. 전체 M8 완료 전 commit/push하지 않았습니다.
