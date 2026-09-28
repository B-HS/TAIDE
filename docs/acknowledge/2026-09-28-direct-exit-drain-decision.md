# 직접 앱 종료의 자원 대기 선택

## 대상 파일

- `src-tauri/src/lib.rs`의 `RunEvent::Exit` 처리
- `crates/taide-runtime/src/exit_drain.rs`의 `ExitDrain`

## 리포트

사용자는 직접 `RunEvent::Exit`도 제한 시간 없이 전체 등록 자원 완료까지 기다리는 A안을 선택했습니다. 기존 `ExitRequested`와 같은 TaskSupervisor 작업·operation, LSP 설치 lease, LSP process worker, AI request owner, PTY worker를 대상으로 합니다. 새 시간 제한이나 강제 종료를 추가하지 않습니다.

## 상세

- `ExitRequested`는 `prevent_exit`로 이벤트 루프를 유지하며 비동기 대기합니다. 직접 `Exit`는 이미 끝나는 이벤트 루프의 callback 안에서 동기 대기합니다.
- 직접 경로도 신규 입장을 닫고 등록 자원 완료를 기다립니다. 이는 OS I/O나 메인 스레드 응답이 멈춘 자원의 무기한 대기 위험을 수용한 결정이며, 강제 OS 종료·등록되지 않은 자원·그룹 밖 자손의 회수 보장은 아닙니다.
- 실제 native 종료 이벤트 순서·GUI/main-thread callback의 교착 여부는 합성 fixture로 증명할 수 없어 M6/M7 실기 gate에 남깁니다.
