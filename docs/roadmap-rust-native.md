# Rust-native 전환 로드맵

> 상태: Phase 0 진행 중 — IPC contract manifest 완료, Phase 1 model crate 첫 slice 진행 중
> 계약: `docs/acknowledge/2026-09-23-rust-native-transition-contract.md`
> 검증: `docs/quality-assurance/2026-09-23-rust-native-parity-plan.md`
> 현행 구조: `docs/architecture.md`

## 1. 기준선

| 영역 | 현재 규모·구조 | 전환 원칙 |
| --- | --- | --- |
| Rust | 163파일, 71,565줄, 25개 도메인 | 서비스·타입·인프라와 테스트를 우선 재사용한다. |
| TypeScript·TSX·CSS | 874파일, 93,645줄 | UI를 기능 slice별로 대체하며 마지막까지 fallback으로 유지한다. |
| IPC | Rust 생성 `bindings.ts` 3,358줄, command/event/raw channel | 두 UI가 공유하는 Rust application facade로 먼저 치환한다. |
| 검증 | TS·Rust 테스트 파일 294개, QA6·성능 기준선 | 현행 계약을 golden fixture와 native parity gate로 승격한다. |

현행 Rust 코어는 단순 Tauri bridge가 아니다. `AppState`, project capability, Git, PTY, LSP process, watcher, remote·IDE server가 이미 Rust에 있으므로 전환의 중심은 알고리즘 재작성보다 Tauri DI·event·window lifecycle을 명시적 Rust port로 바꾸고 native editor·terminal·UI를 만드는 일이다.

## 2. 목표 crate 경계

| crate | 책임 | 금지 의존 |
| --- | --- | --- |
| `taide-model` | ID, 직렬화 DTO, error, persistence schema, `AppEvent` | UI toolkit, Tauri, OS window handle |
| `taide-core` | project/layout/file/git/search/settings/theme/agent 정책, document transaction | UI toolkit, HTTP/WebSocket wire, native window API |
| `taide-infra` | filesystem, PTY, LSP process, watcher, persist, archive, HTTP, keyring 구현 | UI crate, protocol consumer |
| `taide-runtime` | `AppServices`, Tokio runtime, task supervisor, store registry, event bus, shutdown | UI widget, remote wire decoder |
| `taide-platform` | window, menu, dialog, notification, clipboard, drag/drop, external open, preview host | domain 상태 직접 수정 |
| `taide-editor` | document/view store, transaction·undo, selection, layout, decorations, input·IME | Tauri IPC, remote wire |
| `taide-lsp` | server detection/install, process, JSON-RPC, document mirror, generation/replay | UI toolkit, editor renderer |
| `taide-terminal` | terminal parser/model/history/effects/input encoding, renderer-facing snapshot | Tauri, xterm, UI shell 상태 |
| `taide-ui` | native app shell, panels, tabs/splits, commands, theme, accessibility | persistence·remote protocol 직접 접근 |
| `taide-remote` | 기존 HTTP·WebSocket protocol과 기본 거부 dispatch | UI crate, raw state lock |
| `taide-ide` | IDE/MCP protocol adapter | UI crate, raw state lock |
| `taide-app` | composition root, 명시적 dependency injection, 실행 파일 | 도메인 로직 재구현 |

의존 방향은 `taide-app → UI·platform·protocol adapter → runtime·core·infra → model`로 제한한다. `taide-model`이 최하위이며 `taide-app`만 전체를 조립한다. `Arc<AppServices>`는 composition root가 만들고 명시적으로 주입하며 전역 mutable singleton을 만들지 않는다.

현재 Git의 libgit2 호출은 별도 `infra/repo.rs`가 아니라 `domain/git/service.rs`의 정책·DTO와 한 구현입니다. M3에서 독립성이 없는 Git 래퍼를 만들지 않고, M4에서 Git 구현과 테스트를 기능별 서비스 crate로 함께 이전합니다.

