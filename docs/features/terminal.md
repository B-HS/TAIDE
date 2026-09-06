# 기능 — 터미널

> FR-G. pty 세션·셸 감지·flow control·블록(OSC 133)·파일 링크·폰트 크기·복원.
> API 확정 근거: `docs/research/xterm-pty.md` (xterm 6.0 breaking·portable-pty 0.9 시그니처·
> Channel Raw 전송 — 구버전 자료와 다르므로 반드시 이 문서 기준). ADR-0005.

## 1. 구성

- view: `@xterm/xterm@6` + **실제 설치·로드된 addon 5종**(fit, webgl(+onContextLoss 재로드),
  search, unicode11, web-links(URL)) + 커스텀 파일 링크 provider. canvas addon 은 v6 미대응 —
  폴백은 DOM 렌더러.
  - `serialize`·`clipboard`(OSC52)·`ligatures`·`progress` 는 **설치돼 있지 않다**(2026-09-04 확인 —
    `package.json` 의 `@xterm/addon-*` 는 fit/search/unicode11/web-links/webgl 5개뿐). 도입은 백로그.
  - `search` 는 로드만 돼 있고 검색 UI 는 아직 없다(§8).
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
- **리플레이 바이트는 이 집계에서 제외한다**(d-56 T2-F3, 2026-09-07). `pty_attach` 는 최대 2MB
  스크롤백을 라이브와 **같은 채널**로 재생하므로, 그대로 세면 탭을 오갈 때마다 이미 지나간 출력이
  HIGH_WATER 를 넘겨 멀쩡한 자식 프로세스를 멈춘다. `pty_attach` 가 돌려주는
  `PtyAttachResult.replayBytes` 가 **예산**이 되어, `terminal-session.tsx` 의 attach 콜백이 도착
  순서대로 차감하고(`terminal-replay-budget.ts` 의 `consumeReplayBudget`) 남은 만큼만
  `TerminalAttachHandle.write(data, backlogBytes)` 의 두 번째 인자로 넘긴다. xterm `write` 자체와
  perf 카운터(`TERMINAL_OUTPUT_BYTES`)는 리플레이를 포함해 그대로 센다 — 화면에는 실제로 그려지기
  때문이다. 리플레이는 커맨드 **안에서** 전송되므로 결과(예산)보다 먼저 도착할 수 있어, 세션은
  결과가 올 때까지 청크를 큐에 담았다가 순서대로 흘린다(왕복 1회 지연).
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
- **축출은 개행 경계로 정렬한다(d-56 T2-F8, 2026-09-07)**. 2MB 를 넘긴 링은 넘치는 만큼을 바이트
  단위로 버렸고, 그래서 재부착 첫 화면이 잘린 OSC/CSI 한가운데에서 시작해 이스케이프 잔여가 텍스트로
  찍혔다(§12.2-C 가 남은 한계로 적어 두었던 것). 이제 `ScrollbackRing::append` 가 축출 지점을
  **그 이후 첫 `\n` 다음**까지 전진시킨다. 개행이 `MAX_EVICTION_SCAN_BYTES`(64KB) 안에 없으면
  원래 위치에서 자른다 — 개행이 드문 TUI 출력에서 링이 통째로 비는 것을 막기 위해서다. 비용은
  capacity 를 최대 한 줄만큼 덜 채우는 것.
