# M8 native Spreadsheet worker·cache·surface 기본 연결

## 대상과 완료 경계

대상은 `native/taide-native-app/src/{preview_spreadsheet,preview_spreadsheet_cache,preview_spreadsheet_surface,host,application,lib}.rs`와 `tests/preview-spreadsheet-host.rs`입니다. 원본 `src/shared/lib/spreadsheet.ts`·`src/features/preview/{spreadsheet-preview,preview-status}.tsx`, 기존 approved read/PPTX/cache/host, 고정 egui/AccessKit source와 실제 Lucide TableProperties 노드를 확인했습니다. 현재 완료는 기존 XLSX·CSV parser를 실제 Spreadsheet File 탭에 연결한 기본 경계입니다. XLS 및 모든 spreadsheet 형식/픽셀/제품 gate를 완료한 것은 아닙니다.

1. typed Request(path/token)·HostCommand·SourceReady·결과를 기존 bounded channel과 tracked blocking worker로 연결했습니다. 기존 read_approved의 열린 project/절대 경로/canonical source/identity/종료/사후 root 재검사를 재사용합니다. raw read 실패와 decode 실패를 나누고 decode 결과를 nested Result로 반환해 parse 실패도 사후 권한 검사를 건너뛰지 않습니다. external action은 기존 승인된 OpenPath 요청이며 검사에서 실제 OS 프로그램은 열지 않습니다.
2. 파일별 Arc<Workbook>과 tab별 선택을 분리했습니다. 같은 파일의 두 탭은 같은 내용만 공유하며 선택은 독립입니다. bytes 교체 후 원본처럼 저장된 선택은 유지하고 표시만 clamp합니다. sheet 수가 다시 늘면 이전 index가 복구됩니다. invalidation·stale progress/result·cancel/reset·path 변경·close·project 제거·shutdown 및 submit 실패 회수를 기존 Application 경로에 연결했습니다.
3. 데이터가 준비되기 전은 원본 outer read와 같은 빈 배경이고 Spreadsheet에 별도 loading 문구를 추가하지 않았습니다. ready는 sheet 탭·500행 제한 안내·양축 scroll·빈 sheet/없음 상태, 실패는 공통 FileWarning/외부 열기 상태입니다. tabBar/editor/explorer/app/status의 실제 theme key를 사용하고 en/ko/ja의 기존 message catalog를 재사용합니다. 셀은 null 빈 문자열, boolean 소문자, JS number 표시와 literal text입니다. nowrap의 ASCII whitespace collapse와 NBSP 보존을 구분하며 markup을 실행하지 않습니다.
4. 표는 전체 보이는 500행의 열 폭을 한 번 계산하고 row/column viewport 밖 text paint를 생략합니다. 열 폭을 UI의 영구 temp map에 남기는 초기 경로를 발견해 파일 cache로 옮겼습니다. sheet별 열 폭의 최대 저장 비용을 결과 admission에서 미리 예약하고 64MiB per-workbook 및 image/PDF/PPTX/Spreadsheet 합산 128MiB에 포함합니다. bytes 교체·실패·마지막 탭 close 때 내용과 열 폭을 같이 회수합니다. 모든 provider의 Application admission 계산에 Spreadsheet 비용도 넣었습니다.
5. AccessKit source에서 SelectableLabel은 기본 Button임을 확인해 실제 노드를 Tab/TabList 및 selected로 명시했습니다. 기본 toggled 값은 제거했습니다. Table의 행/열 수와 보이는 Cell의 index도 연결했습니다. 실제 VoiceOver·보조 창·virtualized offscreen cell 탐색 및 전체 AX gate를 통과했다는 뜻은 아닙니다.

## 실제 검증 결과

