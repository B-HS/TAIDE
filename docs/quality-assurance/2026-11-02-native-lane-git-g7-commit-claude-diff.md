# native lane git 묶음 7/8 (g7-commit-claude-diff) — 커밋 diff·Claude diff·파일 이력

> 작성일: 2026-11-03 · worktree `TAIDE-lanes/git`(branch `lane/git`) · cargo target `TAIDE-lanes/targets/git`
> 사양 `docs/research/2026-10-28-lane-git-spec.md` 2.0.3·2.0.4·git-47·49~55, 3.1 G7. 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안), `docs/acknowledge/2026-10-26-native-autonomous-continuation.md`

## 1. 대상 ID와 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| git-47 | complete | 커밋 파일 diff 탭 본문: parent(before_path) 대 rev(path), NotFound 빈 쪽, parent 없음 빈 원본, 그 밖 오류 `editor.diffLoadFailed`, 경로 기반 언어, side-by-side 고정(Alt+\ 없음), 숨김 설정 반영, 불변 캐시 |
| git-49 | complete | IDE `openDiff` → 수신함 → 대기 목록 → ClaudeDiff 탭(제목 tab_name, preview false, target None), `ide_auto_open_diff` false면 탭 없이 Rejected |
| git-50 | partial | 32px 막대 + diff(원본 디스크 파일 읽기 전용, 수정 요청 내용 편집 가능), 언어 동기화, `ignoreTrimWhitespace` true, 편집 뒤 200ms 재계산. 수정 쪽 revert 화살표(결정 4 A) 남음 |
| git-51 | complete | Accept(수정 쪽 현재 텍스트로 saved)/Reject(rejected), 진행 중 비활성·중복 무시, 성공 toast 뒤 탭 닫기, 실패 오류 toast |
| git-52 | partial | 탭 닫기 → 백엔드 TabClosed, IDE close_tab → 대기 목록 제거, 표면 해제 때 응답 없음, 다른 창 트리에 탭이 있으면 대기 유지. 보조 창 이동 App 검사 남음 |
| git-53 | complete | 창별 우측 도킹 dialog(폭 384, 왼쪽 모서리 8px, scrim, 바깥 클릭·Escape 닫힘), 머리말 36px(뒤로·제목·닫기 tooltip), 목록 오류·로딩·빈 문구, 40px 항목(요약 / 작성자·상대 시간·7자 해시) |
| git-54 | complete | 항목 선택 → dialog 본문을 inline 커밋 diff로 교체(owner `TabId::new()`), 머리말 제목 = 파일명, 뒤로 → 목록, 목록·닫힘에서 가상 문서 해제 |
| git-55 | partial | 탭 메뉴 History, 탐색기 파일 메뉴 History, 명령 `git.openFileHistory`(활성 파일 편집기 없으면 무동작). 편집기 우클릭 `9_git` 항목 남음 |
| explorer-search-23 | complete | 탐색기 파일 메뉴 "비교 위해 선택/선택 항목과 비교" 다음 `git.fileHistory` → 그 창 dialog |
| shell-89 | partial | diff(G5)·commit diff(git-47)·claudeDiff(git-50) 탭 본문 연결. searchEditor 본문은 explorer-search lane, ClaudeDiff revert 화살표는 git-50 남은 범위 |

## 2. 원본 근거

사양 각 절을 기준으로 아래 구간만 다시 확인했습니다.

