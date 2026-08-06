# 기능 — 커맨드 팔레트 · 퀵오픈

> Phase 7.5 에서 확장(사용자 지적 10번). 기존 `⌘⇧P`/`⌘P` 구현을 **확장 가능한 레지스트리**로 바꾼다.
> 키 선언 정본은 `shared/lib/keymap.ts`, 키 계층 규칙은 `editor.md` §6.

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
- 파일 퀵오픈은 `tree_rows` 결과를 쓰되, **트리에 아직 로드되지 않은 폴더의 파일도 찾아야 한다.**
  → 퀵오픈 전용으로 Rust 에 **전체 파일 경로 인덱스**를 두거나(무시목록 공유),
  `search` 도메인의 파일 순회를 재사용한다. 트리의 lazy 로딩에 의존하면 안 된다.

## 4. UI

- shadcn `command`(cmdk) 기반. **native `<input>` 금지**(acknowledge §3.1) — 자체 컴포넌트로.
- 항목 행: 아이콘 · 제목 · (커맨드면) 카테고리 · (키 있으면) 우측 키 배지.
- 결과 상한과 가상 스크롤: 파일이 수만 개일 수 있으므로 **상한 + 가상 스크롤** 둘 다 적용한다.
- `Esc` 닫기, `↑↓` 이동, `Enter` 실행, `⌘Enter`(있으면) 옆 pane 에 열기.

## 5. 수명주기

- 팔레트는 전역 1개. 열림 상태는 컴포넌트 로컬(zustand 불필요).
- 전역 키 구독은 `use-global-keymap` **캡처 단계** 하나만 쓴다(중복 리스너 금지).
- 닫힐 때 입력값·선택 인덱스를 초기화한다(다음에 열 때 잔상 금지).
