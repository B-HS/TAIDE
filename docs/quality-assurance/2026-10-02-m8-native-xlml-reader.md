# M8 native SpreadsheetML XML 기본 읽기

## 범위와 구현

기존 N5-P1f의 내용 판별 후속입니다. `native/taide-native-app/src/preview_spreadsheet_xlml.rs`와 dispatcher, 해당 parser/host tests 및 `tests/fixtures/spreadsheet-xlml*-reference.json`을 연결했습니다. 전체 Spreadsheet/N5/M8 완료가 아닙니다.

1. UTF8/BOM·선행 공백과 UTF16LE의 `<` 내용 경계를 판별해 CSV로 XML 원문을 표시하지 않습니다. 기존 quick-xml 0.41.0과 chrono 0.4.45를 재사용하며 의존성/lock/MSRV를 변경하지 않았습니다. 읽기 승인은 기존 typed worker가 담당하고 XML의 URL/외부 파일/수식/script는 실행하지 않습니다.
2. 1차 streaming scan으로 worksheet 순서/이름·Row/Cell Index·빈 셀·MergeAcross 뒤 다음 열·전체 범위를 수집합니다. 원본의 self-closing Row는 현재 행을 범위에 넣은 뒤 Index를 적용하는 동작까지 참조 결과대로 처리합니다. ExpandedRowCount/ExpandedColumnCount와 병합 끝점 자체는 원본 raw 범위를 늘리지 않으므로 사용하지 않습니다. 역순 Index도 처리합니다.
3. 표시 grid/합산 비용을 먼저 계산하고 500행만 할당한 뒤 2차 scan으로 해당 범위에 typed 값/캐시를 넣습니다. 숫자·Boolean·Error/null·String/빈 String·기본 rich Font text·`_xHHHH_`와 UTC DateTime의 Excel serial/1900 경계를 처리합니다. JS 숫자 변환은 기존 CSV numeric을 실제 두 번째 소비자로 재사용하며 NaN/Infinity의 표시도 원본과 대조했습니다. 수식 토큰과 style/display format은 평가하지 않습니다.
4. encoded 20MiB, projected grid/합산 retained 64MiB, 1,024 sheets, 현대 Excel 최대 좌표, XML 128-depth/256-byte tag name을 제한합니다. DTD/외부·알 수 없는 일반 entity, 중복 sheet 이름, 잘린 XML/UTF16·invalid UTF8·invalid Index/merge와 큰 grid는 명시적으로 거절합니다. full RSS/CPU/즉시 cancel 상한을 증명한 것은 아닙니다.

## 실제 검증

Cargo 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 성공한 무변경 검사는 반복하지 않습니다.

- [x] 원본 fixture 생성 중 CDATA 두 입력은 SheetJS 0.18.5 자체가 `Unrecognized tag`로 거절했습니다. 한글 UTF16LE XML도 원본 `read_utf16`→`parse_xlml_xml`에서 `Bad state: worksheet|false`였습니다. 이를 성공 reference로 바꾸거나 native 생성 기대값으로 대체하지 않았습니다. 12개의 정상 UTF8/XML 입력과 별도 원본 ASCII UTF16 reference만 고정했습니다.
- [x] 신규 parser reference 검사 최초 FAIL: writer XML이 CSV의 Sheet1/Latin1 원문 셀로 표시됐습니다. XML 내용을 분기하고 streaming reader를 구현한 뒤 실패 검사 1회 재실행으로 1건 PASS, compile 3.68초·suite 0.01초입니다. 원본 writer/수동 입력의 typed 값/표시·한글/Latin·희소/역순 범위·empty/blank·self-closing Row·병합/캐시 수식·날짜·503 total/500행을 12입력에서 대조했습니다.
- [x] filter `xlml은_할당전` — 큰 단일 grid/3-sheet 합산을 할당 전에 거절하며 적합한 한 sheet는 유지합니다. XML depth/shape/DTD/entity·coordinate/index/merge·encoded 거절 포함 1건 PASS, compile 2.19초·suite 0.06초입니다.
- [x] `--test preview-spreadsheet-host xlml_host` — `.xls` 확장자의 실제 XML read/SourceReady·shared Arc cache·typed 값/503 total/500행·DTD Decode failure·close 비용 회수/task shutdown 검사 1건 PASS, compile 1.53초·suite 0.01초입니다. 기존의 unchanged 승인/outside/stale/선택/UI 성공은 재사용했습니다.
- [x] filter `xlml은_원본_ascii` — 원본 ASCII UTF16LE/BOM/UTF-16 선언 대조와 truncated/unpaired surrogate 거절 1건 PASS, compile 2.21초·suite 0.00초입니다. 한글 UTF16 원본 실패를 동일성 성공으로 계산하지 않습니다.
- [x] 최종 `cargo clippy … --lib --bin taide-native-app --test preview-spreadsheet-xlml --test preview-spreadsheet-host -- -D warnings` — 최초 constant chunks_exact lint 두 건 실패를 기존 project/안정 Rust `as_chunks`로 수정했습니다. 실패 관련 검사 1회 재실행 exit 0, 1.22초입니다. 속성 디코딩 deprecation은 설치 API의 동일 대체 메서드로 해결했고 검사기를 끄지 않았습니다. 테스트 명령의 두 filter 인자 오류는 실행 전에 정정했으며 기능 검사 실패/성공으로 세지 않았습니다.

## API 근거와 남은 gate

실제 SheetJS `parse_xlml_data`·Row/Cell 범위 처리·원본 `spreadsheet.ts`의 `sheet_to_json(raw:true,defval:null)`을 읽었습니다. 설치 quick-xml 0.41.0의 `Event::Empty`, `BytesRef::resolve_char_ref`, `decoded_and_normalized_value`, `Reader::from_str`의 UTF8 encoding 고정 구현과 chrono date API를 확인했습니다. [quick-xml의 reference 계약](https://docs.rs/quick-xml/latest/quick_xml/events/struct.BytesRef.html)과 [chrono DateTime API](https://docs.rs/chrono/latest/chrono/struct.DateTime.html)도 조회했습니다. latest 문서는 quick-xml 0.42.0이므로 고정 0.41.0 계약은 설치 source를 기준으로 구현했습니다.

- [ ] XML 전체 writer/corpus·비표준 index/attribute·namespace/case·non-BMP 숫자 entity/고립 surrogate·rich text/empty self-closing Data·모든 날짜/encoding/style는 남습니다. CDATA는 원본 오류를 재현하는 명시적 거절이며, native UTF16 Unicode decoder는 올바른 변환을 할 수 있어도 원본 실패와 동일성 검증은 하지 않았습니다. XML을 전체 XLS 완료로 계산하지 않습니다.
- [ ] HTML/SYLK/DIF/DBF/Lotus/RTF/기타 ZIP 내용과 standalone BIFF8 등 추가 형식 판별, CSS table auto-layout/scroll·실제 OS/GPU/AX·peak RSS/CPU/cancel/crash isolation은 남습니다. 보존 데이터 64MiB는 입력/파서 임시 값/metadata와 RSS 전체를 합친 상한이 아닙니다.
- [ ] 다른 preview providers·terminal·213 view·N1~N8·Rust99%/제품 TS 제거·배포는 미완료입니다. workflow/서브에이전트를 쓰지 않았고 사용자 실기 bundle·OS 설정/데이터를 조작하지 않았습니다. 전체 M8 완료 뒤만 commit/push합니다.
