# M8 그룹 포커스·이동 keymap 기본 연결

## 대상·원본 계약

- `native/taide-native-app/src/shell_keymap.rs`, `native/taide-native-ui/src/commands.rs`가 대상이며 PROCESS N4-C의 남은41 action 연결 중 15개 그룹 동작을 main이 직접 구현했습니다.
- 원본 `src/shared/lib/pane-tree.ts`, `src/widgets/editor-area/group-shortcut-targets.ts`와 tests, `editor-area.tsx`의 handler를 읽었습니다. 방향은 실제 픽셀 거리가 아니라 가장 가까운 동일 축 ancestor의 sibling subtree를 선택합니다. 오른쪽/아래는 첫 leaf, 왼쪽/위는 마지막 leaf이며 가장자리에서 순환하지 않습니다.
- 번호 1~9는 depth-first leaf 순서이며 현재 pane/없는 번호는 no-op입니다. 그룹 포커스는 active tab이 없는 빈 그룹에서도 작동합니다. 탭 이동은 현재 active ID를 이웃 그룹 strip 끝 index에 보내며 기존 runtime/layout service가 고정 탭 영역 clamp·destination focus/active·정규화를 소유합니다.

## 구현·검사

- [x] 4방향 focus, 9개 번호 focus, left/right move의 15개 action을 공통 window registry와 기존 App→ShellIntent pipeline에 연결했습니다. registry는 Save 포함 기존 8개에서 23개이며 terminal command 이동 2개는 terminal 자체가 계속 소유합니다. 이 수치는 실행 경로 연결 수이지 전체 keymap/실기 완료율이 아닙니다.
- [x] 새 ShellMutation::MoveTab은 기존 `layout_actions::layout_move_tab`을 사용하며 direct tree rewrite나 사용자 pinned/dirty 상태 초기화를 하지 않습니다. destination tabs length는 checked u32 conversion을 사용합니다.
- [x] app `--lib group_keymap`은 신규 1 PASS(suite 0.00초·compile 4.28초)입니다. 실제 중첩 수평/수직 4그룹에서 nearest neighbor/entry leaf·가장자리·1~9/no-op·빈 active/no move·잘못된 focused ID, 원본 Cmd+K Cmd+Right resolver→typed focus와 실제 runtime dispatch를 확인했습니다. pinned active tab은 대상의 기존 pinned 뒤/일반 탭 앞에 들어가며 active/focus·다른 프로젝트 불변을 확인했습니다. 합성 상태의 전용 UUID data dir만 회수했습니다.
- [x] 처음 test compile은 local closure의 문자열 수명 추론 E0597로 실패해 실행되지 않았습니다. 입력 id를 `&str`로 명시한 뒤 해당 새 검사만 실행해 통과했습니다. 제품 로직 실패나 반복 실측으로 세지 않습니다. 같은 성공 상태를 다시 실행하지 않습니다.
- [x] 실제 App 배선 check lib exit 0(1.16초), app clippy lib strict exit 0(0.97초)입니다. 기존 window/editor/Save/PTY generic resolver와 Core 성공은 변경 없는 해당 경로의 QA에서 재사용합니다. Cargo는 기존 CARGO_HOME·locked/offline·격리 target으로 직렬 실행했습니다. 제품 TS/root/MSRV·보호 bundle·OS clipboard/browser/입력기/VoiceOver는 변경하지 않았고 새 package/unsafe/suppression은 없습니다.

## 남은 gate

- [ ] 이 검사는 실제 runtime mutation과 pure target 결정을 증명하며 NativeApplication 전체 창의 click/focus frame·Core view retarget/selection·aux/shell slot·capture/IME를 완료로 세지 않습니다. 실제 viewport/editor focus 검증과 lifecycle/queue-full/late layout은 후속 전체 gate입니다.
- [ ] Close All의 pinned 제외·dirty 일괄 확인/취소·순차 닫기, 나머지 APP_KEYMAP action과 palette/command rows, AppFile·raw Unicode/Text·aggregate budget은 남습니다.
- [ ] remount 정책 A/B는 응답 대기입니다. 전체 M8 N1~N8 0/8·213 view/cutover/TS 제거·성능/보안/배포는 미완료이며 전체 완료 뒤만 commit/push합니다.

## 정적 검사 기록

대상 Rust 2파일 exact fmt와 tracked diff check exit 0, native-ui lib strict exit 0(0.38초)입니다. 이 QA만 명시한 `--ignore-path /dev/null` Prettier write unchanged/check exit 0이며 docs를 무시하는 기본 실행을 문서 통과로 세지 않습니다. shell_keymap의 no-index whitespace 출력은 비어 있으며 /dev/null과 내용 차이 exit 1은 오류로 분류하지 않습니다.

## 다음 Close All의 확인된 경계

원본 `use-request-close-tab.tsx`는 여러 dirty tab에 한 번만 질문하고 Save는 모든 dirty 저장 성공 뒤에만 전체 close를 시작합니다. Save As 취소/readonly/write 실패는 전체 닫기를 취소합니다. Discard는 전체 dirty를 먼저 clean으로 만들고 Cancel은 아무 탭도 닫지 않습니다. `close-tabs-serially.ts`는 snapshot ID 순서로 닫으며 이미 사라진 NotFound는 건너뛰고 다른 실패를 수집합니다. 현재 NativeApplication은 `PendingTabClose` 한 개뿐이므로 단순한 tab별 확인 queue로 연결하지 않습니다. bulk 확인/저장 준비와 순차 닫기의 두 단계를 구현해야 하며 이번 group 연결 결과에 포함하지 않습니다.
