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

  > **§4 정정(Phase E 검토 반영)**: 위 수치(1.40→1.92:1, "6.6배")는 `global.css` `:root`
  > 폴백값 — 즉 builtin `taide-dark` 테마 한 종에만 해당한다("6.6배"는 팔레트 배경 대비
  > 상대휘도 배수이고, `--accent` 대비로는 1.94배). 번들 테마 36종은 각자의 테마 JSON 이
  > 런타임에 이 토큰을 덮어쓰므로 실제 개선폭은 테마마다 다르며, 착수 시점(커밋 e8ab133)의
  > `list.activeBackground` 매핑은 가드 없는 `chain()`이라 일부 테마에서 팔레트 배경과
  > 동일값(1.00:1, 미개선)이 되는 문제가 있었다 — 이는 이 배치가 아니라 이후
  > `docs/acknowledge/2026-08-20-theme-list-colors-contract.md`(d-26b)에서
  > `derived()` + `isUsableListBackground` 가드로 해소됐다. §4 참고.

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
- **이월(범위 외, §1.4 명시분 — 기록 누락분 추가)**: fuzzy 가중치(파일명 세그먼트 우선·연속
  매칭 이외 보정) 재설계는 범위 외로 §1.4 가 이미 정했으나 구현 기록에 반영이 빠져 있었다 —
  매칭 대상만 절대경로→프로젝트 상대경로로 교체했고 `fuzzyMatch`/`fuzzyFilter` 의 점수 산식
  자체는 무변경.

### 3.5 검증

- `bun run verify` (typecheck·eslint·prettier·bun test 1421 pass·cargo fmt/clippy/test 1054+
  pass) 전부 exit 0. `bunx vite build` exit 0. `git status` 로 `shared/api/bindings.ts` 등
  bindings 관련 파일 무변경 확인.
- 신규 순수 함수 테스트: `shared/lib/fuzzy-match.test.ts`(`buildFuzzyHighlightSegments`),
  `widgets/command-palette/command-palette-file-match.test.ts`(`splitFileMatchForDisplay`).

---

## 4. Phase E 검토 반영 (2026-08-20)

커밋 `e8ab133` 기준 4렌즈 검토(발견 17건) 중 독립 재검증으로 살아남은 5건(확정 4·경미 강등 1,
기각 0)을 반영했다. 검토 시점 이후 별도 배치(d-26b, 커밋 `67fdeef`)가 번들 테마 12개의
`list.hoverBackground`/`list.activeBackground` 결함을 이미 정정하고 `mapping-tables.ts` 에
`derived()` + `isUsableListBackground` 가드를 추가해 두었다 — 이번 반영은 그 위에서
번들 테마 36종 실값을 다시 스윕해 판단했다(`src-tauri/resources/themes/*.json`, WCAG 상대휘도
대비비, 알파는 배경 위 합성).

### 4.1 correctness-1 / design-1 (major, confirmed) — 선택 하이라이트 토큰 강건화

d-26b 이후 스윕 결과, `list.activeBackground`(현재 채택 토큰)는 이미 `explorer.itemSelected`
(검토가 제안한 대안)보다 우수했다 — `list.activeBackground` 는 `list.background` **및**
`list.hoverBackground` 양쪽에 가드가 걸려 있는 반면(§ mapping-tables.ts:610-616),
`explorer.itemSelected` 는 `explorer.background` 하나만 가드된다(:471-475). 실측(팔레트 배경
`panel.background` 대비, 36종):

| 토큰 | <1.5:1 | <1.2:1 |
|---|---|---|
| `list.activeBackground`(현행 유지) | 22/36 | 6/36 |
| `explorer.itemSelected`(검토 제안 대안) | 25/36 | 13/36 |

→ **판정: `bg-list-active-background` 유지, `bg-explorer-item-selected` 로 교체하지 않는다.**

