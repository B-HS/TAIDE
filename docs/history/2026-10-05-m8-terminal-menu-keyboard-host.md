# M8 terminal menu leaf keyboard·actual host 검증

대상 변경은 `native/taide-native-app/tests/terminal-host.rs`·관련 문서입니다. 제품 Rust/TS와 engine/vendor는 수정하지 않았습니다.

## 확인과 정정

ordinary Button의 Space keyup을 메뉴에 그대로 적용한다는 이전 추정은 철회했습니다. 실제 patched Button은 `UiKind::Menu`에서 ordinary arm/release를 제외하고 기존 context key_pressed 경로를 사용합니다. 원본 MenuItem과 같은 Enter/Space keydown 선택이 이미 구현되어 있었습니다.

실제 root UI의 disabled copy→selectAll/copy/clear/paste·keydown 닫힘/복귀/PTY exit0와 actual child4방향/new/kill→typed command→bounded HostBridge worker→layout/Closed reply/owned child 회수를 검사로 고정했습니다. clipboard/environment는 합성 port이며 사용자 OS/데이터를 사용하지 않습니다.

## 증거

신규2 PASS: root4.56초/.64초, actual host4.24초/.31초입니다. fixture 공통화 영향2 PASS(.14초), terminal-host strict.89초·Rust1 exactfmt/diff입니다. Center match/E0502/초기 Resize queue fixture 오류는 제품 RED와 구분합니다. 정확한 명령·재사용·한계는 context-menu QA 맨 위 leaf Enter/Space·host worker 절이 정본입니다.

현재 leaf 단계4/4(100%)·M8 checklist346/413(83.78%, 공수비 아님)·최종0/8·goal active입니다. 전체 ETA는 산정 보류이며 raw mixed/current topology·full App/OS/cutover/Rust99%가 남습니다. 완전한 M8 완료 전 commit/push하지 않습니다.
