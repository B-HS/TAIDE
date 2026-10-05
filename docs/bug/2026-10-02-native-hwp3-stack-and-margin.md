# HWP3 worker stack과 margin overflow

## 대상

`native/taide-native-app/vendor/rhwp/src/parser/hwp3/{limits,mod,drawing}.rs`, `native/taide-native-app/tests/preview-hwp3-boundary.rs`입니다.

## 원인·관찰값

공유 depth 제한을 128로 두었지만 native recursive paragraph/control의 실제 frame은 2MiB stack에서 제한에 도달하기 전에 stack overflow/SIGABRT(exit 101)를 일으켰습니다. byte/grid 제한이나 catch_unwind로 회수할 수 있는 오류가 아니었습니다.

원본 margin의 `i16 * 4`와 음수 cell padding을 `u32`로 바꾼 뒤의 곱셈은 debug overflow panic이었습니다. 이를 checked error로만 바꾸면 원본 release/WASM에서 유지하던 16비트 modular 결과와 다릅니다.

## 수정·검증

명시적 공유 parse context의 기본 depth를 32로 제한했습니다. actual 2MiB thread의 depth 32 거절과 depth 31 Core/page/SVG가 각각 PASS했습니다. caller가 임의로 큰 max_depth를 지정하는 API는 그 caller의 책임이며 앱은 보수적 제한을 사용해야 합니다.

margin은 명시적인 16비트 wrapping으로 원본 release 변환을 보존했습니다. optimized 원본 식 reference의 세 값 -4와 final table/page-border 검사 2 PASS 및 strict exit 0입니다. source/read/grid/decoded 제한은 유지하며 overflow 검사기를 끄지 않았습니다. 정확한 명령·시간·잔여 범위는 `docs/quality-assurance/2026-10-02-m8-native-hwp3-context.md`에 있습니다.

HWP3 앱 admission은 아직 거절 상태이며 이 수정만으로 HWP3 provider나 M8 완료를 주장하지 않습니다.
