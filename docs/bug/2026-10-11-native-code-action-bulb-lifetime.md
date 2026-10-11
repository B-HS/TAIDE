# 자동 전구의 공백 왕복과 수정 목록 취소

## 증상과 재현

92cbb325에서 코드 위치의 자동 수정 응답을 받은 뒤 공백으로 이동했다가 같은 위치로 돌아오면 전구가 복원되지 않았습니다. 전구의 수정 목록을 Escape로 닫아도 후보가 함께 폐기되어 다시 열 수 없었습니다. 소유 임시 App와 순수 상태의 bulb-red.log 두 검사가 모두 실패했습니다.

## 원인과 수정

[controller](../../native/taide-native-app/src/editor-code-actions-controller.rs)의 공백 분기는 표시/대기 요청만 회수하고 observed를 남겼습니다. 취소를 제외한 Request.matches가 같은 위치를 이미 조회한 것으로 보아 다음 자동 요청을 막았습니다. 공백 분기에 observed 회수를 추가했습니다.

[상태](../../native/taide-native-app/src/editor-code-actions.rs)의 close는 목록 닫기와 작업 취소를 같이 처리했습니다. 완료한 후보를 선택 전 닫을 때는 menu_open만 내리고, 대기/resolve 취소·문서/뷰/공급자 만료·적용 완료의 회수는 유지했습니다. 설치 Monaco controller의 onHide도 편집기 포커스와 장식을 회수하며 후보를 폐기하지 않습니다.

## 확인

nearby-fixed.log의 관련 17건 중 순수 상태/실제 App가 공백 왕복 뒤 새 요청·Escape 뒤 같은 후보/UUID 재열기·16프레임 재요청 없음·선택 적용/원 디스크·종료 task 0을 확인했습니다. 전체 기능이나 전구 위치/겹침/OS 실기 완료의 근거로 확대하지 않습니다. 상세 실패·명령·범위는 [배치 23 QA](../quality-assurance/2026-10-10-native-batch23-editor.md)의 c5c 후속 절에 보존했습니다.
