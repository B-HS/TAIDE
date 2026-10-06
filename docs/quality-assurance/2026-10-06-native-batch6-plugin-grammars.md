# Native 배치 6 단계 1/3 — 플러그인·VSIX 문법 로딩 (2026-10-07)

상태: L1~L4 구현 완료. 검증 계약 V1~V4 는 모두 exit 0 입니다. GUI 는 실행하지 않았습니다. `Cargo.toml`·`Cargo.lock` 은 건드리지 않았습니다.

기준 시점은 HEAD `62f698fe` 이고 시작할 때 작업 트리의 변경은 `docs/PROCESS.md` 하나였습니다(이 단계가 만든 변경이 아닙니다). 경로는 저장소 루트 기준입니다. 약어: `SHIKI-PRIM` 은 `node_modules/@shikijs/primitive/dist/index.mjs`(4.4.3), `SHIKI-MONACO` 는 `node_modules/@shikijs/monaco/dist/index.mjs`(4.4.3), `VSCODE-TM` 은 `node_modules/@shikijs/vscode-textmate/dist/index.js`(10.0.2), `ENGINE` 은 `/Users/hyunseokbyun/development/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ferriki-textmate-0.12.0/src` 입니다.

## 0. 먼저 알아야 할 것

1. 화면 변화: 플러그인(또는 VSIX 가져오기)이 문법을 제공하는 언어의 문서가 처음으로 색이 입혀집니다. 번들 언어와 id 또는 scope 가 겹치는 플러그인 문법이 있으면 그 번들 언어의 색도 달라질 수 있습니다(2절 표).
2. 사용자 파일: `trimTrailingWhitespaceOnSave` 가 켜져 있으면 플러그인 문법 언어의 파일도 저장 때 후행 공백이 지워집니다(문자열·정규식 안은 보존). 지금까지는 지워지지 않았습니다. 문법이 엔진에서 거절되거나 정규식이 컴파일되지 않는 플러그인 언어는 문자열을 구분하지 못하므로 모든 줄에서 지워집니다(TS 도 같습니다. 8절 1번).
3. include 만으로 자기 자신에게 돌아오는 플러그인 문법은 엔진에 싣지 않습니다. 엔진이 이런 문법에서 끝없이 재귀해 프로세스가 중단될 수 있기 때문입니다(3절 L3, 4절 2번). 이 판단은 엔진 소스를 읽어 내린 것이고 실제로 중단시켜 확인하지는 않았습니다.
4. 메인 판단이 필요한 사항 6건이 8절에 있습니다.

## 1. 한눈에 보는 결과

| 항목 | 결과 |
| --- | --- |
| L1 TS 동작 확인 | 발견·검증·등록·충돌·임베드·재로드·실패 규칙을 TS·Shiki 소스로 확정(2절) |
| L2 앱 쪽 공급 | 조율자가 `plugin_list`·`plugin_read_grammar` 를 blocking 작업으로 불러 문법을 읽고 토큰화 스레드에 넘김. 목록이 바뀌면 다시 읽음 |
| L3 구문 크레이트 | 플러그인 등록 자료형, Shiki 의 언어 등록 순서 이식, include 순환 검사, 번들 + 플러그인 레지스트리 구성, 파이프라인·worker 연결 |
| L4 저장 정리 | 플러그인 문법 언어 문서가 번들 언어와 같은 경로로 토큰 정보를 공급 |
| 테스트 | 구문 크레이트 63 → 89, 앱 lib 365 → 367, `save-syntax` 3 → 4 |

## 2. L1 — TS 기준 동작

