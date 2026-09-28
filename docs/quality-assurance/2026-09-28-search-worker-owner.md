# 검색 worker 종료 소유 QA

## 대상 파일과 리포트

`search_actions.rs`의 네 blocking 경로, `SearchStore` 전체 취소, Tauri/원격 배선과 종료 source 계약을 확인합니다. 자기 메모리·UUID 임시 프로젝트만 사용하고 실제 사용자 파일·앱을 실행하지 않습니다.

## 수행 결과

- [x] 새 `SearchStore::cancel_all` 부재 E0599(exit 101)를 먼저 확인했습니다. 후속 store 테스트 1건이 통과했습니다.
- [x] 검색 요청 abort/root와 panic 정리 fixture의 worker helper 부재를 RED로 확인했습니다. `cargo test -p taide-runtime search_actions::tests --lib` 3건이 통과했습니다. panic 오류 접두사 회귀는 일반 helper에서 먼저 실패한 뒤 원래 접두사로 수정했습니다.
- [x] `cargo test -p taide --test search_actions_runtime` 5건이 통과했습니다. 검색 batch·명시적 취소·치환 skip/경로·목록과 닫힌 감독자 admission을 포함합니다.
- [x] Tauri 종료 source 계약 1건, `cargo clippy -p taide-runtime -p taide --lib --test search_actions_runtime -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps --quiet`, 실제 TypeScript binding 생성 1건이 exit 0입니다. binding/manifest diff는 없습니다.
- [x] `cargo fmt --all -- --check`, 대상 MD Prettier check, `git diff --check`가 exit 0입니다. 기존 PROCESS/architecture 전체 포맷은 무관한 재작성 없이 유지했습니다.

## 남은 gate

- [ ] 실제 OS read stall·native/remote 요청 취소와 GUI 실기는 미검증입니다.
- [ ] 다른 AppServices/callback 경계를 포함한 M6-HK 전체와 M7/M8은 미완료입니다.
