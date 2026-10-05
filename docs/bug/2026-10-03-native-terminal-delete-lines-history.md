# Native terminal DL의 삭제된 명령 marker 이력 잔류

## 대상 파일

`native/taide-native-terminal/vendor/alacritty-terminal/src/term/mod.rs`, `src/grid/mod.rs`, `native/taide-native-terminal/tests/commands.rs`입니다.

## 증상과 원인

OSC133 A/C/D로 첫 두 행에 block을 만든 뒤 cursor를 첫 행으로 옮겨 CSI M을 실행하면 block이 1개 대신 2개 남았습니다. Alacritty의 delete-lines는 scroll-up relative를 재사용하며 region start가 0이면 삭제된 행도 history에 저장합니다. 제품 xterm의 DL은 buffer splice 삭제이고 history를 만들지 않아 start marker도 폐기됩니다.

## 수정과 확인

native-retained에서만 제한 region의 전체 Row swap/reset을 수행하는 delete-lines 경계를 사용합니다. history/display offset은 유지하고 삭제 행은 reset으로 marker liveness를 폐기합니다. selection/vi cursor 회전·damage 처리를 유지하며 IL/DL의 cursor column 0·pending wrap 해제를 제품 source와 맞췄습니다. feature-off upstream handler는 보존합니다.

최초 수명 검사에서 2≠1 실패를 확인한 뒤 같은 filter의 최종 1 PASS(0.03초)입니다. 후속 resize fixture는 상대 화면 행과 절대 buffer 행을 혼동해 정정했으며 해당 과정은 terminal-commands QA에 기록합니다. 전체 region/selection/reflow·실제 OS VT 동등성은 아직 완료가 아닙니다.
