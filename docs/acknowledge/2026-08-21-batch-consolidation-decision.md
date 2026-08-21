# 배치 통합(뭉탱이) 운영 결정 (2026-08-21, d-31 이후 적용)

> 사용자 지시(2026-08-21): "아직도 15스텝이 더 남은거야? 좀 묶어서 뭉탱이로 한방에 할 수
> 없는거야?" — 잔여 큐를 5묶음으로 압축해 진행하기로 확정.

## 1. 원칙

- **같은 성격·같은 위험 축이면 한 계약·한 구현 Workflow·한 검토 회전으로 묶는다**(선례:
  T2-D/F/G 통합·d-24 2결정 통합 — 둘 다 검토 커버리지 유지 실증).
- 줄이는 것은 **오버헤드**(계약·커밋·병합 왕복)뿐이다 — 검토·적대적 검증 자체는 축소하지
  않는다("효율보다 완벽" 유지): 렌즈를 배치 내 영역별로 배분하고 적대적은 major 건별 유지.
- Rust 는 여전히 한 시점 한 에이전트(묶음 내 순차 다단). 커밋은 논리 단위로 분리 가능
  (묶음 1개 ≠ 커밋 1개 강제 아님).

## 2. 확정 큐 (5묶음)

| 번호 | 묶음 | 내용 |
|------|------|------|
| d-31 | T2-B TS 일괄 | mapping-tables·lsp-session-registry·git-panel·explorer-container·command-palette(실사 후 유효분 — d-26 대수술로 얇아졌을 수 있음) + d-26 이월(github 계열 알파 matchHighlight 토큰·search-match-row 저대비 — mapping-tables/contrast 범위 겹침) |
| d-32 | Rust 구조 일괄 | lib.rs 도메인 이관 + layout 커맨드 18중복 골격 공통화(d-28 이월 — T1-H 락 축·위험 중상: 동시성 렌즈 최우선) |
| d-33 | 재구조화+이월 소형 일괄 | T2-C shared/lib 디렉토리 재편(107파일 평면) + FILE.RAW 무효화 선재 결함(d-25 발견)·Switch 행 프리미티브(d-29 이월)·workspaceSymbol 하이라이트(d-26 이월)·T1 3차 이월 소형분(ws.rs writer·TREE_ROWS 센티널 등 실사 유효분) |
| d-34 | AppError 캠페인 | T2-E/J 369지점 taxonomy — 워크플로 1회 다단(설계→Rust 적용→TS 소비) 순차·검토 1회전 |
| d-35 | Rust 하드닝 이월 일괄 | T1-H 이월(git2 in-process 동기 IO 13건·repo 단위 재진입 직렬화·원격 dispatch 동시 상한·pty_spawn)·NoCache 검토 |

## 3. 자율 진행 종점 이후 (사용자 개입 필요 — 배치 아님)

1. 제품 결정 3건: ide_publish_diagnostics/notify_at_mention 원격 게이트·agent_hooks
   install/uninstall 대칭화·키링 게이팅 비대칭(R3#7).
2. AI 응답 동형 타입 3종 통합 — **bindings 표면 변경 승인** 필요(d-28 §3.1 #17).
3. codex 절단 보정 — 항상-에러 → 관용 분기는 **행동 변경**(제품 판단, d-28 #18) ·
   ollama↔omlx 파이프라인 트레이트 재설계는 이와 묶어 판단.
4. e2e 파일럿·QA-W1 실기 — 사용자 준비 절차(HANDOFF §7).
5. Phase 8 배포 — 전문 QA(d) 통과 후.
