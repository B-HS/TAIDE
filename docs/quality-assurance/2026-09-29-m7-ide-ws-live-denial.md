# M7 격리 앱 IDE WebSocket 인증 거부 실기

같은 계측용 release 앱의 IDE listener가 `127.0.0.1:33832`에서 LISTEN 중인 것을 확인했습니다. 인증 헤더 없이 `Connection: Upgrade`, `Upgrade: websocket`, `Sec-WebSocket-Version: 13`, 합성 `Sec-WebSocket-Key`, `Sec-WebSocket-Protocol: mcp`를 넣은 HTTP/1.1 요청 한 번의 응답은 `HTTP/1.1 401 Unauthorized`였습니다.

요청은 격리 앱의 loopback listener에만 전송했고 IDE lockfile·토큰·키 파일은 읽거나 출력하지 않았습니다. 기본 sandbox의 loopback 연결 거부(exit 7)는 같은 listener의 `lsof` 확인 뒤 권한 허용 명령으로 해결했습니다.

이 결과는 실제 실행 앱의 **무인증 upgrade 거부**만 입증합니다. 유효 토큰의 WebSocket 연결, `initialize`·`tools/list`·`tools/call` 응답, 탭 수명과 통합 종료는 아직 실측하지 않았습니다. 합성 토큰을 쓰는 기존 핸드셰이크 테스트와 실제 앱의 성공 인증을 혼동하지 않습니다.
