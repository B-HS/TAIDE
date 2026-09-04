# 기능 — 커맨드 팔레트 · 퀵오픈

> Phase 7.5 에서 확장(사용자 지적 10번). 기존 `⌘⇧P`/`⌘P` 구현을 **확장 가능한 레지스트리**로 바꾼다.
> 키 선언 정본은 `shared/lib/keymap/keymap.ts`, 키 계층 규칙은 `editor.md` §6.

## 1. 모드 (VSCode/Cursor 동일)

단일 입력창에서 **접두어로 모드를 전환**한다.

| 접두어 | 모드 | 예 |
|--------|------|-----|
| (없음) | 파일 퀵오픈 | `⌘P` 로 진입. 프로젝트 파일 fuzzy 검색 |
| `>` | 커맨드 실행 | `⌘⇧P` 로 진입. 등록된 커맨드 fuzzy 검색 |
| `:` | 줄 이동 | `:120` → 현재 파일 120 줄 |
| `@` | 심볼 이동 (구현 완료) | LSP `documentSymbol` — `CommandPaletteSymbolGroup` |
| `#` | 워크스페이스 심볼 (구현 완료) | LSP `workspaceSymbol` — `CommandPaletteWorkspaceSymbolGroup` |

- 모드 전환은 **입력값을 지우지 않고** 접두어만 붙였다 뗄 수 있어야 한다.
- `⌘P` 로 열면 빈 문자열, `⌘⇧P` 로 열면 `>` 가 미리 채워진 상태로 연다.

## 2. 커맨드 레지스트리 (확장성 — 10번의 핵심)

> 사용자 요구: "확장성있게 만들어놔서 **나중에 뭐든 잘 붙을 수 있게**"

커맨드를 팔레트 컴포넌트에 하드코딩하지 않는다. `shared/lib/command-registry.ts` 에
**등록 기반**으로 둔다.

실제 타입명은 `AppCommand` 이고 제목·카테고리는 i18n 키다(`titleKey`/`categoryKey` +
`titleDefaultValue`), 키바인딩 참조는 `keymapId`, 조건부 노출은 `isEnabled` 다. 아래는 개념 예시:

```ts
type Command = {
    id: string                     // 'window.reload' (§2.1 의 <영역>.<동작> 체계)
    title: string                  // 실구현은 titleKey (i18n)
    category?: string              // 실구현은 categoryKey (i18n)
    keybinding?: string            // 실구현은 keymapId — keymap.ts 의 id 참조 (중복 선언 금지)
    when?: (ctx: CommandContext) => boolean   // 활성 프로젝트 유무 등
    run: (ctx: CommandContext) => void | Promise<void>
}
```

- `CommandContext` 는 활성 프로젝트·활성 탭·QueryClient 등 **실행에 필요한 것만** 담는다.
- **`keymap.ts` 와 이중 선언하지 않는다.** 키는 keymap 이 정본이고 커맨드는 `keybinding` 으로 참조만 한다.
- 등록 지점은 앱 조립부(`app/`) — 각 도메인이 자기 커맨드를 등록해 올린다.
  플러그인(`plugins.md`)이 나중에 커맨드를 기여할 자리도 여기다.

### 2.1 내장 커맨드 (실등록 24종 — `shared/lib/command-catalog.ts` `DEFAULT_COMMANDS` 정본)

id 체계는 `<영역>.<동작>` 이다(초안의 `workbench.action.*` VSCode 식 id 는 채택하지 않았다).

| 영역 | id |
|------|-----|
| 창·뷰 | `window.reload`, `view.toggleSidebar`, `view.toggleTerminal`, `view.explorer`, `view.git`, `view.welcome`, `view.toggleZenMode` |
| 탭 | `tab.close`, `tab.reopenClosed`, `tab.cycleNext` / `tab.cyclePrev`, `tab.moveToNewWindow` / `tab.moveToMainWindow` |
| 에디터 | `editor.save`, `editor.find`, `editor.split` |
| 검색·파일 | `file.quickOpen`, `search.find`, `search.replace` |
| 설정·기타 | `settings.open`, `keybindings.open`, `app.openSettingsFile`, `terminal.new`, `terminal.copyImeDebug` |

