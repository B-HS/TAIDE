# 기능 — 터미널

> FR-G. pty 세션·셸 감지·flow control·블록(OSC 133)·파일 링크·폰트 크기·복원.
> API 확정 근거: `docs/research/xterm-pty.md` (xterm 6.0 breaking·portable-pty 0.9 시그니처·
> Channel Raw 전송 — 구버전 자료와 다르므로 반드시 이 문서 기준). ADR-0005.

## 1. 구성

- view: `@xterm/xterm@6` + addon(fit, webgl(+onContextLoss 재로드), search, serialize, unicode11,
  clipboard(OSC52 — Tauri clipboard-manager 위임 provider), web-links(URL), ligatures, progress).
  canvas addon 은 v6 미대응 — 폴백은 DOM 렌더러.
- Rust: `domain/terminal` + `infra/pty.rs`(portable-pty 0.9). 세션 구조체가
  master·writer(take_writer 1회 — 보관 필수)·killer(`clone_killer`)·child 를 소유.
  **spawn 후 slave drop 확인**(EOF 감지 조건 — research 함정 2).
- 세션은 Rust 소유(ADR-0004): 탭 전환·view reload 에도 pty 유지(FR-G5).

## 2. 데이터 경로 (성능 핵심)

- **Channel 전용**(이벤트 금지 — 순서 뒤바뀜·JSON 오버헤드). 출력은
  `InvokeResponseBody::Raw(bytes)` 로 전송(`Vec<u8>` 직접 send 금지 — JSON 숫자 배열이 됨).
- Rust reader 스레드: 64KB read buf, **4ms/64KB 배칭** 후 send.
- **flow control**: view 는 `term.write(bytes, callback)` 미완료 바이트를 집계 —
  HIGH_WATER(512KB) 초과 시 `pty_set_paused(true)`, LOW_WATER(64KB) 미만 시 재개.
  Rust reader 는 pause 플래그(Condvar) 로 read 루프 정지 → pty 커널 버퍼가 자식을 자연 블록.
- **pause 는 세션 단위이고 detach 로 풀리지 않는다**(`pty_detach` 는 구독자 목록만 건드린다).
  그래서 view 는 (d-51 F5, 2026-08-29) ① attach 이펙트 cleanup 에서 자기가 올린 pause 를 내리고
  ② attach 시 무조건 `pty_set_paused(false)` 로 재동기한다. ①이 없으면 버스트 도중 탭을 바꾼
  터미널의 자식 프로세스가 백그라운드에서 멈추고, ②가 없으면 재마운트한 view 가
  `INITIAL_FLOW_CONTROL_STATE`(paused=false)에서 출발해 재개를 영영 보내지 않는다(감사 §4-B D5 —
  webview reload·창 종료처럼 cleanup 이 돌지 않은 경로까지 ②가 덮는다).
- UTF-8 은 어디서도 미리 문자열화하지 않고 bytes 그대로 `term.write(Uint8Array)`
  (청크 경계 멀티바이트 — xterm 이 처리).
- 상수(`HIGH_WATER` 등)는 `shared/constants/terminal.ts` (research 값 채택).

## 3. 스크롤백 · 복원

- **Rust 가 세션별 출력 ring buffer(기본 2MB)를 보관**한다. view (재)마운트 시
  `pty_attach(sessionId)` 가 ring buffer 를 재생(replay)한 뒤 라이브 스트림을 잇는다 —
  view reload·탭 전환 복원의 근거(NFR-3).
- **d-50 S4(2026-08-29)**: 링버퍼가 `VecDeque` 기반이 되어(§2 M-5) 리플레이는 링의 두 조각을
  앞→뒤 순서로 최대 2청크에 나눠 보낸다(되감기기 전에는 1청크). 리플레이와 구독자 등록은 하나의
  락 아래 원자적이라 그 경계에서 청크가 중복되거나 유실되지 않는다 — 계약은
  `docs/ipc-contract.md` "d-50 S4" 절.
- xterm `scrollback: 10_000`. 앱 재시작 시 스크롤백은 복원하지 않는다(`data-model.md` §1) —
  터미널 탭은 같은 cwd·셸로 새 세션 + "이전 세션" 안내.
- 백그라운드(비활성) 터미널 탭: xterm 인스턴스는 유지하되 WebGL addon 은 dispose(활성화 시 재로드)
  — GPU/rAF 낭비 방지(research §12).

### 3.1 세션 로스터 캐시 (d-51 F5, 2026-08-29)

재부착 판정의 실제 근거는 `terminal_sessions(projectId)` 쿼리(`TERMINAL.SESSIONS`)다 —
탭이 들고 있는 `sessionId` 가 **아직 살아 있는지**를 이 목록으로 확인해 attach / 새 스폰을 가른다
(`isTerminalSessionAlive`). 이 쿼리는 `staleTime: Infinity` 라 스스로 다시 받아오지 않으므로,
로스터를 바꾸는 사건은 전부 캐시에 직접 써야 한다(`entities/terminal/terminal-session-cache.ts`).

