# 제품 Rust·TypeScript 비율 측정

## 대상과 정의

대상 도구는 `tools/migration-metrics`입니다. 독립 Cargo workspace이며 제품 의존성을 바꾸지 않습니다. 정책 식별자는 `tracked-product-nonblank-physical-v1`입니다.

- Git이 추적하는 `src/`, `src-tauri/src/`, `crates/*/src/`의 `.rs`, `.ts`, `.tsx`만 포함합니다. 실험·도구·문서·의존성·산출물은 제외합니다.
- 테스트·fixture·generated·testing·test-support 디렉터리, `.test.`·`.spec.` 파일, `test_support.rs`, 생성 `src/shared/api/bindings.ts`는 제외합니다.
- Rust는 syn AST로 순수 `cfg(test)` 항목과 `#[test]` 함수를 제외합니다. `all(test, unix)`는 제외하지만 `any(test, debug_assertions)`는 제품에서도 존재할 수 있으므로 유지합니다. 문자열의 중괄호나 파일 중간 테스트 모듈 이후 코드를 정규식으로 잘라내지 않습니다.
- 공백 행만 제외한 물리적 줄 수입니다. 제품 주석은 포함합니다. AST 항목과 제품 코드가 같은 물리적 줄에 있으면 해당 줄 전체를 제외하므로 최종 소스는 일반적인 줄 분리 포맷을 사용해야 합니다. 의미 기반 코드량이나 실행시간 비율이 아닙니다.
- 동일 정책에서 `Rust / (Rust + TS + TSX) × 100`을 계산합니다. 파일 수는 별도로 출력하며 비율의 분자로 사용하지 않습니다.

Rust 구문 분석 실패·Git 실패·빈 제품 소스는 오류로 종료합니다. 플랫폼별 제품 코드의 합계를 측정하며 현재 운영체제에서 실행되는 코드만 계산하지 않습니다. 매크로 확장과 모든 조건부 컴파일의 의미를 추정하지 않습니다.

## 실행

저장소 루트에서 실행합니다.

```sh
cargo run --manifest-path tools/migration-metrics/Cargo.toml --locked -- --revision 2824005
cargo run --manifest-path tools/migration-metrics/Cargo.toml --locked -- --worktree
```

기본값은 HEAD의 불변 snapshot입니다. `--worktree`는 추적 파일의 현재 내용을 사용하며 untracked 파일은 포함하지 않습니다. 완료 비율은 제품 파일을 commit한 뒤 정확한 commit snapshot으로 다시 측정해야 합니다.

## 기준값과 검증

2026-09-30, commit `2824005573ea1e1e6a70136499016b14aa06a2bf`의 실제 출력:

```json
{"policy":"tracked-product-nonblank-physical-v1","commit":"2824005573ea1e1e6a70136499016b14aa06a2bf","worktree":false,"rust_files":351,"typescript_files":577,"rust_lines":42530,"typescript_lines":47336,"rust_percent":47.3260}
```

`cargo test --manifest-path tools/migration-metrics/Cargo.toml --locked`의 단위 검사 5건이 통과했습니다. 경로 제외, 파일 중간 테스트와 raw string, 복합 cfg, impl 내부 테스트 메서드, 구문 오류를 검사합니다. 기준값 측정도 exit 0입니다. 같은 입력의 성공 검사는 재실행하지 않습니다.

기존 M8 착수 문서의 82,349 / 52,212줄은 임시 참고치입니다. 공백·inline Rust 테스트 등의 제외 규칙이 달라 이 기준값과 직접 비교하지 않습니다. 제품 런타임의 TS/Tauri 참조 0, 기능·성능·보안·배포 게이트는 이 비율과 별도이며 숫자만으로 M8을 완료 처리하지 않습니다.
