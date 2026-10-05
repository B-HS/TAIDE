# Rust-native 전환 감사 — 구조·품질·잘못된 수정 (architecture)

> 감사일: 2026-10-06 / 브랜치 `to_rust_native` / 읽기 전용 감사 (빌드·테스트 미실행)
> 판정 근거는 전부 실제 파일입니다. `docs/HANDOFF.md`·`docs/PROCESS.md`의 자체 진척 수치는 사용하지 않았습니다.

## 1. 범위

| 구분 | 읽은 경로 |
| --- | --- |
| 추적 변경분 | `git diff --stat` 44개 파일, `git diff -- crates src-tauri Cargo.toml` 전문 |
| 미추적 crate | `crates/taide-remote-wire`, `crates/taide-remote/src/command-policy.rs`, `crates/taide-runtime/src/{agent_host,native_file_actions,native_lsp_actions,sync_gist_http,terminal_env}.rs`, `crates/taide-lsp/src/native*`(파일 구성·줄 수), `crates/taide-model/src/identifier.rs` |
| native | `native/*/Cargo.toml` 6개, `native/taide-native-app/src/{main,lib,application,host,application-ports,bootstrap,remote-dispatch,remote-gateway,remote-assets}.rs`, `native/taide-native-ui/src/{lib,commands,shell}.rs`, `native/taide-native-editor/src/{lib,syntax,view}.rs`, `native/taide-remote-web/src/lib.rs`, 각 crate 의 파일 목록·줄 수 |
| vendor | `native/taide-native-app/vendor/{egui-input,wry-preview,rhwp,cfb-retained,zip-retained}`, `native/taide-native-terminal/vendor/alacritty-terminal`, `experiments/native-shell-spike/vendor/eframe`, `experiments/terminal-core-spike/vendor/vte` 의 UPSTREAM.md·TAIDE-CHANGES.md·Cargo.toml·LICENSE 목록 |
| experiments·tools | `experiments/*/Cargo.toml` 7개, `experiments/native-shell-spike/{package-spike.sh,bundle/egui-Info.plist,.gitignore}`, `tools/migration-metrics`, `tools/keybinding-catalog/export.ts`, `tools/m8-*.ts` 머리 |
| 기준 문서 | `docs/roadmap-rust-native.md`, `docs/acknowledge/2026-09-23-rust-native-transition-contract.md`, `docs/acknowledge/2026-09-30-m8-code-first-parity.md`, `docs/quality-assurance/2026-09-30-m8-native-shell-spike.md`(머리), `docs/bug/2026-10-05-native-application-owner-not-wired.md` |
| 대조용 Tauri | `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`(plugin·lifecycle 줄), `src-tauri/src/remote_gateway.rs`, `.github/workflows/*.yml` |

규모(줄 수는 `wc -l` 실측):

| crate | src 줄 수 | 그중 src 안 테스트 | 통합 테스트(`tests/`) |
| --- | --- | --- | --- |
| taide-native-app | 83,415 | 29,616 (61개 `*-tests.rs`) | 61개 파일 |
| taide-native-ui | 24,091 | 516 | 13개 파일 |
| taide-remote-web | 7,178 | 0 | 17개 파일 + `tests/browser-probe` 별도 crate |
| taide-native-editor | 4,825 | 0 | 12개 파일 |
| taide-native-terminal | 2,384 | 0 | 9개 파일 |
| taide-native-retained(+derive) | 553 | 0 | 1개 파일 |
| 합계 | 122,446 | 30,132 | 44,171줄 |

제품 코드 약 92,300줄, 테스트 약 74,300줄(비율 약 0.8 : 1)입니다.

## 2. 총평

1. 추적 파일 44개 변경은 대부분 "Tauri 전용 코드를 공유 crate 로 옮긴 리팩터링"이며, 읽은 범위에서 IPC 계약·원격 허용표를 깨는 내용 변경은 발견하지 못했습니다(컴파일은 미검증).
2. native 구현은 실제 실행 경로(main.rs → `NativeApplication::new` → `application_ports.start`)에 연결돼 있지만, 제품으로 실행·배포하기 위한 기반(workspace·CI·패키징·기본 데이터 경로·로깅·메뉴·다중 창·구문 강조 엔진)이 비어 있습니다.
3. 구조상 가장 큰 문제는 세 가지입니다. (a) native crate 6개가 각각 독립 workspace·독립 `Cargo.lock`·중복 `[patch]` 이고 제품 crate 가 `experiments/` 경로에 의존합니다. (b) 실행 파일 crate(`taide-native-app`)에 원격 HTTP/WS 서버·IDE 서버·미리보기 파서·LSP 브리지가 전부 들어가 로드맵의 crate 경계를 벗어납니다. (c) `application.rs` 5,534줄·`terminal_surface.rs` 8,294줄의 단일 소유 객체에 상태가 집중됩니다.
4. egui 는 `docs/acknowledge/2026-09-30-m8-code-first-parity.md` 상 "임시 사용"이며 최종 채택·root MSRV 변경 승인이 아닙니다. hard gate 6종 중 코드로 닫힌 것은 없습니다(§6).

## 3. 영역별 판정

### 3.1 추적 파일 44개 변경 (지시 1)

