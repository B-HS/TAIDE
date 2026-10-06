export const meta = {
  name: 'native-batch7-language-config-editing-commands',
  description: 'native 전환 배치 7: 언어 구성과 자동 들여쓰기·괄호 쌍, 편집 명령과 Monaco 기본 키를 직렬 구현하고 찾기 정규식 방언을 조사',
  phases: [
    { title: 'Research', detail: '찾기 위젯 정규식 방언 조사(읽기 전용)', model: 'opus' },
    { title: 'Implement', detail: '직렬 3단계 구현과 검증', model: 'opus' },
    { title: 'Review', detail: '커밋 기준 대비 변경 리뷰', model: 'sonnet' },
    { title: 'Fix', detail: '리뷰 차단 항목 수정과 재검증', model: 'opus' },
  ],
}

const ROOT = '/Users/hyunseokbyun/development/TAIDE'
const BASE_COMMIT = '06b19ca0'
const TARGET = ROOT + '/experiments/native-shell-spike/target'
const QA = ROOT + '/docs/quality-assurance'
const DESIGN = ROOT + '/docs/research/2026-10-06-native-editor-display-layer-design.md'
const NA = ROOT + '/native/taide-native-app'
const NU = ROOT + '/native/taide-native-ui'
const NE = ROOT + '/native/taide-native-editor'
const NS = ROOT + '/native/taide-native-syntax'
const S = ROOT + '/src'
const REG = '/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f'
const CARGO_TAIL = `--locked --offline --target-dir ${TARGET}`

const BACKGROUND = `
당신은 TAIDE(IDE) 저장소 ${ROOT} 의 Rust-native 전환 작업자입니다. 응답과 문서는 한국어 존댓말, 이모지·아스키아트 금지입니다.

# 배경 (메인이 확인한 사실)
- 기존 제품은 Tauri + React/TypeScript 입니다(TS UI: ${S}). 목표는 TS view 를 걷어내고 UI 까지 Rust-native(egui/eframe 0.36.2)로 만드는 것이며, 기존 TS 화면·상태·상호작용(편집기는 Monaco + Shiki 의 실제 동작)을 그대로 재현합니다. 새 기능·새 디자인을 추가하지 않습니다.
- native 구현은 ${ROOT}/native 에 있습니다: taide-native-app(실행 파일), taide-native-ui(공유 UI, 브라우저 Wasm 클라이언트 taide-remote-web 도 소비), taide-native-editor(rope 문서·뷰 스토어·표시 모델), taide-native-terminal, taide-native-retained. 각 크레이트는 독립 workspace 와 자체 Cargo.lock 을 갖습니다. taide-remote-web 은 동결 상태이며 컴파일만 유지합니다(신규 기능 금지, 의존 그래프와 Cargo.lock 변경 금지).
- kebab-case.rs 파일은 lib.rs 에서 #[path] 로 연결됩니다. *-tests.rs 는 테스트입니다.
- 현재 HEAD 는 ${BASE_COMMIT} 이고 배치 시작 시점의 작업 트리는 깨끗합니다.
- 설계 정본은 ${DESIGN} 입니다. 0절 "읽는 법"의 약어, 2.4절(syntax.rs 와 install_syntax), 5절(구문 강조 엔진), 6절(테마 연결), 7절 "단계 3"을 먼저 읽으십시오. 설계 문서는 빌드·테스트 없이 작성됐으므로 서술과 줄 번호를 실제 파일로 다시 확인하고, 어긋나면 실제 파일을 따르되 차이를 QA 문서에 기록합니다.
- 사용자 결정(${ROOT}/docs/acknowledge/2026-10-06-native-transition-decisions.md): 엔진은 ferriki-textmate =0.12.0 + ferroni =1.8.1 을 적합성 게이트 통과 조건으로 채택, 새 의존성 반입 승인, 문법 자산은 @shikijs/langs 4.4.3 의 30종과 임베드 모듈을 회색 지대 4종(elixir·toml·yaml·erb)까지 전부 포함. lockfile 변경은 ${NA}/Cargo.lock 과 신규 ${NS}/Cargo.lock 에 한정합니다.
- 두 크레이트의 소스는 이미 cargo registry 에 내려받아져 있습니다: ${REG}/ferriki-textmate-0.12.0, ${REG}/ferroni-1.8.1. 네트워크 없이 --offline 으로 해석·빌드할 수 있습니다. 메인이 확인한 것: ferriki-textmate 는 build 스크립트가 없고 src/grammar.rs 에 tokenize_line, tokenize_line2, src/registry.rs 에 SyncRegistry, src/parse_raw_grammar.rs 에 parse_raw_grammar 가 있습니다. ferroni 의 build.rs 는 기본으로 꺼진 ffi feature 에서만 C 소스를 컴파일합니다. API 는 기억이 아니라 이 소스와 README·docs 주석을 직접 읽어 사용합니다.
- 직전 배치들의 결과는 ${QA}/2026-10-06-native-batch{1,2,3,4,5,6}-*.md 에 있습니다. 관련 요점: 편집 입력 보정(배치 1: 이동·삭제 키, undo 그룹, Enter 선행 공백 유지, Shift+Tab), 명령 레지스트리(배치 2: ${NU}/src/command-registry.rs 에 TS 등록 명령 212개, 실행 경로 있음/없음 구분, ${NA}/src/command-dispatch.rs), 표시 계층·word wrap(배치 4), 구문 강조와 taide-native-syntax 크레이트(배치 5: ferriki-textmate + ferroni, 편집 저널, 줄 토큰), 장식·접기(배치 6: decoration.rs, folding.rs, EditorRequest·show_request). 기준 테스트 수(메인이 직접 실행): native 앱 전체 608 통과(인수 이전부터 실패하는 tests/preview-spreadsheet-xlml.rs 1건 제외), taide-native-syntax 89, taide-native-editor 124, taide-native-ui 전체 241, taide-remote-web 53.
- 디스크 사용을 줄이기 위해 필요한 테스트 대상만 빌드하고, native 앱의 전체 테스트 대상(필터 없는 cargo test)은 실행하지 않습니다(메인이 배치 끝에 1회 실행).
- ${ROOT}/docs/PROCESS.md(약 1MB)와 ${ROOT}/docs/HANDOFF.md 는 읽지 마십시오.
`

