# Native terminal copy의 출력 공백·빈 칸 구분

## 대상·관찰

`native/taide-native-app/src/terminal_surface.rs`, `native/taide-native-terminal/src/{lib,selection_text}.rs`, native Alacritty fork의 `term/{cell,mod}.rs`입니다.

합성 `abc   `를 출력하고 12열 전체를 선택한 신규 `selection_copy` 검사에서 이전 수동 추출은 `abc         `를 반환했습니다. 기대값 `abc   `와 달라 실제 RED였으며, 출력하지 않은 오른쪽 빈 칸까지 복사한 원인입니다. 단순 `trim_end`는 실제 출력된 세 공백까지 삭제하므로 해결이 아닙니다.

## 원인·수정

설치된 xterm `SelectionService.selectionText`는 `BufferLine.getTrimmedLength`의 HAS_CONTENT를 기준으로 출력된 공백과 untouched null cell을 구분합니다. Alacritty 기본 Cell은 둘 다 `' '`이고 기존 native view는 매 셀을 끝까지 더했습니다. native-retained 전용으로 기존 u16 Flags의 남은 bit에 NATIVE_CONTENT를 보존합니다. 실제 출력은 표시하고 erase/reset·wide cleanup은 기존 경로에서 제거하며, reflow의 occupied length에도 반영합니다. 새 Cell 필드·별도 parser·전체 화면 복사본은 추가하지 않습니다.

같은 Core의 immutable grid에서 Simple/Block 선택을 bounded 문자열로 추출합니다. 실제 공백·wrap·wide/NFD를 보존하고 TAB/NBSP는 xterm처럼 공백으로, untouched 오른쪽 빈 칸은 제외합니다. 바이트/방문 상한 초과는 오류이며 view에 표시합니다. 오류를 무시하거나 부분 문자열 성공으로 바꾸지 않습니다.

## 검증·제한

Core 신규 1 PASS(0.00초), 영향 app copy 1 PASS(0.00초), borrowed paint 1 PASS(0.01초), 실제 자기 PTY/headless 전체 선택·복사 출력 1 PASS(0.16초)입니다. PTY 검사 최초의 `starts_with` 가정은 시작 대기 입력도 같은 PTY에 보낸 기존 fixture와 맞지 않아 실패했고 준비 문자열 포함 조건으로 수정했습니다. 특정 prefix의 실제 내용이나 OS clipboard 동작을 검증했다고 주장하지 않습니다.

단어/줄 선택·선택 anchor의 출력/trim/resize 수명·검색/context/마우스 전체·OS clipboard·전체 N4/M8은 아직 미완료입니다. 명령과 나머지 gate는 `docs/quality-assurance/2026-10-02-m8-native-terminal-selection.md`에 기록합니다.
