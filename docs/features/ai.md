# 기능 — AI (자동완성 · Inline Edit · 커밋 메시지 생성)

> 계약: `docs/acknowledge/2026-08-16-wave-g-ai-contract.md`(Inline Edit·커밋 메시지·공통 기반 리네임).
> IPC 정본: `docs/ipc-contract.md` §3 "ai"/"git"(`git_diff_staged_text`). 설정 필드는 `docs/data-model.md`.
> 세 기능(자동완성·Inline Edit·커밋 메시지)은 provider·토큰·기본 provider/model 설정을 공유한다
> (아래 §1). 자동완성 자체는 Wave G 이전부터 존재했으나 이번 문서에서 처음 정리한다.

## 1. Provider · 토큰 · 기본 provider/model (공통 기반)

- 지원 provider 3종(`AiProviderId`): `ollamaCloud`(Ollama Cloud API 키) · `codex`(비공식,
  ChatGPT/Codex 자격증명 — 설정 UI 에 경고 문구) · `omlx`(로컬 OpenAI 호환 서버, base URL 설정 가능).
- 토큰은 OS 시크릿 스토어(`SecretStore`, `SecretAccount::Ai{OllamaCloud,Codex,Omlx}`)에 저장한다.
  설정 화면(AI 섹션)에서 provider 별로 저장/삭제(`ai_set_token`/`ai_clear_token`), 저장 여부는
  `ai_token_status` → `AiTokenStatus{ ollamaCloud, codex, omlx }` 로 조회한다.
- **기본 provider/model**은 `Settings.aiProvider`/`Settings.aiModel`(문자열, 미설정 시 `null`) 이
  단일 출처다. 설정 화면의 provider 선택 → `ai_list_models(provider)` 로 모델 목록 조회 →
  모델 선택까지 하나의 흐름으로 이 두 필드에 반영된다. 세 기능(자동완성·Inline Edit·커밋 메시지) 모두
  요청에 `provider`/`model` 을 **생략하면 이 기본값으로 폴백**한다(§3·§4). 아래 §5 는 이 필드가
  `ai_auto_tab_provider`/`ai_auto_tab_model` 에서 리네임된 배경을 설명한다.
- `ai_auto_tab_enabled`(bool)은 자동완성 전용 토글로 이름 그대로 유지한다 — 리네임 대상이 아니다.

## 2. 자동완성 (`inlineSuggest`, FIM)

- 모나코 `registerInlineCompletionsProvider` 로 등록(`shared/lib/ai/inline-completion.ts` →
  `acquireAiInlineCompletionProvider`), `code-editor.tsx` 가 `aiAutoTabEnabled` 가 켜져 있을 때만
  획득한다. `resolveAiInlineCompletionConfig(settings, tokenStatus)` 가 provider/model 이 설정되어
  있고 그 provider 의 토큰이 저장돼 있을 때만 `AiInlineCompletionConfig` 를 반환 — 하나라도
  없으면 자동완성은 조용히 비활성.
- 입력 300ms 디바운스(`AUTO_TAB_DEBOUNCE_MS`), prefix 4,000자/suffix 2,000자 상한
  (`INLINE_COMPLETION_PREFIX_MAX_CHARS`/`SUFFIX_MAX_CHARS`), 커서 위치+텍스트 해시 기반 LRU 캐시
  (최대 50개, djb2 해시)로 동일 컨텍스트 재요청을 피한다.
- Rust `ai_inline_complete(request: AiInlineCompleteRequest)` — FIM 우선(`AiFimPromptTemplate`,
  `{prompt, suffix, stop}`), provider 가 FIM 을 지원하지 않으면 chat 폴백(`AiChatPromptTemplate`).
  프롬프트는 `resources/prompts/auto-tab-default.json`(id `auto-tab-default`,
  §4 의 오버라이드 규약과 동일)에서 로드해 `prompt::render`(`{prefix}{suffix}{language}{filePath}`
  치환, `filePath` 는 basename 만 — 절대경로의 사용자명·디렉터리 구조를 provider 에 보내지 않는다)로
  렌더링한다.
- 취소는 `requestId` 기반(§3 의 `AiRequestStore` 공용) — 새 입력이 들어오면 이전 요청을
  `ai_request_cancel` 로 취소한다.

