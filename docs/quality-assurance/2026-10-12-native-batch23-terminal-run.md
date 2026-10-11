# 배치 23 선택 텍스트의 터미널 실행 진입

현재 상태: editor-82의 메뉴 경로에 두 팔레트/재지정 키 진입을 연결했습니다. 실제 앱의 네 경우와 기존 명령 분기/등록 검사 25건이 통과했습니다. 추가 문서 종류·본문/peek 소유·보조 창에서 두 명령의 실제 실행은 후속으로 남아 partial이며 배치 23과 전체 전환은 미완료입니다.

## 원본과 구현

- `src/widgets/editor-pane/editor-pane.tsx:290`은 file pane에 `taide.runSelectedTextInTerminal`을 등록하며 메뉴에서 선택 또는 현재 줄을 실행합니다. 원본 기본 키는 없습니다.
- `src/entities/terminal/terminal.commands.ts`의 `terminal.runSelectedText`와 `src/widgets/editor-area/editor-area.tsx:219`의 팔레트 경로는 현재 pane의 등록된 본문 편집기를 사용합니다. `focused-editor-tab.ts`는 file/appFile/untitled만 허용합니다. peek의 선택을 본문 선택으로 바꾸어 실행하는 새 동작을 만들지 않습니다.
- `src/shared/lib/editor-selection.ts`는 비어 있지 않은 주 선택을 읽고 빈 선택은 현재 줄 전체를 사용합니다. `terminal-tab-targets.ts`와 `session-write-order.ts`는 같은 pane의 기존 터미널·준비 중 입력·직렬 쓰기를 유지합니다.
- 현재 native 메뉴/Host/PTY 경로는 이미 구현돼 있었지만 두 명령의 execution은 Unavailable이었습니다. native-host 전용 `DocumentEdit::RunSelectedText`를 기존 등록에 연결하고 실제 현재 창/프로젝트/pane/tab의 본문 뷰에서 기존 `EditorContextMenu` typed 요청을 생성합니다. peek 로컬 키도 원본의 본문 소유를 유지합니다.
- 원본 커스텀 액션은 file pane에만 등록되므로 appFile/untitled에서는 해당 Monaco 액션을 지원 집합에서 제외합니다. 일반 터미널 명령과 구분합니다. 새 기본 키·의존성·엔진·디자인은 추가하지 않습니다.

대상은 `native/taide-native-ui/src/command-registry.rs`, `native/taide-native-app/src/{application.rs,command-dispatch.rs,editor-context-menu.rs,editor-locations.rs,application-context-menu-tests.rs}`입니다.

## 재현과 검사 전제 교정

최초 실제 앱 검사에서 같은 제목의 두 팔레트 항목이 모두 비활성이었습니다. `terminal-entry-red.log`는 0통과/1실패·9.30초·exit 101입니다. 원본도 제목이 같은 두 항목을 제공하므로 검사에서는 명령 ID로 항목을 확인하고 실제 방향키로 원하는 항목을 선택합니다.

첫 전달 검사는 터미널의 display cell을 단순 연결해 두 칸 이모지의 보조 칸을 공백 문자로 취급했습니다. 연속 문자열을 찾지 못한 51.10초/35.43초 실패를 제품 전달 실패로 단정하지 않습니다. 설치된 alacritty_terminal의 `term/mod.rs` 텍스트 추출과 같은 WIDE_CHAR_SPACER/LEADING_WIDE_CHAR_SPACER 제외를 검사 보조 함수에 적용했습니다. 검색 기대값이나 10초 상한을 완화하지 않았습니다. 한 프레임 후 아직 큐에 있던 요청을 즉시 실행 완료로 기대한 26.95초의 진단 실패도 별도 전제 오류입니다.

검사 작성의 dirty 필드 위치 오류, 요청 큐의 중복 가변 참조와 inspection의 소유권 오류, 기존 scope 참조/이동 값을 새 처리 위치까지 연장한 컴파일 오류는 실제 검사가 실행되기 전 실패이며 통과 수에 포함하지 않습니다. 현재 창에서 소유 뷰를 국소적으로 구하고 typed 요청을 기존 intent 소비 시점에 처리하도록 정리했습니다.

## 실제 결과

모든 Cargo는 `--locked --offline --target-dir experiments/native-shell-spike/target`로 직렬 실행했습니다. 앱은 `--features inspection -- --test-threads=1`을 사용합니다.

