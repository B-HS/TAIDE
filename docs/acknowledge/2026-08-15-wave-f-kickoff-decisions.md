# Wave F 착수 결정 (2026-08-15)

> 세션 복원(HANDOFF `d525640` 기준) 후 HANDOFF §6 확인 대기 3건에 대한 사용자 결정.
> 전부 추천안 채택("진행해"). 캠페인 계약: `2026-08-14-remaining-features-pro-qa-plan.md`.

## 1. 결정

| # | 결정 지점 | 선택 |
|---|-----------|------|
| 1 | Wave E(`80c99ac`) prod 병합 | **병합** — A~D 와 동일 기준(verify 그린·4렌즈 검토 완료·실기 미검증 공통). `git branch -f main dev` 후 `git push origin main` 으로 fast-forward 완료(`59f6993` → `d525640`). 실기 미검증 KNOWN ISSUE 는 존속 |
| 2 | 다음 방향 | **Wave F(에디터 표현) 진행** — 캠페인 확정 순서(A→I) 유지. Semantic Tokens·사용자 스니펫·Format on Type/Paste·Emmet. 문서 부채(ipc-contract·data-model·qa6)는 기존 결정대로 전문 QA 착수 전 문서화 워크플로로 일괄 |
| 3 | recent_searches gist 동기화 | **유지** — Wave D 계약 승인대로 sync 대상 유지(비밀 아님·상한 20). 변경 없음 |

## 2. 기각 대안

| 안 | 사유 |
|----|------|
| Wave E prod 보류(실기 확인 후 병합) | A~D 병합 기준과 비일관 — 기각 |
| 문서 부채 먼저 정리 | 부채는 전문 QA 선행 조건이지 Wave F 선행 조건이 아님 — 순서 유지 |
| 전문 QA 조기 전환 | 캠페인 순서(잔여 기능 → 전문 QA) 재론 없음 |
| recent_searches 동기화 제외 | 프라이버시 재확인 여지였으나 사용자가 유지 선택 |

## 3. 복원 검증 부기 (같은 세션)

- HANDOFF §3.1 대표 신규 파일 6건(workspace-edit-applier·code-action·commit-file-diff·
  search-editor-pane·shell_integration·task/service) 실물 대조 — 계약·수정 기록과 전부 일치, 신규 불일치 0.
- 문서 부채(§3.4) 실측 확인 — `data-model.md` 에 recent_searches·SearchEditor·remote_allowed_hosts 0건.
  단 **HANDOFF §3.4 의 ipc-contract 누락 예시 중 `detect_tasks` 는 실제로는 반영되어 있음**
  (ipc-contract.md:107, Wave E §6 문서 갱신분). Wave C git 커맨드 누락은 사실 — 문서화 워크플로 때
  HANDOFF 예시 문언만 정정할 것.
