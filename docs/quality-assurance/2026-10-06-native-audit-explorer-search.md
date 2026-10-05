# Native 전환 감사: 탐색기·검색·아웃라인·문제 패널

감사일 2026-10-06, 읽기 전용 감사입니다. 이전 에이전트의 HANDOFF.md, PROCESS.md 수치는 사용하지 않고 실제 파일과 호출 체인으로만 판정했습니다. 빌드와 테스트는 실행하지 않았습니다.

## 1. 범위

TS 기준(전부 읽음, 테스트 제외)

- /Users/hyunseokbyun/development/TAIDE/src/widgets/explorer/ (explorer-panel, explorer-container, use-explorer-entry-crud, use-explorer-clipboard, use-explorer-auto-reveal, paste-plan, open-to-the-side-plan, file-tree-git-status)
- /Users/hyunseokbyun/development/TAIDE/src/widgets/search-panel, outline-panel, problems-panel
- /Users/hyunseokbyun/development/TAIDE/src/features/explorer, search, outline, problems
- /Users/hyunseokbyun/development/TAIDE/src/entities/tree, search
- 보조로 shared/lib/entry-name.ts, typeahead.ts, shared/ui/file-group-header.tsx, widgets/auxiliary-window-shell, inventory 문서 2026-09-28-ts-view-inventory.md

native 대조

- /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src 의 explorer.rs, explorer_toolbar.rs, explorer_clipboard.rs, explorer_clipboard_owners.rs, explorer_delete.rs, explorer_move.rs(개요만), explorer_source_missing.rs(개요만), delete_dialog.rs, open_with.rs, problems.rs, problems-icons.rs, lsp-diagnostics.rs, remote-search.rs
- application.rs 의 AppSurfaces(ShellSurfaces 구현), host.rs 의 Tree 계열 명령, native/taide-native-ui/src/shell.rs(project_shell, 보조 창 분기), shell_keymap.rs, keymap-defaults.json
- /Users/hyunseokbyun/development/TAIDE/crates/taide-tree, taide-search, taide-runtime(tree_actions, search_actions), taide-lsp/src/native, taide-model(tree.rs, layout.rs, settings.rs)

## 2. 요약

- 탐색기 파일 트리 한 개 뷰만 native 로 실제 앱에 연결돼 있고 동작도 대부분 TS 와 대응합니다(생성, 이름 변경, 삭제, 클립보드, 컨텍스트 메뉴 일부, 키보드, 감시 갱신).
- 사이드바는 `ShellSurfaces::explorer` 하나만 호출하므로 files/search/git/outline 4개 뷰 전환 자체가 없습니다. 검색 패널과 아웃라인 패널의 native UI 는 존재하지 않습니다.
- 검색 백엔드(taide-search, taide-runtime::search_actions)는 완성돼 있으나 native UI 가 호출하지 않고, 원격 게이트웨이(remote-search.rs)로만 노출됩니다. 아웃라인용 LSP documentSymbol 요청 타입은 taide-lsp native 에 있으나 앱이 요청하지 않습니다.
- 문제 패널은 TS 와 거의 동일하게 구현·연결돼 있습니다(진단 소스가 LSP 로 한정되는 차이만 남음).
- 파일 트리 행에 chevron, 파일/폴더 아이콘, Git 데코레이션이 없고, 자동 reveal, 루트 누락 복구, 보조 창 탐색기, 컨텍스트 메뉴의 5개 항목(터미널 열기, 폴더에서 찾기, 비교 선택 2종, 파일 히스토리)이 없습니다.

## 3. 기능 대응표

상태 표기는 done / partial / unwired / missing / n/a 입니다. native 근거 경로는 모두 /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/src 기준(별도 표기 제외)입니다.

