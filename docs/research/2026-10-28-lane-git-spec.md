# lane git 사양 — Git 영역 남은 64행 (git-1 ~ git-64)

> 작성일: 2026-10-28 · 기준 커밋 `72065136`(branch `to_rust_native`) · 운영 방식 `docs/memory/2026-10-28-native-lane-operations.md`
> 이후 구현 작업자는 이 문서와 여기서 인용한 파일 구간만 읽습니다. 원본 TS를 처음부터 다시 조사하지 않습니다.
> `application.rs`·`host.rs`(native-app), `commands.rs`·`shell.rs`(native-ui)는 단계 0에서 영역 모듈로 옮겨지는 중이라 줄 번호 없이 심볼 이름으로만 인용합니다. `docs/research/2026-10-28-native-lane-seams.md`는 작성 시점에 없었습니다. 이름은 이미 옮겨진 영역 모듈(`native/taide-native-app/src/domains/commands.rs`·`domains/shell.rs`, `native/taide-native-ui/src/intents/commands.rs`)의 형태를 따랐고, seams 문서가 생기면 그 문서의 이름이 우선합니다.
> 표기: `TS/` = `src/`, `APP/` = `native/taide-native-app/src/`, `UI/` = `native/taide-native-ui/src/`, `ED/` = `native/taide-native-editor/src/`, `CR/` = `crates/`. 검사 표기: `[C]` crate·순수 모듈 단위, `[U]` UI egui 하네스, `[A]` 앱 lib 실제 App.

---

## 1. 요약

### 1.1 규모와 상태

| 구분 | 수 | ID |
| --- | --: | --- |
| 대상 | 64 | git-1 ~ git-64 (git-65 locale은 complete, 대상 외) |
| missing | 29 | 1, 3, 11, 12, 16, 17, 18, 19, 22, 23, 24, 25, 26, 28, 29, 30, 31, 37, 38, 40, 41, 42, 46, 48, 50, 54, 58, 61, 63 |
| unwired | 34 | 2, 4, 5, 6, 7, 8, 9, 10, 13, 14, 15, 20, 21, 27, 32, 33, 34, 35, 36, 39, 43, 44, 45, 47, 49, 51, 52, 53, 55, 56, 57, 59, 60, 62 |
| partial | 1 | 64 |

백엔드는 거의 다 있습니다. `CR/taide-runtime/src/git_actions.rs`가 원본 Tauri 명령 41개와 같은 함수를 모두 갖고 있고(원격 경로 `APP/remote-git.rs`가 이미 그대로 부름), IDE diff 응답은 `CR/taide-runtime/src/ide_actions.rs::ide_resolve_diff`, AI 커밋 메시지는 `CR/taide-runtime/src/ai_actions.rs::ai_commit_message`·`ai_request_cancel`입니다. 빠진 것은 로컬 UI 캐시·표면·요청 경로·diff 위젯입니다. 새 백엔드 로직은 diff 줄 계산 함수 하나(`taide_git::service::diff_hunks`)뿐입니다.

조사 중 확인한 대응표와 실제 코드의 차이(구현 첫 묶음에서 대응표를 정정):

- **git-37(타이틀바 branch)**: shell-61·shell-6(complete)이 같은 기능을 이미 연결했습니다(`APP/title-branch.rs`, `HostCommand::GitBranch`, 오류면 `None`). 비저장소 숨김 단언 1건만 남습니다.
- **git-64**: `.git` watcher attach(`APP/projects.rs`의 `restore_watchers`, `git_watchers`)와 `GitStatusChanged`·`GitRefsChanged` 발행, 백엔드 status 캐시 무효화(`APP/event-relay.rs:12-40` `GitEvents::record`, boot에서 `activate`)는 있습니다. 로컬 UI 캐시·재조회 소비자만 없습니다.
- **git-52**: 탭 닫힘 시 `TabClosed` 응답은 `APP/tabs.rs`의 ClaudeDiff 닫기 분기(`take_pending_diff` → `IdeDiffOutcome::TabClosed`)가 이미 처리합니다. 남은 것은 표면(git-50/51), 창 이동 시 무응답, `IdeCloseTabRequested`의 대기 목록 정리입니다.
- **terminal-agent-69(ideAutoOpenDiff)는 complete로 표시돼 있으나** native에서 `AppEvent::IdeDiffRequested`를 로컬 UI가 소비하는 곳이 없습니다(`rg IdeDiffRequested` 결과가 발행·remote relay·테스트뿐). git-49에서 연결하고 그 행도 정정합니다.

### 1.2 다른 lane 의존 (먼저 필요한 것)

| 선행 | 이 lane이 쓰는 것 | 없을 때 |
| --- | --- | --- |
| 단계 0 lane 경계(필수) | `domains/<영역>.rs` 형태(`Command`·`Reply`·`State`·`dispatch`, `impl NativeApplication { apply_<영역>_reply, apply_<영역>_intent }`), `HostCommand::<영역>(..)`·`HostReply::<영역>(..)`, `ShellIntent::<영역>(intents::<영역>::Intent)` | 시작 불가. 경계가 끝난 HEAD에서 분기 |
| wave 1 editor+preview lane | `UI/editor_surface.rs`·`ED/*`에 git-40~43·56~63이 추가만 합니다. 같은 파일을 wave 1에서 고치므로 그 병합 뒤 시작 | 병합 충돌 |
| wave 1 explorer-search lane(사양 4장 D1~D3 추천안 기준) | D1 A: 프로젝트별 status 공급 `APP/git-status-supply.rs`(공개 API `status(&ProjectId) -> Option<&GitStatus>`, `generation(&ProjectId)`)와 탐색기 Git 장식(explorer-search-7 = git-38)을 그 lane이 만듭니다. D2 A: 탐색기 "파일 히스토리" 메뉴(explorer-search-23)는 git lane으로 이관. D3 A: "비교 위해 선택/선택 항목과 비교" 메뉴·`TabKind::Diff { compare_with }` 탭 생성은 그 lane, 본문은 git-45 | D1이 다른 안이면 git-38 구현과 공급 설계를 2.0.3의 대안대로 바꿈 |
| wave 1 shell lane | `APP/provider-events.rs`(`provider_events::Inbox`)와 `domains::shell::consume_provider_events`, `APP/ide-sync.rs`(`crate::ide_sync`, IDE save·진단). git-49/52는 이 수신함·모듈에 diff·close 분기를 추가만 합니다. shell-21(⌃⇧G)·shell-59(Git 보기)·shell-78(탭 메뉴 Git 항목)은 git-1·48·55와 같은 기능이라 이 lane이 구현하고 shell 행은 근거만 갱신 | 수신함이 없으면 git lane이 `IdeDiffRequested`를 자체 `Changes`로 기록(2.0.3과 같은 방식) |
| wave 1 terminal-agent lane | terminal-agent-68(Claude diff 탭) = git-49~52. shell 사양 결정 A에 따라 diff 부분은 git lane이 구현하고 68·69를 함께 갱신 | 영향 없음 |
| wave 2 settings lane(동시 진행) | OS 알림 공통 진입점 `domains::settings::notify(category, project, fallback_title_key, body)`(settings 사양 settings-23 설계 3, 잘라내기·첫 전달 안내 포함). settings 사양은 diff 설정 2종·IDE diff 자동 열기 소비자를 만들지 않는다고 명시(settings-28·80) → git lane이 소비자 구현 | git-7 알림만 settings 병합 뒤로 미룸(3.1 G3 비고) |
| wave 3 commands-ui lane(후행) | 없음. commands-ui-14·36~39·62는 이 lane이 실행 경로를 만든 뒤 그 lane이 행만 닫음 | 영향 없음 |

### 1.3 겹치는 행 처리

| 다른 영역 행 | 같은 git ID | 처리 |
| --- | --- | --- |
| shell-21, shell-59, commands-ui-14, commands-ui-62 | git-1 | git-1 근거로 Git 부분 갱신 |
| shell-78 | git-48, git-55 | 탭 메뉴 "변경 열기·파일 이력" 근거 |
| shell-89 | git-39~47, 49~52 | diff/commit diff/claudeDiff 부분만. searchEditor 부분은 explorer-search lane |
| shell-92, shell-93 | git-49·52, git-64 | IDE diff·Git 이벤트 부분 근거 |
| editor-73, 74, 75, 76 | git-58~60, 61~63, 53~57, 40·47·50·63 | 같은 근거로 함께 갱신 |
| explorer-search-7, 22, 23 | git-38, git-45, git-55 | 1.2의 D1~D3 추천안 |
| terminal-agent-68, 69 | git-49~52 | 같은 근거, 69는 정정 |
| settings-28(diff 2종), settings-80(IDE diff 자동 열기), settings-23(git 알림) | git-42, git-49, git-7 | 해당 부분 근거만 추가 |
| commands-ui-36~39, 87 | git-36, 57, 55, gutter/blame 클래스 | wave 3에서 그 lane이 닫음 |

---

## 2. ID별 사양

### 2.0 공통 설계 (모든 ID가 참조)

#### 2.0.1 모듈 배치

새 파일은 native-app의 kebab-case + `#[path]` 관례를 따릅니다.

| 위치 | 내용 | 주요 ID |
| --- | --- | --- |
| `APP/domains/git.rs` | 영역 진입점: `pub enum Command`, `pub enum Reply`, `pub(crate) struct State`, `pub(crate) async fn dispatch(worker, command)`, `impl NativeApplication { apply_git_reply, apply_git_intent, tick_git, show_git_view, git_tab_content, git_editor_layers, git_editor_footer, show_git_dialogs }` | 공통 |
| `UI/intents/git.rs` | `pub enum Intent`(ShowView, ShowAuxiliaryView, OpenChanges, OpenFileHistory, ToggleBlame, StageSelection, RevertHead, CreateTagOnHead) → `ShellIntent::Git(Intent)` | 1, 36, 48, 55, 57, 60 |
| `APP/git-cache.rs` | 프로젝트별 조회 캐시·관찰·무효화(2.0.3). explorer-search의 `git-status-supply.rs`를 흡수 | 64, 공통 |
| `APP/git-scm.rs` | SCM 패널 상태·그리기(헤더, 커밋 박스, 변경 목록, stash, 그래프 pane 배치) | 1~26, 30 |
| `APP/git-scm-model.rs` | 순수: 섹션 구성, 행 평탄화, sticky, roving, 커밋 gate, 접힘 변환, 다중 선택, discard 문구 | 13, 16~18, 22, 25, 26 |
| `APP/git-commit.rs` | 커밋 초안 기억, AI 메시지(요청·취소·sanitize·요약) | 10~14 |
| `APP/git-branch.rs` | 브랜치 스위처 popup | 3~5 |
| `APP/git-graph.rs` | 레인 계산·그래프 그리기·컨텍스트 메뉴·태그 대화상자 | 27~29, 33~35 |
| `APP/git-commit-detail.rs` | 커밋 상세 패널 | 31, 32 |
| `APP/git-diff.rs` | diff 탭 상태(작업 트리·staged·compare·commit), hunk stage lane | 39~48 |
| `APP/git-claude-diff.rs` | ClaudeDiff 대기 목록·탭 표면·수락/거절 | 49~52 |
| `APP/git-history.rs` | 파일 이력 우측 패널 | 53~55 |
| `APP/git-blame.rs` | 커서 줄 footer, 전체 overlay | 56, 57 |
| `APP/git-gutter.rs` | 편집기 gutter 막대·hunk discard·선택 stage | 58~60 |
| `APP/git-conflict.rs` | 충돌 마커 파서·장식·해결/비교 대화상자 | 61~63 |
| `UI/diff-view.rs`(신규) | 공용 diff 위젯: 가상 문서 2개(side-by-side) 또는 1개(inline) + filler zone + 줄 배경 + 스크롤 동기화 + 분할선 끌기 + 숨김 영역 막대 | 40~42, 47, 50, 54, 63 |
| `UI/editor_surface.rs`(추가만) | 새 진입점 `NativeEditor::show_request_with_extras(.., extras: &SurfaceExtras)` — `SurfaceExtras { zones, line_end_texts, scroll_override }`. 기존 `show_request_with_*` 래퍼는 `SurfaceExtras::default()`를 넘김(기존 `EditorRequest` 생성 20여 곳은 바꾸지 않음). `EditorOutput`에 `lane_press: Option<usize>`(gutter lane 클릭 줄, `EditorOutput`은 이 파일 안에서만 생성) | 40, 43, 57, 59, 61 |
| `ED/document.rs`·`ED/store.rs`(추가만) | `DocumentKey::Virtual(VirtualKey { owner: TabId, side: VirtualSide })`, `EditorStore::open_virtual(key, text, language_id, read_only)`, `replace_virtual(document, text)`(undo 비움) | 40, 50, 54, 63 |
| `CR/taide-git/src/service.rs`(추가만) | `pub fn diff_hunks(original: &str, modified: &str, ignore_trim_whitespace: bool) -> AppResult<Vec<DiffHunk>>` | 40, 43, 50 |
| `UI/command-registry.rs`(추가만) | `Run::ShowGit`, `Run::Git(GitRun)`(`RevertHead`, `CreateTagOnHead`, `ToggleBlame`, `OpenFileHistory`, `StageSelection`) — `Run`이 `Copy`라 `GitRun`도 `Copy` | 1, 36, 55, 57, 60 |

테스트 파일: 순수 로직은 각 모듈 옆 `git-*-tests.rs`, 실제 App 검사는 `APP/application-git-<묶음>-tests.rs`(`domains/git.rs`에서 `#[cfg(test)] #[path]`로 등록), UI 위젯은 `native/taide-native-ui/tests/diff-view.rs`, crate 단위는 `CR/taide-git/src/service.rs`의 기존 tests 모듈.

#### 2.0.2 Host 명령·응답과 실행 규칙

- 형태: `HostCommand::Git(domains::git::Command)` → `domains::git::dispatch`, 응답 `HostReply::Git(domains::git::Reply)` → `apply_git_reply`. 단계 0의 `domains::commands`와 같은 모양입니다.
- **비동기 규칙(중요)**: host worker는 명령을 하나씩 await합니다(`host.rs`의 `connect_with_integrations` 루프, 큐 `HOST_COMMAND_CAPACITY` 64). pull/push/AI 요청이 worker를 막으면 저장·파일 열기까지 멈춥니다. 그래서 git `dispatch`는 모든 명령을 `services.tasks.spawn_transient("native-git-<op>", ..)`로 띄우고 결과를 `worker.sender`로 보낸 뒤 `repaint()`를 부르며 즉시 `None`을 돌려줍니다(`HostCommand::GitBranch` 처리와 같은 패턴). spawn 실패면 즉시 실패 응답을 돌려줍니다.
- `submit`이 큐 가득(`native host command queue is full`)으로 실패하면 그 캐시 항목은 fetching으로 바꾸지 않고 다음 프레임에 다시 냅니다. 한 프레임에 내는 조회 요청 수는 상한(예: 16)으로 자릅니다.
- 런타임 호출은 `taide_runtime::git_actions::*`를 그대로 씁니다. 컨텍스트 `GitActionContext::new(&services.state, &services.git, &services.tasks)`, 이벤트 sink `services.events.as_ref()`. `git_status`의 첫 인자는 `|| {}`(boot가 `GitEvents::activate`를 이미 부름, `HostCommand::GitBranch`와 같음). `git_diff_file`의 overlay 로더는 `taide_plugin::service::language_overlays(&taide_plugin::service::ensure_loaded(&services.plugin, &services.state.paths.plugins_dir()))`(`APP/remote-git.rs` `git_diff_file` 분기와 같음).
- 변경 계열은 runtime이 직렬화합니다(`state.begin_owned_mutation()`, push는 `push_fetch_lock`). UI가 따로 잠그지 않습니다.

