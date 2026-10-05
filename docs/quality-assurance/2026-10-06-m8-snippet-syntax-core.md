# M8 Snippet raw 문법 코어

후속 기본값 전파/최종 tabstop·compiler metadata 계약과 원본212 case 근거는 [정규화 QA](2026-10-06-m8-snippet-normalization.md)입니다. 아래 기본값 미완료 표기는 당시 raw 경계이며 실제 변수/transform 실행·placeholder session·추천/삽입은 계속 미완료입니다.

## 대상 파일

- `native/taide-native-editor/src/snippet-syntax.rs`, `src/lib.rs`
- `native/taide-native-editor/LICENSE-MONACO-SNIPPET`
- `native/taide-native-ui/tests/snippet-parser.rs`
- `native/taide-native-ui/tests/fixtures/snippet-parser-oracle.mjs`

## 리포트

실제 native Application과 browser Files는 같은 NativeEditor를 사용하지만 snippet 후보를 소비하거나 추천·삽입하는 controller가 아직 없습니다. 원본 completion provider의 InsertAsSnippet은 body를 그대로 붙이지 않고 Monaco snippet session을 호출합니다. 설치된 `snippetParser.js`/`snippetSession.js`와 [공식 snippet 문법](https://code.visualstudio.com/docs/editing/userdefinedsnippets#_grammar)을 대조하여 공용 Rust raw 문법 AST를 추가했습니다. 의존성/버전/lock/MSRV 변경은 없으며 Monaco 기반 파싱 동작의 MIT 고지는 별도 license 파일에 보존합니다.

AST는 text·숫자 tabstop/default children·variable/default children·choice·transform의 pattern/options/capture format을 보존합니다. escape 문맥과 닫히지 않은 default의 prefix/기존 children 복원을 구별합니다. format은 capture·case shorthand·if/else 분기를 보존하며 malformed choice/transform은 원본처럼 일반 text 파싱으로 돌아갑니다. Index는 원본 JavaScript Number의 범위/반올림을 유지하며 ASCII 숫자/변수 이름만 토큰화합니다.

정규식 유효성은 명시적으로 받은 validator가 결정합니다. 이번 oracle는 실제 JS RegExp constructor의 성공/실패를 전달하여 raw parser만 비교합니다. Rust 정규식 실행·기본값 전파/반복 placeholder·변수 해석·최종 tabstop·선택 동기 편집·추천 화면·실제 삽입은 아직 구현하지 않았습니다. validator를 항상 true로 둔 생산 호출은 없습니다. AST만으로 원본 SnippetParser.parse 전체 또는 제품 자동완성을 완료 처리하지 않습니다.

ParseLimits의 byte/중첩/marker 생성 예산을 초과하면 EditorError::Capacity로 거절합니다. recursive 파싱은 stack 상한128을 추가로 적용합니다. 원본의 모든 깊이를 지원한다는 근거가 아니며, 향후 소비자 예산/깊은 문법 처리와 완전 동등성 범위가 정해지기 전 부모 구현 항목은 pending입니다.

## 검증

```sh
TAIDE_M8_ORACLE_BUN=/Users/hyunseokbyun/development/js/bun/bin/bun CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-parser -- --nocapture
TAIDE_M8_ORACLE_BUN=/Users/hyunseokbyun/development/js/bun/bin/bun CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-parser 설치된_monaco의 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

- [x] raw grammar oracle34 case: 실제 설치된 Monaco의 `_parse` 단계 전체 AST를 비교했습니다. 숫자 JSON1/1.0 타입 차이 RED 뒤 기대값 숫자만 f64로 정규화하여 관련1건 최종 PASS(build.35초/suite.02초·filtered1)입니다. grammar/실제 Rust 숫자 값은 수정하지 않았습니다.
- [x] byte/중첩/marker 생성 및 비정상 limit·빈/정상 입력의 독립 budget1건: 첫 실행 PASS(build2.71초/suite 전체.03초)이며 parity 실패 정정 뒤 반복하지 않았습니다.
- [x] 원본 case에는 Unicode·문맥별 escape·중첩/default·잘못 닫힌 입력·유효/빈/거절 choice·변수·regex/flags 실패 복원·조건식/shorthand/빈 format·32단 중첩·u32 초과/JS 안전 정수 반올림/매우 큰 index가 포함됩니다. 반복 default/삽입 결과 비교는 아닙니다.
- [x] Rust3파일 format·diff 검사는 exit0입니다. 원본 oracle JS는 Prettier를 적용했습니다. 처음 apply_patch의 license hunk 문법 실패는 파일 변경 없이 수정 후 적용했습니다.
- [x] normal Canvas Wasm check1.26초 exit0/경고0입니다. JS oracle는 테스트 프로세스에서만 실행하고 생산 Wasm에는 포함하지 않습니다. 현재 실행 handle은 모두 종료됐습니다.
- [ ] 기본값 전파·변수/transform 평가·placeholder session/원본 최종 tabstop의 별도 oracle 및 실제 UI 삽입은 다음 구현에서 검증합니다.

## 현재 상태

자동완성 입력 연결 상위 추적1/4(25%)이며 raw 문법 하위 경계만 구현·검증했습니다. Snippets1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8·전체 ETA 산정 보류입니다. 전체 native/browser 자동완성·UI/폰트/포커스·native shutdown·나머지 Settings/App/assets와 TS 제거/Rust99%·최종 gate는 계속 남습니다. goal active·main 직접·전체 M8 완료 전 Git 없음입니다. browser bindings/screenshot은 선행 lifecycle 소스이고 새 parser의 실제 Chrome 삽입 근거가 아닙니다.