- git 동작(commit/push/pull)·LSP 재시작·플러그인 리로드·프로젝트 열기/닫기는 팔레트 커맨드로
  등록하지 않았다 — 각각 git 패널·설정 LSP 섹션·플러그인 매니저·사이드바 UI 로 노출된다.
- `view.welcome`(2026-09-04) 은 `TabKind::Welcome` 탭을 **이 창의 focused pane** 에 연다
  (`currentWindowFocusedPane`). 규약 3가지:
  - **기본 단축키 없음.** `keymapId` 가 없으므로 `buildKeybindingRows` 가 `runsViaCommand` 행으로
    만들어 주고, 사용자가 키바인딩 에디터에서 직접 바인딩할 수 있다(§5 · `keymap.md`). `APP_KEYMAP`
    충돌 표면을 넓히지 않기 위한 선택이다.
  - **탭 제목은 로케일이 아니라 리터럴** `WELCOME_TAB_TITLE`(`shared/constants/tab.ts`) — Rust
    `layout::service::default_layout()` 이 쓰는 리터럴과 같다. `open_tab` 은 같은 leaf 에 동일 kind
    탭이 있으면 재사용하면서 **기존 title 을 갱신하지 않으므로**, 로케일 제목을 넘기면 복원된 탭과
    새 탭의 제목이 갈린다. 팔레트 라벨은 `app.welcome` 로케일 키를 쓴다.
  - 활성 프로젝트가 없으면 목록에서 감추지 않고 실행 시 `app.openProjectFirst` 토스트를 띄운다
    (`settings.open`·`terminal.new`·`app.openSettingsFile` 와 동일).

## 3. 매칭·정렬

- fuzzy 매칭은 기존 `shared/lib/fuzzy-match.ts` 순수 함수를 그대로 쓴다(테스트 있음).
- 정렬: 점수 내림차순 → **최근 사용(MRU)** → 알파벳.
  MRU 는 view 로컬이 아니라 `settings` 에 저장해 재시작 후에도 유지한다.
- 파일 퀵오픈은 **트리에 아직 로드되지 않은 폴더의 파일도 찾아야 한다** — `tree_rows`(사이드바와
  동일한 지연 로딩 트리)에 의존하면 안 된다. 전용 커맨드 `search_list_files(projectId) →
  string[]`(d-42, `search` 도메인의 `build_walk` 파일 순회 재사용 — 신규 크레이트 없이 기존
  `ignore` 크레이트 인프라를 그대로 씀)가 프로젝트 root 이하 전체 파일을 매 호출마다 새로 순회해
  반환하고, `entities/search/search.query.ts` 의 `projectFilesQueryOptions` 가 이를 감싼다. (과거
  구현은 `tree_rows` 를 직접 필터해 이 요구사항을 위반했었다 — `docs/acknowledge/
  2026-08-25-d42-e2e-defects-contract.md` §3 item d 로 근본 수정)
- 파일 퀵오픈의 fuzzy 매칭 대상은 `search_list_files` 의 **절대 경로가 아니라 활성 프로젝트 기준 상대
  경로**다(`toProjectRelativePath`). 활성 프로젝트 root 가 아직 로딩 중이면 매칭·렌더 자체를
  보류한다(빈 상태에 로딩 표시) — 절대 경로가 매칭 대상으로 잠깐이라도 노출되지 않는다.

### 3.1 인덱스 신선도 규약 (2026-09-04)

`search_list_files` 는 매 호출마다 전체를 다시 순회하고 Rust 측 캐시가 없다. 따라서 퀵오픈 인덱스의
유일한 저장소는 프론트의 `QUERY_KEY.SEARCH.PROJECT_FILES(projectId)` 쿼리 캐시 한 장이고, "이 배열이
언제 갱신되는가" 가 곧 신선도 계약이다. 무효화 경로는 셋이며 각각 역할이 다르다.

