# native lane git 묶음 3/8 (g3-header-branches) — 헤더·브랜치·stash·원격

> 작성일: 2026-11-02(검증 2026-11-03) · worktree `TAIDE-lanes/git`(branch `lane/git`) · 사양 `docs/research/2026-10-28-lane-git-spec.md` git-3~9·3.1 G3
> 결정: `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안, git 검사의 전역 설정 읽기 허용), `docs/acknowledge/2026-10-26-native-autonomous-continuation.md`
> 선행: G1(`2026-11-02-native-lane-git-g1-base.md`)·G2(`2026-11-02-native-lane-git-g2-changes.md`)의 미커밋 변경 위에서 작업했습니다. 앞 작업자가 이 묶음 구현·검사 초안을 남겨 두었고, 이번에 검사 등록·검사 helper 정정·검증·문서를 마쳤습니다.

## 1. 대상과 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| git-3 | complete | 헤더 왼쪽 트리거(GitBranch 14px + medium 12px 브랜치명 말줄임, 없으면 `git.noRepositoryLabel`·비활성 50%), tooltip(아래)·이름 `git.branchSwitch`, 열림 상태 = accesskit expanded |
| git-4 | complete | popover 256px(시작 정렬, 4px 아래), 필터 입력 자동 focus·`git.branchFilterPlaceholder`, 로컬/원격 그룹(trim·대소문자 무시 포함 비교, 빈 그룹 숨김), HEAD 아이콘·Check accent, 원격 `git.checkoutRemoteBranch`, ↑↓/Home/End(끝에서 멈춤)·포인터 강조·Enter/클릭 실행, Escape·바깥 클릭 닫기(필터 유지), 체크아웃·원격 체크아웃 → `git.branchSwitched {name}`/오류 toast |
| git-5 | complete | trim한 필터와 정확히 같은 이름(로컬·원격 전체)이 없으면 맨 위 Plus + `git.branchCreateNamed {name}`, 생성+전환 → `git.branchSwitched`, 만들기 항목만 있을 때 `git.branchNotFound` 미표시 |
| git-6 | complete | `has_remote`일 때만 ahead/behind(ArrowUp/Down 12px + 수, 0이면 숨김), 첫 remote 이름(11px 보조 색, 왼쪽 4px)은 remote가 있으면 표시 |
| git-7 | partial | Sync: `has_remote`일 때만, 클릭 → pull, 성공 시 push(transient task), 진행 중 Loader 회전·비활성, 각 실패 오류 toast·pull 실패면 push 미실행. 남은 것: pull/push OS 알림 4종(진입점 병합 뒤 연결) |
| git-8 | complete | Archive 버튼(tooltip `git.stashPush`, 헤더 오른쪽 정렬 시작), 변경이 있을 때만 활성, `git_stash_push(None)` → `git.stashPushed`/오류 toast |
| git-9 | complete | stash 0개면 섹션 숨김, 머리말 `git.stash` + 건수 배지(접혀도), 기본 접힘(git-18 설정), 행(Archive + 메시지 말줄임 + `git.stashApply` 글자 버튼 + Trash2 오류 색 tooltip `git.stashDrop`), push/apply/drop 진행 중 두 버튼 비활성, 확인 없이 실행, 성공 toast 없음·실패 오류 toast, 변경 목록과 한 스크롤 영역, roving 정거장(stash 머리말) |

## 2. 원본 근거

사양 git-3~9 절을 따랐고, 이번에 다시 확인한 원본 구간만 적습니다.

- 스위처: `src/features/git/branch-switcher.tsx`(trim 필터, `hasExactMatch` = 이름 전체 일치, 만들기 그룹 조건, 선택 시 닫기·필터 비움, 트리거 `gap-1.5 px-1 py-0.5`·`size-3.5`·`font-medium`·`disabled:opacity-50`, `PopoverContent align='start' w-64 p-0`, cmdk `shouldFilter={false}`), `src/features/git/branch-group.tsx`.
- 헤더: `src/widgets/git-panel/git-panel.tsx:494-536`(`h-8 gap-1.5 px-2 text-xs`, `hasRemote && ahead > 0` 등, stash 버튼 `ml-auto`, sync는 `hasRemote`일 때만, remote 이름은 `remote &&`라 `has_remote`와 무관하게 표시).
- stash: `src/features/git/stash-list.tsx`(행 `gap-1.5 px-2 py-1`, Apply `px-1.5 py-0.5` 글자 버튼, Drop `IconButton` `text-status-error p-1`, 둘 다 `disabled`), `git-panel.tsx:632-644`.
- 실행: `src/widgets/git-panel/git-panel-container.tsx:109-138,206,283-284`, `src/entities/git/git.query.ts:190-237`(pull 성공 뒤 push, 알림).

## 3. 구현

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-app/src/git-branch.rs`(신규) | 순수 `listing(branches, filter)` → `Listing { create, local, remote }`, `Listing::choices()` → `Choice::{Create, Checkout, CheckoutRemote}`(cmdk 항목 순서). `show`: 트리거(비활성이면 hover sense만, tooltip 없음, accesskit expanded), egui 임시 메모리의 `Picker { is_open, filter, active, focus_input, composition }`(패널 mount id 단위, 원본 popover 로컬 state). `popup`: `egui::Popup`(트리거 기준 BOTTOM_START, gap 4, 폭 256, `CloseOnClickOutside`), 검색 아이콘 + 단일 줄 입력(첫 비 sizing 프레임에 focus, focus lock filter로 화살표가 focus를 옮기지 않음), ↑↓/Home/End는 입력 focus 중·IME 조합 아닐 때만, 필터가 바뀌면 첫 항목 강조, 포인터 이동이 강조를 옮김, Enter/클릭 → intent + 상태 초기화 + 트리거 focus, Escape(조합 중 제외) → 닫고 트리거 focus. `Items`: 빈 목록 `git.branchNotFound`, 만들기 항목(Plus), 그룹 머리말, 항목(강조 배경 + accent 안쪽 링, HEAD 아이콘·Check accent, 원격 힌트 10px) |
| `native/taide-native-app/src/git-scm.rs` | `View`에 `remotes`·`branches`·`stashes`·`is_stashing`·`is_syncing`. `header`: 스위처 폭만 줄어들고 ahead/behind → stash 버튼 → sync 버튼 → remote 이름을 오른쪽부터 배치, sync 진행 중 `LoaderCircle`을 `input.time` 기반 각도로 회전(repaint 요청). stash 섹션: 변경 목록 아래 같은 `ScrollArea`에 머리말(배지·접힘 토글) + 펼침 시 행들, roving 정거장 끝에 stash 머리말(`stops`, ←/→ 접기·펼치기). `stash_row` |
| `native/taide-native-app/src/domains/git.rs` | `Command::{BranchCheckout, CheckoutRemote, BranchCreate(checkout true), StashPush(None), StashApply, StashDrop, Pull, Push}` 모두 `change` → `spawn_transient`. `MutationOp::{SwitchBranch(name), StashPush, StashApply, StashDrop, Pull, Push}`. 응답: 성공이면 가변 캐시 stale + (전환) `git.branchSwitched`·(stash push) `git.stashPushed`, 실패면 `Toasts::ipc_error`. `State.stashing`(프로젝트별 진행 수)·`State.syncing`(Pull 성공 응답에서 Push 제출, 실패·제출 불가·Push 응답에서 해제), 재연결·프로젝트 닫힘에서 정리. `Intent::Shown`이 `Remotes`·`Branches`·`Stashes` 캐시 항목도 관찰. `header_tests` 등록 |
| `native/taide-native-ui/src/intents/git.rs` | `Intent::{CheckoutBranch, CheckoutRemoteBranch, CreateBranch, Sync, StashPush, StashApply, StashDrop}` |
| 테스트 | `git-branch-tests.rs`(3), `application-git-header-tests.rs`(4, 이번에 `domains/git.rs`에 등록), G2 `application-git-changes-tests.rs`의 `View` 직접 생성 1곳에 새 필드 추가 |

