# 배치 23 — 편집기의 나머지 기능

기준: 이동/명령 커밋 `a296a28e`와 배치 23 종료 회수·추가 앱 검증 작업 트리, 배정 33행. 메인이 직접 수행하며 서브에이전트/workflow를 사용하지 않습니다. 전체 전환율·잔여 시간은 미산정이며 완료 288·미완료 300행을 유지합니다. editor-51은 아래 성공과 실패를 구분한 근거에 따라 partial을 유지하며 complete로 올리지 않습니다.

## editor-51 문서 하이라이트 원본과 현재 경계

원본 `src/shared/lib/lsp/adapters/document-highlight.ts`는 capability가 있을 때 공급자를 등록하고 `textDocument/documentHighlight`로 모델 URI·UTF-16 위치를 전달합니다. 취소되었거나 null 응답이면 빈 배열을 돌려줍니다. 종류 생략은 Text이며 Text/Read/Write를 Monaco의 해당 종류로 변환합니다.

설치된 원본 `node_modules/monaco-editor/esm/vs/editor/contrib/wordHighlighter/browser/wordHighlighter.js`에서 확인한 동작은 다음과 같습니다.

- 기본 occurrencesHighlight는 singleFile이며 기본 occurrencesHighlightDelay는 0입니다. 요청 진입의 runDelayer는 50ms입니다. 이 값은 설치된 `editorOptions.js`와 wordHighlighter의 실제 상수 근거이며 구현 시 이름 있는 상수로 둡니다.
- 텍스트 포커스가 있는 편집기의 주 선택만 기준으로 사용합니다. 여러 줄 또는 단어 바깥까지 선택하면 하이라이트를 지웁니다.
- 등록 순서/점수에 따른 첫 유효 공급자 결과를 사용합니다. 빈 배열도 유효하므로 다음 공급자로 넘어가지 않습니다. TAIDE adapter의 null 응답도 빈 배열이 됩니다. 공급자 오류는 다음 공급자 처리와 구분합니다.
- URI가 일치하는 여러 편집기의 본문에 결과를 표시합니다. 읽기·쓰기·텍스트 종류별 배경/테두리와 overview center·minimap 표시가 있습니다. 정확한 색 기본값/별칭은 `highlightDecorations.js`의 등록과 현재 테마의 원본 색을 기준으로 구현합니다.

최초 조사 시 native 앱에는 요청·결과·표시를 연결한 문서 하이라이트 소비자가 없었습니다. 기존 `crates/taide-lsp/src/native/feature.rs`는 typed DocumentHighlight 요청을 이미 제공합니다. 아래 경계에서 새 엔진이나 의존성 없이 공급과 표시를 연결했습니다.

- `native/taide-native-app/src/lsp.rs`의 feature_providers·세션/문서 소유·명령/응답 worker와 기존 문서 도움말/구문 접기 공급 패턴
- `native/taide-native-app/src/application.rs`의 LSP reply 소비·본문 EditorRequest 장식·포커스 경계
- `native/taide-native-app/src/editor-locations.rs`의 peek EditorRequest·보조 입력/장식 경계
- `native/taide-native-editor/src/decoration.rs`의 Inline/Overview·revision·stickiness와 기존 위치/구문/문서 도움말의 취소·문서/뷰 소유 검증
- `native/taide-native-ui/src/presentation.rs`와 `crates/taide-model/src/theme.rs`의 현재 테마 경계

설치 SDK의 `lsp-types 0.97.0/src/document_highlight.rs`와 egui/ecolor 원문을 추가로 읽어 typed 파라미터·응답과 색 API를 확인했습니다. 최초 공급·표시 커밋에서는 원본 F7/Shift+F7 연결이 남아 있었으며 후속 연결은 다음 절에 기록합니다. docs.rs의 두 타입 페이지 조회는 도구 접근 오류였으므로 공식 API 검증 성공으로 세지 않습니다. Editor/UI에 정규식·구문 엔진 의존성을 추가하지 않았습니다.

## 구현·검증 대기

- [ ] 요청/취소/소유: 주 선택의 UTF-16 위치·단어 조건·debounce·편집/언어/파일/탭/뷰/공급자 변경·종료·오래된 응답·capability 변경을 실제 공급 경계에 연결합니다.
- [ ] 실제 표시/명령: 종류별 본문·peek·동일 문서 mirror 표시·테마/overview/minimap과 원본 하이라이트 명령을 연결합니다.
- [ ] 실제 child/앱: 정상·빈/null·오류·미지원·UTF-16/잘못된 범위·취소/오래된 응답과 readonly/tier/편집/undo/다중 뷰를 관련 검사로 검증합니다.
- [ ] 배치 게이트: 변경 크레이트 전체 대상은 배치 종료에 --no-fail-fast로 직접 실행하며 같은 성공은 재사용합니다. frozen compile·fmt/범위·디스크·실기/출시 부채와 선별 Git 근거를 기록합니다.

나머지 32행의 원본 대조와 구현은 미완료입니다. editor-51은 missing에서 partial로 갱신하고 완료 합계는 변경하지 않았습니다.

## 공급·표시 구현과 관련 검증

`editor-highlights.rs`·`lsp-editor-highlights.rs`에서 뷰/소유 탭·문서/언어/revision·선택·공급자 generation/capability 소유, 50ms debounce와 단일 제출, 취소/오래된 응답 거절, Text 기본값·Read/Write 배경/테두리/overview center/minimap을 구현했습니다. null은 TS adapter처럼 빈 배열로 처리해 첫 정상 공급자 결과에서 멈추며 오류는 다음 공급자 후보로 넘어갑니다. 실제 공급자 여러 개의 우선순위 검사는 아직 남아 있습니다.

본문과 peek는 같은 viewport 상태를 공유하며 같은 document의 mirror에 표시합니다. `editor-locations::Provider`의 공유 상태 수명과 프레임별 대여 수명을 분리해 후속 포커스/명령 처리에서의 대여 충돌을 해결했습니다. UI의 실제 본문/peek 장식 소비 지점에 연결했지만 peek에서 시작하는 새 하이라이트 흐름의 실제 앱 검사는 아직 남아 있습니다.

검증과 수정 근거는 다음과 같습니다. 모든 Cargo/fmt는 이전 프로세스의 terminal을 확인한 뒤 직렬로 실행했습니다.

- 최초 example 컴파일은 공유 상태/프레임 대여 충돌 2건이었고 수명을 분리해 해결했습니다. 다음 example 오류의 JSON-RPC ID Option을 기존 서버 계약대로 검증해 해결했습니다. `highlights-mock-root-final.log`의 example 빌드는 exit 0입니다.
- 최초 lib 검사 컴파일의 EditorLimits 경로·disconnect API·기존 공급자 fixture 필드를 실제 API에 맞게 수정했습니다. 첫 실행은 상태 7통과·기본 색 1실패·child 2실패였습니다. 색 표본 `127 vs 53`의 원인은 선형 HSV를 sRGB 계산에 사용한 것이어서 gamma RGB를 직접 변환하도록 수정했습니다. 설치 Monaco Color 원본에서 얻은 4개 표본과 u8 색 양자화 허용 차이 2 이내를 확인했습니다. 이 표본 검사는 실제 화면의 전체 픽셀 게이트를 대신하지 않습니다.
- `highlights-diagnostic.log`는 상태/종류/취소/재요청 억제/mirror/테마/색 8건 통과입니다. 동일한 상태 코드의 성공은 재사용했습니다. 이후 검사 이름만 snake_case로 정리했으며 검사 본문과 입력은 변경하지 않았습니다.
- child 대기는 단계를 기록해 initialize임을 확인했고 `highlights-ready-diagnostic.log`의 `Degraded`와 임시 stderr의 `fixture requires valid roots and client capabilities`로 새 mock 모드의 root 검증 목록 누락을 확정했습니다. root/폴더 검증을 기존 파일 기반 모드와 동일하게 연결했으며 시간 제한을 늘리거나 검증을 해제하지 않았습니다.
- `highlights-child-final.log`는 실제 child/mirror·UTF-16 위치·정상/빈/null/오류/잘못된 응답/미지원·실제 시작된 대기 요청의 뷰 회수 취소·disconnect 두 검사 통과, exit 0입니다. `highlights-actual-app.log`의 실제 constructor/worker/poll/UI 본문 검사 1건도 exit 0이며 LSP 배경이 그리기 결과에 있고 document revision이 그대로임을 확인했습니다. 기존 도움말/peek/완성 흐름도 같은 앱 검사에 포함됐습니다. 이 검사로 새 하이라이트의 peek 시작/readonly 전체를 통과했다고 주장하지 않습니다.

최종 활성 프로젝트/종료 소유 조건을 포함한 `highlights-actual-app-final.log`의 동일 실제 앱 검사도 1건 통과·exit 0이며 중복 합산하지 않습니다. `cargo check --tests`에서 기존 통합 검사의 공개 Reply 열거형 처리가 새 Highlights 변형을 빠뜨린 것을 확인해 기존의 예상 밖 심볼 요청 실패 분기에 명시적으로 포함했습니다. 해당 검사를 무시하도록 바꾸지 않았습니다. `highlights-final-check-fixed.log`의 App 전체 테스트 대상 컴파일과 최종 App fmt check는 exit 0입니다. 새 snake_case 경고는 제거했으며 기존 vendor 경고와 linker unwind 경고는 변경하지 않았습니다. 디스크 여유는 555GiB/70%입니다.

최초 공급·표시 로그의 공통 경로는 `/private/tmp/taide-batch23-`입니다. 해당 시점의 서로 다른 관련 성공은 상태 8·child 2·실제 앱 1의 11건이며 원본 스크립트 출력·최초 실패 로그도 보존했습니다. 전체 App/배치 실행 게이트, 실제 화면/OS IME/접근성/대형 성능/soak/출시 게이트는 부분 구현에서 실행하지 않았습니다.

## 하이라이트 이동·실행 명령과 재사용

설치 wordHighlighter의 `moveNext`/`moveBack`, `_run`의 저장된 범위 재사용, 명령 등록과 `restoreViewState`/`restore(250)`을 읽었습니다. F7/Shift+F7은 표시된 하이라이트가 있을 때만 실행하며 trigger의 기본 키는 없습니다. 시작 위치로 정렬한 범위를 순환하고 목적지 시작으로 단일 커서를 옮겨 화면 밖이면 중앙으로 드러냅니다. 중복 범위는 이동 대상에서 합치며 범위 밖에서는 다음=첫 범위·이전=마지막 범위를 사용해 원본의 음수 인덱스 특이 동작을 강제 재현하지 않습니다.

UI 명령 레지스트리의 typed HighlightCommand·native-host 기본 키·readonly 가용성을 연결했습니다. App의 실제 명령 가용성은 표시 상태를 확인합니다. 본문·peek 명령 소비와 마지막 소스 뷰의 소유를 연결했고 명시 trigger는 빈 결과를 다시 요청하며 250ms 표시 지연을 적용합니다. 완료된 표시 안의 선택 변경만 재사용하며 요청 중의 선택·문서 revision/언어·공급자·뷰 회수 검증은 유지합니다.

- `highlights-reuse-repro.log`와 `highlights-reuse-repro-actual.log`는 잘못된 필터로 검사 0건입니다. 통과로 세지 않습니다. 수정한 `highlights-reuse-repro-module.log`에서 기존 8건 통과·새 커서 이동 회귀 1건 실패를 확인한 뒤 수정했습니다.
- `highlights-navigation-state.log`에서 테스트가 비공개 reveal 필드를 직접 읽은 컴파일 오류를 확인했습니다. 기존 공개 `take_selection_reveal`로 바꿨으며 프로덕션 필드의 가시성을 넓히지 않았습니다. `highlights-navigation-state-final.log`는 상태 12건 통과·exit 0입니다.
- `highlights-navigation-app.log`는 실제 constructor/child/worker/poll/그리기 흐름 1건 통과입니다. F7·Shift+F7 왕복/순환과 커스텀 F8 trigger·문서 revision 유지가 포함됩니다. 자동 요청만으로 trigger 검사도 통과할 수 없도록 실제 지연 마감 관찰을 추가한 `highlights-navigation-app-final.log`도 1건 통과·exit 0이며 중복 합산하지 않습니다.
- `highlights-navigation-registry.log`는 readonly 포함 명령 등록 10건, `highlights-navigation-keymap.log`는 Mac/Windows/Linux·재지정·IME를 포함한 플랫폼 키맵 10건 통과·exit 0입니다. 키 입력은 내부 egui RawInput이며 OS 합성 입력을 사용하지 않았습니다.
- `highlights-navigation-boundary-repro.log`에서 readonly 메타데이터의 이동·내용/버전 보존 1건이 통과했고, 기호로 시작하는 표시로 이동한 뒤 자동 관찰이 표시를 지우는 회귀 1건을 재현했습니다. 원본 `_ignorePositionChangeEvent` 처리에 맞춰 이미 이동한 선택과 문서/소유/공급자가 그대로이면 비단어 시작도 유지합니다. 일반 사용자 선택 변경의 단어 조건은 유지하며 `highlights-navigation-state-boundary-final.log`는 전체 관련 상태 14건 통과·exit 0입니다.
- `highlights-navigation-check.log`의 App 전체 테스트 대상 컴파일과 App/UI fmt check는 exit 0입니다. 앞서 성공한 child 2건과 UI 등록/플랫폼 20건은 해당 공급·UI 코드가 바뀌지 않아 재사용합니다. frozen host 컴파일은 `highlights-navigation-frozen-host.log`에서 exit 0입니다. 배치 전체 --no-fail-fast 실행, 실제 peek 시작·readonly/tier·추가 공급자·실기/출시 게이트와 나머지 배치 23 요구사항은 미완료입니다.

## 진척·반복 점검

기호 시작 회귀 수정 뒤 `highlights-navigation-app-boundary-final.log`도 실제 앱 1건 통과·exit 0이며 이전 실제 앱 성공과 중복 합산하지 않습니다. 최종 `highlights-navigation-check-final.log`는 App 전체 테스트 대상 컴파일 exit 0입니다. 현재 관련 직접 실행의 서로 다른 성공은 상태 14·실제 앱 1·UI 등록 10·플랫폼 10의 35건이며 실행되지 않은 전체 기능/출시 게이트로 확대하지 않습니다.

최종 App fmt check와 변경 없는 UI fmt check는 exit 0입니다. frozen host/Wasm 컴파일은 `highlights-navigation-frozen-host.log`/`highlights-navigation-frozen-wasm.log`에서 각각 exit 0이며 source·manifest·lockfile·의존 그래프를 수정하지 않았습니다. 기존 vendor와 frozen 경고는 요청 범위 밖으로 유지합니다. 디스크 여유는 552GiB/70%이며 보호 앱과 실제 데이터/OS/클립보드/Keychain/Trash를 건드리지 않고 정리 명령도 실행하지 않았습니다.

최초 진척 점검은 실제 감사 JSON 599행을 다시 집계했습니다. 제외 10·동결 1을 뺀 588행 중 complete 288·partial 92·unwired 112·missing 96으로 미완료 300행이었습니다. 공급/표시 검증 뒤 editor-51을 partial로 갱신한 현재 분류는 complete 288·partial 93·unwired 112·missing 95로 미완료 300행을 유지합니다. 모든 미완료 ID의 배치 배정 누락 0·중복 0·배치 23 미완료 33행·최종 배치 33을 직접 확인했습니다. 최초 계획의 302행은 배치 22 완료 전 배정 스냅샷입니다.

최초 진척 조사 시 HEAD는 `69c3438b`였고 upstream 차이 0/0을 확인했습니다. 최근 제품 커밋 `603e6025`·`7172a544`·`3f9a2c18`에는 기본 선택/색 견본·상세 공유 상태·Read More 마우스 버튼의 서로 다른 변경이 있습니다. 당시 프로세스 이름과 CPU 누적 시간으로 빌드/테스트와 이전 조사 스크립트의 실행 여부를 확인했고 해당 작업 프로세스는 없었습니다. 이는 당시 실행의 점검이며 전체 작업에서 반복 낭비가 전혀 없었다는 증거로 확대하지 않습니다. 최초 조사에서는 Cargo/fmt나 이전 성공 검사를 다시 실행하지 않았으며 이후 부분 구현의 실제 실행 결과는 앞 절에 구분했습니다.

## peek·종료 회수·다중 공급자 추가 검증

이번 실행 로그의 경로는 /private/tmp/taide-batch23-입니다. 이전 절의 성공은 당시 변경 상태의 기록이며 최신 확장 앱 검사 전체의 성공으로 합산하지 않습니다.