const IMPL_RULES = `
# 절대 규칙
- 파일 수정은 Edit·Write 도구로만 합니다. 파일 읽기·검색은 Read·Grep·Glob 을 씁니다.
- 셸은 절대 경로를 쓴 한 줄 단순 명령만 허용합니다: grep, rg, ls, wc, find(저장소 경로 한정), sed -n, cat, diff, git status, git diff, git log, git show, cargo check, cargo test, cargo fmt, cargo clippy, 그리고 아래 "단계별 허용"에 적힌 명령. cd, 파이프, &&, ;, 변수 대입, 루프, 명령 치환, heredoc, python, sed -i, rm, npm 은 금지입니다. 홈 디렉터리나 저장소 밖을 탐색하지 않습니다(예외: ${REG} 와 ${ROOT}/node_modules 읽기).
- cargo 는 "--manifest-path <절대경로> ... ${CARGO_TAIL}" 형태로, 한 번에 하나만 실행합니다(cargo fmt 는 --manifest-path 만 붙입니다). 다른 target 디렉터리를 만들지 않습니다. cargo clean 은 실행하지 않습니다.
- Cargo.toml·Cargo.lock 변경은 "단계별 허용"에 적힌 범위에서만 합니다. 그 밖의 새 의존성이 필요하다고 판단되면 구현하지 말고 unresolved 에 적어 보고합니다.
- git 상태를 바꾸는 명령(add, commit, stash, checkout, reset, restore, push)은 금지입니다. 구현을 임시로 끄거나 되돌려 실패를 재현하는 편집도 하지 않습니다. 다른 에이전트를 띄우지 않습니다.
- GUI 앱을 실행하지 않습니다. 사용자 데이터, OS 설정, 실제 클립보드·Keychain·Trash 를 건드리지 않습니다. 테스트는 합성 fixture 만 사용합니다. 네트워크에 접근하지 않습니다.
- .env, 키 파일은 읽지 않습니다.
- 지정된 범위 밖 파일은 원칙적으로 읽기 전용입니다. 범위 밖 수정이 불가피하면 최소한으로 하고 changedFiles 와 QA 문서에 사유를 적습니다. 요청 밖 리팩터링·리네임·재포맷은 금지입니다.
- 코드 주석을 새로 쓰지 않습니다(기존 주석은 유지). 이름·구조·타입으로 설명합니다. 매직넘버는 의미 있는 const 로 둡니다. #[allow(...)] 로 lint 를 끄거나 증상만 덮는 우회를 하지 않고 근본 원인을 고칩니다. 테스트 기대값을 구현에 맞춰 바꾸지 않습니다. 주변 코드의 스타일·에러 처리·테스트 작성 방식(테스트 이름이 한국어인 파일은 한국어)을 그대로 따릅니다.
- 사용자에게 보이는 문자열은 taide-locale 카탈로그 키를 쓰고 하드코딩 영어 문구를 추가하지 않습니다.
- TS(Shiki·Monaco)에 없는 동작을 추측으로 만들지 않습니다. 각 동작은 설계 문서가 인용한 TS·Shiki·Monaco 소스를 직접 확인해 근거로 삼고 QA 문서에 경로를 적습니다. 엔진의 출력이 TS 기준과 다를 때 기준은 TS 출력입니다. 기준 자료를 엔진 출력에 맞춰 고치지 않습니다.
- 제3자 자산(문법 JSON, 옮긴 Monaco·Shiki 소스)은 출처와 라이선스 고지를 함께 추가합니다. 문법 파일은 추출한 문자열을 수정 없이 저장합니다.
- 컨텍스트 절약(직전 배치에서 작업자가 컨텍스트 한도를 넘겨 중단된 적이 있습니다): 큰 파일(editor_surface.rs, application.rs, store.rs, editing.rs, command-registry.rs, Monaco 소스)은 통째로 읽지 말고 Grep 으로 위치를 찾아 필요한 구간만 offset·limit 으로 읽습니다. cargo 는 --quiet 를 붙이고 실패 출력이 길면 테스트 이름으로 필터해 다시 실행합니다. 같은 파일을 반복해서 다시 읽지 않습니다. QA 문서는 작업 절반 지점에 초안을 써 두고 끝에 갱신합니다. 작업 항목이 한 번에 끝내기에 너무 크면 완성된 부분이 컴파일·테스트 통과 상태가 되게 마무리하고 남은 항목을 unresolved 에 정확히 적어 반환합니다.
- 같은 가정에 기반한 수정이 세 번 연속 실패하면 멈추고 unresolved 에 원인 가설과 함께 보고합니다. 실행하지 않은 검사를 통과했다고 쓰지 않습니다.
`

