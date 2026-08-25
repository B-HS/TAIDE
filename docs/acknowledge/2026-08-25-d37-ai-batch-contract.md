# d-37 — AI 묶음(응답 타입 통합·codex 절단 보정·파이프라인 판단) 계약 (2026-08-25)

> 정본: `2026-08-25-post-batch-user-decisions.md` §2(bindings 표면·행동 변경 승인) / 이월 원문
> `2026-08-20-t2a-dedup-contract.md` #17(deferred)·#18(deferred)·#12(deferred)·§3.4-2·3.
> 선행: d-36 완료 후 착수(Rust 한 시점 한 에이전트).

## 0. 착수 전 메인 실사 (2026-08-25)

- **#17 동형 3타입**: `domain/ai/types.rs` `AiInlineCompleteResponse`(:48)·`AiInlineEditResponse`
  (:141)·`AiCommitMessageResponse`(:162) — 전부 `{request_id, text: Option<String>}` 동형.
  TS 소비: `shared/lib/ai/inline-completion.ts`(이름 직접 import)·`widgets/git-panel/
  ai-commit-message.ts`(+test)·`bindings.ts`. 구조체 직렬화라 **와이어 페이로드는 타입명과
  무관** — 통합 시 런타임 동작 불변, TS 타입명 표면만 변경(승인 완료).
- **#18 codex 절단**: `providers/codex.rs` `classify_codex_event`(:134)가
  `response.failed | response.incomplete` 를 동일 arm(:141)에서 상시 에러 처리 —
  ollama/omlx 의 `fail_on_truncation` 분기(auto-tab 관용/instruct 에러화)가 codex 에는 부재.
- **#12 파이프라인**: ollama(`options.num_predict`·`done_reason`) vs omlx(평탄 `max_tokens`·
  `choices[].finish_reason`) — 이름만 같고 JSON 형태 상이(d-28 실사 그대로).

## 1. 범위 (사용자 승인 — 설계 세부 위임)

- **a. 응답 타입 통합(#17)**: 동형 3타입을 단일 타입으로 통합(명명은 구현이 기존 관행 대조로
  확정·기록, 예: `AiTextResponse`). 커맨드 반환 타입·TS 소비처 전수 갱신. bindings 델타는
  "3타입명 소멸·1타입 신설"만이어야 하며 §3 에 명시 대조 기록. 와이어 형태(camelCase 필드)
  불변 확인(원격 dispatch 직렬화 테스트로).
