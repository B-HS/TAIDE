# 배치 4 조사 원문 — 주제 1~5 (2026-09-04, wf_c018b9c9 opus·xhigh 읽기 전용)

> 계약 정본은 `docs/acknowledge/2026-09-04-usability-batch4-contract.md`. 라인 번호는 조사 시점(배치 3 구현 wf 진행 중) 기준.


# 주제 5 — git 탭 UX

## findings
## 1. 현재 git 탭의 섹션 구조 (파일:라인)

**패널 골격** — `src/widgets/git-panel/git-panel.tsx`

- `:152` 루트 `flex h-full min-h-0 w-full flex-col`
- `:153-186` 상단 헤더 바(h-8): `BranchSwitcher` + ahead/behind + Sync `IconButton`(`:174-184`) + remote 이름(`:185`)
- `:188-197` `CommitBox`(shrink-0, 하단 `border-b`)
- `:199` `ScrollContainer className='min-h-0 flex-1'` — **스크롤러는 하나**이고 그 안에 stash·changes·graph 가 전부 들어간다
- `:200-211` **stash 섹션이 스크롤 영역의 최상단**. 조건은 `stashes.length > 0 || canStash`
- `:212-248` `changesListRef` 를 가진 `div role='group'` — merge(`:213`)·staged(`:224`)·unstaged(`:236`) 3그룹
- `:250-269` graph 섹션. 헤더가 `ResourceGroupHeader` 가 **아니라** 인라인 마크업(`:252` `text-panel-section-header px-2 pb-1 text-[11px] font-semibold tracking-wide uppercase`)이고, 앞에 `border-t mt-2 pt-2`(`:251`)가 있다 → 현재 패널 안에 **섹션 헤더 스타일이 2종** 공존한다.

**`canStash` 의 실제 의미** — `src/widgets/git-panel/git-panel-container.tsx:244`

```
canStash={(status?.rows.length ?? 0) > 0}
```

즉 **변경이 하나라도 있으면 stash 섹션이 무조건 렌더**된다. 스태시가 0건이어도 `git-panel.tsx:200` 조건이 참이 되어 헤더 "스태시 0" + `stash-list.tsx:17` 의 빈 문구("스태시가 없습니다")가 **Changes 그룹 바로 위에** 그려진다. 반대로 워킹트리가 깨끗하고 스태시도 없으면 섹션 자체가 사라진다 → 파일을 하나 고칠 때마다 스크롤 최상단에 섹션이 나타났다 사라지는 점프가 생긴다. **이것이 "구분이 모호하다"는 보고의 최대 원인**이다(항상 첫 자리 + 대부분 비어 있음 + 아래 그룹과 동일 스타일).

## 2. 헤더와 행이 시각적으로 거의 동일하다

`src/features/git/resource-group-header.tsx:13`

```
group text-panel-section-header hover:bg-explorer-item-hover flex h-6 items-center gap-1.5 px-2 text-[11px] font-semibold tracking-wide uppercase
```

`src/features/git/status-row-item.tsx:55`

```
group hover:bg-explorer-item-hover focus-within:bg-explorer-item-focused flex h-6 w-full cursor-default items-center gap-1.5 px-2 text-xs select-none
```

- **높이 동일**(h-6=24px), **좌우 패딩 동일**(px-2), **hover 배경 동일**(`explorer-item-hover`), **들여쓰기 차이 0**.
- 차이는 글자 크기 11px vs 12px, uppercase/semibold, 그리고 카운트 뱃지(`resource-group-header.tsx:15-17`, `bg-app-sidebar-item-active rounded-full h-4 min-w-4 text-[10px]`)뿐이다.
- 배경 톤 차·구분선·chevron·들여쓰기 중 **어느 것도 없다**. 스크롤 중 헤더가 위로 밀려 사라지므로(sticky 아님) 긴 목록에서는 지금 보고 있는 행이 staged 인지 unstaged 인지 알 방법이 없다.

## 3. 접기/펼치기가 전혀 없다

- `ResourceGroupHeader` 는 `title/count/actionLabel/actionIcon/onAction` 5개 prop 뿐(`:4-10`)이고 `expanded`·`onToggle`·`aria-expanded` 가 없다.
- 반면 같은 저장소에 **이미 접이식 그룹 헤더 선례**가 있다 — `src/shared/ui/file-group-header.tsx:18-34`(`ChevronRight` + `rotate-90`, `role='button'`, `tabIndex=0`, `aria-expanded`, `createActivationKeyDownHandler`). 검색·문제 패널이 이것을 쓴다(`src/features/search/search-results-list.tsx:66`, `src/features/problems/problems-panel.tsx:100`). git 패널만 이 패턴을 안 쓰고 있다.

## 4. 그룹 hover 액션은 키보드로 도달 불가

`resource-group-header.tsx:24` — `containerClassName='ml-auto hidden group-hover:flex'`.
`hidden` 이 tab order 에서 제거하는데 `group-focus-within:` 변형이 없다. 행 액션(`status-row-item.tsx:76` `hidden ... group-hover:flex group-focus-within:flex`)은 focus-within 이 있어 키보드로 닿지만, **Stage All / Unstage All / Stash 는 마우스 전용**이다. 2026-08-29 키보드 내비 작업(`features/git.md` §2 "키보드")과 정합이 깨져 있다.

## 5. 키보드 내비의 현행 계약

`src/widgets/git-panel/change-row-navigation.ts:8-12` 는 순수 함수(인덱스만 다룸)이고, 실제 DOM 질의는 `git-panel.tsx:141`:

```
changesListRef.current?.querySelectorAll<HTMLElement>('[data-git-change-row]')
```

- stash 섹션은 `changesListRef` **밖**(`:200-211`)이라 ↑↓ 로빙에 포함되지 않는다 — 즉 키보드 상으로는 이미 "다른 영역"인데 화면에서만 그 사실이 드러나지 않는다.
- `change-row-navigation.ts:3-4` doc comment 는 `activeIndex = -1` 을 "스크롤 컨테이너나 **그룹 헤더**에 포커스가 있는 경우"로 정의한다. **헤더를 포커스 가능하게 만들면** ↓ 가 항상 목록 맨 위로 점프하는 오동작이 되므로(중간 헤더에서 ↓ → 첫 그룹 첫 행), 헤더를 접이식으로 만드는 순간 이 계약과 doc comment·테스트를 함께 손봐야 한다.

## 6. 테마 토큰 — 신규 토큰 없이 대비를 올릴 수 있는가

`src/shared/styles/global.css` 의 폴백 값(런타임에 테마가 `--taide-*` 를 재주입 — `docs/theme-system.md` §4.1):

- `:35-37` `explorer-item-hover #313244` / `item-selected #585b70` / `item-focused #45475a`
- `:43` `panel-section-header #a6adc8`, `:34` `explorer-background #181825`, `:9` `app-border #313244`
- `@theme inline` 노출은 `:182-184`, `:190`

즉 **헤더 배경으로 쓸 "표면" 토큰이 이미 3단(hover/focused/selected) 있다.** 다만 `theme-system.md:190` 이 말하는 "테마 토큰 5곳 동기" 때문에 **신규 토큰 추가는 비싸고**, `theme-system.md` §8.2.2·§8.2.4 는 vsix 변환 테마에서 list 상태 배경이 배경색과 충돌해 손수정한 전례를 기록한다 → **정적 헤더 배경으로 `explorer-item-hover` 를 쓰면 일부 임포트 테마에서 행 hover 피드백과 구분이 사라질 위험**이 있다. 구분선(`border-app-border`) + 들여쓰기 + sticky 조합이 테마 안전하다.

## 7. sticky 가능 여부

`src/shared/scroll/scroll-container.tsx:33-41` — 스크롤포트는 `overflow-y-auto overflow-x-hidden` 을 **직접 가진 뷰포트 div** 이고, 그 사이에 다른 overflow 조상이 없다. 같은 저장소에 `src/widgets/settings-view/settings-view.tsx:166` 이 `ScrollContainer` 안에서 `sticky top-8` 을 실제로 쓰고 있다 → **`position: sticky` 는 그대로 동작**한다. 각 그룹이 이미 자기 `div` 로 감싸져 있으므로(`git-change-group.tsx:160`) 헤더에 `sticky top-0` 만 주면 "다음 섹션이 이전 헤더를 밀어내는" VS Code 식 동작이 자연스럽게 나온다. 단 헤더가 투명하면 아래 행이 비쳐 보이므로 `bg-explorer-background`(패널 루트가 쓰는 값 — `src/widgets/explorer/explorer-panel.tsx:126`)를 명시해야 한다.

## 8. i18n — 신규 키 0으로 가능

`src-tauri/resources/locales/{ko,en,ja}.json` 에 이미 존재:
`git.changes`(:256) `git.stagedChanges`(:304) `git.mergeChanges`(:286) `git.stash`(:305) `git.stashEmpty`(:308) `git.noChanges`(:287).

특히 **`git.noChanges` 는 카탈로그(`src-tauri/src/domain/locale/service.rs:132`)에 등록돼 있으나 프론트 소비처가 하나도 없다**(현재 사용처는 `error.git.noChanges` 뿐). 빈 상태 문구에 그대로 재사용하면 신규 키가 0이다. 새 키 1종을 추가하려면 JSON 3개 + `locale/service.rs` 카탈로그 + 파리티 테스트(`service.rs:1294` `en_메시지의_모든_키는_required_message_keys에_포함된다`)를 동시에 건드려야 한다.

## 9. 접힘 상태 영속의 선택지와 비용

- **컴포넌트 state**: 사이드바 뷰를 파일/검색으로 바꾸면 `GitPanelContainer` 가 언마운트된다(`src/entities/git/commit-message-memory.ts:13-15` 가 같은 사실을 기록) → 매번 초기화.
- **모듈 스코프 메모리**: 같은 문제를 이미 이렇게 푼 선례가 `entities/git/commit-message-memory.ts:23-42`(Map + LRU 상한). IPC·설정·i18n 비용 0, 앱 재시작 시 초기화.
- **설정 필드**: `src-tauri/src/domain/settings/types.rs:150-195` 형태로 필드 추가 → Rust 타입 + `#[serde(default)]` + patch 타입(`:339` 근처) + specta 바인딩 재생성 + 설정 UI 노출 판단. 재시작 후에도 유지되지만 비용이 크다.
- **레이아웃 영속**: `src-tauri/src/domain/layout/types.rs:146-159` 의 `ShellViewState`/`ShellViewPatch` 가 후보이나 `ProjectLayout.version`(`:180`) 마이그레이션(`service.rs:1161`)이 걸리고, **지금 다른 워크플로가 layout 도메인을 편집 중**이라 충돌 위험이 크다.

## 10. 테스트 환경 제약

`src/shared/ui/error-boundary.test.tsx:6-10` 이 명시한다 — **이 프로젝트에는 DOM 렌더 하네스가 없다**(`@testing-library/react`·jsdom·happy-dom 모두 없음). e2e(`e2e/specs/07-git-stage-unstage.e2e.ts`)도 IPC 레벨 검증이다. 따라서 이번 UX 변경의 검증은 `change-row-navigation.ts` 처럼 **순수 함수로 분리해 `bun test`** 하는 방식만 가능하다(섹션 가시성·카운트·접힘 해석 등).

## 11. 문서상의 설계 의도

`docs/features/git.md` §2 는 구성을 "헤더 → 커밋 입력 → 리소스 그룹(Merge/Staged/Changes) → Graph" 로 규정하고 **stash 섹션을 아예 언급하지 않는다**(stash UI 는 §8 표에서 2차 항목으로 구현 완료만 기록). `docs/research/vscode-behaviors.md:22-44` 도 VS Code SCM 을 **3개 리소스 그룹으로만** 문서화하며 stash 리소스 그룹은 없다. `docs/roadmap.md:222` 만 "git 패널 **상단** stash 섹션"이라 적어 현재 배치의 유일한 근거다. `docs/PROCESS.md:53-63` 은 이 주제를 "사용성 배치 4 ⑤"로 등록하며 **"git 탭은 VS Code SCM 파리티 전제"** 라고 이미 방향을 못박았다. 백로그(`docs/backlog.md`)·2026-08-29 감사에는 이 항목의 중복 기재가 없다.

## options
## A안 — VS Code SCM 파리티: 접이식 리소스 그룹 통일 + Stash 강등 (추천)

**핵심**: 섹션 헤더를 한 컴포넌트로 통일(제목 + 카운트 뱃지 + chevron + sticky + hover/focus 액션)하고, **Stash 를 변경 그룹 아래의 독립 접이식 섹션으로 내리며 스태시가 0건이면 아예 렌더하지 않는다.** 그 대신 "변경사항 스태시" 액션은 저장소 레벨 액션이므로 **상단 헤더 바(Sync 옆)** 로 옮긴다.

변경 파일과 데이터 흐름:

- `features/git/resource-group-header.tsx` — props 확장: `expanded: boolean`, `onToggle: () => void`, `actions: { id, label, icon, onClick }[]`(단일 `actionLabel/actionIcon/onAction` 대체). 마크업은 `file-group-header.tsx:21-33` 패턴 복제 — `role='button'` + `tabIndex=0` + `aria-expanded` + `createActivationKeyDownHandler` + `ChevronRight`(`expanded && 'rotate-90'`). 클래스에 `sticky top-0 z-10 bg-explorer-background border-app-border border-t` 추가, 액션 컨테이너는 `hidden group-hover:flex group-focus-within:flex` 로 키보드 도달 보장(현행 결함 §4 동시 해소).
- `features/git/git-change-group.tsx` — `expanded`/`onToggle` 을 그대로 헤더에 전달하고 `expanded === false` 면 행 렌더를 건너뛴다. 행 컨테이너에 `pl-4`(헤더는 `px-2`) 들여쓰기 부여 → 계층이 눈에 보인다.
- `features/git/stash-list.tsx` — 빈 상태 분기(`:17`) 제거(섹션 자체가 안 그려지므로), `li` 에 동일 `pl-4` 적용.
- `widgets/git-panel/git-panel.tsx` — 순서를 `Merge → Staged → Changes → Stashes → Graph` 로 재배치. stash 섹션 조건을 `stashes.length > 0` 로 좁힘. `canStash` 는 상단 헤더 바의 Stash `IconButton`(`Archive` 아이콘, `disabled={!canStash}`) 게이팅으로 이동. graph 헤더(`:252`)도 같은 `ResourceGroupHeader` 로 통일(접이식). 키 핸들러의 셀렉터를 `'[data-git-change-row], [data-git-section-header]'` 로 확장해 헤더가 로빙 시퀀스에 들어오게 하고, `ArrowLeft/ArrowRight` 로 접기/펼치기.
- `entities/git/git-section-collapse-memory.ts`(신규) — `commit-message-memory.ts` 와 동형의 모듈 스코프 `Map<GitSectionId, boolean>`. 기본값: merge/staged/changes/graph 펼침, stashes 접힘. 뷰 전환·프로젝트 전환에도 유지, 앱 재시작 시 초기화.
- `widgets/git-panel/git-sections.ts`(신규, 순수) + 테스트 — `rows`·`stashes`·`collapsed` 를 받아 "어떤 섹션이 어떤 카운트로 보이는가"를 반환. DOM 하네스가 없는 이 저장소에서 유일하게 가능한 회귀 검증 지점(§10).
- `widgets/git-panel/change-row-navigation.ts` — 함수 시그니처는 무변경(인덱스 기반이라 헤더를 섞어도 그대로 동작). doc comment 의 "그룹 헤더 = activeIndex -1" 문구만 정정 + 테스트 보강.

장점: 사용자가 이미 확정한 "VS Code SCM 파리티" 전제와 정확히 일치. 빈 stash 섹션이 사라져 보고된 모호함의 최대 원인이 근본 제거된다. 접기로 대형 저장소에서 DOM 행 수도 줄어든다. 신규 i18n 키 0·신규 의존성 0.
단점: 변경 표면이 가장 넓다(6파일 + 신규 2). Stash 버튼 위치가 바뀌어 기존 사용자에게 재학습 비용이 1회 발생한다. 키보드 로빙 규칙이 바뀌어 회귀 위험이 있다.

## B안 — 최소 변경: 시각 구분만 (접기 없음)

**핵심**: 구조·순서·상태를 그대로 두고 **구분선 + sticky + 들여쓰기 + 빈 stash 숨김** 4가지만 적용한다.

변경 파일: `resource-group-header.tsx`(sticky + `border-t` + 액션 focus-within), `git-change-group.tsx`·`stash-list.tsx`(행 `pl-4`), `git-panel.tsx`(stash 조건을 `stashes.length > 0` 로 좁히고 Stash 버튼을 상단 바로 이동, graph 헤더 통일).
데이터 흐름 변화: **없음**(상태 추가 0, 신규 파일 0, 신규 테스트 대상 0).

장점: 위험이 가장 낮고 diff 가 작다. 접힘 영속이라는 결정 자체가 필요 없어진다. 키보드 로빙 계약을 건드리지 않는다.
단점: 긴 목록에서 "Staged 를 접고 Changes 만 보기" 같은 VS Code 의 핵심 동선을 못 준다. 파리티 전제(§11)에 미달이라 나중에 A안을 다시 하게 될 가능성이 높다.

## C안 — Stash 를 SCM 목록 밖으로 완전 분리

**핵심**: stash 를 스크롤 목록에서 빼고 **패널 하단 고정 드로어**(또는 `BranchSwitcher` 옆의 별도 팝오버)로 옮긴다. 변경 그룹 목록에는 stash 가 일절 등장하지 않는다.

변경 파일: `git-panel.tsx`(하단 고정 영역 신설 또는 팝오버 트리거), `stash-list.tsx`(팝오버/드로어 레이아웃), `resource-group-header.tsx`(변경 그룹 3종만 담당).
장점: 두 영역의 혼동이 구조적으로 0이 된다. 스크롤 점프도 사라진다.
단점: VS Code·Fork 어느 쪽 참조 모델과도 다른 자체 UX 라 §11 의 파리티 전제에서 이탈한다. 하단 고정 드로어는 커밋 박스 + 그래프와 세로 공간을 다투어 사이드바 폭·높이에서 손해가 크다. 팝오버로 가면 스태시 존재 자체의 발견성이 떨어진다.

## recommendation
**A안(접이식 리소스 그룹 통일 + Stash 강등)을 추천합니다.**

근거 4가지입니다.

1. **사용자 전제와 일치**. `docs/PROCESS.md:63` 이 이 주제를 "git 탭은 VS Code SCM 파리티 전제"로 이미 못박았고, `docs/research/vscode-behaviors.md:22-44` 가 정의한 참조 모델(3 리소스 그룹 + 카운트 뱃지 + hover 액션)에 접이식 헤더만 얹으면 정확히 파리티가 됩니다.
2. **보고된 모호함의 원인을 근본 제거**합니다. 원인은 색 대비가 아니라 배치입니다 — `git-panel-container.tsx:244` 의 `canStash = rows.length > 0` 때문에 **스태시가 0건이어도 변경이 있으면 stash 섹션이 목록 맨 위에 뜹니다**(`git-panel.tsx:200`). 색만 손보는 것은 증상 완화이고, "빈 stash 섹션을 그리지 않는다 + Changes 아래로 내린다"가 근본 수정입니다.
3. **비용이 낮습니다**. 접이식 헤더는 `shared/ui/file-group-header.tsx:18-34` 라는 동작 검증된 선례가 이미 있고, sticky 는 `settings-view.tsx:166` 에서 같은 `ScrollContainer` 조합으로 이미 돌고 있으며, 필요한 i18n 키 6종은 3개 로케일에 전부 존재합니다(§8). **신규 의존성 0, 신규 i18n 키 0, Rust 변경 0, IPC 변경 0**입니다.
4. **기존 결함 2건을 같은 작업으로 정리**합니다 — 그룹 액션의 키보드 도달 불가(§4)와 graph 헤더만 다른 마크업을 쓰는 이중 스타일(§1)입니다.

세부 확정안:

- **섹션 순서**: Merge Changes → Staged Changes → Changes → Stashes → Graph.
- **헤더 사양**: `sticky top-0 z-10 bg-explorer-background` + `border-t border-app-border` + chevron(`rotate-90`) + 기존 카운트 뱃지 유지 + 액션은 `hidden group-hover:flex group-focus-within:flex`. **정적 배경 톤 차는 넣지 않습니다** — `theme-system.md` §8.2.2·§8.2.4 가 기록한 vsix 임포트 테마의 list 상태 배경 충돌 위험(§6) 때문이며, 구분선 + sticky + 행 `pl-4` 들여쓰기만으로 계층이 충분히 드러납니다.
- **빈 섹션**: 변경 그룹은 0건이면 렌더하지 않습니다(현행 유지). 저장소가 완전히 깨끗해 3그룹이 모두 비면 기존 미사용 키 `git.noChanges` 로 "변경사항 없음" 한 줄을 그립니다. Stashes 섹션은 `stashes.length > 0` 일 때만 렌더하고, Stash 실행 버튼은 상단 헤더 바(Sync 옆, `disabled={!canStash}`)로 이동합니다.
- **접힘 영속**: `entities/git/git-section-collapse-memory.ts` 모듈 스코프 메모리(재시작 시 초기화). `commit-message-memory.ts` 와 같은 계층·같은 수명 모델이라 새 개념을 도입하지 않고, 설정/레이아웃 스키마를 건드리지 않아 **지금 layout 도메인을 편집 중인 다른 워크플로와 충돌하지 않습니다**. 기본값은 stashes 만 접힘.
- **검증**: DOM 하네스가 없으므로(§10) 섹션 가시성·카운트·접힘 해석을 `widgets/git-panel/git-sections.ts` 순수 함수로 분리해 `bun test` 로 잠급니다. 키보드 로빙은 셀렉터 확장 + `change-row-navigation.test.ts` 보강으로 커버합니다.

B안은 A안의 시각 레이어와 정확히 부분집합이므로, 일정이 부족하면 A안을 B안 범위까지만 먼저 착지시키고 접힘·영속을 후속으로 미룰 수 있습니다(되돌릴 작업 없음).

## filesToTouch
- /Users/hyunseokbyun/development/TAIDE/src/features/git/resource-group-header.tsx
- /Users/hyunseokbyun/development/TAIDE/src/features/git/git-change-group.tsx
- /Users/hyunseokbyun/development/TAIDE/src/features/git/stash-list.tsx
- /Users/hyunseokbyun/development/TAIDE/src/widgets/git-panel/git-panel.tsx
- /Users/hyunseokbyun/development/TAIDE/src/widgets/git-panel/git-sections.ts
- /Users/hyunseokbyun/development/TAIDE/src/widgets/git-panel/git-sections.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/git/git-section-collapse-memory.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/git/git-section-collapse-memory.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/widgets/git-panel/change-row-navigation.ts
- /Users/hyunseokbyun/development/TAIDE/src/widgets/git-panel/change-row-navigation.test.ts
- /Users/hyunseokbyun/development/TAIDE/docs/features/git.md
- /Users/hyunseokbyun/development/TAIDE/docs/PROCESS.md

## risks
- 키보드 로빙 회귀 — `change-row-navigation.ts:3-4` doc comment 가 '그룹 헤더 포커스 = activeIndex -1' 로 정의돼 있어, 헤더를 포커스 가능하게 만들면 중간 헤더에서 ↓ 를 눌렀을 때 목록 맨 위로 점프한다. 셀렉터를 `'[data-git-change-row], [data-git-section-header]'` 로 확장하고 doc comment·테스트를 함께 고치지 않으면 2026-08-29 에 확정한 내비 계약(`features/git.md` §2)이 깨진다.
- 섹션을 접은 채로 커밋 — Staged 를 접어 둔 상태에서 사용자가 '스테이지된 게 없다'고 오인하면 `commit-gate.ts` 의 `confirmStageAll` 경로로 흘러 워킹트리 전체를 스테이지·커밋할 수 있다. 헤더 카운트 뱃지는 접힌 상태에서도 항상 보이게 유지해야 하고(A안 사양), `resolveCommitGate` 자체는 rows 기반이라 무변경이므로 게이트 로직은 안전하다.
- 테마 대비 붕괴 — 헤더에 정적 배경(`explorer-item-hover` 등)을 주면 `theme-system.md` §8.2.2·§8.2.4 가 기록한 vsix 임포트 테마의 list 상태 배경 충돌 사례처럼 일부 테마에서 헤더 배경과 행 hover 가 같은 색이 되어 오히려 구분이 사라진다. 추천안은 구분선+들여쓰기+sticky 만 쓰므로 이 위험을 회피하지만, 배경 톤을 넣기로 결정하면 번들 테마 전량(`src-tauri/resources/themes/*.json`) 육안 확인이 필요하다.
- sticky 헤더의 배경 누락 — 헤더에 `bg-*` 를 명시하지 않으면 투명이라 스크롤된 행이 헤더 뒤로 비친다. 패널 루트가 `bg-explorer-background`(`explorer-panel.tsx:126`)이므로 반드시 같은 토큰을 헤더에도 지정해야 한다.
- Stash 버튼 이동에 따른 발견성 저하 — 현재 stash 실행은 stash 섹션 헤더 hover 액션(`resource-group-header.tsx:18-27`)뿐이다. 상단 헤더 바로 옮기면 기존 사용자가 한 번은 찾아야 한다. 툴팁 라벨(`git.stashPush`)은 그대로 유지되고, 상단 바는 이미 Sync 같은 저장소 레벨 액션의 자리라 개념적으로는 더 맞다.
- 접힘 메모리의 전역성 — 모듈 스코프 단일 레코드는 프로젝트 A 에서 접은 상태가 프로젝트 B 에도 적용된다. 뷰 설정 성격이라 의도된 동작으로 보지만, 프로젝트별 분리를 원하면 `commit-message-memory.ts` 처럼 `ProjectId` 키 + 상한(LRU)으로 바꿔야 한다.
- 다른 워크플로와의 파일 충돌 — `src/widgets/git-panel/git-panel.tsx` 는 이번 배치의 다른 주제와 겹치지 않지만, 같은 세션에서 `entities/layout`·`entities/file` 을 편집 중인 워크플로가 있다. 추천안은 layout/settings/Rust 를 일절 건드리지 않으므로 충돌 표면이 프론트 git 파일 6개로 한정된다.
- 검증 공백 — DOM 렌더 하네스가 없어(`error-boundary.test.tsx:6-10`) sticky·chevron·포커스 이동은 자동 검증이 불가능하다. 순수 함수 테스트로 커버되는 것은 섹션 가시성·카운트·접힘 해석뿐이며, 나머지는 손 QA 체크리스트(`docs/quality-assurance/`)로 남겨야 한다.

## questionsForUser
- Stash 섹션 위치 — **Changes 아래로 이동(추천: 참조 모델인 VS Code SCM 에는 stash 리소스 그룹 자체가 없고, 빈 섹션이 목록 첫 자리를 차지하는 것이 이번 보고의 최대 원인)** / 현행 최상단 유지(구분 강화만)
- 빈 Stash 섹션 처리 — **스태시 0건이면 섹션을 렌더하지 않고 Stash 실행 버튼은 상단 헤더 바로 이동(추천)** / 섹션은 항상 표시하되 '스태시가 없습니다' 문구 유지
- 접힘 상태 영속 범위 — **모듈 스코프 메모리(뷰·프로젝트 전환에는 유지, 앱 재시작 시 초기화 — `commit-message-memory.ts` 선례, 비용 0) (추천)** / 설정 필드(재시작 후에도 유지, Rust `Settings` 필드+patch+바인딩 재생성 필요) / 컴포넌트 state(사이드바 뷰를 바꿀 때마다 초기화)
- 기본 접힘 상태 — **Stashes 만 접힘, 나머지 전부 펼침(추천)** / 전부 펼침 / Staged·Changes 만 펼침
- 헤더 대비 방식 — **구분선(border-t) + sticky + 행 들여쓰기(pl-4)만, 배경 톤 차 없음(추천: 임포트 테마 충돌 위험 회피)** / 여기에 헤더 정적 배경 톤(`explorer-item-hover`)까지 추가
- 섹션 헤더 라벨 — **기존 키 `git.stash`(스태시/Stash/スタッシュ) 재사용, 신규 i18n 키 0(추천)** / 'STASHES' 복수형 신규 키 1종을 3언어+카탈로그에 추가
- ↑↓ 로빙에 섹션 헤더 포함 여부 — **포함 + ←/→ 로 접기·펼치기(추천: VS Code 트리 동작과 동일, 헤더 액션 키보드 도달도 함께 해결)** / 현행처럼 행만 순회(헤더는 Tab 으로만 도달)
- Graph 섹션 헤더도 같은 접이식 컴포넌트로 통일할지 — **통일(추천: 현재 인라인 마크업이라 스타일 이중화)** / 현행 유지

## newDependencies



# 주제 2 — Welcome 커맨드·탭 0 자동 표시

## findings
> 읽기 전용 조사. 병행 워크플로가 `pane-tree.ts`·`layout` 등을 편집 중이라 **행 번호는 ±수 행 이동 가능**(예: `isPaneTreeEmpty` 는 조사 중 55→68 행으로 밀림). 인용은 심볼명 기준으로 재확인할 것.

## 1. Welcome 은 이미 "탭 kind" 다 (별도 상태 아님)

- `src-tauri/src/domain/layout/types.rs:72` — `TabKind::Welcome` (unit variant, 필드 없음). `src/shared/api/bindings.ts:2209` 에 `{ kind: "welcome" }` 로 생성됨.
- `src-tauri/src/domain/layout/service.rs:110-147` `default_layout()` — 새 프로젝트 기본 레이아웃은 Leaf 1개에 `[Welcome, Terminal]`, active = Welcome. **탭 제목은 `service.rs:114` 에서 `"Welcome"` 하드코딩(영어, 비로케일)**.
- 렌더러 2곳(중복 없음, d-27 계약 §1.2 "적용 면 2곳 통일"):
  - `src/widgets/editor-area/pane-node-view.tsx:145-149` — `activeTab.kind.kind === 'welcome'` → `<Suspense><WelcomeContainerLazy projectId={projectId} /></Suspense>`
  - `src/widgets/app-shell/app-shell.tsx:172-181` — `projects.length === 0` 전체화면 → `WelcomeContainerLazy projectId={null}`
- `src/widgets/welcome/welcome-container-lazy.ts:13` — `lazy()` 래퍼가 이미 공용(부팅 청크에서 분리됨, audit §1-1).
- `src/widgets/welcome/welcome-container.tsx:33-39` — `projectId: ProjectId | null` 이 유일한 prop. `null` = 프로젝트 0개 화면(파일 열기 비활성), 실 id = 프로젝트 내부 welcome 탭.

## 2. 3가지 "빈" 상태는 코드상 이미 구분돼 있다

`app-shell.tsx` 분기 기준:

| 상태 | 조건 | 현재 렌더 |
|------|------|-----------|
| 프로젝트 0개 | `projects.length === 0` (`app-shell.tsx:172`) | 전체화면 Welcome |
| 프로젝트는 있으나 활성 없음 | `activeProjectId == null` (`app-shell.tsx:221-222`) | `app.selectProject` 텍스트 |
| **활성 프로젝트 + 탭 0** | `PaneNodeView` 의 `!activeTab` | `app.selectProject` 아님 — `editor.noFileOpen` 텍스트 (`pane-node-view.tsx:207`) |

**→ 주제 2-(ii) 가 채워야 할 자리는 정확히 `pane-node-view.tsx:207` 한 줄이다.**

## 3. `!activeTab` == "이 창의 트리 전체가 빈 것" (스플릿 빈 pane 은 존재할 수 없음)

- `normalize_owned` (`service.rs:364-425`) 가 split 의 자식 중 `is_empty_leaf` 를 전부 제거하고, 자식이 1개면 승격·0개면 빈 Leaf 로 접는다.
- `normalize` 호출 지점: `service.rs:510`(untitled 변환) · `530`(close_tab) · `810/818/836/872`(move_tab / move_tab_to_window) — **탭이 빠질 수 있는 모든 경로가 normalize 를 탄다.**
- `extract_tab` (`service.rs:273-289`) — `active` 가 `None` 이 되는 유일한 경우는 `tabs.is_empty()`.
- 결론: `activeTab === null` ⟺ 루트가 빈 Leaf ⟺ 그 창의 pane 트리에 탭이 0개. **스플릿 중 한쪽만 빈 pane 은 만들어질 수 없다.** 이게 (ii) 를 pane 단위 렌더로 풀어도 안전한 근거다.

## 4. 보조창은 빈 트리가 되면 스스로 닫힌다 — (ii) 설계의 핵심 제약

- `src/widgets/auxiliary-window-shell/auxiliary-window-shell.tsx:44-50` — `isPaneTreeEmpty(paneTree.root)` 이면 `getCurrentWindow().close()`. (Wave I 계약 §3.2 "마지막 탭 이동/닫기 시 창 정리" 의 프론트 절반)
- Rust 쪽 `cleanup_emptied_auxiliary_windows` (`layout/commands.rs:259-276`) 는 *탭 이동* 경로만 담당.
- **따라서 "탭 0 이면 Welcome 탭을 자동으로 넣는다"(옵션 B/C) 를 보조창 트리에도 적용하면 보조창이 영원히 안 닫힌다 — Wave I 계약 회귀.** 반드시 메인 트리로 스코프해야 한다.
- 반대로 옵션 A(빈 상태 렌더 교체)는 레이아웃을 안 건드리므로 이 자동 닫힘이 그대로 유지된다. 단 닫히기 전 1프레임 동안 보조창에 Welcome 이 번쩍일 수 있고, `WelcomeContainer.handleSelectRecent` (`welcome-container.tsx:76-83`) 가 `useOpenProject`/`useActivateProject` 로 **전역 활성 프로젝트 세션을 바꾸는데, 이는 `app.tsx:26-33` 이 보조창에 금지한 바로 그 동작**이다 → A 는 메인 창으로 게이팅해야 한다.

## 5. 커맨드 레지스트리 — 추가 절차 (정확히 5곳 + 로케일)

