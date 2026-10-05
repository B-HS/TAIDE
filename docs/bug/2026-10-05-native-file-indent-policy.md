# Native 파일 들여쓰기 우선순위 차이

## 대상

`native/taide-native-app/src/application.rs`, `native/taide-native-ui/src/editor_surface.rs`, `native/taide-remote-web/src/browser-editor.rs`, `native/taide-native-editor/src/indent.rs`입니다.

## 관찰과 원인

원본 editorconfig.ts는 tab style에서 tab_width를 먼저 쓰지만 native formatter는 style에 관계없이 indent_size를 우선했습니다. 실제 파일 metadata에는 resolved EditorConfigOptions가 있어도 native/shared editor의 Tab은 global appearance.indent만 사용했습니다. 소스 대조로 확인한 차이이며 변경 전 실패 실행을 하지 않았으므로 실제 RED라고 주장하지 않습니다.

## 해결과 증거

공용 resolver로 style별 우선순위·미지정 축 보존을 구현하고 actual native formatter/show_document와 BrowserEditor.show_file을 연결했습니다. 원래 editor를 바꾸지 않고 파일별 copy의 실제 Tab 입력만 설정합니다. 새 core golden1·shared 실제 Tab1·Chrome-Wasm prepared-config1이 각각 처음 PASS이며 상세는 shared-file-indent-and-save QA입니다. 이 수정만으로 detected model·manual per-file state·화면 tab-stop까지 완료하지 않았으며 같은 QA의 잔여 게이트로 유지합니다.
