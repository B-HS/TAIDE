# M8 native terminal 응답의 일시적 전송 포화

## 대상·원인

`native/taide-native-app/src/{terminal_writer,terminal_dispatch}.rs`, `tests/{terminal-writer,terminal-dispatch}.rs`입니다. Dispatcher가 query 응답을 `Writer::submit`으로 제출하여 일시적인 count/byte 포화도 오류로 처리했습니다. 호출자는 effects 실패로 Session을 종료할 수 있었습니다.

Core에서 실제 색 query effect를 생성하고 첫 write를 gate에 멈춰 writer count=1을 채웠습니다. Dispatcher future를 직접 한 번 poll하여 대기 대신 오류가 반환되는 RED를 확인했습니다. sleep 기반 추정이나 실제 사용자 PTY는 사용하지 않았습니다.

## 구현

- [x] Writer에 `submit_wait`를 추가했습니다. 동일한 Vec capacity+Envelope payload 계산으로 영구적인 단일 전송 초과를 먼저 거절하고, 가능한 전송의 byte/count permit을 비동기로 기다립니다. retry timer·busy loop·새 channel·무상한 producer를 추가하지 않았습니다.
- [x] 설치된 Tokio 1.53.1과 [Semaphore 공식 문서](https://docs.rs/tokio/1.53.1/tokio/sync/struct.Semaphore.html)를 확인했습니다. byte/count의 개별 permit 대기는 Tokio의 취소·반납 경계를 사용합니다. 이를 별도 UI pending까지 포함한 전역 FIFO 증거로 확대하지 않습니다.
- [x] actor가 abort되어 semaphore close 코드에 도달하지 못해도 receiver 종료를 함께 기다려 admission을 종료합니다. 사용자 취소·writer close·root stop에서 부분 취득 permit이 반환됩니다. 이미 실제 실행 중인 blocking write는 기존 supervisor가 완료까지 관리합니다.
- [x] Dispatcher의 query 응답은 admission과 실제 write receipt 완료를 순서대로 기다립니다. 일반 UI의 `try_submit`/Pending API는 유지했습니다. 기존 payload 검증을 두 진입점이 공유합니다.

## 실제 검사

공통 실행은 app manifest `native/taide-native-app/Cargo.toml`, 직렬 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`와 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다.

| 검사 | 실제 결과 |
| --- | --- |
| `--test terminal-dispatch query_pressure`, 최초 | exit 101, compile 4.56초 / suite 0.00초. `transient writer pressure failed the query instead of waiting` |
| 같은 검사, admission 대기 연결 후 | 신규 1 PASS, compile 5.56초 / suite 0.00초. future의 실제 Pending·먼저 실행한 write 뒤 정확한 query 응답·Dispatcher 성공·worker/task 회수를 확인했습니다. |
| `--test terminal-writer` | 변경된 공유 payload 검증과 async admission의 4 PASS, compile 1.20초 / suite 0.00초. 신규 byte-wait 취소/close 1건, async waiter를 추가한 root-stop 1건, 기존 순서/불량 payload·write 실패 2건입니다. |

새 byte 검사는 단일 payload 초과의 즉시 거절, byte permit 대기 중 future drop 뒤 다른 입력 승인, count 대기 중 close 종료를 확인했습니다. root-stop 검사는 semaphore close가 실행되지 않는 actor abort에서도 새 waiter가 종료하며 in-flight worker는 실제 gate release까지 남는 것을 확인했습니다. 테스트 gate는 실패 시에도 열려 blocking worker를 남기지 않습니다.

기존 actual PTY focus/lifecycle·input epoch·mouse 검사는 코드가 바뀌지 않은 해당 경계의 근거로 재사용했습니다. 이번 신규 query 압력 검사는 actual Core/Dispatcher/Writer와 제어된 sink를 사용했으며 실제 OS PTY의 모든 포화 동작을 검증한 것은 아닙니다.

app `clippy --lib --test terminal-dispatch --test terminal-writer -- -D warnings` exit 0(1.32초)와 authored 4파일 exact rustfmt·추적 diff check exit 0입니다. 기존 Wry 17 warnings와 authored strict 결과를 구분합니다.

## 미완료 경계

- [x] 후속 `2026-10-03-m8-native-terminal-input-order-queue.md`에서 Session pending과 query의 공통 admission 순서 표식을 연결해 actual PTY 추월을 수정했습니다. Views가 payload를 보유하더라도 최초 표식은 유지합니다. 아직 Session에 들어오지 않은 raw event와 output parse/dispatch의 전체 시간 순서·모든 producer 공정성은 별도이며, 이번 변경을 전체 입력 순서 완료로 계산하지 않습니다.
- [ ] admission 대기 future는 query Vec 하나를 보유하며, 아직 취득하지 못한 byte permit의 예산 밖에 있습니다. 현재 Dispatcher는 한 Session에서 한 응답씩 기다리지만 이 사실만으로 앱 전체 retained/RSS·다수 임의 caller의 상한을 보장하지 않습니다. 기존 frame/effect 예산 및 대기 payload의 aggregate 검증은 남습니다.
- [ ] 실제 child 종료와 admission 대기의 경쟁, 모든 mode/query/input interleave·focus 전용 상한 초과 복구·일반 captured-wheel 숨김·모호한 event 정렬·OS/다중 창/VT/IME/AX·213 TS view/full cutover/TS 제거는 미완료입니다.

보호 실기 bundle·기존 TS/root/MSRV·사용자 데이터/OS 설정을 유지했습니다. M8 N1~N8 전체 0/8이며 전체 완료 후 commit·push합니다.
