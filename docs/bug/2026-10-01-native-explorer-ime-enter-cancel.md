# Native 탐색기 IME Enter의 이름 변경 취소

## 대상·재현

- 대상: `native/taide-native-app/src/explorer.rs`의 inline 이름 입력, `tests/explorer.rs`의 입력 프레임 검사입니다.
- 실제 egui Context에서 이름 입력을 시작하고 `ImeEvent::Preedit`와 Enter key를 같은 frame에 전달하면 rename action은 없지만 입력 상태가 삭제됐습니다. 최초 기능 검사에서 `explorer.rename.is_some()` assertion이 실패했습니다.
- 사용자 OS 입력기나 실기 앱을 조작하지 않았으며 합성 native event만 사용했습니다.

## 원인·수정

- 커스텀 rename key 처리에서 조합 중 키를 실행하지 않는 것만으로는 부족했습니다. 고정 egui의 singleline TextEdit은 전달받은 Enter를 입력 종료로 처리하고 포커스를 해제합니다. 이 상태가 blur 취소로 이어졌습니다.
- focused 입력의 Enter/Escape는 먼저 consume하고, 조합 중이거나 IME commit이 있는 frame에는 rename 확정/취소 액션만 실행하지 않습니다. IME text event는 TextEdit에 그대로 전달합니다.
- 관련 검사에서 IME 단계 수정 후 후속 Escape 취소 assertion이 실패했습니다. `lost_focus` 응답을 처리하면서 현재 frame에도 `owns_focus`가 참일 것을 요구하던 조건을 제거했습니다. 새 입력 시작·처리 중 latch·disabled UI 구간은 별도로 보호합니다.
- 중간 수정에서 더 이상 읽히지 않는 focus 대입도 제거해 strict warning을 없앴습니다. 검사기를 끄거나 기대값을 완화하지 않았습니다.

## 실제 결과·남은 경계

- 수정한 관련 입력 1건은 0.02초에 통과했습니다. 전체 이름 선택·일반 Enter·중복 요청·오류 유지·조합/commit Enter 비실행·Escape·포커스 이탈·빈 이름 취소를 검사합니다. 별도 실제 파일 이동/새 tree 선택 통합 1건은 이전 성공 결과를 재사용합니다.
- 최종 app/explorer/lsp strict clippy exit 0, 0.57초입니다.
- 실제 시스템 IME의 event 순서·commit 후 포커스·VoiceOver·OS 메뉴/키맵 검증은 아직 수행하지 않았습니다. 해당 실기 게이트의 완료 근거로 사용하지 않습니다.
