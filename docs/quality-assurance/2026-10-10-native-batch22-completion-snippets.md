# 배치 22 — 자동완성과 사용자 스니펫 소비

현재 상태: 배치 22의 실제 공급자·본문/peek·스니펫 세션·본문 미리보기·후보/상세 창 크기 조절을 연결했습니다. CLIPBOARD 연결 전 제품 상태의 SDK/editor/Syntax/UI/app 전체 143대상을 --no-fail-fast로 직접 1회 실행하고 최초 UI 3/app 2실패의 영향만 재검사했습니다. 서로 다른 1690건이 통과했으며 기존 성능 ignored 4·보호 Trash 제외 3과 실기/출시 게이트를 구분합니다. c 완료, d/e/f/g 미완료입니다. CLIPBOARD 변수의 앱 공급을 연결하고 completion-clipboard-final.log의 관련 46건과 completion-clipboard-actual-app.log의 실제 앱 1건을 확인했습니다. 파일/색 후보 표시 대조와 남은 구현 후의 배치 완료 게이트가 남아 두 기능 행은 partial입니다. 마지막 배치는 33 계획이며 전체 전환율/잔여 시간은 미산정입니다. main 직접 수행·서브에이전트/workflow 없음·Cargo/fmt는 종료 확인 뒤 직렬로 실행합니다.

## 체크리스트

- [x] a. TS/Monaco·공식 typed API·후보/자동 trigger/필터/삽입/키/테마·native 공급 대조
- [x] b. 후보/범위/UTF-16·일반/스니펫 삽입·요청 교체/취소·빈/오류 모델 검증
- [x] c. 실제 typed LSP·사용자/플러그인 스니펫/원본 단어 공급·프로젝트/본문/peek 수명
- [ ] d. 원본 후보 popup/문서/preview·명령/직렬 입력·선택/수락/취소·snippet session 소비
- [x] d1. CLIPBOARD 실제 앱 비동기 공급·값 재사용·다중 커서·기본값·취소/늦은 응답·읽기 실패 알림·원문/종료 보존
- [ ] e. 실제 앱·dirty/readonly/두 뷰·언어/서버/종료·입력 소유 회귀와 실기 부채 구분
- [ ] f. 변경 전체 대상·동결 host/Wasm·포맷/diff·디스크/보호 확인
- [ ] g. 실제 QA/기능표/완료 근거/PROCESS·선별 커밋·일반 푸시·다음 범위

## 범위와 보호 경계

실제 TS completion adapter·사용자 snippet provider/설정·code-editor 옵션과 Monaco suggest의 후보 목록·필터/정렬·선택/수락·취소·문서/preview·키를 기존 native typed LSP/본문/peek/스니펫 session에 연결합니다. 새 기능/디자인·원본 버그/내부 수치 강제 재현·새 engine/패키지 추가·동결 browser 변경·OS 합성 입력은 하지 않습니다.

기존 TextMate 엔진·실제 앱 데이터/OS 설정/clipboard/Keychain/Trash·보호 app bundle·TS/Tauri/Monaco/xterm을 유지합니다. 테마의 7자리 HEX 결정, 실기 pixel/IME/접근성·대형 성능/soak/보조 창/패키징과 나머지 LSP/AI/출시는 잔여 전체 목표입니다. 변경 없는 batch21의 성공 근거만 재사용하며 변경 크레이트는 전체 대상을 직접 실행합니다. batch21에서 기존 child 초기화 시간 초과가 관찰돼 다음 전체 검증은 테스트 내부 동시 실행을 제한하고 실제 결과를 기록합니다.

## 현재 근거

batch21의 121대상/1412건·최초 실패와 영향 재검사·동결/포맷/경계·디스크 623GiB/66%는 별도 QA에 보존했습니다. 자동완성/스니펫의 실제 본문 소비 완료 근거로 전용하지 않습니다. TS adapter/사용자 snippet provider·code-editor의 suggest preview 설정은 범위 선택 때 읽었으며 상세 Monaco 계약과 현재 native API를 a에서 대조합니다.

## 실제 원본과 공식 API

TS `src/shared/lib/lsp/adapters/completion.ts`·`initialize-params.ts`·`snippet-completion.ts`와 `src/features/editor/code-editor.tsx`를 읽었습니다. 설치된 Monaco의 suggest.js/completionModel/suggestModel/suggestController/suggestWidget/renderer/details/기본 옵션·word provider/worker와 filters.js/strings.js, 공식 설치 `lsp-types 0.97.0/src/completion.rs`의 typed response/item/edit/options/params를 대조했습니다. native의 후보 수집기·catalog·언어 단어 규칙/토큰·snippet parse/normalize/whitespace/variables/expansion/insertion/session·change journal·문서/뷰/UTF-16 API를 확인했습니다. 새 parser/regex/구문 engine 의존성은 필요하지 않습니다.

TS completion은 list/array/null을 받고 label/kind/detail/documentation·textEdit.newText→insertText→label·snippet 형식·sortText/filterText와 range를 전달합니다. 기본 범위는 현재 단어 시작부터 커서까지입니다. 요청에는 context를 보내지 않지만 initialize에는 snippetSupport/contextSupport를 선언합니다. 정확한 언어로 등록된 사용자 snippet은 해당 언어 json/global scope·다중 prefix·이름/detail·description·기존 body를 공급합니다. LSP와 같은 provider 우선순위 그룹이며 wildcard 단어 provider는 해당 그룹에 후보가 없을 때 소비합니다. 완료 응답의 isIncomplete와 원본에서 낮춘 문서 종류/typed 범위는 버그 강제 재현 불필요 지시에 따라 정상 수명과 표시 모델에 보존합니다. 원본 adapter가 전달하지 않는 resolver/추가 edit/command 등은 typed 데이터와 실제 소비 필요성을 대조하며 새 UI를 임의로 추가하지 않습니다.

quick suggestion 기본은 10ms trailing·other만 활성이고 comments/strings는 꺼집니다. inline completion 표시/공급의 억제 규칙을 보존해야 하며 native AI 공급은 아직 별도 잔여입니다. 숫자 단어·커서가 단어 중간인 경우는 자동 trigger에서 제외하되 한 글자를 기존 단어 앞에 입력한 경우는 허용합니다. trigger 문자는 마지막 입력 문자를 보고 해당 provider만 재요청합니다. 조합 중 필터링을 보류하고 조합 종료 후 갱신합니다. 명시 popup Loading은 50ms 뒤이며 자동 빈 결과는 숨기고 명시 빈 결과는 Empty 상태를 표시합니다.

후보는 sortText 소문자→label UTF-16→Monaco kind 순으로 초기 정렬하고 현재 접두사/공급 filterText의 fuzzy·단어 시작/camel/separator·약한 첫 일치 제외·인접 오타를 소비합니다. filterText가 label과 다르면 별도 label 강조를 계산하며 기존 팔레트 matcher의 단순 subsequence를 그대로 대용할 수 없습니다. snippetSuggestions는 inline, selection은 first, localityBonus는 false입니다. tabCompletion은 off이고 이미 열린 popup의 Tab/Enter만 수락합니다. snippet/plain text 삽입 모두 기존 snippet 삽입/공백/undo 수명을 사용하며 plain `$` 구문을 스니펫으로 해석하면 안 됩니다.

본문 명시 키는 Ctrl+Space(Windows/Linux), mac Ctrl+Space/Alt+Escape와 Mod+I입니다. 목록에서 Up/Down·Mod 변형/mac Ctrl+P/N·PageUp/Down, Enter/Tab·Shift 대체 수락, Escape/ShiftEscape를 소비합니다. 명시 키를 다시 실행하면 선택/상세 문서를 토글합니다. 기본 상세 문서는 닫혀 있고 종류 아이콘·label 강조/detail·선택색은 editorSuggestWidget 테마를 사용합니다. 기본 폭 430·12개 줄, 글꼴/줄 높이는 editor 값을 사용하고 화면 여유/내용·원본 resize에 맞춰 제한합니다. suggest preview는 TS 설정을 소비하며 실제 수락 전 문서를 수정하지 않습니다.

단어 후보는 현재 문서를 먼저 보고 열린 같은 언어의 sync 가능한 문서를 수집하며 현재 단어/숫자를 제외하고 중복을 제거합니다. 원본의 10000개 상한을 넘긴 한 개까지 수집하는 버그는 강제 재현하지 않습니다. inline 공급이 있고 활성인 경우 기본 wordBasedSuggestions를 억제합니다. 기존 앱 전체 snippet catalog는 snapshot/cached files·60초 신선도·편집 invalidation·owner/종료 수명을 이미 제공하므로 이를 본문 공급에 재사용합니다. 실제 사용자의 clipboard는 검사에서 읽거나 쓰지 않습니다.

## 삽입과 후보 필터의 현재 검증

`Candidate::prepare`는 현재 문서 ID/revision·주 커서와 같은 접두/접미사 범위를 확인한 뒤 일반 텍스트의 `$`를 Marker::Text로 보존합니다. 스니펫은 기존 parse_complete·변수 평가·각 삽입 위치의 whitespace/EOL·총량 제한을 사용하며, 준비 도중 오류가 나면 문서를 수정하지 않습니다. 기존 insert의 readonly/IME/선택/세대/겹침 검증과 분리 undo, Session의 choice/mirror/tabstop을 재사용합니다. clipboard와 파일/시간/작업공간·정규식 transform의 실제 앱 resolver는 남아 있습니다.

삽입 검사 `cargo test --test completion-insertion --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`는 `/private/tmp/taide-batch22-completion-insertion-after-original-indent.log`에서 exit 0·5통과/0실패/0ignored입니다. 일반 `$`·한글 다중 커서/다른 접두사·공유뷰/undo/redo, 스니펫 커서 번호·choice/mirror/다음 tabstop, 커서별 들여쓰기/CRLF, 변수 오류/총량/readonly/IME/선택/오래된 준비물 거절을 확인했습니다. 최초 4통과/1실패는 검사 기대값의 탭 계산 오류였습니다. Monaco 원본 normalizeIndentation을 직접 실행해 공백 2개+탭+한글이 공백 4개+한글이 되는 것을 확인하고 기대값만 수정했습니다.

