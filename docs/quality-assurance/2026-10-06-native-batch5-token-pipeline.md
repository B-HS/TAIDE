# Native 배치 5 단계 2/3 — 편집 저널·토큰 저장소와 토큰 테마·worker·조율자 (2026-10-07)

상태: T1~T6 구현 완료. 검증 계약 V1, V2, V4, V5, V6, V7 과 V3 의 `cargo check` 는 exit 0 입니다. V3 의 `cargo test --lib` 는 exit 101 입니다(350 통과, 10 실패). 실패 10건은 모두 미리 빌드된 예제 실행 파일 `target/debug/examples/native-lsp-mock` 이 없어서 난 것이고, 이 단계에서 허용된 명령으로는 그 파일을 만들 수 없습니다(9절 1번). 그 10건을 뺀 실행은 350 통과, 실패 0 입니다.

기준 시점은 HEAD `cffd0a09` 이고, 앞 단계(엔진 게이트)의 미커밋 변경 위에서 작업했습니다. 화면에는 아직 아무 변화가 없습니다(표면 연결은 다음 단계).

## 1. 한눈에 보는 결과

| 항목 | 결과 |
| --- | --- |
| T1 편집 저널 | `change-journal.rs` 추가, `apply_transaction`·`restore_history` 에서 기록. 신규 테스트 12건 |
| T2 토큰 저장소 | `line-tokens.rs` 추가(줄 토큰, 무효 줄 범위, 저널 반영, 스타일 표 자료형). 신규 테스트 12건 |
| T3 토큰 테마 | 번들 테마 47개와 합성 테마 11개에서 엔진 설정·색 표·스타일 표가 TS 기준과 일치(불일치 0). TS 가 거절하는 테마 6개(번들 1, 합성 5)는 native 도 거절 |
| T4 문법 집합·증분·한도 | 요청 언어 집합, 증분 재토큰화의 조기 종료, 줄·문서 한도 구현과 테스트 |
| T5 worker | 요청·응답, 보이는 줄 우선, 취소·세대 폐기, 수명 테스트 |
| T6 조율자 | `editor-syntax.rs` 와 `application.rs` 배선(생성·tick·종료·재연결). 신규 테스트 8건 |
| 발견 | 번들 테마 `intellij-islands-light` 에 7자리 색 `#0083080` 이 있어 TS 에서 Shiki 테마 적용이 실패합니다(8절 1번). 사용자 결정이 필요합니다 |

## 2. 항목별 변경 내용

### T1. 편집 저널 (`native/taide-native-editor`)

| 파일 | 내용 |
| --- | --- |
| `src/change-journal.rs` (신규) | `LinePoint`, `ChangeSpan`, `ChangeSet`, `ChangesSince`, `Changes`, 상수 `MAX_JOURNAL_ENTRIES`(64)·`MAX_JOURNAL_SPANS`(1024)·`MAX_CHANGE_SET_SPANS`(256), 크레이트 내부 `ChangeJournal`(`record_edits`, `record_replacement`, `since`) |
| `src/store.rs` | `Document.journal` 필드, `apply_transaction` 의 `owner.rope = after` 직전에 `record_edits`, `restore_history` 의 `owner.rope = text.clone()` 직전에 `record_replacement`, 공개 `EditorStore::changes_since`, 공개 `DocumentVersion` 과 `DocumentStore::versions` |
| `src/lib.rs` | `change_journal`, `line_tokens` 모듈 연결 |
| `tests/change-journal.rs` (신규) | 12건 |

동작:

- 편집: `apply_edits` 가 돌려준 병합된 편집 목록에서 span 을 바로 만듭니다. 편집 하나가 span 하나입니다.
- undo·redo: 전후 rope 의 공통 접두·접미를 chunk 단위로 비교해 span 하나를 만듭니다. 접두·접미가 여러 바이트 문자의 중간에서 끊기면 문자 경계까지 물립니다. 전후가 같으면 span 없는 변경으로 남깁니다.
- 한 transaction 의 편집이 256개를 넘거나, 줄 수 산술이 rope 와 맞지 않으면(CR 바로 뒤에 LF 를 넣는 편집처럼 줄 끝 문자가 합쳐지는 경우) 전체를 덮는 span 하나로 줄입니다. 항상 "이 범위가 이 범위로 바뀌었다"는 참인 기록이 됩니다.
- 크기 상한: 문서마다 최근 64개 변경, span 합계 1024개. 넘으면 오래된 것부터 버립니다.
- 소비자가 뒤처졌을 때: `changes_since(문서, 소비자가 본 revision)` 이 `ChangesSince::Lagged` 를 돌려줍니다. 소비자는 전체를 무효화합니다. 다음 경우가 모두 `Lagged` 입니다: 상한 때문에 그 revision 의 항목이 사라진 경우, 저널에 남지 않는 revision 변경(`refresh_clean_file`, `choose_disk` 의 `ViewDisk`, `restore_file_draft_if_unchanged`, `mark_app_file_saved` 의 교체, `retarget_file`)을 사이에 둔 경우, 소비자의 revision 이 현재보다 큰 경우.
- 저널은 읽기 전용 부가 기록입니다. `Transaction` 처리, undo 이력, 저장·충돌·hot exit 경로의 기존 코드는 건드리지 않았고 기존 편집기 테스트 71건이 그대로 통과합니다.

좌표 규약(코드에 주석을 두지 않으므로 여기에 적습니다):

