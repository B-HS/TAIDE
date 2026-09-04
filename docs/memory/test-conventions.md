# 테스트 규범 — DOM 하네스 사용법과 커버 범위 한계

> 사용성 배치 4 H.2-1 에서 DOM 렌더 하네스를 도입하며 신설(2026-09-04).
> 계약 `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §H.2-1, 결정 정본
> `2026-09-04-usability-batch4-user-decisions.md` 2차 결정 5.
> **이 문서가 "bun test 로 무엇을 검증할 수 있고 무엇은 e2e 로만 잡히는가"의 정본이다.**

---

## 1. 하네스 구성

| 파일 | 역할 |
|------|------|
| `bunfig.toml` | `[test] preload = ["./src/shared/testing/dom-preload.ts"]` — `bun test` 에만 적용된다(스크립트 실행·빌드는 영향 없음) |
| `src/shared/testing/dom-preload.ts` | 테스트 프로세스당 1회: happy-dom 전역 등록 + `beforeEach(mock.restore)` + `afterEach(cleanup)` |
| `src/shared/testing/render.tsx` | `renderWithProviders` · `renderHookWithProviders` · `createTestQueryClient` + Testing Library 재수출 |
| `src/shared/testing/render.test.tsx` | 하네스 자체의 스모크(effect·이벤트·ref·provider) — 여기가 깨지면 컴포넌트/훅 테스트 전부를 믿을 수 없다 |

### 개발 의존성 4종 (전부 devDependency — 앱 번들 무영향)

| 패키지 | 버전 | 비고 |
|--------|------|------|
| `happy-dom` | 20.14.0 | DOM 구현체 |
| `@happy-dom/global-registrator` | 20.14.0 | **`GlobalRegistrator` 는 `happy-dom` 본체에 없다**(실물 확인: `happy-dom/lib` 에 미포함, 별도 패키지). 조사·계약이 "happy-dom 제공"으로 적은 부분의 정정 |
| `@testing-library/react` | 16.3.3 | `render`/`renderHook`/`fireEvent`/`cleanup` |
| `@testing-library/dom` | 10.4.1 | 위 패키지의 peerDependency — `screen`·역할/라벨 쿼리 |

버전은 캐럿 없이 고정한다(`docs/tech-stack.md` 서문 규칙).

---

## 2. 쓰는 법

- 테스트에서 Testing Library 를 **직접 import 하지 않는다.** 항상 `@shared/testing/render` 에서 가져온다
  (FSD barrel 금지 규칙과 무관 — 레이어 재수출이 아니라 하네스 파사드다).
- 컴포넌트 테스트 파일은 대상 옆에 `*.test.tsx` 로 둔다. `describe`/`test` 설명은 한국어.
- 쿼리는 **역할·라벨 우선**(`getByRole`·`getByLabelText`·`findByText`). `querySelector`·`data-testid` 는
  역할/라벨로 접근할 수 없을 때만.

```tsx
import { fireEvent, renderWithProviders, screen } from '@shared/testing/render'

