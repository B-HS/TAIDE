# M8 native XLSX 스트리밍 parser

## 대상과 좁은 완료 범위

대상은 `native/taide-native-app/{Cargo.toml,Cargo.lock,src/lib.rs,src/preview_spreadsheet.rs,tests/preview-spreadsheet.rs}`입니다. 원본 `src/shared/lib/{spreadsheet.ts,spreadsheet.test.ts}`·`src/features/preview/spreadsheet-preview.tsx`와 설치 SheetJS의 실제 `sheet_to_json`·DSV·workbook 판별을 읽었습니다. 현재 완료는 XLSX parser의 기본 투영과 할당 전 보호입니다. XLS·CSV·approved worker/cache/실제 Spreadsheet 탭은 미연결이며 전체 provider/M8 완료가 아닙니다.

1. workbook 순서대로 sheet name·typed text/number/boolean/null·blank rows·총 행 수를 유지하고 첫 500행만 retained grid로 만듭니다. 원본의 `header: 1, raw: true, defval: null`처럼 cached formula 값을 읽으며 계산/매크로 실행은 하지 않습니다. 오류 셀은 null, 날짜 형식이 붙은 숫자는 원래 Excel serial number입니다. XML entity로 들어온 script 문자열은 문자열로 유지합니다.
2. Calamine의 dense `worksheet_range`를 사용하지 않고 공식 `worksheet_cells_reader`/`next_cell` API로 XML cell을 읽습니다. declared dimension의 시작 위치와 blank rows를 유지하며 declaration이 없는 경우 cell 범위를 추론합니다. out-of-order rows에서 origin이 앞당겨지면 첫 500행 밖 sparse 값을 split_off로 회수합니다. 비용을 새 셀/교체/회수에 따라 누적하므로 매 셀마다 전체 map의 비용을 재순회하지 않습니다.
3. encoded bytes 20MiB, ZIP 전체 decompressed aggregate 64MiB·retained workbook 64MiB·1024 sheets·Excel의 행/열 범위를 검사합니다. 기존 zip 2.4.2로 실제 decompression/CRC를 포함한 사전 읽기 상한을 적용하고 XML DTD를 거절합니다. filename을 추출하거나 filesystem/URL을 읽지 않습니다. projection dense allocation 전에 rows×columns×Cell 및 문자열 예산을 확인합니다.
4. Calamine 소스의 `read_shared_strings`는 비신뢰 `uniqueCount`를 그대로 `Vec::reserve`에 사용합니다. 모든 XML entry를 quick-xml로 사전 검사하고 해당 값이 `64MiB / (size_of::<String>() × 2)`를 넘거나 올바른 정수가 아니면 Calamine 생성 전에 거절합니다. metadata·XML 임시 버퍼/스타일/전체 shared strings·sparse와 dense의 중첩 수명을 포함한 실제 RSS 절대 상한이나 crash isolation을 완료한 것은 아닙니다.

## 의존성과 API 근거

