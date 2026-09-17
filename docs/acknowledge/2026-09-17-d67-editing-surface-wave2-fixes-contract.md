# d-67 — 편집 표면 2차 조사 확인 결함 일괄 수정 (2026-09-17)

> 사용자 지시(d-66 과 동일 포괄 지시): "버그될 만한 건 다 바꿔 확실하게 수정" + "커밋 푸시 draft 잘 생성해두고".
> 조사 정본 `docs/research/2026-09-17-editing-surface-bug-audit-wave2.md`(7관점 → 반박 검증 생존 24: major 11·minor 13, 기각 4, info 1).
> 항목 번호(#1~#24)는 그 문서 §1 을 그대로 쓴다. 증상·재현·원인 인용·fixNote 는 §2 가 정본이고, 여기서는 **수정 결정·소유·순서**만 정한다.
> **선행 조건**: d-66(`wf_d32c59d4`) 완료 → 메인 검증 → 커밋 뒤에 착수한다(같은 파일 다수).

## 0. 판정 (메인)

- 24건 전부 수정. 중복 4쌍(#5↔aux-3, #13↔aux-4·5, #6↔terminal-2, #9↔session-2)은 한 지점에서 한 번.
- **#8(dirty 탭 닫기 확인)** 은 `tabs.md` §8 이 이미 요구하는 사양의 미구현이다. 3버튼(저장 / 저장 안 함 / 취소) 다이얼로그를
  단일 진입점 `requestCloseTab` 뒤에 둔다. 직렬 닫기(다른 탭·오른쪽·전체·⌘K ⌘W)는 dirty 목록을 모아 **한 번만** 묻는다.
  "저장 안 함" 은 닫기 전에 `setTabDirty(false)` 를 보내 #21 과 정합. 백그라운드(언마운트) 탭의 "저장" 은 에디터 액션 대신
  `model-registry` 의 모델 값(없으면 미러 내용)으로 `useSaveFile` 경로를 탄다. 터미널 탭의 실행 중 프로세스 확인은 기각 항목
  `tab-menu-2`(백로그) 그대로 범위 밖.
- **#5·#12 flush 핸드셰이크** 는 앱 종료용 `HotExitFlushRequested`/`file_flush_complete` 를 **스코프 있는 단일 계약**으로 일반화한다:
  `scope: 'all' | { window: label } | { project: id }`. 프론트 `HotExitFlushProvider` 는 자기 창·해당 프로젝트 flusher 만 flush 하고
  확인 커맨드를 부른다. 보조 창 닫기(`handle_auxiliary_close_requested`)는 `prevent_close` → 창 스코프 flush → 완료/타임아웃 후 실제 close.
  `project_close` 는 프로젝트 스코프 flush 완료/타임아웃 후 제거·detach·이벤트 fan-out. 타임아웃 폴백은 기존 상수·패턴 재사용.
  복귀한 탭의 유령 dirty(#5 후반)는 `return_auxiliary_window_tabs` 가 그 경로의 미러 존재로 dirty 를 재조정.
- **#11(디렉토리 심링크)** 는 UX 변경이지만 "실존 디렉토리를 없는 파일로 보고" 하는 것은 명백한 오동작이라 수정한다. 끊어진 심링크는
  종전대로 파일 행. 기존 리팩토링 불변성 테스트 2건은 새 동작으로 기대값 교체.
- **#18(스크롤백)** 은 `PtySpawnOptions.scrollback_bytes` 를 추가하고 프론트가 `terminalScrollback`(줄) × `SCROLLBACK_BYTES_PER_LINE_ESTIMATE`(=512,
  상수로 명명·JSDoc) 을 `DEFAULT_SCROLLBACK_BYTES`(2MiB) 하한·`MAX_SCROLLBACK_BYTES`(32MiB) 상한으로 clamp 해 넘긴다. 실행 중 세션 재설정 커맨드는
  만들지 않는다(다음 스폰부터 적용, 문서 명시).
- **#23("폴더에서 찾기")** 은 ① 범위 우선을 채택한다 — 사용자가 명시한 폴더는 검색 대상이다. `SearchQuery` 에 명시적 `scope_dir`(프로젝트 상대)를
  추가해 `findInFolder` 가 glob 대신 이것을 쓰고, walk 는 scope_dir 을 루트로 하며 **scope 서브트리 안에서는 `is_ignored_dir` 가지치기를 하지 않는다**.
  일반 검색(scope 없음)의 동작은 불변.
- **#14·#15** 는 `ProjectRef.root_missing` 미러 + 슬롯 헤더 경고 배지 + 사이드바 비활성 + 탐색기 전용 빈 상태("프로젝트 폴더를 찾을 수 없습니다")
  + "다시 열기" 액션(`project_close` 후 `project_open`) 으로 한 세트.
- **#10** 은 Rust `MirrorEntry.source_missing` 노출(`list_mirrors` 가 원본 부재 미러도 반환, conflict=false) + 프론트 에러 화면 대신 "원본이 삭제됨 —
  초안을 다른 이름으로 저장" 배너 + 초안 본문 표시 + `releaseClosedFileTabPath` 가 원본 부재면 `clearMirror` 를 건너뜀 + `prune_mirrors` keep 확인.
- **#9** 는 미러가 있는 닫힌 프로젝트를 건너뛰고 `ForgetRecentOutcome.skipped_with_drafts` 로 안내(토스트, 로케일 3종).
- **#24** 는 Rust `tree_collapse_all`(=`expanded.clear()`, dispatch 테이블 등재) + 프론트 단일 mutation. 개별 `collapse` 의 자손 기억은 유지.
- 기각 4건(`save-during-transitions-1`·`aux-window-lifecycle-2`·`session-window-ids-5`·`tab-menu-2`) 재론 금지. info(`tree-watcher-search-4`)는 문서
  에이전트가 Rust 주석·`ipc-contract.md` 를 정정한다(코드 주석 정정은 문서 단계에서 허용).

## 1. 수정 표 (웨이브·소유자별)

### 웨이브 1 (병렬)

**R1 — Rust (1 에이전트, 직렬. bindings 재생성은 마지막에 1회)**

| # | 파일 | 수정 |
|---|---|---|
| #13 | `src-tauri/src/domain/layout/service.rs` `return_auxiliary_window_tabs` | push 전 `is_dedupable` + kind 동등 dedupe(dirty·active 승격). 테스트 |
| #20 | 같은 파일 `reopen_closed`·`return_auxiliary_window_tabs` | `clamp_to_pinned_zone` 경유(회수는 루프 후 `sort_by_key(!pinned)` 안정 정렬). 테스트 2건 |
| #21 | 같은 파일 `push_closed` | 스택에 넣기 전 `tab.dirty = false`. 테스트 |
| #9 | `src-tauri/src/domain/project/service.rs` `forget_recent_projects` + `types.rs` | 미러(경로·untitled) 있는 프로젝트 스킵 + `ForgetRecentOutcome.skipped_with_drafts`. 테스트 |
| #14·#15-R | `src-tauri/src/domain/project/{types.rs,service.rs}` | `ProjectRef.root_missing` 미러(`upsert_project_ref`·`restore_session`·`open_project` already_open 분기에서 재계산). 기존 워처 배제 테스트 유지 |
| #11 | `src-tauri/src/domain/tree/service.rs` | `read_children` 심링크 엔트리만 추적 `metadata` 로 kind·has_children 재판정. 테스트 2건 기대값 교체 + 신규 |
| #24-R | `src-tauri/src/domain/tree/{service.rs,commands.rs}`, `lib.rs`, `domain/remote/dispatch.rs` | `collapse_all` + `tree_collapse_all` 커맨드 등재. 테스트 |
| #10-R | `src-tauri/src/domain/file/{types.rs,service.rs}` | `MirrorEntry.source_missing`, `list_mirrors` 원본 부재도 반환, `prune_mirrors` keep 동작 확인. 테스트 |
| 공통 | `src/shared/api/bindings.ts` | 프로젝트 관례대로 재생성(수기 편집 금지) |

**TS-L — LSP (파일 소유: `src/widgets/editor-pane/use-lsp-session.ts`, `src/entities/lsp/**`, `src/shared/lib/lsp/**`, `src/shared/lib/monaco/model-dirty-tracker.ts`, `src/widgets/editor-pane/use-editor-lsp-integration.ts`(+각 test))**

| # | 수정 |
|---|---|
| #1 | `onDidChangeContent` → `didChange` 등록을 `acquireDocument`/`releaseDocument` 의 uri refcount 아래로(문서당 1 리스너). 테스트: 두 pane 마운트 시 1회 전송, 한쪽 해제 후에도 유지, 마지막 해제에서 didClose |
| #2 | WorkspaceEdit 이 열린 파일 모델에 착지하면 그 경로의 탭 dirty 를 세운다. `shared/lib` 은 `entities` 를 import 못 하므로 applier 는 콜백/브리지(`onModelEditedExternally(path)`)를 받고, `use-editor-lsp-integration.ts`(widgets) 가 `collectAllPaneTabs(layout)` 로 그 경로 탭 전부에 `setTabDirty(true)`. 테스트 |
| #3 | 재시도 소진 분기에서 `rejectPendingRequests` + `finalizeSessionDisposal`(내장 폴백 복원 포함). 테스트 |

**TS-T — 탭·터미널·슬롯 UI (파일 소유: `src/widgets/editor-area/{editor-area.tsx,pane-tab-bar.tsx}`, `src/features/tab/**`, 신규 `src/features/tab/close-dirty-tab-dialog.tsx`(위치는 관례 확인), `src/widgets/terminal-pane/terminal-session.tsx`, `src/features/shell-slot/shell-slot-header.tsx`, `src/shared/lib/shell-slot.ts`, `src/app/providers/shell-slot-provider.test.tsx`, 로케일 3종 `src-tauri/resources/locales/{en,ko,ja}.json` + `MESSAGE_NAMESPACES`(+각 test))**

| # | 수정 |
|---|---|
| #8 | `requestCloseTab(tab)` 단일 진입점(X·컨텍스트 메뉴·휠 클릭·⌘W·직렬 4루프·⌘K ⌘W) + dirty(file/untitled) 게이트 3버튼 다이얼로그(기존 `shared/ui/alert-dialog` 패턴). 저장 = 활성이면 `taide.saveFile`, 언마운트면 `useSaveFile` + 모델/미러 값. 저장 안 함 = `setTabDirty(false)` 후 닫기. 로케일 키 `tab.confirmCloseDirty*`. 테스트: 3버튼 각 경로 + 직렬 루프 1회 질문 |
| #22 | 직렬 루프 5곳을 `closeTabsSerially(ids)` 로 통일: NotFound 는 건너뛰고 계속, 그 외 실패는 모아 1회 토스트. 테스트 |
| #16 | `runInTerminal`: 활성 탭이 터미널이면 우선. 테스트 |
| #19 | `toggleTerminal` 복귀 대상 `kind !== 'terminal'`. 테스트 |
| #17 | 스폰 실패 분기에 재시작 버튼(`handleRestart` 가 `setFailure(null)`). 테스트 |
| #7 | 슬롯 헤더 ✕ 에 포커스 무시 마커, `resolveShellSlotIdFromEventTarget` 이 마커를 만나면 `null`. 테스트(비포커스 슬롯 ✕ pointerdown → `session_focus_shell_slot` 미호출) |

### 웨이브 2 (R1·TS-L·TS-T 완료 후, 병렬)

**R2 — Rust (1 에이전트, 직렬. bindings 재생성 마지막 1회)**

| # | 파일 | 수정 |
|---|---|---|
| #5·#12 | `src-tauri/src/{state.rs, domain/window/commands.rs, domain/project/commands.rs, domain/file/commands.rs(flush 확인), events.rs}` | 스코프 있는 flush 핸드셰이크(§0). 보조 창 닫기 `prevent_close` + 창 스코프, `project_close` 프로젝트 스코프. 타임아웃 폴백. `return_auxiliary_window_tabs` 복귀 탭 dirty 재조정(미러 존재 기준 — layout service 소량 수정 허용). 테스트 |
| #6 | `src-tauri/src/domain/terminal/commands.rs` + `events.rs` | `pty_spawn` 말미 `terminal:spawned(sessionId, projectId, cwd, shell)` emit. 테스트 |
| #18 | `src-tauri/src/domain/terminal/{types.rs,commands.rs}` | `PtySpawnOptions.scrollback_bytes`(옵션, 기본 2MiB, 상한 32MiB clamp) → `SessionOutput::new`. 테스트 |
| #23 | `src-tauri/src/domain/search/{types.rs,service.rs,commands.rs}` | `SearchQuery.scope_dir` + walk 루트/가지치기 면제(§0). 테스트: `node_modules/pkg` scope 에서 매치 반환, scope 없음 동작 불변 |
| 공통 | `src/shared/api/bindings.ts` | 재생성 |

**TS-W1 — 프론트(R1 bindings 의존) (파일 소유: `src/app/providers/ipc-sync-provider.tsx`(#4 만), `src/widgets/editor-pane/editor-pane.tsx`·`use-editor-file-persistence.ts`(#10 만), `src/entities/layout/tab-path-change.ts`(#10 가드), `src/entities/tree/**`(#24 훅), `src/widgets/explorer/explorer-container.tsx`(#24·#14), `src/features/shell-slot/shell-slot-header.tsx`(#14 배지 — TS-T 완료 후라 충돌 없음), `src/widgets/app-sidebar/**`(#14 비활성), `src/widgets/welcome/**`·`src/widgets/app-shell/**`(#9 토스트), `src/entities/project/**`(#14 다시 열기), 로케일 3종(+각 test))**

| # | 수정 |
|---|---|
| #4 | `layoutChanged` 핸들러가 캐시된 `auxiliaryWindows` 가 줄었음을 감지하면 `FILE.MIRRORS(projectId)` 무효화(창이 실제로 사라질 때만). 테스트 |
| #10-FE | `editor-pane.tsx` isError + `source_missing` 미러 → 배너("원본이 삭제됨 — 다른 이름으로 저장") + 초안 본문(읽기 전용 CodeEditor 또는 saveAs 경로) / `releaseClosedFileTabPath` 원본 부재면 `clearMirror` 생략. 로케일. 테스트 |
| #9-FE | forget recent 호출부(Welcome·사이드바·네이티브 메뉴 핸들러)가 `skipped_with_drafts > 0` 이면 안내 토스트. 로케일. 테스트 |
| #14·#15-FE | 슬롯 헤더 경고 배지·사이드바 비활성·탐색기 전용 빈 상태 + "다시 열기"(`project_close` → `project_open`). 로케일. 테스트 |
| #24-FE | `useCollapseAllTree` 훅 → `collapseAllExpanded` 단일 mutation. 테스트 |
| #25(d-66 렌즈 minor) | `src/entities/layout/layout.query.ts`(+`use-open-file-tab.test.tsx`): `useOpenFileTab` 이 mutate 전에 `withCurrentWindowTarget` 로 target 을 확정해 넘기고, `onSuccess` 의 `openedFileTabIdOf` 는 그 확정 paneId 만 쓴다(응답 레이아웃의 focusedPane 재계산 제거 — 같은 경로가 다른 pane 에도 열려 있을 때 reveal 이 새는 좁은 레이스 차단). JSDoc 정정. 테스트 |

### 웨이브 3 (R2 완료 후)

**TS-W2 (파일 소유: `src/app/providers/{hot-exit-flush-provider.tsx,ipc-sync-provider.tsx}`, `src/entities/editor/mirror-flush-registry.ts`, `src/entities/file/**`(flush 확인 IPC), `src/entities/terminal/**`, `src/widgets/terminal-pane/terminal-session.tsx`(#6·#18 판정/옵션), `src/widgets/explorer/explorer-container.tsx`(#23 `findInFolder`), `src/widgets/search-panel/**`(범위 칩), `src/entities/search/**`(+각 test))**

| # | 수정 |
|---|---|
| #5·#12-FE | 스코프 flush 요청 처리(자기 창 / 해당 프로젝트 flusher 만) + 확인 커맨드. `mirror-flush-registry` 에 projectId·window 키. 테스트 |
| #6-FE | `terminal:spawned` → `setQueriesData(TERMINAL.SESSIONS_ALL, upsertTerminalSession)`. 테스트: 로스터 캐시에 없는 살아 있는 sessionId 마운트 시 `attachPty` |
| #18-FE | 설정 줄 수 → `scrollback_bytes` 환산 상수(§0) 로 스폰 옵션 전달. 설정 라벨 보조문구("다음 스폰부터"). 테스트 |
| #23-FE | `findInFolder` 가 `scope_dir` 사용, 검색 패널 범위 칩 표시 유지. 테스트 |

### 문서 (마지막, 1 에이전트)

- 이 계약 §3, `docs/bug/2026-09-17-editing-surface-audit-wave2-fixes.md`, `features/tabs.md`(dirty 닫기 다이얼로그·재열기 dirty 정규화·회수 dedupe)·
  `editor.md`(공유 문서 didChange 1회·WorkspaceEdit dirty·미러 복원 창 간·source_missing·flush 스코프)·`terminal.md`(spawned 이벤트·스크롤백 예산·재시작·
  §10 dispose 서술 현행화)·`explorer-sidebar.md`(심링크·scope 검색·collapse_all·root_missing)·`layout-shell.md`(슬롯 ✕ 포커스 무시)·`window-chrome.md`
  §7.1(Clear Recent 초안 보존)·`tasks.md` §3(활성 터미널 우선)·`keymap.md`(토글 복귀)·`ipc-contract.md`(신규/변경 커맨드·이벤트)·`data-model.md:341`·
  wave-i 계약 :105 정정(AppFile dirty 흐름 전제)·`backlog.md`(tab-menu-2 크로스레퍼런스)·Rust `search/service.rs:586` 주석 정정(info).

## 2. 실행·검토 계획

- 웨이브 1: R1 ∥ TS-L ∥ TS-T → 웨이브 2: R2 ∥ TS-W1 → 웨이브 3: TS-W2 → 통합 검증(sonnet·high) → 렌즈 3(sonnet·xhigh: Rust 도메인 /
  LSP·지속성·flush / 탭·터미널·탐색기·슬롯 UI) → major·실패 있으면 수정 1회 + 재검증 → 문서. fixer 는 전부 opus·xhigh.
- 적대적 검증은 조사 단계에서 건별 수행(생존 24 = 반박 실패). 렌즈 major 는 메인이 소스로 판정.
- 메인 2차 검증: `bun run verify` + `bunx vite build` → 커밋 분할 → dev 푸시 → main ff → v0.2.3 릴리스(태그·Release 런·draft).

## 3. 기록 (구현·검토·검증)

> 실행: 웨이브 1(R1 ∥ TS-L ∥ TS-T) → 웨이브 2(R2 ∥ TS-W1) → 웨이브 3(TS-W2) → 통합 검증 → 렌즈 3관점 →
> 수정 1회 + 재검증. fixer 는 전부 opus·xhigh, 검증·렌즈는 계약대로.
> **무엇을 고쳤는가의 정본은 `docs/bug/2026-09-17-editing-surface-audit-wave2-fixes.md`** 이고, 여기에는
> 계약과 실제 구현이 갈린 지점(이탈)·검증 수치·렌즈 결과·남은 위험만 적는다.

### 3.1 구현 — 소유자별 요지

**R1 (Rust 웨이브 1 — #13·#20·#21·#9·#14/15-R·#11·#24-R·#10-R)**

- 변경: `domain/layout/service.rs`, `domain/project/{types,service,commands}.rs`,
  `domain/tree/{service,commands}.rs`, `domain/file/service.rs`, `domain/remote/dispatch.rs`, `lib.rs`,
  `src/shared/api/bindings.ts`(재생성).
- 이탈 4건:
  1. **소유 밖 최소 변경** — `domain/project/commands.rs` 의 `project_forget_recent` 반환 타입을
     `AppResult<u32>` → `AppResult<ForgetRecentOutcome>` 로. 이 커맨드가 `skipped_with_drafts` 가 TS 로 나가는
     유일한 통로라, 바꾸지 않으면 §0 의 "#9 … 안내(토스트)" 자체가 성립하지 않는다.
  2. `ForgetRecentOutcome` 정의를 `service.rs` → `types.rs` 로 이동(specta `Type` 파생이 필요).
  3. **#13 의 "aux active 였다면 survivor 를 active 로" 를 `!had_active` 가드 아래로 좁혔다.** 계약 문구를 문자
     그대로 구현하면 기존 테스트가 고정한 "복귀는 main 의 현재 포커스를 빼앗지 않는다" 불변식과 충돌한다.
  4. `ProjectRef` 구조체 리터럴이 있던 테스트 6곳에 `root_missing: false` 추가(필드 추가에 따른 컴파일 필수).
- **미채택**: #13 의 `preview` 승계(계약이 dirty·active 만 명시) — 렌즈가 minor 로 재지적, 후속.
- 테스트 18건 + 기존 기대값 교체 3건. **검출력 실측**: layout 4 / project 4 / tree 4 / file 2 = 14건이 수정 전
  코드에서 FAILED 임을 임시 되돌리기로 확인 후 원상 복구.

**TS-L (LSP — #1·#2·#3)**

- 변경: `entities/lsp/lsp-session-registry.ts`, `shared/lib/lsp/{model-dirty-tracker,workspace-edit-applier}.ts`,
  `widgets/editor-pane/{use-lsp-session,use-editor-lsp-integration}.ts`(+각 test).
- 계약 경로 정정: 계약 §1 표의 `src/shared/lib/monaco/model-dirty-tracker.ts` 는 실재하지 않는다 —
  실제 파일은 `src/shared/lib/lsp/model-dirty-tracker.ts` 이며 `shared/lib/lsp/**` 도 소유 범위다.
- 이탈 3건: (a) #2 의 부가 제안 2건(`releaseClosedFileTabPath` 최종 dirty 가드 / `disposeModel` 시
  `externallyDirtyPaths` 정리)은 TS-W1 소유 파일이라 하지 않았다 — 주 증상과 독립이고 후속으로 넘겼다.
  (b) #3 의 별건(상태바 LSP 표시 버튼화)은 소유 밖. (c) "소진 후 **새 요청** 즉시 reject" 는 `client.ts` 에
  disposed 가드가 없어 문자 그대로 구현 불가 — 재시도 루프가 도는 **동안** 발생한 요청이 소진 시점에 reject
  되는 것으로 테스트했다.
- 테스트 8건. 검출력 실측: #1 3건 fail, #3 1건 타임아웃, #2 2건 fail.

**TS-T (탭·터미널·슬롯 UI — #8·#22·#16·#19·#17·#7)**

- 변경: `widgets/editor-area/{editor-area,pane-tab-bar}.tsx` + 신규
  `{terminal-tab-targets,close-tabs-serially,use-request-close-tab}`, 신규
  `features/tab/close-dirty-tab-dialog.tsx`, `widgets/terminal-pane/terminal-session.tsx`,
  `features/shell-slot/shell-slot-header.tsx`, `shared/lib/shell-slot.ts`, 로케일 3종 + `MESSAGE_NAMESPACES`.
- 결정 2건(계약이 선택지를 남긴 곳): **untitled 탭의 저장은 훅이 Save As 를 직접 수행**한다(pane 의 핸들러는
  성공 여부도 취소 여부도 돌려주지 않아 "취소했는데 닫는" 데이터 손실이 생기고, 언마운트 탭에는 아예 없다).
  **다이얼로그는 훅이 ReactNode 로 돌려주고 각 호출부가 렌더**한다(`PaneNodeView` 수정 회피) — 대신 모듈 레벨
  카운터로 "확인 중 다른 닫기 무시" 를 전역 보장했다.
- 이탈 3건: (a) `MESSAGE_NAMESPACES`(Rust) 등재 — 계약 §1 이 명시한 소유 범위이고, 미등재 키는 locale 테스트가
  막는다. (b) `collectClosableTabIdsInFocusedGroup` 의 반환 타입을 `Tab[]` 로 바꾸지 않고 호출부에서
  `findPaneLeaf` 로 되돌렸다(그 파일이 어느 fixer 표에도 없다). (c) `PaneTabBar`·`EditorArea` 단위 wiring 렌더
  테스트는 만들지 않았다(dnd-kit·Radix·OverlayScrollbar 전부 세워야 함) — 훅 레벨 10 케이스로 고정하고 wiring 은
  통독으로 확인했다. **이 (c) 를 렌즈가 minor 로 지적했고, 렌즈 major 1 수정에서 마운트 경로 테스트가 들어갔다.**
- 테스트 28건. 검출력 실측 전 항목 수행(각 수정을 임시로 되돌려 fail 확인 후 md5 로 원상 복구 검증).

**R2 (Rust 웨이브 2 — #5·#12·#6·#18·#23)**

- 변경: `state.rs`, `events.rs`, `lib.rs`, `domain/window/{commands,service}.rs`,
  `domain/file/commands.rs`, `domain/project/commands.rs`, `domain/layout/service.rs`(헬퍼),
  `domain/terminal/{types,commands}.rs`, `domain/search/{types,service}.rs`, `tests/domain_boundaries.rs`,
  bindings 재생성.
- 설계 결정 4건: `FlushScope` 를 `state.rs` 에 둔 것(핸드셰이크 상태가 사는 곳), **스코프별 토큰**(늦은
  타임아웃이 다음 핸드셰이크를 강제완료하지 못하게), `project_close` 의 **인라인 await**(커맨드가 "프로젝트가
  실제로 사라진 뒤" resolve 한다는 기존 계약 유지 — 대기는 `begin_mutation` **전**), 보조 창은 자기 티켓을
  await 한 뒤 **스스로 `window.close()` 재발행**(프로젝트 스코프와 대칭 + 도메인 간 엣지 제거).
- 이탈 3건: (a) `tests/domain_boundaries.rs` 에 `window/service → file::service` 엣지 등재(등재 없이는 실패).
  (b) **#23 의 scope 오류에 신규 로케일 키를 만들지 않았다** — `MESSAGE_NAMESPACES` 가 그 시점 TS-W1 과
  충돌 위험이 커서 `InvalidArgument` 평문 + 기존 `error.path.outsideProjectRoot` 로 갈음했다.
  (c) `domain/layout/service.rs` 를 "소량 헬퍼" 보다 조금 넓게 건드렸다(유령 dirty 헬퍼 + 테스트 4건 +
  `SearchQuery` 필드 추가에 따른 기계적 수정) — R1 의 `return_auxiliary_window_tabs` 본문은 한 줄도 바꾸지 않았다.
- 테스트 30건. **검출력 실측은 #23 2건뿐**이다 — 나머지는 대상 함수·타입 자체가 수정 전에 없어(컴파일 불가)
  실측이 성립하지 않는다.

**TS-W1 (프론트 웨이브 2 — #25·#4·#24-FE·#9-FE·#14/15-FE·#10-FE)**

- 변경: `entities/layout/{layout.query,tab-path-change}.ts`, `entities/{tree,project}/**`,
  `app/providers/ipc-sync-provider.tsx`, `features/shell-slot/shell-slot-header.tsx`,
  `widgets/{app-shell,app-sidebar,explorer,editor-pane}/**`, 로케일 3종 + `MESSAGE_NAMESPACES`(+각 test).
- 결정 3건: **#14 의 "사이드바 비활성" 을 클릭 차단이 아니라 흐림 + 툴팁**으로 해석(복구 액션에 닿으려면 그
  슬롯을 포커스할 수 있어야 한다), **탐색기 빈 상태 조건을 `rootMissing || treeRows isError` 의 OR** 로(세션
  도중 드라이브가 빠지면 `root_missing` 이 갱신되지 않는다), **#10 초안 본문을 `CodeEditor` 가 아니라 읽기 전용
  `<pre>`** 로(같은 `registryTabId` 로 monaco 를 하나 더 마운트하는 것이 editor-corpse 크래시 클래스다).
- 이탈 3건: (a) `MESSAGE_NAMESPACES` 에 자기 키 5개 등재. (b) `widgets/explorer/explorer-panel.tsx` 수정
  (컨테이너에서 단락시키면 뷰 전환기까지 사라진다). (c) TS-L 소유 테스트의 `project.ipc` 부분 목에
  `forgetRecentProjects` 한 줄 추가(`mock.module` 이 프로세스 전역이라 미추가 시 같은 런의 다른 파일이
  `SyntaxError` 로 깨진다 — 스코프 실행으로 재현 확인).
- **미완**: #9-FE 의 토스트에 도달할 호출부가 없다고 보고했다(렌즈가 major 로 확정 → 3.3 에서 해소).
- 테스트 27건. 검출력 실측: #25 1 / #14·#15 3 / #10 2건 FAILED.

**TS-W2 (프론트 웨이브 3 — #5·#12-FE·#6-FE·#18-FE·#23-FE)**

- 변경: `app/providers/{hot-exit-flush-provider,ipc-sync-provider}.tsx`,
  `entities/editor/mirror-flush-registry.ts`, `entities/file/file.ipc.ts`, 신규
  `entities/terminal/scrollback-budget.ts`, `entities/search/**`, `shared/constants/terminal.ts`,
  `widgets/{terminal-pane,explorer,search-panel,search-editor,settings-view,editor-pane}/**`, 로케일 3종.
- 이탈 2건(계약 문구와 다르게 구현): (a) **#6-FE 를 `setQueriesData(TERMINAL.SESSIONS_ALL, …)` 가 아니라
  `setQueryData(TERMINAL.SESSIONS(projectId))`** 로 — `upsertTerminalSession` 은 append 라 전 로스터 스윕은 그
  세션을 남의 프로젝트 로스터에도 집어넣는다. 이벤트가 projectId 를 실어 오므로 정확한 키가 오염 없이 같은
  목적을 달성한다. (b) 소유 밖 5건(`shared/lib/bridge/search-panel-bridge.ts` 의 `scopeDir` 필드,
  `explorer-panel.tsx` prop, `search-editor-pane.tsx` 2줄, `untitled-pane`·`use-editor-view-state` 의 registry
  인자, `MESSAGE_NAMESPACES` 1줄) — 전부 해당 항목이 성립하기 위한 최소 추가다.
- 테스트 20건. 검출력 실측: #18 3건, #23 1건 FAILED.

### 3.2 검증

| 단계 | 결과 |
|---|---|
| `bun run typecheck` · `typecheck:e2e` | exit 0 |
| `bun run lint` | exit 0 — error 0 / **warning 11(기존 baseline 과 동일 파일·동일 내용)** |
| `bun run format:check` | exit 0 |
| `bun test` | **2912 pass / 0 fail**(288 파일, 6431 expect) |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo test -p taide --lib` | **1731 passed / 0 failed** |
| `cargo test --workspace` | 통합 포함 전부 통과(`domain_boundaries`·`capability_symmetry`·`session_restore`·`cli`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0(`#[allow]` 추가 없음) |
| `src/shared/api/bindings.ts` | specta export 테스트로 재생성 — 생성 헤더 유지, 수기 편집 흔적 없음 |

- 웨이브 경계에서는 의도적으로 빨간 구간이 있었다(R1 의 `MirrorEntry.sourceMissing` 필수화 → TS-W1 소유 3파일,
  R2 의 `fileFlushComplete(scope)`·`scopeDir` → TS-W2 소유 2파일). 전부 **다음 웨이브 소유 파일**이었고 웨이브 3
  완료 시점에 0 이 됐다.
- 위 수치는 렌즈 수정 **이후**의 재검증 값이다(수정 전 통합 검증은 2907 pass / 1730 passed).

### 3.3 렌즈 검토 — 전 9건과 처리

| # | severity | 제목 | 처리 |
|---|---|---|---|
| 1 | **major** | dirty 탭 닫기의 "저장" 이 실제 디스크 쓰기 완료를 기다리지 않는다(`use-request-close-tab.tsx` → monaco 액션 → `handleSave` 가 `mutate` 로 발행) | **수정**. `handleSave` 를 `mutateAsync` 기반 boolean 반환으로 바꾸고 신설 `entities/editor/save-request-registry.ts`(tabId 키)로 pane 파이프라인의 결과를 닫기 확인에 전달. 테스트 5건 |
| 2 | **major** | `#9-FE` 초안 보존 안내가 어떤 호출부로도 도달 불가(유일 트리거인 네이티브 메뉴가 `ForgetRecentOutcome` 을 버린다) | **수정**. `project_forget_recent` 가 신규 이벤트 `project:recent-cleared` 를 무조건 emit, 프론트는 `useRecentProjectsClearedNotice`(IpcSyncProvider) 한 곳에서 안내. 이벤트 수 29 → 30 |
| 3 | minor | `project_close` 의 재진입 가드가 flush 대기 중 동시 호출을 못 막고 doc 문구가 사실과 다름 | **수정**(후속 wf `wf_518a3c47`). `begin_mutation` 직후 `projects.contains_key` 재검사 — 이미 닫혔으면 조용히 `Ok(())`(메인 판정: 이중 닫기는 목표 달성이라 NotFound 토스트를 띄우지 않는다). doc 정정 + 소스 위치 테스트 |
| 4 | minor | #13 dedupe 가 `open_tab` 과 달리 `preview` 를 승격하지 않음 | **수정**(후속 wf). 병합 분기에 `open_tab` 과 같은 `preview` 승격 + 테스트 |
| 5 | minor | `externallyDirtyPaths` 가 `disposeModel` 시 정리되지 않아 재개방 시 거짓 dirty | **수정**(후속 wf). `clearExternallyDirtyMark(path)` 를 `TabPathChangeDeps` 로 주입해 `releaseClosedFileTabPath` 가 `disposeModel` 직후 호출 + 테스트(신규 `model-dirty-tracker.test.ts` 3건) |
| 6 | minor | #3 의 내장 TS 폴백 복원을 단언하는 테스트가 없다 | **테스트 부채**(backlog / `docs/quality-assurance`). harness 개조 범위 과도 |
| 7 | minor | `persistMirror` 의 `setQueryData` 제네릭 생략으로 `sourceMissing` 누락을 tsc 가 못 잡음 | **수정**(후속 wf). `<MirrorEntry[]>` 명시 + `sourceMissing: diskModifiedMs === null`(백엔드 `list_mirrors` 와 같은 유도 — `false` 하드코딩은 외부 삭제된 파일의 초안을 닫기 시 파기시킬 위험) |
| 8 | minor | #8 회귀 테스트가 mounted(활성) 탭 경로를 검증하지 않음 | 렌즈 major 1 수정에서 **해소** |
| 9 | minor | #9-FE 안내 도달 불가(위 2 의 테스트 측면) | 렌즈 major 2 수정에서 **해소** |

- major 2건은 메인이 소스로 직접 판정했다(렌즈 severity 를 그대로 신뢰하지 않는다는 기존 원칙).
- 렌즈 major 2 는 계약 §1 이 프론트 전용으로 잡았던 항목이지만 근본 원인이 Rust 쪽(커맨드가 결과를 창에 알리지
  않음)이라 이벤트 추가 + bindings 재생성까지 했다. **미채택**: 렌즈 제안 후반부("Welcome·사이드바에 진입점
  추가")는 새 제품 표면 결정이고 그 파일들은 d-67 이 만진 적이 없어 하지 않았다.

### 3.4 미확인 · 실기 대상

- **앱 실기(`tauri dev`/build)는 전 구간에서 하지 않았다.** 테스트는 fake monaco·fake IPC·순수 헬퍼 층까지만
  덮는다. 실기 체크리스트는 `docs/bug/2026-09-17-editing-surface-audit-wave2-fixes.md` §9 가 정본이다.
- 특히 확인이 필요한 것: flush 왕복 3경로(앱 종료·보조 창 닫기·프로젝트 닫기)와 **보조 창 라벨이 Rust
  `FlushScope::Window` 와 실제로 일치하는지**(어긋나면 데이터 손실은 없고 2.5초 지연으로 나타난다), 실제 vtsls
  에서의 didChange 카운트와 크래시 3회 후 내장 구문검사 복귀, 심링크가 많은 `node_modules` 펼침 속도,
  스크롤백 예산 상향 후 다중 터미널의 상주 메모리, 네이티브 `File > Clear Recent` 경로, 그리고 신규 UI 5종
  (닫기 다이얼로그·슬롯 경고 배지·탐색기 빈 상태·삭제 배너·터미널 재시작 버튼)의 **라이트·다크 렌더**.
- 알려진 표면 변화 3건(회귀가 아니라 의도): 단일 탭 닫기 실패가 이제 토스트를 띄운다, 보조 창 닫기가 첫
  `CloseRequested` 를 가로채 한 왕복만큼 늦어진다(`layout_move_tab_to_window` 의 빈 창 자동 닫기 포함),
  `project_close` 가 프론트 왕복 1회만큼 느려진다(`project_open` 실패 롤백 경로 3곳 포함).

## 4. 후속

> 아래 2건 + 렌즈 minor 에서 나온 6건 + #10 의 prune keep 잔여는 전부
> `docs/backlog.md` 의 "d-67 …에서 분리된 후속 후보" 표에 사유와 함께 등재했다.

- 보조 창 파일의 IDE `save_document` 실저장(d-66 #2 잔여) — 요청 릴레이 설계 백로그.
- 슬롯 닫기 시 살아남은 슬롯 리마운트(기각 `save-during-transitions-1` 의 minor 잔여: key 를 슬롯 정체성으로 분리) — 백로그.