- highlights-peek-close-repro.log: 실제 앱 종료 직후 완료된 하이라이트 요청의 취소 watch가 false인 실패를 재현했습니다. NativeApplication::close에서 State::clear를 호출해 전체 뷰포트의 대기/완료 요청을 취소하고 sources를 비웁니다. highlights-peek-close-final.log는 readonly 추가 전 실제 앱 1건 통과·exit 0·5.58초입니다.
- highlights-close-state-child-final.log: 상태 15·실제 child 3건, 총 18건 통과·exit 0·5.01초입니다. 새 다중 공급자 검사는 Python의 ruff/basedPyright를 실제 child로 실행해 6가지 응답 조합과 두 프로젝트의 공급자 소유를 확인합니다. 정상 첫 결과·빈 배열·null은 첫 공급자에서 끝나며 오류·잘못된 응답·미지원은 다음 유효 공급자 결과를 사용합니다. 종료 후 tracked task는 0입니다.
- mock의 --native-documentation-peek-highlights와 --native-highlights-alternate는 기존 모드의 의미를 바꾸지 않고 실제 peek/다중 공급자 검사를 지원합니다. mock 빌드는 highlights-multi-mock.log에서 exit 0입니다.
- readonly 이전 실제 peek 검사에는 별도 peek 문서의 실제 표시, F7/Shift+F7, Root 명령의 마지막 peek 소스 소비, 명시 trigger, 편집 후 취소와 닫기가 포함됩니다. OS 합성 입력 대신 내부 egui RawInput을 사용합니다.

실패를 성공으로 합산하지 않습니다. 다음은 수정 또는 진단 근거입니다.

- highlights-peek-app.log와 highlights-peek-app-diagnostic.log: wait_for가 logic만 실행해 UI의 50ms 관찰이 진행되지 않았습니다. 필요한 predicate에서 UI 프레임을 실행했고 highlights-peek-app-frames.log는 앱 1건 통과·exit 0·5.81초입니다. 타이머를 늘리지 않았습니다.
- highlights-multi-child.log: 실제 manifest ID인 basedPyright 대신 basedpyright를 기대해 fixture lookup이 실패했습니다. 세 대체 사례의 기대 ID를 원본 manifest에 맞췄고 최종 18건 실행에서 해당 다중 공급자 검사가 통과했습니다.
- highlights-peek-readonly-close-final.log: dirty 문서를 refresh_clean_file로 바꾸려다 UnsavedChanges로 실패했습니다. 소유한 임시 파일을 실제 저장하고 공개 mark_saved를 거친 뒤 readonly 메타데이터를 적용했습니다. highlights-peek-readonly-close-saved-final.log의 private SaveSnapshot.rope 접근 컴파일 오류는 공개 rope()로 수정했습니다.
- highlights-peek-readonly-close-api-final.log와 highlights-peek-readonly-close-focus-final.log: readonly 이동의 실제 키 입력이 0 위치에 남았습니다. 프레임 갱신/이동 방향만 바꿔도 해결되지 않았으므로 해당 포커스 가정을 종료했습니다.
- highlights-peek-readonly-close-pass-final.log와 highlights-peek-readonly-close-active-final.log: 기존 긴 앱 검사의 F4 단계가 시간 초과했습니다. 진단 중 이동 목적지를 입력 소스로 잘못 활성화한 오류도 highlights-peek-readonly-close-source-active-final.log에서 session=None으로 드러났습니다. F4는 현재 source에서 누른 뒤 다른 destination을 확인하도록 고쳤으며 활성화·실제 본문 포커스·키 해제와 실패 시 세션 상태를 명시했습니다.
- highlights-peek-readonly-close-source-destination-final.log와 highlights-peek-readonly-close-command-diagnostic.log: F4 이후 readonly까지 진행했지만 Shift+F7 이동은 실패했습니다. 후자의 실패 이전 단언에서 실제 앱의 queued Previous/Next 왕복은 관찰했고, 키 입력 뒤에는 first caret=0·queued=false·status=None입니다. 이 단언들은 독립된 성공 테스트로 세지 않습니다. 현재 키 입력 경로 진단이 미완료이며 검사 전체는 실패입니다.

본문과 도움말/완성 컨트롤의 focus ID가 같은 뷰에 등록됩니다. NativeEditor의 inspection 전용 is_body_focus_target은 실제 InputState가 있는 본문 ID를 구분합니다. 제품 화면이나 편집 동작을 추가하지 않으며 readonly 키 검사에 실제 본문·등록/실행 허용·이전 키 입력 소유 조건을 명시해 다음 진단에 사용합니다.

Cargo/fmt 종료를 확인하기 전에 mock 빌드를 시작한 실행 순서 오류 1건이 있었습니다. fmt session 74697과 빌드 session 20901의 종료·exit 0은 확인했지만 이번 실행 전체를 직렬이었다고 보고하지 않습니다. 복원 시 프로세스 이름 조회에서 Cargo/rustfmt/rustc는 없었고, 이후에는 live session의 종료를 확인한 뒤 다음 Cargo/fmt를 실행합니다. 성공한 18건은 State/worker 동작이 변경되지 않아 재사용합니다.

진척 재점검: 실제 JSON 599행 중 complete 288·partial 93·unwired 112·missing 95·제외 10·동결 1입니다. 미완료 300 ID의 배정 누락 0·중복 0, 배치 23/24/25/26/27/28/29/30의 잔여는 33/52/23/64/32/39/51/6행이며 최종 배치는 33입니다. 행 개수 비율을 전체 진척률이나 ETA로 사용하지 않습니다. 조회 시 최근 8커밋은 서로 다른 변경이고 HEAD a296a28e·upstream 0/0이었습니다. 이는 전체 실행의 종료성을 증명하지 않으며 현재 실패 반복과 원인 분리의 한계를 그대로 기록합니다.

디스크는 이번 확인에서 544GiB·71%입니다. 정리 명령·보호 앱/실제 데이터/OS/클립보드/Keychain/Trash 변경은 없습니다. 최신 실제 앱 성공, 전체 대상 컴파일·선별 Git, 큰 tier/공급자 변화와 배치 전체 --no-fail-fast·실기/출시 게이트는 아직 미완료입니다.

## 최종 단위 검증 결과와 잔류 단축키 부채

highlights-peek-readonly-close-body-focus-diagnostic.log와 highlights-peek-readonly-close-cache-diagnostic.log는 본문 포커스·캐시·source/owner·전역 편집기 연결·등록/실행 허용 조건이 모두 성립해도 Shift+F7이 대기열에 들어오지 않는 실패입니다. busy/disk choice/tab close/shutdown도 false였습니다. 단순 포커스나 readonly 계산 수정 가정을 더 반복하지 않았습니다.

highlights-peek-readonly-close-chord-diagnostic.log의 ChordStatus.pending 필드 오용은 실행 전 컴파일 오류이며 shortcut 필드로 수정했습니다. highlights-peek-readonly-close-chord-api-diagnostic.log는 앞 단계의 CtrlK 연속 단축키 대기가 실제 남은 것을 확인한 실패입니다. highlights-peek-readonly-close-neutral-final.log에서는 Escape 뒤에도 대기가 남았습니다. 이 연계 문제는 미해결로 보존하며 readonly 기본 키의 성공과 구분합니다.

독립 readonly 상호작용 단계의 사전 조건을 기존 Views::clear_keymap_chord로 초기화하고 대기 없음·실제 본문·문서/소유·명령 허용을 단언합니다. 제품의 키 처리나 이동 계산은 바꾸지 않았습니다. highlights-peek-readonly-close-clean-phase-final.log는 실제 앱 1건 통과·exit 0·5.64초이며 peek의 공급/표시/F7/ShiftF7/명시 trigger/편집/닫기, normal tier readonly 문서의 queued 왕복과 ShiftF7·입력 거절/내용/revision/readonly 보존, 앱 종료 취소/작업 0을 포함합니다. 현재 직접 실행 성공은 상태 15·child 3·실제 앱 1의 19건입니다. 이전 앱 검사와 각 실패 이전 단언은 중복 합산하지 않습니다.

일반 사용자 흐름의 CtrlK→peek/도움말 닫기→Escape→F7 연계는 위 초기화를 넣어 완료 처리하지 않습니다. 별도의 재현에서 전역 단축키 pending/deferral과 로컬 입력 소비를 원본 Monaco와 대조하고 실제 정상 취소를 검증해야 합니다. 큰 tier/공급자 변화/원본 설정과 배치 전체 게이트도 미완료여서 editor-51은 partial입니다.

최종 App 전체 테스트 대상 컴파일은 highlights-close-app-check-final.log에서 exit 0·1분 16초, UI native-host/inspection 전체 테스트 대상 컴파일은 highlights-close-ui-check-final.log에서 exit 0·32.90초입니다. App/UI fmt check도 각각 직접 실행해 exit 0입니다. 전체 테스트 실행과 컴파일을 구분하며 배치 전체 --no-fail-fast 실행은 아직 아닙니다.

최종 감사 도구의 --write는 highlights-close-audit-final.log에서 exit 0·599행/293근거 경로/588대상·288완료이며 표의 행 비율은 전체 전환율이 아닙니다. QA/JSON/표 포맷과 소유 파일 diff check가 통과했습니다. frozen source·manifest/lock 변경은 0이며 이전 host/Wasm 성공은 변경 없는 조건에서 재사용합니다. 최신 디스크는 542GiB/71%이고 빌드 정리나 보호 범위 변경은 없습니다. 검증된 종료 회수·실제 peek/readonly/공급자 검사와 본문 inspection, 관련 QA/기능표/PROCESS 상단만 선별 Git 대상입니다. 이전 무관한 문서 변경과 PROCESS 하단은 보존합니다.

## 로컬 Escape와 전역 연속 단축키 취소

기준은 `6f1ae6ba` 이후의 수정입니다. 위 절의 잔류 단축키 부채를 별도 실패 검사로 재현했습니다. `Views::capture_window_keymap_with_local`은 로컬 입력을 남기는 분기에서 전역 keymap 처리를 건너뛰어 pending/편집기 deferral을 보존했습니다. 설치된 Monaco의 `abstractKeybindingService.js:194`는 연속 단축키 도중 대응 명령이 없는 입력에서 `_leaveChordMode`를 호출합니다. DOM 팝업 소비까지 동일하다고 확대하지 않으며, 정상 Escape 취소와 로컬 팝업 입력 보존을 native 경계에서 검증했습니다.

- `local-escape-chord-repro.log`: 전역 pending과 편집기 deferral 두 경우를 준비한 단일 검사가 첫 pending 사례의 취소 단언에서 실패했습니다. 0통과·1실패·exit 101·0.04초입니다. Escape 누름/뗌은 로컬 입력으로 남지만 전역 대기는 남았습니다.
- `local-escape-chord-final.log`: 로컬 입력 분기의 수식 키 없는 Escape 누름에서 현재 뷰포트의 전역 대기를 회수합니다. 다른 로컬 키와 수식 키 Escape는 이 회수 조건에 포함하지 않습니다. 두 대기 종류가 모두 해제되고 Escape 누름/뗌이 그대로 남으며 새 명령을 만들지 않는 검사 1건이 통과했습니다. exit 0·0.02초입니다.
- `local-escape-chord-app-final.log`: 실제 앱의 후기 readonly 단계에 대기가 남아 있어야 한다는 새 단언이 실패했습니다. 제품 수정으로 앞선 팝업의 Escape가 이미 대기를 회수한 것이며 실행 0통과·1실패·exit 101·5.71초입니다. 후기 단계에서 다시 대기를 가정하지 않고 실제 peek 단계의 취소 전후에 단언을 옮겼습니다.
- `local-escape-chord-app-linked-final.log`: CtrlK 이후 로컬 명령까지 대기 존재, peek 찾기 Escape 뒤 대기 없음, readonly 단계의 대기 없음과 ShiftF7·입력 거절·앱 종료 회수를 실제 앱에서 확인했습니다. 검사 준비의 `clear_keymap_chord`를 제거했습니다. 앱 1건 통과·exit 0·5.66초입니다.
- `local-escape-chord-window-final.log`: 기존 사건별 포커스·terminal/editor/button·AX·메뉴와 유효한 로컬 chord 전달 검사 6건 통과·exit 0·0.06초입니다.

이번 수정의 직접 실행 성공은 서로 다른 8건입니다. 이전 앱 성공과 실패를 중복 합산하지 않고, 변경 없는 상태/child 검사의 성공 근거는 재사용합니다. Cargo/fmt는 앞 프로세스의 종료를 확인한 뒤 직렬로 실행했습니다. 큰 tier/공급자 변화/원본 설정·배치 전체 실행·실기/출시 게이트는 남아 있어 editor-51과 배치 23 전체는 미완료입니다.

진척 점검에서 감사 JSON을 직접 재집계한 결과는 599행 중 complete 288·partial 93·unwired 112·missing 95·제외 10·동결 1입니다. 유효 588행의 미완료는 300행이며 행별 작업량 차이 때문에 전체 완성률·신뢰할 잔여 시간을 산정하지 않습니다. 현재 배치 23·최종 계획 33을 유지합니다. 프로세스 이름 필터에서 실행 중인 Cargo/rustfmt/rustc/Bun/앱 검사는 없었고 최근 커밋은 다른 변경이었습니다. 이는 현재 실행 점검이며 이전 불필요한 재시도나 장기 대기 기록을 없애거나 전체 세션의 종료성을 증명하지 않습니다.

`local-escape-chord-app-check-final.log`는 App inspection 전체 테스트 대상 컴파일 exit 0·31.96초입니다. `local-escape-chord-fmt-final.log`의 App fmt check와 `local-escape-chord-audit-final.log`의 감사 재생성도 exit 0입니다. UI/Editor/SDK/Syntax·frozen host/Wasm의 변경 없는 성공은 재사용하며 배치 전체 `--no-fail-fast` 실행으로 확대하지 않습니다. 이번 frozen source·manifest/lock 변경은 0이고 디스크 여유는 542GiB·사용률 71%입니다. 빌드 정리나 보호 범위 변경은 없습니다.

## 크기 등급 변경과 LSP 공급 경계

기준은 `7dd5703d` 이후 수정입니다. 원본 `use-lsp-session.ts:33`의 Normal 등급 연결과 파일 권한/인코딩 때문에 발생한 read_only 플래그를 구분했습니다. TS 설정과 `code-editor.tsx`에는 occurrencesHighlight/occurrencesHighlightDelay의 사용자 설정 연결이 없으며 설치 Monaco의 기본값은 singleFile/0ms입니다. 기존 native의 50ms 관찰과 명시 trigger의 250ms 지연은 별도 동작입니다.

`EditorStore::observe_file`은 내용이 같으면 revision을 유지하면서 메타데이터를 갱신합니다. 기존 하이라이트 Request는 크기 등급을 비교하지 않아 대기·완료 표시를 유효하게 판단했고, `LspBridge::highlight_providers`는 이전 child의 열린 문서만 보고 제한 등급에도 공급자를 반환했습니다. 이 창을 실제 child가 연결된 상태에서 재현했습니다. Request의 문서 식별에 tier를 포함하고 LSP 공급 경계에서 Normal 이외 등급을 즉시 제외했습니다. 원본 기본 텍스트 공급자의 등급 정책까지 이 LSP 차단으로 대체하지 않습니다.

- `highlight-tier-state-repro.log`: 존재하지 않는 Colors::defaults를 사용한 검사 준비 컴파일 오류입니다. 기존 colors() 헬퍼로 수정했습니다.
- `highlight-tier-state-child-repro.log`: 기존 readonly 검사의 파일 fixture를 공통화하면서 남긴 CONTENT 상수 참조의 컴파일 오류입니다. FILE_CONTENT로 수정했습니다. 두 컴파일 실패는 제품 재현/성공으로 세지 않습니다.
- `highlight-tier-repro-final.log`: 실제 실행 0통과·2실패·exit 101·3.99초입니다. 상태 검사에서 Large 등급 변경 뒤 이전 요청이 계속 문서를 설명하는 실패, 실제 child 검사에서 이전 서버가 연결된 채 Large 등급에도 공급자를 반환하는 실패를 각각 확인했습니다.
- `highlight-tier-state-child-final.log`: 상태 16·실제 child 4건, 총 20건 통과·exit 0·5.36초입니다. 새 상태 검사는 Large/ReadOnly 각각 대기·완료 상태를 확인하며 동일 revision, 즉시 표시 만료, reconcile 취소, 오래된 응답 거절과 Normal 복원을 포함합니다. 실제 child 검사는 두 등급 모두 서버의 비동기 문서 회수 전에 즉시 공급자 차단, 실제 mirror 닫기·재열기, 오래된 응답 거절, Normal/read_only 파일의 실제 새 하이라이트와 정상 종료/작업 0을 확인합니다.
- 기존 readonly 이동 fixture는 크기 기준 ReadOnly 등급의 가짜 공급자로 실행하던 조건을 Normal/read_only로 바로잡았습니다. 원본의 실제 LSP 연결 가능 조건에서 내용을 바꾸지 않는 이동을 검증하며 이전 실제 앱의 Normal/read_only 검증과 일치합니다.

