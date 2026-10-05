# Native terminal menu Shift+Tab 예약 이동

대상은 `native/taide-native-app/src/terminal_surface.rs`·`tests/terminal-host.rs`입니다.

## 관찰과 원인

실제 root/child 기본 탐색 초안의 신규 연속 UI/owned PTY 검사는 root index9 Shift+Tab 뒤 actual AX focus가 SelectAll 대신 다른 항목을 가리켜 실패했습니다(compile12.55초/suite0.49초/exit101). 원본 Radix MenuContent는 모든 Tab을 preventDefault합니다.

고정 egui0.36.2 `memory/mod.rs`의 `interested_in_focus`는 Shift+Tab을 그리기 중 `id_next_frame`에 예약합니다. 후반 메뉴 navigation에서 key를 소비하고 `move_focus(None)`·`request_focus`를 호출해도 이미 생긴 다음-pass 예약이 남습니다. 사용자가 OS 설정을 바꾸거나 테스트를 반복해서 해결할 문제는 아닙니다.

## 해결과 증거

실제 focused 메뉴 owner/item에 egui `EventFilter`의 tab/horizontal_arrows/vertical_arrows를 유지하고 메뉴의 actual enabled 순서로 탐색합니다. 다음 입력의 generic navigation 예약을 막되 Escape와 default action 경로는 그대로 유지합니다. 새 engine API나 fake focus tree는 추가하지 않았습니다.

관련 신규1 검사만 재실행해 root12/child10의 서로 다른 입력이 PASS(compile4.10초/suite0.48초/exit0)했고 submenu Enter/Space·all-disabled owner·실제 copy/select/clear/paste 영향3도 PASS(suite0.15초/exit0)입니다. 앱 lib/tests strict3.64초/exit0·authored2 exactfmt입니다. 정확한 명령과 성공 재사용은 `docs/quality-assurance/2026-10-03-m8-native-terminal-context-menu.md` 맨 위 기본 탐색 절이 정본입니다.

첫/동적 focus와 Tab이 같은 raw batch에 있는 예약·여러 navigation/default action의 source setTimeout 순서·전체 App/실기 graph는 이번 완료 범위가 아니며 기존 M8 메뉴 부모에 미완료로 유지합니다.
