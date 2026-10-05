# M8 native terminal의 bounded Frame 전달

## 대상·상태

`native/taide-native-app/src/terminal_frames.rs`, `tests/terminal-frames.rs`, app manifest/lock/lib와 native terminal `src/session.rs`의 Frame derive입니다. 메인이 직접 구현했습니다. app에 이미 검증한 native terminal path edge와 최상위 vte patch를 연결했고 기존 고정 fork/helper의 feature와 lock package를 재사용합니다. 새 package를 다운로드하지 않았으며 root manifest/MSRV·제품 scanner·보호 `.app`은 바꾸지 않았습니다. app의 native-terminal-queue-fixture bin은 기존 합성 session fixture source만 공유하며 desktop executable/bundle이 아닙니다. 단일 registry·제품 spawn/UI/전체 N4/M8는 미완료입니다.

## 전달 계약

- Frame의 필드 전체를 RetainedBytes derive로 측정합니다. Outcome의 text/overlap String capacity·effect Vec capacity와 typed effect의 실제 owned payload가 포함됩니다. Delivery inline 크기의 차이도 byte quota에 더합니다. 방문 상한·byte 산술/표현·count 상한·0 limits를 검증합니다. opaque owner의 비용을 0으로 간주하지 않습니다.
- Sender가 queued와 consumer가 보유한 Delivery 모두 count/byte permit을 유지합니다. Receiver가 pop했다고 permit을 즉시 반환하지 않습니다. Delivery는 borrowed Frame와 관찰 Instant만 공개하고 owned Frame을 추출하는 API를 제공하지 않습니다. 처리 완료/drop까지 같은 payload와 quota가 유지됩니다.
- accepted revision은 1부터 정확히 다음 번호만 허용합니다. 역순/중복·quota/retained 측정 실패·닫힌 receiver는 첫 failure로 고정하고 후속 수락을 닫습니다. accepted buffered Frame은 원래 순서로 drain한 뒤 실패를 반환합니다. 남은 Sender가 살아 있어도 빈 receiver의 대기는 failure/close에서 깨어납니다. 모든 Sender의 정상 drop은 buffered drain 뒤 None입니다.
- Queue 자체는 PTY 권한을 소유하지 않습니다. 실제 SharedTerminal callback의 submit 실패→false를 통해 기존 DeliveryGuard가 Core 실패와 weak stop을 요청합니다. 원본 Core는 enqueue 전에 이미 output을 적용했으므로 포화는 무손실 성공이나 부분 성공으로 보고하지 않습니다.
- caller가 quota를 정합니다. 이는 Frame/Delivery의 논리 payload admission이며 mpsc block/Arc/semaphore/visitor temporary/allocator/aggregate/peak/RSS/OS memory cap이 아닙니다. Core와 queue의 합산 제품 예산·정상 종료의 final Frame 소비·metadata/query actor·원본 UI와 성능은 아직 남습니다.

공식 계약은 [Tokio bounded mpsc의 disconnection/clean shutdown](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html)과 [owned semaphore permit](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html), 실제 고정 source를 함께 확인했습니다. queue count만으로 pop 뒤 in-flight 비용이 제한된다고 주장하지 않습니다.

## 실제 검증

Cargo는 직렬·offline·공유 target입니다. 새 native path graph의 첫 `cargo check … --lib`만 unlocked, 이후 locked입니다. 최초 check exit 0(6.96초)입니다. root/MSRV·사용자 실기 앱·OS 입력기/VoiceOver/clipboard·사용자 파일은 유지했습니다.

1. [x] app `cargo test … --test terminal-frames -- --nocapture`: 신규 3 PASS, compile 14.81초/suite 0.00초. actual Core Unicode Frame·borrowed count 유지·정확 byte quota/in-flight·큰 text capacity·순서/중복·첫 실패 보존·빈 receiver wake·정상 Sender drop/닫힌 receiver·invalid visits를 확인했습니다.
2. [x] app `cargo test … --test terminal-frames 실제_pty -- --nocapture`: 신규 실제 PTY 1 PASS, compile 1.05초/suite 0.32초. private 합성 helper의 초기 Frame을 실제 보유한 상태에서 encoded continue 입력을 bounded writer로 전송했습니다. 후속 frame 포화→Failed(Delivery)→실제 completion join/is_finished·잘못된 성공 finish 거절→writer 실제 완료/tracked_count 0을 확인했습니다. 최초 test compile의 Vec에 String용 into_bytes 호출 E0599는 실제 InputAction 타입대로 직접 submit하여 수정했습니다. 기존 3개 runtime 성공은 반복하지 않았습니다.
3. [x] app `cargo clippy … --lib --test terminal-frames --bin native-terminal-queue-fixture -- -D warnings`: exit 0(3.84초). inherited Wry 17 warnings와 own strict 성공을 구분합니다.
4. [x] authored frame module/test/native session exact fmt와 `git diff --check`: exit 0. 보호 `.app` 대신 test helper만 실행했습니다. `/bin/stty`는 그 helper의 private PTY 설정만 변경합니다.

다음은 같은 Delivery의 metadata/agent/query consumer를 단일 registry·AppServices spawn lease/TaskSupervisor·HostIntent/terminal 화면에 연결합니다. 전체 M8 완료 전 commit·push하지 않습니다.