| 변경 | 판정 | 근거 |
| --- | --- | --- |
| 원격 허용·거부 표를 `src-tauri/src/remote_gateway.rs` → `crates/taide-remote/src/command-policy.rs` 로 이동 | 정상 | 허용 항목 수 HEAD 380줄(= IMPLEMENTED 203 + ALLOWED 177) 과 현재 203 + 177 이 일치. `remote_gateway.rs:3` 이 새 모듈을 import, 분할 완전성 테스트(`remote_gateway.rs:1342~1390`) 유지 |
| wire 프레임 함수·`RemoteRequest`·상수를 `crates/taide-remote-wire` 로 이동 | 정상 | `crates/taide-remote-wire/src/protocol.rs:76~103` 이 삭제된 `crates/taide-remote/src/protocol.rs` 본문과 동일 형식. `RemoteRequest` 필드 동일(Serialize 추가) |
| LSP process 에 bounded/owned transport 추가 | 정상 | `crates/taide-infra/src/lsp_proc.rs` 의 기존 `spawn` 은 `IncomingBuffer::Legacy` + `OutgoingTransport::Direct` 로 위임, `frame_limits: None` 이라 기존 분기 동작 유지 |
| mirror 파일에 `write_id` 추가, 조건부 삭제 API 추가 | 정상(하위 호환) | `crates/taide-file/src/service.rs` `MirrorFile.write_id` 는 `#[serde(default, skip_serializing_if)]`. 기존 `mirror_dirty` 는 새 함수의 결과에서 `disk_modified_ms` 만 반환 |
| agent 프로세스 탐지·CLI 상태·gist HTTP 를 `taide-runtime`·`taide-sync` 로 이동 | 정상 | `src-tauri/src/domain/agent/commands.rs:28~35`, `src-tauri/src/domain/sync/github.rs`(1줄 re-export), `crates/taide-runtime/src/{agent_host,sync_gist_http}.rs` |
| `AppError`·`LocalizedError`·`MirrorEntry` 에 `Deserialize`/`Clone` 추가, `ThemeEditorContext`·`MirrorWriteId` 추가 | 정상 | `crates/taide-model/src/{error,file,theme,ids}.rs`. 직렬화 형식 불변 |
| `taide-model` 이 `taide-remote-wire` 에 의존 | 보강 필요 | `crates/taide-model/Cargo.toml` 추가 줄. 로드맵 §2 는 `taide-model` 을 최하위로 규정. wire crate 에는 브라우저 client 상태기계(`client.rs` 255줄)까지 포함 |
| 거부 정책의 보안 근거 doc 주석 약 150줄 삭제 | 보강 필요(낮음) | diff 의 `RemoteDenialPolicy` 각 variant 설명이 `command-policy.rs:4~16` 에서 사라짐. `remote_gateway.rs:957` 등은 삭제된 doc 을 여전히 참조. 근거를 `docs/` 로 옮기지 않았음 |
| 공유 crate 에 host 이름을 딴 모듈 추가 | 보강 필요(낮음) | `crates/taide-runtime/src/{native_file_actions,native_lsp_actions}.rs`, `crates/taide-lsp/src/native/`(약 4,800줄). Tauri 빌드에도 `lsp-types`·`regex` 가 새로 링크됨(`crates/taide-lsp/Cargo.toml`) |
| Tauri 앱 컴파일·테스트 통과 여부 | 미확인 | 빌드 금지 조건. import 정리는 육안으로 맞아 보이나 `cargo clippy --workspace -- -D warnings`(`.github/workflows/ci.yml:73`) 재실행 필요 |

### 3.2 workspace·lock·patch 구조 (지시 2)

- 독립 workspace 가 최소 13개입니다. native 6개(`native/*/Cargo.toml` 모두 `[workspace]` 선언) + `native/taide-remote-web/tests/browser-probe` + experiments 5개 + `tools/migration-metrics`. `Cargo.lock` 도 각자 보유합니다(패키지 수: app 821, ui 371, remote-web 370, terminal 273, editor 48, root 649).
- 같은 `[patch.crates-io] egui = vendor/egui-input` 이 4개 manifest 에 복제돼 있습니다(`native/taide-native-app/Cargo.toml`, `native/taide-native-ui/Cargo.toml`, `native/taide-remote-web/Cargo.toml`, `tests/browser-probe/Cargo.toml`). `vte` patch 도 app·terminal 에 중복입니다. 한 곳만 빠지면 같은 소스가 다른 egui 로 빌드됩니다.
- 제품 crate 가 `experiments/` 에 의존합니다. `native/taide-native-app/Cargo.toml` 의 `eframe = { path = "../../experiments/native-shell-spike/vendor/eframe" }`, `vte = { path = "../../experiments/terminal-core-spike/vendor/vte" }`, `[[example]] path = "../../experiments/lsp-coordinator-spike/src/bin/mock-server.rs"`, `[[bin]] native-terminal-queue-fixture path = "../taide-native-terminal/tests/fixtures/session.rs"`(테스트 fixture 가 제품 패키지의 bin). `experiments/` 를 정리하면 제품이 빌드되지 않습니다.
- MSRV·edition 분리: native 는 `rust-version = "1.95"`, edition 2024, root 는 1.89, edition 2021. egui 0.36.2 자체가 1.95 를 요구합니다(`vendor/egui-input/Cargo.toml:14`). parity 합의서는 root MSRV 변경을 승인하지 않았습니다.
- 버전 불일치: root lock 과 app lock 에서 tokio·serde·axum·hyper·reqwest·git2·notify·portable-pty·sysinfo·uuid·regex·lsp-types·rustls 등 22개를 대조했고 전부 동일했습니다(정상). 다만 강제 수단이 없어 한쪽만 `cargo update` 하면 공유 `crates/taide-*` 가 서로 다른 의존 버전으로 두 번 빌드됩니다.
- app lock 내부 중복: `zip` 3개(2.4.2, 8.6.0 registry, 8.6.0 local), `cfb` 2개, `resvg`·`usvg`·`tiny-skia`·`fontdb`·`skrifa`·`read-fonts` 각 2개, 사용하지 않는 `svg2pdf`·`pdf-writer`(rhwp 경유), `windows-sys` 5개.
- CI 미포함: `.github/workflows/{ci,release,cache-warm}.yml` 에 native·experiments 단계가 없습니다. `cargo fmt --all --check`·clippy 는 root workspace 만 대상입니다.
- 219GB 원인이 되는 구성: (a) 모든 native crate 를 `--target-dir experiments/native-shell-spike/target` 하나로 공유(`docs/HANDOFF.md:465,497`), (b) 어떤 native manifest 에도 `[profile]` 이 없어 dev 기본(전체 debuginfo·incremental), (c) feature·patch 조합이 다른 workspace 6개가 같은 target 에 서로 다른 fingerprint 를 누적, (d) eframe·wgpu·wry 를 정적 링크하는 통합 테스트 바이너리가 app 만 61개.