### 3.1 탐색기 패널과 파일 트리

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- |
| 사이드바 4개 뷰 tablist(files/search/git/outline) | src/widgets/explorer/explorer-panel.tsx:26-31,175-200 | missing | application.rs:4641-4750 의 explorer() 는 파일 트리만 그림, native/taide-native-ui/src/shell.rs:545-555 가 explorer() 만 호출 | tablist, 뷰 상태(프로젝트·슬롯별), 키보드 전환, 접근성 역할. 검색어 sidebarSwitchLabel, ExplorerView 등으로 native 전체 검색 시 결과 0건 | M |
| 헤더(프로젝트명 대문자) + 툴바 4개(새 파일/새 폴더/새로고침/모두 접기), hover·focus 시 표시 | explorer-panel.tsx:202-209, file-tree-toolbar.tsx | done | explorer.rs:670-732, explorer_toolbar.rs:1-131(아이콘 직접 그림, 툴팁 연결), application.rs:4672-4677 | 없음 | - |
| 가상 스크롤, 로딩 상태 | file-tree.tsx:122-128 | done | explorer.rs:819-845(show_rows, 고정 22px), application.rs:4655-4661(스피너, load_trees) | 없음 | - |
| 행 들여쓰기, 이름 표시 | file-tree-row.tsx:53-83 | done | explorer.rs:871-917 | 없음 | - |
| 행 chevron(확장 방향) | file-tree-row.tsx:66-68 | missing | explorer.rs 행 렌더는 Label 만 그림. chevron, Chevron 검색 시 problems-icons.rs 의 Glyph::ChevronRight(문제 패널 전용, lib.rs:98-100 에서 비공개 mod)만 존재 | 확장 상태를 시각으로 알 수 없음(rows 의 expanded 플래그는 키보드 로직에만 사용) | S |
| 파일·폴더 타입 아이콘(FileTypeIcon, FolderTypeIcon) | file-tree-row.tsx:69-75, 입력 폴더명·확장자별 | missing | explorer.rs 에 아이콘 호출 없음. problems-icons.rs:200-246 의 file() 은 8색 coarse glyph 로 비공개 | 확장자별 아이콘, 폴더 열림/닫힘 아이콘, 테마 색 | M |
| Git 데코레이션(행 색, A/M/D/R/U/! 배지, 폴더 점, 조상 전파) | widgets/explorer/file-tree-git-status.ts:48-66, file-tree-row.tsx:25-42,77-81 | missing | TreeRow(crates/taide-model/src/tree.rs:11-20)에 git 필드 없음. native-app 에서 git status 소비처는 remote-git.rs, event-relay.rs, projects.rs 뿐(UI 아님) | git status 조회 -> 경로별 데코레이션 맵 -> 행 렌더. Git 영역 감사와 의존 | M |
| 선택/포커스 시각 구분(selected vs focused) | file-tree-row.tsx:63 | partial | explorer.rs:904-911 선택은 한 색(selection.bg_fill)만 | 트리 포커스 유무에 따른 두 가지 배경 | S |
| 디렉터리 확장·접기(클릭, ←/→, 부모로 이동) | file-tree.tsx:156-159,234-253 | done | explorer.rs:937-945,1451-1474, host.rs:1406-1420 | 없음 | - |
| 단일/Shift/Cmd·Ctrl 다중 선택 | file-tree.tsx:141-161, shared/lib/list-selection | done | explorer.rs:336-386(select_row, 앵커 포함) | 없음 | - |
| 키보드 이동(↑↓), 선택 행 스크롤 | file-tree.tsx:130-139(scrollToIndex, 최소 스크롤) | partial | explorer.rs:388-393, 835-838: 선택 시 reveal 을 세팅하고 `vertical_scroll_offset(index * ROW_HEIGHT)` 로 선택 행을 항상 뷰포트 맨 위로 이동 | 화면 안에 보이는 행으로 이동해도 매번 점프함. 최소 스크롤(가시성 확보)로 교체 필요 | S |
| typeahead(700ms 버퍼) | file-tree.tsx:168-175, shared/lib/typeahead.ts | partial | explorer.rs:1478-1494(0.7초, 선택 행부터 순환) | 선택 행이 없을 때 TS 는 맨 위부터 검색, native 는 무시(1479-1485). IME 중 보호는 있음 | S |
| 파일 열기(클릭, Space, ⌘↓), 폴더는 토글 | explorer-container.tsx:104-107, explorer-shortcuts.ts:34-48 | done | explorer.rs:937-945,1416-1428, application.rs:4686-4693(OpenFileTab) | TS 도 모든 경로가 preview:false 로 연다(대응 동일) | - |
| 인라인 새 파일/새 폴더(⌘N, ⇧⌘N, 툴바, 컨텍스트, 빈 영역 더블클릭, 중첩 이름, 미확장 폴더 자동 확장, 생성 후 열기·선택·reveal, 중복 커밋 방지) | use-explorer-entry-crud.ts:79-131, file-tree-draft-row.tsx | done | explorer.rs:405-542,1497-1575(create_entry: refresh -> reveal -> layout_open_tab), 이름 검증 217-241(예약어, 금지 문자, 끝 점, 중복) | 실패 시 TS 는 토스트 Retry 버튼을 추가로 제공, native 는 인라인 오류+툴팁만(535-540). 낮은 영향 | - |
| 인라인 이름 변경(F2/Enter, IME, 검증, 오류, Escape/blur 취소) | use-explorer-entry-crud.ts:133-178 | done | explorer.rs:544-619, RenameDraft::show(144-215, IME 조합 보호), explorer.rs:1577-1607 rename_entry -> explorer_move::move_selected | native 는 LSP workspace rename 편집과 열린 문서 경로 갱신까지 처리(TS 범위 초과). Retry 토스트 없음 | - |
| 삭제(⌘⌫, 메뉴) + 확인 대화상자 | entry-delete-dialog.tsx, crud.confirmDelete | done | delete_dialog.rs:1-83(Modal, 취소에 초기 포커스, Escape), explorer_delete.rs:1-143, application.rs:4246-4247,4723-4729 | native 는 열린 문서 보존 draft 저장까지 처리(TS 범위 초과). 다중 선택 삭제는 TS 도 단일 행만 | - |
| 잘라내기/복사/붙여넣기(고유 이름 접미사, 같은 폴더 cut 무시, 충돌 재시도 8회, 슬롯별 클립보드) | use-explorer-clipboard.ts, paste-plan.ts | done | explorer.rs:283-324, explorer_clipboard.rs(DESTINATION_ATTEMPT_LIMIT 8), explorer_clipboard_owners.rs, application.rs:4648-4749 | 없음 | - |
| 드래그로 이동 | TS 파일 트리에 drag 구현 없음(grep drag/dnd 결과 0건: features/explorer, widgets/explorer) | n/a | native 에도 없음. explorer_move.rs 는 이름 변경의 이동 처리이지 DnD 가 아님 | TS 에 없는 기능이므로 대응 불필요 | - |
| 컨텍스트 메뉴: 새 파일/폴더, 나란히 열기, 열기 방식(파일 형식 있을 때), 브라우저에서 열기(html), Finder 에서 보기, 잘라내기/복사/붙여넣기/경로 복사/상대 경로 복사, 이름 변경, 삭제 | file-tree-context-menu.tsx:96-216 | done | explorer.rs:951-1140, open_with.rs, application.rs:4694-4745 | 단축키 표기가 mac 글리프 하드코딩(TS 도 동일) | - |
| 컨텍스트 메뉴: 터미널에서 열기 | file-tree-context-menu.tsx:155-158, explorer-container.tsx:160-167 | missing | 시도한 검색어: openInTerminal, open_in_terminal, NewTerminalAt, explorer.openInTerminal(native 전체에서 로케일 외 0건). TabKind::Terminal cwd 필드와 layout_open_tab 은 이미 존재 | 메뉴 항목+Action+HostCommand(cwd 지정 터미널 탭 열기) | S |
| 컨텍스트 메뉴: 폴더에서 찾기 | file-tree-context-menu.tsx:159-164, explorer-container.tsx:177-180 | missing | 시도한 검색어: FindInFolder, find_in_folder, findInFolder, scopeDir, explorer.findInFolder(native 0건) | 검색 패널이 없어 선행 불가 | S(검색 패널 이후) |
| 컨텍스트 메뉴: 비교 위해 선택 / 선택 항목과 비교 | file-tree-context-menu.tsx:132-139, explorer-container.tsx:182-195 | missing | 시도한 검색어: selectForCompare, compareWithSelected, select_for_compare, TabKind::Diff. Diff 탭은 모델에만 있고 application.rs:4916 이후 File 이 아닌 탭은 렌더되지 않음 | 메뉴 상태(compareSourcePath)+Diff 탭 렌더(다른 영역과 의존) | M |
| 컨텍스트 메뉴: 파일 히스토리 | file-tree-context-menu.tsx:140-143, widgets/file-history | missing | 시도한 검색어: fileHistory, file_history, requestOpenFileHistory, git.fileHistory(native 0건) | Git 히스토리 패널 필요(Git 영역과 의존) | L |
| 트리 빈 영역 우클릭/더블클릭(메뉴: 새 파일/폴더/붙여넣기, 선택 해제) | file-tree.tsx:267-295 | done | explorer.rs:1146-1191 | 없음 | - |
| 활성 탭 자동 reveal(`explorerAutoReveal`) | use-explorer-auto-reveal.ts, explorer-auto-reveal.ts | missing | 시도한 검색어: explorer_auto_reveal, explorerAutoReveal, decideAutoReveal, auto_reveal. 설정 UI 스위치만 있음(native/taide-native-ui/src/settings-controls.rs:136,156,180,224)이고 읽는 코드 없음 | 활성 파일 -> tree_reveal -> 선택 로직과 sidebar 가시성 조건 | S |
| 탭 메뉴의 "탐색기에서 보기", "이름 바꾸기" 가 탐색기를 구동 | explorer-panel.tsx:151-173(reveal·rename bridge) | missing | 시도한 검색어: revealInExplorerView, tab.rename, RevealInExplorer. native ShellIntent(native/taide-native-ui/src/commands.rs:53-65)에 해당 의도 없음 | 탭 메뉴(다른 영역)와 탐색기 간 intent. 교차 영역 | S |
| 프로젝트 루트 누락 안내와 "프로젝트 다시 열기" | explorer-panel.tsx:212-221, explorer-container.tsx:88,249-253 | missing | 시도한 검색어: root_missing, rootMissing, projectRootMissing, reopenProject. root_missing 필드는 lsp-recovery.rs:187, projects.rs:124 에서만 사용, 탐색기 UI 소비 0 | 누락 안내 화면, 재열기 액션 | S |
| 파일 시스템 감시 기반 트리 갱신, 새로고침(루트+펼쳐진 폴더), 모두 접기 | tree.query.ts, explorer-container.tsx:261-267 | done | application.rs:3573-3590(tree_changes -> RefreshTree), host.rs:1196-1260,1421-1436 | 없음 | - |
| 트리 로드 오류 표시 | explorer-container.tsx:63,88(isTreeRowsError -> 안내 화면) | partial | application.rs:1306 오류를 상태줄 문자열(status)로만 기록, 재시도·빈 상태 안내 없음 | 오류 전용 화면+재시도 | S |
| 보조 창(auxiliary window)의 탐색기 패널(+접기, 폭 조절) | widgets/auxiliary-window-shell/auxiliary-window-shell.tsx:125-141 | missing | native/taide-native-ui/src/shell.rs:133-161 보조 창은 타이틀바+pane_tree 만 그림 | 보조 창 사이드바. 이 영역과 창 영역의 교차 | M |

