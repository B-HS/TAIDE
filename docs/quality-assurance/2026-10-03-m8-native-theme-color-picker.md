# M8 native 색상 선택기 primitive

## 대상 파일

- `native/taide-native-app/src/theme-color-picker.rs`, `theme-color-picker-tests.rs`
- `native/taide-native-app/src/presentation.rs`, `lib.rs`
- 원본: `src/features/theme/color-picker.tsx`, `color-picker-drag.ts`, `src/shared/lib/color.ts`

## 리포트

원본 색상 선택기의 HSV·입력·drag-local 상태와 native popup/gradient/keyboard/hex primitive를 구현했습니다. ThemeEditor/Settings/App에 아직 조립하지 않았습니다. 이 primitive 검사를 전체 화면·픽셀·AX·OS pointer capture 통과로 계산하지 않습니다.

## 구현 계약

1. 원본의 RGB→HSV 및 HSV→RGB/hex 계산과 short hex 확장·alpha를 제외한 HSV 해석을 옮겼습니다. 슬라이더 이동은 opaque6자리 hex를 생성하며 직접 hex blur는 원본 alpha/짧은 hex 형식을 유지하고 소문자로 정규화합니다. transparent는 원본 HSV 기본값 hue0/saturation1/value1로 표시합니다.
2. drag는 Picker 내부에만 보관하며 pointerdown/move로 draft/App preview 변경을 내보내지 않습니다. matching primary release에서 최종 pointer 좌표를 반영해 한 번만 문자열을 반환하고 drag를 비웁니다. 추가 release는 no-op입니다. PointerGone/창 focus loss/강제 popup 닫힘은 local drag를 폐기합니다. 원본 browser pointercancel/lostpointercapture와 실제 winit OS capture의 대응은 아직 실기 gate입니다.
3. Square는 s/v를0~~1로 clamp하고 hue를 유지합니다. hue slider는 s/v를 유지하며 hue0~~360을 적용합니다. Home/End와 SV0.05/색상1도 arrow 이동은 즉시 commit합니다. slider focus와 arrow filter, translated slider widget info를 연결했습니다. 실제 AX orientation/range/value text/action·OS keyboard는 완료로 표시하지 않습니다.
4. Hex 입력은 저장값 변경에 keyed 상태로 바꾸며 Enter만으로 blur하지 않습니다. blur에서 원본 허용 hex/transparent만 commit하고 잘못된 입력은 로컬 오류로 유지합니다. 새 popup 열기에서는 원본 input remount처럼 raw 문자열을 저장값으로 복원합니다. 오류 flag는 원본의 상위 useState처럼 유지합니다. label/tooltip·색상 swatch/alpha·HSV layer gradient/색상 thumb와 popup을 렌더합니다. 기본176px 높이/폭·14px hue와208 default-width를 사용하며 원본 CSS의 실제 내부 폭·border/radius/shadow/폰트/error input outline·완전한 픽셀 일치는 추가 gate입니다.
5. 전역 presentation color 파서가 기존6/8자리 hex만 받아 원본 유효한3자리 hex/transparent draft를 앱 미리보기로 적용할 수 없었습니다. 이 파서를 원본 trim/transparent/#RGB 범위로 확장해 shared consumer가 같은 입력을 처리하게 했습니다. 기존6/8자리 alpha·오류 구분을 보존합니다. 검증되지 않은 색을 강제로 검은색 등으로 바꾸는 처리는 넣지 않았습니다.

고정 egui0.36.2 설치 source의 Response/Popup/WidgetInfo/TextEditState/Context와 epaint Mesh 공개 API를 확인했습니다. Response/Popup docs.rs 조회2건은 접근 실패였으며 기억만으로 API를 대체하지 않았습니다. `Context.run`을 잘못 작성한 fixture compile은 실제 `run_ui` 계약으로 정정했습니다. private API·새 의존성·vendor 변경·검사 억제를 사용하지 않습니다. test-only trace는 widget id/rect4개만 기록하고 제품 debug log를 만들지 않았습니다.

