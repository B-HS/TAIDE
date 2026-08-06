# 터미널 AI 에이전트(Claude Code)와 IDE 연동

> 조사일: 2026-08-06. 아래 내용은 조사 시점의 공식 문서·저장소 원본을 직접 확인한 결과다.
> 공식 문서로 확정되지 않은 항목은 **[미확인]** 또는 **[커뮤니티 출처]** 로 명시했다.

---

## 버전 확정 (2026-08 기준)

| 대상 | 버전 | 확인 근거 |
|---|---|---|
| Claude Code CLI | **2.1.223** (CHANGELOG 최상단) | `anthropics/claude-code` `CHANGELOG.md` raw 직접 확인 |
| Claude Code 배포 방식 | npm(`@anthropic-ai/claude-code`) **deprecated**, `curl -fsSL https://claude.ai/install.sh \| bash` 권장 | npm 페이지 / 릴리스 노트 |
| Claude Code VS Code 확장 요구사항 | VS Code **1.94.0** 이상 | 공식 `vs-code` 문서 |
| `Ctrl+G` 외부 에디터 최초 도입 | **2.0.10** ("Press Ctrl-G to edit your prompt in your system's configured text editor") | CHANGELOG 라인→버전 매핑 |
| `Ctrl+X Ctrl+E` alias 추가 | **2.1.83** | CHANGELOG |
| 외부 에디터에 직전 응답을 주석 컨텍스트로 표시 옵션 | **2.1.110** (`externalEditorContext`) | CHANGELOG + settings 문서 |
| GUI 에디터 열림 중 마우스/포커스 잔상 수정 | **2.1.216** | CHANGELOG |
| tauri (crates.io) | **2.11.5** (2026-07-01) | crates.io API |
| tauri-plugin-single-instance | **2.4.3** (2026-07-13) | crates.io API |
| tauri-plugin-deep-link | **2.4.9** (2026-05-02) | crates.io API |
| portable-pty | **0.9.0** (2025-02-11) | crates.io API |
| sysinfo | **0.39.6** (2026-07-09) | crates.io API |

미확인 항목:

- **`promptEditor` 설정 키**: 공식 `settings.md` 전문(280KB)을 받아 grep한 결과 **존재하지 않는다.** 일부 2차 자료가 이 키를 언급하나 공식 문서·CHANGELOG 어디에도 없다. → **미확인(문서화되지 않음). 사용 금지.**
- **`Ctrl+G`의 에디터 결정 우선순위**: 공식 문서는 "your default text editor"라고만 적고 해석 순서를 명시하지 않는다. 아래 §1.2 참조. → **부분 미확인.**

---

## 핵심 API·사용법

### 1. Claude Code 의 외부 에디터 열기 (`Ctrl+G`)

#### 1.1 공식 문서 원문 (interactive-mode)

> `Ctrl+G` or `Ctrl+X Ctrl+E` — **Open in default text editor**
> Edit your prompt or custom response in your default text editor. `Ctrl+X Ctrl+E` is the readline-native binding. Turn on **Show last response in external editor** in `/config` to prepend Claude's previous reply as `#`-commented context above your prompt; Claude Code strips the comment block when you save

같은 문서의 transcript viewer 항목은 환경변수를 **명시적으로** 언급한다:

> `v` — Write the conversation to a temporary file and open it in `$VISUAL` or `$EDITOR`. Requires fullscreen rendering

즉 **transcript `v` 경로는 `$VISUAL` → `$EDITOR` 순으로 공식 확정**이다.

#### 1.2 `Ctrl+G` 의 에디터 결정 — 확정/미확정 구분

확정된 사실:

- CHANGELOG 2.1.142: "Fixed `claude agents` "v to open in editor" using the daemon's default editor instead of your shell's `$EDITOR`/`$VISUAL`" → Claude Code는 내부적으로 `$EDITOR`/`$VISUAL` 개념을 사용한다.
- CHANGELOG 2.1.212: 플랜 승인 다이얼로그 푸터에 **"ctrl+g to edit in \<editor\>"** 문구가 있다 → 결정된 에디터 이름을 UI에 노출한다.
- CHANGELOG 2.1.216: "Fixed mouse and focus garbage in the terminal while a **GUI editor** from `/memory`, `/plan`, `/keybindings`, or Ctrl+G is open" → **GUI 에디터와 터미널 에디터를 구분해서 취급**한다.

미확정(추측 금지 대상):

- GitHub Issue **#18990** (open, stale, `area:tui`/`enhancement`) 는 "Ctrl+G 가 설치된 IDE(VS Code, Cursor)를 자동 감지하고 `EDITOR`/`VISUAL` 을 무시한다"고 주장하며 설정 가능화를 요청한다. **아직 미해결(open)** 이다.
- 반면 커뮤니티 gist(2.1.70 기준)는 `~/.claude/settings.json` 의 `env.EDITOR` 로 지정하면 동작한다고 보고한다. **[커뮤니티 출처]**

```json
// ~/.claude/settings.json — [커뮤니티 출처] 2.1.70 기준 보고. 공식 문서 미기재.
{
  "env": {
    "EDITOR": "zed"
  }
}
```

→ **TAIDE 는 이 우선순위에 의존하지 말고, "환경변수 `EDITOR`/`VISUAL` 로 지정된 경우 올바르게 동작하는 CLI 헬퍼"를 제공하는 데 집중해야 한다.** 우선순위 자체는 Claude Code 구현 세부이고 변한다.

#### 1.3 동작 방식 (임시파일 + 프로세스 대기)

공식 문서에 상세 서술은 없다. 다만 **간접 확정 근거**가 두 개 있다:

1. transcript `v` 는 "Write the conversation to a **temporary file** and open it in `$VISUAL` or `$EDITOR`" — 임시파일 방식이 공식 명시.
2. CHANGELOG 2.1.216: "`/memory` **no longer waits for the editor to close**" — 즉 다른 경로들은 **에디터 프로세스가 종료될 때까지 대기**한다.

커뮤니티 gist의 서술 **[커뮤니티 출처]**:

> creates a temp file, starts the editor, and waits for the **_process it started_** to finish.

여기서 도출되는 결정적 함의(TAIDE 설계의 핵심):

- 저장 감지는 파일 watch 가 **아니라 자식 프로세스 종료(exit)** 기준이다.
- 따라서 **"기존 인스턴스에 파일을 넘기고 즉시 exit 하는" 런처는 전부 깨진다.** gist가 보고한 Zed/Notepad++ 사례가 정확히 이것이다:

> Editors like zed and notepad++ that "reuse" their first instance: if an editor is already open, your Ctrl+G from Claude Code will be ignored — a long deleted temp file will be opened for edit and changes are ignored.

- Windows GitBash 에서 `zed` 같은 PATH 이름만 주면 **GitBash 가 별도 셸을 띄워 에디터를 병렬 실행하고 즉시 exit** → 변경 미감지. 전체 실행 파일 절대경로(`C:/Users/<u>/AppData/Local/Programs/Zed/Zed.exe`, 슬래시 사용)를 줘야 한다. **[커뮤니티 출처]**

#### 1.4 관련 설정

`externalEditorContext` (global config settings, 공식 `settings.md` 원문):

> **Default**: `false`. Prepend Claude's previous response as `#`-commented context when you open the external editor with `Ctrl+G`. Appears in `/config` as **Show last response in external editor**

즉 켜면 임시파일 상단에 `#` 주석 블록이 붙고, **저장 시 Claude Code 가 그 블록을 제거**한다. TAIDE 에디터가 이 파일을 열 때 `#` 주석을 임의로 건드리면 안 된다.

관련 알려진 이슈(참고):