| 질문 | 확인한 사실 | 근거 |
| --- | --- | --- |
| 매니페스트 필드 | 언어 기여는 `id`, `extensions`, `aliases`, `grammar`(플러그인 루트 기준 상대 경로), `embeddedLanguages` 다섯 개입니다. 파일명으로 언어를 정하는 필드와 `injectTo` 는 없습니다. `scopeName` 은 매니페스트가 아니라 문법 JSON 안에서 읽습니다 | `crates/taide-model/src/plugin.rs:8-19` |
| 파일 → 언어 id | 편집기는 백엔드가 준 `OpenedFile.languageId` 를 그대로 씁니다. 그 값은 활성 플러그인의 `extensions` 를 먼저 보고(플러그인 디렉터리 이름순, 기여 순서대로 첫 일치) 그다음 번들 확장자 표를 봅니다. native 의 파일 열기 경로도 같은 함수를 이미 씁니다 | `crates/taide-infra/src/language.rs:83-115`, `crates/taide-plugin/src/service.rs:52-62`, `src/entities/editor/model-registry.ts:39-56`, `native/taide-native-app/src/host.rs:1361-1372` |
| 이미 열린 문서 | 플러그인 설치·재로드가 열린 문서의 언어 id 를 바꾸지 않습니다. 언어 id 는 파일을 다시 열거나 이름을 바꿀 때만 다시 정해집니다 | `src/entities/layout/tab-path-change.ts:272`, `src/entities/editor/model-registry.ts:127-132` |
| 발견 | 활성(`enabled`) 플러그인의 언어 기여 중 `grammar` 가 비어 있지 않은 것만 대상입니다. 순서는 플러그인 목록 순서, 그 안에서 기여 순서입니다 | `src/entities/plugin/plugin-grammar.ts:32-36` |
| 읽기 실패 | 문법 본문을 읽지 못하면 그 문법만 버립니다 | `plugin-grammar.ts:38-49` |
| JSON·scopeName 검증 | `JSON.parse` 가 실패하거나, 결과가 객체가 아니거나, `scopeName` 이 길이 1 이상의 문자열이 아니면 그 문법만 버립니다 | `plugin-grammar.ts:5-17` |
| 등록 내용 | 문법 JSON 의 최상위 키를 그대로 두고 `name` 을 언어 id 로 바꾸며, `patterns` 가 배열이 아니면 빈 배열, `repository` 가 객체(배열 포함)가 아니면 빈 객체로 둡니다. `embeddedLangs` 는 매니페스트의 `embeddedLanguages` 입니다 | `plugin-grammar.ts:19-29` |
| 호스트 쪽 사전 검증 | 문법 파일이 5MB 를 넘거나 JSON 이 아니거나 `scopeName` 이 비면 플러그인 전체가 비활성이 됩니다. plist 문법은 여기서 걸립니다(TS 도 plist 를 지원하지 않음) | `crates/taide-plugin/src/service.rs:177-216`, `crates/taide-model/src/plugin.rs:6` |
| 플러그인끼리의 충돌 | 언어 id 나 scopeName 이 앞선 플러그인과 겹치면 뒤 기여의 `grammar` 를 없앱니다(먼저 온 플러그인이 이김) | `crates/taide-plugin/src/service.rs:95-130` |
| 번들 언어와 id 충돌 | Shiki 는 이름이 같은 등록 중 나중 것만 싣습니다. 플러그인 문법이 번들 뒤에 오므로 그 언어 id 의 문서는 플러그인 문법으로 토큰화됩니다. 번들 문법의 scope 는 다른 문법의 include 용으로 남습니다 | `src/shared/lib/shiki/shiki-monaco.ts:144-153`, SHIKI-PRIM 305-328, 337-343, 353-363 |
| 번들 언어와 scope 만 충돌 | vscode-textmate 는 scope 마다 처음 컴파일한 문법을 계속 씁니다. 그래서 두 언어 id 의 문서는 모두 번들 문법으로 토큰화되고, 다른 문법이 그 scope 를 include 할 때만 나중에 등록된 플러그인 문법이 쓰입니다 | SHIKI-PRIM 273-296, VSCODE-TM 2415-2429, 3049-3060, 3082-3100, 3174-3186 |
| id 와 scope 가 모두 충돌 | 플러그인 문법이 번들 문법을 완전히 대신합니다(문서와 include 모두) | 위 두 행의 조합 |
| Shiki 별칭 | 번들 문법에는 Shiki 별칭이 있습니다(`shellscript` 의 `bash`·`sh`·`shell`·`zsh`, `rust` 의 `rs` 등 13개 문법). 이미 실린 문법의 별칭과 id 가 같은 플러그인 문법은 실리지 않고 그 id 는 별칭의 번들 문법을 가리킵니다 | SHIKI-PRIM 210-221, 269-274, 285-287, `native/taide-native-syntax/grammars/manifest.json` |
| 임베드 언어 | `embeddedLanguages` 의 이름 중 TAIDE 31개 언어 id 는 "요청된 언어 집합"에 더합니다(줄어들지 않음). 실리지 않은 이름은 경고만 남기고 버립니다. 그 밖의 효과는 없습니다 | `shiki-monaco.ts:43, 120-142, 161-166` |
| 재로드 | 부팅 때 한 번, 그리고 데스크톱 UI 의 재로드·설치·제거·VSIX 가져오기 뒤마다 문법을 전부 다시 읽고 하이라이터를 새로 만듭니다 | `src/app/providers/theme-provider.tsx:89-96`, `src/entities/plugin/plugin.query.ts:19-45`, `src/entities/vsix/vsix.query.ts:9`, `shiki-monaco.ts:302-309` |
| 하이라이터 생성 실패 | 예외가 나면 하이라이터 없이 남고 모든 언어가 평문이 됩니다 | `shiki-monaco.ts:161-171, 302-309` |
| 줄 토큰화 실패 | Monaco 가 예외를 잡아 그 줄을 기본 토큰으로 두고 상태를 그대로 넘깁니다 | 배치 5 QA(`2026-10-06-native-batch5-token-pipeline.md` T5) |
| VSIX 가져오기 | 확장의 `contributes.grammars` 중 `language` 가 있는 것만 `grammars/<언어 id>.tmLanguage.json` 으로 옮기고 `taide-plugin.json` 을 만들어 일반 플러그인으로 설치합니다. `language` 가 없는 주입 문법은 건너뜁니다. 이후 경로는 일반 플러그인과 같습니다 | `crates/taide-vsix/src/service.rs:526-599`, `crates/taide-runtime/src/plugin_actions.rs:70-87` |

## 3. 항목별 변경 내용

### L2. 앱 쪽 공급 (`native/taide-native-app`)

| 파일 | 내용 |
| --- | --- |
| `src/editor-syntax.rs` | 비공개 `plugin_grammars(state, store)`, 공개 `EditorSyntax::follow_plugins(state, store, tasks)`, 비공개 `install_plugin_grammars`·`with_wake`. 필드 `wake`, `followed_plugins`, `plugin_grammar_read`. 추적 조건을 `is_bundled_language` 에서 `TokenPipeline::accepts_language` 로 바꿈. `new` 는 테스트 전용 |
| `src/application.rs` | `background_tick` 에서 `tick` 직전에 `follow_plugins` 호출 1곳(+5줄) |
| `src/editor-syntax-tests.rs` | 신규 2건과 도우미 |
| `tests/save-syntax.rs` | 신규 1건(L4)과 도우미 |

