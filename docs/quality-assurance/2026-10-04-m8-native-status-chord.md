# M8 실제 chord 상태 표시

## 대상과 원본

대상은 `native/taide-native-app/src/{keymap,keymap-chord-status-tests,keybinding-catalog,terminal_surface,status-chord,application,presentation-refresh,lib}.rs`입니다. 원본은 `src/{features/window/status-bar,widgets/window-chrome/status-bar-content}.tsx`, `src/shared/lib/keymap/{keymap,keymap-chord-store}.ts`, 기존 `resources/keybindings/keyboard.svg`와 영어 locale입니다.

실제 입력에 사용되는 viewport별 Keymap Windows에서 pending prefix/편집기 유예/NoMatch를 읽습니다. 별도 화면 전용 단축키 map이나 IPC 요청은 없습니다. 기존 catalog의 Stage label을 재사용하므로 macOS/그 외 OS modifier 순서·키 이름이 같은 출처입니다. 원본 편집기 유예 표시는 고정된 mod+K를 사용합니다. 기존5초 대기·1.5초 불일치 표시와 불일치 우선 문구를 보존하며, 새 pending이 생겨도 남은 불일치 표시가 즉시 사라지지 않습니다.

새 물리 이벤트의 결정 생성 시에만 NoMatch deadline을 갱신합니다. 같은 frame/index를 공유하는 listener는 기존 memoized 결정을 사용하므로 deadline을 다시 연장하지 않습니다. 편집기 fallback의 추가 EnterChord 처리도 기존 dispatcher에 남깁니다. map 처리/같은 이벤트 텍스트 소유권·resolve/clear/retain·포커스 소실/캡처의 기존 clear 경로는 변경하지 않았습니다. 입력이 없는 timeout에서도 다음 repaint를 deadline에 예약하고 실제 결정 변경은 다음 프레임을 요청합니다. window map 제거 뒤 표시도 사라집니다.

AppSurfaces에 원본 locale·warning/error·11px 글꼴·12px Keyboard icon·4px gap을 연결하고 초기/일반/preview 테마 모두 갱신합니다. Keyboard SVG의 사각형/round dot/space 경로를 native vector로 재현했으며 신규 의존성/외부 리소스 요청은 없습니다. 기존 Lucide 라이선스를 유지합니다. 시스템 사용량 모달이 열려 있는 프레임에는 기존 toast/keybinding editor의 입력 gate도 닫습니다. 이 두 gate는 소스 연결과 strict 근거이며 실제 겹친 GUI 입력을 실행한 근거는 아닙니다.

## 실행 근거

공통 prefix는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo`, flags는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] 최초 `test --lib chord_status -- --nocapture` compile FAIL(handle29944): 새 terminal_surface getter에 필요한 Instant import를 정정했습니다. 의존성/검사 우회는 없습니다.
- [x] 같은 필터(handle9403) compile9.85초/suite0.02초 중 headless renderer1 PASS입니다. 문구/NoMatch 우선·11px font·색상·Keyboard의 native dot/space glyph를 확인했습니다. 이 성공은 이후 재실행하지 않았습니다.
- [x] 같은 실행의 route fixture만 FAIL입니다. 편집기는 원래 ObserveEditorPrefix 뒤 fallback handler에 EnterChord도 전달하는데 fixture가 두 결정을 같은 값으로 기대했습니다. 실제 handler 호출 순서에 맞춰 정정하고 `test --lib keymap::chord_status_tests -- --nocapture`만 1 PASS(handle83551), compile4.92초/suite0.01초, filtered225입니다. 실제 viewport route와 listener2개의 decision cache·NoMatch deadline 중복 없음·macOS/비macOS prefix·5초/1.5초 경계·NoMatch 중 재진입·clear/편집기 유예·viewport map 회수를 확인했습니다. 시간 경계는 Instant 결정적 비교이며 실제5초 wall clock 계측이 아닙니다.
- [x] 최초 native strict(handle55286)는 fixture의 모든 필드 지정 뒤 남은 `..Default`에 needless_update 오류입니다. 해당 test 생성 문법만 정정했으며 성공 동작은 재사용합니다.
- [x] 최종 `clippy --lib --bin taide-native-app --tests -- -D warnings` exit0·14.30초(handle72549), authored Rust8 exact rustfmt입니다. 기존 Wry dependency17 warnings는 authored strict와 별도입니다. test 생성 문법 정정은 의미 변화가 없어 앞선 두 동작 성공을 재사용했습니다.

## 미완료

- [ ] 실제 NativeApplication focus/terminal/editor·입력 없는 wall-clock timeout·여러 viewport·배율/좁은 창/전체 status bar/AX/픽셀 비교를 최종 화면 gate에서 확인합니다. headless를 실기라고 주장하지 않습니다.
- [ ] 원본 전부의 Problems panel/marker store·전체 diagnostics/provider/편집기 유예 행동·same-generation handshake/dispose/reacquire·actual App assets/effects/ports owner/Settings/AppFile/IDE 화면·Rust remote UI는 미완료입니다. chord 표시만으로 기존 모든 키 조합/편집기 기능이 완성된 것은 아닙니다.
- [ ] keybinding Tab RED/PTY remount·N1~N8 0/8·전체 M8 active·최종 TS 제거/배포/commit/push는 미완료입니다.

Cargo는 직렬/locked/offline/기존 target입니다. 이번 범위의 manifest/lock/dependency/root/Tauri/MSRV/제품TS/Git은 불변입니다. GUI 앱/사용자 home·clipboard·자격 증명·보호 bundle·OS 설정/IME/VoiceOver에는 접근하지 않았습니다. egui의 Context request_repaint_after/Ui/Painter/Shape API는 설치된0.36.2 공식 소스를 사용하며 신규 모델/서브에이전트/workflow는 사용하지 않았습니다.
