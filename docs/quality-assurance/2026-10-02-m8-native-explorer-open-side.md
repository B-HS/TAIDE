# M8 native 탐색기 옆으로 열기

## 범위와 기준

원본 `src/widgets/explorer/open-to-the-side-plan.ts`, `explorer-container.tsx`, `src/features/explorer/file-tree-context-menu.tsx`와 실제 공통 `taide-runtime::layout_actions::layout_open_tab_in_split`, `taide-layout::service::{insert_new_leaf,normalize}`를 대조했습니다.

파일 행에만 locale `explorer.openToTheSide` 메뉴를 표시합니다. row 이름/경로, 선택 project, 현재 창의 focused pane, right edge, preview=false를 기존 OpenTabInSplitRequest로 전달합니다. 별도의 open-then-split·kind dedupe·임의 단축키·새 의존성은 추가하지 않았습니다. 레이아웃이 아직 없거나 auxiliary slot/project가 일치하지 않으면 명령을 만들지 않습니다.

대상은 native app `src/{explorer,host,application}.rs`, `tests/{explorer,host}.rs`입니다. actual AppSurfaces가 해당 row project의 snapshot layout와 NativeShell.scope를 사용하므로 다른 활성 project's focused pane을 잘못 사용하지 않습니다. typed HostCommand::OpenFileToSide는 기존 bounded host worker에서 원본 runtime의 단일 mutation을 호출하고 기존 Failed 회신/status 경로를 유지합니다.

## 실제 검증

- [x] 초기 실제 egui 메뉴/요청 검사 1건 PASS(0.05초·compile 4.17초)입니다. 파일 메뉴 클릭의 typed action, right edge/preview=false/project/pane/name/path, directory·missing layout·missing auxiliary slot 거절을 확인했습니다.
- 최초 host 검사는 default_layout에 Welcome/Terminal 탭이 있다는 것을 빈 pane fixture가 반영하지 않아 timeout FAIL(3.01초)이었습니다. 실제 empty leaf로 바꿨습니다. 이후 empty root의 split 기대 FAIL(0.01초)을 통해 공통 normalize가 원래 empty pane을 제거한다는 실제 원본 동작을 확인했습니다. 제품 서비스를 수정하지 않았습니다.
- [x] empty leaf·실제 auxiliary focus/다른 project 거절을 추가한 최종 메뉴/요청 검사 1건 PASS(0.05초·compile 1.13초)입니다. 이 성공은 host 실패 뒤에도 재실행하지 않았습니다. 실제 OS auxiliary Explorer를 연 검사는 아닙니다.
- [x] 원본 normalize 기대를 반영한 실제 host 영향 검사 `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test host 실제_host_옆으로열기는`: 1건 PASS(0.01초·compile 0.80초)입니다. empty pane은 기존 공통 정책대로 새 파일 pane 하나로 접히며, 그 파일을 다시 옆으로 열면 다른 ID의 두 탭과 두 pane이 유지됩니다. 각 요청은 revision을 한 번 올리고 기존 파일 tab ID를 유지합니다. root 밖 경로의 Failed 회신/레이아웃 무변경·disk 내용·disconnect/shutdown/tracked_count 0도 확인했습니다.
- [x] app lib/bin/explorer/host strict clippy exit 0(1.03초)입니다. 구현과 typed host/surface를 검사한 뒤 마지막 fixture의 원본 empty-pane 기대만 정정했으며 해당 fixture compile은 위 최종 실행으로 확인했습니다. 관련 없는 기존 Cut/Copy/Trash/PTY·OS 계측/앱 재시작은 반복하지 않았습니다.

## 설명과 실제 코드의 차이

TypeScript plan의 주석/순수 테스트는 empty group에서도 split 요청을 만드는 것을 설명하지만, 실제 Rust 서비스의 insert_new_leaf는 normalize를 호출하고 normalize_owned는 empty leaf를 제거합니다. 요청이 만들어지는 것과 두 pane이 실제 유지되는 것은 다릅니다. M8은 기존 실제 동작을 보존했으며 공통 서비스에 새로운 empty-pane 보존 정책을 넣지 않았습니다. 향후 이 원본 동작을 바꾸려면 M8 parity와 분리해 UI 계약·저장/복구·split/close 전체 정책을 검토해야 합니다.

## 남은 경계

- [ ] 실제 Shell snapshot 갱신/새 editor focus·pane별 독립 view·원본 icon/메뉴 픽셀/scroll·IME/VoiceOver·OS 다중 창·auxiliary 실제 Explorer는 전체 host/window gate에서 확인합니다.
- [ ] Open With의 provider/override와 preview surface·나머지 context action·전체 213 view와 N1부터 N8은 미완료입니다.

실제 사용자 앱·실기 bundle·clipboard·파일·OS 설정은 변경하지 않았습니다. 합성 fixture만 사용하고 자동 정리했습니다. TS 제거·root dependency/MSRV 변경·전체 M8 완료·commit/push는 수행하지 않았습니다.
