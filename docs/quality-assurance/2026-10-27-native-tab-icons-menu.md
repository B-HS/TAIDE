# 탭 에이전트 아이콘과 탭 메뉴 남은 항목

## 대상

- shell-75: 탭 종류별 아이콘입니다. 이번에는 agent가 감지된 터미널 탭의 아이콘·활동색·세션 tooltip을 다룹니다.
- shell-78: 탭 context menu의 남은 항목입니다. 이번에는 Explorer 보기 reveal, rename, 다시 열기(에디터/preview)를 다룹니다.
- 원본: `src/widgets/editor-area/pane-tab-bar.tsx`, `src/features/tab/tab-item.tsx`, `src/features/tab/tab-context-menu.tsx`

## 원본 동작

- 탭 아이콘(`pane-tab-bar.tsx` 53-70, 104, 181-188)
    - 터미널 탭의 `sessionId`가 비어 있지 않으면 `projectAgents`에서 같은 세션의 agent를 찾습니다.
    - agent가 있으면 `Sparkles` 아이콘을 `ICON_AGENT_ACTIVITY_CLASS[activity]` 색(`app-sidebar-icon-agent-*`)으로 그립니다. 없으면 기존 터미널 아이콘입니다.
    - agent가 있으면 아이콘 칸에 `agent.sessionTooltip`(이름 — 상태) tooltip을 아래쪽에 붙입니다(`tab-item.tsx` 58-65).
- 탭 메뉴(`tab-context-menu.tsx` 121-159, `pane-tab-bar.tsx` 206-233)
    - 파일 탭의 Finder reveal 묶음에 `tab.revealInExplorerView`, `tab.rename`, `tab.openChanges`, `git.fileHistory`가 이어집니다.
    - 미리보기 가능한 파일(`resolvePreviewKind`)이면 묶음 뒤에 `tab.reopenEditorWith` 하위 메뉴(`explorer.openWithEditor`/`explorer.openWithPreview`)가 붙습니다.
        - 선택은 `setOpenWithOverride(path, 'editor' | null)`만 합니다(`open-with-registry.ts` 32-39). 탭을 다시 열지 않습니다.
    - reveal/rename은 bridge로 방송되고, 포커스된 slot의 explorer만 받습니다(`explorer-panel.tsx` 151-173).
        - 보기를 files로 바꾸고, 접힌 sidebar를 펼칩니다(`project-shell.tsx` 92-123).
        - `tree_reveal`로 조상을 펼친 페이지를 받아 경로를 선택·스크롤합니다(`explorer-container.tsx` 224-241, `file-tree.tsx` 311-317).
        - rename은 같은 페이지의 행으로 inline rename을 시작합니다.
    - runtime `tree_reveal`은 조상 디렉터리를 모두 펼친 전체 페이지를 돌려줍니다(`crates/taide-runtime/src/tree_actions.rs` 142-160).

## 구현

- UI(`native/taide-native-ui`)
    - `glyph-icons.rs` 614-627: `tab(kind, has_agent)`가 agent 있는 터미널을 `Sparkles`로 풉니다. 원본 `getTabIcon(kind, agent?)`와 같은 분기입니다.
    - `shell.rs` 1138, 1342-1371: 탭바가 `project_agents`를 한 번 읽습니다.
        - 세션 id가 비어 있지 않은 터미널 탭만 같은 세션 agent를 찾습니다.
        - 활동별 rail 색(`appSidebar.iconAgent*`, 원본과 같은 토큰)을 `tab_icon`에 넘깁니다.
        - 아이콘 칸 hover에 아래쪽 정렬 `agent.sessionTooltip`을 보입니다.
    - `ShellSurfaces::tab_icon`에 `agent_color` 인자를, `can_reopen_with`에 기본 구현(false)을 추가했습니다. 동결 remote-web은 이 trait을 구현하지 않아 영향이 없습니다.
    - `shell.rs` 1763-1792: 파일 탭 메뉴가 원본 순서로 `Finder reveal → Explorer 보기 reveal → rename → 다시 열기 하위 메뉴`를 보입니다.
        - reveal/rename은 주 창 slot에서만 보입니다. `PaneViewContext`/`TabMenuContext`에 slot을 실어 `ShellIntent::RevealInExplorer { project, slot, path, rename }`을 냅니다. 메뉴 선택 intent 앞에는 기존 규칙대로 `FocusSlot`이 들어가 원본의 "포커스된 slot" 조건과 같습니다.
        - 다시 열기는 `can_reopen_with`가 참일 때만 보이고 `ShellIntent::ReopenWith { path, preview }`를 냅니다.
