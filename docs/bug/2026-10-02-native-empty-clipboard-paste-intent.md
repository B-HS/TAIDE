# Native 내부 파일 Paste의 빈 OS clipboard 입력 손실

## 현재 판정

격리 native app의 keyboard 의도 보존과 Explorer/NativeEditor 코드 검사는 완료했습니다. 실제 OS clipboard·원격/보조 창 실기·미연결 terminal·ViewportCommand::RequestPaste 경로는 아직 전체 입력 gate에 남습니다. OS clipboard를 비우거나 실제 앱을 조작해 재현했다고 주장하지 않습니다.

## 원인과 영향

egui-winit은 Command+V를 발견하면 clipboard.get을 호출합니다. 값이 없거나 CRLF 정규화 뒤 빈 문자열이면 Event::Paste를 만들지 않으며 분기에서 바로 return하므로 Key::V도 없습니다. 비어 있지 않은 값이 있을 때만 Paste를 내보냅니다.

원본 `src/widgets/explorer/use-explorer-clipboard.ts`는 mode/path/kind를 component-local state에 저장하며 OS text clipboard를 요구하지 않습니다. 현재 `native/taide-native-app/src/explorer.rs`의 내부 clipboard도 이 계약을 재현하므로, 내부 Copy/Cut은 성공했지만 OS clipboard가 비어 있거나 읽기 실패하면 shortcut Paste가 입력 어댑터에서 사라집니다. 메뉴 Paste 및 주입된 Paste event의 성공은 이 경계의 해결 근거가 아닙니다.

## 적용한 근본 수정

기존 eframe vendor의 feature-gated common `epi_integration::on_window_event`에서 upstream 변환 전 event 수를 저장하고 변환 후 실제 비합성 key-down의 logical/physical key·창별 modifier를 확인합니다. upstream이 이미 nonempty Paste나 다른 event를 만들었으면 추가하지 않습니다. 입력이 사라진 Paste만 semantic Key::Paste로 보존하며 원본 modifier/physical metadata를 유지합니다. Explorer의 기존 focus·exact modifier·IME/editing/popup gate에서 내부 파일 Paste로 소비합니다. 빈 Text/Paste payload와 Ctrl+V 바이트를 만들지 않습니다.

처음에는 semantic Key::Paste의 눌림만 추가했지만 실제 egui keys_down 상태가 남는 검사를 1건 FAIL, 0.01초로 재현했습니다. 물리 V release와 semantic Paste는 다른 key이므로 같은 frame의 의미상 눌림·해제 쌍으로 수정했습니다. 최종 helper/실제 Explorer·NativeEditor 검사 2건 PASS, 0.03초이며 한 번의 파일 action과 focused editor의 본문·revision·선택 보존, 잔류 key 없음이 확인됐습니다. focus loss/Destroyed와 backend live window map prune으로 modifier 수명도 정리합니다.

기본 비활성 feature를 격리 native app에서만 켰으며 기존 사용자 실기 bundle·OS clipboard/설정은 변경하지 않았습니다. wgpu app compile과 최종 strict clippy exit 0, 0.75초입니다. full winit KeyEvent·OS 실패를 실기로 확인한 검사는 아니고 terminal/비활성 glow·OS RequestPaste gate도 남습니다.

OS clipboard에 sentinel/파일 경로를 강제로 쓰기, key-up에서 추측해 대신 Paste 실행, 발생하지 않은 이벤트의 테스트만 통과시킨 뒤 완료 처리, registry source를 직접 수정하는 방식은 해결로 쓰지 않습니다.

## 연결 문서

`docs/quality-assurance/2026-10-02-m8-native-explorer-clipboard.md`, `docs/PROCESS.md`에 코드 검사 범위와 남은 창별 component owner·전체 메뉴/OS/terminal gate를 구분했습니다. 전체 내부 Cut·Copy·Paste/M8은 계속 미완료이며 GUI menu/worker·dirty 문서·8회 제한의 기존 성공을 재사용합니다.
