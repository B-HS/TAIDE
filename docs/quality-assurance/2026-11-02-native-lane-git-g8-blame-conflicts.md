# native lane git 묶음 8/8 (g8-blame-conflicts) — blame·gutter·충돌

> 작성일: 2026-11-03 · worktree `TAIDE-lanes/git`(branch `lane/git`) · cargo target `TAIDE-lanes/targets/git`
> 사양 `docs/research/2026-10-28-lane-git-spec.md` 2.0.2~2.0.6·git-55~63, 3.1 G8. 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안), `docs/acknowledge/2026-10-26-native-autonomous-continuation.md`

## 1. 대상 ID와 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| git-56 | complete | 파일 편집기 아래 24px footer(배경 `editorBlame.background`, 글자 `editorBlame.foreground` 11px, 위 경계선 `app.border`, 좌우 8px, 말줄임). 커서 줄 300ms 안정 뒤 `git_blame_range(line, line)` 첫 항목, 원본 문구 규칙, 새 줄 응답 전 이전 문구 유지, 오류·문서 밖 줄은 빈 문자열, 경로가 바뀌면 즉시 비움 |
| git-57 | complete | `git.toggleBlame`(명령)·`monaco.taide.toggleBlame`(keybinding id)·편집기 메뉴 navigation 맨 앞 → 켤 때 줄 수로 `git_blame_range(1, n)`(켤 때마다 다시 읽음), 줄 끝 `"    " + 문구` 기울임·80% 불투명, 다시 실행하면 꺼짐, 경로가 바뀌면 꺼짐 |
| git-58 | complete | `git_gutter(path)` hunk → added·modified 3px 막대, deleted 삼각형(`editorGutter.*Background`), 그 경로 fs 변경·저장소 변경으로 다시 읽음 |
| git-59 | complete | gutter lane 누름 → hunk discard 대화상자 → `git_discard_hunk`, 실패 오류 toast, hunk 밖 줄 무시 |
| git-60 | complete | 편집기 메뉴 `9_git` 첫 항목 `git.stageChanges`·keybinding id `monaco.taide.gitStageSelection`: 선택이면 `git_stage_lines`(다음 줄 첫 칸 끝 제외), 커서만이면 그 줄 hunk `git_stage_hunk`, 충돌 파일·hunk 밖 무시 |
| git-61 | complete | status 충돌 행이면 마커 파서 영역 → current/incoming 줄 배경, 시작 줄 `git.conflicted` 3px 막대(pointer), hunk 막대 대신. 첫 파싱 즉시, 편집 뒤 마지막 변경 150ms 뒤 |
| git-62 | complete | 시작 줄 lane 누름 → 해결 대화상자(취소·비교·둘 다·수신·현재) → 문서 전체 한 번의 undo 단위 편집 → 마커가 없으면 `git_resolve_conflict` → 저장 성공 처리 재사용 정착 + `git.conflictResolved`, 실패 오류 toast, 마커가 남으면 편집만 |
| git-63 | complete | 비교 → `git_conflict_sides`(누를 때마다 다시 읽음) → 70% 높이·최대 폭 896 대화상자 안 ours↔theirs side-by-side, 누른 뒤 실패 응답만 오류 toast, 닫으면 가상 문서 해제 |
| editor-73 | complete | git-58~60 근거 |
| editor-74 | complete | git-61~63 근거 |
| editor-75 | complete | git-56·57 + git-55 남은 범위(편집기 메뉴 File History, `monaco.taide.openFileHistory`) 이번 묶음, 파일 이력 패널은 G7 |
| editor-76 | partial | diff 표면 중 git-63(충돌 비교) 이번 묶음. 남은 것: git-40 문자 단위 강조·±·overview ruler, git-50 ClaudeDiff revert 화살표(결정 4 A) |
| (함께) git-55 | complete | G7 남은 범위였던 편집기 우클릭 `9_git` File History(Stage Changes 다음, peek 제외)와 keybinding id 실행 연결 |

## 2. 원본 근거

사양 각 절을 기준으로 아래 구간만 다시 확인했습니다.