## 3. Tauri 제거를 가능하게 하는 선행 분리

| 현행 결합 | 목표 port·adapter |
| --- | --- |
| `AppHandle::emit`과 named Tauri event | `EventSink::publish(AppEvent)` |
| `Manager::state::<T>()` | 명시 필드를 가진 `AppServices` |
| Tauri window 목록·focus·close | `WindowRegistry` |
| dialog·notification·menu·external URL | `PlatformServices` |
| `tauri::async_runtime` | `TaskSupervisor`가 소유하는 Tokio runtime과 cancellation |
| `#[tauri::command]`·`State<T>` | toolkit과 무관한 application action facade |
| Specta·생성 `bindings.ts` | Rust DTO와 native action/event 타입 |
| raw `Channel<T>` | in-process bounded stream 또는 typed subscription |

기존 Tauri command와 새 native UI는 전환 기간에 같은 facade를 호출한다. 따라서 facade 추출이 끝날 때까지 command signature와 event payload를 변경하지 않는다.

## 4. Native editor와 LSP

### 4.1 편집기 상태

- `DocumentStore`: canonical URI별 `DocumentId`, rope buffer, revision, disk baseline, dirty·read-only·encoding·size tier, undo history를 단일 소유한다.
- `ViewStore`: split별 `ViewId`, selection set, multi-cursor, scroll, fold, viewport, IME composition을 소유한다.
- `Transaction`: typing, paste, format, AI edit, workspace edit, external restore를 동일한 edit·selection·undo group 형식으로 적용한다.
- `Presentation`: syntax, Git gutter·blame, diagnostics, semantic token, inlay hint, search·peek을 revision-tagged decoration layer로 합성한다.
- `TextLayout`: grapheme, bidi, ligature, font fallback, emoji와 hit-test를 제공하고 byte·scalar·grapheme·UTF-16 위치 변환은 indexed line map 한 곳에서 처리한다.

초기 후보는 Ropey, Tree-sitter, COSMIC Text지만 확정하지 않는다. `M0` spike에서 한글 IME, emoji·bidi, multi-cursor, undo, 50,000줄 스크롤과 accessibility를 통과한 뒤 pinned version·MSRV·license를 승인한다.

### 4.2 LSP 단일 상태기계

`LspCoordinator`가 process, JSON-RPC transport, server capability, generation, pending request, document mirror를 함께 소유한다.

| 상태 | 진입·종료 계약 |
| --- | --- |
| `Detected` | executable, root, manifest가 확정됨 |
| `Installing` | checksum·archive·권한 검증 후 `Detected` 또는 `Degraded` |
| `Spawning` | process·stdio를 만들고 generation을 배정 |
| `Initializing` | generation마다 initialize handshake 한 번만 허용 |
| `Running` | initialize, capability 등록, 열린 문서 replay가 모두 끝남 |
| `Degraded` | 원인 코드와 마지막 stderr tail을 노출하고 무한 `connecting`을 금지 |
| `Restarting` | pending request 실패·취소, generation 증가, process 교체 |
| `Stopping`·`Stopped` | listener 제거, didClose/shutdown/exit/kill과 child reap 완료 |

모든 요청은 session generation, document revision, timeout, cancellation을 가진다. crash 후에는 initialize → 열린 문서 snapshot `didOpen` replay → capability 재등록 → generation 확인을 완료한 뒤에만 `Running`으로 전환한다.

TypeScript/JavaScript의 Monaco 내장 worker를 제거하기 전에 Rust-owned LSP가 unavailable일 때의 syntax-only fallback 또는 명시적 degraded UX를 구현한다.

## 5. Native terminal

### 5.1 구조

