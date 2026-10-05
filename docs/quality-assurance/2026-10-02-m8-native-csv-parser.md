# M8 native CSV parser 기본 투영

## 대상과 완료 범위

대상은 `native/taide-native-app/src/{preview_spreadsheet,preview_spreadsheet_csv,lib}.rs`, app manifest/lock, `tests/preview-spreadsheet-csv.rs`와 `tests/fixtures/spreadsheet-csv-reference.json`입니다. 원본 SheetJS의 실제 ArrayBuffer 입력 결과 27개를 고정해 셀 타입·JavaScript 표시 문자열·sheet 이름·행 수·truncated를 비교했습니다. 이 완료는 CSV parser 기본 경계이며 전체 Spreadsheet provider/M8 완료가 아닙니다.

1. delimiter의 첫 1024 UTF-16 unit 추정·sep=·따옴표/줄바꿈·CRLF·빈 행·trailing field와 ragged padding을 원본 입력 결과에 맞춥니다. UTF-8 BOM·UTF-16LE BOM은 decoding하고 non-BOM bytes는 원본처럼 binary 문자열로 취급합니다. CJK non-BOM 동작을 임의로 개선하지 않습니다.
2. 대문자 TRUE/FALSE, 원본 fuzzy number의 radix·쉼표·달러·백분율·괄호, cached value 없는 수식/null 및 literal 문자열 수식을 투영합니다. 수식·매크로·외부 URL은 실행하지 않습니다. numeric overflow의 경로별 차이와 JavaScript의 NaN/Infinity/-0/지수 표기 경계를 유지합니다.
3. 기존 timezone의 legacy date와 UTC ISO date를 구분합니다. 실제 Local wrapper를 사용하며 reference 검사는 주입한 JST 고정 offset으로 실행합니다. reference 생성의 TZ는 해당 Bun child process에만 적용했고 시스템 timezone/입력기를 변경하지 않았습니다. 실제 local wrapper의 numeric date와 ISO date 변환도 검사했습니다.
4. encoded 20MiB·Excel 행/열 범위·첫 500행 및 전체 행 수·64MiB retained workbook 상한을 적용합니다. 첫 scan에서 범위와 grid 비용을 계산해 dense 할당 전에 거절하고 둘째 scan에서 보이는 셀과 문자열 비용만 유지합니다. 전체 임시 문자열/RSS/cancel/crash isolation을 완료한 것은 아닙니다.

## 의존성 근거

CSV crate는 추가하지 않았습니다. 원본의 fuzzy coercion은 표준 CSV reader만으로 재현되지 않습니다. 설치된 동일 source와 공식 API를 확인한 chrono 0.4.45·num-bigint 0.4.8·num-traits 0.2.19를 기존 native lock에서 direct edge로 재사용했습니다. chrono는 날짜 rollover/Local/TimeZone, BigUint와 ToPrimitive는 임의 길이의 2·8·16진수에서 Number의 f64 반올림을 유지하기 위해 필요합니다. chrono/num-bigint/num-traits는 MIT 또는 Apache-2.0이며 root MSRV를 바꾸지 않았습니다.

새 package는 [ryu-js 1.0.3 공식 API](https://docs.rs/ryu-js/1.0.3/ryu_js/) 하나입니다. Rust 표준 숫자 표시와 원본 `String(number)`는 지수 경계·-0·비유한 수에서 다릅니다. ryu-js는 ECMAScript 표시를 제공하며 Apache-2.0 또는 BSL-1.0·MSRV 1.71입니다. 공개 crates.io fetch만 정상 escalation으로 실행했고 .env/키/인증 파일을 읽지 않았습니다. 제품 TS·root manifest/lock·사용자 실기 bundle은 이 변경에서 유지했습니다.

## 실제 결과와 정정

- 첫 compile에서 Days의 private tuple 생성과 scan closure의 중복 borrow가 실패했습니다. Days::new와 EOF guard로 수정했고 check exit 0(1.78초)이었습니다. date 시간 값을 읽는 eager Option 대안과 두 글자 literal 수식의 역방향 slice도 테스트 전에 제거했습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-spreadsheet-csv -- --nocapture` — 당시 1건 PASS, compile 7.23초·suite 0.08초. 원본 27 입력 대조, Local 날짜 경계, 510행 중 500행/total, encoded oversize와 숫자 표시를 확인했습니다.
- 기존 wide fixture는 Excel 열 수 초과만 거절하므로 grid 메모리 보호의 증거가 아니었습니다. 허용된 16,384열×500행으로 바꾸고 오류 종류까지 확인하는 별도 검사를 만들었습니다. 통과한 27 입력 검사는 다시 실행하지 않았습니다.
- [x] 같은 cargo 명령의 filter `csv는_허용된_열_수의_큰_grid도_할당_전에_거절한다` — 1건 PASS, compile 1.10초·suite 0.52초. 허용된 열 수에서 grid budget 오류, 16,385열에서 dimensions 오류를 구분했습니다.
- 첫 strict는 collapsible_if와 고정 크기 chunks_exact 스타일 경고 두 건으로 실패했습니다. 검사기를 끄지 않고 같은 조건식과 as_chunks 반복으로 수정했습니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app --test preview-spreadsheet-csv -- -D warnings` — 수정 후 exit 0, 1.93초. 두 수정은 조건/UTF-16 pair 순서를 바꾸지 않으므로 이미 통과한 runtime 대조를 재사용합니다.

## 잔여 gate

- [ ] CSV 전체 corpus·separator Unicode surrogate·malformed UTF-8/UTF-16·추가 JS Date parsing/DST gap·원본의 모든 숫자 및 날짜 경계를 비교합니다. 27 reference의 whitespace 입력은 literal 역슬래시-u0085를 포함하므로 실제 NEL 문자 대조를 통과했다고 주장하지 않습니다. 이 parser에 걸리는 실제 파일의 차이가 발견되거나 전체 parity gate 실행 때 추가합니다.
- [ ] XLS의 CFB/BIFF constructor allocation·chain cycle·SST count와 dense cell/formula Range를 안전하게 제한한 reader를 연결합니다. Calamine::Xls를 앱에서 직접 생성하지 않았습니다.
- [x] XLSX·CSV approved worker/cache/실제 Spreadsheet surface의 기본 path/token/close/root 변경·aggregate 128MiB를 연결했습니다. 이후 host/UI·최종 CSV/열 폭/AX 결과와 전체 미완료 gate는 `2026-10-02-m8-native-spreadsheet-preview.md`에 기록했습니다. XLSX 및 다른 preview의 같은 dependency 상태 성공은 재사용합니다.
- [ ] 원본 sheet 선택 유지/clamp·empty/error/truncated·3 locale/theme/scroll·actual OS/GPU/AX·전체 RSS/cancel/crash·terminal/다른 provider·N1부터 N8·Rust99%/TS 제거/배포는 미완료입니다.