- `src/widgets/editor-pane/use-editor-blame.ts` 전체(300ms debounce, `keepPreviousData` 채택 규칙, overlay 줄 수 고정·토글마다 재조회, navigation 그룹 action), `src/shared/lib/blame-format.ts`·`blame-format.test.ts`, `src/features/editor/blame-footer-bar.tsx`, `src/entities/git/git.query.ts:52-103,146-153,171-188,303-330`, `src/shared/constants/query-key.ts`(blame·conflict-sides·current-user 모두 `GIT.PROJECT` 접두사, 불변 범위 아님).
- `src/widgets/editor-pane/use-editor-git-gutter-and-conflicts.ts` 전체, `src/features/git/conflict-marker.ts`·`conflict-marker.test.ts`, `conflict-resolution-dialog.tsx`, `conflict-compare-dialog.tsx`, `hunk-discard-dialog.tsx`, `src/shared/lib/selection-line-range.ts`, `src/widgets/editor-pane/conflict-status.ts`, `src/widgets/editor-pane/editor-pane.tsx:156-186,439-533`(경로 전환 초기화, footer 배치, 대화상자 배치), `src/widgets/editor-area/pane-node-view.tsx:178-179`(EditorPane은 파일 탭만), `src/shared/styles/global.css:371-440`, `src/entities/git/git.commands.ts`.
- native: `crates/taide-runtime/src/git_actions.rs`(`git_gutter`·`git_blame_range`·`git_current_user`·`git_discard_hunk`·`git_conflict_sides`·`git_resolve_conflict`·`git_stage_hunk`·`git_stage_lines`), `crates/taide-git/src/service.rs`(`gutter` deleted 표식 = `new_start.max(1)`, `blame_range`는 `blame_file`로 HEAD 기준, `conflict_sides` 비충돌 경로는 `error.git.conflictInfoNotFound`), libgit2 `blame.c:449-453`(max_line을 검증 없이 쓰므로 원본과 같이 줄 수 그대로 요청).

## 3. 구현

### 3.1 공통

- 캐시(`git-cache.rs`): `Key::Gutter { path }`(가변, 그 경로 fs 변경), `Key::CurrentUser`(가변, 시간 stale 없음), `Key::BlameLine { path, line }`(가변), `Key::BlameOverlay { path, lines }`(가변, stale 0 = 다시 붙을 때마다 다시 읽음; `lines`는 켤 때 줄 수라 원본의 줄 수 고정과 같음), `Key::ConflictSides { path }`(가변, 응답은 `Value::Diff`로 ours/theirs + hunk). `Value::Gutter`·`CurrentUser`·`Blame`, 읽기 `gutter()`·`current_user()`·`blame()`, `GitCache::refetch(project, key)`(TanStack `refetch()`: generation을 올려 진행 중 응답도 한 번 더 읽음).
- 명령(`domains/git.rs`, 모두 `spawn_transient`): `Command::DiscardHunk`·`StageLines`(`MutationOp::DiscardHunk`·`StageLines`, 성공이면 저장소 무효화, 실패 오류 toast), `Command::ResolveConflict { project, path, content, snapshot }` → `git_resolve_conflict` 뒤 `file_open`으로 디스크 내용 → `Reply::ConflictResolved`.
- 상태: `State.editors: git_gutter::Editors`(pane별 `Editor { project, tab, path, blame, pending_hunk, conflicts }`, 다른 파일이면 새로 시작, 그리지 않은 pane은 다음 tick에 버림 = EditorPane 해제), 외형 `git_gutter::Appearance`(theme 토큰, 새 토큰 없음).
- 실제 앱 경로: `main.rs` → `NativeApplication` → 탭 내용 `domains/editor.rs` `show_document`(파일 탭) → (1) `git_blame::split_footer`로 편집기 아래 24px 확보, (2) `State::editor_layers`가 decoration layer를 기존 `find_layers` 체인에 추가, (3) 편집기 그린 뒤 `AppSurfaces::show_git_editor`(footer·overlay·lane 누름·대화상자, `Intent::EditorShown`) → `apply_git_intent` → `apply_git_editor_intent`(관찰·debounce·파싱·대화상자 결정). 편집기 메뉴는 `run_editor_context_menu`의 `Run::Git` 분기 → `run_git_editor_command`. 명령·keybinding은 `shell_keymap::intent`의 `Run::Git(ToggleBlame | StageSelection)`(focus pane 활성 파일 탭).
- 모듈: `git-blame.rs`(문구·debounce·footer·overlay), `git-conflict.rs`(파서·변환·재파싱 시점·비교 요청), `git-gutter.rs`(외형·layer·hunk 찾기·선택 줄 범위·`alert` 대화상자·pane 상태), `git-editor.rs`(`domains::git`의 하위 모듈: 앱 상태를 쓰는 intent 처리와 그리기).

