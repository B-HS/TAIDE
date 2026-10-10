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
