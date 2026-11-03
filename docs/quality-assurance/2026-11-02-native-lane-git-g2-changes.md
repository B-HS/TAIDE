# native lane git 묶음 2/8 (g2-changes) — 변경 목록 상호작용

> 작성일: 2026-11-02 · worktree `TAIDE-lanes/git`(branch `lane/git`) · 사양 `docs/research/2026-10-28-lane-git-spec.md` git-17·20~26·3.1 G2
> 결정: `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안), `docs/acknowledge/2026-10-26-native-autonomous-continuation.md`
> 선행: G1(`2026-11-02-native-lane-git-g1-base.md`)의 미커밋 변경 위에서 작업했습니다.

## 1. 대상과 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| git-17 | complete | 평탄화(행 key = 그룹+경로)·`show_rows` 가상화·sticky 머리말(불투명, 행 위·다음 머리말 아래)·키보드 스크롤 위 여백 24px |
| git-20 | complete | 그룹별 행 버튼 실행: Stage·Unstage(transient task, 성공 시 캐시 stale, 실패 오류 toast)·Discard(git-22)·Open File(파일 탭) |
| git-21 | complete | Unstage All·Stage All, hover/머리말 focus 때만, 클릭이 접힘으로 번지지 않음 |
| git-22 | complete | 확인 대화상자(1개 경로/다건 개수), Cancel 처음 focus, Escape 취소, 확인 시 discard(추적 파일만 실검사) |
| git-23 | complete | 목록 공유 행 메뉴, 그룹별 항목·순서, 우클릭 단독 선택, 머리말 메뉴 없음, 대상 소멸 시 닫힘, Copy Path·Reveal은 intent 값만 단언 |
| git-24 | complete | 클릭·Enter/Space → 단독 선택 + diff 탭(절대 경로, staged 구분, before_path, 제목 `"{파일명} (diff)"`), 같은 행 재클릭 탭 1개 |
| git-25 | complete | `resolveListSelection` 이식, Shift 범위가 그룹 경계를 넘음, Cmd/Ctrl 토글, 수식키 클릭은 탭을 열지 않음, 선택 배경·pressed |
| git-26 | partial | 변경 목록 안 roving 전부. stash 머리말(G3)·그래프 머리말(G6) 정거장은 해당 묶음에서 연결(순수 함수는 준비) |

## 2. 원본 근거

사양 각 절을 따랐고, 이번에 확인한 원본 구간만 적습니다.

- 평탄화·sticky·roving: `src/widgets/git-panel/change-row-navigation.ts`(+ `.test.ts`), `git-panel.tsx:252-289`(virtualizer `scrollPaddingStart` 24, overscan 12, `rangeExtractor`가 sticky 머리말을 범위에 넣음), `382-405`(↑/↓, 행 안 버튼 focus면 무시), `451-455`(스크롤 뒤 다음 렌더에 focus 재시도), `563-611`(sticky 머리말만 흐름 배치, 나머지 절대 배치).
- 머리말: `src/features/git/git-section-header.tsx`(→ 접힘이면 펼침, ← 펼침이면 접힘, Enter/Space 토글, 그룹 버튼은 hover·focus-within 때만, 클릭 `stopPropagation`, 버튼 hover 배경 `explorer.itemSelected`).
- 그룹 config: `src/features/git/git-change-group.tsx`(행 버튼·행 메뉴·행 클릭, Stage All/Unstage All 대상 = 그룹 행 전부, Open Changes 대상 = 절대 경로 + rename 원래 절대 경로).
- 행: `src/features/git/status-row-item.tsx:48-99`(클릭 대상은 이름 영역 `role=button`, `aria-pressed=selected`, 선택 배경은 hover·focus-within보다 약함).
- 선택: `src/shared/lib/list-selection.ts`(+ `.test.ts`), `git-panel.tsx:414-443`(수식키 클릭은 diff를 열지 않음, 키보드 활성은 단독 선택 + diff).
- 메뉴: `git-panel.tsx:139-146,291-295,407-417,558-628`(목록 전체가 메뉴 하나 공유, 머리말·빈 곳 우클릭은 메뉴 없음, 대상은 path로 기억, 목록에서 사라지면 닫힘).
- discard: `git-panel.tsx:350-354,695-712`, `src/shared/ui/alert-dialog.tsx`(Radix AlertDialog: 처음 focus는 Cancel, Escape는 취소, 바깥 클릭은 닫지 않음).
- 실행: `src/widgets/git-panel/git-panel-container.tsx:208-218,273-279`(diff 탭 제목 `"{파일명} (diff)"`, target null, preview false, 실패 오류 toast; Open File은 `openFileTab` preview false; Copy Path/Reveal은 절대 경로).

## 3. 구현

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-app/src/git-scm-model.rs` | `row_key`, `header_indexes`, `sticky_header_index`(`resolveStickyHeaderIndex`), `Direction`·`FocusTarget`·`next_item_index`·`focus_target`(`resolveNextChangeRowIndex`·`resolveGitPanelFocusTarget`), `Selection`·`resolve_selection`(`resolveListSelection`, 원본 반환의 `primaryId`는 패널이 쓰지 않아 뺐습니다), `RowAction`·`row_menu`(그룹별 메뉴 순서·구분선), `discard_description`(1개 = 경로, 여럿 = `git.fileCount`) |
| `native/taide-native-app/src/git-scm.rs` | 패널 상태에 `selected`·`anchor`·`menu`·`discard`(slot별, 언마운트 때 함께 사라짐). 목록은 행 key(그룹+경로)·머리말 그룹으로 만든 고정 egui id를 씁니다(`List`). `show_rows` 범위 안에서 행 → sticky 머리말(첫 보이는 index 이하 마지막 머리말을 viewport 위에 불투명하게) → 나머지 머리말 순으로 그려 원본의 z 순서(행 < sticky < 다음 머리말)를 맞췄습니다. 화살표는 focus된 항목에서만 소비(egui 방향 이동은 focus lock filter로 막음), 목표가 그려지지 않았으면 위 여백 24px 포함 `scroll_to_rect` 뒤 egui 임시 메모리의 대기 id로 그려지는 프레임에 focus. 머리말 ←/→ 접기·펼치기, Enter/Space(egui 가짜 클릭) 토글. 행은 들여쓴 영역(메뉴용, focus 불가) + 이름 영역 버튼(focus·클릭·`aria-pressed` = accesskit toggled) + hover/focus 안 때 그룹별 버튼. 머리말 그룹 버튼(Unstage All/Stage All)은 hover·focus 안 때만, 버튼 클릭은 토글로 번지지 않습니다. 메뉴는 목록 전체 하나(`egui::Popup`, 포인터 고정, 대상 key마다 id), 대상이 목록에서 사라지면 그리지 않아 닫힙니다. discard 확인은 `egui::Modal`(처음과 이후 Cancel focus 유지, Escape 취소, 바깥 클릭 무시, 확인은 오류 색) |
| `native/taide-native-app/src/domains/git.rs` | `Command::{Stage, Unstage, Discard}`(모두 `spawn_transient`, 결과 `Reply::Mutated`, 성공 = 가변 범위 캐시 stale, 실패 = `Toasts::ipc_error`), `Command::OpenTab`(`layout_open_tab`, target None, preview false, 실패 `HostReply::Failed` → 오류 toast), `MutationOp::{Stage, Unstage, Discard}`, intent 처리(Open File = `editor::Command::OpenFileTab{ pane: None, preview: false }`, Open Changes = `TabKind::Diff{ path, staged, before_path }` + 제목 `"{파일명} (diff)"`) |
| `native/taide-native-ui/src/intents/git.rs` | `RowKey`, `Intent::{Stage, Unstage, RequestDiscard, ResolveDiscard, OpenFile, OpenChanges, Select, RowMenu}` |
| 테스트 | `git-scm-model-tests.rs` 7개 추가, 신규 `application-git-changes-tests.rs`(`domains/git.rs`에 `#[cfg(test)] #[path]` 등록), G1 `application-git-base-tests.rs`의 helper·`Workspace`를 `pub(super)`로 열어 재사용 |

