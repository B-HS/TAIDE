# 메뉴 owner의 탐색 뒤 Enter가 같은 batch에서 선택하지 않음

대상: actual `native/taide-native-app/src/terminal_surface.rs`의 navigate_menu·root/child choice 연결입니다.

source Content owner End는 동기 Kill focus를 만들고 뒤 Enter가 Kill을 선택합니다. native는 버튼 뒤 탐색만 실행해 메뉴를 열어 둡니다. 실제 source 합성 task1회와 actual UI/owned PTY RED(4.51초/0.38초)를 대조했습니다.

navigate에서 surviving raw Enter/Space의 당시 enabled item ID를 반환하고 실제 current root/child Response ID의 MenuAction에 매핑합니다. typed command/actual button·root 동기/항목 지연 이동을 유지합니다. 같은 메뉴 활성 batch의 후행 문자는 기존 terminal admission으로 계속 차단합니다.

신규1 PASS(4.35초/0.19초)·root/host/mixed Space 영향3 PASS(0.36초)·strict3.93초입니다. fixture의 불필요한 후행 Escape/enum Debug/import 오류는 제품 RED와 구분합니다. 정본은 context-menu QA owner 탐색 뒤 선택 절이며 전체 여러 action/동적 topology/원본 Presence native timing/App graph 완료가 아닙니다.