- **스폰 성공** → `upsertTerminalSession` 으로 방금 만든 세션을 목록에 넣는다. 이 쓰기가 없으면
  로스터가 스폰 이전 상태로 남아, 탭을 떠났다 돌아올 때마다 "지속된 세션이 죽었다"고 읽고 **새 셸을
  또 스폰**하며 앞의 셸을 고아로 만든다(감사 §4-B A6 — 이 절이 규정한 복원 설계 위반).
  `shell` 필드는 `pty_spawn` 의 `unwrap_or_else(|| "default")` 를 `DEFAULT_SHELL_LABEL` 로 미러한다
  (specta 표면이 아니라 값 미러가 유일한 수단).
- **`terminal:exited`** → `ipc-sync-provider` 가 **전역으로** 받아 `markTerminalSessionExited` 로
  `running:false` 를 찍는다. 배경 탭의 세션은 마운트된 컴포넌트가 없어 자기 세션 가드 안에서는 아무도
  듣지 못했고, 그 결과 죽은 세션에 attach 해 입력만 먹는 터미널이 됐다(감사 §4-B B14). 무효화가 아니라
  즉시 쓰기인 이유는 리페치가 도착하기 전에 재마운트가 캐시를 읽고 attach 해버리기 때문이다.
- **탭이 닫히며 고아가 될 세션** → `removeTerminalSession` + `pty_kill`(§10).

### 3.2 탭 활성 직후의 입력·포커스 (d-51 F5, 2026-08-29)

터미널 탭을 활성화하면 view 가 새로 마운트되고, 크기를 잰 뒤(`onReady`)에야 스폰이 시작된다.
그 사이의 두 구멍을 닫는다(감사 §4-B C14).

- **스폰 완료 전 타이핑**: `sessionId` 가 없다고 버리지 않고 `pendingInputRef` 에 모았다가 스폰 직후
  한 번에 `pty_write` 한다(`appendPendingTerminalInput`, 상한 4096자·앞에서부터 버림 —
  `terminal-write-bridge` 큐 정책과 동일). 스폰이 진행 중일 때만 모은다(플러시할 주체가 없으면
  버린다). 스폰 실패 시 버퍼를 비운다.
- **포커스**: view 생성 직후 `term.focus()`. 단 **포커스된 pane 의 탭일 때만**(`autoFocus` prop =
  `node.id === focusedPaneId`) — 분할된 레이아웃을 복원할 때 배경 pane 의 터미널이 포커스된 pane 의
  에디터와 포커스를 다투지 않게 한다. 에디터(`code-editor.tsx` 의 모델 부착 시 `editor.focus()`)와
  같은 관례다.

## 4. 셸 (FR-G1)

- 기본: `CommandBuilder::new_default_prog()` — unix 로그인 셸 규약($SHELL→passwd→/bin/sh,
  argv[0] `-zsh`), Windows `%ComSpec%`. **default_prog 빌더에 arg 추가 금지(panic)**.
- 셸 선택 UI 용 열거(`shell_profiles` query): macOS `/etc/shells` + Homebrew 경로 존재 검사,
  Linux `/etc/shells`, Windows 프로필 열거(PowerShell7/5.1, cmd, Git Bash 레지스트리+기본경로 폴백,
  WSL 은 Lxss 레지스트리 — `wsl -l -q` UTF-16 함정 회피). 프로젝트별 셸 오버라이드 설정.
- env: `TERM=xterm-256color`, `COLORTERM=truecolor`, `TERM_PROGRAM=TAIDE` +
  `TAIDE_SHELL_INTEGRATION`·EDITOR 관련(`agent-integration.md`).
- Windows: `windowsPty: { backend: 'conpty', buildNumber }` 를 xterm 에 전달(리사이즈 유실 방지 —
  research 함정 10).

## 5. 명령 블록 — OSC 133 (Wave E 구현 완료)

> 계약: `docs/acknowledge/2026-08-15-wave-e-terminal-tasks-contract.md`. 설계 근거:
> `docs/research/xterm-pty.md` §8(구현 반영 갱신됨).

