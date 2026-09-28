# 파일 worker 종료 소유 QA

## 대상 파일과 리포트

`crates/taide-runtime/src/file_actions.rs`의 네 blocking action, Tauri file command, 원격 gateway, 기존 file integration fixture를 확인합니다. 사용자 파일·프로세스·앱은 실행하지 않고 자기 UUID 임시 파일/메모리 차단 worker만 사용합니다.

## 수행 결과

- [x] 새 guard/root owner fixture는 helper 부재 E0425(exit 101)로 먼저 실패했습니다. 구현 후 `cargo test -p taide-runtime file_actions::tests --lib` 4건이 통과했습니다. 요청 abort 뒤 실제 worker 반환 전 guard/root 대기, 정상 열기·저장·복사·mirror, 기존 보호 파일과 원자 저장 실패를 포함합니다.
- [x] `cargo test -p taide --test file_actions_runtime` 4건이 통과했습니다. CLI/root 권한과 overlay 순서, 생성·이름변경·복사, mutation guard를 기다리지 않는 mirror, Tauri adapter를 확인합니다.
- [x] `cargo clippy -p taide-runtime -p taide --lib --test file_actions_runtime -- -D warnings`와 `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps --quiet`가 exit 0입니다.
- [x] `cargo test -p taide --lib typescript_바인딩을_생성한다` 1건이 통과했고 생성된 binding/manifest diff는 없습니다. 공개 command의 wire 인자·응답은 변경하지 않았습니다.
- [x] `cargo fmt --all -- --check`, 신규 audit/bug/history/QA의 대상 Prettier check, `git diff --check`가 exit 0입니다. 전체 PROCESS/architecture의 기존 Prettier baseline과 무관한 포맷 재작성은 하지 않았습니다.

## 남은 gate

- [ ] 실제 앱 종료·원격 요청 취소·OS I/O stall과 사용자 파일/GUI 실기는 미검증입니다.
- [ ] search 4개·tree 1개의 직접 blocking worker와 전체 M6-HK/M7/M8은 미완료입니다.
