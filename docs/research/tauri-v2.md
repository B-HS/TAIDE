# Tauri 2 최신 현황과 IDE 구축에 필요한 전체 API

> 조사 기준일: 2026-08-06. 모든 버전은 공식 릴리즈 페이지(v2.tauri.app/release) 및 crates.io API 로 확인함.
> 확인하지 못한 항목은 본문에 `미확인` 으로 표기했다.

---

## 버전 확정 (2026-08 기준)

### 코어

| 패키지 | 버전 | 최근 릴리즈일 | 출처 |
|---|---|---|---|
| `tauri` (crate) | **2.11.5** | 2026-07-01 | crates.io API `max_stable_version` |
| `tauri-cli` / `@tauri-apps/cli` | **2.11.4** | 2026-06-28 | v2.tauri.app/release |
| `@tauri-apps/api` (npm) | **2.11.1** | 2026-06-17 | v2.tauri.app/release |
| `tauri-build` | 2.6.3 | - | v2.tauri.app/release |
| `tauri-codegen` | 2.6.3 | - | 〃 |
| `tauri-macros` | 2.6.3 | - | 〃 |
| `tauri-plugin` (플러그인 저작용) | 2.6.3 | - | 〃 |
| `tauri-runtime` | 2.11.3 | - | 〃 |
| `tauri-runtime-wry` | 2.11.4 | - | 〃 |
| `tauri-utils` | 2.9.3 | - | 〃 |
| `tauri-bundler` | 2.9.4 | - | 〃 |
| `tauri-driver` (E2E) | 2.0.6 | - | 〃 |
| `wry` (웹뷰 추상화) | 0.56.0 | - | 〃 |
| `tao` (윈도잉) | 0.36.0 | - | 〃 |
| `create-tauri-app` | 4.7.3 | - | 〃 |

Cargo 의존성은 `tauri = { version = "2", features = [...] }` 처럼 **major 범위(`"2"`)로 지정**하는 것이 공식 스캐폴딩 기본값이다. 2.x 는 SemVer 하위호환을 유지한다.

### 공식 플러그인 전체 (2026-08 시점 최신 버전)

| 플러그인 | 버전 | 최근 릴리즈 |
|---|---|---|
| autostart | 2.5.1 | |
| barcode-scanner | 2.4.5 | 모바일 전용 |
| biometric | 2.3.2 | 모바일 전용 |
| cli | 2.4.1 | |
| clipboard-manager | 2.3.2 | |
| deep-link | 2.4.9 | 2026-05-02 |
| dialog | 2.7.2 | 2026-07-18 |
| fs | 2.5.1 | 2026-05-02 |
| geolocation | 2.3.2 | 모바일 위주 |
| global-shortcut | 2.3.2 | 데스크톱 전용 |
| haptics | 2.3.2 | 모바일 전용 |
| http | 2.5.9 | |
| localhost | 2.3.2 | |
| log | 2.9.0 | 2026-07-13 |
| nfc | 2.3.5 | 모바일 전용 |
| notification | 2.3.3 | 2025-10-27 |
| opener | 2.5.4 | 2026-05-02 |
| os | 2.3.2 | 2025-10-27 |
| persisted-scope | 2.3.7 | |
| positioner | 2.3.3 | |
| process | 2.3.1 | 2025-10-27 |
| shell | 2.3.5 | 2026-02-03 |
| single-instance | 2.4.3 | 2026-07-13 |
| sql | 2.4.0 | |
| store | 2.4.4 | 2026-07-18 |
| stronghold | 2.3.1 | |
| updater | 2.10.1 | 2026-04-04 |
| upload | 2.4.0 | |
| websocket | 2.4.2 | |
| window-state | 2.4.1 | 2025-10-27 |

npm 대응 패키지는 `@tauri-apps/plugin-<name>` 이며 Rust crate 는 `tauri-plugin-<name>` 이다. **npm 버전과 crate 버전은 함께 릴리즈되어 동일 번호를 쓴다**(공식 릴리즈 페이지가 단일 표로 관리).

### 관련 서드파티

| 항목 | 버전 | 비고 |
|---|---|---|
| `tauri-specta` | **2.0.0-rc.25** (2026-05-08) | **stable 릴리즈 없음.** crates.io `max_stable_version` 은 여전히 1.0.2(Tauri v1 용) |
| `specta` | 2.0.0-rc.25 (2026-05-07) | tauri-specta rc.25 가 `=2.0.0-rc.25` 로 핀 고정 |
| `specta-typescript` | 0.0.12 | `^0.0.12` |
| `specta-serde`, `specta-util` | 0.0.12 | |
| `notify` (파일 와처) | **8.2.0** stable (9.0.0-rc.4 존재, 2026-05-02) | 프로덕션은 8.2.0 |
| `tokio` | 1.53.1 (2026-07-20) | |

주의: tauri-specta `main` 브랜치는 이미 `specta 2.0.0-rc.26` 을 요구하며 crates.io 에 미공개라 `[patch.crates-io]` 로 git 을 가리킨다. **crates.io 에서 설치할 때는 rc.25 조합(specta =2.0.0-rc.25 / specta-typescript 0.0.12)을 써야 한다.**

### 최근 주요 변경 (확인 범위)

- 2.0 stable 은 2024-10-02 릴리즈. 이후 2.x 마이너가 누적되어 2026-07 기준 2.11.5.
- 공식 블로그에는 2.1~2.11 각각의 상세 릴리즈 노트 포스트가 없다. 세부 변경은 GitHub `tauri-apps/tauri` 릴리즈 노트/`.changes` 를 봐야 한다. — **버전별 변경 상세는 미확인**.
- 확인된 개별 변경: **메뉴 아이템 아이콘(`set_icon`, `IconMenuItem` 의 `Image` 지원)은 Tauri 2.8.0 부터** (공식 window-menu 문서 명시).
- 2025-03-17 "Experimental Tauri Verso Integration" — Servo 기반 브라우저(Verso)를 웹뷰로 쓰는 실험적 통합 발표. 여전히 실험 단계로 보이며 **프로덕션 가용성은 미확인**.
- `plugin-opener` 가 `shell` 의 `open` 역할을 대체하는 방향으로 분리되어 있다(공식 플러그인 목록에 "Open files and URLs in external applications" 로 별도 존재).

