# ⌘P 퀵오픈 UX 정비 계약 (2026-08-20, d-26)

> 사용자 실기 보고(2026-08-20): ① 방향키로 selection 이 안 됨 ② 파일 결과가 full path
> (`/Users/{u}/{proj}/app`)로 나와 가독성 나쁨 — 파일명 + "프로젝트 상대 경로" 부제(`/app`)
> 형태여야 함 ③ 파일명 매칭 위치 하이라이트 부재. "몇몇 디테일이 너무 아쉽네".
> 착수 전 메인 실사(2026-08-20, 전건 실코드):
> - 팔레트 = `widgets/command-palette/command-palette.tsx`(409줄, shadcn Command/cmdk 1.1.1·
>   `shouldFilter={false}`+자체 `fuzzyFilter`). 파일 행 렌더 = `:361-364` **`{item.path}` 단일
>   truncate span**(풀 경로·하이라이트 없음) — ②③의 직접 원인.
> - **③ 의 데이터는 기존재**: `shared/lib/fuzzy-match.ts` 의 `fuzzyMatch` 가 `indices`(매칭
>   문자 위치 배열)를 반환하고 `fuzzyFilter` 가 `{item, match}` 로 전달 중 — 렌더만 미구현.
> - **② 의 헬퍼도 기존재**: `shared/lib/relative-path` 의 `toRelativePath`·`fileNameOf`
>   (브레드크럼 사용 중). fileRows = `treeRowsQueryOptions` rows(path 형태 실사 필요 — 절대
>   경로 정황).
> - **① 원인 후보 서열**(정순 키맵 바인딩은 무관 판명 — Arrow 엔트리는 mod+Arrow·
>   terminalFocus 한정): (1순위) 선택 표시 비가시 — CommandItem 선택 스타일이
>   `data-[selected=true]:bg-accent` 인데 `--accent: var(--taide-list-hover-background)`
>   (global.css:143) — hover 색과 동일해 팔레트 배경 대비가 미미할 수 있음(선택이 움직이는데
>   안 보이는 체감). (2순위) cmdk 선택 자체 미동작 — 캡처 리스너 2중(useGlobalKeymap+팔레트
>   자체 useKeydownCapture)·CommandItem value 부재·cmdk 1.1.1×React 19 상호작용.
>   사용자 판별 정보(방향키 후 Enter 가 다른 파일을 여는지) 수신 시 계약에 반영.

## 1. 범위

### 1.1 방향키 선택 (①)

- 원인 확정이 선행 — 구현이 cmdk 소스(node_modules)·캡처 리스너 체인·선택 스타일 대비를
  실사해 (1)/(2) 중 확정 후 근본 수정. (1)이면: 팔레트 선택 전용 대비 토큰 적용(기존 테마
  토큰 체계 내 — 신규 토큰 남발 금지, `--taide-list-active-*` 계열 실사 후 재사용 우선).
  (2)이면: 원인 지점 근본 수정(캡처 리스너의 삼킴이면 해당 경로, cmdk 계약 위반이면 value
  등 정합). 추측 수정 금지 — 판정 근거 기록.

### 1.2 파일 결과 2단 렌더 (②)

- 파일 행 = **파일명(주 라벨) + 프로젝트 상대 디렉토리 경로(부제·소프트 색)** 2단(VS Code
  Quick Open 형태). `fileNameOf`·`toRelativePath` 재사용. 프로젝트 루트 직속 파일은 부제
  생략. treePage rows 의 path 가 절대 경로인지 실사 후 프로젝트 root 기준 상대화(멀티
  프로젝트 시 활성 프로젝트 root — 실사).
- cmdk value 정합: 렌더 분리 후에도 CommandItem 의 선택/식별 value 는 고유 유지(path).

### 1.3 매칭 하이라이트 (③)