| `git::Command` | runtime 함수 | `git::Reply` |
| --- | --- | --- |
| `Status(project)` | `git_status` | `Status { project, result: AppResult<GitStatus> }` |
| `Log(project)` | `git_log(skip 0, take 100)` | `Log { .. Vec<LogEntry> }` |
| `Remotes`·`Branches`·`Stashes`·`Tags`·`CurrentUser`(각 project) | `git_remotes`·`git_branches`·`git_stash_list`·`git_tags`·`git_current_user` | 같은 이름 |
| `Diff { project, path, mode, before_path }` | `git_diff_file` 후 같은 task에서 `diff_hunks` | `Diff { key, result: AppResult<(DiffSides, Vec<DiffHunk>)> }` |
| `Show { project, rev, path }` | `git_show_file` | `Show { key, result }`(`AppErrorKind::NotFound` 구분 보존) |
| `CommitFiles { project, rev }`·`FileLog { project, path }` | `git_commit_files`·`git_file_log(skip 0, take 100)` | 같은 이름 |
| `Gutter { project, path }`·`Blame { project, path, from, to }`·`ConflictSides { project, path }` | `git_gutter`·`git_blame_range`·`git_conflict_sides` | 같은 이름 |
| `ReadFile(path)` | `taide_runtime::file_actions::file_open` | `File { path, result: AppResult<OpenedFile> }` |
| `Hunks { key, original, modified, ignore_trim_whitespace }` | `taide_git::service::diff_hunks`(spawn_blocking) | `Hunks { key, result }` |
| 변경: `Init`, `Stage`, `Unstage`, `Discard`, `Commit { message, stage_all }`, `Pull`, `Push`, `BranchCreate`, `BranchCheckout`, `CheckoutRemote`, `StashPush`, `StashApply`, `StashDrop`, `DiscardHunk`, `StageHunk`, `UnstageHunk`, `StageLines`, `ResolveConflict { path, content }`, `Revert { rev }`, `TagCreate`, `TagDelete` | 같은 이름의 `git_*` | `Mutated { project, op: MutationOp, result }`(op는 호출 측 문맥: 어느 버튼·대화상자인지) |
| `AiCommitMessage { request_id, owner, recent }` | `git_diff_staged_text` → 주입 포트(2.0.7)로 `ai_commit_message` | `AiCommitMessage { request_id, result: AppResult<(StagedDiffText, AiTextResponse)> }` |
| `AiCancel { request_id, owner }` | `ai_actions::ai_request_cancel` | 없음(오류 무시) |
| `ResolveIdeDiff { request_id, outcome, content }` | `ide_actions::ide_resolve_diff(&services.state, &services.ide_save_file, &services.ide, ..)`(`APP/remote-ide.rs` 선례) | `IdeDiffResolved { request_id, result }` |
| `OpenTab { project, kind, title, target, preview }` | `layout_actions::layout_open_tab` | 실패만 `OpenTabFailed(AppError)` |
| `PatchSettings(SettingsPatch)` | `crate::host::update_control_settings(services, reconcile, patch)`(`domains::commands`의 `SetKeymapOverrides`와 같은 경로) | 기존 설정 응답 |

변경 성공 응답을 받으면 앱은 그 프로젝트의 가변 범위 캐시를 모두 stale로 만듭니다(`TS/entities/git/git.query.ts:171-188` `useGitMutation`). `ResolveConflict`는 그 파일의 열린 편집기 문서도 디스크 내용으로 정착시킵니다(git-62).

#### 2.0.3 조회 캐시·무효화 (git-64의 본체, 모든 표면이 사용)

원본: TanStack Query(`TS/app/query-client.ts` staleTime 60초, gcTime 10분, retry 0, 창 focus 재조회 없음), 키 `QUERY_KEY.GIT.*`, 불변 범위(커밋 파일 목록·커밋 blob)는 무효화 제외(`git.query.ts:155-164`). current user는 무기한(`146-153`), blame overlay는 `staleTime: 0`(`83-90`), blame 줄은 이전 값 유지(`keepPreviousData`, `60-70`).

`APP/git-cache.rs`:

- `GitCache { projects: HashMap<ProjectId, ProjectGit> }`. 항목 `Entry<T> { value: Option<AppResult<T>>, fetching: bool, stale: bool, generation: u64, fetched_at: Instant, last_observed: Instant }`.
- 범위: 가변 = status, log, remotes, branches, stashes, tags, diff(path, mode, before_path), gutter(path), blame line(path, line), blame overlay(path), conflict sides(path), file log(path). 불변 = commit files(rev), show(rev, path). current user = 무기한.
- 관찰: 표면이 그리는 프레임에 `observe(key)`. 프레임 끝 `requests()`는 관찰됐고 (값 없음 또는 stale 또는 60초 경과) 이면서 fetching이 아닌 항목만 `git::Command`로 냅니다. 10분 동안 관찰 없는 항목은 버립니다. 같은 키 동시 요청은 하나로 합칩니다.
- 응답: 요청 때의 generation과 다르면(그 사이 무효화) 값을 쓰되 stale을 유지해 한 번 더 읽습니다. 오류도 값으로 저장합니다(retry 없음). 이전 값은 다음 값이 올 때까지 유지합니다.
- 무효화 입력: `APP/git-cache.rs`의 `Changes`(`title_branch::Changes`와 같은 Mutex 기록 방식, `PaintSink::publish`에서 `record`)가 아래를 모으고 `tick_git`이 소비합니다.
  - `AppEvent::GitStatusChanged`·`GitRefsChanged { project_id }`: 가변 범위 전부(`TS/app/providers/ipc-sync-provider.tsx:399-411`).
  - `AppEvent::FsChanged { project_id, change: FsChange { paths, .. } }`: status 무조건 + 그 경로의 diff(모드 구분 없음)·gutter만(`ipc-sync-provider.tsx:489-535`, `isGitWorktreeQueryForChangedPaths` 72-76). 경로는 절대 경로로 비교.
  - `AppEvent::FsRescanRequired { project_id }`: 가변 범위 전부(`ipc-sync-provider.tsx:599-617`). 백엔드 `GitEvents::record`는 이 이벤트로 status TTL(2초, `CR/taide-git/src/store.rs`)을 비우지 않으므로 최대 2초 늦을 수 있습니다(원본도 같음, 수정 대상 아님).
  - 변경 성공 응답: 가변 범위 전부.
  - 프로젝트 닫힘(`ProjectClosed` 또는 열린 프로젝트 집합에서 사라짐): 항목 삭제.
- status 공급 흡수(D1 A 기준): explorer-search lane의 `git-status-supply.rs`가 가진 status 저장소를 이 캐시의 status 범위로 옮기고, 그 파일의 공개 API(`status(&ProjectId) -> Option<&GitStatus>`, `generation(&ProjectId)`)는 캐시를 읽는 얇은 함수로 남겨 탐색기·제목줄 호출부를 바꾸지 않습니다. 캐시는 `AppResult`를 보관해 git-2가 오류를 보이게 합니다(공급 API는 `Ok`만 노출). 그 lane의 host 명령(`GitBranch`를 넓힌 `GitStatus(project)`)은 `git::Command::Status`로 대체하고 옛 변형은 지웁니다. D1이 다른 안으로 정해졌으면: B안이면 탐색기 전용 조회를 이 캐시 읽기로 바꾸고, C안이면 git-38을 이 lane이 2.1의 설계대로 구현합니다.
- 제목줄 branch(`title_branch::Branches`)는 공급이 이미 흡수했으면 그대로 두고, 아니면 이 캐시의 status를 읽도록 바꿉니다(shell-61 검사 회귀 확인).

#### 2.0.4 diff 표면 공통 (git-40·47·50·54·63이 사용)

- 가상 문서: `DocumentKey::Virtual(VirtualKey { owner, side })`. owner는 diff 탭이면 그 `TabId`, 탭이 아닌 표면(충돌 비교 대화상자, 파일 이력 inline)은 표면 생성 시 `TabId::new()`. 읽기 전용은 `DocumentMetadata.read_only = true`, baseline = 처음 내용(dirty 아님). 내용 갱신은 `replace_virtual`(같은 내용이면 무동작).
- `DocumentKey`에 변형을 더하면 아래의 전수 match가 컴파일 오류로 드러납니다. 모두 "파일 아님"으로 처리합니다(저장·mirror·LSP·진단·IDE 선택·hot exit 대상 아님): `ED/store.rs`(baseline), `APP/application.rs`(완성 `model_path` 등), `APP/editor-locations.rs`, `APP/persistence.rs`(mirror: `Ok(false)`), `APP/host.rs`, `APP/diagnostics.rs`, `APP/editor-problems.rs`, `APP/editor-json.rs`, `APP/editor-json-validation.rs`. 동결 `taide-remote-web`는 `DocumentKey`를 전수 match하지 않아 소스 변경이 없습니다(Wasm check로 확인).
- view: 한 탭 안에 편집기 두 개라 `ViewKey { window, pane, tab }`가 겹칩니다. peek 미리보기 선례(`APP/editor-locations.rs`의 `store.attach_view(ViewKey { window, pane: PaneId::new(), tab: TabId::new() }, ..)`)처럼 표면이 만든 합성 ViewKey로 붙이고, 표면이 사라지면(탭 닫힘, 대화상자 닫힘, 프로젝트 닫힘) `detach_view`·문서 해제합니다. 합성 view는 view-state 저장 대상이 아닙니다.
- 줄 계산(4.1 결정 7 A 기준): `diff_hunks`(libgit2 `Patch::from_buffers`, `context_lines(0)`, `indent_heuristic(true)`, `git_gutter`·`stage_hunk`와 같은 옵션 — `CR/taide-git/src/service.rs:571-623,750-827`)가 `DiffHunk { old_start, old_lines, new_start, new_lines }`를 돌려줍니다. `ignore_trim_whitespace`가 참이면 양쪽 각 줄을 앞뒤 공백 제거한 사본으로 diff합니다(줄 번호는 그대로라 monaco `ignoreTrimWhitespace`의 줄 단위 판정과 같음). git diff 표면은 `false`(`TS/features/git/diff-view.tsx:46-50`), ClaudeDiff는 monaco 기본 `true`(`TS/widgets/claude-diff-pane/claude-diff-pane.tsx:58`이 옵션을 주지 않음, monaco 기본값 `node_modules/monaco-editor/esm/vs/editor/common/config/diffEditor.js:12`).
- 재계산: 내용이 바뀌면(편집 가능한 ClaudeDiff 수정 쪽) 200ms 뒤 다시 계산(monaco `diffEditorViewModel.js:86` `RunOnceScheduler(.., 200)`). 계산은 `git::Command::Hunks`로 worker에서 하고 응답 키(문서 revision)가 현재와 다르면 버립니다.
- side-by-side 렌더: 원본·수정 NativeEditor 두 개. 각 hunk에서 줄 수가 적은 쪽에 차이만큼 filler zone(높이 = 줄 수 차 × 줄 높이, 빗금 `diffEditor.diagonalFill` dark `#cccccc33`/light `#22222233`). 줄 배경 원본 `diff.removedLineBackground`, 수정 `diff.insertedLineBackground`. 한쪽 세로 스크롤을 다른 쪽에 같은 표시 행으로 `scroll_override`로 반영. 가운데 분할선 끌기(기본 0.5, 0.1~0.9, monaco `splitViewDefaultRatio`). 폭 900px 이하면 inline(monaco `renderSideBySideInlineBreakpoint: 900`, `useInlineViewWhenSpaceIsLimited: true`).
- inline 렌더: 수정 문서 하나. 삭제 줄은 해당 위치의 zone 안에 원본 문서의 그 줄들을 같은 구문 색으로 읽기 전용 그림(삭제 배경). gutter 줄 번호는 원본·수정 두 열.
- 숨김 영역(git-42): 변경 주변 3줄을 남기고 3줄 이상 연속 미변경 영역을 접은 뒤 막대(숨긴 줄 수, 위·아래 20줄 펼침, 전체 펼침)를 zone으로 그림(monaco 기본 `contextLineCount 3`, `minimumLineCount 3`, `revealLineCount 20`). 막대 배경 `appSidebar.background`(monaco `diffEditor.unchangedRegionBackground` = `sideBar.background`), 글자 `app.foreground`.
- 문자 단위 강조·± 표시·overview ruler·ClaudeDiff revert 화살표의 범위는 4.1 결정 4.

#### 2.0.5 표시 공통

- 색 토큰(`CR/taide-theme/src/service.rs:350-413`, 모두 기본 테마에 있음): 상태 글자 `git.modified/added/deleted/renamed/untracked/conflicted`(typeChange는 modified), staged lane `git.staged`, 그래프 `graph.lane1..12`·`graph.refBranch`, gutter `editorGutter.addedBackground/modifiedBackground/deletedBackground`, blame `editorBlame.foreground/background`, diff `diff.insertedBackground`(문자)·`diff.insertedLineBackground`(줄)·`diff.removedBackground`·`diff.removedLineBackground`·`diff.border`(`TS/shared/lib/monaco/theme.ts:28-32` 매핑), 목록 `explorer.itemHover/itemSelected/itemFocused/background`, 입력 `panel.inputBackground/inputBorder`, 오류 글자 `statusIndicator.error`, 보조 글자 `appSidebar.iconDefault`, 배지 배경 `appSidebar.itemActive`. 새 토큰은 없습니다.
- 상태 글자: modified `M`, added `A`, deleted `D`, renamed `R`, untracked `U`, typeChange `T`, conflicted `!`(`TS/features/git/status-row-item.tsx:11-29`, `TS/widgets/git-panel/commit-detail-panel.tsx:24-42`).
- 짧은 해시 7자(`TS/entities/git/git.constant.ts:5`). 상대 시간(그래프·이력): `relativeTimeToken`의 `git.timeDaysAgo {n}`/`git.timeHoursAgo`/`git.timeMinutesAgo`/`git.timeJustNow`. blame 문구는 locale 없이 영어 고정(`TS/shared/lib/blame-format.ts`). 현재 시각은 주입한 now로 계산.
- toast: 오류 본문은 `UI/toast.rs`의 `describe_error`(= `describeIpcError`), 오류 toast는 `Toasts::ipc_error`, 성공·경고·정보는 `Toasts::success/warning/info`, 설명이 붙으면 `Toasts::notify(Kind, title, Options { description, .. }, now)`.
- 아이콘: `APP/resources/icons/*.svg`(Lucide, `LICENSE.txt`) + `APP/navigation-icons.rs`·`UI/glyph-icons.rs`의 `include_bytes!` 표. 이미 있음: git-branch(`resources/problems/git-branch.svg`), file-diff, sparkles, loader-circle(Loader2), x·file·chevron-right(`resources/problems`). 추가: archive, arrow-up, arrow-down, refresh-cw, minus, plus, undo-2, rotate-ccw, tag, tags, trash-2, check, arrow-left, history(explorer-search lane이 history·check·x를 먼저 추가했으면 재사용). 원본 path는 `node_modules/lucide-react/dist/esm/icons/<name>.mjs`의 iconNode. 새 crate 의존성 없음.

