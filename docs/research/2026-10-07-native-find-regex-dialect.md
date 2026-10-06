# Native 찾기/바꾸기 위젯의 정규식 방언 조사 (2026-10-07)

상태: 읽기 전용 조사입니다. 코드·설정은 바꾸지 않았고 빌드·테스트·실행도 하지 않았습니다. 설계 정본 `docs/research/2026-10-06-native-editor-display-layer-design.md` 7절 "단계 6"과 8절 D6 의 후속 문서입니다.

## 0. 읽는 법

- 기준 시점: HEAD `06b19ca0`. 경로는 저장소 루트(`/Users/hyunseokbyun/development/TAIDE`) 기준 상대 경로입니다.
- 약어: MONACO = `node_modules/monaco-editor/esm/vs` (0.56.0, `package.json` 의 `vscodeCommitId` 는 `f487add297079a02eb836810185b165e50cadabc`).
- 코드 서술은 직접 읽은 파일과 줄 번호를 근거로 달았습니다. 외부 크레이트 수치는 2026-10-07 에 crates.io API·docs.rs·공식 저장소에서 조회했고 출처는 9절에 있습니다. 확인하지 못한 것은 8절에 모았습니다.
- 이 문서에서 "JS 정규식"은 ECMAScript 정규식을 뜻합니다. TS 제품은 Tauri 웹뷰에서 돌기 때문에 macOS 에서의 실제 엔진은 WKWebView 의 JavaScriptCore 입니다(2.10).

## 1. 결론 요약

1. Monaco 찾기는 `new RegExp(source, 'g' + 'i'? + 'm'? + 'u')` 하나로 동작합니다. `u` 는 항상 켜지고, `m` 은 패턴에 줄바꿈 관련 표기(`\n`, `\r`, `\W`, 실제 LF)가 있을 때만 켜지며 그때는 문서 전체를 LF 로 이어 붙인 문자열에서 검색합니다. 그렇지 않으면 줄 하나를 문자열 하나로 넘겨 줄 단위로 검색합니다(2.1, 2.2).
2. 단어 단위 옵션, 빈 일치 전진, 일치 수 한도 19999, 바꾸기 패턴(`$1`·`$&`·`\n`·`\u` 등), 대소문자 보존은 전부 정규식 엔진 밖의 Monaco 자체 코드입니다. 엔진에 요구되는 것은 "지정 위치부터 다음 일치와 캡처 그룹 범위를 돌려주는 것" 하나입니다(2.3~2.6).
3. TS 앱은 찾기 옵션을 하나도 바꾸지 않았습니다. Monaco 기본값 그대로입니다(2.9).
4. TS 제품은 이미 방언이 둘입니다. 찾기 위젯은 JS 정규식, 검색 패널은 `crates/taide-search` 의 Rust `regex` 방언(줄 단위, 여러 줄 일치 없음, 캡처 치환 없음)입니다. 사용자에게 방언을 안내하는 문구는 어느 쪽에도 없습니다(4절).
5. 추천안: 찾기 위젯은 JS 방언을 유지하고, 엔진은 ECMAScript 문법을 목표로 하는 `regress` 0.12.0 을 앱 쪽 구현으로 넣되 적합성·성능 게이트 통과를 조건으로 채택합니다. 게이트 실패 시 차선은 `fancy-regex` 0.19.2 입니다(5절).
6. 사용자 결정이 필요한 것은 방언(엔진) 선택, 새 의존성 1개 반입, 백트래킹 폭주 대응 수준, Monaco 특이 동작의 재현 여부입니다(7절).

## 2. TS 기준: Monaco 찾기 모델

### 2.1 정규식 생성과 플래그

- 진입점은 `SearchParams.parseSearchRequest` 입니다(MONACO `editor/common/model/textModelSearch.js:19-53`). 검색어가 비면 `null`, 정규식 생성이 예외를 던져도 `null` 이며 그 경우 일치는 0건으로 조용히 처리됩니다.
- 실제 생성은 `createRegExp` 입니다(MONACO `base/common/strings.js:131-160`).
    - 정규식 모드가 아니면 검색어를 `escapeRegExpCharacters` 로 이스케이프합니다. 대상 문자는 `\ { } * + ? | ^ $ . [ ] ( )` 입니다(`strings.js:68-70`).
    - 플래그: `g` 항상, 대소문자 구분이 꺼져 있으면 `i`, 여러 줄이면 `m`, `u` 항상. `s`·`y`·`v` 는 쓰지 않습니다.
    - `wholeWord` 는 항상 `false` 로 넘깁니다. 단어 단위는 정규식의 `\b` 가 아니라 2.3 의 별도 판정입니다.
- 정규식 모드가 아니고 대소문자 영향이 없는 검색(대소문자 구분이 켜져 있거나 검색어의 대소문자 변환 결과가 같은 경우)은 정규식을 쓰지 않고 `indexOf` 로 찾습니다(`textModelSearch.js:47-52, 213-227`). 정규식 모드가 아닌 대소문자 무시 검색은 이스케이프한 검색어를 `giu` 로 컴파일하므로 유니코드 대소문자 폴딩이 적용됩니다.
- `u` 플래그의 결과: `\p{...}` 사용 가능, `\u{1F600}` 사용 가능, 서로게이트 쌍이 문자 하나로 처리됨, 의미 없는 이스케이프(`\-` 등 클래스 밖)와 짝 없는 `{`·`}` 가 문법 오류가 됨.
- ECMAScript 규칙상 `u` 가 켜져도 `\d` 는 `[0-9]`, `\w` 는 `[A-Za-z0-9_]` 입니다. `i` 와 `u` 가 함께 켜진 경우에만 `\w` 가 대소문자 폴딩으로 위 문자에 대응되는 유니코드 문자까지 포함합니다(MDN, 9절).

### 2.2 줄 단위 검색과 여러 줄 검색

- 여러 줄 판정(`isMultilineRegexSource`, `textModelSearch.js:55-78`): 정규식 모드에서는 패턴에 실제 LF 문자가 있거나 `\n`, `\r`, `\W` 표기가 있으면 여러 줄입니다. 정규식 모드가 아니면 검색어에 LF 가 있을 때만 여러 줄입니다. `\s`, `[^a]`, `.` 은 여러 줄 판정에 들어가지 않으므로 `\s+` 는 줄을 넘지 못합니다.
- 줄 단위(`_doFindMatchesLineByLine`, `textModelSearch.js:188-210`): 줄 내용 문자열(줄 끝 문자 제외)을 하나씩 엔진에 넘깁니다. `m` 플래그가 없으므로 `^`·`$` 는 그 문자열의 처음과 끝, 곧 줄의 처음과 끝에만 맞습니다.
    - 검색 범위의 첫 줄과 마지막 줄은 `substring` 으로 잘라서 넘깁니다(`:193, 198, 206`). 그래서 선택 영역 안에서 찾기를 하면 `^` 가 범위 시작 열에서 맞고, 후방 탐색은 범위 앞의 글자를 보지 못합니다.
    - 다음 일치 찾기(`_findFirstMatchInLine`, `:293-301`)는 반대로 줄 전체를 넘기고 `lastIndex` 를 시작 열로 둡니다. 여기서는 후방 탐색이 시작 열 앞의 글자를 봅니다. 엔진에 "문자열 전체 + 시작 위치" 형태의 호출이 필요한 이유입니다.
