# TAIDE 1차 지원 언어 LSP 서버 선정과 운용

조사일: 2026-08-06. 모든 버전은 공식 레지스트리(npm/PyPI/crates.io)와 GitHub Releases API로 당일 확인했다.
확인하지 못한 항목은 본문에 **미확인**으로 명시했다. 추측은 사실로 적지 않았다.

---

## 버전 확정 (2026-08 기준)

### JS/TS

| 후보 | 최신 버전 | 배포 | 라이선스 | 최근 릴리스 | 비고 |
|---|---|---|---|---|---|
| TypeScript 7 내장 LSP (`tsc --lsp`) | **7.0.2** (npm `typescript`) | npm + 플랫폼별 optionalDependencies 바이너리 | Apache-2.0 | 7.0 GA 2026-07-08 | Go 네이티브 포트. LSP 네이티브 지원 |
| **vtsls** (`@vtsls/language-server`) | **0.3.0** | npm (Node 18+) | MIT | npm 2025-12-24 | VSCode TS 확장을 LSP로 래핑 |
| typescript-language-server | **5.3.0** | npm (Node ≥20) | Apache-2.0 | 2026-05-21 | tsserver 프로토콜 브리지 |
| `@typescript/native-preview` (tsgo) | 7.0.0-dev.20260707.2 | npm | Apache-2.0 | 2026-07-07 | **폐기 경로**. TS 7.0 GA로 대체됨 |

- `typescript` npm `dist-tags`: `latest=7.0.2`, `rc=7.0.1-rc`, `beta=6.0.0-beta`, `next=7.1.0-dev.20260805.1`.
- TypeScript 7.0의 npm `bin`은 **`tsc` 하나뿐**이다 (`tsgo`가 아님). 실행 바이너리는 `@typescript/typescript-{darwin,linux,win32}-{x64,arm64}` 등 20개 플랫폼 optionalDependencies로 배포된다.

### Rust

| 항목 | 값 |
|---|---|
| rust-analyzer 최신 릴리스 | **2026-08-03** (주간 릴리스, 태그 = 날짜) |
| nightly 태그 | 2026-08-05 (prerelease) |
| 라이선스 | Apache-2.0 (+ MIT 듀얼, 리포지토리 관례) |
| 바이너리 크기 | `rust-analyzer-aarch64-apple-darwin.gz` 13.2MB / `x86_64-apple-darwin.gz` 13.9MB / `x86_64-unknown-linux-gnu.gz` 14.2MB / `x86_64-pc-windows-msvc.zip` 16.7MB |

### Python

| 후보 | 최신 버전 | 배포 | 라이선스 | 최근 릴리스 |
|---|---|---|---|---|
| **basedpyright** | **1.39.9** | PyPI (+npm, brew, VSIX) | MIT | 2026-06-27 |
| pyright | **1.1.411** | npm only | MIT | 2026-06-25 |
| ruff (lint/format LSP) | **0.16.1** | PyPI | MIT | 2026-07-30 |
| pyrefly (Meta) | **1.2.0** | PyPI | 미확인(Apache-2.0 추정 — 확인 필요) | 2026-08-01 |
| ty (Astral) | **0.0.67** | PyPI | 미확인 | 2026-08-05 |

- basedpyright wheel: `basedpyright-1.39.9-py3-none-any.whl` **12.8MB** (플랫폼 무관, Node 런타임 필요).
- pyright npm 패키지 unpacked **18.4MB**.

### Markdown

| 항목 | 값 |
|---|---|
| marksman 최신 릴리스 | **2026-02-08** (태그가 날짜 형식) |
| 라이선스 | MIT (Copyright (c) 2021 Artem Pyanykh) |
| 마지막 push | 2026-02-08 (약 6개월 정체 — 유지보수 속도 유의) |
| 바이너리 크기 | `marksman-macos` 41.8MB / `marksman-linux-x64` 21.5MB / `marksman-linux-arm64` 20.8MB / `marksman.exe` 19.6MB |
| macOS 바이너리가 universal2인지 | **미확인** (릴리스 자산이 `marksman-macos` 하나뿐, 41.8MB로 x64/arm64 fat 가능성이 높으나 검증 안 함) |

---

## 핵심 API·사용법

### 1. JS/TS — vtsls (권장 기본값)

설치·기동:

```bash
npm install -g @vtsls/language-server   # 또는 앱 내장 node_modules
vtsls --stdio
```

`--stdio`가 기본값이고 `--version`/`-V`로 버전 확인이 가능하다 (`packages/server/src/index.ts`의 CLI 파서 확인).