- 읽기: `plugin_actions::plugin_list` 로 목록을 받고, 활성 플러그인의 `grammar` 가 있는 언어마다 `plugin_actions::plugin_read_grammar` 로 본문을 읽어 `PluginGrammar::from_contribution` 으로 등록을 만듭니다. 읽기나 검증에 실패한 문법은 건너뜁니다(`plugin-grammar.ts:32-50` 과 같은 순서와 규칙).
- 메인 스레드: 읽기는 `TaskSupervisor::spawn_transient` 안에서 `run_blocking_result` 로 blocking 풀에서 돕니다(`native/taide-native-app/src/terminal_fonts.rs:94-108`, `app-file.rs:137-154` 와 같은 방식). 끝나면 조율자의 `wake`(앱에서는 `request_repaint`)를 부르고, 다음 `follow_plugins` 가 결과를 반영합니다. 메인 스레드에서 하는 일은 플러그인 저장소의 읽기 잠금 한 번과 목록 비교뿐입니다.
- 언제 읽는가: 조율자가 만들어진 뒤 첫 tick 에 한 번(TS 의 부팅 시 읽기), 이후에는 플러그인 저장소의 목록이 조율자가 마지막으로 읽은 목록과 다를 때입니다. 원격 클라이언트의 `plugin_reload`, 설치·제거·VSIX 가져오기가 저장소를 바꾸면 다음 프레임에 감지됩니다.
- 반영: 받은 문법 집합을 `TokenPipeline::set_plugin_grammars` 에 넘기고, 더는 받아들여지지 않는 언어의 문서를 추적에서 뺍니다. 새로 받아들여지는 언어의 문서는 같은 프레임의 `tick` 이 추적을 시작합니다.
- 실패: 읽기 작업이 실패하면 경고 로그만 남기고 이전 문법 집합을 유지합니다. 같은 목록에 대해서는 다시 시도하지 않습니다(프레임마다 재시도하지 않기 위함).

### L3. 구문 크레이트 (`native/taide-native-syntax`)

| 파일 | 내용 |
| --- | --- |
| `src/plugin-grammars.rs` (신규) | `PluginGrammar`, `PluginGrammar::from_contribution(언어 id, 임베드 언어, 문법 JSON)`, `language_id`·`scope_name`·`embedded_languages`, 크레이트 내부 `registration`·`raw_grammar` |
| `src/grammar-registrations.rs` (신규) | `LanguageRegistration`, `LoadedGrammars`, `loaded_grammars`: Shiki 의 언어 등록 순서 이식 |
| `src/include-cycles.rs` (신규) | `IncludeSource`, `plugins_on_include_only_cycles`: include 만으로 이어진 순환에 든 플러그인 문법 찾기 |
| `src/bundled-grammars.rs` | 매니페스트의 `aliases` 읽기, `BundledRegistration`, `bundled_registrations`(TS 가 문법을 싣는 순서) |
| `src/textmate-tokenizer.rs` | `TextmateTokenizer::with_plugin_grammars(요청 언어, 플러그인 문법, 테마, 한도)`. 기존 `new` 는 서명·동작 그대로이고 공통 부분을 `with_registry` 로 뺌 |
| `src/token-worker.rs` | `WorkerRequest::SetPluginGrammars(Vec<PluginGrammar>)`, worker 가 문법 집합을 보관하고 `Configure` 때마다 사용 |
| `src/token-pipeline.rs` | `TokenPipeline::set_plugin_grammars`, `accepts_language` |
| `src/lib.rs` | 모듈 연결, `PluginGrammar` 공개 |
| `NOTICE.md`, 루트 `THIRD_PARTY_LICENSES.md` | 이식 고지 추가(범위 밖 파일, 5절 6번) |

- 등록(`from_contribution`): 2절의 "JSON·scopeName 검증"과 "등록 내용"을 그대로 옮겼습니다. 배열 `repository` 는 인덱스를 키로 한 객체로 바꿉니다(JS 에서 배열을 이름으로 조회할 때와 같은 결과). 엔진이 형만 검사하고 토큰화에 쓰지 않는 `fileTypes`·`firstLineMatch` 는 뺍니다(TS 는 이 값을 읽지 않으므로 형이 틀려도 문법이 실립니다. `ENGINE/raw_grammar.rs:41-46`).
- 등록 순서(`loaded_grammars`): 번들 등록(TS 의 `loadTaideGrammars` 순서) 뒤에 플러그인 등록을 놓고 Shiki 의 `Resolver` 생성자, `Registry.loadLanguages`, `Registry.loadLanguage` 를 순서대로 따라가 세 가지를 냅니다. 문서용 문법(scope 마다 처음 실린 등록), include 용 문법(scope 마다 마지막에 실린 등록), 언어 id → scope 표(실린 이름과 별칭).
- 레지스트리 구성(`with_plugin_grammars`): include 용 문법을 전부 엔진 레지스트리에 넣습니다. 문서용 문법이 include 용과 다른 scope(2절 "scope 만 충돌")는 문서용 문법을 잠시 넣어 컴파일해 따로 보관한 뒤 include 용으로 되돌립니다.
- 잘못된 문법의 격리:

| 경우 | TS | native |
| --- | --- | --- |
| JSON 오류, 객체 아님, `scopeName` 없음 | 그 문법만 버림 | `from_contribution` 이 `None`. 그 언어는 받아들여지지 않아 평문 |
| 정규식 컴파일 오류 | 그 규칙에 닿는 줄이 예외 → 기본 토큰 | 엔진이 오류를 돌려주고 worker 가 그 줄을 기본 스타일 한 토큰으로 둠(배치 5 의 기존 경로) |
| include 만으로 순환 | 줄마다 `RangeError` → 기본 토큰 | 엔진에 싣지 않음. 그 언어의 줄은 기본 스타일 한 토큰 |
| 엔진이 읽지 못하는 구조(`patterns` 에 객체가 아닌 값 등) | 하이라이터 생성이 실패하면 모든 언어가 평문(2절) | 그 문법만 싣지 않음 |