## 실제 검증

Cargo는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`에서 직렬 실행했습니다.

- [x] `cargo test … --lib native_theme_color_picker -- --test-threads=1`: fixture E0599/exit101 정정 뒤2 PASS, suite0.04초/컴파일5.62초. short/6/alpha hex·transparent·잘못된 입력·RGB/HSV 기준 색상, shared color parser와 alpha 보존을 확인했습니다. 실제 headless popup에서 pointerdown/move0회/release1회/추가 release0회·취소·Home/End와 hex valid blur/Enter 단독 no-commit을 확인했습니다. popup은 fixture에서 직접 열어 검사했으며 trigger 최초 클릭/자동 전체 선택/Tab/실제 재열기 완료의 증거는 아닙니다.
- [x] `cargo test … --lib native_theme_color_picker는_drag -- --test-threads=1`: 제품의 reopening raw input 복원·focus loss admission·외부 invalid 표시를 추가한 뒤 영향 UI1건만 PASS, suite0.04초/컴파일2.95초. 불변인 HSV/shared parser 성공은 다시 실행하지 않았습니다.
- [x] `cargo clippy … --lib --bins --tests -- -D warnings`: exit0,12.63초. inherited Wry17 warnings와 authored strict를 구분합니다.
- [x] 이번 slice 포함 authored8파일 `rustfmt --check --edition 2024 --config skip_children=true` exit0. repository/vendor 전체 formatter를 실행하지 않았습니다.
- [x] 최신5문서 Prettier exit0, tracked PROCESS/HANDOFF diff check exit0입니다. 이번 slice authored8개·재개 프롬프트/신규 QA2개 등11파일의 no-index whitespace check는 모두 출력 없음/exit1(내용 차이만)로 오류가 없습니다.

실제 앱을 열거나 보호 bundle을 조작하지 않았습니다. 합성 locale/theme과 raw headless input만 사용하며 OS 설정·사용자 문서·실제 clipboard/IME/VoiceOver·lock/MSRV·dependency·제품 TS는 변경하지 않았습니다.

## 남은 게이트

후속 연결 정본은 [ThemeEditor 검증](2026-10-03-m8-native-theme-editor.md)입니다. 실제 Settings/HostBridge/renderer·token 순서/filter/reset/style/custom 목록/dialog·local preview와 viewport/활성 owner별 App palette 선택을 구현했고 관련11건을 검사했습니다. 실제 trigger/hex/reset blur의 적용 순서 RED는 [버그 기록](../bug/2026-10-03-native-theme-reset-blur-order.md)에 있습니다. 아래 최초 primitive 단계의 미연결 TODO는 이 후속 근거로 기본 연결만 완료됐으며 전체 GUI/픽셀/AX는 계속 미완료입니다.

- [ ] ThemeEditor/Settings/App wiring, original token namespace 순서/filter/reset·syntax styles/custom 목록·모든 dialog와 preview 원복/선행 worker/닫힌 owner 수명.
- [ ] trigger 최초 click·keyboard/Tab/autofocus/선택/reopen, invalid hex blur 실제 입력, hue keyboard/외부 값 변경·drag 중 popup Escape·window loss와 OS pointer capture. 기존 성공 입력을 반복 측정하는 대신 새 위험이 실제 연결될 때 해당 경계를 한 번 검사합니다.
- [ ] popup·gradient·thumb·error outline·font/radius/shadow/폭·locale·반응형/모든 테마의 원본 pixel·색공간/메모리/CPU/GPU, AX slider orientation/range/value text/action. 현재 primitive 기본 geometry/가독성을 전체 UI 같음의 증거로 쓰지 않습니다.
- [ ] 전체 M8 N1~N8 0/8·213view/41action/Monaco21/palette·cutover/TS제거/Rust99%·성능/보안/배포/rollback. 별개 keybinding Tab RED/PTY remount 선택도 남아 있습니다. 목표active·전체 M8 완료 후에만 commit/push합니다.
