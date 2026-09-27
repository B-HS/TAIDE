# PTY 종료 관찰 오류의 소유 child wait

## 대상과 원인

`crates/taide-infra/src/pty.rs`의 `PtyChildWaitOwner::finish`는 Unix의 `wait_for_exit_unreaped` 오류 뒤 killer 권한을 닫고 바로 반환했습니다. 그 뒤 owner Drop은 권한이 없다는 이유로 `child.wait()`를 시도하지 않아 소유 child가 회수되지 않는 경로가 있었습니다. 정상 관찰의 권한 반납→wait 순서는 유지합니다.

## 수정과 재현

자기 `/bin/sh -c 'exit 0'` child를 `WaitRecordingChild`로 감싸 Unix downcast가 `Unsupported`를 반환하도록 했습니다. 테스트는 종료 관찰 오류와 공유 killer 권한 반납을 유지하면서도 wrapper의 실제 wait 호출을 요구합니다. 수정 전 `was_waited` assertion이 exit 101로 실패했고, fixture가 별도로 child를 wait해 남은 자기 프로세스를 회수했습니다. 수정 뒤 관찰 결과와 무관하게 killer 권한을 먼저 반납하고 `child.wait()`를 시도한 다음 원래 관찰 오류를 반환합니다. Windows의 wait→권한 반납 순서는 변경하지 않았습니다.

## 검증과 한계

신규 1건과 기존 PTY 32건을 포함한 `cargo test -p taide-infra --lib pty::tests -- --skip drop은_일시정지된 --skip drop은_셸_통합` 33건이 통과했습니다. `cargo clippy -p taide-infra -p taide-terminal -p taide-runtime --all-targets -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check`도 exit 0입니다. 소유 child wrapper의 `Unsupported`는 실제 커널 waitid 실패가 아니며, `child.wait()` 자체가 오류를 반환하거나 child가 종료되지 않는 경우의 실제 회수/종료 상한을 증명하지 않습니다. SIGHUP 무시·그룹 이탈 자손·Windows·직접 native Exit·전체 M6/M7/M8은 여전히 미완료입니다.
