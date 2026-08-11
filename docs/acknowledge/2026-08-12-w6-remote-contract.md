# W6 remote-control 구현 계약 (2026-08-12)

> 정찰 리서치(wf_5639db9b-912, opus+medium 5영역) 결과를 메인이 코드로 교차검증한 뒤 확정한 구현 계약.
> 상위 결정은 `2026-08-11-qa5-batch-decisions.md` §1·§2. 이 문서는 그 위의 **구현 세부 확정**이다.
> 리서치 보고서 원문: `$CLAUDE_JOB_DIR/tmp/w6-research-{1..5}-*.md`(세션 소멸 시 접근 불가 — 요지는 이 문서에 반영).

## 1. 확정 사실 (소스 검증됨)

- 커맨드 **131종** = `collect_commands!` 128 + `RAW_CHANNEL_COMMANDS` 3(`pty_spawn`·`pty_attach`·`file_read_raw`, `lib.rs:230`). 전부 `pub async fn`. `Window`/`WebviewWindow` 파라미터 0개.
- `AppHandle` 의존 46종은 이벤트 emit(41) + 프로세스 전역 API(5: asset scope·lsp spawn·ide start/stop·hooks server·settings toggles). 원격 no-op 필요 커맨드 **0종** — 디스패처가 `app.clone()` 주입.
- `Channel::<T>::new(closure)` public·Webview 불요(`tauri/src/ipc/channel.rs:212`). 프론트가 넘기는 실체는 `"__CHANNEL__:<id>"` 문자열(`IPC_PAYLOAD_PREFIX`). 브리지가 자체 Channel 생성해 커맨드에 주입. `on_drop=None` → **종료 신호는 브리지 프로토콜이 명시적으로** 보내야 함(JS Channel 은 `{end:true,index}` 로 unregister).
- Channel 페이로드: `search_run`=`Channel<SearchMatch>`(JSON), `lsp_spawn`=`Channel<String>`(문자열 JSON, 이중 인코딩 — 데스크톱과 동일하게 유지), `pty_spawn`/`pty_attach`=`Channel<InvokeResponseBody>`(**바이너리** `Raw(Vec<u8>)`). `index` 는 Rust AtomicUsize 단조 카운터 → 브리지 서버가 발급해 순서·end 보장.
- 인자 매핑: 매크로가 컴파일타임에 Rust 파라미터명 → `heck::ToLowerCamelCase`. rename_all 미사용 → 131종 전부 camelCase 키. 값 추출 = 최상위 JSON 객체를 camelCase 키로 get 후 serde 역직렬화. 없는 키 + `Option<T>` = `None`.
- reject 값 = `serde_json::to_value(&AppError)` = `{"code":..,"message":..}` plain object. `bindings.ts` `typedError` 가 `e instanceof Error` 면 throw → **브리지는 반드시 Error 아닌 plain object 로 reject**. 성공 값 = `serde_json::to_string(&T)` 를 JSON.parse.
- 이벤트 22종: `Emitter::emit` 이 웹뷰·Rust 리스너 양쪽 배달(`tauri/src/manager/mod.rs:534`). `setup()` 에서 이름별 `AppHandle::listen_any` 등록 → emit 커맨드 **무수정** 팬아웃. 이름 == `E::NAME`(public const). tauri_specta `listen_any` 는 역직렬화 후 expect 패닉 → **tauri raw `listen_any`** 사용(payload 문자열 그대로 전달). dead event: `AppReady`·`TerminalCwdChanged`.
- shim 최소 표면(`__TAURI_INTERNALS__`): `isTauri`, `metadata`(동기, `{currentWindow:{label:'main'},currentWebview:{label:'main'}}`), `transformCallback`/`unregisterCallback`/`runCallback`/`callbacks`, `invoke`, `convertFileSrc`, `plugins:{}`, 그리고 별도 전역 `__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener`. 참조 구현 = `@tauri-apps/api/mocks.js` `mockInternals()`. 이벤트는 전부 `invoke('plugin:event|listen'/'unlisten'/'emit'/'emit_to')` 경유 → shim 이 로컬 가로채 eventId→handlerId 맵 관리. 배달 형태 `runCallback(handlerId, {event, id, payload})`.
- invoke 인자 직렬화(`process-ipc-message-fn.js`): top-level ArrayBuffer/TypedArray 는 octet-stream, 그 외 JSON.stringify + replacer(`__TAURI_TO_IPC_KEY__` 치환, Channel 은 중첩 위치에 있어 재귀 순회 필요 — 예: terminal `{opts, onData}`).
- shim 주입 시점: `metadata` 동기 접근 → 앱 번들 **평가 전** 실행 필수. dist index.html 은 `/assets/index-*.js` type=module → 그 앞 클래식 script 삽입이면 보장.
- axum **0.8.9**, MSRV 1.80(현재 `rust-version=1.77.2` 상향), `default-features=false, features=[http1,tokio,ws]`. tower-http 불요, 미들웨어 `middleware::from_fn`, 쿠키 헤더 수동, Range 이식. 신규 승인 = **axum 단독**(승인분). ws 는 tokio-tungstenite 0.29 유입 → ide/server.rs 의 0.30 과 이중 컴파일(타입 혼용 안 하면 무해).
- asset_resolver: prod=임베드+CSP주입, dev(devUrl)=`../dist` 디스크 직접 읽기(CSP 미주입). SPA 폴백 내장.
- Range 실사용: preview-pane video/audio 2줄(convertFileSrc). 그 외 프리뷰는 `file_read_raw`(ArrayBuffer). CSP media-src 'self' 포함 → 동일 오리진 Range 라우트는 CSP 완화 불요.
- 재사용 선례: `domain/ide/service.rs` `constant_time_eq:46`·`generate_auth_token:53`·`random_port:61`; `domain/agent/hooks.rs` `bind("127.0.0.1:0")`; `domain/ide/server.rs` broadcast(String,cap 64)+세션별 subscribe+mpsc writer 태스크.