### 3.2 검색·교체 패널

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- |
| 검색 패널 본체(질의 입력, 대소문자/단어/정규식/.gitignore 토글, 제외 glob, 최근 검색 드롭다운, 범위 칩, 실행·결과 상태 문구) | search-panel.tsx, search-option-toggles.tsx, search-exclude-glob-input.tsx, search-history-dropdown.tsx, search-panel-container.tsx:36-251 | missing | 시도한 검색어: SearchQuery, search_run, SearchFileMatches, search_replace, taide_search, "search." 로케일 키 사용(native 전체에서 소비는 remote-search.rs, 설정 화면 제목 1건 settings-view.rs:1600 뿐) | UI 전부 | L |
| 결과 목록(파일 그룹 접기, 매치 행 하이라이트, 컨텍스트 라인, 가상 스크롤, 클릭 시 reveal 하며 열기, 잘림 안내) | search-results-list.tsx, search-result-rows.ts, search-match-row.tsx, entities/search/search-result.ts | missing | 위와 동일(결과 누적·병합·context 중복 제거 로직도 없음) | 스트리밍 누적기, 가상 목록, OpenFileTab+reveal(editor_reveal.rs 에 reveal 수단은 있음) | L |
| 교체(토글, 입력, 파일별 체크, 확인 대화상자, 질의 일치 전에는 비활성+안내, 결과 요약 토스트, 건너뜀 보고, 네이티브 알림, 교체 후 재검색) | search-panel.tsx:108-133,175-197,268-281, search-panel-container.tsx:124-162, replace-skip-report.ts | missing | 백엔드 search_actions::search_replace 만 존재(crates/taide-runtime/src/search_actions.rs:105) | UI 전부, notify_search_replace 설정 연결(consumer 0건) | M |
| 입력 시 검색(searchOnType, 디바운스) | search-panel-container.tsx:184-203 | missing | settings-controls.rs:140,184,240,364-409 설정 스위치만 있고 소비 코드 없음 | 디바운스 실행, 실행 취소 | S |
| 검색 이력 저장(최근 20개, 쓰기 경합 큐) | entities/search/search-history.ts | missing | Settings.recent_searches 필드는 있음(crates/taide-model/src/settings.rs:342), native 쓰기·읽기 UI 없음 | 이력 큐와 갱신 패치 | S |
| 검색 에디터 탭으로 열기 | search-panel-container.tsx:164-176, widgets/search-editor | missing | TabKind::SearchEditor 는 모델에만 있음(crates/taide-model/src/layout.rs:92). application.rs 탭 렌더에 해당 분기 없음(4759-4916) | 탭 렌더(교차 영역: 탭·에디터) | L |
| ⌘⇧F, ⌘⇧H, 선택 텍스트 seed, 포커스·전체 선택, openReplace | explorer-panel.tsx:131-140, editor-area.tsx:297 | missing | keymap-defaults.json 에 search, search-replace, explorer, git 항목은 있으나(39-61) shell_keymap.rs:7-37 ACTIONS 에 없어 키 입력이 아무 동작도 하지 않음 | 키 핸들러+패널 열기 | S |
| 검색 백엔드(전체 파일 순회, 정규식/단어/대소문자, gitignore, scopeDir, 취소, 교체 + 건너뜀 보고, list_files) | entities/search/search.ipc.ts | unwired | crates/taide-search/src/service.rs(1605줄), crates/taide-runtime/src/search_actions.rs(81,105,176,182), SearchStore. 호출처는 src-tauri/src/domain/search/commands.rs 와 native remote-search.rs(원격 게이트웨이 전용, 브라우저 Wasm 쪽 UI 도 없음: taide-remote-web/src 에 search 심볼 0건) | 백엔드는 완성. native UI 에서 services.search 를 사용하지 않음 | - |

