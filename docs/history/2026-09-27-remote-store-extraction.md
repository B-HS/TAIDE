# RemoteStore 공유 상태 분리

상태: M6의 이 변경 단위는 자동 검증 완료. M6 전체와 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-remote/src/store.rs`가 서버 실행 상태·Tokio 핸들·종료 신호·클라이언트 수와 링크/nonce/세션 digest·로그인 잠금을 보관합니다. 내부 상태는 공유 Arc<Mutex>, 이벤트 broadcast와 세션 epoch watch는 같은 채널 sender clone을 공유합니다. `crates/taide-runtime/src/app_services.rs`가 생성한 동일 인스턴스를 `src-tauri/src/lib.rs`에서 Tauri State로 등록하며 AppServices는 총 17개 상태·포트를 조립합니다.

`src-tauri/src/domain/remote/commands.rs`는 기존 RemoteStore·RemoteShutdownState 경로를 재수출합니다. HTTP/WS 서버와 라우팅·키링 읽기/쓰기·상태 이벤트·TaskSupervisor 조립은 Tauri adapter에 남겼습니다. 감독자가 원래 반환하는 Tokio 핸들을 그대로 저장해 불필요한 Tauri wrapper만 제거했습니다. 중복 시작의 후보 취소·shutdown 신호·grace wait/abort·감독 등록 거부 시 직접 취소 순서는 유지합니다.

기존 순수 store 정책 28건은 remote crate로 옮기고 limiter 2건은 기존 위치에 유지했습니다. store가 쓰는 용량·TTL·로그인 잠금 상수 6개도 remote crate로 옮겼으며 기존 adapter 경로는 재수출합니다. 값·인증 판정·nonce/링크 1회 소모·세션 만료·잠금 축·epoch 정책은 바꾸지 않았습니다. 기존 parking_lot·Tokio와 로컬 remote 의존만 연결하고 패키지 버전은 바꾸지 않았습니다.

[Tokio broadcast Sender](https://docs.rs/tokio/latest/tokio/sync/broadcast/struct.Sender.html)·[watch Sender](https://docs.rs/tokio/latest/tokio/sync/watch/struct.Sender.html)·[JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)의 공식 공유·취소 계약을 확인했습니다.

## 검증

- 변경 전 `cargo test -p taide --lib domain::remote::commands::tests:: --quiet`: 30건 통과. 새 AppServices 검사는 remote 필드 부재 E0609 4건(exit 101)으로 먼저 실패했습니다.
- `cargo test -p taide-remote --lib store:: --quiet`: 기존 상태·인증 정책 28건과 clone 상태/event/epoch 공유·원본 해제 후 채널 수명 1건으로 29건 통과.
- `cargo test -p taide --test app_services_runtime --test task_supervisor --test rust_native_phase0_contract --quiet`: 공유 조립 2건·감독/중복 시작/종료 배선 22건·IPC 계약 7건 통과.
- `cargo test -p taide --lib domain::remote:: --quiet`: 기존 adapter·WebSocket·limiter 검사 23건 통과. 이동된 28건만 기존 Tauri 검사 51건에서 빠졌고 두 crate의 합계는 새 clone 검사 1건만 늘었습니다. 변경 후 관련 검사 합계는 83건입니다.
- remote/runtime/Tauri all-target clippy(`-D warnings`)·remote/runtime strict rustdoc(`RUSTDOCFLAGS='-D warnings'`)·workspace fmt·diff 검사: exit 0. normal 의존 그래프에 Tauri가 없고 bindings SHA-256은 `174a2d617372f4782b8397351660f503194dc567ece4feb82c2a79199c09977a`로 불변입니다.

이 변경 뒤 전체 Rust workspace tests·TypeScript 검사·실제 원격 HTTP/WS 연결/세션 해제·키링 UI·GUI 종료는 미검증입니다. 앱을 실행하거나 실제 키링·시크릿 파일을 읽거나 쓰지 않았습니다. 기존 IPC DTO·command/event 등록·TS UI는 변경하지 않았습니다. Tauri 전체 strict rustdoc의 기존 private link 문제는 이번 두 순수 crate 성공 범위에 포함하지 않습니다. 원격 push는 기존 승인 거절로 재시도하지 않습니다.