- **b. codex 절단 보정(#18)**: `fail_on_truncation` 을 codex 경로에 전파 —
  `response.incomplete` 는 관용 경로(auto-tab)에서 **부분 텍스트 반환**, instruct 경로는
  현행 에러화 유지(ollama/omlx 와 의미 일치). `response.failed` 는 상시 에러 유지.
  기존 SSE 테스트 보강(incomplete 관용/에러 두 분기).
- **c. ollama↔omlx 트레이트 판단(#12)**: 실사 후 **실질 공통부만** 추상화(요청 빌더·응답
  추출을 트레이트/콜백 파라미터화). JSON 형태가 상이한 부분의 억지 통합 금지 — 통합/보류
  판단과 근거를 §3 에 기록(보류도 유효한 결론).
- 검증: cargo fmt/clippy/test 전량 + bindings 재생성 델타 명시 대조 + `bun run typecheck`+
  `bun test` 전량 + TS 소비처 죽은 import 0.
- 범위 외: provider 신규 기능·프롬프트 변경·d-34 에러 taxonomy 재론·요청(Request) 타입 통합
  (동형 아님 — 대상은 Response 3종만).

## 2. 실행·검증

- 구현: Rust+TS 1 에이전트(sonnet+xhigh, d-36 종료 후) — §3 기록 필수.
- 검토 3렌즈(opus+xhigh): ① 표면 델타(bindings 3소멸·1신설 외 0·와이어 불변) ② 행동 동등성
  (codex 관용 분기가 ollama/omlx 의미와 일치·instruct 에러 경로 불변·#12 처분 타당성) ③
  컨벤션(명명·doc·테스트 관행). major 적대적 → 수정 → 메인 2차. 커밋은 사용자 규칙.

---

## 3. 구현 완료 기록 (구현 에이전트가 기록)

### 3.1 a. 응답 타입 통합(#17)

**명명**: `AiTextResponse`(계약 §1 예시안 채택). 근거 — 저장소 전역에서 `pub struct.*Response`
패턴을 재실사(`domain/*/types.rs`·`domain/*/*.rs` grep)한 결과, 대상 3타입(`AiInlineCompleteResponse`/
`AiInlineEditResponse`/`AiCommitMessageResponse`) 외에 참조할 만한 "여러 커맨드가 공유하는 응답
타입" 선례가 없었다(`ide/server.rs::JsonRpcResponse`는 JSON-RPC 프로토콜 봉투로 무관). 기존
`AiTokenStatus`·`AiModelInfo`도 커맨드 이름이 아니라 **페이로드가 무엇인지**로 명명되어 있어,
이 관행을 따라 페이로드(`text`)를 이름에 반영하는 `AiTextResponse`로 확정했다. `AiInlineComplete*`
등 커맨드-접두 명명을 유지한 채 통합하면(예: 셋 중 하나로 흡수) 나머지 두 커맨드의 응답이 자신과
무관한 이름을 갖게 되어 부적절.

**소비처 전수 갱신**:
- `src-tauri/src/domain/ai/types.rs` — 3개 구조체 삭제, `AiTextResponse` 1개 신설(위치는 기존
  `AiInlineCompleteResponse` 자리, doc 포함). 직렬화 카멜케이스 회귀 테스트 1건 신규 추가.
- `src-tauri/src/domain/ai/commands.rs` — `ai_inline_complete`/`ai_inline_edit`/`ai_commit_message`
  3개 커맨드의 반환 타입(`AppResult<...>`)과 각 함수 말미의 구조체 리터럴 3곳을 `AiTextResponse`로
  교체. import 목록에서 옛 3타입명 제거.
- `src/shared/lib/ai/inline-completion.ts` — 저장소 전체에서 옛 타입명을 **이름으로 직접 import**
  하는 유일한 TS 파일(계약 §0 실사와 일치). `AiInlineCompleteResponse` → `AiTextResponse`로 import·
  `AiInlineCompletionClient.complete`의 반환 타입 교체.

**전수 grep으로 갱신 불필요를 확인한 소비처**(코드 변경 없음, 사유 명시):
- `src-tauri/src/domain/ai/service.rs` — `complete`/`inline_edit`/`commit_message` 전부 `Option<String>`만
  반환, 옛 3타입 어디도 참조하지 않음.
- `src-tauri/src/domain/remote/dispatch.rs` — `ai_inline_complete`/`ai_inline_edit`/`ai_commit_message`를
  `respond::<T: Serialize>(...)` 제네릭으로 감싸 호출할 뿐 타입명을 언급하지 않음 — 커맨드 함수
  시그니처 변경이 그대로 흘러 들어간다. 원격 dispatch 테스트 128건 전부 재통과 —
  단 이 군은 커맨드 이름 파리티·거부 정책만 고정하고 AI 페이로드 직렬화는 실행하지 않는다
  (**§4 L1-03 정정** — 와이어 불변의 실근거는 `respond`→`serde_json::to_string` 이 같은
  구조체를 쓰고 그 serde 속성이 통합 전후 동일하며 `types.rs` 카멜케이스 테스트가 이를
  단언한다는 것. §4 수정 4로 dispatch 계층 와이어 단언 테스트 1건 추가).
- `src/entities/ai/ai.ipc.ts`·`src/entities/ai/ai-inline-edit.ipc.ts` — `commands.aiInlineComplete(...)`
  등의 반환 타입을 추론에 맡길 뿐 타입명을 import하지 않음 — bindings 갱신이 투과적으로 적용됨.
- `src/widgets/git-panel/ai-commit-message.ts`(+`.test.ts`)·`git-panel-container.tsx` —
  `sanitizeAiCommitMessageResponse`는 문자열 새니타이저 **함수명**(우연히 "Response"를 포함)이지
  타입 참조가 아님.
- `src/features/editor/code-editor.tsx` — `AiInlineCompletionClient`(=`inline-completion.ts`가 정의,
  이미 갱신됨)만 참조, bindings 타입을 직접 참조하지 않음.

**bindings 델타 원문**(`git diff -- src/shared/api/bindings.ts`, cargo test로 재생성):

```diff
-	aiInlineComplete: (request: AiInlineCompleteRequest) => typedError<AiInlineCompleteResponse, AppError>(__TAURI_INVOKE("ai_inline_complete", { request })),
+	aiInlineComplete: (request: AiInlineCompleteRequest) => typedError<AiTextResponse, AppError>(__TAURI_INVOKE("ai_inline_complete", { request })),
...
-	aiInlineEdit: (request: AiInlineEditRequest) => typedError<AiInlineEditResponse, AppError>(__TAURI_INVOKE("ai_inline_edit", { request })),
+	aiInlineEdit: (request: AiInlineEditRequest) => typedError<AiTextResponse, AppError>(__TAURI_INVOKE("ai_inline_edit", { request })),
...
-	aiCommitMessage: (request: AiCommitMessageRequest) => typedError<AiCommitMessageResponse, AppError>(__TAURI_INVOKE("ai_commit_message", { request })),
+	aiCommitMessage: (request: AiCommitMessageRequest) => typedError<AiTextResponse, AppError>(__TAURI_INVOKE("ai_commit_message", { request })),

- export type AiCommitMessageResponse = { requestId: string, text: string | null };   (삭제, 원위치)
- export type AiInlineCompleteResponse = { requestId: string, text: string | null };  (삭제, 원위치)
- export type AiInlineEditResponse = { requestId: string, text: string | null };      (삭제, 원위치)
+ export type AiTextResponse = { requestId: string, text: string | null };            (신설, doc 포함)
```

`git diff --stat`: `37 +++++++++++++++++++------------------`(19 insertions, 18 deletions) — **3
타입명 소멸(정의 3곳 + 사용 3곳) · 1 타입 신설(정의 1곳 + 사용 3곳)** 외 다른 변경 없음(계약 §1
요건과 일치). 필드(`requestId: string, text: string | null`)는 세 타입 모두 완전히 동일했으므로
와이어 페이로드 불변 — 직렬화 카멜케이스 테스트(`ai_텍스트_응답은_request_id와_text를_카멜케이스로_직렬화한다`)로
Rust 측도 재확인.

### 3.2 b. codex 절단 보정(#18)

**실사**: ollama(`providers/ollama.rs::extract_chat_text`)·omlx(`providers/omlx.rs::extract_chat_text`)
둘 다 `fail_on_truncation: bool` 파라미터로 "잘림(`done_reason`/`finish_reason` == `"length"`)"을
`false`(auto-tab, 관용 — 잘린 텍스트를 그대로 반환) / `true`(`instruct` — 에러화) 두 갈래로 나눈다.
codex.rs의 `classify_codex_event`는 SSE `response.failed`(진짜 실패)와 `response.incomplete`(단순
잘림)를 **같은 match arm**에서 항상 `CodexStreamStep::Failed`로 접어 넣어, 이 구분 자체가 없었다.

**수정**:
- `CodexStreamStep`에 `Incomplete(String)` variant 신설(`Failed`와 분리).
- `classify_codex_event`가 `response.failed`→`Failed`, `response.incomplete`→`Incomplete`로 분류
  (에러 메시지 추출 로직은 2회 사용이 확인되어 `codex_event_message` 헬퍼로 공통화).
- `read_codex_completion`에 `fail_on_truncation: bool` 파라미터 추가. `Incomplete` 스텝을 만나면
  `fail_on_truncation`이 `true`면 마스킹된 에러, `false`면 **그때까지 누적된 델타 텍스트**를 반환
  (스트림이 아무 종결 이벤트 없이 끝났을 때의 기존 관용 처리와 동일한 코드 경로/의미).
- `send_responses_request`에 `fail_on_truncation` 파라미터를 추가해 `read_codex_completion`까지
  스레딩. `AiProviderClient::complete`(auto-tab)는 `false`, `instruct`는 `true`로 호출 —
  ollama/omlx의 `complete_chat`/`instruct` 호출 패턴과 동형.
- `response.failed`는 두 분기 모두에서 무조건 에러(변경 없음).

**전후 의미표**:

| SSE 이벤트 | 수정 전 | 수정 후 — auto-tab(`fail_on_truncation=false`) | 수정 후 — instruct(`fail_on_truncation=true`) |
|---|---|---|---|
| `response.completed` | 누적 텍스트 반환 | 불변 | 불변 |
| `response.failed` | 항상 마스킹 에러 | **불변**(항상 에러) | **불변**(항상 에러) |
| `response.incomplete` | 항상 마스킹 에러(`failed`와 동일 취급) | **행동 변경**: 그때까지 누적된 델타 텍스트 반환(에러 아님) | 마스킹 에러(수정 전과 동일 결과) |
| 종결 이벤트 없이 스트림 끝 | 누적 텍스트 반환 | 불변 | 불변 |

ollama(`done_reason: "length"`)/omlx(`finish_reason: "length"`)의 `fail_on_truncation` 의미와
대응 확인(**§4 lens2 정밀화**: `response.incomplete` 는 절단을 포함하는 상위집합 —
content_filter 중단도 이 이벤트로 오며, 참고 구현과 동일하게 `incomplete_details` 는
모델링하지 않는다. 진짜 1:1 이 필요해지면 reason 파싱 추가가 별도 합의 대상) — "관용 경로는 잘린 결과를 그대로 쓰고, instruct 경로는 잘린 결과를 완성된 제안처럼
보이게 두지 않는다"는 정책이 세 provider에서 이제 동일하다. `response.failed`(진짜 실패)는 두
provider의 "네트워크/HTTP 에러" 취급과 마찬가지로 `fail_on_truncation`과 무관하게 항상 에러 —
d-34가 도입한 `provider_http_error`/`provider_transport_error` 에러 경로는 이번 변경에서 호출
지점도 분기도 손대지 않았다(행동 불변).

**테스트**: 기존 `incomplete_이벤트도_실패로_취급된다`를 `incomplete_이벤트는_failed와_별개의_스텝으로_분류된다`로
개명(단언을 `CodexStreamStep::Incomplete`로 교체). 신규 4건 — 메시지-없는 `incomplete` 폴백,
`response.failed`가 `fail_on_truncation=false`에서도 에러인지(회귀 방지), 관용 경로의 부분 텍스트
반환, instruct 경로의 에러화. 기존 `read_codex_completion` 호출 4곳 모두 새 파라미터를 받도록 갱신.

### 3.3 c. ollama↔omlx 트레이트 판단(#12) — **부분 통합**

계약이 지목한 5개 대상(`complete`/`complete_chat`/`send_chat_request`/`extract_chat_text`/
`build_chat_body`)을 두 provider 파일에서 나란히 대조했다.

| 대상 | 실사 결과 | 판단 |
|---|---|---|
| `build_chat_body` | ollama: `{model, messages, stream:false, options:{num_predict}}`(중첩) / omlx: `{model, messages, stream:false, max_tokens}`(평탄) — 필드 이름·중첩 구조 자체가 다름 | **보류**(억지 통합 금지 대상, 계약 §1 명시) |
| `extract_chat_text` | ollama: `OllamaChatResponse{message, error, done_reason}` / omlx: `OmlxChatResponse{choices:[{message, finish_reason}]}` — 응답 DTO 구조가 다름(단일 객체 vs 배열의 첫 원소) | **보류**(억지 통합 금지 대상, 계약 §1 명시) |
| `send_chat_request` | "바디 생성 → POST 전송 → 상태코드 분류(`provider_http_error`/`provider_transport_error`) → JSON 디코드 → `extract_chat_text` 위임" 골격이 두 provider에서 **완전히 동일**. 실제로 다른 부분(URL 조립·인증 헤더 부착)은 이미 호출부에서 `RequestBuilder` 인자로 흡수 가능 | **통합** — `providers/mod.rs`에 `post_json_and_parse<T: DeserializeOwned>(provider, request, body)` 신설(기존 `provider_http_error`/`provider_transport_error`와 같은 `pub(crate)` 가시성·같은 파일 — 기존 배치 선례). 두 provider의 `send_chat_request`가 이를 호출하도록 교체. 바디 구성(`build_chat_body`)과 응답 추출(`extract_chat_text`)은 각 provider의 함수 그대로 유지 — DTO도 전혀 손대지 않음 |
| `complete` | FIM 우선 시도 → 실패 시 `complete_chat` 폴백이라는 5줄 오케스트레이션이 두 provider에서 **바이트 단위로 동일**(자기 자신의 `complete_fim`/`complete_chat`을 호출한다는 점만 다름) | **보류** — 근거는 아래 |
| `complete_chat` | 템플릿 렌더 후 자신의 `send_chat_request`(auto-tab 예산, `fail_on_truncation=false`) 호출 — `complete`의 오케스트레이션 계층에 속함 | `complete`과 동일 판단(보류) |

`complete`/`complete_chat`을 보류한 근거: 이 5줄을 공유하려면 `AiProviderClient`(3-provider
공용 트레이트, Codex도 구현)와는 별개의 "FIM+chat 폴백" 전용 트레이트를 새로 도입해야 한다.
그런데 `codex.rs::send_responses_request`의 기존 doc 주석이 이미 "Codex has no separate FIM
endpoint... unlike Ollama Cloud/oMLX's FIM-first/chat-fallback split"라고 **의도적 설계 차이**로
명문화하고 있다 — Codex를 이 트레이트에 억지로 끼워 맞추면(더미 `complete_fim`이 항상 `None`을
반환하도록 구현) 이 문서화된 차이를 부정하게 된다. Codex를 트레이트 밖에 두더라도, 5줄 중복
제거를 위해 신설한 트레이트 정의+제네릭 함수(대략 +15~20줄)가 절감분과 상쇄되어 순 코드량이
줄지 않고, 오히려 "이 provider의 `complete()`가 정확히 무엇을 하는가"를 확인하려면 `mod.rs`(또는
신규 트레이트 파일)의 오케스트레이션 함수와 provider 파일의 `complete_fim`/`complete_chat`
구현을 오가야 해서 지금(한 파일 안에서 위→아래로 읽힘)보다 응집도가 떨어진다. "보류도 유효한
결론 — 억지 추상화가 더 나쁘다"(계약 §1)에 해당한다고 판단해 유지했다.

**결과**: `send_chat_request`의 HTTP 전송 골격만 부분 통합(ollama.rs −8줄, omlx.rs −8줄,
`providers/mod.rs` +26줄 — 대부분 판단 근거를 담은 doc). `build_chat_body`/`extract_chat_text`의
JSON 형태·DTO는 무변경, `complete`/`complete_chat`의 오케스트레이션 중복(provider당 5줄)은
의도적으로 남겨두었다.

### 3.4 검증

- `export CARGO_HOME=... RUSTUP_HOME=... PATH=...` 후 `cargo fmt --all` → `cargo fmt --all --check`
  클린(수정 없음).
- `cargo build --lib` 성공(경고 0).
- `cargo test --workspace`: 전량 green — lib 1087 + `domain_boundaries` 3 + `session_restore` 6 +
  `taide_cli` 17. AI 도메인만 필터링(`domain::ai::`) 시 100 tests pass(신규 5건: `types.rs` 1 +
  `codex.rs` 4). `tests::typescript_바인딩을_생성한다`·
  `tests::collect_commands_매크로_출력과_dispatch_테이블은_커맨드_이름_집합이_일치한다` 통과 —
  bindings 재생성과 커맨드-테이블 파리티 확인. `domain::remote::` 128 tests 전량 통과(원격 dispatch
  직렬화 무회귀).
- `cargo clippy --workspace --all-targets -- -D warnings` — 0건.
- `git diff --stat -- src/shared/api/bindings.ts` → 3.1의 델타(19 insertions, 18 deletions)만 —
  다른 커맨드·타입 변경 없음.
- `bun run typecheck` — exit 0(출력 없음).
- `bun test` — 1481 pass, 0 fail(2444 `expect()`), 149 파일.
- `bunx eslint src/shared/lib/ai/inline-completion.ts` — 0.
- 죽은 import: `grep -rn "AiInlineCompleteResponse\|AiInlineEditResponse\|AiCommitMessageResponse" src`
  → `bindings.ts`의 신규 doc 주석(생성기가 Rust doc에서 그대로 복사)과 무관한 함수명
  `sanitizeAiCommitMessageResponse` 외 0건.

**변경 파일**: `src-tauri/src/domain/ai/types.rs`·`commands.rs`·`providers/mod.rs`·
`providers/codex.rs`·`providers/ollama.rs`·`providers/omlx.rs`, `src/shared/api/bindings.ts`(생성),
`src/shared/lib/ai/inline-completion.ts`. 계약 §1 범위 외 파일(프롬프트·d-34 에러 taxonomy·Request
타입) 무접촉.

---

## 4. 검토 반영 (2026-08-25)

> 검토 wf_d572cca0-182(3렌즈 opus+xhigh): major 1·minor 2(동일 지점 중복 1)·info 7.
> 수정 wf_cf587d48-bb5(sonnet+xhigh) 6건. 원문 전문: 태스크 출력 `w8nljrvoh.output`.

- **major d37-L1-01(확정 — 적대적 생략)**: IPC 정본 문서 2건(`ipc-contract.md` :283-286·
  `data-model.md` :422·:425·:618)이 소멸한 3타입명을 유지 — "구현 시 이 문서를 먼저 갱신"
  자체 규칙 위반. **메인이 grep 으로 직접 기계 재현**(옛 타입명 6히트·AiTextResponse 0히트)
  했고 수정이 기계적 문서 갱신뿐이라 d-34 §5 선례(기계 확정)로 적대적 검증 생략. 수정 1.
- 수정 목록(§4 수정 wf): 1 정본 문서 2건 갱신(major) / 2 `AiTextResponse` doc "null=취소만"
  오서술 정정+bindings 재생성(L1-02·conv-1) / 3 `text: null` 직렬화 케이스 추가(L1-04) /
  4 dispatch `respond` 와이어 단언 테스트 1건(L1-03 보강) / 5 `Incomplete` doc 상위집합
  사실 보강(lens2-superset) / 6 `post_json_and_parse` 블록 이동 — 에러 분류 짝 인접화(conv-3).
- 기록 처분(코드 무접촉): conv-2 — 명명 실사는 `*Response` 접미사 범위였고 저장소 다수파는
  `*Result`/`*Outcome` 이나, 소멸 3타입과의 접미사 연속성을 우선해 `Response` 유지(개명은
  필요 시 동일 델타 규모로 가능함만 기록) / lens2-wiring — complete=false·instruct=true
  배선은 HTTP 클라이언트 mock 부재로 자동 테스트 미고정(분기 자체는 6건이 고정 — 한계 기록) /
  conv-4 — 범위·금지 패턴 전수 위반 0.
- §3 서술 정정 2건(dispatch 128건 커버리지 과장·incomplete "1:1" 문언)은 메인 직접 반영
  (본문 인라인 "§4 정정" 표기).