## 3. Inline Edit (⌘I)

- **진입**: 모나코 에디터 액션 `taide.aiInlineEdit`(`entities/ai/ai.constant.ts` 의
  `AI_INLINE_EDIT_MONACO_ACTION_ID`) — 기본 키 **⌘I**, `code-editor.tsx` 가
  `attachAiInlineEditAction`(`features/editor/ai-inline-edit.ts`)으로 매 `[t]` 변경 시 재등록한다.
  팔레트에도 `ai.inlineEdit`(`entities/ai/ai.commands.ts` → `AI_COMMANDS`, `app/bootstrap-commands.ts`
  가 배선)로 노출되며, 팔레트 실행은 `editor-pane-command-bridge` 의 `run-monaco-action` 으로 같은
  액션을 트리거한다(`git.toggleBlame`/`git.openFileHistory` 와 동일한 브리지 패턴).
  - **`editorTextFocus` 스코프는 `keybindingContext`(액션의 `precondition` 이 아니다)**: 모나코는
    `precondition` 을 `run()`(`isSupported()`) 자체의 게이트로도 쓰므로, 거기에 `editorTextFocus`
    를 넣으면 팔레트 실행 시점(포커스가 팔레트 입력에 있음)에 `run()` 이 조용히 no-op 된다. ⌘I 라는
    키보드 단축키만 에디터 포커스로 스코프하려면 `keybindingContext` 를 써야 한다 — 모나코 빌트인
    액션(예: `editor.action.formatDocument`)들의 관례와 동일.
  - **⌘I 는 모나코 `editor.action.triggerSuggest` 의 보조 키바인딩을 대체한다**: 모나코가 mac 에서
    `editor.action.triggerSuggest` 의 secondary 키로 이미 ⌘I 를 등록해 두고 있어, 커스텀 액션의
    동적 키바인딩(우선순위 높음)이 이를 덮어쓴다. 주 바인딩인 `⌃Space` 는 그대로 동작하므로 자동완성
    자체는 잃지 않지만, ⌘I 로 자동완성을 트리거하던 사용자에게는 동작 변경으로 보일 수 있다(VS Code 도
    동일한 선택을 했다).
  - **키맵 카탈로그 노출 한계**: `taide.aiInlineEdit` 의 ⌘I 는 모나코 자체 keybinding 시스템에만
    존재하고 `APP_KEYMAP`/`KeymapActionId` 카탈로그에는 없어, 설정 → 키맵 에디터에는 "미할당"으로
    표시된다. 이는 `git.toggleBlame`/`git.openFileHistory` 등 기존 커스텀 모나코 액션 전부가 가진
    동일한 구조적 한계이며(Wave G 가 새로 만든 문제가 아님), ⌘K/chord 엔진 통합(Wave H 이후) 전까지는
    해소하지 않는다(계약 §4).
  - 선택 영역이 없으면 현재 줄을 선택으로 승격한다(`shared/lib/editor-selection.ts` 의
    `resolveSelectedTextOrCurrentLine` 과 동일한 폴백 규칙).
- **입력 위젯**: 선택 영역 상단에 뜨는 `ContentWidget`(레포 최초 도입) 하나가 idle(입력)→
  loading(생성 중, 스피너+취소)→preview(수락/거절) 3단계를 전환하며 표시한다. 에디터당 세션은
  최대 1개 — ⌘I 재실행 시 새 위젯을 쌓지 않고 기존 입력에 refocus 만 한다.
- **프리뷰 — 모델 무변경 불변식(핵심)**: 응답을 받기 전까지, 그리고 응답을 받아 프리뷰를 그리는
  동안에도 에디터 모델은 **전혀 건드리지 않는다.**
  - 원본 선택 영역은 삭제 데코레이션만 씌운다 — Wave C 충돌 데코 계열 클래스
    (`taide-conflict-incoming-background`, `--taide-diff-removed-line-background` 토큰)를 재사용.
  - 제안 코드는 선택 아래 `ViewZone`(레포 최초 도입)에 렌더 — `monaco.editor.colorize()` 로
    구문 강조(언어 미등록 등으로 실패하면 플레인 텍스트 `pre.textContent` 폴백).
  - 데코·zone·위젯은 전부 DOM/모나코 오버레이일 뿐 모델 변경이 아니므로, dirty 표시·LSP
    `didChange`·hot-exit 미러·git gutter 재계산 등 문서 변경에 연쇄되는 어떤 부수효과도 트리거하지
    않는다(§6 참조).
