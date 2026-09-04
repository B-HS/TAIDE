# 외부 링크가 TAIDE 창 안에서 열림 / 일부 터미널 링크는 아무 반응 없음 (수정)

> 사용자 실기 보고(2026-09-04): "터미널에서 ⌥클릭한 링크가 TAIDE 창 안에서 열린다". 계약·설계 정본은
> `docs/acknowledge/2026-09-04-usability-batch3-contract.md` §B, 기능 정본은
> `docs/features/terminal.md` §6.1, WebView 가드는 `docs/architecture.md`.

## 증상

1. 외부 URL 이 OS 기본 브라우저가 아니라 **앱 창 안에서** 열린다. 앱 webview 에는 주소창도
   뒤로가기도 없어서 사용자는 그 페이지에서 빠져나올 수 없다(앱 재시작 외 복구 수단 없음).
2. 같은 터미널 안에서도 **어떤 링크는 열리고 어떤 링크는 ⌥클릭에 아무 반응이 없다.** 반응이 없는
   쪽은 gh·vite·bun·eza·Claude Code 처럼 OSC 8 하이퍼링크로 URL 을 찍는 출력이다.

## 원인 — 서로 다른 두 경로

### 경로 A. 터미널 OSC 8 하이퍼링크가 앱 핸들러를 거치지 않는다 (증상 2)

`terminal-view.tsx` 의 Terminal 생성 옵션에 `linkHandler` 가 없었다. xterm 코어는 자체
`OscLinkProvider` 를 **우선순위 0** 으로 먼저 등록하고 같은 셀에서 겹치는 하위 링크를 제거하므로,
OSC 8 링크는 우리가 주입한 `WebLinksAddon` 핸들러(수식어 게이트 + `onOpenLink`)에 **도달조차 하지
않는다.** 대신 xterm 의 `defaultActivate` 가 실행되는데, 그 구현은

```
confirm(`Do you want to navigate to ${uri}?…`) → window.open() → newWindow.location.href = uri
```

이고 wry 0.55.1 은 `runJavaScriptConfirmPanel` 을 구현하지 않는다(grep 0). 데스크톱에서 `confirm()`
은 항상 false 로 돌아오므로 그 자리에서 조용히 끝난다 — 이것이 "아무 반응 없음"이다. 평문 URL 링크는
`WebLinksAddon` 이 잡아 정상 동작했기 때문에 같은 화면에서 두 결과가 섞여 보였다.

### 경로 B. 마크다운 프리뷰 앵커에 클릭 가로채기가 없다 (증상 1 — 확정 재현)

`shared/lib/markdown.ts` 의 `marked` 는 `target="_blank"` 를 붙이지 않고,
`markdown-preview.tsx` 의 `dangerouslySetInnerHTML` 이후 앱 어디에도 `a[href]` 클릭을 가로채는 코드가
없었다(grep 0). `tauri_plugin_opener::init()` 이 주입하던 클릭 인터셉터는 `target="_blank"`/Ctrl/Shift
클릭만 대상으로 하고, 잡더라도 capability 에 `opener:allow-open-url` 이 없어 `preventDefault` 만 하는
**죽은 인터셉터**였다. 결과적으로 프리뷰의 평범한 `<a href="https://…">` 좌클릭은 webview 를 그대로
외부 오리진으로 네비게이트한다. WebView 레벨 가드도 없었다 — wry 0.55.1 의
`wry_navigation_delegate.rs` 는 핸들러가 없으면 모든 네비게이션을 허용한다.

### 부수 원인. 열기 순서가 뒤집혀 있었다

`terminal-link-opener.ts` `openTerminalLink` 는 **데스크톱에서도 `window.open()` 을 먼저** 시도하고
`null` 일 때만 `system_open_external_url` IPC 로 폴백했다. 팝업이 성공으로 보이는 환경에서는 IPC
경로가 영원히 실행되지 않고, 열린 창은 앱 webview 안이다. `terminal-link-opener.test.ts:5-14` 가
"창이 돌아오면 IPC 를 부르지 않는다"를 단언해 이 순서를 테스트로 고정하고 있었다.

## 미확증 사항 (반증된 가설 — 실측 미확인)