const IMPL_SCHEMA = {
  type: 'object',
  properties: {
    step: { type: 'string' },
    summary: { type: 'string' },
    gatePassed: { type: 'boolean', description: '엔진 게이트 단계에서만 의미가 있음. 다른 단계는 true' },
    gateDetail: { type: 'string' },
    changedFiles: { type: 'array', items: { type: 'string' } },
    completed: { type: 'array', items: { type: 'string' } },
    verification: {
      type: 'array',
      items: {
        type: 'object',
        properties: { command: { type: 'string' }, exitCode: { type: 'integer' }, result: { type: 'string' } },
        required: ['command', 'exitCode', 'result'],
      },
    },
    unresolved: { type: 'array', items: { type: 'string' } },
    qaDocPath: { type: 'string' },
  },
  required: ['step', 'summary', 'gatePassed', 'gateDetail', 'changedFiles', 'completed', 'verification', 'unresolved', 'qaDocPath'],
}

const REVIEW_SCHEMA = {
  type: 'object',
  properties: {
    step: { type: 'string' },
    verdict: { type: 'string', enum: ['pass', 'needs-fix'] },
    blocking: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          file: { type: 'string' },
          line: { type: 'integer' },
          problem: { type: 'string' },
          failureScenario: { type: 'string' },
          fix: { type: 'string' },
        },
        required: ['file', 'problem', 'failureScenario', 'fix'],
      },
    },
    nonBlocking: { type: 'array', items: { type: 'string' } },
    outOfScopeChanges: { type: 'array', items: { type: 'string' } },
  },
  required: ['step', 'verdict', 'blocking', 'nonBlocking', 'outOfScopeChanges'],
}

