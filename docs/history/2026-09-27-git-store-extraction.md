# GitStore 공유 상태 분리와 슬롯 회수 경합 수정

상태: M6의 이 변경 단위는 자동 검증 완료. M6 전체와 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-git/src/store.rs`가 repo root·status 캐시와 같은 repo의 push/fetch 락을 소유합니다. 공유 Arc 내부에 기존 Mutex·OnceLock을 보관하고 `crates/taide-runtime/src/app_services.rs`가 생성한 같은 clone을 `src-tauri/src/lib.rs`에서 Tauri State로 등록합니다. AppServices는 총 16개 상태·포트를 조립합니다.

`src-tauri/src/domain/git/commands.rs`는 기존 GitStore 경로를 재수출하고 repo root 해석·blocking 호출·이벤트 구독과 발행을 유지합니다. 최초 구독의 1회 실행은 store가 제어하고 Tauri 콜백이 FsChanged·GitStatusChanged·GitRefsChanged를 조회 전에 등록합니다. 기존 2초 TTL·emit 전 무효화·같은 repo 락 정책은 유지합니다. 순수 정책 19건은 Git crate로 옮겼고 기존 wire 검사 1건은 adapter에 남겼습니다.

같은 프로젝트 ID의 슬롯을 회수·재생성하면 이전 계산이 새 결과를 덮던 경합은 슬롯 identity와 generation을 함께 확인해 수정했습니다. 재현과 영향은 `docs/bug/2026-09-27-git-status-slot-reuse.md`에 기록했습니다. 기존 parking_lot·Tokio와 로컬 taide-git 의존을 조립했으며 패키지 버전은 바꾸지 않았습니다.

공유·초기화 계약은 [Arc 공식 문서](https://doc.rust-lang.org/std/sync/struct.Arc.html)·[OnceLock 공식 문서](https://doc.rust-lang.org/std/sync/struct.OnceLock.html)·[Tokio Mutex 공식 문서](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html)를 확인했습니다.

## 검증

- 변경 전 Git command 정책 20건 통과. 새 슬롯 재생성 회귀는 assertion 실패(exit 101), AppServices 공유 검사는 git 필드 부재 E0609 2건(exit 101)으로 먼저 실패했습니다.
- `cargo test -p taide-git --lib store:: --quiet`: 기존 정책·슬롯 경합·clone 간 구독 초기화 21건 통과.
- `cargo test -p taide --test app_services_runtime --test rust_native_phase0_contract --test platform_event_sink --quiet`: 공유 조립 2건·IPC 계약 7건·이벤트 29건 통과.
- `cargo test -p taide --lib domain::git::commands::tests:: --quiet`와 agent foreground PID 조립 검사: adapter 구독/wire 2건·소스 경계 1건 통과. 변경 후 합계 62건입니다.
- git/runtime/Tauri all-target clippy(`-D warnings`)·git/runtime strict rustdoc(`RUSTDOCFLAGS='-D warnings'`)·workspace fmt·diff 검사: exit 0. normal dependency graph에 Tauri가 없고 bindings SHA-256은 `174a2d617372f4782b8397351660f503194dc567ece4feb82c2a79199c09977a`로 불변입니다.

처음 지정한 `event_sink_runtime` target은 없어 해당 명령이 검사 실행 전에 실패했습니다. 실제 target 목록의 `platform_event_sink`로 수정해 위 38건을 실행했습니다. 초기 회귀 이름의 대문자 ID 경고는 소문자 id로 수정했고 검사기 억제는 사용하지 않았습니다.

이 변경 뒤 전체 Rust workspace tests·TypeScript 검사·실제 Git 초기화/상태 배지·push/fetch·GUI 종료는 미검증입니다. Tauri 전체 strict rustdoc의 기존 private link 문제도 이번 두 순수 crate의 성공 범위에 포함하지 않습니다. 기존 IPC DTO·command/event 등록·TS UI는 변경하지 않았습니다. 원격 push는 기존 승인 거절로 재시도하지 않습니다.
