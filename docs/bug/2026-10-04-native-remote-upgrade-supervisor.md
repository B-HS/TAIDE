# Native remote WebSocket upgrade 감독 수명

## 증상과 재현

`native/taide-native-app/src/remote-http.rs`가 axum on_upgrade callback에서 SocketAction을 직접 기다렸습니다. 실제 authenticated WS·settings_get 왕복 후 remote_stop을 먼저 부르지 않고 `TaskSupervisor::shutdown`을 호출하는 신규 합성 검사는 5.02초 timeout으로 실패했습니다.

## 원인

axum0.8.9 on_upgrade는 자체 tokio::spawn을 사용합니다. callback의 operation lease는 종료까지 추적하지만 그 consumer를 supervisor가 취소할 수 없었습니다. writer/event/server 태스크는 취소됐어도 socket read loop와 upgrade/WS operation이 남아 shutdown이 끝나지 않았습니다. standalone remote_stop 성공은 감독자 직접 종료 증거가 아닙니다.

## 해결과 검증

upgrade 이전 lease는 유지하고 실제 SocketAction을 `native-remote-socket` supervised transient task에 등록했습니다. callback은 JoinHandle 완료를 기다리고 SocketTask RAII가 callback cancellation에도 abort합니다. consumer의 writer/event AbortOnDrop과 generation-aware count Drop을 함께 사용합니다. 제품 시간 제한·OS 종료 방식을 바꾸지 않았습니다.

관련 신규 직접 종료 1건만 수정 후 실행해 compile6.51초/suite0.03초 PASS, tracked0/client0/실제 소켓 종료를 확인했습니다. 영향을 받는 왕복/TTL/permit-restart 3건도 suite3.05초 PASS이며 큐 unit·직전 직접 종료는 반복하지 않았습니다. 나머지 production App lifecycle/GUI Exit는 아직 미완료이며 QA `docs/quality-assurance/2026-10-04-m8-native-remote-ws.md`가 정본입니다.
