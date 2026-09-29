# M7 원격 세션 TTL 결정적 검사

## 대상과 방법

`crates/taide-remote/src/store.rs`의 실제 `RemoteStore`에서 세션 발급 시각, 만료 시각, 유효성 검사와 스윕을 확인했습니다. 최초 3건은 테스트 내부의 세션 만료 시각을 과거로 설정해 통과했습니다. 이후 사용자 선택에 따라 세션 시각을 Tokio `Instant`로 연결해 테스트 런타임의 일시 정지·전진 시계를 주입했습니다. 제품의 7일 TTL 상수와 인증 정책은 유지했습니다.

## 결과

- 발급 전·후 `Instant` 사이에 실제 만료 시각이 각각 `REMOTE_SESSION_TTL_MS`만큼 더해진 구간 안에 있었습니다.
- 만료된 세션은 `has_active_session`에서 거부되고 저장된 digest도 제거됐습니다.
- 만료 세션 스윕은 만료된 digest만 제거하고 별도 유효 세션은 유지했습니다.
- 최초 `cargo test --offline -p taide-remote --lib 세션_만료_ --quiet`는 3건 통과, 76건 필터, exit 0이었습니다. 당시 대상 Clippy·Rust fmt·diff 검사도 exit 0이었습니다.
- 가상 시계 적용 후 같은 대상 3건이 다시 통과했습니다. 만료된 세션 검사와 스윕은 테스트 시계를 실제로 7일 전진시켜 검증합니다.
- 별도 합성 루프백 Axum 서버의 인증 경로가 제품 미들웨어와 동일한 `has_authenticated_session_cookie` 함수를 사용하고, 열린 WebSocket이 제품과 동일한 `session_expiration_close` 경로를 사용하도록 했습니다. 만료 전 HTTP 200·WS 연결, 7일 전진 후 HTTP 401·WS close 4001을 한 번의 TCP 통합 검사에서 확인했습니다. `cargo test --offline -p taide --lib 가상_시계_만료는_http_401과_연결된_ws_4001을_반환한다 -- --nocapture`는 로컬 포트 권한을 허용한 실행에서 1건 통과, 191건 필터, exit 0이었습니다. 첫 샌드박스 실행의 exit 101은 테스트 리스너 바인딩 `Operation not permitted`였습니다.

## 미판정

이번 401·4001은 실제 TCP/HTTP/WebSocket 응답이지만 Tauri `AppHandle` 전체가 필요한 제품 `build_router`가 아니라 동일 세션 인증 함수·만료 함수로 조립한 합성 서버에서 관찰했습니다. 앞선 격리 제품 앱의 실제 인증·WS·폐기 관찰과 함께 M7-C1b의 증거로 사용하며, 제품 앱에서 시계를 7일 전진시킨 실측이라고 주장하지 않습니다.
