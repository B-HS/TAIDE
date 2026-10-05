# M8 native shell 격리 실험

## 대상과 범위

대상은 `experiments/native-shell-spike`의 egui·iced executable과 공유 `ShellFixture`입니다. 제품 root workspace와 기존 Tauri 앱은 변경하지 않습니다. 10,000개 합성 파일명, 식별자가 고정된 50개 탭, 두 native 창, 테마·3개 locale·입력란을 비교합니다. 파일 dialog와 drop은 metadata만 관찰하며 파일 내용을 읽거나 실제 프로젝트·설정을 저장하지 않습니다.

macOS Apple Silicon, rustc/cargo 1.98.1에서 확인했습니다. 모든 결과는 기술 실험에 한정하며 전체 shell 기능 동등성 통과가 아닙니다. egui는 multiline, iced는 single-line 입력란이므로 editor 동등성 비교로 사용할 수 없습니다.

최신 정적 상태: 아래 후속 품질 정리에서 default egui feature의 모든 target strict clippy가 통과했습니다. 앞선 probe/context 구현 단계의 clippy 실패는 당시 이력이며 iced/all-feature·MSRV·배포·실제 GUI hard gate 통과로 확장하지 않습니다. 사용자 실기 bundle에는 후속 context/keyboard 코드를 반영하지 않았습니다.

## 후보와 공식 근거

| 후보        | 고정 버전 | MSRV              | 라이선스          | 현재 판정                                          |
| ----------- | --------- | ----------------- | ----------------- | -------------------------------------------------- |
| egui/eframe | 0.36.2    | 1.95              | MIT OR Apache-2.0 | 보조 창 AX 초기화의 국소 수정 후 검토 계속         |
| iced        | 0.14.0    | 1.88              | MIT               | 현재 구성의 보조 창 AX 컨트롤 부재, hard gate 실패 |
| muda        | 0.21.0    | 1.90              | MIT OR Apache-2.0 | egui의 실제 macOS menu·Window menu에서 확인        |
| rfd         | 0.17.2    | metadata에 미표시 | MIT               | egui의 실제 OS file dialog에서 확인                |

버전·MSRV·license는 해당 버전의 Cargo metadata와 공식 소스를 확인했습니다. 제품의 기존 MSRV 1.89는 변경하지 않았습니다. 실험만 1.95를 요구합니다. 후보 확정 시 MSRV와 유지보수 부담을 다시 결정해야 합니다.