### 3.2 git-56·57

- `Blame::cursor`가 줄이 바뀌면 시각을 다시 재고 300ms 뒤 `settled`를 정하며 남은 시간만큼 `request_repaint_after`. `adopt`는 정해진 줄 키의 새 응답(`fetched_at` 변경)만 받아 첫 항목 또는 오류면 없음으로 바꾸고, 응답 전에는 이전 값을 둡니다(원본 `isPlaceholderData`·오류 분기).
- footer 문구는 `format_line`(원본 `formatBlameLine`)으로 그리고, 줄이 문서 줄 수를 넘으면 빈 문자열입니다. 경로가 바뀐 pane은 상태 필터(`path`·`tab` 일치)에서 빠져 첫 그림부터 빈 footer입니다.
- overlay는 `paint_overlay`가 편집기 그린 뒤 보이는 줄의 줄 끝 caret 위치(`EditorGeometry::caret_rect`)에 텍스트 영역으로 잘라 그립니다. 레이아웃·커서에 관여하지 않습니다.

### 3.3 git-58~60

- `hunk_layer`: added/modified `LaneMark::Bar`, deleted `LaneMark::DeletedTriangle`(기존 gutter 그리기). 충돌 파일은 `conflict_layer`로 대체.
- lane 누름: UI `diff_view::lane_line`(G5 `lane_press`를 일반화한 공개 함수)이 줄 장식 칸 누름 줄을 돌려주고, 충돌이면 시작 줄만·pointer 커서, 아니면 hunk 줄만(원본 CSS처럼 pointer 없음). `Intent::EditorLanePress` → `pending_hunk` 또는 `conflicts.pending`.
- 대화상자: `git_gutter::alert`(egui Modal, 첫 단추 취소 focus, Escape = 취소, 바깥 클릭 무시 — AlertDialog). 설명은 `{start} {end}` 인자.
- Stage Changes: 실행 시점 주 선택으로 `selected_lines`(원본 `resolveSelectedLineRange`) 또는 커서 줄 hunk.

### 3.4 git-61~63

- 재파싱: `Conflicts::observe(is_conflicted, revision, now)`가 첫 관찰은 즉시, 이후 revision이 바뀌면 마지막 변경에서 150ms 뒤 파싱을 알리고, 충돌이 풀리면 상태를 잊어 다시 충돌이면 즉시 파싱합니다.
- 해결: `git_conflict::resolve` 결과를 `EditorStore::apply_separate`(전체 범위 1 편집, `UndoGroup(revision)`, 병합 없음)로 넣고 `editor_menu_changes`에 기록해 다음 프레임에 dirty·mirror 경로를 탑니다. 마커가 없으면 `save_snapshot` → `Command::ResolveConflict` → 응답 성공 시 저장소 무효화 + `domains::editor::Reply::Saved`를 `apply_editor_reply`로 넘겨 저장 성공과 같은 정착(`mark_saved`·`observe_file`·`SetDirty`·persistence 정리) + success toast.
- 비교: `ConflictAction::Compare` → 요청 시점 응답 시각을 `seen`으로 두고 `refetch`. 응답이 Ok이면 `diffs.shown(owner)`로 가상 문서를 유지하고 대화상자에서 공용 `show_surface`(side-by-side, stage lane 없음, 파일 언어, `editor_diff_hide_unchanged_regions`)를 그립니다. 새 오류 응답이면 오류 toast. X 단추(`common.close` 이름)·Escape·바깥 클릭으로 닫으면 `CloseConflictCompare` → 다음 tick `release_hidden`이 문서를 해제합니다.

### 3.5 범위 밖 최소 수정과 사양 차이