#### 2.0.6 대화상자 공통

확인/취소 대화상자는 `APP/delete_dialog.rs`(egui::Modal, 첫 프레임 취소 버튼 focus, 결과 `Option<bool>`) 형태를 따릅니다. 취소 문구 `common.cancel`, Escape·바깥 클릭은 취소. 공통 modal 정책 `taide_native_ui::modal`(`Chrome::scrim`, `FocusReturn`, `takes_escape`, `trap_focus`)을 씁니다. IME 조합 판별은 `taide_native_ui::modal::Composition::observe`.

#### 2.0.7 검사 공통과 금지

- 하네스: `[C]` `CR/taide-git` 기존 tests 모듈(임시 저장소) 또는 앱·UI crate 안 순수 모듈 테스트, `[U]` `native/taide-native-ui/tests/*.rs`(`--features native-host,inspection`), `[A]` `application-*-tests.rs`의 실제 `NativeApplication` + frame helper(`APP/application-explorer-views-tests.rs`의 `Clock`, `APP/application-auxiliary-window-tests.rs`의 `Fixture`·`isolated_state`·`launch`) + accesskit 노드 단언.
- 명령(모두 lane worktree의 `CARGO_TARGET_DIR=<worktree>/target-lane`): 앱 `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib -- <필터> --test-threads=4`, UI `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test <파일>`, crate `cargo test --manifest-path crates/taide-git/Cargo.toml <필터>`, UI·editor crate를 바꾼 묶음은 동결 remote-web `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline`.
- Git fixture: `APP/remote-git-tests.rs:147-201` 방식 재사용 — `git init --initial-branch=main --template <빈 디렉터리>`, local config `user.name/email`, `core.hooksPath=<빈>`, `init.templateDir`, `core.fsmonitor=false`, `core.attributesFile=/dev/null`, `core.excludesFile=/dev/null`, `commit.gpgSign=false`, `tag.gpgSign=false`, `credential.helper=""`, `credential.interactive=false`, `protocol.allow=never`, `protocol.file.allow=always`, `pull.rebase=false`, `push.default=upstream`; 테스트 쪽 git 실행은 `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, `GIT_TERMINAL_PROMPT=0`. 원격은 같은 임시 디렉터리의 bare 저장소(file 경로)만. 충돌 fixture는 두 브랜치가 같은 줄을 바꾼 뒤 테스트 쪽 `git merge`로 만듭니다. 이 fixture는 G1에서 `APP/application-git-fixture.rs`(cfg(test))로 한 번 만들고 이후 읽기 전용입니다.
- 주입 포트(가짜로 교체): AI commit message 함수(`APP/editor-ai-edit.rs`의 `State.complete` `Arc` 클로저 방식), OS 알림(settings lane 진입점의 가짜 platform), clipboard·reveal(기존 탭 메뉴 검사의 intent 단언), git init(4.2 허락 6), 시간(now 주입).
- 금지: GUI 실행, 실제 OS 알림, 실제 Keychain·AI provider, 실제 clipboard, Finder reveal 실행, **추적 안 된 파일 discard**(`service::discard`가 사용자 휴지통으로 옮김, `CR/taide-git/src/service.rs:217-248`), 네트워크 원격, 저장소 밖 탐색, 작업 저장소 git 상태 변경, 허용 밖 의존성, `tests/explorer-delete.rs` 실행.

#### 2.0.8 원본 상수 대응

| 원본 | 값 | 출처 |
| --- | --- | --- |
| 변경 행·머리말 높이 | 24px | `TS/widgets/git-panel/git-panel.tsx:61`, `TS/features/git/git-section-header.tsx:28` |
| 가상 목록 overscan | 12 | `git-panel.tsx:63`, `commit-graph.tsx:45` |
| 행 들여쓰기 | 16px(`pl-4`) | `git-section-header.tsx:21` |
| 패널 헤더 줄 | 32px(`h-8`), 좌우 8px, 간격 6px, 12px 글자 | `git-panel.tsx:496` |
| 커밋 입력 | 3행, 오른쪽 28px 비움(AI 버튼 20px, 위·오른쪽 6px) | `TS/features/git/commit-box.tsx:20,65-74` |
| 로그·파일 이력 페이지 | 100 | `git.query.ts:50` |
| AI 요약 커밋 수 | 20 | `TS/widgets/git-panel/ai-commit-message.ts:4` |
| 커밋 초안 기억 | 프로젝트 8개 | `TS/entities/git/commit-message-memory.ts` |
| 그래프 행/레인/노드/선 | 24 / 14 / r 3 / 1.5px, 색 12개 | `TS/widgets/git-panel/commit-graph.tsx:40-48` |
| 그래프 pane | 기본 240px(`Settings.git_graph_panel_size_px`), 최소 120px, 접힘 24px, 저장 지연 120ms | `git.constant.ts:12`, `TS/shared/constants/layout.ts`, `TS/entities/layout/pane-resize-commit.ts` |
| 분리선 두께 | `Settings.resizer_thickness`(기본 1), 끌기 영역 8px | `git-panel.tsx:553,649` |
| blame 지연 | 300ms, overlay 앞 공백 4칸, 메시지 50자 | `TS/widgets/editor-pane/use-editor-blame.ts:10-12`, `blame-format.ts:3` |
| 충돌 재파싱 지연 | 150ms | `TS/widgets/editor-pane/use-editor-git-gutter-and-conflicts.ts:29` |
| 파일 이력 패널 | 폭 384px, 머리말 36px, 항목 40px | `TS/widgets/file-history/file-history-panel.tsx:52-54,101` |
| diff 자동 inline | 폭 900px 이하 | monaco `diffEditor.js:33-34` |
| diff 재계산 지연 | 200ms | monaco `diffEditorViewModel.js:86` |
| 숨김 영역 | 문맥 3줄, 최소 3줄, 펼침 20줄 | monaco `diffEditor.js:25-30` |
| 백엔드 status TTL | 2초 | `CR/taide-git/src/store.rs` |

---

### git-1 SCM 뷰 진입 (missing)

- 원본: 탐색기 4보기 중 Git(`TS/widgets/explorer/explorer-panel.tsx` 아이콘 GitBranch, `git.title`, 본문 `:259` `GitPanelContainer`). 진입 3경로: 키맵 `git`(⌃⇧G) → `requestShowExplorerView('git')`(`TS/widgets/app-shell/app-shell.tsx:143-147`, 보조 창 `auxiliary-window-shell.tsx:96-100`), 명령 `view.git`(`TS/shared/lib/command-catalog.ts:94`, isEnabled 없음 = 항상), 명령 `git.createTagOnHead`(`TS/entities/git/git.commands.ts:24-27`). `git`과 `explorer` 키는 같은 전역 키맵 처리라 프로젝트가 없어도 키를 소비합니다. 요청은 focus된 shell slot만 받고(`explorer-panel.tsx:142-149`) 접힌 사이드바를 펼칩니다(`project-shell.tsx`, 보조 창 `auxiliary-window-shell.tsx:106-107`).
- 현재: `APP/symbol-sidebar.rs`의 `View::Git` 전환 버튼·slot별 보존. 사이드바 렌더(`application.rs`의 explorer region 그리기)가 `View::Search | View::Git`이면 본문 없이 반환. 키 `git` ⌃⇧G 정의(`UI/keymap-defaults.json`), `view.git` 명령 정의(`UI/keybinding-commands.json`), 그러나 `keymap_run`에 `"git"`이 없어 미소비(`UI/command-registry.rs` 테스트가 `"git"`을 미지원으로 단언, `APP/shell_keymap.rs` 테스트 `assert!(!supports("git"))`·`auxiliary("git").is_none()`, `APP/command-dispatch.rs` 테스트). Files 보기 전환·접힘 해제는 `show_explorer_files`/`show_auxiliary_explorer_files`(application)에 있음.
- 빠진 것: Git 본문, ⌃⇧G·`view.git` 실행, 보조 창 처리.
- 설계: `Run::ShowGit`(`"git"`, `("view.git", Run::ShowGit)`)을 더하고 `runs_without_focused_project`에 넣습니다(`ShowExplorer`와 같은 규칙). `shell_keymap::intent`에서 `ShellIntent::Git(Intent::ShowView { project, slot })`(보조 창 `ShowAuxiliaryView`)를 냅니다. `apply_git_intent`가 기존 Files 함수를 view 인자를 받는 `show_explorer_view(project, slot, View)`로 일반화해 부릅니다(Files 동작 불변). 사이드바 렌더는 `View::Git`이면 `show_git_view(ui, project, slot, ..)`를 부릅니다(git-2 이후 내용). shell 사양 2.13의 "`Run::ShowExplorer`에 보기 인자" 안과 결과가 같고, 이 lane이 먼저 구현하므로 shell-21은 행만 갱신합니다.
- 검사: `[C]` `keymap_run("git") == Some(Run::ShowGit)`, `view.git` 실행값. `[A]` 메인 2 slot에서 ⌃⇧G → focus slot만 Git 보기·다른 slot 유지, 접힌 사이드바 펼침, 보조 창 키 → 그 창 explorer 펼침·Git, 프로젝트 없을 때 키 소비(편집기에 문자 안 들어감). 기존 세 테스트의 `"git"` 미지원 단언 갱신.
- 완료: 3경로가 실제 App에서 Git 보기를 보이고 slot·보조 창 규칙을 지킴.

### git-2 비저장소 안내와 초기화 (unwired)

- 원본(`TS/widgets/git-panel/git-panel-container.tsx:142-143,247-256`): status 조회가 **어떤 오류든** 실패하면 패널 대신 가운데 정렬 안내 `git.notARepository`(보조 색 14px) + outline sm 버튼 `git.initRepository`(초기화 중 비활성). 클릭 → `git_init` → 성공 toast `git.initSuccess`, 실패 toast `git.initFailed`(오류 상세 없음). 성공 시 무효화로 패널 표시. 오류 상태는 재조회 성공 전까지 유지(retry 0).
- 현재: `git_actions::git_init`(store 제거 + status/refs 이벤트).
- 설계: `git-scm.rs`가 `cache.status(project)`가 `Err`이면 안내 화면. `git::Command::Init`, `init_pending` 동안 버튼 비활성.
- 검사: `[A]` 비저장소 프로젝트(임시 디렉터리)에서 Git 보기 → 안내·버튼 노드, 클릭 → 가짜 init 포트 성공 → success toast·패널 전환, 실패 포트 → `git.initFailed`, 진행 중 버튼 비활성.
- 완료: 안내·버튼·비활성·두 toast·성공 후 패널 전환 근거.

### git-3 브랜치 스위처 트리거 (missing)

- 원본(`TS/features/git/branch-switcher.tsx:47-60`, `git-panel.tsx:497-504`): 헤더 왼쪽 버튼(GitBranch 14px + 굵은 브랜치명 말줄임, 없으면 `git.noRepositoryLabel`), tooltip(아래)·aria `git.branchSwitch`, `branch == None`이면 비활성(50% 불투명).
- 현재: 없음(`GitStatus.branch`).
- 설계: `git-branch.rs::trigger(ui, branch, disabled)`. tooltip은 앱 공통 `self.tooltips.show`.
- 검사: `[A]` 저장소에서 브랜치명 노드·tooltip, status 미도착이면 `git.noRepositoryLabel`+비활성, detached HEAD는 `current_branch` 결과대로.
- 완료: 문구·비활성·tooltip 근거.

### git-4 브랜치 목록·체크아웃 (unwired)

- 원본(`branch-switcher.tsx:20-79`, `TS/features/git/branch-group.tsx`): popover(폭 256px, 시작 정렬) 안 필터 입력(`git.branchFilterPlaceholder`), 항목이 없으면 `git.branchNotFound`, 그룹 `git.branchLocal`·`git.branchRemote`, 필터 = 이름 소문자 포함 비교, 빈 그룹 숨김. 행: GitBranch 16px(HEAD면 `app.accent`) + 이름 + HEAD면 오른쪽 Check(accent), 원격이면 오른쪽 `git.checkoutRemoteBranch`(10px 보조 색). 선택 → 닫기·필터 초기화 → 원격이면 `git_checkout_remote_branch(remoteRef)` 아니면 `git_branch_checkout(name)`. 성공 toast `git.branchSwitched {name}`(원격은 remoteRef 그대로), 실패 오류 toast(`git-panel-container.tsx:119-132`).
- 현재: `git_branches`·`git_branch_checkout`·`git_checkout_remote_branch`.
- 설계: egui `Popup`(anchor=트리거) + 단일 줄 입력 자동 focus + ↑↓ 선택·Enter 실행(cmdk 동작), Escape 닫기. 데이터 `cache.branches`.
- 검사: `[A]` fixture(로컬 `main`·`feature`, bare 원격 `origin/feature-remote`) — 필터 대소문자 무시, 빈 그룹 숨김, 체크아웃 후 branch 갱신·toast, 원격 체크아웃이 로컬 추적 브랜치 생성(테스트 쪽 `git rev-parse --abbrev-ref HEAD`), 실패(작업 트리 충돌) 오류 toast, 키보드 선택.
- 완료: 위 분기 근거.

### git-5 새 브랜치 만들기 (unwired)

- 원본(`branch-switcher.tsx:25-26,41-45,66-73`): 필터 trim이 비어 있지 않고 같은 이름 브랜치(로컬·원격 이름 전체 비교)가 없을 때 목록 맨 위 그룹에 Plus + `git.branchCreateNamed {name}`. 선택 → 닫기·필터 초기화 → `git_branch_create(name, checkout: true)` → `git.branchSwitched {name}` / 오류 toast(`git-panel-container.tsx:134-138`).
- 설계: 같은 popup의 조건부 첫 항목.
- 검사: `[A]` 새 이름 생성·전환, 기존 이름 정확 일치 시 항목 없음, 앞뒤 공백 trim, 그 상태에서 `git.branchNotFound` 미표시.
- 완료: 조건·생성·전환·toast 근거.

### git-6 ahead/behind와 remote 이름 (unwired)

- 원본(`git-panel.tsx:505-516,536`): `has_remote && ahead > 0`이면 ArrowUp 12px + 수, behind도 ArrowDown. 맨 오른쪽 `remotes[0].name`(11px 보조 색, 왼쪽 4px, 말줄임).
- 현재: `GitStatus { ahead, behind, has_remote }`, `git_remotes`.
- 검사: `[A]` bare 원격에 push 안 한 커밋 1개 → `↑1`, 원격 쪽 커밋을 테스트 쪽에서 fetch → `↓1`, 원격 없으면 둘 다·이름 없음.
- 완료: 표시 조건 4가지 근거.

### git-7 Sync (unwired)

- 원본(`git-panel.tsx:526-535`, `git-panel-container.tsx:206`, `git.query.ts:190-237`): `has_remote`일 때만 버튼(tooltip `git.sync`, RefreshCw, pull 또는 push 진행 중이면 Loader2 회전·비활성). 클릭 → pull, 성공 시 push, 각 실패 오류 toast. pull·push 각각 OS 알림: 성공 범주 `gitRemote`, 제목 = 프로젝트 표시명(없으면 `notification.gitPullSucceeded|gitPushSucceeded`), 본문 `[성공 문구, 캐시된 branch]`; 실패 범주 `error`, 제목 같은 규칙(`...Failed`), 본문 `[실패 문구, describe_error]`, ` · `로 연결.
- 현재: `git_pull`·`git_push`. 알림 진입점은 settings lane(`domains::settings::notify`, settings-23 설계 3)이 wave 2에서 만듭니다.
- 설계: 패널 상태 `sync: Idle | Pulling | Pushing`(프로젝트별). Pull 성공 응답에서 Push 제출. 알림은 settings 진입점을 부르고 새 경로를 만들지 않습니다.
- 검사: `[A]` bare 원격 fixture에서 pull→push 순서(원격 ref 갱신), pull 실패(원격 경로 제거) 시 push 미실행·오류 toast, 진행 중 비활성. 알림: 가짜 platform으로 4종 범주·본문 단언(실제 발송 금지).
- 완료: 순서·스피너·오류 근거 + 알림 근거(settings 병합 뒤). settings-23 git 부분 근거로 기록.

### git-8 Stash push 버튼 (unwired)

- 원본(`git-panel.tsx:517-525`, `git-panel-container.tsx:109-113,283`): Archive 아이콘(tooltip `git.stashPush`, 헤더 오른쪽 정렬 시작), `status.rows.len() > 0`일 때만 활성. 클릭 → `git_stash_push(message: None)` → 성공 toast `git.stashPushed`, 실패 오류 toast.
- 검사: `[A]` 변경 없으면 비활성, 변경 후 stash → toast·변경 목록 비고 stash 섹션 1건.
- 완료: 활성 조건·toast 근거.

### git-9 Stash 섹션 (unwired)

- 원본(`TS/features/git/stash-list.tsx:22-50`, `git-panel.tsx:632-644`, `git-panel-container.tsx:115-117,284`): stash 0개면 섹션 숨김, 머리말 `git.stash` + 건수 배지(접혀도 표시), 기본 접힘(git-18). 펼치면 행: Archive + 메시지(말줄임) + `git.stashApply` 텍스트 버튼 + Trash2 아이콘(tooltip `git.stashDrop`, 오류 색). push/apply/drop 진행 중에는 두 버튼 비활성. Apply·Drop은 확인 없이 실행, 성공 toast 없음, 실패 오류 toast.
- 검사: `[A]` stash 2개 fixture로 배지 2, 기본 접힘, 펼쳐 Apply(작업 트리 복원)·Drop(목록 감소), 진행 중 비활성.
- 완료: 숨김·배지·두 동작·비활성 근거.

### git-10 커밋 박스 (unwired)

- 원본(`commit-box.tsx:39-80`, `git-panel-container.tsx:150-157`): 3행 입력(placeholder `git.commitMessagePlaceholder`, 12px, 테두리 `panel.inputBorder`), 아래 sm 버튼(`git.commit`, 진행 중 `git.committing`). 버튼 활성 = trim 비어 있지 않음 && 진행 중 아님 && blockedReason 없음. blockedReason이 있으면 버튼 위 오류 색 11px 문구. 성공 → 입력 비움(초안 삭제), 성공 toast 없음, 실패 오류 toast. 실행은 git-13 gate를 거침.
- 현재: `git_commit`(git CLI `commit -m`, `stage_all`이면 먼저 `add -A`, `CR/taide-git/src/service.rs:250-263`).
- 설계: `git-commit.rs::CommitBox`(text, committing). egui multiline 3행 고정 높이.
- 검사: `[A]` 빈 메시지 비활성, 커밋 후 입력 비움·HEAD 요약 = 메시지(테스트 쪽 `git log -1 --format=%s`), 실패(변경 없음 상태로 커밋) 오류 toast.
- 완료: 상태 4종·성공/실패 근거.

### git-11 Mod+Enter 커밋 (missing)

- 원본(`commit-box.tsx:42-48`): 입력 안에서 mac Cmd+Enter / 그 외 Ctrl+Enter, IME 조합 중이면 무시, 기본 동작 막고 `can_commit`일 때만 실행(gate 포함). 비활성이어도 줄바꿈은 들어가지 않음.
- 설계: 입력 focus 상태에서 그 키 이벤트를 소비. 조합 판별은 `modal::Composition`.
- 검사: `[A]` Cmd+Enter 실행, 조합 중(Ime Preedit) 무시, 비활성 상태에서 무시·줄바꿈 없음.
- 완료: 세 분기 근거.

### git-12 프로젝트별 초안 기억 (missing)

- 원본(`TS/entities/git/commit-message-memory.ts`, `git-panel-container.tsx:56-81`): 모듈 전역 Map(창마다 별도), 키 ProjectId, 쓰기는 삭제 후 삽입(최근순), 빈 문자열이면 삭제, 8개 초과 시 가장 오래된 것 제거. 패널은 입력마다 기록하고 프로젝트가 바뀌면 즉시 다시 읽으며 AI 진행 표시도 지움. 보기 전환에도 유지.
- 설계: `CommitDrafts`(창별, 4.1 결정 2) LRU 8. 같은 창 여러 slot이 같은 프로젝트를 보이면 같은 초안을 읽습니다.
- 검사: `[C]` 상한·빈 문자열 삭제·재기록 최신화. `[A]` 프로젝트 A 입력 → B 전환 → A 복귀 시 복원, Files 보기 다녀와도 유지.
- 완료: 단위 + App 근거.

### git-13 커밋 gate (unwired)

- 원본(`TS/widgets/git-panel/commit-gate.ts`, `git-panel.tsx:335-348,714-725`, container `150-157`): 충돌 행이 하나라도 있으면 `BlockedByConflicts`(blockedReason `git.commitBlockedByConflicts`, 버튼·단축키 차단), 아니면 staged 0 && unstaged > 0이면 `ConfirmStageAll`, 아니면 `Commit`. staged/unstaged 판정은 충돌 행 제외. ConfirmStageAll → 대화상자(제목 `git.stageAllTitle`, 설명 `git.stageAllDescription`, 취소, 확인 `git.commit`) → 확인 시 unstaged 경로 stage + 커밋(`stage_all: true`) + 닫기.
- 원본 세부: 확인 시 Stage(경로들)와 Commit(`stage_all`)을 연달아 보내지만 runtime 변경 잠금으로 직렬화되고 `add -A`가 같은 결과를 냅니다. native는 `Commit { stage_all: true }` 하나만 보냅니다(관찰 결과 동일).
- 설계: `git-scm-model.rs::commit_gate`, 확인 대화상자(2.0.6).
- 검사: `[C]` 3분기(원본 `commit-gate` 사례). `[A]` staged 없음+unstaged 있음 → 대화상자 → 확인 후 HEAD에 변경 포함·작업 트리 깨끗, 취소 시 미실행. 충돌 fixture에서 문구·비활성·Cmd+Enter 무시.
- 완료: 3분기 근거.

### git-14 AI 커밋 메시지 (unwired)

- 원본(`git-panel-container.tsx:159-245`, `TS/widgets/git-panel/ai-commit-message.ts`, `commit-box.tsx:50-75`, `TS/entities/ai/ai.ipc.ts:22-31`):
  - 버튼: 입력 오른쪽 위 Sparkles(진행 중 Loader2 회전), tooltip(왼쪽) 진행 중 `git.generatingCommitMessage` / 가능 `git.generateCommitMessage` / 불가 `git.noChangesForCommitMessage`. 활성 = 진행 중이거나 (staged+unstaged+merge 행 > 0).
  - 클릭(진행 중 아님): request_id = UUID → `git_diff_staged_text` → `ai_commit_message(request_id, owner = 창 label, provider None, model None, diff_text, recent)`. recent = 로그 앞 20개 `"{id[..7]} {summary}"`를 `\n`으로.
  - 클릭(진행 중): 최신 요청 표시 해제 → cancel(오류 무시) → 진행 해제.
  - 응답: 최신 요청이 아니면 버림. text 없음(취소)이면 무동작. sanitize = 코드펜스 제거(`stripCodeFence`) → trim → 바깥 따옴표 쌍(`"` `'` `` ` `` `“”` `‘’`) 하나 제거 후 trim. 빈 결과 → 오류 toast `git.commitMessageEmptyResponse`. 아니면 입력 교체(초안 기록) + success `git.commitMessageGenerated`, 설명 = notice를 ` · `로: `used_fallback` → `git.commitMessageUsedUnstaged`, `truncated` → `git.commitMessageDiffTruncated`, `skipped_files.len() > 0` → `git.commitMessageFilesSkipped {count}`.
  - 실패: 최신 요청일 때만 오류 toast `git.generateCommitMessageFailed` + 설명 describe_error.
  - 프로젝트 전환·패널 사라짐: 진행 중 요청을 버리고 cancel(결과가 다른 프로젝트 입력에 들어가지 않게).
