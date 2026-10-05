# Native terminal menu label 검색 누락

대상은 `native/taide-native-app/src/terminal_surface.rs`·`tests/terminal-host.rs`입니다.

## 관찰과 원인

actual 영어 locale/UI/owned PTY 검사에서 `c` 뒤 disabled Copy를 건너뛰어 Clear로 이동하지 않고 실제 AX focus가 owner에 남았습니다(compile4.39초/suite0.33초/exit101). 기존 메뉴에는 방향키 탐색만 있고 원본 Radix MenuContent의 검색 상태/getNextMatch 경로가 없었습니다.

## 수정

actual enabled item의 locale label과 root/child별1초 search 수명을 연결합니다. 반복 문자·현재 label 제외/순환·case-insensitive prefix와 검색 중 Space의 item/SubTrigger 선택 억제를 유지합니다. UTF-16 한 단위 Text는 실제 살아 있는 선행 pressed Key에만 연결하며 ctrl/alt/command/mac_cmd·IME Commit/Paste/standalone/astral Text를 character keydown으로 취급하지 않습니다. 메뉴가 직접 억제한 Space index만 별도 허용하고 같은 frame pass의 query 중복도 시작 snapshot으로 방지합니다.

## 검증과 한계

신규1 PASS(compile12.04초/suite0.51초/exit0)·consumed-Key 추가 경계 뒤 관련1 PASS(8.86초/0.53초/exit0)·탐색/Enter-Space/action 영향3 PASS(0.19초)·app lib/tests strict4.19초·Rust2 exactfmt/diff입니다. owned PTY cleanup과19 actual focus snapshots를 확인했습니다. 정확한 명령·locale fixture 오류/E0277·검증 범위/성공 재사용은 context-menu QA 맨 위 typeahead 절이 정본입니다.

leaf default action/host·같은 raw batch·dynamic topology/partial disabled·강제 discard·재열기/locale/Unicode/backend·전체 App/OS/CJK/VoiceOver·M8 게이트는 아직 미완료입니다. 기본 검색 성공을 전체 원본 parity 완료로 바꾸지 않습니다.