- #9218 — v2.0.11 에서 Ctrl+G 후 터미널 손상/복구 불가.
- #19076 — SSH 세션에서 TTY attach 실패로 에디터가 뜨지 않음 (**closed as not planned**).
- #50032 — 마우스/키보드 escape 시퀀스가 채팅 입력으로 새어 들어옴.
- CHANGELOG 2.1.136 / 2.1.129 — Ctrl+G 이후 Backspace 키 스왑, 대화 히스토리 blank 이슈 수정.

→ **TAIDE 가 GUI 에디터로서 EDITOR 역할을 하면 이 TTY 계열 함정 대부분을 회피**한다(터미널 화면을 점유하지 않으므로). 단 §4 참조.

---

### 2. VS Code `code --wait` 의 동작 원리 (원본 소스 확인)

TAIDE CLI 헬퍼가 그대로 베껴야 하는 정본 설계다. 아래는 `microsoft/vscode` main 브랜치 소스를 직접 읽어 확인했다.

#### 2.1 마커 파일 생성 — `src/vs/platform/environment/node/wait.ts`

```ts
export function createWaitMarkerFileSync(verbose?: boolean): string | undefined {
	const randomWaitMarkerPath = randomPath(tmpdir());
	try {
		writeFileSync(randomWaitMarkerPath, ''); // use built-in fs to avoid dragging in more dependencies
		if (verbose) { console.log(`Marker file for --wait created: ${randomWaitMarkerPath}`); }
		return randomWaitMarkerPath;
	} catch (err) { /* ... */ return undefined; }
}
```

#### 2.2 CLI 측 대기 — `src/vs/code/node/cli.ts`

```ts
waitMarkerFilePath = createWaitMarkerFileSync(args.verbose);
if (waitMarkerFilePath) {
	addArg(argv, '--waitMarkerFilePath', waitMarkerFilePath);
}
// ...
await Promise.race([
	whenDeleted(waitMarkerFilePath!),
	Event.toPromise(Event.fromNodeEventEmitter(child, 'error')),
	childExitPromise
]);
```

**핵심: `code --wait` 는 자식 프로세스 종료를 기다리는 게 아니라, "마커 파일이 삭제될 때까지" 기다린다.** 자식 exit/error 는 안전망일 뿐이다.

#### 2.3 삭제 감지 — `src/vs/base/node/pfs.ts` (1초 폴링)

```ts
export function whenDeleted(path: string, intervalMs = 1000): Promise<void> {
	return new Promise<void>(resolve => {
		let running = false;
		const interval = setInterval(() => {
			if (!running) {
				running = true;
				fs.access(path, err => {
					running = false;
					if (err) { clearInterval(interval); resolve(undefined); }
				});
			}
		}, intervalMs);
	});
}
```

파일시스템 watcher 가 아니라 **1초 간격 `fs.access` 폴링**이다. 네트워크 FS·크로스 플랫폼 신뢰성을 위한 선택.

#### 2.4 마커 삭제 측 (앱 내부) — `src/vs/workbench/electron-browser/window.ts`

```ts
// In wait mode, listen to changes to the editors and wait until the files
// are closed that the user wants to wait for. When this happens we delete
// the wait marker file to signal to the outside that editing is done.
// However, it is possible that opening of the editors failed, as such we
// check for whether editor panes got opened and otherwise delete the marker
// right away.
if (openedEditorPanes.length) {
	return this.trackClosedWaitFiles(URI.revive(request.filesToWait.waitMarkerFileUri), /* files */);
} else {
	return this.fileService.del(URI.revive(request.filesToWait.waitMarkerFileUri));
}

private async trackClosedWaitFiles(waitMarkerFile: URI, resourcesToWaitFor: URI[]): Promise<void> {
	// Wait for the resources to be closed in the text editor...
	await this.instantiationService.invokeFunction(accessor => whenEditorClosed(accessor, resourcesToWaitFor));
	// ...before deleting the wait marker file
	await this.fileService.del(waitMarkerFile);
}
```

→ 신호는 **"탭이 닫힐 때"** 지 "저장할 때"가 아니다. (그래서 `--wait` 를 파일 저장 기준으로 바꿔달라는 이슈 #150368 이 존재)

#### 2.5 윈도우 재사용 — `src/vs/workbench/api/node/extHostCLIServer.ts`

```ts
const waitMarkerFileURI = waitMarkerFilePath ? URI.file(waitMarkerFilePath) : undefined;
const preferNewWindow = !forceReuseWindow && !waitMarkerFileURI && !addMode && !removeMode;
```

`--wait` 가 걸리면 **새 윈도우를 선호하지 않고 기존 윈도우를 재사용**한다.

#### 2.6 실행 중 인스턴스로의 전달

- 통합 터미널에서는 `VSCODE_IPC_HOOK_CLI` 환경변수가 가리키는 소켓/네임드파이프로 "이 파일 열어라" 요청을 보낸다. 이 소켓의 서버가 `extHostCLIServer.ts` 다.
- 그 밖의 경우 `Code` 실행파일을 자식으로 띄우고, 실행 중 인스턴스가 있으면 Electron single-instance 로 argv 가 기존 인스턴스에 전달된다.

#### 2.7 요약 다이어그램

```
code --wait file.txt
  ├─ tmpdir()/<random>  마커 파일 생성 (빈 파일)
  ├─ --waitMarkerFilePath=<marker> 를 argv 에 추가
  ├─ VSCODE_IPC_HOOK_CLI 소켓 or 자식 프로세스로 전달 → 기존 윈도우 재사용
  └─ whenDeleted(<marker>) 1초 폴링으로 블록
                                        ↑
     워크벤치: 해당 파일 탭이 전부 닫히면 마커 파일 삭제 ────┘
     (에디터가 하나도 안 열렸으면 즉시 삭제 → CLI 즉시 반환)
```

---

### 3. TAIDE 가 `EDITOR` 로 동작하기 위한 CLI 헬퍼 요구사항

`$EDITOR="taide --wait"` 로 지정했을 때 Claude Code(및 git, kubectl 등 모든 POSIX 도구)가 정상 동작하려면 아래를 모두 만족해야 한다.

#### 3.1 필수 요구사항 (하나라도 빠지면 조용히 깨진다)

| # | 요구사항 | 이유 |
|---|---|---|
| R1 | CLI 헬퍼는 **`--wait`/`-w` 가 있으면 반드시 블록**하고, 편집 종료 전에 exit 하면 안 된다 | Claude Code 는 프로세스 종료를 저장 완료 신호로 본다 |
| R2 | **stdout/stderr 를 오염시키지 않는다** | 일부 호출자가 출력을 파싱한다 |
| R3 | 종료 코드 0 = 정상, 0이 아니면 호출자가 편집 취소로 간주 가능 | git 은 non-zero 시 커밋 중단 |
| R4 | GUI 앱을 **detach 로 띄우고 CLI 프로세스는 살아 있어야** 한다 | 자식 exit 이 곧 완료 신호로 오인됨 |
| R5 | single-instance 로 **기존 TAIDE 창에 파일을 전달**해야 한다 | 매번 새 앱 인스턴스는 사용성·메모리 파탄 |
| R6 | 두 번째 인스턴스가 파일을 넘긴 뒤 **탭이 닫힐 때까지 대기** | R1 을 single-instance 환경에서 성립시키는 유일한 방법 |
| R7 | **파일 경로는 절대경로로 정규화**해서 전달 | 앱의 cwd 와 CLI 의 cwd 가 다르다 |
| R8 | Windows 에서는 PATH 이름이 아니라 **실행파일 절대경로**가 안전 | GitBash 가 별도 셸을 띄워 병렬 실행 후 즉시 exit |
| R9 | GUI 앱을 **포그라운드로 raise/focus** | 사용자가 창을 찾지 못하면 hang 으로 보인다 |

#### 3.2 권장 아키텍처 — VS Code 의 마커 파일 방식을 그대로 채택

가장 견고하고, Tauri 2 의 single-instance 플러그인과 자연스럽게 맞물린다.

```
taide --wait <file>
  1) tmpdir()/taide-wait-<uuid> 마커 파일 생성
  2) single-instance IPC 로 {file: <abs>, waitMarker: <marker>} 전달
     - 실행 중 인스턴스 없으면: 앱을 detach 로 spawn 하며 argv 로 동일 payload 전달
  3) whenDeleted(marker) 폴링(200~500ms)으로 블록
  4) 마커 삭제되면 exit 0
앱 측:
  - payload 수신 → 탭 열기 → 창 focus
  - 해당 탭이 닫히면 marker 파일 삭제
  - 탭 열기 실패 시 즉시 marker 삭제(호출자가 영구 hang 하지 않도록)
  - 앱 종료 시 미해결 marker 전부 삭제 (안전망)
```

**왜 "프로세스가 그냥 살아 있기"가 아니라 마커 파일인가**: TAIDE 앱과 CLI 헬퍼는 서로 다른 프로세스다. IPC 응답을 기다리는 방식은 앱이 크래시하면 CLI 가 영구 hang 한다. 마커 파일 + 앱 종료 시 정리 조합이 크래시 안전성을 준다.

#### 3.3 Tauri 2 구현 — single-instance

`tauri-plugin-single-instance` **2.4.3**. 두 번째 인스턴스의 argv/cwd 를 첫 인스턴스 콜백으로 넘겨준다.

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2.11", features = [] }
tauri-plugin-single-instance = "2.4"
serde_json = "1"
```

```rust
// src-tauri/src/lib.rs
use tauri::{Emitter, Manager};

#[derive(serde::Serialize, Clone)]
struct OpenRequest {
    path: String,
    wait_marker: Option<String>,
}

const OPEN_EVENT: &str = "taide://open-file";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(req) = parse_argv(&argv) {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
                let _ = app.emit(OPEN_EVENT, req);
            }
        }))
        .setup(|app| {
            let argv: Vec<String> = std::env::args().collect();
            if let Some(req) = parse_argv(&argv) {
                let _ = app.emit(OPEN_EVENT, req);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running TAIDE");
}

fn parse_argv(argv: &[String]) -> Option<OpenRequest> {
    let mut path: Option<String> = None;
    let mut wait_marker: Option<String> = None;
    let mut it = argv.iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--wait" | "-w" => {}
            "--wait-marker" => wait_marker = it.next().cloned(),
            other if !other.starts_with('-') => path = Some(other.to_string()),
            _ => {}
        }
    }
    let p = path?;
    let abs = std::fs::canonicalize(&p)
        .map(|x| x.to_string_lossy().to_string())
        .unwrap_or(p);
    Some(OpenRequest { path: abs, wait_marker })
}
```

탭이 닫힐 때 프론트엔드가 호출하는 커맨드:

```rust
#[tauri::command]
fn release_wait_marker(marker: String) {
    let _ = std::fs::remove_file(marker);
}
```

#### 3.4 CLI 헬퍼 본체 (별도 작은 바이너리 권장)

GUI 번들과 분리된 얇은 실행파일이어야 한다. macOS `.app` 번들 내부 바이너리를 직접 EDITOR 로 지정하면 dock/activation 문제가 생긴다.

```rust
// crates/taide-cli/src/main.rs
use std::{env, fs, path::PathBuf, process::Command, thread, time::Duration};

