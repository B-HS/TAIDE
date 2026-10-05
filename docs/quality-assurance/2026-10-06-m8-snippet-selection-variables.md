# M8 Snippet 문서 선택 변수·clipboard 분배

## 대상 파일

- `native/taide-native-editor/src/snippet-variables.rs`, `src/lib.rs`
- `native/taide-native-ui/tests/snippet-parser.rs`
- `native/taide-native-ui/tests/fixtures/snippet-variable-context-oracle.mjs`

## 리포트

후속 [actual EditorStore 삽입 transaction](2026-10-06-m8-snippet-store-insertion.md)이 준비된 Expansion의 실제 문서 삽입·첫 placeholder 선택·독립 undo 경계를 연결했습니다. 아래 삽입 미완료는 입력 controller/동기 session·UI/host 공급자의 전체 경계를 포함한 당시 상태입니다.

원본 SelectionBasedVariableResolver를 실제 DocumentSnapshot/Rope·SelectionSet의 UTF-8 byte 경계에 연결했습니다. SELECTION/TM_SELECTED_TEXT는 방향과 무관한 정렬된 선택 범위에서 읽고 TM_CURRENT_LINE/TM_LINE_INDEX/TM_LINE_NUMBER는 선택의 head 위치에서 구합니다. CURSOR_INDEX/CURSOR_NUMBER는 원본 selection 순서의 cursor index를 사용합니다. 조회는 문서/선택/undo를 변경하지 않습니다.

선택 값이 비어 있을 때만 overtyping 정보로 대체하며 원본처럼 overtyped 빈 문자열은 Some("")로 유지하고 정보 없는 빈 선택은 None입니다. 여러 줄 값은 선택 시작 줄의 cursor 이전 leading whitespace와 변수 앞선 마지막 Text marker 줄을 비교합니다. 공통 접두부 이후의 추가 들여쓰기를 각 원래 CRLF/CR/LF 뒤에 붙이고 trailing newline도 보존합니다. choice 옵션은 앞선 Text 문맥을 바꾸지 않습니다.

TM_CURRENT_WORD는 실제 문서/head를 전달하는 typed 언어별 word delegate입니다. oracle가 제공한 단어 결과와 delegate 오류/빈 값/예산 경계를 검증했지만 원본 언어별 wordPattern engine이나 host 소비자는 아직 연결하지 않았습니다. 임의 ASCII/Unicode 단어 규칙으로 대체하지 않았습니다.

clipboard_value는 host가 이미 취득한 문자열만 받아 원본의 spread/count/index 규칙을 적용합니다. CRLF/CR/LF로 나누고 ECMAScript 공백만으로 된 줄을 제거했을 때 cursor 수와 같으면 해당 줄, 아니면 원문입니다. NEL U+0085는 원본처럼 비공백이며 NBSP/BOM은 공백입니다. 이번 합성 검사는 실제 OS clipboard를 읽거나 쓰지 않았습니다.

선택/cursor·UTF-8 scalar·CRLF 내부의 잘못된 위치를 거절하고 선택 값/현재 줄/추가 들여쓰기/단어/clipboard byte 예산을 제한합니다. clipboard 전체가 예산을 넘으면 spread 결과가 작아도 거절하는 명시적 자원 정책이며 무상한 원본의 전체 입력 허용과 동일하다고 주장하지 않습니다. 실제 범위 편집/삽입 session·RSS 전체 예산 검사는 아닙니다.

## 검증

```sh
TAIDE_M8_ORACLE_BUN=/Users/hyunseokbyun/development/js/bun/bin/bun CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-parser snippet_variables는 -- --nocapture
TAIDE_M8_ORACLE_BUN=/Users/hyunseokbyun/development/js/bun/bin/bun CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-parser snippet_variables는_실제_rope_선택과_원본_커서_문맥_clipboard분배를_보존한다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

- [x] 독립 경계/예산/언어 delegate 오류1 첫 PASS입니다. 최초 build1.73초이며 경고0입니다. 잘못된 scalar/cursor/CRLF 내부와 oversized 값/추가 들여쓰기·clipboard 거절, 문서/undo 불변을 확인했습니다.
- [x] 실제 EditorStore 문서6문맥×4앞선 marker 문맥×9변수=216 case와 clipboard14 case의 설치된 원본 값 비교1 최종 PASS입니다. compile.34초/suite.05초·filtered9입니다. reverse selection·별도 primary cursor·overtyped 빈 값/다중 줄·빈 문서·choice/중첩 Text 문맥을 포함합니다.
- [x] 최초 oracle 모듈은 출력 후 Monaco 내부 타이머 때문에 종료하지 않았습니다. 확인한 합성 자식 PID53841만 TERM 회수하여 최초 suite35.43초/exit101(경계1 PASS·비교1 실패)을 기록했습니다. stdout flush 완료 뒤 정상 종료를 추가하고 clipboard 기대 개수15를 실제14로 정정한 뒤 실패한 비교만1회 실행했습니다. 제품 코드/시스템 설정 우회는 없습니다.
- [x] 최신 normal Canvas Wasm.71초 exit0·경고0입니다. 기존 raw/정규화/변수 확장69/들여쓰기90 및 통과한 경계 성공을 반복하지 않았습니다. 의존성/lock/MSRV/제품 TypeScript/Git 변경은 없습니다.
- [ ] 실제 언어별 단어·파일/경로/workspace/time/random/댓글 변수와 OS clipboard/overtyping 취득, regexp compiler/evaluator·삽입/session·native/browser UI/owner는 미완료입니다.

## 현재 상태

선택/clipboard provider 코어 구현·새 위험 검증·기록3/3(100%)입니다. 자동완성 연결 상위1/4(25%)·Snippets1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8을 유지합니다. 전체 ETA 보류·goal active·main 직접·전체 M8 완료 전 Git 없음입니다. 모든 실행 handle은 종료됐으며 bindings/screenshot은 선행 lifecycle입니다.