- **응답 처리**: `ai_inline_edit` 단발 요청(스트리밍 없음, 계약 §4 보류) → 응답 텍스트를
  `stripCodeFence`(`shared/lib/inline-edit-fence.ts`)로 후처리 — 응답 전체가 펜스 블록 하나일 필요는
  없다(모델이 지시를 어기고 앞뒤에 설명문을 덧붙여도 첫 여는 펜스~마지막 펜스 사이만 취하고 나머지는
  버림), 중첩 펜스는 최외곽만 벗기고 보존, CRLF 는 LF 로 정규화. 같은 파서를 §4 의
  `sanitizeAiCommitMessageResponse` 도 재사용한다(중복 구현 금지). 빈 응답이면 토스트
  (`ai.inlineEditEmptyResponse`) 후 세션 종료. 실패 토스트(`ai.inlineEditFailed`)는 에러 메시지를
  `description` 으로 노출한다(provider 미설정 등 원인을 사용자가 알 수 있도록).
- **수락/거절 — 상태 머신**: `shared/lib/inline-edit-preview-state.ts` 의
  `advanceAiInlineEditPreview(state, event)` 순수 리듀서가 idle/loading/preview 전이를 관리한다
  (submit/resolve/rejectResponse/accept/reject/invalidate/cancel).
  - **Return**: idle 상태에서 제출. **⌘Enter**: preview 상태에서 수락 — `model.pushStackElement()` →
    `editorInstance.executeEdits(...)` 1회 → `model.pushStackElement()` 로 **undo 경계 1개**를 만든다
    (⌘Z 한 번으로 전체 원복). **Esc**: loading 이면 요청 취소(`ai_request_cancel`), preview 면 거절 —
    두 경우 모두 데코·zone·위젯 폐기만 하고 모델은 손대지 않는다. 버튼(취소×/거절/수락)도 동일 동작을
    노출한다. IME 조합 중(`event.isComposing`) 입력은 전부 무시.
  - **무효화**: `onDidChangeModelContent` 구독이 걸려 있어, 세션 도중(생성 중이든 프리뷰 중이든)
    문서가 다른 경로로 편집되면 즉시 세션을 무효화하고 정리한다(stale 프리뷰가 옮겨간 좌표에 잘못
    적용되는 것을 방지). 탭/모델 전환(`onDidChangeModel`)·에디터 dispose 시에도 세션·리스너·데코·
    zone·위젯 전부 정리한다.
- **컨텍스트**: 선택 영역 자체(`selection`)와 지시문(`instruction`)은 자르지 않고, 주변 컨텍스트만
  앞뒤 각 2,000자로 클램프한다 — 프론트(`INLINE_EDIT_CONTEXT_CHAR_LIMIT`,
  `features/editor/ai-inline-edit.ts`)와 백엔드(`prompt::INLINE_EDIT_CONTEXT_CHAR_LIMIT`,
  `prompt.rs`) 양쪽에 같은 상수가 있다 — 프론트 클램프는 IPC payload 크기만 절약할 뿐, 백엔드가
  어차피 다시 클램프하므로 신뢰 경계는 백엔드다. UTF-8 문자 경계 안전(멀티바이트 절단 없음).
- **취소**: 생성 중 요청은 `entities/ai/ai.ipc.ts` 의 `cancelAiRequest(requestId)` 로 취소 —
  `ai_inline_edit.ipc.ts` 의 `requestAiInlineEdit` 와 짝을 이루되, 취소 자체는 auto-tab·커밋 메시지와
  같은 `ai_request_cancel` 커맨드 하나를 공유한다(entities 파일 분리와 무관하게 취소 래퍼는
  `entities/ai/ai.ipc.ts` 한 곳에만 둔다 — 중복 래퍼 금지).

## 4. AI 커밋 메시지 생성

