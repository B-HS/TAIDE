# 검색 요청 취소 뒤 세션 회수 누락

## 대상 파일

- `crates/taide-runtime/src/search_actions.rs`, `search_store.rs`
- `src-tauri/src/lib.rs`의 종료 경로

## 리포트

기존 `search_run`은 직접 `spawn_blocking`을 기다린 caller에서 `SearchStore::finish`를 실행했습니다. caller가 중단되면 시작한 blocking worker는 계속 실행될 수 있지만 finish는 건너뛰어 세션 token이 남고 정상 root는 worker 완료를 추적하지 못했습니다. 치환·목록 worker도 같은 미등록 경계였습니다.

## 해결 및 잔여

- 세션 소유 객체의 Drop에서 finish를 실행하며 객체를 worker로 이동했습니다. 요청 Drop은 취소 flag를 설정하고 root는 감독자에 등록된 worker 완료를 기다립니다.
- 종료 이벤트는 TaskSupervisor의 신규 입장을 먼저 닫고 SearchStore의 현재 세션 전체를 취소합니다. 이미 실행 중인 검색은 서비스의 협력적 취소 검사에서 멈춥니다.
- 실제 OS read 정지와 앱 callback 대기는 이 변경으로 시간 제한되지 않습니다. 관련 재현과 미검증 항목은 [QA](../quality-assurance/2026-09-28-search-worker-owner.md)에 기록했습니다.