- `src/widgets/commit-file-diff/commit-file-diff.tsx`: 두 `gitShowFileQueryOptions`(parent 없으면 조회 안 함), `NotFound`만 빈 쪽, 그 밖 오류 문구, 둘 다 준비 전 빈 배경, `getLanguageIdFromPath(path)`, `resolveDiffViewSettingsProps`.
- `src/widgets/editor-area/pane-node-view.tsx:203-232`: `rev != null`이면 `CommitFileDiff`(`beforePath ?? path`, `renderSideBySide`), ClaudeDiff는 `ClaudeDiffPane`.
- `src/widgets/claude-diff-pane/claude-diff-pane.tsx` 전체: 수정 모델은 마운트 때 대기 목록 내용(없으면 빈 문자열), 원본은 `fileQueryOptions(path)`(retry 없음, 실패면 빈 문자열), 언어 `file.languageId ?? 'plaintext'` 양쪽 동기화, 막대 `h-8 border-b bg-editor-background justify-end gap-2 px-2`, Reject outline xs → Accept default xs, 진행 중 비활성, 성공 뒤 `removePendingClaudeDiff`·toast·`closeTab`, 언마운트 때 모든 창 트리에서 탭을 찾고 없을 때만 rejected.
- `src/app/providers/ide-sync-provider.tsx:49-68,131-133`, `src/entities/ide/claude-diff-registry.ts`.
- `src/widgets/file-history/file-history-panel.tsx` 전체, `src/features/explorer/file-tree-context-menu.tsx:115-145`, `src/features/tab/tab-context-menu.tsx:120-148`, `src/entities/git/git.commands.ts`(`git.openFileHistory` = focus 편집기 동작), `src/shared/ui/dialog.tsx`(scrim·`modal.background`·`modal.border`), `src/shared/ui/button.tsx`·`src/shared/styles/global.css`(primary = `button.primaryBackground`, outline hover = `list.hoverBackground`).
- native: `native/taide-native-app/src/ide-tools.rs`(`open_diff`·`close_tab`), `src/tabs.rs`(ClaudeDiff 닫기 → `TabClosed`), `crates/taide-runtime/src/ide_actions.rs::ide_resolve_diff`, `crates/taide-git/src/service.rs`(`show_file`의 `NotFound`, `map_git_err`는 없는 rev를 Internal로).

## 3. 구현

### 3.1 공통

- 캐시(`git-cache.rs`): `Key::CommitDiff { rev, parent, path, before_path }`(불변, 시간 stale 없음), `Key::FileLog { path }`(가변, worktree 변경 무관), `Key::File { path }`(저장소 변경 무관, 그 경로 fs 변경만), `Value::FileLog`·`Value::File`, `file_log()`·`file()`.
- diff 표면(`domains/git.rs`): G5 `show_diff`를 `show_surface(ui, Parts, Surface, Colors)`로 나눠 diff 탭·커밋 diff·ClaudeDiff·파일 이력이 함께 씁니다. 한쪽 텍스트 정책 `SideText::Read { text, fresh }`(새 응답이면 `replace_virtual`) / `SideText::Editable(text)`(처음 한 번 편집 가능하게 열고 이후 언어만 맞춤). 응답 반영 여부 `take_fresh`, 로딩·실패 화면 `loaded_diff`.
- 실제 앱 경로: `main.rs` → `NativeApplication` → 탭 내용(`domains/editor.rs` 탭 분기) → `show_diff_tab`/`show_claude_diff_tab`, 프레임 `show_window` → `show_git_file_history`, 매 프레임 `tick_git_title_branches` → `tick_git_cache` → `tick_claude_diffs`. git 명령은 모두 `spawn_transient`(host worker 비차단).

### 3.2 git-47

`show_diff_tab`이 `rev`가 있으면 `Key::CommitDiff`를 관찰·표시합니다(`Intent::DiffShown`에 `rev`·`parent_rev` 추가). 읽기는 `git_show_file`을 parent(before_path)·rev(path) 순으로 부르고 `AppErrorKind::NotFound`는 빈 문자열, parent 없음은 빈 원본, 언어는 `taide_infra::language::language_id_for_path(path, overlays)`, hunk는 `ignore_trim_whitespace: false`. 커밋 diff는 side-by-side 고정(Alt+\ 소비 안 함, 900px 자동 inline은 위젯 공통), stage lane 없음.

### 3.3 git-49~52