const POLL_MS: u64 = 300;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let wait = args.iter().any(|a| a == "--wait" || a == "-w");
    let Some(file) = args.iter().find(|a| !a.starts_with('-')) else {
        eprintln!("usage: taide [--wait] <file>");
        return std::process::ExitCode::from(2);
    };
    let abs = fs::canonicalize(file).unwrap_or_else(|_| PathBuf::from(file));

    let marker = wait.then(|| {
        let p = env::temp_dir().join(format!("taide-wait-{}", uuid_v4()));
        let _ = fs::write(&p, b"");
        p
    });

    let mut cmd = Command::new(app_binary_path());
    cmd.arg(&abs);
    if let Some(m) = &marker {
        cmd.arg("--wait-marker").arg(m);
    }
    // 자식 exit 여부와 무관하게 동작해야 하므로 detach 로 spawn
    let child = cmd.spawn();

    if let (Some(m), Ok(mut child)) = (marker.as_ref(), child) {
        loop {
            if !m.exists() {
                break; // 편집 완료
            }
            // 앱이 죽었고 마커도 안 지워졌으면 무한 hang 방지
            if let Ok(Some(_)) = child.try_wait() {
                if !m.exists() { break; }
                // single-instance 로 전달만 하고 종료한 경우엔 계속 대기해야 하므로
                // 마커가 살아 있으면 계속 폴링한다.
            }
            thread::sleep(Duration::from_millis(POLL_MS));
        }
        let _ = fs::remove_file(m);
    }
    std::process::ExitCode::SUCCESS
}
```

> 주의: 위 코드에서 `try_wait()` 분기는 **자식이 곧바로 종료해도 대기를 유지**하는 것이 핵심이다. single-instance 환경에서 두 번째 프로세스는 argv 를 넘긴 뒤 즉시 exit 하기 때문이다. 이것이 `code --wait` 가 `Promise.race` 로 childExit 을 포함시키면서도 실제로는 마커 삭제를 1차 신호로 삼는 이유와 같은 문제다. VS Code 는 `code` 셸 스크립트가 기존 인스턴스에 IPC 로 붙을 때 자식을 아예 spawn 하지 않는 경로를 별도로 갖는다.

**더 안전한 변형**: CLI 가 먼저 IPC(유닉스 소켓/네임드 파이프)로 실행 중 앱을 탐지 → 있으면 소켓으로 payload 전달하고 자식을 spawn하지 않음, 없을 때만 앱을 spawn. 이러면 `try_wait` 분기 자체가 필요 없어진다.

#### 3.5 deep link 를 쓰지 말아야 하는 이유

`tauri-plugin-deep-link` (2.4.9) 로 `taide://open?path=...` 를 만들 수는 있지만, **`--wait` 시그널링이 불가능**하다. `open`/`xdg-open`/`Start-Process` 는 URL 을 핸들러에 넘기고 즉시 반환하기 때문이다. deep link 는 "브라우저·문서에서 TAIDE 를 여는 용도"로만 쓰고, **EDITOR 경로는 반드시 CLI 헬퍼 + 마커 파일**로 간다.

#### 3.6 설치 가이드 (사용자 문서용)

```bash
# macOS/Linux
export EDITOR="taide --wait"
export VISUAL="taide --wait"
git config --global core.editor "taide --wait"
```

```json
// ~/.claude/settings.json  — [커뮤니티 출처] 검증 필요
{ "env": { "EDITOR": "taide --wait", "VISUAL": "taide --wait" } }
```

Windows(GitBash)에서는 절대경로를 슬래시로:

```
EDITOR="C:/Program Files/TAIDE/taide.exe --wait"
```

---

### 4. 터미널 pty 자식 프로세스에서 실행 중인 에이전트 감지

목표: TAIDE 내장 터미널의 pty 안에서 지금 `claude` / `codex` / `gemini` 중 무엇이 돌고 있는지 알아내 UI(탭 아이콘, 상태 표시, 컨텍스트 패널)를 바꾼다.

#### 4.1 macOS / Linux — foreground process group

정본 방식은 **pty master fd 에 대한 `tcgetpgrp(3)`** 이다.

