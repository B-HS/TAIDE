# 배치 21 — 일반 호버와 시그니처 도움말

현재 상태: 구현·실제 앱 통합과 전체 게이트를 마쳤습니다. 체크리스트 6/7이며 현재 기능 대응표는 284/588(48.3%)입니다. 변경 전체 121대상과 실패 영향 재검사에서 서로 다른 editor 219·UI 390·app 719·SDK 84, 총 1412건이 통과했습니다. frozen host/Wasm·4크레이트 fmt/diff exit 0, 디스크 623GiB/66%입니다. 완료 행 기록과 선별 Git을 이어갑니다. 전체 출시 전환율/잔여 시간은 미산정입니다. main이 직접 수행하고 Cargo/fmt는 하나씩 실행합니다.

## 체크리스트

- [x] a. 실제 TS/Monaco·공식 typed API·Markup/지연/trigger/키/테마·현재 native 공급 대조
- [x] b. 호버·시그니처/Markdown 모델·UTF-16 인자·요청 교체/취소·빈/오류 검증
- [x] c. 실제 typed LSP·현재 프로젝트/provider·peek mirror·편집/닫힘/재시작 공급
- [x] d. 원본 호버/서명 표시·명령/직렬 입력·본문/peek 포커스 소비
- [x] e. 실제 앱·문서/뷰/서버 수명·dirty/readonly·종료 통합, 실기/대형 성능 부채 구분
- [x] f. 변경 전체 대상·동결 host/Wasm·포맷/diff·디스크/보호 확인
- [ ] g. 실제 QA/기능표/완료 근거/PROCESS·선별 커밋·일반 푸시·다음 범위

## 범위와 보호 경계

일반 문서 호버와 시그니처 도움말을 실제 TS adapter/Monaco 화면·상태·상호작용과 현재 native typed LSP/표면에 연결합니다. 기존 정의 hover와 중복 요청·포커스·입력 소유를 확인하고 순수 peek 공급은 실제 소유/문서 수명으로 연결합니다. 원본 버그/내부 수치 강제 재현·새 기능/디자인·동결 browser 변경·OS 합성 입력을 추가하지 않습니다.

actual 사용자 데이터/OS 설정/clipboard/Keychain/Trash와 보호 app bundle, TS/Tauri/Monaco/xterm을 유지합니다. editor/UI에 regex·구문 engine을 추가하지 않습니다. 테마의 7자리 HEX 결정·실기 pixel/IME/접근성/성능/soak/패키징·전체 workspace/CI 부채와 기타 LSP 기능은 잔여 전체 목표입니다.

## 검증 근거

core 7·앱 Markdown 8·요청 상태 8·SDK 2·실제 typed child 5건이 통과했습니다. 최신 메모리 도움말 UI 13·플랫폼 키 8·코드/테마 1·실제 앱 본문/peek/종료 1·파일 링크 3·이미지 4건도 통과했습니다. 전체 editor/UI/SDK 실행을 마쳤으며 앱과 동결/포맷/경계 게이트를 이어갑니다. 변경 없는 경계의 배치 20 성공 근거만 재사용하고 변경한 크레이트는 f에서 전체 대상을 직접 1회 실행합니다.

## 원본과 공식 계약

실제 TS hover/signature-help adapter·protocol/initialize·code-editor와 설치된 Monaco getHover/markdownHoverParticipant/hoverOperation/contentHoverController/hoverActions/hoverActionIds·parameterHintsModel/provideSignatureHelp/parameterHintsWidget/parameterHints/CSS·editorOptions·markdownRenderer를 읽었습니다. 설치된 공식 lsp-types 0.97.0의 hover.rs/signature_help.rs/lib.rs와 SDK native feature/session/capabilities/registration, native editor surface/definition hover/본문·peek/토큰 공급을 대조했습니다.