`completion-filter.rs`는 원본 filters.js의 UTF-16 점수/첫 강한 일치·camel/구분자/대소문자·인접 오타·anyScore/연속 강조를 engine 없이 제공합니다. 요청별 Scorer가 행렬을 재사용하며 새로운 정규식/구문 의존성을 추가하지 않습니다. `docs/utils/2026-10-10-monaco-completion-filter-oracle.js`가 원본 normal/graceful/any scorer를 직접 실행한 8100개의 점수/강조 위치 표본을 TSV로 저장합니다. 원본 MIT 고지를 기존 LICENSE-MONACO-SNIPPET과 THIRD_PARTY_LICENSES에 연결했습니다.

`cargo test --test completion-filter --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`는 `/private/tmp/taide-batch22-completion-filter-first.log`에서 exit 0·1통과/0실패/0ignored이며 8100표본의 점수/UTF-16 강조가 일치했습니다. 현재 검사는 원본의 빈/공백 접두사·경계/camel/오타·한글/조합 문자/서로게이트/확장 lowercase/Greek sigma·길이 경계·시작 위치/옵션을 포함합니다. 실제 공급 후보의 초기 정렬/현재 입력 refilter·원본 2000개 기준의 graceful 선택과 앱 수명/목록 UI는 아직 연결하지 않았습니다. core fmt는 `/private/tmp/taide-batch22-completion-model-fmt.log`에서 exit 0이며 같은 의미 변경 상태의 성공 검사를 반복하지 않습니다.

## 문서별 completion 옵션의 현재 검증

SDK SessionSnapshot은 기존 문서별 signature 옵션과 함께 typed CompletionOptions의 정적+동적 값을 제공합니다. phase/generation/capability revision/열린 문서 키가 같으면 Arc 캐시를 보존하고 동적 해제·닫힘·runner 종료에서 해당 값을 회수합니다. 두 typed 소비자가 생겨 기존 내부 옵션 수집을 같은 generic 경계로 공유했으며 의존성/lock을 바꾸지 않았습니다. 앱의 기존 SessionSnapshot 검사 fixture 두 곳에 새 필드를 반영했습니다. 실제 앱 빌드/child 공급·trigger UI는 아직 미검증입니다.

`cargo test --manifest-path crates/taide-lsp/Cargo.toml --lib trigger --locked --offline --target-dir experiments/native-shell-spike/target`는 `/private/tmp/taide-batch22-completion-signature-options.log`에서 exit 0·4통과/0실패/0ignored입니다. 새 completion의 정적/문서 selector/동적 trigger·commit·label details·resolve/캐시/해제/닫힘/종료와 기존 signature 회귀를 확인했습니다. 잘못된 completion 옵션 거절 검사는 기존 CompletionRegistrationOptions의 flatten 검증으로 이미 통과해 중복 validator를 추가하지 않았습니다. 최초 SDK 실행을 app workspace의 dependency에 -p로 지정한 명령은 dev-dependency 검사 불가로 exit 101이었고 SDK manifest 경계로 수정했습니다. 해당 실행은 테스트 성공으로 계산하지 않습니다.

## 앱 요청 상태와 실제 child 검증

다음 공급 작업의 추가 원본을 확인했습니다. TS bootstrap-snippets는 앱 수명 동안 list query observer를 유지하며 사용자 파일의 후보를 정확한 언어 ID로 등록하고, 플러그인이 추가한 언어 ID에도 같은 provider를 등록합니다. 기존 native SettingsViews::snippet_catalog의 Arc snapshot·초기 needs_read·60초 신선도/편집 invalidation·기존 앱 host 읽기/회수 경계를 재사용합니다. 원본에 없는 별도 플러그인 snippet body 공급은 임의로 추가하지 않습니다.

Monaco word provider는 현재 문서를 먼저 두고 열린 같은 언어의 sync 가능한 문서들을 읽습니다. sync 상한은 byte 크기가 아닌 모델 UTF-16 길이 50*1024*1024이며 readonly tier와 별개입니다. 실제 단어는 현재 언어의 word definition으로 줄별 순회하고 JS Number로 숫자로 해석되는 단어·현재 단어·중복을 제외합니다. 최대 10000개로 수집하며 원본의 10001개 버그는 강제 재현하지 않습니다. 원본 word 후보는 커서까지 insert 범위와 전체 현재 단어 replace 범위를 구분하므로 LSP/사용자 snippet의 기본 범위와 혼동하지 않습니다. 현재 syntax MonacoLanguage는 내부 JsRegex::ranges를 사용하고 앱에서 공개 monaco_language로 같은 규칙을 조회할 수 있습니다. 엔진은 syntax 경계에 유지하고 배경 작업의 취소/회수와 후보 상한을 연결해야 하며 아직 해당 소비자를 구현하지 않았습니다.

`editor-completion.rs`의 요청 모델은 프로젝트·본문/peek source·owner/view key/document·선택·문서 ID/key/revision/언어·provider owner/generation/capability revision·viewport·요청 token/watch를 보존합니다. 요청 교체·편집/선택/IME·owner 폐기·프로젝트/provider 변경·창/활성 탭 해제·닫힘/종료는 이전 watch를 취소합니다. 후보는 provider 순서와 isIncomplete를 유지하며 외부 provider·중복/오래된 응답·실패의 기존 후보 재사용을 거절합니다. 목록 UI의 입력 중 refilter/취소·snippet session과 아직 연결하지 않았습니다.

`cargo test --lib editor_completion::tests --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`는 `/private/tmp/taide-batch22-completion-request-after-store-boundary.log`에서 exit 0·6통과/0실패/0ignored입니다. 최초 컴파일은 SelectionSet::validate가 core 내부 API여서 거절됐고, store가 이미 검증한 선택에서 공개 primary 항목을 확인하는 경계로 수정했습니다. 이전 Snapshot 필드 추가 상태의 app check --tests는 `completion-options-app-check.log`에서 exit 0이며 새 요청/child는 이후 lib 검사 빌드로 검증했습니다. 전체 앱 대상의 현재 성공으로 확대하지 않습니다.

LspBridge/실제 typed Completion 요청·프로젝트 mirror·독립 transient task·watch/future drop cancel·응답/활성 탭 gate·닫힘/종료 회수를 연결했습니다. 초기화에는 TS의 snippetSupport/contextSupport와 SDK가 구현한 동적 completion 등록 지원을 선언합니다. 요청 context는 TS처럼 보내지 않고, 정적/문서별 옵션을 현재 provider identity와 함께 공급합니다. 실제 목록을 여는 UI 소비자는 아직 연결하지 않았으며 non-test 빌드의 새 미소비 경고는 UI 연결 때 제거할 범위입니다.

`cargo test --lib lsp::editor_completion_tests --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -- --test-threads=1`는 `/private/tmp/taide-batch22-completion-child-after-root.log`에서 exit 0·3통과/0실패/0ignored(검사 2.98초)입니다. 실제 child의 list/array·스니펫/textEdit 우선순위·UTF-16·Markup/plain/opaque data·미지 numeric kind의 Text fallback·isIncomplete·옵션/프로젝트 격리, 빈/null/서버 오류/형식 오류/미지원, 보류→편집 취소→실제 cancel/늦은 응답과 종료 task 0을 확인했습니다. 보호 OS 입력과 실제 사용자 데이터는 사용하지 않았습니다.

최초 child 검사 0통과/3실패는 새 mock 모드를 기존 프로젝트 root 판정에 포함하지 않아 fixture가 초기화를 거절한 오류였습니다(`completion-child-first.log`, Degraded/TransportClosed, stderr의 fixture requires valid roots and client capabilities). mock의 해당 판정을 수정한 뒤 `completion-mock-after-root.log`에서 build exit 0과 실패 영향 3건의 재검사를 확인했습니다. 기존 17 Wry/큰 __eh_frame 경고는 남아 있으며 검사 timeout이나 production gate를 완화하지 않았습니다. 현재 디스크 여유는 619GiB·사용 67%입니다. 전체 대상/동결 host/Wasm·최종 fmt/diff·실제 본문/peek UI·대형 성능/실기 검증은 아직 남아 있습니다.

## 사용자 스니펫과 단어 공급의 현재 검증

`editor-completion-supply.rs`는 기존 snippet catalog의 실제 언어 파일·전역 scope·플러그인 언어 ID·다중 prefix·이름/설명/본문을 typed 후보로 변환합니다. 실제 TS `register-plugin-languages.ts`와 모델의 플러그인 계약에는 별도 스니펫 본문이나 단어 정규식 기여가 없습니다. 등록된 플러그인 언어의 사용자 파일을 동일하게 소비하고, 별도 공급을 추측해 추가하지 않습니다.

단어 후보는 현재 문서 우선·문서 생성 순서와 같은 언어의 열린 뷰를 사용합니다. 뷰가 없는 호버 코드 도우미 문서는 제외합니다. 원본의 현재 언어 word definition·줄별 수집·현재 전체 단어/JavaScript 숫자 제외·중복 제거·insert/replace 범위를 유지하며, 후보 10,000개와 문서 50Mi UTF-16 상한을 적용합니다. 원본의 10,001개 오차는 재현하지 않습니다. 기존 앱 전용 TextMate 의존성의 `JsRegex`에 중단 가능한 방문을 추가해 줄 전체의 모든 일치를 먼저 담지 않고, 소유된 blocking 작업에서 줄/일치 사이에 요청 취소를 확인합니다. editor/UI에 engine·의존성을 추가하지 않았습니다.

