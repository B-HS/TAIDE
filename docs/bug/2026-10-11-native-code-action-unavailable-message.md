# 코드 수정의 빈 응답과 비활성 안내

## 증상과 재현

5ff13b75에서 수동 수정의 빈 응답이 빈 메뉴 상태로 남고, 공급자가 없으면 수동 명령이 아무 안내 없이 종료됐습니다. `notice-red.log`의 상태 검사 11건 중 두 건이 각각 `menu_open`과 요청 생성에서 실패했습니다.

## 원인과 수정

[상태](../../native/taide-native-app/src/editor-code-actions.rs)는 후보 목록과 수동 안내를 구분하지 않았습니다. 빈 수동 응답과 유효한 후보가 없는 Quick Fix에는 명령별 안내를, 한 개의 비활성 후보를 자동 적용하는 명령에는 서버 사유를 표시합니다. 자동 빈 응답은 조용히 유지합니다. 정상 또는 큰 문서의 공급자 없는 수동 명령도 빈 결과 안내를 만들며 읽기 전용·IME·공급자 수명 경계를 보존합니다.

[UI](../../native/taide-native-ui/src/editor-code-actions.rs)는 현재 커서 위치·본문 글꼴·원본 안내 테마를 사용하고 메뉴 포커스를 가져가지 않습니다. Escape·blur·창 포커스 상실·비활성화에서 안내를 회수하고 일반 본문 입력을 보존합니다. 실제 peek 안내의 Escape는 안내를 먼저 닫습니다.

## 확인

관련 App 검사 최초 20통과/1실패 뒤 실패한 한 검사만 통과해 서로 다른 21건을 확인했습니다. 실패한 검사는 UI 프레임 없이 peek 생성까지 기다리고 있었으며 기존 실제 화면/파일 읽기 순서로 고쳤습니다. UI 좁은 검사는 최초 12통과/1실패 뒤 입력한 `x`를 기대값에 반영한 한 검사가 통과했습니다. 컴파일 오류 두 건과 최초 실패·최종 전체 UI/동결 경계는 [배치 23 QA](../quality-assurance/2026-10-10-native-batch23-editor.md)에 기록합니다. 로그 접두사는 `/private/tmp/taide-batch23-code-actions-`입니다.

설치 Monaco의 `StandaloneBulkEditService.hasPreviewHandler()`는 false이며 apply의 preview 옵션을 소비하지 않습니다. 원본과 같은 수정 수락 키는 일반 적용으로 확인하며 별도 미리보기 화면을 추가하지 않습니다. 이 확인을 전체 code action 기능이나 실기 게이트 완료로 확대하지 않습니다.
