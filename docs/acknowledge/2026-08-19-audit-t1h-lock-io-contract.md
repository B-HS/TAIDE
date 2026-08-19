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

---

## 4. 구현 완료 기록 (2026-08-19, Phase E 검토 전)

> 구현 wf_4d766dcb-e89(Rust 단독 sonnet+xhigh) 완료. 메인 2차: 스팟 전건 실물 일치(git_push/
> fetch 무가드 spawn_blocking·git_pull begin_mutation_blocking 유지·tree_rows 되쓰기 제거·
> update_index 제거 doc·font OnceLock·plugin stage/commit 분리·sync 3단계 doc) + verify·vite
> build 메인 직접 재실행.

- **R4#3 git 네트워크 — 실사 정정**: service::push/pull/fetch 는 git2 가 아니라 **git CLI
  서브프로세스**(`run_git`). push·fetch 는 가드 제거+spawn_blocking(로컬 영향은 .git refs 만
  — 워킹트리 무접촉, .git 무결성은 git 자체 락이 보호·터미널 git 병행과 동일 실사 근거 doc).
  fetch 의 유일 인터리브(동시 pull 의 FETCH_HEAD 재기록)는 동등 계산 근거로 수용 기록.
  **git_pull 부분 보류**: CLI `git pull` 이 fetch+config 의존 통합(merge/rebase)을 한
  프로세스에 융합 — 분리 시 rebase 설정 repo 의미 변화. 계약 §1.1 분기대로 전체 락 유지,
  spawn_blocking+begin_mutation_blocking 이관(§2.1 위반 중 async 워커 블로킹 반쪽만 해소 —
  네트워크 단계 락 탈출은 보류. **§5 정정**: 이 shape 는 blocking 풀 교착 위험이 검토에서
  확인돼 async 가드+spawn_blocking 동형으로 재수정됨). ~~commit·stage 계열은 실사 후 무수정
  (§1.4 — 과잉 수정 금지)~~ **§5 정정(검토 T1H-C-02 confirmed)**: 이 문장은 §1.4 오인용 —
  R4#3 원문이 commit 을 포함하며 git_commit(`add -A`+commit 서브프로세스)은 장기 IO 실재.
  §5 에서 git_commit·git_init·git_undo_last_commit 3건 이관으로 해소. §4 의 "신규 11" 도
  과대 기술(당시 실측 신규 6·이행 5)로 정정.
- **R5#7 sync**: upload = ①짧은 가드 스냅샷·payload 조립 → ②가드 밖 gist 왕복 → ③재획득 후
  **현재 settings 재독** sync 필드만 overlay(라운드트립 중 landed 된 settings_update 를
  롤백하지 않는 정합 규약 doc — sync 북키핑은 last-write·나머지는 라이브 소유). download =
  fetch·parse·schema 가드 밖 / check-then-apply 가드 안 **라이브 기준 재평가**(fetch 중
  disconnect·재지정 시 stale 적용 중단 — 기존 에러 변종·와이어 무변경).
- **R7#10 plugin/vsix**: 가드 보유 중 128MB 해제 실측 확인 → stage(가드 밖 spawn_blocking,
  advisory 중복 검사)/commit(가드 안 — **권위적** 중복 검사+atomic rename+실패 시 스테이징
  정리) 분리. 기존 락의 실보호("중복 id 결정적 실패"·uninstall/reload 직렬화)는 commit+
  reload 가 가드 안 잔류로 완전 보존. vsix 는 `stage_vsix_import` 분리 후 plugin 의
  `commit_staged_install` 재사용(경계 화이트리스트 기존 엣지 재사용·신규 엣지 0).
- **R8#11 font**: OnceLock 프로세스 수명 캐시·첫 스캔만 spawn_blocking·재시작 반영 doc.
- **R4#1 tree_rows — 되쓰기 제거(근본)**: 히트는 read 락 무쓰기 서빙, 미스는 락 밖 빌드 후
  `entry().or_insert` 국소 삽입(경합 시 기존 엔트리 승리). lost update·매 호출 전체 딥카피
  동시 제거. begin_mutation 추가 방식 미채택. 뮤테이션 커맨드의 clone→되쓰기는 가드 하
  직렬화라 범위 밖 무수정.
- **R4#11 git_status**: `update_index(true)` 제거 — git2 0.21 doc 실사로 순수 성능 최적화
  (반환 결과 불변) 확증. 영향(stat-stale 재해시 비용 유계·타 경로가 어차피 갱신) doc 기록.