새 앱 요청은 같은 줄의 커서 앞/뒤 전체 단어를 검증하고 LSP/사용자 스니펫의 커서까지 insert 범위와 단어 공급의 전체 replace 범위를 구분합니다. 단어 중간 요청이 거절되는 현상은 `/private/tmp/taide-batch22-completion-middle-word-first.log`에서 exit 101·1실패로 재현했습니다. 이후 `/private/tmp/taide-batch22-completion-supply-after-filter.log`는 `cargo test --lib 'editor_completion::' --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -- --test-threads=4` exit 0·15통과/0실패/0ignored(검사 2.06초)입니다. 기존 요청 6건과 새 중간 범위 1·공급 8건을 함께 확인했습니다. 실제 LSP/스니펫 그룹이 비었을 때만 단어 fallback을 표시하고, 대기 작업·늦은 결과·종료 task 0을 검증했습니다.

`docs/utils/2026-10-10-monaco-completion-words-oracle.js`는 설치된 실제 `EditorWorker.$textualSuggest`와 `DEFAULT_WORD_REGEXP`, 고정된 Monaco 언어 설정으로 33언어 ID/5가지 leading word의 165사례를 생성합니다. JS Number 숫자 제외 4,680표본도 실제 내장 Number로 생성하며 두 비교 검사가 모두 일치했습니다. 생성기의 최초 출력 폴더 누락을 수정했고, 자료 생성 뒤 Monaco 모듈의 잔여 timer가 프로세스를 유지해 최초 실행을 종료했습니다. 생성기의 종료를 명시한 뒤 Bun 실행 exit 0을 확인했습니다. 처음의 넓은 검사 필터는 미변경 actual child까지 포함하므로 테스트 실행 전 중단(exit 130)하고 위의 앱 상태/공급 범위로 실행했습니다. 중단한 검사는 성공 근거로 계산하지 않습니다.

실제 임시 스니펫 파일의 host 조회·캐시·저장/변경/삭제·수명 검사에 typed 자동완성 후보 소비를 추가했습니다. `/private/tmp/taide-batch22-completion-snippet-host.log`의 `cargo test --lib 'snippet_host_tests::snippet_전역_catalog' --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -- --test-threads=1`는 exit 0·1통과/0실패/0ignored(검사 0.04초)입니다. before/after 스니펫 본문·prefix와 삭제 후 빈 후보, 실제 host의 종료 회수를 확인했고 실제 사용자 데이터·OS clipboard/합성 입력은 사용하지 않았습니다. source/owner 요청 모델과 앱 frame의 word reply 수명은 연결했지만 실제 본문/peek에서 목록을 여는 소비자는 아직 남아 있습니다. c/d와 현재 기능 대응표를 완료로 올리지 않습니다.

기존 정규식 범위 소비자의 영향 검사는 `/private/tmp/taide-batch22-completion-word-language-regression.log`의 `cargo test --test language-configuration --manifest-path native/taide-native-syntax/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`에서 exit 0·7통과/0실패/0ignored(검사 0.03초)입니다. 원본 23언어의 정규식·단어 범위/Enter/들여쓰기/괄호/접기 기준이 유지됩니다. editor/syntax/app Cargo fmt 세 명령은 모두 실제 종료 exit 0, 소유된 tracked 파일 diff check exit 0, 새 생성기/QA Prettier exit 0을 확인했습니다. 변경 manifest/lock/frozen diff는 없고 디스크 여유는 615GiB·사용 67%입니다. 이는 전체 대상/host/Wasm 통합 게이트를 대신하지 않습니다.

다음 소비에서는 원본 CompletionModel의 최초 정렬·현재 줄/커서 증분에 따른 후보별 overwrite/refilter와 incomplete provider 재요청을 연결해야 합니다. 현재 요청 모델의 strict revision/selection 만료는 늦은 응답 보호이며, 화면 입력 중 완료된 후보를 그대로 재사용하는 소비까지 구현됐다는 뜻이 아닙니다. 기존 change journal·범위 검증과 원본 SuggestModel을 대조해 삽입 범위를 다시 검증하고, 오래된 revision 검사를 우회하지 않습니다.

## 목록의 초기 정렬과 입력 문맥 필터 검증

core `completion-model.rs`는 후보별 overwrite 길이·원본 종류 순서·UTF-16 라벨 정렬·filterText 점수와 라벨 강조·공백/빈 입력·현재 입력 증분을 소비합니다. 증가하는 입력은 직전 결과를 재필터링하고 줄 문맥이 바뀌거나 감소하면 전체를 평가합니다. 현재 source가 2,000개를 넘으면 원본처럼 일반 scorer를 쓰고, 줄어든 증분 source는 graceful scorer로 전환합니다. 생성기는 실제 `CompletionModel`과 설치된 원본 후보 constructor/comparator 본문을 그대로 실행하며, 알고리즘을 복제해 기대값으로 쓰지 않습니다. DOM에 의존하는 suggest 모듈의 최초 직접 import는 Bun에서 `window is not defined`로 종료돼 해당 순수 소스 경계를 읽어 실행했습니다. 브라우저/OS 입력이나 새 환경 의존성을 추가하지 않았습니다.

`docs/utils/2026-10-10-monaco-completion-model-oracle.js`의 5시나리오/17문맥 변경은 일반/스니펫/종류 tie·raw UTF-16 순서·sortText case·후보마다 다른 overwrite·filterText/라벨 강조·Unicode·2010개 공급과 증분 graceful 전환을 비교합니다. 정렬 값이 일부에만 있는 경우의 비추이 comparator는 원본 버그를 강제 재현하지 않는 지시에 따라 LSP 계약의 누락/빈 sortText→label로 보완해 일관된 전체 순서를 만듭니다. 모든 후보에 정렬 값이 없을 때의 raw 라벨 순서는 보존하며, 혼합 3후보의 6입력 순열에서 동일한 순서를 검증했습니다.

`/private/tmp/taide-batch22-completion-ranking-first.log`의 `cargo test --test completion-model --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`는 3통과/1실패였습니다. 기존 prefix가 전부 지워진 후보의 길이를 음수 대신 0으로 제한해 빈 입력의 기본 점수를 보존하고, 실패한 비교만 같은 target의 `후보모델의_정렬` 필터로 재실행했습니다. `/private/tmp/taide-batch22-completion-ranking-after-prefix-filter.log`는 exit 0·1통과/0실패/0ignored(검사 0.04초)이고 서로 다른 4건이 통과했습니다. 잘못 입력한 중간 검사 필터는 exit 0이나 실행 0건이므로 성공 근거에 포함하지 않습니다. 이전 revision/잘린 UTF-8 경계 거절·빈 후보·같은 호출의 캐시·원본 후보/문서 불변도 확인했습니다.

앱 State는 원본 provider 그룹과 snippet/word 공급에 따라 소유된 목록 모델을 생성·재사용하고, 요청/응답 세대 교체·word fallback 갱신·닫힘 시 캐시를 만료합니다. `/private/tmp/taide-batch22-completion-supply-ranking.log`의 `cargo test --lib 'editor_completion::supply::tests::completion_실제단어worker' --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -- --test-threads=1`는 exit 0·1통과/0실패/0ignored(검사 0.02초)입니다. pending 상태에는 모델을 표시하지 않고, 같은 요청은 같은 모델을 재사용하며 LSP/snippet의 raw 후보가 있으면 filter 결과가 비어도 word fallback으로 바꾸지 않습니다. 새 요청은 별도 모델, 종료 후는 None, fallback의 실제 강조 0..3을 확인했습니다. 아직 실제 popup/원본 입력·accept/preview·본문/peek 화면은 연결하지 않았으며, 문서가 편집된 뒤의 후보 rebase/incomplete 재요청과 resolver도 미완료입니다. 그 상태에서 c/d와 기능 완료 판정을 올리지 않습니다.

## 후보·다중 커서 범위의 이전 단계 기록

core `completion.rs`의 typed 후보는 list/array/null·isIncomplete·detail/documentation/sortText/filterText/opaque data와 원본 삽입 우선순위를 보존합니다. 일반 `$1` 텍스트를 스니펫으로 해석하지 않고 insertTextFormat으로 구분합니다. 단일 줄·같은 시작의 insert/replace 접두 관계, 실제 UTF-16 경계와 요청 커서·문서 ID/revision을 확인하고 잘못된 개별 후보를 제외합니다.

원본 snippetSession의 adjustSelection/createEditsAndSnippetsFromSelections를 추가로 읽었습니다. 다중 커서는 주 커서와 같은 앞/뒤 텍스트만 overwrite를 확장하며 다른 접두사/접미사는 보존합니다. 현재 helper는 스칼라 경계를 넘는 보조 커서 확장을 원래 선택으로 되돌려 Rust가 저장할 수 없는 반쪽 UTF-16 삽입을 만들지 않습니다. 실제 template/변수/공백·삽입/undo/session/choice와 앱 요청 상태·fuzzy·표면은 아직 미완료입니다.

최신 core 검사 9건은 `/private/tmp/taide-batch22-completion-ranges-after-generation.log`에서 exit 0·0실패이며 `cargo test --test completion --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`를 실행했습니다. UTF-16 요청 커서의 반쪽 경계 수용은 `completion-request-boundary-first.log`의 1실패로 재현해 strict 요청 위치를 연결했습니다. 변경 revision에 이전 후보의 삽입 범위를 재사용하는 실패는 `completion-stale-ranges-first.log`(8통과/1실패, Ok([0..3]) vs StaleRevision)로 재현해 문서 ID/revision 검증으로 수정했습니다.

최초 검사 코드의 serde_json 직접 참조는 editor의 기존 의존성에 없어 컴파일이 거절됐습니다(`completion-model-first.log`). 의존성을 추가하지 않고 typed opaque 문자열과 누락 kind를 검사하며 raw 숫자 kind의 protocol 검증은 앱 typed 경계에서 이어갑니다. 다중 커서 최초 검사의 존재하지 않는 Selection::caret API는 공개 Selection 필드로 수정했습니다(`completion-ranges-first.log`). 최종 이름의 snake-case와 포맷을 정리했고 Cargo fmt는 exit 0입니다. 포맷 변경만으로 이미 성공한 검사를 반복하지 않습니다.

