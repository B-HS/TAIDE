# M7 전체 crate 분리 검증 게이트 완료

## 대상 파일과 범위

`docs/PROCESS.md`의 M7-A~C, `docs/quality-assurance/2026-09-23-rust-native-parity-plan.md`의 Phase 0 기준선, `docs/quality-assurance/2026-09-04-perf-baseline.md`의 현행 release 성능 표를 판정했습니다. M6의 여러 자원 동시 직접 `Exit` adapter gate와 M8의 native 동등성은 이 완료 판정에 포함하지 않습니다.

## 리포트

기존 [자동 게이트](../quality-assurance/2026-09-28-m7-automated-gate.md)의 `bun test` 2,949건, frontend typecheck·build, 권한 허용 Rust workspace 전체 테스트, all-target Clippy·fmt 성공을 공통 경계의 근거로 재사용했습니다. 후속 원격 큐·TTL 및 성능 계측 변경은 [대상 검증](../quality-assurance/2026-09-29-m7-perf-readout-live.md)과 각 QA 기록의 테스트·typecheck·Clippy·fmt·release 빌드로 덮었습니다. [IPC·원격·IDE/CLI·저장 fixture와 대표 앱 실기](../quality-assurance/2026-09-23-rust-native-parity-plan.md)를 M7-C에 연결했습니다. 성공 명령은 입력·환경이 같은 상태에서 문서 변경만을 이유로 다시 실행하지 않았습니다.

현행 앱 release 성능 11개 세부 행은 지표별 단일 표본으로 채웠습니다. 터미널은 [실제 전면 200만 줄 출력](../quality-assurance/2026-09-29-m7-terminal-foreground-render.md)에서 Rust 20,777,785바이트 수신, 파서 완료, 89회 렌더와 마지막 행 표시를 확인했습니다. 부팅은 사용자가 [표시 준비 대리지표](../acknowledge/2026-09-29-m7-boot-visible-ready-proxy.md)를 승인해 프로세스 시작→콘텐츠 준비 약 1119ms와 창 표시 완료 약 1120ms를 기록했습니다. 실제 첫 픽셀 시각·독립 픽셀 렌더 MB/s·분포 통계는 주장하지 않습니다.

GUI·직접 종료는 [대표 화면 실기](../quality-assurance/2026-09-29-m7-ts-view-representative-gate.md)와 [원격 WebSocket·PTY 동시 직접 Exit](../quality-assurance/2026-09-29-m7-remote-direct-exit-verified.md)의 기존 성공 근거를 재사용했습니다. 사용자 프로젝트·키는 사용하지 않았고 합성 fixture와 격리 앱만 사용했습니다. M6-PJ-2의 LSP·watcher callback까지 겹친 전체 자원 동시 종료와 M8의 212개 view native 동등성은 여전히 미완료입니다.

## 상세 검증

이번 문서 변경은 Markdown Prettier와 `git diff --check`로 확인합니다. 제품 소스·IPC 계약은 바꾸지 않았으므로 자동 테스트·빌드를 재실행하지 않습니다. 일반 commit·push 결과는 `docs/PROCESS.md`의 현재 M7 상태와 Git 이력으로 확인합니다.