- **R8#13 실사**: 정본 복원 불가(감사 압축). 독립 실사로 후보 1건 식별 — **pty_spawn 이
  가드 보유 중 스크립트 3파일 기록+openpty+fork/exec+스레드 3생성**. 보류(근거 3: ms 급
  점유로 등급 상이·분리 시 신규 회수 경로가 pty 회수 부재(R8#1)·T1-J 와 충돌 여지·R8#13
  확증 없음 — 추측 수정 금지).
- **검증**: cargo fmt·clippy 0·test 1041+3+6+17 전량 그린(신규 11·이행 5·삭제 0). bindings
  102줄 diff 전수 검사 — 비주석 변경 0(doc 전파만)·커맨드·시그니처·이벤트 무변경. tsc 통과.
- **보류 목록(Phase E 재확인 대상)**: git_pull 네트워크 단계 락 탈출 / pty_spawn 락-IO 결합.

---

## 5. Phase E 검토 결함 수정 반영 (2026-08-19)

> 검토 wf_6f7ce886-f9f(4렌즈 opus+xhigh — 동시성 최우선, 대상 dev `100e6a4` diff) — 발견 29
> (major 8·minor 21, 4렌즈가 sync_upload 재검증 부재에 수렴) → 적대적 검증(opus+high, major
> 전건 8) **confirmed 8·refuted 0**(이 배치도 검토가 실경합을 전량 적중 — 락 해제 구간의
> 인터리빙 결함은 기계 검증이 못 잡는 축임을 재입증) → 수정 wf_947a9c63-7b6(Rust 단독 fixer
> sonnet+xhigh, 12파일). 메인 2차: 스팟 4축 실물 재검증 + verify·vite build 직접 재실행.

- **[confirmed major·4렌즈 수렴] sync_upload 재검증 부재** — 60초 gist 왕복 중 landed 된
  `sync_disconnect` 를 ③되쓰기가 되살림(토큰은 이미 삭제 — 복구 불가 불일치 영속·
  connected:true 거짓 emit). → `overlay_sync_bookkeeping` 신설: 재획득 직후 라이브
  `sync_gist_id` 를 ①스냅샷과 대조, 불일치 시 되쓰기·이벤트 스킵. connected 는 시크릿 실측.
  `load_token` 을 ①가드 안으로 복원(원본 순서 — disconnect 선행 시 네트워크 쓰기 전 실패).
- **[confirmed major] sync_download stale 적용·역행** — 재평가가 gist id 만 검사해 동시
  upload 완료 후 stale payload 적용+`sync_last_synced_at` 역행(유령 remote_newer 영구). →
  fetch 직전 `(gist_id, last_synced_at)` 원자 스냅샷 + `decide_download_apply` 가 라이브
  전진 감지 시 기존 재시도 에러 변종으로 중단(와이어 무변경). 충돌 판정도 parse/schema
  **앞**으로 복원(T1H-C3·C-07 — 동일 입력의 반환 종류 원본 동등).
- **[confirmed major] git_pull blocking 풀 교착** — spawn_blocking 안 `begin_mutation_
  blocking` park 가 "가드 보유 중 spawn_blocking.await" 커맨드 6건과 풀 고갈 순환(원격
  dispatch 무상한 spawn 으로 512 동시 호출 도달 가능). → async `begin_mutation().await`
  후 **가드 쥔 채** spawn_blocking(6건과 동형 — 락 대기 중 스레드 점유 소멸로 순환 절단).
  state.rs doc 에 begin_mutation_blocking "유계·짧은 대기 전용" 규약 명문화(잔여 사용처는
  T0#17 search_replace 원형뿐).
- **[confirmed major] git_commit 계열 §1.4 오인용(프로세스 결함)** — 실사 재수행: 가드
  보유+동기 서브프로세스 3건(git_init·git_commit·git_undo_last_commit) 가드 쥔 채
  spawn_blocking 이관. stage/discard/stash 계열은 git2 in-process 실측 — 잔여 §2.1 노출로
  보고(서브프로세스 아님·본 배치 기준 밖).
- **minor 실질 수정**: tree 뮤테이션 3종 전체 되쓰기 → write 락 하 in-place 갱신(반대 방향
  lost update 구조 소멸)+조회 미스의 닫힌 프로젝트 부활 방지(projects 재확인·락 순서 검증)+
  doc 과장 정정+역방향 단언 / git_status spawn_blocking 이관 / 최초 gist 생성 경로만 가드
  왕복 유지(F5 — orphan gist 시크릿 노출 구조 배제, 계정당 1회라 비용 수용) / stage 3함수
  인자 순서 통일+.tmp 청소 책임 doc(F4) / sync 규약을 순수 함수 추출+테스트로 고정(F6 —
  doc-only 탈피) / 폰트 테스트 공개 진입점 경유·git_status 테스트 전제 단언(공허 통과 제거)
  / doc 정정 5곳(pre-push 훅·fetch 락파일 경합·낡은 git_push 서술 2곳·과장 축소).
- **이월(보고)**: C11-GIT-3 repo 단위 push/fetch 재진입 직렬화·원격 dispatch 동시 상한
  (교착 축은 GIT-1 수정으로 해소 — 상한 설계 후속) / 잔여 §2.1: 가드 보유 git2 in-process
  동기 IO 13건(서브프로세스 아님 — 후속 배치 후보) / pty_spawn(§4 보류 유지).
- **검증(메인 직접)**: cargo test 1050+3+6+17(신규 9)·bun test 1375·verify·vite build
  exit 0. bindings 비주석 토큰 1071==1071 완전 동일(표면 무변경).
