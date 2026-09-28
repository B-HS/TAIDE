# watcher Drop과 callback 실제 완료의 차이

## 대상 파일

- `crates/taide-infra/src/watcher.rs`
- `src-tauri/src/domain/file/capability.rs`, `src-tauri/src/domain/git/capability.rs`

## 리포트

기존 `WatcherHandle`은 debouncer를 Drop합니다. 설치된 `notify-debouncer-full 0.7.0`에서 Drop은 이벤트 스레드의 중지 플래그만 세우며 join하지 않습니다. 따라서 project detach나 앱 종료에서 handle이 map에서 제거됐다는 사실만으로 진행 중인 callback의 실제 완료를 입증할 수 없습니다.

## 현재 해결과 잔여

명시적 `WatcherHandle::stop(self)`와 자기 watcher의 즉시 callback 해제 검사를 추가했습니다. 이 수단은 아직 제품 detach/Exit에 연결되지 않았으며, root가 live/retired watcher 완료를 기다리는 정책과 잠금 순서를 별도 구현해야 합니다.
