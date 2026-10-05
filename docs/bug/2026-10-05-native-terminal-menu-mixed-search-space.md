# 같은 batch 검색 문자 뒤 Space의 잘못된 메뉴 선택

대상: `native/taide-native-app/src/terminal_surface.rs`의 prepare_menu_search·actual menu default buttons입니다.

원본 actual Clear focus에서 같은 JS task의 s→Space는 searchRef가 즉시 갱신되어 선택하지 않고 메뉴를 유지한 채 지연 focus가 Split으로 이동합니다. native는 query가 빈 프레임의 모든 Space를 default button에 남겨 Clear를 실행하고 메뉴를 닫았습니다. 실제 owned PTY/AX 검사에서 missing actual item tab.split로 재현했습니다.

버튼보다 먼저 surviving raw Key/Text·modifier/UTF-16·사건별 owner/window loss를 스캔해 Space 억제 index를 정합니다. 실제 query/focus 갱신과 프레임 시작 복원은 기존 navigate에 유지합니다. 신규1 PASS(11.51초/0.38초)·검색/일반 선택 영향2 PASS(0.34초)·strict2.01초입니다.

정본: `docs/quality-assurance/2026-10-03-m8-native-terminal-context-menu.md`의 같은 batch 절입니다. source 키는 합성 trusted=false이며 물리 시간/전체 mixed/default action/App graph 완료 증거가 아닙니다.
