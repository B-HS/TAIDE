# 기능 — 윈도우 크롬 (타이틀바 · footer · 기동 연출)

> Phase 7.5 신규. 사용자 지적 4·15·17·18번. 색 토큰은 `theme-system.md` §3 을 따른다.

## 1. 타이틀바 (헤더)

현재 `tauri.conf.json` 이 `titleBarStyle: "Overlay"` + `hiddenTitle: true` 라
좌측에 신호등 버튼만 있고 나머지가 비어 있다.

### 1.1 구성

```
[신호등 여백]   [중앙: 활성탭 이름 · 프로젝트명 · git 브랜치]   [우측: 여백/액션]
```

- **중앙 표시(18번)**: `{활성 탭 이름} — {프로젝트 이름}` + git 브랜치 배지.
  - 활성 탭은 `layout` 도메인의 `focused_pane` 의 활성 탭에서 얻는다.
  - 브랜치는 `git_status` 의 `branch`. git 저장소가 아니면 배지를 숨긴다.
  - 폭이 좁아지면 브랜치 → 프로젝트명 → 탭 이름 순으로 생략한다.
- 드래그 영역은 `data-tauri-drag-region`. **이 속성은 상속되지 않으므로**
  자식 요소(텍스트·배지)에도 개별 부여하거나 `pointer-events: none` 으로 통과시킨다
  (`research/tauri-v2.md` 함정 12).

### 1.2 다크/라이트 미반영 버그 (17번)

**증상**: 테마를 바꿔도 신호등이 있는 헤더 영역 색이 안 바뀐다.

원인 후보(구현 시 실측 확인):
1. `tauri.conf.json` 의 `backgroundColor: "#1e1e2e"` 가 **정적 값**이라 윈도우 자체 배경이
   다크에 고정돼 있다. → 테마 전환 시 `window.set_background_color()` 로 갱신해야 한다.
2. 타이틀바 영역을 그리는 DOM 이 `--taide-app-background` 를 안 쓰고 있다.

→ **둘 다 확인**한다. 1번이면 테마 적용 흐름(`theme-system.md` §5)에 윈도우 배경색 갱신을 추가한다.

**갱신(손 QA 1차 수정, 2026-08-18)**: 1번(`window.set_background_color()` 동적 갱신)은 아직
**backlog 로 유지**한다 — vibrancy 와의 상호작용을 실기로 먼저 확인해야 해서, 이번 수정 범위에는
포함하지 않았다(계약: `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` §2.2·§3). 2번은 별건인
아래 §2 의 reveal 게이트 정합으로 처리됐다.

## 2. 기동 시 흰 화면 (4번)

**증상**: "헤더는 색상 그대로인데 아래는 흰색으로 나왔다가 사이드바/본문이 렌더링된다."

`theme-system.md` §4.1 은 FOUC 방지를 **window `backgroundColor` + `visible: false` → 테마 적용 후 `show()`**
로 정의했다. 헤더만 색이 맞았다는 건 이 정책이 **부분적으로만** 동작한다는 뜻이다.

원인 후보:
1. `show()` 시점이 **테마 CSS 변수 주입보다 빠르다** — 현재 `ThemeProvider` 는 `isFetched` 로
   reveal 하는데, `isFetched` 는 **데이터 도착** 시점이지 **DOM 반영** 시점이 아니다.
   → 변수 주입 effect 가 끝난 뒤 reveal 해야 한다.
2. `<body>` 배경은 CSS 로 칠해지지만 **첫 페인트 전** 브라우저 기본 흰색이 보인다.
   → `index.html` 의 인라인 `<style>` 로 `html,body { background: #1e1e2e }` 를 **번들 로드 전에** 깔아둔다.
   (localStorage 선주입은 ADR-0004 위반이므로 쓰지 않는다. **정적 기본색 인라인은 허용** —
   테마 값이 아니라 "첫 페인트용 바탕색"이기 때문.)

→ 두 가지를 함께 적용하고, **실제로 흰 깜빡임이 사라졌는지 눈으로 확인**한 뒤 완료 처리한다.

### 2.1 reveal 게이트 vs CSS 게이트 불일치 (손 QA 1차 수정, 2026-08-18)

> 계약: `docs/acknowledge/2026-08-18-hand-qa-fix-contract.md` §2.2. 위 1번 원인의 실제 사례 —
> "변수 주입 effect 가 끝난 뒤 reveal" 이 부분적으로만 구현돼 있었다.

