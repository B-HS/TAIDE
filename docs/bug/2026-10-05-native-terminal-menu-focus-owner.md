# Native terminal 메뉴의 keyboard owner와 focus wire 누락

현재 상태: 기본 secondary press→메뉴 owner/loss→global release→Escape 복귀/gain을 실제 PTY plain/SGR에서 수정·검증했습니다. 메뉴 전체 AX tree/navigation·연속/동일-frame 닫힘·current topology/실기·full M8는 미완료입니다.

대상은 native app `src/terminal_surface.rs`, `tests/terminal-host.rs`, terminal queue fixture와 native-only vendor egui의 `src/{context,pass_state}.rs`입니다. 원본 실제 두 TS 컴포넌트 측정은 context-menu QA 눌림 절의 단일 자료를 재사용했습니다.

눌림 열기만 수정한 native는 local 입력을 mask했지만 actual keyboard focus는 terminal에 남았고 Focus1004 loss/gain이 없었습니다. 실제 owned PTY의 원본 보고/입력 순서 검사는 child 완료 timeout RED(compile7.39초/suite3.44초)였습니다. 첫 hit의 pointer terminal focus와 그 직후 menu focus를 같은 요청 하나로 처리하면 SGR press와 blur 순서도 보존할 수 없습니다.

pass 수명의 trigger→menu owner 선언을 실제 이전 viewport hit로 검증해 raw event 전/후 요청 cache를 분리했습니다. normalized/raw local route·Button·App window scope·최종 Memory·ordered wire가 이를 공유합니다. 실제 focusable menu widget에 root role/focus action을 붙이고 press report 다음에 menu loss phase를 전송합니다. generic global shortcut을 막지 않으며 뒤 AX가 최종 owner이면 초기 메뉴 focus를 다시 덮지 않습니다. closed 선언 단독으로 새 owner를 입장시키지 않습니다. global release는 기존 captured mouse 경로로 유지합니다.

최종 actual PTY plain/SGR 검사1건0.39초, window prefix/menu generic scope/final AX 검사1건0.02초, 메뉴 영향3건0.15초·engine/app strict28.11초가 PASS입니다. 코드·정확한 wire/명령·성공 재사용·fixture/새 build 경로·한계는 `quality-assurance/2026-10-03-m8-native-terminal-context-menu.md`의 focus owner 절이 정본입니다. 단순 root role 설정과 keyboard owner 성공을 실제 AX parent/items/VoiceOver 완료로 바꾸지 않습니다.

기존 Cargo target/debug/deps 디렉터리 열거에 두 실행이 정체됐습니다. 소유 rustc 샘플86/85회가 getdirentries64였고 sandbox 제거로 해결되지 않았습니다. 각각 확인된 이번 build PID만 정상 SIGINT/exit130으로 종료하고 기존 cache를 보존한 채 Cargo의 새 임시 target 옵션으로1분13초 빌드했습니다. compile/typecheck 자체 실패나 제품 테스트 결과로 세지 않습니다. 다음 검사는 `/private/tmp/taide-m8-menu-build.j6Efnw`를 재사용합니다. root/Tauri·manifest/lock·개인 profile/키·OS/clipboard·보호 앱·Git은 변경하지 않았습니다.