| 경로 | 트리거 | 방식 | 근거 |
|------|--------|------|------|
| 인앱 뮤테이션 | `useCreateEntry`·`useRenameEntry`·`useCopyEntry`·`useDeleteEntry` 의 `onSuccess` | `invalidateQueries({ refetchType: 'all' })` (`entities/file/file.query.ts` 의 `invalidateProjectFileIndex`) | 사용자 1회 조작 = walk 1회. 워처 에코(300ms 디바운스)보다 결정적이고 빠르다 |
| 워처 에코 | `fs:changed` 의 `kind !== 'modified'` (`app/providers/ipc-sync-provider.tsx`) | 기본 `refetchType`(active 만) | 외부 대량 변경(브랜치 전환·설치)이 다수 배치로 쪼개져 오므로 `'all'` 은 walk 폭주가 된다 |
| 열기 실패 | `layout_open_tab` 이 `NotFound` 를 돌려줌 | `useOpenFileTab` 이 그 자리에서 무효화 | `NotFound` 는 "이 목록이 낡았다" 는 신호 자체다. 같은 죽은 행을 다음 `⌘P` 가 또 내놓지 않는다 |

- **재-walk 시점은 "다음 열기"다.** 팔레트 쿼리는 열려 있는 동안만 `enabled` 이고, TanStack Query 의
  `refetchQueries` 는 `refetchType` 과 무관하게 disabled 쿼리를 제외한다(query-core 5.101 의
  `!query.isDisabled()` 필터 — 옵저버가 하나라도 있으면 `isDisabled = !isActive`, 그 옵저버가 항상
  마운트된 `<CommandPalette/>` 다). 그래서 팔레트가 닫힌 채 일어난 뮤테이션은 인덱스를 stale 로
  표시하고, 실제 walk 는 다음 `⌘P` 오픈 시 1회 돈다. 그 첫 프레임이 낡은 배열을 보여 주는 창은 아래
  "갱신 중 표시" + "열기 선검증" 두 겹이 막는다. `refetchType: 'all'` 은 **살아 있는 옵저버가 없는
  인덱스**(활성이 아니게 된 프로젝트)까지 무효화 범위를 넓히는 역할이다. 팔레트가 닫혀 있어도 즉시
  재-walk 하게 하려면 `app/bootstrap-snippets.ts` 처럼 영구 `QueryObserver` 를 붙여야 하는데, 그러면
  워처 에코까지 항상 활성 쿼리를 때려 외부 대량 변경에서 walk 폭주가 되므로 채택하지 않았다.
- `useSaveFile` 은 위 목록에 **넣지 않는다.** 저장은 인덱스에 이미 있는 경로를 덮어쓰고, 자동 저장이
  빈도를 무한정 늘린다. 새 파일이 생기는 유일한 저장 경로(무제목 탭의 Save As)는 워처의 `created`
  에코가 덮는다.
- **열기 선검증**: 파일 탭은 Rust `layout_open_tab` 이 경로가 실제 파일인지 확인한 뒤에만 열린다.
  낡은 행을 눌러도 빈 에디터가 열리고 본문에 `No such file or directory (os error 2)` 가 뜨는 대신,
  탭이 아예 열리지 않고 `error.file.notFound` 토스트가 뜬다.
