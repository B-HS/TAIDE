# Rust-native 배치 8 통합 검증

현재 상태: XLML 오분류·찾기 엔진·순수 코어·위젯·앱 명령 연결을 구현했습니다. 변경 크레이트의 전체 대상을 한 번씩 실행하고 실패 영향만 수정 뒤 재실행했습니다. 동결 브라우저 호스트/Wasm 컴파일·최종 포맷·디스크·구현 일반 푸시를 확인했습니다. 전체 Rust-native 전환의 완료 판정은 아니며 다음 표시 옵션 배치로 계속 진행합니다.

## 범위와 기준

사용자 최신 전체 목표와 `../acknowledge/2026-10-09-native-full-resume.md`에 따라 기존 배치별 중단 지시를 해제하고 메인이 직접 직렬 구현했습니다. 서브에이전트·workflow를 사용하지 않았고 Cargo는 한 번에 한 프로세스만 실행했습니다.

원본 기준은 TS CodeEditor와 Monaco 0.56.0입니다. 새 화면·새 디자인·기능을 추가하지 않았습니다. 찾기 JS 방언은 앱 전용 `regress =0.12.0`이며 편집기/UI에는 엔진 의존성을 넣지 않았습니다. ferriki-textmate 0.12.0·ferroni 1.8.1 구문 강조 경로를 유지합니다. 원본 결함을 강제 재현하지 않는 결정과 100ms 성능 게이트·1초 검색 상한·단일 줄 내부 폭주 수용 조건은 `../acknowledge/2026-10-09-native-find-regex-decisions.md`를 따릅니다.

| 변경 | 구현·검증 근거 |
| --- | --- |
| XLML | 첫 XML 요소가 Workbook인 입력을 HTML로 오분류하지 않게 수정. 수정 전 실패 재현 뒤 XLML 4건·HTML 3건 통과. `../bug/2026-10-09-native-xlml-workbook-classification.md` |
| 찾기·치환 코어와 엔진 | engine 3,800개·찾기 2,432개·치환 패턴 2,624개 기준값 검증. 최적화 검색 32.256333ms·수집 4.6865ms 게이트 통과. 이전 탐색의 가짜 줄 끝/단어 경계 결함 수정. `2026-10-09-native-batch8-find.md` |
| 위젯 | 원본 크기·반응형 폭·리사이즈·테마·Codicon·두 입력·옵션·개수/오류·탐색/치환·범위·접기/닫기·이력·일치 번호 입력. 전체 대상의 UI/controller 13건·플랫폼 키 5건과 후속 비활성 위젯 회귀 포함 |
| 앱 연결 | native-host registry·Find 편집 큐 보존·뷰별 상태·revision/범위 추적·읽기 전용·접기 해제·선택 드러내기·문서 변경 통지·사용자 키 재지정/해제. 기존 편집/IME 경로와 browser 동결 유지 |

## 전체 대상 실행

공통 옵션은 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 변경 4개 크레이트의 전체 테스트 대상을 `--no-fail-fast`로 한 번씩 직접 실행했습니다. UI는 기존 inspection 대상을 포함했습니다. 앱에서는 실제 Trash API를 호출하는 보호 대상 세 검사만 정확한 이름으로 제외했습니다. 임시 데이터·로컬 서버·파일 감시·ImageIO 검사의 기존 실행 환경 제한을 피하기 위해 앱 전체 실행은 제한 밖에서 수행했으며 보호 검사 세 건을 포함하지 않았습니다.

```sh
cargo test --manifest-path native/taide-native-editor/Cargo.toml --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target
cargo test --manifest-path native/taide-native-syntax/Cargo.toml --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target
cargo test --manifest-path native/taide-native-ui/Cargo.toml --features inspection --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target
cargo test --manifest-path native/taide-native-app/Cargo.toml --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target -- --skip remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다 --skip 실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다 --skip 실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다
```

| 크레이트 | 통과 | 실패 | ignored | 제외 | exit |
| --- | ---: | ---: | ---: | ---: | ---: |
| editor, 전체 | 188 | 0 | 1 | 0 | 0 |
| syntax, 전체 | 152 | 0 | 3 | 0 | 0 |
| UI, 전체 inspection | 271 | 0 | 0 | 0 | 0 |
| app, 전체 최초 실행 | 611 | 1 | 0 | 3 | 101 |
| app, 실패 1건 최종 선별 재검사 | 1 | 0 | 0 | 나머지 371 filtered out | 0 |

앱 전체의 실패는 `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`의 글꼴 개수 기대값 한 건입니다. 새 아이콘 font_data 1개를 기존 정적/가변 공유 글꼴과 별도로 확인해 두 기대값에 포함했습니다. 첫 선별 재검사에서는 fallback 꼬리를 잘라내는 기존 `chain` helper를 아이콘 family에 적용해 테스트 helper의 뺄셈 overflow가 발생했습니다. 실제 family 목록을 직접 확인하도록 수정한 뒤 해당 검사 한 건만 `--lib -- --exact <위 이름>`으로 재실행해 통과했습니다. 제품 글꼴 공유 구현은 수정하지 않았습니다.

