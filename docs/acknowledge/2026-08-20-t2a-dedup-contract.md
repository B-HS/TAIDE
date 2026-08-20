# T2-A 중복 제거 배치 계약 (2026-08-20, d-28)

> 정본: 감사 `2026-08-18-architecture-audit.md` §5 T2-A 행(C2 잔여 — fileNameOf 6+3·확장자
> 추출 4·NOOP_DISPOSABLE 16·constant_time_eq 3·CodeEditor 프롭 배선 3·LSP 심볼 로딩 2·
> Location 변환 2·ANSI 토큰 3·기본값 상수 6·layout 커맨드 18) + §6.3 증분(constant_time_eq
> 4번째 사본 R6#9·ollama↔omlx provider 전체 R6#10·경로 봉쇄 원시함수 R7#5·provider match
> 3벌 R6#13·훅 JSON API 2벌 R6#15·게이트 arm 복제 R6#17·AI 응답 동형 타입 3종 R6#18·codex
> 절단 보정 R6#6).
> 사용자 goal("추천안대로 배치 계속진행") 하 HANDOFF §8-2 순번 배치. **X-A 선례 형식 —
> 항목별 실사 후 유효분만 수정·판정 근거 전수 기록**(감사는 2026-08-18 기준이라 이후 12+
> 배치로 지형 변화 — 기처리 가능성 높은 예: fileNameOf 일부(d-27 검토가 welcome 재구현 제거)·
> TS 경로 봉쇄(d-27 이 isWithinRoot 를 shared/lib/path-root 로 승격 — R7#5 는 Rust 측이라
> 별개 실사)·기본값 상수 일부(T1-B/T2-G). 전 항목 재실사가 출발점).

## 1. 범위·원칙

- 감사 열거 18항목(10+8) 전수 실사 → 판정(fixed/already-handled/invalid-claim/
  unidentifiable/deferred) → 유효분만 수정. **공통화는 2회 이상 실사용이 확인된 것만**
  (common.md 룰 — 감사 주장 수치도 재검증). 승격 위치는 FSD 규칙(shared/lib·entities)·
  기존 선례 우선.
- **대형 재설계급은 deferred**: ollama↔omlx provider 전체 파이프라인(R6#10)·layout 커맨드
  18중복·CodeEditor 프롭 배선 3 등은 실사 후 국소 공통화가 아니면 분해 배치(T2-B)와의 경계
  판단·이월 기록(반쪽 수정 금지).
- 동작 무변경 절대 조건. Rust 항목(constant_time_eq·경로 봉쇄·provider match·훈 JSON API·
  게이트 arm·AI 응답 타입·codex 보정) 포함 — 한 에이전트 순차. 표면(커맨드·이벤트·bindings·
  원격 정책) 무변경.
- 범위 외: T2-B 분해·T2-C 재구조화·T2-E/J 캠페인·FILE.RAW 무효화 선재 결함(d-25 발견 —
  별도 배치).

## 2. 실행·검증

- 구현 Workflow(sonnet+xhigh 단독): 실사 → 수정 → 신규/갱신 테스트 → cargo fmt →
  `bun run verify` + `bunx vite build` exit 0 + bindings 무변경.
- Phase E 4렌즈(실사충실성 — 판정 독립 재검증·2회 룰 / 정확성 — 공통화 전후 동작 등가·미묘
  분기 유실 / 설계 — 승격 위치·과추상화 / 계약 — 표면·경계·컨벤션) → 적대적(major 이상) →
  confirmed 수정 → 메인 2차 → 커밋 → prod 병합.

---

## 3. 구현 완료 기록 (Phase E 검토 전)

> 실사 방식: 감사 수치를 신뢰하지 않고 전 항목 현재 코드 grep·Read 재실사. 판정표는 재실사
> 결과 기준(감사 원 수치와 다를 수 있음 — 각 행에 재실사 실측치 기록).

### 3.1 판정표 (18항목)

| # | 항목 | 재실사 실측 | 판정 | 수정 요지 |
|---|------|------------|------|-----------|
| 1 | fileNameOf 6+3 | shared 원본(`relative-path.ts`) 존재, 인라인 재정의 7곳(메인 2차가 `problems-panel-container.tsx:19` 7번째 사본 추가 발견·직접 정정)(`file-group-header.tsx`·`explorer-container.tsx`·`git-panel-container.tsx`·`editor-area.tsx`·`commit-detail-panel.tsx`·`file-history-panel.tsx`) | fixed | 6곳 전부 `@shared/lib/relative-path`의 `fileNameOf` import 로 교체. `file-history-panel.tsx`는 다른 구현(split/filter/at)이었으나 이 콜사이트는 항상 파일 경로만 받아(디렉토리 트레일링 슬래시 입력 없음) 동작 등가 확인 후 통일 |
| 2 | 확장자 추출 4 | `unique-entry-name.ts`(base+ext 분리, 점 포함, 대소문자 보존 — 계약 상이)·`preview-kind.ts`·`language-from-path.ts`·`file-icon.ts`(3곳은 `dotIndex<=0?null:slice+toLowerCase` 동일 계약) | fixed | 동일 계약 3곳만 `shared/lib/file-extension.ts`(`extractFileExtension`)으로 공통화. `unique-entry-name.ts`는 계약이 달라(점 포함/대소문자 보존) 분기 보존 |
| 3 | NOOP_DISPOSABLE 16 | `lsp/command-relay.ts` + `lsp/adapters/*.ts` 15개 파일에 `{ dispose: () => {} }` 각자 선언 = 16 | fixed | `shared/lib/lsp/noop-disposable.ts` 신설, 16곳 전부 import 로 교체 |
| 4+11 | constant_time_eq 3(+4번째 R6#9) | 실사 결과 실제 사본은 1개만 남음: `domain/agent/service.rs`(자체 재구현). `domain/ide/service.rs`·`domain/remote/service.rs`·`domain/remote/commands.rs`는 이미 `infra::crypto::constant_time_eq` re-export/import 사용 중(선행 배치가 처리) | fixed(부분 already-handled) | `agent/service.rs`의 자체 구현을 삭제하고 `pub use crate::infra::crypto::constant_time_eq;` 로 교체(ide/service.rs 와 동일 패턴). 나머지 3곳은 이미 정상이라 재확인만 |
| 5 | CodeEditor 프롭 배선 3 | `editor-pane.tsx`·`untitled-pane.tsx`·`app-file-pane.tsx` 3곳에 동일한 setting-derived prop 16개(`fontFamily`~`aiAutoTabEnabled`) 배선 중복 확인. `formatOnType`/`formatOnPaste`만 편집기별 분기(untitled·app-file 은 false 고정, editor-pane 은 settings 참조) | fixed | `shared/lib/code-editor-settings.ts`(`resolveCodeEditorSettingsProps`) 신설, 3곳 모두 `{...resolveCodeEditorSettingsProps(settings)}` 스프레드로 교체. formatOnType/formatOnPaste 분기는 콜사이트에 보존(강제 통일 안 함) |
| 6 | LSP 심볼 로딩 2 | 실사 결과 3곳(`breadcrumbs-bar.tsx`·`outline-panel-container.tsx`·`command-palette.tsx`)에 동일한 waiter-loop(~20줄) 중복. 기존 `buildDocumentSymbolWaiters` 공통화는 root 후보 산출까지만 담당, 그 뒤 순회+요청+상태반영 루프는 미공통화 | fixed | `document-symbol-session-waiters.ts`에 `loadDocumentSymbolsForPath` 추가(대기+순회+cleanup 전량 캡슐화). 3곳 모두 `return loadDocumentSymbolsForPath({...})` 로 교체 |
| 7 | Location 변환 2 | 실사 결과 2곳(`adapters/definition.ts::toMonacoLocation`+`targetPathOf`, `command-relay.ts::toMonacoLocationArg`) JSDoc 포함 사실상 동일 구현. `adapters/references.ts::targetPathOf`도 동일 계약의 3번째 사본 발견 | fixed | `shared/lib/lsp/position.ts`에 `lspLocationToMonaco`·`lspLocationTargetPath` 추가, 3개 콜사이트 전부 교체 |
| 8 | ANSI 토큰 3 | 실사 결과 3곳(`xterm-theme.ts::ANSI_KEYS`·`theme-convert/types.ts::TERMINAL_ANSI_TOKENS`·`entities/theme/theme-tokens.ts::TERMINAL_TOKENS`) 16개 ANSI 이름 리터럴 중복 + `theme-live-preview.tsx`의 매직넘버(`TERMINAL_ANSI_TOKEN_COUNT=16`) slice | fixed(부분 제약으로 유지) | `xterm-theme.ts`는 `theme-convert/types.ts`의 `TERMINAL_ANSI_TOKENS` import 로 교체(중복 1건 해소). `entities/theme/theme-tokens.ts::TERMINAL_TOKENS`는 **리터럴 유지** — `domain/theme/service.rs`의 패리티 테스트가 TS 소스를 정적 regex 파싱(`'...'` 리터럴만 인식, spread 인식 불가)하므로 spread 로 바꾸면 그 가드가 무력화됨(최초 시도 시 `cargo test` 로 실제 발견·복원). `theme-live-preview.tsx`는 매직넘버 slice 를 `TERMINAL_ANSI_TOKENS` 직접 참조로 교체(매직넘버 제거는 유지) |
| 9 | 기본값 상수 6 | `DEFAULT_EDITOR_TAB_SIZE`·`DEFAULT_EDITOR_RENDER_WHITESPACE`·`DEFAULT_EDITOR_CURSOR_STYLE`·`DEFAULT_EDITOR_CURSOR_BLINKING` 4개 상수가 3개 위젯에 각각 재선언(12곳) | fixed | #5 의 `code-editor-settings.ts` 신설에 흡수 — 4개 상수 전부 그 파일로 이전, 3개 위젯 로컬 선언 제거 |
| 10 | layout 커맨드 18중복 | `domain/layout/commands.rs` 17개 명령이 `begin_mutation→read().clone()→locate_project_with_X→get_layout_mut→mutate→finish_mutation→write()` 골격 반복(locate 전략 3종 혼재: with_tab/with_pane/직접 project_id) | **deferred** | AppState 락(T1-H 배치가 전담 하드닝한 축)과 얽힌 대형 리팩터 — 클로저 기반 헬퍼 추출도 락 판정 범위 변경 위험. 계약 §1 "대형 재설계급 deferred" 예시 항목과 일치. T2-B 로 이월 |
| 12 | ollama↔omlx provider 전체 R6#10 | `complete`/`complete_chat`/`send_chat_request`/`extract_chat_text`/`build_chat_body` 함수명은 겹치나 실제 JSON 바디 필드(`options.num_predict` vs 평탄 `max_tokens`)·응답 DTO(`OllamaChatResponse.done_reason` vs `OmlxChatResponse.choices[].finish_reason`)가 상이 — 이름만 같은 구조, 로직은 provider 고유 | **deferred** | 공통 추상화가 필요하려면 요청/응답 빌더를 트레이트·콜백으로 파라미터화하는 재설계가 필요(계약 §1 명시 대형 항목). T2-B 로 이월 |
| 13 | 경로 봉쇄 원시함수 R7#5 | `infra/root_guard.rs::canonicalize_lenient`(private)와 `domain/plugin/service.rs::canonicalize_lenient` 재구현이 byte-identical. 에러 타입 분기(`Forbidden` vs `InvalidArgument`)는 `canonicalize_lenient` 자체가 아니라 상위 호출부(`ensure_within_root` vs `resolve_contribution_path`)의 선택이라 공통화해도 무변경 | fixed | `canonicalize_lenient`를 `pub(crate)`로 승격, `plugin/service.rs`의 재구현 삭제 후 `root_guard::canonicalize_lenient` 호출로 교체. 각 호출부의 에러 타입 분기는 보존 |
| 14 | provider match 3벌 R6#13 | `list_models`/`complete`/`instruct` 3개 함수가 동일한 `match provider { OllamaCloud/Codex/Omlx }` 자격증명 로딩 분기 반복(`AiProviderClient`가 `impl Future` 반환이라 object-safe 아님 — dyn 통합 불가) | fixed | `ResolvedAiProvider` enum + `resolve_provider()` 신설(자격증명 로딩 1곳화) + `impl AiProviderClient for ResolvedAiProvider`. 3개 함수는 `resolve_provider(...)?.method(...).await` 한 줄로 축소 |
| 15 | 훅 JSON API 2벌 R6#15 | `has_hook_entries_for_url`(url 계열, `MANAGED_HOOK_EVENTS` 고정)과 `has_command_hook_entries_for_command`(command 계열, `events` 파라미터 + 빈 배열 가드)가 이미 `inject_taide_managed_entries`/`remove_taide_managed_entries` 공통 프리미티브는 공유 중 — "조회(has_*)" 축만 미공통화, 빈 배열 가드는 events 파라미터가 정적 상수가 아닐 때만 의미 있는 실질 차이 | fixed | `has_managed_entries_matching(root, events, matches_handler)` 신설(빈 배열 가드 보존 — `MANAGED_HOOK_EVENTS`는 항상 비어있지 않아 가드 추가해도 무변경). 두 함수 모두 이를 위임 |
| 16 | 게이트 arm 복제 R6#17 | `agent_hooks_install`의 `Project`/`User` 두 match arm이 동일한 `agent_hooks_enabled` 설정 게이트를 각자 선두에 중복 | fixed | 게이트를 `match scope` 앞으로 호이스트(scope 무관 조건이므로 순서 무변경) |
| 17 | AI 응답 동형 타입 3종 R6#18 | `AiInlineCompleteResponse`/`AiInlineEditResponse`/`AiCommitMessageResponse` 3개 구조체가 `{request_id, text: Option<String>}`로 동형이나 각각 `#[derive(... Type)]`(specta)로 독립 명명 — TS `bindings.ts`에 3개 별도 타입으로 생성되고 `shared/lib/ai/inline-completion.ts`가 `AiInlineCompleteResponse`를 이름으로 직접 import | **deferred** | 통합(단일 타입/별칭)은 specta 생성 타입명이 사라지거나 바뀌어 `bindings.ts` 표면이 변경됨 — 계약 §1 "표면(bindings) 무변경" 정면 위반. bindings 변경을 전제로 하는 결정은 T2-A 범위 밖(프론트 소비처 갱신 포함 별도 승인 필요) |
| 18 | codex 절단 보정 R6#6 | Ollama/OMLX는 `fail_on_truncation` 플래그로 auto-tab(관용)과 instruct(에러화) 경로를 분기하나 Codex 는 SSE `response.incomplete`를 항상 `Failed`(에러)로 처리 — "미적용"이 아니라 상이한 스트리밍 프로토콜(SSE) 위에서 그 구분 자체가 없음 | **deferred** | 추가하려면 `read_codex_completion`/`classify_codex_event`에 `fail_on_truncation` 전파 + `response.incomplete`를 관용 경로에서 부분 텍스트 반환으로 바꿔야 함 — 이는 현재 항상-에러 동작을 바꾸는 **행동 변경**이라 "동작 무변경" 원칙과 정면 충돌. #12(ollama↔omlx 파이프라인 통합)와 묶어 T2-B 에서 제품 판단과 함께 처리 |

### 3.2 검증

- `bun run typecheck` / `bunx eslint .`(사전 존재 warning 6건 외 0 error) / `prettier --check .` /
  `bun test`(1431 pass) / `cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D
  warnings`(0) / `cargo test`(1068 lib + 3 domain_boundaries + 6 session_restore + 17 cli, 전부
  pass) — `bun run verify` 전체 exit 0.
- `bunx vite build` exit 0.
- `git diff --stat src/shared/api/bindings.ts` 비어있음 — bindings 무변경 확인.
- 발견 즉시 복원 사례: ANSI 토큰(#8) 1차 시도(entities/theme-tokens.ts 를 spread 로 파생)가
  `domain::theme::service::tests::테마_토큰_목록은_rust와_theme_tokens_ts에서_일치한다` 를
  깨뜨림 → 리터럴로 원복하고 사유를 해당 파일 JSDoc 에 기록.

### 3.3 신규 테스트

- `src/shared/lib/file-extension.test.ts`(신규) — `extractFileExtension` 케이스 4종.
- `src/shared/lib/code-editor-settings.test.ts`(신규) — `resolveCodeEditorSettingsProps` 기본값/오버라이드 케이스 3종.
- 기존 `src/shared/lib/lsp/*.test.ts`(320 tests) 전부 재통과 확인(NOOP_DISPOSABLE·Location 변환·문서 심볼 로딩·서버 필터 리팩터 대상).
- Rust 기존 테스트(`infra::root_guard::tests::*`·`infra::crypto::tests::*`·`domain::theme::service::tests::*`) 전부 재통과 확인 — 신규 Rust 테스트는 추가하지 않음(순수 구조 위임이라 기존 커버리지로 충분 판단).

### 3.4 이월 (T2-B)

1. layout 커맨드 18중복(#10) — AppState 락 경계와 얽힘.
2. ollama↔omlx provider 파이프라인 통합(#12) + codex 절단 보정(#18) — 공통 추상화 설계 + 행동
   변경 제품 결정 필요, 두 항목을 하나의 배치로 함께 처리 권장.
3. AI 응답 동형 타입 3종(#17) — bindings.ts 표면 변경을 전제하므로 별도 승인 필요.