- **주입**(`infra/shell_integration.rs`, spawn 시 `TAIDE_SHELL_INTEGRATION` 미설정일 때만):
  - **zsh**: `ZDOTDIR` 을 프로젝트마다 새로 만드는 임시 디렉터리로 돌리고, 그 안에 `.zshenv`·
    `.zprofile`·`.zshrc` 세 파일을 심는다(VS Code 방식). 세 파일 모두 "원래 `ZDOTDIR`(또는
    `$HOME`)로 잠깐 돌아가 사용자의 동명 파일을 source 한 뒤 다시 임시 디렉터리로 복귀"하는
    패스스루 구조라, zsh 자체가 이 세 파일보다 먼저 읽는 어떤 시작파일도 임시 디렉터리 안에서
    빈손으로 끝나지 않는다. `.zshrc` 만은 사용자 rc 를 source 한 뒤 `ZDOTDIR` 을 영구 복원하고
    OSC133 훅(precmd/preexec)·`PS1` 마커를 마지막에 덧붙인다 — 로그인 셸의 `.zlogin` 은 이 시점
    이후 사용자의 실제 `ZDOTDIR` 에서 그대로 읽히므로 별도 임시 파일이 필요 없다.
    powerlevel10k instant prompt 충돌 방지로 `POWERLEVEL9K_INSTANT_PROMPT=quiet` 기본값을 깐다.
  - **bash**: `--init-file` 로 주입 스크립트를 지정한다. `CommandBuilder::new(shell)` 경로는 로그인
    프리픽스가 없어, `config.shell` 이 비어 기본 로그인 셸로 스폰됐을 세션은 `/etc/profile` +
    `.bash_profile`/`.bash_login`/`.profile` 캐스케이드를 스크립트가 직접 재현한다(비로그인
    오버라이드는 `.bashrc` 만). `PROMPT_COMMAND`/`PS0`/`PS1` 체이닝으로 A/B/C/D 를 방출.
    **알려진 한계**: macOS 기본 배포 bash(3.2.57)는 `PS0` 를 지원하지 않는(bash 4.4+ 전용) 구버전이라
    `C`(output-start) 마커가 발생하지 않는다 — `A`/`D` 만으로 블록 경계·종료코드는 정상 동작하는
    degradation 으로 수용(사용자 승인 필요 시 DEBUG trap 대안 검토, 2026-08-15 acknowledge).
  - **fish**: fish-shell#10352(fish **4.0+**)부터 `fish_prompt`/`fish_preexec`/`fish_postexec` 가
    OSC133 을 네이티브로 방출하므로 주입하지 않는다(`FishNative` 분기, `prepare()` 가 `None`
    반환). **알려진 갭**: fish 4.0 미만(2025년 이전 설치, 여전히 존재)은 버전 감지·폴백 주입이
    없어 OSC133 을 전혀 받지 못한다 — 2026-08-15 acknowledge 에 기록된 의도적 보류.
  - 공통: 각 셸 스크립트는 자기 임시 디렉터리를 정리 후 삭제(`rm -rf`), 경로는
    `infra/shell_quote::posix_quote` 로 인용(홑따옴표 idiom + fish 의 `\\`/`\'` 이스케이프까지
    고려한 백슬래시 이중 이스케이프).
  - PowerShell 은 후속(macOS zsh/bash/fish 우선).
- **파싱**(`features/terminal/terminal-osc133.ts`): `parser.registerOscHandler(133, ...)` → 순수
  리듀서(`applyOsc133Event`)가 A(시작)/C(출력 시작)/D(종료+exit) 로 블록 모델을 갱신한다(핸들러는
  `false` 반환 — 기본 처리 위임). `startMarker` 가 블록의 유일한 안정 식별자이며, 데코레이션은
  `startMarker` 를 키로 `WeakMap` 에 보관한다.
  - **dead marker 정리**: 블록의 `startMarker` 가 scrollback 밀림으로 `onDispose` 되면
    `pruneDisposedBlocks` 가 배열에서 제거하고 `currentBlockIndex` 를 (숫자 위치가 아니라) 블록
    식별자 기준으로 재계산한다 — 앞선 블록이 먼저 정리돼도 열린 블록의 인덱스가 어긋나지 않는다.
  - **상한**: 개행 없이 `133;A` 만 반복하는 비정상 출력에 대비해 `MAX_TRACKED_COMMAND_BLOCKS`(500)
    를 넘으면 가장 오래된 블록의 마커를 강제 dispose 해 같은 정리 경로를 재사용한다.
- **UX**: 거터 박스섀도 + `overviewRulerOptions` 스크롤바 데코(테마 success/failure 색상, exit
  code 0/비0), `⌘↑`/`⌘↓`(키맵 `terminal-jump-to-previous-command`/`-next-command`)로 이전/다음
  명령 블록으로 스크롤. 블록 단위 복사는 여전히 2차(§11) — OSC7 cwd 추적 기본 배선은
  X-A 배치(2026-08-19)에서 완성됐다(§9), 남는 고도화(청크 경계에 걸친 시퀀스 재조립 등)만 2차.

## 5.1 태스크 러너 · Run Selected Text (Wave E, `tasks.md`)

팔레트의 "Run Task"(`detect_tasks` query)와 에디터의 "Run Selected Text in Terminal" 은 모두 이
문서의 IPC(§9) 를 재사용해 텍스트를 터미널에 흘려보낸다. 상세는 `docs/features/tasks.md`.

## 6. 파일 링크 (FR-G2)

- URL 은 web-links addon, **파일 경로는 커스텀 `registerLinkProvider`** (research §10 구현 채택 —
  `path:line:col` 패턴, 1-based 좌표, wrapped line 처리).
- cmd(ctrl)+click → Rust `resolve_terminal_path(path, cwd)` (OSC7 cwd 기준 존재 검증) →
  해당 프로젝트 새 탭으로 열기 + line/col 로 커서 이동. modifier 없는 클릭은 무동작(터미널 관례).