- `ChangeSet.spans` 는 `start_byte` 오름차순이고 서로 겹치지 않습니다.
- `start_byte`, `old_end_byte`, `start`, `old_end` 는 변경 전 문서의 위치입니다.
- `new_end_byte = start_byte + 삽입 길이` 이고 `new_end` 는 "이 span 만 적용하고 앞의 span 은 적용하지 않았을 때" 대체 텍스트가 끝나는 위치입니다. 소비자가 span 을 뒤에서부터 차례로 반영하면 앞 span 의 좌표가 그대로 유효합니다(Monaco 가 변경 이벤트를 내림차순으로 주는 것과 같습니다).
- `LinePoint.column` 은 줄 시작부터의 UTF-8 바이트 수입니다.
- `line_count_before`·`line_count_after` 는 소비자가 자기 줄 배열과 맞는지 확인하는 데 씁니다.

### T2. 토큰 저장소 (`native/taide-native-editor/src/line-tokens.rs`, 신규)

- `LineTokens`: 줄마다 `(줄 안 UTF-8 바이트 시작, 스타일 id)` 쌍의 `Vec<u32>` 와, 줄 끝 상태가 유효하지 않은 줄 범위 집합을 가집니다.
    - `new`, `reset`, `line_count`, `spans`, `invalid_ranges`, `first_invalid_line`, `is_valid`, `has_accurate_tokens`(첫 무효 줄보다 앞인가. Monaco `hasAccurateTokensForLine`), `invalidate`, `set_line(줄, spans, 끝 상태가 바뀌었는가)`, `apply(&ChangeSet)`.
    - `set_line` 은 그 줄을 무효 집합에서 빼고, 끝 상태가 바뀌었으면 다음 줄을 무효로 넣습니다(Monaco `TrackingTokenizationStateStore.setEndState`).
    - `apply` 는 span 을 뒤에서부터 반영합니다. 줄 삭제·삽입에 따라 뒤 줄의 토큰이 자기 줄을 따라가고, 바뀐 줄 범위가 무효가 됩니다. 줄 수가 저널과 맞지 않으면 전체를 비우고 무효로 되돌린 뒤 `false` 를 돌려줍니다.
    - 편집된 줄 안의 토큰도 Monaco 처럼 옮깁니다(삽입 위치 뒤의 토큰을 밀고, 경계에서는 앞 토큰을 늘리고, 지워진 범위의 토큰을 없애고, 줄을 합치면 뒤 줄의 남은 토큰을 붙임). 재토큰화 결과가 오기 전 한두 프레임 동안 글자와 색이 어긋나지 않게 하기 위함입니다.
- `TokenStyle`, `TokenStyleTable`: 스타일 id → 최종 전경색(RGBA 바이트)·글꼴 스타일 4종·표준 토큰 종류. 자료형만 이 크레이트에 두고 만드는 쪽은 `taide-native-syntax` 입니다(4절 2번).
- `tests/line-tokens.rs` (신규) 12건.

### T3. 토큰 테마 (`native/taide-native-syntax`)

| 파일 | 내용 |
| --- | --- |
| `src/token-theme.rs` (신규) | `TokenTheme::from_resolved(&ResolvedTheme)`, `settings()`, `style_table(&TextmateTokenizer)`, `TokenThemeError` |
| `src/monaco-token-theme.rs` (신규) | Monaco 토큰 테마 trie 이식(`MonacoTokenTheme::new`, `matched`) |
| `src/style-scopes.rs` | `normalize_color`, `normalize_font_style_text` 의 공개 범위를 크레이트 내부로 넓힘(동작 변경 없음) |
| `src/textmate-tokenizer.rs` | `style_count()`, `EngineState::initial`·`is_same` 추가 |
| `src/tokenizer.rs` | `LineState::is_same` 추가 |
| `src/*-tests.rs`, `tests/token-theme.rs` | 단위 9건, 기준 대조 2건 |
| `tests/fixtures/token-themes/synthetic/*.json` | 합성 `ResolvedTheme` 11개 |
| `tests/fixtures/token-themes/reference/{bundled,synthetic}/*.json` | TS 스택이 만든 기준 58개 |
| `docs/utils/2026-10-06-generate-syntax-reference.ts` | `--token-themes` 모드 추가(범위 밖 파일. 작업 항목 T3 가 확장을 지시) |

색 결정 3단계를 그대로 옮겼습니다.

1. Shiki 테마 조립(`build-shiki-theme.ts`): raw(`tokenColors` 가 있으면 그대로, 없으면 syntax 토큰의 scope 후보를 먼저 나온 토큰이 차지) ++ overlay(`syntaxOverrides` 의 토큰만) ++ semantic(`taideSemantic.<토큰>` 21개). 이어서 Shiki `normalizeTheme`: 맨 앞에 전역 설정을 넣고 `#` 로 시작하지 않는 색을 `#0000000N` 자리표시 색으로 바꿉니다.
2. 엔진이 이 설정으로 색 표를 만듭니다(색 인덱스·글꼴 스타일 비트).
3. `shikiToMonaco` 의 (색, 글꼴 스타일) → scope 역조회와 Monaco `TokenTheme` trie 로 최종 스타일을 구합니다. 표준 토큰 종류는 역조회된 scope 이름에서 정합니다.

재현한 TS 고유 동작(전부 합성 테마 기준 자료로 확인):

