# QA6 후속 1차 구현 계약 (2026-08-12)

> 사용자 실기 보고 5건(단축키 확장·OMLX provider·IME 버그 2건·툴팁 전수)에 대한 정찰
> (wf_830fe067-9a9: fable+high 디버그 1 + opus+medium 정찰 3, 핵심 주장 전부 메인 실물 재검증)
> 결과와 사용자 결정을 확정한 계약. 직전 계약: `2026-08-12-w7-textmate-contract.md`.

## 1. 사용자 결정 (2026-08-12 확정)

| # | 결정 지점 | 선택 |
|---|-----------|------|
| 1 | IME 버그 발생 표면 | monaco 에디터 한정 (터미널·일반 input 정상) |
| 2 | W7 CSP 항목 | 하이라이팅 실기 동작 확인, devtools 콘솔 무오류 확인은 잔여 |
| 3 | 자모 분리(버그 3) 처리 | **monaco 최신 버전 조사** 후 업그레이드 검토, 없으면 backlog |
| 4 | 단축키 범위 | **전 커맨드 노출 + 별도 모달**(VS Code 식 검색·재바인딩) — 추천안 전부 채택 |
| 5 | OMLX | 리서치 위임 → jundot/omlx 특정. **auto-tab(FIM)은 반드시 동작해야 함** — chat-only 단순화 기각, 클라이언트 측 FIM 센티널 조립로 구현 |
| 6 | 툴팁/i18n | 감사 결과 ①~⑥ 전부 수정 |

## 2. 확정 사실 (메인 실물 재검증 완료)

### 2.1 IME 버그 2건 (fable+high 디버그)

- **버그 4(상하 떨림)**: 원인 = `MONOSPACE_FONT_FALLBACK_STACK` 에 한글 글리프 폰트 부재.
  monaco 는 한글 음절 조합마다 라인 위에 visible textarea 오버레이를 토글하는데
  (textAreaEditContext.js `_render`), textarea/뷰 라인 간 한글 폴백 폰트 해석이 비결정적.
  수정 = 스택에 `"Apple SD Gothic Neo"` 추가(generic monospace 앞). **수정 완료** — 실기 확인 대기.
- **버그 3(자모 분리)**: 앱 코드 무혐의 — inline-completion 은 monaco 가 조합 중 ghost text 억제
  (`inlineCompletionsModel.js` inComposition), keymap 은 modifier 필수라 매칭 불가, 조합 중
  모델 쓰기 경로 없음(전역 grep). 원인 = WKWebView↔monaco 숨김 textarea 계층: monaco 는 macOS
  에서 유휴 시 커서 앞 단어를 textarea 에 시드(textAreaEditContext.js — 메인이 주석 원문 확인)
  하는 유일한 표면이 에디터이고, IME 전환 직후 첫 조합의 `compositionupdate.data` 자체가 분리
  자모로 도착. monaco 는 매 update 마다 직전 조합 전체를 치환하므로 "다음 글자에 해소"와 정합.
  앱 레벨 수정 = 조합 이벤트 가로채기(hack)뿐 → **수정하지 않고 monaco 신버전 조사로 대응**.
  - 조사 결과(2026-08-12, 정밀 보정 포함): monaco 0.56.0 이 최신 — 상위 버전 부재, vscode main
    에도 수정 0건 → **backlog 확정**. 시드 분기 보정: TAIDE 기본은 "커서 앞 단어"가 아니라
    `SimplePagedScreenReaderStrategy`(페이지 단위) — `accessibilitySupport: 'off'` 로 단어 단위
    축소 가능(공개 API, 미검증 가설). 상세: `docs/bug/2026-08-12-editor-korean-ime.md`.

### 2.2 단축키 (opus 정찰, 하중 지점 메인 대조)

- 커맨드 레지스트리 20개(DEFAULT 18 + SYNC 2) + 커맨드 없는 키맵 액션 3 = 모달 대상 23행.
  미노출 5개의 원인은 바인딩 모델이 고정 union `KeymapActionId` 로 키잉된 것.
