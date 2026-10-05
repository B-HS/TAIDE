# M8 native remote WebSocket 경계

## 대상과 결과

`native/taide-native-app/src/remote-ws.rs`, `remote-ws-tests.rs`, `remote-http.rs`, `remote-http-tests.rs`, `lib.rs`, native manifest와 `crates/taide-remote/src/store.rs`가 대상입니다. 원본은 `src-tauri/src/domain/remote/ws.rs`, `types.rs`, `dispatch.rs`와 root protocol/store/limiter입니다. M8 기존 Settings/remote pending 범위 안의 transport 체크포인트이며 전체 N1~N8은 0/8입니다.

실제 loopback HTTP upgrade와 WebSocket 왕복으로 고유 5개 검사를 확인했습니다. 전체 명령 dispatcher·제품 assets·App/Settings 조립·GUI·전체 보안·메모리·성능 완료로 확대하지 않습니다. fixture의 mandatory dispatch는 실제 settings_get/file_read_raw runtime과 합성 channel/error 처리기이며 제품 dispatcher를 대신하지 않습니다.

## 구현 계약

- 필수 typed `Dispatch { json, raw }`, `ResponseBody`, `ChannelFactory`와 `socket_action`을 제공합니다. SocketAction은 Arc 콜백으로 실제 구성 포트를 캡처하며 no-op/default 제품 조립은 없습니다. 요청은 원본 `RemoteRequest` DTO, 응답/채널은 기존 root wire 함수를 그대로 사용합니다. `file_read_raw`만 binary response이고 실패는 JSON error입니다.
- 채널 ID 숫자 변환 실패는 0, JSON 해석 실패는 null, atomic index와 마지막 sink Drop의 chanEnd를 보존합니다. 수신자 종료/포화는 오류로 전달해 domain pruning이 가능합니다. 원본처럼 256프레임의 try_send와 sticky saturation signal을 사용하며 포화 시 writer를 drain하지 않고 종료합니다.
- event broadcast, bulk session epoch와 개별 7일 Tokio deadline을 연결합니다. 개별 만료는 Close 4001/session_expired이고 같은 cookie의 HTTP 요청도 401입니다. 이벤트 구독은 connection admission 전에 준비합니다. 원본은 연결 자체에 상태 이벤트를 발행하지 않으므로 새 상태 이벤트를 추가하지 않았습니다.
- 요청은 연결 read loop를 막지 않고 감독 태스크 안에서 기존 semaphore permit을 기다립니다. 이미 대기 중인 요청이 폐기/만료 뒤 permit을 받으면 실행되는 원본 정책을 유지합니다. 원본 3초 writer 종료 제한도 유지하며 supervisor cancellation에서는 writer/event RAII abort와 connection Drop으로 회수합니다.
- root RemoteStore에 서버 시작별 generation과 같은 mutex 아래 connect/disconnect 판정을 추가했습니다. 원래 API는 유지하며 새 native connection만 generation-aware API를 사용합니다. stop/restart 전 connection의 늦은 Drop은 새 서버 count를 변경하지 않습니다. session epoch는 revoke에도 바뀌므로 서버 identity로 재사용하지 않았습니다.
- axum on_upgrade의 외부 task는 upgrade operation lease로 추적하고, 실제 socket consumer는 별도 감독 태스크로 등록합니다. callback cancellation은 SocketTask RAII abort, 감독자 직접 종료는 consumer abort/join 뒤 외부 lease Drop까지 기다립니다.

## 실행 근거

