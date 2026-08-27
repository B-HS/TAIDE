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
| `@` | 심볼 이동 (2차) | LSP `documentSymbol` |
| `#` | 워크스페이스 심볼 (2차) | LSP `workspaceSymbol` |

- 모드 전환은 **입력값을 지우지 않고** 접두어만 붙였다 뗄 수 있어야 한다.
- `⌘P` 로 열면 빈 문자열, `⌘⇧P` 로 열면 `>` 가 미리 채워진 상태로 연다.

## 2. 커맨드 레지스트리 (확장성 — 10번의 핵심)

> 사용자 요구: "확장성있게 만들어놔서 **나중에 뭐든 잘 붙을 수 있게**"

커맨드를 팔레트 컴포넌트에 하드코딩하지 않는다. `shared/lib/command-registry.ts` 에
**등록 기반**으로 둔다.

```ts
type Command = {
    id: string                     // 'workbench.action.reloadWindow'
    title: string                  // '창 다시 불러오기'
    category?: string              // '보기' — 팔레트에서 그룹 표시
    keybinding?: string            // keymap.ts 의 id 참조 (중복 선언 금지)
    when?: (ctx: CommandContext) => boolean   // 활성 프로젝트 유무 등
    run: (ctx: CommandContext) => void | Promise<void>
}
```

- `CommandContext` 는 활성 프로젝트·활성 탭·QueryClient 등 **실행에 필요한 것만** 담는다.
- **`keymap.ts` 와 이중 선언하지 않는다.** 키는 keymap 이 정본이고 커맨드는 `keybinding` 으로 참조만 한다.
- 등록 지점은 앱 조립부(`app/`) — 각 도메인이 자기 커맨드를 등록해 올린다.
  플러그인(`plugins.md`)이 나중에 커맨드를 기여할 자리도 여기다.

### 2.1 1차 내장 커맨드 (최소 집합)

| id | 제목 |
|----|------|
| `workbench.action.reloadWindow` | 창 다시 불러오기 |
| `workbench.action.restartApp` | 앱 재시작 |
| `workbench.action.openSettings` | 설정 열기 |
| `workbench.action.openThemeEditor` | 테마 편집기 열기 |
| `workbench.action.toggleSidebar` | 사이드바 토글 |
| `workbench.action.toggleTerminal` | 터미널 토글 |
| `workbench.action.splitEditor{Up,Down,Left,Right}` | 에디터 분할 |
| `workbench.action.closeTab` / `closeOthers` / `closeAll` | 탭 닫기 계열 |
| `workbench.action.reopenClosedTab` | 닫은 탭 다시 열기 |
| `project.open` / `project.close` | 프로젝트 열기·닫기 |
| `git.commit` / `git.push` / `git.pull` / `git.sync` | git 동작 |
| `lsp.restartServer` | LSP 서버 재시작 |
| `plugin.reload` | 플러그인 다시 읽기 |

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
