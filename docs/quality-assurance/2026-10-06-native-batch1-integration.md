# 전환 배치 1 통합 검증 (2026-10-06)

## 범위

배치 1은 기존 native 코드의 결함과 기반을 고친 세 단계입니다. 단계별 상세는 `2026-10-06-native-batch1-{editor-input,runtime-liveness,shell-foundation}.md`입니다. 세 단계 모두 리뷰 판정은 pass(차단 항목 0)였습니다. 이 문서는 메인이 직접 실행한 통합 검증과 남은 부채를 기록합니다.

## 메인이 직접 실행한 검증

| 검사 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` (배치 1 적용 상태) | 322건 중 314 통과, 8 실패 |
| 같은 명령을 배치 1 변경을 치운 커밋 기준(`1d5af3b`)에서 실행 | 320건 중 313 통과, 7 실패. 실패 7건은 위 8건 중 `remote_assets` 1건을 뺀 것과 동일 |
| 회귀 1건 수정 뒤 `--lib -- application_ports remote_assets application::` | 9건 통과 |
| 기준에서도 실패하던 5개 모듈을 `--test-threads=1`로 실행 | 29건 중 25 통과, tooltip 4건만 실패 |
| `cargo test --manifest-path native/taide-native-terminal/Cargo.toml --offline` (lockfile 갱신 뒤) | 전체 통과(lib 4, session 8 포함) |
| `cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline` | exit 0 |
| `cargo fmt -- --check` (app, ui, editor, terminal) | 전부 exit 0 |

## 배치 1이 만든 회귀와 수정

- 증상: `remote_assets::prepare_tests::production_startup과_settings는_자산_준비_실패시_listen하지_않고_성공후에만_시작한다`가 `tracked_count` 1로 실패했습니다.
- 원인: 단계 2가 에이전트 감지 폴링 등록을 `application_ports::Ports::start`에 넣어, 통합 서버를 켜지 않은 상태에서도 감독 작업이 1개 남았습니다.
- 수정: 폴링 등록을 `Ports::start`에서 떼어 `NativeApplication::new`의 기동 지점에서 `start_agent_poll`을 직접 호출하도록 옮겼습니다. Tauri가 통합 서버 reconcile과 별개로 앱 setup에서 `agent-poll`을 등록하는 구조와 같습니다. 단계 2가 `Ports::start` 기준으로 바꿨던 테스트 기대값은 원래 값(0)으로 되돌렸습니다.

## 메인이 추가로 고친 기존 문제

- `native/taide-native-terminal/Cargo.lock`에 `taide-remote-wire`가 없어 `--locked` 명령이 시작되지 않았습니다. `--offline` 실행으로 갱신했습니다(+9줄).
- `experiments/lsp-coordinator-spike/src/bin/mock-server.rs`의 import 순서 1곳 때문에 native 앱 `cargo fmt -- --check`가 실패했습니다. 포맷을 적용했습니다.

## 화면 확인

- [x] 창 제목 `TAIDE`, 크기 1400x900 — 격리 데이터 디렉터리로 앱을 실행해 창 정보와 창 캡처 크기로 확인
- [ ] 테마의 전역 외형 적용, 자체 타이틀바와 OS 타이틀바 중복 해소, welcome 화면 — 실행 당시 화면이 잠겨 있어(최상위 창 소유자 `ScreenLock`) 창 내용이 그려지지 않았습니다. 화면이 켜진 상태에서 다시 캡처해야 합니다.
- [ ] 단계별 QA 문서의 실기 확인 목록(편집기 9건, 터미널 과부하·resize, 탐색기 스크롤)

## 테스트 부채

| 항목 | 상태 | 필요 시점 |
| --- | --- | --- |
| tooltip 테스트 4건(`problems::tests` 3건, `status_ide::tests` 1건)이 커밋 기준에서도 항상 실패 | 원인 미확정. 단계 3 구현자는 테스트가 tooltip provider의 frame 시작·종료를 호출하지 않는다고 보고 | 배치 2 첫 단계 |
| `projects::tests`, `remote_dispatch::tests`, `remote_projects::tests` 각 1건이 전체 병렬 실행에서만 실패 | 단일 스레드에서는 통과 | 배치 2 첫 단계 |
| `tests/projects.rs`의 `미연결_hook는_열기_전_거절하고_…`가 2026-10-04에 제거된 동작을 기대해 실패 | 미수정 | 배치 2 첫 단계 |
| `tests/terminal-host.rs` 전체 실행이 간헐적으로 `openpty` 오류(code -6) | 구현자 관찰 6회 중 4회. 단언 실패는 없음 | 배치 2 첫 단계 |
| Diff·ClaudeDiff·SearchEditor 폴백 화면의 자동 테스트 없음 | 코드 읽기로만 확인 | 해당 탭 구현 시 |

## 다음 배치로 넘긴 동작 차이

- 편집기: `{` 뒤 Enter의 추가 들여쓰기(autoIndent full), 자동 들여쓰기 공백 정리(trimAutoWhitespace), 탭 크기 전달, `scrollBeyondLastLine`, 스크롤바 테마 토큰
- 터미널: 프레임 큐 hard 한도 초과와 효과 예산 초과는 여전히 세션 실패, 보류 중 child가 응답을 읽지 않을 때의 교착 가능성, 닫힌 viewport에 남은 view의 resize 판정
- 셸: 버튼 외형이 secondary 변형 기준이라 outline·ghost 계열과 어긋날 수 있음, 선택 외곽선 색, 임시 문구 키 `tab.contentUnavailable`