설계 판단(원본에 가장 가까운 안):

- 행 클릭 영역: 원본 행의 클릭 대상은 이름 영역(`flex-1 role=button`)이고 상태 글자·버튼 영역과 들여쓰기는 클릭하지 않습니다. 우클릭은 목록 컨테이너가 받으므로 들여쓰기·글자 영역에서도 메뉴가 열립니다. native도 같은 범위로 나눴습니다.
- 클릭 focus: 브라우저에서 `tabIndex=0` 요소를 누르면 focus가 가므로 행·머리말 포인터 클릭은 focus를 줍니다(focus-within 배경이 선택 배경보다 앞서는 것도 원본과 같음). 우클릭은 focus를 옮기지 않습니다.
- "항목 밖에서 ↓/↑": 원본 스크롤 컨테이너·그룹은 focus를 받지 않아 변경 목록 안에서는 도달하지 않습니다. 순수 함수와 단위 검사로만 옮겼습니다.
- 그래프·stash 머리말 정거장: G3(stash)·G6(그래프)이 아직 없어 `focus_target(.., has_graph_pane = false)`, 항목 수 = 변경 목록입니다. 두 묶음에서 항목 수와 그래프 플래그만 넘기면 됩니다.
- 메뉴 id를 대상 key마다 다르게 둔 이유: egui popup은 이미 열린 같은 id에 클릭이 오면 닫으므로, 열린 상태에서 다른 행을 우클릭할 때 새 행으로 다시 열리게 하려는 것입니다(원본 Radix 메뉴와 같은 결과). 같은 행을 다시 우클릭하면 egui 규칙대로 닫힙니다(원본은 그 자리에서 다시 엶, 사소한 차이).
- 대화상자 위치: 공용 `show_overlays`에 git 호출 줄이 없어(1.4절 표의 "자기 기존 호출 줄 뒤 한 줄" 조건 불충족) 원본처럼 패널 렌더 안에서 `egui::Modal`을 그립니다. 선택은 intent로 다음 상태에 반영합니다. 공용 파일은 이번 묶음에서 고치지 않았습니다.
- 대화상자 동안 전역 단축키: 원본 AlertDialog도 창 전역 키맵 리스너를 막지 않으므로 별도 차단을 두지 않았습니다(egui Modal이 포인터·focus만 가둠).