- `domains/editor.rs`(editor 영역): `show_document`에 footer 칸 분리 1줄, git layer 계산과 체인 추가, 편집기 뒤 `show_git_editor` 호출, `run_editor_context_menu`에 `Run::Git` 분기. 사양 2.0.1의 "편집기 그리기 지점 호출" 자리입니다.
- `editor-context-menu.rs`(APP·UI, editor 영역): `Context.git`(파일 문서이고 peek 아님), navigation 맨 앞 Toggle Blame, `9_cutcopypaste`와 `9_terminal` 사이 `9_git`(Stage Changes, File History), locale 라벨 `git.toggleBlame`·`git.stageChanges`·`git.fileHistory`.
- `application.rs`: git_state 리터럴 끝 `editors` 한 줄.
- `command-registry.rs`: `GitRun::ToggleBlame`·`StageSelection`, `git.toggleBlame`·`monaco.taide.*` 세 id 실행 연결. `monaco.taide.*`는 기존 규칙대로 `SupportedEditorAction`이라 편집기 활성일 때만 실행되며, 활성 편집기 동작 목록 기대값에 세 id를 더했습니다.
- 사양 2.0.1의 `SurfaceExtras.line_end_texts`·`EditorOutput.lane_press` 대신: overlay는 편집기 뒤 공개 `EditorGeometry::caret_rect`로 그리고(그리기 전용이라 같은 결과), lane 누름은 G5가 diff 위젯에 둔 판정을 `diff_view::lane_line`으로 공개해 재사용했습니다. 새 `NativeEditor` 진입점이 필요 없고 기존 `show_request_with_code_lenses` 호출을 바꾸지 않습니다.
- 사양 2.0.2의 `Gutter`·`Blame`·`ConflictSides` 개별 명령 대신 공용 캐시 `Command::Read`의 키를 씁니다(G5~G7과 같은 방식).
- 원본 결함 미재현: 원본은 경로가 바뀌어도 `pendingHunk`를 지우지 않지만 native는 pane 상태를 새로 시작합니다(모달이라 실제로는 같은 결과).
- footer 상대 시간은 채택 시각 기준으로 만듭니다(원본 effect가 채택·사용자 변경 때 `Date.now()`로 한 번 만드는 것과 같음).
- 원본 `blame_range`는 HEAD 기준이라 미커밋 줄도 마지막 커밋으로 표시됩니다(`isUncommitted`는 사실상 나오지 않음). native도 같은 백엔드라 그대로이며 미커밋 문구는 단위 검사로만 확인했습니다.

## 4. 검증

모든 cargo 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/git`.

| 명령 | 결과 |
| --- | --- |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- git_blame git_conflict git_gutter git_cache --test-threads=2` | 21 passed(`[C]` blame 7, conflict 3, gutter 3, 캐시 신규 1 포함) |
| 같은 crate `--lib -- editor_tests --test-threads=2` | 3 passed(아래 App 검사). 첫 두 실행은 검사 쪽 결함으로 실패: 같은 색의 다른 사각형을 세던 것(3px 폭 조건 추가), 미커밋 문구 기대(원본 백엔드가 HEAD blame이라 두 번째 커밋 줄로 바꿈), 비교 실패 뒤 문서 해제를 한 프레임 일찍 단언(`until`로 변경) |
| 같은 crate `--lib -- git shell_keymap command_dispatch title_branch context_menu_tests --test-threads=2` | 137 passed(G1~G7 git App 검사, 편집기 메뉴 App 검사 회귀 포함). fmt 뒤 `git shell_keymap command_dispatch context_menu_tests` 135 passed |
| 같은 crate `--lib -- editor --test-threads=2` | 351 passed, 1 failed: `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`(단독 재실행도 같은 실패, 글꼴 face 목록 차이). 이 묶음은 `editor-fonts.rs`·`terminal_fonts`를 바꾸지 않았습니다 |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --test editor-context-menu --test diff-view -- --test-threads=2` | 8 + 9 passed(신규 `9_git`·Toggle Blame 순서 검사 포함, diff 위젯 lane 회귀) |
| UI `--lib -- command_registry command_palette --test-threads=2` | 54 passed(첫 실행 2 실패: `monaco.taide.*` enablement 기대를 `SupportedEditorAction`으로, 활성 편집기 동작 기대값에 세 id 추가) |
| UI `--test workbench -- --test-threads=2` | 45 passed |
| `cargo check --quiet --all-targets --manifest-path native/taide-native-app/Cargo.toml` | 성공(경고는 기존 vendor·`editor-code-actions.rs`·`zen.rs`·`application-inlay-hints-tests.rs`) |
| `cargo check --quiet --all-targets --features native-host,inspection --manifest-path native/taide-native-ui/Cargo.toml` | 성공(경고는 기존 `tests/workbench.rs:1816`) |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 성공(동결 crate 기존 경고만) |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml`, UI crate 같은 명령 | 이 묶음 파일과 `domains/editor.rs`의 이 묶음 분기만 정리(`git diff`로 확인). 정리 뒤 위 check·테스트 재실행 |

