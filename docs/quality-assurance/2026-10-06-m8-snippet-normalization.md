# M8 Snippet 기본값 전파·최종 tabstop

## 대상 파일

- `native/taide-native-editor/src/snippet-normalization.rs`
- `native/taide-native-editor/src/snippet-syntax.rs`, `src/lib.rs`
- `native/taide-native-ui/tests/snippet-parser.rs`
- `native/taide-native-ui/tests/fixtures/snippet-parser-oracle.mjs`

## 리포트

후속 [변수 확장 코어](2026-10-06-m8-snippet-variable-expansion.md)가 일반 해석 callback·text/범위/부모를 구현했습니다. 아래 변수 미완료는 실제 값 공급자/regexp 실행·session/소비자 경계를 포함한 당시 상태이며 후속 좁은 코어 성공으로 전체 완료 처리하지 않습니다.

선행 [raw 문법 코어](2026-10-06-m8-snippet-syntax-core.md) 뒤에 설치된 Monaco SnippetParser.parseFragment의 기본값 참조/복제와 ensureFinalTabstop을 연결했습니다. 첫 nonempty 기본값·나중 정의/반복 placeholder·choice·nested children·순환 index를 보존합니다. 최종0은 기본값 전파에서 제외하고, 실제 최종0의 존재와 insert/enforce 옵션을 구별합니다.

기본값은 최초 marker의 child 객체 참조를 보존합니다. 원본처럼 교체 전 객체가 남아 있는 ID arena에서 새 복제와 parent 교체를 수행하고, 마지막에 살아 있는 트리만 반환합니다. 처음 저장한 문자열 snapshot으로 모든 occurrence를 덮거나 순환을 임의로 빈 문자열 처리하지 않습니다. 파싱/정규화는 소유한 임시 트리에서 수행하므로 Capacity/실패 결과를 부분 삽입으로 반환하지 않습니다.

원본 Transform.clone은 regexp.source를 다시 컴파일하면서 ignoreCase/global만 유지합니다. 이번 compiler 계약은 canonical source·해당 flags를 반환하고 복제 시 실제 재컴파일 성공 여부를 확인합니다. 초기 regex/flags 거절은 원본 raw text 복원이고, 복제 compiler 실패는 전체 정규화 실패입니다. 실행기를 임의로 다른 정규식 의미로 대체하지 않았으며, 현재 Rust native/browser compiler·transform 실행기는 아직 연결하지 않았습니다. oracle만 실제 JS RegExp 결과를 제공합니다.

marker/문자 byte와 중첩 budget을 복제에도 적용합니다. compiler source 길이와 checked arithmetic을 검사하며 AST arena의 누적 복제 비용을 제한합니다. 이 경계는 RSS 전체/실제 삽입 session 예산 완료 근거가 아닙니다.

## 검증

```sh
TAIDE_M8_ORACLE_BUN=/Users/hyunseokbyun/development/js/bun/bin/bun CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-parser snippet_normalization은 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

- [x] 원본 정규화/최종 tabstop oracle1 첫 PASS:53개 문법×insert/enforce4종=212 case의 전체 AST·초기 미해석 text·placeholder traversal/UTF-8 byte range가 일치합니다. 새2건 build2.52초/suite.05초·filtered2입니다.
- [x] 독립 복제 budget1 첫 PASS: raw parse는 허용되지만 기본값 복제의 누적 byte/marker·완성 트리 깊이가 초과되는 입력과 oversized compiler source를 Capacity로 거절했습니다. 실제 EditorStore/UI 변경 검사는 아닙니다.
- [x] 원본의 늦은 기본값·첫 값 우선·동일 index/다른 index 순환·nested variable 내부 placeholder·choice 전파·최종0 body·transform source/flags 복제·Unicode range·32단 서로 다른 index 중첩을 포함합니다. 모든 regexp를 Rust 실행기로 평가한 검사가 아닙니다.
- [x] Rust4파일 format·diff 검사는 exit0이며 oracle JS를 Prettier로 확인했습니다. 기존 raw34/budget2와 title/Close/Chrome 성공은 반복하지 않았습니다. 의존성/버전/lock/MSRV/제품 TS/Git 변경은 없습니다.
- [x] normal Canvas Wasm check1.13초 exit0/경고0입니다. 현재 실행 handle은 모두 종료됐으며 bindings는 선행 lifecycle입니다.
- [ ] 변수 해석·문맥/들여쓰기·transform 실행·placeholder 동기 편집/undo·native/browser 추천/삽입 및 전체 owner 수명은 후속 구현/검증입니다.

동일 index32단의 raw 문법은 선행 검사에서 확인했지만 이번 완전 정규화212에는 포함하지 않았습니다. 복제 깊이/메모리 한계와 원본 깊은 순환의 정규화 비용은 소비자 예산이 연결될 때 별도 확인합니다. 이 생략 때문에 전체 중첩/자원 동등성 gate를 완료로 표기하지 않습니다. 공식 문법의 일반 정의보다 설치된 원본 버전의 실제 기본값/복제 동작을 우선 대조했습니다.

## 현재 상태

이번 정규화 구현/검증/기록3/3(100%)이며 자동완성 연결 상위1/4(25%)·Snippets1/4(25%)·provider2/4(50%)·M8 363/433(83.83%)·최종0/8입니다. 원본 compiler/변수/transform 평가와 실제 추천·삽입이 없어 자동완성 완료가 아닙니다. 전체 ETA 산정 보류·goal active·main 직접·전체 M8 완료 전 Git 없음입니다. bindings/screenshot은 선행 lifecycle이며 이번 초기 text/range 검사를 실제 Chrome 삽입으로 쓰지 않습니다.
