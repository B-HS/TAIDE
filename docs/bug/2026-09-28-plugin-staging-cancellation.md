# 설치 요청 취소 뒤 staging 잔류

## 대상 파일

- `crates/taide-runtime/src/plugin_actions.rs`, `src/vsix_actions.rs`, `src/plugin_install_worker.rs`
- `src-tauri/tests/plugin_actions_runtime.rs`

## 리포트

기존 plugin 설치 요청을 취소하면 stage가 반환한 임시 경로를 정리할 소유자가 없어 bytes가 남습니다. 두 설치 경로의 raw PathBuf 반환/guard 대기 원인을 공통 감독과 RAII로 수정했습니다.

## 재현과 해결

1. 자기 UUID의 정상 plugin source를 준비하고 mutation guard를 점유한 채 설치를 시작했습니다. 자기 `.tmp`에 stage 생성이 관찰된 뒤 request를 abort하자 staging count는 기대 0 대신 1이었고 검사 exit 101이었습니다. 사용자 파일은 사용하지 않았습니다.
2. 전체 async 설치와 nested blocking stage를 같은 TaskSupervisor에 등록합니다. 요청 Drop은 async 작업 abort를 요청하고 반환 artifact는 Drop 시 자기 임시 경로를 정리합니다. stage가 아직 실행 중이면 정상 root는 실제 stage와 결과 전송/cleanup 시도 완료까지 기다립니다.
3. 같은 실제 회귀가 수정 뒤 통과했으며 기존 action 9건과 신규 owner 4건·Tauri 포트/실제 binding·IPC도 통과했습니다. 상세 명령과 결과는 `docs/history/2026-09-28-plugin-install-owner.md`에 있습니다.
4. source/archive·정상 설치본을 제거하지 않으며 callback/서비스 오류와 stage→guard→commit/cache 순서는 유지합니다. 이미 시작한 stage의 강제 중단·OS 삭제 실패·서비스가 임시 경로 반환 전에 panic한 경우의 완전 정리는 보장하지 않습니다. 전체 M6/M7 보안/자원 회수 합격으로 해석하지 않습니다.