- 여러 줄(`_doFindMatchesMultiline`, `:169-187`): 검색 범위의 텍스트를 줄 끝을 전부 LF 로 바꿔 한 문자열로 만든 뒤(`getValueInRange(..., EndOfLinePreference.LF)`) `m` 플래그로 검색합니다. 문서가 CRLF 이면 일치 위치를 `LineFeedCounter` 로 보정합니다(`:89-130, 146-168`). 결과적으로 `\n` 은 CRLF 문서의 줄 끝에도 맞고, `\r` 은 어떤 문서에서도 맞지 않습니다.
- 이전 일치 찾기는 여러 줄일 때 문서 처음부터 시작 위치까지 전부 찾고 마지막 것을 고르며, 내부 한도는 `10 * LIMIT_FIND_COUNT` = 9990 입니다(`:313-324`, `LIMIT_FIND_COUNT` 는 `:11` 의 999).

### 2.3 Searcher: 빈 일치와 단어 단위

- `Searcher.next`(`textModelSearch.js:416-452`)는 `regex.exec` 를 반복 호출합니다.
    - 직전 일치의 끝이 문자열 끝이면 종료합니다.
    - 같은 위치에서 같은 길이의 일치가 다시 나오면, 길이가 0 인 경우 `lastIndex` 를 코드 포인트 하나만큼(서로게이트 쌍이면 2, 아니면 1) 전진시키고 다시 찾습니다. 길이가 0 이 아니면 종료합니다.
- 단어 단위(`isValidMatch`, `:354-403`)는 `wordSeparators` 분류기 기준입니다. 일치의 왼쪽은 "문자열 시작이거나, 앞 글자가 구분자·공백·CR·LF 이거나, 일치의 첫 글자가 구분자"이면 경계로 보고 오른쪽도 대칭입니다. 일치가 조건을 만족하지 않으면 버리고 다음 일치를 찾습니다.
    - 구분자 기본값은 `` `~!@#$%^&*()-=+[{]}\|;:'",.<>/? `` 입니다(MONACO `editor/common/core/wordHelper.js:8`, `editor/common/config/editorOptions.js:3423`). 공백과 탭은 분류기가 따로 공백으로 표시합니다(`wordCharacterClassifier.js:23-26`).
    - JS 정규식의 `\b` 와 다릅니다. 예를 들어 `_` 와 한글·악센트 문자는 구분자가 아니고, 구분자 목록에 없는 유니코드 구두점(`—` 등)도 구분자가 아닙니다.

### 2.4 한도

| 항목 | 값 | 근거 |
| --- | --- | --- |
| 강조하는 일치 수 | 19999 (`MATCHES_LIMIT`) | MONACO `editor/contrib/find/browser/findModel.js:68, 179` |
| 한도 도달 시 표시 | 개수 뒤에 `+`, 툴팁 "Only the first {0} results are highlighted, but all find operations work on the entire text." | `findWidget.js:59, 327-355` |
| 한도 도달 시 탐색 | 장식 목록이 아니라 모델의 `findNextMatch`·`findPreviousMatch` 로 직접 찾음. 순환 옵션이 꺼져 있어도 순환 탐색 허용 | `findModel.js:243-284, 317-332`, `findState.js:233-241` |
| overview ruler 표식 병합 | 일치가 1000 개를 넘으면 가까운 줄을 병합 | `findDecorations.js:141-173` |
| 모델 `findMatches` 기본 한도 | 999 (찾기 위젯은 19999 또는 `MAX_SAFE_SMALL_INTEGER` 를 직접 넘김) | `textModel.js:79, 854` |
| 재검색 지연 | 문서가 50MB 초과(`isTooLargeForSyncing`)이면 240ms 타이머, 아니면 동기 실행 | `findModel.js:69, 122-142`, `textModel.js:119, 220` |
| 본문 변경 뒤 재검색 | 100ms 스케줄러 | `findModel.js:79-85, 94-104` |
| 선택 영역 시드 최대 길이 | 524288 | `findController.js:43, 59` |
| 모두 바꾸기·모든 일치 선택 | 한도 없음(`MAX_SAFE_SMALL_INTEGER` = 1073741824) | `findModel.js:462, 476` |

### 2.5 바꾸기 패턴

- 정규식 모드일 때만 치환 문자열을 해석합니다. 아니면 문자 그대로입니다(`findModel.js:378-383`).
- 해석기는 `parseReplaceString` 입니다(MONACO `editor/contrib/find/browser/replacePattern.js:176-287`). JS 의 `String.prototype.replace` 규칙이 아니라 Monaco 자체 규칙입니다.

| 표기 | 동작 |
| --- | --- |
| `\n`, `\t`, `\\` | LF, 탭, 역슬래시 |
| `\u`, `\l` | 뒤따르는 캡처 치환의 한 글자를 대문자·소문자로 |
| `\U`, `\L` | 뒤따르는 캡처 치환의 남은 글자 전부를 대문자·소문자로 |
| `$$` | `$` |
| `$&`, `$0` | 일치 전체 |
| `$1`~`$99` | 캡처 그룹. 두 자리 수를 먼저 시도 |
| 그 밖 | 그대로 둠. `$<name>`, `` $` ``, `$'` 는 지원하지 않음 |

- 대소문자 지시는 여러 개를 쌓을 수 있고 다음 캡처 치환 한 번에만 적용된 뒤 비워집니다(`:195, 236, 258-259, 274-275, 280-281`). 적용은 UTF-16 코드 단위 하나씩입니다(`:62-91`).
- 없는 그룹 번호는 뒤 자리부터 떼어 가며 다시 시도합니다(`_substitute`, `:96-114`). 그룹이 3개일 때 `$10` 은 `$1` 의 값 뒤에 `0` 이 붙고, 끝까지 없으면 `$` 와 숫자가 문자 그대로 남습니다. 참여하지 않은 그룹은 빈 문자열입니다.

### 2.6 대소문자 보존 바꾸기

- `buildReplaceStringWithCasePreserved`(MONACO `base/common/search.js:7-37`)가 담당하며 치환 패턴이 고정 문자열일 때만 적용됩니다(`replacePattern.js:43-51`). 캡처 치환이 있는 패턴에는 적용되지 않습니다.
- 규칙 순서: 일치와 치환에 `-` 만(또는 `_` 만) 같은 개수로 있으면 조각별로 재귀 적용 → 일치가 전부 대문자이면 치환도 대문자 → 전부 소문자이면 소문자 → 첫 글자가 대문자이면 치환 첫 글자만 대문자 → 첫 글자가 소문자이면 치환 첫 글자만 소문자 → 그 밖은 그대로.

### 2.7 바꾸기와 모두 바꾸기의 두 경로

- 바꾸기 한 번(`replace`, `findModel.js:384-405`): 선택 시작 위치에서 다음 일치를 캡처 포함으로 찾고, 선택이 그 일치와 정확히 같으면 치환하고 아니면 그 일치로 이동만 합니다.
- 모두 바꾸기(`replaceAll`, `:410-423`):
    - 일반 경로(`_regularReplaceAll`, `:459-469`): 한도 없이 모든 일치를 찾아 일치마다 치환 문자열을 만들고 한 명령으로 적용합니다.
    - 대량 경로(`_largeReplaceAll`, `:424-458`): 선택 영역 찾기가 아니고 일치 수가 19999 이상이면 문서 전체 텍스트(LF)에 `String.prototype.replace` 를 한 번 적용하고 전체 범위를 교체합니다.