const { queryClient } = renderWithProviders(<SomeWidget projectId={projectId} />)
fireEvent.click(screen.getByRole('button', { name: 'tab.close' }))
```

- `renderWithProviders` 가 감싸는 것은 **`QueryClientProvider` + `I18nextProvider` 둘뿐**이다.
  `AppProviders` 와 달리 토스터·툴팁 프로바이더는 없다 — 필요한 컴포넌트는 넘기는 `ui` 안에서 직접 감싼다.
- i18n 은 앱과 같은 `i18next` 싱글톤이며 **리소스 번들이 비어 있다.** `t('tab.close')` 는 키 문자열
  `'tab.close'` 를 그대로 렌더하므로, 단언은 번역문이 아니라 **키**로 쓴다.
- `queryClient` 를 함께 돌려주므로 캐시 시드(`setQueryData`)·검사(`getQueryState`)가 가능하다.
  기본값은 `retry: 0` · `gcTime: 0` · `staleTime: 0` · `networkMode: 'always'`(IPC 앱이므로 프로덕션과 동일).
- 훅 단독 테스트는 `renderHookWithProviders`, 상태 갱신은 `act(() => ...)` 로 감싼다.
- 비동기 해소는 `findBy*` / `waitFor` 를 쓴다(둘 다 내부에서 `act` 로 감싸진다).

---

## 3. 프로세스 전역 규약 (bun 특성)

- **bun 은 테스트 파일 간 모듈을 격리하지 않는다.** 한 프로세스에서 전 파일이 돌아간다.
- `beforeEach(mock.restore)` 가 preload 에 등록돼 있다 — `spyOn` 으로 만든 스파이는 매 테스트 전에
  원본으로 되돌아간다. **`mock.restore()` 는 `mock.module()` 을 되돌리지 않는다.** 모듈 목은 등록한
  파일 밖으로도 남고 마지막 등록이 이긴다. 그래서:
  - `mock.module` 팩토리는 그 모듈의 **export 전체**를 채운다(부분 목은 다른 파일에서
    "named export not found" 를 만든다 — `entities/file/file.query.test.ts` JSDoc 선례).
  - 목의 동작을 테스트마다 바꿔야 하면 팩토리 안에서 **가변 참조 1개를 읽게** 만든다
    (`entities/notification/notify.test.ts` 의 `sendNativeNotificationImpl.current` 패턴).
- `afterEach(cleanup)` 가 마운트된 트리를 전부 언마운트한다. RTL 의 자동 cleanup 은 `afterEach` 가
  러너 전역일 때만 스스로 붙는데 bun 에서는 훅이 `bun:test` import 라 붙지 않는다 — preload 가 대신 건다.
- 모듈 전역 상태(모듈 스코프 Map·플래그)를 쓰는 모듈은 테스트가 **스스로 리셋**해야 한다. 전역 훅은
  이를 대신해 주지 않는다.
- **목의 원본을 되돌릴 때 라이브 네임스페이스를 읽으면 무한 재귀가 된다.** `mock.module('x')` 을 건 뒤
  `afterEach` 에서 `ns.fn` 으로 원복하면 그 `ns.fn` 은 이미 목이라 가짜가 자기 자신을 부른다 — 스위트가
  실패하지 않고 **통째로 멈춘다**(2026-09-04 `shared/lib/window-appearance.test.ts` 에서 실제 발생).
  원본은 `mock.module` **등록 전에** 상수로 붙잡는다.

### `createTestQueryClient` 의 `gcTime` 은 0 이다

`shared/testing/render.tsx` 의 테스트 클라이언트는 `retry: 0` · `staleTime: 0` · **`gcTime: 0`** 이라,
관찰자가 없는 쿼리는 즉시 회수된다. 캐시를 미리 채워 두고 `await` 를 끼는 테스트는 그 사이에 시드가
사라지므로 `fetchQuery({ ...options, gcTime: Infinity })` 로 심는다.

---

## 4. 하네스가 정의하는 런타임 환경 (계약)

| 전역 | 값 | 이유 |
|------|-----|------|
| `location` | `http://localhost/` (쿼리 없음) | `getWindowContext()` 가 **main 창**으로 해소된다. 보조창 분기를 테스트하려면 `readWindowContext(search)` 순수 함수를 직접 부른다 |
| `navigator.userAgent` | macOS WKWebView UA 고정 | happy-dom 은 UA 괄호부에서 `navigator.platform` 을 만든다. `IS_MAC`(`shared/constants/platform.ts`)이 수식키 라벨과 macOS 전용 커맨드 카탈로그를 좌우하므로 **호스트 OS 에 따라 테스트 결과가 바뀌지 않도록** 고정했다(ubuntu CI 러너 대비) |
| `navigator.clipboard` | 존재함(`writeText`/`readText`) | 원격 미러(평문 HTTP)의 "클립보드 없음" 분기를 보려면 테스트에서 own property 로 가려야 한다 — `widgets/terminal-pane/terminal-clipboard-availability.test.ts` 의 `withoutClipboardApi` 참고 |
| `window.__TAURI_INTERNALS__` | **없음** | `invoke`·`listen` 은 여전히 거부(reject)된다. IPC 모듈은 `mock.module` 로 대체하는 기존 관례를 유지한다. `getCurrentWindow().label` 이 필요한 코드(예: `isRemoteMirrorRuntime`)만 그 파일에서 `{ metadata: { currentWindow: { label: 'main' } } }` 를 심고 `afterAll` 에서 지운다 |

---

## 5. 커버 범위 한계 — 하네스가 잡지 못하는 것

**"컴포넌트 테스트 그린"을 실기·e2e 통과와 동일시하지 않는다.**

- **WKWebView 고유 결함은 e2e 전용이다.** happy-dom 은 WebKit 이 아니다. 감사가 반복 확인한 결함
  클래스 — IME `keyCode` 229 전면 사망, 클립보드/`keyboard.type()` 콘텐츠 손상, WebKit #269922·#274700 —
  는 **어떤 DOM 하네스로도 재현되지 않는다.**
- **레이아웃·페인트가 없다.** 요소 크기는 전부 0, `getBoundingClientRect()` 는 0 을 돌려준다 — sticky·
  스크롤 위치·가상화의 실제 창 계산·chevron 회전 같은 시각 회귀는 검증되지 않는다.
