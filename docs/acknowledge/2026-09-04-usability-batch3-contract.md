# 사용성 배치 3 계약 — 퀵오픈 미발견 · 터미널 링크 창내 열림 · 파일트리 자동 reveal (2026-09-04)

> 사용자 보고 3건. 요구: **근본 수정** + **향후 고도화를 고려한 추상화**. 진행은 역할표
> (`docs/agent-operations.md` §1, 2026-09-04 갱신) — 구현 wf opus·xhigh → 리뷰 wf sonnet·xhigh →
> 테스트 wf fable·medium. 조사는 이 세션에 한해 Explore 3개 결과를 소비했다(이후 리서치는 wf).
> 상태 추적은 `docs/PROCESS.md` "사용성 배치 3" 절.

## 0. 공통 규칙 (구현·리뷰·테스트 전 에이전트)

- 컨벤션 전부 준수: `~/.claude/convention/*.md` (arrow fn 만 · 코드 주석 금지(JSDoc 영어만) ·
  any/enum 금지 · 매직넘버 금지 · useCallback/useMemo 금지 · FSD 위→아래 · barrel 금지 · 타입은
  원본에서 유도). 기존 코드 패턴을 따르고 요청 밖 리팩토링 금지(minimal diff).
- 로케일은 `src-tauri/resources/locales/{ko,en,ja}.json` 3언어 동시 추가(키 알파벳 정렬 유지).
- bindings(`src/shared/api/bindings.ts`)는 specta 자동 생성 — Rust 타입 변경 후 `cargo test`
  (또는 `bun run rust:test`)로 재생성·검증한다. 손으로 고치지 않는다.
- 검증 사다리: `bun run typecheck` → `bun run lint` → `bun run format:check` → `bun test` →
  Rust 변경 시 `bun run rust:fmt`·`rust:lint`·`rust:test`. cargo PATH:
  `export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup"; export PATH="$CARGO_HOME/bin:$PATH"`
- 커밋·푸시 금지(메인이 마지막에 수행). 워킹트리에만 남긴다.
- 설명·트레이드오프는 코드 주석이 아니라 `docs/features/*.md` 에 남긴다.

## A. 퀵오픈(⌘P) 파일 열기 "찾을 수 없는 파일"

### A.1 현행 사실 (조사 완료 — 구현 전 실물 재확인 필수)

- 후보 목록: `command-palette.tsx:105-108` `useQuery({ ...projectFilesQueryOptions(activeProjectId),
  enabled: open && mode === 'files' && !!activeProjectId })` → `entities/search/search.query.ts:27-32`
  (`QUERY_KEY.SEARCH.PROJECT_FILES(projectId)`) → `search_list_files` (`src-tauri/src/domain/search/
  commands.rs:226`, 매 호출 전체 walk·절대 경로·`to_string_lossy`). Rust 측 캐시 없음, **캐시는 프론트
  TanStack Query 뿐**(전역 `staleTime 60_000`·`retry 0`·`refetchOnWindowFocus false` —
  `src/app/query-client.ts`).
- 그 캐시를 무효화하는 **유일한 경로**는 `ipc-sync-provider.tsx:362` 의 fs 워처 에코
  (`kind !== 'modified'` 일 때 invalidate) 이고, 워처는 300ms 디바운스(`constants.rs:17
  WATCH_DEBOUNCE_MS`). 앱 내부 뮤테이션은 무효화하지 않는다: `file.query.ts:88 useCreateEntry`(무효화 0),
  `:116 useRenameEntry`·`:133 useCopyEntry`·`:151 useDeleteEntry`(FILE.CONTENT·GIT.PROJECT 만).
- 팔레트가 닫혀 있으면 `enabled:false` 라 invalidate 는 stale 마킹만 하고, 다시 열면 **옛 배열을 즉시
  렌더하며 백그라운드 re-walk** — `isFetching`/`isError` 를 읽지 않아(`:105`, 로딩 표시는
  `isPending` 만 `:269`) 낡은 행이 그대로 클릭 가능.
- 클릭: `command-palette-files-group.tsx:23` `onSelect={() => onOpenFile(path)}` →
  `command-palette.tsx:284-291` `openTab({ kind:{kind:'file',path}, title:fileNameOf(path), target:null,
  preview:true }, { onError: toast })`. **`layout_open_tab` 은 경로 존재를 검증하지 않는다**
  (`layout/service.rs:1276-1289`) → 탭이 열린 뒤 `editor-pane.tsx:67` `fileQueryOptions` →
  `file_open` → `file/service.rs:71` `std::fs::metadata(path)?` ENOENT.
- 에러 문구: `error.rs:83-89` `From<io::Error>` 가 `AppError::NotFound(value.to_string())` (localeKey
  없음) → `use-ipc-error-message.ts:14-16` 원문 폴백 → 에디터 본문 빨간 텍스트(`editor-pane.tsx:277-279`)
  `No such file or directory (os error 2)`. **로케일 키 `error.file.notFound` 는 존재하지 않는다.**
- 기각된 원인: cmdk index 밀림(클로저 캡처·`shouldFilter={false}`)·다중 루트 혼입(전부 `activeProjectId`
  키잉)·심링크/대소문자/NFC(루트 canonicalize·정규화 코드 0)·경계 검사(루트 밖은 Forbidden).
- 부수 결함: `search_list_files` 가 비-UTF8 파일명을 U+FFFD 로 치환한 **존재하지 않는 경로**를 내보낸다
  (`commands.rs:230`). 별개 결함: `target:null` 이 가리키는 `focusedPane` 이 사라졌으면
  `layout/service.rs:429` `NotFound("pane not found")` 토스트 — 이번 범위 밖(백로그 등재만).
- `kind:'file'` 탭을 여는 호출부 10곳(전부 검증 없음): `command-palette.tsx:287,309` ·
  `explorer-container.tsx:82,86,121` · `search-editor-pane.tsx:100` · `git-panel-container.tsx:196` ·
  `problems-panel-container.tsx:53` · `editor-area.tsx:277` · `breadcrumbs-bar.tsx:116` ·
  `welcome-container.tsx:77` · `agent-external-open-provider.tsx:32`.
- 선례: `entities/layout/layout.query.ts:80-90` `useOpenAppFileTab` — "appFile 탭 열기 + 에러 토스트"를
  한곳에 모은 공용 훅. `useOpenTabInProject`(`:99`)는 projectId 를 변수로 받는 변형.

### A.2 설계 — 근본 원인 2축(인덱스 신선도 + 열기 검증)을 각각 단일 지점에서 해결

1. **Rust — 파일 탭 열기 선검증 (command 층)**: `layout::commands::layout_open_tab` 에서 `kind` 가
   `TabKind::File { path }` 이면 `root_guard::resolve_owning_project(&projects, Path::new(&path))` 로
   경계 검사 후 `std::fs::metadata(..)?.is_file()` 이 아니면 **탭을 열지 않고**
   `AppError::localized(AppErrorKind::NotFound, "error.file.notFound", format!("file not found: {}", ..))
   .with_arg("path", ..)` 를 반환한다. `service::open_tab` 은 손대지 않는다(레이아웃 서비스 테스트가
   가짜 경로로 탭을 여는 것을 유지). 구현 전 Rust `open_tab(` 호출부와 TS `kind: 'file'` 호출부를
   grep 해 "아직 존재하지 않는 경로로 파일 탭을 여는" 정당한 흐름이 없는지 확인하고, 있으면 계약을
   깨지 말고 보고한다. 원격 dispatch 경로(`remote/dispatch.rs`)는 같은 command 를 타므로 자동 적용.
2. **Rust — `file::service::open_file`**: `metadata` 의 `io::ErrorKind::NotFound` 만 같은
   `error.file.notFound` localized 에러(`{{path}}` arg)로 매핑한다(다른 io 에러는 기존 `?` 유지).
   에디터 본문·토스트에서 원문 대신 로케일 문구가 보이게 된다. 로케일 3언어 신설:
   `error.file.notFound` — ko "파일을 찾을 수 없습니다: {{path}}" / en "File not found: {{path}}" /
   ja "ファイルが見つかりません: {{path}}". `locale/service.rs` 의 키 목록/backfill 규약이 있으면 등재한다.
3. **Rust — `search_list_files`**: `to_string_lossy` 대신 `to_str()` 이 `None` 인 엔트리는 제외
   (`filter_map`) + 단위 테스트 1건(비-UTF8 이름은 목록에 없음, unix `OsStrExt`). `ipc-contract.md`
   `search_list_files` 항목에 "비-UTF8 경로는 제외" 명시.
4. **FE — 파일 탭 열기 단일 진입점 `useOpenFileTab`** (`entities/layout/layout.query.ts`,
   `useOpenAppFileTab` 옆): `useOpenTabInProject` 위에 얹어 projectId 를 호출 시 받는다.
   ```ts
   type OpenFileTabRequest = { projectId: ProjectId; path: string; preview: boolean; target: PaneId | null; title?: string }
   export const useOpenFileTab = () => {
       const queryClient = useQueryClient()
       const { mutate: openTab } = useOpenTabInProject()
       return (request: OpenFileTabRequest, callbacks?: { onSuccess?: () => void; onError?: (error: unknown) => void }) =>
           openTab({ projectId, kind: { kind: 'file', path }, title: title ?? fileNameOf(path), target, preview }, {
               onSuccess: callbacks?.onSuccess,
               onError: (error) => {
                   if (isNotFoundIpcError(error)) void queryClient.invalidateQueries({ queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId) })
                   toast.error(describeIpcError(error))
                   callbacks?.onError?.(error)
               },
           })
   }
   ```
   `isNotFoundIpcError` 는 `error instanceof IpcError && error.code === 'NotFound'`(`unwrap-result.ts` 의
   code 필드 실물 확인). **위 10곳 전부 이 훅으로 치환**하되 각 호출부의 `preview`/`target`/title 의미는
   그대로 보존한다(title 이 `fileNameOf(path)` 가 아닌 곳이 있으면 `title` 옵션으로 전달). 호출부가
   이미 갖고 있던 동일한 `toast.error(describeIpcError(error))` 는 제거(중복 토스트 금지), 추가 후처리는
   `callbacks` 로 유지. 이 훅이 이후 MRU 기록·autoReveal 등 "파일 열기" 고도화의 유일한 확장 지점이다.
5. **FE — 인덱스 신선도 계약**: `file.query.ts` 의 `useCreateEntry`·`useRenameEntry`·`useCopyEntry`·
   `useDeleteEntry`(+ 같은 파일의 다른 fs 뮤테이션이 있으면 전부) `onSuccess` 에서
   `queryClient.invalidateQueries({ queryKey: QUERY_KEY.SEARCH.PROJECT_FILES(projectId), refetchType: 'all' })`
   — 팔레트가 닫혀 있어도 즉시 재-walk 해 다음 ⌘P 가 신선하게 열린다(사용자 1회 조작 = walk 1회).
   `useCreateEntry` 는 현재 projectId 를 받지 않으므로 `useCreateEntry(projectId)` 로 시그니처를 맞추고
   호출부(`use-explorer-entry-crud.ts` 등)를 갱신한다. 4곳 이상 반복이므로 파일 내 헬퍼
   `invalidateProjectFileIndex(queryClient, projectId)` 로 공통화. 워처 에코(`ipc-sync-provider.tsx:362`)
   는 그대로 둔다(외부 대량 변경 시 refetchType 'all' 은 walk 폭주 위험).
6. **FE — 팔레트 갱신 중 표시**: `command-palette.tsx` 가 `isFetching` 을 읽어
   `CommandPaletteFilesGroup` 에 `isRefreshing` 을 넘기고, 그룹 헤딩 옆에 로케일 문구
   `palette.filesRefreshing`(ko "목록 갱신 중" / en "Refreshing" / ja "更新中") 을 기존 로딩 표시와 같은
   스타일로 보여준다. 항목 클릭은 막지 않는다(선검증 1 이 방어).