- **증상·원인**: 윈도우 표시(`useRevealWindow`, OS 레벨 `getCurrentWindow().show()`)는 **테마
  쿼리의 `isFetched` 만** 대기했는데, body 자체의 시각적 노출은
  `html[data-theme-ready][data-locale-ready] body`(`global.css`) 로 **테마+로케일 이중 게이트**를
  걸고 있었다(로케일 게이트는 이후 커밋에서 추가됐고 reveal 쪽은 함께 갱신되지 않았다). 로케일
  쿼리가 테마보다 늦게 도착하면 **창은 이미 떠 있는데 body 는 아직 숨김** 상태가 되어, 그 사이에는
  `index.html`/`tauri.conf.json` 의 `#1e1e2e` 정적 배경만 노출됐다 — 콜드 스타트에서 "테마가 안
  먹은 것처럼" 보이는 원인.
- **수정**: `useRevealWindow` 를 호출하는 쪽(`theme-provider.tsx`)이 로케일 쿼리(`currentLocaleQueryOptions`,
  같은 `QueryClient` 캐시라 추가 IPC 없음)의 `isFetched` 도 함께 보도록 확장 —
  `useRevealWindow(isWindowReadyToReveal(themeFetched, localeFetched))`(신설 `isWindowReadyToReveal`,
  `shared/hooks/use-reveal-window.ts`). CSS 게이트와 정확히 같은 두 신호를 보게 되어 창이 body 보다
  먼저 뜨는 창이 사라진다. 두 `isFetched` 모두 **쿼리가 에러로 확정돼도 `true`** 로 뒤집히므로
  (React Query 의미상) 실패한 쿼리를 영원히 기다리는 데드락은 없다.
- **표면화 보강**: 테마 쿼리가 실패하면 이전에는 (도착 대기 자체가 없어) 아무 표시 없이 조용히
  넘어갔다. `locale-provider.tsx` 의 실패 배너+재시도 패턴을 `theme-provider.tsx` 에도 이식해
  `isError` 시 상단 배너(`t('theme.loadFailed')` + 재시도 버튼)를 띄운다.

### 2.2 main 창은 setup 에서 직접 만든다 (2026-09-04)

- `tauri.conf.json` `app.windows[0]` 은 `"create": false` 이고, 실제 생성은 `lib.rs` `.setup` 의
  첫 문장 `create_main_window` 가 `WebviewWindowBuilder::from_config` 로 **같은 config 를 그대로**
  재생한다. 이유는 웹뷰 네비게이션 가드(`architecture.md` §4.1)의 두 핸들러가 빌더에만 붙기
  때문이고, 크롬 관점에서 바뀌는 것은 없다 — `backgroundColor`·`visible:false`·Overlay
  타이틀바·`hiddenTitle` 이 전부 config 에서 오므로 위 §2·§2.1 의 FOUC 정책과 `useRevealWindow`
  게이트는 그대로다.
- 생성 시점도 같은 구간이다(Tauri 는 setup 훅 **직전**에 config 창을 만든다) — "창이 먼저 뜨고
  무거운 부팅 작업은 뒤로" 라는 `2026-08-20-boot-watcher-defer-contract.md` 의 전제가 유지된다.

## 3. footer (상태바)

### 3.1 구성 (좌 → 우)

1. git 브랜치 · ahead/behind (헤더와 중복이면 헤더 우선, footer 는 생략 가능)
2. 커서 위치 `Ln 12, Col 5` · 선택 길이
3. 파일 인코딩 · 줄바꿈(LF/CRLF) · 언어
4. LSP 서버 상태 (`lsp.md` §4 — 인덱싱 중/크래시 + 재시작 버튼)
5. **폰트 크기 컨트롤 (15번)**

### 3.2 폰트 크기 컨트롤

**범위는 에디터·터미널 폰트만이다**(사용자 확정 — acknowledge §1.3). 앱 UI 배율은 건드리지 않는다.

- `−` / 현재 크기 / `+` / 클릭 시 기본값 리셋.
- 값은 `settings` 도메인의 `editor_font_size` · `terminal_font_size` 에 저장 → 전 에디터·터미널에 즉시 반영.
- 터미널은 크기 변경 후 **반드시 `fit.fit()`** 을 호출한다(`terminal.md` §7 함정 16).
- 하한·상한은 `shared/constants/code-font-size.ts` 의 `MIN_CODE_FONT_SIZE`(6)·
  `MAX_CODE_FONT_SIZE`(48) — 매직넘버 금지, 상수로. (`shared/constants/terminal.ts` 의
  `MIN_FONT_SIZE` 는 터미널용 별개 상수다.)

