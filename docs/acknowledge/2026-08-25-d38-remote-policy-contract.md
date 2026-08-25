# d-38 — 원격 dispatch 정책(키링 게이트) 계약 (2026-08-25)

> 정본: `2026-08-25-post-batch-user-decisions.md` §3(객관식 확정). 이월 원문 감사 R3#7·
> `2026-08-19-audit-t1k-default-deny-contract.md` §1.3(정책 변경 없이 이전 — "후속 결정 시
> 목록 이동 1줄"이 되도록 만든 구조의 그 후속). 선행: d-37 완료 후 착수(Rust 순차).

## 0. 착수 전 메인 실사 (2026-08-25)

- `dispatch.rs` 허용 목록에 `ai_token_status`(:171)·`ai_set_token`(:172)·`ai_clear_token`
  (:173) 실재. GitHub PAT(sync 도메인) set/clear 계열 커맨드는 구현이 전수 실사(감사 R3#7:
  "키링 5계정 중 원격 거부 1계정뿐 — AI 토큰 3 + GitHub PAT 원격 변경 가능").
- `ide_publish_diagnostics`/`ide_notify_at_mention`(:167·:170) ALLOWED — **허용 유지 확정**.
- `agent_hooks_uninstall` User 스코프 원격 허용(비대칭) — **유지 확정**(제거는 백도어를 없애는
  방향이라는 논리).

## 1. 범위 (사용자 결정 2026-08-25)

- **a. 키링 변경 원격 거부 전환(코드)**: `ai_set_token`·`ai_clear_token` + GitHub PAT
  변경 계열(실사로 커맨드명 확정)을 거부 목록으로 이동. 거부 정책 variant 는 기존
  `RemoteDenialPolicy` 에서 적합한 것을 재사용하거나 신설(신설 시 d-34 로케일 규약대로
  `error.remote.denied<Variant>` 키 3언어+`MESSAGE_NAMESPACES` 동기+파리티). **상태 조회**
  (`ai_token_status` 등 존재 여부만 반환)는 허용 유지. 거부 테스트는 기존
  `assert_forbidden_denial` 관행.
- **b. 명문화(문서)**: `ipc-contract.md` 원격 dispatch 정책 절에 ① 진단·알림 주입 허용 =
  검토 완료·의도된 허용 ② 훅 install/uninstall 비대칭 = 의도된 설계(제거는 위험 감소 방향)
  ③ 키링 변경 거부 전환을 각각 결정 일자와 함께 기록.
- 검증: cargo 전량 + dispatch 파리티 테스트 + 로케일 파리티(신설 키 시) + bindings 무변경.
- 범위 외: QoS 캠페인(사용자 기각 — 착수 금지)·다른 커맨드 정책 변경·세션 인증 방식 변경.

## 2. 실행·검토

- 구현: Rust 1 에이전트(sonnet+xhigh, d-37 종료 후) — §3 기록. 검토 2렌즈(opus+xhigh —
  ① 보안·정책 완전성(목록 이동 누락 0·프론트 원격 경로 소비처 영향) ② 컨벤션·로케일 파리티)
  + major 적대적 → 수정 → 메인 2차. 소형 배치라 렌즈 2 로 축소하되 보안 렌즈는 유지.

---

## 3. 구현 완료 기록 (구현 에이전트가 기록)

> 구현: Rust 1 에이전트(sonnet, 2026-08-25). 범위 §1 a·b 완료. 검증 게이트 전부 green(§4).

### 3.1 실사 — 키링 5계정 × 원격 변경 커맨드 전수

`infra::secret::SecretAccount` 5계정을 기준으로, 각 계정의 값을 쓰거나(`set`) 지우는(`delete`)
커맨드를 `grep -rn "SecretAccount::"` 로 전수 추적했다(조회 `get` 은 제외 — 계정별 결과는
`src-tauri/src/domain/ai/service.rs`·`src-tauri/src/domain/sync/commands.rs`·
`src-tauri/src/domain/remote/commands.rs` 3파일에 전부 모여 있다):

| 계정(`SecretAccount`) | 쓰기 커맨드 | 지우기 커맨드 | 원격 dispatch 처분(전) | 처분(후) |
|---|---|---|---|---|
| `AiOllamaCloud` | `ai_set_token(provider: 'ollamaCloud')` | `ai_clear_token(provider: 'ollamaCloud')` | 허용 | **거부**(신규) |
| `AiCodex` | `ai_set_token(provider: 'codex')` | `ai_clear_token(provider: 'codex')` | 허용 | **거부**(신규) |
| `AiOmlx` | `ai_set_token(provider: 'omlx')` | `ai_clear_token(provider: 'omlx')` | 허용 | **거부**(신규) |
| `GithubSync` | `sync_connect(pat)` | `sync_disconnect()` | 허용 | **거부**(신규) |
| `RemoteAccess` | `remote_set_password(password)` | `remote_clear_password()` | 거부(`SelfAccessExpansion`, 기존) | 거부(변경 없음) |

`ai_set_token`/`ai_clear_token` 은 커맨드 1개씩이 `provider` 인자로 3계정을 함께 담당하므로,
실제로 이동한 커맨드는 4개(`ai_set_token`·`ai_clear_token`·`sync_connect`·`sync_disconnect`)다.
**조회/변경 구분 실물 확인**: `ai_token_status`(`ai::commands::ai_token_status` → `secret.get(...)
.is_some()` 3계정 boolean 만 반환, 토큰 값 자체는 응답에 없음)와 `sync_status`(`sync::commands::
sync_status` → `secret.get(SecretAccount::GithubSync)?.is_some()` 로 `connected` 만 계산, 값은
게이트 통과용 지역 변수로만 쓰이고 응답 `SyncStatus` 구조체 어디에도 담기지 않음) 둘 다 존재
여부만 반환하며 값을 `set`/`delete` 하는 코드 경로가 없다 — 계약 §1 대로 허용에 남겼다.

### 3.2 variant 선택/신설 근거

기존 8개 `RemoteDenialPolicy` variant(`SelfAccessExpansion`/`UnreachableDesktopWindow`/
`LocalFilesystemEscape`/`InstallOrProcessExecution`/`DesktopCliInterception`/
`DesktopExitControl`/`SharedSingletonStateRace`/`LocalProjectHistoryExposure`)를 전부 검토했으나
재사용에 정확히 들어맞는 것이 없었다:

- `SelfAccessExpansion` 은 "원격 접속 게이트 자체"(비밀번호·링크)의 자가 확장만 다룬다 — AI/GitHub
  자격증명은 앱의 원격 접속 게이트가 아니라 제3자 서비스 인증 수단이라 이 카테고리가 아니다.
- `DesktopCliInterception` 이 뜻은 가장 가까웠다(세션을 넘어 남는 백도어)지만, 대상이 "CLI 훅
  파일" 로 고정 서술돼 있어 "키링 자격증명" 을 그 이름·문서에 억지로 끼워넣으면 독자가 헷갈린다.
- 나머지 5개는 로컬 창/파일시스템/설치/종료제어/싱글톤 경합/프로젝트 이력 노출로 전혀 다른 위협
  모델이다.

**신설**: `RemoteDenialPolicy::CredentialStoreTampering` — d-34 규약대로 처리했다.

- enum variant: `src-tauri/src/domain/remote/dispatch.rs` `RemoteDenialPolicy` 에
  `LocalProjectHistoryExposure` 다음·`Unclassified` 앞에 추가(최근 추가 항목이 sentinel 앞에
  쌓이는 기존 순서 유지). 영문 doc 주석에 대상 4커맨드·5계정 중 4/5 커버·`RemoteAccess` 는
  `SelfAccessExpansion` 로 이미 커버됨·위협 시나리오(토큰 바꿔치기로 AI/gist 트래픽이 공격자 계정에
  흐름, 또는 지속적 DoS)·상태 조회는 대상 아님을 전부 서술.
- `denial_error()` match arm 1개 추가(같은 위치, `LocalProjectHistoryExposure` 다음).
- `REMOTE_DENIED_COMMANDS` 에 4행 추가(`project_list_recent` 다음).
- `REMOTE_ALLOWED_COMMANDS` 에서 4개 제거, `dispatch()` 의 `match` 에서 대응 arm 4개 제거(허용
  테이블과 실제 arm 집합이 항상 일치해야 하는 기존 불변식 — 파이썬으로 직접 재계산해 확인:
  ALLOWED 156 = match arm 156 = 156, 교집합 0, 합집합 = 전체 180종).

### 3.3 로케일 키 — `error.remote.deniedCredentialStoreTampering`

d-34 규약(§0 실사) 그대로 4곳 동기:

1. `src-tauri/resources/locales/en.json` — `"A remote session cannot change a stored credential
   (AI provider token, GitHub personal access token): {{command}}"`
2. `src-tauri/resources/locales/ko.json` — `"원격 세션에서는 저장된 자격 증명(AI 공급자
   토큰·GitHub 개인 액세스 토큰)을 변경할 수 없습니다: {{command}}"`(완결 문장 하우스 스타일, 조사
   공백 분리는 기존 키에 로마자 고유명사가 없어 해당 없음 — "AI"·"GitHub" 는 붙여 쓰는 기존
   키들의 표기를 그대로 따름, 예: `error.remote.deniedDesktopCliInterception` 의 "CLI")
3. `src-tauri/resources/locales/ja.json` — `"リモートセッションは保存された認証情報(AI
   プロバイダーのトークン・GitHub 個人アクセストークン)を変更できません: {{command}}"`(다른 ja
   키처럼 "keyring"류 외래어를 가타카나로 번역 — "認証情報"로 대체)
4. `src-tauri/src/domain/locale/service.rs` — `MESSAGE_NAMESPACES` 의 `error` 네임스페이스 배열,
   `remote.channelArgRequired` 다음·`remote.deniedDesktopCliInterception` 앞에 알파벳 순
   삽입(`Credential` < `Desktop`)

세 언어 모두 "GitHub PAT" 대신 **"GitHub personal access token"/"GitHub 개인 액세스
토큰"/"GitHub 個人アクセストークン"** 을 썼다 — `settings.syncTokenPlaceholder` 키가 이미 이
표현으로 3언어 정착돼 있어(`en`: "Paste personal access token", `ko`: "개인 액세스 토큰
붙여넣기", `ja`: "個人アクセストークンを貼り付け") 새 용어를 만들지 않고 그대로 재사용했다.
`dispatch.rs` 의 `RemoteDenialPolicy::denial_error()` 영문 fallback 도 동일하게 맞췄다(코드
내부 doc 주석·이 계약 문서의 산문은 개발자용이라 "GitHub PAT" 축약을 그대로 둔다 — 사용자에게
실제로 보이는 로케일 메시지·fallback 만 기존 UI 용어에 맞춘다).

3개 로케일 파일 모두 전체가 알파벳 순 정렬돼 있어(다른 네임스페이스도 포함) 그 규칙을 그대로
따랐다. 파리티 테스트(`내장_3종_로케일은_같은_키_집합을_가진다`·
`en_메시지의_모든_키는_required_message_keys에_포함된다`) green.

### 3.4 프론트 소비처 영향 실사 (범위 외 — 확인만, UI 미변경)

`src/entities/ai/ai.ipc.ts`(`setAiToken`/`clearAiToken`) · `src/entities/sync/sync.ipc.ts`
(`connectSync`/`disconnectSync`) 가 각각 `unwrapResult(commands.ai*/sync*(...))` 로 바인딩을
그대로 호출하고, `src/widgets/settings-view/settings-ai-section.tsx`·`settings-sync-section.tsx`
가 이를 `useMutation` 으로 소비한다 — 원격 미러 세션에서도 설정 화면 자체가 별도로 가드되어
있지 않아 이 4개 커맨드는 그대로 호출된다. 코드 추적 결과:

- **와이어 레벨은 d-34 로케일 경로가 이미 정상 배선돼 있다** — `unwrapResult`(`src/shared/api/
  unwrap-result.ts`)가 백엔드 `AppError::Localized` 를 `IpcError{ localeKey, localeArgs }` 로
  정규화하고, `describeIpcError`/`useIpcErrorMessage`(`src/shared/lib/ipc-error-message.ts`)가
  `localeKey` 를 `i18next.t(...)` 로 해석해 토스트/인라인 텍스트에 쓸 수 있게 되어 있다(이미 다른
  거부 케이스들이 쓰는 공통 경로 — 이번 배치가 새로 만든 배선이 아니다).
- **그런데 이 4개 커맨드의 실제 UI 호출부는 그 공통 경로를 쓰지 않는다**:
  - `settings-ai-section.tsx`: `setAiToken(..., { onError: () => toast.error(t('settings.
    aiTokenSaveFailed')) })` — 원인과 무관한 고정 문구. `clearAiToken` 호출(`handleClearAiToken`)
    에는 **`onError` 자체가 없어** 원격 세션에서 거부되면 토스트조차 뜨지 않고 완전히 조용히
    실패한다.
  - `settings-sync-section.tsx`: `connectSync`/`disconnectSync` 모두 `onError: () => toast.error(
    t('settings.syncConnectFailed'/'syncDisconnectFailed'))` — 역시 원인과 무관한 고정 문구.
  - 결과적으로 원격 세션에서 이 4개 버튼을 누르면(토큰 저장/삭제, GitHub 연결/해제) 서버는
    Forbidden(d-34 `Localized`, 로케일 키 `error.remote.deniedCredentialStoreTampering`)을 정확히
    반환하지만, 화면에는 "AI 토큰 저장 실패"/"동기화 연결 실패"/"동기화 연결 해제 실패" 같은 일반
    실패 문구만(혹은 clear 의 경우 아무 것도) 보인다 — 원인이 "원격에서는 금지" 라는 사실이
    사용자에게 전달되지 않는다.
- **범위 외 확정**: 이 UI 갭(고정 문구 대신 `describeIpcError`/`useIpcErrorMessage` 사용, `clear`
  에 `onError` 추가)은 계약 §1 이 "확인만" 을 지시했으므로 이번 배치에서 고치지 않았다 — 후속
  UX 정리 대상으로 남긴다.

### 3.5 변경 파일

- `src-tauri/src/domain/remote/dispatch.rs` — `RemoteDenialPolicy::CredentialStoreTampering`
  신설(variant + `denial_error` arm), `REMOTE_ALLOWED_COMMANDS`(160→156)/`REMOTE_DENIED_COMMANDS`
  (20→24) 갱신, `dispatch()` match arm 4개 제거, 카운트 doc 주석 2곳 정합(159→156 entries 등,
  기존에 158/159 로 1개 어긋나 있던 것도 이번에 실측치로 바로잡음), 테스트 2건 추가.
- `src-tauri/resources/locales/{en,ko,ja}.json` — `error.remote.deniedCredentialStoreTampering`
  1키씩 추가.
- `src-tauri/src/domain/locale/service.rs` — `MESSAGE_NAMESPACES` 에 같은 키 추가.
- `docs/ipc-contract.md` — "원격 dispatch 정책" 절에 d-38 확정 블록(①②③) 추가, 허용/거부
  카운트(160/20→156/24) 전수 갱신, 거부 테이블에 `CredentialStoreTampering` 행 추가, `ai_*`/
  `sync_*` 허용 종수 서술 정정, `agent_hooks` 비대칭·"신규 커맨드 등재 규칙" 문단의 "후속 결정
  이월" 문구를 "d-38 에서 확정" 으로 갱신, `ai` 도메인 절의 "ai 8종 전부 허용" 오기 정정(직접
  모순되는 문장이라 §1 b 범위로 함께 수정).

## 4. 검증 결과

`export CARGO_HOME="$HOME/development/rust/cargo"; export RUSTUP_HOME="$HOME/development/rust/rustup";
export PATH="$CARGO_HOME/bin:$PATH"` 로 실행.

- `cargo fmt --all` → `cargo fmt --all -- --check`: 클린(diff 0).
- `cargo test --workspace`: **1116 passed / 0 failed**(`taide` lib 1090 + `domain_boundaries` 3 +
  `session_restore` 6 + `taide_cli` 17, doc-test 0) — `dispatch` 파리티 전량(`허용_테이블과_거부_
  테이블은_전체_커맨드를_교집합_없이_정확히_분할한다`·`허용_테이블의_모든_커맨드는_실제_match_arm_
  을_가진다`·`bindings와_dispatch_테이블은_커맨드_이름_집합이_일치한다` 포함) 및 로케일 파리티
  전량(`내장_3종_로케일은_같은_키_집합을_가진다`·`en_메시지의_모든_키는_required_message_keys에_
  포함된다`) green. dispatch.rs 테스트는 31 → **33**(+2, §3.1/§3.5).
- `cargo clippy --workspace --all-targets -- -D warnings`: **0 warnings**.
- `git diff src/shared/api/bindings.ts`: 비어 있지 않지만(39줄) **d-38 관련 토큰(`ai_set_token`/
  `ai_clear_token`/`sync_connect`/`sync_disconnect`/`CredentialStoreTampering`/`remote`)이 diff
  안에 0개**임을 `grep` 으로 직접 확인 — 실제 diff 내용은 d-37(`AiInlineCompleteResponse`/
  `AiInlineEditResponse`/`AiCommitMessageResponse` → `AiTextResponse` 통합)의 세션 시작 전부터
  있던 미커밋 변경으로, `cargo test` 가 구동하는 바인딩 생성 테스트(`typescript_바인딩을_생성한다`)
  가 이미 존재하던 Rust 소스와 디스크상 파일을 다시 동기화시킨 것이다 — dispatch 게이트 테이블은
  specta 등록·커맨드 시그니처에 관여하지 않으므로 이 배치의 4커맨드 이동은 bindings 표면에
  구조적으로 나타날 수 없다(§1 계약대로 무변경 확인).
- `bun test`: **1481 pass / 0 fail**(2444 `expect()`, 149 파일) — 무회귀.

---

## 5. 검토 반영 (2026-08-25)

> 검토 wf_043efb11-4be(2렌즈 opus+xhigh): major 1(2렌즈 수렴)·minor 4·info 2. 수정
> wf_3438a2b2-c73(sonnet+xhigh) 6건. 원문 전문: 태스크 출력 `wy5eszt2y.output`.

- **major D38-L2-01/SEC-01(확정 — 적대적 생략)**: `docs/features/ai.md` §8 이 "ai_set_token
  포함 … 거부 arm 없음"으로 d-38 전환의 정반대를 서술(구현이 ipc-contract 에 "현행화 필요"
  포인터만 남김). 2렌즈 독립 수렴 + 메인 직접 재현(:236-239 원문 확인) + 수정이 기계적 문서
  정정 — d-34 §5 선례로 적대적 생략. 수정 1(ai.md 정정+stale 포인터 소거).
