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
- UTF-8 은 어디서도 미리 문자열화하지 않고 bytes 그대로 `term.write(Uint8Array)`
  (청크 경계 멀티바이트 — xterm 이 처리).
- 상수(`HIGH_WATER` 등)는 `shared/lib/constants/terminal.ts` (research 값 채택).

## 3. 스크롤백 · 복원

- **Rust 가 세션별 출력 ring buffer(기본 2MB)를 보관**한다. view (재)마운트 시
  `pty_attach(sessionId)` 가 ring buffer 를 재생(replay)한 뒤 라이브 스트림을 잇는다 —
  view reload·탭 전환 복원의 근거(NFR-3).
- xterm `scrollback: 10_000`. 앱 재시작 시 스크롤백은 복원하지 않는다(`data-model.md` §1) —
  터미널 탭은 같은 cwd·셸로 새 세션 + "이전 세션" 안내.
- 백그라운드(비활성) 터미널 탭: xterm 인스턴스는 유지하되 WebGL addon 은 dispose(활성화 시 재로드)
  — GPU/rAF 낭비 방지(research §12).

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

## 5. 명령 블록 — OSC 133 (warp 스타일, 선택 기능)

- 주입: zsh ZDOTDIR 트릭 / bash `--init-file` / fish·PowerShell 은 2차(research §8 스니펫 채택).
  **opt-out 설정 + 수동 설치 안내 병행**(p10k instant prompt 충돌 사례 — 함정 15).
- 파싱: `parser.registerOscHandler(133, ...)` → `registerMarker` 로 블록 모델(A/B/C/D;exit).
  핸들러는 `false` 반환(기본 처리 전달). dead marker 주기 정리(함정 13).
- UX: 블록 경계 표시, exit code 배지, 이전/다음 명령 점프, 블록 단위 복사(serialize range).
- OSC 7 로 cwd 추적(파일 링크 해석·새 탭 cwd 계승·복원 cwd 에 사용).

## 6. 파일 링크 (FR-G2)

- URL 은 web-links addon, **파일 경로는 커스텀 `registerLinkProvider`** (research §10 구현 채택 —
  `path:line:col` 패턴, 1-based 좌표, wrapped line 처리).
- cmd(ctrl)+click → Rust `resolve_terminal_path(path, cwd)` (OSC7 cwd 기준 존재 검증) →
  해당 프로젝트 새 탭으로 열기 + line/col 로 커서 이동. modifier 없는 클릭은 무동작(터미널 관례).
- hover 툴팁 DOM 은 `term.element` 내 `xterm-hover` 클래스(이벤트 관통 방지).

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

## 9. IPC

- mutation: `pty_spawn(opts, onData: Channel, onExit: Channel) → sessionId`,
  `pty_write(sessionId, data)`, `pty_resize(sessionId, cols, rows)`, `pty_kill(sessionId)`,
  `pty_set_paused(sessionId, paused)`, `pty_attach(sessionId, onData) `(재생+재구독)
- query: `shell_profiles`, `terminal_sessions(projectId)`, `resolve_terminal_path(path, cwd)`
- event: `terminal:exited(sessionId, code)`, `terminal:cwd-changed(sessionId, cwd)`(OSC7 은 view 파싱
  → mutation 으로 Rust 에 보고), `agent:state-changed`(`agent-integration.md`)

## 10. 수명주기 · 누수 방지

- 탭 닫기: 포그라운드 자식 프로세스 실행 중이면 확인 다이얼로그 → Rust `pty_kill`
  (killer 는 `clone_killer` 사본 — wait 스레드와 분리, Mutex+wait 데드락 금지 — 함정 20) →
  ring buffer 해제. view 는 xterm `dispose`.
- 프로세스 종료(exit) 감지: try_wait 폴링 또는 wait 스레드 → `terminal:exited` → 탭에
  "[process exited]" 표시 + 재시작 버튼.
- view unmount(탭 전환): xterm dispose 하지 않고 DOM 분리 유지(활성 pane 내 다중 터미널 전환 시).
  프로젝트 전환으로 위젯 트리가 내려가면 xterm dispose — 재마운트 시 ring buffer 재생.
- 앱 종료: 전 세션 kill(자식 프로세스 잔존 금지 — Drop + 명시 shutdown 이중화).

## 11. 범위

| 1차 | 2차 |
|-----|-----|
| pty spawn/기본 셸·Channel Raw+배칭·flow control·ring buffer 복원·리사이즈·폰트 크기·파일 링크(cmd+click)·검색·복사/붙여넣기·셸 프로필 열거 | OSC 133 블록 UX·OSC7 cwd 추적 고도화·progress 뱃지·이미지/리거처·분할 내 터미널 다중화·serialize 스냅샷 내보내기 |


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

### 12.3 반증된 가설 (조사해서 아니라고 확인)

- 배칭이 ANSI 시퀀스를 쪼갠다 → **아니다.** xterm 파서가 `_preserveStack` 로 write 경계를 넘어
  상태를 보존한다.
- flow control 이 데이터를 버린다 → **아니다.** `PauseGate` 가 `read()` **이전에** 블록한다.
- DEC 2026 synchronized output / alt screen 미지원 → **아니다.** typings 에 `synchronizedOutputMode` 존재.
- WebGL 렌더러 잔상 → **미확인, 가능성 낮음.**

### 12.4 수정안

**P0 (잔상 해결)**
1. reader 에 **타이머 flush** 도입 — 별도 flusher 스레드 5ms 틱. (A)
2. **실측 cols/rows 로 spawn** + 셸 준비 후 재-resize 안전망. (B)
3. 탭 전환 시 unmount 대신 **숨김 유지**로 replay 경로 자체를 제거. (C)

**P1**
4. `pty_write`/`pty_resize`/`pty_set_paused` 를 전역 `begin_mutation` 락에서 **분리**.
   현재 키 입력이 파일 저장·git 작업 뒤에 줄을 선다(`state.rs` mutation guard).

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