> `tcgetpgrp()` returns the process group ID of the foreground process group on the terminal associated to fd. (POSIX, man7)

`portable-pty` 0.9.0 은 이걸 이미 감싸고 있다. `MasterPty` 트레이트:

```rust
/// If applicable to the type of the tty, return the local process id of the
/// process group or session leader
fn process_group_leader(&self) -> Option<pid_t>;

/// If get_termios() and process_group_leader() are both implemented and return Some,
/// then as_raw_fd() should return the same underlying fd associated with the stream.
fn as_raw_fd(&self) -> Option<RawFd>;

fn get_termios(&self) -> Option<Termios>;
```

wezterm 원본 구현(`pty/src/unix.rs`)이 정확히 `tcgetpgrp` 이다:

```rust
fn process_group_leader(&self) -> Option<libc::pid_t> {
    match unsafe { libc::tcgetpgrp(self.fd.0.as_raw_fd()) } {
        pid if pid > 0 => Some(pid),
        _ => None,
    }
}
```

즉 **TAIDE 는 별도 unsafe 코드 없이 `master.process_group_leader()` 만 호출하면 된다.**

pid → 프로세스 이름 해석:

```rust
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Agent { Claude, Codex, Gemini, Other }

pub fn detect_agent(master: &dyn portable_pty::MasterPty) -> Option<Agent> {
    let pgid = master.process_group_leader()?;
    let name = process_name(pgid)?;
    Some(match name.as_str() {
        "claude" => Agent::Claude,
        "codex" => Agent::Codex,
        "gemini" => Agent::Gemini,
        _ => Agent::Other,
    })
}

#[cfg(target_os = "linux")]
fn process_name(pid: i32) -> Option<String> {
    // /proc/<pid>/comm 은 최대 15자로 잘리므로, 긴 이름은 cmdline 을 함께 본다
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    let comm = comm.trim().to_string();
    if comm.len() < 15 { return Some(comm); }
    let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let first = cmdline.split(|b| *b == 0).next()?;
    let s = String::from_utf8_lossy(first);
    Some(Path::new(s.as_ref()).file_name()?.to_string_lossy().to_string())
}

#[cfg(target_os = "macos")]
fn process_name(pid: i32) -> Option<String> {
    // libproc 를 쓰는 편이 빠르지만, 의존성 없이 갈 거면 ps 로도 된다
    let out = std::process::Command::new("ps")
        .args(["-o", "comm=", "-p", &pid.to_string()])
        .output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { return None; }
    Some(Path::new(&s).file_name()?.to_string_lossy().to_string())
}
```

Linux 대안(마스터 fd 없이): `/proc/<shell_pid>/stat` 의 8번째 필드 `tpgid` 가 그 프로세스의 제어 터미널 foreground pgid 다.

주의: `claude` 는 Node 런타임 위에서 돌 수 있어 `comm` 이 `node` 로 보일 수 있다. **`comm` 만 믿지 말고 `cmdline` 전체를 검사**한다(예: `/cmdline` 에 `claude` 가 argv[0] 또는 argv[1] 로 등장). 네이티브 바이너리 설치(2026 기준 권장 설치 경로)에서는 `claude` 로 직접 잡힌다.

#### 4.2 Windows — foreground process group 개념 없음

Windows/ConPTY 에는 프로세스 그룹·`tcgetpgrp` 개념이 없다. **프로세스 트리 탐색**이 유일한 방법이다.

방법:

1. pty 로 띄운 셸의 PID 를 보관한다 (`portable_pty::Child::process_id()`).
2. 전체 프로세스 스냅샷을 떠서 부모→자식 관계로 트리를 구성하고, 셸의 후손 중 관심 있는 실행파일명을 찾는다.
   - Win32: `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)` + `Process32FirstW`/`Process32NextW` → `PROCESSENTRY32W.th32ParentProcessID`, `szExeFile`.
   - 또는 `NtQueryInformationProcess(ProcessBasicInformation)` 의 `InheritedFromUniqueProcessId`.
   - VS Code 는 같은 목적으로 `windows-process-tree` 네이티브 모듈을 쓴다.
3. Rust 에서는 `sysinfo` **0.39.6** 이 크로스 플랫폼으로 이걸 덮는다.

```rust
use sysinfo::{ProcessesToUpdate, System};

pub fn descendant_named(root_pid: u32, names: &[&str]) -> Option<String> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    // parent -> children 인덱스
    let procs = sys.processes();
    let mut stack = vec![sysinfo::Pid::from_u32(root_pid)];
    while let Some(cur) = stack.pop() {
        for (pid, p) in procs {
            if p.parent() == Some(cur) {
                let n = p.name().to_string_lossy().to_lowercase();
                let stem = n.trim_end_matches(".exe").to_string();
                if names.contains(&stem.as_str()) { return Some(stem); }
                stack.push(*pid);
            }
        }
    }
    None
}
```

성능 주의: 전체 스냅샷은 비싸다. **1~2초 주기 폴링 + 변화 없으면 UI 갱신 skip**, 또는 pty 출력이 있을 때만 재검사한다. macOS/Linux 는 `tcgetpgrp` 가 syscall 하나라 부담이 없으므로 플랫폼별로 주기를 다르게 잡는다.

#### 4.3 보조 신호 — 환경변수와 OSC

프로세스 탐색 없이도 쓸 수 있는 보조 신호:

- **`CLAUDECODE=1`** — Claude Code 가 자기 자식 프로세스(Bash 툴, hook, statusline, stdio MCP 서버)에 설정한다. IDE 확장도 통합 터미널에 설정한다. → "이 셸이 Claude Code 안쪽인가" 판정용. 공식 문서 명시.
- **`CLAUDE_CODE_CHILD_SESSION=1`** — Bash/PowerShell/Monitor 툴, hook, statusline 서브프로세스에만 설정되고 stdio MCP 서버에는 설정되지 않는다. 중첩 세션과 최상위 세션을 확실히 구분한다. v2.1.172 이상.
- **OSC 133 셸 통합 시퀀스** — 셸이 프롬프트/명령 시작·종료를 알려주므로, 명령줄 문자열을 직접 잡을 수 있다. 다만 사용자의 셸 rc 설정이 필요해 **의존하지 말고 보조로만** 쓴다.

---

### 5. Claude Code 의 외부 통합 포인트 — IDE 가 활용할 수 있는 것

우선순위 순으로 정리한다.

#### 5.1 [최우선] IDE MCP 서버 — 공식 문서에 명세된 진짜 통합 경로

공식 `vs-code` 문서 "The built-in IDE MCP server" 절 원문 요지:

> When the extension is active, it runs a local MCP server that the CLI connects to automatically. This is how the CLI opens diffs in VS Code's native diff viewer, reads your current selection for `@`-mentions, and — when you're working in a Jupyter notebook — asks VS Code to execute cells.
>
> The server binds to `127.0.0.1` on a **random port in the range 10000–65535**, and the port is not configurable. The transport is unencrypted `ws://`. Each extension activation generates a fresh random auth token, writes it to a lock file at **`~/.claude/ide/<port>.lock`**, and the CLI must present it as the **`X-Claude-Code-Ide-Authorization`** header to connect. The lock file has `0600` permissions in a `0700` directory. If `CLAUDE_CONFIG_DIR` is set, the lock file is written to `$CLAUDE_CONFIG_DIR/ide/` instead.
>
> The server hosts a dozen tools, but only two are visible to the model. The rest are internal RPC the CLI uses for its own UI — opening diffs, reading selections, saving files — and are filtered out before the tool list reaches Claude.

모델에게 보이는 도구 2개(공식 표):

