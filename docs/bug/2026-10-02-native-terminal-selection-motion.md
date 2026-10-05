# Native terminal 출력·reflow 뒤 선택 이동

## 대상

`native/taide-native-app/src/terminal_surface.rs`와 native terminal Core/Alacritty Grid·Term의 선택 이동/trim 경계입니다.

## 실제 재현

처음 화면의 `first`를 선택하고 한 줄 새 출력을 보내면 같은 viewport-relative Point가 새 `second` 행에 붙어 `secon`을 복사했습니다. headless actual Core 검사에서 `secon`≠`first` RED를 확인했습니다. 이후 긴 24셀 단어를 선택하고 가로 폭을 12→6으로 바꾸며 history가 trim되면 초기 단어 길이가 6셀로 축소됐습니다. `ghijkl`≠`ghijklmnopqrstuvwx\nline2` RED입니다.

## 원인·수정

view 좌표만 저장하고 실제 top scroll/history trim을 추적하지 않았습니다. Word/Line도 원래 최소 셀 길이·optional raw end 대신 현재 inclusive range만 저장했습니다. 실제 Grid origin/trim·Term buffer/rows/input epoch를 같은 Core stamp로 연결하고 독립 view가 copy/paint/pointer 전에 조정합니다. Word/Line은 원래 start/길이/end를 보존하며 기존 xterm model의 trim/끝점 정책을 사용합니다. 출력마다 선택을 지우거나 새로운 grid를 만들지 않습니다.

## 검증·잔여

재현 두 건은 수정 뒤 각각 1 PASS(0.03/0.00초)입니다. 후속 half-open 끝점에서 `cdef`≠`cdef\n` RED도 재현했습니다. 원래 Point/Side의 bounded half-open copy와 실제 셀 highlight를 분리해 newline·폭 밖 column을 보존하며 신규 slice/render 2건 PASS(0.00/0.00초)입니다. 고유 신규 총 9건·실제 모델/copy 변경의 영향 검사·native/app 최종 strict exit 0을 `docs/quality-assurance/2026-10-02-m8-native-terminal-selection-lifetime.md`에 기록했습니다. 전체 block/Unicode/VT·auto-scroll/OS/terminal/N4/M8 완료나 OS clipboard 성공을 주장하지 않습니다.
