# Native 전환 감사: Git 패널·히스토리·diff 뷰

감사일 2026-10-06, 읽기 전용. 이전 에이전트의 HANDOFF/PROCESS 수치는 신뢰하지 않고 실제 파일과 호출 체인으로 판정했습니다. 빌드·테스트는 실행하지 않았습니다.

## 결론

- native UI(`/Users/hyunseokbyun/development/TAIDE/native` egui 앱)에는 Git 화면이 하나도 없습니다. SCM 패널, 브랜치 스위처, 커밋 그래프, diff 탭, 파일 히스토리, blame, gutter, 충돌 해결 UI가 전부 없고 TS 쪽만 존재합니다.
- 있는 것은 백엔드뿐입니다. `crates/taide-git`(libgit2+git CLI), `crates/taide-runtime/src/git_actions.rs`(41개 액션), 그리고 이를 원격 JSON 게이트웨이에 노출하는 `native/taide-native-app/src/remote-git.rs`입니다. `git_actions::`를 호출하는 곳은 `remote-git.rs`뿐이며 egui UI 어디서도 호출하지 않습니다. 즉 "연결됨"은 원격(WebSocket JSON) 클라이언트 경로 한정이고 native 화면 경로에서는 도달 불가입니다.
- 이전 에이전트가 적은 "Git41명령 domain adapter 연결"(HANDOFF.md 386행)은 이 원격 경로 얘기이며 UI 전환 진척이 아닙니다.
- TS 기능 65행 기준 판정은 done 0, partial 3, unwired 33(백엔드만 존재), missing 28, n/a 1입니다.
- 가장 큰 구조적 공백은 세 가지입니다. (1) 사이드바가 Explorer 단일 뷰라 SCM 뷰 진입점이 없음, (2) 텍스트 diff 엔진과 diff 렌더러가 native에 없음, (3) `taide-native-ui`의 에디터 표면(`editor_surface.rs`)에 gutter/인라인 데코레이션 확장점이 없어 blame·git gutter·충돌 표시를 얹을 곳이 없음.

## 범위

TS 기준(읽은 파일, 테스트 제외):

- `/Users/hyunseokbyun/development/TAIDE/src/widgets/git-panel/{git-panel,git-panel-container,commit-graph,commit-detail-panel,commit-gate,git-sections,change-row-navigation,ai-commit-message}`
- `/Users/hyunseokbyun/development/TAIDE/src/widgets/{file-history/file-history-panel,diff-pane/diff-pane,diff-pane/diff-stageability,diff-pane/diff-hunk-range,commit-file-diff/commit-file-diff,claude-diff-pane/claude-diff-pane}`
- `/Users/hyunseokbyun/development/TAIDE/src/features/git/*`(branch-switcher, branch-group, commit-box, stash-list, status-row-item, git-change-group, git-section-header, create-tag-dialog, diff-view, hunk-discard-dialog, conflict-resolution-dialog, conflict-compare-dialog, conflict-marker)
- `/Users/hyunseokbyun/development/TAIDE/src/entities/git/*`
- 범위 밖이나 Git 기능에 직결되어 추가로 읽은 TS: `src/widgets/editor-pane/use-editor-blame.ts`, `use-editor-git-gutter-and-conflicts.ts`, `src/widgets/explorer/file-tree-git-status.ts`, `src/widgets/window-chrome/title-bar-content.tsx`, `src/widgets/editor-area/pane-tab-bar.tsx`(Open Changes/File History), `src/widgets/editor-area/pane-node-view.tsx`(diff 탭 분기)
- inventory: `docs/quality-assurance/2026-09-28-ts-view-inventory.md`, `2026-09-28-ts-overlay-inventory.md`, `2026-09-29-ts-feature-inventory-a.md`

native 기준(읽은 파일):

- `native/taide-native-app/src/{remote-git.rs, remote-git-tests.rs(목록·구조), application.rs(4605-5150 ShellSurfaces 구현), bootstrap.rs, event-relay.rs, projects.rs, tabs.rs, ide-tools.rs, shell_keymap.rs, explorer.rs(show 시그니처), problems-icons.rs}`
- `native/taide-native-ui/src/{shell.rs, editor_surface.rs, settings-controls.rs, settings-code-controls.rs, keybinding-commands.json, keymap-defaults.json, theme-editor-tokens.rs}`
- `crates/taide-git/src/{service.rs, store.rs}`, `crates/taide-runtime/src/git_actions.rs`, `crates/taide-model/src/{git.rs, layout.rs}`, `crates/taide-remote/src/command-policy.rs`, `crates/taide-locale/resources/locales/*.json`, `crates/taide-settings/src/service.rs`

missing 판정 전 사용한 검색어(native 전체, target/vendor 제외): `Git[A-Za-z_]`, `git_`, `\bgit\b`, `taide_git`, `GitService`, `scm`, `stash`, `blame`, `hunk`, `diff`, `commit_graph`/`CommitGraph`, `branch`, `conflict`, `gutter`, `<<<<<<<`, `ConflictRegion`, `conflict_marker`, `git.revertHead`/`git.createTagOnHead`/`git.toggleBlame`/`git.openFileHistory`, `git_sections_collapsed`/`git_graph_panel`, `hide_unchanged`/`show_moves`, `ClaudeDiff`/`claude_diff`/`IdeDiffRequested`/`ide_resolve_diff`, `notify_git_remote`, `similar|imara|diffy|dissimilar`(diff 엔진 크레이트), `computeGraphLanes|graph_lanes|relative_time|tags_targeting`, `git.renamed|git.conflicted|git.staged`, `decor|status_by`(explorer). 결과: native UI 코드에서 Git 화면 구현에 해당하는 히트 0건. 히트는 아래 "끊긴 지점" 목록의 항목뿐입니다.