- `KeymapOverrideEntry` 파서는 형태만 검사(`isKeymapOverrideEntry`) → actionId 를 string
  (commandId)으로 확장해도 저장 포맷 하위호환. Rust 는 `keymap_overrides: Option<String>`
  JSON 문자열 통과라 **무접촉**.
- 실결함 확인 2건: ① 캡처 중 전역 키맵 선점(useGlobalKeymap 이 window capture 단계 —
  캡처 중 mod+S 누르면 저장이 실행됨) ② `formatKeymapShortcut` 표기(구분자 없음·비mac Ctrl 중복).

### 2.3 OMLX (opus 리서치, 저장소 실재 메인 확인)

- **jundot/omlx** (Apache-2.0, Apple Silicon MLX 서버). 기본 `http://localhost:8000/v1`,
  API key **선택**(Bearer/x-api-key), `/v1/models`·`/v1/chat/completions`·`/v1/completions`
  (SSE 지원). **`/v1/completions` 에 `suffix` 필드 없음** — 저장소 전체에 fim/infill 0건.
- TAIDE 선례: base URL 사용자 입력 provider 없음(Ollama/Codex 는 상수 base). 토큰은
  keyring(`SecretAccount`), Codex 는 저장 전 whoami 검증. `AI_AUTO_TAB_PROVIDERS`
  화이트리스트 누락 시 설정이 조용히 drop(`sanitize_optional_enum`).
- **프롬프트 추상화 점검(메인 직접)**: `prompt::render` 가 임의 템플릿에 `{prefix}/{suffix}/
  {language}/{filePath}` 치환 → FIM 센티널 조립을 기존 `AiFimPromptTemplate` 스키마 변경 없이
  표현 가능. Ollama 의 "FIM 우선→chat 폴백" 계약을 그대로 재사용한다.

### 2.4 툴팁/i18n (opus 전수 감사, 재렌더 결함 메인 실물 검증)

- **누락 i18n 키 0건**(en/ko/ja 각 499, 편차 0). raw key 노출의 진짜 원인은 재렌더 결함:
  react-i18next 기본 `bindI18nStore: ''` → `addResourceBundle` 의 store 이벤트를 아무도 구독
  안 함 + `i18n.ts` 가 `lng:'en'` 초기화라 en 사용자는 `changeLanguage` 미호출 →
  **리소스 도착 후에도 재렌더 0회**(메인이 defaults.js·useTranslation.js 실물 확인).
- 부수: git-panel 하드코딩 영문 15건, 툴팁 부재 아이콘 컨트롤 37건(aria 도 없는 것 3건),
  native title= 대용 8곳, `{title}` 단일중괄호 보간 2키(수동 replace 우회),
  `MESSAGE_NAMESPACES` ↔ 실문안 87키 불일치(커스텀 팩 검증 구멍), dead 키 후보 ~55.

## 3. 확정 설계

### 3.1 단축키 모달 (전부 추천안)

- **데이터 모델 A안**: `KeymapOverrideEntry.actionId: string`(commandId 포함)으로 확장.
  런타임 바인딩 목록 = 커맨드 레지스트리 ∪ APP_KEYMAP (`shared/lib/keybinding-catalog.ts` 신설).
  매칭 시 커맨드 실행 경로 추가하되 기존 4곳 액션 핸들러 병존 — keymapId 있는 커맨드는 기존
  경로 우선(중복 실행 방지).
- **unbind**: override `{ actionId, key: '', mods: [] }` 센티널 + `matchesKeymapEntry` 상단
  빈 key 즉시 false.
