# eframe 0.36.2 실험용 수정

이 디렉터리는 crates.io의 eframe 0.36.2 소스입니다. 상위 저작권과 주석은 수정하지 않고 보존했으며 이 복사본은 MIT 라이선스로 사용합니다. 제품 workspace가 아니라 격리 기술 실험에만 연결합니다.

대상은 `src/native/wgpu_integration.rs`입니다. 원본은 root viewport에만 `egui_winit::State::init_accesskit`을 호출하고 추가 창은 호출하지 않습니다. macOS에서 실제 보조 창을 선택하면 화면은 나타나지만 접근성 트리에 내부 컨트롤이 없는 현상을 재현했습니다.

수정은 이벤트 루프 proxy를 공유 상태에 보관하고 신규 viewport를 숨긴 상태로 생성해 State의 AccessKit adapter를 초기화한 뒤 요청한 visible 값을 복구하는 것입니다. 이미 표시된 창에 adapter를 붙이면 accesskit_winit 0.32.2가 panic하므로 초기화 순서는 필수입니다. 접근성 검사를 끄거나 가짜 노드를 주입하지 않습니다.

수정 후 격리 앱의 실제 보조 창에서 checkbox·button·tree 항목·labelled text entry가 macOS AX에 노출됐습니다. 변경 대상은 wgpu backend이며 glow backend는 포함하지 않습니다. 아직 제품 의존성 채택·VoiceOver 실기·전체 hard gate 통과를 의미하지 않습니다. 검증 결과와 유지보수 판단은 `docs/quality-assurance/2026-09-30-m8-native-shell-spike.md`에서 추적합니다.

## 2026-10-02 native 입력 재처리 경계

`src/epi.rs`에 기본 구현이 기존 `raw_input_hook`으로 위임하는 `raw_input_hook_with_replay`를 추가했습니다. `src/native/epi_integration.rs`의 공통 `prepare_raw_input`은 pending 입력과 새 입력을 합치기 전에 pending event 개수를 기록해 새 hook으로 전달합니다. 웹의 기존 hook 호출은 유지합니다.

logic-only 호출은 egui UI pass 없이 필터링된 RawInput을 다음 호출에 다시 붙입니다. 터미널은 pointer 이벤트를 egui의 focus/drag용으로 남기면서 별도 패킷에 복사하므로, 기존 hook만으로는 같은 이벤트를 두 번 복사했습니다. 새 인자는 기존 입력과 새 입력을 값 비교나 추측 없이 구분합니다. 실제 headless 재현에서 2개가 4개로 늘어난 RED를 수정했고, actual Views→PTY 검사에서도 logic 재호출과 두 UI pass 뒤 중복 없는 수신을 확인했습니다. `docs/quality-assurance/2026-10-02-m8-native-terminal-mouse-adapter.md`가 검증 정본입니다. 실제 OS/보조 창 gate는 남습니다.

## 2026-10-27 immediate viewport의 편집 요청

대상은 `src/native/wgpu_integration.rs`입니다. `ViewportCommand::RequestCut`·`RequestCopy`·`RequestPaste`는 `egui_winit::process_viewport_commands`가 그 viewport의 `actions_requested`에 쌓고, 원본은 이를 `egui::Event::Cut`·`Copy`·`Paste`로 바꾸는 drain을 root·deferred viewport 그리기 경로에만 둡니다. `show_viewport_immediate`로 그리는 `render_immediate_viewport`는 이 목록을 읽지 않아 보조 창으로 보낸 요청이 조용히 버려졌습니다(키보드 ⌘C·⌘V는 egui-winit이 키 입력에서 바로 이벤트를 만들어 정상).

수정은 원본 drain을 `push_requested_edits`로 옮겨 두 경로가 함께 쓰게 한 것입니다. root 경로는 원래처럼 그리기 뒤 Screenshot을 먼저 꺼낸 다음 부르므로 동작이 같습니다. immediate 경로는 `take_egui_input` 바로 앞에서 불러, 앞 프레임 root 출력 처리에서 쌓인 요청이 그 viewport의 이번 입력으로 들어갑니다. Paste는 원본과 같이 클립보드 텍스트의 `\r\n`을 `\n`으로 바꾸고 비어 있지 않을 때만 넣습니다. immediate viewport의 Screenshot 요청은 원본처럼 처리하지 않고 목록에 그대로 둡니다. glow backend는 포함하지 않습니다. headless에서는 winit 창·event loop가 없어 이 경로를 직접 실행할 수 없으므로 앱 쪽 검사와 실제 창 확인은 `docs/quality-assurance/2026-10-27-native-app-menu.md`에서 추적합니다.
