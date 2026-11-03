# native lane git 묶음 1/8 (g1-base) — 기반·진입·변경 목록 표시

> 작성일: 2026-11-02 · worktree `TAIDE-lanes/git`(branch `lane/git`) · 사양 `docs/research/2026-10-28-lane-git-spec.md` 2.0·git-1·2·16·18·19·37·38·64·3.1 G1
> 결정: `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안, D1 A), `docs/acknowledge/2026-10-26-native-autonomous-continuation.md`

## 1. 대상과 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| git-64 | complete | `git-cache.rs` 조회 캐시·무효화·관찰·요청 상한, PaintSink 기록, 실제 App 재조회 |
| git-1 | complete | ⌃⇧G·`view.git`·보조 창 키로 Git 보기, focus slot 규칙, 접힌 사이드바 펼침 |
| git-2 | complete | 비저장소 안내·초기화 버튼·진행 중 비활성·성공/실패 toast·성공 후 패널 전환 |
| git-37 | complete | shell-61·shell-6 구현 + 비저장소 숨김·초기화 뒤 branch 표시 단언 추가 |
| git-38 | partial | D1 A: 공급(`GitCache::status_result`)만. 탐색기 주입점이 이 worktree에 없어 병합 뒤 연결 |
| git-16 | complete | Merge/Staged/Changes 구성, 건수 배지, 빈 그룹 숨김, `git.noChanges` |
| git-18 | complete | 섹션 접힘 토글마다 전체 목록 저장, 연속 토글 보존, 새 App 복원 |
| git-19 | complete | 파일명+디렉터리, 상태 글자·색 7종, 행 tooltip, hover 시 액션 노출·글자 숨김 |

## 2. 원본 근거

사양 각 절을 따랐습니다. 이번에 확인한 원본 구간만 적습니다.

- 그룹·접힘: `src/widgets/git-panel/git-sections.ts`, `src/entities/git/git-section.ts`, `src/widgets/git-panel/commit-gate.ts`(isStagedRow/isUnstagedRow), `src/widgets/git-panel/git-panel.tsx:195-312,440-469,520-650`(로컬 map + ref, 토글마다 전체 목록 저장, 늦게 온 설정 1회 채택).
- 행·머리말: `src/features/git/status-row-item.tsx`, `git-change-group.tsx`(그룹별 액션 목록), `git-section-header.tsx`(24px, chevron 12px, 대문자 11px semibold, `border-t`, 배지), `git-section-count-badge.tsx`.
- 비저장소: `src/widgets/git-panel/git-panel-container.tsx:142-143,247-256`, 버튼 `src/shared/ui/button.tsx`(outline sm) + `src/shared/styles/global.css`(`--background`=`app.background`, `--accent`=`list.hoverBackground`, `--accent-foreground`=`list.foreground`, `--border`=`app.border`).
- 캐시·무효화: `src/entities/git/git.query.ts:140-188`(`isGitQueryScopeMutable`, current user `staleTime: Infinity`, 변경 성공 무효화), `src/app/providers/ipc-sync-provider.tsx:396-411,489-535`(status/refs = 가변 범위 전부, fs = status + 그 경로 diff·gutter).

## 3. 구현

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-app/src/git-cache.rs`(신규) | `Changes`(GitStatusChanged·GitRefsChanged·FsRescanRequired → 저장소 범위, FsChanged → 경로 묶음, 프로젝트별 병합), 범용 `GitCache<K, V>`(`observe`·`requests`·`begin`·`finish`·`invalidate`·`retain`·`cancel_fetches`), `CacheKey`(`is_mutable`·`follows_worktree`·`stale_after`), `Key::Status`/`Value::Status`, 공급 `status_result` |
| `native/taide-native-app/src/git-scm-model.rs`(신규) | `build_sections`, `list_rows`(머리말+펼친 그룹 행), `Collapsed`(id 목록 ↔ 플래그, 모르는 id 무시, 렌더 순서로 쓰기), 상태 글자·색 토큰, 행 tooltip, 경로 분리 |
| `native/taide-native-app/src/git-scm.rs`(신규) | `Appearance`(테마 토큰), slot별 `Panels`(마운트 때 설정을 읽고 보이는 동안 로컬 map 유지, 다음 프레임에 안 보이면 해제), 비저장소 화면·변경 목록(`ScrollArea::show_rows`)·머리말·행 그리기 |
| `native/taide-native-app/src/domains/git.rs` | `Command::{Read, Init, PatchSettings}`, `Reply::{Read, Mutated}`, `MutationOp`, `InitPort`(주입 포트, 기본 `git_actions::git_init`), `State` 필드, 모든 조회·init은 `spawn_transient`로 worker를 막지 않음, `apply_git_intent`(ShowView/ShowAuxiliaryView/Shown/ToggleSection/Init), `tick_git_cache`(무효화 소비·열린 프로젝트 정리·외형 갱신·요청 상한 16·큐 가득이면 다음 프레임), `show_git_view` |
| `native/taide-native-ui/src/intents/git.rs` | `Section`, `Intent` 5변형 |
| `native/taide-native-ui/src/command-registry.rs` | `Run::ShowGit`, `"git"` 키맵, `("view.git", Run::ShowGit)`, 검사 수 갱신 |
| `native/taide-native-app/src/shell_keymap.rs` | `runs_without_focused_project`에 `ShowGit`, 메인 focus slot `ShowView`·보조 창 `ShowAuxiliaryView` |
| 공용(영역 블록 추가만) | `application.rs`: PaintSink `git_cache` 기록·생성자, `git_state` 리터럴 필드, `AppSurfaces.git`, intent 라우팅 한 줄에 `&context, &snapshot` 인자. `lib.rs`: 모듈 3개. `navigation-icons.rs`: `Plus`·`Minus`·`Undo2`(+ `resources/icons/{plus,minus,undo-2}.svg`, Lucide 원본 path) |
| 다른 영역 파일(최소) | `domains/explorer_search.rs` `explorer()`: 사용하지 않던 intent 인자에 이름을 붙이고 `View::Git`이면 `show_git_view` 호출 3줄. 사이드바 렌더가 단계 0에서 이 파일로 옮겨져 다른 위치가 없습니다 |