- 기존 `portable-pty`, writer ordering, resize/kill, pause, roster와 raw ring을 우선 유지한다.
- 세션당 하나의 `TerminalCore`가 raw bytes를 파싱해 primary/alternate grid, history, cursor, mode, selection basis, damage와 `TerminalEffects`를 생성한다.
- `TerminalEffects`는 OSC 7 cwd, OSC 8 hyperlink, OSC 9·777 notification, OSC 133 command marker, title, 안전한 normalized text를 제공한다.
- 현재 `OutputScanner`와 terminal emulator의 이중 파싱은 shadow comparison 뒤 제거한다.
- renderer는 glyph atlas, font fallback, shaping, color·decoration·cursor, selection, hyperlink hit-test, damage-only draw를 소유한다.
- input adapter는 OS key·text·IME·mouse·wheel을 terminal mode에 맞는 PTY bytes로 인코딩한다.

### 5.2 후보와 선행 gate

첫 headless 비교 후보는 `alacritty_terminal`, 두 번째는 `wezterm-term`이다. parser-only `vte`는 grid·history·input·selection을 모두 직접 구현해야 하므로 우선 후보가 아니다. GPU substrate는 `wgpu`를 검토하되 terminal widget을 제공한다고 가정하지 않는다.

선행 Go/No-Go gate는 native surface의 pane bounds·clip·z-order·DPI·IME·split 이동·보조 창을 검증하는 것이다. 이 gate 전에는 xterm 또는 Tauri 자산을 삭제하지 않는다.

## 6. Native UI와 preview

### 6.1 UI framework spike

같은 fixture로 `egui/eframe`와 `iced`의 shell spike를 비교한다. GPUI는 pre-1.0 API와 접근성·외부 drop 불확실성 때문에 editor surface 연구 후보로만 둔다. Slint는 외부 앱 DnD가 하드 게이트를 통과하기 전 주 후보에서 제외한다. 전 후보가 실패하면 `winit + wgpu + AccessKit` custom toolkit의 장기 비용을 별도 승인한다.

공통 spike 범위는 다음과 같다.

- 실제 창 2개, shell slot, tab 이동, 내부·외부 파일 DnD
- native menu, context menu, clipboard, file dialog, notification
- 10,000행 virtual tree, 50개 tab, theme·locale 전환
- VoiceOver와 키보드 전용 조작, 한글·일본어·중국어 IME
- GPU device loss와 software fallback, release signing·packaging

접근성, CJK IME, 다중 창, 외부 파일 drop, native menu·dialog 중 하나라도 구현 불가하면 성능과 무관하게 탈락한다.

### 6.2 Preview 이관

| 형식 | 목표 경로 | 완료 조건 |
| --- | --- | --- |
| image·SVG | Rust decoder + GPU texture, SVG sanitize 또는 raster | zoom·error·대형·비신뢰 fixture 동등 |
| video·audio | OS media backend port 또는 격리 helper | 인앱 재생·seek·pause·lifecycle 동등 |
| PDF | PDFium 계열 격리 renderer/helper spike | page, zoom, text selection, 손상 PDF crash 격리 |
| HTML | 권한 없는 별도 WebView/helper | script 금지, parent bridge 없음, root 제한 resource |
| XLS·XLSX·CSV | Calamine 후보 + native table | sheet·값·500행 정책과 error fixture 동등 |
| PPTX | Rust OOXML outline + 설치된 LibreOffice PDF fallback | 현행 outline·면책·이미지/텍스트 동등 |
| HWP·HWPX | Rust-hosted WASM/helper를 먼저 격리 | 현행 SVG 결과·오류·lifecycle 동등, 외부 앱만으로 대체 금지 |

## 7. 실행 단계

### Phase 0 — 계약과 baseline 고정

- command/event/raw channel, remote allow·deny, IDE/MCP, CLI marker와 persistence fixture를 golden contract로 고정한다.
- `docs/quality-assurance/2026-09-04-perf-baseline.md`의 비어 있는 실기 지표를 동일 기기·fixture에서 3회 중앙값으로 채운다.
- editor, LSP, terminal, preview, shell의 기능 inventory에 현행 자동·실기 증거를 연결한다.
- 완료: 이후 모든 phase가 비교할 기능·성능·데이터 기준선이 존재한다.