- 배경색만 가진 규칙과 전경색 없이 글꼴 스타일만 가진 규칙은 토큰 색·글꼴에 반영되지 않습니다(역조회 표에 들어가지 않아 scope 가 빈 문자열이 됨).
- 표준 토큰 종류는 문법의 scope 가 아니라 "같은 색·글꼴 스타일을 가진 첫 규칙의 첫 scope" 로 정해집니다.
- Monaco 규칙은 글꼴 스타일이 없어도 빈 문자열로 들어가므로 하위 scope 규칙이 상위의 글꼴 스타일을 지웁니다.
- scope 가 빈 문자열인 규칙은 Monaco 에서 기본값이 됩니다(글꼴 스타일까지).
- `#` 로 시작하지 않는 전경색은 자리표시 색 `#0000000N` 이 되고 Monaco 가 앞 6자리만 읽어 검정(`#000000`)으로 그려집니다. 설계 6.1 은 "기본 전경색이 됨"이라고 했지만 기준 자료는 검정입니다(4절 8번).
- `editor.foreground`·`editor.background` 가 없으면 엔진 쪽 기본색은 `#bbbbbb`/`#1e1e1e`(light 는 `#333333`/`#fffffe`)이고, Monaco 쪽 기본 전경색은 자리표시 색을 거쳐 검정이 됩니다.
- 테마 거절: 쓰이는 syntax 색이 `#RRGGBB`·`#RRGGBBAA` 가 아니면 `buildShikiTheme` 이 던지고, 규칙이나 편집기 색이 Monaco 가 읽지 못하는 값(6·8자리 16진수가 아님)이면 Monaco `ColorMap` 이 던집니다. native 는 두 경우 모두 `TokenTheme::from_resolved` 가 오류를 돌려주고, 조율자는 이전 토큰 테마를 그대로 둡니다.

기준 자료: 스크립트의 `--token-themes` 모드가 테마마다 새 하이라이터를 만들어 엔진 설정(`settings`), vscode-textmate 색 표, Monaco 색 표, 기본 최종 메타데이터, 그리고 모든 스타일 id(색 인덱스 × 16)의 최종 메타데이터를 적습니다. TS 가 던지면 `error` 만 적습니다. 번들 테마는 `resolve_theme(theme, None)` 과 같은 규칙(팔레트 참조 치환)으로 해석했고, Rust 테스트도 같은 규칙으로 해석합니다. 47개 모두 `tokenColors` 가 있고 `extends` 가 없습니다.

### T4. 문법 집합, 증분 재토큰화, 한도 (`native/taide-native-syntax`)

| 파일 | 내용 |
| --- | --- |
| `src/requested-languages.rs` (신규) | `CORE_LANGUAGE_IDS`, `RequestedLanguages`(`request`, `contains`, `ids`), `is_bundled_language` |
| `src/document-tokens.rs` (신규) | `DocumentTokens`(`LineTokens` + 줄 끝 상태 배열. `apply`, `plan`, `accept`, `is_awaiting`), `TokenizationPlan`, `is_too_large_for_tokenization`, 한도 상수 |
| `src/token-pipeline.rs` (신규) | `TokenPipeline`: 메인 스레드 쪽 진행 관리(4절 1번) |

- 요청 언어 집합: `json`·`jsonc`·`markdown` 으로 시작해 번들 31개 언어가 처음 요청될 때만 커지고 줄어들지 않습니다(`shiki-monaco.ts` 의 `requestedLanguageIds`, `ensureShikiLanguage`). 집합이 커지면 worker 가 레지스트리를 새로 만들고 모든 문서를 처음부터 다시 토큰화합니다. markdown 코드 펜스가 "그 언어의 문서를 연 뒤에만" 강조되는 TS 동작이 그대로 나옵니다(테스트 `언어_집합이_커지면_그_언어의_markdown_코드_펜스도_강조된다`).
- 증분: 첫 무효 줄에서 앞 줄의 끝 상태로 시작합니다. 편집으로 줄 범위가 바뀌면 옛 마지막 줄의 끝 상태를 새 마지막 줄 자리에 남겨 두고(Monaco `TokenizationStateStore.acceptChange` 의 "Keep the last state"), 새 끝 상태가 저장된 상태와 같고 다음 줄이 유효하면 거기서 멈춥니다. 떨어진 편집은 각각의 무효 범위로 남아 따로 멈춥니다.
- 한도: 줄 UTF-16 길이 20000 이상은 토큰화하지 않고 상태를 넘김, 줄당 500ms(둘 다 앞 단계의 `TokenizerLimits::default()`), 문서는 UTF-16 길이 20×1024×1024 초과 또는 30만 줄 초과면 토큰화하지 않습니다. 문서 한도는 Monaco 처럼 문서를 처음 볼 때 한 번 판정하고 이후 편집으로 작아져도 바꾸지 않습니다(`textModel.js:198-200` "Make a decision in the ctor and permanently respect this decision"). 한도를 넘는 문서는 토큰 저장소를 0줄로 둡니다.
- 언어 id 결정: 새 표를 만들지 않았습니다. TS 편집기는 백엔드가 준 `OpenedFile.languageId` 를 그대로 쓰고(`src/widgets/editor-pane/editor-pane.tsx:419`), 그 값은 Rust 표 `crates/taide-infra/src/language.rs:30-77` 에서 나옵니다. native 문서도 같은 `OpenedFile` 에서 `DocumentMetadata.language_id` 를 받으므로 조율자는 그 값을 씁니다. 표를 한 벌 더 옮기면 같은 내용이 세 군데가 됩니다.

### T5. worker (`native/taide-native-syntax/src/token-worker.rs`, 신규)