| 도구 | 동작 | read-only |
|---|---|---|
| `mcp__ide__getDiagnostics` | 언어 서버 진단(Problems 패널의 에러/경고) 반환. 파일 단위로 스코프 가능 | Yes |
| `mcp__ide__executeCode` | 활성 Jupyter 노트북 커널에서 Python 실행 (실행 전 네이티브 Quick Pick 확인 필수) | No |

또한 공식 문서:

> **Selection and open-file context.** While connected, the CLI includes your current editor selection and the path of the active file as context on each prompt you send. The transcript shows a `⧉ Selected N lines from <file>` line when this happens.

**TAIDE 는 이 로컬 WebSocket MCP 서버를 직접 구현하면, 아무 확장 설치 없이 `claude` CLI 가 TAIDE 를 IDE 로 인식한다.** 이것이 TAIDE 의 킬러 통합 포인트다.

lock 파일 스키마와 나머지 RPC 이름은 공식 문서에 없다. 아래는 **[커뮤니티 출처: `coder/claudecode.nvim` PROTOCOL.md]** — Neovim 용 구현이 실제로 동작하는 근거로 신뢰도가 높으나 비공식이다.

```jsonc
// ~/.claude/ide/<port>.lock  (권한 0600, 디렉터리 0700)
{
  "pid": 12345,
  "workspaceFolders": ["/path/to/project"],
  "ideName": "TAIDE",
  "transport": "ws",
  "authToken": "32-char lowercase hex token (128 bits) from the OS CSPRNG"
}
```

CLI 가 확인하는 환경변수 **[커뮤니티 출처]**:

- `CLAUDE_CODE_SSE_PORT` — WebSocket 서버 포트
- `ENABLE_IDE_INTEGRATION` — `"true"`

전송: **WebSocket + JSON-RPC 2.0**. 인증 헤더 `x-claude-code-ide-authorization: <authToken>` (HTTP 헤더는 대소문자 무관, 공식 표기는 `X-Claude-Code-Ide-Authorization`).

IDE → Claude 알림 **[커뮤니티 출처]**:

- `selection_changed` — 사용자 선택 변경 시
- `at_mentioned` — 사용자가 선택 영역을 컨텍스트로 명시 전송할 때

Claude → IDE 도구 호출 (12개) **[커뮤니티 출처]**:

`openFile`, `openDiff`(블로킹), `getCurrentSelection`, `getLatestSelection`, `getOpenEditors`, `getWorkspaceFolders`, `getDiagnostics`, `checkDocumentDirty`, `saveDocument`, `close_tab`, `closeAllDiffTabs`, `executeCode`

TAIDE 서버 구현 뼈대:

```rust
// 1) 127.0.0.1:0 로 바인드해 커널이 준 포트를 받는다 (10000-65535 범위 강제 필요 시 재시도)
// 2) 32자 소문자 hex 토큰 생성 (getrandom)
// 3) ~/.claude/ide/<port>.lock (또는 $CLAUDE_CONFIG_DIR/ide/) 에 0600 으로 기록,
//    디렉터리는 0700
// 4) WebSocket 업그레이드 시 x-claude-code-ide-authorization 헤더를 상수시간 비교
// 5) JSON-RPC 2.0 으로 tools/list, tools/call 처리
// 6) 앱 종료 시 lock 파일 삭제 (Drop / 시그널 핸들러 양쪽)
```

수명주기 주의: 워크스페이스가 바뀌면 `workspaceFolders` 를 갱신해야 하고(CLI 는 워크스페이스 매칭으로 후보를 거른다), 앱이 비정상 종료하면 stale lock 이 남는다 → 시작 시 `pid` 가 살아있지 않은 lock 파일을 청소한다.

#### 5.2 Hooks — 이벤트 구독

`settings.json` 의 `hooks` 로 Claude Code 의 거의 모든 라이프사이클에 붙을 수 있다. TAIDE 는 프로젝트 `.claude/settings.json` 에 hook 을 심어 **HTTP 훅으로 자기 자신에게 이벤트를 흘려보낼 수 있다.**

주요 이벤트(공식 hooks 문서):

- 세션당 1회: `SessionStart`, `Setup`, `SessionEnd`
- 턴당 1회: `UserPromptSubmit`, `UserPromptExpansion`, `Stop`, `StopFailure`
- 툴 호출당: `PreToolUse`, `PermissionRequest`, `PermissionDenied`, `PostToolUse`, `PostToolUseFailure`, `PostToolBatch`
- 비동기/기타: `Notification`, `MessageDisplay`, `SubagentStart`, `SubagentStop`, `TaskCreated`, `TaskCompleted`, `TeammateIdle`, `InstructionsLoaded`, `ConfigChange`, `CwdChanged`, `DirectoryAdded`, `FileChanged`, `WorktreeCreate`, `WorktreeRemove`, `PreCompact`, `PostCompact`, `Elicitation`, `ElicitationResult`

stdin 으로 들어오는 공통 JSON:

```json
{
  "session_id": "abc123",
  "prompt_id": "550e8400-e29b-41d4-a716-446655440000",
  "transcript_path": "/home/user/.claude/projects/.../transcript.jsonl",
  "cwd": "/home/user/my-project",
  "permission_mode": "default|plan|acceptEdits|auto|dontAsk|bypassPermissions",
  "hook_event_name": "PreToolUse",
  "effort": { "level": "low|medium|high|xhigh|max" }
}
```

툴 이벤트에는 `tool_name`, `tool_input`, `tool_use_id` 가 추가된다.

종료 코드: `0` 성공(stdout 을 JSON 으로 파싱), `2` 블로킹 에러(stderr 를 Claude/사용자에게 노출하고 동작 차단), 그 외는 non-blocking.

