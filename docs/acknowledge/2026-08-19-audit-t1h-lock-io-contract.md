# 감사 T1-H — 전역 락 IO 분리(C11) 계약 (2026-08-19)

> 정본: 감사 `2026-08-18-architecture-audit.md` §4.1-C11(양축 분해, 566-585행)·§6.2 T1-H(위험
> **높음** — "96개 호출 지점이 단일 락을 공유하는 상태에서 락을 쪼개면 우연히 성립하던 순서
> 보장이 깨질 수 있다", 단독 진행 권고)·§9 결정 4(별도 캠페인 추천 강화 — 사용자 기결: T1 잔여
> 묶음으로 유지, HANDOFF §8-3).
> 사용자 상시 지시("전부 추천대로 계속 진행해봐") 하에 착수 — **위험 최고 재고지 완료**(T1-I
> 완결 보고에 명시: begin_mutation 입도 변경은 96(현 실측 105) 호출지점의 암묵 순서 보장이
> 걸림). 직전 산출물 T1-I 는 main 병합 완료(main=9bad3df).
> 착수 전 메인 실물 재확인: begin_mutation = `state.rs:61` tokio Mutex<()> 전역 단일 +
> `begin_mutation_blocking`(:68, T0#17 신설) — 호출 105지점. git_push/pull/fetch(git/commands.
> rs:267-295) 가 가드 보유 중 `service::push/pull/fetch` **동기 네트워크 호출**(spawn_blocking
> 미사용 — §2.1 이중 위반) / sync 커맨드 3곳 가드 보유 중 GitHub 왕복 / tree_rows(tree/
> commands.rs:63-73) **무가드 `*tree_store.0.write() = trees` 되쓰기 잔존**(감사 671행 "T0
> 처리" 권고가 T0 계약에 미반영됐음을 실측) / git_status → service.rs:1555 `update_index(true)`
> 잔존 / font_list(font/commands.rs:7) 는 **락 무관**(state 미수령) — 본질은 캐시 부재 /
> plugin·vsix 압축 해제 락 보유 여부는 구현 실사.
> **기처리 확인(범위 제외)**: R7#3 search_replace 가드+spawn_blocking(T0#17)·R8#2 pty_write
> 락 축소(T0#20)·R6#2 ide_resolve_diff 가드(T1-I R2).

## 1. 범위 (C11 10건 중 기처리 3건 제외 실작업 6건 + R8#13 실사)

### 1.0 접근 원칙 — 위험 관리의 핵심

- **`begin_mutation` 의 전역 단일 입도는 이번 배치에서 바꾸지 않는다**(락 쪼개기·도메인별
  락 분리 금지). 감사 T1-H 문언의 "락 단위 축 분리 검토"는 검토 결과 **기각·후속 이월** —
  105 호출지점의 암묵 순서 보장을 건드리는 재설계는 이 배치의 국소 수정(락-IO 결합 해소)과
  분리해야 위험이 통제된다.
- 축 A 의 수정 패턴은 **"락 안 IO 를 락 밖으로"** 국소 이동: 락 안에서는 상태 판독·검증만,
  블로킹/네트워크 단계는 가드 드롭 후 `spawn_blocking`, 상태 반영이 필요하면 완료 후 재획득.
  T0#17 search_replace(대상 확정 후 파일 단위 재획득)가 선례.
- **동시성 의미론 보존이 절대 조건**: 각 지점에서 기존 락이 실제로 보호하던 불변식을 실사해
  기록하고, 그 보호가 실재하면 해당 단계만 락 안에 남긴다. 예: git pull 의 **네트워크 단계
  (fetch)는 락 밖**, **워킹트리 반영 단계는 락 안**(파일 뮤테이션과의 직렬화가 실보호) 분리.
  분리점이 불명하거나 반쪽 수정이 되는 지점은 **수정하지 말고 보고 후 보류**(반쪽 기능 금지).

### 1.1 축 A — 락 보유 중 블로킹/네트워크 IO (4건)

- **R4#3 git 네트워크 4종+**(push/pull/fetch/commit — commit 은 네트워크 아님·장기 IO 여부
  실사, git_commit_files 등 유사 패턴 전수 실사): 네트워크 왕복을 가드 밖 `spawn_blocking`
  으로. pull 은 fetch 단계/워킹트리 merge 단계 분리(git2 API 가 분리를 허용하는지 실사 —
  불허 시 전체를 락 안에 유지하고 보고). push·fetch 는 로컬 상태 영향(refs 갱신) 실사 후
  분리. 같은 repo 동시 git 조작 배제가 필요하면 그 보호의 실재를 근거로 판단.
- **R5#7 sync upload/download**: GitHub 왕복(타임아웃 60초)을 가드 밖으로 — 다운로드 결과의
  로컬 반영(설정 적용)만 락 안. 락 해제 중 설정이 바뀐 경우의 정합(재검증 또는 last-write 규약
  명시) 실사.
- **R7#10 plugin/vsix 압축 해제**: 최대 128MB 해제+재귀 복사의 락 보유 여부 실사 후 락 밖
  `spawn_blocking` 으로(이미 밖이면 spawn_blocking 여부만 정합).
- **R8#11 font_list**: 락 무관(실측) — 본질은 매 호출 시스템 폰트 전수 스캔. 프로세스 수명
  캐시(1회 스캔) 도입. 폰트 설치 변경 반영은 현 UX 요구에 없음(재시작 반영) — doc 명시.

### 1.2 축 B — 써야 할 곳에 가드가 없음 (2건)

- **R4#1 tree_rows**: 무가드 `read().clone()` → `rows_page` → `write() = trees` 전체 되쓰기
  (lost update·critical→major 하향 기록). 수정: 조회 경로가 스토어 전체를 되쓰지 않게 —
  `ensure_entry` 계열로 해당 엔트리만 갱신하거나, 페이지 결과가 캐시를 바꿀 필요가 없으면
  되쓰기 자체를 제거(성능 축: 매 호출 전체 딥카피 제거 동반). begin_mutation 추가가 아니라
  **되쓰기 구조 제거가 근본**(조회는 조회로).
- **R4#11 git_status 의 `update_index(true)`**: query 가 인덱스를 기록하는 부수효과 제거 —
  git2 StatusOptions 의 해당 플래그 제거 후 상태 판독 정확도 회귀 실사(플래그가 stat 캐시
  갱신용이면 제거 시 성능·정확도 영향 확인, 필요 시 명시적 별도 경로로).

### 1.3 R8#13 — 감사 압축으로 상세 소실

- 티어 표(감사 735행)에만 존재. 구현 에이전트가 R8 소유 영역(terminal·system·shell_
  integration)의 락-IO 결합·가드 부재를 실사해 해당 건을 식별 시도, **식별 불가 시 수정하지
  않고 보고**(추측 수정 금지).

### 1.4 범위 외

- begin_mutation 입도 재설계(§1.0 — 후속 이월 기록). C11 1차 나머지(R2#3 pty kill 누수·
  R2#14·R3#6·R1#10·R1#12 133 직접 락 접근)는 T1-H 티어 배정 밖 — 무수정.
- 커맨드 표면·와이어·원격 정책 무변경(커맨드 176+raw3·이벤트 23·ALLOWED 160⊎DENIED 19).
- T1-I 이월분(AppState 사각지대·순환 2쌍)·T2.

## 2. 실행 구조

- **Rust 단독 1 에이전트(sonnet+xhigh)**: §1.1~§1.3 전량 + 지점별 "기존 락이 보호하던 것"
  실사 기록. 각 수정에 동시성 회귀 테스트(가능 범위 — 락 순서·재획득 정합) + 기존 테스트
  전량 그린. cargo fmt/clippy/test + bindings 무변경 확인. git stash 금지.
- **Phase E 검토(별도 Workflow)**: 4렌즈(opus+xhigh — **동시성**: 락 해제 구간에 끼어드는
  뮤테이션과의 경합 전수(pull 중 file_save·sync 반영 중 settings_update 등)·재획득 후 상태
  가정 stale 여부 / 정확성: git2 분리점 적정·tree_rows 되쓰기 제거의 캐시 정합·update_index
  제거의 상태 판독 영향 / 계약: 표면 무변경·보류 판단의 근거 실증 / 설계: spawn_blocking
  경계·캐시 수명) → 적대적(opus+high, major 이상 건별) → confirmed 수정 → 메인 2차 →
  커밋(dev).

## 3. 완료 조건

- `bun run verify` + vite build 그린. bindings 무변경. 표면 파리티 전부 그린.
- 각 수정 지점의 "락이 보호하던 것 → 보존 방식" 기록 전수(계약 §4 에). 보류 지점은 사유와
  함께 명시(무단 반쪽 수정 0).
- 실기 이월(qa6): 대형 repo push/pull 중 앱 반응성(프리즈 해소 체감)·pull 중 저장 경합·
  sync 중 설정 변경·폰트 목록 즉시 표시.