### 3.3 vendored fork (지시 3)

| fork | 위치 | 바꾼 것 | 유지 위험 | 라이선스 파일 |
| --- | --- | --- | --- | --- |
| egui 0.36.2 (`egui-input`) | `native/taide-native-app/vendor/egui-input` | 116개 중 10개 파일: `context.rs`, `pass_state.rs`, `ui.rs`, `hit_test.rs`, `memory/mod.rs`, `containers/{scroll_area,modal,menu}.rs`, `widgets/button.rs`, `Cargo.toml`. Button Space keyup 활성화, focus·pointer owner 조회 API, submenu open override 등 DOM 동작 재현용 | 높음. focus·hit-test 핵심부라 egui 업그레이드마다 재이식 필요 | LICENSE-MIT |
| eframe 0.36.2 | `experiments/native-shell-spike/vendor/eframe` | `wgpu_integration.rs` 보조 창 AccessKit 초기화, `raw_input_hook_with_replay` 추가, `preserve_empty_paste_shortcuts` feature | 중간. 실험 디렉터리에 있으면서 제품이 사용 | LICENSE-MIT |
| wry 0.55.1 (`wry-preview`) | `native/taide-native-app/vendor/wry-preview` | feature `native-preview-deny-permissions`: 파일 업로드 패널·카메라·마이크 요청 거부 | 낮음. 국소 | MIT·Apache |
| rhwp 0.8.2 | `native/taide-native-app/vendor/rhwp` | 10건: 환경변수 실험 블록 제거, 표 grid overflow 방지, ViewText AES 시작 offset 수정, HWP3 범위·예산 검사, 16비트 wrapping, retained derive, lazy resolver adapter | 중간~높음. 파서 수정이 많음 | LICENSE(MIT), THIRD_PARTY_LICENSES.md |
| cfb 0.14.0 / zip 8.6.0 (`*-retained`) | `native/taide-native-app/vendor/` | retained 크기 계산용 조건부 derive 만 추가 | 낮음. 단 registry 판과 중복 링크 | LICENSE(MIT) |
| alacritty_terminal 0.26.0 | `native/taide-native-terminal/vendor/alacritty-terminal` | `native-retained` feature 아래 다수 동작 변경: 로그 redaction, keyboard stack 오류 수정, mouse protocol(X10·SGR pixel), DECSET 1004, CSI 2J, DL/IL, OSC133 row anchor, selection anchor 등 | 높음. xterm 6.0.0 동작 재현을 fork 안에서 수행 | LICENSE-APACHE (Apache-2.0 단독) |
| vte 0.15.0 | `experiments/terminal-core-spike/vendor/vte` | OSC raw buffer 상한 4096, `OscObserver`/`StreamObserver`, `command_marker` | 중간. 실험 디렉터리에 있으면서 제품이 사용 | MIT + Apache |

- 모든 fork 에 UPSTREAM.md 또는 TAIDE-CHANGES.md 와 원본 라이선스 파일이 있습니다(정상).
- 배포용 고지는 없습니다. 루트 `THIRD_PARTY_LICENSES.md` 의 "Rust crates" 절에 egui·eframe·alacritty·wry·ropey·calamine·resvg 가 없습니다(검색 결과 일치 줄은 npm 쪽 xterm·monaco·`@rhwp/core` 등). 이식 코드 라이선스(`native/taide-native-app/LICENSE-{LUCIDE,SONNER,FLOATING-UI,RADIX-TOOLTIP,XTERM-LINKS}`, `native/taide-native-editor/LICENSE-MONACO-SNIPPET`, `native/taide-native-ui/license-command-score.txt`)도 crate 별로 흩어져 있고 번들에 포함하는 경로가 없습니다.
- 변경 내용은 각 UPSTREAM 문서 기준이며 원본과의 바이트 비교는 수행하지 않았습니다.

### 3.4 "UI까지 Rust-native" 목표와 어긋나는 요소 (지시 4)

