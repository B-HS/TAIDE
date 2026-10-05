# Native LSP owner 종료 뒤 활성 요청 수 잔존

## 증상·원인

`crates/taide-lsp/src/native/session.rs::SessionClient::snapshot`은 watch sender 종료를 Degraded/TransportClosed로 변환하지만 마지막 publication의 활성 수는 그대로 반환했습니다. TaskSupervisor·ExitDrain으로 Running actor와 child를 회수한 뒤 실제 pending 객체가 해제돼도 snapshot에 pending 1과 progress token 2가 남았습니다. 객체 누수의 증거가 아니라 폐기된 상태를 활성처럼 표시하는 결함입니다.

## 재현·수정

`experiments/lsp-coordinator-spike/tests/runtime-host.rs`가 실제 합성 child의 held definition 요청을 기존 runtime root direct Exit로 종료합니다. 회수·waiter 종료·추적 0 뒤 pending 0 assertion은 관찰값 1로 실패했습니다(exit 101, 0.17초).

닫힌 owner의 snapshot을 Degraded로 변환할 때 pending·server_pending·progress_tokens·registrations를 0으로 정정했습니다. 마지막 PID와 generation의 진단 정보는 유지하며 이 snapshot만으로 OS worker join을 주장하지 않습니다. raw watch subscriber는 채널 종료를 직접 처리하는 기존 경계를 유지합니다. graceful 종료·root 종료 정책·기존 Tauri caller는 바꾸지 않았습니다.

## 검증

- 수정 뒤 runtime host 실제 두 child의 명시적 stop·강제 root 회수 검사 1건: 0.33초, 통과
- closed_owner_snapshot unit 1건: 네 활성 수 해제·generation 보존·Degraded/TransportClosed, 0.00초, 통과
- runtime-host strict clippy와 제품 LSP lib/tests strict clippy: 각각 0.55초·0.38초, exit 0

명령·해석 실패·초기 불완전 assertion과 실제 실패/수정의 순서는 `docs/quality-assurance/2026-09-30-m8-lsp-coordinator-spike.md`의 Running runtime host 항목에 기록합니다. native UI·실제 언어 서버·M8 전체 완료 증거는 아닙니다.