## 스니펫 변환·변수와 실제 삽입 준비

원본 snippetParser의 Transform/FormatString과 snippetVariables·snippetSession의 resolver 순서, standaloneServices의 실제 URI label/workspace 서비스를 읽었습니다. 기존 regress 0.12.0의 공식 설치 API에 있는 UTF-16/UCS-2 입력을 사용하며 새 정규식 엔진/패키지를 추가하지 않습니다. syntax manifest에 utf16 기능만 활성화합니다. 시간대 이름을 위해 chrono가 이미 반입한 iana-time-zone =0.1.65를 앱에서 직접 참조하며 app lock은 root 의존 간선 1줄만 추가됩니다. editor/UI 엔진 의존성과 frozen source/manifest/lock은 바꾸지 않았습니다. 고지에 실제 라이선스의 Andrew D. Straw 저작권과 사용 경계를 반영했습니다. 이 선언 변경은 batch22 전체/동결 컴파일 게이트에서 다시 확인합니다.

core의 최초 옵션 검사는 복제되지 않은 변수의 작성 순서 ygim을 igmy로 기대해 실패했습니다(snippet-options-first.log). 실제 복제 경로로 검사를 고친 뒤 nested 기본값의 s가 빈 문자열로 누락됨을 snippet-options-copy-first.log에서 재현했습니다. clone_transform이 검증된 d/m/s/u/v/y 옵션을 보존하도록 수정했습니다. /private/tmp/taide-batch22-snippet-options-after-copy.log의 cargo test --test snippet-options --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target는 exit 0·1통과/0실패/0ignored입니다. 원본 Transform.clone의 i/g만 복사하는 버그를 강제 재현하지 않습니다.

syntax MonacoSnippetTransforms는 엄격한 JS 옵션/중복/u+v 검증·RegExp.source 메타데이터, 숫자 캡처/조건·전체/첫/연속 sticky 일치·빈 일치의 Unicode 전진·shorthand 7종·없는 일치의 else를 제공합니다. 비-Unicode는 UCS-2, u/v는 UTF-16로 평가하고 최종 저장 문자열의 짝 없는 단위는 U+FFFD로 변환합니다. 원본의 named capture 콜백에 부수 인자가 섞이는 버그나 sticky 평가의 숨은 이전 상태를 전용 동작으로 만들지 않습니다. 캐시/입력/format/출력 총량을 제한하며 정상 변환에는 추가 엔진을 사용하지 않습니다. 실제 UI accept/step 소비는 아직 남았습니다.

docs/utils/2026-10-10-monaco-snippet-transforms-oracle.js는 실제 SnippetParser/Transform.resolve와 내장 RegExp로 3240변환/130메타데이터를 생성했습니다(Bun exit 0). 첫 syntax 검사는 3통과/1실패였고 비-Unicode 입력에서도 서로게이트 쌍을 합쳐 캡처한 문제를 UCS-2 경로로 수정했습니다(snippet-transforms-first.log). 중간 잘못된 필터는 exit 0이나 실행 0건이므로 성공 근거로 세지 않습니다(snippet-transforms-after-ucs2.log). ASCII utf16 필터는 실제 1건/3240표본을 통과했고, format/캐시 경계를 보강한 최종 /private/tmp/taide-batch22-snippet-transforms-after-bounds.log의 cargo test --test snippet-transforms --manifest-path native/taide-native-syntax/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target는 exit 0·4통과/0실패/0ignored(0.06초)입니다.

앱 resolver는 실제 파일/untitled 모델 경로·선택/현재 줄/단어/커서·언어 주석·제공된 clipboard와 spread·날짜/시간대·난수/UUID를 평가합니다. standalone 기본 workspace는 실제 원본 실행대로 이름 빈 문자열/폴더 /이며 RELATIVE_FILEPATH도 label service의 실제 절대 경로를 보존합니다. 프로젝트 root를 임의로 workspace/상대 경로에 대신 넣지 않습니다. tests는 fixed clock과 제공한 문자열을 사용하고 OS clipboard를 읽거나 쓰지 않습니다. 원본 Random의 가변 길이 끝자리 문자열 대신 6자리 숫자/hex와 UUID를 정상 형식으로 제공합니다.

docs/utils/2026-10-10-monaco-snippet-variables-oracle.js는 실제 resolver와 읽은 원본 standalone class를 실행해 파일 70·시간 85·작업공간 3표본을 생성했습니다(Bun exit 0). /private/tmp/taide-batch22-completion-variables-first.log의 cargo test --lib editor_completion::variables:: --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -- --test-threads=4는 exit 0·4통과/0실패/0ignored(0.01초)입니다. 최초 app check는 테스트 모듈 파일 누락/context 이동의 컴파일 오류였고 test 파일과 소유를 수정했습니다(completion-variables-lock-check.log). lock의 간선 갱신을 위한 이 최초 명령만 --offline로 실행했으며 이후 모든 검사에 --locked를 적용했습니다.

State.prepare_candidate는 현재 source/owner/request 토큰·snapshot과 후보를 확인하고 변경 없이 core 준비물을 만듭니다. 원본 snippetSession처럼 각 후보의 실제 overwrite 범위를 변수 selection으로 사용하고, 현재 문서의 실제 엔진/클립보드 행/언어/들여쓰기/EOL을 평가합니다. plain 달러는 평가하지 않습니다. 초기 검사 코드의 Store transaction 필드/문서 ID 호출 오류를 실제 API대로 수정했습니다(completion-preparation-first.log). 최종 /private/tmp/taide-batch22-completion-preparation-multi.log의 cargo test --lib completion_preparation --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -- --test-threads=4는 exit 0·4통과/0실패/0ignored(0.02초)입니다. 단어 con의 overwrite 선택·경로/시간/주석/멀티라인 transform·수락 전 불변/plain 달러/undo·mirror/tabstop·두 커서의 selection/clipboard 행·토큰/편집/닫힘/용량 거절을 확인했습니다. Clock.now와 준비물 재export는 실제 UI 소비 전이라 미사용 경고가 남습니다.

삽입 뒤 Session.new에서 들여쓰기+choice/다중 커서 메타데이터 합계가 한도를 넘을 수 있는 문제를 snippet-session-limits-first.log의 2실패로 재현했습니다. core insert가 marker/choice/transform 비용과 줄 leading whitespace 합계를 문서 변경 전에 검사하도록 수정했습니다. /private/tmp/taide-batch22-snippet-session-limits-after-cost.log의 cargo test --test snippet-insertion --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target는 exit 0·5통과/0실패/0ignored입니다. 실패 거절은 문서/선택/undo를 바꾸지 않으며 기존 3건도 통과했습니다. 영향 completion-insertion 5건은 completion-insertion-after-session-cost.log에서 exit 0, 앱 준비 영향 3건은 completion-preparation-after-session-cost.log에서 exit 0입니다.

기존 찾기 영향 cargo test --test find-model --test find-regex-gate --manifest-path native/taide-native-syntax/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target -- --test-threads=4는 /private/tmp/taide-batch22-snippet-find-regression.log에서 exit 0·5통과/0실패/0ignored입니다. 기존 100ms 채택 기준/1초 검색 상한의 새 성능 측정으로 대용하지 않습니다. 전체 대상과 frozen host/Wasm 게이트는 아직 실행하지 않았습니다.

현재 배치의 서로 다른 성공 72건(core 25·SDK 4·app 23·child 3·host 1·syntax 16)을 기록하되 실제 본문/peek 요청/표시 소비, 후보 편집 rebase/incomplete 재요청, preview·popup 상세/명령·직렬 snippet 입력·nested 삽입은 여전히 c/d의 미완료입니다. 체크리스트는 2/7, 기능표는 286/588(48.6%)을 유지합니다. 디스크 여유 606GiB·사용 67%이며 다음으로 실제 후보 창/본문·peek 입력을 연결합니다.

검증한 독립 결함만 복제 옵션 5ee71a0d(3파일)·수락 전 용량 1d49809f(3파일)로 선별 커밋하고 origin/to_rust_native에 일반 푸시했습니다. staged diff/최종 한국어 메시지·사용자 단독 author/AI 트레일러 없음과 푸시 종료 exit 0·로컬/원격 0/0을 확인했습니다. 신규 모델/SDK/앱 소비 소스와 기존 HANDOFF/architecture/합의/운영 문서, PROCESS 하단 변경은 포함하지 않았습니다. App lock의 827패키지 name/version이 918b764b와 동일함을 실제 비교했습니다. editor/syntax/app fmt의 최종 명령은 exit 0, JavaScript/고지/QA Prettier와 소유된 tracked diff check는 exit 0입니다. Unicode 테스트의 literal을 같은 값의 escape로 바꾸고 format check에서 줄바꿈 1차이가 나와 syntax fmt로 정리했습니다. 이 포맷 검사를 테스트 성공으로 세지 않습니다.

## 후보 창과 공유 문서 렌더러의 현재 검증

호버/시그니처의 실제 Markdown 표시 코드를 editor-markup.rs로 옮겨 자동완성 상세 문서도 같은 코드·이미지·링크 경계를 사용합니다. 기존 Provider의 세 표시 callback을 blanket 구현으로 연결하고 공개 Colors 경로는 보존했습니다. 의미 없는 hover 요청 stub이나 editor/UI 엔진 의존성을 추가하지 않았습니다. codicon 폰트 이름도 실제 두 소비자의 공유 font-families 경계로 옮겼습니다.