- 대량 경로는 일반 경로와 결과가 달라질 수 있습니다. 코드 읽기로만 확인했고 실행으로 재현하지는 않았습니다.
    - 여러 줄 패턴이 아니어도 `m` 플래그를 붙여 전체 텍스트에서 실행하므로(`:431-440`) `\s+$` 같은 패턴이 줄을 넘어 일치할 수 있습니다.
    - 단어 단위 판정(2.3)을 거치지 않습니다. 정규식 자체에는 단어 단위가 들어 있지 않기 때문입니다(`:425-430, 448-455`).

### 2.8 위젯의 검증과 시드

- 입력 검증: 정규식 모드에서 `new RegExp(value, 'gu')` 가 던진 예외의 `message` 를 입력 상자 아래에 그대로 보여 줍니다(`findWidget.js:783-795`). 문구는 엔진(JavaScriptCore)이 만든 영어 문장입니다.
- 선택 영역으로 검색어를 채울 때 정규식 모드이면 `escapeRegExpCharacters` 로 이스케이프합니다(`findController.js:219-224, 238-248`).
- 옵션 상태(대소문자, 단어 단위, 정규식, 대소문자 보존)는 작업 영역 범위 저장소에 보관됩니다(`findController.js:94-97, 139-150`).

### 2.9 TS 앱이 바꾼 부분

- 없습니다. `src` 전체에서 Monaco 찾기 옵션(`find:` 편집기 옵션, `seedSearchStringFromSelection`, `autoFindInSelection`, `addExtraSpaceOnTop`, `wordSeparators` 등)을 설정하는 코드는 검색되지 않았습니다. 기본값은 `cursorMoveOnType: true`, `seedSearchStringFromSelection: 'always'`, `autoFindInSelection: 'never'`, `globalFindClipboard: false`, `addExtraSpaceOnTop: true`, `loop: true`, `history: 'workspace'`, `replaceHistory: 'workspace'` 입니다(MONACO `editor/common/config/editorOptions.js:462-471`).
- TS 가 하는 일은 Monaco 액션을 명령으로 노출하는 것뿐입니다: `actions.find`, `actions.findWithSelection`, `editor.action.nextMatchFindAction`, `editor.action.nextSelectionMatchFindAction`, `editor.action.previousMatchFindAction`, `editor.action.previousSelectionMatchFindAction`, `editor.action.startFindReplaceAction`, `editor.actions.findWithArgs`(`src/shared/lib/monaco/monaco-actions.ts:66-72, 409-463, 609, 741`), 그리고 포커스된 편집기에서 `actions.find` 를 실행하는 키맵 처리(`src/widgets/editor-area/editor-area.tsx:157, 295`).

### 2.10 TS 의 실제 엔진

- Monaco 는 호스트의 `RegExp` 를 그대로 씁니다. TS 제품은 Tauri 웹뷰에서 실행되므로 macOS 에서는 WKWebView 의 JavaScriptCore 가 엔진입니다. 지원 문법의 상한은 OS 에 설치된 WebKit 버전이 정합니다.
- Monaco 가 `v` 플래그를 넘기지 않으므로 `v` 전용 문법(클래스 집합 연산)은 엔진이 지원하더라도 찾기 위젯에서 쓸 수 없습니다.

### 2.11 설계 문서와 어긋난 점

- 설계 문서 7절 단계 6 은 `find` 미지원 단언의 위치를 `command-dispatch.rs:189` 로 적었습니다. 현재 파일에서는 `native/taide-native-app/src/command-dispatch.rs:234` 부근입니다.
- 설계 문서가 적은 `regex` 1.13.1(`native/taide-native-app/Cargo.toml:51`, `=1.13.1`)과 `fancy-regex` 최신 0.19.2 는 현재도 맞습니다.

## 3. 후보 비교

### 3.1 기본 정보

| 항목 | regex | fancy-regex | regress | ferroni |
| --- | --- | --- | --- | --- |
| 확인한 최신 버전(공개일) | 1.13.1 (2026-07-15) | 0.19.2 (2026-09-13) | 0.12.0 (2026-08-23) | 1.9.0 (2026-10-06). 저장소가 고정한 것은 1.8.1 (2026-10-01) |
| 라이선스 | MIT OR Apache-2.0 | MIT | MIT OR Apache-2.0 | BSD-2-Clause |
| MSRV | 1.65 | 1.66 | crates.io 에 미기재. `Cargo.toml` 의 edition 이 2024 이므로 1.85 이상 | 1.94 |
| 저장소 MSRV(1.95, `native/taide-native-app/Cargo.toml:5`)와의 적합 | 적합 | 적합 | 적합 | 적합 |
| 방식 | 유한 오토마타 | 백트래킹 VM + 단순한 부분은 `regex` 에 위임 | 고전 백트래킹 | Oniguruma 이식(백트래킹) |
| 목표 문법 | 자체(Perl 계열 부분집합) | `regex` 문법 + 확장. Oniguruma 호환 플래그 있음 | ECMAScript 2018 + `v` 플래그 | Oniguruma (12개 문법 모드, ECMAScript 모드 없음) |
| 유지 상태 | rust-lang 저장소, 누적 12.2억 다운로드 | 커뮤니티 유지, 누적 2.47억 다운로드, 2026 년에 3회 이상 릴리스 | 최초 공개 2020-05-25, 누적 4551만 다운로드, 저장소 열린 이슈 7건 | 최초 공개 2026-02-13, 누적 4277 다운로드 |
| 앱 lockfile 에 새로 생기는 패키지(예상) | 0 | 1 (`fancy-regex`. `bit-set` 0.8.0, `bit-vec` 0.8.0, `regex-automata` 0.4.18, `regex-syntax` 0.8.11 은 이미 있음) | 1 (`regress`. 필수 의존성 `memchr` 2.8.3 은 이미 있음) | 0 (이미 `taide-native-syntax` 경유로 1.8.1 고정. 앱 `Cargo.toml` 에 직접 의존 선언만 추가) |
| 로컬 cargo registry 에 소스가 있는가 | 있음 | 0.14.0 만 있음. 0.19.2 는 없음 | 없음 | 있음 |

"새로 생기는 패키지" 는 `native/taide-native-app/Cargo.lock` 을 읽고 각 크레이트의 의존성 목록과 대조한 예상치입니다. cargo 를 실행하지 않았으므로 확정이 아닙니다.

### 3.2 ECMAScript 문법·의미와의 차이

기준 열은 Monaco 가 쓰는 조합(`u` 항상, `i`·`m` 조건부)입니다.