const STEPS = [
  {
    key: 'language-config',
    title: '언어 구성과 자동 들여쓰기·괄호 쌍',
    designRef: '7절 "단계 8 이후"의 단계 9(언어 구성과 괄호), 3.1·3.2절의 autoIndent·autoClosing 관련 항목',
    scope: `${NS}/src·tests·Cargo.toml·Cargo.lock(아래 허용 범위), ${NS} 의 언어 구성 자료와 추출 스크립트(${ROOT}/docs/utils), ${NE}/src/{lib.rs,editing.rs,indent.rs,folding.rs} 와 신규 모듈, ${NE}/tests, ${NU}/src/editor_surface.rs(입력 경로의 최소 수정), ${NA}/src/{editor-syntax.rs,application.rs}(언어 구성 공급 배선), ${ROOT}/THIRD_PARTY_LICENSES.md 와 고지 파일`,
    allowed: `
- 정규식이 필요한 규칙(indentationRules, onEnterRules, 접기 표식)은 taide-native-editor 에 정규식 의존성을 추가하지 않고, 이미 승인·고정된 ferroni =1.8.1 을 쓰는 taide-native-syntax 쪽에서 평가합니다. ${NS}/Cargo.toml 에 ferroni = "=1.8.1" 를 직접 의존성으로 추가하는 것과 그에 따른 ${NS}/Cargo.lock·${NA}/Cargo.lock 의 최소 갱신(cargo check --offline, --locked 없이 1회씩)만 허용합니다. crates.io 의 새 크레이트는 추가하지 않고 기존 항목의 버전이 바뀌면 중단해 보고합니다. ${NU}·${NE}·taide-remote-web 의 Cargo.toml·Cargo.lock 은 바뀌지 않아야 합니다.
- 자료 추출을 위해 bun <절대 경로의 스크립트> 실행을 허용합니다(스크립트는 ${ROOT}/docs/utils 아래, 저장소의 node_modules 만 사용, 네트워크·저장소 밖 쓰기 금지). 거부되면 우회하지 말고 보고합니다.`,
    tasks: `
C1. TS 기준 확정: TS 가 등록하는 언어 id(구문 강조 단계의 언어 목록) 각각에 대해 Monaco 가 가진 언어 구성(node_modules/monaco-editor 의 basic-languages 각 언어 conf 와 language 서비스 계열의 구성: comments, brackets, autoClosingPairs, surroundingPairs, indentationRules, onEnterRules, wordPattern, folding.markers·offSide)을 확인하고, Monaco 에 등록되지 않은 언어 id 는 TS 에서 어떤 구성을 갖는지(없으면 기본 구성) 확인합니다. TS 의 autoIndent, autoClosingBrackets, autoClosingQuotes, autoSurround, autoClosingDelete, autoClosingOvertype 설정값도 TS 소스에서 확인합니다.
C2. 언어 구성 자료: Monaco 의 구성을 추출 스크립트로 자료 파일(정규식은 소스 문자열과 플래그 그대로)로 뽑아 taide-native-syntax 에 포함하고 출처·라이선스(Monaco MIT)를 고지합니다. JS 정규식을 ferroni(Oniguruma 문법)로 평가할 때의 문법 차이를 조사해, 추출된 모든 규칙 정규식이 컴파일되는지와 Monaco 소스의 대표 입력에서 JS 와 같은 결과를 내는지 테스트로 고정합니다(기준 결과는 추출 스크립트에서 JS 로 계산해 fixture 로 둡니다). 차이가 나는 규칙은 임의로 고치지 말고 목록으로 보고합니다.
C3. Enter 동작: Monaco 의 Enter 처리(getEnterAction 과 autoIndent full 의 getInheritIndentForLine·getGoodIndentForLine·getIndentForEnter·getIndentActionForType 계열, 설계가 인용한 소스와 cursorTypeEditOperations 를 직접 확인)를 옮겨, 괄호 뒤 Enter 의 추가 들여쓰기, 괄호 사이 Enter 의 두 줄 벌리기, 주석 블록 이어쓰기(onEnterRules), 닫는 괄호 입력 시 내어쓰기를 구현합니다. 편집 코어(taide-native-editor)는 정규식을 모르는 인터페이스(자료형 또는 trait)만 보고, 평가는 앱이 taide-native-syntax 로 수행해 넘깁니다. 언어 구성이 없는 문서(동결된 브라우저 클라이언트 포함)는 배치 1 의 현재 동작(선행 공백 유지)을 유지합니다.
C4. 괄호·따옴표 쌍: 자동 닫기(조건: 뒤 문자, 문자열·주석 안 여부는 구문 토큰의 표준 토큰 종류 사용), 닫는 문자 덮어쓰기(overtype), 쌍 삭제(Backspace), 선택 감싸기(surround)를 Monaco 규칙대로 구현합니다. 다중 커서와 undo 그룹, IME 조합 중 입력과 올바르게 결합해야 합니다.
C5. 접기 표식과 offSide: folding.rs 의 영역 계산에 언어 구성의 markers(region 표식)와 offSide 규칙을 연결합니다(Monaco indentRangeProvider.js 근거). 배치 6 이 "언어 구성 없음"으로 실행 불가로 둔 접기 명령 중 이것으로 가능해지는 것(foldAllMarkerRegions 등)을 명령 레지스트리에서 실행 가능으로 바꿉니다.
비목표: 괄호 매칭 강조와 괄호 쌍 색·가이드(다음 배치), 주석 토글 명령(다음 단계), wordPattern 을 쓰는 단어 이동·더블클릭 선택의 변경은 C1 에서 차이만 기록, LSP 기반 on-type formatting.`,
    verify: `
C2·C3·C4 는 실패하는 테스트부터 작성합니다. Enter·쌍 동작의 기대값은 Monaco 소스의 규칙에서 유도하고 근거 줄을 QA 문서에 적습니다.
V1. cargo test --quiet --manifest-path ${NS}/Cargo.toml ${CARGO_TAIL}
V2. cargo test --quiet --manifest-path ${NE}/Cargo.toml ${CARGO_TAIL}
V3. cargo test --quiet --manifest-path ${NU}/Cargo.toml --lib --test editor_surface ${CARGO_TAIL}
V4. cargo check --quiet --manifest-path ${NA}/Cargo.toml ${CARGO_TAIL}, cargo test --quiet --manifest-path ${NA}/Cargo.toml --lib ${CARGO_TAIL}(실패 0 유지), cargo test --quiet --manifest-path ${NA}/Cargo.toml --test save --test save-syntax --test paste-shortcuts ${CARGO_TAIL}
V5. cargo check --quiet --manifest-path ${ROOT}/native/taide-remote-web/Cargo.toml ${CARGO_TAIL}
V6. 변경한 크레이트마다 cargo fmt --manifest-path <Cargo.toml> -- --check, git diff 로 lockfile 변경이 허용 범위인지 확인`,
  },
  {
    key: 'line-commands',
    title: '편집 명령 1: 줄·텍스트 조작과 Monaco 기본 키',
    designRef: '3.2절(제품이 쓰는 Monaco 기능)과 감사 보고서 2026-10-06-native-audit-editor.md 의 "편집 명령" 항목',
    scope: `${NE}/src/{editing.rs,lib.rs} 와 신규 모듈(줄·텍스트 명령), ${NE}/tests, ${NU}/src/{command-registry.rs,keymap.rs,keymap-defaults.json,editor_surface.rs(명령 적용 진입점)}, ${NA}/src/{command-dispatch.rs,shell_keymap.rs,application.rs(명령 큐 배선의 최소 수정)}, 관련 테스트`,
    allowed: `- 이 단계에서는 Cargo.toml·Cargo.lock 을 수정하지 않습니다.`,
    tasks: `
E1. 대상 확정: 명령 레지스트리의 monaco.* 명령 중 "줄·텍스트 조작"에 해당하고 아직 실행 경로가 없는 것을 전수 열거합니다. 최소 범위: 줄 위·아래 이동, 줄 위·아래 복사, 줄 삭제, 위·아래에 줄 삽입, 줄 합치기, 줄 오름·내림차순 정렬, 중복 줄 제거(TS 가 등록했다면), 글자·단어 맞바꾸기(transpose), 대소문자 변환(upper·lower·title·snake·camel·kebab 등 TS 가 등록한 것), 줄 들여쓰기·내어쓰기(editor.action.indentLines·outdentLines), 후행 공백 제거, 줄 주석 토글·블록 주석 토글·줄 주석 추가·제거(앞 단계의 언어 구성 comments 사용), 왼쪽·오른쪽 전부 삭제, 단어 단위 삭제 변형. 각 명령의 정확한 동작(다중 커서, 선택 유무, 빈 줄, 마지막 줄, 접힌 영역, 읽기 전용)을 Monaco 소스(node_modules/monaco-editor/esm/vs/editor/contrib/linesOperations, comment, wordOperations 등)에서 확인합니다.
E2. 구현: 각 명령을 편집 코어의 순수 함수(문서 스냅샷·선택 → 트랜잭션)로 구현하고 undo 한 단위로 적용합니다. 실행은 배치 2·6 이 만든 경로(Run → ShellIntent 의 문서 편집 큐 → 다음 프레임 적용)를 따르며, 명령 레지스트리에서 해당 명령을 실행 가능으로 바꿉니다.
E3. Monaco 기본 키: TS 에서 Monaco 가 편집기 문맥에서 기본으로 처리하던 키(이 단계 대상 명령과 배치 6 의 접기 명령: ⌥↑/↓ 줄 이동, ⇧⌥↑/↓ 줄 복사, ⇧⌘K 줄 삭제, ⌘Enter·⇧⌘Enter 줄 삽입, ⌘/ 줄 주석, ⇧⌥A 블록 주석, ⌘]·⌘[ 들여쓰기, ⌥⌘[·⌥⌘] 접기, ⌘K 로 시작하는 chord 등)를 Monaco 소스의 keybinding 등록(macOS 와 그 외 플랫폼 분기 포함)에서 확인해 native 키맵에 편집기 문맥(when) 기본 키로 추가합니다. TS 가 Monaco 기본 키를 제거하거나 바꾼 것(${S}/shared/lib/monaco 의 keybinding 조정)을 반영하고, 앱 전역 키맵과 충돌하면 TS 에서의 우선순위를 따릅니다. 사용자 키 재지정(monaco 명령 행 override)이 이 기본 키에도 적용되게 합니다.
E4. 키바인딩 편집기에 표시되는 기본 키 라벨이 실제 동작하는 키와 일치하는지 확인합니다.
비목표: 커서·선택·다중 커서 명령(다음 단계), 찾기, LSP 명령, 스니펫, 포맷.`,
    verify: `
명령마다 Monaco 동작에서 유도한 실패하는 순수 테스트부터 작성합니다(다중 커서와 경계 포함).
V1. cargo test --quiet --manifest-path ${NE}/Cargo.toml ${CARGO_TAIL}
V2. cargo test --quiet --manifest-path ${NU}/Cargo.toml --lib --test editor_surface --test keymap-platform ${CARGO_TAIL}
V3. cargo check --quiet --manifest-path ${NA}/Cargo.toml ${CARGO_TAIL}, cargo test --quiet --manifest-path ${NA}/Cargo.toml --lib ${CARGO_TAIL}(실패 0 유지; 키맵·키바인딩 카탈로그 fixture 테스트 포함)
V4. cargo check --quiet --manifest-path ${ROOT}/native/taide-remote-web/Cargo.toml ${CARGO_TAIL}
V5. 변경한 크레이트마다 cargo fmt --manifest-path <Cargo.toml> -- --check`,
  },
  {
    key: 'cursor-commands',
    title: '편집 명령 2: 커서·선택·다중 커서',
    designRef: '3.2절(다중 커서)과 감사 보고서 2026-10-06-native-audit-editor.md 의 "다중 커서·컬럼 선택" 항목',
    scope: `${NE}/src/{editing.rs,view.rs,lib.rs} 와 신규 모듈(커서·선택 명령), ${NE}/tests, ${NU}/src/{command-registry.rs,keymap.rs,keymap-defaults.json,editor_surface.rs(포인터·키 입력 경로)}, ${NA}/src/{command-dispatch.rs,shell_keymap.rs}, 관련 테스트`,
    allowed: `- 이 단계에서는 Cargo.toml·Cargo.lock 을 수정하지 않습니다.`,
    tasks: `
U1. 대상 확정: 명령 레지스트리의 monaco.* 명령 중 "커서·선택·다중 커서"에 해당하고 아직 실행 경로가 없는 것을 전수 열거합니다. 최소 범위: 위·아래에 커서 추가, 줄 끝마다 커서 추가, 다음 일치 항목 선택 추가(⌘D)·건너뛰기·이전 일치, 모든 일치 항목 선택, 줄 선택(⌘L), 선택 확장·축소(smart select: Monaco 가 LSP 없이 단어·괄호로 제공하는 범위), 커서 undo·redo, 괄호로 이동·괄호까지 선택(언어 구성의 brackets 사용), 단어 부분 이동(TS 가 등록했다면), 선택 영역을 위·아래로 한 줄 확장. Monaco 소스(contrib/multicursor, smartSelect, cursorUndo, bracketMatching, 그리고 coreCommands)에서 정확한 동작을 확인합니다.
U2. 포인터 입력: ⌥클릭으로 커서 추가(TS 의 multiCursorModifier 설정값 확인), ⇧⌥드래그 컬럼 선택, 더블클릭 단어 선택·트리플클릭 줄 선택, 드래그로 선택 확장 시 단어·줄 단위 유지를 Monaco 규칙대로 구현합니다. 현재 구현돼 있는 것은 그대로 두고 빠진 것만 채웁니다.
U3. 다중 커서의 입력 결합: 타이핑·붙여넣기(줄 수가 커서 수와 같을 때의 분배는 배치 1 구현 확인)·삭제·Enter·자동 닫기·IME 조합이 모든 커서에 올바르게 적용되고, 겹치는 선택이 병합되며, Escape 가 다중 커서를 해제하는지 확인하고 빠진 것을 고칩니다.
U4. 구현과 키: 앞 단계와 같은 방식으로 순수 함수 구현, 명령 레지스트리 연결, Monaco 기본 키(⌥⌘↑/↓, ⇧⌥I, ⌘D, ⌘K ⌘D, ⇧⌘L, ⌘L, ⌃⇧⌘→/←, ⌘U, ⇧⌘\\ 등 실제 등록을 Monaco 소스에서 확인)를 편집기 문맥 기본 키로 추가합니다.
비목표: 찾기 위젯과 연동되는 일치 강조, LSP selection range, 괄호 매칭 강조 장식.`,
    verify: `
명령마다 Monaco 동작에서 유도한 실패하는 순수 테스트부터 작성합니다. 포인터 입력은 UI 테스트로 검증합니다.
V1. cargo test --quiet --manifest-path ${NE}/Cargo.toml ${CARGO_TAIL}
V2. cargo test --quiet --manifest-path ${NU}/Cargo.toml --lib --test editor_surface --test keymap-platform ${CARGO_TAIL}
V3. cargo check --quiet --manifest-path ${NA}/Cargo.toml ${CARGO_TAIL}, cargo test --quiet --manifest-path ${NA}/Cargo.toml --lib ${CARGO_TAIL}(실패 0 유지)
V4. cargo check --quiet --manifest-path ${ROOT}/native/taide-remote-web/Cargo.toml ${CARGO_TAIL}
V5. 변경한 크레이트마다 cargo fmt --manifest-path <Cargo.toml> -- --check`,
  },
]