- hover 툴팁 DOM 은 `term.element` 내 `xterm-hover` 클래스(이벤트 관통 방지).
- **좌표는 문자열 인덱스가 아니라 셀 열이다**(d-51 F5, 2026-08-29). 매치 문자 자체는 전부 단일폭
  ASCII 지만 **그 앞에 오는 것**(CJK 로그 접두·이모지 상태 표시)은 아니어서, 와이드 글리프 하나마다
  링크 range 가 한 칸씩 밀렸다(감사 §4-B C13 — 밑줄·클릭 판정이 실제 경로와 어긋남).
  `readTerminalRowColumns` 가 행을 셀 단위로 훑어 문자열(코드유닛)→열 매핑을 만들고 range 를 그
  매핑으로 잡는다. xterm 공개 API 로는 대체 불가다 — 내부 `BufferLine.translateToString` 은
  `outColumns` 아웃파라미터를 받지만 공개 `BufferLineApiView` 가 인자 3개까지만 전달하고 4번째를
  버린다(그대로 넘기면 조용히 빈 배열이 온다).

### 6.1 URL 링크 열기 — 외부 URL 은 항상 OS 브라우저 (사용성 배치 3, 2026-09-04 전면 개정)

> 계약: `docs/acknowledge/2026-09-04-usability-batch3-contract.md` §B. 증상·원인 정본은
> `docs/bug/2026-09-04-external-link-opens-in-app-window.md`. 이 절은 2026-08-18 손 QA 1차 수정
> (`2026-08-18-hand-qa-fix-contract.md` §2.1)의 "`window.open` 선시도 → IPC 폴백" 설계를 **폐기**하고
> 대체한다.

**폐기 사유.** 1차 설계는 데스크톱에서도 `window.open()` 을 먼저 부르고 `null` 일 때만
`system_open_external_url` IPC 로 폴백했다. 이 순서는 두 가지를 전제하는데 둘 다 유지될 수 없다.
① 데스크톱 webview 가 팝업을 *반드시* 거부한다는 전제 — 거부 여부는 wry/WKWebView 의 구현 세부이고,
`new_window_req_handler` 가 없을 때 무엇이 돌아오는지에 앱의 정책을 걸어 두면 그 세부가 바뀌는 순간
외부 페이지가 TAIDE 창 안에 열린다(주소창·뒤로가기가 없어 사용자가 빠져나올 수 없다).
② 실패했을 때만 폴백하므로, 팝업이 "성공"으로 보이면 IPC 경로는 영원히 실행되지 않는다.
현행 설계는 순서를 정하는 대신 **환경별로 경로를 분리**해 데스크톱에서 `window.open` 을 아예 호출하지
않는다.

**단일 진입점.** 앱에서 외부 URL 을 여는 모든 코드는 `entities/system/external-url.ts` 의
`openExternalUrl` 하나만 쓴다. 구현은 순수 팩토리 `shared/lib/external-url-opener.ts`
`createExternalUrlOpener` 이고, 주입되는 세 의존은 아래와 같다.

| 의존 | 데스크톱 | 원격 미러 |
|------|---------|----------|
| `isRemoteMirror` | `false` (`getCurrentWindow().label` 이 `main`·`editor-<n>`) | `true` (라벨 `remote`) |
| `openViaShell` | `system_open_external_url` IPC — **항상 이 경로** | 호출하지 않음 |
| `openViaBrowser` | **호출하지 않음** | `openViaBrowserWindow` (빈 탭 open → `opener` 절단 → `location.href`) |

- 환경 판별은 `shared/lib/remote/runtime-environment.ts` `isRemoteMirrorRuntime` — 원격 shim 이
  이미 보고하는 `getCurrentWindow().label === REMOTE_WINDOW_LABEL('remote')` 를 재사용하며 전역
  상태를 새로 만들지 않는다. 순수 판정부 `isRemoteMirrorLabel(label)` 이 분리돼 있어 `bun:test`
  (window 없음)에서 검증된다.
- 원격 미러에서 IPC 를 쓰지 않는 이유: 원격 dispatch 가 `system_open_external_url` 을 명시적으로
  거부한다(`remote/dispatch.rs`). 반대로 데스크톱에서 `window.open` 을 쓰지 않는 이유가 위 폐기
  사유다. 원격에서 브라우저가 팝업을 거부하면 더 시도할 곳이 없으므로 throw 한다.
- Rust 측은 `system_open_external_url` 이 http(s) 화이트리스트로 스킴을 재검증한다
  (`ipc-contract.md`). 프론트 판정과 2중 방어다.

**터미널 링크 3종이 같은 게이트·같은 오프너를 탄다.** 수식어 게이트
`shouldActivateTerminalLink`(`event.altKey || (isMac ? event.metaKey : event.ctrlKey)` — ⌘/Ctrl
또는 ⌥ + 클릭, 수식어 없는 클릭은 무동작)를 세 경로가 공유한다.

| 링크 종류 | 제공자 | 활성화 경로 |
|-----------|--------|------------|
| 평문 URL | `WebLinksAddon`(핸들러 주입형) | 게이트 → `onOpenLink` → `openExternalUrl` |
| `path:line:col` | `terminal-file-link.ts` `registerLinkProvider` | 게이트 → `resolve_terminal_path` → 에디터 탭 |
| **OSC 8 하이퍼링크** | xterm 코어 `OscLinkProvider`(우선순위 0) | **Terminal 옵션 `linkHandler`** → 게이트 → `onOpenLink` |

