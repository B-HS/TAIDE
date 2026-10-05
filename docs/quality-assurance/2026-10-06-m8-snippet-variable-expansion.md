# M8 Snippet 변수 확장 코어

## 대상 파일

- `native/taide-native-editor/src/snippet-expansion.rs`, `src/lib.rs`
- `native/taide-native-editor/src/snippet-syntax.rs`, `src/snippet-normalization.rs`
- `native/taide-native-ui/tests/snippet-parser.rs`
- `native/taide-native-ui/tests/fixtures/snippet-parser-oracle.mjs`

## 리포트

후속 [삽입 들여쓰기](2026-10-06-m8-snippet-insertion-whitespace.md)와 [선택 값 provider](2026-10-06-m8-snippet-selection-variables.md)가 Text 정규화/선택·줄·커서/clipboard 코어를 구현했습니다. 아래 미완료 공급자는 실제 언어별 단어/파일/workspace/time/random/댓글 및 host 취득/삽입 경계를 포함한 당시 상태입니다.

선행 [기본값 정규화](2026-10-06-m8-snippet-normalization.md) 뒤에 원본 Variable.resolve의 해석/기본값 유지·빈 문자열 덮어쓰기·transform callback을 공용 Rust에 구현했습니다. 원본 TAIDE custom provider는 body를 InsertAsSnippet으로 직접 전달하므로 미해석 변수 이름을 임의로 placeholder로 바꾸지 않습니다. 해석된 값은 literal Text이며 `$1`이나 `$UNKNOWN`을 다시 파싱하지 않습니다.

해석 순서는 원본 깊이 우선 traversal이며 resolver에 앞선 마지막 Text marker의 마지막 줄을 전달합니다. Choice 옵션은 Text traversal 문맥을 바꾸지 않습니다. 해석된 변수는 기존 기본값 자식을 제거하고 새 Text를 추가하며, 미해석 변수는 기본값 내부를 계속 해석합니다. 변수 transform은 미해석 값을 빈 문자열로 받아 실행하고 placeholder transform은 이번 삽입 단계에서 실행하지 않습니다.

반환값은 AST·최종 text·UTF-8 placeholder byte 범위·marker 경로·가장 가까운 부모부터 나열한 enclosing 관계입니다. 입력 AST는 빌려 읽고 별도 임시 결과를 구성합니다. resolver/evaluator 오류·누적 byte/marker/깊이 예산·빈 choice 거절 때 부분 결과나 EditorStore 변경을 반환하지 않습니다. 메모리 전체 RSS/실제 삽입 session 예산 검증으로 확대하지 않습니다.

oracle는 설치된 원본 Monaco Transform.resolve/JS RegExp를 실제 실행합니다. Rust 검사는 그 결과를 evaluator에 주입하고 호출의 transform/value/순서와 후속 트리를 대조합니다. Rust regexp compiler/실행기 및 native/browser 실제 변수 공급자는 아직 없습니다. 합성 값 환경의 성공을 clipboard/model/workspace/time 공급자 성공으로 쓰지 않습니다.

## 검증

```sh
TAIDE_M8_ORACLE_BUN=/Users/hyunseokbyun/development/js/bun/bin/bun CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-parser snippet_expansion은 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --test snippet-parser --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

- [x] 원본23문법×3합성 값 환경69 case 비교1 첫 PASS와 독립 오류/예산1 첫 PASS입니다. build2.17초/suite.03초·filtered4입니다.
- [x] 최초 새 테스트 이름의 대문자 AST로 non_snake_case 경고1이 발생했습니다. 이름만 ast로 수정했고 테스트 재실행 없이 compile-only6.35초 exit0·경고0으로 확인했습니다.
- [x] 최신 normal Canvas Wasm check1.24초 exit0·경고0입니다. 이전 raw/정규화/font/Close/Chrome 성공은 반복하지 않았습니다.
- [ ] 실제 compiler/evaluator·변수 공급자·삽입 들여쓰기·placeholder 동기 편집/undo·추천 UI/입력/owner 수명은 다음 구현/검증입니다.

## 현재 상태

변수 확장 코어 구현/새 위험 검증/문서화 경계는3/3(100%)입니다. 자동완성 연결 상위1/4(25%)·Snippets1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8을 유지합니다. ETA 산정 보류·goal active·main 직접·전체 M8 완료 전 Git 없음입니다. 모든 실행 handle은 종료됐으며 browser bindings/screenshot은 선행 lifecycle입니다.