---

## 핵심 API·사용법

### 1. Commands (프론트 → Rust)

```rust
// src-tauri/src/commands.rs
#[tauri::command]
pub fn my_custom_command() {
    println!("I was invoked from JavaScript!");
}

// 인자: JS 는 camelCase 로 보내고 Rust 는 snake_case 로 받는다
#[tauri::command]
pub fn login(user: String, password: String) -> String {
    format!("Hello, {}!", user)
}
```

JS 쪽에서 snake_case 키를 그대로 보내고 싶으면:

```rust
#[tauri::command(rename_all = "snake_case")]
pub fn login(user: String, password: String) -> String { /* ... */ }
```

등록:

```rust
// src-tauri/src/lib.rs
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::my_custom_command,
            commands::login,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application")
}
```

`generate_handler!` 는 **앱 전체에서 단 한 번**만 호출한다. 커맨드 이름은 모듈이 달라도 전역에서 유일해야 한다(중복 시 컴파일 에러).

프론트:

```ts
import { invoke } from '@tauri-apps/api/core'
await invoke('login', { user: 'alice', password: 'secret' })
```

#### 에러 처리 (thiserror + serde)

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

#[tauri::command]
pub fn read_config(path: String) -> Result<String, Error> {
    Ok(std::fs::read_to_string(path)?)
}
```

프론트에서는 `Err` 가 **rejected Promise** 로 도착한다.

#### async command

```rust
#[tauri::command]
async fn my_command(value: String) -> String {
    some_async_function().await;
    value
}
```

- async command 는 별도 async 태스크에서 실행되어 UI 를 블로킹하지 않는다.
- **borrowed 인자(`&str`, `State<'_, T>`)를 쓰려면 반환 타입이 반드시 `Result<_, _>` 여야 한다** (lifetime 이슈 회피).

```rust
#[tauri::command]
async fn my_command(value: &str) -> Result<String, ()> {
    some_async_function().await;
    Ok(value.to_string())
}
```

#### 주입 가능한 특수 인자

```rust
#[tauri::command]
async fn my_command(
    window: tauri::WebviewWindow,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, MyState>,
) -> Result<String, String> {
    Ok("success".into())
}
```

`tauri::Window`, `tauri::WebviewWindow`, `tauri::AppHandle`, `tauri::State<'_, T>`, `tauri::ipc::Channel<T>`, `tauri::ipc::Request`, `tauri::ipc::Response` 등이 시그니처에 그냥 선언하면 주입된다.

### 2. Events (Rust → 프론트)

```rust
use tauri::{AppHandle, Emitter, EventTarget};

#[tauri::command]
fn download(app: AppHandle, url: String) {
    app.emit("download-started", &url).unwrap();       // 전역 브로드캐스트
    app.emit_to("login", "login-result", "loggedIn").unwrap();  // 특정 라벨
    app.emit_filter("open-file", url, |target| match target {   // 필터
        EventTarget::WebviewWindow { label } => label == "main" || label == "file-viewer",
        _ => false,
    }).unwrap();
}
```

Rust 측 수신:

```rust
use tauri::Listener;

let id = app.listen("download-started", |event| {
    println!("{}", event.payload());   // payload 는 JSON 문자열
});
app.once("ready", |_event| { /* 1회성 */ });
app.unlisten(id);
```

프론트:

```ts
import { listen, once, emit } from '@tauri-apps/api/event'

const unlisten = await listen<string>('download-started', (event) => {
    console.log(event.payload)
})
unlisten()

await once('ready', () => {})
await emit('frontend-event', { foo: 1 })   // 프론트 → Rust
```

**이벤트 payload 는 항상 JSON 문자열로 직렬화된다.** 공식 문서 원문: "The event system is not designed for low latency or high throughput situations."

### 3. Channel (대량/스트리밍 데이터 — IDE 핵심)

```rust
use tauri::ipc::Channel;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
enum DownloadEvent {
    Started { url: String, content_length: usize },
    Progress { chunk_length: usize },
    Finished,
}

#[tauri::command]
fn download(url: String, on_event: Channel<DownloadEvent>) {
    on_event.send(DownloadEvent::Started { url, content_length: 1000 }).unwrap();
    on_event.send(DownloadEvent::Progress { chunk_length: 150 }).unwrap();
    on_event.send(DownloadEvent::Finished).unwrap();
}
```

```ts
import { invoke, Channel } from '@tauri-apps/api/core'

type DownloadEvent =
  | { event: 'started'; data: { url: string; contentLength: number } }
  | { event: 'progress'; data: { chunkLength: number } }
  | { event: 'finished' }

const onEvent = new Channel<DownloadEvent>()
onEvent.onmessage = (message) => {
    if (message.event === 'progress') console.log(message.data.chunkLength)
}
await invoke('download', { url: 'https://…', onEvent })
```

바이너리 스트리밍(공식 예시):

```rust
#[tauri::command]
async fn load_image(path: std::path::PathBuf, reader: tauri::ipc::Channel<&[u8]>) {
    // tokio::io::AsyncReadExt 필요
    let mut file = tokio::fs::File::open(path).await.unwrap();
    let mut chunk = vec![0; 4096];
    loop {
        let len = file.read(&mut chunk).await.unwrap();
        if len == 0 { break; }
        reader.send(&chunk).unwrap();
    }
}
```

#### 대량 데이터 전송 성능 비교 (공식 문서 근거)