설계 판단(원본에 가장 가까운 안):

- 관찰자 재부착: TanStack은 관찰 중인 쿼리를 시간만으로 다시 읽지 않고 새 관찰자가 붙을 때 staleTime을 봅니다. 사양 2.0.3의 "(값 없음 또는 stale 또는 60초 경과)"를 매 프레임 적용하면 보이는 동안 60초마다 다시 읽게 되므로, "직전 요청 라운드에 관찰되지 않았다가 다시 관찰된 항목"만 경과 시간을 봅니다.
- 접힘의 `adopted` 플래그: native는 설정이 App 생성 전에 메모리에 있어 늦게 도착하는 경우가 없습니다. 패널은 마운트(첫 Shown) 때 설정을 읽고, 이후 토글은 로컬 map을 먼저 바꾸고 전체 목록을 저장합니다. 원본과 같은 결과입니다.
- 패널 수명: 렌더는 `&self`(오류 경계 안)라 상태를 바꾸지 않고 `Intent::Shown`을 냅니다. 다음 tick에서 그 프레임에 보이지 않은 slot의 패널을 해제합니다(원본 언마운트). 같은 intent가 캐시 관찰입니다.
- 보기 전환: `apply_explorer_search_intent(ShowExplorer/ShowAuxiliaryExplorer)`로 Files 동작(접힘 해제·보조 창 explorer 열기)을 그대로 쓰고 같은 처리 안에서 보기를 Git으로 바꿉니다. 사양의 "`show_explorer_view`로 일반화"와 결과가 같고 explorer_search 파일을 고치지 않습니다.
- 행 액션 버튼은 hover/focus 때만 위젯을 만들고(원본 `hidden group-hover:flex`), 누름 처리는 git-20(G2)에서 연결합니다.
- Stash·Graph 섹션 건수는 G3·G6의 조회가 생길 때까지 0을 넘깁니다(모델은 다섯 섹션 모두 처리).

## 4. 사양·대응표와 실제 코드 차이

