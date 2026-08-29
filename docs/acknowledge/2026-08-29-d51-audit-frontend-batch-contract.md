# d-51 계약 — 전수조사 프론트 버그+성능 일괄 (2026-08-29)

> 발견 정본: `docs/quality-assurance/2026-08-29-full-audit.md` §1·§4-B. d-50 과 한 workflow 로
> 병행 실행(FE 직렬 체인 F1~F8). 산출물은 커밋 지시 전까지 워킹트리.

## §0 범위 제외

- L1-03 팔레트 fuzzy(기지 — 실측 후 판단 결정 유지), perf-1 의 monaco 자체 지연(구조 수술 — 후순위).
- B8 의 탭 kind 영속화(Rust 표면) — FE 메모리 보존으로 우선 해소, kind 확장은 §5 이월.
- UX 신기능(d-53)·CI(d-52)·백로그(§7) 항목.

## §1 스테이지 (직렬 F1→F8)

| 스테이지 | 항목 |
|---|---|
| F1 저장·미러 정착 | A1(복원·sync 순서 역전 근본 수정)·A7+B7(경로 단위 저장 settle 브로드캐스트 — ide_save 포함 동일 클래스)·perf-9(getValue 지연) |
| F2 검색 UI | A5(쿼리 스냅샷 치환)·B8(FE 보존·자동 재실행 제거)·B9(projectId 키·리셋)·C9(미실행/실패 분기)·C10 FE(스킵·잘림 표시)·C11(placeholder)·D4(검증 후 정리)·perf-4(결과 가상화) |
| F3 git FE | A4(충돌 중 커밋 차단)·B10(diff 탭 절대경로 일원화)·B11 FE(beforePath 배선)·C5·C6·D8(검증 후) |
| F4 탐색기·탭·팔레트·키맵 | A2 FE(충돌 시 재시도 흐름)·B1(이중 발동)·B2(보조 창 등록)·B3(hasWidgetFocus)·B4(pinned 가드 3종)·C1·C3·C4·C7·C8·D1·D2·D3(검증 후) |
| F5 터미널 FE | A6(스폰 캐시 반영)·B14(exited 전역 무효화)·C13(와이드 문자 좌표)·C14(큐잉·포커스·고아)·D5(attach 시 resume) |
| F6 테마·설정·플러그인·아웃라인 | B5(테마 삭제 fallback)·B6(draft 소스 보존)·B12(심볼 재요청)·B15(공통 onError·blur 재기입)·C12·D6(검증 후)·D7 |
| F7 성능 묶음 | perf-1(lazy 분리)·2(pdf worker 지연)·5(리사이즈 디바운스+에코 억제)·7(grammar 온디맨드)·8(git predicate)·11(심볼 디바운스)·12(Problems 가상화)·13(fs:changed 통합)·14(팔레트 렌더 비용) |
| F8 IME 공통 가드 | B13 — keyCode 229 공통 가드 도입(키맵 chord·검색 Enter·draft row 등 전수), 근거 `docs/bug/2026-08-06` 실측 |

## §2 공통 규칙 (d-50 §2 준용)

- 유력(D 그룹)은 **선검증 후 수정** — 확증 실패 시 수정하지 않고 §5 에 판정 기록.
- 버그는 재현 테스트 우선(로직 분리로 bun test 가능한 형태), 컨벤션(arrow·주석 금지·타입 유도) 준수.
- 커밋·푸시 금지.

## §3 구현 기록 (스테이지별 append)

### F1 저장·미러 정착 (A1 · A7+B7 후반 · perf-9)

**A1 — 핫엑시트 미러 복원 vs 디스크 sync 순서 역전 (근본 수정)**

- 원인 확정(현재 워킹트리 기준 재확인): `EditorPane` 한 커밋에서 복원 effect
  (`use-editor-file-persistence.ts` 의 `[editor, path, mirrors]`)와 sync effect
  (`editor-pane.tsx` 의 `[editor, syncedContent, dirty, path]`)가 **둘 다 `queueMicrotask`** 로
  예약된다. 두 microtask 사이에 리렌더가 없으므로 sync 의 `useEffectEvent` 가 읽는 `dirty` 는
  **구조적으로 스냅샷(=false)** 이고, 복원이 세운 dirty 를 볼 수 없어 가드를 통과 →
  `applyExternalContent` 가 복원 버퍼를 디스크 원본으로 되씌움. 되씌움이 다시
  `onDidChangeModelContent` → `handleChange` 를 태워 500ms 뒤 **미러에 디스크 내용이 기록**되며
  복구 데이터까지 소실.
- 수정 1(라이브 dirty): `use-editor-file-persistence.ts` 에 `dirtyRef` 도입. 상태 setter 를
  `setDirty(next)` 하나로 좁혀 `dirtyRef` 와 `useState` 가 **같은 문장에서 함께** 움직이도록 했다.
  sync 가드는 스냅샷 대신 `isDraftDirty()` 로 라이브 값을 읽는다. 복원→sync / sync→복원 **어느
  순서로 실행되든** 안전하다.
- 수정 2(자기 변경 에코 차단): `entities/editor/model-registry.ts` 의 `applyExternalContent` 가
  `setValue` 구간 동안 모델을 `modelsApplyingExternalContent`(WeakSet)에 넣고,
  `features/editor/code-editor.tsx` 의 change 구독이 `isApplyingExternalContentTo(model)` 인
  이벤트를 무시한다. 프로그램적 콘텐츠 적용(디스크 sync·미러 복원·"디스크 보기")이 키 입력으로
  오인돼 dirty 를 세우고 미러 debounce 를 무장하던 경로를 근본 차단 — A1 후반부(미러 파괴)와,
  조사 중 발견한 **"디스크 보기" 직후 tab 이 즉시 다시 dirty 가 되던 동근 결함**을 함께 닫는다.
- 로직 분리: `widgets/editor-pane/editor-draft-sync.ts` 신설 — `syncModelFromDisk(deps)` 가
  "실행 시점에 읽는" 결정을 담당(스킵 / 미관측 모델 편집 인수 / 디스크 적용). one-shot 플래그
  `consumeExternallyDirtyModel` 은 clean 일 때만 소비하도록 순서를 보존.
- 재현 테스트(선작성): `editor-draft-sync.test.ts` 의 `runCommitWithPendingMirrorRestore` 가 두
  microtask 예약 순서를 실제로 재현한다 — 복원 microtask 가 먼저 dirty 를 세운 뒤 sync 가 도는
  상황에서 `applyDiskContent` 가 호출되지 않아야 통과(스냅샷 기반 가드로는 실패).

**A7 + B7 후반 — 저장 성공의 경로 단위 정착**

- 신설 `entities/editor/file-save-settle-registry.ts`: 경로별 리스너 레지스트리
  (`subscribeFileSaveSettle` / `publishFileSaveSettle`). 리스너는 서로 격리(throw 전파 없음),
  통지 중 자기 해제해도 순회 무결.
- 발행 지점을 **모든 저장이 통과하는 한 곳**(`entities/file/file.query.ts` 의 `useSaveFile.onSuccess`)
  으로 잡아 ⌘S·자동 저장·`ide:save-requested`(ide-sync-provider)·untitled Save As 를 한 번에 덮었다.
  충돌 해소는 `settleAfterDiskWrite` 가 직접 발행한다.
- 구독: `use-editor-file-persistence.ts` 의 `[projectId, path, tabId]` effect. 정착 내용은
  `settleDraftToDiskContent(content)` 로 단일화 — epoch bump·미러 타이머 해제·`pendingMirrorRef`
  해제·`setSyncedContent`·dirty 해제(+`layout_set_tab_dirty`)·restoreNotice 해제·`FILE.MIRRORS`
  캐시에서 해당 path 엔트리 제거. `handleSave.onSuccess` / `handleViewDisk` /
  `settleAfterDiskWrite` 가 전부 이 함수를 쓴다(중복 6줄 제거).
- 가드: `shouldSettleDraftAfterDiskWrite(draft, written)` — 저장 왕복 중 타이핑된 pane 은 정착
  대상에서 제외(dirty·미러 유지). 이미 clean 인 pane 은 `layout_set_tab_dirty` IPC 를 생략해
  저장 1회당 레이아웃 무효화가 pane 수만큼 발생하지 않게 했다.
- 미러 캐시: `setQueryData` 업데이터가 지울 엔트리가 없으면 `undefined` 를 반환(= 변경 없음).
  미조회 `staleTime: Infinity` 쿼리에 `[]` 를 심어 프로젝트 전체 미러를 소실시키는 사고와, 매
  저장마다 새 배열로 전 구독자를 리렌더시키는 낭비를 동시에 막는다.
- `ide-sync-provider.tsx`: `useSaveFile()` 이 projectId 없이 호출되어 `FILE.MIRRORS` 를
  무효화하지 못하던 구멍을 보완 — 저장 성공 후 `payload.projectId` 로 명시 무효화. (마운트된
  pane 은 방송으로 정착, 마운트 안 된 탭은 이 무효화로 미러 부활 차단.)
- 가짜 배너 판정을 `hasChangedOnDiskConflict({ isDirty, syncedContent, diskContent })` 로 추출해
  `editor-pane.tsx` 가 사용. 재현 테스트(`editor-draft-sync.test.ts`)는 실제 레지스트리 + 실제
  가드로 스플릿 두 pane 을 모사해 **저장 전 가짜 `changedOnDisk` 배너·미러 무장 → 발행 후 해소**
  를 검증한다.

**perf-9 — 키스트로크마다 `model.getValue()` 전체 문자열 생성**

- `CodeEditorProps.onChange` 를 `(value: string) => void` → `(readContent: () => string) => void`
  (게으른 접근자)로 전환. `code-editor.tsx` 의 change 구독은 문자열 대신 클로저만 넘긴다.
- `use-editor-file-persistence.ts` 의 `draftRef` 를 `string | null` → `DraftReader | null` 로 바꾸고
  모든 읽기를 `readDraft()` 로 통일. 미러 debounce(500ms)·마크다운 프리뷰 debounce(200ms)는
  **발화 시점에** 읽으므로 키 입력당 문자열 생성이 0이 되고, 읽는 값도 더 최신이다. 미러 복원·
  "디스크 보기"처럼 모델이 아닌 출처는 `constantDraft(content)` 로 감싼다.
- 다른 두 호스트(`untitled-pane.tsx`·`app-file-pane.tsx`)는 즉시 읽기(`readContent()`)로 계약만
  맞춰 최소 변경.
- 충돌 마커 재파싱 동일 경로: `use-editor-git-gutter-and-conflicts.ts` 의
  `onDidChangeModelContent` → `parseConflictMarkers(model.getValue())` 를
  `CONFLICT_REPARSE_DEBOUNCE_MS`(150ms)로 디바운스(cleanup 에서 타이머 해제). 해소 경로
  (`applyConflictResolution`)는 이 구독에 의존하지 않으므로 영향 없음.

**변경 파일**

- 신규: `src/entities/editor/file-save-settle-registry.ts`(+`.test.ts`),
  `src/widgets/editor-pane/editor-draft-sync.ts`(+`.test.ts`)
- 수정: `src/widgets/editor-pane/editor-pane.tsx`,
  `src/widgets/editor-pane/use-editor-file-persistence.ts`,
  `src/widgets/editor-pane/use-editor-git-gutter-and-conflicts.ts`,
  `src/widgets/editor-pane/untitled-pane.tsx`, `src/widgets/app-file-pane/app-file-pane.tsx`,
  `src/features/editor/code-editor.tsx`, `src/entities/editor/model-registry.ts`,
  `src/entities/file/file.query.ts`, `src/app/providers/ide-sync-provider.tsx`,
  `docs/features/editor.md`
- IPC 커맨드·이벤트·타입 표면 변경 없음(bindings·dispatch·원격 정책·로케일 무영향).

**검증**: `bun run typecheck` 통과, `bun run lint` 0 error(경고 7건은 기존 동일 계열 —
`exhaustive-deps` 는 이 파일이 이미 `persistMirror` 로 동일 패턴), `bun run format:check` 통과,
`bun test` 1557 pass / 0 fail(F1 추가 21건 포함).

### F2 검색 UI (A5 · B8 · B9 · C9 · C10 FE · C11 · D4 · perf-4)

선행 S1a(d-50) 계약 §3-(5) 준수 — `SearchResultGroup`·`SearchMatchRowData`·배치 수신부는 그대로
쓰고, S1a 가 새로 반환하는 `skipped`/`skippedCount` 를 소비하며, 파일 간 순서 비결정을 전제로
가상화 key 를 경로 기준으로 잡았다.

**A5 — Replace All 이 클릭 시점 입력값으로 치환**

- `useSearchRun` 이 `run(query)` 시점의 쿼리를 `ranQuery`(ref+state 동시 이동)로 스냅샷한다.
  `search-panel-container.tsx` 의 `handleReplaceAll` 은 `buildQuery()` 가 아니라 이 스냅샷으로
  `searchReplace` 를 호출하고, 치환 후 재검색도 같은 스냅샷으로 돈다(예전에는 `handleSubmit()` 이
  현재 입력으로 다시 검색해 "치환한 것"과 "다시 보이는 것"이 어긋났다).
- 게이트: 신설 `entities/search/search-query.ts` 의 `isSameSearchQuery(현재 입력, ranQuery)` 가
  false 면 Replace All 버튼 비활성 + `search.replaceStaleHint` 안내. `normalizeSearchQuery` 로
  생략 옵션을 기본값으로 채워 비교하므로 탭에 저장된 부분 쿼리와 패널의 완전 쿼리가 같게 판정된다.
  `contextLines` 는 매치 집합을 바꾸지 않으므로 비교에서 제외(테스트로 고정).
- 추가 테스트: `search-query.test.ts` 9케이스.

**B8 — Search Editor 매치 클릭 시 상태 전파괴 + 복귀 자동 재실행**

- 원인 재확인: `pane-node-view.tsx` 는 pane 의 **활성 탭만** 렌더하므로, 매치 클릭이 같은 pane 에
  파일 탭을 열면 `SearchEditorPane` 이 통째로 언마운트된다. 모든 상태가 컴포넌트 state 였고
  마운트 effect 가 탭의 원래 `query` 로 자동 재실행했다.
- 신설 `entities/search/search-editor-memory.ts`: 탭 id 키 메모리(입력 7종 + 결과 스냅샷).
  `writeSearchEditorMemory` 는 재저장 시 delete→set 으로 최근성을 갱신하고
  `SEARCH_EDITOR_MEMORY_LIMIT`(16) LRU 로 제한한다. `readSearchEditorMemory(tabId, projectId)` 는
  projectId 불일치 항목을 돌려주지 않고 **폐기**한다(B9 와 같은 교차 오염 방지).
- `useSearchRun(projectId, sessionId, initialSnapshot?)` 3번째 인자 신설 + `readSnapshot()` 반환.
  스냅샷 복원은 `search-result.ts` 의 신설 `restoreSearchResultAccumulator(groups, total)` 로
  `indexByPath` 를 다시 세워, 복원 후 같은 경로 배치가 도착해도 그룹이 둘로 갈라지지 않는다.
- 자동 재실행 제거: 메모리 항목이 있으면 마운트 자동 실행을 하지 않는다(탭이 처음 열릴 때·앱
  재시작 후처럼 메모리가 없을 때만 1회). 언마운트 cleanup 이 `latestRef`(매 렌더 갱신되는
  form+readSnapshot)로 최신 값을 저장한다 — cleanup 이 첫 렌더 클로저를 잡는 문제를 ref 로 차단.
