# xterm.js + Rust portable-pty 로 iTerm 급 터미널 구현

작성일: 2026-08-06 / 조사 방식: npm registry API, crates.io API, docs.rs 원본 소스, GitHub Releases API, 공식 문서 직접 조회.
"미확인" 표기가 없는 버전·시그니처는 위 1차 출처에서 실제 응답으로 확인한 값이다.

---

## 버전 확정 (2026-08 기준)

### 프론트엔드 (npm, `dist-tags.latest` 실측)

| 패키지 | 최신 stable | 발행일 | 비고 |
|---|---|---|---|
| `@xterm/xterm` | **6.0.0** | 2025-12-22 | beta 태그는 `6.1.0-beta.292` |
| `@xterm/headless` | 6.0.0 | 2025-12-22 | DOM 없이 파서/버퍼만 |
| `@xterm/addon-fit` | 0.11.0 | 2025-12-22 | |
| `@xterm/addon-webgl` | 0.19.0 | 2025-12-22 | |
| `@xterm/addon-search` | 0.16.0 | 2025-12-22 | |
| `@xterm/addon-web-links` | 0.12.0 | 2025-12-22 | |
| `@xterm/addon-serialize` | 0.14.0 | 2025-12-22 | |
| `@xterm/addon-unicode11` | 0.9.0 | 2025-12-22 | |
| `@xterm/addon-unicode-graphemes` | 0.4.0 | 2025-12-22 | experimental |
| `@xterm/addon-clipboard` | 0.2.0 | 2025-12-22 | OSC 52 |
| `@xterm/addon-image` | 0.9.0 | 2025-12-22 | Sixel / iTerm IIP |
| `@xterm/addon-ligatures` | 0.10.0 | 2025-12-22 | |
| `@xterm/addon-progress` | 0.2.0 | 2025-12-22 | OSC 9;4, v6 신규 |
| `@xterm/addon-attach` | 0.12.0 | 2025-12-22 | WebSocket 전용 — Tauri 에선 불필요 |
| `@xterm/addon-web-fonts` | 0.1.0 | 2026-01-04 | 웹폰트 로딩 완료 후 리렌더 |
| `@xterm/addon-canvas` | 0.7.0 | 2024-04-05 | **v6 비대응.** peerDeps `^5.0.0`, `addons/addon-canvas` 디렉터리가 master 에서 제거됨(GitHub 404). beta `0.8.0-beta.48` 만 존재 |

### 백엔드 (crates.io, `max_stable_version` 실측)

| crate | 최신 stable | 갱신일 |
|---|---|---|
| `portable-pty` | **0.9.0** | 2025-02-11 |
| `tauri` | 2.11.5 | 2026-07-01 |
| `tauri-plugin-shell` | 2.3.5 | 2026-02-03 |
| `tauri-plugin-pty` (서드파티, Tnze) | 0.3.1 | 2026-07-08 |

- `@tauri-apps/api`: **2.11.1** (2026-06-17)
- `tauri-pty` (npm, 위 플러그인의 JS 측): 0.3.1 (2026-07-08)

### xterm.js 6.0.0 주요 변경 (릴리스 노트 원문 기준)

기능: DEC mode 2026 synchronized output, WebGL 렌더러의 shadow DOM 지원, detailed ligatures/variants, `progress-addon` 신규, `reflowCursorLine` 옵션, esbuild 기반 **ESM 지원**, `onWriteParsed` API 공개, ANSI **OSC 52** 지원.

Breaking(릴리스 노트에 :warning: 로 명시된 것):
- `ITerminalOptions.overviewRulerWidth` → `ITerminalOptions.overviewRuler` 하위 속성으로 이동
- viewport/스크롤바가 VS Code `base/` platform 채택으로 **동작이 크게 달라짐**(잠재적 breaking)
- deprecated `windowsMode`, `fastScrollModifier` **제거**
- alt → ctrl+arrow 매핑 hack 제거 → 필요하면 임베더가 직접 키바인딩

---

## 핵심 API·사용법

### 1. 의존성

```bash
npm i @xterm/xterm@6 \
  @xterm/addon-fit @xterm/addon-webgl @xterm/addon-search \
  @xterm/addon-web-links @xterm/addon-serialize \
  @xterm/addon-unicode11 @xterm/addon-clipboard \
  @xterm/addon-image @xterm/addon-ligatures @xterm/addon-web-fonts
```

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = [] }
portable-pty = "0.9"
serde = { version = "1", features = ["derive"] }
parking_lot = "0.12"
anyhow = "1"
```

### 2. addon 전체 목록 — 용도와 설정 (타입 정의 실측)

`@xterm/xterm@6` CSS 는 반드시 import 한다: `import '@xterm/xterm/css/xterm.css'`.

#### FitAddon (`0.11.0`)
```ts
class FitAddon {
  fit(): void
  proposeDimensions(): { rows: number; cols: number } | undefined
}
```
컨테이너 크기에 맞춰 `cols`/`rows` 를 계산해 `terminal.resize()` 를 호출한다. `proposeDimensions()` 로 계산만 하고 실제 resize 는 직접 제어할 수도 있다(PTY resize 와 순서를 맞출 때 유용).

#### WebglAddon (`0.19.0`)
```ts
class WebglAddon {
  constructor(preserveDrawingBuffer?: boolean)
  textureAtlas?: HTMLCanvasElement
  readonly onContextLoss: IEvent<void>
  readonly onChangeTextureAtlas: IEvent<HTMLCanvasElement>
  clearTextureAtlas(): void
}
```
GPU(webgl2) 렌더러. **`onContextLoss` 처리가 필수**다(아래 함정 참고).

#### SearchAddon (`0.16.0`)
```ts
new SearchAddon({ highlightLimit: 1000 })
findNext(term: string, opts?: ISearchOptions): boolean
findPrevious(term: string, opts?: ISearchOptions): boolean
clearDecorations(): void
clearActiveDecoration(): void
readonly onDidChangeResults: IEvent<{ resultIndex: number; resultCount: number }>
// ISearchOptions: regex, wholeWord, caseSensitive, incremental, decorations
// decorations: matchBackground, matchBorder, matchOverviewRuler(필수),
//              activeMatchBackground, activeMatchBorder, activeMatchColorOverviewRuler(필수)
```

#### WebLinksAddon (`0.12.0`)
```ts
new WebLinksAddon(
  handler?: (event: MouseEvent, uri: string) => void,
  options?: { hover?, leave?, urlRegex?: RegExp },
)
```
URL 만 처리한다. **파일 경로는 이걸로 못 한다** → `terminal.registerLinkProvider` 를 직접 쓴다(7번 항목).

#### SerializeAddon (`0.14.0`)
```ts
serialize(options?: {
  range?: { start: IMarker | number; end: IMarker | number }
  scrollback?: number
  excludeModes?: boolean
  excludeAltBuffer?: boolean
}): string
serializeAsHTML(options?: {
  scrollback: number
  onlySelection: boolean
  includeGlobalBackground: boolean
  range?: { startLine: number; endLine: number; startCol: number }
}): string
```

#### Unicode11Addon / UnicodeGraphemesAddon
문자폭 테이블 갱신. 활성화는 addon 로드 후 `terminal.unicode.activeVersion = '11'` 로 지정한다. `UnicodeGraphemesAddon` 은 grapheme cluster(이모지 ZWJ 조합 등) 처리까지 하지만 experimental 이며 `allowProposedApi` 가 필요할 수 있다(정확한 요구 여부 **미확인**).

#### ClipboardAddon (`0.2.0`) — OSC 52
```ts
new ClipboardAddon(base64?: IBase64, provider?: IClipboardProvider)
// ClipboardSelectionType: SYSTEM='c', PRIMARY='p'
// 기본 제공: Base64, BrowserClipboardProvider
```
Tauri 에선 `IClipboardProvider` 를 직접 구현해 `@tauri-apps/plugin-clipboard-manager` 로 위임하면 브라우저 권한 이슈를 피한다.

#### ImageAddon (`0.9.0`)
```ts
new ImageAddon({
  enableSizeReports, pixelLimit, storageLimit, showPlaceholder,
  sixelSupport, sixelScrolling, sixelPaletteLimit, sixelSizeLimit,
  iipSupport, iipSizeLimit,
})
// getImageAtBufferCell(x, y) / extractTileAtBufferCell(x, y) / reset()
// storageLimit(rw), storageUsage(ro), showPlaceholder(rw)
```
`imgcat`, matplotlib 등 iTerm inline image 프로토콜(IIP)과 Sixel 을 처리한다. `storageLimit`(MB) 를 반드시 제한한다.

#### LigaturesAddon (`0.10.0`)
```ts
new LigaturesAddon({ fallbackLigatures: string[], fontFeatureSettings: string })
```
v6 부터 detailed ligatures/variants 지원. 커서가 리거처 범위 안에 있으면 자동 해제된다(#5277).

#### ProgressAddon (`0.2.0`) — OSC 9;4
```ts
readonly onChange: IEvent<{ state: 0|1|2|3|4; value: number }>
```
`state`: 0 remove, 1 set, 2 error, 3 indeterminate, 4 paused (ConEmu OSC 9;4 관례. 값 대응은 문서상 명시 없어 **부분 미확인**). IDE 상태바/탭에 진행률 표시에 그대로 쓸 수 있다.

#### WebFontsAddon (`0.1.0`)
웹폰트가 로드되기 전에 측정된 셀 크기 때문에 레이아웃이 깨지는 문제를 해결한다. API 표면 상세는 **미확인**.

### 3. Terminal 생성 (실전)

```ts
import { Terminal } from '@xterm/xterm'
import '@xterm/xterm/css/xterm.css'
import { FitAddon } from '@xterm/addon-fit'
import { WebglAddon } from '@xterm/addon-webgl'
import { SearchAddon } from '@xterm/addon-search'
import { SerializeAddon } from '@xterm/addon-serialize'
import { Unicode11Addon } from '@xterm/addon-unicode11'