TAIDE 에 가장 유용한 조합 — **HTTP 훅으로 TAIDE 로컬 서버에 이벤트 전송**:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write|NotebookEdit",
        "hooks": [
          { "type": "http", "url": "http://127.0.0.1:PORT/claude/hook", "timeout": 5 }
        ]
      }
    ],
    "SessionStart": [
      { "hooks": [{ "type": "http", "url": "http://127.0.0.1:PORT/claude/hook" }] }
    ],
    "Stop": [
      { "hooks": [{ "type": "http", "url": "http://127.0.0.1:PORT/claude/hook" }] }
    ]
  }
}
```

활용: `PostToolUse(Edit|Write)` → TAIDE 파일 트리·열린 탭 즉시 리로드. `Stop` → 완료 알림/사운드. `Notification` → OS 알림. `CwdChanged` → TAIDE 워크스페이스 동기화.

matcher 규칙: `"*"`/`""`/생략 = 전체 매칭. 문자·숫자·`_`·`-`·공백·`,`·`|` 만 있으면 정확 문자열 또는 리스트. 그 외 문자가 있으면 **unanchored JS 정규식**.

HTTP 훅 응답: 2xx + 빈 바디 = 성공, 2xx + 평문 = 컨텍스트로 추가, 2xx + JSON = command hook 과 동일 스키마. non-2xx/연결실패 = non-blocking 에러(진행됨).

#### 5.3 Status line — 세션 상태 읽기

`settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "~/.claude/statusline.sh",
    "padding": 2,
    "refreshInterval": 5
  }
}
```

stdin 으로 오는 JSON(공식 전체 스키마 발췌):

```json
{
  "cwd": "/current/working/directory",
  "session_id": "abc123...",
  "session_name": "my-session",
  "prompt_id": "550e8400-...",
  "transcript_path": "/path/to/transcript.jsonl",
  "model": { "id": "claude-opus-5", "display_name": "Opus" },
  "workspace": {
    "current_dir": "...", "project_dir": "...", "added_dirs": [],
    "git_worktree": "feature-xyz",
    "repo": { "host": "github.com", "owner": "anthropics", "name": "claude-code" }
  },
  "version": "2.1.90",
  "output_style": { "name": "default" },
  "cost": { "total_cost_usd": 0.01234, "total_duration_ms": 45000,
            "total_api_duration_ms": 2300, "total_lines_added": 156, "total_lines_removed": 23 },
  "context_window": {
    "total_input_tokens": 15500, "total_output_tokens": 1200,
    "context_window_size": 200000, "used_percentage": 8, "remaining_percentage": 92,
    "current_usage": { "input_tokens": 8500, "output_tokens": 1200,
                       "cache_creation_input_tokens": 5000, "cache_read_input_tokens": 2000 }
  },
  "exceeds_200k_tokens": false,
  "fast_mode": false,
  "effort": { "level": "high" },
  "thinking": { "enabled": true },
  "rate_limits": {
    "five_hour": { "used_percentage": 23.5, "resets_at": 1738425600 },
    "seven_day": { "used_percentage": 41.2, "resets_at": 1738857600 }
  }
}
```

기타 필드: `vim.mode`, `agent.name`, `pr.number`/`pr.url`/`pr.review_state`, `worktree.name`/`path`/`branch`/`original_cwd`/`original_branch`.

갱신 시점(공식): 세션 시작(재개 포함) 1회 → 이후 새 assistant 메시지 도착 / `/compact` 완료 / permission mode 변경 / vim mode 토글 / `refreshInterval` 타이머. **300ms 디바운스**, 실행 중 새 트리거가 오면 진행 중 스크립트를 취소한다.

출력: 여러 줄 가능, ANSI 컬러 가능, **OSC 8 하이퍼링크 지원**.

**TAIDE 활용**: statusline 커맨드를 TAIDE 가 소유한 작은 바이너리로 지정하면, 컨텍스트 사용률·비용·rate limit 을 **TAIDE 네이티브 UI(상태바)에 그대로 표시**할 수 있다. stdout 으로는 빈 문자열이나 최소 텍스트를 내보내고, 실제 데이터는 TAIDE 로컬 서버에 POST 하면 된다.

주의: 캐시 파일명을 `$$`/`pid` 로 만들면 매 호출마다 바뀌어 캐시가 무효화된다 → **`session_id` 를 사용**하라는 공식 권고가 있다.

#### 5.4 Deep link — `claude-cli://`

공식 `deep-links` 문서:

- 형식은 `claude-cli://open` 하나만 허용. 파라미터: `q`(프롬프트, URL 인코딩, `%0A` 개행, 최대 5000자), `cwd`(절대경로, UNC/네트워크 경로 및 보이지 않는 제어문자 포함 경로는 거부), `repo`(GitHub `owner/name` 슬러그).
- `cwd` 와 `repo` 를 같이 주면 `cwd` 가 이긴다.
- 핸들러는 **인터랙티브 세션에서 첫 프롬프트를 보낼 때** 등록된다. 사용자 레벨 위치에만 기록:
  - macOS `~/Applications/Claude Code URL Handler.app`
  - Linux `$XDG_DATA_HOME/applications` (기본 `~/.local/share/applications`) 의 `claude-code-url-handler.desktop`
  - Windows `HKEY_CURRENT_USER\Software\Classes\claude-cli`
- 터미널 선택: macOS 는 **최근 인터랙티브 세션에서 쓴 터미널을 기억해 재사용**(iTerm2, Ghostty, kitty, Alacritty, WezTerm, Terminal.app). Linux 는 `$TERMINAL` → `x-terminal-emulator` → 일반 목록. Windows 는 Windows Terminal → PowerShell → cmd.exe 고정 순서.
- 프롬프트는 채워지기만 하고 **자동 전송되지 않는다.** `Prompt from an external link` 경고가 뜬다.
- `disableDeepLinkRegistration: "disable"` 로 등록 차단 가능.

**TAIDE 활용**: TAIDE 내부에서 "이 파일/선택 영역에 대해 Claude 에게 물어보기" 버튼을 만들 때 참고 모델. 단 TAIDE 는 자체 내장 pty 가 있으므로 deep link 를 거치지 말고 pty 에 직접 쓰는 편이 낫다.

VS Code 확장 쪽 핸들러(참고): `vscode://anthropic.claude-code/open?prompt=<urlencoded>&session=<id>`

#### 5.5 CLI/명령 표면

- `claude --ide` — 유효한 IDE 가 정확히 하나면 시작 시 자동 연결.
- `/ide` — IDE 통합 관리 및 상태 표시.
- `CLAUDE_CODE_AUTO_CONNECT_IDE` — 자동 연결 override(`false` 차단, `true` 강제; tmux 로 부모 터미널이 가려질 때 `true`). `autoConnectIde` 설정보다 우선.
- `CLAUDE_CODE_IDE_SKIP_AUTO_INSTALL=1` — IDE 확장 자동 설치 차단.
- `disableAllHooks: true` — 모든 hook 과 커스텀 statusline 비활성화.

---

### 6. VS Code 용 Claude Code 확장의 통합 방식 (참고)

TAIDE 설계 시 벤치마킹 대상. 공식 `vs-code` 문서 기준.

**두 가지 모드가 공존한다.**

1. **그래픽 패널 모드(기본)** — 확장이 CLI 사본을 자체 번들해 실행하고, 네이티브 웹뷰 UI 를 그린다. `claudeProcessWrapper` 설정으로 외부 `claude` 바이너리를 지정할 수도 있다. **확장 설치만으로는 `claude` 가 PATH 에 추가되지 않는다.**
2. **터미널 모드** — `claudeCode.useTerminal: true`. 통합 터미널에서 CLI 를 그대로 띄운다.

**CLI ↔ 에디터 연결은 §5.1 의 IDE MCP 서버**로 이뤄진다. 확장이 활성화되면 로컬 `ws://127.0.0.1:<random>` 서버를 띄우고 lock 파일을 쓰고, 통합 터미널에서 `claude` 를 실행하면 자동으로 붙는다. 외부 터미널이면 `/ide` 로 수동 연결.

제공 기능:

- **네이티브 diff 뷰어** — 편집 제안을 side-by-side 로 보여주고 권한을 묻는다. **diff 뷰에서 사용자가 직접 수정하면 Claude 에게 "수정됨"이 통지**되어 원본 제안과 파일이 같다고 가정하지 않는다.
- **선택 영역 자동 컨텍스트** — 연결 중에는 현재 선택과 활성 파일 경로가 매 프롬프트에 자동 포함되고, transcript 에 `⧉ Selected N lines from <file>` 로 표시된다. 제외하려면 해당 경로에 `Read` deny 룰을 건다.
- **`Option+K` / `Alt+K`** — 선택 영역을 `@file.ts#5-10` 형태 at-mention 으로 프롬프트에 삽입.
- **`Cmd+Esc` / `Ctrl+Esc`** — 에디터 ↔ Claude 입력 포커스 토글.
- **진단 공유** — `mcp__ide__getDiagnostics` 로 Problems 패널 내용 전달.
- **checkpoints / rewind** — 메시지 hover 로 "Fork conversation from here", "Rewind code to here", "Fork and rewind".
- **`@terminal:name`** — 터미널 출력을 프롬프트에 참조.
- **autosave** (기본 `true`) — Claude 가 읽거나 쓰기 전에 파일 자동 저장.
- **plan 모드** — 플랜을 풀 마크다운 문서로 열어 인라인 코멘트로 피드백.
- **URI 핸들러** `vscode://anthropic.claude-code/open`.

CLI 대비 확장의 한계(공식 표): 명령/스킬은 일부만, MCP 서버 설정은 부분적(추가는 CLI, 관리는 `/mcp`), `!` bash 단축은 없음, 탭 완성 없음.

