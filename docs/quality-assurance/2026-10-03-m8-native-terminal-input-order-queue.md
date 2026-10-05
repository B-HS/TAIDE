# M8 native terminal 입력·응답의 공통 admission 순서

## 대상·재현

`native/taide-native-app/src/{terminal_writer,terminal_host,terminal_surface}.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다. query-pressure 후 Views Outbox의 오래된 PendingInput을 새 응답이 추월하던 경계를 연결했습니다.

실제 합성 child에게 `s`를 보내기 전에 `x`를 지연 등록하고, child가 상태 query를 출력한 다음 `z`를 등록했습니다. child는 `s`, `x`, `ESC[0n`, `z` 순서를 기대합니다. 기존 구현은 응답이 먼저 전달되어 child가 exit 1로 종료했으며 input epoch는 여전히 3이었습니다. 즉 입력 개수만으로 순서 오류를 찾을 수 없었습니다.

## 구현

- [x] Writer마다 동일한 OrderQueue를 사용합니다. Session은 인코딩된 쓰기를 처음 등록할 때 순서 표식을 발급하며 PendingInput이 writer 승인까지 그 표식을 소유합니다. Dispatcher의 비동기 query도 같은 queue에 등록합니다. UI가 재시도하더라도 최초 표식을 유지합니다.
- [x] 가장 앞의 표식만 실제 byte/count admission에 진입합니다. enqueue 이후 표식을 반납하여 다음 쓰기가 같은 bounded channel에 들어가도록 합니다. 실패/취소/거절/토큰 drop도 표식을 회수하고 대기자를 깨웁니다. 마지막 payload를 다시 encode하거나 input epoch를 선행 증가시키지 않습니다.
- [x] `submit_wait`는 순서와 용량을 차례로 기다립니다. 닫기 flag/Notify와 receiver 종료 감시를 함께 사용해, 아직 이전 UI token이 남은 상태의 명시적 close와 actor abort 모두 admission 대기를 끝낼 수 있게 했습니다.
- [x] focus batch는 활성 순서 표식이 서로 인접한 경우에만 합칩니다. 중간 query가 있으면 별도 fragment로 보관합니다. query가 취소되어 중간 표식이 제거된 경우에는 다시 합칠 수 있습니다. 새로운 일반 입력은 focus fragment 대기를 추월하지 못합니다.
- [x] 직접 `Writer::try_submit`이 반환한 raw Pending Vec는 호출자가 아직 순서 표식을 보유하지 않는 기존 API입니다. 앱의 Session PendingInput 경로는 별도 소유 표식을 유지합니다. 모든 임의 외부 raw Vec 재제출까지 최초 호출 순서를 보장한다고 주장하지 않습니다.

## 명시적 상한

- OrderQueue는 미승인 순서 표식 최대 256개이며 id 증가도 checked overflow로 거절합니다. queue는 payload를 복제하지 않습니다. 기본 앱의 일반 64개·focus 최대 64 fragment·현재 Dispatcher의 응답 하나와 일시적 후보 등록을 포괄합니다.
- 일반 Outbox는 기존 receipt+pending 64개와 byte 예산을 유지합니다. focus는 기존 단일 slot에서 최대 64개 fragment로 바뀌었으며, 합계 `Vec capacity + PendingInput inline`은 `64KiB + 64 * size_of::<PendingInput>()` 이하입니다. 각 fragment payload는 별도로 `min(64KiB, writer payload 상한)`을 넘지 않습니다. 오래된 focus-pressure 문서의 단일 slot 설명을 현재 상한으로 사용하지 않습니다.
- OrderQueue shared allocation·작업 중 복사 peak·대기 query payload·전체 앱 retained/RSS 검증은 별도입니다. 이 경계의 논리 상한을 OS/allocator/GPU 전체 상한으로 확장하지 않습니다.

## 실제 검사

Cargo는 직렬 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. app manifest는 `native/taide-native-app/Cargo.toml`입니다.

| 검사 | 실제 결과 |
| --- | --- |
| app `--test terminal-host query_order`, 최초 | exit 101, compile 3.98초 / suite 0.33초. `(Exited(1), 3)`과 기대 `(Exited(0), 3)` 불일치 |
| 같은 검사, 순서 표식 연결 후 | 신규 1 PASS, compile 13.09초 / suite 0.35초. actual PTY의 x→응답→z와 epoch+3·exit 0·close/join/task 0 |
| app `--lib control_batch` | 확장 1 PASS, compile 5.47초 / suite 0.00초. 기존 identity/byte 상한·중간 query 표식의 합치기 금지·query 취소 후 인접 합치기·이전 token을 보유한 채 writer close 때 새 waiter 종료 |
| app `--test terminal-host focus_pressure` | 영향 1 PASS, compile 10.83초 / suite 1.59초. 새 focus fragment/표식 배선의 실제 포화·바이트 순서·epoch/종료/회수 |
| app `--test terminal-writer` | 영향 4 PASS, compile 2.07초 / suite 0.00초. 공통 순서 등록을 거친 일반 try/비동기 대기·byte/cancel/close·root-stop·write 실패 |

이전 query-pressure 및 startup/mouse 성공은 해당 위험 범위의 증거로 재사용했습니다. 순서 표식 도입 뒤 변경된 writer 검사는 한 번만 수행했습니다. 새 child fixture는 자기 PTY만 noncanonical로 설정하며 사용자 terminal/app/OS 설정은 조작하지 않았습니다.

최종 app `clippy --lib --test terminal-host --test terminal-writer --test terminal-dispatch -- -D warnings` exit 0(4.39초), native `clippy --bin native-session-fixture -- -D warnings` exit 0(0.99초), authored 5파일 exact rustfmt·추적 diff check exit 0입니다. 기존 Wry 17 warnings와 authored strict 결과를 구분합니다.

## 남은 경계

- [ ] 이번 순서는 Session의 인코딩된 입력 등록과 Dispatcher의 query admission 기준입니다. raw UI packet이 아직 Session에 들어오지 않은 상태의 순서, output parse 시점과 Dispatcher delivery 사이의 정확한 경계, 동일 event 일부 소비·UI 주입·mode ABA/forced selection은 아직 남습니다.
- [ ] query가 focus fragment 사이에 있는 실제 다중 창/OS PTY 조합은 단위 adjacency 검사와 구분합니다. 모든 fragment 상한 초과 복구·256개 표식 포화/overflow 실측·다중 producer 공정성 스트레스·child 종료 경쟁·aggregate/RSS는 별도 검증 대상입니다.
- [ ] 일반 captured-wheel 숨김·context/search/link/project Hub·전체 VT/IME/AX/GUI·213 TS view/full cutover/TS 제거와 M8 N1~N8 전체는 미완료입니다.

보호 bundle·기존 TS/root/MSRV·사용자 데이터/OS 설정을 유지했습니다. 전체 M8 완료 후 commit·push합니다.
