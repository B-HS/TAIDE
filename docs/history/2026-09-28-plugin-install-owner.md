# plugin·VSIX 설치 취소의 staging 소유와 정상 root 대기

## 대상 파일

- `crates/taide-runtime/src/plugin_install_worker.rs`, `src/plugin_actions.rs`, `src/vsix_actions.rs`, `src/lib.rs`
- `src-tauri/src/domain/plugin/commands.rs`, `src/domain/vsix/commands.rs`, `src/lib.rs`, `tests/plugin_actions_runtime.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, `docs/bug/2026-09-28-plugin-staging-cancellation.md`

## 리포트

요청이 취소된 뒤 남는 plugin staging을 자기 UUID fixture로 재현하고 전체 설치 작업·nested stage를 같은 TaskSupervisor로 감독했습니다. 서비스가 반환한 staging 경로는 RAII로 소유하며 정상 root는 이미 시작한 stage와 cleanup 시도의 완료를 기다립니다.

## 상세

1. 기존 요청은 blocking stage 반환 또는 mutation guard 대기 중 Drop될 수 있으며 raw PathBuf에는 cleanup이 없습니다. RED에서 자기 staging 1개가 남아 기대 0과 달랐습니다.
2. 전체 async 설치에 Drop 시 abort하는 요청 waiter를 두고 별도의 blocking stage를 등록했습니다. request/root abort가 stage waiter를 Drop해도 이미 시작한 blocking worker는 감독에 남습니다. 수신자가 없으면 worker 안의 결과 전송 실패가 반환 artifact를 Drop하고, 이미 반환됐으면 async 작업 Drop이 정리합니다.
3. stage 뒤 mutation guard와 기존 중복 최종 검사·atomic commit·cache 순서는 유지합니다. VSIX는 같은 조립 함수 포인터와 AppHandle을 소유한 Send callback으로 실행합니다. runtime에는 Tauri 의존을 추가하지 않았고 기존 등록 감독자만 State로 주입합니다. 내부 Rust signature는 바뀌었으나 실제 생성 wire payload는 동일합니다.
4. cleanup은 서비스가 생성·반환한 자기 staging 경로에 한정됩니다. 정상 rename 뒤에는 그 경로가 없고 원본 archive/source·설치본은 삭제하지 않습니다. 서비스 내부에서 경로 반환 전 panic하거나 OS가 삭제를 거부하는 경우의 완전 회수·강제 bounded 종료는 보장하지 않습니다.
5. 실제 서비스 stage/commit·자기 UUID archive와 메모리 차단 worker만 실행했습니다. 사용자 plugin·앱·외부 설치/네트워크·사용자 프로세스는 실행하지 않았습니다.

## 검증 기록

기존 요청 abort 테스트는 staging count 1≠0으로 exit 101이었습니다. 수정 뒤 같은 회귀와 기존 정상/오류/cache/VSIX guard 검사를 포함한 최종 action 10건이 통과했습니다. owner 4·action 10·포트 1·실제 생성 1·IPC 7, 서로 다른 검사 23건입니다.

| 명령 | 실제 결과 |
| --- | --- |
| `cargo test -p taide-runtime --lib plugin_install_worker::tests` | owner 4건, exit 0 |
| `cargo test -p taide --test plugin_actions_runtime` | 최종 action 10건, exit 0 |
| `cargo test -p taide --lib 파일_git_ide_vsix는_조립부의_플러그인_포트를_사용한다` | 같은 포트/guard 배선 1건, exit 0 |
| `cargo test -p taide --lib tests::typescript_바인딩을_생성한다` | 실제 생성 1건, exit 0 |
| `cargo test -p taide --test rust_native_phase0_contract` | IPC 7건, exit 0 |
| `cargo clippy -p taide-runtime --all-targets -- -D warnings` | exit 0 |
| `cargo clippy -p taide --lib --test plugin_actions_runtime -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps` | exit 0 |
| `cargo fmt --all -- --check`, `git diff --check` | exit 0 |

생성 binding digest `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`, manifest `343950f91338121d04edd36d5aa8d09491437cdb78e096fa0f519d3315a04bce`는 불변입니다. 의존 입력 불변으로 runtime normal graph 543줄의 Tauri 0개 성공을 재사용했습니다. TaskSupervisor/ExitDrain 코드는 변경하지 않았으며 새 helper의 실제 root 경로를 검사했습니다.

## 남은 경계

정적 entry 배치 F173/S0/A13/P20은 그대로입니다. locale 보안 선택·Git guard/worker·나머지 application/root·직접 Exit/OS 오류·M6/M7/M8는 미완료입니다. M6 전체 완료 뒤 일반 push 조건을 유지합니다.
