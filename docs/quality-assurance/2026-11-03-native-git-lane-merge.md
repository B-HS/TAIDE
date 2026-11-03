# native git lane 병합 (2026-11-03)

worktree `TAIDE-lanes/git`(브랜치 `lane/git`)에서 `git merge to_rust_native`를 마친 기록입니다. to_rust_native에는 explorer-search·shell·terminal-agent·editor·preview·settings lane이 먼저 병합돼 있었고, lane/git은 Git SCM 패널·`git-cache.rs`·diff·로그·Claude diff·blame·충돌 해결(G1~G8)과 `DocumentKey::Virtual`을 더했습니다. 소유 판단은 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`와 `docs/research/2026-10-28-lane-git-spec.md`(1.2, 2.0.3, git-7·38·49)를 따랐습니다.

## 1. 충돌 해결

| 파일 | 판단 |
| --- | --- |
| `APP/shell_keymap.rs` | 프로젝트 없이 처리하는 분기에 git lane의 `Run::Git(RevertHead)`와 terminal-agent lane의 `Run::Cli`·`OpenWelcomeTab`(무프로젝트 안내)을 함께 둡니다. 프로젝트 분기에는 git lane의 `ShowGit`(⌃⇧G, 보조 창은 그 창 explorer)·`CreateTagOnHead`·`OpenFileHistory`·`ToggleBlame`/`StageSelection`과 explorer-search lane의 `OpenSearch { replace }`를 함께 둡니다. Git 보기·⌃⇧G는 git lane 소유(사양 1.2)라 shell 쪽 구현은 없습니다 |
| `UI/command-registry.rs` | `Run`에 `ShowGit`·`Git(GitRun)`과 `OpenSearch`를 함께 둡니다. 실행 id(`git.*`·`monaco.taide.*` Git 항목과 `cli.*`), 키맵 `git`·`search`·`search-replace`를 합쳤고 표 크기 상수는 양쪽 합(`NATIVE_KEYMAP_COUNT` 38+찾기, `NATIVE_COMMANDS` 39)으로 맞췄습니다. 검사도 두 lane 단언을 모두 남기고 미지원 목록에서 이제 지원되는 `git`·`search`·`search-replace`를 뺐습니다 |
| `APP/command-dispatch.rs` | `KEYMAP_ACTIONS` 39(`git` + `find`/`search`/`search-replace`). 미지원 목록에서 `git`·`git.revertHead`를 빼고, `git.revertHead`·`git.createTagOnHead`는 `task.runTask`처럼 프로젝트가 있어야 받습니다. terminal-agent lane의 `cli.*`·`view.welcome`·`tab.moveToNewWindow`·`tab.moveToMainWindow` 단언은 그대로입니다 |
| `APP/explorer.rs` | `Action`에 explorer-search lane의 `OpenInTerminal`·`FindInFolder`와 git lane의 `SelectForCompare`·`CompareWithSelected`·`FileHistory`를 함께 둡니다. `Explorer`는 explorer-search의 `auto_revealed`(옛 `request_mount` 제거)와 git의 `compare_source`를 가집니다. 파일 행 메뉴 순서는 원본 `file-tree-context-menu.tsx:112-168`대로 옆에서 열기 → 연결 프로그램 → 브라우저 → 비교 위해 선택 → 선택 항목과 비교 → 파일 이력 → 구분선 → Finder → 터미널 → 폴더에서 찾기입니다(D2·D3 A: 메뉴는 git lane, 자리는 explorer-search lane) |
| `APP/domains/explorer_search.rs` | 사이드바 보기 분기에서 Search는 explorer-search의 검색 패널, Git은 git lane의 `show_git_view`를 그리고 각각 돌아갑니다. 보조 창 pane 계산은 explorer-search의 `this_window_pane`을 함께 씁니다. 비교 선택은 mount별 explorer(`Mounts::get_mut(project, &slot)`)에 두고, 비교 탭은 `compare_tab(.., window_pane)`으로 그 창의 focus pane에 엽니다(메인 창은 `None`, 다른 탐색기 메뉴와 같은 규칙). `FileHistory`는 `Intent::OpenFileHistory`로 넘깁니다 |
| `APP/navigation-icons.rs` | 두 lane 아이콘을 합쳤습니다. git의 `LoaderCircle`과 explorer-search의 `Loader`는 같은 `loader-circle.svg`라 `Icon::Loader` 하나로 합치고 `git-scm.rs` 스피너를 그 이름으로 바꿨습니다. `Check`도 하나만 둡니다 |
| `APP/application-explorer-views-tests.rs` | explorer-search의 검색 본문 단언(`search.liveSearchHint`)을 남기고, git lane이 Git 본문을 그리므로 explorer-search의 "Git 빈 본문" 단언 대신 Git 보기가 비저장소 안내 `git.notARepository`를 그릴 때까지 기다리게 했습니다. 이름도 `..._검색은_패널을_git은_scm_본문을_보인다`로 바꿨습니다 |
| 기능표 shell-89·explorer-search-22·23 | 양쪽 근거를 합쳤습니다(아래 5절) |

## 2. 의미 충돌

- `DocumentKey::Virtual`: main 쪽 전수 match와 "파일 아님" 필터에 가상 문서를 넣었습니다. editor lane 내장 서비스의 tick·적용 필터(`editor-css.rs` `dialect`, `editor-html.rs` `is_html`, `editor-completion-json.rs` `applies`, `editor-json-formatting.rs` `is_eligible`, `editor-selection-ranges.rs` `is_json`), 문제 이동(`editor-problems.rs`), IDE 진단(`ide-sync.rs`), peek 모델(`peek-models.rs`)이 `AppFile | Virtual`을 제외합니다. 컴파일러가 잡는 전수 match는 이미 lane/git에서 처리돼 있었습니다(`diagnostics.rs`, `editor-locations.rs`, `persistence.rs`, `domains/editor.rs`, `editor-json*.rs`).
- 구조 변경 추종: `PaintSink`에서 `title_branches` 필드를 빼고 `git_cache`·`providers`를 둡니다(terminal-agent 검사 fixture도 같이). explorer `Mounts` API(`get_mut(project, slot)`, `get(slot)`, `of_project`)로 비교 선택·검사를 바꿨습니다. terminal-agent lane의 `LayoutActions.wait_markers` 필드를 git claude 검사 fixture에 넣었습니다. ToastLayers·settings Events는 lane/git 코드가 직접 쓰지 않아 컴파일 확인만 했습니다.

## 3. lane 사이 연결

| 항목 | 결과 |
| --- | --- |
| (a) git-38 / explorer-search-7 | explorer-search lane은 `git-status-supply.rs` 없이 주입점(`GitDecorations::set`·`mounted_projects`)만 두었고 별도 status 조회는 없었습니다. `tick_git_cache`가 `mounted_projects()`마다 `cache.observe(project, Key::Status)` 후 `status_result`의 성공 값을 `set`에 넣습니다(오류·비저장소는 `None`으로 장식 제거). 검사: `실제_app_탐색기_git_장식은_git_cache_status로_파일_배지와_상위_폴더를_그리고_git_변경_뒤_갱신한다` |
| (b) git-7 | `apply_git_reply`의 `MutationOp::Pull`·`Push` 응답이 `notify_git_remote` → shell `notify_event`를 부릅니다. 성공: 범주 `GitRemote`, 본문 `[성공 문구, 캐시된 branch]`. 실패: 범주 `Error`, 본문 `[실패 문구, describe_error]`. ` · `로 잇고 제목은 프로젝트 표시명(없으면 문구 키). 검사: 가짜 platform(`notifications_tests::FakePlatform`, 가시성만 `pub(in crate::application)`으로 넓힘)으로 `실제_app_sync의_pull과_push는_성공과_실패를_프로젝트_제목의_os_알림으로_보낸다` — pull·push 성공 2건, push 실패(bare 원격 `objects` 읽기 전용), pull 실패(원격 경로 이동)의 제목·본문 |
| (c) git-49 | 사양 git-49 설계대로 shell `provider_events::Inbox`에 `Event::IdeDiff`(요청, 요청 id가 있는 close)를 추가하고 `consume_provider_events`가 `accept_claude_diff_event`를 부릅니다. lane 시기 대체 경로(`git_cache::Changes.ide`, `git_claude_diff::Inbox`, tick 안 `consume_ide_diff_events`)는 지웠습니다. 수신함 단위 검사는 `provider-events.rs`로 옮겼습니다. 상한(256)은 shell 사양의 "상한 같은 대기열"을 따릅니다 |
| (d) 제목줄 branch | 사양 2.0.3 마지막 항목(공급 흡수가 없으면 캐시 status를 읽게 바꿈)에 따라 `title-branch.rs`·검사·`HostCommand::Git(GitBranch)`·`Reply::GitBranch`를 지우고, 제목줄이 보이는 프로젝트(메인 focus·보조 창)를 `Key::Status`로 관찰해 `State::title_branch`(캐시 status의 branch, 오류면 `None`)로 읽습니다. shell-61·shell-6·git-37 App 검사를 이 경로로 재실행했습니다 |

## 4. 병합 검증 중 고친 결함

### 4.1 파일 편집기의 Git 관찰 intent가 포커스 slot을 빼앗음(git lane 결함)

- 증상: to_rust_native의 `실제_app_팔레트_view_explorer는_포커스_슬롯만_접힘을_풀고_파일_보기로_바꾼다`가 병합 트리에서 실패(부하가 있을 때 거의 매번). 디버그 출력으로 두 slot의 focus가 몇 프레임마다 번갈아 바뀌는 것을 확인했습니다.
- 원인: `show_git_editor`가 그려진 모든 파일 편집기마다 매 프레임 `ShellIntent::Git(EditorShown)`를 냈고, `UI/shell.rs`는 focus되지 않은 slot이 그리는 동안 intent가 하나라도 나오면 사용자 활성화로 보고 `FocusSlot`을 넣습니다(`widget_activated`, 두 브랜치 모두 같은 규칙). 그래서 두 slot에 파일 편집기가 있으면 실제 앱에서도 focus가 계속 뒤바뀝니다. lane/git 검사에는 두 slot에 편집기를 둔 경우가 없어 드러나지 않았습니다.
- 수정: 관찰은 사용자 의도가 아니므로 intent 경로에서 빼 프레임 출력으로 옮겼습니다. `AppSurfaces.git_editors: Vec<domains::git::ShownEditor>`에 모으고, intent를 적용하던 같은 자리(같은 `is_enabled` 블록, intent 루프 직전)에서 `observe_git_editors`가 적용합니다. `UI/intents/git.rs`의 `Intent::EditorShown` 변형은 지웠습니다.
- 검사: `실제_app_두_slot의_파일_편집기는_매_프레임_git_관찰을_해도_포커스_slot을_바꾸지_않는다`(두 저장소·두 slot 파일 편집기, 두 편집기 모두 Git 관찰 유지, 12프레임 동안 focus 불변). 기존 팔레트 검사도 통과합니다.

### 4.2 공유 캐시 때문에 바뀐 검사 전제(동기화만 수정, 기대값 유지)

- `실제_app_브랜치_스위처는_..._toast로_알린다`: "첫 status 전에는 스위처 비활성·`git.noRepositoryLabel`" 단언은 Git 보기가 status를 처음 관찰하는 표면이라는 전제였습니다. 병합 뒤 제목줄·탐색기 장식이 같은 캐시를 먼저 채우므로(원본 TanStack Query 공유 키와 같은 동작) 그 전제가 사라졌습니다. 첫 프레임 전에 그 slot 보기를 Git으로 두고 시작해 Git 보기가 첫 관찰자가 되게 했습니다.
- `실제_app_브랜치가_하나도_없으면_목록은_찾을_수_없음을_보인다`: status가 캐시에 있어 Git 보기의 첫 프레임에 바로 클릭하게 되던 것을, 다른 검사처럼 `open_picker`(포인터 치우기·안정 프레임 뒤 클릭)로 엽니다.
- `실제_auto_tab_토글은_편집기_자동완성_요청_경로를_켠다`(settings lane): 토글이 AI 토큰 상태 도착 전 비활성(`add_enabled_ui(is_selected_configured)`)인데 검사는 존재만 기다리고 눌렀습니다. 디버그 출력으로 클릭 프레임에 checkbox `is_disabled = true`, 토큰 상태 `Pending`을 확인했습니다. 활성 상태를 기다린 뒤 누르게 바꿨습니다. 제품 코드는 바꾸지 않았습니다.

### 4.3 추가 검사

- `실제_app_css와_html_파일_diff_탭의_가상_문서는_내장_구조_서비스_대상이_아니고_오류_toast가_없다`: `.css`·`.html` 작업 트리 diff 탭마다 가상 문서 2개, 1초 동안 오류 toast 0, 내장 구조 서비스(`editor_json`, CSS 접기·HTML 모델 포함) 문서 목록에 가상 문서 없음.

## 5. 기능표 갱신

- 충돌 3행: shell-89(네 본문 연결, 가상 문서 제외 근거에 `.css`·`.html` App 검사 추가, 남은 것 git-50 revert 화살표), explorer-search-22(complete, mount별 선택·그 창 pane), explorer-search-23(complete).
- 연결로 바뀐 행: git-7(complete), git-38(complete), explorer-search-7(complete), git-49(complete, 수신 경로 정정), git-37·shell-61·shell-6(캐시 경로 근거), terminal-agent-68(unwired → partial), terminal-agent-69(정정 근거), settings-23(git 범주 연결), settings-28(diff 숨김 소비자), settings-80(IDE diff 자동 열기 소비자).
- shell 사양 3.3 묶음 3 행(shell-59·78·92·93): 구현 잔여가 없다는 근거만 넣고 상태 판정은 wave 3 몫으로 남겼습니다(git 사양 1.2 "shell 행은 근거만 갱신").

## 6. 검증

공통 인자: `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/git`, manifest는 `/Users/hyunseokbyun/development/TAIDE-lanes/git/native/<crate>/Cargo.toml`. `native-lsp-mock` 예제는 이 target에 이미 있어 다시 빌드하지 않았습니다.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --all-targets`(app) | 성공. 경고는 기존 vendor·`editor-code-actions.rs` `is_applying`·`zen.rs`·`application-inlay-hints-tests.rs`(두 브랜치에 같은 코드) |
| `cargo check --quiet --all-targets --features native-host,inspection`(ui) | 성공. 경고는 기존 `tests/workbench.rs:1880` |
| `cargo check --quiet --target wasm32-unknown-unknown --lib`(remote-web) | 성공(병합 직후와 4.1 수정 뒤 두 번). 경고는 기존 `browser-editor.rs`·`browser-application.rs` 미사용 필드·메서드 |
| 앱 lib `-- git explorer shell json css html provider_events ide_sync notifications auxiliary_window search --test-threads=2`(1차) | 350 passed, 3 failed: 팔레트 `view.explorer`(4.1), 브랜치 스위처 2건(4.2) |
| 같은 필터 + `commands`(4.1·4.2 수정 뒤) | 359 passed |
| 앱 lib 전체 `-- --test-threads=2`(1차) | 1230 passed, 2 failed: `실제_auto_tab_토글은`(4.2, 단독 재실행도 실패) · `editor_fonts::..._가변_face는_공유하지_않는다`(기존 실패, PROCESS 2026-11-03 기록과 같음) |
| 앱 lib 전체 `-- --test-threads=2`(최종) | 1231 passed, 1 failed: `editor_fonts` 기존 실패 1건뿐 |
| UI `--test workbench` | 46 passed |
| UI `--lib` | 226 passed |
| `cargo fmt -- --check`(app, ui) | 차이 없음 |

실행하지 않은 것: 앱 통합 테스트 전체(`tests/*.rs`, 특히 실제 휴지통을 쓰는 `tests/explorer-delete.rs`), GUI 실행, 실제 OS 알림, 네트워크. editor·syntax·terminal crate는 이번 병합 작업에서 바꾸지 않아 다시 돌리지 않았습니다.

## 7. 남은 문제

- `editor_fonts` 단위 검사 1건은 병합 전부터 있던 실패입니다(글꼴 face 목록 차이).
- git-50 ClaudeDiff 수정 쪽 revert 화살표, git-52 보조 창 이동 뒤 응답 없음·보조 창 Accept App 검사, git-42 `editorDiffShowMoves`는 lane 남은 범위 그대로입니다.
- settings-23 범주 7종 묶음 App 검사와 shell 묶음 3 행 판정은 메인 통합·wave 3 몫입니다.
