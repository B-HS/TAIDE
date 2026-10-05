# M8 native 에디터 글자 크기 keymap

## 대상·원본 근거

- 대상은 `native/taide-native-app/src/{shell_keymap,application,host,presentation,terminal_surface}.rs`, `native/taide-native-ui/src/commands.rs`입니다. PROCESS N4-C의 기존41 action 범위에서 main이 직접 구현했습니다.
- 원본 `KeybindingsRuntimeProvider`, `use-editor-font-size.ts`, `code-font-size.ts`를 대조했습니다. font-size-up/down은 project/editor/terminal focus와 무관한 전역 binding이며 editorFontSize만 1씩 바꾸고 6~48로 제한합니다. terminalFontSize나 전체 UI zoom을 바꾸지 않습니다.
- 원본 settings command/runtime의 저장→live 상태 교체→통합 toggle observer→SettingsChanged 순서를 읽었습니다. IDE/agent/remote observer는 각 enable flag가 같으면 no-op입니다. 이번 host는 editor_font_size만 patch하며 일반 Settings/통합 toggle 구현을 대체하지 않습니다.
- 실제 egui/NativeEditor와 고정 epaint0.36.2 Galley text/size API source를 읽었습니다. 기존 line-height1.5 배율과 appearance를 재사용합니다.

## 구현

- [x] 공통 resolver→ChangeEditorFontSize intent는 project가 없어도 소비합니다. App frame의 현재 editorFontSize에서 원본 step/clamp 계산 후 bounded HostCommand::SetEditorFontSize(u32)를 제출합니다. 처리 대기 중 여러 입력의 target capture/coalescing은 전체 입력 gate에서 확인하며 worker가 임의로 size를 누적하는 정책을 만들지 않았습니다.
- [x] 기존 runtime settings_update와 SettingsPatch의 editor_font_size만 사용합니다. 저장 실패는 기존 HostReply::Failed/status 경로로 전달하며 live 상태/SettingsChanged를 먼저 성공시킨 뒤 디스크 오류를 숨기지 않습니다. 통합 flag를 변경하지 않는 이 전용 명령의 callback은 no-op이며 native 일반 Settings와 통합 자원 설정은 별도 미완료입니다.
- [x] App의 frame reconciliation에서 설정의 editorFontSize를 기존 NativeEditor font.size/line_height에 적용하고 실제 변화가 있을 때만 repaint합니다. 같은 size는 no-op이며 font family·colors·padding·Core 문서/undo/선택을 바꾸지 않습니다. terminal appearance의 기존 독립 reconciliation은 유지합니다.
- [x] dependency/unsafe/suppression/code comment·제품 TS/root/MSRV·보호 bundle·사용자 데이터·OS 설정/clipboard를 변경하지 않았습니다.

## 실제 검사

Cargo는 기존 CARGO_HOME·locked/offline·격리 `experiments/native-shell-spike/target`으로 직렬 실행했습니다.

1. app `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib font_keymap -- --nocapture` 신규1 PASS(compile4.91초·suite0.09초)입니다. 최초 test fixture의 E0063(ViewKey.pane)·E0609(ViewState.selection)·E0599(DocumentSnapshot.rope)를 실제 타입/source로 정정했으며 그 compile 실패는 제품 RED 재현이나 성공 검사로 세지 않습니다.
2. 한 검사에서 실제 egui의 no-project Cmd+=/Minus 입력 소비/단일 action, 전역 intent·6/48 clamp·unsigned 경계·step을 확인했습니다. 실제 HostBridge의 증가/복귀/상하한 저장·SettingsChanged·전체 Settings equality(터미널/다른 설정 불변)·합성 settings.json의 재파싱을 검사했습니다.
3. 실제 NativeEditor를 48/6으로 렌더해 해당 텍스트 shape의 Galley 높이가 달라지고 동일 적용은 no-op이며 family/color·Core revision/text/선택이 유지됨을 확인했습니다. 이것은 headless shape/layout 검사이며 OS 실제 픽셀/AX 증거가 아닙니다.
4. 소유한 합성 settings.json을 보관 경로로 옮기고 같은 경로를 directory로 만든 write 실패에서 Failed reply·live Settings 불변·추가 SettingsChanged 없음·worker 종료/task0을 확인했습니다. 실제 사용자 settings/.env/키 파일은 읽거나 쓰지 않았으며 fixture는 검사 종료 시 정리했습니다.

app lib strict clippy exit0(1.60초), 변경6파일 exact rustfmt check·추적diff exit0입니다. 기존 Wry17 dependency warning을 authored strict 실패로 세지 않습니다. 이전 terminal/reopen/group/Save/Close All/editor 입력 성공은 재사용하며 통과한 검사를 반복하지 않았습니다.

## 남은 gate

- [ ] 실제 NativeApplication의 입력→host reply/event→다음 frame·여러 editor pane/aux 창·shared settings·terminal focus/IME/custom override·빠른 연속 key/queued·포화/late/exit·파일 재실행 복원을 확인합니다. 합성 component 검사만으로 실제 제품 창 전체를 완료했다고 주장하지 않습니다.
- [ ] status bar font control/reset·일반 Settings UI/통합 observers·theme/font family·전체 입력/500ms chord 상태/Monaco21·palette/OS·성능/보안/배포를 구현·검증합니다.
- [ ] 현재29 shell action+terminal 자체2 이동은 기본 연결 수입니다. 나머지10 공통 action·전체M8 N1~N8 0/8·213view/cutover/TS제거는 미완료이며 remount A/B는 응답 대기입니다. 전체완료 뒤만 commit/push합니다.

## 문서 검사

기본 Prettier의 docs 제외 설정 대신 이 QA만 `--ignore-path /dev/null`로 실제 포맷 검사합니다. untracked authored 파일의 no-index whitespace 출력과 내용 차이 exit1을 구분합니다.