- **rename 행**: `taide_git::service::collect_status_rows`는 `git2::StatusEntry::path()`를 쓰는데, git2 0.21은 `head_to_index`/`index_to_workdir`의 `old_file` 경로를 돌려줍니다(`git2-0.21.0/src/status.rs:320-329`). 그래서 `git mv b.txt c.txt`의 행 path는 `b.txt`이고 `orig_path`는 같은 값이라 걸러져 `None`입니다. 원본 TS도 같은 백엔드를 쓰므로 원본 화면도 `b.txt`·R·tooltip `b.txt`입니다. 사양의 "[A] rename fixture(git mv) tooltip `orig → path`"는 실제로 나오지 않아, rename tooltip 문자열은 [C]로, 실제 App tooltip은 하위 디렉터리 파일(`docs/d.txt` → tooltip `docs/d.txt`, 행 `d.txt docs`)로 검사했습니다. 백엔드 변경은 범위 밖입니다.
- **git-37**: 대응표 missing → 실제로는 shell-61·shell-6이 구현. 이번 묶음에서 비저장소 숨김과 초기화 뒤 branch 표시를 실제 App으로 단언했습니다.
- **git-38**: D1 A의 `APP/git-status-supply.rs`가 이 worktree에 없습니다. 공급(`GitCache::status_result`)만 만들었고 탐색기 행 렌더·관찰은 병합 뒤 연결합니다(5절).
- **git-1의 세 번째 진입(`git.createTagOnHead`)**: 명령 활성·태그 대화상자는 git-36(G6) 범위입니다. Git 보기를 여는 부분은 같은 `Intent::ShowView`를 씁니다.
- **겹치는 행**: 사양 1.3대로 shell-21의 Git 부분을 git-1 근거로 갱신했습니다(남은 범위가 git 키뿐이어서 complete). commands-ui-14·62는 commands-ui lane이 행을 닫습니다. shell-59의 "Git 패널 본체·⌃⇧G/`view.git`" 남은 범위는 git-1·2·16·19로 충족됐지만, 같은 행의 검색 부분을 explorer-search lane이 고칠 예정이라 병합 때 두 근거를 한 번에 반영합니다.
- **기존 검사 갱신**: `application-explorer-views-tests.rs`의 "검색과 git은 빈 본문" 단언에서 Git을 빼고 이름을 고쳤습니다. `application-shell-dialogs-tests.rs`의 "git 키는 통과" 구간을 지우고 이름을 고쳤습니다(⌃⇧G 동작은 git 검사로 이동). `application-auxiliary-window-tests.rs`의 보조 창 helper 4개와 `Viewports`(필드 2개)를 `pub(super)`로 열어 git 검사가 재사용합니다. `shell_keymap.rs`·`command-dispatch.rs`·UI `command-registry.rs` 검사의 `"git"` 미지원 단언을 지원 단언으로 바꿨습니다.