| 방식 | 직렬화 | 적합한 상황 |
|---|---|---|
| **Event** (`emit`/`listen`) | JSON 문자열 | 소량 알림·상태 브로드캐스트. "low latency or high throughput 용으로 설계되지 않음" |
| **Channel** (`ipc::Channel`) | 순서 보장 스트림, JSON 직렬화 회피 가능 | 다운로드 진행률, 자식 프로세스 stdout, WebSocket 메시지, **대용량 파일 청크** |
| **`ipc::Response`** (반환) | raw bytes → JS `ArrayBuffer` | 단발성 대용량 바이너리 반환 |
| **`ipc::Request` (raw body)** | raw bytes 업로드 | 대용량 업로드 |

정량 벤치마크 수치는 공식 문서에 없음 — **구체 수치는 미확인**. 원칙만 확정: 소량=Event, 스트림=Channel, 단발 대용량=Response.

raw 반환:

```rust
use tauri::ipc::Response;

#[tauri::command]
fn read_file() -> Response {
    let data = std::fs::read("/path/to/file").unwrap();
    tauri::ipc::Response::new(data)   // JS 에서 ArrayBuffer 로 수신
}
```

raw 요청:

```rust
#[tauri::command]
fn upload(request: tauri::ipc::Request) -> Result<(), Error> {
    let tauri::ipc::InvokeBody::Raw(data) = request.body() else {
        return Err(Error::RequestBodyMustBeRaw);
    };
    let Some(auth) = request.headers().get("Authorization") else {
        return Err(Error::MissingHeader("Authorization"));
    };
    Ok(())
}
```

```ts
const data = new Uint8Array([1, 2, 3])
await invoke('upload', data, { headers: { Authorization: 'apikey' } })
```

### 4. State 관리 (Manager / State / async)

```rust
use std::sync::Mutex;
use tauri::{Builder, Manager};

#[derive(Default)]
struct AppStateInner {
    open_files: Vec<String>,
    counter: u32,
}

type AppState = Mutex<AppStateInner>;   // 타입 별칭으로 이중 래핑 실수 방지

pub fn run() {
    Builder::default()
        .setup(|app| {
            app.manage(AppState::default());
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap()
}
```

커맨드에서:

```rust
#[tauri::command]
fn increase_counter(state: tauri::State<'_, AppState>) -> u32 {
    let mut s = state.lock().unwrap();
    s.counter += 1;
    s.counter
}
```

커맨드 밖(이벤트 핸들러·스레드)에서:

```rust
use tauri::{Manager, Window, WindowEvent};

fn on_window_event(window: &Window, _event: &WindowEvent) {
    let app_handle = window.app_handle();
    let state = app_handle.state::<AppState>();
    let mut state = state.lock().unwrap();
    state.counter += 1;
}
```

핵심 규칙 (공식 문서 명시):

- **`Arc` 로 감쌀 필요 없다.** Tauri 의 `State` 가 내부적으로 처리한다. 스레드로 넘길 때는 `AppHandle` 을 move 한다.
- **표준 `std::sync::Mutex` 를 기본으로 쓴다.** Tokio 문서 인용: "it is ok and often preferred to use the ordinary Mutex from the standard library in asynchronous code." `tokio::sync::Mutex` 는 **guard 를 `await` 지점을 넘겨 들고 있어야 할 때만** 쓴다.
- **타입 불일치는 컴파일 에러가 아니라 런타임 패닉이다.** `manage(Mutex::new(X))` 했는데 `State<'_, X>` 로 받으면 "state not found" 패닉. 위처럼 타입 별칭 하나로 고정한다.

async command + tokio mutex:

```rust
#[tauri::command]
async fn increase_counter(state: tauri::State<'_, tokio::sync::Mutex<AppStateInner>>) -> Result<u32, ()> {
    let mut s = state.lock().await;
    s.counter += 1;
    Ok(s.counter)
}
```

백그라운드 태스크는 `tauri::async_runtime::spawn` 으로 띄운다(Tauri 내장 Tokio 런타임 사용):

```rust
tauri::async_runtime::spawn(async move {
    // AppHandle 을 move 해서 emit 가능
});
```

### 5. Sidecar (외부 바이너리 번들)

`src-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "externalBin": [
      "binaries/my-sidecar"
    ]
  }
}
```

상대 경로는 `tauri.conf.json`(= `src-tauri`) 기준. 실제 파일은 **target triple 접미사**가 붙어야 한다:

- `binaries/my-sidecar-aarch64-apple-darwin`
- `binaries/my-sidecar-x86_64-apple-darwin`
- `binaries/my-sidecar-x86_64-pc-windows-msvc.exe`

triple 확인:

```sh
rustc --print host-tuple
# 구버전 Rust
rustc -Vv | grep host | cut -f2 -d' '
```

리네임 스크립트 예시(공식):

```js
import { execSync } from 'child_process'
import fs from 'fs'

const extension = process.platform === 'win32' ? '.exe' : ''
const targetTriple = execSync('rustc --print host-tuple').toString().trim()
fs.renameSync(
  `src-tauri/binaries/sidecar${extension}`,
  `src-tauri/binaries/sidecar-${targetTriple}${extension}`,
)
```

Rust 에서 실행 (shell 플러그인 필요):

```rust
use tauri::Emitter;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;

let sidecar_command = app.shell().sidecar("my-sidecar").unwrap();
let (mut rx, mut child) = sidecar_command.spawn().expect("Failed to spawn sidecar");

tauri::async_runtime::spawn(async move {
    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stdout(line_bytes) = event {
            let line = String::from_utf8_lossy(&line_bytes);
            app.emit("message", Some(format!("'{}'", line))).expect("failed to emit event");
            child.write("message from Rust\n".as_bytes()).unwrap();
        }
    }
});
```

`sidecar()` 인자는 **경로 전체가 아니라 파일명만**("my-sidecar").

인자 전달 시 capability 에 화이트리스트 필요:

