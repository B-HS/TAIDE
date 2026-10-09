# 배치 10 — 괄호 색상과 안내선 (2026-10-09)

상태: 분석·설정/테마 공급·문자 색상·괄호/들여쓰기 안내선 구현, 변경 크레이트 전체 대상 게이트와 브라우저 host/Wasm·포맷·동결 diff를 마쳤습니다. `a33ca36b`을 선별 커밋·일반 푸시하고 로컬/원격 차이 0/0을 확인했습니다. 서브에이전트·workflow 없이 메인이 직렬 수행했고 배치 11의 상단 고정 줄로 계속 진행합니다.

## 원본과 구현 경계

- TS `code-editor.tsx`·`code-editor-settings.ts`: 색상 기본 true, 안내선 기본 false. Large/ReadOnly는 색상만 끄며 안내선 옵션은 유지합니다. Monaco 0.56.0 `config/editorOptions.js`: 들여쓰기/활성 안내선 true, 가로 괄호 선은 활성 짝에만 표시하고 짝별 독립 색 풀은 false입니다.
- `model/bracketPairsTextModelPart/bracketPairsTree/{parser,brackets,bracketPairsTree,tokenizer}.js`와 `languages/supports/languageBracketsConfiguration.js`: 언어 구성의 일반/색상 괄호 합집합, 문자열·주석·정규식 제외, 가장 가까운 닫을 수 있는 조상, 미완성/예상 밖 괄호, 색상 중첩과 안내선 중첩의 별도 깊이를 사용합니다. 명시 색상 구성이 없으면 `<`/`>`는 색상 풀에서 제외합니다. native의 기존 정규식 탐색은 이미 두 구성의 합집합입니다.
- `model/guidesTextModelPart.js`·`viewParts/indentGuides/{indentGuides.js,indentGuides.css}`·`viewModel/viewModelLines.js`: 주 커서의 가장 안쪽 짝, 여는/닫는 괄호·내용의 최소 들여쓰기 열, 닫는 괄호 앞 내용에 따른 가로 선 위치, 기본 들여쓰기 안내선과 중복 제거, 탭/공백/빈 줄/off-side·wrap·접기를 반영합니다. 원본이 주석으로 인정하는 wrap 안내선 좌표 결함을 강제로 재현하지 않습니다.
- `common/core/editorColorRegistry.js`와 TS `monaco/theme.ts`: 괄호 팔레트는 TS에서 매핑하지 않은 Monaco 기본 dark `#FFD700/#DA70D6/#179FFF`, light `#0431FA/#319331/#7B3814`입니다. 예상 밖 괄호는 RGBA(255,18,18,0.8), 비활성 짝 안내선은 팔레트 0.3 alpha·활성은 원색입니다. 들여쓰기 색은 `editor.indentGuide`, 활성 색은 TS가 매핑한 whitespace 기본 경로입니다.
- Shiki Monaco adapter `@shikijs/monaco/dist/index.mjs`는 색/스타일로 scope를 역추론하며 embedded language metadata를 제공하지 않습니다. native는 확정된 TextMate 토큰의 실제 종류를 사용해 문자열/주석/정규식 괄호가 분석에 들어가지 않게 합니다. 엔진이나 원본 문법을 교체하지 않습니다.

문서 소유 분석 캐시는 문서가 닫히거나 병합되어 제거될 때 함께 회수합니다. 변경 저널로 영향 줄만 다시 읽고, 토큰 갱신 여부와 종류를 확인해 분석을 갱신합니다. 뷰의 선택·wrap/접기·테마·화면 좌표는 문서 분석과 분리합니다. editor/UI에 regex/구문 엔진 의존성을 추가하지 않습니다. native-host 경계로 브라우저 소스·동작·의존 그래프·lockfile을 보존합니다.

테마 `intellij-islands-light.json`의 7자리 속성 값 색 `#0083080`은 사용자 결정 대기입니다. 의도한 HEX 값의 근거가 없어 추측으로 수정하지 않습니다. 해당 규칙만 제외하고 기본 전경색을 쓰는 추천안을 질문했으며 독립된 분석/표시 구현은 계속합니다.

## 게이트와 보호