| 항목 | ECMAScript (`u`) | regex | fancy-regex | regress | ferroni |
| --- | --- | --- | --- | --- | --- |
| 후방 참조 `\1`, `\k<name>` | 있음 | 없음 | 있음 | 있음 | 있음 |
| 전방 탐색 `(?=)`, `(?!)` | 있음 | 없음 | 있음 | 있음 | 있음 |
| 후방 탐색 `(?<=)`, `(?<!)` | 있음(가변 길이) | 없음 | 있음(가변 길이는 기본 feature) | 있음(가변 길이, 캡처 포함) | 있음 |
| 이름 있는 그룹 | `(?<name>)` | `(?<name>)`, `(?P<name>)` | `(?<name>)`, `(?P<name>)` | `(?<name>)`, 중복 이름 그룹 | `(?<name>)`, `(?'name')`, `(?P<name>)` |
| 유니코드 속성 `\p{...}` | 있음 | 있음 | 있음 | 파서에 구현됨. 크레이트 문서는 미구현이라고 적혀 있어 서로 어긋남 | 있음(902개 이름, Unicode 17.0) |
| `\d` | `[0-9]` | `\p{Nd}` | `\p{Nd}` (`unicode_mode` 를 끄면 ASCII) | ECMAScript 규칙 | Oniguruma 규칙. 기본 범위는 이번 조사에서 미확인 |
| `\w` | `[A-Za-z0-9_]`, `iu` 에서 폴딩 대응 문자 포함 | 유니코드(`\p{Alphabetic}` + `\p{M}` + `\d` + `\p{Pc}` + `\p{Join_Control}`) | `regex` 와 같음 | ECMAScript 규칙(0.11.0 에서 U+212A, U+017F 처리 수정) | Oniguruma 규칙. 미확인 |
| `\b` | ASCII `\w` 기준 | 유니코드 `\w` 기준. `(?-u:\b)` 로 ASCII | `regex` 와 같음 | ECMAScript 규칙 | Oniguruma 규칙. 미확인 |
| `^`·`$` 의 줄 끝 인식(여러 줄) | `\n`, `\r`, U+2028, U+2029 | `\n` 만. CRLF 모드(`R`)에서 `\r`·`\n` | `regex` 와 같음(`crlf` 옵션) | ECMAScript 규칙 | 기본 문법에서 `^`·`$` 가 항상 줄 경계에 맞음(`src/api.rs:584-598`). 옵션 `ONIG_OPTION_MULTILINE` 은 dotall 을 뜻함(`src/api.rs:574-582`) |
| `.` 이 제외하는 문자 | `\n`, `\r`, U+2028, U+2029 | `\n` (CRLF 모드에서 `\r` 도) | `regex` 와 같음 | ECMAScript 규칙 | Oniguruma 규칙. 미확인 |
| 코드 포인트 이스케이프 | `\uXXXX`, `\u{...}` | `\x7F`, `\x{...}`, `\uXXXX`, `\u{...}`, `\UXXXXXXXX` | `regex` 와 같음 | `\uXXXX`, `\u{...}` | `\uHHHH`, `\x{...}`. `\u{...}` 플래그는 문법 정의에서 찾지 못함 |
| JS 에 없는 문법이 추가로 통과하는가 | 기준 | `\A`, `\z`, `(?i)` 인라인 플래그, `[a&&b]` 집합 연산, `\<`·`\>` | 왼쪽 + `(?>)`, `\K`, `\G`, `\h`, `\R`, 조건식, 서브루틴 | 아니오(ECMAScript 문법) | 소유 수량자, `(?>)`, `\h`, `\G`, 조건식, callout 등 다수 |
| 지정 위치부터 찾기(`g`·`lastIndex` 대응) | `lastIndex` | `find_at`, `captures_at`. 시작 위치 앞 문맥을 고려 | `find_from_pos`, `captures_from_pos` | `find_from(text, start)` | 고수준 `find_with` 는 0 부터만. 저수준 `onig_search` 에 시작 위치 인자 |
| sticky `y` | Monaco 가 쓰지 않음 | 해당 없음 | 해당 없음 | `g`·`y` 플래그 없음. 호출로 대체 | 해당 없음 |

- regress 는 `u` 와 `v` 플래그를 `Flags` 의 `unicode`·`unicode_sets` 필드로 받고 `i`·`m`·`s`·`u`·`v` 문자를 해석합니다. 인라인 수정자 `(?i:...)` 도 파서에 있습니다.
- MDN 은 2026-10-07 조회 시점에 `\A`·`\z`·`\Z` 버퍼 경계 단정을 JS 문법으로 소개합니다. JavaScriptCore 와 regress 의 지원 여부는 확인하지 못했습니다.

### 3.3 성능과 최악 시간

| 항목 | regex | fancy-regex | regress | ferroni |
| --- | --- | --- | --- | --- |
| 최악 시간 | 패턴 크기 m, 입력 길이 n 에 대해 `O(m * n)` 보장 | 확장 문법이 없는 패턴은 `regex` 에 위임되어 선형. 확장 문법을 쓰면 보장 없음 | 보장 없음(공식 문서가 명시) | 보장 없음 |
| 백트래킹 폭주 방어 | 필요 없음. 컴파일 크기 한도 `size_limit` | `backtrack_limit` 기본 1,000,000. 초과 시 `RuntimeError::BacktrackLimitExceeded` | 0.12.0 공개 API 에는 없음(docs.rs 전체 항목에 budget 관련 항목 없음). 저장소 master 에는 `find_from_budgeted` 가 있으나 미공개 | `SearchOptions` 로 시간 제한·재시도 한도·스택 한도를 검색마다 지정. 기본 재시도 한도 10,000,000(`src/regint.rs:25`), 기본 시간 제한 없음 |
| 컴파일 단계 방어 | 중첩 한도, 크기 한도 | `delegate_size_limit` | 캡처 그룹·루프·중첩 깊이·문자 집합 길이 상한, 0.12.0 에서 과도한 중첩 파싱 강화 | 파싱 깊이 4096(`src/regint.rs:16`), 백트래킹 위험 패턴 거부 옵션 |
| 처리량 | 네 후보 중 일반적으로 가장 빠름(ferroni README 도 인정) | 단순 패턴은 `regex` 수준. 확장 문법 사용 시 미확인 | 수치 미확인. `memchr` 기반 최적화가 있다는 것만 확인 | 공급자 주장만 있음. 독립 검증 없음 |

- TS 기준인 JS 엔진도 백트래킹 방식이며 Monaco 는 실행 시간을 제한하지 않습니다. 문서가 50MB 이하이면 입력할 때마다 동기 실행합니다(2.4). 즉 폭주 패턴에서 멈추는 것은 TS 와 같은 동작입니다.

### 3.4 대형 문서와 rope

- 네 후보 모두 연속된 `&str`(또는 `&[u8]`)만 받습니다. rope 조각을 직접 받지 못합니다.
- 줄 단위 검색은 줄 하나를 `&str` 로 넘기면 됩니다. native 문서는 ropey 1.6.1 이고(`native/taide-native-editor/Cargo.toml:14`) `Rope::line(i)` 가 `RopeSlice` 를 돌려줍니다. `RopeSlice::as_str()` 는 한 조각 안에 있을 때만 `Some` 이고, 그렇지 않으면 `Cow<str>` 변환으로 복사가 한 번 생깁니다(ropey `src/slice.rs:615, 1762`). 스토어는 `rope()` 접근자를 이미 공개합니다(`native/taide-native-editor/src/store.rs:56`).
- 여러 줄 검색은 Monaco 자신이 범위 전체를 문자열 하나로 만듭니다(2.2). native 도 LF 로 정규화한 `String` 을 만들고 위치를 되돌려 계산하면 같은 동작입니다. 20MB 문서에서 임시 20MB 가 생기는 것도 Monaco 와 같습니다.
- rope 를 복사 없이 검색하는 크레이트로 `regex-cursor` 0.1.5(MIT OR Apache-2.0, MSRV 1.65, ropey `^1.6.1` 연동 feature)가 있습니다. `regex-automata` 엔진의 포크라서 전후방 탐색·후방 참조가 없고 최종 릴리스가 2025-02-26 입니다. 위 방식으로 충분하므로 이번에는 필요하지 않습니다.