### Phase 1 — model·core·infra crate 분리

- model, error, event, persistence schema와 순수 service를 workspace crate로 이동한다.
- Tauri command는 새 application facade를 호출하되 기존 UI 동작을 바꾸지 않는다.
- 첫 slice인 `taide-model`은 `ids`·`error`부터 실제 이전합니다. 전체 도메인 이전은 `docs/PROCESS.md`의 M1~M7 체크리스트를 따르며, Phase 0의 나머지 기준선과 기존 앱 회귀 확인을 병행합니다.
- crate는 미리 정한 개수·이름에 끼워 맞추지 않고 기능별 책임과 실제 의존 DAG에 따라 나눕니다. 공통 `taide-model` DTO 이동만으로 기능별 서비스 분리가 완료된 것으로 보지 않습니다.
- 완료: domain boundary, session restore, persistence, 기존 전체 검증이 새 경계에서 통과한다.
- 롤백: 기존 Tauri adapter가 같은 facade 위에 남는다.

### Phase 2 — runtime·platform port 분리

- `AppServices`, `TaskSupervisor`, `EventSink`, `WindowRegistry`, `PlatformServices`를 도입한다.
- flush, capability attach/detach, watcher·PTY·LSP·remote lifecycle을 명시적 supervisor로 이전한다.
- 완료: core가 `AppHandle` 없이 compile되고 기존 Tauri 앱이 port 구현으로 동작한다.

### Phase 3 — 병렬 기술 spike

진입 조건: Phase 0의 전체 기능·데이터·성능 baseline과 M1~M7의 기능별 crate 이관·검증이 끝나야 합니다. 기술 비교용 spike는 이 gate 이후에 수행하며, 그 전에는 native UI를 구현하지 않습니다.

- UI: egui/eframe과 iced shell 비교, GPUI editor surface 제한 비교.
- Editor: rope transaction + native text/IME/render 한 view.
- LSP: Rust coordinator와 mock server로 initialize, document sync, crash replay.
- Terminal: alacritty_terminal과 wezterm-term headless digest 비교, native surface host.
- 완료: 모든 hard gate를 통과한 후보만 별도 결정 문서로 확정한다.

### Phase 4 — native shell vertical slice

- native executable에서 project/session, tree, layout, tabs/splits, file open, command palette, theme·locale, window lifecycle을 구현한다.
- 기존 Tauri 앱과 동일 `taide-core`와 persistence를 사용한다.
- 완료: shell parity와 데이터 양방향 호환, multi-window·AT·IME·DnD 실기 통과.

### Phase 5 — native editor와 LSP

- document/view store, transaction·undo, renderer, save/hot-exit, Git decoration, AI, snippets·Emmet, full LSP adapter를 이관한다.
- native editor를 opt-in으로 제공하고 Monaco fallback을 유지한다.
- 완료: editor·LSP matrix와 large-file·crash soak·성능 gate 통과.

### Phase 6 — native terminal

- `TerminalCore`, native renderer, input·IME, selection·search·links·OSC, snapshot+delta attach를 구현한다.
- xterm 표시를 session별 opt-in으로 교체하고 fallback build를 유지한다.
- 완료: terminal matrix, 장기 flood, multi-window attach, memory·throughput gate 통과.

### Phase 7 — 나머지 기능 이관

- Git, search, settings, theme editor, plugin·VSIX, snippets, tasks, preview, remote UI, agent UI, notification과 system usage를 native UI로 이관한다.
- remote HTTP/WS와 IDE/MCP는 UI에서 분리된 protocol adapter로 기존 wire contract를 유지한다.
- 완료: 전체 기능 inventory와 security fixture 통과.
- 각 native 화면은 기존 TS view inventory의 표시·상태·동작·키보드·멀티윈도·테마/로케일·접근성 항목과 일대일 대조합니다. 기능 누락 0과 유사한 시각적 구성의 실기 확인 없이 기존 view를 제거하지 않습니다.

