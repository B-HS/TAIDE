# PROCESS — TAIDE 작업 상태

> 기준 문서: `~/.claude/convention/*.md`(전 컨벤션), `docs/acknowledge/`(결정), 이 문서(체크리스트).
> 세션 범위 합의: **이번 세션은 문서화만 완결** (구현은 다음 세션부터 — `docs/roadmap.md` 순서).

## 작업: 문서화 (2026-08-06)

- [x] a. git init + docs/ 구조 생성
- [x] b. 초기 결정 기록 — `docs/acknowledge/2026-08-06-initial-decisions.md`
- [x] c. 기술 리서치 (Workflow, opus·medium) — `docs/research/` 10건 완료
- [x] d. PRD — `docs/PRD.md`
- [x] e. 아키텍처 — `docs/architecture.md`
- [x] f. 기술 스택 확정 — `docs/tech-stack.md` (버전 정본)
- [x] g. ADR — 0001~0011 전건 승인 (0007·0011 은 사용자 확정 반영)
- [x] h. IPC 계약 — `docs/ipc-contract.md`
- [x] i. 데이터 모델·영속화 — `docs/data-model.md`
- [x] j. 테마 시스템 — `docs/theme-system.md`
- [x] k. 기능 상세 스펙 — `docs/features/` 9건 (layout-shell/tabs/explorer-sidebar/editor/lsp/terminal/agent-integration/git/plugins)
- [x] l. 로드맵 + 전체 구현 체크리스트 — `docs/roadmap.md`
- [x] m. 정합성 검수 — `docs/quality-assurance/docs-review-checklist.md` (불일치 5건 해소, 대기 5건 기록)
- [x] n. 사용자 결정 5건 확정 → 관련 문서 갱신 완료 (`quality-assurance` §3)
- [x] o. 최종 보고 (커밋은 사용자 요청 시 — 수동 커밋 합의)

## 다음 세션 시작 시

1. `docs/roadmap.md` Phase 0 부터. 이 파일에 Phase 체크리스트를 복사·세분화해 진행.
2. 버전은 `docs/tech-stack.md`, API 사용법은 `docs/research/*.md` 를 정본으로 사용
   (research 의 "미확인" 항목은 `quality-assurance` §4 에서 실측 후 진행).