- 수신함: shell lane의 `provider_events::Inbox`가 이 worktree에 없어 사양 1.2의 대안대로 git 자체 기록을 씁니다. `git_cache::Changes`에 `ide: git_claude_diff::Inbox`를 두고 `record`에서 `IdeDiffRequested`·`IdeCloseTabRequested { request_id: Some }`를 모읍니다(`PaintSink`·`application.rs` 생성자 변경 없음). shell lane 병합 때 그 수신함 분기로 옮기면 됩니다.
- `consume_ide_diff_events`: 요청 → 대기 목록 기록 → 설정 `ide_auto_open_diff` false면 지우고 `Command::ResolveIdeDiff { tab: None, Rejected }`(toast 없음, 원본처럼 실패만 오류 toast), 아니면 `Command::OpenTab { kind: ClaudeDiff { request_id, path }, title, target None, preview false }`. close-tab 알림 → 대기 목록 제거.
- 표면(`show_claude_diff_tab`): 32px 막대(`git_claude_diff::bar`, Reject outline·Accept primary, 24px 높이, 좌우 8px, 6px 모서리, 12px 글자, 진행 중 50% 불투명·클릭 없음) + `show_surface`(원본 `Key::File` 읽기, 실패·로딩이면 빈 문자열과 `plaintext`; 수정 `SideText::Editable(대기 목록 내용)`; `hide_unchanged: false`).
- 줄 diff: `Intent::ClaudeDiffShown`이 두 문서 revision을 알리고 `git_claude_diff::State::shown`이 처음은 즉시, 이후 마지막 변경에서 200ms 뒤로 예약합니다. `tick_claude_diffs`가 `Command::Hunks`(`diff_hunks(.., true)`, spawn_blocking)를 내고, 응답 revision이 현재 문서와 다르면 버립니다.
- Accept/Reject: `Intent::ResolveClaudeDiff`(Accept면 수정 문서 현재 텍스트) → `begin_resolve`(진행 중이면 무시) → `Command::ResolveIdeDiff` → `Reply::IdeDiffResolved` → 성공이면 대기 목록 제거, `ide.diffAccepted` success / `ide.diffRejected` info, `request_tab_close`; 실패면 `Toasts::ipc_error`.
- 탭 닫힘·이동: 표면 해제 때 응답을 보내지 않습니다(사양 git-52 설계). 사용자가 탭을 닫으면 기존 `tabs.rs` 경로가 `TabClosed`를 보냅니다. 대기 목록은 그려진 적 있는 요청의 탭이 어느 창 트리에도 없을 때만 지웁니다(`collect_claude_diff_tab_ids` 전 창, `takePendingClaudeDiffIfUnresolved`와 같은 정리). 탭 전환으로 숨으면 원본처럼 가상 문서를 해제하고 다시 보이면 요청 내용으로 새로 엽니다.

### 3.4 git-53~55, explorer-search-23

- `git-history.rs`: `Panels`(창 `ViewportId`별 `Panel { project, path, selected, owner, pane }`), `modal`(오른쪽 위 고정, `modal::Chrome` 테두리·그림자, 왼쪽만 8px, scrim `Chrome::scrim`), `header`(뒤로 ArrowLeft·닫기 X 20px 단추, 14px 아이콘, hover `explorer.itemHover`, tooltip `git.fileHistory`/`common.close`, 제목 12px medium 말줄임), `list`(스크롤, 알림 12px, 항목 40px 두 줄 12px/10px, 상대 시간 `git_graph::relative_time`, 7자 해시 mono).
- `show_git_file_history`(`application.rs` `show_window`에 호출 한 줄): 열린 창의 `Key::FileLog`를 관찰하고, 선택이 있으면 `Key::CommitDiff`(before_path = path)를 관찰·`diffs.shown(owner)`로 문서를 유지하며 inline(`side_by_side: false`)으로 그립니다. 바깥 클릭·Escape(`modal::takes_escape`)는 닫기, 닫기·뒤로는 상태만 바꾸고 다음 tick이 숨은 문서를 해제합니다. 창이 닫히거나 프로젝트가 닫히면 dialog도 지웁니다.
- 진입점: UI `shell.rs` 파일 탭 메뉴에서 Open Changes 다음 `git.fileHistory`(→ `Intent::OpenFileHistory`), 탐색기 파일 메뉴 `explorer::Action::FileHistory` → `domains::explorer_search::explorer`에서 같은 intent, 명령 `git.openFileHistory` → `Run::Git(GitRun::OpenFileHistory)`(Enablement Always) → `shell_keymap`이 활성 창 focus pane의 활성 탭이 파일 탭일 때만 intent. 요청은 그 intent를 적용한 창의 dialog로 갑니다(원본은 slot마다 dialog가 있고 focus slot만 받지만 dialog가 창 전체 오른쪽에 뜨므로 창당 하나와 같음).
- 아이콘: `resources/icons/arrow-left.svg`(Lucide iconNode), `navigation_icons::Icon::ArrowLeft`.