### 3.3 아웃라인

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- |
| 아웃라인 패널(심볼 트리 평탄화, 접기, 종류 아이콘, 가상 스크롤, ↑↓←→ Enter 키보드, 빈 상태 2종, 클릭 시 심볼 위치로 reveal, 문서 변경 시 400ms 갱신) | outline-panel.tsx, outline-symbol-row.tsx, outline-rows.ts, outline-panel-container.tsx:27-68 | missing | 시도한 검색어: DocumentSymbol, documentSymbol, outline(로케일 키 outline.title/empty/noActiveFile), SymbolKind. 결과는 taide-lsp native 의 요청 매핑(crates/taide-lsp/src/native/feature.rs:115, registration.rs:86, capabilities.rs:29)뿐 | UI+LSP 요청 호출+평탄화/접기+reveal | L |
| LSP documentSymbol 호출 경로 | document-symbol-session-waiters.ts | unwired | SessionClient::request_typed(crates/taide-lsp/src/native/session.rs:311)와 DocumentSymbolRequest 라우팅은 있으나 native-app 은 request_typed 를 호출하지 않음(src, tests 검색 결과 0건) | 앱 측 요청 어댑터, 세션 대기 | M |

### 3.4 문제 패널

| 기능 | TS 근거 | native 상태 | native 근거 | 빠진 것 | effort |
| --- | --- | --- | --- | --- | --- |
| 헤더(제목, 심각도 4종 필터와 개수, 닫기) | problems-panel.tsx:60-73, problem-severity-filter.tsx | done | problems.rs:305-447, application.rs:4617-4640 -> shell.rs:556-567(분할 영역, 리사이즈) | 없음 | - |
| 파일 그룹/문제 행 목록(경로 정렬, 줄·열 정렬, 접기, 가상 스크롤, 빈 상태 2종, source·위치 표시) | problems-panel.tsx:75-119, problem-list-rows.ts | done | problems.rs:112-173,455-663 | 없음 | - |
| 문제 클릭 시 파일 탭(preview) 열고 위치 reveal | problems-panel-container.tsx:46-47 | done | problems.rs:709-726, application.rs:4631-4639, host.rs:1277-1303(OpenProblem -> layout_open_tab+reveal) | 없음 | - |
| 패널 열기/닫기(상태바 오류 개수 버튼, 슬롯별 상태, 보조 창은 닫힘 고정) | status-bar-content.tsx, project-shell.tsx | done | problems.rs:229-303, application.rs:5132 | 없음 | - |
| 문제 데이터 소스 | useMonacoMarkers(Monaco 마커: LSP 진단 + 내장 TypeScript 모드 등) | partial | lsp-diagnostics 경로: application.rs:3066-3079(publish), diagnostics.rs:84(열린 문서에 한정). 소스는 LSP 만 | 내장 언어 검증(TS 쪽 builtin-typescript-mode.ts 가 관여하던 마커)에 해당하는 진단 소스 없음. LSP 미설치 상태에서는 문제가 비어 있음. 확인 필요 사항으로 6장에 기록 | M |

