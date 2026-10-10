# 배치 23 — 편집기의 나머지 기능

기준: HEAD `69c3438b`, 배치 23 배정 33행. 메인이 직접 수행하며 서브에이전트/workflow를 사용하지 않습니다. 전체 전환율·잔여 시간은 미산정이며 완료 288·미완료 300행을 유지합니다. 아래 조사만으로 기능 구현이나 검증 통과를 주장하지 않습니다.

## editor-51 문서 하이라이트 원본과 현재 경계

원본 `src/shared/lib/lsp/adapters/document-highlight.ts`는 capability가 있을 때 공급자를 등록하고 `textDocument/documentHighlight`로 모델 URI·UTF-16 위치를 전달합니다. 취소되었거나 null 응답이면 빈 배열을 돌려줍니다. 종류 생략은 Text이며 Text/Read/Write를 Monaco의 해당 종류로 변환합니다.

설치된 원본 `node_modules/monaco-editor/esm/vs/editor/contrib/wordHighlighter/browser/wordHighlighter.js`에서 확인한 동작은 다음과 같습니다.

- 기본 occurrencesHighlight는 singleFile이며 기본 occurrencesHighlightDelay는 0입니다. 요청 진입의 runDelayer는 50ms입니다. 이 값은 설치된 `editorOptions.js`와 wordHighlighter의 실제 상수 근거이며 구현 시 이름 있는 상수로 둡니다.
- 텍스트 포커스가 있는 편집기의 주 선택만 기준으로 사용합니다. 여러 줄 또는 단어 바깥까지 선택하면 하이라이트를 지웁니다.
- 등록 순서/점수에 따른 첫 유효 공급자 결과를 사용합니다. 빈 배열도 유효하므로 다음 공급자로 넘어가지 않습니다. TAIDE adapter의 null 응답도 빈 배열이 됩니다. 공급자 오류는 다음 공급자 처리와 구분합니다.
- URI가 일치하는 여러 편집기의 본문에 결과를 표시합니다. 읽기·쓰기·텍스트 종류별 배경/테두리와 overview center·minimap 표시가 있습니다. 정확한 색 기본값/별칭은 `highlightDecorations.js`의 등록과 현재 테마의 원본 색을 기준으로 구현합니다.

현재 native 앱에는 요청·결과·표시를 연결한 문서 하이라이트 소비자가 없습니다. 기존 `crates/taide-lsp/src/native/feature.rs`는 typed DocumentHighlight 요청을 이미 제공합니다. 새 엔진이나 의존성 없이 다음 경계를 사용할 수 있습니다.

- `native/taide-native-app/src/lsp.rs`의 feature_providers·세션/문서 소유·명령/응답 worker와 기존 문서 도움말/구문 접기 공급 패턴
- `native/taide-native-app/src/application.rs`의 LSP reply 소비·본문 EditorRequest 장식·포커스 경계
- `native/taide-native-app/src/editor-locations.rs`의 peek EditorRequest·보조 입력/장식 경계
- `native/taide-native-editor/src/decoration.rs`의 Inline/Overview·revision·stickiness와 기존 위치/구문/문서 도움말의 취소·문서/뷰 소유 검증
- `native/taide-native-ui/src/presentation.rs`와 `crates/taide-model/src/theme.rs`의 현재 테마 경계

설치 SDK의 정확한 typed 파라미터·응답 경계와 F7/Shift+F7 진입은 구현 전에 추가 대조해야 합니다. docs.rs의 두 타입 페이지 조회는 도구 접근 오류였으므로 공식 API 검증 성공으로 세지 않습니다. 코드 소유 범위는 App 요청/상태/본문·peek 소비, 기존 UI 표시/테마 경계와 관련 검사로 한정합니다. Editor/UI에 정규식·구문 엔진 의존성을 추가하지 않습니다.

## 구현·검증 대기

- [ ] 요청/취소/소유: 주 선택의 UTF-16 위치·단어 조건·debounce·편집/언어/파일/탭/뷰/공급자 변경·종료·오래된 응답·capability 변경을 실제 공급 경계에 연결합니다.
- [ ] 실제 표시/명령: 종류별 본문·peek·동일 문서 mirror 표시·테마/overview/minimap과 원본 하이라이트 명령을 연결합니다.
- [ ] 실제 child/앱: 정상·빈/null·오류·미지원·UTF-16/잘못된 범위·취소/오래된 응답과 readonly/tier/편집/undo/다중 뷰를 관련 검사로 검증합니다.
- [ ] 배치 게이트: 변경 크레이트 전체 대상은 배치 종료에 --no-fail-fast로 직접 실행하며 같은 성공은 재사용합니다. frozen compile·fmt/범위·디스크·실기/출시 부채와 선별 Git 근거를 기록합니다.

나머지 32행의 원본 대조와 구현은 미완료입니다. editor-51의 missing 상태와 완료 합계는 변경하지 않았습니다.

## 진척·반복 점검

이번 점검은 실제 감사 JSON 599행을 다시 집계했습니다. 제외 10·동결 1을 뺀 588행 중 complete 288·partial 92·unwired 112·missing 96으로 미완료 300행입니다. 모든 미완료 ID의 배치 배정 누락 0·중복 0·배치 23 미완료 33행·최종 배치 33을 직접 확인했습니다. 최초 계획의 302행은 배치 22 완료 전 배정 스냅샷입니다.

현재 HEAD는 `69c3438b`이며 upstream 차이는 0/0입니다. 최근 제품 커밋 `603e6025`·`7172a544`·`3f9a2c18`에는 기본 선택/색 견본·상세 공유 상태·Read More 마우스 버튼의 서로 다른 변경이 있습니다. 프로세스 이름과 CPU 누적 시간으로 빌드/테스트와 이전 조사 스크립트의 현재 실행 여부를 확인했고 해당 작업 프로세스는 없었습니다. 이는 현재 실행의 점검이며 전체 작업에서 반복 낭비가 전혀 없었다는 증거로 확대하지 않습니다. 이번 조사에서는 Cargo/fmt나 이전 성공 검사를 다시 실행하지 않았습니다.
