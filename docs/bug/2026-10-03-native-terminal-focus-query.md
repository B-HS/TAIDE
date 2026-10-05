# 포커스 보고 모드 활성화 시 현재 상태 응답 누락

## 대상·원인

native Alacritty `src/term/mod.rs`의 DECSET 1004가 mode bit만 켰습니다. 제품의 xterm은 최초·반복 활성화 때마다 현재 focus를 보고하므로, native에서는 같은 프로그램이 현재 상태 응답을 받지 못했습니다. 최소 SharedTerminal 검사에서 빈 effect 목록과 기대 FocusOut의 불일치를 재현했습니다.

## 수정·결과

SharedTerminal의 focus 관찰을 기존 Term 상태에 기록하고 native-retained의 1004 handler가 해당 시점의 상태를 bounded PtyWrite effect로 생성합니다. 입력 승인/선택 epoch와 실제 UI focus 관찰을 분리했습니다. feature 밖 동작과 기존 이벤트 메모리 예산을 유지합니다.

최소 query 1건과 실제 PTY의 최초 Out을 포함한 Focus 11개+x·종료/회수가 통과했습니다. 정확한 명령·결과와 남은 포화·query/input 동시 순서는 `docs/quality-assurance/2026-10-03-m8-native-terminal-focus-query.md`에 기록합니다.