App 검사(`application-git-editor-tests.rs`, `domains::git::editor_tests`):

- `실제_app_파일_편집기는_gutter_막대와_blame_footer_overlay를_원본대로_그린다`: 수정·추가·삭제 3 hunk → modified·added 3px 막대 각 1, deleted 삼각형 1, 수정 막대가 2행 위치. 파일 다시 쓰기 + `FsChanged` → hunk 1개로 갱신. footer `You, … • test(fixture): initial`, 커서를 3행으로 → 응답 전 이전 문구 유지 → `… • test(fixture): second`. `git.toggleBlame` → 두 커밋 요약 overlay(줄 끝 뒤, 기울임, `a1` 글자 위치·커서 불변). 편집기 메뉴 `git.toggleBlame` → 꺼짐. 다시 켠 뒤 다른 파일 열기 → 첫 그림 footer·overlay 없음 → 새 파일 footer, overlay 꺼짐.
- `실제_app_gutter_누름은_hunk_discard_대화상자를_열고_편집기_메뉴는_선택_줄과_커서_hunk를_stage한다`: hunk 밖 1행 lane 무시, 7행 lane → 대화상자 → 취소 → 디스크 불변, 2행 lane → 설명 `2`~`2` → 확인 → 2행만 HEAD로 복원. `c.txt` 2행 시작~3행 시작 선택 + 메뉴 Stage Changes → staged diff에 `+C2`만, 6행 커서 + `monaco.taide.gitStageSelection` → `+C6` 추가.
- `실제_app_충돌_파일은_영역_배경과_lane을_그리고_해결_비교_대화상자로_정리한다`: 테스트 쪽 `git merge`로 2영역 충돌 → 영역 시작 2·12, 시작 줄 막대 2, current·incoming 줄 배경, hunk 막대 없음. 2행 lane → 설명 `2`~`6` → 현재 수락 → 문서 = 첫 영역 해결, dirty, 디스크 불변, `undo` 1회로 병합 원문, `redo`. 150ms 뒤 영역 8 하나. 8행 → 비교 → 가상 문서 2개 = HEAD ours·feature theirs, Escape → 해제. 8행 → 수신 수락 → success toast, 디스크 = 결과, dirty 해제, status 충돌 없음. 충돌이 풀린 파일에 비교 요청(실패 주입) → 오류 toast 1개, 비교 대화상자 문서 해제, 캐시 오류 보관.

금지 확인: 실제 원격·AI·Keychain·클립보드·휴지통·OS 알림·GUI 실행 없음. 저장소 조작은 임시 디렉터리 fixture와 테스트 쪽 git CLI(`GIT_CONFIG_GLOBAL=/dev/null` 등)만 썼습니다.

## 5. 남은 범위

- editor-76: git-40 문자 단위 강조·±·overview ruler, git-50 ClaudeDiff revert 화살표(결정 4 A). 이 묶음 대상 ID 밖입니다.
- commands-ui-36~39·87(gutter/blame/파일 이력 명령 행)은 사양 1.3대로 wave 3 commands-ui lane이 이 근거로 닫습니다(행 미변경).
- 앞 묶음에서 이어진 병합 뒤 연결: git-7 pull/push OS 알림(이 worktree에 settings·shell 알림 진입점이 아직 없음, `domains`에 `fn notify` 없음).
- 비고: `editor_fonts` 실패 1건은 이 묶음 밖(글꼴 영역) 기존 상태로 보이며, 병합 게이트 전체 실행 때 확인이 필요합니다.

## 6. 리뷰 수정(2026-11-03)

### 6.1 차단 2: 해결 대화상자의 오래된 영역으로 슬라이스 범위 초과 패닉(git-62, editor-74)

