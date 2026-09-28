# M7 원격 WebSocket 전송 큐 보안 수정 범위

## 사용자 결정

2026-09-29에 사용자는 기존 Phase 0 기준선에서 후속 위험으로 남긴 원격 WebSocket 무상한 응답 큐를 별도 보안 수정으로 M7 범위에 추가하고 검증하는 A안을 선택했습니다. 이어 연결별 256프레임 상한과 포화 시 연결 종료를 선택했고, 64MiB 바이트 상한은 적용하지 않고 대형 단일 프레임의 메모리 위험을 별도로 기록하는 B안을 선택했습니다. 이 결정은 현재 실행 작업에 한하며 원격 서버를 GUI에서 실제 활성화하는 별도 확인을 대신하지 않습니다.

## 확인한 기존 경계

- `src-tauri/src/domain/remote/ws.rs`의 연결별 `mpsc::unbounded_channel::<WsOut>()`을 요청 응답·binary channel·event fanout·세션 만료 close가 함께 사용합니다. writer가 느리거나 멈추면 연결당 대기 프레임 수의 상한이 없습니다.
- `ChannelSink`는 동기 closure라 send 경로에서 비동기 대기를 넣을 수 없습니다. 수신자가 사라졌을 때 Err를 돌려 채널 구독자가 자신을 제거하는 계약과 JSON→binary→channel-end 순서를 유지해야 합니다.
- [Tokio bounded mpsc 문서](https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.channel.html)는 유한 용량 채널을 제공하고, [Sender::try_send 문서](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Sender.html#method.try_send)는 포화/닫힘을 즉시 구분합니다. [watch::Sender::send_replace 문서](https://docs.rs/tokio/latest/tokio/sync/watch/struct.Sender.html#method.send_replace)는 연결 관리 loop에 포화 신호를 전달할 수 있는 API를 명시합니다.

## 적용 정책과 잔여 위험

연결별 응답·channel·event가 공유하는 송신 큐를 256프레임으로 제한합니다. 포화 시 기존 프레임을 조용히 버리거나 생산자를 무한 대기시키지 않고 해당 WebSocket을 종료합니다. channel sink에는 Err를 반환해 도메인 구독자가 끊긴 연결을 제거할 수 있게 합니다. 테스트는 빈 큐 정상 순서, 정확한 256프레임 경계, 느린 writer를 모사한 포화·종료 신호와 수신자 소멸을 구분합니다.

프레임 개수만 제한하므로 1개의 매우 큰 raw 응답과 최대 256개 대형 프레임의 메모리 크기는 고정 바이트 예산으로 제한되지 않습니다. 이를 상한 검증 완료와 구분해 `docs/quality-assurance`에 재현 조건·생략 이유·위험·재검토 시점을 기록합니다.
