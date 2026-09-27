# SecretStore 포트 공유 조립

상태: M6의 이 변경 단위는 자동 검증 완료. M6 전체와 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-infra/src/secret.rs`의 기존 `SecretStoreState(Arc<dyn SecretStore>)`에 Clone을 유도했습니다. [Rust Arc](https://doc.rust-lang.org/std/sync/struct.Arc.html)의 공유 소유권 계약에 따라 복제본은 같은 저장소 구현을 가리킵니다.

`crates/taide-runtime/src/app_services.rs`는 시크릿 포트를 생성자 인수로 받아 보유합니다. OS 키링 구현과 service identifier 선택은 `src-tauri/src/lib.rs` 조립부가 계속 맡으며 기존 Tauri State에는 `services.secrets.clone()`을 등록합니다. AppServices는 이 시점에 19개 상태·포트를 조립합니다. 생성자는 문자열과 Arc만 보관하므로 등록 위치 변경으로 키링 접근이 앞당겨지지 않습니다.

AI·remote·sync의 기존 소비 경로, account 이름, KeyringSecretStore 구현, 오류·NoEntry 정책과 IPC는 변경하지 않았습니다. Cargo 의존성이나 버전도 바꾸지 않았습니다. 원격 WebSocket 소스 조립 검사의 기존 등록 문자열만 새 위치에 맞췄습니다.

## 검증

- 메모리 포트를 전달한 AppServices 공유 검사는 생성자 인수 부족 E0061 1건·필드 부재 E0609 3건(exit 101)으로 먼저 실패했습니다.
- `cargo test -p taide-infra --lib secret:: --quiet`: 메모리 저장소·계정 정책 4건 통과.
- `cargo test -p taide --test app_services_runtime --test taide_infra_extraction --test rust_native_phase0_contract --quiet`: 공유 조립 2건·기존 infra 공개 경로 16건·IPC 계약 7건 통과. 주입된 Arc identity와 clone 간 저장·조회·삭제를 메모리에서 확인했습니다.
- `cargo test -p taide --lib tests::원격_websocket은_조립부의_json_raw_게이트웨이를_사용한다 --quiet`: 조립 소스 1건 통과. 관련 검사 합계는 30건입니다.
- infra/runtime/Tauri all-target clippy(`-D warnings`)·infra/runtime strict rustdoc(`RUSTDOCFLAGS='-D warnings'`)·workspace fmt·diff 검사: exit 0. runtime normal 그래프에 Tauri가 없고 bindings SHA-256 `174a2d617372f4782b8397351660f503194dc567ece4feb82c2a79199c09977a`는 불변입니다.

실제 OS 키링·시크릿 파일을 읽거나 쓰지 않았으며 앱도 실행하지 않았습니다. 이 변경 뒤 전체 workspace tests·TypeScript 검사·실제 provider/remote/sync와 GUI 실기는 미검증입니다. Tauri 전체 strict rustdoc의 기존 링크 문제는 순수 두 crate의 성공 범위에 포함하지 않습니다. 원격 push는 기존 승인 거절로 재시도하지 않습니다.
