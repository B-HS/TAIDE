# M8 상태바 커서·글꼴 조작

## 대상과 실제 호출

대상은 신규 `native/taide-native-app/src/status-editor.rs`와 `lib.rs`, `application.rs`, `host.rs`, `presentation-refresh.rs`입니다. 원본 `src/features/window/status-bar.tsx`, `font-size-stepper.tsx`, `src/entities/settings/use-editor-font-size.ts`, `src/widgets/window-chrome/status-bar-content.tsx`를 대조했습니다.

NativeApplication의 포커스 프로젝트와 focused pane/active tab으로 ViewKey를 만들고 실제 EditorStore의 primary selection head를 읽습니다. 기존 byte_to_position 변환을 재사용해 UTF-16 기반 행·열을 1-based로 표시합니다. active tab에 native editor view가 없거나 회수됐으면 표시하지 않습니다. 문서 바이트 오프셋이나 Unicode scalar 수를 Monaco column으로 오인하지 않습니다.

AppSurfaces status_bar의 오른쪽에 editor/terminal font stepper를 연결했습니다. 원본의 14px icon slot·12px glyph·16px 감소/증가 button·24px 숫자 button·2px control gap·11px 글꼴과 색상/hover 키를 사용합니다. 고정 폭 allocation으로 오른쪽 배치에서도 각 stepper 내부는 감소→초기화→증가 순서를 유지합니다. Stable viewport/target/operation ID와 실제 클릭 sense·Button WidgetInfo·locale tooltip을 사용합니다. 초기 생성·정상 테마 갱신·preview에 Appearance 갱신도 연결했습니다.

editor는 기존 SetEditorFontSize를 사용하고 terminal은 같은 기존 update_control_settings 경로의 SetTerminalFontSize를 추가했습니다. 두 크기는 공유 clamp6~48/step1 함수를 사용하고 각 모델의 기존 default13으로 초기화합니다. font 변경에는 IDE/hooks/remote toggle 변화가 없습니다. full Settings reconcile/production App ports가 연결됐다고 주장하지 않으며 기존 HostBridge의 제한된 font 저장 경로만 사용했습니다. SettingsChanged 뒤 실제 NativeApplication의 기존 editor appearance/terminal palette/font-size 동기화 경로가 반영합니다.

## 최소 검사

명령 prefix는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo`, 공통 flags는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] 최초 `test --lib status_editor::tests`는 신규 fixture Sink에서 SettingsChanged의 Box<Settings>를 unbox하지 않은 E0308(handle2996)이었습니다. 생산 코드 오류나 제품 RED로 세지 않으며 실제 event 계약대로 fixture만 수정했습니다.
- [x] 수정 뒤 같은 신규 범위 2 PASS·compile13.76초/suite0.06초(handle45852), filtered218입니다. cursor 검사는 CRLF/CJK/emoji·primary 선택·다른 tab/없음/detach·UTF-16 행열과 양쪽 font clamp/default를 확인합니다. 버튼 검사는 egui의 실제 pointer press/release로 양쪽 감소/초기화/증가 총6개 동작을 생성하고 정확한 target/value·button geometry·HostBridge 실제 settings file persist·live Settings·SettingsChanged·다른 설정 불변·shutdown/task0·weak owner 회수를 확인합니다. 서로 다른6동작이지 같은 검사3회 반복이 아닙니다.
- [x] 변경된 native lib/bin/tests의 `clippy -- -D warnings` exit0·16.15초(handle36664), authored5 exact rustfmt입니다. 기존 Wry dependency17 warnings는 별도입니다. 신규 Rust5 no-index whitespace 출력 없음(exit1은 신규 diff)입니다.
- [x] 앞선 LSP 상태 UI의 집계/renderer/실제 child recovery 성공은 재사용했습니다. editor font keyboard와 terminal font 표시의 이전 성공도 동일 동작 범위에서 재사용하며 전체 회귀를 반복하지 않았습니다.

Cargo는 직렬/locked/offline/기존 target만 사용했습니다. 이 slice에서 manifest/lock/root/Tauri/MSRV/제품 TS/Git은 불변입니다. 합성 UUID 디렉터리의 설정만 읽고 회수했으며 사용자 home·clipboard·앱·보호 bundle·OS/IME/VoiceOver·키 파일에는 접근하지 않았습니다.

마무리: 결과4문서 Prettier 완료·tracked whitespace exit0, 신규 QA no-index whitespace 출력 없음(exit1은 신규 diff)입니다. 모든 Cargo handle은 종료됐으며 성공한 동작 검사는 다시 실행하지 않았습니다.

## 확인한 API와 아이콘

설치된 공식 egui0.36.2의 allocate_exact_size·allocate_ui_with_layout·with_layout·read_response·WidgetInfo·Response click 계약과 StrokeKind를 확인했습니다. 원본 아이콘은 공개 Lucide [type](https://github.com/lucide-icons/lucide/blob/main/icons/type.svg), [square-terminal](https://github.com/lucide-icons/lucide/blob/main/icons/square-terminal.svg), [minus](https://github.com/lucide-icons/lucide/blob/main/icons/minus.svg), [plus](https://github.com/lucide-icons/lucide/blob/main/icons/plus.svg) SVG를 읽어 재현했습니다. 기존 LICENSE-LUCIDE ISC/MIT 고지를 유지합니다. 현재 공식 SVG 기준이며 pinned 원본 binary의 픽셀 비교는 아직 수행하지 않았습니다.

## 미완료

- [ ] 실제 GUI/OS 접근성·keyboard focus ring·tooltip top 배치·tabular numeral glyph·좁은 창 overflow·DPI/anti-alias·원본 전체 상태바 비교는 최종 화면 gate에서 확인합니다. 합성 pointer 검사를 사용자 화면 검증으로 세지 않습니다.
- [ ] AppSurfaces가 pane보다 먼저 status bar를 그리는 실제 프레임 순서의 cursor 변경 표시와 여러 slot/auxiliary window 실기는 미실행입니다. headless cursor/model와 실제 코드 연결을 분리하며 전체 창 parity에서 확인합니다.
- [ ] IDE·system usage/detail·Problems·chord 표시와 LSP provider/diagnostics UI·same-generation handshake/dispose/reacquire는 남습니다.
- [ ] actual App 필수 assets/effects/ports owner·Settings/AppFile/IDE 화면·Rust remote UI·keybinding RED/PTY remount·N1~N8 0/8·최종 TS 제거/배포는 미완료입니다. 전체 완료 전에 commit/push하지 않습니다.
