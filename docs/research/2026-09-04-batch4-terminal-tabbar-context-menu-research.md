# 배치 4 조사 원문 — 터미널 우클릭 메뉴(주제 6) · 탭 바 우클릭 메뉴(주제 7) (2026-09-04, wf_6aa56329 opus·xhigh 읽기 전용)

> 계약 정본은 `docs/acknowledge/2026-09-04-usability-batch4-contract.md` F·G 절. 이 문서는 조사 에이전트의 원문(파일:라인 인용)을 보존한다. 라인 번호는 조사 시점(배치 3 구현 wf 진행 중) 기준.

# 주제 6 — 터미널 우클릭 컨텍스트 메뉴


## findings
## 1. 터미널에는 컨텍스트 메뉴가 없다 (사실 확인)

- `src/features/terminal/terminal-view.tsx:358` — 터미널 view 의 전체 마크업은 `return <div ref={containerRef} className='h-full w-full' />` 한 줄이다. `contextmenu` 리스너·Radix `ContextMenu` 는 이 파일에도 `src/widgets/terminal-pane/terminal-pane.tsx`·`terminal-session.tsx` 에도 없다(grep 0 확인).
- `ContextMenu` 사용처는 12개 파일뿐이고 터미널은 그중에 없다: `features/tab/*`, `features/explorer/file-tree*`, `features/git/git-change-group.tsx`, `widgets/app-sidebar/sortable-project-icon.tsx`, `widgets/explorer/*`, `widgets/git-panel/commit-graph.tsx`.

## 2. 우클릭은 지금 "무반응"이 아니라 xterm 이 이미 반응하고 있다

- `node_modules/@xterm/xterm/src/browser/CoreBrowserTerminal.ts:355-357` — xterm 이 `term.element` 에 `contextmenu` 리스너를 직접 건다(Firefox 만 `mousedown` 분기, :348-353).
- `node_modules/@xterm/xterm/src/browser/Clipboard.ts:83-93` `rightClickHandler` — ① 숨은 textarea 를 마우스 커서 아래로 옮기고(`moveTextAreaUnderMouseCursor`, Clipboard.ts:63-78: `width/height 20px`, `zIndex 1000`) ② `textarea.focus()` ③ `textarea.value = selectionText; textarea.select()`. **`preventDefault()` 는 하지 않는다** — OS 네이티브 메뉴의 복사/붙여넣기를 이 textarea 로 받으려는 설계다.
- **`rightClickSelectsWord` 의 기본값은 `isMac`, 즉 macOS 에서 true 다** (`node_modules/@xterm/xterm/src/common/services/OptionsService.ts:50`; 타이핑 선언은 `typings/xterm.d.ts:237`). TAIDE 는 이 옵션을 설정하지 않으므로(`terminal-view.tsx:185-206` 의 Terminal 옵션 목록에 없음), **현재 macOS 에서 터미널 우클릭은 이미 커서 아래 단어를 선택하고 있다**(`SelectionService.ts:802-810` `rightClickSelect` — 기존 선택 영역 안을 클릭한 경우는 보존, 밖이면 단어로 교체).
- `SelectionService.ts:449-457` — button 2 는 선택이 있으면 조기 return, 그 외 비주 버튼도 return. 즉 우클릭이 선택을 지우지는 않는다.
- 마우스 리포팅 모드(vim·tmux 등 TUI 가 켠 경우)에서는 `CoreBrowserTerminal.ts:779-796` 의 `mousedown` 핸들러가 **우클릭 버튼을 pty 로 전송**한다. 컨텍스트 메뉴를 붙이면 "TUI 도 우클릭을 받고 메뉴도 뜬다"가 된다.

## 3. 네이티브 메뉴 억제는 메인 창에만 있다

- `src/widgets/app-shell/app-shell.tsx:101-105` `handleNativeContextMenu` — `input, textarea, [contenteditable="true"]` 안이면 통과, 그 외 `preventDefault()`. `:128-129` 에서 `document` 에 **버블 단계**로 등록.
- `src/app/app.tsx:52-56` — 보조 편집 창(`AuxiliaryWindowShell`)은 `AppShell` 을 마운트하지 않는다. `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx` 에 `contextmenu` 처리 0건. **보조 창의 터미널은 지금 OS 네이티브 메뉴가 그대로 뜬다.**
- Radix 는 자체적으로 막는다: `node_modules/@radix-ui/react-context-menu/dist/index.js` 의 Trigger `onContextMenu` 가 `handleOpen(event); event.preventDefault();` 를 수행한다. React 19 루트는 `#root`(`src/main.tsx:12,22`)에 위임 리스너를 달므로 **React(Radix) → document(app-shell)** 순서로 실행되며, xterm 의 네이티브 리스너(`.xterm` 요소)가 그보다 먼저 돈다.
- 부작용: xterm 이 옮겨 놓은 20×20 textarea 는 `_syncTextArea`(CoreBrowserTerminal.ts:301-325, `onCursorMove` 에서만 호출)가 커서를 움직일 때까지 제자리에 남는다. 유휴 터미널에서 같은 지점을 연속 우클릭하면 두 번째 이벤트의 `target` 이 그 textarea 가 되어 `app-shell` 의 예외 분기에 걸려 **네이티브 메뉴가 커스텀 메뉴와 함께 뜬다**.

## 4. 분할 인프라 — 현재 의미는 "이동"이다