const implPrompt = (s, order) => `${BACKGROUND}${IMPL_RULES}
# 이번 작업 (배치 7, 단계 ${order}/3): ${s.title}

## 목표와 완료 조건
설계 문서 ${s.designRef} 를 구현하고 검증 계약의 모든 명령이 exit 0 이면 완료입니다. 완료하지 못한 항목은 이유와 함께 unresolved 에 적습니다.

## 수정 범위
${s.scope}

## 단계별 허용
${s.allowed}

## 작업 항목
${s.tasks}

## 실행 순서
1. git status 와 git diff --stat 으로 작업 트리 상태를 확인합니다. 이 단계 범위의 파일에 미커밋 변경이 이미 있다면(이 단계가 중단 후 재시작된 경우) 그 변경을 읽고 이어서 작업하며, 처음부터 다시 만들지 않습니다.
2. 설계 문서의 해당 절과 대상 native 파일, 설계가 인용한 TS·Shiki·Monaco 소스, 엔진 소스를 읽어 현재 동작과 기준 동작을 확정합니다.
3. 항목별로 테스트 → 구현 → 해당 테스트 실행 순으로 진행합니다.
4. 검증 계약의 나머지 명령을 실행합니다.
5. ${QA}/2026-10-06-native-batch7-${s.key}.md 를 Write 로 작성합니다: 항목별 변경 내용(파일·함수), TS·Shiki·Monaco·엔진 근거 경로, 설계 문서와 달라진 점과 이유, 실행한 명령과 실제 결과(통과 수·시간·exit code), 실패했다가 고친 내역, 남은 위험과 실기 확인이 필요한 것.
6. StructuredOutput 을 반환합니다. step 은 "${s.key}", qaDocPath 는 방금 쓴 절대 경로입니다. gatePassed 는 엔진 게이트 단계에서는 판정 결과, 다른 단계에서는 true 입니다.

## 검증 계약
${s.verify}
성공한 검사는 반복하지 않고, 실패 수정 뒤에는 영향받은 검사만 다시 실행합니다.

## 통합 계약
배치 7 의 세 단계는 같은 작업 트리에서 직렬로 실행됩니다(커밋은 메인이 배치 끝에 합니다). 이 단계가 끝날 때 작업 트리는 반드시 컴파일되는 상태여야 합니다. 앞 단계의 미커밋 변경을 되돌리지 않고 그 위에 작업합니다. 동시에 읽기 전용 리뷰어가 앞 단계 파일을 읽고 있을 수 있으나 파일을 수정하지는 않습니다. 설계 문서 자체는 수정하지 않습니다.`

