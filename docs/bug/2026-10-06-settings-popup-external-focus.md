# Settings Popup 닫힘 중 외부 focus 복귀 오류

## 대상 파일·관찰

`native/taide-native-ui/src/settings-code-view.rs`의 Picker입니다. Snippets NewFile에서 global 옵션을 선택하면 새 입력기가 자동 focus되지만 닫히는 Popup이 완료 시 trigger로 focus를 되돌려 실제 Text 입력이 사라졌습니다. 250ms slow frame에서 Presence deadline이 지난 경우도 동일하게 재현했습니다.

## 원인·해결

열린 Popup에만 외부 focus guard를 적용하면 closing motion을 회수할 때 원본 Radix의 outside-interaction 계약을 놓칩니다. `self.open || self.motion.is_some()` 동안 content_focused/현재 focus/owned 후보를 확인하고 외부 focus이면 open과 restore_focus를 해제합니다. 시간 만료의 was_present 값에 의존하지 않습니다.

## 증거·남은 검증

신규 Snippets 실제 Picker/global 자동 focus·IME Escape·disabled·create 검사 slow250ms RED→GREEN1.09초/.12초입니다. 기존 Popup Tab/검색/옵션/닫힘 Presence/trigger 복귀 영향1 PASS3.65초/.14초는 첫 guard 시점이며 owned-focus 경로는 최종 조건에서도 불변입니다. 최종 native check10.49초/Canvas Wasm1.13초 exit0입니다. 실제 브라우저/full AX/OS 검증은 별도 미완료입니다.