## 4. 수명주기

- 타이틀바·footer 는 활성 프로젝트/탭 변경 이벤트만 구독한다(`layout:changed`, `git:status-changed`).
  **폴링 금지.**
- footer 의 커서 위치는 Monaco `onDidChangeCursorPosition` 을 **에디터 위젯이 올려주는 값**으로 받는다.
  footer 가 에디터 인스턴스를 직접 잡지 않는다(FSD 역참조 방지).

## 5. 보조 편집 창의 크롬 (Wave I)

> 멀티 윈도우 모델 자체는 `layout-shell.md` §7 이 정본이다. 여기서는 그 창의 **크롬**만 다룬다.

- 보조 창(`editor-<n>`)은 앱 사이드바·탐색 사이드바·상태바가 전혀 없다 — 탭바 + pane 트리(스플릿
  포함)만 있는 **에디터 전용 크롬**(`AuxiliaryWindowShell`).
- 타이틀바는 macOS 에서만 렌더(다른 플랫폼은 OS 네이티브 타이틀바를 그대로 쓴다 — main 창과 동일
  정책)하며, main 창의 `TitleBar` 컴포넌트를 그대로 재사용한다. 단 표시하는 값(활성 탭 이름·
  프로젝트명·git 브랜치)은 전역 활성 프로젝트가 아니라 **이 창 자신의 고정 `(projectId,
  windowSlot)`** 에서 유도한다(`AuxiliaryTitleBarContent`).
- 보조 창은 `shell_view`(Zen·사이드바 접힘)의 영향을 받지 않는다 — 애초에 숨길 사이드바/상태바가
  없다. 아래 §6 의 Zen 모드는 main 창 전용이다.

## 6. Zen 모드 (Wave I)

> 계약: `docs/acknowledge/2026-08-16-wave-i-shell-workspace-contract.md` §3.2. 스키마:
> `data-model.md` §8 `ShellViewState`. IPC: `layout_set_shell_view`.

### 6.1 무엇을 숨기는가

- 탐색 사이드바(explorer/search/git 패널) — 강제 접힘.
- **앱 사이드바(프로젝트 아이콘 레일, §2)도 함께 숨긴다.** 계약 원문은 "사이드바·탭바(+상태바)"만
  명시해 아이콘 레일 포함 여부가 불명확했는데, VS Code 의 실제 Zen Mode(Activity Bar 도 숨김)에
  맞춰 완전한 distraction-free 로 해석했다 — 좁게 해석하고 싶다면 `app-shell.tsx` 의 조건 렌더
  한 줄만 되돌리면 된다.
- 모든 pane 의 탭바.
- 상태바 — `Settings.zen_hide_status_bar`(기본 true)가 켜져 있을 때만.
- 위 넷 다 `layout.shell_view.zen` 하나로 제어되고, **프로젝트 단위로 영속**된다(창을 닫았다 다시
  열거나 앱을 재시작해도 유지 — 로컬 `useState` 가 아니다).

### 6.2 진입·이탈

| 방법 | 동작 |
|------|------|
| `⌘K Z` chord | `toggle-zen-mode` 커맨드 — `open-keybindings-editor`(`⌘K ⌘S`)와 1단(`⌘K`)을 공유하는 두 번째 chord (`keymap.md` 참조) |
| 팔레트 `view.toggleZenMode` | main 창에서만 활성(보조 창엔 `shell_view` 개념이 없다) |
| `Esc` | Zen 상태일 때만 리스너를 붙이는 `window` **bubble 단계** 리스너 — Radix 다이얼로그·팔레트·monaco 자체 Escape 처리가 전부 먼저 `preventDefault()` 할 기회를 가진 **뒤에** 평가되므로, 다이얼로그가 열려 있을 때 Esc 를 누르면 다이얼로그만 닫히고 Zen 은 유지된다(`event.defaultPrevented` 가드) |
| 진입 힌트 오버레이 | Zen 진입 시 3초간 "Zen Mode · Press Esc to exit" 배너(`zen.hint`/`zen.hintExit`) — 자동 소멸 |