- 현재: `ai_actions::ai_commit_message`·`ai_request_cancel`(owner+request_id), `APP/editor-ai-edit.rs`의 private `strip_code_fence`(같은 규칙 — 두 번째 사용이라 공용 함수로 올림), 주입 클로저 패턴.
- 설계: owner는 편집기 AI와 같은 창 label. `latest: Option<String>`을 slot 패널이 보유하고 프로젝트 변경과 패널 해제 시 abandon. 요청은 2.0.2 비동기 규칙으로 worker를 막지 않습니다.
- 검사: `[C]` sanitize 표(펜스/따옴표 5종/빈 결과), 요약 20개 형식. `[A]`(가짜 AI 포트) 성공·notice 조합 설명, 진행 중 재클릭 취소, 늦게 온 응답 무시, 프로젝트 전환 시 다른 입력에 안 들어감, 실패 toast.
- 완료: 분기 전부 근거, 실제 provider·Keychain 미사용.

### git-15 amend·fetch·branch delete·undo (unwired)

- 원본: 화면 사용처 없음(`TS/entities/git/git.ipc.ts`에 `deleteGitBranch`만, amend는 `CommitOptions.amend=false` 고정, fetch·undo는 IPC 함수도 없음).
- 현재: runtime `git_fetch`·`git_branch_delete`·`git_undo_last_commit`·`CommitOptions.amend`, remote 경로로 노출.
- 설계: 새 기능 추가 금지 원칙에 따라 native UI를 만들지 않습니다.
- 검사: 없음(기존 `APP/remote-git-tests.rs`가 backend 근거).
- 완료: 4.1 결정 1에 따라 not-applicable로 정리. 결정 전에는 손대지 않습니다.

### git-16 변경 그룹 구성 (missing)

- 원본(`TS/widgets/git-panel/git-sections.ts`, `git-panel.tsx:222-229,630`, `TS/features/git/git-change-group.tsx`): Merge(충돌 행, kind 항상 conflicted), Staged(충돌 아님 && staged 있음), Changes(충돌 아님 && unstaged 있음). 같은 파일이 Staged·Changes 둘에 나올 수 있음. 제목 `git.mergeChanges`/`git.stagedChanges`/`git.changes`, 건수 배지(접혀도 표시), 빈 그룹 숨김, 세 그룹이 모두 비면 `git.noChanges`(보조 색 12px, 위아래 6px 좌우 8px). 순서 Merge → Staged → Changes → Stash → Graph. 그래프 배지 = 로그 수.
- 설계: `git-scm-model.rs::build_sections(rows, stash_count, graph_count, collapsed)`. 배지 = 16px 높이 원형(최소 폭 16, 좌우 4px, 10px 글자, 배경 `appSidebar.itemActive`, `TS/features/git/git-section-count-badge.tsx`).
- 검사: `[C]` 원본 `git-sections.test.ts` 사례. `[A]` 같은 파일 두 그룹 표시, 깨끗한 저장소 `git.noChanges`.
- 완료: 단위 + App.

### git-17 가상 스크롤과 sticky 머리말 (missing)

- 원본(`git-panel.tsx:263-286,563-611`, `TS/widgets/git-panel/change-row-navigation.ts`): 세 그룹을 한 줄 목록(머리말 + 행, 접힌 그룹은 머리말만)으로 평탄화, 행 id `row:{group}:{path}`. 맨 위로 지나간 마지막 머리말을 상단 고정(불투명 `explorer.background`, 위 z). 키보드로 행을 맞출 때 머리말 아래 한 행 여백(`scrollPaddingStart` 24). 목록과 stash 섹션이 한 스크롤 영역.
- 현재: 탐색기의 `ScrollArea::show_rows` 가상화 방식.
- 설계: `show_rows(24, count)`로 보이는 행만 그리고 범위 첫 index 이하 마지막 머리말을 viewport 상단에 다시 그림. `scroll_to_row` 시 상단 여백 24px.
- 검사: `[C]` 평탄화·sticky index. `[A]` 행 500개 fixture에서 그려진 행 노드 수가 viewport 기준으로 제한, 스크롤 후 상단 머리말이 현재 그룹 제목.
- 완료: 단위 + App.

### git-18 섹션 접힘 영속 (missing)

- 원본(`TS/entities/git/git-section.ts`, `git-panel.tsx:208-218,297-312,457-469`): 섹션 id 순서 `merge, staged, changes, stashes, graph`, 기본 접힘은 stashes만. `Settings.git_sections_collapsed`가 없으면 기본값, 있으면 그 목록(모르는 id 무시). 토글마다 로컬 map을 먼저 바꾸고 전체 목록을 id 순서로 저장. 설정이 늦게 도착하면 한 번만 채택하고 사용자가 이미 토글했으면 무시. 저장 실패는 설정 쪽 `settings.saveFailed`.
- 현재: 모델 필드·sanitize(`CR/taide-model/src/settings.rs:367-368`, `CR/taide-settings/src/service.rs`).
- 설계: 패널 상태 `collapsed`(slot별) + `adopted` 플래그, `git::Command::PatchSettings`.
- 검사: `[C]` 변환 양방향. `[A]` 토글 후 settings.json 반영(격리 데이터 디렉터리), 새 App에서 복원, 연속 토글 두 번 모두 저장.
- 완료: 단위 + App 왕복.

