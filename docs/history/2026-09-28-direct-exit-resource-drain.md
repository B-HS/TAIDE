# 직접 Exit 전체 자원 완료 대기

## 대상 파일

- `src-tauri/src/lib.rs`
- `crates/taide-runtime/src/exit_drain.rs`
- `docs/architecture.md`, `docs/PROCESS.md`, 종료 lifecycle QA와 사용자 결정 문서

## 리포트

기존 `ExitRequested`는 감독 작업·설치·LSP·AI·PTY를 비동기로 기다렸지만 직접 `Exit`는 감독 중지와 설치 lease 대기만 수행했습니다. 사용자 A 선택에 따라 두 경로가 같은 등록 자원 완료 함수를 사용하게 했습니다. 직접 경로는 이벤트 루프 종료 callback에서 `block_on`으로 해당 함수를 기다립니다.

## 상세

1. `ExitDrain::wait_for_owned_resources`는 감독 작업과 operation owner, 설치 lease, LSP process worker, AI request owner, PTY worker 순서로 대기합니다. 기존 정상 종료의 준비 플래그·콜백은 이 함수의 성공 뒤에만 실행됩니다.
2. `wait_for_direct_exit`는 위 자원들의 신규 입장을 닫고 감독 작업 취소를 요청한 다음 같은 대기 함수를 호출합니다. 기존 공통 `RunEvent` 분기의 AppState·server·layout shutdown과 공개 IPC는 변경하지 않았습니다.
3. 새 감독 작업·설치·AI owner fixture는 API 부재 E0599로 먼저 실패했고 구현 뒤 통과했습니다. Unix 자기 `/bin/sh` fixture에서는 LSP와 PTY exit callback을 각각 보류했다가 풀어 직접 경로가 두 반환을 모두 기다림을 확인했습니다. Tauri source contract는 실제 `Exit` 분기의 공유 drain 호출과 네 State 인수를 확인했습니다.
4. `ExitRequested`에서 실제 native callback을 계속 처리하는 조건과 달리 직접 `Exit`에서는 메인 이벤트 루프 응답이 멈출 수 있습니다. OS I/O·main-thread callback이 끝나지 않으면 사용자 선택대로 시간 제한 없이 기다립니다. 실제 앱·사용자 프로세스·GUI는 실행하지 않았고 강제 종료나 등록되지 않은 자원의 회수도 증명하지 않았습니다.

## 검증

- `cargo test -p taide-runtime exit_drain::tests --lib`: 기존 경로와 첫 직접 fixture 8건 통과, 새 LSP/PTY 직접 fixture 1건 별도 통과.
- `cargo test -p taide --lib 직접_exit은_전체_자원_drain을_사용한다`: 1건 통과.
- `cargo clippy -p taide-runtime -p taide --lib --tests -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps --quiet`, `cargo fmt --all -- --check`, `git diff --check`가 exit 0입니다. 신규 acknowledge/history 및 종료 QA의 대상 Prettier check도 exit 0이며 공개 IPC/생성 입력은 변경하지 않았습니다.
