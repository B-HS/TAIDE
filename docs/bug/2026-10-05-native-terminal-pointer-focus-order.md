# Native terminal 누름 포커스·기본 버튼 prefix·늦은 AX 순서

현재 상태: primary/middle 누름의 알려진 실제 terminal owner를 연결했고, 늦은 AX를 완료 click이 덮는 오류를 수정했습니다. secondary 메뉴·Touch·현재 topology·전체 M8 완료는 아닙니다.

## 대상과 재현

대상은 `native/taide-native-app/src/terminal_surface.rs`, vendor egui-input의 `src/{context,pass_state}.rs`·`src/memory/mod.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다.

실제 동일 session 두 view에서 IME commit `con`, terminal1 누름, `tinue\n`, 해제 순서의 정확한 PTY wire가 실패했습니다. 최초 compile4.85초/suite0.37초/exit101이며 reverse=false child exit1이 기대 exit0과 달랐습니다. source xterm6.0.0 `CoreBrowserTerminal.bindMouse`는 mousedown에서 먼저 focus하고 그 뒤 mouse report를 보냅니다. 기존 native의 완료 click/drag focus와 pointer unknown barrier로는 이 계약을 표현하지 못했습니다.

## 수정과 추가 원인

직전 pass에 실제 running/enabled terminal만 pointer focus를 선언합니다. 엔진은 실제 이전 widgets·영역 순서·transform·input region·modal 허용 여부의 정확한 hit-test로 각 raw 누름의 target을 계산합니다. viewport/pass 수명의 선언과 viewport raw 요청 cache를 keymap·local owner·wire·최종 Memory focus가 공유합니다. 해제는 keyboard focus를 옮기지 않습니다. pointer loss/gain phase는 같은 index의 mouse report보다 먼저 전송하고, 제거된 wheel의 기존 CapturedInput phase는 다음 survivor의 AX보다 먼저 유지합니다.

최종 Memory focus를 미리 반영하자 이전 Button의 Enter prefix가 사라지는 별도 RED가 있었습니다(compile13.67초/suite0.01초/exit101). 일반 Button도 입력 시작 focus snapshot과 raw 요청 index를 읽도록 수정해 prefix를 보존했습니다.

실제 terminal의 완료 click 뒤 AX가 왔을 때 aggregate `response.request_focus()`가 그 AX를 덮는 별도 RED도 확인했습니다(compile13.17초/suite0.37초/exit101, 최종 Memory ID가 terminal1/기대 terminal0). aggregate click/drag는 알려진 사건별 최종 focus가 false이면 다시 요청하지 않습니다. explicit App request와 unknown fallback은 유지합니다. 새 fixture mode8의 mouse-report 활성화 누락은 검증 fixture에서 정정했습니다. 해당 RED는 wire 통과 주장 없이 실제 Memory ID 불일치로 기록합니다.

## 확인 범위

primary/middle 실제 PTY 정/역 draw1건, 이전 Button prefix 정/역1건, declared/enabled/cover의 hit/owner1건, pointer·AX 순서의 window scope1건, 실제 완료 click 뒤 AX의 Memory와 정확한 wire 정/역1건이 PASS입니다. 기존 일반 Button AX·Space 취소 영향2건도 PASS입니다. 각 명령·시간·strict·재사용 기준은 `docs/quality-assurance/2026-10-05-m8-keymap-event-owner.md`의 terminal mousedown 절이 정본입니다.

기존 no-AX binaryFF/liveSGR·backpressure/replay/discard PTY 영향1건은 새 prepared stage 분기로 바뀌어 재사용하지 않았습니다. 중간 epoch 기대3/실제8 RED는 이전 writer-submit 경계의 테스트 기대였습니다. stage admission에서 첫6개 입력을 이미 record하고 다음 `d`는 pending인 계약에 맞춰 중간 기대만 정정했습니다. 제품 epoch 구현 불변·수정 뒤 compile7.09초/suite0.60초/exit0에서 exact wire·최종7회·child exit0·owned join을 확인했습니다.

window scope fixture의 첫 실패는 기존 keydown capture가 keyup도 제거한다고 기대한 테스트 오류였습니다. 기존 keydown 전용 router를 확인하고 surviving release 순서만 정정했으며 제품 keyup 처리는 바꾸지 않았습니다.

secondary 메뉴의 같은 배치 입력 차단·Touch·새/현재 disabled/hidden/viewport topology·editor/generic pointer·App composition·전체 GUI/source parity는 미완료입니다. 보호 앱·사용자 프로필·OS 설정·clipboard·Keychain·제품TS·root/Tauri·의존성·manifest/lock/MSRV·Git을 변경하지 않았습니다.
