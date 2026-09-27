# 보호된 파일 저장 action 분리

상태: M6의 이 변경 단위는 자동 검증 완료. M6 전체와 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-runtime/src/file_actions.rs`로 `save_file_within_open_projects`의 본문과 공개 문서를 이전했습니다. 이전 본문과 바이트 비교 결과가 같으며 루트/CLI 권한 확인→모드 보존 원자 저장→self-write 표시→프로젝트 hot-exit 미러 정리 순서를 유지합니다. 기존 `src-tauri/src/domain/file/service.rs`는 같은 함수를 재수출하므로 file_save와 IDE diff가 동일 action을 소비합니다.

AppState만 인수로 받는 `IdeSaveFile` 포트도 runtime으로 옮겼습니다. `AppServices::new`는 조립부의 저장 포트를 명시적으로 주입받고 Tauri State는 그 clone을 등록합니다. 기존 IDE commands의 공개 타입 경로는 재수출로 유지하며 AppServices는 이 시점에 20개 상태·포트를 조립합니다. mutation guard·blocking 실행, 프로젝트가 닫힌 뒤 IDE diff의 Forbidden 처리와 응답 순서는 기존 adapter에 남겼습니다.

runtime에 기존 로컬 taide-file 경로 의존만 추가했습니다. Cargo.lock의 변경은 이 경로 연결 한 줄이며 외부 패키지·버전과 IPC 시그니처는 불변입니다. [Rust 함수 포인터 비교](https://doc.rust-lang.org/std/ptr/fn.fn_addr_eq.html)의 제한을 확인했으며 공개 재수출 비교 외에 실제 저장·거절·미러 정리와 주입 콜백 호출을 검증했습니다.

## 검증

- 변경 전 `cargo test -p taide --test taide_file_extraction --quiet`: 기존 공개 경로·guarded save 2건 통과. 새 runtime 공개 경계·주입 검사는 E0432/E0425/E0061/E0609 6건(exit 101)으로 먼저 실패했습니다.
- `cargo test -p taide-runtime --lib file_actions:: --quiet`: CLI 승인 파일의 저장/표시와 원자 저장 실패의 기존 대상/표시/임시 파일 정책 2건 통과. 실패 단언을 Io로 강화한 뒤 같은 관련 2건을 재통과시켰습니다.
- `cargo test -p taide --test app_services_runtime --test taide_file_extraction --test rust_native_phase0_contract --test domain_boundaries --quiet`: 공유 조립 2건·공개 경로/미러 정리 2건·IPC 7건·도메인 경계 3건 통과.
- `cargo test -p taide --lib tests::ide_diff_저장은_조립부의_파일_저장_경로를_사용한다 --quiet`: IDE·파일·원격 조립 1건 통과. 관련 검사 합계는 17건입니다.
- runtime/Tauri all-target clippy(`-D warnings`)·strict runtime rustdoc(`RUSTDOCFLAGS='-D warnings'`)·fmt·diff: exit 0. 테스트 전용 Io 단언 강화 뒤 관련 runtime all-target clippy만 재확인하고 나머지 구현 동일 상태의 성공 근거는 재사용합니다. runtime normal 그래프에 Tauri가 없으며 bindings SHA-256 `174a2d617372f4782b8397351660f503194dc567ece4feb82c2a79199c09977a`는 불변입니다.

저장 테스트는 UUID로 분리한 임시 디렉터리를 정리했습니다. 실제 사용자 파일·시크릿·키링에는 접근하지 않았으며 앱도 실행하지 않았습니다. 전체 workspace tests·TypeScript 검사·실제 파일/IDE/remote와 GUI 실기는 이 변경 후 미검증입니다. 다른 file 명령의 action facade와 AppHandle callback 포트는 후속 경계입니다. 원격 push는 기존 승인 거절로 재시도하지 않습니다.
