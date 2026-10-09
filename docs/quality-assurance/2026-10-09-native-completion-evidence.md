# Rust-native 전체 완료 근거 점검

기준: 2026-10-10, 브랜치 `to_rust_native`. 최초 읽기 전용 점검은 HEAD `d1d98d4d`에서 수행했고 이후 batch8~19의 실제 구현·검증과 batch14 전수 대응표를 native Source `8832ac61` 기준으로 갱신했습니다. 서브에이전트·workflow 없이 메인이 직접 수행합니다. 실제 화면/OS 입력의 전체 게이트는 미검증입니다.

## 완료율을 판정할 기준

사용자 목표는 기존 TS 화면·상태·상호작용을 Rust-native UI로 정확히 구현하고 전체 배치와 완료 게이트를 끝내는 것입니다. 원본 버그의 강제 재현 조건은 `../acknowledge/2026-10-09-native-find-regex-decisions.md`의 최신 결정으로 제외합니다. 실제 데이터·OS 설정·클립보드·Keychain·Trash 보호와 remote-web 동결은 유지합니다.

`2026-10-06-native-audit-summary.md`의 600행 목록은 재감사 대상으로 사용할 수 있으나 당시 판정을 현재 완료율로 사용할 수 없습니다. 다음 근거를 해당 요구사항 범위에 연결한 뒤 최신 분모·분자를 집계해야 합니다.

| 요구사항       | 완료를 입증할 근거                                                                          | 현재 판정                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| -------------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 전체 기능 대응 | TS 요구사항별 native 구현·앱 도달 경로·해당 동작 묶음의 자동 검증을 연결한 최신 전수 대응표 | batch14의 실제 599행 재판정 뒤 batch15~19의 검증된 요구사항을 갱신했습니다. 웹 기술/내부 지표 10·동결 1을 제외한 588행 중 완료 282행(48.0%), 부분 92·미연결 113·미구현 101입니다. batch19의 LSP 구문 공급·수동/Import·원본 카탈로그 19종 접기 1행을 완료로 반영했습니다. 복합 요구사항 행의 완료율이며 원자 기능 수/전체 출시 전환율은 아닙니다. 스냅샷/표·근거 36묶음·232경로 참조                                                                                                                                                                                                     |
| 화면 대응      | 테마·로케일·표시 상태별 실제 화면과 기존 TS 구성 대조                                       | 배치 4~7 QA의 실제 화면 확인 항목 미완료                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 자동 검사      | 변경 크레이트 전체 대상 결과와 미해결 실패·보호 검사의 해소 근거                            | batch19 변경 editor/UI/app/LSP SDK의 전체 117대상을 직접 1회 실행했습니다. 최초 실패 3건은 영향 대상 재검사로 해소했고 서로 다른 최신 성공은 editor 206·UI 362·app 663·LSP SDK 81, 총 1312건입니다. 변경 없는 syntax batch13 159·egui SDK batch12 단위 52/문서 167은 별도 근거입니다. 보호 Trash 3·기존 ignored 5·실기 부채는 미검증입니다. frozen host/Wasm·변경 native/SDK fmt/diff exit 0, 디스크 668GiB·64%입니다. prototype standalone은 lock 불일치로 실행이 거절됐고 edition별 fmt 차이가 남아 있습니다. batch19 QA에 실제 실패/수정·잔여 근거와 Cargo 직렬 실행을 기록했습니다. |
| 실기·성능·출시 | roadmap Phase 5~9의 IME·접근성·다중 창·대형 파일·soak·보안·패키징·beta·삭제 게이트          | 전체 통과 증거 없음. ignored 성능 검사와 실기 부채를 통과로 처리하지 않음                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |

