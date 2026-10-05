# M8 native HTML spreadsheet 기본 투영

## 대상과 구현

N5-P1f의 원본 `xlsx.read`가 `.xls` 내부 HTML table을 읽는 내용 분기 후속입니다. 일반 HTML preview provider와는 다릅니다. `native/taide-native-app/src/preview_spreadsheet_html.rs`, XML dispatcher와 CSV fuzzy 값 경계, 해당 parser/host tests와 `tests/fixtures/spreadsheet-html-reference.json`을 연결했습니다. 전체 Spreadsheet/N5/M8 완료가 아닙니다.

1. 원본의 첫 opening sample/선언·HTML marker 판별과 문자열 table/row/cell parser를 재현했습니다. 복수 table은 Sheet1/Sheet2 순서이며 문서 제목/DOM id/data-v는 문자열 기반 원본과 같이 값으로 사용하지 않습니다. 일반 DOM 브라우저 파서로 대체하지 않았습니다.
2. 기존 lock의 regex 1.13.1을 같은 feature/version의 격리 app direct edge로 재사용했습니다. root/새 lock package/MSRV 변경은 없고 MIT OR Apache-2.0/MSRV 1.65입니다. 표준 Rust 문자열만으로 원본의 regex capture/split/연속 replacement를 정확히 표현하기 어려워 기존 패키지를 재사용했으며 pattern은 비신뢰 입력이 아닌 고정 상수입니다. OnceLock으로 한 번 컴파일해 재사용합니다.
3. 원본의 빈 segment/range·rowspan/colspan 뒤 위치·rich tag/공백/br·일곱 named entity 변환 순서·t/data-t의 문자열 강제·Boolean/fuzzy 숫자/기본 날짜를 처리합니다. numeric entity는 원본 HTML 변환이 해석하지 않으므로 literal `&#65;`를 유지합니다. CSV와 실제 두 번째 소비자에서 fuzzy 숫자/날짜를 공유했으며 CSV 동작은 바꾸지 않았습니다.
4. 1차 scan의 범위로 전체 행 수와 표시 grid/합산 retained 비용을 계산하고 500행만 할당한 뒤 2차 scan으로 visible 값만 보존합니다. table 개수/Sheet vector 비용도 할당 전에 예약하고 큰 grid·합산/좌표/span/merge를 거절합니다. 파일 링크·URL·이미지·script/iframe은 문자열 데이터만 읽으며 외부 열기/실행/다운로드하지 않습니다.

## 실제 검증

Cargo 공통 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 최초 direct edge lock 갱신 check만 `--locked` 없이 offline으로 실행했고 새 package는 없습니다. 성공한 무변경 검사는 재사용했습니다.

- [x] reference 검사 최초 FAIL: 실제 HTML 입력에 XML Workbook root 오류가 나왔습니다. 내용 분기를 연결하고 원본 문자열 처리/range/merge를 구현한 뒤 실패 검사 1회 재실행으로 1건 PASS, compile 5.79초·suite 0.11초입니다. SheetJS 0.18.5 실제 HTML writer와 12입력의 복수 표·typed 값/강제 문자열·한글/Latin·rich 공백/entity·raw URL/script text·span/빈 범위·503 total/500행/BOM을 대조했습니다. 날짜 reference 생성은 Asia/Tokyo이고 native 검사는 FixedOffset +09:00으로 고정했습니다.
- [x] `cargo check … --offline --lib --bin taide-native-app` — exit 0, 1.43초입니다.
- [x] filter `html은_내용분기` — 실제 dispatcher·비실행 링크/script 본문·큰 projected grid/3-table 합산·적합한 단일 table·span/table-count/불완전 HTML 거절 1건 PASS, compile 2.40초·suite 0.21초입니다. 64MiB는 retained 데이터 경계이며 전체 RSS를 측정한 결과가 아닙니다.
- [x] `--test preview-spreadsheet-host xlml_html_host` — 기존 XML host 검사에 HTML source를 추가했습니다. `.xls` 안의 XML/HTML 읽기·SourceReady·공유 Arc/typed 값·복수 표/DTD Decode failure·close 회수/task shutdown 1건 PASS, compile 1.11초·suite 0.11초입니다. XML host 이름은 기존 `xlml_host`에서 이 combined 이름으로 바뀌었으며 기존 XML/CSV/XLS/BIFF의 다른 unchanged 성공은 재사용했습니다.
- [x] 최종 `cargo clippy … --lib --bin taide-native-app --test preview-spreadsheet-html --test preview-spreadsheet-xlml --test preview-spreadsheet-host -- -D warnings` — exit 0, 1.76초입니다. 초기 reference test에서 아직 쓰지 않던 dispatcher import 경고는 후속 실제 dispatcher 경계 검사에서 사용하며 사라졌습니다. 검사기를 끄지 않았습니다.

## 근거와 미완료 gate

원본 SheetJS `parse_xlml_xml`, `html_to_workbook`, `html_to_sheet`, `htmldecode`, `parsexmltag`와 실제 read/HTML writer를 확인했습니다. 기존 regex 1.13.1 source/manifest와 [공식 Regex API](https://docs.rs/regex/latest/regex/struct.Regex.html)의 captures/split/replace/byte boundary·iteration complexity를 확인했습니다. 원본은 browser DOM 기반 table parser와 다른 경로를 사용하므로 DOM 동작을 통과 기준으로 삼지 않았습니다.

- [ ] 모든 malformed/nested HTML·style/숨김·tag/attribute·UTF16/non-BMP opening sample·ECMAScript/Rust Unicode whitespace/시간대/DST·날짜 corpus는 남습니다. native의 invalid/0/fraction/거대 span은 명시적으로 거절하며 원본의 무한/비정상 좌표를 따라가지 않습니다.
- [ ] 전체 regex iteration/merge CPU·파서 임시 문자열/source/metadata+RSS·즉시 cancel/crash isolation과 정확한 CSS table/scroll/실제 OS/GPU/AX는 남습니다. encoded 20MiB/retained 64MiB/table/merge 상한이 모든 중간 메모리나 CPU 시간 상한을 증명한 것은 아닙니다.
- [ ] 추가 XLS/ZIP/SYLK/DIF/DBF/Lotus/RTF 형식·기존 XML/XLS 전체 corpus와 일반 HTML/HWP/audio/video provider·terminal·213 view·N1~N8·Rust99%/제품 TS 제거·배포는 미완료입니다. 사용자 실기 bundle/OS/데이터를 조작하지 않았으며 workflow/서브에이전트를 쓰지 않았습니다. 전체 M8 완료 뒤만 commit/push합니다.
