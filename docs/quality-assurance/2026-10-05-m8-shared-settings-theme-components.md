# M8 Settings 테마 상태·공용 색상 선택기와 미리보기

## 대상 파일

- `crates/taide-model/src/identifier.rs`, `src/lib.rs`, `crates/taide-infra/src/root_guard.rs`
- `native/taide-native-ui/src/settings-owner.rs`, `theme-draft.rs`, `theme-edit.rs`, `src/lib.rs`, `tests/theme-portable.rs`
- 같은 UI의 `theme-color-picker.rs`, `theme-live-preview.rs`, `theme-editor-tokens.rs`, `tooltip-trigger.rs`, `Cargo.toml`
- `native/taide-native-app/src/settings-view.rs`, `theme-draft.rs`, `theme-edit.rs`, `theme-color-picker.rs`, `theme-live-preview.rs`, `theme-editor-tokens.rs`, `tooltips.rs`, `Cargo.toml`

## 리포트

기존 후속47의 전체 Settings provider 이식을 위한 실제 공용 코드 이동입니다. 테마 초안·설정 탭 소유자·편집 요청 상태와 기존 색상 선택기/미리보기/토큰 순서/툴팁 입력 계약을 공유합니다. native 호출은 re-export로 유지하며 디스크 실행은 기존 native-host feature에만 남깁니다. 전체 Settings 화면, 테마 편집기, Catalog, 원격 RPC provider와 제품 bundle은 아직 미완료입니다.

## 보존·경계

1. `ensure_safe_component`의 기존 본문·localized 오류·인자를 순수 model로 동일 이동하고 infra 경로를 re-export했습니다. 파일 시스템 기능을 브라우저로 가져오지 않습니다. 기존처럼 빈 값·dot/dotdot·slash/backslash를 거절하며 서버의 canonical/root/쓰기 검증을 대체하지 않습니다.
2. Draft의 dirty/reset·최소 diff·preview·raw token 규칙/metadata·slug와 UUID 충돌 처리 본문은 보존합니다. `from_resolved`는 typed 목록/source/base를 받고 동일 출처와 theme type/base id를 확인합니다. native load는 기존 shutdown/목록/builtin edit 거절/실제 theme_get 뒤 동일 생성자를 호출합니다. source/mode provenance는 Session이 유지합니다.
3. Owner는 같은 project/pane/tab이며 순수 `is_active_in`은 main 및 auxiliary root의 실제 활성 Settings tab을 검사합니다. native `is_active`는 기존처럼 먼저 해당 project의 layout을 선택합니다. 브라우저 provider도 해당 project의 layout을 선택해야 합니다. Session ticket이 살아 있다는 사실만으로 layout 활성 여부나 서버 권한이 보장되지 않습니다.
4. Session/요청의 고유 Arc ticket·operation identity·Drop 폐기·다른 source/mode 저장 거절을 동일 유지합니다. portable 요청은 read-only owner/source/mode/theme 접근과 load resolve를 제공하며 폐기된 ticket은 resolve를 거절합니다. 실제 browser RPC 제출/응답/late/Closed 및 UI mount 검사는 다음 provider 단계입니다. native supervised load/save/delete·mutation guard·삭제 전 fallback/settings 저장/사건 순서는 같은 본문을 사용합니다.
5. ColorPicker HSV/hex blur/실제 slider/drag release/keyboard·Preview의 editor/terminal/ANSI16·token 배열은 기존 본문을 이동했습니다. import/가시성/시험 cfg와 formatter의 공백·마지막 쉼표를 제외한 원본 비교는 동일합니다. Tooltip Trigger/wrap_button만 순수 UI로 이동하고 기존 native Provider는 같은 type을 받습니다. 전체 tooltip popup/placement/motion provider의 브라우저 이식은 아직 아닙니다.

native App의 dev-dependency만 같은 UI의 `inspection` feature를 켭니다. 기존 시험의 picker geometry 접근을 유지하기 위한 읽기 전용 traces이며 일반 native/remote 제품 feature에서는 제외합니다. 새 registry 의존성·버전·lock/MSRV 변경은 없습니다. 기존 native Draft/Picker unit test source는 공용 UI 모듈에서 native-host 시험으로 사용하고 원본 파일을 지우거나 테스트를 무력화하지 않았습니다.

