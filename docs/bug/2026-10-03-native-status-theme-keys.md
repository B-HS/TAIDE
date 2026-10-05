# Native App 상태 색상 키 불일치

## 대상 파일

`native/taide-native-app/src/application.rs`, `keybinding-editor.rs`.

## 리포트

native keybindings 실제 UI 검사의 Appearance 생성에서 `native theme color is unavailable: status.warning`이 발생했습니다. raw bundled fixture 문제로 처음 판단했으나 runtime의 실제 theme_get에서도 같은 오류가 확인됐습니다. 원본 CSS의 status-warning/error는 theme의 `statusIndicator.warning/error`에 매핑됩니다. App initializer4곳과 새 keybindings Appearance가 존재하지 않는 status.warning/error를 조회하고 있었습니다.

## 수정·검증

정본 namespace로 조회 키를 수정했습니다. 실제 modal 검사1 PASS(0.56초), 기존 runtime 목록의 모든 builtin resolved theme 경계 신규1 PASS(0.06초), app lib/bin/test strict exit0(14.02초)입니다. core theme resolver/사용자 theme 파일을 바꾸거나 fixture 색상 alias를 주입하지 않았습니다. 보호 앱·실제 GUI 초기화는 실행하지 않았으며 keybinding-editor QA의 잔여 gate를 유지합니다.