const reviewPrompt = (s, impl) => `${BACKGROUND}
# 이번 작업: 배치 7 "${s.title}" 구현 리뷰 (읽기 전용)

파일을 일절 수정하지 않고 cargo·bun 도 실행하지 않습니다(다른 작업자가 같은 작업 트리에서 다음 단계를 구현 중일 수 있습니다). 파일 읽기·검색은 Read·Grep·Glob, 셸은 절대 경로를 쓴 한 줄 단순 명령(grep, rg, ls, wc, sed -n, cat, diff, git status, git diff, git log, git show)만 허용합니다. cd, 파이프, && 는 금지입니다. 홈 디렉터리나 저장소 밖을 탐색하지 않습니다(예외: ${REG} 와 ${ROOT}/node_modules 읽기). 다른 에이전트를 띄우지 않습니다.

## 리뷰 대상
- 설계 기준: ${DESIGN} 의 ${s.designRef}
- 구현자가 보고한 변경 파일: ${JSON.stringify(impl.changedFiles)}
- 구현자 요약: ${impl.summary}
- 게이트 판정: ${impl.gatePassed} / ${impl.gateDetail}
- 구현자가 보고한 미해결: ${JSON.stringify(impl.unresolved)}
- 구현 기록: ${impl.qaDocPath}
- 작업 지시(원문):
${s.tasks}

## 변경 확인 방법
배치 시작 기준 커밋은 ${BASE_COMMIT} 입니다. 파일마다 git diff ${BASE_COMMIT} -- <절대 경로> 로 변경을 확인하고 새 파일은 직접 읽습니다(문법 JSON 과 기준 자료 같은 대용량 데이터는 통째로 읽지 말고 구조와 표본만 확인). 작업 트리에는 앞 단계와 진행 중인 다음 단계의 미커밋 변경이 함께 들어 있을 수 있습니다. 구현자가 보고한 파일과 이 단계 범위("${s.scope}")에 해당하는 변경만 판정하고, 그 밖의 변경은 outOfScopeChanges 에만 적습니다.

## 판정 기준
blocking 은 다음에 해당하는 것만 올립니다. 각 항목에 구체적 실패 시나리오(입력·상태 → 잘못된 결과)를 적습니다.
1. 정확성 결함: 오프셋 단위 혼동(UTF-16 과 UTF-8 바이트), 줄 이동 반영 오류로 다른 줄에 토큰이 붙는 경로, 낡은 revision 의 토큰이 그려지거나 저장 정리에 쓰이는 경로, worker 스레드의 교착·누수·종료 시 패닉, 패닉 가능성(슬라이스 경계), 캐시 무효화 누락, 기존 동작 회귀. 저장 정리가 잘못된 토큰 정보로 사용자 파일의 내용을 바꿀 수 있는 경로는 반드시 blocking 입니다.
2. TS(Shiki·Monaco) 기준 동작과 다른 구현(근거 경로를 제시). 기준 자료를 엔진 출력에 맞춰 만들었거나, 적합성 테스트가 엔진 출력끼리 비교하는 순환 검증이면 blocking 입니다.
3. 작업 항목 누락 또는 테스트가 실제 동작을 검증하지 않는 경우. 게이트 단계에서는 게이트 판정이 실제 측정·대조 결과와 맞지 않는 경우.
4. 우회: #[allow] 로 lint 끄기, 오류 삼킴, 증상만 가리는 분기.
5. 허용 범위를 넘은 Cargo.toml·Cargo.lock 변경(기존 의존성 버전 변동, 승인되지 않은 새 크레이트), 동결된 taide-remote-web 의 기능·의존 그래프 변경, 수정된 문법 파일, 출처·라이선스 고지 없는 제3자 자산, 정책과 맞지 않는 라이선스의 문법 포함.
6. 프레임마다 전 문서를 다시 토큰화하거나 메인 스레드에서 토큰화하는 등 편집을 막는 구조.
스타일 취향, 사소한 이름, 범위 밖 개선 제안은 nonBlocking 에 적습니다. 확신이 없는 추정은 blocking 에 올리지 말고 nonBlocking 에 "확인 필요"로 적습니다.
StructuredOutput 의 step 은 "${s.key}" 입니다.`