const term = new Terminal({
    allowProposedApi: true,
    fontFamily: '"JetBrainsMono Nerd Font", "SF Mono", Menlo, monospace',
    fontSize: 13,
    lineHeight: 1.2,
    letterSpacing: 0,
    cursorBlink: true,
    cursorStyle: 'bar',
    cursorInactiveStyle: 'outline',
    scrollback: 10_000,
    smoothScrollDuration: 0,
    macOptionIsMeta: true,
    minimumContrastRatio: 1,
    drawBoldTextInBrightColors: true,
    theme: { background: '#101014', foreground: '#e6e6e6', cursor: '#e6e6e6' },
})

const fit = new FitAddon()
const search = new SearchAddon({ highlightLimit: 1000 })
const serialize = new SerializeAddon()
term.loadAddon(fit)
term.loadAddon(search)
term.loadAddon(serialize)
term.loadAddon(new Unicode11Addon())
term.unicode.activeVersion = '11'

term.open(containerElement)

const webgl = new WebglAddon()
webgl.onContextLoss(() => {
    webgl.dispose()
    term.loadAddon(new WebglAddon())
})
term.loadAddon(webgl)
fit.fit()
```

`ITerminalOptions` 전체 키(v6 typings 실측): `allowProposedApi, allowTransparency, altClickMovesCursor, convertEol, cursorBlink, cursorStyle, cursorWidth, cursorInactiveStyle, customGlyphs, disableStdin, documentOverride, drawBoldTextInBrightColors, fastScrollSensitivity, fontSize, fontFamily, fontWeight, fontWeightBold, ignoreBracketedPasteMode, letterSpacing, lineHeight, linkHandler, logLevel, logger, macOptionIsMeta, macOptionClickForcesSelection, minimumContrastRatio, reflowCursorLine, rescaleOverlappingGlyphs, rightClickSelectsWord, screenReaderMode, scrollback, scrollOnEraseInDisplay, scrollOnUserInput, scrollSensitivity, smoothScrollDuration, tabStopWidth, theme, windowsPty, wordSeparator, windowOptions, overviewRuler`.

### 4. portable-pty 0.9.0 — 트레이트/시그니처 (docs.rs 소스 실측)

```rust
pub struct PtySize { pub rows: u16, pub cols: u16, pub pixel_width: u16, pub pixel_height: u16 }
// Default = 24x80, pixel 0

pub struct PtyPair {
    pub slave: Box<dyn SlavePty + Send>,   // slave 가 먼저 drop 되도록 필드 순서가 고정됨
    pub master: Box<dyn MasterPty + Send>,
}

pub trait PtySystem: Downcast {
    fn openpty(&self, size: PtySize) -> anyhow::Result<PtyPair>;
}

pub trait MasterPty: Downcast + Send {
    fn resize(&self, size: PtySize) -> Result<(), Error>;
    fn get_size(&self) -> Result<PtySize, Error>;
    fn try_clone_reader(&self) -> Result<Box<dyn Read + Send>, Error>;
    fn take_writer(&self) -> Result<Box<dyn Write + Send>, Error>; // 두 번 호출 불가
    #[cfg(unix)] fn process_group_leader(&self) -> Option<pid_t>;
    #[cfg(unix)] fn as_raw_fd(&self) -> Option<RawFd>;
    #[cfg(unix)] fn tty_name(&self) -> Option<PathBuf>;
    #[cfg(unix)] fn get_termios(&self) -> Option<Termios> { None }
}

pub trait SlavePty {
    fn spawn_command(&self, cmd: CommandBuilder) -> Result<Box<dyn Child + Send + Sync>, Error>;
}

pub trait Child: Debug + ChildKiller + Downcast + Send {
    fn try_wait(&mut self) -> IoResult<Option<ExitStatus>>;  // 논블로킹
    fn wait(&mut self) -> IoResult<ExitStatus>;              // 블로킹
    fn process_id(&self) -> Option<u32>;
    #[cfg(windows)] fn as_raw_handle(&self) -> Option<RawHandle>;
}

pub trait ChildKiller: Debug + Downcast + Send {
    fn kill(&mut self) -> IoResult<()>;
    fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync>; // wait 중인 스레드와 별개로 kill
}

pub fn native_pty_system() -> Box<dyn PtySystem + Send>;
#[cfg(unix)]    pub type NativePtySystem = unix::UnixPtySystem;
#[cfg(windows)] pub type NativePtySystem = win::conpty::ConPtySystem;  // Windows 기본이 ConPTY
```

`ExitStatus` 는 `code: u32` + `signal: Option<String>` 를 가지며 `success()`, `Display` 를 제공한다.

`CommandBuilder` (전체 메서드):
```rust
CommandBuilder::new<S: AsRef<OsStr>>(program: S) -> Self
CommandBuilder::from_argv(args: Vec<OsString>) -> Self
CommandBuilder::new_default_prog() -> Self      // 사용자 기본 셸을 로그인 셸로 실행
arg / args / env / env_remove / env_clear / cwd / clear_cwd
get_argv / get_argv_mut / get_env / get_cwd / get_shell / is_default_prog
iter_extra_env_as_str / iter_full_env_as_str / as_unix_command_line
set_controlling_tty(bool) / get_controlling_tty()    // 기본 true, flatpak 등에서만 false
#[cfg(unix)] umask(Option<mode_t>)
```

`new_default_prog()` 의 unix 구현(소스 실측)은 **로그인 셸 규약을 정확히 지킨다**:
```rust
let shell = self.get_shell();                      // $SHELL → passwd db
let basename = shell.rsplit('/').next().unwrap_or(&shell);
cmd.arg0(&format!("-{}", basename));               // argv[0] = "-zsh"
cmd.current_dir(dir);                              // cwd 없으면 $HOME → passwd pw_dir → "/"
cmd.env_clear();
cmd.env("SHELL", shell);
cmd.envs(self.envs...);
```
`new_default_prog()` 로 만든 builder 에 `arg()` 를 호출하면 **panic** 한다.

### 5. PTY 세션 관리자 (Rust 전체 예시)

```rust
// src-tauri/src/pty.rs
use anyhow::Result;
use parking_lot::Mutex;
use portable_pty::{Child, ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tauri::ipc::{Channel, InvokeResponseBody};

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    child: Box<dyn Child + Send + Sync>,
}