## 4. 잘못 구현됐거나 보강이 필요한 코드

1. 선택 이동 시 스크롤이 항상 맨 위로 점프합니다. explorer.rs:388-393 에서 reveal 을 세팅하고 835-838 이 `vertical_scroll_offset(index * ROW_HEIGHT)` 를 적용합니다. TS 는 `scrollToIndex` 로 필요한 만큼만 이동합니다. 화면 중간 행을 ↓로 이동해도 목록이 매번 움직이는 UX 결함입니다.
2. 단축키가 보이지만 동작하지 않습니다. native/taide-native-ui/src/keymap-defaults.json:39-61 에 search, search-replace, explorer, git 이 있고 키 바인딩 편집기에 표시되지만 shell_keymap.rs:7-37 의 ACTIONS(31개)에 없어 처리되지 않습니다.
3. 설정만 있고 소비자가 없습니다. explorerAutoReveal, searchOnType, searchOnTypeDebounceMs, notifySearchReplace(native/taide-native-ui/src/settings-controls.rs:136,140,191,364). native-app 에는 읽는 코드가 없습니다.
4. 탐색기 상태가 ProjectId 단위입니다. application.rs:178 `explorers: HashMap<ProjectId, Explorer>` 와 4662 의 `entry(project)` 때문에, 같은 프로젝트를 두 슬롯에 열면 선택·초안(rename/create)·pending 상태를 공유합니다. 클립보드는 슬롯 단위(explorer_clipboard_owners.rs)라 불일치합니다. TS 는 컨테이너 인스턴스 단위입니다.
5. 트리 로드 실패가 상태줄 문자열로만 남습니다(application.rs:1306). 이후 explorers 캐시나 로딩 플래그 상태에 따라 스피너가 남을 수 있습니다(확인 필요, 6장).
6. 보조 창에는 탐색기가 없습니다(shell.rs:133-161). TS 보조 창은 탐색기+에디터 구성입니다.
7. typeahead 는 선택 행이 없으면 입력을 무시합니다(explorer.rs:1479-1485). TS 는 맨 위부터 검색합니다. 선택 배경이 포커스 유무와 무관하게 한 색입니다(explorer.rs:904-911).
8. 아이콘 자산이 문제 패널 전용 비공개 모듈에 갇혀 있습니다(lib.rs:98-100 `mod problems_icons`, problems-icons.rs:13-246). 탐색기 아이콘을 추가할 때 중복 구현하지 않도록 공용 모듈로 올려야 합니다. 설계 사항입니다.
9. 문제 패널의 데이터는 LSP 진단뿐이고 열린 문서에 한정됩니다(diagnostics.rs:84, application.rs:2598). Monaco 마커 대비 범위가 좁습니다.