- **리플레이 앞에는 SGR 리셋 프리앰블 `\x1b[0m` 1회**를 보낸다(같은 배치). 개행 정렬이 시퀀스를
  반으로 자르는 것은 막지만, 잘린 지점 앞에서 열린 SGR 을 닫아 주던 `\x1b[m` 까지 되살리지는
  못한다 — 프리앰블이 없으면 재생된 첫 줄이 그 색을 물려받아 화면 끝까지 끌고 간다. 이 4바이트도
  리플레이의 일부이므로 `PtyAttachResult.replayBytes` 에 포함되어 flow control 집계에서 빠진다(§2).
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
  - **빈 프롬프트 블록 폐기**(d-55 §1.B): 셸 훅은 프롬프트를 다시 그릴 때마다 `133;D;$?` → `133;A`
    를 찍으므로, 아무것도 입력하지 않은 Enter·Ctrl-C 로 비운 줄은 `C` 없이 `A → D` 만 남는다.
    상태의 `hasSeenOutputStart` 래치(그 세션에서 `C` 를 한 번이라도 열린 블록에 받으면 true, 이후
    유지)가 켜진 세션에서 `outputStartMarker` 없이 `D` 가 오면 그 블록은 빈 프롬프트로 보고 end
    마커를 등록하지 않은 채 배열에서 제거한다(`discardedBlock` 으로 돌려주면 트래커가
    `startMarker` 를 dispose 해 회수 — 그 dispose 가 다시 `pruneDisposedBlocks` 로 재진입해도 이미
    빠진 블록이라 무해). 그래서 빈 줄에 성공(초록)·Ctrl-C 실패(빨강) 거터 바가 생기지 않고,
    `⌘↑`/`⌘↓` 점프 후보와 500개 예산에서도 빠진다. `C` 를 낼 수 없는 셸(위 bash 3.2 한계)은 래치가
    켜지지 않으므로 **현행 그대로 모든 블록을 유지**한다 — 그 대신 그 셸에서는 빈 프롬프트 블록이
    남고, `C` 를 내는 셸에서도 **세션 첫 명령 이전의 빈 Enter 1회**는 래치가 아직 꺼져 있어 남는다.
- **UX**: 거터 박스섀도 + `overviewRulerOptions` 스크롤바 데코(테마 success/failure 색상, exit
  code 0/비0), `⌘↑`/`⌘↓`(키맵 `terminal-jump-to-previous-command`/`-next-command`)로 이전/다음
  명령 블록으로 스크롤. 블록 단위 복사는 여전히 2차(§11) — OSC7 cwd 추적 기본 배선은
  X-A 배치(2026-08-19)에서 완성됐고(§9), **청크 경계에 걸친 시퀀스 재조립은 d-54(§5.2)에서 해소**됐다.
- **완료 알림용 실행 시간은 프론트가 아니라 Rust 가 잰다**(사용성 배치 4 웨이브 1 리뷰 F-1):
  이 트래커는 xterm 인스턴스와 생사를 같이 하는데, `pane-node-view.tsx` 는 활성 탭만 렌더하므로
  **터미널 탭이 배경으로 가면 통째로 언마운트**된다 — 정작 알림이 필요한 "빌드 걸어두고 다른 탭으로
  갔다" 가 그 상태다. 그래서 이 트래커는 데코레이션·점프만 소유하고, "명령이 끝났다 + 얼마나
  걸렸다" 는 pty reader 스레드가 §5.2 의 스캐너로 같은 OSC 133 `C`/`D` 를 읽어
  `terminal:command-finished` 이벤트로 발행한다(§9). 프론트에서 재면 재부착 시
  스크롤백 재생이 `C`/`D` 를 수 ms 간격으로 다시 파싱해 몇 시간짜리 명령도 0ms 로 측정된다.

## 5.1 태스크 러너 · Run Selected Text (Wave E, `tasks.md`)

팔레트의 "Run Task"(`detect_tasks` query)와 에디터의 "Run Selected Text in Terminal" 은 모두 이
문서의 IPC(§9) 를 재사용해 텍스트를 터미널에 흘려보낸다. 상세는 `docs/features/tasks.md`.

## 5.2 pty 출력 스캐너 — `infra/terminal_scan.rs` (d-54, 2026-09-06)

> 계약: `docs/acknowledge/2026-09-06-d54-agent-activity-signals-contract.md` §1.3.
> 이전에는 `infra::shell_integration` 의 `extract_latest_cwd`·`extract_command_markers` 가 청크마다
> 시퀀스 종류별로 버퍼를 전수 탐색했고, 청크 경계에 걸린 시퀀스는 재조립하지 않았다. 두 함수는
> 제거되고 이 스캐너로 통합됐다(`CommandMarker` 타입과 셸 스크립트 조립은 `shell_integration` 소유
> 그대로).

- **소유**: 세션당 `OutputScanner { carry, text_tail }` 하나를 `pty_spawn` 의 `on_data` 클로저가
  `command_started_at` 과 같은 수명으로 캡처한다(`parking_lot::Mutex`).
  `scan(&mut self, chunk) -> ScanOutcome { events, text, overlap }` 이 청크당 1회 불린다.
  세션이 없는 호출자(테스트 등)를 위한 무상태 편의 함수 `scan_once(bytes)` 도 있다.