크기 등급은 작은 임시 문서의 메타데이터로 주입해 비동기 회수 경계와 내용이 같은 메타데이터 갱신을 분리했습니다. 실제 20MB/30만 줄 문서의 성능·화면/OS 게이트를 실행했다고 확대하지 않습니다. App 외 크레이트와 mock 소스는 바꾸지 않았으며 이번 Cargo/fmt는 앞 프로세스의 종료를 확인하고 직렬 실행했습니다.

추가 원본 대조에서 `textualHighlightProvider.js`의 기본 단어 공급자가 누락된 것을 확인했습니다. 설치 Monaco는 LSP 제공자가 없어도 해당 언어의 단어를 얻어 대소문자 구분·기본 구분자에 따른 whole word 검색으로 Text 종류를 표시합니다. LSP의 유효한 빈/null(원본 adapter가 []로 변환) 결과는 대체 검색을 하지 않고, 모든 공급자의 실패/미지원은 기본 텍스트 공급자로 이어져야 합니다. 기본 검색의 999 결과 상한과 `wordHighlighter.js:670`의 20Mi UTF-16 단위/30만 줄 초과 게이트도 원본 근거이며 원본의 고정 모델 수명 버그는 강제 재현하지 않습니다. 현재 native는 공급자가 비면 닫고 있어 이 경로가 미구현입니다. 별도 구현/앱 연결·오래된 응답/취소 검증과 실제 공급자 교체, 배치 전체/실기 게이트가 남아 editor-51은 partial입니다.

`highlight-tier-app-check-final.log`의 App inspection 전체 테스트 대상 컴파일은 exit 0·27.52초입니다. `highlight-tier-fmt-final.log`의 App fmt check와 `highlight-tier-audit-final.log`의 감사 재생성도 exit 0입니다. 감사는 599행/293근거 경로·유효 588행·완료 288·미완료 300이며 기능 전체 완성률로 환산하지 않습니다. App 외 크레이트/이전 Escape 및 실제 앱 성공의 근거를 재사용하되 이번의 20건과 중복 합산하거나 배치 전체 테스트 실행으로 확대하지 않습니다. 이번 frozen source·전체 manifest/lock 변경은 0이고 디스크는 540GiB·71%입니다. 보호 범위나 빌드 정리는 건드리지 않았습니다.

## 기본 텍스트 공급자와 실제 앱 소비

기준은 `f394f33d` 이후 수정입니다. 기존 FindQuery의 대소문자 구분·whole-word 리터럴 검색을 재사용하며 앱의 공개 MonacoFindPatternCompiler 경계만 참조합니다. 새 의존성이나 Editor/UI 구문·정규식 의존성을 추가하지 않았습니다. 언어별 기존 word_range로 찾을 단어를 얻고 기본 구분자·999 결과 상한·20Mi UTF-16 단위/30만 줄 초과 차단을 적용합니다. 원본처럼 리터럴 검색은 동기 소비하며 1초 검색 상한과 검색 전후 취소를 확인합니다. 실제 대형 문서 성능/프레임 게이트까지 통과했다고 확대하지 않습니다.

LSP 공급자가 없거나 연결이 없으면 50ms 관찰 뒤 기본 Text 종류를 동일한 Entry/DecorationLayer/이동·mirror·취소 경계에서 소비합니다. 프로젝트가 없는 Consumer를 가짜 ProjectId로 표현하지 않고 Context/Request의 프로젝트를 Option으로 나타냅니다. LSP 요청은 실제 프로젝트에만 연결합니다. 유효한 provider의 빈/null 결과는 보존하고 모든 공급자의 실패·미지원/제출 실패는 기본 검색으로 대체합니다. IME·선택/문서/등급·소유·공급자 변경과 종료의 기존 만료 조건은 유지합니다.

- `textual-consumer-repro.log`: 서버가 없는 Consumer에서 요청조차 남지 않는 실패 0통과·1실패·exit 101·0.03초입니다.
- `textual-state-child-final.log`: 앱 전용 구문 크레이트의 비공개 모듈을 경유한 참조의 컴파일 오류입니다. 실제 공개 root export로 수정했습니다.
- `textual-state-child-api-final.log`: 앱에 직접 없는 ropey 경로를 검사에서 참조한 컴파일 오류입니다. 저장된 Rope의 기존 메서드를 사용했고 의존성을 추가하지 않았습니다. 두 컴파일 실패를 제품 성공/재현으로 합산하지 않습니다.
- `textual-state-child-verified.log`: peek/mirror 추가 전 상태/child 23건 통과·exit 0·5.90초입니다. 서버/프로젝트 없는 소비와 이동, 대소문자·whole word·emoji 뒤 UTF-16 범위·999 상한·취소, UTF-16 길이/줄 수의 경계값과 유효한 빈/null 대비 오류/미지원 대체를 확인했습니다.
- `textual-peek-mirror-state-child-final.log`: 최신 상태 20·실제 child 4건, 총 24건 통과·exit 0·6.17초입니다. 기본 공급자의 별도 peek 문서/owner 분리, 같은 문서 mirror 표시 공유·각 선택의 독립성·이동·뷰 닫기 취소를 추가했습니다.

`textual-actual-app-final.log`는 Root 기본 공급자 추가 시 실제 앱 1건 통과·exit 0·5.69초입니다. `textual-peek-readonly-actual-app-final.log`는 최신 실제 앱 1건 통과·exit 0·6.22초입니다. 앱의 주입된 LSP 경계를 임시로 보관해 None으로 둔 상태에서 실제 peek 및 Normal/read_only 본문의 기본 단어 표시·DecorationLayer와 그려진 shape를 확인합니다. 동일한 bridge를 복원하면 기본 요청 watch가 취소되고 실제 서버의 종류별 결과로 전환됩니다. 기존 peek 이동/trigger/입력·닫기, Escape 연계, readonly 명령·입력 거절·정상 종료 회수도 포함합니다. 이는 실제 OS 합성 입력이나 사용자 앱/데이터 변경이 아닌 내부 egui RawInput·소유 임시 데이터 검증입니다.

최신 직접 성공은 서로 다른 25건입니다. 이전 23건/앱 성공과 중복 합산하지 않습니다. None→연결 복원의 실제 소비 전환과 서버의 실제 재시작/세대·동적 capability 변경 검증을 구분합니다. 후자 및 배치 전체/실기 게이트가 남아 editor-51은 partial입니다. 이번 Cargo/fmt는 앞 프로세스 종료 뒤 직렬로 실행했습니다.

`textual-app-check-final.log`의 App inspection 전체 테스트 대상 컴파일은 exit 0·27.35초이며 `textual-fmt-final.log`의 App fmt check도 exit 0입니다. `textual-audit-final.log`는 599행/295근거 경로·유효 588행·완료 288·미완료 300으로 재생성했습니다. 원본 찾기/앱 전용 compiler의 실제 재사용 경로를 근거에 추가했습니다. frozen source·전체 manifest/lock 변경은 0이며 App 밖 변경 없는 성공은 재사용합니다. 배치 전체 `--no-fail-fast` 실행은 아직 아닙니다. 디스크는 539GiB·71%이고 보호 범위 변경이나 빌드 정리를 하지 않았습니다.

## 실제 서버 재시작과 동적 capability 수명

기준은 `9ab20583` 이후이며 제품 코드 변경 없이 기존 검증용 mock과 App 검사만 확장했습니다. `--native-highlights-crash`의 소유 child는 crash 접두사의 요청에서 기존 종료 코드를 반환하고 공용 SDK가 재시작합니다. 문서 revision을 바꾸기 전에 공급자 세대 변경만으로 이전 요청의 취소와 표시 회수를 확인하고, 새 mirror 동기화 뒤 실제 새 세대의 3종 결과만 적용합니다.

`--native-highlights-dynamic`은 현재 앱이 광고하는 completion 동적 등록·해제를 이용해 하이라이트 공급자의 공용 capability revision을 변경합니다. 같은 문서 revision에서 이전 실제 응답의 소비를 거절하고, 등록 해제 후 새 capability revision의 실제 하이라이트와 정상 종료 task 0을 확인합니다. documentHighlight의 동적 등록 지원은 추가하지 않았습니다. 원본 TS의 register/unregister handler는 null로 답하고 앱 초기화도 해당 지원을 광고하지 않습니다. 현재 SDK의 `registration.rs`는 광고하지 않은 등록을 거절합니다. 이를 무시하거나 원본의 무처리를 강제로 재현하지 않았습니다.

실행 로그 접두사는 `/private/tmp/taide-batch23-highlight-lifecycle-`입니다.

- `mock-build.log`: 독립 prototype의 기존 lock 불일치로 `--locked` 빌드가 즉시 exit 101입니다. lock을 갱신하지 않고 App manifest의 기존 `native-lsp-mock` example을 사용했습니다. `app-mock-build.log`는 exit 0·18.53초, 최종 `corrected-mock-build.log`는 exit 0·1.89초입니다.
- `selected.log`와 `phase.log`: 광고하지 않은 documentHighlight 동적 등록을 가정한 fixture로 0통과/1실패·exit 101입니다. 첫 실행은 11.29초, 단계 진단을 추가한 실행은 initial providers 대기·10.87초로 원인을 좁혔습니다. 제품 동작 실패로 합산하거나 제한 시간을 늘려 숨기지 않았습니다.
- `corrected-selected.log`: `cargo test --lib highlight_lifecycle --manifest-path native/taide-native-app/Cargo.toml --locked --offline --features inspection --target-dir experiments/native-shell-spike/target -- --test-threads=1` 직접 실행은 1통과/0실패·exit 0·1.65초입니다. 하나의 검사 안에서 실제 재시작과 동적 등록·해제 두 사례를 각각 검증했습니다. 빌드는 12.39초입니다.
- `fmt-check.log`와 `mock-fmt-check.log`: App fmt와 소유 mock의 edition 2024 format check는 각각 exit 0입니다. 기존 App 상태/child/실제 앱의 서로 다른 25건은 제품 경로와 기존 fixture 모드의 동작이 같아 재사용하며 새 성공에 중복 합산하지 않습니다. 전체 App 테스트 대상을 다시 실행한 것은 아닙니다.

진척 점검에서 실제 JSON은 599행·완료 288/부분 93/미연결 112/미구현 95/제외 10/동결 1이며 대상 588·미완료 300입니다. 현재 미완료 ID의 배정 누락·중복은 0이며 최종 계획은 33입니다. 최근 8커밋 중 제품 변경은 서로 다른 6건이고 문서 변경은 2건입니다. 조회 중 짧게 보인 Cargo/rustc PID는 재조회 시 소멸했지만 소유 작업을 확인하지 못했으므로 이번 작업의 실행이나 전체 세션의 종료성을 증명한 결과로 확대하지 않습니다. 장시간 조사 프로세스·불필요한 재시도 이력은 기존 진척 감사에 보존합니다.

원본 설정 최종 대조·나머지 배치 23 요구사항·배치 전체/실기 게이트는 남아 partial을 유지합니다. Cargo/fmt는 확인한 session 종료 뒤 직렬로 실행했습니다. frozen source·전체 manifest/lock 변경은 0이고 디스크는 537GiB·71%입니다. 보호 범위나 빌드 정리는 건드리지 않았습니다.

## 원본 하이라이트 설정 최종 대조

TS의 `src/features/editor/code-editor.tsx`와 `src/shared/lib/code-editor-settings.ts`는 occurrencesHighlight/selectionHighlight를 생성·갱신 옵션이나 사용자 설정으로 덮어쓰지 않습니다. 설치 Monaco `editorOptions.js`의 occurrencesHighlight 기본값은 singleFile, selectionHighlight는 true입니다. 현재 native의 문서/동일 문서 mirror 표시·선택/본문/peek 소유 경계를 기존 검증과 대조했습니다. 새 토글이나 설정을 만들지 않았으며 이전 상태/child/앱/서버 수명 성공을 다시 실행하지 않았습니다. 해당 하위 대조를 닫지만 batch 전체/성능/실기 게이트로 확대하지 않습니다.

## 고정 줄의 정의 이동과 hover 표시

기준은 `64b2f335` 이후입니다. 설치 Monaco `stickyScrollController.js`의 Ctrl/Meta gesture는 고정 줄의 실제 문자 위치에서 정의를 요청하고 응답에 따라 밑줄을 표시합니다. 일반 클릭·Shift 종료 줄 이동·접기는 별도입니다. native는 sticky의 보조키 클릭을 건너뛰는 동안 본문 정의 gesture가 가려진 다른 줄의 좌표를 소비하고 있었습니다.

`StickyState`의 실제 Row/rect를 기존 definition gesture의 snapshot에 보관했습니다. 고정 줄 영역에서는 해당 줄의 문자 hit만 사용하고 gutter/여백을 본문으로 투과시키지 않습니다. hover 응답의 밑줄도 실제 고정 줄 rect에 그리며 원본처럼 별도 정의 미리보기 tooltip을 추가하지 않습니다. 기존 `editor_locations::Provider::request_at`을 재사용합니다. App Consumer의 해당 메서드는 실제 byte를 LSP position으로 변환하고 기존 Definition/GoTo/Aside 공급·활성화 경로로 전달합니다. UI spy의 선택 보존은 UI가 직접 선택을 바꾸지 않는다는 검증이며 Consumer가 수행하는 실제 이동 선택과 구분합니다.

실행 로그 접두사는 `/private/tmp/taide-batch23-sticky-definition-`입니다.

- `repro.log`: 0통과/1실패·exit 101·0.03초입니다. Mac의 고정 줄 byte 2 클릭이 실제로 `(29, GoTo)`를 요청해 기대 `(2, GoTo)`와 달랐습니다. 컴파일은 성공했고 제품 동작의 실패입니다.
- `fixed.log`: 최초 수정의 같은 1건이 통과·exit 0·0.35초입니다. Mac/Windows/Linux × 정상/읽기 전용 × GoTo/Aside의 12사례에서 source byte·hover 요청·원문과 UI 선택 보존을 확인했습니다. 이후 최종 성공과 중복 합산하지 않습니다.
- `regression.log`: 최신 editor-locations 15건·editor-sticky-scroll 17건, 총 32건 통과·exit 0입니다. 실제 shape의 밑줄 위치/너비·보조키 해제·tooltip 미추가와 gutter/여백/드래그/미지원/스크롤 변화의 요청 거절을 확인했습니다. 기존 본문 정의/peek·고정 줄 클릭/접기/키/정상 읽기 전용/Unicode/가로 스크롤 동작도 통과했습니다.
- `actual-app.log`: 기존 실제 앱 회귀 1건 통과·exit 0·5.82초입니다. UI 변경 후 App을 다시 빌드해 본문/peek·심볼/접기·하이라이트/입력/종료 소비가 유지됨을 확인합니다. 새 고정 줄 클릭의 child E2E를 직접 추가한 검사는 아니며 해당 입력의 직접 UI 공급자 검증과 구분합니다.
- `frozen-host.log`: inspection을 포함한 frozen host 전체 테스트 대상 컴파일은 exit 0·41.25초입니다. `frozen-wasm.log`는 canvas/inspection에 `--tests`를 포함하여 host 검사들이 표준 Instant를 Wasm API에 전달하는 타입 오류로 exit 101입니다. 동결 파일을 수정하거나 이 실패를 통과로 쓰지 않습니다. `frozen-wasm-lib.log`의 실제 canvas/inspection Wasm 라이브러리 compile은 exit 0·0.15초입니다. host 검사의 Wasm 포팅은 동결 예외 범위이며 필요한 경우 사용자 결정과 함께 재검토할 부채입니다.

최종 검토에서 definition hit에 기존 본문의 content rect 경계를 명시적으로 적용했습니다. 고정 줄의 일부가 밀려 표시 영역 밖에 놓여도 그 영역으로 정의 요청이 나가지 않습니다. 바깥 영역 거절 사례를 추가한 `bounds.log`의 위치 검사 15건은 통과·exit 0·0.31초이며 `bounds-fmt-check.log`도 exit 0입니다. 고정 줄 17건·기존 실제 앱 1건의 성공은 적용 경로가 같은 범위에서 재사용합니다. definition-link 모듈은 native-host 전용이므로 마지막 경계 추가는 frozen의 컴파일 경로를 바꾸지 않았습니다.