7. **문서**: `docs/features/command-palette.md` §3 에 "인덱스 신선도 규약(인앱 뮤테이션 즉시 무효화 ·
   워처 에코 · 열기 선검증)" 절, `docs/ipc-contract.md` `layout_open_tab`(File kind 선검증·NotFound
   localized)·`search_list_files`(비-UTF8 제외) 갱신, `docs/bug/2026-09-04-quick-open-stale-index-not-found.md`
   신설(증상·원인·해결), `docs/backlog.md` 에 "target:null pane NotFound 토스트" 등재.

### A.3 기각·보류

- Rust 측 FsChanged 기반 인덱스 캐시(`backlog.md:102`): 별도 배치. 이번엔 무효화 계약으로 스테일 창을
  닫는다.
- 열기 시 in-flight fetch 를 await 해 재검증: 선검증(1)으로 충분하고 지연만 추가 → 기각.

## B. 터미널 ⌥클릭 링크가 TAIDE 창 안에서 열림

### B.1 현행 사실 (조사 완료 + 메인 소스 확증 — 구현 전 실물 재확인 필수)

- 열기 순서가 뒤집혀 있다: `terminal-session.tsx:176-181` `handleOpenLink` → `terminal-link-opener.ts:6-8`
  `openTerminalLink` 가 **데스크톱에서도 `window.open()` 을 먼저** 시도(`openViaBrowserWindow`: 인자
  없는 `window.open()` → `opener=null` → `location.href=uri`)하고 null 일 때만 `systemOpenExternalUrl`
  IPC 로 폴백한다. `terminal-link-opener.test.ts:5-14` 는 "창이 돌아오면 IPC 를 부르지 않는다"를
  단언해 이 순서를 테스트로 고정하고 있다.
- **OSC 8 하이퍼링크가 우리 핸들러를 거치지 않는다**: `terminal-view.tsx:178-192` Terminal 옵션에
  `linkHandler` 가 없다. xterm 6.0 코어는 `OscLinkProvider` 를 **우선순위 0** 으로 먼저 등록하고
  (`CoreBrowserTerminal.ts:160`), 같은 셀에서 겹치는 하위 링크는 제거된다(`Linkifier.ts:153-173`).
  OSC 8 링크(gh·vite·bun·eza·Claude Code 출력)는 `OscLinkProvider.defaultActivate` →
  `confirm(...)` → `window.open()` 경로로 가며 **수식어 게이트가 없다**. wry 0.55.1 은
  `runJavaScriptConfirmPanel` 을 구현하지 않아(grep 0) 데스크톱에서 `confirm()` 은 false → 조용히
  무동작. "어떤 링크는 열리고 어떤 링크는 아무 반응 없음"의 원인.
- WebView 레벨 가드 0: `src-tauri` 에 `on_navigation`/`on_new_window` 없음. wry 0.55.1
  `wry_navigation_delegate.rs:118-122` 는 핸들러가 없으면 **모든 네비게이션 허용**이고, 프레임을
  구분하지 않는다(iframe 네비게이션도 같은 정책 함수 통과). `create_web_view_for_navigation_action`
  (`wry_web_view_ui_delegate.rs:141-258`)은 `new_window_req_handler` 가 없으면 None → 소스상 데스크톱
  `window.open()` 은 null. **"데스크톱에서 non-null 이 돌아온다"는 B.0 의 초기 가설은 소스로
  반증되었고 실측 미확증** — 어느 쪽이든 아래 설계는 유효하다(창내 네비게이션 자체를 구조적으로
  봉쇄).
- 마크다운 프리뷰 앵커는 **확정 재현되는 창내 열림 경로**: `shared/lib/markdown.ts` `marked` 는
  `target=_blank` 를 붙이지 않고, `markdown-preview.tsx:33` `dangerouslySetInnerHTML` 이후 앱 어디에도
  `a[href]` 클릭 가로채기가 없다(grep 0). `tauri_plugin_opener::init()`(`lib.rs:433`) 이 주입하는
  클릭 인터셉터(`open_js_links_on_click` 기본 true)는 `target=_blank`/Ctrl/Shift 만 잡고, 잡아도
  capability 에 `opener:allow-open-url` 이 없어 `preventDefault` 만 남는 **죽은 인터셉터**다.
- 메인 창은 `tauri.conf.json` `app.windows[0]`(label `main`, `visible:false`, Overlay 타이틀바)로
  자동 생성된다. tauri-utils 2.9.3 `WindowConfig.create`(기본 true)를 false 로 두면 자동 생성이
  건너뛰어지고(`tauri/src/app.rs:2524` `filter(|w| w.create)`), 문서화된 패턴
  `WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?` 로 setup 에서
  동일 설정으로 만들며 `.on_navigation(..)`/`.on_new_window(..)` 를 붙일 수 있다. 보조 창은
  `domain/window/commands.rs:102` `WebviewWindowBuilder::new(..)` 로 이미 코드 생성.
- 환경 판별: 원격 미러 shim 은 `getCurrentWindow().label === REMOTE_WINDOW_LABEL('remote')`
  (`tauri-internals-shim.ts:85-87`). `shared/lib/window-context.ts` 는 main/auxiliary 만 구분.
  원격 dispatch 는 `system_open_external_url` 을 명시 거부(`dispatch.rs:519`).
- 앱 오리진: prod `tauri://localhost`(macOS/Linux)·`http://tauri.localhost`(Windows), dev
  `build.devUrl = http://localhost:5173`. CSP `frame-src 'self' blob:` — `preview-pane.tsx:68`
  html 프리뷰 iframe 은 `URL.createObjectURL` blob 을 `sandbox=''` 로 로드한다(`about:srcdoc`·
  `blob:` 네비게이션은 허용해야 한다).

### B.2 설계 — "외부 URL 은 항상 OS 브라우저" 를 단일 추상화 + WebView 가드로 보장

1. **환경 판별 헬퍼** `src/shared/lib/remote/runtime-environment.ts`(신설):
   `isRemoteMirrorRuntime = () => typeof window !== 'undefined' && getCurrentWindow().label === REMOTE_WINDOW_LABEL`
   (`@tauri-apps/api/window`). 전역 상태 추가 없음(shim 이 이미 보고하는 라벨 재사용).
2. **순수 오프너 팩토리** `src/shared/lib/external-url-opener.ts`(신설, `bun:test`):
   ```ts
   export type ExternalUrlOpenerDeps = {
       isRemoteMirror: () => boolean
       openViaShell: (uri: string) => Promise<unknown>
       openViaBrowser: (uri: string) => Window | null
   }
   export const createExternalUrlOpener = (deps: ExternalUrlOpenerDeps) => async (uri: string) => {
       if (!deps.isRemoteMirror()) { await deps.openViaShell(uri); return }
       if (deps.openViaBrowser(uri)) return
       throw new Error(`browser refused to open ${uri}`)
   }
   ```
   데스크톱에서는 **`window.open` 을 절대 부르지 않는다**(테스트로 고정). `openViaBrowserWindow` 는
   `widgets/terminal-pane/terminal-link-opener.ts` 에서 이 파일로 이동(원격 미러 전용, JSDoc 유지).
   `terminal-link-opener.ts` 와 그 테스트는 삭제(대체됨 — 새 테스트가 반대 방향을 단언).
3. **앱 단일 진입점** `src/entities/system/external-url.ts`(신설):
   `export const openExternalUrl = createExternalUrlOpener({ isRemoteMirror: isRemoteMirrorRuntime,
   openViaShell: systemOpenExternalUrl, openViaBrowser: (uri) => openViaBrowserWindow(uri, () => window.open()) })`.
   이후 앱에서 외부 URL 을 여는 모든 코드는 이 함수만 쓴다.
4. **터미널**: `terminal-session.tsx` `handleOpenLink` → `openExternalUrl(uri).catch(() => toast.error(t('terminal.openLinkFailed')))`.
   `terminal-view.tsx` Terminal 옵션에 **`linkHandler`** 추가:
   `{ activate: (event, text) => { if (!shouldActivateTerminalLink(event)) return; onOpenLinkRef.current(text) }, allowNonHttpProtocols: false }`
   — OSC 8 링크도 같은 수식어 게이트·같은 오프너를 탄다(Rust `validate_external_url` 이 http(s) 만
   허용하므로 2중 방어). hover 툴팁은 추가하지 않는다(WebLinksAddon 도 없음 — 기존 동작 일치).
5. **앵커 위임 핸들러** `src/app/providers/external-link-provider.tsx`(신설, `agent-external-open-provider.tsx`
   와 같은 자리): `document` capture-phase `click` 리스너에서 `event.composedPath()` 의 첫 `HTMLAnchorElement`
   를 찾아 `href` 프로토콜이 `http:`/`https:` 이고 오리진이 앱 오리진(`window.location.origin`)과 다르면
   `preventDefault()` + `openExternalUrl(href)`(실패 시 `toast.error(t('common.openExternalLinkFailed'))`).
   버튼(0)·`defaultPrevented` 아닌 경우만. `app.tsx` 루트에 마운트(메인·보조 창 모두 같은 React 앱).
   판정 로직은 순수 함수 `shouldOpenAnchorExternally(anchorHref, appOrigin)` 로 분리해 `shared/lib` 에
   두고 테스트한다. `lib.rs` 의 `tauri_plugin_opener::init()` 은
   `tauri_plugin_opener::Builder::new().open_js_links_on_click(false).build()` 로 바꿔 죽은 인터셉터를 끈다
   (`2026-08-18-hand-qa-fix-contract.md:140` "opener JS+capability 개방 기각" 결정과 정합).
6. **Rust WebView 가드** (2중 방어 — 어떤 JS 경로가 새더라도 앱 창은 오리진 밖으로 이동하지 않는다):
   - `src-tauri/src/infra/navigation_guard.rs`(신설): `pub fn is_navigation_allowed(url: &Url, dev_url: Option<&Url>) -> bool`
     — 허용: scheme `tauri`·`asset`·`about`·`blob`, host `tauri.localhost`/`asset.localhost`/`ipc.localhost`,
     그리고 `dev_url` 과 origin(scheme+host+port)이 같은 URL. 그 외 전부 false + `log::warn!`. 단위
     테스트(허용 6종·거부 http(s) 외부·`javascript:`·`file:`).
   - `pub fn open_new_window_externally(url: Url) -> NewWindowResponse`: `validate_external_url` 통과 시
     `tauri_plugin_opener::open_url` 로 OS 브라우저에 열고 항상 `Deny` (`validate_external_url` 은
     `domain::system::commands` 에서 `pub(crate)` 로 노출하거나 `domain::system::service` 로 이동).
   - `tauri.conf.json` `app.windows[0]` 에 `"create": false` 추가, `lib.rs` `.setup` 의 **가장 앞**에서
     `WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?.on_navigation(..).on_new_window(..).build()?`.
     setup 안에서 `MAIN_WINDOW_LABEL` 창을 참조하는 코드가 모두 그 뒤에 오는지 grep 으로 확인.
     `tauri_plugin_window_state`·`single_instance`·부팅 워처/blank-window 계약(`2026-08-20-*`)과의
     순서 영향을 읽고 보고한다. `dev_url` 은 `app.config().build.dev_url.clone()` 을 클로저에 캡처.
   - `domain/window/commands.rs:102` 보조 창 빌더에도 같은 두 핸들러 부착(정책 함수 공유).
7. **로케일**: `common.openExternalLinkFailed` 3언어(ko "링크를 외부 브라우저로 열지 못했습니다" /
   en "Could not open the link in your browser" / ja "リンクをブラウザーで開けませんでした").
8. **문서**: `docs/features/terminal.md` §6.1 을 새 설계로 전면 교체(원 계약 §2.1 의 "window.open
   선시도" 폐기 사유 명기), `docs/architecture.md`(또는 CSP 를 다루는 기존 문서)에 "WebView 네비게이션
   가드" 절, `docs/bug/2026-09-04-external-link-opens-in-app-window.md` 신설(터미널 OSC 8·마크다운 앵커
   두 경로·미확증 사항), `docs/ipc-contract.md` 변경 없음.

### B.3 기각·보류

- `tauri_plugin_opener` capability 개방 + JS `openUrl` 사용: 원 계약에서 기각(단일 통로 유지) → 유지.
- xterm `linkHandler.hover` 툴팁: 기존 WebLinksAddon 도 없음 → 이번 범위 밖(백로그).
- `mailto:` 외부 열기: Rust 화이트리스트가 http(s) 뿐 → 범위 밖.