## 5. 검증

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir ../targets/git` | 통과(기존 경고 `is_applying`, vendor wry-preview 경고만) |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets ...` | 통과(기존 경고 3개: `is_applying`, `zen.rs:543` mut, `fixture_with_emmet_peek`) |
| `cargo check --quiet --manifest-path native/taide-native-ui/Cargo.toml --all-targets --features native-host,inspection ...` | 통과(기존 경고 `tests/workbench.rs:1816` mut만) |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib ...` | 통과(remote-web 기존 dead code 경고만) |
| `cargo test ... taide-native-app --lib -- git_cache git_scm_model shell_keymap command_dispatch title_branch --test-threads=2` | 31 통과 |
| `cargo test ... taide-native-app --lib -- domains::git::base_tests --test-threads=2` | 5개 중 4 통과·1 실패(rename 기대값, 4절) → 검사 수정 뒤 해당 1개 재실행 통과 |
| `cargo test ... taide-native-app --lib -- application::explorer_views_tests application::shell_dialogs_tests domains::git --test-threads=2` | 10 통과 |
| `cargo test ... taide-native-app --lib -- application::auxiliary_window_tests title_branch explorer_search symbol_sidebar --test-threads=2` | 15 통과·1 실패(`native-lsp-mock` 미빌드) → mock 빌드 뒤 단독 재실행 통과 |
| `cargo test ... taide-native-app --lib -- palette keybinding keymap app_menu --test-threads=2` | 67 통과 |
| `cargo test ... taide-native-ui --features native-host,inspection --lib` | 167 통과 |
| `cargo test ... taide-native-ui --lib -- command_registry`(기능 없음) | 15 통과 |
| `cargo fmt --manifest-path <app·ui>/Cargo.toml -- --check` | 통과(app은 신규 파일만 `cargo fmt` 적용, `git status`에 다른 파일 변화 없음) |
| 외형 색 저장을 배열로 바꾼 뒤 `cargo test ... taide-native-app --lib -- domains::git git_cache git_scm_model --test-threads=2`, `cargo check --lib`, `cargo fmt --check` | 17 통과, 기존 경고만, 통과 |
| `git -C <worktree> status --short` | remote-web·`Cargo.toml`·`Cargo.lock` 변경 없음 |

자동 검사 내용:

- `[C]` `git-cache-tests.rs` 6개: 이벤트 기록·병합, 저장소 무효화(불변 범위 제외·current user 포함)와 fs 무효화(status + 그 경로만), 읽는 중 무효화 경쟁(값 쓰고 한 번 더)·오류도 값·이전 값 유지, 재부착 관찰자만 stale 시간 적용·blame overlay 0초·무기한 항목·10분 폐기, 요청 상한 16과 시작 못 한 요청 재요청, 재연결·닫힌 프로젝트.
- `[C]` `git-scm-model-tests.rs` 6개: 원본 `git-sections.test.ts`·`git-section.test.ts` 사례(그룹 분리, 두 그룹 동시, 충돌 제외, 빈 섹션 숨김, 접혀도 건수, 기본 접힘, 모르는 id 무시, 렌더 순서), 목록 평탄화, 글자·색 7종, tooltip·경로 분리.
- `[C]` `shell_keymap`(메인 focus slot `ShowView`, 보조 창 `ShowAuxiliaryView`, focus slot 없으면 None, 프로젝트 없이 키 소비), `command_dispatch`(무프로젝트 허용 목록에 `git`), UI `command_registry`(`keymap_run("git")`, `view.git` 실행값, 수).
- `[A]` `application-git-base-tests.rs`(main.rs가 쓰는 `NativeApplication`과 frame 경로, 테스트 쪽 git CLI는 `GIT_CONFIG_GLOBAL=/dev/null` fixture):
  1. 메인 2 slot·접힘에서 ⌃⇧G → focus slot만 펼침·Git·비저장소 안내, 다른 slot 유지, 팔레트 명령 채널의 `view.git` → Git.
  2. 보조 창 viewport에 ⌘B로 explorer를 닫고 ⌃⇧G → 그 창 explorer 열림·Git·본문 표시, 메인 slot 유지.
  3. 비저장소: 안내·버튼, 제목줄 branch 없음, 실패 포트 → `git.initFailed`, 대기 포트 → 버튼 비활성(accesskit disabled), 해제 → `git.initSuccess`·`git.noChanges`·제목줄 `main`.
  4. 저장소: 같은 파일 Staged·Changes 두 행, 대문자 머리말·expanded, 배지 2·3, 글자 색(M·R·U), hover 시 tooltip `docs/d.txt`·액션 3개·글자 숨김, 벗어나면 복귀, 테스트 쪽 `git add` + `GitStatusChanged` → Changes 사라짐·`A`, 파일 생성 + `FsChanged` → 새 행.
  5. 접힘: Changes 토글 → 행 숨김·settings.json `["changes","stashes"]`, Staged·Changes 연속 토글 → `["staged","stashes"]`, App 종료 뒤 같은 데이터 디렉터리를 `load_settings`로 읽은 새 App → Staged 접힘·Changes 펼침 복원.

실행하지 않은 것: 필터 없는 앱 전체 테스트(메인이 병합 때), 실제 `git init`(허락 6), 실제 watcher가 `.git` 변경을 이벤트로 바꾸는 경로(이벤트 발행은 기존 `projects`·`event-relay` 검사 범위, 이번 검사는 같은 `services.events` sink로 진입).

## 6. 남은 범위

- git-38: 병합 뒤 explorer-search lane의 `git-status-supply.rs` 공개 API(`status`, `generation`)를 이 캐시를 읽는 얇은 함수로 바꾸고, 탐색기가 보이는 프로젝트를 `cache.observe(project, Key::Status, now)`로 관찰하게 연결합니다. 그 lane의 `GitStatus(project)` host 명령은 `Command::Read(Key::Status)`로 대체합니다. 제목줄(`title_branch`)도 그때 캐시 status로 옮깁니다(사양 2.0.3).
- 섹션 접힘 저장 실패의 `settings.saveFailed` toast는 기존 `HostReply::SettingsFailed` 경로를 그대로 쓰며 이번에 실패 주입 검사는 하지 않았습니다.
- G2 이후: 행 선택 배경·클릭(git-24·25), 액션 실행(git-20), 머리말 액션(git-21), sticky·roving(git-17·26), Stash·Graph 섹션(git-9·30).

## 7. 리뷰 수정(2026-11-03)

- git-64 근거 보강: 사양의 "`.git` 변경 → 이벤트 → 패널 갱신"은 한 검사로 잇지 않고 두 구간으로 확인합니다. (1) 실제 OS watcher가 `.git/index` 변경을 `GitStatusChanged`로 `services.events`에 발행하는 구간은 기존 `tests/projects.rs` `실제_os_watcher는_파일과_git_변경을_전달하고_종료까지_join한다`를 이 lane에서 실행해 확인했습니다(`cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --test projects --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/git -- --test-threads=2 실제_os_watcher는_파일과_git_변경을_전달하고` → 1 passed, 임시 디렉터리만 감시). (2) `services.events` → git 캐시 → 패널 재조회 구간은 5절 App 검사 4(같은 sink에 이벤트 주입)입니다. 대응표 행에 이 두 구간을 적었습니다.
- git-19: rename 행 tooltip은 백엔드가 rename 행에 원래 경로를 주지 않아 실제 App에서는 path만 보이며(4절, 원본 동일), 사양의 rename fixture App 검사는 하지 못했습니다. 대응표 행에 한계를 적었습니다.