`initialize` 요청 예시:

```jsonc
{
  "processId": 12345,
  "clientInfo": { "name": "TAIDE", "version": "0.1.0" },
  "rootUri": null,
  "workspaceFolders": [
    { "uri": "file:///Users/me/proj", "name": "proj" }
  ],
  "initializationOptions": {
    "hostInfo": "taide",          // vtsls가 읽는 필드
    "tsLogPath": "/tmp/taide/tsserver.log"
  },
  "capabilities": {
    "workspace": {
      "workspaceFolders": true,
      "configuration": true,
      "didChangeConfiguration": { "dynamicRegistration": true },
      "didChangeWatchedFiles": { "dynamicRegistration": true },
      "fileOperations": { "didRename": true }
    },
    "textDocument": {
      "synchronization": { "didSave": true },
      "completion": { "completionItem": { "labelDetailsSupport": true, "snippetSupport": true } },
      "rename": { "prepareSupport": true },
      "inlayHint": { "dynamicRegistration": true },
      "semanticTokens": { "requests": { "full": true, "range": true }, "tokenTypes": [], "tokenModifiers": [], "formats": ["relative"] },
      "codeAction": { "codeActionLiteralSupport": { "codeActionKind": { "valueSet": [] } }, "resolveSupport": { "properties": ["edit"] } }
    }
  }
}
```

vtsls는 `initializationOptions`가 아니라 **`workspace/configuration` 응답 및 `workspace/didChangeConfiguration`으로 설정을 받는다.** 네임스페이스는 VSCode와 동일한 `typescript.*` / `javascript.*` 에 vtsls 고유 `vtsls.*`가 추가된다.

```jsonc
// workspace/didChangeConfiguration 의 settings
{
  "typescript": {
    "tsserver": { "maxTsServerMemory": 4096, "useSyntaxServer": "auto" },
    "inlayHints": {
      "parameterNames": { "enabled": "literals", "suppressWhenArgumentMatchesName": true },
      "parameterTypes": { "enabled": true },
      "variableTypes": { "enabled": true },
      "propertyDeclarationTypes": { "enabled": true },
      "functionLikeReturnTypes": { "enabled": true },
      "enumMemberValues": { "enabled": true }
    },
    "preferences": {
      "importModuleSpecifier": "shortest",
      "includeCompletionsForModuleExports": true,
      "quoteStyle": "single"
    },
    "updateImportsOnFileMove": { "enabled": "always" }
  },
  "javascript": { "inlayHints": { "parameterNames": { "enabled": "literals" } } },
  "vtsls": {
    "autoUseWorkspaceTsdk": true,
    "experimental": { "completion": { "enableServerSideFuzzyMatch": true } },
    "tsserver": { "globalPlugins": [] }
  }
}
```

Vue/Svelte 등 embedded language는 `vtsls.tsserver.globalPlugins`에 `{ name, location, languages, configNamespace }`를 넣어 TS 플러그인으로 붙인다 (nvim-lspconfig `vtsls.lua` 문서에 동일 패턴).

### 2. JS/TS — TypeScript 7 네이티브 LSP (전략적 목표)

```bash
npm install typescript@7            # 프로젝트 로컬
./node_modules/.bin/tsc --lsp --stdio
```

