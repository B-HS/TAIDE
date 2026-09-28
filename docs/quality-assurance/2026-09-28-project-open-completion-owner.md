# 프로젝트 열기 완료 소유 QA

## 수행 결과

- [x] Native 세 열기 배선 source 검사가 최초 완료 소유 부재로 RED(exit 101)였고, 이후 `cargo test -p taide --test project_lifecycle_actions_runtime` 15건이 통과했습니다.
- [x] 자기 UUID 프로젝트·대기 capability port에서 session 기록 뒤 attach를 멈추고 요청을 취소했습니다. 성공·실패 각각 `stop_all` 뒤 정상 root가 대기했고, 재개 뒤 성공 이벤트 또는 프로젝트 close rollback 이벤트까지 확인했습니다.
- [x] `cargo test -p taide --lib domain::project::commands::tests` 11건과 `cargo test -p taide --lib typescript_바인딩을_생성한다` 1건이 통과했습니다. 생성 binding/manifest diff는 없습니다.
- [x] `cargo clippy -p taide --lib --test project_lifecycle_actions_runtime -- -D warnings`, `cargo fmt --all -- --check`, `git diff --check`가 exit 0입니다.

## 남은 gate

- [ ] fixture는 runtime action과 supervisor를 합성하며 Native 세 entry의 배선은 source/컴파일 검사입니다. 실제 AppHandle·watcher·메뉴/원격 취소 및 OS 종료는 실행하지 않았습니다.
- [ ] 다른 project action의 요청 중단, 전체 M6/M7/M8와 push는 별도입니다.
