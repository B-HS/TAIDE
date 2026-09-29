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
- 후속 `src-tauri/tests/remote_product_router_ttl.rs`는 사용자 데이터·키링 대신 메모리 상태를 주입한 실제 Tauri `AppHandle`과 제품 `server::build_router`를 사용합니다. 인증된 `/__taide/ws`가 열린 뒤 Tokio 시계를 7일 전진시켜 제품 HTTP `/`의 401과 같은 열린 소켓의 close 4001을 단언했습니다. `cargo test --offline -p taide --test remote_product_router_ttl`는 기본 샌드박스의 루프백 바인딩 거부로 exit 101이었고, 동일 바이너리를 루프백 권한으로 실행해 exit 0이었습니다. 링크·쿠키 값은 출력하거나 파일에 저장하지 않았습니다.
- 새 테스트 파일의 `rustfmt --edition 2021`은 exit 0입니다. 첫 `cargo fmt --all --check`가 새 파일과 이전 M7 TTL 테스트 파일 `src-tauri/src/domain/remote/ws.rs`의 import 순서를 지적해 exit 1이었고, 두 파일의 형식만 바로잡은 뒤 같은 형식 검사는 exit 0입니다. `cargo clippy --offline -p taide --test remote_product_router_ttl -- -D warnings`도 exit 0입니다. 테스트 링크 단계의 `__EMBED_INFO_PLIST` 중복 경고는 비치명적이었으며 실제 TTL 검사 실행은 exit 0입니다.

## 미판정

최초 401·4001은 공유 함수로 조립한 합성 서버의 응답이었습니다. 후속 검사에서 제품 `build_router`·WS handler의 실제 TCP 경로도 같은 결과를 보였지만, 별도 테스트용 Tauri 프로세스와 메모리 secret 상태를 사용했습니다. 실행 중인 격리 release 앱의 시계를 전진시킨 결과나 실제 7일 경과·외부 기기 연결 결과로 확대하지 않습니다.
