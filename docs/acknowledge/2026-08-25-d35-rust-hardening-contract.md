# d-35 — Rust 하드닝 이월 일괄 계약 (2026-08-25)

> 정본: `2026-08-21-batch-consolidation-decision.md` §2 d-35 / 이월 원문
> `2026-08-19-audit-t1h-lock-io-contract.md` §3 이월 절("가드 보유 git2 in-process 동기 IO
> 13건·repo 단위 push/fetch 재진입 직렬화·원격 dispatch 동시 상한·pty_spawn §4 보류") /
> d-25 계약 범위 외(NoCache) / d-32 §4(capability deferred-attach — 트레잇 재설계 선행 조건).
> 운영: /goal — 커밋·푸시 없음, 결정은 추천안 자체 채택·기록. **위험 최상 배치 — 동시성
> 렌즈 최우선, 의미 변화 발견 시 해당 항목 보류가 잘못된 수정보다 낫다.**

## 0. 착수 전 메인 실사 (2026-08-25)

- T1-H(2026-08-19)가 축 A(락 안 IO)의 서브프로세스 계열·축 B(가드 부재 4건: search_replace·
  ide_resolve_diff·tree_rows·git_status)를 **완결**함을 실측 확인(search/ide/tree 에
  begin_mutation 가드 실재·per-file 재획득 패턴 존재). d-35 는 그 계약이 명시 이월한 4건+
  NoCache+d-32 이월 1건이 전부다.
- git/commands.rs 실측: begin_mutation 29·spawn_blocking 25 — 서브프로세스 계열은 이관
  완료 상태. 잔여는 **가드 보유 중 git2 라이브러리(in-process) 동기 IO 13건**(stage/discard/
  stash 계열 등 — T1-H §2.1 실측 목록, 구현이 현재 기준 재실사·재확정).

## 1. 범위 (추천안 자체 채택)

- **a. git2 in-process 동기 IO 13건**: 가드 쥔 채 동기 git2 호출을 T1-H 확립 동형 패턴
  ("async `begin_mutation().await` 후 가드 쥔 채 spawn_blocking")으로 이관 — 락 의미(가드
  보유 구간) 불변, 스레드 점유만 blocking 풀로. 13건 목록은 실사 재확정(기처리 발견 시
  제외·기록). 각 건 이관 전후 테스트 무회귀.
- **b. repo 단위 push/fetch 재진입 직렬화**(C11-GIT-3): 동일 repo 에 대한 네트워크 git
  (push/fetch — 무가드 spawn_blocking 상태)의 동시 실행을 repo 경로 키 뮤텍스로 직렬화.
  설계 제약: 전역 락과의 중첩 순서 고정(데드락 신설 금지 — 획득 순서 문서화+검증), 다른
  repo 간 병행성 보존. git_pull 의 전체 락 유지(T1-H 계약 §4 분기)는 불변.
- **c. 원격 dispatch 동시 상한**: 무상한 spawn(512 동시 도달 가능 — T1-H 유도치. **§5 정정**: "실측" 아님)에
  semaphore 상한. 상한값은 근거 있는 상수(blocking 풀 크기·기존 타임아웃과의 관계 —
  JSDoc/doc 근거 필수, 매직넘버 금지). 초과 시 대기(거부 아님 — 행동 변경 금지).
- **d. pty_spawn 락-IO 결합**: T1-H §4 보류 사유 원문 실사 후, a 와 동형으로 풀리면 이관,
  의미 변화가 얽히면 **보류 유지·사유 갱신 기록**.
- **e. NoCache 검토(판단·기록)**: watcher FileIdMap→NoCache 전환 — 부팅 walk 비용 제거
  대비 rename 경로 의미 변화(d-25 범위 외 사유)를 분석해 채택/기각을 근거와 함께 확정.
  채택 시에만 구현(+rename 시나리오 테스트), 기각이면 기록만.
- **f. capability deferred-attach(d-32 이월, 선택)**: 워처 2엣지(file::capability·git::watch)
  한정 — `ProjectCapability` 에 build/register 분리 도입이 d-25 부팅 계약(가드 밖 build)과
  충돌 없이 성립하는지 실사. 트레잇 재설계 비용이 크면 **이월 유지 기록**(d-32 §4 조건 승계).
- 범위 외: begin_mutation 입도 재설계(전역→세분 — 별도 캠페인)·git_pull 락 분리(T1-H 기결
  분기 재론 금지)·LspInstallProgress(d-34 이월분)·builtin_light(사용자 결정).

## 2. 실행·검증

- 구현: Rust 단독 순차 2 — R1(a+d — 기계적 이관 계열) → R2(b+c+e+f — 설계 계열). 각자
  계약 §3 기록. cargo fmt 필수.
- 검토 3렌즈(opus+xhigh): ① **동시성**(가드 구간 의미·락 획득 순서·데드락 신설 0·직렬화
  키 정합·상한과 blocking 풀 상호작용 — 최우선) ② 동등성(이관 전후 동작·에러 경로·테스트)
  ③ 경계·컨벤션(도메인 경계·doc 근거·매직넘버). major 적대적 → 수정 → 메인 2차(verify+
  vite+bindings 무변경). 커밋 없음.

## 3. R1 구현 완료 기록 (2026-08-25)

### a. git2 in-process 13건 — 실사 재확정 + 이관

`src-tauri/src/domain/git/commands.rs` 전수 grep(`begin_mutation().await` 보유 함수 중
`spawn_blocking` 없이 `service::*` 를 직접 호출하는 지점)으로 13건을 재산출 — T1-H §2.1 이
남긴 개수(13)와 정확히 일치, 기처리 발견 0건:

| # | 커맨드 | `service::` 함수 | git2 동작 성격 |
|---|---|---|---|
| 1 | `git_stage` | `stage` | `index.add_path`/`remove_path` + `index.write` |
| 2 | `git_unstage` | `unstage` | `repo.reset_default` |
| 3 | `git_discard` | `discard` | `repo.checkout_index`(+ untracked 는 파일시스템 휴지통 이동) |
| 4 | `git_branch_create` | `branch_create` | `repo.branch` (+옵션 `checkout_tree`) |
| 5 | `git_branch_checkout` | `branch_checkout` | `repo.checkout_tree` |
| 6 | `git_branch_delete` | `branch_delete` | `repo.graph_descendant_of` + `branch.delete` |
| 7 | `git_stash_push` | `stash_push` | `repo.stash_save` |
| 8 | `git_stash_apply` | `stash_apply` | `repo.stash_apply`(+안전 checkout) |
| 9 | `git_stash_drop` | `stash_drop` | `repo.stash_drop` |
| 10 | `git_resolve_conflict` | `resolve_conflict` | 워킹트리 write + `index.add_path`/`write` |
| 11 | `git_tag_create` | `tag_create` | `repo.tag`/`repo.tag_lightweight` |
| 12 | `git_tag_delete` | `tag_delete` | `repo.tag_delete` |
| 13 | `git_checkout_remote_branch` | `checkout_remote_branch` | `repo.branch`(옵션) + `repo.checkout_tree` |

13건 전부 T1-H 확립 동형 패턴으로 이관: `let _guard = state.begin_mutation().await;` →
(동기 준비) → `tauri::async_runtime::spawn_blocking(move || service::xxx(...))
.await.map_err(...)??;` — 가드는 함수 스코프 전체(진입~끝)에서 불변으로 유지, 이동한 것은
"블로킹 작업이 실행되는 스레드"(async 워커 → blocking 풀)뿐. non-Send 이슈: 13건 모두
`service::xxx` 가 `&Path`/`&str`/원시값만 받고 **함수 내부에서 `git2::Repository` 를 열고
닫는다** — `Repository` 값 자체가 스레드 경계를 넘는 지점이 없으므로(기존 이관 5건
`commit`/`stage_hunk`/`unstage_hunk`/`stage_lines`/`unstage_lines`/`discard_hunk`/
`revert_commit`/`undo_last_commit` 과 동일 형태) 구조 변경 없이 그대로 이관 — 보류 0건.
개별 특이사항: `git_branch_create` 의 `checkout: bool` 은 `Copy` 라 클로저로 move 해도
바깥 바인딩이 await 뒤 분기(`if checkout { emit_status_changed(...) }`)에서 그대로 재사용
가능(값이 클로저 안에 복사됨) — 원본 흐름 불변. `git_stash_push` 의 `message: Option<String>`
은 `.as_deref()` 결과(`Option<&str>`)를 클로저 밖으로 못 들고 나가므로 `message: Option<String>`
자체를 move 하고 `.as_deref()` 를 클로저 안에서 호출하도록 재배치(반환 타입·의미 불변).

### d. pty_spawn — 실사 후 이관 (보류 해제)

T1-H §4 원문의 보류 사유 3개를 현재 코드 기준 재검증:

1. **"ms 급 점유로 등급 상이"** — 우선순위 분류 사유일 뿐 수정 불가 사유가 아님. 실측:
   `pty::spawn`(`infra/pty.rs:236`)이 가드 보유 중 하는 일은 `shell_integration::prepare`
   의 스크립트 파일 기록(zsh 3파일/bash 1파일, 동기 `std::fs::write`) + `openpty` +
   `slave.spawn_command`(fork/exec) + `std::thread::spawn` 3회 — 13건과 동일하게 "동기
   블로킹 작업이 async 워커 스레드를 점유"하는 형태.
2. **"분리 시 신규 회수 경로가 pty 회수 부재(R8#1)"** — 전제가 더 이상 성립하지 않음.
   (i) R8#1(pty 세션 회수 경로 부재)은 T1-J 에서 해소 — `TerminalStore::kill_project`/
   `kill_session` 이 각각 `project_close`/탭 닫기에서 실제로 호출됨(commands.rs:79-109 문서
   확인). (ii) 더 결정적으로, `PtySession` 자체가 `Drop` 을 구현(`infra/pty.rs:134-142`)해
   "no matter how it stops being reachable" 자식 프로세스 kill + pause 해제 + 임시 디렉터리
   정리를 보장 — `spawn_blocking` 이관이 새로 여는 유일한 창(그 join 을 기다리는 도중 바깥
   future 가 드롭되어 `TerminalStore` 삽입 전에 결과값이 그대로 버려지는 경우)이 정확히 이
   Drop guard 가 "그 외 어떤 경로로도 도달 불가능해지는" 케이스로 커버하는 대상이라, 신규
   회수 경로를 별도로 만들 필요가 없음.
3. **"T1-J 와 충돌 여지"** — T1-J 는 이미 완료·병합됨(PROCESS.md d-8). 진행 중 작업과의
   충돌 가능성 자체가 소멸.
4. **"R8#13 확증 없음"** — R8#13 식별 여부는 이번 판단과 무관(계약 §1-d 는 "a 와 동형으로
   풀리는지"만 요구, 특정 감사 항목 번호의 확증을 요구하지 않음).

세 사유 모두 소멸을 확인해 **보류 해제, a 와 동형 패턴으로 이관**: `domain::terminal::
commands::pty_spawn` 의 `let handle = pty::spawn(config, on_data, on_exit)?;` 를
`tauri::async_runtime::spawn_blocking(move || pty::spawn(config, on_data, on_exit))
.await.map_err(...)??;` 로 교체. `_guard` 는 함수 진입 시 획득한 그대로 함수 끝까지 보유
(범위 불변). `on_data`/`on_exit` 클로저는 이미 `pty::spawn` 자체의 제네릭 바운드
(`Fn(&[u8]) + Send + 'static`/`FnOnce(Option<i32>) + Send + 'static`)로 `Send + 'static`
이 강제돼 있었으므로 바깥 클로저를 한 겹 더 씌우는 데 새 바운드가 필요 없음. `PtySession`
의 필드 전부(`Box<dyn ... + Send>`/`Arc<Mutex<...>>`/`Option<u32>`/`Option<PathBuf>`)가
Send 라 반환값도 `spawn_blocking` 요건을 그대로 만족. 락 대기(`begin_mutation_blocking`)
를 쓰지 않고 async `begin_mutation().await` 로 먼저 획득한 뒤 가드 쥔 채 진입하는 형태라
Phase E 가 찾아낸 GIT-1(블로킹 풀 안에서 락 대기 → 교착) 클래스에 해당하지 않음.

### 검증

- `cargo build --lib`: 통과(경고 0).
- `cargo fmt --all --check`: 통과(diff 0).
- `cargo test --workspace`: 1073+3+6+17 전량 통과, 0 실패(신규/이행 테스트는 없음 — 락
  의미·반환 타입 불변이라 기존 테스트가 그대로 회귀 검증 역할).
- `cargo clippy --workspace --all-targets -- -D warnings`: 통과(경고 0).
- bindings 대조(`git diff src/shared/api/bindings.ts`): 커밋 계열(주석 아닌 실토큰) 변경
  0건 — 이번 배치가 만든 변경은 13건에 새로 붙인 `///` doc 주석이 TS 쪽 JSDoc 으로 전파된
  것뿐(커맨드 이름·파라미터·반환 타입 전부 동일). `pty_spawn` 은 본문에 `//` 라인 주석만
  추가(specta 비대상)라 bindings diff 자체가 0. 파일에 남아 있는 그 외 diff(`AppError` 에
  `Localized` variant·`AppErrorKind`·`LocalizedError` 추가)는 이번 작업 시작 전부터 이미
  워킹트리에 있던 `src-tauri/src/error.rs` 등 d-31~d-34 이월분 변경의 산물 — 이번 R1 작업이
  건드린 범위 밖.

## 4. R2 구현 완료 기록 (2026-08-25)

### b. repo 단위 push/fetch 재진입 직렬화 — 구현

`GitStore`(`src-tauri/src/domain/git/commands.rs`)를 필드 2개로 확장: 기존 `repo_roots:
Mutex<HashMap<ProjectId, PathBuf>>`(튜플 구조체 `.0` → 이름 있는 필드로 전환, 접근부
전수(`resolve_repo_root`·`git_init`) 갱신)에 `push_fetch_locks: Mutex<HashMap<PathBuf,
Arc<tokio::sync::Mutex<()>>>>`를 신설. `GitStore::push_fetch_lock(&repo_root)` 이
`entry(...).or_insert_with(...)` 로 repo 경로별 `Arc<Mutex<()>>` 를 get-or-create 해
반환하고, `git_push`/`git_fetch` 는 `resolve_repo_root` 직후 이 락을 얻어 서브프로세스
대기가 끝날 때까지 쥔다(`git_stage` 등 13건과 같은 "가드 쥔 채 spawn_blocking" 모양이 아니라
"이 락만 쥔 채 spawn_blocking" — `begin_mutation` 은 애초에 안 잡으므로 관여 없음).

- **락 순서·데드락**: `git_push`/`git_fetch` 는 `begin_mutation` 을 전혀 잡지 않는다(T1-H
  가 이미 제거) — 이번에 추가한 락이 두 커맨드가 잡는 **유일한** 락이라, "잡는 순서"라는
  개념 자체가 성립하지 않는다(둘이 동시에 걸리는 지점이 없으므로). `git_pull` 은 기존과
  동일하게 `begin_mutation` 전체 보유를 유지하고 이 신규 락은 전혀 호출하지 않는다(계약의
  "git_pull 전체 락 유지 불변" 그대로) — pull 과 push/fetch 의 동시 실행은 T1-H 가 이미
  수용한 대로 여전히 레이스이며(git 자체 lock 파일이 안전망), 이번 배치가 그 관계를 넓히지
  않았다.
- **타 repo 병행성**: 맵이 경로로 키잉되어 서로 다른 repo 는 독립된 `Arc` 를 받아 완전
  병행. 단위 테스트 `동일_repo_락은_동시_보유를_막고_다른_repo_락은_병행된다` 로 직접 검증
  (한쪽이 락을 쥔 상태에서 같은 repo 두 번째 획득은 50ms 안에 끝나지 않고, 다른 repo 획득은
  50ms 타임아웃 안에 즉시 성공).
- **맵 엔트리 수명**: `GitStore::remove`(`GitCacheCapability::detach` 경유, `project_close`
  때 호출)가 `repo_roots` 항목 제거와 함께 `push_fetch_locks` 항목도 `Arc::strong_count ==
  1`(이 맵만 쥐고 있음 — 진행 중 push/fetch 없음)일 때만 제거한다. 진행 중인 호출이 자신의
  `Arc` clone 을 쥔 상태에서 프로젝트가 닫히면 그 클론이 `Mutex` 를 계속 살려두므로 즉시
  제거해도 안전 자체엔 문제가 없지만, 그 사이 같은 경로가 **새 프로젝트로 재오픈**되면
  재오픈된 쪽의 다음 push/fetch 가 `entry(...).or_insert_with(...)` 로 무관한 새 `Mutex` 를
  만들어 옛 호출과의 직렬화가 깨진다 — 이 조건부 제거로 그 경우를 막는다. 대가는 best-effort
  누수: "push/fetch 도중 닫히고 같은 경로가 다시 열리지 않는" 프로젝트마다 최대 1개의
  `Arc<Mutex<()>>` 가 앱 수명 동안 남을 수 있다(기존 `repo_roots` 자체도 동일하게 명시적
  제거 전까지 무기한 유지되는 구조 — `resolve_repo_root` 의 기존 doc 이 이미 인정). 단위
  테스트 4건(`push_fetch_lock은_같은/다른_repo_root...`·`remove는_사용중이_아닌/사용중인_
  push_fetch_락...`)으로 get-or-create 동일성·독립성과 조건부 제거 양쪽 분기를 모두 검증.

### c. 원격 dispatch 동시 상한 — 구현

`ws.rs::handle_socket` 의 요청당 `tauri::async_runtime::spawn`(T1-H 가 512 동시 도달을
실측한 지점) 안에서, 스폰된 태스크 맨 앞에 `RemoteDispatchLimiter::acquire().await` 로
permit 을 얻고 `handle_request` 끝까지 쥔다. permit 을 스폰 **밖**이 아니라 **안**에서
얻도록 배치한 이유를 코드 주석에 명시: 연결의 단일 읽기 루프(`tokio::select!`)가 상한
포화 중에도 `stream.next()`/세션 만료·철회 두 arm 을 계속 폴링해야 하므로(포화 중에도
세션 철회는 즉시 반영되어야 함), 스폰 자체를 막아서는 안 되고 개별 요청 태스크만 대기시켜야
한다.

`RemoteDispatchLimiter`(`src-tauri/src/domain/remote/commands.rs`)는 `RemoteStore` 와
동일한 Tauri managed-state 관례로 신설, `lib.rs` 의 `RemoteStore::default()` 관리 직후에
`app.manage(RemoteDispatchLimiter::default())` 로 등록. 내부는
`tokio::sync::Semaphore::new(REMOTE_DISPATCH_MAX_CONCURRENT)` 하나뿐이고 `acquire()` 가
permit(`Option` — 세마포어가 닫히면 `None`, 이 앱 수명 동안 닫는 코드가 없어 사실상 항상
`Some`)을 돌려준다. permit 획득 실패(이론상만 가능한 경로)는 패닉 대신 로그 후 해당 요청만
조용히 처리하지 않고 리턴 — 세마포어 자체가 죽는 상황이면 그 원인이 이 요청 하나를
처리한다고 해결되지 않으므로, 개별 요청을 실패시키는 편이 태스크를 패닉시켜 커넥션 전체를
끊는 것보다 안전하다는 판단.

- **상한값 근거**(`REMOTE_DISPATCH_MAX_CONCURRENT = 128`,
  `src-tauri/src/domain/remote/types.rs`): 이 앱은 Tauri 기본 비동기 런타임을 그대로
  쓴다(`tauri::async_runtime::default_runtime` → `TokioRuntime::new()` = 순정
  `tokio::runtime::Runtime::new()` — 코드베이스 전체에 `Builder::max_blocking_threads` 호출
  0건, `grep` 로 재확인) — 즉 tokio 문서가 명시하는 기본값 그대로 blocking 스레드 풀 상한이
  512 다(**§5 정정 — C6**: T1-H 의 512 는 실측이 아니라 같은 tokio 기본값에서의 유도치라 이 문장은 순환 논거였음. 128 의 실근거는 tokio 기본 max_blocking_threads=512 단일 갈래·정상 사용 동시 요청이 수십 건을 넘지 않는다는 관측이며, 데스크톱 측 spawn_blocking 은 무상한이라 "384 슬롯 상시 확보" 함의는 성립하지 않음). 이
  풀은 데스크톱 자체 IPC 커맨드의 `spawn_blocking` 과 원격 dispatch 의 `spawn_blocking`
  이 **공유**하는 프로세스 전역 자원이므로, 상한을 512 에 딱 맞추면 데스크톱 쪽 몫이 전혀
  안 남는다. 512 의 1/4 인 128 로 설정해 데스크톱 쪽에 384 슬롯의 여유를 항상 남기면서도,
  정상적인 원격 사용(프로젝트 열기·탭 전환 한 번에 많아야 수십 개 커맨드가 동시에 뜨는
  정도)은 128 에 전혀 근접하지 않아 체감 지연이 없다 — doc 주석에 이 산출 과정을 그대로
  기록.
- **초과 시 거부 아님**: `acquire()` 는 permit 이 빌 때까지 `.await` 로 대기만 하지 에러를
  반환하지 않는다 — 계약의 "초과 시 대기(거부 금지)" 그대로. 단위 테스트
  `상한을_넘는_요청은_거부되지_않고_permit_반환을_대기한다`(permit 1개짜리 세마포어로
  구성해 재현)로 직접 검증: 두 번째 획득 시도가 첫 번째가 permit 을 반환하기 전까지
  50ms 넘게 미완료 상태로 남고, 반환 후에야 완료된다. `서로_다른_permit은_동시에_진행된다`
  로 기본 상한(128) 미만 동시 요청은 대기 없이 즉시 진행됨도 함께 확인.
- **기존 dispatch 테스트 무회귀**: `collect_commands_매크로_출력과_dispatch_테이블은_
  커맨드_이름_집합이_일치한다`·`typescript_바인딩을_생성한다` 등 기존 dispatch/바인딩
  테스트 전량 통과 — 세마포어 도입이 커맨드 표면(이름·시그니처)에 어떤 영향도 주지 않음을
  재확인(애초에 `RemoteDispatchLimiter` 는 Tauri 커맨드도 specta 대상도 아니라 바인딩에
  나타나지 않는다).

### e. NoCache 검토 — 기각(판단·기록)

`infra/watcher.rs` 는 `notify_debouncer_full::new_debouncer(..., None, ...)` 로
`RecommendedCache`(macOS/Windows 에서는 `FileIdMap`, Linux/Android/wasm 에서는 이미
`NoCache` — `notify-debouncer-full` 0.7.0 `cache.rs`)를 암묵 사용 중이다. 이 앱의 릴리스
타깃은 macOS 단독(`\.github/workflows/release.yml` 의 `runs-on: macos-latest` 만 존재)이라
검토 대상은 사실상 "macOS 에서 `FileIdMap` 대신 `NoCache` 를 명시 채택할 것인가" 하나다.

**메커니즘 실사** (vendored 소스 직독,
`~/development/rust/cargo/registry/.../notify-8.2.0/src/fsevent.rs`·
`notify-debouncer-full-0.7.0/src/{lib,file_id_map,cache}.rs`):

1. macOS 백엔드(FSEvents)는 rename 의 from/to 를 연결할 **cookie 를 아예 제공하지 않는다**
   — notify 자체 주석 원문: "FSEvents provides no mechanism to associate the old and new
   sides of a rename event." (`fsevent.rs:192-193`). 즉 macOS 에서 `notify-debouncer-full`
   의 `handle_rename_to` 가 rename 의 from/to 를 짝짓는 **유일한** 경로는
   `file_ids_match`(=`FileIdCache::cached_file_id` 비교) 뿐이다 — `trackers_match` 는
   macOS 에서 항상 거짓.
2. **단순 분류(우리가 처음 걱정했던 것)는 캐시 종류와 무관하게 동일한 결과를 낸다**: 짝이
   맞춰지면(`FileIdMap`) `Modify(Name(Both))` 이벤트 1개(`paths=[old,new]`)가, 안 맞춰지면
   (`NoCache`) `Modify(Name(Any))` 이벤트 2개(**§5 정정 — L3-7**: macOS FSEvents 백엔드는 양쪽 다 RenameMode::Any 로 올린다. 각각 `paths=[old]`,
   `paths=[new]`)가 나온다. `infra::watcher::group_relevant_changes` 는 어느 kind 든
   `Modify(Name(_))` 를 전부 `Renamed` 버킷에 경로별로 평탄화해 담으므로, 두 경우 모두
   최종 `renamed: Vec<String>` 에 old·new 가 똑같이 들어간다 — **여기까지는 캐시 종류가
   관측 가능한 차이를 전혀 만들지 않는다.**
3. **그러나 실제 의미 변화는 다른 곳에 있다**: 매칭이 성립하면
   (`push_rename_event`, `lib.rs:413-`) old 경로에 이미 큐잉돼 있던 **다른** 이벤트들(예:
   같은 디바운스 창 안에서 rename 직전에 일어난 content modify)도 전부 new 경로로
   재귀속(`for e in &mut source_queue.events { e.paths = vec![event.paths[0].clone()]; }`)
   된다. 매칭이 실패하면(`NoCache`) 이 재귀속이 일어나지 않아, "수정 후 같은 창 안에서
   rename" 시퀀스가 **더 이상 존재하지 않는 old 경로에 대한 Modified 이벤트**로 잘못
   보고된다(new 경로는 rename 만 됐지 modify 는 보고되지 않음). 이것이 정확히 d-25 가
   범위 외로 남긴 "rename 경로 의미 변화" 사유의 실체 — 짐작이 아니라 소스 레벨로 확인.
4. **부팅 walk 비용**: `Debouncer::watch` → `add_root` → `cache.add_path(root, Recursive)`
   가 프로젝트 루트 전체를 `WalkDir` 로 훑는 지점(`notify-debouncer-full-0.7.0/src/lib.rs`
   의 `add_root`:577·`watch`:600, `data.cache.add_path` 호출:589 — TAIDE 자체 `lib.rs` 가
   아니라 vendored 크레이트)이자 d-25 가 지목한 비용의 실체. 하지만 d-25 가 이미
   `attach_watcher`/`build_git_watcher_handle` 호출 자체를
   `spawn_blocking` + 백그라운드 태스크(`restore_project_watchers`)로 부팅 크리티컬
   패스 밖으로 옮겨뒀다 — `NoCache` 로 바꿔도 얻는 건 "이미 비차단인 백그라운드 작업의
   총 소요 시간 단축"뿐, 부팅 자체는 더 빨라지지 않는다.

**결론**: 기각. 3번에서 확인한 재귀속 상실이 실제 정확도 저하(코드 에디터의 파일 캐시
무효화가 존재하지 않는 경로를 가리키게 됨)이고, 4번의 유일한 이득(부팅 walk 비용)은 이미
d-25 로 비차단화되어 크지 않다 — 계약이 명시한 "판단이 애매하면 기각 쪽" 기준에 부합하는
정도를 넘어, 이번엔 명확한 근거로 기각. `infra/watcher.rs` 는 무수정.

### f. capability deferred-attach — 이월 유지(판단·기록)

`domain::file::capability::attach_watcher` 의 기존 doc(수정 없이 그대로 인용)이 이미 이
판단의 핵심을 명문화하고 있었다: "`project_open`'s `FileWatcherCapability::attach` is the
only caller that still builds and registers in one call — it already runs its entire
`ProjectCapabilities::attach_all` walk under one `AppState::begin_mutation` acquisition,
so splitting internally here changes nothing about when that guard is held." 즉
`ProjectCapability::attach` 트레잇 메서드 자체를 build/register 2단계로 쪼개도,
`project_open`(`domain/project/commands.rs`)이 `attach_all` 을 `begin_mutation` 을 쥔 채로
**한 번에** 동기 호출하는 구조가 그대로면 그 무거운 `FileIdMap` walk 은 여전히 가드 안에서
돈다 — 트레잇 시그니처만 바꿔서는 아무것도 달라지지 않는다.

실제로 이득을 보려면 `project_open` 자체를 부팅 복원 경로(`restore_project_watchers`)와
같은 모양으로 재구성해야 한다: watcher 계열 capability 만 골라 **가드 밖에서 먼저 build**
하고, 그 결과를 들고 가드를 잡아 **등록만** 하는 식. 이건 두 가지 비용 중 하나를 반드시
문다 — (a) `ProjectCapability` 트레잇 자체를 모든 구현체(agent·layout·ide·terminal·tree
등 watcher 아닌 5개 포함)가 build/register 2단계를 갖도록 일반화하면, 정작 그 분리가 필요
없는 다섯 곳에 전부 보일러플레이트 no-op 단계가 늘어난다. (b) 반대로 watcher 2개만
`project_open` 안에서 특별 취급하면, `ProjectCapabilities::attach_all` 이 "등록 순서 자체가
정합성 계약"(`project/capability.rs` 의 doc — dirty-layout flush 순서·terminal reap 순서
등)이라며 유지해온 "단일 순회로 전부 처리" 불변식이 깨진다 — 두 capability 만 그 순회 밖으로
빼내면 나머지와의 상대적 순서를 별도로 다시 보장해야 한다.