- App(`native/taide-native-app`)
    - `application.rs` 9516-9536: `tab_icon`이 agent 색이면 `Sparkles`를 그 색으로 그립니다. `can_reopen_with`는 기존 `open_with::preview_kind`(원본 `resolvePreviewKind` 확장자 표)를 파일 이름에 적용합니다.
    - `application.rs` 8083-8119
        - reveal/rename: 해당 slot의 symbol sidebar를 Files로 바꿉니다. sidebar가 접혀 있으면 기존 `SetSidebarCollapsed(false)`로 펼칩니다. 그다음 `HostCommand::RevealExplorerPath`를 보냅니다.
        - 다시 열기: 기존 `open_with::Registry::set`(Editor/Preview)만 호출합니다.
    - `host.rs` 1975-1993: 기존 runtime `tree_actions::tree_reveal`을 호출해 `HostReply::ExplorerPathRevealed`로 돌려줍니다.
    - `application.rs` 1819-1834: 성공하면 tree 페이지를 교체하고 `Explorer::select_path`(`explorer.rs` 541-544, 기존 `select_index` 재사용)로 선택·스크롤합니다. rename이면 같은 페이지 행으로 기존 `start_rename`을 시작합니다. 실패는 다른 tree 응답과 같이 상태줄에 보입니다.
- 범위 밖 수정: `application-project-rail-tests.rs`의 기존 fixture·입력 helper를 `pub(super)`로 바꿔 새 앱 검사에서 재사용했습니다(동작 변경 없음).

## 검증

- `cargo test --manifest-path native/taide-native-ui/Cargo.toml … --features native-host,inspection --test workbench --lib`는 26/26, 158/158 통과입니다.
    - 새 실제 egui 검사 2개
        - 활동 4종의 agent 색 전달을 확인했습니다. agent 없는 세션, 빈 세션 id(같은 빈 id agent가 있어도), 파일 탭은 None입니다.
        - 아이콘 hover tooltip 문구와 아이콘 아래 위치를 확인했습니다. 일반 터미널에는 tooltip이 없습니다.
        - 파일 탭 메뉴 순서(Finder → Explorer 보기 → rename → 다시 열기 → 분할)와 Git 변경·파일 이력·창 이동 미표시를 확인했습니다.
        - reveal·rename intent의 project/slot/path/rename 값을 확인했습니다.
        - 하위 메뉴 Editor/Preview 선택의 `ReopenWith` 값을 확인했습니다.
        - 비미리보기 파일은 다시 열기를 숨기고, 터미널 탭은 세 항목을 숨기며, 보조 창은 reveal/rename을 숨깁니다.
    - glyph 단위 검사에 agent 터미널 `Sparkles` 분기를 추가했습니다.
    - 기존 탭 아이콘 정렬·탭 메뉴 검사도 같이 통과합니다.
- `cargo test --manifest-path native/taide-native-app/Cargo.toml … --lib tab_menu_tests`는 1/1 통과입니다.
    - 실제 `NativeApplication`(임시 project, 합성 파일, 합성 agent)에서 진행했습니다.
        - AgentStore에 합성 agent를 넣으면 `native-glyph-Sparkles-` 텍스처가 생깁니다. 넣기 전에는 없습니다.
        - 탭 우클릭 → 다시 열기 하위 메뉴 Editor/Preview가 `open_with` surface를 Editor/Image preview로 바꿉니다.
        - Explorer 보기 reveal이 다음을 반영합니다.
            - 접힌 sidebar 펼침(runtime layout)
            - Outline에서 Files로 보기 전환
            - `src/nested/deep.rs`의 조상 펼침과 선택
        - rename이 같은 행의 inline rename draft를 엽니다.
- `cargo test … --lib problems_icons`(앱 glyph 호환 포함)는 1/1 통과입니다.
- App `--all-targets` check와 동결 remote-web Wasm lib check는 exit 0입니다. 경고는 모두 기존 경고입니다.
- `cargo fmt -- --check` 결과
    - UI는 exit 0입니다.
    - App은 이번 변경 구간 diff가 0입니다. 기존 diff(`application.rs` 908 들여쓰기·테스트 mod 순서, `lib.rs` mod 순서, `experiments/lsp-coordinator-spike` mock-server)는 변경 전부터 남아 있습니다.

## 남은 범위

- 원본과 다른 점이 있습니다. 원본의 panel `expand()`는 저장하지 않는 표시 상태입니다. native에는 비저장 펼침 상태가 없어 기존 저장 경로 `SetSidebarCollapsed(false)`를 씁니다.
- shell-78 연결 대기 항목
    - `tab.openChanges`, `git.fileHistory`: Git 변경 diff 탭·파일 이력 패널이 native에 없습니다(배치 26).
    - `tab.moveToNewWindow`, `tab.moveToMainWindow`, `tab.moveToWindowNumbered`: 보조 창 이동 소비자가 없습니다(배치 24 단계 D).
    - 보조 창의 Explorer 보기 reveal/rename: native 보조 창에 explorer가 없습니다(단계 D).
- 메뉴 아이콘(lucide)은 기존 native 메뉴 항목처럼 텍스트만 보입니다. 화면 픽셀과 실기는 배치 31입니다.
