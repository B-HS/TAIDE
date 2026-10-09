# Rust-native 전체 완료 근거 점검

기준: 2026-10-09, HEAD `d1d98d4d`, 브랜치 `to_rust_native`. 서브에이전트·workflow 없이 메인이 현재 코드와 기존 실행 로그를 읽었습니다. 이번 점검에서는 코드 수정·의존성 반입·빌드·테스트·화면 조작을 실행하지 않았습니다.

## 완료율을 판정할 기준

사용자 목표는 기존 TS 화면·상태·상호작용을 Rust-native UI로 정확히 구현하고 전체 배치와 완료 게이트를 끝내는 것입니다. 원본 버그의 강제 재현 조건은 `../acknowledge/2026-10-09-native-find-regex-decisions.md`의 최신 결정으로 제외합니다. 실제 데이터·OS 설정·클립보드·Keychain·Trash 보호와 remote-web 동결은 유지합니다.

`2026-10-06-native-audit-summary.md`의 600행 목록은 재감사 대상으로 사용할 수 있으나 당시 판정을 현재 완료율로 사용할 수 없습니다. 다음 근거를 해당 요구사항 범위에 연결한 뒤 최신 분모·분자를 집계해야 합니다.

| 요구사항 | 완료를 입증할 근거 | 현재 판정 |
| --- | --- | --- |
| 전체 기능 대응 | TS 기능별 native 구현·앱 도달 경로·동작 검증을 연결한 최신 전수 대응표 | 배치 1~7 이후 전수 재판정 없음. 현재 전체 기능 완료율 미산정 |
| 화면 대응 | 테마·로케일·표시 상태별 실제 화면과 기존 TS 구성 대조 | 배치 4~7 QA의 실제 화면 확인 항목 미완료 |
| 자동 검사 | 변경 크레이트 전체 대상 결과와 미해결 실패·보호 검사의 해소 근거 | batch7 통합 QA에서 app XLML 1건 실패·Trash 보호 검사 3건 미완료. 이후 batch8에서 XLML 실패를 수정해 관련 XLML 4건·HTML 3건 통과, app 전체 게이트는 후속 실행 대기 |
| 실기·성능·출시 | roadmap Phase 5~9의 IME·접근성·다중 창·대형 파일·soak·보안·패키징·beta·삭제 게이트 | 전체 통과 증거 없음. ignored 성능 검사와 실기 부채를 통과로 처리하지 않음 |

테스트 개수나 배치 체크리스트 완료 비율을 전체 기능 대응률로 환산하지 않습니다. 잔여시간은 남은 기능과 게이트별 규모·실제 실행 시간 근거를 확보한 뒤 산정합니다.

## 현재 코드에서 확인한 미완료 경로

| 대상 | 실제 근거 | 판정 |
| --- | --- | --- |
| 찾기 명령 | `native/taide-native-ui/src/command-registry.rs:223`의 `editor.find`는 `Enablement::Never`. app의 `command-dispatch.rs:721` 검사는 `editor.action.startFindReplaceAction`을 실행 가능 집합에서 제외 | 기존 리터럴 일치 선택 기능만으로 찾기/바꾸기 위젯 완료를 입증할 수 없음 |
| 표시 설정 공급 | `native/taide-native-ui/src/presentation.rs:236`은 `word_wrap`만 설정하고 나머지는 기본값. `native/taide-native-app/src/presentation-refresh.rs:174`는 bold 글꼴을 추가, `application.rs:5646`은 folding을 추가 | 설정 화면의 필드 존재를 편집기 표시 기능 완료로 판정하지 않음 |
| 표시 설정 소비 | `native/taide-native-ui/src/editor_surface.rs:106`의 14필드 중 표면에서 소비하는 것은 word wrap·folding·bold family. 전체 UI 소스에서 아래 11필드의 렌더 소비 경로 없음 | 3/14는 이 구조체의 소비 경로 수이며 기능 완료율이 아님 |
| LSP 사용자 상호작용 | `native/taide-native-app/src/lsp.rs`에서 직접 확인한 typed request는 저장 시 Formatting·ExecuteCommand·CodeActionRequest·CodeActionResolveRequest 경로 | 저장 기능 검증으로 완성·hover·signature·이동·peek 등 전체 LSP UI 완료를 입증할 수 없음 |

