# Native terminal menu 접근성 부모와 release 포커스

대상은 `native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`, `vendor/egui-input/src/context.rs`입니다.

실제 AX tree 생성 검사에서 secondary release 뒤 focus가 Menu가 아니라 Window였습니다. visible 빈 pass의 focus는 유지돼 release 원인을 구분했습니다. 기존 open 시점 Memory/wire 확인만으로 최종 AX focus 유지까지 증명할 수 없었습니다.

focus widget 본문 밖 Frame 경계 release가 기본 Clicks 외부 클릭으로 처리되는 원인입니다. 실제 focused ID·직전 viewport menu owner·press 없는 secondary-only release에만 surrender 예외를 적용했습니다. 다른 release/press/일반 widget·Presses/Never는 기존 정책입니다.

실제 버튼들이 focus widget의 자식 아닌 형제였으므로 UiBuilder accessibility_parent로 실제 child Ui를 연결했습니다. 실제 bounds/actions/disabled를 유지하며 MenuItem·Split HasPopup Menu·Separator Splitter/Horizontal을 부여했습니다. Split의 장식 화살표가 포함된 이름도 원본 locale 이름으로 정정했습니다.

actual AX 신규1 PASS(4.07초/0.43초), 변경 영향3 PASS(0.42초), engine/app strict3.98초·fmt/parse/diff exit0입니다. 최초 E0599는 fixture의 없는 Role::Separator 오류이며 제품 RED와 구분합니다. 정확한 명령/실패/재사용/한계는 context-menu QA 실제 AX 구조 절입니다.

전체 submenu/keyboard/bounds/관계·mixed/current topology/VoiceOver/OS GUI/M8 완료 증거가 아닙니다. 목표active·M8 0/8·완료 전 commit/push 없음입니다.
