# M8 native BIFF2~5 기본 셀·codepage 투영

## 대상과 완료 경계

대상은 `native/taide-native-app/src/preview_spreadsheet_biff.rs`, 격리 app Cargo manifest/lock과 `tests/preview-spreadsheet-biff.rs`, `tests/fixtures/spreadsheet-biff-{legacy,codepage}-reference.json`입니다. 기존 N5-P1f의 XLS reader 후속이며 상위 Spreadsheet/N5/M8 완료가 아닙니다. 선행 CFB/BIFF8 worker 연결 근거는 `2026-10-02-m8-native-xls-reader.md`입니다.

1. BOF의 버전/식별자를 구분하고 BIFF2~4 standalone Sheet1과 BIFF5 workbook globals/BoundSheet를 기존 2-pass 투영에 연결했습니다. BIFF5의 sheet BOF가 BIFF8로 표기되는 호환 경계에서 global version을 기준으로 읽습니다. legacy Dimensions의 행 필드·16,384행/256열 한도·cell attributes·integer/number/label/boolean·formula alias와 legacy String 길이를 별도로 처리합니다. 원본 writer는 BIFF3/4 BOF에서도 BIFF2 셀 레코드를 썼으므로 해당 조합도 지원합니다.
2. 표준 라이브러리에는 Windows/CJK codepage 변환이 없어 기존 Calamine transitive인 codepage 0.1.3과 encoding_rs 0.8.42의 direct edge만 격리 app에 추가했습니다. 새 lock package는 없으며 기존 버전/feature를 재사용합니다. codepage는 Apache-2.0 OR MIT/MSRV 1.36, encoding_rs는 (Apache-2.0 OR MIT) AND BSD-3-Clause/MSRV 1.88입니다. root/MSRV는 유지합니다. 설치된 동일 source의 `to_encoding_no_replacement`와 `decode_without_bom_handling` 계약을 읽었고 문자열마다 byte 길이를 먼저 제한합니다. 알 수 없거나 replacement-only codepage는 명시적으로 거절합니다.
3. CODEPAGE의 원본 alias 0x5212/0x8000/0x8001을 반영하고 기본 Windows-1252를 사용합니다. unicode BIFF8 문자열 경로는 유지합니다. cached formula의 empty-string marker를 null이 아닌 빈 Text로 고쳤습니다. 암호화된 standalone도 명시적으로 거절하고 legacy row 범위를 먼저 검사합니다.

## 실제 검증

Cargo 공통 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. manifest direct edge 최초 갱신만 기존 lock 목록 재사용을 위해 `--locked` 없이 offline check했습니다. 성공 결과는 같은 상태에서 반복하지 않습니다.

- [x] lib/bin offline check — exit 0, 1.89초. 새 package 다운로드/추가 없이 native app lock의 dependency edge만 갱신했습니다.
- [x] `cargo test … --test preview-spreadsheet-biff legacy_biff -- --nocapture` 최초 FAIL: BIFF3 BOF에서 BIFF2 cell ID를 무시해 0행이 됐습니다. 원본 serializer/reader의 실제 조합을 확인하고 record ID에 따른 셀 헤더/문자열 폭을 구현했습니다. 실패한 검사만 1회 재실행해 최종 1건 PASS, compile 2.01초·suite 0.00초입니다. SheetJS 0.18.5의 BIFF2/3/4 raw 및 BIFF5 CFB 직렬화 네 결과, 원본 read/workbookToSheets의 typed 셀·sheet 이름/순서·offset/empty/blank·503 total/500행 제한을 대조했습니다.
- [x] filter `legacy_biff는_표준` — 표준 BIFF3/4 record를 합성한 뒤 원본 SheetJS read 결과를 고정한 12 reference의 Windows-1252/Shift-JIS/GBK/EUC-KR/Big5/1252 alias·C3:E5 range/빈행·number/boolean을 대조했습니다. 빈 formula Text·알 수 없는 codepage·standalone encryption·legacy row limit 거절도 같은 검사에 포함했습니다. 1건 PASS, compile 1.15초·suite 0.00초입니다. 한글 `가`, 일본어 `日`, 중국어 `中`과 Latin `Été`/`€`의 결과가 일치합니다.
- [x] 변경된 공통 BIFF 파서의 영향 filter `biff8은_실제` — 기존 실제 inline/SST XLS reference 1건 PASS, build 0.22초·suite 0.00초입니다. 입력·환경이 같은 무변경 반복이 아니라 버전/문자열 경로 변경에 대한 영향 검사입니다. 다른 CFB/host/surface의 변경 없는 성공은 재사용했습니다.
- [x] `cargo clippy … --lib --bin taide-native-app --test preview-spreadsheet-biff --test preview-spreadsheet-host -- -D warnings` — exit 0, 2.08초입니다.

## 근거와 남은 gate

[OpenOffice의 Excel format 문서](https://www.openoffice.org/sc/excelfileformat.pdf)의 BOF·LABEL·STRING·Dimensions와 셀 레코드 구분, 설치 SheetJS reader/writer의 실제 데이터, 고정 codepage/encoding_rs source를 확인했습니다. [encoding_rs API](https://docs.rs/encoding_rs/latest/encoding_rs/struct.Encoding.html)와 최신 manifest의 MSRV/license도 확인했습니다. 일부 고정 docs.rs/GitHub URL은 조회 오류가 있어 동일 설치 package source를 근거로 사용했습니다.

- [ ] BIFF2~5 전체 formula cache/continuation/RString·모든 codepage/OEM·malformed encoding·비표준 BOF/worksheet/workspace·chart/macro/VBA·중복 이름과 실제 corpus는 남습니다. 기본 버전 연결을 전체 XLS 지원으로 계산하지 않습니다.
- [ ] standalone BIFF8 및 원본의 XML/HTML/SYLK/DIF/DBF/Lotus/RTF/ZIP 추가 형식 판별은 남습니다. 원본 serializer가 지원하지 않는 문자는 저장 당시 `_`로 치환됐으며 fixture의 치환 결과를 원본대로 비교했습니다. 비Latin 저장 fidelity를 증명한 것은 아닙니다.
- [ ] 전체 SST/metadata/worker/cache/renderer RSS·즉시 cancel/worker guard·crash isolation, 정확한 화면/scroll/OS/GPU/AX 및 다른 provider·terminal·213 view·N1~N8·Rust99%/제품 TS 제거·배포는 미완료입니다. 사용자 실기 bundle과 OS 설정은 조작하지 않았으며 전체 M8 완료 뒤만 commit/push합니다.