- include 순환 검사: 엔진은 include 만 가진 규칙(match·begin 이 없는 규칙)의 패턴을 모을 때 방문 표시 없이 재귀합니다(`ENGINE/rule.rs:506-520, 808-818`, 규칙 id 는 `ENGINE/rule_factory.rs:193-210` 에서 재귀 전에 예약되므로 구조 컴파일은 끝납니다). 이런 규칙끼리 고리를 이루면 스택이 넘치고, Rust 의 스택 넘침은 잡을 수 없어 프로세스가 중단됩니다. 그래서 싣기 전에 문법 JSON 에서 "include 만 가진 규칙 → 그 규칙이 모으는 include 만 가진 규칙" 그래프를 만들고(번들 문법 포함, `$self`·`$base`·`#이름`·`scope`·`scope#이름` 을 `ENGINE/include_reference.rs:22-41`·`rule_factory.rs:399-441` 의 규칙으로 해석), 강연결 요소 중 고리가 있는 것에 든 플러그인 문법을 전부 뺍니다. begin 이나 match 를 가진 규칙을 거치는 재귀는 고리로 보지 않습니다(정상 문법의 재귀는 모두 이 형태입니다). `$base` 와 규칙 안 `repository` 는 가능한 대상을 모두 따라가는 쪽으로 넓게 봅니다.
- 요청 언어 집합: `set_plugin_grammars` 가 임베드 이름 중 번들 언어 id 를 요청 집합에 더합니다. 플러그인이 사라져도 집합은 줄지 않습니다.
- 재구성: 문법 집합이 이전과 다르면 worker 에 `SetPluginGrammars` 를 보낸 뒤 새 세대의 `Configure` 를 보냅니다. 설정 응답을 받으면 모든 문서의 토큰을 비우고 다시 토큰화합니다(테마 변경과 같은 경로).
- 토큰화 대상: 파이프라인은 번들 언어와, 받은 문법 집합에 든 언어 id 의 문서를 토큰화합니다. worker 가 싣지 않은 문법의 언어는 줄마다 기본 스타일 한 토큰이 됩니다.

### L4. 저장 정리

코드 변경은 추적 조건 하나입니다. 플러그인 문법 언어 문서가 추적되면 `supply_save_cleanup` 이 번들 언어와 같은 스냅샷을 만듭니다. 통합 테스트 `플러그인_문법_언어의_저장_정리도_문자열_안_후행_공백을_남기고_주석_뒤는_지운다` 가 실제 플러그인 디렉터리·읽기 작업·엔진·worker·`save::prepare` 로 확인합니다.

## 4. 근거 경로

TS: `src/entities/plugin/plugin-grammar.ts:5-50`, `src/entities/plugin/plugin.query.ts:19-45`, `src/entities/vsix/vsix.query.ts:9`, `src/shared/lib/shiki/shiki-monaco.ts:43, 120-171, 187-196, 227-231, 302-309`, `src/shared/lib/shiki/lang-map.ts:56-110`, `src/shared/lib/monaco/register-plugin-languages.ts:23-37`, `src/app/providers/theme-provider.tsx:89-96`, `src/entities/editor/model-registry.ts:39-56, 127-132`.

Rust 공유 크레이트: `crates/taide-model/src/plugin.rs:4-19, 78-86`, `crates/taide-plugin/src/service.rs:15-62, 95-130, 177-247`, `crates/taide-runtime/src/plugin_actions.rs:10-21, 64-87`, `crates/taide-infra/src/language.rs:83-115`, `crates/taide-vsix/src/service.rs:526-599`, `native/taide-native-app/src/remote-plugins.rs:28-47`.

Shiki 4.4.3: SHIKI-PRIM 202-204(`resolveLangs`), 210-221(`resolveLangAlias`), 224-329(`Registry`), 332-373(`Resolver`), `node_modules/@shikijs/langs/dist/{html,cpp,cpp-macro,glsl,ruby,graphql}.mjs` 의 기본 내보내기 배열, SHIKI-MONACO 93-128.

vscode-textmate 10.0.2: VSCODE-TM 2293-2321(`Grammar` 생성자), 2415-2429(`getExternalGrammar`), 3026-3101(`SyncRegistry`), 3104-3206(`Registry`).

엔진: `ENGINE/registry.rs:65-83, 95-117`, `ENGINE/grammar.rs:150-210`, `ENGINE/rule_factory.rs:72-87, 193-254, 366-441`, `ENGINE/rule.rs:506-520, 808-836`, `ENGINE/raw_grammar.rs:28-53, 195-247`, `ENGINE/include_reference.rs:22-41`, `ENGINE/lib.rs:36-76`.

두 가지는 실행이 아니라 소스 읽기로 내린 판단입니다.

1. 엔진이 include 만으로 된 순환에서 끝없이 재귀한다는 것. 구현을 끄고 재현하는 것은 이 단계에서 금지돼 있고, 재현하면 테스트 프로세스가 중단됩니다.
2. 2절의 Shiki 충돌 규칙. TS 스택을 실행해 대조하지 않았습니다(이 단계의 허용 명령에 `bun` 이 없습니다). `loaded_grammars` 는 위 줄들을 순서대로 옮긴 것이고, 단위 테스트의 기대값은 그 소스를 손으로 따라가 정했습니다.

## 5. 설계 문서·지시와 달라진 점

