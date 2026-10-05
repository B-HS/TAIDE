# Snippets Dialog 첫 Tab·Shift-Tab 처리

## 대상·관찰

공용 `native/taide-native-ui/src/snippet-editor.rs`의 신규 실제 UI 연속 검사에서 첫 Tab이 Picker에 머물렀고, 이를 수정한 뒤 Shift-Tab이 Cancel 방향으로 이동했습니다. [Dialog QA](../quality-assurance/2026-10-06-m8-snippet-dialog-presence.md)에 각 실패/정정/최종 성공과 검사 범위를 기록했습니다.

## 원인·해결

1. SDK `set_focus_lock_filter`는 had_focus_last_frame와 has_focus를 모두 만족할 때만 적용합니다. `Response.request_focus` 뒤 별도 필터 설정은 처음 포커스를 받은 pass의 Tab을 보호하지 못합니다. 처음부터 실제 포커스와 필터를 `request_focus_with_filter`로 동시에 적용했습니다.
2. SDK `consume_key(NONE)`는 추가 Shift/Alt를 무시합니다. NONE을 먼저 소비하면 Shift-Tab도 정방향으로 처리됩니다. modifiers NONE/SHIFT만 정확히 구별해 해당 Tab만 소비하고 역방향 loop를 선택했습니다.
3. 오류 줄을 초기 포커스 assertion으로 잘못 해석해 불필요한 Picker trigger ID capture를 한 번 시도했습니다. 실제 실패는 다음 첫 Tab assertion이었으며 capture를 제거하고 SDK 원인으로 수정했습니다. 다음 유사 오류에서는 출력의 정확한 라인과 해당 assertion부터 대조합니다.

실제 연속 검사 최종1 PASS(.13초), 변화한 기존 입력2 PASS(.12초)입니다. SDK/vendor 변경·검사 억제·OS 입력기 변경·사용자 데이터 사용은 없으며 성공 검사는 반복하지 않았습니다.
