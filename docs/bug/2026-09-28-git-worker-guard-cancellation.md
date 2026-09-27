# Git 요청 취소 시 blocking worker보다 먼저 풀리는 guard

## 대상 파일

- `crates/taide-runtime/src/git_actions.rs`
- `crates/taide-runtime/src/task_supervisor.rs`

## 리포트

caller가 borrowed mutation/repo guard를 보유하고 raw blocking JoinHandle을 await하는 기존 구조는 caller abort 뒤 guard를 해제하지만 이미 시작한 blocking 작업은 중단하지 않습니다. 다음 mutation 또는 같은 repo push/fetch가 실제 작업 완료 전에 입장할 수 있었습니다.

## 재현과 해결

1. 기존 global guard 취득→메모리 차단 blocking worker 시작→caller abort의 primitive 패턴을 실행했습니다. worker를 해제하지 않았는데도 두 번째 guard 취득이 성공해 유지 기대 assertion이 실패했습니다(exit 101). 실제 Git 서비스/훅을 실행한 재현은 아닙니다.
2. 41개 action에 감독 worker와 operation을 연결했습니다. async owned guard를 caller/worker가 공유한 단일 owner가 보유하고, 마지막 guard Drop 뒤 lease를 반납합니다.
3. 최종 동일 회귀·repo guard 독립성·worker 완료 뒤 post-await owner·실제 ExitDrain 대기·서비스/panic 오류·종료 입장 거절을 포함한 Git unit 9건이 통과했습니다. 기존 cache/service 인수/event 순서는 보존했습니다.

정상 root 대기의 등록 operation 범위와 취소 뒤 cache/event·동기 discover·OS 실패·직접 Exit·bounded 종료의 한계는 `docs/history/2026-09-28-git-worker-operation-owner.md`에 구분했습니다. 전체 M6 종료 gate 완료로 판정하지 않습니다.