## 기능 대응표

상태 표기 규칙: unwired는 "백엔드(taide-git/git_actions/remote-git)는 있으나 native UI에서 도달 불가"를 뜻합니다. 모든 unwired 행의 "백엔드 근거"는 `crates/taide-runtime/src/git_actions.rs`의 해당 함수와 `native/taide-native-app/src/remote-git.rs`의 원격 arm이며, 원격 JSON 클라이언트에서만 호출됩니다. 별도 표기가 없으면 effort는 UI 구현량 기준입니다.

| # | 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
|---|------|---------|-------------|-------------|---------|--------|
| 1 | SCM 뷰 진입: Explorer 4뷰(files/search/git/outline) 전환, 단축키 ctrl+shift+g, command의 requestShowExplorerView('git') | `src/widgets/explorer/explorer-panel.tsx:259`, `src/entities/git/git.commands.ts:21-24` | missing | 사이드바는 Explorer 단일 패널(`native/taide-native-ui/src/shell.rs:545-553` `surfaces.explorer`). `keymap-defaults.json:57-60`에 `git` 단축키 데이터만 있고 `shell_keymap.rs`는 git 액션 미지원 | 뷰 전환 UI·상태, git 뷰 라우팅 전체 | M |
| 2 | 비저장소 안내 + 저장소 초기화 버튼 | `git-panel-container.tsx:247-256,142-143` | unwired | `git_actions::git_init`(git_actions.rs:117), 원격 arm `remote-git.rs:88` | 안내 화면, 버튼, 성공/실패 toast | S |
| 3 | 브랜치 스위처 트리거: 현재 브랜치명 표시, 저장소 없음 라벨, disabled | `src/features/git/branch-switcher.tsx:49-60` | unwired | `git_actions::git_branches`(344), `git_status` | 팝오버, 트리거 버튼, tooltip | S |
| 4 | 브랜치 스위처 목록: 필터 입력, 로컬/원격 그룹, 체크아웃, 원격 체크아웃 | `branch-switcher.tsx:62-79`, `branch-group.tsx` | unwired | `git_branch_checkout`(372), `git_checkout_remote_branch`(673) | 검색 가능한 리스트 위젯, 키보드 선택, 성공 toast(`git.branchSwitched`) | M |
| 5 | 새 브랜치 만들기(필터어 기반, exact match 없을 때, 생성 즉시 checkout) | `branch-switcher.tsx:32-42,65-72`, `git-panel-container.tsx:134-138` | unwired | `git_branch_create`(352) | 생성 항목 UI | S |
| 6 | ahead/behind 표시, remote 이름 표시 | `git-panel.tsx:505-516,536` | unwired | `GitStatus.ahead/behind/has_remote`(taide-model git.rs), `git_remotes`(209) | 헤더 렌더 | S |
| 7 | Sync 버튼(pull 성공 후 push, 진행 스피너, 오류 toast) | `git-panel.tsx:526-535`, `git-panel-container.tsx:206` | unwired | `git_pull`(312), `git_push`(300). 원격 arm 존재. fetch는 TS UI 미사용 | 버튼, 직렬 실행·오류 toast | S |
| 8 | Stash push 버튼(변경 있을 때만, 성공 toast) | `git-panel.tsx:517-525`, `git-panel-container.tsx:109-113` | unwired | `git_stash_push`(414) | 버튼, 활성 조건 | S |
| 9 | Stash 섹션: 목록, Apply/Drop, 접기, 건수 배지, 비어 있으면 섹션 숨김 | `src/features/git/stash-list.tsx`, `git-panel.tsx:632-644` | unwired | `git_stash_list`(406), `git_stash_apply`(430), `git_stash_drop`(441) | 섹션 UI, 처리 중 비활성 | S |
| 10 | 커밋 박스: 3행 메시지 입력, 커밋 버튼, committing 상태, 성공 시 입력 초기화, blockedReason 표시 | `src/features/git/commit-box.tsx`, `git-panel-container.tsx:150-157` | unwired | `git_actions::git_commit`(282, CommitOptions{amend,stage_all}) | 박스 UI(멀티라인 입력 위젯), 상태 | M |
| 11 | Mod+Enter 커밋 단축키, IME 조합 중 무시 | `commit-box.tsx:41-47` | missing | native 코드에 대응 없음(`commit`/`git` 검색 히트 없음) | 키 처리, IME 가드(egui 입력 경로 필요) | S |
| 12 | 프로젝트별 커밋 메시지 초안 기억(최대 8개, 전환 시 재로드, 빈 문자열이면 삭제) | `src/entities/git/commit-message-memory.ts`, `git-panel-container.tsx:56-81` | missing | 대응 로직 없음 | 초안 저장소(앱 메모리), 프로젝트 전환 연동 | S |
| 13 | 커밋 게이트: 충돌 있으면 차단 + 사유 문구, staged 없고 unstaged 있으면 stage-all 확인 다이얼로그, `stageAll` 옵션 전달 | `commit-gate.ts`, `git-panel.tsx:335-348,714-725` | missing | 백엔드 옵션 `stage_all`(service.rs:250-253)만 존재. 게이트 판정·확인 다이얼로그 Rust 없음 | `resolve_commit_gate`, 확인 다이얼로그 | S |
| 14 | AI 커밋 메시지: 생성/취소 토글, stale 요청 가드, 프로젝트 전환 시 abandon, 코드펜스·따옴표 sanitize, 최근 커밋 20개 요약 변수, fallback/잘림/건너뜀 notice toast | `git-panel-container.tsx:165-245`, `ai-commit-message.ts` | unwired | `ai_actions::ai_commit_message`(ai_actions.rs:132), `git_diff_staged_text`(177), 원격 `remote-ai.rs`. sanitize·요약 빌드 로직은 Rust 없음 | 버튼/상태 UI, 취소 흐름, sanitize, 최근 커밋 요약, notice | M |
| 15 | amend, fetch, branch delete, undo last commit | TS UI 없음(`git.ipc.ts`에 함수만 일부 존재, 화면 사용처 없음) | n/a | 백엔드 존재(`git_fetch`, `git_branch_delete`, `git_undo_last_commit`, `CommitOptions.amend`) | 사용자 지시 "fetch/pull/push, amend"에 대해: TS 동작 100% 재현 목표라 추가하지 않음. 필요하면 신규 기능으로 별도 결정 | - |
| 16 | 변경 그룹 구성: Merge/Staged/Changes 분류, 건수 배지, 비어 있는 그룹 숨김, "No changes" | `git-sections.ts`, `commit-gate.ts`(isStagedRow/isUnstagedRow), `git-panel.tsx:222-229,630` | missing | 분류 로직 Rust 없음. 데이터 `git_status`는 원격 arm에만 | 분류 함수, 그룹 UI | S |
| 17 | 변경 목록 가상 스크롤 + sticky 섹션 헤더(스크롤 시 소속 그룹 표시) | `git-panel.tsx:271-286,563-611`, `change-row-navigation.ts` | missing | 대응 없음 | egui 가상화 리스트, sticky 헤더 | M |
| 18 | 섹션 접힘 상태 영속(merge/staged/changes/stashes/graph, `Settings.gitSectionsCollapsed`) | `git-panel.tsx:216-312`, `entities/git/git-section.ts` | unwired | `crates/taide-settings/src/service.rs:226,301-372` 필드·sanitize 존재. native 소비자 없음 | UI에서 읽기·쓰기 연결 | S |
| 19 | 변경 행 표시: 파일명+디렉터리, 상태 글자(M/A/D/R/U/T/!)와 색, rename 툴팁 `orig → path`, hover/focus 시 액션 노출 | `src/features/git/status-row-item.tsx` | missing | 대응 없음. 테마 키 `git`은 `theme-editor-tokens.rs:117`에 정의만 | 행 위젯, 상태 색 팔레트 소비 | S |
| 20 | 행 액션: Stage / Unstage / Discard / Open File | `src/features/git/git-change-group.tsx:112-157` | unwired | `git_stage`(239), `git_unstage`(250), `git_discard`(266). Open File은 `HostCommand::OpenFileTab` 재사용 가능 | 액션 버튼 UI 및 toast 오류 | S |
| 21 | 그룹 헤더 액션: Stage All / Unstage All | `git-change-group.tsx:97-103,132-134`, `git-section-header.tsx` | unwired | 위와 동일 | 헤더 액션(포커스/호버 시 노출) | S |
| 22 | Discard 확인 다이얼로그(단일 경로명/다건 개수 문구) | `git-panel.tsx:350-354,695-712` | missing | native AlertDialog 대응(예: `delete_dialog.rs`)은 다른 용도 | 다이얼로그 | S |
| 23 | 행 컨텍스트 메뉴: Open File, Open Changes, Stage/Unstage/Discard, Copy Path, Reveal(Merge 그룹은 일부 제외) | `git-change-group.tsx:82-88,116-123,147-155`, `git-panel.tsx:407-417,558-628` | missing | 대응 없음. Copy/Reveal은 `HostCommand::CopyText`/`RevealPath`가 있어 재사용 가능 | 컨텍스트 메뉴 구성, 대상 경로 추적 | S |
| 24 | 행 클릭/키보드 활성 시 Open Changes(staged 여부 구분, 절대경로·rename의 beforePath 전달) | `git-change-group.tsx:42,89,124,156`, `git-panel-container.tsx:208-218` | missing | `TabKind::Diff`(taide-model layout.rs:63)는 있으나 호출자 없음 | diff 탭 오픈 연동 | S |
| 25 | 변경 행 다중 선택(shift 범위, cmd/ctrl 추가) | `git-panel.tsx:425-443`, `shared/lib/list-selection` | missing | 대응 없음 | 선택 모델 | S |
| 26 | 키보드 roving: ArrowUp/Down 항목 이동(헤더→행→stash 헤더→graph 헤더), 헤더 ArrowLeft/Right 접기·펼치기, Enter/Space 활성 | `change-row-navigation.ts`, `git-panel.tsx:382-405`, `git-section-header.tsx:70-90` | missing | 대응 없음 | 포커스 순서 모델, egui 포커스 처리 | M |
| 27 | 커밋 로그 데이터(refs·parents 포함, 초기 `skip=0, take=LOG_PAGE_SIZE`) | `git.query.ts:113-119` | unwired | `git_actions::git_log`(193), `service::log`(541) | UI 소비, 페이지 크기 상수 | S |
| 28 | 그래프 레인 계산(`computeGraphLanes`: lane, edges, passthroughLanes, continuesFromAbove) | `src/shared/lib/graph-lanes.ts` | missing | Rust에 대응 알고리즘 없음(검색: `graph_lanes`, `lane`) | 알고리즘 이식(+테스트) | M |
| 29 | 커밋 그래프 렌더: SVG 레인·노드·곡선 엣지, 12색 레인 팔레트, refs 뱃지, 요약/작성자/상대시간/7자 해시, 가상 스크롤, 선택 하이라이트, 선택 토글 | `commit-graph.tsx:129-244`, `shared/lib/relative-time` | missing | 대응 없음. 상대시간 로케일 키는 `git.timeDaysAgo` 등 존재하나 변환 로직 없음 | egui painter 기반 렌더, 상대시간 토큰 이식, 가상화 | L |
| 30 | 그래프 pane: 변경 목록 하단 수직 분할, 헤더 접기, 드래그 리사이즈, 영속 높이(`gitGraphPanelSizePx`, 기본 240), minSize 미만 드래그 시 접힘으로 기록 | `git-panel.tsx:70,320-333,487-492,648-692` | missing | 설정 필드는 있음(`taide-settings/src/service.rs:219-226,373`)이나 UI 소비 없음 | 리사이즈 가능한 하단 pane | M |
| 31 | 커밋 상세 패널: 선택 커밋 요약/해시/닫기, 변경 파일 목록(상태 글자·색), 로딩/오류 | `commit-detail-panel.tsx` | unwired | `git_commit_files`(592), `service::commit_files`(1169) | 패널 UI, 상태 | S |
| 32 | 상세 패널 파일 클릭 -> 커밋 diff 탭(preview, 제목 `name @ hash`, rename은 origAbsPath를 beforePath로) | `commit-detail-panel.tsx:44-74` | missing | `TabKind::Diff{rev,parent_rev,before_path}`는 모델에만 있음 | 탭 오픈 연동 | S |
| 33 | 그래프 컨텍스트 메뉴 Revert(성공 toast, 충돌이면 warning + 첫 충돌 파일 열기) | `commit-graph.tsx:87-103,216-219` | unwired | `git_revert_commit`(617), `service::revert_commit`(1299) | 메뉴, 후속 처리 | S |
| 34 | 태그 생성: 컨텍스트 메뉴 + CreateTagDialog(이름/메시지, 열 때 초기화, annotated=true, 빈 이름 차단) | `commit-graph.tsx:105-117,220-223,245-251`, `create-tag-dialog.tsx` | unwired | `git_tag_create`(644), `git_tags`(636) | 다이얼로그, 입력 위젯 | S |
| 35 | 태그 삭제 서브메뉴(커밋을 가리키는 태그만 나열, destructive 표시) | `commit-graph.tsx:224-238`, `shared/lib/git-tags.ts` | unwired | `git_tag_delete`(662), `git_tags` | 서브메뉴, `tagsTargetingCommit` 이식 | S |
| 36 | 명령 팔레트/키맵 명령: git.revertHead(HEAD revert), git.createTagOnHead(git 뷰 열고 태그 다이얼로그) | `src/entities/git/git.commands.ts` | unwired | `keybinding-commands.json:1386-1395`에 항목만 있고 실행 핸들러 없음(`shell_keymap.rs`에 git 액션 없음) | 명령 실행 경로, 뷰 전환 브리지 | S |
| 37 | 타이틀바 브랜치 표시(메인·보조 창, 저장소 오류면 숨김) | `window-chrome/title-bar-content.tsx:22-28` | unwired | 렌더 코드는 있음(`shell.rs:177-178,228-259`) 그러나 `ShellSurfaces::branch`가 항상 `None`(`application.rs:4611-4613`) | `git_status` 조회·캐시를 `branch()`에 연결 | S |
| 38 | Explorer 파일 트리 Git 데코레이션: 변경 종류별 색, 폴더로 우선순위 전파(conflicted>added>untracked>renamed>modified>deleted) | `widgets/explorer/file-tree-git-status.ts`, `explorer-container.tsx:65,90` | missing | `explorer.rs` `Explorer::show`는 `page`와 `locale`만 받음(644행), git 입력 없음. `problems-icons.rs`의 `git.*` 키는 파일 아이콘 색 팔레트일 뿐 | 상태 맵 계산 이식, 행 색 적용 | M |
| 39 | Diff 탭(rev 없음) 데이터: workdirVsIndex/indexVsHead, rename beforePath, 원본/수정 본문 + languageId | `diff-pane.tsx:34-59` | unwired | `git_diff_file`(git_actions.rs:158, plugin 언어 overlay 포함), `service::diff_file`(335) | 탭 UI에서 호출 | S |
| 40 | Diff 렌더: side-by-side, 줄 단위 변경 하이라이트, 공백 변경도 diff(`ignoreTrimWhitespace:false`), 읽기 전용, 자동 레이아웃, 언어별 하이라이트 | `features/git/diff-view.tsx:26-83` | missing | native에 diff 엔진(similar/imara 등) 의존성 없음, diff 렌더러 없음. 탭 본문은 라벨+상태 문자열 폴백(`application.rs:4916-4922`) | 줄 diff 엔진 선택·구현, 2열 렌더러, 스크롤 동기화, 구문 하이라이트 연동 | XL |
| 41 | Inline 모드 및 Alt+\ 토글 | `diff-pane.tsx:17,61-70`, `diff-view.tsx:75-77` | missing | 대응 없음 | inline 렌더, 토글 단축키 | M |
| 42 | `editorDiffHideUnchangedRegions` / `editorDiffShowMoves` 설정이 diff에 반영 | `shared/lib/diff-view-settings`, `diff-view.tsx:79-81` | unwired | 설정 토글은 native Settings에 있음(`settings-code-controls.rs:37-38,160-161,260-265`) 그러나 diff 뷰가 없어 소비자 없음 | 미변경 영역 접기, move 감지 | L |
| 43 | Hunk gutter stage/unstage: 수정 쪽 gutter 클릭, staged면 unstage, 변경 hunk 범위 장식 | `diff-pane.tsx:80-117`, `diff-hunk-range.ts` | unwired | `git_stage_hunk`(508), `git_unstage_hunk`(529) | hunk 범위 추출, gutter 클릭 영역, 장식 | M |
| 44 | stageable 규칙: 충돌 파일·compareWith 비교에서는 hunk stage 비활성(abs/relative 경로 둘 다 대조) | `diff-stageability.ts` | missing | 대응 없음 | 규칙 이식 | S |
| 45 | 파일 대 파일 비교(`compareWith`): 두 파일을 읽어 diff | `diff-pane.tsx:42-47,53-58` | missing | 대응 없음. 탭 모델 필드 `compare_with`만 존재(layout.rs:63-66) | 진입점(탐색기 비교), 파일 로드 | M |
| 46 | Diff 로딩 빈 화면 / 실패 문구(`editor.diffLoadFailed`) | `diff-pane.tsx:119-127` | missing | 로케일 키는 있음, UI 없음 | 상태 렌더 | S |
| 47 | 커밋 파일 diff 탭(CommitFileDiff): rev vs parentRev, NotFound는 빈 쪽 처리(추가/삭제 파일), 부모 없음(initial commit) 처리, 설정 반영 | `commit-file-diff.tsx` | unwired | `git_show_file`(185), `service::show_file`(294, 크기 제한) | 탭 UI + 40번 diff 렌더 의존 | S(40 선행) |
| 48 | 탭 우클릭 메뉴 Open Changes(현재 파일을 Diff 탭으로, 포커스 pane 대상, preview) | `pane-tab-bar.tsx:210-225` | missing | `shell.rs` 탭 컨텍스트 메뉴에 해당 항목 없음 | 메뉴 항목, 오픈 연동 | S |
| 49 | Claude diff 요청 수신: `ide:diff-requested` -> ClaudeDiff 탭 생성(`ideAutoOpenDiff` 존중) | `app/providers/ide-sync-provider.tsx:54`, `claude-diff-pane.tsx` | missing | 이벤트는 발행·릴레이됨(`ide-tools.rs:179`, `event-relay.rs:170`)이나 native 소비자가 없음. 탭을 여는 코드 없음(`TabKind::ClaudeDiff` 생성 지점 검색 0건). 설정 스위치는 있음(`settings-controls.rs:61`) | 이벤트 -> 탭 오픈, 자동 열기 설정 소비 | M |
| 50 | ClaudeDiffPane 렌더: 원본=디스크 파일, 수정=요청 내용(편집 가능), 언어 동기화 | `claude-diff-pane.tsx:55-85,141-153` | missing | `tab_content` 폴백(`application.rs:4916-4922`) | 편집 가능한 diff 에디터(40번 선행) | L |
| 51 | Accept/Reject: saved(편집된 내용 전송)/rejected, 진행 중 중복 방지, toast, 탭 닫기 | `claude-diff-pane.tsx:110-139` | unwired | `ide_actions::ide_resolve_diff`(ide_actions.rs:52), 원격 arm `remote-ide.rs:54` | 버튼 UI, 결과 반영, 탭 닫기 | S |
| 52 | 탭 닫힘/다른 창 이동 시 암묵적 reject 구분(모든 창 트리 검사) | `claude-diff-pane.tsx:35-36,101-108` | partial | 탭 닫기 경로는 연결됨: `tabs.rs:128-131`이 pending diff를 `TabClosed`로 해제, `layout_service::collect_claude_diff_tab_ids`(ide-tools.rs:371) 사용. 탭 자체가 native에서 생성되지 않아 사실상 도달 불가이며, 창 간 이동 구분 로직은 확인하지 못함 | 이동 vs 닫힘 판별(`isTabStillOpenInLayout` 동등) | S |
| 53 | 파일 히스토리 패널: 우측 도킹 다이얼로그, 항목(요약/작성자/상대시간/해시), 로딩·오류·빈 상태 | `file-history-panel.tsx:47-111` | unwired | `git_file_log`(600), `service::file_log`(1225) | 다이얼로그 UI, 목록 | M |
| 54 | 히스토리 항목 선택 -> inline CommitFileDiff, 뒤로가기 버튼 | `file-history-panel.tsx:55-84` | missing | 대응 없음 | 47번 + 41번(inline) 의존 | S |
| 55 | 파일 히스토리 진입점: 탭 메뉴, Explorer 컨텍스트 메뉴, 에디터 우클릭 액션, 명령 git.openFileHistory, 슬롯 포커스 게이트 | `pane-tab-bar.tsx:224`, `explorer-container.tsx:214`, `use-editor-git-gutter-and-conflicts.ts:303-313`, `git.commands.ts:40-45` | missing | `keybinding-commands.json:1401-1404` 항목만. 핸들러·메뉴 없음 | 4개 진입점 연결 | S |
| 56 | 커서 라인 blame footer(300ms debounce, 작성자/요약/상대시간 포맷, 본인이면 You, 경로 전환 시 즉시 비움) | `use-editor-blame.ts:40-107`, `shared/lib/blame-format.ts` | unwired | `git_blame_range`(225), `git_current_user`(336) | 에디터 하단 footer 위젯, 포맷 이식, debounce | M |
| 57 | Blame overlay 토글(전체 파일 줄 끝 after-text) + 명령 git.toggleBlame | `use-editor-blame.ts:109-145`, `git.commands.ts:34-39` | unwired | 위 + 테마 키 `editorBlame`(`theme-editor-tokens.rs:94`)만 존재. `editor_surface.rs`에 줄 끝 장식 확장점 없음 | 에디터 인라인 장식 API, 토글 상태 | L |
| 58 | Git gutter 변경 막대(added/modified/deleted) | `use-editor-git-gutter-and-conflicts.ts:56-58,204-228` | unwired | `git_gutter`(217), `service::gutter`(571, `GutterHunk`) | gutter 데코 레이어(`editor_surface.rs:669` gutter_width는 줄번호 폭만 계산) | L |
| 59 | gutter 클릭 -> hunk discard 확인 다이얼로그(`HunkDiscardDialog`) | `use-editor-git-gutter-and-conflicts.ts:150-160`, `hunk-discard-dialog.tsx` | unwired | `git_discard_hunk`(450) | gutter 히트 테스트, 다이얼로그 | M |
| 60 | 에디터 우클릭 Stage Changes(선택 있으면 선택 줄 stage_lines, 없으면 커서가 속한 hunk, 충돌 파일이면 무시) | `use-editor-git-gutter-and-conflicts.ts:255-300` | unwired | `git_stage_lines`(550), `git_stage_hunk`(508) | 컨텍스트 메뉴 항목, 선택 줄 범위 계산 | S |
| 61 | 충돌 마커 파싱 및 표시: `<<<<<<<` / `|||||||` / `=======` / `>>>>>>>` 영역, current/incoming 배경, gutter 액션, 변경 시 디바운스 재파싱 | `features/git/conflict-marker.ts`, `use-editor-git-gutter-and-conflicts.ts:162-252` | missing | Rust에 파서 없음(검색 `<<<<<<<`는 `remote-git-tests.rs` 픽스처 외 0건). 에디터 데코 확장점 없음 | 파서 이식, 줄 배경/gutter 데코 | L |
| 62 | 충돌 해결 다이얼로그: Accept Current/Incoming/Both/Compare, 마커 모두 사라지면 `git_resolve_conflict`로 저장+재stage, 파일 쿼리 무효화 | `conflict-resolution-dialog.tsx`, `use-editor-git-gutter-and-conflicts.ts:100-140`, `git.query.ts:303-324` | unwired | `git_resolve_conflict`(491), `service::resolve_conflict`(738) | accept 편집 함수 3종, 다이얼로그, 에디터 편집 적용 | M |
| 63 | 충돌 비교 다이얼로그(ours vs theirs side-by-side) | `conflict-compare-dialog.tsx` | unwired | `git_conflict_sides`(483), `service::conflict_sides`(709) | 다이얼로그 + 40번 diff 렌더 의존 | S(40 선행) |
| 64 | git 상태 갱신 파이프라인: 프로젝트 `.git` 워처 attach, fs/git 이벤트 -> 상태 캐시 무효화 -> UI 재조회 | `src/app/providers` 이벤트 구독, `git.query.ts:163-181` | partial | 워처·무효화는 실제 앱에서 가동: `projects.rs:91-160,221-262`가 `AppEvent::GitStatusChanged/GitRefsChanged` 발행, `event-relay.rs:15-37`이 `GitStore::invalidate_status` 호출, `bootstrap.rs:75,103`에서 activate. 그러나 native UI에 구독/재조회 소비자가 없음 | UI 쪽 이벤트 구독 및 상태 모델(스냅샷 갱신) | M |
| 65 | git.* 로케일 메시지(ko/ja/en 79키) | `src/shared/i18n` | partial | 키는 `crates/taide-locale/resources/locales/{ko,ja,en}.json`에 존재. Git 화면이 없어 소비처 0 | 화면 구현 시 `presentation::message`로 소비, 상대시간 토큰 이식 | S |

