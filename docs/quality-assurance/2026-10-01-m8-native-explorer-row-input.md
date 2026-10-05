# M8 native 탐색기의 전체 행·빈 영역 입력

## 원본과 변경

- 원본은 `src/features/explorer/{file-tree,file-tree-row,file-tree-context-menu}.tsx`의 22px virtual row/12px indent/text-xs, rowAtClientY, 빈 영역 context 선택 해제와 root 생성, 빈 영역 double click, 클릭 selection modifier 판정입니다.
- 대상은 `native/taide-native-app/src/explorer.rs`, `tests/explorer.rs`입니다. 고정 egui 0.36.2의 UI allocation/interaction, ScrollAreaOutput/show_rows, WidgetInfo, pointer multi-click/ModifiersChanged source를 확인했습니다.
- 기존 이름 Label만이 아니라 indent 왼쪽과 오른쪽 끝까지 row 전체에서 선택/열기/우클릭을 받습니다. 22px 행 높이와 pitch를 유지하며 본문은 12px로 표시하고 잘라냅니다. selected/hover 배경은 전체 행에 그립니다. 원본 theme·icon/AX/tree role·정확한 ellipsis/픽셀 동등성은 별도 미완료입니다.
- scroll area의 실제 viewport에서 scrollbar를 제외하고 content offset·실제 visible row bottom을 함께 사용해 빈 영역을 구합니다. 원본처럼 빈 영역 우클릭은 선택을 지우며 row-scoped Rename/Delete를 노출하지 않습니다. New File/New Folder는 기존 root 생성 입력에 연결하고 double click은 root File 입력을 시작합니다. inline 입력 중에는 해당 blank response를 만들지 않습니다.
- 원본의 hasSelectionModifier는 Shift/Meta/Control입니다. native의 modifiers.any가 Alt까지 열기를 막던 차이를 수정했습니다. Alt 단독 클릭은 preview=false로 열고 Shift/Control/Command 클릭은 열지 않습니다. 아직 전체 multi-selection 상태를 구현한 것은 아닙니다.

## 실제 검사·실패

Cargo 공통 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다.

- [x] `cargo test ... --test explorer 탐색기_전체행과`: 최초 1건은 blank 시작 좌표가 실제 마지막 row와 겹쳐 실패, 0.02초였습니다. content 높이에 visible row bottom을 함께 적용했습니다. 후속 검사에서 geometry/root context까지 통과했지만 double click assertion은 실패, 0.03초였습니다.
- 고정 pointer source는 이전 두 click 시각을 사용해 triple click을 우선 판정합니다. fixture가 바로 앞의 메뉴 클릭과 새 blank gesture를 짧은 synthetic 시간에 묶었습니다. 새 gesture 이전에 1초의 결정적 input time만 분리했습니다. 제품 double click 판정을 약화하거나 실패 assertion을 삭제하지 않았고 실제 sleep/앱 재시작은 없습니다.
- [x] 위 새 입력 검사 최종 1건 PASS, 0.03초입니다. indent/오른쪽 끝 실제 pointer·22px height/pitch·blank 선택 해제·row Delete 비노출·root Folder 메뉴 입력·Escape·blank single/double click의 root File 입력을 확인했습니다. 같은 성공은 재실행하지 않았습니다.
- Alt fixture 최초 compile은 0.36.2 RawInput에 없는 modifiers 필드로 실패했습니다. 실제 Event::ModifiersChanged 경계로 수정했으며 compile 실패를 실행한 기능 검사로 세지 않습니다.
- [x] `cargo test ... --test explorer 탐색기_alt_click`: native actual output []/원본 기대 Open(false)로 RED, 0.02초였습니다. selection modifier 조건 수정 뒤 관련 1건만 PASS, 0.02초입니다. Alt 열기와 Shift/Control/Command 비열기를 같은 결정 검사에 담았습니다.
- [x] lib/bin/explorer strict clippy는 row/blank 변경에서 exit 0, 0.67초이며 후속 modifier/new fixture의 최종 결과도 exit 0, 0.63초입니다. 이는 입력/환경이 같은 재실행이 아니라 후속 코드 변경을 검사한 결과입니다. 이전 생성·rename·삭제·Trash·복구 성공은 반복하지 않았습니다.
- [x] app cargo fmt --check, 해당 QA Prettier, git diff --check는 exit 0입니다. PROCESS/HANDOFF는 관련 활성 상태만 갱신했고 무관한 전체 문서를 재포맷하지 않았습니다.

## 남은 전체 범위

- [ ] 빈 tree/아주 긴 virtual list의 actual scrollbar·부분 visible row·가로 폭/ellipsis, 폴더 row/double click, inline draft 위 context의 원본 세부 정책, pane/window별 focus scope와 keyboard context opening/모든 popup keyboard navigation은 남습니다.
- [ ] Shift/Meta/Control의 실제 multi-selection/anchor/range·typeahead와 전체 keymap, clipboard/paste·모든 context action/icon/shortcut label·Git badges/auto reveal·원본 theme/AX/tree semantics·실제 픽셀/OS GUI/IME/VoiceOver는 미완료입니다.
- [ ] 전체 213 view/editor/LSP/terminal·성능/보안/beta/rollback/배포·TS 제거/Rust99%와 M8 N1~N8은 미완료입니다.

검사는 headless egui 입력/geometry입니다. 사용자 실기 앱·bundle·OS 설정·실제 clipboard/파일/Trash를 조작하지 않았고 새로운 dependency나 commit/push는 없습니다.