### 3.5 범위 밖 최소 수정과 사양 차이

- `domains/editor.rs`(editor 영역) 탭 분기 한 줄: `show_claude_diff_tab(ui, tab)` → `(ui, project, pane, tab, intents)`. 표면이 프로젝트·pane·intent를 써야 해서입니다.
- `domains/explorer_search.rs`·`explorer.rs`(explorer-search 영역): `Action::FileHistory` 한 변형, 메뉴 단추, intent 변환 블록(D2 A로 이 lane 몫).
- `application.rs`: `show_window`에 `self.show_git_file_history(&context);` 한 줄, git_state 리터럴 끝에 네 필드. 창별 dialog는 다른 git 호출 지점이 없어 seams 1.4의 "새 소비 시점이 꼭 필요할 때만 한 줄" 규칙을 따랐습니다.
- UI `tests/workbench.rs`: G5 때 "History 항목 없음"을 단언하던 탭 메뉴 검사를 원본 순서(Open Changes < History < Reopen With)와 History intent 단언으로 갱신.
- 사양 2.0.2의 `Show`·`FileLog`·`ReadFile`·`Hunks { key, .. }` 명령 대신 공용 캐시 `Command::Read`의 `Key::CommitDiff`·`Key::FileLog`·`Key::File`과 ClaudeDiff 전용 `Command::Hunks { tab, revisions, .. }`를 씁니다. 사양 2.0.1의 `git-claude-diff.rs`·`git-history.rs` 배치는 그대로입니다.
- 탭 메뉴·탐색기 메뉴의 History 아이콘은 그리지 않았습니다. native 탭 메뉴·탐색기 메뉴는 원본 아이콘(FolderOpen, FileDiff 등)을 다른 항목에도 그리지 않는 기존 형태라 같은 형태를 따랐습니다.

## 4. 검증

모든 cargo 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/git`.

| 명령 | 결과 |
| --- | --- |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- claude_history_tests git_claude_diff git_cache --test-threads=2` | 14 passed(App 4, `[C]` claude 3, 캐시 7 중 신규 1) |
| 같은 crate `--lib -- claude_history_tests::실제_app_파일_이력 --test-threads=2`(탐색기 경로 추가 뒤 재실행) | 1 passed. 첫 실행은 실패: 픽스처 프로젝트 루트가 `/var`(정규화 전) 경로라 탐색기 행 경로가 정규화된 기대값과 달랐고 그 경로의 `git_file_log`가 `pathOutsideRepository`였습니다. 단언을 정규화 비교로 바꾸고 빈 이력 문구는 정규화 경로 파일 탭의 명령 경로로 확인 |
| 같은 crate `--lib -- git shell_keymap command_dispatch title_branch navigation_icons --test-threads=2` | 103 passed(G1~G6 git App 검사, G5 diff 탭 회귀 포함) |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --test explorer -- --test-threads=2` | 21 passed |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test workbench -- --test-threads=2` | 45 passed |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib -- command_registry --test-threads=2` | 15 passed(`git.openFileHistory` 실행값·Always) |
| `cargo check --quiet --all-targets --manifest-path native/taide-native-app/Cargo.toml` | 성공(경고는 기존 vendor·`editor-code-actions.rs`·`zen.rs`·`application-inlay-hints-tests.rs`) |
| `cargo check --quiet --all-targets --features native-host,inspection --manifest-path native/taide-native-ui/Cargo.toml` | 성공(경고는 기존 `tests/workbench.rs:1816`) |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 성공(동결 crate 기존 경고만) |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml`, UI crate `-- --check` | app은 이 묶음 파일만 정리, UI 차이 없음. 정리 뒤 app `-- --check` 차이 없음 |
| 정리 뒤 같은 crate `--lib -- claude_history_tests git_claude_diff git_cache diff_tests --test-threads=2` | 21 passed(G5 diff 탭 7 포함) |

App 검사(`application-git-claude-history-tests.rs`):