이미 `build_watcher_handle`/`register_watcher_handle`(`file/capability.rs`)과
`build_git_watcher_handle`/`register_git_watcher_handle`(`git/watch.rs`)이 **함수 단위**
빌드/등록 분리를 갖추고 있어(부팅 복원 경로가 실제로 쓰고 있음), d-32 가 걱정했던 "필요한
빌딩 블록"은 이미 존재한다. 남은 건 그 블록들을 `project_open` 경로에서도 쓰도록
`project_open`/`ProjectCapability` 트레잇을 재설계하는 것뿐인데, 위 (a)/(b) 비용이 실재하고
`project_open` 은 부팅 복원과 달리 **단일 프로젝트, 사용자가 방금 누른 동기적 액션**이라
체감 지연의 실사용 임팩트도 다건 복원 경로보다 작다(복원 루프가 N 개 프로젝트를 순차로
훑는 것과 달리 project_open 은 1건). 계약 §1-f 의 "트레잇 재설계가 필요하면 이월 유지
기록" 조건이 정확히 성립 — 이번 배치에서는 트레잇·`project_open` 구조 무수정, 이월 유지.

### 검증

- `cargo fmt --all --check`: 통과(diff 0).
- `cargo test --workspace`: 1080+3+6+17 전량 통과, 0 실패(git 5건·remote 2건 = 신규 7건(초판 "6건"은 검토 L3-8 정정) 신규 테스트
  전부 포함).