아직 소비하지 않는 표시 필드는 `render_whitespace`, `rulers`, `cursor_style`, `cursor_blinking`, `smooth_caret`, `scroll_beyond_last_line`, `smooth_scrolling`, `sticky_scroll`, `minimap`, `bracket_pair_colorization`, `bracket_pair_guides`입니다. 리거처 설정은 이 구조체와 별도로 후속 표시 구현에서 확인해야 합니다.

배치 7의 문서 명령 34개·커서 명령 23개 연결은 `command-registry.rs`의 `line_command`·`cursor_command`, app `command-dispatch.rs`의 `apply_document_edits`와 해당 통합 QA에서 확인했습니다. 명령 카탈로그 전체나 LSP provider까지 완성됐다는 뜻으로 확대하지 않습니다.

## XLML 실패 원인의 코드 대조

기존 실패 로그 `/private/tmp/taide-batch7-app-final-20261009.log:1089`는 `tests/preview-spreadsheet-xlml.rs:94`에서 `<Row ss:Index="0"><Cell/></Row>`를 수락했음을 기록합니다. 해당 parser·test는 batch7 이후 변경되지 않았으므로 이 실행 증거를 재사용합니다.

현재 호출 경로는 다음과 같습니다.

1. `preview_spreadsheet.rs:68`은 첫 비공백 바이트가 `<`인 입력을 XLML `decode`로 전달합니다.
2. `preview_spreadsheet_xlml.rs:468`은 XML 파싱 전에 HTML의 `is_html`을 호출합니다.
3. `preview_spreadsheet_html.rs:99`는 앞 1,024문자의 따옴표 문자열을 지우고 소문자로 만든 뒤 `<?xml`이 없고 `<table`이 있으면 HTML로 분류합니다.
4. 실패 테스트의 `workbook` 헬퍼는 XML 선언 없이 `<Workbook ...><Worksheet ...><Table>...`를 생성합니다. 이 입력은 HTML로 분류돼 XLML의 `scan`·`attributes`를 거치지 않습니다.
5. `preview_spreadsheet_xlml.rs:132`의 인덱스 파서는 이미 `checked_sub(1)`로 0을 거절합니다. `scan`의 Start와 Empty 모두 `State::start`에서 속성을 읽으므로 빈 Cell만의 속성 누락으로 추정해 수정해서는 안 됩니다.

이 원인 대조 이후 batch8에서 실제 Workbook 루트를 확인하는 분류로 수정했습니다. 수정 전 두 실패를 재현하고 수정 후 XLML 4건·HTML 3건 모두 통과했습니다. 상세는 `../bug/2026-10-09-native-xlml-workbook-classification.md`입니다. app 전체 게이트는 통합 단계에 남아 있습니다.

## 점검 상태

- [x] a. 전체 완료 게이트와 현재 증거의 범위 대조
- [x] b. 찾기·표시 설정·LSP 경로와 배치 7 명령 연결 대조
- [x] c. 기존 XLML 실패 로그와 실제 parser 분류 경로 대조
- [ ] d. 최신 전수 기능 대응표·실기·성능·출시 증거 확보 — 전체 전환 작업에서 계속 확인

이 근거 점검은 3/4 완료(75%)이며 전체 전환율이 아닙니다. 이후 사용자가 기존 배치별 중단 지시를 해제해 전체 전환을 재개했으며 `../acknowledge/2026-10-09-native-full-resume.md`를 따릅니다. 최신 전체 대응표·실기·성능·출시 근거는 전환 작업에서 계속 확보합니다.
