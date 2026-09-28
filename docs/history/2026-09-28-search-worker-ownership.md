# 검색 blocking worker와 세션 종료 소유

## 대상 파일

- `crates/taide-runtime/src/search_actions.rs`, `search_store.rs`
- `src-tauri/src/domain/search/commands.rs`, `src-tauri/src/remote_gateway.rs`, `src-tauri/src/lib.rs`
- `src-tauri/tests/search_actions_runtime.rs`, `docs/architecture.md`, `docs/PROCESS.md`

## 리포트

검색·치환 대상 스캔·파일별 치환·프로젝트 파일 목록의 직접 `spawn_blocking` 네 경로를 기존 TaskSupervisor에 등록했습니다. `search_run`의 세션 완료는 worker가 소유해 caller가 취소되거나 worker가 panic을 일으켜도 항목을 회수합니다. 정상 종료는 신규 작업 입장을 닫고 현재 검색 세션의 취소 flag를 설정합니다.

## 상세

1. `search_run`은 프로젝트 루트 확인 뒤 operation을 시작하고 세션을 등록합니다. 요청 Drop은 검색 취소 flag만 설정하며 이미 시작한 worker의 완료와 세션 정리는 root가 기다립니다. `search_cancel`의 명시적 취소 계약은 유지했습니다.
2. 치환은 대상 스캔 및 파일별 worker가 감독을 받고, 파일별 mutation guard와 성공 시 self-write 표시는 같은 worker 내부에 남습니다. 기존 skip 집계와 오류 접두사를 유지했습니다.
3. 목록과 검색 batch는 기존 결과·Channel 전송·비UTF-8 경로 제외를 보존합니다. Tauri/원격은 동일 AppServices 또는 TaskSupervisor State를 전달하고 공개 IPC 인자는 바뀌지 않았습니다.

## 검증

- 새 취소/root·panic 테스트, store 전체 취소 테스트, Native 검색 통합 5건과 종료 source 계약 1건이 통과했습니다. 테스트 선행 RED와 후속 검사는 [검색 QA](../quality-assurance/2026-09-28-search-worker-owner.md)에 기록했습니다.
- runtime/Tauri Clippy, strict runtime rustdoc, Rust fmt, 실제 TypeScript binding 생성은 통과했고 binding/manifest 변경은 없었습니다.
- 실제 사용자 프로젝트·앱은 실행하지 않았습니다. OS 파일 I/O 정지 시 worker 강제 중단이나 종료 시간 상한은 보장하지 않으며 M6-HK 전체는 별도입니다.
