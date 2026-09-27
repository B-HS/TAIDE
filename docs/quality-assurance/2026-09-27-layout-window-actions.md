# 탭 창 이동·복귀 검증 경계

## 대상 파일

- `crates/taide-runtime/src/layout_actions.rs`
- `crates/taide-runtime/tests/layout_window_actions.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/domain/window/commands.rs`

## 리포트

- [x] UUID 임시 데이터와 주입 callback으로 guard 대기, 새 창 선생성, 생성 실패 무변경, main/기존 창 이동, 빈 슬롯 정리, registry 비회수와 이벤트/state 순서를 검사했습니다. runtime integration 8건이 통과했습니다.
- [x] mirror 유무·조회 실패·중복 file 병합·untitled dirty·main active 유지·이미 닫힌 프로젝트/슬롯 무호출을 검사했습니다. mirror는 fixture 경로에만 쓰고 조회 후 제거하지 않는 기존 정책을 확인했습니다.
- [ ] 실제 OS 창 선생성/close와 close 실패 처리, 보조 창 닫힘 flush handshake·registry 회수·다중 창 복귀를 M7 사용자 실기로 확인합니다.
- [ ] 새 창 생성 뒤 실제 이동 실패가 도달 가능한 정책으로 바뀌면 실패를 먼저 재현하고 callback rollback과 OS 창 잔류 여부를 검사합니다.

## 상세

새 창 action은 mutation guard를 얻고 해당 탭을 먼저 찾은 뒤 local layout clone만 수정합니다. 실제 창 생성 adapter는 layout을 변경하지 않습니다. 현재 정상 layout에서는 선생성 뒤 `move_tab_to_new_window`의 방어적 실패를 외부 입력으로 유도할 수 없으므로, 오직 실패 테스트를 위해 product mutation hook을 추가하지 않았습니다.

순수 service의 없는 탭 실패에서 새 layout 슬롯을 회수하는 검사는 실행했습니다. action의 move 오류 뒤 close 호출과 성공 뒤 빈 창 정리는 기존 본문 대조 및 순서 검사로 고정했습니다. 이는 실제 OS 창 rollback 성공을 실행 검증한 것이 아닙니다. 생성 callback의 행동이나 layout 이동 전제조건이 변경될 때 이 테스트 부채를 다시 평가합니다.

현재 요청에서는 앱 실행/재시작·실제 창 생성/닫기를 수행하지 않습니다. synthetic callback은 OS adapter를 대체하고, close 오류를 무시하는 기존 adapter body만 검사합니다. OS close가 실패하면 실제 창이 남을 수 있는 위험과 해당 실기 gate는 미완료로 유지합니다. mirror 조회 실패를 빈 목록으로 처리하는 동작도 변경하지 않았으며, 그때 file dirty를 지우는 기존 정책을 별도 실패 fixture로 확인했습니다.

이 결과는 M6 전체 완료, M7 전체 workspace/frontend/GUI gate, M8 native UI 착수 gate를 대신하지 않습니다.
