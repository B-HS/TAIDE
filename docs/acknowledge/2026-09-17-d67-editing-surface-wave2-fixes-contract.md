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

(구현 wf 완료 후 문서 에이전트가 채운다)

## 4. 후속

- 보조 창 파일의 IDE `save_document` 실저장(d-66 #2 잔여) — 요청 릴레이 설계 백로그.
- 슬롯 닫기 시 살아남은 슬롯 리마운트(기각 `save-during-transitions-1` 의 minor 잔여: key 를 슬롯 정체성으로 분리) — 백로그.