- `cargo clippy --workspace --all-targets -- -D warnings`: 통과(경고 0).
- bindings 대조(`git diff src/shared/api/bindings.ts`, 주석 라인 제외 실토큰만): 변경
  0건 — git_push/git_fetch 에 새로 붙인 doc 주석이 JSDoc 으로 전파된 것 외에는 R1 과 동일한
  판정(워킹트리에 이미 있던 `AppError` `Localized` 관련 diff 는 이번 R2 범위 밖).
- b·c 는 코드 변경(GitStore 확장·RemoteDispatchLimiter 신설)+테스트 6건, e·f 는 판단만(코드
  무수정) — 계약 §2 의 "각자 계약 §3 기록" 요구를 §4 로 충족.
- 보류 0건(a·d 전건 이관 완료). R2(b+c+e+f)는 범위 밖 — 별도 진행.

---

## 5. 검토·적대적 반영 (2026-08-25)

> 검토 wf_9c3d1c72-6f9(3렌즈): 동시성 major 3(C1 취소 우선순위 역전·C2 철회 후 실행 창·C3
> repo 락 무한 대기 확산)+동등성 major 1(eq-01 — C7 과 동류의 기록 결함)·minor 계 ~19.
> 적대적 wf_d577f0d2-795(건별): **3건 전부 downgraded** — 구조 사실은 전건 성립하나 실사용
> 도달성(128 포화는 단일 사용자+보조 원격 세션 실사용에서 비현실·UI 자가 잠금·gitFetch 프론트
> 호출 0)과 심각도(자료 손상·교착 0·자동 해소·baseline 대비 집합 불변/시간축만 변화·전역
> 자원은 오히려 개선) 미달. **처분 = 이번 배치 코드 무수정, doc 수용 비용 명시 + 아래 이월.**