## 2. 확정 설계

### 2.1 서버·수명
- 앱당 1개. `RemoteStore`(app.manage, parking_lot::Mutex): running 플래그·포트·1회용 토큰·쿠키 세션 토큰 집합·접속 수·broadcast Sender·shutdown 핸들. `take_shutdown_state()` 관용구로 이중 shutdown 차단.
- 포트 OS 할당(`bind 127.0.0.1:0` → `local_addr().port()`). 기본 루프백 바인드.
- 종료 배선: `lib.rs` `RunEvent::ExitRequested|Exit` + `settings/commands.rs` 토글 지점(ide 선례와 동일 2곳). WS 세션은 watch 신호 + select 로 graceful shutdown 이 걸리게(영구 연결이 앱 종료를 막지 않도록).

### 2.2 인증 (1회용 URL 토큰 → HttpOnly 쿠키)
- 설정에서 링크 발급 → `generate_auth_token()` 1회용 토큰을 URL 쿼리(`?t=<token>`)에 담아 반환. `remote_issue_link`.
- 브라우저가 그 URL 로 첫 접속 시: 서버가 1회용 토큰을 `constant_time_eq` 로 검증·소모하고, 세션 쿠키(`Set-Cookie: <name>=<session>; HttpOnly; SameSite=Strict; Path=/`) 로 승격. 이후 요청·WS 핸드셰이크는 쿠키로 인증.
- LAN 평문은 secure context 아님 → `Secure` 쿠키·`__Host-`/`__Secure-` 접두사 사용 불가. Origin/Host 화이트리스트 검사(CSWSH·DNS rebinding 방어) + 짧은 TTL 로 보완.
- 토큰 비교는 sha2 다이제스트 후 `constant_time_eq`. **1회용 토큰 URL 은 로그·토스트·IPC 응답에 재노출 금지**(발급 응답 1회만).
- 설정 UI 커맨드: `remote_status`(running·port·url 없이 상태·접속 수), `remote_start`/`remote_stop`(또는 settings 토글), `remote_issue_link`(1회용 URL 반환), `remote_revoke_sessions`(전 쿠키 세션 무효화). 정확한 커맨드 집합은 스파인에서 확정.

### 2.3 디스패치
- `domain/remote/dispatch.rs`: `match name` 으로 **원본 fn 직접 호출**(IPC 계층 우회). State 는 `app.state::<T>()`, AppHandle 은 `app.clone()`. 성공=`serde_json::to_string`, 실패=`serde_json::to_value(&AppError)` plain object. raw 3종(`file_read_raw` 응답 바이너리·`pty_*` 채널 바이너리)은 별도 시그니처 그룹.
- Channel 커맨드: 브리지가 `Channel::new(move |body| ws_send(...))` 생성·주입. body `Json(String)`→텍스트·`Raw(Vec<u8>)`→바이너리 프레임. 채널 종료 시 명시적 `{end}` 프로토콜 메시지.
- **파리티 테스트**(SSOT 대신 대조 방식): `include_str!("../../src/shared/api/bindings.ts")` 정규식으로 `__TAURI_INVOKE("name")` 128종 추출 + `RAW_CHANNEL_COMMANDS` 3종 → 디스패치 테이블 이름 집합과 양방향 차집합 공집합 단언. 신규 커맨드 추가 시 반드시 실패. 에러 포맷 스냅샷(`AppError`→`{code,message}`)도 고정.

### 2.4 WS 프로토콜
- 요청: `{seq, command, args}`(+ 채널 파라미터는 args 내 `"__CHANNEL__:<id>"` 문자열 유지, 브리지가 seq·채널 id 로 상관). 응답: `{seq, ok|err, payload}`. 바이너리 응답은 별도 프레임 타입 or seq 헤더+바이너리.
- 채널 메시지: `{channelId, index, message}` / `{channelId, index, end}`(index 서버 발급).
- 이벤트 브로드캐스트: `{event, payload(문자열)}`. shim 이 JSON.parse 후 `runCallback`.
- seq 로 응답 순서 무관하게 상관(요청별 태스크 spawn 허용).