| 번호 | 설계·지시 | 실제 | 이유 |
| --- | --- | --- | --- |
| 1 | 설계 5.1: 충돌은 "먼저 온 플러그인이 이긴다" | 그 규칙은 플러그인끼리만 해당합니다(호스트가 처리). 번들 언어와의 충돌은 Shiki 의 등록 순서로 정해지며 2절 표와 같습니다 | 설계가 번들과의 충돌을 다루지 않아 L1 에서 Shiki 소스로 확정 |
| 2 | 설계 5.6: 플러그인 문법도 worker 설정에 포함 | `WorkerConfiguration` 은 그대로 두고 `WorkerRequest::SetPluginGrammars` 를 따로 보냅니다 | 기존 테스트가 `WorkerConfiguration { .. }` literal 과 `WorkerResponse::Configured { generation, result }` 패턴을 씁니다. 필드를 더하면 기존 테스트를 고쳐야 합니다 |
| 3 | 작업 항목 L3: 플러그인 언어의 파일 연결을 번들 매핑과 합침 | 구문 크레이트에 표를 만들지 않았습니다 | 파일 → 언어 id 는 공유 크레이트 `taide_infra::language::language_id_for_path` 가 이미 플러그인 우선으로 정하고 native 의 모든 파일 열기 경로가 그 값을 씁니다(2절 2행). 배치 5 의 같은 결정(`2026-10-06-native-batch5-token-pipeline.md` T4)을 따랐습니다. 매니페스트에 파일명 필드는 없습니다 |
| 4 | 설계에 없음 | include 순환 검사(3절 L3) | 작업 항목 L3 의 "순환 include 가 토큰화 스레드를 죽이지 않아야 한다". TS 는 예외로 끝나지만 Rust 는 프로세스가 중단됩니다 |
| 5 | 설계 5.6: 재로드 시 레지스트리를 다시 만듦 | 문법 집합이 이전과 같으면 다시 만들지 않습니다 | 같은 집합으로 다시 만들어도 결과가 같습니다. 목록이 같고 문법 파일 내용만 바뀐 재로드는 8절 2번 |
| 6 | 수정 범위는 구문 크레이트의 `src`·`tests` 와 앱의 세 파일 | `native/taide-native-syntax/NOTICE.md` 와 루트 `THIRD_PARTY_LICENSES.md` 에 이식 고지를 더했습니다 | "옮긴 Shiki 소스는 출처와 라이선스 고지를 함께 추가"하는 규칙. prettier 검사는 실행하지 못했습니다(허용 명령에 없음) |
| 7 | 검증 계약: 실패하는 테스트를 먼저 작성 | 구문 크레이트는 구현 전 실패를 확인했습니다(6절 2·3번). 앱 쪽 테스트 3건은 앱 구현을 쓴 뒤에 작성했고 실패를 본 적이 없습니다 | 순서를 지키지 못했습니다. 구현을 되돌려 재현하는 것은 금지돼 있습니다 |

설계 문서가 인용한 줄 번호(`plugin-grammar.ts:10-30`, `shiki-monaco.ts:120-171, 302-309`, `register-plugin-languages.ts:23-37`, `plugin.query.ts:21-22`, `service.rs:95-130, 199-247`, `crates/taide-vsix/src/service.rs:554-555`, `remote-plugins.rs:30-40`)는 실제 파일과 일치했습니다.

## 6. 실행한 명령과 실제 결과

cargo 명령은 모두 `--locked --offline --target-dir experiments/native-shell-spike/target` 을 붙였습니다(`cargo fmt` 제외). 경로는 저장소 루트 기준으로 줄였습니다.

| 순서 | 명령 | 결과 |
| --- | --- | --- |
| 1 | `git status --short`, `git diff --stat` (시작) | `docs/PROCESS.md` 만 변경돼 있음 |
| 2 | `cargo test --manifest-path native/taide-native-syntax/Cargo.toml --lib --test plugin-grammars --test token-pipeline` (등록·순환·순서 모듈과 토크나이저까지 쓴 뒤, 파이프라인·worker 구현 전) | exit 101. 신규 테스트 파일의 raw 문자열 오류 2건(`r#"…"#` 안에 `"#r"`). 테스트 파일의 실수 |
| 3 | 같은 명령(문자열 고친 뒤) | exit 101. `TokenPipeline::set_plugin_grammars`·`accepts_language` 가 없어 E0599 13건(파이프라인 신규 테스트의 구현 전 실패) |
| 4 | 같은 명령(파이프라인·worker 구현 뒤) | exit 0. lib 47, plugin-grammars 10, token-pipeline 21. 논리 실패 없이 첫 실행에 통과 |
| 5 | `cargo check --manifest-path native/taide-native-app/Cargo.toml` | exit 0. 경고는 기존 `vendor/wry-preview` 17건뿐 |
| 6 | `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib` | exit 0. 367 통과(기존 365 + 신규 2), 5.89초 |
| 7 | `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --test save-syntax` | exit 0. 4 통과(기존 3 + 신규 1), 3.99초 |
| 8 | `cargo fmt --manifest-path native/taide-native-syntax/Cargo.toml -- --check` | exit 1. 신규 테스트 파일 4개의 줄 나눔 |
| 9 | `cargo fmt --manifest-path native/taide-native-syntax/Cargo.toml` | 그 4개 파일만 바뀜 |
| 10 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` | exit 1. `editor-syntax.rs`, `editor-syntax-tests.rs`, `tests/save-syntax.rs` 의 줄 나눔 |
| 11 | `cargo fmt --manifest-path native/taide-native-app/Cargo.toml` | 그 3개 파일만 바뀜 |
| 12 | `cargo clippy --manifest-path native/taide-native-syntax/Cargo.toml --all-targets` | exit 0. 경고 0 |
| 13 | `cargo clippy --quiet --manifest-path native/taide-native-app/Cargo.toml --lib --test save-syntax` | exit 0. 앱 크레이트 경고 0(기존 vendor 경고 17건) |
| 14 | V1 `cargo test --quiet --manifest-path native/taide-native-syntax/Cargo.toml` (번들 문법 중복 파싱을 없앤 최종 코드) | exit 0. lib 48, engine-gate 8 통과·3 ignored(17.64초), plugin-grammars 10, token-pipeline 21(2.74초), token-theme 2. 합계 89 통과 |
| 15 | V4 `cargo fmt --manifest-path native/taide-native-syntax/Cargo.toml -- --check` | exit 0 |
| 16 | 앱 테스트 도우미를 VSIX 가져오기와 같은 디렉터리 모양으로 바꾼 뒤 `cargo fmt … taide-native-app … -- --check` → `cargo fmt` | exit 1 → 테스트 파일 1개만 바뀜 |
| 17 | V2 `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --lib` (최종) | exit 0. 367 통과, 5.89초 |
| 18 | V2 `cargo test --quiet --manifest-path native/taide-native-app/Cargo.toml --test save-syntax` (최종) | exit 0. 4 통과, 3.99초 |
| 19 | V2 `cargo check --quiet --manifest-path native/taide-native-app/Cargo.toml` (최종) | exit 0 |
| 20 | V3 `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0 |
| 21 | V4 `cargo fmt --manifest-path native/taide-native-app/Cargo.toml -- --check` (최종) | exit 0 |
| 22 | `cargo clippy --manifest-path native/taide-native-syntax/Cargo.toml --all-targets` (최종) | exit 0. 경고 0 |
| 23 | `git status --short`, `git diff --numstat` (끝) | 10절 목록과 같음. `Cargo.toml`·`Cargo.lock` 변경 없음 |

