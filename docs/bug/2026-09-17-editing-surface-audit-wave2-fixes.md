# 편집 표면 2차 전수조사 확인 결함 24건 일괄 수정 (2026-09-17)

> 사용자 지시(d-66 과 같은 포괄 지시): "버그될 만한 건 다 바꿔 확실하게 수정" + "커밋 푸시 draft 잘 생성해두고".
> 조사 정본은 `docs/research/2026-09-17-editing-surface-bug-audit-wave2.md`(7관점 finder → 반박 검증 생존
> major 11·minor 13, 기각 4, info 1), 수정 결정·소유·순서는
> `docs/acknowledge/2026-09-17-d67-editing-surface-wave2-fixes-contract.md`.
> 아래 `#N` 은 전부 그 조사 문서 §1 의 항목 번호이며, **증상·재현의 정본은 조사 문서 §2** 다. 이 문서는
> **무엇을 실제로 고쳤는가**를 기록한다. 선행 배치는 `docs/bug/2026-09-17-editing-surface-audit-fixes.md`(d-66),
> 그 앞은 `docs/bug/2026-09-17-stale-focused-pane-and-focus-not-following-click.md`(d-65)다.
> 항목 번호 뒤의 `-R`/`-FE` 는 같은 결함의 Rust 쪽 / 프론트 쪽 절반을 뜻한다(계약 §1 의 웨이브 분할).

## 1. Rust — 레이아웃 불변식 · 탭 회수

### #13 — 보조 창을 닫으면 같은 파일 탭이 한 pane 에 두 개가 된다

- **원인**: `src-tauri/src/domain/layout/service.rs` `return_auxiliary_window_tabs` 가 회수한 탭을 무조건
  push 했다. `open_tab` 은 kind 동등 dedupe 를 하는데 회수 경로만 그 판정을 안 탔다.