### 3.5 제외한 후보

| 후보 | 제외 이유 |
| --- | --- |
| `pcre2` 0.2.11 (Unlicense OR MIT) | C 라이브러리 바인딩. 순수 Rust 가 아니고 방언도 PCRE2 |
| `onig` | C Oniguruma 바인딩. 설계 문서 5절에서 이미 C 의존을 피하기로 함 |
| `oxc_regular_expression` 0.153.0 (MIT) | 실행 엔진이 아니라 ECMAScript 정규식 파서. MSRV 1.97 로 저장소 MSRV 1.95 를 넘음 |
| `swc_ecma_regexp`, `js-regex` | 파서·검증기. 실행 엔진 아님 |
| `js-regexp` | 브라우저 `RegExp` 바인딩. native 에서 쓸 수 없음 |

## 4. 워크스페이스 검색과의 일관성

### 4.1 `taide-search` 의 엔진과 방언

- 엔진은 `regex` 입니다(`crates/taide-search/Cargo.toml:10`, `regex = "1"`). native 앱 lockfile 에서는 `taide-runtime` 경유로 들어오며 1.13.1 로 해석됩니다.
- 정규식 컴파일은 `RegexBuilder::new(&query.text).case_insensitive(!query.case_sensitive)` 뿐입니다(`crates/taide-search/src/service.rs:126-138`). 여러 줄 플래그, CRLF 모드, 유니코드 끄기 설정이 없으므로 Rust `regex` 기본 방언 그대로입니다.
- 검색은 파일을 `text.lines()` 로 나눠 줄 단위로 합니다(`service.rs:363-364`). 줄을 넘는 일치는 불가능합니다. Monaco 의 여러 줄 모드에 해당하는 경로가 없습니다.
- 단어 단위는 일치 앞뒤 글자가 `char::is_alphanumeric()` 또는 `_` 가 아닌지로 판정합니다(`service.rs:29-37`). Monaco 의 구분자 목록 방식(2.3)과 다릅니다.
- 정규식이 아닌 대소문자 무시 검색은 ASCII 범위만 접습니다(`service.rs:54-98`). Monaco 는 유니코드 폴딩입니다(2.1).
- 바꾸기는 치환 문자열을 그대로 넣습니다(`service.rs:673-683`). `$1` 같은 캡처 치환, `\n`, 대소문자 지시가 없습니다.
- 일치 수 한도는 10,000 입니다(`crates/taide-model/src/search.rs:4`).

### 4.2 TS 가 사용자에게 안내하는 방언

- 검색 패널의 정규식 토글 라벨은 "정규식 사용"(`crates/taide-locale/resources/locales/ko.json:608`, en 은 "Use Regular Expression")뿐이고 방언 설명은 없습니다.
- 잘못된 패턴은 `error.search.invalidRegex`("잘못된 정규식입니다: {{detail}}")로 Rust `regex` 의 오류 문장을 그대로 붙여 보여 줍니다(`service.rs:130-137`, `ko.json:196`). 입력 중 실행에서는 토스트를 띄우지 않고 패널의 실패 표시로만 알립니다(`src/entities/search/use-search-run.ts:14-19`).
- 찾기 위젯은 JavaScriptCore 의 오류 문장을 보여 줍니다(2.8).
- 정리하면 TS 제품은 두 방언을 구분 없이 "정규식"이라고만 부르고, 차이는 오류 문구와 동작으로만 드러납니다. VS Code 도 같은 구조입니다. 편집기 찾기는 JS 정규식이고 파일 검색은 ripgrep 의 Rust `regex` 이며, 지원하지 않는 문법이면 PCRE2 로 넘어갑니다(VS Code 1.29 릴리스 노트, 9절).

### 4.3 TS 에서 이미 존재하는 차이

| 항목 | 찾기 위젯 (Monaco) | 검색 패널 (`taide-search`) |
| --- | --- | --- |
| 전후방 탐색, 후방 참조 | 가능 | 컴파일 오류 |
| `\d`·`\w`·`\b` | ASCII 기준 | 유니코드 기준 |
| 줄을 넘는 일치 | `\n` 등이 있으면 가능 | 불가능 |
| 단어 단위 | 구분자 목록 | 영숫자·`_` 판정 |
| 정규식 아닌 대소문자 무시 | 유니코드 폴딩 | ASCII 만 |
| 바꾸기의 `$1` | 정규식 모드에서 지원 | 지원 안 함 |
| 일치 한도 | 19999(강조), 연산은 전체 | 10,000 |

### 4.4 통일 선택지

| 선택지 | 내용 | 평가 |
| --- | --- | --- |
| X. 두 방언 유지 | 위젯은 JS 방언, 패널은 Rust `regex` 방언. TS 와 같음 | TS 동작을 그대로 재현한다는 원칙에 부합. 추천 |
| Y. 위젯을 패널 방언에 맞춤 | 위젯 엔진을 `regex` 또는 `fancy-regex` 로 | 패널에서 통하는 패턴이 위젯에서도 같은 의미가 됨. 대신 위젯의 `\d`·`\w`·`\b`·줄 끝 의미가 TS 와 달라지고, `regex` 만 쓰면 전후방 탐색·후방 참조가 사라짐 |
| Z. 패널을 위젯 방언에 맞춤 | `taide-search` 의 엔진을 바꿈 | Tauri 제품과 공유하는 크레이트를 바꾸는 일이고, 선형 시간 보장과 처리량을 잃음. 이번 범위 밖. 비추천 |

## 5. 추천안

### 5.1 추천: JS 방언 유지 + `regress` 0.12.0 (게이트 조건부)

- 구조: 찾기 모델은 `native/taide-native-editor/src/find.rs` 에 두고 정규식 부분만 trait 로 뺍니다. 구현은 `native/taide-native-app` 이 넣습니다. `taide-native-editor` 와 `taide-native-ui` 에는 의존성이 늘지 않으므로 동결된 `taide-remote-web` 의 의존 그래프와 lockfile 이 바뀌지 않습니다. 설계 문서 7절 단계 6 의 권고와 같습니다.
- trait 가 요구하는 것은 2절의 결론대로 최소입니다.

```rust
pub struct FindPatternOptions {
    pub is_regex: bool,
    pub match_case: bool,
    pub multiline: bool,
}

pub struct FindCaptures {
    pub groups: Vec<Option<std::ops::Range<usize>>>,
}

pub trait FindPattern {
    fn captures_at(&self, haystack: &str, start: usize) -> Result<Option<FindCaptures>, FindPatternError>;
}

pub trait FindPatternCompiler {
    fn compile(&self, source: &str, options: &FindPatternOptions) -> Result<Box<dyn FindPattern>, FindPatternError>;
}
```