6번과 17번 사이에 앱 크레이트에서 바뀐 것은 서식과 테스트 도우미뿐이고, 구문 크레이트에서는 번들 문법을 scope 마다 한 번만 파싱하도록 고쳤습니다.

실패했다가 고친 내역:

1. 신규 테스트의 raw 문자열 구분자 오류(2번). `r##"…"##` 로 고쳤습니다.
2. `cargo fmt -- --check` 가 신규·변경 파일의 줄 나눔을 지적해 `cargo fmt` 로 맞췄습니다(8~11번, 16번). 두 크레이트 모두 그 전에는 검사를 통과하던 상태라 이 단계가 쓴 파일만 바뀌었습니다.

구현이 테스트 기대값과 어긋나서 고친 것은 없습니다. 테스트 기대값을 구현에 맞춰 바꾼 것도 없습니다.

## 7. 신규 테스트

- `native/taide-native-syntax/src/plugin-grammars-tests.rs` (3건): 버려지는 입력 10종(빈 문자열, 깨진 JSON, 배열, null, 문자열, `scopeName` 없음·빈 문자열·숫자·null, BOM), 등록 내용의 정규화, 등록의 동등 비교
- `native/taide-native-syntax/src/grammar-registrations-tests.rs` (5건): id 충돌, scope 충돌, 둘 다 충돌, 같은 이름이 처음 등록된 자리에서 실리는 순서(html·heex), 별칭
- `native/taide-native-syntax/src/include-cycles-tests.rs` (4건): 순환 10종(`$self`, `$base`, 두 규칙, 배열 형태 repository 와 include 축약, 이름 없는 중첩 규칙, `scope#이름`, capture 안, begin/end 안, 규칙 안 repository, injections), 순환이 아닌 7종, 여러 플러그인에 걸친 순환과 구경하는 문법, 번들 문법을 거치는 순환
- `native/taide-native-syntax/src/bundled-grammars-tests.rs` (1건 추가, 기존 1건에 단언 1개 추가): 등록 순서가 `@shikijs/langs` 의 배열 순서와 같은지
- `native/taide-native-syntax/tests/plugin-grammars.rs` (신규 대상, 10건): 실제 엔진으로 플러그인 문법 토큰화, 임베드한 번들 언어, id·scope·둘 다 충돌, 별칭, 정상 재귀 문법, 정규식 오류, 순환·읽지 못하는 구조, 닫히지 않는 include 고리
- `native/taide-native-syntax/tests/token-pipeline/plugins.rs` (3건): 문법 집합의 등록·변경·제거에 따른 재토큰화, 임베드 언어의 요청 집합 반영, 잘못된 문법 3종이 worker 와 다른 언어를 건드리지 않음
- `native/taide-native-app/src/editor-syntax-tests.rs` (2건): 실제 플러그인 디렉터리(VSIX 가져오기가 만드는 모양)에서 읽기 → 토큰화 → 디스크만 바뀐 상태 → 재로드 → 제거, 읽지 못하거나 잘못된 문법 4종과 정상 문법·번들 언어의 공존
- `native/taide-native-app/tests/save-syntax.rs` (1건): L4

## 8. 메인 판단이 필요한 사항

