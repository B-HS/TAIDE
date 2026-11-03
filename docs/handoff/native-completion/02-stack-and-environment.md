# 02. 스택과 환경: 버전, 디렉터리, 빌드·검사·실행 명령, 알려진 실패

> 작성일: 2026-11-04(2026-11-03 판을 실제 파일로 다시 확인해 갱신) · 기준: `to_rust_native` `7be1ad14`(git lane 병합 `f3a5b5d0` 포함)
> 모든 버전은 `Cargo.toml`·`Cargo.lock`·`package.json`·rustup 설치 메타데이터·문서에서 확인한 값입니다. 이 문서를 쓰는 동안 cargo·rustc·bun은 실행하지 않았습니다. 실제 작업을 시작하기 전에 직접 `rustc --version`, `cargo --version`, `bun --version`으로 한 번 확인하십시오.
> 2026-11-03 판과 달라진 중요한 정정: 12.3절의 "실제 휴지통을 쓰는 검사" 이름 3개(이전 판의 `--skip` 목록 중 2개가 틀렸음), 3.1절의 egui 변경 파일 수, 2.1절의 툴체인 근거, 12.1절 `editor_fonts` 실패의 성격.

## 0. 요약

| 항목 | 값 | 근거 |
| --- | --- | --- |
| OS·기기 | macOS, Apple Silicon(arm64), zsh. CPU 18코어·메모리 128GB 기준으로 lane 수를 정했습니다 | `docs/HANDOFF.md` 7절 119줄, `docs/memory/2026-10-28-native-lane-operations.md` 2.2절 |
| Rust 툴체인 | 저장소에 `rust-toolchain`·`rust-toolchain.toml`·`.cargo/config*`이 없습니다. rustup 기본 툴체인 `stable-aarch64-apple-darwin`이 rustc 1.98.1(48a229cea 2026-09-01)입니다. `1.95.0-aarch64-apple-darwin` 툴체인도 설치돼 있습니다 | `ls -a` 결과, `/Users/hyunseokbyun/development/rust/rustup/settings.toml`의 `default_toolchain`, `.../toolchains/stable-aarch64-apple-darwin/lib/rustlib/multirust-channel-manifest.toml`의 `[pkg.rustc] version` |
| native crate 선언 | `edition = "2024"`, `rust-version = "1.95"` | `native/*/Cargo.toml` `[package]`(앱·UI·editor·syntax·terminal·retained는 4·5줄, remote-web은 5·6줄) |
| 공용 crate·Tauri 앱 선언 | `edition = "2021"`, `rust-version = "1.89"` | `crates/taide-git/Cargo.toml` 5·6줄, `src-tauri/Cargo.toml` 7·8줄 |
| UI 프레임워크 | egui/eframe 0.36.2(eframe는 vendored, egui는 `[patch.crates-io]`로 vendored) | `native/taide-native-app/Cargo.toml` 56·112줄 |
| 원본 화면 | React 19.2.8, Monaco 0.56.0, Shiki 4.4.3, xterm 6.0.0, Vite 8.2.0, TypeScript 5.9.3, Tauri 2 | `package.json`, `src-tauri/Cargo.toml` |
| JS 런타임 | Bun. `package.json`에 버전 고정(`packageManager`·`engines`)이 없습니다. CI는 `bun-version: 1.3.14`, 로컬 기록은 1.4.2(2026-10-06)입니다 | `.github/workflows/ci.yml` 29줄, `docs/quality-assurance/2026-10-06-native-batch5-engine-gate.md` 226줄 |
| 메인 트리 cargo target | `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target` | `docs/utils/2026-11-03-native-merge-gate.sh` 5줄 |
| lane cargo target | `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>` | `docs/PROCESS.md` 28줄, `ls /Users/hyunseokbyun/development/TAIDE-lanes/targets` |
| cargo 홈 | `/Users/hyunseokbyun/development/rust/cargo`(registry 소스: `.../registry/src/index.crates.io-1949cf8c6b5b557f/<crate>-<버전>`) | `native/taide-native-app/vendor/egui-input/UPSTREAM.md` 11줄 |

## 1. 플랫폼 전제