- `src/shared/lib/command-registry.ts:4-12` `CommandContext` = `{ activeProjectId, activeEditorActionIds, openSettingsTab, openSettingsFile, openTerminalTab, reopenClosedTab, switchToFileSearchMode }`. 14-22 `AppCommand` = `{ id, titleKey, categoryKey?, keymapId?, titleDefaultValue?, run, isEnabled? }`.
- `src/shared/lib/command-catalog.ts:17-174` `DEFAULT_COMMANDS` — **현재 정확히 23개**. `view.*` 선례: `view.toggleSidebar`(63-68) · `view.explorer`(85-91) · `view.git`(92) · `view.toggleTerminal`(121-127) · `view.toggleZenMode`(161-173). 카테고리는 `KEYMAP_CATEGORY.VIEW` (`keymap-category.ts:3`).
- `src/app/bootstrap-commands.ts:11` `registerCommands(DEFAULT_COMMANDS)` — 등록 지점.
- 탭 여는 커맨드 선례(`command-palette.tsx:152-181`): `openTerminalTab`/`openSettingsTab`/`openSettingsFile` 셋 다 **`activeProjectId` 없으면 `toast.info(t('app.openProjectFirst'))` 후 return**, 있으면 `openTab({ projectId, kind, title, target: null, preview: false }, { onError: describeIpcError })`. `commandContext` 조립은 `command-palette.tsx:197-205`.
- `keymapId` 없는 커맨드는 `buildKeybindingRows` (`keybinding-catalog.ts:38-59`) 가 `runsViaCommand: true` · `key: ''` 행으로 만들어 **키바인딩 에디터에서 사용자가 나중에 바인딩 가능**하고, 디스패치는 `command-palette.tsx:216-233` 의 두 번째 캡처 리스너가 담당한다. → 기본 단축키 없이 커맨드만 추가해도 구조상 완결.
- 팔레트는 **메인 창에만 마운트**된다(`app.tsx:34-38, 91`). 원격 브라우저 세션은 쿼리스트링이 없어 `getWindowContext()` 가 `{kind:'main'}` 으로 읽히므로(`window-context.ts:34`) 팔레트가 뜬다.

## 6. 로케일 — `app.welcome` 키는 과거에 **삭제**됐다

- `src-tauri/resources/locales/{en,ko,ja}.json` 각 968키. `app.*` 목록(en.json:22-37)에 `welcome` **없음**. 있는 건 `errorBoundary.welcome`(en `Welcome` / ko `시작 화면` / ja `ようこそ画面`, 각 파일 212행).
- `docs/acknowledge/2026-08-19-t2i-locale-externalization-contract.md:79-80` — `app.welcome` 은 T2-I 미참조 32키 정리에서 **3언어 + `MESSAGE_NAMESPACES` 동반 제거**됨.
- 재추가 시 필수 3곳: `src-tauri/src/domain/locale/service.rs:13-47` 의 `MESSAGE_NAMESPACES` `"app"` 배열 + en/ko/ja json. 누락하면 `service.rs:1287-1301` 의 두 테스트(필수키 존재 / 카탈로그키 ⊆ 필수키)가 양방향으로 잡는다.

## 7. 탭 중복 정책 · 제목 — `view.welcome` 설계에 직접 영향

- `open_tab` (`service.rs:428-458`): **같은 leaf 안에서 `existing.kind == tab.kind` 면 기존 탭을 재사용·활성화**하고 새로 만들지 않는다. `TabKind::Welcome` 은 unit variant라 항상 일치 → 포커스 pane 에 Welcome 이 이미 있으면 그 탭이 활성화된다(멱등).
- 다만 **재사용 경로는 기존 탭의 `title` 을 갱신하지 않는다**(`service.rs:435-442`). 즉 팔레트에서 로케일 제목을 넘겨도 복원된 `"Welcome"` 탭에는 반영되지 않아 **제목이 섞인다**. 설정 탭 선례(`title: t('settings.title')`)는 Rust 가 만드는 일이 없어 이 문제가 없다.
- 다른 pane 에 Welcome 이 있어도 포커스 pane 에는 새로 하나 더 생긴다(설정 탭과 동일한 기존 동작).
- `layout_open_tab` 의 존재 검증(`layout/commands.rs:26-40` `ensure_file_tab_target_exists`)은 **`TabKind::File` 에만** 적용 — welcome 탭은 그대로 통과.
- `layout_open_tab` 은 원격 허용 커맨드다(`remote/dispatch.rs:27`, 570, 838).

## 8. 탭을 모두 닫는 경로 · 자동 표시와 충돌 지점

- `pane-tab-bar.tsx:154-158` `handleCloseAll` — **캡처한 `tabs` 배열을 순차 await 로 닫는다**(pinned 제외). 중간에 새 탭이 끼어들어도 그 탭은 닫지 않는다.
- `editor-area.tsx:104-114` `closeFocusedTab`(⌘W) — pinned 는 `tab.pinnedCloseBlocked` 경고 후 유지 → pinned 가 하나라도 있으면 트리가 비지 않는다.
- `reopen_closed` (`service.rs:555-575`) — 닫은 탭 스택에서 pop 해 **기록된 pane/index 로 삽입**. 자동으로 넣은 Welcome 탭이 있으면 ⇧⌘T 후 `[Welcome, 복원탭]` 이 된다(자동 삽입분을 식별할 마커가 `Tab` 에 없어 되돌릴 수 없음 — 되돌리려면 스키마 필드 추가).
- `is_volatile` (`service.rs:1062-1064`) 은 `ClaudeDiff` 뿐. hot-exit 영속(`strip_volatile_tabs`, `service.rs:1105-1118`)은 Welcome 탭을 그대로 저장하고, 빈 보조창 엔트리만 버린다. `load_layout` (`service.rs:1193-1206`) 은 파싱 불가/미래 버전일 때만 `default_layout()` 로 폴백 — **"탭 0으로 저장된 레이아웃" 은 그대로 탭 0으로 복원된다.**
- 탭 0 상태에서도 `PaneTabBar` 는 렌더되고 `TabBarAddMenu`(`pane-tab-bar.tsx:260`)가 남아 있어 탈출구는 확보돼 있다.

## 9. 기존 문서·백로그와의 중복 여부

- `docs/PROCESS.md:53-63` — **이 주제가 "대기: 사용성 배치 4" 의 항목 ②(Welcome 커맨드 + 탭 0 자동 표시)로 이미 큐잉**돼 있고, 계약 문서는 `acknowledge/2026-09-04-usability-batch4-contract.md` 예정. 백로그·감사 발견에는 중복 항목 **없음**(`docs/backlog.md` 전문·`quality-assurance/2026-08-29-full-audit.md` 확인 — 후자의 welcome 언급은 lazy 분리 1건뿐, 28행).
- Welcome 전용 feature 문서는 없다. 정본은 `docs/acknowledge/2026-08-20-welcome-page-contract.md`(d-27) + `docs/features/layout-shell.md:56,64,70`.
- **문서 부정합 발견**: `docs/features/tabs.md:17` 은 기본 레이아웃을 "`[File(웰컴/빈 에디터), Terminal]`" 로 적었지만 실제는 `TabKind::Welcome` 이다(`File` 아님). `docs/PRD.md:85` FR-B2 도 "1 파일뷰 + 1 터미널" 표현. 이번 배치에서 같이 정정 대상.
- `docs/features/command-palette.md` §2.1 은 "실등록 23종" 과 영역별 id 표를 정본으로 명시 — 커맨드 추가 시 **반드시 동반 갱신**.

## 10. 설정 1종 추가 시 실제 touchpoint (선례 `explorer_auto_reveal` 실측)

`src-tauri/src/domain/settings/types.rs:195`(필드) · `:339`(Patch) · `:445`(Default) → `settings/service.rs:403`(apply_patch) → `sync/service.rs:136`(gist 내보내기) → `locale/service.rs:409-410`(키 2종) → `resources/locales/{en,ko,ja}.json` → `bindings.ts`(specta 자동생성) → `src/entities/settings/settings.ipc.ts:49`(`emptySettingsPatch`) → `src/widgets/settings-view/settings-interface-section.tsx` 의 `SwitchField`(선례 82-87행). **총 8~9곳.**

## options
## (i) 커맨드 `view.welcome` — 설계 1안 (변형 축만 다름)

레지스트리 구조상 선택지가 사실상 하나다(선례 `settings.open` 을 그대로 복제). 변경 파일·흐름:

```
⌘⇧P → CommandPaletteCommandsGroup → runCommand(cmd)
     → cmd.run(commandContext) → commandContext.openWelcomeTab()
     → useOpenTab(activeProjectId).mutate({ kind:{kind:'welcome'}, target:null, preview:false })
     → layout_open_tab → open_tab(중복이면 기존 Welcome 활성화) → applyFreshLayout
```

| 파일 | 변경 |
|------|------|
| `src/shared/lib/command-registry.ts` | `CommandContext` 에 `openWelcomeTab: () => void` 추가 |
| `src/shared/lib/command-catalog.ts` | `{ id: 'view.welcome', titleKey: 'app.welcome', categoryKey: KEYMAP_CATEGORY.VIEW, run: (c) => c.openWelcomeTab() }` (`view.git` 바로 뒤) |
| `src/widgets/command-palette/command-palette.tsx` | `openSettingsTab`(160-167) 복제한 `openWelcomeTab` + `commandContext`(197-205) 에 주입 |
| `src/shared/constants/app-file.ts` (또는 신설 `shared/constants/tab.ts`) | `WELCOME_TAB_TITLE = 'Welcome'` — Rust `default_layout()` 과 제목 일치 |
| 로케일 3종 + `locale/service.rs` MESSAGE_NAMESPACES | `app.welcome` 재추가(en `Welcome` / ko `시작 화면` / ja `ようこそ画面`) |
| `command-catalog.test.ts`, `command-registry.test.ts` | `dummyContext` 에 `openWelcomeTab: () => {}` (없으면 타입체크 실패) + runnable 목록에 `'view.welcome'` |
| `docs/features/command-palette.md` §2.1 | 23 → 24종, 창·뷰 행에 `view.welcome` |

**변형 축 3개(각각 사용자/구현 판단 필요):**
- **탭 제목** — (a) 리터럴 `'Welcome'`(Rust 와 일치, 재사용 경로가 title 을 안 고치는 §7 문제 없음) / (b) `t('app.welcome')`(로케일화되나 복원 탭과 제목이 갈림).
- **비활성 조건** — (a) `isEnabled` 없이 실행 시 `app.openProjectFirst` 토스트(= `settings.open` 선례) / (b) `isEnabled: (c) => c.activeProjectId !== null` 로 목록에서 흐리게.
- **기본 단축키** — 없음(권장, `runsViaCommand` 로 사용자 바인딩 가능) / `APP_KEYMAP` 에 신규 `KeymapActionId` 추가.

---

## (ii) 탭 0 자동 Welcome — 설계 3안

### A안 — 빈 pane 렌더를 Welcome 으로 교체 (레이아웃 무변경)

`pane-node-view.tsx:207` 의 `{!activeTab && <div>{t('editor.noFileOpen')}</div>}` 를 메인 창에서만 `WelcomeContainerLazy` 로 바꾼다.

```
layout(탭 0) → EditorArea → PaneNodeView(!activeTab)
   ├ getWindowContext().kind === 'main' → <Suspense><WelcomeContainerLazy projectId={projectId}/></Suspense>
   └ auxiliary → 기존 editor.noFileOpen (창은 곧 스스로 닫힘, §4)
```

- 변경 파일: `src/widgets/editor-area/pane-node-view.tsx` **1개**(+선택적 설정 1종).
- 장점: 레이아웃/영속/revision/닫은-탭-스택/pinned/preview/원격/보조창 **어느 것도 건드리지 않는다**. hot-exit 복원·⇧⌘T 와 구조적으로 충돌 불가. Rust 0줄. 테스트 회귀면 0.
- 장점: 이미 `lazy` 청크가 있어 번들 증가 0, 프로젝트 0개 화면과 자동으로 동일 UI.
- 단점: **탭이 아니다** — 탭 바에 아무것도 안 생기고 ⌘W 로 닫을 수 없다. "Welcome 이 자동으로 보이게" 를 문자 그대로만 충족.
- 단점: 마지막 탭을 닫으면 즉시 Welcome 이 차서 "탭이 안 닫혔나?" 로 읽힐 수 있다.
- 설정 축: 끄기 옵션은 프론트 분기 1줄(`settings.welcomeOnEmptyEditor ?? true`)이면 되고, 끄면 기존 `editor.noFileOpen` 로 되돌아간다.

### B안 — Rust 가 메인 트리가 비면 Welcome 탭을 자동 삽입

`layout/service.rs` 에 `ensure_main_tree_not_empty(layout)` 를 신설해 `close_tab_and_finish`(1303-1320) · `move_tab_to_window`(메인→보조 이동) · `load_layout`(1193-1206) 뒤에서 호출. **`layout.root` 만** 대상(보조창 트리 제외 — §4).

```
layout_close_tab → close_tab → normalize → is_layout_tree_empty(root)?
   → open_tab(root leaf, Tab{kind:Welcome, title:"Welcome"}) → finish_mutation(revision++ + layout:changed)
```

- 변경 파일: `src-tauri/src/domain/layout/service.rs`(+테스트) · `commands.rs`(호출 지점) · 설정 게이트 시 `settings/*`.
- 장점: 진짜 탭이라 탭 바·⌘W·DnD·분할 이동이 전부 그대로 동작. 데스크톱·보조창·원격이 **한 곳에서 원자적으로** 같은 결과를 본다(begin_mutation 가드 안).
- 장점: 복원 시점(`load_layout`)도 같은 함수로 커버 — "탭 0으로 저장된 프로젝트" 가 Welcome 과 함께 열린다.
- 단점: ⇧⌘T 하면 `[Welcome, 복원탭]` 이 된다. 자동 삽입분을 되돌리려면 `Tab` 에 마커 필드가 필요 → **영속 스키마 변경**(`LAYOUT_SCHEMA_VERSION` 승격 검토) — 비용이 급증.
- 단점: 설정으로 끌 수 있게 하려면 `layout` 도메인이 `settings` 를 읽어야 한다(선례는 있다 — `open_tab_and_finish:1274` 가 `state.settings.read().enable_preview_tabs` 를 읽음).
- 단점: Rust 테스트(`service.rs` 의 `close_tab` 계열 다수, 예 2166-2190 "welcome + terminal + kept.rs" 단언)가 **트리가 비는 시나리오마다 기대값이 바뀐다** — 회귀 폭이 가장 크다.

### C안 — 프론트 이펙트가 탭 0을 감지해 Welcome 탭을 연다

`EditorArea`(또는 `AppShell`)에 `isPaneTreeEmpty(paneTree.root)` 이면 `openTab({kind:'welcome'})` 하는 가드된 `useEffect` 를 둔다. 보조창은 자기 창을 닫으므로 `windowContext.kind === 'main'` 게이트 필수.

- 변경 파일: `src/widgets/editor-area/editor-area.tsx` 1개(+설정).
- 장점: 진짜 탭(B안의 UX) 을 Rust 0줄로 얻는다. 설정 읽기가 프론트라 게이팅이 가장 싸다.
- 단점: **데스크톱 메인 창과 모든 원격 세션이 동시에 발화**한다(`window-context.ts:34` — 원격도 main 으로 읽힘, `layout_open_tab` 은 원격 허용). `open_tab` kind 중복 재사용 덕에 결과는 멱등이지만 revision·이벤트가 세션 수만큼 튄다.
- 단점: 실패 시 재시도 루프 위험(열기 실패 → 여전히 빈 트리 → 이펙트 재발화)이라 `useRef` 가드가 필수 — hack 성 코드가 붙는다.
- 단점: 복원 직후·프로젝트 전환 순간의 과도기 레이아웃에서도 발화할 수 있어 타이밍 방어가 필요. `applyFreshLayout` 의 revision 경합과도 얽힌다.

## recommendation
## 추천

**(i) `view.welcome` 커맨드**: 위 1안 그대로, 변형 축은 **탭 제목 = 리터럴 `WELCOME_TAB_TITLE = 'Welcome'`**, **`isEnabled` 없이 `app.openProjectFirst` 토스트**, **기본 단축키 없음** 을 추천.

- 제목을 리터럴로 두는 근거는 §7 — `open_tab` 의 중복 재사용 경로가 기존 탭 `title` 을 갱신하지 않으므로, 로케일 제목을 넘기면 `default_layout()` 이 만든 `"Welcome"` 탭과 새로 만든 탭의 제목이 갈린다. `app.welcome` 로케일 키는 **팔레트 라벨 전용**으로만 재추가한다(§6 의 3곳 동시 반영 필수).
- 토스트 방식은 `settings.open`·`terminal.new`·`app.openSettingsFile` 3선례와 동일해 커맨드 목록의 활성/비활성 규칙이 흔들리지 않는다.
- 단축키 없음이 맞는 이유: `keymapId` 없는 커맨드는 자동으로 `runsViaCommand` 행이 되어 사용자가 키바인딩 에디터에서 직접 바인딩할 수 있다(`keybinding-catalog.ts:38-59`). `APP_KEYMAP` 을 늘리면 기존 24개 바인딩과의 충돌 판정 표면만 넓어진다.

**(ii) 탭 0 자동 표시**: **A안(빈 pane 렌더 교체)** 을 추천.

근거:
1. **최소 변경 대비 효과가 압도적** — 파일 1개·Rust 0줄이고, "탭 0" 을 판정할 새 상태가 필요 없다. §3 에서 확인했듯 `!activeTab` 이 이미 "이 창의 탭 0" 과 정확히 동치라 새 조건식조차 필요 없다.
2. **회귀 표면이 0에 가깝다** — 레이아웃을 안 바꾸므로 hot-exit 영속·`revision` 경합·닫은 탭 스택(⇧⌘T)·pinned·preview·원격 이중 발화·보조창 자동 닫힘(§4, Wave I 계약 §3.2) 어느 것과도 구조적으로 충돌하지 않는다. B/C 안은 각각 스키마 마커 필요(⇧⌘T)·다세션 동시 발화라는 새 문제를 들여온다.
3. **UI 일관성이 공짜** — 프로젝트 0개 화면과 welcome 탭이 이미 같은 `WelcomeContainer` 를 쓰므로(d-27 §1.2), 세 번째 표면도 같은 컴포넌트를 그대로 재사용해 "적용 면 통일" 계약이 유지된다. `lazy` 청크도 이미 있어 번들 증가 없음.
4. B안의 장점("진짜 탭")은 (i) 커맨드가 이미 커버한다 — 탭으로 원하면 ⌘⇧P → Welcome. 자동 표시는 "빈 화면을 유용하게" 가 목적이므로 탭일 필요가 없다.

**메인 창 게이팅은 필수**(옵션 아님): 보조창의 빈 트리는 곧 창을 닫으므로 렌더가 무의미하고, Welcome 의 "최근 프로젝트" 클릭이 전역 활성 프로젝트 세션을 바꾸는데 그건 `app.tsx:26-33` 이 보조창에 금지한 동작이다. `pane-node-view.tsx` 에서 `getWindowContext().kind === 'main'` 1줄로 판정(`command-catalog.ts` 가 같은 방식으로 모듈 스코프 호출하는 선례 있음).

**설정 축**: 켜고 끄는 설정 `welcomeOnEmptyEditor`(기본 true)를 **함께 넣는 것을 추천하되, 사용자 승인 대상**으로 남긴다(질문 3번). A안이면 프론트 분기 1줄로 끝나지만 §10 대로 Rust 설정 파이프라인 8~9곳이 딸려온다 — "끄기"가 실수요가 아니라면 지금은 넣지 않고 A안만 넣는 편이 배치 크기에 맞는다.

**구현 순서 제안**: 로케일 3언어 + MESSAGE_NAMESPACES → (i) 커맨드 5곳 + 테스트 2곳 → (ii) `pane-node-view.tsx` 1곳 → (설정 채택 시) settings 파이프라인 → docs(`command-palette.md` §2.1 24종 · `tabs.md` §2 의 `File(웰컴)` → `TabKind::Welcome` 정정 · 신설 또는 `layout-shell.md` 에 "빈 에디터 영역 = Welcome" 계약 한 줄) → `bun run verify`.

## filesToTouch
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/command-registry.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/command-catalog.ts
- /Users/hyunseokbyun/development/TAIDE/src/widgets/command-palette/command-palette.tsx
- /Users/hyunseokbyun/development/TAIDE/src/shared/constants/app-file.ts
- /Users/hyunseokbyun/development/TAIDE/src/widgets/editor-area/pane-node-view.tsx
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/command-catalog.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/command-registry.test.ts
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/locale/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/en.json
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/ko.json
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/ja.json
- /Users/hyunseokbyun/development/TAIDE/docs/features/command-palette.md
- /Users/hyunseokbyun/development/TAIDE/docs/features/tabs.md
- /Users/hyunseokbyun/development/TAIDE/docs/features/layout-shell.md
- /Users/hyunseokbyun/development/TAIDE/docs/PROCESS.md

## risks
- 보조창 회귀(가장 치명적) — 탭 0 자동 표시를 '탭 삽입'(B/C안)으로 구현하면서 보조창 트리까지 대상에 넣으면 `auxiliary-window-shell.tsx:44-50` 의 `isPaneTreeEmpty` 자동 닫힘이 영구히 무력화되어 Wave I 계약 §3.2 가 깨진다. A안을 택하더라도 메인 창 게이팅을 빠뜨리면 창이 닫히기 직전 1프레임 동안 Welcome 이 번쩍이고, 그 화면의 '최근 프로젝트' 클릭이 `app.tsx:26-33` 이 보조창에 금지한 전역 활성 프로젝트 변경을 일으킨다.
- 탭 제목 분기 — `open_tab`(service.rs:435-442)의 중복 재사용 경로는 기존 탭의 `title` 을 갱신하지 않는다. `view.welcome` 에서 `t('app.welcome')` 를 넘기면 `default_layout()` 이 만든 `"Welcome"` 탭과 팔레트가 만든 탭의 제목이 갈리고, 언어를 바꿔도 기존 탭 제목은 그대로 남는다(모든 탭 제목이 생성 시점에 동결되는 기존 성질).
- 로케일 키 누락 — `app.welcome` 을 en/ko/ja 3파일과 `locale/service.rs` 의 `MESSAGE_NAMESPACES` 중 어느 하나라도 빠뜨리면 `service.rs:1287-1301` 의 두 테스트가 실패한다(카탈로그↔필수키 양방향 검사). 3언어 키 개수 968 동수도 유지해야 한다.
- 테스트 타입 파손 — `CommandContext` 에 `openWelcomeTab` 을 추가하는 순간 `command-catalog.test.ts:7-15` 와 `command-registry.test.ts:14-20` 의 `dummyContext` 가 타입 에러가 난다. 동반 수정 필수(typecheck 로 즉시 검출됨).
- ⇧⌘T 상호작용(B/C안 채택 시) — `reopen_closed`(service.rs:555-575)는 자동 삽입된 Welcome 탭을 인지하지 못해 복원 후 `[Welcome, 복원탭]` 이 남는다. 되돌리려면 `Tab` 에 마커 필드가 필요하고 이는 영속 스키마 변경(LAYOUT_SCHEMA_VERSION)으로 번진다.
- 다세션 동시 발화(C안 채택 시) — 원격 브라우저 세션도 `getWindowContext()` 상 main 이고(`window-context.ts:34`) `layout_open_tab` 은 원격 허용 커맨드(`remote/dispatch.rs:27`)라 데스크톱과 모든 원격 세션이 동시에 자동 열기를 시도한다. kind 중복 재사용 덕에 결과는 멱등이나 revision 증가와 `layout:changed` 이벤트가 세션 수만큼 발생하고, 실패 시 재시도 루프 위험이 있어 ref 가드가 필요하다.
- 성능 — A안은 빈 에디터 영역에서 `WelcomeContainer` 가 `projectListQueryOptions`·`recentProjectsQueryOptions`·`settingsQueryOptions`·`layoutQueryOptions` 4개를 구독한다. 전부 셸이 이미 구독 중이라 순증 요청은 사실상 `project_list_recent` 1건이지만, 원격 세션에서는 이 커맨드가 항상 Forbidden 을 돌려주므로(정책 `LocalProjectHistoryExposure`) 빈 화면마다 실패 IPC 가 1회 발생한다(기존 welcome 탭과 동일한 기지 동작).
- UX 오해 — 마지막 탭을 닫는 즉시 Welcome 이 화면을 채우므로 '탭이 안 닫혔다' 로 읽힐 수 있다. 실기 확인 항목으로 `docs/quality-assurance` 체크리스트에 넣는 편이 안전하다.
- 문서 부정합 확대 — `docs/features/tabs.md:17` 과 `docs/PRD.md:85`(FR-B2)가 기본 탭을 아직 `File(웰컴)` 로 기술한다. 이번 변경을 반영하면서 정정하지 않으면 '기본 레이아웃' 서술이 세 문서에서 각각 달라진다.
- 동시 편집 충돌 — `pane-node-view.tsx`·`command-palette.tsx`·`layout` 도메인·`settings` 파이프라인은 사용성 배치 3 구현 wf 가 지금 편집 중인 파일들과 겹친다(예: `pane-tree.ts` 에 `activeFilePathOf` 가 조사 중 추가됨, `settings.ipc.ts:49` 에 `explorerAutoReveal` 이미 존재). 착수 전 재-diff 필요.

## questionsForUser
- `view.welcome` 로 여는 탭의 제목 — **A) 리터럴 'Welcome'(추천: Rust `default_layout()`(service.rs:114)과 일치, 중복 재사용 시 제목이 갈리지 않음) / B) 로케일 `t('app.welcome')`(ko '시작 화면')로 하되 복원된 기존 탭은 'Welcome' 로 남는 것을 감수 / C) 탭 바 렌더 시점에 welcome kind 만 제목을 로케일로 덮어쓰기(변경 범위가 tab bar 로 확대)**
- 탭 0 자동 표시 방식 — **A) 빈 에디터 영역 렌더를 Welcome 으로 교체(추천: 파일 1개·Rust 0줄, 복원·⇧⌘T·보조창과 구조적 무충돌. 단 탭 바에는 아무것도 안 생김) / B) Rust 가 메인 트리가 비면 진짜 Welcome 탭을 자동 삽입(탭 UX 완전하나 ⇧⌘T 후 Welcome 이 남고 Rust 테스트 회귀 폭 최대) / C) 프론트 이펙트로 자동 탭 열기(원격 세션 동시 발화·재시도 가드 필요)**
- 끄기 설정(`welcomeOnEmptyEditor`, 기본 켜짐)을 이번 배치에 포함할지 — **A) 포함하지 않음(추천: A안이면 설정 없이 1파일 변경으로 끝나는데 설정 1종은 Rust·로케일·UI 8~9곳을 늘린다. 실수요 확인 후 후속) / B) 포함(설정 > 인터페이스 섹션에 SwitchField 추가)**
- `view.welcome` 을 활성 프로젝트가 없을 때 — **A) 목록에 그대로 두고 실행 시 'Open a project first' 토스트(추천: settings.open·terminal.new 선례와 동일) / B) `isEnabled` 로 비활성 처리**
- 기본 단축키 — **A) 부여하지 않음(추천: `runsViaCommand` 행으로 키바인딩 에디터에서 사용자가 직접 바인딩 가능, APP_KEYMAP 충돌 표면 불변) / B) `APP_KEYMAP` 에 신규 액션 id 추가해 기본 키 부여(어떤 키인지 지정 필요)**
- Welcome 전용 기능 문서를 신설할지 — **A) 신설하지 않고 `docs/features/layout-shell.md` 에 '빈 에디터 영역 = Welcome' 절을 추가(추천: 현재 정본이 acknowledge d-27 계약 + layout-shell 이라 분산 최소) / B) `docs/features/welcome.md` 신설해 3표면(프로젝트 0개·welcome 탭·빈 에디터 영역)을 한곳에 정본화**

## newDependencies



# 주제 4 — 프로젝트 목록 아이콘·라벨

## findings
## 1. 현재 사이드바 렌더 경로 (실코드)

- `src/widgets/app-sidebar/app-sidebar.tsx:56` — 레일은 `w-14`(56px), 항목 간 `gap-1`. `useQuery(projectListQueryOptions())` 로 **`ProjectRef[]` 만** 받는다(`:29`). 에이전트는 이미 `useQueries` 로 프로젝트당 1쿼리를 돌린다(`:35-38`) — 프로젝트별 N쿼리 패턴의 선례가 이미 이 파일에 있다.
- `src/widgets/app-sidebar/sortable-project-icon.tsx:51` — 아이콘이 **`icon={<Folder className='size-5' />}` 로 하드코딩**되어 있다. 여기가 요청의 정확한 지점.
- 같은 파일 `:74-81` — 컨텍스트 메뉴가 이미 있다(`project.close` / 구분선 / `project.openInFileManager` / `project.copyPath`). `shared/ui/context-menu.tsx` 는 `ContextMenuSub`/`SubTrigger`/`SubContent`/`RadioGroup`/`RadioItem`/`CheckboxItem` 까지 전부 export 한다(`:169-185`) — 서브메뉴·라디오 선택 인프라가 이미 완비.
- `src/features/project/project-icon-button.tsx:29` — 버튼은 `size-10`(40px) + `text-xs`(12px) + `font-medium`. `overflow-hidden` 은 **없다**.
- 같은 파일 `:16`·`:34` — **`initialsOf` 는 사실상 dead code**다. `icon: ReactNode` 는 optional 이 아니고 유일 호출부가 항상 `<Folder/>` 를 넘기므로 `{icon ?? initialsOf(name)}` 의 우변은 절대 실행되지 않는다. 즉 "이름 이니셜 표시"는 코드에 흔적만 남고 동작하지 않는다.
- `:25` — `aria-label` 은 **프로젝트 이름**(+에이전트 상태)이다. 짧은 라벨을 도입해도 이 값은 이름으로 유지해야 접근성이 유지된다.
- 툴팁은 이미 이름+루트+에이전트 세션을 `side='right'` 로 보여준다(`sortable-project-icon.tsx:59-69`, 규약 정본 `docs/memory/tooltip-conventions.md:11`) — "넘치면 잘라서 툴팁" 요구의 툴팁 절반은 **이미 충족**되어 있다.

## 2. 우클릭과 DND 의 충돌 없음 (확인 완료)

`@dnd-kit/core/dist/core.esm.js:1642` — `if (!event.isPrimary || event.button !== 0) return`. PointerSensor 는 우클릭으로 드래그를 시작하지 않는다. 컨텍스트 메뉴 확장은 DND 와 충돌하지 않는다.

다만 기존 결함 1건: `useSortable` 의 `attributes` 는 `{role:'button', tabIndex:0, ...}` 를 반환하고(`core.esm.js:3432-3439`) `sortable-project-icon.tsx:44` 가 이를 **실제 `<button>` 을 감싼 `div`** 에 펼친다 → 중첩 button 시맨틱·이중 tabstop. 이번 범위 밖이지만 같은 파일을 건드리므로 인지 대상.

## 3. 영속 스키마 — 어디에 저장 가능한가