소유 파일 diff check는 exit 0입니다. 중간 patch 적용은 포맷된 문맥 차이로 한 번 거절돼 실제 문맥을 읽고 적용했으며 제품 실패로 합산하지 않습니다. Cargo/fmt는 live session의 종료를 확인한 뒤 직렬 실행했습니다. 전체 batch `--no-fail-fast` 실행은 아직 아닙니다. 최신 서로 다른 직접 실행 성공은 UI 32·실제 앱 1건입니다. 새 고정 줄 정의 이동 child E2E/OS 입력/시각/IME/접근성·대형 성능은 전체 게이트에서 검토하며 임시 데이터·내부 RawInput만 사용했습니다. frozen source·manifest/lock·의존 그래프 변경은 0, 디스크는 536GiB·71%이고 보호 범위·빌드 정리는 건드리지 않았습니다. editor-16/51은 배치 전체 게이트 전까지 partial을 유지합니다.

## 들여쓰기 자동 감지와 문서별 옵션

기준은 `801b80c9` 이후입니다. 원본 `code-editor.tsx`의 전역 `tabSize`/`insertSpaces`/`detectIndentation`과 모델별 EditorConfig 효과를 설치 Monaco 0.56.0의 `indentationGuesser.js`, `textModel.js`, `model.js`에 대조했습니다. native는 전역값과 EditorConfig만 사용해 실제 2칸 문서를 4칸으로 처리했습니다. 실제 앱 검사에 감지 결과 단언을 넣은 `taide-batch23-indent-repro.log`에서 제품 동작의 실패를 재현했습니다. 0통과/1실패, exit 101이며 검사 4.91초/빌드 59.34초입니다.

Rope의 처음 10,000줄에서 선행 ASCII 공백/탭·내용이 있는 줄만 평가합니다. 정렬 행 제외, 스타일의 동률 기본값 유지, 탭의 기본 표시 폭 보존과 공백 폭 후보/2칸 우선 조건을 원본 규칙에 맞췄습니다. 줄 전체를 문자열로 복사하지 않으며 Unicode의 정렬 열 비교는 UTF-16 위치를 사용합니다. EditorConfig는 감지 후 적용하고 폭은 최소 1로 정규화합니다. 원본의 잘못된 정수 변환 등 숫자 버그를 구현 목표로 삼지 않습니다.

문서가 감지 옵션을 소유하며 같은 전역 들여쓰기 설정/언어/EditorConfig에서는 텍스트 편집마다 다시 추측하지 않습니다. 설정 변화는 다음 본문/peek 사용이나 저장 포맷 참여 시 갱신합니다. 새 문서 스냅샷은 언어/EditorConfig가 바뀐 이전 옵션을 노출하지 않습니다. 감지/옵션 변경은 텍스트 revision·dirty·undo를 변경하지 않으며 mirror는 같은 문서를 사용합니다. 본문/미리보기의 실제 입력과 저장의 FormattingOptions는 같은 옵션을 소비합니다. 저장 참여의 실제 LSP 요청 값까지 새 child 검사로 단언한 것은 아니므로 해당 검증은 명령 통합 단계에 남깁니다.

실행 로그 접두사는 `/private/tmp/taide-batch23-indent-`입니다.

- `core.log`: 최초 컴파일은 Ropey Chars가 표준 DoubleEndedIterator를 제공한다는 가정 때문에 E0599·exit 101로 실패했습니다. 마지막 문자를 RopeSlice의 안전한 문자 위치로 조회하도록 수정했습니다.
- `core-fixed.log`: 들여쓰기 대상 9건 통과·exit 0·0.31초/빌드 2.81초입니다. 새 4건은 설치 Monaco가 생성한 100비교 사례, 감지 줄 수 경계/긴 줄/빈 줄, 편집 시 옵션 보존·전역 감지 설정 변경·undo, EditorConfig/언어 변경·readonly 메타데이터를 확인합니다. 기존 입력/설정 우선순위 5건도 포함하며 사례를 별도 테스트 수로 합산하지 않습니다.
- `ui.log`: 관련 8건 중 기존 7건 통과·새 1건 실패·exit 101입니다. 새 fixture가 readonly 입력에서 기존 계약의 ReadOnly 오류도 없다고 가정했습니다. 기존 큰 파일 입력 검사와 코드의 계약을 대조해 readonly 거절을 정확히 단언하도록 fixture를 수정했습니다. 제품의 readonly 동작을 우회하거나 바꾸지 않았습니다.
- `ui-fixed.log`: 새 UI 1건의 4사례가 통과·exit 0·0.06초/빌드 1.41초입니다. 공백 2칸/탭 표시 폭 8·본문 Tab·mirror 공유·readonly 거절과 내용/revision/dirty 보존·기본 appearance 보존을 확인합니다. 변경 없는 기존 UI 7건은 재사용하며 서로 다른 UI 성공은 8건입니다.
- `peek.log`: 처음 App 검사 컴파일은 기존 symbol-location-host fixture의 Provider 초기화에 새 설정 필드를 빠뜨려 E0063·exit 101로 실패했습니다. 해당 fixture에 같은 전역 설정 변환을 전달했습니다. 단언을 잘못 넣은 순수 State 준비 검사에서는 제거하고 실제 render_preview를 실행한 직후로 옮겼습니다.
- `peek-fixed.log`: 위치/미리보기 상태와 실제 렌더·본문 명령/저장·포커스의 12건 통과·exit 0·1.28초/빌드 37.34초입니다. 실제 preview 대상의 감지 폭 2를 직접 단언했습니다.
- `app-fixed.log`: 최초 제품 실패와 같은 실제 App 검사 1건 통과·exit 0·6.99초/빌드 0.31초입니다. 실제 본문 옵션 2와 기존 심볼·접기·peek·하이라이트/readonly·입력·reveal/종료 소비를 확인합니다.
- `oracle-final.log`: staged diff 검사에서 빈 입력의 TSV 마지막 열이 trailing tab으로 검출돼 입력 hex 열을 중간으로 옮겼습니다. 100사례의 입력/예상값은 열 순서 외에 동일함을 비교했습니다. 변경한 파서의 같은 oracle 1건/100사례가 다시 통과·exit 0·0.01초/빌드 0.84초이며 Editor 전체 성공 수에 중복 합산하지 않습니다. 나머지 8건과 제품/앱/동결 성공은 입력과 해당 실행 경로가 같아 재사용합니다.

- `frozen-host.log`: frozen host의 inspection 전체 테스트 대상 컴파일은 exit 0·21.69초입니다. 실제 테스트 실행을 의미하지 않습니다.
- `frozen-wasm.log`: 실제 canvas/inspection Wasm 라이브러리 컴파일은 exit 0·15.10초입니다. 앞선 고정 줄 단위에서 기록한 host 테스트의 Wasm 시간 타입 오류를 재실행하거나 해결됐다고 쓰지 않습니다.

서로 다른 직접 실행 성공은 Editor 9·UI 8·App 13, 총 30건입니다. Editor/UI/App fmt check 세 개는 직렬로 실행해 exit 0입니다. Tab 폭은 본문/추가 행·minimap·입력/접기·wrap 설정으로 전달하며 wrap cache도 바뀐 폭을 비교하는 기존 경로를 검토했습니다. 탭/공백 변환·명시 detect/reindent·수동 설정 QuickPick과 전체 배치/실기/성능 게이트는 아직이며 editor-30은 partial을 유지합니다. 의존성/manifest/lock/동결 경로 변경은 0이고 디스크는 533GiB·71%입니다. 보호 앱/실제 사용자 데이터/OS 설정과 빌드 정리는 건드리지 않았습니다. 전체 전환율과 잔여 시간은 산정하지 않습니다.

## 들여쓰기 변환과 명시 감지 명령

기준은 `5825d52c` 이후입니다. 설치 Monaco의 `indentation.js`에서 변환은 줄의 선행 ASCII 공백/탭만 바꾸고, 탭 하나를 표시 폭만큼 공백으로 펼치며, 연속 공백의 완전한 폭 단위만 탭으로 바꾸는 것을 확인했습니다. 원본은 주 선택을 추적하고 변환 앞뒤에 undo stop을 두며 문서 옵션은 텍스트 undo에 포함하지 않습니다. 명시 detect는 전역 생성 기본값으로 다시 추측하며 readonly/EditorConfig에서도 모델 옵션만 덮어씁니다. 실제 EditorConfig/언어/전역 들여쓰기 값이 바뀌면 설정을 다시 평가합니다.

Core의 기존 Plan/선택 추적·분리된 편집 단계를 사용합니다. 전체 변환 후 옵션을 바꾸고 용량 초과/readonly 거절에서는 텍스트·옵션·undo를 부분 적용하지 않습니다. 큰 폭의 탭 확장은 결과 용량을 먼저 검사해 거대한 공백 문자열을 만들지 않습니다. 세 action ID를 native-host에서만 실행 경로와 재지정 키에 연결하며 동결 client의 명령 지원 범위를 늘리지 않습니다. 본문/peek의 기존 편집 큐는 문서의 최신 옵션을 사용하고 한 큐의 변환 뒤 줄 명령에도 갱신한 값을 넘깁니다.

실행 로그 접두사는 `/private/tmp/taide-batch23-indent-commands-`입니다.

- `repro.log`: 기존 등록 ID의 native 실행 누락을 새 registry 검사로 재현했습니다. 0통과/1실패·exit 101·0.00초/빌드 7.55초입니다.
- `core.log`: 13건 중 11통과/2실패·exit 101입니다. 실패는 readonly undo API가 false를 반환한다는 fixture 가정이었으며 실제 ReadOnly 오류를 확인했습니다. 오류를 정확히 단언한 뒤 readonly를 해제해 빈 undo stack도 확인합니다. `core-fixed.log`의 해당 2건은 통과·exit 0·0.00초/빌드 1.82초입니다. 변경 없는 11건을 재사용해 서로 다른 Core 성공은 13건입니다. 13변환 사례의 혼합 prefix·공백뿐인 줄·CRLF/Unicode/NBSP·본문 공백 보존, 주 선택/undo/redo·문서 옵션의 undo 비참여, 명시 readonly/EditorConfig override와 설정 변화, 용량/readonly 거절을 확인합니다.
- `input-repro.log`: 초기 UI fixture는 EditorRequest의 참조 closure 수명 추론이 부족해 컴파일에 실패했습니다. 인자 참조 타입을 명시한 `input-repro-fixed.log`에서 실제 제품 실패를 재현했습니다. 변환 뒤 같은 프레임의 Tab이 이전 설정을 써서 기대 두 탭 대신 공백 네 개와 탭이 저장됐습니다. 0통과/1실패·exit 101·0.01초/빌드 2.87초입니다.
- `input-fixed.log`: 일반/문제/고정 줄의 처리된 키맵 뒤 입력 옵션과 표시/wrap 폭을 즉시 갱신했습니다. 새 1건의 변환/감지 두 사례는 통과·exit 0·0.02초/빌드 5.60초입니다.
- `registry.log`: 기존 11건 중 10통과/1실패·exit 101입니다. 원본 메타데이터와 새 명령 게이팅은 통과했으며 실패한 기존 활성 action 집합에는 새 세 명령의 기대값이 빠져 있었습니다. 해당 fixture를 수정해 영향 검사만 재실행합니다.

- `registry-fixed.log`: 실패한 활성 action 집합 검사 1건 통과·exit 0·0.00초/빌드 5.25초입니다. 변경 없는 10건을 재사용하며 registry 성공은 11건입니다.
- `surface.log`: 기존 편집기 표면 전체 83건 통과·exit 0·0.65초입니다. 새 같은 프레임 옵션 갱신 검사도 포함하므로 앞선 1건과 중복 합산하지 않습니다.
- `dispatch.log`: 기존 앱 명령 큐 8건 통과·exit 0·1.27초/빌드 44.26초입니다. 새 검사는 한 큐의 변환 뒤 줄 들여쓰기와 다음 요청의 오래된 context에서도 최신 옵션을 적용하고 세 단계 undo를 확인합니다.
- `peek.log`: 실제 미리보기 12건 중 11통과/1실패·exit 101·1.29초입니다. 추가한 변환/undo 뒤 문서 revision은 3인데 저장 검사에 이전 1을 남겼습니다. `peek-fixed.log`에서도 마지막 접기 검사의 두 번째 이전 기대값을 놓쳐 0통과/1실패·exit 101·1.94초였습니다. 저장/접기 모두 대소문자 변경 직전 revision에 1을 더한 값을 확인하도록 fixture를 수정했습니다. `peek-final.log`는 영향받은 실제 미리보기 1건 통과·exit 0·1.25초/빌드 13.42초입니다. 기존 11건은 재사용하며 미리보기 성공은 12건입니다.
- `app.log`: 실제 앱 본문의 변환/undo·명시 detect와 기존 심볼/접기/peek/하이라이트·정상 종료가 1건 통과·exit 0·6.93초/빌드 0.36초입니다.

이번 변경 단위의 서로 다른 성공은 Core 13·UI 94·App 21, 총 128건입니다. 수동 폭/방식 QuickPick·reindent·실제 LSP 포맷 요청 값과 전체 batch/실기/성능 게이트는 미완료이며 editor-30은 partial을 유지합니다.

`frozen-host.log`는 host inspection 전체 테스트 대상 컴파일 exit 0·14.26초이고 `frozen-wasm.log`는 canvas/inspection Wasm 라이브러리 컴파일 exit 0·5.23초입니다. 테스트 실행이나 기존 Wasm 테스트 시간 타입 오류의 해결을 뜻하지 않습니다. Core/UI/App fmt check 세 개와 소유 경로의 diff check가 직렬로 통과했습니다. 감사 도구는 599행/297근거 경로·288완료/93부분/112미연결/95미구현/제외 10/동결 1을 확인했고 QA/JSON/표 포맷도 통과했습니다. native 의존성/manifest/lock/frozen 변경은 0, 디스크는 532GiB/71%이며 보호 범위와 빌드 정리는 건드리지 않았습니다.

사용자의 진척/반복/끝점 질문에 현재 파일을 재집계해 대상 588·잔여 300, 배정 누락/중복 0·완료된 배정 editor-41/79와 최종 배치 33을 확인했습니다. 48.6%는 이전 요구사항 행의 산술 비율이었고 전체 전환율로 반복 제시한 것은 잘못입니다. 현재 행 비율도 전체 전환율/ETA로 쓰지 않습니다. 실제 프로세스 이름 조회에는 TAIDE Cargo/rustc/rustfmt/test/Bun 조사가 없었으며 최신 커밋 5825d52c·801b80c9·9ab20583의 diff는 서로 다른 제품 변경입니다. 마지막 UI 83건도 종료 결과를 확보했습니다. 현재 실행의 무한 반복 증거는 없지만 전체 세션이나 모든 제품 코드의 종료성을 증명한 결과는 아닙니다. 작은 수정/검증을 지나치게 쪼갠 운영 문제와 이번 fixture의 두 번째 버전 기대값을 놓친 재시도도 구분해 기록합니다. 배치 번호를 추가하지 않고 현재 범위의 기능을 닫으며 같은 상태의 성공 검사는 재사용합니다.

## 전체/선택 재들여쓰기 명령

기준은 `29dc6d0c` 이후입니다. 설치 Monaco 0.56.0의 `editor/contrib/indentation/common/indentation.js`와 browser action, `ProcessedIndentRulesSupport`를 대조했습니다. `editor.action.reindentlines`와 `editor.action.reindentselectedlines`를 기존 native-host의 typed 편집 큐에 연결했습니다. 전체 명령은 첫 줄의 기존 들여쓰기를 기준으로 사용하고 선택 명령은 직전 줄을 기준에 포함하되 끝점이 다음 줄의 첫 열이면 그 줄을 제외합니다. 언어 규칙이 없으면 no-op입니다. 새 엔진/의존성은 추가하지 않았습니다.

Core는 정확한 토큰을 받아 String으로 시작하는 줄을 보존하고 String/Comment/Regex 안의 설정된 괄호를 들여쓰기 평가에서 제외합니다. 처음 무시하는 줄을 건너뛰며 중간 무시하는 줄의 상태 상속, 임시 다음 줄 들여쓰기와 전역 들여쓰기를 구분합니다. 선행 ASCII 공백/탭만 바꾸며 CRLF/Unicode/본문을 보존합니다. 기존 Plan의 중복 편집 제거·모든 선택/mirror 추적·독립 undo를 사용하고 readonly/최종 저장 용량을 검사합니다. 탭은 표시 폭이 아닌 실제 저장 바이트로 용량을 평가하며 큰 공백 할당 전에도 상한을 확인합니다.