```json
{
  "permissions": [
    {
      "identifier": "shell:allow-execute",
      "allow": [
        {
          "name": "binaries/my-sidecar",
          "sidecar": true,
          "args": ["arg1", "-a", "--arg2", { "validator": "\\S+" }]
        }
      ]
    }
  ]
}
```

```rust
let sidecar_command = app.shell().sidecar("my-sidecar").unwrap()
    .args(["arg1", "-a", "--arg2", "any-string-that-matches-the-validator"]);
let (mut _rx, mut _child) = sidecar_command.spawn().unwrap();
```

JS:

```ts
import { Command } from '@tauri-apps/plugin-shell'
const command = Command.sidecar('binaries/my-sidecar', ['arg1', '-a', '--arg2', 'x'])
const output = await command.execute()
```

JS 쪽 `Command.sidecar` 에 넘기는 문자열은 **`externalBin` 배열에 적은 문자열과 정확히 일치**해야 한다(Rust 쪽은 파일명만, JS 쪽은 설정 문자열 그대로 — 비대칭이니 주의).

### 6. Capabilities / Permissions 보안 모델

파일 위치: `src-tauri/capabilities/*.json`(또는 `.toml`).

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "main-capability",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:path:default",
    "core:event:default",
    "core:window:default",
    "core:app:default",
    "core:resources:default",
    "core:menu:default",
    "core:tray:default",
    "core:window:allow-set-title"
  ]
}
```

주요 필드:

| 필드 | 의미 |
|---|---|
| `identifier` | capability 고유 이름 |
| `description` | 설명 |
| `windows` | 적용 윈도우 라벨 배열(`["*"]` glob 가능) |
| `webviews` | 적용 웹뷰 라벨 |
| `permissions` | 부여할 permission 식별자 배열(스코프 객체 포함 가능) |
| `platforms` | `linux` / `macOS` / `windows` / `iOS` / `android` |
| `remote.urls` | 원격 URL 이 커맨드를 호출하도록 허용(glob 가능) |
| `local` | 로컬 컨텐츠 대상 여부 |

플랫폼 한정:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "desktop-capability",
  "windows": ["main"],
  "platforms": ["linux", "macOS", "windows"],
  "permissions": ["global-shortcut:allow-register"]
}
```

원격 허용:

```json
{
  "$schema": "../gen/schemas/remote-schema.json",
  "identifier": "remote-tag-capability",
  "windows": ["main"],
  "remote": { "urls": ["https://*.tauri.app"] },
  "permissions": ["nfc:allow-scan"]
}
```

`tauri.conf.json` 참조 / 인라인 혼용:

```json
{
  "app": {
    "security": {
      "capabilities": [
        {
          "identifier": "my-capability",
          "description": "My application capability used for all windows",
          "windows": ["*"],
          "permissions": ["fs:default", "allow-home-read-extended"]
        },
        "my-second-capability"
      ]
    }
  }
}
```

`capabilities` 배열이 **비어 있으면 `src-tauri/capabilities/` 의 모든 파일이 자동 적용**되고, 값이 있으면 지정한 것만 적용된다.

permission 식별자 규칙: `<plugin>:<permission>` (코어는 `core:<module>:<permission>`). 관례상 `default` / `allow-*` / `deny-*`.

스코프가 붙는 permission 은 객체 형태로 쓴다:

```json
{
  "identifier": "main-capability",
  "windows": ["main"],
  "permissions": [
    "core:default",
    {
      "identifier": "fs:allow-read-text-file",
      "allow": [{ "path": "$HOME/projects/**" }],
      "deny": [{ "path": "$HOME/projects/**/.env" }]
    }
  ]
}
```

`deny` 가 `allow` 보다 항상 우선한다.

경고(공식): 하나의 윈도우가 여러 capability 에 속하면 **권한이 합집합으로 병합**된다. IDE 처럼 신뢰도가 다른 윈도우(에디터 vs 프리뷰)를 둘 땐 라벨을 분리하고 capability 도 분리해야 실효가 있다.

capability 는 프론트엔드 침해 영향을 줄이는 장치일 뿐, **악의적 Rust 코드나 잘못 설정한 스코프는 막지 못한다**(공식 명시).

### 7. tauri-specta (Rust → TS 타입 자동생성)

**상태: 2.x 는 여전히 release candidate(2.0.0-rc.25). stable 없음.** 채택 시 이 점을 감수해야 한다.

`Cargo.toml`:

```toml
[dependencies]
tauri = { version = "2", features = [] }
specta = { version = "=2.0.0-rc.25", features = ["derive"] }
specta-typescript = "0.0.12"
tauri-specta = { version = "=2.0.0-rc.25", features = ["derive", "typescript"] }
serde = { version = "1", features = ["derive"] }
thiserror = "2"
```

`lib.rs` / `main.rs`:

```rust
use serde::{Deserialize, Serialize};
use specta::Type;
use specta_typescript::Typescript;
use tauri_specta::{collect_commands, collect_events, Builder};

#[tauri::command]
#[specta::specta]
fn hello_world(my_name: String) -> String {
    format!("Hello, {my_name}!")
}

#[tauri::command]
#[specta::specta]
async fn async_hello_world(my_name: String) -> String {
    format!("Hello, {my_name}!")
}

// 타입 안전한 에러
#[derive(thiserror::Error, Debug, Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum MyError {
    #[error("io error: {0}")]
    IoError(#[serde(skip)] #[from] std::io::Error),
    #[error("some other error: {0}")]
    AnotherError(String),
}

#[tauri::command]
#[specta::specta]
fn may_fail() -> Result<(), MyError> {
    Err(MyError::AnotherError("oh no".into()))
}

// 타입 안전한 이벤트
#[derive(Serialize, Deserialize, Debug, Clone, Type, tauri_specta::Event)]
#[tauri_specta(event_name = "fileChanged")]
pub struct FileChanged(String);

pub fn run() {
    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![hello_world, async_hello_world, may_fail])
        .events(collect_events![FileChanged])
        .constant("universalConstant", 42);

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/bindings.ts")
        .expect("Failed to export typescript bindings");

    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);      // 이벤트 사용 시 필수
            FileChanged::listen(app, |e| { dbg!(e.payload); });
            FileChanged("a.ts".into()).emit(app).ok();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

프론트:

```ts
import { commands, events } from './bindings'

