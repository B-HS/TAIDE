# 전환 배치 2 통합 검증 (2026-10-06)

## 범위

배치 2는 기존 실패 테스트 정리, 명령 레지스트리, 커맨드 팔레트의 세 단계입니다. 단계별 상세는 `2026-10-06-native-batch2-{test-health,command-registry,command-palette}.md`입니다. 세 단계 모두 리뷰 판정은 pass(차단 항목 0)였습니다.

## 메인이 직접 실행한 검증

| 검사 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib --locked --offline` (배치 2 최종 상태) | 336 통과, 0 실패(13.93초). 배치 시작 기준은 313 통과 7 실패 |
| `cargo fmt -- --check` (app, ui) | exit 0 |
| `crates/taide-infra/src/pty.rs` 변경의 공개 시그니처 | 변경 없음(`git diff`에 `pub` 줄 없음) |
| 삭제한 낡은 테스트가 실제 홈의 사용자 레벨 훅 파일을 건드렸는지 | 대상 4개 경로 중 존재하는 것은 `~/.codex/hooks.json` 하나이고 수정 시각이 2026-09-23이라 오늘 테스트 실행으로 바뀌지 않았습니다 |

단계별 작업자가 보고한 나머지 검증(native-ui lib 87건, host 8건, editor-reveal 2건, taide-infra lib 296건, terminal-host 연속 5회 52건)은 이후 입력이 바뀌지 않아 재실행하지 않았습니다.

## 화면 확인

- [ ] 커맨드 팔레트 외형(너비 512px, 입력 행 36px, 항목 32/48px, 목록 300px, 선택 행, 강조), 배치 1의 테마 적용·타이틀바·welcome — 검증 시점에도 화면이 잠겨 있어(최상위 창 소유자 `ScreenLock`) 캡처하지 못했습니다.

## 사용자 결정이 필요한 사항

1. **전역 ⌘N**: `application.rs`에 새 untitled 파일을 만드는 하드코딩 ⌘N이 있습니다. TS에는 전역 ⌘N이 없고(⌘N은 탐색기 포커스 한정 새 파일, untitled는 탭바 + 메뉴) 이번에는 그대로 두었습니다. 유지, 제거, 탐색기 한정 중 선택이 필요합니다.
2. **실행 경로가 없는 명령의 노출**: 등록 명령 212개 중 177개는 native 실행 경로가 아직 없습니다. 키바인딩 편집기는 TS처럼 전부 편집 가능한 행으로 보여 주고(바인딩해도 실행되지 않음), 팔레트는 흐리게 표시해 실행을 막습니다. 기능이 구현될 때까지 편집기에서도 숨기거나 비활성 표시할지 결정이 필요합니다.
3. **팔레트가 열린 동안의 배경**: 셸 전체를 비활성으로 그려 배경이 흐려집니다. TS는 scrim만 덮습니다. 실기 확인 뒤 맞출지 결정합니다.

## 다음 배치로 넘긴 것

- 팔레트: 고정 아이콘 5종, 열림·닫힘 애니메이션, `@`·`#` 모드의 LSP 조회, 파일 목록 조회를 host worker와 분리, 인앱 파일 변경 직후의 목록 무효화, 팔레트 입력의 IME 조합 중 전역 키맵 가드
- 레지스트리: monaco 명령의 키 재지정, 비동기 실패의 toast 연결, `view.explorer`·`view.welcome`·`tab.moveToNewWindow` 등 런타임은 있으나 화면 경로가 없는 명령
- 공용화: Modal·scrim·포커스 trap 구성이 스니펫 편집기·키바인딩 편집기·팔레트 세 곳에 복제돼 있습니다
- 테스트 부채: `NativeApplication` 안의 배선(팔레트 표시, 명령 실행, 파일 열기, reveal 대기)은 앱 단위 테스트 하네스가 없어 구성 함수 테스트와 컴파일로만 확인했습니다. `src/terminal-input-tests.rs:267`의 `clippy::await_holding_lock` 1건이 `clippy --all-targets -D warnings`를 막습니다.
