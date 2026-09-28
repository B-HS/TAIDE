# 파일 application blocking worker 종료 소유

## 대상 파일

- `crates/taide-runtime/src/file_actions.rs`
- `src-tauri/src/domain/file/commands.rs`, `src-tauri/src/remote_gateway.rs`, `src-tauri/tests/file_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, 관련 bug·QA

## 리포트

file_open/save/copy/mirror_dirty의 네 직접 `spawn_blocking`을 같은 AppServices TaskSupervisor의 결과형 worker로 이전했습니다. save/copy는 worker에 이동 가능한 owned mutation guard와 operation lease를 전달합니다. Tauri 공개 command와 원격 gateway는 기존 등록 감독자 State만 추가 전달하고 인자·응답·생성 TypeScript binding은 그대로입니다.

## 상세

1. `run_guarded_file_worker`는 종료 admission을 guard 대기 전에 시작하고 기존 guard 뒤 prepare 순서를 유지합니다. worker가 끝나면 guard와 lease가 함께 해제됩니다. 요청이 취소돼도 이미 시작한 worker는 실제 완료까지 supervisor가 추적합니다.
2. copy는 기존 root 검증을 guard 안에서 수행하고 복사 뒤 self-write 표시를 worker 안으로 이동했습니다. save는 기존 보호된 atomic 저장 함수를 그대로 호출합니다.
3. open은 root/CLI 권한 확인 뒤 plugin overlay를 lazy 호출하는 순서를 보존합니다. dirty mirror는 전역 mutation guard를 취하지 않고 worker만 감독합니다. 네 action은 시작 시 supervisor 입장을 확인합니다.
4. 자기 UUID 파일로 열기·저장·복사·dirty mirror 결과, CLI/root 거절, mutation guard 비취득과 종료 뒤 신규 작업 거절을 확인했습니다. 합성 차단 worker는 요청 abort 뒤 guard와 root owner가 실제 worker 반환까지 유지됨을 확인했습니다. 초기 자기 파일 테스트의 macOS 임시 경로 정규화 기대 차이를 테스트 비교 경로에 반영했고 코드 동작은 바꾸지 않았습니다.

## 검증

- helper 부재 E0425(exit 101) 뒤 `cargo test -p taide-runtime file_actions::tests --lib`: 4건 통과.
- 기존 integration의 State 인수 누락 E0061을 실제 배선에 맞춘 뒤 `cargo test -p taide --test file_actions_runtime`: 4건 통과.
- `cargo clippy -p taide-runtime -p taide --lib --test file_actions_runtime -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps --quiet`, 실제 `typescript_바인딩을_생성한다`: exit 0. 생성된 binding/manifest diff는 없습니다.
- `cargo fmt --all -- --check`, 신규 audit/bug/history/QA의 대상 Prettier check, `git diff --check`가 exit 0입니다. 전체 PROCESS/architecture의 기존 Prettier baseline은 재작성하지 않았습니다. 실제 GUI·사용자 파일, OS I/O stall, search/tree의 미등록 blocking worker는 별도 M6 gate입니다.