### Phase 8 — beta와 cutover

- native 앱을 기본값으로 전환하고 이전 Tauri 릴리스를 rollback으로 유지한다.
- 실제 프로젝트 장기 session, crash/restart, sleep/wake, multi-window, large repository soak를 수행한다.
- 완료: parity·performance·security·packaging gate와 beta 회귀 기간 통과.

### Phase 9 — TypeScript·Tauri 제거

- React·TS source, Vite/Bun frontend build, Tauri command/event adapter, Specta TS generation, Monaco/xterm와 WebView 자산을 제거한다.
- Rust-native release를 서명·공증하고 이전 데이터 migration·rollback 문서를 확정한다.
- 완료: 금지 참조 0건, Rust workspace 검증과 native E2E·실기 QA 통과.

## 8. 작업 의존성과 병렬화

| 선행 | 병렬 가능 작업 | 합류 조건 |
| --- | --- | --- |
| Phase 0 | editor, terminal, UI framework 조사 | baseline·fixture 이름과 parity 포맷 공유 |
| Phase 1 | protocol manifest, persistence fixture | facade와 model 타입 확정 |
| Phase 2 | UI shell, LSP coordinator, terminal core spike | runtime event·task·window port 확정 |
| Phase 3 | editor/LSP, terminal, shell 구현 | hard gate를 통과한 dependency 결정 |
| Phase 4 | editor/LSP와 terminal vertical slice | native window·render·input host 안정화 |
| Phase 5·6 | Git/search/preview UI 일부 | document·terminal action/event interface 안정화 |
| Phase 7 | 전체 parity·security·packaging | 기능 inventory 전건 구현 |

같은 file·crate 경계를 바꾸는 작업은 직렬로 수행하고, 독립 spike와 fixture 작성은 병렬화한다. 각 phase는 별도 contract와 PROCESS checklist를 만든 뒤 구현한다.

## 9. 기술 후보의 공식 근거

- UI: [egui](https://github.com/emilk/egui), [iced](https://github.com/iced-rs/iced), [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui), [Slint](https://github.com/slint-ui/slint), [AccessKit](https://github.com/AccessKit/accesskit)
- Editor: [Ropey](https://github.com/cessen/ropey), [Tree-sitter](https://tree-sitter.github.io/tree-sitter/), [COSMIC Text](https://github.com/pop-os/cosmic-text), [Helix architecture](https://github.com/helix-editor/helix/blob/master/docs/architecture.md)
- LSP: [Language Server Protocol 3.18](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/lsp/3.18/specification.md)
- Terminal: [alacritty_terminal](https://github.com/alacritty/alacritty/tree/master/alacritty_terminal), [wezterm-term](https://github.com/wezterm/wezterm/tree/main/term), [vte](https://github.com/alacritty/vte), [wgpu](https://github.com/gfx-rs/wgpu)
- Preview: [pdfium-render](https://docs.rs/pdfium-render/), [Calamine](https://github.com/tafia/calamine)

후보의 최신 release, MSRV, license와 API 안정성은 각 spike 착수 시 다시 확인한다. 이 문서는 dependency 확정 기록이 아니다.

## 10. 삭제 게이트

다음 순서를 뒤집지 않는다.

1. native implementation과 자동·실기 증거를 만든다.
2. 기존 Tauri 실행 경로와 shadow 또는 opt-in 비교를 수행한다.
3. native를 기본값으로 전환하고 fallback을 한 beta/release 동안 유지한다.
4. rollback과 데이터 호환을 재검증한다.
5. 대응 TS·Tauri·Monaco·xterm code와 dependency를 삭제한다.

중간 phase에서 성능이 기대보다 낮거나 framework hard gate가 실패하면 제품 코어를 버리지 않고 해당 candidate만 폐기한다.