**TAIDE 가 가져가야 할 것**: (a) IDE MCP 서버를 직접 구현해 CLI 가 자동 연결되게 할 것, (b) `openDiff` 를 TAIDE 네이티브 diff 뷰어에 연결할 것, (c) 선택 영역·활성 파일을 `selection_changed` 로 흘려보낼 것, (d) at-mention 삽입 단축키를 제공할 것, (e) autosave 정책을 채택할 것(Claude 가 읽기 전 저장 → 스테일 컨텍스트 방지).

---

## TAIDE 적용 가이드

### 우선순위별 로드맵

**P0 — IDE MCP 서버 (가장 큰 ROI)**

TAIDE 가 `~/.claude/ide/<port>.lock` 을 쓰고 `ws://127.0.0.1:<port>` JSON-RPC 서버를 띄우면, **사용자가 TAIDE 내장 터미널에서 `claude` 를 치는 것만으로** diff 뷰어·선택 영역 컨텍스트·진단 공유가 전부 살아난다. 확장 설치도, 사용자 설정도 필요 없다.

구현 체크리스트:

- [ ] `127.0.0.1` 바인드, 포트 10000–65535 확보
- [ ] OS CSPRNG 32자 소문자 hex 토큰
- [ ] lock 디렉터리 `0700`, lock 파일 `0600` (Windows 는 ACL 로 현재 사용자만)
- [ ] `CLAUDE_CONFIG_DIR` 존재 시 `$CLAUDE_CONFIG_DIR/ide/` 사용
- [ ] `x-claude-code-ide-authorization` 헤더 상수시간 비교
- [ ] JSON-RPC 2.0 `tools/list`, `tools/call`
- [ ] 최소 구현 도구: `openFile`, `openDiff`, `getCurrentSelection`, `getOpenEditors`, `getWorkspaceFolders`, `getDiagnostics`, `checkDocumentDirty`, `saveDocument`, `close_tab`
- [ ] 알림: `selection_changed`, `at_mentioned`
- [ ] pty 자식 환경에 `ENABLE_IDE_INTEGRATION=true`, `CLAUDE_CODE_SSE_PORT=<port>` 주입 (커뮤니티 스펙 — 실측 검증 필요)
- [ ] 워크스페이스 변경 시 lock 갱신, 종료 시 삭제, 시작 시 stale lock 청소

**P1 — CLI 헬퍼 (`taide --wait`)**

§3 그대로. VS Code 의 마커 파일 방식을 채택한다. 이것이 없으면 사용자가 `EDITOR=taide` 를 설정했을 때 Claude Code `Ctrl+G` 뿐 아니라 `git commit`, `kubectl edit`, `crontab -e` 가 전부 깨진다.

**P2 — pty 에이전트 감지**

macOS/Linux 는 `portable_pty::MasterPty::process_group_leader()` (내부적으로 `tcgetpgrp`), Windows 는 `sysinfo` 로 프로세스 트리 탐색. 감지 결과로 탭 배지·툴바를 전환한다. 보조 신호로 `CLAUDECODE`/`CLAUDE_CODE_CHILD_SESSION` 를 함께 본다.

**P3 — hooks / statusline 브리지**

TAIDE 가 프로젝트 `.claude/settings.local.json` (gitignore 대상)에 자기 HTTP 훅과 statusline 커맨드를 주입한다. **사용자 동의 UI 를 반드시 거치고, 기존 사용자 설정을 덮어쓰지 않는다.**

- `PostToolUse(Edit|Write|NotebookEdit)` → 파일 트리/탭 리로드
- `Stop`, `Notification` → OS 알림
- `CwdChanged` → 워크스페이스 동기화
- statusline → 컨텍스트 사용률·비용·rate limit 을 TAIDE 상태바에 표시

### 아키텍처 배치 제안

```
src-tauri/
  src/
    ide_mcp/          # P0: WebSocket JSON-RPC 서버 + lock 파일 라이프사이클
      server.rs
      lockfile.rs
      tools.rs
    terminal/
      pty.rs          # portable-pty 래핑
      agent_detect.rs # P2: tcgetpgrp / 프로세스 트리
    editor_bridge/
      wait_marker.rs  # P1: 마커 파일 생성/삭제/청소
    claude_bridge/
      hooks_http.rs   # P3: 로컬 HTTP 수신 엔드포인트
      settings_inject.rs
crates/
  taide-cli/          # P1: 얇은 CLI 헬퍼 바이너리 (앱 번들과 분리)
```

---

## 함정·주의

1. **`Ctrl+G` 는 파일 저장이 아니라 "에디터 프로세스 종료"로 완료를 판단한다 [커뮤니티 출처, 강한 정황증거].** 인스턴스 재사용 런처는 즉시 exit 하므로 편집 내용이 통째로 무시된다. TAIDE CLI 헬퍼는 **절대 조기 종료하면 안 된다.**

2. **`code --wait` 는 "저장"이 아니라 "탭 닫힘"에 반응한다.** TAIDE 가 이걸 흉내낼 때, 사용자가 저장만 하고 탭을 안 닫으면 호출자는 계속 대기한다. UI 에서 "저장 후 탭을 닫아야 Claude 에 반영됩니다" 힌트를 반드시 노출한다.

3. **마커 파일이 영원히 안 지워지면 Claude Code 가 영구 hang 한다.** 안전망 필수: (a) 탭 열기 실패 시 즉시 삭제(VS Code 도 그렇게 한다), (b) 앱 종료 시 미해결 마커 전부 삭제, (c) CLI 측 타임아웃 상한(예: 30분) 고려.

4. **`whenDeleted` 는 1초 폴링이다.** watcher 로 바꾸고 싶어도 네트워크 FS·Windows 에서 신뢰도가 떨어진다. 200~500ms 폴링이 현실적 타협.

5. **`promptEditor` 설정 키는 공식 문서에 없다.** 2차 자료를 믿고 문서·코드에 넣지 마라. 현재 공식 확정 경로는 `$VISUAL`/`$EDITOR`(transcript `v` 는 명시), `Ctrl+G` 는 미명시.

6. **Issue #18990 은 아직 open** — Ctrl+G 가 IDE 를 자동 감지하고 `EDITOR` 를 무시한다는 보고가 살아 있다. TAIDE 가 IDE 로 자동 감지되면 오히려 유리하지만, **감지 로직은 비공개이고 변한다.** 기능 설계를 여기에 걸지 마라.