- `src-tauri/src/domain/layout/service.rs:824-875` `split(layout, target_pane, edge, tab_id)`: `extract_tab` 으로 **기존 탭을 소스 트리에서 뽑아** 새 Leaf 에 넣는다. 즉 layout_split 은 **복제가 아니라 이동**이다. `DropEdge::Center` 는 `move_tab` 으로 위임(:828-830), 방향은 Left/Right→Horizontal, Top/Bottom→Vertical(:840-842).
- 트리 변형: 부모 Split 이 같은 방향이면 형제 삽입 + 대상 share 반분(`insert_split_at`, :313-345), 다르면 `wrap_leaf_in_split` 으로 50/50 감싸기(:299-311). 루트 Leaf 도 처리(:852-861). **어떤 방향도 구조적으로는 항상 가능하다.**
- 프론트 경로: `src/entities/layout/layout.ipc.ts:17-18` `splitPane` → `src/entities/layout/layout.query.ts:174` `useSplitPane` → `src/widgets/editor-area/pane-tab-bar.tsx:184` `onSplit={(edge) => splitPane({ paneId, edge, tabId: tab.id })}`.
- **터미널 탭은 이미 4방향 분할을 갖고 있다**: `src/features/tab/tab-context-menu.tsx:150-164` 의 Split 서브메뉴는 탭 kind 무관하게 무조건 렌더된다(`SPLIT_EDGE_OPTIONS`, :30-37). 탭 바에서 터미널 탭을 우클릭하면 상/하/좌/우가 전부 나온다.
- `⌘\` 도 마찬가지다: `src/shared/lib/keymap/keymap.ts:75` `{ id: 'split', key: '\\', mods: ['mod'] }` 에 `when` 이 없고, `src/widgets/editor-area/editor-area.tsx:147-152` `splitActiveEditor` 는 포커스 pane 의 활성 탭을 그대로 오른쪽으로 옮긴다 — 활성 탭이 터미널이면 터미널이 이동한다.
- 결론: 사용자가 요청한 "터미널 우클릭 분할"이 **이동 의미라면 신규 IPC 0**, **"새 터미널 생성" 의미라면 신설이 필요**하다.

## 5. "새 터미널을 그 방향에 생성"에는 단일 커맨드가 없다

- 현재 조합 가능한 유일한 경로는 2회 mutation: `layout_open_tab(target = 현재 paneId)` → `layout_split(paneId, edge, 새 tabId)`.
- 이 순서는 **기존 감사에서 이미 잡았던 결함을 재생산한다**:
  - `open_tab` 은 항상 새 탭을 활성화한다(`service.rs:441,457` `*active = Some(id)`), `PaneNodeView` 는 활성 탭만 렌더하므로(`src/widgets/editor-area/pane-node-view.tsx:131-139`) **기존 터미널 view 가 즉시 unmount → xterm dispose → split 이후 remount → ring buffer 전량 replay** 가 발생한다.
  - 새 터미널 탭은 마운트 즉시 `handleReady` 에서 스폰을 시작한다(`src/widgets/terminal-pane/terminal-session.tsx:139-148`, `terminal-view.tsx:321` 이 생성 이펙트 안에서 동기 호출). 이어지는 split 이 그 탭을 다른 Leaf 로 옮기면 컴포넌트가 다시 마운트되고 `spawnStartedRef`(`terminal-session.tsx:44`)가 초기화되어 **두 번째 셸을 스폰**한다 — 감사 §4-B A6/C14 가 고친 "고아 셸" 패턴과 동일하다.
- `open_tab` 은 `kind` 동등성으로 중복 제거를 한다(`service.rs:436-444`). `TabKind::Terminal { session_id: "", cwd: None }`(types.rs:41-46) 두 개는 같은 kind 라 **세션 id 가 아직 안 박힌 터미널 탭이 있으면 "새 터미널"이 새 탭 대신 기존 탭 활성화로 끝난다.**
- 단일 커맨드로 만들 자리는 이미 정리돼 있다: `service.rs:1261-1290` `open_tab_and_finish` 가 탭 생성 + guard + finish 를, `service.rs:824-875` `split` 이 leaf 삽입을 담당하므로, "새 Leaf 를 만들어 끼우는" 부분(`is_root_target` 분기 + `insert_split_at`)만 뽑아 두 함수가 공유하면 된다.

## 6. "상황에 따라" 비활성 판정의 실제 근거는 최소 pane 크기뿐이다

- `src/widgets/editor-area/pane-node-view.tsx:32` `MIN_PANEL_SIZE_PX = 120`, `:83-88` 의 `<Panel minSize={MIN_PANEL_SIZE_PX}>`.
- `node_modules/react-resizable-panels/dist/react-resizable-panels.d.ts:343-347` — "Numbers are interpreted as pixels". 실제로는 `dist/react-resizable-panels.js:103-110` 에서 그룹 크기 대비 **퍼센트로 환산**된다. 그룹이 240px 미만이면 두 자식의 minSize 합이 100% 를 넘고, 라이브러리는 분할을 거부하는 대신 자체 정규화로 뭉갠다 — **막으려면 앱(메뉴)이 사전에 걸러야 한다.**
- 축 분리: Left/Right 는 pane **폭**을, Top/Bottom 은 pane **높이**를 반분한다. 반대 축은 영향 없다.
- 높이 계산 시 탭 바를 빼야 한다: `src/widgets/editor-area/pane-tab-bar.tsx:242,257` 이 `h-9`(36px), Zen 모드에서는 탭 바 자체가 없다(`pane-node-view.tsx:123-125`).
- 구분자도 공간을 먹는다: `src/shared/constants/layout.ts:1` `DEFAULT_RESIZER_THICKNESS = 1`(설정값), `:7` `RESIZE_HIT_TARGET_SIZE = { fine: 8, coarse: 20 }`.
- 참고 수치: fontSize 기본 13(`src/shared/constants/terminal.ts:9`) 기준 120px 는 대략 15열 / 7행(탭 바 제외 시 ~5행)이라 **레이아웃 최소치 자체가 이미 터미널로서는 한계선**이다.
- 현재 pane 크기를 아는 컴포넌트가 없다. `TerminalSession` 은 `projectId/tabId/sessionId/autoFocus` 만 받고(`terminal-session.tsx:35-43`) `paneId` 도 모른다. DOM 에 `data-pane-id` 류 표식도 없다(grep 0).

## 7. xterm 이 제공하는 메뉴 액션 API (전부 존재, 신규 의존성 불필요)

`node_modules/@xterm/xterm/typings/xterm.d.ts` — `hasSelection():1162`, `getSelection():1168`, `clearSelection():1178`, `selectAll():1191`, `selectLines():1198`, `scrollToBottom():1227`, `clear():1238`, `paste(data):1275`, `reset():1296`.
- `paste()` 는 `Clipboard.ts:50-56` 를 타서 CRLF→CR 정규화 + bracketed paste(`\x1b[200~`) 를 적용하고 `triggerDataEvent(text, true)` 로 발사한다 → `terminal-view.tsx:311-316` 의 `term.onData` → `writePty`. `insertTextDeduper.onXtermData`(`src/shared/lib/ime-input.ts:40-49`)는 armed 된 suppress 가 없으면 그대로 forward 하므로 삼켜지지 않는다. **`writePty` 직접 호출보다 `term.paste()` 가 옳다**(bracketed paste 보존).
- 현재 노출된 핸들은 `TerminalAttachHandle = { write, jumpToPreviousCommand, jumpToNextCommand }`(`terminal-view.tsx:55-59`)뿐이라 확장이 필요하다.

## 8. VS Code 항목 ↔ TAIDE 대응

| VS Code 항목 | TAIDE 현황 | 판정 |
|---|---|---|
| Copy | `term.getSelection()` + `navigator.clipboard.writeText`(선례: `pane-tab-bar.tsx:185`) | 채택 가능 |
| Paste | `navigator.clipboard.readText()` 는 앱 전체에서 **사용처 0** + `term.paste()` | 채택 가능(가드 필요) |
| Select All / Clear | `term.selectAll()` / `term.clear()` | 채택 가능 |
| Split Terminal | §4·§5 | 이 주제의 본체 |
| New Terminal | `layout_open_tab(target=paneId)` — `pane-tab-bar.tsx:126-130`, `tab-bar-add-menu.tsx:33-36`, 팔레트 `command-palette.tsx:152-158`, 키맵 `⌃⇧\``(`keymap.ts:81`) 로 이미 4경로 존재 | 중복 검토 필요 |
| Rename | **탭 제목 변경 커맨드가 존재하지 않는다**(`layout_rename_tab`/`set_title` grep 0) | 제외(백로그) |
| Kill Terminal | `layout_close_tab` 이 `close_tab_and_finish` 로 pty 회수(`service.rs:1292-1301`, `terminal.md` §10) | 채택 가능 |
| Change font size | 상태바(`widgets/window-chrome/status-bar-content.tsx:152-162`)·설정(`settings-terminal-section.tsx:44-48`)에 이미 있음. `terminal.md` §7 이 말하는 `⌘+/-` 커스텀 키 핸들러는 **미구현**(`terminal-view.tsx:228-233` 은 Shift+Enter 만 처리) | 메뉴 제외 권장 |
| Find (⌘F) | `SearchAddon` 은 로드만 되고(`terminal-view.tsx:209,220`) `findNext`/검색 UI 참조가 **0건** — `terminal.md` §8 의 "검색 바" 는 미구현 | 제외(백로그) |
| Run Recent Command | OSC133 블록 모델은 있으나 UI 없음 | 제외(백로그) |

부수 확인: `package.json:41-46` 의 xterm 애드온은 fit/search/unicode11/web-links/webgl 5개뿐 — **`@xterm/addon-clipboard` 는 설치돼 있지 않다**. `terminal.md` §1·§8 의 "clipboard(OSC52)" 서술은 현행과 다르다.

## 9. 로케일 관례

- 3언어 flat JSON: `src-tauri/resources/locales/{en,ko,ja}.json`, `keySeparator: false`(`src/shared/i18n/i18n.ts:11`).
- **Rust 가 키 목록의 정본**: `src-tauri/src/domain/locale/service.rs:13` `MESSAGE_NAMESPACES`, `terminal` 네임스페이스는 :209-219 의 7키(`title, processExited, restart, copyImeDebugLog, imeDebugCopied, runSelectedText, openLinkFailed`). :1300 테스트가 **JSON 에 있으나 등록되지 않은 키를 실패로 만든다** — 신규 키는 Rust 상수 + 3개 JSON 을 동시에 고쳐야 한다.
- 분할 라벨은 이미 있다: `en.json:71-74` `editorArea.splitBottom/Left/Right/Top`("Split Down/Left/Right/Up"), `en.json:874` `tab.split`. `tab-context-menu.tsx:32-37` 가 이미 이 키들을 재사용하므로 터미널도 재사용하면 신규 키가 줄어든다. `en.json:866` `tab.newTerminal` 도 재사용 가능.

## 10. 키맵 충돌 · 원격 미러

- `docs/feedback/2026-08-29-keyboard-shortcut-care.md` — 터미널 포커스에서는 **기본적으로 키를 PTY 로 통과**시키고 앱이 가로채는 키는 명시적 화이트리스트여야 한다는 원칙. 컨텍스트 메뉴는 마우스 제스처라 이 원칙과 직접 충돌하지 않지만, 키보드 대체 진입점(Shift+F10 / ContextMenu 키)을 추가한다면 이 표에 등록해야 한다.
- 원격 미러: `src-tauri/src/domain/remote/dispatch.rs:574` `layout_split`, `:716-717` `pty_spawn`/`pty_attach`, `:669` `pty_write` 전부 **허용 목록에 있다** → 분할·새 터미널·종료는 원격에서도 동작한다. 반면 `:519` `system_open_external_url` 은 거부.
- 새 커맨드를 추가하면 `REMOTE_ALLOWED_COMMANDS` 또는 `REMOTE_DENIED_COMMANDS` 중 하나에 반드시 등재해야 한다 — `dispatch.rs:557-560` 의 완전분할 테스트(`허용_테이블과_거부_테이블은_전체_커맨드를_교집합_없이_정확히_분할한다`)가 미등재를 컴파일/테스트 단계에서 잡는다.
- `docs/features/remote-control.md:68-69` — **LAN 평문 HTTP 는 secure context 가 아니다** → 원격 미러에서 `navigator.clipboard` 자체가 없을 수 있다(복사·붙여넣기 항목이 조용히 죽는다). 기존 `pane-tab-bar.tsx:185-186` 등도 같은 미가드 상태다(선행 갭).
- cwd 상속 함정: `src-tauri/src/domain/terminal/commands.rs:511-523` `pty_default_options` 는 넘긴 cwd 를 `ensure_within_root` 로 검증한다. `terminal-session.tsx:180` 의 `effectiveCwd` 는 OSC7 라이브 cwd 라 `cd /tmp` 이후 **루트 밖일 수 있고, 그대로 넘기면 스폰이 거부된다.**

## 11. 배치 4 사전 확정 전제 (중복 방지)

`docs/acknowledge/2026-09-04-usability-batch4-user-decisions.md:39-40` — "**터미널 우클릭(6): 분할은 VS Code 처럼 '새 터미널을 그 방향에 생성', 불가능한 방향(최소 크기 등)은 비활성 표시. 복사·붙여넣기·모두 선택·지우기·새 터미널·종료 동반**" 이 이미 사용자 고지·무이의로 확정돼 있다.
같은 문서 :28-30 의 결정 3(주제 7, 별도 wf)이 **탭 바 여백 메뉴에 "분할"을 넣기로** 확정했으므로 항목 중복 조율이 필요하다.
`docs/backlog.md` 에 터미널 컨텍스트 메뉴 항목은 없고, `docs/features/terminal.md` §11 2차 목록의 "분할 내 터미널 다중화"가 인접 항목이다.

## options
## 안 A (추천) — "새 터미널을 그 방향에 생성", 신규 Rust 커맨드 1개

**의미**: 우클릭 → 분할 ▸ 위/아래/왼쪽/오른쪽 = 그 방향에 **새 pane 을 만들고 거기서 새 셸을 띄운다**(VS Code `workbench.action.terminal.split` 파리티). 현재 터미널은 제자리·같은 세션 유지.

**데이터 흐름**
```
TerminalContextMenu(features)  ──onSplitNewTerminal(edge)──▶  TerminalPane(widget)
   ▶ TerminalSession(widget): useOpenTabInSplit({ projectId, paneId, edge, kind: Terminal{cwd} })
      ▶ layout.ipc → commands.layoutOpenTabInSplit
         ▶ Rust layout_open_tab_in_split → service::open_tab_in_split
            (begin_mutation 1회 · Tab 생성 → 새 Leaf → split_with_leaf → finish_mutation)
      ▶ layout:changed 1회 → LAYOUT.DETAIL 무효화 → PaneNodeView 재렌더
         ▶ 새 Leaf 에 TerminalSession 최초 마운트 → onReady → pty_spawn (스폰 1회)
```

**Rust 변경(최소 diff)**: `service.rs:824-875` 의 `split` 에서 "새 leaf 를 target 위치에 끼우는" 부분(`is_root_target` 분기 + `insert_split_at` + `set_tree_focused_pane`/`normalize`)을 `fn insert_new_leaf(layout, target_tree, target_pane, dir, new_leaf)` 로 추출하고, 기존 `split`(탭 이동)과 신규 `open_tab_in_split`(탭 신규 생성)이 그것을 공유한다.

- 장점: 스폰 1회·`layout:changed` 1회·기존 터미널 unmount 0 — §5 의 이중 스폰/ring buffer replay 가 **구조적으로 발생 불가**. 사용자 확정 전제와 일치. 원격에서도 그대로 동작.
- 단점: 커맨드 +1(177→178), specta 바인딩 재생성, `REMOTE_ALLOWED_COMMANDS` 등재, `IMPLEMENTED_JSON_COMMANDS`·`dispatch` match arm·완전분할 테스트 갱신, `ipc-contract.md` 갱신.

## 안 B — 기존 `layout_split` 재사용, "현재 터미널을 그 방향으로 이동"

**의미**: 분할 ▸ 방향 = 지금 보고 있는 터미널 탭을 그 방향의 새 pane 으로 옮긴다(원 pane 에는 나머지 탭이 남는다).

**데이터 흐름**: `TerminalContextMenu → TerminalSession → useSplitPane({ paneId, edge, tabId })` — `pane-tab-bar.tsx:184` 와 완전히 동일한 호출.

- 장점: **신규 IPC 0, Rust 변경 0**. 구현량 최소. pty 세션은 `Tab` 째 이동하므로 무손실.
- 단점: ① `tab-context-menu.tsx:150-164`(탭 우클릭)·`⌘\`·주제 7 의 탭바 여백 메뉴와 **기능이 3중 중복**된다. ② "분할"을 눌렀는데 터미널이 하나뿐인 사용자에게는 원 pane 이 빈 placeholder 가 되어 "화면만 반쪽 남았다"로 읽힌다. ③ 사용자가 이미 고지받은 전제(§11)와 다르다. ④ 탭이 옮겨지며 컴포넌트가 재마운트되어 ring buffer replay 1회가 발생한다.

## 안 C — 하이브리드: 기본 A + 하위 항목으로 B 병기

분할 서브메뉴를 두 그룹으로 나눈다.
```
분할 ▸  새 터미널 — 위 / 아래 / 왼쪽 / 오른쪽      (안 A)
        ─────────
        이 터미널 옮기기 — 위 / 아래 / 왼쪽 / 오른쪽 (안 B)
```
- 장점: 두 의미를 모두 노출, 학습 비용 없이 선택 가능.
- 단점: 항목 8개짜리 서브메뉴 — 우클릭 메뉴로는 과하고, 하단 4개는 탭 우클릭과 정확히 중복이다. 비활성 판정도 8칸으로 늘어난다.

---

## 공통 설계 — 비활성 조건 판정 (순수 함수)

`src/widgets/terminal-pane/terminal-split-availability.ts` (형제 선례: 같은 폴더의 `terminal-flow-control.ts`·`pending-terminal-input.ts` 가 React 무관 순수 모듈 + `bun:test`)

```ts
export type SplitEdge = Extract<DropEdge, 'left' | 'right' | 'top' | 'bottom'>

type PaneSplitMetrics = {
    paneWidthPx: number
    paneHeightPx: number
    minPaneSizePx: number      // MIN_PANEL_SIZE_PX
    resizerThicknessPx: number // settings.resizerThickness ?? DEFAULT_RESIZER_THICKNESS
}

export const resolveSplitAvailability = (metrics: PaneSplitMetrics): Record<SplitEdge, boolean> => {
    const required = metrics.minPaneSizePx * 2 + metrics.resizerThicknessPx
    const horizontal = metrics.paneWidthPx >= required
    const vertical = metrics.paneHeightPx >= required
    return { left: horizontal, right: horizontal, top: vertical, bottom: vertical }
}
```
- `bun:test` 로 전부 검증 가능(DOM 무관). 경계값(239/240/241px), 구분자 두께 0~8, 정사각형/세로긴/가로긴 pane, 0 크기(측정 실패) 케이스.
- 선택적 2단계(질문 3): 결과 pane 이 터미널로 쓸 수 있는지를 셀 단위로도 검사 — `term.cols/2 >= MIN_TERMINAL_COLS(20)`, `(term.rows - tabBarRows)/2 >= MIN_TERMINAL_ROWS(5)`.

**측정값 공급 방식(택1)**
1. **(추천)** `PaneNodeView` 의 leaf 루트 div(`pane-node-view.tsx:122`)에 `ref` 를 달고 `paneElementRef` 를 `TerminalSession → TerminalPane` 으로 내린다. 메뉴 `onOpenChange(true)` 시점에 `getBoundingClientRect()` 1회만 읽는다 — 리렌더 0, ResizeObserver 추가 0.
2. `PaneNodeView` 가 ResizeObserver 로 재고 `paneWidth/paneHeight` 를 props 로 내린다 — 리사이즈마다 리렌더가 발생해 비추천.
3. `data-pane-id` 속성 + `container.closest('[data-pane-id]')` — 위젯이 상위 pane DOM 을 뒤지는 형태라 결합도가 나쁘다.

## 공통 설계 — 상태 소유권 (FSD, `architecture.md` §5.1 판정 기준 = query/mutation/IPC 소유 여부)

| 파일 | 레이어 | 책임 |
|---|---|---|
| `features/terminal/terminal-context-menu.tsx` | features | props(boolean·라벨)+콜백만 받는 순수 뷰. `ContextMenu`/`ContextMenuSub` 마크업, `disabled` 표시. IPC·query 0 |
| `widgets/terminal-pane/terminal-split-availability.ts` | widgets(형제 선례) 또는 shared/lib | 순수 판정 함수 + 테스트 |
| `widgets/terminal-pane/terminal-pane.tsx` | widgets | `attachRef` 소유자 → 메뉴 열림 시 `hasSelection` 스냅샷(`useState`), pane rect 측정, 닫힘 시 `term.focus()` 복원. xterm 액션(copy/paste/selectAll/clear) 실행 |
| `widgets/terminal-pane/terminal-session.tsx` | widgets | 레이아웃 mutation 소유(`useOpenTabInSplit`/`useSplitPane`/`useOpenTab`/`useCloseTab`), cwd 결정, 토스트 |
| `widgets/editor-area/pane-node-view.tsx` | widgets | `paneId`·`paneElementRef` 를 `TerminalSession` 에 전달(현재 `autoFocus={node.id === focusedPaneId}` 와 같은 자리) |

`TerminalAttachHandle`(`terminal-view.tsx:55-59`) 확장:
```ts
export type TerminalAttachHandle = {
    write: (data: Uint8Array) => void
    jumpToPreviousCommand: () => void
    jumpToNextCommand: () => void
    focus: () => void
    hasSelection: () => boolean
    getSelection: () => string
    selectAll: () => void
    clear: () => void
    paste: (text: string) => void
}
```

## 공통 설계 — 메뉴 항목 (§11 확정 전제 반영)

```
복사                      disabled = !hasSelection
붙여넣기                  disabled = 클립보드 read 불가(원격 비보안 컨텍스트)
모두 선택
──────────
지우기 (Clear)
──────────
터미널 분할 ▸  위 / 아래 / 왼쪽 / 오른쪽     각 항목 disabled = !availability[edge]
새 터미널                 같은 pane 에 터미널 탭 추가
──────────
터미널 종료               layout_close_tab (pty kill 포함)
```
- 라벨 재사용: `editorArea.splitTop/Bottom/Left/Right`, `tab.split`, `tab.newTerminal`. 신규 키는 `terminal.copy` / `terminal.paste` / `terminal.selectAll` / `terminal.clear` / `terminal.kill` 5개(3언어 + `MESSAGE_NAMESPACES`).
- 방향은 **숨기지 않고 `disabled`** 로 둔다 — `tabs.md` §3.1 의 "조건 불일치 항목은 숨긴다" 관례와 반대지만, 여기서는 "이 방향이 원래 존재하는데 지금은 좁아서 안 된다"를 알려야 하므로 사용자 확정 전제(§11 "불가능한 방향은 비활성 표시")를 따른다.

## 공통 설계 — 우클릭 이벤트 배선

1. `TerminalPane` 이 `<ContextMenuTrigger asChild><div className='h-full w-full'>{<TerminalView/>}</div></ContextMenuTrigger>` 로 감싼다(TerminalView 는 ref 를 forward 하지 않으므로 래퍼 div 필요).
2. Radix Trigger 가 `preventDefault()` 하므로 **보조 창에서도 네이티브 메뉴가 억제된다**(§3 의 기존 구멍이 부수적으로 닫힌다).
3. `ContextMenuContent onCloseAutoFocus={(e) => { e.preventDefault(); attachRef.current?.focus() }}` — 메뉴 닫힘 후 포커스를 xterm 으로 되돌린다(`@radix-ui/react-menu/dist/index.d.ts:77`).
4. xterm 의 `rightClickHandler` 는 우리보다 먼저 돌아 textarea 를 옮겨 놓는다 — 메뉴 닫을 때 `term.focus()` 로 되돌리고, `rightClickSelectsWord` 정책은 질문 2 로 확정한다.

## 원격 미러에서의 동작

- 분할·새 터미널·종료: `layout_*`/`pty_*` 가 전부 remote allow 이므로 **데스크톱과 동일하게 동작**한다.
- 복사·붙여넣기: `navigator.clipboard` 가 비보안 컨텍스트에서 부재할 수 있다 → `entities/system/external-url.ts` 가 쓰는 `isRemoteMirrorRuntime`(`shared/lib/remote/runtime-environment.ts:23`) 과 같은 패턴으로 **순수 판정부 + 주입**으로 갈라, 불가하면 항목을 `disabled` 처리한다(조용한 실패 금지).
- 비활성 판정은 pane rect 기반이라 원격 브라우저 창 크기에 자연히 맞춰진다.

## recommendation
**안 A 채택 — "분할 = 그 방향에 새 터미널 생성", 신규 Rust 커맨드 `layout_open_tab_in_split` 1개 신설.**

근거 4가지:

1. **사용자 확정 전제와 일치**한다 — `docs/acknowledge/2026-09-04-usability-batch4-user-decisions.md:39-40` 이 "VS Code 처럼 새 터미널을 그 방향에 생성 / 불가능한 방향은 비활성 표시"를 이미 고지·무이의로 확정했다.
2. **안 B(이동)는 신규 기능이 아니다** — 터미널 탭 우클릭(`tab-context-menu.tsx:150-164`)·`⌘\`(`keymap.ts:75` + `editor-area.tsx:147-152`)로 이미 4방향 이동이 되고, 주제 7 이 탭바 여백 메뉴에도 분할을 넣기로 확정돼 있어 터미널 표면에까지 놓으면 4중 중복이 된다.
3. **2회 mutation 조합은 금지 수준의 결함을 재생산한다** — `open_tab` 이 새 탭을 즉시 활성화(`service.rs:441,457`)해 기존 터미널을 unmount 시키고, 새 탭은 마운트 즉시 스폰을 시작(`terminal-session.tsx:139-148`)했다가 split 으로 재마운트되어 두 번째 셸을 낳는다(감사 §4-B A6/C14 와 동일 패턴). 단일 커맨드는 이 창을 **구조적으로 없앤다**.
4. 비용이 작다 — `service.rs:824-875` 의 leaf 삽입부만 추출해 공유하면 되고, 커맨드 +1 은 원격 허용 목록·완전분할 테스트가 미등재를 자동으로 잡아 준다.

**부가 결정**
- 안 B 의 "이 터미널 옮기기"는 터미널 표면 메뉴에 **넣지 않는다**(중복 회피). 필요하면 백로그.
- 비활성 판정은 `resolveSplitAvailability({ paneWidthPx, paneHeightPx, minPaneSizePx, resizerThicknessPx })` 순수 함수 1개로 두고, pane rect 는 `PaneNodeView` 가 내려 준 `paneElementRef` 를 **메뉴 열림 시점에 1회만** 읽는다(리렌더·옵저버 0).
- 분할 4방향은 `ContextMenuSub` 서브메뉴로 묶어 `tab-context-menu.tsx` 와 시각·라벨을 통일하고 `editorArea.split*` 키를 재사용한다.
- `rightClickSelectsWord: false` 를 **명시**할 것을 권장한다(질문 2) — 현재 macOS 기본 true 라 우클릭이 선택을 바꿔 "복사" 항목의 대상이 예측 불가해진다.
- 메뉴에 검색·폰트 크기·이름 바꾸기는 넣지 않는다(각각 미구현/이미 상태바·설정에 존재/커맨드 부재).

## filesToTouch
- /Users/hyunseokbyun/development/TAIDE/src/features/terminal/terminal-context-menu.tsx
- /Users/hyunseokbyun/development/TAIDE/src/features/terminal/terminal-view.tsx
- /Users/hyunseokbyun/development/TAIDE/src/widgets/terminal-pane/terminal-pane.tsx
- /Users/hyunseokbyun/development/TAIDE/src/widgets/terminal-pane/terminal-session.tsx
- /Users/hyunseokbyun/development/TAIDE/src/widgets/terminal-pane/terminal-split-availability.ts
- /Users/hyunseokbyun/development/TAIDE/src/widgets/terminal-pane/terminal-split-availability.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/widgets/editor-area/pane-node-view.tsx
- /Users/hyunseokbyun/development/TAIDE/src/entities/layout/layout.ipc.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/layout/layout.query.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/api/bindings.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/constants/terminal.ts
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/layout/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/layout/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/lib.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/remote/dispatch.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/locale/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/en.json
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/ko.json
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/ja.json
- /Users/hyunseokbyun/development/TAIDE/docs/features/terminal.md
- /Users/hyunseokbyun/development/TAIDE/docs/features/tabs.md
- /Users/hyunseokbyun/development/TAIDE/docs/ipc-contract.md
- /Users/hyunseokbyun/development/TAIDE/docs/PROCESS.md

## risks
- **동시 편집 충돌**: `pane-node-view.tsx`·`terminal-pane.tsx`·`terminal-session.tsx`·`terminal-view.tsx`·`domain/layout/service.rs`·`lib.rs` 가 전부 다른 워크플로의 편집 대상이다. 특히 `pane-node-view.tsx` 에 `paneId`/`paneElementRef` prop 을 추가하는 변경과 `service.rs:824-875` 의 leaf 삽입부 추출 리팩토링은 충돌 확률이 높다. 착수 시점에 실제 파일을 다시 읽고 시작해야 한다.
- **2회 mutation 방식을 택하면 고아 셸이 재발한다**: `open_tab` → `split` 순서는 (a) 기존 터미널 unmount + ring buffer 전량 replay, (b) 새 탭의 스폰이 재마운트로 2회 실행돼 참조 없는 셸이 앱 종료까지 생존, 두 결함을 동시에 만든다(감사 §4-B A6/C14 와 동일 패턴). 안 A 의 단일 커맨드가 아니면 반드시 이 두 경로에 대한 방어가 필요하다.
- **`open_tab` 의 kind 중복 제거**(`service.rs:436-444`): `TabKind::Terminal { session_id: "", cwd }` 두 개는 같은 kind 로 취급된다. "새 터미널"을 연타하면 두 번째 호출이 새 탭 대신 기존(아직 세션 id 미기록) 탭을 활성화하고 끝난다.
- **xterm 의 textarea 잔류**: 우클릭 후 커서가 움직이기 전까지 20×20 · `zIndex 1000` textarea 가 클릭 지점에 남는다(`Clipboard.ts:63-78`, `_syncTextArea` 는 `onCursorMove` 에서만 호출). 같은 지점을 연속 우클릭하면 `app-shell.tsx:102` 의 `closest('textarea')` 예외에 걸려 네이티브 메뉴가 커스텀 메뉴와 함께 뜬다.
- **포커스 유실**: Radix 는 닫힐 때 Trigger 로 포커스를 되돌리므로, `onCloseAutoFocus` 에서 `preventDefault()` + `term.focus()` 를 하지 않으면 메뉴를 닫은 뒤 키 입력이 pty 로 가지 않는다.
- **마우스 리포팅 TUI 와의 충돌**: vim·tmux 등이 마우스 트래킹을 켜면 `CoreBrowserTerminal.ts:779-796` 가 우클릭 mousedown 을 pty 로 보낸다. TUI 가 우클릭을 소비하는 동시에 우리 메뉴도 열려 이중 반응이 된다.
- **`rightClickSelectsWord` 기본 true(macOS)**: 지금도 우클릭이 커서 아래 단어를 선택하고 있다(`OptionsService.ts:50`). 그대로 두면 "복사" 항목의 대상이 사용자가 만든 선택이 아니라 방금 자동 선택된 단어가 되는 경우가 생긴다.
- **원격 미러의 클립보드**: LAN 평문 HTTP 는 secure context 가 아니라(`docs/features/remote-control.md:68-69`) `navigator.clipboard` 가 부재할 수 있다. 복사/붙여넣기 항목이 조용히 실패한다(기존 `pane-tab-bar.tsx:185-186` 도 같은 미가드 상태 — 선행 갭).
- **cwd 상속이 거부될 수 있다**: `pty_default_options`(`domain/terminal/commands.rs:511-523`)가 `ensure_within_root` 로 cwd 를 검증한다. OSC7 로 추적한 라이브 cwd(`terminal-session.tsx:51,180`)가 `cd /tmp` 등으로 루트 밖에 있으면 새 터미널 스폰이 Forbidden 으로 실패한다 — 루트 밖이면 null 로 폴백해야 한다.
- **로케일 3중 계약**: 신규 키는 `MESSAGE_NAMESPACES`(`domain/locale/service.rs:209-219`) + en/ko/ja JSON 을 **동시에** 갱신해야 한다. 한쪽만 넣으면 `service.rs:1300` 테스트가 실패한다. PROCESS.md 기준선의 "로케일 792키×3" 수치도 갱신 대상이다.
- **커맨드 수·원격 분할 테스트**: 커맨드 신설 시 `IMPLEMENTED_JSON_COMMANDS`·`REMOTE_ALLOWED_COMMANDS`·`dispatch` match arm·`ipc-contract.md`·PROCESS.md 기준선(커맨드 177) 을 모두 갱신해야 하며, 누락 시 `dispatch.rs` 완전분할 테스트가 실패한다.
- **주제 7(탭바 여백 메뉴)과 항목 중복**: 결정 3 이 여백 메뉴에도 "분할"·"새 터미널"을 넣기로 확정했다. 두 워크플로가 각자 다른 의미(이동 vs 신규 생성)의 "분할"을 붙이면 사용자 관점에서 일관성이 깨진다 — 계약 단계에서 의미를 통일해야 한다.
- **터미널 종료/실패 화면에는 메뉴가 안 붙는다**: `terminal-session.tsx:261-274` 가 exited/failure 시 `TerminalPane` 자체를 렌더하지 않으므로, 그 상태에서 우클릭하면 메뉴가 없다(`app-shell` 이 네이티브 메뉴만 막는다).
- **`docs/features/terminal.md` 의 현행 불일치**: §1·§8 이 clipboard(OSC52) 애드온과 검색 바를, §7 이 `⌘+/-` 커스텀 키 핸들러를 기술하지만 실제로는 애드온 미설치(`package.json:41-46`)·SearchAddon 미사용(`terminal-view.tsx:209`)·Shift+Enter 만 처리(`terminal-view.tsx:228-233`)다. 메뉴 항목을 이 문서 기준으로 설계하면 없는 기능을 노출하게 된다.

## questionsForUser
- **분할 의미 재확인** — 배치 4 결정 문서(:39-40)가 이미 "새 터미널을 그 방향에 생성"으로 확정했지만, 이는 **신규 Rust 커맨드 1개(커맨드 177→178)** 를 수반합니다. **A안(추천: 확정 전제 준수·이중 스폰 구조적 차단)** / B안(기존 `layout_split` 재사용 = 현재 터미널 이동, 신규 IPC 0이지만 탭 우클릭·⌘\ 와 3중 중복) / C안(둘 다 병기, 서브메뉴 8항목)
- **`rightClickSelectsWord`** — 현재 macOS 에서 기본 true 라 우클릭이 커서 아래 단어를 자동 선택합니다. **A안(추천: `false` 로 명시 — 메뉴의 "복사" 대상이 사용자가 만든 선택으로 고정, 전 OS 동일)** / B안(macOS 기본 유지 — VS Code macOS 감각 보존, 대신 복사 대상이 우클릭 위치에 따라 바뀜)
- **비활성 판정 기준** — **A안(추천: 레이아웃 최소치만 — pane 폭/높이 ≥ `MIN_PANEL_SIZE_PX*2 + 구분자`, 즉 약 241px)** / B안(A + 터미널 셀 최소치 20열×5행 — 더 보수적이지만 임계값 2개가 새 매직넘버가 됨) / C안(비활성 없이 항상 4방향 활성 — 확정 전제와 배치)
- **새 터미널의 cwd** — **A안(추천: 현재 터미널의 라이브 cwd 상속, 단 프로젝트 루트 밖이면 루트로 폴백 — `ensure_within_root` 거부 회피)** / B안(항상 프로젝트 루트) / C안(탭에 기록된 `TabKind::Terminal.cwd` 만 상속)
- **원격 미러의 복사·붙여넣기** — 비보안 컨텍스트에서 `navigator.clipboard` 가 없을 수 있습니다. **A안(추천: 사용 불가 시 항목 `disabled` + 툴팁)** / B안(항목 숨김) / C안(그대로 두고 실패 시 토스트)
- **"터미널 종료" 확인 다이얼로그** — 현재 탭 닫기는 포그라운드 프로세스 유무와 무관하게 즉시 kill 입니다(`terminal.md` §10 이 미구현으로 기록). **A안(추천: 이번 범위에서는 확인 없이 즉시 종료 — 기존 탭 닫기와 동일 동작 유지)** / B안(이 메뉴에서 확인 다이얼로그를 처음 도입 — `foreground_pid` 는 Rust 에 이미 존재하나 IPC·i18n·다이얼로그 신설 필요)
- **마우스 리포팅 TUI(vim·tmux) 안에서의 우클릭** — **A안(추천: 항상 메뉴를 연다 — 예측 가능, VS Code 와 동일)** / B안(마우스 모드 중에는 메뉴를 열지 않고 pty 로만 전달) / C안(⌥+우클릭 등 수식어를 요구)
- **"새 터미널" 항목 포함 여부** — 이미 탭바 + 메뉴 + 팔레트 + `⌃⇧\`` 4경로가 있고 주제 7 의 탭바 여백 메뉴에도 들어갑니다. **A안(추천: 포함 — 확정 전제 문언에 있고 VS Code 파리티)** / B안(제외 — 중복 최소화)
- **메뉴에서 제외 확정** — 검색(SearchAddon 로드만·UI 0)·이름 바꾸기(탭 제목 변경 커맨드 부재)·폰트 크기(상태바·설정에 이미 존재)는 이번 범위에서 빼고 backlog 로 넘기는 것을 추천합니다. 이의 있으신지요?
- **순수 판정 함수 배치** — `architecture.md` §5.1 은 "React 무관 순수 함수는 `shared/lib`" 라고 규정하지만, 형제 파일 `widgets/terminal-pane/terminal-flow-control.ts`·`pending-terminal-input.ts` 는 같은 성격으로 widgets 에 남아 있습니다. **A안(추천: 형제 선례 따라 `widgets/terminal-pane/terminal-split-availability.ts`)** / B안(§5.1 문언 따라 `shared/lib/terminal-split-availability.ts`)

## newDependencies



# 주제 7 — 탭 바 우클릭 컨텍스트 메뉴


## findings
## 1. 탭 바 현행 구조 — `src/widgets/editor-area/pane-tab-bar.tsx` (작업트리 미변경, HEAD 기준 라인 정확)

`return` 은 `:234-264`. 3층이다.
- 스크롤 컨테이너 `:236-254` — `role='tablist'`, `onMouseDown={() => focusPane(paneId)}`(`:239`, 우클릭 mousedown 도 여기서 pane 포커스가 잡힌다), `onWheel` 가로 스크롤(`:240`), `overflow-x-auto`.
  - pinned `SortableContext` `:245-249` / unpinned `SortableContext` `:250-252`
  - **여백 filler** `:253` — `<div ref={setContainerRef} onDoubleClick={handleNewUntitledFile} className='min-w-8 flex-1' />`
- 우측 액션 영역 `:255-261` — 스크롤 컨테이너 **밖의 별도 div**. `TabBarAddMenu` 를 담는다(`:260`).
- `OverlayScrollbar` `:262`.

핵심: **filler 는 탭들의 부모가 아니라 형제**다. 여기에만 Trigger 를 달면 탭 우클릭과 DOM 상 겹치지 않는다. filler 는 동시에 dnd 드롭 타깃이다(`:92-95` `useDroppable({id:'pane-container:<paneId>'})`, 드롭 처리 `editor-area.tsx:415-419`).

탭 0 인 leaf 도 탭 바를 그린다 — `pane-node-view.tsx:123-125` 는 `zen` 만 검사하므로 빈 pane 에서는 여백이 바 전체 폭이다.

## 2. "새 파일·새 터미널"은 이미 구현돼 있다 — 없는 것은 우클릭 진입점뿐

`src/features/tab/tab-bar-add-menu.tsx:15-39` 의 `+` 드롭다운이 `tab.newUntitledFile`·`tab.newTerminal` 2항목을 이미 제공한다. 핸들러는 `pane-tab-bar.tsx:123-129`:
- `handleNewUntitledFile` → `openUntitledTab({ projectId, target: paneId })` — **pane 타깃**
- `handleNewTerminal` → `openTab({ kind:{kind:'terminal',sessionId:''}, title:t('terminal.title'), target: paneId, preview:false })`

여백 **더블클릭**도 새 untitled 다(`:253`). 즉 사용자가 요구한 두 기능은 존재하고, 결손은 "여백 우클릭" 제스처다.

## 3. 기존 탭 메뉴 전량 — `src/features/tab/tab-context-menu.tsx:96-180`

close / closeOthers / closeToRight / closeSaved / closeAll → pin·unpin → keepOpen(preview 탭만) → copyPath·copyRelativePath(file 만) → revealInFinder·revealInExplorerView·openChanges·fileHistory(file 만) → `reopenEditorWith` 서브(preview 가능 확장자만) → split 서브 4방향 → moveToNewWindow·moveToMainWindow·moveToWindow N.

정본은 `docs/features/tabs.md` §3.1. 그 절은 **의도적 미구현 목록**도 못박고 있다 — Copy Remote File URL(From…)·Open on Remote (Web)·Add File to Agent Thread·Find File References·Split in Group. `Copy into New Window` 도 `docs/backlog.md` 에 미구현 잔여 항목으로 명시돼 있다. 이 6개는 재제안 시 별도 근거가 필요하다.

## 4. Radix 이벤트 전파 — "탭 바 전체 감싸기"는 이중 메뉴가 된다 (라이브러리 소스 확인)

`radix-ui` 1.6.7 → `@radix-ui/react-context-menu` 2.3.7.
`node_modules/@radix-ui/react-context-menu/dist/index.mjs:92-96`:
```js
onContextMenu: disabled ? props.onContextMenu : composeEventHandlers(props.onContextMenu, (event) => {
  clearLongPress(); handleOpen(event); event.preventDefault();
}),
```
`preventDefault()` 만 하고 **`stopPropagation()` 은 하지 않는다.** React 합성 이벤트는 트리 위로 버블하므로, 탭이 자기 Trigger(`sortable-tab.tsx:74`→`tab-context-menu.tsx:94`)를 가진 채로 탭 바 전체를 감싸는 Trigger 를 추가하면 **탭 위 우클릭이 두 메뉴를 동시에 연다.**

부수 확인 2건:
- Trigger 가 함께 렌더하는 `MenuPrimitive.Anchor` 는 `virtualRef` 를 받아 **DOM 노드를 만들지 않는다** — `node_modules/@radix-ui/react-popper/dist/index.mjs:78` `return virtualRef ? null : …`. flex 레이아웃에 영향 0.
- Trigger 의 `onPointerDown`/`Move`/`Up` 은 `whenTouchOrPen` 래핑이라 마우스에서는 no-op → dnd-kit 포인터 센서·좌클릭과 충돌 없음.

## 5. 네이티브 메뉴는 전역 차단 — 신설은 순증

`src/widgets/app-shell/app-shell.tsx:101-104`(`handleNativeContextMenu`) + `:128` 이 `document` 의 contextmenu 를 input/textarea/contenteditable 밖에서 전부 `preventDefault` 한다. 지금 탭 바 여백 우클릭은 **아무 일도 일어나지 않는다** → 회귀 위험 없이 새 동작을 얹는 자리다.

## 6. "빈 영역 메뉴" 선례는 앱 안에 이미 있다 (탐색기)

`src/features/explorer/file-tree.tsx:248` 이 트리 전체를 **하나의** `FileTreeContextMenu` 로 감싸고, `handleContainerContextMenu`(`:205-226`)가 Y 좌표 ÷ 행 높이로 행을 히트테스트해 `contextRow` 를 정한다. `row === null`(빈 영역)이면 `file-tree-context-menu.tsx:60-101` 이 "새 파일/새 폴더"만 남긴다. 단 이 패턴을 탭 바에 옮기려면 탭의 개별 Trigger 를 걷어내야 한다(§4).

## 7. 각 후보 항목의 실행 경로 — 가능/불가

- **새 파일(untitled)**: `useOpenUntitledTab`(`entities/layout/layout.query.ts:190`) → `layout.ipc.ts:35-36` → `layout_open_untitled(projectId, target)`(`src-tauri/src/domain/layout/commands.rs:241~`). Rust 가 `target.unwrap_or(layout.focused_pane)` 에 열고 `next_untitled_index`(`service.rs:462-483`)로 번호를 재사용한다. **pane 지정 가능.**
- **새 터미널**: `useOpenTab` + `{kind:'terminal', sessionId:''}`. 팔레트의 `terminal.new` 는 `target: null`(포커스 pane)로 연다(`command-palette.tsx:150-157`, `shared/lib/command-catalog.ts:33-39`).
- **닫은 탭 다시 열기**: `useReopenClosedTab`(`layout.query.ts:188`) → `layout_reopen_closed(projectId)`(`layout.ipc.ts:33`). 커맨드 `tab.reopenClosed`(`command-catalog.ts:40-46`), 키맵 `reopen-closed-tab` = ⇧⌘T(`shared/lib/keymap/keymap.ts:78`). 활성 판정은 `layout.closedTabs`(`shared/api/bindings.ts:1013-1017`, `:1517`)로 가능. **단 untitled 탭은 closed 스택에 쌓이지 않는다**(`src-tauri/src/domain/layout/service.rs:462-463` 주석).
- **분할**: `splitPane`(`layout.ipc.ts:17-18`) → `layout_split(paneId, edge, **tabId**)`(`commands.rs:143-154`). `service::split`(`service.rs:824-834`)이 그 `tabId` 를 **추출해** 새 leaf 로 옮기므로 **탭이 0인 pane 은 분할 자체가 불가**(NotFound). 앱 커맨드 `editor.split` 도 `leaf.active` 없으면 no-op(`editor-area.tsx:147-152`, edge 는 항상 `'right'`).
- **닫기 계열**: `handleCloseOthers`·`handleCloseToRight`·`handleCloseSaved`·`handleCloseAll` 이 `pane-tab-bar.tsx:131-159` 에 **이미 존재**하고 pinned 를 건너뛴다. 여백 메뉴는 `handleCloseSaved`/`handleCloseAll` 을 인자 없이 그대로 재사용할 수 있다(다른 둘은 기준 탭이 필요해 여백 메뉴에 부적합).
- **실제 파일 생성(탐색기 draft)**: `startDraft`→`targetDirFor`(`widgets/explorer/explorer-container.tsx:76-79`) = 선택 행의 디렉토리, 선택 없으면 **프로젝트 루트** → 인라인 입력 → `createEntry`→`refreshTreeDir`→`revealTreeNode`→탭 열기(`use-explorer-entry-crud.ts:64-70, 94-110`). 탭 바에는 "선택 행" 개념이 없으므로 이 경로를 쓰면 항상 프로젝트 루트가 된다.
- **Welcome**: `TabKind::Welcome` 은 Rust `default_layout`(`service.rs:110-135`)만 만든다. 프론트에 `{kind:'welcome'}` 을 여는 경로가 **0건**(grep). 렌더는 이미 있고(`pane-node-view.tsx:145-149`), `layout_open_tab` 은 file 탭만 존재 검증하며(`commands.rs:55-68`) `open_tab` 이 **동일 kind 를 dedupe**(`service.rs:436-444`)하므로 FE 1줄로 열 수 있고 중복 탭도 안 생긴다. 다만 이건 `docs/PROCESS.md:58-59` 의 **배치 4 주제 ②(Welcome 커맨드 + 탭 0 자동 표시)** 범위다.

## 8. 로케일 키 추가 절차 (4곳 동기)

`src-tauri/resources/locales/{en,ko,ja}.json`(알파벳 정렬·1줄 1엔트리) + `MESSAGE_NAMESPACES` 의 `"tab"` 배열(`src-tauri/src/domain/locale/service.rs:248-274`). 현재 `tab.*` 23키(`ko.json:854-876`). 테스트가 3언어 키 집합 동일(`service.rs:1281`), 미등록 키 금지(`:1295-1300`), 원문 중복 키 금지(`:1343-1370`)를 강제한다. 하드코딩된 키 개수 단언은 없다.

## 9. FSD 배치 · 테스트 환경

- `docs/architecture.md:225-235`: query/mutation/IPC 직접 호출 = `widgets`, props/콜백만 = `features`. `tab-context-menu`·`sortable-tab`·`tab-bar-add-menu` 는 전부 props 전용 `features/tab`. **새 메뉴 컴포넌트도 `features/tab`.**
- `@testing-library/react` 없음(package.json devDeps) → 메뉴 컴포넌트 유닛 테스트 불가. 앱 선례는 로직을 순수 함수로 뽑는 것(`features/git/git-change-group.tsx` 의 `buildGitChangeGroupConfig`, `widgets/editor-area/focused-editor-tab.ts`). e2e(Playwright `e2e/specs/*`)에 탭 바 스펙 없음.
- `ContextMenuShortcut` 이 `shared/ui/context-menu.tsx:165` 에 정의돼 있으나 **앱 전체 미사용** — 단축키 힌트 표기는 아직 이 앱의 관례가 아니다.

## 10. 접근성

Radix ContextMenu 는 Trigger 요소에 키보드 포커스가 있어야 Shift+F10/컨텍스트 키로 열린다. filler div 는 `tabIndex` 가 없어 **키보드로는 열 수 없다**. 탐색기는 컨테이너가 `tabIndex={0}`(`file-tree.tsx:279`)이라 도달 가능하다. 여백에 tabIndex 를 주면 Tab 순회에 "빈 항목"이 끼는 트레이드오프가 생긴다. `docs/backlog.md` 의 "접근성(키보드만으로 전체 조작)" 항목과 연결된다.

## options
## A안 (추천 골격) — 여백(filler) 전용 `TabBarContextMenu` 신설

**변경 파일**: `src/features/tab/tab-bar-context-menu.tsx`(신설, props 전용) + `src/widgets/editor-area/pane-tab-bar.tsx:253` 한 줄 래핑 + 로케일 4곳.

**데이터 흐름**: `PaneTabBar`(widgets — 이미 `openUntitledTab`·`openTab`·`closeTabAsync`·`splitPane`·`useReopenClosedTab` 을 전부 보유하거나 한 줄로 추가 가능) → props 콜백 → 메뉴(features). pane 컨텍스트는 `paneId` 로 고정 — 새 탭은 **우클릭한 그 pane** 에 열린다(`+` 메뉴·더블클릭과 동일 정책, `target: paneId`).

```
<TabBarContextMenu onNewFile={…} onNewTerminal={…} …>
    <div ref={setContainerRef} onDoubleClick={handleNewUntitledFile} className='min-w-8 flex-1' />
</TabBarContextMenu>
```
`ContextMenuTrigger asChild` 로 감싸면 Slot 이 ref 를 합성하므로 dnd droppable ref 와 공존한다.

**항목 제안** (실행 경로는 findings §7):
| 항목 | 실행 | 표시 조건 |
|---|---|---|
| 새 파일 | `openUntitledTab({projectId, target: paneId})` | 항상 |
| 새 터미널 | `openTab({kind:{kind:'terminal',sessionId:''}, target: paneId})` | 항상 |
| 닫은 탭 다시 열기 | `useReopenClosedTab(projectId)` | `layout.closedTabs?.length > 0` |
| 저장된 탭 닫기 | 기존 `handleCloseSaved` | `tabs.length > 0` |
| 모든 탭 닫기 | 기존 `handleCloseAll` | `tabs.length > 0` |
| 분할 ▸ 위/아래/왼쪽/오른쪽 | `splitPane({paneId, edge, tabId: activeTabId})` | **`activeTabId != null`** (탭 0 이면 숨김 — 없으면 Rust NotFound) |

- 장점: §4 전파 충돌이 **구조적으로 없다**. diff 최소(신설 1 + 래핑 1줄). 기존 핸들러 전량 재사용 → 새 IPC·새 mutation 0. 동시 편집 중인 파일 충돌면이 `pane-tab-bar.tsx` 한 곳으로 국한.
- 단점: 탭이 꽉 차면 여백이 `min-w-8`(32px)로 줄어 표적이 작다(스크롤 우측 끝). 우측 `+` 액션 영역(`:255-261`)은 스크롤 컨테이너 밖이라 그쪽 우클릭은 여전히 무반응 → **같은 메뉴로 그 div 도 한 번 더 감싸면 해결**(래핑 2곳, 권장 보강).

## B안 — 탭 바 전체 단일 ContextMenu + 히트테스트 (탐색기 패턴 이식)

**변경 파일**: `sortable-tab.tsx`(TabContextMenu 제거), `tab-context-menu.tsx`(선택 탭 = null 허용으로 시그니처 개편), `tab-item.tsx`(`data-tab-id` 부여), `pane-tab-bar.tsx`(스크롤 컨테이너 전체를 하나의 ContextMenu 로 감싸고 `onContextMenu` 에서 `event.target.closest('[data-tab-id]')` 로 대상 판정 → `contextTabId` state).

- 장점: 여백 폭과 무관하게 **바 전체가 우클릭 표적**. 탭 메뉴와 여백 메뉴의 공통 항목(닫기 계열·분할)을 한 곳에서 관리. 탐색기(`file-tree.tsx:205-226,248`)와 동일 패턴이라 앱 내 일관성이 높다.
- 단점: 4파일 구조 변경 — **동시 편집 중인 `pane-tab-bar.tsx` 와 충돌 위험 최대**. 탭별 조건부 항목(file 여부·pinned·preview·창 이동 슬롯)을 전부 "선택된 탭" state 경유로 옮겨야 해 회귀면이 넓다(`tabs.md` §3.1 계약 전량이 회귀 대상). 드래그 중 우클릭·탭 재정렬 중 `contextTabId` 무효화 같은 새 엣지케이스가 생긴다.

## C안 — A안 + 항목 descriptor 공통화 (`+` 드롭다운과 영구 동기)

**변경 파일**: A안 + `src/features/tab/tab-bar-menu-items.ts`(순수 빌더) + `tab-bar-menu-items.test.ts` + `tab-bar-add-menu.tsx`(같은 빌더 소비).

`buildTabBarMenuItems({ hasTabs, hasActiveTab, hasClosedTabs, handlers }) => TabBarMenuItem[]` 를 순수 함수로 두고, `TabBarAddMenu`(DropdownMenu primitive)와 `TabBarContextMenu`(ContextMenu primitive)가 같은 배열을 각자 렌더한다(선례: `git-change-group.tsx` 의 `buildGitChangeGroupConfig` + `buildContextMenuEntries`).

- 장점: `+` 메뉴와 우클릭 메뉴가 영구 동기. **순수 함수라 `bun test` 로 표시 조건(탭 0 → 분할 숨김, closedTabs 0 → 재열기 숨김)을 검증 가능** — testing-library 부재(§9) 제약의 유일한 해법. Welcome 등 후속 항목이 1곳 추가로 끝난다.
- 단점: 초기 diff 가 A보다 3파일 크다. `+` 버튼에 "모든 탭 닫기" 같은 파괴적 항목까지 노출할지 제품 판단이 필요(현행 2항목 유지하려면 빌더에 필터 인자를 둬야 함).

## (b축) 기존 탭 메뉴 확장 후보 — 중복 없음 확인분만

| 후보 | 실행 경로 | 판단 |
|---|---|---|
| **이름 바꾸기** | 탭에는 rename 경로가 전무. `explorer-reveal-bridge.ts`(4줄, `createFireAndForgetBridge`) 선례대로 `explorer-rename-bridge` 를 신설해 `explorer-panel.tsx:116-123` 처럼 구독 → 탐색기 인라인 rename(`use-explorer-entry-crud.ts:115-118 startRename`) 재사용. file 탭 한정. **가장 저렴하고 가치 높음(추천)** |
| 다른 이름으로 저장 | untitled 은 `untitled-pane.tsx handleSaveAs` 로 이미 가능하나 **탭 메뉴에는 없다**. 키 `tab.saveAsTitle` 이미 존재 | 저비용 |
| 저장 / 되돌리기 | `editor-pane-command-bridge` 의 `save-active-tab` 은 **활성 탭 한정**이라 우클릭 대상 탭과 다를 수 있음 → 브리지 페이로드에 tabId 추가 계약 필요 | 중간 |
| 이 탭만 남기기 | `closeOthers` 와 동일 — **중복, 제외** |
| Copy into New Window | `docs/backlog.md` 미구현 명시 항목 — **제외** |
| 터미널 탭 전용(cwd 복사·세션 재시작) | 배치 4 **주제 ⑥(터미널 우클릭 메뉴)** 와 범위 중복 — **제외** |

## recommendation
**A안 골격 + C안의 순수 빌더**를 채택하고, (b축은) 기존 탭 메뉴에 "이름 바꾸기" 1건만 추가한다.

근거:
1. **전파 충돌 회피가 구조적**이어야 한다. Radix Trigger 가 `stopPropagation` 을 하지 않는 것이 라이브러리 소스로 확인됐으므로(findings §4), 탭의 형제인 filler 에만 Trigger 를 두는 A안이 "고치는" 게 아니라 "생기지 않게" 하는 유일한 안이다.
2. **동시 편집 리스크**. `pane-tab-bar.tsx`·`features/terminal` 등이 다른 워크플로에서 편집 중이라, `sortable-tab`·`tab-item` 까지 건드리는 B안은 병합 비용이 크다. A안의 접점은 `pane-tab-bar.tsx` 한 파일의 2줄(래핑)뿐이다.
3. **테스트 가능성**. testing-library 가 없어 메뉴 컴포넌트를 직접 테스트할 수 없으므로, 표시 조건(탭 0 → 분할 숨김 / closedTabs 0 → 재열기 숨김)을 순수 빌더로 뽑아 `bun test` 로 고정한다. 이게 없으면 "탭 0 pane 에서 분할 → Rust NotFound 토스트" 회귀를 잡을 수단이 없다.
4. **신규 IPC·mutation 0.** 6항목 전부 `pane-tab-bar.tsx` 가 이미 들고 있는 핸들러(`:123-159`)와 `useReopenClosedTab` 한 줄 추가로 끝난다. 신규 의존성 0.

구체 실행 순서:
1. `features/tab/tab-bar-menu-items.ts` — `buildTabBarMenuItems({hasTabs, hasActiveTab, hasClosedTabs})` 순수 빌더 + 테스트.
2. `features/tab/tab-bar-context-menu.tsx` — props 전용 ContextMenu(`file-tree-context-menu.tsx` 스타일 준수, 조건 불충족 항목은 **숨김**: `tabs.md` §3.1 "비활성 대신 숨긴다" 규칙).
3. `pane-tab-bar.tsx` — filler(`:253`)와 우측 액션 영역(`:255-261`)을 같은 메뉴로 래핑(둘 다 `asChild`), `useReopenClosedTab` 1줄 추가.
4. 로케일: 신규 키는 최소 2개(`tab.reopenClosedTab` 대신 기존 `keymap.reopenClosedTab` 재사용 가능 → 실질 신규는 0~1). 분할 4방향은 기존 `editorArea.split*` 재사용, 닫기 계열은 기존 `tab.closeSaved`/`tab.closeAll` 재사용 → **신규 로케일 키를 최소화**하는 것이 이 안의 부수 이점이다.
5. `docs/features/tabs.md` §3.1 아래에 §3.2 "탭 바 여백 context menu" 절 추가.

Welcome 항목은 **넣지 않는다** — 배치 4 주제 ②의 결정 대상이고, 확정 후 빌더에 1줄 추가하면 되도록 자리만 비워 둔다.

## filesToTouch
- /Users/hyunseokbyun/development/TAIDE/src/features/tab/tab-bar-context-menu.tsx (신설 — props 전용 ContextMenu)
- /Users/hyunseokbyun/development/TAIDE/src/features/tab/tab-bar-menu-items.ts (신설 — 순수 항목 빌더)
- /Users/hyunseokbyun/development/TAIDE/src/features/tab/tab-bar-menu-items.test.ts (신설 — 표시 조건 회귀 테스트)
- /Users/hyunseokbyun/development/TAIDE/src/widgets/editor-area/pane-tab-bar.tsx (여백 :253 · 액션영역 :255-261 래핑, useReopenClosedTab 추가)
- /Users/hyunseokbyun/development/TAIDE/src/features/tab/tab-bar-add-menu.tsx (C안 빌더 공유 시)
- /Users/hyunseokbyun/development/TAIDE/src/features/tab/tab-context-menu.tsx (b축 '이름 바꾸기' 채택 시)
- /Users/hyunseokbyun/development/TAIDE/src/features/tab/sortable-tab.tsx (b축 채택 시 prop 통과)
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/bridge/explorer-rename-bridge.ts (신설 — b축 '이름 바꾸기' 채택 시)
- /Users/hyunseokbyun/development/TAIDE/src/widgets/explorer/explorer-panel.tsx (rename 브리지 구독 — b축 채택 시)
- /Users/hyunseokbyun/development/TAIDE/src/widgets/explorer/explorer-container.tsx (startRename 노출 — b축 채택 시)
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/en.json
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/ko.json
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/ja.json
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/locale/service.rs (MESSAGE_NAMESPACES "tab" 배열 :248-274)
- /Users/hyunseokbyun/development/TAIDE/docs/features/tabs.md (§3.1 아래 여백 메뉴 절 신설)

## risks
- Radix Trigger 가 stopPropagation 을 하지 않는다(@radix-ui/react-context-menu/dist/index.mjs:92-96) — 탭 바 전체를 감싸는 순간 탭 위 우클릭이 탭 메뉴와 여백 메뉴를 동시에 연다. B안 채택 시 반드시 탭 개별 Trigger 를 제거하거나 inner 에서 stopPropagation 해야 한다.
- 탭 0 pane 에서 '분할'을 실행하면 layout_split 이 tabId 를 요구하므로(commands.rs:143-154, service.rs:824-834) NotFound 에러 토스트가 뜬다. activeTabId === null 이면 항목을 반드시 숨겨야 한다.
- filler 를 ContextMenuTrigger 로 감쌀 때 asChild 를 빼면 Primitive.span 이 삽입돼 flex-1/min-w-8 레이아웃이 깨진다. asChild 유지 + Slot 의 ref 합성(useDroppable setContainerRef 와 공존) 검증 필요.
- 탭이 넘칠 때 여백은 min-w-8(32px)로 축소되고 스크롤 우측 끝에만 존재한다 — 우측 '+' 액션 영역(pane-tab-bar.tsx:255-261, 스크롤 컨테이너 밖)까지 함께 래핑하지 않으면 실사용 표적이 매우 좁다.
- pane-tab-bar.tsx·features/terminal·entities/layout 등이 다른 워크플로에서 동시 편집 중 — 라인 번호 기반 패치는 무효화될 수 있다(현재 features/tab·pane-tab-bar.tsx 는 작업트리 미변경이나 src-tauri/domain/layout/commands.rs·locales 3종은 이미 modified).
- open_tab 이 동일 kind 를 dedupe 하므로(service.rs:436-444) sessionId 가 아직 빈 터미널 탭이 있는 상태에서 '새 터미널'을 연타하면 새 탭이 생기지 않고 기존 탭이 활성화된다. 기존 '+' 메뉴와 동일 결함이지만 진입점이 늘어 노출 빈도가 증가한다.
- untitled 탭은 closed_tabs 에 쌓이지 않는다(service.rs:462-463) — '닫은 탭 다시 열기'가 방금 닫은 Untitled-N 을 되살리지 못해 사용자 기대와 어긋날 수 있다.
- 여백 div 는 focusable 이 아니라 Shift+F10/컨텍스트 키로 메뉴를 열 수 없다(탐색기는 file-tree.tsx:279 tabIndex={0} 로 가능). tabIndex 부여는 Tab 순회에 빈 항목을 추가하는 트레이드오프.
- 로케일 키를 추가하면 en/ko/ja 3파일 + MESSAGE_NAMESPACES 배열 4곳을 동시에 갱신해야 하며, 누락 시 Rust 테스트 3종(service.rs:1281·1295·1343)이 실패한다. 카탈로그는 알파벳 정렬·1줄 1엔트리 형식을 지켜야 한다.
- docs/features/tabs.md §3.1 이 '구현하지 않았다'고 못박은 5항목과 backlog 의 Copy into New Window 를 여백 메뉴에 되살리면 확정 결정을 번복하는 셈이 된다.
- Welcome 항목을 이 주제에서 확정하면 배치 4 주제 ②(docs/PROCESS.md:58-59)와 계약이 경합한다.

## questionsForUser
- 여백 메뉴 항목 확정 — **추천안(새 파일 · 새 터미널 · 닫은 탭 다시 열기 · 저장된 탭 닫기 · 모든 탭 닫기 · 분할▸상하좌우)** / 최소안(새 파일 · 새 터미널만) / 최대안(추천안 + Welcome 열기 + 설정 열기)
- 우클릭 표적 범위 — **추천: 여백 + 우측 '+' 버튼 영역 2곳 래핑(A안)** / 여백만 / 탭 바 전체(B안 — sortable-tab·tab-item 구조 변경 동반)
- '+' 드롭다운과 항목 동기화 — **추천: 공통 순수 빌더로 완전 동기(C안, 테스트 가능)** / '+' 는 현행 2항목 유지하고 우클릭만 확장
- 기존 탭 메뉴에 추가할 항목 — **추천: '이름 바꾸기' 1건만(탐색기 인라인 rename 을 브리지로 재사용)** / + '다른 이름으로 저장' / + '저장·되돌리기' / 추가 없음
- 여백 메뉴의 '새 파일' 의미 — **추천: untitled 버퍼(현행 '+' 메뉴·더블클릭과 동일)** / 프로젝트 루트에 실제 파일 생성(탐색기 draft 흐름 재사용, 이름 입력 UI 필요)
- 'Welcome 열기' 항목을 이 주제에서 다룰지 — **추천: 배치 4 주제 ② 계약으로 미루고 빌더에 자리만 비워 둠** / 지금 함께 확정
- 메뉴에 단축키 힌트(ContextMenuShortcut, 현재 앱 전체 미사용) 표기를 도입할지 — 도입 시 기존 탭·탐색기·git 메뉴도 일관 적용 대상이 된다. **추천: 이번 범위에서는 미도입**

## newDependencies