조사 초기 가설은 "데스크톱 `window.open()` 이 non-null 을 반환해 경로 B 대신 경로 A 로 창이 열린다"
였다. **소스로는 반증됐다** — wry 0.55.1
`wry_web_view_ui_delegate.rs::create_web_view_for_navigation_action` 은 `new_window_req_handler` 가
없으면 `None` 을 반환하므로 소스상 데스크톱 `window.open()` 은 `null` 이다. 다만 **실제 실행으로 이
반환값을 확인하지는 못했다.** 어느 쪽이든 아래 해결은 유효하다 — 데스크톱에서 `window.open` 을 아예
호출하지 않고, WebView 가드로 창내 네비게이션 자체를 봉쇄하기 때문이다. 사용자가 본 창내 열림이 경로
A·B 중 어느 쪽이었는지도 실기 로그가 없어 단정하지 않는다(경로 B 는 소스상 확정 재현, 경로 A 는
`confirm()` 이 false 인 한 무동작).

## 해결

1. **환경별 경로 분리 (`shared/lib/external-url-opener.ts` `createExternalUrlOpener`)** — 데스크톱은
   `system_open_external_url` IPC **만**, 원격 미러는 `window.open` **만** 쓴다. 폴백 체인을 없애
   "데스크톱에서 `window.open` 을 부르지 않는다"를 테스트로 고정했다. 환경 판별은
   `shared/lib/remote/runtime-environment.ts` `isRemoteMirrorRuntime`(원격 shim 라벨 재사용).
2. **앱 단일 진입점 `entities/system/external-url.ts` `openExternalUrl`** — 외부 URL 을 여는 코드는
   전부 이 함수만 쓴다. `terminal-link-opener.ts` 와 그 테스트는 삭제했다.
3. **OSC 8 게이트 (`terminal-view.tsx`)** — Terminal 옵션에
   `linkHandler: { activate: 수식어 게이트 → onOpenLink, allowNonHttpProtocols: false }` 추가. 평문
   URL·`path:line:col`·OSC 8 세 경로가 같은 게이트와 같은 오프너를 탄다.
4. **앵커 위임 핸들러 (`app/providers/external-link-provider.tsx`)** — `document` capture-phase
   `click` 에서 `composedPath()` 의 첫 앵커를 찾아, 순수 판정
   `shared/lib/external-anchor.ts` `shouldOpenAnchorExternally(href, appOrigin)`(절대 http(s) + 다른
   오리진만 참)이 참이면 `preventDefault()` + `openExternalUrl`. `app.tsx` 의 메인·보조 창 두 분기
   모두에 마운트한다. 죽은 인터셉터는 `tauri_plugin_opener` 를 `open_js_links_on_click(false)` 로
   빌드해 끈다.
5. **WebView 네비게이션 가드 (Rust)** — `on_navigation` 은 앱 스킴·dev 오리진만 허용하고,
   `on_new_window` 는 http(s) 검증 후 OS 브라우저로 열고 항상 `Deny` 한다. JS 경로가 새더라도 앱 창이
   오리진 밖으로 이동하지 않는 2중 방어다. 상세는 `docs/architecture.md`.

## 검증

- `src/shared/lib/external-url-opener.test.ts` — 데스크톱은 셸만 호출하고 `openViaBrowser` 는 호출
  횟수 0, 셸 실패 시 브라우저로 폴백하지 않고 에러 전파, 원격은 브라우저만 호출, 원격에서 브라우저가
  `null` 이면 throw, `openViaBrowserWindow` 3건(정상·거부·`opener` 재할당 예외).
- `src/shared/lib/external-anchor.test.ts` — http(s) 외부/동일 오리진/포트 상이/`mailto:`·
  `javascript:`·`file:`·`blob:`·`tauri:`/상대 경로/빈 문자열.
- `src/shared/lib/remote/runtime-environment.test.ts` — `remote`/`main`/`editor-1`/빈 라벨.
- `src/features/terminal/terminal-view.test.ts` — 기존 `shouldActivateTerminalLink` 6건이 OSC 8
  경로의 게이트도 함께 보증한다(같은 함수를 `linkHandler.activate` 가 재사용).
- 실기 확인이 남는 항목(DOM·webview 필요): 마크다운 프리뷰의 외부 링크 좌클릭이 OS 브라우저로 열리는
  것, OSC 8 을 찍는 CLI(gh·vite·bun) 출력의 ⌥클릭, 원격 미러에서 새 탭으로 열리는 것.