### git-19 변경 행 표시 (missing)

- 원본(`status-row-item.tsx:48-99`): 24px 행, 좌우 8px, 간격 6px, 12px 글자. 파일명 + (디렉터리 있으면) 보조 색 70% 디렉터리(둘 다 말줄임). 오른쪽 상태 글자(굵게, 12px 폭, 색) — hover 또는 행 내부 focus 시 글자 대신 액션 아이콘 버튼(16px, tooltip 아래). 선택 `explorer.itemSelected`, hover `explorer.itemHover`, focus-within `explorer.itemFocused`. 행 tooltip(아래) = rename이면 `"{orig_path} → {path}"`, 아니면 path(저장소 상대).
- 설계: `git-scm.rs::status_row`. 액션 목록은 git-20.
- 검사: `[C]` 글자·토큰 매핑 7종. `[A]` rename fixture(`git mv`) tooltip, hover 시 액션 노드 표시·글자 숨김.
- 완료: 문구·색 매핑·hover 전환 근거.

### git-20 행 액션 (unwired)

- 원본(`git-change-group.tsx:79-81,112-115,142-146`, container `273-276`): Merge = [Open File]. Staged = [Unstage(Minus, `git.unstageChanges`), Open File(File, `git.openFile`)]. Changes = [Stage(Plus, `git.stageChanges`), Discard(Undo2, `git.discard`) → git-22, Open File]. Stage/Unstage는 그 행 저장소 상대 path 하나, 실패 오류 toast. Open File = 절대 경로 파일 탭(target None, preview false).
- 현재: `git_stage`/`git_unstage`/`git_discard`, 파일 탭 열기 host 경로(`HostCommand::OpenFileTab` 또는 단계 0 이후 해당 영역 명령).
- 검사: `[A]` 각 버튼 뒤 그룹 이동, Open File이 그 파일 탭 활성.
- 완료: 세 그룹 액션 근거.

### git-21 그룹 머리말 액션 (unwired)

- 원본(`git-change-group.tsx:97-103,132-134`, `git-section-header.tsx`): Staged = Unstage All(Minus, `git.unstageAll`), Changes = Stage All(Plus, `git.stageAll`), Merge 없음. hover 또는 머리말 focus 때만 표시, 클릭이 접기 토글로 번지지 않음. 대상 = 그 그룹 행 path 전부.
- 검사: `[A]` Stage All 뒤 Changes 그룹 사라짐, 클릭이 접힘을 안 바꿈, 키보드 focus로 표시.
- 완료: 두 액션 + 비전파 근거.

### git-22 Discard 확인 (missing)

- 원본(`git-panel.tsx:350-354,695-712`): 제목 `git.discardTitle`, 설명 `git.discardDescription {target}` — 1개면 그 경로(저장소 상대), 여럿이면 `git.fileCount {count}`. 취소 `common.cancel`, 확인 `git.discardConfirm`(destructive). 확인 시 그 목록 discard.
- 주의: 추적 안 된 파일 discard는 사용자 휴지통 이동(2.0.7) — 자동 검사는 추적 파일만.
- 검사: `[C]` 문구 조립(1개/다건). `[A]` 추적 파일 1개 discard 확인 → 내용 복원, 취소 → 불변.
- 완료: 단위 + 실제 단일.

### git-23 행 컨텍스트 메뉴 (missing)

- 원본(`git-change-group.tsx:82-88,116-123,147-155`, `git-panel.tsx:407-417,558-628`): 메뉴 하나를 목록 전체가 공유, 우클릭한 행을 단독 선택·anchor로 만들고 그 행 기준 항목 표시. 머리말·빈 곳 우클릭은 메뉴 없음. 대상은 path로 기억하고 목록 갱신으로 그 path가 사라지면 메뉴 닫힘.
  - Merge: Open File, Open Changes(unstaged), ─, Copy Path, Reveal(`git.openFile`, `git.openChanges`, `explorer.copyPath`, `explorer.reveal`).
  - Staged: Open File, Open Changes(staged), Unstage Changes, ─, Copy Path, Reveal.
  - Changes: Open File, Open Changes(unstaged), Stage Changes, Discard Changes(destructive, 확인), ─, Copy Path, Reveal.
  - Copy Path = 절대 경로 clipboard, Reveal = OS reveal(실패 오류 toast).
- 현재: `ShellIntent::CopyText`, `ShellIntent::RevealPath`(탭 메뉴에서 사용 중).
- 검사: `[A]` 우클릭 항목 순서 표, 머리말 우클릭 메뉴 없음, Copy Path는 intent 값 단언, Reveal은 intent만 단언(실행 금지), 메뉴 열린 채 대상 행 사라짐 → 닫힘.
- 완료: 세 그룹 항목·대상 근거.

### git-24 행 클릭·키보드 활성 → Open Changes (missing)

- 원본(`git-change-group.tsx:42,89,124,156`, `git-panel-container.tsx:208-218`): 수식키 없는 클릭 또는 Enter/Space → 단독 선택 + diff 탭. 탭 = `TabKind::Diff { path: abs_path, staged: group == Staged, before_path: orig_abs_path }`, 제목 `"{파일명} (diff)"`, target None, preview false, 실패 오류 toast. Merge 행은 unstaged.
- 현재: `layout_open_tab`(같은 kind 탭이면 그 탭 활성, `CR/taide-layout/src/service.rs`).
- 설계: `git::Command::OpenTab`.
- 검사: `[A]` 클릭 뒤 활성 탭 kind·제목, rename 행 before_path, 같은 행 두 번 → 탭 1개.
- 완료: 탭 생성 근거(내용은 git-39).

### git-25 다중 선택 (missing)

- 원본(`TS/shared/lib/list-selection.ts`, `git-panel.tsx:425-443`): Shift+클릭 = anchor~클릭 범위(Cmd/Ctrl도 누르면 기존에 합침), Cmd/Ctrl+클릭 = 토글(anchor 이동), 수식키 있으면 diff를 열지 않음. 순서 = 평탄화 행 id 순서(머리말 제외). 선택 배경 `explorer.itemSelected`. 다중 선택 일괄 동작은 원본에 없음(표시만).
- 설계: `git-scm-model.rs::resolve_selection` — TS 함수 그대로 이식.
- 검사: `[C]` 원본 `list-selection.test.ts` 사례. `[A]` Shift 범위가 그룹 경계를 넘음, 수식키 클릭은 탭 미열림.
- 완료: 단위 + App.

### git-26 키보드 roving (missing)

- 원본(`change-row-navigation.ts:92-130`, `git-panel.tsx:356-405`, `git-section-header.tsx`): 순서 = 변경 목록(머리말·행) → stash 머리말(있을 때) → 그래프 머리말(다른 pane). ↓/↑는 양끝에서 멈춤. 항목 밖(스크롤 영역)에서 ↓ = 처음, ↑ = 그래프 머리말(있으면) 아니면 마지막. 그래프 머리말에서 ↓ 무시, ↑ = 마지막 항목. 항목 0개면 ↓ = 그래프 머리말. 행 안 버튼(stash Apply 등)에 focus가 있으면 화살표 무시. 머리말: → 펼침(접혀 있을 때), ← 접힘(펼쳐 있을 때), Enter/Space 토글. 행: Enter/Space = git-24. 가상 목록 밖 행으로 가면 스크롤 후 다음 프레임에 focus.
- 설계: index별 egui focus id, 이동 요청을 `pending_focus`로 보관해 그려진 프레임에 `request_focus`.
- 검사: `[C]` 원본 `change-row-navigation.test.ts` 사례. `[A]` ↓ 연속으로 그래프 머리말 도달, ←/→ 접힘, 스크롤 밖 행 focus.
- 완료: 단위 + App.

### git-27 커밋 로그 데이터 (unwired)

- 원본(`git.query.ts:113-119`): `git_log(skip 0, take 100)`, parents·refs 포함. 백엔드 정렬 TOPOLOGICAL|TIME, 시작점 로컬 브랜치 전부 + HEAD, refs = 로컬 브랜치명·태그명(`CR/taide-git/src/service.rs:541-569`).
- 설계: `cache.log(project)`. 로그 0개면 그래프 섹션 숨김(git-16).
- 검사: `[A]` 커밋 3개 fixture에서 그래프 행 3, refs `main`·태그 배지.
- 완료: 그래프·AI 요약(git-14) 양쪽에 공급 근거.

### git-28 그래프 레인 계산 (missing)

- 원본(`TS/shared/lib/graph-lanes.ts`): 열린 레인 배열. 커밋 id가 열린 레인에 있으면 그 레인·`continues_from_above`, 없으면 첫 빈 레인(없으면 추가). 첫 parent가 다른 레인에 이미 있으면 현재 레인을 비우고 그쪽으로 edge, 아니면 현재 레인에 parent를 이어 edge. parent 없으면 레인 비움. 나머지 parent는 기존 레인 또는 새 레인. passthrough = 계산 전 열린 레인 중 자기 레인 제외.
- 설계: `git-graph.rs::compute_lanes` 순수 이식.
- 검사: `[C]` 원본 `graph-lanes.test.ts` 사례 전부.
- 완료: 사례 통과.

### git-29 그래프 렌더 (missing)

- 원본(`commit-graph.tsx:57-244`): 가상 목록(행 24px, overscan 12). 행 = 그림(폭 `(max_lane+1)*14`, max는 lane·edge.to_lane·passthrough 최대) + refs 배지(10px, `graph.refBranch` 글자·테두리, 둥근 2px, 좌우 4px, 태그도 같은 색) + 요약(말줄임) + 작성자 + 상대 시간 + 7자 해시(mono), 보조 색, 간격 8px. 그림: passthrough 레인 세로선 전체, 위에서 이어지면 위 절반 세로선, edge는 같은 레인이면 아래 절반 직선, 다른 레인이면 3차 곡선 `M x(l),c C x(l),24 x(to),c x(to),24`, 노드 원 r=3, 색 `graph.lane{(lane % 12)+1}`, 선 1.5. hover `explorer.itemHover`, 선택 `explorer.itemSelected`. 클릭·Enter/Space로 선택 토글(같은 커밋 다시 → 해제, `git-panel.tsx:674`).
- 설계: egui painter(`CubicBezierShape`). 시간은 주입 now.
- 검사: `[C]` 노드별 shape 목록(선·곡선·원 좌표). `[A]` 행 노드·선택 토글.
- 완료: 좌표 규칙 + App 선택.

### git-30 그래프 pane (missing)

- 원본(`git-panel.tsx:314-333,471-492,550-693`): 변경 목록 pane과 그래프 pane의 세로 분할(분리선 두께 `resizer_thickness`, 끌기 영역 8px), 그래프 기본 높이 `git_graph_panel_size_px`(240), 최소 120, 접힘 24(머리말만). 머리말 chevron 토글 = 접힘 저장(git-18의 `graph`). 사용자 끌기로 최소 미만이면 접힘으로 기록(높이 저장 안 함), 그 외 높이 변경은 120ms 지연 후 `git_graph_panel_size_px` 저장(지연 타이머는 slot별, 값은 전역 하나). 펼칠 때 저장 높이로 복원. 로그 0개면 그래프 pane·분리선 없음.
- 현재: 분할 계산 `UI/split.rs`(`child_rects`, `resized_pair`), shell의 탐색기 폭 끌기 패턴.
- 검사: `[A]` 끌어서 높이 변경 → 120ms 뒤 설정 저장, 최소 미만 끌기 → 접힘 기록·높이 미저장, 펼침 시 저장 높이.
- 완료: 세 분기 근거.

### git-31 커밋 상세 패널 (missing)

- 원본(`TS/widgets/git-panel/commit-detail-panel.tsx:67-112`, 배치 `git-panel.tsx:678-689`): 선택 커밋이 있고 그래프 펼침일 때 그래프 아래(pane 높이 최대 절반, 자체 스크롤). 위 경계선·위 여백 4px. 줄1: 요약(굵게, 말줄임) + 7자 해시(10px mono) + 닫기 X(tooltip `common.close`, 선택 해제). 줄2: `git.changedFiles`(11px 굵게 대문자) + 건수 배지. 오류 `editor.diffLoadFailed`(오류 색), 로딩 `common.loading`, 행 = 저장소 상대 path + 오른쪽 상태 글자(색). 커밋이 바뀌면 새로 그림.
- 현재: `git_commit_files`(불변 캐시).
- 검사: `[A]` 선택 시 파일 목록, 닫기, 로딩 노드(지연 포트), 오류(없는 rev 주입).
- 완료: 상태 3종 근거.

### git-32 상세 파일 → 커밋 diff 탭 (unwired)

- 원본(`commit-detail-panel.tsx:44-74`): 클릭·Enter/Space → `TabKind::Diff { path: abs_path, staged: false, rev: commit.id, parent_rev: parents[0], before_path: renamed면 orig_abs_path(없으면 abs_path) 아니면 abs_path }`, 제목 `"{파일명} @ {7자}"`, target None, **preview true**, 실패 오류 toast.
- 검사: `[A]` 탭 kind 필드·제목·preview, 다른 파일 클릭 시 preview 탭 교체.
- 완료: 탭 생성 근거(내용은 git-47).

### git-33 그래프 Revert (unwired)

- 원본(`commit-graph.tsx:81-103,216-219`): 행 우클릭 메뉴 첫 항목 RotateCcw `git.revert` → `git_revert_commit(rev)`(확인 없음). 성공·충돌 없음 → success `git.revertSuccess`. 충돌 → warning `git.revertConflict` + `conflicted_abs_paths[0]`을 파일 탭으로 엶. 실패 오류 toast.
- 검사: `[A]` 깨끗한 저장소 revert → 새 커밋, 충돌 유도 fixture → warning + 파일 탭 활성, staged 변경이 있을 때 오류 toast.
- 완료: 3분기 근거.

### git-34 태그 생성 (unwired)

- 원본(`commit-graph.tsx:105-117,220-223,245-251`, `TS/features/git/create-tag-dialog.tsx`): 메뉴 Tag `git.createTag` → 대화상자(제목 `git.createTag`, 설명 = 대상 문자열(커밋 id 또는 `HEAD`), 이름 입력 자동 focus(`git.tagNamePlaceholder`), 메시지 입력(`git.tagMessagePlaceholder`), 버튼 취소(outline) · `git.createTag`(이름 trim 비면·진행 중 비활성)). 열릴 때마다 두 입력 초기화. Enter 처리 없음. 확인 → `git_tag_create(name.trim, target, annotated: true, message: trim 비면 None)` → success `git.tagCreated` + 닫기 / 오류 toast(열린 채). 백엔드는 메시지 없으면 경량 태그.
- 검사: `[A]` 메시지 있음/없음 두 태그(테스트 쪽 `git cat-file -t` tag/commit), 재열기 초기화, 빈 이름 비활성, 중복 이름 오류 toast.
- 완료: 근거 전부.

