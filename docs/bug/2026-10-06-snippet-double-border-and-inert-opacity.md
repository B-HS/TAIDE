# Snippets 전환 구현 중 중복 border·inert opacity 경계

## 대상

`native/taide-native-ui/src/snippet-editor.rs`의 Button builder·add_button·paint_button_focus입니다.

## 원인과 수정

focus border를 매 프레임 따로 보간하면서 기본 Button의 app.border를 그대로 그리면 같은 border가 두 번 합성됩니다. 반투명 테마 또는 disabled opacity에서는 원본보다 진해집니다. 기본 Frame은1px transparent stroke로 원래 border-box/padding을 예약하고 보간 border만 한 번 그립니다. 테두리 폭을 제거해 레이아웃을 바꾸지 않습니다.

Dialog Presence 동안 배경의 inert Ui는 disabled alpha1로 입력만 막습니다. 그 부모 상태를 먼저 선택하면 실제 isDeleting=false→true 버튼의 disabled50%가 무시됩니다. 자신의 enabled=false를 우선 적용하고 부모 opacity는 따로 보존합니다. Modal 입력 차단이나 닫힘 Presence를 우회하지 않습니다.

두 항목은 이번 전환 구현 중 코드 대조에서 발견·수정한 경계이며 원본 TypeScript 오류로 분류하지 않습니다. 실제 반투명 border 한 번·삭제 요청·닫히는 Dialog와75/150ms border/shadow를 확인한 검사1 PASS(1.27초/.12초), 변경 후 normal Canvas Wasm.55초 exit0/경고0입니다. [검증 정본](../quality-assurance/2026-10-06-m8-snippet-button-state-motion.md)에 명령과 나머지 범위를 기록했습니다.
