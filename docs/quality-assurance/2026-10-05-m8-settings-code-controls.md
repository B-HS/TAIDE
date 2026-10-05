# M8 Editor·Terminal 설정 변경 계약

아래는 변경 계약만 구현한 선행 단계의 당시 기록입니다. 실제 화면·목록 소비·현재 상태는 `2026-10-05-m8-settings-code-view-and-resources.md`가 정본입니다. 이 문서의 당시 1/4·화면 미구현 상태를 현재 상태로 사용하지 않습니다.

## 대상 파일

- `native/taide-native-ui/src/settings-code-controls.rs`
- `native/taide-native-ui/src/settings-controls.rs`
- `native/taide-native-ui/src/lib.rs`
- `native/taide-native-ui/tests/settings-code-controls.rs`

## 리포트

원본 `src/widgets/settings-view/settings-editor-section.tsx`와 `settings-terminal-section.tsx` 전체 및 실제 FontPicker·OptionPicker·TextField·NumericField·ShellProfileList·editor-rulers·Settings/SettingsPatch를 확인했습니다. 기존 renderer에는 두 섹션이 없으므로 먼저 공유 가능한 typed 변경 계약을 구현했습니다. 실제 화면이나 font/shell RPC 연결을 완료했다고 주장하지 않습니다.

26개 스위치는 원본 표시 키/설명 키/Settings 필드에서 값을 읽고 자기 SettingsPatch 필드 하나만 변경합니다. 선택값은 기존 EditorRenderWhitespace/EditorCursorStyle/EditorCursorBlinking/TerminalCursorStyle을 그대로 사용하며 원본 옵션 순서·메시지 키를 보존합니다. 새 enum이 기존 모델 타입을 문자열로 재정의하거나 검증을 약화하지 않습니다. 공용 Change::Code가 기존 SettingsPatch 경로를 사용하므로 별도 저장 경로를 만들지 않습니다.

폰트 기본 선택은 원본 null 선택처럼 빈 문자열 patch입니다. Editor/Terminal FontSize는 6~48, AutoSaveDelay는 0~60000, EditorTabSize는 1~8, TerminalScrollback은 100~100000이며 기존 NumericDraft의 blur 시 저장값 복원·실제 ack 값 변경 때만 동기화 규칙을 재사용합니다. 소수는 기존 typed u32 경계의 정수 오류로 처리합니다. 타이핑 중 값을 저장된 값처럼 유지하는 낙관적 동작은 추가하지 않습니다.

눈금은 원본 JS Number의 십진/지수/무부호 0x·0b·0o 표현을 처리하고 정수1~1000만 남겨 중복 제거·오름차순·최대16개로 정규화합니다. ES 문자열 trim의 BOM과 공백 집합을 유지하며 JS 공백이 아닌 U+0085를 Rust trim으로 잘못 허용하지 않습니다. 눈금 TextDraft는 blur 즉시 정규화된 표시를 유지하고 동일 저장값의 ack가 없는 경우에도 폐기된 문자를 남기지 않습니다. 셸 입력은 원본 callback처럼 patch에서만 trim하며 실제 저장값이 바뀌기 전 raw 표시를 강제 바꾸지 않습니다.

## 검증

- [x] 새 단일 portable 검사 첫 PASS: 빌드1.91초·검사.00초·1 passed/0 failed. 26개 스위치의 값/단일 patch, 5개 범위의 clamp/소수 오류/NaN/저장값 복원, 원본 typed 옵션, 눈금 표기·정렬·상한·동일 값 정규화, 셸 raw draft/ack, 폰트 초기화를 확인했습니다.
- [x] 변경된 공용 Change가 실제 browser/Wasm 소비 경로에 전파되어 normal production Wasm/canvas check1.28초 exit0을 확인했습니다. pure 단일 검사와 다른 플랫폼/소비 타입 위험을 덮는 검사이며 런타임을 반복 실행한 것이 아닙니다.
- [x] 수정 Rust4개의 exact rustfmt 및 대상 diff 검사 exit0입니다. 같은 성공 검사는 다시 실행하지 않습니다.
- [ ] 실제 Editor·Terminal 화면, font_list/shell_profiles의 독립 로딩과 matched 응답/owner/remount 수명, 실제 입력·설정 ack의 연속 검증은 남습니다.

실제 명령은 다음과 같습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-code-controls -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

## 다음 구현의 확인 사항

font_list와 shell_profiles는 원본 두 useQuery의 독립 자원입니다. 기존 Theme/Locale Catalog 완료를 두 목록 응답에 묶어 관련 없는 Appearance·Language를 지연시키지 않습니다. Native font_list의 원본은 `taide_font::service::list_families`, shell_profiles는 `taide_runtime::terminal_actions::shell_profiles`와 같은 서비스 정책입니다. 기본 portable/Wasm에 native 폰트 스캔이나 PTY/OS 의존성을 추가하지 않습니다. 목록 오류 표시/검색·키보드·Popup 수명은 실제 원본 query/provider/Command와 대조한 뒤 구현하며 추정하지 않습니다.

FontPicker의 system-default 항목도 원본 Command 필터의 대상이므로 검색과 무관하게 항상 표시한다고 가정하지 않습니다. 실제 원본 필터를 확인해야 합니다. OptionPicker의 unknown 값은 placeholder이며 첫 옵션 fallback을 추가하지 않습니다. 현재 코드의 네 가지 선택 모델은 원본 Settings 자체가 이미 typed enum인 경우입니다.

이번 변경은 Rust4개만이며 제품 TypeScript/OS/사용자 파일/키체인/보호 앱/의존성/manifest/lock/MSRV/Git은 변경하지 않았습니다. egui 온라인 TextEdit 문서 조회는 접근 불가였으며 사용 예정인 id_salt/return_key/lost_focus 계약은 설치된 실제 egui0.36.2 공식 소스·API 문서에서 확인했습니다. UI 구현이나 실제 픽셀 parity의 근거로 삼지 않습니다.

Editor·Terminal 세부 단계1/4(25%)·provider2/4(50%)·전체363/433(83.83%)·최종0/8입니다. M8 미완료·ETA 산정 보류·goal active·main 직접·전체완료 전 Git 없음·실행 중 검사 없음입니다.
