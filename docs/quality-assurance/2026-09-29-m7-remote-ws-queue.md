# M7 원격 WebSocket 연결별 송신 큐 검증

## 계약

사용자 선택은 [범위 기록](../acknowledge/2026-09-29-m7-remote-ws-queue-scope.md)대로 연결별 256프레임 상한, 포화 시 연결 종료, 바이트 상한 생략입니다. 응답·binary channel·event fanout·세션 만료 close가 같은 상한을 공유해야 합니다. 채널 sink의 동기 Err는 수신자 소멸과 포화 모두에서 상위 구독자가 해당 채널을 제거할 수 있게 해야 합니다.

## 검사 항목

- [x] 기존 무상한 큐에서 수신하지 않는 상태로 256프레임 다음 1프레임을 보낼 때 거부되지 않는 실패를 대상 테스트로 재현했습니다. `cargo test --offline -p taide --lib 느린_수신자의_전송_큐가_상한을_넘으면_송신을_거부한다 --quiet`는 수정 전 assertion 실패로 exit 101이었습니다.
- [x] 수정 후 정확히 256프레임까지 수락하고 257번째에서 Err·watch 포화 신호가 발생했습니다. 한 프레임을 수신해 빈자리가 생겨도 후속 송신은 Err이고 guard drop도 추가 프레임을 적재하지 않습니다. 같은 대상 테스트는 exit 0입니다.
- [ ] 포화 신호를 받은 실제 연결 관리 loop가 writer를 중단해 세션을 종료하는지 격리 WebSocket에서 확인합니다. 현재는 source와 큐 단위 테스트만 있습니다.
- [x] JSON→binary→channel-end 순서, 수신자 소멸 Err, close frame 변환과 sender clone의 종료 대기를 포함한 `cargo test --offline -p taide --lib domain::remote::ws::tests --quiet` 5건이 exit 0입니다. 실제 세션 TTL 만료 실기는 미완료입니다.
- [x] 대상 `cargo clippy --offline -p taide --lib -- -D warnings`, `cargo fmt --all -- --check`, 관련 문서 Prettier가 exit 0입니다. 실제 격리 앱 WebSocket의 느린 수신자 경계는 아직 미판정입니다.

## 잔여 테스트 부채

재현 조건: 원격 명령 또는 `file_read_raw`가 단일 거대 binary 프레임을 생성하거나, 느린 수신자에게 256개의 큰 프레임이 대기하는 경우. 생략 이유: 사용자가 호환성 영향을 가진 64MiB 바이트 상한을 이번 수정에서 제외하기로 선택했습니다. 남은 위험: 프레임 수는 유한해도 큐가 점유하는 실제 바이트와 프레임 생성 중 메모리는 고정 상한이 아닙니다. 대형 파일 원격 전송 정책을 별도 합의하거나 실제 메모리 압력이 관찰되면 바이트 예산·청크 프로토콜을 다시 검토합니다.