1. 문법이 없는 플러그인 언어의 저장 정리. TS 는 토큰 공급자가 없는 언어의 모든 줄에서 후행 공백을 지웁니다(배치 5 QA `2026-10-06-native-batch5-surface-highlight.md` 2절). native 는 문법이 없거나 `from_contribution` 이 버린 플러그인 언어의 파일에서는 지우지 않습니다. 사용자 파일을 바꾸는 동작이고 작업 항목 L4 의 문구("번들 언어와 같은 규칙")를 넘어서 구현하지 않았습니다. 반대로 `from_contribution` 은 통과했지만 worker 가 싣지 않았거나 정규식이 컴파일되지 않는 문법의 언어는, 줄이 기본 토큰으로 "정확히" 토큰화된 것으로 남으므로 TS 와 같이 모든 줄을 지우게 됩니다. 이 두 번째 경우는 코드 경로로 판단한 것이고 저장 정리까지 이어서 테스트하지는 않았습니다(토큰이 기본 스타일 한 개로 채워지는 것까지는 `tests/token-pipeline/plugins.rs` 와 `editor-syntax-tests.rs` 가 확인합니다).
2. 목록이 같고 문법 파일 내용만 바뀐 재로드. 조율자는 플러그인 저장소의 목록 내용이 달라졌을 때만 다시 읽습니다. 저장소(`crates/taide-plugin` 의 `PluginStore`)에 변경 횟수가 없어 "재로드가 있었다"는 사실 자체는 알 수 없습니다. 지금 재로드를 일으킬 수 있는 것은 원격 클라이언트뿐이고, TS 데스크톱은 원격에서 온 재로드를 재시작 전까지 전혀 반영하지 않으므로 이 경우의 결과는 TS 보다 나쁘지 않습니다. native 플러그인 UI 를 만들 때는 조율자에 재로드 신호를 넣어야 TS 의 "변경 뒤 항상 다시 읽기"가 됩니다.
3. 깊은 include 사슬. 순환이 아니어도 수만 단계로 이어진 include 사슬은 엔진의 구조 컴파일(`ENGINE/rule_factory.rs:193-254`)에서 스택을 넘칠 수 있습니다. 5MB 문법으로 만들 수 있는 크기입니다. TS 는 예외로 끝납니다. 깊이 상한을 두려면 근거 있는 값이 필요해 넣지 않았습니다.
4. 문법 JSON 안의 Shiki 등록 키. TS 는 문법 파일의 최상위 키를 그대로 Shiki 에 넘기므로 파일 안에 `injectTo`, `aliases`, `embeddedLangsLazy`, `balancedBracketSelectors`, `unbalancedBracketSelectors` 가 있으면 동작합니다. native 는 TextMate 문법 필드만 읽습니다. 이 키들은 tmLanguage 형식이 아니고 VS Code 확장은 `injectTo` 를 `package.json` 에 두므로(가져오기가 버림) 실제 문법 파일에 있을 가능성은 낮습니다. bracket selector 는 색과 글꼴에 영향이 없습니다.
5. 자기 문법이 없는 플러그인 언어가 TS 에서 강조되는 경우. 플러그인이 문법 없이 언어 id 만 기여했는데 그 id 가 실린 번들 문법의 이름(`sql`, `xml`, `graphql` 등 임베드 전용 문법 포함)이나 별칭과 같으면 TS 는 그 번들 문법으로 강조합니다. native 는 번들 31개 언어 id 와 플러그인 문법의 언어 id 만 토큰화합니다.
6. 범위 밖 파일 수정(`NOTICE.md`, `THIRD_PARTY_LICENSES.md`)과 prettier 미실행(5절 6번).

## 9. TS 와 다르게 남는 점

| 항목 | TS | native | 영향 |
| --- | --- | --- | --- |
| 번들 언어를 나중에 요청할 때의 scope 충돌 | 하이라이터에 언어를 덧붙이므로, 플러그인 문법이 먼저 컴파일된 scope 는 다음 재로드까지 플러그인 문법이 문서용으로 남음 | 요청 집합이 커지면 레지스트리를 새로 만들므로 항상 "재로드 직후의 TS" 상태 | scope 만 같은 플러그인 문법이 있을 때, 그 번들 언어의 문서를 처음 연 순간부터 번들 문법으로 바뀜 |
| `embeddedLangsLazy` 로 인한 재적재 | markdown·haml 이 나열한 이름의 언어가 실리면 그 문법을 다시 컴파일 | 옮기지 않음 | markdown·haml 의 scope 와 scope 만 겹치는 플러그인 문법이 있을 때만 include 대상이 달라질 수 있음 |
| scope 만 충돌할 때 문서용 문법 안의 자기 scope include | 플러그인 문법으로 감 | 번들 문법으로 감(컴파일하는 동안 레지스트리에 번들 문법이 들어 있음) | 자기 scope 를 전체 이름으로 include 하는 문법에서만 차이 |
| 구조가 잘못된 문법 | 모든 언어가 평문 | 그 문법만 평문 | 작업 항목 L3 의 요구에 따름 |
| 순환 문법을 include 하는 다른 문법 | 그 규칙에 닿은 줄만 평문 | 순환 문법이 실리지 않아 include 가 비거나 번들 문법으로 감 | 순환 문법이 번들 scope 를 덮은 경우에만 해당 |
| 실리지 않은 임베드 이름 | 콘솔 경고 | 로그 없음 | 없음 |
| 플러그인 언어 문서의 기본 글자색 | 기본 토큰색 | 추적되는 문서는 스타일 표의 기본색, 추적되지 않는 문서는 `editor.foreground`(배치 5 QA 7절과 같은 차이) | `editor.foreground` 에 알파가 있는 테마에서만 |

## 10. 변경 파일

