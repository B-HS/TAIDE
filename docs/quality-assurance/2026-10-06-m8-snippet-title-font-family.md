# M8 Snippets 제목 semibold family

## 대상 파일

- `native/taide-native-ui/src/font-families.rs`
- `native/taide-native-ui/src/snippet-editor.rs`
- `native/taide-native-ui/tests/snippet-editor.rs`
- `native/taide-native-app/src/ui-fonts.rs`

## 리포트

원본 DialogTitle/AlertDialogTitle의 `font-semibold`은600입니다. 기존 `.strong()`은 egui의 강조색 선택이며 굵기600 face를 선택하지 않았습니다. 새 실제 UI 검사에서 등록된 semibold family 대신 Proportional을 선택하는 RED를 재현한 뒤 별도 `taide-ui-semibold` family를 연결했습니다. 기존18px 크기·New18px/Alert28px line-height·app.foreground를 보존합니다. 새 파일·파일 삭제·항목 삭제·미저장 폐기 Dialog가 같은 선택 경계를 사용합니다.

native 준비는 기존400/500에600 family를 추가하고 같은 폰트 스택·variable wght 축·공유 byte budget·거절 fallback을 사용합니다. macOS는 설치된 objc2-app-kit0.3.2의 NSFontDescriptor/NSFont API와 [Apple NSFontWeightSemibold](https://developer.apple.com/documentation/appkit/nsfontweightsemibold?language=objc)를 대조하여 OS가 선택한 face/variation을 사용합니다. 지원하지 않는 내부 weight는 InvalidArgument로 거절합니다. OS 폰트를 저장소에 복사하거나 라이브러리/예산/lock을 변경하지 않았습니다.

공용 helper는 등록된 family가 없으면 기존 Proportional을 선택합니다. browser Canvas는 아직 원본 OS/CSS 폰트가 등록되지 않아 fallback이며 이 변경을 browser600 실현 또는 전체 글꼴 일치로 주장하지 않습니다.

## 검증

환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, cargo 실행 경로는 같은 디렉터리의 `bin/cargo`, target-dir는 `/private/tmp/taide-m8-menu-build.j6Efnw`입니다.

```sh
cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-editor snippet_dialog_제목은_등록된_semibold -- --nocapture
cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --lib ui_fonts::tests:: -- --nocapture
cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

- 새 실제 UI1: 최종 build.65초/suite.26초·17 filtered PASS. 등록/미등록 각각 한 연속 Scene에서4종 Dialog의 실제 galley family·크기·line-height·theme 색상을 확인했습니다. 등록 fixture는 이름 선택 경계의 검사이며600 실제 face 증거는 아래 native 검사입니다.
- 수정된 native 폰트2: 첫 실행 build21.31초/suite.96초·318 filtered PASS. 합성400/500/600 face 선택·공유 budget 소진/0-byte 거절과 OS 실제 face/variation·등록된 renderer glyph mesh를 확인했습니다.
- native build에는 기존 vendor wry deprecated/unused unsafe17개와 lib-test linker의 `__eh_frame` compact unwind 크기 경고1개가 있습니다. 경고0으로 표기하지 않습니다. 요청 밖 vendor 변경은 없습니다.
- normal Canvas Wasm check.86초 exit0/경고0입니다. 이전 Close 성공은 재사용하고 반복하지 않았습니다.

최초 테스트 작성 시 private Appearance.foreground 접근 compile 실패를 public presentation::color로 수정했습니다. 다음 실행에서 실제 family RED를 관찰했고 생산 family 연결 뒤 fixture가 잘못 사용한 `delete-file:rust.json` 식별자 실패를 실제 파일 선택→`delete-file` 경로로 정정했습니다. 서로 다른 원인이고 성공 검사는 반복하지 않았습니다.

## 남은 범위

전체 native/browser font raster·fallback·theme/DPI/AX/포커스, 실제 자동완성 UI/삽입과 전체 native shutdown은 미완료입니다. 이 좁은 family 경계는 구현/검증/기록3/3이고 부모 Snippets1/4(25%)·M8 363/433(83.83%)·최종0/8을 유지합니다. 전체 ETA는 산정 보류이며 전체 M8 완료 전 Git 작업은 없습니다. browser bindings/screenshot은 선행 lifecycle 소스입니다.