### git-35 태그 삭제 서브메뉴 (unwired)

- 원본(`commit-graph.tsx:119-125,224-238`, `TS/shared/lib/git-tags.ts`): 그 커밋을 가리키는 태그(`TagInfo.target == commit.id`)가 있을 때만 Tags `git.deleteTag` 서브메뉴, 항목 = 태그 이름(destructive), 선택 → `git_tag_delete` → success `git.tagDeleted` / 오류 toast. 확인 없음.
- 검사: `[A]` 태그 2개 커밋에서 항목 2, 삭제 후 1, 태그 없는 커밋에는 서브메뉴 없음.
- 완료: 근거.

### git-36 명령 git.revertHead·git.createTagOnHead (unwired)

- 원본(`TS/entities/git/git.commands.ts:14-33`): 둘 다 활성 프로젝트가 있을 때만 활성(제목 `git.revert`·`git.createTag`, 범주 GIT). revertHead = `git_revert_commit("HEAD")`, 충돌이면 warning `git.revertConflict`(파일은 열지 않음), 아니면 success `git.revertSuccess`, 실패 오류 toast. createTagOnHead = Git 보기 표시(git-1) + 태그 대화상자(대상 `HEAD`).
- 원본 결함: 대화상자를 그래프 컴포넌트가 구독하므로 Git 보기·그래프가 안 보이거나 접혀 있으면 요청이 사라지고, slot이 여럿이면 slot마다 대화상자가 뜹니다. 재현하지 않고 focus slot에 하나만 띄웁니다(4.1 결정 3).
- 현재: 레지스트리 enablement `ActiveProject`(`UI/command-registry.rs`의 `enablement`), 실행 경로 없음.
- 설계: `Run::Git(GitRun::RevertHead | CreateTagOnHead)` → `ShellIntent::Git(Intent::RevertHead | CreateTagOnHead { project })`.
- 검사: `[C]` registry 실행값. `[A]` 팔레트 실행으로 HEAD revert·toast, 태그 명령이 Git 보기 전환 + 대화상자(그래프 접힘 상태에서도).
- 완료: 두 명령 실제 실행 근거, commands-ui-36/37 자료.

### git-37 타이틀바 branch (missing → 실제 구현됨)

- 원본: `TS/widgets/window-chrome/title-bar-content.tsx:22-28` — status 오류면 branch 숨김.
- 현재: shell-61/shell-6 complete(`APP/title-branch.rs`, `HostCommand::GitBranch` 오류 → None, 메인·보조 창, Git 이벤트 재조회). D1 A면 explorer-search lane의 공급을 거쳐 같은 결과.
- 빠진 것: 비저장소에서 숨김 단언(없으면 1건 추가).
- 완료: 대응표를 shell-61 근거와 함께 complete로 정정.

### git-38 탐색기 Git 장식 (missing, = explorer-search-7)

- 원본(`TS/widgets/explorer/file-tree-git-status.ts:7-66`, `TS/features/explorer/file-tree-row.tsx:25-42,77-81`): 상태 = 충돌이면 conflicted, 아니면 staged·unstaged 종류(typeChange→modified) 중 우선순위 높은 것(conflicted 6 > added 5 > untracked 4 > renamed 3 > modified 2 > deleted 1). 프로젝트 루트 아래 조상 디렉터리로 같은 규칙 전파(`startsWith(root + '/')`). 키는 절대 경로. 글자색 added `explorer.gitAdded`, modified `explorer.gitModified`, deleted `explorer.gitDeleted`, renamed `git.renamed`, untracked `explorer.gitUntracked`, conflicted `git.conflicted`. 배지: 파일 A/M/D/R/U/!, 디렉터리 6px 원.
- 소유: D1 A면 explorer-search lane이 wave 1에서 `explorer-git-decorations.rs`·행 렌더까지 구현합니다(그 사양 explorer-search-7). git lane은 2.0.3의 공급 흡수 뒤 회귀만 확인합니다. D1 C면 git lane이 같은 설계(순수 함수 `tree_decorations(rows, root)` + `explorer.rs` 행 그리기에 조회 1개 추가)로 구현합니다.
- 검사: D1 A — `[A]` 공급 흡수 뒤 그 lane의 장식 검사 회귀 1회. D1 C — `[C]` 원본 `file-tree-git-status.test.ts` 사례, `[A]` 수정 파일 행 색·배지, 상위 폴더 점.
- 완료: explorer-search-7과 같은 근거로 행 갱신.

### git-39 Diff 탭(rev 없음) 데이터 (unwired)

- 원본(`TS/widgets/diff-pane/diff-pane.tsx:34-59`, 탭 분기 `TS/widgets/editor-area/pane-node-view.tsx:203-212`): compare_with 없으면 `git_diff_file(path, staged ? IndexVsHead : WorkdirVsIndex, before_path)` → `{original, modified, language_id}`. compare_with 있으면 git-45.
- 현재: `git_diff_file`(rename은 before_path로 왼쪽만 읽음, 심볼릭 링크는 대상 문자열, 파일 없으면 빈 문자열 — `CR/taide-git/src/service.rs:335-379`). 탭 아이콘 `Glyph::FileDiff`.
- 설계: 탭 내용 그리기에서 `TabKind::Diff`를 `git_tab_content`로 보냄. 캐시 키 (path, mode, before_path). fs 변경으로 자동 재조회(2.0.3).
- 검사: `[A]` workdir/staged 두 탭 내용, rename 원본 쪽이 이전 경로 blob, 파일 저장 후 자동 갱신.
- 완료: 근거 + shell-89 diff 부분.

### git-40 Diff 렌더 (missing)

- 원본(`TS/features/git/diff-view.tsx:26-83`): monaco diff editor, side-by-side 기본, `ignoreTrimWhitespace: false`, 읽기 전용, 자동 레이아웃, 언어별 하이라이트. monaco 0.56 기본: 폭 900px 이하면 inline, 분할선 끌기, 문자 단위 강조(advanced), ± 표시, overview ruler, filler 빗금.
- 현재: NativeEditor(읽기 전용 `metadata.read_only`), `DecorationKind::LineBackground`·`Inline`·`Overview`(`ED/decoration.rs`), preview·AI zone용 내부 zone 조립(`UI/editor_surface.rs`의 `preview_layout`, `VerticalLayout::with_additional_zones`), 구문 강조.
- 빠진 것: 가상 문서, 줄 계산, 외부 zone 입력, 두 뷰 스크롤 동기화, inline 합성.
- 설계: 2.0.4 그대로. `UI/diff-view.rs::DiffView::show(ui, store, sides: DiffSides { original: ViewId, modified: ViewId }, hunks, options) -> DiffOutput { lane_press, .. }`. 앱은 `git-diff.rs`가 문서·view 수명과 캐시를 관리.
- 검사: `[C]` `diff_hunks`가 같은 입력의 `diff_index_to_workdir` hunk와 같음(공백만 변경 포함), `ignore_trim_whitespace` 참이면 공백만 바뀐 줄이 hunk에서 빠짐. `[U]`(`tests/diff-view.rs`) filler 높이·줄 배경 위치·900px 경계 inline 전환·스크롤 동기화·분할선 끌기. `[A]` diff 탭이 두 편집기 노드를 그림, 탭 닫으면 가상 문서·합성 view 해제.
- 완료: 단위·UI·App 근거, editor-76 diff 부분.

### git-41 Inline 모드와 Alt+\ (missing)

- 원본(`diff-pane.tsx:17,28,61-70`): DiffPane마다 `render_side_by_side`(처음 true, 비활성 탭이 되어 언마운트되면 초기화). 창 전역 keydown에서 Alt + `Backslash`(code) → 토글, 기본 동작 막음. 그래서 창에 보이는 모든 DiffPane이 함께 토글됩니다(4.1 결정 5). 커밋 diff·충돌 비교에는 이 키가 없습니다.
- 설계: 탭별 상태, 탭이 활성에서 빠지면 제거. 키 처리 위치는 결정 5.
- 검사: `[A]` Alt+\ 두 번 왕복, 다른 탭 갔다 오면 side-by-side.
- 완료: 근거.

### git-42 diff 설정 반영 (missing)

- 원본(`TS/shared/lib/diff-view-settings.ts:12-15`, `diff-view.tsx:79-81`): `editor_diff_hide_unchanged_regions`(기본 false), `editor_diff_show_moves`(기본 false)를 DiffPane·CommitFileDiff·충돌 비교에 같은 값으로 적용. ClaudeDiff에는 적용하지 않음(`claude-diff-pane.tsx:58`).
- 현재: 설정 UI(settings-27 complete). settings 사양은 소비자를 git lane 몫으로 둠(settings-28).
- 설계: 숨김은 2.0.4 숨김 영역. 이동 표시는 4.1 결정 6.
- 검사: `[U]` 숨김 막대 위치·줄 수, 펼침 20줄·전체. `[A]` 설정 변경 뒤 열린 diff 탭 즉시 반영, ClaudeDiff는 불변.
- 완료: 근거, settings-28 diff 부분 근거.

### git-43 Hunk gutter stage/unstage (unwired)

- 원본(`diff-pane.tsx:72-117`, `TS/widgets/diff-pane/diff-hunk-range.ts`, CSS `TS/shared/styles/global.css:424-446`): stage 가능할 때(git-44) 수정 쪽 각 hunk 범위 gutter 줄 장식 칸에 `git.staged` 3px 막대(손가락 커서). 순수 삭제 hunk는 `max(new_start, 1)` 한 줄. 그 칸 클릭 줄이 속한 hunk → staged 탭이면 `git_unstage_hunk`, 아니면 `git_stage_hunk(path, start, end)`, 실패 오류 toast. 성공 후 diff 재조회로 막대 갱신.
- 현재: `LaneMark::Bar`(3px, `UI/editor-gutter.rs:13,140-157`), 백엔드 stage/unstage hunk(같은 `gutter_range` 규칙, unstage는 역방향 diff의 old 쪽 = index 쪽으로 맞춤, `CR/taide-git/src/service.rs:775-791,840-877`).
- 빠진 것: lane 클릭 출력(`EditorOutput.lane_press`)과 diff 탭 연결.
- 설계: hunk 범위 = `diff_hunks`의 new 쪽에 `gutter_range` 규칙(백엔드와 같은 함수 규칙, 같은 옵션이라 `hunkNotFound`가 나지 않음).
- 검사: `[A]` workdir 탭에서 두 hunk 중 하나 stage → staged 탭에 하나, staged 탭에서 unstage, 범위 밖 클릭 무시.
- 완료: 근거, editor-73 diff 부분.

### git-44 stage 가능 규칙 (unwired)

- 원본(`TS/widgets/diff-pane/diff-stageability.ts:18-19`): compare_with 없음 && 그 path가 충돌 행이 아님(행의 `abs_path` 또는 `path` 어느 쪽이든 일치).
- 검사: `[C]` 원본 `diff-stageability.test.ts` 사례. `[A]` 충돌 파일 diff 탭에 lane 막대 없음.
- 완료: 근거.

### git-45 파일 대 파일 비교 (unwired)

- 원본(`diff-pane.tsx:42-58`): 탭 `TabKind::Diff { path, staged: false, compare_with: Some(src) }`, 원본 = src 파일 내용, 수정 = path 파일 내용, 언어 = 수정 파일. 둘 중 하나라도 오류면 git-46 오류 화면. 진입 메뉴(제목 `"{src 이름} vs {대상 이름}"`, preview true)는 D3 A로 explorer-search lane.
- 현재: `file_actions::file_open`.
- 설계: `git::Command::ReadFile` 두 개(캐시는 path 키, fs 변경 무효화).
- 검사: `[A]` `TabKind::Diff { compare_with }` 탭을 직접 열어 두 파일 내용, 한쪽 읽기 실패 시 오류 문구.
- 완료: 근거, explorer-search-22 본문 근거.

### git-46 로딩·실패 화면 (missing)

- 원본(`diff-pane.tsx:119-127`): 오류면 편집기 배경 + 가운데 오류 색 14px `editor.diffLoadFailed`. 로딩 또는 데이터 없음이면 빈 편집기 배경.
- 검사: `[A]` 지연 포트로 빈 화면, 오류 주입으로 문구.
- 완료: 근거.

### git-47 커밋 파일 diff (unwired)

- 원본(`TS/widgets/commit-file-diff/commit-file-diff.tsx`, 탭 `pane-node-view.tsx:213-223`): 원본 = `git_show_file(parent_rev, before_path)`(parent 없으면 조회 안 함, 빈 문자열), 수정 = `git_show_file(rev, path)`. `NotFound`는 그 쪽 빈 문자열(추가/삭제 파일), 그 외 오류는 `editor.diffLoadFailed`. 둘 다 준비될 때까지 빈 화면. 언어 = 경로 기반(`getLanguageIdFromPath`, native `taide_infra::language::language_id_for_path(path, overlays)`). 탭은 side-by-side(파일 이력 안에서는 inline 고정), 설정 반영, Alt+\ 없음. 불변 캐시.
- 검사: `[A]` 수정·추가·삭제·최초 커밋 4사례, 큰 blob(`error.git.blobTooLarge`) 오류 문구.
- 완료: 근거, shell-89 commit diff 부분.

### git-48 탭 메뉴 Open Changes (missing)

- 원본(`TS/widgets/editor-area/pane-tab-bar.tsx:208-222`, `TS/features/tab/tab-context-menu.tsx:120-148`): 파일 탭이면 "Reveal in Finder / Reveal in Explorer View / Rename" 다음에 `tab.openChanges`(아이콘 없음). 실행 → `TabKind::Diff { path, staged: false }`, 제목 `"{탭 제목} (diff)"`, target = 현재 창 focus pane, **preview true**, 실패 오류 toast.
- 현재: 탭 메뉴(`UI/shell.rs`의 파일 탭 분기: copy path → copy relative → reveal in Finder → (slot·창 있을 때) reveal in explorer view·rename → reopen with).
- 설계: rename 다음, reopen-with 앞에 항목 추가(slot·창 유무와 무관하게 파일 탭이면 표시). `ShellIntent::Git(Intent::OpenChanges { project, path, title, target })`.
- 검사: `[U]` workbench 항목 순서. `[A]` 실행 뒤 diff 탭이 focus pane에 preview로.
- 완료: 근거, shell-78 갱신 자료.

### git-49 Claude diff 요청 수신 (unwired)