앱 명령 큐는 현재 언어를 Core에 전달합니다. 실제 미리보기 Consumer 검사에서는 Ruby 문서의 whole/selected 명령을 재지정 키로 실행하고 본문 내용·undo·후속 detect/save/접기/포커스를 확인했습니다. 별도 실제 앱 생성자의 cold cache를 검증한 검사는 아니며 OS 합성 입력이나 실제 사용자 데이터를 쓰지 않았습니다.

원본 생성 도구는 `docs/utils/2026-10-10-monaco-reindent-oracle.js`, 비교 입력은 `native/taide-native-syntax/tests/fixtures/reindent-reference.json`입니다. 설치 Monaco의 실제 함수를 사용해 23언어 × 12텍스트 × 2스타일 × 5선택, 총 2760사례를 생성했습니다. 원본 basic language 설정에 indentationRules가 없는 언어의 no-op이 포함되며 실제 텍스트가 변한 것은 Ruby/Elixir 81사례입니다. 주입한 줄 토큰의 비교이며 실제 TextMate 엔진/모든 내장 언어 서비스의 검증으로 확대하지 않습니다. 첫 생성은 출력 뒤 Bun 타이머가 남아 session 55830을 종료했고 exit 130입니다. 기존 oracle 패턴처럼 출력 뒤 종료하도록 수정한 `oracle-final.log`는 exit 0입니다.

실행 로그 접두사는 `/private/tmp/taide-batch23-reindent-`입니다.

- `registry-repro.log`: 새 두 명령이 실행 불가능한 제품 실패 0통과/1실패·exit 101입니다. 연결 뒤 `registry.log`는 UI registry 12건 통과·exit 0·0.03초입니다.
- `core.log`: 다중 선택 fixture가 서로 겹쳐 기존 정규화로 primary 0이 된 것을 primary 1로 기대해 실패했습니다. 겹치지 않는 역방향 선택으로 수정한 `core-selection-repro.log`는 1건 통과·exit 0이며 제품 선택 추적 실패로 보고하지 않습니다. `accurate-repro.log`는 정확한 토큰 대신 빠른 Untokenized 토큰을 사용해 문자열 줄과 괄호 들여쓰기를 바꾸는 실제 제품 실패 0통과/1실패·exit 101입니다.
- `core-accurate-final.log`: 정확한 토큰 처리 후 language-typing 17건 통과·exit 0·0.00초입니다. 추가한 무시/임시 들여쓰기 4사례의 검사 `next-ignore.log`는 1건 통과·exit 0입니다. `tab-capacity-repro.log`는 표시 폭이 큰 탭을 저장 용량 초과로 거절하는 실제 제품 실패이며 저장 바이트로 수정한 `tab-capacity-fixed.log`는 1건 통과·exit 0·0.00초/빌드 1.54초입니다. 같은 상태에서 통과한 앞선 검사와 중복 합산하지 않으며 Core의 서로 다른 성공은 19건입니다.
- `oracle.log`는 fixture 어댑터의 필수 trait 메서드 누락과 Regex 타입 이름 오류로 컴파일에 실패했습니다. 수정 뒤 최신 `oracle-final-check.log`는 Syntax 1건에서 2760비교를 모두 통과·exit 0·0.15초입니다. `dispatch.log`의 Selection/SelectionSet import 누락은 컴파일 실패이며 수정한 `dispatch-fixed.log`는 App 큐 9건 통과·exit 0·2.45초입니다. `peek.log`는 App 미리보기 12건 통과·exit 0·0.96초입니다.
- `frozen-host-final.log`는 inspection host 테스트 대상 컴파일 성공·9.57초이고 `frozen-wasm-final.log`는 canvas/inspection Wasm lib 컴파일 성공·2.01초입니다. 마지막 직렬 실행에서 Editor fmt check가 끝났고 앞선 UI/Syntax/App fmt check의 성공은 해당 코드가 그대로라 재사용합니다. 컴파일 성공을 테스트 실행이나 기존 frozen Wasm 테스트의 시간 타입 오류 해결로 보고하지 않습니다.

최종 코드 검토에서 기존 탭 기준줄을 공백으로 정규화하는 경로도 할당 전에 저장 바이트 상한을 확인하도록 같은 인코딩 함수를 사용했습니다. 메모리 할당 실패를 직접 재현한 검사로 보고하지 않습니다. 이 변경 뒤 `capacity-preallocation.log`의 관련 재들여쓰기 5건은 통과·exit 0·0.00초/빌드 3.96초입니다. 변경 없는 language-typing 14건과 Syntax/UI/App 성공을 재사용하며 중복 합산하지 않습니다. 최신 `frozen-host-preallocation.log`·`frozen-wasm-preallocation.log`는 각각 컴파일 성공·9.89초/2.42초이고 `fmt-editor-preallocation.log`도 exit 0입니다. 이 마지막 직렬 session 88768의 종료 결과를 확인했습니다.

이번 단위의 서로 다른 직접 성공은 Core 19·Syntax 1·UI 12·App 21, 총 53건입니다. 2760비교 사례를 테스트 수에 더하지 않습니다. native manifest/lock/동결 경로 변경은 0이며 디스크는 530GiB·72%입니다. 마지막 프로세스 조회에 남은 Cargo 58182는 `development/R-BMS`의 `cargo test --workspace`였고 TAIDE 작업이 아니었습니다. 현재 TAIDE Cargo/rustc/rustfmt/Bun 조사 프로세스는 없으며 재실행 없이 끝난 로그를 회수했습니다. 이 관찰을 전체 세션의 종료성 증명으로 확대하지 않습니다.

실제 앱의 아직 준비되지 않거나 부분 준비된 구문 캐시 처리, tabSize/indentSize 분리·수동 폭/방식 QuickPick·실제 LSP 포맷 요청 값, 나머지 배치 23과 전체 실기/성능/출시 게이트는 미완료입니다. `SyntaxLease::accurate_tokens`는 준비된 캐시만 읽고 Monaco의 `doesLineStartWithString`도 cheap-tokenization 조건을 사용하므로 이 경로는 실제 상태로 대조해야 합니다. editor-30은 partial이며 요구사항 행 판정 288완료/588대상·300미완료를 전체 전환율/잔여 시간으로 환산하지 않습니다. 최종 배치 계획은 33을 유지합니다.

## 재들여쓰기의 미준비/부분 준비 구문

기준은 `ec8504ad` 이후입니다. 설치 Monaco의 `indentationLineProcessor.js:149`는 규칙 평가 전에 해당 줄의 `forceTokenization`을 호출합니다. `doesLineStartWithString`의 cheap 조건만 확인하고 준비된 캐시를 읽는 것으로는 이 계약을 만족하지 않습니다. 실제 앱의 TextMate 워커를 쓰는 준비된 Ruby heredoc과 미준비 문서를 비교해 문자열 내부에 공백을 삽입하는 제품 실패를 재현했습니다.

Core의 LineSyntax에 준비 경계를 추가하고 재들여쓰기의 전체/선택 범위에 필요한 토큰을 한 번 준비합니다. 기본 구현은 기존의 토큰 공급자를 그대로 사용하며 Core/UI에 엔진 의존성을 추가하지 않았습니다. 앱의 SyntaxLease와 PeekTokens는 필요한 캐시가 정확하면 재사용하고, 아니면 현재 설정·문서 스냅샷·줄 범위를 기존 구문 워커에 전달합니다. 워커는 필요한 범위의 앞선 문법 상태부터 같은 ferriki/ferroni 엔진으로 평가하고 TokenKind만 돌려줍니다. 현재 엔진의 설정 전체와 플러그인 문법이 같으면 재사용하고 다르면 요청의 설정/문법으로 별도 엔진을 준비하여 진행 중인 표시 작업의 상태나 세대를 덮어쓰지 않습니다. peek의 준비 포트 캐시도 동일한 설정 전체를 비교합니다.

원본처럼 명령이 토큰 준비를 기다리는 동기 경계입니다. 이미 정확한 토큰은 다시 평가하지 않으며 문서의 토큰화 크기 예외를 보존합니다. 워커 종료/구문 평가 실패는 편집 전에 Refused로 거절하고 readonly는 준비 요청 전 ReadOnly로 거절합니다. 보관한 peek가 같은 문서의 이전 revision이면 현재 스냅샷을 준비하고 문서/언어가 달라졌으면 StaleRevision으로 거절합니다. 준비 포트는 요청 송신자의 Weak만 보관해 peek 캐시가 워커 종료를 막지 않습니다.

최초 peek는 토큰/스타일이 모두 준비되기 전에도 명령 준비 포트를 갖습니다. 스타일이 아직 없으면 표시용 frame은 None이며 기존 기본 텍스트 표시를 사용합니다. 도움말 코드와 위치/hover 소비자는 이 선택적 frame을 사용하도록 맞췄습니다. 표시용 캐시를 정확한 문자열 토큰처럼 취급하는 우회는 하지 않았습니다.

실행 로그 접두사는 `/private/tmp/taide-batch23-reindent-preparation-`입니다.

- `repro.log`: App 실제 구문 워커 검사 0통과/1실패·exit 101·0.30초입니다. 준비된 문자열의 `if {`와 `TEXT`는 보존되지만 미준비 본문에서는 각각 공백 네 개가 붙었습니다.
- `compile.log`: 선택적 frame으로 API를 바꾼 뒤 도움말 코드의 남은 소비자 네 곳에서 E0609로 컴파일에 실패했습니다. 도움말 코드의 map을 and_then으로 맞춘 뒤 `fixed.log`의 새 두 검사는 통과·exit 0·0.55초입니다. 이후 확장한 selected/readonly 사례를 포함한 최종 성공과 중복 합산하지 않습니다.
- `app-syntax.log`: App 구문 관련 19건 통과·exit 0·1.32초입니다. 새 두 검사는 준비 완료 대비 미준비 본문/peek, undo 뒤 부분 캐시와 오래된 peek 스냅샷, 스타일 없는 최초 peek의 준비와 보관한 Weak 포트의 정상 종료를 확인합니다. 기존 문법/테마/플러그인·문서/표시/도움말 소비 회귀도 포함합니다. 최종 readonly/선택 범위 보완 뒤 `readonly-final.log`의 영향 두 검사는 통과·exit 0·0.80초/빌드 14.26초입니다. selected는 시작 범위 전의 heredoc 상태와 끝 첫 열 제외를 보존하며 no-op에서 문서/선택/undo를 바꾸지 않습니다. 워커 종료 후 Refused, readonly의 ReadOnly와 원문/옵션/선택 보존, 언어 교체 후 StaleRevision도 확인했습니다.
- `pipeline.log`: Syntax의 워커·문서/플러그인/표시 우선순위·편집/취소/수렴 관련 23건 통과·exit 0·2.99초입니다. `core.log`는 Core language-typing 19건 통과·exit 0·0.00초입니다. `oracle-compile.log`는 필터가 실제 테스트 이름과 달라 0건 실행이므로 검사 성공으로 세지 않습니다. 올바른 필터의 `oracle.log`는 Syntax 1건/기존 2760비교 통과·exit 0·0.14초입니다.
- `dispatch.log`와 `peek.log`는 App 큐 9·실제 preview Consumer 12건, 총 21건 통과·exit 0·0.78초/0.81초입니다. `frozen-host.log`는 host inspection 테스트 대상 컴파일 성공·9.36초이며 `frozen-wasm.log`는 canvas/inspection Wasm lib 컴파일 성공·3.10초입니다. Core/Syntax/App fmt check와 마지막 App 변경 뒤 fmt check도 직렬 session 종료/exit 0을 확인했습니다. frozen 테스트 실행이나 기존 Wasm 테스트 시간 타입 오류 해결로 확대하지 않습니다.

최종 설정 전환 검사는 `native/taide-native-syntax/tests/token-pipeline/worker.rs`에 있습니다. `configuration-repro.log`는 세대 번호가 같지만 요청 언어 구성이 다른 경우 기존 JSON 엔진을 재사용해 Rust가 UnknownLanguage인 실제 실패 0통과/1실패·exit 101입니다. 세대 번호 대신 설정 전체/플러그인 문법을 비교한 `configuration-fixed.log`는 1건 통과·exit 0·0.01초입니다. 요청의 Rust 문자열 평가, 잘못된 범위 거절, 기존 JSON 표시 작업의 문자열 평가 보존, 포트 보관 중 종료와 종료 후 거절을 확인했습니다. `configuration-app-final.log`의 영향 두 앱 검사는 통과·exit 0·0.82초이며 Source/Core/기존 워커의 변경 없는 성공은 재사용합니다. 마지막 Syntax/App fmt와 session 91737의 exit 0도 확인했습니다. Core/UI/frozen 입력은 마지막 동결 검사 뒤 변경하지 않아 컴파일 성공을 재사용합니다.

서로 다른 성공은 Core 19·Syntax 25·App 40, 총 84건입니다. 새 두 검사의 이전/최종 실행, oracle의 비교 사례, 이전 재들여쓰기 단위의 성공을 중복 합산하지 않습니다. Cargo/fmt는 한 번에 하나만 실행했습니다. native manifest/lock/frozen 경로 변경은 0이며 디스크는 528GiB·72%입니다. 보호 앱·실제 데이터·OS 설정·클립보드/Keychain/Trash를 건드리지 않았습니다.

구문 준비 경계의 확인을 닫지만 tabSize/indentSize 분리·수동 폭/방식 QuickPick·실제 포맷 요청 값·내장 언어 서비스와 나머지 batch 23의 전체 게이트는 남아 editor-30은 partial입니다. 큰 문서/깊은 선택에서의 동기 준비 시간·프레임 성능과 줄 예산 초과 동작의 실제 성능 재현은 배치 31 게이트에 남깁니다. 원본 강제 토큰화와 단위 구문 검사를 전체 언어 서비스/성능/출시 완료로 대신하지 않습니다. 현재 288완료/588대상·미완료 300행, 배치 23/최종 33이며 전체 전환율/잔여 시간은 미산정입니다.

## editor-30 두 폭의 문서 모델과 편집 소비 (ad9750e8 이후)

기존 IndentOptions의 두 필드와 동결 browser-editor의 두 생성 계약을 유지했습니다. DocumentSnapshot의 선택적 indent_size와 ModelIndentOptions가 탭 표시 폭·편집 폭·공백 방식을 분리하며 문서 소유 상태에서 자동 연결과 명시한 숫자 폭을 구분합니다. 새 엔진·의존성·lockfile은 추가하지 않았습니다. 이번 단위는 Core 모델/입력과 실제 UI 소비이며 수동 선택창과 세 명령의 앱 연결은 아직 완료하지 않았습니다.

원본 근거는 설치 Monaco의 common/model.js TextModelResolvedOptions, common/model/textModel.js updateOptions/detectIndentation, common/services/modelService.js _setModelOptionsForModel, common/cursor/cursorCommon.js, cursorDeleteOperations.js, cursorTypeEditOperations.js, common/commands/shiftCommand.js, common/core/misc/indentation.js, contrib/indentation/common/indentation.js입니다. TS의 src/features/editor/code-editor.tsx와 src/shared/lib/editorconfig.ts는 지정한 tabSize/insertSpaces 축만 model.updateOptions에 보냅니다.

- 초기 기본 편집 폭은 tabSize와 함께 바뀝니다. UseSpaces/UseTabs와 명시 Detect는 숫자 편집 폭을 저장하고 이후 DisplaySize가 그 숫자를 바꾸지 않습니다. 탭/공백 변환은 방식만 바꾸며 숫자 폭을 보존합니다.
- 표시·커서 열 측정은 tabSize를 사용하고, 공백 Tab/shift는 indentSize, 탭 shift는 표시 tabSize를 사용합니다. Backspace의 이전 들여쓰기 정지와 Enter/reindent의 정규화는 원본의 별도 규칙을 사용합니다. 줄 이동/삽입과 언어 입력도 최신 문서 모델을 소비합니다.
- 설정 변경의 재감지에서는 감지한 숫자 폭을 저장한 뒤 EditorConfig의 지정 축을 반영합니다. 같은 설정에서 EditorConfig만 바뀌면 미지정 축과 기존 숫자 편집 폭을 보존합니다. 옵션 변경은 readonly에서도 허용되며 텍스트·revision·dirty·undo에 넣지 않고 동일 문서 mirror가 공유합니다.
- 실제 egui 본문 검사에서는 tabSize=8·indentSize=4로 Tab을 입력하여 공백 네 개를 삽입하고 탭 문자 자체는 여덟 칸으로 그리는 것을 확인했습니다. 읽기 전용에서는 옵션/표시를 유지하고 입력을 거절합니다. OS 합성 입력을 사용하지 않았습니다.

실패와 수정 근거:

