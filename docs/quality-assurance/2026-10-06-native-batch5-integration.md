# 전환 배치 5 통합 검증 (2026-10-07)

## 범위

배치 5는 구문 강조입니다(설계 문서 7절 단계 3의 3a~3d). 단계별 상세는 `2026-10-06-native-batch5-{engine-gate,token-pipeline,surface-highlight}.md`입니다. 3e(플러그인·VSIX 문법)는 다음 배치입니다.

| 단계 | 결과 |
| --- | --- |
| 엔진 게이트 | 통과. `ferriki-textmate 0.12.0` + `ferroni 1.8.1`을 감싼 `native/taide-native-syntax` 신설. 표본 32개(31개 언어 + markdown 임베드 시나리오) × 테마 2개, 3,068줄·span 19,955개에서 TS 스택 기준 자료와 불일치 0 |
| 토큰 파이프라인 | 편집 저널, 토큰 저장소, 토큰 테마(번들 테마 47개와 합성 테마 11개에서 TS 기준과 불일치 0), 전용 토큰화 스레드, 앱 조율자 |
| 표면 연결 | 번들 언어 문서를 토큰 색·굵기·기울임·밑줄·취소선으로 그림. 저장 직전에 정확히 토큰화된 줄까지의 토큰 정보를 저장 정리에 공급 |

리뷰 판정: 엔진 게이트는 차단 1건(새 JSON 자산이 저장소 prettier 검사 대상에 들어감 → `.prettierignore`에 두 경로 추가), 나머지 두 단계는 pass.

성능(구현자 측정, release 빌드): 가장 큰 문법 cpp의 첫 줄 30ms, 1만 줄 문서 cold 608ms·warm 315ms(cpp), typescript 159ms·96ms, 19,999자 줄 0.62ms 이하.

## 사용자 파일에 영향을 주는 변경

이 배치부터 `trimTrailingWhitespaceOnSave`가 켜져 있으면 plaintext 외 파일의 후행 공백이 저장 시 실제로 지워집니다. 지금까지는 토큰 정보가 공급되지 않아 native에서 이 설정이 동작하지 않았습니다. 문자열·정규식 안의 후행 공백은 보존하고, 아직 토큰화되지 않은 줄은 건너뜁니다(Monaco `trimTrailingWhitespaceCommand`와 같은 규칙).

## 메인이 직접 실행한 검증

| 검사 | 결과 |
| --- | --- |
| `cargo test --manifest-path native/taide-native-app/Cargo.toml --no-fail-fast` (전체 대상) | 604 통과, 1 실패(인수 이전부터의 XLML 테스트) |
| `cargo test --manifest-path native/taide-native-syntax/Cargo.toml` | 63 통과, 3 ignored(성능 측정 전용) |
| `cargo test --manifest-path native/taide-native-editor/Cargo.toml` | 97 통과, 1 ignored |
| `cargo test --manifest-path native/taide-native-ui/Cargo.toml --features inspection --no-fail-fast` | 213 통과 |
| `cargo test --manifest-path native/taide-remote-web/Cargo.toml --features inspection --no-fail-fast` | 53 통과 |
| `native/taide-native-app/Cargo.lock` 변경 | `taide-native-syntax`, `ferriki-textmate 0.12.0`, `ferroni 1.8.1` 추가뿐(35줄), 기존 항목의 버전 변동 없음 |
| `bunx prettier --check THIRD_PARTY_LICENSES.md native/taide-native-syntax` | 통과 |

단계별 작업자가 보고한 앱 lib 테스트 실패 10건은 빌드 캐시를 비운 뒤 `native-lsp-mock` 예제 실행 파일이 없어서 난 것이었고, 메인이 예제를 빌드한 뒤 전체 실행에서 통과했습니다.

## 임베드 문법 7종의 라이선스

엔진 게이트 단계가 네트워크 없이 채운 표를 메인이 2026-10-07에 `tm-grammars` README(`shikijs/textmate-grammars-themes` main 브랜치)로 확인해 고쳤습니다. `cpp-macro`·`sql`·`xml`(microsoft/vscode), `regexp`(MagicStack/MagicPython), `graphql`(prisma-labs/vscode-graphql), `haml`(karuna/haml-vscode)은 MIT입니다. `glsl`(polym0rph/GLSL.tmbundle)은 상류 표에 라이선스가 없습니다. `glsl`은 `cpp` 모듈이 임베드해 기존 웹 번들에도 이미 들어 있습니다. 확인 대상은 `@shikijs/langs 4.4.3`이 빌드된 정확한 시점의 표가 아니라 main 브랜치의 표입니다.

## 화면 확인

- [ ] 구문 강조의 실제 화면 — 캡처용 샘플 프로젝트와 세션 데이터를 준비해 앱을 실행했으나 화면이 잠겨 있어(최상위 창 소유자 `ScreenLock`) 창 내용이 그려지지 않았습니다. 화면이 켜진 상태에서 다시 캡처해야 합니다.

## 사용자 결정이 필요한 사항

1. **번들 테마의 잘못된 색 값**: `crates/taide-theme/resources/themes/intellij-islands-light.json:1119`의 `"#0083080"`은 7자리입니다. TS에서도 이 테마를 적용하면 토큰 색 적용이 실패하고, native도 같은 테마를 거절해 이 테마에서는 구문 강조가 꺼집니다. 올바른 색 값을 정해야 합니다.
2. **`glsl` 문법 포함 유지 여부**: 상류에 라이선스 표기가 없는 다섯 번째 회색 지대 항목입니다. 기존 웹 번들에도 들어 있어 지금은 포함해 두었습니다.
3. **토큰화 한도를 넘는 문서의 저장 정리**: 30만 줄 초과 문서는 Monaco처럼 모든 줄의 후행 공백을 지우게 구현됐습니다. 작업 지시보다 넓은 동작이고 사용자 파일을 바꿉니다.
4. **TS와 일부러 다르게 둔 것**: 세션 중 테마를 바꾸면 TS는 색이 섞이지만(테마 이름이 항상 같아 내부 레지스트리가 갱신되지 않음) native는 새 테마로 다시 토큰화합니다. TS가 줄당 500ms 한도에 걸려 일부만 강조하던 긴 줄은 native에서 끝까지 강조됩니다.

## 남은 차이와 부채

- 기울임은 합성 기울임(italic face 미사용), 밑줄 두께 1pt 고정
- 파일을 연 직후와 테마 전환 직후에는 토큰화 스레드 응답 전까지 기본색으로 보임
- 토큰화 스레드가 패닉으로 죽으면 자동 재시작이 없음
- 번들 밖 언어와 토큰 테마가 적용된 적 없는 상태에서는 저장 정리가 후행 공백을 지우지 않음(TS는 토큰 공급자가 없으면 모든 줄을 지움)
- clippy `too_many_arguments` 경고 2건 추가(`show_tokenized`, `reveal_tokenized`)
- 루트 `bun run format:check`는 체크포인트 때 들어온 `native/` 아래 JSON(키바인딩 fixture, vendored 설정) 때문에 이 브랜치에서 통과하지 못할 것으로 보입니다. 실행해 확인하지는 않았습니다.
- 인수 이전부터 실패하는 XLML 테스트 1건

## 디스크

배치 시작 전 여유 751GB, 배치와 전체 테스트 빌드 뒤 729GB(사용률 61%).