- OSC 8 이 별도 처리를 요구하는 이유: 코어의 `OscLinkProvider` 가 우선순위 0 으로 먼저 등록되고,
  같은 셀에서 겹치는 하위 링크는 제거되므로 `WebLinksAddon` 핸들러가 아예 호출되지 않는다.
  `linkHandler` 를 주지 않으면 xterm 의 `defaultActivate` 가 `confirm(...)` 뒤에 `window.open()` 을
  부르는데, wry 0.55.1 은 `runJavaScriptConfirmPanel` 을 구현하지 않아 데스크톱에서 `confirm()` 은
  항상 false — gh·vite·bun·eza·Claude Code 처럼 OSC 8 로 링크를 찍는 출력만 "아무 반응 없음"이 됐다.
- `linkHandler.allowNonHttpProtocols: false` — OSC 8 텍스트가 http(s) 가 아니면 링크로 취급하지
  않는다(xterm 이 `new URL(text).protocol` 로 걸러낸다). Rust 화이트리스트와 합쳐 2중 방어.
- `linkHandler` 는 **OSC 8 에만** 적용된다(xterm 소스 확인: `OscLinkProvider.ts` 외 참조처 없음).
  `WebLinksAddon`·커스텀 파일 링크 provider 의 `activate` 는 영향을 받지 않는다.
- hover 툴팁은 추가하지 않는다(`WebLinksAddon` 에도 없어 기존 동작과 일치 — 백로그).
- Terminal 생성 옵션 `altClickMovesCursor: false` 는 그대로 유지한다. 기본값(`true`)이면 ⌥클릭이
  "커서 이동"과 "링크 열기"로 동시 해석돼 충돌한다(사용자 승인을 거친 트레이드오프).
- 실패 시 `toast.error(t('terminal.openLinkFailed'))`.

**앵커 위임 핸들러(터미널 밖).** 마크다운 프리뷰가 렌더한 `<a href>` 는 터미널을 거치지 않는다 —
`marked` 는 `target="_blank"` 를 붙이지 않고, `tauri_plugin_opener` 가 주입하던 클릭 인터셉터는
`target=_blank`/Ctrl/Shift 만 잡는 데다 capability 가 없어 `preventDefault` 만 하는 죽은 코드였다.
그래서 앱 루트(`app/providers/external-link-provider.tsx`, `app.tsx` 의 **메인·보조 창 두 분기 모두**)
에서 `document` capture-phase `click` 을 받아 `composedPath()` 의 첫 `HTMLAnchorElement` 를 찾고,
순수 판정 `shared/lib/external-anchor.ts` `shouldOpenAnchorExternally(href, appOrigin)` 이 참이면
`preventDefault()` 후 같은 `openExternalUrl` 로 넘긴다. 판정 규칙은 **절대 http(s) URL + 앱 오리진과
다른 오리진**뿐 — 동일 오리진(앱 라우트·자산)·상대 경로·`mailto:`·`file:`·`blob:`·`tauri:` 는 전부
앱에 남는다. 좌클릭(button 0)이 아니거나 이미 `defaultPrevented` 인 클릭은 건드리지 않아 컨텍스트
메뉴와 기존 인앱 핸들러가 그대로 동작한다. 실패 시 `toast.error(t('common.openExternalLinkFailed'))`.

**WebView 레벨 가드.** JS 경로가 어떤 이유로든 새더라도 앱 창이 오리진 밖으로 이동하지 못하게
`on_navigation`/`on_new_window` 가드를 둔다 — 상세는 `docs/architecture.md` 의 "WebView 네비게이션
가드" 절.

## 7. 폰트 크기 (FR-G3)

- `attachCustomKeyEventHandler` 로 cmd(ctrl) `+`/`-`/`0`(리셋) 처리 → `term.options.fontSize` 대입 +
  `fit.fit()` (안 부르면 커서 밀림 — 함정 16). MIN 6.
- 크기는 설정(settings 도메인)에 저장 — 전 터미널 공통, 변경 시 열린 터미널 전체 반영.
- 컨테이너 리사이즈: ResizeObserver + rAF + `proposeDimensions()` 비교 후 fit
  (숨김 컨테이너 NaN 가드 — 함정 11). 순서: xterm resize → pty resize(함정 12).

## 8. 검색·기타 (FR-G4)

- 검색 바(`⌘F` — 터미널 포커스 시): SearchAddon, 결과 카운트(`onDidChangeResults`)·
  overviewRuler 하이라이트.
- 복사/붙여넣기: 선택 시 자동 복사 옵션, `⌘V`. OSC52 지원(clipboard addon).
- 진행률: progress addon(OSC 9;4) → 탭·앱 사이드바 뱃지에 반영 가능(2차).
- 이미지(Sixel/IIP)·리거처는 옵션(기본 off, `storageLimit` 제한).
- Shift+Enter → LF(`\n`) 변환(2026-08-29): xterm.js 는 kitty keyboard protocol 미지원이라
  Shift+Enter 를 CR 로만 보낼 수 있어, Claude Code 등 TUI 의 "줄바꿈 삽입"이 불가했다.
  `terminal-view.tsx` 의 커스텀 키 핸들러가 조합 키 없는 Shift+Enter keydown 만 LF(=Ctrl+J)로
  변환한다. 일반 셸에선 CR/LF 모두 accept-line 이라 동작 보존. 결정 정본
  `acknowledge/2026-08-29-terminal-shift-enter-decision.md`.