집계: 총 65행. done 0 / partial 3(#52, #64, #65) / unwired 33 / missing 28 / n/a 1(#15).

## 영역별 판정 요약(요청 항목)

- SCM 뷰(변경 목록, stage/unstage/discard, 메시지, 커밋, amend): UI 전부 없음(#1,#10-#13,#16-#26). 백엔드와 원격 RPC는 있음. amend는 TS UI도 없으므로 n/a(#15).
- 브랜치·원격·stash: UI 없음(#3-#9). `git_fetch`는 백엔드·원격 RPC만 있고 TS UI도 없음(#15).
- 커밋 그래프: 데이터(`git_log`)만 있고 레인 계산·렌더·pane·상세·컨텍스트 메뉴 전부 없음(#27-#36).
- 파일 히스토리: 데이터만 있고 UI·진입점 없음(#53-#55).
- blame: `git_blame_range` 백엔드만. footer/overlay UI와 에디터 데코 API 없음(#56-#57).
- diff pane(side-by-side/inline/hunk): diff 엔진과 렌더러 자체가 없음. `git_diff_file`, hunk stage/unstage 백엔드만 있음(#39-#46).
- commit-file-diff: `git_show_file` 백엔드만. 탭 렌더 없음(#47).
- claude-diff-pane: `ide_resolve_diff` 백엔드와 탭 닫힘 해제(tabs.rs)만 있고 탭 생성·렌더 없음(#49-#52).
- 파일 트리 Git 데코레이션: 없음(#38).
- 키보드 내비게이션: 없음(#26). 키맵 카탈로그에 git 명령 4개와 `git` 기본 단축키 데이터만 있고 실행 경로 없음(#1, #36, #55, #57).
- 백엔드/원격 경로만 있는 항목과 UI가 아예 없는 항목의 구분: unwired(33행)=백엔드 O·UI X, missing(28행)=백엔드 로직까지 없거나 순수 UI/알고리즘 이식 필요.

## 잘못 구현됐거나 보강이 필요한 native 코드

1. `ShellSurfaces::branch`가 항상 `None`. 타이틀바 브랜치 표시 코드(`native/taide-native-ui/src/shell.rs:177-178,228-259`)가 죽은 경로입니다. 근거: `native/taide-native-app/src/application.rs:4611-4613`.
2. 미지원 탭 종류 폴백이 사용자에게 빈 화면+상태바 문구를 냅니다. `TabKind::Diff`/`ClaudeDiff`는 `tab_content`에서 라벨 한 줄과 `"native tab surface is not connected: ..."` 상태 문자열로 떨어집니다(`application.rs:4916-4922`). 레이아웃 복원이나 원격 클라이언트가 만든 diff 탭이 native에서 죽은 탭이 됩니다.
3. `ide_open_diff`가 사용자 응답을 영영 받을 수 없는 대기 상태를 만듭니다. `ide-tools.rs:144-196`은 `IdeDiffRequested`를 발행하고 최대 600초(`IDE_DIFF_TIMEOUT`, 27행) 대기하지만, native에는 이 이벤트로 ClaudeDiff 탭을 여는 코드가 없습니다. Claude Code가 openDiff를 호출하면 10분 뒤 Rejected로만 끝납니다. native 단독 환경에서는 에이전트 diff 승인 흐름이 성립하지 않습니다.
4. 키맵 카탈로그와 실행 경로 불일치. `keybinding-commands.json:1386-1404`의 git 명령 4개와 `keymap-defaults.json:57-60`의 `ctrl+shift+g`는 키 바인딩 편집기에 노출되지만 `shell_keymap.rs`는 이를 처리하지 않아 설정해도 아무 동작이 없습니다.
5. native Settings에 diff 옵션 토글(`settings-code-controls.rs:37-38`)이 있으나 소비자가 없어 켜도 효과가 없습니다.
6. 에디터 표면에 확장점이 없습니다. `editor_surface.rs`의 `EditorAppearance`/`gutter_width`(26,669행)는 줄번호 폭만 계산하고 gutter 아이콘, 줄 배경 데코, 줄 끝 인라인 텍스트 API가 없습니다. blame·git gutter·충돌 표시(#56-#61)는 이 API 설계가 선행되어야 합니다. 잘못 구현은 아니나 Git UI 전환의 설계상 선행 과제입니다.
7. 진척 보고의 오독 위험. `docs/HANDOFF.md:386`의 "Git41명령 domain adapter 연결"은 원격 게이트웨이 한정입니다. egui 화면과 무관하며 UI 전환 완료 기준(done)으로 집계하면 안 됩니다.
8. 백엔드 `run_git`(`crates/taide-git/src/service.rs:2080-2113`)은 `GIT_TERMINAL_PROMPT=0` 등 비대화형 환경변수를 설정하지 않고 타임아웃을 300초(`GIT_COMMAND_TIMEOUT_SECS`, 58행)로 둡니다. GUI 앱에서 인증이 필요한 push/pull은 stdin null 덕에 곧바로 실패하겠지만, credential helper가 별도 프롬프트를 띄우는 환경에서는 최대 5분 대기 가능성이 있습니다. 실행 검증은 하지 않았습니다(낮음).
9. `commit`의 `stage_all`은 `git add -A`를 실행합니다(`service.rs:250-253`). TS 게이트(`confirmStageAll`)와 의미가 같지만 UI 쪽 확인 다이얼로그가 없으면(#13) 사용자 확인 없이 전체 스테이징이 일어날 수 있으므로 UI 이식 시 게이트를 반드시 같이 가져와야 합니다.
10. 프로젝트마다 `.git` 워처와 상태 무효화가 가동되지만 native UI는 상태를 읽지 않습니다(#64). 현재는 이벤트만 흐르는 비용입니다(낮음).

## 실제 앱 연결이 끊긴 지점

1. `taide_runtime::git_actions::*` -> 호출처는 `native/taide-native-app/src/remote-git.rs`(원격 JSON arm)뿐입니다. egui `NativeApplication`(application.rs)에는 `HostCommand::Git*`가 없고 `git_actions::`를 호출하는 코드도 없습니다.
2. `ShellSurfaces::explorer`(application.rs:4641)가 사이드바의 유일한 내용입니다. SCM 뷰 선택, git 상태 입력이 없습니다.
3. `ShellSurfaces::branch`(application.rs:4611)는 `None` 고정입니다.
4. `ShellSurfaces::tab_content`(application.rs:4751-5120)는 File/Untitled/Terminal/AppFile/Settings만 렌더하고 Diff/ClaudeDiff/Welcome 등은 폴백입니다.
5. `AppEvent::IdeDiffRequested`(event-relay.rs:170)는 원격 이벤트로만 전달되고 native 탭 생성 소비자가 없습니다.
6. `AppEvent::GitStatusChanged/GitRefsChanged`(projects.rs, event-relay.rs)는 내부 캐시 무효화 후 UI 구독자가 없습니다.
7. `keybinding-commands.json` git.*와 `keymap-defaults.json` git 항목은 `shell_keymap.rs` 액션 목록(9-39행)에 없습니다.
8. Settings 필드 `git_sections_collapsed`, `git_graph_panel_size_px`, `editor_diff_hide_unchanged_regions`, `editor_diff_show_moves`는 저장/검증은 되나 Git 화면이 없어 소비자가 없습니다.
9. 브라우저 Wasm 클라이언트(`native/taide-remote-web`)에도 Git 화면 코드가 없습니다(검색 `git_` 0건). 원격 RPC는 서버쪽만 준비된 상태입니다.

## 권장 구현 순서(의존 관계 포함)

1. 선행 설계(결정 필요): (a) 텍스트 diff 엔진 선택(native에 `similar`/`imara-diff` 등 의존성 없음, git2 patch 활용 여부 포함), (b) 에디터 표면의 gutter/줄 배경/줄 끝 인라인 데코 API, (c) 사이드바 뷰 전환 구조. 이 셋이 이후 단계의 공통 선행입니다.
2. 사이드바 뷰 전환(#1)과 git 상태 모델: `git_status` 조회, `GitStatusChanged` 구독, `branch()` 연결(#37, #64). 이후 모든 UI가 이 모델에 의존합니다.
3. SCM 변경 목록 최소 경로: 그룹 분류(#16), 행 표시(#19), stage/unstage/discard + 확인(#20-#22), 커밋 박스와 게이트(#10-#13), 헤더(#3-#8), stash(#9). 이 단계가 끝나면 일상 커밋 작업이 가능합니다.
4. diff 탭: diff 엔진+렌더(#40) -> Open Changes 연결(#24, #48) -> hunk stage(#43-#44) -> inline/Alt+\(#41) -> 설정 반영(#42) -> compareWith(#45) -> 로딩/오류(#46).
5. 커밋 그래프: 레인 알고리즘(#28) -> 렌더(#29) -> pane/영속(#30) -> 상세 패널(#31-#32) -> Revert/Tag(#33-#36), CommitFileDiff(#47)는 4단계 diff 렌더 이후.
6. 파일 히스토리(#53-#55): CommitFileDiff(#47) 선행.
7. Claude diff: ClaudeDiff 탭 생성(#49) -> 편집 가능 diff 렌더(#50, 4단계 선행) -> Accept/Reject(#51) -> 이동/닫힘 구분(#52). 3번 결함(10분 대기)은 최소한 탭 오픈 + Accept/Reject만이라도 우선 처리할 가치가 큽니다.
8. 에디터 Git 기능: gutter(#58) -> hunk discard(#59), Stage Changes(#60) -> blame footer(#56) -> overlay(#57) -> 충돌 파서/데코(#61) -> 해결 다이얼로그(#62) -> 비교(#63).
9. 파일 트리 데코레이션(#38), AI 커밋 메시지(#14), 초안 기억/단축키/키보드 roving(#11, #12, #25, #26)은 독립이라 3단계 이후 병렬 가능합니다.
10. 키맵 연결(#36, #55, #57 명령)은 각 기능이 생긴 뒤 `shell_keymap.rs` 액션으로 마감합니다.

## 확인하지 못한 것(불확실성)

- 빌드·테스트를 실행하지 않았습니다. `remote-git-tests.rs`의 4개 테스트 이름만 확인했고 통과 여부는 모릅니다. 백엔드 함수의 동작 정확도(hunk 경계, rename 처리 등)는 `taide-git` 테스트를 읽지 않아 판정하지 않았습니다.
- `src-tauri/src/domain/git` 쪽 구현과 `taide-git`의 동등성은 비교하지 않았습니다. TS가 쓰던 Tauri 명령과 `git_actions` 41개의 시그니처 동등성은 `command-policy.rs` 목록과 `remote-git.rs` COMMANDS가 일치하는 것만 확인했습니다.
- `application.rs`(약 5.5천 줄)는 ShellSurfaces 구현부와 호출 체인 검색만 확인했고 전체를 읽지 않았습니다. `HostCommand` 변형 전체 목록은 grep 결과로만 확인했습니다(Git 변형 0건).
- `git.*` 로케일 키가 TS의 모든 키와 1:1인지는 열거만 했고 값 대조는 하지 않았습니다.
- `isTabStillOpenInLayout`(#52)의 창 간 이동 구분 동등 로직이 native `tabs.rs`/layout 서비스에 이미 있는지는 확인하지 못했습니다.
- `LOG_PAGE_SIZE` 값과 TS의 git 쿼리 stale/무효화 세부는 확인하지 않았습니다.
- 보조 창(auxiliary) 쉘이 같은 `ShellSurfaces`를 쓰는지(`ShellSlotId::Auxiliary`)는 타입 정의만 확인했고 SCM이 슬롯별로 마운트되는 TS 동작(d-62)에 대응하는 native 구조는 조사하지 않았습니다.
- 접근성(키보드·스크린리더) 동등성은 Git 화면이 없으므로 판정 대상이 되지 못했습니다.