| 요소 | 판정 | 근거 |
| --- | --- | --- |
| WebView(wry) 의존 | 사용자 결정 필요 | macOS 전용 의존(`Cargo.toml` `[target.'cfg(target_os = "macos")'.dependencies] wry`). HTML·오디오·비디오 미리보기에만 사용(`src/preview_web_view.rs:127~149`: JS 비활성, incognito, 새 창·다운로드 거부). 로드맵 §6.2 는 HTML 을 "권한 없는 별도 WebView/helper" 로 허용하므로 로드맵 위반은 아니나, "TypeScript 제거 후에도 WebView 가 바이너리에 남는다"는 점은 목표 문구와 다릅니다. 비 macOS 는 경로 없음(`preview_web_view.rs:219`) |
| `taide-remote-web`(브라우저 Wasm) | 범위 확장 + 미연결 | 로드맵에 없는 두 번째 프런트엔드 타깃. TS 제거 후 원격 브라우저 UI 를 유지하려면 필요하지만, 데스크톱 동등성보다 먼저 7,178줄 + 테스트 17개가 투입됨. rlib 뿐이고 cdylib·wasm 진입점은 테스트용 `tests/browser-probe` 에만 존재. `remote-public/bundle-manifest.json`(`src/remote-assets.rs:14~16`) 을 만드는 스크립트가 저장소에 없음 |
| app 안의 remote-*.rs HTTP/WS 서버 | 경계 위반 | 로드맵 §2: `taide-remote` 가 "HTTP·WebSocket protocol 과 기본 거부 dispatch", `taide-ide` 가 "IDE/MCP protocol adapter", `taide-app` 은 "도메인 로직 재구현 금지". 실제로는 `native/taide-native-app/src/remote-*.rs` 20개 모듈(제품 약 3,500줄)과 `ide-server.rs`·`ide-tools.rs`·`agent-hooks.rs`·`event-relay.rs` 가 실행 파일 crate 에 있음 |
| 같은 로직의 이중 구현 | 결함 | 원격 게이트 전처리(훅 설치 scope 거부, `settings_update` patch strip, `app_file_write` settings strip)가 `src-tauri/src/remote_gateway.rs` 와 `native/taide-native-app/src/remote-gateway.rs:76~112` 양쪽에 따로 존재. 서버(`src-tauri/src/domain/remote/{server,ws,serving}.rs` 대 native `remote-{http,ws,serving}.rs`)와 IDE 서버(`domain/ide/server.rs` 대 `ide-server.rs`)도 이중 |
| 추출 가능성 | 정상 확인 | 위 24개 모듈에 `egui`·`eframe`·`taide_native_ui` import 가 없음(검색 0건). toolkit 과 무관하므로 crate 이동이 기계적으로 가능 |

### 3.5 egui hard gate 현재 상태 (지시 5)

| gate | 코드로 확인한 상태 | 근거 |
| --- | --- | --- |
| 다중 창 | unwired | `native/taide-native-ui/src/shell.rs:54~56,133` 에 `WindowScope::Auxiliary` 분기가 있으나 app 은 `WindowScope::Main` 만 생성(`application.rs:254`). `show_viewport_deferred`·`show_viewport_immediate` 검색 0건. `layout_move_tab_to_window` 는 native 에서 테스트 파일 1곳에만 등장 |
| CJK IME | 코드만 존재, 실기 미검증 | 편집기 `native/taide-native-ui/src/editor_surface.rs:503~560`(Preedit·Commit·DeleteSurrounding), 터미널 `native/taide-native-terminal/src/input.rs`, `terminal_surface.rs`. spike QA 는 "실제 CJK 조합은 사용자 실기 대기" 로 기록 |
| 접근성 | 부분 | `eframe` feature `accesskit` 활성. `widget_info`·accesskit 사용 파일은 shell·settings·toast·tooltip·terminal·preview surface 등. `editor_surface.rs` 에는 접근성 노드 등록이 없음(검색 0건). VoiceOver 실기 미수행 |
| native 메뉴 | missing | native 전체에서 `muda`·`NSMenu`·`set_menu`·`menu_bar` 검색 0건. spike 는 `muda` 를 썼으나(`experiments/native-shell-spike/Cargo.toml`) 제품 의존성에 없음. Tauri 는 `src-tauri/src/domain/window/menu.rs` 330줄 |
| 외부 파일 drop | missing | `dropped_files`·`hovered_files`·`DroppedFile`·`HoveredFile` 검색 0건. TS 는 `src/widgets/app-shell/app-shell.tsx` 에서 `onDragDropEvent` 사용 |
| GPU fallback | missing | `native/taide-native-app/src/main.rs:32~35` 가 `NativeOptions { viewport, ..Default::default() }` 만 지정. eframe feature 는 `wgpu` 뿐(`glow` 없음). `wgpu_options`·`power_preference`·`force_fallback` 검색 0건. spike QA 도 device-loss 복구·software fallback 을 미구현으로 기록 |

### 3.6 코드 품질 (지시 6)

