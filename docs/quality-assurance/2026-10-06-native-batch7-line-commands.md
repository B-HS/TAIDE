# Native 배치 7 단계 2 — 줄·텍스트 조작과 기본 키

현재 상태: 구현·연결·최종 검증·기록을 완료했습니다. 서브에이전트·workflow는 사용하지 않았습니다. 배치 전체 검사 결과와 보호 대상의 미완료 항목은 통합 문서에 기록했습니다.

## 범위와 기준

- 기준 native 코드: `fbc28b84`와 같은 상태에서 2026-10-09 재개했습니다. 브랜치 HEAD는 문서 커밋을 포함한 `bf5175ca`입니다.
- TS 카탈로그 `src/shared/lib/monaco/monaco-actions.ts`와 native의 동일 카탈로그 `keybinding-commands.json`에 editorLines 명령이 33개 있습니다. editorSelection의 `deleteInsideWord`와 기존 기본 키의 단어 삭제도 함께 검증합니다.
- 기본 키는 Monaco 0.56.0의 실제 등록과 `src/shared/lib/monaco/monaco-keybinding.ts`의 사용자 재지정, 앱 키맵의 편집기·터미널 문맥을 기준으로 삼습니다.
- 정규식·구문 엔진은 편집기나 UI에 추가하지 않습니다. 정규식 평가가 필요한 단어·대소문자 규칙은 기존 앱 전용 syntax 경계에서 처리합니다. locale 정렬은 UI·앱에 이미 있는 ICU collator로 비교를 공급할 수 있으므로 의존성을 추가할 이유가 없습니다.
- 이식 근거는 Monaco 소스(Microsoft Corporation, MIT)입니다. 저장소 `THIRD_PARTY_LICENSES.md`의 Monaco 고지를 적용하며 구현 파일과 근거를 아래에 기록합니다.

## 활성 체크리스트

- [x] TS 대상과 기존 경로 확인 — editorLines 33개, 기존 연결 `deleteAllLeft`·`outdentLines` 2개 확인
- [x] 줄 이동·복사·선택 복제·삭제·삽입·합치기 — 언어 보정·CRLF·undo/redo 코어와 앱 큐 연결 구현. 줄 이동·복사는 아래 Monaco 대조에 포함
- [ ] 정렬·중복 제거·역순 — 코어와 ICU 공급·앱 연결 구현, 앱 비교 검사 진행 중
- [x] 대소문자 7종 — 실제 Monaco 메서드 182개 결과, 단어 범위 1,656개 기준값과 일치. 선택·다중 선택·UTF-16 길이 변화·undo 검사 통과. 앱 연결 완료
- [x] 줄·블록 주석 4종 — 코어·앱 연결, 언어별 기준값·빈 끝 줄·역방향 선택·언어 미지원 검사 통과
- [x] 좌우·단어 삭제, 들여쓰기, 후행 공백·마지막 개행, 괄호 제거·맞바꾸기 — editor 최종 전체 검사 중 line-commands 24건 통과
- [x] 명령 레지스트리·큐·앱 적용 경로와 기본 키·chord·재지정·플랫폼 분기 — 34개 문서 명령 연결. registry 5건·플랫폼 키 route 4건과 transpose를 포함한 앱 큐 검사 통과
- [x] 단계 검증과 기록 완성 — editor 175·syntax 147·UI 257건 전체 통과, 앱 큐의 transpose 포함 경로 통과. app의 알려진 실패와 보호 대상 제약은 통합 문서 참조

## 구현·검증 이력

