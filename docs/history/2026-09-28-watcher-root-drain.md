# watcher callback 완료의 정상·직접 종료 소유

## 대상 파일

- `crates/taide-infra/src/watcher.rs`
- `crates/taide-runtime/src/watcher_stop.rs`, `crates/taide-runtime/src/state.rs`, `crates/taide-runtime/src/exit_drain.rs`
- `src-tauri/src/domain/file/capability.rs`, `src-tauri/src/domain/git/watch.rs`, `src-tauri/src/lib.rs`

## 리포트

설치된 `notify-debouncer-full 0.7.0`의 debouncer Drop은 중지 요청만 하고 스레드를 join하지 않습니다. 앞선 단위에서 추가한 `WatcherHandle::stop(self)`를 제품의 file/Git watcher에 연결했습니다. 두 빌더가 만든 handle은 등록 전부터 AppState의 `WatcherStopTracker`에 중지 작업을 예약할 수 있습니다. detach·중복 attach·복원 중 미등록 결과의 Drop은 mutation guard 안에서 join하지 않으며 tracker의 별도 스레드가 callback 반환과 debouncer 스레드 join을 완료합니다.

## 상세

1. `WatcherStopTracker`는 예정된 중지 작업 수와 변경 통지를 보유합니다. 대기 future가 취소돼도 작업의 완료 ticket은 남고, 재시도한 대기자는 같은 작업의 실제 종료를 기다립니다.
2. AppState shutdown은 file/Git의 live handle 맵을 비웁니다. ExitDrain은 감독된 build·attach 작업이 끝난 뒤 맵을 다시 비우고 tracker가 idle일 때까지 대기합니다. 정상 ExitRequested와 직접 Exit가 같은 완료 조건을 사용합니다.
3. `WatcherHandle::stop(self)`의 동기 경로는 그대로 유지합니다. 제품의 Drop 경로만 host scheduler를 사용하며 공개 IPC·이벤트 형식·프로젝트 capability 순서는 바꾸지 않았습니다.

## 검증과 잔여

자기 UUID watcher의 Drop→중지 예약→callback 자원 해제, live 맵 폐기 뒤 지연된 실제 완료, 대기 future 폐기·재대기, 정상·직접 Exit 보류 fixture를 추가했습니다. 관련 명령과 결과는 [QA](../quality-assurance/2026-09-28-watcher-root-drain.md)에 기록합니다.

실제 macOS GUI의 watcher callback 교착, OS watcher 오류·스레드 생성 실패, 강제 종료 상한, Windows 실기는 검증하지 않았습니다. M6 전체·M7·M8와 원격 push는 별도 gate입니다.