## C. 파일트리 자동 reveal (VS Code `explorer.autoReveal` 파리티)

### C.1 현행 사실 (조사 완료 — 구현 전 실물 재확인 필수)

- reveal 인프라는 **이미 완비**: Rust `tree_reveal`(`src-tauri/src/domain/tree/service.rs:275-282`
  `reveal` — 조상 전부 lazy 로드+펼침, `plan_reveal_reads` 로 미캐시 조상 일괄 prefetch) →
  `src/entities/tree/tree.ipc.ts:10` `revealTreeNode` / `tree.query.ts:33-39` `useRevealTreeNode`
  (onSuccess 로 `TREE.ROWS` 전체 페이지 교체) → `explorer-container.tsx:229-234`
  `onRevealInExplorerRequest`(reveal 후 `setSelectPathRequest(path)`) → `file-tree.tsx:239-245`
  `selectPathRequest` 이펙트(행이 아직 없으면 `rows` 갱신 시 재시도) → `selectByIndex`
  (`setSelectedId` + `onSelectionChange` + `rowVirtualizer.scrollToIndex(index)`, **DOM focus 는
  건드리지 않음** → 에디터 포커스를 뺏지 않는다).
- 유일한 발행처는 탭 우클릭 "탐색기에서 보기"(`pane-tab-bar.tsx:205` →
  `shared/lib/bridge/explorer-reveal-bridge.ts` `requestRevealInExplorer`) → `explorer-panel.tsx:116-123`
  구독(`setView('files')` + `onRevealInExplorerRequest`).
- 활성 파일 경로 관용구(5곳 복붙, 전용 hook 없음):
  `const activeTab = layout ? findActiveTab(layout.root, layout.focusedPane) : null;
  const activePath = activeTab?.kind.kind === 'file' ? activeTab.kind.path : null`
  — `outline-panel-container.tsx:31-32`, `command-palette.tsx:111-112`, `title-bar-content.tsx:15`,
  `status-bar-content.tsx:56`, `auxiliary-title-bar-content.tsx:27`.
- 사이드바는 **주창에만** 마운트(`app-shell.tsx:197-206`). 보조창 활성 탭이 주창 트리를 흔들면
  안 되므로 autoReveal 은 `layout.root`/`layout.focusedPane` 주 트리만 본다(`resolveWindowPaneTree`
  사용 금지).
- 사이드바 접힘/Zen 은 Rust `ProjectLayout.shellView.sidebarCollapsed`/`zen`
  (`use-zen-mode.ts:31-32`). `Panel collapsible collapsedSize={0}` 라 접혀도 Explorer 는 언마운트되지
  않는다. 사이드바 내부 뷰(`files`/`search`/`git`/`outline`)는 `explorer-panel.tsx:92` view-local.
- 성능 제약: `useRevealTreeNode` 성공 시 전체 트리 페이지를 재직렬화·교체한다(`ipc-contract.md:234`
  "응답 형태 불변"). 따라서 **이미 보이는 행이면 IPC 없이 선택만** 해야 한다.
- 결함 동반 발견: `app-shell.tsx:132-133` 은 `subscribeOpenSearchPanel`/`subscribeShowExplorerView`
  에만 `expand()` 를 걸어, 사이드바가 접힌 상태에서 탭 우클릭 "탐색기에서 보기"는 무반응처럼 보인다.
- 문서 오류: `explorer-sidebar.md:25-26` "펼침 상태 layout 영속화(재시작 복원)" 은 코드와 불일치
  (`expanded_paths`/`restore_expanded` 호출부 0). 이번 범위에서는 문서만 정정한다(구현 X).
- 로케일은 `src-tauri/resources/locales/*.json`(프론트 `src/shared/i18n/locales` 아님).
- e2e 스펙 13(`13-quick-open-unexpanded-folder.e2e.ts`)의 주석 "This spec never expands `src/`" 은
  autoReveal 기본 ON 이면 앱 쪽에서 `src/` 가 펼쳐지므로 사실과 달라진다 — assert 는 탭 가시성뿐이라
  통과하지만 주석을 갱신한다(테스트 wf).

### C.2 설계 (채택안 — 전용 hook + 순수 판정 함수, 전역 상태 신설 0)

1. **`src/shared/lib/pane-tree.ts` 에 `activeFilePathOf(layout)` 추가** — `findActiveTab` 을 감싸
   `kind.kind === 'file'` 인 경우만 path 반환. 위 5곳 관용구를 이 함수로 치환한다(중복 제거 —
   동작 불변, 각 소비처가 주 트리/창별 트리 중 무엇을 봤는지 그대로 유지. 보조창 타이틀바처럼
   `resolveWindowPaneTree` 결과를 넘기던 곳은 그 결과를 그대로 인자로 넘긴다).
2. **`src/shared/lib/explorer-auto-reveal.ts` (신설, React 무관 순수 함수 + `bun:test`)**
   ```ts
   export type AutoRevealDecision = 'skip' | 'select-only' | 'reveal-then-select'
   export const decideAutoReveal = (input: {
       enabled: boolean
       activePath: string | null
       visiblePaths: ReadonlySet<string>
       sidebarVisible: boolean
       explorerViewActive: boolean
       lastRevealedPath: string | null
   }): AutoRevealDecision
   ```
   규칙: `!enabled || !activePath || !sidebarVisible || !explorerViewActive` → `skip`;
   `activePath === lastRevealedPath` → `skip`(중복 억제); `visiblePaths.has(activePath)` →
   `select-only`; 그 외 → `reveal-then-select`. 테스트 케이스: 비활성/경로 없음/접힘/Zen/비-files
   뷰/중복/이미 보임/미보임 전량.
3. **`src/widgets/explorer/use-explorer-auto-reveal.ts` (신설 훅)** — 형제 훅
   (`use-explorer-clipboard.ts`·`use-explorer-entry-crud.ts`)과 같은 "input 객체 + 컨테이너 mutation
   주입" 시그니처. 내부에서 `layoutQueryOptions(projectId)`·`settingsQueryOptions()` 를 읽고
   `activeFilePathOf(layout)`·`layout.shellView.{sidebarCollapsed,zen}`·`rows`·`explorerViewActive`
   로 `decideAutoReveal` 을 호출, `useEffect` 에서 결과에 따라 `setSelectPathRequest(path)` 또는
   `await revealTreeNode({projectId, path})` 후 `setSelectPathRequest(path)`. `lastRevealedRef`
   (`useRef`) 로 중복 억제. reveal 실패는 조용히 무시(토스트 X — 자동 동작이므로).
   - **활성 파일이 트리 표시 대상 밖**(숨김 디렉토리·프로젝트 루트 밖 파일)이면 `tree_reveal` 이
     에러/무효과일 수 있다 — 실물 확인 후 `ancestors_under_root` 가 빈 경우의 동작을 확인하고,
     프로젝트 루트 밖 경로는 훅에서 선판정으로 `skip` 한다(`project.root` 접두 검사 — 기존 경로
     유틸 `toRelativePath`/`explorer-path.ts` 재사용).
   - 사이드바가 접혀 있다가 펼쳐지거나 뷰가 `files` 로 돌아오면 이펙트 의존성으로 자동 재실행되어
     현재 활성 파일로 reveal 된다.
4. **`explorer-panel.tsx` view 상태를 controlled 로** — `ExplorerContainer` 가 `view`/`onViewChange`
   를 소유해 넘기고, 패널 내부 `setView` 호출(`subscribeOpenSearchPanel`·`subscribeShowExplorerView`·
   `subscribeRevealInExplorer`·탭 클릭)을 `onViewChange` 로 치환. 최소 변경.
5. **설정 `explorerAutoReveal: boolean`, 기본 `true`**(VS Code 기본값 파리티) — d-53 패턴 그대로:
   `types.rs` `Settings`(`#[serde(default = "default_true")]` + doc comment 파리티 근거)·
   `SettingsPatch`·`Default` / `service.rs apply_patch` / `sync/service.rs settings_to_sync_patch` /
   bindings 재생성 / `settings.ipc.ts emptySettingsPatch` / `settings-interface-section.tsx`
   `SwitchField`(`enablePreviewTabs` 옆) / 로케일 `settings.explorerAutoReveal`·
   `settings.explorerAutoRevealDescription` 3언어.
6. **동반 수정**: `app-shell.tsx` 에 `subscribeRevealInExplorer(() => explorerPanelRef.current?.expand())`
   추가 — 명시적 reveal 만 사이드바를 펼친다(autoReveal 은 브리지를 타지 않으므로 자동 구분).
7. `features/explorer/file-tree.tsx` 는 무수정(`selectPathRequest` 계약이 이미 정확). Rust 신규
   커맨드 0(`tree_reveal` 재사용), IPC 계약 문서 변경 없음.
8. 문서: `docs/features/explorer-sidebar.md` §2.2 에 autoReveal 절(게이트 3축·IPC 0 경로·설정) +
   §2.2 펼침 영속화 문구 정정, `docs/features/settings-ui.md` §1.1 설정 1종 추가.

### C.3 기각안

- 전역 브리지(`active-file-bridge`) 신설: `layout` 이 이미 TanStack Query 로 방송되므로 2중 방송,
  `architecture.md` §6.4 소유권 표 부담, 보조창 발행 격리 가드 필요 → 기각.
- `explorer-container.tsx` 인라인 이펙트: 컨테이너가 이미 포화(244줄·mutation 10)이고 두 차례 훅
  추출 이력에 역행 → 기각.
- 3값 설정(`true`/`focusNoScroll`/`false`): 이 코드베이스는 포커스를 주지 않으므로 boolean 으로 충분.

## 3. 구현 기록 (wf `wf_b172fc7a`, opus·xhigh, 에이전트 7/에러 0, 2026-09-04)
> 검증(통합 에이전트 실측): `bun run verify` 전 단계 exit 0 — typecheck 0 · lint 0 error/9 warning(기존) · format 통과 · bun test 1848/0 · cargo fmt/clippy 클린 · cargo test 1264/0 · `bunx vite build` exit 0 · `typecheck:e2e` 통과. 메인 2차 검증은 §4 에 기록.
### 3.1 에이전트별 요약
- **Rust C 설정+로케일**: 계약 C.2 항목 5(설정 explorerAutoReveal) + 이번 배치 전체 로케일 키 5종 + bindings 재생성을 완료했습니다.
    1) Settings 필드 — d-53(f25783b) 패턴 그대로: `types.rs` Settings 에 `explorer_auto_reveal: bool`(`#[serde(default = "default_true")]`, `enable_preview_tabs` 바로 뒤) + VS Code 파리티 근거 doc comment(`explorer.autoReveal` 기본 true, VS Code 의 3값 중 `"focusNoScroll"` 은 이 앱의 reveal 이 `file-tree.tsx` `selectByIndex` 로 DOM focus 를 건드리지 않아 구분할 것이 없으므로 boolean — 실물 확인 완료) · `SettingsPatch` 에 `Option<bool>` · `impl Default` 에 `default_true()` · `settings/service.rs apply_patch` · `sync/service.rs settings_to_sync_patch`.
  2) Rust 테스트 3건 추가 — 기본값 켜짐 / patch 로 끄고 생략 시 보존(settings/service.rs) / sync patch 포함(sync/service.rs). 기존 파리티 테스트 `settings_필드_집합은_rust와_bindings_ts에서_일치한다` 는 bindings 재생성 후 통과.
  3) `settings.ipc.ts emptySettingsPatch()` 에 `explorerAutoReveal: null` 한 줄 추가.
  4) 로케일 3언어(ko/en/ja)에 5개 키를 알파벳 정렬 위치(파일 전체가 바이트 정렬·4스페이스 flat JSON)에 추가. **최종 키 이름은 전부 지시대로**: `error.file.notFound`, `palette.filesRefreshing`, `common.openExternalLinkFailed`, `settings.explorerAutoReveal`, `settings.explorerAutoRevealDescription`. `common.` 네임스페이스는 이미 존재하므로 대체 없음. `Description` 접미사는 d-53 선례(`settings.editorBracketPairGuidesDescription` 등 25개)와 일치 — 같은 파일의 `enablePreviewTabsHint` 는 `Hint` 지만 SwitchField 설명문은 `Description` 이 지배 관례입니다.
  5) `locale/service.rs` 의 `MESSAGE_NAMESPACES` 에 5키 등재(필수 — `en_메시지의_모든_키는_required_message_keys에_포함된다` 테스트가 en.json 전 키의 등재를 강제). `error` 네임스페이스는 알파벳 정렬이라 `file.notAFile` 뒤에, `common`/`palette`/`settings` 는 삽입 순서라 관련 키 옆(a3cecb3 선례)에 배치.
  6) `cargo test` 로 `src/shared/api/bindings.ts` 재생성 — `Settings.explorerAutoReveal?: boolean`(doc comment 동반), `SettingsPatch.explorerAutoReveal: boolean | null` 확인. 손으로 고치지 않았습니다.
    다음 에이전트 참고: 설정 UI(`settings-interface-section.tsx` SwitchField)는 제 소