배경 토큰 하나로는 여전히 6개 테마가 1.2:1 미만이므로, 검토 제안 (2)의 2차 단서를 추가했다.
`app.focusBorder`(검토 예시 토큰)를 먼저 스윕했으나 36종 중 15개가 팔레트 배경 대비 1.5:1
미만(24개가 3.0:1 미만)으로 전 테마 선명성 기준에 미달 — 대안으로 `app.accent`(전 36종
0개가 1.5:1 미만, 최저 1.73:1)를 채택했다.

| 토큰 | <1.5:1 | <3.0:1 |
|---|---|---|
| `app.focusBorder`(검토 예시, 기각) | 15/36 | 24/36 |
| `app.accent`(채택) | 0/36 | 12/36 |

`shared/ui/command.tsx` 의 `CommandItem` 기본 클래스에
`data-[selected=true]:bg-list-active-background data-[selected=true]:ring-1
data-[selected=true]:ring-inset data-[selected=true]:ring-app-accent` 를 적용했다(design-2 처리와
결합 — §4.2). 링은 `bg-popover` 소비처(폰트/언어/옵션 피커) 기준으로도 재검증했다: 36종 중
`popover.background` 가 `panel.background` 와 다른 13종에서도 `app.accent` 링은 최저
vscode-abyss 1.33:1 을 제외하면 전부 1.5:1 이상.

### 4.2 design-2 (major→minor, confirmed/downgrade) — CommandItem 레벨 승격

검토는 "선택 비가시 결함이 7개 cmdk 소비처 전부에 있다"는 사실은 맞지만(재검증으로 확인),
당시(착수 시점) 제안한 수정 층(`list.activeBackground` 를 `CommandItem` 기본에 승격)은 그
시점 기준으로는 12개 번들 테마에서 무효했을 것이라 검토 자체가 severity 를 `minor` 로
강등했다. d-26b 로 그 전제가 바뀌었으므로(§4.1 표) 이번에 실제로 승격했다:

- `shared/ui/command.tsx` 의 `CommandItem` 이 `data-[selected=true]:bg-list-active-background`
  + `data-[selected=true]:ring-1 ring-inset ring-app-accent` 를 기본 제공 — `grep -rl
  CommandItem src` 로 확인된 7개 소비처(`command-palette`·`branch-switcher`·`branch-group`·
  `task-runner-dialog`·`font-picker`·`language-picker`·`option-picker`) 전부에 일괄 적용된다.
  `--accent` 토큰 자체(다른 컴포넌트도 공유)는 손대지 않았다.
- `command-palette.tsx` 의 팔레트 한정 `PALETTE_ITEM_SELECTED_CLASSNAME` 상수와 5개
  `CommandItem` 호출부의 개별 `className` 적용을 제거했다(design-4 도 함께 해소 — 반복 자체가
  없어짐).
- **eslint ignore 목록 정합(지시 판단·기록)**: `command.tsx` 는 `eslint.config.js` 의 shadcn
  생성물 무시 목록에 남아 있다. 같은 목록의 `dialog.tsx` 도 이미 프로젝트 자체 `Dialog` 조합으로
  가볍게 손질된 채 무시 목록에 있고, `command.tsx` 자체도 원래부터 `CommandDialog` 가
  `@shared/ui/dialog` 의 프로젝트 컴포넌트를 쓰도록 이미 손질돼 있었다 — "shadcn 스캐폴드 +
  가벼운 프로젝트 손질" 은 이 저장소가 이미 채택한 패턴이며, `npx shadcn add command` 재생성 시
  이번 토큰 교체도 함께 재적용해야 하는 유지보수 모델은 동일하다. **결정: 무시 목록 유지,
  `eslint.config.js` 무변경.**

### 4.3 correctness-2 / design-3 (major, confirmed) — 매칭 하이라이트 전경 전환

`HighlightedText`(`command-palette.tsx`)의 `<mark>` 를 배경 채움(`bg-panel-match-highlight
text-app-background`)에서 전경 강조(`bg-transparent text-panel-match-highlight font-semibold`)로
전환했다 — 브라우저 기본 `<mark>` 배경(노란 하이라이트)을 `bg-transparent` 로 명시 해제하고,
글자색만 테마 토큰으로 바꾼 뒤 굵기로 시각적 무게를 더했다.