## 5. 실제 앱 연결이 끊긴 지점

1. 검색: taide-search -> taide-runtime::search_actions 는 Tauri 명령과 remote-search.rs 로만 호출됩니다. native NativeApplication 은 services.search 를 UI 에서 사용하지 않습니다.
2. 아웃라인: taide-lsp native 의 documentSymbol 요청 타입은 존재하나 native-app 에서 request_typed 호출이 없습니다.
3. 사이드바: ShellSurfaces 트레이트(native/taide-native-ui/src/shell.rs:65-71)가 explorer() 하나뿐이라 다른 뷰를 연결할 접점이 없습니다.
4. 키 바인딩 4종(search, search-replace, explorer, git)은 카탈로그에만 있고 shell_keymap.rs ACTIONS 에서 끊겼습니다.
5. 설정 4종(위 4-3)은 설정 화면에서 값은 저장되지만 소비되지 않습니다.
6. TabKind::SearchEditor, TabKind::Diff 는 모델과 레이아웃 서비스에만 있고 탭 렌더(application.rs:4759-4916)로 이어지지 않습니다.
7. Git 데코레이션 입력: git status 는 remote-git.rs 경로에만 연결되고 탐색기 UI 로 이어지지 않습니다.

## 6. 권장 구현 순서 (의존 관계)