## 4. 사양·대응표와 실제 코드 차이

- **바깥 클릭(git-22)**: 사양 2.0.6은 "Escape·바깥 클릭은 취소"이지만 원본은 Radix AlertDialog라 바깥 클릭으로 닫히지 않습니다(`src/shared/ui/alert-dialog.tsx`가 dismiss 동작을 바꾸지 않음). 원본을 따라 바깥 클릭은 무시합니다.
- **`resolveListSelection.primaryId`(git-25)**: 패널은 쓰지 않으므로(`git-panel.tsx:440-442`) 이식에서 뺐고, 원본 테스트의 primary 단언은 옮기지 않았습니다.
- **diff 탭 열기 명령(git-24)**: 사양 2.0.2는 모든 git 명령을 transient task로 띄우지만 `OpenTab`은 git 작업이 아닌 layout 변경이라 다른 영역(`editor::Command::OpenFileTab`, terminal 새 탭)처럼 worker에서 바로 await합니다. 실패는 `HostReply::Failed` → 오류 toast로 사양의 `OpenTabFailed`와 같은 결과입니다.
- **rename before_path(git-24)**: G1 QA 4절대로 백엔드 rename 행에 원래 경로가 없어 실제 App에서 `before_path`가 채워지는 경우를 만들 수 없습니다. 코드는 `orig_abs_path`를 그대로 넘깁니다.
- **[A] 행 수 fixture**: 사양의 500행 대신 staged 3 + untracked 500 = 503행입니다(그룹 경계·sticky 전환 확인용).
- **절대 경로**: git이 임시 디렉터리의 `/var` → `/private/var` 링크를 풀어 절대 경로를 주므로 검사 기대값도 정규화 경로로 비교합니다(원본도 같은 백엔드 값 사용).

## 5. 검증

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml --all-targets --locked --offline --target-dir ../targets/git` | 통과(기존 경고만: `is_applying`, `zen.rs:543` mut, `fixture_with_emmet_peek`, vendor wry-preview) |
| `cargo check --quiet --manifest-path native/taide-native-ui/Cargo.toml --all-targets --features native-host,inspection ...` | 통과(기존 경고 `tests/workbench.rs:1816`만) |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib ...` | 통과(remote-web 기존 dead code 경고만) |
| `cargo test ... taide-native-app --lib -- git_scm_model --test-threads=2` | 13 통과(G1 6 + G2 7) |
| `cargo test ... taide-native-app --lib -- domains::git::changes_tests --test-threads=2` | 첫 실행 5개 중 4 실패 → 원인: 검사 쪽 기대 경로 비정규화, 보조 surface `TexturesDelta` 미정리, 같은 문구의 행 버튼·tooltip 노드 혼동, 이전 행 tooltip(hoverable, 아래 행을 덮음)이 우클릭을 받음. 모두 검사 helper 수정(`real`, `rows`, `wide`, `right_click`)으로 해결, 구현 변경 없음 → 5 통과 |
| `cargo test ... taide-native-app --lib -- git title_branch explorer_views_tests shell_dialogs_tests shell_keymap command_dispatch --test-threads=2` | 59 통과(G1 base 5·G2 5·캐시·모델·remote-git 포함) |
| `cargo test ... taide-native-ui --features native-host,inspection --lib -- --test-threads=2` | 167 통과 |
| `cargo fmt --manifest-path <app·ui>/Cargo.toml -- --check` | 통과(`cargo fmt`는 이번 묶음 파일 4개만 바꿈, `git status` 파일 목록 변화 없음) |