분석 결과와 메모리 egui 문자/shape를 사용해 짝·잘못된 괄호·토큰 제외·줄 편집/토큰 재적용·설정/테마·활성 선택·wrap/접기/스크롤·탭/Unicode·대형 문서를 검증합니다. 변경 크레이트 전체 대상은 --no-fail-fast로 한 번씩 직접 실행하고 실패 영향만 다시 확인합니다. Cargo는 한 번에 하나만 실행합니다. 브라우저 host/Wasm·패키지 포맷·동결 diff·디스크를 확인합니다.

기존 보호 Trash 3건·ignored 성능·실제 UI/OS 입력·IME/RTL/접근성/대형 실기·soak를 통과로 표시하지 않습니다. 보호 앱 번들·실제 데이터·클립보드·Keychain·OS 설정·Trash를 건드리지 않습니다. OS 합성 입력을 사용하지 않습니다. sticky scroll·minimap·합자·진단/overview·주입/블록·LSP 등은 전체 목표의 후속 범위입니다.

## 분석 코어 검증

`docs/utils/2026-10-09-native-bracket-reference.js`는 실제 Monaco `BracketPairsTree`·`GuidesTextModelPart`를 메모리 모델로 직접 실행해 23언어·230표본의 색상 괄호·일반 짝·최소 들여쓰기·빈 줄/off-side·활성 들여쓰기 기준값을 생성했습니다. 제품 코드나 OS UI 입력을 실행하지 않았습니다. 원본의 깊이 제한 결함은 복제하지 않으며 4096 중첩을 반복문으로 처리합니다.

editor --tests 초기 컴파일 exit 0(4.49초), UI --tests 초기 컴파일 exit 0(3.84초)입니다. syntax 새 target의 첫 실행은 테스트의 Transaction 인자/빌림 수명 가정 때문에 컴파일 실패했습니다. 실제 API와 명시한 토큰 빌림 경계를 확인해 테스트를 고친 뒤 기준값 230표본·문자열/주석/정규식/부정확 토큰 제외·4096 중첩의 3검사가 통과했습니다. 동일 조회에서 Arc를 불필요하게 복사하는 실패 1건은 실제 재현했습니다(exit 101, `/private/tmp/taide-batch10-bracket-core-runtime-20261009.log`). 캐시가 현재 상태이면 mutable 복사 전에 반환하도록 근본 경로를 고친 뒤 해당 캐시/줄 삽입/토큰 종류/언어 변경 회귀 1건이 통과했습니다(exit 0, 빌드 1.24초, `/private/tmp/taide-batch10-bracket-cache-after-20261009.log`). 전체 크레이트 게이트의 최종 결과는 아래에 별도로 기록합니다.

토큰이 정확하지 않은 뒷줄에 닫는 괄호가 있을 때 앞줄의 정상 여는 괄호를 오류로 표시하는 문제를 실패 재현했습니다(exit 101, `/private/tmp/taide-batch10-bracket-partial-before-20261009.log`). 전체 토큰이 정확하거나 알려진 조상 괄호에서 해당 짝이 버려졌을 때만 미완성 괄호를 확정 오류로 표시하도록 수정했고 해당 토큰 회귀가 통과했습니다(exit 0, 빌드 1.14초, `/private/tmp/taide-batch10-bracket-partial-after-20261009.log`). 대형 문서 조회는 이전 모든 짝을 순회하지 않도록 interval tree로 범위를 제한합니다. 5만 줄 조회와 문서 종료 회수·토큰 종류만 변경되는 회귀도 전체 syntax 게이트에서 통과했습니다.

## 표시 구현과 선별 검증

native-host의 선택적 `EditorBracketColors`로 기존 필수 편집기 API와 브라우저 화면을 보존합니다. 앱 초기화·설정/테마 갱신·pane 공급에서 색/옵션을 전달하며 Large/ReadOnly는 색상만 끕니다. 실제 syntax 문자 색 위에 괄호 색을 적용하고 글꼴 스타일·장식 순서는 보존합니다. 가이드는 기존 표시 줄의 실제 caret 좌표·clip을 사용하며 주 커서의 가장 안쪽 짝과 기본 들여쓰기 안내선을 렌더합니다.