설계 판단(원본에 가장 가까운 안):

- 첫 강조 항목: cmdk는 목록의 첫 항목을 강조합니다. 브랜치 순서는 백엔드(`taide_git::service::branches`, libgit2 ref 순회 = 이름 정렬)를 그대로 쓰므로 `feature`가 `main`보다 앞에 옵니다. 원본도 같은 백엔드 순서입니다.
- Escape·바깥 클릭: Radix Popover는 닫을 때 필터를 비우지 않으므로(`setFilter('')`는 선택 때만) 다시 열면 필터가 남습니다. native도 같습니다.
- sync 상태: 원본 `isSyncing = pull.isPending || push.isPending`. native는 Sync 클릭부터 push 응답까지 한 집합으로 묶어 pull→push 사이 한 프레임 깜빡임이 없습니다(원본의 두 mutation 사이 렌더 간격은 사용자에게 보이지 않는 수준이라 결함 재현 대상으로 보지 않음).
- stash 진행 상태: 원본 `stashPending`은 push·apply·drop 세 mutation의 pending 합이므로 프로젝트별 진행 수로 셉니다.

## 4. 사양·대응표와 실제 코드 차이

- **git-7 알림 진입점**: 사양 1.2·git-7은 settings lane의 `domains::settings::notify`를, 이번 지시는 shell lane 진입점을 말합니다. 이 worktree에는 어느 쪽 진입점도 없습니다(`grep "fn notify" native/taide-native-app/src/domains/` 결과 없음). 새 경로를 만들지 않고 git-7을 partial로 두었습니다.
- **git-4 검사 fixture**: 사양은 원격 `origin/feature-remote`를 bare 원격에 만든다고만 적었습니다. 검사는 `git push origin main:feature-remote` 뒤 fetch로 만들어, 원격 체크아웃 뒤 `a.txt`가 main 내용인 것으로 추적 브랜치 생성을 확인합니다.
- **git-6 behind fixture**: 사양은 "원격 쪽 커밋을 테스트 쪽에서 fetch"입니다. 같은 저장소에서 커밋·push 뒤 `reset --hard`로 되돌려 behind 1을 만들었습니다(원격 ref 갱신이 같은 결과).