- 모델이 직접 구현하는 것(엔진과 무관): 여러 줄 판정, 줄 단위·여러 줄 구동, CRLF 위치 보정, 빈 일치 전진, 단어 단위 판정, 한도 19999, 바꾸기 패턴 해석, 대소문자 보존, 모두 바꾸기의 두 경로. 정규식이 아닌 대소문자 구분 검색은 Monaco 처럼 엔진 없이 부분 문자열 탐색으로 처리합니다.
- 앱의 구현: `regress::Regex::with_flags(source, flags)` 에 `u` 를 항상, `i`·`m` 을 조건부로 넘기고 `find_from(text, start)` 의 첫 결과를 돌려줍니다. 정규식이 아닌 대소문자 무시 검색은 Monaco 와 같이 이스케이프한 검색어를 같은 엔진에 `iu` 로 넘깁니다.

근거:

1. 동작 기준이 TS 입니다. Monaco 찾기는 JS 정규식이고, 후보 중 ECMAScript 문법과 의미를 목표로 하는 실행 엔진은 `regress` 하나입니다. `\d`·`\w`·`\b` 의 ASCII 기준, 줄 끝 문자 집합, 후방 탐색, 후방 참조, 이름 있는 그룹, `\u{...}` 가 변환 없이 맞습니다.
2. 방언이 둘인 것은 TS 의 현재 동작입니다(4.3). 위젯을 패널 방언에 맞추는 것은 통일이 아니라 위젯 동작의 변경입니다.
3. 변환 계층이 필요 없습니다. `regex`·`fancy-regex` 로 JS 의미를 내려면 `\d`·`\w`·`\b`·`.`·`$` 를 고쳐 쓰는 패턴 파서가 필요하고, 그래도 문법 오류 조건(짝 없는 `{`, 의미 없는 이스케이프)까지는 맞추기 어렵습니다.
4. 반입 부담이 작습니다. 새 패키지 1개, 새 전이 의존성 0개(예상), MIT OR Apache-2.0, 최초 공개 후 6년, 누적 4551만 다운로드입니다. lockfile 변경은 `native/taide-native-app/Cargo.lock` 한 곳입니다.
5. 엔진이 trait 뒤에 있으므로 게이트에서 탈락하면 구현 하나만 바꾸면 됩니다.

위험과 대응:

- 백트래킹 폭주: 0.12.0 에는 실행 중단 수단이 없습니다. TS 도 같은 조건이라 동작 차이는 아니지만, native 는 편집기와 셸이 한 프로세스이므로 멈춤의 범위가 넓습니다. 모델이 줄과 일치 사이에서 경과 시간을 확인해 중단하는 것은 엔진과 무관하게 가능하고, 한 줄 안에서의 폭주는 막지 못합니다. 대응 수준은 사용자 결정 사항입니다(7절 3번). master 의 `find_from_budgeted` 가 릴리스되면 채택합니다.
- 처리량: 수치를 확인하지 못했습니다. 게이트에서 측정합니다.
- 문서 불일치: 크레이트 문서는 `\p{...}` 가 미구현이라고 적지만 master 의 파서에는 구현이 있습니다. 0.12.0 에서의 실제 동작을 게이트의 첫 항목으로 확인합니다.
- 오류 문구: 입력 검증에 표시되는 문장이 JavaScriptCore 의 문장과 다릅니다. 어느 엔진을 고르든 생기는 차이입니다.

### 5.2 채택 게이트

1. 방언 적합성: 6.2 의 표 테스트가 전부 통과합니다. 실패 항목이 있으면 목록을 보고하고 채택을 보류합니다.
2. Monaco 테스트 이식분(6.1)이 통과합니다.
3. 성능: 30만 줄·20MB 문서에서 일치가 없는 정규식 전체 검색과 19999 개에서 멈추는 검색의 시간을 측정해 보고합니다. 기준값은 측정 결과를 본 뒤 사용자와 정합니다. 근거 없는 수치를 미리 정하지 않습니다.
4. 폭주 패턴(`(a+)+b` 와 긴 `a` 줄)에서의 동작이 7절 3번의 결정과 일치합니다.

### 5.3 차선: `fancy-regex` 0.19.2

- 게이트 1~3 에서 탈락하거나 사용자가 폭주 방어를 필수로 정하면 `fancy-regex` 로 바꿉니다.
- 장점: 전후방 탐색·후방 참조·이름 있는 그룹 지원, 확장 문법이 없는 패턴은 선형 시간, 백트래킹 한도 내장, 패널과 같은 `regex` 문법 계열, 새 패키지 1개(MIT).
- JS 와 달라지는 점과 처리:

| 차이 | 처리 |
| --- | --- |
| `\d`·`\w`·`\b` 가 유니코드 기준 | 변환 계층에서 `\d`→`[0-9]`, `\w`→`[0-9A-Za-z_]`, `\b`→`(?-u:\b)` 로 고쳐 쓰는 방법이 있으나 문자 클래스 안·이스케이프 연속을 구분하는 파서가 필요합니다. 고쳐 쓰지 않고 차이를 문서로 고지하는 쪽이 단순합니다 |
| 여러 줄에서 `^`·`$` 가 `\n` 만 인식, `.` 이 `\r`·U+2028·U+2029 를 포함 | Monaco 가 여러 줄 텍스트를 LF 로 정규화하므로 `\r` 은 영향이 없습니다. U+2028·U+2029 는 차이로 남깁니다 |
| JS 에 없는 문법이 통과(`\A`, `\z`, `(?i)`, `(?>)`, `\K` 등) | 상위 집합이므로 막지 않고 고지합니다 |
| JS 에서 오류인 패턴이 통과하거나 반대 | 고지합니다 |
| 한도 초과 오류 | 입력 검증 영역에 오류로 표시합니다. 새 UI 를 만들지 않습니다 |

- 고지 위치는 `docs/` 의 사용자 문서와 릴리스 노트입니다. 위젯에 새 안내 문구나 버튼을 추가하지 않습니다(새 디자인 금지).

### 5.4 추천하지 않는 선택

- `regex` 단독: 의존성이 늘지 않고 가장 빠르지만 전후방 탐색과 후방 참조가 없어 TS 에서 되던 패턴이 컴파일 오류가 됩니다.
- `ferroni`: 이미 lockfile 에 있고 시간 제한 API 가 있지만 Oniguruma 방언입니다. 기본 문법에서 `^`·`$` 가 항상 줄 경계에 맞고 multiline 옵션이 dotall 을 뜻하는 등 JS·패널 어느 쪽과도 다른 세 번째 방언이 됩니다. 최초 공개가 2026-02-13 인 신생 크레이트이기도 합니다.
- 두 엔진 자동 전환(`regex` 로 컴파일을 시도하고 실패하면 다른 엔진): 패턴에 전후방 탐색이 있느냐에 따라 `\d`·`\w` 의 의미가 바뀝니다.

### 5.5 새 의존성

| 크레이트 | 확인한 최신 버전 | 라이선스 | 왜 필요한가 | 넣는 곳 |
| --- | --- | --- | --- | --- |
| regress | 0.12.0 (2026-08-23) | MIT OR Apache-2.0 | 찾기 위젯의 정규식을 ECMAScript 의미로 실행 | `native/taide-native-app/Cargo.toml` 에 `=0.12.0`. lockfile 은 `native/taide-native-app/Cargo.lock` 만 변경 |