- /private/tmp/taide-batch23-model-indent-repro.log: 실제 1건 실패·exit 101입니다. 명시한 편집 폭 4와 표시 폭 8에서 Tab의 관찰값이 공백 8개, 기대값이 공백 4개였습니다. 문서 모델의 편집 폭을 입력에 전달하도록 수정했습니다. 최초 잘못된 필터로 실행된 0건은 성공 검사에서 제외했습니다.
- /private/tmp/taide-batch23-model-indent-first.log: snapshot이 Result인데 Option의 ok_or를 적용한 E0599 컴파일 실패입니다. 공개 Result 계약에 맞춰 ?로 수정했습니다.
- /private/tmp/taide-batch23-model-indent-editorconfig-repro.log: 실제 1건 실패·exit 101입니다. 방식만 지정한 EditorConfig 변경이 기존 표시 폭 8을 기본 4로 되돌렸습니다. 같은 설정/언어의 현재 문서 옵션을 미지정 축의 기준으로 사용하도록 수정했습니다.
- /private/tmp/taide-batch23-model-indent-monaco.json: 원본 ShiftCommand와 normalizeIndentation을 직접 실행한 10개 접두부/방식 조합입니다. render 8·indent 4에서 탭·혼합/부분/전체 공백의 shift/unshift/정규화 결과를 확인했습니다. 읽기 전용 Bun 조사는 출력 후 명시 종료·exit 0입니다. 비교 사례를 Rust 테스트 개수로 합산하지 않습니다.

Core의 서로 다른 두 폭 입력 13사례, 자동 연결/명시 감지·변환·readonly/mirror·옵션/undo·EditorConfig/전역 변경과 언어 Enter/reindent를 검사했습니다. 최종 전체 대상/소비 회귀와 동결 컴파일의 결과는 이 절의 후속 검증 기록에 합칩니다. 최초/최종 실행과 중복된 부분 검사를 반복 합산하지 않습니다.

수동 폭 선택창의 1~8·현재/기본·필터/선택/취소·소스 수명, 본문/peek의 세 명령 통합, 실제 LSP 포맷 값과 스니펫의 독립 폭 소비 및 내장 언어 서비스, 큰 문서 성능/배치 전체/실기/출시 게이트는 미완료입니다. editor-30과 b2d를 partial/미완료로 유지하며 이번 Core 모델 검사를 전체 기능 완료로 대신하지 않습니다.

최종 검증 기록 (2026-10-11):

| 범위         | 실행과 결과                                                                                                                                                                        |
| ------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Core 전체    | --all-targets --no-fail-fast 직접 실행, 37대상·285통과/0실패/1ignored, editor-final-all.log                                                                                        |
| 실제 UI 본문 | --features native-host,inspection --test editor_surface -- --test-threads=4, 84통과/0실패, ui-final.log                                                                            |
| 앱 큐/구문   | --features inspection --lib command_dispatch::tests -- --test-threads=1의 최종 9통과; --lib 재들여쓰기는_의 영향 없는 2통과 재사용, dispatch-final.log/app-reindent.log            |
| 구문 소비    | --test language-editing의 38통과 재사용; 마지막 EditorConfig 미지정 축 수정은 고정 옵션/원본 비교 입력에 영향을 주지 않음, syntax.log                                              |
| 동결/형식    | host --tests --features inspection·Wasm --lib --target wasm32-unknown-unknown --features canvas,inspection와 Core/UI fmt check exit 0, frozen-host-final.log/frozen-wasm-final.log |

모든 Cargo 검사는 --locked --offline --target-dir experiments/native-shell-spike/target을 사용했습니다. 로그 공통 접두부는 /private/tmp/taide-batch23-model-indent-입니다. 최종 묶음 session 58179 exit 0과 앞선 session 43392 exit 0을 확인했으며 소유 Cargo/fmt를 직렬 실행했습니다. 서로 다른 성공은 Core 285·UI 84·Syntax 38·App 11, 총 418건이며 이전 Core 284/부분 14·18/동일 UI·앱 큐의 성공을 중복 합산하지 않습니다. 기존 Core tests/display-layout.rs의 수십만 줄 wrap 성능 측정 1건은 ignored로 실제 실행하지 않았고 배치 31에 남깁니다. 새 Core 검사 8건과 UI 1건을 포함하며 비교 조합 수를 테스트 개수로 합산하지 않습니다.

검사 뒤 Core 수정은 없으며 마지막 수정은 EditorConfig의 미지정 축 보존입니다. 그 영향의 Core 전체·UI·App 큐와 동결 컴파일만 다시 실행했고 구문/앱 토큰 준비의 변경 없는 성공은 재사용했습니다. native manifest/lock/frozen 경로 diff는 0, 디스크 531GiB·72%입니다. 배치 23의 전체 변경 크레이트 게이트·실기/성능/출시는 아직 미완료입니다. 행 상태는 288완료/588대상·300미완료, 배치 23/최종 33을 유지하며 전체 전환율과 ETA는 미산정입니다.

문서 형식 검사는 저장소의 docs ignore 때문에 초기 명령이 파일을 검사하지 않은 결과를 제외했습니다. 대상 세 파일만 --ignore-path /dev/null --config prettier.config.js로 명시 포함했고 실제 파일별 포맷 출력과 최종 check exit 0을 확인했습니다. 전체 문서의 재포맷은 적용하지 않았습니다.

## 수동 들여쓰기 선택창과 세 명령 (2026-10-11)

설치된 Monaco indentation.js의 ChangeIndentationSizeAction을 읽었습니다. 1~8의 숫자, 현재/기본/일치 값의 설명, 현재 표시 폭의 초기 선택과 8 상한, readonly 옵션 변경, 열 때 캡처한 모델의 폐기 확인을 기준으로 기존 native 팔레트를 확장했습니다. 숫자 설명은 inline 한 줄이며 항목 아이콘은 그리지 않습니다. command palette에서 이어질 때 같은 모달과 원래 포커스를 유지하므로 원본의 모달 전환 회피용 50ms 타이머를 복제하지 않았습니다. 새로운 화면 디자인이나 명령/기본 키를 추가하지 않았습니다.

원본 영어 메시지와 dev/vs/nls/lang/ko.js·ja.js의 1185~1188을 직접 읽어 기존 locale 카탈로그/namespace에 네 메시지를 추가했습니다. UI 선택 상태는 native-host에서만 존재하며 frozen browser 코드/의존 그래프에는 새 기능을 넣지 않았습니다. 세 기존 명령은 typed IndentationCommand/Run/ShellIntent로 등록하고 사용자 키 재지정과 readonly 실행을 지원합니다.

App은 실제 본문/peek 포커스 ID와 소유 탭을 대조하고 Request에 원래 DocumentId/DocumentKey를 보관합니다. 선택 시 현재 전역 설정을 사용하며 다른 탭·교체 문서로 적용을 넘기지 않습니다. 원래 뷰가 닫혀도 살아 있는 mirror의 문서는 적용하고, 폐기 후 동일 키로 다시 연 문서는 거절합니다. 옵션 변경은 텍스트/revision/dirty/undo를 만들지 않으며, 표시 폭만 변경할 때 숫자 편집 폭을 보존합니다. Escape/배경 취소·일반 팔레트 전환·새 선택은 이전 요청을 회수합니다.

### 재현과 직접 검증

로그 접두사는 /private/tmp/taide-batch23-indentation-picker-입니다.

- registry-repro.log: 새 명령 미지원의 실제 실패 1건·exit 101입니다. 첫 indentUsingTabs의 keymap_run이 None이었으며 세 명령을 native 경로에 연결했습니다.
- ui-lib.log: 신규 선택창 4건과 새 등록 검사 등 130건 통과/기존 기대 집합 1건 실패·exit 101입니다. 기존 모든 editor action 집합에 세 명령의 readonly 지원을 추가해 기대 계약을 갱신했습니다. 부분 성공을 전체 성공으로 보고하지 않습니다.
- app-request.log: 요청 수명 2건 통과/새 폐기 fixture 1건 실패·exit 101입니다. 내용이 있는 untitled는 UnsavedChanges이므로 명시 discard_document로 폐기해 재개 모델 거절을 검사합니다. 제품의 폐기 게이트를 약화하지 않았습니다.
- ui-all.log: 변경 후 --all-targets --no-fail-fast·native-host,inspection·test-threads=4, 22대상/435건 통과/0실패/0ignored·exit 0입니다. 목록·현재/기본/일치·상한·숫자 필터/빈 결과·키/마우스·포커스·IME/취소·일반 모드 전환·readonly/등록/재지정·기존 UI 회귀를 포함합니다. 앞선 부분 성공을 중복 합산하지 않습니다.

App 전체 검증과 실패 영향 검사를 마쳤으며 본문의 세 명령, 실제 순수 peek의 F9/F10/F11 재지정, readonly 옵션 변경·원본 텍스트/revision/dirty와 본문 격리·포커스 복원·수명 및 전체 회귀를 확인했습니다. 실제 Trash를 사용하는 기존 3검사는 보호 지시에 따라 제외했습니다. 전체 전환율/잔여 시간은 미산정이며 editor-30과 배치 23은 아직 partial/진행입니다.

App 전체 실행 중 실제 startup 본문/peek/readonly 확장 검사와 Request 3건은 통과했습니다. lib의 기존 preview 직렬 명령 검사 1건에서 reindent의 관찰값이 공백 2칸·기대값 4칸으로 실패했습니다. 해당 준비 코드는 Detect로 숫자 편집 폭 2를 만든 뒤 legacy override_indentation으로 표시 폭/방식만 변경했습니다. c85d1ebe의 독립 폭 계약에 맞게 새 set_indentation(UseSpaces(default width))으로 두 폭을 설정해 기존 4칸 변화/undo 단언을 유지합니다. 제품 모델의 숫자 폭 보존을 제거하거나 기대값을 2칸으로 낮추지 않습니다. 전체 명령의 소유 세션이 종료한 뒤 이 검사만 재실행하며 전체 App를 반복하지 않습니다.

### 최종 결과와 잔여 범위

app-all.log는 --all-targets --no-fail-fast·inspection·test-threads=1로 67대상/802통과/1실패/0ignored·보호 Trash 3제외·exit 101입니다. 제품 코드는 그대로 두고 기존 preview 준비만 올바른 두 폭 API로 수정했습니다. app-preview-final.log에서 정확한 실패 검사 1건이 2.40초/exit 0으로 통과했습니다. 최종 App 서로 다른 803건/미해결 실패 0이며 전체 App 명령을 반복 실행하거나 최초 exit 101을 exit 0으로 바꿔 보고하지 않습니다. 실제 startup의 본문 세 명령·순수 peek 세 키·readonly 옵션 변경·텍스트/revision/dirty/본문 격리·포커스 복원과 정상 종료, Request의 mirror/동일 키 재개/현재 설정 3건은 이 전체에 포함됩니다.

locale-all.log는 --package taide-locale --all-targets --no-fail-fast·test-threads=4로 20건/0실패·exit 0입니다. UI 435·App 803·locale 20의 최종 서로 다른 성공은 1258건입니다. 초기 실패/중간 성공/원본 비교 사례를 중복 합산하지 않습니다. Core는 수정하지 않았으며 c85d1ebe의 전체 285통과/기존 성능 1ignored 근거를 재사용합니다. 기존 성능/OS 입력/시각/IME/접근성·대형 성능/soak/출시와 나머지 배치 23은 완료로 올리지 않습니다.

frozen-host.log의 --tests/inspection은 9.30초, frozen-wasm.log의 --lib/wasm32-unknown-unknown/canvas,inspection은 1.86초로 각각 exit 0입니다. UI/App/locale fmt check·소유 diff check exit 0, frozen/manifest/lock 변경 0입니다. 소유 Cargo/fmt는 각각 앞 세션의 실제 종료 뒤에만 실행했으며 최초 전체 중에도 Rust/포함 리소스를 수정하지 않았습니다. 디스크는 529GiB/72%이고 보호 앱/실제 데이터/OS 설정/클립보드/Keychain/Trash를 건드리지 않았으며 빌드 정리를 하지 않았습니다.

b2d의 문서 모델/수동 선택창 단위를 닫고 editor-30은 실제 포맷/스니펫 독립 폭·내장 서비스/큰 tier 성능·배치 전체/실기/출시 게이트가 남아 partial을 유지합니다. 최신 요구사항 집계는 588대상/288완료/300미완료, 현재 배치 23/최종 33이며 전체 전환율과 잔여 시간은 미산정입니다.

## 포맷과 스니펫의 현재 편집 폭 (2026-10-11)

기준은 3589990b 이후입니다. 설치 Monaco textModel.js:452의 getFormattingOptions는 indentSize를 tabSize로 전달하고 :491의 normalizeIndentation도 indentSize를 사용합니다. snippetSession.js:338 이후의 최초 삽입과 :96의 활성 placeholder 변환은 현재 모델의 normalizeIndentation을 소비합니다. 변환 결과의 첫 줄은 그대로 두고 두 번째 줄부터 현재 편집 폭으로 정규화합니다. TS LSP formatting 어댑터는 이 옵션을 그대로 서버에 전달합니다.

Core ModelIndentOptions::formatting_options는 문서의 편집 폭/방식을 기존 IndentOptions로 유도합니다. App의 실제 스니펫 미리보기·수락과 저장 포맷 DTO가 이를 사용하며, Core 활성 스니펫 변환은 최초 수락 시 보관한 값 대신 현재 문서 옵션을 읽습니다. 미설정 기존 fixture는 세션의 기존 fallback을 유지합니다. 미리보기 캐시도 현재 편집 폭/방식 변경으로 무효화하며 표시 폭·문서/revision/dirty·소유/readonly·오래된 요청의 기존 경계를 보존합니다.

로그 접두사는 /private/tmp/taide-batch23-format-indent-입니다.

- core-repro.log는 1실패·exit 101입니다. 수락 후 편집 폭을 4에서 2로 바꿔도 변환 요청에 4가 전달되었습니다. 최초 테스트의 단일 첫 줄 정규화 기대는 원본 계약에 맞춰 첫 줄 보존/둘째 줄 정규화로 수정했습니다.
- consumer-repro.log는 테스트에서 다른 하위 모듈의 private Preview.ghosts를 읽어 E0616으로 컴파일에 실패했습니다. 제품 필드의 가시성을 넓히지 않고 실제 egui에 그린 문자열을 확인하도록 테스트를 수정했습니다.
- mock-build.log는 기존 실제 child에 FormattingOptions echo 모드만 추가한 빌드의 exit 0입니다. app-repro.log는 세 검사 모두 제품 실패·exit 101입니다. 편집 폭 4/표시 폭 8에서 저장 결과는 formatted:8:true:input!, 미리보기는 공백 8개 뒤 body, 수락도 console 다음 줄에 공백 8개를 만들었습니다.
- core-all.log는 --all-targets --no-fail-fast를 직접 1회 실행해 37대상/286통과/0실패/기존 성능 1ignored·exit 0입니다. 새 활성 세션 검사에서 현재 편집 폭 2·둘째 줄 정규화와 표시 폭 8 보존을 확인했습니다. 최초 실패/중간 결과를 중복 합산하지 않습니다.

신규 App 검사는 실제 NativeApplication의 request_tab_save에서 LspBridge/실제 child/파일 저장까지 전달되는 4/공백과 변경 후 2/탭을 확인합니다. 실제 Consumer는 본문·별도 owner가 있는 peek의 미리보기 재평가, 수락·undo·원래 owner 보존을 확인합니다. 외부 언어 서비스의 새 기능·엔진·의존성이나 원본 버그를 추가하지 않았습니다.

### 전체 실행과 최종 판정

app-all.log는 --all-targets --no-fail-fast·inspection·test-threads=1·보호 Trash 3제외로 직접 1회 실행했습니다. 67대상/780통과/26실패/0ignored·exit 101이며 새 저장/미리보기/수락 3건은 모두 통과했습니다. 기존 로컬 서버/preview listener의 Operation not permitted·PermissionDenied와 그 시작 실패의 후속 단언, macOS ImageIO의 decoded pixels materialize 실패 1건, 실제 임시 OS watcher의 fs=0/git=0 대기 실패 1건을 관찰했습니다. macOS 이미지와 watcher의 원인을 포트 권한 오류로 단정하지 않습니다.

소유 전체 세션의 종료 뒤 이미 컴파일한 8개 테스트 실행 파일에서 실패 이름 26개만 --exact·test-threads=1로 권한이 허용된 환경에서 실행했습니다. app-permission-retry.log와 retry-run.log에서 26통과/0실패·exit 0을 확인했습니다. 성공 780건은 반복하지 않았으며 제품 코드를 변경하지 않았습니다. 최종 서로 다른 App 성공은 806건이고 최초 전체의 exit 101은 그대로 기록합니다. 이미지·watcher의 재검사 성공은 원인 해결의 증명이 아니며 배치 31의 실행 환경/soak 부채로 남깁니다. Core 286·App 806의 최신 서로 다른 성공은 1092건입니다. 변경하지 않은 UI 435·locale 20의 이전 전체 성공을 재사용하며 이 숫자를 원자 기능 수나 전체 전환율로 쓰지 않습니다.

