# Native transpose의 잘못된 UTF-16 단위 처리

사용자 결정: 2026-10-09 "잘못된 단위를 대체 문자 U+FFFD로 변환".

Monaco `editor.action.transpose`는 선택한 범위의 UTF-16 단위를 역순으로 바꿉니다. 이때 단독 surrogate가 생기는 경우 Rust UTF-8 문서에 저장하기 전에 U+FFFD로 변환합니다. 유효한 surrogate 쌍은 정상 문자로 보존합니다. `transposeLetters`의 문자소 단위 동작은 별도로 유지합니다.

기준은 설치된 Monaco 0.56의 `TransposeAction`, `TextModel.validateRange`, 선택 추적의 `nodeAcceptEdit`, `PieceTreeTextBuffer`의 개행 정규화입니다. 변환 후 내용과 커서 위치를 원본 실행 기준값으로 검증합니다. 찾기 엔진·테마 색 값 등 이전 결정 대기 항목은 이 합의에 포함되지 않습니다.
