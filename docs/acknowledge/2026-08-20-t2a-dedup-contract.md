# T2-A 중복 제거 배치 계약 (2026-08-20, d-28)

> 정본: 감사 `2026-08-18-architecture-audit.md` §5 T2-A 행(C2 잔여 — fileNameOf 6+3·확장자
> 추출 4·NOOP_DISPOSABLE 16·constant_time_eq 3·CodeEditor 프롭 배선 3·LSP 심볼 로딩 2·
> Location 변환 2·ANSI 토큰 3·기본값 상수 6·layout 커맨드 18) + §6.3 증분(constant_time_eq
> 4번째 사본 R6#9·ollama↔omlx provider 전체 R6#10·경로 봉쇄 원시함수 R7#5·provider match
> 3벌 R6#13·훅 JSON API 2벌 R6#15·게이트 arm 복제 R6#17·AI 응답 동형 타입 3종 R6#18·codex
> 절단 보정 R6#6).
> 사용자 goal("추천안대로 배치 계속진행") 하 HANDOFF §8-2 순번 배치. **X-A 선례 형식 —
> 항목별 실사 후 유효분만 수정·판정 근거 전수 기록**(감사는 2026-08-18 기준이라 이후 12+
> 배치로 지형 변화 — 기처리 가능성 높은 예: fileNameOf 일부(d-27 검토가 welcome 재구현 제거)·
> TS 경로 봉쇄(d-27 이 isWithinRoot 를 shared/lib/path-root 로 승격 — R7#5 는 Rust 측이라
> 별개 실사)·기본값 상수 일부(T1-B/T2-G). 전 항목 재실사가 출발점).

## 1. 범위·원칙

- 감사 열거 18항목(10+8) 전수 실사 → 판정(fixed/already-handled/invalid-claim/
  unidentifiable/deferred) → 유효분만 수정. **공통화는 2회 이상 실사용이 확인된 것만**
  (common.md 룰 — 감사 주장 수치도 재검증). 승격 위치는 FSD 규칙(shared/lib·entities)·
  기존 선례 우선.
- **대형 재설계급은 deferred**: ollama↔omlx provider 전체 파이프라인(R6#10)·layout 커맨드
  18중복·CodeEditor 프롭 배선 3 등은 실사 후 국소 공통화가 아니면 분해 배치(T2-B)와의 경계
  판단·이월 기록(반쪽 수정 금지).
- 동작 무변경 절대 조건. Rust 항목(constant_time_eq·경로 봉쇄·provider match·훈 JSON API·
  게이트 arm·AI 응답 타입·codex 보정) 포함 — 한 에이전트 순차. 표면(커맨드·이벤트·bindings·
  원격 정책) 무변경.
- 범위 외: T2-B 분해·T2-C 재구조화·T2-E/J 캠페인·FILE.RAW 무효화 선재 결함(d-25 발견 —
  별도 배치).

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독): 실사 → 수정 → 신규/갱신 테스트 → cargo fmt →
  `bun run verify` + `bunx vite build` exit 0 + bindings 무변경.
- Phase E 4렌즈(실사충실성 — 판정 독립 재검증·2회 룰 / 정확성 — 공통화 전후 동작 등가·미묘
  분기 유실 / 설계 — 승격 위치·과추상화 / 계약 — 표면·경계·컨벤션) → 적대적(major 이상) →
  confirmed 수정 → 메인 2차 → 커밋 → prod 병합.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

(작성 예정)