36종 재스윕(전경색을 실제 행 배경 `panel.background` 위에 합성해 대비 계산) 결과 AA(4.5:1)
미달이 여전히 8종(`ayu-light`·`everforest-light`·`github-dark`·`github-light`·
`rose-pine-dawn`·`solarized-light`·`vscode-abyss`·`vscode-quiet-light`) 남는다 — 수치 자체는
이전 배경-채움 방식과 거의 동일(대비비는 두 색 사이의 상대휘도 비율이라 어느 쪽을 전경/배경으로
두는지에 무관하게 값이 같다). 다만 `github-dark`/`github-light` 를 제외한 6종은 이번 전환으로
근본 실패 모드(글자가 자기 배경 패치 속으로 사라져 파일명 중간이 "빈칸"처럼 보이는 현상)가
사라진다 — 전경-only 렌더는 저대비 상황에서도 글리프 윤곽이 anti-aliasing 으로 남아 배경 채움
방식보다 판독 저하가 덜하고, `font-semibold` 가 색과 무관한 2차 단서를 더한다.

**잔존 결함(범위 외, 이월)**: `github-dark`(`panel.matchHighlight: #ffd33d22`, 알파 13%)·
`github-light`(`#ffdf5d66`, 알파 40%) 는 이 토큰이 `list.highlightForeground`(불투명 전경) 가
아니라 `editor.findMatchHighlightBackground`(반투명 오버레이 전용 값)로 폴백되는 경우라
전경/배경 어느 용법으로 써도 배경에 흡수돼 낮은 대비가 유지된다. 근본 해결은
`theme-convert/mapping-tables.ts` 의 `panel.matchHighlight` 체인에 알파 배제 또는
`contrast.ts` 의 `CONTRAST_PAIRS` 편입이 필요해 이 배치(팔레트 컴포넌트 국소 수정) 범위 밖 —
차기 배치로 이월한다. `features/search/search-match-row.tsx` 의 동일 토큰 조합(`ContextLine`)도
같은 문제를 갖고 있으나 **이번 배치는 지시 범위상 `command-palette.tsx` 만 수정**했다 — 별건
이월.

### 4.4 correctness-4·5 / regression-1 (minor, 실행 재현 후 근본 수정) — fuzzy-match 인덱스 보존

`shared/lib/fuzzy-match.ts` 의 `fuzzyMatch` 를 인덱스-보존 알고리즘으로 재작성했다:

- 이전: `target.toLowerCase()` 로 문자열 전체를 한 번에 소문자화한 뒤 `indexOf` 로 검색 —
  (a) 서로게이트 쌍(이모지 등 astral 문자)이 매칭되면 상위 서로게이트 코드유닛 1개만
  `indices` 에 담겨 하이라이트 시 문자가 반으로 쪼개짐. (b) 소문자화 시 길이가 늘어나는 문자
  (예: 튀르키예어 `İ` → `i` + combining dot, 2코드유닛)가 타깃에 있으면 그 뒤 모든 인덱스가
  밀려 엉뚱한 문자가 강조됨.
