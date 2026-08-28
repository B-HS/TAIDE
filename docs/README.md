# docs — 문서 지도

> TAIDE 의 모든 작업은 `docs/` 를 베이스로 진행된다. 새 세션의 진입 순서:
> **① `HANDOFF.md`(세션 스냅샷) → ② `PROCESS.md`(작업 체크리스트) → ③ 필요한 정본 문서.**

## 1. 정본 (현행 유지 의무 — 코드와 어긋나면 즉시 갱신)

| 문서 | 내용 |
|------|------|
| `architecture.md` | 시스템 구조 — Rust 소유 상태·레이어·프로세스 경계 |
| `data-model.md` | 도메인 데이터 모델 |
| `ipc-contract.md` | IPC 커맨드 전수(181종)·원격 허용/거부 정책 |
| `theme-system.md` | 테마 파이프라인·번들 카탈로그·게이트 |
| `tech-stack.md` | 스택·버전 정본 |
| `roadmap.md` | 구현 순서 정본 |
| `PRD.md` / `backlog.md` | 요구사항·백로그 |
| `deployment.md` | 릴리스 파이프라인·태그 규칙(숫자 4 금지)·secrets |
| `debugging.md` | 로그·검증 사다리·실증 계측 기법·함정 목록 |
| `agent-operations.md` | AI 세션 역할 5단·계약(d-##) 파이프라인·커밋 규칙 |
| `HANDOFF.md` | 세션 인수인계 스냅샷(단일 진입점) |
| `PROCESS.md` | 작업 상태·체크리스트(약 300줄 초과 시 `history/` 로 아카이브) |

## 2. 시점 기록 (보존 대상 — 소급 수정하지 않는다)

| 디렉토리 | 내용 | 규칙 |
|----------|------|------|
| `acknowledge/` | 사용자 결정·합의·**계약(d-##)** — 구현·검토·판정의 정본 | 계약 §3 은 작업 중 실시간 동기, 종결 후엔 불변 |
| `adr/` | 아키텍처 결정 기록 11건 | 교체 시 새 ADR 로 대체(기존 문서에 supersede 표기) |
| `bug/` | 버그 케이스 기록(증상·원인·해결) | - |
| `feedback/` | 교정 리포트 — 왜 틀렸나·어떻게 고치나·언제 적용하나 | 보편화되면 컨벤션/`memory/` 로 승격 |
| `history/` | PROCESS 아카이브 | - |
| `research/` | 조사 자료(작성 시점 기준) | 낡아도 소급 수정 없이 새 문서로 |
| `quality-assurance/` | QA 체크리스트·감사·e2e 하네스/파일럿 | 체크리스트류는 진행 중 갱신 |
| `memory/` | 장기 재사용 지식 | - |
| `utils/` | 작업 보조 스크립트·소스 출처 기록 | - |
| `features/` | 기능별 스펙·구현 서술 | 준정본 — 해당 기능 변경 시 함께 갱신 |

## 3. 자주 쓰는 진입점

- 결함 처리 절차·역할 분담: `agent-operations.md`
- 릴리스 방법: `deployment.md` (태그 4 금지: `acknowledge/2026-08-28-release-tag-no-digit-4.md`)
- 진단·계측: `debugging.md`
- e2e 실행: `quality-assurance/2026-08-18-e2e-harness.md`
- 실기 QA 마스터: `quality-assurance/2026-08-11-qa6-checklist.md`
- 커밋 규칙 우선순위: `feedback/2026-08-27-commit-despite-no-commit-directive.md`