const fixPrompt = (s, review) => `${BACKGROUND}${IMPL_RULES}
# 이번 작업: 배치 7 "${s.title}" 리뷰 차단 항목 수정

배치 7 의 구현 단계가 모두 끝난 뒤의 직렬 수정 단계입니다. 지금은 다른 작업자가 코드를 수정하지 않습니다. 작업 트리에는 앞선 단계들의 미커밋 변경이 모두 들어 있으므로, 뒤 단계가 같은 코드를 이미 바꿨을 수 있습니다. 현재 코드 기준으로 지적이 아직 유효한지 먼저 확인합니다.

## 수정 범위
${s.scope}

## 단계별 허용
${s.allowed}

## 리뷰어가 올린 차단 항목
${JSON.stringify(review.blocking, null, 2)}

## 실행 순서
1. 각 항목을 실제 코드와 TS·Shiki·Monaco·엔진 근거로 직접 검증합니다. 리뷰어의 지적이 틀렸거나 뒤 단계에서 이미 해소됐다면 수정하지 말고 근거(파일·줄)를 기록합니다.
2. 옳은 지적은 실패하는 테스트로 재현한 뒤 근본 원인을 고칩니다.
3. 영향받은 검증 명령만 다시 실행합니다. 검증 계약은 다음과 같습니다:
${s.verify}
4. ${QA}/2026-10-06-native-batch7-${s.key}.md 끝에 "리뷰 후속" 절을 Edit 로 추가합니다(항목별 수정 또는 반증, 실행한 명령과 실제 결과).
5. StructuredOutput 을 반환합니다. step 은 "${s.key}", completed 에는 항목별 "수정" 또는 "반증"을 적고 gatePassed 는 true 로 둡니다(게이트 단계의 수정이라면 수정 후 판정).`

const implOpts = (s) => ({ label: `impl:${s.key}`, phase: 'Implement', model: 'opus', effort: 'xhigh', schema: IMPL_SCHEMA })
const review = (s, impl) =>
  impl
    ? agent(reviewPrompt(s, impl), { label: `review:${s.key}`, phase: 'Review', model: 'sonnet', effort: 'xhigh', schema: REVIEW_SCHEMA })
    : Promise.resolve(null)