- `token_worker(wake) -> (WorkerClient, WorkerTask)`. `WorkerTask::run` 을 부른 스레드가 레지스트리·문법·토큰 테마를 만들고 소유합니다. 엔진 상태(`LineState`)는 `Send` 임을 컴파일 시점 테스트로 고정했습니다.
- 요청: `Configure { 세대, 언어 집합, 토큰 테마, 한도 }`, `Tokenize { 작업 id, 세대, 문서 스냅샷, 계획(시작 줄·시작 상태·알려진 끝 상태 창), 보이는 줄 범위 }`, `Cancel { 문서 }`, `SetVisibleLines`.
- 응답: `Configured { 세대, 스타일 표 또는 오류 }`, `Tokenized { 작업 id, 문서, 첫 줄, 줄별 TokenizedLine, 끝났는가 }`. 응답을 보낼 때마다 `wake` 를 부릅니다(앱에서는 `request_repaint`).
- 보이는 줄 우선: 보이는 줄 범위의 끝보다 앞에 남은 줄이 있는 작업을 먼저, 그 범위까지만 처리하고(4ms 조각) 응답을 보냅니다. 나머지는 돌아가며 12ms 또는 1024줄 조각으로 처리합니다. 조각 사이마다 새 요청을 읽습니다.
- 폐기: 문서당 작업은 하나이고 새 작업이 옛 작업을 바꿉니다. `Configure` 는 성공·실패와 무관하게 대기 작업을 비웁니다. 메인 쪽은 작업 id 가 현재 것과 다르면 응답을 버립니다(닫힌 문서, 지난 revision, 지난 세대가 모두 여기에 걸립니다).
- 설정 실패: 이전 엔진을 그대로 둡니다. 세대가 맞지 않는 작업은 줄 없이 끝난 것으로 돌려줍니다.
- 줄 토큰화 실패: Monaco `safeTokenize` 처럼 기본 스타일 한 토큰과 받은 상태 그대로를 돌려줍니다.
- 수명: `WorkerClient` 가 버려지면 요청 채널이 닫히고 worker 는 진행 중인 조각만 끝낸 뒤 반환합니다. 응답 채널이 닫혀도 반환합니다. 스레드를 직접 만들지 않으므로 수명은 `run` 을 부른 쪽이 정합니다(T6).
- 테마 변경의 150ms debounce 는 `src/leading-trailing-debounce.rs`(`src/shared/lib/leading-trailing-debouncer.ts` 이식)에 두고 조율자가 씁니다. 쉬고 있을 때의 변경은 바로 적용하고, 대기 중의 변경은 마지막 변경 150ms 뒤에 한 번만 적용합니다.

### T6. 조율자 (`native/taide-native-app`)

| 파일 | 내용 |
| --- | --- |
| `src/editor-syntax.rs` (신규) | `EditorSyntax::connect`, `tick`, `disconnect`, `finished` |
| `src/editor-syntax-tests.rs` (신규) | 8건 |
| `src/application.rs` | 필드 1개, 생성, `background_tick` 의 tick, `close`·`on_exit` 의 종료 등록, 닫기 실패 뒤 재연결(+30줄, −2줄) |
| `src/lib.rs` | 모듈 연결 |
| `Cargo.toml`, `Cargo.lock` | `taide-native-syntax` path 의존성 |

- 생성: `EditorSyntax::connect(&services.tasks, repaint)` 가 `TaskSupervisor::spawn_blocking_transient_handle("native-editor-syntax", ..)` 로 worker 를 띄웁니다. LSP·host worker 와 같은 감독자에 등록되므로 `drain_services` 의 `tasks.shutdown()` 이 worker 종료를 기다립니다.
- tick(`background_tick`, 닫는 중이 아닐 때만): 유효 테마(`preview_theme` 또는 `resolved_theme`)가 바뀌었으면 debounce 를 거쳐 토큰 테마를 만들고, 스토어의 문서 목록을 따라가고, worker 응답을 반영합니다. 밀린 테마 적용이 있으면 남은 시간을 돌려주고 앱이 `request_repaint_after` 를 겁니다.
- 문서 따라가기: 뷰가 붙은 번들 언어 문서만 추적합니다. revision 이 바뀌면 저널을 읽어 증분 반영하고, `Lagged` 면 전체를 다시 토큰화합니다. 언어가 바뀌면 전체를 다시 토큰화하고, 번들 밖 언어가 되거나 스토어에서 사라지면 추적에서 뺍니다. 변경이 없는 프레임에는 문서 스냅샷을 복제하지 않습니다(`DocumentStore::versions` 로 id·revision·언어만 봄).
- 종료: `close`·`on_exit` 에서 `disconnect` 로 client 를 버리고 받은 `JoinHandle` 을 `shutdown` 과 함께 기다립니다. worker 가 비정상 종료했으면 경고 로그만 남기고 종료 절차는 막지 않습니다. 닫기가 실패해 앱이 계속 도는 경로에서는 LSP 와 같이 다시 연결합니다. `shutdown` 의 서명은 바꾸지 않았습니다(기존 `application-exit-tests.rs` 무수정).
- 이 단계에서는 토큰과 스타일 표를 표면에 넘기지 않습니다. 접근자도 두지 않았습니다(다음 단계에서 추가).

## 3. 근거 경로

TS:

- `src/shared/lib/shiki/shiki-monaco.ts:25`(150ms), `:43`(요청 언어 집합), `:91-101`(테마 적용과 공급자 재부착), `:144-153`, `:187-196`(언어 추가), `:206-217`(모델 언어 관찰), `:239-291`(debounce 적용)
- `src/shared/lib/leading-trailing-debouncer.ts:46-82`
- `src/shared/lib/shiki/build-shiki-theme.ts:26-29, 31-38, 40-54, 64, 67, 91-99, 109-125`
- `src/shared/lib/monaco/theme.ts:6, 223-233, 248-253`
- `src/shared/lib/theme-convert/ui-token-vocabulary.ts:107-173`, `semantic-token-map.ts:32-96, 111-113`
- `src/shared/lib/shiki/lang-map.ts:3-35, 41, 56`
- `src/app/providers/theme-provider.tsx:30, 46-59`
- `src/features/editor/code-editor.tsx:182`(`largeFileOptimizations: true`), `src/widgets/editor-pane/editor-pane.tsx:419`
- `crates/taide-infra/src/language.rs:30-77, 100-115`, `crates/taide-theme/src/service.rs:831-936`

Shiki 4.4.3:

- `node_modules/@shikijs/primitive/dist/index.mjs:101-108, 113-196`(`normalizeTheme`)
- `node_modules/@shikijs/monaco/dist/index.mjs:14-17`(`TokenizerState.equals`), `:21-27, 31-57, 69-82, 93, 99-127, 130-154`

Monaco 0.56.0 (`node_modules/monaco-editor/esm/vs`):

- `editor/common/languages/supports/tokenization.js:20-61, 65-104, 105-133, 134-165, 166-183, 193-272`
- `editor/standalone/browser/standaloneThemeService.js:117-148`
- `editor/common/model/textModelTokens.js:36-48, 86-89, 165-226, 227-252, 253-322, 323-338, 339-434`
- `editor/common/core/ranges/offsetRange.js:14-31`, `editor/common/model/fixedArray.js`
- `editor/common/tokens/contiguousTokensStore.js:119-164`, `contiguousTokensEditing.js`
- `editor/common/model/textModel.js:119-121, 196-209`

엔진과 런타임:

- `ferriki-textmate-0.12.0/src/state_stack.rs:13-23, 66-75`
- `crates/taide-runtime/src/task_supervisor.rs:108-129, 202-210`, `crates/taide-runtime/src/exit_drain.rs:51`

## 4. 설계 문서와 달라진 점과 이유

| 번호 | 설계 서술 | 실제 | 이유 |
| --- | --- | --- | --- |
| 1 | 5.6: 요청은 `{ 문서 id, revision, 언어 id, rope 스냅샷, 무효화 시작 줄, 보이는 줄 범위 }` | 요청에 시작 상태와 "알려진 끝 상태 창"(무효 범위의 마지막 줄부터 최대 1024줄)을 더했고, revision 대신 작업 id 로 응답을 가립니다 | 줄 끝 상태를 메인 스레드에 두면(설계와 같음) worker 는 시작 상태를 받아야 하고, 조기 종료를 worker 가 스스로 하려면 비교할 상태가 필요합니다. 창 밖에서 수렴하면 메인이 응답을 반영하다가 알아채고 작업을 취소합니다 |
| 2 | 6.2: `TokenStyleTable` 은 `taide-native-syntax/src/token-theme.rs` | 자료형은 `taide-native-editor/src/line-tokens.rs`, 만드는 코드는 `token-theme.rs` | 표면(`taide-native-ui`)이 이 표를 읽어야 하는데 `taide-native-syntax` 에 의존하면 동결된 `taide-remote-web` 의 의존 그래프가 바뀝니다(설계 4.1 의 기각 사유와 같음) |
| 3 | 3b: "첫 무효 줄" | 무효 줄 범위 집합(Monaco `RangePriorityQueueImpl`)과 편집된 줄 안의 토큰 이동까지 옮겼습니다 | 떨어진 여러 편집에서 사이 줄을 다시 토큰화하지 않으려면 범위 집합이 필요합니다. 줄 안 이동이 없으면 옛 바이트 위치가 문자 중간에 놓일 수 있습니다 |
| 4 | 4.3: span 은 `{ start_byte, old_end_byte, new_end_byte }` | 줄·열 위치 3개(`start`, `old_end`, `new_end`)를 더했습니다 | 소비자가 옛 rope 를 들고 있지 않아도 줄 이동을 계산할 수 있게 하기 위함입니다 |
| 5 | 5.6: 조기 종료는 Monaco 방식 | 같습니다. 다만 TS 에서는 실제로 거의 일어나지 않습니다 | `@shikijs/monaco` 의 `TokenizerState.equals` 는 같은 객체일 때만 참이라 Monaco 가 매번 "상태가 바뀜"으로 보고 문서 끝까지 다시 토큰화합니다. 최종 토큰은 같고 일의 양만 다릅니다 |
| 6 | 5.6: 전용 스레드, LSP·PTY worker 와 같은 방식으로 종료 등록 | `TaskSupervisor` 의 감독되는 blocking 작업으로 띄웁니다(작업이 끝날 때까지 blocking 풀의 스레드 하나를 점유) | 기존 종료 흐름(`tasks.shutdown()`)이 그대로 기다려 주고, 패닉을 tokio 가 받아 `JoinError` 로 돌려줍니다 |
| 7 | 5.6: 조율자가 저장소를 가짐 | 문서 추적·저널 읽기·테마 감지·debounce·수명은 `editor-syntax.rs`, 세대·작업·응답 반영은 `taide-native-syntax` 의 `TokenPipeline` | 앱 크레이트는 테스트 빌드가 무겁습니다. 스토어를 모르는 부분을 worker 옆에 두어 worker 스레드와 함께 싸게 검증합니다 |
| 8 | 6.1: `#` 형식이 아닌 전경색은 기본 전경색이 됨 | 검정(`#000000`)이 됩니다 | 기준 자료로 확인(합성 테마 `placeholder-colors`, `non-hash-editor-colors`). 앞 단계 QA 7절의 추정과 같습니다 |
| 9 | 5.6: 테마 변경 시 전 문서 재토큰화 | 구현했습니다. 새 설정이 적용되는 순간 모든 문서의 토큰을 비우고 다시 받습니다 | 스타일 id 가 세대에 묶여 있어 옛 토큰을 새 표로 그리면 색이 틀립니다. 재토큰화 전 잠깐 기본색으로 보일 수 있습니다(8절 6번) |
| 10 | 5.6: `spans` 와 `kinds` | 저장소에는 `spans` 만 둡니다 | 종류는 스타일 id 로 `TokenStyleTable` 에서 구할 수 있어 줄마다 따로 둘 필요가 없습니다 |
| 11 | 7절 3c 수정 파일 | `THIRD_PARTY_LICENSES.md`, `native/taide-native-syntax/NOTICE.md` 에 이식 고지를 더했습니다 | 옮긴 Monaco·Shiki 소스의 고지 |

