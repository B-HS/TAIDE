# watcher 명시 중지의 join 수단

## 대상 파일

- `crates/taide-infra/src/watcher.rs`

## 리포트

`WatcherHandle::stop(self)`를 추가해 소유한 debouncer의 이벤트 스레드 종료와 callback 해제를 기다릴 수 있게 했습니다. 기존 Drop 경로는 변경하지 않았습니다. 설치된 `notify-debouncer-full 0.7.0`의 [Debouncer 문서](https://docs.rs/notify-debouncer-full/0.7.0/notify_debouncer_full/struct.Debouncer.html)와 실제 원천에서 Drop은 중지 플래그만 세우고 `stop(self)`는 thread join을 수행하는 차이를 확인했습니다.

## 검증과 잔여

자기 UUID watcher의 명시 중지 검사는 API 부재 E0599(exit 101)로 먼저 실패했습니다. 구현 뒤 callback 자원 해제를 반환 직후 확인했고 watcher 검사 26건, infra Clippy·strict rustdoc·Rust fmt/diff가 통과했습니다. [QA](../quality-assurance/2026-09-28-watcher-explicit-stop.md)에 명령을 기록합니다.

project detach·부팅 restore·정상/직접 Exit에서 이 API를 호출하거나 완료를 감독하는 배선은 아직 없습니다. 기존 live/retired watcher는 Drop으로 중지 요청만 하므로 M6 watcher 종료 gate는 미완료입니다. 실제 사용자 프로젝트·앱은 사용하지 않았습니다.
