# 터미널 구현 재평가 — xterm.js 존속 판단과 잔상 버그 원인 규명

조사일: 2026-08-06 / 조사 방식: TAIDE 저장소 소스 통독, 로컬 `node_modules/@xterm/xterm/typings/xterm.d.ts` 원본,
npm registry API 실측, GitHub API·Issues 원문, microsoft/vscode `main` 브랜치 소스 원문, xterm.js 공식 가이드.
"미확인" 표기가 없는 서술은 위 1차 출처에서 실제로 확인한 내용이다.

선행 문서: `docs/research/xterm-pty.md`(API·버전 확정), `docs/features/terminal.md`(설계), ADR-0005.
이 문서는 그 위에 **"현재 구현이 실제로 무엇을 잘못하고 있는가"** 를 얹는다.

---

## 0. 결론 먼저

### 0.1 xterm.js 를 계속 쓸 것인가

**계속 쓴다. 교체하지 않는다.** (권고 1개)

근거는 3.과 4.에 상세히 있고, 요약하면:

1. **웹뷰에 임베드 가능한 성숙한 터미널 에뮬레이터는 2026-08 현재 xterm.js 가 사실상 유일**하다.
   Ghostty·WezTerm·Alacritty·Rio 는 전부 네이티브 애플리케이션/네이티브 라이브러리이며 Tauri 웹뷰 안에서
   렌더링할 수 없다. "터미널 에뮬레이터"와 "웹 임베드 가능한 라이브러리"는 다른 범주다.
2. **xterm.js 는 VS Code 팀이 주 개발 주체**다. VS Code 는 `@xterm/xterm@^6.1.0-beta.292` 를 직접 의존한다
   (microsoft/vscode `main` 의 `package.json` 실측). 즉 "VS Code 는 어떻게 했나"의 답은
   "xterm.js 를 쓰되 그 주변 파이프라인을 다르게 설계했다"이다.
3. **사용자가 겪은 증상은 xterm.js 의 버그가 아니라 TAIDE 파이프라인의 결함**으로 판정된다(2. 참조).
   라이브러리를 바꿔도 동일 증상이 재현될 구조다.

### 0.2 사용자가 보고한 증상의 가장 유력한 원인

> "zsh-autocomplete 로 auto complete 한 내용이 백에서 남는다"
> = 회색 제안 텍스트가 지워지지 않고 화면에 잔류한다.

**원인은 하나가 아니라 세 개가 겹쳐 있다.** 유력도 순으로:

| # | 원인 | 상태 | 위치 |
|---|---|---|---|
| **A** | Rust 배칭 루프가 **버스트의 마지막 청크를 무기한 붙잡는다** | **확인됨(코드)** | `src-tauri/src/infra/pty.rs:148-167` |
| **B** | PTY 를 **80x24 로 고정 spawn** 하고 뒤늦게 resize → 셸이 잘못된 폭으로 동작 | **확인됨(코드)** | `src-tauri/src/domain/terminal/commands.rs` `DEFAULT_TERMINAL_COLS/ROWS` + `src/widgets/terminal-pane/terminal-session.tsx:36-38` |
| **C** | 탭 전환마다 xterm 을 **파괴 후 원시 바이트 ring buffer 를 재생**(+ 재생 중 데이터 유실 레이스) | **확인됨(코드)** | `src/widgets/editor-area/pane-node-view.tsx:83`, `commands.rs` `pty_attach` |

A 가 단독으로 이 증상을 정확히 설명한다. **zsh-autocomplete 는 제안을 그릴 때와 지울 때 모두
작은 ANSI 시퀀스 조각(수십~수백 바이트)을 뒤늦게 내보낸다.** 우리 배칭 루프는 그 마지막 조각을
"다음 read 가 반환될 때"까지 들고 있다가, 다음 키 입력이 있어야만 내보낸다. 즉
**제안을 지우는 시퀀스가 화면에 도달하지 않아 회색 텍스트가 남는다.** 사용자가 다음 글자를 치면
그때서야 밀린 조각이 함께 도착하므로 "가끔 사라졌다가 또 남는" 산발적 증상으로 보인다.