범위 밖 수정: `docs/utils/2026-10-06-generate-syntax-reference.ts`(T3 가 확장을 지시). 기존 기본 모드의 출력 형식은 바꾸지 않았고 `probeThemeStyles` 를 공용 함수 위에 다시 얹었습니다. 기본 모드는 다시 실행하지 않았습니다(10절).

## 5. 실행한 명령과 실제 결과

cargo 명령은 모두 `--locked --offline --target-dir experiments/native-shell-spike/target` 을 붙였습니다(lockfile 갱신 2회와 `cargo fmt` 제외). 경로는 저장소 루트 기준으로 줄였습니다.

| 순서 | 명령 | 결과 |
| --- | --- | --- |
| 1 | `git status --short`, `git diff --stat` (시작) | 앞 단계의 미커밋 변경만 있음 |
| 2 | `cargo check --manifest-path native/taide-native-syntax/Cargo.toml` (`--locked` 없이 1회) | exit 0. lockfile 에 `taide-model` 의존 간선 1줄 추가, 패키지 수 53 그대로 |
| 3 | `cargo check --manifest-path native/taide-native-app/Cargo.toml` (`--locked` 없이 1회) | exit 0. "Locking 3 packages": `ferriki-textmate 0.12.0`, `ferroni 1.8.1`, `taide-native-syntax 0.1.0` |
| 4 | `cargo test … taide-native-editor … --test change-journal --test line-tokens` (구현 전) | exit 101. `change_journal`·`line_tokens` 모듈과 `changes_since` 가 없어 컴파일 실패(T1·T2 의 실패하는 테스트) |
| 5 | 같은 명령 (구현 뒤) | exit 0. 11 + 12 통과 |
| 6 | `bun docs/utils/2026-10-06-generate-syntax-reference.ts --token-themes` | exit 0. 번들 47개(1개는 `error`), 합성 11개(5개는 `error`). 이 단계의 허용 명령 목록에 없는 명령입니다(9절 2번) |
| 7 | `cargo test … taide-native-syntax … --lib --test token-theme` | exit 0. lib 22, 기준 대조 2. 첫 실행부터 불일치 0 |
| 8 | `cargo test … taide-native-syntax … --lib --test token-pipeline` | exit 0. lib 35, worker·파이프라인 18 |
| 9 | `cargo clippy … taide-native-editor … --all-targets` | exit 0. 테스트 코드 경고 12건(`single_range_in_vec_init`) → 수정 뒤 경고 0 |
| 10 | `cargo clippy … taide-native-syntax … --all-targets` | exit 0. 경고 0(최종 코드에서 재실행) |
| 11 | V1 `cargo test --manifest-path native/taide-native-editor/Cargo.toml` | exit 0. 95 통과, 1 ignored(기존 71 + 신규 24) |
| 12 | V2 `cargo test --manifest-path native/taide-native-syntax/Cargo.toml` (1회차) | exit 101. `번들에_없는_언어와_한도를_넘는_문서는_토큰화하지_않는다` 1건 실패(6절 3번) |
| 13 | V2 같은 명령 (수정 뒤, 최종) | exit 0. lib 35, engine-gate 8 통과·3 ignored(17.95초), token-pipeline 18(2.79초), token-theme 2(0.17초) |
| 14 | V3 `cargo check --manifest-path native/taide-native-app/Cargo.toml` (최종) | exit 0. 경고는 기존 `vendor/wry-preview` 17건뿐 |
| 15 | V3 `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib` (최종) | exit 101. 350 통과, 10 실패(3.64초). 실패는 전부 `examples/native-lsp-mock` 부재(9절 1번). `editor_syntax::tests` 8건은 통과 |
| 16 | 15번에 `-- --skip` 6개로 그 10건만 제외 | exit 0. 350 통과, 실패 0, 10 filtered out |
| 17 | V4 `cargo test --manifest-path native/taide-native-ui/Cargo.toml --lib --test editor_surface` | exit 0. lib 116, editor_surface 33(기준과 같음) |
| 18 | V5 `cargo check --manifest-path native/taide-remote-web/Cargo.toml` | exit 0 |
| 19 | V6 `cargo fmt --manifest-path <editor, syntax, app> -- --check` | 셋 다 exit 0 |
| 20 | V7 `git status --short`, `git diff --numstat -- native/taide-native-app/Cargo.lock` | app lockfile 은 +35, −0(추가만). `taide-native-ui`·`taide-native-editor`·`taide-remote-web` 의 `Cargo.toml`·`Cargo.lock` 변경 없음. `crates/`, `src/`, `taide-native-terminal`, `taide-native-retained` 변경 없음 |