### 5.1 이월 — "원격 dispatch QoS" 후속 캠페인 (한 묶음, 개별 패치 금지)

C1(취소 계열 permit 면제/차등)·C2(대기 중 세션 재검증 또는 연결 단위 CancellationToken)·
C3(run_git 수명 관리 — Child 기반 kill 경로·GIT_TERMINAL_PROMPT=0·lowSpeed 상한, 영향 반경
run_git 9개 호출부 전체)·C4(permit 실패 시 seq 에러 응답)는 전부 "요청 태스크의 생애 관리"
라는 같은 뿌리 — 개별 패치하면 서로의 전제를 흔들므로 한 캠페인으로 묶는다. **선행 조건**:
① 철회의 in-flight/큐잉 요청 semantics 를 계약으로 먼저 확정(진행 중 dispatch 취소 여부·
에러 코드 계약·mutation 계열 중도 취소 안전성 감사) ② handle_socket 구동 통합 테스트(가짜
소켓+상한 1 축소 limiter)로 배선을 먼저 고정(eq-05 — 현행 테스트 7건은 프리미티브만 고정,
permit 블록·락 획득부를 지워도 그린인 한계). 분류 단일 출처는 dispatch.rs.
채택 불가 확정(적대적 판정): spawn_blocking timeout 씌우기(가드만 풀리고 서브프로세스 생존 —
§1-b 가 제거한 .git 경합 재유입)·GIT_TERMINAL_PROMPT=0 단독 추가(지배 트리거 미해소+9경로
회귀 표면).

