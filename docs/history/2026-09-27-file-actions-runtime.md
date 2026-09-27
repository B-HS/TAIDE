# 파일 application action 15개 runtime 분리

상태: 이 M6 변경 단위는 자동 검증 완료. M6 전체는 잔여 DI·adapter·감독 경계 확인 전까지 미완료입니다. 현재 사용자 목표는 M6까지 완료한 뒤 commit·push이며 M7·M8 구현은 진행하지 않습니다.

## 대상과 변경

`crates/taide-runtime/src/file_actions.rs`가 창 flush 완료 확인을 제외한 15개 파일 command의 application action을 제공합니다. 기존 `src-tauri/src/domain/file/commands.rs`는 같은 공개 함수 경로·주입 인수 타입·응답 타입을 유지하며 runtime에 위임합니다. save/mirror의 사용하지 않는 AppHandle 인수 이름만 `_app`으로 바뀌었고 생성된 IPC 인수에는 영향이 없습니다. 기존 `save_file_within_open_projects` 본문과 `IdeSaveFile` 재수출은 보존했습니다.

루트/CLI 승인, rename/delete의 entry 권한, mutation guard, self-write, mirror의 display path와 disk baseline, 오류 변환 순서를 유지했습니다. file_open은 권한 확인과 설정 snapshot 뒤에만 plugin overlay callback을 호출합니다. open/save/copy/dirty mirror의 blocking 작업은 Tokio로 실행하며 save/dirty mirror는 공유 AppState clone을 소유합니다. untitled mirror와 목록·clear·prune의 기존 동기 I/O 배치는 변경하지 않았습니다. dirty/untitled mirror 생성은 기존처럼 전역 mutation guard를 취하지 않습니다.

실제 창 label을 확인하는 file_flush_complete의 본문과 exit 조건은 Tauri에 그대로 남습니다. file_read_raw는 runtime에서 바이트를 반환하고 Tauri에서만 ipc::Response를 조립합니다. AppServices의 21개 필드·기존 Tauri State 등록 20개, 외부 dependency 버전과 Cargo.lock은 변경하지 않았습니다.

[Tokio spawn_blocking](https://docs.rs/tokio/1.53.1/tokio/task/fn.spawn_blocking.html)의 closure 소유권과 실행·취소 계약, [Tauri async runtime](https://docs.rs/tauri/latest/src/tauri/async_runtime.rs.html)의 Tokio 위임을 확인했습니다. Tauri JoinError는 [공식 오류 정의](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri/src/error.rs)의 transparent 변환이므로 기존 Internal 문자열 변환을 유지합니다. 시작된 blocking 작업의 취소 정책을 새로 바꾸지 않았습니다.

## 검증

- 변경 전 `cargo test -p taide --test taide_file_extraction --quiet`: 기존 2건 통과. 새 `file_actions_runtime` 검사는 비공개 module E0603(exit 101)으로 먼저 실패했습니다.
- `cargo test -p taide --test file_actions_runtime --test taide_file_extraction --test app_services_runtime --test domain_boundaries --quiet`: 새 action 4건·기존 경로/저장 2건·공유 조립 2건·도메인 경계 3건 통과. CLI 승인/외부 거절, overlay 호출 순서, 생성/저장/rename/copy, mirror/untitled 목록·prune·clear·안전 ID, 전역 mutation 대기 여부와 15개 adapter 위임을 확인했습니다.
- `cargo test -p taide-runtime --lib file_actions:: --quiet`: 기존 CLI 저장/실패 정책 2건 통과. `cargo test -p taide-file --quiet`: 기존 파일 정책 60건 통과. `cargo test -p taide --lib tests::파일_git_ide_vsix는_조립부의_플러그인_포트를_사용한다 --quiet`: 조립 포트 1건 통과.
- bindings 생성 1건 통과 뒤 실제 diff는 공개 API 문서 5개만 변경되었고 command/type/인수/응답은 불변입니다. SHA-256은 `49ff1b20f9fedd9001c5443014fb86608dadac8d93dc45180030012b088742a4`이며 manifest의 digest 한 줄을 동기화했습니다. `cargo test -p taide --test rust_native_phase0_contract --quiet`: 203 command·30 event·raw channel 등을 확인하는 기존 7건 통과. 관련 동작/경계 검사는 합계 81건이며 bindings 생성 1건은 별도입니다.
- `cargo clippy -p taide-runtime -p taide --all-targets -- -D warnings`, strict runtime rustdoc, `cargo fmt --all --check`, `git diff --check`: exit 0. runtime normal 의존 그래프에 Tauri가 없습니다. manifest Prettier 경고(exit 1)는 변경 전 HEAD에서도 재현되므로 무관한 전체 재포맷 없이 digest 한 줄만 수정했습니다. 생성 bindings는 기존 formatter 제외 정책을 유지합니다.

새 테스트는 UUID로 분리한 임시 디렉터리를 정리했습니다. 실제 사용자 파일·시크릿·키링에는 접근하지 않았고 앱도 실행하지 않았습니다. 새 delete action의 외부 경로 거절은 검증했으나 실제 사용자 파일의 휴지통 이동은 실행하지 않았습니다. 전체 workspace·TypeScript·GUI 실기 gate는 이번 변경의 검증 범위가 아닙니다. 사용자는 이후 GitHub B-HS/TAIDE의 to_rust_native로 기존 미푸시 커밋을 포함한 일반 push를 명시 승인했으며 M6 전체 완료 후 적용합니다. 승인 범위는 `docs/acknowledge/2026-09-27-m6-completion-push-approval.md`에 기록했습니다.