- **진입**: SCM 패널 커밋 입력란(`features/git/commit-box.tsx`) 우상단의 Sparkles
  `IconButton`(생성 중에는 `Loader2` 스피너로 교체, 재클릭 시 취소). staged·unstaged 변경이
  **둘 다** 없으면(작업 트리에 변경 자체가 없음) 비활성 + 툴팁이 `git.noChangesForCommitMessage`
  로 바뀐다(Wave H — staged 만 보던 기존 조건을 unstaged 폴백과 함께 넓혔다. 이 툴팁 키는 Wave H
  이전 `git.noStagedChangesForCommitMessage` 를 대체한 이름이다 — "스테이지 없음"이 아니라 "변경
  없음"이 정확한 의미이므로).
- **조립** (`widgets/git-panel/git-panel-container.tsx::handleGenerateCommitMessage`):
  1. `getGitDiffStagedText(projectId)` (§7) 로 diff 조회. staged 델타가 0건이면 §7 의 워킹트리
     폴백이 자동으로 발동한다 — 이 계층은 `usedFallback` 여부를 신경 쓰지 않고 항상 같은 응답
     셰이프(`StagedDiffText`)를 받는다.
  2. `buildRecentCommitsSummaryForAi(log)`(`widgets/git-panel/ai-commit-message.ts`)가 이미 로드된
     git log 캐시에서 최근 20건(`RECENT_COMMITS_FOR_AI_CONTEXT_COUNT`)을
     `{shortHash(7자)} {summary}` 줄로 조립 — **새 IPC 조회를 추가하지 않고** 패널이 이미 들고 있는
     `gitLogQueryOptions` 캐시를 재사용한다.
  3. `ai_commit_message({ requestId, provider: null, model: null, diffText, recentCommits })` 호출 —
     provider/model 은 항상 생략해 §1 의 기본값 경로만 쓴다.
  4. 응답 텍스트를 `sanitizeAiCommitMessageResponse`(`stripCodeFence` 로 코드펜스 스트립 —
     Inline Edit(§3)과 동일 파서 — + 겹/홑/백틱/유니코드 따옴표 스트립 + trim)로 후처리한 뒤 커밋
     입력란을 **덮어쓴다**(비어있든 아니든 — VS Code 관행, 계약 §3.3. undo 위험이 낮은 텍스트
     입력이라 확인 다이얼로그 없이 즉시 대체). 단, 후처리 결과가 빈 문자열이면(모델이 빈/공백뿐인
     응답을 준 경우) 덮어쓰지 않고 `git.commitMessageEmptyResponse` 토스트만 띄운다 — 기존 입력을
     실수로 지우지 않기 위함.
  5. 성공 토스트(`git.commitMessageGenerated`)에 diff 가 절삭됐으면(`truncated`)
     `git.commitMessageDiffTruncated`, 제외 파일이 있으면(`skippedFiles.length`)
     `git.commitMessageFilesSkipped({count})`, staged 0건이라 워킹트리 폴백을 썼으면
     (`usedFallback`) `git.commitMessageUsedUnstaged`(Wave H) 를 부가 설명으로 붙인다(사용자용).
     `truncated`/`skippedFiles` 는 **모델에게도** 전달된다 — `diffText` 본문 자체에 절삭/제외
     안내 문자열이 이미 포함돼 있다(§7). `usedFallback` 은 **사용자에게만**(토스트) 알린다 — 모델이
     받는 `diffText` 는 staged/워킹트리 어느 쪽이든 동일한 unified diff 포맷이라 굳이 구분해
     알려줄 필요가 없고(모델은 "무엇이 바뀌었는가"만 요약하면 된다), 실제로 사람이 확인해야 하는
     사실("내가 스테이지한 적 없는 변경으로 메시지가 만들어졌다")은 사용자 쪽이기 때문이다.
     실패 시 `git.generateCommitMessageFailed` 토스트 + 에러 메시지를 설명으로.
  6. 취소 후 곧바로 재요청하거나 응답이 늦게 도착하는 레이스는 `latestCommitMessageRequestIdRef`
     (컨테이너 로컬 ref)로 방어한다 — 완료된 요청이 `requestId` 를 리렌더 시점 기준 최신 요청과
     비교해, 자신이 최신이 아니면 커밋 입력란도 `commitMessageRequestId` 상태도 건드리지 않는다.
- **프롬프트** (`resources/prompts/commit-message-default.json`, id `commit-message-default`):
  diff 는 그대로 요약 대상, 최근 커밋은 "스타일 참고용 — 내용 복사 금지"로 명시. 기존 스타일이
  뚜렷하면 그것을 따르고, 아니면 Conventional Commits(영어)로 폴백하도록 지시. 응답은 커밋 메시지
  텍스트만(코드펜스·따옴표·설명 없이) 출력하도록 강제하지만, 모델이 이를 어겨도 수신측에서
  스트립하므로(위 4번) 안전망이 이중이다.
- **취소**: `cancelAiRequest(requestId)` — Inline Edit 과 동일한 `entities/ai/ai.ipc.ts` 래퍼,
  같은 `ai_request_cancel` 커맨드.

## 5. 프롬프트 오버라이드 (2파일 신규 + 기존 1파일)

- 프롬프트는 번들 기본값(`resources/prompts/{id}.json`, 앱에 `include_str!` 로 내장)과 사용자
  오버라이드(`{app_data}/prompts/{id}.json`) 두 계층 — 오버라이드가 있으면 **전체 대체**(테마/로케일
  팩과 달리 `extends` 병합 개념 없음), 오버라이드 파일이 없거나 파싱 실패면 조용히 번들로 폴백한다
  (경고 없음 — `prompt::load_template<T>`, 세 프롬프트 로더가 공유하는 공통 함수).
- id 3종: `auto-tab-default`(기존, `AiPromptTemplate{version, fim, chat}`) ·
  `inline-edit-default`(신규, `AiInlineEditPromptTemplate{version, system, user}`) ·
  `commit-message-default`(신규, `AiCommitMessagePromptTemplate{version, system, user}`).
- **왜 auto-tab 템플릿에 섹션을 추가하지 않고 별도 파일 2개를 신설했는가**: `AiPromptTemplate` 의
  `fim`/`chat` 은 `serde` 기본값이 없는 필수 필드라, 기존 파일에 새 섹션을 추가하면 사용자가
  이미 커스터마이즈해 둔 오버라이드 파일은 새 필드가 없어 파싱에 실패하고 — 위 폴백 규칙에 따라
  **경고 없이 조용히 번들 기본값으로 리셋**된다. 별도 파일이면 이 위험이 없다(계약 §2-3/§4).
- 치환 변수: auto-tab `{prefix}{suffix}{language}{filePath}` / inline-edit
  `{selection}{instruction}{language}{filePath}{prefix}{suffix}`(prefix/suffix 는 §3 의 2,000자
  상한 클램프 적용) / commit-message `{diff}{recentCommits}`. `{filePath}` 는 항상 basename 만
  (절대경로 노출 방지, §2).
- **치환은 단일 패스**: `prompt::render_placeholders` 가 템플릿을 한 번만 스캔해 각 플레이스홀더를
  치환값으로 바꾼다(체인 `str::replace` 가 아니다) — 치환값(예: `selection`·`diff`)에 우연히
  `"{instruction}"` 같은 플레이스홀더 형태 문자열이 들어 있어도, 그 값은 원본 템플릿을 다시 스캔하지
  않으므로 재확장되지 않는다. 시스템 프롬프트도 `<SELECTION>`/`<DIFF>` 등 구분자 안쪽 텍스트는
  데이터일 뿐 지시가 아니라고 명시한다(프롬프트 주입 완화).

## 6. Provider 어댑터 — `instruct` (공통 기반)

- 기존 `AiProviderClient` trait 는 `list_models`·`complete`(auto-tab 전용, FIM-우선/chat-폴백)
  2메서드뿐이었다 — Inline Edit·커밋 메시지처럼 이미 렌더링된 system/user 텍스트로 범용 대화를
  보낼 진입점이 없었다. Wave G 가 3번째 메서드
  `instruct(client, model, system, user) -> AppResult<Option<String>>` 를 추가했다 —
  `complete` 는 **무변경**(auto-tab 회귀 없음).
- 3 provider(`OllamaCloudProvider`/`CodexProvider`/`OmlxProvider`) 모두 기존에 `complete` 내부에서
  쓰던 chat 폴백 구현(ollama `/api/chat` stream:false, codex responses API 누적, omlx
  `/v1/chat/completions`)을 공용 함수로 승격해 `instruct` 가 재사용한다 — 새 HTTP 경로가 아니라
  기존 요청/파싱 로직의 재사용.
- **출력 토큰 예산은 auto-tab 과 분리**: ollama/omlx 는 각각 `num_predict`/`max_tokens` 로 출력
  상한을 건다. auto-tab(짧은 ghost text 한두 줄)은 기존 256 을 그대로 유지해 회귀가 없고,
  `instruct`(선택 영역 전체 대체)는 별도 상수(`OLLAMA_INSTRUCT_NUM_PREDICT`/`OMLX_INSTRUCT_MAX_TOKENS`,
  4,096)를 쓴다 — 같은 예산을 그대로 물려받으면 긴 Inline Edit 제안이 조용히 잘려 문법이 깨진 코드가
  프리뷰에 뜨기 때문. 그래도 잘리는 경우(`done_reason`/`finish_reason` == `"length"`)는 `instruct`
  경로에서만 에러로 처리해(auto-tab 은 짧은 완성이 예산에 걸려도 정상 동작이므로 그대로 반환)
  ⌘I/커밋 메시지 실패 토스트로 드러난다. codex 는 출력 상한을 두지 않는다(`response.incomplete`
  이벤트가 이미 에러로 처리됨).
- 에러 마스킹(`mask_provider_error` — Bearer 토큰·20자 이상 불투명 토큰 문자열 리댁션, 500자 상한)과
  공용 HTTP 타임아웃(60s)은 `complete`/`instruct` 양쪽에 동일하게 적용된다.
- **요청 크기 상한**: `ai_inline_edit`/`ai_commit_message` 는 원격 dispatch 가 허용돼 있어(§8) 정상
  UI 흐름(에디터 선택 크기, `git_diff_staged_text` 의 자체 상한)을 우회한 호출도 가능하다 —
  `commands.rs` 가 `selection`(100KB)·`instruction`(4KB)·`diffText`(64KB)·`recentCommits`(8KB)
  바이트 상한을 넘는 요청을 `begin()` 이전에 `InvalidArgument` 로 거부한다(잘라내지 않고 거부 —
  §3 의 "선택 영역은 자르지 않는다" 불변식과 충돌하지 않도록).
- `service::resolve_provider_and_model(request_provider, request_model, default_provider,
  default_model)` — Inline Edit·커밋 메시지 커맨드가 공유하는 로직. 요청이 provider/model 을
  명시하면 그것을, 생략하면 `Settings.aiProvider`/`aiModel`(§1)을 쓴다. 둘 다 없거나 provider
  문자열이 알 수 없는 값이면 `InvalidArgument` 로 실패(추측 폴백 없음).

## 7. `git_diff_staged_text` (git 도메인, 커밋 메시지 생성 전용 조회)

> 상세는 `git.md` §4/§6 — git2 native 구현·상한·바이너리/lock 제외 로직은 거기서 관리한다. 여기서는
> AI 커밋 메시지가 이 커맨드를 어떻게 소비하는지만 요약한다.

- `git_diff_staged_text(projectId) → StagedDiffText{ diffText, truncated, skippedFiles,
  usedFallback }` — HEAD 트리 ↔ index 의 통합 unified diff 텍스트. 초기 커밋(HEAD 없음)에서는
  libgit2 의 암묵적 빈 트리로 처리(Wave C 선례 재사용).
- 32KiB(`STAGED_DIFF_TEXT_MAX_BYTES`) 상한 초과 시 UTF-8 문자 경계 안전하게 절삭하고
  `truncated: true`. 바이너리 델타·lock 파일(`bun.lock`·`Cargo.lock`·`package-lock.json`·
  `pnpm-lock.yaml`·`yarn.lock`)·시크릿류 파일(`.env`·`.env.*`·`.netrc`·`.npmrc`·`*.pem`·`*.key`·
  `*.p12`·`*.pfx`·`*.jks`·`*.keystore`·`*.ppk`·`*.der`·`*.crt`·
  `id_rsa`/`id_dsa`/`id_ecdsa`/`id_ed25519`)은 본문에서 완전히 제외하고 경로만 `skippedFiles` 에
  나열한다 — lock 파일은 AI 요약에 의미 있는 정보를 주지 않으면서 토큰만 소모하고, 시크릿류 파일은
  실수로 staged 됐을 때 외부 provider 로 평문 전송되면 안 되기 때문(security.md §1).
- **절삭·제외 사실은 `diffText` 본문에도 안내 문자열로 남는다**(`truncated`/`skippedFiles` 필드만이
  아니라) — 그렇지 않으면 모델이 부분 diff 를 전체로 착각하거나, 바이너리/시크릿 파일이 변경됐다는
  사실 자체를 몰라 부정확한 커밋 메시지를 자신 있게 써낸다. 파일명만 노출하고 내용은 절대 넣지 않는다
  (위 제외 규칙과 동일 경계).
- **staged 델타가 0건이면 HEAD ↔ 워킹트리(index 무시, `include_untracked`+
  `show_untracked_content`) 전체 변경으로 폴백한다**(Wave H, VS Code 관성 동일 — 아무것도 스테이지
  하지 않은 사용자도 "생성" 버튼을 누를 수 있게). untracked 파일도 포함하는 이유: "워킹트리 변경"의
  의미가 §2 의 `gutter` 전체 워킹트리 diff 와 동일해야 하기 때문(이미 추적 중인 파일 수정만이
  아니라). 어느 diff 를 썼는지는 `usedFallback` 으로만 보고된다 — **`truncated`/`skippedFiles`
  와 달리 `diffText` 본문에는 폴백 안내 문자열을 넣지 않는다**(모델이 받는 unified diff 자체는
  staged 든 워킹트리든 포맷이 동일해 굳이 구분해 알려줄 필요가 없다). 절삭·바이너리/lock/시크릿
  제외 파이프라인은 어느 diff 를 골랐든 **동일하게** 적용된다(폴백 diff 도 예외 없음).
- `spawn_blocking` 조회(무거운 git2 diff 관행, ADR-0006 스레드 모델).

## 8. 원격 세션 노출

- 신규 커맨드 3종(`ai_inline_edit`·`ai_commit_message`·`git_diff_staged_text`)과 리네임된
  `ai_request_cancel` 은 전부 원격 dispatch 허용 목록(`domain/remote/dispatch.rs`)에 등록돼
  있다 — 다만 `ai_set_token`/`ai_clear_token` 은 **d-38(2026-08-25)** 에서
  `RemoteDenialPolicy::CredentialStoreTampering` 거부로 전환됐다(근거는 `ipc-contract.md`
  §"원격 dispatch 정책" 참조). 원격 세션에서도 Inline Edit·커밋 메시지·자동완성은 그대로 쓸 수 있고,
  토큰 저장/삭제만 막힌다.

## 9. 범위 밖 (보류·기각 — 계약 §4)

| 항목 | 처리 |
|------|------|
| ⌘K 를 Inline Edit 기본 키로 | 보류 — 모나코 chord 21건이 이미 ⌘K 1단계를 점유, `when` 평가기 미구현. Wave H(chord 엔진) 이후 재검토 |
| 토큰 단위 스트리밍 응답 | 보류 — ollama/omlx 는 `stream:false` 고정이라 스트림 리더 신설이 필요. 1차는 일괄 응답 |
| diff 탭/임베드 DiffEditor 프리뷰 | 기각 — 제자리 편집 체감 상실, zone 내 별도 에디터 인스턴스 비용 |
| 모델 선반영(스트리밍 직접 편집) | 기각 — dirty·LSP·미러·undo 부작용 사슬. §3 의 모델 무변경 프리뷰 채택 |
| hunk 단위 부분 수락 | 보류 — Inline Edit 은 선택 영역=1 hunk 로 고정. 다중 위치 편집은 재설계 필요 |
| 커밋 메시지 컨텍스트에 최근 커밋 full body | 기각(1차) — summary 20건으로 시작, 품질 미달 시 후속 |
| staged 0건일 때 unstaged 로 폴백해 diff 생성 | ~~보류~~ → **Wave H 에서 구현 완료** — 버튼은 staged·unstaged 둘 다 없을 때만 비활성이고, `git_diff_staged_text` 는 staged 델타 0건이면 HEAD↔워킹트리 diff 로 자동 폴백한다(`usedFallback` — §4·§7이 정본) |
