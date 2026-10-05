# M8 탐색기의 원본 열기 설정·row context 생성

## 대상·원본

- 대상: `native/taide-native-app/src/explorer.rs`, `tests/explorer.rs`입니다.
- 원본: `src/features/explorer/{file-tree,file-tree-context-menu,explorer-shortcuts}`, `src/widgets/explorer/explorer-container.tsx`입니다. 실제 wrapper의 설정을 기능 callback 이름보다 우선했습니다.
- `explorer-container.tsx`의 `openRowFileTab(row, preview)`는 directory에 아무 동작을 하지 않습니다. onOpenPreview와 onOpenPinned 모두 이 함수에 false를 넘깁니다. 따라서 파일 단일 클릭·Space·Cmd+Down도 원본처럼 preview=false입니다. callback 이름만 보고 preview=true로 바꾸거나 TS의 설정을 수정하지 않습니다.

## 구현·검사

- [x] 기존 native 단일 클릭의 preview=true를 원본 false로 수정했습니다. 실제 pointer 클릭의 기존 입력 assertion도 원본 근거로 기대값을 변경했으며 테스트 조건을 제거하거나 검사를 끄지 않았습니다.
- [x] focused tree의 Space/Cmd+Down을 exact modifier 비교로 처리합니다. 현재 row가 file일 때만 non-preview open을 요청하며 선택이 없거나 directory이면 소비 후 열지 않습니다. popup이 열려 있으면 tree 키를 소비하지 않습니다. 기본 Enter/F2 이름 변경은 유지합니다.
- [x] row의 secondary click은 해당 경로를 선택합니다. 원본 context의 New File/New Folder를 기존 생성 흐름에 연결했습니다. 선택한 row가 사라진 경우의 root fallback·현재 parent 검사·접힌 directory 경로별 완료·IME/검증·실제 생성은 선행 구현을 재사용합니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer 탐색기_열기는 --locked --offline --target-dir experiments/native-shell-spike/target`: 최초 1건 실패, 0.02초입니다. 실제 출력은 Open preview=true였고 원본 기대값 false와 달랐습니다. 구현을 수정한 뒤 관련 1건만 0.02초에 통과했습니다. 클릭·Space·Cmd+Down 설정과 directory 비열기까지 검사합니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --test explorer 우클릭_생성은 --locked --offline --target-dir experiments/native-shell-spike/target`: 실제 headless secondary click→context item pointer 클릭→해당 directory inline focus→취소→새 id의 toolbar folder 생성 신규 1건 통과, 0.03초입니다.
- [x] lib/bin/explorer strict clippy 최종 exit 0, 0.67초입니다. 앞선 생성/실제 rename·Trash 성공 검사는 반복하지 않았습니다. fmt와 문서/diff 검사는 create QA의 최종 결과로 공유합니다.

## 아직 완료가 아닌 범위

- [ ] blank 영역 context/root 생성·전체 row-width hit·keyboard context opening/OS native menu·모든 submenu/action·context icon/shortcut label/geometry/theme·실제 픽셀·popup keyboard 탐색의 전체 검증은 남습니다.
- [ ] clipboard·paste·open-to-side/open-with/browser/history/compare/Git/auto reveal·multi-selection/typeahead와 전체 keymap은 후속입니다. 삭제 확인·menu/단축키·별도 GUI-confirmed Trash 경로의 좁은 연결과 결과는 `2026-10-01-m8-native-explorer-delete.md`에 기록합니다. 원본 탐색기 삭제에는 LSP의 dirty 거절이 없으므로 기존 목록의 “dirty 파일 거절”을 GUI 구현 계약으로 사용하지 않습니다. 전체 삭제 수명·실기 게이트는 미완료입니다.
- [ ] 실제 한글·일본어·중국어 입력기·VoiceOver는 사용자 담당 마지막 실기입니다. 앱/bundle·OS 설정·실제 clipboard나 Finder/terminal/browser는 조작하지 않았습니다. M8 전체와 최종 commit/push는 미완료입니다.