## 9. IPC

- mutation: `pty_spawn(opts, onData: Channel) → sessionId`(종료 통지는 `terminal:exited` 이벤트로만
  — onExit Channel 은 없다), `pty_write(sessionId, data)`, `pty_resize(sessionId, cols, rows)`,
  `pty_kill(sessionId)`, `pty_set_paused(sessionId, paused)`,
  `pty_attach(sessionId, onData) → subscriptionId`(재생+재구독 — 다중 구독자/멀티윈도우 지원),
  `pty_detach(sessionId, subscriptionId)`(Wave I)
- **`pty_write` 는 호출 순서를 스스로 보장하지 않는다**(d-50 S4 에서 블로킹 풀로 이관된 뒤로 — 같은
  틱에 발사된 두 호출은 독립 블로킹 태스크로 writer 뮤텍스를 경쟁한다). 같은 세션의 순서는
  **프론트가** `entities/terminal/session-write-order.ts` 의 세션별 프라미스 체인으로 보장하며,
  `writePty` 가 그것을 통과한다. 대기 큐 flush(`terminal-write-bridge.ts`)처럼 한 틱에 N개를 연속
  발사하는 경로가 실제로 있으므로 필요한 계약이다.
- query: `shell_profiles`, `terminal_sessions(projectId)`, `resolve_terminal_path(path, cwd)`,
  `pty_default_options`
- event: `terminal:exited(sessionId, code)`, `terminal:cwd-changed(sessionId, cwd)`(X-A 배치
  (2026-08-19)에서 배선 완성 — 이 항목이 계획하던 "OSC7 은 view 파싱 → mutation 으로 Rust 에 보고"
  방향은 실제 구현과 다르다: `events.rs` 가 애초에 `terminal:cwd-changed` 를 `Event` derive 로
  선언해(Rust→view 단방향) mutation 대응 짝이 없었고, X-A 배치도 그 구조를 그대로 따라 **Rust 가
  OSC7 을 직접 파싱해 발행**한다 — `infra::shell_integration` 의 zsh/bash 훅이 매 프롬프트마다
  `\e]7;$PWD\e\\` 를 pty 출력에 실어 보내고, `extract_latest_cwd` 가 그 raw 바이트를 스캔해
  `terminal::commands::pty_spawn` 의 `on_data` 콜백에서 이전 cwd 와 달라졌을 때만 이 이벤트를
  발행한다), `agent:state-changed`(`agent-integration.md`)

## 10. 수명주기 · 누수 방지

- 탭 닫기: `layout_close_tab`/IDE 도구의 탭 닫기가 공유하는 `close_tab_and_finish` 가 닫힌 탭이
  `TabKind::Terminal` 이면 그 `sessionId` 를 `TerminalStore::kill_session` 으로 즉시 회수한다
  (T0 감사 #21, `docs/acknowledge/2026-08-18-audit-t0-fix-contract.md` §2.3 — **접합부 수정,
  Phase D**: 사용자 결정 9 는 이 배선을 명시했으나 실구현 단계에서 어느 트랙도 실제 호출부를 연결하지
  않아 `pty_kill`/`killPty` 호출부가 0건인 채 남아 있었다). **"포그라운드 자식 프로세스 실행 중이면
  확인 다이얼로그"는 설계 의도로만 남아 있고 프론트에 실구현이 없다** — 현재는 무조건 즉시 kill 이다
  (탭 닫기용 프론트 확인 다이얼로그가 미구현이라는 뜻이다 — `PtySession::foreground_pid`/
  `TerminalStore::foreground_pids` 자체는 존재하며 시스템 사용량(`system/commands.rs`)·에이전트
  감지 폴링(`domain::agent::commands::poll_agents`) 용도로 별도 쓰인다, 감사로 확인). ring
  buffer 는
  탭이 닫히며 그 view state 와 함께 사라진다(별도 해제 커맨드 없음). view 는 xterm `dispose`.
- **스폰 도중 탭 닫기**(d-51 F5, 2026-08-29): 위의 회수는 탭에 **기록된** `sessionId` 만 죽인다.
  첫 스폰이 진행 중인 탭은 그 값이 아직 비어 있고, `pty_spawn` 이 전역 mutation guard 를 쥐고 있어
  `layout_close_tab` 은 pty 가 살아난 **뒤에야** 실행되므로, 닫기가 이긴 경우 셸이 아무 참조도 없이
  앱 종료까지 살아남았다(감사 §4-B C14). view 는 스폰 완료 후 `layout_set_terminal_session` 의
  성공 여부로 이를 가른다 — 탭이 사라졌으면 `NotFound` 가 오고, 그때만 `pty_kill` + 로스터에서 제거.
  단순 탭 전환은 탭이 그대로 있어 기록에 성공하므로 세션이 보존된다(둘 다 unmount 라 프론트 상태만
  으로는 구분할 수 없다).