- 원본(`TS/app/providers/ide-sync-provider.tsx:49-68`, `TS/entities/ide/claude-diff-registry.ts`): `ide:diff-requested` → 대기 목록에 `{old_path, new_contents, tab_name}` 저장 → 설정 `ide_auto_open_diff`가 false면 목록에서 지우고 `ide_resolve_diff(rejected)` → 아니면 `TabKind::ClaudeDiff { request_id, path: new_path }`, 제목 `tab_name`, 프로젝트 = 이벤트의 `project_id`, target None, preview false. 실패 오류 toast.
- 현재: `APP/ide-tools.rs`가 `AppEvent::IdeDiffRequested` 발행 후 응답 대기(timeout이면 rejected), 로컬 소비자 없음.
- 설계: shell lane의 `provider_events::Inbox`에 `IdeDiffRequested`(상한 같은 대기열)·`IdeCloseTabRequested` 기록 행을 추가하고, `consume_provider_events`에서 `domains::git` 함수(`accept_claude_diff_request`)를 부릅니다(추가만). 대기 목록 `ClaudeDiffs: HashMap<request_id, Pending>`은 앱 전역 하나(원본은 메인 창 하나에서만 처리).
- 검사: `[A]` IDE 도구 open_diff 호출(기존 `APP/ide-tools-tests.rs` fixture) → ClaudeDiff 탭 생성·제목, 설정 false → 즉시 Rejected 응답·탭 없음.
- 완료: 근거, terminal-agent-68/69·settings-80 IDE diff 부분·shell-92 diff 부분 갱신.

### git-50 ClaudeDiffPane 렌더 (missing)

- 원본(`claude-diff-pane.tsx:38-85,141-153`): 위 32px 줄(편집기 배경, 아래 경계선, 오른쪽 정렬 간격 8px: `ide.rejectChanges` outline xs, `ide.acceptChanges` default xs, 진행 중 둘 다 비활성) + diff editor(side-by-side 기본, **수정 쪽 편집 가능**, 원본 쪽 읽기 전용, monaco 기본 `ignoreTrimWhitespace: true`, diff 설정 2종 미적용, Alt+\ 없음). 원본 = 디스크 파일(`file_open`, 실패면 빈 문자열, 디스크가 바뀌면 원본만 갱신), 수정 = 요청 내용(대기 목록 없으면 빈 문자열), 언어는 파일 language_id(없으면 `plaintext`)로 양쪽 동기화.
- 설계: 수정 쪽 가상 문서 `read_only: false`, 원본 쪽 `true`. 렌더는 `UI/diff-view.rs` 재사용, `ignore_trim_whitespace: true`, 편집 뒤 200ms 재계산(2.0.4). monaco revert 화살표는 4.1 결정 4.
- 검사: `[A]` 원본·수정 내용, 수정 쪽 입력 반영·재계산, 원본 쪽 입력 거부, 공백만 다른 줄은 변경 표시 없음.
- 완료: 근거, shell-89 claudeDiff 부분.

### git-51 Accept/Reject (unwired)

- 원본(`claude-diff-pane.tsx:110-139`): 진행 중이면 무시. Accept → 수정 편집기 현재 내용(없으면 요청 내용)으로 `ide_resolve_diff(saved, content)` → 대기 목록 제거 → success `ide.diffAccepted` → 탭 닫기. Reject → `rejected` → info `ide.diffRejected` → 탭 닫기. 실패 오류 toast, 진행 해제.
- 현재: `ide_actions::ide_resolve_diff`(저장은 `IdeSaveFile`).
- 검사: `[A]` 편집 후 Accept → 파일 내용 = 편집 결과·IDE 응답 Saved, Reject → Rejected·파일 불변, 연타 1회만.
- 완료: 근거.

### git-52 탭 닫힘·창 이동·IDE close 구분 (unwired)

- 원본(`claude-diff-pane.tsx:26-36,101-108`, `ide-sync-provider.tsx:131-133`): 표면이 사라질 때 모든 창 트리에서 탭을 찾고, 있으면(탭 전환·다른 창 이동) 무동작, 없고 미해결이면 rejected. 백엔드 닫기 observer가 먼저 `TabClosed`를 보냄. IDE의 close_tab 요청(`IdeCloseTabRequested { request_id }`)은 대기 목록에서 지움.
- 현재: `APP/tabs.rs` 닫기 경로가 `take_pending_diff` 후 `TabClosed` 응답, 이동 경로는 응답 없음.
- 설계: 표면 해제 때 별도 응답을 보내지 않습니다(백엔드 처리로 충분, 중복 응답은 `NotFound`). 탭 이동 뒤에도 대기 목록 유지, 보조 창에서 표면 표시. `IdeCloseTabRequested`면 목록에서 제거(git-49의 수신함 분기).
- 검사: `[A]` 탭 닫기 → IDE 응답 TabClosed, 새 창 이동 → 응답 없음·보조 창에서 Accept 가능, 다른 탭 전환 후 복귀 시 유지, IDE close_tab → 목록 제거.
- 완료: 근거.

### git-53 파일 이력 패널 (unwired)

- 원본(`TS/widgets/file-history/file-history-panel.tsx:23-116`): 오른쪽 전체 높이 도킹 대화상자(폭 384, 왼쪽 모서리만 둥글게, scrim·바깥 클릭/Escape 닫힘, 접근성 제목 `git.fileHistory`). 머리말 36px: (항목 선택 시) 뒤로 ArrowLeft(tooltip `git.fileHistory`) + 제목(선택 시 파일명, 아니면 `git.fileHistory`, 12px 굵게) + 닫기 X(`common.close`). 목록: 오류 `editor.diffLoadFailed`, 로딩 `common.loading`, 빈 `git.noFileHistory`, 항목 40px 두 줄(요약 / 작성자·상대 시간·7자 해시, 10px 보조 색). 닫으면 path·선택 초기화. 조회 `git_file_log(path, 0, 100)`.
- 설계: `git-history.rs` 창별 overlay(요청한 창).
- 검사: `[A]` 커밋 3개 파일 이력 항목 3, 이력 없는 새 파일 빈 문구, Escape 닫힘.
- 완료: 근거.

### git-54 이력 항목 → inline diff (missing)

- 원본(`file-history-panel.tsx:55-84`): 항목 클릭·Enter/Space → 본문을 `CommitFileDiff(rev = entry.id, parent = parents[0], path, before_path = path, inline 고정)`로 교체, 뒤로 버튼으로 목록 복귀(rename 추적 없음).
- 설계: git-47 표면을 overlay 안에서 재사용(가상 문서 owner = `TabId::new()`).
- 검사: `[A]` 항목 선택 뒤 diff 노드(inline), 뒤로 뒤 목록, 패널 닫으면 가상 문서 해제.
- 완료: 근거.

### git-55 파일 이력 진입점 (unwired)

- 원본: (1) 탭 메뉴 History `git.fileHistory`(아이콘 History, Open Changes 다음, 파일 탭)(`tab-context-menu.tsx:142-147`), (2) 탐색기 파일 메뉴(`TS/features/explorer/file-tree-context-menu.tsx:140-143`, D2 A로 이 lane), (3) 편집기 우클릭 `9_git` 그룹 order 3(`use-editor-git-gutter-and-conflicts.ts:303-313`), (4) 명령 `git.openFileHistory`(focus 편집기 동작 실행 — 편집기 없으면 무동작)(`git.commands.ts:40-45`). 요청은 focus slot의 패널만 받음(`file-history-panel.tsx:29-46`). 키바인딩 id `monaco.taide.openFileHistory`.
- 현재: 레지스트리 정의만. 편집기 메뉴 그룹 `UI/editor-context-menu.rs::groups`(navigation, 1_modification, 9_cutcopypaste, 9_terminal).
- 설계: `Run::Git(GitRun::OpenFileHistory)`(활성 편집기 필요). 편집기 메뉴에 `9_git` 그룹을 `9_cutcopypaste`와 `9_terminal` 사이에 추가(monaco 그룹 사전순), `embedded`(peek)면 제외. 탭 메뉴 항목. 탐색기 파일 메뉴는 explorer-search-20 구조의 파일 묶음 끝(그 사양 explorer-search-23).
- 검사: `[U]` 편집기 메뉴 그룹 순서·peek 제외. `[A]` 4경로 각각 패널 열림, 2 slot 중 focus slot만.
- 완료: 근거, editor-75·explorer-search-23·shell-78·commands-ui-39 자료.

### git-56 커서 줄 blame footer (unwired)

- 원본(`TS/widgets/editor-pane/use-editor-blame.ts:27-96`, `TS/features/editor/blame-footer-bar.tsx`, `blame-format.ts`, 배치 `TS/widgets/editor-pane/editor-pane.tsx:439-444`): 파일 편집기 아래 24px 막대(배경 `editorBlame.background`, 글자 `editorBlame.foreground` 11px, 위 경계선, 좌우 8px, 말줄임). 코드 편집기가 그려질 때 항상 표시되고 markdown 미리보기를 켜도 편집기 쪽 아래에만 있습니다(refused·원본 삭제 화면에는 없음). 커서 줄이 300ms 멈추면 `git_blame_range(line, line)` 첫 항목. 문구: 미커밋이면 `You, now - Uncommitted changes`, 아니면 `{작성자 또는 You(current user와 같을 때)}, {상대} • {요약}`. 상대 = `just now`(<60s) / `N minute(s) ago` / hour / day(<30일) / month(<365일) / year, 시간 없으면 `unknown time`. 요약은 공백 접기·50자 초과 시 잘라 `...`. 줄이 문서 줄 수보다 크면 빈 문자열, 오류면 빈 문자열. 경로가 바뀌면 즉시 비움. 새 줄 조회 중에는 이전 값 유지.
- 현재: `git_blame_range`, `git_current_user`(무기한 캐시).
- 설계: `git-blame.rs`. footer는 파일 탭 편집기 영역 아래에 `git_editor_footer`로(편집기 그리기 지점에 호출 1개 추가).
- 검사: `[C]` 원본 `blame-format.test.ts` 사례. `[A]` 커서 이동 300ms 뒤 footer 문구, 미커밋 줄, 파일 전환 즉시 비움.
- 완료: 근거, editor-75 갱신.

### git-57 Blame overlay와 git.toggleBlame (unwired)

- 원본(`use-editor-blame.ts:98-147`, `git.commands.ts:34-39`, CSS `global.css:410-414`): 편집기 우클릭 navigation 그룹 맨 앞(order 0) `git.toggleBlame`, 명령 `git.toggleBlame`(항상 활성, focus 편집기에 실행). 켜면 그 시점 줄 수로 `git_blame_range(1, n)`(토글마다 새로 조회, 입력으로 줄 수가 바뀌어도 재조회 없음), 각 줄 끝에 `"    " + 문구`(기울임, `editorBlame.foreground`, 80% 불투명). 문서 줄 수를 넘는 줄은 생략. 경로가 바뀌면 꺼짐. 키바인딩 id `monaco.taide.toggleBlame`.
- 설계: `SurfaceExtras.line_end_texts`(paint 전용, 레이아웃·커서 불변). `DecorationKind`는 `Copy`라 문자열 변형을 넣지 않습니다. 레지스트리 `Run::Git(GitRun::ToggleBlame)`.
- 검사: `[U]` 줄 끝 텍스트가 레이아웃·커서 위치를 바꾸지 않음. `[A]` 명령 실행 뒤 각 줄 끝 텍스트, 다시 실행 시 제거, 파일 전환 시 꺼짐, 우클릭 메뉴 첫 항목.
- 완료: 근거, commands-ui-38 자료.

### git-58 편집기 gutter 막대 (missing)

- 원본(`use-editor-git-gutter-and-conflicts.ts:31-35,58,204-228`, CSS `global.css:371-408`): 충돌 파일이 아니면 `git_gutter(path)`(HEAD ↔ 작업 트리) hunk마다 added 3px 막대(`editorGutter.addedBackground`), modified 3px 막대(`modifiedBackground`), deleted 삼각형(폭 6, 높이 8, 줄 위쪽 경계, `deletedBackground`).
- 현재: `LaneMark::Bar`/`DeletedTriangle`가 같은 크기로 있음(`UI/editor-gutter.rs:13-15,140-167`). 공급자 없음.
- 설계: `git-gutter.rs`가 편집기 view별 `DecorationLayer`(revision = 문서 revision)를 `git_editor_layers`로 공급. 저장·fs 변경 시 캐시 무효화로 갱신.
- 검사: `[A]` 수정·추가·삭제 줄 lane mark 위치, 저장 후 갱신.
- 완료: 근거, editor-73 갱신.

### git-59 gutter 클릭 → hunk discard (unwired)

- 원본(`:150-160,236-252`, `TS/features/git/hunk-discard-dialog.tsx`): 충돌 아님일 때 줄 장식 칸 클릭 줄이 속한 hunk → 대화상자(제목 `git.discardHunkTitle`, 설명 `git.discardHunkDescription {start} {end}`, 취소, 확인 `git.discardConfirm`(기본 variant, destructive 아님)) → `git_discard_hunk` → 실패 오류 toast.
- 검사: `[A]` 추적 파일 hunk discard 뒤 디스크 내용 복원, 취소 시 불변.
- 완료: 근거.

### git-60 편집기 우클릭 Stage Changes (unwired)

- 원본(`:254-301`, `TS/shared/lib/selection-line-range.ts`): 우클릭 메뉴 `9_git` order 1 `git.stageChanges`(peek 제외), 실행 시점 선택으로: 충돌 파일이면 무시, 선택 있으면 `git_stage_lines(start, end)`(끝이 1열에서 끝나고 여러 줄이면 끝 줄 제외), 선택 없으면 커서 줄이 속한 gutter hunk를 `git_stage_hunk`, 없으면 무시. 오류 toast. Unstage 항목은 없음. 키바인딩 id `monaco.taide.gitStageSelection`.
- 설계: `Run::Git(GitRun::StageSelection)`, 메뉴 `9_git` 그룹 첫 항목.
- 검사: `[C]` 줄 범위 함수. `[A]` 2줄 선택 stage 뒤 staged diff에 그 2줄만, 커서 hunk stage.
- 완료: 근거.

### git-61 충돌 마커 파싱·표시 (missing)

- 원본(`TS/features/git/conflict-marker.ts:30-67`, `use-editor-git-gutter-and-conflicts.ts:65,184-228`, `TS/widgets/editor-pane/conflict-status.ts`): 충돌 판정은 status 행 `abs_path == 탭 경로 && is_conflicted`. 파싱: 줄 끝 `\r` 제거, `<<<<<<<` 시작(라벨 = 나머지 trim) → (선택) `|||||||` base → 정확히 `=======` → `>>>>>>>` 끝(라벨). 짝이 안 맞는 영역은 버림. 표시: 시작 ~ (base 또는 구분선) - 1 줄 배경 `diff.insertedLineBackground`(current), 구분선 ~ 끝 줄 배경 `diff.removedLineBackground`(incoming), 시작 줄 장식 칸에 `git.conflicted` 3px 막대(클릭 대상). 충돌 파일은 hunk 막대 대신 이것을 그림. 내용이 바뀌면 150ms 뒤 재파싱(문서 해제 확인), 첫 파싱은 즉시.
- 설계: `git-conflict.rs` 순수 파서 + 장식 layer.
- 검사: `[C]` 원본 `conflict-marker.test.ts` 사례. `[A]` merge 충돌 fixture 파일을 열면 배경·lane, 한 영역 해결 편집 150ms 뒤 갱신.
- 완료: 근거, editor-74 갱신.