테스트 개수나 배치 체크리스트 완료 비율을 전체 기능 대응률로 환산하지 않습니다. 현재 기능 대응표 기준은 282/588(48.0%)이고 전체 전환율/잔여시간은 미산정입니다. 잔여시간은 남은 306개 복합 요구사항과 별도 구조/실기/출시 게이트별 규모·실제 수행 시간 근거를 확보한 뒤 산정합니다.

## 현재 코드의 기능 연결과 잔여 경로

| 대상                | 실제 근거                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 판정                                                                                                                           |
| ------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| 찾기 명령           | batch8에서 `native/taide-native-ui/src/editor-find.rs`·`editor-find-widget.rs`, registry의 native-host Find 실행·지원 gate, app의 뷰별 상태/큐/키/강조 연결을 구현. 순수 코어 13건과 최종 UI 전체 271건 통과, 앱 전체·실패 선별 재검사와 동결 컴파일 확인                                                                                                                                                                                                                                                             | 구현·자동 검증 연결됨. 실제 화면/OS 입력은 미검증. `2026-10-09-native-batch8-find.md` 참조                                     |
| 표시 설정 공급      | batch9~12 `presentation::editor_presentation`이 native-host의 공백·rulers·캐럿·스크롤 7필드와 괄호 색상/안내선 2필드·고정 줄·미니맵을 실제 설정으로 공급합니다. app의 presentation-refresh·application은 테마 공백/ruler/스크롤바·괄호/들여쓰기·고정 줄·미니맵 색과 기존 bold/folding을 연결하고 갱신합니다. 고정 줄 명령·메뉴 토글은 원본의 세션 메모리 변경이고 실제 설정 변경에 동기화됩니다. 미니맵 토글은 원본 SettingsPatch 저장 경로와 queued fresh 값으로 연결합니다. browser 공급은 기존 word-wrap 경로 유지 | 설정→표시→앱 경로 자동 검증 연결. 실제 화면은 미검증                                                                           |
| 표시 설정 소비      | 기존 14옵션 중 word wrap·folding·bold family와 신규 공백·rulers·캐럿 스타일/깜빡임/이동·아래 여백/부드러운 스크롤·괄호 색상/안내선·고정 줄·미니맵을 소비합니다. `editor-display.rs`·`editor-caret.rs`·`editor-scroll.rs`·`editor-brackets.rs`·`editor-sticky-scroll.rs`·`editor-minimap.rs`·`editor-minimap-layout.rs`와 surface 연결 및 별도 optional 색 공급입니다                                                                                                                                                  | 기존 14옵션 중 14개의 소비 근거입니다. 전체 표시 기능 완료를 뜻하지 않습니다. 기능 완료율이 아닙니다. batch9~12 표시 QA 참조   |
| 기본 괄호 일치 강조 | batch13 BracketModel near/enclosing·Other 토큰·현재 문서/뷰와 실제 본문 글자/테마/clip·선택/focus를 연결했습니다. 원본 23언어·483문서·5185위치와 메모리 화면 7건·전체 대상 통과. 찾기창/고정 줄 focus에서 본문 강조를 유지하고 고정 줄 테두리/배경은 복제하지 않습니다                                                                                                                                                                                                                                                | 본문과 현재 관련 위젯의 자동 검증 연결. overview near-only 장식·실제 OS UI/대형 성능은 후속 범위                               |
| LSP 사용자 상호작용 | `native/taide-native-app/src/lsp.rs`의 저장 Formatting/ExecuteCommand/CodeAction/Resolve, batch16 DocumentSymbolRequest·@/고정 줄·batch17 outline/breadcrumb, batch18 typed WorkspaceSymbolRequest·# 팔레트·파일 이동, batch19 typed FoldingRangeRequest·수동/Import·19종 명령/기본 키·본문/gutter/고정 줄·앱 reveal                                                                                                                                                                                                  | 해당 공급/소비자의 자동 검증을 연결했습니다. 완성·hover·signature·정의 이동·참조/peek 등 전체 LSP UI 완료로 확대하지 않습니다. |