UI State는 원본 430px·최대 12행/8..1000 행 높이·종류 codicon·UTF-16 강조·선택·스크롤·Loading/Empty·상세 문서와 키를 제공합니다. 실제 egui RawInput을 사용하는 메모리 검사에서 문서 불변인 탐색, Shift 대체 수락/undo, Text 뒤 같은 프레임 Tab 수락, 10ms 자동 요청/문자 trigger, 50ms 명시 Loading/Empty, 상세 focus/Escape 뒤 문자 전달, 5000개 후보의 가상 행/마지막 reveal, caller 키 재정의, IME preedit/commit, 포인터 수락과 변경 보고, 다른 입력란의 문자/포커스 보존을 확인했습니다. OS 합성 입력이나 실제 클립보드를 사용하지 않았습니다.

상세 focus/Escape 검사가 처음 5통과/1실패였고 focus 진단에서 다음 프레임의 focus가 None임을 확인했습니다. 요청 시점만 바꿔도 실패했습니다. 설치된 egui Context::create_widget가 같은 Area의 비포커스 root를 만들 때 focus를 해제하는 실제 코드를 확인하고 화면 Area ID와 내부 focus ID를 분리해 해결했습니다. 입력 순서를 앞당기는 우회는 하지 않았습니다. 신규 IME fixture의 selection 필드 오기는 실제 active_range_chars API로 수정했습니다. 도움말의 시그니처 탐색/취소를 raw 사전 소비에서 본문 직렬 처리로 옮겨 caller 키맵과 새 후보 명령 뒤에 처리하도록 했습니다.

최신 completion-ui-after-owner.log는 cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --features inspection --test editor-completion -- --test-threads=4의 exit 0·10통과입니다. completion-ui-after-ime-api.log의 같은 UI 명령 --test editor-completion --test editor-documentation는 exit 0·9+13통과이며 후속 completion 10건과 도움말 13건을 중복 집계하지 않습니다. 공유 renderer 초기 shared-markup-documentation.log도 exit 0·13통과였으며 별도 성공 수를 더하지 않습니다. completion-ui-all-check.log의 cargo check --tests --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --features inspection는 exit 0입니다. 본문 요청 initializer 전부를 native-host cfg로 맞췄으며 실제 앱 initializer에는 아직 None을 유지합니다.

이 기록 시점의 서로 다른 scoped 성공은 95건입니다. UI caller mock으로 표시/직렬 순서를 검증한 근거이며 실제 앱의 후보 공급/수락 완료로 확대하지 않습니다. 실제 Body/peek Provider·원본 theme와 파일/색 종류 표시·문서 토글 포인터·preview·후보 rebase/incomplete 재요청·snippet/nested session·전체/동결 게이트는 미완료입니다. 체크리스트 2/7·기능표 286/588(48.6%)을 유지하고 디스크 여유 604GiB·사용 68%를 확인했습니다.

최종 라벨 대조에서 Monaco 0.56.0의 실제 suggest.css/renderer·HighlightedLabel.escapeNewLines를 확인해 CRLF/CR/LF를 단일 반환 기호로 표시하고 detail의 줄바꿈만 제거하도록 반영했습니다. 강조의 bold 폰트, Deprecated의 0.66 opacity/strike와 강조 제외, 선택된 행의 85% detail·기본 둥근 모서리/그림자를 소비합니다. completion-ui-final.log는 exit 0·11통과이며 Unicode/CRLF/강조/Deprecated 검사를 추가해 현재 서로 다른 scoped 성공은 96건입니다. 신규 UI 코드·태그 helper의 테스트와 core/UI fmt는 exit 0입니다.

앱의 전체 테스트 대상 check는 Completion 응답을 기존 tests/lsp.rs의 exhaustive match에 포함하지 않아 E0004로 실패했습니다(completion-app-surface-check.log). 예상하지 않은 편집기 응답을 거절하는 기존 분기에 Completion을 추가했고 completion-app-surface-after-reply.log의 같은 --tests 명령이 exit 0입니다. 이를 실제 LSP 통합 검사 실행으로 세지 않습니다. UI의 새로운 표면 API initializer는 전부 컴파일됩니다. 앱의 실제 completion consumer가 남아 있어 미사용 경고는 억제하지 않았으며 배치 전체 테스트 대상 실행은 아직 하지 않았습니다.

선별 모델 커밋 3194cb15는 core 모델/검사/두 원본 추출 도구/독립 QA의 13파일만 포함합니다. App SDK/엔진/변수 공급, UI 표면, 기존 사용자 문서와 PROCESS 하단을 포함하지 않았습니다. TSV의 마지막 빈 강조 목록 필드가 staged diff check에서 trailing whitespace로 보고돼 도구/decoder/표본을 명시적인 none으로 함께 수정하고 원본 8100표본을 재생성했습니다. completion-filter-after-fixture.log의 필터 1건과 completion-model-final.log의 전체 목록 4건은 exit 0입니다. 마지막 staged diff check exit 0·13경로와 실제 검증 파일의 동일성·최종 한국어 메시지/사용자 단독 author/AI 트레일러 없음·일반 푸시 exit 0·0/0을 확인했습니다. app surface format check도 exit 0이며 실제 앱 소비 연결과 전체 게이트는 계속 남아 있습니다.

## 입력 후 후보 캐시와 불완전 공급자

completion-cache-first.log의 completion_cache 검사 1건은 입력 후 state.model이 None이 되어 exit 101로 실패했습니다. 완료된 후보를 변경 journal로 갱신하고 원래 필터 길이와 현재 입력 delta를 유지하도록 고쳤습니다. 표시 token과 transport 요청 token을 분리해 목록 선택/수락은 유지하며 이전 RPC와 단어 worker는 취소합니다. 불완전 그룹만 다시 요청하는 동안 완료된 그룹과 기존 모델을 유지하고 응답 시 보존 그룹/스니펫/단어의 범위를 현재 revision으로 함께 갱신합니다. 줄바꿈·공백·새 단어·문서/뷰/provider/언어/조합/읽기 전용 경계는 취소 계약을 유지합니다.

첫 변경 검사 completion-cache-after-rebase.log는 내부 byte_to_char 호출의 E0603 때문에 컴파일 실패했습니다. 공개 위치 변환 API로 바꾼 completion-cache-after-public-boundary.log는 exit 0·1통과입니다. completion-rebase-tests.log의 core --test completion-model completion_rebase는 exit 0·2통과입니다. completion-cache-app-regression.log의 앱 --lib completion_ -- --test-threads=4는 exit 0·29통과·0실패이며 기존 요청/공급/변수/수락 준비와 실제 child 3건을 포함합니다. 위 명령은 모두 --locked --offline --target-dir experiments/native-shell-spike/target를 사용했습니다.

새로운 서로 다른 검사는 core 2·앱 캐시 3건이며 이전 96건과 합친 scoped 성공은 101건입니다. 실제 앱/peek Provider와 재요청 dispatch, snippet/preview·테마 연결은 남아 있어 배치 22 체크리스트 2/7과 기능 감사 286/588(48.6%)을 유지합니다. 이 기록을 전체 대상 실행이나 실제 화면 완료 근거로 확대하지 않습니다.

## 실제 공급자와 직렬 스니펫 입력 연결

기본 문자 입력 뒤 snippet 세션이 만료되는 실패를 UTF-16 journal 추적과 인접 mirror의 범위 소유로 수정했습니다. 독립 근거는 199f05e3의 core QA에 기록했습니다. snippet-default-input-after-range-ownership.log는 completion-insertion 5건·snippet-insertion 7건이 exit 0이며 앞선 aggregate exit 101 안의 journal 13건 개별 통과를 전체 명령 성공으로 세지 않습니다. 199f05e3은 선별 커밋·일반 푸시 exit 0과 0/0을 확인했습니다.

앱 Provider를 본문과 peek에 연결하고 실제 사용자 catalog·단어 worker·프로젝트 LSP·테마·공유 Markdown/코드/이미지/파일 링크·스니펫 변수/변환을 소비합니다. 새 요청은 문서 mirror 처리 뒤 다음 background tick에서 전달하며 repaint callback은 실제 egui Context를 사용합니다. 완료/불완전 공급자 캐시와 snippet 세션을 owner/문서/언어/읽기 전용/IME/선택 경계에서 회수합니다. 실제 클립보드를 읽지 않았으며 해당 변수의 앱 소비는 아직 연결하지 않았습니다.

completion-consumer-check.log의 앱 cargo check --tests는 exit 0입니다. 첫 소비자 검사는 attach_view의 문서 인수 누락으로 E0061이 발생했습니다(completion-consumer-first-tests.log). 수정 후 completion-consumer-after-attach.log는 2통과·1실패였고 같은 프레임 Enter/Text/Tab의 문서가 con 그대로이며 focus None으로 확인됐습니다(completion-consumer-focus-diagnostic.log). 설치된 egui Memory::set_focus_lock_filter가 이전 프레임 포커스를 요구하는 실제 소스를 확인하고 native-host의 초기 포커스 요청에 필터를 함께 설치했습니다. completion-consumer-after-first-focus.log의 앱 --lib native_completion_consumer -- --test-threads=4는 exit 0·3통과·0실패입니다. 이 명령은 --locked --offline --target-dir experiments/native-shell-spike/target를 사용했습니다.

세 검사는 실제 Provider·core 언어 입력·egui RawInput으로 후보 수락 뒤 같은 프레임 Unicode 문자와 Tab/ShiftTab mirror 이동·Esc 회수, peek 원본문만 변경/owner 회수, 실제 단어/숫자/토큰/커서의 자동 요청 조건을 확인합니다. OS 합성 입력은 사용하지 않았습니다. 본문/peek의 기본 keymap 우선순위와 재정의 명령, preview·choice/nested·장식, 전체/동결 게이트는 남아 있습니다. 체크리스트 2/7·기능 감사 48.6%를 유지하며 scoped 101건에 새 소비자 3건/별도 core 회귀를 구분해서 기록합니다.