- **TS A useOpenFileTab**: 계약 A.2 의 4·5·6·7 프론트 부분을 구현했습니다.
    1) 단일 진입점 — `entities/layout/layout.query.ts` 에 `useOpenFileTab` 신설(`useOpenTabInProject` 바로 아래, `useOpenAppFileTab` 과 같은 묶음). `{projectId, path, preview, target, title?}` 를 받아 `kind:{kind:'file',path}` + `title ?? fileNameOf(path)` 로 열고, 실패 시 `isNotFoundIpcError` 이면 `SEARCH.PROJECT_FILES(projectId)` 를 무효화한 뒤 `toast.error(describeIpcError(error))` 를 띄웁니다. 설계 근거는 영어 JSDoc 으로 남겼습니다.
  2) `isNotFoundIpcError` 는 `shared/lib/ipc-error-message.ts` 의 `isIpcErrorKey` 옆에 두었습니다. `unwrap-result.ts` 의 `normalizeAppError` 가 `Localized` 의 `kind` 를 `code` 로 평탄화하므로 맨 `NotFound` 와 `Localized{kind:NotFound}` 둘 다 잡힙니다 — 테스트 2건 추가.
  3) 호출부 치환 — grep 으로 전수 확인 후 explorer 4곳(row 열기·검색매치·Open to the Side·Open With 2)·palette 2곳·search-editor·git-panel·problems-panel·editor-area·breadcrumbs-bar·welcome·agent-external-open-provider 를 전부 치환했습니다. preview/target/title 의미는 보존(explorer 는 `row.name` 을 title 옵션으로 전달). 중복 `toast.error(describeIpcError())` 는 전부 제거하고, 후처리(pane split·waitMarker 등록/해제)는 callbacks 로 유지했습니다. 남은 `useOpenTab`/`useOpenTabInProject` 사용처는 terminal·diff·settings·appFile·searchEditor·claudeDiff 로, 파일 탭이 아닙니다(단 예외 1건은 deviations 참고).
  4) 인덱스 무효화 — `file.query.ts` 에 `invalidateProjectFileIndex(queryClient, projectId)`(refetchType:'all') 헬퍼를 두고 `useCreateEntry`·`useRenameEntry`·`useCopyEntry`·`useDeleteEntry` 의 `onSuccess` 에 걸었습니다. `useCreateEntry(projectId)` 시그니처 변경에 따라 `explorer-container.tsx` 호출부를 갱신했습니다(`use-explorer-entry-crud.ts` 는 `ReturnType<typeof useCreateEntry>['mutateAsync']` 로 받아 무수정).
  5) 갱신 표시 — `command-palette.tsx` 가 projectFiles 쿼리의 `isFetching` 을 읽어 `CommandPaletteFilesGroup` 에 `isRefreshing` 을 넘기고, features 컴포넌트는 props 만 받아 그룹 헤딩 옆에 기존 로딩 표시와 같은 스타일(`Loader2 size-3 animate-spin` + `t('palette.filesRefreshing')`)을 렌더합니다. 항목 클릭은
- **TS B 외부 URL**: 계약 B.2 항목 1~5(프론트)·8 을 구현했습니다.
    먼저 실물 확인: `node_modules/@xterm/xterm/typings/xterm.d.ts:1353` `ILinkHandler.activate(event, text, range)` + `allowNonHttpProtocols`, 옵션은 `xterm.d.ts:163`. 나아가 `xterm.js.map` 의 sourcesContent 를 열어 **`linkHandler` 소비처가 `OscLinkProvider.ts` 단 한 곳**임을 확인했습니다(WebLinksAddon·커스텀 파일 링크 provider 의 activate 는 영향 없음). `defaultActivate` 가 `confirm()` → `window.open()` 인 것도 원문으로 확증했습니다. `@tauri-apps/api/window` 의 `getCurrentWindow(): Window` / `Window.label` 확인, 프로젝트 안에서 bun:test 임포트가 안전함도 실측했습니다.
    신설: `shared/lib/remote/runtime-environment.ts`(`isRemoteMirrorLabel` 순수 판정 + `isRemoteMirrorRuntime`), `shared/lib/external-url-opener.ts`(`createExternalUrlOpener` + 이동해온 `openViaBrowserWindow`), `shared/lib/external-anchor.ts`(`shouldOpenAnchorExternally`), `entities/system/external-url.ts`(단일 진입점 `openExternalUrl`), `app/providers/external-link-provider.tsx`(document capture-phase click 위임). 테스트 3파일 22건 추가(데스크톱은 shell 만·browser 호출 0, 셸 실패 시 폴백 없음, 원격은 browser 만, 원격 null 이면 throw, `openViaBrowserWindow` 3건 이관, 앵커 판정 12건, 라벨 4건).
    배선: `terminal-session.tsx` `handleOpenLink` → `openExternalUrl` 로 교체(이제 `systemOpenExternalUrl` 의 소비처는 새 단일 진입점 하나뿐), `terminal-link-opener.ts`·테스트 삭제(다른 참조 0 — grep 확인). `terminal-view.tsx` Terminal 옵션에 `linkHandler { activate: 수식어 게이트 + onOpenLinkRef.current(text), allowNonHttpProtocols: false }` 추가. `app.tsx` 는 main/auxiliary 두 분기 트리로 갈라져 있어 **양쪽 모두**에 `ExternalLinkProvider` 를 마운트했습니다.
    문서: `docs/features/terminal.md` §6.1 을 전면 교체(“window.open 선시도” 폐기 사유·환경별 경로 분리 표·링크 3종 게이트 표·OSC 8 linkHandler 근거·앵커 위임·Rust 가드는 architecture.md 로 포인터만), `docs/bug/2026-09-04-external-link-opens-in-app-window.md` 신설(두 경로 분리 서술 + “데스크톱 window.open non-null” 가설이 wry 소스로 반증됐고 실측 미확증임을 명기).
- **Rust A 열기 선검증**: 계약 A.2 항목 1·2·3 + ipc-contract 문서를 구현했다.
    1. layout_open_tab 파일 선검증 (src-tauri/src/domain/layout/commands.rs): 커맨드 층에 private `ensure_file_tab_target_exists(&HashMap<ProjectId, Project>, &TabKind)` 를 신설하고 `layout_open_tab` 진입부에서 호출한다. `TabKind::File { path }` 일 때만 `root_guard::resolve_owning_project` 로 경계 검사(실패 시 root_guard 의 Forbidden 을 그대로 전파) 후 resolved 경로의 `std::fs::metadata` 가 `is_file` 이 아니면 탭을 만들지 않고 `AppError::localized(NotFound, "error.file.notFound", "file not found: {path}").with_arg("path", ...)` 를 반환한다. `service::open_tab` / `open_tab_and_finish` 는 손대지 않았다. `state.projects.read().clone()` 후 호출하는 형태로 file_open 커맨드의 기존 패턴을 그대로 따랐다.
     - 사전 grep 결과: Rust `open_tab(` 호출부는 전부 layout/service.rs 내부(1285 프로덕션 1곳 + 테스트)이고, TS 의 파일 탭 열기는 이미 다른 에이전트가 `useOpenFileTab`(entities/layout/layout.query.ts) 단일 진입점으로 모아둔 상태였다. "존재하지 않는 경로로 파일 탭을 여는" 흐름은 use-explorer-entry-crud.ts 의 생성 직후 열기 1건뿐인데 `await createEntry(...)` 가 먼저라 정상이다 — 계약을 깨는 흐름 없음.
     - 원격: remote/dispatch.rs:838 이 같은 `layout::layout_open_tab` 커맨드 함수를 호출하므로 원격 세션에도 자동 적용된다. 반대로 IDE 도구 핸들러(ide/server.rs:270 `tool_open_file`)는 `layout_service::open_tab_and_finish` 를 직접 부르므로 이 게이트를 타지 않는다 — 다만 그 경로는 이미 자체적으로 `ide::service::ensure_path_within_any_project` 로 경계 검사를 하고 있어 계약 의도(커맨드 층 단일 지점)와 일치한다. 문서에 명시했다. **(§3.5 에서 정정 — 경계 검사는 존재 검사가 아니어서 IDE 경로는 여전히 빈 탭을 열 수 있었다. 존재 검사는 infra 로 옮겨 두 진입점이 공유한다.)**
  2. open_file NotFound 로케일화 (src-tauri/src/domain/file/service.rs): `std::fs::metadata(path)?` 를 `map_err(open_metadata_error)` 로 바꿔 `io::ErrorKind::NotFound` 만 같은 `error.file.notFound` localized 에러(arg `path`)로 매핑하고, 나머지 io 에러는 기존 `From<io::Error>` 변환을 유지한다.
  3. search_list_files 비-UTF8 제외 (src-tauri/src/domain/search/commands.rs): `to_string_lossy` 대신 private `utf8_paths(Vec<PathBuf>) -> Vec<String>` 가 `into_os_string().into_string().ok()` 로 필터링한다(`to_str()==None` 과 동일 의미, 유효 경로에서 재할당 없음).
  4. 테스트: lay
- **TS C autoReveal**: 계약 C.2 항목 1·2·3·4·6·7·8 을 구현했습니다.
    1) `shared/lib/pane-tree.ts` 에 `activeFilePathOf(tree)` 추가 — 인자 타입을 `WindowPaneTree | null | undefined` 로 잡아 주 트리(ProjectLayout 구조 호환)와 `resolveWindowPaneTree` 결과를 각 소비처가 그대로 넘길 수 있게 했습니다. 테스트 4건(파일 탭/비파일 탭/트리 없음/pane 없음) 추가.
  2) `shared/lib/explorer-auto-reveal.ts` 신설 — 계약 그대로의 `decideAutoReveal`(게이트 3축 → 중복 억제 → 보임 여부). 테스트 9건(비활성·경로없음·사이드바 접힘·뷰 비활성·중복·이미 보임→select-only·미보임→reveal-then-select·게이트 우선·다른 파일 재reveal).
  3) `widgets/explorer/use-explorer-auto-reveal.ts` 신설 — `layoutQueryOptions(projectId)`·`settingsQueryOptions()` 를 훅 내부에서 읽고, 활성 경로는 주 트리 고정(`activeFilePathOf(layout)`), `sidebarVisible = !zen && !sidebarCollapsed`(실물 필드 `layout.shellView.{zen,sidebarCollapsed}` 확인), `visiblePaths` 는 rows 에서, 프로젝트 루트 밖 경로는 `toRelativePath` 재사용 선판정으로 skip. select-only 는 `setSelectPathRequest`, reveal-then-select 는 `revealTreeNode` 후 select(실패는 조용히 무시). 중복 억제는 `lastRevealedRef`.
  4) `explorer-panel.tsx` view 를 controlled(`view`/`onViewChange`, `ExplorerView` export)로 바꾸고 `explorer-container.tsx` 가 소유 — 내부 `setView` 4곳(검색 구독·뷰 구독·reveal 구독·탭 클릭) 전부 `onViewChange` 로 치환, 컨테이너에서 `useExplorerAutoReveal` 호출(`explorerViewActive: view === 'files'`).
  5) `app-shell.tsx` 에 `subscribeRevealInExplorer(() => explorerPanelRef.current?.expand())` 이펙트를 기존 두 구독 옆에 추가(명시적 reveal 만 사이드바 펼침).
  6) `settings-interface-section.tsx` 에 `explorerAutoReveal` SwitchField 추가(enablePreviewTabs 옆, 기본 true, 로케일 키 실물 확인).
  7) 활성 파일 경로 관용구는 실제로 2곳(outline-panel-container·command-palette)만 존재해 그 2곳을 치환했습니다(나머지 3곳은 경로가 아니라 tab.title/tab.id 소비 — deviations 참조).
  8) 문서: `explorer-sidebar.md` §2.2 에 autoReveal 절 신설 + "펼침 상태 layout 영속화(재시작 복원)" 문구를 코드 실태(`TreeStore` 인메모리, `expanded_paths`/`restore_expanded` 호출부 테스트뿐)로 정정, `settings-ui.md` §1.1 에 설정 1종 기재.