- 프로세스 종료(exit) 감지: try_wait 폴링 또는 wait 스레드 → `terminal:exited` → 탭에
  "[process exited]" 표시 + 재시작 버튼. 로스터 갱신은 §3.1(전역 처리).
- view unmount(탭 전환): xterm dispose 하지 않고 DOM 분리 유지(활성 pane 내 다중 터미널 전환 시).
  프로젝트 전환으로 위젯 트리가 내려가면 xterm dispose — 재마운트 시 ring buffer 재생.
- **프로젝트 닫기**: `project_close` 가 그 프로젝트 소유의 모든 pty 세션을
  `TerminalStore::kill_project` 로 일괄 회수한다(T0 감사 #21 — 이전에는 프로젝트를 닫아도 세션이
  앱 종료까지 계속 살아 있었다).
- 앱 종료: 전 세션 kill(자식 프로세스 잔존 금지 — Drop + 명시 shutdown 이중화). `PtySession::drop`
  이 kill 과 일시정지 게이트 해제(`pause.set_paused(false)`)를 함께 수행하므로(T0 감사 #21) 탭
  닫기·프로젝트 닫기·앱 종료 중 어느 경로로 죽어도, 그리고 일시정지된 채로 죽어도 reader/flusher
  스레드가 새지 않는다. `pty_kill` → `kill_project`/`kill_session` 순으로 중복 kill 이 겹쳐도
  `kill()` 자체가 멱등해 무해하다.

## 11. 범위

| 1차 | 2차 |
|-----|-----|
| pty spawn/기본 셸·Channel Raw+배칭·flow control·ring buffer 복원·리사이즈·폰트 크기·파일 링크(cmd+click)·검색·복사/붙여넣기·셸 프로필 열거·**OSC 133 명령 블록(Wave E)**·**태스크 러너·Run Selected Text(Wave E, `tasks.md`)**·**OSC7 cwd 추적 기본 배선(X-A, 2026-08-19)** | OSC7 cwd 추적 고도화(청크 경계 재조립 등)·블록 단위 복사(serialize range)·progress 뱃지·이미지/리거처·분할 내 터미널 다중화·serialize 스냅샷 내보내기·PowerShell rc 주입·fish 4.0 미만 폴백 |


## 12. Phase 7.5 재평가 결과 (2026-08-06) — **xterm 유지, 우리 코드가 원인**

정본: `docs/research/terminal-reevaluation.md`.

### 12.1 결론: xterm.js 를 계속 쓴다

- Tauri 웹뷰에 임베드 가능한 성숙 라이브러리는 **xterm.js 가 사실상 유일**하다.
  Ghostty·WezTerm·Alacritty·Rio 는 전부 네이티브라 웹뷰 임베드 불가.
- **VS Code 도 같은 xterm.js 를 쓴다** — VS Code `main` 의 `package.json` 실측:
  `@xterm/xterm@^6.1.0-beta.292`. "VS Code 는 어떻게 했나"의 답은
  **"같은 라이브러리 + 다른 주변 파이프라인"** 이다.
- 유일한 후보 `@wterm/ghostty`(libghostty-vt WASM)는 npm 최초 발행 2026-04-30, 0.3.2 → **시기상조**.
  **6~12개월 뒤 재평가**로 기록.

### 12.2 자동완성 잔상의 원인 — 코드에서 특정한 3건

> 사용자 증상: "zsh-autocomplete 로 auto complete 한 내용이 백에서 남는다"

**A. (최유력) reader 배칭에 타이머 flush 가 없다** — `src-tauri/src/infra/pty.rs`

flush 판정이 `read()` **직후에만** 존재한다. 버스트 마지막의 소량 청크
(= zsh-autocomplete 가 회색 제안을 지우는 시퀀스)는 `elapsed < OUTPUT_BATCH_MS` 라 flush 되지 않고,
reader 가 블로킹 `read()` 로 들어가면 **다음 키 입력이 있을 때까지 Rust 메모리에 갇힌다.**
→ 화면에는 제안이 남아 있는 것처럼 보인다.

VS Code 는 `TerminalDataBufferer` 가 **첫 데이터 도착 시 `setTimeout(flush, 5)`** 를 걸어
시간 기준으로 무조건 내보낸다. 우리에게 없는 것이 정확히 그 타이머다.

**B. xterm 마운트 전에 80×24 로 spawn** — `domain/terminal/commands.rs` + `terminal-session.tsx`

`DEFAULT_TERMINAL_COLS/ROWS = 80/24` 로 먼저 띄우고 나중에 resize 한다.
셸이 SIGWINCH 를 놓치면 `COLUMNS` 가 80 에 굳는다.
xterm 의 `reflowCursorLine` 은 **기본 `false`**("shells usually handle this themselves")라
커서 줄이 리플로되지 않는다.
→ vscode#121891 의 증상 기술이 사용자 문구와 동일: *"autocomplete suggestions appearing on
incorrect lines or overwriting typed text"*.

**C. 탭 전환마다 xterm 파괴 → 2MB replay** — `pane-node-view.tsx` 가 활성 탭만 렌더

서로 다른 폭에서 생성된 바이트를 한 폭으로 재생하고, ring buffer 를 **바이트 단위로 절단**해
이스케이프 시퀀스 중간이 잘린다. 추가로 `pty_attach` 가 `ring_buffer` 와 `subscriber` 를
**별개 잠금**으로 잡아 스냅샷~교체 사이 청크가 유실되는 레이스가 있다.
→ **레이스는 d-50 S4(2026-08-29)에서 해소**됐다(감사 §4-A-5). 스크롤백과 구독자 목록이 하나의
`SessionOutput` 락으로 합쳐져 리플레이~등록이 원자적이다. 폭 차이·바이트 단위 절단은 그대로 남는
한계다(위 3번 항목의 replay 경로 자체를 없애는 안).

### 12.3 반증된 가설 (조사해서 아니라고 확인)

- 배칭이 ANSI 시퀀스를 쪼갠다 → **아니다.** xterm 파서가 `_preserveStack` 로 write 경계를 넘어
  상태를 보존한다.
- flow control 이 데이터를 버린다 → **아니다.** `PauseGate` 가 `read()` **이전에** 블록한다.
- DEC 2026 synchronized output / alt screen 미지원 → **아니다.** typings 에 `synchronizedOutputMode` 존재.
- WebGL 렌더러 잔상 → **미확인, 가능성 낮음.**

### 12.4 수정안

**P0 (잔상 해결)**
1. reader 에 **타이머 flush** 도입 — 별도 flusher 스레드 5ms 틱. (A)
   → 현재는 상시 틱이 아니라 **condvar 대기**(배치에 남은 바이트가 있을 때만 깨워 5ms 뒤 flush)다.
   지연 규약은 그대로이고 유휴 세션의 초당 200회 wakeup 만 사라졌다(d-50 S4, 감사 §2 L-2).
2. **실측 cols/rows 로 spawn** + 셸 준비 후 재-resize 안전망. (B)
3. 탭 전환 시 unmount 대신 **숨김 유지**로 replay 경로 자체를 제거. (C)

**P1**
4. `pty_write`/`pty_resize`/`pty_set_paused` 를 전역 `begin_mutation` 락에서 **분리**.
   현재 키 입력이 파일 저장·git 작업 뒤에 줄을 선다(`state.rs` mutation guard). — 코드 확인 결과 세
   커맨드 모두 애초에 `AppState::begin_mutation` 을 잡지 않는다(전역 락과는 무관). 실제 경합은
   `TerminalStore` 자신의 내부 `Mutex<HashMap<...>>` 쪽이었다: `pty_write` 가 자식 프로세스 stdin 이
   막혀 블로킹되는 동안 이 store 락을 붙들고 있어 다른 세션의 `pty_resize`/`pty_kill`·
   `terminal_sessions` 전부가 줄을 섰다 — T0 감사 #20(`docs/acknowledge/
   2026-08-18-audit-t0-fix-contract.md` §2.2)이 `pty_write` 를 "핸들 조회까지만 store 락, 실제
   쓰기는 `PtySession` 이 소유한 `Arc<Mutex<Writer>>` 로" 분리해 해소했다.

### 12.5 수정 전 원인 판별 체크리스트

세 원인 중 무엇인지 먼저 가른다(추측 금지):

- 터미널에서 `tput cols` → 80 이면 **B 확정**
- 잔상 상태에서 키 1회 입력 시 사라지면 **A 확정**(갇힌 청크가 다음 입력에 밀려 나옴)
- 탭 전환 직후에만 발생하면 **C 확정**
- DOM 렌더러로 바꿔도 재현되면 WebGL 무관

## 13. CJK 입력 (2026-08-06 추가)

**Tauri 2 의 macOS WKWebView 는 `compositionstart/update/end` 를 발생시키지 않는다.**
IME 결과가 `input` 이벤트의 `insertReplacementText` 로만 전달되며, xterm 의 `_inputEvent` 는
`insertText` 만 처리하므로 첫 자모 외 전부 유실된다.

→ `src/shared/lib/ime-input.ts` 의 `resolveImeInput` 어댑터가 이 경로를 번역한다.
이전 조합 길이만큼 `\x7f` 를 보내고 새 문자열을 보내 화면상 올바른 음절로 수렴시킨다.
xterm 이 이미 보내는 `insertText` 는 중복 전송하지 않고 조합 상태만 기록한다.

- **근본 수정이 아니다.** 원인·실측 비교·시도 이력은 `docs/bug/2026-08-06-wkwebview-ime-composition.md`.
- 상위 이슈 xterm.js #5887 은 open. Tauri 상위 버전에서 해결되면 어댑터를 제거한다.
- **터미널 라이브러리 교체는 해법이 아니다** — 웹뷰가 조합 이벤트를 안 주면 어떤 웹 터미널도 같다.
  Safari 에서 같은 xterm 이 한글을 정상 처리하는 것으로 xterm 무죄가 확인됐다(§12.1 결론 유지).
