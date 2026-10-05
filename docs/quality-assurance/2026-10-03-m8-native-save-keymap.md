# M8 저장 keymap과 등록된 native editor 연결

## 대상·근거

- 대상은 `native/taide-native-app/src/{application,shell_keymap,save}.rs`, `native/taide-native-ui/src/{commands,editor_surface}.rs`, `native/taide-native-ui/tests/editor_surface.rs`입니다. PROCESS N4-C의 Save keymap 통합 범위를 main이 직접 수행했습니다.
- 원본 `focused-editor-tab.ts`와 `editor-area.tsx`는 focused shell의 active File/AppFile/Untitled 중 등록된 editor instance에서만 `taide.saveFile`을 실행합니다. `code-editor.tsx`의 해당 action에는 별도 keybinding이 없고 APP_KEYMAP이 저장 키를 소유합니다. 사라진 파일 draft는 CodeEditor가 아닌 읽기 전용 pre와 Save As 버튼이므로 전역 Save는 no-op입니다.
- native의 external keymap 경로에는 내부 Cmd/Ctrl+S fallback을 두지 않습니다. standalone `show` wrapper의 기존 API 동작만 유지하며 앱은 `show_with_keymap`을 사용합니다. Save는 창 공통 결정기로만 선택하고 기존 `HostCommand::Save`의 participant/format/cleanup·revision/epoch 경로로 전달합니다.

## 구현·검증

- [x] shell registry의 8번째 action인 Save를 `RequestSaveTab`으로 연결했습니다. 실제 화면에서 이번 frame에 성공적으로 표시한 editor TabId→DocumentId만 등록합니다. preview/loading/missing draft/지원하지 않는 AppFile surface는 등록되지 않아 이전 retained Core를 잘못 저장하지 않습니다. File document는 실제 files map과 DocumentKey를, Untitled는 정확한 TabId를 확인합니다.
- [x] 실제 NativeEditor의 external keymap이 예전 Cmd/Ctrl+S를 저장하는 RED를 재현했습니다. UI `--test editor_surface external_keymap`은 수정 전 1 FAIL(suite 0.02초·compile 0.69초), 수정 뒤 1 PASS(suite 0.02초·compile 1.15초)입니다. 동일 입력의 standalone wrapper는 기존 Save 요청을 유지합니다. 이후 항상 false인 내부 결과 필드만 제거했고 아래 실제 앱 component 검사로 외부 경로를 다시 확인했습니다.
- [x] app `--lib save_keymap`은 1 PASS(suite 0.09초·compile 5.47초)입니다. 실제 NativeEditor/공통 창 keymap의 기본 Save, bare J override와 인접 Text 소비, override 뒤 예전 Cmd/Ctrl+S no-op, extra Shift no-op, 합성 IME J/漢 commit과 2-pass를 한 번 검사했습니다. action은 정확히 2개이며 Core 내용은 `漢draft`입니다. keyup은 원본 Key/Text 뒤에 넣어 repeat 합성을 피했습니다.
- [x] 같은 검사에서 focused File의 RequestSaveTab, 미등록 document no-op, 등록 File의 snapshot→기존 prepare→승인 HostBridge Save→실제 디스크 `漢draft`와 mark_saved, readonly 오류, Untitled Save As용 typed 요청/다른 TabId 거절, File에 Untitled Core 거절, Terminal의 Save no-op을 확인했습니다. 시스템 Save As dialog나 실제 앱 창을 실행한 것으로 확대하지 않습니다. 전용 UUID 임시 디렉터리만 회수하고 bridge disconnect/TaskSupervisor 0을 확인했습니다.
- [x] app check lib exit 0(1.24초), app clippy lib `-D warnings` exit 0(2.14초), native-ui lib/editor_surface strict exit 0(0.63초)입니다. Cargo는 기존 `CARGO_HOME`, `--locked --offline --target-dir experiments/native-shell-spike/target`으로 직렬 실행했습니다. 기존 Wry 경고 17개는 authored strict와 구분하며 새 의존성/unsafe/suppression은 없습니다.

기존 save participant/EditorConfig/format·draft/close·window-keymap/actual PTY/ruler/Core의 변경 없는 성공은 각 QA에서 재사용합니다. 동일 성공을 세 번 반복하지 않았습니다. 실제 이번 frame registry·NativeApplication 배선은 compile와 코드 경로로 확인하며 이 검사는 NativeApplication 전체 창의 입력 capture 순서를 증명하지 않습니다.

## 남은 전체 gate

- [ ] AppFile editor/승인된 app-owned 저장, 남은 APP_KEYMAP action과 palette·command rows/Monaco 명령·chord 표시/no-match를 연결합니다. native AppFile surface가 없으므로 Save 전체 parity 완료로 세지 않습니다.
- [ ] 실제 Main/aux 창·shell slot·focus 변경/capture ordering·중복 listener 부수 효과·비ASCII/Copy/Cut/Paste·키맵 aggregate budget과 실제 Untitled Save As dialog 취소/동시 저장을 확인합니다. 합성 IME는 OS 입력기/VoiceOver 검증이 아닙니다.
- [ ] 원본 remount replay와 canonical Core 단일 파싱의 A/B 응답을 기다립니다. 보호 bundle/OS clipboard/browser/input method/VoiceOver·제품 TS/root/MSRV는 그대로입니다. 전체 M8 N1~N8 0/8·213 view/cutover/TS 제거·성능/보안/배포와 전체 완료 뒤 commit/push 조건은 유지합니다.

## 정적 기록

최종 authored Rust 6파일의 exact rustfmt check와 tracked diff check exit 0입니다. 프로젝트 `.prettierignore`가 docs를 제외하므로 이 QA만 `--ignore-path /dev/null`로 명시해 Prettier write exit 0(unchanged)와 check exit 0을 확인했습니다. authored Rust 6파일과 이 QA의 no-index whitespace 출력은 모두 비어 있습니다. 내용 차이 exit 1은 오류로 분류하지 않습니다.