const msg = await commands.helloWorld('Brendan')

const res = await commands.mayFail()
if (res.status === 'error') console.error(res.error)   // Result 는 태그드 유니온으로 생성됨

await events.fileChanged.listen((e) => console.log(e.payload))
```

추가 기능(rc.24/rc.25 시점 확인):

- `.semantic_types(specta_typescript::semantic::Configuration::default())` — `chrono::DateTime` → `Date`, `bytes::Bytes` → `Uint8Array`, `url::Url` → `URL` 매핑.
- **phase-specific types** — `#[serde(rename(serialize = "...", deserialize = "..."))]` 같이 직렬화/역직렬화 이름이 다른 경우를 정확히 반영. 끄려면 `.disable_serde_phases()`.
- 출력 레이아웃: `Typescript::default().layout(Layout::Files | Layout::Namespaces)`.
- `JSDoc::default()` 로 JS + JSDoc 출력(`javascript` feature).
- `tauri::ipc::Channel<T>` 도 시그니처에서 타입 생성됨.
- `crates/tauri-specta-query` — TanStack Query 연동 크레이트가 리포에 존재. **crates.io 공개 여부/버전은 미확인.**

대안: `taurpc`(리포지토리 존재 확인, **버전·유지보수 상태 미확인**). tauri-specta 를 안 쓸 경우 커맨드 시그니처를 손으로 `.ts` 에 미러링하는 방식이 된다.

### 8. Vite 통합

`vite.config.ts` (공식 템플릿 그대로):

```ts
import { defineConfig } from 'vite'

const host = process.env.TAURI_DEV_HOST

export default defineConfig({
    // prevent vite from obscuring rust errors
    clearScreen: false,
    server: {
        port: 5173,
        strictPort: true,
        host: host || false,
        hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
        watch: { ignored: ['**/src-tauri/**'] },
    },
    envPrefix: ['VITE_', 'TAURI_ENV_*'],
    build: {
        target: process.env.TAURI_ENV_PLATFORM == 'windows' ? 'chrome105' : 'safari13',
        minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
        sourcemap: !!process.env.TAURI_ENV_DEBUG,
    },
})
```

`src-tauri/tauri.conf.json`:

```json
{
  "build": {
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../dist"
  }
}
```

CLI 가 주입하는 환경변수: `TAURI_ENV_PLATFORM`, `TAURI_ENV_ARCH`, `TAURI_ENV_FAMILY`, `TAURI_ENV_DEBUG`, `TAURI_DEV_HOST`.

### 9. 멀티윈도우

설정 파일 방식 (`tauri.conf.json` → `app.windows` 배열):

```json
{
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "TAIDE",
        "width": 1400,
        "height": 900,
        "decorations": false,
        "url": "index.html"
      }
    ]
  }
}
```

Rust 런타임 생성:

```rust
use tauri::{TitleBarStyle, WebviewUrl, WebviewWindowBuilder};

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let win_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
                .title("Transparent Titlebar Window")
                .inner_size(800.0, 600.0);

            #[cfg(target_os = "macos")]
            let win_builder = win_builder.title_bar_style(TitleBarStyle::Transparent);

            let window = win_builder.build().unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

라벨로 접근은 `Manager` 트레잇의 `get_webview_window(label)` 을 쓴다(반환 `Option<WebviewWindow>`). 프론트에서는:

```ts
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

const w = new WebviewWindow('editor-2', { url: '/editor', title: 'Editor 2' })
w.once('tauri://created', () => {})
w.once('tauri://error', (e) => console.error(e))

const existing = await WebviewWindow.getByLabel('main')
```

커스텀 타이틀바:

- `"decorations": false`
- 드래그 영역은 HTML 요소에 `data-tauri-drag-region` 속성. **직접 적용한 요소에만 동작**하며 자식으로 상속되지 않는다.
- 세밀한 제어는 `appWindow.startDragging()` 을 마우스다운 핸들러에서 호출.
- macOS 는 `TitleBarStyle::Transparent` + 신호등 버튼 유지가 완전 커스텀보다 낫다(스냅·풀스크린 등 네이티브 동작 보존).

메뉴:

```rust
use tauri::menu::MenuBuilder;

let menu = MenuBuilder::new(app)
    .text("open", "Open")
    .copy()
    .separator()
    .build()?;
