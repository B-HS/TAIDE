# watcher Drop과 callback 실제 완료의 차이

## 대상 파일

- `crates/taide-infra/src/watcher.rs`
- `src-tauri/src/domain/file/capability.rs`, `src-tauri/src/domain/git/capability.rs`

## 리포트

기존 `WatcherHandle`은 debouncer를 Drop합니다. 설치된 `notify-debouncer-full 0.7.0`에서 Drop은 이벤트 스레드의 중지 플래그만 세우며 join하지 않습니다. 따라서 project detach나 앱 종료에서 handle이 map에서 제거됐다는 사실만으로 진행 중인 callback의 실제 완료를 입증할 수 없습니다.

## 해결과 잔여

명시적 `WatcherHandle::stop(self)`와 자기 watcher의 즉시 callback 해제 검사를 추가했습니다. 제품의 file/Git watcher 빌더는 `with_stop_scheduler`로 AppState의 `WatcherStopTracker`에 완료 소유를 등록합니다. detach나 미등록 build 결과의 Drop은 잠금 안에서 join하지 않고 별도 작업에서 `stop()`을 수행하며, 정상·직접 Exit는 감독 작업 종료 뒤 live 맵을 한 번 더 비우고 tracker가 idle일 때까지 기다립니다.

자기 UUID watcher의 map 폐기와 callback 자원 해제, 대기 future 폐기 뒤 재대기, 정상·직접 Exit의 보류 fixture를 확인했습니다. 실제 GUI callback 교착·OS watcher 오류·강제 종료 상한은 검증하지 않았습니다.