1. `ED/src/line-commands.rs`와 `ED/tests/line-commands.rs`를 추가했습니다. `ED/src/editing.rs`의 기존 `Plan`에 편집 시작 기준 상대 위치인 `Mark::InEdit`를 추가해 줄 이동과 선택 복제 등에서 선택 위치를 계산합니다. 새 기능을 추가한 것이 아니라 TS의 기존 명령을 옮기는 단계입니다.
2. 첫 명령군의 실패 상태를 먼저 실행했습니다. 초기 진입점이 명령을 적용하지 않는 상태에서 1 통과·7 실패, exit 101이었습니다. 경계 이동의 무변경 대조만 통과했습니다.
3. 구현 뒤 같은 `--test line-commands`가 8 통과·0 실패, exit 0입니다. 검사 명령은 `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test line-commands --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 테스트 프로필 컴파일은 0.86초, 테스트 본문은 0.00초였습니다.
4. 테스트는 유니코드 위치·다음 줄 시작에서 끝나는 선택·빈 줄 복사·선택 복제·마지막 줄 삭제·인접 커서 삭제·선택을 지우지 않는 줄 삽입·CRLF 합치기·읽기 전용·각 편집의 undo/redo를 확인합니다. 언어 규칙이 없는 첫 명령군 검사이며 단계 전체나 앱 연결 통과를 뜻하지 않습니다.
5. `JoinLinesAction`의 기존 후행 공백은 한 칸으로 줄어들며 그 공백 뒤가 접합 커서입니다. `"a  \n  b"`의 기대 커서는 `"a |b"`이며 소스의 `columnDeltaOffset` 규칙으로 유도했습니다.
6. 좌우·단어 삭제·들여쓰기·수동 공백/개행 정리와 토큰 정확도 검사를 추가해 line-commands 17건 통과했습니다. `deleteInsideWord`는 실제 Monaco `WordOperations`를 실행해 1행 3열~9열 삭제(`"hello "`)를 확인하고 잠정 기대값을 `"a z"`로 정정했습니다. 문자열·정규식·미확정 토큰은 공백 정리에서 제외되며 주석·Other는 정리됩니다. 실제 앱 SyntaxLease의 정확도 검사 1건도 통과했습니다(33.19초, 기존 wry 17·linker 1 경고).
7. 정렬·중복 제거·역순을 추가한 line-commands 21건 통과, 문자소·줄 끝·다음 줄 시작의 transposeLetters 새 1건 통과입니다. locale 정렬은 앱의 기존 ICU collator를 공급하며 lexical 비교는 코어 단위 fixture에만 씁니다.
8. `docs/utils/2026-10-09-monaco-case-oracle.js`가 원본 `_modifyText`를 평가해 Unicode·약어·경계·CRLF·astral 포함 26입력 × 7변환 = 182개 fixture를 생성했습니다. 모두 일치합니다. Kebab의 가운데 캡처만 치환해야 하는 규칙과 비-Unicode 정규식의 UTF-16 3단위 소비를 실제 실패로 확인해 보정했습니다. 새 syntax 의존성을 추가하지 않았습니다.
9. `docs/utils/2026-10-09-monaco-line-oracle.js`는 원본 MoveLinesCommand·ShiftCommand·LineCommentCommand·BlockCommentCommand·CopyLinesCommand를 실행하고 원본 TextModel 범위 검증과 PieceTreeTextBuffer 편집을 적용합니다. 23언어 11,730개 내용 결과가 일치합니다(최종 좁은 검사 exit 0, 5.10초). 최초 이동/들여쓰기 4,692개는 먼저 통과했고 주석·복사 확대 후 끝 빈 줄 처리 9건과 oracle의 미검증 0열 처리 9건을 구분해 고쳤습니다. 이 기준값은 내용 검증이며 모든 선택 상태를 전수 검증했다는 뜻은 아닙니다.
10. 같은 도구의 원본 getWordAtText 1,656개 기준값이 native 구성 단어와 일치합니다. 1,000 UTF-16 창과 JS 기본 단어 정규식도 적용했습니다. 언어 편집 대상 33건 통과 뒤 괄호 제거 새 1건, UTF-16 길이 변화 새 경계가 포함된 대소문자 명령 1건을 통과했습니다.
11. 원본 keybinding 등록 평가 사본 `docs/utils/2026-10-09-monaco-editor-keybindings.js`와 `UI/src/editor-keymap-defaults.json`을 추가했습니다. mac·win·linux 줄 복사·블록 주석·구두점 키, 주석 chord·재지정·기본 키 해제·IME 입력 억제의 실제 egui route 3건 통과입니다. registry의 초기 미지원 기대값을 새 실행 경로에 맞춰 갱신해 관련 5건이 통과했습니다.
12. 2026-10-09 사용자 결정으로 `transpose`의 잘못된 UTF-16 단위를 U+FFFD로 변환합니다. 결정은 `docs/acknowledge/2026-10-09-native-transpose-utf16.md`에 기록했습니다. 원본 TransposeAction·TextModel 범위/위치 검증·nodeAcceptEdit·PieceTreeTextBuffer를 실행한 내용·커서 기준값의 실패(exit 101)를 먼저 확인했고 좁은 44개와 연속 astral 4개를 추가한 최종 48개 기준값이 모두 통과했습니다(exit 0). CRLF 역순의 LF·CR은 원본대로 문서 EOL 두 개로 정규화합니다. 다중 커서·선택 건너뛰기·읽기 전용·마지막 줄 끝·내용이 같은 경우의 커서 이동과 undo/redo도 editor 24건에서 확인했습니다. 마지막 transpose를 포함한 실제 앱 큐 검사도 통과했습니다.

## 검증 범위의 한계

locale 정렬은 기존 앱 ICU collator를 사용하며 앱 기본 locale의 악센트·대소문자와 숫자 문자열 정렬을 확인했습니다. 대소문자 변환은 일반 Unicode 기준값을 검증했으며 tr·az·lt locale별 변환은 검증하지 않았습니다. 줄 명령 11,730개는 내용 기준값이며 모든 조합의 선택 상태·겹치는 다중 편집까지 전수 확인한 결과가 아닙니다. 실제 OS IME와 RTL·대형 파일 성능 검증은 통합 문서의 실기 부채로 남깁니다.

## 소스 근거

- `MONACO/editor/contrib/linesOperations/browser/linesOperations.js`: CopyLines 26~144, MoveLines 144~206, 정렬·중복 제거·역순 206~370, 삭제 406~496, 삽입·좌우 삭제 538~743, 합치기 743~885, 맞바꾸기·대소문자 885~1151
- 같은 디렉터리의 `copyLinesCommand.js`·`moveLinesCommand.js`·`sortLinesCommand.js`: 범위·선택 추적과 언어 들여쓰기 보정
- `MONACO/editor/common/cursor/cursorTypeEditOperations.js`의 `EnterOperation.lineInsertBefore`·`lineInsertAfter` 570~607
- `MONACO/editor/common/commands/trimTrailingWhitespaceCommand.js`와 `MONACO/editor/contrib/insertFinalNewLine/browser/insertFinalNewLineCommand.js`

ED는 `native/taide-native-editor`, MONACO는 `node_modules/monaco-editor/esm/vs`입니다.
