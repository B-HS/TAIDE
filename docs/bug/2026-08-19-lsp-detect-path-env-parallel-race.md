# LSP 서버 탐지 테스트의 전역 PATH env 병렬 경합 (2026-08-19)

## 대상 파일
- `src-tauri/src/domain/lsp/service.rs`(`find_in_path`·`detect_servers` 체인) — 수정
- `src-tauri/src/domain/lsp/commands.rs`(`spawn_process`·`lsp_detect_servers` 프로덕션 호출부) — 수정

## 증상
- `cargo test --workspace`(병렬 기본) 산발 실패: `jdtls는…available`(service.rs)·
  `confirms_healthy_restart는…`(commands.rs) 대표. `--test-threads=1`·재실행은 항상 통과. 원본 HEAD
  에서도 재현되는 기존 결함(캠페인·감사 무관). 세션 내내 방치돼 verify 게이트를 산발 파손.
- 세 번째 flaky(앞선 serde_json float_roundtrip·kill 폴링 레이스에 이어).

## 근본 원인 (메인 진단 확정)
- `find_in_path`(service.rs:38)가 전역 `env::var_os("PATH")` 를 읽고, service.rs 테스트 4곳이 전역
  `env::set_var("PATH", ...)` 로 조작해 detect_servers 를 호출·복원. 같은 lib 테스트 바이너리의 스레드
  병렬 실행에서 (a) 조작 테스트가 PATH 를 바꾼 사이 다른 테스트(detect_servers 호출·PATH 미조작)가
  오염된 PATH 를 봄, (b) 조작 중 다른 조작 테스트가 또 바꿈.
- **confirms_healthy_restart 도 collateral damage**: 그 테스트의 `sleeping_proc()` 이 `Command::new("sh")`
  (비한정 명령어)를 스폰 → OS 가 스폰 시점 PATH 로 `sh` 검색. service.rs 테스트가 PATH 를 "sh" 없는 임시
  디렉터리로 바꾼 순간과 겹치면 `sh` 스폰 실패 → `.expect("프로세스 spawn 성공")` 패닉. 로직·타이밍 문제
  아님(스폰 직후 5초 sleep 프로세스 즉시 확인이라 레이스 여지 없음).

## 수정
- **전역 env 조작 완전 제거·PATH 파라미터 주입**(컨벤션: env 주입으로 테스트 가능화):
  - `find_in_path_within(command, path_var: &OsStr)` 신설(순수 로직·env 미접근). `find_in_path`(생산
    편의 래퍼, 유일 직접 소비처 `run_toolchain_install`)만 OS 경계에서 env 를 읽어 위임.
  - `resolve_command`·`resolve_spec_command`·`evaluate_sdk_probes`·`detect_servers` 시그니처에
    `path_var: &OsStr` 추가.
  - 프로덕션 호출부 2곳: `spawn_process`(commands.rs:259)·`lsp_detect_servers`(commands.rs:722)가
    `env::var_os("PATH").unwrap_or_default()` 를 읽어 전달(동작 무변경).
  - 테스트 4곳의 `set_var`/`remove_var("PATH")` 전부 삭제 — 통제 필요분은 `path_dir.as_os_str()` 직접
    주입, 무관분은 `real_path_var()` 헬퍼(env 읽기만·조작 없음). confirms_healthy_restart 는 PATH 오염이
    사라지면 자연 해소 — 코드 무변경.

## 검증
- 저장소 전체 `set_var("PATH")`/`remove_var("PATH")` 실질 0건(주석 1건 제외). 병렬 `cargo test --workspace`
  메인 직접 6회 반복 전건 통과(에이전트 10회 통과 보고 별개로 재확증). verify·clippy 그린.
- flaky 독립 결함이라 T1 배치와 별도 커밋.
