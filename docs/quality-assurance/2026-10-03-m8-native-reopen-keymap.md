# M8 native 닫은 탭 다시 열기 keymap

## 대상·원본 근거

- 대상은 `native/taide-native-app/src/{shell_keymap,application,terminal_surface}.rs`, `native/taide-native-ui/src/commands.rs`입니다. PROCESS N4-C 기존41 action 중 reopen-closed-tab을 main이 직접 연결했습니다.
- 원본 command-palette의 handler는 project가 없으면 no-op이며 useReopenClosedTab→layout.ipc→layout_reopen_closed를 사용합니다. 현재 창의 pane에 강제로 새 탭을 만들지 않고 기존 project 닫힌 탭 스택을 재사용합니다.
- 기존 `taide-layout::service::reopen_closed`는 LIFO·원래 pane 존재 시 복귀·없으면 main focused pane/첫 leaf·pinned-zone index 보정·tab ID/kind/title/state·닫힘 시 dirty 정규화와 revision을 이미 소유합니다. 새 복원 알고리즘이나 직접 문서/PTY 생성은 추가하지 않았습니다.

## 구현·검사

- [x] 기존 APP_KEYMAP의 Cmd/Ctrl+Shift+T→ShellMutation::ReopenClosed(ProjectId)→기존 runtime mutation에 연결했습니다. active tab이 없어도 project 요청은 가능하며 프로젝트 없는 global handler는 원본처럼 키 소비 후 no-op입니다. 전체 NativeApplication/aux 입력 검증까지 완료했다고 주장하지 않습니다.
- [x] app `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib reopen_keymap -- --nocapture` 신규1 PASS(compile5.97초·suite0.00초)입니다. 합성 탭을 실제 기존 close service로 닫은 뒤 두 번 typed dispatcher→runtime로 복원했습니다. Cmd+Shift+T resolver·focused project·LIFO·원래 ID/kind/title·dirty=false/preview=false·사라진 pane fallback·각 revision+1·타 프로젝트 불변·빈 stack 무변경·없는 active 요청을 확인했습니다.
- [x] app lib strict clippy exit0(1.37초), 변경4파일 exact rustfmt check/추적diff exit0입니다. 기존 Wry17 dependency warning과 authored 실패를 구분합니다. Cargo는 기존 CARGO_HOME·locked/offline·격리 target으로 직렬 실행했습니다.
- [x] 이전 terminal-tab-keymap/실제 no-project egui gate와 기존 group/Close All/Save/editor·runtime tab 수명 성공은 재사용했습니다. 변경 없는 successful 검사를 반복하지 않았고 dependency/unsafe/suppression/comment·보호 bundle·제품 TS/root/MSRV·OS 설정·clipboard/사용자 파일은 변경하지 않았습니다.

## 남은 gate

- [ ] actual App의 controller snapshot→loading·같은 ID의 새 editor view·문서/draft/PTY attach/restart·reveal/focus·빠른 연속 재열기/queued command·프로젝트 교체/닫힘·no-project capture·aux 창 복귀를 검증합니다. 이 검사는 layout/runtime 기본 연결이며 실제 restored text/terminal process를 확인하지 않았습니다.
- [ ] 기존 service pinned-zone/view-state 복원·volatile tab 거절·aux tree target의 전체 기능 및 실제 OS/IME/AX·aggregate는 기존 M8 gate에 유지합니다. root service를 수정하지 않았으며 해당 동작 전체를 신규 검사가 증명했다고 확대하지 않습니다.
- [ ] 현재27 shell action+terminal 자체2 이동은 기본 연결 수입니다. 나머지12 공통 action·전체 Monaco21/팔레트·N1~N8 0/8·213view/cutover/TS제거·성능/보안/배포는 미완료입니다. remount A/B는 응답 대기이며 전체M8 완료 뒤만 commit/push합니다.

## 문서 검사

docs 제외 기본 설정을 우회해 이 QA를 `prettier --ignore-path /dev/null`로 실제 포맷 검사합니다. untracked 파일의 no-index 내용 차이 exit1은 빈 whitespace 출력과 구분합니다.
