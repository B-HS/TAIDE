# Native IDE 연결 이벤트의 동기 notification 유실

## 대상 파일

- `native/taide-native-app/src/ide-server.rs`
- `native/taide-native-app/src/ide-server-tests.rs`
- 원본 비교: `src-tauri/src/domain/ide/server.rs:428`의 handle_connection

## 리포트

새 native 연결 처리기가 IdeStatusChanged.connected를 발행한 뒤 broadcast receiver를 구독해, 해당 이벤트에 동기 반응하는 observer의 첫 알림이 유실됐습니다. 합성 EventSink가 connected 이벤트 안에서 즉시 broadcast하도록 구성한 실제 loopback WebSocket 검사는5초 timeout RED였습니다. notification 구독을 ConnectionOwner의 connected 발행 앞으로 옮긴 뒤 같은 검사1건이 통과했습니다.

## 상세

원본 Tauri 처리기는 subscribe와 notification task를 만들고 client_connected/상태 이벤트를 발행합니다. 새 native 코드에서 connection count의 RAII 수명을 먼저 조립하면서 이 순서를 뒤집었습니다. handshake 이후 receiver를 먼저 생성하면 writer/notify task가 아직 spawn되지 않아도 broadcast channel이 observer 메시지를 보유합니다. 임의 sleep·재시도·첫 메시지 재발행은 추가하지 않았습니다.

재현은 실제 127.0.0.1 WebSocket·합성 UUID 임시 data 디렉터리·인증 token과 EventSink만 사용했습니다. 최초4검사 compile9.16초/suite5.01초에서 해당 테스트가 Elapsed로 실패했습니다. 수정 후 관련 필터 compile4.76초/suite0.06초에서 같은 test가 PASS했고 독립 고유 성공으로 집계했습니다. 해당 필터에 포함된 다른 성공 검사1건의 중복은 QA에 기록했으며 전체 기능/GUI/동시 수명 완료로 확대하지 않습니다.