app.set_menu(menu)?;
app.on_menu_event(move |_app, event| {
    match event.id().0.as_str() {
        "open" => println!("Open clicked"),
        _ => {}
    }
});
```

macOS 는 모든 아이템이 `SubmenuBuilder` 안에 있어야 하며 첫 서브메뉴가 앱 메뉴가 된다. 아이콘 메뉴 아이템(`Image` 기반)은 **Tauri 2.8.0+**.

### 10. 빌드·서명·배포

#### macOS

1. Apple Developer 계정 필요(공증은 유료 계정 필수, $99/년).
2. CSR 생성 → Apple Developer 사이트에서 인증서 발급.
   - App Store 배포: **Apple Distribution**
   - 외부 배포(IDE 는 보통 이쪽): **Developer ID Application**
3. `.cer` 다운로드 후 더블클릭해 키체인에 설치.
4. identity 확인: `security find-identity -v -p codesigning`
5. 설정:

```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application: Your Name (TEAMID)"
    }
  }
}
```

또는 `APPLE_SIGNING_IDENTITY` 환경변수.

6. CI: `.p12` 내보내기 → `openssl base64 -A -in certificate.p12 -out certificate-base64.txt` → `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD` 시크릿 등록.
7. 공증(둘 중 하나):
   - App Store Connect API: `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`
   - Apple ID: `APPLE_ID`, `APPLE_PASSWORD`(앱 전용 비밀번호), `APPLE_TEAM_ID`
8. 빌드: `pnpm tauri build --bundles dmg`
9. 개발용 임시 서명은 `"signingIdentity": "-"` (Apple Silicon 에서 실행은 되지만 Gatekeeper 경고는 남는다).

`hardenedRuntime`, `entitlements`, `providerShortName`, universal binary 타깃(`--target universal-apple-darwin`)에 대한 상세 필드 설명은 이번 조사에서 **원문 확인 실패 — 미확인** (config 레퍼런스 `bundle.macOS` 섹션 확인 필요).

#### Windows

- **EV 인증서**: Microsoft SmartScreen 즉시 신뢰. 경고 없음.
- **OV 인증서**: 저렴하지만 SmartScreen 경고가 초기에 뜬다.
- 2023-06-01 이후 발급 인증서는 하드웨어 토큰/HSM 보관이 강제되어, 전통적 `certificateThumbprint` 방식은 그 이전 발급분에 해당한다.

전통적 OV 방식:

```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "A1B2C3...",
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

HSM/클라우드 방식(현재 주류):

- **Azure Key Vault** + `relic` (`relic.conf` 작성 후 `bundle.windows.signCommand` 지정)
- **Azure Trusted Signing / Artifact Signing** + `artifact-signing-cli`, Azure client id/secret/tenant 환경변수

`signCommand` 는 `%1` 플레이스홀더에 서명 대상 파일 경로가 들어가는 임의 명령을 지정하는 방식이다.

CI 는 base64 시크릿을 PowerShell 로 디코드해 인증서 저장소에 임포트한 뒤 빌드한다.

#### 업데이터

```bash
pnpm tauri signer generate -- -w ~/.tauri/taide.key
```

```bash
export TAURI_SIGNING_PRIVATE_KEY="키 파일 경로 또는 내용"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
```

```json
{
  "bundle": { "createUpdaterArtifacts": true },
  "plugins": {
    "updater": {
      "pubkey": "PUBLIC_KEY_CONTENT",
      "endpoints": ["https://releases.example.com/{{target}}/{{arch}}/{{current_version}}"],
      "windows": { "installMode": "passive" }
    }
  }
}
```

정적 JSON 응답:

```json
{
  "version": "2.0.0",
  "notes": "Update notes",
  "pub_date": "2026-08-06T00:00:00Z",
  "platforms": {
    "darwin-aarch64": { "url": "https://...", "signature": "SIGNATURE_CONTENT" },
    "windows-x86_64": { "url": "https://...", "signature": "SIGNATURE_CONTENT" }
  }
}
```

동적 서버는 업데이트 없음이면 **HTTP 204**, 있으면 200 + 위 형식.

```ts
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'

const update = await check()
if (update) {
    await update.downloadAndInstall()
    await relaunch()
}
```

capability: `"permissions": ["updater:default"]` (+ 재시작 위해 `process:allow-restart`).

### 11. 대용량 파일 스트리밍 + notify 파일 와처 브릿지

#### 대용량 파일 읽기

3가지 선택지 (앞의 성능 표 참조):

```rust
// (a) 단발 대용량 — ArrayBuffer 로 한 번에
#[tauri::command]
fn read_file_bytes(path: String) -> tauri::ipc::Response {
    let data = std::fs::read(path).unwrap();
    tauri::ipc::Response::new(data)
}

// (b) 청크 스트리밍 — Channel
use tokio::io::AsyncReadExt;

#[tauri::command]
async fn stream_file(path: std::path::PathBuf, reader: tauri::ipc::Channel<&[u8]>) -> Result<(), String> {
    let mut file = tokio::fs::File::open(path).await.map_err(|e| e.to_string())?;
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let len = file.read(&mut chunk).await.map_err(|e| e.to_string())?;
        if len == 0 { break; }
        reader.send(&chunk[..len]).map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

```ts
const chunks: Uint8Array[] = []
const ch = new Channel<ArrayBuffer>()
ch.onmessage = (buf) => chunks.push(new Uint8Array(buf))
await invoke('stream_file', { path, reader: ch })
```

(c) 커스텀 프로토콜(`asset://` / `register_uri_scheme_protocol`)로 HTTP Range 응답을 주는 방식 — 에디터에서 미디어/거대 로그를 뷰어에 물릴 때 유리. **Range 처리 구현 세부는 이번 조사에서 원문 미확인.**

#### notify → Tauri 이벤트 브릿지

`Cargo.toml`:

```toml
notify = "8.2"
```

`src-tauri/src/watcher.rs`:

```rust
use std::path::Path;
use std::sync::Mutex;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsChange {
    kind: String,
    paths: Vec<String>,
}

pub struct WatcherState(pub Mutex<Option<RecommendedWatcher>>);

#[tauri::command]
pub fn watch_workspace(app: AppHandle, path: String) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
    let mut watcher = RecommendedWatcher::new(tx, Config::default()).map_err(|e| e.to_string())?;
    watcher
        .watch(Path::new(&path), RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    // watcher 를 state 에 살려둔다 — drop 되면 감시가 즉시 중단된다
    let state = app.state::<WatcherState>();
    *state.0.lock().unwrap() = Some(watcher);

    let app_handle = app.clone();
    std::thread::spawn(move || {
        for res in rx {
            match res {
                Ok(event) => {
                    let payload = FsChange {
                        kind: format!("{:?}", event.kind),
                        paths: event.paths.iter().map(|p| p.display().to_string()).collect(),
                    };
                    let _ = app_handle.emit("fs://change", payload);
                }
                Err(e) => eprintln!("watch error: {e:?}"),
            }
        }
    });

    Ok(())
}
```