### 2.5 서빙
- prod=`asset_resolver().get(path)`(임베드+CSP), dev=vite(5173) 리버스 프록시(HMR ws 포기).
- **[정정 2026-08-12] shim 은 별도 bootstrap.js 라우트가 불필요하다.** `@tauri-apps/api` 가 metadata 를 호출 시점(getCurrentWindow 등)에 읽으므로, shim 을 `src/main.tsx` **첫 import** 로 넣고 `if (!window.__TAURI_INTERNALS__)` self-gate 하면 데스크톱은 no-op·브라우저(원격)만 활성이며 앱 번들 평가 전 설치가 보장된다. CSP `script-src 'self'` 문제도 소멸(인라인 아님, 번들 일부). 스파인이 만든 `/__taide/bootstrap.js` 스텁 라우트는 미사용으로 제거하거나 빈 채로 둔다.
- 서빙이 할 일: 동일 dist 번들 서빙(prod asset_resolver / dev vite proxy) + **CSP 헤더 override**(connect-src 에 동일 오리진 ws 추가, asset:/ipc: 제거).
- 웹용 CSP: asset:/ipc: 제거, `connect-src` 에 ws(동일 오리진) 추가, media-src 'self' 유지(Range 라우트 동일 오리진).
- convertFileSrc 대체: `/__taide/file?path=` HTTP Range 라우트. tauri asset.rs Range 로직 이식(206/416, 단일 range 1000KiB 상한, Accept-Ranges). **인증 + 프로젝트 루트 스코프 검사 자체 구현**(asset 프로토콜은 웹뷰 전용이라 인증 없었음 — root_guard `ensure_within_root` / `AppState.projects` 화이트리스트 재사용). no-store.

### 2.6 프론트 shim
- `src/shared/lib/remote/` 에 shim. 원격 오리진에서만 활성(부트스트랩이 주입). `__TAURI_INTERNALS__` 최소 표면 + `__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener` 구현. invoke→WS RPC, Channel→WS 채널, 이벤트 listen→로컬 맵+WS 구독, convertFileSrc→`/__taide/file?path=` URL.
- 데스크톱 번들 무영향(shim 은 원격 서빙 시에만 로드). bindings.ts·13개 접점 무변경이 목표.

## 3. 원격 UX 정책 (기본값 — 기술적으로 강제되거나 최소·가역)

| 대상 | 정책 | 근거 |
|------|------|------|
| 네이티브 다이얼로그 open/save (app-shell·app-sidebar·untitled-pane·vsix 4곳) | shim 이 가로채 **null(취소) 반환** + 안내 토스트. 원격 파일 피커는 backlog | 호스트에 다이얼로그 띄우면 원격 요청이 **행(hang)**. 포워딩 불가. 호출부 4곳 모두 null=취소 처리 확인됨 |
| `system_open_path`/`reveal`/`open_in_browser`/`open_app_data_path` | 호스트에서 실행(무해·비블로킹), 시그니처 무변경 | 원격 사용자가 결과를 못 보지만 hang 없음. 차단은 과함 |
| 창 API: `show()`(reveal)·`setTheme()`·`onFocusChanged`·`onDragDropEvent` | shim 로컬 no-op/대체(focus 는 브라우저 focus, drag-drop 은 원격 미전달) | 호스트 창 상태를 브라우저에 오인 전달 방지. `refetchOnWindowFocus:false` 라 실피해 낮음 |
| clipboard 7곳 | 그대로(LAN 평문은 degrade, tunnel HTTPS 는 정상) | secure context 제약 — 문서화만 |
| 이벤트 스코프 | 전 22종 무차별 팬아웃(모든 프로젝트 상태 노출) | 사용자 확정 "데스크톱과 상태 완전 공유". 세션별 필터 없음 |

## 4. 미결·위험 (구현 중 확인)

- 실제 컴파일(State lifetime `.await` 가로지르기·Send 경계·매크로 전개) — cargo 로만 확인.
- tokio-tungstenite 0.29/0.30 이중 컴파일 빌드영향 — 타입 혼용 안 하면 무해, 실측.
- broadcast Lagged 유실 → 재연결 refetch 로 재동기화. 용량 상향(근거 없는 추정치 — 실측 전 256 가정).
- dev 리버스 프록시에서 vite allowedHosts 거부 여부·shim 로드 순서 보장 — 실측.
- monaco/xterm/pdfjs 워커가 원격 오리진 `worker-src 'self' blob:` 하 동작 — QA6 실측.
- `mime_type` 공개 API 가부 — 불가 시 자체 확장자 테이블(신규 의존 회피).
- rust-version 1.80 상향이 다른 의존과 충돌 없는지.
