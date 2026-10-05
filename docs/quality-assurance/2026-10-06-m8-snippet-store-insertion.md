# M8 actual EditorStore Snippet 삽입 transaction

## 대상 파일

- `native/taide-native-editor/src/snippet-insertion.rs`, `src/lib.rs`
- `native/taide-native-editor/src/store.rs`
- `native/taide-native-editor/tests/snippet-insertion.rs`

## 리포트

준비된 Expansion과 cursor별 replacement byte 범위를 실제 EditorStore의 canonical 문서에 단일 transaction으로 삽입합니다. 편집 순서는 문서 위치로 정렬하지만 결과 snippet과 선택은 원래 cursor 순서를 보존합니다. 앞선 교체의 누적 removed/inserted byte로 placeholder·snippet 위치를 보정하고, 각 snippet의 첫 nonzero index(없으면 final0) occurrence 전체를 선택합니다. 원본 primary cursor의 첫 선택을 primary로 유지하며 다른 공유 view의 선택/fold/composition 보정은 기존 Store가 수행합니다.

삽입 전 원본 ViewState의 문서 ID·선택과 현재 owner를 비교합니다. 문서 revision이 같아도 선택이 바뀐 오래된 후보는 Refused입니다. owner 회수/다른 문서·stale revision·read-only·IME 조합 중·잘못된 UTF-8/겹침·placeholder 범위/경로/enclosing·누적 text/placeholder 예산을 거절합니다. 실제 문서 용량 초과는 Store의 기존 원자적 admission을 사용합니다. 준비된 typed Expansion을 소비하는 코어이며 외부 wire 입력/regexp 실행기는 아닙니다.

Store에 apply_separate를 추가해 기존 apply와 같은 validation/원자적 기록을 사용하면서 앞뒤 undo group과 병합하지 않도록 했습니다. 일반 apply는 기존 can_merge 조건을 그대로 유지합니다. 삽입 실패 전에 undo 경계를 임의로 변경하지 않습니다. empty edit는 제거하므로 빈 body/빈 range 삽입은 문서 revision/dirty/undo를 만들지 않으며 필요한 선택 변경만 수행합니다.

이번 구현은 실제 문서 삽입과 초기 선택·undo 경계까지입니다. placeholder 범위는 삽입 시점의 snapshot이며 후속 edit에 자동 추적되는 decoration/session이 아닙니다. 원본처럼 Tab/Shift-Tab 이동 중 transform 적용·mirror 동기 편집·choice UI·중첩 session merge/취소·undo 이후 retire와 actual NativeEditor 추천 입력 controller는 아직 남습니다. 초기 선택을 전체 자동완성 완료로 표기하지 않습니다.

## 검증

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-insertion -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-insertion snippet삽입_거절과_빈삽입은_문서_선택_조합_undo를_부분변경하지_않는다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test store preedit은_view에만_남고_undo_group과_저장_완료의_revision을_지킨다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-insertion snippet삽입은_같은revision의_선택변경과_중복범위_owner회수를_거절한다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

- [x] 실제 다중 cursor 삽입/초기 mirror 선택·primary·공유 view·앞뒤 동일 group의 독립 undo/redo1 첫 PASS입니다. build.33초/suite.00초입니다. 이후 ViewState admission을 추가한 source에서 기존 검사의 호출 signature를 갱신·컴파일했고 이전 성공을 반복하지 않았습니다. 최신 admission 검사에는 변경된 선택의 새 유효 삽입도 포함합니다.
- [x] 실패/빈 삽입1 최종 PASS(.25초/.00초·filtered1)입니다. byte/revision/count/capacity/잘못된 span·IME/read-only와 문서/선택/undo 불변을 확인했습니다. 최초 fixture의 read-only undo를 false로 예상한 마지막 assert만 실제 Store 계약인 ReadOnly로 정정했습니다.
- [x] 일반 apply의 기존 IME/연속 같은 group merge/undo/save 영향1은 변경된 Store source에서1회 PASS(.40초/.00초·filtered6)입니다. 이전 source의 같은 성공을 중복 실행한 것이 아닙니다.
- [x] 최신 ViewState admission/selection-only stale·겹침/원자적 거절·final0 body 새 유효 삽입/undo·owner detach1 첫 PASS(.53초/.00초·filtered2), normal Canvas Wasm.76초 exit0/경고0입니다. 같은 성공을 반복하지 않았습니다.
- [x] 최초 새 fixture에 실제 OpenedFile에 없는 encoding/byte_length 필드를 적어 compile이 실패했습니다. 실제 DTO 선언을 읽고 path/byte_size로 정정한 뒤 최초 실행을 진행했습니다. 검사 억제/의존성/lock/MSRV/Git 변경은 없습니다.

Rust 변경4파일 format check exit0입니다. 기존 파싱/정규화/변수 확장/들여쓰기/선택 변수/Chrome 성공은 재사용하며 실제 native/browser 전체 입력/종료 검사로 대체하지 않습니다. 제품 파일이나 clipboard를 읽거나 쓰지 않고 합성 Store만 사용했습니다.

## 미완료와 현재 상태

- [ ] 원본 동기 placeholder 편집·Tab/Shift-Tab/transform/choice·중첩 session/undo 취소와 tracked 범위/owner 수명을 연결합니다.
- [ ] actual compiler/evaluator·언어별 word/파일/workspace/time/random/댓글·OS clipboard/overtyping 취득을 연결합니다.
- [ ] 실제 NativeEditor 추천 UI/키/입력 controller와 native/browser 후보/삽입/owner shutdown을 연속 검증합니다.

삽입 transaction 구현/새 위험 검증/기록3/3(100%)이며 자동완성 상위1/4(25%)·Snippets1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8입니다. 전체 ETA 보류·goal active·main 직접·전체 M8 완료 전 Git 없음입니다. 실행 handle은 모두 종료됐으며 bindings/screenshot은 선행 lifecycle입니다.
