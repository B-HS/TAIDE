# native lane git G6 — 로그·그래프·상세·태그 (git-27 ~ git-36)

> 기준: `docs/research/2026-10-28-lane-git-spec.md` 2절 git-27~36, 3.1 G6, 4.1 결정 3(추천안 A). 작업 위치 worktree `TAIDE-lanes/git`(브랜치 `lane/git`), cargo target `TAIDE-lanes/targets/git`.

## 1. 대상 ID와 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| git-27 | complete | `Key::Log`(skip 0, take 100) 공급을 그래프 섹션 건수·행·refs 배지에 연결(AI 요약 공급은 G4에서 연결됨) |
| git-28 | complete | `git-graph.rs::compute_lanes` 순수 이식, 원본 5사례 |
| git-29 | complete | 그래프 행 그림·refs 배지·요약/작성자/상대 시간/7자 해시·hover/선택 배경·선택 토글 |
| git-30 | complete | 변경/그래프 세로 분할, 머리말 접힘, 끌기(최소 미만 접힘 기록), slot별 120ms 뒤 높이 저장, 펼치면 저장 높이 |
| git-31 | complete | 커밋 상세 패널(요약·해시·닫기, CHANGED FILES + 건수, 로딩·실패·파일 행) |
| git-32 | complete | 상세 파일 → commit diff preview 탭(`이름 @ 7자`, rename before_path) |
| git-33 | complete | 그래프 메뉴 Revert(성공 toast, 충돌 warning + 첫 충돌 파일, 실패 오류 toast) |
| git-34 | complete | 태그 생성 대화상자(열 때마다 초기화, 빈 이름·진행 중 비활성, annotated/경량, 중복 오류 시 열린 채) |
| git-35 | complete | 태그 삭제 서브메뉴(그 커밋을 가리키는 태그만, destructive) |
| git-36 | complete | `git.revertHead`·`git.createTagOnHead` 팔레트 실행(결정 3 A: focus slot 하나, 그래프 접힘과 무관) |

함께 연결된 다른 행: git-26의 마지막 남은 범위였던 "그래프 머리말 roving 정거장"을 이번에 연결했습니다(↓ 마지막 항목 → 그래프 머리말, 그래프 머리말 ↓ 무시·↑ 마지막 항목, ←/→ 접기·펼치기). 대응표 갱신 범위 규칙상 git-26 행은 고치지 않았고, 근거(아래 4절 `changes_tests::실제_app_화살표는_양끝에서_멈춘다` 갱신)를 병합 때 반영하면 됩니다.

## 2. 원본 근거

사양 2절 각 ID의 원본 인용을 그대로 따릅니다. 이번에 추가로 확인한 원본 구간:

- `src/shared/lib/graph-lanes.ts`·`graph-lanes.test.ts`(5개 사례), `src/shared/lib/relative-time.ts`, `src/shared/lib/git-tags.ts`.
- `src/widgets/git-panel/commit-graph.tsx`: 행 마크업(flex gap-2 px-2 text-xs, 요약 truncate·나머지 shrink-0), 곡선 `M x(l) c C x(l) 24, x(to) c, x(to) 24`, 간선 색은 노드 레인 색, 메뉴 3항목(RotateCcw·Tag·Tags 서브메뉴), 대화상자 구독.
- `src/widgets/git-panel/git-panel.tsx:314-333`(`handleGraphLayoutChanged`), `:550-693`(Group·Panel·PaneSeparator), `:487-492`(접힘 동기화 effect는 펼칠 때만 저장 높이로 resize).
- `node_modules/react-resizable-panels`: 최소 미만 크기는 `(collapsedSize + minSize) / 2`보다 작으면 접힘, 아니면 최소. `onLayoutChanged`는 포인터를 놓은 뒤 1회.
- `src/features/split/pane-separator.tsx`: `bg-app-border hover:bg-ring`(ring = `app.focusBorder`, `global.css`).
- `src/widgets/git-panel/commit-detail-panel.tsx`, `src/features/git/create-tag-dialog.tsx`, `src/entities/git/git.commands.ts`, `src/entities/git/git.query.ts`(tags 가변, commit files `staleTime: Infinity`·불변 범위, mutation 성공 시 가변 범위 무효화), `git-panel-container.tsx:276`(충돌 파일은 비preview 파일 탭).
- 백엔드 `crates/taide-git/src/service.rs`: `log`의 refs는 `refs/tags/*` 참조의 직접 target(annotated는 태그 객체 id라 커밋과 안 맞음), `tags`는 annotated면 peel된 커밋 id, `revert_commit`은 `ensure_clean_index`로 staged 변경이 있으면 실패.