- 실행 중 언마운트는 스냅샷 status 를 `completed` 로 낮춘다(백엔드 run 은 취소되므로 "도착한
  만큼이 결과 전부"가 사실이다). 편집된 쿼리의 탭 kind 영속화는 §5 이월.
- 추가 테스트: `search-editor-memory.test.ts` 5케이스, `search-result.test.ts` 복원 2케이스.

**B9 — 프로젝트 전환 후 검색 잔존·레이아웃 교차 오염**

- 재확인: `app-shell.tsx` 는 `ExplorerContainer` 를 projectId 로 키잉하지 않아, 검색 뷰를 켠 채
  프로젝트를 바꾸면 이전 프로젝트의 쿼리·결과·폴더 범위가 그대로 남고 그 매치를 클릭하면
  **새 프로젝트의 layout** 에 남의 경로로 탭을 열었다.
- `explorer-panel.tsx`: `<SearchPanelContainer key={projectId} …>` + projectId 변경 감지 시
  `searchRequest`(폴더 범위·seed·replace 요청) 리셋(같은 파일이 이미 쓰는 렌더 중 상태 조정 패턴).
  컴포넌트 JSDoc 에 근거를 남겼다.

**C9 — 미실행·실패를 0건과 구분**

- 신설 `entities/search/search-run-state.ts`: `SearchRunStatus`(idle·running·completed·failed) +
  `resolveSearchResultsView({status, hasResults})`. `useSearchRun` 이 `isSearching` 대신 `status` 를
  내보내고(구 `isSearching` 은 소비처가 없어져 제거), 패널·Search Editor 둘 다 본문을 이 한 함수로
  가른다. 헤더 카운트 줄은 `completed` 에서만 뜬다 — 검색어를 입력만 하고 Enter 를 누르지 않은
  상태에서 "일치하는 결과가 없습니다"가 뜨던 오표시가 사라진다.
- `.finally` 로 `isSearching` 을 내리던 자리는 `.then`→`completed` / `.catch`→`failed` 로 분리.
- 추가 테스트: `search-run-state.test.ts` 7케이스.

**C10 FE — 치환 스킵 목록 · 10,000건 잘림 배지**

- 신설 `entities/search/replace-skip-report.ts`: `buildReplaceSkipReport(result, describe)` 가
  `skipped`(백엔드 상한 50) 중 `REPLACE_SKIP_LIST_LIMIT`(5)까지 줄로 만들고 `skippedCount` 기준
  잔여 수를 계산한다. 컨테이너는 성공 토스트와 별개로 경고 토스트(사유별 목록 + "외 N개")를 띄운다.
  `REPLACE_SKIP_REASON_MESSAGE_KEY` 는 `ReplaceSkipReason` union 을 `Record` 로 받아 사유가 늘면
  타입에러가 나게 했다.
- 잘림: 신설 `shared/constants/search.ts` 의 `SEARCH_MATCH_LIMIT`(Rust
  `domain::search::types::SEARCH_MATCH_LIMIT` 미러 — 이 상수는 specta 표면이 아니라 값 미러가 유일한
  수단) + `isSearchResultTruncated(total)`. `completed && isTruncated` 일 때 두 화면 모두
  `search.truncated` 를 띄운다. (S1a §5 가 "어떤 매치가 남는지는 비결정 → 표시 정책으로 다룬다"고
  이월한 몫이 이것.)
- 추가 테스트: `replace-skip-report.test.ts` 5케이스.

**C11 — 제외 글롭 placeholder**

- 실제 매처(`domain/search/service.rs::glob_match`)는 프로젝트 상대 경로 **전체**와 대조하는 단순
  `*` 와일드카드이고 `*` 가 `/` 를 넘는다 — `**/`·중괄호·쉼표 목록은 없다. 기존 예시 `**/*.test.ts`
  는 루트 직속 파일을 못 걸러 실제와 어긋났다. ko/en/ja 3로케일을 `*.test.ts` 예시로 교체.

**D4 — 선검증 후 확증분만 수정**

- **확증①: 명시 cancel + run 경합.** `run()` 이 `void cancelSearch(sessionId)` 를 fire-and-forget
  으로 앞세운 뒤 곧바로 `runSearch` 를 호출했다. 두 커맨드는 각각 `state.begin_mutation().await` 를
  기다리는 별개 태스크라 순서가 보장되지 않고, `search_run` 이 먼저 `begin_search` 로 **새 토큰을
  등록한 뒤** 취소가 도착하면 새 실행이 그대로 취소된다(결과 0건·에러 없음). `begin_search` 자체가
  같은 `(owner, session_id)` 의 직전 토큰을 취소하므로 앞선 호출은 애초에 불필요 → **제거**.
- **확증②: 언마운트 미취소.** 기존 cleanup 은 flush 타이머만 정리해, 언마운트 뒤에도 백엔드가
  프로젝트 전체를 계속 스캔했다. cleanup 에서 generation 을 올리고 **실행 중일 때만**
  `cancelSearch(sessionId)` 를 보낸다(실행 중이 아닐 때 보내면 위 ①의 역경합을 새로 만든다).
- **확증③: 히스토리 경합.** `useAddRecentSearch` 가 렌더 시점 `recentSearches` 를 기준으로
  read-modify-write 했다 — 설정 왕복 안에 검색을 두 번 하면 둘 다 같은 기준에서 계산해 두 번째가
  첫 번째를 덮고 앞 검색어가 사라졌다. `RecentSearchQueue`(pending + inFlight) 를 도입해 진행 중인
  쓰기가 있으면 자기 직전 결과를 기준으로 쌓고, 전부 정착하면 서버 값으로 복귀한다.
  추가 테스트: `search-history.test.ts` 4케이스(첫 케이스가 유실 재현).

**perf-4 / 보고서 §1-4 — 결과 리스트 가상화**

- 신설 `features/search/search-result-rows.ts`: `buildSearchResultRows(groups, collapsedPaths)` 가
  그룹 헤더 + 매치 행을 한 목록으로 평탄화(접힌 그룹은 헤더만), `estimateSearchResultRowHeight` 가
  컨텍스트 줄 수로 높이를 계산한다.
- `search-results-list.tsx` 가 `useVirtualizer` + 자체 스크롤 뷰포트 + `OverlayScrollbar` 를
  소유한다(file-tree·commit-graph 와 같은 패턴). 행 높이가 가변이라 `estimateSize` 와
  `measureElement` 실측을 함께 쓰고, `getItemKey` 는 인덱스가 아니라 **경로 기반 id** 다
  (S1a 의 파일 간 순서 비결정 계약). 호출부는 결과가 있을 때 `ScrollContainer` 로 감싸지 않는다.
- Search Editor 뷰도 같은 컴포넌트를 쓰므로 함께 적용된다(컨텍스트 줄 포함 전량 DOM 문제 해소).
- 추가 테스트: `search-result-rows.test.ts` 6케이스.

**변경 파일**

- 신규: `src/shared/constants/search.ts`, `src/entities/search/search-query.ts`(+`.test.ts`),
  `src/entities/search/search-run-state.ts`(+`.test.ts`),
  `src/entities/search/replace-skip-report.ts`(+`.test.ts`),
  `src/entities/search/search-editor-memory.ts`(+`.test.ts`),
  `src/features/search/search-result-rows.ts`(+`.test.ts`)
- 수정: `src/entities/search/{use-search-run.ts,search-result.ts,search-result.test.ts,
  search-history.ts,search-history.test.ts}`, `src/features/search/{search-results-list.tsx,
  search-panel.tsx}`, `src/widgets/search-panel/search-panel-container.tsx`,
  `src/widgets/search-editor/search-editor-pane.tsx`, `src/widgets/explorer/explorer-panel.tsx`,
  `src-tauri/src/domain/locale/service.rs`(search 네임스페이스 키 10 등록),
  `src-tauri/resources/locales/{ko,en,ja}.json`(신규 10키 × 3 + `excludeGlobPlaceholder` 교체),
  `docs/features/explorer-sidebar.md`
- IPC 커맨드·이벤트·타입 표면 변경 없음 → bindings 재생성·dispatch 등재·원격 정책 분류 불요.
  로케일만 3언어 파리티로 추가(`내장_3종_로케일은_같은_키_집합을_가진다` ·
  `en_메시지의_모든_키는_required_message_keys에_포함된다` 통과).

**검증**: `bun run typecheck` 통과, `bun run lint` 0 error(경고 8건 — 기존 7건 + 신규 1건은
`search-results-list.tsx` 의 react-virtual "Compilation Skipped", `file-tree.tsx:115` ·
`commit-graph.tsx:61` 과 동일 계열), `bun run format:check` 통과, `bun test` 1593 pass / 0 fail
(F2 추가 36건 포함). Rust 는 `cargo fmt --all` 후 `cargo clippy --workspace --all-targets -D warnings`
무경고, `cargo test --lib locale` 17 통과.

### F3 git FE (A4 · B10 · B11 FE · C5 · C6 · D8)

선행 S3(d-50) 계약 §3-(5)·§5 준수 — `getGitDiffFile` 이 이미 받는 선택 인자
`beforePath` 를 소비하고, S3 가 §5 에 남긴 "F3 가 `beforePath` 를 넘기기 시작하면 `QUERY_KEY.GIT.DIFF`
에도 반드시 포함" 을 그대로 이행했다(끝에 덧붙여 `fs:changed` 의 path 인덱스 3 을 보존).

**A4 — 미해결 충돌 중 커밋 차단**

- 원인 재확인(현재 워킹트리): `git-panel.tsx` 의 staged/unstaged 판정이 둘 다
  `!row.isConflicted` 로 충돌 행을 뺀다. 머지 중 다른 변경이 없으면 패널이 보는 값은
  **0 staged · 0 unstaged** 라 stage-all 확인 다이얼로그를 건너뛰고 바로 `onCommit()` 을 부르고,
  컨테이너의 `handleCommit` 은 `stageAll: !hasStaged` 로 **전량 스테이지**를 요청했다 —
  `<<<<<<<`/`=======`/`>>>>>>>` 마커가 그대로 머지 해소 커밋이 된다.
- 신설 `widgets/git-panel/commit-gate.ts`: `resolveCommitGate(rows)` 가
  `blockedByConflicts` / `confirmStageAll` / `commit` 을 돌려주는 **단일 판정처**다. 행 판정
  `isStagedRow`/`isUnstagedRow` 도 여기로 옮겨(그룹 렌더와 게이트가 같은 술어를 쓰도록) 판정이
  갈라질 여지를 없앴다.
- 소비 2곳: `git-panel.tsx` 의 `requestCommit`(차단 시 무동작) 과
  `git-panel-container.tsx` 의 `handleCommit`(차단 시 무동작, `stageAll` 은
  `gate === 'confirmStageAll'` 로 유도). 컨테이너까지 막은 이유는 `stageAll` 결정이 거기 있기
  때문이다 — 패널만 막으면 결정이 두 곳에 갈라진 채 남는다.
- 안내: `CommitBox` 에 `blockedReason: string | null` 신설. 버튼 비활성 + 사유 줄을 그리고,
  **⌘/Ctrl+Enter 단축키도 같은 `canCommit` 로 막는다**(단축키는 버튼의 disabled 를 보지 못한다).
  로케일 `git.commitBlockedByConflicts` 3언어 + `locale/service.rs` 필수 키 등록.
- 추가 테스트: `commit-gate.test.ts` 7케이스(첫 두 케이스가 재현 — 충돌만 있는 상태와 충돌+staged
  혼재 상태 둘 다 차단).

**B10 — diff 탭 경로 절대경로 일원화**

- 세 증상의 공통 원인은 **`TabKind::Diff.path` 표기 이원화**였다. SCM 패널만
  `StatusRow.path`(저장소 상대)를 넘기고, 탭바 "Open Changes"·탐색기 비교는 절대경로를 넘겼다.
  탭 재사용은 `layout/service.rs` 의 `existing.kind == tab.kind`(구조 전체 동등성)라 같은 파일이
  **다른 탭 둘**이 되고, `fs:changed` 는 절대경로만 담으므로 상대경로로 키잉된 `GIT.DIFF` 는
  **영영 무효화되지 않으며**, `DiffPane` 의 충돌 판정은 반대로 절대경로 탭에서 `row.path` 와
  만나지 못해 **상시 false** 였다(충돌 파일 위에 스테이지 거터가 떴다).
- 생산자 통일: `git-change-group.tsx` 가 `diffTargetOf(row)`(= `{ path: row.absPath, beforePath:
  row.origAbsPath }`)를 넘기고, `commit-detail-panel.tsx` 의
  `buildCommitFileDiffOpenTabInput` 도 `file.absPath`/`file.origAbsPath` 를 쓴다(파일 히스토리
  패널이 이미 절대경로로 `GIT.SHOW` 를 키잉하고 있어 같은 `(rev, file)` 이 두 캐시로 갈라져 있었다).
  탭바·탐색기 비교는 이미 절대경로라 무변경. `onOpenChanges` 시그니처는 `(target: GitDiffTarget,
  group)` 으로 바뀌어 경로와 원경로가 한 값으로 이동한다.
- 소비자: 충돌 판정을 `widgets/diff-pane/diff-stageability.ts` 의 `isDiffHunkStageable` 로 추출하고
  `absPath`·`path` **두 표기를 모두** 대조한다 — 통일 이전에 영속된 `layout.json` 의 diff 탭은
  여전히 상대경로를 들고 복원되므로, 그 탭에서 충돌 판정이 다시 뚫리지 않게 하기 위해서다.
- **기존 테스트 정정**: `commit-detail-panel.test.ts` 가 `path: 'src/a.ts'`·`beforePath:
  'src/old-name.ts'` 를 기대해 상대경로 생산을 정본처럼 고정하고 있었다 → 절대경로 기대로 정정
  (개명 케이스는 `path` 단언도 함께 추가). `ipc-sync-provider.test.ts` 의 DIFF 케이스는 손으로 쓴
  키 배열이라 실제 팩토리와 어긋날 수 있었다 → `QUERY_KEY.GIT.DIFF(...)` 호출로 교체하고,
  **상대경로 키는 절대경로 `changedPaths` 와 만나지 못한다**는 케이스를 새로 추가해 무효화 불발의
  근거를 테스트로 고정했다.
- 추가 테스트: `diff-stageability.test.ts` 5케이스 + `ipc-sync-provider.test.ts` 1케이스.

**B11 FE — staged/워킹트리 diff 의 `beforePath` 배선**

- 배선 축(전부 신규): `GitDiffTarget.beforePath` → `openDiffTab` → `TabKind::Diff.beforePath` →
  `pane-node-view.tsx` → `DiffPane` prop → `gitDiffFileQueryOptions({ beforePath })` →
  `QUERY_KEY.GIT.DIFF(projectId, path, mode, beforePath)` → S3 가 만든 `getGitDiffFile` 인자.
  `TabKind::Diff.beforePath` 는 커밋 diff 축이 이미 쓰던 필드인데 `rev == null` 분기에서만
  **버려지고 있었다** — 그 구멍을 메운 것이 이번 배선의 실질이다.
- 키 확장은 끝에 덧붙였다(S3 §5 지시): `fs:changed` 매칭이 읽는 인덱스 3 은 그대로 `path` 다.
  기본값 `null` 이라 기존 호출부는 형태가 바뀌지 않는다.
- **다만 상태 행에는 원경로 소스가 없다** — S3 §5 가 실측으로 남긴 대로 `StatusRow.origPath`/
  `origAbsPath` 는 구조적으로 항상 `null`(git2 `StatusEntry::path()` 가 `old_file.path` 를
  돌려주고, `collect_status_rows` 가 `filter(|orig| orig != path)` 로 거른다). 따라서 이 축이 실제로
  값을 나르는 것은 커밋 diff 뿐이고, 개명 행의 표시 정정은 감사 §2 M-2 선행이 필요하다 → §5 기록.

**C5 — 태그 다이얼로그 입력 리셋**

- `CreateTagDialog` 는 `open` 만 토글되고 언마운트되지 않아 이름·메시지가 다음 열림까지 남았다
  (취소한 태그 이름이 **다른 커밋**의 다이얼로그에 그대로 떠서 확인 한 번이면 엉뚱한 커밋에
  붙는다). 닫힘→열림 전이를 렌더 중 감지해 두 입력을 비운다(`explorer-panel.tsx` 가 이미 쓰는
  "prop 변화에 따른 상태 조정" 패턴 — effect 로 하면 리셋 전 프레임이 한 번 그려진다).

**C6 — 커밋 메시지 projectId 스코프 보존**

- 신설 `entities/git/commit-message-memory.ts`: projectId 키 메모리(`readCommitMessageDraft` /
  `writeCommitMessageDraft`, `COMMIT_MESSAGE_MEMORY_LIMIT` 8 LRU, 빈 문자열은 항목 삭제).
- `git-panel-container.tsx`: 초기값을 메모리에서 읽고(`useState(() => read…)`), projectId 변경을
  렌더 중 감지해 그 프로젝트의 메시지로 교체한다. 쓰기는 `applyCommitMessage` 하나로 좁혀
  입력·AI 생성·커밋 성공 후 비우기가 전부 상태와 메모리를 **같은 문장에서** 움직인다.
- 두 증상이 한 축에서 닫힌다: 뷰 전환은 컨테이너를 언마운트하지만 메모리가 남아 **소실되지 않고**,
  프로젝트 전환은 컨테이너를 언마운트하지 않지만 렌더 중 교체로 **교차 잔존이 사라진다**.
- 추가 테스트: `commit-message-memory.test.ts` 5케이스(뷰 전환 보존·프로젝트 격리·빈 문자열 삭제·
  상한·최근성).

**D8 — hunk 경계 정확일치 가정 (선검증 → 확증, 수정)**

- 가정 확인: `build_whole_hunk_patch` 는 프런트가 보낸 `(hunk_start, hunk_end)` 를 libgit2 hunk 의
  `gutter_range` 와 `start != hunk_start || end != hunk_end` 로 **정확일치** 비교한다. 프런트가 그
  값을 만드는 곳은 `DiffPane` 의 `diffEditor.getLineChanges()` — **monaco 자체 diff** 결과다.
- **확증(실측)**: monaco 0.56 의 diff 기본값은 `ignoreTrimWhitespace: true`
  (`common/config/diffEditor.js`). `linesDiffComputers.getDefault().computeDiff` 를 스크래치
  스크립트로 직접 돌려 두 케이스를 확인했다. ① `['alpha','    keep','beta']` →
  `['ALPHA','keep','BETA']`: true 면 변경이 **둘**(1-1, 3-3)로 쪼개지고 false 면 **하나**(1-3) —
  libgit2 는 공백 민감이라 `@@ -1,3 +1,3 @@` 한 덩어리이므로, true 상태에서 거터를 누르면 그
  범위가 백엔드에 존재하지 않아 `error.git.hunkNotFound` 로 실패한다. ② `['a','  b','c']` →
  `['a','b','c']`: true 면 변경이 **0건**이라 공백만 바뀐 실제 변경이 화면에도 거터에도 없다.
- 수정: `diff-view.tsx` 의 `createDiffEditor` 에 `ignoreTrimWhitespace: false`. DiffView 의 모든
  소비처가 git diff(또는 파일 대 파일 비교)라 바이트 충실이 옳고, 백엔드 비교와 같은 기준이 된다.
  잔여 위험(알고리즘 차이)은 §5 에 기록.
- 재현을 커밋 테스트로 남기지 않은 이유는 §5.

**변경 파일**

- 신규: `src/widgets/git-panel/commit-gate.ts`(+`.test.ts`),
  `src/widgets/diff-pane/diff-stageability.ts`(+`.test.ts`),
  `src/entities/git/commit-message-memory.ts`(+`.test.ts`)
- 수정: `src/widgets/git-panel/{git-panel.tsx,git-panel-container.tsx,commit-detail-panel.tsx,
  commit-detail-panel.test.ts}`, `src/features/git/{commit-box.tsx,git-change-group.tsx,
  create-tag-dialog.tsx,diff-view.tsx}`, `src/widgets/diff-pane/diff-pane.tsx`,
  `src/widgets/editor-area/pane-node-view.tsx`, `src/entities/git/git.query.ts`,
  `src/shared/constants/query-key.ts`, `src/app/providers/ipc-sync-provider.test.ts`,
  `src-tauri/src/domain/locale/service.rs`(git 키 1 등록),
  `src-tauri/resources/locales/{ko,en,ja}.json`(`git.commitBlockedByConflicts` × 3),
  `docs/features/git.md`
- IPC 커맨드·이벤트·타입 표면 변경 없음 → bindings 재생성·dispatch 등재·원격 정책 분류 불요
  (`gitDiffFile` 의 `beforePath` 인자는 S3 가 이미 만들어 bindings 에 반영했고, F3 는 그 표면을
  소비만 한다). 로케일만 3언어 파리티로 1키 추가.

**검증**: `bun run typecheck` 통과, `bun run lint` 0 error(경고 8건 — F2 와 동일 목록, 신규 없음),
`bun run format:check` 통과, `bun test` **1611 pass / 0 fail**(F3 추가 18건 포함).
로케일은 ko/en/ja 각 **933키·키 집합 완전 일치**를 파싱으로 확인했다. Rust 는 `cargo fmt --all` 후
`cargo fmt --all -- --check` 통과, `cargo test --lib locale` **17 통과**(이 스테이지의 키 등록
직후 실행). 이후 재실행 시점에는 동시 진행 중인 다른 스테이지가 `src-tauri/src/infra/lsp_proc.rs`
를 편집 중이라 lib test 가 그 파일의 컴파일 오류로 멈춘다 — F3 가 만진 표면(로케일 상수 1줄)과
무관하며, 전체 사다리는 최종 검증 스테이지가 수행한다.

### F4 탐색기·탭·팔레트·키맵 (A2 FE · B1~B4 · C1 · C3 · C4 · C7 · C8 · D1 · D2 · D3)

선행 S2(d-50) 계약 §3-(2) 준수 — S2 가 신설한 `error.file.destinationExists`
(`InvalidArgument`, `with_arg("path")`)를 그대로 소비한다. 이 스테이지는 그 에러 표면을 **읽기만**
하고 새 IPC 커맨드·이벤트·타입을 만들지 않았다.

**A2 FE — 목적지 존재 오류 수신 시 접미 재시도(붙여넣기) / 안내(생성·개명)**

- 원인 재확인: `use-explorer-clipboard.ts` 의 형제 이름 목록은 `rows`(=화면에 보이는 트리 행)에서만
  나온다. 트리는 lazy 라 **접힌 폴더의 자식은 애초에 조회된 적이 없고**, 그래서 로컬로 계산한
  "유일한" 이름이 디스크에서는 충돌한다. S2 이전에는 그 충돌이 조용한 덮어쓰기였고, S2 이후에는
  거절이므로 프런트가 그 거절을 **다음 후보 이름으로 흡수**해야 사용자가 붙여넣기를 완료할 수 있다.
- 신설 `widgets/explorer/paste-plan.ts`: `pasteWithUniqueEntryName({clipboard, siblingNames,
  conflictSuffix, run, attemptLimit})` 가 후보 이름을 만들어 `run(name)` 을 돌리고,
  `isIpcErrorKey(error, 'error.file.destinationExists')` 인 실패만 taken 집합에 접어 다음 후보로
  재시도한다(`PASTE_DESTINATION_ATTEMPT_LIMIT` 8). **그 외 오류는 재시도 없이 그대로 전파**되고,
  상한까지 전부 충돌하면 마지막 백엔드 오류를 그대로 던져 기존 토스트 경로로 나간다.
- 생성(`commitDraft`)·개명(`commitRename`)은 재시도하지 않는다 — 사용자가 **이름을 직접 지정한**
  조작이라 임의 접미를 붙이면 의도와 다른 결과가 된다. S2 의 로컬라이즈 메시지가 인라인 오류 +
  토스트로 그대로 표시된다(`describeIpcError` → `error.file.destinationExists` 3언어 카탈로그).
- 추가 테스트: `paste-plan.test.ts` 8케이스(재시도 시퀀스·비대상 오류 전파·상한 소진 포함).

**C1 — 잘라내기 제자리 붙여넣기 no-op**

- 같은 파일의 `isSamePlaceCutPaste(clipboard, targetDir)`. 잘라낸 항목 자신이 형제 이름 목록에
  들어 있어 `buildUniqueEntryName` 이 "이름 복사본" 을 만들고, cut 경로가 그것을 `renameEntry` 로
  실행해 **제자리에서 이름만 바뀌던** 경로를 닫는다. 이제 클립보드만 비우고 아무 IPC 도 보내지 않는다.

**C3 — 폴더 복사 접미의 확장자 오분해**

- `shared/lib/unique-entry-name.ts` 의 `splitBaseAndExtension` 이 종류를 몰라 폴더 `v1.2` 를
  `v1 복사본.2` 로 만들었다. `buildUniqueEntryName(desired, existing, suffix, kind)` 4번째 인자
  신설(기본값 `'file'` — 기존 호출부 동작 불변), `kind === 'directory'` 면 확장자를 분해하지 않는다.
  종류는 `ExplorerClipboardEntry.kind` 로 실어 나른다(`onCut`/`onCopy` 가 `row.kind` 를 담는다).
- 추가 테스트: `unique-entry-name.test.ts` 2케이스.

**C4 — 인라인 생성·개명 이중 Enter**

- `use-explorer-entry-crud.ts` 에 `draftCommitInFlightRef` / `renameCommitInFlightRef`. 입력 행은
  IPC 왕복 내내 마운트·포커스 상태로 남아 있어 Enter 를 두 번 치면 같은 이름으로 두 번째 커밋이
  날아가고, 그 두 번째가 "이미 존재" 오류 토스트를 띄웠다(첫 번째는 성공한 뒤였다). 검증 통과 뒤에
  가드를 세우고 `finally` 로 해제해 실패 후 재시도(토스트의 "다시 시도")는 그대로 동작한다.
- 순수 로직으로 떼어낼 수 있는 부분이 없어(훅 상태 + 실제 mutation 왕복) 유닛 테스트는 붙이지
  않았다 — 이 레포에는 React 훅 테스트 인프라(testing-library·DOM 환경)가 없다.

**B1 — APP 키맵과 재바인딩 커맨드의 동일 키 이중 발동**

- 원인 재확인: `command-palette.tsx` 는 `useGlobalKeymap`(APP_KEYMAP 디스패치)과 **별개의** window
  capture 리스너를 하나 더 등록한다. 형제 리스너라 `stopPropagation` 이 닿지 않으므로,
  `runsViaCommand` 행을 `APP_KEYMAP` 이 이미 쓰는 키로 재바인딩하면 한 번의 키 입력에 **두 액션이
  모두** 실행됐다(에디터는 저장 시 경고만 하고 막지 않는다 — 그 정책은 유지가 옳다고 판단했다.
  경고가 재바인딩을 금지하면 "먼저 상대를 해제한 뒤 다시 시도" 라는 왕복을 강제한다).
- 신설 `shared/lib/keymap/command-binding-dispatch.ts` 의 `decideCommandBindingRun` 이 **매칭 자체를
  공유**한다: `pending`/`monacoDeferral` 단락 뒤에 `decideKeymapDispatch`(순수 함수 — 두 번 평가해도
  부작용 0)를 한 번 더 돌려 `none` 이 아니면 커맨드 쪽이 물러난다. 승자는 항상 `APP_KEYMAP` 이고,
  `when` 불충족으로 APP 키맵이 놓치는 keydown 은 커맨드가 그대로 가져간다.
- 팔레트는 `entries`(오버라이드 적용 후)와 `monacoChordPrefixes`(`deriveMonacoChordPrefixes`)를
  `useGlobalKeymap` 과 동일하게 구성해 넘긴다.
- 추가 테스트: `command-binding-dispatch.test.ts` 6케이스(두 번째가 이중 발동 재현).

**B2 — 보조 창 ⌘K ⌘S 무동작**

- 원인 재확인: 핸들러(`'open-keybindings-editor'`)를 등록하던 곳이 `AppShell` 하나였고, 보조 창은
  `AppShell` 을 마운트하지 않는다. 그래서 보조 창에서는 chord 가 정상 해소되고 `preventDefault`
  까지 된 뒤 **핸들러 맵이 비어 아무 일도 일어나지 않았다** — 키만 삼켜지는 상태.
- `KeybindingsRuntimeProvider`(두 창 모두에 마운트)가 `useGlobalKeymap({ 'open-keybindings-editor':
  () => setOpen(true) })` 로 직접 등록하고, `AppShell` 의 등록은 제거했다(창마다 소유자 1개).
  브리지 구독(`subscribeOpenKeybindingsEditor`)은 키 입력 없이 오는 두 경로(팔레트 커맨드·설정
  화면 버튼) 때문에 그대로 남는다.

**B3 — monaco Find 위젯 포커스 중 ⌘F 이탈**

- `editor-area.tsx::openFind` 가 `hasTextFocus()`(= `textarea.inputarea` 한정)로 에디터를 찾아,
  Find 위젯 입력에 포커스가 있는 상태에서 ⌘F 를 다시 누르면 "포커스된 에디터 없음" 으로 읽혀
  전역 검색 패널이 열렸다. `hasWidgetFocus()`(에디터 컨테이너 + 오버레이 위젯)로 교체.

**B4 — pinned 탭 보호 3종**

- `tab-item.tsx`: 라벨이 이미 `tab.unpinAriaLabel` 이던 트레일링 버튼이 `onClose` 를 부르고 있었다
  (고정 해제처럼 보이는 버튼 한 번에 탭이 사라진다) → 고정 탭에서는 `onTogglePin`. 아이콘도
  고정 탭이면 hover 에서도 `Pin` 을 유지하고 `X` 를 보이지 않는다(`docs/features/tabs.md` §3 의
  "닫기 버튼 대신 pin 아이콘"). 미들 클릭은 고정 탭에서 무시.
- `editor-area.tsx::closeFocusedTab`(⌘W): 활성 탭이 pinned 면 `tab.pinnedCloseBlocked` 경고 후 유지.
  로케일 3언어 + `locale/service.rs` 의 `tab` 네임스페이스에 키 등록.
- context menu 의 **Close 는 그대로 닫는다** — 실수로 발동하는 제스처가 아니라 명시적 선택이므로
  탈출구로 남겼다(Close Others/To the Right/Saved/All 은 이미 pinned 를 건너뛴다).
- `SortableTab` 이 이미 받고 있던 `onTogglePin` 을 `TabItem` 으로 내려보내는 배선만 추가했고,
  드래그 오버레이의 `TabItem` 은 기존 no-op 스타일 그대로 `() => {}` 를 넘긴다.

**C7 — 무수식 2단 chord 캡처 허용**

- 이중 차단이었다: `keybinding-row.tsx` 가 수식어 없는 캡처를 stage 구분 없이 경고로 막고,
  `keybindings-editor.tsx::handleChangeBinding` 이 `chord.mods.length === 0` 저장을 거부했다.
  둘 다 1단에만 적용되도록 고쳤다 — 2단은 프리픽스로 이미 스코프되고, `APP_KEYMAP` 자신이 무수식
  2단 chord(⌘K Z, `toggle-zen-mode`)를 갖고 있어 캡처 UI 가 기본 제공 바인딩조차 재현할 수 없었다.
  무수식 Enter 는 여전히 "단일 키로 확정" 경로다(수식어를 얹은 Enter 는 2단이 될 수 있다 — 기존 주석).

**C8 — 키 검색 모드의 숨은 텍스트 필터**

- `toggleKeySearchMode` 진입 시 `setQuery('')`. 그 모드에서는 텍스트 입력이 캡처 버튼으로 교체돼,
  남은 필터가 보이지도 지워지지도 않는 채 목록을 계속 좁혔다(키로 검색이 "결과 없음" 처럼 보이는 원인).

**D1 — 팔레트 닫힘 시 포커스 복원 역전 (선검증 → 확증, 수정)**

- 확증: Radix `Dialog` 는 닫힘 시 열기 직전 포커스로 되돌린다. 팔레트의 실행 경로는 **먼저 액션을
  실행하고 그다음 닫으므로**, `code-editor.tsx` 가 모델 부착 시 호출하는 `editor.focus()`(실측:
  `[path, language]` 이펙트 마지막 줄) 뒤에 복원이 도착해 포커스를 이전 표면으로 도로 가져간다.
- `closedByActionRef` + `closeAfterAction()` 도입: 액션으로 닫힌 경우에만 `onCloseAutoFocus` 에서
  `preventDefault`. **Escape·바깥 클릭 닫힘은 기존대로 포커스를 되돌린다**(무조건 preventDefault 는
  아무 것도 안 하고 닫았을 때 포커스를 body 로 떨어뜨린다). 액션 경로 5곳(`runCommand`·`openFile`·
  `selectDocumentSymbol`·`selectLineTarget`·`selectWorkspaceSymbol`)만 새 헬퍼를 쓴다.

**D2 — 단일 키 재바인딩의 기본 when 승계 (선검증 → 확증, 수정)**

- 확증: `applyKeymapOverrides` 가 `{ ...entry, key, mods, chord }` 로 `when` 을 그대로 승계한다.
  `APP_KEYMAP` 에서 `when` 과 `chord` 를 **동시에** 가진 엔트리는 `open-keybindings-editor` ·
  `toggle-zen-mode` 2건뿐이고, 둘의 `!terminalFocus` 는 각 엔트리 주석이 밝히듯 **1단이 ⌘K 라서**
  붙어 있다. 그래서 ⌘K ⌘S 를 ⌘J 로 재바인딩하면 근거 없는 게이트만 남아 터미널 포커스 중에는
  새 바인딩이 조용히 죽는다.
- 수정: `resolveOverriddenKeymapWhen` — **base 에 `chord` 가 있고 오버라이드의 1단(key+mods)이
  base 와 다를 때만** `when` 을 떨어뜨린다. 1단을 유지한 채 2단만 바꾸면 ⌘K 는 그대로이므로 게이트도
  유지되고, `chord` 없는 엔트리의 의미적 `when`(터미널 jump 2건의 `terminalFocus`)은 어떤
  재바인딩에도 손대지 않는다 — 데이터 모양이 판정 근거라 새 필드를 만들지 않았다.
- 추가 테스트: `keymap.test.ts` 3케이스(1단 변경 시 해제 / 1단 유지 시 존치 / chord 없는 엔트리 존치).

**D3 — 충돌 검사가 monaco 기본 바인딩을 모른다 (선검증 → 확증, 수정)**

- 확증: `buildKeybindingRows` 는 `monaco.*` 행의 `key` 를 `''` 로 둔다(그 행의 실제 바인딩은 monaco
  내부에 있고 카탈로그는 표시용 라벨만 안다). `findConflictingRow` 는 `row.key` 가 비면 즉시 null 이라
  ~200개 monaco 행 전부가 충돌 판정에서 빠져 있었다 — ⌘D·⌘/·F12 위로 앱 액션을 재바인딩해도 무경고.
- 신설 `shared/lib/monaco/monaco-binding-label.ts`: `parseMonacoDefaultBindingLabel` 이 라벨을
  `{ key, mods, chord }` 로 되읽는다(`⌘`→`mod`·`⌥`→`alt`·`⇧`→`shift`·`⌃`→`ctrl`, 공백 분리 2단,
  `⌫`/`Space`/F키 매핑). 카탈로그의 **모든** 기본 라벨이 파싱됨을 테스트로 고정했다(실측: 미파싱 0).
- `keybinding-catalog.ts::resolveKeybindingRowBinding` 이 "그 행이 실제로 응답하는 바인딩" 을 주고,
  `findKeymapConflict` 에 신설한 `resolveBinding` 인자로 주입된다(기본값은 엔트리 자신 — 기존 호출부
  동작 불변). 오버라이드가 있으면 그쪽이 이기고, 해제(unbind)된 행은 라벨을 보지 않는다.
  라벨 파싱은 라벨 단위 캐시라 카탈로그 전체 대조(O(n²))에 추가 할당이 없다.
- 실측 부작용 1건: 이 판정이 켜지면서 **`find`(⌘F) ↔ `monaco.actions.find`(⌘F)** 한 쌍이 충돌로
  표시된다(전수 스캔 결과 신규 충돌 쌍은 이것뿐). 사실 관계로는 참이라 그대로 두었다 — §5 기록.
- 추가 테스트: `monaco-binding-label.test.ts` 5케이스, `keybinding-catalog.test.ts` 3케이스.

**변경 파일**

- 신규: `src/widgets/explorer/paste-plan.ts`(+`.test.ts`),
  `src/shared/lib/keymap/command-binding-dispatch.ts`(+`.test.ts`),
  `src/shared/lib/monaco/monaco-binding-label.ts`(+`.test.ts`)
- 수정: `src/widgets/explorer/{use-explorer-clipboard.ts,use-explorer-entry-crud.ts,
  explorer-container.tsx}`, `src/shared/lib/unique-entry-name.ts`(+`.test.ts`),
  `src/shared/lib/keymap/{keymap.ts,keymap.test.ts,keybinding-catalog.ts,keybinding-catalog.test.ts}`,
  `src/widgets/command-palette/command-palette.tsx`, `src/app/providers/keybindings-runtime-provider.tsx`,
  `src/widgets/app-shell/app-shell.tsx`, `src/widgets/editor-area/editor-area.tsx`,
  `src/features/tab/{tab-item.tsx,sortable-tab.tsx}`, `src/features/settings/keybinding-row.tsx`,
  `src/widgets/keybindings-editor/keybindings-editor.tsx`,
  `src-tauri/src/domain/locale/service.rs`(tab 키 1 등록),
  `src-tauri/resources/locales/{ko,en,ja}.json`(`tab.pinnedCloseBlocked` × 3),
  `docs/features/{keymap.md,tabs.md,explorer-sidebar.md}`
- IPC 커맨드·이벤트·타입 표면 변경 없음 → bindings 재생성·dispatch 등재·원격 정책 분류 불요
  (`error.file.destinationExists` 는 S2 가 만든 표면을 소비만 한다). 로케일만 3언어 파리티로 1키 추가.

**검증**: `bun run typecheck` 통과, `bun run lint` 0 error(경고 8건 — F2·F3 와 동일 목록, 신규 0),
`bun run format:check` 통과, `bun test` **1638 pass / 0 fail**(F4 추가 27건 포함).
로케일은 ko/en/ja 각 **934키**로 키 집합이 완전히 일치한다. Rust 는 `cargo fmt --all` 후
`cargo fmt --all -- --check` 통과, `cargo test --lib locale` **17 통과**
(`내장_3종_로케일은_같은_키_집합을_가진다` · `en_메시지의_모든_키는_required_message_keys에_포함된다` 포함).

### F5 터미널 FE (A6 · B14 · C13 · C14 · D5)

선행 S4(d-50) 계약 §3 준수 — `pty_attach` 리플레이/구독 원자성과 링버퍼 2청크 분할은 그대로 두고,
이 스테이지는 그 위에서 **누가 언제 attach 하는가**(로스터 정확도)와 **pause 상태의 소유권**만 손댔다.

**A6 — 스폰 성공이 `TERMINAL.SESSIONS` 로스터에 반영되지 않음**

- 원인 재확인(현재 워킹트리): `terminal.query.ts` 의 `terminalSessionsQueryOptions` 는
  `staleTime: Infinity` 라 마운트마다 다시 받지 않는다. 그런데 `terminal-session.tsx` 는 스폰 성공을
  **컴포넌트 state(`spawnedSessionId`)와 layout(탭의 `sessionId`)에만** 기록했다. 탭을 떠나면
  `pane-node-view.tsx` 가 활성 탭만 렌더하므로 컴포넌트가 언마운트돼 state 가 사라지고, 돌아오면
  로스터는 여전히 **스폰 이전** 목록이라 `persistedSession?.running` 이 false → 새 셸을 또 스폰하고
  앞의 셸은 아무도 참조하지 않는 고아가 된다(`features/terminal.md` §3 의 "재마운트 시 replay" 설계
  위반). 탭을 오갈 때마다 셸이 하나씩 늘어난다.
- 신설 `entities/terminal/terminal-session-cache.ts`: 로스터를 바꾸는 순수 업데이터 4종 —
  `upsertTerminalSession`(스폰) · `markTerminalSessionExited`(종료) · `removeTerminalSession`(고아 kill)
  · `isTerminalSessionAlive`(재부착 판정). 앞의 셋은 **바뀔 것이 없으면 `undefined`** 를 돌려
  `setQueryData` 가 아무 것도 쓰지 않게 한다(미조회 `Infinity` 쿼리에 부분 목록을 심어 그것이 전체
  진실로 굳는 사고 방지 — F1 의 `FILE.MIRRORS` 처리와 같은 이유).
- `terminal-session.tsx` 는 스폰 직후 `upsertTerminalSession` 으로 로스터에 자기 세션을 넣는다.
  엔트리는 `terminal_sessions` 가 돌려줄 값과 **구분 불가**해야 하므로 `shell` 은 `pty_spawn` 의
  `opts.shell.unwrap_or_else(|| "default")` 를 `DEFAULT_SHELL_LABEL`(신설, `shared/constants/terminal.ts`)
  로 미러했다(F2 의 `SEARCH_MATCH_LIMIT` 미러와 같은 근거 — specta 표면이 아니다).
- 판정부도 `isTerminalSessionAlive(liveSessions, persistedSessionId)` 로 옮겨 "로스터에 무엇이 쓰였나"와
  "attach 할 것인가"가 한 함수로 이어지게 했다.
- 추가 테스트: `terminal-session-cache.test.ts` 14케이스. 그중 `isTerminalSessionAlive` 의 첫 두
  케이스가 A6 재현이다 — 스폰을 반영하지 않은 로스터는 `false`(=새 스폰), `upsertTerminalSession` 을
  거친 로스터는 `true`(=재부착).

**B14 — `terminal:exited` 무효화가 자기 세션 가드 뒤에 있었다**

- 원인 재확인: 무효화가 `if (payload.sessionId !== sessionId) return` **아래**에 있어, 그 세션을 보고
  있는 컴포넌트만 로스터를 갱신했다. 배경 탭의 터미널은 애초에 마운트돼 있지 않으므로 그 세션의 종료는
  **아무도 듣지 못하고**, 로스터에 `running: true` 가 영구히 남아 그 탭으로 돌아가면 죽은 세션에
  attach 한다(입력은 먹고 출력은 없고 "[process exited]" 재시작 버튼도 뜨지 않는다).
- 처리를 `app/providers/ipc-sync-provider.tsx` 로 올렸다 — 이 프로바이더는 두 창 모두에서 **열린 탭과
  무관하게** 마운트된다(`app.tsx`). 페이로드에 projectId 가 없어 `QUERY_KEY.TERMINAL.SESSIONS_ALL`
  (신설 접두 키, `AGENT.HOOKS_PROJECT` 선례)로 캐시된 모든 프로젝트 로스터를 훑고
  `markTerminalSessionExited` 로 해당 세션만 내린다.
- **무효화가 아니라 즉시 쓰기**를 택했다. 리페치는 비동기라 그 사이 재마운트가 `running: true` 를 읽고
  attach 해버리는 — 닫으려는 바로 그 — 경합이 남고, 더 나쁘게는 마운트 시 `onReady` 가 이미 한 번
  발화한 뒤 리페치가 도착하면 `sessionId` 가 null 로 떨어져 스폰도 attach 도 하지 않는 정지 상태가 된다.
- `terminal-session.tsx` 에는 자기 세션의 "[process exited]" 화면 전환만 남겼다(가드 유지가 옳은 부분).

**C13 — 파일 링크 좌표가 와이드 문자에서 어긋남**

- 원인 재확인: `createTerminalFileLinkProvider` 가 `translateToString(true)` 의 **문자열 인덱스**를
  그대로 1-based 열로 썼다. 매치 문자 자체는 전부 단일폭 ASCII 라는 기존 JSDoc 의 근거는 맞지만,
  **매치 앞에 오는 문자**는 임의다 — CJK 글리프는 문자열 1자에 셀 2칸이라 앞에 한 글자 있을 때마다
  링크의 밑줄·클릭 판정이 한 칸씩 왼쪽으로 밀린다.
- **`outColumns` 는 쓸 수 없다(실측)**: 내부 `BufferLine.translateToString(trimRight, start, end,
  outColumns)` 은 아웃파라미터를 지원하지만, 공개 API 가 돌려주는 `BufferLineApiView` 가
  `this._line.translateToString(trimRight, startColumn, endColumn)` 로 **4번째 인자를 버린다**
  (`node_modules/@xterm/xterm/lib/xterm.js.map` 의 원본 확인). 넘겨도 빈 배열이 돌아온다.
- 신설 `readTerminalRowColumns(line)`: 행을 셀 단위로 훑어 `{ text, columns }` 를 만든다 — xterm 내부
  구현과 같은 규약(빈 셀은 공백, 다음 셀은 `getWidth() || 1` 칸 뒤, 코드유닛마다 열 1개 push)이라
  서로게이트 페어·결합 문자도 문자열 인덱스와 1:1 로 대응한다. range 는 `columns[startIndex] + 1` ~
  `columns[endIndex]`(= 매치 다음 셀의 0-based 열 = 마지막 셀의 1-based 열)로 잡는다 —
  단일폭 행에서는 기존 값과 동일해 기존 4케이스가 그대로 통과한다.
- 테스트 fake 를 문자열에서 **셀 배열**로 바꿨다(와이드 글리프 = 폭2 셀 + 폭0 continuation 셀).
  기존 단언은 손대지 않았고, 재현 케이스 2개(`'한글 src/a.ts:10 끝'` · `'🚀 a.ts'`)와
  `readTerminalRowColumns` 단위 3케이스를 더했다.

**C14 — 스폰 전 타이핑 유실 · 탭 활성 시 포커스 미이동 · 스폰 중 닫기 고아**

- **타이핑 유실**: `handleWrite` 가 `if (!sessionId) return` 으로 버렸다. 신설
  `pending-terminal-input.ts` 의 `appendPendingTerminalInput`(상한 4096자, 넘치면 앞에서부터 —
  `terminal-write-bridge` 큐 정책과 동일)로 모아 스폰 직후 한 번에 `pty_write` 한다. **스폰이 진행
  중일 때만** 모은다(플러시할 주체가 없는 입력은 그대로 버린다), 스폰 실패 시 버퍼를 비운다
  (`handleSpawnFailure` 로 중복 catch 2곳을 통합).
- **포커스**: `terminal-view.tsx` 가 `term.open`+`fit` 직후 `term.focus()`. 무조건이 아니라
  `autoFocus` prop 이 참일 때만이고, `pane-node-view.tsx` 가 `node.id === focusedPaneId` 를 내려준다 —
  세션 복원처럼 여러 pane 이 동시에 마운트될 때 배경 pane 의 터미널이 포커스된 pane 의 에디터와
  포커스를 다투지 않게 하기 위해서다(에디터는 `code-editor.tsx` 가 모델 부착 시 무조건 focus 한다).
  탭 클릭 경로는 mousedown 의 `focusPane` 이 click 의 `activateTab` 보다 먼저 정착하므로 마운트 시점에
  이미 포커스된 pane 이다.
- **스폰 중 닫기**: 언마운트만으로는 "탭 전환"과 "탭 닫기"를 구분할 수 없다(둘 다 언마운트다). 그런데
  `pty_spawn` 이 전역 mutation guard 를 쥐고 있어 `layout_close_tab` 은 pty 가 살아난 뒤에야 돌고,
  그때 탭에 기록된 `sessionId` 는 아직 비어 있어 아무 것도 회수되지 않는다. 그래서 백엔드 자신에게
  묻는다 — `layout_set_terminal_session` 은 탭이 사라졌으면 `NotFound` 로 실패한다
  (`run_layout_mutation` 의 `LayoutLocate::WithTab`). `settleSpawnedSession` 이 그 실패를 받고
  **언마운트 상태일 때만** `pty_kill` + 로스터에서 제거한다. 전환이면 persist 가 성공하므로 세션이
  보존된다(A6 의 재부착 근거이기도 하다). 마운트 중 persist 실패는 화면에 살아 있는 터미널이므로
  기존대로 삼킨다. 마운트 여부는 `isMountedRef`(StrictMode 재마운트 대비해 마운트에서 재무장).
- `mutate` → `mutateAsync` 로 바꿨다. v5 는 언마운트 후에도 mutation 자체와 그 옵션의 `onSuccess`
  (=`applyFreshLayout`)를 계속 실행하므로, 전환 중 스폰이 끝나도 layout 캐시에 세션이 기록된다.

**D5 — flow-control pause 중 detach 시 영구 pause (선검증 → 확증, 수정)**

- 확증(코드 실측): `pty_set_paused` 는 `PtySession` 이 소유한 **세션 단위** `PauseGate` 를 세우고
  (`infra/pty.rs`), reader 스레드는 `wait_while_paused` 에서 condvar 로 잠든다 — 깨우는 것은
  `set_paused(false)` 의 `notify_all` 뿐이다(자식이 죽어도 안 깨운다는 사실은 `Drop` 주석이 이미 명기).
  `pty_detach` 는 구독자 목록만 건드리고 pause 를 풀지 않는다. 프론트는 재마운트 때
  `flowStateRef.current = INITIAL_FLOW_CONTROL_STATE`(paused=false)로 **로컬만** 되돌리므로 백엔드가
  paused 인 줄 모르고 재개를 영영 보내지 않는다 → 그 세션은 출력이 멈춘 채로 남는다.
- 수정(`terminal-pane.tsx` attach 이펙트 양끝): ① cleanup 에서 `flowStateRef.current.paused` 면
  `onSetPaused(false)` — 버스트 도중 탭을 바꿔도 백그라운드 자식이 계속 진행한다. ② attach 시
  무조건 `onSetPaused(false)` 로 재동기 — cleanup 이 돌지 못한 경로(webview reload, 창 종료, 다른 창의
  구독 종료)까지 덮는다. 이미 running 인 pty 에 대한 resume 은 무해한 no-op 이다.
- 이펙트 실행 순서상 cleanup 시점의 `onSetPausedRef` 는 **직전 커밋의 핸들러**(=옛 sessionId)라
  재시작으로 sessionId 가 바뀌는 경우에도 옳은 세션을 향한다.

**변경 파일**

- 신규: `src/entities/terminal/terminal-session-cache.ts`(+`.test.ts`),
  `src/widgets/terminal-pane/pending-terminal-input.ts`(+`.test.ts`)
- 수정: `src/widgets/terminal-pane/{terminal-session.tsx,terminal-pane.tsx}`,
  `src/features/terminal/{terminal-view.tsx,terminal-file-link.ts,terminal-file-link.test.ts}`,
  `src/widgets/editor-area/pane-node-view.tsx`, `src/app/providers/ipc-sync-provider.tsx`,
  `src/shared/constants/{terminal.ts,query-key.ts,query-key.test.ts}`,
  `docs/features/terminal.md`, `docs/ipc-contract.md`
- IPC 커맨드·이벤트·타입 표면 변경 없음 → bindings 재생성·dispatch 등재·원격 정책 분류·로케일 추가
  모두 불요(새 문구 0). `docs/ipc-contract.md` 에는 표면이 아니라 **클라이언트 규약**(pause 는 detach
  로 풀리지 않으니 attach 시 재동기 · 로스터는 프론트가 직접 갱신) 2건을 명기했다.

**검증**: `bun run typecheck` 통과, `bun run lint` 0 error(경고 8건 — F2~F4 와 동일 목록, 신규 0),
`bun run format:check` 통과, `bun test` **1663 pass / 0 fail**(F5 추가 25건 포함).
Rust 무변경(`src-tauri` 미수정)이라 cargo 계열은 이 스테이지의 검증 대상이 아니다.

### F6 테마·설정·플러그인·아웃라인 (B5 · B6 · B12 · B15 · C12 · D6 · D7)

**B5 — 활성 테마 삭제 시 builtin fallback**

- 원인 재확인: `theme_delete` 는 파일만 지우고 `settings.theme_id` 를 손대지 않는다. 그래서 활성
  테마를 지우면 `theme_get_current` → `load_theme` 가 `NotFound` 로 실패하고, `ThemeProvider` 는
  `resolvedTheme` 없이 렌더한다 — `applyThemeVariables`·`applyShikiTheme` 가 아예 호출되지 않아
  CSS 변수도 monaco 테마도 적용되지 않는다(다음 실행은 오류 배너 + 하이라이트 전무).
- 신설 `entities/theme/theme-selection.ts`: `resolveThemeIdAfterDelete({deletedThemeId,
  deletedThemeType, activeThemeId})` 가 fallback builtin id 또는 `null` 을 돌려주는 단일 판정처.
  `theme-editor.tsx` 에 흩어져 있던 `builtinIdForType` 도 여기로 옮겨 `resolveBaseThemeId` 와 공용.
- 순서를 **전환 → 삭제**로 잡았다(`settings_set_theme` 은 대상 존재를 검증하므로 반대 순서면 실패한다
  — 삭제 후 전환은 그 사이 무테마 구간도 남긴다). 전환 실패 시 삭제하지 않는다.
- `followSystemTheme` 이 켜져 있으면 `settings_set_theme`(그 플래그를 끄는 명시적 선택 커맨드) 대신
  `settings_update` 패치로 `themeId` 만 고친다 — 표시 중인 테마가 아니므로 id 복구는 순수 부기이고,
  이 복구가 사용자를 시스템 추종에서 몰래 내리면 안 된다. (dangling id 를 그대로 두면 나중에 플래그를
  끄는 순간 같은 실패가 뒤늦게 터진다.)
- 추가 테스트: `theme-selection.test.ts` 4케이스(활성 dark/light 전환·비활성 무전환·활성 id 미확정).

**B6 — vsix 임포트 테마의 tokenColors·author·source 보존 + 미리보기 반영**

- 원인 재확인: `buildThemeFromDraft` 는 `version/id/name/type/extends/palette/colors/syntax/terminal`
  만 쓴다. `theme_save` 는 그 값을 파일에 **통째로** 덮어쓰므로, 임포트 테마를 열어 아무것도 바꾸지 않고
  저장만 해도 `tokenColors`(실제 구문 강조의 원본 진실)·`author`·`source` 가 파일에서 사라진다.
  이후 그 테마는 `build-shiki-theme.ts` 의 `fallbackFromSyntax`(31토큰 역생성) 경로로 떨어진다.
  임포트 테마는 `extends` 가 내장 dark/light(=`token_colors: None`)라 상속으로도 복구되지 않는다.
- `ThemeDraft` 에 `metadata: ThemeDraftMetadata`(= `Pick<ResolvedTheme, 'tokenColors'|'author'|
  'license'|'source'>`) 신설. `createThemeDraft` 는 미지정 시 전부 null(기존 호출·테스트 동작 불변),
  `buildThemeFromDraft` 가 네 필드를 그대로 싣는다.
- 무엇을 실을지는 `resolveThemeDraftMetadata(source, base)` 가 정한다 — `ResolvedTheme.tokenColors`
  는 base 상속분과 자기 것을 구분해 주지 않으므로(`resolve_token_colors`), **base 의 해석 결과와 다를
  때만** 싣는다. 번들 테마 복제는 `extends` 상속이 유지되고(원본이 갱신되면 따라간다), 임포트 테마는
  자기 규칙을 파일에 남긴다. 비교는 직렬화 동등성(같은 Rust 직렬화기가 만든 값이라 키 순서가 안정).
- 미리보기: `theme.query.ts` 의 `toResolvedThemeFromDraft` 가 `draft.metadata.tokenColors ??
  baseTokenColors` 를 쓰고 attribution 도 채운다 — 편집기를 여는 순간 창 전체가 base(=없음) 기준으로
  납작해지던 것이 실제 테마와 같아진다.
- 추가 테스트: `theme-draft.test.ts` 3케이스(임포트 메타데이터 왕복 보존·base 동일 시 미탑재·
  메타데이터 미지정 드래프트).

**B12 — 아웃라인·브레드크럼 심볼의 편집 후 stale**

- 원인 재확인: 두 소비처의 이펙트 deps 는 `[path, languageId, servers, projectId, project?.root]` 뿐이라
  `textDocument/documentSymbol` 은 파일당 사실상 1회만 나간다. 심볼 트리는 요청 시점 스냅샷이므로 한 글자만
  고쳐도 아웃라인·브레드크럼이 탭이 열려 있는 내내 옛 구조를 가리키고, 클릭하면 엉뚱한 줄로 점프한다.
- `loadDocumentSymbolsForPath` 에 선택 인자 `subscribeContentChange`/`refreshDelayMs`/`scheduler` 를
  더했다. 내부를 generation 기반으로 재구성해(이전 `cancelled` 불리언 → `runGeneration !== generation`)
  재요청이 직전 실행을 무효화하고 그 waiter 를 취소하도록 했다. 디바운스는 **trailing-only**
  `DOCUMENT_SYMBOL_REFRESH_DEBOUNCE_MS`(400ms) — 버스트의 첫 키 입력은 가장 쓸모없는 요청이라 leading
  edge 를 두지 않았다(`shiki-monaco.ts` 의 leading+trailing 과 다른 선택, 근거는 JSDoc).
- 구독원은 신설 `entities/editor/model-registry.ts::subscribeModelContentChange` — `monaco.editor.
  getModel` + `onDidCreateModel` 로 **아직 없는 모델·재생성된 모델**(retargetModel·탭 재개봉·peek 승격)
  까지 추종한다. 단순히 `getModel(path)?.onDidChangeContent` 를 걸면 그 순간의 모델에만 붙어 조용히 죽는다.
- 재요청이 최신 텍스트를 받는 근거: `use-lsp-session.ts` 가 **같은** `onDidChangeContent` 에서
  `didChange` 를 동기 전송한다(디바운스 없음). 팔레트 로더는 열 때 1회 조회라 대상에서 제외.
- 추가 테스트: `document-symbol-session-waiters.test.ts` 3케이스(주입 스케줄러로 재요청·버스트 합류·
  cleanup 시 타이머/구독 해제).

**B15 — 설정 저장 실패 무음 · 숫자 필드 blur**

- `useUpdateSettings`/`useSetThemeId` 에 공통 `onError` 신설(`settings.saveFailed` +
  `describeIpcError(error)` 설명줄). 앱의 모든 설정 쓰기가 이 두 훅을 지나므로 새 컨트롤이 실패 보고를
  잊을 수 없다. 중복 toast 방지를 위해 `settings-remote-section.tsx` 의 per-call `onError` 1건을
  제거했다(v5 는 훅·호출 양쪽 `onError` 를 모두 실행한다 — 로케일 키는 카탈로그에 남겨 둔다).
- `numeric-field.tsx`: blur 시 입력을 **저장값으로 되돌려 쓰고**(`event.currentTarget.value =
  String(value)`) 커밋 여부는 신설 `shared/lib/numeric-field-commit.ts::resolveNumericFieldCommit`
  가 정한다(NaN·clamp 결과가 저장값과 같으면 커밋 없음). 기존 코드는 clamp 한 값을 보내면서 화면은
  건드리지 않아 "쓰여 있는 값 ≠ 적용된 값" 이 남았고, 저장이 실패해도 입력한 숫자가 그대로 남아
  저장된 것처럼 보였다. 성공 시에는 기존 `key={value}` 리마운트가 새 값으로 갱신한다.
- 추가 테스트: `numeric-field-commit.test.ts` 4케이스.

**C12 — 프로바이더 전환 후 모델 미선택 오표시**

- `option-picker.tsx` 의 `?? options[0]` 폴백이 "선택 없음" 과 "첫 항목 선택" 을 같은 화면으로 만들었다.
  프로바이더 전환은 `aiModel: ''` 로 비우므로 모델 피커가 첫 모델을 선택된 것처럼 광고했고, 그 상태로
  자동완성을 켜면 사용자가 고른 적 없는 모델이 쓰인다. 폴백을 제거하고 선택지가 없을 때 `placeholder`
  (muted)를 표시한다 — `settings-ai-section.tsx` 의 모델 피커가 기존 키
  `settings.aiModelSelectPlaceholder` 를 넘긴다. 다른 소비처(설정 enum 3종·스니펫 언어)는 값이 항상
  옵션에 있어 무영향이라 placeholder 를 주지 않는다.

**D6 — 선검증 4건: 전건 확증, 전건 수정**

- **① 플러그인 제거 후 disposed highlighter 참조.** `@shikijs/monaco` 의 `shikiToMonaco` 는 로드된 언어
  마다 `setTokensProvider` 를 부르고 disposable 을 버린다(소스 확인). 문법이 줄어드는 `reinitShiki`
  (=그 언어를 기여하던 플러그인 제거)에서는 사라진 언어의 provider 가 재등록 대상이 아니라 그대로 남고,
  `reinitShiki` 가 곧 stale 하이라이터를 dispose 한다. monaco 는 언어 등록 해제가 없어 그 파일은 계속
  그 언어로 열리고, `getLanguage` 가 `ShikiError: Shiki instance has been disposed` 를 던진다
  (`@shikijs/primitive` 의 `ensureNotDisposed` 실측). 신설
  `shared/lib/shiki/tokens-provider-registry.ts::swapTokensProviderRegistrations` 가 `shikiToMonaco`
  실행 구간 동안 등록을 가로채 모으고 **새 등록 뒤** 직전 묶음을 dispose 한다 — monaco 의
  `TokenizationRegistry` dispose 는 동일성 검사를 하므로(소스 확인) 재등록된 언어는 새 provider 를
  유지하고 사라진 언어만 plain 토큰화로 떨어진다. 재생성이 실패해 `highlighter` 가 null 로 남는 경로도
  같이 닫았다(그 경우 stale 은 dispose 되는데 provider 는 전 언어가 그것을 붙들고 있었다).
  추가 테스트: `tokens-provider-registry.test.ts` 3케이스(monaco 레지스트리 동일성 검사 모사 포함).
- **② 플러그인 언어 스니펫 provider 부재.** provider 는 `'*'` 금지 규약 때문에 언어 id 별로 등록되는데
  등록 대상이 부팅 시 `TAIDE_LANGUAGE_IDS` 고정이라, 플러그인 언어에는 `<languageId>.json` 도 전역
  `.code-snippets` 의 `scope` 도 전혀 뜨지 않았다. `snippet-completion.ts` 를 단일 installation +
  요청된 플러그인 언어 집합 구조로 바꾸고 `registerSnippetCompletionsForLanguages` 를 신설,
  `registerPluginLanguages` 가 새로 등록한 id 를 넘긴다(두 호출자 모두 자동 적용, 부트스트랩 순서 무관).
  추가 테스트: `snippet-completion.test.ts` 3케이스.
- **③ 미완성 스니펫 무음 폐기.** `draftsToSnippetContent` 가 이름·prefix·body 중 하나라도 빈 항목을
  조용히 버린다 — 저장은 성공 toast 를 띄우고 그 스니펫만 사라진다(행은 재선택 전까지 화면에 남아
  손실이 보이지 않는다). `findIncompleteSnippetEntryDrafts` 로 **입력이 있는데 미완성인** 행만 잡아
  저장을 막고 `snippetEditor.incompleteEntryError` 로 알린다(완전히 빈 행은 잃을 것이 없으므로 제외 —
  막으면 방치한 행 때문에 저장이 영영 불가능해진다).
- **④ 드래프트 무경고 소실.** 스니펫 에디터는 파일 선택 시 렌더 중 `setDraftEntries(null)` 로, 설정
  복귀 시 언마운트로 편집을 버렸다. 테마 에디터도 닫기로 드래프트를 버리는데, 라이브 프리뷰 때문에
  화면은 이미 편집 결과를 보여주고 있어 손실이 더 보이지 않는다. 두 에디터 모두 확인 다이얼로그를
  붙였다 — 스니펫은 `hasUnsavedSnippetDraftChanges`(직렬화되지 않는 미완성 행도 변경으로 센다, 파일
  전환·닫기·새 파일 생성 3경로), 테마는 `serializeThemeDraftEdits` 로 로드 시점 서명과 비교
  (`create` 모드는 아직 어디에도 없으므로 항상 확인). 신규 로케일 `common.unsavedChangesTitle`/
  `unsavedChangesDescription`/`discardChanges` 를 두 에디터가 공용한다.
  추가 테스트: `snippet-draft.test.ts` 7케이스(③ 3 + ④ 4).

**D7 — sync "Pull Remote" 무피드백 (선검증 → 확증 위치 정정, 수정)**

- 설정 화면 경로(`settings-sync-section.tsx`)는 이미 성공·실패 toast 가 있다. 실제 무피드백 지점은
  **팔레트 커맨드**(`entities/sync/sync.commands.ts`)였다 — 충돌 toast 의 액션이
  `() => void downloadSync(true)` 로 결과를 버려, 성공해도 아무 표시가 없고 실패는 unhandled rejection
  으로만 남았다. `force` 여도 `RetryGistChanged`/`RetrySyncCompleted`(둘 다 "다시 시도" 를 요구하는
  오류)로 실패할 수 있어, 사용자는 설정이 교체됐는지 알 방법이 없었다.
- `runSyncDownload(force)` 로 매개변수화해 액션이 같은 함수로 재진입한다(성공·실패 보고 공유).
  재귀는 종료된다 — `sync_download` 는 `force` 가 false 일 때만 `conflict` 를 돌려준다(Rust
  `decide_download_apply` 실측).

**변경 파일**

- 신규: `src/entities/theme/theme-selection.ts`(+`.test.ts`),
  `src/shared/lib/numeric-field-commit.ts`(+`.test.ts`),
  `src/shared/lib/shiki/tokens-provider-registry.ts`(+`.test.ts`)
- 수정: `src/entities/theme/theme.query.ts`, `src/entities/settings/settings.query.ts`,
  `src/entities/sync/sync.commands.ts`, `src/entities/editor/model-registry.ts`,
  `src/shared/lib/{theme-draft.ts,theme-draft.test.ts,snippet-draft.ts,snippet-draft.test.ts,
  snippet-completion.ts,snippet-completion.test.ts}`,
  `src/shared/lib/lsp/{document-symbol-session-waiters.ts,document-symbol-session-waiters.test.ts}`,
  `src/shared/lib/monaco/register-plugin-languages.ts`, `src/shared/lib/shiki/shiki-monaco.ts`,
  `src/features/settings/{numeric-field.tsx,option-picker.tsx}`,
  `src/widgets/theme-editor/theme-editor.tsx`, `src/widgets/snippet-editor/snippet-editor.tsx`,
  `src/widgets/settings-view/{settings-ai-section.tsx,settings-remote-section.tsx}`,
  `src/widgets/outline-panel/outline-panel-container.tsx`,
  `src/widgets/editor-pane/breadcrumbs-bar.tsx`,
  `src-tauri/src/domain/locale/service.rs`(common 3 · settings 1 · snippetEditor 1 키 등록),
  `src-tauri/resources/locales/{ko,en,ja}.json`(신규 5키 × 3),
  `docs/theme-system.md`, `docs/features/{settings-ui.md,plugins.md,editor.md,lsp.md}`,
  `docs/ipc-contract.md`
- IPC 커맨드·이벤트·타입 표면 변경 없음 → bindings 재생성·dispatch 등재·원격 정책 분류 불요.
  로케일만 3언어 파리티로 5키 추가. `docs/ipc-contract.md` 에는 표면이 아니라 **클라이언트 규약**
  (sync 충돌 후 force 재시도도 결과를 보고한다) 1건을 명기했다.

**검증**: `bun run typecheck` 통과, `bun run lint` 0 error(경고 8건 — F2~F5 와 동일 목록, 신규 0),
`bun run format:check` 통과, `bun test` **1706 pass / 0 fail**(F6 추가 43건 포함).
Rust 는 `cargo fmt --all` 후 `cargo fmt --all -- --check` 통과, `cargo test --lib locale` **17 통과**
(`내장_3종_로케일은_같은_키_집합을_가진다` · `en_메시지의_모든_키는_required_message_keys에_포함된다`
포함, ko/en/ja 각 **939키**).

### F7 성능 묶음 (perf-1 · 2 · 5 · 7 · 8 · 11 · 12 · 13 · 14)

**perf-1 / 보고서 §1-1 — React.lazy 코드 스플리팅 (전후 dist 실측)**

- 분할 지점 5곳. ① `preview-pane.tsx` 의 문서 뷰어 4종(`PdfPreview`·`SpreadsheetPreview`·
  `PresentationPreview`·`HwpPreview`)을 `lazy` + 공용 `withPreviewChunkSuspense` 로 분리 —
  fallback 은 원시 바이트 대기 화면과 같은 빈 면이라 로딩 상태가 둘로 갈리지 않는다. 이미지·
  비디오·오디오·HTML 은 네이티브 요소 래퍼라 eager 유지. ② `pane-node-view.tsx` 의 `SettingsView`
  (설정 탭). ③ 신설 `widgets/welcome/welcome-container-lazy.ts` — `WelcomeContainer` 는
  `app-shell.tsx`(무프로젝트 화면)와 `pane-node-view.tsx`(welcome 탭) 두 곳에서 쓰여, `lazy()` 를
  각자 부르면 청크는 공유해도 로드 상태가 갈라지므로 래퍼를 한 곳에 둔다. ④ `settings-view.tsx`
  안에서 `ThemeEditor`·`SnippetEditor`(둘 다 조기 return 으로 화면 전체를 차지 — 설정 첫 페인트에
  필요 없다), `settings-plugins-section.tsx` 에서 `PluginManager`. ⑤
  `keybindings-runtime-provider.tsx` 의 `KeybindingsEditor` — 이 프로바이더는 모든 창에 상주하는데
  다이얼로그는 ⌘K ⌘S·팔레트·설정 버튼으로만 열린다. 열기 전까지 렌더하지 않도록
  `KEYBINDINGS_DIALOG_STATE`(`unmounted`→`open`/`closed`) 3상태를 뒀다 — 단순 boolean 으로
  `{open && ...}` 하면 닫는 순간 언마운트돼 Radix 닫힘 애니메이션이 잘린다.
- **전후 실측**(`bun run build`, `dist/index.html` 의 entry + `modulepreload` 합계 = 부팅 페이로드):

  | | 부팅 페이로드 | 청크 수 | 전체 JS |
  |---|---|---|---|
  | 전 | **6916.40 kB** | 4 | 20098.45 kB (127 청크) |
  | 후 | **5832.79 kB** | 19 | 20095.20 kB (157 청크) |
  | 차 | **−1083.61 kB (−15.7%)** | +15 | ±0 (재분할) |

  전 구성은 `index` 5270.09 + `standaloneServices` 1616.46 + `commands` 29.15 + runtime 0.70.
  후는 rolldown 이 monaco 를 `editor.api`·`toggleHighContrast` 등으로 재분할해 청크 수가 늘었지만
  합계가 줄었다. 새로 떨어져 나온 지연 청크: `pdf-preview` 467.5 · `spreadsheet-preview` 327.8 ·
  `hwp-preview` 88.3 · `presentation-preview` 5.5 · `settings-view` 88.0 · `plugin-manager` 32.0 ·
  `theme-editor` 28.7 · `snippet-editor` 14.4 · `keybindings-editor` 13.6 ·
  `welcome-container` 5.1 kB.
- 보고서가 §0 에서 제외한 **monaco 자체 지연**(`standaloneServices` 1616 kB + `editor.api` 980 kB
  + `toggleHighContrast` 1160 kB)은 손대지 않았다 — 부팅 페이로드의 대부분은 여전히 monaco 다.

**perf-2 / §1-2 — pdf 워커 지연 스폰**

- `shared/lib/pdf/setup.ts` 의 최상위 `GlobalWorkerOptions.workerPort = new PdfWorker()` 를
  `getPdfjsWithWorker()` 로 바꿨다. 최초 호출에서만 포트를 만들고(이후 같은 포트 재사용 — pdf.js 의
  `workerPort` 는 문서 단위가 아니라 장수명 공용 채널이다), 유일한 소비처인 `pdf-preview.tsx` 가
  문서 로딩 effect 안에서 부른다. perf-1 의 `pdf-preview` 청크 분리와 합쳐, PDF 를 한 번도 열지
  않는 세션은 `pdf.worker` 청크(1180.6 kB)를 받지도 스폰하지도 않는다.

**perf-5 / §1-5 — 리사이즈 커밋 디바운스 + layout 이벤트 자기 에코 억제**

- **선행 사실 정정**: 보고서가 적은 "pointermove 당 `resizePane` IPC" 는 현재 코드에 없다.
  `react-resizable-panels` 4.12 의 `onLayoutChanged` 는 드래그 중(전역 상태 `active`)에는 호출되지
  않고 포인터를 놓아야 발화한다(`dist` 소스 `dt = B().state !== "active"` 확인). 남은 홍수는
  **키보드 리사이즈**다 — 화살표 키는 매 입력마다 "완료된 레이아웃 변경"으로 보고되므로 키를 누르고
  있으면 오토리핏 1회당 `layout_resize_pane` 1회(앱 전역 뮤테이션 가드 + `layout.json` 재기록 +
  `layout:changed` 발행)가 나간다.
- 신설 `entities/layout/pane-resize-commit.ts` 의 `schedulePaneResizeCommit(paneKey, run)` —
  `paneKey` 별 trailing 전용 디바운스(`PANE_RESIZE_COMMIT_DEBOUNCE_MS` 120ms, leading edge 없음:
  리사이즈 중간 폭은 저장할 가치가 없다). 타이머를 `PaneNodeView` 의 ref 가 아니라 모듈 상태에 둔
  이유는 레이아웃 리페치가 매 입력마다 node 객체를 갈아 끼워도 커밋이 살아남아야 하고, 같은 pane 에
  경쟁하는 타이머가 둘 생기면 안 되기 때문이다. `run` 은 호출마다 새로 받으므로 발화 시점의 최신
  뮤테이션 바인딩을 쓴다. 키는 `${projectId}:${paneId}`.
- **자기 에코 억제**: `ipc-sync-provider.tsx` 의 `layout:changed` 핸들러에 신설 순수 함수
  `isLayoutEchoAlreadyInCache(cachedRevision, eventRevision)` 를 추가했다. 기존
  `lastLayoutRevisionByProjectRef` 게이트는 **이벤트로 배달된** revision 만 기억하므로, 뮤테이션이
  응답으로 이미 받아 `applyFreshLayout` 으로 캐시에 쓴 레이아웃의 에코가 항상 "새 것"으로 보여
  `layout_get` 왕복을 한 번 더 냈다. 캐시 revision 이 이벤트 revision 이상이면 무효화를 건너뛴다 —
  타 창의 변경이나 아직 도착하지 않은 자기 응답은 캐시가 뒤처져 있어 종전대로 리페치된다. 이 부수
  리페치는 리사이즈만이 아니라 탭 열기·닫기·활성화·이동·핀·dirty 토글 등 모든 레이아웃 뮤테이션에
  1회씩 붙어 있었다.
- 추가 테스트: `pane-resize-commit.test.ts` 5케이스(trailing 전용·연속 호출 1회 병합·키 분리·만료
  후 취소 없음·기본 지연), `ipc-sync-provider.test.ts` 의 `isLayoutEchoAlreadyInCache` 4케이스.

**perf-7 / §1-7 — shiki grammar 온디맨드**

- `lang-map.ts`: `TAIDE_CORE_LANGUAGE_IDS`(`json`·`jsonc`·`markdown`) · `isTaideLanguageId` 타입
  가드 신설, `loadAllTaideGrammars()` → `loadTaideGrammars(ids)`. 코어 3종은 **앱이 사용자 동작 없이
  스스로 여는** 표면(`settings.json`/`keybindings.json` 앱파일 탭, 마크다운)이라 지연해도 곧바로
  재토큰화를 부를 뿐이다.
- `shiki-monaco.ts`: 모듈 상태 `requestedLanguageIds`(코어 + 이후 요구된 언어)를 두고
  `createConfiguredHighlighter` 가 그 집합으로 빌드한다 — `reinitShiki`(플러그인 재구성)가 열려
  있는 파일의 문법을 코어 3종으로 되돌리지 않는다. 신설 `ensureShikiLanguage(languageId)` 가
  `highlighter.loadLanguage(...)` + tokens provider 재부착을 `runExclusive` 큐에서 수행한다(큐 안에서
  `getLoadedLanguages()` 를 재확인 — id 기록 이후에 시작된 빌드가 이미 실었다면 재로드+전량 재부착은
  순수 낭비다).
- 요구 시점은 `observeModelLanguages()` — `monaco.editor.onDidCreateModel` + 각 모델의
  `onDidChangeLanguage`, 그리고 부팅 시 이미 존재하는 모델을 `getModels()` 로 일괄 훑는다.
  `model-registry.ts` 를 후킹하지 않은 이유는 diff 한쪽·peek 프리로드 등 모델을 만드는 경로가 그
  레지스트리 밖에도 있기 때문이다. `initShiki` 가 (하이라이터 유무와 무관하게) 이 관찰자를 무장한다.
- 플러그인 접점 정합(F6 이 만진 현재 코드 기준): `sanitizePluginGrammarEmbeddedLangs` 는 미로드
  언어를 `embeddedLangs` 에서 떨어뜨리므로, 코어만 싣는 구성에서는 플러그인 문법의 임베딩이 조용히
  깨질 수 있었다. `rememberTaideLanguagesEmbeddedByPluginGrammars` 가 빌드 직전에 플러그인 grammar 의
  `embeddedLangs` 중 TAIDE id 를 요구 집합에 합류시켜 이를 막는다. F6 의
  `swapTokensProviderRegistrations`·`disposeTokensProviderRegistrations` 경로는 그대로 재사용한다
  (온디맨드 부착도 같은 swap 을 지난다).
- **실측**: 부팅 시 받아 파싱하던 grammar 청크 30개 합계 **2266.4 kB**(`cpp` 단독 778.3 kB) →
  코어 3개 **63.7 kB**(−97%). 부팅 *페이로드*(modulepreload) 수치에는 잡히지 않는다 — 이들은 원래도
  동적 import 였고, 줄어든 것은 부팅 시 실제로 받아 파싱하는 양이다.
- 추가 테스트: `lang-map.test.ts` 7케이스(코어가 전체의 진부분집합·무거운 grammar 비포함 ·
  `isTaideLanguageId` 2 · `loadTaideGrammars` 3 — 그중 하나는 **id 하나의 grammar 묶음이 자기
  `embeddedLangs` 를 스스로 해소한다**는 언어 단위 로드의 전제를 31언어 전수로 고정한다).

**perf-8 / §1-8 — git 이벤트 무효화에 `isGitQueryScopeMutable` 적용**

- `ipc-sync-provider.tsx` 의 `git:status-changed`·`git:refs-changed` 두 핸들러가
  `QUERY_KEY.GIT.PROJECT` 접두사에 `predicate: (query) => isGitQueryScopeMutable(query.queryKey)`
  를 AND 로 결합한다(뮤테이션 `useGitMutation`·`useSaveGitDiffText` 가 이미 쓰는 그 술어를
  `entities/git/git.query.ts` 에서 그대로 가져다 쓴다 — 사본을 만들지 않았다).
- 이로써 `REV_IMMUTABLE_SCOPES`(commit-files·show — 불변 SHA 키, `staleTime: Infinity`) 보장이
  **뮤테이션 경로에서는 지켜지고 그 이벤트 에코에서 무너지던** 비일관성이 닫힌다. 커밋 상세·파일
  히스토리 패널이 열려 있는 동안의 스테이지/언스테이지/스태시/커밋마다 블롭을 다시 받지 않는다.

**perf-11 / §1-11 — 워크스페이스 심볼 검색 디바운스**

- `use-workspace-symbol-search.ts` 의 effect 가 `WORKSPACE_SYMBOL_SEARCH_DEBOUNCE_MS`(200ms) trailing
  타이머 뒤에 `load()` 를 돌린다. 이 요청은 프로젝트의 **준비된 LSP 세션 전부**로 팬아웃해 각 서버가
  자기 인덱스를 훑는, 팔레트가 낼 수 있는 가장 비싼 요청이다.
- 빈 질의(공백만)는 디바운스 없이 즉시 결과를 비운다(early return) — 지우는 동작까지 늦출 이유가
  없다. cleanup 은 `clearTimeout` + 기존 `workspaceSymbolSearch.cancel()` 을 함께 수행한다.
- 값 판단은 F6 의 이월 기록대로 이 스테이지 몫이다: 문서 심볼 재요청(400ms, 편집 중 배경 갱신)보다
  짧게 잡았다 — 이쪽은 사용자가 결과를 기다리며 타이핑하는 전경 동작이다.
  `DOCUMENT_SYMBOL_REFRESH_DEBOUNCE_MS` 는 F6 값 그대로 두었다(축이 다르고 실측 근거도 없다).

**perf-12 / §1-12 — Problems 패널 가상화**

- 신설 `features/problems/problem-list-rows.ts`: `buildProblemListRows(groups, collapsedPaths)` 가
  그룹 헤더 + 문제 행을 한 목록으로 평탄화(접힌 그룹은 헤더만), `PROBLEM_LIST_ROW_HEIGHT_PX`(20)이
  두 행 종류의 공통 높이다(둘 다 `text-xs` 16px 라인 박스 + `py-0.5`; 메시지는 `truncate` 라 줄바꿈
  없음). `ProblemGroup` 타입도 여기로 옮겨 `problems-panel.tsx` ↔ 신설 모듈의 타입 순환을 피했다
  (소비처 `problems-panel-container.tsx` 의 import 1줄 변경).
- `problems-panel.tsx` 가 `ScrollContainer` 대신 자체 스크롤 뷰포트 + `useVirtualizer` +
  `OverlayScrollbar` 를 소유한다(`search-results-list.tsx`·`file-tree.tsx`·`commit-graph.tsx` 와
  같은 패턴). `getItemKey` 는 **그룹 내 인덱스** 기반이다 — 두 서버가 같은 위치·같은 메시지를
  보고하면 내용 기반 key 가 충돌한다. 파일 내 로컬 `toggleInSet` 사본은 `@shared/lib/set` 의 공용
  함수로 교체했다(같은 패턴의 다른 가상 목록과 동일).
- 마커 수집 자체(`use-monaco-markers.ts` 의 전량 스냅샷)는 그대로다 — 보고서 §1-12 의 "마커 전체
  재수집" 은 §5 에 이월 사유를 적었다.
- 추가 테스트: `problem-list-rows.test.ts` 5케이스(평탄화 순서·접힘·다중 그룹 id 순서·동일 내용
  중복 시 key 유일성·빈 입력).

**perf-13 / §1-13 — `fs:changed` 배치 무효화 통합**

- `filePathQueryKeysToInvalidate(path)` 순회(경로당 `invalidateQueries` 2회)를 신설 술어
  `isFilePathQueryForChangedPaths(queryKey, changedPaths)` + `QUERY_KEY.FILE.ALL` 접두사 1회
  호출로 대체했다. `invalidateQueries` 한 번이 캐시 전체를 훑고 notify 하므로, 브랜치 전환·
  `npm install`·프로젝트 전역 치환 같은 대형 배치에서는 그 전수 스캔이 `paths.length × 2` 회 돌고
  있었다.
- 스코프 문자열은 `QUERY_KEY.FILE.CONTENT('')[1]` / `RAW('')[1]` 로 **키 팩토리에서 유도**해, 리프
  이름이 바뀌면 문자열 리터럴이 조용히 어긋나는 대신 타입 에러가 난다. 같은 `FILE.ALL` 접두사 아래
  있지만 `ProjectId` 로 키잉된 `MIRRORS`/`UNTITLED_MIRRORS` 는 스코프 검사에서 배제된다(핫엑시트
  미러가 워처 에코로 날아가면 안 된다). `changedPaths` Set 은 git 술어와 공유한다.
- 추가 테스트: `ipc-sync-provider.test.ts` 4케이스(CONTENT·RAW 매치 / 미포함 경로 / MIRRORS·
  UNTITLED_MIRRORS 비매치 / path 비문자열). 기존 `filePathQueryKeysToInvalidate` describe 는 대체됐다.

**perf-14 / §1-14 — 팔레트 상시 렌더 비용**

- **커맨드 목록 스냅샷 승격**: `command-registry.ts` 의 `listRegisteredCommands()` 가 매 호출
  `Array.from(...)` 로 새 배열을 만들어, React Compiler 가 그 위에 쌓인 계산을 전혀 메모할 수 없었다.
  모든 쓰기(`registerCommand`/`registerCommands`/`unregisterCommand`/`clearCommandRegistry`)가
  `publishCommandsSnapshot()` 을 지나 스냅샷 배열을 갱신·통지하고, `subscribeRegisteredCommands` 를
  더해 팔레트가 `useSyncExternalStore` 로 읽는다. 레지스트리는 부팅에서만 바뀌므로 이 참조는 앱
  수명 동안 안정적이다. (`registerCommands` 는 루프 후 1회만 발행하도록 바꿨다.)
- **모드 분기 내 계산 이동**: 한 번에 한 그룹만 화면에 있는데 `filteredFiles`(프로젝트 전 파일 퍼지
  스캔)·`filteredCommands`(전 카탈로그 퍼지 스캔 + 커맨드마다 `t()` 라벨 포맷)·문서 심볼 평탄화가
  **팔레트가 닫혀 있을 때까지 포함해** 매 렌더 전부 돌았다. 각각을 자기 모드로 게이트했다
  (`mode === 'files'` / `'commands'` / `'symbol'` / `'workspaceSymbol'` / `'line'`).
  `commandKeybindingRows` 는 keydown 캡처 리스너가 항상 필요로 하므로 게이트하지 않고, 위 스냅샷
  덕에 메모 대상이 된다.

**변경 파일**

- 신규: `src/widgets/welcome/welcome-container-lazy.ts`,
  `src/entities/layout/pane-resize-commit.ts`(+`.test.ts`),
  `src/features/problems/problem-list-rows.ts`(+`.test.ts`)
- 수정: `src/app/providers/{ipc-sync-provider.tsx,ipc-sync-provider.test.ts,
  keybindings-runtime-provider.tsx}`, `src/shared/lib/pdf/setup.ts`,
  `src/shared/lib/shiki/{lang-map.ts,lang-map.test.ts,shiki-monaco.ts}`,
  `src/shared/lib/command-registry.ts`, `src/features/preview/pdf-preview.tsx`,
  `src/features/problems/problems-panel.tsx`, `src/widgets/preview-pane/preview-pane.tsx`,
  `src/widgets/editor-area/pane-node-view.tsx`, `src/widgets/app-shell/app-shell.tsx`,
  `src/widgets/settings-view/{settings-view.tsx,settings-plugins-section.tsx}`,
  `src/widgets/problems-panel/problems-panel-container.tsx`,
  `src/widgets/command-palette/{command-palette.tsx,use-workspace-symbol-search.ts}`,
  `docs/theme-system.md`, `docs/features/{preview.md,plugins.md,git.md}`, `docs/ipc-contract.md`
- IPC 커맨드·이벤트·타입 표면 변경 없음 → bindings 재생성·dispatch 등재·원격 정책 분류 불요.
  신규 로케일 키 0(추가한 UI 문자열이 없다 — Suspense fallback 은 전부 빈 면/`null`). `ipc-contract.md`
  에는 표면이 아니라 **클라이언트 규약**(`layout:changed` 자기 에코를 캐시 revision 으로 억제한다)
  1건을 `layout:changed` 절에 명기했다.

**검증**: `bun run typecheck` 통과, `bun run lint` 0 error(경고 9건 — F2~F6 의 8건 + 신규 1건은
`problems-panel.tsx` 의 `useVirtualizer` "Compilation Skipped: Use of incompatible library" 로,
`file-tree`·`search-results-list`·`commit-graph` 3개 기존 가상 목록과 동일 클래스),
`bun run format:check` 통과, `bun test` **1730 pass / 0 fail**(F7 추가 24건 포함),
`bun run build` 성공(위 전후 실측 표). Rust 변경 없음.

### F8 IME 공통 가드 (B13)

**공통 가드 — `shared/lib/ime-composition.ts` 신설 (1곳)**

- `isImeCompositionKeydown(event)` = `isComposing || nativeEvent?.isComposing || keyCode === 229`
  (`IME_COMPOSITION_KEY_CODE`). `docs/bug/2026-08-06-wkwebview-ime-composition.md` 의 실측표가
  근거다 — 이 앱의 WKWebView 는 `compositionstart/update/end` 를 아예 발생시키지 않아
  `KeyboardEvent.isComposing` 이 **영구히 false** 인 반면 `keyCode` 는 Safari 와 동일하게 229 를
  보고한다. 그래서 기존에 `event.isComposing` 으로만 쓴 가드는 실기에서 전부 죽은 코드였다.
  cmdk 가 자기 `Command` 루트에 쓰는 판정(`e.nativeEvent.isComposing || e.keyCode === 229`,
  `node_modules/cmdk/dist/index.mjs`)과 같은 식이며, 소스를 열어 확인하고 맞췄다.
- 파라미터 타입은 네이티브 `KeyboardEvent` 와 React 합성 이벤트를 **한 함수로** 받도록 구조적으로
  정의했다(`isComposing?` · `keyCode?` · `nativeEvent?.isComposing?`) — React 합성 키보드 이벤트는
  `keyCode` 는 갖지만 `isComposing` 은 `nativeEvent` 로만 노출하기 때문이다(`@types/react` 의
  `KeyboardEvent` 확인). 판정 순서는 `isComposing` 계열이 먼저 — 훗날 WKWebView 가 고쳐지거나
  브라우저 기반 dev/e2e 로 돌 때도 deprecated `keyCode` 에 의존하지 않고 성립한다.

**적용 ① 키맵 — `keymap-dispatch.ts` (chord 대기·유예에 더해 단일 키 매칭까지)**

- 보고서는 "chord 대기" 만 지목했지만, 실제 노출면은 더 넓었다. `normalizeKeymapEventKey` 는
  `event.key` 가 깨끗한 한 글자가 아니면(조합 중에는 `Process`) **`event.code` 로 폴백**하므로,
  한글을 한 글자 칠 때마다 그 물리 키에 걸린 **수식어 없는 바인딩**(사용자 재바인딩으로 얼마든지
  생긴다)이 정상 발동한다. 그래서 `isIgnorableKeydown`(대기 소비 제외) 안이 아니라
  `decideKeymapDispatch` **진입부**에 가드를 두어 단일 키 매칭·chord 1단 진입·2단 해소·monaco 유예
  소비를 한 번에 차단했다. 반환은 기존 `ignore-modifier-only` 를 재사용한다 — 호출부
  (`use-global-keymap.ts`)가 이 액션에 대해 아무 부수효과도 내지 않고, `decideCommandBindingRun`
  이 "`none` 이 아니면 커맨드 바인딩은 물러난다" 로 읽으므로 **커맨드 바인딩 리스너도 같이** 물러난다
  (팔레트의 두 번째 window 캡처 리스너).
- `isIgnorableKeydown` 에서는 `isComposing` 항을 제거했다(진입부 가드가 상위 집합이라 죽은 조건).
  `KeymapEvent` 에 `keyCode?: number` 를 추가했다 — 키 매칭에는 쓰지 않고 오직 이 가드가 읽는다는
  사실을 JSDoc 에 못박았다.

**적용 ② Enter 실행 지점**

- `features/search/search-panel.tsx` — 검색 패널 쿼리 입력의 Enter 실행.
- `widgets/search-editor/search-editor-pane.tsx` — Search Editor 쿼리 입력의 Enter 실행.
- `features/explorer/file-tree-draft-row.tsx` — 인라인 생성·개명 draft row 의 **Enter 확정과
  Escape 취소 둘 다**(조합 확정용 Enter 로 파일이 만들어지거나, 조합 취소 Escape 로 입력이
  통째로 사라지던 자리).
- `features/git/commit-box.tsx` — 커밋 메시지 ⌘Enter 커밋(한글 본문이 가장 흔한 입력면).
- `features/settings/remote-allowed-hosts-row.tsx` — 허용 호스트 Enter 추가.

**적용 ③ Escape 이탈 지점**

- `features/editor/ai-inline-edit.ts` — 기존 `if (event.isComposing) return` 을 공통 가드로 교체.
  이 자리가 B13 이 말한 "가드 전면 사망" 의 문자 그대로의 사례다(프롬프트 입력창이라 한글 입력이
  잦은데, Escape 취소·Enter 제출·⌘Enter 수락이 전부 조합 중에 발동했다).
- `widgets/app-shell/use-zen-mode.ts` — Zen 모드 Escape 이탈. 이 리스너는 `defaultPrevented` 로
  선행 처리를 감지하는데, 조합을 취소하는 Escape 는 입력기가 삼킬 뿐 `preventDefault` 를 남기지
  않으므로 그 방어가 통하지 않았다.
- `shared/ui/dialog.tsx` · `shared/ui/popover.tsx` — radix 의 `onEscapeKeyDown` 을 감싸
  조합 중이면 `preventDefault()` 로 닫힘을 거부하고 호출부 핸들러도 건너뛴다. radix 자체에는 조합
  가드가 없다(`@radix-ui/react-use-escape-keydown` 은 `event.key === 'Escape'` 만 본다 — 소스 확인).
  이 두 곳으로 커맨드 팔레트 질의·태그 다이얼로그 이름·키바인딩 검색·태스크 러너 질의(Dialog)와
  폰트 검색·브랜치 필터(Popover)가 한꺼번에 덮인다.

**전수 grep 과 제외 판정** — `onKeyDown`/`addEventListener('keydown')`/`'Enter'`/`'Escape'`/
`isComposing`/`keyCode` 전량을 훑었다. 아래는 **조합이 시작될 수 없는 대상**(비편집 요소)이라
의도적으로 손대지 않았다: `file-tree.tsx` 컨테이너(`role=tree` div — draft row 가 자기 keydown 을
`stopPropagation` 하므로 입력 이벤트가 올라오지도 않는다), `activation-key.ts` 계열 row 활성화
(`role=button` div + `target !== currentTarget` 선행 차단), 키 캡처 UI 2곳
(`keybinding-row.tsx`·`keybindings-editor.tsx` — 캡처 대상이 `<button>`),
`option-picker`/`font-picker`/`language-picker` 의 트리거 ArrowDown, `diff-pane.tsx` 의 Alt 조합,
`color-picker` 의 화살표. cmdk 루트(`Command`)는 자체 229 가드를 이미 갖고 있어 중복 적용하지 않았다.

**추가 테스트 (12건)**

- `shared/lib/ime-composition.test.ts` 7건 — WKWebView 형태(`isComposing:false` + `keyCode:229`)
  판정, Safari 형태, `keyCode` 없는 환경, React 합성(`nativeEvent.isComposing`), 일반 키,
  **조합 확정 뒤의 맨 Enter(`keyCode:13`)는 false**(제출이 막히면 안 된다), 빈 이벤트.
- `keymap-dispatch.test.ts` 5건(신규 describe) — 수식어 없는 단일 키 바인딩이 조합 keydown 으로
  발동하지 않음 / **같은 키의 비조합 keydown 은 그대로 발동**(가드가 정상 입력을 막지 않음) /
  chord 2단 미소비 / monaco 유예 미소비 / chord 1단 미진입. 수정 전 5건 중 4건이 실패하는 것을
  확인한 뒤 고쳤다(나머지 1건은 회귀 방지용 대조군이라 처음부터 통과).

**문서** — `docs/features/keymap.md` §4 의 `isIgnorableKeydown` 항목을 실제 구조(진입부 가드 +
`event.code` 폴백 때문에 단일 키 매칭까지 포함)로 갱신했고, `docs/bug/2026-08-06-...md` 에
"2026-08-29 후속" 절을 붙여 실측표 → 공통 가드 → 적용 지점 → 검증 방법을 잇는 흐름을 남겼다.

**검증**: `bun run typecheck` 통과, `bun run typecheck:e2e` 통과, `bun run lint` 0 error(경고 9건 —
F7 시점과 동일, 신규 0), `bun run format:check` 통과, `bun test` **1742 pass / 0 fail**
(F8 추가 12건 포함). IPC 표면·로케일 키 변경 없음, Rust 변경 없음.

## §4 검토·수정 기록

### 렌즈 검토 판정표 (2026-08-29)

> 렌즈 A(데이터 안전·상태 일관성) · B(Rust 커맨드·계약) · C(프론트 UX) 의 발견 중 **이 계약 소관**
> 분. major 는 실물 코드로 적대적 재검증(반증 시도) 후 확증분만 수정했고, minor 는 비용 대비 명백한
> 것만 손댔다. Rust·S8 접점 발견은 d-50 §4 에 있다.

| # | 렌즈 | 발견 | 판정 | 조치 |
|---|---|---|---|---|
| 1 | C major (= A minor) | perf-9 의 지연 draft 리더가 dispose 된 monaco 모델을 붙들어 탭 닫기·개명 시 예외 → `unhandledrejection` 이 앱 에러 로그를 오염 | **확증** | `readDraftSafely` 신설 + flush 순서 교정 + 충돌 재파싱 타이머 가드 (아래 (1)) |
| 2 | A minor (= C minor) | read-only(대용량·lossy) 파일에서 ⌘S 가 아무 피드백 없이 무시됨 | **확증** | 명시적 저장에 한해 토스트, 로케일 키 1종 순증 ko/en/ja (아래 (2)) |
| 3 | A minor | `persistMirror` 의 `FILE.MIRRORS` seed 가 같은 배치가 두 곳에서 회피한 "미조회 쿼리 부분 seed" 를 그대로 함 | **확증**(영향은 작음, 일관성 결함) | 업데이터가 `previous` 미조회 시 `undefined` 반환 (아래 (3)) |
| 4 | C minor | 팔레트 `closedByActionRef` 가 `onCloseAutoFocus` 에서만 리셋 → 재열기 시 잔류 | **확증** | 여는 모든 경로에서 무장 해제 (아래 (4)) |
| 5 | C minor | 테마 에디터 create 모드가 편집 없이도 항상 discard 확인 | **확증** | 로드 직후 서명을 create 에서도 세움 (아래 (5)) |
| 6 | C minor | IME 가드가 `decideKeymapDispatch` 진입부에 있어 수식키 조합 단축키까지 차단 | **확증(위험) · 범위 축소로 수정** | Cmd/Ctrl 조합은 가드 예외 (아래 (6)) |
| 7 | C minor | 검색 결과·Problems 가상화로 뷰포트 밖 행의 Tab 도달성 상실 | **확증 · 이월** | §5 F7 기록 + `docs/backlog.md` 등재 (로빙 포커스는 기능 규모) |
| 8 | C minor | D3 도입으로 `find` ↔ `monaco.actions.find` 가 기본 설치 상태에서 상시 충돌 경고 | **확증 · 이월** | §5 F4 기록 유지 + `docs/backlog.md` 에 "기본 상태 경고 0" 목표로 등재 |
| 9 | C minor | `NumericField` 가 blur 시 저장값으로 되돌린 뒤 성공 후 다시 튀는 시각적 왕복 | **기각** | 아래 (7) — B15 가 고친 실패 모드를 되살리는 교환이라 현행 유지 |

### (1) dispose 된 모델을 가리키는 draft 리더

파일: `src/widgets/editor-pane/{editor-draft-sync.ts,editor-draft-sync.test.ts,
use-editor-file-persistence.ts,use-editor-git-gutter-and-conflicts.ts}`.

- 적대적 재검증에서 두 경로 모두 성립했다. F1(perf-9)이 `draftRef` 를 문자열에서
  `() => model.getValue()` 리더로 바꿨고, d-50 S8 이 같은 배치에서 **모델 dispose 를 새로 도입**했다
  (`releaseClosedFileTabPath` → `disposeModel`, `retargetModel` → 옛 모델 `dispose`). 둘 다 **동기**로
  일어나므로 리더가 자기 모델보다 오래 산다.
  - 닫기: `useCloseTab.onSuccess` 가 `releaseClosedFileTabPath` → `applyFreshLayout` 순서라 dispose 가
    언마운트보다 먼저다. 이어지는 미러 flush 이펙트 cleanup 의 `void flush()` 가 `readDraft()` 를
    부르고, monaco 가 `BugIndicatingError('Model is disposed!')` 를 던진다. `flush` 는 async 라
    unhandled rejection 이 되고 `shared/lib/error-log-forwarding.ts` 가 그것을 **앱 로그에 error 로**
    기록한다. 조건은 "세션 중 한 번이라도 타이핑한 파일 탭을 닫는다" 뿐이다.
  - 개명: `EditorPane` 은 `key={activeTab.id}` 라 언마운트되지 않지만, `path` 가 바뀌면 같은 이펙트의
    deps 가 바뀌어 cleanup 이 돌며 같은 throw 를 낸다.
- 데이터 손실은 아니라는 발견의 판단도 확인했다(닫기 경로는 `clearMirror` 를, 개명 경로는
  `migrateMirror` 를 이미 발행했다). 그러나 **안전망이 설계가 아니라 순서의 우연에 기대는 상태**이고,
  진단용 에러 로그가 정상 조작으로 오염되는 것은 그 자체로 결함이다.
- 조치: 순수 모듈 `editor-draft-sync.ts` 에 **`readDraftSafely(reader)`** 를 두어 "리더는 자기 모델보다
  오래 살 수 있고, 그때 읽으면 draft 가 **없는** 것" 이라는 불변식을 명시적 계약으로 만들었다. 훅의
  `readDraft` 와 `handleChange` 의 두 디바운스 타이머(미러 쓰기·마크다운 프리뷰)가 전부 이것을 지난다.
  또 `flush` 를 `pendingMirrorRef` 검사 → 리더 호출 순으로 뒤집어, 쓸 것이 없는 대다수 경우에는 리더를
  아예 부르지 않는다.
- 같은 축의 파생 1건: `use-editor-git-gutter-and-conflicts.ts` 의 150ms 충돌 재파싱 타이머는 이펙트
  시점에 캡처한 `model` 을 읽으므로 개명 직후(dispose ~ cleanup 사이)에 같은 throw 를 낼 수 있다 —
  콜백에 `model.isDisposed()` 조기 반환을 넣고 근거를 그 이펙트 JSDoc 에 적었다.
- 테스트: `editor-draft-sync.test.ts` 3케이스(리더 없음 / 살아 있는 리더 / throw 하는 리더 → null).

### (2) read-only 파일의 ⌘S 무음

파일: `src/widgets/editor-pane/{use-editor-file-persistence.ts,editor-pane.tsx}` ·
`src-tauri/resources/locales/{ko,en,ja}.json` · `src-tauri/src/domain/locale/service.rs`.

- `handleSave` 의 `if (file?.readOnly) return` 이 실제로 도달하는 경우는 monaco 가 입력을 막는 상태에서
  draft 가 존재하는 때 — 핫엑시트 미러 복원(`applyMirrorRestore`) 또는 백그라운드 모델 편집 인수
  (`adoptUnobservedModelEdit`)로 dirty 가 된 read-only 탭이다. 사용자에게는 dirty 점이 붙은 탭에서
  ⌘S 가 **아무 반응 없이** 무시되는 것으로만 보인다. 같은 배치가 B15(설정 저장 실패 전면 무음)를 결함
  으로 잡은 것과 같은 축이다.
- 조치: **명시적 저장(⌘S)일 때만** `toast.error(t('editor.readOnlySaveBlocked'))`. 자동 저장은 사용자가
  누른 것이 아니므로 조용히 둔다. 훅에 `t` 를 주입하는 방식은 같은 위젯의 `useEditorBlame` 선례를 따랐다.
- 로케일 키 **1종 순증**(`editor.readOnlySaveBlocked`) — ko/en/ja 3언어 + `MESSAGE_NAMESPACES` 등재,
  파리티 테스트 통과. 사유 설명은 이미 떠 있는 상단 배너(`editor.readOnlyLossyEncoding` /
  `readOnlyLargeFile`)가 맡으므로 토스트는 "저장하지 않았다" 만 말한다.

### (3) `persistMirror` 의 부분 목록 seed

`(previous ?? []).filter(...)` 로 미조회 상태에서도 1건짜리 배열을 심던 것을, `previous` 가 없으면
`undefined`(= TanStack 의 "이 쿼리는 그대로 둬라")를 돌려주도록 맞췄다. 같은 파일의
`settleDraftToDiskContent` 와 `entities/layout/tab-path-change.ts` 의 `rewriteMirrorCache` 가 이미
같은 이유(`staleTime: Infinity` 쿼리에 부분 목록을 심으면 "이 프로젝트에는 미러가 없다"로 굳는다)로
그렇게 하고 있었고, 세 번째 자리만 예외였다. 백엔드에는 이미 기록돼 있으므로 첫 조회가 그 항목을
가져온다.

### (4) 팔레트 `closedByActionRef` 잔류

액션으로 닫힌 직후 닫힘 애니메이션이 끝나기 전에 ⌘P 를 다시 누르면 Radix 의 `onCloseAutoFocus` 가
발화하지 않은 채 `open` 이 true 로 돌아가 플래그가 true 로 남고, **다음** Escape/바깥 클릭 닫힘에서
`preventDefault` 가 잘못 걸려 포커스가 body 로 떨어진다(D1 이 피하려던 바로 그 상태). 여는 경로가
`setOpen(true)` 직접 호출 3곳이었으므로 `openPalette(query)` 로 모아 그 안에서 무장 해제하고,
`handleOpenChange(true)`(Radix 가 여는 경우)에도 같은 리셋을 뒀다.

### (5) 테마 에디터 create 모드의 상시 discard 확인

`loadedDraftSignature` 를 create 모드에서만 `null` 로 두어 `hasUnsavedChanges` 가 구조적으로 항상 참
이었다. "복제를 열자마자 아무것도 바꾸지 않고 나가기" 가 매번 확인 다이얼로그를 통과해야 했다. 아무도
편집하지 않은 복제본은 원본과 바이트 단위로 같으므로 버려도 잃는 것이 없다 — 로드 직후 서명을 두 모드
모두에서 세우고, JSDoc 의 근거를 그에 맞게 다시 썼다.

### (6) IME 가드의 수식키 조합 예외

- `decideKeymapDispatch` 진입부의 `isImeCompositionKeydown(event)` 가 수식키 유무와 무관해서, 229 가
  실린 keydown 은 ⌘S·⌘W·⌘P 까지 전부 `ignore-modifier-only` 로 떨어졌다. B13 이 지목한 노출면
  (키맵 chord 2단·검색 Enter·탐색기 draft)은 **전부 수식키 없는 키**이고, 가드의 근거("조합 중인 음절이
  단일 키 바인딩을 발동시키면 안 된다")도 그 범위에만 들어맞는다.
- 판단: 입력기는 Cmd/Ctrl 조합 keydown 을 소비하지 않으므로 예외를 두어도 잃을 것이 없고, 만약 229 를
  실어 보내는 환경이 하나라도 있으면 예외가 없을 때 **조합 중 모든 앱 단축키가 무음 사망**한다 —
  §5 F8 이 "실기 확인이 남아 있다" 고 적어 둔 바로 그 미검증 전제 위에 전 단축키를 얹는 셈이었다.
  실패 모드가 비대칭이므로 좁히는 쪽을 택했다.
- 조치: `isImeCompositionKeydownWithoutCommandModifier`(= 기존 판정 && `!metaKey` && `!ctrlKey`)를 두고
  0번 분기만 그것으로 바꿨다. **Option/Alt 는 예외에 넣지 않았다** — macOS 는 Option 으로 dead key 를
  조합한다. `isImeCompositionKeydown` 자체(검색 Enter·탐색기 draft·다이얼로그 등 다른 호출부)는 손대지
  않았다 — 그쪽은 전부 수식키 없는 Enter/Escape 다.
- 테스트: 기존 "조합 중 keydown 이 chord 1단 프리픽스로도 진입하지 않는다" 를 **수식키 없는** 케이스로
  다시 쓰고(같은 보호는 유지), `Cmd·Ctrl 조합 keydown 은 229 가 실려 있어도 가드에 걸리지 않는다`
  (chord 진입 + 단일 dispatch 두 축)를 추가했다.

### (7) `NumericField` 스냅백 — 기각

- 제안(성공 시에만 되돌리기 / 낙관적 clamp 표시)은 **B15 가 고친 실패 모드를 되살린다**: 커밋 값을 화면에
  남겨 두면 `useUpdateSettings` 왕복이 실패했을 때 저장되지 않은 숫자가 저장된 것처럼 서 있게 된다
  (`resolve-numeric-field-commit` 의 JSDoc 이 근거로 붙들고 있는 바로 그 상태). 현재 동작은 "화면에 있는
  값 = 실제로 적용된 값" 을 항상 유지하며, "입력값 → 옛값 → 새값" 의 되튐은 그 불변식의 정직한 표현이다
  (왕복이 느릴수록 오래 보이는 것도 사실의 반영이다).
- 깜빡임 없이 둘 다 만족시키려면 필드를 controlled + pending 상태로 바꿔야 하는데 이는 설계 변경이고,
  같은 패턴을 쓰는 다른 설정 필드까지 함께 다뤄야 한다. 이번 검토 범위 밖으로 두고 현행 유지한다.

## §5 이월·판정 기록

### F1

- **`workspace-edit-applier.ts` 의 직접 `commands.fileSave` 저장은 정착 방송에 포함하지 않음** —
  LSP rename/create/delete 가 `useSaveFile` 을 거치지 않고 IPC 를 직접 호출한다. B7 의 LSP 파트는
  F1 범위가 아니고, 해당 경로는 자체적으로 `markModelDirtyExternally`·`mirrorDirtyExternally`
  로 별도 부기를 하고 있어 임의로 묶으면 그 계약을 흔든다. 필요 시 후속 스테이지에서 같은
  `publishFileSaveSettle` 를 태우면 된다(레지스트리는 이미 경로 단위라 추가 표면 없음).
- **`ide_save` 의 `GIT.PROJECT` 무효화는 추가하지 않음** — 항목이 요구한 것은 dirty·
  `syncedContent`·미러의 정착이고, git status 는 fs watcher(`ipc-sync-provider`) 경로가 이미
  담당한다. 최소 diff 유지.
- **[검토 추가] draft 리더가 자기 모델보다 오래 사는 것은 이제 계약이다** — §4 (1) 의
  `readDraftSafely` 가 그 경우를 "draft 없음" 으로 정규화한다. 근본적으로 없애려면 `disposeModel`/
  `retargetModel` 이 그 경로의 pane 에 무효화를 통지해야 하는데(경로 단위 레지스트리가 이미 있으므로
  표면 추가는 없다), 통지 순서가 또 하나의 순서 계약을 만들어 이번 범위 밖으로 남긴다. 현재 형태는
  "리더는 모델보다 오래 살 수 있다" 를 허용하고 읽는 쪽에서 닫는다.
- **[검토 추가] read-only 파일의 draft 는 여전히 폐기 경로가 배너뿐이다** — ⌘S 는 이제 사유를 말하지만
  (§4 (2)), 복원된 draft 를 버리려면 "디스크 보기"(`handleViewDisk`, 배너가 떠 있을 때만 노출)를 눌러야
  한다. 저장 불가 토스트에서 그 동작으로 바로 보내려면 토스트 액션 버튼 + 로케일 키가 더 필요해 이번
  범위 밖이다.
- **[검토 추가] LSP 가 이미 열린 모델에 넣는 편집은 read-only 파일에서도 막지 않는다** — 디스크로 나가는
  출구 3곳(`handleSave`·`workspace-edit-applier` 의 파일 경로·`ide-sync-provider`)이 모두 `readOnly` 를
  거르므로 파일은 안전하지만, 그 탭은 저장할 수 없는 dirty 상태가 된다(d-50 §5 S2 에 같은 취지 기록).

### F2

- **B8 의 탭 kind 영속화는 이월(계약 §0 명시)** — 편집한 쿼리·토글·컨텍스트 줄 수는 FE 메모리
  (`search-editor-memory.ts`)에만 남는다. 앱을 재시작하면 탭이 열릴 때의 원래 `SearchQuery` 로
  돌아온다. 영속화하려면 `TabKind::SearchEditor.query` 를 갱신하는 layout 커맨드(신규 IPC 표면)가
  필요해 F2 범위를 넘는다.
- **Search Editor 메모리 항목은 탭 닫기로 지워지지 않는다** — 닫힘을 관측하려면 `useCloseTab`
  경로에 배선이 필요한데, 그것은 F4(탭)·S8(layout) 접점이다. 대신 `SEARCH_EDITOR_MEMORY_LIMIT`(16)
  LRU + projectId 불일치 폐기로 무한 증가를 막았다. 배선을 붙이려면 write/read 만 쓰는 현재 API 에
  삭제 1개를 추가하면 된다.
- **탐색기 뷰 전환(파일↔검색)은 여전히 검색 패널을 언마운트한다** —
  `explorer-panel.tsx` 가 `{view === 'search' && …}` 로 조건 렌더하기 때문이고,
  `docs/features/explorer-sidebar.md` §4 의 "Search 뷰 keep-alive" 서술과 어긋난다. B9 가 지적한
  것은 **프로젝트 전환 시 잔존**이라 이번엔 키잉·리셋만 했다(반대 방향 결함이라 같이 고치면
  B9 의 리셋과 충돌한다). keep-alive 를 실제로 원하면 별건으로 판단해야 한다 — 서술/구현 불일치
  사실만 기록.
- **`useAddRecentSearch` 의 큐는 훅 인스턴스(surface) 단위**다. 패널과 Search Editor 탭이 **동시에**
  검색을 시작하는 경우의 경합은 남는다. 큐를 모듈 전역으로 올리면 그 경합도 닫히지만, TanStack
  v5 는 `mutate` 의 per-call 콜백을 컴포넌트 언마운트 시 호출하지 않으므로 `inFlight` 가 영구히
  남아 이후 모든 히스토리 갱신이 서버 값을 무시하게 된다(전역 상태를 죽은 pending 에 고정).
  인스턴스 단위면 그 누수가 언마운트와 함께 사라져 실패 모드가 훨씬 안전하다고 판단했다.
- **10,000 매치 상한에 걸린 결과의 Replace All 은 "표시된 파일에만" 적용된다** — 패널은 선택된
  그룹 경로를 `paths` 로 넘기므로 잘린 뒤쪽 파일은 치환되지 않는다. 이는 프로젝트 전역으로 조용히
  퍼지는 것보다 안전한 쪽이라 그대로 두고, 잘림 사실만 `search.truncated` 로 알린다.
- **`search.replaceSkipped`/`replaceSkippedMore`/`truncated` 는 i18next 복수형 변형을 만들지 않았다**
  — 기존 `search.matchCount` 등과 동일한 단일 형태 정책(감사 §4-B "복수형 변형 부재는 기록만")을
  따랐다.

### F3

- **B11 의 개명 표시 정정은 이월(선행 필요: 감사 §2 M-2)** — FE 배선은 끝났지만 상태 행에
  `beforePath` 를 채울 소스가 없다. d-50 S3 §5 가 실측으로 확정한 대로 git2 의
  `StatusEntry::path()` 가 `head_to_index`(없으면 `index_to_workdir`)의 **`old_file.path`** 를
  돌려주므로 `collect_status_rows` 의 `path` 는 개명 행에서도 옛 경로이고, `orig_path` 는 같은
  값을 `filter(|orig| orig != path)` 로 걸러 **어떤 행에서도 채워지지 않는다**. 그래서 스테이지된
  개명의 diff 는 여전히 "HEAD 의 옛 경로 vs 인덱스의 옛 경로(없음)" 로 계산돼 통째 삭제처럼
  보인다. 고치려면 상태 행이 새 경로와 옛 경로를 함께 실어야 하고(= 감사 §2 M-2 양방향 rename,
  d-50 §0 에서 명시적 범위 제외·백로그 이관), 그 순간 이번 배선이 값을 나르기 시작한다.
  F3 가 M-2 를 대신 착수하지 않은 이유는 그것이 Rust 상태 모델 변경이라 이 스테이지(git FE)의
  경계를 넘고, d-50 이 이미 제외 결정을 내린 항목이기 때문이다.
- **옛 `layout.json` 이 남긴 상대경로 diff 탭은 그대로 복원된다** — 생산자를 절대경로로 통일해도
  이미 저장된 탭의 `path` 는 바뀌지 않는다. 그 탭은 (a) `fs:changed` 무효화를 여전히 받지 못하고
  (b) 탭바에서 같은 파일을 "Open Changes" 하면 별도 탭이 하나 더 열린다. 충돌 판정만
  `isDiffHunkStageable` 이 두 표기를 모두 받아 막아 뒀다(마커 위 스테이지 거터는 데이터 파괴 쪽
  실패라 우선 닫았다). 레이아웃 마이그레이션(탭 kind 경로 정규화)은 layout 버전 축 작업이라
  별건 — 해당 탭은 preview 성격이라 한 번 닫고 다시 열면 스스로 정상화된다.
- **D8 의 재현을 커밋 테스트로 남기지 않았다** — 재현하려면 monaco 의 diff 계산기
  (`monaco-editor/esm/vs/editor/common/diff/linesDiffComputers.js`)를 직접 import 해야 하는데,
  ① 패키지 `exports` 가 그 서브패스를 막아 `node_modules` 상대경로로만 불러진다(경로 alias 규약
  위반) ② bun 런타임에서 그 모듈을 로드하면 프로세스가 종료되지 않는다(실측 — 백그라운드 전환
  후에도 살아 있었다). `bun test` 전체를 멈춰 세울 위험이 있어, 확증은 스크래치 스크립트 실측으로
  하고 그 입력·출력을 §3 에 남기는 쪽을 택했다. 옵션 자체의 회귀는 `diff-view.tsx` 의 JSDoc 이
  근거를 붙들고 있다.
- **`ignoreTrimWhitespace: false` 로도 알고리즘 차이는 남는다** — monaco 는 `diffAlgorithm:
  'advanced'`, libgit2 는 Myers + `indent_heuristic(true)` 다. 반복 줄이 많은 애매한 정렬에서는
  둘 다 "유효한 diff" 이면서 hunk 경계가 다를 수 있고, 그때 거터 스테이지는 여전히
  `error.git.hunkNotFound` 로 실패한다. 다만 실패 모드가 **눈에 보이는 에러 토스트**(조용한 오적용이
  아니라)라 데이터 안전 쪽이고, 근본 해소는 "프런트가 라인 범위를 보내는" 계약 자체를 바꾸는 일
  (백엔드가 hunk 목록을 주고 프런트는 인덱스를 보내는 구조)이라 별건으로 남긴다.
- **에디터 거터의 stage 는 이번 축과 무관하다** — `use-editor-git-gutter-and-conflicts.ts` 는
  `git_gutter` 가 준 백엔드 hunk 를 그대로 되돌려 보내므로 monaco diff 를 거치지 않는다. 다만 그
  hunk 는 HEAD↔워킹트리 좌표라 부분 스테이지된 파일에서는 `git_stage_hunk`(인덱스↔워킹트리)와
  어긋나는데, 이는 그 파일 JSDoc 이 이미 명시한 기존 계약이라 F3 에서 건드리지 않았다.
- **커밋 메시지 메모리는 프로젝트 닫힘으로 지워지지 않는다** — 관측하려면 `projectClosed` 경로
  (`ipc-sync-provider.tsx`)에 배선이 필요한데 그것은 F3 범위 밖이다. 대신
  `COMMIT_MESSAGE_MEMORY_LIMIT`(8) LRU 와 "빈 메시지는 항목 삭제" 로 무한 증가를 막았다.
- **`git.commitBlockedByConflicts` 는 충돌 해소 방법을 안내하지 않는다** — 문구는 "충돌을 먼저
  해결해야 커밋할 수 있습니다" 한 줄이고, 해소 UI 로 유도하는 링크·버튼은 두지 않았다. Merge
  Changes 그룹이 바로 위에 떠 있어 경로가 자명하다고 판단했다.

### F4

- **Rust `close_tab` 의 pinned 가드는 넣지 않았다(계약 지시대로 기록만)** — 프런트에서 닫히는
  경로 3종(트레일링 버튼·미들 클릭·⌘W)은 전부 막았지만, `layout_close_tab` 커맨드 자체는 여전히
  pinned 탭도 닫는다. 그래서 (a) context menu 의 Close, (b) `ide:close-tab-requested`(외부 에이전트),
  (c) 원격 제어 경로는 그대로 닫는다. (a)는 의도적 탈출구지만 (b)·(c)는 사용자의 고정 의사를
  우회하는 셈이다 — 백엔드 가드는 layout 도메인(S8) 소관이라 이 스테이지에서 착수하지 않았다.
- **`find`(⌘F) ↔ `monaco.actions.find`(⌘F) 가 이제 충돌 1쌍으로 표시된다** — D3 로 monaco 기본
  바인딩이 판정에 들어오면서 생긴 유일한 신규 충돌(전수 스캔으로 확인: 다른 쌍 0). 사실 관계로는
  참이다(두 행이 같은 ⌘F 를 광고한다). 앱 `find` 핸들러가 에디터 포커스 시 monaco 의 `actions.find`
  로 위임하는 **의도된 겹침**이라 경고가 노이즈처럼 보일 수 있는데, 이를 없애려면 (a) `APP_KEYMAP`
  의 `find` 에 `when` 을 붙여 스코프를 나누거나 (b) "위임 관계" 를 데이터로 표현해야 한다 — 둘 다
  디스패치 동작 변경이라 이 스테이지(판정만 손대기) 밖으로 남긴다.
  - **[검토 반영]** 결과적으로 **아무 재바인딩도 하지 않은 기본 설치 상태에서 없앨 수 없는 경고 1쌍**이
    상시 노출되고, 사용자가 만든 실제 충돌과 구분되지 않아 경고의 신뢰도를 깎는다. D3 자체는 유지하되
    "기본 상태 경고 0" 을 목표로 `docs/backlog.md` 에 (b) 안을 등재했다.
- **`filterKeybindingRowsByCapturedKey`(키로 검색)는 여전히 monaco 기본 바인딩을 모른다** —
  `row.key` 가 빈 monaco 행은 ⌘D 로 검색해도 나오지 않는다. D3 이 지정한 것은 "충돌 검사" 라
  범위를 그쪽으로만 좁혔다. 붙이려면 그 함수의 `row` 를 `resolveKeybindingRowBinding(row)` 로
  바꾸는 한 줄이면 되고, 그 순간 "미할당 행은 어떤 캡처에도 걸리지 않는다" 테스트의 의미가
  달라지므로 함께 조정해야 한다.
- **B1 의 승자 규칙은 "핸들러가 실제로 있는지" 를 보지 않는다** — `decideKeymapDispatch` 가
  `dispatch` 를 돌려주면 커맨드 쪽은 물러난다. 그 `APP_KEYMAP` 액션의 핸들러가 이 창에 마운트돼
  있지 않으면(예: 보조 창의 `explorer`) 재바인딩된 커맨드도 함께 침묵한다. 핸들러 유무는
  `useGlobalKeymap` 소비처마다 흩어진 런타임 정보라 순수 판정 함수에서 알 수 없고, 애초에
  사용자가 만든 충돌 상태라 보수적으로(둘 다 안 하는 쪽) 두었다. 근본 해소는 충돌 저장 차단이
  아니라 "한 키에 한 액션" 을 카탈로그 수준에서 강제하는 설계 변경이다.
- **C4 의 in-flight 가드는 유닛 테스트가 없다** — 훅 상태 + 실제 mutation 왕복이라 순수 함수로 뗄
  수 있는 결정이 없고, 이 레포에는 React 훅/컴포넌트 테스트 인프라(testing-library·DOM 환경)가
  없다. `docs/features/explorer-sidebar.md` §2.4 가 근거를 붙들고 있다.
- **붙여넣기 재시도 상한(8)에 걸리면 마지막 백엔드 오류를 그대로 보여준다** — 접힌 폴더에
  `name 복사본 2..8` 이 모두 이미 있는 경우다. 무한 루프 대신 사용자가 대상 폴더를 펼쳐 상황을
  보게 하는 쪽을 택했다(그 순간부터는 형제 목록이 채워져 첫 후보부터 정확해진다).
- **잘라내기를 하위 폴더로 붙여넣는 경우는 백엔드가 막는다** — `copyIntoSelf` 가드는 copy 축이고,
  cut(=rename) 을 자기 하위로 옮기는 것은 `std::fs::rename` 이 OS 레벨에서 거절한다(그 오류가
  토스트로 나간다). 프런트 선제 가드는 두지 않았다 — C1 이 지정한 것은 "제자리" 케이스뿐이다.

### F5

- **`pty_attach` 자체는 여전히 죽은 세션도 받아준다** — 종료된 세션의 `SessionEntry` 는 kill 전까지
  스토어에 남고 `find_entry` 가 성공하므로, 백엔드에는 liveness 가드가 없다. B14 이후 attach 를 막는
  것은 **프론트 로스터뿐**이다. `running` 을 attach 조건으로 강제하려면 `domain/terminal/commands.rs`
  변경(Rust)이라 이 스테이지 밖이다 — 로스터가 유일한 방어선이라는 사실만 기록한다.
- **로스터 갱신은 "이미 캐시된" 프로젝트 목록에만 적용된다** — `markTerminalSessionExited` 는
  `setQueriesData` 로 `SESSIONS_ALL` 접두에 매칭되는 엔트리만 고친다. 한 번도 조회되지 않은 프로젝트의
  로스터는 첫 조회 때 백엔드 사실을 그대로 받으므로 문제가 없다(빈 캐시에 부분 목록을 심지 않는 것이
  업데이터의 계약이다).
- **재시작(`handleRestart`)한 세션의 옛 항목은 로스터에 `running:false` 로 남는다** — 프로젝트를 닫을
  때까지 누적되지만, 재시작 횟수만큼이고 판정에 쓰이는 것은 `id` 일치 + `running` 이라 오작동은 없다.
  지우려면 종료 이벤트에서 remove 로 바꿔야 하는데, 그러면 "[process exited]" 화면이 참조하는 근거가
  사라져 UI 쪽 판정을 함께 바꿔야 한다 — 이득이 없어 두었다.
- **포커스는 마운트 시점에만 이동한다** — 이미 마운트된 터미널이 있는 pane 으로 포커스가 옮겨가는
  경우(⌘1 류 pane 포커스 커맨드)는 다루지 않는다. 에디터도 같은 규약(`code-editor.tsx` 는 모델 부착
  시에만 focus)이라 터미널만 다르게 만들지 않았다. pane 포커스 이동 시 DOM 포커스까지 옮기는 것은
  editor-area 축의 별건이다.
- **C14 의 포커스·닫기 처리와 D5 는 유닛 테스트가 없다** — 셋 다 이펙트 수명주기와 실제 IPC 왕복에
  걸린 결정이라 순수 함수로 뗄 수 있는 부분이 없고, 이 레포에는 React 훅/컴포넌트 테스트 인프라가
  없다(F4 의 C4 와 같은 사유). 대신 D5 는 근거를 코드 실측으로 §3 에 남기고 `terminal-pane.tsx` 의
  JSDoc·`docs/features/terminal.md` §2 가 회귀를 붙들게 했다. 큐잉(C14)·로스터(A6·B14)처럼 순수
  로직으로 떨어지는 부분은 전부 테스트로 고정했다.
- **입력 큐가 둘이다(로컬 typeahead · `terminal-write-bridge`)** — 스폰 완료 전에 둘 모두에 입력이
  쌓이면 어느 쪽이 먼저 pty 에 도착하는지는 정해져 있지 않다(브리지는 `sessionId` 가 서는 이펙트에서,
  로컬 큐는 스폰 continuation 에서 flush 한다). "탭을 새로 열자마자 태스크를 실행하면서 동시에 타이핑"
  이라는 조합에서만 관측되고, 합치려면 브리지가 세션 수명주기를 알아야 해(현재는 tabId 만 안다)
  범위를 넘는다.
- **와이드 문자 좌표 수정은 wrapped line 을 잇지 않는다** — `provideLinks` 가 물리 행 하나만 보는
  기존 스코프 컷은 그대로다(파일 상단 JSDoc). C13 이 지정한 것은 같은 행 안의 열 매핑이다.
- **`readTerminalRowColumns` 는 xterm 내부 구현을 재현한 것이다** — 공개 API 가 `outColumns` 를
  버리는 한(§3 실측) 대안이 없다. xterm 이 `BufferLineApiView` 에 4번째 인자를 전달하도록 바뀌면 이
  함수는 그 호출로 대체할 수 있다.

### F6

- **B5 의 근본 방어선은 프론트 한 곳뿐이다** — 앱 안에서 테마를 지우는 경로는 테마 편집기 하나라
  이번 수정으로 닫히지만, `settings.themeId` 가 가리키는 파일이 다른 이유로 사라지면(사용자가
  `themes/` 폴더에서 직접 삭제, sync 로 내려받은 목록에서 빠짐) `theme_get_current` 는 여전히
  `NotFound` 로 실패하고 앱은 무테마 상태가 된다. 근본 해소는 `theme_get_current` 가 로드 실패 시
  같은 타입 builtin 으로 폴백하는 것(Rust `domain/theme/commands.rs`)인데, 그것은 d-50(Rust 배치)
  경계이고 이 항목이 FE 스테이지로 배정돼 착수하지 않았다. 프론트가 그 실패를 감지해 스스로
  고치는 방식은 "백엔드 사실을 프론트가 되돌려 쓴다" 는 형태라 채택하지 않았다.
- **B6 의 tokenColors 비교는 직렬화 동등성이다** — 같은 Rust 직렬화기(serde, BTreeMap 아님·Vec)가
  만든 두 값이라 키 순서가 안정적이라는 전제에 기댄다. 전제가 깨지면 판정이 "다르다" 로 기울고,
  그 결과는 base 와 같은 규칙을 자식 파일에 한 번 더 적는 것(무해·상속만 끊김)이다. 반대로 잘못
  "같다" 가 나오려면 문자열이 완전히 일치해야 하므로 손실 방향의 오판은 구조적으로 없다.
- **B6 은 `extends` 재지정 자체를 고치지 않는다** — `resolveBaseThemeId` 는 커스텀 테마를 편집할 때
  base 를 항상 내장 dark/light 로 재계산하므로, 번들 테마를 복제해 만든 사용자 테마를 편집·저장하면
  `extends` 가 번들 id → 내장 id 로 바뀐다. 색 값은 diff 로 전부 파일에 남아 화면은 같고,
  tokenColors 도 이제 함께 실려 강조가 유지된다(이 항목이 실제로 막은 손실). `extends` 를 원본
  그대로 보존하려면 프론트가 raw `Theme`(현재 `theme_get` 은 `ResolvedTheme` 만 준다)을 볼 수 있어야
  해서 IPC 표면 추가가 필요하다 — 별건.
- **B12 의 재요청은 LSP 심볼만 갱신한다** — 브레드크럼의 경로 세그먼트(트리 행)와 아웃라인의
  "활성 파일" 판정은 기존 쿼리 무효화 경로 그대로다. 또한 LSP 세션이 없는 언어는 원래대로 심볼이
  비어 있고(재요청도 `[]` 를 반환), 이 스테이지는 그 정책을 바꾸지 않았다.
- **B12 의 디바운스 값(400ms)은 실측 기반이 아니다** — "타이핑 버스트를 하나로 합치되 편집에서
  눈을 떼기 전에 따라온다" 는 판단값이다. F7(perf-11 심볼 디바운스)이 같은 축을 다루므로, 값 조정이
  필요하면 그 스테이지가 이 상수(`DOCUMENT_SYMBOL_REFRESH_DEBOUNCE_MS`) 한 곳만 고치면 된다 —
  F6 이 배선을 끝냈으므로 perf-11 의 남은 몫은 값·범위 판단뿐이다.
- **B15 의 공통 toast 는 `useUpdateSettings`/`useSetThemeId` 두 훅에 한정된다** — 설정 화면이 쓰는
  다른 도메인 뮤테이션(AI 토큰·원격 비밀번호·sync·스니펫·플러그인)은 각자의 보고 경로를 그대로
  둔다. 그 훅들은 이미 자체 `onError`/호출부 toast 를 갖고 있고, 한 곳으로 모으면 도메인별 문구가
  전부 일반 문구로 퇴화한다.
- **`remote.allowedHostsSaveFailed` 는 더 이상 쓰이지 않는다** — 중복 toast 를 없애며 호출부
  `onError` 를 제거했고, 키 자체는 3로케일 카탈로그와 `locale/service.rs` 필수 키에 남겨 뒀다
  (제거는 Rust 필수 키 목록 + 3파일을 함께 건드리는 별도 정리 작업이고, 남아 있어도 파리티 테스트에
  걸리지 않는다).
- **C12 는 "선택되지 않았다" 를 보여줄 뿐 강제하지 않는다** — 모델 미선택 상태에서도 자동완성 토글은
  켤 수 있다(`aiAutoTabEnabled` 는 프로바이더 구성 여부만 본다). 모델 미선택 시 실제 요청이 어떻게
  처리되는지는 AI 도메인(ai.md) 몫이라 이 스테이지에서 건드리지 않았다.
- **D6-① 이후에도 monaco 는 사라진 플러그인 언어의 등록을 유지한다** — `monaco.languages.register`
  는 해제 API 가 없다. 그 언어로 열리는 파일은 이제 예외 없이 **plain 텍스트**로 토큰화된다(옛
  동작: 매 줄 예외). 언어 목록·확장자 연결에서 지우려면 webview 재시작이 필요하다는 기존 제약은
  그대로다.
- **D6-② 의 새 스니펫 파일 다이얼로그는 여전히 내장 언어만 나열한다** — provider 등록은 플러그인
  언어까지 확장했지만, `new-snippet-file-dialog.tsx` 의 선택지는 `TAIDE_LANGUAGE_IDS` 고정이다.
  파일을 손으로 만들거나 전역 `.code-snippets` 의 `scope` 로는 이미 동작한다. 목록을 넓히려면
  플러그인 언어 목록(entities)을 그 위젯에 주입해야 해 배선이 한 겹 늘어난다 — 별건.
- **D6-④ 의 확인 다이얼로그는 창 닫기·앱 종료를 막지 않는다** — 설정 탭 자체를 닫거나(레이아웃
  경로) 앱을 끄면 두 에디터의 드래프트는 예전처럼 사라진다. 탭 닫힘을 가로채려면 `useCloseTab`
  경로에 취소 가능한 훅이 필요하고(현재 계약에 없다), 그것은 layout 축(S8) 접점이다.
- **D6-③ 은 저장을 막을 뿐 어떤 행이 문제인지 표시하지 않는다** — toast 가 개수만 알린다.
  행 단위 인라인 오류 표시는 `snippet-entry-editor.tsx` 에 유효성 상태를 내려보내는 UI 변경이라
  이 스테이지(무음 손실 차단)의 목적을 넘어선다.
- **테마·스니펫 에디터의 unsaved 판정은 문자열 비교다** — 드래프트가 커져도 비용은 무시할 만하지만
  (색 토큰 133 + 구문 31, 스니펫 수십 행), 매 렌더 직렬화라는 점은 기록해 둔다. 실측 문제가 되면
  변경 플래그를 상태로 승격하면 된다.
- **D7 은 팔레트 경로만 고쳤다** — 설정 화면의 sync 섹션은 이미 성공·실패 toast 를 갖고 있어 손대지
  않았다(감사 §4-B D7 이 지목한 위치와 실제 결함 위치가 달랐다는 사실은 §3 에 기록). 두 경로가
  같은 실패 문구(`settings.syncDownloadFailed`)를 쓰지만, 백엔드의 구체적 사유(gist 변경 vs 다른
  동기화 완료)는 여전히 문구로 구분되지 않는다 — 사유별 로케일 키 분화는 sync 도메인 별건.

### F7

- **monaco 자체는 지연하지 않았다** — 계약 §0 의 제외 항목("perf-1 의 monaco 자체 지연 — 구조 수술,
  후순위") 그대로다. 분할 후 부팅 페이로드 5832.79 kB 중 약 3756 kB(`standaloneServices` 1615.81 +
  `editor.api` 980.22 + `toggleHighContrast` 1160.04)가 monaco 다. 에디터는 앱의 첫 화면이 거의
  항상 필요로 하므로, 이걸 지연하려면 "탭 없는 첫 프레임" 을 별도 셸로 설계해야 한다 — 별건.
- **[검토 추가] 가상화가 뷰포트 밖 행의 키보드 도달성을 없앴다** — `SearchMatchRow`·`FileGroupHeader`·
  `ProblemRow` 는 모두 `role='button' tabIndex={0}` 이라 가상화 이전에는 Tab 으로 목록 전체를 순회할 수
  있었다. `overscan: 12` 범위만 DOM 에 존재하는 지금은 첫 화면 분량을 넘어 Tab 으로 이동할 수 없고
  포커스가 목록 밖으로 빠진다(존재하지 않는 요소로는 포커스가 가지 않으므로 스크롤도 유발되지 않는다).
  같은 레포의 선례인 `features/explorer/file-tree.tsx` 는 컨테이너 로빙 포커스(`role='tree'` +
  `onKeyDown` + `scrollToIndex`)로 이 문제를 피하는데, 신규 두 목록에는 그 대체 수단이 없다.
  개별 tabstop 을 로빙으로 바꾸는 작업(행 3종 + 컨테이너 2곳 + 활성 인덱스 상태)은 접근성 기능 규모라
  `docs/backlog.md` 로 이관한다. 마우스·클릭 경로에는 영향이 없다.
- **`ts.worker` 청크(6791.7 kB)는 손대지 않았다** — 워커 자산이라 부팅 페이로드에 잡히지 않고,
  `monaco/setup.ts` 의 `MonacoEnvironment.getWorker` 가 monaco 의 요청 시점에 만든다(이미 지연).
  TypeScript 파일을 여는 순간 받는 크기 자체를 줄이는 것은 monaco 구성 축 별건.
- **grammar 코어 3종 선정은 실측이 아니라 판단이다** — "앱이 사용자 동작 없이 스스로 여는 표면"
  기준으로 골랐다. 세션 복원으로 열리는 파일(대개 ts/tsx/rs)은 온디맨드 경로로 들어가므로, 복원
  직후 그 언어의 하이라이팅이 한 박자 늦게 붙는다(모델 생성 → `loadLanguage` → provider 재부착).
  체감이 문제가 되면 코어 목록에 추가하는 것이 아니라 **직전 세션에서 요구된 언어를 영속**해 부팅
  시 선로드하는 쪽이 옳다 — 새 저장 표면이 필요해 이 스테이지에서 착수하지 않았다.
- **온디맨드 로드 1건마다 tokens provider 를 전량 재부착한다** — `shikiToMonaco` 는 언어 하나만
  등록하는 API 를 노출하지 않아, 새 언어가 붙을 때마다 로드된 전 언어분을 다시 등록하고 직전 묶음을
  dispose 한다(F6 의 swap 이 동일성 검사로 안전하게 처리). 서로 다른 언어의 파일을 처음 여는 횟수
  만큼만 일어나고 그 뒤로는 발생하지 않으므로 수용했다.
- **perf-5 의 보고서 기술은 현재 라이브러리에서 성립하지 않는다** — §3 에 근거를 적었다.
  구현한 디바운스는 **키보드 리사이즈** 홍수를 막고, 자기 에코 억제가 리사이즈를 포함한 모든
  레이아웃 뮤테이션의 중복 리페치를 없앤다. 포인터 드래그 자체는 원래도 커밋 1회였다.
- **`schedulePaneResizeCommit` 의 pending 타이머는 언마운트로 취소되지 않는다** — 120ms 안에
  스플릿이 사라지면 이미 언마운트된 컴포넌트의 `mutate` 가 호출되고, 그 경우 마지막 폭이 저장되지
  않을 수 있다(무해 — 그 pane 자체가 없어졌다). 취소하면 "리사이즈 직후 다른 탭으로 이동" 같은
  정상 흐름에서 저장이 사라지므로 취소가 아니라 방치를 택했다. 모듈 맵도 그때 항목이 남지 않는다
  (발화 시 스스로 delete).
- **`app-shell.tsx` 의 `handleShellLayoutChanged` 는 디바운스하지 않았다** — 그것은 폭이 아니라
  사이드바 collapse 여부(boolean)만 영속하고, 값이 실제로 바뀌는 경우가 드래그당 최대 1회다.
- **perf-12 의 "마커 전체 재수집" 은 이월** — `use-monaco-markers.ts` 는 `onDidChangeMarkers` 마다
  `getModelMarkers({})` 로 전 마커를 다시 만든다. 보고서 §1-12 는 가상화와 이것을 함께 묶었지만,
  증분화하려면 이벤트가 주는 변경 리소스 목록으로 스냅샷을 부분 갱신해야 하고 그러면 스냅샷이
  `useSyncExternalStore` 계약(불변 참조)과 얽힌다. 가상화만으로 DOM 비용(수천 행)이 사라지고 남는
  것은 배열 재생성 비용이라 비용/위험이 맞지 않아 착수하지 않았다.
- **perf-14 의 팔레트 파일 퍼지 스캔(L1-03) 은 여전히 전량 스캔이다** — 계약 §0 의 제외 항목.
  이번 변경은 그 스캔이 **files 모드일 때만** 돌게 했을 뿐(닫혀 있거나 다른 모드면 0), 스캔 자체의
  알고리즘은 그대로다.
- **커맨드 스냅샷은 공유 배열을 그대로 돌려준다** — `listRegisteredCommands()` 소비처
  (`command-palette.tsx`·`keybindings-editor.tsx`)는 전부 filter/map 만 하지만, 정렬 등 파괴적 연산을
  추가하면 레지스트리 상태가 오염된다. JSDoc 에 read-only 계약을 명기했다(복사본을 돌려주면 이번
  최적화가 무의미해진다).
- **로케일 신규 키 0** — 이 스테이지가 추가한 사용자 표시 문자열이 없다. Suspense fallback 은
  전부 빈 면 또는 `null` 이라 "로딩 중" 문구를 새로 만들지 않았다(지연 청크는 로컬 파일이라 사실상
  즉시 해석된다 — 문구를 띄우면 오히려 깜빡임이 된다).

### F8

- **가드가 "조합 확정 Enter" 를 한 번 먹는 것은 의도된 동작이다** — 한글을 치고 Enter 를 누르면
  첫 Enter 는 조합 확정(`keyCode 229`)이라 실행되지 않고, 확정 뒤 다시 누른 Enter(`keyCode 13`)가
  실행한다. VS Code·브라우저 폼과 동일한 표준 IME UX 이며, "한 번에 실행되던 것이 두 번이 됐다" 는
  회귀가 아니라 B13 이 고치라고 한 바로 그 동작이다. 대조군 테스트(비조합 Enter 는 그대로 실행)로
  가드가 정상 입력을 막지 않음을 고정해 두었다.
- **JSX 핸들러 호출부 자체는 단위 테스트가 없다** — 이 저장소에는 DOM 렌더 하네스가 없다
  (`shared/ui/error-boundary.test.tsx` 가 같은 제약을 문서화). 그래서 판정 로직 전부를
  `isImeCompositionKeydown` 한 함수로 뽑아 그 함수와, 그 함수를 소비하는 순수 결정 함수
  (`decideKeymapDispatch`)를 테스트했다. 각 컴포넌트 쪽은 그 함수를 부르는 1줄 early return 뿐이다.
- **[검토 반영] 키맵 디스패치의 가드는 수식키 없는 keydown 으로 좁혔다** — §4 (6). 도입 시점에는
  `decideKeymapDispatch` 진입부의 가드가 수식키 유무를 보지 않아 ⌘S·⌘W·⌘P 까지 함께 삼켰고, 그것은
  아래 "실기 확인이 남아 있다" 는 미검증 전제 위에 앱 단축키 전체를 얹는 셈이었다(실패 모드가 무음
  이라 원인 추적도 어렵다). 지금은 Cmd/Ctrl 조합이 예외이며, Option/Alt 는 macOS dead key 조합 때문에
  예외가 아니다. B13 이 지목한 노출면(chord 2단·검색 Enter·탐색기 draft)은 전부 수식키가 없어 보호가
  그대로 유지된다.
- **실기 확인은 남아 있다** — `keyCode === 229` 가 WKWebView 에서 문자 키뿐 아니라 **조합을 확정·
  취소하는 Enter/Escape 에도** 실리는지는 이 저장소 안에서 계측할 수 없다(앱 런타임 필요). WebKit
  의 일반 동작(입력기가 소비한 키는 229 로 보고)과 cmdk 가 같은 판정만으로 Safari 의 IME Enter 를
  막고 있다는 사실이 근거이고, 검증 절차는 `docs/bug/2026-08-06-...md` 의 "2026-08-29 후속" 절에
  적어 두었다. 만약 Enter 가 229 로 오지 않는 환경이 확인되면 그때는 `compositionend` 직후
  일정 시간 Enter 를 무시하는 시간 창 방식으로 보강해야 한다 — 지금 선제 도입하지 않은 이유는
  근거 없는 매직 타임아웃이 되기 때문이다.
- **AlertDialog 에는 가드를 넣지 않았다** — `AlertDialogContent` 사용처 11개 파일을 전수 확인한 결과
  텍스트 입력을 품은 것이 하나도 없다(전부 확인·경고 다이얼로그). 조합이 시작될 수 없는 표면에
  가드를 다는 것은 의미 없는 diff 라 제외했고, 나중에 입력이 들어가면 `dialog.tsx` 와 동일한 3줄을
  복제하면 된다.
- **`DropdownMenu`·`ContextMenu`·`Select` 도 제외** — 같은 이유(메뉴 항목에 텍스트 입력 없음).
  이들의 typeahead 는 `event.key` 한 글자를 보는데, 조합 중 `key` 는 `Process` 라 애초에 걸리지 않는다.
- **monaco 내부는 손대지 않았다** — `docs/bug/2026-08-06-...md` 가 실측으로 확인한 대로 monaco 는
  textarea 값을 diff 해 입력을 계산하므로 조합 이벤트 없이도 정상 동작한다. 에디터 위젯 중 우리가
  만든 것(`ai-inline-edit.ts`)만 가드 대상이다.
