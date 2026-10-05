# M8 native XLS CFB·BIFF8 기본 reader 연결

## 대상과 완료 경계

대상은 `native/taide-native-app/src/{preview_spreadsheet,preview_spreadsheet_xls,preview_spreadsheet_biff,lib}.rs`, 격리 app의 Cargo manifest/lock, `tests/preview-spreadsheet-{xls,biff,host}.rs`와 `tests/fixtures/spreadsheet-xls-reference.json`입니다. N5-P1f의 기존 XLS reader 범위를 구현했으며 전체 Spreadsheet/N5/M8 완료가 아닙니다.

1. 표준 라이브러리와 기존 ZIP reader는 CFB를 읽지 못하고 Calamine의 CFB module은 private입니다. 직접 XLS constructor는 DIFAT chain과 Dimensions reserve/dense Range 비용을 안전하게 제한하지 못하므로 사용하지 않습니다. 격리 app에만 `cfb = "=0.15.0"`을 추가했습니다. 공개 crates.io fetch는 exit 0이며 새 lock package는 cfb와 fnv 1.0.7 두 개입니다. cfb의 MIT/MSRV 1.74와 고정 source의 chain/allocator 검증을 확인했습니다. root manifest/MSRV와 기존 제품 TS는 유지합니다.
2. CFB 입력은 기존 20MiB 한도, little-endian/version/sector layout 및 완전한 물리 sector를 먼저 검사합니다. 물리 sector 수에 따른 bool flags로 DIFAT cycle·중복 FAT·범위 밖/겹치는 FAT/DIFAT를 constructor 전에 거절합니다. Workbook 또는 Book만 메모리에서 읽고 광고 길이를 물리 파일 길이로 제한합니다. 사용자 파일·VBA·외부 URL은 실행하지 않습니다.
3. SheetJS 0.18.5가 저장한 XLS는 물리 파일 밖 FAT padding에 ENDOFCHAIN을 넣습니다. cfb의 permissive reader도 이를 거절하는 실제 실패를 확인했습니다. 해당 **주소 불가능한 padding**의 ENDOFCHAIN만 FREESECT로 정규화하는 bounded Cow를 추가했습니다. 입력 byte와 실제 sector의 링크는 바꾸지 않으며 필요한 경우에만 최대 20MiB 복사합니다. permissive library의 다른 validation은 유지합니다. 부분 sector와 malformed corpus 전체 호환성은 아직 gate입니다.
4. BIFF8 stream은 record의 길이·substream/offset·Excel 행/열·SST 수·문자 수와 retained budget을 검사합니다. 전체 dense Range/formula 평가 대신 첫 scan의 범위와 두 번째 scan의 최대 500행만 투영합니다. 숫자/RK/MulRk·boolean/error·inline/shared Unicode·Continue의 compressed/wide 전환·rich/extension skip·cached formula의 number/boolean/string을 읽습니다. 식·VBA·하이퍼링크·외부 참조를 실행하지 않습니다. SST와 retained grid는 각각 64MiB 상한이며 이는 전체 RSS 상한이 아닙니다.
5. OLE와 raw BIFF8 workbook을 기존 decode dispatcher→approved read worker→SourceReady→공유 cache→Spreadsheet surface 경로로 연결했습니다. BIFF2~5와 standalone BIFF8 worksheet는 아직 명시적으로 거절합니다. 원본의 모든 XLS 지원을 완료했다는 뜻이 아닙니다.

## 실제 검증과 실패 구분

