# IdeStore 공유 상태 분리

상태: M6의 이 변경 단위는 자동 검증 완료. M6 전체와 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-ide/src/store.rs`가 서버/연결 Tokio 핸들·pending diff/save 응답·현재/최신 선택·진단 준비 상태·클라이언트 수를 공유 Arc<Mutex>에 보관합니다. 알림 broadcast sender도 같은 채널을 공유합니다. `crates/taide-runtime/src/app_services.rs`가 생성한 동일 인스턴스를 `src-tauri/src/lib.rs`에서 Tauri State에 등록하며 AppServices는 총 18개 상태·포트를 조립합니다.

`src-tauri/src/domain/ide/store.rs`는 기존 상태 타입과 선택 snapshot의 공개 경로를 재수출합니다. readiness 대기의 Boolean 판단만 순수 crate로 이전하고 실제 AppState 설정 조회·대기 시간·PTY 환경 주입은 Tauri adapter에 유지했습니다. 서버/연결은 감독자가 원래 반환하던 Tokio 핸들을 그대로 저장하며 중복 시작·종료 뒤 등록 거부·완료된 연결 회수 정책은 유지합니다. MCP 서버·lockfile·layout/file action·상태 이벤트·연결별 JoinSet 조립도 adapter에 남겼습니다.

원격 owner 라벨은 `crates/taide-model/src/remote.rs`를 단일 출처로 옮겼습니다. 기존 remote crate와 Tauri 경로는 같은 상수를 재수출해 값과 wire를 유지하며 IDE crate에 remote crate 의존을 추가하지 않았습니다. 기존 parking_lot·Tokio와 로컬 IDE 의존만 연결하고 패키지 버전은 바꾸지 않았습니다.

[Tokio JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)의 완료/취소와 [broadcast Sender](https://docs.rs/tokio/latest/tokio/sync/broadcast/struct.Sender.html)의 공유 채널 계약을 확인했습니다.

## 검증

- 변경 전 store 정책 14건 통과. 새 AppServices 공유 검사는 ide 필드 부재 E0609 3건(exit 101)으로 먼저 실패했습니다.
- `cargo test -p taide-ide --lib store:: --quiet`: 이전된 상태/readiness 정책 14건과 clone pending/진단/알림·원본 해제 후 채널 수명 1건으로 15건 통과.
- `cargo test -p taide --test app_services_runtime --test task_supervisor --test rust_native_phase0_contract --test domain_boundaries --test platform_event_sink --quiet`: 공유 조립 2건·감독 22건·IPC 7건·도메인 경계 3건·이벤트 배선 29건 통과.
- 권한 허용 루프백의 `cargo test -p taide --lib domain::ide:: --quiet`: MCP 토큰 핸드셰이크/경로 정책 4건 통과. `tests::ide_`와 탭 닫기 후처리 조립 검사 3건도 통과했습니다. 이전된 14건만 Tauri 기존 IDE 검사 18건에서 빠졌고 두 crate 합계는 clone 회귀 1건만 늘었습니다. 변경 후 관련 검사 합계는 85건입니다.
- IDE/runtime/Tauri all-target clippy(`-D warnings`)·IDE/runtime/model/remote strict rustdoc(`RUSTDOCFLAGS='-D warnings'`)·workspace fmt·diff 검사: exit 0. normal IDE/runtime 그래프에 Tauri가 없고 IDE는 remote crate를 참조하지 않습니다. bindings SHA-256은 `174a2d617372f4782b8397351660f503194dc567ece4feb82c2a79199c09977a`로 불변입니다.

이 변경 뒤 전체 Rust workspace tests·TypeScript 검사·실제 외부 MCP 클라이언트/선택/진단·PTY env·다중 창 탭 닫기·GUI 종료는 미검증입니다. 앱을 실행하거나 실제 lockfile·키링·시크릿 파일을 읽거나 쓰지 않았습니다. 기존 IPC DTO·command/event 등록·TS UI는 변경하지 않았습니다. Tauri 전체 strict rustdoc의 기존 private link 문제는 이번 네 순수 crate 성공 범위에 포함하지 않습니다. 원격 push는 기존 승인 거절로 재시도하지 않습니다.
