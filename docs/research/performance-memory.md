# Tauri 기반 IDE 의 성능·메모리 최적화와 누수 방지

조사일 2026-08-06. 아래 버전은 crates.io / npm registry API 를 직접 조회해 확정한 값이다. 확인하지 못한 항목은 `미확인` 으로 표기했다.

---

## 버전 확정 (2026-08 기준)

| 대상 | 버전 | 확인 방법 |
|---|---|---|
| `tauri` (crate) | **2.11.5** (stable, 2026-07-01 릴리스) | crates.io API |
| `@tauri-apps/api` (npm) | **2.11.1** | npm registry |
| `notify` (crate) | stable **8.2.0** (2025-08-03) / pre **9.0.0-rc.4** (2026-05-02) | crates.io API |
| `notify-debouncer-full` | stable **0.7.0** (2026-01-23) / pre **0.8.0-rc.2** (2026-05-02) | crates.io API. MSRV 1.85 |
| `monaco-editor` | **0.56.0** | npm registry |
| `@xterm/xterm` | **6.0.0** (latest) / **6.1.0-beta.292** (beta) | npm dist-tags |
| `@tanstack/react-virtual` | **3.14.9** | npm registry |
| WebView (Windows) | WebView2 (Chromium/Edge 기반), Windows 11 기본 탑재 | Tauri v2 docs |
| WebView (macOS/iOS) | WKWebView, macOS 10.10+ 동봉 webview, OS 업데이트에 종속 | Tauri v2 docs |
| WebView (Linux) | WebKitGTK (Tauri v2 는 `webkit2gtk-4.1` 사용). 배포판별 편차 큼(2.20~2.36 언급) | Tauri v2 docs. **정확한 최소 버전 명시는 미확인** |
| `@xterm/addon-webgl`, `@xterm/addon-fit` 등 애드온 버전 | **미확인** (개별 조회 안 함) | - |
| `portable-pty` (wezterm-pty) 최신 버전 | **미확인** | - |

주의: `notify` 9.x 와 `notify-debouncer-full` 0.8.x 는 2026-08 시점에도 **rc 단계**다. TAIDE 는 stable 조합(`notify 8.2` + `notify-debouncer-full 0.7`)으로 시작하고, 9.0 정식 릴리스 후 마이그레이션하는 편이 안전하다. 두 crate 는 버전이 짝지어져야 하므로(디바운서가 notify 를 re-export) 반드시 함께 올린다.

---

## 핵심 API·사용법

### 1. WebView 프로세스 모델과 메모리 특성

Tauri 공식 문서(Process Model)는 Electron/브라우저와 유사한 **멀티프로세스 구조**임을 명시한다.

- **Core 프로세스**: Rust. 앱 진입점, 창/트레이/알림 생성, **모든 IPC 라우팅**, 전역 상태(설정·DB 커넥션) 보유. OS 전권.
- **WebView 프로세스**: Core 가 스폰. UI 를 직접 그리지 않고 OS 제공 WebView 라이브러리를 구동. WebView 라이브러리는 **최종 바이너리에 포함되지 않고 런타임에 동적 링크**된다.

플랫폼별 특성(문서 + 이슈 트래커 기준):

| 플랫폼 | 엔진 | 프로세스 특성 | 메모리 관점 함의 |
|---|---|---|---|
| Windows | WebView2 (Chromium) | Chromium 프로세스 모델을 따름. **WebView2 컨트롤당 대체로 renderer 프로세스 1개**(브라우저 탭과 유사). 여러 WebView2 앱이 브라우저 프로세스를 공유할 수 있음 | 창을 늘리면 renderer 프로세스가 늘어 RSS 가 계단식 증가. 창을 여러 개 두는 IDE 레이아웃보다 **단일 창 + 내부 패널** 이 메모리상 유리 |
| macOS | WKWebView (WebKit) | WKWebView 는 out-of-process 렌더링(`com.apple.WebKit.WebContent` 별도 프로세스). OS 와 함께 업데이트되므로 **버전을 앱이 고정할 수 없음** | Activity Monitor 에서 앱 RSS 만 보면 실제 사용량을 과소평가한다. WebContent/Networking 자식 프로세스를 합산해야 함 |
| Linux | WebKitGTK | UI 프로세스 + `WebKitWebProcess`(+ NetworkProcess) 분리. DMA-BUF 렌더러로 UI/Web 프로세스 간 버퍼 공유 | 드라이버 조합에 따라 `WebKitWebProcess` 가 CPU/메모리를 크게 먹는 보고 존재. 트러블슈팅 환경변수 `WEBKIT_DISABLE_DMABUF_RENDERER=1`, 최후수단 `WEBKIT_DISABLE_COMPOSITING_MODE=1` (렌더 성능 희생) |

Electron 대비: "2026 벤치마크에서 Tauri 가 동등 Electron 빌드의 약 절반 RSS" 라는 2차 자료 서술이 있으나 **1차 출처로 검증하지 못했다(미확인)**. 근거로 제시된 구조적 이유(별도 Chromium 프로세스 트리 없음, renderer 당 V8 isolate 없음, Node main 프로세스 없음)는 타당하다.