모든 Cargo 명령의 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, target은 `experiments/native-shell-spike/target`, `--locked --offline`입니다. loopback 검사는 정확한 합성 대상에만 escalation했습니다. 실제 앱·키체인·사용자 프로젝트·클립보드·시스템 입력기·VoiceOver·보호 bundle은 사용하지 않았습니다. native 기존 Tokio 1.53.1의 dev test-util feature만 활성화했고 새 package/version/checksum은 추가하지 않았습니다. offline metadata exit 0이며 root lock/MSRV는 이 경계에서 유지했습니다.

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib remote_ws::tests:: -- --nocapture --test-threads=1`: compile22.36초/suite3.06초에서 3 PASS·왕복1 FAIL입니다. permit/폐기/restart, 7일 HTTP401/WS4001, queue/wire unit이 통과했습니다. outside path가 원본 Localized/Forbidden인데 fixture가 plain Forbidden을 기대한 실패입니다. 제품 오류 계약을 바꾸지 않았습니다.
- [x] 왕복 실패 1건만 `--exact` 재실행: compile3.00초/suite0.03초 PASS입니다. 무인증 upgrade401, invalid request 무시, 실제 settings 응답/guarded binary 파일/경로 거절, channel JSON→binary→end→response와 event fanout·일반 close를 확인했습니다.
- [x] 감독자 직접 종료 신규 검사: compile4.13초/suite5.02초에서 timeout RED입니다. 실제 consumer가 axum 외부 task에 남는 원인을 재현했습니다. 감독 등록 후 관련 1건만 compile6.51초/suite0.03초 PASS이며 active socket·upgrade owner·tracked0·client0입니다.
- [x] 위 HTTP upgrade 수명 수정의 영향을 받는 왕복/TTL/permit-restart 3건만 재검사: compile0.22초/suite3.05초 3 PASS입니다. 큐 unit과 직전 직접 종료 성공은 제외·재사용했습니다. 같은 상태의 성공을 3회 반복한 증거가 아닙니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --lib --bin taide-native-app --tests -- -D warnings` exit0/23.23초, `cargo clippy -p taide-remote --lib --tests -- -D warnings` exit0/4.58초입니다. 기존 Wry dependency 경고17개는 authored 코드의 strict 검사와 구분하며 억제하지 않았습니다.
- [x] authored native5 edition2024/root1 edition2021 exact rustfmt check exit0입니다. root tracked diff check exit0, untracked native source5/manifest/lock의 no-index whitespace 검사는 빈 출력(exit1: 새 파일의 diff 존재)입니다. PROCESS/HANDOFF/재개/QA/bug5문서 Prettier exit0이며 실제 실행 결과를 재사용합니다.

TTL 검사는 실제 HTTP/WS를 먼저 연결해 JSON 왕복까지 확인한 뒤 current-thread Tokio clock을 pause/advance(원본 7일)/resume합니다. OS 시계나 제품 TTL은 변경하지 않으며 advance 이전에 만료될 fixture timeout을 감싸지 않았습니다. 실제 7일 벽시계 대기 증거는 아닙니다.

## 미완료와 검증 부채

- [ ] 원본 remote_gateway의 전체 명령 default-deny/조건부 policy·owner 강제·gated Settings와 channel domain adapter를 native dispatcher에 연결합니다. 현재 transport의 mandatory fixture를 제품에 조립하지 않습니다.
- [ ] 제품 asset resolver/개발 proxy·NativeApplication startup/Exit·state/event bridge와 Settings IDE→hooks→remote, HostBridge AppFile write/저장 키·production IDE layout/diff/save를 연결합니다.
- [ ] 느린 실제 네트워크 receiver stress/큰 단일 frame 메모리·한꺼번에 많은 permit 대기 요청·원본 domain store의 idle sink pruning·전체 동시 lifecycle을 검증합니다. 재현 조건은 TCP receiver 읽기 정지 또는 대형 raw/channel response입니다. 이번 단계는 queue 단위 상한과 실제 소켓 수명만 검증하며 원본 정책을 바꾸지 않았습니다. 제품 remote 조립·보안/메모리 gate에서 실행합니다.
- [ ] 원본은 프레임 수 상한만 있으며 byte 상한은 사용자 B 결정대로 도입하지 않습니다. 이미 대기한 요청의 session 재검증/취소와 cancel-class 우선 처리도 원본에 없는 변경으로 별도 보안 계약이 필요합니다. transport 성공은 해당 위험의 해결 증거가 아닙니다.
- [ ] 원본 TS 제거/Rust99%·213view/41action/Monaco21·실제 픽셀/AX·성능/배포/rollback·keybinding Tab RED와 PTY remount 결정은 기존 미완료입니다. 이 체크포인트에서 commit/push하지 않습니다.

## API 근거

[axum WebSocket 0.8.9](https://docs.rs/axum/0.8.9/axum/extract/ws/struct.WebSocket.html), [Tokio JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)와 설치된 axum0.8.9 `extract/ws.rs`의 on_upgrade spawn, Tokio1.53.1 `time/clock.rs`의 pause/advance/resume 원문을 확인했습니다. 버전 기억으로 API를 새로 작성하지 않았습니다.