7. **SSH/TTY 함정 (#19076, closed as not planned)** — 터미널 에디터는 SSH 세션에서 TTY attach 에 실패할 수 있다. TAIDE 는 GUI 앱이라 이 문제를 피하지만, **원격 개발(SSH remote) 시나리오에서는 TAIDE 가 원격 머신에 없다** → EDITOR 통합이 성립하지 않는다. 원격은 IDE MCP 서버 경로(포트 포워딩)로 풀어야 한다.

8. **GUI 에디터 열림 중 터미널 상태 오염** — CHANGELOG 2.1.216 이 "GUI editor 열려 있는 동안 마우스/포커스 잔상"을 수정했고, 2.1.136 이 Backspace 스왑을, 2.1.129 가 대화 히스토리 blank 를 고쳤다. TAIDE 내장 터미널이 이 escape 시퀀스를 정확히 처리하는지 실기기 검증이 필요하다. **tsc·유닛테스트로는 절대 안 잡힌다.**

9. **`externalEditorContext` 가 켜져 있으면 임시파일 상단에 `#` 주석 블록이 붙는다.** Claude Code 가 저장 시 이를 제거하므로, TAIDE 가 임의로 포맷팅/린팅해서 주석 구조를 깨면 안 된다. 임시파일(tmpdir 하위, 확장자 없음)에는 **포맷터/린터/LSP 를 붙이지 말 것.**

10. **`comm` 은 Linux 에서 15자로 잘린다.** 그리고 `claude` 가 Node 위에서 돌면 `node` 로 보인다. 반드시 `/proc/<pid>/cmdline` 까지 본다.

11. **Windows 는 프로세스 그룹 개념이 없다.** `tcgetpgrp` 등가물이 존재하지 않는다. 트리 탐색은 비싸므로 폴링 주기를 플랫폼별로 분리한다.

12. **IDE lock 파일의 stale 처리.** 앱 크래시 시 lock 이 남아 `claude` 가 죽은 IDE 에 붙으려다 실패한다(#36284, #16434 계열). 시작 시 `pid` 생존 확인으로 청소한다. WSL/원격 환경에서 lock 이 지워지는 사례도 보고되어 있다(#14421).

13. **IDE MCP 서버의 세부 프로토콜(12개 도구, 알림 이름, `CLAUDE_CODE_SSE_PORT`)은 비공식 리버스 엔지니어링 결과다.** 공식 문서가 보증하는 것은 lock 파일 경로, `ws://127.0.0.1`, 포트 범위, 인증 헤더 이름, 파일 권한, `mcp__ide__getDiagnostics`/`mcp__ide__executeCode` 두 개뿐이다. **버전 업으로 깨질 수 있으니 실측 테스트를 CI 에 넣어라.**

14. **`ws://` 는 평문이다.** 공식 문서도 인정한다("the socket is loopback-only, any process that could capture the traffic can also read the token from the lock file"). 반드시 `127.0.0.1` 에만 바인드하고, `0.0.0.0` 을 절대 쓰지 마라.

15. **hook/statusline 을 사용자 설정 파일에 자동 주입하는 것은 침습적이다.** 프로젝트 `.claude/settings.json` 은 커밋 대상이라 팀원에게 퍼진다. **`.claude/settings.local.json`(gitignore) 을 쓰고, 사용자 동의를 받고, 제거 UI 를 제공하라.**

16. **statusline 은 300ms 디바운스 + 실행 중 취소**된다. 무거운 스크립트를 두면 표시가 밀린다. TAIDE statusline 바이너리는 **로컬 소켓에 fire-and-forget POST 후 즉시 종료**해야 한다.

17. **npm 설치는 deprecated.** 문서·설치 가이드에서 `npm i -g @anthropic-ai/claude-code` 를 안내하지 마라.

---

## 참고 링크

### 공식 문서 (Claude Code)

- Interactive mode (Ctrl+G, Ctrl+X Ctrl+E, transcript `v`): https://code.claude.com/docs/en/interactive-mode
- Settings (`externalEditorContext`, `editorMode`, `disableAllHooks`): https://code.claude.com/docs/en/settings
- Settings (raw markdown, grep 용): https://code.claude.com/docs/en/settings.md
- Environment variables (`CLAUDECODE`, `CLAUDE_CODE_CHILD_SESSION`, `CLAUDE_CODE_AUTO_CONNECT_IDE`): https://code.claude.com/docs/en/env-vars
- Hooks (전체 이벤트·JSON 스키마·exit code·HTTP/MCP 훅): https://code.claude.com/docs/en/hooks
- Status line (JSON 스키마·갱신 트리거·ANSI/OSC8): https://code.claude.com/docs/en/statusline
- VS Code 확장 + built-in IDE MCP server (lock 파일·포트·인증 헤더): https://code.claude.com/docs/en/vs-code
- Deep links (`claude-cli://`): https://code.claude.com/docs/en/deep-links
- CLI reference (`--ide`): https://code.claude.com/docs/en/cli-reference
- Commands (`/ide`): https://code.claude.com/docs/en/commands
- MCP: https://code.claude.com/docs/en/mcp
- Terminal configuration: https://code.claude.com/docs/en/terminal-config

### Claude Code 저장소 / 이슈

- CHANGELOG (버전 확정 정본): https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md
- #18990 외부 에디터 설정 가능화 요청 (open): https://github.com/anthropics/claude-code/issues/18990
- #19076 SSH 에서 Ctrl+G 무반응 (closed as not planned): https://github.com/anthropics/claude-code/issues/19076
- #9218 Ctrl+G 터미널 손상: https://github.com/anthropics/claude-code/issues/9218
- #50032 escape 시퀀스 누출: https://github.com/anthropics/claude-code/issues/50032
- #4922 외부 에디터 지원 최초 요청: https://github.com/anthropics/claude-code/issues/4922
- #36284 lock 파일 미생성: https://github.com/anthropics/claude-code/issues/36284
- #16434 Windows lock 파일 PID 불일치: https://github.com/anthropics/claude-code/issues/16434
- #14421 WSL 통합 터미널에서 lock 삭제: https://github.com/anthropics/claude-code/issues/14421
- npm 패키지: https://www.npmjs.com/package/@anthropic-ai/claude-code

### VS Code CLI `--wait` 원본 소스

- 마커 파일 생성: https://raw.githubusercontent.com/microsoft/vscode/main/src/vs/platform/environment/node/wait.ts
- CLI 대기 로직: https://raw.githubusercontent.com/microsoft/vscode/main/src/vs/code/node/cli.ts
- `whenDeleted` (1초 폴링): https://raw.githubusercontent.com/microsoft/vscode/main/src/vs/base/node/pfs.ts
- 마커 삭제 측 (`trackClosedWaitFiles`): https://raw.githubusercontent.com/microsoft/vscode/main/src/vs/workbench/electron-browser/window.ts
- CLI 서버 (`preferNewWindow`): https://raw.githubusercontent.com/microsoft/vscode/main/src/vs/workbench/api/node/extHostCLIServer.ts
- #150368 `--wait` 를 파일 저장 기준으로: https://github.com/microsoft/vscode/issues/150368
- #744 `--wait` 최초 도입: https://github.com/microsoft/vscode/issues/744

### IDE MCP 프로토콜 (비공식 / 리버스 엔지니어링)

- `coder/claudecode.nvim` PROTOCOL.md: https://github.com/coder/claudecode.nvim/blob/main/PROTOCOL.md

### pty / 프로세스 감지

- `portable-pty` `MasterPty` 트레이트: https://docs.rs/portable-pty/latest/portable_pty/trait.MasterPty.html
- wezterm `pty/src/unix.rs` (`tcgetpgrp` 구현): https://raw.githubusercontent.com/wez/wezterm/main/pty/src/unix.rs
- `tcgetpgrp(3)` man page: https://man7.org/linux/man-pages/man3/tcsetpgrp.3.html
- `NtQueryInformationProcess`: https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntqueryinformationprocess
- ConPTY 소개: https://devblogs.microsoft.com/commandline/windows-command-line-introducing-the-windows-pseudo-console-conpty/
- `vscode-windows-process-tree`: https://github.com/microsoft/vscode-windows-process-tree

### 크레이트

- tauri: https://crates.io/crates/tauri
- tauri-plugin-single-instance: https://crates.io/crates/tauri-plugin-single-instance
- tauri-plugin-deep-link: https://crates.io/crates/tauri-plugin-deep-link
- portable-pty: https://crates.io/crates/portable-pty
- sysinfo: https://crates.io/crates/sysinfo

### 커뮤니티 자료 (검증 필요)

- "Setting Default Editor for Claude Code" gist (2.1.70 기준 `env.EDITOR`, 인스턴스 재사용 함정): https://gist.github.com/maddada/6eec96f4c8b467b81d69d291d4ac130e
- ClaudeLog 외부 에디터 FAQ: https://claudelog.com/faqs/how-to-edit-prompt-in-editor-in-claude-code/
