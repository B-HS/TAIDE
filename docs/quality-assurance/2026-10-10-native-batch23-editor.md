# 배치 23 — 편집기의 나머지 기능

기준: HEAD `2ea355ed`와 배치 23 변경, 배정 33행. 메인이 직접 수행하며 서브에이전트/workflow를 사용하지 않습니다. 전체 전환율·잔여 시간은 미산정이며 완료 288·미완료 300행을 유지합니다. editor-51은 공급/표시 연결과 아래 검증 근거에 따라 partial로 갱신하며 complete로 올리지 않습니다.

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

설치 SDK의 `lsp-types 0.97.0/src/document_highlight.rs`와 egui/ecolor 원문을 추가로 읽어 typed 파라미터·응답과 색 API를 확인했습니다. 원본 F7/Shift+F7 진입은 설치 wordHighlighter에서 확인했지만 연결은 남아 있습니다. docs.rs의 두 타입 페이지 조회는 도구 접근 오류였으므로 공식 API 검증 성공으로 세지 않습니다. 코드 소유 범위는 App 요청/상태/본문·peek 소비와 관련 검사로 한정하며 Editor/UI에 정규식·구문 엔진 의존성을 추가하지 않았습니다.

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

위 로그의 공통 경로는 `/private/tmp/taide-batch23-`입니다. 서로 다른 관련 성공은 상태 8·child 2·실제 앱 1의 11건이며 원본 스크립트 출력·최초 실패 로그도 보존했습니다. 전체 App/배치 실행 게이트, frozen compile, 실제 화면/OS IME/접근성/대형 성능/soak/출시 게이트는 이번 부분 구현에서 실행하지 않았습니다. F7/Shift+F7/명시 trigger·같은 하이라이트 안의 커서 이동 시 원본 재사용 경계·peek 시작·readonly/tier/추가 공급자 변화의 앱 검증과 배치 전체 체크리스트는 대기입니다.

## 진척·반복 점검

최초 진척 점검은 실제 감사 JSON 599행을 다시 집계했습니다. 제외 10·동결 1을 뺀 588행 중 complete 288·partial 92·unwired 112·missing 96으로 미완료 300행이었습니다. 공급/표시 검증 뒤 editor-51을 partial로 갱신한 현재 분류는 complete 288·partial 93·unwired 112·missing 95로 미완료 300행을 유지합니다. 모든 미완료 ID의 배치 배정 누락 0·중복 0·배치 23 미완료 33행·최종 배치 33을 직접 확인했습니다. 최초 계획의 302행은 배치 22 완료 전 배정 스냅샷입니다.

최초 진척 조사 시 HEAD는 `69c3438b`였고 upstream 차이 0/0을 확인했습니다. 최근 제품 커밋 `603e6025`·`7172a544`·`3f9a2c18`에는 기본 선택/색 견본·상세 공유 상태·Read More 마우스 버튼의 서로 다른 변경이 있습니다. 당시 프로세스 이름과 CPU 누적 시간으로 빌드/테스트와 이전 조사 스크립트의 실행 여부를 확인했고 해당 작업 프로세스는 없었습니다. 이는 당시 실행의 점검이며 전체 작업에서 반복 낭비가 전혀 없었다는 증거로 확대하지 않습니다. 최초 조사에서는 Cargo/fmt나 이전 성공 검사를 다시 실행하지 않았으며 이후 부분 구현의 실제 실행 결과는 앞 절에 구분했습니다.
