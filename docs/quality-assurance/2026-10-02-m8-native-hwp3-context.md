# M8 HWP3 공유 parse context

## 대상과 범위

`native/taide-native-app/vendor/rhwp/src/parser/hwp3/{limits,mod,drawing}.rs`와 `native/taide-native-app/tests/preview-hwp3-boundary.rs`입니다. 기존 N5-P1g 확장 입력 작업이며 HWP3 앱 admission/host 완료가 아닙니다.

- [x] paragraph/control/drawing group/textbox에 동일한 명시적 context를 전달합니다. depth·node·합산 table grid를 IR 생성 전에 확인하며 drawing의 LimitExceeded를 무시하지 않습니다. TLS/global/unsafe는 도입하지 않습니다.
- [x] 기본 depth 128에서 실제 2MiB worker stack overflow/SIGABRT를 재현했습니다. 기본 depth를 32로 제한한 뒤 거절 경계와 허용 마지막 경계의 DocumentCore/page/SVG를 각각 확인했습니다. 설치된 Tokio 1.53.1 builder의 기본 stack 2MiB와 일치하는 합성 thread이며 실기 앱은 조작하지 않았습니다.
- [x] cell의 음수 padding과 큰 signed margin의 debug overflow를 재현했습니다. 단순 checked rejection은 원본 release 동작과 달라 제거했습니다. 최종 `wrapping_mul`과 동일한 16비트 재해석은 원본 결과를 유지하고 debug overflow만 제거합니다. 검사기/profile overflow 설정은 유지합니다.

## 실제 검증

공통 Cargo 인자는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 같은 성공을 다시 실행하지 않았습니다.

1. `cargo test … --test preview-hwp3-boundary hwp3_context`: shared depth/node와 개별/합산 grid 2 PASS, compile 14.02초/suite 0.00초입니다. custom depth 4의 actual drawing/textbox와 normal paragraph를 포함합니다.
2. 신규 default 거절 경계: depth 128에서 SIGABRT/exit 101(compile 0.51초), 기본 32 수정 뒤 해당 실패 검사만 1 PASS(compile 4.26초/suite 0.00초)입니다.
3. 신규 default 허용 경계: depth 31의 실제 Core/page/legacy SVG, 2MiB stack에서 1 PASS(compile 1.01초/suite 0.01초)입니다.
4. margin 초기 검사 1 RED(compile 0.61초/suite 0.00초), 중간 checked 구현 검사 성공은 최종 fidelity 근거로 사용하지 않습니다. 최종 `cargo test … --test preview-hwp3-boundary hwp3_margin` 2 PASS(compile 6.83초/suite 0.00초)이며 cell -1/-4, signed max/-4, table padding의 기존 cell overwrite, page-border u16 max/-4를 확인합니다.
5. 최종 `cargo clippy … --lib --bin taide-native-app --test preview-hwp3-boundary -- -D warnings`: exit 0, 2.86초입니다.

원본 release의 세 산술식을 `std::hint::black_box`와 `rustc -O`로 실행한 독립 합성 reference의 실제 출력은 `inner_negative=-4`, `signed_max=-4`, `border_max=-4`이고 exit 0입니다. 원본 WASM pixel 실행을 했다는 뜻은 아닙니다. [Rust wrapping_mul](https://doc.rust-lang.org/std/primitive.i16.html#method.wrapping_mul)·[Cargo release overflow checks](https://doc.rust-lang.org/cargo/reference/profiles.html#overflow-checks)의 정의를 확인했습니다.

## 남은 경계

- [ ] HWP3 앱 20MiB encoded/64MiB 합산 입력, 정확한 Additional OLE payload의 physical CFB/stream 예산과 engine 진입 전 경계를 연결합니다. 현재 앱 admission은 HWP3를 거절합니다.
- [ ] 단일 bounded parse의 DocumentCore 초기화·native raster·approved host를 연결합니다. 전체 IR/RSS/CPU/cancel/crash isolation과 원본 corpus/font/픽셀 gate는 별도 미완료입니다.
- [ ] 사용자 실기 bundle·제품 TS·root dependency/MSRV는 유지하며 N1~N8 전체 완료 뒤만 commit/push합니다.

## 같은 날짜의 후속 연결

위 완료 시점의 HWP3 거절 상태와 미연결 입력/host는 후속 기본 연결에서 해소했습니다. 현재 source/합산/단일 parser/Core/raster/host 증거는 `2026-10-02-m8-native-hwp3-admission.md`가 정본입니다. 전체 IR/RSS/CPU/cancel/crash isolation·corpus/픽셀·M8 gate는 계속 미완료입니다.
