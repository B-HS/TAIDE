# 배치 7 편집 명령 2: 커서·선택·다중 커서

현재 상태: 구현·연결·최종 검증·기록을 완료했습니다. 서브에이전트·workflow는 사용하지 않았습니다. 배치 전체 검사 결과와 보호 대상의 미완료 항목은 통합 문서에 기록했습니다.

기준은 `src/shared/lib/monaco/monaco-actions.ts`와 설치된 Monaco 0.56의 multicursor·cursorUndo·smartSelect·bracketMatching·caretOperations·lineSelection·anchorSelect입니다. TS 카탈로그에 word-part 이동과 anchor 소비 명령은 없으므로 새 등록을 추가하지 않습니다. 찾기 위젯/LSP/시각적 괄호 가이드는 다음 배치 범위입니다.

- [x] U1. 등록 범위와 원본 동작 확인 — 카탈로그의 선택 20개(편집 명령 1의 deleteInsideWord 제외), cursorUndo·cursorRedo·jumpToBracket을 확인합니다. 원본의 선택 순서, 줄 끝 제외, 읽기 전용 제한과 중복 병합을 구분합니다.
- [x] U2. 순수 커서 명령과 상태 — 코어 13건, 원본 괄호·스마트 선택 2,760개, 리터럴 일치 48개 기준값과 실제 앱 큐 검사를 통과했습니다
- [x] U3. 포인터 입력 — Alt 클릭·토글, Shift+Alt 컬럼 드래그, 더블/트리플 클릭·단어 드래그와 기존 문자 드래그 검사를 통과했습니다. 실제 화면 합성 입력은 사용하지 않았습니다
- [x] U4. 다중 커서 입력 — 중복·겹침 병합과 타이핑·붙여넣기·삭제·Enter·자동 닫기·IME·Escape 검사를 통과했습니다
- [x] U5. 명령·키·앱 큐 연결 — registry 5건·플랫폼 키 route 4건과 앱 큐의 다른 탭 명령 보존 검사를 통과했습니다
- [x] U6. 단계 검증·검토·기록 — editor 175·syntax 147·UI 257건 전체 통과. app 전체 실행과 실패 선별 재검사, 보호 대상·실기 부채를 배치 통합 문서에 기록했습니다

## 대상 파일과 동작

- `native/taide-native-editor/src/cursor-commands.rs`, `bracket-navigation.rs`, `view.rs`, `store.rs`: 23개 명령, revision에 묶인 커서 undo/redo·스크롤 복구, 연속 수직 이동의 표시 열 유지, 괄호 이동·선택, 스마트 선택 캐시, 일치 선택과 anchor 추적을 구현했습니다. 문서 변경은 커서 이력과 선택 컨트롤러를 초기화합니다. 선택 정규화는 원본 CursorCollection의 방향·주 커서·중복 병합 규칙을 따릅니다.
- `native/taide-native-ui/src/editor-pointer.rs`, `editor_surface.rs`, `editor-paint.rs`, `editor-gutter.rs`: Alt 커서·컬럼 선택·다중 클릭·단위 드래그, 모든 커서의 IME preedit·commit, anchor의 원본 색/너비, 주 커서와 다른 드러내기 대상의 스크롤을 연결했습니다. gutter의 줄 번호와 장식 영역 입력을 구분합니다. RTL 포함 문자열의 블록 선택 제외 조건을 원본 containsRTL 범위와 대조했습니다.
- `native/taide-native-ui/src/command-registry.rs`, `keymap.rs`, `editor-keymap-defaults.json`, `native/taide-native-app/src/command-dispatch.rs`, `application.rs`: 원본 기본 키·chord·재지정·읽기 전용과 편집 큐를 연결합니다. 앱 전용 syntax에서 wordPattern을 평가하며 editor/UI에 구문·정규식 의존성을 추가하지 않았습니다.
- `docs/utils/2026-10-09-monaco-cursor-oracle.js`: 실제 Monaco 괄호 파서·Word/BracketSelectionRangeProvider와 리터럴 검색을 실행해 기준값을 만듭니다. `2026-10-09-monaco-editor-keybindings.js`는 실제 등록의 플랫폼별 기본 키를 추출합니다.

## 검증과 수정 근거

1. 원본 괄호·스마트 선택은 orphan opener·예상 밖 closer를 포함한 2,760개 결과가 일치합니다. 초기 154개 차이를 재현한 뒤 괄호의 엄격한 enclosing 경계와 예상 밖 closer 건너뛰기를 수정했습니다. 토큰 Other만 괄호 쌍에 참여합니다. 리터럴 검색은 Rope를 스트리밍하며 CRLF·Unicode 대소문자 기준값 48개가 일치합니다.
2. 코어 13건은 추가/줄 끝 커서·회전, 일치 선택 순환과 내용 편집 후 초기화, smart select 복구, 커서 undo/redo의 실제 스크롤, anchor의 편집·undo·redo 추적, 읽기 전용·정규화를 확인합니다. 짧은 줄을 지난 수직 커서의 목표 열과 anchor undo 위치에서 실제 실패를 먼저 확인했습니다. 전체 editor 최종 게이트는 175 통과·1 ignored·0 실패(exit 0)입니다.
3. UI `editor_surface` 80건 재검사와 `snippet-session` 5건 재검사가 통과했습니다. 다중 커서 자동 닫기·IME·Enter 3건, anchor paint와 주 커서 밖 드러내기 검사도 통과했습니다. preedit의 잘못된 UTF-8 경계를 수락하는 실패를 재현하고 입력 경계에서 거절하도록 수정했습니다. 이 검사는 메모리 내 egui 이벤트이며 실제 OS 입력 검사가 아닙니다.
4. 기존 선택 복제 기대값은 원본의 중복 병합에 따라 2개에서 1개로 수정했습니다. 인접 snippet mirror가 병합된 뒤에는 원본 isSelectionWithinPlaceholders의 개수 조건처럼 session이 취소되므로 취소·일반 삭제·undo 분리를 확인했습니다. snippet의 편집 겹침 거절 검사는 별도 유효 커서로 유지했습니다. 기존 문자 드래그 검사는 빠른 연속 클릭이 원본 더블클릭으로 해석되지 않도록 명시적 시간을 주었습니다.
5. 원본 기본 키의 mac·win·linux 추가 커서 차이, Cmd/Ctrl+D·커서 undo·주석 chord와 사용자 재지정을 플랫폼 검사 4건에서 확인했습니다. 앱 큐 검사는 선택만 바뀌는 명령의 문서 revision 보존과 다른 탭의 대기 명령 보존을 확인했습니다. 최종 전체 UI/app 게이트는 통합 문서에 기록합니다.

## 남은 실기 검증

실제 OS 한글/다국어 IME, 접근성 입력, RTL 레이아웃, 대형 파일에서의 응답성·메모리, 모든 중첩 다중 커서 편집 조합은 미실행입니다. 스마트 선택 기준값은 word·bracket provider 범위이며 LSP selectionRange 결과와 실제 서버 연결은 후속 LSP 작업에서 검증해야 합니다. 이 항목들은 기능 대응률 100%나 실기 완료로 보고하지 않습니다.
