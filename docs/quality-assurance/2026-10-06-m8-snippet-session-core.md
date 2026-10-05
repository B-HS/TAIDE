# M8 공용 Rust 스니펫 세션 코어

## 대상 파일

- `native/taide-native-editor/src/snippet-session.rs`, `snippet-tracking.rs`, `lib.rs`
- `native/taide-native-editor/src/snippet-insertion.rs`, `snippet-whitespace.rs`, `editing.rs`
- `native/taide-native-ui/tests/snippet-session.rs`, `tests/fixtures/snippet-session-oracle.mjs`

## 리포트

실제 EditorStore 문서에 삽입한 placeholder를 UTF-16 좌표로 추적하고 canonical Rope의 UTF-8 선택으로 변환합니다. 설치된 Monaco의 `IntervalNode/nodeAcceptEdit`, `OneSnippet`, `SnippetSession.adjustWhitespace`, `Transform.resolve`를 직접 호출한 합성 oracle와 대조했습니다. 활성 그룹과 enclosing placeholder는 양 끝에서 자라고 final0은 자라지 않습니다. 원본 삽입 문맥의 leading whitespace와 원래 primary cursor를 Insertion에 보존합니다.

Session은 다중 mirror 편집, Tab/Shift-Tab, 소실된 nonempty 그룹 건너뛰기, choice 데이터, 그룹을 떠날 때 transform 요청 및 후속 줄의 원본 들여쓰기/EOL 정규화를 제공합니다. 정규식 실행 결과는 typed evaluator가 공급합니다. 이 검사는 설치된 JavaScript 실행 결과를 주입한 것이며 Rust 정규식 실행기 완료를 뜻하지 않습니다. 실제 추천 UI·NativeEditor 입력 소비자에도 아직 연결되지 않았습니다.

기존 replacement planner는 transaction 생성과 적용 wrapper로 분리했습니다. 일반 편집의 source 범위 병합/문자 경계/primary 정책은 변경하지 않았습니다. Session은 삽입과 분리된 안정된 undo group을 사용합니다. 편집 결과의 정렬된 collapsed caret 중 같은 위치는 원본 cursor normalization처럼 하나로 합치고 primary를 재지정합니다. 이로 인해 mirror 수가 달라져 세션을 종료하면 원본 controller처럼 undo 경계도 닫습니다.

owner/view/document/revision/선택·read-only·IME·범위/byte/marker 예산을 검사합니다. 조합 중 편집은 preedit와 세션을 유지한 채 거절합니다. 잘못된 evaluator 결과나 첫 적용 전 admission 실패는 canonical 문서·선택을 변경하지 않습니다. 취소는 보유 state를 해제합니다. 외부 revision 변경/undo는 현재 세션을 안전하게 회수하지만 원본의 외부 편집 decoration 추적·alternative version 기반 undo 수명까지 구현된 것은 아닙니다.

## 실제 검증 기록

아래 명령은 실행 당시의 검사 수를 기록한 것입니다. 같은 성공 검사는 다시 실행하지 않았습니다. 후속 source 변경은 새 경계 검사와 컴파일로 확인했으며 영향을 받지 않는 선행 oracle 결과를 재사용했습니다.

```sh
TAIDE_M8_ORACLE_BUN=/Users/hyunseokbyun/development/js/bun/bin/bun CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-session -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test editing 다중_선택_입력과_경계_거절은_문서에_원자적으로_적용된다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-session snippet_session은_여러cursor의_primary_조합_삭제와_겹친caret회수를_보존한다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-session snippet_session은_stale_choice를_좌표변환전에_거절한다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-session snippet_session의_편집후_선택이탈은_다음_동일group과_undo를_분리한다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

- [x] 원본 range 이동1,680개와7문법×LF/CRLF2문맥14 session·독립 실패/예산/선택이탈/undo/owner 회수의 초기2건이 첫 PASS입니다. build2.54초/suite.06초입니다. 원본 Transform 호출 시점·결과 및 선택/choice/text를 비교했습니다. 테스트 이름의 대문자 Tab 경고만 수정하고 compile-only1.11초 exit0/경고0으로 확인했으며 성공 테스트는 반복하지 않았습니다.
- [x] 변경된 일반 editing wrapper의 기존 원자적 다중 선택 검사1건 PASS(.77초/.00초·filtered2)입니다. 일반 planner 변경 위험을 직접 확인했으며 이전 source의 성공을 중복 실행하지 않았습니다.
- [x] 새 다중 cursor primary/IME 거절·한글/비BMP·그룹 이동/삭제·회수·삽입과 편집의 독립 undo1건 첫 PASS(.37초/.00초·filtered2)입니다. 이 합성 사례에서 예상했던 중복 caret 문제는 재현되지 않았고, 뒤의 단일 cursor 인접 mirror 사례에서 별도로 재현됐습니다.
- [x] 새 stale choice 검사1건은 기대한 StaleRevision을 반환하지 않아 RED(.35초/.00초)였습니다. 좌표 조회보다 revision 검사를 먼저 하도록 수정한 뒤 해당 검사만1회 PASS(1.11초/.00초·filtered3)입니다.
- [x] 새 단일 cursor 인접 mirror 삭제/동일 group 후속 편집 undo 경계 검사1건은 세션이 남아 RED(.38초/.00초)였습니다. collapsed caret 중복 제거·primary 보존과 선택이탈 undo 경계 분리 후 해당 검사만1회 PASS(1.34초/.00초·filtered4)입니다.

초기 normal Canvas Wasm1.14초 exit0/경고0을 기록했고, stale guard 변경과 mirror 회수 변경 각각의 source에서.71초 exit0/경고0을 확인했습니다. 서로 다른 source 상태의 컴파일이며 같은 성공 반복이 아닙니다. 최신 Rust7파일 exact fmt check exit0입니다. 이전 문법/정규화/변수/들여쓰기/선택 변수/Store 삽입/Chrome 성공은 재사용합니다. 실제 앱/제품 데이터/OS clipboard/입력기/VoiceOver는 사용하지 않았고 의존성·lock·MSRV·Git 변경은 없습니다.

## 남은 범위

- [ ] 실제 NativeEditor 입력/IME commit·삭제/Tab/Escape·choice 추천 UI와 세션 decoration을 연결합니다.
- [ ] 실제 JavaScript 호환 regexp compiler/evaluator와 남은 값 공급자·OS clipboard/overtyping 취득을 연결합니다.
- [ ] 원본 중첩 snippet session merge·외부 편집/undo/redo 추적과 native/browser owner shutdown을 연결합니다.
- [ ] 전체 원본 UI·포커스/AX·native/browser 실기 및 M8 최종 게이트를 완료합니다.

이번 세션 코어의 구현·직접 위험 검증·기록3/3(100%)입니다. 자동완성 상위1/4(25%)·Snippets1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8은 그대로입니다. 전체 ETA 산정 보류·goal active·main 직접·전체 M8 완료 전 Git 없음이며 실행 handle은 모두 종료했습니다. 다음은 실제 입력 controller와 세션 소비자 연결입니다.