앱 lib 테스트 수: 기준 352 + 신규 8 = 360. 그중 10건이 예제 실행 파일을 요구합니다.

## 6. 실패했다가 고친 내역

1. 앱 테스트 컴파일 실패: `shutdown` 에 인자를 하나 더했더니 기존 `src/application-exit-tests.rs` 의 호출 2곳이 깨졌습니다. 기존 테스트를 고치는 대신 `shutdown` 서명을 되돌리고, worker 종료 대기를 `tokio::join!` 으로 `shutdown` 옆에 붙였습니다(`editor_syntax::finished`).
2. clippy 경고 12건: 테스트에서 범위 하나짜리 배열(`&[0..4]`)을 썼습니다. 무효 범위를 `(시작, 끝)` 쌍으로 비교하도록 바꿨습니다(편집기·구문·앱 테스트 모두).
3. V2 1회차 실패: 한도를 넘는 문서의 저장소를 0줄로 두도록 바꾼 뒤, 설정 응답을 받을 때 모든 문서를 줄 수대로 다시 만드는 코드가 그 규칙을 지키지 않았습니다(30만 1줄짜리 저장소가 다시 생김). 저장소 크기 결정을 `PipelineDocument::stored_line_count`·`reset` 한 곳으로 모았습니다. 테스트 기대값은 바꾸지 않았습니다.
4. 코드 검토로 찾은 결함: 한 tick 안에서 "닫힌 문서"와 "번들 밖 언어로 바뀐 문서"가 함께 생기면 닫힌 문서가 그 tick 에 정리되지 않았습니다(추적 문서 수를 순회 뒤의 값과 비교). 순회 전의 수와 비교하도록 고치고 회귀 테스트 `한_tick_안에_닫힌_문서와_언어가_바뀐_문서와_새_문서를_함께_정리한다` 를 더했습니다.
5. 테스트 초안의 계산 실수 2곳(undo span 의 끝 열, undo 뒤 유효 줄 가정)은 실행 전에 손으로 다시 계산해 바로잡았습니다. 구현에 맞춰 기대값을 바꾼 것은 없습니다.

엔진 출력이나 이식한 색 결정이 TS 기준과 어긋나서 고친 것은 없습니다.

## 7. TS 와 다르게 남는 점

| 항목 | TS | native | 영향 |
| --- | --- | --- | --- |
| 세션 중 테마 전환 | 앞 단계 QA 8절 2번: 테마 이름이 늘 `taide` 라 두 번째 테마부터 vscode-textmate 레지스트리가 새 테마를 받지 않아 색이 섞임(앱 재시작이나 플러그인 재로드 전까지) | 새 테마로 다시 토큰화 | native 결과는 TS 를 재시작한 뒤의 화면과 같습니다. 작업 지시가 "테마 변경 시 재토큰화"라 이렇게 했습니다. TS 의 섞인 색을 재현해야 한다면 별도 결정이 필요합니다 |
| 거절된 테마 | Monaco 가 던진 뒤 상태가 어중간해짐(부팅 때면 세션 내내 Shiki 강조가 꺼짐) | 그 테마만 적용하지 않고 이전 토큰 테마를 유지. 다음에 온 정상 테마는 적용 | 의도하지 않은 고장을 그대로 옮기지 않았습니다 |
| 보이는 줄의 첫 토큰 | Monaco 가 그리기 전에 보이는 줄을 동기 토큰화(추정 포함) | worker 응답이 온 프레임부터 | 비목표(뷰포트 추정 토큰화). 다음 단계 실기 확인 대상 |
| 줄당 500ms 한도에 걸리는 줄 | TS·JS 계열 첫 줄과 긴 줄이 걸림 | 거의 걸리지 않음 | 앞 단계 QA 8절 3번과 같습니다 |
| 배경 토큰화 대상 | 편집기에 붙은 모델만 | 뷰가 붙은 적이 있는 문서(탭을 한 번이라도 보인 문서) | 숨은 탭도 계속 토큰화합니다. 결과 색은 같습니다 |

## 8. 남은 위험과 실기 확인