공유 mock은 App example 빌드와 실제 child를 사용하는 App 전체에서 검증했습니다. 추가로 시도한 독립 experiments/lsp-coordinator-spike의 --locked --offline 전체 실행은 기존 Cargo.lock 불일치로 컴파일 전 exit 101입니다. 잠금 파일을 바꾸거나 --locked를 제거하지 않았으며 실행하지 않은 테스트를 통과로 보고하지 않습니다. Core/App fmt check는 exit 0입니다. 독립 실험의 fmt check는 기존 2021/2024 공유 mock 스타일 차이와 수정하지 않은 native-session.rs 형식 차이로 exit 1이며 해당 파일을 재포맷하지 않습니다. 두 독립 실험 검사 부채도 배치 31에 보존합니다.

frozen-host.log의 host --tests/inspection은 10.09초, frozen-wasm.log의 --lib/wasm32-unknown-unknown/canvas,inspection은 3.26초이며 각각 exit 0입니다. 소유 제품 diff check exit 0, frozen/manifest/lock 변경 0, 디스크 여유 525GiB/72%입니다. Cargo/fmt는 이전 소유 세션이 실제 종료한 뒤 직렬 실행했으며 보호 앱/실제 데이터/OS 설정/클립보드/Keychain/Trash와 빌드 산출물을 변경·정리하지 않았습니다.

editor-30의 원본 요구사항은 tabSize·insertSpaces·EditorConfig와 감지/변환/indent/outdent입니다. 기존 전체/선택 재들여쓰기·수동 선택창·문서 설정/수명·실제 본문/peek 검증에 현재 포맷/스니펫 소비를 더해 기능 판정을 complete로 갱신합니다. formatOnType/formatOnPaste(editor-32), 내장 언어 서비스(editor-57), 큰 tier(editor-19), 전체 실기/성능/출시는 별도 요구사항/게이트로 계속 미완료입니다. 배치 23 전체를 완료로 올리지 않습니다. 최신 요구사항은 588대상/289완료/299미완료이며 배치 23/최종 33 계획과 전체 전환율·잔여 시간 미산정을 유지합니다.

기능표 생성의 최초 실행은 experiments/ 경로를 직접 source 근거에 넣어 기존 경로 검증에서 Invalid evidence path로 실패했습니다. 검증기를 넓히지 않고 공유 mock 근거는 이 QA의 실제 빌드/child 결과로 보존하며 native Consumer/세션/실제 앱 테스트 경로로 연결했습니다. 같은 감사 도구의 다음 실행은 599행/309근거 경로·정합성 exit 0입니다.

## 문서·선택 포맷의 외부 요청과 실제 소비 (2026-10-11)

기준은 19230aca입니다. TS lsp/adapters/formatting.ts의 문서/범위 어댑터와 설치 Monaco의 formatActions.js·format.js·formattingEdit.js·editorWebWorker.js, 기존 typed Formatting/RangeFormatting과 App LspBridge/EditorStore를 대조했습니다. 문서는 문서 공급자를 우선하고 없을 때 범위 공급자에 전체 범위를 요청하며, 선택은 범위 공급자만 사용합니다. 빈 선택은 현재 줄 전체로, 접하거나 겹친 선택은 합친 범위로 바꿉니다. 서로 다른 요청의 응답 편집이 겹치면 두 요청 범위를 합쳐 재요청하며 그룹 수가 반드시 하나씩 줄어듭니다. 성공/null/취소/실패 경계와 현재 세대/capability revision·문서 key/revision/언어/tier·뷰/owner·주 커서·IME·현재 편집 폭/방식을 확인합니다.

Core formatting.rs는 UTF-16 범위의 최소 편집을 계산하고 EOL을 모델 방식으로 정규화합니다. 동일 결과는 편집/dirty/undo를 만들지 않으며, 유효하지 않은 UTF-16과 겹친 편집은 원자적으로 거절합니다. Unicode scalar와 CRLF 경계를 보존하는 유한 차이 계산은 최대 100,000 UTF-16 단위·1,000,000 단계에서 raw 편집으로 대체합니다. 31개 문자열의 961쌍 재구성과 상한 대체는 각각 일반 검사 한 건으로 계산합니다. 원본의 내부 수치나 버그 재현을 목표로 삼지 않습니다.

기존 두 명령에 Mac/Windows Shift+Alt+F·Linux Ctrl+Shift+I와 공통 Cmd/Ctrl+K, Cmd/Ctrl+F를 연결하고 재지정도 유지했습니다. Linux의 Ctrl 조합은 기존 keymap 규칙에 따라 IME 중에도 전달되지만 실제 요청은 조합 상태에서 시작하지 않습니다. 팔레트가 열리면 본문 입력이 비활성화되어 peek의 focus ID 기록이 두 프레임 뒤 사라지는 제품 실패를 재현했습니다. 해당 원래 ID를 팔레트가 열린 동안 유지하고 닫기 전에 포맷의 source/owner를 캡처해 실제 요청에 전달합니다. 읽기 전용 본문에서는 포맷을 제외하고, 읽기 전용 본문이 소유한 쓰기 가능한 peek에는 허용합니다. 본문 저장 포맷도 같은 최소 편집 계산을 사용하며 저장 참여자의 순서는 바꾸지 않습니다.

로그 접두사는 /private/tmp/taide-batch23-editor-format-입니다.

- registry-repro.log는 새 두 명령의 미지원 실패 1건·exit 101입니다. core.log는 4통과/1실패였으며 이름 끝에 삽입한 공백의 after affinity를 무시한 기대값을 실제 이름 안의 선택으로 정정했습니다. core-retry.log는 해당 1건 통과입니다.
- state.log는 Root의 Context 참조 누락으로 컴파일에 실패했습니다. product.log는 테스트의 disconnect 반환값과 IndentOptions 기본값 API를 잘못 사용해 컴파일에 실패했습니다. product-retry.log는 상태 4통과/실제 앱 1실패/child 1실패입니다. TS use-editor-file-persistence.ts:254의 모든 편집에서 dirty를 유지하는 계약에 맞춰 undo 기대값을 정정했습니다. child.log의 초기화 대기는 테스트가 재도색만 오는 공급자 상태 변화를 다시 확인하지 않아 발생했습니다. 조건을 직접 다시 확인하도록 고친 child-final.log는 문서/선택·범위 대체·겹침 재요청과 실제 취소/늦은 응답 거절의 2건 통과·exit 0입니다.
- app-final.log와 app-diagnostic.log는 peek 팔레트가 원래 대상을 잃어 읽기 전용 본문으로 판단한 실제 실패입니다. 수정한 app-pass.log는 실제 본문 기본 키·선택 chord·readonly owner의 peek 팔레트·현재 4/공백→2/탭과 표시 8·undo·문서/디스크 저장 분리까지 1건 통과·exit 0입니다. 정의 공급자의 탐색 자체는 새로 검증하지 않았으며 기존 peek 응답 모델을 fixture로 사용했습니다.
- core-all.log는 전체 38대상/291통과/0실패/기존 성능 1ignored·exit 0입니다. ui-all.log는 전체 22대상/435통과/2실패·exit 101입니다. 기존 지원 목록과 IME 플랫폼 기대값을 정정한 ui-registry-retry.log는 13통과/1실패이며 readonly guard가 generic Native 분기로 빠지는 제품 누락을 추가로 확인했습니다. guard 수정 뒤 ui-registry-final.log와 ui-key-final.log의 해당 검사 1건씩이 통과해 최종 서로 다른 UI 437건입니다. 전체 성공 435건을 다시 실행하지 않았고 최초 exit 101을 보존합니다.
- app-all.log는 기존 통합 검사의 새 Reply 분기 누락으로 컴파일 전 exit 101입니다. 분기를 보완한 app-all-final.log는 전체 67대상/811통과/1실패·exit 101이며 보호 Trash 3건을 제외했습니다. 실패는 command-dispatch가 새 포맷 명령을 아직 미지원으로 기대한 검사입니다. 기대값을 수정한 app-actions-retry.log의 해당 1건 통과로 최종 서로 다른 App 812건입니다. 앞선 부분/중간/원본 비교 사례를 중복 합산하지 않으며 Core/UI/App의 최신 서로 다른 성공은 1540건입니다.

### 남은 판정과 검증 부채

editor-49는 수동 외부 포맷 소비가 추가됐지만 내장 언어 서비스의 포맷 fallback(editor-57)이 남아 partial을 유지합니다. editor-32의 자동 입력/붙여넣기는 아직 연결하지 않았으며 missing입니다. 이번 일반 최소 편집·기본 선택/scroll/undo 성공만으로 다음 조건까지 검증했다고 확대하지 않습니다.

- raw 전체 교체로 대체할 때 원본 replace/replaceMove의 마커 affinity는 별도 실제 Monaco 비교를 실행하지 않았습니다. 현재 상한 검사는 텍스트 재구성만 확인합니다. 전체 교체에서 선택 위치가 달라질 위험을 c2s에서 원본 편집과 native 표면으로 확인합니다.
- 포맷이 커서 위의 줄 수를 바꿀 때 상대 세로 위치와 wrap/접기/scroll을 함께 보존하는지는 아직 실제 표면에서 비교하지 않았습니다. Core의 저장 scroll 보존과 nearest reveal을 해당 UI 계약의 완료로 간주하지 않으며 c2s에서 직접 확인합니다.

이 두 대조를 닫기 전에는 PROCESS c2 전체를 완료로 올리지 않습니다. 현재 일반 요청/명령·실제 소비의 검증한 c2a 단위만 저장합니다. 기존 독립 실험 lock/fmt·이미지/OS watcher 원인 미확정·큰 tier/성능/실기/출시 부채는 유지합니다. 요구사항은 588대상/289완료/299미완료, 현재 배치 23/최종 33이며 전체 전환율·잔여 시간은 미산정입니다.

frozen-host.log의 host --tests/inspection은 17.00초, frozen-wasm.log의 Wasm --lib/canvas,inspection은 6.07초이며 각각 exit 0입니다. Core/UI/App fmt check exit 0, frozen/manifest/lock 변경 0, 디스크 여유 518GiB/72%를 확인했습니다. 소유 Cargo/fmt는 이전 세션의 실제 종료 뒤에만 직렬로 실행했으며 실행 중 Rust/포함 리소스를 수정하지 않았습니다. 보호 앱/실제 데이터/OS 설정/클립보드/Keychain/Trash를 건드리거나 빌드 산출물을 정리하지 않았습니다. 독립 실험 lock/fmt 실패는 입력이 같은 기존 결과를 재사용합니다.

## 포맷 후 선택과 상대 커서 세로 위치 (2026-10-11)

기준은 34bcf4a5입니다. 설치 Monaco formattingEdit.js의 전체 범위 replace와 부분 범위 replaceMove, oneCursor.js의 tracked selection, intervalTree.js의 nodeAcceptEdit와 stableEditorScroll.js의 상대 위치 복원을 직접 읽었습니다. docs/utils/2026-10-11-monaco-formatting-markers-oracle.js는 실제 FormattingEdit.execute와 IntervalNode를 실행해 동일 길이/축소/확장/Unicode 접두부의 네 결과를 TXT fixture에 저장합니다. 이를 native 다중 선택·mirror·undo/redo와 비교합니다. 이 조사는 계측한 editor 객체와 scroll 0을 사용하므로 브라우저 위젯/픽셀이나 상대 scroll의 브라우저 실기를 검사한 결과가 아닙니다.

Core는 최소 편집이 raw 전체 범위 교체로 대체될 때에만 기존 절대 UTF-16 선택 위치를 보존하고 원래 EOF는 새 EOF로 옮깁니다. 새 문서의 Unicode scalar와 CRLF 중간 경계를 보정합니다. 다른 편집/LSP의 기존 선택 매핑은 유지합니다. 공통 formatting::apply_edits를 외부 수동 요청과 앱 저장 참여자에 연결하며 identity/revision/readonly/유효 범위·원자성·undo 경계를 보존합니다.

포맷 전후 revision/주 커서/scroll을 담은 요청은 한 번 소비하고 사용자의 커서/scroll/편집/undo 변화로 만료합니다. native-host UI는 마지막으로 실제 그린 caret의 display row와 wrap/접기/view zone을 포함한 row top을 사용해 상대 세로 위치를 복원합니다. 최상단 scroll 0과 진행 중 smooth scroll은 원본 계약에 따라 복원하지 않습니다. 일반·wrap·wrap+접기 각각 최상단/스크롤·줄 추가/삭제를 실제 NativeEditor/egui 좌표로 검사했으며 OS 합성 입력을 사용하지 않았습니다.

로그 접두사는 /private/tmp/taide-batch23-format-scroll-입니다.

- marker-repro.log는 테스트의 serde/serde_json 사용으로 컴파일 전 실패입니다. Core에 의존성을 추가하지 않고 TXT fixture로 변경했습니다. marker-repro-final.log는 실제 1실패로 모든 선택이 새 EOF로 모이는 문제를 재현했습니다.
- ui-repro.log는 검사 fixture의 TexturesDelta 미회수 실패입니다. 이를 고친 ui-repro-final.log는 실제 커서 Y가 60에서 80으로 이동한 제품 실패입니다. 기존 단순 reveal을 포맷 전용 상대 복원으로 바꿨습니다.
- crlf-repro.log의 untitled 모델은 LF 메타데이터 때문에 실제 CRLF를 검증하지 않았습니다. OpenedFile payload로 바꾼 crlf-file-repro.log는 실제 중간 커서 77/유효 기대 76의 1실패입니다. 중간 경계 보정 후 core-final.log의 관련 포맷 8건이 통과했습니다. 파일 payload는 가상 경로를 사용하며 실제 파일을 읽거나 쓰지 않습니다.

### 최종 직접 검사

모든 Cargo 검사는 --locked --offline --target-dir experiments/native-shell-spike/target이며 전체 검사는 --all-targets --no-fail-fast로 직접 1회 실행했습니다.

| 범위           | 결과                                                                                                                                                                                                                                                                                                                               |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Core 전체/영향 | core-all.log: 38대상/293통과/0실패/기존 성능 1ignored·exit 0. 실제 CRLF 수정 뒤 core-final.log: 포맷 8통과·exit 0. 중복을 제외한 서로 다른 성공은 294건이며 전체 294건을 다시 실행한 결과가 아닙니다.                                                                                                                              |
| UI 전체        | ui-all.log: native-host,inspection·test-threads=4·22대상/438통과/0실패/0ignored·exit 0.                                                                                                                                                                                                                                            |
| App 전체       | app-all.log: inspection·test-threads=1·67대상/811통과/1실패/0ignored·exit 101. 실제 본문/peek/저장과 formatter child는 통과했고 보호 Trash 3건을 제외했습니다. 실패는 프로젝트 열기 5초 상한입니다.                                                                                                                                |
| App 영향       | app-hook-retry.log: Cargo의 동일 exact 검사 0통과/1실패·5.46초·exit 101. 같은 빌드 실행 파일을 직접 실행한 app-hook-probe.log는 저장소 cwd에서 1통과/3.81초, app-hook-stack-probe.log는 앱 크레이트 cwd·스택 수집을 포함해 1통과/3.23초, 각각 exit 0입니다. 최초/Cargo 단독 실패를 없애거나 전체 App exit 0으로 보고하지 않습니다. |
| 동결/형식/경계 | frozen-host.log의 --tests/inspection 13.00초·frozen-wasm.log의 --lib/wasm32-unknown-unknown/canvas,inspection 3.66초는 각각 exit 0입니다. Core/UI/App fmt check exit 0·frozen/manifest/lock 변경 0, 디스크 512GiB/73%입니다.                                                                                                       |

서로 다른 검사 성공은 Core 294·UI 438·App 812입니다. 두 직접 probe의 동일 성공이나 중간/원본 비교 사례는 중복 합산하지 않습니다. App 전체와 Cargo 단독 실패의 재현 조건을 유지하며 이 집계를 모든 환경에서 전체 검사가 통과했다는 뜻으로 사용하지 않습니다. 기존 Core 성능 ignored 1건과 보호 Trash 3건은 실행하지 않았습니다. 소유 Cargo/fmt는 직렬이며 이전 실행이 실제 종료한 뒤에만 Rust/포함 리소스를 수정했습니다.

### 프로젝트 열기 지연의 관찰과 남은 게이트