- **monaco·xterm 은 하네스에서 띄우지 않는다.** monaco 는 `base/browser/window.js` 로 실 DOM 을 요구하고
  (`shared/lib/lsp/command-relay.ts` JSDoc), xterm 은 캔버스/WebGL 렌더러를 쓴다. 기존 관례대로
  `mock.module` 로 대체하고, 에디터·터미널의 실제 동작은 e2e 로 남긴다. (하네스에서의 렌더 가능 여부는
  **미검증** — 시도한다면 먼저 별도로 확인하고 결과를 이 문서에 적는다.)
- **네이티브 다이얼로그·OS 알림·창 관리**는 Rust 소유다. Rust 순수 함수(`decide_delivery` 등)와
  `cargo test` 가 정본이고, 하네스는 FE 쪽 게이트 함수까지만 본다.
- **DOM 변화를 기다리는 `waitFor` 는 쓰지 않고, 마이크로태스크 플러시로 대체한다.** happy-dom 은
  `MutationObserver` 레코드를 `queueMicrotask` 로 전달하므로 `await Promise.resolve()` 반복이면 충분하고
  더 결정적이다 — `shared/hooks/use-overlay-scrollbar.test.tsx` 선례. 기본 `timeout` 을 그대로 둔 채
  영영 오지 않을 DOM 조건을 기다리다 **런이 멈춘 것처럼 보이는**(출력 0, SIGTERM) 사례가 이 규약의
  출발점이다.
  - **절대 금지는 아니다(정정).** 현재 고정 버전 조합(bun 1.4.0 · happy-dom 20.14.0 ·
    `@testing-library/dom` 10.4.1)에서는 실패 조건도 지정한 `timeout` 에 정상적으로 에러로 끝난다
    (실측: `{ timeout: 1000 }` → 1005ms 만에 실패). 멈춤은 **DOM 변화를 기다리는 대기 + 긴 기본
    타임아웃** 조합에서 나타난 것이라, 그 형태를 피하는 것이 규약의 실체다.
  - **현재 예외 3파일** — `entities/settings/settings.query.test.ts` ·
    `entities/layout/use-open-file-tab.test.tsx` · `widgets/explorer/use-explorer-auto-reveal.test.tsx`
    (총 12곳). 전부 DOM 이 아니라 **TanStack Query 상태·목 호출 배열**이 채워지기를 기다리고, 세 파일을
    함께 돌려 894ms 에 전부 그린이다(실측). 새 대기를 추가할 때는 이 형태(비-DOM 조건)인지 먼저
    확인하고, DOM 조건이면 마이크로태스크 플러시를 쓴다.

---

## 6. 하네스 도입으로 낡아진 기록 (다음 테스트 배치에서 정정)

아래 JSDoc 은 "이 프로젝트에는 DOM 하네스가 없다"를 전제로 쓰였고, 이제 사실과 다르다. 해당 파일을
손대는 작업이 생길 때 함께 정정한다(이번 배치는 최소 변경 원칙으로 두 파일만 고쳤다).

- `src/shared/ui/error-boundary.test.tsx:5-41` — "This project has no DOM render harness" 선언과,
  그래서 포기했다고 적은 "크래시 → 폴백 → 재시도 클릭 → children 재마운트" 왕복. 이제 검증 가능하다.
- `src/shared/lib/lsp/document-symbol-session-waiters.ts:46` — "no DOM/testing-library environment
  configured for `bun:test`".
- `src/shared/lib/window-context.ts` · `src/shared/lib/remote/runtime-environment.ts` ·
  `src/shared/lib/error-log-forwarding.ts` · `src/widgets/terminal-pane/terminal-clipboard-availability.ts` —
  "`bun:test` 에는 `window`/`navigator` 가 없다"는 문장. 순수 함수를 분리한 **설계 자체는 여전히 유효**하고
  (전역에 의존하지 않는 편이 낫다) 문장만 낡았다.
- `src/features/tab/tab-bar-menu-items.ts:69` — "메뉴 컴포넌트는 이 저장소에서 단위 테스트가 불가
  (no testing-library)". `features/tab/tab-bar-context-menu.test.tsx` 가 이제 실제로 렌더한다. 순수
  빌더로 표시 규칙을 분리한 설계는 유지한다(빌더 테스트가 조합을, 컴포넌트 테스트가 렌더를 본다).

---

## 7. 관련 문서

- `docs/tech-stack.md` — 패키지·버전 정본
- `docs/quality-assurance/` — e2e 하네스 제약(스위트 후반 열화, `Option` 설정 원복 불가)
- 컨벤션 `frontend.md` §11 (Testing Library·역할/라벨 쿼리) · `backend.md` §13 (한국어 describe)
