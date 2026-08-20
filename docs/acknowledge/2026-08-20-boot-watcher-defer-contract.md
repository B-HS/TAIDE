# 부팅 워처 attach 후절화 계약 (2026-08-20, d-25)

> 사용자 결정(d-23): 부팅 ~5초 수정 A안 채택("다 추천안으로"). 진단 정본: 광역
> wf_14010876-849 rank 1(high) — 스크래치패드 `incident-broad-diagnosis.json`.
> 확정 병목: `lib.rs setup()` 이 **창 생성 전에 동기로** 복원 프로젝트 전수의
> `attach_watcher`/`attach_git_watcher` 를 수행 — notify-debouncer-full 의
> RecommendedCache(FileIdMap)가 `debouncer.watch` 시 프로젝트 루트 전체(WalkDir·
> node_modules 포함)를 파일별 stat 워크(RESUME 15,320파일 ~0.5-1초·대형 루트는 수 초,
> 프로젝트 수만큼 곱·git 워처가 .git 에 추가 1회). 이벤트용 ignore 필터는 이 워크에 미적용.
> dev 모드 monaco eager 로드(2~4초)는 별개 축(범위 외 — release 에서 자연 소멸).

## 1. 범위

- **워처 attach 를 부팅 임계 경로 밖으로**: setup() 의 복원 프로젝트 워처 attach(파일·git)를
  창 생성·표시를 막지 않는 비동기 경로(spawn — 표시 후)로 이동. `restore_state`(세션·레이아웃
  복원)와 프로젝트 Store 등재는 동기 유지(트리·에디터가 즉시 뜨는 데 필요) — **워처만 후절**.
- **의미 보존 실사(절대 조건)**:
  - attach 지연 구간(부팅~attach 완료)에 발생한 파일 변경은 이벤트로 안 옴 — 기존 코드가
    attach 후/트리 조회 시 재스캔으로 자연 수렴하는지 실사(tree 는 조회 시 빌드 — 확인).
    수렴 안 하는 소비자(git status 갱신 등)가 있으면 attach 완료 시 1회 refresh 로 보정.
  - `project_open`(신규 열기) 경로의 동기 attach 는 **무변경**(사용자 조작 문맥 — 진단 범위
    밖·T1-I capability 순회 의미 보존).
  - 부팅 attach 실패의 기존 처리(log-warn 삼킴)와 동일 수위 유지. AppState 락 문맥(begin_
    mutation 규약 — T1-H) 준수: 후절 태스크의 Store 접근은 기존 attach 경로와 동일 가드 의미.
  - 다중 프로젝트 순차/병렬은 기존 순서 의미 실사 후 결정(순서 의존 없으면 순차 유지가 안전).
- **계측(경량)**: setup 구간·attach 태스크 완료에 기존 로깅 수위로 elapsed 1줄씩(진단
  discriminator — 재발 검증용. TRACE 홍수 금지·info 1~2줄).
- 범위 외: NoCache 전환(rename 경로 의미 변화 — 후속 검토)·dev 모드 monaco lazy 경계·
  reveal 게이트 재설계·워처 ignore 정책 변경.

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독, Rust 전용): 실사 → 이동 → 회귀 테스트(부팅 attach 가
  결국 수행됨·프로젝트 상태 동일 — 기존 테스트 유지+가능 범위 보강) → cargo fmt → verify +
  vite build exit 0 + bindings 무변경.
- Phase E 4렌즈(동시성: 후절 태스크와 사용자 조작(project_open/close·파일 조작) 경합·락 문맥 /
  정확성: 지연 구간 이벤트 공백 수렴·attach 실패 처리 / 설계: spawn 위치·순서 / 계약: 표면
  무변경·문서) → 적대적(major 이상) → confirmed 수정 → 메인 2차 → 커밋 → 병합.
- 실기 확증(사용자): 부팅 체감(창 표시까지)·표시 직후 파일 트리/git 상태 정상·부팅 직후
  외부 파일 변경 반영.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

(작성 예정)