등록:

```rust
tauri::Builder::default()
    .manage(WatcherState(Mutex::new(None)))
    .invoke_handler(tauri::generate_handler![watch_workspace])
```

프론트:

```ts
import { listen } from '@tauri-apps/api/event'

await invoke('watch_workspace', { path: workspaceRoot })
await listen<{ kind: string; paths: string[] }>('fs://change', (e) => {
    refreshTree(e.payload.paths)
})
```

고빈도 변경(빌드 산출물·`node_modules`)이 많으면 **`emit` 대신 Channel + Rust 측 디바운스**로 바꾼다. notify 8.x 계열에는 디바운서(`notify-debouncer-full`)가 별도 크레이트로 존재하나 **버전은 미확인** — 사용 전 crates.io 확인 필요. 직접 구현할 경우 100~300ms 창으로 경로 집합을 모아 한 번에 보낸다.

`RecommendedWatcher` 는 macOS 에서 FSEvents, Windows 에서 ReadDirectoryChangesW, Linux 에서 inotify 를 쓴다.

---

## TAIDE 적용 가이드

### 스택 픽스 제안

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-fs = "2"
tauri-plugin-dialog = "2"
tauri-plugin-shell = "2"
tauri-plugin-store = "2"
tauri-plugin-log = "2"
tauri-plugin-window-state = "2"
tauri-plugin-opener = "2"
tauri-plugin-process = "2"
tauri-plugin-os = "2"
tauri-plugin-global-shortcut = "2"
tokio = { version = "1", features = ["fs", "io-util", "process", "sync"] }
notify = "8.2"
serde = { version = "1", features = ["derive"] }
thiserror = "2"

[build-dependencies]
tauri-build = "2"
```

데스크톱 IDE 라면 **single-instance + deep-link + window-state + updater** 조합이 사실상 필수다.

### 플러그인 선정 (IDE 관점)

| 용도 | 플러그인 |
|---|---|
| 워크스페이스 파일 읽기/쓰기 | `fs` (+ `persisted-scope` 로 사용자가 연 폴더 스코프 영속화) |
| 폴더 열기 다이얼로그 | `dialog` |
| 터미널·LSP·포매터 프로세스 스폰 | `shell` (sidecar 포함) |
| 에디터 설정·최근 프로젝트 | `store` |
| 진단 로그 | `log` |
| 창 크기/위치 복원 | `window-state` |
| 자동 업데이트 | `updater` + `process`(relaunch) |
| `taide://` URL 로 파일 열기 | `deep-link` |
| 중복 실행 방지 + 인자 전달 | `single-instance` |
| 외부 브라우저/파일 매니저 열기 | `opener` |
| 키맵 전역 단축키 | `global-shortcut` |
| 플랫폼 분기 | `os` |

`persisted-scope` 는 런타임에 추가된 fs 스코프를 디스크에 보존해준다 — "폴더 열기" 로 확장된 권한이 재시작 후에도 유지되어야 하는 IDE 에 정확히 맞는다.

### IPC 채널 설계 규칙

| 데이터 | 방식 |
|---|---|
| 파일 열기/저장, 설정 CRUD | 일반 command (`Result<T, E>`) |
| 파일 트리 스캔 결과(수천 항목) | command + 한 번에 반환 (JSON 도 이 크기는 감당) |
| 대용량 파일 본문(> 수 MB) | `ipc::Response` 또는 `Channel<&[u8]>` |
| LSP 메시지, 터미널 출력, 빌드 로그 | **`Channel<T>`** (세션당 1개) |
| 파일 시스템 변경 알림 | 디바운스 후 `Channel` 또는 `emit` |
| 앱 전역 상태 변화(테마, 프로젝트 전환) | `emit` (저빈도) |

원칙: **세션/스트림 성격 = Channel, 브로드캐스트 알림 = Event.**

### State 설계

```rust
pub struct AppStateInner {
    pub workspace_root: Option<PathBuf>,
    pub open_buffers: HashMap<BufferId, Buffer>,
}
pub type AppState = Mutex<AppStateInner>;

// LSP 세션처럼 await 를 넘겨 락을 들고 있어야 하는 것만 분리
pub type LspState = tokio::sync::Mutex<LspSessions>;
```

동기 상태와 async 상태를 **하나의 Mutex 에 섞지 않는다.** 짧은 동기 접근은 `std::sync::Mutex`, `await` 가 끼는 세션 관리만 `tokio::sync::Mutex`.

### 타입 안전성

tauri-specta 채택 여부를 초기에 결정한다. 커맨드가 수십 개를 넘어가면 손 미러링은 무너진다. RC 상태가 부담이면 대안은 (a) 커맨드 시그니처를 얇게 유지하고 payload 타입만 수동 공유, (b) `zod` 스키마를 프론트 경계에 두고 런타임 파싱. **RC 를 쓸 경우 버전을 `=` 로 핀 고정**한다(specta 생태계는 rc 간 breaking 이 잦다).

### 보안 경계

에디터 윈도우와 프리뷰/확장 윈도우를 라벨로 분리하고 capability 를 분리한다.

```
capabilities/
  main.json        → identifier: "main", windows: ["main"],  fs 전체 워크스페이스 스코프
  preview.json     → identifier: "preview", windows: ["preview-*"], core:default 만
```

프리뷰 웹뷰에 `fs:default` 를 주지 않는다. 렌더링할 마크다운/HTML 은 신뢰 불가 입력이다.

---

## 함정·주의