- 5,000줄 이상 단일 파일: `src/terminal_surface.rs` 8,294줄, `src/application.rs` 5,534줄, `tests/terminal-host.rs` 6,099줄.
- `application.rs` 의 `NativeApplication` 은 필드 92개(102~194행)를 가진 단일 소유 객체입니다. `poll` 461~1359행(약 900줄), `poll_lsp` 2651~3160행(약 510줄), `ui` 3618~4376행(약 760줄). `new` 가 UI 스레드에서 `runtime.block_on` 을 3회 호출합니다(211, 238, 453행).
- 오류 표면: 오류를 `self.status: Option<String>` 한 칸에 덮어쓰는 대입이 `application.rs` 에만 102곳입니다. 로거 초기화(`set_logger` 등)는 native 전체에서 검색 0건이라 9개 파일의 `log::` 호출이 전부 버려집니다. Tauri 는 `tauri_plugin_log`(`src-tauri/src/lib.rs:839`)를 사용합니다.
- `HostBridge`: 명령을 worker 1개가 순차 `await` 합니다(`src/host.rs:452~475`). 용량 64 큐에 `try_send` 하고 가득 차면 `"native host command queue is full"` 오류로 명령을 버립니다(`host.rs:485~500`). HWP·PDF·스프레드시트 미리보기 읽기(`host.rs:964~1025`)가 진행되는 동안 저장·트리 토글·mirror 기록이 뒤에서 대기합니다.
- kebab-case 와 `#[path]`: lib.rs 의 `#[path]` 는 app 55개, ui 27개, remote-web 14개, editor 9개입니다. 같은 crate 안에 `terminal_surface.rs` 와 `remote-http.rs` 가 섞여 있고 규칙이 없습니다. 공유 crate 에도 번졌습니다(`crates/taide-remote/src/lib.rs` 의 `#[path = "command-policy.rs"]`).
- 1줄짜리 재수출 파일 9개: `src/{theme-editor,theme-live-preview,settings-controls,theme-draft,toast,css-motion,keybinding-icons,theme-editor-tokens,theme-color-picker}.rs` 가 `pub use taide_native_ui::…` 한 줄입니다. 그중 5개는 `#[cfg(test)]` 로만 포함됩니다(`lib.rs`).
- crate 간 `include!` 우회: `src/keymap.rs:1~21`, `src/keybinding-editor.rs:1~19`, `src/keybinding-search.rs:1~5`, `src/tooltips.rs:14~53` 이 `#[cfg(not(test))]` 에서는 `taide_native_ui` 를 재수출하고 `#[cfg(test)]` 에서는 `include!("../../taide-native-ui/src/….rs")` 로 UI crate 소스를 app crate 안에 다시 컴파일한 뒤 테스트를 붙입니다. 반대로 `native/taide-native-ui/src/tooltips.rs:1383` 은 app 의 테스트 파일을 include 합니다. 테스트가 검증하는 타입은 출시 바이너리의 타입과 다른 복사본입니다.
- 역방향 리소스 경로: `native/taide-native-ui/src/icons.rs:78~98`, `toast.rs:228~230` 이 `../../taide-native-app/resources/…` 를, `keybinding-search.rs:207`·`keybinding-catalog.rs:487` 이 `../../taide-native-app/tests/fixtures/…` 를 `include_*!` 합니다. 하위 crate 가 상위 crate 디렉터리에 의존합니다.
- stub·no-op: `todo!`·`unimplemented!` 는 0건입니다. `host.rs:911,1489` 의 `None => Box::pin(async {})` 는 reconcile 포트가 없을 때의 무동작 분기입니다. `application.rs:4611` `fn branch(..) -> Option<&str> { None }` 는 Git 브랜치 표시를 항상 비웁니다. `application.rs:4916~4922` 는 File·Terminal·Settings·AppFile·Untitled 이외 탭에 `"native tab surface is not connected"` 를 표시합니다.
- unsafe: native 제품 코드 약 35곳, 전부 macOS FFI(`preview_macos.rs`, `preview_pdf_macos.rs`, `motion-preference.rs`, `ui-fonts.rs`)입니다. 다른 native crate 와 새 `crates/` 코드는 0건입니다(정상).
- 플랫폼: `src/bootstrap.rs:150~185` 의 열기·Finder 표시는 `/usr/bin/open`, 알림은 `/usr/bin/osascript` 실행이며 비 macOS 는 `"native … platform is not connected"` 오류를 반환합니다.

### 3.7 목표 crate 경계 대비 (지시 7)

| 로드맵 crate | 현재 대응 | 차이 | 권장 정리 |
| --- | --- | --- | --- |
| `taide-app` (조립·실행 파일) | `native/taide-native-app` 83,415줄 | 조립 외에 원격·IDE 서버, 미리보기 파서, LSP 브리지, 터미널 surface, 탐색기까지 포함. 기존 `crates/taide-app`(앱 파일 서비스 281줄)과 이름 충돌 | 조립 전용으로 축소. 실행 파일 이름은 기존 crate 와 겹치지 않게 결정 |
| `taide-ui` | `native/taide-native-ui` 24,091줄 + app 안의 surface 다수 | `native-host` feature 로 `taide-runtime`·`taide-project`·`taide-infra`·`taide-file` 에 직접 의존(`src/commands.rs` 가 runtime action 직접 호출). 탐색기·문제 패널·상태바·터미널 surface 는 app 에 있음 | UI 는 intent 만 내보내고 host 가 실행. app 의 surface 를 UI crate 로 이동 |
| `taide-editor` | `native/taide-native-editor` 4,825줄 + `taide-native-ui/src/editor_surface.rs` 724줄 | 레이아웃·데코레이션·구문 토큰화가 없음. `src/syntax.rs` 40줄은 타입 정의뿐 | 구문 엔진·decoration layer 를 여기에 추가 |
| `taide-terminal` | `native/taide-native-terminal` 2,384줄 + 기존 `crates/taide-terminal`(PTY 세션 store) | 이름 충돌. 렌더·입력 surface 8,294줄은 app 에 있음 | core 는 crate 로, surface 는 UI 로 분리 |
| `taide-platform` | 없음 | 메뉴·dialog·알림·클립보드·drop·외부 열기가 `bootstrap.rs`·`host.rs`·`application.rs` 에 흩어짐 | 신설. 창 상태·단일 인스턴스·PATH 보정·로깅 포함 |
| `taide-remote` / `taide-ide` | 정책·store 만 crate, 서버는 app 과 src-tauri 에 이중 | §3.4 | 서버를 crate 로 올리고 양쪽이 공유 |
| (로드맵에 없음) | `taide-native-retained`, `taide-remote-web`, `taide-remote-wire` | 신규 | retained 는 infra 성격, remote-web 은 사용자 결정 뒤 위치 확정 |