- **단일 패스**: `0x1b`(ESC)를 한 번만 찾아 도입 문자로 분기한다.
  - `]`(OSC) → `;` 앞의 ident 로 분기: `7` → `Cwd`(TAIDE 훅이 내는 순수 경로만 —
    `file://` 로 시작하면 건너뛴다) / `133` → `CommandMarker`(`C`·`D` 만) / `0`·`2` → `Title` /
    `777` → `notify;taide-agent;<body>` 인 것만 `AgentEvent`(body 는 `;` 재결합) /
    `9` → `Notification9`(첫 필드가 순수 숫자면 진행률 서브커맨드로 보고 버린다). 그 외 ident 는 버린다.
    종결자는 **BEL 과 ST(`ESC \`) 중 먼저 오는 쪽**이다 — ST 를 우선하면 BEL 로 끝난 시퀀스가 뒤쪽의
    무관한 ST 까지 삼킨다.
  - `[`(CSI) → 정규화 텍스트에만 반영(아래), `P`·`X`·`^`·`_`(DCS/SOS/PM/APC) → 페이로드째 제거,
    `(`·`)` → 3바이트 제거, 그 외 → 2바이트 제거.
- **경계 이월**: 종결자가 없는 미완 시퀀스는 `carry` 에 남겨 다음 청크 앞에 붙인다. 그래서
  두 청크에 걸쳐 잘린 `133;D` 나 다이얼로그 프레임이 유실되지 않는다.
  `carry` 가 `MAX_OSC_PAYLOAD_BYTES`(4096)를 넘으면 폐기하고 경고만 남긴다 — pty 출력은
  신뢰할 수 없고, OSC 를 열고 닫지 않는 상대가 세션마다 메모리를 고정하게 둘 수 없다.
- **하드닝**: 채택하는 문자열은 **할당 전에 길이로 거절**한다(`MAX_TITLE_BYTES` 512 — 타이틀·OSC 9
  본문, `MAX_AGENT_EVENT_BYTES` 1024 — 인밴드 이벤트 본문). 채택 후에는 C0 제어문자
  (0x00-0x1F·0x7F)를 제거한다.
- **정규화 텍스트**(`ScanOutcome.text`): 에이전트 활동 판정(`agent-integration.md` §1.2)이 읽는
  문자열이다. `CSI n G`·`CSI n C`(열 이동) → 공백 1개, `CSI … A/B/E/F/H/d/f`(행 이동) → 개행,
  나머지 CSI(SGR·erase·private mode 등)와 OSC/DCS/APC/PM·`ESC ( ) = >` → 제거, `\r` 제거,
  `\t` → 공백, 그 외 C0 제거, **연속 공백은 1개로** 접는다. Claude Code 가 다이얼로그 산문을
  단어마다 `CSI n G` 를 끼워 그리기 때문에, 이 치환을 거쳐야 문구가 하나의 문자열로 복원된다.
- **텍스트 꼬리**: 스캐너는 정규화 텍스트의 마지막 `TEXT_OVERLAP_BYTES`(128)만 보관했다가 다음
  청크의 `ScanOutcome.overlap` 으로 돌려준다(대용량 출력에서도 메모리 고정). `text` 와 합쳐 주지
  않는 이유는 소비처가 둘로 갈리기 때문이다 — 경계를 넘는 문구 매칭은 둘을 이어 보고, "이 청크가
  실제로 얼마나 찍었나" 는 `text` 만 센다.
- **소비**: `terminal::commands::dispatch_scan_outcome` 한 곳이 결과를 나눠 준다. 먼저
  `ScanOutcome::latest_cwd()`(청크의 **마지막** OSC 7 만 — 한 청크에 프롬프트가 두 번 실려도
  중간값을 발행하지 않는다) → `terminal:cwd-changed`, 다음 `CommandMarker` → §5 의
  `terminal:command-finished`, 마지막으로 결과 전체를 `PtySessionObservers` 에 넘긴다(에이전트
  도메인의 세션 신호 — `lib.rs` 가 조립하는 assembly-owned 배선).
- **계측**: 청크당 이벤트 수를 `pty.scan_events` 카운터에 더한다(`debugging.md` §4.1). 리더
  스레드라 구간 시간은 재지 않는다(기존 `pty.output_bytes`·`pty.output_chunks` 와 같은 이유).

## 6. 파일 링크 (FR-G2)

- URL 은 web-links addon, **파일 경로는 커스텀 `registerLinkProvider`** (research §10 구현 채택 —
  `path:line:col` 패턴, 1-based 좌표, wrapped line 처리).
- **정규식 매치는 존재 검증을 통과한 것만 링크가 된다(d-56 T5-01, 2026-09-07)**. 예전에는 매치 전부에
  밑줄·포인터를 그리고 검증은 클릭 뒤에 했으므로 `v18.20.4`·`127.0.0.1:8080`·`0.123s`·`e.g` 가
  링크처럼 보였고 클릭하면 실패 토스트만 남았다. 이제 provider 가 한 행의 후보를 모아
  `terminal_resolve_link_candidates(cwd, candidates)` 로 **한 번에** 물어(입력 순서 보존, 행당 최대
  16개, 루트 밖·부재는 똑같이 `null` — 존재 여부 오라클 비노출) 절대 경로가 돌아온 후보만 `ILink` 로
  만든다. 답은 (cwd, 행 텍스트) 키 FIFO 캐시(`LINK_RESOLVE_CACHE_MAX_ROWS`=256)에 담겨 같은 행을 다시
  hover 해도 IPC 가 나가지 않고, 실패는 캐시하지 않아 다음 hover 에서 다시 시도한다. xterm 은
  `provideLinks` 의 지연 콜백을 허용하므로 비동기 해석이 안전하다.
- **캐시는 부재(`null`)도 담는다 — 의도한 트레이드오프(d-56 검토 d56-1, 2026-09-07)**. 후보가 전부
  파일이 아닌 행(버전 문자열·호스트:포트 로그)이야말로 포인터가 가장 자주 지나는 행이라 이것을 매번
  다시 물으면 캐시가 무의미해진다. 대신 **행이 찍힌 뒤에 생긴 파일**은 그 행에서 링크가 되지 않는다 —
  캐시 항목이 FIFO 로 밀려나거나 cwd 가 바뀌어야 다시 검사한다. 그 파일을 여는 경로(탐색기·검색·직접
  경로 입력)는 그대로 살아 있고, resolver 호출 자체가 실패한 경우만 캐시에서 빠진다.
- cmd(ctrl)+click → 이미 해석된 절대 경로로 해당 프로젝트 새 탭 열기 + line/col 로 커서 이동
  (클릭 시점의 두 번째 IPC 없음). modifier 없는 클릭은 무동작(터미널 관례).
- provider 는 `features` 레이어라 IPC 를 직접 부르지 않는다 — cwd getter 와 resolver 를 주입받고,
  둘 다 `widgets` 의 `terminal-session.tsx` 가 넘긴다(FSD).
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
| 파일 경로 + 좌표 접미사 (아래 목록) | `terminal-file-link.ts` `registerLinkProvider` | 게이트 → `resolve_terminal_path` → 에디터 탭 |
| **OSC 8 하이퍼링크** | xterm 코어 `OscLinkProvider`(우선순위 0) | **Terminal 옵션 `linkHandler`** → 게이트 → `onOpenLink` |

- 좌표 접미사 문법은 `src/shared/lib/terminal-link.ts` 의 `TERMINAL_LINK_SUFFIX_PATTERNS` 한 곳에
  모여 있고, 경로 뒤에서 **가장 긴 성공 접미사**를 채택한다(d-55 §1.A, 2026-09-06). 지원 형식:
  `path:12` · `path:12:3`(뒤에 붙는 `: error` 의 콜론은 링크 밖) · `path:10-20`(범위 → 시작 줄) ·
  `path(12,3)` · `path(12)`(tsc `--pretty false`·MSBuild) · `path#L42` · `path#L10-L20` ·
  `path#L10-20`(GitHub) · `path", line 42` · `path', line 42`(python 트레이스백 — 경로를 닫는
  따옴표가 문법의 일부).
- 밑줄 범위(`TerminalLinkMatch.text`/`endIndex`)는 접미사를 포함해 `(12,3)`·`, line 42` 까지 덮지만
  `path` 는 접미사를 제외한 원문이다 — Rust `resolve_terminal_path` 는 `match.path` 만 받으므로 IPC
  계약(§9)은 문법이 늘어도 그대로다.
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

### 6.2 우클릭 컨텍스트 메뉴 (사용성 배치 4, 2026-09-04)

> 계약: `docs/acknowledge/2026-09-04-usability-batch4-contract.md` §F. 조사 원문
> `docs/research/2026-09-04-batch4-terminal-tabbar-context-menu-research.md` 주제 6.

**항목.**

```
복사(선택 없음·클립보드 불가 시 비활성) · 붙여넣기(클립보드 불가 시 비활성) · 모두 선택
── 지우기
── 분할 ▸ 왼쪽 / 오른쪽 / 위 / 아래 (각 방향은 크기가 모자라면 비활성) · 새 터미널
── 터미널 종료
```

- 라벨은 신규 5키(`terminal.copy`·`paste`·`selectAll`·`clear`·`kill`) + 기존 `tab.split`·
  `editorArea.split*`·`tab.newTerminal` 재사용.
- **분할 = "그 방향에 새 터미널을 만든다"**(VS Code `workbench.action.terminal.split` 파리티).
  탭 우클릭 메뉴·`⌘\` 의 분할이 **이 탭을 옮기는** 것과 의미가 다르다. 전용 커맨드
  `layout_open_tab_in_split` 을 쓰는 이유는 왕복 절감이 아니라 정확성이다 — 프론트에서
  `layout_open_tab` + `layout_split` 으로 합성하면 (a) 새 탭이 원래 pane 에서 먼저 활성화돼 보고
  있던 터미널이 unmount 되고 ring buffer 를 전량 replay 하며 (b) 새 탭이 마운트 즉시 스폰을
  시작한 뒤 split 으로 재마운트돼 **셸을 두 번 스폰**하고(감사 §4-B A6/C14 의 고아 셸 패턴)
  (c) `open_tab` 의 kind 동등 dedupe 때문에 `session_id` 가 아직 비어 있는 터미널 탭이 있으면
  새 탭이 아예 생기지 않는다.
- **비활성 방향은 숨기지 않는다.** 탭 메뉴의 "조건 불일치 항목은 숨긴다" 관례(`tabs.md` §3.1)와
  반대인데, 방향이 사라지면 "이 앱은 아래로 못 나눈다"로 읽히고 흐려지면 "지금 크기로는 안
  된다"로 읽히기 때문이다(사용자 확정 전제).
- 새 터미널 = 같은 pane 에 터미널 탭 추가(`layout_open_tab`, 탭 바 `+`·팔레트와 동일 경로),
  종료 = `layout_close_tab`(pty 회수 포함, 확인 다이얼로그 없음 — 일반 탭 닫기와 동일).

**비활성 판정 (`widgets/terminal-pane/terminal-split-availability.ts`, 순수 함수 + `bun:test`).**

- `resolveSplitAvailability({ paneWidthPx, paneHeightPx, minPaneSizePx, resizerThicknessPx })` →
  `required = minPaneSizePx * 2 + resizerThicknessPx`, 좌/우는 폭·상/하는 높이로 판정.
  측정 실패(0)는 그 축을 막는다.
- `MIN_PANEL_SIZE_PX`(120)는 `pane-node-view.tsx` 에서 `shared/constants/layout.ts` 로 승격해
  렌더러와 이 판정이 같은 수를 읽는다. react-resizable-panels 는 픽셀 `minSize` 를 그룹 대비
  퍼센트로 환산한 뒤 **정규화**하므로, 너무 좁은 pane 의 분할을 거부하는 대신 조용히 뭉갠다 —
  그래서 앱이 사전에 걸러야 한다.
- 측정은 `PaneNodeView` 가 leaf **콘텐츠 박스**(탭 바 아래, 곧 터미널이 채우는 영역)에 단 ref 를
  `TerminalSession → TerminalPane` 으로 내려 **메뉴 열림 시 `getBoundingClientRect()` 1회**만 한다
  (ResizeObserver·리렌더 0). 콘텐츠 박스를 재기 때문에 탭 바 높이는 자동으로 빠지고 Zen 모드
  (탭 바 없음)도 별도 분기 없이 맞는다.

**클립보드 (`terminal-clipboard-availability.ts`, 순수 함수 + `bun:test`).**

- `resolveTerminalClipboardAvailability({ hasWriteText, hasReadText })` 로 복사/붙여넣기 가능
  여부를 따로 판정한다. async clipboard API 는 secure context 에만 노출되고 원격 미러는 LAN 평문
  HTTP(`remote-control.md`)라 `navigator.clipboard` 자체가 없을 수 있다 — 두 메서드 존재 확인이
  곧 secure context 판정이라 별도 `isSecureContext` 검사는 두지 않는다. 눌러도 아무 일이 없는
  항목 대신 비활성 항목을 보여 준다.
- 붙여넣기는 `writePty` 가 아니라 **`term.paste()`** 로 넣는다. CRLF 정규화와 bracketed paste
  (`\x1b[200~`)를 적용하는 쪽이 xterm 이라, 원문을 그대로 쓰면 여러 줄 붙여넣기가 줄마다 실행된다.

**입력·포커스.**

- Terminal 옵션 `rightClickSelectsWord: false` 를 명시한다. 기본값이 macOS 에서 `true` 라 우클릭이
  커서 아래 단어를 선택해 버려, "복사"의 대상이 사용자가 고른 범위와 달라진다.
- Radix `ContextMenuTrigger` 가 `contextmenu` 를 `preventDefault` 하므로 **보조 창의 네이티브
  메뉴도 함께 막힌다**(억제는 지금까지 메인 창 `app-shell.tsx` 에만 있었다).
- `ContextMenuContent onCloseAutoFocus` 에서 기본 동작을 막고 `term.focus()` 를 부른다 — 안 그러면
  메뉴가 닫힌 뒤 xterm textarea 가 blur 된 채로 남아 다음 키 입력이 어디에도 가지 않는다.
- 마우스 리포팅 모드(vim·tmux 등)에서도 메뉴는 항상 뜬다(VS Code 동일).

**cwd 상속.** 새 터미널은 `resolveSplitTerminalCwd({ liveCwd, persistedCwd, tabCwd, projectRoot })`
가 고른 cwd 로 스폰한다 — OSC 7 라이브 cwd 우선, 없으면 세션 cwd → 탭 cwd. 단 `pty_default_options`
가 넘겨받은 cwd 를 `ensure_within_root` 로 검증하므로 **루트 밖(`cd /tmp` 이후)이면 `null` 로
떨어뜨려 프로젝트 루트에서 연다**(Forbidden 스폰 실패 대신).

## 7. 폰트 크기 (FR-G3)

- 크기는 설정(settings 도메인)에 저장 — 전 터미널 공통, 변경 시 열린 터미널 전체 반영. 조절
  진입점은 **상태바(`status-bar-content.tsx`)와 설정 화면**(`settings-terminal-section.tsx`)이다.
  MIN 6(`shared/constants/terminal.ts`).
- **`⌘+`/`⌘-`/`⌘0` 커스텀 키 핸들러는 미구현이다**(2026-09-04 확인). `attachCustomKeyEventHandler`
  는 Shift+Enter → LF 변환만 처리한다(§8). 도입 시 폰트 대입 후 `fit.fit()` 을 반드시 부른다
  (안 부르면 커서 밀림 — 함정 16).
- 컨테이너 리사이즈: ResizeObserver + rAF + `proposeDimensions()` 비교 후 fit
  (숨김 컨테이너 NaN 가드 — 함정 11). 순서: xterm resize → pty resize(함정 12).

## 8. 검색·기타 (FR-G4)

- **검색 바는 미구현이다**(2026-09-04 확인). `SearchAddon` 은 로드만 돼 있고(`terminal-view.tsx`)
  `findNext`/검색 UI 참조가 0건이다. 도입 시 결과 카운트(`onDidChangeResults`)·overviewRuler
  하이라이트까지 함께 한다 — 백로그.
- 복사/붙여넣기: **우클릭 컨텍스트 메뉴**(§6.2)가 유일한 진입점이다. 선택 시 자동 복사 옵션·
  OSC52(clipboard addon)는 미구현이며 `@xterm/addon-clipboard` 는 설치돼 있지 않다 — 백로그.
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
  `pty_attach(sessionId, onData) → PtyAttachResult{subscriptionId, replayBytes}`(재생+재구독 —
  다중 구독자/멀티윈도우 지원. `replayBytes` 는 이 attach 가 라이브 출력보다 먼저 흘린 바이트 수로,
  flow control 집계에서 빼는 데 쓴다 — §2),
  `pty_detach(sessionId, subscriptionId)`(Wave I)
- **`pty_write` 는 호출 순서를 스스로 보장하지 않는다**(d-50 S4 에서 블로킹 풀로 이관된 뒤로 — 같은
  틱에 발사된 두 호출은 독립 블로킹 태스크로 writer 뮤텍스를 경쟁한다). 같은 세션의 순서는
  **프론트가** `entities/terminal/session-write-order.ts` 의 세션별 프라미스 체인으로 보장하며,
  `writePty` 가 그것을 통과한다. 대기 큐 flush(`terminal-write-bridge.ts`)처럼 한 틱에 N개를 연속
  발사하는 경로가 실제로 있으므로 필요한 계약이다.
- query: `shell_profiles`, `terminal_sessions(projectId)`, `resolve_terminal_path(path, cwd)`,
  `terminal_resolve_link_candidates(cwd, candidates) → (string | null)[]`(§6 — 데스크톱 링크 경로가
  실제로 쓰는 쪽. `resolve_terminal_path` 는 커맨드로 남아 있지만 프론트 호출자는 없다),
  `pty_default_options`
- event: `terminal:exited(sessionId, code)`, `terminal:cwd-changed(sessionId, cwd)`(X-A 배치
  (2026-08-19)에서 배선 완성 — 이 항목이 계획하던 "OSC7 은 view 파싱 → mutation 으로 Rust 에 보고"
  방향은 실제 구현과 다르다: `events.rs` 가 애초에 `terminal:cwd-changed` 를 `Event` derive 로
  선언해(Rust→view 단방향) mutation 대응 짝이 없었고, X-A 배치도 그 구조를 그대로 따라 **Rust 가
  OSC7 을 직접 파싱해 발행**한다 — `infra::shell_integration` 의 zsh/bash 훅이 매 프롬프트마다
  `\e]7;$PWD\e\\` 를 pty 출력에 실어 보내고, `infra::terminal_scan` 스캐너(§5.2)가 그 raw 바이트를
  읽어 `terminal::commands::pty_spawn` 의 `on_data` 콜백에서 이전 cwd 와 달라졌을 때만 이 이벤트를
  발행한다. 한 청크에 여러 번 실렸으면 **마지막 것만** 적용한다),
  `terminal:command-finished(sessionId, cwd, exitCode, durationMs)`(§5 —
  같은 스캐너가 같은 패스에서 OSC 133 `C`/`D` 를 읽어 세션별 `Instant` 로 실제
  경과 시간을 재고 발행한다. `C` 를 못 본 `D` 는 발행하지 않는다. 소비자는 메인 창의
  `native-notification-provider.tsx` 하나 — 10초 이상 걸린 명령만 OS 알림이 된다),
  `agent:state-changed`(`agent-integration.md`)

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
| pty spawn/기본 셸·Channel Raw+배칭·flow control·ring buffer 복원·리사이즈·폰트 크기(상태바·설정)·파일 링크(cmd+click)·**우클릭 컨텍스트 메뉴(복사/붙여넣기/모두 선택/지우기/분할/새 터미널/종료 — 사용성 배치 4, §6.2)**·셸 프로필 열거·**OSC 133 명령 블록(Wave E)**·**태스크 러너·Run Selected Text(Wave E, `tasks.md`)**·**OSC7 cwd 추적 기본 배선(X-A, 2026-08-19)**·**pty 출력 스캐너 — 단일 패스·청크 경계 이월·정규화 텍스트(d-54, §5.2)** | 검색 바 UI(SearchAddon 은 로드만 됨)·`⌘+`/`⌘-` 폰트 키·OSC52(clipboard addon)·블록 단위 복사(serialize range)·progress 뱃지·이미지/리거처·분할 내 터미널 다중화·serialize 스냅샷 내보내기·PowerShell rc 주입·fish 4.0 미만 폴백 |


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
`SessionOutput` 락으로 합쳐져 리플레이~등록이 원자적이다.
→ **바이트 단위 절단도 d-56(2026-09-07)에서 해소**됐다 — 축출이 개행 경계로 정렬되고 리플레이 앞에
SGR 리셋 프리앰블이 붙는다(§3). 폭 차이는 그대로 남는 한계다(위 3번 항목의 replay 경로 자체를
없애는 안).

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