1. **tauri-specta 2.x 는 stable 이 아니다** (rc.25). crates.io 의 `max_stable_version` 은 1.0.2 = Tauri v1 용이라 `cargo add tauri-specta` 하면 **v1 용이 설치된다.** 반드시 `--version =2.0.0-rc.25` 로 지정.
2. **tauri-specta main 브랜치와 crates.io 릴리즈가 어긋나 있다.** main 은 미공개 `specta 2.0.0-rc.26` 을 `[patch.crates-io]` 로 참조 중이라 GitHub README/예제 코드를 그대로 복사하면 빌드가 깨질 수 있다. rc.25 는 `specta =2.0.0-rc.25` + `specta-typescript ^0.0.12` 조합.
3. **State 타입 불일치는 런타임 패닉.** `manage(Mutex::new(X))` ↔ `State<'_, Mutex<X>>` 를 타입 별칭으로 고정.
4. **async command 에 borrowed 인자를 쓰려면 반환이 `Result` 여야 한다.** `State<'_, T>` 도 borrowed 이므로 async command 는 사실상 항상 `Result` 반환.
5. **Event payload 는 JSON 문자열 직렬화.** 큰 배열/바이너리를 `emit` 으로 보내면 성능이 급격히 나빠진다. 공식 문서가 명시적으로 부적합하다고 서술.
6. **`Channel` 은 반드시 `invoke` 인자로 넘겨 생성된 것을 써야 한다.** JS 에서 만든 `Channel` 인스턴스를 여러 커맨드에 재사용하는 패턴은 검증 필요 — **미확인**.
7. **notify `Watcher` 는 drop 되는 순간 감시가 끊긴다.** 커맨드 로컬 변수로 두면 즉시 종료된다. 반드시 Tauri state 로 소유권을 잡아둔다.
8. **sidecar 이름 규칙 비대칭**: Rust `shell().sidecar("my-sidecar")` 는 파일명만, JS `Command.sidecar('binaries/my-sidecar')` 는 `externalBin` 설정 문자열 그대로.
9. **sidecar 는 target triple 접미사 없이는 번들되지 않는다.** 크로스 플랫폼 릴리즈는 CI 에서 플랫폼별로 바이너리를 준비/리네임해야 한다.
10. **capability 병합**: 한 윈도우가 여러 capability 에 걸리면 권한이 합쳐진다. 좁히려고 파일을 추가했는데 오히려 넓어질 수 있다.
11. **`tauri.conf.json` 의 `app.security.capabilities` 를 비워두면 `capabilities/` 전체가 적용**된다. 명시적으로 나열해 통제하는 편이 안전하다.
12. **`data-tauri-drag-region` 은 상속되지 않는다.** 타이틀바 컨테이너에만 붙이면 자식 위에서는 드래그가 안 된다.
13. **macOS 완전 커스텀 타이틀바는 스냅/풀스크린 등 네이티브 동작을 잃는다.** `TitleBarStyle::Transparent` 권장.
14. **Vite `build.target` 을 기본값으로 두면 macOS/Linux WebKit 에서 깨진다.** 위 템플릿의 `safari13` / `chrome105` 분기를 유지할 것.
15. **`server.strictPort: true` 필수.** 포트가 밀리면 Tauri 가 dev 서버를 찾지 못한다.
16. **`watch.ignored: ['**/src-tauri/**']`** 를 빼면 Rust 빌드 산출물 변경마다 Vite 가 리로드를 돌린다.
17. **업데이터 개인키를 `.env` 에 넣지 말 것** (공식 경고). 분실 시 이후 업데이트 발행 불가.
18. **Windows 코드서명은 2023-06-01 이후 발급 인증서부터 HSM 보관 강제.** `certificateThumbprint` 로컬 방식은 그 이전 인증서에만 해당하며, 신규는 Azure Trusted Signing 등 `signCommand` 경로를 써야 한다.
19. **macOS 공증은 유료 Apple Developer 계정 필수.** 무료 계정은 서명만 되고 공증 불가 → 사용자에게 Gatekeeper 경고.
20. **`generate_handler!` 는 앱당 1회.** 커맨드를 모듈로 쪼개도 호출은 한 번에 모아야 한다.
21. **커맨드 이름은 전역 유일.** 모듈이 달라도 동명 커맨드는 컴파일 에러.
22. **버전별 상세 변경 로그는 공식 블로그에 없다.** 2.x 업그레이드 시 GitHub 릴리즈 노트를 직접 확인해야 한다.

---

## 참고 링크

- Tauri 릴리즈 버전 표: https://v2.tauri.app/release/
- Tauri 블로그: https://v2.tauri.app/blog/
- Calling Rust from the Frontend (commands): https://v2.tauri.app/develop/calling-rust/
- Calling the Frontend from Rust (events, Channel): https://v2.tauri.app/develop/calling-frontend/
- State Management: https://v2.tauri.app/develop/state-management/
- Embedding External Binaries (sidecar): https://v2.tauri.app/develop/sidecar/
- Capabilities: https://v2.tauri.app/security/capabilities/
- Permissions: https://v2.tauri.app/security/permissions/
- Plugin 목록: https://v2.tauri.app/plugin/
- Updater 플러그인: https://v2.tauri.app/plugin/updater/
- Vite 통합: https://v2.tauri.app/start/frontend/vite/
- Window Customization: https://v2.tauri.app/learn/window-customization/
- Window Menu: https://v2.tauri.app/learn/window-menu/
- macOS 서명/공증: https://v2.tauri.app/distribute/sign/macos/
- Windows 서명: https://v2.tauri.app/distribute/sign/windows/
- tauri-specta 리포: https://github.com/specta-rs/tauri-specta
- tauri-specta 예제(app): https://github.com/specta-rs/tauri-specta/tree/main/examples/app
- tauri-specta docs.rs (rc.21 이 마지막 성공 빌드): https://docs.rs/tauri-specta/2.0.0-rc.21/tauri_specta/index.html
- specta: https://specta.dev/ , https://github.com/specta-rs/specta
- notify (crates.io): https://crates.io/crates/notify
- Tauri crates.io: https://crates.io/crates/tauri
- Tauri GitHub (릴리즈 노트 원본): https://github.com/tauri-apps/tauri/releases