### 2. 프론트 누수 방지 패턴

#### 2-1. Tauri `listen()` / `unlisten`

공식 문서의 핵심 경고 두 가지:

1. "Always use the unlisten function when your execution context goes out of scope such as when a component is unmounted."
2. `listen()` 은 **Promise** 다. resolve 전에 `unlisten()` 을 호출하면 등록이 취소되지 않는다.

```typescript
// 잘못된 예 — 리스너가 아직 등록되지 않아 해제되지 않는다
const unlisten = listen('sync-complete', () => {})
unlisten()

// 올바른 예
const unlisten = await listen('sync-complete', () => {})
unlisten()
```

React 공식 예제(문서 원문):

```typescript
useEffect(() => {
    const unlisten = listen<number>('download-progress', (event) => {
        setProgress(event.payload)
    })

    return () => {
        unlisten.then((fn) => fn())
    }
}, [])
```

React 18+ StrictMode 는 effect 를 mount → unmount → mount 로 두 번 돌리므로, 위 패턴이 **비동기 경쟁**을 만든다. TAIDE 는 취소 플래그를 넣은 헬퍼로 통일한다.

```typescript
// shared/lib/use-tauri-event.ts
import { useEffect } from 'react'
import { listen, type EventCallback } from '@tauri-apps/api/event'

export const useTauriEvent = <T,>(event: string, handler: EventCallback<T>) => {
    const handlerRef = useRef(handler)
    handlerRef.current = handler

    useEffect(() => {
        let disposed = false
        let unlisten: (() => void) | undefined

        listen<T>(event, (e) => handlerRef.current(e)).then((fn) => {
            if (disposed) {
                fn()
                return
            }
            unlisten = fn
        })

        return () => {
            disposed = true
            unlisten?.()
        }
    }, [event])
}
```

`handlerRef` 를 쓰는 이유: 핸들러를 deps 에 넣으면 렌더마다 리스너를 재등록해 IPC 브릿지에 등록/해제가 폭주한다. 이벤트명만 deps 로 둔다.

