# Settings Popup의 옵션 Tab 이동과 즉시 종료

## 대상 파일

`native/taide-native-ui/src/settings-code-view.rs`, `tooltip-motion.rs`, vendored egui `src/containers/popup.rs`입니다.

## 리포트

원본 Radix/cmdk Popup은 옵션에 tabindex가 없고 FocusScope가 loop합니다. 기존 native는 옵션마다 `Sense::click()`으로 focus 후보를 등록해 Tab이 검색 입력에서 옵션으로 이동했습니다. close bool만으로 Area를 감추어 exit animation/닫힘 중 controls/지연된 focus 복귀도 사라졌습니다.

## 상세

새 연속 portable 검사로 첫 Tab의 실제 focus 이동을 RED 재현했습니다. 옵션을 focusable하지 않은 CLICK으로 바꾸고 검색 입력/옵션 Dialog에 Tab 소유 필터를 적용했습니다. open과 150ms Presence를 분리하고 기존 CSS ease motion의 opacity/scale을 Popup에도 사용하며 Area의 중복 fade-in을 끕니다. focusoutside/outside 클릭의 강제 복귀는 막고 정상 unmount 후 트리거 focus를 복구하며 레이어 transform을 회수합니다.

portable 연속/pure motion 및 새 Chrome/Wasm 연속 검사에서 해당 경계가 통과했습니다. Inspector early rect NaN과 disabled 재활성화 자동 focus 가정은 생산 버그와 구분해 QA에 기록했습니다. 전체 modifier 탐색·viewport edge clip·원본 전체 화면 재현은 아직 완료하지 않았습니다.

검증 정본: `docs/quality-assurance/2026-10-06-m8-settings-popup-presence.md`입니다.
