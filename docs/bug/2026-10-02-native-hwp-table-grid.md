# Native HWP 표 grid metadata와 u16 span 경계

## 대상·증상

`native/taide-native-app/vendor/rhwp/src/model/table.rs`, `vendor/rhwp/src/render/table_layout.rs`, app `src/preview_hwp_preflight.rs`와 `tests/preview-hwp-table-budget.rs`입니다. 원본 engine source를 native로 연결하는 N5-P1g에서 확인했습니다.

작은 HWP5 TABLE record 또는 HWPX tbl XML이 매우 큰 u16 행·열 수를 선언할 수 있습니다. upstream rebuild_grid의 4000000 셀 상한은 grid reservation만 막고, renderer는 선언된 행·열로 다른 중첩 layout 버퍼를 구성합니다. 기존 decoded byte/record/XML event cap만으로 이 확대를 막지 못합니다. 별도로 cell row/col와 span의 u16 덧셈은 실제 debug panic을 일으켰습니다.

## 재현·수정

새 target 두 검사는 수정 전 RED였습니다. `Table { rows: 1, cols: 1 }`에 u16::MAX 위치/span의 합성 cell을 넣어 rebuild_grid를 실행하면 table.rs의 addition overflow로 panic했습니다. HWPX 65535×65535 tbl은 admission이 Ok를 반환했습니다. 거대 표의 renderer를 실행하거나 OOM을 유도하지는 않았습니다.

1. HWP5 tag 77 payload의 rows/cols와 HWPX tbl rowCnt/colCnt를 parser 전에 검사합니다. 각 표는 `(max(rows, 1)+1) × (max(cols, 1)+1)` slot을 checked 계산하고 문서/embedded container 전체에서 262144 slot으로 합산 제한합니다. 경계용 행·열과 aggregate를 포함하며, 각각 허용되는 128×1024 표 두 개도 합산 초과로 거절합니다.
2. vendor rebuild_grid는 위치/span을 usize로 넓힌 뒤 더하고 실제 rows/cols까지 반복을 자릅니다. grid가 없는 범위를 무의미하게 순회하거나 narrow addition으로 overflow하지 않습니다. 정상 1×1 cell은 기존처럼 Some(0)입니다.

수정 후 `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-hwp-table-budget` 2 PASS, compile 5.99초/suite 0.00초입니다.

## 위험·잔여

표 metadata 논리 상한이지 전체 IR/layout peak RSS 보호는 아닙니다. 큰 정상 표가 거절될 수 있으므로 전체 corpus와 큰 문서 호환 gate가 남습니다. 다른 payload 내부 count·paragraph/image/font expansion·CPU/cancel/crash와 실제 화면 동등성은 별도로 검증해야 합니다. 기존 TypeScript 제품 코드나 사용자 앱을 수정한 결과가 아닙니다.
