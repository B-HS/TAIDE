# Native ThemeEditor reset과 hex blur의 적용 순서

## 대상 파일

- `native/taide-native-app/src/theme-editor.rs`
- `native/taide-native-app/src/theme-editor-tests.rs`

## 리포트

열린 색상 popup의 hex 입력을 편집한 뒤 reset을 누르면, 기본색 대신 아직 commit하지 않은 입력색이 남았습니다. 원본은 blur commit 다음 click reset이 최종 값을 결정하지만 immediate renderer는 reset 버튼을 먼저 그리며 즉시 기본색으로 바꾼 뒤, 뒤에 그리는 picker의 lost-focus 결과를 다시 draft에 적용했습니다.

## 재현과 해결

1. 합성 `taide-dark` duplicate의 `app.accent`를 `#ff0000`으로 바꿉니다. 검색 `accent`에서 실제 picker trigger를 열고 hex에 `#00ff00`을 입력합니다. blur 전 draft는 `#ff0000`입니다.
2. 기본값 reset 버튼을 누르면 최초 검사는 `left #00ff00 / right #89b4fa`로 실패했습니다. `cargo test … --lib native_theme_editor는_picker -- --test-threads=1` exit101, suite0.10초/컴파일3.47초입니다.
3. color/syntax 행은 reset 클릭 의도를 보존하고 picker의 blur 결과를 먼저 처리한 뒤 해당 reset을 마지막에 적용합니다. picker 검사기/입력을 끄거나 popup을 강제로 없애지 않습니다.
4. 동일 실제 입력 검사 1건이 수정 후 PASS(exit0, suite0.10초/컴파일2.42초)입니다. 정상 입력의 색상 commit과 base reset을 모두 유지하며 저장된 theme/OS 설정을 변경하지 않습니다.

syntax의 동일 코드 순서도 수정했지만 hex blur와 syntax reset 동시 입력의 별도 실측은 하지 않았습니다. 원본 전체 pixel/AX·NativeApplication 실기와 M8 완료로 확대하지 않습니다.