| 검사                                                                                                                                 | 범위                                                                                                        | 실제 결과                  |
| ------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- | -------------------------- |
| App `--lib application::context_menu_tests::실제_터미널_실행의_팔레트와_재지정_키는_현재줄과_선택을_같은_세션에_전달한다 -- --exact` | 두 팔레트·두 재지정 키, 현재 줄·Unicode/역방향 선택, 동일 세션 재사용, 내용/revision/dirty/선택/디스크 보존 | 1통과/0실패·26.85초·exit 0 |
| App `--lib command_dispatch::tests`                                                                                                  | 기존 명령 분기·활성 편집기·읽기 전용·창 문맥                                                                | 9통과/0실패·8.28초·exit 0  |
| UI `--features inspection --lib command_registry::tests`                                                                             | 전체 명령 메타데이터·지원/읽기 전용·활성 조건                                                               | 15통과/0실패·0.02초·exit 0 |

새 검사는 App 1건의 네 경우이며 총 25건에 포함됩니다. 원문은 임시 프로젝트 파일이고 PTY는 소유 `/bin/cat`입니다. 실제 사용자의 앱 데이터·셸 명령·클립보드·Keychain·Trash·OS 설정과 보호 M8 앱을 사용하지 않았습니다. OS 합성 입력은 사용하지 않았습니다.

로그는 `/private/tmp/taide-batch23-terminal-entry-{red,compile,delivery-red,request,scope-compile,wide-cell-red,green,dispatch,registry}.log`에 있습니다. 변경 없는 메뉴의 준비/오류/종료·보조 창 성공 근거는 `2026-10-11-native-batch23-context-menu-app.md`와 제품 기준 885b4417의 전체 실행에서 재사용하며 이번 두 명령의 추가 앱 경계를 대신 검증한 것으로 계산하지 않습니다.

이전 App 전체는 885b4417에서 67대상/891통과/1실패·보호 Trash 3제외·exit 101입니다. 이번 코드의 전체 실행 결과로 바꾸지 않습니다. 기존 프로젝트 open 초기화의 원인 미확정 실패는 PROCESS f1/배치 31에 남기고 조건이 같은 실패 검사는 반복하지 않습니다. 배치 완료의 전체 게이트는 아직 닫지 않았습니다.

## 진척과 반복 실행

기능표의 기존 editor-82 missing 판정은 이미 연결된 메뉴를 누락했으므로 부분 구현으로 바로잡습니다. 대상 588·296완료/292미완료, 배치 23 미완료 25행과 최종 33은 유지합니다. 상태 분류만 partial 93/missing 87로 바뀝니다. 행수 비율·검사 수를 전체 완료율로 환산하지 않으며 전체율/잔여 시간은 미산정입니다.

이번 실패→진단→검사 전제 교정→성공의 결과를 분리해 기록합니다. 수정 없는 성공 검사는 반복하지 않습니다. 각 실행은 종료했으며 과거 모든 실행이나 제품 전체의 모든 루프 종료성을 증명한 것으로 확대하지 않습니다.

## 컴파일·형식·동결 경계

App `cargo check --tests --features inspection`는 1분 49초·exit 0입니다. 직접 실행한 25건과 구분하며 검사 컴파일을 전체 테스트 실행 성공으로 부르지 않습니다. 소유 6Rust의 rustfmt check와 diff check는 exit 0입니다.

frozen remote-web host `cargo check --tests`는 24.20초·exit 0입니다. Wasm의 제품 `cargo check --target wasm32-unknown-unknown`는 0.09초·exit 0이며 모든 명령은 locked/offline과 같은 target-dir를 사용했습니다. 그 전에 추가로 시도한 Wasm `--tests`는 `tests/snippet-catalog.rs`에서 `std::time::Instant`와 `taide_native_ui::toast::Instant`(web_time)의 기존 불일치 7건으로 exit 101입니다. 해당 검사와 toast/settings의 시간 API·의존성은 이번에 바뀌지 않았습니다. frozen 소스/manifest/lock/의존 그래프는 변경하지 않고 배치 31 검사 이식 부채로 보존합니다. Wasm 테스트 컴파일 통과로 보고하지 않습니다.

Engine/Editor/frozen과 모든 Cargo.toml/Cargo.lock의 작업 diff는 0이며 디스크 여유는 331GiB/82%입니다. 빌드 정리나 보호 앱 변경은 하지 않았습니다.

UI의 테스트 대상 컴파일도 `cargo check --tests --features inspection`에서 39.41초·exit 0입니다. 현재 소유 Cargo 핸들은 모두 종료했습니다. 전체 대상 실행이나 OS 실기 통과로 확대하지 않습니다.

검증한 소유 8경로와 PROCESS 상단을 `6d7532cc`로 선별 저장했습니다. 제품 기능표의 코드 기준은 이 커밋으로 고정하며 이후 문서 전용 커밋으로 바꾸지 않습니다. 작업 PROCESS 하단의 기존 사용자 변경 SHA-256은 `7b26f85e8fdcbd03592f4c6722e88350d490341515c9175d0725ece101c1a4ed`를 보존했습니다.
