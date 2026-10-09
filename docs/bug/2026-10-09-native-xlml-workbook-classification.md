# XML 선언 없는 Workbook의 HTML 오분류 수정

## 증상과 원인

`native/taide-native-app/src/preview_spreadsheet_html.rs`의 `is_html`은 앞 1,024문자에 `<?xml`이 없고 `<table`이 있으면 HTML로 분류했습니다. XML 선언은 생략할 수 있으므로 `<Workbook><Worksheet><Table>...`도 HTML로 잘못 분류됐습니다. 시트 이름·셀 내용이 사라지고 XLML의 인덱스·할당·XML 보안 검증을 우회했습니다.

기존 `tests/preview-spreadsheet-xlml.rs`의 `xlml은_할당전_grid_합산_인덱스와_xml_보안_경계를_거절한다`가 `<Row ss:Index="0"><Cell/></Row>`를 수락해 실패한 원인입니다. 인덱스 파서에는 이미 `checked_sub(1)` 검증이 있었으므로 그 파서는 변경하지 않았습니다.

## 수정과 회귀 범위

기존 quick-xml 0.41.0의 메모리 reader로 첫 Start/Empty 요소를 읽습니다. 로컬 이름이 Workbook이면 XLML 경로를 유지하고, 그 외는 기존 HTML 탐지 규칙을 적용합니다. 주석·선언·공백을 건너뛰며 XML 네임스페이스 접두사는 로컬 이름 비교로 처리합니다. 문서나 외부 엔티티를 실행하지 않습니다.

`tests/preview-spreadsheet-xlml.rs`에는 XML 선언 없는 기본 Workbook·선행 주석·네임스페이스 접두사를 UTF-8·UTF-8 BOM·UTF-16 BOM으로 읽는 9개 조합을 추가했습니다. `tests/preview-spreadsheet-html.rs`에는 주석·셀 안의 Workbook 문구와 HTML DOCTYPE이 HTML 분류를 유지하는 3개 조합을 추가했습니다. 기존 정상 SheetJS 기준값과 XLML·HTML의 크기·할당·인덱스·XML 보안 검사를 함께 실행했습니다.

## 실제 검증

```sh
cargo test --manifest-path native/taide-native-app/Cargo.toml --test preview-spreadsheet-xlml --test preview-spreadsheet-html --no-fail-fast --locked --offline --target-dir experiments/native-shell-spike/target
cargo fmt --manifest-path native/taide-native-app/Cargo.toml --check
```

- 수정 전: HTML 3건 통과, XLML 2건 통과·2건 실패, exit 101. 새 정상 Workbook 회귀와 기존 인덱스/보안 검사가 실패했습니다. 로그는 `/private/tmp/taide-batch8-xlml-before-20261009.log`입니다.
- 수정 후: HTML 3건·XLML 4건 모두 통과, exit 0. Cargo 표시 시간 6.60초, 대상 실행 시간 0.23·0.07초입니다. 로그는 `/private/tmp/taide-batch8-xlml-after-20261009.log`입니다.
- app 패키지의 포맷 검사 exit 0, 해당 diff 공백 검사 exit 0입니다. 기존 vendored wry 경고 17건과 linker unwind 경고는 수정하지 않았습니다.

이 결과는 두 미리보기 관련 대상의 검사 결과입니다. app 전체 대상 검증은 배치 8 통합 단계에서 실행합니다. 새 의존성·Cargo/lockfile·remote-web·실제 앱 데이터·OS 설정·클립보드·Keychain·Trash 변경은 없습니다.