- `Settings.zen_fullscreen`(기본 false)이 켜져 있으면 Zen 진입/이탈이 OS 창 전체화면도 함께
  전환한다(`window_set_fullscreen` — 호출한 창 자신이 대상, 라벨 불필요). 설정을 Zen 도중에 바꿔도
  즉시 반영되고, Zen 을 나가면 그 순간의 설정값과 무관하게 항상 전체화면을 해제한다(전체화면에 갇히지
  않는다).
- 사이드바 **접힘 여부**(`shell_view.sidebar_collapsed`)는 Zen 과 별개로 드래그/`⌘B` 양쪽 경로 모두
  영속된다. 사이드바 **폭**은 기존과 동일하게 프론트 로컬 debounce 로만 저장한다(ADR-0004 예외 —
  변경 없음).

### 6.3 설정 UI

설정 화면 INTERFACE 섹션에 `zen_fullscreen`/`zen_hide_status_bar` 토글 2개가 있다(`settings-view.tsx`
— 기존 `settingsUpdate` 플로우 그대로, 별도 저장 경로 없음).

## 7. 앱 메뉴바 (macOS 네이티브 메뉴)

> 사용성 배치 5 웨이브 1(d-58, 2026-09-15)에서 `File` 메뉴를 신설했다. 계약
> `docs/acknowledge/2026-09-15-d58-usability-batch5-wave1-contract.md` §1.E.

### 7.1 구성

```
TAIDE   File   Edit   Window
         └ Open Recent ▸  <최근 프로젝트 최대 10>
                          ──────────
                          Clear Recent
```

- 메뉴는 `domain::window::commands::build_app_menu` 이 조립하고, `File` 서브메뉴만
  `domain::window::menu` 이 만든다(`build_file_submenu`). `lib.rs` 의 `setup` 은 **`AppState` 를
  `manage` 한 뒤에** `set_menu` 를 호출한다 — `Open Recent` 는 `AppPaths` 가 가리키는 디스크의
  프로젝트 기록에서 만들어지므로 상태보다 먼저 만들 수 없다(그 제약은 `build_app_menu` 의 문서
  주석이 정본이다).
- **window 도메인은 project·settings 도메인을 호출하지 않는다.** 클릭 디스패치도 메뉴 갱신도
  전부 조립(`lib.rs`)이 소유한다 — architecture.md §2, `tests/domain_boundaries.rs` 가 강제한다.
  `window::menu` 가 갖는 교차 참조는 최근 목록·라벨을 **읽는** 두 건(`project::service`,
  `locale::service`)뿐이고 화이트리스트에 사유와 함께 등재돼 있다.
- **Open Recent 항목**: `project::service::list_recent_projects`(=`last_opened_at` 내림차순) 상위
  `RECENT_PROJECT_MENU_LIMIT`(10, `src-tauri/src/constants.rs`)개. 라벨은 프로젝트의 표시 라벨
  (`display.label`)이 있으면 그것, 없으면 폴더명 — 알림 제목이 프로젝트를 부르는 방식과 같다.
  폴더가 사라진 기록(`rootMissing`)은 **목록에 남되 비활성**이다(조용히 사라지면 어디로 갔는지
  알 수 없다). 목록이 비면 비활성 `No Recent Projects` 한 줄이 대신 선다.
- **Clear Recent**: 열려 있지 않은 프로젝트의 디스크 레코드를 지운다(`project_forget_recent`).
  목록이 비어 있으면 비활성. 열려 있는 프로젝트는 지우지 않는다 — 그 레코드는 히스토리가 아니라
  레이아웃·표시 오버라이드·id 재사용의 정본이다.

### 7.2 클릭 처리 (조립이 디스패치한다)

- `window::menu::menu_action(menuId) → MenuAction`(`Quit`·`OpenRecent(projectId)`·`ClearRecent`·
  `Ignored`)은 **순수 함수**다 — id 표만 보고 판정하므로 이벤트 루프 없이 테스트된다.
- `lib.rs` 의 `on_menu_event` → `dispatch_menu_action` 이 그 값을 주인 도메인으로 보낸다:
  `Quit` → `window::commands::request_quit`(메인 스레드에서 그대로), 나머지는
  `tauri::async_runtime::spawn` 으로 `project::commands::project_open` /
  `project_forget_recent`. 메뉴 이벤트 핸들러는 메인 스레드에서 돌기 때문에 — `project_open` 의
  뮤테이션 가드를 거기서 블로킹하면 그 open 이 필요로 하는 이벤트 루프가 멈춘다 — 그리고 최근
  목록 읽기가 디스크 IO 이기 때문이다.