- **Rust B WebView 가드**: 계약 B.2 항목 5(Rust 부분)·6·8을 구현했습니다.
    1. API 실사(cargo registry): tauri-2.11.5 `WebviewWindowBuilder::{from_config, on_navigation, on_new_window}`(전부 제네릭 impl, cfg 게이트 없음), `tauri::webview::{NewWindowResponse, NewWindowFeatures}`, tauri-utils 2.9.3 `WindowConfig.create`(기본 true, Clone) + `tauri::app::setup` 의 `filter(|w| w.create)`, wry 0.55.1 `wkwebview/navigation.rs` `navigation_policy`(URL 문자열만 전달 — 프레임 무구분 확증), tauri-plugin-opener 2.5.4 `Builder::new().open_js_links_on_click(bool).build()`, tauri-build 의 `cfg_alias("dev", ...)`(check-cfg 선언됨).
  2. `infra/navigation_guard.rs` 신설 — `is_navigation_allowed(&Url, Option<&Url>)`(스킴 tauri/asset/about/blob, http(s)의 tauri.localhost/asset.localhost/ipc.localhost, dev_url 과 `Url::origin()` 동일), `open_new_window_externally::<R>(Url) -> NewWindowResponse<R>`(검증 통과 시 `tauri_plugin_opener::open_url`, 항상 Deny), 두 빌더가 공유하는 `apply_navigation_guard(builder, &Config)`. `url` 크레이트는 추가하지 않고 `tauri::Url` 재수출을 씁니다. 단위 테스트 5건(허용 6종+dev 동일 오리진 / 거부 https·포트 불일치·javascript:·file:·data:·비-http 스킴의 허용 호스트).
  3. `tauri.conf.json` `app.windows[0]` 에 `"create": false`, `lib.rs` `.setup` 의 첫 문장에서 `create_main_window()` 가 `from_config` 로 동일 config 를 재생 + 가드 부착 후 `build()?`. 실패는 setup 오류로 전파합니다.
  4. 순서 영향 판단: Tauri 는 config 창을 setup 훅 **직전**에 만들므로 "setup 첫 문장" 은 사실상 같은 지점입니다. `MAIN_WINDOW_LABEL` 참조는 `lib.rs:410`(single_instance 콜백 — 런타임)과 `window/commands.rs:269`(런타임) 둘뿐이라 setup 내부 참조가 없고, `restore_auxiliary_windows`·`app.set_menu` 등 setup 본문은 전부 창 생성 이후입니다. window-state 플러그인은 `on_window_ready` 훅이라 수동 생성 창에도 그대로 적용되고, single_instance 콜백은 런타임이라 무관, 부팅 워처 후절(2026-08-20)·blank-window(프론트) 계약에도 영향 없습니다. capability `main.json` 은 라벨 매칭이라 무변경입니다.
  5. `domain/window/commands.rs` 보조 창 빌더에도 같은 헬퍼를 부착했습니다(정책 공유, 중복 0).
  6. `tauri_plugin_opener::init()` → `

### 3.2 계약 이탈(deviations) — 전건 사유 포함
- [Rust C 설정+로케일] 계약/지시에 명시되지 않았던 필수 작업 1건: `src-tauri/src/domain/locale/service.rs` 의 `MESSAGE_NAMESPACES` 상수에 신규 5키를 등재했습니다. 지시가 "키 등록/backfill 목록이 있을 때만" 이라 했고, 실제로 `required_message_keys()` + `en_메시지의_모든_키는_required_message_keys에_포함된다` 테스트가 en.json 의 모든 키를 이 상수에 요구하므로 등재 없이는 cargo test 가 실패합니다.
- [Rust C 설정+로케일] 소유 밖 TS 타입 오류 3건(수정하지 않음, 다른 에이전트의 진행 중 작업): src/widgets/git-panel/git-panel-container.tsx(196,11) TS6133 'openGitFileTab' is declared but its value is never read / (239,13) TS2322 `useOpenFileTab` 반환 시그니처(OpenFileTabRequest 객체)를 `(path: string) => void` 자리에 넘김. 계약 A.2-4 의 useOpenFileTab 치환이 이 파일에서 미완인 상태로 보입니다.
- [Rust C 설정+로케일] 첫 `cargo test --workspace` 실행은 `domain::settings::types::tests::settings_필드_집합은_rust와_bindings_ts에서_일치한다` 에서 실패했습니다. 같은 실행 안에서 `tests::typescript_바인딩을_생성한다` 가 bindings.ts 를 재생성하는데 파리티 테스트가 재생성 이전의 파일을 읽기 때문입니다(테스트 순서 의존). 재실행 시 전부 통과 — 기존 구조상 정상 동작이라 수정하지 않았습니다.
- [Rust C 설정+로케일] 워킹트리에 `D  src/widgets/terminal-pane/terminal-link-opener.ts` 가 이미 스테이징되어 있습니다(다른 에이전트가 `git rm` 한 것으로 보임). 제가 스테이징한 것은 없습니다.
- [TS A useOpenFileTab] callbacks.onSuccess 를 계약의 `() => void` 대신 `(layout: ProjectLayout) => void` 로 넓혔습니다. explorer 의 'Open to the Side' 가 열린 뒤의 layout 으로 focusedPane 을 찾아 split 해야 해서(기존 openTabAsync 반환값 사용) 필요했습니다. `() => void` 구현은 그대로 대입 가능하므로 계약이 상정한 호출부에는 영향이 없습니다.
- [TS A useOpenFileTab] `useSaveFile` 은 인덱스 무효화 대상에 넣지 않았습니다(계약의 '같은 파일의 다른 fs 뮤테이션 전부'와 다름). 저장은 이미 인덱스에 있는 경로를 덮어쓰고 자동 저장이 빈도를 무한정 늘려 저장 1회 = 프로젝트 전체 walk 1회가 되며, 새 파일이 생기는 유일한 저장 경로(무제목 탭 Save As)는 워처의 `created` 에코가 이미 무효화합니다. 사유는 `invalidateProjectFileIndex` JSDoc 과 command-palette.md §3.1 에 기록했습니다.
- [TS A useOpenFileTab] 계약이 열거한 10곳 외에 파일 탭 호출부가 1곳 더 있습니다 — `src/widgets/app-shell/app-shell.tsx:65` (드래그앤드롭 파일 열기, `useOpenTabInProject().mutateAsync`). 소유 파일 밖이라 손대지 않았습니다. 이 경로만 NotFound 인덱스 무효화·단일 토스트 계약에서 빠집니다.
- [TS A useOpenFileTab] `agent-external-open-provider.tsx` 의 부팅 drain 이펙트 의존성을 `[queryClient, openTabInProject]` → `[]` + `useEffectEvent` 로 바꿨습니다. `useOpenFileTab` 은 매 렌더 새 클로저를 반환하므로 의존성에 두면 렌더마다 대기 큐를 재처리합니다. `useTauriEvent` 경로는 훅이 이미 useEffectEvent 로 감싸므로 그대로 둡니다.
- [TS A useOpenFileTab] `explorer-container.tsx` 의 로컬 row 어댑터 `openFileTab(row, preview)` 를 `openRowFileTab` 으로 개명했습니다(훅 바인딩 이름과 충돌). `use-explorer-entry-crud.ts` 의 입력 필드명 `openFileTab` 은 그대로 두고 `openFileTab: openRowFileTab` 로 넘겨 그 파일은 무수정입니다.
- [TS A useOpenFileTab] `git-panel-container.tsx` 의 1회용 로컬 `openFileTab` 헬퍼는 유일한 JSX 사용처에 inline 했습니다(한 줄 — frontend.md JSX inline 규칙).
- [TS A useOpenFileTab] `src/widgets/explorer/use-explorer-clipboard.ts` 는 `kind:'file'` 탭을 여는 코드가 없어 변경하지 않았습니다.
- [TS B 외부 URL] app.tsx 마운트가 '1줄'이 아니라 2곳입니다. app.tsx 는 `getWindowContext()` 로 main/auxiliary 두 프로바이더 트리로 완전히 갈라져 있어(보조창 분기는 AgentExternalOpenProvider·IdeSyncProvider·CommandPalette 를 건너뜀), 계약의 '메인·보조 창 모두 같은 React 앱' 을 만족하려면 양쪽 트리에 각각 넣어야 합니다. AppProviders 바로 안쪽(루트)에 두었고, 그 결과 두 JSX 트리의 들여쓰기가 한 단계씩 밀렸습니다(동작 변화 없음). 파일 상단 JSDoc 의 '두 분기 공통 프로바이더' 목록에도 ExternalLinkProvider 를 추가했습니다.
- [TS B 외부 URL] openViaBrowserWindow 의 JSDoc 을 그대로 옮기지 않고 마지막 문장만 고쳤습니다. 원문 말미가 'so callers can fall back to an IPC-based opener' 인데 그 폴백은 이번 설계에서 폐기됐으므로, 사실과 어긋나는 문장을 남기지 않고 '원격 미러만 이 함수를 부른다'로 바꿨습니다(noopener 함정 설명 본문은 그대로 유지).
- [TS B 외부 URL] openViaBrowserWindow 의 catch 블록에 있던 설명 코드 주석(2026-08-18 아키텍처 감사가 comments.md 위반으로 이미 지목한 항목)을 제거하고, catch 가 `return false` 를 갖는 이름있는 헬퍼 `revokeOpener` 로 분리했습니다. 주석을 지우면 빈 catch 가 되어 eslint `no-empty` 에 걸리는데, 코드 주석은 금지고 `eslint-disable` 도 금지라 구조로 해결했습니다. 코드베이스 전체에 빈 catch 선례가 0 인 것도 확인했습니다.
- [TS B 외부 URL] terminal-view.test.ts 는 수정하지 않았습니다. linkHandler.activate 가 기존 shouldActivateTerminalLink 를 그대로 재사용해 새로운 순수 판정 로직이 생기지 않았고, 그 함수는 이미 6건(mac meta/alt/둘다/없음, 비-mac ctrl/meta)으로 전량 커버돼 있습니다. 계약의 '없으면 게이트 함수 재사용 사실만 summary' 조항을 따랐습니다.
- [TS B 외부 URL] shouldOpenAnchorExternally 는 `new URL(href, appOrigin)` 이 아니라 인자 1개 `new URL(href)` 로 파싱합니다. 계약의 '파싱 실패 false' 문구와 맞고, 상대 경로는 절대 URL 이 아니므로 앱 내부 처리(false)가 되는 것이 의도한 동작입니다. 호출부는 항상 `anchor.href`(DOM 이 절대화한 값)를 넘깁니다.
- [TS B 외부 URL] docs/features/terminal.md §6(파일 링크) 첫 불릿의 'URL 은 web-links addon' 문구는 손대지 않았습니다(minimal diff). OSC 8 이 별도 경로라는 사실은 바로 아래 §6.1 표에서 정정됩니다.
- [Rust A 열기 선검증] 비-UTF8 테스트를 파일시스템 픽스처가 아니라 순수 필터 단위 테스트로 작성했다. 계약은 'unix 에서 OsStrExt::from_bytes 로 비-UTF8 이름 파일을 만든 테스트' 를 요구했으나, 개발 플랫폼(macOS/APFS)이 UTF-8 파일명을 강제해 `std::fs::write` 가 EILSEQ(Illegal byte sequence)로 실패한다 — 실제로 픽스처 버전을 먼저 작성해 cargo test 에서 이 실패를 확인한 뒤 교체했다. 현재 테스트는 `OsStr::from_bytes(b"broken-\xff.rs")` 로 walker 가 그런 파일시스템에서 돌려주는 것과 동일한 PathBuf 를 합성해 `utf8_paths` 가 그것을 버리는지 단언하고, 그 사유를 테스트 doc 주석(영어 JSDoc 관례)에 남겼다.
- [Rust A 열기 선검증] 테스트 위치: 계약/역할표는 비-UTF8 테스트를 search/service.rs 에 두라고 했으나, 필터가 commands.rs 의 `utf8_paths` 에 있으므로 같은 파일(소유 파일)의 tests 모듈에 넣었다. search/service.rs 는 수정하지 않았다.
- [Rust A 열기 선검증] root_guard.rs 는 수정하지 않았다 — `resolve_owning_project` 가 이미 pub 이라 pub 노출 변경이 불필요했다.
- [Rust A 열기 선검증] src/shared/api/bindings.ts 가 12줄 변경된 상태로 워킹트리에 남아 있다. 이는 내 변경이 아니라 다른 에이전트의 Settings(`explorerAutoReveal`) Rust 타입 추가가 내 `cargo test` 실행으로 재생성된 결과다(specta 자동 생성 — 손대지 않았다).
- [TS C autoReveal] 계약이 말한 '활성 파일 경로 관용구 5곳' 중 실제로 경로를 유도하는 곳은 2곳뿐이었습니다. `title-bar-content.tsx:15`·`auxiliary-title-bar-content.tsx:27` 은 `activeTab?.title`, `status-bar-content.tsx:56` 은 `findActiveTab(...)?.id` 를 쓰므로 `activeFilePathOf` 로 치환하면 동작이 바뀝니다(파일 경로가 아니라 탭 제목/ID 소비). 그 3곳은 `findActiveTab` 그대로 두었습니다. 치환은 outline-panel-container·command-palette 2곳.
- [TS C autoReveal] 계약 C.2-3 은 '훅에서 decideAutoReveal 호출 → useEffect 에서 실행' 이지만, 렌더 중 `lastRevealedRef.current` 를 읽으면 eslint `react-hooks/refs` 가 error 를 냅니다(실측: 'Cannot access refs during render'). 그래서 결정 계산을 `useEffectEvent`(app-shell 이 이미 쓰는 React 19 API) 안으로 옮겨, 렌더는 순수하게 두고 이펙트 의존성은 전부 실제 사용되는 값(enabled·revealablePath·rows·sidebarVisible·explorerViewActive)만 두었습니다. 판정 함수 시그니처·규칙은 계약 그대로입니다. 부수 효과로 같은 결정이 두 번 전달돼도(StrictMode 이중 마운트) 경로를 선점(claim)해 `tree_reveal` 이 중복 발사되지 않습니다.
- [TS C autoReveal] `decideAutoReveal` 의 입력 타입은 별도 export 하지 않고 훅에서 `Omit<Parameters<typeof decideAutoReveal>[0], 'lastRevealedPath'>` 로 유도했습니다(타입 원본 유도 규칙).
- [TS C autoReveal] `explorer-sidebar.md` §2.2 펼침 영속화 문구 정정은 계약대로 '문서만' 수정했고, 재시작 복원 구현은 하지 않았습니다(호출부 0 임을 `TreeStore`·`domain/layout` grep 으로 재확인).
- [Rust B WebView 가드] `validate_external_url` 을 `pub(crate)` 로만 노출하지 않고 `src-tauri/src/infra/external_url.rs`(신설)로 이동했습니다. 이유: `tests/domain_boundaries.rs` 의 `infra는_화이트리스트_밖의_domain_참조를_가질_수_없다` 가 infra→domain 참조를 기계 강제로 거부합니다(타입 참조 4건만 사유와 함께 화이트리스트). 계약이 지정한 `infra/navigation_guard.rs` 위치에서 그 함수를 부르면 실행 경로 역참조가 되어 즉시 FAIL 했고(실제로 1차 구현에서 FAIL 재현), 화이트리스트에 등재하는 것은 소유 밖 파일 수정 + 아키텍처 불변식 약화라 채택하지 않았습니다. 대신 검증기(순수 문자열 정책)를 infra 로 내려 domain→infra 정상 방향으로 뒤집었습니다 — 2026-08-18 아키텍처 감사가 이미 '`validate_external_url`(+테스트)은 commands.rs 밖으로' 를 권고한 방향과도 일치합니다. 함수 본문·상수·테스트 9건은 그대로 옮겼고(로직 변경 0), `domain/system/commands.rs` 는 `use crate::infra::external_url::validate_external_url` 로 호출하며 불필요해진 `AppErrorKind` import 만 제거했습니다.
- [Rust B WebView 가드] `dev_url` 을 무조건 신뢰하지 않고 `cfg!(dev)` 일 때만 허용 목록에 넣었습니다(`dev_server_url`). `build.devUrl` 은 릴리스 바이너리의 임베드 config 에도 남지만 Tauri 자신은 `cfg(dev)` 에서만 그리로 이동하므로(`manager::AppManager::get_app_url`), 릴리스에서 localhost:5173 을 신뢰하면 허용 범위만 넓어집니다. 계약보다 엄격한 방향이라 기능 영향은 없습니다.
- [Rust B WebView 가드] `lib.rs` 에서 `app.config().app.windows[0]` 인덱싱 대신 `label == constants::MAIN_WINDOW_LABEL` 로 찾습니다(없으면 setup 오류). 배열 순서에 의존하지 않기 위함이며 현재 config 에서는 동일한 창을 가리킵니다.
- [Rust B WebView 가드] `docs/ipc-contract.md` 는 손대지 않았습니다(계약 §B.8 '변경 없음' + 다른 에이전트가 §A 로 편집 중). 다만 그 문서 2026-08-18 절의 `system_open_external_url` 항목 '터미널 ... 이 유일한 호출부다' 는 이번 배치(앵커 프로바이더 신설 + navigation_guard 의 open_url 사용)로 사실과 달라졌습니다 — 아래 openQuestions 참조.
- [통합] [통합] 계약 A.2-4 의 '파일 탭 여는 호출부 10곳 전부 치환' 은 실제로 11곳이었고, 11번째인 `src/widgets/app-shell/app-shell.tsx` 의 드래그앤드롭 열기는 의도적으로 치환하지 않았습니다. 그 루프는 `for … await` 로 여러 파일을 순차로 열어야 하는데(동시 발사하면 각 응답의 `applyFreshLayout` 이 완료 순서대로 캐시를 덮어써 마지막 도착한 낡은 레이아웃이 이깁니다), `useOpenFileTab` 은 `mutate` 기반이라 await 할 대상이 없습니다. OS 파일 매니저가 주는 경로는 스테일 인덱스 출처가 아니라 NotFound 무효화를 놓쳐도 잃는 것이 없고, 토스트는 그 호출부가 이미 띄웁니다. 사유는 `command-palette.md` §3.1 과 `useOpenFileTab` JSDoc 에 기록했습니다.
- [통합] [통합] 계약이 테스트 wf 몫으로 배정한 e2e 스펙 13 주석 갱신을 통합에서 처리했습니다(이번 실행에 테스트 wf 가 없음). 'This spec never expands src/' 을 '스펙 자신은 확장하지 않음 + autoReveal 이 탭 개봉 후 tree_reveal 로 펼치므로 팔레트 결과의 근거가 될 수 없음' 으로 정확화했습니다.
- [통합] [통합] 계약 B.2-8 은 `docs/ipc-contract.md` 무변경이었으나, 그 문서 2026-08-18 절의 '터미널 WebLinksAddon 이 `system_open_external_url` 의 유일한 호출부다' 가 이번 배치로 사실과 달라져 해당 불릿에 '정정(2026-09-04)' 문단을 덧붙였습니다. 커맨드 계약 자체는 불변임을 명시했습니다.
- [통합] [rustC] 계약에 없던 필수 작업 1건 — `locale/service.rs` 의 `MESSAGE_NAMESPACES` 에 신규 5키 등재. `en_메시지의_모든_키는_required_message_keys에_포함된다` 테스트가 en.json 전 키의 등재를 강제해 없으면 cargo test 가 실패합니다.
- [통합] [rustB] `validate_external_url` 을 계약대로 `domain::system::commands` 에 `pub(crate)` 노출하는 대신 `src-tauri/src/infra/external_url.rs` 로 이동했습니다. `tests/domain_boundaries.rs` 가 infra→domain 실행 경로 참조를 기계 강제로 거부하므로, `infra/navigation_guard.rs` 에서 그 함수를 부르면 즉시 FAIL 합니다. 화이트리스트 등재(아키텍처 불변식 약화) 대신 검증기를 infra 로 내려 domain→infra 정상 방향으로 뒤집었고, 로직 변경 0 입니다.
- [통합] [rustB] `dev_url` 을 `cfg!(dev)` 일 때만 허용 목록에 넣었습니다(계약보다 엄격). 릴리스 바이너리에도 `build.devUrl` 이 임베드되지만 Tauri 자신은 `cfg(dev)` 에서만 그리로 이동하므로, 릴리스에서 localhost:5173 을 신뢰하면 허용 범위만 넓어집니다.
- [통합] [rustB] `lib.rs` 가 `app.config().app.windows[0]` 인덱싱 대신 `label == MAIN_WINDOW_LABEL` 로 창 config 를 찾습니다(배열 순서 비의존, 현재 config 에서는 동일 창).
- [통합] [rustB] `app.tsx` 의 `ExternalLinkProvider` 마운트가 계약의 '루트 1줄' 이 아니라 2곳입니다. app.tsx 가 `getWindowContext()` 로 main/auxiliary 두 프로바이더 트리로 갈라져 있어 양쪽에 각각 넣어야 '메인·보조 창 모두' 가 성립합니다.
- [통합] [rustB] `openViaBrowserWindow` 이관 시 catch 블록의 설명 코드 주석(comments.md 위반, 2026-08-18 감사 지적분)을 제거하고 `revokeOpener` 이름있는 헬퍼로 분리했습니다. 주석 제거만 하면 빈 catch 가 되어 eslint `no-empty` 에 걸리는데 `eslint-disable` 도 금지라 구조로 해결했습니다.
- [통합] [rustA] 비-UTF8 테스트를 파일시스템 픽스처가 아니라 `utf8_paths` 순수 필터 단위 테스트로 작성했습니다. macOS/APFS 가 UTF-8 파일명을 강제해 `std::fs::write` 가 EILSEQ 로 실패하는 것을 실제로 재현한 뒤 교체했습니다. 위치도 계약이 지목한 search/service.rs 가 아니라 필터가 실제로 있는 search/commands.rs 입니다.
- [통합] [tsA] `useOpenFileTab` 의 `callbacks.onSuccess` 를 계약의 `() => void` 대신 `(layout: ProjectLayout) => void` 로 넓혔습니다. explorer 의 'Open to the Side' 가 열린 뒤 layout 으로 focusedPane 을 찾아 split 해야 하기 때문이며, `() => void` 구현은 그대로 대입 가능합니다.
- [통합] [tsA] `useSaveFile` 은 인덱스 무효화 대상에서 제외했습니다(계약의 '같은 파일의 다른 fs 뮤테이션 전부' 와 다름). 저장은 이미 인덱스에 있는 경로를 덮어쓰고 자동 저장이 빈도를 무한정 늘리며, 새 파일이 생기는 유일한 저장 경로(무제목 탭 Save As)는 워처 `created` 에코가 덮습니다. 사유는 JSDoc + command-palette.md §3.1 에 기록.
- [통합] [tsA] `agent-external-open-provider.tsx` 부팅 drain 이펙트를 `[queryClient, openTabInProject]` → `[]` + `useEffectEvent` 로 바꿨습니다. `useOpenFileTab` 은 매 렌더 새 클로저라 의존성에 두면 렌더마다 대기 큐를 재처리합니다.
- [통합] [tsC] 계약 C.2-1 의 '활성 파일 경로 관용구 5곳' 중 실제로 경로를 유도하는 곳은 2곳(outline-panel-container·command-palette)뿐이라 그 2곳만 치환했습니다. title-bar-content·auxiliary-title-bar-content 는 `activeTab?.title`, status-bar-content 는 `findActiveTab(...)?.id` 를 소비하므로 `activeFilePathOf` 로 바꾸면 동작이 바뀝니다(실물 확인 완료).
- [통합] [tsC] 계약 C.2-3 의 '훅에서 decideAutoReveal 호출 → useEffect 에서 실행' 대신 결정 계산을 `useEffectEvent` 안으로 옮겼습니다. 렌더 중 `lastRevealedRef.current` 를 읽으면 eslint `react-hooks/refs` 가 error 를 냅니다. 판정 함수 시그니처·규칙은 계약 그대로이고, 부수 효과로 StrictMode 이중 마운트에서도 `tree_reveal` 이 중복 발사되지 않습니다.
- [통합] [tsC] `explorer-sidebar.md` §2.2 의 '펼침 상태 layout 영속화(재시작 복원)' 문구는 계약대로 문서만 정정했고 구현은 하지 않았습니다(`expanded_paths`/`restore_expanded` 호출부 0 재확인).

### 3.3 미결(openQuestions) — 리뷰·사용자 판단 대상
- [TS A useOpenFileTab] `src/widgets/app-shell/app-shell.tsx:65` 의 드래그앤드롭 파일 열기도 `useOpenFileTab` 로 치환할지 — 소유 파일 밖이라 미처리. 치환하면 그 경로도 NotFound 인덱스 무효화를 받습니다.
- [TS A useOpenFileTab] `useSaveFile` 무효화 제외를 승인하는지(위 deviations 2). 승인 시 command-palette.md §3.1 문구가 그대로 정본이 됩니다.
- [TS A useOpenFileTab] `target:null` 이 가리키는 focusedPane 이 사라졌을 때의 `NotFound("pane not found")` 가 파일 부재와 같은 코드라, 파일은 멀쩡한데 인덱스 무효화(walk 1회)가 함께 일어납니다. 백로그에 등재만 했고 이번 배치에서 코드/로케일 키를 분리할지는 미결정입니다.
- [TS B 외부 URL] '데스크톱 webview 의 window.open() 이 정말 null 인가' 는 wry 0.55.1 소스로만 반증했고 실측하지 못했습니다. 새 설계는 데스크톱에서 window.open 을 아예 호출하지 않으므로 결론에는 영향이 없지만, bug 문서에 미확증으로 명기해 두었습니다. 실기 QA 항목(마크다운 프리뷰 외부 링크 좌클릭 / gh·vite·bun 의 OSC 8 출력 ⌥클릭 / 원격 미러 새 탭)은 bug 문서 '검증' 절에 남겼습니다 — 필요하면 docs/quality-assurance 체크리스트로 승격이 필요한지 결정 부탁드립니다.
- [TS B 외부 URL] 계약 B.2-5 의 `tauri_plugin_opener::Builder::new().open_js_links_on_click(false)` 전환과 B.2-6 Rust WebView 가드는 제 소유 밖(다른 에이전트)입니다. 그것이 들어오기 전까지는 죽은 인터셉터가 남아 있지만, 그 인터셉터는 target=_blank/Ctrl/Shift 클릭에서 preventDefault 만 하므로 새 capture-phase 핸들러(먼저 실행되며 defaultPrevented 를 확인)와 충돌하지 않습니다.
- [Rust A 열기 선검증] layout_open_tab 선검증이 켜지면서 agent-external-open-provider(Claude Code 의 외부 열기 요청)가 '아직 만들어지지 않은 파일' 경로를 넘길 경우, 예전에는 빈 에러 탭이 열리던 것이 이제 토스트만 뜨고 탭이 안 열린다. 계약이 명시한 의도된 동작이라 그대로 뒀으나, 에이전트가 파일 생성 직전에 탭을 먼저 여는 사용 패턴이 있다면 확인이 필요하다.
- [Rust A 열기 선검증] root_missing 프로젝트(루트가 사라진 프로젝트)의 파일 탭은 `resolve_owning_project` 의 root canonicalize 실패로 Forbidden(error.path.outsideOpenProjects)으로 거절된다. file_open 도 동일하게 실패하던 경로라 일관되지만, 사용자에게는 'NotFound' 대신 '열린 프로젝트 밖' 문구가 보인다.
- [TS C autoReveal] `findActiveTab` 을 쓰는 나머지 3곳(title-bar·status-bar·auxiliary-title-bar)은 탭 제목/ID 를 보므로 이번 관용구 통합 대상이 아니었습니다. 원하시면 `activeTabOf(tree)` 같은 별도 헬퍼로 추가 통합할 수 있으나, 계약 범위 밖이라 손대지 않았습니다.
- [TS C autoReveal] autoReveal 기본 ON 이라 앱 부팅 직후 활성 파일까지 트리가 펼쳐집니다(VS Code 파리티). e2e 스펙 13 의 'This spec never expands src/' 주석 갱신은 계약상 테스트 wf 몫이라 건드리지 않았습니다.
- [Rust B WebView 가드] `docs/ipc-contract.md:984-985` 의 '터미널(xterm WebLinksAddon)이 `system_open_external_url` 의 유일한 호출부다' 문구가 이번 배치로 낡았습니다(앵커 위임 프로바이더 추가 + Rust 새 창 승격 경로). 해당 파일은 제 소유가 아니어서 두었으니, 메인이 병합 시 한 줄 정정할지 결정해 주세요.
- [Rust B WebView 가드] `docs/research/2026-09-04-batch4-topics1-5-research.md:1650` 이 `validate_external_url` 을 `domain/system/commands.rs:221-296` 으로 인용합니다(이제 `infra/external_url.rs`). 리서치 기록물이라 정정하지 않았습니다.
- [Rust B WebView 가드] 실기 확증 미수행: OSC 8 링크·마크다운 앵커·`window.open()` 이 실제로 OS 브라우저로 열리는지, HTML 프리뷰 iframe(blob/about:srcdoc)이 가드 이후에도 정상 렌더되는지, dev 모드 부팅(HMR·리로드)과 보조 창 생성이 정상인지는 앱 실행이 필요합니다(이번 세션에서는 실행하지 않았습니다).
- [통합] `layout_open_tab` 선검증이 켜지면서, 아직 만들어지지 않은 경로를 넘기는 외부 열기 요청(Claude Code 의 `taide open`)은 이제 빈 에러 탭 대신 토스트만 뜨고 탭이 열리지 않습니다. 계약이 명시한 의도된 동작이라 그대로 뒀으나, 에이전트가 '파일 생성 직전에 탭 먼저 열기' 패턴을 쓴다면 확인이 필요합니다.
- [통합] 루트가 사라진 프로젝트(root_missing)의 파일 탭은 `resolve_owning_project` 의 canonicalize 실패로 Forbidden(`error.path.outsideOpenProjects`)으로 거절됩니다. `file_open` 도 동일하게 실패하던 경로라 일관되지만, 사용자에게는 'NotFound' 대신 '열린 프로젝트 밖' 문구가 보입니다.
- [통합] `app-shell.tsx` 드래그앤드롭 열기를 단일 진입점에 합치려면 `useOpenFileTab` 에 순차성을 보존하는 async 변형이 필요합니다. 지금은 예외로 두고 문서화만 했는데, 추가할지 결정 부탁드립니다.
- [통합] `useSaveFile` 의 인덱스 무효화 제외를 승인하시는지(승인 시 `command-palette.md` §3.1 문구가 그대로 정본이 됩니다).
- [통합] 실기 확증 미수행 항목: OSC 8 링크·마크다운 앵커·`window.open()` 이 실제로 OS 브라우저로 열리는지, HTML 프리뷰 iframe(blob/about:srcdoc)이 네비게이션 가드 이후에도 렌더되는지, dev 부팅(HMR·리로드)과 보조 창 생성이 정상인지, autoReveal 이 실제 트리에서 기대대로 동작하는지. `docs/quality-assurance` 체크리스트로 승격할지 결정 부탁드립니다.

### 3.4 리뷰 수정 (2026-09-04, 리뷰 wf 결과 반영)

리뷰 wf 가 올린 발견 중 확증 major 는 0건이었고, 강등 1건 + minor 3건을 다음과 같이 처리했다.

- **A-1 (minor, `entities/file/file.query.ts`) — 수정(문서·JSDoc 정정).** `invalidateProjectFileIndex`
  의 `refetchType: 'all'` 이 "팔레트가 닫혀 있어도 즉시 재-walk" 를 달성한다는 서술이 사실과 다르다.
  `node_modules/@tanstack/query-core@5.101.4` 실물 확인: `queryClient.refetchQueries` 는
  `findAll(filters).filter((q) => !q.isDisabled() && !q.isStatic())` 를 `type`(=`refetchType`) 값과
  **무관하게** 적용하고, `Query.isDisabled()` 는 옵저버가 1개 이상이면 `!isActive()` 다
  (`query.js:97-106`). `<CommandPalette/>` 는 `app.tsx` 에 항상 마운트되어 `enabled:false` 상태의
  옵저버를 1개 유지하므로, 팔레트가 닫혀 있으면 `'all'` 이든 기본값이든 refetch 대상에서 제외된다 —
  실제 재-walk 는 다음 ⌘P 오픈 시(invalidated → stale → mount fetch) 1회 돈다.
  - **코드 동작은 바꾸지 않았다.** 즉시 재-walk 를 진짜로 달성하려면 `app/bootstrap-snippets.ts` 처럼
    영구 `QueryObserver` 로 쿼리를 항상 active 로 유지해야 하는데, 그러면 워처 에코(300ms 디바운스)
    까지 매번 전체 walk 를 돌려 계약 A.2-5 가 명시적으로 피한 "walk 폭주" 가 된다. `refetchType:'all'`
    자체는 살아 있는 옵저버가 없는 인덱스(활성이 아니게 된 프로젝트)까지 범위를 넓히므로 그대로 둔다.
  - 정정 대상: `file.query.ts` JSDoc 2문단, `docs/features/command-palette.md` §3.1 표 1행 + 재-walk
    시점 불릿 신설, `docs/bug/2026-09-04-quick-open-stale-index-not-found.md` 해결 절.
  - 잔여 스테일 창(첫 프레임의 낡은 배열)은 이미 배치가 만든 두 겹 — `palette.filesRefreshing` 표시와
    `layout_open_tab` 열기 선검증 — 이 막는다. 계약 A 의 사용자 증상 해결에는 영향이 없다.
- **conv-02 (minor, `explorer-auto-reveal.ts` 배치) — 수정(파일 이동).** 계약 C.2-2 는 `shared/lib` 를
  지정했으나 소비처가 `widgets/explorer/use-explorer-auto-reveal.ts` 하나뿐이라 fsd.md 의 승격 기준
  (2곳 이상)과 어긋난다. `src/widgets/explorer/explorer-auto-reveal.ts`(+ `.test.ts`)로 옮겼다 —
  같은 슬라이스의 `paste-plan.ts`·`file-tree-git-status.ts` 와 같은 자리이고, 위젯 안에서도 React 무관
  순수 함수 + `bun:test` 성격은 그대로다. 함수·타입·테스트 내용은 무변경이며 import 경로와
  `docs/features/explorer-sidebar.md` 의 2개 언급만 갱신했다. **이 절이 계약 C.2-2 의 경로를 대체한다.**
- **C-1 (minor, `app-shell.tsx:142` Zen 가드) — 미수정(reject).** Zen 모드에서는
  `pane-node-view.tsx` 의 `{!zen && <PaneTabBar/>}` 때문에 탭 바 자체가 렌더되지 않고,
  `explorer-reveal-bridge` 의 유일한 발행처는 그 탭 우클릭 메뉴(`pane-tab-bar.tsx:205`)다. 브리지는
  창별 in-process 버스(`fire-and-forget-bridge.ts`)라 보조 창의 우클릭도 주창 구독에 닿지 않는다 —
  따라서 지적된 시나리오가 현재 도달 불가다. 함께 지적된 `subscribeOpenSearchPanel`·
  `subscribeShowExplorerView` 는 전역 키맵으로 Zen 중에도 도달하지만 이는 이번 배치 이전부터의
  동작이고, 셋 다에 `if (zen) return` 을 거는 것은 "명시적 요청이 Zen 을 뚫는가" 라는 UX 정책 결정
  이라 배치 3 범위 밖이다. 도달 불가 사실만 `docs/features/explorer-sidebar.md` 에 명기했다.
- **conv-01 (minor, `explorer-container.tsx` 훅 배치 순서) — 미수정(defer).** `useExplorerAutoReveal`
  은 형제 훅(`useExplorerEntryCrud`·`useExplorerClipboard`) 바로 옆에 놓여 기존 파일 패턴을 그대로
  따랐다. frontend.md §3.2 순서를 실제로 회복하려면 이번 배치가 만들지 않은 두 훅까지 포함해 컨테이너
  상단 블록 전체를 재배치해야 하므로(minimal diff 위반), 별도 정리 작업으로 넘긴다.
- 참고: 당시 반박했던 major(A-2, IDE `tool_open_file` 이 존재 검증 게이트를 우회한다는 지적)는
  **§3.5 에서 인정하고 수정했다** — 반박의 근거였던 `ensure_path_within_any_project` 는 경계 검사일
  뿐 존재 검사가 아니다. 함께 지적된 info 2건(`external-anchor.ts` JSDoc 의 mailto/file/javascript
  서술, `open_new_window_externally` 의 UI 스레드 실행)은 지시대로 손대지 않았다.

### 3.5 테스트 단계 보완 (2026-09-04) — A-2 기술 격차 봉합

리뷰 발견 A-2 를 다시 판정해 **수정으로 뒤집었다.** §3.4 의 반박은 `ide::service::ensure_path_within_any_project`
가 "자기 경로를 이미 검증한다"는 데 기댔으나, 그것은 `root_guard::resolve_owning_project` 를 그대로
호출하는 **경계 검사**일 뿐 존재 검사가 아니다. 따라서 Claude Code 가 없는 경로로 `openFile` 을 부르면
계약 A.2-1 의 게이트를 우회해 여전히 빈 파일 탭이 열렸다(증상은 A 의 원 보고와 동일 — 본문에 `os error 2`).

- **존재 검사를 infra 로 추출**: `infra/root_guard.rs` 에 `pub fn ensure_existing_file(resolved: &Path,
  display_path: &str) -> AppResult<()>` 신설. `layout::commands::ensure_file_tab_target_exists` 의
  `metadata().is_file()` + `AppError::localized(NotFound, "error.file.notFound").with_arg("path", ..)`
  본문을 그대로 옮긴 것이고, 커맨드 층 함수는 `resolve_owning_project` 후 이 함수를 부르는 2줄로 줄었다.
  **동작·에러·arg 는 무변경**이므로 기존 layout/commands.rs 테스트 5건은 손대지 않고 그대로 통과한다.
- **위치 선택 근거**: `tests/domain_boundaries.rs` 소스 확인 결과 두 후보가 모두 허용된다 —
  `ide/server.rs → layout::service` 는 이미 `ALLOWED_CROSS_DOMAIN_EDGES` 에 있고, infra 는 도메인
  어디서나 참조 가능하다. infra 를 택한 이유는 (1) 화이트리스트를 건드리지 않고, (2) 두 호출부가 이미
  같은 `root_guard` 의 경로 해석을 쓰고 있어 "해석한 경로를 검증한다"가 같은 모듈의 연속이며,
  (3) 존재 검사는 레이아웃 도메인의 지식이 아니기 때문이다. `layout::service` 는 계약대로 여전히
  무검증(순수 인메모리 트리)이다.
- **IDE 호출부**: `ide/server.rs` 의 `tool_open_file` 에서 경로 해석부를 순수 함수
  `resolve_open_file_target(&HashMap<ProjectId, Project>, &str) -> Result<(ProjectId, String, String), ToolError>`
  로 분리하고, 경계 검사 뒤 `root_guard::ensure_existing_file` 을 호출한다. 실패는 그 핸들러의 기존
  관례대로 `tool_error(RPC_INVALID_PARAMS, error.to_string())` 로 변환한다(전용 에러 코드 신설 없음).
  분리한 이유는 테스트 가능성 — `tool_open_file` 나머지는 살아 있는 `AppHandle` 이 필요하고 이 저장소에는
  `tauri::test` mock-app 하네스가 없다(terminal/commands.rs:542·project/commands.rs:210 의 기존 판단과 동일).
- **테스트 1건 추가**(`ide/server.rs` tests, 기존 관례대로 `use super::*` + temp dir):
  `존재하지_않는_경로의_open_file은_탭을_만들기_전에_거절된다` — 실재 파일은 통과(title `a.rs`)하고,
  같은 루트의 사라진 경로는 `RPC_INVALID_PARAMS` + `file not found` 로 거절된다(= `open_tab_and_finish`
  에 닿기 전이므로 탭 미생성). `ToolError` 는 `Debug` 를 구현하지 않아 `expect_err` 대신 let-else 로 받는다
  (공개 타입에 derive 를 추가하지 않기 위함).
- **문서 정정**: `docs/ipc-contract.md` `layout_open_tab` 절의 "IDE 도구 핸들러는 이 게이트를 타지
  않는다" 를 "두 진입점이 `ensure_existing_file` 을 공유한다"로 교체, `layout/commands.rs` 의 해당 doc
  comment 를 같은 사실로 정정, `docs/features/agent-integration.md` 보안 절에 "존재 선검증" 불릿 신설.

#### 3.5.1 단위 테스트 산출물 (테스트 wf, `bun test` 1848 → 1867 / +19건)

배치 3 신규 코드 중 테스트가 없던 쿼리 계층·순수 로직을 덮었다. 프로덕션 소스는 무변경이다.

| 파일 | 신규/보강 | 파일 내 케이스 | 고정한 계약 |
|------|:--------:|:-----------:|------|
| `src/entities/file/file.query.test.ts` | 신규 | 5 | 계약 A.2-5 — `useCreateEntry`·`useRenameEntry`·`useCopyEntry`·`useDeleteEntry` 성공 시 `QUERY_KEY.SEARCH.PROJECT_FILES(projectId)` 무효화(+`refetchType:'all'` 로 inactive 쿼리까지 재조회), 다른 프로젝트 인덱스 불변, `useSaveFile` 은 인덱스 미접촉 |
| `src/shared/lib/ipc-error-message.test.ts` | 보강 | 13 | `isNotFoundIpcError` 의 `Forbidden`·`Io`·`InvalidArgument`·message-only·문자열/null 경계 |
| `src/widgets/explorer/explorer-auto-reveal.test.ts` | 보강 | 13 | 계약 C.2-2 `decideAutoReveal` 게이트 우선순위(enabled → 뷰/사이드바 → 중복 억제 → 가시성) |
| `src/shared/lib/external-anchor.test.ts` | 보강 | 17 | 계약 B.2-5 `shouldOpenAnchorExternally` — 동일 오리진 false, 스킴/호스트 정규화, malformed·상대 href·`about:blank` false |
| `src/shared/lib/external-url-opener.test.ts` | 보강 | 8 | 계약 B.2-2 — 데스크톱은 `window.open` 미호출, 미러에서 브라우저 거부 시 셸 폴백 없음, `isRemoteMirror` 매 호출 재평가 |
| `src/shared/lib/remote/runtime-environment.test.ts` | 보강 | 5 | 계약 B.2-1 라벨 정확 일치 |
| `src/features/terminal/terminal-view.test.ts` | 보강 | 12 | 계약 B.2-4 수식어 게이트가 WebLinksAddon·`path:line:col`·OSC 8 세 링크 종류 공유(describe 명만 정정) |
| `src/shared/lib/pane-tree.test.ts` | (구현 단계) | 21 | 계약 C.2-1 `activeFilePathOf` |

- 8파일 합계 **94건 통과**. `file.query.test.ts` 는 `invalidateProjectFileIndex` 를 export 하지 않고
  `react-dom/server` `renderToString` + `QueryClientProvider` 로 훅을 1회 렌더해 `mutateAsync` 를 실제
  배선 위에서 실행한다(어느 훅이 무효화를 부르는가까지 고정). `@entities/layout/layout.ipc` 는 파일
  로드 순서 독립을 위해 전체 표면(19 export)을 스텁했다(`lsp.query.test.ts` 선례와 같은 사유).
- Rust 는 위 A-2 봉합 1건만 늘어 lib **1238 → 1239**.

#### 3.5.2 e2e 스펙 3건 — 작성 완료, **미실행**

`bun run e2e` 는 개발자가 `bun run tauri dev` 로 앱을 띄운 뒤 REMOTE 창을 준비해야 도는 하네스라
(문서 `docs/quality-assurance/2026-08-18-e2e-harness.md` §5) **이번 워크플로에서는 실행하지 못했다.**
`bun run typecheck:e2e` 만 통과(exit 0). 아래 3건은 **사용자 수동 실행 대상**이다.

| 스펙 | 케이스 | 검증 |
|------|:-----:|------|
| `e2e/specs/14-explorer-auto-reveal.e2e.ts` | 1 | ⌘P 로 `nested-only` 열기 → treeitem 출현 + `aria-selected=true`, 이어서 `index.ts` 열면 선택 이동. 전제(`explorerAutoReveal` 켜짐)는 `settings_get` 읽기로 확인 |
| `e2e/specs/15-explorer-auto-reveal-setting-off.e2e.ts` | 1 | 팔레트 → App: Settings → switch OFF → ⌘P 로 열어도 트리 행 0건, `try/finally` 로 ON 복원 |
| `e2e/specs/16-quick-open-missing-file-toast.e2e.ts` | 1 | 팔레트에 옵션이 뜬 뒤 Node `fs.rm` → 클릭 → `File not found` 토스트 + 탭 0건 + `layout_get` 미포함 (계약 A.2-1·A.2-2 의 사용자 증상 그대로) |

- 하네스 자산: `e2e/lib/explorer.ts` 신설(`explorerTreeRow` 로케이터 + `EXPLORER_SELECTED_ATTRIBUTE`),
  `e2e/lib/palette.ts` 의 `escapeRegExpLiteral` export(2곳 사용 → 공통화),
  `e2e/lib/fixture-project.ts` 에 `src/ephemeral.ts` 추가(다른 스펙의 심볼 카운트와 무관).
  `docs/quality-assurance/2026-08-18-e2e-harness.md` §2 실행표(16종)·§5 표(13~16 행)·§8 디렉토리 갱신.
- 미확인 가정 3가지(실행 시 우선 확인): 스펙 15 의 Radix Switch accessible name 이 `<label>` 래핑에서
  실제로 잡히는지, 스펙 16 의 `rm` → 클릭이 워처 에코(300ms 디바운스)보다 빠른지, 스펙 14 의 두 번째
  열기가 preview 탭 교체 흐름에서도 `select-only` 로 선택을 옮기는지.

#### 3.5.3 최종 검증 (메인, 2026-09-04 실측)

`export CARGO_HOME=... RUSTUP_HOME=... PATH=...` 후 실행.

- `bun run verify` → **exit 0** (typecheck 0 · lint 0 error/9 warning(전부 기존 `react-hooks/incompatible-library`·`exhaustive-deps`) ·
  `format:check` 통과 · `bun test` **1867 pass / 0 fail**, 3102 expect, 193 files ·
  `cargo fmt --all --check` 통과 · `cargo clippy --workspace --all-targets -- -D warnings` 통과 ·
  `cargo test --workspace` **1265 pass / 0 fail**(lib 1239 · domain_boundaries 3 · session_restore 6 · taide_cli 17 · doc 0))
- `bunx vite build` → **exit 0** (기존 chunk-size 경고만)
- `bun run typecheck:e2e` → **exit 0**
- `cargo test` 의 specta 재생성 후 `git diff src/shared/api/bindings.ts` 재확인 → 이번 배치의
  `explorerAutoReveal` 2필드(+doc comment)뿐, 예상 밖 diff 0.