- 수정 목록: 2 신규 거부 테스트에 로케일 키·args 단언 추가(SEC-02 — variant 오매핑 형해화
  방지)+조회 2종 허용 소속 단언 / 3 **검토 반영 범위 확장**: 원격 거부 무피드백 실패는 이번
  배치가 신설한 경로(SEC-04)이므로 4개 호출부(setAiToken·clearAiToken·connectSync·
  disconnectSync)의 onError 를 d-34 공통 경로(`describeIpcError` || 고정 fallback)로 교체 —
  clearAiToken 은 onError 신설(전용 fallback `settings.aiTokenClearFailed` 3언어 신설 —
  카탈로그의 액션별 `*Failed` 쌍 관행 준거, 로케일 918키×3 파리티 유지) / 4 variant doc 에
  pty_spawn 능력 상한 단서(SEC-05 — 형제 variant 기조 정합) / 5 ko "공급자"→"프로바이더"
  (L2-02 — settings.aiProviderLabel 정착 용어) / 6 ipc-contract 에 SEC-03 한정 문구.
- §3.3 의 ko 인용문도 "프로바이더" 로 정정해 읽는다(수정 5 반영 — 원문 유지, 이 절이 정정 기록).

## 6. 이월 — `ai_omlx_base_url` 원격 스트립 편입 여부 (SEC-03, 사용자 결정 필요)

- **잔존 경로**: 키링 write/delete 는 닫혔으나, OMLX 는 `Settings.ai_omlx_base_url`(원격 허용
  `settings_update`·`sync_download` 로 변경 가능)이 저장된 API 키의 **전송 대상 호스트**를
  결정한다 — 원격 세션이 base_url 을 공격자 호스트로 바꾸면 이후 데스크톱의 OMLX 트래픽
  (Authorization Bearer 포함)이 그쪽으로 흐르고 settings.json 에 영속. ollama/codex 는 호스트
  하드코딩이라 해당 없음.
- **선례**: `shell_override` 가 동일한 "세션 초과 지속성" 근거로 이미
  `strip_remote_gated_settings_patch`/`strip_remote_gated_settings` 스트립 대상.
- **결정 갈래**: ① `ai_omlx_base_url` 을 스트립 목록에 편입(선례 정합 — 권장) ② 현상 유지
  (pty_spawn 능력 상한 논리로 수용). ipc-contract 에는 한정 문구로 현황 명시(수정 6).

> [확정 2026-08-25] 사용자 결정: **① 스트립 편입 채택** — d-41 배치로 착수(PROCESS 큐 등재). ②(현상 유지)는 기각.