- 원인(실제 코드 확인): lane 누름 때 저장한 `conflicts.pending`(`git-editor.rs` `Intent::EditorLanePress`)은 이후 재파싱에서 갱신·검증되지 않고, `apply_conflict_resolution`이 그 영역을 최신 rope 문자열에 그대로 적용했습니다. `git_conflict::resolve`는 `lines[from-1..to]`·`lines[..start-1]`·`lines[end..]`를 검사 없이 잘라, 문서가 영역 끝보다 짧아지면 패닉했습니다(원본 TS의 `slice`는 패닉하지 않고 잘못된 결과만 냄).
- 재현: 수정 전 실행에서 App 검사가 `range start index 4 out of range for slice of length 3`(`git-conflict.rs:95`), 단위 검사가 `range end index 4 out of range for slice of length 2`로 실패했습니다.
- 수정:
  - `apply_conflict_resolution`: 현재 문서를 `git_conflict::parse`로 다시 읽어 pending 영역과 같은 항목이 있을 때만 적용합니다. 없으면 편집 없이 끝나며, pending은 `resolve_git_conflict`가 이미 꺼냈으므로 대화상자가 닫힙니다.
  - `git_conflict::resolve`: 네 구간을 `lines.get(..)`으로 잘라 하나라도 범위를 벗어나면 원문을 그대로 돌려줍니다(방어선).
  - 원본과의 차이: 원본은 영역이 밀린 문서에 오래된 줄 번호로 잘못된 편집을 했습니다. 원본 결함이라 재현하지 않고 무동작으로 둡니다.
- 검사:
  - `[C]` `해결_중_문서가_짧아져_영역이_범위를_벗어나면_원문을_그대로_돌려준다`: 두 줄로 줄어든 문서에 세 해결 모두 원문, 끝 마커가 잘린 문서에 원문, 빈 문서에 빈 문자열.
  - `[A]` `실제_app_해결_대화상자가_열린_동안_문서가_짧아지면_수락은_패닉_없이_무동작으로_닫힌다`: merge 충돌 fixture → 2행 lane → 해결 대화상자(설명 `2`~`6`) → 대화상자가 열린 동안 같은 문서를 다른 출처(`origin: None`, 보조 창 편집과 같은 경로)에서 `head\ntail\n`으로 줄임 → "현재 변경 수락" → 대화상자 닫힘, pending 없음, 문서 = 줄인 내용, 디스크 = 병합 원문, `tail` 표시, 오류 toast 없음.
- 대화상자가 떠 있는 동안 충돌이 풀려도 pending만 보고 그리는 동작(`git-editor.rs` 그리기)은 원본과 같아 바꾸지 않았습니다. 수락은 위 재파싱 검사로 무동작이 됩니다.

### 6.2 비차단: blame 요약 길이를 UTF-16 단위로

- 원본 `truncateBlameMessage`는 JavaScript `length`(UTF-16 단위)로 50을 셉니다. native `truncate_message`는 char 수로 세어 BMP 밖 글자에서 결과가 달랐습니다.
- 수정: 길이 판정을 `encode_utf16().count()`로, 자르기를 누적 UTF-16 단위가 50 이하인 글자까지로 바꿨습니다(상수 `MESSAGE_MAX_UTF16_UNITS`). 50번째 단위에 걸친 BMP 밖 글자는 버립니다. 원본은 그 자리에 짝 없는 surrogate를 남기는데, 이는 원본 결함이라 재현하지 않습니다.
- 검사 `[C]` `요약_길이는_원본_string_length처럼_utf16_단위로_센다`(수정 전 1 failed, 수정 뒤 통과): BMP 밖 글자 30개 → 25개 + `...`, `a` + 25개(51단위) → `a` + 24개 + `...`.

### 6.3 검증

모든 cargo 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/git`, 앱 lib 검사는 `--test-threads=2`.

| 명령 | 결과 |
| --- | --- |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib -- 해결_중_문서가_짧아져 해결_대화상자가_열린_동안 json_파일_diff_탭은`(수정 전 → 뒤) | 3 failed → 3 passed(JSON 항목은 g5 문서 6절) |
| 같은 crate `--lib -- utf16_단위로_센다`(수정 전) | 1 failed |
| 같은 crate `--lib -- git_blame:: domains::git::editor_tests`(수정 뒤) | 12 passed(blame 단위 8, 편집기 App 4: 기존 3 + 신규 1) |
| 같은 crate `--lib -- domains::git:: git_conflict:: editor_json application::json_tests`(blame 수정 전) | 53 passed |
| `cargo check --quiet --all-targets --manifest-path native/taide-native-app/Cargo.toml`(blame 수정 뒤) | 성공(경고는 기존 vendor·`editor-code-actions.rs`·`zen.rs`·`application-inlay-hints-tests.rs`) |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | 차이 없음 |

- 이 lane에서 확인하지 못한 것: `editor_fonts` 실패가 기준 커밋에서도 나는지(기준 커밋 checkout은 이 작업에서 금지된 git 상태 변경이라 병합 게이트에서 비교).