메모리 egui의 실제 문자 색·가이드 shape로 새 회귀 7건이 통과했습니다. 색상/종류/옵션/테마, 활성 중첩과 가이드 중복, Unicode 가로 선과 닫는 괄호 앞 내용, wrap/접기, 기본 들여쓰기/빈 줄/탭, 문서 편집과 두 뷰의 캐시 공유, 가로/세로 스크롤·clip을 포함합니다. app --tests 컴파일 exit 0(11.93초), UI 첫 실행에서 4건 통과·접기 1건 실패였고 실패 원인은 테스트가 접기 바이트 범위에 줄 번호를 넣은 것입니다. 실제 API에 맞춘 선별 접기 회귀 1건이 통과했습니다(exit 0, 빌드 2.99초, `/private/tmp/taide-batch10-ui-bracket-fold-after-20261009.log`). 추가한 두 뷰/스크롤 회귀 2건도 통과했습니다(exit 0, 빌드 0.68초, `/private/tmp/taide-batch10-ui-bracket-views-20261009.log`). 테스트의 ByteIndex·Selection·callback 수명·f32 스크롤 타입 가정으로 발생한 컴파일 실패는 실제 API를 확인해 테스트에서 바로잡았습니다. 제품 동작을 테스트의 잘못된 가정에 맞추지 않았습니다.

## 전체 대상 게이트

동일한 변경 상태에서 각 크레이트의 전체 대상을 직접 한 번씩 실행합니다. 공통 옵션은 `--locked --offline --target-dir experiments/native-shell-spike/target --no-fail-fast`입니다.

1. editor: `cargo test --manifest-path native/taide-native-editor/Cargo.toml` exit 0, 188 통과·0 실패·1 ignored, 26 대상, 빌드 4.90초입니다. `/private/tmp/taide-batch10-editor-full-20261009.log`입니다.
2. syntax: `cargo test --manifest-path native/taide-native-syntax/Cargo.toml` exit 0, 158 통과·0 실패·3 ignored, 11 대상, 빌드 2.50초입니다. 새 bracket-display target 6건과 기존 구문/찾기/언어 구성 회귀를 포함합니다. `/private/tmp/taide-batch10-syntax-full-20261009.log`입니다.
3. UI: `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features inspection` exit 0, 294 통과·0 실패·0 ignored, 17 대상, 빌드 7.49초입니다. 새 화면 7건과 이전 대형 ruler·캐럿·스크롤·위젯 회귀를 포함합니다. `/private/tmp/taide-batch10-ui-full-20261009.log`입니다.
4. app: `cargo test --manifest-path native/taide-native-app/Cargo.toml`에 아래 보호 제외 옵션을 추가해 exit 0, 612 통과·0 실패·보호 3 제외, 67 대상, 빌드 39.33초입니다. 모든 builtin 테마의 가이드/팔레트 공급 확인을 포함합니다. 기존 승인된 loopback·감시·ImageIO 경계에서 실행했습니다. `/private/tmp/taide-batch10-app-full-20261009.log`입니다.

보호 제외 옵션은 `-- --skip 'remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다' --skip '실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다' --skip '실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다'`입니다. 보호 Trash 3건과 ignored 성능 4건은 통과 수에 포함하지 않습니다. generator에 정상 종료를 추가하고 포맷한 후 다시 실행해 23언어·230표본 생성과 종료 exit 0을 확인했습니다. `/private/tmp/taide-batch10-bracket-reference-final-20261009.log`입니다.

## 최종 정적 검사와 범위

- remote-web host `cargo check --tests --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 3.60초입니다. `/private/tmp/taide-batch10-remote-check-20261009.log`입니다.
- remote-web Wasm `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --features canvas --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 2.02초입니다. `/private/tmp/taide-batch10-remote-wasm-check-20261009.log`입니다.
- 변경 4개 크레이트의 개별 `cargo fmt --manifest-path native/<crate>/Cargo.toml --check`, generator의 프로젝트 Prettier 명시 ignore-path 검사, 이번 source/docs의 `git diff --check` exit 0입니다. `cargo fmt --all`은 사용하지 않았습니다.
- 기준 `b3dedc77` 대비 remote-web 전체와 4개 크레이트 manifest·lockfile의 `git diff --exit-code` exit 0입니다. 새 의존성·lockfile 변경 없음입니다. 디스크 여유 677GiB·사용률 64%이며 캐시 정리를 실행하지 않았습니다.

배치 10 체크리스트와 기존 표시 옵션의 소비 경로 12/14는 전체 기능 대응률이 아닙니다. `2026-10-09-native-completion-evidence.md`에서 sticky scroll·minimap 미소비와 실제 화면/OS 입력·전체 LSP·출시 게이트를 후속 범위로 보존합니다. 테마 7자리 값은 사용자 결정 대기입니다.