#[derive(Default)]
pub struct PtyRegistry(Mutex<HashMap<u32, PtySession>>);

#[derive(serde::Deserialize)]
pub struct SpawnOptions {
    pub shell: Option<String>,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Vec<(String, String)>,
    pub cols: u16,
    pub rows: u16,
}

#[tauri::command]
pub fn pty_spawn(
    registry: tauri::State<'_, Arc<PtyRegistry>>,
    id: u32,
    opts: SpawnOptions,
    on_data: Channel<InvokeResponseBody>,
    on_exit: Channel<u32>,
) -> Result<u32, String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: opts.rows, cols: opts.cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    let mut cmd = match opts.shell.as_deref() {
        Some(shell) => {
            let mut c = CommandBuilder::new(shell);
            c.args(&opts.args);
            c
        }
        None => CommandBuilder::new_default_prog(),
    };
    if let Some(cwd) = &opts.cwd {
        cmd.cwd(cwd);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "TAIDE");
    for (k, v) in &opts.env {
        cmd.env(k, v);
    }

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    let killer = child.clone_killer();
    let pid = child.process_id().unwrap_or(0);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    std::thread::spawn(move || {
        let mut buf = [0u8; 64 * 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if on_data.send(InvokeResponseBody::Raw(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let registry_for_wait = registry.inner().clone();
    let mut child_for_wait = child.clone_killer();
    std::thread::spawn(move || {
        // wait 는 별도 스레드에서. registry 에 넣은 child 를 꺼내 wait 하는 대신
        // 아래 구조에서는 child 를 registry 에 보관하고 try_wait 폴링해도 된다.
        let _ = &mut child_for_wait;
        let _ = &registry_for_wait;
        let _ = on_exit.send(0);
    });

    registry.0.lock().insert(id, PtySession { master: pair.master, writer, killer, child });
    Ok(pid)
}

#[tauri::command]
pub fn pty_write(registry: tauri::State<'_, Arc<PtyRegistry>>, id: u32, data: String) -> Result<(), String> {
    let mut map = registry.0.lock();
    let s = map.get_mut(&id).ok_or("no such pty")?;
    s.writer.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    s.writer.flush().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_resize(registry: tauri::State<'_, Arc<PtyRegistry>>, id: u32, cols: u16, rows: u16) -> Result<(), String> {
    let map = registry.0.lock();
    let s = map.get(&id).ok_or("no such pty")?;
    s.master
        .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pty_kill(registry: tauri::State<'_, Arc<PtyRegistry>>, id: u32) -> Result<(), String> {
    let mut map = registry.0.lock();
    if let Some(mut s) = map.remove(&id) {
        s.killer.kill().map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

주의: `take_writer()` 는 한 번만 호출 가능하고, drop 하면 slave 에 EOF 가 간다 → 세션 구조체에 계속 보관한다. `try_clone_reader()` 는 여러 번 호출 가능하다.

### 6. Tauri ↔ xterm 데이터 연결 (Channel + backpressure)

**결론: Event 가 아니라 Channel 을 쓴다.** Tauri 공식 문서: 이벤트 시스템은 "low latency 나 high throughput 을 위해 설계되지 않았고" payload 가 항상 JSON 문자열이며, async 리스너에서는 **순서가 뒤바뀔 수 있다**. Channel 은 "fast, ordered" 로 설계됐고 Tauri 내부의 다운로드 진행률·child process 출력 스트리밍에 쓰인다(인덱스 기반 순서 보장).

Rust 측 API (docs.rs 실측):
```rust
Channel::new(F: Fn(InvokeResponseBody) -> Result<()> + Send + Sync + 'static)
channel.id() -> u32
channel.send<T: IpcResponse>(data: T) -> Result<()>
// IpcResponse 는 impl<T: Serialize> 로 blanket 구현됨
pub enum InvokeResponseBody { Json(String), Raw(Vec<u8>) }  // IpcResponse 구현, From<Vec<u8>> 있음
```

**중요:** `Vec<u8>` 를 그대로 `send()` 하면 blanket `Serialize` 구현을 타서 **JSON 숫자 배열**로 나간다(1바이트당 최대 4문자). PTY 출력처럼 고속 바이너리는 반드시 `InvokeResponseBody::Raw(bytes)` 로 감싸 보낸다. Raw 는 `application/octet-stream` 으로 전달되어 JS `onmessage` 에 **ArrayBuffer** 로 도착한다.

JS 측:
```ts
import { invoke, Channel } from '@tauri-apps/api/core'

const decoder = new TextDecoder()

const onData = new Channel<ArrayBuffer | number[]>()
onData.onmessage = (msg) => {
    const bytes = msg instanceof ArrayBuffer ? new Uint8Array(msg) : new Uint8Array(msg)
    enqueue(bytes)
}

const onExit = new Channel<number>()
onExit.onmessage = (code) => term.write(`\r\n[process exited with code ${code}]\r\n`)

await invoke('pty_spawn', {
    id: 1,
    opts: { shell: null, args: [], cwd: projectRoot, env: [], cols: term.cols, rows: term.rows },
    onData,
    onExit,
})

term.onData((data) => invoke('pty_write', { id: 1, data }))
term.onBinary((data) => invoke('pty_write', { id: 1, data }))
term.onResize(({ cols, rows }) => invoke('pty_resize', { id: 1, cols, rows }))
```

**Backpressure / flow control**: xterm.js `write` 는 콜백을 받는다.
```ts
write(data: string | Uint8Array, callback?: () => void): void
```
파서가 해당 청크를 처리 완료하면 콜백이 불린다. 이걸로 VS Code 식 flow control 을 구현한다 — 미확인 write 가 임계치를 넘으면 PTY reader 를 일시정지시키고, 아래로 내려가면 재개한다.

```ts
const HIGH_WATER = 512 * 1024
const LOW_WATER = 64 * 1024
let pending = 0
let paused = false

const enqueue = (bytes: Uint8Array) => {
    pending += bytes.byteLength
    if (!paused && pending > HIGH_WATER) {
        paused = true
        invoke('pty_set_paused', { id: 1, paused: true })
    }
    term.write(bytes, () => {
        pending -= bytes.byteLength
        if (paused && pending < LOW_WATER) {
            paused = false
            invoke('pty_set_paused', { id: 1, paused: false })
        }
    })
}
```

Rust 쪽 reader 스레드는 `Arc<(Mutex<bool>, Condvar)>` 로 pause 플래그를 확인해 read 루프를 정지시킨다. PTY 는 커널 버퍼가 차면 자연히 writer(자식 프로세스)를 블록시키므로, reader 를 멈추는 것만으로 진짜 backpressure 가 걸린다.

**청크 합치기(coalescing)**: `yes` 같은 출력은 초당 수만 번 read 가 발생한다. reader 스레드에서 1~8ms 단위로 배치해 한 번에 보내면 IPC 왕복이 급감한다.
```rust
let mut batch: Vec<u8> = Vec::with_capacity(256 * 1024);
let mut last_flush = std::time::Instant::now();
// read 후 batch.extend_from_slice(&buf[..n]);
if batch.len() >= 64 * 1024 || last_flush.elapsed() >= std::time::Duration::from_millis(4) {
    on_data.send(InvokeResponseBody::Raw(std::mem::take(&mut batch)))?;
    last_flush = std::time::Instant::now();
}
```

**UTF-8 경계**: `Uint8Array` 를 `term.write()` 에 넘기면 xterm.js 가 내부적으로 UTF-8 로 해석하고 청크 경계에 걸친 멀티바이트도 처리한다(typings: "Raw bytes will always be treated as UTF-8 encoded"). 그러니 **Rust/JS 어디서도 `String::from_utf8` 로 미리 문자열화하지 말 것** — 경계에서 깨진다.

### 7. OS별 사용자 기본 셸 감지

#### 가장 단순한 답: portable-pty 에 맡기기
`CommandBuilder::new_default_prog()` 는 unix 에서 `$SHELL` → 실행 가능 검사 → 실패 시 `getpwuid(getuid())->pw_shell` → 그것도 실패면 `/bin/sh` 순으로 해결한다(소스 실측). Windows 에서는 `%ComSpec%` → `cmd.exe`. 즉 "그냥 기본 셸 열기"는 추가 코드가 필요 없다.

다만 IDE 는 **셸 선택 UI**가 필요하므로 열거가 따로 있어야 한다.

#### macOS
```rust
// 1순위: $SHELL
std::env::var("SHELL")
// 2순위: Directory Services (검증됨 — 로컬 실행 결과 "UserShell: /bin/zsh")
//   dscl . -read /Users/<user> UserShell
// 3순위: getpwuid (portable-pty 내부와 동일)
// 후보 목록: /etc/shells 를 읽어 '#' 주석 제외
```
`/etc/shells` 실측 예: `/bin/bash /bin/csh /bin/dash /bin/ksh /bin/sh /bin/tcsh /bin/zsh`.
Homebrew 설치분(`/opt/homebrew/bin/{fish,nu,zsh}`, Intel 은 `/usr/local/bin/...`)은 `/etc/shells` 에 들어가 있을 수도 아닐 수도 있으므로 경로 존재 검사로 보강한다.

```rust
pub fn detect_default_shell() -> String {
    if let Ok(s) = std::env::var("SHELL") {
        if std::path::Path::new(&s).exists() { return s }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("dscl")
            .args([".", "-read", &format!("/Users/{}", whoami()), "UserShell"]).output()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(v) = s.split_whitespace().nth(1) { return v.to_string() }
        }
    }
    "/bin/sh".into()
}
```

#### Linux
`$SHELL` → `getent passwd $USER` 의 7번째 필드 → `/etc/passwd` 직접 파싱 → `/bin/sh`.
후보 목록은 `/etc/shells`.

#### Windows
Windows 에는 "사용자 기본 셸" 이라는 단일 개념이 없다. VS Code 처럼 **프로필을 열거**한다.

| 프로필 | 탐지 방법 |
|---|---|
| PowerShell 7 (`pwsh.exe`) | `PATH` 검색 + `%ProgramFiles%\PowerShell\7\pwsh.exe` |
| Windows PowerShell 5.1 | `%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe` |
| Command Prompt | `%ComSpec%` (기본 `C:\Windows\System32\cmd.exe`) |
| Git Bash | `HKLM\SOFTWARE\GitForWindows` 의 `InstallPath` → `\bin\bash.exe`. 폴백: `%ProgramFiles%\Git\bin\bash.exe` |
| WSL 배포판 | `HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss` 하위 GUID 키의 `DistributionName` 값 열거. 또는 `wsl.exe -l -q` 출력 파싱(UTF-16LE 출력 주의) → 실행은 `wsl.exe -d <name>` |
| Cygwin | `C:\cygwin64` / `C:\cygwin` (VS Code 는 이를 "unsafe path" 로 분류해 자동 노출하지 않음) |
| MSYS2 | `C:\msys64` (동일하게 unsafe path 취급) |

Git Bash 레지스트리 키 이름(`GitForWindows`/`InstallPath`)은 널리 쓰이는 값이지만 이번 조사에서 Microsoft/Git 공식 문서로 **직접 확인하지 못했다(미확인)** — 구현 시 레지스트리와 기본 설치 경로 폴백을 함께 둔다.

주의: `wsl.exe -l -q` 는 전통적으로 UTF-16LE 로 출력한다. Rust 에서 바이트를 UTF-8 로 가정하면 널 문자가 섞인다. 레지스트리 열거가 더 안정적이다.

### 8. OSC 133 shell integration — Warp 스타일 명령 블록

#### 시퀀스 (Otty/Contour 문서 기준)
```
OSC 133 ; <kind> [ ; <params> ] ST
```
| kind | 시점 | 의미 |
|---|---|---|
| `A` | 프롬프트 출력 직전 | prompt start |
| `B` | 프롬프트 출력 후, 입력 전 | prompt end / command input start |
| `C` | 명령 실행 시작 | output start |
| `D ; <exit>` | 명령 종료 | command end + exit code |
| `L` | 선택 | 도구 내부 입력 프롬프트 시작 |

`ST` 는 `ESC \` (`\e\\`) 또는 BEL(`\a`). `aid=`(application id), `cl=`(cleanup) 같은 확장 파라미터와 `OSC 133;P;k=...`(prompt kind) 는 iTerm2/kitty 계열에서 쓰이나 이번 조사에서 1차 스펙 원문 확인에 실패했다 → **미확인**. 구현 시 알 수 없는 파라미터는 무시하도록 관대하게 파싱한다.

#### 셸별 주입 스니펫

zsh:
```zsh
_taide_precmd()  { print -Pn "\e]133;D;$?\e\\"; print -Pn "\e]133;A\e\\" }
_taide_preexec() { print -Pn "\e]133;C\e\\" }
autoload -Uz add-zsh-hook
add-zsh-hook precmd  _taide_precmd
add-zsh-hook preexec _taide_preexec
PS1="%{$(printf '\e]133;B\e\\')%}$PS1"
```

bash:
```bash
_taide_prompt() {
  local code=$?
  printf '\e]133;D;%s\e\\' "$code"
  printf '\e]133;A\e\\'
}
PROMPT_COMMAND="_taide_prompt${PROMPT_COMMAND:+; $PROMPT_COMMAND}"
PS0='\[\e]133;C\e\\\]'
PS1='\[\e]133;B\e\\\]'"$PS1"
```
(`PS0` 는 bash 4.4+ 에서 명령 실행 직전에 출력되는 프롬프트라 `C` 마킹에 정확히 맞는다.)

fish:
```fish
function _taide_prompt_start --on-event fish_prompt
    printf '\e]133;A\e\\'
end
function _taide_preexec --on-event fish_preexec
    printf '\e]133;C\e\\'
end
function _taide_postexec --on-event fish_postexec
    printf '\e]133;D;%s\e\\' $status
end
```

PowerShell:
```powershell
$Global:__taide_orig_prompt = $function:prompt
function prompt {
  $code = if ($?) { 0 } else { 1 }
  "$([char]0x1b)]133;D;$code$([char]0x1b)\" +
  "$([char]0x1b)]133;A$([char]0x1b)\" +
  (& $Global:__taide_orig_prompt) +
  "$([char]0x1b)]133;B$([char]0x1b)\"
}
```

#### 주입 방법 (VS Code 방식 참고)
사용자 rc 파일을 건드리지 않고 자동 주입하려면:
- **zsh**: 임시 디렉터리를 만들어 `.zshrc` 에 (a) TAIDE 통합 스크립트 source, (b) 원래 `ZDOTDIR` 복원 후 사용자 rc 재-source 를 넣고, `ZDOTDIR` 을 그 임시 디렉터리로 지정해 스폰한다. 초기화 후 임시 디렉터리는 정리한다.
- **bash**: `bash --init-file <스크립트>` 인자 주입. 스크립트 끝에서 `~/.bashrc` 를 source 한다. 오래된 bash 에선 실패할 수 있다.
- **fish / PowerShell**: `XDG_DATA_DIRS` 확장(fish 는 `vendor_conf.d` 자동 로드), PowerShell 은 `-NoExit -Command`. 정확한 인자는 셸 버전별 검증 필요 → **부분 미확인**.
- 스폰 시 `TAIDE_SHELL_INTEGRATION=1` 등 환경변수를 심어 스크립트가 중복 로드/사용자 opt-out 을 감지하게 한다.

#### xterm.js 측 파싱 → 블록 모델
```ts
type Block = {
    command: string
    startMarker: IMarker
    outputStartMarker?: IMarker
    endMarker?: IMarker
    exitCode?: number
}

const blocks: Block[] = []
let current: Block | undefined

term.parser.registerOscHandler(133, (data) => {
    const [kind, ...params] = data.split(';')
    switch (kind) {
        case 'A':
            current = { command: '', startMarker: term.registerMarker(0) }
            blocks.push(current)
            break
        case 'B':
            break
        case 'C':
            if (current) current.outputStartMarker = term.registerMarker(0)
            break
        case 'D':
            if (current) {
                current.endMarker = term.registerMarker(0)
                current.exitCode = params[0] !== undefined ? Number(params[0]) : undefined
                current = undefined
            }
            break
    }
    return false // false = 다른 핸들러/기본 처리로도 전달
})
```
`registerOscHandler(ident: number, callback: (data: string) => boolean | Promise<boolean>): IDisposable`, `registerMarker(cursorYOffset?: number): IMarker` 는 v6 typings 실측 시그니처다. `IMarker` 는 버퍼가 스크롤돼도 논리 행을 추적하고, 그 행이 scrollback 밖으로 밀려나면 dispose 된다.

블록 기반 UX(명령 단위 접기/복사/재실행, exit code 배지, 이전/다음 명령 점프)는 이 `blocks` 배열 + `IDecoration`(`term.registerDecoration({ marker })`)으로 구현한다. `term.scrollToLine(marker.line)` 로 점프.

#### 구현 반영 (Wave E, 2026-08-15) — 위 설계 대비 확정된 사실·변경점

정본 구현: `src-tauri/src/infra/shell_integration.rs`(주입) ·
`src/features/terminal/terminal-osc133.ts`(파싱·블록 모델). 계약:
`docs/acknowledge/2026-08-15-wave-e-terminal-tasks-contract.md`.

- **zsh 주입은 `.zshrc` 하나가 아니라 `.zshenv`/`.zprofile`/`.zshrc` 세 파일**이 필요하다(§8 원안의
  "(a)(b)"는 `.zshrc` 만 다뤘으나 불충분함이 구현 후 리뷰에서 드러남). zsh 는 `.zshenv`(항상)와
  `.zprofile`(로그인 셸)을 `.zshrc` 보다 먼저, 그 시점의 `$ZDOTDIR`(=임시 디렉터리) 기준으로
  읽는다. `.zshrc` 안에서만 `ZDOTDIR` 을 복원하면 이미 늦어 사용자의 실제 `.zshenv`/`.zprofile`
  (PATH·env 설정 — 예: Homebrew 기본 설치가 쓰는 `~/.zprofile` 의 `shellenv`)이 조용히 유실된다.
  VS Code 실제 소스(`shellIntegration-env.zsh`/`-profile.zsh`)를 실측 확인해, 세 파일 모두 "원래
  `ZDOTDIR` 로 잠깐 전환 → 사용자 동명 파일 source → 다시 임시 디렉터리로 복귀" 패스스루 구조를
  채택했다. `.zlogin` 은 `.zshrc` 가 `ZDOTDIR` 을 영구 복원한 **이후** 읽히므로 패스스루가
  필요 없다(사용자의 실제 `.zlogin` 이 그대로 읽힘).
- **fish 는 주입하지 않는다** — 원안의 "이벤트 함수" 스니펫(위 코드블록)은 실제로는 불필요했다.
  fish 는 fish-shell#10352(**fish 4.0+**)부터 `fish_prompt`/`fish_preexec`/`fish_postexec` 가
  OSC133 을 네이티브로 방출한다(fish 4.8.1 로 실측 확인). **다만 fish 4.0 미만(여전히 사용 중인
  구버전)은 버전 감지도 폴백 주입도 없어 OSC133 을 전혀 받지 못한다** — 이 gap 은 위 이벤트-함수
  스니펫으로 메울 수 있으나, 이번 구현은 버전 감지 로직 추가 없이 "fish 는 항상 네이티브"로
  단순화했다. 의도적 보류로 기록(사용자 승인 필요 시 후속 채택).
- **bash 는 로그인/비로그인 캐스케이드를 손수 재현한다** — `CommandBuilder::new(path)`(주입 시
  필수 경로)는 `new_default_prog()` 와 달리 argv0 에 로그인 프리픽스(`-`)를 붙이지 않는다. 그래서
  주입 전 로그인 셸로 스폰됐을 세션(= `config.shell` 이 `None`)은 `/etc/profile` +
  `.bash_profile`/`.bash_login`/`.profile` 캐스케이드를 스크립트가 직접 재현하고, 명시적 셸
  오버라이드(이미 비로그인이었던 세션)는 `.bashrc` 만 재현한다.
  **알려진 한계**: macOS 기본 배포 bash 는 3.2.57(2007년 GPLv2 동결)로 `PS0` 를 지원하지 않는다
  (`PS0` 는 bash **4.4+** 전용). 따라서 이 환경에서는 `C`(output-start) 마커가 발생하지 않고
  `outputStartMarker` 가 항상 `null` 로 남는다 — `A`/`D` 만으로 블록 경계·종료코드 배지는 정상
  동작하므로 degradation 으로 수용(DEBUG trap 기반 대안은 재진입·중복 발화 가드가 필요해 별도
  검증 없이는 채택하지 않음).
- **`posix_quote`(`infra/shell_quote.rs`) 는 백슬래시도 이중 이스케이프한다** — 원래 구현은
  홑따옴표(`'`)만 `'\''` 로 이스케이프했는데, 이는 POSIX sh/bash/zsh 에는 충분해도 **fish 에는
  불충분**하다. fish 는 홑따옴표 안에서도 `\'`·`\\` 를 살아있는 이스케이프로 해석하므로, 원본 값에
  홑따옴표 앞에 백슬래시가 있으면(`foo\'; rm -rf ~ #` 등) 그 백슬래시가 이 함수 자신의 `'\''`
  이스케이프와 결합해 fish 에서 인용을 조기 종료시키고 `;` 뒤가 별도 명령으로 실행될 수 있었다.
  백슬래시를 항상 `\\` 로 이중 이스케이프하면 POSIX 셸에서는 무해(홑따옴표 안 백슬래시는 원래도
  완전히 리터럴)하면서 fish 에서도 인용 탈출을 막는다.
- OSC7 cwd 추적은 원안 그대로 계약 §3.1 에 따라 이번 범위 밖(backlog).

### 9. 폰트 크기 동적 변경 + FitAddon 재계산

폰트 변경 → 셀 크기 변경 → cols/rows 변경 → PTY resize 까지 한 흐름으로 묶는다.

```ts
const setFontSize = (size: number) => {
    term.options.fontSize = size          // v5+ 는 setOption 대신 options 프로퍼티 직접 대입
    term.options.lineHeight = 1.2
    fit.fit()                             // 새 셀 크기로 cols/rows 재계산 + term.resize 호출
    // term.onResize 핸들러가 pty_resize 를 호출하므로 별도 호출 불필요
}

// Ctrl/Cmd +, -, 0
term.attachCustomKeyEventHandler((e) => {
    if (e.type !== 'keydown' || !(e.metaKey || e.ctrlKey)) return true
    if (e.key === '=' || e.key === '+') { setFontSize(term.options.fontSize! + 1); return false }
    if (e.key === '-') { setFontSize(Math.max(6, term.options.fontSize! - 1)); return false }
    if (e.key === '0') { setFontSize(13); return false }
    return true
})
```

컨테이너 리사이즈는 `ResizeObserver` + rAF 디바운스로:
```ts
let raf = 0
const ro = new ResizeObserver(() => {
    cancelAnimationFrame(raf)
    raf = requestAnimationFrame(() => {
        const dims = fit.proposeDimensions()
        if (!dims || !isFinite(dims.cols) || !isFinite(dims.rows)) return
        if (dims.cols !== term.cols || dims.rows !== term.rows) fit.fit()
    })
})
ro.observe(containerElement)
```
`proposeDimensions()` 로 먼저 비교하는 이유: 값이 같은데 `fit()` 을 부르면 불필요한 리플로우와 PTY SIGWINCH 가 발생한다. 컨테이너가 `display:none` 이면 `proposeDimensions()` 가 `undefined` 또는 NaN 을 낼 수 있으니 반드시 가드한다.

WebGL 사용 시 폰트가 바뀌면 텍스처 아틀라스가 재생성되는데, 남는 캔버스가 있으면 `webgl.clearTextureAtlas()` 로 강제 갱신한다.

### 10. 파일 경로 링크 감지 (커스텀 LinkProvider)

`WebLinksAddon` 은 URL 전용이므로, 경로는 `registerLinkProvider` 로 직접 구현한다.

```ts
interface ILinkProvider {
    provideLinks(bufferLineNumber: number, callback: (links: ILink[] | undefined) => void): void
}
interface ILink {
    range: IBufferRange              // 1-based, { start: {x,y}, end: {x,y} }
    text: string
    decorations?: { pointerCursor?: boolean; underline?: boolean }
    activate(event: MouseEvent, text: string): void
    hover?(event: MouseEvent, text: string): void
    leave?(event: MouseEvent, text: string): void
    dispose?(): void
}
```

```ts
// path:line:col / path(line,col) / ./rel/path.ts / /abs/path.rs / ~/x.md
const PATH_RE =
    /(?:^|[\s'"`([<])((?:~|\.{1,2})?\/?[\w.@+-]+(?:\/[\w.@+-]+)*\.[A-Za-z0-9]{1,10})(?::(\d+))?(?::(\d+))?/g

term.registerLinkProvider({
    provideLinks(y, callback) {
        const line = term.buffer.active.getLine(y - 1)
        if (!line) return callback(undefined)
        const text = line.translateToString(true)

        const links: ILink[] = []
        PATH_RE.lastIndex = 0
        let m: RegExpExecArray | null
        while ((m = PATH_RE.exec(text))) {
            const [, path, lineNo, colNo] = m
            const startIndex = m.index + m[0].indexOf(path)
            links.push({
                text: m[0].slice(m[0].indexOf(path)),
                range: {
                    start: { x: startIndex + 1, y },
                    end: { x: startIndex + m[0].length - m[0].indexOf(path), y },
                },
                decorations: { pointerCursor: true, underline: true },
                activate: async () => {
                    const resolved = await invoke<string | null>('resolve_terminal_path', {
                        path,
                        cwd: currentCwd,
                    })
                    if (resolved) openInEditor(resolved, Number(lineNo) || 1, Number(colNo) || 1)
                },
                hover: (_e, t) => showHoverTooltip(t),
                leave: () => hideHoverTooltip(),
            })
        }
        callback(links.length ? links : undefined)
    },
})
```

핵심 포인트:
- `provideLinks` 의 `bufferLineNumber` 는 **1-based**, `buffer.active.getLine()` 은 **0-based** → `y - 1`.
- `range` 좌표도 1-based.
- `translateToString(true)` 로 trailing whitespace 를 제거해 인덱스 계산을 단순화한다. wrapped line 은 `line.isWrapped` 로 이어붙여야 정확하다.
- 존재 여부 검증(`resolve_terminal_path`)은 **Rust 에서** 하고, cwd 는 OSC 7(`\e]7;file://host/path\e\\`)을 파싱해 추적하는 게 정확하다. `term.parser.registerOscHandler(7, ...)`.
- hover 툴팁 DOM 은 `term.element` 안에 만들고 `xterm-hover` 클래스를 붙여야 마우스 이벤트가 뚫고 내려가지 않는다(typings 문서 명시).

### 11. serialize addon 으로 세션 복원

```ts
// 저장
const snapshot = serialize.serialize({ scrollback: 5000, excludeAltBuffer: true })
await invoke('save_terminal_snapshot', { id: 1, snapshot })

// 복원 — 새 Terminal 을 만든 뒤 PTY 연결 전에 write
term.write(snapshot)

// 명령 블록 단위 복사(OSC 133 marker 활용)
const blockText = serialize.serialize({
    range: { start: block.outputStartMarker!, end: block.endMarker! },
})

// HTML export (버그 리포트 첨부용)
const html = serialize.serializeAsHTML({
    scrollback: 1000,
    onlySelection: false,
    includeGlobalBackground: true,
})
```

- `serialize()` 는 **VT 시퀀스 문자열**을 반환하므로 `term.write()` 로 바로 되돌릴 수 있다(색·속성 포함).
- `excludeModes: true` 로 하면 DECSET 모드 복원을 생략한다. alt buffer 상태(vim 등)까지 복원할 필요가 없으면 `excludeAltBuffer: true`.
- 세션 복원은 어디까지나 **화면 복원**이다. 프로세스는 죽어 있으므로, 복원 후 새 PTY 를 붙이고 "이전 세션" 구분선을 그려주는 게 정직하다.
- `range` 에 `IMarker` 를 넘길 수 있어 OSC 133 블록과 자연스럽게 결합된다.
- v6 에서 `addon-serialize` 의 HTML escape 버그가 수정됐다(#5020) — HTML export 쓸 거면 반드시 0.14.0 이상.

### 12. 성능 옵션

| 항목 | 권장 | 근거 |
|---|---|---|
| 렌더러 | `WebglAddon` (webgl2 canvas) | README 명시 GPU 가속 렌더러. canvas addon 은 v6 미대응 |
| 폴백 | DOM 렌더러(기본) | webgl 실패 시 자동으로 남음 |
| `scrollback` | 5,000 ~ 20,000 | 기본 1000. **공식 상한 명시 없음(미확인)**. 행당 셀 데이터가 메모리를 차지하므로 탭 수 × scrollback 으로 계산 |
| `smoothScrollDuration` | 0 | 부드러운 스크롤은 rAF 를 계속 돌린다 |
| `minimumContrastRatio` | 1(비활성) | 1 초과면 셀마다 대비 계산 + 색 캐시 |
| `allowTransparency` | false | typings 에 "negatively impact performance" 명시 |
| `customGlyphs` | true(기본) | 박스드로잉이 폰트 대신 직접 그려져 이음새가 깔끔. DOM 렌더러에선 무효 |
| 리거처 | 필요할 때만 | 셰이핑 비용이 있음 |
| write | `Uint8Array` 직접 | 문자열 변환 왕복 제거 |
| IPC | `InvokeResponseBody::Raw` + 4ms 배칭 | JSON 직렬화·이벤트 오버헤드 제거 |
| 백그라운드 탭 | 렌더러 dispose 또는 `@xterm/headless` | 보이지 않는 탭에서 GPU/rAF 낭비 방지 |

`@xterm/headless@6.0.0` 는 DOM 없이 파서·버퍼만 돌린다. 백그라운드 탭을 headless 로 유지하다가 활성화 시 `serialize()` → 실제 Terminal 로 이관하는 전략도 가능하다(구현 복잡도는 있음).

---

## TAIDE 적용 가이드

### FSD 배치

```
src/entities/terminal/
  terminal.type.ts        # PtySpawnOptions, TerminalBlock, ShellProfile
  terminal.api.ts         # invoke 래퍼: ptySpawn/ptyWrite/ptyResize/ptyKill/listShells
  terminal.query.ts       # useShellProfiles (셸 목록만 서버상태), 'use client'
  terminal.store.ts       # zustand: 탭 목록, activeId, fontSize (고빈도 전역 → 실수요 인정)
src/features/terminal/
  terminal-tab-bar.tsx    # 순수 UI (props + 콜백)
  terminal-block-badge.tsx
  terminal-search-bar.tsx
src/widgets/terminal/
  terminal-panel.tsx      # PTY 연결·Channel 구독·블록 파싱 등 비즈니스 로직
  use-xterm.ts            # Terminal 인스턴스 수명 관리 훅
src/shared/lib/constants/terminal.ts   # HIGH_WATER, LOW_WATER, DEFAULT_SCROLLBACK 등
```

`Terminal` 인스턴스는 React state 에 넣지 않는다. `useRef` 로 잡고, `useEffect` 는 return 바로 위에 두어 `term.open` / `dispose` 만 담당한다.

```ts
export const useXterm = (containerRef: RefObject<HTMLDivElement>) => {
    const termRef = useRef<Terminal>(null)
    const fitRef = useRef<FitAddon>(null)

    useEffect(() => {
        if (!containerRef.current) return
        const term = new Terminal({ /* 위 옵션 */ })
        const fit = new FitAddon()
        term.loadAddon(fit)
        term.open(containerRef.current)
        const webgl = new WebglAddon()
        webgl.onContextLoss(() => { webgl.dispose(); term.loadAddon(new WebglAddon()) })
        term.loadAddon(webgl)
        fit.fit()
        termRef.current = term
        fitRef.current = fit
        return () => term.dispose()
    }, [])

    return { termRef, fitRef }
}
```

`useCallback`/`useMemo` 는 쓰지 않는다(React Compiler 위임). 단 `enqueue` 같은 flow-control 상태는 컴포넌트 리렌더와 무관해야 하므로 `useRef` 객체 안에 보관한다.

### 매직넘버

`HIGH_WATER = 512 * 1024`, `LOW_WATER = 64 * 1024`, `READ_BUF = 64 * 1024`, `BATCH_MS = 4`, `DEFAULT_SCROLLBACK = 10_000`, `MIN_FONT_SIZE = 6` 은 전부 `shared/lib/constants/terminal.ts` 에 UPPER_SNAKE 상수로 둔다.

### 단계별 구현 순서 권장

1. Rust `PtyRegistry` + `pty_spawn/write/resize/kill` + Channel Raw 전송 (셸은 `new_default_prog()` 고정)
2. React `useXterm` + FitAddon + ResizeObserver + `onData/onResize` 배선 → 여기까지가 "동작하는 터미널"
3. WebGL + flow control(HIGH/LOW water) + 4ms 배칭 → 여기까지가 "빠른 터미널"
4. 셸 프로필 열거(OS별) + 탭/분할
5. OSC 133 주입 + 블록 모델 + Decoration → 여기서 Warp 급으로 올라감
6. OSC 7 cwd 추적 + 파일 경로 LinkProvider → IDE 통합의 핵심 (에디터 열기)
7. Serialize 스냅샷(세션 복원/블록 복사) + Search + Clipboard/Image addon

6번(경로 클릭 → 에디터 열기)이 "IDE 내장 터미널"의 차별점이므로 5·6 을 묶어 우선순위를 올리는 게 좋다.

### Tauri 설정

- Channel 은 capability 별도 선언이 필요 없다(command 인자로 전달). command 자체는 `tauri.conf.json`/capabilities 에서 노출 범위를 관리한다.
- `tauri-plugin-shell` 은 PTY 를 제공하지 않는다(단발 실행/사이드카 전용). 터미널에는 `portable-pty` 직접 사용이 맞다.
- 서드파티 `tauri-plugin-pty` (0.3.1) 는 프로토타이핑엔 빠르지만, flow control·OSC 133·셸 프로필처럼 TAIDE 고유 요구가 많으므로 직접 구현을 권한다.

---

## 함정·주의

1. **`take_writer()` 는 한 번만.** 두 번 호출하면 실패한다. 그리고 writer 를 drop 하면 slave 에 EOF 가 가서 셸이 종료된다 → 세션 구조체에 반드시 보관.
2. **`PtyPair` 의 slave 를 붙잡고 있으면 셸이 죽어도 EOF 가 안 온다.** `spawn_command` 후 slave 를 drop 해야 reader 가 정상적으로 0 을 읽고 종료를 감지한다. 필드 순서상 `PtyPair` 자체를 drop 하면 slave 가 먼저 정리되지만, master 만 따로 보관하는 위 예제 구조에서는 `pair.slave` 가 스코프 종료로 drop 되는지 반드시 확인할 것.
3. **`Vec<u8>` 를 `Channel::send` 에 그대로 넘기면 JSON 숫자 배열이 된다.** `IpcResponse` 가 `impl<T: Serialize>` blanket 이라 컴파일은 통과하고 성능만 조용히 죽는다. 항상 `InvokeResponseBody::Raw`.
4. **이벤트(`emit`)로 PTY 출력을 보내면 순서가 깨질 수 있다.** Tauri 공식 문서가 async 리스너에서의 out-of-order 를 명시한다. Channel 필수.
5. **UTF-8 을 Rust/JS 에서 미리 문자열화하면 안 된다.** 64KB 경계에 멀티바이트가 걸린다. bytes → `term.write(Uint8Array)`.
6. **flow control 없으면 `yes` 한 방에 UI 가 멈춘다.** `term.write` 의 콜백을 반드시 쓴다.
7. **`WebglAddon` 은 context loss 를 처리하지 않으면 화면이 통째로 사라진다.** `onContextLoss` → dispose 후 재로드. 또한 v6 에서 "WebglRenderer 가 throw 한 뒤 리스너가 남던" 버그가 고쳐졌으므로(#5305) 0.19.0 미만을 쓰지 말 것.
8. **`@xterm/addon-canvas` 는 v6 에서 쓸 수 없다.** stable 0.7.0 의 peerDeps 는 `^5.0.0` 이고 저장소 master 에서 디렉터리가 제거됐다. 렌더러 폴백은 DOM 이다.
9. **v6 breaking 3종을 반드시 확인**: `overviewRulerWidth` → `overviewRuler`, `windowsMode`/`fastScrollModifier` 제거, alt→ctrl+arrow 매핑 제거(직접 키바인딩 필요). viewport/스크롤바 구현이 바뀌었으므로 v5 시절 커스텀 스크롤바 CSS 는 재검증 대상.
10. **Windows ConPTY**: `native_pty_system()` 이 Windows 에서 `win::conpty::ConPtySystem` 을 고른다. xterm 측에 반드시 알려야 한다.
    ```ts
    new Terminal({ windowsPty: { backend: 'conpty', buildNumber: 19045 } })
    ```
    typings 문서 원문: 값이 설정되면 (a) rows 증가 시 scrollback 에서 되돌아오지 않고 빈 행이 생기는 ConPTY 동작에 맞춘 보정이 켜지고, (b) `backend === 'conpty' && buildNumber >= 21376` 이 아니면 **reflow 가 비활성화**되고 "마지막 문자가 공백이 아니면 wrapped 로 간주" 휴리스틱이 적용된다. buildNumber 를 안 넘기면 리사이즈 시 데이터가 유실될 수 있다.
11. **`fit()` 을 숨겨진 컨테이너에서 호출하면 cols/rows 가 NaN/0 이 된다.** `proposeDimensions()` 결과를 `isFinite` 로 가드.
12. **resize 순서**: xterm `resize` → PTY `resize` 순으로 하면 SIGWINCH 후 셸이 다시 그리는 내용이 새 크기와 맞는다. 반대 순서면 한 프레임 깨진 화면이 남을 수 있다.
13. **`registerMarker` 로 만든 IMarker 는 그 행이 scrollback 밖으로 나가면 dispose 된다.** OSC 133 블록 목록에서 dead marker 를 주기적으로 정리하지 않으면 메모리와 UI 가 어긋난다.
14. **OSC 핸들러 반환값**: `false` 를 반환해야 기본 처리/다른 핸들러로 전달된다. `true` 를 반환하면 소비된다. 통계 수집 목적이면 `false`.
15. **셸 통합 자동 주입은 사용자 rc 를 깨뜨릴 수 있다.** ZDOTDIR 트릭은 원래 `ZDOTDIR` 복원과 rc 재-source 를 정확히 해야 하고, powerlevel10k 처럼 instant prompt 를 쓰는 설정과 충돌 사례가 보고돼 있다. opt-out 환경변수와 "수동 설치 안내"를 반드시 함께 제공한다.
16. **`term.options.fontSize` 대입 후 `fit()` 을 안 부르면** 셀 크기와 cols/rows 가 어긋나 커서 위치가 밀린다.
17. **웹폰트 사용 시 초기 측정 오류.** 폰트 로드 전에 `open()`+`fit()` 하면 잘못된 셀 크기로 고정된다. `document.fonts.ready` 를 await 하거나 `@xterm/addon-web-fonts` 를 쓴다.
18. **ImageAddon 의 `storageLimit`** 을 안 잡으면 이미지가 쌓여 메모리를 먹는다. `storageUsage` 로 모니터링.
19. **`allowProposedApi: true`** 를 켜지 않으면 일부 addon(unicode-graphemes 등)이 throw 한다. 반대로 켜면 proposed API 변경에 노출된다.
20. **`clone_killer()`** 를 써야 `wait()` 로 블록된 스레드와 무관하게 kill 할 수 있다. `Child` 를 Mutex 로 감싸고 `wait()` 를 호출하면 그 Mutex 가 영원히 잠긴다 — 실전에서 가장 흔한 데드락.
21. **크로스플랫폼 개행**: PTY termios 가 `\n` → `\r\n` 을 처리하므로 `convertEol` 은 **켜지 않는다**(typings 문서가 명시적으로 권하지 않음).

---

## 참고 링크

xterm.js
- 저장소: https://github.com/xtermjs/xterm.js
- 6.0.0 릴리스 노트: https://github.com/xtermjs/xterm.js/releases/tag/6.0.0
- 릴리스 목록: https://github.com/xtermjs/xterm.js/releases
- npm: https://www.npmjs.com/package/@xterm/xterm
- v6 타입 정의(본 문서 시그니처 출처): https://unpkg.com/@xterm/xterm@6.0.0/typings/xterm.d.ts
- 공식 문서: https://xtermjs.org/docs/
- 다운로드 가이드: https://xtermjs.org/docs/guides/download/
- addon 타입 정의: `https://unpkg.com/@xterm/addon-<name>@<version>/typings/addon-<name>.d.ts`

portable-pty / Rust
- crates.io: https://crates.io/crates/portable-pty
- docs.rs 0.9.0: https://docs.rs/portable-pty/0.9.0/portable_pty/
- MasterPty: https://docs.rs/portable-pty/0.9.0/portable_pty/trait.MasterPty.html
- CommandBuilder: https://docs.rs/portable-pty/0.9.0/portable_pty/cmdbuilder/struct.CommandBuilder.html
- cmdbuilder 소스(셸 감지 로직 원문): https://docs.rs/portable-pty/0.9.0/src/portable_pty/cmdbuilder.rs.html
- lib.rs 소스(트레이트 정의 원문): https://docs.rs/portable-pty/0.9.0/src/portable_pty/lib.rs.html

Tauri
- Calling the Frontend from Rust (Channel 공식 예제): https://v2.tauri.app/develop/calling-frontend/
- IPC 개념: https://v2.tauri.app/concept/inter-process-communication/
- `ipc::Channel`: https://docs.rs/tauri/2.11.5/tauri/ipc/struct.Channel.html
- `ipc::InvokeResponseBody`: https://docs.rs/tauri/2.11.5/tauri/ipc/enum.InvokeResponseBody.html
- IPC 내부 구조 정리: https://deepwiki.com/tauri-apps/tauri/4-ipc-and-frontend-backend-communication
- Deprecate JSON in IPC 이슈: https://github.com/tauri-apps/tauri/issues/7706

OSC 133 / shell integration
- 스펙 원문(Per Bothner, semantic-prompts): https://gitlab.freedesktop.org/Per_Bothner/specifications/blob/master/proposals/semantic-prompts.md (직접 fetch 차단됨 — 2차 출처로 대체 확인)
- Contour 문서: https://contour-terminal.org/vt-extensions/osc-133-shell-integration/
- Otty 문서: https://docs.otty.sh/vt/osc/osc-133
- Ghostty OSC 133 구현: https://deepwiki.com/ghostty-org/ghostty/9.3-osc-133-prompt-marking
- Ghostty shell integration 시스템: https://deepwiki.com/ghostty-org/ghostty/9-shell-integration
- VS Code shell integration 문서: https://code.visualstudio.com/docs/terminal/shell-integration
- VS Code zsh 통합 스크립트 원본: https://github.com/microsoft/vscode/blob/main/src/vs/workbench/contrib/terminal/common/scripts/shellIntegration-rc.zsh
- VS Code ZDOTDIR 이슈: https://github.com/microsoft/vscode/issues/157128
- Windows Terminal shell integration 튜토리얼: https://github.com/MicrosoftDocs/terminal/blob/main/TerminalDocs/tutorials/shell-integration.md

셸 감지
- VS Code Terminal Profiles: https://code.visualstudio.com/docs/terminal/profiles
- WSL Lxss 레지스트리 키: https://renenyffenegger.ch/notes/Windows/registry/tree/HKEY_CURRENT_USER/Software/Microsoft/Windows/CurrentVersion/Lxss/index

참고 구현
- Tnze/tauri-plugin-pty: https://github.com/Tnze/tauri-plugin-pty (예제: https://github.com/Tnze/tauri-plugin-pty/tree/main/examples/vanilla)
- marc2332/tauri-terminal: https://github.com/marc2332/tauri-terminal
- emee-dev/terax-ai-tauri-terminal: https://github.com/emee-dev/terax-ai-tauri-terminal