- **UX**: Dialog(max-w-3xl) + ul + fuzzyFilter(커맨드명·키 양방향 검색, cmdk 미사용),
  행별 변경/기본값 복원/해제, 충돌은 저장 허용 + 행 경고 + 상대 해제 제안 + 충돌 필터 칩,
  미할당 표기·필터. 진입점 = 설정 KEYMAP 섹션 버튼 + 신규 커맨드 `keybindings.open`
  (bridge 로 app 상시 마운트 모달 오픈 — explorer-panel-bridge 선례).
- **캡처 중 전역 키맵 비활성화**(선점 결함 수정), 캡처 중 Escape 는 stopPropagation(Dialog
  닫힘 방지). `formatKeymapShortcut` 수정: mac 심볼 연접 유지, 비mac `Ctrl+Shift+G` 형식 +
  mod/ctrl 중복 해소. 커맨드 등록을 command-palette 모듈 사이드이펙트 → app 부트스트랩 이동.
  `categoryKey` 전 커맨드 부여("카테고리: 이름" 렌더).
- chord·when 은 backlog 유지(dead field `when` 은 모달 비노출).

### 3.2 OMLX provider

- `AiProviderId::Omlx`("omlx"), `AiTokenStatus.omlx`, `SecretAccount::AiOmlx`("ai-omlx").
- **base URL**: `Settings.ai_omlx_base_url: Option<String>`(평문 — 비밀 아님), 기본 후보
  `http://localhost:8000`(placeholder), `sanitize_optional_url`(http/https 만) 신설,
  `AI_AUTO_TAB_PROVIDERS` 에 "omlx". **configured = base URL 설정됨**(API key 무관 —
  `token_status.omlx = base_url.is_some()`). API key 는 선택 입력 시 keyring 저장.
- 저장 시 서버 다운이어도 허용(로컬 서버 특성), 모델 조회 실패는 UI 에러 표시. 모델 전량 노출.
- **완성(auto-tab) — FIM 필수 동작**: `providers/omlx.rs` 에 Ollama 와 동일한 FIM 우선→chat
  폴백 계약. FIM = `POST /v1/completions` + **모델 패밀리별 센티널 테이블로 단일 prompt 조립**
  (model id 소문자 부분문자열 매칭):
  qwen(-coder) / codegemma → `<|fim_prefix|>P<|fim_suffix|>S<|fim_middle|>`,
  deepseek-coder → `<｜fim▁begin｜>P<｜fim▁hole｜>S<｜fim▁end｜>`,
  starcoder → `<fim_prefix>P<fim_suffix>S<fim_middle>`,
  codellama → `<PRE> P <SUF>S <MID>`, codestral → `[SUFFIX]S[PREFIX]P` —
  **구현 시 각 패밀리 공식 모델 카드로 센티널·stop 토큰 재검증 필수**(Continue.dev 오토컴플리트
  템플릿 대조). 미지 패밀리는 FIM 건너뛰고 chat 폴백. 렌더는 기존 `prompt::render` 재사용
  (사용자 템플릿 오버라이드 유지), `stream:false`, `max_tokens` 상수, `stop` = 패밀리 stop +
  템플릿 stop. 에러는 전부 `mask_provider_error` 경유.
- 신규 IPC 0개(기존 6개 시그니처 확장 — `ai_list_models`·`ai_set_token` 에 State<AppState>).
  디스패치 137종 유지, bindings 는 타입 변경분 재생성.
- 프론트: `AI_PROVIDER_OPTIONS` 추가, OMLX 전용 설정 행(base URL + 선택 API key),
  `resolveAiInlineCompletionConfig` 는 `tokenStatus.omlx` 키 일치로 최소 수정.

### 3.3 툴팁/i18n ①~⑥

1. 재렌더 결함: `i18n.ts` init 에 `react: { bindI18nStore: 'added removed' }` +
   `locale-provider` 로딩 게이트(`if (!locale) return null`) — 첫 렌더 raw key flash 제거.
2. git-panel 15건 i18n 화 — 기존 dead 키(git.stage/unstage 등) 문안 맞으면 재사용, 아니면
   신규 10키(mergeChanges·stagedChanges·changes·unstageAll·stageAll·openFile·openChanges·
   unstageChanges·stageChanges·graph). dialog.tsx sr-only 2곳은 common.close 재사용.