- `tsc --lsp --stdio`가 LSP 기동 커맨드다 (TypeScript 7.0 GA 블로그 및 nvim-lspconfig 마이그레이션 이슈 #4467 기준). **로컬에서 실행 검증은 하지 않았다 — 도입 시 실측 필요.**
- 서버는 `serverInfo.name = "typescript-go"`로 응답한다.
- `initializationOptions` 스키마(`internal/lsp/lsproto/lsp_generated.go`의 `InitializationOptions`):

```jsonc
{
  "disablePushDiagnostics": true,        // pull diagnostics만 쓸 때
  "codeLensShowLocationsCommandName": "taide.showReferences",
  "userPreferences": { /* TS userPreferences + formatting options */ },
  "enableTelemetry": false,
  "logVerbosity": "off",
  "trackFlakyDiagnostics": null
}
```

- 설정(inlay hints 등)은 `workspace/didChangeConfiguration`의 `typescript.inlayHints.*` 로 준다 (nvim-lspconfig `tsgo.lua`가 쓰는 그 형태).
- **push/pull diagnostics 양쪽을 지원한다.** `diagnosticProvider = { identifier: "typescript", interFileDependencies: true }` 를 선언하고, 동시에 push도 하므로 `disablePushDiagnostics: true`로 중복을 끈다.

### 3. Rust — rust-analyzer

```bash
# 사용자 환경 우선
rustup component add rust-analyzer     # ~/.cargo/bin/rust-analyzer 프록시 생성
rustup component add rust-src          # 표준 라이브러리 소스(필수)

# 또는 standalone
curl -L https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-aarch64-apple-darwin.gz \
  | gunzip -c - > ~/.local/bin/rust-analyzer && chmod +x ~/.local/bin/rust-analyzer

# 기동: 인자 없이 stdin/stdout LSP
rust-analyzer
```

**rust-analyzer는 `--stdio` 플래그가 없다.** 인자 없이 실행하면 stdio 서버가 된다.

설정은 **`initializationOptions`에 `rust-analyzer.` prefix를 벗긴 트리를 그대로** 넣는다 (rust-analyzer `docs/dev/lsp-extensions.md` 규약, nvim-lspconfig `before_init`가 이 변환을 수행):

```jsonc
{
  "initializationOptions": {
    "cargo": { "features": "all", "buildScripts": { "enable": true } },
    "procMacro": { "enable": true },
    "check": { "command": "clippy" },
    "checkOnSave": true,
    "inlayHints": {
      "typeHints": { "enable": true },
      "parameterHints": { "enable": true },
      "chainingHints": { "enable": true },
      "closureReturnTypeHints": { "enable": "with_block" }
    },
    "lens": { "enable": true, "run": { "enable": true }, "implementations": { "enable": true } },
    "linkedProjects": ["/abs/path/a/Cargo.toml", "/abs/path/b/Cargo.toml"],
    "files": { "excludeDirs": ["target", "node_modules"] }
  }
}
```

유용한 커스텀 요청/알림:

- `rust-analyzer/reloadWorkspace` — Cargo 워크스페이스 재적재
- `experimental.serverStatusNotification: true`를 클라이언트 capability에 넣으면 인덱싱 상태 알림 수신
- experimental capabilities 선언: `externalDocs`, `hoverRange`, `joinLines`, `matchingBrace`, `moveItem`, `onEnter`, `openCargoToml`, `parentModule`, `childModules`, `runnables`, `ssr`, `workspaceSymbolScopeKindFiltering`

### 4. Python — basedpyright (권장)

```bash
uv tool install basedpyright        # 또는 pip install basedpyright
basedpyright-langserver --stdio
```

pyright를 쓸 경우 `pyright-langserver --stdio` (npm 설치).

설정은 `workspace/configuration` / `didChangeConfiguration` 으로 전달한다. basedpyright는 `basedpyright.*`와 `python.*` 를 모두 읽는다.

```jsonc
{
  "python": { "pythonPath": "/abs/path/.venv/bin/python" },
  "basedpyright": {
    "analysis": {
      "autoSearchPaths": true,
      "diagnosticMode": "openFilesOnly",     // 대안: "workspace" (무겁다)
      "typeCheckingMode": "standard",        // basedpyright 기본은 "recommended" (매우 엄격)
      "inlayHints": {
        "variableTypes": true,
        "callArgumentNames": true,
        "functionReturnTypes": true,
        "genericTypes": false
      }
    },
    "disableTaggedHints": true               // unreachable/deprecated hint 소음 억제
  }
}
```

- 인터프리터 전환은 `python.pythonPath`를 바꾸고 `workspace/didChangeConfiguration`을 보내면 반영된다 (nvim-lspconfig `set_python_path` 패턴).
- `workspace/executeCommand`의 `basedpyright.organizeimports` (pyright는 `pyright.organizeimports`)는 **capabilities에 광고되지 않는 private 커맨드**다. 클라이언트가 `executeCommandProvider.commands` 화이트리스트만 보고 호출을 막으면 안 된다.
- lint/format은 타입체커가 담당하지 않는다. `ruff server` (ruff 0.16.1)를 **두 번째 LSP로 동시에 붙이는 구성**이 사실상 표준이다.

### 5. Markdown — marksman

```bash
# macOS
brew install marksman
# 또는 릴리스 바이너리 다운로드 후 chmod +x

marksman server        # stdin/stdout LSP ("Start LSP server on stdin/stdout")
marksman --verbose server
```

- 프로젝트 루트 마커: `.marksman.toml` 또는 `.git`. **둘 중 하나가 없으면 크로스파일 기능(참조/완성)이 동작하지 않는다** (공식 FAQ).
- `initializationOptions` 없음. 설정은 워크스페이스 루트의 `.marksman.toml` 파일로 한다.
- 트리거 문자: `[`, `#`, `(`.

---

## TAIDE 적용 가이드

### 언어별 최종 추천

| 언어 | 1차 채택 | 이유 | 후속 |
|---|---|---|---|
| JS/TS | **vtsls 0.3.0** | 다중 워크스페이스 폴더 정식 지원, VSCode와 동일한 리팩터링/코드액션 세트, MIT, 단일 프로세스 모노레포 처리 | TS 7 `tsc --lsp`를 **옵션 백엔드**로 병행 준비 |
| Rust | **rust-analyzer (2026-08-03)** | 사실상 유일한 선택지. pull diagnostics까지 지원 | 주간 릴리스 추종 정책 필요 |
| Python | **basedpyright 1.39.9** + `ruff server` 0.16.1 | PyPI 배포로 설치 감지가 쉬움, Pylance 전용이던 inlay hints/semantic tokens가 오픈소스로 들어옴, pull diagnostics | pyrefly 1.2.0 재평가(2026 하반기) |
| Markdown | **marksman (2026-02-08)** | 자체완결 바이너리, MIT, workspaceFolders 지원 | 유지보수 정체 시 대안 재조사 |

### JS/TS를 vtsls로 시작하되 TS7을 추상화해 두는 이유

TypeScript 7.0은 2026-07-08 GA로 **8~12배 빠른 빌드 + LSP 네이티브 + 멀티스레드**를 가져왔고, 장기적으로 TAIDE의 기본값이 되어야 한다. 다만 지금 바로 기본으로 못 박기 어려운 사유가 있다.

1. TS 7은 **안정적인 programmatic API를 아직 노출하지 않는다.** Vue/Svelte/Angular/MDX/Astro 같은 embedded language 워크플로는 TypeScript 6.0이 필요하고, 7.1까지 기다려야 한다(공식 GA 공지).
2. TS 7 LSP는 `workspace.workspaceFolders` 서버 capability를 **선언하지 않는다.** `initialized` 처리에서 `workspaceFolders`의 길이가 **정확히 1일 때만** 그 폴더를 cwd로 삼고, 그 외에는 `rootUri`/`rootPath`/프로세스 cwd로 폴백한다(`internal/lsp/server.go`). 즉 다중 루트는 프로세스 분리로 풀어야 한다.
3. TS 6/7은 `target: es5`, `downlevelIteration`, 레거시 `moduleResolution` 제거 등 breaking change가 크다. 사용자 프로젝트가 TS 6 이하면 TS 7 서버로 열 수 없다.

→ **설계 지침**: `LanguageServerSpec { id, cmd, args, initializationOptions, settingsNamespace }` 형태로 서버를 데이터로 기술하고, JS/TS는 `vtsls | tsgo | tsls` 중 선택 가능한 설정 키를 노출한다. 프로젝트의 `typescript` 의존성 major가 7 이상이면 `tsc --lsp --stdio`를 자동 제안하는 정도가 안전하다.

### 프로세스 모델 권장안

- **서버 인스턴스 키 = (언어 서버 id, 프로젝트 루트)** 로 잡는다.
- vtsls / basedpyright / marksman / rust-analyzer 4종 모두 `workspace.workspaceFolders.supported = true`, `changeNotifications = true`를 선언한다. 따라서 TAIDE가 여러 프로젝트를 하나의 창에서 열 때 **서버 프로세스를 재사용하고 `workspace/didChangeWorkspaceFolders`로 폴더를 추가**하는 모델이 성립한다.
- 다만 rust-analyzer는 실무상 워크스페이스 루트별 프로세스 분리를 권장한다(아래 함정 참조).

### 배포 전략 — 로컬 감지 우선, sidecar는 선택적

권장은 **하이브리드**다.

1. **감지 순서**: 프로젝트 로컬(`node_modules/.bin`, `.venv/bin`) → 사용자 PATH / rustup 프록시 → TAIDE 관리 디렉터리(`~/Library/Application Support/TAIDE/servers/…`) → 온디맨드 다운로드.
2. 앱 번들에 전부 넣지 않는다. 4종 전부 sidecar로 넣으면 macOS arm64 기준으로만 rust-analyzer 13.2MB + marksman 41.8MB + Node 런타임(vtsls/basedpyright용) 수십 MB가 더해져 **설치 용량이 100MB 단위로 늘고, 서버 업데이트가 앱 릴리스에 묶인다.**
3. 온디맨드 다운로드 시 **SHA256 고정 + 서명 검증**을 반드시 넣는다.

| 서버 | 라이선스 | 재배포 부담 | 크기(대표) | 업데이트 주기 | 권장 |
|---|---|---|---|---|---|
| rust-analyzer | Apache-2.0(+MIT) | 낮음(고지 필요) | 13.2MB(gz, mac arm64) | 주 1회 | rustup 감지 → 없으면 다운로드 |
| marksman | MIT | 낮음 | 41.8MB(macOS) | 느림(2026-02 이후 정체) | 다운로드. 번들은 크기 대비 손해 |
| vtsls | MIT (단, 실행에 Node 필요) | 낮음 | npm 소형 + Node 런타임 | 불규칙(2025-12) | Node 감지 → npm 설치 |
| typescript(tsc 7) | Apache-2.0 | 낮음 | 플랫폼 바이너리 | 잦음 | **프로젝트 로컬 의존성만 사용** |
| basedpyright | MIT | 낮음 | wheel 12.8MB + Node 런타임 | 잦음(월 단위) | `uv tool install` 유도 |
| pyright | MIT | 낮음 | npm 18.4MB | 월 단위 | basedpyright 대체재 |

라이선스는 4종 모두 MIT/Apache-2.0으로 **재배포에 법적 장애가 없다.** 다만 Apache-2.0(rust-analyzer, typescript)은 NOTICE/저작권 고지 동봉 의무가 있으므로 앱 내 "오픈소스 라이선스" 화면이 필요하다. Pylance(마이크로소프트 폐쇄 확장)는 **어떤 형태로도 번들할 수 없다** — basedpyright를 쓰는 근거 중 하나다.

Tauri 2에서 굳이 sidecar로 갈 경우(예: marksman만 번들):

```jsonc
// src-tauri/tauri.conf.json
{ "bundle": { "externalBin": ["binaries/marksman"] } }
```

파일은 `binaries/marksman-aarch64-apple-darwin` 처럼 **타깃 트리플 접미사**를 붙여 둔다(`rustc --print host-tuple`로 확인). Rust 측 기동:

```rust
use tauri_plugin_shell::ShellExt;

let cmd = app.shell().sidecar("marksman")?.args(["server"]);
let (mut rx, mut child) = cmd.spawn()?;
```

프런트에서 직접 띄우려면 capability에 권한이 필요하다.

```jsonc
// src-tauri/capabilities/default.json
{ "permissions": [
  { "identifier": "shell:allow-execute",
    "allow": [{ "name": "binaries/marksman", "sidecar": true, "args": ["server"] }] }
] }
```

참고 버전: `tauri` 2.11.5, `tauri-plugin-shell` 2.3.5 (crates.io, 2026-08-06 확인).

**권장은 LSP 프로세스를 Rust 코어에서 직접 관리하는 것**이다. 프런트에서 shell 권한을 열면 임의 인자 실행 표면이 생기고, stdio 프레이밍(`Content-Length` 헤더) 처리도 Rust 쪽이 훨씬 안정적이다.

---

## 함정·주의

### 공통

- **stdio 프레이밍**: 모든 서버가 `Content-Length: N\r\n\r\n` + UTF-8 JSON 이다. `Content-Type` 헤더는 오지 않을 수 있으니 필수로 파싱하지 말 것.
- **positionEncoding**: LSP 3.17의 `general.positionEncoding` 협상을 반드시 하라. TS 7 서버는 `positionEncoding`을 명시 응답한다. 협상하지 않으면 기본은 UTF-16이고, Rust 쪽 문자열(UTF-8 바이트 인덱스)과 오프셋이 어긋나 이모지/한글에서 커서가 밀린다.
- **pull diagnostics 중복**: rust-analyzer, TS 7, basedpyright 모두 pull(`textDocument/diagnostic`)과 push(`publishDiagnostics`)를 함께 낼 수 있다. TS 7은 `initializationOptions.disablePushDiagnostics: true`, basedpyright는 클라이언트가 pull capability를 선언하면 서버가 동적으로 pull 기능을 등록한다. **한쪽만 소비하도록 정하고 UI 중복 표시를 막을 것.**

### JS/TS

- vtsls는 `documentLinkProvider`를 **의도적으로 비활성**한다(소스에서 `undefined`). Markdown/JSDoc 링크 클릭은 기대하지 말 것.
- vtsls는 `declarationProvider`, `colorProvider`, `monikerProvider`, `typeHierarchyProvider`, `inlineValueProvider`가 모두 false다.
- vtsls는 저장 알림(`textDocumentSync.save`)이 false다. 저장 기반 워크플로를 서버에 의존하지 말 것.
- vtsls는 대형 프로젝트에서 OOM 사례가 보고된다. `typescript.tsserver.maxTsServerMemory`를 노출하고, 프로세스 죽으면 재기동하는 supervisor를 반드시 둘 것.
- **typescript-language-server는 단일 루트다.** 소스의 `getWorkspaceFolders()`가 `this.workspaceRoot` 하나만 배열로 감싸 반환하고, 서버 capabilities에 `workspace.workspaceFolders`가 없다. 다중 폴더 IDE에는 부적합 → TAIDE 채택 비추천.
- TS 7은 embedded language(Vue/Svelte/Astro) 지원이 **7.1까지 불가**다. 해당 프로젝트는 vtsls + TS 플러그인 경로로 폴백해야 한다.
- vtsls와 typescript-language-server를 **동시에 붙이면 안 된다**(중복 진단·중복 코드액션).
- Deno 프로젝트에서 TS 서버가 붙지 않도록 `deno.json`/`deno.lock`을 루트 탐지에서 제외하는 로직이 필요하다(nvim-lspconfig가 쓰는 "가장 가까운 lockfile 비교" 알고리즘이 검증된 참고 구현).

### Rust

- **rust-analyzer 설정은 `settings`가 아니라 `initializationOptions`에 `rust-analyzer.` prefix를 뗀 채로 넣어야 한다.** 이걸 틀리면 조용히 전부 기본값으로 돈다. 가장 흔한 실수다.
- `rust-src` 컴포넌트가 없으면 std 정의로 이동이 실패한다. 설치 여부를 감지해 사용자에게 안내할 것.
- **다중 루트가 실질적으로 취약하다.** 서버는 `workspaceFolders.supported = true`를 선언하지만, `linkedProjects`가 항상 워크스페이스의 첫 번째 폴더를 기준 디렉터리로 잡는 이슈(rust-analyzer #16791, #16030)가 알려져 있다. **Cargo 워크스페이스 루트당 프로세스 1개**가 안전하다.
- `workspace_diagnostics: false`다. pull diagnostics는 파일 단위만 되고 워크스페이스 전체 진단은 못 받는다.
- `type_hierarchy_provider`, `linked_editing_range_provider`, `execute_command_provider`가 없다. 코드액션은 `codeActionProvider` 경로로만 처리된다.
- 루트 탐지는 `cargo metadata --no-deps`로 `workspace_root`를 구하는 것이 정확하다. 단순히 가장 가까운 `Cargo.toml`을 쓰면 워크스페이스 멤버 크레이트마다 서버가 뜬다.
- `~/.cargo/registry/src`, `~/.rustup/toolchains` 아래 파일(의존성 소스로 점프한 경우)은 **새 서버를 띄우지 말고 기존 클라이언트에 붙여야** 한다.

### Python

- basedpyright의 기본 `typeCheckingMode`는 pyright보다 엄격하다. 사용자 프로젝트에 그대로 적용하면 진단이 폭증한다. TAIDE 기본은 `standard`로 낮추되, 프로젝트의 `pyproject.toml`/`pyrightconfig.json` 설정이 있으면 **덮어쓰지 말 것**.
- 같은 이유로 `useLibraryCodeForTypes`를 클라이언트에서 명시 지정하는 것은 공식 문서가 권장하지 않는다(프로젝트 설정을 무력화한다).
- `diagnosticMode: "workspace"`는 대형 리포에서 CPU/메모리를 크게 먹는다. 기본은 `openFilesOnly`.
- 인터프리터(`python.pythonPath`)를 못 잡으면 서드파티 임포트가 전부 미해결로 뜬다. venv 탐지는 필수 기능으로 간주할 것.
- basedpyright/pyright는 **포매팅 프로바이더가 없다**(`documentOnTypeFormattingProvider`만 있고 `documentFormattingProvider` 없음). 포맷은 ruff 등 별도 서버/도구가 담당해야 한다.
- pyright는 npm 전용이라 Python 사용자에게 Node 설치를 강요한다 — basedpyright(PyPI)를 택한 실무적 근거다. 단 basedpyright도 **런타임은 Node**이며 wheel에 Node를 포함하지 않는다(별도 확인 필요 항목: wheel이 Node 런타임을 동봉하는지 여부는 **미확인**).

### Markdown

- `.marksman.toml` 또는 `.git`이 없는 폴더에서는 크로스파일 기능이 죽는다. TAIDE가 새 폴더를 열 때 이 조건을 확인해 사용자에게 안내하거나 빈 `.marksman.toml`을 제안할 것.
- marksman은 클라이언트가 **VSCode로 식별되면** `documentSymbolProvider`와 `workspaceSymbolProvider`를 끈다(`not clientDesc.IsVSCode`). `clientInfo.name`을 `"Visual Studio Code"`로 위장하지 말 것 — 기능이 사라진다. `"TAIDE"`로 보내면 정상적으로 켜진다.
- `didRename` 파일 오퍼레이션은 **의도적으로 비활성**이다. 파일 이름 변경 시 `didClose`(구 경로) + `didOpen`(신 경로)를 클라이언트가 보내 줘야 상태가 동기화된다.
- inlay hints, pull diagnostics, formatting, signature help가 **전부 없다**. 진단은 push(`publishDiagnostics`)로만 온다.
- 마지막 릴리스가 2026-02-08이다. 6개월간 신규 릴리스가 없으므로 유지보수 리스크를 인지하고 도입할 것.

### 서버별 capabilities 비교

`initialize` 응답의 실제 선언값을 소스에서 직접 확인한 결과다.

| 기능 | vtsls 0.3.0 | tsc 7 (typescript-go) | tsls 5.3.0 | rust-analyzer | basedpyright | marksman |
|---|---|---|---|---|---|---|
| rename (prepare) | O (prepare O) | O (prepare 조건부) | O (클라 지원 시 prepare) | O (prepare O) | O (prepare O) | O (prepare 조건부) |
| inlay hints | O | O | O | O (resolve 조건부) | O | X |
| semantic tokens | full + range | full + range | full + range | full **delta** + range | full only | full + range |
| pull diagnostics | **X** (push only) | **O** | **X** (push only) | **O** (workspace 진단은 X) | **O** (동적 등록) | **X** (push only) |
| code lens | O (resolve) | O (resolve) | O (resolve) | O | X | O |
| call hierarchy | O | O | O (TS 3.8+) | O | O | X |
| type hierarchy | X | 미확인 | X | X | X | X |
| declaration | X | X | X | **O** | O | X |
| implementation / typeDefinition | O / O | O / O | O / O | O / O | O / O | X / X |
| document formatting | O (range/onType 포함) | O | O (range 포함) | O | onType only | X |
| folding range | O | O | O | O | X | X |
| selection range | O | 미확인 | O | O | X | X |
| linked editing | O | O | X (false) | X | X | X |
| signature help | O | O | O | O | O | X |
| workspace symbol | O | O | O | O | O | O (비-VSCode 한정) |
| document symbol | O | O | O | O | O | O (비-VSCode 한정) |
| executeCommand | O | O | O | **X** | O | O (빈 목록) |
| notebook 동기화 | X | X | X | X | **O** (jupyter) | X |
| **workspaceFolders 지원** | **O** (changeNotifications O) | **X** (미선언, 단일 cwd) | **X** (단일 root) | **O** (changeNotifications O) | **O** (changeNotifications O) | **O** (changeNotifications O) |
| 파일 오퍼레이션 | didRename | 미확인 | X | willRename | willRename | didCreate/didDelete |

### 하나의 서버 프로세스를 여러 프로젝트가 공유 가능한가

| 서버 | 공유 가능 여부 | 근거 / 권장 |
|---|---|---|
| **vtsls** | **가능** | `workspace.workspaceFolders.supported=true` + `onDidChangeWorkspaceFolders` 핸들러 존재. tsserver가 파일별로 가장 가까운 `tsconfig.json`을 찾아 프로젝트를 나눠 관리하므로 **모노레포는 프로세스 1개로 충분**하다. 단 무관한 프로젝트를 다수 붙이면 메모리가 선형 증가하고 OOM 위험이 커진다 → 3~4개 루트 정도로 상한을 두는 것을 권장 |
| **tsc 7 LSP** | **불가(권장하지 않음)** | 서버가 workspaceFolders capability를 선언하지 않고, 폴더가 1개일 때만 그 폴더를 cwd로 삼는다. 프로젝트당 프로세스 1개 |
| typescript-language-server | 불가 | 단일 `workspaceRoot` 모델 |
| **rust-analyzer** | 이론상 가능, **실무상 분리 권장** | capability는 지원하지만 다중 루트 이슈가 남아 있다. Cargo 워크스페이스 루트당 1 프로세스. 한 워크스페이스 안의 여러 크레이트는 당연히 1 프로세스로 처리된다 |
| **basedpyright / pyright** | **가능** | `WorkspaceFactory`가 폴더별 워크스페이스를 관리하고 `handleWorkspaceFoldersChanged`로 동적 추가/제거를 처리한다. 다만 **폴더마다 인터프리터가 다르면** `python.pythonPath`를 `workspace/configuration`의 scopeUri별 응답으로 다르게 줘야 한다 |
| **marksman** | **가능** | `extractWorkspaceFolders`로 초기 폴더 목록을 읽고 `WorkspaceDidChangeWorkspaceFolders`를 구현한다. 폴더별로 독립 folder 상태를 유지하므로 공유가 자연스럽다 |

**TAIDE 구현 지침**: 서버를 "프로젝트 종속"이 아니라 "폴더 집합을 갖는 세션"으로 모델링하고, 폴더 추가/제거 시 서버가 workspaceFolders를 지원하면 `workspace/didChangeWorkspaceFolders`, 아니면 프로세스를 새로 띄우는 두 가지 전략을 `LanguageServerSpec`의 플래그로 분기하라.

---

## 참고 링크

### JS/TS
- TypeScript 7.0 GA 공지 (2026-07-08): https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/
- TypeScript 7.0 RC: https://devblogs.microsoft.com/typescript/announcing-typescript-7-0-rc/
- typescript-go LSP 서버 소스 (capabilities, InitializationOptions): https://github.com/microsoft/typescript-go/blob/main/internal/lsp/server.go
- nvim-lspconfig — tsgo → TypeScript 7.0(tsc) 마이그레이션 이슈: https://github.com/neovim/nvim-lspconfig/issues/4467
- vtsls 리포지토리: https://github.com/yioneko/vtsls
- vtsls 서버 capabilities 소스: https://github.com/yioneko/vtsls/blob/main/packages/server/src/capabilities.ts
- vtsls npm: https://www.npmjs.com/package/@vtsls/language-server
- nvim-lspconfig `vtsls.lua` (실전 설정 예시): https://github.com/neovim/nvim-lspconfig/blob/master/lsp/vtsls.lua
- nvim-lspconfig `tsgo.lua`: https://github.com/neovim/nvim-lspconfig/blob/master/lsp/tsgo.lua
- typescript-language-server 설정 문서: https://github.com/typescript-language-server/typescript-language-server/blob/master/docs/configuration.md
- typescript-language-server 릴리스: https://github.com/typescript-language-server/typescript-language-server/releases

### Rust
- rust-analyzer 릴리스: https://github.com/rust-lang/rust-analyzer/releases
- 바이너리 설치 문서: https://rust-analyzer.github.io/book/rust_analyzer_binary.html
- 설정 레퍼런스: https://rust-analyzer.github.io/book/configuration.html
- LSP 확장 문서 (initializationOptions 규약): https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/lsp-extensions.md
- capabilities 소스: https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/lsp/capabilities.rs
- 다중 루트 이슈: https://github.com/rust-lang/rust-analyzer/issues/16791 , https://github.com/rust-lang/rust-analyzer/issues/16030
- nvim-lspconfig `rust_analyzer.lua`: https://github.com/neovim/nvim-lspconfig/blob/master/lsp/rust_analyzer.lua

### Python
- basedpyright 문서: https://docs.basedpyright.com/
- basedpyright 리포지토리: https://github.com/DetachHead/basedpyright
- basedpyright PyPI: https://pypi.org/project/basedpyright/
- pyright 리포지토리: https://github.com/microsoft/pyright
- 언어서버 설정 문서: https://docs.basedpyright.com/latest/configuration/language-server-settings/
- 타입체커 비교 (Posit): https://opensource.posit.co/blog/2026-03-31_python-type-checkers/
- 타입체커 비교 (pydevtools): https://pydevtools.com/handbook/explanation/how-do-mypy-pyright-and-ty-compare/
- pyrefly IDE 문서: https://pyrefly.org/en/docs/IDE/
- ruff PyPI: https://pypi.org/project/ruff/

### Markdown
- marksman 리포지토리: https://github.com/artempyanykh/marksman
- marksman 릴리스: https://github.com/artempyanykh/marksman/releases
- capabilities 소스: https://github.com/artempyanykh/marksman/blob/main/Marksman/Server.fs

### Tauri 배포
- Tauri 2 sidecar 문서: https://v2.tauri.app/develop/sidecar/
- tauri-plugin-shell: https://crates.io/crates/tauri-plugin-shell