### git-62 충돌 해결 대화상자 (unwired)

- 원본(`TS/features/git/conflict-resolution-dialog.tsx`, `use-editor-git-gutter-and-conflicts.ts:97-140`, `conflict-marker.ts:84-99`, `git.query.ts:303-324`): 시작 줄 lane 클릭 → 대화상자(제목 `git.resolveConflictTitle`, 설명 `git.resolveConflictDescription {start} {end}`, 버튼 순서 취소 · `git.compareChanges`(outline) · `git.acceptBothChanges`(outline) · `git.acceptIncomingChange`(outline) · `git.acceptCurrentChange`(기본)). 선택 → 문서 전체를 변환 결과로 한 번의 undo 단위 편집(current = 시작+1 ~ base/구분선-1, incoming = 구분선+1 ~ 끝-1, both = current+incoming) → 마커가 하나도 없으면 `git_resolve_conflict(path, content)`(디스크 쓰기 + 재stage) → 성공 시 편집기 dirty 해제·mirror 정리(저장 성공과 같은 정착) + success `git.conflictResolved`, 실패 오류 toast. 마커가 남으면 디스크 쓰기 없이 편집만.
- 현재: 편집기 저장 정착 경로(`HostReply::Saved` 처리), `git_resolve_conflict`.
- 설계: 정착은 저장 성공 처리와 같은 함수를 부름(편집기 영역 함수 재사용, 새 경로 없음).
- 검사: `[C]` 세 변환 표. `[A]` 2영역 충돌에서 첫 해결 뒤 디스크 불변·dirty, 두 번째 해결 뒤 디스크 = 결과·dirty 아님·status에서 충돌 해제·toast, undo 1회로 직전 상태.
- 완료: 근거.

### git-63 충돌 비교 대화상자 (missing)

- 원본(`TS/features/git/conflict-compare-dialog.tsx`, `use-editor-git-gutter-and-conflicts.ts:78-95,136-150`): Compare → 해결 대화상자 닫고 `git_conflict_sides(path)` 요청(누를 때마다 재시도), 성공하면 큰 대화상자(높이 70% 화면, 최대 폭 896, 제목 `git.compareChanges`) 안 side-by-side diff(원본 = ours, 수정 = theirs, 없으면 빈 문자열, 언어 = 파일 언어, diff 설정 반영, 대화상자 안 로딩 상태 없음). 실패는 오류 toast(누른 경우에만). 닫으면 요청 해제. 경로가 바뀌면 닫힘.
- 검사: `[A]` 비교 대화상자 두 편집기 내용, 실패 주입 toast, 닫으면 가상 문서 해제.
- 완료: 근거.

### git-64 git 상태 갱신 파이프라인 (partial)

- 원본: 2.0.3 전체(`git.query.ts:155-188`, `ipc-sync-provider.tsx:72-76,399-411,489-535,599-617`).
- 현재: watcher attach·이벤트·백엔드 status 캐시 무효화. 로컬 UI 캐시 없음.
- 설계: 2.0.3의 `GitCache`. 첫 묶음에서 만들고 모든 표면이 씁니다.
- 검사: `[C]` 이벤트별 무효화 범위(status/refs → 가변 전부, fs → status + 그 경로 diff·gutter, 불변 유지), generation 경쟁, 관찰 없는 항목 제거, 요청 상한·큐 가득 재시도. `[A]` 테스트 쪽 git CLI로 stage → `.git` 이벤트로 패널 갱신, 작업 트리 편집 → fs 이벤트로 새 행 표시, 커밋 diff 탭은 stage 이벤트에 재조회되지 않음(요청 수 단언).
- 완료: 단위 + App, shell-93 Git 부분 근거.

---

## 3. 작업 묶음 계획

새 crate 의존성은 모든 묶음에서 없습니다(diff 계산은 `taide-git`의 git2 0.21 `Patch::from_buffers`, 아이콘은 SVG 파일). lockfile 변화 없음.

### 3.1 묶음 목록 (의존 순서)

| 묶음 | ID (수) | 선행 | 수정 예상 파일 |
| --- | --- | --- | --- |
| G1 기반·진입·변경 목록 표시 | 64, 1, 2, 37, 38, 16, 18, 19 (8) | 단계 0, wave 1 병합 | 신규 `APP/domains/git.rs`·`UI/intents/git.rs`·`APP/git-cache.rs`·`APP/git-scm.rs`·`APP/git-scm-model.rs`·`APP/application-git-fixture.rs`; 추가만 `UI/command-registry.rs`(`Run::ShowGit`, `Run::Git` 골격), `APP/shell_keymap.rs`, `APP/command-dispatch.rs`(테스트 갱신), `host.rs`(`HostCommand::Git`·`HostReply::Git` 루트), `application.rs`(state 필드·`PaintSink` 기록·사이드바 Git 본문·탭 내용·편집기 레이어/footer 훅 자리), `UI/commands.rs`(`ShellIntent::Git`), `git-status-supply.rs` 흡수, 아이콘 SVG 전부·`glyph-icons.rs`·`navigation-icons.rs` 표, 대응표·QA |
| G2 변경 목록 상호작용 | 17, 20, 21, 22, 23, 24, 25, 26 (8) | G1 | `git-scm.rs`, `git-scm-model.rs` |
| G3 헤더·브랜치·stash·원격 | 3, 4, 5, 6, 7, 8, 9 (7) | G1 | `APP/git-branch.rs`, `git-scm.rs`. git-7 알림 호출은 settings lane 병합 확인 뒤(미병합이면 git-7을 partial로 두고 G8 끝에서 연결) |
| G4 커밋 | 10, 11, 12, 13, 14, 15 (6) | G1 | `APP/git-commit.rs`, `git-scm.rs`, `APP/editor-ai-edit.rs`(`strip_code_fence` 공용화) |
| G5 diff 렌더러·작업 트리 diff 탭 | 39, 40, 41, 42, 43, 44, 45, 46, 48 (9) | G1 | `CR/taide-git/src/service.rs`(`diff_hunks`), `ED/document.rs`·`ED/store.rs`(가상 문서), 2.0.4의 `DocumentKey` match 9곳, 신규 `UI/diff-view.rs`·`native/taide-native-ui/tests/diff-view.rs`, `UI/editor_surface.rs`(`show_request_with_extras`·`lane_press`), `APP/git-diff.rs`, `UI/shell.rs`(탭 메뉴 1항목) |
| G6 로그·그래프·상세·태그 | 27, 28, 29, 30, 31, 32, 33, 34, 35, 36 (10) | G1 | `APP/git-graph.rs`, `APP/git-commit-detail.rs`, `git-scm.rs`(그래프 pane), `UI/command-registry.rs`(`RevertHead`·`CreateTagOnHead`) |
| G7 커밋 diff·Claude diff·파일 이력 | 47, 49, 50, 51, 52, 53, 54, 55 (8) | G5 | `APP/git-diff.rs`, `APP/git-claude-diff.rs`, `APP/git-history.rs`, `APP/provider-events.rs`·`APP/ide-sync.rs`(분기 추가만), `UI/editor-context-menu.rs`(`9_git` 그룹), `UI/shell.rs`(History 항목), 탐색기 파일 메뉴(추가만), `UI/command-registry.rs`(`OpenFileHistory`) |
| G8 blame·gutter·충돌 | 56, 57, 58, 59, 60, 61, 62, 63 (8) | G5(lane 클릭·extras·diff 위젯) | `APP/git-blame.rs`, `APP/git-gutter.rs`, `APP/git-conflict.rs`, `UI/editor_surface.rs`(line_end_texts), `UI/editor-context-menu.rs`(toggleBlame·stage 항목), `UI/command-registry.rs`(`ToggleBlame`·`StageSelection`) |

합계 8+8+7+6+9+10+8+8 = 64.

### 3.2 두 하위 lane 병렬 판단

가능합니다. 조건은 G1을 먼저 끝내 lane 브랜치에 병합한 뒤 나누는 것입니다.

- 하위 lane A(SCM 패널): G2 → G3 → G4 → G6. 소유: `git-scm*.rs`, `git-branch.rs`, `git-commit.rs`, `git-graph.rs`, `git-commit-detail.rs`.
- 하위 lane B(diff·이력·편집기): G5 → G7 → G8. 소유: `git-diff.rs`, `git-claude-diff.rs`, `git-history.rs`, `git-blame.rs`, `git-gutter.rs`, `git-conflict.rs`, `UI/diff-view.rs`, editor crate·`editor_surface.rs`·`editor-context-menu.rs` 추가분, `CR/taide-git` 추가분, `DocumentKey` match 9곳.
- 공유 파일과 충돌 방지:
  - `APP/domains/git.rs`(`Command`/`Reply`/`State`/`apply_git_reply`/`apply_git_intent`): G1에서 A 블록·B 블록 자리를 나눠 두고 각자 자기 블록 끝에만 추가합니다.
  - `UI/intents/git.rs`: G1에서 Intent 변형을 모두 선언합니다. 아직 구현하지 않은 변형은 그 묶음 전까지 어떤 키·메뉴·명령도 내지 않습니다(연결은 해당 묶음에서).
  - `application.rs`·`host.rs`·`UI/commands.rs`: G1에서 모든 호출 지점(사이드바 본문, 탭 내용, 편집기 레이어·footer, 대화상자, PaintSink 기록, host 루트)을 만들고 이후 두 하위 lane은 이 파일을 고치지 않습니다.
  - `APP/git-cache.rs`: G1 이후 키 추가는 각 하위 lane 블록에만.
  - `UI/command-registry.rs`: `GitRun` 변형을 G1에서 모두 선언, 실행 연결은 A(`RevertHead`·`CreateTagOnHead`)와 B(`ToggleBlame`·`OpenFileHistory`·`StageSelection`)가 서로 다른 match 분기에서.
  - `UI/shell.rs` 탭 메뉴: B만(git-48·55).
  - 아이콘 표(`glyph-icons.rs`·`navigation-icons.rs`)와 fixture: G1에서 전부 추가, 이후 읽기 전용.
- 비용: worktree·target이 2벌이라 디스크 측정치 2배가 필요합니다(운영 문서 2.2 공식). wave 2에서 settings lane과 동시이므로 동시 lane 수가 3이 됩니다. 디스크가 모자라면 3.1 순서대로 직렬 진행합니다.
- 의존 주의: A의 git-32(커밋 diff 탭 열기)는 B의 git-47(내용)과 독립 판정(탭 kind·제목만). B의 git-62 정착은 편집기 저장 경로(wave 1 editor lane 완료분)를 씁니다. git-7 알림은 settings lane 병합에 의존합니다.

### 3.3 묶음별 최소 검사

각 묶음은 그 묶음에서 바꾼 crate만: 앱 lib `--lib -- git --test-threads=4`(필터 `git`·`title_branch`·`explorer`), UI를 바꾼 묶음은 해당 `tests/*.rs`, `CR/taide-git`을 바꾼 G5는 `cargo test --manifest-path crates/taide-git/Cargo.toml diff_hunks`, UI·editor crate를 바꾼 G1·G5·G7·G8은 remote-web Wasm check, 바꾼 crate별 `cargo fmt --check`. lane 끝 리뷰 전 1회 전체 게이트는 운영 문서 2.3을 따릅니다.

---

## 4. 결정·허락

### 4.1 사용자 결정이 필요한 항목 (추천안 먼저)

1. **git-15(원본 화면 없음: amend·fetch·branch delete·undo)** — **A. not-applicable로 정리(추천: 새 기능 금지 원칙, 원본 사용자 표면 없음)** / B. unwired 유지.
2. **커밋 초안 기억 범위(git-12)** — **A. 창별 분리(추천: 원본은 창마다 별도 realm)** / B. 앱 전체 공유.
3. **createTagOnHead의 원본 결함(그래프가 안 보이면 요청 유실, slot마다 대화상자)** — **A. 재현하지 않고 focus slot에 하나만, 그래프 접힘과 무관하게 표시(추천)** / B. 원본대로.
4. **monaco 기본 장식 재현 범위(git-40·50)** — **A. 900px 자동 inline·분할선 끌기·filler 빗금·± 표시·문자 단위 강조·overview ruler(기존 `DecorationKind::Overview` 재사용)·ClaudeDiff 수정 쪽 revert 화살표 포함(추천: 원본에서 보이고 누를 수 있는 것은 모두, native에 기반이 이미 있음)** / B. ClaudeDiff revert 화살표 제외 / C. 줄 배경·filler만.
5. **Alt+\ 적용 범위(git-41)** — **A. focus pane의 diff 탭만(추천: 원본 창 전역 리스너가 보이는 모든 diff를 함께 바꾸는 것은 결함으로 봄)** / B. 원본대로 창의 모든 diff 탭.
6. **showMoves(이동 표시, git-42)** — **A. monaco `computeMovedLines`(`node_modules/monaco-editor/esm/vs/editor/common/diff/defaultLinesDiffComputer/computeMovedLines.js`) 규칙을 libgit2 hunk 위에 이식(추천: 설정 UI가 이미 complete라 소비자가 필요)** / B. 미반영(git-42 partial 유지).
7. **diff 줄 계산 기준(git-40·43)** — **A. libgit2 hunk(`diff_hunks`, 추천: stage 백엔드와 같은 경계라 hunk stage가 항상 맞음)** / B. monaco advanced 알고리즘 이식(시각 일치 우선, 이식량 큼).

explorer-search 사양 D1~D3, shell 사양 결정 A(provider 소비자 소유), settings 사양(소비자 미구현 방침)과의 경계는 각 사양의 추천안을 전제로 썼습니다(1.2). 그 결정이 다른 안으로 나면 2.0.3·git-38·git-45·git-49·git-55의 소유 문단만 바꿉니다.

### 4.2 허락이 필요한 항목 (자동 검사에서는 하지 않음)

1. 실제 OS 알림 발송(git-7 push/pull) — 가짜 platform으로만 검사합니다.
2. 추적 안 된 파일 discard(git-20/22/23) — 백엔드가 사용자 휴지통으로 옮깁니다. 자동 검사는 추적 파일만, 실제 확인은 허락 필요.
3. 실제 원격 서버 push/pull(사용자 자격 증명) — 로컬 bare 원격만 씁니다.
4. 실제 AI provider 호출과 Keychain 토큰(git-14) — 가짜 포트만 씁니다.
5. Finder reveal·실제 clipboard(git-23) — intent만 단언합니다.
6. `git init`은 git CLI라 사용자 전역 설정(template 등)을 읽습니다(git-2). UI 검사는 가짜 init 포트, runtime 근거는 기존 remote-git 검사 재사용. 실제 신규 초기화 검사는 허락 필요.
7. 커밋·push·pull의 git CLI는 사용자 전역 설정을 읽기 전용으로 참조합니다(fixture local config가 hook·서명·자격 증명을 덮음). 기존 `APP/remote-git-tests.rs`와 같은 수준이며, 추가 허락 없이 진행해도 되는지 확인이 필요합니다.