- 전이 의존성은 `memchr`(`^2.4.0`, 이미 2.8.3 으로 고정)뿐입니다. `hashbrown` 은 `alloc` feature 전용 선택 의존성이라 기본 구성에서는 들어오지 않습니다.
- `utf16` feature 는 켜지 않습니다. native 문서는 UTF-8 바이트 위치를 쓰고 그 feature 는 최적화 손실이 있다고 `Cargo.toml` 에 적혀 있습니다.
- 로컬 registry 에 소스가 없으므로 반입할 때 네트워크 내려받기가 한 번 필요합니다.
- 차선으로 바뀌는 경우의 의존성은 `fancy-regex` 0.19.2(MIT)이고 새 패키지는 1개(예상)입니다.

## 6. 구현 시 테스트 전략

### 6.1 Monaco 테스트 이식

- Monaco npm 패키지에는 테스트가 없습니다(`node_modules/monaco-editor/esm/vs/editor/contrib/find` 에는 `browser` 만 있음). 원본은 VS Code 저장소(MIT)에 있습니다.
    - `src/vs/editor/test/common/model/textModelSearch.test.ts`: 약 50개 이상. 여러 줄 찾기, `^`·`$`·`^$`·`.*`, 캡처 포함 찾기, 이전·다음 찾기, `isMultilineRegexSource`, "\n matches \r\n", "\r can never be found", "\W should match line break"(#53415), "\d* finds empty string and stops searching"(#74715), "Zero-length matches should properly step over surrogate pairs"(#100134), 유니코드 이스케이프.
    - `src/vs/editor/contrib/find/test/browser/findModel.test.ts`, `replacePattern.test.ts`, `findController.test.ts`, `find.test.ts`.
- 이식 기준 커밋은 Monaco 0.56.0 의 `vscodeCommitId` `f487add297079a02eb836810185b165e50cadabc` 입니다. 이식한 테스트 파일에는 원본 경로와 커밋을 `docs/` 에 기록합니다. 출처 표기 방식은 저장소의 기존 합의를 따릅니다.
- 위치와 단위 차이를 먼저 정리합니다. Monaco 의 열은 UTF-16 단위 1 기준이고 native 는 바이트 범위입니다. 테스트 헬퍼에서 한 번만 변환합니다.

### 6.2 방언 적합성 표 테스트 (앱 쪽 구현 대상)

엔진을 바꿔도 그대로 돌릴 수 있도록 `FindPatternCompiler` 구현에 대해 표 형태로 작성합니다.

| 묶음 | 사례 |
| --- | --- |
| 문자 클래스 | `\d` 가 전각 숫자·아라비아 숫자에 맞지 않음, `\w` 가 한글·`é` 에 맞지 않음, `\b` 가 `한a` 사이에서 경계, `i` 와 함께 `\w` 가 U+017F·U+212A 에 맞음 |
| 확장 문법 | `(?<=a)b`, `(?<!a)b`, 가변 길이 후방 탐색, `(a)\1`, `(?<n>a)\k<n>`, `\p{L}`, `\p{Script=Hangul}`, `\u{1F600}`, `.` 이 서로게이트 쌍 문자 하나에 맞음 |
| 오류가 되어야 하는 패턴 | `(`, `[`, `a{`, `\-`(클래스 밖), `(?<n>a)(?<n>b)`(같은 경로) |
| 여러 줄 | `m` 에서 `^`·`$` 가 `\n` 앞뒤에 맞음, `.` 이 `\n` 에 맞지 않음, U+2028 처리 |
| 시작 위치 | `captures_at(text, start)` 에서 후방 탐색이 `start` 앞 글자를 봄, `^` 가 `start` 가 아닌 문자열 처음에만 맞음 |
| 대소문자 무시 | `ß`·`ẞ`, `K`·U+212A, `Σ`·`σ`·`ς`, 터키어 `İ`·`ı` |

기대값은 ECMAScript 사양과 MDN 을 근거로 적고, 가능하면 TS 제품의 찾기 위젯에서 같은 패턴을 실행해 확인한 결과를 QA 문서에 남깁니다.

### 6.3 모델 순수 테스트 (엔진과 무관)

- 한도: 일치 20000 개 문서에서 개수 19999 와 `+` 표시 조건, 한도 도달 시 다음·이전 탐색이 모델 직접 탐색으로 넘어가는지, 순환 옵션과 `canNavigateBack`·`canNavigateForward`.
- 빈 일치: `^`, `$`, `\d*`, 서로게이트 쌍 건너뛰기.
- 단어 단위: 구분자 목록, `_`, 한글, 일치 첫 글자가 구분자인 경우.
- CRLF: 여러 줄 일치의 위치 보정, `\n` 이 CRLF 줄 끝에 맞고 `\r` 이 맞지 않음.
- 바꾸기 패턴: 2.5 의 표 전부, `$10` 의 뒤 자리 떼기, 참여하지 않은 그룹, 대소문자 지시 중첩(`\u\L$1`).
- 대소문자 보존: 2.6 의 규칙 순서, `-`·`_` 조각.
- 모두 바꾸기: 일반 경로와 대량 경로의 경계(19998, 19999), 7절 4번 결정에 따른 대량 경로의 동작.
- 시드: 정규식 모드에서 선택 영역이 이스케이프되는지, 여러 줄 선택은 시드하지 않는지.

### 6.4 성능·폭주

- 5.2 의 3·4번을 재현 가능한 벤치로 남깁니다. 대상 문서 생성 스크립트와 측정값은 `docs/quality-assurance` 에 기록합니다.
- 전체 테스트 대상은 메인이 배치 끝에 한 번 실행하므로, 작업자는 `find` 관련 테스트 대상만 빌드합니다.

## 7. 사용자 결정이 필요한 사항

1. 찾기 위젯의 방언과 엔진 — **A안(추천: TS 와 같은 JS 방언, 변환 계층 불필요): `regress` 0.12.0 을 게이트 조건부로 채택, 탈락 시 B안** / B안: `fancy-regex` 0.19.2(패널과 같은 문법 계열, 폭주 한도 내장, JS 와의 차이는 고지) / C안: 기존 `regex` 만(의존성 0, 전후방 탐색·후방 참조 없음) / D안: `ferroni`(의존성 0, Oniguruma 방언)
2. 새 의존성 반입 — **A안(추천): 1번에서 고른 크레이트 1개를 `native/taide-native-app` 에 정확한 버전으로 고정해 반입** / B안: 반입하지 않음(1번은 C안 또는 D안으로 제한)
3. 백트래킹 폭주 대응 수준 — **A안(추천: TS 와 같은 조건이며 추가 설계가 가장 작음): 모델이 줄·일치 사이에서 경과 시간을 확인해 중단하고, 한 줄 안의 폭주는 TS 와 같이 수용. regress 의 budget API 가 릴리스되면 적용** / B안: 폭주 방어를 채택 필수 조건으로 두고 1번을 B안으로 / C안: 검색을 작업 스레드로 옮김(TS 에 없는 구조이며 중단할 수 없는 스레드가 남을 수 있음)
4. Monaco 대량 모두 바꾸기의 특이 동작(일치 19999 이상에서 `m` 플래그 추가, 단어 단위 무시, 2.7) — **A안(추천: 원칙대로 TS 동작 재현): 그대로 재현하고 `docs/` 에 기록** / B안: 일반 경로와 같은 결과가 되도록 고침
5. 검색 패널과의 방언 통일 — **A안(추천: TS 와 같음): 통일하지 않음** / B안: 위젯을 패널 방언에 맞춤(1번 B안 또는 C안과 묶임)
6. 정규식 오류 문구 — **A안(추천: Monaco·패널과 같은 방식): 엔진의 영어 문장을 그대로 표시** / B안: 번역 키로 감싸 "잘못된 정규식입니다: …" 형태로 표시

## 8. 확인하지 못한 사항

- `regress` 0.12.0 의 처리량과 대형 문서 검색 시간. 벤치를 실행하지 않았습니다.
- `regress` 0.12.0 에서 `\p{...}` 가 실제로 동작하는지. 크레이트 문서(미구현)와 master 파서(구현)가 어긋나며 0.12.0 소스는 열람하지 않았습니다.
- `regress::Regex::find_from(text, start)` 에서 후방 탐색이 `start` 앞 글자를 보는지, `^` 가 `start` 에서 맞지 않는지.
- `regress` 의 대소문자 무시 결과가 JavaScriptCore 와 모든 문자에서 같은지.
- `regress` master 의 `find_from_budgeted` 가 언제 릴리스되는지.
- `regress` 가 `\A`·`\z`·`\Z` 버퍼 경계 단정을 지원하는지, JavaScriptCore 가 지원하는지.
- Boa 가 `regress` 를 쓴다는 것은 검색 결과로만 보았고 Boa 의 `Cargo.toml` 은 확인하지 않았습니다.
- `fancy-regex` 가 `\/` 같은 비메타 구두점 이스케이프, `\cX`, `[^]`, `\0` 등 JS 전용 표기를 어떻게 처리하는지.
- `ferroni` 의 `\d`·`\w`·`\b` 기본 범위와 ASCII 제한 옵션. 문법 플래그 정의만 읽었습니다.
- 새 패키지 수. cargo 를 실행하지 않았고 lockfile 대조로만 예상했습니다. `fancy-regex` 는 `regex-automata` 의 feature 구성이 달라질 수 있습니다.
- TS 제품이 지원하는 macOS 최소 버전과 그 WebKit 의 정규식 지원 범위. Windows·Linux 웹뷰는 조사하지 않았습니다.
- VS Code 테스트 파일은 main 브랜치에서 존재와 테스트 이름만 확인했습니다. 커밋 `f487add2…` 시점의 내용은 열람하지 않았습니다.
- Monaco 대량 모두 바꾸기의 특이 동작(2.7)은 코드 읽기 결과이며 실행으로 재현하지 않았습니다.
- 검색 패널의 검색어·옵션을 편집기 찾기 위젯으로 넘기는 연동은 `src` 검색 범위에서 찾지 못했습니다. 없다고 단정하지는 않습니다.
- `regex` 의 `size_limit`·`dfa_size_limit` 기본값 수치.
- ECMAScript 열의 일부 항목(유니코드 모드에서 짝 없는 `{`·의미 없는 이스케이프가 오류라는 점, `^`·`$`·`.` 의 줄 끝 문자 집합 `\n`·`\r`·U+2028·U+2029)은 사양 규칙으로 적었고 이번 조사에서 해당 MDN 페이지 본문으로 재확인하지는 못했습니다. `\d`·`\w` 의 정의는 MDN 으로 확인했습니다.

## 9. 출처

저장소 안:

- `node_modules/monaco-editor/esm/vs/editor/common/model/textModelSearch.js`, `textModel.js`
- `node_modules/monaco-editor/esm/vs/editor/contrib/find/browser/{findModel.js, findController.js, findWidget.js, findState.js, findDecorations.js, replacePattern.js}`
- `node_modules/monaco-editor/esm/vs/base/common/{strings.js, search.js}`
- `node_modules/monaco-editor/esm/vs/editor/common/core/{wordHelper.js, wordCharacterClassifier.js}`, `editor/common/config/editorOptions.js`
- `src/shared/lib/monaco/monaco-actions.ts`, `src/widgets/editor-area/editor-area.tsx`, `src/entities/search/use-search-run.ts`
- `crates/taide-search/{Cargo.toml, src/service.rs}`, `crates/taide-model/src/search.rs`, `crates/taide-locale/resources/locales/{ko,en}.json`
- `native/taide-native-app/{Cargo.toml, Cargo.lock}`, `native/taide-native-editor/{Cargo.toml, src/store.rs}`
- cargo registry 소스: `ferroni-1.8.1/{README.md, Cargo.toml, src/api.rs, src/regint.rs, src/regsyntax.rs}`, `ropey-1.6.1/src/{slice.rs, rope.rs}`

외부(2026-10-07 조회):

- regex: https://crates.io/api/v1/crates/regex , https://docs.rs/regex/1.13.1/regex/ , https://docs.rs/regex/1.13.1/regex/struct.Regex.html , https://docs.rs/regex/1.13.1/regex/struct.RegexBuilder.html
- fancy-regex: https://crates.io/api/v1/crates/fancy-regex , https://crates.io/api/v1/crates/fancy-regex/0.19.2/dependencies , https://docs.rs/fancy-regex/0.19.2/fancy_regex/ , https://docs.rs/fancy-regex/0.19.2/fancy_regex/struct.RegexOptionsBuilder.html , https://docs.rs/fancy-regex/0.19.2/fancy_regex/struct.Regex.html , https://raw.githubusercontent.com/fancy-regex/fancy-regex/main/README.md
- regress: https://crates.io/api/v1/crates/regress , https://crates.io/api/v1/crates/regress/0.12.0/dependencies , https://docs.rs/regress/0.12.0/regress/ , https://docs.rs/regress/0.12.0/regress/struct.Regex.html , https://docs.rs/regress/0.12.0/regress/struct.Flags.html , https://docs.rs/regress/0.12.0/regress/all.html , https://github.com/ridiculousfish/regress , https://github.com/ridiculousfish/regress/releases , https://raw.githubusercontent.com/ridiculousfish/regress/master/Cargo.toml , https://raw.githubusercontent.com/ridiculousfish/regress/master/src/parse.rs , https://raw.githubusercontent.com/ridiculousfish/regress/master/src/api.rs , https://raw.githubusercontent.com/ridiculousfish/regress/master/src/classicalbacktrack.rs
- ferroni: https://crates.io/api/v1/crates/ferroni
- regex-cursor: https://crates.io/api/v1/crates/regex-cursor , https://docs.rs/regex-cursor/0.1.5/regex_cursor/
- 제외 후보: https://crates.io/api/v1/crates/pcre2 , https://crates.io/api/v1/crates/oxc_regular_expression
- ECMAScript 의미: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Character_class_escape , https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Regular_expressions/Input_boundary_assertion , https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/RegExp/multiline
- VS Code: https://code.visualstudio.com/updates/v1_29 , https://code.visualstudio.com/docs/editing/codebasics , https://github.com/microsoft/vscode/tree/main/src/vs/editor/contrib/find/test/browser , https://github.com/microsoft/vscode/blob/main/src/vs/editor/test/common/model/textModelSearch.test.ts