호버 기본값은 delay/hidingDelay 300ms, sticky/above true입니다. async 요청은 150ms에 시작하고 표시 300ms/Loading 900ms를 유지합니다. 범위가 없으면 실제 단어/anchor 범위를 사용하고 provider별 실패를 격리해 원래 우선순위로 표시합니다. pointer가 popup에 들어오거나 키보드로 열었거나 focus/resize/텍스트 선택 중이면 유지합니다. 편집/문서·언어/뷰·공급 교체, 스크롤·본문 입력·blur의 수명과 정의 hover 중복을 확인합니다. 명시 showHover는 Mod+K Mod+I이며 이미 표시되면 focus합니다. popup focus의 방향키/PageUpDown/HomeEnd·Escape와 실제 링크/선택을 원본으로 연결합니다.

시그니처 기본값은 enabled/cycle true, 120ms trailing이며 명시 Mod+Shift+Space는 즉시입니다. trigger의 마지막 입력 문자와 활성 시 retrigger, 활성 상태의 본문/커서 갱신을 소비하고 mouse 선택/blur/모델·언어·provider 교체에서는 닫습니다. 여러 provider는 우선순위상 첫 non-null 응답을 사용하고 부분 오류는 다음 provider로 넘깁니다. 빈 signatures는 닫고 서명별 activeParameter가 전역보다 우선합니다. Up/Down·Alt 변형/mac Ctrl+P/N은 복수 서명에서 순환하고 Escape/ShiftEscape는 닫습니다. popup은 최대 폭 440px·높이 max(editorHeight/4, 250), 원본 editorHoverWidget foreground/background/border/highlight와 22px controls·현재 인자 bold/parameter 문서·서명 문서를 사용합니다.

TS는 hover 언어 코드를 value로만 전달하고 signature documentation의 MarkupContent를 문자열로 낮춥니다. native는 원본 버그 강제 재현 불필요라는 사용자 지시에 따라 공식 MarkedString language/value와 plaintext/markdown 종류, SignatureInformation.activeParameter를 보존합니다. TS처럼 signature 요청에는 context를 보내지 않고 initialize의 contextSupport를 새로 선언하지 않습니다.

## Markdown 의존성과 경계