미니맵은 batch12에서 원본 prebaked 문자·표시 줄 layout·선택·테마·click/drag/touch/wheel·설정 저장을 연결했습니다. 진단/검색 미니맵 장식과 빈 줄 배경은 batch15에서 공급했으며 SCM 장식은 후속 공급 범위입니다. 기본 들여쓰기 안내선은 batch10에서 실제 표시 줄·wrap/접기·clip·스크롤에 연결했습니다. 고정 줄은 batch11에서 버전/언어를 확인하는 모델과 접기 fallback·표시/입력·세션 토글을 연결했습니다. LSP 문서 심볼의 selectionRange 헤더·최대 범위/선호 provider 공급은 batch16에서 연결했습니다. 구문 접기 provider는 batch19에서 본문과 고정 줄에 연결했고 Ctrl/Meta 정의 이동은 후속 LSP 범위입니다. 기본 괄호 일치 본문 강조는 batch13, near-only overview·진단 밑줄/메시지·F8 계열 문제 이동·파일 간 preview/reveal/view zone은 batch15에서 연결했습니다. 리거처·일반 LSP hover/완성·SCM·주입/블록과 나머지 표시 설정은 후속 구현에서 확인해야 합니다. batch13의 전체 1300건 성공 근거·앱 컴파일 오류 수정 뒤 전체 실행·보호 3 제외·ignored 성능 4건과 재사용 SDK 문서 1 ignored를 개별 QA에 기록합니다. 브라우저 host/Wasm·포맷·동결 경계는 각 batch QA에 기록합니다.

배치 7의 문서 명령 34개·커서 명령 23개 연결은 `command-registry.rs`의 `line_command`·`cursor_command`, app `command-dispatch.rs`의 `apply_document_edits`와 해당 통합 QA에서 확인했습니다. 명령 카탈로그 전체나 LSP provider까지 완성됐다는 뜻으로 확대하지 않습니다.

batch17은 파일/아웃라인 진입, 20px 가상 심볼 트리·종류/이름/detail·키/마우스·빈 상태와 32px 상대 경로/현재 caret 체인·형제 메뉴·preview 1:1·현재 탭 reveal을 실제 앱에 연결했습니다. 프로젝트/창 슬롯/pane/탭·문서/revision/언어/서버 세대와 root guard, 대기 중 원본 교체·IME·Tab/Escape/typeahead·AccessKit 메모리 입력을 검증했습니다. 실제 OS 접근성/IME와 검색/Git 나머지 2뷰를 완료로 세지 않습니다. `2026-10-10-native-batch17-outline-breadcrumbs.md`에 실제 67대상 전체 실행·추가 실패/수정·라이선스·보호/실기 부채를 기록했습니다.

batch18은 typed 여러 프로젝트 root의 workspace/symbol을 등록 순서로 합치고 한 서버의 오류/미지원을 독립 처리합니다. 단일 200ms trailing·JS 공백·질의/프로젝트/owner/세대/취소와 # Hash/컨테이너/이름 강조·loading/빈 상태·키/마우스 선택을 현재 창의 preview/기존 탭과 UTF-16 reveal에 연결했습니다. 실제 child의 요청 보류/다른 문서 동기화/취소·서버 종료/재시작/replay, host 경계/늦은 선택과 실제 앱의 이모지 뒤 위치 이동을 검증했습니다. 전체 86대상·1018건·보호/실기 부채·동결 컴파일·Cargo 직렬 규칙 이탈은 `2026-10-10-native-batch18-workspace-symbols.md`에 기록했습니다.

