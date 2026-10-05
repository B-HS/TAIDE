# Native terminal submenu 키보드 진입과 관계

대상은 `native/taide-native-app/src/terminal_surface.rs`·`tests/terminal-host.rs`, `vendor/egui-input/src/containers/menu.rs`입니다.

원본 Radix SubTrigger는 Enter/Space/Right로 열고 first enabled 방향을 focus하며 Left로 trigger에 복귀합니다. 기존 native SubMenu는 click/hover만 처리했고 실제 child Menu 관계·entry focus가 없었습니다. Right 뒤 expanded None 및 Enter 뒤 focus!=first item을 실제 AX/owned PTY 검사에서 재현했습니다. 최초 MenuItem!=Menu 실패는 새 fixture의 root-only assertion 재사용 오류로 구분합니다.

기존 SubMenu stack에 pass별 optional open override를 추가하고 실제 입력 시작 trigger/item/child owner에 opener/Left를 배정합니다. 실제 visible enabled item에 pending focus를 적용하며 네 방향 모두 disabled면 actual focusable child Menu에 focus합니다. 실제 child parent·labelled_by와 trigger expanded/controls를 연결하고 닫힌 item/pending을 회수합니다. 원본 source 구현/공식 문서를 대조했으며 별도 popup/cache·의존성·unsafe·OS API 우회는 없습니다.

신규3 PASS: Right/Left(12.40초/0.40초), Enter/Space(7.85초/0.51초), all-disabled(7.58초/0.32초)입니다. root AX/actual actions 영향2 PASS(0.42초), engine/app strict3.86초·fmt/parse/diff exit0입니다. 정확한 명령·실패·성공 재사용은 context-menu QA 실제 submenu 절이 정본입니다.

전체 roving/typeahead·mixed current topology·leaf host 왕복·bounds/시각/OS GUI/M8 완료는 아닙니다. M8 0/8·목표active·전체완료 전 commit/push 없음입니다.
