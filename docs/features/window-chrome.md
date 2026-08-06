# 기능 — 윈도우 크롬 (타이틀바 · footer · 기동 연출)

> Phase 7.5 신규. 사용자 지적 4·15·17·18번. 색 토큰은 `theme-system.md` §3 을 따른다.

## 1. 타이틀바 (헤더)

현재 `tauri.conf.json` 이 `titleBarStyle: "Transparent"` + `hiddenTitle: true` 라
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
- 하한은 `MIN_FONT_SIZE`(6). 상한도 둔다(예: 48) — 매직넘버 금지, 상수로.

## 4. 수명주기

- 타이틀바·footer 는 활성 프로젝트/탭 변경 이벤트만 구독한다(`layout:changed`, `git:status-changed`).
  **폴링 금지.**
- footer 의 커서 위치는 Monaco `onDidChangeCursorPosition` 을 **에디터 위젯이 올려주는 값**으로 받는다.
  footer 가 에디터 인스턴스를 직접 잡지 않는다(FSD 역참조 방지).