전체 성공 결과와 최종 재검사를 재사용하면 서로 다른 app 검사 612건 통과, 실패 0건, 보호 대상 3건 미검증입니다. 보호 대상을 포함한 앱 전체 통과라고 표현하지 않습니다. 이전 XLML 실패는 이번 앱 전체 실행의 XLML 4건에서도 해소됐습니다. 기존 vendored wry 경고 17건과 대형 lib 테스트 바이너리의 linker unwind 경고가 남습니다.

실행 로그는 `/private/tmp/taide-batch8-{editor,syntax,ui,app}-full-20261009.log`, `/private/tmp/taide-batch8-app-font-sharing-after-20261009.log`, `/private/tmp/taide-batch8-app-font-sharing-final-20261009.log`입니다. 각 전체 로그의 `test result`를 합산했으며 선별 재검사와 중복된 테스트를 통과 수에 다시 더하지 않았습니다.

커밋 전 검토에서 비활성 위젯의 이전 포커스가 키를 라우팅하는 실패를 추가로 재현했습니다. 위젯 입력/치환/번호 입력과 포커스를 활성 UI에 한정한 뒤 관련 `--test editor-find` 14건이 통과했습니다(exit 0). 로그 `/private/tmp/taide-batch8-find-disabled-before-20261009.log`·`/private/tmp/taide-batch8-find-disabled-after-20261009.log`입니다. 전체 UI 성공 271건 중 이 target의 기존 13건을 최종 검사 결과로 대체하고 새 회귀 1건을 포함하면 서로 다른 UI 검사 272건 통과입니다. 다른 성공 대상을 중복 실행하지 않았습니다.

이 보완 뒤 앱 `cargo check --tests --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`도 exit 0(13.07초)입니다. `/private/tmp/taide-batch8-app-final-check-20261009.log`입니다. browser에서 제외되는 native-host 전용 위젯 내부 변경이므로 앞선 동결 호스트/Wasm 컴파일 증거를 재사용합니다.

## 동결·포맷·보호 경계

- [x] remote-web 호스트 `cargo check --tests --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`: exit 0, 4.83초. `/private/tmp/taide-batch8-remote-check-20261009.log`
- [x] remote-web 실제 Wasm canvas 컴파일: `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --features canvas --locked --offline --target-dir experiments/native-shell-spike/target`: exit 0, 41.90초. `/private/tmp/taide-batch8-remote-wasm-check-20261009.log`
- [x] 변경 4개 크레이트 `cargo fmt --check --manifest-path native/<crate>/Cargo.toml`: 각 exit 0. `git diff --exit-code bd8370f4 -- native/taide-remote-web native/taide-native-editor/Cargo.toml native/taide-native-editor/Cargo.lock native/taide-native-ui/Cargo.toml native/taide-native-ui/Cargo.lock`: exit 0. 디스크 여유 683GiB·사용률 63%
- [x] 새 기본 키 JSON Prettier 검사: exit 0. 변경 파일의 whitespace diff 검사도 exit 0

배치 중 remote-web의 소스·manifest·lockfile과 editor/UI manifest·lockfile은 변경하지 않았습니다. regress·ferriki·ferroni는 remote-web lockfile에 없습니다. 새 Find UI·registry·기본 키·빈 일치 표시는 native-host에서만 활성화합니다. 실제 앱 데이터·OS 설정·클립보드·Keychain·Trash와 보호 앱 번들은 조작하지 않았습니다. 빌드 산출물을 삭제하지 않았습니다. Wasm 검사 종료 후 디스크 여유 683GiB·사용률 63%를 확인했습니다.

## 남아 있는 실기·전체 목표 조건

실제 화면·OS 키/마우스·IME·접근성·RTL·대형 문서 UI·soak 검사는 수행하지 않았습니다. 새 입력 검사는 egui 메모리 RawInput이며 OS 합성 입력으로 보호 조건을 우회하지 않았습니다. editor ignored 1건과 syntax ignored 3건은 성능 기록 검사로 자동 실행했다고 보고하지 않습니다.

단일 줄 내부 정규식 호출은 1초 확인으로 중단할 수 없습니다. 승인된 한계를 격리 probe로 확인했으며 별도 프로세스 종료를 제품 우회로 넣지 않았습니다. 실제 제품 검색은 줄·일치 사이에서 1초 상한을 확인합니다.

배치 8 뒤 전체 목표를 계속 진행합니다. 다음은 실제로 공급/렌더되지 않는 편집기 표시 설정과 나머지 표시 계층입니다. 전체 기능 완료율은 최신 전수 대응표가 없어 미산정이며 테스트 수나 이 배치의 체크리스트 비율을 기능 대응률로 바꾸지 않습니다. LSP UI·나머지 패널/메뉴·다중 창·전환/출시 게이트는 `2026-10-09-native-completion-evidence.md`와 roadmap의 잔여 범위에 보존합니다.

## Git 기록

변경 소스·검사·고지·QA와 PROCESS의 이번 작업 절만 선별 스테이징했습니다. XLML 수정 `38b0f664`, 엔진/코어 `bd8370f4`, 위젯·앱 연결 `361786d3`를 `origin/to_rust_native`에 일반 푸시했으며 로컬/원격 추적 차이 0/0을 확인했습니다. 최종 메시지는 한국어 Conventional Commit이며 사용자 단독 author·AI 트레일러 없음입니다. 기존 HANDOFF·architecture·이전 결정/운영 문서의 다른 변경은 포함하지 않았습니다. PROCESS 배치 8 체크리스트를 8/8로 마치고 배치 9를 작성했습니다.