실패 지점은 projects-tests.rs의 projects.open을 감싼 5초 timeout이며 이후 hook release 대기 지점이 아닙니다. fixture의 hook은 mutation guard를 해제한 뒤 release를 기다립니다. project_open도 attach 앞에서 guard를 해제하고, project build는 기존 blocking worker의 파일 감시 준비를 기다립니다.

성공한 3.23초 검사 중 /private/tmp/taide-batch23-format-scroll-project-open.sample의 673표본에서 build_files→watcher.rs:304의 start_watch→notify FsEventWatcher.run→runloop 준비 수신 대기를 확인했습니다. 다른 FSEvents 스레드는 FSEventStreamStart→register_with_server→f2d_register_rpc→mach_msg2_trap에 있었습니다. 해당 표본은 OS 등록 RPC 대기를 보여주며 CPU 반복이나 hook guard 상호 대기를 관찰한 결과가 아닙니다. 두 5초 실패 시점의 스택과 지연 변동의 근본 원인은 확인하지 않았습니다. 직접 검사의 성공을 원인 해결로 확대하지 않습니다.

시간 상한 확대·검사 생략·OS 설정 변경·프로세스 재시작으로 우회하지 않았습니다. 같은 가정의 반복 검사는 종료하고 실제 감시 등록/실행 환경·soak 신뢰성 위험을 기존 이미지/watcher와 독립 실험 lock/fmt 부채와 함께 배치 31에 유지합니다. 보호 앱/실제 데이터/클립보드/Keychain/Trash를 건드리지 않았으며 빌드 정리를 하지 않았습니다.

외부 포맷 공통 경로 c2와 선택/상대 위치 c2s를 닫습니다. editor-49는 내장 공급자(editor-57)가 남아 partial이고 editor-32의 입력/붙여넣기는 missing, 배치 23 전체와 실기/성능/출시는 미완료입니다. 요구사항 588대상/289완료/299미완료·배치 23/최종 33을 유지하며 전체 전환율·잔여 시간은 미산정입니다.

## 입력·붙여넣기 포맷의 원본 경계 (2026-10-11, 진행 중)

기준은 965401bd입니다. TS code-editor.tsx:299의 옵션 전달과 editor-pane.tsx:425~~426의 현재 설정, untitled-pane.tsx:202~~203/app-file-pane.tsx:167~168의 false, LSP formatting.ts:47 이후의 first/more 트리거·UTF-16 position/options와 설치 Monaco formatActions.js/format.js/codeEditorWidget.js를 직접 읽었습니다.

입력은 실제 onDidType의 마지막 문자, 최신 onType 공급자, 단일 빈 선택과 현재 모델의 편집 폭/방식을 사용합니다. 일반 텍스트/Enter의 keyboard type 이벤트와 IME의 compositionType 경로를 구분하며 compositionType 자체는 onDidType을 발행하지 않습니다. 자동 닫기 overtype처럼 문서 revision이 같아도 실제 type 이벤트와 변경된 커서가 있으면 요청 후보입니다. 원본의 UTF-16 반쪽 단위가 만드는 잘못된 문자열을 재현하지 않고 native의 유효한 마지막 Unicode 문자로 전달합니다.

붙여넣기는 범위 공급자만 사용하며 document formatter로 대체하지 않습니다. 실제 keyboard paste 전 선택 시작부터 후 선택 시작까지의 UTF-16 범위와 단일 커서를 사용합니다. Peek의 EmbeddedCodeEditorWidget은 부모 getRawOptions를 상속하고 referencesWidget은 자동 포맷 옵션을 덮어쓰지 않습니다. native 앱은 본문 파일 탭과 그 peek만 연결하며 untitled/app-file owner의 자동 요청은 시작하지 않습니다. 두 자동 경로 모두 FormattingEdit.execute의 addUndoStops=true를 사용하므로 공통 포맷의 앞/뒤 undo stop을 유지합니다.

현재 UI의 실제 Text/Enter/Paste를 출력하는 검사 1건은 실패→수정→통과했습니다. /private/tmp/taide-batch23-auto-format-input-repro.log의 첫 필터는 0건 실행이므로 성공 근거에서 제외합니다. input-repro-final.log의 실제 1건은 typed 입력 출력이 0개로 실패했고 input-related.log의 해당 1건은 5.23초 컴파일 뒤 통과했습니다. SDK의 onType 옵션·앱/peek 요청 연결은 구현 후 --tests/inspection 컴파일을 진행 중이며 실제 서버/앱과 취소/undo·변경 크레이트 전체·동결 컴파일은 아직 실행하지 않았습니다. editor-32와 c3/c3a/c3b는 미완료를 유지합니다.

후속 구현은 SDK SessionSnapshot에 문서별 정적/동적 onType 옵션과 기존 캐시/회수 경계를 더하고 typed OnTypeFormatting을 최신 공급자의 트리거에서만 요청합니다. Paste는 실제 범위의 RangeFormatting만 사용합니다. native-host 출력은 소유 파일 본문/peek의 로컬 명령으로 전달하며 현재 문서/revision·단일 커서/IME·프로젝트/owner·설정 변경과 종료를 확인합니다. 앱의 기존 manual/save와 같은 최소 편집·선택/상대 scroll·앞/뒤 undo stop 경계를 재사용합니다. 새 엔진/의존성/기본 키나 원본 버그를 추가하지 않았습니다.

auto-format-compile.log의 --tests/inspection은 기존 lsp-status/recovery의 SessionSnapshot 준비 두 곳의 새 필드 누락으로 컴파일 전 exit 101입니다. 두 준비만 기본 빈 옵션으로 갱신했습니다. state-repro.log는 6건 중 5통과/1실패·exit 101이며 자동 포맷의 현재 위치/범위/옵션·입력과 분리한 undo는 통과했습니다. 실패는 paste 포맷 대기 중 selection anchor만 바꿔도 head가 같으면 요청을 유지하는 문제입니다. 요청에서 전체 SelectionSet을 캡처·비교해 anchor/주 선택/다중 선택 변경도 회수하도록 수정했습니다. 영향 6건과 실제 본문/peek의 Type/Paste·서버의 현재 UTF-16 위치/트리거/옵션·분리된 undo를 검사 중이며 아직 결과를 완료로 처리하지 않습니다.

mock-build.log는 App example native-lsp-mock의 --locked --offline 빌드 22.24초·exit 0입니다. 기존 포맷 옵션 모드에 onType 광고와 현재 mirror의 실제 trigger 위치/FormattingOptions 검사·typed 응답만 추가했습니다. 독립 실험 lock/fmt 부채는 고치거나 반복 실행하지 않았습니다. 현재 소유 Cargo 세션은 SDK 전체→상태 영향→실제 앱 순서이며 실행 중 Rust/포함 리소스를 수정하지 않습니다.

### 입력 포맷의 현재 직접 검사

sdk-all.log는 SDK 전체 --all-targets --no-fail-fast의 87통과/0실패·exit 0입니다. 정적 first/more 트리거와 Rust 문서만의 동적 옵션·텍스트 변경 시 Arc 재사용·해제/닫힘/runner 종료 회수를 확인했습니다. state-final.log는 전체 선택 비교 수정 뒤 관련 6통과/0실패·exit 0이며 paste anchor만의 변경·현재 옵션/설정/입력 revision·readonly owner의 writable peek·분리된 undo와 취소/늦은 응답을 포함합니다. 같은 SDK/상태 성공을 실제 앱 준비 변경 때문에 반복하지 않습니다.

app-actual.log는 실제 앱 1실패·17.17초로 미리보기 Type 단계에서 timeout입니다. app-actual-diagnostic.log와 app-focus-diagnostic-final.log는 실제 미리보기 텍스트가 소비되지 않는 1실패씩입니다. app-focus-diagnostic.log는 짧은 이름에 --exact를 잘못 적용해 0건 실행했으므로 검증에서 제외합니다. code-editor 포커스와 egui 모달의 이전/현재 프레임 소유, 팔레트의 is_modal=false 닫힘을 대조했습니다. 검사 helper의 wait_for는 logic만 진행하므로 팔레트 닫힘 뒤 통상 UI 프레임을 그리기 전에 다음 입력을 보냈습니다.

정상 닫힘 프레임을 추가한 app-actual-final.log는 미리보기 Type/Paste와 두 단계 undo를 모두 지난 뒤 부모 readonly 단언에서 1실패·12.63초입니다. 관찰값은 부모 텍스트/revision/dirty 동일, readonly만 true→false입니다. 실제 file service는 해당 정상 크기·정상 인코딩 파일을 writable로 반환하며 검사에서만 주입한 readonly와 모순됩니다. 외부 파일 관측을 멈추거나 플래그를 강제로 재설정하지 않았습니다. 실제 파일 상태의 자동 포맷을 먼저 검사하고, 기존 manual peek의 readonly owner 준비는 그 검사 직전에 주입하도록 분리했습니다. actual 자동 peek의 readonly owner까지 검사했다고 확대하지 않으며 요청 상태 검사에서 별도로 확인한 범위를 유지합니다.

app-actual-owner-final.log는 최종 전체 이름 exact 1통과/0실패·exit 0, 컴파일 9.13초/검사 6.78초입니다. 실제 본문과 순수 peek의 Type/Paste·UTF-16/현재 트리거/편집 폭/방식·포맷만 undo한 뒤 입력 undo·부모 내용/revision/dirty/실제 metadata·디스크 보존, 기존 문서/선택/readonly owner peek 팔레트/저장 포맷을 포함합니다. 닫힘 후 정상 프레임에서도 같은 preview 소유를 유지합니다. 제품의 입력 상태나 대기 키를 직접 초기화하거나 실제 OS 입력·클립보드를 사용하지 않았습니다. UI/App 전체·동결 컴파일/형식·feature 판정·선별 Git은 진행 중이며 아직 전체 성공으로 처리하지 않습니다.

같은 시점에 JSON을 직접 재집계해 599행/10제외/1동결/588대상, complete 289·partial 92·unwired 112·missing 95·미완료 299를 확인했습니다. 최종 batch 33·batch 23 최초 33/미완료 32, 잔여 배정 누락/중복 0입니다. 프로세스 점검의 소유 Cargo는 UI 전체 한 개였고 종료되지 않은 이전 Monaco 조사 프로세스는 관찰하지 않았습니다. 각 실제 앱 검사는 종료했고 상한 없는 재검사를 하지 않습니다. 이 관찰을 전체 세션의 모든 루프가 종료한다는 증명이나 전체 기능 완성률로 확대하지 않습니다.

ui-all.log는 native-host,inspection·test-threads=4의 전체 22대상/440통과/0실패/0ignored·exit 0입니다. app-all.log는 inspection·test-threads=1의 전체 67대상/809통과/5실패/0ignored·exit 101이며 보호 Trash 3건은 제외했습니다. actual Type/Paste·선택 anchor 취소 검사도 이 전체에서 통과했습니다. library 대상은 568통과/5실패·160.75초입니다. 실패는 projects-tests.rs:129의 기존 프로젝트 open 5초 상한, projects-tests.rs:298의 watcher/lockfile 준비 fixture 전체 상한, remote-dispatch-tests.rs:329의 실제 project/file와 pty JSON 명령 두 검사, remote-projects-tests.rs:556의 attach 진입 대기입니다. 뒤의 네 건은 이번 실행의 추가 시간 상한 실패이며 기존 FSEvents 스택을 이 실패들의 직접 원인으로 전용하지 않습니다. 앞의 동일 프로젝트 open 실패는 f1의 반복 조사를 다시 시작하지 않습니다. 추가 네 건은 해당 실행 파일/정확한 검사 이름으로 각각 한 번만 확인하고 결과/남은 위험을 기록합니다.

전체 실행이 종료된 뒤 완료 전 코드 검토에서 completion.event의 상세창 Text 소비는 문서나 선택을 바꾸지 않는다는 점을 확인했습니다. details-repro.log는 실제 상세 포커스에서 본문 텍스트는 fo 그대로인데 자동 포맷 출력이 발생해 1실패·exit 101입니다. completion이 소비한 Text는 이벤트 전후 문서 revision이나 선택 변화가 있을 때에만 출력하도록 수정했습니다. overtype의 커서 변화는 보존하고 상세창이 삼킨 텍스트는 출력하지 않으며 Escape 후 같은 프레임의 실제 본문 입력은 출력합니다. UI 전체는 이 추가 변경 때문에 ui-all-final.log에서 한 번 직접 실행 중이며 App의 성공을 이 마지막 UI 변경의 전체 검사라고 표현하지 않습니다. SDK/상태/변경 없는 Core 성공은 재사용합니다.

### 외부 입력 포맷 단위의 최종 결과

로그 접두사는 /private/tmp/taide-batch23-auto-format-입니다. 각 전체 실행은 --locked --offline --target-dir experiments/native-shell-spike/target·--all-targets --no-fail-fast이며 마지막 의미 변경에 해당하는 영향 검사만 추가했습니다.

| 범위            | 직접 확인한 결과                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SDK             | sdk-all.log: 전체 87통과/0실패·exit 0. 마지막 UI/앱 준비 변경은 SDK에 영향이 없어 재사용합니다.                                                                                                                                                                                                                                                                                                                                                                     |
| UI              | ui-all-final.log: native-host,inspection·test-threads=4·22대상/441통과/0실패/0ignored·exit 0. 앞의 440건이나 관련 1건을 중복 합산하지 않습니다. 마지막 검사 시간 상수 hoist는 같은 네 값을 유지합니다.                                                                                                                                                                                                                                                              |
| App             | app-all.log: inspection·test-threads=1·67대상/809통과/5timeout·exit 101. retry-projects/remote-files/remote-pty/remote-projects.log는 같은 최신 library 실행 파일을 저장소 cwd에서 exact·test-threads=1로 각각 한 번만 실행해 1통과씩·exit 0, 1.05/0.95/1.00/5.81초입니다. 추가 네 실패는 재현되지 않았지만 최초 실패를 지우거나 원인 해결로 표현하지 않습니다. 기존 프로젝트 open 실패는 단독 반복하지 않았고 미해결입니다. 현재 서로 다른 App 성공은 813건입니다. |
| 마지막 App 영향 | app-ui-final.log: 최종 UI 의미 변경 후 기존 실제 본문/peek/저장과 Type/Paste·undo/소유 검사를 exact 1통과/0실패·exit 0, 컴파일 59.36초/검사 9.73초에 확인했습니다. 전체 App를 다시 실행한 결과가 아니며 기존 813건에 중복 합산하지 않습니다.                                                                                                                                                                                                                        |
| 동결/형식/경계  | frozen-host.log의 --tests/inspection 14.24초, frozen-wasm.log의 --lib/wasm32-unknown-unknown/canvas,inspection 35.00초는 exit 0입니다. SDK/UI/App fmt check exit 0·frozen/manifest/lock 변경 0·디스크 503GiB/73%입니다. Core는 변경하지 않아 앞 단위의 서로 다른 294건/성능 1ignored를 재사용하며 해당 성능 검사는 실행하지 않았습니다.                                                                                                                             |

Wasm 검사의 write_stdin이 아직 session_id를 반환했는데 SDK/App fmt check를 시작한 순서 이탈 1회가 있었습니다. 이후 Wasm과 App fmt의 실제 종료 exit 0을 각각 확인했습니다. 형식 검사 도중 소스를 수정하지 않았지만 전체 Cargo/fmt가 항상 직렬이었다고 보고하지 않습니다. 나머지 테스트/소스 변경은 앞 실행의 실제 종료를 확인한 뒤 처리했습니다. 성공 입력이 같은 검사를 이 순서 이탈 때문에 다시 실행하지 않았습니다. 독립 실험 lock/fmt·프로젝트 open/최초 네 timeout·기존 이미지/watcher 지연·실기/성능/출시는 배치 31에 미해결 또는 미검증으로 유지합니다.

SDK/현재 트리거·range-only paste·파일 본문/peek·옵션·readonly/IME/한 커서·전체 선택 취소·분리된 undo·완성 상세창 소비의 외부 공통 단위를 닫습니다. editor-32는 missing에서 partial로 옮깁니다. editor-57 내장 공급자가 아직 없어 editor-32/49 전체를 complete로 올리지 않습니다. 599행/588대상·complete 289·partial 93·unwired 112·missing 94·미완료 299, batch 23 최초 33/미완료 32·최종 33·배정 누락/중복 0입니다. 전체 전환율/잔여 시간은 미산정이며 기능표 행 비율이나 검사 개수를 대신 제시하지 않습니다. 보호 M8 앱·실제 데이터/OS 설정/클립보드/Keychain/Trash·기존 TS/Monaco/xterm을 변경하지 않았습니다. 정상 compile 외에 frozen 소스/의존 그래프/lock을 수정하지 않았습니다.