- `실제_app_커밋_diff_탭은_수정_추가_삭제_최초_커밋을_그리고_rev_오류는_실패_문구를_그린다`: 수정(a), 추가(b: 원본 빈), 삭제(c: 수정 빈), 최초 커밋(parent 없음) 네 탭의 두 가상 문서 내용, 저장소 무효화 뒤 `fetched_at` 불변(불변 범위), 없는 rev → `editor.diffLoadFailed`.
- `실제_app_ide_open_diff는_claude_diff_탭을_열고_수정_쪽_편집_뒤_accept로_저장한다`: 실제 `ide_tools::dispatch("openDiff")` → ClaudeDiff 탭(제목, preview false), 공백만 다른 1행은 hunk 없음(3행만), 원본 쪽 입력 거부, 수정 쪽 입력 → 200ms 뒤 hunk 2~3행으로 재계산, Accept → IDE 응답 Saved, 파일 = 편집 결과, success toast, 탭 닫힘, 대기 목록 비움.
- `실제_app_claude_diff는_자동_열기_끄기_reject_탭_닫기_ide_close_tab을_원본대로_처리한다`: 설정 false → 탭 없이 Rejected, Reject → Rejected·파일 불변·info toast·탭 닫힘, `request_tab_close` → TabClosed, IDE `close_tab` 도구 → TAB_CLOSED·대기 응답 TabClosed·대기 목록 제거.
- `실제_app_파일_이력은_명령_탭_메뉴로_열리고_항목을_inline_diff로_보여준다`: 편집기 없을 때 명령 무동작, 파일 탭에서 명령 → 커밋 3개 항목, 항목 클릭 → inline diff 두 문서 내용, 뒤로 단추(tooltip 이름) → 목록·문서 해제, Escape 닫힘, 탭 메뉴 History → 열림, 탐색기 파일 메뉴 History → 그 파일 dialog, 추적 안 된 파일 → `git.noFileHistory`.

금지 확인: 실제 원격·AI·Keychain·클립보드·휴지통·OS 알림·GUI 실행 없음. IDE 도구는 테스트 런타임 안에서 앱 서비스로만 호출했습니다.

## 5. 남은 범위

- git-50: ClaudeDiff 수정 쪽 hunk revert 화살표(결정 4 A). git-40의 문자 단위 강조·±·overview ruler도 ClaudeDiff에 함께 남습니다(diff 위젯 공통).
- git-52: 탭을 보조 창으로 옮긴 뒤 응답 없음·보조 창 Accept의 실제 App 검사. 코드 경로는 창 무관(탭 내용 분기, 전 창 트리 검사 단위 검사)입니다.
- git-55: 편집기 우클릭 `9_git` 그룹의 "File History"(`9_cutcopypaste`와 `9_terminal` 사이, peek 제외). editor 영역 `UI/editor-context-menu.rs`·`APP/editor-context-menu.rs`와 그 영역 검사(`application-context-menu-tests.rs`의 그룹 목록 단언)를 함께 바꿔야 해서 G8(편집기 메뉴 toggleBlame·Stage Changes 추가)과 묶어 처리하는 것이 맞습니다. 키바인딩 id `monaco.taide.openFileHistory` 실행 연결도 그때 함께(commands-ui-39).
- shell-89: searchEditor 탭 본문(explorer-search lane).
- 리뷰 수정(2026-11-03) git-49: 수신은 shell lane `provider_events::Inbox`가 아니라 사양이 허용한 대체 경로(`git_cache::Changes.ide`)입니다. complete 판정은 이 대체 경로 기준이며 병합 뒤 이전이 남은 과제라는 점을 대응표 행에 적었습니다. `.json` 파일 ClaudeDiff의 내장 JSON 서비스 오류 toast 결함은 g5 문서 6.1에서 함께 고쳤습니다(필터가 모든 가상 문서를 제외, `.json` App 검사는 작업 트리 diff 탭 1건).
- 병합 메모: shell lane의 `provider_events::Inbox`가 병합되면 `git_cache::Changes.ide` 기록을 그 수신함 분기로 옮깁니다. terminal-agent-68(Claude diff 탭)·69(ideAutoOpenDiff 소비자)·settings-80·shell-92 diff 부분 행은 이 문서를 근거로 각 행 소유 lane 병합 때 갱신합니다(사양 1.1의 terminal-agent-69 정정은 git-49로 실제 소비자가 생겨 complete가 맞게 됨).
