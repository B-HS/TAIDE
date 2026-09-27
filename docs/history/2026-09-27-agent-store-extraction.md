# AgentStore 공유 상태 분리

상태: M6의 이 변경 단위는 자동 검증 완료. M6 전체와 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-agent/src/store.rs`가 활동 diff·세션 신호·PID 이름 캐시·wait marker·외부 열기 대기열을 소유합니다. 내부 Arc<Mutex>를 공유하는 clone을 `crates/taide-runtime/src/app_services.rs`가 생성하며 `src-tauri/src/lib.rs`는 같은 인스턴스를 기존 Tauri State에 등록합니다. AppServices는 총 15개 상태·포트를 조립합니다.

`src-tauri/src/domain/agent/commands.rs`는 기존 AgentStore 경로를 재수출합니다. OS 프로세스 조회·PTY 신호 전달·AgentForegroundPids 주입과 marker 파일 정리는 Tauri adapter에 유지했습니다. 기존 상태 정책은 유지하고, 에이전트 교체 시 이전 세션 신호를 버리는 테스트를 agent crate로 함께 이전했습니다. crate 간 호출에 필요한 scan/input·PID 캐시 메서드만 공개했습니다. Cargo 의존성은 변경하지 않았습니다.

## 검증

- AppServices 공유 테스트는 `agents` 필드 부재 E0609 2건(exit 101)으로 먼저 실패했습니다.
- `cargo test -p taide-agent --lib store:: --quiet`: 만료·세션 교체 회귀 2건 통과.
- `cargo test -p taide --test app_services_runtime --test task_supervisor --test rust_native_phase0_contract --quiet`: 공유/배선 2건·감독 22건·IPC 계약 7건 통과.
- `cargo test -p taide --lib domain::agent:: --quiet`와 플러그인 조립 포트 단독 검사: 기존 agent 21건·조립부 1건 통과.
- agent/runtime/Tauri all-target clippy·agent/runtime strict rustdoc·fmt·diff 검사: exit 0. bindings SHA-256은 `174a2d617372f4782b8397351660f503194dc567ece4feb82c2a79199c09977a`로 불변입니다.

초기 통합 컴파일에서 사용량 라벨·종료 처리에 필요한 AgentStore import 누락 E0425 2건을 확인해 복원했고 관련 통합 검사가 통과했습니다. 플러그인 포트 소스 검증의 종료 경계를 새 등록 위치로 갱신했습니다. 이 줄의 첫 fmt 검사 실패는 줄바꿈 정리 후 재검사로 해소했습니다. agent crate로 이전된 순수 테스트 1건만 Tauri의 기존 22건에서 빠져 두 검사 합계는 유지됩니다.

이 변경 뒤 전체 Rust workspace·TypeScript typecheck·실제 agent 활동 배지/외부 CLI 대기·앱 종료 GUI는 미검증입니다. 기존 IPC DTO·command/event 등록·TS UI는 변경하지 않았습니다. 원격 push는 기존 승인 거절로 실행하지 않습니다.