batch19는 typed FoldingRangeRequest의 사용자 문자열 kind를 보존하고 실제 LSP 구문 범위·종류를 수동 범위와 합쳐 원본 카탈로그 19종 명령/기본 키·본문/gutter·고정 줄에 연결했습니다. 종류/빈/null/오류/미지원·편집/닫힘 취소·보류 중 다른 문서 응답·서버 재시작/mirror·프로젝트 격리를 실제 child 4건에서 확인했고 앱의 Import 접기와 UTF-16 reveal 펼치기가 통과했습니다. 빈 줄 접기·삭제 회수·수동 범위 편집/undo/redo·다중 뷰/readonly·Arc 교체를 검증했습니다. 최초 전체 실패 3건과 영향 재검사·프로토타입 standalone lock/edition 및 실기/성능/출시 부채는 `2026-10-10-native-batch19-syntax-folding.md`에 기록했습니다.

## XLML 실패 원인의 코드 대조

기존 실패 로그 `/private/tmp/taide-batch7-app-final-20261009.log:1089`는 `tests/preview-spreadsheet-xlml.rs:94`에서 `<Row ss:Index="0"><Cell/></Row>`를 수락했음을 기록합니다. 해당 parser·test는 batch7 이후 변경되지 않았으므로 이 실행 증거를 재사용합니다.

현재 호출 경로는 다음과 같습니다.

1. `preview_spreadsheet.rs:68`은 첫 비공백 바이트가 `<`인 입력을 XLML `decode`로 전달합니다.
2. `preview_spreadsheet_xlml.rs:468`은 XML 파싱 전에 HTML의 `is_html`을 호출합니다.
3. `preview_spreadsheet_html.rs:99`는 앞 1,024문자의 따옴표 문자열을 지우고 소문자로 만든 뒤 `<?xml`이 없고 `<table`이 있으면 HTML로 분류합니다.
4. 실패 테스트의 `workbook` 헬퍼는 XML 선언 없이 `<Workbook ...><Worksheet ...><Table>...`를 생성합니다. 이 입력은 HTML로 분류돼 XLML의 `scan`·`attributes`를 거치지 않습니다.
5. `preview_spreadsheet_xlml.rs:132`의 인덱스 파서는 이미 `checked_sub(1)`로 0을 거절합니다. `scan`의 Start와 Empty 모두 `State::start`에서 속성을 읽으므로 빈 Cell만의 속성 누락으로 추정해 수정해서는 안 됩니다.

이 원인 대조 이후 batch8에서 실제 Workbook 루트를 확인하는 분류로 수정했습니다. 수정 전 두 실패를 재현하고 수정 후 XLML 4건·HTML 3건 모두 통과했습니다. 이후 batch8 앱 전체 대상에서도 XLML 4건이 통과했습니다. 상세는 `../bug/2026-10-09-native-xlml-workbook-classification.md`와 `2026-10-09-native-batch8-integration.md`입니다.

## 점검 상태

- [x] a. 전체 완료 게이트와 현재 증거의 범위 대조
- [x] b. 찾기·표시 설정·LSP 경로와 배치 7 명령 연결 대조
- [x] c. 기존 XLML 실패 로그와 실제 parser 분류 경로 대조
- [x] d. 최신 전수 기능 대응표 확보 — batch14 Source·앱/요청 소비·관련 검사 묶음 재판정 599행, 원본/ID 중복·누락 0, 완료 282/588(48.0%)
- [ ] e. 실기·성능·보안·출시의 전체 증거 확보 — 보호/ignored·TS 제거 조건을 보존하며 전체 전환 작업에서 계속 확인

이 근거 점검은 4/5 완료(80%)이며 전체 전환율이 아닙니다. 기능 대응표 기준 48.0%와 별도 전체 구조/실기·성능·출시 미완료를 구분합니다. 사용자가 기존 배치별 중단 지시를 해제한 전체 전환 합의 `../acknowledge/2026-10-09-native-full-resume.md`를 따릅니다. 전체 목표는 계속 진행하며 batch19의 구문 접기 공급·관련 명령까지 구현·검증했습니다. 다음 정의/선언/타입 정의/구현 이동·참조/peek와 잔여 LSP 사용자 기능을 이어갑니다.