- 이후: `Array.from`-등가 순회(`for...of`)로 타깃을 코드포인트 단위 배열로 분해하고 각 원소의
  **원본 코드유닛 시작 인덱스**를 함께 기록, 코드포인트 단위로 개별 `toLowerCase()` 비교. 매칭된
  코드포인트가 서로게이트 쌍이면 두 코드유닛 인덱스를 모두 `indices` 에 push(계약: "매칭된
  코드유닛 전부"). 소문자화로 길이가 늘어나는 문자는 단일 코드포인트 문자열과 문자열 동등
  비교라 애초에 매칭되지 않아(다음 문자로 자연 진행) 이후 인덱스가 밀리지 않는다.
- `fuzzy-match.test.ts` 에 실행 재현 케이스 고정: 서로게이트 쌍(🚀) 매칭 시 `indices` 2개
  코드유닛 모두 포함 + `buildFuzzyHighlightSegments` 로 세그먼트 분리해도 문자가 쪼개지지
  않음, `İstanbul.ts` 에서 `t` 매칭이 올바른 원본 인덱스(2)를 가리킴.

### 4.5 correctness-6 / regression-2 / design-5 (minor) — activeProject 로딩 게이팅

`toProjectRelativePath` 의 절대경로 폴백 자체는 유지하되(파일 열기 등 다른 소비처에 영향 없이
그대로 두는 것이 안전), **파일 결과 렌더/필터를 `activeProject` 확보 전까지 보류**하는 쪽을
택했다(검토가 제시한 두 옵션 중 "로딩 표시" — 렌더 보류보다 기존 관행과의 정합이 크다:
`symbol` 모드가 이미 `documentSymbolsLoaded` 미충족 시 `t('common.loading')` 을 보여주는
패턴을 그대로 따른다). `fileProjectRootLoaded = !activeProjectId || !!activeProject` 로
`fileRows`/`filteredFiles` 를 게이팅하고, `resolveEmptyStateMessage` 에 `files` 모드 로딩 분기를
추가했다. 이제 `activeProject` 미해결 구간에는 파일 결과 자체가 비어 있고(빈 상태 메시지 =
"로딩 중"), 해결 후에만 프로젝트 상대경로 기준으로 매칭·렌더된다 — 절대경로가 매칭 대상이나
부제로 노출되는 경로가 완전히 없어진다.

### 4.6 contract-2 (minor) — 테스트 파일 한국어 주석 제거

`command-palette-file-match.test.ts` 의 `//` 주석 3건을 제거하고 설명을 각 `test()` 설명
문자열에 흡수했다(comments.md — 코드 주석 금지, 테스트명으로 표현).

### 4.7 contract-1 (minor) — 기능 정본 문서 갱신

`docs/features/command-palette.md` 를 실코드 기준으로 갱신했다: 파일 모드 2단 렌더(파일명 +
프로젝트 상대 디렉토리), fuzzy 매칭 대상이 프로젝트 상대 경로라는 사실, 매칭 하이라이트 적용
범위(files·commands·symbol / workspaceSymbol 은 이월 제외)를 반영했다.

### 4.8 correctness-3 / contract-4 (minor) — §3.2 전제 정정

§3.2 에 정정 인용문을 추가했다(위 §3.2 참고): 인용 수치(1.40→1.92:1, "6.6배")는 `global.css`
`:root` 폴백값 = builtin `taide-dark` 한 종 기준이며, 번들 테마 36종은 런타임에 각자의 값으로
덮어써진다는 전제를 명시했다. 착수 시점 `list.activeBackground` 가드 부재 지적은 d-26b 로
해소됐음을 교차 참조했다.

### 4.9 contract-3 (minor) — 이월 기록 보완

§3.4 에 §1.4 가 이미 명시했던 "fuzzy 가중치 재설계는 범위 외" 이월 기록이 실제 구현 기록에는
빠져 있던 것을 추가했다(위 §3.4 참고).

### 4.10 검증

- `bun run verify`(typecheck → eslint → prettier check → bun test → cargo fmt → cargo clippy
  -D warnings → cargo test) exit 0.
- `bunx vite build` exit 0.
- `git diff --stat -- src/shared/api/bindings.ts` 무변경(빈 diff) 확인.
- 변경 파일: `src/widgets/command-palette/command-palette.tsx`,
  `src/shared/ui/command.tsx`, `src/shared/lib/fuzzy-match.ts`,
  `src/shared/lib/fuzzy-match.test.ts`,
  `src/widgets/command-palette/command-palette-file-match.test.ts`,
  `docs/features/command-palette.md`, 이 계약 문서. `mapping-tables.ts`·`contrast.ts`·
  `search-match-row.tsx` 는 지시 범위 밖으로 무변경(§4.3 이월 참고).