Rust의 [조건부 컴파일 문서](https://doc.rust-lang.org/reference/conditional-compilation.html)와 기존 runtime/모델/renderer 본문을 확인했습니다. include macro의 공식 문서도 조사했지만 별도 source 복제/비위생적 include로 renderer를 중복 구현하지 않고 module 이동/re-export와 시험 feature를 사용했습니다.

## 검증

모든 Cargo는 같은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, 해당 `bin/cargo`, `--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`로 직렬 실행했습니다. 성공한 입력/환경은 다시 실행하지 않았습니다.

- [x] 새 portable `cargo test --manifest-path native/taide-native-ui/Cargo.toml … --no-default-features --test theme-portable`: 2 PASS, build4.16초/suite .00초. 조회 출처·최소 변경/reset/dirty·복제·입력 경계·동일 owner remount/요청별 identity/Drop/다른 draft·실패 Reply를 확인했습니다.
- [x] 변경된 Draft의 기존 `… --manifest-path native/taide-native-ui/Cargo.toml … --lib native_theme_draft -- --test-threads=1`: 2 PASS, build8.93초/suite .40초. 모든 builtin/bundled의 실제 저장/상속 및 import metadata/token 규칙·reset/색상/식별자/충돌/shutdown을 확인했습니다.
- [x] native `… --manifest-path native/taide-native-app/Cargo.toml … --lib native_theme_edit -- --test-threads=1`: 7 PASS, build20.90초/suite .33초. 필터는 edit3 및 editor4를 포함했습니다. 실제 저장/삭제/fallback 실패·owner 폐기·mutation 대기 및 UI 요청/재시도/입력 순서를 확인했습니다. 첫 compile의 E0433 43개는 Owner 이동 후 기존 Settings 시험이 부모 import를 상속하던 문제였으며 test-only import를 복원한 실패1만 재실행했습니다. 성공7건을 후속 component 이동 뒤 반복하지 않았습니다.
- [x] 이동된 Picker의 `… native/taide-native-ui/Cargo.toml … --lib theme_color_picker::tests::native_theme_color_picker는_drag_단일commit과_keyboard_hex_blur를_연결한다 -- --exact --nocapture`: 1 PASS, build1.06초/suite .04초. 실제 egui drag 단일 commit·취소/keyboard/hex blur를 확인했습니다.
- [x] native Provider의 `… native/taide-native-app/Cargo.toml … --lib tooltips::theme_tests::theme_tooltip은_picker와_ansi16의_공용_theme_ax와_bottom_top을_보존한다 -- --exact --nocapture`: 1 PASS, build21.82초/suite .16초. 공용 Picker/Preview의 trigger type과 native Provider·각 builtin theme·AX/label/bottom/top 및 ANSI16을 확인했습니다.

순수 상태와 기존 native 영향은 고유13건 PASS입니다. 새 Chrome 또는 GUI 검사를 실행한 것은 아닙니다. 앞선 Chrome canvas/preferences 결과는 당시 source의 독립 증거이고 이번 소스의 브라우저 렌더/제품 UI 완료 근거로 확대하지 않습니다.

정적 검사는 상태 이식 후 production Wasm check2.35초·portable lib/test strict4.87초, component 이식 후 production remote-Wasm/canvas lib strict .62초 exit0입니다. 첫 component formatter/compiler의 unclosed delimiter는 이동 범위 끝의 닫는 brace 누락이었으며 즉시 정확한 함수 경계로 정정한 실패1만 재실행했습니다. authored Rust exactfmt·tracked diff whitespace exit0입니다. Wasm normal graph에는 공용 UI만 있고 runtime/infra/Tokio/resvg/native-app package는 없습니다. 출력의 egui vendor 경로가 native-app 디렉터리를 가리키는 것은 기존 patch이지 native-app package 의존성이 아닙니다. 기존 Wry17·linker eh-frame 경고는 변경/억제하지 않았습니다.

## 다음 게이트·상태

다음은 실제 전체 Settings/ThemeEditor renderer와 icons/Catalog·tab/mount·toast/툴팁 소비를 같은 Rust UI로 이식하고 BrowserApplication/CanvasContents에 연결하는 것입니다. 현재 공용 Picker/Preview만으로 사용자가 제품 브라우저에서 전체 설정 화면을 사용하는 상태라고 주장하지 않습니다. public bundle/폰트/다른 surfaces/패키징 및 close 전체 handshake/deadline/auto-save/LSP·GUI/IME/VoiceOver/성능/security/beta/cutover/Rust99/TS제거는 기존 미완료 항목입니다.

이번 공용 component 세부3/3(100%)·후속47 2/4(50%)·전체 M8 363/433(83.83%, 공수비 아님)·N1~N8 0/8입니다. 전체 ETA는 남은 화면/패키징/최종 gate 공수 미확정으로 산정 보류입니다. goal active·main 직접·전체완료 전 commit/push 없음·OS/보호 앱/사용자 데이터/제품 TS 불변·live handle 없음입니다.
