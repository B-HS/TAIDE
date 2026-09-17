# 편집 표면 버그 전수 조사 — 탭/pane·보조 창·지속성·탐색기·키맵·셸 슬롯 (2026-09-17)

> 범위: 편집 표면(레이아웃 불변식, stale id, 포커스·키맵, 에디터 지속성, 열기 경로, 탭바 DnD, 셸 슬롯·보조 창, 파일 조작↔탭)에서 사용자 가시 결함을 찾는 읽기 전용 조사.
> 실행: 8관점 finder 병렬(읽기 전용, 시간 상한) → 후보 건별 반박 검증(독립 검증자, 소스 재추적) → 메인 분류.
> 반박 규칙: 검증자는 세 가지 중 하나라도 성립하면 기각(refuted)한다. (a) 워킹트리·최근 커밋에서 이미 수정됨, (b) `docs/acknowledge`·`docs/HANDOFF.md`·`docs/backlog.md` 등에서 이미 검토 후 기각·보류로 확정됨, (c) 소스 추적상 재현이 성립하지 않음. 셋 다 아니면 생존.
> severity 열은 finder 원등급이 아니라 **검증자 조정값**이다. 검증자가 둘이고 등급이 갈린 경우 두 값을 함께 적었다.
> 이 조사에서 나온 결론은 전부 소스 통독 기반이며, 앱 실기·테스트 실행은 하지 않았다(읽기 전용 지시).

## 0. 관점(finder) 8종

| key | 조사 축 |
|---|---|
| `layout-invariants` | Rust `domain/layout` 불변식(Leaf.active·sizes·closed_tabs·revision·aux 창 정리·mutation 원자성) |
| `stale-ids` | 지워진 tabId/paneId/windowId/path 를 들고 IPC·캐시를 갱신하는 경로 |
| `focus-keymap` | 포커스 대상 해소(`focusedPane`·활성 파일)와 키맵 등록 범위(메인·보조·Zen) |
| `persistence` | dirty·draft·hot-exit 미러·모델 공유·저장 정착 |
| `open-paths` | 파일/탭을 여는 모든 진입점(⌘P·검색·진단·브레드크럼·탐색기·CLI·IDE)과 reveal |
| `tabbar-dnd` | 탭 배열 순서·핀 경계·preview 슬롯·드래그 드롭 index 계산 |
| `slots-windows` | 셸 슬롯 분할/닫기/포커스 승계, 보조 창 마운트 비대칭 |
| `fileops-tabs` | 파일 생성·이름변경·삭제·이동 ↔ 트리/탭/미러 동기화, 워처 |

## 1. 생존 발견 (severity 순)