모든 Cargo 명령은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`을 사용했습니다. 입력은 합성 fixture만이며 실제 OS 프로그램·사용자 실기 bundle/설정은 조작하지 않았습니다.

- [x] 최초 CFB `cargo test … --test preview-spreadsheet-xls -- --nocapture` — 1건 PASS, compile 8.01초·suite 0.00초. V3/V4·Workbook/Book/대소문자·mini/regular stream·중복 FAT·DIFAT/directory/regular stream cycle·거대한 stream length·missing/truncated/범위 밖/20MiB 초과를 검사했습니다. 당시 strict exit 0, 2.18초입니다.
- [x] 최초 BIFF lib/bin check — exit 0, 1.85초. 신규 test의 괄호 syntax 오류 exit 101은 fixture 코드에서 수정했습니다. parser 성공으로 세지 않습니다.
- [x] 최초 `cargo test … --test preview-spreadsheet-biff -- --nocapture` — 합성 Continue/rich/extension·RK/MulRk·cached formula·최대 65,536×256의 500행 투영·범위/uniqueCount/invalid SST/암호화/잘린 stream/offset 거절 1건 PASS, suite 0.00초입니다. 독립 원본 XLS 대조는 위 FAT padding 실패로 당시 FAIL이었습니다.
- [x] 원본 fixture는 설치된 SheetJS 0.18.5의 `write(..., {bookType:'biff8', type:'base64', bookSST})`로 inline/SST 두 XLS를 직렬화한 뒤 원본 `read`·`workbookToSheets` 결과를 한 번 고정했습니다. 각 5개 sheet의 Unicode/literal text·number/boolean/empty·C3:D4 offset·blank 3행·503 total/500 retained를 대조합니다. JSON Number의 정수/실수 enum 표현 차이에 의한 assertion FAIL은 reference의 숫자만 f64 비교로 정규화했습니다. cell 값을 바꾸거나 기대 결과를 native 결과로 교체하지 않았습니다. filter `biff8은_실제` 최종 1건 PASS, compile 1.09초·suite 0.00초입니다.
- [x] `cargo test … --test preview-spreadsheet-host xls_host -- --nocapture` — actual OLE/raw BIFF8 approved worker·SourceReady·두 내용 동일·공유 Arc/cache·outside root Read 거절·마지막 close retained 0·tracked task 0, 1건 PASS, compile 2.54초·suite 0.01초입니다. 기존 XLSX/CSV surface/선택/locale 성공을 반복하지 않았습니다.
- [x] filter `biff8은_잘못된` — 추가 substream type/standalone 거절·EOF 및 다음 Formula 전에 누락된 cached string 거절 1건 PASS, compile 1.06초·suite 0.00초입니다.
- [x] CFB 정규화 변경의 영향 검사 `cargo test … --test preview-spreadsheet-xls -- --nocapture` — 기존 안전 경계 1건 PASS, compile 1.11초·suite 0.00초입니다. 변경 없는 성공의 반복이 아니라 reader 변경 영향 검사입니다.
- [x] `cargo clippy … --lib --bin taide-native-app --test preview-spreadsheet-xls --test preview-spreadsheet-biff --test preview-spreadsheet-host -- -D warnings` — exit 0, 2.16초입니다. 같은 성공 runtime 검사는 재사용합니다.

## 공식 근거

- [cfb API](https://docs.rs/cfb/latest/cfb/)와 [고정 manifest](https://github.com/mdsteele/rust-cfb/blob/master/Cargo.toml), 설치된 cfb 0.15.0 source의 allocator/chain/open validation을 확인했습니다.
- [SheetJS write 옵션](https://docs.sheetjs.com/docs/api/write-options/)을 읽고 설치된 0.18.5의 실제 reader/serializer를 대조 기준으로 사용했습니다. 최신 문서의 지원 설명을 설치 버전의 실제 실행 결과와 혼동하지 않습니다.
- Microsoft의 [CFB](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/53989ce4-7b05-4f8d-829b-d08d6148375b), [SST](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xls/b6231b92-d32e-4626-badd-c3310a672bab), [Unicode continuation](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xls/173d9f51-e5d3-43da-8de2-be7f22e119b9), [Dimensions](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-xls/5fd3837c-9f3d-4952-8a85-ad93ddb37ced)를 구현 경계 근거로 확인했습니다.

## 남은 gate

- 후속 `2026-10-02-m8-native-biff-legacy-reader.md`에서 BIFF2~~5 기본 투영과 CJK/Latin codepage reference를 연결했습니다. 위 BIFF2~~5 미연결 문장은 해당 시점의 역사입니다. 전체 구형 XLS/codepage/formula/corpus 게이트는 계속 열려 있습니다.

- [ ] BIFF2~5/codepage·standalone worksheet·chart/macro/VBA sheet·중복 이름/원본 순서·오류 종류·RString/continuation 전체·malformed encoding/부분 sector·실파일 corpus와 모든 내용 판별을 완료합니다. 현재 fixture/필수 안전 경계를 통과한 기본 BIFF8 연결만 완료로 계산합니다.
- [ ] SST/metadata/CFB/normalized copy/worker/retained cache/renderer 전체 peak RSS와 동시 파일 합산·decode cancel/worker guard 수명·crash isolation을 검증합니다. 입력/개별 decoded/retained 상한은 전체 프로세스 메모리 증거가 아닙니다.
- [ ] 실제 pixels/scroll/auto table layout/OS/GPU/AX, 다른 provider·실제 terminal·전체 213 view·N1~N8·Rust99%/TS 제거/배포를 완료합니다. 전체 M8 완료 뒤만 commit/push합니다.