- 최근 항목 클릭은 프론트를 거치지 않고 project 커맨드 함수를 직접 호출한다 — **창이 0개여도
  동작해야** 하기 때문이다(macOS 는 마지막 창을 닫아도 앱이 살아 있고, 그 상태가 "최근 프로젝트
  다시 열기"가 가장 쓸모 있는 상태다).
- 메뉴 항목의 id 는 `taide-recent:<projectId>` 이고, root 는 클릭 시점에 레코드를 **다시 읽어**
  얻는다(`menu::recent_project_root` — 그 사이 Clear Recent 로 지워졌을 수 있다).

### 7.3 동적 갱신 (이벤트 구독)

- `lib.rs` 의 `listen_for_app_menu_refresh` 가 `project:list-changed`·`project:activated` 를
  구독해 `window::menu::refresh_recent_menu` 를, `settings:changed` 를 구독해 (언어가 실제로
  바뀐 경우에만) `window::commands::refresh_app_menu` 를 부른다. **커맨드가 메뉴를 부르지
  않는다** — 열기·닫기·활성·forget 은 이미 그 이벤트들을 내보내고 있었고, 구독으로 바꾸면서
  project/settings → window 실행 경로가 사라졌다(d-58 §1.E-보강).
- 두 반응 모두 `tauri::async_runtime::spawn_blocking` 으로 넘긴다. 리스너는 이벤트를 emit 한
  스레드에서 그대로 실행되는데, `project_close`/`project_activate` 는 아직
  `AppState::begin_mutation` 을 쥔 채 emit 하므로 그 자리에서 디스크를 읽으면
  architecture.md §2.1 위반이다.
- `settings:changed` 페이로드에는 이전 값이 없으므로 리스너가 **마지막으로 그린 언어를
  기억**해 비교한다. 언어를 건드리지 않는 설정 변경(토글·에디터 옵션)은 메뉴를 다시 만들지
  않는다.
- 갱신은 `Open Recent` 안의 항목만 교체하고, **제거·추가를 `run_on_main_thread` 클로저 하나**
  안에서 한다. `Submenu::append`/`remove_at` 은 각자 메인 스레드로 마샬링하지만
  (`run_item_main_thread!` → `send_user_message` 는 이미 메인 스레드면 그 자리에서 실행) 그건
  원자성이 아니다 — 갱신 둘이 겹치면 서로의 제거·추가가 끼어들어 두 목록이 섞인 메뉴가 남을 수
  있다. 디스크 읽기는 클로저 **밖**(호출한 blocking 스레드)에 둔다.
- 갱신 실패는 `log::warn!` 로만 남긴다(메뉴가 낡은 것은 미관 문제고, 갱신을 유발한 커맨드는 이미
  성공했다).
- **언어 변경 시에는 전체 메뉴를 다시 만든다**(`commands::refresh_app_menu` → `set_menu`).
  서브메뉴의 제목(`File`·`Open Recent`)은 만들 때 고정되므로 항목만 교체해서는 바뀌지 않는다.

### 7.4 알려진 제약

- **`language: "system"` 이면 메뉴는 영어다.** 메뉴 라벨은 Rust 가
  `locale::service::lookup_builtin_message` 로 내장 카탈로그에서 직접 읽는데, OS 로케일은 프론트의
  `navigator.language`(=`locale_get_current(systemLanguage)` 인자)로만 이 앱에 들어오고 Rust
  프로세스는 그 값을 관측하지 않는다. 언어를 명시적으로 `ko`/`ja` 로 고르면 메뉴도 그 언어가 된다.
  (후속: 프론트가 해석한 로케일 id 를 Rust 에 보관하고 메뉴를 그 값으로 그린다.)
- **Dock 메뉴는 구현하지 않는다.** tauri 2.11 에 Dock 메뉴 API 가 없다(결정:
  `docs/acknowledge/2026-09-15-usability-batch5-user-decisions.md` §2).
- **TAIDE/Edit/Window 메뉴의 문자열은 아직 영어 하드코딩**이다(범위 밖 — 계약 §4 후속).
- 사용자 정의 로케일 팩(디스크 `.json`)의 메뉴 라벨은 읽지 않는다 — 메뉴 재구성 경로에 파일 IO 를
  두지 않기 위해 내장 3종만 조회한다.