## 3. 구현

- 캐시(`APP/git-cache.rs`): `Key::Tags`(가변, 60초), `Key::CommitFiles { rev }`(불변, 시간 stale 없음), `Value::Tags`·`Value::CommitFiles`, `tags()`·`commit_files()`.
- 영역(`APP/domains/git.rs`): `Command::Revert { opens_conflict }`·`TagCreate`·`TagDelete`(모두 `spawn_transient`, host worker 비차단), `Reply::Reverted`, `MutationOp::TagCreate { slot }`·`TagDelete`, `finish_revert`. `Intent::Shown`은 그래프가 보이고 펼쳐져 있을 때만 Tags를, 선택 커밋이 로그에 있을 때만 그 CommitFiles를 관찰합니다(원본 `CommitGraph`·`CommitDetailPanel` 마운트 조건). 높이 저장은 `tick`에서 `Panels::take_due_resizes`로 `PatchSettings { git_graph_panel_size_px }`, 남은 대기는 `request_repaint_after`. host 재연결 시 진행 중 태그 생성 표시를 풉니다.
- 그래프(`APP/git-graph.rs`, 신규): `compute_lanes`, `max_lane`, `marks`(그림 순서·좌표), `relative_time`, `tags_targeting`, `short_hash`, 가상 목록(행 24px, overscan 12), 행 메뉴(egui `Atom::custom` 자리에 Lucide 아이콘, `SubMenuButton`), 태그 대화상자(egui `Modal`, 초안은 열기 세대별 temp 메모리).
- 상세(`APP/git-commit-detail.rs`, 신규): `height`, `file_tab`, 패널 그리기(pane 높이 절반까지, 넘으면 자체 스크롤).
- SCM 패널(`APP/git-scm.rs`): 그래프 섹션 건수 = 로그 수, 변경 pane/분리선/그래프 pane 배치(`graph_layout`·`snap_graph_height`·`graph_separator`), 패널별 선택 커밋·메뉴 대상·태그 대화상자·끌린 높이, slot별 120ms 저장 대기(`Panels.resizes`, 패널이 사라져도 유지), chevron 토글은 끌린 높이를 버리고 저장 높이로, roving에 그래프 머리말 정거장.
- 명령: UI `command-registry.rs`에 `GitRun { RevertHead, CreateTagOnHead }`·`Run::Git`, `execution`에 두 id. 앱 `shell_keymap.rs`가 RevertHead → `Intent::RevertHead`, CreateTagOnHead → 주 창 `Intent::CreateTagOnHead { focused slot }`/보조 창 `CreateAuxiliaryTagOnHead`. `apply_git_intent`가 `ShowView`/`ShowAuxiliaryView`를 적용한 뒤 그 slot 패널에 `HEAD` 대화상자를 엽니다(패널이 아직 없으면 그 자리에서 마운트해 다음 프레임에 보임).
- UI intent(`UI/intents/git.rs`): `SelectCommit`·`GraphMenu`·`Revert`·`RevertHead`·`RequestCreateTag`·`CreateTagOnHead`·`CreateAuxiliaryTagOnHead`·`CloseTagDialog`·`CreateTag`·`DeleteTag`·`ResizeGraph`·`OpenCommitFile`.
- 아이콘: `resources/icons/rotate-ccw.svg`·`tag.svg`·`tags.svg`(Lucide iconNode 그대로, `currentColor` 채움은 기존 파일처럼 흰색), `Icon::X`는 기존 `Glyph::X`.
- 범위 밖 최소 수정: `application-git-changes-tests.rs`(G2 검사) — `View`에 `graph: Default::default()` 한 줄, "화살표는 양끝에서 멈춘다"는 이제 원본대로 ↓가 그래프 머리말에서 멈추므로 그 단언을 그래프 머리말 정거장·←/→ 접기로 갱신. `command-dispatch.rs` 검사 — `git.revertHead`를 "실행 불가" 목록에서 빼고 활성 프로젝트에서만 실행되는 단언으로 바꿈. `lib.rs`에 모듈 2줄(알파벳 위치).

### 결정·차이

