# M8 Snippets 이름 배치·Dialog 제목 색상

## 대상·원본

공용 `snippet-editor.rs`와 실제 `tests/snippet-editor.rs`입니다. 원본 `src/features/snippet/snippet-entry-editor.tsx`는 첫 이름 label도 flex-col/gap-1이며, Dialog 제목은 앱의 전경색을 상속합니다. 실제 Chrome lifecycle screenshot에서 확인한 시각 대조의 후속으로 같은 생산 renderer를 검사했습니다.

## 재현·수정·검증

- [x] 이름 label/input 단일 실제 UI 검사 RED→GREEN입니다. label [[293,130]–[324.6,146]]과 input [[341.6,135]–[922,151]]로 같은 행에 놓여 실패(build .65초/suite .12초)했습니다. 이름 셀의 `allocate_ui`가 부모 horizontal layout을 상속한 원인이며 명시적 top_down layout으로 수정했습니다. 원본 세로 gap과 좌측 border 1px+padding 8px inset을 검사한 최종 1 PASS(build 1.24초/suite .12초, filtered 10)입니다.
- [x] Dialog title 실제 UI 검사 RED→GREEN입니다. taide-dark의 app.foreground #CDD6F4 대신 SDK strong 색 #FFFFFF가 사용되어 실패(build .63초/suite .11초)했습니다. 설치된 egui 0.36.2 widget_text의 `get_text_color`에서 strong이 override_text_color보다 먼저 선택됨을 확인했습니다. 명시적 app.foreground 연결 후 최종 1 PASS(build 1.24초/suite .11초, filtered 11)입니다. 실제 Chrome의 시스템 기본 강조색에도 의존하지 않도록 같은 생산 소스를 수정했습니다.
- [x] 최종 정상 Canvas Wasm check .54초 exit 0/경고 0입니다. 성공한 Chrome 수명/Notice/global input 검사는 재사용하고 반복하지 않았습니다. 최신 browser bindings와 screenshot은 이름 배치·제목 색상 수정 전 lifecycle 소스이며 이번 색상/배치는 실제 공용 UI 도형 검사까지입니다.
- [ ] 원본 전체 버튼 focus-visible·전환/padding·글꼴/AX/모든 theme/DPI/픽셀·자동완성 UI/삽입·전체 native shutdown·나머지 Settings/App/assets/최종 gate·사용자 담당 마지막 CJK/VoiceOver는 남습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_이름_label과 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_dialog_제목은 -- --nocapture
```

이번 두 좁은 UI 경계 2/2(100%)·Snippets 1/4(25%)·provider 2/4(50%)·M8 363/433(83.83%)·최종 0/8·전체 ETA 산정 보류입니다. 전체 완료 전 Git 없음·main 직접·live 검사 없음·OS/Keychain/사용자 데이터/제품 TS/의존성/lock/MSRV 불변입니다.