공식 API: [eframe App](https://docs.rs/eframe/0.36.2/eframe/trait.App.html), [egui Context](https://docs.rs/egui/0.36.2/egui/struct.Context.html), [iced 0.14 multi-window 예제](https://github.com/iced-rs/iced/blob/0.14.0/examples/multi_window/src/main.rs), [iced 0.14 Cargo](https://github.com/iced-rs/iced/blob/0.14.0/Cargo.toml), [rfd FileDialog](https://docs.rs/rfd/0.17.2/rfd/struct.FileDialog.html).

iced 빌드에서 `block 0.1.6`의 future-incompatibility 경고가 발생했습니다. 후보 재검토 시 상세 리포트·업스트림 해소 상태를 확인해야 합니다. 경고를 숨기지 않았습니다.

## 실제 결과

- [x] 공유 fixture의 탭 이동·주 창 선택 복구와 보조 창 종료 시 전체 탭 반환: 단위 검사 2건 통과. 같은 성공 상태를 재사용합니다.
- [x] egui와 iced debug executable 빌드 성공. iced의 `advanced` feature 누락으로 발생한 컴파일 오류는 공식 feature와 입력 이벤트 타입 경로를 확인해 수정했습니다.
- [x] egui 주 창의 checkbox·button·50개 탭·가상 tree 항목·labelled multiline 입력란이 AX에 노출됩니다. Dark theme 전환과 입력란에서 Tab으로 Copy 버튼에 이동하는 것을 확인했습니다.
- [x] egui 실제 Window menu에서 주 창과 보조 창을 별도 native 창으로 선택했습니다. 보조 창에서 컨트롤이 없던 원본 실패를 재현하고, 수정 후 tree 항목·checkbox·button·labelled 입력란의 AX 노출을 확인했습니다. VoiceOver 낭독 통과를 의미하지 않습니다.
- [x] egui 입력란에 합성 `한글 日本語 中文`을 붙여넣고 AX의 실제 문자열을 확인했습니다. 당시 IME preedit/commit은 0이므로 실제 IME 조합 통과로 계산하지 않습니다.
- [x] egui의 native file dialog에서 실험 `Cargo.toml`을 선택했습니다. dialogs 1과 “file was not read” 상태를 확인했습니다. macOS Go To 입력은 native AX setValue로 지정했습니다. clipboard paste 시도는 timeout이었으며 dialog의 클립보드 검증은 미완료입니다.
- [x] 두 `.app`의 plist lint·ad-hoc 서명·strict signature 검사가 통과했습니다. Developer ID·공증·fresh install·upgrade·배포 통과가 아닙니다.
- [ ] iced 주 창과 보조 창의 실제 AX 상태는 standard window·close/fullscreen/minimize·제목뿐이며 내부 tree·버튼·입력란이 없습니다. 보조 창을 닫은 뒤 주 창에서도 확인했습니다. stable 0.14 구성은 현재 hard gate 실패이며 VoiceOver 낭독을 추정하지 않습니다.
- [ ] egui 실제 창 사이 drag/drop·native context menu·외부 file drop·notification·locale 전환·tree scroll·GPU device loss/software fallback·창 close/recreate와 장기 lifecycle은 남았습니다. egui painted context menu만으로 native context menu 조건을 통과 처리하지 않습니다.
- [ ] 실제 CJK 조합·취소·후보 창 위치와 VoiceOver 낭독·탐색은 사용자 실기 결과 대기입니다.

상태 UI의 UI ms는 UI 작성 시간이며 input-to-paint latency가 아닙니다. 두 창 통계는 창별로 분리하고 cumulative frame 번호로 multipass 중복 이벤트 집계를 방지합니다. 변경 빌드·bundle 반영 후 최신 주 창이 실행 중이며 사용자가 실기할 수 있습니다. 성능 기준선으로 사용하지 않습니다.

## eframe 보조 창 수정과 회귀

`vendor/eframe`는 0.36.2의 MIT 소스이며 실험 package에만 path patch로 연결합니다. 변경은 `src/native/wgpu_integration.rs`에 한정합니다. 원본은 root에만 AccessKit을 초기화합니다. 이벤트 루프 proxy를 child 창 생성 경로에 전달하고 숨겨 생성 → adapter 초기화 → 요청한 visibility 복구로 순서를 정정했습니다.

최초 수정은 이미 표시된 창에 adapter를 초기화해 `accesskit_winit 0.32.2:198`의 “must be created before the window is shown” panic과 exit 134를 발생시켰습니다. 정확한 stderr를 확인하고 창 생성 순서를 수정한 뒤 빌드 exit 0·실제 보조 창 AX 컨트롤 노출을 확인했습니다. 접근성 비활성화·가짜 노드·panic 억제를 사용하지 않습니다. glow backend는 수정 대상이 아니며 제품 채택·업스트림 병합을 주장하지 않습니다.

## Surface·device·software fallback와 외부 drop source 조사

대상은 고정 egui-wgpu 0.36.2의 `src/{lib,setup,winit}.rs`, egui-winit 0.36.2의 file event 경로, vendor eframe의 renderer/native 경로와 독립 `src/gpu-adapter-probe.rs`입니다. 실험 manifest에 이미 eframe이 사용하는 MIT pollster 1.0.1을 optional direct dependency로 재사용하고 독립 bin을 추가했습니다. 독립 lock의 실험 package dependency 목록만 확장하며 새로운 package·제품 root lock·제품 의존성·MSRV는 추가/변경하지 않았습니다. 새 async executor를 직접 만들지 않습니다.

- [WgpuConfiguration](https://docs.rs/egui-wgpu/0.36.2/egui_wgpu/struct.WgpuConfiguration.html)의 default surface callback은 Outdated를 재설정하고 Lost를 surface 재생성으로 처리합니다. `winit.rs::recreate_surface`는 같은 Instance·기존 render state를 유지하며 Surface를 다시 만들고 설정합니다. GPU Device·Queue·renderer·textures를 새로 만드는 device-loss 복구 경로는 아닙니다. 해당 source에서 별도 device-lost callback·전체 device 재생성은 확인하지 못했으며 실제 device-loss 시험은 수행하지 않았습니다.
- 현재 eframe native Renderer는 Wgpu/Glow이며 이 실험은 wgpu만 빌드합니다. default adapter 요청은 force_fallback_adapter false이고 native_adapter_selector는 이미 제공된 adapter에서 선택합니다. selector가 CPU renderer를 설치하거나 없던 software adapter를 만드는 것은 아닙니다. 현재 Metal-only 구성에 native CPU paint/present 경로는 구현하지 않았습니다. 이를 모든 egui host에서 software renderer 구현이 불가능하다는 주장으로 확대하지 않습니다.
- egui-winit의 HoveredFile/Cancelled/DroppedFile은 native WindowEvent에서 hovered/drop input으로 전달됩니다. `main.rs::show_shell`은 현재 창의 raw drop 수를 누적하지만 파일 내용을 읽지 않습니다. 이 source 경로 존재는 Finder의 실제 drag, 보조 창 routing, 보안 root admission, 창 사이 tab 이동의 drop 실기를 대신하지 않습니다. 실행 중인 사용자 실기 앱은 조작하지 않았습니다.

[wgpu Instance](https://docs.rs/wgpu/30.0.1/wgpu/struct.Instance.html#method.enumerate_adapters)와 고정 RequestAdapterOptions source를 확인했습니다. probe는 compiled backend로 headless Instance를 만들며 adapter 목록과 `force_fallback_adapter: true` 요청 결과만 관측합니다. Window·Surface·Device를 만들거나 destroy/reset하지 않습니다. 환경 변수 override를 읽는 `from_env/with_env` 대신 `InstanceDescriptor::new_without_display_handle`를 사용하며 사용자 파일·설정에 접근하지 않습니다. eframe이 별도로 환경 override를 적용할 수 있으므로 이 probe를 실행 중인 앱의 실제 adapter 설정과 동일하다고 단정하지 않습니다.

```sh
cargo build --manifest-path experiments/native-shell-spike/Cargo.toml --bin taide-gpu-adapter-probe --locked --offline
experiments/native-shell-spike/target/debug/taide-gpu-adapter-probe
cargo clippy --manifest-path experiments/native-shell-spike/Cargo.toml --bin taide-gpu-adapter-probe --locked --offline -- -D warnings
rustfmt --edition 2024 --check experiments/native-shell-spike/src/gpu-adapter-probe.rs
git diff --check
```

최초 build는 InstanceDescriptor에 Default 구현이 없어 E0277로 실패했습니다. 설치된 공식 source의 `new_without_display_handle`로 정정하고 build exit 0(0.47초)을 확인했습니다. 먼저 실행한 sandbox probe는 compiled METAL·adapter 0·CPU 0·fallback unavailable·exit 0이었지만 기존 GUI 실행 증거와 달라 실제 hardware 부재로 판정하지 않았습니다. 해당 창 없는 조회만 escalation한 보완 실행은 다음 값을 출력하고 exit 0으로 끝났습니다. 같은 환경의 성공 검사를 반복한 것이 아니며 실제 실행은 환경이 다른 2회입니다.

```text
compiled_backends=Backends(METAL)
adapter_count=1
adapter_name=Apple M5 Max backend=Metal device_type=IntegratedGpu
cpu_adapter_count=0
fallback_available=false
window_surface_tested=false device_loss_tested=false renderer_recovery_tested=false
```

대상 strict clippy는 dependency로 검사한 기존 `src/lib.rs:87`의 manual_slice_fill 경고로 exit 101입니다. 이번 probe가 변경하지 않은 fixture이며 검사 억제·무관한 refactor를 추가하지 않았습니다. build 성공을 strict clippy 통과로 표기하지 않습니다. 새 probe format·diff는 exit 0입니다. 앱 main/vendor 소스·bundle은 이번 조사에서 바꾸거나 재빌드/실행하지 않았으므로 사용자 실기 환경은 유지됩니다.

현재 구성의 software fallback 제공 경로가 없어 GPU fallback gate는 미충족입니다. 실제 Lost surface 재생성·device 복구와 소프트웨어 화면 표시·재전환·데이터 보존·접근성/IME 및 latency를 구현·검증해야 합니다. 외부 drop도 실제 실기가 필요합니다. 후보 채택/CPU host 또는 custom toolkit 비용을 별도 승인 없이 제품 구현에 고정하지 않으며 이 조사만으로 N1 또는 M8을 완료 처리하지 않습니다.

### CPU backend 보완 후보와 승인 경계

설치된 epaint 0.36.2의 `src/text/font.rs::allocate_glyph_uncached`는 vello_cpu 0.1.0으로 glyph outline을 작은 Pixmap에 rasterize해 texture atlas에 넣습니다. 이 dependency가 있다는 사실은 clipped mesh·이미지·전체 UI를 CPU로 합성하거나 native 창에 표시하는 fallback 구현을 의미하지 않습니다. [Vello 공식 자료](https://github.com/linebender/vello)는 별도 CPU renderer를 제공하지만 eframe의 device-loss·software backend 전환을 제공한다고 명시하지 않습니다.

[egui_software_backend 공식 manifest](https://raw.githubusercontent.com/DGriffin91/egui_software_backend/main/Cargo.toml)와 [버전표](https://github.com/DGriffin91/egui_software_backend)를 조회했습니다. 0.0.3은 MIT OR Apache-2.0이며 egui/egui-winit 0.34와 softbuffer 0.4를 요구합니다. 현재 egui/eframe 0.36.2의 mesh·texture 타입과 직접 연결하는 호환 버전이 아니며 기존 실험을 내려 성공 결과를 그대로 재사용할 수 없습니다. 최신 저장소 source 조회는 버전 고정·build·API 호환 검증 또는 공급망 검토를 대신하지 않습니다. 이 package를 내려받거나 lock/product dependency에 추가하지 않았습니다.

[softbuffer 공식 자료](https://github.com/rust-windowing/softbuffer)는 CPU pixel buffer의 창 표시를 제공하고 drawing primitive는 제공하지 않는다고 구분합니다. 따라서 별도 CPU painter뿐 아니라 texture 수명·clip·alpha·DPI·다중 창의 표시 host 연결이 필요합니다. [egui_backend_selector 공식 자료](https://github.com/AlexanderSchuetz97/egui_backend_selector)는 macOS에서 항상 eframe을 선택하며 egui 0.34 계열입니다. 현재 macOS의 device loss 후 상태 보존·CPU 전환 경로를 이 selector가 해결한다고 추정하지 않습니다.

기존 GPU 조회·단위 검사·실기 성공은 재실행하지 않았습니다. 이 조사는 현재 후보의 보완 비용을 드러내는 근거이며 모든 egui host의 구현 불가능 판정은 아닙니다. CPU backend의 0.36 이식과 별도 host 연결 실험, 또는 custom toolkit의 장기 유지보수 비용 중 다음 방향을 사용자에게 한 번 묶어 확인합니다. 승인 전 후보 version downgrade·새 CPU dependency·대규모 host 구현·제품 채택을 하지 않습니다. 사용자 IME/VoiceOver 결과 질문은 반복하지 않으며 기존 bundle·열린 앱·시스템 설정을 유지합니다.

## Native context menu 연결 선행

대상은 실험의 `src/native-context-menu.rs`, egui main의 탭 우클릭 경계, 공유 fixture의 move_tab_from/window_title와 실험 Cargo manifest/lock입니다. 제품 root dependency·lock·MSRV·eframe vendor는 바꾸지 않았습니다. 현재 사용자가 실기하는 `.app`의 executable·번들은 교체하거나 재실행하지 않았습니다. 이번 검사는 compile/unit까지만이며 해당 앱에 새 메뉴가 이미 표시된다는 주장이 아닙니다.

기존 egui response.context_menu는 화면 내부 painted 메뉴였습니다. macOS에서는 muda NSMenu popup을 연결하고 다른 플랫폼의 기존 painted 경로는 유지합니다. 메뉴의 단일 동작은 현재 locale의 합성 탭을 다른 창으로 이동하는 것뿐이며 파일 읽기·쓰기·설정·외부 명령을 실행하지 않습니다. 팝업이 선택으로 종료될 때만 동작하고 취소는 탭을 바꾸지 않는 코드 경계입니다. 실제 취소 입력 검사는 미완료입니다.

Frame raw handle을 무조건 재사용하면 보조 창에서도 주 창의 NSView를 가리킬 수 있어 사용하지 않았습니다. AppKit에서 같은 프로세스의 key window를 얻고 현재 pane의 고정 title과 대조합니다. 불일치·창/뷰 부재는 오류로 끝냅니다. MainThreadMarker로 메인 스레드를 확인하고 window와 contentView의 Retained 소유자를 호출 scope에 유지합니다. view.window의 windowNumber가 일치하는지도 확인한 뒤 해당 NSView 포인터를 동기 muda 호출에만 전달하고 저장하거나 worker에 전송하지 않습니다. title 대조는 두 고정 실험 창의 경계이며 제품의 안정적 native window identity나 다른 앱에 대한 권한 판정으로 사용하지 않습니다.

unsafe 호출의 전제는 유효한 NSView와 설치된 window입니다. 설치된 muda 0.21.0 ContextMenu와 macOS show_context_menu source, objc2 0.6.4 MainThreadMarker::new, objc2-app-kit 0.3.2의 sharedApplication/keyWindow/contentView/title/windowNumber 정의를 확인했습니다. 공개 docs.rs 조회는 접근 오류였으므로 API 판단에는 기존 설치 package의 공식 source를 사용했습니다. 현재 cursor 위치는 muda의 None position 경로를 사용해 OS가 좌표/화면 경계를 처리하며 keyboard anchor·배율 전환의 실제 검사는 남습니다. NSMenu의 modal tracking 중 Rust UI 재진입 여부도 실제 GUI 검증 전까지 미확정입니다.

이미 eframe/muda가 사용하는 objc2 0.6.4·objc2-app-kit 0.3.2를 macOS egui feature의 직접 의존성으로 재사용했습니다. 이유는 보조 창의 실제 NSView를 안전하게 소유하는 OS API가 필요하기 때문입니다. 새 registry package·version downgrade·제품 GUI 선정은 없으며 실험 lock의 root package dependency edge만 추가했습니다. AppKit feature는 std·NSApplication·NSResponder·NSView·NSWindow로 한정합니다.

```sh
cargo check --manifest-path experiments/native-shell-spike/Cargo.toml --bin taide-egui-spike --offline
cargo test --manifest-path experiments/native-shell-spike/Cargo.toml --lib context_action --locked --offline -- --nocapture
```

egui bin compile exit 0(0.58초)과 신규 context source guard unit 1건 0.00초가 통과했습니다. unit은 잘못된 pane·범위 밖 tab·이미 이동된 tab의 중복 동작 거절, 양방향 이동 후 선택 복구·총수 보존과 세 locale label을 확인합니다. 기존 탭 이동/창 종료 성공 2건은 같은 구현에 재사용했습니다. native-shell strict clippy는 이미 기록한 기존 return_auxiliary_tabs의 manual_slice_fill 경고가 있어 이번 성공으로 주장하지 않으며 suppression·무관한 helper 재작성도 하지 않았습니다.

- [x] macOS native popup source 연결·실험 compile·합성 탭 source guard
- [ ] 실제 주/보조 창의 메뉴 표시·선택·Escape/외부 click 취소·잘못된 key window 거절
- [ ] 키보드 context invocation·focus 반환·VoiceOver·DPI/화면 경계·modal 재진입/lifecycle

사용자의 기존 IME·VoiceOver 실기가 끝난 뒤 별도의 변경 bundle에 반영하고 위 실제 gate를 한 번 확인해야 합니다. 이 선행 구현은 native context menu hard gate 또는 N1/M8 완료가 아닙니다. GPU 보완 방향 선택 질문과 사용자 실기 결과 질문은 반복하지 않았습니다.

### Keyboard invocation·포커스 수명 보완

위 native popup에 focused tab의 정확한 Shift+F10 non-repeat press 경로를 추가했습니다. 메뉴를 띄우기 위한 사용자 OS key setting 변경이나 global shortcut 등록은 하지 않습니다. egui의 consume_key는 extra Alt/Shift를 논리적으로 무시하고 repeat도 포함하므로 이 계약에는 그대로 쓰지 않았습니다. 설치된 InputState/Response/Context 공식 source를 확인해 해당 focused widget의 정확한 modifiers만 소비하고 repeat press는 재호출 없이 소비합니다. 다른 키·수정 키·비포커스 event는 유지합니다. 현재 pane의 cumulative frame 번호로 multipass popup 중복을 막는 코드도 연결했으며 실제 multipass/OS 동작은 별도 gate입니다.

keyboard popup은 현재 tab의 visible clip과 viewport rect 안으로 anchor를 clamp하고 egui zoom만 곱해 native logical 위치로 변환합니다. backing scale까지 다시 곱해 이중 배율을 적용하지 않습니다. finite rect/clip·양수 zoom을 먼저 검사하고 실제 screen fit은 기존 muda 경로가 처리합니다. 이 계산 통과만으로 Retina/zoom·다중 화면의 실제 위치를 검증했다고 주장하지 않습니다.

취소는 원래 tab의 egui focus만 복구하고 OS Focus 명령을 보내 다른 창/앱에서 focus를 빼앗지 않습니다. 탭 이동은 source guard 뒤 실제 destination을 반환하고 해당 tab을 destination viewport에서 렌더할 때 widget focus·scroll-to-visible·OS viewport Focus 명령을 소비합니다. tab widget ID는 pane/tab identity로 구분합니다. destination 보조 창이 닫히면 pending focus를 주 창으로 반환하며 실제 상태가 바뀐 경우에만 repaint를 요청해 닫힌 상태의 상시 repaint를 만들지 않습니다. OS 창 focus·scroll 반영과 close callback ordering은 실제 GUI에서 아직 검증하지 않았습니다.

초기 UI 연결에서 input_mut closure 안에 Response::has_focus를 호출하는 중첩 context 잠금 위험을 source로 확인했습니다. focus를 먼저 읽고 input_mut 안에는 입력 소비만 두는 전용 경계를 실제 UI와 headless 검사에서 공유합니다. deadlock을 일부러 실행하거나 실제 앱에서 재현했다는 주장은 하지 않습니다.

```sh
cargo test --manifest-path experiments/native-shell-spike/Cargo.toml --bin taide-egui-spike keyboard_context --locked --offline -- --nocapture
cargo test --manifest-path experiments/native-shell-spike/Cargo.toml --lib menu_completion --locked --offline -- --nocapture
cargo test --manifest-path experiments/native-shell-spike/Cargo.toml --bin taide-egui-spike keyboard_ui_boundary --locked --offline -- --nocapture
cargo check --manifest-path experiments/native-shell-spike/Cargo.toml --bin taide-egui-spike --locked --offline
```

focused/exact/repeat 입력·정상/clipped logical anchor·invalid 좌표 unit 1건과 취소/이동/stale source의 논리적 focus unit 1건은 각각 0.00초 통과했습니다. egui::Context::run_ui로 실제 Response focus를 읽고 정확한 key event가 해당 tab에서만 한 번 소비되는 headless UI 검사 1건은 0.02초 통과했습니다. 해당 fixture의 최초 실행은 renderer 없이 생성한 texture delta를 그냥 Drop해 공식 debug assertion으로 실패했습니다(0.01초). headless 입력 검사에서만 texture delta를 명시적으로 clear하는 공식 경계를 적용한 뒤 실패한 검사만 한 번 재실행했으며 제품 renderer의 texture 처리나 warning/assertion을 끄지 않았습니다. 이 검사는 픽셀 표시/렌더 품질 증거가 아닙니다.

두 test-name filter를 한 Cargo 호출에 넣은 최초 명령은 CLI 인자 오류로 실행되지 않았습니다. 별도 정확한 명령으로 위 세 결과를 얻었고 같은 성공 검사는 반복하지 않았습니다. 마지막 close/repaint source 수정 뒤 target bin compile만 추가했습니다. 기존 strict clippy warning의 미통과 상태는 유지합니다. UI ms에 native modal menu tracking 대기가 포함될 수 있어 성능 기준선으로 사용하지 않습니다.

실기 중인 bundle·standalone GUI executable·OS 설정·사용자 데이터는 변경하지 않았습니다. actual mouse/Shift+F10 popup·Escape/외부 click 취소·OS destination focus/close·VoiceOver·IME·modal 재진입 검사는 여전히 미완료입니다. 기존 GUI 보완 승인·사용자 실기 질문을 반복하지 않습니다.

### Shell 실험 strict 정적 품질 후속

이 M8에서 만든 공유 fixture의 return_auxiliary_tabs는 수동 loop로 모든 소유자를 Main으로 바꿨습니다. 이후 keyboard close 경계가 이 helper를 직접 사용하고 기존 warning이 새 native 코드의 strict 검사를 막으므로 [slice::fill 공식 계약](https://doc.rust-lang.org/std/primitive.slice.html#method.fill)에 맞춰 한 줄의 fill로 정리했습니다. 탭 수·ID·active selection 복구와 할당/저장 정책은 바꾸지 않았습니다. root 제품 helper·dependency·GUI bundle은 변경하지 않았습니다.

첫 후속 clippy는 native main의 close_requested→close_auxiliary 중첩 if를 collapsible_if로 거절했습니다. 같은 short-circuit 조건을 한 guard로 합쳐 수정했으며 lint suppression은 없습니다.

```sh
cargo test --manifest-path experiments/native-shell-spike/Cargo.toml --lib 보조_창을_닫으면 --locked --offline -- --nocapture
cargo clippy --manifest-path experiments/native-shell-spike/Cargo.toml --all-targets --locked --offline -- -D warnings
```

변경된 fill 경로의 기존 전체 50개 탭 반환 검사 1건은 0.00초, 수정 뒤 default egui feature의 모든 target strict clippy는 exit 0(0.18초)입니다. 신규 keyboard/context/좌표 성공 검사는 구현이 같아 재사용했고 다시 실행하지 않았습니다. iced feature를 활성화한 검사·새 GUI 실행·제품 채택·native 메뉴 표시/낭독·GPU recovery/CPU fallback 검증은 수행하지 않았습니다.

## 진행을 막는 승인·실기 게이트

현재 source·로드맵·parity §11은 framework 후보 gate 통과와 별도 dependency 승인을 제품 대규모 구현의 전제로 유지합니다. CPU fallback 보완 방향과 실제 IME/VoiceOver 결과의 기존 질문에는 아직 답이 없습니다. 같은 조건은 여러 goal 재개에서도 유지됐으며 그동안 가능한 독립 LSP 소유/종료·메모리 경계와 context/keyboard 코드 준비를 진행했습니다.

다음 핵심 작업은 egui 0.36 CPU backend/표시 host 보완 실험 또는 custom toolkit 비용 검토의 방향 선택입니다. 임의 version downgrade·추가 CPU package·큰 GUI fork·제품 채택으로 대신하지 않습니다. 사용자가 현재 bundle에서 실기 중이므로 변경 bundle을 덮어쓰거나 별도 GUI를 전면에 띄워 입력·focus를 방해하지 않습니다. 사용자가 보고하기로 한 IME/VoiceOver 결과를 paste/AX 구조 또는 headless event로 대신하지 않습니다.

이 결정과 실기 결과를 받은 뒤 미충족 N1 gate를 계속하고, 전체 승인 뒤 N2~N8 제품 구현·동등성·배포·cutover를 진행해야 합니다. 현재 M8은 미완료이며 완료 후 commit/push 조건도 아직 성립하지 않았습니다. 기존 질문은 재전송하지 않습니다.

## 사용자 실기 계약과 순서

사용자는 2026-09-30에 “시스템 설정은 유지하고 해당 실기 검증은 제가 수행”을 선택했습니다. 에이전트는 입력기·VoiceOver를 변경하지 않습니다. 사용자가 선택한 입력기와 설정에서 다음 항목을 한 번씩 검사합니다.

1. `TAIDE M8 Egui Spike`의 주 창에서 `IME input probe`를 클릭합니다. 한글·일본어·중국어 중 사용 가능한 입력기로 합성 텍스트를 조합·확정합니다. 미확정 조합을 Escape로 취소해 확정 문자열에 남지 않는지 확인합니다. 각 언어의 조합 표시·확정·취소·후보 창 위치와 하단 preedit/commit 변화를 기록합니다. 지원 입력기가 없으면 해당 언어는 미검증으로 남깁니다.
2. Window menu → `TAIDE M8 auxiliary`에서도 같은 조합·확정·취소를 한 번 확인합니다. 공유 입력 문자열이 일치하고 후보 창이 현재 입력란 근처인지 확인합니다.
3. 사용자가 VoiceOver를 켜기로 한 경우 주 창과 보조 창의 입력란 이름, 체크박스 상태, 파일 버튼, tree 항목을 읽고 키보드로 탐색·실행 가능한지 확인합니다. 단순히 앱 제목만 읽히면 실패입니다. 사용자가 직접 원래 설정으로 복원합니다.
4. 결과는 `주 창 IME / 보조 창 IME / VoiceOver / 미검증 언어`로 보고합니다. 이상이 있으면 입력기·언어·조합·관찰 동작을 함께 기록합니다.

이 실기는 N1 일부입니다. 두 결과가 통과해도 editor·terminal 후보, 나머지 hard gate와 N2~N8이 남으므로 M8 전체 완료가 아닙니다.