- **프론트 단일 진입점**: 파일 탭을 여는 모든 호출부는 `entities/layout/layout.query.ts` 의
  `useOpenFileTab` 을 쓴다. 에러 토스트와 위 `NotFound` 무효화가 이 한 곳에만 있고, 이후 MRU 기록·
  autoReveal 같은 "파일 열기" 고도화도 여기서 확장한다.
  - 예외 1곳: `app-shell.tsx` 의 드래그앤드롭 열기는 `useOpenTabInProject().mutateAsync` 를 유지한다.
    여러 파일을 한 번에 떨어뜨리면 `for … await` 로 **한 개씩 순차 열기**를 해야 하는데(동시 발사하면
    각 응답의 `applyFreshLayout` 이 완료 순서대로 캐시를 덮어써 마지막에 도착한 낡은 레이아웃이
    이긴다), `useOpenFileTab` 은 `mutate` 기반이라 await 할 대상이 없다. OS 파일 매니저가 건네는
    경로는 스테일 인덱스 출처가 아니므로 `NotFound` 무효화를 놓쳐도 잃는 것이 없고, 토스트는 그
    호출부가 이미 띄운다. 순차성을 잃지 않는 async 변형이 필요해지면 그때 훅에 추가한다.
- **갱신 중 표시**: 팔레트가 열려 있는 동안 재-walk 가 도는 창은 `files` 그룹 헤딩 옆에
  `palette.filesRefreshing` 스피너로 드러낸다. 항목 클릭은 막지 않는다 — 선검증이 이미 방어한다.

## 4. UI

- shadcn `command`(cmdk) 기반. **native `<input>` 금지**(acknowledge §3.1) — 자체 컴포넌트로.
- 항목 행: 아이콘 · 제목 · (커맨드면) 카테고리 · (키 있으면) 우측 키 배지.
  파일 모드는 **파일명(주 라벨) + 프로젝트 상대 디렉토리 경로(부제, 소프트 색) 2단** 렌더 —
  프로젝트 root 직속 파일은 부제를 생략한다.
- fuzzy 매칭 문자는 강조 표시한다(files·commands·symbol 모드 — LSP `workspaceSymbol` 결과는
  매칭 인덱스가 없어 이월 제외). 강조는 배경 채움이 아니라 **글자색 + 굵기**로 표현한다(테마별
  대비가 배경 채움보다 안정적).
- 선택 행(방향키 이동)은 배경색 변화 + 얇은 링 2가지 단서로 표시한다(테마 의존 저대비 대응 —
  `docs/acknowledge/2026-08-20-palette-ux-contract.md` §4.1).
- 결과 상한과 가상 스크롤: 파일이 수만 개일 수 있으므로 **상한 + 가상 스크롤** 둘 다 적용한다.
- `Esc` 닫기, `↑↓` 이동, `Enter` 실행, `⌘Enter`(있으면) 옆 pane 에 열기.

## 5. 수명주기

- 팔레트는 전역 1개. 열림 상태는 컴포넌트 로컬(zustand 불필요).
- 전역 키 구독은 `use-global-keymap` 캡처 단계에 더해, 커맨드 실행형(`runsViaCommand`) 바인딩
  전용의 두 번째 캡처 리스너를 팔레트 자체가 하나 더 둔다(`command-palette.tsx` 의
  `useKeydownCapture` — Wave H 계약 근거는 해당 콜백의 JSDoc 참고). 전역 키맵에 엔트리가 없는
  커맨드 바인딩은 이 리스너가 유일한 디스패치 경로라 의도된 구조다.
- 닫힐 때 입력값·선택 인덱스를 초기화한다(다음에 열 때 잔상 금지).
- **열릴 때 포커스·캐럿 계약 (2026-08-30)**: 입력에 포커스하고 캐럿을 값 끝에 둔다(선택 없음).
  Radix `FocusScope` 가 첫 포커스에 `select()` 를 걸고 WKWebView 도 프로그램적 포커스에 전체선택을
  복원해, ">" 프리픽스가 선택된 채 열려 첫 타이핑이 프리픽스를 지우던 문제의 수정. 경로 2개 —
  `DialogContent.onOpenAutoFocus`(preventDefault + `focusTextInputCaretAtEnd`)와 닫힘 애니메이션
  중 재열림용 `[open]` 이펙트(`shared/lib/text-input-caret.ts`, 테스트 5건).
