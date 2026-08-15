# Wave D Phase B3 — Search Editor 구현 결정 기록

> 정본: `2026-08-15-wave-d-search-nav-contract.md` §3.4·§3.5. 이 문서는 B3(sonnet+xhigh) 구현 중 정본이
> 명시하지 않은 세부 사항에 대해 내가 내린 결정을 기록한다.

## 1. locale 키 부족 — `search.excludeGlobPlaceholder`

- Phase A(백엔드)가 추가한 14개 locale 키 중 excludeGlob 입력의 placeholder 텍스트에 해당하는 키가 없다.
  (`search` 네임스페이스에 추가된 것은 `respectGitignore`·`recentSearches` 2개뿐 — exports 문서와 일치 확인함.)
- `locale/service.rs` 는 이번 웨이브에서 B3 소유 파일 목록(`src/widgets/search-panel/`·`src/widgets/search-editor/`·
  `src/entities/search/`·`pane-node-view`·`src/features/search/`)밖이라 직접 추가하지 않았다.
- 임시 조치: `features/search/search-exclude-glob-input.tsx` 에서
  `t('search.excludeGlobPlaceholder', { defaultValue: 'Files to exclude (e.g. **/*.test.ts)' })` 로
  i18next 의 `defaultValue` 폴백을 사용했다 (하드코딩 문자열이 아니라 `t()` 경유 — `command-registry.ts` 의
  `titleDefaultValue` 선례와 동일 메커니즘). 이 상태에서는 en/ko/ja 구분 없이 항상 영어 placeholder 가 보인다.
- 후속 조치 필요: `locale/service.rs` 의 `search` 네임스페이스에 `excludeGlobPlaceholder` 키를 en/ko/ja 3곳 +
  `MESSAGE_NAMESPACES` 에 추가해야 한다. 제안 문구 — en: "Files to exclude (e.g. **/*.test.ts)",
  ko: "제외할 파일 (예: **/*.test.ts)", ja: "除外するファイル (例: **/*.test.ts)".
- **해결됨 (Wave D 검토 결함 수정 패스)**: 위 제안 문구 그대로 `locale/service.rs` 의 `MESSAGE_NAMESPACES`
  `search` 네임스페이스 + en/ko/ja 메시지 3곳에 `excludeGlobPlaceholder` 를 추가했다.
  `search-exclude-glob-input.tsx` 의 `defaultValue` 폴백·`EXCLUDE_GLOB_PLACEHOLDER_FALLBACK` 상수는 제거하고
  다른 `t()` 사용처와 동일하게 순수 키 참조(`t('search.excludeGlobPlaceholder')`)로 통일했다.

## 2. Search Editor 탭의 query 는 오픈 시점에 고정 — 실시간 동기화하지 않음

- `TabKind::SearchEditor.query` 를 갱신하는 백엔드 커맨드(`layout_set_view_state` 등)를 이번 작업에서 새로
  배선하지 않았다. `entities/layout/layout.query.ts` 는 B3 소유 파일 목록 밖이라 새 훅을 추가하지 않기로 했다.
- 대신 `SearchEditorPane` 은 탭이 열릴 때의 `query` 를 `useRef` 로 한 번만 캡처해 최초 자동 검색에 쓰고,
  이후 사용자가 옵션(대소문자·정규식·gitignore·contextLines·excludeGlob)을 바꾸거나 검색어를 다시 실행해도
  탭의 영속 `kind.query` 는 갱신되지 않는다. hot-exit·세션 복원 시에는 항상 "탭이 처음 열렸을 때의 쿼리"로
  복원되어 재검색된다.
- 이는 백엔드 exports 문서의 설명("the tab reopens to the query and re-searches rather than replaying stale
  results")과 일치하는 해석이며, 백엔드가 애초에 `query` 필드 하나만 영속시키기로 설계했으므로(§3.4) "마지막
  편집 쿼리"를 저장할 스키마 자체가 없다 — 추가하려면 백엔드 스키마 변경이 필요해 이번 범위 밖으로 판단했다.

## 3. Search Editor 의 "결과 내 편집" = 실제 파일 열기 (별도 인라인 편집 버퍼 없음)

- 계약 §3.4 는 "결과 내 편집(실제 파일 열어 반영 또는 in-place → 파일 반영)" 을 옵션으로 제시했다. VS Code 의
  Search Editor 처럼 검색 결과를 하나의 편집 가능한 가상 버퍼로 만들어 저장 시 원본에 diff 반영하는 방식은
  구현 복잡도(가상 버퍼 파싱·라인 매핑·부분 저장)가 매우 크고 근본적으로 새로운 편집기 표면을 만드는 일이라
  판단해 채택하지 않았다.
- 대신 결과 행 클릭 시 `requestReveal(path, line, column)` 을 먼저 호출한 뒤 실제 파일 탭을 연다
  (`problems-panel-container.tsx` 와 동일한 선례 패턴). 사용자는 열린 실제 파일에서 평소처럼 편집하고
  저장한다 — "실제 파일 열어 반영" 문구를 문자 그대로 만족시키는 가장 단순하고 안전한 해석이다.
- 일괄 치환이 필요하면 기존 `search_replace`(패널의 Replace All 과 동일 백엔드 커맨드)를 쓸 수 있으나,
  이번 범위에서는 Search Editor 자체에 Replace UI 를 추가하지 않았다(범위 통제 — 계약 §3.4 항목 1의 3개
  요구사항에 Replace 는 명시되어 있지 않음).

## 4. 검색 결과 클릭 시 줄 이동 — `explorer-container.tsx` 를 건드리지 않고 해결

- 정찰 문서(§risks)는 `explorer-container.tsx` 의 `openSearchMatch` 가 line 을 전달하지 않는 기존 갭을
  지적했다. 이 파일은 B3 소유 목록 밖이라 직접 수정하지 않았다.
- 대신 `SearchPanelContainer`(소유 파일) 안에서 `onOpenMatch(path, line, column)` 을 받아
  `requestReveal(path, line, column)` 을 먼저 호출한 뒤, 부모로부터 받은 기존 `onOpenMatch(path)`(경로만
  받는 시그니처, `explorer-container.tsx` 소유)를 그대로 호출하도록 래핑했다. `requestReveal` 은 대상 에디터가
  아직 열려 있지 않으면 pending 큐에 적재되어 있다가 `openTab` 이 만든 에디터가 마운트될 때 소비된다
  (`entities/editor/reveal-registry.ts` 의 기존 동작). 결과적으로 `explorer-container.tsx` 를 전혀 수정하지
  않고도 클릭 시 줄 이동이 동작한다.
