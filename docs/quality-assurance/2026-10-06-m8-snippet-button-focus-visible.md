# M8 Snippets Button 포커스 표시

## 대상·원본

공용 `snippet-editor.rs`와 실제 `tests/snippet-editor.rs`입니다. 원본 `src/shared/ui/button.tsx`의 focus-visible:ring-[3px]/ring-ring50%, outline border-ring, destructive ring-destructive20%와 Close의 별도 focus 규칙을 구분했습니다. [WICG 원천](https://github.com/WICG/focus-visible/blob/main/src/focus-visible.js)·[MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Selectors/:focus-visible) 및 설치된 egui Context/InputState/Response/IdTypeMap 원천을 확인했습니다.

## 구현

viewport별 Context 임시 데이터에 입력 modality와 현재 focus 표시를 저장합니다. 입력 raw events를 frame당 한 번만 읽어 Tab 등의 일반 키와 Alt/Ctrl/Meta 계열을 구분하고 포인터/touch 시작·초기 포인터·WindowFocused·AccessKit Focus를 처리합니다. 같은 focused 버튼의 표시를 유지하며 다른 버튼으로 포커스가 이동하면 입력 방식에 맞춰 판단합니다. 입력을 소비하거나 포커스를 강제하지 않습니다.

Back/Save/Delete/New/Add·Dialog Confirm/Cancel에 실제 Response 기반 표시를 연결했습니다. 일반 버튼은 app.focusBorder50%의 바깥3px, destructive는 statusIndicator.error20%의 바깥3px이며 outline은 안쪽1px border 색상도 변경합니다. disabled/창 비활성 버튼은 표시하지 않습니다. Close는 원본대로 기존 항상-focus2px/offset2px 규칙을 유지하고 Trash는 원본 IconButton 경계를 유지합니다.

## 검증·실패

- [x] 신규 actual Settings→파일 선택→Delete Alert의 합성 입력 연속 검사 RED→GREEN입니다. 첫 실제 Tab 뒤 destructive 링 누락으로 실패(build1.20초/suite.11초)했습니다. 생산 표시 연결 후 일반/destructive 링·Shift-Tab·단순 hover 보존은 통과했지만 WindowFocused assertion은 실패(build2.70초/suite.11초)했습니다.
- [x] 두 번째 실패는 fixture의 Event::WindowFocused(false)와 RawInput.focused=true 불일치였습니다. SDK Response::has_focus는 RawInput에서 만든 input.focused를 따르므로 실제 backend처럼 두 값을 일치시키고 복귀 상태도 유지하도록 Scene을 정정했습니다. 다른 기존 Scene 입력에는 영향이 없습니다. 최종 1 PASS(build.62초/suite.12초, filtered12)이며 pointer-triggered Alert Cancel에 링0·Tab Confirm의 error20%·Shift-Tab Cancel의 focus50%·mousemove 보존·창 비활성0/복귀표시·disabled0입니다.
- [x] 현재 정상 Canvas Wasm check1.05초 exit0/경고0입니다. 이전 Chrome 수명/이름·제목/global input/Close 검사는 같은 위험이 아니므로 성공 결과를 재사용하고 반복하지 않았습니다.
- [ ] 실제 Chrome/native OS 입력·AccessKit Focus 실기·모든 modifier/ordered mixed 입력·전체 전역 입력 scope/프로그램 포커스 그래프/색상 전환·전체 theme/DPI/픽셀을 이 검사로 통과 처리하지 않습니다. Context 초기 pointer/창 복귀 및 assistive branch는 구현했지만 이번 합성 UI가 덮지 않은 범위는 다음 통합 경계에 남습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_button은 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

이번 표시 경계 구현/검사/기록3/3(100%)·Snippets1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8·전체 ETA 산정 보류입니다. 색상 전환/전체 원본 시각·자동완성 UI/삽입·전체 native shutdown·나머지 Settings/App/assets/gate·사용자 담당 마지막 CJK/VoiceOver는 미완료입니다. 최신 browser bindings는 선행 lifecycle 소스이며 새 이름/제목/focus 변경은 이번 normal Wasm compile까지입니다. 전체완료 전 Git 없음·main 직접·live handle 없음·제품 TS/OS/Keychain/사용자 데이터/보호 앱/의존성/lock/MSRV 불변입니다.