- `fuzzyFilter` 의 `match.indices` 를 렌더에 연결 — 매칭 문자를 강조 표시(테마 토큰 경유·
  기존 검색 매치 강조 선례 실사 후 동일 계열). **인덱스 매핑 주의**: indices 는 매칭 대상
  라벨 기준인데 ②의 2단 분리 렌더와 정합해야 함 — 매칭 대상을 무엇으로 할지(§1.4)와 함께
  설계.
- 커맨드·심볼 모드도 동일 패턴이 국소면 포함, 아니면 파일 모드만 하고 이월 기록.

### 1.4 매칭 대상 실사 (② ③ 과 결합)

- 현재 `fuzzyFilter(searchTerm, fileRows, (row) => row.path)` — **절대 경로 전체**가 매칭
  대상(사용자 홈 경로 문자까지 매칭됨 — ②와 같은 근원). 매칭 대상을 "프로젝트 상대 경로"로
  교체(파일명 가중치 재설계는 범위 외 — 이월 기록). 하이라이트 인덱스는 이 상대 경로 기준으로
  일관.

### 1.5 범위 외

- fuzzy 알고리즘 자체(가중치·순위) 재설계. 팔레트 다른 모드(@·#·:) 구조 변경. 커맨드 표면·
  이벤트·bindings·원격 정책 무변경.

## 2. 실행·검증

- **선행 조건: d-24 검토 반영 완료 후 착수**(동일 트리 순차 — 동시 쓰기 금지).
- 구현 Workflow(sonnet+xhigh 단독, TS 전용) → 메인 2차 → Phase E 4렌즈(정확성: 선택 원인
  판정·인덱스 매핑 / 회귀: 팔레트 전 모드·Enter/preview 열기·keybinding 행 / 설계: 렌더
  구조·토큰 / 계약: 표면·i18n·컨벤션) → 적대적(major 이상) → 수정 → 커밋 → 병합.
- 실기 확증(사용자): 방향키 선택 가시 이동·상대 경로 부제·하이라이트 표시.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

### 3.1 ① 방향키 선택 — 원인 판정

**판정: (1순위) 선택 표시 비가시 — 확정. (2순위) cmdk 선택 자체 미동작 — 기각.**

- **(2순위) 기각 근거(코드 실사)**: `node_modules/cmdk/dist/index.js` 의 `CommandRoot`
  `onKeyDown` 은 리액트 합성 이벤트(버블 단계, `Command` 루트 `div`)로 등록되고, 진입 시
  `if (!(t.defaultPrevented || isComposing)) switch(t.key){ case "ArrowDown": ue(t) ... }` 로
  Arrow 를 처리한다. `useKeydownCapture`(`useGlobalKeymap` 및 팔레트 자체 리스너 2곳 모두)는
  `window.addEventListener('keydown', handler, true)` — **캡처 단계**로 등록되어 항상 cmdk 의
  버블 핸들러보다 먼저 실행된다. 두 캡처 리스너가 plain `ArrowDown`/`ArrowUp` 에 대해
  `preventDefault`/`stopPropagation` 을 호출하면 cmdk 의 `defaultPrevented` 가드에 걸려 선택이
  실제로 죽는 시나리오가 되므로 이를 검증: `APP_KEYMAP`(`shared/lib/keymap.ts`) 의 유일한 Arrow
  엔트리(`terminal-jump-to-previous/next-command`)는 `mods:['mod']` + `when:'terminalFocus'`
  로, plain Arrow 는 `findMatchingKeymapEntry` 가 매치하지 않아 `decideKeymapDispatch` 가
  `{type:'none'}` 을 반환(`preventDefault` 없음). 팔레트 자체의 두 번째 캡처 리스너
  (`findRunnableCommandBinding`)도 `command-registry.ts`/`keybinding-catalog.ts` 전수 조회 결과
  Arrow 키를 기본 바인딩으로 쓰는 커맨드가 없어 동일하게 무동작. 즉 두 캡처 리스너 모두 plain
  Arrow 에서 `preventDefault` 를 호출하지 않으므로 cmdk 의 버블 핸들러는 방해받지 않고
  정상적으로 `aria-selected`/`data-selected` 를 이동시킨다 — 선택 로직 자체는 살아있다.
- **(1순위) 확정 근거(테마 토큰 실값 실사)**: `shared/ui/command.tsx` 의 `CommandItem` 기본
  클래스는 `data-[selected=true]:bg-accent`. `shared/styles/global.css` 에서
  `--accent: var(--taide-list-hover-background)`(:143) 이고, 팔레트의 `Command` 루트는
  `bg-panel-background`(`--taide-panel-background: #181825`) 로 렌더된다(`cn()` 이 `command.tsx`
  기본 `bg-popover` 를 덮어씀). `--taide-list-hover-background: #313244` 와 배경
  `#181825` 의 WCAG 상대휘도 대비비를 계산하면 **약 1.40:1** — 선택 상태의 유일한 시각 변화인
  배경색 차이가 거의 지각 불가능한 수준이다(전경색은 `--accent-foreground`=
  `--taide-list-foreground: #cdd6f4` 로 기본 `text-app-foreground` 와 동일해 텍스트 색 변화도
  없음). 즉 방향키가 선택 state 를 정상적으로 이동시키지만, 그 결과가 화면에 거의 보이지 않아
  "선택이 안 됨" 으로 체감된 것으로 판정.

### 3.2 수정 — 팔레트 한정 대비 클래스 (기존 토큰 재사용)

- `--taide-list-active-*` 계열 실사: `global.css` 에 `--taide-list-active-background: #45475a`
  가 이미 정의되어 있고 `@theme inline` 에서 `--color-list-active-background` 로 노출되어
  Tailwind 유틸 `bg-list-active-background` 로 즉시 사용 가능(신규 토큰 불필요). 대비비는
  팔레트 배경(#181825) 대비 **약 1.92:1** 로 기존 `--accent`(1.40:1) 대비 상대휘도 기준
  약 6.6배 밝은 값이며, 동일 hex(`#45475a`)가 이미 `--taide-app-sidebar-item-active` 로 사이드바
  "활성" 표시에 쓰여 실사용 검증된 값이다.
  `--taide-explorer-item-selected`(#585b70, 대비 2.63:1)도 후보였으나 explorer 트리 전용
  의미이므로 이름이 맞는 `list-active` 계열을 우선 채택.
  `global.css`/`shared/ui/command.tsx` 는 다른 6개 소비처(`font-picker`·`branch-switcher`
  등)가 공유하므로 건드리지 않고, `command-palette.tsx` 의 모든 `CommandItem` 에
  `className='data-[selected=true]:bg-list-active-background'` 를 개별 적용(twMerge 가 같은
  `data-[selected=true]:bg-*` 그룹 내에서 나중 값으로 병합)해 팔레트 전용으로 범위를 좁혔다.

### 3.3 ② 파일 결과 2단 렌더 — 경로 실사

- `treePage.rows`(`entities/tree/tree.query.ts` → `TreeRow.path`, `shared/api/bindings.ts:1910`)
  는 **절대 경로**다(Rust `tree/service.rs` 테스트가 `fixture.root.join("src")` 형태로 구성).
  프로젝트 root 는 기존에 이미 `entities/project/project.query.ts` 의
  `projectQueryOptions(projectId)` → `project.root` 로 조회하며, `toRelativePath(project.root,
  path)` 조합이 `widgets/editor-pane/breadcrumbs-bar.tsx`·`widgets/editor-area/pane-tab-bar.tsx`·
  `widgets/explorer/explorer-container.tsx` 에 이미 동일 패턴으로 존재 — 그대로 재사용했다.
- 팔레트의 `activeProject` 쿼리(`projectQueryOptions`)는 기존에 `mode==='symbol'` 에서만
  `enabled` 였던 것을 `mode==='symbol' || mode==='files'` 로 확장(멀티 프로젝트 시에도 항상
  "활성 프로젝트" 기준 — 팔레트가 다루는 트리 자체가 `activeProjectId` 스코프이므로 별도 분기
  불필요).
- 루트 직속 파일(디렉토리 세그먼트 없음)은 `dirPath: null` 로 부제 자체를 생략
  (`command-palette-file-match.ts`).
- cmdk value 정합: `CommandItem` 에 `value={item.path}` 명시. cmdk 는 `value` prop 이 없으면
  DOM `textContent` 로 값을 유도하는데(node_modules/cmdk `we()`), 2단 렌더로 파일명+경로가
  한 아이템에 같이 렌더되면 서로 다른 파일이 같은 파일명을 가질 때(`index.ts` 등) 유도값이
  충돌할 위험이 있어 `path` 로 고정.

### 3.4 ③ 매칭 하이라이트 — 인덱스 매핑 설계

- `fuzzyFilter` 의 매칭 대상을 절대경로(`row.path`) → 프로젝트 상대경로(`toProjectRelativePath`
  경유)로 교체(§1.4). 하이라이트 인덱스는 이 상대경로 기준으로 산출되므로 2단 분리 렌더와
  자연히 정합.
- 순수 함수 2개로 분리·테스트:
  - `shared/lib/fuzzy-match.ts` `buildFuzzyHighlightSegments(text, indices)` — 문자 단위
    인덱스를 연속 구간(run)으로 묶어 `{text, matched}[]` 로 반환. 파일명/디렉토리/커맨드
    라벨/심볼 이름 등 라벨 문자열 전반에 재사용 가능한 범용 유틸이라 `fuzzy-match.ts` 에
    위치(해당 파일이 이미 `FuzzyMatch.indices` 의 단일 출처).
  - `widgets/command-palette/command-palette-file-match.ts` `splitFileMatchForDisplay(
    relativePath, matchIndices)` — 상대경로를 파일명/디렉토리로 나누고, 원본 인덱스를
    구분자(`/`) 기준으로 각 파트 로컬 인덱스로 재계산(구분자 자체가 매칭되면 양쪽 모두에서
    제외). 파일 표시 전용 로직이라 팔레트 위젯 로컬 파일에 배치
    (`widgets/editor-pane/breadcrumb-path.ts` 의 위젯 로컬 유틸 다중 export 선례를 따름).
  - 렌더 측 `HighlightedText`(`command-palette.tsx` 비export 지역 컴포넌트, `search-match-row.tsx`
    의 `ContextLine` 선례와 동일 패턴)가 두 함수를 조합해 `<mark
    className='bg-panel-match-highlight text-app-background rounded-xs'>` 로 렌더 — 기존 검색
    매치 강조(`features/search/search-match-row.tsx`)와 동일 토큰 재사용.
- **커맨드·심볼 모드**: 국소(1줄 라벨, 기존 `fuzzyFilter` 결과에 이미 `match.indices` 보유)로
  판단해 동일 패턴 적용 — `commands` 모드 라벨, `symbol` 모드 심볼 `name` 에도
  `HighlightedText` 연결.
- **이월(범위 외)**: `workspaceSymbol` 모드는 `fuzzyFilter` 를 거치지 않고
  `workspaceSymbolSearch.search()`(LSP 기반, indices 없음)로 결과를 얻으므로 이번 배치에서는
  하이라이트 미적용 — 인덱스 확보 자체가 별도 설계가 필요해 범위 외로 이월.

### 3.5 검증

- `bun run verify` (typecheck·eslint·prettier·bun test 1421 pass·cargo fmt/clippy/test 1054+
  pass) 전부 exit 0. `bunx vite build` exit 0. `git status` 로 `shared/api/bindings.ts` 등
  bindings 관련 파일 무변경 확인.
- 신규 순수 함수 테스트: `shared/lib/fuzzy-match.test.ts`(`buildFuzzyHighlightSegments`),
  `widgets/command-palette/command-palette-file-match.test.ts`(`splitFileMatchForDisplay`).
