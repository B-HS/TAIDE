# AppServices 이벤트 포트 주입

상태: M6의 이 변경 단위는 자동 검증 완료. M6 전체와 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-runtime/src/app_services.rs`는 `Arc<dyn EventSink>`를 생성자에서 명시적으로 받아 `events` 필드에 보유합니다. [Rust Arc 공유 소유권](https://doc.rust-lang.org/std/sync/struct.Arc.html)에 따라 호출자는 같은 발행 구현의 clone을 사용할 수 있습니다. runtime은 Tauri나 UI toolkit을 알지 않습니다.

`src-tauri/src/platform/services.rs`의 기존 TauriPlatformServices가 EventSink도 구현하며 `TauriEventSink(&self.0).publish(event)`로 위임합니다. `src-tauri/src/lib.rs`는 한 platform Arc를 만들어 기존 OS 포트와 이벤트 포트 양쪽에 제공합니다. 기존 빌린 TauriEventSink 타입과 30종 wire 매핑, 대상 창·remote relay·발행 오류 처리에는 변경이 없습니다. 새 adapter 타입·이벤트 버스·의존성도 추가하지 않았습니다.

AppServices의 필드는 이 시점에 21개이며 기존 20개 상태·포트의 Tauri State 등록은 유지합니다. events는 AppServices를 통해 제공하고 별도 Tauri State로 등록하지 않습니다. 기존 command의 이벤트 발행 경로는 그대로입니다.

## 검증

- 새 플랫폼 trait·주입·공유 검사는 E0277 1건·E0061 1건·E0609 3건(exit 101)으로 먼저 실패했습니다.
- `cargo test -p taide --test app_services_runtime --test platform_services_runtime --test platform_event_sink --test rust_native_phase0_contract --quiet`: 공유 조립 2건·platform 정책/trait 경계 4건·이벤트 배선 29건·IPC 7건, 총 42건 통과.
- 주입 Arc identity와 두 clone의 이벤트 payload·발행 순서를 메모리 sink에서 확인했습니다. TauriPlatformServices의 EventSink 구현은 컴파일 타임 함수 시그니처로 검증하고 기존 매핑 위임은 소스 경계 검사로 확인했습니다. 실제 Tauri 앱을 생성하거나 이벤트를 OS 창에 발행하지 않았습니다.
- runtime/Tauri all-target clippy(`-D warnings`)·strict runtime rustdoc(`RUSTDOCFLAGS='-D warnings'`)·workspace fmt·diff: exit 0. runtime normal 그래프에 Tauri가 없고 bindings SHA-256 `174a2d617372f4782b8397351660f503194dc567ece4feb82c2a79199c09977a`는 불변입니다.

전체 workspace tests·TypeScript 검사·실제 다중 창/remote relay·GUI 실기는 이 변경 후 미검증입니다. 나머지 application action facade와 AppHandle callback adapter는 후속 경계입니다. 사용자 파일·키링·시크릿에는 접근하지 않았으며 원격 push는 기존 승인 거절로 재시도하지 않습니다.
