# Native terminal 마우스 이동·휠과 AX wire 순서

현재 상태: 알려진 AX 배치의 마우스 이동·tracking wheel·alternate wheel을 문자/포커스 stage에 연결했습니다. 클릭/Touch owner 이동·전체 graph/M8 완료는 아닙니다.

## 대상과 증상

`native/taide-native-app/src/terminal_surface.rs`, `tests/terminal-host.rs`, `native/taide-native-terminal/tests/fixtures/session.rs`입니다.

동일 session의 실제 두 view에서 IME commit `con`, 마우스 이동, SGR wheel, AX Focus, Text `tinue\n` 순서가 기대 wire와 달랐습니다. compile4.77초/suite0.44초/exit101이며 reverse=false의 child `Exited(Some(1))`이 기대 `Exited(Some(0))`과 달랐습니다. 앞선 E0308/E0063은 테스트 tuple/MouseWheel phase 작성 오류로, runtime RED와 구분합니다.

## 원인과 수정

모든 이동/휠을 wire barrier로 처리해 known AX stage가 해제됐습니다. 캡처 pointer는 별도 global drain에서 즉시 submit해 shared Outbox의 문자/Focus stage와 합쳐지지 않았습니다. 제거된 wheel은 다음 surviving event와 같은 index를 가져 그 index 안의 선후 관계도 필요했습니다.

마우스 이동·휠은 keyboard owner를 바꾸지 않는 기존 engine 계약을 사용합니다. 현재 viewport/frame의 알려진 AX 배치에서 캡처 packet을 `CapturedInput` phase로 stage하고, 같은 index의 Focus/Input보다 먼저 dispatch합니다. 여러 캡처 packet은 기존 전역 sequence와 stable sort로 순서를 유지합니다. pass 완료에서는 pointer를 먼저 stage한 뒤 합쳐진 입력을 전송합니다. 이전 frame·다른 viewport·unknown/클릭/Touch는 기존 경로를 유지합니다.

## 검증과 남은 범위

SGR motion/wheel exact wire 신규1건은 정/역 draw PASS(compile10.07초/suite0.40초)입니다. 별도 alternate wheel의 위/아래 두 key와 AX의 같은 surviving boundary FIFO 신규1건도 정/역 draw PASS(5.20초/0.46초)입니다. owned child/session/task를 회수했습니다. 최종 app lib/tests strict19.50초 exit0·authored Rust3 exactfmt exit0입니다. 명령·재사용 범위는 keymap-event-owner QA 마우스 이동/휠 절이 정본입니다.

실제 클릭/Touch의 owner 이동·current-pass disabled/modal/hidden/viewport·ordered attaching·full App/OS/GUI는 남습니다. 보호 앱·제품TS·engine/vendor·root/Tauri·의존성/manifest/lock/MSRV·OS·Git은 바꾸지 않았습니다.