const RESEARCH_DOC = ROOT + '/docs/research/2026-10-07-native-find-regex-dialect.md'
const RESEARCH_SCHEMA = {
  type: 'object',
  properties: {
    docPath: { type: 'string' },
    summary: { type: 'string' },
    recommendation: { type: 'string' },
    candidates: { type: 'array', items: { type: 'string' } },
    openDecisions: { type: 'array', items: { type: 'string' } },
    unverified: { type: 'array', items: { type: 'string' } },
  },
  required: ['docPath', 'summary', 'recommendation', 'candidates', 'openDecisions', 'unverified'],
}
const researchPrompt = `${BACKGROUND}
# 이번 작업: 찾기/바꾸기 위젯의 정규식 방언 조사 (읽기 전용 + 문서 1개)

## 규칙
- 코드와 설정 파일을 수정하지 않습니다. 산출물은 ${RESEARCH_DOC} 하나를 Write 로 새로 만드는 것뿐입니다.
- 파일 읽기·검색은 Read·Grep·Glob, 셸은 절대 경로를 쓴 한 줄 단순 명령(grep, rg, ls, wc, sed -n, cat, git log, git show)만 허용합니다. cd, 파이프, &&, cargo, bun, npm, python 은 금지입니다. 다른 작업자가 같은 작업 트리에서 구현 중이므로 빌드·테스트를 실행하지 않습니다. 큰 파일은 필요한 구간만 읽습니다.
- 외부 라이브러리 정보는 기억에 의존하지 말고 공식 문서를 조회합니다. ToolSearch 로 context7 문서 조회 도구와 WebSearch, WebFetch 를 불러 사용하고, 버전·라이선스·MSRV·유지 상태는 crates.io·docs.rs·공식 저장소에서 확인해 출처 URL 을 남깁니다. 확인하지 못한 것은 "미확인"으로 적습니다. 다른 에이전트를 띄우지 않습니다.

## 배경
설계 문서 ${DESIGN} 7절 "단계 6. 찾기/바꾸기 위젯"과 8절 D6: Monaco 의 찾기는 JS 정규식이며, native 는 찾기 모델의 정규식 부분을 trait 로 빼서 앱이 구현을 넣는 방식을 권합니다. 앱은 regex 1.13.1 을 이미 쓰고, 구문 강조용으로 ferroni 1.8.1(Oniguruma 이식, 순수 Rust)이 승인·고정돼 있습니다. 워크스페이스 검색(${ROOT}/crates/taide-search)이 어떤 정규식 엔진과 방언을 쓰는지도 확인 대상입니다(TS 의 검색 패널과 찾기 위젯이 사용자에게 같은 방언으로 보이는지).

## 조사 항목
1. TS 기준: Monaco 찾기 모델이 정규식을 다루는 방식(${ROOT}/node_modules/monaco-editor 의 contrib/find 의 findModel·replacePattern, common/model/textModelSearch: 플래그, 유니코드, 여러 줄 일치, 단어 단위, 대소문자 보존 바꾸기, 바꾸기 패턴의 $1·$&·\\n·\\u 등 대소문자 변환 지시), 한도(일치 수 19999 등), TS 앱이 찾기 옵션을 바꾼 부분.
2. 후보 비교: regex 크레이트, fancy-regex, ferroni(Oniguruma), 그 밖에 JS 정규식(ECMAScript) 호환을 목표로 하는 Rust 크레이트를 공식 문서로 조사해, ECMAScript 문법과의 차이(후방 참조, 전후방 탐색, 이름 있는 그룹 문법, 유니코드 속성, \\d·\\w·\\b 의 유니코드 동작, 줄 끝 처리, sticky·global 의미), 성능과 최악 시간(백트래킹 폭주 방어), 대형 문서 검색 방식(rope 를 줄 단위로 넘길 수 있는지), 라이선스·MSRV·유지 상태·새 의존성 수를 표로 정리합니다.
3. 워크스페이스 검색과의 일관성: taide-search 의 엔진·방언과 TS 검색 패널이 사용자에게 안내하는 방언을 확인하고, 찾기 위젯과 방언을 통일할지에 대한 선택지를 정리합니다.
4. 추천안 1개와 근거, 새 의존성 목록(크레이트명, 확인한 최신 버전, 라이선스, 왜 필요한지), JS 와 달라지는 문법을 사용자에게 어떻게 다룰지(변환 계층 또는 차이 고지), 구현 시 테스트 전략(Monaco 테스트 사례 이식).
5. 사용자 결정이 필요한 사항과 확인하지 못한 사항.

${RESEARCH_DOC} 를 작성하고 StructuredOutput 을 반환합니다(docPath 는 그 절대 경로).`

const implementation = async () => {
  const impls = [null, null, null]
  const reviews = [null, null, null]
  impls[0] = await agent(implPrompt(STEPS[0], 1), implOpts(STEPS[0]))
  log(`단계 1 구현 ${impls[0] ? '완료' : '결과 없음'}`)
  const second = await parallel([() => review(STEPS[0], impls[0]), () => agent(implPrompt(STEPS[1], 2), implOpts(STEPS[1]))])
  reviews[0] = second[0]
  impls[1] = second[1]
  log(`단계 2 구현 ${impls[1] ? '완료' : '결과 없음'}`)
  const third = await parallel([() => review(STEPS[1], impls[1]), () => agent(implPrompt(STEPS[2], 3), implOpts(STEPS[2]))])
  reviews[1] = third[0]
  impls[2] = third[1]
  log(`단계 3 구현 ${impls[2] ? '완료' : '결과 없음'}`)
  reviews[2] = await review(STEPS[2], impls[2])
  const fixes = []
  for (let i = 0; i < STEPS.length; i++) {
    const r = reviews[i]
    if (r && r.blocking.length > 0) {
      log(`${STEPS[i].key}: 차단 항목 ${r.blocking.length}건 수정 시작`)
      fixes.push(await agent(fixPrompt(STEPS[i], r), { label: `fix:${STEPS[i].key}`, phase: 'Fix', model: 'opus', effort: 'xhigh', schema: IMPL_SCHEMA }))
    } else {
      fixes.push(null)
    }
  }
  return STEPS.map((s, i) => ({ step: s.key, impl: impls[i], review: reviews[i], fix: fixes[i] }))
}

const [research, steps] = await parallel([
  () => agent(researchPrompt, { label: 'research:find-regex-dialect', phase: 'Research', model: 'opus', effort: 'high', schema: RESEARCH_SCHEMA }),
  implementation,
])

return { research, steps }