자동 검사 내용:

- `[C]` `git-scm-model-tests.rs` 7개: 평탄화 key(같은 경로 두 그룹)·머리말 위치, sticky index(원본 사례), `resolveNextChangeRowIndex`·`resolveGitPanelFocusTarget` 원본 사례 전부(stash·그래프 포함), `resolveListSelection` 원본 사례(일반·토글 추가/해제·Shift·Cmd+Shift·사라진 항목·목록 밖 클릭), 그룹별 메뉴 순서·구분선·문구 키, discard 문구 1개/다건.
- `[A]` `application-git-changes-tests.rs`(main.rs가 쓰는 `NativeApplication`과 frame 경로, 임시 저장소 fixture):
  1. 행 Stage → staged, Unstage → unstaged, Changes 머리말 hover 때만 Stage All 표시·실행 → Changes 사라짐·접힘 상태 불변, 행 클릭 뒤 ↑로 Staged 머리말 focus → pointer 없이 Unstage All 표시·실행, discard: 대화상자 제목·설명(`c.txt`), Cancel focus, Cancel → 내용 불변, Escape → 불변, 확인 → HEAD 내용 복원·행 사라짐, Open File → 활성 탭 `TabKind::File{절대 경로}`.
  2. 행 클릭 → 활성 `Diff{staged false}`·제목 `b.txt (diff)`·pressed, 같은 행 재클릭 → 탭 1개, Staged 행 → `staged true`, Shift 클릭 → Staged a·Changes a·b·c 선택(그룹 경계), Cmd 클릭 → b 해제·탭 수 불변, 우클릭 → Changes 메뉴 순서·단독 선택, 메뉴 Open Changes → c diff·메뉴 닫힘, Staged 메뉴 순서, Escape 닫힘, 머리말 우클릭 메뉴 없음, 메뉴 연 채 테스트 쪽 `git add c.txt` + `GitStatusChanged` → 메뉴 닫힘.
  3. 같은 렌더 함수(`git_scm::show`)에 App의 캐시·패널·외형을 넣은 별도 egui context에서 메뉴의 Copy Path → `ShellIntent::CopyText(절대 경로)`, Reveal → `ShellIntent::RevealPath(절대 경로)`(실행하지 않음, 허락 5).
  4. 503행: 그려진 untracked 행 노드 < 60, 휠 스크롤 뒤 Staged 머리말·첫 행 사라지고 Changes 머리말이 viewport 위에 고정, 위로 복귀, 행 클릭 뒤 ↑ → 머리말 focus, ← 접힘·→ 펼침, ↓ 40번 → 매번 다음 항목 focus(스크롤 밖 행 포함), 마지막 focus 행이 sticky 머리말 아래·화면 안, Enter → 그 행 diff 탭·단독 선택.
  5. 한 행 목록에서 ↓ 두 번은 행에 머묾, ↑ 두 번은 머리말에 머묾(양끝 멈춤).

실행하지 않은 것: 필터 없는 앱 전체 테스트(메인이 병합 때), 추적 안 된 파일 discard(허락 2), 실제 clipboard·Finder reveal(허락 5).

## 6. 남은 범위

- git-26: stash 머리말(G3, git-9)·그래프 머리말(G6, git-30)을 roving 항목 수와 `has_graph_pane`에 넣고, 그래프 머리말에서 ↑/↓를 받는 처리를 그 묶음에서 연결합니다.
- git-17: stash 섹션이 같은 스크롤 영역 아래에 붙는 부분은 G3.
- 메뉴·대화상자의 Copy Path/Reveal/추적 안 된 파일 discard 실제 실행 확인은 허락 뒤.

## 7. 리뷰 수정(2026-11-03)

- git-24: rename 행의 `before_path` 전달은 백엔드 status가 원래 경로를 주지 않아 코드 경로만 있고, 사양의 rename fixture App 검사는 하지 못했습니다. 대응표 행에 한계를 적었습니다.
- git-26: 6절의 남은 범위였던 stash 머리말(G3)과 그래프 머리말(G6) 정거장이 모두 연결됐습니다. G6가 갱신한 `changes_tests::실제_app_화살표는_양끝에서_멈춘다`(↓ 마지막 항목 → 그래프 머리말, 그래프 머리말 ←/→ 접기·펼치기)를 포함한 `domains::git::` App 검사 53개가 이번 수정 뒤 통과해(g5 문서 6.3) 대응표 행을 complete로 갱신했습니다.