- [x] 최초 lib/bin `cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app` — exit 0, 1.77초.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-spreadsheet-host -- --nocapture` — 기본 host/UI 2건 PASS, compile 6.03초·suite 0.06초. actual XLSX host progress/공유/선택 유지/재해석/stale/cancel/root component/rename/외부·상대 경로 거절/깨진 archive/project close/task 회수, actual egui의 en·ko·ja 키보드 선택·12px 셀/JS 표기/whitespace·empty/noSheets·shrink/grow clamp 복구·500/503 안내/viewport paint·read/decode 외부 의도 및 metadata/aggregate budget 거절을 확인했습니다.
- [x] 위 기본 상태의 lib/bin/해당 test strict clippy — exit 0, 1.90초. 같은 기본 runtime 검사는 반복하지 않았습니다.
- source 판별은 원본 SheetJS의 readSync를 추가 확인했습니다. 단순 PK 접두어를 ZIP으로 잘못 분류하던 범위를 원본의 뒤 두 byte 조건으로 좁혔고 malformed ZIP fixture도 PK 03 04로 정정했습니다. OLE/legacy BIFF는 안전한 XLS reader가 없으면 명시적으로 거절합니다. 원본의 XML/HTML/SYLK/DIF/DBF/Lotus/RTF 등 추가 내용 판별까지 완료한 것은 아닙니다.
- [x] 같은 cargo test의 filter `csv_host` — 최종 CSV/열 폭 cache/AX 연결 1건 PASS, compile 4.48초·suite 0.02초. actual approved host에서 PKlabel CSV·number/boolean/literal tag·SourceReady를 읽고, actual AccessKit update의 TabList/Tab selected/Table 2×2/Cell 4개, 열 폭 예약 비용 불변·마지막 tab close 후 retained 0·OLE/BIFF/깨진 ZIP 명시적 거절과 tracked_count 0을 확인했습니다. CSV 기본 27 reference·XLSX parser와 변경 없는 host/선택/locale 성공은 재사용했습니다.
- 열 폭 회수 변경 뒤 관련 lib/bin/test strict clippy — exit 0, 1.66초. 설치된 Lucide TableProperties의 실제 path에 맞춰 추가 vertical line도 제거했습니다. 최종 정적 결과는 아래 후속 기록을 따릅니다.
- [x] Lucide path 정정 후 동일 범위 strict clippy — exit 0, 1.27초. 변경 없는 runtime 성공은 재사용했습니다. 대상 rustfmt --check와 git diff --check도 exit 0이며 사용자 실기 app 빌드/실행은 하지 않았습니다.
- 검사는 합성 임시 project만 만들고 Drop에서 해당 경로만 제거했습니다. 사용자 실기 bundle·OS 앱/clipboard·시스템 입력기/VoiceOver·제품 TS·root MSRV는 변경하지 않았습니다. 새 package 없이 기존 dependency만 사용했습니다.

## 다음 경계와 잔여 gate

- [x] XLS CFB·BIFF8 기본 reader는 후속 `2026-10-02-m8-native-xls-reader.md`에서 approved worker에 연결했습니다. 직접 Calamine XLS constructor 대신 bounded CFB preflight와 streaming BIFF8 투영을 사용합니다. 위 OLE/BIFF 거절 기록은 이 연결 전의 역사이며 현재는 잘못된 CFB/미연결 BIFF 버전만 거절합니다. 전체 XLS/BIFF2~5/standalone/corpus/memory gate는 여전히 미완료입니다.
- [ ] 원본의 모든 내용 판별·XLS/XLSX/CSV 실파일 corpus·추가 JS Date/DST/encoding·XLSX 관계와 ISO cell·A1:A1·형식별 blank/error/범위 차이, sheet 탭 key와 path/mount/다중 project 전체 수명을 완료합니다.
- [ ] CSS auto table의 정확한 열 너비 분배/행 높이·border collapse·scrollbar·truncated wrapping·empty status geometry·모든 theme/locale/font/pixels와 실제 Application/OS/GPU/보조 창/VoiceOver를 확인합니다. 현재 열의 여유 폭은 균등 분배이며 browser의 모든 auto-layout 규칙과 동일하다고 주장하지 않습니다. 보이는 행만 그리는 AX의 offscreen row 접근도 남습니다.
- [ ] decode 중 invalidation/cancel의 즉시 중단·read worker의 decode 완료까지 소유 guard 유지·전체 peak RSS/egui font cache/metadata/열 폭 계산 성능·crash isolation과 전체 128MiB 검증을 마무리합니다. retained 예약은 실제 전체 RSS 상한이 아닙니다.
- [ ] 다른 provider·실제 terminal surface·전체 213 view·N1부터 N8·Rust99%/TS 제거/배포 gate는 미완료이며 전체 M8 완료 뒤만 commit/push합니다.