### 3.8 패키징·서명·번들 (지시 8)

- native 제품용 패키징은 없습니다. `native/` 에 Info.plist·아이콘·entitlements·번들 스크립트·build.rs 가 없습니다(검색 결과 0건).
- 존재하는 것은 실험용 `experiments/native-shell-spike/package-spike.sh` 하나입니다. debug 바이너리를 `target/<이름>.app` 으로 복사하고 `codesign --force --sign -`(ad-hoc) 후 `--verify --strict` 만 수행합니다. 대상은 spike 실행 파일이며 `taide-native-app` 이 아닙니다. Developer ID·공증·dmg 는 없습니다.
- 실행 조건: `src/bootstrap.rs:24~52` 가 `--data-dir <절대 경로>` 를 필수로 요구합니다. 인자 없이 실행하면 종료합니다. keychain service 는 `net.gumyo.taide.native-isolated`(`bootstrap.rs:15`)로, Tauri 의 `net.gumyo.taide`(`tauri.conf.json` identifier, `src-tauri/src/lib.rs:929`)와 다릅니다. 기존 사용자 데이터·자격 증명으로 실행하는 경로가 없어 계약 §3.1·§4.3 을 검증할 수 없습니다.
- 창 설정: `main.rs:6~7,32~35` 는 크기 1280×800 과 제목 `"TAIDE Native"` 뿐입니다. Tauri 의 최소 크기 720×480·overlay titlebar·아이콘(`tauri.conf.json`)·창 상태 복원(`lib.rs:857`)·단일 인스턴스(`lib.rs:821`)·`fix_path_env`(`lib.rs:798`)에 대응하는 코드가 없습니다. LSP 는 `std::env::var_os("PATH")`(`application.rs:319,3521`, `remote-lsp.rs:50`)를 그대로 쓰므로 Finder 에서 실행하면 사용자 셸 PATH 가 빠집니다.
- 원격 번들: `Catalog::packaged`(`src/remote-assets.rs:72~100`)는 `Contents/Resources/remote-public` 또는 실행 파일 옆 `remote-public` 을 기대하지만 생성 경로가 없습니다. `docs/bug/2026-10-05-native-application-owner-not-wired.md` 도 "실제 배포물의 원격 시작은 유효한 자산을 갖출 때까지 실패"로 기록합니다.
- CI: native 빌드·테스트·릴리스 단계가 없습니다(§3.2).

## 4. 문제 목록

### 4.1 잘못 구현됐거나 보강이 필요한 코드 (partial 25건)

| # | 문제 | 근거 | effort |
| --- | --- | --- | --- |
| P1 | 독립 workspace·lock 13개, CI 미포함 | §3.2 | M |
| P2 | 제품 crate 의 `experiments/` 경로 의존, 테스트 fixture 가 제품 bin | `native/taide-native-app/Cargo.toml` | S |
| P3 | `[patch.crates-io]` 4중 복제 | §3.2 | S (P1 과 함께) |
| P4 | MSRV 1.95·edition 2024 분리, egui 채택 미승인 | parity 합의서, `vendor/egui-input/Cargo.toml:14` | 결정 사항 |
| P5 | 공유 target-dir + profile 부재 | `docs/HANDOFF.md:465,497` | S |
| P6 | `NativeApplication` 단일 객체 92필드, 3개 함수 500~900줄 | `application.rs:102~194,461,2651,3618` | L |
| P7 | `terminal_surface.rs` 8,294줄 단일 파일 | `wc -l` | L |
| P8 | crate 간 `include!` 로 테스트용 소스 복제 | `src/{keymap,keybinding-editor,keybinding-search,tooltips}.rs`, `native-ui/src/tooltips.rs:1383` | M |
| P9 | UI crate 가 app 의 resources·tests/fixtures 를 경로로 포함 | `native-ui/src/{icons,toast,keybinding-search,keybinding-catalog}.rs` | S |
| P10 | kebab·snake 혼용, `#[path]` 105개, 1줄 재수출 9개 | §3.6 | M |
| P11 | src 안 `*-tests.rs` 62개·30,132줄 | §1 | M |
| P12 | 원격·IDE·hooks 서버가 실행 파일 crate 에 위치 | §3.4 | L |
| P13 | 원격 게이트·서버·IDE 서버의 Tauri/native 이중 구현 | `src-tauri/src/remote_gateway.rs`, `native/.../remote-gateway.rs:76~112` | L (P12 와 함께) |
| P14 | 미리보기 파서(BIFF·XLS·XLML·CSV·HWP preflight·PPTX)가 실행 파일 crate 에 위치 | `src/preview_*.rs` 약 40개 | M |
| P15 | `HostBridge` 직렬 worker + 가득 차면 명령 폐기 | `host.rs:452~500` | M |
| P16 | 오류가 단일 status 문자열로만 표시 | `application.rs` 102곳 | M |
| P17 | 생성자·종료에서 UI 스레드 `block_on` | `application.rs:211,238,453,4391` | S |
| P18 | PlatformServices 가 macOS 셸 명령 실행, 알림은 osascript | `bootstrap.rs:150~185` | M |
| P19 | `--data-dir` 필수, 격리 keychain | `bootstrap.rs:15,24~52` | M |
| P20 | vendored fork 8종 유지 부담, 배포용 라이선스 고지 부재 | §3.3 | M |
| P21 | app lock 의 중복 의존(zip 3, cfb 2, resvg 2 등) | §3.2 | S |
| P22 | `taide-model` → `taide-remote-wire` 역의존, host 이름 모듈 | §3.1 | S |
| P23 | 삭제된 보안 근거 주석과 낡은 doc 참조 | §3.1 | S |
| P24 | `tools/` 의 TS 의존(bun·playwright·zod·`/private/tmp` 고정 경로), 키 바인딩 JSON 이 TS 소스에서 생성 | `tools/keybinding-catalog/export.ts:13~25`, `tools/m8-remote-rust-browser-probe.ts:26~28` | S |
| P25 | UI crate 의 runtime 직접 호출(`native-host`) | `native-ui/Cargo.toml`, `src/commands.rs:67~136` | M |