1. 사이드바 뷰 상태와 tablist(E1): 슬롯 또는 프로젝트별 ExplorerView 상태, ShellSurfaces 에 뷰 전환 접점, 키 바인딩(explorer, search, search-replace, git) 연결. 이후 모든 뷰의 선행 조건입니다.
2. 탐색기 시각 보강(chevron, 파일/폴더 아이콘 공용화, 선택/포커스 구분), 스크롤 점프 수정. 선행 조건 없음, 1과 병렬 가능합니다.
3. 검색 패널 UI: 질의/토글/제외 glob -> 스트리밍 결과 목록 -> 클릭으로 열기+reveal. search_actions 를 호출하는 호스트 명령을 추가합니다(1 이후).
4. 교체 흐름, searchOnType, 이력, notify_search_replace 연결, 폴더에서 찾기(탐색기 메뉴). 3 이후입니다.
5. 아웃라인: LSP documentSymbol 요청 어댑터 -> 패널 -> reveal. 1 이후이며 3 과 병렬 가능합니다.
6. 자동 reveal, 루트 누락 복구, 터미널에서 열기, 트리 오류 화면. 독립 작업입니다.
7. Git 데코레이션과 파일 히스토리, 비교 메뉴, Diff 탭 렌더: Git·탭 영역 감사의 구현과 맞춰 진행합니다.
8. 보조 창 사이드바, 탭 메뉴에서의 reveal/rename 연동, 검색 에디터 탭: 창·탭 영역과 조율합니다.
9. 문제 패널 데이터 소스 확장(내장 검증 대응 여부 결정).

## 7. 확인하지 못한 것

- 빌드, 테스트, 실제 앱 실행을 하지 않았습니다. native 의 tests/explorer*.rs 가 어떤 동작까지 검증하는지는 읽지 않았습니다.
- TS 의 `FileTypeIcon`, `FolderTypeIcon` 의 정확한 매핑 규모는 읽지 않아 아이콘 effort 는 추정입니다.
- explorer_move.rs, explorer_source_missing.rs 는 전체 본문이 아니라 개요만 확인했습니다(이름 변경과 삭제의 LSP·문서 후처리).
- 트리 로드 실패 후 스피너가 계속 남는지(application.rs:1306, 4655-4661 의 상호작용)는 정적 추정입니다.
- 문제 패널에 TS 쪽 내장 TypeScript 진단이 실제로 활성인 시나리오가 있는지(builtin-typescript-mode.ts 의 활성 조건)는 확인하지 않았습니다.
- 다른 에이전트가 병렬로 감사하는 Git, 탭, 창 영역과 교차하는 항목(탭 메뉴 연동, Diff 탭, 파일 히스토리, 보조 창)은 이 영역 관점에서만 기록했습니다.
- 사용한 도구 제약상 Grep 도구가 없어 grep 명령으로 검색했습니다. 위 시도한 검색어는 본문 각 행에 기록했습니다.