Cargo manifest/lock·기존 native 코드와 캐시에 Markdown parser가 없음을 확인했습니다. 표/중첩 강조/목록/코드/링크의 표준 파싱을 임의 부분 parser로 대신하지 않기 위해 [공식 최신 릴리스](https://github.com/pulldown-cmark/pulldown-cmark/releases/tag/v0.13.4)·[공식 API](https://docs.rs/pulldown-cmark/0.13.4/pulldown_cmark/)와 다운로드한 crate src/lib.rs/Cargo.toml/license를 확인했습니다. app 전용에 pulldown-cmark =0.13.4/default-features=false를 추가했고 app lock에 pulldown-cmark와 unicase 2.10.0 두 패키지만 추가했습니다. parser는 MIT, unicase는 MIT OR Apache-2.0입니다. HTML 출력/serde/CLI/별도 구문 engine은 활성화하지 않습니다.

실제 cargo add/fetch는 각 log에서 exit 0이며 현재 macOS target에 두 패키지를 캐시했습니다. 이후 검증은 locked/offline과 공용 target-dir입니다. editor/UI에 parser/regex/구문 engine을 추가하지 않았고 frozen browser source/features/manifest/lock/graph는 변경하지 않았습니다. MarkupContent를 core 표시 구조로 변환하며 강조·목록/quote·표 alignment·fenced language·이미지 alt/title을 보존합니다. 비신뢰 HTML을 렌더하지 않고 command/javascript/상대경로 링크는 클릭 대상에서 제외합니다. 실제 파일 링크/이미지 공급은 d/e에서 기존 root guard/미디어 경계로 연결하며 아직 앱 도달 완료로 판정하지 않습니다.

## 현재 모델 검사

core `cargo test --test documentation --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`는 exit 0·6통과·0실패이며 log는 `/private/tmp/taide-batch21-documentation-model.log`입니다. content 종류/배열/언어 코드·빈/반전/다른 위치·UTF-16 보조단위 경계·문자열 인자의 JS word/정규식 특수문자·서명 순환/우선 인자·범위밖 인자를 확인했습니다.

app `cargo test --lib --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target editor_markup::tests`는 exit 0·6통과·0실패(449 filtered), `/private/tmp/taide-batch21-markup-model.log`입니다. nested style·inline code/hard break/link·heading/quote/ordered nested task list/fenced code/rule·table alignment·plaintext·비신뢰 HTML/링크·이미지 alt/title을 확인했습니다. loose task list와 SDK dynamic signature trigger 형식의 실패 재현을 추가했습니다. 실제 연결/요청 상태·표면/앱·전체 게이트는 남아 있습니다.

이후 SDK 동적 등록이 숫자/객체 trigger 배열을 수용하는 실패를 `/private/tmp/taide-batch21-signature-registration-first.log`(exit 101)로 재현하고 typed SignatureHelpOptions 검증과 정적/동적 문서별 옵션 cache를 연결했습니다. 영향 2건은 `/private/tmp/taide-batch21-signature-options-after-validation.log`에서 exit 0(82 filtered)입니다. 언어 selector·문서 edit의 Arc 재사용·unregister/close/종료 공급 회수를 확인했습니다.

loose task list의 checkbox가 paragraph frame에 저장돼 사라지는 실패는 `/private/tmp/taide-batch21-markup-loose-list-first.log`(exit 101, None vs Some(true))로 재현했습니다. 가장 가까운 list item 소유로 수정한 뒤 `/private/tmp/taide-batch21-markup-after-list-owner.log`에서 7통과/exit 0(449 filtered)입니다. core trigger/retrigger 통합을 포함한 최신 7건은 `/private/tmp/taide-batch21-documentation-triggers.log`에서 exit 0입니다.

요청 상태는 선택/문서/언어/뷰/owner/provider 세대와 늦은 응답·독립 호버/서명·잘못된 범위·재요청 표시 유지·부분 응답 순서/Arc 재사용을 검증했습니다. 최초 FnMut closure가 providers를 이동한 컴파일 오류는 실제 API에 따라 clone으로 수정했고 최신 8건은 `/private/tmp/taide-batch21-documentation-state-progressive.log`에서 exit 0(461 filtered)입니다.

## 실제 typed child 공급과 수명

앱 edition 2024의 `native-lsp-mock` example 빌드는 `/private/tmp/taide-batch21-documentation-mock-build.log`에서 exit 0입니다. 실제 child 최초 3건은 `/private/tmp/taide-batch21-documentation-child-first.log`에서 exit 0이며 추가 provider/재시작을 포함한 최신 5건은 `/private/tmp/taide-batch21-documentation-child-providers.log`에서 exit 0(464 filtered), 테스트 실행 5.37초입니다. 같은 입력의 성공 검사를 다시 반복하지 않습니다.

typed HoverRequest/SignatureHelpRequest의 UTF-16 line 1/character 4·extra context 없음·Markdown/plaintext/언어 코드·인자 [2,4]를 byte 2..6으로 변환·trigger/retrigger를 확인했습니다. 빈/null/protocol 오류/형식 오류/미지원은 개별 provider에 격리됩니다. 두 Python provider와 두 프로젝트에서 호버는 모두 병렬 요청하고 부분 응답을 즉시 공급한 뒤 등록 우선순위로 표시합니다. 서명은 첫 non-null을 소비하고 빈 signatures면 하위 provider로 넘어가지 않습니다. 부분 오류/null은 다음 provider로 진행하고 다른 프로젝트 owner 집합은 서로 겹치지 않습니다.

보류 중 다른 문서 mirror를 처리하고 편집·preview owner/뷰 폐기로 cancel을 전송합니다. 서버 crash/자동 재시작 뒤 mirror와 provider generation을 비교해 이전 요청을 회수하고 새 응답만 적용합니다. 모든 fixture는 disconnect·tasks.shutdown 뒤 tracked_count 0입니다. 실제 사용자 데이터/OS/Trash를 사용하지 않았습니다.

최초 앱 tests check는 새 Reply::Documentation에 대한 기존 integration test의 exhaustive match 누락으로 exit 101(`/private/tmp/taide-batch21-documentation-bridge-check.log`)이었습니다. 기존 unexpected-feature 분기에 새 variant를 추가했습니다. 순수 peek는 원본문 프로젝트 root 안의 현재 preview 문서만 mirror에 포함하며 cross-root/CLI-only 문서는 LSP로 반입하지 않습니다. 실제 앱 연결과 전체 check 결과는 이어서 기록합니다.

## 메모리 UI와 앱 연결의 현재 결과

표면의 포인터/명시 호버·자동/명시 시그니처와 직렬 입력, registry/keymap·본문/peek 공급을 연결했습니다. 앱 check의 최초 cfg 필드 누락과 Rc 내부 가변 참조의 lifetime 불변성 오류는 실제 컴파일 결과에 따라 수정했습니다. 최신 앱 tests check는 `/private/tmp/taide-batch21-documentation-app-wiring-after-lifetimes.log`에서 exit 0이며 기존 Wry 경고 17건만 남았습니다. 이후 표시 변경은 전체 게이트 전에 다시 확인합니다.

UI inspection의 최신 6건은 `/private/tmp/taide-batch21-documentation-memory-ui-focus-loading.log`에서 exit 0, 6통과/0실패입니다. 150ms 요청·300ms 표시/숨김과 sticky, 명시 호버 재실행 포커스·Escape 이후 같은 프레임 본문 문자, 120ms trailing 시그니처·복수 서명 키, provider 교체/선택 변경, 900ms Loading과 부분 문서 유지, 다음 버튼 클릭 후 같은 프레임 문자/본문 포커스를 확인했습니다. OS 합성 입력은 사용하지 않았습니다.

최초 sticky 검사는 SDK의 이전 pass response 좌표를 사용해 실패했습니다. 현재 EditorOutput inspection 좌표로 관찰하도록 수정한 뒤 통과했습니다. Loading 최초 pass는 Area 크기를 계산하는 프레임이므로 실제 painting 프레임에서 검사합니다. 버튼 클릭 뒤 본문 포커스가 사라지는 실패는 SDK의 기존 pointer-preserves-keyboard-focus 계약을 버튼에 등록해 수정했습니다. 실패 로그는 `/private/tmp/taide-batch21-documentation-memory-ui-loading-buttons.log`(4통과/2실패), 좌표 실패는 `/private/tmp/taide-batch21-documentation-sticky-observation.log`입니다. 크기 조절·Markdown 코드/이미지·순수 peek의 실제 앱 소비와 전체 게이트는 계속 진행 중입니다.

## 이미지의 앱 공급과 파일 경계

원본 Markdown renderer의 HTTP/HTTPS·file·data 이미지와 `|width=... height=...`를 기존 앱 preview 디코더/텍스처·애니메이션 playback에 연결했습니다. 새 이미지 engine/egui loader 의존성은 추가하지 않았고 이미 app dev에 있던 base64 =0.23.1을 같은 버전의 일반 의존성으로 옮겼습니다. app lock의 기존 base64 package/graph는 변하지 않았으며 동결 browser에는 변경이 없습니다.

이미지 task는 현재 문서 payload의 프로젝트/URI/renderer 한계로 캐시하며 문서·프로젝트/뷰 닫힘에서 취소하고 회수합니다. 파일은 현재 소유 프로젝트 root 안의 실제 파일만 읽고 canonical symlink 밖은 거절합니다. HTTP는 기존 API client timeout과 스트리밍 파일 상한, data는 bounded decode, 실제 디코딩/애니메이션은 기존 RGBA/텍스처 상한을 사용합니다. 동시에 진행하는 디코딩 수는 기존 두 메모리 상한의 비율로 제한합니다. pending/실패는 alt 문구를 보존합니다.

최신 이미지 4건은 `/private/tmp/taide-batch21-documentation-images-after-paint-observation.log`, exit 0·테스트 실행 0.20초입니다. 실제 PNG 픽셀·렌더 출력/크기·중복 캐시·root/symlink 격리·잘못된 데이터·SVG data URI·animation step/예산·실제 loopback HTTP 응답과 닫힌 대기 요청/종료 task 0을 검사했습니다. 최초 Context 참조 누락은 `images-first` 로그의 컴파일 오류로 수정했고, `images-after-context`의 실패는 egui RectShape를 Mesh로 직접 관찰한 검사 오류와 미회수 TextureDelta였습니다. 실제 tessellation 출력으로 관찰하고 테스트 texture delta를 명시 회수한 뒤 통과했습니다. 사용자 데이터/OS 합성 입력은 사용하지 않았습니다.

## 코드·자동 링크와 실제 peek의 추가 근거

최신 화면 13건은 `/private/tmp/taide-batch21-documentation-readonly-views-after-expected-error.log`, exit 0입니다. 이미지 링크 클릭/포커스·짧은 내용의 너비/높이 상한·긴 내용의 축소와 readonly/같은 모델의 두 뷰를 포함합니다. 짧은 내용을 빈 공간까지 늘리는 실패는 `image-link-resize-first`로 재현했고 실제 폰트의 무줄바꿈 폭·전체 내용 높이·배치 방향의 여유 공간으로 제한했습니다. 원본 max(editorWidth ×0.66, 750) 초기 폭과 내용 높이/화면 cap을 사용합니다. readonly의 Text 거절은 정상 ReadOnly 오류이며 검사 harness에서 그 한 입력만 명시적으로 기대하도록 수정했습니다. 두 이미지 문서의 독립 ID/두 번째 링크 클릭도 `multiple-doc-ids-first`에서 통과했습니다.

파일 링크 3건은 `/private/tmp/taide-batch21-documentation-file-links-after-uri.log`, exit 0입니다. Monaco #L줄,열 및 range의 시작 위치·좌표 상한/기본값, 실제 root 권한·본문/보조 창 pane/viewport·dirty 같은 탭 재사용·owner/프로젝트 만료를 검사했습니다. host는 mutation guard 안에서 활성 원본문 owner와 허용 경로를 확인하며 최신 목적지 layout만 앱 reveal에 소비합니다. 최초 typed Uri 참조 오류는 as_str API로 수정했고 검사명 snake-case 경고도 제거했습니다.

본문까지 확장한 실제 앱 1건은 `main-peek-integration-first`에서 통과했고 열린 도움말을 유지한 종료까지 추가한 최신 검사는 `/private/tmp/taide-batch21-documentation-main-peek-exit.log`, exit 0·3.98초입니다. 본문/peek typed 응답·실제 TextMate 코드 색/서명 순환/포커스·수정 없는 문서, 닫힘의 요청 취소·소유 코드 모델만 회수·종료 task 0을 확인했습니다. 일반 close와 직접 on_exit 모두 도움말 resources를 회수합니다.

popup State.close가 HTTP 이미지 대기를 다음 paint까지 유지하는 실패는 `image-close-first`에서 실제 socket EOF 대기 만료로 재현했습니다. 닫는 즉시 남은 활성 문서에 포함되지 않는 이미지 task를 취소합니다. 최신 이미지 4건은 `/private/tmp/taide-batch21-documentation-image-close-after-retain.log`, exit 0·0.20초입니다. 다른 뷰/서명에서 같은 URI를 쓰면 유지하며 현재 선택되지 않은 서명의 문서는 이미지/코드 로딩에서 제외합니다. 선택 서명 제외 회귀와 최신 변경 상태는 전체 게이트에서 확인합니다.

코드 블록은 기존 TextMate pipeline/테마와 편집기 글꼴을 사용합니다. Monaco 등록 파일 81개에서 별칭 139개를 추출한 앱 전용 JSON과 canonical/플러그인 별칭을 사용하며 언어가 없으면 현재 문서 언어, 알려지지 않은 언어는 plaintext입니다. 코드 소스는 이 캐시가 소유하는 무뷰 임시 모델이고 LSP/탭/사용자 draft에 등록하지 않습니다. 초기 정리 검사의 UnsavedChanges 실패는 untitled 모델의 저장 상태와 맞지 않는 release API 사용이 원인이었습니다. 캐시 소유 ID·생성 revision·무뷰 조건을 검증하는 discard API로 수정했고 dirty 본문/뷰를 보존하며 pipeline 회수를 확인했습니다. 최신 1건은 `/private/tmp/taide-batch21-documentation-code-theme-after-owned-discard.log`에서 exit 0입니다. 필터 오기로 0건 실행한 log는 성공 검증 근거로 사용하지 않습니다.

GFM 자동 링크가 누락되는 실패를 `/private/tmp/taide-batch21-documentation-gfm-first.log`(7통과/1실패)로 재현했습니다. 설치된 Monaco Marked의 URL/email·문장부호 규칙을 기존 앱 전용 syntax 경계의 compiler로 평가합니다. editor/UI에 engine이나 의존성을 추가하지 않았습니다. 최신 Markdown 8건은 `/private/tmp/taide-batch21-documentation-gfm-after.log`에서 exit 0입니다. 코드/기존 링크 내부와 plaintext는 재해석하지 않습니다.

provider 교체 뒤 포커스된 호버의 같은 프레임 문자 누락과 미연결 크기 조절은 `/private/tmp/taide-batch21-documentation-focus-resize-first.log`에서 실패 재현했습니다. 포커스 회수·입력 소유 인계와 오른쪽/배치 방향의 상하 sash를 연결했습니다. IME 조합 동안 명시 요청을 거절하고 modifier를 누른 명시 호버는 유지합니다. 최신 도움말 UI 10건과 플랫폼 키 8건은 `/private/tmp/taide-batch21-documentation-command-modifier-after.log`에서 모두 exit 0입니다. keymap은 원본처럼 command modifier가 있는 조합 중 단축키를 dispatch할 수 있고 실제 명령 소비자가 조합을 보존하며 요청을 거절합니다. 초기 잘못된 keymap 검사 가정은 소비자 검사로 옮겼습니다.

실제 앱 fixture는 본문과 같은 root의 순수 peek만 mirror에 등록해 typed 호버/서명 응답과 실제 문구·UTF-16 인자 [2,4]→byte 2..6·원본문 owner/preview source를 확인했습니다. modifier 숨김과 외부 위치 위젯의 Cmd+K prefix 가로채기를 각각 실패 재현 뒤 수정했습니다. preview에는 prefix를 전달하고 Cmd+K F2가 소비되면 preview keymap의 대기 chord도 회수합니다. 기존 dirty 미리보기 편집/찾기/저장·실제 탭 인수·순환/종료 task 회수도 함께 통과했습니다. 최신 실제 앱 1건은 `/private/tmp/taide-batch21-documentation-actual-peek-after-prefix.log`, exit 0·테스트 실행 3.82초입니다. 이전 실패 log는 `actual-peek-first`와 `actual-peek-after-modifier`입니다. 원본 ImageIO/loopback/file-watch 경계만 escalation으로 검사하고 OS 합성 입력/사용자 데이터/Trash는 사용하지 않았습니다.

## 전체 대상 실행과 실패 영향 재검사

변경 크레이트마다 공통 `--locked --offline --target-dir experiments/native-shell-spike/target --all-targets --no-fail-fast`를 사용했고 UI/app은 `--features inspection`을 포함했습니다. SDK 문서 대상은 별도 `--doc --no-fail-fast`로 실행했습니다. 실행한 Cargo process의 실제 종료를 확인한 뒤 다음 Cargo를 시작했습니다.

| 대상    | 최초 전체 로그                                                        | 대상 수 | 최초 통과/실패/ignored | 현재 서로 다른 통과 |
| ------- | --------------------------------------------------------------------- | ------: | ---------------------- | ------------------: |
| editor  | `/private/tmp/taide-batch21-full-editor.log`                          |      31 | 219/0/1, exit 0        |                 219 |
| UI      | `/private/tmp/taide-batch21-full-ui.log`                              |      21 | 389/1/0, exit 101      |                 390 |
| app     | `/private/tmp/taide-batch21-full-app.log`                             |      67 | 716/3/0, exit 101      |                 719 |
| LSP SDK | `/private/tmp/taide-batch21-full-lsp-sdk.log`, `full-lsp-sdk-doc.log` |       2 | 84/0/0, 문서 exit 0    |                  84 |

전체 121대상과 실패 영향 재검사를 합친 현재 서로 다른 성공은 1412건입니다. 최초 실패를 삭제하거나 전체 재실행 통과로 바꾸어 쓰지 않습니다. SDK 전체 84건의 결과는 `test result: ok`이며 문서 대상은 0건입니다. 이번 상태에서 추가한 비활성 서명 문서의 코드/이미지 공급 제외 검사도 앱 전체에서 통과했습니다.

UI의 실패는 전체 명령 목록의 기대 집합에 ShowHover/TriggerParameterHints 두 명령이 빠진 검사 오류였습니다. native-host gate를 적용한 두 기대값을 추가했고 `/private/tmp/taide-batch21-full-ui-lib-after-catalog.log`에서 library 123건/exit 0을 확인했습니다. 전체 UI의 나머지 성공 결과는 재사용합니다.

app의 세 실패는 workspace/document symbol과 folding의 기존 실제 child 초기화/전체 대기 시간 초과였습니다. 최초 workspace 관찰값은 Running 대신 Initializing이고 stderr는 비어 있었습니다. production 코드나 timeout을 변경하지 않고 실패 세 건만 각각 실행해 `full-app-retry-workspace`·`full-app-retry-document`·`full-app-retry-folding.log`에서 모두 1통과/exit 0을 확인했습니다(실행 1.40/1.35/1.65초). 동시에 많은 child를 시작한 전체 실행의 초기화 부하가 원인으로 추정됩니다. 원인을 확정한 성능 분석은 아니며 다음 전체 검증부터 테스트 내부 동시 실행을 제한해 재발을 관찰합니다. 해당 단독 성공으로 실제 서버/성능 soak를 통과 처리하지 않습니다.

실제 Trash를 사용하는 remote file 14명령·workspace explorer delete·workspace delete 세 검사는 명시 `--skip`으로 보호했습니다. editor의 기존 성능 1ignored, 변경 없는 syntax 3ignored/egui SDK 문서 1ignored와 실기 pixel/OS IME/접근성·대형 문서/도움말 성능·보조 창 실기/soak·패키징은 이번 성공에 더하지 않습니다. 기존 Wry 경고 17개와 큰 `__eh_frame` 링크 경고는 남아 있습니다.

동결 host `check --tests`는 `/private/tmp/taide-batch21-web-host.log` exit 0·6.46초, Wasm `check --features canvas --target wasm32-unknown-unknown`는 `web-wasm.log` exit 0·2.66초입니다. 공통 locked/offline/target-dir를 유지했습니다. source/features/manifest/lock/graph 변경은 없고 editor/UI manifest/lock도 변하지 않았습니다. app만 pulldown-cmark/unicase 두 패키지를 추가하고 기존 base64의 dev→일반 이동을 유지합니다. ferriki-textmate =0.12.0/ferroni =1.8.1과 앱 전용 syntax engine 경계를 보존했습니다.

editor/UI/app/SDK의 `cargo fmt -- --check`는 `/private/tmp/taide-batch21-*-fmt-check.log`에서 모두 exit 0입니다. 마지막 UI 기대값 배열의 포맷 check 실패는 해당 배열 줄바꿈만 수정한 뒤 확인했고 의미 변경 없는 포맷 때문에 통과한 검사를 반복하지 않았습니다. scoped diff check와 QA/별칭/키 JSON Prettier check exit 0이며 `df -h .`의 디스크 여유는 623GiB·사용 66%입니다. 보호 번들/실제 앱 데이터/OS/clipboard/Keychain/Trash를 변경하지 않았습니다.