3. `shared/ui/icon-button.tsx` 신설(aria-label + Tooltip 강제) + 툴팁 부재 37건 일괄 적용
   (aria 조차 없는 3건 최우선: syntax-token-row B/I 토글, stash 적용).
4. native title= 8곳 Tooltip 승격.
5. `{title}` 보간 2키를 `{{title}}` 로 + tab-item.tsx 수동 replace 를 `t(key, { title })` 로.
6. `MESSAGE_NAMESPACES` 87키 동기 + locale 테스트에 `en ⊆ required` 역방향 단언 추가.
- 툴팁 side 규범: 사이드바=right, 툴바·탭=bottom, 상태바=top (docs/features 문서화).

### 3.4 실행 구조 (역할 분담)

- **Phase A(스파인 단일, sonnet+high)**: settings types/service(ai_omlx_base_url·sanitize·
  화이트리스트) + secret.rs + ai/types.rs(Omlx variant) + locale 4곳(단축키 25키·OMLX 5키 +
  87키 동기 + {title} 보간 + en⊆required 테스트) + bindings 재생성 + emptySettingsPatch.
- **Phase B(파일 소유권 분리 병렬, sonnet+high ×3)**: B1 단축키 모달(프론트 전용) /
  B2 OMLX(ai 도메인 + dispatch arm + 설정 UI) / B3 툴팁·i18n(감사 목록 전량 — locale/service.rs
  는 Phase B 동안 B3 단독 소유, B1·B2 는 키 부족 시 보고만).
- **Phase C(병렬 리서치, opus+medium)**: monaco 0.56 이후 IME/composition 수정 조사 +
  업그레이드 영향(shikiToMonaco peer 등).
- **Phase D(검토)**: 렌즈 3종(계약·스파인 정합 / 정확성 / 컨벤션·FSD, opus+high) → 적대적
  검증(opus+medium) → 수정(sonnet+high). **Phase E(메인 2차)**: verify 전체 + vite build + 커밋.

## 4. 기각된 대안

| 기각안 | 사유 |
|--------|------|
| OMLX chat-only 단순화(1차 FIM 생략) | 사용자 기각 — auto-tab 이 핵심 가치. 센티널 조립이 기존 추상화로 표현 가능함을 확인 |
| FIM 센티널을 사용자 템플릿 파일에 위임(concat) | 모델 바꿀 때마다 사용자가 센티널 수동 교체 — 패밀리 테이블이 out-of-box 동작 |
| base URL 을 keyring JSON 에 (Codex 패턴) | base URL 은 비밀이 아님 — settings 평문이 의미상 정확 |
| 단축키 B안(KeymapActionId union 에 5개 추가) | 커맨드 늘 때마다 union 수정 — "전 커맨드 노출" 요구와 상충 |
| 모달을 cmdk(Command) 로 | 4열 표 + 행별 액션에 부적합 — ul + fuzzyFilter |
| 자모 분리의 앱 레벨 수정 | 조합 이벤트 가로채기 = hack. 원인이 WKWebView↔monaco 계층 |
| i18n 수정을 키 추가로 | 누락 키 0건 — 재렌더 결함이 원인. 키 추가는 증상 무해소 |
| 툴팁 부재를 개별 Tooltip 래핑 37회 | IconButton 공용 컴포넌트가 재발까지 방지 |

## 5. 완료 조건

- `bun run verify` 전체 통과 + vite build. 파리티(디스패치 137·locale 4곳·en⊆required 신규).
- 커밋 단위: IME 폰트 수정 / 단축키 모달 / OMLX / 툴팁·i18n (dev, 자동 커밋·푸시).
- 실기 확인은 사용자: IME 2건 절차(§2.1)·단축키 모달·OMLX 실서버 왕복·툴팁 육안 + QA6 잔여.