## 5. 검증

명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/git`을 붙여 lane worktree에서 실행했습니다.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --all-targets --manifest-path native/taide-native-app/Cargo.toml` | 첫 실행 실패 1건: G2 `application-git-changes-tests.rs:602`의 `git_scm::View` 직접 생성에 새 필드 5개 누락 → 검사 쪽에 App 캐시 값·`false`를 넣어 해결. 이후 통과(기존 경고만: `is_applying`, `zen.rs:543` mut, `fixture_with_emmet_peek`, vendor wry-preview) |
| `cargo check --quiet --all-targets --features native-host,inspection --manifest-path native/taide-native-ui/Cargo.toml` | 통과(기존 경고 `tests/workbench.rs:1816`만) |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 통과(remote-web 기존 dead code 경고만, 소스 변경 없음) |
| `cargo test --quiet ... taide-native-app --lib git_branch:: -- --test-threads=2` | 3 통과 |
| `cargo test --quiet ... taide-native-app --lib domains::git::header_tests -- --test-threads=2` | 첫 실행: 이 검사 파일이 `domains/git.rs`에 등록되지 않아 컴파일되지 않고 있었음 → 등록. 4개 중 3 통과·1 실패(스위처: 첫 강조 항목 기대값이 `main`, 실제 백엔드 순서는 `feature` 먼저) → 기대값을 원본과 같은 백엔드 순서로 정정. 단독 재실행 2회 `phase=picker` 시간 초과 → 진단 출력으로 `node(label)`가 트리거 대신 남아 있던 tooltip 노드(같은 문구, 아래 위치)를 집어 클릭이 빗나간 것을 확인 → 트리거는 expanded 상태가 있는 노드(`trigger`), 버튼은 `Role::Tooltip`이 아닌 노드(`button`/`buttons`)로 고르도록 검사 helper 정정(구현 변경 없음) → 4 통과 |
| `cargo test --quiet ... taide-native-app --lib -- git title_branch explorer shell_keymap command_dispatch --test-threads=2` | 1차 43개 중 42 통과·1 실패(sync 검사 `phase=pushed`, 위와 같은 tooltip 노드 혼동) → helper 정정 뒤 `cargo fmt` 후 재실행 67 통과(G1 base·G2 changes·G3 header·브랜치·캐시·모델·remote-git·title_branch·explorer 포함) |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml` 후 `-- --check`, `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 통과(`cargo fmt`는 이 묶음 파일 4개만 바꿈: `git-branch.rs`·`git-scm.rs`·`domains/git.rs`·`application-git-header-tests.rs`, `git status` 파일 목록 변화 없음) |

자동 검사 내용:

- `[C]` `git-branch-tests.rs` 3개: 빈 필터·공백 필터는 만들기 없이 로컬/원격 전부(cmdk 항목 순서 = 만들기 → 로컬 → 원격), trim·대소문자 무시 포함 비교와 빈 그룹, 정확히 같은 이름(로컬·원격 전체 이름, 대소문자 구분)이면 만들기 없음, 브랜치가 없으면 항목 없음.
- `[A]` `application-git-header-tests.rs`(main.rs가 만드는 `NativeApplication`과 같은 frame 경로, ⌃⇧G로 Git 보기, 임시 저장소·같은 임시 디렉터리의 bare 원격만):
  1. 스위처: 첫 status 전 `git.noRepositoryLabel` + 비활성, 도착 뒤 `main` + 활성, hover tooltip이 트리거 아래, 클릭 → expanded, 세 항목·두 그룹 머리말·원격 힌트·입력 focus, 첫 항목 강조, `FEAT` 입력 → 만들기 항목이 맨 위·강조, ↓/↓↓(끝에서 멈춤)/Home, ↓ Enter → `feature` 체크아웃(파일 내용·`git rev-parse` 확인)·toast·popover 닫힘, 다시 열면 필터 비어 있음, ` feature ` → 만들기 없음·`git.branchNotFound` 없음, Escape → 닫힘·트리거 focus, 다시 열면 필터 유지, 포인터 이동 강조 → 원격 클릭 → 추적 브랜치 `feature-remote` 생성·전환·toast, `topic` 만들기 → 빈 그룹 머리말 숨김 → Enter 생성·전환, 작업 트리 충돌 체크아웃 → 오류 toast·HEAD·내용 불변, detached HEAD → `HEAD` 표시.
  2. 브랜치 없는 새 저장소: 목록 `git.branchNotFound`, 입력하면 만들기 항목만 있고 빈 목록 문구 없음.
  3. 원격 없음 → sync·remote 이름 없음, 원격 추가·upstream 설정 → `origin` 표시·sync 버튼, 수 0이면 숫자 없음, behind 1 → `↓1`, 테스트 쪽 mutation 잠금을 쥔 채 sync 클릭 → `syncing`·버튼 비활성 → 잠금 해제 → pull로 HEAD가 원격과 같아지고 표시 사라짐·버튼 활성, ahead 1 → sync → 원격 `refs/heads/main`이 로컬 커밋(push 실행)·오류 없음, 원격 경로를 옮겨 pull 실패 → 오류 toast·원격 ref 불변(push 미실행).
  4. stash: 변경 없으면 stash 버튼 비활성·섹션 없음, 변경 → 활성 → stash → `git.stashPushed`·목록 비움·파일 복원(2회), 섹션 배지 `2`·기본 접힘·Apply 없음, 펼침 → Apply·Drop 각 2, 잠금 쥔 채 Apply → 두 버튼 모두 비활성 → 해제 → 파일 복원·성공 toast 없음, Drop → 1개(테스트 쪽 `git stash list` 확인), 행 클릭 뒤 ↓ → stash 머리말 focus(expanded), ← 접힘, ↑ → 행으로 복귀.

실행하지 않은 것: 필터 없는 앱 전체 테스트(메인이 병합 때), 실제 원격 서버 push/pull(허락 3), OS 알림(진입점 없음·허락 1).

## 6. 남은 범위

- git-7: pull/push 성공·실패 OS 알림 4종(성공 범주 `gitRemote`, 실패 범주 `error`, 제목 = 프로젝트 표시명 또는 `notification.gitPull|PushSucceeded|Failed`, 본문 `[문구, 캐시된 branch 또는 describe_error]`를 ` · `로 연결). 알림 진입점(settings lane `domains::settings::notify` 또는 shell lane 진입점)이 병합된 뒤 `apply_git_reply`의 `MutationOp::Pull`·`Push` 분기에서 호출하고, 가짜 platform으로 4종을 단언합니다(사양 3.1 G3 비고대로 G8 끝 또는 병합 뒤).
- 이번 묶음 밖 행에 미친 영향(대응표는 이번 ID 행만 갱신): git-26의 stash 머리말 정거장과 git-17의 "stash 섹션이 같은 스크롤 영역"은 이번 구현과 4번 검사로 충족됐습니다. 두 행의 갱신은 그 행을 다루는 묶음(git-26은 G6 그래프 머리말)에서 합니다.
