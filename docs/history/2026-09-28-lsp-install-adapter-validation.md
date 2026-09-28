# LSP 설치 adapter 현행 검사

## 대상 파일

- `src-tauri/tests/taide_lsp_install_store_extraction.rs`
- `src-tauri/src/lib.rs`, `crates/taide-runtime/src/exit_drain.rs`
- `crates/taide-runtime/src/lsp_install_actions.rs`, `lsp_install_toolchain.rs`

## 리포트

현재 설치 자원 완료 경로를 재검증하는 과정에서 Tauri source fixture가 이전 직접 Exit의 `lib.rs` 인라인 `LspInstallStore::wait_for_idle` 호출을 기대해 실패했습니다. 제품은 이미 `ExitDrain::wait_for_direct_exit`를 통해 같은 설치 lease를 기다리므로 fixture의 소유 경로만 현재 코드에 맞췄습니다.

## 상세

- 기존 source fixture 2건 중 설치 슬롯 1건은 통과했고 adapter 1건은 옛 인라인 대기 문자열이 없어 실패했습니다. 첫 수정은 공유 종료 분기와 direct 분기의 문자열 범위를 혼동해 순서 검사가 실패했습니다. 최종 검사는 `.run` 본문의 `TaskSupervisor::stop_all`이 직접 drain 호출보다 앞서고, 설치 store clone이 직접 drain에 전달되며, `ExitDrain`이 등록 자원 대기를 수행함을 확인합니다.
- 제품 코드·IPC·설치 정책·의존성은 변경하지 않았습니다. 직접 Exit는 사용자 결정대로 새 전역 timeout/강제 종료 없이 등록 자원 완료를 기다립니다.
- 설치 runtime 29건은 자기 임시 경로·loopback fixture만 사용합니다. 샌드박스의 listener bind 제한 5건을 제품 실패로 분류하지 않았고, 같은 입력의 권한 허용 실행에서 전건 통과를 확인했습니다.

## 검증

- `cargo test --offline -p taide-runtime --lib lsp_install_ --quiet` — 샌드박스 24/29건 통과, bind `Operation not permitted` 5건으로 exit 101. 같은 fixture의 권한 허용 실행 29/29건 통과(exit 0).
- `cargo test --offline -p taide --test taide_lsp_install_store_extraction --quiet` — 수정 전 1/2건 통과, 첫 수정 뒤 1/2건 통과, 범위 정정 뒤 2/2건 통과(exit 0).
- `cargo clippy --offline -p taide --test taide_lsp_install_store_extraction -- -D warnings`, `cargo fmt --all --check`, `git diff --check` — exit 0.
- 같은 제품 입력의 `cargo test --offline -p taide-lsp --lib install::tests --quiet` 14건과 `cargo test --offline -p taide-infra --lib lsp_install::tests --quiet` 16건은 직전 성공을 재사용합니다.

## 미검증

JD~JG의 자기 자원 합성 검사·소유 구현·배선 검증·로컬 commit은 현재 증거로 완료 처리합니다. 이는 설치 lifecycle이 모든 운영체제에서 강제로 완료된다는 판정이 아닙니다. Linux/Windows process tree, 그룹 밖 자손·OS 강제 종료 오류와 실제 설치기·native GUI Exit는 실행하지 않았으며 M6 전체의 남은 gate입니다. 직접 Exit의 새 전역 timeout·강제 종료 없이 등록 자원을 기다린다는 사용자 결정과, 그룹 밖 자손의 강제 회수 보장이 없다는 한계는 `docs/acknowledge/2026-09-28-direct-exit-drain-decision.md`에 유지합니다.