| 파일 | 구분 |
| --- | --- |
| `native/taide-native-syntax/src/plugin-grammars.rs`, `plugin-grammars-tests.rs` | 신규 |
| `native/taide-native-syntax/src/grammar-registrations.rs`, `grammar-registrations-tests.rs` | 신규 |
| `native/taide-native-syntax/src/include-cycles.rs`, `include-cycles-tests.rs` | 신규 |
| `native/taide-native-syntax/src/{bundled-grammars.rs, bundled-grammars-tests.rs, textmate-tokenizer.rs, token-pipeline.rs, token-worker.rs, lib.rs}` | 범위 안 |
| `native/taide-native-syntax/tests/plugin-grammars.rs`, `tests/token-pipeline/plugins.rs` | 신규 |
| `native/taide-native-syntax/tests/token-pipeline.rs` | 모듈 연결 2줄 |
| `native/taide-native-app/src/{editor-syntax.rs, editor-syntax-tests.rs, application.rs}` | 범위 안 |
| `native/taide-native-app/tests/save-syntax.rs` | 관련 통합 테스트 |
| `native/taide-native-syntax/NOTICE.md`, `THIRD_PARTY_LICENSES.md` | 범위 밖(5절 6번) |
| `docs/quality-assurance/2026-10-06-native-batch6-plugin-grammars.md` | 이 문서 |

기존 테스트의 본문과 기대값은 바꾸지 않았습니다. 기존 테스트 파일에서 바뀐 것은 `use` 줄, `bundled-grammars-tests.rs` 의 단언 1개 추가, `save-syntax.rs` 의 도우미 `open`·`open_dirty`·`tokenize` 를 언어·플러그인을 받는 함수로 위임하게 한 것입니다.

## 11. 남은 위험과 실기 확인

남은 위험:

1. 8절 3번(깊은 include 사슬에서의 프로세스 중단).
2. 엔진의 다른 불변식 `expect`(`ENGINE/tokenize_string.rs`, `ENGINE/rule.rs`)가 신뢰할 수 없는 문법에서 깨질 수 있는지는 확인하지 않았습니다. worker 가 패닉으로 죽으면 강조가 멈추고 자동 재시작이 없습니다(배치 5 의 부채 그대로).
3. 순환 검사는 넓게 보는 쪽이라, 규칙 안 `repository` 에 같은 이름을 여러 번 쓰는 문법은 순환이 없어도 빠질 수 있습니다. 번들 37개 문법은 검사 대상이 아니라(플러그인 문법만 뺌) 영향이 없습니다.
4. 플러그인 문법이 하나라도 있으면 설정을 바꿀 때마다(테마 미리보기 포함) 순환 검사를 위해 실린 번들 문법 JSON 을 한 번 더 파싱합니다. worker 스레드의 일이고 시간은 재지 않았습니다.
5. 프레임마다 플러그인 목록을 조율자가 본 목록과 내용으로 비교합니다. 플러그인이 수백 개면 비용이 보일 수 있습니다(측정하지 않음).
6. 앱이 프레임을 그리지 않는 동안에는 목록 변경을 감지하지 않습니다. 다음 프레임에 반영됩니다.
7. `plugin_list` 가 처음 불릴 때 플러그인 디렉터리를 읽고 문법 파일을 검증합니다(blocking 풀). 같은 시점에 파일 열기 경로가 같은 일을 할 수 있어 시작 직후 한 번 중복될 수 있습니다(결과는 같음).

실기 확인이 필요한 것:

- [ ] 문법이 든 플러그인(직접 만든 것과 VSIX 가져오기)을 둔 상태로 앱을 시작해 그 언어의 파일이 TS 화면과 같은 색으로 보이는지
- [ ] 임베드 언어를 선언한 플러그인 문법에서 임베드 구간이 강조되는지
- [ ] 번들 언어와 id·scope 가 같은 VSIX(예: toml, dart)를 가져온 뒤 그 언어 파일과 markdown 코드 펜스의 색이 TS 와 같은지
- [ ] 원격 클라이언트에서 플러그인을 재로드·제거했을 때 열린 문서의 색이 다음 프레임에 바뀌는지, 깜빡임이 거슬리지 않는지
- [ ] 잘못된 문법이 든 플러그인이 있어도 다른 언어의 강조와 입력이 정상인지
- [ ] `trimTrailingWhitespaceOnSave` 를 켜고 플러그인 문법 언어 파일을 저장했을 때 문자열 안 공백이 남는지
- [ ] 플러그인이 많은 환경에서 시작 시간과 입력 지연이 달라지지 않는지

## 12. 테스트 부채

- [ ] TS 스택과의 충돌 규칙 대조. 재현 조건: 합성 플러그인 문법(id 충돌, scope 충돌, 둘 다, 별칭)을 `createHighlighterCore` 에 넣어 줄별 토큰을 기준 자료로 만들고 `tests/plugin-grammars.rs` 와 비교. 생략 이유: 이 단계에서 `bun` 실행이 허용되지 않습니다. 남은 위험: 2절 규칙을 소스에서 잘못 읽었을 가능성. 필요한 시점: 실기 확인에서 충돌 문법의 색이 TS 와 다를 때.
- [ ] 엔진이 순환 문법에서 실제로 스택을 넘치는지. 재현하면 프로세스가 중단됩니다. 필요한 시점: 엔진 버전을 올릴 때(엔진이 방문 표시를 넣으면 검사를 뺄 수 있습니다).
- [ ] 5MB 에 가까운 플러그인 문법의 읽기·검사·컴파일 시간. 필요한 시점: 큰 VSIX 문법에서 지연이 보고될 때.
- [ ] 읽기 작업이 진행 중일 때의 종료·재연결 경로. `TaskSupervisor` 가 작업을 추적하는 것까지만 `save-syntax` 의 `tracked_count` 단언으로 확인했습니다.
- [ ] 같은 목록·다른 문법 내용의 재로드(8절 2번). 재로드 신호가 생기면 테스트를 추가합니다.
