# M8 Snippet 삽입 들여쓰기 코어

## 대상 파일

- `native/taide-native-editor/src/snippet-whitespace.rs`, `src/lib.rs`
- `native/taide-native-ui/tests/snippet-parser.rs`
- `native/taide-native-ui/tests/fixtures/snippet-parser-oracle.mjs`

## 리포트

원본 SnippetSession.adjustWhitespace/normalizeIndentation을 설치된 Monaco에서 직접 실행해 변수 해석 이전 Text marker의 삽입 위치별 들여쓰기를 공용 Rust로 옮겼습니다. 첫 text offset0은 기존 들여쓰기만 정규화하고, 이후 줄 및 앞선 marker가 줄바꿈으로 끝난 text 시작은 owner 줄의 cursor 이전 leading whitespace를 더합니다. choice 문자열/옵션은 변환하지 않지만 이후 text의 줄 시작 판정에 반영합니다. 중첩 placeholder/variable 기본값도 원본 traversal 순서로 처리합니다.

들여쓰기 조정이 꺼져도 Text의 CRLF/CR/LF는 문서 EOL로 정규화합니다. 탭은 다음 indent tabstop까지의 열 수를 구하고 insertSpaces에 따라 탭·나머지 공백으로 변환합니다. NBSP/다른 Unicode 공백은 원본처럼 들여쓰기 공백으로 취급하지 않습니다. UTF-8 byte cursor는 경계에서 검증하며 줄 중간/범위 밖 byte·tab size0·줄 안 newline을 거절합니다.

새 AST와 정규화 문자열은 소유한 임시 결과이며 입력 AST와 EditorStore를 변경하지 않습니다. 누적 AST byte/marker/깊이·렌더 text와 EOL 확장 예산을 제한하고 초과 시 부분 결과를 반환하지 않습니다. 실제 UI 삽입·다중 cursor overwrite·placeholder session은 이 성공으로 완료 처리하지 않습니다.

## 검증

```sh
TAIDE_M8_ORACLE_BUN=/Users/hyunseokbyun/development/js/bun/bin/bun CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test snippet-parser snippet_whitespace는 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
```

- [x] 원본15문법×6삽입 문맥90 case AST·text·UTF-8 placeholder 범위/경로/enclosing 비교1 첫 PASS입니다. nested/choice/빈 placeholder·같은 줄/줄 시작·부분 owner whitespace·탭/공백·LF/CRLF·조정 꺼짐을 포함합니다.
- [x] 독립 문맥/확장 예산 거절1 첫 PASS입니다. 새2건 build2.43초/suite.06초·filtered6이며 경고0입니다. 입력 AST 불변을 확인했습니다.
- [x] 최신 normal Canvas Wasm check.74초 exit0·경고0입니다. 이전 raw/정규화/변수69/Chrome 성공은 반복하지 않았습니다.
- [x] 설치된 SnippetSession의 모듈 API 조회는 static method 확인 후 모듈 타이머 때문에 살아 있었습니다. 해당 합성 조회 PID53308만 TERM으로 회수했고 종료143을 확인했습니다. 실제 oracle는 stdout flush 뒤 자기 프로세스를 정상 종료하며 제품 앱/시스템 설정은 건드리지 않았습니다.
- [ ] 실제 변수 공급자/compiler·다중 cursor 삽입/undo·placeholder 편집/session·native/browser 추천 UI/owner는 후속 구현/검증입니다.

## 현재 상태

삽입 들여쓰기 구현/새 위험 검증/기록3/3(100%)이며 자동완성 상위1/4(25%)·Snippets1/4(25%)·M8 363/433(83.83%)·최종0/8을 유지합니다. 전체 ETA 보류·goal active·main 직접·전체 M8 완료 전 Git 없음입니다. bindings/screenshot은 선행 lifecycle입니다.
