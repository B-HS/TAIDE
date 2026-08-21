# T2-B 2호 — command-registry 관심사 4분할 계약 (2026-08-21, d-30)

> 정본: 감사 `2026-08-18-architecture-audit.md` §5 T2-B 행 + :209 실측(301줄·동거 4개 —
> 상수·팔레트 파서·레지스트리·카탈로그). goal 하 T2-B 파일별 단독 배치 순번.
> 착수 전 메인 실사: 현 302줄(감사와 등치 — 기처리 아님)·export 19·소비처 다수(팔레트·키맵
> 계열). 형제 editor-pane-command-bridge·monaco-action-commands 는 이미 분리되어 있음.

## 1. 범위·원칙

- shared/lib 내 4파일 분리(관심사별·kebab-case — 정확한 경계·명명은 실사로 확정: 상수 /
  팔레트 모드 파서 / 커맨드 레지스트리(등록·조회) / 카탈로그(빌트인 커맨드 정의)). 기존
  파일명 유지 여부(레지스트리 본체가 command-registry.ts 잔류)는 소비처 diff 최소 기준.
- **동작·API 무변경 절대**: export 시그니처 유지(소비처는 import 경로만 변경). 타입은
  원본 유도 유지. barrel 금지 — 소비처가 분리 파일을 직접 import.
- 테스트(command-registry.test.ts)는 대상 분리에 맞춰 동행 분할 또는 유지(실사 판단).
- 범위 외: 커맨드 추가/제거·팔레트 동작 변경·T2-B 다른 파일.

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독, TS 전용) → 메인 2차 → Phase E(소형이므로 2렌즈:
  동등성 — export·소비처 전수 / 계약 — 컨벤션·경계) + major 적대적 → 수정 → 커밋 → 병합.
- 검증: bun run verify + bunx vite build exit 0 + bindings 무변경.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

(작성 예정)