### 4.2 누락된 기반 (missing 9건)

| # | 누락 | 검색어(0건 확인) | effort |
| --- | --- | --- | --- |
| M1 | native 메뉴 바·최근 프로젝트 메뉴 | `muda`, `NSMenu`, `set_menu`, `menu_bar`, `MenuBar` | M |
| M2 | 외부 파일 drop | `dropped_files`, `hovered_files`, `DroppedFile`, `HoveredFile` | S |
| M3 | GPU fallback·device loss 처리 | `wgpu_options`, `WgpuConfiguration`, `power_preference`, `force_fallback`, `glow` feature | M |
| M4 | 제품 패키징·서명·공증·번들 리소스 | `Info.plist`, `build.rs`, `*.sh`, `codesign`(native 내) | L |
| M5 | 로깅 초기화·로그 파일 | `set_logger`, `set_boxed_logger`, `env_logger`, `simplelog` | S |
| M6 | 단일 인스턴스·창 상태 복원·PATH 보정 | `single_instance`, `outer_position`·`with_position`·`persist_window`, `fix_path`·`login_shell`·`"-ilc"` | M |
| M7 | 구문 강조 엔진과 plugin grammar 소비 | `tree_sitter`·`tree-sitter`, `syntect`, `textmate`·`TextMate`, `cosmic_text`, `onig` (native·crates 전체) | XL |
| M8 | Diff·ClaudeDiff·SearchEditor 탭 표면 | `TabKind::Diff`, `ClaudeDiff`, `SearchEditor` — `src/tabs.rs` 와 테스트 1곳 외 0건, `application.rs:4916~4922` 가 미연결 메시지 출력 | XL |
| M9 | 명령 팔레트·빠른 열기 | `command_palette`, `CommandPalette`, `quick_open`, `QuickOpen` | L |

M7~M9 는 편집기·검색·Git 영역 감사의 범위이며 여기서는 기반 부재만 기록합니다.

### 4.3 실제 앱 연결이 끊긴 지점 (unwired 3건 + 관련)

| # | 끊긴 지점 | 있는 것 | 끊긴 곳 |
| --- | --- | --- | --- |
| U1 | 보조 창 | `taide-native-ui/src/shell.rs:54~56,133` 의 `WindowScope::Auxiliary` 렌더 분기, `explorer.rs:88`·`lsp.rs:87`·`app-file-views.rs:324` 의 scope 분기 | `application.rs:254` 가 Main 만 생성. viewport 생성 호출 없음 |
| U2 | 원격 브라우저 UI | `taide-remote-web` 7,178줄(`BrowserApplication`·`BrowserWorkbench`·`BrowserEditor`), native 서버 `remote-http.rs`·`remote-ws.rs`, 전체 허용 명령 dispatch(`remote-dispatch-tests.rs:399~401` 이 허용표와 일치 검증) | wasm 진입점·cdylib·bundle manifest 생성기가 없어 `remote-public` 이 만들어지지 않음. 실행 경로는 `tests/browser-probe` 뿐 |
| U3 | LSP 기능 요청 계층 | `crates/taide-lsp/src/native/{feature,session,registration,capabilities}.rs` 에 completion·hover·definition 등 요청 정의 | native app·ui 에서 해당 메서드 문자열을 쓰는 곳은 `lsp-diagnostics-tests.rs` 뿐. 완성·hover UI 소비자가 없음 |
| - | Git 브랜치 표시 | `ShellSurfaces::branch`(`shell.rs:62`) | `application.rs:4611` 이 항상 `None` |
| - | 테스트 전용 모듈 | `lib.rs` 의 `css_motion`, `theme_editor`, `theme_editor_tokens`, `theme_live_preview`, `ui_icons` | `#[cfg(test)]` 로만 포함. 제품 경로는 `taide_native_ui` 를 직접 사용 |

### 4.4 정상 확인 (done 12건)