- 결정 3 A: 원본은 그래프 컴포넌트가 대화상자를 구독해 그래프가 안 보이거나 접혀 있으면 요청이 사라지고 slot마다 대화상자가 떴습니다. native는 focus slot 패널 하나에만, 접힘과 무관하게 띄웁니다(실제 App 검사로 확인).
- 열린 대화상자에 다른 대상 요청이 오면 입력을 유지하고 대상만 바꿉니다(원본: `open`이 그대로라 초기화 안 됨).
- 분리선 키보드 크기 조절(react-resizable-panels Separator의 화살표 키)은 구현하지 않았습니다(사양 git-30 범위 밖, 5절).
- 메뉴 아이콘 색은 egui 메뉴 글자색(`visuals.text_color()`)을 씁니다(원본 `size-4` 아이콘은 currentColor).
- annotated 태그는 refs 배지에 나오지 않습니다. 공용 백엔드 `collect_refs_by_oid`가 참조 target(태그 객체 id)으로 묶기 때문이며 원본 화면도 같습니다. 백엔드 수정은 이 묶음 범위가 아닙니다.

## 4. 검증

| 명령 | 결과 |
| --- | --- |
| `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir ../targets/git -- git_graph --test-threads=2` | 9 통과(`[C]` 레인 5사례·그림 좌표·상대 시간·태그/해시·`file_tab`) |
| 같은 crate `-- domains::git::graph_tests --test-threads=2` | 6 통과(`[A]` 로그·행·선택·상세·commit diff 탭 / 분리선 끌기·120ms·접힘·펼침 / revert 성공·staged 실패·충돌 / 태그 생성·초기화·중복·Escape·삭제 서브메뉴 / 팔레트 revertHead·createTagOnHead / 상세 로딩·실패 색) |
| 같은 crate `-- git shell_keymap command_dispatch title_branch --test-threads=2` | 95 통과(이전 묶음 git 검사 포함). 첫 실행에서 갱신 전 두 검사(`changes_tests::실제_app_화살표는_양끝에서_멈춘다`, `command_dispatch::tests::명령_gate…`)가 이번 동작 변화로 실패 → 원본 기준으로 단언 갱신 후 통과 |
| 같은 crate `-- application::explorer_views_tests application::auxiliary_window_tests symbol_sidebar navigation_icons --test-threads=2` | 16 통과 |
| `cargo test --quiet --manifest-path native/taide-native-ui/Cargo.toml --features native-host,inspection --lib ... -- command_registry` | 15 통과(`[C]` `git.revertHead`·`git.createTagOnHead` 실행값·활성) |
| `cargo check --quiet ... taide-native-app/Cargo.toml --all-targets` | 통과(남은 경고 3건은 이 묶음이 고치지 않은 `editor-code-actions.rs`·`zen.rs`·`application-inlay-hints-tests.rs`) |
| `cargo check --quiet ... taide-native-ui/Cargo.toml --all-targets --features native-host,inspection` | 통과(경고 1건은 HEAD부터 있는 `tests/workbench.rs:1816`) |
| `cargo check --quiet ... taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 통과(경고는 동결 crate 기존분) |
| `cargo fmt --manifest-path ...taide-native-app/Cargo.toml -- --check`, 같은 UI crate | 둘 다 차이 없음 |

실제 앱 경로: 모든 `[A]` 검사는 `NativeApplication` 프레임(`logic` → `ui`)으로 사이드바 Git 보기(`domains::explorer_search` → `show_git_view` → `git_scm::show` → `change_list` → `git_graph::show_pane`/`git_commit_detail::show`)와 팔레트(`palette_commands` → `command_dispatch` → `shell_keymap::intent` → `apply_git_intent`)를 지나며, git 명령은 host worker의 transient task로 임시 저장소에서 실행됩니다. 원격·AI·Keychain·휴지통·실제 알림은 쓰지 않았습니다.

## 5. 남은 범위와 관찰

- 분리선 키보드 크기 조절 없음(원본 Separator는 화살표 키로 조절 가능). 필요하면 shell 분할선의 `KEYBOARD_RESIZE_STEP` 방식으로 추가할 수 있습니다.
- 그래프 행 Enter/Space 선택과 상세 파일 행 Enter/Space 열기는 egui 클릭(focus된 위젯의 키 활성)으로 같은 분기를 타지만 키 입력 검사는 따로 두지 않았습니다(클릭 검사가 같은 코드 경로).
- 관찰(다른 lane): shell의 slot focus 판정(`UI/shell.rs` `pointer_in_body`)이 모달 위 클릭도 그 아래 slot 영역 클릭으로 봅니다. 두 slot 화면에서 대화상자 버튼이 다른 slot 위에 있으면 클릭 뒤 focus slot이 바뀝니다(원본은 portal 대화상자라 바뀌지 않음). discard·stage-all 대화상자도 같습니다. 이 묶음 검사는 명령 순서를 바꿔 영향을 피했고, 수정은 shell 영역입니다.
- git-26 행 갱신(그래프 머리말 정거장 연결)은 병합 때 반영할 근거만 남겼습니다(1절).