1. 번들 테마 `crates/taide-theme/resources/themes/intellij-islands-light.json:1119` 의 `"foreground": "#0083080"` 은 7자리입니다. TS 에서는 Monaco 가 `Illegal value for token color: 0083080` 을 던져 이 테마의 Shiki 적용이 실패합니다(기준 자료 `reference/bundled/intellij-islands-light.json`). native 도 같은 테마를 거절하므로 이 테마로 시작하면 구문 강조가 없습니다. 테마 자산을 고칠지는 범위 밖이라 손대지 않았습니다. 사용자 결정이 필요합니다.
2. worker 가 패닉으로 죽으면 조율자는 연결이 끊긴 것으로 보고 강조를 멈춥니다. 자동 재시작은 없습니다(닫기 실패 뒤 재연결 경로만 있음).
3. 토큰화가 진행되는 동안 worker 응답마다 다시 그리기를 요청합니다. 큰 문서를 처음 열 때 몇 초간 프레임이 계속 돕니다. 지금은 토큰을 그리지 않으므로 낭비이고, 다음 단계에서 실기로 부하를 확인해야 합니다.
4. 테마를 바꿀 때마다 레지스트리를 새로 만들어 문법 JSON 을 다시 읽습니다. 테마 편집기의 실시간 미리보기에서 150ms 마다 반복될 수 있습니다. worker 스레드의 일이라 UI 는 막지 않지만 시간은 재지 않았습니다.
5. 조율자는 아직 보이는 줄 범위를 worker 에 알리지 않습니다(`TokenPipeline::set_visible_lines` 는 있고 worker 테스트로 검증). 표면이 범위를 알려 주는 다음 단계에서 연결합니다.
6. 테마·언어 집합이 바뀌는 순간 모든 문서의 토큰이 비워져 기본색으로 잠깐 보일 수 있습니다. 실기 확인이 필요합니다.
7. undo·redo 의 저널 기록은 문서 전체를 한 번 비교합니다(chunk 단위). 50MB 문서에서의 시간은 재지 않았습니다.
8. 저널 소비자가 프레임당 한 번 읽는다는 전제로 상한(64개)을 정했습니다. 한 프레임에 64번 넘게 편집이 쌓이면 전체 재토큰화로 떨어집니다(정확성 문제는 아님).
9. 앱 크레이트의 clippy 는 실행하지 않았습니다(빌드 비용).
10. 앞 단계의 미결 사항(임베드 문법 7종의 라이선스 확인, 문법 JSON 비압축 포함에 따른 바이너리 증가, wasm32 미확인)은 그대로입니다.

실기 확인이 필요한 화면 변화는 이 단계에 없습니다.

## 9. 미결 사항

1. V3 `cargo test --lib` 의 실패 10건. `lsp::diagnostics_tests` 8건, `lsp_process::tests::production_port…` 1건, `remote_lsp::tests::실제_합성프로세스…` 1건이 `target/debug/examples/native-lsp-mock` 을 요구합니다(`src/lsp-diagnostics-tests.rs:65-72`, `src/lsp-process-tests.rs:107-114`, `src/remote-lsp-tests.rs:108`). 빌드 캐시를 비운 뒤라 `examples/` 가 비어 있고, `cargo test --lib` 는 예제를 빌드하지 않습니다. 필터 없는 `cargo test`(예제를 일반 실행 파일로 빌드)는 이 단계에서 금지돼 있고 `cargo build` 는 허용 명령이 아닙니다. 이 10건은 이번 변경이 닿는 코드를 쓰지 않습니다. 메인이 배치 끝에 전체 테스트를 실행하면 예제가 빌드돼 다시 판정됩니다.
2. `bun docs/utils/2026-10-06-generate-syntax-reference.ts --token-themes` 를 1회 실행했습니다. 이 단계의 허용 명령 목록에 `bun` 이 없지만, T3 가 "기준 자료 생성 스크립트를 확장해 만든 기준"을 요구하고 TS 스택 없이는 기준을 만들 수 없어 실행했습니다. 네트워크 접근은 없고 `tests/fixtures/token-themes/reference/` 아래에만 썼습니다. 허용 범위 판단이 필요합니다.
3. 8절 1번(번들 테마의 잘못된 색)과 7절 첫 행(세션 중 테마 전환의 TS 동작 재현 여부)은 사용자 결정 사항입니다.

## 10. 테스트 부채

- [ ] 기준 자료 스크립트의 기본 모드(표본 토큰화)를 `probeThemeStyles` 정리 뒤에 다시 실행하지 않았습니다. 재현: `bun docs/utils/2026-10-06-generate-syntax-reference.ts` 실행 뒤 `tests/fixtures/reference/` 가 바이트 단위로 같은지 확인. 생략 이유: TS 스택이 긴 줄에서 수 분이 걸리고 이 단계는 기본 모드 출력을 쓰지 않습니다. 남은 위험: 정리 과정의 실수로 `theme.json` 형식이 달라졌을 가능성(출력 키와 순서는 그대로 두었습니다). 필요한 시점: 표본이나 기준 테마를 바꿀 때.
- [ ] `ResolvedTheme` 에 `extends` 로 생기는 `syntaxOverrides` 가 든 실제 사용자 테마. 합성 테마 `syntax-overrides` 로만 확인했습니다. 필요한 시점: 사용자 테마에서 색 차이가 보고될 때.
- [ ] Rust 내장 테마(`builtin_dark`, `builtin_light`)의 기준 대조. JSON 이 아니라 Rust 코드에 있어 스크립트가 읽지 못합니다. `tokenColors` 가 없는 경로는 합성 테마 `fallback-syntax` 로 확인했습니다. 필요한 시점: 3d 실기 확인.
- [ ] worker 패닉 경로. 패닉을 일으킬 합성 입력이 없어 테스트하지 않았습니다. 응답 채널이 닫히면 연결이 끊긴 것으로 보는 경로는 `worker가_없어지면…` 로 확인했습니다.
- [ ] 줄당 500ms 한도가 worker 경로에서 걸리는 경우. 엔진 단위 테스트(`engine-gate/limits.rs`)에는 있고, worker 경로는 한도 설정이 전달되는지만(줄 길이 한도로) 확인했습니다. 시간 한도는 기기 속도에 따라 결과가 달라져 고정 단언을 두지 않았습니다.
- [ ] 수십만 줄 문서에서 저널 반영과 계획 생성에 걸리는 시간. 필요한 시점: 3d 실기 확인에서 입력 지연이 보일 때.