1. 원격 허용·거부 표가 내용 변경 없이 crate 로 이동했고 Tauri 가 그대로 사용합니다.
2. wire 프레임 형식과 `RemoteRequest` 필드가 유지됩니다.
3. 기존 LSP `spawn` 경로의 동작이 유지됩니다.
4. mirror 파일 형식이 하위 호환입니다.
5. root lock 과 native app lock 의 주요 공유 의존 22개 버전이 일치합니다.
6. native 원격 dispatch 가 허용표 전체를 구현하고 공유 정책(`command_policy::admit`)을 통과합니다.
7. 원격·IDE·hooks·event-relay 모듈이 toolkit 과 독립입니다.
8. `todo!`·`unimplemented!` 가 없고 unsafe 는 macOS FFI 4개 파일에 한정됩니다.
9. 모든 vendored fork 에 출처 문서와 원본 라이선스 파일이 있습니다.
10. wry WebView 가 JS 비활성·incognito·권한 거부로 제한돼 있습니다.
11. 실제 실행 경로가 연결돼 있습니다: `main.rs:9~51` → `NativeApplication::new`(`application.rs:197`) → `bootstrap::connect_shell` → `HostBridge::connect_with_application_ports` → `LspBridge::connect` → `application_ports.start`(`application.rs:453~457`).
12. `.gitignore` 의 `target` 패턴이 experiments·tools 의 target 디렉터리를 제외합니다.

## 5. 권장 구현 순서

1. 기준선 고정 — root workspace 검증(`cargo clippy --workspace`·`cargo test --workspace`)으로 추적 44개 변경이 Tauri 앱을 깨지 않는지 한 번 확인한 뒤, 미커밋 상태를 논리 단위로 커밋합니다. 이후 모든 단계의 전제입니다.
2. 프레임워크 결정 — egui 채택과 MSRV 1.95 를 사용자와 확정합니다(P4). 다중 창·메뉴·drop·GPU fallback 을 egui 로 닫을 수 있는지가 3~7단계의 전제입니다.
3. workspace 통합 (P1·P2·P3·P5·P21) — `native/` 단일 workspace·단일 lock·단일 `[patch]`·전용 target·profile 을 만들고, eframe·vte fork 를 `native/vendor` 로 옮겨 `experiments/` 의존을 끊습니다. CI 에 native job 을 추가합니다. 2단계에 의존합니다.
4. 테스트 구조 정리 (P8·P9·P10·P11) — `include!` 복제를 제거하고 테스트·리소스를 소유 crate 로 옮깁니다. 3단계 뒤에 수행해야 lock·patch 변화로 다시 깨지지 않습니다.
5. crate 경계 정리 (P12·P13·P14·P22·P25) — 원격·IDE 서버를 `crates/` 로 추출해 Tauri 와 공유하고, 미리보기 파서를 별도 crate 로, 플랫폼 서비스를 `taide-platform` 으로 분리합니다. 3단계에 의존합니다.
6. 호스트 골격 보강 (P6·P7·P15·P16·P17·M5) — `NativeApplication` 을 기능별 컨트롤러로 분해하고, `HostBridge` 를 도메인별 비직렬 실행으로 바꾸고, 오류 채널과 로깅을 넣습니다. 5단계와 같은 파일을 건드리므로 직렬로 진행합니다.
7. 제품 기반 (M1·M2·M3·M6·U1·P18·P19) — 메뉴, 다중 창, 외부 drop, GPU fallback, 단일 인스턴스·창 상태·PATH, 기본 데이터 경로와 기존 데이터 호환. `taide-platform`(5단계)에 의존합니다.
8. 기능 표면 (M7·M8·M9·U3) — 구문 엔진, Diff·검색 편집기·팔레트, LSP UI. 다른 영역 감사의 순서를 따르되 6단계의 분해가 끝난 뒤 착수합니다.
9. 원격 브라우저 번들 (U2·P24) — wasm 진입점·빌드·bundle manifest 생성. 데스크톱 동등성(8단계) 뒤로 미루는 것을 권장합니다.
10. 패키징·고지 (M4·P20) — 번들 스크립트, 서명·공증, 라이선스 고지 통합. 7·9단계에 의존합니다.

## 6. 확인하지 못한 것

- 빌드·테스트를 실행하지 않았습니다. 추적 변경분과 native 의 컴파일·테스트 통과 여부는 판정하지 않았습니다.
- `application.rs` 는 구조와 일부 구간(44~460, 1359~1418, 3599~3708, 4600~4930)만, `terminal_surface.rs`·`lsp.rs`·`explorer.rs` 는 본문을 읽지 않았습니다.
- vendored fork 의 변경 범위는 각 UPSTREAM 문서 기준이며 원본과 비교하지 않았습니다.
- 219GB 의 원인은 구성에서 추론했고 target 내부는 조사하지 않았습니다.
- hard gate 의 실기 동작(IME 조합, VoiceOver, 실제 GPU 장애)은 코드 유무만 확인했습니다.
- `remote-public` 부재 시의 실제 시작 실패 동작은 실행이 아니라 `docs/bug/2026-10-05-native-application-owner-not-wired.md` 의 기록에 근거합니다.
- `TabKind::Welcome` 의 표시 경로(`shell.rs:197,763`)는 끝까지 추적하지 않았습니다.
- `docs/HANDOFF.md`·`docs/PROCESS.md` 는 target-dir 관련 줄만 검색했습니다.