| # | id | severity | 영역 | 파일:줄 | 증상(한 줄) | 반박 결과 | 수정안 요지 |
|---|---|---|---|---|---|---|---|
| 1 | `layout-invariants-1` | major | layout / preview 슬롯 | `src-tauri/src/domain/layout/service.rs:495` | dirty 인 preview 탭이 다음 preview 열기에 통째로 교체돼 미저장 편집이 사라지고 ⌘⇧T 로도 못 되살린다 | 2/2 생존 (major) | `set_dirty(dirty=true)` 에서 `tab.preview=false` (편집 시 영구 탭 승격) |
| 2 | `stale-ids-1` | major | ide-sync / 보조 창 | `src/app/providers/ide-sync-provider.tsx:81` | 보조 창의 더티 파일을 못 찾고 Claude Code 에 "저장됨" 으로 거짓 응답 | 2/2 생존 (major) | `findFileTabByPath(layout.root,…)` → `collectAllPaneTabs(layout)` |
| 3 | `stale-ids-2` | major | layout / hot-exit 미러 | `src/entities/layout/tab-path-change.ts:123` | 탭 닫기 시 "다른 pane 에 아직 열림" 판정 **전에** 미러·wait 마커를 해제 | 2/2 생존 (major) | 마커 해제·`clearMirror` 를 `stillOpenElsewhere` 가드 아래로 이동 |
| 4 | `stale-ids-3` | major | terminal / 보조 창 | `src/widgets/terminal-pane/terminal-session.tsx:87` | 보조 창에서 연 터미널이 지정한 cwd 를 잃고 프로젝트 루트에서 스폰 | 2/2 생존 (major) | 창 소유 트리(`resolveWindowPaneTree`)에서 탭 조회, 또는 cwd 를 prop 으로 하향 |
| 5 | `focus-keymap-2` | major | command-palette / 보조 창 | `src/widgets/command-palette/command-palette.tsx:134` | 보조 창의 `@`(심볼)·`:`(라인) 모드가 메인 창 활성 파일을 대상으로 동작 | 2/2 생존 (major) | `activeFilePathOf(resolveWindowPaneTree(layout, getWindowContext()))` |
| 6 | `focus-keymap-3` | major | outline / 보조 창 | `src/widgets/outline-panel/outline-panel-container.tsx:31` | 보조 창 아웃라인이 메인 창 활성 파일의 심볼을 보여줌 | 2/2 생존 (major) | 동일(창 트리 경유), null 가드 필요 |
| 7 | `persistence-3` | major | split view / 모델 공유 | `src/widgets/editor-pane/editor-pane.tsx:205` | 이미 미저장 편집이 있는 파일을 두 번째 pane 에 열면 공유 monaco 모델이 디스크 내용으로 덮인다 | 2/2 생존 (major) | `hasUnobservedModelEdit` 를 "모델 값 ≠ syncedContent" 로 일반화해 adopt 경로로 |
| 8 | `persistence-4` | minor→**major** | hot-exit 복원 / split | `src/widgets/editor-pane/use-editor-file-persistence.ts:498` | 같은 파일 두 번째 pane 에서 "복구됨" 배너 오탐 + 미러 시점으로 공유 버퍼 롤백 | 1/1 생존 (major 로 상향) | 마운트 시 "이미 존재하던 모델인가" 로 분기해 복원 대신 현재 값 채택 |
| 9 | `open-paths-1` | major | editor / reveal | `src/entities/editor/reveal-registry.ts:43` | 줄 이동(reveal)이 다른 pane 에서 먼저 소비돼 정작 탭이 열린 pane 은 1행에 머문다(+포커스 강탈) | 2/2 생존 (major) | reveal 을 항상 pending 큐잉 후 실제 대상 tabId 의 에디터로만 소비 |
| 10 | `tabbar-dnd-1` | minor→**major** | 탭 DnD / 핀 경계 | `src-tauri/src/domain/layout/service.rs:899` | 고정 탭을 드래그하면 raw 탭 순서가 깨져 ⌃Tab·"오른쪽 탭 닫기" 가 화면과 어긋난다 | 1/1 생존 (major 로 상향) | `move_tab` 의 `insert_tab` 직전에 pinned 경계로 index 클램프 |
| 11 | `tabbar-dnd-2` | major | 탭 핀 / preview | `src-tauri/src/domain/layout/service.rs:865` | preview 탭을 고정해도 preview 플래그가 남아, 다음 단일 클릭이 고정 탭을 덮어쓴다 | 2/2 생존 (major) | `pin_tab(pinned=true)` 에서 `tab.preview=false` + `open_tab` 교체 대상에서 pinned 제외 |
| 12 | `fileops-tabs-1` | major (이견 있음) | explorer / 워처 | `src/app/providers/ipc-sync-provider.tsx:239` | `fs:rescan-required` 후 파일 트리가 영구히 낡은 상태로 남는다 | 1 생존(major) / 1 기각(기지 한계) | rescan 을 invalidate 가 아니라 실제 재조회로(캐시 무효화 또는 expanded 전부 `tree_refresh`) |
| 13 | `fileops-tabs-2` | major | explorer CRUD | `src/widgets/explorer/use-explorer-entry-crud.ts:171` | 디렉토리를 삭제해도 선택이 남아 다음 "새 파일"이 삭제된 폴더를 되살린다 | 2/2 생존 (major) | 선택 상태를 rows 의 함수로 유지(사라지면 클리어) + `targetDirFor` 루트 폴백 |
| 14 | `slots-windows-2` | minor↔major (검증자 이견) | keymap / 보조 창 | `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx:88` | 보조 창에서 ⌘= / ⌘− 가 동작하지 않는다(상태바가 없어 등록처 부재) | 2/2 생존, 등급만 갈림 | font-size 키맵을 두 창 분기 모두에 있는 provider 로 이전 |
| 15 | `focus-keymap-5` | minor | keymap / 보조 창 | `src/widgets/window-chrome/status-bar-content.tsx:137` | (#14 와 동일 사안) 폰트 크기 두 액션의 유일 등록처가 메인 창 전용 상태바 | 1/1 생존 (minor) | #14 과 한 번의 수정으로 동시 해소 |
| 16 | `layout-invariants-2` | minor | layout / kind dedupe | `src-tauri/src/domain/layout/service.rs:481` | 세션 id 가 없는 터미널 탭끼리 kind 동일로 판정돼 터미널을 연속 두 번 열면 하나만 열린다 | 1/1 생존 (minor) | `open_tab` dedupe 에서 `Terminal{session_id:""}` 제외 |
| 17 | `tabbar-dnd-3` | minor | 터미널 탭 생성 | `src/widgets/editor-area/pane-tab-bar.tsx:132` | (#16 과 같은 뿌리) spawn 실패한 터미널 탭이 있으면 그 pane 에서 새 터미널이 영구히 안 열린다 | 1/1 생존 (minor) | #16 과 동일 수정으로 해소 |
| 18 | `focus-keymap-4` | minor | explorer / 보조 창 | `src/widgets/explorer/use-explorer-auto-reveal.ts:55` | 보조 창 autoReveal 이 메인 창 활성 파일을 따라가고, 게이트도 메인 창 사이드바 접힘을 읽는다 | 1/1 생존 (minor) | 창 트리 경유 + `sidebarVisible` 을 호출부 주입으로, 낡은 JSDoc 갱신 |
| 19 | `open-paths-2` | minor | explorer / split | `src/widgets/explorer/explorer-container.tsx:149` | "Open to the Side" 가 포커스 그룹에 탭이 1개뿐인 상황에서 분할 없이 끝난다 | 1/1 생존 (minor) | `useOpenTabInSplit`(`layout_open_tab_in_split`) 단일 mutation 으로 교체 |
| 20 | `slots-windows-3` | minor | shell-slot / 포커스 승계 | `src-tauri/src/domain/project/service.rs:252` | 포커스된 셸 슬롯을 닫으면 포커스가 이웃이 아니라 항상 '첫 슬롯'으로 점프 | 1/1 생존 (minor) | d-65 `successor_leaf_after_prune` 와 동형 헬퍼를 슬롯 층에 추가 |

**중복 관계**: #1↔`persistence-2`, #3↔`persistence-1`, #4↔`focus-keymap-1`, #14↔#15, #16↔#17 은 서로 같은 결함을 다른 관점이 각각 찾아낸 것이다. 수정은 한 지점에서 한 번만 한다.

## 2. 생존 발견 상세

### 1. [layout-invariants-1] `src-tauri/src/domain/layout/service.rs:495` — dirty preview 탭이 in-place 로 파괴된다

- **증상**: preview 탭을 편집해 dirty 로 만든 뒤 다른 파일을 preview 로 열면, 편집 중이던 탭이 사용자 확인 없이 사라진다. `close_tab` 을 거치지 않으므로 `closed_tabs` 에도 남지 않아 ⌘⇧T(Reopen Closed Tab)로 복구할 수 없다.
- **재현**: ① 기본 설정(Preview Tabs 켜짐)에서 탐색기의 파일 A 를 단일 클릭 → A 가 preview 탭으로 열린다. ② A 에 한 글자 입력(⌘S 하지 않음) → dirty 점이 뜨지만 여전히 preview. ③ 곧바로 ⌘P 로 B 를 열거나 탐색기에서 B 를 단일 클릭. ④ A 탭이 사라지고 ⌘⇧T 로도 돌아오지 않는다. (②→③ 을 `HOT_EXIT_MIRROR_DEBOUNCE_MS` 안에 하면 미러 스냅샷조차 없다.)
- **원인**:
  ```rust
  // service.rs:492-497 — open_tab 의 preview 분기
  if preview {
      if let Some(pos) = tabs.iter().position(|existing| existing.preview) {
          tabs[pos] = tab;          // 495: existing.dirty 검사가 없다
      } else { tabs.push(tab); }
  ```
  ```rust
  // service.rs:1052-1056 — set_dirty 는 preview 를 건드리지 않는다
  let tab = find_tab_mut_in_layout(layout, tab_id).ok_or_else(...)?;
  tab.dirty = dirty;
  ```
  프런트 전수(`grep setTabPreview src/`) 결과 `preview:false` 를 내리는 호출부는 `src/widgets/editor-area/pane-tab-bar.tsx:238` 의 `onKeepOpen`(컨텍스트 메뉴 "Keep Open") **한 곳뿐**이다. 편집 경로(`use-editor-file-persistence.ts:234-235`, `299-300` 등)는 `setTabDirty` 만 부른다. 즉 "편집 시작 시 영구 탭으로 승격" 규칙이 구현 자체가 없다 — 그런데 `docs/features/tabs.md:54`("더블 클릭·편집 시작·pin 시 일반 탭으로 승격")와 `docs/research/vscode-behaviors.md:249,265` 가 이미 그 동작을 요구사항으로 명시했다. preview 는 기본 ON(`domain/settings/types.rs:570` `enable_preview_tabs: default_true()`)이고, preview 로 여는 호출부는 ⌘P·탐색기 단일 클릭·문제 패널·검색 결과·브레드크럼 등 다수다.
- **수정안**: 판정을 프런트에 흩지 말고 Rust 한 지점에 둔다.
  ```rust
  tab.dirty = dirty;
  if dirty { tab.preview = false; }   // 편집된 preview 탭은 영구 탭으로 승격 — preview 슬롯을 비운다
  ```
  이러면 495 줄의 교체가 dirty 탭에 닿는 상태 자체가 표현 불가능해진다. 회귀 테스트: (a) dirty preview 탭이 있는 pane 에 preview 로 새 탭을 열면 두 탭이 공존 (b) `set_dirty(true)` 후 그 탭의 `preview == false`.
- **주의점(fixNote)**:
  - 1차 수정 범위는 `set_dirty` 한 줄로 좁힌다. 방어심층으로 제안된 "교체로 밀려나는 clean preview 탭도 `push_closed` 에 기록" 은 별개의 UX 판단이다 — preview 는 본래 훑고 버리는 탭이라 단일 클릭마다 `closed_tabs`(상한 20)에 쌓이면 ⌘⇧T 히스토리가 미리보기로 도배된다. 사용자 확인 후 결정할 것.
  - finding 의 impact 중 "교체 대상이 ClaudeDiff 탭이면 `reconcile_closed_tab` 미해소" 는 현재 코드에서 재현 불가다 — ClaudeDiff 는 `ide-sync-provider.tsx:69` 에서 항상 `preview:false` 로만 열린다.
  - 인접 결함(같은 스펙 문장 위반): `pin_tab`(service.rs:865)도 pin 시 preview 를 해제하지 않는다 → 본 문서 #11 과 같은 커밋에서 함께 처리 가능.

### 2. [stale-ids-1] `src/app/providers/ide-sync-provider.tsx:81` — 보조 창의 더티 파일에 "저장됨" 거짓 응답

- **증상**: 보조 창에서만 열려 편집 중인 파일을 Claude Code 가 `save_document` 로 저장 요청하면, TAIDE 는 아무것도 저장하지 않은 채 성공을 돌려준다. 에이전트는 디스크의 옛 내용을 읽어 작업을 이어간다.
- **재현**: ① `a.ts` 탭을 "Move into New Window" 로 보조 창에 보낸다. ② 보조 창에서 `a.ts` 를 수정(저장 X). ③ 메인 창의 Claude Code 세션에서 그 파일을 저장시킨다(ide `save_document` → `ide:save-requested`). ④ "Document saved successfully" 가 돌아오지만 디스크는 그대로이고 보조 창 탭은 여전히 dirty.
- **원인**:
  ```ts
  // ide-sync-provider.tsx:81-85
  const tab = layout ? findFileTabByPath(layout.root, payload.path) : null   // 메인 트리만
  if (!tab?.dirty) { await resolveIdeSave({ requestId: payload.requestId, saved: true }); return }
  ```
  `move_tab_to_new_window`(`domain/layout/service.rs:1121~`)는 `extract_tab` 으로 소스(메인) 트리에서 탭을 실제로 제거하므로 보조 창 탭은 `layout.auxiliaryWindows[].root` 에만 남는다. `tab` 이 `null` 이면 `!undefined === true` 라 그대로 `saved:true` 로 빠진다. 같은 프로젝트의 다른 호출부(`claude-diff-pane.tsx:26-34`, `pane-tab-bar.tsx:260`, `use-editor-view-state.ts:131`, `tab-path-change.ts:130`)는 전부 메인+aux 를 함께 보는 `collectAllPaneTabs`(`shared/lib/pane-tree.ts:161-164`)를 쓰는데 이 호출부만 단일 트리다. Rust 쪽 `ide/server.rs:241-252` 의 `find_file_tab` 은 `all_roots()` 로 aux 까지 보므로 요청 자체는 정상 emit 된다 — 프런트만 범위가 좁다.
- **수정안**: 로컬 `findFileTabByPath`(26-32행)를 삭제하고 `collectAllPaneTabs(layout).find((t) => t.kind.kind === 'file' && t.kind.path === payload.path) ?? null` 로 교체한다. 백엔드 `set_tab_dirty` 는 `find_tab_mut_in_layout`(id 기반, 트리 무관)이라 tab.id 만 올바르면 후속 `setTabDirty` 도 그대로 동작한다.
- **주의점(fixNote)**: 이 수정은 **거짓 성공 응답을 제거**하지만 "보조 창 파일을 실제로 저장"까지 되지는 않는다. `getModel(payload.path)` 가 보는 `model-registry` 는 모듈 레벨 싱글턴이고 `IdeSyncProvider` 는 메인 창에만 마운트되므로(`docs/architecture.md:576`), 탭이 보조 창에만 있으면 모델이 없어 `saved:false`(model not found) 로 떨어진다. 커밋/PR 설명에 "거짓 성공 제거"와 "보조 창 실저장은 후속 과제(요청 릴레이 또는 미러 기반 저장)"를 구분해 적을 것.

### 3. [stale-ids-2] `src/entities/layout/tab-path-change.ts:123` — 미러·wait 마커를 "다른 pane 에 아직 열림" 판정 전에 해제

- **증상**: 같은 파일이 두 pane 에 열린 상태에서 한쪽 탭만 닫으면, 살아 있는 pane 의 hot-exit 미러가 지워지고 외부 CLI 의 `--wait` 마커도 풀린다. 미저장 편집의 안전망이 조용히 사라지고, `taide --wait` 로 대기 중이던 CLI 가 조기 반환한다.
- **재현**: (A 미러) ① pane 1 에 `a.ts` 를 연다. ② 분할 후 pane 2 에도 `a.ts` 를 연다. ③ 타이핑하고 1초 이상 기다린다(미러 커밋, `pendingMirrorRef=false`). ④ pane 2 의 탭만 ⌘W. ⑤ 저장하지 않은 채 ⌘Q 후 재실행 → `a.ts` 가 디스크 내용으로 돌아와 있다. (B 마커) `taide --wait a.ts` 로 대기 중 같은 파일을 두 pane 에 띄운 뒤 한쪽만 닫으면 CLI 가 즉시 반환한다.
- **원인**: `releaseClosedFileTabPath`(119-139) 본문 순서가 뒤집혀 있다 — 123 의 `takeWaitMarkers` 루프와 125-128 의 `clearMirror` + `invalidateQueries` 가 **무조건** 실행되고, `stillOpenElsewhere` 판정(130-131)은 그 아래에서 `setOpenWithOverride`/`disposeModel` 만 보호한다. 같은 함수의 JSDoc(93-100)은 "the hot-exit mirror … 를 포함해 no tab addresses it any more 일 때만 해제한다" 로 정반대를 적어 구현이 자기 문서를 어긴다.
  같은 파일이 두 pane 에 뜨는 것은 정상 경로다 — `open_tab`(service.rs:481)의 dedupe 는 `find_leaf_mut` 로 얻은 **그 leaf 안에서만** 동작한다. 모델은 `model-registry.ts` 가 경로 키 싱글턴이라 두 pane 이 공유하지만, `pendingMirrorRef`/디바운스는 pane 별이라 이미 커밋된 뒤(=pending false)에는 종료 시 `flushAllMirrors()` 도 아무것도 다시 쓰지 않는다(`use-editor-file-persistence.ts:539-546` `if (!pendingMirrorRef.current) return`). wait 마커도 `agent-wait-marker-registry.ts:59-66` 이 **경로 단위 전량 take** 라 무관한 탭이 먼저 닫혀도 함께 풀린다.
- **수정안**: 123 의 마커 루프와 125-128 의 `clearMirror` 를 `stillOpenElsewhere` 가드(130-131) 아래로 옮긴다. 판정 기준은 이미 있는 `collectAllPaneTabs(layout)`(메인+보조 창) 그대로라 새 헬퍼가 필요 없다.
- **주의점(fixNote)**:
  - 기존 테스트 `tab-path-change.test.ts:327-339`("다른 페인·창에 같은 파일이 남아 있으면 모델을 폐기하지 않는다")가 지금 `expect(recorder.clearedMirrors).toEqual(['/repo/split.ts'])` 로 **버그 동작을 기대값으로 고정**하고 있다. 이 단언을 `[]` 로 뒤집지 않으면 새 회귀 테스트와 서로 모순된 채 공존한다.
  - `invalidateQueries(FILE.MIRRORS)` 만 위에 남겨도 무해하지만, 그러면 스플릿 뷰(흔한 경로)에서 매번 불필요한 재조회가 난다 — 세 줄 전부 가드 아래로 옮기는 편이 깔끔하다.
  - 현재 스위트에 "마커가 실제로 있는 경로 + `stillOpenElsewhere=true` → `releasedMarkers=[]`" 케이스가 없다. 이 케이스를 추가해야 마커 축이 실제로 잠긴다.

### 4. [stale-ids-3] `src/widgets/terminal-pane/terminal-session.tsx:87` — 보조 창 터미널이 cwd 를 잃는다

- **증상**: 보조 창 탐색기에서 "Open in Terminal" 로 연 터미널이 지정한 폴더가 아니라 프로젝트 루트에서 스폰된다. 에러·토스트가 없어 `pwd` 를 치기 전까지 알 수 없다.
- **재현**: ① 아무 탭이나 "Move into New Window" 로 보조 창을 연다. ② 보조 창 탐색기에서 하위 폴더(예: `src/widgets`)를 우클릭 → "Open in Terminal". ③ `pwd` 가 `src/widgets` 가 아니라 프로젝트 루트를 가리킨다. (메인 창에서는 정상)
- **원인**:
  ```ts
  // terminal-session.tsx:87-88
  const activeTabKind = layout ? findPaneTab(layout.root, tabId)?.kind : null   // 메인 트리 고정
  const tabCwd = activeTabKind?.kind === 'terminal' ? (activeTabKind.cwd ?? null) : null
  ```
  탐색기의 `openInTerminal`(`explorer-container.tsx:155-160`)은 `target: null` 로 열고, `withCurrentWindowTarget`(`layout.query.ts:95-98`)이 **그 창의** focusedPane 으로 채우므로 탭은 `layout.auxiliaryWindows[].root` 에만 생긴다(cwd 는 정상 저장됨). `findPaneTab`(`pane-tree.ts:24-30`)은 인자로 받은 노드 하위만 재귀하므로 `null` 을 돌려주고, `pty_default_options`(`domain/terminal/commands.rs:711-724`)는 `cwd: None` 이면 `resolved_cwd = root` 로 떨어진다. `handleGetCwd` 의 폴백 체인(`cwd ?? persistedSession?.cwd ?? tabCwd`)도 `cwd` state 가 이미 잘못된 `defaults.cwd` 로 채워져 구제하지 못한다. 같은 위젯 계열의 `editor-area.tsx:116-117` 은 `resolveWindowPaneTree(layout, getWindowContext())` 를 쓰는데 이 파일만 `layout.root` 고정이다.
- **수정안**: 두 가지 중 하나.
  1. (최소) `resolveWindowPaneTree(layout, getWindowContext())?.root` 에서 `findPaneTab` — 기존 창-인지 컴포넌트의 관례와 일치한다.
  2. (근본) `PaneNodeView`(`pane-node-view.tsx:183-190`)가 이미 올바른 창의 `activeTab.kind`(cwd 포함)를 갖고 있으므로 `TerminalSession` 에 prop 으로 내려 이 컴포넌트가 `layout` 을 재조회하지 않게 한다. 중복 데이터 소스가 사라져 같은 클래스의 재발이 구조적으로 막힌다.
- **주의점(fixNote)**: `collectAllPaneTabs` 전역 탐색도 동작은 하지만, 이 컴포넌트는 자기 창 서브트리 안에서만 마운트되므로 "이 창의 트리" 의미를 정확히 표현하는 1·2 안이 낫다. 회귀 테스트는 보조 창 컨텍스트에서 aux-only 터미널 탭을 렌더해 `ptyDefaultOptions` 인자가 해당 cwd 인지 단언한다.

### 5. [focus-keymap-2] `src/widgets/command-palette/command-palette.tsx:134` — 보조 창 팔레트의 심볼·라인 모드가 메인 창 파일을 본다

- **증상**: 보조 창에서 ⌘P → `@`(Go to Symbol) 또는 `:`(라인) 을 쓰면 그 창이 보여주는 파일이 아니라 메인 창의 활성 파일이 대상이 된다. 항목을 골라도 그 창에서는 아무 일도 일어나지 않는다.
- **재현**: ① 파일 A 를 메인 창에 둔 채 파일 B 탭을 새 창으로 뺀다. ② 보조 창에서 ⌘P → `@` → A 의 심볼 목록이 뜬다. ③ 선택해도 `requestReveal(A경로, …)` 라 그 창에서는 이동이 없다. ④ 메인 창에 파일이 하나도 없으면 보조 창에서 B 를 보고 있는데도 "활성 파일 없음" 이 뜬다.
- **원인**: `const activePath = activeFilePathOf(layout)` — `layout` 은 `layoutQueryOptions(projectId)` 의 `ProjectLayout` 전체다. `ProjectLayout` 은 `{root, focusedPane, …}` 구조라 `WindowPaneTree` 와 구조적으로 호환돼 타입체크는 통과하지만, `activeFilePathOf`(`pane-tree.ts:102-105`)는 그대로 메인 트리를 읽는다. 같은 함수 JSDoc(95-101)이 "보조 창에서도 렌더될 수 있는 위젯은 자기 `resolveWindowPaneTree` 결과를 넘기라" 고 계약을 명시한다. `CommandPalette` 는 d-62 §1.D 이후 보조 창에도 마운트된다(`src/app/app.tsx:77`, 파일 상단 주석 42행도 "mounted in *both* branches as of d-62 §1.D"). 같은 파일 `:205` 의 `currentWindowFocusedPane(layout)` 은 내부에서 창 분기를 거치는데 `activePath` 만 빠져 있다. 소비처: `:291` `documentSymbolsLoaded`, `:316` `palette.noActiveFile`, `:342-343`/`:348-349` `requestReveal`, `:445` 라인 그룹.
- **수정안**: `const activePath = activeFilePathOf(layout ? resolveWindowPaneTree(layout, getWindowContext()) : null)`. 메인 창에서는 값이 동일해 동작 무변경이고, d-65 F1 의 `withExistingFocusedPane` 보정도 함께 적용된다.
- **주의점(fixNote)**: `resolveWindowPaneTree` 의 첫 인자는 non-optional 이므로 `layout` 이 `undefined` 인 초기 로딩을 반드시 널가드로 감싼다(`auxiliary-window-shell.tsx:78` 과 같은 패턴). 더 나은 지점으로는 `pane-tree.ts` 에 `currentWindowFocusedPane` 과 대칭되는 `currentWindowActiveFilePath(layout)` 헬퍼를 추가해 캡슐화하면, 이후 다른 위젯이 aux 에 새로 마운트될 때 같은 실수를 구조적으로 막는다.

### 6. [focus-keymap-3] `src/widgets/outline-panel/outline-panel-container.tsx:31` — 보조 창 아웃라인이 메인 창 심볼을 보여준다

- **증상**: 보조 창 사이드바의 Outline 뷰가 그 창의 활성 파일이 아니라 메인 창 활성 파일의 심볼 트리를 보여준다. 항목을 클릭해도 이동이 없고, 보조 창에서 탭을 바꿔도 아웃라인이 따라오지 않는다. 덤으로 메인 창 파일에 대한 LSP documentSymbol 요청을 중복 유발한다.
- **재현**: ① 메인 창에 A, 보조 창에 B 를 활성으로 둔다. ② 보조 창 사이드바에서 Outline 으로 전환 → A 의 심볼이 뜬다. ③ 클릭하면 A 경로로 reveal 이 나가 보조 창에서는 아무 이동도 없다.
- **원인**: `#5` 와 동일하게 `activeFilePathOf(layout)` 를 raw `ProjectLayout` 으로 호출한다. `OutlinePanelContainer` ← `explorer-panel.tsx:236`(`view === 'outline'`) ← `ExplorerContainer` ← `auxiliary-window-shell.tsx:121`(`<ExplorerContainer projectId={projectId} zen={false} />`, 같은 파일 39행 주석이 "files/search/git/outline switcher 를 함께 들고 온다"고 명시)로 보조 창에도 게이팅 없이 마운트된다. `activePath` 는 `:33` 파일 조회, `:37` `symbolsForPath.path === activePath` 비교, `loadDocumentSymbolsForPath`, 심볼 클릭 시 `requestReveal` 로 이어진다. 같은 클래스가 d-62 에서 이미 한 번 고쳐진 전례가 있다 — `explorer-container.tsx` 의 `openToTheSide` 가 `layout.focusedPane` 을 쓰다가 `resolveWindowPaneTree` 로 교체됐고, 그 자리 JSDoc(`explorer-container.tsx:137`)이 "`ProjectLayout.focusedPane` 는 항상 메인 트리의 pane 을 가리킨다" 고 남아 있다. `outline-panel-container.tsx` 에는 예외를 정당화하는 근거가 전혀 없다(`use-explorer-auto-reveal.ts` 는 의도적 예외를 JSDoc 으로 명시한 반면).
- **수정안**: `const paneTree = layout ? resolveWindowPaneTree(layout, getWindowContext()) : null` 후 `activeFilePathOf(paneTree)`. `resolveWindowPaneTree` 가 슬롯 미발견 시 `null` 을 주므로 기존 "활성 파일 없음" 경로로 안전하게 떨어진다.
- **주의점(fixNote)**: `getWindowContext()` 는 인자 없이 `window.location.search` 만 읽으므로 이 파일 내부 수정만으로 끝나고 상위 prop 변경이 필요 없다. `outline-panel-container.test.tsx` 가 존재하지 않아 이 회귀를 막는 테스트도 없다 — 함께 추가할 것.

### 7. [persistence-3] `src/widgets/editor-pane/editor-pane.tsx:205` — 두 번째 pane 이 공유 모델을 디스크 내용으로 덮는다

- **증상**: 이미 미저장 편집이 있는 파일을 두 번째 pane 에 열면, 두 pane 이 공유하는 monaco 모델이 디스크 내용으로 `setValue` 되어 편집이 통째로 사라진다. dirty 표시는 남아 있어 ⌘S 를 누르면 디스크 내용을 그대로 다시 쓴다.
- **재현**: (결정적 — 루트 밖 파일) ① `taide` CLI 로 프로젝트 루트 **밖** 파일(예: Claude Code Ctrl+G 임시 파일)을 연다 — 이 탭은 미러를 쓰지 않는다. ② 텍스트를 입력(dirty). ③ 분할 후 새 pane 을 포커스한 상태로 ⌘P 로 **같은 파일**을 다시 연다. ④ 두 pane 의 내용이 디스크 원본으로 되돌아가고 입력이 사라진다. (연쇄 — 프로젝트 내부 파일) 두 pane 에 P 를 열고 타이핑 후 1초 대기 → 한쪽만 ⌘W(#3 으로 미러 삭제) → 다시 열면 같은 증상.
- **원인**: 같은 path 의 두 `EditorPane` 은 `getOrCreateModel`(`code-editor.tsx:350`)로 하나의 모델을 공유하지만 `dirty`/`syncedContent` 는 pane 별 state 다.
  ```ts
  // editor-pane.tsx:165, 205-208
  } else if (file && syncedContent === null) { setSyncedContent(file.content) }   // 디스크 내용
  useEffect(() => {
      if (!editor || syncedContent === null || dirty) return   // B 의 dirty 는 false
      queueMicrotask(syncModelOrPickUpExternalEdit)
  }, [editor, syncedContent, dirty, path])
  ```
  `syncModelFromDisk`(`editor-draft-sync.ts:33`)의 두 번째 게이트 `hasUnobservedModelEdit()` 는 `consumeExternallyDirtyModel` 뿐이고, 이 플래그는 `workspace-edit-applier`(LSP WorkspaceEdit)만 세운다(`model-dirty-tracker.ts:9` JSDoc 이 스스로 명시). 따라서 A 의 평범한 타이핑은 통과하지 못하고 `applyDiskContent()` → `applyExternalContent`(`model-registry.ts:202-214`)가 공유 모델을 `setValue` 한다. 이 setValue 는 `modelsApplyingExternalContent` 로 마스킹돼 A·B 양쪽의 `onDidChangeModelContent` 가 무시하므로(`code-editor.tsx:194-198`) A 는 롤백을 인지하지 못한다. 유일한 방어인 미러 복원(`use-editor-file-persistence.ts:519-523`)은 미러 엔트리가 있어야만 발동하는데, (a) 루트 밖 파일은 미러를 아예 안 쓰고(`:238` `if (!isOutsideProjectRoot)`), (b) `HOT_EXIT_MIRROR_DEBOUNCE_MS`(500ms) 이전이거나, (c) #3 으로 미러가 지워진 직후면 없다.
- **수정안**: 새 레지스트리를 만들기보다 `hasUnobservedModelEdit()` 를 일반화한다 — `consumeExternallyDirtyModel(path) || editor.getModel()?.getValue() !== syncedContent`. "모델의 현재 값이 쓰려는 디스크 내용과 이미 다르다"는 사실 자체가 관측되지 않은 변경의 충분조건이므로(출처가 LSP 든 다른 live pane 이든 무관), 별도 카운터 없이 기존 `adoptUnobservedModelEdit` 경로(`setDirty(true)` + `setTabDirty`)로 자연히 빠진다.
- **주의점(fixNote)**: `editor-draft-sync.test.ts` 에 "모델이 disk 와 다르면 (LSP 플래그 없이도) `applyDiskContent` 대신 adopt 한다" 케이스를 추가해야 일반화가 잠긴다. 기존 E2E #24(`docs/quality-assurance/2026-08-18-e2e-harness.md:177`)는 "두 pane 을 먼저 열고 편집"하는 순서라 이 버그(편집이 먼저, 두 번째 pane 오픈이 나중)를 커버하지 않으므로, 순서를 뒤집은 변형 스펙이 필요하다.

### 8. [persistence-4] `src/widgets/editor-pane/use-editor-file-persistence.ts:498` — "복구됨" 배너 오탐 + 미러 시점 롤백

- **증상**: 이미 dirty 인 파일을 두 번째 pane 에 열면 "미저장 내용을 복구했습니다" 배너가 잘못 뜬다(크래시 복구가 아니라 옆 pane 이 편집 중일 뿐). 미러가 최신이 아니면 공유 버퍼가 마지막 미러 시점으로 롤백된다.
- **재현**: ① 파일 P 를 pane A 에서 편집하고 1초 이상 멈춘다(미러 기록). ② 분할 후 새 pane 에서 ⌘P 로 같은 P 를 연다 → 복구 배너가 뜬다. ③ 롤백 변형: A 에서 계속 입력하다가 마지막 미러 기록 후 500ms 안에 ②를 하면 두 pane 내용이 미러 시점으로 되돌아간다.
- **원인**:
  ```ts
  // :498-505 applyMirrorRestore
  applyExternalContent(path, mirror.content, editor)
  draftRef.current = constantDraft(mirror.content)
  setDirty(true); setTabDirty({ tabId, dirty: true })
  setRestoreNotice(mirror.conflict ? 'mirrorRestoredConflict' : 'mirrorRestored')
  // :519-523 복원 effect
  if (!editor || draftRef.current !== null) return   // 새 pane 은 항상 null
  const mirror = (mirrors ?? []).find((entry) => entry.path === path)
  if (mirror) queueMicrotask(() => applyMirrorRestore(mirror))
  ```
  이 판정은 "크래시 후 첫 마운트" 와 "같은 파일을 옆 pane 에 하나 더 열었다" 를 구분하지 못한다. `applyExternalContent` 는 값이 다르면 `setValue` 하므로, 미러가 최대 `HOT_EXIT_MIRROR_DEBOUNCE_MS`(`shared/constants/mirror.ts:1` = 500) 뒤처진 내용이면 그 차이만큼 형제 pane 의 입력이 사라진다. 그 setValue 도 마스킹돼 형제의 `onChange` 에 잡히지 않는다(`code-editor.tsx:196`). 형제의 `draftRef` 는 `() => model.getValue()` 동적 클로저라, 이후 그 pane 에서 ⌘S 를 누르면 롤백된 값이 그대로 디스크에 쓰인다.
- **수정안**: 마운트 시 "이 path 의 모델이 이미 라이브 편집 중인가" 를 가드로 둔다. 새 dirty 레지스트리를 만들지 않고도, `code-editor.tsx` 의 model-attach 지점에서 `getOrCreateModel` 직전에 `getModel(path)` 로 "이미 존재하던 모델을 재사용하는가" 를 판별해 이 훅에 넘기면 된다. 기존 모델이면 `applyMirrorRestore` 대신 `adoptUnobservedModelEdit` 와 같은 형태로 현재 모델 값을 draft 로 채택하고(`setDirty(true)`, `setTabDirty`), `restoreNotice` 는 `'none'` 으로 둔다. 미러는 형제 pane 이 계속 관리하므로 건드리지 않는다.
- **주의점(fixNote)**: finder 의 원 제안은 `#7` 의 "경로 단위 dirty 레지스트리" 재사용인데, 그 레지스트리는 다른 finding 의 산출물이라 단독 적용 시 미해결 의존성이 생긴다. 위의 "이미 존재하던 모델인가" 판별이 자기완결적이다. 테스트: 형제 dirty 상태에서 마운트 시 `applyExternalContent` 미호출 + `restoreNotice === 'none'`.

### 9. [open-paths-1] `src/entities/editor/reveal-registry.ts:43` — reveal 이 다른 pane 에서 먼저 소비된다

- **증상**: 검색 결과·진단·심볼·정의로 이동이 분할 화면에서 목적 줄로 가지 않는다. 같은 파일이 다른 pane 에 열려 있으면 그 pane 이 점프하며 포커스까지 가져가고, 정작 새로 열린 탭은 1행에 머문다.
- **재현**: ① 좌/우 2분할. ② 왼쪽에서 `src/foo.ts` 를 활성 탭으로 둔다. ③ 오른쪽 탭을 클릭해 포커스를 옮긴다. ④ 사이드바 Search 에서 `foo.ts` 120행에만 있는 문자열을 검색하고 매치를 클릭 → 왼쪽이 120행으로 점프하며 포커스를 가져가고, 오른쪽에는 `foo.ts` 탭이 새로 열리지만 커서는 1행이다. 문제 패널 진단, ⌘P 심볼, go to definition, 터미널 `path:line:col` Cmd-클릭, 브레드크럼도 같은 순서라 동일하다.
- **원인**:
  ```ts
  // reveal-registry.ts:19-22, 41-46
  const findEditorForPath = (path: string) =>
      monaco.editor.getEditors().find((e) => e.getModel()?.uri.toString() === toKey(path)) ?? null   // webview 전역
  export const requestReveal = (path, line, column = 1, ttlMs = REVEAL_PENDING_TTL_MS) => {
      const editor = findEditorForPath(path)
      if (editor) { applyReveal(editor, { line, column }); return }   // pending 큐잉 없이 종료
  ```
  모든 호출부가 `requestReveal` 을 **먼저** 부르고 탭을 연다(`search-panel-container.tsx:157-159`, `problems-panel-container.tsx:47-49`, `command-palette.tsx:354-356`, `search-editor-pane.tsx:95-96`, `editor-area.tsx:340-341`, `breadcrumbs-bar.tsx:110-111`). Rust `open_tab`(service.rs:474-481)의 dedupe 는 대상 leaf 안에서만 도므로 다른 pane 에 이미 열려 있어도 새 탭이 생기고, 그 탭은 `view_state: None` 이라 1행에 마운트된다. 새 에디터의 `consumePendingReveal`(`editor-pane.tsx:223`)이 소비할 pending 은 애초에 큐잉되지 않았다. 게다가 워킹트리의 d-65 F2(`pane-node-view.tsx` `onFocusCapture={requestPaneFocus}`)가 `applyReveal` 의 프로그래매틱 `editor.focus()` 까지 캡처해 `focusPane` IPC 를 쏘므로 `focused_pane` 도 함께 흔들린다.
- **수정안**: reveal 대상을 "경로"가 아니라 "이번에 열릴 탭"으로 좁힌다. ① `requestReveal` 의 즉시 `applyReveal` 분기를 없애고 항상 pending 으로 큐잉, ② 탭을 연 쪽(`useOpenFileTab` 의 `onSuccess` 또는 `consumePendingReveal`)이 `entities/editor/editor-instance-registry`(tabId→editor, `getEditorInstance`/`subscribeEditorInstance`)로 **그 tabId 의 에디터**에만 적용한다. 이미 열려 있어 path 가 안 바뀌는 탭은 `[editor, path]` 의존성이라 effect 가 재실행되지 않으므로 소비 키를 `tabId`(+nonce)로 바꾼다.
- **주의점(fixNote)**:
  - dedupe 로 기존 tabId 가 돌아오는 경로(신규 마운트 없음)는 `getEditorInstance(tabId)` 로 즉시 적용, 새 탭이 생기는 경로는 `subscribeEditorInstance(tabId, …)` 로 마운트를 기다렸다 적용 — 두 경로를 모두 처리해야 한다.
  - `useOpenFileTab` 은 현재 `onSuccess` 로 `ProjectLayout` 만 주므로, 방금 연 tabId 를 확정적으로 알아내는 보조 헬퍼가 필요하다.
  - tabId 키로 바뀌어도 `REVEAL_PENDING_TTL_MS` 와 동일한 만료 정리를 유지해야 "오래된 요청이 엉뚱한 시점에 발동" 을 재도입하지 않는다.
  - `setTimeout` 으로 reveal 을 미루는 우회는 금지 — 경합만 좁아질 뿐 다른 pane 이 먼저 잡는 구조가 남는다.
  - d-65 F2 의 `onFocusCapture` 가 프로그램적 focus 와 사용자 클릭을 구분하지 못하는 점은 별도 항목으로 남는다(현재 d-65 계약 §3.4 에 미기재).

### 10. [tabbar-dnd-1] `src-tauri/src/domain/layout/service.rs:899` — 핀 탭 드래그가 raw 탭 순서를 깬다

- **증상**: 고정 탭이 있는 pane 에서 탭을 드래그하면 화면 순서(핀 구역 좌측 고정)와 raw 배열 순서가 어긋난 채 `layout.json` 에 영속된다. ⌃Tab 순환이 화면과 다른 탭으로 가고, "오른쪽 탭 닫기"가 화면상 오른쪽 탭을 남긴다.
- **재현**: ① 한 pane 에 A,B,C 를 열고 A 를 고정 → 화면·raw 모두 `[A(pinned), B, C]`. ② A 를 오른쪽으로 끌어 C 위에 놓는다 → `rawIndex=2`, `pinnedCount=1` → `index=min(2,1)=1` → Rust 는 A 를 extract 한 뒤 `[B,C]` 의 index 1 에 삽입 → raw `[B, A, C]`. 탭 바는 여전히 `[A | B, C]` 로 보여 드래그가 무효처럼 보인다. ③ B 를 활성화하고 ⌃Tab → 화면상 다음인 C 가 아니라 A 로 간다. ④ A 우클릭 → "오른쪽 탭 닫기" → C 만 닫히고 B 가 남는다.
- **원인**: 핀 구역 불변식을 강제하는 곳은 `pin_tab`(service.rs:874-877 `tabs.sort_by_key(|tab| !tab.pinned)`) 하나뿐이고, `move_tab`(899~)은 `extract_tab` 후 `insert_tab(leaf, tab, Some(index))` 만 한다(`insert_tab`:298 `let position = index.unwrap_or(tabs.len()).min(tabs.len()); tabs.insert(position, tab);`). 즉 판정이 프런트 호출부 3곳에 흩어져 있는데 셋 다 틀리거나 비어 있다.
  - `editor-area.tsx:490-494`: `pinnedCount` 를 **추출 전** 배열에서 세므로 같은 pane 안에서 pinned 탭 자신을 옮길 때 자기 자신을 포함한다(Rust 는 추출 후 삽입 → 남은 pinned 는 `pinnedCount-1`) → 항상 한 칸 넘어간다.
  - 같은 파일 475·484행(`tab-container`/`center` 드롭)은 `index: leaf.tabs.length` 로 무조건 맨 뒤(클램프 없음).
  - `editor-area.tsx:196` `moveActiveTabToGroup`(⌘K ⌘⇧←/→)도 `index: adjacent.tabs.length`.
  탭 바는 `pane-tab-bar.tsx:103-104` 에서 `pinnedTabs`/`unpinnedTabs` 를 분리 렌더하므로 화면은 정상으로 보이고, raw 순서를 그대로 쓰는 `cycleTab`(`editor-area.tsx:175-179`)·`handleCloseToRight`(`pane-tab-bar.tsx:143-150`)만 어긋난다. `docs/features/tabs.md:126` 이 "pinned 탭은 pinned 구역(좌측) 안에서만 재정렬 가능" 으로 이 불변식을 명시한다.
- **수정안**: 불변식을 Rust 한 곳으로 옮긴다. `move_tab` 에서 `extract_tab` 이후·`insert_tab` 직전에 대상 leaf 의 `pinned_count` 를 구해 클램프한다 — 이동 탭이 pinned 면 `index.min(pinned_count)`, 아니면 `index.max(pinned_count)`. 그러면 드래그 3경로·⌘K ⌘⇧←/→·향후 신규 호출부가 자동으로 보장된다.
- **주의점(fixNote)**: `open_tab` 의 `tabs.push` 도 같은 규칙을 태우면 pinned 구역 뒤 삽입이 일관된다(pinned:true 로 직접 여는 API 가 생길 때 재발 방지). 프런트 `editor-area.tsx:492-493` 의 클램프는 Rust 가 최종 보장하면 드래그 미리보기 용도로만 남기되, 남긴다면 `activeData.paneId === overData.paneId && activeData.pinned` 일 때 `pinnedCount - 1` 로 보정한다. Rust 테스트: (a) 같은 pane 에서 pinned 를 맨 뒤 index 로 move → pinned 구역 끝 유지 (b) 다른 pane 의 맨 뒤 index 로 pinned 이동 → 대상 pane 의 pinned 구역 끝 (c) unpinned 를 index 0 으로 → pinned 구역 뒤.

### 11. [tabbar-dnd-2] `src-tauri/src/domain/layout/service.rs:865` — 고정한 preview 탭이 다음 단일 클릭에 덮인다

- **증상**: preview 탭을 고정(Pin)해도 `preview` 플래그가 남아, 탐색기에서 다른 파일을 단일 클릭하면 고정해 둔 탭이 사라지고 그 자리에 새 탭이 들어온다(고정도 풀린 채로).
- **재현**: ① 탐색기에서 파일 A 를 단일 클릭 → preview 탭(이탤릭). ② A 탭 우클릭 → 고정. 탭이 pinned 구역으로 가고 핀 아이콘이 붙지만 제목은 여전히 이탤릭. ③ 다른 파일 B 를 단일 클릭 → A 탭이 사라지고 B 가 그 자리에 들어온다.
- **원인**:
  ```rust
  // service.rs:865-878 pin_tab
  if let Some(tab) = tabs.iter_mut().find(|tab| &tab.id == tab_id) { tab.pinned = pinned; }
  tabs.sort_by_key(|tab| !tab.pinned);
  // service.rs:493-497 open_tab 의 preview 교체 — pinned 를 보지 않는다
  if let Some(pos) = tabs.iter().position(|existing| existing.preview) { tabs[pos] = tab; }
  ```
  탭 구조체 전체가 교체되므로 id·pinned·view_state·dirty 가 모두 사라진다. 프런트에서 preview 를 내리는 경로는 `pane-tab-bar.tsx:238` 의 "Keep Open" 하나뿐이고, 핀 토글(`:217`)은 `layout_pin_tab` 만 부른다. `tab-item.tsx:66` 은 pinned 와 무관하게 `preview` 만으로 이탤릭을 그리므로 ②의 증상도 그대로 재현된다. `docs/features/tabs.md` §3 이 "더블 클릭·편집 시작·pin 시 일반 탭으로 승격" 을 스펙으로 규정하는데 pin 축이 미구현이다. ⌘W·휠 클릭·닫기 버튼은 전부 고정 탭을 막아 두었는데 이 경로만 무방비다.
- **수정안**: `pin_tab` 에서 `pinned == true` 로 바꿀 때 `tab.preview = false` 를 함께 설정한다(고정 해제 시 되살리지 않는 단방향 승격 — `convert_untitled_to_file`(:564)이 이미 쓰는 패턴). 방어심층으로 `open_tab` 의 교체 대상 탐색을 `position(|existing| existing.preview && !existing.pinned)` 로 좁힌다.
- **주의점(fixNote)**: 교체된 새 탭은 `pinned=false` 인 채 pinned 구역 위치에 들어가므로 #10 의 raw↔화면 순서 어긋남까지 동시에 만든다 — 두 건을 같은 커밋에서 처리하는 편이 낫다. Rust 테스트: (a) preview 탭 pin → `preview == false` (b) pinned+preview 인 탭이 있는 leaf 에 `open_tab(preview=true)` → 그 탭 유지 + 새 탭 추가. 같은 스펙 문장의 나머지 축("편집 시작 시 승격")은 #1 이 담당한다.

### 12. [fileops-tabs-1] `src/app/providers/ipc-sync-provider.tsx:239` — rescan 후 파일 트리가 영구히 낡는다

> 검증자 이견: 1인은 **생존/major**, 1인은 **기각(이미 기록된 기지 한계)** 로 판정했다. 아래에 양쪽을 모두 적는다.

- **증상**: `git checkout` · `npm install` · sleep/wake 등으로 워처 이벤트가 유실돼 `fs:rescan-required` 가 뜨면, 펼쳐둔 디렉토리가 변경 **이전** 목록 그대로 남고 접었다 펴도 복구되지 않는다. 삭제된 파일 행을 클릭하면 "File not found" 만 뜨고, 새로 생긴 파일은 탐색기에서 접근할 수 없다. 툴바 Refresh 를 눌러야 정상으로 돌아온다.
- **재현**: ① 큰 저장소를 연다. ② `src/` 등을 펼쳐 둔다. ③ 앱 밖 터미널에서 `git checkout <다른 브랜치>` 로 파일 수천 개를 바꿔 FSEvents 큐를 넘긴다. ④ 펼쳐둔 디렉토리가 체크아웃 이전 목록이다. ⑤ 접었다 펴도 그대로. ⑥ 유령 행을 클릭 → 에러 토스트. ⑦ Refresh 로만 복구.
- **원인**: `rescanInvalidations`(238-243)가 `TREE.ROWS(projectId)` 를 **invalidate 만** 한다. 그 invalidate 가 부르는 `tree_rows`(`domain/tree/commands.rs:145-156`)는 `plan_root_read`(`domain/tree/service.rs:137-141`, `if !state.cache.contains_key(&state.root)` 일 때만 디스크 읽음) → `rows_page_from_store`(캐시 히트 시 메모리 스냅샷 재직렬화) 경로라 **디스크를 한 바이트도 다시 읽지 않는다**. `expand`(service.rs:190-199)도 `if !state.cache.contains_key(path)` 라 접었다 펴도 재조회가 없다. 실제로 디스크를 다시 읽는 경로는 `tree_refresh`(=`invalidate`) 하나뿐이다. 그리고 rescan 은 정의상 `fs:changed` 들이 유실됐다는 신호이므로(`infra/watcher.rs:196-204` `notify::Event::need_rescan`, FSEvents `kMustScanSubDirs`) 유실 구간의 경로들은 다시 오지 않는다. 같은 파일 JSDoc(225-227)이 이 사실을 스스로 인정한다.
- **기각 측 근거**: 이 한계는 d-57 에서 이미 식별·triage 됐다 — `docs/acknowledge/2026-09-06-d57-infra-hardening-wave1-contract.md` F4 가 "rescan 이 이미 펼친 디렉토리를 교정하지 못함 | minor | 기록만 — 이미 문서화된 기지 한계, 코드 변경 없음" 으로 확정했고, 완전 교정 경로(Rust 캐시 무효화 또는 FE 의 디렉토리별 `tree_refresh`)를 "이번 계약 범위 밖" 으로 명시했다. `docs/HANDOFF.md:168-171` · `docs/ipc-contract.md:2172-2176` 도 같은 문장을 남겼다. 즉 새 결함이 아니라 열린 백로그다.
- **생존 측 근거**: 그 "별도 계약" 이 d-58~d-65 어디에서도 나오지 않았고, 최근 커밋 `b9ac6f3`(트리 하드 제외 해제로 `node_modules/`·`.next/` 노출)로 사용자가 이 경로에 부딪힐 빈도가 올라갔다. 현재 워킹트리에도 수정이 없다.
- **수정안**: rescan 을 invalidate 가 아니라 실제 재조회로 처리한다. 같은 파일의 `syncTreeRowsForChangedDirs(dirs, { refreshTreeDir, setTreeRows, invalidateTreeRows })`(161-178, `fs:changed` 핸들러가 쓰는 헬퍼)를 그대로 재사용하되, rescan 은 "바뀐 디렉토리 목록" 이 없으므로 `queryClient.getQueryData<TreeRowPage>(QUERY_KEY.TREE.ROWS(projectId))` 로 마지막 캐시를 읽어 `rows.filter(r => r.kind === 'directory' && r.expanded).map(r => r.path)` + 프로젝트 루트를 `dirs` 로 구성해 넘긴다. 이는 툴바 Refresh(`explorer-container.tsx` `refreshVisibleTree`)가 이미 쓰는 검증된 패턴이다. 나머지 세 항목(`SEARCH.PROJECT_FILES`/`GIT.PROJECT`/`FILE.ALL`)은 invalidate 가 정답이므로 그대로 둔다.
- **주의점(fixNote)**: d-57 F4 의 "기록만" 결정을 뒤집는 작업이므로 착수 전 사용자 확인이 필요하다. 새 Rust 커맨드(`tree_invalidate_all`) 신설은 과설계 — 기존 헬퍼 재사용으로 충분하다. JSDoc(225-227)은 코드를 고친 **뒤에** 갱신한다.

### 13. [fileops-tabs-2] `src/widgets/explorer/use-explorer-entry-crud.ts:171` — 삭제된 폴더가 "새 파일"로 되살아난다

- **증상**: 디렉토리를 삭제해도 탐색기 선택이 삭제된 경로를 계속 가리켜, 다음 "새 파일"이 그 경로에 만들어지며 `create_dir_all` 로 폴더가 재생성된다. 입력 행은 트리 맨 위·루트 깊이에 그려져 만들어지는 위치도 화면과 다르다.
- **재현**: ① 탐색기에서 `src/utils` 폴더를 선택한다. ② 우클릭 → Delete → 확인(트리에서 사라짐). ③ 다른 곳을 클릭하지 않고 툴바 "새 파일". ④ 입력 행이 트리 맨 위·루트 깊이에 나타난다. ⑤ `a.ts` 입력 후 Enter. ⑥ 방금 삭제한 `src/utils/` 가 다시 나타나고 그 안에 `a.ts` 가 들어 있다. (행을 바로 우클릭→Delete 해도 동일 — `file-tree.tsx:258-273` 의 `handleContainerContextMenu` 가 우클릭 대상을 먼저 selection 으로 만든다.)
- **원인**: `confirmDelete`(171-181)는 `deleteEntryAsync` → `refreshTreeDir` → `setDeleteTarget(null)` 만 하고 선택을 건드리지 않는다. `selectedRow`(`explorer-container.tsx:62`)를 비우는 유일한 콜백 `onClearSelection`(:209)은 빈 공간 더블클릭(`file-tree.tsx:253-254`)·빈 공간 우클릭(266-267)에서만 호출된다. 이후 `startDraft`(71-78)의 `targetDirFor(selectedRow)`(`explorer-container.tsx:85-88`)가 `row.kind === 'directory'` 이므로 삭제된 경로를 반환하고, `rows.find(row => row.path === targetDir)` 가 undefined 인 것을 알고도 `setDraft({ kind, parentDir: 삭제된경로 })` 를 그대로 실행한다. 표시는 `buildDisplayRows`(`file-tree.tsx:76-83`)가 `targetRow` 미발견 시 `depth=0, insertIndex=0` 으로 그려 더 어긋난다. 커밋 시 `joinPath` → `file_create` → `canonicalize_lenient`(`infra/root_guard.rs:154-177`)가 존재하는 조상까지 거슬러 해석해 루트 검사를 통과시키고, `service::create_entry`(`domain/file/service.rs:194-208`)의 `create_dir_all(parent)` 가 삭제된 디렉토리를 재생성한다.
- **수정안**: 선택 상태를 트리 행 집합의 함수로 유지한다. ① `displayRows` 에 `selectedId` 가 없으면 `setSelectedId(null)` + `onClearSelection()` 로 보정(삭제·외부 변경을 한 곳에서 덮는다). ② 방어선으로 `targetDirFor` 가 `rows` 에 없는 경로를 받으면 `project.root` 로 폴백. ③ `buildDisplayRows` 가 `targetRow` 를 못 찾으면 루트 깊이로 그리지 말고 draft 를 성립 불가로 본다(①②가 들어가면 도달 불가 분기).
- **주의점(fixNote)**:
  - finding 의 "rename/cut-paste 이동에도 적용된다" 확장 주장은 **성립하지 않는다** — `commitRename`(:161)과 `pasteClipboard`(`use-explorer-clipboard.ts:71`)는 둘 다 `setSelectPathRequest(destination)` 을 호출해 `file-tree.tsx:289-295` 이펙트가 새 경로로 재동기화한다. delete 경로에만 이 호출이 빠져 있다.
  - 더 근본적인 대안: `selectedRow` 객체를 state 로 들지 말고 `selectedId` 만 두고 `rows.find(r => r.id === selectedId) ?? null` 로 매 렌더 파생시키면 "존재하지 않는 행을 가리키는 stale 객체" 클래스가 구조적으로 불가능해진다. 현재 컨테이너의 `selectedRow` 와 `FileTree` 내부 `selectedId` 가 이중으로 존재하는 구조가 근본 원인이다.

### 14·15. [slots-windows-2 / focus-keymap-5] 보조 창에서 ⌘= / ⌘− 가 동작하지 않는다

- **증상**: 보조 창(탭을 새 창으로 뺀 창)에서 에디터 글꼴 확대/축소 단축키가 아무 동작도 하지 않는다. 설정값 `editorFontSize` 는 전역이라 메인 창에서 바꾸면 보조 창에도 반영된다 — 조절 권한만 메인 창에 묶인 비대칭이다. 보조 창에는 상태바가 없어 대체 UI 진입점도 없다(설정 화면 경유만 가능).
- **재현**: ① 탭을 "Move into New Window" 로 보조 창에 보낸다. ② 보조 창 에디터에 포커스를 두고 ⌘= / ⌘−. → 변화 없음. ③ 메인 창에서 같은 키 → 변하고, 그 변화가 보조 창에도 적용된다.
- **원인**: `font-size-up`/`font-size-down`(`shared/lib/keymap/keymap.ts:137-138`, `when` 게이트 없음)의 **유일한** 핸들러 등록처가 `status-bar-content.tsx:137` 의 `useGlobalKeymap({ 'font-size-up': …, 'font-size-down': … })` 이고, `StatusBarContent` 는 `app-shell.tsx:33,228`(메인 창)에만 마운트된다. `app.tsx:65-87` 의 보조 창 분기는 `AuxiliaryWindowShell`·`CommandPalette`·`TaskRunnerDialog` 만 렌더하며, `auxiliary-window-shell.tsx:88-92` 의 `useGlobalKeymap` 은 `toggle-sidebar`/`explorer`/`git` 셋만 등록한다(`docs/acknowledge/2026-09-15-d62-project-split-groups-contract.md:84` 의 배선 내역과 일치 — font-size 는 애초에 대상이 아니었다). `use-global-keymap.ts` 의 dispatch 는 `const handler = handlers[action.entryId]; if (!handler) return` 이라 `preventDefault` 도 없이 조용히 버려진다.
- **선례**: 같은 클래스가 이미 한 번 고쳐졌다 — `docs/features/keymap.md` §5(214-220)가 d-51 F4 사례로 "`open-keybindings-editor`(⌘K ⌘S) 핸들러가 `AppShell` 에만 있어 보조 창에서는 chord 만 해소되고 아무 일도 일어나지 않았다" 를 기록하고, 해법으로 핸들러를 두 창 분기 모두에 마운트되는 `KeybindingsRuntimeProvider` 로 옮겼다. font-size 두 액션만 그 이전에서 누락됐다.
- **수정안**: `increaseEditorFontSize`/`decreaseEditorFontSize`(+`clampFontSize`, `CODE_FONT_SIZE_STEP`) 로직을 공용 훅으로 한 번만 추출하고, 등록은 `KeybindingsRuntimeProvider`(이미 `app.tsx` 두 분기 모두에 마운트, `useSettingsQuery` 도 보유)에서 한다. `status-bar-content.tsx` 의 버튼(`onEditorFontSizeIncrease`/`Decrease`, :166-167)은 같은 훅을 재사용하고, 그 파일의 `useGlobalKeymap` 등록에서는 두 엔트리를 제거해 메인 창 이중 등록을 피한다.
- **주의점(fixNote)**: 보조 창에 `useGlobalKeymap` 을 한 벌 더 복제하는 방식은 등록 판정이 3곳으로 늘어나므로 피한다. severity 는 검증자 둘이 minor(설정 화면·메인 창 우회 존재)와 major(보조 창을 주 편집 화면으로 쓰는 경우 진입점 부재)로 갈렸다 — 수정 비용이 작으므로 등급 논쟁 없이 처리 가능하다.

### 16·17. [layout-invariants-2 / tabbar-dnd-3] 세션 id 가 없는 터미널 탭이 kind 동등으로 dedupe 된다

- **증상**: (A 타이밍) 터미널 추가 버튼·"New Terminal" 을 빠르게 두 번 누르면 두 번째 탭이 생기지 않고 첫 탭만 다시 활성화된다. (B 영구) 셸 경로가 잘못돼 spawn 이 실패하면 그 탭의 `session_id` 가 영구히 `""` 로 남아, 설정을 고친 뒤에도 그 pane 에서는 탭을 직접 닫기 전까지 새 터미널을 열 수 없다.
- **재현**: A) 탭 바 `+` → "새 터미널" 을 연속 두 번 빠르게 클릭(첫 pty spawn 왕복 전). B) 설정의 셸 경로를 없는 실행 파일로 지정 → "새 터미널" → 실패 화면으로 남는 탭 → 이후 몇 번을 눌러도 그 탭만 활성화된다.
- **원인**: 터미널 탭은 항상 `session_id: ""` 로 생성되고(`editor-area.tsx:236,263`, `pane-tab-bar.tsx:132`, `terminal-session.tsx:248`, `explorer-container.tsx:159`, `layout.query.ts:150`), PTY spawn 이 끝난 뒤에야 `set_terminal_session`(service.rs:1059)이 실제 id 를 채운다. 그런데 `open_tab`(service.rs:481)의 dedupe 는 `existing.kind == tab.kind` 전체 동등이라 `Terminal{session_id:"", cwd:None}` 끼리 참이 된다. 같은 파일이 이 함정을 이미 알고 있다 — `open_tab_in_split`(service.rs:1000 부근) doc 이 "Unlike `open_tab` there is no kind-equality dedupe — … two terminal tabs that have not been given their session id yet compare equal" 이라고 적었는데, 그 대비가 split 경로에만 있다. `handleSpawnFailure` 는 `settleSpawnedSession` 을 호출하지 않으므로 B 는 영구다. 이 결함은 `docs/research/2026-09-04-batch4-terminal-tabbar-context-menu-research.md:272,474` · `docs/features/terminal.md:369` · `docs/ipc-contract.md:2090` 에 이미 세 번 기록됐으나 수정·기각 어느 쪽으로도 닫히지 않았다.
- **수정안**: "아직 배정되지 않은 session id" 는 정체성이 아니므로 dedupe 대상에서 제외한다.
  ```rust
  fn is_dedupable(kind: &TabKind) -> bool {
      !matches!(kind, TabKind::Terminal { session_id, .. } if session_id.is_empty())
  }
  ```
  481 줄의 `find` 를 이 가드로 감싼다. 프런트에서 임시 session id 를 미리 채우는 우회는 `set_terminal_session` 계약을 흐리므로 쓰지 않는다.
- **주의점(fixNote)**: 매치 가드는 `!matches!(tab.kind, TabKind::Terminal{..}) && existing.kind == tab.kind` 형태로 좁혀 File/Settings/Welcome/Diff/Untitled 의 기존 dedupe 와 `next_untitled_index`·preview 치환 로직에 영향이 없게 한다. 탐색기의 "폴더에서 터미널 열기"(`explorer-container.tsx:159`, `cwd: dir`)도 같은 경로를 타므로 같은 dir 을 연속 두 번 여는 케이스가 함께 해소되는지 확인한다. 회귀 테스트: `세션id가_빈_터미널_탭은_중복_제거되지_않는다`(기존 `새_탭_분할은_같은_kind_터미널_탭을_중복_제거하지_않는다`(service.rs:1910)와 짝) + "이미 session_id 가 채워진 터미널 2개는 여전히 dedupe" 보존 테스트. 관련 문서 3곳의 서술도 함께 갱신한다. 실패 화면에 재시도 버튼이 있다는 finder 서술은 사실과 다르므로(정적 에러 메시지만, `terminal-session.tsx:355-357`) 커밋 설명에 쓰지 않는다.

### 18. [focus-keymap-4] `src/widgets/explorer/use-explorer-auto-reveal.ts:55` — 보조 창 autoReveal 이 메인 창을 따라간다

- **증상**: 보조 창의 "활성 파일 따라가기"가 항상 다른 창을 따라간다. 보조 창에서 탭을 바꿔도 그 창 트리는 움직이지 않고, 메인 창이 파일을 바꾸면 보조 창 트리가 점프한다. 메인 창 사이드바를 ⌘B 로 접으면 보조 창은 펼쳐져 있는데도 autoReveal 이 통째로 멈춘다.
- **재현**: ① 메인 창에 A, 보조 창에 B. ② 보조 창에서 탭을 C 로 바꾼다 → 보조 창 트리는 C 를 선택하지 않고, 메인 창이 A→D 로 바뀌면 보조 창 트리가 D 로 점프. ③ 메인 창 사이드바를 ⌘B 로 접는다 → 보조 창 autoReveal 정지.
- **원인**: `const activePath = activeFilePathOf(layout)`(메인 트리)과 `:60` `const sidebarVisible = !zen && !(layout?.shellView?.sidebarCollapsed ?? false)`. `shellView.sidebarCollapsed` 는 `project-shell.tsx:57,64`(메인 창 `persistSidebarCollapsed`)만 쓰고 보조 창은 자기 `explorerPanelRef` 로 접힘을 따로 관리한다. 같은 파일 JSDoc(29-32)이 근거를 "the sidebar is mounted in the main window alone" 으로 적었는데, 이 전제는 d-62 §1.D 이후 거짓이다 — `auxiliary-window-shell.tsx:121` 이 같은 `ExplorerContainer` 를 보조 창에 마운트하고 그 안에서 이 훅이 호출된다(`explorer-container.tsx:127`). 즉 의도적 결정이 아니라 전제가 낡은 것이다. 같은 파일의 `openToTheSide` 는 d-62 F2 에서 이미 `resolveWindowPaneTree` 로 교체됐다.
- **수정안**: `activeFilePathOf(layout ? resolveWindowPaneTree(layout, getWindowContext()) : null)` 로 바꾸고, `sidebarVisible` 은 `layout.shellView` 대신 호출부가 주입한다 — `ExplorerContainer` 가 `zen` 을 넘기듯 `sidebarVisible` 을 prop 으로 넘기는 형태. 낡은 JSDoc(29-32)과 `docs/features/explorer-sidebar.md:77`("사이드바는 주창에만 마운트되므로 `resolveWindowPaneTree` 금지")도 새 근거로 교체한다.
- **주의점(fixNote)**: `sidebarVisible` 은 "호출부가 이미 아는 값" 이 아니다 — `project-shell.tsx` 와 `auxiliary-window-shell.tsx` 둘 다 `explorerPanelRef.current?.isCollapsed()` 를 콜백 안에서 명령형으로만 읽는다. 두 셸에 `isCollapsed` state(panel `onLayout`/토글 핸들러에서 갱신)를 새로 만들어 d-62 F2 가 `zen` 에 했던 것과 같은 관통 배선을 한 축 더 해야 한다. 또한 이 수정 없이는 두 창이 각자 `lastRevealedRef` 를 갖고 같은 메인 창 경로에 대해 `tree_reveal` 을 중복 발행한다.

### 19. [open-paths-2] `src/widgets/explorer/explorer-container.tsx:149` — "Open to the Side" 가 분할 없이 끝난다

- **증상**: 포커스 그룹이 그 파일 하나만 갖게 되는 상황(에디터가 비었거나 그 파일 탭 하나만 열림)에서 "Open to the Side" 를 하면 파일은 열리지만 화면이 분할되지 않는다. 앱을 막 켜 Welcome 만 있는 상태에서 처음 쓰는 경로라 첫인상에 걸린다.
- **재현**: ① 모든 탭을 닫아 에디터 영역을 비운다. ② 탐색기에서 아무 파일 우클릭 → "Open to the Side". → 파일은 열리지만 분할되지 않는다. (포커스 그룹에 그 파일 탭 하나만 있을 때 같은 파일을 다시 "Open to the Side" 해도 동일 — `open_tab` 이 중복 탭을 안 만들어 탭 수가 1로 유지된다.)
- **원인**: `openToTheSide` 는 "포커스 pane 에 연 다음 그 탭을 오른쪽으로 분할해 내보낸다" 는 2단계다(`:138-152`). 2단계의 Rust `split`(service.rs:982-996)은 `extract_tab` 으로 소스 pane 을 비운 뒤, `source_tree == target_tree` 라 `normalize` 를 건너뛰고 `insert_new_leaf`(951-975)로 넘긴다. 그 말미의 `normalize(tree_root_mut(...))`(972)가 `normalize_owned`(371-421)에서 빈 leaf 를 `continue` 로 버리고(376 부근), 남은 자식이 1개면 그 자식을 그대로 반환(417-421)해 방금 만든 Split 이 다시 단일 Leaf 로 접힌다. 완전 붕괴는 target leaf 가 그 창의 유일한 leaf(트리 root)이고 그 leaf 의 유일한 탭을 내보낼 때다. `move_tab`(888-914)은 같은 클래스의 엣지케이스를 주석으로 명시하고 추출-재삽입을 한 leaf 안에서 끝내 방지하는데, `split` 은 새 형제 leaf 로 넣으므로 그 보호가 없다.
- **수정안**: 2-mutation 조합 대신 **처음부터 새 pane 에 여는 단일 mutation** 으로 바꾼다. 같은 문제(소스 pane 활성화 → 언마운트 → 재스폰)를 위해 만들어 둔 `layout_open_tab_in_split`(`commands.rs:192`, 서비스 `open_tab_in_split`, 프런트 `useOpenTabInSplit`)이 정확히 이 용도이며 터미널 컨텍스트 메뉴의 "Split" 이 이미 쓴다. `openTabInSplit({ projectId, targetPane: 현재 창의 focusedPane, edge: 'right', kind: { kind: 'file', path }, title, preview: false })` 로 교체하면 빈 소스 pane 자체가 생기지 않아 normalize 접힘이 원천적으로 없고 `layout:changed` 도 1회가 된다. 파일 존재 선검증도 동일하게 돈다(`commands.rs:188-189`).
- **주의점(fixNote)**: `open_tab_in_split` 은 문서화된 대로 kind dedupe 를 하지 않으므로, 수정 후에는 이미 열린 파일에 "Open to the Side" 를 다시 하면 같은 파일의 새 탭이 실제로 하나 더 생겨 분할된다. "Open to the Side 는 항상 분할한다" 는 의미에서 더 올바른 시맨틱이지만 의도된 동작 변경이므로 커밋/PR 설명에 명시한다. 기존 테스트 `기존_split_은_추출_이후에도_탭을_원래_페인에서_옮긴다`(1928-1953)는 `default_layout()` 이 탭 2개를 미리 채워 이 엣지케이스를 커버하지 못한다.

### 20. [slots-windows-3] `src-tauri/src/domain/project/service.rs:252` — 셸 슬롯 포커스가 이웃이 아니라 첫 슬롯으로 점프

- **증상**: 3개 이상 분할한 창에서 포커스된 슬롯을 닫으면 포커스가 바로 옆 슬롯이 아니라 항상 맨 첫 슬롯으로 간다. 상태바·타이틀바·⌘P·⌘B 대상이 전부 그 프로젝트로 바뀌고, 슬롯 포커스가 `active_project` 를 재정의하므로 네이티브 File 메뉴·Welcome 최근 순서까지 따라 바뀐다.
- **재현**: ① 프로젝트 3개를 연다. ② 레일에서 두 번째를 첫 슬롯 오른쪽 가장자리로 끌어 분할(`[A | B]`). ③ 세 번째를 B 의 오른쪽 가장자리로 끌어 분할(`[A | B | C]`, 포커스 C). ④ C 의 헤더 닫기 버튼으로 C 를 닫는다. ⑤ 포커스가 B 가 아니라 A 로 간다. (슬롯이 2개면 남는 슬롯이 하나라 증상이 안 보인다.)
- **원인**: `close_shell_slot`(239-257)은 `remove_slot` 후 `reconcile_focus(session)`(252) 하나로 승계를 끝낸다. `reconcile_focus`(68-90)의 선호 순서는 ① 살아 있는 `focused_shell_slot` 유지 → ② `active_project` 가 속한 슬롯 → ③ `first_slot(tree)`(86, `leaves()[0]`). 포커스 슬롯을 닫으면 ①은 사라진 id 라 탈락하고 ②도 `slot_of_project` 가 `None` 이라 항상 ③으로 간다. 슬롯 분할은 항상 이진 중첩이므로 `[A|B|C]` 는 `Split(A, Split(B,C))` 이고, C 를 닫으면 남은 `Split(A,B)` 의 첫 리프 = A 가 포커스된다. 같은 문제를 한 층 아래 pane 트리에서는 방금 고쳤다 — d-65 §1 R1-1 이 `successor_leaf_after_prune`(이전 형제의 마지막 리프, 없으면 다음 형제의 첫 리프)을 도입하고 테스트로 고정했다. 슬롯 층에는 그 승계가 없고 `reconcile_focus` 주석도 "lands somewhere deterministic" 으로 결정성만 보장한다.
- **수정안**: `shell_slots.rs` 에 pane 층과 동형의 순수 헬퍼(`successor_slot_after_prune(tree, slot_id) -> Option<ShellSlotId>`)를 추가하고, `close_shell_slot` 이 `remove_slot` **전에** 후보를 계산해 ⓐ 닫는 슬롯이 그 순간의 `focused_shell_slot` 일 때만 ⓑ prune 후 후보가 살아 있으면 `session.focused_shell_slot = Some(후보)` 로 세팅한 뒤 `reconcile_focus` 를 부른다(그러면 ①분기가 받아 `active_project` 도 함께 재유도된다). 비포커스 슬롯을 닫을 때는 종전대로 포커스 불변.
- **주의점(fixNote)**: 셸 슬롯 트리는 항상 이진 분할이라 pane 의 N-ary 형제 순회보다 단순하다 — 닫히는 리프를 담은 Split 의 "다른 쪽 자식" 이 유일한 형제이므로 리프면 그대로, 서브트리면 그 첫/마지막 리프로 내려가면 된다. 더 중요한 것은 `close_project`(495-509)도 `remove_project` + `normalize_shell_slots` → 같은 `reconcile_focus` 폴백을 타 동일 증상을 낸다는 점이다(실사용 빈도가 더 높다). `close_shell_slot` 만 고치면 그 경로가 남으므로, `reconcile_focus` 가 호출 전에 계산된 승계 후보를 선택적으로 받도록 만들어 두 경로가 헬퍼를 공유하게 하는 편이 근본적이다. 테스트는 d-65 R1 의 (a)~(d) 와 같은 형태로 슬롯판을 추가한다.

## 3. 반박으로 기각된 발견 (재론 금지 목록)

아래 3건은 검증에서 기각됐다. 같은 지적이 다시 올라오면 이 표의 근거를 먼저 확인하고, 뒤집으려면 새 근거(사용자 보고·기존 결정 변경)를 제시한 뒤 사용자 확인을 받는다.

| id | 제목 | 기각 근거 |
|---|---|---|
| `stale-ids-4` | `layout:changed` 무효화로 시작된 `getLayout` 재조회 응답에 revision 가드가 없어 더 새 레이아웃을 되돌릴 수 있다 (`src/app/providers/ipc-sync-provider.tsx:316`) | 메커니즘 서술 자체는 소스와 일치(`layoutQueryOptions.queryFn` 은 `isStaleLayoutRevision` 게이트를 안 탄다). 그러나 **이미 검토·확정된 항목**이다 — `docs/acknowledge/2026-08-25-d42-e2e-defects-contract.md` §4.2 판정표 **L1-07** 이 "재조회(refetch) 경로는 revision 무가드 — 단 재유입 창이 극협·자가 치유 | 기록만 … 조회 경로 가드는 TanStack 계약 비틀기라 배제" 로 닫았고, finding 의 1안(`queryFn` 을 revision 인지로)이 바로 그 배제된 방향이다. 등급 info. (참고: 2안 — 이벤트 핸들러에서 `invalidateQueries` 대신 `fetchQuery` 결과를 `applyFreshLayout` 에 흘리기 — 는 `queryFn` 을 건드리지 않아 L1-07 이 배제한 지점과 다르므로, 재론이 필요해지면 이쪽이 후보다.) |
| `open-paths-3` | 프로젝트 루트 밖을 가리키는 심링크 파일은 어느 진입점으로도 열리지 않고 Forbidden 만 뜬다 (`src-tauri/src/infra/root_guard.rs:155`) | 동작 서술은 정확하지만 **d-58 에서 이미 식별·결정된 의도적 경계**다 — `docs/acknowledge/2026-09-15-d58-usability-batch5-wave1-contract.md` 검토 항목 **B-3** 가 같은 시나리오를 minor 로 적시하고, 채택 해법은 동작 변경이 아니라 "`command-palette.md` §3 에 비대칭 명시 + 회귀 테스트" 였다. 둘 다 이미 구현돼 있다(`docs/features/command-palette.md:117-122`, `root_guard.rs:274-296` 의 `루트_밖을_가리키는_심링크는_루트_안에_있어도_거부된다` 테스트가 "나중에 링크를 따라가도록 넓히는 변경" 을 명시적으로 pin). 부가 주장도 과장 — 토스트는 리터럴 `Forbidden` 이 아니라 `error.path.outsideProjectRoot`("경로가 프로젝트 루트 밖에 있습니다: {path}")다. `b9ac6f3`(트리 하드 제외 해제)로 노출 빈도가 는 것은 사실이나 새 결함을 만들지는 않았다. |
| `fileops-tabs-3` | 다른 창에서 편집 중인 파일을 이름 변경하면 그 창의 미저장 편집이 화면에서 사라진다 (`src/entities/file/file.query.ts:178`) | 인과 사슬은 소스와 일치하지만 **d-50 에서 확증 → 절반 수정 → 잔여분 백로그 이관까지 끝난 항목**이다. `docs/acknowledge/2026-08-29-d50-audit-rust-batch-contract.md` §4 판정표 #2 가 이 시나리오를 "확증" 으로 기록하고, 그 조치로 지금 코드에 있는 `useRenameEntry.onMutate`(`file.query.ts:174-177`, 개명 **전** `FILE.MIRRORS` 강제 재조회)를 신설해 치명적 절반(재시작 복구조차 안 되던 것)을 고쳤다. 남은 절반(라이브 세션 중 화면 버퍼만 디스크 내용으로 리셋, 재시작하면 복구됨)은 같은 문서와 `docs/features/tabs.md` §7.1(d-50 S8), `docs/backlog.md:141`("개명 시 다른 창의 라이브 버퍼 이관")에 창 간 이관 브로드캐스트가 필요한 백로그로 명시됐다. 등급 minor. |

## 4. info

| id | 제목 | 내용 |
|---|---|---|
| `persistence-5` | untitled 탭에서 ⌘S 가 아무 동작도 하지 않는다 (`src/widgets/editor-area/focused-editor-tab.ts:19`) | **기지 결함 재확인**. `const SAVE_ROUTABLE_TAB_KINDS: ReadonlySet<TabKind['kind']> = new Set(['file', 'appFile'])` 에 `untitled` 이 없어 `resolveSaveRoutableTabId`(`editor-area.tsx:208`)가 `null` 을 주고 `saveActiveTab`(:216)이 no-op 된다. 근본 원인은 `untitled-pane.tsx:194-210` 의 `<CodeEditor>` 가 `registryTabId` 를 넘기지 않아 `editor-instance-registry` 에 인스턴스가 등록되지 않는 것이고(`code-editor.tsx:435-445` 의 등록 effect 가 `if (!registryTabId) return` 로 빠짐), 그 결과 `untitled-pane.tsx:206` 의 `onSave={() => void handleSaveAs()}` 는 도달 불가능한 dead code 다. `focused-editor-tab.ts` JSDoc 이 이미 같은 내용을 기록했고 d-65 계약이 "범위 외·별건" 으로 분류했다. 수정한다면 `registryTabId={tabId}` 추가 + `SAVE_ROUTABLE_TAB_KINDS` 에 `'untitled'` 추가(`appFile` 을 고친 것과 같은 방식). 내용 자체는 untitled 미러로 보존되므로 소실은 아니다. |

## 5. finder 별 미조사 범위 (coverage 원문 요약)

> 아래는 각 finder 가 스스로 보고한 "읽은 범위 / 못 본 범위" 다. 이 조사에서 **다루지 못한 축**을 후속 작업이 중복 없이 이어받기 위한 기록이다. 공통 사항: 8관점 모두 **테스트를 실행하지 않았고 앱 실기 확인도 하지 않았다**(읽기 전용 지시 + 시간 상한).

### layout-invariants
- 읽음: `domain/layout/service.rs` 구현 전 구간(1~1490), `types.rs` 전문, `commands.rs` 1~200·300~525, `capability.rs`, `domain/window/service.rs` 55~210, `project/commands.rs::project_close`, d-65·d-63 계약 전문.
- 못 봄: `service.rs` 테스트 모듈 본문(1491~3428), `commands.rs` 200~300·525~705, `layout.query.ts` 전문, `pane-tree.ts` 전문, **셸 슬롯 닫기 경로**, `migrate_layout` 본문(1313~1345), 핫엑싯 미러 키 체계.
- 미확정으로 남긴 것: 손상된 `layout.json` 이 들어올 때 `normalize_owned` 의 `children.into_iter().zip(sizes)` 가 조용히 자식을 자르는지(=`migrate_layout` 의 길이 검증 유무) 미확인. 이벤트 emit 순서 vs `isStaleLayoutRevision` 의 프런트 적용 경로(`applyFreshLayout`) 미독.
- finding 으로 올리지 않은 관찰: `cleanup_emptied_auxiliary_windows` 가 move 경로에만 있는 비대칭(프런트가 빈 트리 감지로 보완), `retarget_file_tabs` 가 closed_tabs 만 바뀔 때 revision 을 안 올리는 것(사용자 가시 효과 없음), aux pane 에서 닫은 탭을 main 에서 ⌘⇧T 하면 aux 로 복원되는 것(설계 판단 영역).

### stale-ids
- 읽음: 프런트 전문 — `layout.query.ts`, `layout.ipc.ts`, `tab-path-change.ts`, `pane-tree.ts`, `layout-revision.ts`, `window-context.ts`, `query-client.ts`, `ide-sync-provider.tsx`(1~140), `hot-exit-flush-provider.tsx`, `claude-diff-registry.ts`, `agent-wait-marker-registry.ts`, `mirror-flush-registry.ts`, `auxiliary-window-shell.tsx`. grep 전수: `useCloseTab`·`useOpenFileTab`/`useOpenTabInProject`·`layout.root` 직접 참조·`commands.{layout|session|window|file}` 직접 호출 전건.
- 못 봄: `docs/features/*.md` 와 `docs/ipc-contract.md` 미독(발견이 문서상 의도된 동작으로 이미 기록됐을 가능성은 배제 못 함 — 단 생존 건들은 코드 자체 JSDoc 과 모순). `widgets/{command-palette,git-panel,search-panel,diff-pane,welcome,problems-panel,app-shell,window-chrome}` 본문 미통독, `features/*`·`shared/lib/{shell-slot,keymap,bridge,monaco}`·`shell-slot-provider.tsx` 전부 미독. Rust `domain/{file,project,session,search,tree}` 미독 → **session/window 축의 stale id 위험은 사실상 미조사**.
- 제외 판단: 보조 창의 `windowSlot` 이 프로젝트 스코프라 `resolveWindowPaneTree` 가 `windowContext.projectId` 를 무시하는 점은 잠재 위험이나 도달 호출부를 못 찾아 제외. `pane-tab-bar` 의 `void handle…()` close 루프 unhandled rejection 은 d-63 §7 과 같은 클래스라 재론 안 함.

### focus-keymap
- 읽음: `editor-area.tsx`(120~420), `pane-node-view.tsx`, `focused-editor-tab.ts`, `use-global-keymap.ts`, `use-keydown-capture.ts`, `keymap-dispatch.ts`, `keymap-chord-store.ts`(1~240), `keymap-context.ts`, `pane-tree.ts`, `shell-slot-provider.tsx`, `app.tsx`(55~115), `command-palette.tsx`(1~300), `command-registry.ts`/`command-catalog.ts`(1~80), `status-bar-content.tsx`, `auxiliary-window-shell.tsx`(60~140), `terminal-pane.tsx`(150~230), `terminal-session.tsx`(60~130), `use-explorer-auto-reveal.ts`, `explorer-container.tsx`(125~175), `outline-panel-container.tsx`(1~45), `code-editor.tsx`(320~370), `layout.query.ts`(60~170), `ide-sync-provider.tsx`(40~120). Rust 는 `pty_default_options` 만 실물 확인.
- 못 봄: Rust `domain/layout/service.rs`·`window`·`ide`·`search`·`tree` 실물 미통독(d-65 계약 기술로 갈음). `widgets/{search-panel,git-panel,diff-pane,claude-diff-pane,welcome,task-runner}` 내부 미통독. `keymap-when.ts`·`command-binding-dispatch.ts`·`monaco-keybinding.ts` 내부 미독. `entities/*/*.commands.ts` 6파일 grep 만.
- 미확정: "포커스 루프/IPC 폭주" 는 `code-editor.tsx` 의 `isPathChange` 가드와 `useKeydownCapture` 의 `focusin` deferral 해제까지 확인해 **재현 경로를 찾지 못했고**, 백그라운드 셸 슬롯의 `autoFocus` 발화 트리거를 실측하지 못해 미확정으로 남긴다. chord/`defer-to-monaco` 잔류는 창내 포커스 이동 시 해제 + 5s 타임아웃 + 상태바 표시가 있어 결함 아님으로 판정.

### persistence
- 읽음(전문): `use-editor-file-persistence.ts`, `editor-pane.tsx`, `code-editor.tsx`, `untitled-pane.tsx`, `editor-draft-sync.ts`, `entities/editor/{model-registry,mirror-flush-registry,file-save-settle-registry,untitled-registry}.ts`, `hot-exit-flush-provider.tsx`, `on-save-cleanup.ts`, `focused-editor-tab.ts`. 부분: `tab-path-change.ts`(90~270), `file.query.ts`(60~200), `layout.query.ts`(190~270), `pane-tab-bar.tsx`(205~300), `editor-area.tsx`(200~270·400~440), Rust `layout/service.rs`(470~660·870~900·1040~1070·1195~1210), `file/service.rs`(355~410).
- 못 봄: `use-editor-lsp-integration.ts`·`workspace-edit-applier.ts`·`model-dirty-tracker.ts` 미통독 → **"LSP workspace-edit 가 dirty/버전을 어긋나게 하는 경로" 는 미조사**. `use-editor-git-gutter-and-conflicts.ts` 미독. **저장 중 프로젝트 전환 / 셸 슬롯 닫기 / 창 닫기 경로 미조사**(이 축의 절반이 공백). `convert_untitled_to_file`(Rust) 본문 미확인.
- 미확정: 크로스 윈도우(main ↔ aux) — `file-save-settle-registry`·`model-registry` 가 창별 모듈 인스턴스라 한쪽 창의 저장이 다른 창 pane 의 `dirty`/`syncedContent` 를 settle 하지 못하는 정황은 봤으나, aux 창의 `layout:changed`/쿼리 무효화 브로드캐스트 유무를 확인하지 못해 finding 으로 올리지 않았다.

### open-paths
- 읽음: 전수 grep(`useOpenFileTab|useOpenTabInProject|useOpenTab(|useOpenAppFileTab|useOpenTerminalTab|openTabInSplit`) 후 `layout.query.ts`·`pane-tree.ts`·`reveal-registry.ts`·`fire-and-forget-bridge.ts`·`editor-opener-bridge.ts`·`shell-slot-context.tsx` 전문, `shell-slot-provider.tsx`(60~110), `agent-external-open-provider.tsx`(1~100), `explorer-container.tsx`(85~305), `command-palette.tsx`(100~230·325~360), `search-panel-container.tsx`(120~185), `search-editor-pane.tsx`(60~110), `problems-panel-container.tsx`, `breadcrumbs-bar.tsx`, `editor-pane.tsx`(195~245), `git-panel`·`welcome`·`app-shell`·`terminal-session`·`editor-area` 해당 구간. Rust: `layout/service.rs`(355~620·930~1030), `layout/commands.rs`(28~205), `project/{service.rs:408~440, commands.rs:135~225}`, `root_guard.rs`(24~177).
- 못 봄: `features/tab`·`features/split`·`widgets/diff-pane`·`widgets/outline`·`features/snippet`·`file-history-panel` 본문 미통독(grep 으로 open 호출 부재만 확인). `terminal-file-link.ts` 의 후보 해소(cwd·상대경로·심링크) 본문 미독. `useExplorerEntryCrud`·`use-explorer-auto-reveal` 본문 미독(파일 생성 후 자동 열기 경로).
- 미조사: 워처 에코 ↔ quick-open 인덱스 무효화 경합은 `useOpenFileTab` 의 `isNotFoundIpcError → invalidateQueries(SEARCH.PROJECT_FILES)` 한 곳만 확인했고 tree/워처 이벤트 타이밍은 못 봤다.
- 결함 없음으로 판정: 슬롯 A/B 프로젝트 혼선 — 모든 열기 진입점이 위젯의 `projectId` prop(슬롯 스코프) 또는 `useFocusedProjectId()` 를 쓰고, `withCurrentWindowTarget` 이 해당 프로젝트 레이아웃만 읽으며, 브로드캐스트 브리지 구독자는 `useIsShellSlotFocused()` 로 게이팅된다(실기 검증은 안 함).

### tabbar-dnd
- 읽음(전문): `pane-tab-bar.tsx`, `pane-node-view.tsx`, `tab-context-menu.tsx`, `sortable-tab.tsx`, `tab-item.tsx`, `split-drop-zones.tsx`, `query-client.ts`. 부분: `editor-area.tsx` 108~215·390~545, `terminal-session.tsx` 1~140, `explorer-container.tsx` 165~190. Rust: `layout/service.rs` 의 `is_empty_leaf`/`extract_tab`/`insert_tab`(276~310), `open_tab`(443~510), `successor_leaf_after_prune`/`close_tab`/`push_closed`/`reopen_closed`(582~700), `pin_tab`/`set_preview`/`move_tab`(760~925), `leaf_with_tab`~`focus_pane`(934~1045), `is_volatile`~`ensure_focused_pane_valid`(1196~1245), `close_tab_and_finish`(1471~1490). 문서: d-65 계약 전문, `tabs.md` §3~§4.2·§6·§7.
- 못 봄: **d-63 감사 본문과 d-63 계약을 읽지 못했다**(HANDOFF §4·`tabs.md` 로만 재론 금지 항목을 걸렀다) → 생존 3건 중 d-63 에서 이미 기각된 것이 섞였을 가능성은 배제 못 함(다만 셋 다 타이밍 클래스가 아니라 순서·플래그·동등성 규칙이라 겹칠 소지는 낮다). 터미널 축(pane 이동 시 pty 유지·프로젝트 닫기 kill·replay·제목 rename 추종) 전부 미조사(`terminal-session.tsx` 140행 이후·`terminal-pane.tsx`·터미널 도메인 Rust 미열람). dnd-kit 의 ESC 취소 시 `onDragEnd` 동반 호출 여부 미확인.
- finding 에서 제외: 직렬 `closeTabAsync` 루프의 중간 실패 시 조용한 중단 + unhandled rejection(결정적 실패 조작을 특정 못 함), 삭제된 파일이 닫은 탭 스택에 쌓여 ⇧⌘T 가 없는 파일을 여는 건(`tabs.md:236` 에 이미 백로그), `reopen_closed` 가 pane 소실 시 항상 main 트리로 폴백하는 점(보조 창 UX 의도 미확정).

### slots-windows
- 읽음(전문): `shell-slot.ts`, `shell-slot-context.tsx`, `project-drag.ts`, `shell-slot-provider.tsx`, `app.tsx`, `session.query.ts`, `session.ipc.ts`, `shell-slot-header.tsx`, `app-shell.tsx`, `shell-slot-tree-view.tsx`, `project-shell.tsx`, `use-project-drag.ts`, `use-window-chrome.ts`, `auxiliary-window-shell.tsx`, Rust `project/shell_slots.rs`(1~230). 부분: `project/service.rs` 40~300·380~560·990~1030, `project/commands.rs` 17~115·185~290·455~535, `ipc-sync-provider.tsx` 240~330, d-65 계약 전문, d-62 계약 §0.1·구현 기록·§4.
- 못 봄: 닫힌 슬롯 프로젝트의 **dirty 탭·터미널 PTY·LSP 세션 수명**(`project/capability.rs`·`domain/terminal`·`domain/lsp` 의 detach/reap 경로 미독). **Q2(탭 새 창 이동 → 창 닫기 → 재시작 복원 3단 정합)는 사실상 미조사** — `layout_move_tab_to_new_window`·`return_auxiliary_window_tabs`·`cleanup_emptied_auxiliary_windows`·`plan_auxiliary_window_restorations` 본문 미독. `project_group_*` 커맨드 본문(`groups.rs`·`commands.rs` 580~900)은 emit 지점만 매핑.
- 검증 불가: Zen 중 `hidden` Panel 아래의 monaco `automaticLayout`·xterm `proposeDimensions` 가 0 크기를 안전하게 통과하는지는 런타임 계측이 필요해 확인 못 했다(추측 finding 은 올리지 않음).
- 보고하지 않은 관찰: `close_shell_slot` 의 마지막 슬롯 거부 메시지가 비-로케일 raw 영어(UI 가드로 실질 도달 불가), `problemsOpenSlotIds` 가 사라진 슬롯 id 를 회수하지 않는 메모리 nit, 슬롯 포커스 이동마다 `save_project`+`save_session`+`ProjectActivated` 가 발생해 Welcome 최근 순서가 재정렬되는 것(성능/정책 추정).

### fileops-tabs
- 읽음(전문): `explorer-container.tsx`, `use-explorer-entry-crud.ts`, `use-explorer-clipboard.ts`, `explorer-path.ts`, `file-tree.tsx`(주요부), `file-tree-draft-row.tsx`, `entry-name.ts`, `file.query.ts`, `tree.query.ts`/`tree.ipc.ts`, `tab-path-change.ts`, `ipc-sync-provider.tsx`, `editor-draft-sync.ts`. 부분: `layout.query.ts`(1~200), `model-registry.ts`(retargetModel·applyExternalContent), `editor-pane.tsx`(1~400), `use-editor-file-persistence.ts`(미러/복원/settle 구간). Rust: `layout/{types.rs, service.rs 660~870, commands.rs}`, `file/{service.rs 190~340, commands.rs 58~120}`, `tree/{service.rs, commands.rs}` 전문, `root_guard.rs`(1~180), `watcher.rs`(1~120·196~310).
- 못 봄: **LSP 문서 동기**(rename/delete 시 didClose/didOpen 재발화 여부) — `use-editor-lsp-integration.ts`·`use-lsp-session.ts`·`workspace-edit-applier.ts` 미독. 미리보기 탭(`widgets/preview-pane`)의 `FILE.RAW` 경로와 `open-with-registry`. `domain/search/service.rs`(퀵오픈 walk 의 무시목록 적용 실코드) — d-64 커밋 문서 기술만 근거로 삼아 **"잘못된 카운트" 축은 미검증**. 워처 `group_relevant_changes`/`is_ignored_path`/self-write TTL 의 정확한 판정 코드(일부만 확인). `app-shell.tsx` 드래그앤드롭 열기, 탭 바 드래그 분할, 터미널/diff 탭.