- `src-tauri/src/domain/project/types.rs:26-43` — `Project { id, root, name, capabilities, root_missing, last_opened_at }`. `projects/<id>/project.json` 에 저장(`service.rs:218-220`).
- `types.rs:18-24` — `ProjectRef { id, root, name }`. `session.json` 에 저장(`service.rs:197-199`). `service::upsert_project_ref`(`service.rs:246-258`)가 `root`/`name` 을 Project 로부터 **이미 미러링**하고 있다 — 중복 필드의 동기화 지점이 하나 존재한다.
- **migration 0 선례가 정확히 있다**: `last_opened_at` 은 `#[serde(default)]` 로 무마이그레이션 흡수됐다(`types.rs:36-42` 주석, `docs/data-model.md:684-702` §18). 규칙 정본은 `docs/data-model.md:145-147` "필드 추가는 `#[serde(default)]` 로 무마이그레이션 흡수, 구조 변경만 버전을 올린다".
- **settings.json 은 부적합**: `ProjectId` 는 머신 로컬 uuid v4(`src-tauri/src/ids.rs:11-13`, `:40`)인데 settings 는 gist 로 업로드된다(`sync/service.rs settings_to_sync_patch`). 머신 간 무의미한 id 키가 동기화된다. 또 `SettingsPatch`/`apply_patch`/`settings_to_sync_patch`/`emptySettingsPatch` 4곳 수동 미러 문제(감사 R5#3, `docs/quality-assurance/2026-08-18-architecture-audit-raw-R4-X1.md`)를 키운다.
- **layout.json 도 부적합**: revision 낙관적 동시성·창별 축을 가진 구조라 표시 메타데이터와 성격이 다르다.

## 4. `#[serde(default)]` → TS optional 드리프트 (알려진 함정)

`src/shared/api/bindings.ts:1481-1486` — `capabilities?:`·`rootMissing?:` 처럼 **specta 가 `#[serde(default)]` 필드를 optional 로 생성**한다. 감사 R5#5 가 지적한 "소비처마다 `??` 폴백을 손으로 다시 적어 값이 어긋난다"(실제로 `agentStatusBadgeEnabled` 가 `?? false` vs `?? true` 로 갈렸다 — `app-sidebar.tsx:39`)의 원인이다. 표시 설정도 같은 함정에 들어가므로 **폴백 지점을 `shared/lib` 헬퍼 1곳으로 강제**해야 한다.

## 5. 아이콘 카탈로그 — 이미 있는 정답 패턴

- `src/shared/icons/file-icon-registry.ts:34` — `FILE_ICON_COMPONENT_MAP: Record<FileIconName, ComponentType<LucideProps>>` 로 **29종을 정적 import 해 큐레이션**한다. 이름 union 은 `src/shared/lib/file-icon.ts:3-31`, 렌더는 `src/shared/icons/file-type-icon.tsx`. **원하는 것과 정확히 같은 구조가 이미 존재**한다.
- 앱 전체 lucide 정적 import: 84개 import 문 / 고유 아이콘 식별자 142종.
- `lucide-react` 1.28.0 은 `lucide-react/dynamic` 으로 `DynamicIcon`·`iconNames`·`dynamicIconImports` 를 노출한다(`node_modules/lucide-react/dynamic.mjs:8-9`). 구현은 `dist/esm/DynamicIcon.mjs:14-40` — `Object.keys(dynamicIconImports)` + `useEffect` 안에서 `await dynamicIconImports[name]()`.
- **전체 노출 비용(실측)**: `dist/esm/dynamicIconImports.mjs` 에 `import()` 항목 **2007개**(파일 118,672 bytes), 아이콘 모듈 **2007개 / 합계 1,409,867 bytes**. 이를 참조하면 Vite/Rolldown 이 아이콘당 별도 청크를 뱉어 `dist/assets` 에 수천 파일이 생긴다. `iconNames` 만 import 해도 맵 전체가 딸려온다(`Object.keys` 대상이므로 tree-shake 불가).
- 큐레이션 시 비용: 아이콘 1종 실질 소스 약 450 bytes(예 `icons/rocket.mjs` 850B 중 라이선스 헤더 약 250B, `icons/folder.mjs` 571B). **56종 ≈ 25KB raw / 약 12KB minified**, 그중 상당수는 이미 앱이 import 중이라 순증은 더 작다.
- CSP `script-src 'self'`(`src-tauri/tauri.conf.json:26`)는 동적 청크(동일 오리진)를 막지는 않는다 — 즉 기술적 차단 사유가 아니라 **번들 위생 사유**로만 dynamic 을 기각한다.

## 6. 색 토큰 체계

- `docs/theme-system.md:83` — `appSidebar.*` 는 background/itemHover/itemActive/iconDefault/iconAgent{Running,Working,Awaiting,Idle,Unknown}/badge **10종 고정**. 프로젝트 색용 토큰은 없다.
- 토큰을 늘리면 비용이 크다: `src/entities/theme/theme-tokens.ts:3-107`(TS)와 `src-tauri/src/domain/theme/service.rs:77-216`(Rust) **두 곳 수동 미러**(감사가 "교차 검증 없음"으로 지적), `required_color_keys()`(`service.rs:283-288`)를 내장 dark/light 테마가 전부 채워야 하는 테스트(`service.rs:1090-1112`), 번들 테마 36종의 base 채움 경고까지 따라온다.
- **재사용 가능한 12색 팔레트가 이미 있다**: `graph.lane1..lane12`(`theme-tokens.ts:79-100`, `src/shared/styles/global.css:77-88`). 소비처는 커밋 그래프 1곳뿐이고 접근 방식도 인라인 `var(--taide-graph-laneN)` 선례가 있다(`src/widgets/git-panel/commit-graph.tsx:49`). 모든 테마가 이미 이 12색을 정의한다.

## 7. 폭 제약 실측

기본 Tailwind 스케일(전역 `@theme` 에 spacing 재정의 없음 — `global.css:153` 의 `@theme inline` 은 색 매핑만): 레일 `w-14`=56px, 버튼 `size-10`=40px, `text-xs`=12px.

- 라틴 4자(12px, medium) ≈ 26~30px → 40px 안에 들어간다.
- **CJK 4자 ≈ 48px > 40px** → 그대로 두면 넘친다. 현재 버튼에 `overflow-hidden` 이 없어 실제로 삐져나온다. 글자수 기준이 아니라 **표시 폭 기준(글자수 사다리로 폰트 축소 + `overflow-hidden`)** 이 필요하다.

## 8. 신규 커맨드의 고정 비용 (테스트가 강제)

- `src-tauri/src/lib.rs:395`·`:707` — 커맨드는 `collect_commands![]`(`lib.rs:~250-356`)에 등록하고 bindings 는 `BINDINGS_PATH`(`lib.rs:37`)로 자동 생성된다. `lib.rs:719-736` 테스트가 **그 순간 생성되는 바인딩과 dispatch 테이블의 일치**를 강제한다(커밋된 bindings.ts 가 낡아도 잡힌다).
- `src-tauri/src/domain/remote/dispatch.rs:16` `IMPLEMENTED_JSON_COMMANDS` + `:829-835` match arm + `:560` `REMOTE_ALLOWED_COMMANDS`(또는 DENIED). `:1736` `허용_테이블과_거부_테이블은_전체_커맨드를_교집합_없이_정확히_분할한다` 가 **분류 누락을 컴파일 후 테스트로 잡는다**.
- 원격 미러는 같은 SPA 를 서빙한다(`domain/remote/server.rs:28` `fallback(serving::serve_static)`) → 사이드바가 원격에서도 그대로 렌더된다. 이벤트는 `lib.rs:530-551` `fanout_remote_events!` 에 등록된 것만 원격으로 나간다(`ProjectListChanged` 는 이미 등록됨 — `lib.rs:534`).
- i18n: 키는 `src-tauri/resources/locales/{en,ko,ja}.json` 3벌(`project.close` 등은 `ko.json:465-467`) + `domain/locale/service.rs:675` `("project", &["close","openInFileManager","copyPath"])` 네임스페이스 등록. 현재 기준선 792키×3(`docs/PROCESS.md:48-50`).

## 9. 보조창·Welcome 노출 범위

- **보조창은 사이드바를 렌더하지 않는다** — `src/app/app.tsx:54-76` 은 `AuxiliaryWindowShell` 로 분기하고 `AppSidebar` 는 `app-shell.tsx:186` 에만 있다. "보조창에서 동일 표시"는 사이드바 기준으로는 무의미하고, 굳이 확장한다면 `auxiliary-title-bar-content.tsx:22`(이미 `projectQueryOptions` 사용)가 대상이다.
- Welcome 최근 목록은 `Project[]` 를 그대로 받아 이름·루트만 그린다(`src/features/welcome/welcome-screen.tsx:70-90`) — 표시 설정을 `Project` 에 넣으면 **추가 IPC 없이** 여기까지 확장 가능하다.

## 10. 기존 결정·백로그와의 관계 (중복 없음)

- `docs/backlog.md` 전체에 프로젝트 아이콘/라벨 항목 **없음**.
- `docs/acknowledge/2026-09-04-usability-batch4-user-decisions.md` "질문 없이 확정한 전제" — **이미 사용자 고지·무이의로 확정**: "우클릭 메뉴에서 lucide 아이콘 선택 + 1~4자 텍스트 라벨(넘치면 잘라서 툴팁) + 색 토큰. 프로젝트별 저장. 이모지는 컨벤션상 제외." → **무엇을** 만들지는 이미 합의됐고, 남은 건 **어디에 저장·어떤 UI·FR-A4 와의 관계**다.
- `docs/PROCESS.md:53-64` — 이 조사는 그 배치의 "a. 조사 wf ④ 프로젝트 목록 아이콘/라벨 고도화" 항목이고, 미결 항목은 "b. 사용자 결정 회신 — … 프로젝트 목록 표시 방식".

## 11. ⚠ FR-A4 충돌 (가장 중요한 발견)

- `docs/PRD.md:78-79` **FR-A4**: "앱 사이드바의 프로젝트 항목 아이콘은 그 프로젝트에서 현재 **focus 된 content 타입**(파일뷰·터미널·설정·code agent 등)에 따라 바뀐다."
- `docs/features/layout-shell.md:31-40` 이 타입별 아이콘 표를 정의하고, `:80` 은 "FR-A4 아이콘은 이 캐시 기반 유도로 구현하는 것이 **정본**"이라고 못박는다.
- **그런데 실코드에는 구현이 없다**: `focusKind`/`FocusKind` grep 결과 src·src-tauri 전체 0건. `project:focus-kind-changed` 이벤트와 `FocusKind` 타입은 X-A 배치에서 제거됐다(`docs/ipc-contract.md:105-110`). 즉 현재 아이콘은 영구 `<Folder/>` 이고, §2.2 중 구현된 것은 에이전트 오버레이 배지뿐이다.
- 사용자 지정 아이콘/라벨을 넣으면 이 미구현 스펙과 **같은 슬롯을 다툰다**. 우선순위를 정하지 않으면 PRD 와 코드가 또 어긋난 채로 굳는다.


## options
## 저장 위치 (A / A' / B)

### 안 A — `Project`(project.json)에만 저장 + 사이드바가 프로젝트별 `project_get` N쿼리

- 변경: `types.rs` 에 `ProjectDisplay` 신설 + `Project.display: ProjectDisplay` `#[serde(default)]`. `service::set_project_display` + `commands::project_set_display`. 사이드바는 `useQueries(projects.map(p => projectQueryOptions(p.id)))` 추가.
- 데이터 흐름: `project.json` → `state.projects`(메모리 맵) → `project_get` → `PROJECT.DETAIL(id)` → 사이드바. 갱신 브로드캐스트를 위해 **신규 이벤트 `ProjectDisplayChanged` 필요**(events.rs + lib.rs `collect_events!` + `fanout_remote_events!` + `ipc-sync-provider`).
- 장점: 단일 정본(중복 0). `project_list_recent` 도 자동으로 표시 설정을 얻어 Welcome 확장이 공짜.
- 단점: 사이드바에 N쿼리 1축 추가(에이전트에 이어 두 번째). 신규 이벤트 1종(등록 4곳). `project.json` 파손 시 `restore_session`(`service.rs:230-237`)이 ProjectRef 로 합성하며 **표시 설정이 소실**된다.

### 안 A' — `Project` + `ProjectRef` 양쪽에 저장 (기존 name/root 미러 규칙에 편승) 【추천】

- 변경: `ProjectDisplay` 를 `Project` 와 `ProjectRef` **둘 다**에 `#[serde(default)]` 로 추가. `upsert_project_ref`(`service.rs:246-258`)가 root/name 과 함께 display 를 미러. `set_project_display` 는 `save_project` + `save_session` 후 **기존 `ProjectListChanged` 만 emit**.
- 데이터 흐름: `project_set_display` → project.json·session.json 동시 기록 → `ProjectListChanged{projects: ProjectRef[]}`(**payload 자체가 새 상태**) → 기존 핸들러 `ipc-sync-provider.tsx:187-189` 가 `PROJECT.LIST` 무효화 → 사이드바 재조회. 원격·다중 창은 `lib.rs:534` 의 기존 fanout 으로 이미 배선됨.
- 장점: **신규 이벤트 0 · 신규 쿼리 0 · FE 배선 0**. 두 파일 모두 `#[serde(default)]` 라 **마이그레이션 0**(d-27 선례 그대로). `project.json` 파손 복구 경로에서도 표시 설정이 살아남는다.
- 단점: 중복 필드 1개 증가(단, root/name 이 이미 중복이고 동기화 지점은 `upsert_project_ref` 한 곳). 다른 창의 `PROJECT.DETAIL`/`RECENT` 는 즉시 갱신되지 않음 — 단 현재 그 두 쿼리의 소비처 중 display 를 그리는 곳이 0이라 무해하고, Welcome 확장 시 그때 이벤트를 추가하면 된다.

### 안 B — `settings.json` 의 `projectDisplays: Record<ProjectId, ...>`

- 장점: Rust 변경이 settings 도메인 1곳으로 모임, 이미 `SettingsChanged` 브로드캐스트 존재.
- 단점(치명): `ProjectId` 가 머신 로컬 uuid(`ids.rs:11-13`)인데 settings 는 gist 로 동기화된다 → 타 머신에 죽은 키가 쌓인다. `SettingsPatch`·`apply_patch`·`settings_to_sync_patch`·`emptySettingsPatch` 4중 수동 미러(감사 R5#3)를 키운다. 프로젝트를 닫아도 항목이 남아 GC 규칙이 별도로 필요. **기각 권고.**

## UI (U1 / U2 / U3)

### U1 — 컨텍스트 메뉴 인라인 서브메뉴만

`ContextMenuSub` 3개(아이콘 그리드 / 라벨 / 색 스와치) + "기본값으로". 다이얼로그 0.
- 장점: 클릭 최소, 신규 컴포넌트 최소.
- 단점: 아이콘 50+ 종을 검색 없이 서브메뉴에 넣으면 스크롤 지옥. 라벨은 결국 입력 UI 가 필요해 메뉴 안 input(포커스 관리 까다로움) 또는 다이얼로그로 샌다.

### U2 — 컨텍스트 메뉴 1항목 → 전용 다이얼로그 【추천】

메뉴에 `프로젝트 표시…`(+ 커스터마이즈된 경우에만 활성인 `기본값으로 되돌리기`) 1~2개만 추가. 다이얼로그(`features/project/project-display-dialog.tsx`)는 표시 모드(아이콘/라벨/기본) 라디오 + `shared/ui/command` 검색이 붙은 아이콘 그리드 + `maxLength` 라벨 입력 + 12색 스와치 + 실시간 미리보기.
- 장점: 아이콘 검색 가능, 미리보기로 폭 잘림을 즉시 확인, 컨텍스트 메뉴가 비대해지지 않음. `create-tag-dialog.tsx`(닫힘→열림 전이에서 렌더 중 state 리셋, `:29-35`)라는 정확한 선례가 있다.
- 단점: 색만 바꾸는 데도 다이얼로그 왕복 1회.

### U3 — 하이브리드

메뉴에 색 스와치 서브메뉴 + 자주 쓰는 아이콘 12종 퀵픽 서브메뉴 + `더 보기…`(다이얼로그) + `라벨 설정…`(다이얼로그).
- 장점: 흔한 조작은 1홉.
- 단점: 메뉴 항목 4개 증가 + 다이얼로그도 필요 → U1·U2 비용의 합. 표면이 둘로 갈려 유지보수 2배.

## 아이콘 카탈로그 (C1 / C2)

- **C1 큐레이션 정적 레지스트리 【추천】**: `shared/icons/project-icon-registry.ts` 에 `Record<ProjectIconName, ComponentType<LucideProps>>` 로 약 56종. `file-icon-registry.ts:34` 와 **완전히 동일한 패턴**. 비용 약 25KB raw(약 12KB minified), 청크 0개 증가, 타입 exhaustive 체크 유지.
- **C2 `lucide-react/dynamic` 전량(2007종)**: 검색 UX 는 최고. 대신 `dynamicIconImports.mjs` 의 `import()` 2007개가 그대로 청크가 되고(합계 1.41MB raw), `iconNames` 는 tree-shake 불가. Tauri 로컬 번들이라 네트워크 비용은 0이지만 `dist/assets` 파일 수가 수천 개가 된다. **기각 권고**(필요해지면 C1 을 늘리는 편이 항상 싸다).


## recommendation
## 추천: 저장 A' + UI U2 + 카탈로그 C1

### 데이터

```rust
// domain/project/types.rs
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDisplay {
    #[serde(default)] pub icon: Option<String>,   // FE 레지스트리의 kebab-case id (Rust 는 불투명 문자열)
    #[serde(default)] pub label: Option<String>,  // 1~4 코드포인트, 제어문자 제거·trim
    #[serde(default)] pub color: Option<String>,  // 'lane1'..'lane12'
}
```
`Project` 와 `ProjectRef` 양쪽에 `#[serde(default)] pub display: ProjectDisplay` 추가 → `docs/data-model.md:145-147` 규칙 그대로 **마이그레이션 0**, d-27 `last_opened_at` 선례와 동일 형태.

**아이콘 이름 카탈로그는 TS 단독 정본으로 둔다.** Rust 는 길이·문자셋(`[a-z0-9-]{1,64}`)만 sanitize 하고 의미는 모른다. 이유 둘: (1) `FileIconName` 과 동일한 기존 구조라 새 관례를 만들지 않는다, (2) Rust enum 으로 만들면 카탈로그에서 아이콘 하나만 빼도 기존 `project.json` 이 파싱 실패 → `load_project` 가 `.bak` 로 격리(`service.rs:209-215`)하고 capabilities·lastOpenedAt 까지 통째로 날아간다. 미지의 이름은 FE 에서 조용히 `folder` 로 폴백한다. Rust↔TS 목록 이중화(감사가 테마 토큰 200종에서 지적한 드리프트)를 애초에 만들지 않는 선택이다.

**해제(clear) 규약은 기존 것을 그대로 쓴다** — `settings/service.rs:301-310` 의 "빈 문자열 = 해제 / 미지정 = 유지" 규약(`merge_clearable_*`). 3상태를 새 타입으로 발명하지 않는다.

**`??` 폴백은 1곳으로 강제한다.** specta 가 `display?:` 로 내보내므로(`bindings.ts:1485` 선례) `shared/lib/project-display.ts` 의 `resolveProjectDisplay(ref)` 단일 함수만 폴백을 갖고, 컴포넌트는 그 결과만 소비한다 — 감사 R5#5 가 지적한 폴백 산개(`?? false` vs `?? true`)를 원천 차단.

### IPC

`project_set_display(projectId, patch)` 1개. `begin_mutation` 가드 → `save_project` + `save_session` → **기존 `ProjectListChanged` emit**. 신규 이벤트 0. `dispatch.rs` 는 `IMPLEMENTED_JSON_COMMANDS` + match arm + `REMOTE_ALLOWED_COMMANDS` 3곳(분할 테스트 `dispatch.rs:1736` 이 누락을 잡는다). 원격 허용을 권고한다 — 이미 원격에서 보이는 프로젝트의 표시 메타데이터일 뿐이고, `project_reorder` 가 허용인 것과 같은 등급이다.

### 렌더

`ProjectIconButton` 을 `display` 기반으로 바꾸고 dead 인 `initialsOf`(`:16`)를 실제 폴백 사다리로 승격한다:

1. `label` 있으면 라벨 텍스트
2. `icon` 있으면 레지스트리 아이콘(미지 이름은 `folder`)
3. 없으면 현행 `<Folder/>` — **기본 동작 무변경**

- 버튼에 `overflow-hidden` 추가 + 길이별 타이포 사다리 상수(`PROJECT_LABEL_CLASS_BY_LENGTH`, 매직넘버 금지 규칙에 맞춰 constant 로) → CJK 4자(≈48px)도 40px 안에 들어간다.
- 색은 `graph.lane1..12` 재사용 — `commit-graph.tsx:49` 의 `var(--taide-graph-laneN)` 선례를 따르고 **테마 토큰 신설 0**(TS/Rust 이중 목록·36개 번들 테마·required_color_keys 테스트를 전부 건드리지 않는다). 적용 범위는 글리프/텍스트 색만(활성 배경은 기존 `appSidebar.itemActive` 유지) — 대비 파괴 위험이 가장 낮다.
- `aria-label` 은 **프로젝트 이름 그대로 유지**(`project-icon-button.tsx:25` 불변). 짧은 라벨은 시각 표현일 뿐이므로 스크린리더에 새어나가면 안 된다. 툴팁도 이미 이름+루트를 보여주므로(`sortable-project-icon.tsx:59-69`) "잘리면 툴팁" 요구는 추가 작업 없이 충족된다.

### UI

컨텍스트 메뉴에 `프로젝트 표시…` 1항목(+ 커스터마이즈 상태에서만 활성인 `기본값으로`) 추가, 나머지는 다이얼로그. 다이얼로그는 `create-tag-dialog.tsx:29-35` 의 열림 전이 리셋 패턴을 그대로 복제한다.

### 이 조합을 고르는 이유 한 줄

**신규 의존성 0·신규 이벤트 0·신규 쿼리 0·테마 토큰 0·마이그레이션 0** 으로 요구를 전부 충족하면서, 코드베이스에 이미 있는 네 가지 선례(`file-icon-registry` 큐레이션 / `last_opened_at` serde default / `upsert_project_ref` 미러 / clearable 빈문자열 규약)만 재사용한다. `docs/PROCESS.md:48-50` 의 "신규 의존성 0 유지" 기준선을 지킨다.


## filesToTouch
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/project/types.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/project/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/project/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/lib.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/remote/dispatch.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/locale/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/en.json
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/ko.json
- /Users/hyunseokbyun/development/TAIDE/src-tauri/resources/locales/ja.json
- /Users/hyunseokbyun/development/TAIDE/src/shared/api/bindings.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/project-display.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/project-display.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/icons/project-icon-registry.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/project/project.ipc.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/project/project.query.ts
- /Users/hyunseokbyun/development/TAIDE/src/features/project/project-icon-button.tsx
- /Users/hyunseokbyun/development/TAIDE/src/features/project/project-display-dialog.tsx
- /Users/hyunseokbyun/development/TAIDE/src/widgets/app-sidebar/sortable-project-icon.tsx
- /Users/hyunseokbyun/development/TAIDE/docs/features/layout-shell.md
- /Users/hyunseokbyun/development/TAIDE/docs/data-model.md
- /Users/hyunseokbyun/development/TAIDE/docs/ipc-contract.md
- /Users/hyunseokbyun/development/TAIDE/docs/PRD.md
- /Users/hyunseokbyun/development/TAIDE/docs/acknowledge/2026-09-04-usability-batch4-contract.md
- /Users/hyunseokbyun/development/TAIDE/docs/PROCESS.md

## risks
- FR-A4 스펙 충돌 — PRD.md:78-79 와 layout-shell.md:31-40 은 '아이콘이 focus 된 content 타입에 따라 바뀐다'를 정본으로 못박고 있으나 실구현은 0건(focusKind grep 0). 사용자 지정 아이콘이 같은 슬롯을 점유하면 미구현 스펙이 사실상 폐기되는데, PRD 를 갱신하지 않으면 코드-문서 괴리가 또 한 겹 굳는다. 우선순위 결정을 계약에 명시하고 PRD.md·layout-shell.md 를 동시 개정해야 한다.
- ProjectRef 미러 드리프트(안 A' 고유) — root/name 이 upsert_project_ref(service.rs:246-258) 한 곳에서만 동기화되듯 display 도 그 한 곳에 의존한다. 향후 Project.display 를 다른 경로에서 쓰는 코드가 생기면 session.json 과 project.json 이 갈린다. set_project_display 가 반드시 두 파일을 함께 쓰도록 테스트로 고정(양쪽 파일 재로드 후 동일값 assert)해야 한다.
- project.json 파싱 실패 확대 위험 — 아이콘 이름을 Rust enum 으로 만들면 카탈로그에서 아이콘 1종을 제거하는 순간 기존 레코드가 파싱 실패하고 load_project(service.rs:209-215)가 .bak 로 격리하면서 capabilities·lastOpenedAt 까지 통째로 손실된다. 추천안(불투명 문자열 + FE 폴백)을 벗어나면 이 위험이 실현된다.
- specta optional 필드 폴백 산개 — bindings.ts 가 display?: 로 생성되므로(capabilities?/rootMissing? 선례) 소비처마다 ?? 를 적으면 감사 R5#5(agentStatusBadgeEnabled 가 app-sidebar.tsx:39 ?? true vs settings-view ?? false 로 갈린 사건)가 재현된다. resolveProjectDisplay 단일 폴백을 강제하지 않으면 회귀한다.
- 라벨 오버플로 — project-icon-button.tsx:29 의 size-10(40px) 버튼에 overflow-hidden 이 없다. text-xs(12px) 기준 CJK 4자는 약 48px 로 레일(w-14=56px) 안에서 인접 항목·배지와 겹친다. 글자수만 제한하고 표시 폭을 제한하지 않으면 요구('overflow 없이')를 만족하지 못한다.
- 원격 dispatch 분류 누락 — 신규 커맨드를 IMPLEMENTED_JSON_COMMANDS·match arm·ALLOWED/DENIED 중 하나라도 빠뜨리면 dispatch.rs:1736 분할 테스트와 lib.rs:719-736 바인딩 대조 테스트가 실패한다(빌드는 통과). rust:test 를 반드시 돌려야 한다.
- graph.lane* 재사용의 의미 오염 — 이 토큰은 원래 git 그래프 레인용(commit-graph.tsx:49 단독 소비)이다. 프로젝트 색으로 겸용하면 향후 테마 저작자가 레인색을 그래프 가독성 기준으로만 조정할 때 사이드바 대비가 함께 흔들린다. 일부 테마의 lane 색이 appSidebar.background 대비 낮을 수 있어 실측 확인이 필요하다.
- dead code initialsOf 정리 — project-icon-button.tsx:16 의 initialsOf 는 icon 이 항상 non-null 이라 도달 불가다. 이번에 폴백 사다리로 승격하지 않고 방치하면 새 분기와 나란히 죽은 분기가 남는다(ai-process §6.8).
- dnd-kit 중첩 인터랙티브 — sortable-project-icon.tsx:44 가 useSortable attributes(role='button', tabIndex=0 — core.esm.js:3432-3439)를 실제 <button> 을 감싼 div 에 펼쳐 이중 tabstop·중첩 button 시맨틱이 이미 존재한다. 같은 파일에 컨텍스트 메뉴 항목과 다이얼로그 트리거를 얹을 때 포커스 순서가 더 악화될 수 있다.
- 동시 편집 충돌 — 다른 워크플로가 src/widgets/explorer·command-palette·terminal-pane, src/features/terminal, src/entities/layout|file|system, src-tauri/src/domain/layout|file|search|settings|window, lib.rs 를 편집 중이다. 겹치는 파일은 src-tauri/src/lib.rs(커맨드 등록) 하나뿐이라 충돌 위험은 낮지만, 그 한 줄 추가는 반드시 최신 상태에서 해야 한다.
- i18n 3벌 누락 — 신규 UI 문자열은 resources/locales/{en,ko,ja}.json 3벌 + locale/service.rs:675 의 project 네임스페이스 등록을 모두 갱신해야 한다. 한쪽만 빠지면 required-key 테스트가 실패한다(기준선 792키×3).
- lucide 전량 동적 노출을 택할 경우의 번들 오염 — dynamicIconImports.mjs 의 import() 2007개가 그대로 청크가 되어 dist/assets 파일 수가 수천 개로 늘고 iconNames 는 tree-shake 되지 않는다(실측 1,409,867 bytes / 2007 모듈). 추천안(C1)을 벗어나면 발생한다.

## questionsForUser
- 표시 모드 — 아이콘과 라벨을 배타(둘 중 하나만)로 할까요, 아니면 동시 표시(아이콘 + 하단 미니 라벨)까지 허용할까요? 요청 원문은 '아이콘을 바꾸거나 … 텍스트 라벨'이라 배타로 읽히고, 40px 버튼 폭상으로도 배타가 안전합니다. 추천: 배타(아이콘 / 라벨 / 기본 3택).
- FR-A4 우선순위 — PRD.md:78 은 '아이콘이 focus 된 content 타입에 따라 바뀐다'를 요구하지만 구현은 0건입니다. (A) 사용자 지정이 있으면 항상 이기고, 없으면 현행 폴더 아이콘 유지 + FR-A4 는 '보류' 로 PRD 각주 (추천) / (B) 사용자 지정이 있어도 focus 아이콘이 이김(지정은 라벨·색만) / (C) 이번 기회에 FR-A4 를 함께 구현(LAYOUT.DETAIL 캐시 기반 유도 — 별도 범위 증분).
- 색 팔레트 출처 — (A) 기존 graph.lane1..12 재사용: 테마 토큰 신설 0, 36개 번들 테마 무수정 (추천) / (B) appSidebar.projectColor1..8 신설: 의미는 정확하지만 Rust·TS 토큰 목록 이중 수정 + 내장 테마 2종 필수 채움 + 번들 테마 36종 base 채움 경고 동반.
- 색 적용 범위 — (A) 글리프/텍스트 색만 (추천, 대비 위험 최소) / (B) 활성 인디케이터 바(현행 bg-app-accent)까지 / (C) 버튼 배경 틴트까지.
- 표시 설정 노출 범위 — 앱 사이드바에만 적용할까요, 아니면 Welcome 최근 프로젝트 목록(welcome-screen.tsx:70-90, 이미 Project[] 를 받으므로 추가 IPC 0)과 보조창 타이틀바(auxiliary-title-bar-content.tsx:22)까지 확장할까요? 확장하면 다른 창 즉시 동기화를 위해 신규 이벤트 ProjectDisplayChanged 1종이 필요합니다. 추천: 1차는 사이드바만, Welcome 은 후속.
- 원격 세션에서 표시 설정 변경 허용 여부 — project_reorder 와 같은 등급으로 REMOTE_ALLOWED_COMMANDS 에 넣을까요(추천), 아니면 로컬 전용으로 DENIED 에 넣을까요? 표시만 바꾸는 메타데이터라 민감도는 낮습니다.
- 아이콘 후보 개수 — 큐레이션 약 56종(카테고리별: 코드·웹·모바일·DB·인프라·디자인·문서·테스트·AI·게임·금융·기타)을 정적 레지스트리로 제공(추천, 약 12KB minified·청크 0) / 더 많은 수(100~150종)를 원하시면 비용은 선형으로만 늘어납니다 / 전체 2007종 동적 로드는 dist 청크 수천 개 문제로 비추천입니다.

## newDependencies



# 주제 3a — 프론트엔드 성능

## findings
## 0. 조사 방법·주의

- 코드는 수정하지 않았다. 벤치마크는 스크래치패드에서 `bun run` 으로 돌리고 지웠다(레포 무변경). 측정 엔진은 bun(JavaScriptCore) — 앱 런타임 WKWebView 와 같은 엔진 계열이라 상대 비교의 대리치로 쓸 만하다.
- 조사 중 `docs/backlog.md`, `src/widgets/command-palette/command-palette.tsx` 가 다른 워크플로에 의해 변경됐다. 아래 인용 줄번호는 조사 시점 기준이며, 병행 편집 중인 파일(explorer·command-palette·terminal·layout·file·system)은 착수 시 재확인이 필요하다.

## 1. 감사 §1(프론트 성능 14건) 반영 대조 — d-51 vs 백로그

정본: `docs/quality-assurance/2026-08-29-full-audit.md` §1, `docs/acknowledge/2026-08-29-d51-audit-frontend-batch-contract.md` §3 F7·§5 F7, `docs/backlog.md`.

| # | 항목 | 상태 | 코드 증거 |
|---|---|---|---|
| 1 | 코드 스플리팅 전무 | **부분 반영** — 5지점 lazy 분할 완료, **monaco 자체 지연은 미착수** | `preview-pane.tsx:29-32`, `pane-node-view.tsx:39`, `settings-view`/`plugin-manager`/`keybindings-editor` lazy. 계약 §5 F7 "monaco 자체는 지연하지 않았다" |
| 2 | PDF 워커 무조건 스폰 | **반영** | `shared/lib/pdf/setup.ts` → `getPdfjsWithWorker()`, 소비처 `pdf-preview.tsx` |
| 3 | 검색 스트리밍 O(n²) | **반영** | `entities/search/search-result.ts:80-101` 증분 append + `use-search-run.ts:24,142` 50ms 버퍼 flush |
| 4 | 검색 결과 비가상화 | **반영** | `features/search/search-results-list.tsx:42` `useVirtualizer` |
| 5 | 페인 리사이즈 홍수 | **반영(전제 정정 포함)** | `entities/layout/pane-resize-commit.ts` + `ipc-sync-provider.tsx:93,225` 자기 에코 억제 |
| 6 | 닫은 탭 monaco 모델 영구 보존 | **반영** | `entities/layout/tab-path-change.ts:75-79` `stillOpenElsewhere` false 시 `disposeModel` |
| 7 | grammar 31종 전량 로드 | **반영** | `shared/lib/shiki/lang-map.ts` 코어 3종 + `shiki-monaco.ts:43` `requestedLanguageIds` 온디맨드 |
| 8 | git 이벤트 GIT.PROJECT 전폭 무효화 | **반영** | `ipc-sync-provider.tsx:244-256` `isGitQueryScopeMutable` predicate |
| 9 | 키스트로크마다 `getValue()` | **반영** | `features/editor/code-editor.tsx:59-67,182` 지연 리더 `() => model.getValue()` |
| 10 | 팔레트 files fuzzy 전량 스캔(L1-03) | **미반영 — 백로그 "기지 유지"** | `command-palette.tsx:58` `FILE_RESULT_LIMIT=200`, `fuzzyFilter(searchTerm, filePaths, toProjectRelativePath)` 그대로 |
| 11 | 워크스페이스 심볼 미디바운스 | **반영** | `use-workspace-symbol-search.ts` 200ms trailing |
| 12 | 마커 전량 재수집 + Problems 비가상화 | **부분 반영** — 가상화만. **마커 전량 재수집은 이월** | `features/problems/problems-panel.tsx:49` 가상화 O / `shared/hooks/use-monaco-markers.ts:12` `getModelMarkers({})` 그대로 |
| 13 | fs:changed 경로당 2회 invalidate | **반영** | `ipc-sync-provider.tsx:46-50,304-307` predicate 1회 |
| 14 | 팔레트 상시 렌더 비용 | **반영(렌더 게이팅만)** — 스캔 알고리즘은 불변 | `command-palette.tsx` 모드 분기 게이트 + `command-registry` 스냅샷 |

백로그(`docs/backlog.md`)에 프론트 성능으로 남은 것은 **L1-03 팔레트 fuzzy 1건**뿐이고, 나머지 이월(monaco 지연·ts.worker·마커 재수집·grammar 코어 선정·tokens provider 전량 재부착)은 **`docs/backlog.md` 가 아니라 d-51 계약 §5 F7 안에만** 기록돼 있다. 이번 배치가 다루려면 백로그 등재부터 필요하다.

## 2. 현재 기준선 (이번에 실측)

**번들 (레포의 `dist/`, 2026-08-30 빌드 산출물 기준)**

- 부팅 페이로드(entry + `modulepreload`) = **19파일 5,838.96 kB**. 그중 monaco 3종 `standaloneServices` 1,615.8 + `editor.api` 980.2 + `toggleHighContrast` 1,160.0 = **3,756 kB(64.3%)**, 앱 entry `index` 1,694.0 kB.
- 전체 JS 157청크 20,103.96 kB. 최대 비부팅 자산: `ts.worker` 6.8MB급, `pdf.worker`, `rhwp_bg.wasm`.
- **렌더 차단 CSS 4장 244.6 kB**(monaco 163.4 + Tailwind 81.1) + `codicon.ttf` 141 kB — `dist/index.html` 의 `<link rel=stylesheet>` 4개.
- 웹폰트 없음(시스템 폰트만 — `shared/lib/font-stack.ts:1`). **폰트 로딩은 병목 아님.**

**테스트**: `bun test` = **1,835 pass / 0 fail / 191 파일 / 1.26s**.

**순수 함수 벤치(bun, 10~20회 평균)**

| 대상 | 규모 | 실측 | 판정 |
|---|---|---|---|
| `fuzzyFilter('edi', paths, toRelativePath)` | 5,000 파일 | **3.96 ms** | 팔레트 키 입력 1회분 |
| 〃 | 20,000 파일 | **12.68 ms** | 체감 시작 |
| 〃 | 50,000 파일 | **31.23 ms** | 명백한 입력 지연 |
| `buildSearchResultRows` | 10,500행 | 0.33 ms | **저위험 — 과대평가 방지 확인** |
| `buildFileTreeGitStatusByPath` + 트리 행 remap | status 3,000 / tree 20,000 | 1.17 ms | 저위험 |
| `APP_KEYMAP` 스캔 × 6 리스너 | 23 엔트리 | **0.002 ms/keystroke** | **무시 가능 — 가설 기각** |
| 키바인딩 에디터 충돌 O(n²) | 190행 | 0.41 ms/pass (렌더당 2회) | 중 |

> 키맵 디스패치 O(n) 은 "타이핑 지연 원인"이라는 가설을 세웠다가 실측으로 기각했다. 실측 없이 리스트에 넣지 않았다.

## 3. 새로 찾은 것 (파일:라인)

### H (영향 큼)

**H1. monaco 이거(eager) 로드가 부팅 페이로드의 64%** — `src/shared/lib/monaco/setup.ts:1` 의 `import * as monaco from 'monaco-editor'` 를 **27개 모듈이 값으로** import 한다(`bootstrap-lsp.ts:3`, `bootstrap-snippets.ts:4`, `entities/editor/model-registry.ts:2`, `features/outline/outline-symbol-row.tsx:4`(SymbolKind 상수 때문), `features/problems/problem-severity.ts` 등). React 렌더 이전에 3.76MB JS + 163kB CSS 를 파싱·평가하므로, **터미널만 여는 세션도 전액을 지불**한다. 첫 페인트 게이트는 `theme-provider.tsx:104`/`use-reveal-window.ts:4` 의 `getCurrentWindow().show()` 이므로, monaco 평가는 그 앞단 고정 비용이다.

**H2. 마커 스냅샷이 상태바 때문에 앱 수명 내내 켜져 있다** — `use-monaco-markers.ts:27-39` 는 소비자 refcount 로 구독을 켜고 끄지만, `widgets/window-chrome/status-bar-content.tsx:51` 의 `useMonacoMarkers()` 가 **항상 마운트된 소비자**다. 따라서 Problems 패널을 열지 않아도 `onDidChangeMarkers` 마다 `getModelMarkers({})`(`use-monaco-markers.ts:12`)가 **전 모델 전 마커 배열을 새로 만든다**. monaco `MarkerService.read({})` 는 owner/resource 필터가 없으면 전량 순회다(`node_modules/monaco-editor/esm/vs/platform/markers/common/markerService.js:256`). LSP 초기 인덱싱은 파일 단위 `publishDiagnostics` 를 각기 다른 매크로태스크로 밀어 넣으므로(`shared/lib/lsp/adapters/diagnostics.ts:97` `setModelMarkers`), **파일 수 × 총 마커 수**가 된다. 완화 요인: 같은 마이크로태스크 내 변경은 `MicrotaskEmitter`(같은 파일 132행)가 합친다. 상태바가 실제로 쓰는 값은 `errorCount` 하나뿐(`status-bar-content.tsx:55`).

**H3. 팔레트 fuzzy 의 알고리즘 세부** (L1-03 의 구체화) — `shared/lib/fuzzy-match.ts:35` 가 후보 **라벨마다 코드포인트 객체 배열을 새로 할당**하고, `:44-46` 의 `findIndex` 는 `position >= searchFromPosition` 가드를 **콜백 안에서** 검사해 매 질의 문자마다 접두부를 헛스캔하며, 코드포인트마다 `toLowerCase()` 를 호출한다. 여기에 `command-palette.tsx` 가 항목마다 `toProjectRelativePath` 로 상대경로 문자열을 새로 만든다. 실측 §2 표 참조(50k 파일 31ms/keystroke).

### M (영향 중간)

**M1. git 변경 목록이 비가상화 + 행마다 Radix ContextMenu** — `features/git/git-change-group.tsx:168-169` 가 `config.rows.map` 으로 행마다 `<ContextMenu>` 루트+트리거를, `:180-190` 에서 7개 메뉴 항목 엘리먼트를 **미리 생성**한다. `widgets/git-panel/git-panel.tsx:207-247` 은 merge/staged/unstaged 3그룹을 `ScrollContainer` 안에 그대로 쏟는다. 파일트리·검색·Problems·커밋그래프는 전부 가상화됐는데(`file-tree.tsx:115`, `search-results-list.tsx:42`, `problems-panel.tsx:49`, `commit-graph.tsx:61`) **git 변경 목록만 빠졌다**. 브랜치 전환·대량 생성 리포에서 수천 행이 통째로 DOM 에 들어간다.

**M2. 아웃라인 패널 전량 재귀 렌더** — `features/outline/outline-panel.tsx:26` 이 `symbols.map`, `features/outline/outline-symbol-row.tsx:51` 이 `symbol.children?.map` 으로 **접힘 없이 전 심볼 트리**를 그린다. 가상화도 collapse 도 없고, 문서 심볼은 편집 후 400ms 디바운스로 재요청돼 통째로 다시 그려진다.

**M3. 파일 내용/원본 바이트 캐시가 탭을 닫아도 남는다** — `entities/file/file.query.ts:34-40`(`FILE.CONTENT`)·`:61-67`(`FILE.RAW`) 는 `staleTime: Infinity`, gcTime 은 전역 기본 10분(`app/query-client.ts:5`). `entities/layout/tab-path-change.ts:64-80` 의 `releaseClosedFileTabPath` 는 미러 정리·`disposeModel` 까지만 하고 **두 캐시는 `removeQueries` 하지 않는다**. 감사 §1-6 이 회수한 monaco 버퍼와 **같은 크기의 사본이 쿼리 캐시에 10분 더 남는다.** `FILE.RAW` 는 PDF/이미지/xlsx 원본 `ArrayBuffer` 라 수십 MB 단위다(`widgets/preview-pane/preview-pane.tsx:55`).

**M4. 팔레트 결과 200행 비가상화** — `command-palette.tsx:58` `FILE_RESULT_LIMIT = 200`. cmdk `CommandList` 안에 200개 `CommandItem` 이 들어가며, 각 항목은 cmdk 의 값 등록·점수 산출을 거친다. 타이핑마다 전량 재조정.

**M5. 레이아웃 쿼리 구독 폭이 넓고 `select` 로 좁힌 곳이 0** — `layoutQueryOptions` 를 14개 컴포넌트가 통째로 구독한다(`editor-area.tsx:76`, `pane-tab-bar.tsx:81`, `breadcrumbs-bar.tsx:59`, `status-bar-content.tsx:45`, `title-bar-content.tsx:11`, `terminal-session.tsx:57`, `search-editor-pane.tsx:66`, `outline-panel-container.tsx:30`, `welcome-container.tsx:51`, `use-zen-mode.ts:27`, `use-editor-view-state.ts:130`, `auxiliary-window-shell.tsx:41`, `auxiliary-title-bar-content.tsx:23`, `command-palette.tsx:114`). `applyFreshLayout`(`layout.query.ts:58-63`)는 매 뮤테이션마다 새 `ProjectLayout` 객체를 심으므로, **탭 dirty 토글 1회에 14개 구독자 + 그 서브트리 전체가 리렌더**된다. TanStack v5 의 `select`(구조 공유 기반 좁히기)를 쓰는 레이아웃 소비처는 하나도 없다.

**M6. 저장 1회마다 프로젝트 전역 git status 재계산** — `ipc-sync-provider.tsx:342` 가 `fs:changed` 마다 `GIT.STATUS(projectId)` 를 **무조건** 무효화한다(주석대로 경로 축이 없어서). Rust `git_status` 는 백로그 M-2(결과 캐시 미도입) 상태라 전량 재계산이고, 그 응답이 `explorer-container.tsx:74-75` 의 데코레이션 재계산 + 트리 행 전량 remap 을 유발한다. JS 쪽 remap 은 실측 1.17ms(20k행)로 싸지만 **IPC + Rust 전량 스캔이 저장마다** 붙는다.

### L (영향 작음 / 확인용)

**L1. git 변경 목록 방향키 핸들러가 키 입력마다 O(n) DOM 질의** — `widgets/git-panel/git-panel.tsx:141` `querySelectorAll('[data-git-change-row]')` + `:143` `indexOf`. 가상화 도입 시 어차피 재작성 대상.

**L2. 키바인딩 에디터 O(n²) 충돌 판정 ×2** — `keybindings-editor.tsx:93`(`conflictCount`)과 `:276`(행마다 `findConflictingRow`). 190행 기준 렌더당 0.41ms × 2. 검색 입력 타이핑마다 재실행. 비가상화 목록이기도 하다.

**L3. `OverlayScrollbar` 의 MutationObserver 가 자식 전량 재관찰** — `shared/hooks/use-overlay-scrollbar.ts:99-111`: `childList` 변경마다 `Array.from(viewport.children)` 전체를 `resizeObserver.observe`. 가상화 목록은 뷰포트 직속 자식이 sizing div 1개뿐이라 무해하지만, `ScrollContainer`(`shared/scroll/scroll-container.tsx:42`)로 감싼 **비가상화 목록(아웃라인·git 패널·설정)** 은 직속 자식이 곧 행이라 행 수만큼 ResizeObserver 대상이 된다. `orientation='both'` 는 같은 뷰포트에 관찰자 세트를 **2벌** 만든다(`:44-57`).

**L4. `parseKeymapOverrides` 는 렌더마다 `JSON.parse`** — `shared/lib/keymap/keymap.ts:258-269`, 호출부 `use-global-keymap.ts:57`(6개 마운트 소비자) + `command-palette.tsx:128`. React Compiler 가 `overridesJson` 안정 시 메모하지만, 컴파일러 메모 무효화 시점에는 리스너 수만큼 반복된다. 동기 JSON 파싱은 이 한 곳뿐(설정 본문은 Rust 가 파싱).

### 양호 확인 (수정 불요 — 오탐 방지용 기록)

- **터미널 출력 경로는 이미 최적**: `entities/terminal/terminal.ipc.ts:10-14` 가 `Channel<ArrayBuffer>`, Rust 는 `InvokeResponseBody::Raw`(`src-tauri/src/domain/terminal/commands.rs:181`)로 **바이너리 무변환** 전달. JSON 배열/base64 경유 없음.
- **React Context 0개**(`grep createContext src` = 0). 고빈도 Context 갱신 문제는 존재하지 않는다.
- **`staleTime`/`gcTime` 은 대체로 의도적**: 전역 60s/10min(`query-client.ts:4-5`), 불변 스코프는 `Infinity`, 폴링은 `system.query.ts:11`(3s, `refetchIntervalInBackground:false`)·`remote.query.ts:10`(5s, 설정 화면 마운트 중에만). **부적절한 값은 못 찾았다.** 유일한 문제는 M3 의 "큰 페이로드 + 긴 gcTime" 조합.
- **i18n·폰트**: `shared/i18n/i18n.ts:7-15` 는 리소스 빈 상태로 init 후 `addResourceBundle` 1회. 웹폰트 없음.
- **CSS 변수 적용**: `shared/lib/theme-variables.ts:10-11` 이 ~114개 변수를 `documentElement` 에 `setProperty`. 테마 변경/프리뷰 드래그에서만 발생하고 드래그는 이미 rAF 코얼레스+150ms 디바운스(d-45).

## 4. 지표 후보와 측정 방법

현재 프론트에 성능 계측이 **하나도 없다.** Rust 만 `setup 완료: elapsed_ms=`(`src-tauri/src/lib.rs:619`)를 남긴다. `docs/debugging.md` §4 의 실증 기법(rAF 프레임 갭 계측, 원격 페이지 WS 몽키패치, 대조 환경 격리)이 재사용 가능한 선례다.

| 지표 | 정의 | 측정 방법 |
|---|---|---|
| 부팅→첫 페인트 | 프로세스 시작 → `getCurrentWindow().show()` | Rust `setup 완료` 로그 + 프론트에 `performance.timeOrigin` 기준 마크 3개(`main.tsx` 모듈 평가 끝 / 첫 커밋 / `use-reveal-window.ts` show 직전)를 `log` 포워딩(`shared/lib/error-log-forwarding.ts` 경로 재사용)으로 남긴다 |
| monaco 평가 비용 | monaco 청크 파싱+평가 시간 | `shared/lib/monaco/setup.ts` 진입/이탈 마크. devtools Performance 의 "Evaluate Script" 로 교차 확인 |
| 프로젝트 전환 | `project_open` 응답 → 트리 첫 행 페인트 | `explorer-container` 첫 `rows.length>0` 커밋 마크 |
| 파일 열기(소/대) | 탭 클릭 → 에디터 모델 attach 완료 | `code-editor.tsx:304-315` 모델 attach effect 양끝 마크 (1KB/1MB 픽스처 2종) |
| 팔레트 열기 | ⌘P → 입력 포커스 | `command-palette.tsx` `open` 전환 → `useEffect` 마크 |
| 팔레트 타이핑 응답 | keydown → 결과 커밋 | `fuzzyFilter` 호출 양끝 마크 + **bun 벤치(§2 표)로 회귀 고정** |
| 트리 펼침 | `tree_toggle` 호출 → 새 행 페인트 | 뮤테이션 `onSuccess` → 다음 커밋 |
| 검색 결과 렌더 | 첫 배치 도착 → 첫 행 페인트 / 완료까지 프레임 갭 최대값 | `use-search-run.ts` flush 마크 + rAF 갭 계측(d-45 선례) |
| 탭 전환 | activate → 새 pane 페인트 | `pane-node-view` 커밋 마크 |
| 터미널 출력 처리량 | `yes`/대용량 cat 시 초당 바이트 + 프레임 갭 | xterm write 콜백 카운터 + rAF 갭 |
| 메모리 | 파일 20개 열고 닫은 뒤 heap | devtools Memory 스냅샷 + `monaco.editor.getModels().length` + `queryClient.getQueryCache().getAll()` 바이트 추정 |

**bun test 로 회귀 고정 가능한 순수 함수 후보**(전부 DOM·monaco 무의존): `fuzzy-match.ts`, `command-palette-file-match.ts`, `search-result.ts`(`appendSearchFileMatches`), `search-result-rows.ts`, `problem-list-rows.ts`, `file-tree-git-status.ts`, `keybinding-catalog.ts`(`findConflictingRow`), `keymap.ts`(`matchesKeymapEntry`), `graph-lanes.ts`, `pane-tree.ts`(`collectAllPaneTabs`), `theme-convert/*`.
**주의**: `bun:test` 에는 `bench` export 가 없다(실측: `afterAll afterEach beforeAll beforeEach default describe expect expectTypeOf it jest mock onTestFinished setDefaultTimeout setSystemTime spyOn test vi xdescribe xit xtest`). 벤치는 ① 의존성 없는 `scripts/bench-*.ts`(선례 `scripts/convert-vscode-theme.ts` 가 `@shared` alias 를 그대로 쓴다) 또는 ② **연산 횟수 카운터 기반 단위 테스트**(벽시계 비의존 → CI 안정)로 만든다.

## 5. 개선 후보 우선순위

| # | 항목 | 영향 | 난이도 | 회귀 위험 | 근거 |
|---|---|---|---|---|---|
| 1 | H1 monaco 지연 + "에디터 없는 첫 프레임" 셸 | **특대** (부팅 페이로드 −64%) | **특대** | **높음** (27개 값 import, monaco 초기화 순서 계약) | §2 실측 |
| 2 | H3 fuzzy 알고리즘 정정(코드포인트 배열 제거·인덱스 커서·소문자 사전계산·상대경로 캐시) | 큼 | 소~중 | 낮음 (순수 함수 + 기존 테스트 존재) | 3.96→? ms, 50k 31ms |
| 3 | H2 마커 스냅샷 축소(상태바는 severity 카운트만 구독) | 큼 (LSP 인덱싱 구간) | 중 | 중 (`useSyncExternalStore` 불변 참조 계약) | `status-bar-content.tsx:51,55` |
| 4 | M1 git 변경 목록 가상화 + 컨텍스트 메뉴 단일화 | 큼 | 중 | 중 (L1 키보드 내비 재작성 동반, 백로그 "가상화 목록 키보드 도달성"과 합류) | `git-change-group.tsx:168` |
| 5 | M3 탭 닫힘 시 `FILE.CONTENT`/`FILE.RAW` 회수 | 큼(메모리) | 소 | 중 (스플릿·다중 창에서 살아 있는 경로 오회수 주의 — `stillOpenElsewhere` 판정 재사용) | `tab-path-change.ts:75` |
| 6 | M2 아웃라인 평탄화+가상화(+접기) | 중 | 중 | 낮음 | `outline-panel.tsx:26` |
| 7 | M4 팔레트 결과 가상화(cmdk 와 공존 설계 필요) | 중 | 중 | 중 (cmdk 키보드 선택·스크롤 동기) | `command-palette.tsx:58` |
| 8 | M5 레이아웃 구독 `select` 좁히기(구조 공유) | 중 | 중 | **높음** (14곳, d-42/d-43 revision 계약과 얽힘) | `layout.query.ts:58` |
| 9 | L2 키바인딩 충돌 인덱스화(O(n²)→O(n)) + 가상화 | 소 | 소 | 낮음 | 0.41ms×2/render |
| 10 | L3 OverlayScrollbar 관찰 대상 축소(`subtree:false` 유지 + 자식 delta 관찰) | 소 | 소 | 중 (15표면 공용) | `use-overlay-scrollbar.ts:99` |
| 11 | M6 저장마다 git status 전량 — Rust M-2 선행 필요 | 중 | 큼 | — | 백로그 이관 상태 |
| — | 키맵 디스패치 O(n) | **없음(기각)** | — | — | 실측 0.002ms |
| — | 검색 flatten O(n)/flush | **없음(기각)** | — | — | 실측 0.33ms |

## 6. 테스트 공백 맵 — `shared/*`·`entities/*` 중 대응 `.test.ts` 없는 파일 상위 30 (줄 수 내림차순)

전체: 266개 중 **113개 무테스트(42%)**.

| 줄 | 파일 | 비고 |
|---|---|---|
| 2468 | `src/shared/api/bindings.ts` | 생성물 — 대상 아님 |
| 837 | `src/shared/lib/monaco/monaco-actions.ts` | 데이터 테이블(158 액션) — 스키마/중복 id 계약 테스트 가치 |
| 309 | `src/shared/lib/shiki/shiki-monaco.ts` | `runExclusive` 큐·`ensureShikiLanguage` 순서 계약 미고정 |
| 202 | `src/shared/lib/ai/inline-completion.ts` | |
| 200 | `src/shared/hooks/use-overlay-scrollbar.ts` | L3 대상 |
| 195 | `src/entities/file/file.query.ts` | M3 대상 — 캐시 패치/무효화 계약 |
| 179 | `src/entities/search/use-search-run.ts` | 세대 가드·flush 병합 |
| 173 | `src/shared/lib/theme-convert/ui-token-vocabulary.ts` | |
| 155 | `src/shared/lib/lsp/initialize-params.ts` | |
| 123 | `src/shared/lib/theme-convert/semantic-token-map.ts` | |
| 113 | `src/entities/theme/theme.query.ts` | |
| 98 | `src/entities/git/git.ipc.ts` | |
| 83 | `src/shared/lib/theme-convert/resolve-colors.ts` | |
| 81 | `src/entities/project/project.query.ts` | |
| 80 | `src/shared/lib/lsp/adapters/document-symbol.ts` | 어댑터 20종 중 무테스트 |
| 76 | `src/shared/lib/lsp/adapters/completion.ts` | 〃 |
| 76 | `src/entities/settings/settings.ipc.ts` | |
| 66 | `src/entities/agent/agent.query.ts` | |
| 64 | `src/shared/icons/file-icon-registry.ts` | |
| 62 | `src/shared/lib/theme-convert/ansi-palette.ts` | |
| 62 | `src/shared/lib/monaco/monaco-action-commands.ts` | |
| 58 | `src/entities/lsp/lsp.ipc.ts` | |
| 55 | `src/entities/settings/settings.query.ts` | |
| 54 | `src/shared/lib/theme-convert/types.ts` | |
| 50 | `src/entities/layout/layout.ipc.ts` | |
| 47 | `src/shared/hooks/use-keydown-capture.ts` | |
| 45 | `src/entities/plugin/plugin.query.ts` | |
| 43 | `src/shared/lib/theme-convert/merge.ts` | |
| 43 | `src/entities/sync/sync.commands.ts` | |
| 43 | `src/entities/search/search.ipc.ts` | |

성능 배치와 직결되는 무테스트 파일: `file.query.ts`(M3), `use-overlay-scrollbar.ts`(L3), `shiki-monaco.ts`(perf-7 후속), `use-search-run.ts`. 반대로 이번 조사가 손댈 순수 함수 중 **이미 테스트가 있는 것**: `fuzzy-match.test.ts`, `search-result.test.ts`, `search-result-rows.test.ts`, `problem-list-rows.test.ts`, `file-tree-git-status.test.ts`, `keybinding-catalog.test.ts`, `keymap.test.ts` — H3·L2 는 기존 테스트가 안전망이 된다.

## options
## 안 A — 계측 기준선 우선(measurement-first), 수정은 무결정 국소분만 (추천)

**흐름**: ① 프론트 부팅 마크 3종 + 지표별 계측 지점을 `shared/lib` 의 얇은 계측 모듈 하나로 추상화(개발 빌드에서만 로그 포워딩, 프로덕션 no-op) → ② `docs/quality-assurance/` 에 지표·측정 절차 체크리스트 신설 → ③ 순수 함수 벤치 스크립트(`scripts/bench-frontend.ts`, 의존성 0) + 연산 횟수 기반 예산 테스트 추가 → ④ 회귀 위험이 낮고 결정이 불요한 3건만 이번에 고친다: **H3 fuzzy 알고리즘**, **M3 캐시 회수**, **L2 충돌 인덱스화**.

**변경 파일**: `src/shared/lib/perf-mark.ts`(신설) · `src/main.tsx` · `src/shared/hooks/use-reveal-window.ts` · `src/shared/lib/fuzzy-match.ts`(+테스트) · `src/entities/layout/tab-path-change.ts`(+테스트) · `src/shared/lib/keymap/keybinding-catalog.ts`(+테스트) · `scripts/bench-frontend.ts`(신설) · `docs/quality-assurance/…`.

**데이터 흐름**: 계측은 `performance.now()` → 기존 `error-log-forwarding` 경로로 `TAIDE.log` 에 적재 → `docs/debugging.md` §1 의 로그 창구로 회수. 기능 데이터 흐름은 불변.

**장점**: "실측 후 판단"이라는 L1-03 의 기존 결정을 그대로 이행한다. 회귀 위험이 최소이고, 이후 대형 항목(H1·M5)의 효과를 숫자로 증명할 수 있다. 신규 의존성 0.
**단점**: 부팅 시간의 가장 큰 덩어리(monaco)는 이번에 줄지 않는다. 계측 코드가 프로덕션 경로에 no-op으로라도 들어간다.

## 안 B — 렌더 비용 국소 제거(가상화·구독 축소) 묶음

**흐름**: 화면 단위로 남은 비가상화 목록과 과잉 구독을 한 배치로 정리한다. **M1 git 변경 목록**(가상화 + 행 컨텍스트 메뉴를 단일 인스턴스로 승격 + L1 키보드 내비를 `file-tree.tsx` 의 컨테이너 로빙 포커스 패턴으로 통일), **M2 아웃라인 평탄화·가상화·접기**, **M4 팔레트 결과 가상화**, **H2 마커 구독 축소**, **L3 스크롤바 관찰 축소**.

**변경 파일**: `src/features/git/git-change-group.tsx` · `src/widgets/git-panel/git-panel.tsx` · `src/widgets/git-panel/change-row-navigation.ts` · `src/features/outline/outline-panel.tsx` · `src/features/outline/outline-symbol-row.tsx` + `outline-rows.ts`(신설, 평탄화 순수 함수) · `src/widgets/command-palette/command-palette.tsx` · `src/shared/ui/command.tsx` · `src/shared/hooks/use-monaco-markers.ts` · `src/widgets/window-chrome/status-bar-content.tsx` · `src/widgets/problems-panel/problems-panel-container.tsx` · `src/shared/hooks/use-overlay-scrollbar.ts`.

**데이터 흐름**: 각 목록이 "그룹 트리 → 평탄 행 배열 → 가상화 윈도" 라는 기존 확립 패턴(`search-result-rows.ts`·`problem-list-rows.ts`)을 재사용한다. 마커는 "전량 스냅샷 1개" → "severity 카운트 스냅샷 + 전량 스냅샷" 2단 셀렉터로 분리해, 상태바가 전량 배열을 구독하지 않게 한다.

**장점**: 확장성 있는 추상화가 이미 레포 안에 있어 새 개념을 도입하지 않는다. 대형 리포에서 체감이 가장 큰 축(git 패널·아웃라인).
**단점**: 가상화가 접근성(Tab 순회)을 깨는 기지 이슈(백로그 등재분)를 **이번 배치가 함께 해결하지 않으면 확대**된다. cmdk 와 가상화의 공존은 라이브러리 내부 계약 확인이 필요하다. 계측 없이 들어가면 효과를 숫자로 못 남긴다.

## 안 C — 부팅 구조 수술(monaco 지연 + 에디터 없는 첫 프레임)

**흐름**: `shared/lib/monaco/setup.ts` 의 정적 `import * as monaco` 를 **비동기 접근자**(`loadMonaco(): Promise<Monaco>` + 이미 로드된 경우의 동기 getter)로 바꾸고, 27개 값 소비처를 ① 에디터가 실제로 필요한 지점(코드 에디터·diff·프리뷰 없는 탭)에서 await 하는 쪽과 ② 상수만 필요한 쪽(`outline-symbol-row.tsx` 의 `SymbolKind`, `problem-severity.ts` 의 `MarkerSeverity`)은 **로컬 상수 테이블로 대체**하는 쪽으로 가른다. `bootstrap-lsp`/`bootstrap-snippets` 는 `app.tsx` 모듈 top-level import 에서 빼고 첫 에디터 마운트에 종속시킨다. 첫 프레임은 셸(타이틀바·사이드바·상태바) + 탭 콘텐츠 Suspense.

**변경 파일**: `src/shared/lib/monaco/setup.ts` · `src/app/app.tsx` · `src/app/bootstrap-lsp.ts` · `src/app/bootstrap-snippets.ts` · `src/app/providers/{theme-provider,emmet-provider,keybindings-runtime-provider}.tsx` · `src/entities/editor/model-registry.ts` · `src/entities/editor/reveal-registry.ts` · `src/entities/lsp/lsp-session-registry.ts` · `src/features/outline/outline-symbol-row.tsx` · `src/features/problems/problem-severity.ts` · `src/shared/hooks/use-monaco-markers.ts` · `src/shared/lib/shiki/shiki-monaco.ts` · `src/widgets/**`(monaco 값 소비처 전량) · `vite.config.ts`.

**데이터 흐름**: `monaco` 가 "모듈 상수"에서 "약속(promise) 뒤의 자원"으로 바뀐다. shiki 초기화(`theme-provider.tsx:95-102`)·LSP 어댑터 등록·키바인딩 런타임이 전부 그 약속에 줄을 서야 하므로, `runExclusive` 큐(`shiki-monaco.ts:63`)와 유사한 **부팅 순서 계약 1개**를 새로 세워야 한다.

**장점**: 부팅 페이로드 −3.76MB(−64%), 렌더 차단 CSS −163kB. 터미널·프리뷰만 쓰는 세션의 첫 페인트가 구조적으로 빨라진다. 다른 어떤 항목보다 효과가 크다.
**단점**: 접촉 파일 30+, 부팅 순서 계약 신설, d-48(설치본 CSP·monaco 런타임 스타일)·d-45(테마 적용 순서)·Wave H(키맵 런타임)와 전부 교차한다. 회귀가 나면 "빈 창"(d-20 blank-window 클래스) 형태로 나타난다. 실측 없이 들어가면 개선폭도 증명 못 한다.

## recommendation
**안 A 를 먼저, 그 산출물 위에 안 B, 안 C 는 별도 배치로 분리한다.**

이유 세 가지.

1. **이 레포의 기존 결정이 "실측 후 판단"이다.** L1-03(팔레트 fuzzy)은 d-42 §5 에서 "실측 후 판단"으로 이월됐고 `docs/backlog.md` "기지 유지"에 그대로 남아 있다. 이번 조사에서 처음으로 숫자(5k=3.96ms / 20k=12.68ms / 50k=31.23ms)가 나왔으니, 그 결정 조건이 충족된 셈이다. 계측 하네스를 먼저 세워야 나머지 항목도 같은 기준으로 처분할 수 있다.

2. **계측 없는 최적화는 이번 조사에서 실제로 두 번 틀렸다.** 키맵 디스패치 O(n)(0.002ms)과 검색 flatten O(n)/flush(0.33ms)는 코드만 보면 유력한 병목이었지만 실측으로 기각됐다. 감사 §1-5(리사이즈 홍수)도 d-51 이 "현재 라이브러리에서 성립하지 않는다"로 전제를 정정한 선례가 있다. 안 B·C 를 계측 없이 먼저 하면 같은 실수를 반복할 위험이 크다.

3. **안 A 에 포함한 수정 3건(H3·M3·L2)은 결정이 필요 없고 회귀 위험이 낮다.** 전부 기존 테스트가 있는 순수 함수이거나(`fuzzy-match.test.ts`, `keybinding-catalog.test.ts`) 기존 판정 로직(`stillOpenElsewhere`)을 재사용하는 단일 지점이다. IPC 표면·로케일 키·의존성 변화가 0이라 릴리스 리스크도 없다.

안 B 는 계측 기준선이 생긴 직후의 2차 배치로, **백로그의 "검색 결과·Problems 목록 키보드 도달성" 항목과 반드시 묶어서** 착수한다(가상화가 접근성을 깎는 부채를 더 늘리지 않기 위해). 안 C 는 부팅 계약을 새로 세우는 규모라 **단독 배치 + 실기 확인 + e2e 스펙 선행**이 전제이며, 사용자 결정 사항이다(아래 질문 1).

## filesToTouch
- src/shared/lib/perf-mark.ts
- src/shared/lib/perf-mark.test.ts
- src/main.tsx
- src/shared/hooks/use-reveal-window.ts
- src/shared/lib/monaco/setup.ts
- src/shared/lib/fuzzy-match.ts
- src/shared/lib/fuzzy-match.test.ts
- src/widgets/command-palette/command-palette.tsx
- src/entities/layout/tab-path-change.ts
- src/entities/layout/tab-path-change.test.ts
- src/entities/file/file.query.ts
- src/entities/file/file.query.test.ts
- src/shared/lib/keymap/keybinding-catalog.ts
- src/shared/lib/keymap/keybinding-catalog.test.ts
- src/widgets/keybindings-editor/keybindings-editor.tsx
- scripts/bench-frontend.ts
- package.json
- docs/quality-assurance/2026-09-04-frontend-perf-baseline.md
- docs/backlog.md
- docs/PROCESS.md
- docs/acknowledge/2026-09-04-frontend-perf-contract.md

## risks
- 부팅 계측이 프로덕션 경로에 남는 위험 — `perf-mark` 가 no-op 이 아니거나 로그를 과다 발행하면 오히려 부팅이 느려지고 40KB 회전 로그(`docs/debugging.md` §1)를 잡아먹어 실사용 진단 창구를 망가뜨린다. dev/prod 분기와 발행 상한을 계약에 고정해야 한다.
- `fuzzy-match.ts` 정정이 매칭 결과·하이라이트 인덱스를 바꿀 위험 — `indices` 는 UTF-16 코드유닛 오프셋 계약이고, 서로게이트 쌍/터키어 'İ' 같은 케이스를 위해 코드포인트 단위로 짜여 있다(파일 주석 §22-31). 소문자 사전계산으로 바꾸면 그 불변식이 깨질 수 있다. 기존 `fuzzy-match.test.ts` 가 이 케이스를 덮는지 먼저 확인하고, 부족하면 테스트를 먼저 보강한 뒤 손댄다.
- `FILE.CONTENT`/`FILE.RAW` 회수가 살아 있는 소비처를 끊을 위험 — 같은 경로가 다른 창·스플릿·diff 탭·프리뷰·Search Editor 에서 동시에 열려 있을 수 있고, `releaseClosedFileTabPath` 의 `stillOpenElsewhere` 는 `layout` 한 개(그 창의 캐시)만 본다. 다른 창의 레이아웃까지 훑지 않으면 보조 창의 열린 탭 캐시를 지운다. `collectOpenFilePathsOutsideProject`(`ipc-sync-provider.tsx:134`)가 이미 같은 문제를 푼 선례라 그 형태를 재사용해야 한다.
- 쿼리 캐시 제거가 d-43(저장 직후 구식 캐시 채택 클로버) 계약과 충돌할 위험 — `useSaveFile` 의 캐시 동기 패치가 전제하는 엔트리를 회수 타이밍이 앞질러 지우면, 같은 파일을 다시 열 때 빈 캐시 → 재fetch 경로가 열려 d-43 이 닫은 역행 창이 다시 생길 수 있다. 회수는 '탭이 정말 전부 닫힌 뒤' 한 지점에서만 해야 한다.
- 마커 구독 축소가 `useSyncExternalStore` 불변 참조 계약을 깰 위험 — severity 카운트를 파생 스냅샷으로 만들면 매 호출 새 객체가 되어 무한 렌더 루프가 난다(getSnapshot 은 캐시된 동일 참조를 돌려줘야 한다). d-51 §5 가 이 이유로 증분화를 이월한 바로 그 지점이다.
- git 변경 목록 가상화가 접근성을 추가로 깎을 위험 — 백로그에 이미 '가상화 목록 키보드 도달성' 부채가 있고, git 패널은 방금(4faff32) 방향키 로빙 포커스를 넣은 참이다. 가상화하면 `querySelectorAll` 기반 내비가 뷰포트 밖 행을 못 찾아 조용히 끊긴다 — 같은 배치에서 인덱스 기반 로빙 + `scrollToIndex` 로 갈아타야 한다.
- monaco 지연(안 C)이 '빈 창' 회귀를 부를 위험 — 첫 페인트 게이트가 theme/locale 두 신호이고(`use-reveal-window.ts`), shiki·키맵 런타임·LSP 어댑터가 전부 monaco 인스턴스에 줄 서 있다. 순서 계약을 하나라도 놓치면 d-20 blank-window·d-48 설치본 색상 전무와 같은 클래스의 결함이 난다. dev 에서만 재현되지 않는 종류라 설치본 실기 확인이 필수다.
- 벤치 스크립트가 CI 를 불안정하게 만들 위험 — 벽시계 기반 예산 단언은 러너 성능 편차로 깨진다. `bun run verify` 에 넣을 것은 연산 횟수 카운터 기반 테스트만으로 하고, 시간 벤치는 수동 실행 스크립트로 분리해야 한다.
- 병행 편집 충돌 — 다른 워크플로가 `src/widgets/explorer`·`command-palette`·`terminal-pane`, `src/entities/layout`·`file`·`system` 를 지금 고치고 있다. 이 조사가 지목한 `command-palette.tsx`·`tab-path-change.ts`·`file.query.ts` 가 정확히 그 범위와 겹치므로, 착수 전에 해당 파일들을 다시 읽고 줄번호·구조를 재확인해야 한다(조사 중 이미 `command-palette.tsx` 가 변경됐다).
- 감사 §1 의 이월분이 `docs/backlog.md` 에 없다 — monaco 지연·ts.worker·마커 재수집·grammar 코어 선정은 d-51 계약 §5 안에만 있어, 백로그만 보고 판단하면 '이미 처리됨'으로 오독된다. 배치 착수 시 백로그 등재부터 해야 재발을 막는다.

## questionsForUser
- monaco 지연(안 C, 부팅 페이로드 −3.76MB/−64%)을 이번 성능 배치에 넣습니까? — **A안(추천): 이번엔 제외하고 안 A 로 기준선부터 만든 뒤 별도 배치**(접촉 파일 30+, 부팅 순서 계약 신설, d-45·d-48·Wave H 교차, 실패 모드가 '빈 창') / B안: 이번 배치에 포함해 한 번에 / C안: 영구 보류하고 백로그에만 등재
- 성능 계측 코드를 프로덕션 빌드에도 남깁니까? — **A안(추천): dev 전용, prod 에서는 no-op**(로그 40KB 회전 창구를 보호) / B안: prod 에도 부팅 마크 3개만 남겨 설치본 실측 가능 / C안: 계측 없이 devtools·수동 측정만
- 가상화 확대(안 B)와 '가상화 목록 키보드 도달성'(백로그 등재분)을 함께 처리합니까? — **A안(추천): 함께**(git 변경 목록 가상화는 방금 넣은 방향키 내비를 깨므로 사실상 필수) / B안: 가상화만 먼저, 접근성은 별도 / C안: 접근성 부채가 해소될 때까지 추가 가상화 보류
- 벤치마크 형식 — **A안(추천): `bun run verify` 에는 연산 횟수 카운터 기반 예산 테스트만, 시간 벤치는 수동 `scripts/bench-frontend.ts`** / B안: 시간 벤치도 관대한 상한으로 verify 에 포함 / C안: 벤치 없이 실기 계측만
- 이번 배치의 범위 — **A안(추천): 안 A 전체(계측+벤치+H3·M3·L2 3건)** / B안: 안 A + 안 B(가상화 묶음)까지 / C안: 계측 하네스만, 수정은 전부 다음 배치

## newDependencies



# 주제 3b — Rust 성능

## findings
## 0. 전제 확인 (기존 결정과의 중복 회피)

- **릴리스 프로필은 이미 결정·반영 완료**입니다. `Cargo.toml:5-10` 이 `lto = "thin"` · `codegen-units = 1` · `panic = "abort"` · `strip = true` · `opt-level = 3` (커밋 `9575a33`, full-audit §6 Q1 답변분). **이 축에 새 제안은 없습니다** — 다만 `strip = true` 때문에 릴리스 바이너리에 심볼이 없어 Instruments/`sample` 프로파일링이 불가능하니, 측정용 임시 프로필이 필요합니다(§4).
- 사용자 결정 `docs/acknowledge/2026-09-04-usability-batch4-user-decisions.md` §2 에서 **"8지표 계측 내장 + 신규 의존성 0"** 이 이미 확정되어 있습니다. 아래 §4 는 그 결정의 Rust 측 구현안입니다.
- `docs/backlog.md` "전수조사에서 이관된 후속 후보 → Rust 성능·설계" 7항목과 `docs/acknowledge/2026-08-29-d50-audit-rust-batch-contract.md` §5 이월분은 전부 **재발견이 아니라 대조·재평가** 대상으로 취급했습니다.

---

## 1. full-audit §2 Rust 성능 19건 — d-50 반영분 대조 (전수)

| # | 항목 | 현재 상태 (코드 실측) |
|---|---|---|
| H-1 | 검색 단일 스레드 | **해소.** `domain/search/service.rs:513` `configure_walk(...).build_parallel().run(...)` + `std::thread::scope` + mpsc 드레인 |
| H-2 | 검색 파일 전체 read 후 sniff | **해소.** `search/service.rs:260` `read_scannable_bytes` — `metadata.len() >= REFUSED_FILE_BYTES` 선차단 + `take(BINARY_SNIFF_BYTES)` 선-sniff |
| H-3 | file 도메인 spawn_blocking 0건 | **부분 해소.** `file/commands.rs:30`(open)·`:46`(save)·`:150`(mirror_dirty)·copy 4종만 이관. `file_create`/`file_rename`/`file_delete` 는 async 워커 잔존(계약 §5 명시) |
| H-4 | 트리 전역 락 하 동기 read_dir + 전체 재직렬화 | **절반 해소.** 락 밖 선행 read(`tree/commands.rs:127 prefetch_listings`)·창 밖 행 무할당(`tree/service.rs:305 push_page_rows`)·정렬 키 사전 계산(`service.rs:40`)은 완료. **응답 형태는 그대로** — `tree/commands.rs:172·192·212` 셋 다 `Ok(service::full_page(tree))` |
| M-1 | git 조회 7종 spawn_blocking | **해소.** 예: `git/commands.rs:293` gutter, `:311` blame_range |
| M-2 | git_status 캐시·rename 방향 | **미해소(백로그).** `git/service.rs:109 status()` 는 매 호출 `open_repo` + `collect_status_rows` 전량 재계산 |
| M-3 | 에이전트 500ms `ps` fork | **부분 해소.** pid 일괄 배칭(`agent/commands.rs:216-222`)·세션 0 조기 반환(`:306`). **프로젝트당 1회 fork 는 잔존** |
| M-4 | 리터럴 검색 라인당 String 2개 | **해소** (`LiteralMatcher` 무할당 경로) |
| M-5 | 터미널 링버퍼 `Vec::drain` | **해소.** `terminal/service.rs:15 ScrollbackRing` = `VecDeque` |
| M-6 | `pty_write` 블로킹 쓰기 | **해소.** `terminal/commands.rs:364 spawn_blocking` |
| M-7 | `run_git` 무타임아웃 | **해소.** `GIT_COMMAND_TIMEOUT_SECS`(300) + 파이프 드레인 5초 |
| M-8 | `opt-level = "s"` | **해소.** `Cargo.toml:9 opt-level = 3` |
| M-9 | 검색 매치 1건당 Channel 1건 | **해소.** 파일 단위 `SearchFileMatches` 배칭 |
| M-10 | 워처 FileIdMap 전체 트리 워크 | **미해소(백로그, "난이도 L·실수요 확인 후").** → **§2-A 에서 최우선으로 재평가** |
| L-1 | git_log refs 재구축 + O(skip) | **미해소.** `git/service.rs:547 collect_refs_by_oid(&repo)` 매 페이지 + `:550 walk.skip(skip)` |
| L-2 | pty flusher 5ms 상시 wakeup | **해소.** `infra/pty.rs:161 FlushSignal` condvar |
| L-3 | LSP 50ms 폴링 + drain memmove | **해소.** `infra/lsp_proc.rs:158 MessageBuffer` 커서 + 조건부 `compact` |
| L-4 | `ide:diff-requested` 가 파일 전문 운반 | **미해소.** `events.rs:162 pub new_contents: String` |
| L-5 | `search_list_files` 무상한 | **의도적 존치**(d-42 계약). 캐시는 백로그 |

**요약: 19건 중 13건 해소, 3건 부분(H-3·H-4·M-3), 3건 미착수(M-2·M-10·L-1) + L-4·L-5 백로그.**

---

## 2. 신규 발견 (d-50·백로그에 없는 것)

### A. 부팅·프로젝트 전환 — 워처 attach 가 전역 mutation guard 를 수 초간 점유 (최대 영향)

`project_open` 은 `begin_mutation` 을 잡은 채 `attach_all` 을 **동기 호출**합니다.

- `domain/project/commands.rs:69` `let _guard = state.begin_mutation().await;` → `:81` `attach_all(&app, &state, &result.project)`
- → `domain/file/capability.rs:36` `watcher::start_watch(...)` → `infra/watcher.rs:78` `debouncer.watch(&root, RecursiveMode::Recursive)`
- → cargo registry `notify-debouncer-full-0.7.0/src/lib.rs:589` `data.cache.add_path(&path, recursive_mode)` (debouncer 자신의 `data.lock()` 보유 중)
- → `notify-debouncer-full-0.7.0/src/file_id_map.rs:42-53` `WalkDir::new(path).follow_links(true).max_depth(usize::MAX)` + **엔트리마다 `get_file_id`(stat)** — **무시 목록 미적용**

**이 저장소 실측**: `find . -type f | wc -l` = **512,912** vs 무시 목록 적용 시 **1,202**. 즉 워크 대상이 **427배**입니다. warm 캐시에서도 stat 워크만 ~2.5s 소요(`time find . -type f -exec stat -f '%i' {} +` 실측 2.524s). d-25 계약이 기록한 "RESUME 15,320파일 ~0.5-1초"의 33배 규모입니다.

부팅 경로는 d-25(`2026-08-20-boot-watcher-defer-contract.md`)에서 후절화됐지만, 같은 계약 §1 이 **"`project_open`(신규 열기) 경로의 동기 attach 는 무변경"** 으로 명시 제외했습니다. `docs/architecture.md` §2.1 의 "AppState 접근은 짧은 잠금 원칙: 잠금 안에서 IO 금지" 와 정면 충돌하며, `state.rs:63-105`(`begin_mutation`)이 앱 전역 단일 `tokio::sync::Mutex` 이므로 **프로젝트를 여는 동안 file_save·git_*·layout_*·tree_toggle 전부가 정지**합니다.

### B. 워처 FileIdMap 의 메모리 비용 (M-10 의 미기록 절반)

`FileIdMap.paths: HashMap<PathBuf, FileId>` 에 512,912 엔트리 → PathBuf(평균 경로 100B 내외) + `FileId` + HashMap 오버헤드로 **프로젝트당 대략 60~90MB 상주**로 추정됩니다. 백로그의 M-10 은 "워크 비용"만 적고 메모리 축은 없습니다. 프로젝트 3개를 열면 200MB 이상이 워처 캐시만으로 소비됩니다(사용자 결정 §2 의 "메모리" 지표에 직결).

**설계 실현 가능성 확인**: `notify_debouncer_full` 는 `pub use cache::{FileIdCache, NoCache, RecommendedCache};`(`lib.rs:86`)·`pub use file_id;`(`:91`)·`new_debouncer_opt<F, T, C: FileIdCache + Send + 'static>`(`:639`)을 노출하므로, **신규 의존성 없이** 커스텀 캐시를 주입할 수 있습니다. `infra/watcher.rs:9` 가 이미 `crate::constants::is_ignored_dir` 를 import 하고 있어 계층 위반도 없습니다.

**degradation 모델(소스로 확정)**: `lib.rs:389-398 handle_rename_to` 는 `file_ids_match` 가 false 면 rename 을 잇지 않고 "move in"(별도 이벤트)으로 밀어냅니다. 즉 필터된 캐시에서 잃는 것은 **무시 디렉토리 경계를 넘는 이동**뿐이고, 그 경우에도 TAIDE 의 `map_event_kind`(`infra/watcher.rs:131`)는 양쪽을 `Renamed` 로 매핑해 소비자(`ipc-sync-provider` → `tree_refresh(parentDir)`)가 두 디렉토리를 각각 갱신합니다. d-35 가 기각한 것은 **전면 `NoCache`**(모든 rename 파손)이고, 필터 캐시는 그와 다른 성질입니다.

### C. git_status 가 상시 핫패스로 승격됨 (M-2 우선순위 상향 근거)

`0c60cb7 feat(explorer): 파일트리 git 상태 데코레이션` 이후 `gitStatusQueryOptions` 소비처가 **5곳으로 상시 마운트**됐습니다: `widgets/explorer/explorer-container.tsx:59`, `widgets/window-chrome/title-bar-content.tsx:12`, `widgets/editor-pane/use-editor-git-gutter-and-conflicts.ts:59`, `widgets/git-panel/git-panel-container.tsx:76`, `widgets/auxiliary-window-shell/auxiliary-title-bar-content.tsx:24`. 무효화는 `.git/index` 변경(`git/watch.rs:31`)과 `fs:changed` 축(d-44, `ipc-sync-provider.tsx:344`) 양쪽에서 옵니다.

즉 **저장 한 번마다 전 저장소 status 재계산**입니다. 백로그의 "대량 dirty 실측 후" 조건은 이 변경으로 이미 충족됐다고 봅니다.

추가로 libgit2 소스 확인 결과, 현재 `StatusOptions` 는 `NO_REFRESH`/`UPDATE_INDEX` 를 둘 다 끄고 있습니다(`git/service.rs:1788-1796`):
- `libgit2-sys-0.18.7+1.9.6/libgit2/src/libgit2/status.c:296-298` — `NO_REFRESH` 가 없으므로 매 호출 `git_index_read_safely(index)`
- `status.c:315` — `UPDATE_INDEX` 플래그가 없으므로 `GIT_DIFF_UPDATE_INDEX` 미설정 → **stat-stale 엔트리를 매번 재해시**하고 결과를 인덱스에 되쓰지 않음(= 다음 호출도 같은 비용)
- `renames_head_to_index(true)` + `renames_index_to_workdir(true)`(`service.rs:1795-1796`) → 변경 파일 수에 대해 양방향 rename 탐지

### D. `flush_dirty_layouts` 가 async 워커에서 fsync (architecture.md §2.1 위반)

`lib.rs:611-616` 이 `tauri::async_runtime::spawn` 한 2초 주기 태스크에서 `domain::layout::service::flush_dirty_layouts` 를 직접 호출하고, 그 안에서 `save_layout`(`layout/service.rs:1121-1124`) → `persist::write_json`(`infra/persist.rs:95`) → `write_atomic`(`:26 file.sync_all()`)로 **동기 fsync** 가 일어납니다. 프로젝트 N개가 dirty 면 fsync N회를 async 워커 하나가 직렬로 지불합니다. `spawn_blocking` 이관 대상입니다.

### E. `list_themes` 가 호출마다 번들 테마 1.1MB 재파싱

`domain/theme/service.rs:757-763 bundled_themes()` 가 `BUNDLED_THEME_SOURCES` 38종을 `serde_json::from_str::<Theme>` 로 **전량 역직렬화**한 뒤 `summarize`(id/name/type 3필드)만 뽑습니다(`:906`). `du -sh src-tauri/resources/themes` = **1.1M**. 그리고 `THEME.ALL` 은 `SettingsChanged`(`ipc-sync-provider.tsx:296`)와 `ThemeChanged`(`:231`) 양쪽에서 무효화되므로, **설정 화면이 열린 상태의 모든 설정 토글이 1.1MB 재파싱을 유발**합니다. `OnceLock<Vec<ThemeSummary>>` 한 줄로 해소됩니다.

### F. `tree::read_children` 의 엔트리당 여분 syscall

`tree/service.rs:48-84`:
- `item.metadata()`(`:51`) — 엔트리마다 stat 1회. macOS 의 `readdir` 은 `d_type` 을 주므로 `item.file_type()` 이면 대개 syscall 0회입니다(**주의: rust-src 컴포넌트가 설치돼 있지 않아 std 소스로 확증하지 못했습니다 — §4 의 `fs_usage` 로 실측 검증 필요**).
- `directory_has_children(&path)`(`:32`) — **하위 디렉토리마다 `read_dir` 1회 추가**(open+getdirentries+close). 디렉토리가 D개면 D회.

### G. 파일 열기 tier 경로의 이중 읽기

`file/service.rs:90` `is_binary(&read_sniff(path)?)` 로 앞 8KB 를 읽은 뒤 `:94` `std::fs::read(path)` 로 **파일을 처음부터 다시 전량 읽습니다**. 검색 경로는 이미 `read_scannable_bytes`(`search/service.rs:260`)에서 sniff 후 이어읽기로 고쳤는데, file 도메인은 그 패턴이 적용되지 않았습니다. 20MB 파일이면 8KB 를 두 번 읽는 정도라 비용 자체는 작지만, `line_count = content.lines().count()`(`:95`)가 전체를 한 번 더 순회합니다.

### H. IPC 직렬화 크기 (specta 타입)

- `tree_rows` 는 프론트가 항상 `limit: null` 을 보냅니다(`src/entities/tree/tree.query.ts:21`, 테스트가 계약으로 고정: `tree.query.test.ts:18`). `tree_toggle`/`tree_reveal`/`tree_refresh` 도 `full_page` 반환(`tree/commands.rs:172·192·212`). `docs/ipc-contract.md:234` 가 "응답 형태(전체 트리 재직렬화)는 이번 배치에서 **불변**" 으로 명시하고 있습니다 → 계약 변경 필요.
- `search_list_files` 는 프로젝트 전 파일 경로 `Vec<String>` 을 반환(`search/commands.rs:226-234`). 이 저장소 기준 1,202개지만 대형 모노레포에서는 수만 개 × 평균 60자 = 수 MB JSON 이며, `fs:changed` 의 create/rename/delete 마다 무효화됩니다(`ipc-sync-provider.tsx:362`).
- `IdeDiffRequested.new_contents: String`(`events.rs:162`) — `architecture.md` §6 규칙 4("대형 데이터는 이벤트에 싣지 않는다")의 유일한 위반. 원격 팬아웃(`lib.rs` `fanout_remote_events!` 목록에 `IdeDiffRequested` 포함)까지 타면 WS 로도 전문이 나갑니다.

### I. 잠금·스레드풀 경합

- **단일 전역 `mutation_guard`**(`state.rs:40`, `:63`). 커맨드 파일별 취득 횟수: git 36 · file 13 · project 9 · lsp 8 · sync 6 · layout 5 · tree 5 · terminal 4 · search 4 · window 3 · settings 3 · plugin 4 · vsix 2 · app 2 · agent 1 · ide 1. 도메인 무관 커맨드가 전부 한 줄에 섭니다. `state.rs:67-105` 의 doc 이 이미 "guard 보유 중 `spawn_blocking` 대기 → 풀 고갈 자기지속" 위험을 스스로 기록하고 있습니다.
- `pty_write` 는 **키 입력 1회당 invoke 1회 + `spawn_blocking` 1회**(`terminal/commands.rs:358-372`)이고, 자식이 stdin 을 안 읽으면 **상한 없이 풀 스레드를 점유**합니다(같은 doc 이 명시). 세션별 상주 writer 스레드 + mpsc 로 바꾸면 풀 점유와 상한 없음이 동시에 사라집니다.
- `lsp_send` 는 LSP 메시지 1건당 invoke 1회(`lsp/commands.rs:618`). guard 는 안 잡지만(doc 이 명시) `didChange`·완성·시맨틱 토큰 빈도만큼 IPC JSON 왕복이 발생합니다.
- 대부분의 file/project 커맨드가 `state.projects.read().clone()` 으로 **프로젝트 맵 전체를 복제**합니다(`file/commands.rs:24·58·76·146·161·171`, `project/commands.rs:70-71·111-112·139-140`). 프로젝트 수가 한 자릿수라 비용은 작지만 `file_mirror_dirty` 는 500ms 주기라 누적됩니다.

### J. 원격 dispatch

`domain/remote/dispatch.rs:203` `serde_json::to_string(&value)` 로 커맨드 결과를 다시 문자열화합니다(데스크톱 경로의 specta 직렬화와 별개 경로). 이벤트 팬아웃은 `lib.rs:516-519` 에서 `remote.has_event_subscribers()` 조기 반환이 있어 구독자 0일 때 비용이 없습니다 — 이 축은 양호합니다.

### K. 부팅 순서 (lib.rs setup) — 나머지

`lib.rs:462-620` 의 창 생성 전 동기 구간:
1. `:470` `warm_builtin_catalogs()` — 로케일 3종(en 60,062B + ja 74,015B + ko 68,186B = **약 200KB**) `serde_json::from_str` 동기 파싱. `locale/service.rs:1106-1116` 의 doc 이 "부팅 시 큰 소리로 죽게 하려는 의도적 선택" 이라 밝히고 있어 **제거 대상이 아닙니다**(계측 대상일 뿐).
2. `:477` `restore_state` — 프로젝트 수만큼 `load_layout` 동기 read (`project/commands.rs:174-181`).
3. `:487-503` `app.manage(...)` 19개 — 전부 `Default::default()`, 비용 없음.
4. `:504` `restore_auxiliary_windows`, `:505` `restore_project_watchers`(후절화 완료), `:506` `refresh_password_configured_cache`.
5. `:619` `log::info!("setup 완료: elapsed_ms={}")` — **이미 존재하는 유일한 계측 지점**.

---

## 3. Rust 테스트 공백 맵 (상위 20 — `#[test]` 밀도 오름차순, 60줄 초과 모듈)

측정: `grep -c '#\[test\]'` / `wc -l`. 통합 테스트는 `src-tauri/tests/{domain_boundaries.rs(288L), session_restore.rs(261L)}` 2개뿐입니다.

| 순위 | 모듈 | 줄수 | 테스트 | /100L | 비고 |
|---|---|---|---|---|---|
| 1 | `domain/file/commands.rs` | 253 | **0** | 0 | 데이터 파괴 이력 3건(d-50 버그1·2·3)의 진입점 |
| 2 | `domain/ide/commands.rs` | 259 | **0** | 0 | Claude Code 프로토콜·lockfile 수명 |
| 3 | `domain/window/commands.rs` | 314 | **0** | 0 | 보조 창 수명·핫엑시트 핸드셰이크 |
| 4 | `domain/settings/commands.rs` | 92 | **0** | 0 | `apply_and_broadcast` 재진입 교착 계약 |
| 5 | `domain/plugin/commands.rs` | 92 | **0** | 0 | |
| 6 | `infra/archive.rs` | 124 | **0** | 0 | tar/zip/xz 해제(zip-slip 표면) |
| 7 | `domain/remote/serving.rs` | 269 | **0** | 0 | Range·파일 서빙(보안 표면) |
| 8 | `domain/lsp/types.rs` | 294 | **0** | 0 | 매니페스트 상수 |
| 9 | `domain/git/types.rs` | 214 | **0** | 0 | |
| 10 | `events.rs` | 206 | **0** | 0 | 페이로드 계약(lib.rs 파리티 테스트가 이름만 강제) |
| 11 | `domain/git/commands.rs` | 1063 | 5 | 0.47 | 36개 guard 취득·spawn_blocking 합성 |
| 12 | `domain/agent/commands.rs` | 816 | 5 | 0.61 | 500ms 폴링 루프 |
| 13 | `domain/project/commands.rs` | 529 | 4 | 0.75 | open/close/activate — 본 조사 최대 병목 |
| 14 | `domain/lsp/commands.rs` | 1602 | 15 | 0.93 | 세션 재사용·generation·재초기화 |
| 15 | `domain/tree/commands.rs` | 382 | 4 | 1.04 | prefetch/락 분리 |
| 16 | `domain/locale/service.rs` | 1505 | 16 | 1.06 | |
| 17 | `domain/layout/commands.rs` | 614 | 7 | 1.14 | |
| 18 | `domain/terminal/commands.rs` | 776 | 11 | 1.41 | attach 원자성·flow control |
| 19 | `domain/vsix/service.rs` | 1052 | 18 | 1.71 | |
| 20 | `domain/search/commands.rs` | 375 | 8 | 2.13 | |

**구조적 원인**: `commands.rs` 는 `tauri::State`/`AppHandle` 을 받아 테스트에서 구성할 수 없습니다(`domain/terminal/commands.rs` 의 자체 doc 이 "이 코드베이스에는 `tauri::test` mock-app 하네스가 없다"고 기록). 그래서 로직이 `service.rs` 에 있는 도메인은 밀도가 높고(git 2.41 · layout 2.36 · file 2.42), 로직이 `commands.rs` 에 남은 도메인(project·window·ide·file)이 그대로 공백입니다.

---

## 4. 지표 후보와 측정 방법 (criterion 없이 · 신규 의존성 0)

### 4.1 8지표 ↔ Rust 측 프로브 지점

| 지표 | Rust 프로브 | 현재 존재 여부 |
|---|---|---|
| 부팅 → 첫 페인트 | `lib.rs:463 setup_started` ~ `:619`, 단계별 분할(warm/restore_state/manage/restore_watchers) + FE 의 first-paint mark 합산 | setup 총합만 존재 |
| 프로젝트 전환 | `project_open` 전체 · `attach_all` 단독 · `begin_mutation` 대기 시간 | 없음 |
| 파일 열기(소/대) | `file_open` 전체 · `open_file` 내 read/decode/lines/editorconfig 분할 | 없음 |
| 팔레트 열기/입력 | `search_list_files` 전체 · `list_project_files` walk · 결과 직렬화 바이트 | 없음 |
| 트리 펼침 | `tree_toggle` 전체 · `prefetch_listings` · `full_page` · 응답 바이트 | 없음 |
| git status | `git_status` 전체 · `collect_status_rows` · 행 수 | 없음 |
| 전역 검색 | `search_run` 전체 · 파일 수 · 매치 수 · 첫 배치까지 시간(TTFB) | 없음 |
| 터미널 처리량 | `infra/pty.rs` 리더 스레드의 바이트/초 · flush 횟수 · 배치 크기 분포 | 없음 |
| (+) 메모리 | 워처 IdCache 엔트리 수 · 트리 캐시 엔트리 수 · 링버퍼 총합 | 없음 |

### 4.2 측정 수단 (권장 순)

1. **`infra::perf` 인앱 레지스트리 (주 수단)** — `parking_lot::Mutex<HashMap<&'static str, Stat{count,total_ns,min,max,버킷}>>` 를 `app.manage`, RAII `perf::span("git_status")` 로 기록, `perf_snapshot()`/`perf_reset()` 커맨드로 조회. 기록은 `AtomicBool` 게이트 뒤(설정 또는 `TAIDE_PERF=1`)에 두어 off 시 원자적 load 1회만 지불. **신규 의존성 0**(`std::time::Instant` + 기존 `parking_lot`).
2. **커맨드 호출 카운터 (공짜, 전 커맨드 자동)** — `lib.rs:455-461` 의 invoke 디스패치 클로저가 이미 `invoke.message.command()` 를 봅니다. 여기서 이름별 카운터만 올리면 **커맨드 본문을 한 줄도 안 고치고** "무엇이 몇 번 불리는가"(N+1 IPC 탐지)를 얻습니다. 지속시간까지는 안 되는데, `tauri-2.11.5/src/ipc/mod.rs:307 InvokeResolver::new` 이 `pub(crate)` 라 리졸버 래핑이 불가능하고 `tauri-specta-2.0.0-rc.25/src/builder.rs` 에도 미들웨어 API 가 없기 때문입니다(확인 완료). 그래서 지속시간은 §1 의 span 방식뿐입니다.
3. **로그 타이머** — 이미 쓰는 `log::info!("... elapsed_ms=...")` 패턴(`lib.rs:619`). 산출물은 `~/Library/Logs/net.gumyo.taide/TAIDE.log`(`docs/debugging.md:15`). 40KB 회전이라 고빈도 지표에는 부적합, 부팅·프로젝트 전환 같은 저빈도 1회성에만 사용.
4. **결정론적 회귀 테스트 (CI 안전)** — 시간이 아니라 **작업량**을 고정합니다. 합성 픽스처 트리에 대해 `serde_json::to_vec(&full_page(&tree)).len()` 상한, `read_children` 호출 시 발생한 `read_dir` 횟수(테스트용 카운터), `collect_status_rows` 의 행 수를 assert. 벽시계 없이 회귀를 잡는 유일한 CI-안전 수단입니다.
5. **로컬 마이크로 벤치 (`#[ignore]` 테스트)** — `#[test] #[ignore] fn perf_tree_flatten_50k()` 처럼 두고 `cargo test --release -- --ignored --nocapture` 로만 실행. **CI 게이트로 삼지 않습니다**(러너 노이즈). 현재 `#[ignore]` 테스트는 0개라 이 관례를 새로 세우게 됩니다.
6. **macOS 외부 도구** — `fs_usage -w -f filesys -p <pid>`(§2-F 의 syscall 가설 확증), `/usr/bin/time -l`(max RSS, §2-B 확증), `sample <pid> 10`·Instruments Time Profiler. **단 `Cargo.toml:8 strip = true` 때문에 릴리스 바이너리에 심볼이 없으므로**, 프로파일링용으로 `[profile.profiling] inherits = "release", strip = false, debug = 1` 프로필을 별도로 추가해야 합니다.
7. **빌드 시간** — `cargo build --timings`(HTML 산출). 프로필 결정은 이미 끝났으므로 회귀 감시용.

---

## 5. 개선 후보 우선순위 (영향 · 난이도 · 회귀 위험 · 추상화 지점)

| # | 항목 | 영향 | 난이도 | 회귀 위험 | 추상화 지점 |
|---|---|---|---|---|---|
| 1 | `project_open` 워처 attach 를 guard 밖 후절화 (§2-A) | **특대** (프로젝트 전환 중 전 뮤테이션 정지 제거) | 중 | 중 | **이미 존재**: `build_watcher_handle` / `register_watcher_handle` 분리(`file/capability.rs:33·65`). `restore_project_watchers`(`project/commands.rs:325`)의 "밖에서 build, 안에서 register + 재검증" 패턴을 `ProjectCapability` 에 `attach_deferred` 로 승격 |
| 2 | 워처 IdCache 필터링 (M-10, §2-B) | **특대** (워크 427배↓, 프로젝트당 60~90MB↓) | 중 | 중 (교차 경계 rename → 2이벤트 저하) | `infra::watcher::ScopedIdCache: FileIdCache` + `new_debouncer_opt::<_, RecommendedWatcher, ScopedIdCache>`. `WatchScope` 별 정책(GitDir 는 무필터). Linux 는 `cfg` 로 `NoCache` 유지 |
| 3 | `git_status` 결과 캐시 + 워처 무효화 (M-2, §2-C) | **대** (상시 5소비처 · 저장마다 전량 재계산) | 중 | 중 (stale 창) | `GitStore` 에 `status_cache: Mutex<HashMap<ProjectId, CachedStatus>>`, 무효화 훅은 `git/watch.rs:24 classify_git_change` + `AppState::self_writes`. `StatusOptions` 의 `update_index` 채택은 §7 질문 |
| 4 | `list_themes` 번들 요약 `OnceLock` (§2-E) | 중 | **소** | **낮음** | `theme/service.rs` 에 `fn bundled_summaries() -> &'static [ThemeSummary]` 추가, `bundled_themes()` 는 유지 |
| 5 | `flush_dirty_layouts` → `spawn_blocking` (§2-D) | 중 | **소** | **낮음** | `lib.rs:611` 태스크 내부만 변경. `layout::service` 표면 불변 |
| 6 | `read_children`: `metadata()`→`file_type()`, `has_children` 지연화 (§2-F) | 중 | 소 | 낮음 (`file_type` 은 요검증) | `Entry.has_children: Option<bool>` 로 확장 후 `TreeRow` 직렬화 시점에 해석 — IPC 타입 불변 |
| 7 | `search_list_files` FsChanged 캐시 (백로그) | 중 | 중 | 낮음 | `SearchStore` 에 `file_index: RwLock<HashMap<ProjectId, Arc<Vec<String>>>>` + 신규 `search::capability` 로 `project_close` 정리(§6.3 표에 행 추가). d-42 의 "무상한" 계약은 불변 |
| 8 | 트리 mutation 응답 재설계 (H-4 후반) | **대** | **대** | **대** (IPC 계약 + FE 동시 개정, `ipc-contract.md:234` 개정) | `TreeRowPage` 에 `revision: u64` 추가 → mutation 은 revision 만 반환, FE 는 기존 `tree_rows(offset, limit)` 로 필요한 창만 재조회. 페이지네이션 표면이 이미 있어 신규 커맨드 불요 |
| 9 | `pty_write` 세션별 상주 writer 태스크 | 중 (풀 고갈 위험 제거) | 중 | 중 (쓰기 순서 계약 — d-50 §4(7)) | `infra::pty::PtySession` 에 `write_tx: mpsc::Sender<Vec<u8>>` + 전용 스레드. `entities/terminal/session-write-order.ts` 의 FE 직렬화와 계약 정합 필요 |
| 10 | `git_log` refs 캐시 + 커서 재개 (L-1) | 소~중 (태그 수천 규모) | 중 | 중 | `GitStore` 에 refs 캐시(3번과 같은 무효화 훅 재사용), `LogEntry` 페이징을 skip → oid 커서로 |
| 11 | `file_open` sniff 후 이어읽기 + line_count 통합 (§2-G) | 소 | 소 | 낮음 | `search::service::read_scannable_bytes` 와 동형 함수를 `infra` 로 승격해 두 도메인 공유 |
| 12 | `poll_agents` `ps` fork → `sysinfo` | 소 | 중 | **중** (`ps -o state` 문자 → `ProcessStatus` 의미 변화) | `service::ProcessInfo` 를 계약면으로 두고 공급자만 교체. macOS 의 thread status 취득 가능성은 `sysinfo-0.39.6/src/unix/apple/macos/process.rs:168` 확인 — 다만 실측 선행 필요 |
| 13 | `ide:diff-requested` pull 전환 (L-4) | 소 (성능) / 중 (계약 정합) | 중 | 중 | `events.rs:162 new_contents` 제거 + `ide_take_diff_payload(request_id)` 조회 커맨드. 원격 팬아웃 페이로드도 함께 줄어듦 |
| 14 | 전역 `mutation_guard` 분할 (§2-I) | **특대** (이론상) | **특대** | **특대** | **이번 배치 비권장.** 1·3번으로 보유 시간을 줄이는 것이 먼저이고, 분할은 `state.rs:67-105` doc 이 기술한 교착 여유 근거 전체를 다시 세워야 합니다 |

---

## 6. 계측 대상 매핑 (권장안 구현 시)

`perf::span` 을 넣을 최소 집합 15곳: `lib.rs`(setup 4단계) · `project_open`/`attach_all`/`begin_mutation` 대기 · `file_open`(read/decode 분할) · `file_save` · `tree_rows`/`tree_toggle`(prefetch/full_page/직렬화 바이트) · `git_status`(collect_status_rows/행 수) · `git_gutter` · `search_run`(TTFB/파일 수/매치 수) · `search_list_files`(walk/바이트) · `pty` 리더(바이트·flush 횟수) · 워처 attach(엔트리 수·소요) · `lsp_send` 카운터 · `list_themes`.


## options
## 안 A — 계측 우선 + 무결정 저위험 4건 동시 (추천)

**범위**: ①`infra::perf` 레지스트리 + 커맨드 호출 카운터 + 15개 span 내장 → ②그 계측이 켜진 상태에서 저위험·무계약변경 4건(우선순위 4·5·6·11)을 같은 배치에 태워 "계측 → 개선 → 재측정"의 첫 왕복을 한 번에 완결.

**변경 파일**
- 신규 `src-tauri/src/infra/perf.rs`(레지스트리·`Span` RAII·게이트), `infra/mod.rs` 등록
- `src-tauri/src/lib.rs` — `app.manage(PerfRegistry)`, 디스패치 클로저에 이름별 카운터, setup 4단계 span, 커맨드 2종 등록
- `src-tauri/src/domain/app/{commands,types}.rs` — `perf_snapshot`/`perf_reset` (app 도메인이 앱 메타 정보 소유자)
- `theme/service.rs`(요약 OnceLock), `lib.rs`+`layout/service.rs`(flush spawn_blocking), `tree/service.rs`(file_type·has_children), `file/service.rs`+새 `infra` 공유 리더(sniff 이어읽기)
- `src/shared/api/bindings.ts`(생성물) + FE 는 개발용 스냅샷 표시만

**데이터 흐름**: 커맨드 본문 → `perf::span(name)` → drop 시 `PerfRegistry.record(name, ns)` → `perf_snapshot()` → FE 개발 패널/콘솔. 게이트가 off 면 `AtomicBool::load` 1회.

**장점**: 병목 순위를 **추측이 아니라 실측**으로 확정한 뒤 큰 수술에 들어감. 4건은 전부 IPC 계약·동시성 의미 불변이라 회귀 위험이 사실상 0. 사용자 결정 §2("8지표 계측 내장")를 문자 그대로 이행.
**단점**: 이 배치만으로는 최대 병목(§2-A·B·C)이 그대로 남음 — 체감 개선이 §2-E/D 수준에 그침.

---

## 안 B — 최대 병목 3건 직행 (계측은 최소)

**범위**: 우선순위 1·2·3(project_open 후절화 · IdCache 필터 · git_status 캐시)만. 계측은 그 3건 전후 비교에 필요한 span 5개로 축소.

**변경 파일**
- `domain/project/commands.rs`·`domain/project/capability.rs`·`domain/file/capability.rs`·`domain/git/capability.rs` — `attach` 를 build/register 2단으로 나누고 guard 밖 build
- `infra/watcher.rs` — `ScopedIdCache: FileIdCache` 신설, `new_debouncer_opt` 전환, `WatcherHandle` 타입 파라미터 변경
- `domain/git/{commands,service,watch}.rs` — status 캐시 + 무효화 훅
- `docs/architecture.md` §6.3(회수 목록에 캐시 행) · `docs/ipc-contract.md`(표면 불변 명시)

**데이터 흐름**: `project_open` → capability 순회가 build 만 수행(락 없음) → guard 재획득 → register. 워처 이벤트 → `classify_git_change` → status 캐시 무효화 → 다음 `git_status` 만 재계산.

**장점**: 프로젝트 전환 체감이 초 단위로 개선되고 메모리가 프로젝트당 수십 MB 줄어듦. 개선 폭이 가장 큼.
**단점**: 세 건 모두 **동시성·정확성 의미를 바꿉니다**. 계측 없이 들어가면 "얼마나 좋아졌는지"를 사용자에게 숫자로 보고할 수 없고, 회귀가 나도 원인 분리가 어렵습니다. `attach_all` 의 등록 순서 계약(`architecture.md` §3·§6.3, lib.rs 소스 스캔 테스트가 핀 고정)을 건드리는 위험도 있습니다.

---

## 안 C — 안 A 완결 후 안 B 를 별도 배치로 (2단 분리)

**범위**: 배치 1 = 안 A 전량. 배치 2 = 안 B 3건 + 우선순위 7·9·10, 배치 1 의 실측 순위에 따라 재정렬.

**장점**: 각 배치가 독립 검증 가능하고, 배치 2의 착수 순서를 실측 데이터로 정할 수 있음. 큰 동시성 변경이 계측이 켜진 트리 위에서 이뤄짐.
**단점**: 사용자 체감 개선이 배치 2까지 미뤄짐. 배치가 하나 늘어남.


## recommendation
**안 C(2단 분리)를 추천하되, 배치 1과 2를 같은 웨이브에서 연속 실행**하는 형태를 권합니다. 즉 실질적으로는 "안 A 를 먼저 머지 가능한 상태로 완결 → 그 계측으로 순위를 확정 → 안 B 를 두 번째 스테이지로" 입니다.

근거 세 가지입니다.

1. **사용자 결정이 이미 계측 선행을 지정했습니다.** `2026-09-04-usability-batch4-user-decisions.md` §2 는 "8지표를 앱에 계측 코드로 내장해 **기준선을 재고, 병목 상위부터 반복 개선**" 이라고 순서를 못박고 있습니다. 안 B 직행은 이 결정과 어긋납니다.

2. **최대 병목 3건은 전부 동시성·정확성 의미를 바꿉니다.** `project_open` 후절화는 `attach_all` 등록 순서 계약(`architecture.md` §3·§6.3 + lib.rs 소스 스캔 테스트)을 건드리고, IdCache 필터는 rename 스티칭 동작을 바꾸며(d-35 가 이미 한 번 기각한 축), git status 캐시는 stale 창을 새로 만듭니다. 이 셋을 계측 없이 동시에 넣으면 회귀 시 원인 분리가 불가능합니다.

3. **안 A 의 4건은 그 자체로 즉시 이득이면서 회귀 위험이 0에 가깝습니다.** 특히 `list_themes` 재파싱(설정 토글마다 1.1MB)과 `flush_dirty_layouts` 의 async 워커 fsync 는 한 줄~수 줄 수정으로 끝나고 IPC 계약을 전혀 건드리지 않습니다. 계측 배치에 얹기 딱 좋습니다.

배치 2 착수 순서는 **1(project_open 후절화) → 2(IdCache 필터) → 3(git_status 캐시)** 를 권합니다. 1은 표면 변경 없이 락 보유 시간만 줄이는 가장 안전한 큰 개선이고, 2는 1이 없으면 이득의 절반을 부팅 경로에서만 얻습니다. 3은 무효화 훅 설계 검토가 필요해 마지막입니다.

**우선순위 8(트리 응답 재설계)과 14(전역 guard 분할)는 이번 웨이브에서 제외**를 권합니다. 8은 `docs/ipc-contract.md:234` 가 명시적으로 "불변" 으로 고정한 계약이고 FE 동시 개정이 필요하며, 14는 `state.rs` doc 이 세워 둔 교착 여유 근거 전체를 다시 세우는 규모입니다. 둘 다 실측 결과가 이를 요구할 때 별도 배치로 다루는 것이 맞습니다.

**테스트 보강은 계측 배치에 반드시 동반**해야 합니다(사용자 요청 3의 "안정성을 위한 더 세세하고 많은 테스트"). §3 공백 맵의 1~3위(`file/commands.rs`·`ide/commands.rs`·`window/commands.rs`, 전부 0건)는 `tauri::State` 의존이 원인이므로, 새 `infra::perf` 를 만들 때 **순수 함수를 `service` 로 밀어내는 기존 패턴**(git/layout/file service 의 밀도 2.4가 그 증거)을 그대로 적용해 커맨드는 얇게, 로직은 테스트 가능한 곳으로 옮기는 리팩터를 함께 넣는 것을 권합니다.

## filesToTouch
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/infra/perf.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/infra/mod.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/lib.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/app/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/app/types.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/theme/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/layout/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/tree/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/tree/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/file/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/file/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/file/capability.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/project/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/project/capability.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/git/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/git/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/git/watch.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/git/capability.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/search/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/search/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/infra/watcher.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/tests/domain_boundaries.rs
- /Users/hyunseokbyun/development/TAIDE/Cargo.toml
- /Users/hyunseokbyun/development/TAIDE/src/shared/api/bindings.ts
- /Users/hyunseokbyun/development/TAIDE/docs/architecture.md
- /Users/hyunseokbyun/development/TAIDE/docs/ipc-contract.md
- /Users/hyunseokbyun/development/TAIDE/docs/backlog.md
- /Users/hyunseokbyun/development/TAIDE/docs/PROCESS.md
- /Users/hyunseokbyun/development/TAIDE/docs/acknowledge/2026-09-04-usability-batch4-contract.md
- /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-08-29-full-audit.md

## risks
- 계측 오버헤드가 측정 대상을 왜곡: `perf::span` 이 고빈도 경로(pty 리더 루프, `lsp_send`)에 들어가면 `Instant::now()`(macOS 는 `mach_absolute_time`) 비용이 측정값에 섞인다. 고빈도 경로는 개별 지속시간이 아니라 누적 바이트/횟수 카운터만 두고, 지속시간 span 은 커맨드 경계(수십~수백 Hz 이하)로 한정해야 한다.
- 계측 레지스트리 자체가 새 잠금 경합: `Mutex<HashMap>` 를 전역으로 두면 병렬 검색 워커 N개가 같은 락을 두드린다. 기록 슬롯을 `&'static str` 인덱스 고정 배열 + `AtomicU64` 로 만들거나, 게이트 off 를 기본값으로 두어 릴리스 기본 경로에서 락에 닿지 않게 해야 한다.
- `project_open` 후절화가 `attach_all` 등록 순서 계약을 깬다: `docs/architecture.md` §3 이 "등록 순서가 곧 close 의 자원 회수 순서 계약" 이라 명시하고 lib.rs 소스 스캔 테스트가 순서를 핀 고정한다. build/register 2단 분리 시 detach 대칭성과 순서 보장이 함께 재검증돼야 한다.
- `project_open` 후절화의 새 레이스: attach 완료 전 사용자가 곧바로 파일을 만들면 그 이벤트가 유실된다. d-25 가 부팅 경로에서 정리한 "tree 는 조회 시 재스캔으로 자연 수렴 / git status 는 수렴하지 않아 attach 완료 시 1회 refresh 보정 필요" 결론을 project_open 경로에도 그대로 적용해야 한다.
- IdCache 필터로 무시 디렉토리 경계를 넘는 이동(node_modules ↔ src)이 rename 1건이 아니라 From/To 2건으로 온다: `notify-debouncer-full-0.7.0/src/lib.rs:389-406` 확인. 소비자는 두 부모 디렉토리를 각각 갱신하므로 기능상 등가지만, d-50 S8 의 "개명 시 열린 탭 추종"(`layout_apply_path_change`)이 rename 이벤트에 의존한다면 그 경로가 깨질 수 있어 선행 확인이 필수다.
- IdCache 필터가 d-35 §4-e 의 NoCache 기각 사유를 부분 재도입한다는 오해: 기각 대상은 전면 NoCache(모든 rename 파손)였고 필터 캐시는 무시 디렉토리 내부/경계에만 영향을 준다. 그 차이를 계약 문서에 명시하지 않으면 다음 배치가 같은 판단을 다시 뒤집는다.
- git_status 캐시의 stale 창: 워처가 놓친 변경(attach 지연 구간, FSEvents 드롭, 외부 워크트리 조작)이 있으면 캐시가 무기한 낡는다. d-44 가 `fs:changed` 축으로 뚫어 둔 무효화 경로를 캐시에도 반드시 연결해야 하고, 그러지 않으면 d-44 회귀가 된다.
- `StatusOptions::update_index` 채택 시 인덱스 파일 쓰기: `status.c:315` 의 `GIT_DIFF_UPDATE_INDEX` 는 stat 갱신을 `.git/index` 에 되쓴다. 외부 git 프로세스와의 인덱스 락 경합, 그리고 자기 쓰기가 다시 `git/watch.rs` 의 Status 무효화를 유발하는 피드백 루프가 생길 수 있다.
- `read_children` 의 `metadata()` → `file_type()` 전환은 rust-src 미설치로 std 소스를 인용해 확증하지 못했다. macOS 에서 `d_type` 이 DT_UNKNOWN 인 파일시스템(일부 네트워크 볼륨)에서는 fallback stat 이 일어나 이득이 사라지고, 심볼릭 링크 판정 의미가 달라지면 트리 표시가 바뀐다. `fs_usage` 실측과 심링크 픽스처 테스트가 선행돼야 한다.
- `search_list_files` 캐시가 프로젝트 종료 시 회수되지 않으면 메모리 누수 + 재오픈 시 옛 목록 부활. `architecture.md` §6.3 회수 목록에 행을 추가하고 `project::capability` 를 신설해야 하며, 신설 capability 는 lib.rs 등록 순서 파리티 테스트에도 등재돼야 한다.
- `pty_write` 를 상주 writer 태스크로 바꾸면 쓰기 순서 계약이 바뀐다: d-50 §4(7)이 정한 순서 규약과 FE 의 `entities/terminal/session-write-order.ts` 직렬화가 이중으로 걸려 있어, 큐 도입 시 두 계층 중 하나가 불필요해지거나 서로 어긋날 수 있다.
- `poll_agents` 를 sysinfo 로 옮기면 활동 판정이 바뀐다: 현재는 `ps -o state` 의 문자를 파싱해 활동을 추론하는데(`agent/commands.rs:216-224`), sysinfo 의 macOS `ProcessStatus`(`sysinfo-0.39.6/src/unix/apple/macos/process.rs:168-176`)는 thread status 를 우선하고 권한에 따라 결과가 달라질 수 있다. 에이전트 배지가 잘못 켜지거나 꺼지는 회귀 위험.
- 측정용 `[profile.profiling]` 추가가 릴리스 프로필 결정(커밋 9575a33 · full-audit §6 Q1)을 흐릴 위험: 새 프로필이 CI 나 tauri build 경로에 새어 들어가면 배포 바이너리 특성이 바뀐다. 워크스페이스 루트 `Cargo.toml` 에만 두고 CI 에서 참조하지 않음을 테스트나 문서로 못박아야 한다.
- 동시 편집 충돌: 다른 워크플로가 `src-tauri/src/domain/{layout,file,search,settings,window}`·`lib.rs` 를 지금 편집 중이라고 고지됐다. 본 조사가 인용한 줄 번호(특히 `search/commands.rs`·`layout/commands.rs` 는 조사 중에도 줄 수가 변했다)는 착수 시점에 재확인해야 한다.

## questionsForUser
- 계측 노출 범위 — 개발 빌드 전용(`#[cfg(debug_assertions)]` 또는 `TAIDE_PERF=1` 환경변수)만으로 할지, 릴리스에도 설정 토글로 남겨 사용자가 켜서 로그를 보내줄 수 있게 할지. 추천: 환경변수 게이트로 두 빌드 모두에 두되 기본 off(off 시 원자적 load 1회만 지불, 사용자 제보 시 재현 데이터 확보 가능).
- git status 캐시의 정확성 트레이드오프 — 워처가 놓친 변경으로 캐시가 낡을 수 있습니다. (a) 워처 무효화만 신뢰(가장 빠름, 드물게 stale) / (b) 무효화 + 짧은 TTL(예: 2초) 상한 병행(추천: d-44 가 뚫은 fs:changed 경로와 이중 안전망) / (c) 캐시 도입 보류. 어느 쪽인지 결정이 필요합니다.
- libgit2 `StatusOptions::update_index` 채택 여부 — 켜면 stat-stale 파일 재해시가 1회로 끝나 반복 status 가 크게 빨라지지만, 매 status 가 `.git/index` 를 쓰게 되어 외부 git 프로세스와 락 경합이 생기고 자기 쓰기가 다시 status 무효화를 유발할 수 있습니다. 추천: 이번엔 켜지 않고 결과 캐시(위 질문)만으로 처리, 실측 후 재판단.
- 우선순위 8(트리 mutation 응답 재설계 — `full_page` 전체 반환 → revision + 페이지 재조회)을 이번 웨이브에 포함할지. `docs/ipc-contract.md:234` 가 "불변" 으로 고정한 계약이고 프론트 동시 개정이 필요합니다. 추천: 이번 배치 제외, 계측으로 실제 비용을 확인한 뒤 별도 배치.
- `poll_agents` 의 `ps` fork 제거(sysinfo 전환)를 시도할지. 활동 판정 문자(R/S)의 의미가 달라져 에이전트 배지 회귀 위험이 있습니다. 추천: 이번 배치 제외 — 프로젝트당 500ms 1회 fork 는 계측으로 실제 비용을 먼저 확인.
- 프로파일링용 `[profile.profiling]`(strip=false, debug=1)을 워크스페이스 `Cargo.toml` 에 추가해도 되는지. 현재 `strip = true` 라 릴리스 바이너리로는 Instruments/`sample` 프로파일링이 불가능합니다. 릴리스 프로필 자체는 건드리지 않습니다.

## newDependencies



# 주제 3c — 테스트 공백 맵

## findings
## 1. 현행 실측 (2026-09-04, 이 세션 직접 실행)

| 축 | 실측 | 비고 |
|---|---|---|
| bun test | **1835 pass / 0 fail / 3046 expect() / 191 파일 / 5.15s** | HANDOFF §7 기준선 1817(189파일) 대비 +18(배치 3 진행분) |
| bun 커버리지 | `All files 75.28% funcs / 83.65% lines` — **단 로드된 258 파일 한정** | `bun test --coverage` 직접 실행 |
| cargo test | **1248**(HANDOFF §7 기준선). 정적 카운트 `#[test]`/`#[tokio::test]` = **1253**(lib 1227 + cli 17 + tests/ 9) | 다른 워크플로가 `src-tauri` 편집 중이라 `cargo test` 실행은 target 락 경합 우려로 미실행 — 정적 카운트만 제시 |
| e2e | 스펙 **13 파일 / 테스트 14개** (`e2e/specs/01~13-*.e2e.ts`) | `package.json:23` |
| 소스 규모 | TS/TSX 소스 **500 파일 / 43,689 LOC**(테스트 191파일 제외) · Rust **151 파일 / 52,251 LOC** | |

**핵심 수치 — TS 쪽 실질 커버리지는 보고치의 절반이다.**
`bun test --coverage` 표에 등장하는 파일은 258개뿐이고, **소스 500 파일 중 242개(48.4%)는 어떤 테스트도 로드조차 하지 않는다.** 그 242 파일의 합계가 **20,666 LOC = 전체 소스의 47.3%**다. 이를 반영한 전체 라인 커버리지는 약 **44%**(23,023 로드분 × 83.65% ÷ 43,689)다. 83.65% 는 "테스트가 손댄 파일 안에서의" 수치일 뿐이다.

Rust 는 정반대다. 151 파일 중 80 파일이 `#[cfg(test)]` 를 가지며, **미보유 71 파일의 합은 3,905 LOC = 전체 Rust 의 7.5%**뿐이다(대부분 `mod.rs`·`types.rs`·`capability.rs`). 구조적으로는 Rust 가 훨씬 촘촘하다.

## 2. 테스트 관례 (실코드가 정본 — 문서 정본은 없음)

`docs/memory/` 에는 `tooltip-conventions.md` 하나뿐이고 **테스트 관례를 적은 문서가 없다.** 아래는 실코드/계약 문서에서 역추출한 실제 관례다.

- **describe/test 설명은 한국어**. Rust 는 **함수명 자체가 한글 식별자**다 — `src-tauri/src/domain/file/service.rs:653` `fn 폴더를_자기_하위로_복사하면_거부한다()`.
- **Rust: dev-dependencies 0개.** `src-tauri/Cargo.toml` 에 `[dev-dependencies]` 절 자체가 없다. `tempfile` 크레이트를 안 쓰고 `src-tauri/src/domain/file/service.rs:645-647` 처럼 `std::env::temp_dir().join(format!("taide-…-{}", uuid::Uuid::new_v4()))` + 수동 `cleanup()` 로 격리한다.
- **Rust: trait 인라인 대체는 실질 2곳뿐** — `domain/project/capability.rs:23 ProjectCapability`, `domain/ai/providers/mod.rs:9 AiProviderClient`. 나머지는 `service.rs` 순수 함수를 직접 호출하는 방식이며, `commands.rs` 는 "얇게" 규약(architecture.md §2)대로 `State<'_, AppState>` 결합 때문에 테스트가 거의 없다.
- **TS: 순수 함수 우선 추출.** 렌더 로직을 monaco-free 모듈로 떼어내고 그것만 테스트한다 — `src/features/problems/problem-list-rows.ts` ↔ `problem-list-rows.test.ts`, `src/widgets/editor-pane/code-editor-visibility.ts`(`docs/acknowledge/2026-08-20-blank-window-hotfix-contract.md:205-236` 이 이 선택의 근거를 명시).
- **TS: `mock.module` + 동적 `import()`.** `src/entities/git/git.query.test.ts:39` 가 `@entities/git/git.ipc` 를 통째 스텁한 뒤 `git.query.ts` 를 동적 import 한다. 현재 **16개 테스트 파일에서 37회** 사용. `mock.restore()` 는 **한 번도 쓰이지 않는다**.
- **모듈 전역 상태 누수 방어는 수동.** `src/shared/lib/lsp/adapters/definition.test.ts:13-19` — "bun does not isolate modules per file" 라는 JSDoc과 함께 `afterEach(resetPeekModelPreloadStateForTests)`.
- **monaco 정적 import 는 bun test 에서 로드 불가.** `src/shared/lib/monaco/setup.ts:2-6` 의 `…?worker` Vite suffix 때문. 직접 확인: `bun -e "import('./src/shared/lib/monaco/setup.ts')"` → `Missing 'default' export … css.worker.js?worker`. `docs/ipc-contract.md:1007` 이 이 제약 때문에 `lsp-session-flush-registry.ts` 를 분리했다고 기록.

## 3. TS 공백 맵 (레이어별 — 한 번도 로드되지 않는 242 파일)

| 레이어 | 미로드 파일 | 성격 |
|---|---|---|
| **features** | **93 / 96** (`.tsx` 92) | 사실상 **전량**. FSD 상 "비즈니스 로직 없는 순수 컴포넌트" — 렌더 하네스가 없어 원천적으로 미커버 |
| **widgets** | **68** (`.tsx` 57 + 훅 `.ts` 11) | `use-editor-file-persistence.ts`(588 LOC)·`use-explorer-clipboard.ts`·`use-explorer-entry-crud.ts`·`use-zen-mode.ts`·`use-workspace-symbol-search.ts` 등 훅 전량 |
| **shared** | 35 (`.tsx` 13 = `shared/ui` 대부분, `.ts` 22) | `lib/ai/inline-completion.ts`(202 LOC)·`icons/file-icon-registry.ts`·`lib/xterm-theme.ts`·`lib/system-appearance.ts`·`lib/window-appearance.ts`·`hooks/use-object-url.ts`·`hooks/use-ipc-error-message.ts` |
| **entities** | 34 | `.query.ts` 12종(search·theme·task·plugin·snippet·sync·remote·app-file·font·locale·ai·vsix) + `.ipc.ts` 다수 + `use-search-run.ts`(179 LOC) |
| **app** | 11 | `bootstrap-commands/lsp/snippets.ts`·`query-client.ts` + 프로바이더 `.tsx` |

**로드는 되지만 라인 커버리지가 낮은 상위(진짜 취약 지점)**
- `src/entities/file/file.query.ts` **15.60%**, `git.query.ts` 40.57%, `layout.query.ts` 43.53%, `settings.query.ts` 32.35%, `project.query.ts` 34.88%, `lsp.query.ts` 44.19%, `tree.query.ts` 44.44%, `agent.query.ts` 45.71%
- `src/app/providers/ide-sync-provider.tsx` **16.67%**, `ipc-sync-provider.tsx` 19.86%, `keybindings-runtime-provider.tsx` 33.33%
- `src/shared/hooks/use-overlay-scrollbar.ts` 2.25%, `use-tauri-event.ts` 8.70%, `use-global-keymap.ts` 16.95%
- `src/shared/lib/remote/callback-registry.ts` **3.45%**, `tauri-internals-shim.ts` 7.14%, `remote-ws-client.ts` 9.40%
- `src/widgets/explorer/explorer-path.ts` **12.50%**(8줄짜리 순수 함수 2개)
- `src/features/terminal/terminal-view.tsx` 7.64%, `features/settings/keybinding-row.tsx` 7.04%

**주의 — "테스트 파일 sibling 부재"는 공백의 척도가 아니다.** `theme-convert/*` 8개 모듈, `shared/lib/path-root.ts`, `shared/lib/lsp/initialize-params.ts`, `adapters/declaration|implementation|type-definition.ts` 는 sibling 테스트가 없지만 상위 테스트(`convert.test.ts`·`definition.test.ts` 등)를 통해 **라인 100%** 로 이미 커버된다. 공백 판정은 `bun test --coverage` 를 기준으로 해야 한다.

**부수 발견**: `scripts/*.ts`(256 LOC)는 테스트도 없고 `tsconfig.json` 의 `include`(`["src", "vite.config.ts", "eslint.config.js", "prettier.config.js"]`)에도 없어 **`bun run typecheck` 대상 밖**이다.

## 4. Rust 공백 맵

**도메인별 테스트 밀도(테스트 수 / 100 LOC)** — 낮은 순:

| 도메인 | tests / LOC | 밀도 | 감사에서 확인된 결함 |
|---|---|---|---|
| locale | 16 / 1568 | **1.02** | — |
| window | 7 / 489 | **1.43** | `commands.rs` 314 LOC 테스트 0 |
| project | 21 / 1488 | 1.41 | — |
| vsix | 18 / 1145 | 1.57 | §4-B B6(테마 데이터 영구 소실) |
| theme | 40 / 2229 | 1.79 | B5(활성 테마 삭제 fallback 부재) |
| tree | 21 / 1155 | 1.82 | C2(유령 자식 — 캐시 미정리) |
| app | 5 / 284 | 1.76 | — |
| file | 47 / 2514 | 1.87 | §4-A 1·2·3(데이터 파괴 3건) |
| terminal | 21 / 1105 | 1.90 | A6·B14·C14·§2 M-5 |
| layout | 68 / 3529 | 1.93 | A3(탭 미추종) |
| git | 101 / 5110 | 1.98 | §4-A 6·10, B10·B11 |
| (최상위) remote 133/4931=2.70, ai 100/3053=3.28, agent 93/2798=3.32, task 16/428=3.74 | | | |

**`#[cfg(test)]` 가 아예 없는 유의미한 파일**
- `src-tauri/src/infra/archive.rs` (124 LOC, 테스트 0) — **보안 표면**. `extract_hardened_zip`(`:68`)이 zip-slip(`enclosed_name`)·엔트리 수 상한 5,000·누적 해제 128MB 예산·모드 0o644/0o755 강제를 수행하는데 그 어느 것도 테스트가 없다. 같은 성격의 `infra/root_guard.rs` 는 8개, `redact.rs` 8개, `crypto.rs` 3개를 갖고 있어 비대칭이 두드러진다.
- `src-tauri/src/events.rs` (206 LOC, 0) — 프론트행 이벤트 payload 직렬화 정본(ipc-contract Rust 측). 필드명 드리프트를 기계 강제하는 테스트가 없다.
- `src-tauri/src/error.rs` (100 LOC, 0) — `AppError` 중앙 타입. `d-34` 캠페인으로 369지점이 이 타입을 쓰는데 직렬화·localized 인자 계약 테스트가 없다.
- `src-tauri/src/domain/window/commands.rs` (314 LOC, 0), `domain/ide/commands.rs` (259, 0), `domain/file/commands.rs` (253, 0), `domain/settings/commands.rs` (92, 0), `domain/plugin/commands.rs` (92, 0)
- `capability.rs` 7개 중 `project/capability.rs`(2)만 테스트 보유 — file/git/layout/terminal/tree/agent/ide 는 0. `attach`/`detach` 대칭(architecture.md §3 "반드시 대칭")이 기계 강제되지 않는다.
- `infra/clock.rs`(17, 0), `crates/…` 는 17개 보유.

**통합 테스트는 2파일 9케이스뿐** — `src-tauri/tests/session_restore.rs`(6), `domain_boundaries.rs`(3, 소스 스캔 아키텍처 테스트). 549 LOC.

## 5. e2e 하네스 제약 · 미커버 흐름

**제약(정본 `docs/quality-assurance/2026-08-18-e2e-harness.md`)**
1. **앱을 사용자가 직접 켜야 한다** — `webServer` 블록이 의도적으로 없고, `globalSetup` 이 remote-control 포트를 못 찾으면 fail-fast(§2). CI 자동화 불가.
2. REMOTE 비밀번호 설정 + `remotePasswordOnlyLogin` ON + `TAIDE_E2E_PASSWORD` export 가 사전 조건(§1). 로그인 POST 는 런당 1회, 실패 5회 누적 시 60초 잠금(§4).
3. **직렬 고정**(`workers:1`·`fullyParallel:false`·`retries:0`) — 한 앱 인스턴스 공유(§3).
4. **WKWebView 입력 우회** — `keyboard.type()`/`insertText()` 가 콘텐츠를 손상시켜 `lib/monaco-clipboard.ts` 클립보드 붙여넣기로 대체(스펙 01·10·11).
5. **원격 게이트 4필드 회귀는 자동 복구 불가**(§3.2) — 스펙 11 실패는 "테스트 실패"가 아니라 수동 복구가 필요한 보안 이벤트.
6. **`buildFullRestorePatch` 가 `Option<T>` 를 `null` 로 되돌리지 못한다**(§7) — `editorFontFamily`·`uiFontFamily`·`terminalFontFamily`·`keymapOverrides` 를 건드리는 신규 스펙은 별도 원복 전략 선행 필요.
7. **스위트 후반 열화(C8)** — 9번째 테스트쯤부터 `window.__TAURI_INTERNALS__` 미로드 연쇄 실패. 서버 무죄 확정(2026-08-28 라이브 프로브), 원인은 Vite HMR WS 재시도 루프에 의한 webkit 프로세스 열화. **dev 전용이지만 "스펙을 14개 이상으로 늘리면 그대로 재발"한다** — 확장 시 최우선 선결 과제.

**현재 커버 흐름(13스펙)**: 저장 왕복 / 팔레트 커맨드 / `@`문서심볼 / `:`줄이동 / `#`워크스페이스심볼 / 전역검색+excludeGlob / git stage·unstage / 터미널 OSC133 / 테마 전환·원복 / AppFile settings 저장 / 원격 게이트 / 이벤트 팬아웃 / 퀵오픈 미확장 폴더.

**e2e 로 커버 가능한데 미커버인 핵심 흐름** (감사 §4-B 의 데이터 손실·핵심 오동작 클래스와 정확히 겹친다):
- 탐색기 CRUD 전반 — 생성/개명/삭제/복사/붙여넣기 충돌(A2 무경고 덮어쓰기, C1 제자리 잘라내기, C3 확장자 오분해)
- 개명·삭제 후 열린 탭 추종(A3 — 폴더 개명 후 ⌘S 가 옛 경로에 저장하는 조용한 fork)
- 탭 pin 보호·닫기·⇧⌘T 재열기(B4 — tabs.md §3 위반)
- 스플릿 동일 파일 양쪽 dirty·저장 정착(A7)
- 터미널 탭 이탈→복귀 재부착(A6 — 매번 새 셸 + 고아화)
- Replace All 이 화면 결과와 일치하는가(A5 — 비가역)
- merge 충돌 중 커밋 게이트(A4 — 마커째 커밋)
- diff 탭 / hunk stage / stash
- 멀티 윈도우 탭 이동(`layout_move_tab_to_window`) + Zen 모드
- 키맵 재바인딩·2단 chord(B1·B2·C7)
- 플러그인/VSIX 임포트(B6·B7)
- 세션 복원(핫엑시트) 왕복 — Rust `session_restore.rs` 는 있으나 UI 왕복은 없음
- 작업 러너 / Problems 패널 / 아웃라인

## 6. CI 게이트 부재 (안정성 관점의 최대 구멍)

`.github/workflows/` 에 워크플로가 **2개뿐**이며, 테스트를 **실행하는** 것은 `release.yml` 하나다.

- `release.yml:3-5` — 트리거가 `push: tags: ['v*']` + `workflow_dispatch` **뿐**. `bun run test`(`:30`)·`cargo test --workspace`(`:49`)가 **릴리스 태그를 붙일 때만** 돈다.
- `cache-warm.yml` — `push: branches:[main]` 이지만 `cargo test --workspace --no-run`(컴파일만) 이다. **실행하지 않는다.**
- `pull_request` 트리거 워크플로 **없음**. dev·main push 에 대한 테스트 게이트 **없음**.
- CI 어디에도 `typecheck`·`lint`·`format:check` 가 없다(`bun run verify` 는 로컬 전용, `package.json:21`).
- `typecheck:e2e`(`:24`)는 `verify` 에 포함되지 않아 **수동 실행 전용**이다.

즉 현재 회귀 방어선은 **"개발자가 로컬에서 `bun run verify` 를 돌린다"** 는 규율 하나다. 테스트를 대폭 보강해도 이 게이트가 없으면 보강분이 자동으로 지켜지지 않는다.

## 7. 컴포넌트 테스트 — 기존 결정과 실제 제약

- **선언적 정본**: `src/shared/ui/error-boundary.test.tsx:7` — "This project has no DOM render harness (no `@testing-library/react`, no jsdom/happy-dom …)". 이어지는 JSDoc(`:5-41`)이 그 결과로 무엇을 포기했는지를 명시한다("크래시 → 폴백 → 재시도 클릭 → children 재마운트 전체 왕복은 검증 불가").
- 동일 제약이 4개 계약 문서에 반복 기록: `2026-08-20-blank-window-hotfix-contract.md:114,211` · `2026-08-20-crash-class-seal-contract.md:193,245` · `2026-08-24-d33-restructure-carryover-contract.md:424` · `2026-08-29-d51-audit-frontend-batch-contract.md:419,1326,1354`. 모두 "필요 시 **별도 합의 후** 진행 권고"로 끝난다 — **기각이 아니라 미결 상태**다.
- `docs/tech-stack.md:14` 는 이미 `테스트 | bun:test + Testing Library(프론트)` 로 적어 두었다 — **문서와 실제가 어긋나 있다**(현재 Testing Library 미설치).
- `docs/feedback/2026-08-07-radix-aschild-fc-trigger.md:52-53` 은 실제 회귀(Radix `asChild` 자식이 FC → 우클릭 메뉴 소실, **TypeScript 로도 안 잡힘**)를 겪고 "컴포넌트 테스트가 없어 이런 회귀는 기계 검증으로 잡히지 않는다"고 결론지었다.
- `node_modules` 실사: `happy-dom`·`jsdom`·`@testing-library/*`·`react-test-renderer` **전부 미설치**.

**직접 실측한 사실 3가지 (설계 판단의 근거)**
1. **`react-dom/server` 는 신규 의존성 0으로 지금 당장 동작한다.** `bun -e` 로 `renderToStaticMarkup(h(Btn,{label:'Save',disabled:true}))` → `<button type="button" disabled="" aria-label="Save">Save</button>` 확인. `useState` 초기값도 렌더된다. 단 `useEffect`·이벤트·`ref`·DOM 질의는 없다.
2. **React 19.2.8 은 `act` 를 자체 export 한다** (`node_modules/react/cjs/react.development.js` 에 `exports.act`). 즉 훅 테스트에 필요한 건 **DOM 전역 하나뿐**이고 `react-test-renderer` 는 불필요하다.
3. **의존성 비용 실측**(`bun pm view`): `happy-dom@20.14.0` = MIT, 전이 의존 7개(ws·entities·@types/ws·@types/node·whatwg-mimetype·buffer-image-size·@types/whatwg-mimetype), unpacked 8.57MB. `@testing-library/react@16.3.3` = MIT, deps 1(`@babel/runtime@^7`), peer 로 **`@testing-library/dom@^10` 별도 설치 필수**. `@testing-library/dom@10.4.1` = deps 8(lz-string·aria-query·picocolors·pretty-format@27·@babel/runtime@7·@babel/code-frame·@types/aria-query·dom-accessibility-api), 2.43MB. 현재 devDependency 는 `@babel/runtime@8.0.0` 이라 **@babel/runtime@7 사본이 추가 설치**된다. 전부 devDependency 이므로 앱 번들에는 들어가지 않는다.

## options
## 안 A — 순수 로직 확장만 (신규 의존성 0)

**범위**: 미로드 242 파일 중 "런타임 import 가 없거나 `mock.module` 로 우회 가능한" `.ts` 모듈 + 저커버 `.query.ts` + Rust 공백 + e2e 확장. `.tsx` 172개는 손대지 않는다.

**변경 파일**
- 신규 TS 테스트 ~20파일: `src/shared/lib/remote/{callback-registry,remote-json}.test.ts`, `src/shared/lib/keymap/keymap-capture.test.ts`, `src/shared/lib/{xterm-theme,system-appearance,window-appearance}.test.ts`, `src/shared/lib/ai/inline-completion.test.ts`, `src/shared/icons/file-icon-registry.test.ts`, `src/widgets/explorer/explorer-path.test.ts`, `src/entities/{search,theme,task,plugin,snippet,sync,app-file,file,project,settings}/*.query.test.ts`
- Rust: `infra/archive.rs`·`error.rs`·`events.rs`·`domain/window/commands.rs` 에 `#[cfg(test)]` 신설, `domain/{terminal,tree,locale,vsix,file}` 테스트 확장
- e2e: `e2e/specs/14~18-*.e2e.ts` + `e2e/lib/explorer.ts` 헬퍼
- CI: `.github/workflows/ci.yml` 신설(push/PR 게이트)
- 문서: `docs/memory/test-conventions.md` 신설, `docs/tech-stack.md:14` 정정

**데이터 흐름**: 기존과 동일. `mock.module('@entities/x/x.ipc')` → 동적 `import('@entities/x/x.query')` → `queryOptions().queryFn()` 직접 호출 → 반환/`QUERY_KEY` 검증. Rust 는 `service.rs` 순수 함수 직접 호출 + `env::temp_dir()` 격리.

**장점**: 신규 의존성 0(레포 기본 원칙 유지). 기존 관례 100% 재사용이라 학습 비용 0. 회귀 위험 0(프로덕션 코드 무변경). 감사가 지적한 `.query.ts` 저커버(15~45%)를 직접 메운다.
**단점**: **features 96 + widgets 57 = 153개 컴포넌트와 11개 위젯 훅은 영구 미커버**로 남는다. 감사 §4-B 결함의 상당수(A1 effect 순서, A6 재부착, B4 pin 보호, D1 포커스 복원, Radix asChild 회귀)가 정확히 이 사각지대에서 나왔다. e2e 로만 잡히는데 e2e 는 사용자가 앱을 켜야 하고 14스펙 이상 확장 시 C8 열화에 부딪힌다.

---

## 안 B — 안 A + `happy-dom` 단독 (직접 의존성 1개, RTL 없음)

**추가 변경**
- `package.json` devDependencies 에 `happy-dom` 1줄
- `bunfig.toml` 신설(현재 없음): `[test] preload = ["./src/shared/testing/dom-preload.ts"]`
- `src/shared/testing/dom-preload.ts` — `GlobalRegistrator.register()` (happy-dom 제공)
- `src/shared/testing/render.ts` — `createRoot` + React 19 내장 `act` 로 감싼 30줄 내외의 자체 `renderComponent`/`renderHook` 헬퍼(`@testing-library/react` 대체)

**데이터 흐름**: preload 가 `document`/`window` 전역을 심음 → `react-dom/client` `createRoot(container).render(el)` 을 `act()` 안에서 호출 → `container.querySelector`/`dispatchEvent` 로 검증. 훅은 `renderHook` 이 더미 컴포넌트로 감싸 반환값을 ref 에 노출.

**장점**: **`useEffect`·이벤트·ref·포커스가 전부 동작**한다 → A1(effect 순서 역전)·A6(재부착)·A7(스플릿 저장 정착)·D1(포커스 복원) 클래스를 단위 테스트로 재현 가능. 위젯 훅 11개가 테스트 가능해진다. 신규 직접 의존성 1개(전이 포함 8패키지, 8.57MB, 전부 dev). RTL 을 안 쓰므로 `@testing-library/dom` 8패키지 + `@babel/runtime@7` 중복이 없다.
**단점**: `screen`/`getByRole`/`userEvent`/자동 cleanup 을 직접 만들어야 한다(초기 헬퍼 ~80 LOC, 유지 부담). 접근성 기반 쿼리가 없어 "역할·라벨로 찾기"(frontend.md §11 권장)를 못 지키고 `querySelector`(구조 결합)로 흐르기 쉽다. happy-dom 이 WKWebView 가 아니라 **회귀 재현 충실도는 e2e 보다 낮다**(감사가 반복 지적한 WKWebView 고유 결함 — IME keyCode 229·클립보드 손상 — 은 여전히 e2e 전용).

---

## 안 C — 안 A + `happy-dom` + `@testing-library/react` (직접 의존성 3개)

**추가 변경**: 안 B 와 동일 + `@testing-library/dom`·`@testing-library/react` devDependency, `dom-preload.ts` 에 `afterEach(cleanup)` 등록.

**장점**: `render`/`screen.getByRole`/`fireEvent` 로 **접근성 기반 쿼리**를 쓴다 — frontend.md §11("Testing Library 로 사용자 관점(역할·라벨 쿼리)")·§8.5 접근성 최소선과 정확히 일치하고, `docs/tech-stack.md:14` 의 기존 기재와도 맞아떨어진다. Radix/shadcn 컴포넌트(이 레포 UI 의 근간)는 RTL 예제가 풍부해 작성 속도가 안 B 보다 눈에 띄게 빠르다. 자동 cleanup 이 있어 bun 의 "파일 간 모듈 미격리" 누수 위험을 줄인다.
**단점**: 직접 3 + 전이 약 13 = **신규 패키지 ~16개, unpacked ~11.3MB**. `@babel/runtime@7` 이 기존 `@babel/runtime@8.0.0` 과 별도 사본으로 들어온다(빌드 무영향, lockfile 만 커짐). RTL v16 은 `@testing-library/dom` 을 peer 로 요구해 버전 3개를 동시에 관리해야 한다. "신규 의존성 0" 원칙을 명시적으로 깨는 첫 사례가 된다.

## recommendation
## 권고: **안 A 를 즉시 전량 실행**하고, 컴포넌트/훅 하네스는 **안 B(happy-dom 단독)** 로 별도 승인받아 뒤이어 착수한다. 안 C 는 비추천.

### 왜 이 조합인가

1. **안 A 는 결정이 필요 없고 즉시 효과가 크다.** 미로드 47.3% LOC 중 `.ts` 70파일과 저커버 `.query.ts` 10종은 기존 `mock.module` 관례만으로 전부 도달 가능하다. 프로덕션 코드 무변경이라 회귀 위험이 0이고, 다른 워크플로가 편집 중인 파일(explorer·command-palette·terminal-pane·layout·file·search 등)과 충돌하지 않도록 순서만 조정하면 된다.
2. **안 B 를 안 C 보다 권하는 이유는 원칙과 실익의 균형이다.** 이 레포는 "신규 의존성 0"을 v0.1.6 까지 유지해 왔다(HANDOFF 기준선). 하네스 도입의 실질 가치는 **DOM 전역 하나**에서 나온다 — React 19 가 `act` 를 자체 제공(실측 확인)하므로 RTL 은 편의 API 일 뿐 능력을 늘리지 않는다. 직접 1개(전이 8)와 직접 3개(전이 ~16)의 차이는 원칙 훼손 폭에서 두 배 이상이다. 다만 `frontend.md §11`·`docs/tech-stack.md:14` 가 이미 Testing Library 를 명시하고 있어 **컨벤션과의 정합만 놓고 보면 안 C 가 맞다** — 이 상충은 사용자 결정 사항으로 올린다(questionsForUser 1).
3. **`react-dom/server` 만으로 버티는 안(제4안)은 권하지 않는다.** 신규 의존성 0으로 렌더는 되지만 `useEffect`·이벤트가 죽어 있어, 감사가 실제로 찾아낸 결함 클래스(A1 effect 순서, A6 재부착, D1 포커스)를 단 한 건도 재현하지 못한다. 커버리지 숫자만 올리고 안정성은 못 올리는 전형적 함정이다.
4. **CI 게이트가 없으면 보강분이 지켜지지 않는다.** 현재 테스트는 릴리스 태그에서만 돈다(§6). 테스트 보강과 `push`/`pull_request` 게이트 신설은 **같은 배치에서** 해야 한다.

---

## 우선순위별 테스트 추가 계획

### P0 — 순수 모듈 (신규 의존성 0, 즉시) · 예상 **55~70 케이스 / 9 파일**

| 파일 | 대상 | 케이스 개요 | 수 |
|---|---|---|---|
| `shared/lib/remote/callback-registry.test.ts` | 현재 라인 **3.45%** | id 단조 증가 / once 콜백 1회 후 자동 제거 / 미등록 id `runCallback` no-op / `unregisterCallback` 후 재실행 무효 / once 아닌 콜백 반복 실행 | 6 |
| `shared/lib/remote/remote-json.test.ts` | 50% | `parseJson` 파싱 실패 → `null`(throw 안 함) / `isRecord(null)`·`isRecord([])` 판정 / `numberOf`·`stringOf` 폴백(`NaN`·`undefined`·객체) | 7 |
| `widgets/explorer/explorer-path.test.ts` | **12.50%** | `parentDirOf` 루트(`/a`)·중첩·슬래시 없음 / `joinPath` 트레일링 슬래시 정규화 / 두 함수 왕복 일관성 | 8 |
| `shared/lib/keymap/keymap-capture.test.ts` | 45.45% | 구독→`setKeymapCapturing` 통지 / **동일 값 재설정 시 미통지**(조기 return) / unsubscribe 후 미통지 / 다중 구독자 / 스냅샷 일치 | 6 |
| `shared/lib/ai/inline-completion.test.ts` | 미로드(202 LOC) | `resolveAiInlineCompletionConfig` — provider 3종×토큰 상태 조합, 비활성 조건, 기본값. **monaco 를 `import type` 으로만 참조해 bun 에서 그대로 로드된다(실측)** | 12 |
| `shared/lib/xterm-theme.test.ts` | 미로드 | `toXtermTheme` — ANSI 16색 매핑 완전성, 누락 토큰 폴백, 커서/선택 색 | 6 |
| `shared/icons/file-icon-registry.test.ts` | 미로드 | `FILE_ICON_COMPONENT_MAP` 키 집합 ↔ `FileIconName` union 파리티(누락·잉여 0) — `settings.constant.test.ts` 파리티 선례 동형 | 3 |
| `shared/lib/{system,window}-appearance.test.ts` | 미로드 | 시스템 외관 판정·매핑 순수 분기 | 8 |
| `shared/constants/{layout,window-chrome,system-usage}.test.ts` | 미로드 | 상수 파생값 불변식(합·범위·정렬) | 5 |

### P1 — entities 데이터 레이어 (신규 의존성 0) · 예상 **70~95 케이스 / 10~12 파일**

`git.query.test.ts:39` 의 `mock.module(...ipc)` + 동적 import 패턴을 그대로 복제한다. 각 파일 공통 케이스: ① `queryOptions().queryKey` 가 `QUERY_KEY` 중앙 상수와 동일 ② `queryFn` 이 IPC 응답에서 필요한 필드만 언랩 ③ mutation `onSuccess` 의 `invalidateQueries` 스코프가 의도 범위(과대·과소 무효화 없음) ④ 에러 경로.

- 저커버 우선: `file.query.ts`(**15.60%**, ~12케이스) · `settings.query.ts`(32.35%, ~8) · `project.query.ts`(34.88%, ~8) · `git.query.ts`(40.57% → 미커버 mutation 축 ~10) · `layout.query.ts`(43.53% → 미커버 133-166 ~8) · `lsp.query.ts`(44.19%, ~6) · `tree.query.ts`(44.44%, ~6) · `agent.query.ts`(45.71%, ~5)
- 미로드 신규: `search.query.ts` · `theme.query.ts` · `task.query.ts` · `plugin.query.ts` (각 ~5)

### P2 — Rust 공백 (신규 의존성 0) · 예상 **55~75 케이스**

| 대상 | 케이스 개요 | 수 |
|---|---|---|
| `infra/archive.rs` (**테스트 0, 보안 표면**) | zip-slip 경로(`../../etc/x`) 무시 / 엔트리 5,000 초과 거부 / 누적 128MB 예산 초과 시 **압축 해제 도중** 중단 / 아카이브가 주장하는 `unix_mode` 무시하고 0o644·0o755 강제 / 디렉토리 엔트리 처리 / 정상 왕복. `env::temp_dir()+uuid` 관례로 격리, `zip` 크레이트(기설치)로 픽스처 생성 | 10 |
| `events.rs` (206 LOC, 0) | 각 이벤트 payload 의 serde 필드명이 `docs/ipc-contract.md` 계약과 일치(직렬화 스냅샷) | 8 |
| `error.rs` (100 LOC, 0) | `AppError` 변형별 직렬화 형태 / `localized` 의 `with_arg` 누적 / `AppErrorKind` ↔ 코드 매핑 | 6 |
| `domain/window/commands.rs` (314 LOC, 0) | 창 라벨 생성·파싱, 보조 창 id 규칙, 중복 라벨 거부 등 `State` 불필요한 순수 헬퍼부터 | 6 |
| `domain/*/capability.rs` 7개 (project 제외 전부 0) | `attach`/`detach` **대칭 파리티** — `lib.rs` 의 `project_capabilities()` 등록 목록과 각 구현의 detach 존재를 소스 스캔으로 기계 강제(`tests/domain_boundaries.rs` 선례 동형). `src-tauri/tests/` 에 신규 통합 테스트 1파일 | 4 |
| `domain/terminal/service.rs`(밀도 1.90) | 링버퍼 경계(용량 초과 drain·정확히 채움·0바이트), OSC133 파싱 경계 | 8 |
| `domain/tree/commands.rs`(4/382) | 삭제 후 캐시 정리(감사 C2 유령 자식 회귀 가드), flatten 조기 종료 | 6 |
| `domain/locale/service.rs`(밀도 1.02) · `vsix/service.rs`(1.57) · `theme/service.rs` | 언어팩 `extends` 부분 병합 우선순위, VSIX 무변경 저장 시 `tokenColors`/author 보존(감사 B6 회귀 가드), 활성 테마 삭제 시 fallback(B5) | 12 |

### P3 — e2e 확장 (신규 의존성 0, **C8 선결 필요**) · 예상 **5~7 스펙**

**선결**: 현행 14테스트에서 이미 스위트 후반 열화가 재현되므로, 스펙을 늘리기 전에 ① 원격 페이지의 Vite HMR 클라이언트 비활성(또는 프록시 WS 포워딩) 또는 ② `playwright.config.ts` 를 테스트당 브라우저 재기동으로 전환 중 하나를 먼저 처리한다.

1. `14-explorer-crud-conflict.e2e.ts` — 생성/개명/삭제 + **접힌 폴더 대상 붙여넣기 충돌 경고**(A2) + 대소문자만 다른 개명(§4-A-1)
2. `15-tab-follow-rename.e2e.ts` — 파일·폴더 개명 후 열린 탭 경로 추종 + ⌘S 가 새 경로에 저장(A3)
3. `16-terminal-reattach.e2e.ts` — 터미널 탭 이탈→복귀 시 **같은 셸 재부착**(A6, 새 셸 스폰 0)
4. `17-split-same-file-save.e2e.ts` — 스플릿 양쪽 dirty·한쪽 저장 후 반대쪽 정착(A7)
5. `18-tab-pin-guard.e2e.ts` — pin 탭이 ⌘W·미들클릭·unpin 버튼으로 닫히지 않음(B4)
   (+ `e2e/lib/explorer.ts` 트리 조작 헬퍼 신설, `lib/monaco-clipboard.ts` 재사용)

### P4 — 컴포넌트/훅 하네스 (안 B 승인 시) · 예상 **60~90 케이스 / 15~20 파일**

착수 순서: ① 위젯 훅 11개 중 데이터 손실 결부 3개(`use-editor-file-persistence`·`use-explorer-clipboard`·`use-explorer-entry-crud`) ② `app/providers` 저커버 3개(`ide-sync` 16.67%·`ipc-sync` 19.86%·`keybindings-runtime` 33.33%) ③ `shared/ui` 13개 ④ `features/*` 는 감사 결함이 있었던 슬라이스(tab·explorer·git·terminal)부터 선별. **96개 전량 커버는 목표로 삼지 않는다.**

### P5 — 게이트·문서 (안 A 와 동시)

- `.github/workflows/ci.yml` 신설: `push`(dev·main) + `pull_request` → `bun install` → `typecheck` → `lint` → `format:check` → `bun test` → `typecheck:e2e`. Rust job 은 `cargo fmt --check`·`clippy -D warnings`·`cargo test --workspace`. 러너는 프론트 job 만 `ubuntu-latest`(감사 §3 Q3 추천안과 정합), Rust 는 macOS 유지.
- `docs/memory/test-conventions.md` 신설 — §2 의 관례를 정본화(현재 어디에도 없음).
- `docs/tech-stack.md:14` 정정 — 안 A 만 채택 시 "Testing Library" 기재를 실제와 맞춘다.
- `tsconfig.json` `include` 에 `scripts` 추가 검토(현재 typecheck 사각지대).
- `docs/quality-assurance/2026-09-04-test-gap-map.md` — 이 조사의 수치를 체크박스 형태로 정본화.

**총 예상 증분**: P0~P3 만으로 **약 185~240 TS/Rust 케이스 + e2e 5~7 스펙**(bun 1835 → 약 2000, cargo 1248 → 약 1320). P4 포함 시 bun 약 2090.

## filesToTouch
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/remote/callback-registry.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/remote/remote-json.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/keymap/keymap-capture.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/xterm-theme.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/system-appearance.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/window-appearance.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/lib/ai/inline-completion.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/shared/icons/file-icon-registry.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/widgets/explorer/explorer-path.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/file/file.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/settings/settings.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/project/project.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/search/search.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/theme/theme.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/task/task.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/plugin/plugin.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/tree/tree.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/lsp/lsp.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/layout/layout.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src/entities/git/git.query.test.ts
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/infra/archive.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/events.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/error.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/window/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/terminal/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/tree/commands.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/locale/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/vsix/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/src/domain/theme/service.rs
- /Users/hyunseokbyun/development/TAIDE/src-tauri/tests/capability_symmetry.rs
- /Users/hyunseokbyun/development/TAIDE/e2e/specs/14-explorer-crud-conflict.e2e.ts
- /Users/hyunseokbyun/development/TAIDE/e2e/specs/15-tab-follow-rename.e2e.ts
- /Users/hyunseokbyun/development/TAIDE/e2e/specs/16-terminal-reattach.e2e.ts
- /Users/hyunseokbyun/development/TAIDE/e2e/specs/17-split-same-file-save.e2e.ts
- /Users/hyunseokbyun/development/TAIDE/e2e/specs/18-tab-pin-guard.e2e.ts
- /Users/hyunseokbyun/development/TAIDE/e2e/lib/explorer.ts
- /Users/hyunseokbyun/development/TAIDE/e2e/playwright.config.ts
- /Users/hyunseokbyun/development/TAIDE/.github/workflows/ci.yml
- /Users/hyunseokbyun/development/TAIDE/docs/memory/test-conventions.md
- /Users/hyunseokbyun/development/TAIDE/docs/quality-assurance/2026-09-04-test-gap-map.md
- /Users/hyunseokbyun/development/TAIDE/docs/tech-stack.md
- /Users/hyunseokbyun/development/TAIDE/docs/PROCESS.md
- /Users/hyunseokbyun/development/TAIDE/tsconfig.json
- /Users/hyunseokbyun/development/TAIDE/package.json

## risks
- bun 은 테스트 파일 간 모듈을 격리하지 않는다 — `src/shared/lib/lsp/adapters/definition.test.ts:13-19` 가 이 사실을 JSDoc 으로 기록하고 `afterEach` 로 수동 리셋한다. P0/P1 이 모듈 전역 싱글턴(`keymap-capture.ts` 의 `listeners`/`isCapturing`, `inline-completion.ts` 의 `completionCache`·`owners`, `callback-registry` 의 인스턴스)을 다루므로, 리셋 훅 없이 추가하면 **다른 파일의 기존 1835 케이스를 실행 순서에 따라 간헐 실패**시킬 수 있다. 신규 테스트마다 상태 리셋을 명시적으로 넣어야 한다.
- `mock.module` 은 프로세스 전역이다. 현재 16파일·37회 사용 중이며 `mock.restore()` 는 한 번도 호출되지 않는다. P1 에서 `.query.test.ts` 를 10여 개 더 추가하면 `@entities/*/‌*.ipc` 스텁이 나중에 로드되는 다른 테스트 파일에 그대로 남아 **거짓 통과(스텁이 실제 IPC 대신 응답)** 를 만들 수 있다. 파일별 mock 네임스페이스 격리 또는 `beforeEach(mock.restore)` 도입 여부를 먼저 정해야 한다.
- e2e 스펙을 14개 이상으로 늘리면 `docs/quality-assurance/2026-08-25-d39-e2e-pilot-run.md` §C8 의 스위트 후반 연쇄 실패(9번째 테스트 전후, `window.__TAURI_INTERNALS__` 미로드)가 그대로 재발한다. 서버 무죄가 확정됐고 원인은 Vite HMR WS 재시도로 인한 webkit 프로세스 열화이므로, 스펙 추가 전에 HMR 비활성 또는 테스트당 브라우저 재기동을 선행하지 않으면 **신규 스펙이 앱 결함이 아닌 하네스 결함으로 빨간불**을 낸다.
- e2e 신규 스펙이 `editorFontFamily`·`uiFontFamily`·`terminalFontFamily`·`keymapOverrides` 를 건드리면 `globalTeardown` 이 원복하지 못한다(하네스 문서 §7 — `buildFullRestorePatch` 가 `Option<T>` 를 `null` 로 되돌릴 수 없음). 사용자 설정이 영구 오염된다.
- `infra/archive.rs` 테스트는 실제 zip 을 디스크에 쓰고 해제한다. 128MB 예산 초과 케이스를 **진짜 128MB 데이터로** 작성하면 CI 시간·디스크를 잡아먹는다. 예산 상수를 테스트에서 주입 가능하게 바꾸려면 프로덕션 시그니처 변경이 필요하고, 그러지 않으려면 압축률 높은 반복 데이터로 우회해야 한다 — 어느 쪽이든 설계 판단이 선행한다.
- P2 의 Rust 작업은 다른 워크플로가 지금 편집 중인 `src-tauri/src/domain/{layout,file,search,settings,window}` 및 `lib.rs` 와 겹친다. `docs/agent-operations.md` §1 의 "Rust 는 한 시점 한 에이전트" 규칙상 배치 3 구현 wf 완료 후에만 착수해야 하며, 그 전에 시작하면 파일 경합이 발생한다. 이 조사 중에도 `docs/backlog.md` 가 디스크에서 변경됐다.
- happy-dom/RTL 은 WKWebView 가 아니다. 감사가 반복 확인한 결함 클래스(B13 IME keyCode 229 전면 사망, 클립보드/`keyboard.type()` 콘텐츠 손상, WebKit #269922·#274700)는 **어떤 DOM 하네스로도 재현되지 않는다.** 하네스 도입 후 "컴포넌트 테스트 그린"을 실기·e2e 통과와 동일시하면 오히려 방심을 부른다 — 하네스의 커버 범위를 문서에 명시해야 한다.
- CI 게이트(`ci.yml`)를 신설하면 지금까지 태그에서만 돌던 검증이 매 push 에서 돌아, 이미 dev 에 있던 잠복 실패가 한꺼번에 드러날 수 있다. 또한 macOS 러너 사용 시 GitHub Actions 분당 과금이 10배라 비용이 늘어난다(프론트 job 의 ubuntu 전환은 감사 §3 에서 이미 제안된 결정 항목).
- 커버리지 임계값 게이트를 도입하면 `.tsx` 172개가 0%로 잡혀 전체 수치가 44% 대에서 시작한다. 임계값을 현실적으로 잡지 않으면 CI 가 상시 빨간불이 되고, 반대로 파일 필터를 걸면 지표가 왜곡된다.

## questionsForUser
- **컴포넌트/훅 테스트 하네스 도입 여부** — ① **happy-dom 단독**(추천: 직접 의존성 1개·전이 7개·8.57MB, React 19 가 `act` 를 자체 제공하므로 능력은 RTL 과 동일, 렌더/쿼리 헬퍼 80줄 자작) / ② happy-dom + @testing-library/react + @testing-library/dom(직접 3개·전이 약 13개·약 11.3MB — 대신 `frontend.md §11`·`docs/tech-stack.md:14` 의 기존 기재와 정합, Radix/shadcn 작성 속도 우위) / ③ 미도입(신규 의존성 0 원칙 유지 — features 96 + widgets 57 컴포넌트는 e2e 로만 커버). 전부 devDependency 이며 앱 번들에는 들어가지 않습니다.
- **push/PR CI 게이트 신설 여부** — 현재 테스트는 `release.yml` 의 태그 push 에서만 실행되고 dev·main push·PR 에는 어떤 검증도 없습니다. ① **신설(추천)**: `push`(dev·main) + `pull_request` 에 프론트 job(typecheck·lint·format:check·bun test·typecheck:e2e)은 ubuntu, Rust job(fmt·clippy·test)은 macOS / ② 프론트 job 만 신설 / ③ 현행 유지(로컬 `bun run verify` 규율에 의존).
- **착수 범위** — ① **P0+P1+P5(추천)**: 순수 모듈 + entities 데이터 레이어 + CI·문서, 약 125~165 케이스, 신규 의존성 0, 배치 3 과 파일 충돌 없음 / ② P0~P3 전량(Rust·e2e 포함 — Rust 는 배치 3 완료 후 직렬) / ③ P0~P4 전량(하네스 승인 전제).
- **e2e 확장 여부와 선결 처리** — 스펙을 14개 이상으로 늘리려면 파일럿 C8(스위트 후반 열화)을 먼저 처리해야 합니다. ① **원격 페이지 HMR 클라이언트 비활성(추천: dev 전용 현상의 근본 원인, 프로덕션 무영향)** / ② `playwright.config.ts` 를 테스트당 브라우저 재기동으로 전환(느려지지만 앱 무변경) / ③ e2e 는 13스펙에서 동결하고 단위 테스트만 보강.
- **커버리지 게이트 도입 여부** — `bun test --coverage --coverage-threshold` 로 임계값을 걸 수 있습니다. ① **미도입(추천: 현 실질 44% 에서 임계값이 의미를 갖기 어렵고, 파일 필터를 걸면 지표가 왜곡됨)** / ② 신규·수정 파일 한정 임계값 / ③ 전체 라인 임계값(값 지정 필요).
- **`mock.module` 전역 누수 대응** — P1 에서 `.query.test.ts` 를 10여 개 추가하면 스텁이 프로세스 전역으로 남습니다. ① **각 테스트 파일에 `beforeEach(mock.restore)` 도입(추천)** / ② 현행 유지(파일 수가 늘어도 지금처럼 스텁 누적 허용) / ③ 별도 `bun test` 샤딩으로 격리.

## newDependencies
- (조건부 — questionsForUser 1 에서 ①안 채택 시에만) happy-dom@20.14.0 — devDependency. 근거: bun test 에는 DOM 전역이 없어 `react-dom/client` 의 `createRoot` 가 동작하지 않는다. React 19.2.8 은 `act` 를 자체 export 하므로(`node_modules/react/cjs/react.development.js` 의 `exports.act` 실측) 훅·컴포넌트 테스트에 부족한 것은 DOM 전역 하나뿐이다. 표준/기존 도구로 불가능한 이유: 이미 설치된 `react-dom/server`(renderToStaticMarkup)로는 렌더까지만 되고 `useEffect`·이벤트·ref·포커스가 전부 죽어, 감사 §4-B 가 실제로 찾아낸 결함 클래스(A1 effect 순서 역전, A6 재부착, A7 저장 정착, D1 포커스 복원)를 한 건도 재현하지 못한다(bun -e 로 렌더 동작 실측 확인). MIT, 전이 의존 7개(ws·entities·@types/ws·@types/node·whatwg-mimetype·buffer-image-size·@types/whatwg-mimetype), unpacked 8.57MB, 앱 번들 무영향.
- (조건부 — questionsForUser 1 에서 ②안 채택 시에만) @testing-library/react@16.3.3 — devDependency. 근거: `frontend.md §11`("Testing Library 로 사용자 관점(역할·라벨 쿼리)")과 `docs/tech-stack.md:14` 의 기존 기재가 이미 이 라이브러리를 전제하고 있고, Radix/shadcn 기반 UI 는 역할·라벨 쿼리가 구조 결합(querySelector)보다 회귀에 강하다. 표준/기존 도구로 불가능한 이유: happy-dom 만으로는 `screen`/`getByRole`/`fireEvent`/자동 cleanup 이 없어 약 80 LOC 를 자작해야 하며, 자작 시 접근성 기반 쿼리가 빠져 컨벤션을 못 지킨다. MIT, deps 1(@babel/runtime@^7 — 기존 devDependency @babel/runtime@8.0.0 과 별도 사본이 설치됨), unpacked 0.34MB.
- (조건부 — questionsForUser 1 에서 ②안 채택 시에만, 위 항목의 peerDependency 로 필수) @testing-library/dom@10.4.1 — devDependency. 근거: @testing-library/react v16 부터 이것이 peerDependency 로 분리되어 명시 설치가 필요하다(`bun pm view @testing-library/react peerDependencies` 실측: `"@testing-library/dom": "^10.0.0"`). MIT, 전이 의존 8개(lz-string·aria-query·picocolors·pretty-format@27·@babel/runtime@7·@babel/code-frame·@types/aria-query·dom-accessibility-api), unpacked 2.43MB.


# 주제 1 — OS 네이티브 알림

## findings
## 0. 이미 확정된 결정 (중복 방지)

이 주제는 **이미 사용자 결정이 내려져 있다** — `docs/acknowledge/2026-09-04-usability-batch4-user-decisions.md` §결정 1:

> **알림 정책 — 비활성 창 + 완료성 이벤트만.** 창이 포커스를 잃었을 때만, 에이전트·태스크·git 원격 작업·검색 치환·LSP 설치 완료와 error 를 OS 알림으로 보낸다. 포커스 중에는 기존 sonner 토스트 유지. 설정에서 카테고리별 on/off. **공식 `tauri-plugin-notification` 신규 의존성 도입 승인.** 권한 거부 시 macOS 시스템 설정의 알림 창을 바로 연다.

`docs/PROCESS.md` "대기: 사용성 배치 4" 절의 체크리스트 a-① 가 바로 이 조사다. `docs/backlog.md` 에는 알림 관련 항목이 **하나도 없다**(grep 0건) — 백로그 중복 없음.

---

## 1. (a) 현재 토스트 인프라

- **Toaster 마운트 1곳**: `src/widgets/app-toaster/app-toaster.tsx:1-22` — `AppProviders` 안(`src/app/providers/app-providers.tsx:15`)에서 **모든 창(main·auxiliary·remote)** 이 각자 렌더한다. `settings.toastPosition` + `theme.type` 만 소비.
- **위치 상수**: `src/shared/constants/toast.ts` — 9분할(`{top,middle,bottom}×{left,center,right}`), sonner 6종 매핑 + `middle-*` 는 CSS(`MIDDLE_TOAST_CLASS`). 정본 문서는 `docs/features/settings-ui.md` §2.
- **호출부 실측**: `toast.{success|error|info|warning|message|loading|custom|promise|dismiss}` **153건 / 55파일**(`grep -rno` 기준. 프롬프트의 "152곳" 과 사실상 동일).
- **분포**: `widgets/settings-view` 8파일, `widgets/editor-pane` 5, `widgets/plugin-manager` 4, `widgets/git-panel` 3, `app/providers` 3, entities 8슬라이스(`git`·`sync`·`agent`·`remote`·`search`·`project`·`layout`·`settings`), `shared/lib`(command-catalog·lsp/adapters/code-action) 2, `features` 3.
- **성격 분류(실측)**: 대략 **100건이 `toast.error(describeIpcError(error))` 형태의 IPC 실패 보고**, 나머지가 성공/정보. sonner 는 `toast` 함수 객체 + `useSonner()` / `toast.getHistory()` / `getToasts()` 만 공개하고 **전역 구독 콜백 API 가 없다**(`node_modules/sonner/dist/index.d.ts:130-149`). 게다가 `title`/`description` 이 `React.ReactNode` 라(예: `src/widgets/search-panel/search-panel-container.tsx:96-107` 의 JSX description) 토스트 가로채기로는 네이티브 알림 텍스트를 손실 없이 추출할 수 없다.

### 완료성 이벤트 5카테고리의 실제 발생 지점

| 카테고리 | 현재 지점 | 비고 |
|---|---|---|
| 에이전트 완료 | `agent:state-changed` 이벤트(`src-tauri/src/events.rs:132-138`) → `AgentStateSyncProvider`(`src/app/providers/agent-state-sync-provider.tsx`). `AgentActivity::{Idle,Working,AwaitingInput,Unknown}`(`src-tauri/src/domain/agent/types.rs:50-56`) | **완료 토스트가 지금 존재하지 않는다.** `Working→Idle`/`AwaitingInput` 전이를 새로 판정해야 한다. 이 프로바이더는 **main·auxiliary 양쪽에 마운트**된다(`src/app/app.tsx`) — 창 수만큼 중복 발화 위험 |
| 태스크 러너 | `src/widgets/task-runner/task-runner-dialog.tsx:35-36` 이 `requestEditorPaneCommand({type:'run-in-terminal'})` 로 **터미널에 문자열을 던질 뿐** 완료 신호가 없다 | 완료 신호 확보 경로는 이미 있음: **OSC 133 블록 트래커**(`src/features/terminal/terminal-osc133.ts` — `applyOsc133Event` 가 `D` 에서 `exitCode` 확정, `attachOsc133BlockTracker` 를 `src/features/terminal/terminal-view.tsx:309` 가 마운트) |
| git 원격 | `usePushGit`/`usePullGit`(`src/entities/git/git.query.ts:189-191`), `handleSync`(`src/widgets/git-panel/git-panel-container.tsx:194`) | 현재 **성공 토스트가 없다**(에러만 `notifyError`). fetch 전용 커맨드는 없음 |
| 검색 치환 | `src/widgets/search-panel/search-panel-container.tsx:93` `toast.success('search.replaceDone')` + `:96` 스킵 경고 | 완료 토스트 기존재 |
| LSP 설치 | `lsp:install-progress` 이벤트 → `src/entities/lsp/lsp.query.ts:41-58`(`useLspInstallProgressSync`). 마운트는 `src/widgets/settings-view/settings-view.tsx:120` | 설정 탭이 열린 창에서만 구독 — 여러 창이 설정 탭을 열면 중복 |

## 1-b. 창 포커스를 아는 기존 코드

- `src/app/query-client.ts:20-27` — `focusManager.setEventListener((handleFocus) => getCurrentWindow().onFocusChanged(({payload}) => handleFocus(payload)))`. 즉 **TanStack Query 의 `focusManager.isFocused()` 가 이미 이 창의 포커스 상태를 동기적으로 들고 있다**(`node_modules/@tanstack/query-core/build/modern/focusManager.js` — `setFocused` → `#focused`, `isFocused()` 반환). 새 구독이 필요 없다.
- 단 함정 2개: (1) `onFocusChanged` 는 **변화시에만** 발화하므로 부팅 직후 `#focused` 는 `undefined` → `document.visibilityState !== 'hidden'` 폴백으로 **항상 true**. (2) 창마다 별개 JS realm이라 **"보조 창이 포커스 중인데 메인 창은 자기가 비포커스라고 판단"** 한다. "앱 전체가 비포커스" 판정은 Rust 에서만 정확하다 — `Manager::webview_windows()`(`tauri-2.11.5/src/lib.rs:588`) + `WebviewWindow::is_focused()`(`tauri-2.11.5/src/webview/webview_window.rs:1744`).
- 권한: `is_focused` 는 tauri **core default 에 포함**(`tauri-2.11.5/build.rs:56` `("is_focused", true)`) — capabilities 변경 불필요.

---

## 2. (b) 구현 경로 — 소스 확증

현재 `src-tauri/Cargo.toml`·`package.json`·`src-tauri/capabilities/main.json` 에 **알림 관련 의존성/권한은 0건**. (`Cargo.lock` 의 `objc2-user-notifications 0.3.2` 는 `objc2-ui-kit`(iOS) 의 전이 의존일 뿐 macOS 빌드에 들어가지 않는다 — `Cargo.lock:2868`.)

### B-1. `tauri-plugin-notification` 2.4.0 (최신) — 소스 인용

- **데스크톱 권한 API 는 전부 스텁이다.** `src/desktop.rs:61-67`:
  ```rust
  pub fn request_permission(&self) -> crate::Result<PermissionState> { Ok(PermissionState::Granted) }
  pub fn permission_state(&self)  -> crate::Result<PermissionState> { Ok(PermissionState::Granted) }
  ```
  `src/commands.rs:9-27` 의 `is_permission_granted`/`request_permission` 이 이 값을 그대로 반환 → **macOS 에서 항상 `granted`**. ⇒ **플러그인만으로는 "권한 거부"를 절대 감지할 수 없다.** (결정 문서의 "권한 거부 시 …" 를 플러그인 API 로 구현하는 것은 불가능.)
- **전달 실패도 삼킨다.** `src/desktop.rs:216-218`: `tauri::async_runtime::spawn(async move { let _ = notification.show(); })` — 결과를 버린다. 프론트는 성공/실패를 알 수 없다.
- **macOS dev 제약은 실재한다.** `src/desktop.rs:207-214`:
  ```rust
  #[cfg(target_os = "macos")]
  { let _ = notify_rust::set_application(if tauri::is_dev() { "com.apple.Terminal" } else { &self.identifier }); }
  ```
  `tauri::is_dev()` = `!cfg!(feature="custom-protocol")`(`tauri-2.11.5/src/lib.rs:308-310`) → `bun run tauri dev` 는 **알림이 "Terminal" 이름·아이콘으로 뜬다**(= Terminal.app 의 알림 권한을 따른다). 프로덕션 번들에서만 `net.gumyo.taide` 로 뜬다.
- **왜 그런가(하위 스택)**: `notify-rust`(macOS) → `mac-notification-sys`. `set_application` 은 `objc/notify.m:17-30` 에서 `LSCopyApplicationURLsForBundleIdentifier` 로 **LaunchServices 에 등록된 번들만** 허용하고, 아니면 `NO` 를 반환한다. 미번들 dev 바이너리는 자기 identifier 를 쓸 수 없어서 플러그인이 `com.apple.Terminal` 로 스푸핑하는 것이다. 또 `src/lib.rs:147-158` 의 `set_application` 은 `Once` 라 **프로세스당 1회만** 유효하다.
- **백엔드 API 세대**: `notify-rust` 기본 경로는 `NSUserNotificationCenter`(deprecated) 다 — `notify-rust-4.18.0/src/macos/mod.rs:1-8`, `mac-notification-sys-0.6.15/objc/notify.m:79-99`. 현대 `UNUserNotificationCenter` 는 `preview-macos-un` feature 뒤에 있고 플러그인은 그 feature 를 켜지 않는다.
- **JS 게스트 패키지는 `window.Notification` 에 의존한다.** `@tauri-apps/plugin-notification@2.4.0` `dist-js/index.js:73-78, 95-96, 116-122` 가 전부 `window.Notification.*` 를 읽는다. 그 전역은 **플러그인의 `js_init_script`(`src/lib.rs:233-236`, `src/init-iife.js`)가 주입**한다.
- **주의(비자명 결함)**: 그 init 스크립트는 로드 시 무조건 `invoke('plugin:notification|is_permission_granted')` 를 호출하고 `.then(...)` 만 붙인다(`.catch` 없음). capability 에 `notification:allow-is-permission-granted` 를 주지 않으면 **모든 창에서 매 부팅마다 unhandled rejection** 이 나고, 그것이 `src/shared/lib/error-log-forwarding.ts:91` 의 `window.addEventListener('unhandledrejection', ...)` 로 **파일 로그에 error 로 기록**된다.
- **의존성 증가는 작다**: 신규 크레이트는 `tauri-plugin-notification` / `notify-rust` / `mac-notification-sys` 3개뿐. 그 하위(`rand`·`serde_repr`·`time`·`url`·`cc`·`objc2`·`objc2-foundation`·`uuid`)는 **이미 `Cargo.lock` 에 존재**한다. `zbus` 계열은 `cfg(all(unix, not(macos)))` 타깃 게이트라 macOS 빌드에 안 들어간다.
- **자립성 게이트 통과 전망**: `mac-notification-sys` 는 `build.rs` 로 ObjC 를 정적 컴파일하고 `framework=AppKit` 만 링크한다 → `docs/deployment.md` §8 "non-system dylib linkage" 게이트에 걸릴 외부 dylib 없음.
- **최소 권한**: 데스크톱에서 실제 존재하는 플러그인 커맨드는 `notify`·`request_permission`·`is_permission_granted` 3개뿐. `notification:default` 는 16개를 통째로 열므로(`permissions/default.toml`) NFR-7 위반. `notification:allow-notify` / `allow-is-permission-granted` 개별 지정이 가능하다(`permissions/autogenerated/commands/notify.toml`).
- **ACL 은 Rust 호출을 막지 않는다** — `NotificationExt` 로 Rust 에서 부르면 capability 항목이 아예 필요 없다(위 init 스크립트 문제 때문에 `allow-is-permission-granted` 1개만은 주는 편이 낫다).

### B-2. Rust 크레이트 직접(`notify-rust` / `mac-notification-sys`)

- 얻는 것: 플러그인 계층(그리고 `window.Notification` 주입·init 스크립트 문제)을 건너뜀. 잃는 것: 사용자가 승인한 "공식 플러그인" 결정에서 이탈, dev 스푸핑 로직을 직접 재구현해야 함. 권한 감지 능력은 **동일하게 없다**(NSUserNotificationCenter 경로에는 권한 질의 API가 없다).
- 권한을 진짜로 알려면 `UNUserNotificationCenter` 가 필요하다: `mac-usernotifications 0.3.1` 이 `AuthorizationStatus::{NotDetermined,Denied,Authorized,…}`(`src/settings.rs:11-25`)와 `get_notification_settings`(`src/auth.rs:103-106`)를 제공한다. 단 (1) `check_bundle()` 이 `NSBundle.mainBundle().bundleIdentifier()` 를 요구해(`src/lib.rs:239-244`) **미번들 dev 에서 `Error::NoBundleIdentifier`**, (2) `edition = "2024"`, `rust-version = "1.90"` — 현재 `src-tauri/Cargo.toml` 의 `rust-version = "1.80"`·`edition = "2021"` 과 충돌, (3) 사용자가 승인한 의존성이 아니다.

### B-3. Web Notification API (WKWebView)

- **불가.** `wry-0.55.1` 의 `WKUIDelegate` 구현(`src/wkwebview/class/wry_web_view_ui_delegate.rs`)에는 `runOpenPanel` 과 `WKMediaCaptureType` 권한만 있고 **notification 권한 델리게이트가 없으며**, `src/wkwebview/mod.rs` 의 `WKPreferences` 설정 목록(라인 340~549)에도 notification 관련 키가 없다. 즉 데스크톱 웹뷰에 네이티브 `window.Notification` 은 없고, 있는 것처럼 보이면 그건 위 플러그인 init 스크립트가 주입한 shim 이다.

---

## 3. (c) macOS 시스템 설정 알림 창 열기

**이 기기(macOS 26.5.2 / 25F84)에서 확인한 사실:**

- `/System/Library/ExtensionKit/Extensions/NotificationsSettings.appex/Contents/Info.plist`
  - `CFBundleIdentifier = com.apple.Notifications-Settings.extension`
  - `EXAppExtensionAttributes.SettingsExtensionAttributes.allowsXAppleSystemPreferencesURLScheme = true`
  - `legacyBundleIdentifier = com.apple.preference.notifications`
- `/System/Applications/System Settings.app/Contents/Info.plist` → `CFBundleURLSchemes = ["x-apple.systempreferences"]`, `LSIsAppleDefaultForScheme = true`, 번들 id `com.apple.systempreferences`.

⇒ **`x-apple.systempreferences:com.apple.Notifications-Settings.extension`** 이 이 OS 버전에서 유효한 형태다.

**기존 `open_url` 로는 열 수 없다.** `src-tauri/src/domain/system/commands.rs:221-296` 의 `validate_external_url` 이 `EXTERNAL_URL_ALLOWED_SCHEMES = ["http://","https://"]` 만 허용하고, 그 doc 주석이 "generic open-anything primitive 로 만들지 않기 위한" 것이라고 명시한다. ⇒ **화이트리스트를 넓히면 안 되고, URL 을 하드코딩한 전용 커맨드**를 신설해야 한다.

전달 경로 자체는 문제없다: `tauri_plugin_opener::open_url`(`tauri-plugin-opener-2.5.4/src/open.rs:33-36`) → `open` 크레이트 `that_detached` → macOS 는 `Command::new("/usr/bin/open").arg("--").arg(path)`(`open-5.4.1/src/macos.rs:3-7`). `--` 뒤라 플래그 오인 없이 LaunchServices 가 스킴을 해석한다.

---

## 4. (d) 번들 식별자 · 서명 상태

- `src-tauri/tauri.conf.json`: `identifier = "net.gumyo.taide"`, `productName = "TAIDE"`, 창 라벨 `main`(보조 창은 `editor-<n>`).
- `docs/deployment.md` §5: **CI 가 서명 + 공증까지 수행**(팀 `SN98P5V7J4`, secrets 5건 등재 완료). §9 릴리스 이력 v0.1.6 완주.
- ⇒ **프로덕션 dmg 는 정상 번들이라 `LSCopyApplicationURLsForBundleIdentifier("net.gumyo.taide")` 가 성공**하고 TAIDE 이름·아이콘으로 알림이 뜬다. 문제는 dev 뿐이며, 플러그인이 이미 `com.apple.Terminal` 스푸핑으로 우회한다(§2 B-1).
- 알림 제목 기본값은 `self.app.config().product_name` = `"TAIDE"`(`desktop.rs:29-34`).

---

## 5. (e) 설정 구조 선례

- `Settings` 는 플랫 스칼라 필드 + `#[serde(default …)]` 구조(`src-tauri/src/domain/settings/types.rs:77-290`), `SettingsPatch` 는 전부 `Option<T>`(`:295-`). d-53 선례가 그대로 여기 있다(`editor_bracket_pair_guides`·`explorer_auto_reveal`·`trim_trailing_whitespace_on_save` 등 — 전부 `bool` 1필드 + `SwitchField` 1줄).
- 쓰기 경로는 **단일**: `settings::commands::apply_and_broadcast`(`src-tauri/src/domain/settings/commands.rs:45-63`) → `SettingsToggleObservers`(`:21-37`, 조립부가 등록하는 확장점) → `SettingsChanged` 이벤트.
- UI: `SETTINGS_SECTION_ID` 12종 + `SETTINGS_TOC_ITEMS`(`src/widgets/settings-view/settings-view.tsx:56-84`), 섹션 파일 1개 = 1섹션(`settings-*-section.tsx`), 토글은 `SwitchField`(`src/widgets/settings-view/settings-interface-section.tsx:46-60` 패턴).
- 저장 실패는 호출부가 아니라 `entities/settings/settings.query.ts:25` 의 공통 `onError` 가 보고한다 — 새 토글에서 또 토스트하면 안 된다.
- i18n 은 **Rust 가 서빙하는 플랫 dot-key JSON 3개**(`src-tauri/resources/locales/{ko,en,ja}.json`, 현재 968키). `settings.*` 네임스페이스에 키를 3언어 동시 추가해야 한다.

## 6. (f) 원격 미러

- 원격 미러는 라벨 `remote` 로 식별된다(`src/shared/lib/remote/tauri-internals-shim.ts:20` `REMOTE_WINDOW_LABEL`, `src/shared/lib/remote/runtime-environment.ts:23`).
- 원격 dispatch 는 **기본 거부**이고, `REMOTE_ALLOWED_COMMANDS ⊎ REMOTE_DENIED_COMMANDS` 가 전체 커맨드 집합과 **정확히 일치해야 하는 기계 강제 테스트**가 있다(`src-tauri/src/domain/remote/dispatch.rs:1735-1770`). ⇒ **신규 커맨드는 반드시 둘 중 하나에 등재**해야 하고, 미등재면 `cargo test` 가 깨진다.
- 정확히 맞는 기존 분류가 있다: `RemoteDenialPolicy::UnreachableDesktopWindow`(`dispatch.rs:249-260`) — "desktop 자기 화면에 창/다이얼로그/앱을 띄우는 커맨드". `system_open_external_url` 이 이미 이 정책으로 거부된다(`dispatch.rs:519`).
- 플러그인 커맨드(`plugin:notification|notify`)는 `IMPLEMENTED_JSON_COMMANDS` 집합 밖이라 dispatch 화이트리스트에 없다 → 원격에서 호출하면 자동 거부된다. 즉 **미러가 데스크톱 알림을 쏘는 사고는 구조적으로 차단**돼 있다.
- 미러는 진짜 브라우저라 네이티브 `Notification` 이 존재하지만, `installRemoteInternalsShim` 은 플러그인 init 스크립트를 받지 않으므로 shim 도 없다. 브라우저 알림을 쓰려면 별도 코드 + HTTPS 보안 컨텍스트가 필요하다(현재 원격 서버는 127.0.0.1 평문 + 리버스 프록시 전제 — `docs/deployment.md` §7).

## 7. 멀티 윈도우 중복 발화 (설계상 가장 큰 함정)

`src/app/app.tsx` 는 main / auxiliary 두 갈래로 프로바이더 트리를 나누는데, `AgentStateSyncProvider`·`IpcSyncProvider`·`HotExitFlushProvider` 는 **양쪽 모두**에 마운트된다. `agent:state-changed`·`terminal:exited`·`lsp:install-progress` 는 전 창 브로드캐스트다. 프론트에서 "완료 판정 → 알림" 을 하면 **열린 창 수만큼 OS 알림이 중복**된다. `IdeSyncProvider` 가 main 전용인 이유가 정확히 이 레이스이고(`app.tsx` doc 주석), 같은 처방이 필요하다.


## options
## A안 (추천) — Rust 가 게이트를 소유하고, 프론트는 카테고리 태그만 넘기는 `notify()` 파사드

**데이터 흐름**

```
[FE 발생지 ~8곳]  notifyNative({category, titleKey, args})   ← shared/lib/notify.ts (얇은 파사드)
        │            (기존 toast.* 153곳은 그대로 둔다)
        ▼  invoke
[Rust] notification_notify(category, title, body)
        ├ settings 카테고리 토글 OFF        → Suppressed{reason: category-off}
        ├ notifications_only_when_unfocused && 어느 창이든 is_focused() → Suppressed{reason: focused}
        └ 통과 → app.notification().builder().title().body().show()
[Rust] notification_open_system_settings()  → opener::open_url("x-apple.systempreferences:com.apple.Notifications-Settings.extension")
```

- **파사드**(`src/shared/lib/notify.ts`): `notifyNative(input)` 는 (1) `getWindowContext().kind === 'main'` 이 아니거나 `isRemoteMirrorRuntime()` 이면 즉시 반환(중복·미러 차단), (2) `focusManager.isFocused()` 로 1차 조기 컷(IPC 절약), (3) `invoke`. **`toast` 는 건드리지 않는다** — 기존 153곳 무수정.
- **카테고리 열거**: Rust `NotificationCategory { AgentCompleted, TaskCompleted, GitRemote, SearchReplace, LspInstall, Error }` (`as const` union 로 TS 생성). 커맨드 인자로 타입 고정.
- **설정**: `Settings` 에 `notifications_enabled`(default true) · `notifications_only_when_unfocused`(default true) · 카테고리별 bool 6개. `SettingsPatch` 에 동일 `Option` 필드. 새 `Notifications` 섹션 = `SwitchField` 8개 + "알림 설정 열기" 버튼 + "테스트 알림" 버튼.
- **완료 판정 위치**: main 창 전용 신규 `NativeNotificationProvider` 가 `agent:state-changed` 의 `Working→Idle|AwaitingInput` 전이와 `lsp:install-progress` 종료를 감시. 태스크는 `terminal-osc133.ts` 의 `D`(exitCode) 이벤트에 콜백 훅을 하나 추가. git 원격·검색 치환은 기존 mutation `onSuccess`/`onError` 에 `notifyNative` 한 줄 추가.
- **원격**: 두 신규 커맨드 모두 `REMOTE_DENIED_COMMANDS` + `UnreachableDesktopWindow`.
- **capabilities**: `notification:allow-is-permission-granted` **1개만** 추가(init 스크립트 unhandled rejection 방지). `notify` 는 Rust 내부 호출이라 ACL 불필요.
- 장점: 포커스 판정이 **앱 전체 기준으로 정확**(보조 창 포커스 중 오발화 없음), 중복 원천 차단, 게이트 로직이 `service.rs` 순수 함수라 `bun test` 없이 `cargo test` 로 전수 검증 가능, IPC 계약이 specta 단일 출처 유지, 원격 정책 테이블에 자연스럽게 편입.
- 단점: Rust 커맨드 2개 + 도메인 1개 신설(architecture.md 24→25 갱신), 프론트에서 "실제로 떴는지" 는 `NotificationDelivery` 반환값으로만 알 수 있음(플러그인이 실패를 삼키므로 그 이상은 원리적으로 불가).

**변경 파일**: `src-tauri/{Cargo.toml, src/lib.rs, src/domain/mod.rs, src/domain/notification/*, src/domain/settings/{types,service}.rs, src/domain/remote/dispatch.rs, capabilities/main.json, resources/locales/*.json}`, `src/{shared/lib/notify.ts, entities/notification/*, app/providers/native-notification-provider.tsx, app/app.tsx, widgets/git-panel/git-panel-container.tsx, widgets/search-panel/search-panel-container.tsx, entities/lsp/lsp.query.ts, features/terminal/terminal-osc133.ts, widgets/settings-view/*}`

---

## B안 — 프론트 전담 (`@tauri-apps/plugin-notification` JS API 직접 호출)

**데이터 흐름**: 발생지 → `shared/lib/notify.ts` → `sendNotification({title, body})`(npm 패키지) → `invoke('plugin:notification|notify')`. 설정 게이트·포커스 판정 전부 프론트.

- 장점: Rust 변경이 `Cargo.toml` + `lib.rs` 플러그인 등록 2줄 + 설정 필드뿐. 구현량 최소.
- 단점(치명적 3개):
  1. 포커스 판정이 **창 단위**라 보조 창이 포커스 중일 때 메인 창이 알림을 쏜다(§7·§1-b).
  2. 브로드캐스트 이벤트 기반 카테고리가 **창 수만큼 중복 발화** — 창 라벨 게이트를 프론트에서 손으로 지켜야 하고 기계 강제가 없다.
  3. npm 의존성 `@tauri-apps/plugin-notification` 이 추가되고, capability 를 `notification:allow-notify` 까지 열어야 하며, 원격 미러에서 `window.Notification` shim 이 없어 `sendNotification` 이 조용히 `TypeError` 를 던진다(가드 필요).
- 게이트 로직이 TS 에 흩어져 `dispatch.rs` 같은 기계 강제 테이블의 보호를 못 받는다.

---

## C안 — sonner 전역 인터셉트 (호출부 0곳 수정)

**데이터 흐름**: `AppToaster` 안에서 `useSonner()` 로 토스트 배열을 구독 → 새로 추가된 토스트를 카테고리 추론 후 네이티브로 복제.

- 장점: 발생지 코드를 **정말로 한 줄도 안 건드린다**.
- 단점(권장 불가):
  1. **카테고리를 알 수 없다.** 153건 중 어느 것이 "git push 완료"인지 토스트 텍스트로 추론해야 한다 — 결정 문서의 "카테고리별 on/off" 를 구현할 방법이 없다.
  2. `title`/`description` 이 `ReactNode` 라 텍스트 추출이 손실적이다(`search-panel-container.tsx:96-107` 의 JSX description).
  3. 사용자 결정("완료성 이벤트 + error 만")과 정반대로 **모든 토스트가 새어나간다**.
  4. `AppToaster` 가 전 창에 마운트되므로 §7 중복이 그대로.
- 유일한 유효 용도: A안의 `Error` 카테고리를 "전역 에러 토스트 미러" 로 넓히고 싶어질 때의 폴백. 이번 범위에서는 제외.


## recommendation
**A안 (Rust 소유 게이트 + 얇은 FE 파사드)**.

이유:

1. **결정 문서의 3요건("비포커스일 때만" · "카테고리별 on/off" · "권한 거부 시 설정 창")을 전부 정확히 만족하는 유일한 안이다.** 특히 "비포커스" 는 앱 전체 기준이어야 하는데, 그 판정은 `webview_windows().values().any(is_focused)` 로 Rust 에서만 정확하다 — B안은 보조 창이 포커스 중일 때 오발화한다.
2. **중복 발화를 구조로 막는다.** 알림 트리거가 브로드캐스트 이벤트(agent/terminal/lsp) 기반이라 프론트 전담(B안)은 창 수만큼 중복되고, 그 방어를 사람이 지켜야 한다. A안은 파사드 1곳의 `windowContext.kind === 'main'` 가드 + Rust 단일 진입점 두 겹으로 막는다.
3. **기존 기계 강제 체계에 자연 편입.** 신규 커맨드는 `REMOTE_ALLOWED/DENIED` 분할 테스트(`dispatch.rs:1735`)가 등재를 강제하고, 게이트 판정은 `domain/notification/service.rs` 의 순수 함수라 `cargo test` 로 전 조합을 검증할 수 있다(배치 4 요구사항 "테스트 대폭 보강" 과 정합).
4. **최소 권한 유지.** Rust 에서 `NotificationExt` 를 호출하면 ACL 을 타지 않아 capabilities 에 `notification:allow-is-permission-granted` 1개만 추가된다(그것도 init 스크립트 unhandled rejection 방지 목적). npm 의존성 0.
5. **토스트 호출부 153곳 전부 무수정.** 실제로 손대는 발생지는 5카테고리 × 성공/실패 ≈ **8~12곳**뿐이다.

**단, "권한 거부 감지" 는 A안으로도 불가능하다.** 플러그인 데스크톱 `permission_state()` 가 무조건 `Granted` 를 반환하고(`desktop.rs:65-67`) 전달 실패도 삼키기 때문이다(`desktop.rs:216`). 따라서 권장 UX 는 **감지 대신 상시 제공**이다:

- 설정 → Notifications 섹션에 **"테스트 알림 보내기"** + **"macOS 알림 설정 열기"** 버튼을 항상 노출한다.
- 첫 네이티브 알림을 보낸 세션에서 한 번만, 토스트로 "알림이 안 보이면 시스템 설정에서 TAIDE 알림을 켜세요" + `action` 버튼(설정 창 열기)을 띄운다(sonner `action` 은 이미 `use-explorer-entry-crud.ts` 에서 쓰는 패턴).
- dev 실행에서는 알림이 **Terminal 이름으로** 뜬다는 사실을 그 섹션 힌트 문구와 `docs/features/settings-ui.md` 에 명기한다.


## filesToTouch
- src-tauri/Cargo.toml
- Cargo.lock
- src-tauri/src/lib.rs
- src-tauri/src/domain/mod.rs
- src-tauri/src/domain/notification/mod.rs
- src-tauri/src/domain/notification/types.rs
- src-tauri/src/domain/notification/service.rs
- src-tauri/src/domain/notification/commands.rs
- src-tauri/src/domain/settings/types.rs
- src-tauri/src/domain/settings/service.rs
- src-tauri/src/domain/remote/dispatch.rs
- src-tauri/capabilities/main.json
- src-tauri/resources/locales/ko.json
- src-tauri/resources/locales/en.json
- src-tauri/resources/locales/ja.json
- src/shared/api/bindings.ts
- src/shared/lib/notify.ts
- src/shared/lib/notify.test.ts
- src/entities/notification/notification.ipc.ts
- src/entities/notification/notification.type.ts
- src/app/providers/native-notification-provider.tsx
- src/app/providers/native-notification-provider.test.ts
- src/app/app.tsx
- src/features/terminal/terminal-osc133.ts
- src/features/terminal/terminal-view.tsx
- src/entities/lsp/lsp.query.ts
- src/widgets/git-panel/git-panel-container.tsx
- src/widgets/search-panel/search-panel-container.tsx
- src/widgets/settings-view/settings-notification-section.tsx
- src/widgets/settings-view/settings-view.tsx
- src/entities/settings/settings.constant.ts
- docs/features/settings-ui.md
- docs/ipc-contract.md
- docs/architecture.md
- docs/data-model.md
- docs/tech-stack.md
- docs/acknowledge/2026-09-04-usability-batch4-contract.md
- docs/PROCESS.md

## risks
- MSRV 거짓말 — `notify-rust` 4.18.0 은 `rust-version = "1.89"`, `mac-notification-sys` 0.6.15 는 `edition = "2024"`·`rust-version = "1.85"` 인데 `src-tauri/Cargo.toml` 은 `rust-version = "1.80"` 이고 루트 `Cargo.toml` 은 `resolver = "2"`(MSRV 비인지)다. 로컬 1.98·CI `dtolnay/rust-toolchain@stable` 로는 빌드되지만 선언 MSRV 가 사실과 어긋난다. `src-tauri/Cargo.toml` 의 `rust-version` 을 실제값(≥1.89)으로 올려야 한다.
- 권한 거부를 감지할 수 없다 — 플러그인 데스크톱 `permission_state()` 가 항상 `Granted`(desktop.rs:65-67), `show()` 실패도 `let _ =` 로 버려진다(desktop.rs:216). 사용자가 macOS 알림을 껐을 때 앱은 '보냈다'고 판단하고 아무것도 뜨지 않는 무음 실패가 된다. 설정 창 열기는 감지 기반이 아니라 상시 노출로만 제공 가능하다.
- dev 실행에서 알림이 'Terminal' 로 뜬다 — `tauri::is_dev()` 시 `set_application("com.apple.Terminal")`(desktop.rs:207-214). Terminal.app 의 알림이 꺼져 있으면 dev 에서 알림이 아예 안 뜨고, 그것을 코드 버그로 오진할 수 있다. 실기 검증은 반드시 `tauri build` 산출물로 해야 한다.
- init 스크립트 unhandled rejection → 로그 오염 — capability 에 `notification:allow-is-permission-granted` 를 넣지 않으면 플러그인 init-iife.js 가 부팅마다 거부된 invoke 를 `.catch` 없이 호출하고, `shared/lib/error-log-forwarding.ts:91` 이 그것을 파일 로그에 error 로 기록한다(창 수 × 부팅마다).
- 멀티 윈도우 중복 발화 — `agent:state-changed`·`terminal:exited`·`lsp:install-progress` 는 전 창 브로드캐스트이고 `AgentStateSyncProvider` 는 main·auxiliary 양쪽에 마운트된다(app.tsx). 창 라벨 게이트를 빠뜨리면 알림이 창 수만큼 중복된다.
- 포커스 판정 초기값 함정 — `focusManager.isFocused()` 는 첫 `onFocusChanged` 전까지 `#focused === undefined` 라 `document.visibilityState` 폴백으로 항상 true 를 반환한다(query-core/focusManager.js). 부팅 직후 비포커스 상태에서 알림이 잘못 억제될 수 있다 — Rust 측 `webview_windows().any(is_focused)` 를 정본으로 삼아야 한다.
- deprecated API 스택 — 알림 전달은 `NSUserNotificationCenter`(macOS 11 부터 deprecated) 위에서 돌고, 번들 id 스푸핑은 `NSBundle` 스위즐링(`objc/notify.m:19` `installNSBundleHook`)이다. 미래 macOS 에서 조용히 깨질 수 있는 표면이며, 앱이 공증된 서명 바이너리라는 점에서 런타임 스위즐링은 검토 가치가 있다.
- 원격 dispatch 테이블 누락 시 빌드 실패 — `REMOTE_ALLOWED_COMMANDS ⊎ REMOTE_DENIED_COMMANDS` 가 전체 커맨드 집합과 정확히 일치해야 하는 테스트(dispatch.rs:1735-1770)가 있어, 신규 커맨드 2개를 등재하지 않으면 `cargo test` 가 깨진다. 반대로 잘못 ALLOWED 에 넣으면 원격 세션이 데스크톱 화면에 알림·시스템 설정 창을 띄울 수 있다.
- 시스템 설정 URL 은 OS 버전 의존 — `x-apple.systempreferences:com.apple.Notifications-Settings.extension` 은 이 기기(macOS 26.5.2)의 Info.plist 로 확증했지만 이전/이후 버전에서 확장 번들 id 가 바뀔 수 있다. 실패 시 조용히 아무 일도 안 일어나므로(`/usr/bin/open` 은 detached spawn) 사용자에게 폴백 안내(설정 경로 텍스트)를 함께 보여야 한다.
- 알림 폭주 — 에이전트 `Working→Idle` 전이는 짧은 턴마다 발생할 수 있다. 카테고리별 최소 소요시간 임계(예: 작업이 N초 이상 걸렸을 때만) 또는 쿨다운이 없으면 알림 센터가 도배된다. 게이트 로직에 임계값을 상수로 넣고 테스트해야 한다.
- 릴리스 자립성 게이트 — `docs/deployment.md` §8 이 명시하듯 새 네이티브 의존은 vendored/static 여부를 반드시 확인해야 한다. `mac-notification-sys` 는 ObjC 를 `cc` 로 정적 컴파일하고 `framework=AppKit` 만 링크하므로 통과 전망이지만, 실제 `otool -L` 확인 전까지는 미확증이다.

## questionsForUser
- 권한 거부 감지가 원리적으로 불가능합니다(플러그인 데스크톱 `permission_state()` 가 항상 granted, 전달 실패도 삼킴). 대안 중 어느 쪽으로 갈까요 — **A: 설정 섹션에 '알림 설정 열기'+'테스트 알림' 버튼을 상시 노출 + 첫 발송 시 1회 안내 토스트(추천, 신규 의존성 0)** / B: `mac-usernotifications` 크레이트를 추가해 `UNUserNotificationCenter` 로 실제 권한 상태를 조회(정확하지만 승인 밖 신규 의존성 1개 + edition 2024·MSRV 1.90 + 미번들 dev 에서 조회 자체가 실패)
- 설정 필드 형태 — **A: 플랫 bool 8개(`notificationsEnabled`·`notificationsOnlyWhenUnfocused`·카테고리 6개) — 기존 Settings 패턴·SwitchField 와 100% 동일(추천)** / B: `notificationDisabledCategories: Vec<NotificationCategory>` 목록 1필드(카테고리 추가 시 설정 필드가 안 늘지만 기존 패턴에서 이탈)
- 에이전트 완료 알림의 발화 임계 — **A: 작업이 일정 시간(예: 10초) 이상 지속된 뒤 Idle 이 될 때만(추천, 도배 방지)** / B: 모든 `Working→Idle` 전이마다 / C: `AwaitingInput`(사용자 입력 대기)만 알림하고 `Idle` 은 제외
- 태스크 완료 알림의 소스 — **A: 터미널 OSC 133 `D` 이벤트(exitCode 포함)를 재사용 — 태스크 러너로 띄운 명령뿐 아니라 사용자가 손으로 친 장시간 명령까지 커버(추천, zsh·bash 는 주입·fish 4.0+ 는 네이티브)** / B: 태스크 러너로 실행한 명령만 별도 추적 / C: `terminal:exited`(세션 종료)만 사용 — 명령 단위 완료는 못 잡음
- dev(`tauri dev`)에서 알림이 'Terminal' 이름·아이콘으로 뜨는 상류 제약을 그대로 두어도 될까요 — **A: 그대로 두고 설정 힌트·문서에 명기(추천, 프로덕션 번들은 정상)** / B: dev 에서는 네이티브 알림 자체를 끄고 토스트만

## newDependencies
- tauri-plugin-notification 2.4.0 (Rust, crates.io 최신) — 사용자가 `docs/acknowledge/2026-09-04-usability-batch4-user-decisions.md` §결정 1 에서 명시 승인. macOS 네이티브 알림 전달은 표준 라이브러리·기존 의존성(tauri 2.11.5 / tauri-plugin-opener / notify(파일 워처, 무관))으로 불가능하다: wry 0.55.1 의 WKUIDelegate 에 notification 권한 델리게이트가 없고 WKPreferences 에도 관련 키가 없어 WKWebView 에 Web Notification API 자체가 존재하지 않는다(wry-0.55.1/src/wkwebview/class/wry_web_view_ui_delegate.rs, src/wkwebview/mod.rs). 대안인 `osascript -e 'display notification'` 은 Script Editor 이름으로 뜨고 프로세스 스폰 우회라 컨벤션 §6.2/§6.7 위반. tauri 2.10+ 요구 → 현재 2.11.5 와 호환.
- notify-rust 4.x (Rust, 전이 의존 — tauri-plugin-notification 이 `notify-rust = "4.11"` 로 선언, 현재 최신 4.18.0 해석). 직접 추가하는 것이 아니라 플러그인이 끌어온다. macOS 외 타깃의 zbus/dbus 계열은 `cfg(all(unix, not(macos)))` 게이트라 macOS 빌드에 들어가지 않는다. 주의: 4.18.0 의 rust-version 이 1.89 라 src-tauri 의 선언 MSRV(1.80) 상향이 필요하다.
- mac-notification-sys 0.6.15 (Rust, notify-rust 의 macOS 전이 의존). 하위 의존(objc2, objc2-foundation, time, log, uuid, cc)은 전부 이미 Cargo.lock 에 존재해 순증이 없다. build.rs 가 ObjC 를 정적 컴파일하고 framework=AppKit 만 링크하므로 릴리스 자립성 게이트(docs/deployment.md §8 non-system dylib linkage)에 걸릴 외부 dylib 을 만들지 않는다. rust-version 1.85 · edition 2024.
- (추가하지 않음) @tauri-apps/plugin-notification (npm) — 추천안 A 는 Rust 의 NotificationExt 로만 알림을 보내므로 JS 게스트 패키지가 필요 없다. 이 패키지의 API 는 전부 플러그인이 주입한 window.Notification shim 에 의존해(dist-js/index.js:73-122) 원격 미러에서는 어차피 동작하지 않는다.
