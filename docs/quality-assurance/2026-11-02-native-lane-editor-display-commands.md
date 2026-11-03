# lane editor 묶음 1/4 (display-commands) — editor-58·78·77·4 완료, 7·15·34 남음

> 작성일: 2026-11-02 · worktree `TAIDE-lanes/editor`(브랜치 `lane/editor`) · target `TAIDE-lanes/targets/editor`
> 기준: `docs/research/2026-10-28-lane-editor-preview-spec.md`(이하 사양) 2.1·2.2·2.3·2.6·2.9·2.15·2.16, 결정 `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안)

## 대상 ID와 결과

| ID | 결과 | 요약 |
| --- | --- | --- |
| editor-58 파일 간 이동 opener | complete | 보조 창 정의 이동 `[A]`, untitled 대상 창 범위 단위 검사 추가. 구현 변경 없음 |
| editor-78 AI 인라인 편집 | complete | 구현 없음. 현재 HEAD에서 `application::ai_edit_tests` 실행 근거 |
| editor-77 AI 인라인 완성 | complete | `⌘→` 다음 단어 수락, Accept Line, hover/항상 표시 toolbar(이전·개수·다음·Accept·Accept Word·더보기 메뉴), 리뷰 수정 때 `⌥]`/`⌥[` 키 소비 추가(남은 것: 이전·다음 tooltip) |
| editor-4 리거처 | not-applicable(결정 1 A) | epaint 0.36.2가 shaping feature를 받지 않음 |
| editor-7 semantic token | missing 유지 | 이번 묶음에서 착수하지 못함(남은 범위 절) |
| editor-15 minimap | partial 유지 | 착수하지 못함 |
| editor-34 Monaco 액션 | partial 유지 | toolbar 안 Always Show Toolbar 토글만 구현, 팔레트 실행은 남음 |

## editor-58

- 원본: 사양 2.9(`TS/shared/lib/bridge/editor-opener-bridge.ts:39-61`). `file:`은 포커스 창의 포커스 pane에 preview 탭, `untitled:`는 그 창 realm의 열린 편집기에서만 찾고 없으면 무동작.
- native 경로: `main.rs` → `NativeApplication` → 편집기 키맵(F12 `editor.action.revealDefinition`) → `symbol_location_host::open`(요청 `WindowScope`의 포커스 pane에 `enable_preview_tabs` 값으로 탭) / 문제 이동 `domains/editor.rs::open_marker` → `editor_problems::Request::allows_destination`.
- 새 검사
    - `[A]` `application::context_menu_tests::실제_보조창_정의_이동은_그_창의_포커스_pane에_preview_탭을_열고_주창을_바꾸지_않는다`: 보조 창 scope에서 F12 → 보조 창 포커스 pane의 활성 탭이 `peek.rs`(preview 값 = 설정), 정의 위치 byte 12 선택, 주 창 root·focused pane 불변, 보조 창 leaf 1개 유지.
    - 단위 `editor_problems::tests::untitled_문제_대상은_요청한_창의_편집기에_있을_때만_도착하고_다른_창이면_무동작이다`: 주 창 요청 + 주 창 untitled 허용, 보조 창에만 있는 untitled 거절, 보조 창 1 요청 + 주 창/보조 창 2의 untitled 거절, 같은 창(slot 1) 허용, 다른 프로젝트 거절.
- 기존 근거(재실행 안 함, 대응표 기존 비고): c5c2·c9c2e2c 위치 host 검사, b3g2b 문제 이동 File/Untitled.
- 원본 결함 비재현: E6(TS 내장)의 `file:///lib.*.d.ts` 정의 이동 오류는 E6 범위이며 재현하지 않음.

## editor-78

- 사양 2.16: 기능 차이 없음. `application::ai_edit_tests`·`application::ai_completion_tests` 필터 14개 통과(현재 HEAD, 구현 변경 전 1회). partial 사유였던 전체 App 준비 실패는 배치 31 환경 부채로 구분하며 이 판정에 영향 없음.

## editor-77

- 원본: 사양 1.4·2.15. `MON/editor/contrib/inlineCompletions/browser/controller/commands.js:121-165`(acceptNextWord `⌘→`, acceptNextLine 키 없음, toolbar 메뉴 primary Accept·Accept Word, secondary Accept Line), `:313-334`(Always Show Toolbar 토글), `model/inlineCompletionsModel.js:887-923`(단어 경계: 언어 word pattern 첫 일치가 0에서 시작하면 그 끝, 아니면 시작 위치, 첫 공백 묶음 끝이 더 앞이면 그곳 / 줄 경계: 첫 `\n` 다음), `hintsWidget/inlineCompletionsHintsWidget.js:48-230`(이전·`n/m`·다음, 개수 1이면 이전/다음 비활성), `hintsWidget/hoverParticipant.js:53-120`(ghost 글자 위, ghost가 줄 끝일 때 그 뒤 빈 영역, 추가 줄 view zone 위에서 hover), CSS `inlineCompletionsHintsWidget.css`(padding 4px, keybinding margin 4px·opacity 0.6·8px).
- 구현
    - `ED/completion.rs`: `Command::{CommitInline, AcceptNextWord, AcceptNextLine, ShowNextInline, ShowPreviousInline}`와 원본 id 대응, `PartialAccept`·`partial_accept_length`(ECMAScript `\s` 집합 그대로). 사양의 `ShowNext`/`ShowPrevious` 이름은 suggest 목록의 `Next`/`Previous`와 겹쳐 `*Inline`을 붙였습니다.
    - `APP/editor-ai-completion.rs`: `defers`/`event`가 `⌘→`(egui `matches_exact(COMMAND)`, mac에서 ⌃→는 제외)를 가로채 `command(AcceptNextWord)`로 보냄. `command`는 부분 삽입 뒤 기존 forward stability(`Entry::reuse`)로 남은 ghost를 유지, 전체 길이면 Tab과 같은 `commit`. 각 수락은 별도 undo 단위(`replacement_transaction`의 `UndoGroup(revision)`). 언어 word pattern은 `editor-completion-provider.rs`의 `inline_language`(기존 `supply::language`)로 전달.
    - `UI/editor-inline-suggest-toolbar.rs`(신규): hover 300ms 표시·300ms 숨김(기존 문서 hover 상수와 같은 값), Always Show면 지연 없이 표시하고 같은 줄에서 왼쪽 위치 유지, 위 공간이 없으면 아래. 항목은 Monaco 영어 고정 문구(사양 2.0.3), 키 표시 mac `⌘→`/그 외 `Ctrl+RightArrow`, `Tab`. 더보기 메뉴의 Accept Line·Always Show Toolbar(체크 표시). 항상 표시 값은 창(viewport)별 egui 메모리(비영속) — 원본 창 realm별 configuration 값에 대응.
    - `UI/editor-completion.rs`: `Provider::inline_command`·`inline_suggestion_count` 기본 메서드, inline 명령 라우팅, toolbar 위 pointer 누름은 ghost를 닫지 않음.
    - `UI/editor_surface.rs`: 완성 위젯 뒤 toolbar 호출(suggest 위젯이 열려 있으면 ghost 없음으로 넘김), 클릭 뒤 본문 포커스 복원(원본 `editor.focus()`).
- 검사
    - `[C]` `taide-native-editor --test completion` 10개 통과(새 `inline_부분_수락은_monaco의_단어와_공백과_줄_경계를_따른다`: 단어·공백·괄호·전각/NBSP 공백·구분자만·빈 문자열·줄).
    - `[U]` `taide-native-ui --features native-host,inspection --test editor-completion -- inline_suggest_toolbar` 1개 통과: hover 0.15s 미표시 → 0.4s 표시, `1/1`·Accept·Tab·Accept Word·`⌘→` 글자, 비활성 이전 클릭 무시, Accept Word·Accept Line·Accept 명령 전달, 메뉴 열기/닫기, Always Show 뒤 pointer가 떠나도 유지, 해제 뒤 숨김, 선택 불변.
    - `[A]` `application::ai_completion_tests::실제_ai_inline의_cmd_오른쪽과_toolbar는_부분수락과_남은_ghost와_undo_단위를_보존한다` 통과: 주입 응답 `first second\nthird`, `⌘→` → `first`, `⌘→` → `first `(원본 규칙상 두 번째는 공백), 남은 ghost 유지, Always Show toolbar의 Accept 클릭 → 전체, `⌘Z` 세 번이 수락 단위별로 되돌림, 디스크 원문 보존, 종료 tasks 0.
- 원본과 다른 점(작은 결정, 원본에 가장 가까운 안)
    - hover 내용: 원본은 같은 hover 위젯 안의 part이지만 native는 ghost 위(공간 없으면 아래)에 별도 상자로 그립니다. ghost 위 hover는 원본에서 다른 hover 공급자를 부르지 않으므로(foreign anchor) 보이는 내용은 같습니다.
    - 개수 표시: 원본은 hover 표시 때 `triggerExplicitly()`로 공급자를 다시 부른 뒤 개수를 채웁니다. TAIDE AI 공급자는 후보 1개라 결과 `1/1`이 같으므로 native는 재요청 없이 현재 후보 수를 씁니다(추가 AI 요청 방지).
    - 키 표시: 원본 KeybindingLabel의 키 상자 대신 같은 위치·투명도의 글자입니다.

## editor-4

- 조사: `epaint-0.36.2/src/text/text_layout.rs:1400-1434` `shape_text`가 `shaper.shape(buffer, harfrust::ShapeOptions::new())`로 feature 없이 shaping합니다. `TextFormat`(`text_layout_types.rs`)과 `FontTweak`(`fonts.rs:208-263`: scale·y_offset·hinting·subpixel·coords·thin_space·tab_size)에 OpenType feature 필드가 없습니다. 따라서 `liga`·`calt`를 끄거나 켤 API가 없고 HarfBuzz 기본(liga·calt 켜짐)으로만 그립니다.
- 결정 1 A: "egui 한계"로 not-applicable 정정. 원본 기본값(리거처 끔)과 달리 사용자가 리거처 글꼴(Fira Code 등)을 지정하면 native 본문에 항상 리거처가 보입니다. 기본 글꼴(SF Mono/Menlo)에는 리거처가 없어 기본 상태에서는 차이가 없습니다. vendor 패치(결정 1 B)는 허락 항목입니다.

## 남은 범위(다음 작업자가 그대로 이어서)

- editor-7 전체(사양 2.2 설계 1~5와 검사): `ED/semantic-tokens.rs`, `SYN/token-theme.rs::semantic_style_table`, `APP/lsp.rs` initialize capability·refresh, `APP/lsp-editor-semantic-tokens.rs`·`APP/editor-semantic-tokens.rs`, `UI/editor-row-text.rs::restyle`, `MOCK --native-semantic-tokens`. 새 앱 상태는 `domains/editor.rs`에 `State`를 만들어 둡니다(아직 없음).
- editor-15 전체(사양 2.3): section header 이식·표시, 접힌 영역 배경, 선택 발생 강조 확인.
- editor-34(사양 2.6): 표의 액션 전부와 결정 2 A의 editor-88~(Unicode 강조·텍스트 링크 감지) 새 행. `editor.action.inlineSuggest.toggleAlwaysShowToolbar` 팔레트 실행은 `toggle_always_shown`(UI 공개 함수)을 편집기 없이도 부르는 경로가 필요합니다(원본 Action2는 편집기 조건 없음). 팔레트 실행 경로가 commands 영역(`domains/commands.rs`)이라 이번에 손대지 않았습니다.
- editor-77: 처음에 `⌥]`/`⌥[`(Show Next/Previous) 키 소비를 "테스트 부채"로 적었으나 실제로는 원본 기본 키 경로의 구현 공백이었습니다. 아래 "리뷰 수정" 절에서 구현했습니다. 남은 것은 이전·다음 버튼 tooltip 표시 차이뿐입니다.

## 검증 명령과 결과

모든 명령은 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/editor`, manifest는 lane worktree 아래 절대 경로입니다. 같은 성공 검사는 합산하지 않습니다.

| 명령 | 결과 |
| --- | --- |
| `cargo build --example native-lsp-mock`(app) | 성공(lane target 첫 빌드) |
| app `cargo test --lib -- application::ai_edit_tests application::ai_completion_tests --test-threads=2`(구현 전) | 14 통과 |
| app `cargo test --lib -- 보조창_정의_이동은 untitled_문제_대상은 --test-threads=2` | 2 통과(첫 시도 실패 2회: 보조 창 view 생성 전 조회 → 생성 대기 추가, 팔레트 첫 행이 Peek Definition → 실제 F12 키 경로로 변경. 구현 결함 아님) |
| app `cargo test --lib -- cmd_오른쪽과_toolbar --test-threads=2` | 1 통과 |
| editor `cargo test --test completion` | 10 통과 |
| ui `cargo test --features native-host,inspection --test editor-completion --test editor-display` | 35·44 통과 |
| ui `cargo test --features native-host,inspection --lib` | 167 통과 |
| app `cargo test --lib -- application::ai_completion_tests application::ai_edit_tests application::context_menu_tests application::inlay_hints_tests application::emmet_tests editor_ai_completion editor_completion editor_problems symbol_location --test-threads=2` | 148 통과(새 검사 포함, 위 개별 실행과 합산하지 않음) |
| editor·ui(`--features native-host,inspection`)·app `cargo check --quiet --all-targets` | 성공, 기존 경고만(editor `DocumentSymbol::deprecated`, ui `tests/workbench.rs` unused mut, app vendor wry·`is_applying`·`zen.rs` mut·`fixture_with_emmet_peek`) |
| remote-web `cargo check --quiet --target wasm32-unknown-unknown --lib` | 성공, 기존 dead_code 경고만. remote-web 소스·Cargo 파일 무변경 |
| editor·ui·app `cargo fmt -- --check` | 통과(새 코드에 rustfmt 적용 뒤) |

실행하지 않은 것: 필터 없는 앱 전체 테스트(메인 병합 때), remote-web `--tests --features inspection`(이번 지시의 확인 명령은 wasm lib 1개), GUI 실기.

## 바꾼 파일

- `native/taide-native-editor/src/completion.rs`, `tests/completion.rs`
- `native/taide-native-ui/src/editor-inline-suggest-toolbar.rs`(신규), `src/editor-completion.rs`, `src/editor_surface.rs`, `src/lib.rs`(모듈 한 줄, 정렬 위치), `tests/editor-completion.rs`
- `native/taide-native-app/src/editor-ai-completion.rs`, `src/editor-completion-provider.rs`, `src/editor-completion-consumer-tests.rs`(호출 인자 1개), `src/application-ai-completion-tests.rs`, `src/application-context-menu-tests.rs`, `src/editor-problems-tests.rs`
- 문서: 이 파일, `2026-10-09-native-function-matrix.md`(editor-4·15·34·58·77·78 행)

공용 파일은 `ui/src/lib.rs` 모듈 목록 한 줄만 추가했고 `application.rs`·`host.rs`·`commands.rs`·locale은 바꾸지 않았습니다(편집기 위젯 안 문구는 원본처럼 영어 고정이라 새 locale 키 없음). 새 앱 상태가 없어 `domains/editor.rs`의 `State`는 만들지 않았습니다(AI 완성 상태는 기존 `editor_completion.inline`).

## 리뷰 수정 (2026-11-03)

차단 항목은 없었고 과대 판정 3건과 비차단 항목을 반영했습니다.

### editor-77 `⌥]`/`⌥[` — 테스트 부채가 아니라 구현 공백이었음, 구현함

- 원본: `MON/editor/contrib/inlineCompletions/browser/controller/commands.js:25-60`. `ShowNextInlineSuggestionAction`(`Alt+]`)과 `ShowPreviousInlineSuggestionAction`(`Alt+[`)의 precondition이 `writable && inlineSuggestionVisible`이고, `EditorAction`은 precondition을 keybinding `when`에 합칩니다. 따라서 ghost가 보일 때만 키를 소비하고(mac에서 Option 조합 글자 `‘`·`“`가 입력되지 않음), ghost가 없으면 그 글자가 그대로 입력됩니다. 실행은 `model.next()`/`previous()`이며 후보를 다시 요청해 순환합니다.
- 재현: 새 `[A]` 검사를 수정 전에 실행해 실패를 확인했습니다. ghost가 보이는 상태에서 `⌥]`를 누르면 문서가 `let value = ‘`가 됐습니다(기대 `let value = `).
- 원인: 키 경로가 없었습니다. 키맵 기본값과 inline 상태 `defers`/`event`(`APP/editor-ai-completion.rs`) 어디에도 `⌥]`/`⌥[`가 없었습니다. 또 키를 소비해도, 기존 keymap의 `claimed_text`는 키 이름과 같은 글자만 막으므로 뒤따르는 Text 이벤트(`‘`)를 막지 못했습니다.
- 수정
    - `APP/editor-ai-completion.rs`: `cycles_inline`은 `Alt` 단독(`matches_exact(ALT)`) + `CloseBracket`/`OpenBracket`를 `Command::ShowNextInline`/`ShowPreviousInline`으로 바꿉니다. `defers`와 `event`가 ghost가 있을 때만 이 키를 받아 기존 `command`로 보내며, 명령은 `Ok(true)`(소비)를 돌려줍니다. `⌘→`의 `accepts_next_word`와 같은 경로입니다.
    - `UI/editor-completion.rs`: `State.inline_claimed_text`가 inline 공급자가 소비한 Alt 키(ctrl/cmd 없음)의 raw index를 기억합니다. 바로 다음 index의 `Event::Text`만 소비하고, 매 프레임 `begin`에서 지웁니다. keymap `claimed_text`와 같은 index+1 규칙이며, Option 조합 글자는 키 이름과 다르므로 글자 비교는 하지 않습니다.
    - 원본과 다른 점(작은 결정): 원본 `next()`는 공급자를 명시 재요청합니다. TAIDE AI 공급자는 후보가 1개라 결과(`1/1`, 같은 ghost)가 같으므로 추가 AI 요청을 하지 않습니다. 기존 toolbar 개수 표시와 같은 판단입니다.
- 검사 `[A]` `application::ai_completion_tests::실제_ai_inline의_option_대괄호는_ghost가_보일_때만_키와_조합_글자를_소비한다`
    - 실제 App에서 `⌘Q` 명시 요청으로 ghost를 띄운 뒤 `[Key ⌥] 누름, Text "‘", Key 뗌]`와 `⌥[`+`“`를 보냅니다. 문서는 그대로이고 후보 수는 `Some(1)`, ghost `second`는 계속 그려집니다.
    - `Escape`로 ghost를 닫은 뒤 같은 `⌥]`+`‘`는 `let value = ‘`로 입력됩니다(원본처럼 키를 소비하지 않음). 디스크 원문은 보존되고 종료 task는 0입니다.
- 실제 앱 경로: `main.rs` → `NativeApplication` → 편집기 본문 `body_input` → `route_editor_keymap`(`defers`가 참이라 편집기 기본 키를 미룸, `⌥]` 바인딩 없음) → UI 완성 위젯 `event` → `Provider::inline_event` → `editor_ai_completion::State::event`.
- 남은 범위(구현 공백, 영향 작음): 원본 이전·다음 버튼의 tooltip(`appendKeybinding`: `Previous (⌥[)`, `Next (⌥])`)을 native toolbar는 그리지 않습니다. 후보가 1개라 두 버튼은 늘 비활성입니다.

### editor-77 inline 부분 수락의 언어 None — 리뷰 반박

- 리뷰: `supply::language`가 None인 언어 id는 `first_word`가 None이 되어, 공백 전까지 전부 수락한다.
- 실제 코드: `APP/editor-completion-supply.rs:33-42`의 `language`는 등록되지 않은 id면 `monaco_language("plaintext")`로 대체합니다. `SYN/language-configurations/monaco.json`의 plaintext 항목(6-59행)에는 `wordPattern`이 없습니다. 그래서 `SYN/language-configuration.rs:580-586`이 Monaco 기본 단어 정규식 `(-?\d*\.\d\w*)|([^`~!@#$%^&*()\-=+\[{}\]\\|;:'\",.<>/?\s]+)`으로 컴파일합니다. 원본의 `getLanguageConfiguration(langId).wordDefinition` 기본값과 같습니다.
- None은 내장 언어 설정 JSON 자체를 읽지 못할 때만 생깁니다. 이 경우는 빌드에 포함된 자료의 불변식 위반이라 사용자 언어별 차이가 아닙니다. 코드는 바꾸지 않았습니다.

### editor-77 `suggestion_count` 표현

- 리뷰의 표현 사소점을 반영했습니다. `entry.text.iter().count()`를 `usize::from(entry.text.is_some())`로 바꿨고 동작은 같습니다.

### editor-78 — 근거 성격 명시

- 판정은 complete로 유지합니다. 근거는 새 검사가 아니라 기존 `application::ai_edit_tests`·`ai_completion_tests`를 다시 실행한 것이고, editor-78 코드 변경은 없습니다. 기능 대응표 비고에도 이 문장을 넣었습니다.
- 이번 수정 뒤에도 같은 필터 묶음이 다시 통과했습니다(아래 검증). editor-77의 키 경로 추가가 ⌘I 경로에 회귀를 만들지 않았다는 근거입니다.

### editor-4 — settings-28 비고 확인

- 기능 대응표의 settings-28 비고는 "리거처/semantic/... 잔여 소비자는 없습니다"입니다. editor-4의 not-applicable(egui 한계)과, 리거처 글꼴을 지정하면 원본 기본 OFF와 달리 리거처가 보인다는 차이가 아직 반영되지 않았습니다.
- settings-28은 이 lane의 ID가 아니므로 행을 고치지 않았습니다. 병합 때 메인이나 settings lane이 같은 문장을 넣어야 합니다.
- 근거 조사 절차 차이: 사양 2.2의 "cargo doc 소스 페이지로 확인, registry 소스 직접 읽기 금지" 대신 첫 작업에서 registry 소스 줄 번호를 인용했습니다. 이 lane의 허용 명령에는 `cargo doc`이 없고 lane target에 doc 산출물도 없습니다. 리뷰가 같은 내용을 독립적으로 확인했으므로 판정은 유지하고, 절차 차이만 여기에 기록합니다.

### 병합 충돌 관리 — 리뷰 반박

- ED·UI·app `lib.rs`의 모듈 선언을 알파벳 위치에 넣은 것은 `docs/research/2026-10-28-native-lane-seams.md` 103행 규칙("`lib.rs` 모듈 목록 … 새 모듈 한 줄을 정렬 위치에 추가. 충돌해도 정렬 병합이면 된다")을 그대로 따른 것입니다. 영역 블록 끝 추가 규칙은 영역 블록이 있는 파일에만 적용됩니다.
- css/html 앱 검사 모듈을 `application-json-tests.rs` 끝에 등록한 것은 루트 `application.rs`를 피하려는 의도였습니다. 이 파일은 내장 언어 서비스(JSON) 검사 파일이라 editor lane만 손댑니다. 병합 때 충돌이 나도 등록 두 줄을 합치면 됩니다.

### 검증 공백 보완

- 사양 37행의 동결 미러 두 명령을 이번에 실행했습니다. `--tests --features inspection`(host)와 `--lib --features canvas --target wasm32-unknown-unknown`입니다. 첫 묶음에서는 wasm lib 하나만 실행했었습니다.
- 필터 없는 앱 전체 테스트는 지시대로 메인 병합 때 실행합니다.
- 새 파일은 모두 untracked입니다. 선별 스테이징 때 `git status --short`의 `??` 목록(ED `color.rs`·`css-*`·`default-colors.rs`·`html-*`·`json-completion.rs`·`json-hover.rs`, tests 4개, UI `editor-inline-suggest-toolbar.rs`, app `application-css-tests.rs`·`application-html-tests.rs`·`editor-completion-json.rs`·`editor-css.rs`·`editor-html.rs`, QA 4개)이 빠지지 않아야 합니다.

### 리뷰 수정 검증

| 명령 | 결과 |
| --- | --- |
| app `cargo test --lib -- option_대괄호는 --test-threads=2`(app 수정을 일시로 뺀 상태) | 1 실패: `left: "let value = ‘"`, `right: "let value = "` (재현) |
| 같은 명령(수정 복원) | 1 통과 |
| app `cargo test --lib -- application::ai_completion_tests application::ai_edit_tests editor_ai_completion editor_completion --test-threads=2` | 110 통과 |
| ui `cargo test --features native-host,inspection --test editor-completion --test editor-display` | 35·44 통과 |
| ui `cargo test --features native-host,inspection --lib` | 167 통과 |
| ui `cargo check --quiet --all-targets --features native-host,inspection` | 성공, 기존 경고만(`tests/workbench.rs` unused mut) |
| app `cargo check --quiet --all-targets` | 성공, 기존 경고만(vendor wry, `is_applying`, `zen.rs` mut, `fixture_with_emmet_peek`). `editor-selection-ranges.rs` 이름 수정 뒤 1회 다시 실행해 같은 결과 |
| app `cargo test --lib -- application::json_tests editor_selection_ranges --test-threads=2`(이름 수정 뒤) | 19 통과 |
| remote-web `cargo check --quiet --tests --features inspection` | 성공, 경고 없음. remote-web 소스·Cargo 파일 무변경 |
| remote-web `cargo check --quiet --lib --features canvas --target wasm32-unknown-unknown` | 성공, 경고 없음 |
| app·ui `cargo fmt -- --check` | 통과 |

수정 파일: `native/taide-native-app/src/editor-ai-completion.rs`, `src/application-ai-completion-tests.rs`(검사 1개 끝에 추가), `src/editor-selection-ranges.rs`(지역 변수 이름 1개, HTML QA 수정 절), `native/taide-native-ui/src/editor-completion.rs`, 문서는 이 파일·`2026-11-02-native-lane-editor-html-services.md`·`2026-10-09-native-function-matrix.md`(editor-49·77·78 행).
