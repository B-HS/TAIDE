# Snippets 반응형·목록·border-box 불일치

## 대상

`native/taide-native-ui/src/snippet-editor.rs`와 실제 `tests/snippet-editor.rs`입니다.

## 증상·원인

원본 Dialog는640px 미만에서 viewport-32px을 사용하지만 native는512px cap을 항상 적용했습니다. Close의 일반 scope가 부모 layout도 진행시켰습니다. 실제600px viewport rect520px은 원본568px과 달랐습니다. footer도 작은 화면에서 가로 행이었고 폐기 Alert는 원본 동일 두 열이 아니었습니다.

목록 제목은 존재하는 `snippetEditor.snippetListTitle` 대신 다른 키를 사용했습니다. 빈 파일 목록 안내가 없었고 파일명은 가운데 정렬·행 간격8px이었으며 입력 반경은 원본4px 대신2px이었습니다. sidebar256px 밖에 border1px을 추가하고 카드 내부 폭에서도 border를 다시 차감했습니다.

## 해결·근거

후속300px 폐기 Dialog는 기본 Dialog의 margin32px을 잘못 공유했으며 cap을 수정한 뒤에도 grid 셀보다 긴 버튼의 자연 글자 폭 때문에309.5px로 넘쳤습니다. 원본 minmax(0,1fr) 셀과 같은 명시적 text atom 폭을 적용해300px 실제 렌더가 통과했습니다. [후속 QA](../quality-assurance/2026-10-06-m8-snippet-outline-and-small-dialog.md)에 도형/Close focus·shadow와 실제 실패/성공을 기록했습니다.

원본 breakpoint/header/form/footer·절대 Close 배치를 연결하고 실제 CSS radius 매핑과 locale/list를 사용합니다. sidebar border를256px 안에 포함시키고 카드 이중 차감을 제거했습니다. 실제 responsive1 RED→GREEN, 목록/style1 및 변경된 카드/파일 geometry 확장1 PASS입니다. [정본 QA](../quality-assurance/2026-10-06-m8-snippet-responsive-layout.md)에 실제 명령·성공/fixture E0599와 남은 전체 시각/AX/Chrome 범위를 분리했습니다. M8 전체 완료로 표기하지 않습니다.