B 는 잘못된 줄바꿈 위치·제안이 엉뚱한 줄에 그려지는 증상을 만든다(VS Code #121891 과 동일 증상군).
C 는 탭을 갔다 오면 화면이 어긋나는 증상을 만든다.

### 0.3 구체적 수정안 (요약)

P0-1. Rust reader 배칭에 **타이머 flush** 를 넣는다 — read 가 블록돼 있어도 5ms 마다 잔여 배치를 내보낸다.
  (VS Code `TerminalDataBufferer` 의 `throttleBy = 5` 와 동일한 사상. 3.2 참조)
P0-2. spawn 시 **실측 cols/rows** 를 넘긴다 — xterm 을 먼저 마운트·fit 한 뒤 그 값으로 PTY 를 연다.
  추가로 셸 준비 후 1회 재-resize 안전망을 둔다.
P0-3. 탭 전환 시 **xterm 인스턴스를 파괴하지 않는다**(비활성 탭은 unmount 대신 숨김). replay 경로 자체를 제거한다.
P1. `pty_write`/`pty_resize`/`pty_set_paused` 를 **전역 mutation lock 에서 분리**한다.

상세와 근거는 5.에.

---

## 1. 현재 구현 요약 (재평가 대상)

| 계층 | 구현 | 파일 |
|---|---|---|
| 렌더 | `@xterm/xterm@6.0.0` + fit/webgl/search/unicode11/web-links | `src/features/terminal/terminal-view.tsx` |
| flow control | `term.write(bytes, cb)` 미완료 바이트 집계, HIGH 512KB / LOW 64KB | `src/widgets/terminal-pane/terminal-flow-control.ts` |
| 세션 배선 | spawn → attach 분리, Channel `Raw` 전송 | `src/entities/terminal/terminal.ipc.ts`, `terminal-session.tsx` |
| PTY | `portable-pty 0.9`, 64KB/4ms 배칭, `Condvar` pause | `src-tauri/src/infra/pty.rs` |
| 세션 저장 | 2MB 원시 바이트 ring buffer + `pty_attach` 재생 | `src-tauri/src/domain/terminal/commands.rs` |

설치 버전 실측: `@xterm/xterm 6.0.0`, addon fit 0.11.0 / search 0.16.0 / unicode11 0.9.0 / web-links 0.12.0 / webgl 0.19.0,
`portable-pty 0.9`, `tauri 2`.

---

## 2. 증상 원인 가설 검증

각 가설에 대해 **확인됨 / 반증됨 / 미확인** 을 명시한다.

### 2.1 [확인됨 — 최유력] Rust 배칭 루프가 버스트의 마지막 청크를 붙잡는다

`src-tauri/src/infra/pty.rs:148-167` 의 reader 루프:

```rust
loop {
    reader_pause.wait_while_paused();
    match reader.read(&mut buf) {          // ← 블로킹 read
        Ok(0) | Err(_) => break,
        Ok(n) => {
            batch.extend_from_slice(&buf[..n]);
            if should_flush(batch.len(), last_flush.elapsed()) {   // ← flush 판정이 read 직후에만 존재
                on_data(&batch);
                batch.clear();
                last_flush = Instant::now();
            }
        }
    }
}
```

`should_flush` 는 `batch.len() >= 64KB || elapsed >= 4ms` 다. **문제는 이 판정이 "새 데이터가 도착했을 때"만
평가된다는 것이다.** 시나리오:

1. `t=0ms` 큰 청크 도착 → flush, `last_flush = 0ms`.
2. `t=1ms` 셸이 제안 텍스트를 지우는 120바이트 조각을 내보냄 → `batch.len()=120`, `elapsed=1ms` → **flush 안 함**.
3. 셸이 유휴 상태로 진입 → `reader.read()` 가 **블록**된다.
4. 그 120바이트는 **사용자가 다음 키를 누를 때까지 Rust 메모리에 갇힌다.**

결과: 화면에는 2단계 직전의 상태 — 회색 제안이 그려진 상태 — 가 그대로 남는다. 이것이
"auto complete 한 내용이 백에서 남는다"의 기계적 설명이다.

이 결함은 **출력이 잦은 명령(빌드 로그 등)에서는 드러나지 않고, 인터랙티브 프롬프트에서만 드러난다.**
버스트 사이에 유휴 구간이 있어야 발현하기 때문이다. zsh-autocomplete·zsh-autosuggestions·fish 처럼
"매 키 입력마다 소량의 재그리기 시퀀스를 흘리는" 플러그인이 정확히 그 조건을 만든다.

**대조군(VS Code)**: `src/vs/platform/terminal/common/terminalDataBuffering.ts` 의 `TerminalDataBufferer` 는
첫 데이터가 도착한 시점에 `setTimeout(() => this.flushBuffer(id), throttleBy)` 를 걸고(기본 `throttleBy = 5`),
그 타임아웃이 만료되면 **데이터가 더 오든 말든 무조건 flush** 한다. 즉 VS Code 구조에서는 잔여 조각이 갇힐 수 없다.

### 2.2 [확인됨] PTY 를 80x24 로 고정 spawn 한다 (셸이 잘못된 폭으로 동작)

`src-tauri/src/domain/terminal/commands.rs`:

```rust
const DEFAULT_TERMINAL_COLS: u16 = 80;
const DEFAULT_TERMINAL_ROWS: u16 = 24;
// pty_default_options 가 항상 이 값을 반환
```

`src/widgets/terminal-pane/terminal-session.tsx:36-38` 는 **xterm 이 마운트되기 전에** spawn 한다
(`sessionId` 이 없는 동안 `TerminalPane` 은 렌더되지 않고 placeholder 만 그린다). 따라서 셸은 항상 80x24 로 시작하고,
그 뒤 `TerminalView` 가 `fit()` 하며 `term.onResize` → `pty_resize` 로 뒤늦게 크기를 알린다.

여기서 두 가지가 깨진다.

1. **초기 출력(로그인 셸 초기화, 첫 프롬프트, p10k instant prompt 등)이 80칼럼 기준으로 랩된다.**
   VS Code 도 같은 계열의 오래된 이슈를 가지고 있다 — microsoft/vscode#98399
   "Initial output in terminal wraps at 80 columns instead of screen width".
2. **SIGWINCH 레이스**: 셸이 아직 `exec` 중이거나 zle 를 초기화하기 전에 resize 가 도착하면 그 신호는 사실상 유실된다.
   이 경우 셸의 `COLUMNS` 는 80 으로 굳고, xterm 은 실제로 120칼럼이다.
   VS Code#121891 이 정확히 이 상태를 기술한다 — `tput cols` 가 80 을 반환하고,
   증상은 **"autocomplete suggestions appearing on incorrect lines or overwriting typed text"** 다.
   사용자 보고 문구와 사실상 같은 증상이다.

**xterm.js 쪽 기본값이 이 상황을 구제해 주지 않는다.** 로컬 typings 실측:

```ts
/**
 * Whether to reflow the line containing the cursor when the terminal is
 * resized. Defaults to false, because shells usually handle this
 * themselves.
 */
reflowCursorLine?: boolean;
```

즉 xterm.js 는 **"커서가 있는 줄(= 프롬프트 + 제안 텍스트가 그려진 줄)의 리플로는 셸이 알아서 한다"**
는 전제로 기본 off 다. 셸이 SIGWINCH 를 놓치면 그 전제가 깨지고, 커서 줄에 잔상이 남는다.
(관련: xtermjs/xterm.js#5213 — 이 옵션이 추가된 이슈)

동일 진단이 다른 Tauri+xterm+portable-pty 프로젝트에서도 나왔다: hermes-hq/hermes-ide#113 의 Root Cause 1 이
**"PTY initializes with hardcoded 80x24, ignoring actual viewport size"** 이고, 부수 원인으로
"SIGWINCH race condition during shell startup" 을 든다. 우리 코드와 형태가 동일하다.

### 2.3 [확인됨] 탭 전환마다 xterm 파괴 + 원시 바이트 replay (+ 유실 레이스)

`src/widgets/editor-area/pane-node-view.tsx:83` 은 **활성 탭만 렌더**한다.

```tsx
{activeTab?.kind.kind === 'terminal' && (
    <TerminalSession key={activeTab.id} ... />
)}
```

따라서 탭을 옮기면 `TerminalView` 의 cleanup 이 `term.dispose()` 를 부르고, 돌아오면 새 `Terminal` 이 생성된다.
그때 `pty_attach` 가 Rust ring buffer(2MB 원시 바이트)를 통째로 재생한다.

이 설계에는 세 가지 결함이 있다.

**(a) 서로 다른 폭에서 생성된 바이트를 하나의 폭으로 재생한다.** ring buffer 에는 80칼럼 시절의 초기 출력과
120칼럼 시절의 출력이 섞여 있다. 재생 시점의 폭 하나로 전부 해석되므로 줄바꿈이 원리적으로 어긋난다.
(xterm.js 는 절대 커서 이동·랩 상태를 바이트에서 복원할 뿐, "이 바이트는 80칼럼용" 이라는 정보를 갖지 않는다.)

**(b) ring buffer 절단이 시퀀스 중간을 자른다.** `service::ring_buffer_append` 는

```rust
buffer.drain(0..overflow);
```

로 앞을 바이트 단위로 잘라낸다. 2MB 를 넘긴 순간부터 replay 의 **첫 바이트가 ESC 시퀀스 한가운데**일 수 있고,
xterm 파서는 그 잔여 시퀀스를 쓰레기 문자로 처리하거나 상태를 오염시킨다. 또 replay 전에 `RIS`/`ED` 로
터미널을 초기화하지 않으므로 파서 상태(모드·SGR·alt buffer 여부)도 재생 시작점과 어긋난다.

**(c) replay 와 구독자 교체 사이에 데이터 유실 레이스가 있다.** `pty_attach`:

```rust
let snapshot = entry.ring_buffer.lock().clone();
if !snapshot.is_empty() { let _ = on_data.send(...); }
*entry.subscriber.lock() = Some(on_data);
```

reader 콜백은 `ring_buffer` 잠금과 `subscriber` 잠금을 **따로** 잡는다. 그래서
`snapshot` 을 복제한 직후 ~ `subscriber` 를 교체하기 전에 도착한 청크는
**ring 에는 들어가지만 스냅샷에는 없고, 옛 구독자(spawn 시 넘긴 `() => undefined` 채널)로 나가서 버려진다.**
즉 **그 청크는 화면에 영원히 도달하지 않는다.** 유실되는 것이 하필 "제안을 지우는 짧은 시퀀스"라면
증상은 정확히 잔상이다. 반대 순서면 같은 바이트가 두 번 그려진다.

### 2.4 [반증됨] 64KB/4ms 배칭이 ANSI 이스케이프 시퀀스를 쪼개서 렌더가 깨진다

**아니다.** xterm.js 의 `EscapeSequenceParser` 는 `_preserveStack` 로 파서 상태(현재 state, 활성 핸들러,
핸들러 위치, transition, chunk 위치)를 `parse` 호출 경계 너머로 보존한다. 시퀀스가 청크 경계에서 잘려도
다음 `write` 에서 정확히 이어서 처리된다.

UTF-8 도 같다. 로컬 typings 원문:

> The data to write to the terminal. This can either be raw bytes given as Uint8Array from the pty or a string.
> **Raw bytes will always be treated as UTF-8 encoded**, string data as UTF-16.

우리는 `InvokeResponseBody::Raw` → `Uint8Array` → `term.write(bytes)` 경로를 지키고 있으므로
멀티바이트 경계도 안전하다. **이 가설은 기각한다.** (단 2.3(b) 의 ring buffer 절단은 별개 문제다 —
그건 "파서가 처음부터 잘린 스트림을 받는" 경우라 파서 상태 보존과 무관하다.)

### 2.5 [반증됨] flow control 이 pause 중 데이터를 버린다

**버리지 않는다.** `PauseGate::wait_while_paused()` 는 `read()` **이전에** 블록한다. 이미 읽은 바이트는
`batch` 에 남아 있고, PTY 커널 버퍼가 차면 자식 프로세스가 자연스럽게 블록된다. 데이터 유실 경로가 없다.

다만 두 가지를 부기한다.

- pause 상태에서는 2.1 의 배칭 결함이 더 오래 유지된다(잔여 배치가 resume 전까지 갇힘).
- xterm.js 공식 flow control 가이드는 **내부 입력 버퍼 50MB 하드 한계**를 명시하고 그 이상은 폐기된다고 밝힌다.
  우리 HIGH_WATER 는 512KB 라 한참 아래이므로 이 경로로는 문제가 없다. 또 같은 가이드가
  "HIGH 는 500K 를 넘기지 말 것" 을 권하는데 우리 값(512KB)은 그 경계에 정확히 걸쳐 있다 — 낮출 여지가 있다.

### 2.6 [미확인 · 낮음] WebGL 렌더러 자체의 잔상 버그

`@xterm/addon-webgl` 에 알려진 렌더링 이슈는 존재한다 — 리거처와의 조합(xtermjs#3303),
macOS 26.5 beta Safari 에서의 렌더 붕괴(xtermjs#5816) 등. 다만:

- 우리는 리거처 addon 을 쓰지 않는다.
- resize 시 WebGL 렌더러의 `handleResize()` 가 모델을 비우고 전체 뷰포트를 다시 그리는 것으로 보고돼 있다
  (hermes-ide#113 의 조사 결론). 즉 resize 후 잔상의 주범은 렌더러가 아니라 버퍼 내용이다.
- v6 에는 **canvas 폴백이 없다**(`@xterm/addon-canvas` 는 v6 미대응). WebGL 실패 시 남는 폴백은 DOM 뿐이다.

**판정: 이번 증상의 원인으로 보기 어렵다(미확인, 우선순위 낮음).** 다만 진단 시 1회 대조는 해볼 값어치가 있다 —
WebGL addon 을 끄고(=DOM 렌더러) 동일 증상이 재현되면 렌더러 무관이 확정된다. 이 대조가 A/B/C 판정을 가장 싸게 가른다.

### 2.7 [해당 없음] alternate screen / synchronized output 미지원

둘 다 지원한다. alt buffer 는 xterm.js 초창기부터 지원하고, DEC mode 2026(synchronized output)은 v6 에서 추가됐다.
로컬 typings 실측으로 확인:

```ts
readonly synchronizedOutputMode: boolean;   // IModes
```

**이 가설은 기각한다.**

### 2.8 [확인됨 · 부수 원인] 전역 mutation lock 이 터미널 입력·resize·resume 을 직렬화한다

`src-tauri/src/domain/terminal/commands.rs` 의 `pty_write`, `pty_resize`, `pty_set_paused`,
`pty_attach` 는 전부 첫 줄에서

```rust
let _guard = state.begin_mutation().await;
```

를 잡는다. `AppState::begin_mutation` 은 앱 전역 `tokio::sync::Mutex<()>` 다(`src-tauri/src/state.rs:39`).
따라서 **파일 저장·git 작업 등 무관한 mutation 이 길어지면 키 입력·resize·flow control resume 이 그 뒤에 줄을 선다.**

이것 자체가 잔상을 만들지는 않지만, 2.2 의 SIGWINCH 레이스 창을 넓히고(resize 가 늦게 전달됨),
2.1 의 갇힌 배치가 풀리는 시점을 더 늦춘다. 터미널 경로는 상태 정합성 보호가 필요한 mutation 이 아니다.

### 2.9 검증 매트릭스 요약

| 가설 | 판정 | 근거 |
|---|---|---|
| Rust 배칭이 마지막 청크를 붙잡음 | **확인됨** | `pty.rs` 루프 구조. VS Code 는 타이머 flush(대조) |
| 80x24 고정 spawn + SIGWINCH 레이스 | **확인됨** | `commands.rs` 상수 + `terminal-session.tsx` 순서. vscode#98399/#121891, hermes#113 |
| 탭 전환 시 파괴 + 원시 replay | **확인됨** | `pane-node-view.tsx:83`, `pty_attach` |
| replay/구독자 교체 유실 레이스 | **확인됨** | `pty_attach` 의 두 잠금 분리 |
| 전역 mutation lock 직렬화 | **확인됨** | `state.rs:39` |
| 배칭이 ESC 시퀀스를 쪼갬 | **반증됨** | `EscapeSequenceParser._preserveStack`, typings write() 문서 |
| flow control 이 데이터 폐기 | **반증됨** | `PauseGate` 가 read 이전에 블록 |
| WebGL 렌더러 잔상 | **미확인(낮음)** | 알려진 이슈는 리거처/Safari 한정 |
| alt screen / DEC 2026 미지원 | **해당 없음** | typings `synchronizedOutputMode` |
| xterm.js 리사이즈 리플로 결함 | **부분 해당** | `reflowCursorLine` 기본 false — 단 이는 "셸이 처리한다"는 전제, 전제를 깬 건 우리 |

---

## 3. VS Code 는 무엇을 다르게 하는가

xterm.js 는 VS Code 팀(Daniel Imms 등)이 주 기여자다. 따라서 "VS Code 를 참고한다"는 것은
**다른 라이브러리를 쓰는 법이 아니라, 같은 라이브러리를 감싸는 파이프라인 설계를 배우는 것**이다.

### 3.1 버전 고정 정책 — beta 라인을 쓴다

microsoft/vscode `main` 의 `package.json` 실측(2026-08-06):

```
"@xterm/xterm":            "^6.1.0-beta.292"
"@xterm/headless":         "^6.1.0-beta.292"
"@xterm/addon-webgl":      "^0.20.0-beta.291"
"@xterm/addon-search":     "^0.17.0-beta.292"
"@xterm/addon-serialize":  "^0.15.0-beta.292"
"@xterm/addon-unicode11":  "^0.10.0-beta.292"
"@xterm/addon-clipboard":  "^0.3.0-beta.292"
"@xterm/addon-image":      "^0.10.0-beta.292"
"@xterm/addon-ligatures":  "^0.11.0-beta.292"
"@xterm/addon-progress":   "^0.3.0-beta.292"
"node-pty":                "^1.2.0-beta.15"
```

npm registry 실측: `@xterm/xterm` `latest = 6.0.0`(2025-12-22), `beta = 6.1.0-beta.292`(2026-07-27).

**VS Code 는 stable 태그를 쓰지 않는다.** 자기들이 만드는 라이브러리이므로 beta 를 그대로 물고 간다.
TAIDE 는 6.0.0 stable 을 쓰고 있는데, 이는 VS Code 가 검증하는 라인보다 약 7개월 뒤다.
안정성 관점에서는 stable 유지가 맞지만, **"VS Code 에서 고쳐진 버그가 6.0.0 에는 없을 수 있다"**
는 점은 인지해야 한다(6.1 릴리스 시 갱신 대상).

### 3.2 데이터 배칭 — 타이머 기반, 절대 갇히지 않는다

`src/vs/platform/terminal/common/terminalDataBuffering.ts` 의 `TerminalDataBufferer`:

- 첫 데이터 이벤트에서 버퍼를 만들고 **동시에** `setTimeout(() => this.flushBuffer(id), throttleBy)` 를 건다.
- 이후 데이터는 배열에 push 만 한다.
- 타임아웃 만료 시 `buffer.data.join('')` 로 합쳐 콜백에 넘긴다.
- 기본 `throttleBy = 5`(ms). `stopBuffering()`/dispose 에서도 즉시 flush.

**핵심 차이: flush 트리거가 "다음 데이터 도착"이 아니라 "시간"이다.** 우리 구현의 2.1 결함이 여기엔 없다.

### 3.3 flow control — ack 기반 + ptyHost 아키텍처

- PTY 는 **별도 프로세스(ptyHost)** 에서 돌아간다. 창을 닫아도 세션이 살아남는 persistent terminal 의 기반이다.
  (도입 이슈: microsoft/vscode#74620 "Create a node-pty host process with flow control and event batching")
- `src/vs/platform/terminal/node/terminalProcess.ts` 실측:

```ts
if (!this._isPtyPaused && this._unacknowledgedCharCount > FlowControlConstants.HighWatermarkChars) {
    this._isPtyPaused = true;
    ptyProcess.pause();
}
...
if (this._isPtyPaused && this._unacknowledgedCharCount < FlowControlConstants.LowWatermarkChars) {
    this._ptyProcess?.resume();
    this._isPtyPaused = false;
}
```

- 렌더러는 `acknowledgeDataEvent(charCount)` 로 **처리 완료한 문자 수를 되돌려 보고**한다
  (`terminalProcessManager.ts`). ack 자체도 `AckDataBufferer` 가 `FlowControlConstants.CharCountAckSize`
  단위로 묶어 IPC 횟수를 줄인다.
- 종료 시 `ShutdownConstants.DataFlushTimeout = 250` ms 동안 잔여 데이터를 기다린 뒤 exit 이벤트를 보낸다.

**우리와의 차이**: 우리는 "미완료 바이트 총량"을 프론트가 직접 들고 pause/resume 명령을 쏜다. 방향성은 동일하고
(watermark 모델), 설계적으로 열등하지 않다. 다만 **ack 를 배칭하지 않아 IPC 왕복이 잦고**,
pause/resume 명령이 전역 mutation lock 을 통과한다(2.8). 그리고 `pty_write`(입력)에도 배칭이 없다.

### 3.4 렌더러 선택 정책 — auto = WebGL, 실패 시 DOM

- 설정 키는 `terminal.integrated.gpuAcceleration`, 기본값 `"auto"`.
- `auto` 는 **WebGL 을 먼저 시도하고 실패하면 DOM 으로 폴백**한다.
- 과거(`rendererType` 시절)에는 WebGL 실패 → canvas → "첫 20프레임이 느리면" DOM 이라는 3단 폴백이었으나,
  `@xterm/addon-canvas` 가 v6 에서 제거되면서 **오늘날 실질 폴백은 DOM 하나**다
  (cockpit-project/cockpit#22509 이 그 여파를 기록).
- 트러블슈팅 지침은 `--disable-gpu` 또는 `gpuAcceleration: "off"`.

**TAIDE 시사점**: 우리는 WebGL 을 무조건 로드하고 `onContextLoss` 만 처리한다.
**초기 로드 실패(webgl2 미지원·드라이버 블랙리스트)에 대한 try/catch 폴백과 사용자 설정이 없다.**
VS Code 와 동급으로 만들려면 "auto | off" 설정 + 로드 실패 시 DOM 유지가 필요하다.

### 3.5 리사이즈·리플로 처리

- 프로세스 생성 시 **cols/rows 를 인자로 받는다** — `createProcess(shellLaunchConfig, cols, rows, reset)`
  (`terminalProcessManager.ts` 실측). 즉 호출자(xterm 뷰)가 실제 치수를 알고 나서 프로세스를 만든다.
  우리처럼 80x24 로 먼저 열고 나중에 고치는 구조가 아니다.
- xterm.js 의 `reflowCursorLine` 은 기본 false 이며, 문서화된 이유는 "shells usually handle this themselves" 다.
  VS Code 도 이 기본값에 의존한다 — **그 전제가 성립하려면 셸이 올바른 폭을 알고 있어야 한다.**
- Windows 는 ConPTY 특성 때문에 `windowsPty: { backend, buildNumber }` 를 xterm 에 알려야 리플로가 정상 동작한다
  (선행 문서 `xterm-pty.md` 함정 10). 우리는 아직 설정하지 않는다.

### 3.6 shell integration(OSC 133 / 633)과 자동완성의 관계

이 부분이 이번 조사에서 가장 전략적으로 중요한 발견이다.

- VS Code 는 OSC 133 을 확장한 **OSC 633** 을 쓴다. `A`(prompt start) / `B`(prompt end) / `C`(pre-exec) /
  `D[;exit]`(exec end) 에 더해,
  - `OSC 633 ; E ; <commandline> [; <nonce>] ST` — **셸이 해석한 명령줄 원문을 그대로 전달**
  - `OSC 633 ; P ; <Property>=<Value> ST` — `Cwd` 등 속성 보고
  - 시퀀스가 A→B→E→C→D 순서로 이상적으로 오면 `HasRichCommandDetection` 이 참이 된다.
- **VS Code 의 터미널 IntelliSense(suggest widget)는 셸의 회색 제안 텍스트에 의존하지 않는다.**
  OSC 633 으로 "지금 입력 중인 명령줄"과 cwd 를 정확히 알아낸 뒤, **VS Code 가 자기 UI 위젯을 직접 그린다.**

**시사점**: zsh-autocomplete 처럼 셸이 터미널 화면에 직접 그리는 방식의 자동완성은
터미널 에뮬레이터와 재그리기 경합을 벌이는 구조라 근본적으로 취약하다(폭 불일치·잔여 시퀀스에 그대로 노출).
TAIDE 가 장기적으로 IDE 다운 자동완성을 원한다면 **OSC 133/633 기반 자체 suggest UI** 가 정답이고,
그 경우 셸측 자동완성 플러그인 의존을 줄이라고 안내할 수 있다.
(다만 이번 증상의 즉효 처방은 아니다 — 사용자의 zsh-autocomplete 는 계속 동작해야 하므로 2.1~2.3 수정이 선행이다.)

---

## 4. 대안 라이브러리 실사 (2026-08-06 기준)

판정 기준을 명확히 한다: **Tauri 웹뷰(WKWebView/WebView2/WebKitGTK) 안에서 React 트리에 임베드 가능한가.**
"좋은 터미널 에뮬레이터인가"가 아니다.

| 후보 | 형태 | 웹 임베드 | 실측 상태 | 판정 |
|---|---|---|---|---|
| **`@xterm/xterm` 6.x** | JS/TS 라이브러리 | 가능(유일한 성숙 선택) | latest 6.0.0(2025-12-22), beta 6.1.0-beta.292(2026-07-27), star 21,020, open issue 162, 최근 push 2026-08-05 | **채택 유지** |
| `@wterm/ghostty` / `@wterm/core` | libghostty-vt WASM(~400KB) 코어. xterm.js 호환 API 표방 | 가능 | npm 최초 발행 2026-04-30, 현재 0.3.2(2026-08-02), 발행 버전 **3개**(ghostty) / 15개(core) | **시기상조.** 감시만 |
| Ghostty (본체) | 네이티브 앱 + libghostty C API(Zig) | 불가(웹 타깃은 계획/실험 단계) | libghostty-vt 의 wasm 단독 컴파일 데모 존재 | 불가 |
| WezTerm / Alacritty / Rio | 네이티브 터미널 | 불가 | — | 불가 |
| hterm (chromium/libapps) | JS 라이브러리 | 가능 | npm `hterm` 은 2015 년 이후 정지(2.0.2). 실제 배포는 libapps 트리 | 기능·생태계 열세, 채택 불가 |
| `xterm.es` | xterm.js 의 커뮤니티 ESM 포크 | 가능 | 상류 추종 리스크 | 불필요(v6 가 이미 ESM 지원) |

부연:

- **`@wterm/ghostty` 는 진짜로 흥미로운 후보다.** libghostty-vt(= Ghostty 본체와 동일한 VT 파서)를 WASM 으로
  올려 브라우저에서 돌리고, 유니코드 grapheme cluster·SGR 전체·터미널 모드를 지원한다고 표방한다.
  다만 **npm 최초 발행이 2026-04-30, 발행 버전이 3개, 버전 번호가 0.3.x** 다.
  IDE 의 핵심 기능을 얹을 성숙도가 아니다. **6~12개월 뒤 재평가 대상으로 기록**해 둔다.
- 웹 기반 터미널 제품군(ttyd, WeTTY, WebSSH2, electerm, Wave Terminal 등)은 **전부 xterm.js 를 쓴다.**
  즉 "xterm.js 를 대체한 성공 사례"가 존재하지 않는다.

**결론: 교체 대상이 없다. xterm.js 유지가 유일한 합리적 선택이다.**

---

## 5. TAIDE 적용 가이드

### 5.1 P0 — 잔상 증상 직결 수정 (이 순서로)

**P0-1. reader 배칭에 타이머 flush 를 넣는다** (2.1 해소, 최우선)

- 목표: `read()` 가 블록돼 있어도 잔여 배치가 **최대 4~5ms 안에** 반드시 프론트로 나가야 한다.
- 방식(택1, 근거와 함께 결정 필요):
  - (a) reader 스레드는 읽자마자 공유 `Mutex<Vec<u8>>` 에 append 만 하고, **별도 flusher 스레드**가
    5ms 틱으로 비어 있지 않으면 flush. 구조가 단순하고 VS Code `TerminalDataBufferer` 와 사상이 같다. **권장.**
  - (b) reader 스레드 → `mpsc::channel` → 소비자 스레드가 `recv_timeout(5ms)` 로 모으고 타임아웃 시 flush.
- 상수: `OUTPUT_BATCH_MS` 를 4 → **5** 로 맞춰 VS Code `throttleBy` 와 정렬해도 되고 4 를 유지해도 된다.
  중요한 건 값이 아니라 **타이머가 존재한다는 사실**이다.
- pause 중에는 flush 하지 않는다(backpressure 의미가 사라짐). 단 **pause 진입 직전에는 1회 flush** 해서
  잔여 배치가 pause 기간 내내 갇히지 않게 한다.

**P0-2. 실측 cols/rows 로 spawn 한다** (2.2 해소)

- `pty_default_options` 의 `DEFAULT_TERMINAL_COLS/ROWS`(80/24)는 **최후 폴백으로만** 남긴다.
- 마운트 순서를 뒤집는다: `TerminalView` 를 먼저 마운트해 `fit.proposeDimensions()` 로 치수를 얻고,
  그 값을 `PtySpawnOptions.cols/rows` 에 실어 spawn 한다. 치수를 얻기 전에는 PTY 를 열지 않는다.
  (`isFinite` 가드 필수 — 숨겨진 컨테이너에서 NaN 이 나온다. hermes#113 이 `NaN < 10` 가드가 무력했던 사례를 남겼다)
- **셸 준비 후 재-resize 안전망**: spawn 직후 한 번, 그리고 첫 출력 수신 직후 한 번 더 현재 치수로
  `pty_resize` 를 보낸다. SIGWINCH 레이스에 대한 방어다(hermes#113 fix 2 와 동일 처방).
- `reflowCursorLine` 은 **기본값 false 를 유지**한다. P0-2 가 제대로 되면 셸이 처리하는 것이 맞고,
  강제로 켜면 셸의 재그리기와 xterm 의 리플로가 이중으로 걸려 새 깨짐을 만들 수 있다.
  P0-1·P0-2 후에도 잔상이 남을 때만 **대조 실험용으로** 켜 본다.

**P0-3. 탭 전환 시 xterm 을 파괴하지 않는다** (2.3 해소)

- `pane-node-view.tsx:83` 의 조건부 마운트를 **"비활성 탭은 숨김"** 으로 바꾼다
  (`hidden`/`display:none` 유지 마운트). 그러면 replay 경로 자체가 사라진다.
- 숨김 유지 시 주의: `display:none` 컨테이너에서 `fit()` 이 NaN 을 낸다 → ResizeObserver 콜백에서
  이미 `isFinite` 가드를 하고 있으므로 유지하되, **다시 보일 때 1회 `fit()`** 을 명시적으로 호출한다.
- 백그라운드 탭의 WebGL addon 은 dispose(설계 문서 §3 방침 유지) → 복귀 시 재로드.
- 그래도 replay 경로는 **앱 재시작·창 재생성** 대비로 남는다. 남기는 경우 최소 보완:
  - replay 앞에 `\x1bc`(RIS) 또는 최소 `\x1b[!p\x1b[2J\x1b[H` 를 붙여 파서 상태를 초기화한다.
  - ring buffer 절단을 **바이트 오프셋이 아니라 "안전 경계"** 에서 한다(직전 `\n` 이후, ESC 시퀀스 밖).
  - `pty_attach` 의 스냅샷 복제와 subscriber 교체를 **하나의 잠금 안에서** 원자적으로 수행한다(2.3(c) 유실 레이스 제거).
    현재는 `ring_buffer` 와 `subscriber` 가 별개 `Mutex` 라 그 사이가 뚫려 있다.

### 5.2 P1 — 구조적 개선

- **터미널 IPC 를 전역 mutation lock 에서 분리한다**(2.8).
  `pty_write`/`pty_resize`/`pty_set_paused`/`pty_attach` 는 `TerminalStore` 자체 잠금만으로 충분하다.
  키 입력이 파일 저장·git 작업 뒤에 줄 서는 현재 구조는 IDE 터미널로서 부적절하다.
- **입력(`pty_write`) 도 배칭**한다. 빠른 타이핑/붙여넣기마다 `invoke` 왕복이 1회씩 발생한다.
  VS Code 도 입력측에 지연 큐를 둔다.
- **flow control ack 를 묶는다.** 현재는 `term.write` 콜백마다 backlog 를 재평가하고 경계를 넘으면 즉시 invoke 한다.
  VS Code 의 `CharCountAckSize` 배칭에 해당하는 완충을 두면 pause/resume 채터링이 줄어든다.
- **HIGH_WATER 재검토.** 공식 가이드는 "500K 를 넘기지 말 것"인데 현재 512KB(=524,288)라 경계에 걸쳐 있다.
  256KB 정도로 낮추면 대량 출력 중 키 입력 반응성이 좋아진다.

### 5.3 P2 — 완성도

- **WebGL 폴백 정책 명문화**: 초기 로드 실패를 try/catch 로 잡아 DOM 렌더러로 남고,
  `terminal.gpuAcceleration: 'auto' | 'off'` 설정을 노출한다(VS Code 대응). v6 엔 canvas 폴백이 없음을 문서화.
- **Windows `windowsPty` 전달**: `{ backend: 'conpty', buildNumber }` 미설정 시 리사이즈 데이터 유실
  (선행 문서 함정 10). 크로스플랫폼 지원 시점에 반드시.
- **버전 정책 결정**: `@xterm/xterm` 은 stable 6.0.0 유지가 기본. 단 VS Code 가 6.1 beta 라인을 쓰고 있으므로
  6.1.0 stable 릴리스 시 즉시 추종 대상으로 등록한다(고친 버그가 우리 증상과 겹칠 수 있음).
- **OSC 133/633 + 자체 suggest UI**(설계 문서 §5 의 연장): 셸측 화면 그리기 자동완성에 대한 의존을 낮추는
  근본 방향. VS Code 의 `E`(명령줄 원문)·`P;Cwd`(cwd) 확장을 참고해 프로토콜을 정한다.

### 5.4 진단 체크리스트 (수정 전 원인 확정용)

수정 순서를 정하기 전에, 아래를 순서대로 해서 A/B/C 중 무엇이 지배적인지 확인한다.
`docs/quality-assurance` 로 옮겨 체크박스 관리하는 것을 권한다.

- [ ] **폭 확인**: 잔상이 난 터미널에서 `tput cols` / `echo $COLUMNS` / `stty size` 실행.
      80 이 나오면 **B 확정**(2.2). 실제 폭이 나오면 B 는 배제.
- [ ] **정지 여부 확인**: 잔상이 보이는 상태에서 아무 키나 한 번 누른다(예: 방향키).
      **그 순간 화면이 정상으로 복구되면 A 확정**(2.1 — 갇힌 배치가 그때 풀린 것).
- [ ] **탭 전환 상관**: 터미널 탭을 나갔다 돌아온 뒤에만 발생하는지 확인. 그렇다면 **C 가 지배적**(2.3).
- [ ] **렌더러 대조**: WebGL addon 로드를 잠시 끄고(=DOM 렌더러) 재현. 그래도 나면 렌더러 무관(2.6 배제 확정).
- [ ] **플러그인 대조**: `zsh-autocomplete` 를 끄고 순정 zsh 로 재현. 재현되면 우리 파이프라인 문제로 확정,
      안 나면 "저빈도 소량 출력" 조건이 필요한 A 와 정합.
- [ ] **외부 터미널 대조**: 동일 zsh 설정을 iTerm2/Ghostty 에서 재현. 재현되면 셸 플러그인 자체 문제,
      안 되면 TAIDE 문제로 확정.

---

## 6. 함정·주의

1. **타이머 flush 를 넣을 때 pause 상태를 무시하면 backpressure 가 무력화된다.** flusher 는 pause 중 flush 금지,
   단 pause 진입 직전 1회 flush 는 필요하다.
2. **`OUTPUT_BATCH_MS` 를 0 으로 만들지 말 것.** 배칭 자체는 옳다 — IPC 왕복을 줄인다.
   문제는 "타이머가 없다"이지 "배칭한다"가 아니다.
3. **spawn 을 지연시키면 첫 화면이 늦어 보일 수 있다.** 치수 측정 → spawn 사이에 스켈레톤/배경만 그리고,
   `proposeDimensions()` 가 NaN 이면 80x24 폴백으로 즉시 진행한다(무한 대기 금지).
4. **탭 숨김 유지로 바꾸면 터미널 개수만큼 xterm 인스턴스가 상주한다.** scrollback 10,000 × 탭 수의
   메모리를 계산해 상한을 정한다. WebGL addon 은 비활성 시 dispose.
5. **replay 를 남길 경우 RIS(`\x1bc`)는 스크롤백도 지운다.** 재생 직전(빈 터미널)에만 쓰고,
   내용이 있는 터미널에 함부로 쓰지 않는다.
6. **`reflowCursorLine: true` 를 "일단 켜 보는" 임시방편으로 쓰지 말 것.** 근본 원인(폭 불일치)을 가리고
   셸 재그리기와 충돌할 수 있다. 대조 실험 후 원복하거나, 원인 수정 뒤에 별도 판단한다.
7. **xterm.js 내부 입력 버퍼 50MB 한계** — 그 이상은 폐기된다(공식 flow control 가이드).
   flow control 이 잘못 구현돼 resume 이 영원히 안 오면 스트림이 통째로 멈출 수 있다는 경고도 같은 문서에 있다.
   `pty_set_paused` 가 전역 lock 뒤에 갇히는 현재 구조(2.8)는 이 위험에 직접 닿아 있다.
8. **`@wterm/ghostty` 를 "xterm.js 호환"이라는 표현만 보고 드롭인으로 가정하지 말 것.**
   버전 0.3.2, 발행 3회다. 도입 검토 시 addon 생태계(fit/search/serialize/webgl) 부재를 먼저 확인해야 한다.
9. **`@xterm/addon-canvas` 는 v6 에서 사용 불가**다. WebGL 실패 시 폴백은 DOM 하나뿐이라는 전제로 설계한다.
10. **VS Code 이슈의 "resize 하면 고쳐진다"는 보고를 해결책으로 오해하지 말 것**(#121891).
    그건 증상 완화이지 원인 수정이 아니다 — 원인은 초기 폭 전달 실패다.

---

## 7. 참고 링크 (1차 출처)

xterm.js
- 저장소: https://github.com/xtermjs/xterm.js
- 공식 flow control 가이드(50MB 한계·watermark 권고): https://xtermjs.org/docs/guides/flowcontrol/
- `EscapeSequenceParser` 소스(청크 경계 상태 보존): https://github.com/xtermjs/xterm.js/blob/master/src/common/parser/EscapeSequenceParser.ts
- `reflowCursorLine` 도입 이슈 #5213: https://github.com/xtermjs/xterm.js/issues/5213
- 리사이즈 문자 유실 이슈 #2121: https://github.com/xtermjs/xterm.js/issues/2121
- WebGL + 리거처 렌더 이슈 #3303: https://github.com/xtermjs/xterm.js/issues/3303
- Safari 26.5 beta WebGL 붕괴 #5816: https://github.com/xtermjs/xterm.js/issues/5816
- v6 타입 정의(로컬 `node_modules/@xterm/xterm/typings/xterm.d.ts` 와 동일): https://unpkg.com/@xterm/xterm@6.0.0/typings/xterm.d.ts
- npm: https://www.npmjs.com/package/@xterm/xterm

VS Code (microsoft/vscode `main`)
- `package.json`(xterm 버전 고정 실측): https://github.com/microsoft/vscode/blob/main/package.json
- `terminalDataBuffering.ts`(throttleBy=5 타이머 flush): https://github.com/microsoft/vscode/blob/main/src/vs/platform/terminal/common/terminalDataBuffering.ts
- `terminalProcess.ts`(FlowControlConstants·ShutdownConstants): https://github.com/microsoft/vscode/blob/main/src/vs/platform/terminal/node/terminalProcess.ts
- `terminalProcessManager.ts`(createProcess(cols,rows)·AckDataBufferer): https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminal/browser/terminalProcessManager.ts
- ptyHost + flow control + event batching 도입 이슈 #74620: https://github.com/microsoft/vscode/issues/74620
- flow control 지원 이슈 #113827: https://github.com/microsoft/vscode/issues/113827
- 초기 출력이 80칼럼으로 랩되는 이슈 #98399: https://github.com/microsoft/vscode/issues/98399
- 잘못된 폭 보고로 인한 자동완성 깨짐 #121891: https://github.com/microsoft/vscode/issues/121891
- WebGL 기본 전환 이슈 #106202: https://github.com/microsoft/vscode/issues/106202
- gpuAcceleration 문서: https://code.visualstudio.com/docs/terminal/appearance
- shell integration(OSC 633) 문서: https://code.visualstudio.com/docs/terminal/shell-integration
- shell integration 시퀀스 확정 이슈 #155639: https://github.com/microsoft/vscode/issues/155639

동종 구현 사례
- hermes-hq/hermes-ide#113 (Tauri+xterm+PTY, 80x24 하드코딩이 root cause): https://github.com/hermes-hq/hermes-ide/issues/113
- cockpit-project/cockpit#22509 (addon-canvas 제거 여파): https://github.com/cockpit-project/cockpit/issues/22509

대안 후보
- libghostty 웹/WASM 논의(ghostty-org/ghostty Discussion #3599): https://github.com/ghostty-org/ghostty/discussions/3599
- wterm (libghostty 기반 웹 터미널 코어): https://wterm.dev/ghostty
- npm `@wterm/ghostty`: https://www.npmjs.com/package/@wterm/ghostty
- Mitchell Hashimoto — Libghostty Is Coming: https://mitchellh.com/writing/libghostty-is-coming
- xterm.es (ESM 포크): https://github.com/vincentdchan/xterm.es

셸 플러그인 측
- zsh-users/zsh-autosuggestions#525 (제안이 지워지지 않음): https://github.com/zsh-users/zsh-autosuggestions/issues/525
- marlonrichert/zsh-autocomplete#128: https://github.com/marlonrichert/zsh-autocomplete/issues/128