- native 앱은 macOS 전용 의존을 `[target.'cfg(target_os = "macos")'.dependencies]`에 둡니다: `muda`(메뉴바), `wry`(WKWebView 미리보기), `objc2-*`(AppKit·CoreText·CoreGraphics·ImageIO), `block2`(`native/taide-native-app/Cargo.toml` 90~100줄). unix 전용으로 `rustix`(102·103줄)가 있습니다.
- 실제 App 검사 대부분이 `#[cfg(all(test, unix))]`로 묶여 있습니다(`application.rs` 끝의 `#[path = "application-<주제>-tests.rs"] mod <주제>_tests;` 선언들, `domains/shell.rs` 끝의 `notifications_tests` 등). 다른 OS에서는 검사 수가 달라집니다.
- 앱의 OS 알림 구현은 macOS가 아니면 오류를 돌려줍니다(`native/taide-native-app/src/bootstrap.rs` `send_notification`의 `if !cfg!(target_os = "macos")`).
- macOS 외 플랫폼 지원 범위는 미결 결정입니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` "미결" 절 63줄).

## 2. Rust 툴체인과 edition

### 2.1 버전과 그 근거

1. 고정 파일이 없으므로 rustup 기본 툴체인을 씁니다. `/Users/hyunseokbyun/development/rust/rustup/settings.toml`은 `default_toolchain = "stable-aarch64-apple-darwin"`이고 `[overrides]`가 비어 있습니다. 그 툴체인의 채널 manifest(`.../toolchains/stable-aarch64-apple-darwin/lib/rustlib/multirust-channel-manifest.toml`)는 `[pkg.rustc] version = "1.98.1 (48a229cea 2026-09-01)"`, `[pkg.cargo] version = "0.99.0 (797e8a9bc 2026-08-05)"`(채널 manifest의 cargo 패키지 번호 체계), `[pkg.rustfmt-preview] version = "1.9.0"`입니다. 문서 기록도 같습니다(`docs/HANDOFF.md` 119줄 "Rust 1.98.1", `docs/quality-assurance/2026-09-30-m8-native-shell-spike.md` 7줄 "rustc/cargo 1.98.1").
2. 추가 target: 같은 stable 툴체인에 `wasm32-unknown-unknown` 표준 라이브러리가 설치돼 있습니다(`.../stable-aarch64-apple-darwin/lib/rustlib/wasm32-unknown-unknown` 디렉터리와 `manifest-rust-std-wasm32-unknown-unknown`). 동결 remote-web 컴파일 확인(8.2절)에 필요합니다. 다른 기계에서는 `rustup target add wasm32-unknown-unknown`이 필요할 수 있습니다.
3. native crate는 `rust-version = "1.95"`를 선언합니다. 구문 엔진 의존 `ferriki-textmate =0.12.0`·`ferroni =1.8.1`의 선언 MSRV가 1.94입니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 29줄). 별도로 설치된 `1.95.0-aarch64-apple-darwin` 툴체인은 이 선언 버전 확인용으로 보입니다(사용 기록은 찾지 못했습니다).
4. CI는 `dtolnay/rust-toolchain@stable`로 그때의 stable을 씁니다(`.github/workflows/ci.yml` 58줄). 다만 CI는 native crate를 검사하지 않습니다(13절).

### 2.2 edition 2024에서 알아야 할 것

이 저장소가 실제로 쓰는 2024 전용 문법은 let chain입니다. 예: `native/taide-native-app/src/application.rs` `background_tick`의

```rust
if let Some(lsp) = &self.lsp
    && let Some(error) = self.editor_rename.poll(&mut self.store, lsp)
{
    self.status = Some(crate::toast::describe_error(&self.locale, &error));
    self.toasts.ipc_error(&self.locale, &error, Instant::now());
}
```

`if let ... && let ...`과 `if let ... && <bool 식>`처럼 `let` 패턴을 `&&`로 잇는 형태는 edition 2024에서만 컴파일됩니다(`let` 없는 `if a && b`는 어느 edition에서나 됩니다). 다른 예: `application.rs` `show_window`의 `if is_enabled && !self.commands_state.keybindings.is_capturing() && context.input(|input| input.raw.focused) && let Err(error) = ...`. 공용 crate(`crates/`)는 edition 2021이라 이 문법을 쓸 수 없습니다. 그 밖의 2024 차이(꼬리 식 임시값의 drop 시점, `impl Trait` 반환의 lifetime 캡처 규칙, `if let` 임시값 범위, rustfmt의 2024 style edition 정렬)는 Rust 2024 edition guide를 확인합니다. 저장소에 `rustfmt.toml`이 없으므로 rustfmt는 `Cargo.toml`의 edition과 기본 설정으로 동작합니다.

## 3. egui/eframe 0.36.2

### 3.1 소스 위치와 TAIDE 수정

| crate | 쓰는 소스 | 연결 방법 | 수정 기록 |
| --- | --- | --- | --- |
| eframe 0.36.2 | `experiments/native-shell-spike/vendor/eframe` | 앱 `Cargo.toml` 56줄 `eframe = { version = "=0.36.2", path = "../../experiments/native-shell-spike/vendor/eframe", default-features = false, features = ["accesskit", "default_fonts", "wgpu", "preserve_empty_paste_shortcuts"] }`, remote-web `Cargo.toml` 33줄(wasm32 전용, optional) | `experiments/native-shell-spike/vendor/eframe/TAIDE-CHANGES.md` |
| egui 0.36.2 | `native/taide-native-app/vendor/egui-input` | `[patch.crates-io] egui = { path = ... }`: 앱 `Cargo.toml` 112줄, UI `Cargo.toml` 54줄, remote-web `Cargo.toml` 39줄 | `native/taide-native-app/vendor/egui-input/UPSTREAM.md` |
| egui-winit·egui-wgpu·epaint 0.36.2, wgpu 30.0.1, winit 0.30.13, accesskit 0.24.1, accesskit_winit 0.32.2 | cargo registry 원본 | 일반 의존(버전은 앱 `Cargo.lock`) | 없음 |

주의: `native/taide-native-app/vendor/eframe`에는 `data`, `LICENSE-MIT`, `README.md`만 있고 소스가 없습니다. eframe 소스를 읽을 때는 반드시 `experiments/native-shell-spike/vendor/eframe/src`를 봅니다. registry에는 eframe 0.36.2 소스가 없습니다(vendored 경로로만 받음).

eframe 수정 4건(`TAIDE-CHANGES.md` 절 순서):

1. 보조 viewport에도 AccessKit adapter를 초기화합니다(새 viewport를 숨긴 채 만들고 adapter를 붙인 뒤 요청한 visible 값을 복구, `src/native/wgpu_integration.rs`). 이미 표시된 창에 adapter를 붙이면 accesskit_winit 0.32.2가 panic하므로 순서가 필수입니다.
2. `App::raw_input_hook_with_replay` 추가(2026-10-02, `src/epi.rs`, `src/native/epi_integration.rs`): logic-only 재처리 입력과 새 입력을 구분해 터미널 pointer 이벤트 중복 복사를 막습니다.
3. `show_viewport_immediate`로 그리는 보조 창에서도 `ViewportCommand::RequestCut/Copy/Paste`를 `egui::Event::Cut/Copy/Paste`로 바꾸는 drain(`push_requested_edits`, 2026-10-27, `src/native/wgpu_integration.rs`).
4. 외부 event loop로 `create_native`를 돌릴 때 치명 오류를 꺼내는 `EframeWinitApplication::take_result`(2026-11-02, `src/native/run.rs`, `src/lib.rs`). `main.rs`가 `window_state::MonitorProbe`를 통해 `probe.take_result()`로 씁니다. 이 절에 따르면 eframe dev-dependency(`directories`)가 lock에 없어 vendored eframe crate 단위 검사는 `--locked --offline`으로 돌릴 수 없습니다.

egui 수정: 2026-11-04에 `git diff --no-index --stat /Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.36.2 /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/vendor/egui-input`로 비교하면 소스 11개와 `Cargo.toml`이 다릅니다: `src/context.rs`(가장 큼, +815줄 수준), `src/memory/mod.rs`, `src/pass_state.rs`, `src/input_state/mod.rs`, `src/widgets/button.rs`, `src/containers/menu.rs`, `src/containers/popup.rs`, `src/containers/scroll_area.rs`, `src/containers/modal.rs`, `src/hit_test.rs`, `src/ui.rs`. `UPSTREAM.md` 11줄의 "공통 116파일 중 10파일 변경"은 2026-10-05 기준이라 지금은 맞지 않습니다. 내용은 버튼 Space 키 해제 시 활성화, 메뉴 submenu open override, 포인터·키보드 focus 소유 조회(`Context::keyboard_input_route`, `keyboard_focus_request_at`, `keyboard_navigation_is_locked` 등) 같은 입력 경계입니다(`UPSTREAM.md` 각 문단).

### 3.2 0.36 API에서 옛 지식과 다른 점(이 저장소에서 실제로 쓰는 형태)

1. `eframe::App`에는 `update`가 없습니다. `fn logic(&mut self, ctx: &egui::Context, frame: &mut Frame)`(그리기 금지, 창이 숨겨져도 repaint 요청이 있으면 호출)과 `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut Frame)`(root viewport 그리기)로 나뉩니다(`experiments/native-shell-spike/vendor/eframe/src/epi.rs` 152~182줄). TAIDE는 `logic`에서 `background_tick`, `ui`에서 `frame_ui`를 부릅니다(`application.rs` `impl eframe::App for NativeApplication`).
2. 보조 창은 `context.show_viewport_immediate(viewport_id, builder, |ui, _class| { ... })`로 그립니다. 클로저가 `&mut Ui`를 받습니다(`domains/shell.rs` `show_auxiliary_windows`).
3. 옆 패널은 `egui::Panel::left(id)`를 씁니다(UI `shell.rs` 401·1437줄 부근).
4. 시작은 `eframe::run_native`가 아니라 `eframe::create_native(title, options, app_creator, &event_loop)`로 앱 핸들러를 만들고 직접 만든 `winit::event_loop::EventLoop::<eframe::UserEvent>::with_user_event().build()?`의 `run_app`으로 돌립니다(`native/taide-native-app/src/main.rs` `main`). 모니터 목록을 읽기 위한 결정입니다(`docs/acknowledge/2026-10-26-native-autonomous-continuation.md` 단계 E 결정 "shell-3 모니터 목록").
5. headless 검사는 `egui::Context::default()`, `eframe::CreationContext::_new_kittest(context.clone())`, `eframe::Frame::_new_kittest()`, `context.run_ui(egui::RawInput { .. }, |ui| { eframe::App::logic(..); eframe::App::ui(..); })`로 한 프레임을 돌립니다(`native/taide-native-app/src/application-notifications-tests.rs` `launch_with_platform`, `frame`).
6. 의심스러운 API는 기억으로 쓰지 말고 위 vendored 소스와 registry 소스에서 시그니처를 확인합니다.

## 4. native crate별 주요 의존과 고정 버전

모든 native crate는 루트 workspace 밖의 독립 workspace이고(각 `Cargo.toml`에 `[workspace]`), 각자 `Cargo.lock`을 가집니다(`ls native/*/Cargo.lock`: app·ui·editor·syntax·terminal·retained·remote-web 7개). 아래 `=x.y.z`는 `Cargo.toml`의 정확 고정, 괄호 안은 `Cargo.lock` 값입니다.

### 4.1 `native/taide-native-app`(실행 파일)

| 용도 | 의존(버전) |
| --- | --- |
| UI 런타임 | `eframe =0.36.2`(vendored), `winit =0.30.13`(`default-features = false`), `egui`(patch, 0.36.2) |
| native crate | `taide-native-ui`, `taide-native-editor`, `taide-native-syntax`, `taide-native-terminal`, `taide-native-retained`(모두 path) |
| 공용 crate 16개 | `taide-model`, `taide-runtime`, `taide-terminal`, `taide-layout`, `taide-infra`, `taide-remote`, `taide-settings`, `taide-plugin`, `taide-ide`, `taide-file`, `taide-font`, `taide-git`, `taide-system`, `taide-window`, `taide-agent`, `taide-lsp`(모두 path) |
| 비동기 | `tokio = "1"`(lock 1.53.1, features `rt-multi-thread`·`sync`·`time`·`process`·`io-util`·`macros`·`net`), `futures-util =0.3.33` |
| HTTP·WS(원격 서버·미리보기) | `hyper =1.11.0`, `hyper-util =0.1.20`, `http =1.5.0`, `http-body-util =0.1.4`, `axum =0.8.9`, `tokio-tungstenite =0.30.0`, `bytes =1.12.1`, `url =2.5.8` |
| 미리보기 | `image =0.25.10`, `gif =0.14.2`, `resvg =0.48.1`, `calamine =0.36.1`, `cfb =0.15.0`, `zip =2.4.2`, `quick-xml =0.41.0`, `encoding_rs =0.8.42`, `codepage =0.1.3`, `rhwp =0.8.2`(vendored), `pulldown-cmark =0.13.4`, `html5ever =0.38.0`, `dom_query =0.27.0`, `flate2 =1.1.9`, `base64 =0.23.1` |
| 글꼴·로케일 | `skrifa =0.44.0`, `sys-locale =0.3.2`, `icu_normalizer =2.3.0`, `icu_locale_core =2.3.0`, `icu_collator =2.3.1` |
| 기타 | `arboard =3.6.1`(클립보드), `rfd =0.17.2`(파일 선택), `regex =1.13.1`, `chrono =0.4.45`, `iana-time-zone =0.1.65`, `ryu-js =1.0.3`, `num-bigint =0.4.8`, `num-traits =0.2.19`, `uuid = "1"`(v4), `serde`/`serde_json = "1"`, `log = "0.4"`, unix `rustix =1.1.4` |
| macOS 전용 | `muda =0.21.0`, `wry =0.55.1`(vendored `vendor/wry-preview`, features `os-webview`·`native-preview-deny-permissions`), `objc2 =0.6.4`, `objc2-app-kit`·`objc2-foundation`·`objc2-core-text`·`objc2-core-foundation`·`objc2-core-graphics`·`objc2-image-io =0.3.2`, `block2 =0.6.2` |
| 개발 전용 | `taide-native-ui`(features `inspection`), `tokio`(`test-util`), `png =0.18.1`, `ravif =0.13.0` |
| 전이(공용 crate 경유, lock 값) | `git2 0.21.0`(+`libgit2-sys 0.18.7+1.9.6`, `openssl-src 300.6.1+3.6.3`; `crates/taide-git/Cargo.toml` 9줄의 `vendored-libgit2`·`vendored-openssl`), `portable-pty 0.9.0`, `notify 8.2.0`, `trash 5.2.6`, `keyring 3.6.3`, `lsp-types 0.97.0`, `alacritty_terminal 0.26.0`(vendored), `vte 0.15.0`(vendored) |

추가 설정(같은 `Cargo.toml`):

- `[[example]] native-lsp-mock`: 경로 `../../experiments/lsp-coordinator-spike/src/bin/mock-server.rs`(9~11줄).
- `[[bin]] native-terminal-queue-fixture`: 경로 `../taide-native-terminal/tests/fixtures/session.rs`(13~15줄). 기본 bin `taide-native-app`(`src/main.rs`)과 합쳐 bin이 둘입니다.
- `[workspace] exclude = ["vendor/rhwp", "vendor/cfb-retained", "vendor/zip-retained", "vendor/wry-preview", "vendor/egui-input"]`.
- `[features] inspection = ["taide-native-ui/inspection"]`.
- `[patch.crates-io]`: `egui`, `vte`(111~113줄).
- `[profile.dev] debug = "line-tables-only"`, `[profile.dev.package."*"] debug = false`(115~119줄, wave 0 커밋 `72065136`).

### 4.2 `native/taide-native-ui`

`egui =0.36.2`(patch), `resvg =0.48.1`(`default-features = false`), `web-time =1.1.0`, `icu_*`(위와 같음), `serde`/`serde_json`, `taide-model`, `taide-native-editor`. feature `native-host`(기본값)일 때만 `taide-runtime`, `taide-layout`, `taide-project`, `taide-infra`, `taide-file`, `tokio(sync)`이 붙습니다. feature `inspection`은 검사용 조회 API입니다. `[[test]] controller`·`document_admission`·`workbench`는 `native-host`, `snippet-editor`는 `native-host`+`inspection`이 필요합니다(`native/taide-native-ui/Cargo.toml` 11~51줄). 정규식 의존이 없습니다(4.4절 이유).

### 4.3 `native/taide-native-editor`

`lsp-types =0.97.0`, `unicode-segmentation =1.13.3`, `ropey =1.6.1`(features `cr_lines`·`simd`), `taide-model`. egui와 정규식에 의존하지 않습니다.

### 4.4 `native/taide-native-syntax`

`ferriki-textmate =0.12.0`(vscode-textmate 이식), `ferroni =1.8.1`(Oniguruma 이식), `regress =0.12.0`(feature `utf16`, JS 정규식), `serde`(derive), `serde_json`, `taide-model`, `taide-native-editor`. 채택 근거: `docs/acknowledge/2026-10-06-native-transition-decisions.md` "구문 강조 엔진 결정" 표, `2026-10-09-native-find-regex-decisions.md` "성능 기준과 채택". 정규식이 필요한 코드를 이 crate에 두는 이유는 동결 remote-web이 UI·editor crate를 소비하기 때문입니다(7절 5번).

### 4.5 `native/taide-native-terminal`

`alacritty_terminal =0.26.0`(vendored `vendor/alacritty-terminal`, `default-features = false`, feature `native-retained`), `taide-native-retained`, `taide-infra`, `taide-model`, `[patch.crates-io] vte`(vendored `experiments/terminal-core-spike/vendor/vte`), `[[bin]] native-session-fixture`.

### 4.6 `native/taide-native-retained`, `native/taide-remote-web`

- retained: 외부 의존 없음(자체 `derive` 하위 crate를 workspace member로 둠), retained 메모리 계측 유틸.
- remote-web(동결): `taide-native-ui`(`default-features = false`), `taide-native-editor`, `taide-remote-wire`, `taide-model`, `egui =0.36.2`(patch), wasm32 전용 `eframe =0.36.2`(vendored, feature `canvas` 선택 시), `js-sys =0.3.106`, `wasm-bindgen =0.2.129`, `web-sys =0.3.106`. `[[test]] settings-resources`·`snippet-operations`는 feature `inspection`이 필요합니다.

### 4.7 vendored crate 목록

| 경로 | 원본 | 바꾼 이유 | 기록 |
| --- | --- | --- | --- |
| `experiments/native-shell-spike/vendor/eframe` | eframe 0.36.2 | 3.1절 4건 | `TAIDE-CHANGES.md` |
| `native/taide-native-app/vendor/egui-input` | egui 0.36.2(upstream commit `49682f8b...`) | 입력·focus 경계 | `UPSTREAM.md` |
| `native/taide-native-app/vendor/wry-preview` | wry 0.55.1 | 미리보기 WebView의 파일 업로드 패널 거절, 카메라·마이크 권한 Deny(feature `native-preview-deny-permissions`) | `UPSTREAM.md` |
| `native/taide-native-terminal/vendor/alacritty-terminal` | alacritty_terminal 0.26.0 | `native-retained` feature(조회 DTO 등), 로그 redaction, keyboard stack 포화 시 title stack 삭제 오류 수정 | `UPSTREAM.md` |
| `experiments/terminal-core-spike/vendor/vte` | vte 0.15.0 | OSC raw buffer 상한(기본 4096 bytes, 초과 OSC 폐기) 등 | `TAIDE-CHANGES.md` |
| `native/taide-native-app/vendor/rhwp` | rhwp 0.8.2(commit `9b16aa9e...`) | HWP 미리보기(feature `native-retained`), 하위 `cfb-retained`·`zip-retained`를 path로 참조 | `UPSTREAM.md` |
| `native/taide-native-app/vendor/cfb-retained`, `zip-retained` | cfb, zip | rhwp용 retained fork | 각 `UPSTREAM.md` |

vendored 소스는 라이선스상 원본 주석·저작권을 보존합니다. 수정하면 해당 `TAIDE-CHANGES.md`/`UPSTREAM.md`에 날짜 절을 추가합니다.

## 5. 원본 TS 스택

| 항목 | 버전 | 근거 |
| --- | --- | --- |
| React / React DOM | 19.2.8 | `package.json` 60·61줄 |
| monaco-editor / monaco-editor-core | 0.56.0 | 57·91줄 |
| @shikijs/core·engine-javascript·langs·monaco | 4.4.3 | 36~39줄 |
| @xterm/xterm | 6.0.0(addon-fit 0.11.0, search 0.16.0, unicode11 0.9.0, web-links 0.12.0, webgl 0.19.0) | 45~50줄 |
| @tauri-apps/api / cli | 2.11.1 / 2.11.4 | 42·78줄 |
| Vite / TypeScript / Tailwind | 8.2.0 / 5.9.3 / 4.3.3 | 97·95·93줄 |
| 그 밖 | radix-ui 1.6.7, lucide-react 1.28.0, sonner 2.0.7, cmdk ^1.1.1, i18next ^26.3.6, @tanstack/react-query 5.101.4, react-resizable-panels 4.12.2, @dnd-kit/core 6.3.1, emmet-monaco-es 5.7.0, pdfjs-dist ^6.2.108, xlsx ^0.18.5, @rhwp/core ^0.8.2 | 같은 파일 |

- 원본 스크립트(`package.json` `scripts`): `bun test`, `tsc --noEmit`(typecheck), `eslint .`, `prettier --check .`(format:check), `cargo fmt --all --check`·`cargo clippy --workspace --all-targets -- -D warnings`·`cargo test --workspace`(루트 workspace, native 제외).
- 원본 동작을 확인할 때는 메인 작업 트리의 `node_modules`(Monaco·Shiki·xterm 실제 소스)를 읽습니다. lane worktree에는 `node_modules`가 없어 메인 트리 것을 읽기 전용으로 씁니다(`docs/utils/2026-11-02-native-lane-workflow.js` `BACKGROUND` 30줄).
- 원본 동작의 기준값(oracle)을 만드는 스크립트는 `docs/utils/2026-10-09-monaco-*-oracle.js`, `2026-10-10-monaco-completion-*-oracle.js` 같은 파일로 남아 있고 `bun <절대 경로>` 한 줄로 실행합니다(`docs/memory/2026-10-07-native-transition-operations.md` 6절 "bun 스크립트").

## 6. 디렉터리 구조

- `/Users/hyunseokbyun/development/TAIDE`(메인 작업 트리, 브랜치 `to_rust_native`)
    - `src/`: 원본 React/TS 화면(FSD: `app`, `widgets`, `features`, `entities`, `shared`)
    - `src-tauri/`: 원본 Tauri 앱(루트 workspace 멤버). `src-tauri/tests/fixtures/rust-native/`에 영속 형식 fixture(`persistence-v1.json` 등)
    - `crates/taide-*`: 공용 Rust crate 30개(루트 workspace 멤버). native 앱이 path로 씁니다
    - `native/`: 전환 대상 native crate 7개(각각 독립 workspace)
        - `taide-native-app`: 실행 파일, 앱 조립(`src/main.rs`, `src/application.rs`, `src/host.rs`, `src/domains/*.rs`), 통합 검사 `tests/*.rs`(63개 파일 + `tests/fixtures`), `vendor/`, `resources/icons/`
        - `taide-native-ui`: egui 화면(`src/shell.rs`, `src/editor_surface.rs`, `src/intents/`, `src/surfaces/` 등), 검사 `tests/*.rs`
        - `taide-native-editor`: egui 비의존 편집 코어
        - `taide-native-syntax`: 구문 토큰화·정규식, 문법 자산 `grammars/`, `language-configurations/`(둘 다 `.prettierignore`에 있음)
        - `taide-native-terminal`: 터미널 코어, `vendor/alacritty-terminal`
        - `taide-native-retained`: 메모리 계측
        - `taide-remote-web`: 브라우저 Wasm 클라이언트(동결)
    - `experiments/`: 기술 실험. 제품이 쓰는 것은 `native-shell-spike/vendor/eframe`, `native-shell-spike/target`(메인 cargo target), `terminal-core-spike/vendor/vte`, `lsp-coordinator-spike/src/bin/mock-server.rs`(LSP mock 예제 소스)
    - `target/`: 루트 workspace(Tauri 앱·공용 crate)의 기본 cargo target
    - `docs/`: 작업 문서(지도: 01 문서 6절)
    - `node_modules/`: 원본 라이브러리 소스(읽기 전용 참조)
- `/Users/hyunseokbyun/development/TAIDE-lanes/<lane>`: lane worktree(브랜치 `lane/<lane>`). 2026-11-04 `git -C /Users/hyunseokbyun/development/TAIDE worktree list`에 `editor`, `explorer-search`, `git`, `preview`, `settings`, `shell`, `terminal-agent` 7개가 있습니다. 모두 낡았으므로 사용 금지입니다
- `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`: lane 전용 cargo target

## 7. cargo 사용 원칙

1. native crate는 루트 workspace에 없으므로 항상 `--manifest-path <crate 절대 경로>/Cargo.toml`을 씁니다. 저장소 루트에서 `cargo test`를 치면 native crate가 아니라 Tauri 앱과 공용 crate가 빌드됩니다.
2. 항상 `--locked --offline`을 붙입니다. lockfile이 바뀌거나 네트워크로 내려받는 일을 막습니다. 새 path 의존 추가로 lockfile을 갱신해야 할 때만 사용자가 허락한 범위에서 `--locked`를 빼고 `--offline`으로 1회 실행합니다(`docs/HANDOFF.md` 7절 121줄).
3. 메인 작업 트리는 `--target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`을 씁니다. lane은 자기 target `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`을 씁니다. 같은 target을 여러 프로세스가 쓰면 cargo 잠금으로 직렬화됩니다(lane 운영 문서 2.2절).
4. cargo는 한 번에 하나만 실행하고 `--quiet`를 붙입니다(`docs/memory/2026-10-07-native-transition-operations.md` 3절, lane workflow `RULES`).
5. `Cargo.toml`·`Cargo.lock` 변경은 작업마다 허용 범위를 명시합니다. 동결 remote-web의 lockfile과 의존 그래프는 바뀌면 안 됩니다. 그래서 정규식이 필요한 코드는 `taide-native-syntax`(앱만 의존)에 두고 UI·editor crate에는 정규식 의존을 넣지 않습니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 기각 대안 표 39줄).
6. 앱 crate에는 bin이 둘(`taide-native-app`, `native-terminal-queue-fixture`)이라 `cargo run`·`cargo build`에 `--bin taide-native-app`이 필요합니다(transition 운영 문서 6절).

## 8. 빌드·검사 명령 전체

아래는 메인 작업 트리 기준입니다. lane에서는 경로의 `/Users/hyunseokbyun/development/TAIDE`를 `/Users/hyunseokbyun/development/TAIDE-lanes/<lane>`으로, target을 `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`으로 바꿉니다.

### 8.1 선행 빌드: LSP mock

앱 lib 검사의 실제 LSP 경로는 `native-lsp-mock` 예제 실행 파일을 씁니다. target이 비었거나 정리(11절) 뒤에는 먼저 빌드합니다. 없으면 `assertion failed: mock.is_file()`로 실패합니다(transition 운영 문서 6절은 10건, seams 문서 3절 끝은 "수십 개"로 기록).

```
cargo build --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target
```

결과 파일: `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target/debug/examples/native-lsp-mock`.

### 8.2 컴파일 확인

```
cargo check --quiet --all-targets --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target
cargo check --quiet --all-targets --features native-host,inspection --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target
cargo check --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target
```

- UI crate는 `--features native-host,inspection` 없이 검사하면 `[[test]]`의 `required-features` 때문에 일부 대상이 빠집니다.
- remote-web 확인은 UI·editor crate를 바꿨을 때 반드시 실행합니다(동결 crate가 컴파일되는지 확인).
- 기존 경고(새 경고와 구분할 것, 2026-10-28 seams 문서 5.8절 8번 기록이며 이후 줄 번호·개수가 바뀌었을 수 있음): `editor-code-actions.rs` `is_applying`, `zen.rs`의 `mut`, `application-inlay-hints-tests.rs` `fixture_with_emmet_peek`, UI `tests/workbench.rs`의 `mut`, vendor `wry-preview` 경고, remote-web dead code 경고. 새 경고인지 판단하려면 기준 커밋 worktree의 같은 명령 출력과 비교합니다.

### 8.3 앱 crate 검사

앱 전체(66개 대상: lib 1 + `tests/*.rs` 63 + bin 2)를 휴지통 검사 없이 돌리는 명령입니다. `--skip` 세 개는 12.3절의 실제 휴지통 검사입니다. libtest의 `--skip`은 부분 문자열 일치이고 cargo가 모든 테스트 바이너리에 같은 인자를 넘기므로, lib에 있는 검사와 통합 검사 대상에 있는 검사를 한 명령에서 함께 걸러 냅니다.

```
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target --no-fail-fast -- --test-threads=4 --skip 실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다 --skip 실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다 --skip 실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다
```

- `--test-threads=4`는 실제 App 검사의 동시 수를 줄여 host 큐 포화 흔들림을 줄이려는 값입니다(lane 운영 문서 2.3절 3번). lane에서 다른 lane과 동시에 돌릴 때는 `--test-threads=2`를 씁니다(lane workflow `RULES` 48줄).
- lib만 영역 필터로 빠르게 돌릴 때:

    ```
    cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --lib --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target -- --test-threads=2 domains::shell:: provider_events native_notifications
    ```

    libtest는 공백으로 나열한 필터 중 하나라도 이름에 포함되면 실행합니다. 영역별 필터 목록은 `docs/research/2026-10-28-native-lane-seams.md` 3절 표입니다(Git 영역 필터는 `domains::git::`입니다. `title_branch` 필터는 모듈 삭제로 없어졌습니다). 실제 App 검사 모듈 경로는 `application::<주제>_tests::<이름>`(`application.rs` 끝의 `#[path = "application-<주제>-tests.rs"] mod <주제>_tests;`)이거나, 영역 파일에 붙은 것은 `domains::<영역>::<주제>_tests::<이름>`(예: `domains/shell.rs` 끝의 `#[path = "../application-notifications-tests.rs"] mod notifications_tests;`)입니다.
- 한 검사만 정확히: `-- --exact application::view_state_tests::실제_일괄_닫힘은_주_보조_영역의_독립_위치와_접기를_보존한다`.
- 통합 검사 대상 하나만: `cargo test --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --test host --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`. `--test explorer-delete`와 `--test lsp-workspace-worker`는 12.3절 검사를 포함하므로 `--skip` 없이 실행하지 않습니다.

### 8.4 다른 native crate 검사

```
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-ui/Cargo.toml --features native-host,inspection --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target --no-fail-fast
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-editor/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target --no-fail-fast
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-syntax/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target --no-fail-fast
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-terminal/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target --no-fail-fast
```

- 2026-11-03 게이트의 대상 수: UI 30, editor 72, syntax 20, terminal 13(`docs/PROCESS.md` 30줄). UI 검사만 빠르게: `--lib --test workbench`(seams 문서 5.8절 4번).
- remote-web 검사(선택, 과거 게이트에서 사용): `cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-remote-web/Cargo.toml --features inspection --locked --offline --target-dir <target> --no-fail-fast`(transition 운영 문서 4절 1번). 현재 병합 게이트 스크립트는 Wasm check만 합니다.

### 8.5 형식 검사

```
cargo fmt --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml -- --check
```

`taide-native-ui`, `taide-native-editor`, `taide-native-syntax`, `taide-native-terminal`도 같은 형태입니다. 모듈 선언 순서는 rustfmt 기본 `reorder_modules`를 따릅니다(seams 문서 1.4절 표). 저장소 루트의 `prettier --check .`는 `native/` 아래 JSON도 검사하므로, 추출·생성한 JSON(문법 파일 등 라이선스상 수정 금지 자산)은 `.prettierignore`에 넣습니다(transition 운영 문서 6절. 현재 `native/taide-native-syntax/grammars`, `tests/fixtures`, `language-configurations`가 들어 있음).

### 8.6 공용 crate(`crates/`)를 바꿨을 때

공용 crate는 루트 workspace입니다. native 앱과 Tauri 앱이 함께 쓰므로 둘 다 확인합니다.

```
cargo check --workspace --locked --offline --manifest-path /Users/hyunseokbyun/development/TAIDE/Cargo.toml
cargo test -p taide-runtime --locked --offline --manifest-path /Users/hyunseokbyun/development/TAIDE/Cargo.toml
```

`-p taide-git`(또는 루트 workspace 전체 `cargo test`)는 실제 휴지통을 쓰는 검사 4개를 실행하므로 `-- --skip untracked_파일_discard는_휴지통으로_이동해_워킹트리에서_사라진다 --skip tracked와_untracked를_함께_discard해도_각각_처리된다 --skip 미추적_디렉토리_행은_절대경로로_discard해도_워킹트리에서_사라진다 --skip 링크_개명과_삭제는_원본을_보존한다`를 붙입니다(00-README 1절 2번).

`-p` 뒤에는 바꾼 crate 이름을 씁니다. 루트 workspace는 기본 target(`/Users/hyunseokbyun/development/TAIDE/target`)을 씁니다. 2026-11-03 settings 병합 때 공용 crate 6개 검사와 루트 workspace check를 이렇게 했습니다(`docs/PROCESS.md` 32줄). 루트 check가 CPU 0%로 오래 멈춘 사례가 있습니다(transition 운영 문서 6절 첫 항목, 원인 미확인).

### 8.7 병합 게이트 스크립트

```
bash /Users/hyunseokbyun/development/TAIDE/docs/utils/2026-11-03-native-merge-gate.sh <로그를 쓸 절대 경로 디렉터리>
```

스크립트가 하는 일(파일 31줄 전체): LSP mock 빌드 → 앱 전체 검사(`--no-fail-fast -- --test-threads=4`, `app.log`) → UI(`native-host,inspection`, `ui.log`) → editor·syntax·terminal → remote-web Wasm check → 다섯 crate fmt check → 로그마다 `test result: ok` 수와 `... FAILED`·`error` 줄 출력. 각 단계의 종료 코드를 `app 0`처럼 출력합니다. 경로가 메인 작업 트리로 고정돼 있어(3줄 `ROOT=`) lane worktree에서는 그대로 쓸 수 없습니다.

스크립트는 34fb88d7부터 앱의 실제 휴지통 검사 3개를 `--skip`합니다(`TRASH_TESTS` 배열). 루트 workspace의 `taide-git` 휴지통 검사 4개는 스크립트가 실행하지 않으므로 공용 crate 검사(8.6절)에서 직접 `--skip`합니다(00-README 1절 2번).

### 8.8 검증 범위를 고르는 기준

| 바꾼 것 | 최소 검증 |
| --- | --- |
| 앱 crate 한 영역 파일 | 앱 `cargo check --all-targets` + 해당 영역 필터 lib 검사 + 경로를 바꾼 통합 대상 + 앱 fmt |
| UI crate | UI check(features) + UI 관련 검사(`--lib --test workbench` 등) + remote-web Wasm check + UI fmt + 그 UI 코드를 쓰는 앱 영역 필터(앱 crate가 일부 UI 소스를 `include!`로 다시 컴파일하므로, 13절) |
| editor·syntax·terminal crate | 그 crate 전체 검사 + 그 crate를 소비하는 앱 영역 필터 + fmt(editor는 remote-web도 소비하므로 Wasm check 추가) |
| 공용 crate | 8.6절(`-p taide-git`이면 휴지통 검사 4개 이름을 `--skip`) + native 앱 check |
| lane 병합 | 8.1~8.5절 전체(휴지통 검사 제외) |

실행하지 않은 검사를 통과했다고 쓰지 않습니다. 같은 입력·코드의 성공 결과는 다시 돌리지 않습니다(사용자 규칙).

## 9. 앱 실행

### 9.1 빌드와 실행

```
cargo build --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --bin taide-native-app --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target
/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target/debug/taide-native-app --data-dir <격리된 절대 경로>
```

- `--data-dir <경로>`는 첫 인자여야 합니다. 없거나 다른 인자가 먼저 오면 `native app requires --data-dir <absolute isolated directory>` 오류로 끝납니다. 경로가 없으면 `native data directory is missing`, 상대 경로·`..` 포함·파일시스템 루트면 `native data directory must be absolute and must not be a filesystem root`입니다(`native/taide-native-app/src/bootstrap.rs` `LaunchConfig::parse_data_dir`).
- `--data-dir` 뒤의 나머지 인자는 외부 열기 요청(`<file> [--wait-marker <marker>]`)으로 해석합니다(`bootstrap.rs` `Launch::parse`, `crate::external_open::request_from`). unix에서는 `main.rs`가 `single_instance::acquire(&single_instance::socket_path(&config.data_dir), &relay_arguments)`로 같은 데이터 디렉터리의 실행 중 인스턴스를 찾아 인자를 전달하고 끝나거나(`Claim::Relayed`), 자기가 첫 인스턴스가 됩니다. 소켓은 `std::env::temp_dir()` 아래에 만들고 종료 때 지웁니다(`single-instance.rs` `socket_path`, `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` 채택 절).
- Keychain 서비스명은 격리용 `net.gumyo.taide.native-isolated`입니다(`bootstrap.rs` `SECRET_SERVICE`). 원본 앱의 identifier `net.gumyo.taide`와 섞이지 않습니다. 실제 사용자 데이터 경로·keychain 승계는 `shell-15`이며 사용자 허락이 필요합니다.
- 첫 인자가 `--html-preview-helper`이면 GUI 대신 HTML 미리보기 helper 프로세스로 동작합니다(`main.rs`, `preview_web_helper::FLAG`). 앱이 자기 실행 파일을 helper로 다시 띄우는 용도이므로 직접 쓰지 않습니다. helper는 다른 인자를 받지 않습니다.
- 창 제목은 `TAIDE`(`main.rs` `WINDOW_TITLE`), 초기 크기 1400×900, 최소 720×480입니다(`INITIAL_SIZE`, `MINIMUM_SIZE`).
- 종료는 창을 닫거나 `kill -TERM <pid>`를 씁니다. 종료 경로에서 `window_state.save()`와 `exit_application`이 불립니다(`application.rs` `on_exit`).
- lane 작업자(하위 에이전트)는 GUI를 실행하지 않습니다(lane workflow `RULES` 50줄). GUI 실행은 사용자 맥락이 있는 메인만 합니다.

### 9.2 상태가 있는 데이터 디렉터리 만들기

데이터 디렉터리 안의 파일 이름은 `crates/taide-model/src/paths.rs` `AppPaths`가 정합니다: `settings.json`, `session.json`, `projects/<프로젝트 id>/project.json`, `projects/<프로젝트 id>/layout.json`, `projects/<id>/buffers/`, `themes/`, `snippets/`, `locales/`, `plugins/`, `lsp/` 등. `src-tauri/tests/fixtures/rust-native/persistence-v1.json`의 `sessionLegacy`·`projectLegacy`·`layoutLegacy` 형식을 따라 `session.json`, `projects/<id>/project.json`, `projects/<id>/layout.json`을 쓰면 프로젝트와 파일 탭이 열린 상태로 시작합니다(`docs/memory/2026-10-07-native-transition-operations.md` 5절 1번). fixture의 `__FIXTURE_ROOT__`·`__FIXTURE_FILE__` 자리에는 합성 샘플 디렉터리의 절대 경로를 넣습니다. 실제 사용자 프로젝트를 쓰지 않습니다. 이 방법으로 실제 프로젝트가 열리는지는 2026-10-07에 화면 잠금 때문에 확인하지 못했다고 기록돼 있습니다(같은 문서 5절 마지막 문장).

## 10. 화면 캡처

`docs/memory/2026-10-07-native-transition-operations.md` 5절의 방법입니다. 메인만 하고, 하위 작업자는 하지 않습니다.

1. 9.2절처럼 캡처용 데이터 디렉터리를 만듭니다.
2. 9.1절처럼 앱을 실행합니다.
3. CoreGraphics의 `CGWindowListCopyWindowInfo`로 창 목록을 읽어, 소유자 이름(`kCGWindowOwnerName`)에 `taide-native-app`이 들어 있고 창 이름(`kCGWindowName`)이 `TAIDE`인 창의 번호(`kCGWindowNumber`)를 얻습니다. Swift 스크립트 한 파일로 할 수 있습니다(저장소에 스크립트는 남아 있지 않습니다).
4. `screencapture -x -o -l <창 번호> <저장 파일>`로 그 창만 캡처합니다. 다른 창에 가려져 있어도 그 창만 찍힙니다.
5. `kill -TERM <pid>`로 종료합니다.

주의:

- 화면이 잠겨 있으면 창 내용이 그려지지 않아 흰 화면이 찍힙니다. `CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements])`의 첫 창 소유자가 `ScreenLock`이면 잠긴 상태입니다.
- 합성 키·마우스 입력은 사용자의 다른 창으로 갈 수 있어 쓰지 않습니다. 화면 상태는 데이터 디렉터리를 미리 만들어 정하는 방식으로만 바꿉니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 기각 대안 표 42줄).
- `experiments/native-shell-spike/target/TAIDE M8 Egui Spike.app`, `TAIDE M8 Iced Spike.app`은 사용자 실기용 번들이라 건드리지 않습니다(`docs/HANDOFF.md` 5절 8번).

## 11. 디스크 관리

| 사실 | 값 | 근거 |
| --- | --- | --- |
| 앱 검사 대상 한 벌 크기 | 약 25GB | `docs/PROCESS.md` 26·28줄 |
| dev 프로필 | `[profile.dev] debug = "line-tables-only"`, `[profile.dev.package."*"] debug = false`(app·ui·editor·syntax·terminal의 `Cargo.toml`) | wave 0 커밋 `72065136` |
| 과거 최대 | 메인 target 489GB, 디스크 99% | lane 운영 문서 1절 표 |
| 2026-11-04 관찰 | `df -h /Users/hyunseokbyun`: 1.8Ti 중 여유 570Gi(사용 69%) | 이 문서 작성 중 실행 |

1. 메인 target 정리는 `cargo clean --profile dev --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`로 합니다. `--profile dev`는 `debug` 산출물만 지웁니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 기각 대안 표 44줄).
2. 메인 target에 `--profile` 없는 `cargo clean`을 쓰지 않습니다. target 디렉터리 전체가 지워져 그 안의 보호 번들(10절 마지막 항목)까지 사라집니다. 메인 target에는 `debug`, `release`, `tmp`, `wasm32-unknown-unknown`, 보호 번들 두 개가 있습니다.
3. `rm -rf`는 자동 모드에서 거부되고 쓰지 않습니다. `cargo clean *`은 사용자가 허용 규칙을 추가했습니다(transition 운영 문서 6절).
4. lane 병합이 끝나면 사용자 확인 후에 그 lane의 target을 `cargo clean --manifest-path /Users/hyunseokbyun/development/TAIDE-lanes/<lane>/native/taide-native-app/Cargo.toml --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`으로 회수합니다(lane 운영 문서 2.3절 4번). lane target에는 보호 번들이 없습니다.
5. 디스크 여유는 `df -h /Users/hyunseokbyun`으로 확인합니다. 동시 lane 수 = min(4, (여유 − 100GB) / 한 벌 크기)입니다(lane 운영 문서 2.2절). 2026-11-02에는 사용자 판단으로 7개 lane을 동시에 돌렸습니다(PROCESS 28줄).
6. 정리 뒤에는 8.1절 LSP mock을 다시 빌드합니다.

## 12. 알려진 기존 실패와 흔들림

### 12.1 결정적 기존 실패(코드를 안 바꿔도 실패)

| 검사 | 증상 | 판별과 성격 |
| --- | --- | --- |
| 앱 lib `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`(`native/taide-native-app/src/editor-fonts.rs` 514줄 부근) | `left: [0, 2, 3, 5, 6, 2, 3, 6]`, `right: [0, 2, 3, 5, 6]` | 단독 재실행에서도 같은 값으로 실패하면 기존 실패로 기록합니다. W0 게이트(2026-10-28)부터 기록됐습니다(`docs/PROCESS.md` 26줄). `right`는 같은 파일의 `REGULAR_FACES = [0, 2, 3, 5, 6]`이므로 편집기 regular 글꼴 체인 단언(`assert_eq!(paddings(&definitions, &regular, &font), REGULAR_FACES)`)이 실패합니다. seams 문서 0절은 "시스템 글꼴 fallback 체인을 단언"한다고 적었지만, 이 검사는 `stack()`이 내장 글꼴 바이트로 만든 메모리 `Database`로 `terminal_fonts::prepare_requested`를 부르는 순수 함수 검사입니다(`docs/quality-assurance/2026-11-02-native-lane-terminal-agent-terminal-core.md` 93·102줄). 같은 QA는 "편집기 글꼴 체인 꼬리에 UI 코드 글꼴이 섞이는 순서가 의도인지 확인되지 않음"을 남은 위험으로 적었습니다. 즉 원인이 기대값인지 제품 결함인지 미확정입니다. 기대값을 바꿔 통과시키지 말고 원인을 확인한 뒤 고칩니다 |
| 루트 workspace `crates/taide-runtime/tests/agent_probe.rs`의 `native는_같은_등록_감독자와_lazy_os_port를_모든_probe_호출에_주입한다`(292줄) | 실패 | `81e96198`("refactor(crates): native 공유를 위한 Tauri 전용 로직의 crate 이전")부터 있던 실패입니다(`docs/PROCESS.md` 32줄). 이 검사는 `include_str!`로 `src-tauri/src/domain/agent/commands.rs`·`hooks.rs`와 `crates/taide-runtime/src/agent_hook_reconcile.rs`의 소스 문자열을 단언합니다. 이 파일들을 바꾸지 않았는데 실패하면 기존 실패입니다. 이 파일에서 소스 문자열을 단언하는 검사는 이것 하나입니다 |

과거의 상시 실패였던 `tests/preview-spreadsheet-xlml.rs`는 배치 8에서 고쳤습니다(`docs/quality-assurance/2026-10-09-native-batch8-integration.md` 13줄 XLML 행).

### 12.2 부하 시 흔들리는 실제 App 검사

실제 App 검사는 `NativeApplication::new_with_overrides`로 진짜 Tokio runtime·host worker와 함께 앱을 띄워 프레임을 돌리고 시간 제한(대개 10초, 예: `application-notifications-tests.rs` `DEADLINE`) 안에 상태를 기다립니다. 동시에 많이 돌면 host 명령 큐(용량 64, `host.rs` `HOST_COMMAND_CAPACITY`)가 차거나 종료 drain이 늦어 실패합니다.

| 검사 | 관찰된 실패 형태 | 기록 |
| --- | --- | --- |
| `application::context_menu_tests::실제_추가_경계_보조창_메뉴는_클립보드와_터미널의_소유_pane을_보존한다` | `body_view(..).unwrap()` 실패. 비동기로 끝난 다른 layout 변경이 같은 revision을 먼저 차지해 검사의 layout이 건너뛰어짐(`ShellController::apply_layouts`의 `present.revision >= layout.revision`) | seams 문서 3절 끝 |
| `application::view_state_tests::실제_일괄_닫힘은_주_보조_영역의_독립_위치와_접기를_보존한다` | 같은 유형 | seams 3절, QA `2026-10-27-native-auxiliary-window.md` |
| `application::file_loading_tests::실제_거절_캐시는_보조_mirror의_수명_파일_변경과_프로젝트_종료를_따른다` | 보조 창 layout을 만든 직후 단언하는 경쟁 | seams 3절, QA `2026-10-27-native-auxiliary-window-moves.md` |
| `application::project_rail_tests::실제_app_그룹은_새그룹_생성_그룹추가_수정_열기결과_toast와_삭제확인을_runtime에_반영한다` | `native host command queue is full` toast 또는 결과 toast 기한 초과 | PROCESS 21줄(E2 관찰), QA `2026-10-27-native-window-position-mirror-prune.md` |
| `application::startup_tests::actual_app의_아웃라인_workspace_심볼과_구문_접기는_현재_pane과_reveal을_소비한다` | 마지막 종료 대기(`phase=navigation-exit`, tasks=2) 시간 초과 | QA `2026-10-27-native-error-boundary-panels.md` |
| `application::startup_tests::actual_app_constructor는_bundle_startup_settings_file_save와_정상_exit_owner를_연결한다`, `application::view_state_tests::실제_정상과_직접_종료는_마지막_본문_선택과_scroll을_layout에_저장한다` | 2026-10-27에는 단독 재실행에서도 종료 대기 시간 초과(tasks=2). 원인은 abort할 수 없는 blocking task(실제 프로세스 목록을 읽는 agent probe 등)의 환경 의존 시간으로 추정 | PROCESS 19줄(단계 D), QA `2026-10-27-native-auxiliary-window.md` |
| `application::file_drop_tests::실제_app은_놓은_폴더를_프로젝트로_열고_뒤따르는_파일은_그_프로젝트의_preview_탭으로_연다` | 단계 시간 초과(`phase=folder then file`) | QA `2026-10-28-native-explorer-views-theme-providers.md` |

위 검사 이름은 모두 2026-11-04 메인 소스에 있습니다(`rg -n "fn <이름>" native/taide-native-app/src`로 확인). 2026-11-03 게이트 결과: wave 1 병합 때 앱 lib 5건 실패 중 4건이 직렬 재실행에서 통과(흔들림), 1건이 `editor_fonts`. settings 병합 때 흔들림 2건 직렬 통과, `editor_fonts` 기존 실패(`docs/PROCESS.md` 30·32줄). 어느 검사였는지 이름은 PROCESS에 적혀 있지 않습니다.

### 12.3 실행하면 안 되는 검사: 실제 휴지통 사용

공용 `crates/taide-file/src/service.rs` `delete_entry`는 macOS에서 `trash::TrashContext`에 `DeleteMethod::NsFileManager`를 설정해 사용자 휴지통으로 옮깁니다(190~195줄, `trash 5.2.6`). 아래 세 검사는 이 경로를 실제로 탑니다. QA 문서의 "보호 Trash 3제외/3 filtered/3건 skip"이 이 셋입니다(예: `docs/quality-assurance/2026-10-16-native-json-schema-app.md` 58줄, `2026-10-10-native-batch16-document-symbols.md` 37줄. 실제 `--skip` 인자는 QA 문서들에 같은 세 이름으로 3회 이상 기록돼 있습니다).

| 검사 | 위치 | 휴지통 경로 |
| --- | --- | --- |
| `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다` | 통합 대상 `native/taide-native-app/tests/explorer-delete.rs` 401줄 | `LspBridge::delete_entry`로 합성 파일 `delete-synthetic.txt` 삭제 |
| `실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다` | 통합 대상 `native/taide-native-app/tests/lsp-workspace-worker.rs` 387줄 | workspace delete 적용 |
| `remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다` | 앱 lib, `native/taide-native-app/src/remote-files-tests.rs` 393줄(`remote-files.rs` 234줄 `mod tests;`) | 원격 `file_delete` 명령 |

- 같은 `tests/explorer-delete.rs`의 나머지 세 검사(`탐색기_삭제확인은_context와_정확한_단축키_취소초점_backdrop_escape_확정을_보존한다`, `삭제_ack는_늦은_revision을_폐기하지않고_확정한_dirty_pinned_tab의_기록을_정리한다`, `삭제후_공유초안_save_as는_더좁은_root가_있어도_해당_프로젝트의_미러를_복구한다`)는 대화상자·임시 디렉터리·`std::fs::remove_file`만 쓰므로 휴지통에 닿지 않습니다. 2026-11-03 판 이 문서의 `--skip` 목록에는 이 중 두 개가 잘못 들어 있었고, 실제 휴지통 검사 두 개가 빠져 있었습니다.
- lane workflow와 사양 결정은 `tests/explorer-delete.rs` 실행과 "추적 안 된 파일 discard 실검사"를 금지합니다(lane workflow `RULES` 50줄, `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md` 보류 표 27줄). Git의 추적 안 된 파일 discard도 `delete_entry`처럼 휴지통을 쓰는지 새 검사를 만들 때 확인합니다(`crates/taide-git/Cargo.toml`에 `trash = "5"` 의존이 있습니다).
- 과거에 작업자가 금지된 `tests/explorer-delete.rs`를 2회 실행해 합성 파일이 사용자 휴지통에 들어갔을 수 있는 사고가 있었습니다(`docs/PROCESS.md` 19줄).

### 12.4 판별 순서

1. 실패한 검사 이름과 메시지를 로그에서 얻습니다(`grep -E ' \.\.\. FAILED$' <로그>`).
2. 12.1절 표에 있으면 같은 값으로 실패하는지 확인하고 기존 실패로 기록합니다.
3. 아니면 그 검사만 단독으로 한 번 다시 돌립니다: `cargo test --quiet --manifest-path <앱 Cargo.toml> --lib --locked --offline --target-dir <target> -- --exact <모듈 경로::검사 이름>`. 통과하면 부하 흔들림으로 기록하고 실패 메시지(큐 포화·시한·phase 이름)를 함께 적습니다.
4. 단독에서도 실패하면, 바꾸지 않은 커밋에서도 실패하는지 확인해야 회귀와 구분됩니다. 메인 작업 트리에서 `git stash`를 쓰는 판별은 다른 작업자가 돌지 않을 때만 했습니다(transition 운영 문서 4절). lane에서는 worktree가 분리돼 있으므로 기준 커밋의 다른 worktree에서 같은 검사를 돌리는 편이 안전합니다(작업자는 git 상태 변경 금지이므로 이 판별은 메인이 합니다).
5. 같은 가정으로 세 번 연속 실패하면 멈추고 원인 가설과 함께 보고합니다(lane workflow `RULES`).
6. 검사 기대값을 바꿔 통과시키지 않습니다. 실패를 재현하려고 구현을 임시로 끄거나 되돌리는 편집도 하지 않습니다(transition 운영 문서 3절).

## 13. CI와 기타

- `.github/workflows/ci.yml`은 `dev`·`main` push와 pull request에서만 돕니다(5~7줄). frontend job은 ubuntu에서 bun 1.3.14, Rust job은 `macos-latest`에서 `dtolnay/rust-toolchain@stable`입니다. native crate는 루트 workspace 밖이라 CI에서 검사되지 않습니다. native 검증은 위 명령으로 로컬에서 합니다. native를 CI에 넣는 것은 배치 31 구조 정리 범위입니다(`docs/HANDOFF.md` 3.3절).
- `native/taide-native-app/src/keymap.rs`, `keybinding-editor.rs`, `keybinding-search.rs`, `tooltips.rs`, `problems-icons.rs`는 `include!("../../taide-native-ui/src/<파일>.rs")`로 UI crate 소스를 앱 crate 안에서 다시 컴파일합니다(대부분 `cfg(test)` 블록 안의 `compatibility` 모듈). 그래서 같은 UI 코드가 두 컴파일 단위에서 검사되고, UI 파일을 바꾸면 앱 crate 검사도 깨질 수 있습니다(transition 운영 문서 6절 마지막 항목). 이 복제 제거는 배치 31 구조 정리 범위입니다.