### 5.2 §4-f (b) 논거 축소 (L3-6 반영)

이월 유지 비용 (b)의 "attach_all 등록 순서 불변식 파괴" 서술은 과장 — 실물 doc 은 detach
측 순서 근거만 보유(layout flush 선행·terminal 조기 회수). (b)는 "단일 순회 구조·핀 테스트
(공유_순회는_등록_순서_전방이다) 유지 비용"으로 축소해 읽는다. 결론(이월 유지)은 (a)
보일러플레이트 비용만으로 성립.

### 5.3 그 외 minor 처분

fixer wf_7bf3959f-65b 가 doc·문서 일괄 반영(ws.rs permit 수용 비용 2문단·push_fetch_lock
무상한 대기 수용 비용·types.rs 2건 doc 정정·state.rs 홀더 확대·git_fetch 오지시·pty_spawn
// 14줄 → /// 흡수·GitStore::remove 호출자·capability doc·domain_boundaries stale 포인터·
NoCache back-pointer(d-25 계약+watcher.rs)·정본 문서 3곳(git.md·remote-control.md·
architecture.md)·qa 체크 1건·PROCESS 수치). 이 계약의 수치·문언 정정 4건은 메인 직접(§0·
§4-c·§4-e-2·검증 절 — 위 인라인 "§5 정정" 표기).

### 5.4 계약 잔여 오기 정정 (eq-02·eq-04)

- eq-02: §4 검증의 신규 테스트는 "git 5건·remote 2건 = 7건"이 정본(§5.3 의 L3-8 정정과 동일
  건 — R2 초판 "git 6건" 및 검토 지시문의 "8건" 표기는 오기).
- eq-04: `GitStore::remove` 의 호출자는 `GitCacheCapability::detach`(project_close) **및
  `git_init`**(재초기화 시 캐시·락 회수) 2곳 — §4-b 수명 문단에 init 경로 포함으로 읽는다.
  (코드 doc 은 fixer wf_7bf3959f-65b 가 L3-2 로 정정 완료.)