현재 Rust dependency에는 XLS/XLSX reader가 없으며 기존 ZIP decoder만으로 binary XLS·OOXML 셀/문자열/날짜를 파싱할 수 없습니다. 최신 확인한 [Calamine 0.36.1 manifest](https://github.com/tafia/calamine/blob/master/Cargo.toml)·[공식 Reader API](https://docs.rs/calamine/0.36.1/calamine/trait.Reader.html)·[cell 타입](https://docs.rs/calamine/0.36.1/calamine/enum.Data.html)와 설치된 동일 source를 확인했습니다. Calamine은 MIT·MSRV 1.88입니다. 기본/chrono/picture feature는 활성화하지 않았습니다. native app에만 정확한 버전으로 추가했고 root MSRV·제품 manifest/lock·TS는 이번 변경에서 유지했습니다.

Calamine이 요구하는 quick-xml 0.41.0을 새 package 없이 direct edge로도 재사용했습니다. XML allocation metadata 사전 검사에 필요하며 MIT·MSRV 1.79입니다. Calamine의 zip 8.6.0(MIT·MSRV 1.88)이 native lock에 추가됐고 기존 zip 2.4.2도 유지합니다. dependency fetch는 sandbox DNS 실패 뒤 정상 require_escalated로 공개 crates.io 패키지만 취득했습니다. .env/인증 파일을 읽지 않았습니다. 정확한 native lock delta는 11개 신규 package이며 프로젝트 전체 환경을 새로 설치한 것이 아닙니다.

zip 8.6.0의 feature가 공용 flate2에 zlib-rs backend를 활성화한 것은 cargo tree와 flate2의 실제 cfg 선택으로 확인했습니다. 앞선 PNG/PPTX 성공은 이 backend가 바뀐 뒤의 성공으로 간주할 수 없어, 해당 두 소비자만 아래 한 번 검사했습니다. PDF/ImageIO/UI·parser 전체의 무관한 성공을 반복하지 않았습니다.

## 실제 결과

- 첫 XLSX test compile은 fixture의 fmt::Write import와 Copy가 아닌 Cell의 반복 array 선언 누락으로 실패했습니다. 두 fixture를 수정한 뒤 같은 관련 검사만 다시 실행했습니다. 제품 parser runtime 실패를 통과로 바꾸거나 검사기를 끄지 않았습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-spreadsheet -- --nocapture` — 1건 PASS, compile 0.74초·suite 0.02초. sheet 순서·CJK/shared/inline text·cached formula·number/boolean/error/null/date serial·offset/blank/empty/no sheets·503행의 500행 제한/total·out-of-order origin 및 malformed/truncation/CRC/declared oversize·uniqueCount bomb·DTD·큰 grid/Excel 범위 거절입니다. 과도한 uniqueCount가 실제 reservation에 도달하기 전 거절되는 입력을 포함합니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app --test preview-spreadsheet -- -D warnings` — exit 0, 2.77초.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-presentation --test preview -- --nocapture` — 변경된 압축 backend의 actual PNG 1건 PASS(0.02초), PPTX parser 1건 PASS(0.03초), compile 1.19초. 같은 backend 상태로 반복하지 않았습니다.
- [x] SheetJS 원본 함수를 동일한 B3:E505 cell/range 값으로 호출해 totalRowCount 503·shown 500, first text/1.25/true/null·second inline/46297/null/null·blank·lastShown 499/null/null/null을 확인했습니다. native fixture 기대값과 일치합니다. 이는 원본의 전체 workbook corpus나 serialized XLS 비교가 아닙니다.
- [x] 대상 rustfmt --check·git diff --check — exit 0. 신규 package 없이 기존 formatter만 사용했습니다.

## 다음 구현과 남은 gate

- [ ] CSV의 원본 delimiter guess/sep=·quote/newline/blank row·폭 padding·boolean/numeric/formula/date coercion·BOM/UTF-16/non-BOM binary encoding과 JavaScript 숫자 표시를 연결합니다. 원본 실제 probe에서 UTF-8 non-BOM CJK는 binary 문자열, ISO date는 timezone에 따라 serial 소수부가 나옵니다. 임의로 원본의 인코딩/날짜 동작을 개선하지 않습니다. 새 csv crate는 추가하지 않았습니다.
- [ ] XLS는 Calamine constructor에서 sheet와 formula sparse cells를 dense Range로 펼칩니다. native 앱 프로세스에서 이 비신뢰 할당을 그대로 허용하지 않고 사전 BIFF/CFB bounds 또는 격리 reader 경계를 구현한 뒤 연결합니다. 현재 XLS를 지원한다고 주장하지 않습니다.
- [ ] XLSX ISO 날짜/비정상 숫자·A1:A1 명시 dimension과 no dimension의 구분·차트/hidden/다른 namespace·누락 sheet/관계/중복 ZIP·XML encoding·전체 실파일 corpus와 원본의 더 관대한 CRC/DTD/큰 파일 정책 차이를 검증합니다. 미연결 cell 타입은 명시적 오류입니다.
- [ ] 전체 RSS/cancel/crash isolation·ZIP metadata 사전 할당·shared strings/styles/관계 수·비신뢰 aggregate 및 active/closed tab 토큰을 완료합니다. 아직 worker/cache에 연결하지 않았습니다.
- [ ] 실제 Spreadsheet의 tablist/tab·선택/clamp·empty/error/truncated·cell format/표/스크롤·bytes 교체 상태·3 locale/theme·OS/GPU/AX·다른 provider/terminal·N1부터 N8/M8·Rust99%/TS 제거/배포를 완료합니다.
