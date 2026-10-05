# 스니펫 stale choice 검사 순서와 인접 mirror 회수

## 대상 파일

- `native/taide-native-editor/src/snippet-session.rs`
- `native/taide-native-ui/tests/snippet-session.rs`

## 리포트

choice 세션을 남겨 둔 채 문서를 외부에서 지우면 active_choice가 revision 검사보다 이전 placeholder 범위 변환을 먼저 수행했습니다. 새 합성 검사는 StaleRevision 기대값에서 RED였으며 owner 확인 뒤 revision을 먼저 검증하도록 수정한 후1회 PASS했습니다.

단일 cursor의 `${1:漢}$1-${2:end}$0`를 편집·다음/이전 그룹 이동 후 인접 mirror를 삭제하면 같은 위치의 caret 두 개가 남아 placeholder 수 검사에 맞아 세션이 계속 활성 상태였습니다. 설치된 원본 CursorCollection.normalize는 collapsed caret이 접하면 합칩니다. Session의 정렬된 편집 결과에서 동일 caret을 제거하고 primary를 보존하도록 수정했습니다. 선택이탈로 세션을 회수할 때 원본 SnippetController2._updateState처럼 undo 경계를 닫아 동일 group 후속 편집도 합쳐지지 않도록 했습니다. 새 재현 검사1건은 수정 뒤1회 PASS했습니다.

## 상세와 범위

이 수정은 Session 자체 편집 후 결과와 choice 조회에 한정됩니다. 기존 일반 editing planner의 병합 정책이나 전역 SelectionSet은 변경하지 않았습니다. 전체 NativeEditor cursor normalization·외부 편집 decoration 관찰·undo alternative version 및 실제 입력 controller 연결이 완료됐다고 주장하지 않습니다. 성공한 원본 mapping/session14 사례는 영향을 받지 않는 범위의 증거로 재사용했습니다.