- **수정**: push 전에 `is_dedupable(&tab.kind)`(d-66 #16·#17 이 만든 그 가드) + `kind` 동등으로 기존 탭을
  찾아, 있으면 `existing.dirty |= tab.dirty` 로 **미저장 상태만 승계**하고 새 탭은 만들지 않는다. 세션 id 가
  빈 터미널(`is_dedupable=false`)은 종전대로 push 된다.
- **이탈**: 계약의 "aux active 였다면 survivor 를 active 로" 는 **기존 `!had_active` 가드 아래에서만** 적용했다.
  즉 대상 리프가 비어 있어 루프가 원래 `*active` 를 세우던 경우에만 survivor id 를 세운다. 함수 doc 과 기존 테스트
  `보조_창_닫기는_탭을_main_말미로_복귀시키고…` 가 "복귀는 main 의 현재 포커스를 빼앗지 않는다" 를 불변식으로
  고정하고 있어, dedupe 경로만 포커스를 빼앗으면 push 경로와 비일관해진다. 폐기되는 id 가 `active` 로 남는
  댕글링은 이 처리로 함께 해소된다.
- **미채택**: `preview` 승계는 하지 않았다(계약이 dirty·active 만 명시) — 렌즈 minor 로 다시 지적됐고 §7 에서
  후속으로 분류했다.
- **테스트**: 2건(같은 파일 탭 합치기 + dirty 승계 / 세션 id 가 빈 터미널은 합치지 않음). 검출력 실측 — 수정 전
  코드에서 FAILED.

### #20 — ⌘⇧T 재열기·보조 창 회수가 핀 구역을 넘는다

- **원인**: 같은 파일. d-66 #10 이 만든 `clamp_to_pinned_zone` 을 `move_tab` 만 탔고 `reopen_closed`·
  `return_auxiliary_window_tabs` 는 raw index 로 삽입했다.
- **수정**: `reopen_closed` 는 `find_leaf`(불변)로 클램프를 계산한 뒤 `find_leaf_mut` 로 삽입한다
  (`insert_tab` 자체는 그대로 — 다른 호출부 무영향). 회수는 매 탭 클램프 대신 push 루프 종료 후
  `sort_by_key(|tab| !tab.pinned)` **안정 정렬** 1회다(`pin_tab` 과 같은 키) — 각 그룹의 도착 순서가 보존된다.
- **테스트**: 2건(조사 §2 #20 의 재현 ①②). 검출력 실측 — 수정 전 FAILED.

### #21 — ⌘⇧T 로 되살린 탭에 유령 dirty 점이 영구히 남는다

- **원인**: 같은 파일 `push_closed` 가 닫히는 시점의 `dirty` 를 그대로 스택에 적재했다. 복원 탭의 본문은
  디스크인데 플래그만 남아 "저장된 탭 닫기" 가 그 탭을 영원히 건너뛴다.
- **수정**: 스택 적재 **직전** `closed.tab.dirty = false` 로 정규화한다. `close_tab` 이 호출부로 **반환**하는
  `ClosedTab` 은 실제 닫힌 탭 정보이므로 건드리지 않았다(프론트가 닫힘 시점 dirty 를 읽는 경로 보존).
- **테스트**: 1건(스택 top 이 clean + reopen 후에도 clean). 검출력 실측 — 수정 전 FAILED.

### #9 — "최근 항목 지우기" 가 닫힌 프로젝트의 미저장 초안을 지운다

- **원인**: `src-tauri/src/domain/project/service.rs` `forget_recent_projects` 가 `projects/<id>/` 를 통째로
  지웠다. 그 아래에 hot-exit 미러(= 유일본인 미저장 초안)가 있어도 경고가 없었다.
- **수정**: `paths.buffers_dir(id)` 하위에 **파일이 하나라도 있으면** 그 프로젝트를 건너뛴다(경로 미러와
  `untitled/` 하위를 한 번에 덮고 `domain::file` 의 미러 네이밍을 재유도하지 않아 도메인 경계도 지킨다).
  빈 버퍼 디렉터리는 초안으로 보지 않는다. 건너뛴 수는 `ForgetRecentOutcome.skipped_with_drafts` 로 나간다.
- **타입**: `ForgetRecentOutcome` 을 `service.rs` → `types.rs` 로 옮기고 `Serialize + specta::Type +
  serde(camelCase)`, 카운트는 `usize` → `u32`(IPC 관례). 필드는 `removed`·`skipped_with_drafts`·
  `groups_changed`. `project_forget_recent` 반환 타입이 `u32` → `ForgetRecentOutcome` 로 바뀌었다
  (호출부인 `lib.rs` 메뉴 핸들러는 §7 에서 이 값을 쓰게 된다).
- **테스트**: 3건(미저장 초안 스킵 / untitled 초안만 있어도 스킵 / 빈 버퍼 디렉터리는 과보호하지 않음).

### #14·#15-R — 루트가 사라진 프로젝트가 아무 표시 없이 복원된다

- **원인**: `Project.root_missing` 은 있었지만 세션의 `ProjectRef` 에는 없어, 슬롯·사이드바가 읽을 수 있는
  표시가 어디에도 없었다.
- **수정**: `ProjectRef.root_missing`(`#[serde(default)]`)을 추가하고 `restore_session`(참조를 `iter_mut` 로
  돌며 미러)·`upsert_project_ref`(갱신·신규 양쪽)·`open_project` 의 `already_open` 분기(`false` 로 내리고
  upsert)에서 채운다. `projects_pending_watcher_restore` 의 워처 배제는 **그대로 유지**했다 — 세션 중 회복은
  탐색기의 "다시 열기"(close→open)가 담당한다(#15-FE).
- **테스트**: 2건(루트 부재 시 미러 표시 / 이미 열린 프로젝트를 다시 열면 해제).

### #11 — 디렉토리 심링크가 '파일' 행으로 떠서 열 수도 펼칠 수도 없다

- **원인**: `src-tauri/src/domain/tree/service.rs` `read_children` 이 `d_type` 만 봤다(심링크는 `is_dir()`
  이 false).
- **수정**: **심링크 엔트리에 한해서만** 추적 `std::fs::metadata` 1회를 지불해 kind 를 재판정하고,
  `has_children` 도 재판정된 kind 로 계산한다. 비-심링크의 `d_type` 최적화는 그대로다. 끊어진 심링크는
  `metadata` 실패 → 종전대로 파일 행.
- **순환 가드 미추가**: `expand` 는 사용자의 명시적 펼침에만 반응하고 재귀 프리로드가 없으며,
  `directory_has_children` 은 1단계, `ancestors_under_root`/`reveal` 은 경로 컴포넌트로 유한하다 — 무한 순회
  경로가 없어 캐시 canonical 가드는 넣지 않았다(조사 fixNote 의 "검토" 결과).
- **테스트**: 기존 불변성 테스트 2건의 기대값과 이름을 새 동작으로 교체
  (`심링크는_가리키는_대상의_종류로_표시된다`, `디렉토리_심링크는_펼침_가능하고_끊어진_심링크는_아니다`) +
  정렬 1건 신규. 검출력 실측 — 수정 전 3건 FAILED.

### #24-R — "모두 접기" 가 화면 밖 펼침 상태를 남긴다

- **수정**: `service::collapse_all(state)` = `expanded.clear()` 만. **디렉토리 캐시는 유지**한다(사용자
  상태가 아니라 디스크 읽기 캐시). 개별 `collapse` 의 자손 기억은 종전 그대로다.
- **커맨드**: `tree_collapse_all(projectId) → TreeRowPage` 를 `tree_toggle` 패턴을 전수 복제해 등재했다 —
  `lib.rs` `collect_commands!`, `dispatch.rs` 의 `IMPLEMENTED_JSON_COMMANDS`·`REMOTE_ALLOWED_COMMANDS`·
  dispatch match arm. `SpanSlot` 은 추가하지 않았다(`tree_refresh` 에도 없고 접기는 디스크를 읽지 않는다).
- **테스트**: 2건(화면 밖 자손의 펼침까지 지움 / 캐시는 버리지 않음).

### #10-R — 원본이 사라지면 그 파일의 초안을 어디서도 꺼낼 수 없다

- **원인**: `src-tauri/src/domain/file/service.rs` `list_mirrors` 가 원본이 없는 미러를 **목록에서 제외**해,
  유일본인 초안이 앱 어느 표면에도 나타나지 않았다.
- **수정**: 원본 부재 미러도 반환하되 `MirrorEntry.source_missing: true`, `conflict: false`,
  `disk_modified_ms: None` 으로 채운다(원본이 있으면 종전과 동일). `prune_mirrors` 는 원본 존재 여부를 판정에
  넣지 않음을 테스트로 고정했다 — `keep_paths` 에 있으면 생존한다.
- **테스트**: 3건(기존 "제외된다" 테스트를 "source_missing 표시와 함께 남는다" 로 교체 + 원본 있는 미러 회귀
  방지 + prune keep 동작).

## 2. Rust — flush 스코프 · 터미널 · 검색

### #5·#12 — 보조 창 닫기·프로젝트 닫기가 flush 핸드셰이크를 안 탄다

- **증상**: 보조 창을 닫으면 마지막 debounce 구간의 편집이 사라지고 복귀한 탭에는 유령 dirty 점만 남았다.
  `project_close` 는 프로젝트를 **먼저** 제거해, 언마운트 시점의 미러 flush IPC 가 항상 실패했다.
- **수정 — 계약 일반화**: 앱 종료 전용이던 `HotExitFlushRequested`/`file_flush_complete` 를 스코프 있는 단일
  계약으로 넓혔다. `FlushScope = 'all' | { window: label } | { project: id }`(`state.rs`, serde external
  tagging + camelCase). `AppState` 의 단일 `HotExitFlushPhase` 는 스코프별
  `HashMap<FlushScope, FlushHandshake>`(토큰 + oneshot 완료 신호)로 바뀌었고, 기존 All 스코프 공개 API 4종
  (`begin_hot_exit_flush`·`complete_hot_exit_flush`·`forget_hot_exit_flush_window`·
  `force_complete_hot_exit_flush`)과 그 테스트 8종은 **시그니처·동작 그대로**다.
- **토큰이 필요한 이유**: `editor-1` 라벨은 다음 보조 창에 재발급되고 프로젝트는 닫았다 다시 열 수 있다.
  늦게 터진 타임아웃이 같은 스코프를 점유한 *다른* 핸드셰이크를 강제완료해 엉뚱한 창을 닫는 것을 토큰 불일치로
  차단한다.
- **보조 창 닫기**: `handle_auxiliary_close_requested` 가 `prevent_close` → 창 스코프 flush → 확인/타임아웃
  후 `window.close()` **재발행**이다. 완료 처리를 `file_flush_complete` 에서 dispatch 하지 않고 창 스코프
  태스크가 자기 티켓을 await 한 뒤 스스로 재발행하게 해, 프로젝트 스코프와 대칭이 되고
  `domain/file/commands.rs → window::commands` 도메인 간 엣지가 사라졌다. 재발행한 close 가 다시 가로채이지
  않도록 `take_completed_flush`(Ready 인 핸드셰이크를 소비)로 판별한다(별도 마커 집합 없음).
- **프로젝트 닫기**: `project_close` 는 spawn+콜백이 아니라 `FlushTicket::wait(2.5s)` **인라인 await** 다 —
  커맨드가 "프로젝트가 실제로 사라진 뒤" resolve 한다는 기존 계약을 지켜야 `project_open` 의 롤백 경로와
  #15-FE 의 "다시 열기"(close→open)가 깨지지 않는다. 대기는 `begin_mutation` 취득 **전**에 둬 전역 뮤테이션
  가드를 프론트 왕복 동안 쥐지 않는다.
- **창 파괴 정리**: `forget_hot_exit_flush_window(label)` 이 그 라벨의 창 스코프 핸드셰이크도 함께 제거한다.
  `lib.rs` 의 `WindowEvent::Destroyed` 팔이 이미 이 함수를 부르므로 lib.rs 수정 없이 충족된다.
- **복귀 탭 유령 dirty(#5 후반)**: 순수 헬퍼 `clear_auxiliary_window_phantom_dirty(layout, slot, &has_mirror)`
  를 layout service 에 두고, 미러 존재 사실은 window service 가 `file_service::list_mirrors` 로 주입한다.
  대상은 **닫히는 보조 창의 File 탭만**, 시점은 **회수 병합 전** 이다 — main 창 탭은 debounce 중이라 미러가
  없어도 진짜 dirty 일 수 있고, 먼저 해제해야 #13 의 `existing.dirty |= tab.dirty` 가 유령 dirty 를 생존 탭으로
  승계하지 않는다.
- **소유 밖 1건**: `src-tauri/tests/domain_boundaries.rs` 의 `ALLOWED_CROSS_DOMAIN_EDGES` 에
  `domain/window/service.rs → file::service`(`list_mirrors`) 엣지를 사유와 함께 등재했다(등재 없이는 그 테스트가
  실패한다).
- **테스트**: `state.rs` 13건(스코프 독립 / 이전 핸드셰이크 타임아웃이 새 것을 강제완료하지 않음 / 완료 1회
  회수 / 재진입 / 없는 스코프 무시 / 창 망각 / 프로젝트 재닫기 / `FlushTicket` tokio 3건) + 기존 앱 종료 8건
  무수정 통과(All 스코프 동작 불변 증거) + layout 4건(유령 dirty).

### #6 — 터미널 탭을 다른 창으로 옮기면 셸을 새로 스폰한다

- **수정**: `pty_spawn` 말미에 `terminal:spawned(sessionId, projectId, cwd, shell)` 를 emit 한다. emit 은
  `store.insert` **이후** 다 — 이벤트를 받고 즉시 `TerminalStore` 를 조회하는 리스너가 항상 세션을 찾는다.
  `collect_events!` 와 `fanout_remote_events!` 양쪽에 등재했다(의도적 예외 목록은
  `HotExitFlushRequested`/`AgentExternalOpen` 둘뿐이고, 원격 세션도 로스터를 수렴해야 한다).
- **테스트**: 이벤트 페이로드 camelCase 1건 + 이벤트 수 parity 갱신(28 → 29).

### #18 — 스크롤백 설정 10만 줄이 2MiB 링에서 잘린다

- **수정**: `PtySpawnOptions.scrollback_bytes: Option<u32>` 를 추가하고 순수 함수
  `resolve_scrollback_bytes(Option<u32>) -> usize` 가 2MiB 하한·32MiB 상한으로 clamp 해 `SessionOutput::new`
  로 넘긴다. 줄→바이트 환산은 계약대로 프론트에 남겼다(`pty_default_options` 는 `scrollbackBytes: null`).
  실행 중 세션 재설정 커맨드는 만들지 않았다 — **다음 스폰부터** 적용된다.
- **테스트**: 5건(기본 예산 / 하한 / 상한 / 범위 안 / 필드 없는 원격 JSON 역직렬화 하위호환).

### #23 — 무시 디렉토리에서 "폴더에서 찾기" 가 항상 0건이다

- **수정**: `SearchQuery.scope_dir`(프로젝트 상대)을 추가하고, `search()` 안에서 **walk 루트만** scope 로
  옮긴다. scope 가 있으면 `filter_entry` 의 `is_ignored_dir` 가지치기를 통째로 끈다(계약 §0 의 "범위 우선").
  `passes_glob_filters` 의 상대경로 기준은 **프로젝트 루트 그대로** 유지해, 결과 경로와 include/exclude glob
  의미가 scope 유무와 무관하게 동일하다. `search_replace` 의 `collect_project_files` 도 같은 scope 를 타
  "검색 결과 범위" 와 "전체 치환 범위" 가 어긋나지 않는다.
- **검증**: scope 는 `root_guard::ensure_within_root`(루트 밖 → 기존 `error.path.outsideProjectRoot`) +
  `is_dir()`(아니면 `InvalidArgument`) 로 본다. 신규 로케일 키는 만들지 않았다.
- **테스트**: 7건(`node_modules/pkg` scope 에서 매치 반환 / gitignore 된 디렉토리 안 / scope 없음 동작 불변 /
  scope 밖 제외 / 빈 scope / 루트 이탈·부재·파일 거부). 검출력 실측 — `resolve_scope` 를 되돌리면 2건 FAILED.

## 3. TypeScript — LSP

### #1 — split view 에서 `didChange` 가 두 번 나간다

- **원인**: `use-lsp-session.ts` 의 `onDidChangeContent` 등록이 `acquireDocument`/`releaseDocument` 의 uri
  refcount **밖**에 있었다. 두 pane 이 같은 모델·같은 client 를 공유하므로 한 글자가 두 번 전송됐다.
- **수정**: `ConnectionState.contentSubscriptions: Map<string, Disposable>` 를 두고 신설
  `subscribeDocumentContentChanges(client, uri)` 를 `acquireDocument` 의 `current === 0` 분기에서만 등록,
  `releaseDocument` 의 마지막 해제(`current <= 1`)에서 didClose 직전에 dispose 한다. `use-lsp-session.ts` 의
  `contentChangeDisposable`·`monacoRangeToLsp` import 는 제거됐다.
- **reinitialize 정합**: 구독 등록은 `isReinitializing` 게이트 **밖**이다. 그 게이트는 didOpen/didClose 전송만
  막는 것이고 모델은 크래시와 무관하며, 이미 열린 uri 는 `current > 0` 이라 replay 중 재구독이 없다.
- **누수 방어**: `disposeSession` 이 남아 있는 `contentSubscriptions` 를 전부 dispose 한다 — 강제 철거
  (프로젝트 닫기·앱 종료·#3 소진)는 `releaseDocument` 를 거치지 않아 리스너가 모델에 남고 죽은 client 로
  didChange 를 쏘게 된다.
- **테스트**: 3건(두 pane 편집 1회 = didChange 1회, 한쪽 해제 후에도 유지 / 마지막 해제에서 didClose + 리스너
  dispose / 강제 dispose 가 열린 문서의 리스너도 정리). 검출력 실측 — 수정 전 3건 FAILED.

### #2 — 백그라운드 탭에 착지한 WorkspaceEdit 이 dirty 를 못 켠다

- **수정**: `shared/lib/lsp/model-dirty-tracker.ts` 에 `onModelEditedExternally(listener) => unsubscribe` 를
  두고, 알림을 **`markModelDirtyExternally` 안에서** 발행한다(호출부를 1줄로 유지해 "외부 dirty 표시" 와
  "탭 dirty 플래그" 가 드리프트할 수 없게 하는 것이 목적). widgets 의 `use-editor-lsp-integration.ts` 가
  이를 구독해 `collectAllPaneTabs(layout)` 로 그 경로의 모든 file 탭에 `setTabDirty(true)` 를 보낸다.
- **리스너는 단일 슬롯이 아니라 `Set`**: `shell-slot-tree-view.tsx` 가 슬롯마다 다른 projectId 의
  `EditorArea` 를 동시에 마운트하므로 last-wins 는 다른 프로젝트의 편집을 놓친다. 같은 프로젝트의 pane 2개가
  중복 호출하는 경우는 멱등이고 핸들러가 `tab.dirty` 를 건너뛴다.
- **배선은 `useSetTabDirty`(entities mutation) 경유**: raw `commands.layoutSetDirty` 로는 백엔드 플래그만
  바뀌고 layout 쿼리 캐시가 갱신되지 않아 화면의 dirty 점이 안 뜬다(`applyFreshLayout` 이 필요).
  판정 로직은 `createExternalModelEditTabDirtyHandler({ getLayout, setTabDirty })` 팩토리로 분리하고, 훅
  본문은 `useEffect` 배선만 남겼다. 레이아웃은 `useQuery` 가 아니라 캐시 읽기다 — 리비전마다 재구독하면
  키 입력마다 구독이 churn 한다.
- **채택 이유**: 조사 문서가 제안한 `layoutSetTabDirtyForPath` dep 대신 계약 §1 의 브리지안을 택했다.
  applier 호출부 4곳(rename·code-action·apply-handler×2)이 전부 `deps: undefined` 로 `defaultDeps` 를 쓰고
  있어 옵션 dep 은 실제로 배선될 수 없었다.
- **테스트**: 4건(미부착 모델 편집 → 구독자 통지 / 포그라운드 편집·구독 해제 후 무통지 / applier→브리지→핸들러
  실제 체인으로 보조 창 포함 모든 file 탭에 `setTabDirty(true)` / 이미 dirty·다른 경로·터미널 탭 무영향).
  검출력 실측 — 수정 전 2건 FAILED(브리지 알림 0건).

### #3 — 재시도가 소진된 크래시 세션이 철거되지 않는다

- **수정**: `reportLspReinitializeFailure` 직후
  `rejectPendingRequests(new Error('lsp session reinitialize retries exhausted'))` → `finalizeSessionDisposal`.
  이 순서여야 매달린 요청이 "재시작 중" 이 아니라 소진 사유로 settle 된다(뒤따르는 `client.dispose()` 의
  'lsp client disposed' 는 이미 비워진 큐에 도달한다).
- **영구 차단 없음**: `finalizeSessionDisposal` 이 `sessionsByKey` 에서 그룹을 제거하므로 다음
  `acquireLspSession` 이 새 세션을 스폰한다(테스트로 단언).
- **미단언**: 내장 TS 폴백 복원(`typescriptDefaults.setDiagnosticsOptions`)은 단언하지 않았다 — 그러려면
  테스트 harness 의 FAKE_MONACO 에 어댑터 등록 API 20여 개를 추가해야 해 개조 범위가 과도했다. 복원 경로 자체는
  `disposeSession` 의 기존 per-language dispose 루프를 그대로 탄다. 렌즈가 같은 공백을 minor 로 지적했고
  §7 에서 테스트 부채로 분류했다.
- **문언 조정**: "소진 후 **새 요청**이 즉시 reject" 는 문자 그대로 구현하지 않았다 — `client.ts` 의 `request`
  에 disposed 가드가 없어 철거 *이후* 새 요청은 reject 가 아니라 영원히 pending 이고, 가드 추가는 `client.ts`
  계약 변경이라 범위 밖이다. 테스트는 재시도 루프가 도는 **동안** 발생한 요청이 소진 시점에 reject 되는 것으로
  작성했다. 검출력 실측 — 수정 전 5초 테스트 타임아웃(reject 가 오지 않음).

## 4. TypeScript — 탭 닫기 · 터미널 · 슬롯 UI

### #8 — dirty 탭을 닫을 때 확인이 없다 (`tabs.md` §8 미구현)

- **단일 진입점**: `src/widgets/editor-area/use-request-close-tab.tsx` 의 `useRequestCloseTab(projectId)` 가
  모든 닫기(탭 ✕·컨텍스트 메뉴·휠 클릭·⌘W·직렬 4루프·⌘K ⌘W)를 받는다. dirty 인 file/untitled 탭이 하나라도
  있으면 요청 **전체에 대해 한 번만** 묻는다(VS Code 와 같은 방식). 다이얼로그는 순수 UI
  `src/features/tab/close-dirty-tab-dialog.tsx`(저장 / 저장 안 함 / 취소).
- **저장**: 마운트된 file 탭은 그 pane 의 저장 파이프라인(read-only 거부·Code Actions on Save·format on save·
  on-save 공백 정리를 지키는 유일한 경로)을 타고, 언마운트 탭은 모델 값 → 미러 내용 순으로 초안을 읽어
  `useSaveFile` 로 쓴다. untitled 탭은 훅이 Save As 를 직접 수행한다 — `untitled-pane` 의 `handleSaveAs` 는
  쓰기 성공 여부도 사용자가 다이얼로그를 취소했는지도 돌려주지 않아 "취소했는데 닫아버리는" 데이터 손실이
  생기고, 언마운트 탭에는 아예 없다. 저장 실패·read-only 거부·Save As 취소는 **닫기를 취소**한다.
- **저장 안 함**: 닫기 전에 `setTabDirty(false)` 를 보내 #21 과 정합한다(닫힘 스택에 dirty 플래그를 들려
  보내지 않는다).
- **다이얼로그 위치**: `PaneNodeView`(다른 소유자)를 건드리지 않기 위해 훅이 `closeDirtyTabDialog` ReactNode 를
  돌려주고 `EditorArea` 와 각 `PaneTabBar` 가 자기 것을 렌더한다. 인스턴스가 여러 개가 되므로 모듈 레벨
  `openConfirmationCount` 로 "확인이 떠 있으면 어떤 진입점의 닫기 요청도 무시" 를 전역 보장했다 — ⌘W 는
  document capture 라 모달이 막지 못한다. `closeFocusedTab`·`closeAllTabsInFocusedGroup` 도
  `isCloseConfirmationOpen()` 으로 선가드해 pinned 경고 토스트조차 뜨지 않는다.
- **범위 밖**: 터미널 탭의 실행 중 프로세스 확인은 기각 항목 `tab-menu-2` 그대로 범위 밖이다(`backlog.md`).
- **테스트**: 10건(저장된 탭 즉시 닫기 / 터미널 탭은 dirty 여도 즉시 / dirty 는 먼저 질문 / 취소 no-op /
  저장 안 함은 setDirty(false)→close 순서 / 저장은 save→close 순서와 미러 내용 / 저장 실패 시 미닫기 /
  다중 dirty 는 질문 1회·답이 전체 적용 / 확인 중 다른 진입점 무시 / untitled Save As 취소 시 미닫기).
  검출력 실측 — 게이트를 무력화하면 7건, 교차 인스턴스 가드를 빼면 1건 FAILED.
  **렌즈가 이 절의 "마운트된 탭" 경로에서 실제 결함을 찾아냈다 — §7 참조.**

### #22 — 직렬 닫기 루프가 중간 실패에서 조용히 멈춘다

- **수정**: 순수 헬퍼 `closeTabsSerially(tabIds, closeTab)` — `isNotFoundIpcError` 는 조용히 건너뛰고 계속,
  그 외 실패는 모아 반환한다. 직렬 닫기 5곳(탭 바 4개 메뉴 + ⌘K ⌘W)이 전부 이 헬퍼를 탄다. 토스트는
  `useCloseTab` 이 아니라 호출부(`runClose`)가 1회 낸다.
- **사용자에게 보이는 변화**: 단일 닫기(✕·⌘W·휠 클릭)도 이제 실패 시 토스트를 띄운다(NotFound 제외). 종전에는
  무통지였다 — 파괴적 액션의 실패를 알린다는 #22 의 취지에 맞다고 판단했다.
- **테스트**: 4건(전량 닫기 / NotFound 건너뛰고 계속 / 그 외 실패는 모아서 루프 계속 / 빈 목록).

### #16·#19 — 태스크가 엉뚱한 터미널로 가고, ⌃\` 가 터미널을 못 빠져나온다

- **수정**: 순수 헬퍼 `src/widgets/editor-area/terminal-tab-targets.ts` 2종.
  `resolveRunTargetTerminalTab(leaf)` 는 **활성 탭이 터미널이면 그것을 우선**하고 아니면 종전대로 첫 터미널,
  `resolveTerminalToggleFallbackTab(leaf)` 는 복귀 대상에서 터미널을 제외한다(터미널뿐이면 `null` → no-op —
  제자리에 있는 편이 튀는 것보다 낫다).
- **테스트**: 7건(#16 4건 / #19 3건).

### #17 — 스폰 실패한 터미널 탭에 재시도 수단이 없다

- **수정**: `terminal-session.tsx` 실패 분기에 exited 분기와 같은 모양의 `terminal.restart` 버튼을 두고,
  `handleRestart` 가 `setFailure(null)` 을 먼저 한다(둘 다 없으면 버튼을 눌러도 에러 화면이 안 걷힌다).
- **테스트**: 2건. 검출력 실측 — 버튼만 되돌리면 2건, `setFailure(null)` 만 되돌리면 1건 FAILED(두 변경이
  각각 load-bearing).

### #7 — 슬롯 헤더 ✕ 가 닫기 직전 그 슬롯으로 포커스를 옮긴다

- **수정**: `SHELL_SLOT_FOCUS_IGNORE_ATTRIBUTE` 를 신설하고 `resolveShellSlotIdFromEventTarget` 이 슬롯 id
  보다 먼저 이 마커를 만나면 `null` 을 반환한다(두 셀렉터를 `closest` 한 번에 넣어 더 가까운 쪽이 이긴다).
  슬롯 헤더 ✕ 는 `display:contents` 래퍼로 감싸 마커를 단다 — `IconButton` 은 소유 밖이고, 비활성 상태에서
  `pointer-events-none` 때문에 이벤트 타깃이 툴팁 래퍼 span 이 되는 경로까지 덮어야 하며, `contents` 라 flex
  레이아웃은 그대로다.
- **테스트**: 3건(헬퍼 2건 + provider 1건 — 다른 슬롯의 헤더 ✕ pointerdown+focusin 에
  `session_focus_shell_slot` 미호출). 검출력 실측 — 수정 전 2건 FAILED.

## 5. TypeScript — 프로젝트 상태 · 초안 복구 · 탐색기

### #4 — 보조 창에서 친 편집이 복귀한 탭에 복원되지 않는다

- **원인**: `FILE.MIRRORS` 가 창별 `staleTime: Infinity` 스냅샷이라 보조 창이 남긴 미러를 메인 창이 모른다.
- **수정**: `ipc-sync-provider.tsx` 의 `layoutChanged` 핸들러가 `LAYOUT.DETAIL` 무효화 프라미스에 체인해
  리페치 뒤 캐시를 다시 읽고, 보조 창이 실제로 사라졌으면 `FILE.MIRRORS(projectId)` 를 무효화한다. 이벤트
  페이로드가 revision 만 싣기 때문에 이 왕복이 필요하다.
- **판정은 개수가 아니라 슬롯 집합**(`hasAuxiliaryWindowClosed`) — 같은 에코에서 한 창이 닫히고 다른 창이
  열려 개수가 같은 경우도 잡는다. 한쪽 스냅샷이 없으면 false(근거 없음)다.
- **테스트**: 7건(창 1개 사라짐 / 여러 창 중 하나 / 닫힘+열림 동시 / 변화 없음 / 새로 열림만 / 필드 없음 /
  스냅샷 부재).

### #10-FE — 원본이 삭제된 파일의 초안 복구 화면

- **수정**: `editor-pane.tsx` 가 `isError` + `source_missing` 미러이면 에러 화면 대신 배너
  (`editor.sourceDeleted`)와 초안 본문, "다른 이름으로 저장"(`editor.saveDraftAs`)을 그린다. 본문은
  `CodeEditor` 가 아니라 **읽기 전용 `<pre>`** 다 — 이 분기는 `resolveEditorStateForRender` 가 같은 실패로
  `editor` 를 null 로 붙잡고 있는 커밋에서 그려지고, 같은 `registryTabId` 로 monaco 인스턴스를 하나 더
  마운트하는 것이 이 파일이 길게 문서화한 editor-corpse 크래시 클래스다.
- **저장**: `untitled-pane` 의 save-as 경로를 따라 `save({ defaultPath: path })` → `useSaveFile` → 성공 시에만
  원본 경로 미러 `clearMirror`. 기본 경로가 원래 경로라 그대로 확인하면 삭제된 파일이 재생성되고
  `FILE.CONTENT(path)` 무효화로 탭이 회복된다. 미러 리페치는 `clearMirror` 프라미스에 **체인**했다 — 나란히
  쏘면 리페치가 지워지는 중인 엔트리를 다시 읽어 저장된 초안 위에 배너가 남을 수 있다.
- **닫기 가드**: `entities/layout/tab-path-change.ts` `releaseClosedFileTabPath` 가 원본 부재 미러이면
  `clearMirror` 를 건너뛴다. 가드는 **캐시된 미러의 `sourceMissing` 만** 본다 — 앱 내 삭제는 파일이 있던 시절에
  받은 캐시를 상대로 동작하므로 종전대로 정리된다(의도된 동작 유지).
- **테스트**: 6건(가드 3건 + 화면 3건). 검출력 실측 — 각각 1건 FAILED.

### #9-FE — 초안 때문에 건너뛴 최근 기록 안내

- **수정**: `skipped_with_drafts > 0` 이면 `project.clearRecentKeptDrafts` 토스트. 최초 구현은
  `useForgetRecentProjects.onSuccess` 에 뒀으나 **호출부가 존재하지 않아 도달 불가**였고, 렌즈가 이를 major 로
  지적해 §7 에서 이벤트 기반으로 옮겼다.

### #14·#15-FE — 루트가 사라진 프로젝트의 표시와 복구

- **슬롯 헤더**: `ProjectRef.rootMissing` 이면 경고 배지.
- **사이드바 레일**: 아이콘을 **비활성(클릭 차단)이 아니라 흐리게 + 툴팁 경고줄**로 했다. 프로젝트는 실제로
  열려 있고 슬롯도 살아 있으며, #15 의 복구 액션(탐색기 "다시 열기")에 닿으려면 그 슬롯을 포커스할 수 있어야
  한다. 문구는 Welcome/Open Recent 가 이미 쓰는 `app.recentProjectRootMissing` 재사용(신규 키 없음).
- **탐색기 빈 상태**: 조건은 `project?.rootMissing === true || treeRows isError` 의 **OR** 다 — 계약은 부팅
  복원 케이스(둘 다 참)를 말하지만, 세션 도중 드라이브가 빠지면 `root_missing` 은 영영 갱신되지 않고(#15 의
  원인) `tree_rows` 실패만 남는다. 두 경우의 복구 수단이 같아 빈 상태를 공유한다.
- **다시 열기**: `useCloseProject` → `onSuccess` 에서 `useOpenProject(project.root)`. 두 mutation 모두
  `PROJECT.ALL` 을 무효화하므로 id 가 바뀌어도 레일 목록이 따라온다.
- **소유 밖 1건**: `explorer-panel.tsx` 에 `rootUnavailable`·`onReopenProject` prop 2개를 추가했다 —
  컨테이너에서 단락시키면 뷰 전환기(search/git/outline)까지 사라진다.
- **테스트**: 9건(슬롯 헤더 2 / 레일 3 / 탐색기 4). 검출력 실측 — `rootUnavailable` 을 false 로 고정하면
  3건 FAILED.

### #24-FE — "모두 접기" 단일 mutation

- **수정**: `tree.ipc.ts` 에 `collapseAllTreeNodes`, `tree.query.ts` 에 `useCollapseAllTree`(기존 tree
  mutation 과 동일한 `setQueryData(TREE.ROWS)` 형태). 탐색기의 `collapseAllExpanded` N회 루프를 단일
  mutation 으로 교체하고 루프는 삭제했다(`toggleNodeAsync` 는 crud 에서 계속 쓰인다).
- **테스트**: 2건.

### #25 — d-66 렌즈 minor: reveal 이 다른 pane 으로 샌다

- **수정**: `useOpenFileTab` 이 mutate **전에** `withCurrentWindowTarget` 로 request 를 확정하고
  `openedFileTabIdOf(layout, request.target, path)` 로 넘긴다. `openedFileTabIdOf` 의
  `target ?? currentWindowFocusedPane(layout)` 폴백을 제거해 응답 레이아웃의 focusedPane 재계산을 없앴다
  (`withCurrentWindowTarget` 은 explicit target 에 idempotent 라 IPC 로 나가는 호출은 불변).
- **테스트**: 1건(응답 레이아웃의 focusedPane 이 옮겨가 있어도 reveal 은 이번에 연 pane 의 탭에 남는다).
  검출력 실측 — 수정 전 FAILED.

## 6. TypeScript — flush 스코프 · 터미널 · 검색 프론트

### #5·#12-FE — 스코프 flush 요청 처리

- **수정**: `HotExitFlushProvider` 가 `payload.scope` 를 읽어 순수 판정 `planMirrorFlush(scope, windowLabel)`
  으로 갈린다 — `'all'` 은 종전대로 전부 flush, `{ window }` 는 **그 라벨의 창만**(다른 창은 flush·확인 둘 다
  하지 않음: `begin_flush` 가 그 스코프에 라벨 하나만 expected 로 기록했다), `{ project }` 는 **모든 창이**
  해당 프로젝트 flusher 만 돌리고 확인한다(`await_project_flush` 가 전 창을 expected 로 잡으므로 그 프로젝트를
  안 연 창이 침묵하면 닫기가 타임아웃 전체를 기다린다).
- **window 축은 registry 키가 아니라 `getCurrentWindow().label` 비교**다. 창마다 JS realm 이 분리돼 registry
  내용이 이미 그 창의 것이고, Rust 라벨(`editor-<n>`)은 `windowSlot` 과 값이 달라 `getWindowContext()` 로
  유도할 수 없다.
- **LSP flush** 는 plan 이 `'all'` 일 때만 돈다 — project 스코프에서 전 세션을 flush 하면 열린 채 남는
  프로젝트의 서버까지 정리되고, 그 경로는 `ipc-sync-provider` 의 `projectClosed` → `flushLspSessionsForProject`
  가 이미 담당한다.
- **registry**: `mirror-flush-registry` 를 projectId 키로 바꾸고 `flushProjectMirrors(projectId)` 를 추가했다.
  등록부 3곳(`use-editor-file-persistence`·`untitled-pane`·`use-editor-view-state`)이 projectId 를 함께 넘긴다.
- **테스트**: 9건(plan 5건 + registry 4건).

### #6-FE — `terminal:spawned` 로스터 수렴

- **수정**: `ipc-sync-provider` 가 이벤트를 받아 `setQueryData(TERMINAL.SESSIONS(payload.projectId))` 로
  upsert 한다. 계약 문구는 `setQueriesData(TERMINAL.SESSIONS_ALL, …)` 였으나, `upsertTerminalSession` 은
  *append* 라 전 로스터 스윕은 그 세션을 다른 프로젝트 로스터에도 집어넣는다 — 이벤트가 projectId 를 실어
  오므로 정확한 키를 쓰는 편이 오염 없이 같은 목적을 달성한다.
- **테스트**: 2건(로스터가 살아 있다고 하면 attach·스폰 안 함 / 로스터에 없으면 종전대로 스폰).

### #18-FE — 설정 줄 수 → 스크롤백 예산

- **수정**: `entities/terminal/scrollback-budget.ts` 의 `resolveScrollbackBytes(lines)` 가
  `SCROLLBACK_BYTES_PER_LINE_ESTIMATE`(512, JSDoc 에 근거 명시)를 곱해 Rust 와 **같은 clamp 창**
  (`DEFAULT_SCROLLBACK_BYTES` 2MiB ~ `MAX_SCROLLBACK_BYTES` 32MiB)으로 좁혀 `pty_spawn` 옵션에 싣는다.
  설정 미조회(null/undefined)·비유한값은 기존 고정 예산(2MiB)으로 떨어진다 — 아직 설정을 못 받은 창이 종전과
  **정확히 같은** 예산으로 스폰되게 하려는 것이다. 설정 화면에는 `settings.terminalScrollbackHint` 보조문구.
- **테스트**: 6건 + 위젯 3건(환산값 전달 / 상한 clamp / 설정 없으면 기본). 검출력 실측 — 수정 전 3건 FAILED.

### #23-FE — "폴더에서 찾기" 가 scope 를 보낸다

- **수정**: `explorer-container.tsx` 의 `findInFolder` 가 include glob 대신 프로젝트 상대 `scopeDir` 을 보내고,
  검색 패널·Search Editor 가 이를 쿼리와 범위 칩으로 소비한다. 칩은 `scopeDir ?? includeGlob` 로 표시하고,
  탐색기가 더는 glob 을 보내지 않으므로 `/**` 접미사를 벗기던 `SCOPE_GLOB_SUFFIX` 는 제거했다. Search Editor
  탭 재실행은 `scopeDir` 을 `includeGlob` 과 같이 보존한다(안 하면 폴더 범위가 조용히 프로젝트 전체로 넓어진다).
- **소유 밖 2건**: `shared/lib/bridge/search-panel-bridge.ts` 에 `scopeDir` 필드 추가(탐색기 → 검색 패널
  유일 경로), `search-editor-pane.tsx` 2줄.
- **테스트**: 6건(브리지 1 / 탐색기 1 / 패널 3 / 쿼리 동등성 1). 검출력 실측 — 수정 전 탐색기 1건 FAILED.

## 7. 렌즈 검토 — major 2건 추가 수정

> 웨이브 3 완료·통합 검증(전부 green) 뒤 3관점 렌즈(Rust 도메인 / LSP·지속성·flush / 탭·터미널·탐색기·슬롯 UI)를
> 돌려 9건(major 2 · minor 7)이 나왔다. major 2건은 메인이 소스로 확인해 **실제 결함**으로 판정하고 근본 수정했다.

### 렌즈 major 1 (#8) — "저장" 이 디스크 쓰기 완료를 기다리지 않았다

- **원인**: 마운트된 file 탭의 저장이 monaco 액션 `taide.saveFile` 을 await 했는데, 그 뒤 `handleSave` 가
  `useSaveFile` 의 `mutate`(비-async)를 부르고 즉시 반환했다. monaco 는 `IEditorAction.run()` 을
  `Promise<void>` 로 타이핑해 결과를 실어 보낼 수도 없다. 결과적으로 **마운트된 탭**에 한해 "저장 실패 시 닫지
  않는다" 가 보장되지 않았고, 저장 실패 직후 닫기가 미러와 모델을 함께 지웠다.
- **수정**: `use-editor-file-persistence.ts` 의 `handleSave` 를 `mutateAsync` 기반으로 바꿔 **쓰기 정착 후
  성공/실패를 boolean 으로 반환**하게 하고, 신설 `src/entities/editor/save-request-registry.ts`(tabId 키)로 pane
  의 저장 파이프라인을 노출해 닫기 확인이 그 결과를 직접 받는다. 실패는 mutation 의 `onError` 가 이미 토스트하므로
  reject 대신 boolean 이다(닫기 확인이 다시 토스트하는 중복을 피하고, ⌘S·자동 저장 경로에 unhandled rejection 을
  만들지 않는다). 쓸 것이 없으면 `true`(잃을 편집이 없음), read-only 거부와 쓰기 실패는 `false`(초안이 유일본).
- **등록 조건**: pane 의 `editor` 존재로 게이팅했다 — 기존 `getEditorInstance(tab.id)` 분기와 같은 조건이라,
  파일 로딩 중이라 draft 가 아직 없는 pane 은 예전처럼 미러 폴백으로 쓰기가 간다. 등록 effect 는 의존성 배열
  없이 매 커밋 재등록한다(`handleSave` 가 이번 렌더의 `file`·`formatOnSave`·editorconfig 를 클로저로 잡는다).
- **테스트**: 5건(쓰기 전에는 resolve 하지 않음 / 실패 보고 / 레지스트리 등록 / 닫기 확인이 파이프라인 종료를
  기다림 / 실패 보고 시 미닫기·초안 되돌려쓰기 없음). 검출력 실측 — 수정 전 2건 FAILED, 레지스트리 조회를 뺀
  판본에서 2건 FAILED.

### 렌즈 major 2 (#9-FE) — 초안 보존 안내가 어떤 경로로도 도달하지 않았다

- **원인**: `useForgetRecentProjects` 를 부르는 UI 가 하나도 없다. 유일한 실제 트리거인 네이티브
  `File > Clear Recent` 는 `lib.rs` 의 `dispatch_menu_action` 이 Rust 에서 커맨드를 직접 부르고
  `Ok(ForgetRecentOutcome)` 을 버린다.
- **수정**: `project_forget_recent` 가 새 이벤트 `project:recent-cleared(removed, skippedWithDrafts)` 를
  **무조건** emit 하고(의미는 "클리어가 실행됨"), 프론트는 `useRecentProjectsClearedNotice`(IpcSyncProvider 에
  마운트)에서 `skippedWithDrafts > 0` 일 때만 안내한다. 토스트는 이 한 곳에만 둬 어떤 경로로 들어와도 안내가
  1회다(훅으로 불러도 그 창이 같은 이벤트를 받는다).
- **네이티브 메뉴를 프론트 mutation 경유로 라우팅하지 않은 이유**: `dispatch_menu_action` 의 직접 호출은 "창이
  0개여도 메뉴가 동작" 하기 위한 의도된 설계(해당 JSDoc 명시)라, 창들에게 알리는 표준 채널인 이벤트를 택했다.
- **미채택**: 렌즈 제안 후반부("Welcome·사이드바에 '최근 항목 지우기' 진입점 추가")는 새 제품 표면 결정이고 그
  파일들은 d-67 이 만진 적 없어 하지 않았다. `useForgetRecentProjects`/`forgetRecentProjects` 는 여전히 UI
  호출부가 없다 — **이번 변경으로 죽은 것이 아니라 원래 죽어 있던 코드**라 컨벤션대로 삭제하지 않고 §9 에 적는다.
- **테스트**: Rust emit 1건(source-scan 관례) + 프론트 2건(안내 1회 / 0이면 무알림) + 무효화 1건 분리.

### 렌즈 minor 7건 — 처리

| 렌즈 항목 | 처리 |
|---|---|
| `project_close` 재진입 가드가 flush 대기 중 동시 호출을 못 막고 doc 이 사실과 다름 | **후속**(§9). 중복 경로도 멱등(close_project·detach_all·이벤트 재발행)이라 사용자 영향이 없고, `await_project_flush` 의 own/joined 반환은 계약 변경이다 |
| #13 dedupe 가 `preview` 를 승격하지 않음 | **후속**(§9). 계약이 dirty·active 만 명시했고 한 줄로 확장 가능 |
| `externallyDirtyPaths` 가 `disposeModel` 시 정리되지 않아 재개방 시 거짓 dirty | **후속**(§9). 조사 fixNote 의 부가 제안이었고 #2 의 주 증상과 독립이다 |
| #3 내장 TS 폴백 복원 단언 없음 | **테스트 부채**(§9). harness 개조 범위 과도 — 복원 경로 자체는 기존 dispose 루프를 탄다 |
| `persistMirror` 의 `setQueryData` 가 제네릭 생략이라 `sourceMissing` 누락을 tsc 가 못 잡음 | **후속**(§9). 그 경로는 원본이 있는 활성 탭 전용이라 `undefined`→falsy 가 우연히 올바른 기본값과 같다 |
| #8 회귀 테스트가 mounted 경로를 검증하지 않음 | 위 렌즈 major 1 수정에서 **해소**(마운트 경로 테스트 2건 추가) |
| #9-FE 안내 도달 불가 | 위 렌즈 major 2 수정에서 **해소** |

## 8. 검증

- Rust: `cargo test -p taide --lib` **1731 passed / 0 failed**, `cargo test --workspace` 통합 포함 전부 통과
  (`domain_boundaries`·`capability_symmetry`·`session_restore`·`cli`), `cargo fmt --all -- --check` ·
  `cargo clippy --workspace --all-targets -- -D warnings` 전부 exit 0(`#[allow]` 추가 없음).
- TS: `bun test` **2912 pass / 0 fail**(288 파일, 6431 expect), `bun run typecheck` · `bun run typecheck:e2e` ·
  `bun run format:check` exit 0, `bun run lint` error 0 / warning 11 — 이 변경과 무관한 **기존 11건 그대로**
  (`useVirtualizer` incompatible-library 6 + exhaustive-deps 5).
- `src/shared/api/bindings.ts` 는 프로젝트 관례인 specta export 테스트로 재생성했다(수기 편집 없음 — 파일 1행의
  생성 헤더 유지 확인).
- 항목마다 **검출력 실측**(수정만 임시로 되돌려 대상 테스트가 실패하는지)을 수행하고 원상 복구를 확인했다.
  결과는 각 항목의 "테스트" 줄에 있다. 단, #5·#12·#6·#18 의 신규 테스트는 대상 함수·타입 자체가 수정 전에
  존재하지 않아(컴파일 불가) 실측이 성립하지 않는다.
- **테스트로 잠기지 않는 것**: 앱 실기(`tauri dev`/build)는 하지 않았다. 실제 flush 왕복 3경로(앱 종료·보조 창
  닫기·프로젝트 닫기), 보조 창 라벨 일치, 실제 vtsls 의 didChange 카운트, pty 스폰·스크롤백 메모리, 네이티브
  메뉴 경로, 렌더 화면(라이트·다크)은 아래 §9 가 정본이다.

## 9. 실기 대상 (사용자)

| # | 조작 |
|---|------|
| #8 | 파일을 편집해 dirty 로 만든 뒤 탭 ✕ → 저장 / 저장 안 함 / 취소 3버튼이 뜬다. "저장" 은 디스크에 쓰인 뒤 닫히고, read-only 파일이면 닫히지 않는다 |
| #8 | dirty 탭 여러 개가 있는 pane 에서 "모두 닫기" → 질문이 **한 번만** 뜨고 답이 전부에 적용된다. 확인이 떠 있는 동안 ⌘W 가 두 번째 다이얼로그를 쌓지 않는다 |
| #21 | dirty 탭을 "저장 안 함" 으로 닫고 ⌘⇧T → 되살아난 탭에 dirty 점이 없다 |
| #13·#20 | 같은 파일을 메인과 보조 창에 열고 보조 창 닫기 → 탭이 하나로 합쳐지고 미저장 상태가 승계된다. 보조 창의 고정 탭은 핀 구역으로 들어간다 |
| #5 | 보조 창에서 타이핑하고 **1초 안에** 창을 닫기 → 편집이 메인 창 복귀 탭에 남아 있고, 유령 dirty 점은 없다 |
| #12 | 파일을 편집한 직후 프로젝트 닫기 → 재실행 시 그 편집이 복구된다 |
| #1 | 같은 TS 파일을 두 pane 에 띄우고 한 글자 입력 → 존재하지 않는 구문 오류 물결선이 뜨지 않는다 |
| #2 | `b.ts` 에서 F2 rename → 열려 있지만 활성화한 적 없는 `a.ts` 탭에 dirty 점이 즉시 뜬다 |
| #3 | 언어 서버를 3회 강제 종료 → 내장 TS 구문 검사가 돌아오고 기능이 영구 정지하지 않는다 |
| #7 | 슬롯 3분할에서 **비포커스** 슬롯의 헤더 ✕ 를 누름 → 포커스가 그 슬롯으로 옮겨가지 않는다 |
| #16 | pane 에 터미널 2개를 열고 두 번째를 보는 중 "Run Task" → 보고 있던 터미널에 명령이 들어간다 |
| #19 | 터미널만 2개 있는 pane 에서 ⌃\` → 터미널끼리 튀지 않는다. 파일 탭이 있으면 그쪽으로 돌아간다 |
| #17 | 존재하지 않는 셸을 설정해 스폰을 실패시킨 뒤 재시작 버튼 → 에러 화면이 걷히고 다시 스폰한다 |
| #6 | 터미널 탭을 다른 창으로 이동 → 같은 셸에 재부착된다(새 셸이 스폰되지 않고 고아도 생기지 않는다) |
| #18 | 설정에서 스크롤백을 크게 잡고 **새 터미널을 연 뒤** 대량 출력 → 탭을 떠났다 돌아왔을 때 복원량이 늘어난다(기존 세션은 불변) |
| #23 | `node_modules` 폴더 우클릭 → "폴더에서 찾기" → 그 안의 매치가 실제로 나온다. 결과 패널의 범위 칩이 그 폴더를 가리킨다 |
| #11 | 디렉토리 심링크(예: pnpm `node_modules/<pkg>`)가 폴더 행으로 뜨고 펼쳐진다. 끊어진 심링크는 파일 행 그대로다. 대형 `node_modules` 펼침 속도도 함께 본다 |
| #24 | 깊은 서브트리를 펼친 뒤 상위 폴더를 접고 "모두 접기" → 상위를 다시 열어도 자손이 접힌 채다 |
| #14·#15 | 프로젝트 폴더를 외부에서 옮긴 뒤 앱 재시작 → 슬롯 헤더 경고 배지·흐린 레일 아이콘·탐색기 빈 상태가 보이고, 폴더를 되돌린 뒤 "다시 열기" 로 워처가 복구된다 |
| #10 | 파일을 편집한 채 외부에서 삭제 → 에러 화면 대신 "원본이 삭제됨" 배너와 초안 본문이 뜨고, "다른 이름으로 저장" 으로 되살아난다. 그 탭을 닫아도 초안이 지워지지 않는다 |
| #9 | 미저장 초안이 있는 프로젝트를 닫고 File > Clear Recent → 그 항목이 남고 "초안이 있어 보존했다" 안내가 뜬다 |
| #4 | 보조 창에서 편집 → 보조 창 닫기 → 메인 창 복귀 탭에 그 편집이 살아 있다 |
| #25 | 좌/우 분할에서 같은 파일이 왼쪽에 열린 채 오른쪽을 포커스 → 검색 결과 클릭 → 오른쪽 새 탭만 그 줄로 이동한다 |
| #22 | 탭 여러 개를 닫는 도중 하나가 다른 경로로 사라져도 나머지가 전부 닫히고, 진짜 실패는 토스트로 보인다 |
| 공통 | 위 신규 UI(닫기 다이얼로그·경고 배지·탐색기 빈 상태·삭제 배너·재시작 버튼)를 **라이트·다크 모드 둘 다** 확인 |

## 10. 이번 범위 밖 · 후속

- **`project_close` 재진입 가드**(렌즈 minor): flush 대기(최대 2.5초) 중 같은 프로젝트에 두 번째
  `project_close` 가 오면 `begin_flush` 실패 호출도 제거·detach·이벤트 발행을 중복 수행한다. 전부 멱등이라
  사용자 영향은 없지만, `handle_auxiliary_close_requested` 가 `begin_flush` 실패 시 본체를 통째로 건너뛰는 것과
  비대칭이다. `await_project_flush` 가 own/joined 를 반환하게 하는 것이 근본 해법 — `docs/backlog.md`.
- **#13 dedupe 의 `preview` 승계**(렌즈 minor): 보조 창의 "영구" 탭이 main 의 preview 탭으로 합쳐지면 다음 단일
  클릭에 교체될 수 있다. `open_tab` 과 같은 한 줄(`if !tab.preview && existing.preview { … }`)로 확장 가능.
- **`disposeModel` 시 `externallyDirtyPaths` 정리**(렌즈 minor): 같은 경로를 다시 열면 디스크와 동일한 내용이
  "미관측 편집" 으로 채택돼 아무것도 안 바꿨는데 dirty 점이 뜬다.
- **`persistMirror` 의 `setQueryData` 제네릭**(렌즈 minor): `<MirrorEntry[]>` 를 명시하지 않아 필드 완전성이
  tsc 에 잡히지 않는다. `sourceMissing: false` 보강과 함께.
- **`prune_mirrors` 의 keep 집합에 source_missing 미러 포함**: `editor-area.tsx` 의 prune 스윕은 프로젝트
  활성화당 1회 돌고 keep 집합이 "열린 탭의 경로" 라, source_missing 탭을 닫은 뒤 프로젝트를 전환하거나 앱을
  재시작하면 그 미러가 지워진다(#10 의 반쪽).
- **#3 의 내장 TS 폴백 복원 테스트**: `docs/quality-assurance` 의 테스트 부채.
- **"최근 항목 지우기" 프론트 진입점**: `useForgetRecentProjects`/`forgetRecentProjects` 는 UI 호출부가 없다
  (d-67 이전부터). 추가한다면 안내 토스트는 이미 이벤트 훅이 담당하므로 mutation 쪽에 다시 넣으면 중복이다.
- **보조 창 파일의 IDE `save_document` 실저장**(d-66 #2 잔여): 요청 릴레이 설계 — `docs/backlog.md`.
- **슬롯 닫기 시 살아남은 슬롯 리마운트**(기각 `save-during-transitions-1` 의 minor 잔여: key 를 슬롯 정체성으로
  분리) — `docs/backlog.md`.
- **기각 4건**(`save-during-transitions-1`·`aux-window-lifecycle-2`·`session-window-ids-5`·`tab-menu-2`)은 기존
  결정 유지 — 재론 금지. 터미널 탭의 실행 중 프로세스 확인(`tab-menu-2`)은 `docs/backlog.md` 참조.