실제 본문/peek 키맵은 후보/스니펫의 키를 처리하는 동안 일반 기본 binding을 양보하고 명시 override/chord와 앱 명령을 유지합니다. 등록된 triggerSuggest를 typed Completion 명령으로 연결하며 수락/탐색 같은 Monaco 내부 명령 재정의도 같은 프레임의 실제 store 변경 뒤 실행합니다. 뷰별 pending 명령은 입력 직렬 경계에서 소비하고 닫힘/회수 시 제거합니다. completion-keymap-first-check.log는 앱 --tests check exit 0입니다. 최초 소비자 키맵 검사는 원본 명령 목록에 없는 cursorRight override 기대 때문에 3통과·1실패였습니다(completion-consumer-real-keymap.log). 실제 목록의 copyLinesDownAction으로 검사를 고쳤고 completion-app-after-keymap.log의 앱 --lib completion_ -- --test-threads=4는 exit 0·33통과입니다. 실제 child 3건과 소비자 4건이 포함됩니다.

completion-ui-after-keymap.log는 후보 창 11·도움말 13건이 통과했지만 플랫폼 검사 8통과·1실패로 aggregate exit 101입니다. 새 검사의 ShiftTab 기본 binding 기대를 실제 source의 mod+[로 바로잡은 completion-keymap-after-source-key.log는 플랫폼 9건·exit 0입니다. completion-command-registry.log의 UI --lib command_registry는 9건·exit 0입니다. UI 명령은 --features native-host,inspection을 사용하며 모두 --locked --offline --target-dir experiments/native-shell-spike/target를 사용합니다. 이전 성공과 같은 검사를 새 검사로 중복 합산하지 않습니다.

소비자 검사는 실제 앱 Terminal Views 키 경로에서 문자 입력 뒤 같은 프레임 사용자 재정의 수락/후속 Unicode 입력/Tab 이동과 원본 줄 복사 재정의 우선순위를 확인합니다. 플랫폼 검사는 Mac/Windows/Nix에서 기본 binding 양보·명시 override·completion chord를 확인합니다. choice/nested/preview·장식과 전체 게이트는 남아 있습니다. 디스크 여유는 598GiB·사용 68%입니다.

## 선택지 목록과 실제 mirror 수락

Monaco snippetController2의 원본 choice provider/자동 trigger/현재 값 필터/옵션 순서/수락 뒤 next 명령을 읽고 현재 Session의 선택지를 같은 후보 창으로 연결했습니다. 옵션의 정렬 문자열은 원래 반복 문자열의 정렬 순서를 보존하는 고정 폭 인덱스로 표현합니다. 선택지 공급은 실제 LSP나 단어 worker에 요청하지 않으며 기존 주 커서/전체 mirror 범위와 token/revision을 검증합니다. 현재 값의 문자 입력 뒤 수락은 전체 활성 범위를 다시 선택해 값을 대체하고 다음 tabstop으로 이동합니다. 뒤로 이동하면 선택지 창을 다시 열고 hide 후에는 같은 자리에서 반복해서 강제 재개하지 않습니다. 용량 거절은 이전 문서/선택을 유지하며 입력 오류는 UI 검사 출력에 전달합니다.

completion-choice-first.log는 1실패로 choice popup 누락을 재현했습니다. 첫 소비자 연결은 기존 protocol reexport/trait/replace 인수(E0433/E0599/E0061) 누락으로 실패했고 completion-choice-after-public-api.log는 ScrollPosition 이동 E0507로 실패했습니다. 실제 공개 API와 소유권을 수정한 completion-choice-after-scroll-clone.log는 소비자 5건·exit 0입니다. 다중 커서/primary·부분 입력 뒤 전체 범위 수락과 용량 거절 검사를 추가한 completion-choice-primary-capacity.log는 소비자 7건·exit 0입니다. 앱 명령은 --lib native_completion_consumer -- --test-threads=4이며 --locked --offline --target-dir experiments/native-shell-spike/target를 사용합니다.

completion-ui-after-choice.log의 UI --features native-host,inspection --test editor-completion --test editor-documentation -- --test-threads=4는 exit 0·11+13통과입니다. 코어 snippet-choice-core-api.log의 --test snippet-insertion --test completion-insertion는 exit 0·8+5통과입니다. 새 코어 선택지 1건/앱 선택지 3건을 기존 검사의 재실행과 구분합니다. 순수 egui 메모리 입력이며 실제 클립보드/OS 합성 입력을 사용하지 않았습니다. snippet 장식·nested 삽입·suggest preview·파일/색 종류 표시·전체 및 동결 게이트는 남아 있어 체크리스트 2/7·기능 감사 286/588(48.6%)를 유지합니다.

completion-choice-retirement-first.log의 새 소비자 1건은 placeholder 밖의 편집으로 Session만 회수되고 선택지 후보가 남아 exit 101로 실패했습니다. 앱 reconcile 후와 같은 프레임의 Provider.current 갱신에서 실제 활성 선택지 index/세션을 함께 확인해 후보를 회수하도록 수정했습니다. completion-choice-after-retirement.log의 앱 --lib completion_ -- --test-threads=4는 exit 0·37통과·0실패이며 소비자 8건과 실제 child 3건을 포함합니다. 외부 문서 편집 자체는 유지합니다. Unicode 검사 문자열의 값은 유지하고 코드 표기만 escape로 정리했습니다. 최종 앱 --tests check인 completion-consumer-final-check.log는 exit 0이며 app/UI/core fmt와 소유된 diff check도 exit 0입니다.

선택지 코어/검사/독립 QA 3파일은 8c02674b로 선별 커밋·일반 푸시 exit 0·로컬/원격 0/0을 확인했습니다. 사용자 단독 author와 한국어 Conventional Commit/AI 트레일러 없음·staged diff check를 확인했습니다. 실제 공급자/SDK/공유 UI/앱 상태의 나머지 변경과 기존 사용자 문서·PROCESS 하단은 이 커밋에 포함하지 않았습니다. 배치 전체/동결 게이트는 아직 실행하지 않았습니다.

completion-frozen-host.log와 completion-frozen-wasm.log의 동결 크레이트 cargo check --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target는 각각 exit 0입니다. Wasm 명령에는 --target wasm32-unknown-unknown을 추가했습니다. 동결 디렉터리의 실제 git diff 경로는 비어 있으며 manifest/lockfile/기능/의존 그래프를 변경하지 않았습니다. Wasm의 기존 경고 6건을 검사 통과와 구분합니다. 아직 변경 크레이트 전체 테스트 대상을 직접 실행하지 않았으며 이후 공통 코드 변경이 생기면 해당 영향만 다시 검증합니다. 디스크 여유는 595GiB·사용 68%입니다.

다음 표시 기준으로 실제 suggestWidgetAdapter/computeGhostText와 snippetController2/snippetSession을 읽었습니다. 제안 미리보기는 일반 문자열 또는 변수 평가 전 snippet 문자열과 현재 overwrite 범위를 사용하고, 기존 원문을 삭제해야 하는 차이는 ghost로 표시하지 않습니다. 원본은 들여쓰기 차이·subword/prefix/커서 앞 제한·괄호 대응과 큰 diff 제한을 구분합니다. 단순히 접미사 위에 텍스트를 겹쳐 그리면 원래 본문 배치/좌표와 달라져 그대로 구현하지 않습니다. 현재 native DisplayMap/VerticalLayout에는 injected text 투영이 없으며 실제 설정 editor_suggest_preview도 표면에 아직 소비하지 않았습니다. 중첩 삽입은 기존 placeholder를 안쪽 그룹으로 합성하며 새 세션으로 바꾸는 현재 소비자의 동작이 남은 구현 대상입니다.

## 실제 중첩 스니펫 소비

completion-nested-first.log에서 실제 Provider의 일반 수락과 안쪽 final 도달 후 바깥 세션 소실을 2실패로 먼저 재현했습니다. 기존 core Session.insert_nested에 수락을 병합하고 안쪽 final 뒤 바깥 tabstop·ShiftTab 역이동을 유지합니다. 일반 텍스트 삽입은 기존 범위를 갱신하며 유지하고, 새로운 choice index는 표시 상태를 초기화해 실제 후보 창에 연결합니다. 이전 변환 캐시는 초기화하고 살아 있는 안쪽/바깥 변환은 기존 앱 전용 Syntax 엔진에서 평가합니다.

completion-nested-final.log의 app --lib completion_ -- --test-threads=4는 exit 0·41통과·0실패(실제 child 3, 소비자 12 포함)입니다. 신규 소비자 4건은 같은 프레임 중첩 수락/입력/Tab/ShiftTab·Unicode mirror·바깥 다음 위치·일반 수락·안쪽/바깥 변환·선택지 수락을 확인합니다. snippet-nested-core-final.log의 --test snippet-insertion --test completion-insertion는 exit 0·14+5통과입니다. 신규 core 6건과 원자적 preflight·용량·들여쓰기·20회 병합 근거는 2026-10-10-native-completion-model.md에 기록했습니다. 모든 실행은 기존 --locked --offline --target-dir experiments/native-shell-spike/target이며 로그 접두는 /private/tmp/taide-batch22-입니다.

미리보기·색/파일 아이콘 소비와 전체 게이트가 남아 있어 c/d 체크와 기능표 완료 행을 올리지 않습니다.

중첩 문맥의 별도 2건도 snippet-nested-context.log에서 exit 0입니다. 살아 있는 상위 placeholder의 역이동과 기존/안쪽 들여쓰기 문맥의 합산 용량 거절을 확인했습니다. core 삽입의 서로 다른 통과 검사는 기존 14+5와 추가 2의 21건이며, app 자동완성 41건과 함께 해당 변경 위험을 검사했습니다. app 모든 테스트 대상 check와 editor/app fmt check는 completion-nested-app-check.log·nested-core-fmt-check.log·nested-app-fmt-check.log에서 exit 0입니다. 추가 문맥 검사 뒤 core fmt도 exit 0입니다. 기존 groups() dead-code 경고는 억제하지 않았습니다.

공유 core 변경 후 동결 host/Wasm을 completion-nested-frozen-host.log·completion-nested-frozen-wasm.log에서 다시 검사해 각각 exit 0을 확인했습니다. 동결 디렉터리 git status는 비어 있으며 Wasm의 기존 경고 6건은 유지합니다. 디스크 여유는 593GiB·사용 68%입니다. 아직 batch22 전체 --no-fail-fast 게이트는 실행하지 않았습니다.

중첩 core 구현·신규 8검사·모델 QA의 6파일은 0a7d5521로 선별 커밋·일반 푸시했습니다. staged diff check exit 0, 사용자 단독 author·한국어 메시지/AI 트레일러 없음, push exit 0·로컬/원격 0/0·빈 index를 확인했습니다. 앱/UI/Syntax 연결과 전체 batch22 QA/PROCESS의 진행 내용은 작업 트리에 남아 있으며 이후 미리보기·아이콘·전체 게이트와 함께 선별 저장합니다. 사용자 HANDOFF/하단 PROCESS/기타 문서를 이 커밋에 넣지 않았습니다.

## 본문 미리보기 소비와 구문 문맥 검증 (2026-10-10)

설정의 suggest preview를 선택 후보·Shift 대체 범위·스니펫 준비 문자열과 연결했습니다. 첫 줄의 주입 문자, 줄바꿈 뒤 읽기 전용 추가 줄, 숨긴 원문 접미사를 표시하며 원문/undo를 바꾸지 않습니다. wrap·탭·Unicode·접기와 다음 문서 줄의 좌표를 따르고, 주입 글자의 이탤릭 서식 때문에 커서가 0.8px 이동하는 실패를 먼저 재현해 원문 접두사의 캐럿 좌표를 따로 보존했습니다.

기존 TextMate worker는 원문 앞줄 상태에서 미리보기만 평가합니다. 후보별 공유 문자열·토큰 캐시를 재사용하며 문서 revision·선택·들여쓰기·대체 범위·테마 세대·요청 ID가 달라지면 이전 값을 폐기합니다. 원문 전체 접두사를 복사하거나 editor/UI에 엔진 의존성을 추가하지 않습니다. 같은 줄의 다중 커서 주입을 하나의 가상 줄로 합쳐 중복 요청/캐시 교체를 피합니다.

원본 GhostTextView와 CSS를 기준으로 syntax 색/불투명도 0.7·원문과 분리된 이탤릭·짧은 후보의 테마 전경색/점선·배경·테두리를 적용합니다. 추가 줄의 원문 접미사는 원문 토큰을 유지합니다. 픽셀/실제 IME/접근성과 전체 출시 게이트까지 완료했다고 확대하지 않습니다.

검증 로그의 공통 경로는 /private/tmp/taide-batch22-입니다.

- completion-preview-core-final.log: exit 0, 미리보기/표시 투영/추가 zone 23건 통과·기존 성능 검사 1건 ignored. 원본 diff 9711·스니펫 1344·GhostTextView 12표본과 UTF-8/원문 경계를 확인했습니다. 원본 UTF-16 버그 9개는 승인한 유효 경계 정책으로 수정합니다.
- completion-preview-syntax-final.log: exit 0, token-pipeline 전체 23건 통과. 앞줄 주석·추가 줄·원문 토큰 불변·테마/문서 변경·늦은 응답/닫힘·캐시 회수를 포함합니다.
- completion-preview-styled-ui-consumer.log: exit 0, 후보 창/미리보기 14건 통과. 공통 편집기 81건은 completion-preview-styled-ui-first.log의 exit 0 결과를 재사용합니다. 최종 불투명도/배경/테두리 영향 3건은 completion-preview-opacity-ui.log에서 exit 0입니다.
- completion-preview-app-final.log: exit 0, 실제 앱 회귀 42건 통과. 실제 child 3·본문/peek/키 우선순위·선택지/중첩 세션·새 다중 커서 미리보기 표시/취소/수락 1건을 포함합니다. completion-preview-app-check-final.log의 앱 전체 테스트 대상 check도 exit 0이며 vendor 경고와 앱의 기존 테스트 전용 메서드 경고를 검사 결과와 구분합니다.

Core/Syntax의 미리보기 모델은 독립 단위로 선별 커밋합니다. App/UI의 배치 22 소비자는 작업 트리에 유지하며 후보 창 수동 크기 조절·완료 범위 재점검, 실제 앱 추가 회귀와 변경 크레이트 전체 --no-fail-fast 및 최종 기록/기능표/푸시를 이어갑니다. c/d/e/f/g를 완료로 바꾸지 않습니다. 현재 전체 전환율·잔여시간은 미산정이며 최종 배치는 33 계획을 따릅니다.

앱 groups 조회가 검사에서만 쓰이는 것을 확인해 cfg(test) 경계로 옮겼습니다. preview-app-production-check.log의 cargo check --lib는 exit 0이며 이 메서드의 production 미사용 경고가 없어졌습니다. frozen host/Wasm check는 preview-frozen-host-final.log와 preview-frozen-wasm-final.log에서 exit 0이며 Wasm의 기존 경고 6건을 구분합니다. editor/Syntax/UI/App fmt check와 선별 22파일 staged diff check도 exit 0입니다. 전체 working diff check는 무관한 docs/architecture.md 955행의 기존 EOF 빈 줄에서 실패하므로 전체 통과로 보고하지 않으며 그 변경은 스테이징하지 않습니다. 현재 디스크 여유는 588GiB·사용 68%입니다.

미리보기 Core/Syntax 모델·검증·원본 표본·MIT 고지·활성 PROCESS만 22파일을 선별해 6d6befa6 feat(editor): 자동완성 미리보기 문맥·표시 모델 추가로 커밋·일반 푸시했습니다. 실제 최종 메시지·직전 사용자와 같은 단독 author·AI 트레일러 없음·빈 index·로컬/원격 0/0을 확인했습니다. Syntax lib의 PreviewJob export와 MIT 고지·PROCESS 상단만 패치 스테이징했으며 다른 배치 22 소비자/manifest/lock/하단 기록과 무관한 변경은 포함하지 않았습니다. 다음은 원본 후보 창의 수동 크기 조절과 남은 실제 앱/전체 게이트입니다.

## 후보 창·상세 창의 수동 크기와 초기화 (2026-10-10)

대상은 native/taide-native-editor/src/completion.rs, native/taide-native-ui/src/editor-completion.rs·presentation.rs와 UI 검사, native/taide-native-app/src/editor-completion-provider.rs·editor-completion-consumer-tests.rs입니다. 설치된 Monaco suggestWidget.js·suggestWidgetDetails.js, listWidget.js·ResizableHTMLElement와 sash CSS/색 등록, standaloneServices의 InMemoryStorageService, 실제 egui 0.36.2 response/Area/ScrollArea 소스를 읽었습니다. 새 engine/패키지나 OS 입력을 사용하지 않았습니다.

원본 최소 폭 220px·기본 폭 430px/12행·위아래 테두리/모서리·화면 여백/내용 상한과 드래그 방향 고정, 실제 이동 축과 반 행 임계값만 저장합니다. 후보 필터의 표시 높이 제한은 원하는 높이를 바꾸지 않으며 닫을 때 최소 4.3행을 회복합니다. 저장은 원본 standalone과 같은 메모리 수명이며 viewport/본문·내장 편집기별로 분리합니다. PageUp/Down은 현재 보이는 첫/마지막 완전한 행부터 이동하고 다음 입력에서 페이지를 스크롤합니다. 테두리 두 번 클릭은 해당 축을 내용 폭/기본 높이로, 원본 editor.action.resetSuggestSize는 저장된 후보 크기를 초기화합니다.

상세 창은 타입 문자열과 문서를 별도로 공급해 문서가 없는 타입/스니펫도 표시합니다. 원본 좌·우·위·아래 배치/최소 2행과 사용자 크기를 소비하고 크기 조절 중 반대 테두리를 보존합니다. 원본의 화면 밖으로 벗어나는 드래그는 정상 화면 경계로 제한하며 그 버그를 재현하지 않습니다. 후보/본문 앵커가 그대로이면 드래그 위치를 유지하고 후보/앵커/화면이 바뀌면 다시 배치합니다. 상세 닫기 버튼은 본문 focus와 후보 창을 유지하며 테두리 색은 sash.hoverBorder→focusBorder→기존 app.focusBorder에서 가져옵니다.

실패 기록은 모두 /private/tmp/taide-batch22- 접두사입니다. completion-resize-first.log의 실패 1건은 고정 폭을 재현했습니다. completion-resize-suite.log와 diagnostic.log에서는 위쪽 높이 고정·후보 필터 뒤 저장 높이 소실·작은 드래그 release에서 focus None을 확인했습니다. Area의 이전 max rect와 ScrollArea 최소 크기를 요청 크기로 갱신하고 시작점 기준 전체 drag delta를 사용합니다. 작은 드래그의 release는 press에서 보존한 focus를 반환합니다. completion-resize-small-input.log는 release 시점에 수락 없음/focus None을 좁혀 확인했고 small-focus.log의 10초 RwLock 실패는 초기화의 data_mut 안에서 viewport id를 다시 읽는 중첩 잠금이었습니다. key를 잠금 밖에서 계산해 해결했습니다. 해당 로그를 성공으로 기록하지 않습니다.

completion-resize-final.log는 후보 창 19건·exit 0입니다. completion-details-size-first.log는 타입만 있는 후보의 상세 창 누락을 재현했습니다. 이후 상세 창의 경계/고정 모서리/최소 ScrollArea 높이를 수정한 completion-details-size-bounds.log의 관련 1건은 exit 0입니다. completion-resize-details-ui.log는 후보/미리보기/크기 22건과 도움말 13건·exit 0입니다. completion-resize-app.log는 실제 앱 43건·exit 0이며 실제 본문/peek 분류, 타입만 있는 스니펫, 사용자 재정의의 실제 초기화 명령과 닫힌 후보 창에서 명령 회수를 확인했습니다. 현재 앱 새 unused_mut 경고는 읽기 전용 borrow로 제거했으며 전체 대상에서 재확인합니다.

실제 공급/수명 c는 기존 SDK 옵션·실제 child의 정상/없음/오류/취소/종료·사용자/플러그인/단어·본문/peek 모델 소비 근거로 체크했습니다. CLIPBOARD의 준비 모델은 구현돼 있지만 앱 수락 경로의 값은 아직 None입니다. 이 연결과 후보 파일/색 종류 소비, 전체 게이트가 끝나기 전 d/e/f/g·두 기능 행을 완료로 올리지 않습니다. 기존 성공 검사는 불필요하게 반복하지 않으며 현재 변경 상태의 전체 UI 대상을 --no-fail-fast로 직접 실행 중입니다.

## 현재 제품 연결의 전체 대상 검사

직접 실행한 SDK·editor·Syntax·UI 전체 명령은 모두 --locked --offline --target-dir experiments/native-shell-spike/target --no-fail-fast를 사용했습니다. SDK는 --package taide-lsp -- --test-threads=4, UI는 --features native-host,inspection입니다. 로그 경로는 /private/tmp/taide-batch22-completion-product- 접두사입니다.

- sdk-all.log: exit 0, 2대상·86통과·0실패
- core-all.log: exit 0, 38대상·264통과·0실패·기존 성능 1 ignored
- syntax-all.log: exit 0, 13대상·165통과·0실패·기존 성능 3 ignored
- ui-all.log: 전체 23대상·410통과/3실패로 exit 101. 신규 초기화 명령의 읽기 전용 등록, 기존 변환 복제 옵션과 유효 mirror 입력을 거절하던 기대값을 대조했습니다. ui-affected-final.log에서 실패한 lib/snippet-parser/snippet-session의 138건이 exit 0입니다. 전체 UI를 반복 실행하지 않았으며 중복을 뺀 최종 413건·미해결 0입니다. 이후 앱 전체 검사에서 발견한 연속 키 영향은 별도로 다시 검증합니다.

원본 parser oracle는 기존 raw expected와 정규식 복제 옵션을 보존한 correctedExpected를 함께 저장합니다. 원본의 m/s/u 누락을 성공 기대값으로 강제하지 않습니다. mirror 4개가 유효한 상태에서 Session.replace를 정상 실행하고 겹친 caret 회수와 정상 undo를 검사합니다. 처음 oracle 수정 위치가 잘못돼 발생한 ReferenceError와 옛 undo 기대값 때문에 ui-affected.log는 실패했으며 성공 근거에 포함하지 않습니다.

앱 전체 app-all.log는 실제 종료했으며 67대상·760통과/2실패·보호 Trash 3제외로 exit 101입니다. lib의 옛 triggerSuggest 미지원 기대를 수정하고, 본문/peek의 기본 연속 키 K→I가 자동완성 키 양보 때문에 사라지는 원인을 수정했습니다. editor_pending이 있으면 기본 연속 키를 보존합니다. app-chord-final.log의 실제 startup 1건·app-dispatch-final.log의 dispatch 7건, ui-chord-final.log의 플랫폼 키 9건이 exit 0입니다. 전체 앱을 반복 실행하지 않았으며 중복을 뺀 앱 762건·미해결 실패 0입니다. --test-threads=4·보호 Trash 3제외를 유지했고 wry vendor 경고와 디버그 링크의 __eh_frame 16MB compact unwind 경고를 구분합니다.

현재 제품 연결 상태의 전체 143대상에서 SDK 86·editor 264·Syntax 165·UI 413·app 762, 서로 다른 1690건을 확인했습니다. 기존 성능 ignored 4·보호 Trash 제외 3은 실행 성공에 포함하지 않습니다. 남은 CLIPBOARD/종류 표시를 구현하기 전의 검사이며 배치 22 완료나 실기/최종 출시 게이트의 통과로 확대하지 않습니다. frozen-host.log/frozen-wasm.log의 check는 실제 exit 0입니다. editor/Syntax/UI/app/SDK fmt check는 exit 0이고 spike 전체 fmt는 기존 import/documentSymbol/테스트 함수 줄바꿈과 신규 completion 줄바꿈에서 exit 1이므로 전체 포맷 통과로 보고하지 않습니다. 디스크는 578GiB·사용 69%입니다.

## CLIPBOARD의 실제 앱 공급과 취소

설치된 Monaco suggest.js·suggestModel.js·suggestController.js·snippetSession.js·snippetVariables.js와 multiCursorPaste 기본 spread 옵션을 대조했습니다. 공급된 snippet 후보에 CLIPBOARD 참조가 있으면 후보 모델을 노출하기 전에 기존 HostBridge의 주입 가능한 ClipboardReader로 한 번 비동기 읽고, 요청 엔트리에 값을 보관해 실제 Provider.accept의 변수 준비에 전달합니다. 단어 재필터링/불완전 공급 재요청에서는 준비된 값을 재사용하고 새 명시 요청은 새 값을 읽습니다. 원본의 저비용 필요성 추정을 재사용하며 editor/UI에 정규식 엔진을 추가하지 않습니다.

이미 취소/종료한 요청은 포트를 호출하기 전에 거절하고 읽는 중 취소는 읽기 뒤 다시 검사합니다. 실제 앱의 응답 소비는 현재 프로젝트/본문 또는 peek owner·문서/언어/revision/선택·토큰·종료 상태를 확인합니다. 재입력 시 오래된 pending 표시를 회수해 현재 요청이 다시 준비할 수 있게 하며 늦은 결과는 적용하지 않습니다. 빈 값은 원래 snippet 기본값 규칙으로 처리하고 읽기 실패는 원문을 보존한 채 요청을 닫아 기존 오류 보고로 전달합니다. 값은 Arc<String>으로 공유해 수락 때 전체 문자열을 복제하지 않습니다. 기존 거부 파일 크기로 입력 한도를 제한하고 로그에 클립보드 내용을 쓰지 않습니다.

- completion-clipboard-first.log: exit 101, 값 준비 전 pending/model 조건의 실패 1건을 먼저 재현했습니다.
- completion-clipboard-wired.log: exit 0, 관련 43건. 새 3건에서 실제 주입 포트·다중 커서/원래 주 커서·CRLF/빈 줄 spread·빈 값 기본값·재입력/닫힘·읽기 중/이미 취소한 요청의 차단을 확인했습니다. 새 테스트의 대문자 이름 경고는 소문자로 정리했습니다.
- completion-clipboard-final.log: exit 0, 관련 46건. 실제 child 3건과 공급 실패 상태의 필요성 재계산을 포함하며 새로운 이름 경고는 없습니다.
- completion-clipboard-actual-app.log: exit 0, 실제 앱 생성/루프백·임시 설정 파일 저장·클립보드 요청 제출/비동기 응답 반영·원문 보존·정상 종료 1건. 기존 실기 데이터 대신 임시 AppPaths와 주입한 값만 사용했습니다.
- completion-clipboard-actual-app-final.log: exit 101, 추가한 읽기 실패 검사의 status 기대가 실패했습니다. 기존 report는 status가 아니라 오류 토스트를 사용합니다. 이를 변경하지 않고 검사를 실제 기존 경로로 정정했습니다.
- completion-clipboard-actual-app-error-final.log: exit 0, 실제 앱 1건. 성공 읽기 한 번·실패 읽기 한 번과 오류 토스트·현재 요청 폐기·원문 보존·정상 종료/작업 0을 함께 확인했습니다. 실제 앱 전체 검사나 원본 UI 실기의 성공으로 확대하지 않습니다.

로그 접두사는 /private/tmp/taide-batch22-입니다. 포트 검사와 실제 앱 검사에서 정상 종료 후 tasks.tracked_count()가 0입니다. 실제 OS 클립보드·앱 데이터/설정·화면 입력을 사용하지 않았습니다. app fmt check·변경 App/Mock diff check는 exit 0, 디스크 575GiB·사용 69%입니다. 마지막 fmt 실행은 완료 확인 전에 check를 시작한 운영 실수가 있어 두 handle의 실제 exit 0을 각각 확인했습니다. 이 구간을 직렬 실행으로 보고하지 않으며 다음 Cargo/fmt는 session_id가 반환되면 다음 실행을 시작하지 않고 그 handle의 종료부터 확인합니다.

현재 전체 1690건은 이 CLIPBOARD 구현 전 스냅샷의 근거이며 최신 47건으로 전체 성공을 대신하지 않습니다. 후보 파일/색 표시 대조와 남은 배치 완료 게이트가 남아 d/e/f/g·자동완성/스니펫 두 행은 미완료입니다. 현재 변경과 관련된 App/UI 소비자·근거/고지는 배치 22 논리 단위로 함께 선별 커밋합니다.

현재 공개 HostCommand/HostReply 변경의 외부 테스트 타입 계약은 completion-clipboard-all-check.log에서 cargo check --tests --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target을 직접 실행해 exit 0으로 확인했습니다. 이 컴파일 검사를 전체 테스트 실행이나 배치 완료로 보고하지 않습니다. 원본 목록/상세/크기/클립보드 흐름의 MIT 고지를 함께 보존했습니다.

목록/상세/미리보기/크기·스니펫·CLIPBOARD 소비와 회귀·원본 표본·고지·QA/기능표/활성 PROCESS를 41e2af5c feat(native): 자동완성 목록과 스니펫 소비 연결로 선별 커밋했습니다. 직접 허용한 경로 79개를 rename 미검출 방식으로 비교해 추가/누락 0·PROCESS 하단 보존·staged/최종 diff check exit 0을 확인했습니다. Git의 rename 집계는 78파일이며 최종 사용자 단독 author와 AI 트레일러 없음도 확인했습니다. 두 요구사항 행은 partial, d/e/f/g는 미완료를 유지하며 최종 배치 33 계획을 이어갑니다.