관련 알려진 이슈: Next.js + Tauri 에서 새로고침 시 `unlisten` 이 undefined 가 되어 리스너가 누적된다는 보고(tauri-apps/tauri#8913). 위 취소 플래그 패턴이 이를 함께 막는다.

#### 2-2. 이벤트 대신 Channel (고throughput)

문서 원문: "The event system is not designed for low latency or high throughput situations." / "Channels are designed to be fast and deliver ordered data."

터미널 출력, 파일 검색 스트림, LSP 알림처럼 초당 수천 건이 흐르는 경로는 **반드시 Channel** 로 간다.

```rust
use serde::Serialize;
use tauri::{ipc::Channel, AppHandle};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "event", content = "data")]
enum PtyEvent {
    Output { pty_id: u32, chunk: String },
    Exit { pty_id: u32, code: i32 },
}

#[tauri::command]
fn attach_pty(app: AppHandle, pty_id: u32, on_event: Channel<PtyEvent>) {
    on_event.send(PtyEvent::Output { pty_id, chunk: "hello".into() }).unwrap();
}
```

```typescript
import { invoke, Channel } from '@tauri-apps/api/core'

const onEvent = new Channel<PtyEvent>()
onEvent.onmessage = (message) => { /* ... */ }
await invoke('attach_pty', { ptyId, onEvent })
```

Channel 은 컴포넌트 unmount 시 Rust 쪽에서 해당 구독을 끊는 command(`detach_pty`)를 반드시 함께 호출해야 한다. 프론트에서 참조를 버리는 것만으로 Rust 측 sender 가 drop 되지 않는다.

#### 2-3. Monaco model dispose

Monaco 는 `monaco.editor.createModel` 로 만든 TextModel 을 **명시적으로 dispose 하기 전까지 전역 레지스트리에 보관**한다. 에디터 인스턴스를 dispose 해도 model 은 살아남는다. IDE 에서 파일을 수백 개 열면 이게 그대로 누수가 된다.

```typescript
// entities/editor/model-registry.ts
import * as monaco from 'monaco-editor'

const refCount = new Map<string, number>()

export const acquireModel = (path: string, content: string, language: string) => {
    const uri = monaco.Uri.file(path)
    const existing = monaco.editor.getModel(uri)
    const model = existing ?? monaco.editor.createModel(content, language, uri)
    refCount.set(path, (refCount.get(path) ?? 0) + 1)
    return model
}

export const releaseModel = (path: string) => {
    const next = (refCount.get(path) ?? 0) - 1
    if (next > 0) {
        refCount.set(path, next)
        return
    }
    refCount.delete(path)
    const model = monaco.editor.getModel(monaco.Uri.file(path))
    if (!model) return
    monaco.editor.setModelMarkers(model, 'owner', [])
    model.dispose()
}
```

dispose 체크리스트:

- `editor.onDidChangeModelContent` 등 모든 리스너는 `IDisposable` 을 반환한다. `DisposableStore` 처럼 배열에 모아 unmount 때 일괄 `dispose()`.
- `monaco.languages.register*Provider` 로 등록한 provider 도 `IDisposable`. HMR 에서 중복 등록되면 자동완성이 N 배로 뜨고 메모리도 샌다.
- model 을 dispose 해도 해당 model 에 붙은 **marker 는 자동으로 정리되지 않는다**(monaco-editor#2696). 위처럼 빈 배열로 먼저 지운다.
- diff editor 는 original/modified 두 model 을 잡는다. `diffEditor.dispose()` 만으로 model 이 정리되지 않는다(monaco-editor#672 참조).
- Monaco worker(`editor.worker`, `ts.worker`)는 앱 수명 동안 유지된다. 파일 단위로 끄고 켤 수 없으므로 **큰 파일은 애초에 worker 로 보내지 않는 정책**(아래 §5)으로 대응한다.

React 래핑 시 최소 형태:

```typescript
useEffect(() => {
    const editor = monaco.editor.create(hostRef.current!, { automaticLayout: true })
    const model = acquireModel(path, content, language)
    editor.setModel(model)

    const disposables = [
        editor.onDidChangeModelContent(() => markDirty(path)),
        editor.onDidChangeCursorPosition((e) => setCursor(e.position)),
    ]

    return () => {
        disposables.forEach((d) => d.dispose())
        editor.setModel(null)
        editor.dispose()
        releaseModel(path)
    }
}, [path])
```

`editor.setModel(null)` 을 먼저 호출하는 이유: 에디터가 model 을 참조한 채 dispose 되면 view state 정리 순서 문제로 예외가 나는 사례가 있다.

#### 2-4. xterm dispose

```typescript
useEffect(() => {
    const term = new Terminal({ scrollback: 5000, allowProposedApi: true })
    const fit = new FitAddon()
    const webgl = new WebglAddon()
    term.loadAddon(fit)
    term.loadAddon(webgl)
    term.open(hostRef.current!)
    fit.fit()

    const onData = term.onData((d) => invoke('pty_write', { ptyId, data: d }))
    const onResize = term.onResize(({ cols, rows }) => invoke('pty_resize', { ptyId, cols, rows }))
    const ro = new ResizeObserver(() => fit.fit())
    ro.observe(hostRef.current!)

    return () => {
        ro.disconnect()
        onData.dispose()
        onResize.dispose()
        webgl.dispose()
        fit.dispose()
        term.dispose()
    }
}, [ptyId])
```

- `term.dispose()` 는 WebGL 컨텍스트를 반드시 놓아주지 않는 경우가 있으므로 **애드온을 먼저 dispose** 한다. 브라우저의 동시 WebGL 컨텍스트 수 제한(대체로 16개 안팎) 때문에 터미널 탭을 반복 개폐하면 컨텍스트 고갈로 렌더러가 canvas 로 폴백하거나 경고가 뜬다.
- `scrollback` 은 메모리 직결이다. 기본 1000. IDE 에서 5000~10000 이 실용선이고, 그 이상은 라인당 셀 버퍼가 누적된다. 무제한 금지.
- 숨겨진(비활성 탭) 터미널은 `term.dispose()` 하지 말고 DOM 만 떼되, **출력 수신은 Rust 쪽에서 ring buffer 로 받아두고** 재부착 시 한 번에 write 하는 편이 프레임 낭비가 없다.

### 3. "Rust 가 상태를 소유하고 view 는 표시만"

Tauri 공식 State Management 문서 기준. 관리 상태는 `Send + Sync` 여야 하므로 내부 가변성은 `Mutex`/`RwLock` 으로 만든다.

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Manager, State};

#[derive(Default)]
struct Workspace {
    root: Option<std::path::PathBuf>,
    open_files: HashMap<String, OpenFile>,
    revision: u64,
}

struct AppState(Mutex<Workspace>);

#[tauri::command]
fn snapshot(state: State<'_, AppState>) -> WorkspaceSnapshot {
    let ws = state.0.lock().unwrap();
    WorkspaceSnapshot::from(&*ws)
}

#[tauri::command]
fn open_file(path: String, state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    let mut ws = state.0.lock().unwrap();
    ws.open_files.insert(path.clone(), OpenFile::load(&path)?);
    ws.revision += 1;
    let rev = ws.revision;
    drop(ws);
    app.emit("workspace:changed", rev).map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState(Mutex::new(Workspace::default())))
        .invoke_handler(tauri::generate_handler![snapshot, open_file])
        .run(tauri::generate_context!())
        .unwrap();
}
```

**view reload 복원 설계.** WebView 를 새로고침해도 Core 프로세스는 살아 있으므로 Rust 상태는 그대로 유지된다(문서/사례에서 확인). 따라서 프론트 부트 시퀀스를 이렇게 잡는다.

1. 앱 셸 마운트 → `invoke('snapshot')` 로 전체 스냅샷 1회 pull.
2. 스냅샷에 담긴 `revision` 을 프론트에 보관.
3. 이후 `workspace:changed` 이벤트가 오면 **payload 로 온 revision 이 보관값보다 클 때만** 재조회(revision gate). 늦게 도착한 이벤트로 인한 stale 갱신을 막는다.
4. 순수 view 상태(스크롤 위치, 커서, 접힌 트리 노드, 활성 탭)만 프론트가 소유하고 `localStorage`/`sessionStorage` 또는 Rust 의 `view_state` 필드에 저장한다.

프론트에 두면 안 되는 것: 파일 내용의 정본, dirty 버퍼, 워처 상태, LSP 세션, PTY 세션. 전부 Rust 소유이고 프론트는 파생 뷰만 가진다. 이렇게 하면 WebView 크래시/새로고침이 **데이터 손실 없는 재구성**이 된다.

여러 창을 쓰는 경우 문서의 `emit_filter` / `emit_to` 로 대상 webview 를 좁힌다. 전역 `emit` 은 창 수만큼 IPC 직렬화가 반복된다.

```rust
app.emit_filter("open-file", path, |target| match target {
    tauri::EventTarget::WebviewWindow { label } => label == "main",
    _ => false,
})?;
```

실제 사례 근거: awesome-tauri 등재 앱 중 Keadex Mina(서버리스 C4 다이어그램 IDE), 다수 DB 클라이언트가 Rust 소유 상태 + webview 표시 구조를 쓴다고 소개되나, **각 앱의 내부 아키텍처를 코드 수준으로 검증하지는 못했다(미확인)**. VS Code 자체는 이 패턴의 웹판 정본(Extension Host/파일서비스가 상태 소유, renderer 는 표시)이라 참고 가치가 크다.

### 4. 파일 트리 lazy load + 가상 스크롤

수만 파일 규모에서 지켜야 할 원칙.

- **Rust 가 트리를 소유한다.** 프론트는 "현재 펼쳐진 노드로부터 계산된 flat 리스트"만 받는다. 전체 트리를 JSON 으로 넘기면 직렬화 비용 + JS 힙 폭증이 동시에 온다.
- **디렉터리 단위 lazy load.** 확장 시점에 그 디렉터리 한 단계만 읽는다(`std::fs::read_dir`). 재귀 프리로드 금지.
- **flatten 은 Rust 에서.** 프론트가 트리를 매 렌더 flatten 하면 O(N) 이 프레임마다 돈다.

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeRow {
    id: u64,
    name: String,
    depth: u16,
    is_dir: bool,
    is_expanded: bool,
    has_children: bool,
}

#[tauri::command]
fn tree_rows(offset: usize, limit: usize, state: State<'_, AppState>) -> Vec<TreeRow> {
    state.0.lock().unwrap().visible_rows(offset, limit)
}
```

프론트는 `@tanstack/react-virtual` 3.14.9 로 윈도잉한다.

```typescript
const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
    getItemKey: (index) => rows[index]?.id ?? index,
})
```

- `ROW_HEIGHT` 고정(예: 22px). 가변 높이는 measure 비용이 커진다.
- `getItemKey` 를 안정된 노드 id 로 준다. index key 는 펼침/접힘 시 전체 DOM 재생성을 유발한다.
- `overscan` 은 10~20. 과하면 DOM 노드가 늘고 적으면 스크롤 시 흰 줄이 보인다.
- 창 밖 구간은 Rust 에서 페이지 단위로 당겨오되(`tree_rows(offset, limit)`), 요청은 스크롤 정지 후 디바운스(약 50ms)한다.

대안: 트리 전체 flat 배열을 한 번에 받되 **행 데이터는 typed array/문자열 풀**로 압축하는 방법도 있으나, lazy 확장이 있으면 굳이 필요 없다.

### 5. 대형 파일 정책 (VS Code 기준값)

VS Code 소스(`src/vs/editor/common/model/textModel.ts`)의 실제 상수 — 직접 조회로 확인.

```typescript
private static readonly LARGE_FILE_SIZE_THRESHOLD = 20 * 1024 * 1024;              // 20 MB
private static readonly LARGE_FILE_LINE_COUNT_THRESHOLD = 300 * 1000;              // 300K lines
private static readonly LARGE_FILE_HEAP_OPERATION_THRESHOLD = 256 * 1024 * 1024;   // 256M chars (~512MB 메모리)
static _MODEL_SYNC_LIMIT = 50 * 1024 * 1024;                                       // 50 MB
```

동작 요약: `editor.largeFileOptimizations`(기본 on)가 켜져 있고 파일이 위 임계를 넘으면 **토크나이즈를 끈다.** 대형 파일 모델은 워커로 동기화되지 않아 diff 계산·링크 감지·단어 기반 자동완성에서 제외되며, 이를 통해 메모리를 최대 50% 절감했다고 기술되어 있다. 그 외 `files.maxMemoryForLargeFilesMB` 로 대형 파일 처리용 메모리 상한을 조정한다.

TAIDE 권장 정책(위 상수를 기준선으로 삼되 Tauri 특성 반영):

| 파일 크기 | 동작 |
|---|---|
| < 2 MB | 전 기능(하이라이트, LSP, diff, minimap, 접기, bracket colorization) |
| 2 ~ 20 MB | LSP 문서 동기화 중단, minimap off, `wordBasedSuggestions: 'off'`, `bracketPairColorization.enabled: false`, `folding: false`, `occurrencesHighlight: 'off'` |
| 20 MB ~ 50 MB | 위 + 토크나이즈/하이라이트 off(plain text), `wordWrap: 'off'`, 읽기 전용 경고 |
| > 50 MB | Monaco 로 열지 않는다. **뷰어 모드**(Rust 에서 청크 read + 가상 스크롤 라인 뷰)로 전환하거나 외부 앱 열기 제안 |
| 단일 라인 길이 > 10,000자 | 해당 라인 하이라이트 중단(Monaco `maxTokenizationLineLength` 기본값은 **미확인**, 명시 설정 권장) |

읽기는 반드시 Rust 에서 크기를 먼저 확인하고 정책을 결정한 뒤, 필요한 만큼만 전송한다.

```rust
#[tauri::command]
async fn open_document(path: String) -> Result<OpenResult, String> {
    let meta = tokio::fs::metadata(&path).await.map_err(|e| e.to_string())?;
    const VIEWER_ONLY: u64 = 50 * 1024 * 1024;
    if meta.len() > VIEWER_ONLY {
        return Ok(OpenResult::ViewerOnly { size: meta.len() });
    }
    let bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    // 바이너리 판정: 앞 8KB 에 NUL 이 있으면 바이너리로 간주 (git 휴리스틱)
    if bytes.iter().take(8192).any(|b| *b == 0) {
        return Ok(OpenResult::Binary { size: meta.len() });
    }
    Ok(OpenResult::Text {
        content: String::from_utf8_lossy(&bytes).into_owned(),
        degraded: meta.len() > 2 * 1024 * 1024,
    })
}
```

IPC 관점 주의: 파일 내용을 String 으로 넘기면 JSON 직렬화 + WebView 파싱에서 최소 2~3배 피크 메모리가 뜬다. 큰 텍스트는 **Channel 로 청크 스트리밍**하거나, `tauri://localhost` 커스텀 프로토콜(asset protocol)로 바이너리 그대로 넘기는 편이 훨씬 싸다.

### 6. 파일 와처 debounce/집계 (`notify` + `notify-debouncer-full`)

`notify-debouncer-full` 특징(공식 설명): rename From/To 를 매칭해 **단일 Rename 이벤트로 방출**하고, rename 이전에 발생했으나 아직 방출되지 않은 이벤트의 경로를 갱신한다. 선택적으로 파일시스템 ID 를 추적해 플랫폼 간 rename 을 이어붙인다.

```toml
# Cargo.toml
notify = "8.2"
notify-debouncer-full = "0.7"
```

```rust
use std::{path::Path, sync::mpsc, time::Duration};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::new_debouncer;

pub fn spawn_watcher(root: &Path, app: tauri::AppHandle) -> notify::Result<()> {
    let (tx, rx) = mpsc::channel();
    // 1st: debounce timeout, 2nd: tick rate (None = timeout/4)
    let mut debouncer = new_debouncer(Duration::from_millis(300), None, tx)?;
    debouncer.watch(root, RecursiveMode::Recursive)?;

    std::thread::spawn(move || {
        // debouncer 를 스레드로 move 해 살려둔다. drop 되면 워칭이 즉시 끊긴다.
        let _keep_alive = debouncer;
        for result in rx {
            match result {
                Ok(events) => {
                    let paths = aggregate(events); // 경로 단위 dedup + 무시목록 필터
                    if paths.is_empty() { continue; }
                    let _ = app.emit("fs:changed", paths);
                }
                Err(errors) => eprintln!("watch errors: {errors:?}"),
            }
        }
    });
    Ok(())
}
```

집계 규칙(IDE 실전):

- **무시 목록을 워처 등록 전에 적용**한다. `.git/`, `node_modules/`, `target/`, `dist/`, `.next/` 를 재귀 워칭하면 inode/FSEvents 핸들이 수만 개로 늘고 이벤트가 초당 수천 건 쏟아진다. `notify` 는 경로 필터를 내장하지 않으므로 상위 디렉터리를 통째로 watch 하되 콜백에서 걸러내거나, 하위 디렉터리를 선택적으로 watch 한다.
- 300ms 는 에디터 저장 반응성과 폭주 억제의 절충. 빌드 산출물 디렉터리는 별도로 1~2초 디바운스.
- 프론트 방출은 **경로 배열 1건**으로 묶는다. 파일당 이벤트 1건씩 emit 하면 IPC 가 병목이 된다.
- macOS 는 FSEvents 라 디렉터리 단위 이벤트가 오고, Linux inotify 는 **watch descriptor 수 제한**(`fs.inotify.max_user_watches`, 배포판 기본 8192~65536)에 걸린다. 초과 시 워칭이 조용히 실패하므로 에러를 UI 로 노출한다.
- 파일 트리 갱신은 "변경된 디렉터리의 자식만 재조회"로 국소화한다. 루트 전체 재스캔 금지.

### 7. xterm 고속 출력 flood 제어

공식 Flow Control 가이드 기준 사실:

- xterm.js 처리량은 대략 **5~35 MB/s** 수준인데 생산자는 GB/s 급일 수 있다.
- write 버퍼에는 **하드코딩된 50MB 상한**이 있다(초과 시 OOM 방지 목적).
- `write` 는 non-blocking 이며 다음 이벤트 루프에서 처리하고, 한 프레임(16ms) 이내로 끝내도록 설계돼 있다.
- 구식 XON/XOFF 방식은 pty 에서 flow control 이 꺼진 환경(많은 셸 초기화 스크립트가 끈다)에서 동작하지 않는다.

권장은 **watermark 방식**. 가이드 원문 코드:

```javascript
const CALLBACK_BYTE_LIMIT = 100000;
const HIGH = 5;
const LOW = 2;
let written = 0;
let pendingCallbacks = 0;

pty.onData(chunk => {
  written += chunk.length;
  if (written > CALLBACK_BYTE_LIMIT) {
    term.write(chunk, () => {
      pendingCallbacks = Math.max(pendingCallbacks - 1, 0);
      if (pendingCallbacks < LOW) pty.resume();
    });
    pendingCallbacks++;
    written = 0;
    if (pendingCallbacks > HIGH) pty.pause();
  } else {
    term.write(chunk);
  }
});
```

가이드의 명시적 권고: **`HIGH` 는 500K 를 넘기지 말 것**(넘기면 폭주 중 키 입력 반응이 죽는다).

TAIDE 는 pty 가 Rust 쪽에 있으므로 flow control 을 **IPC 를 가로질러** 구성한다.

```typescript
// 프론트: 처리 완료를 Rust 에 알린다
const onEvent = new Channel<PtyEvent>()
let pending = 0
onEvent.onmessage = (msg) => {
    if (msg.event !== 'output') return
    pending++
    if (pending > HIGH) invoke('pty_pause', { ptyId })
    term.write(msg.data.chunk, () => {
        pending = Math.max(pending - 1, 0)
        if (pending < LOW) invoke('pty_resume', { ptyId })
    })
}
```

Rust 쪽 보완:

- pty 읽기 루프에서 **약 4~16KB 청크를 최대 ~8ms 동안 모아 한 번에 send**한다(chunk coalescing). 바이트 단위 send 는 IPC 프레임 오버헤드가 지배적이 된다.
- pause 상태에서는 읽기를 멈추지 말고(파이프가 막혀 자식이 SIGPIPE/블록됨) **ring buffer 에 흡수**하되 상한(예: 4MB)을 넘으면 앞부분을 버리고 "출력 일부 생략" 마커를 남긴다. VS Code 도 유사한 절단 정책을 쓴다.
- 비활성 터미널 탭은 write 자체를 하지 않고 ring buffer 에만 쌓았다가 활성화 시 한 번에 flush.
- WebGL 렌더러(`@xterm/addon-webgl`)를 쓰면 폭주 시 프레임 비용이 크게 줄어든다. 다만 §2-4 의 컨텍스트 수 제한 주의.

### 8. 앱 시작 시간 최적화

Rust 측:

```toml
# src-tauri/Cargo.toml
[profile.release]
lto = true
codegen-units = 1
opt-level = "s"     # 바이너리 크기 우선. 연산 집약이면 3
panic = "abort"
strip = true
```

- `setup()` 안에서 **블로킹 작업 금지**. DB 열기, 인덱싱, LSP 기동, 워처 등록은 `tauri::async_runtime::spawn` 으로 뒤로 미루고 창부터 띄운다.
- CPU 집약 command 는 `tauri::async_runtime::spawn_blocking` 으로. `#[tauri::command]` 가 async 여도 내부에서 동기 블로킹하면 런타임 스레드를 막는다.
- 기능 플래그로 불필요한 Tauri feature 를 끈다.
- 워크스페이스 인덱싱은 "보이는 것 먼저": 루트 한 단계 → 열려 있던 파일 → 백그라운드 전체 스캔.

프론트 측:

- 라우트/패널 단위 코드 스플리팅. **Monaco 와 xterm 은 초기 번들에서 제외**하고 `import()` 로 지연 로드한다. 둘 다 수 MB 급이라 초기 파싱 시간을 지배한다.
- Monaco 는 필요한 언어만 등록한다. `monaco-editor/esm/vs/editor/editor.api` 를 쓰고 전체 `monaco-editor` 배럴 import 를 피한다.
- 첫 페인트에 필요한 상태는 `invoke('snapshot')` 한 번으로 끝낸다. 초기 IPC 왕복을 여러 번 하면 그대로 지연이 된다.
- 창 배경색을 앱 테마와 맞춰 흰/검 플래시를 없앤다(`tauri.conf.json` 의 window `backgroundColor`, 또는 `visible: false` 로 만들고 프론트 준비 완료 이벤트에서 `show()`).

```rust
tauri::Builder::default()
    .setup(|app| {
        let handle = app.handle().clone();
        tauri::async_runtime::spawn(async move {
            index_workspace(&handle).await;
        });
        Ok(())
    })
```

알려진 이슈: Windows 에서 20초 이상 걸린다는 보고(tauri#13727), AppImage 에서 HTML 표시까지 1~2초 지연 보고 등이 존재한다. 배포 형태별 실측이 필요하다.

### 9. 메모리 프로파일링 방법

**WebView(프론트) 쪽**

- 개발 중에는 devtools 로. Tauri v2 는 debug 빌드에서 devtools 가 열리고, release 에서 필요하면 `devtools` feature 를 켠다.
- Chromium 계열(Windows WebView2): Memory 패널의 **Heap snapshot** 으로 detached DOM, 살아있는 Monaco model, 리스너 누적을 확인한다. Performance monitor 의 JS heap size / DOM nodes / Event listeners 3개 지표를 탭 개폐 반복 시 관찰하면 누수가 바로 보인다.
- macOS WKWebView: Safari 의 Develop 메뉴에서 앱의 WebView 에 원격 연결해 Timelines/Memory 를 본다(`WKWebView` inspectable 필요, Tauri debug 빌드에서 활성).
- 누수 회귀 테스트: "탭 50회 개폐 후 heap snapshot 2회 비교" 를 수동 체크리스트로 둔다.

**Rust(Core) 쪽**

- `dhat`(dhat-rs): 크레이트를 넣고 글로벌 allocator 를 교체하면 **모든 플랫폼**에서 할당 프로파일과 피크 메모리를 얻는다. 어떤 코드가 할당을 많이 하는지 파악에 적합.
  ```rust
  #[global_allocator]
  static ALLOC: dhat::Alloc = dhat::Alloc;

  fn main() {
      let _profiler = dhat::Profiler::new_heap();
      app_lib::run();
  }
  ```
- Linux: `heaptrack`, `bytehound`.
- macOS: Xcode **Instruments** (Allocations / Leaks). 단, WebContent 자식 프로세스는 별도로 attach 해야 한다.
- Windows: WebView2 프로세스는 작업 관리자에서 `msedgewebview2.exe` 로 별도 집계되므로 앱 프로세스만 보면 안 된다.

**프로세스 합산 측정(모든 플랫폼 공통 원칙)**: Tauri 는 멀티프로세스이므로 앱 RSS 단독 수치는 무의미하다. macOS `ps -o rss -p <pid>` 를 WebContent/Networking 자식까지 모아 합산하거나, Windows 는 앱 + `msedgewebview2.exe` 자식 트리를 합산해 비교한다.

---

## TAIDE 적용 가이드

1. **IPC 채널 분리 원칙**: 저빈도 상태 알림 = `emit`/`listen`, 고빈도 스트림(PTY 출력, 검색 결과, LSP diagnostics) = `Channel`. 공식 문서가 이벤트 시스템은 고throughput 용이 아니라고 명시한다.
2. **상태 소유권**: 파일 내용·dirty 버퍼·워처·LSP·PTY 는 Rust `State<AppState>` 가 정본. 프론트는 snapshot + revision gate 로 파생 뷰만 유지. WebView reload 는 `invoke('snapshot')` 1회로 완전 복원되게 만든다.
3. **dispose 규율**: `useTauriEvent` 헬퍼, Monaco `acquireModel/releaseModel` refcount 레지스트리, xterm 애드온 선행 dispose. 이 세 가지를 shared 레이어에 정본으로 두고 컴포넌트가 직접 `listen`/`createModel`/`new Terminal` 을 호출하지 않게 한다.
4. **대형 파일 4단계 정책**(2MB / 20MB / 50MB / 뷰어 모드)을 상수로 `shared/lib/constants` 에 두고 Rust 쪽에도 동일 상수를 둔다.
5. **파일 트리**: Rust 소유 트리 + flat row 페이지네이션 + `@tanstack/react-virtual`. 무시 목록은 워처와 트리가 **같은 규칙**을 공유한다.
6. **워처**: `notify 8.2` + `notify-debouncer-full 0.7`, 300ms, 경로 배열 1건으로 집계 방출. `.git`/`node_modules`/`target` 제외.
7. **터미널**: Rust ring buffer + watermark flow control + 청크 병합(4~16KB / ~8ms), `scrollback` 5000, WebGL 애드온.
8. **시작**: `setup()` 논블로킹, Monaco/xterm 동적 import, 창은 `visible: false` → ready 시 `show()`.
9. **회귀 방지**: "탭 50회 개폐 → heap snapshot 증가량", "터미널에 `yes` 10초 → 입력 반응성", "10만 파일 워크스페이스 열기 시간/RSS" 를 수동 QA 체크리스트(`docs/quality-assurance`)로 고정.

---

## 함정·주의

- `listen()` 은 Promise. resolve 전에 unlisten 하면 **해제되지 않고 누적**된다. StrictMode 이중 마운트와 겹치면 리스너가 배로 늘어난다.
- Monaco 는 `editor.dispose()` 만으로 **model 이 해제되지 않는다.** 또한 model dispose 시 marker 가 남는다.
- Monaco `import 'monaco-editor'` 배럴은 모든 언어 워커를 끌고 온다. 번들과 시작 시간을 함께 망친다.
- xterm `write` 버퍼 상한 50MB 는 하드코딩이며, 넘기 전에 앱이 먼저 느려진다. flow control 없이는 반드시 터진다.
- xterm WebGL 컨텍스트 수 제한 때문에 터미널 탭 반복 개폐 시 애드온 dispose 를 빠뜨리면 렌더러가 조용히 열화된다.
- `notify` 의 debouncer 객체를 drop 하면 워칭이 즉시 중단된다. 스레드/구조체에 반드시 소유시켜 둔다.
- Linux inotify watch 수 제한을 넘으면 워칭이 **조용히 실패**한다. 에러를 삼키지 말 것.
- `notify` 9.x / `notify-debouncer-full` 0.8.x 는 2026-08 시점 rc. 프로덕션은 stable 조합 사용.
- 큰 텍스트를 `String` 으로 invoke 반환하면 JSON 직렬화 + 파싱으로 피크 메모리가 2~3배. Channel 스트리밍 또는 커스텀 프로토콜을 쓴다.
- Tauri 는 멀티프로세스다. 메모리 측정 시 자식 WebView 프로세스를 합산하지 않으면 수치가 무의미하다.
- Linux WebKitGTK 는 GPU 드라이버 조합(특히 NVIDIA)에서 DMA-BUF 렌더러 문제가 알려져 있다. `WEBKIT_DISABLE_DMABUF_RENDERER=1` 는 렌더 성능을 희생하는 워크어라운드이므로 기본 적용하지 말고 감지 시에만 안내한다.
- macOS WKWebView 버전은 OS 에 종속돼 **앱이 고정할 수 없다.** 구형 macOS 에서 최신 웹 API 미지원을 가정하고 폴리필/폴백을 준비한다.
- Monaco 의 `maxTokenizationLineLength` 기본값과 VS Code 웹/원격의 파일 크기 상한(1GB/100MB/50MB 언급)은 2차 자료 기반이라 **미확인**. 값에 의존하는 결정 전에 재확인 필요.

---

## 참고 링크

- Tauri Process Model: https://v2.tauri.app/concept/process-model/
- Tauri Calling the Frontend from Rust (events, Channel, unlisten): https://v2.tauri.app/develop/calling-frontend/
- Tauri State Management: https://v2.tauri.app/develop/state-management/
- Tauri Webview Versions: https://v2.tauri.app/reference/webview-versions/
- Tauri Linux Graphics Issues (DMA-BUF, compositing): https://v2.tauri.app/develop/debug/linux-graphics/
- Tauri issue #8913 (unlisten in React useEffect): https://github.com/tauri-apps/tauri/issues/8913
- Tauri issue #13727 (Windows 느린 시작): https://github.com/tauri-apps/tauri/issues/13727
- Tauri discussion #3162 (메모리 사용량 개선): https://github.com/orgs/tauri-apps/discussions/3162
- awesome-tauri: https://github.com/tauri-apps/awesome-tauri
- crates.io tauri: https://crates.io/crates/tauri
- crates.io notify: https://crates.io/crates/notify
- crates.io notify-debouncer-full: https://crates.io/crates/notify-debouncer-full
- docs.rs notify_debouncer_full: https://docs.rs/notify-debouncer-full
- xterm.js Flow Control 가이드: https://xtermjs.org/docs/guides/flowcontrol/
- xterm.js releases: https://github.com/xtermjs/xterm.js/releases
- xterm.js issue #4135 (flood 시 FPS 캡): https://github.com/xtermjs/xterm.js/issues/4135
- xterm.js issue #2077 (flow control/back pressure): https://github.com/xtermjs/xterm.js/issues/2077
- VS Code textModel.ts (대형 파일 상수): https://github.com/microsoft/vscode/blob/main/src/vs/editor/common/model/textModel.ts
- VS Code issue #30243 (large files test plan): https://github.com/Microsoft/vscode/issues/30243
- monaco-editor issue #2696 (model dispose 후 marker 잔존): https://github.com/microsoft/monaco-editor/issues/2696
- monaco-editor issue #672 (diff editor dispose): https://github.com/microsoft/monaco-editor/issues/672
- monaco-editor issue #1693 (memory leakage): https://github.com/microsoft/monaco-editor/issues/1693
- TanStack Virtual: https://tanstack.com/virtual/latest
- dhat (Rust heap profiler): https://docs.rs/dhat/latest/dhat/
- The Rust Performance Book — Profiling: https://nnethercote.github.io/perf-book/profiling.html
- Edge DevTools Heap snapshots: https://learn.microsoft.com/en-us/microsoft-edge/devtools-guide-chromium/memory-problems/heap-snapshots
- WebView2 end user FAQ (프로세스 모델): https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/end-user-faq
- WebKit Graphics (WebKitGTK/WPE, DMA-BUF): https://docs.webkit.org/Ports/WebKitGTK%20and%20WPE%20WebKit/Graphics.html
