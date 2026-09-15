# d-60 — 사용성 배치 5 웨이브 3: 에이전트 감지 다각화 (opencode · codex · pi · gemini) + 활동 신호 에이전트화 + 알림 차단 사유 (2026-09-15)

> 사용자 항목 5. 결정 전제: `acknowledge/2026-09-15-usability-batch5-user-decisions.md` §1 #6 · §3 #1(pi·gemini 는 "설치돼 있다고 가정하고 구현", opencode·codex 는 실측 탐침).
> 조사 정본 `research/2026-09-15-batch5-research.md` T4. 감지 구조 정본 `features/agent-integration.md` §1·§4·§7, d-54 계약. **d-59 커밋 후 착수**. Rust 는 한 시점 한 에이전트.
> 규칙: 코드 주석 금지(JSDoc 만)·arrow only·매직넘버 상수화·이모지 금지·`#[allow]` 금지·신규 의존성 0·시크릿 값 기록 금지(키 이름만). 로케일 3종 동시.

## 0. 현행 (T4 — 메인이 파일 위치 재확인)

| 축 | 현행 | 근거 |
|----|------|------|
| 신원 | `KNOWN_AGENT_NAMES = ["claude","codex","gemini"]`, comm basename 완전일치 → node/bun/deno 면 cmdline basename | `src-tauri/src/domain/agent/types.rs:8,69-71`, `service.rs:23,49-71` |
| 인밴드 OSC 777 | Claude 전용(`CLAUDE_HOOK_BINDINGS`·`claude_agent_event_payload`·`build_claude_agent_hook_command`·`claude_hook_entries`, 페이로드 `"agent":"claude"` 고정) | `service.rs:757-820` |
| 타이틀 글리프 | `parse_title_glyph(title)` 에이전트 인자 없음(Claude `◐◑`/`✳` 표가 전 에이전트에 적용) | `service.rs:392-393,460`, 호출부 `:534` |
| 다이얼로그 시그니처 | Claude 만, 나머지 `&[]` | `service.rs:406-413` |
| 비실질 글리프 | Claude 스피너 기준 9종 | `service.rs:398` |
| HTTP 훅 | codex·gemini 사용자 레벨(`.codex/hooks.json`·`.gemini/settings.json`), 서버·토큰·override | `hooks.rs:120-170,192`, `service.rs:691-739,847-869` |
| 폴백 게이트 | `activity != Unknown || name == claude` — codex/gemini 는 구형 휴리스틱 폴백 | `commands.rs:392` |
| 프론트 | 배지는 activity 키만(에이전트 분기 없음). 설정 UI `AGENT_HOOKS_AGENTS` 3항목 + 로케일 3키 | `src/features/project/agent-status-badge.tsx:9-14`, `src/widgets/settings-view/agent-hooks-project-list.tsx:19-23` |
| 설치 현황(이 Mac) | opencode 1.18.29(`/Users/gkn/.opencode/bin/opencode`, `~/.config/opencode/opencode.json`) · codex 0.144.6(node 셸 → 네이티브 `codex`, `~/.codex/config.toml`) · claude O · **gemini X · pi X** | T4 Q3 |
| 에이전트 기제(1차 출처) | opencode: Bus 이벤트 `session.idle`·`permission.asked`·`permission.replied`·`tool.execute.*`, 플러그인 `.opencode/plugins/`·`~/.config/opencode/plugins/`, 권한 옵션 문구 `Allow once`/`Allow always`/`Reject`, 타이틀 `OC \| …`(글리프 없음). codex: hooks 시스템 실재(`hooks.json`, 이벤트 `PreToolUse`·`PermissionRequest`·`PostToolUse`·`Stop`·`UserPromptSubmit`…, command 핸들러). pi: `earendil-works/pi`(구 badlogic/pi-mono), npm `@earendil-works/pi-coding-agent`, 확장 `~/.pi/agent/extensions/*.ts`·`.pi/extensions/*.ts`, 이벤트 `agent_start`/`agent_settled`/`ui_prompt_start`/`ui_prompt_end`/`tool_call` | T4 Q2 |
| 알림 | `AgentActivity` 4상태(권한 요청·질문이 `awaitingInput` 으로 접힘). Rust 내부 `AgentEvent::PermissionRequest\|QuestionAsked`·`BlockedSource` 는 IPC 를 못 넘음 | `types.rs:110-117`, `service.rs:554-557,645` |


### 0.1 탐침 결과 반영 (2026-09-15, wf `wf_bdc360b2` — 정본 `research/2026-09-15-agent-probe-*.md`)

| 축 | opencode (확정, 바이트 근거) | codex |
|----|------|------|
| 신원 | 단일 네이티브 Mach-O, comm 절대경로 basename `opencode`, pgid 리더 = 본체(자식은 MCP 서버) | pgid 리더 = **node 셸**(`codex.js:195 spawn(stdio:inherit)`), 네이티브 `codex` 는 자식 → 현행 node→cmdline basename 폴백으로 신원 무변경 |
| 타이틀 | OSC 0 만. 기동 `OpenCode`, 첫 턴 요약 후 `OC \| <요약>`, 종료 시 빈 문자열. **선행 글리프 없음, 권한 대기 중 갱신 0건** → `TITLE_GLYPHS_FOR(opencode) = None` | [미확인] (세션 진입 전 차단). 바이너리에 `[tui].terminal_title` 식별자 존재 |
| 스피너 | **유휴·권한 대기 중에도** 6프레임 무한 순환 `◦ U+25E6 · U+00B7 • U+2022 ● U+25CF ○ U+25CB ◌ U+25CC` → `NON_SUBSTANTIVE_GLYPHS` 에 5종(`·` 는 기존) 추가 필수(없으면 영구 Working). 진행 글리프 `⬝ U+2B1D`·`■ U+25A0` 는 턴 진행 중에만 → 실질 출력으로 둬도 옳다 | [미확인] |
| 다이얼로그 | 정규화 텍스트(대소문자 정확) `Permission required` / `Shell command` / `Allow once` / `Allow always` / `Reject` / `enter confirm`. 렌더는 CUP(`CSI row;col H`) 셀 배치이나 **구 단위가 한 셀 런에 들어가** 정규화 후 독립 라인으로 복원됨(완전일치 가능). 시그니처는 `Permission required` 1개만 채택(질문형·계획 승인 다이얼로그 문구 미포착 → 넣지 않음). 사용자 기본 설정은 승인 없이 실행(permission 키 없음) → 다이얼로그 자체가 드묾 | 신뢰 다이얼로그만 채취: `› 1. Yes, continue` / `Press enter to continue`. **단어마다 CUP 이동**(`Do\e[3;6Hyou\e[3;10Htrust`) → 현 정규화(H→개행)에서는 단어 사이에 개행이 들어가 다구 시그니처 매치 불가. 승인 문구 후보(strings): ` needs your approval.` · `Approval requested: ` · `Do you want to approve network access to "` — 미검증이라 표에 넣지 않는다 |
| 인밴드 전제 | **확정**: 플러그인 `process.stdout.write` 의 OSC 777 이 pty 에 그대로 도달(raw 100건, opentui 대체화면과 충돌 없음). `permission.asked`/`session.idle` 이 플러그인 핸들러에 실제 오는 순간은 [미확인](40초 창 안에 미도달) | hooks 이벤트 식별자 4종 실재. command 훅의 `/dev/tty` 쓰기 가능성은 [미확인](간접 근거: codex 자신의 OSC 52 `/dev/tty` 경로·샌드박스 `/dev/tty` 허용 규칙) |
| 설치본 | 1.18.29, 플러그인 `oh-my-openagent@latest` 사용 중(사용자 설정) | `~/.bun/bin/codex` 0.142.0(PATH 1위) · nvm 경로 0.144.6 |

**계약 조정**
- §1.B 에 **스캐너 정규화 보강** 추가: `infra/terminal_scan.rs` 가 CUP(`CSI row;col H`) 을 처리할 때 **같은 행으로의 이동은 공백 1개, 다른 행은 개행**으로 치환(현재는 일괄 개행 — 확인 후). Claude 의 `CSI n G` 처리와 대칭. 회귀: Claude 시그니처 테스트·d-54 리플레이 타임라인 3건 전부 유지. codex 다이얼로그 시그니처는 여전히 빈 표(실측 후).
- §1.C codex 인밴드: 구현하되 `/dev/tty` 도달은 [미확인] → 훅 명령은 Claude 와 동일 형태(`/dev/tty` 로 printf, 실패 시 exit 0). HTTP 서버·토큰 경로는 gemini 용으로 남으므로, 사용자 실기에서 codex 인밴드 이벤트가 관측되지 않으면 codex 를 HTTP 로 되돌리는 것은 설정 1줄(에이전트 스펙 테이블의 전달 방식 필드)로 가능하게 설계한다.
- §1.C opencode 플러그인: 파일은 **사용자 레벨 `~/.config/opencode/plugins/taide-agent.js`** 로 두되, `opencode.json` 의 `plugin` 배열은 건드리지 않는다(디렉토리 자동 로드 여부는 공식 문서 기준 — 자동 로드가 아니면 [미확인] 표기 + 설정 안내 문구). 사용자의 기존 플러그인(`oh-my-openagent`)과 공존.
- §1.B opencode 다이얼로그 시그니처 = `Permission required` 단독. 사용자 설정이 무승인이라 실효는 낮으나 인밴드 `permission.asked` 가 주 신호.

## 1. 수정 방향

### 1.A 실측 탐침 (opencode · codex — 설치본, 세션 내 실행)

- `docs/debugging.md` §4 의 expect + pty 캡처 기법으로 각 CLI 를 실제 구동해 **원시 바이트**를 채취한다. 프롬프트는 무해하고 짧게(예: "현재 디렉토리의 파일 목록을 보여줘" 로 셸 도구 승인 다이얼로그 유도). 인증은 기존 설정 재사용(`~/.codex/auth.json`, `~/.config/opencode`); 키 값·토큰은 어떤 산출물에도 기록하지 않는다. 임시 작업 디렉토리는 세션 스크래치.
- 채취 목표: ① 승인/권한 다이얼로그의 **정규화 텍스트**(스캐너 정규화 후 문자열, `CSI n G` 삽입 여부) ② 타이틀 OSC 0/2 실제 시퀀스 ③ 작업 중 스피너 글리프 집합 ④ 유휴 프롬프트 형태 ⑤ codex 가 command 훅에서 `/dev/tty` 로 쓸 수 있는지(인밴드 전환 전제) ⑥ opencode 플러그인이 pty 로 바이트를 쓸 수 있는지(`process.stdout` 접근).
- 산출: `docs/research/2026-09-15-agent-probe-opencode-codex.md`(바이트 덤프 요약 + 시그니처 표) — 이후 절의 상수는 **이 산출물에서만** 가져온다(추측 금지). pi·gemini 는 탐침 불가 → 문서 기반 + `[미확인]` 표기, 시그니처 표는 비워 둔다(신원·훅 이벤트만).

### 1.B Rust — 신원·활동 신호 에이전트화 (M)

- `KNOWN_AGENT_NAMES` 에 `opencode`·`pi` 추가(+상수). `pi` 는 2글자라 오탐 위험: comm 완전일치 `pi` 또는 cmdline basename `pi` 이되 **인자 첫 토큰이 `pi` 계열 패키지 경로(`pi-coding-agent`)** 이거나 comm 이 `pi` 인 경우만(구현 시 `service.rs:49-71` 매칭 규칙에 에이전트별 보조 조건 훅을 두고 테스트).
- `parse_title_glyph(agent_name, title)` 로 시그니처 변경, 글리프 표를 에이전트별 상수(`TITLE_GLYPHS_FOR`)로. opencode(`OC | …`)는 글리프 없음 → None. 호출부·테스트 20여 건 갱신.
- `dialog_signatures_for` 에 opencode(탐침 확정 문구) 추가. codex 는 탐침 결과에 따라. `NON_SUBSTANTIVE_GLYPHS` 를 에이전트별 합집합으로 확장(탐침 확정분만).
- 폴백 게이트(`commands.rs:392`)를 "훅 관리 에이전트 여부" 기준으로 재정의(하드코딩 `claude` 제거).

### 1.C Rust — 인밴드 OSC 777 통일 (L)

- `CLAUDE_HOOK_BINDINGS`/`claude_agent_event_payload`/`build_claude_agent_hook_command`/`claude_hook_entries` 를 **agent 파라미터화**(페이로드 `"agent"` 필드·이벤트 매핑 표를 에이전트별 상수로). 마커·env 게이트·exit 0 규약(`agent-integration.md` §4.2·§4.3)은 공통.
- **codex**: HTTP 훅 → 인밴드 command 훅(`~/.codex/hooks.json`, 이벤트 `UserPromptSubmit`·`PermissionRequest`·`PostToolUse`·`Stop`; 탐침 ⑤ 가 `/dev/tty` 쓰기 불가로 나오면 HTTP 유지하고 §3 에 근거 기록). 서버·토큰 경로는 gemini 가 남아 있는 동안 유지.
- **opencode**: 플러그인 파일(JS) 설치 — `~/.config/opencode/plugins/taide-agent.js`(사용자 레벨; 프로젝트 레벨 `.opencode/plugins/` 는 쓰지 않음) 가 `permission.asked`→`permission_request`, `permission.replied`·`tool.execute.after`→`tool_complete`, `session.idle`→`stop` 을 OSC 777 로 방출(탐침 ⑥ 가 불가면 HTTP 훅 경로로 폴백 설계 + 기록). 설치·제거·멱등성은 §7.6 서드파티 파일 안전성 규칙(백업·마커·JSON 이 아닌 JS 파일이므로 **파일 통째 소유**: TAIDE 마커 헤더가 있는 파일만 덮어쓰고 그 외엔 건드리지 않음).
- **pi**: 확장 파일(TS) `~/.pi/agent/extensions/taide-agent.ts` — `ui_prompt_start`→`permission_request`, `ui_prompt_end`→`tool_complete`, `agent_start`→working 힌트, `agent_settled`→`stop`. opencode 와 같은 "TAIDE 소유 파일" 규칙. 설치본이 없으므로 로딩 경로·API 시그니처는 README 1차 출처 기준 + `[미확인]` 표기, 파일 내용은 단위 테스트로 고정.
- **gemini**: 현행 HTTP 유지(회귀 검증 불가 → 무변경 원칙). `~/.gemini/settings.json` 훅 로직은 손대지 않는다.
- `hook_scope_for_agent`·`managed_hook_events_for`·`user_level_hooks_path`·`user_level_hook_command_timeout`·`map_hook_event_to_activity` 의 match 팔에 opencode·pi 추가(6곳 흩어진 분기를 에이전트별 `AgentSpec` 테이블 1개로 모으는 리팩터 허용 — 단 동작 동일성 테스트 선행).
- `reconcile`/`remove` 루프(`hooks.rs:121,158`)가 새 에이전트 파일을 포함.

### 1.D Rust·FE — 차단 사유 노출 + 알림 분리 (M)

- `DetectedAgent` 에 `blockedReason?: 'permission' | 'question' | 'dialog'`(`BlockedSource`·`last_event` 에서 유도) 추가 → bindings 재생성. 배지 툴팁에 사유 표기(로케일 3키). 알림(d-58 §1.F 의 `agentAwaitingInputBody`)을 사유별 문구로 분기(`notification.agentPermissionBody`/`agentQuestionBody`). 알림 설정 카테고리 `agentAwaitingInput` 신설(Rust settings 카테고리 enum + 설정 UI 토글 + 로케일) — 사용자가 "입력 대기" 알림만 끌 수 있게.
- `docs/ipc-contract.md` agent 절, `agent-integration.md` §1.2 표 갱신.

### 1.E FE — 설정 UI (S)

- `AGENT_HOOKS_AGENTS` 에 opencode(user)·pi(user) 추가 + 로케일 `settings.agentHooksAgentOpencode`/`…Pi`. 상태 행(`agent-cli-status-row.tsx`)에 설치 감지(`which`)가 있으면 두 CLI 추가.

### 1.F 문서

- `agent-integration.md` §1.1 감지 대상·§1.2 신호 표(에이전트별 열)·§4 훅 절(codex 인밴드·opencode 플러그인·pi 확장·gemini HTTP 유지)·§7.5. `docs/ipc-contract.md`. 탐침 산출물 링크. `[미확인]` 목록(pi·gemini 실측).

## 2. 실행 계획

- **탐침 wf**(opus·xhigh, 1 에이전트, 20분 상한): 1.A → 산출물 작성. 실패(인증 필요 등) 시 사유를 기록하고 멈춘다(우회 금지).
- **구현 wf**(opus·xhigh): **R**(Rust 단일: 1.B → 1.C → 1.D Rust → bindings) → **F**(TS: 1.D FE·1.E) 순차. 각 단계 검증 종료 조건은 d-58 과 동일.
- **렌즈 검토 wf**(sonnet·xhigh 3렌즈: 근본성·회귀(Claude 경로 무변경)·경계(서드파티 파일 안전성)) + major 적대적 2표.
- 메인 2차 verify → 커밋(feat(agent)·feat(notification)) → dev 푸시 → main ff. 사용자 실기: opencode·codex 세션 배지(권한 다이얼로그 즉시 전환·유휴 복귀), 알림 문구.

## 3. 구현 기록

<!-- d60-record-probe -->
- **§1.B 완료 (2026-09-15)** — R1 배정(§1.B 전부 + §0.1 스캐너 보강). §1.C·§1.D 는 미착수.
  - **변경 파일(역할)**
    - `src-tauri/src/domain/agent/types.rs` — `KNOWN_AGENT_NAMES` 를 `AGENT_NAME_*` 상수 조립으로 바꾸고 `opencode`·`pi` 추가, `AGENT_NAME_OPENCODE`/`AGENT_NAME_PI`/`PI_PACKAGE_PATH_MARKER`(`pi-coding-agent`) 신설.
    - `src-tauri/src/domain/agent/service.rs` — ① 신원: `AgentNameSource`(Comm/Cmdline) + `agent_match_is_confirmed` 보조 조건 훅 — `pi` 는 comm 완전일치이거나 cmdline 에 패키지 경로가 있을 때만 채택. ② `AgentTitleGlyphs` 구조체 + `title_glyphs_for(agent)` → `parse_title_glyph(agent_name, title)`(claude 만 표 보유, 나머지 `None`). ③ `dialog_signatures_for` 에 `OPENCODE_DIALOG_SIGNATURES = ["Permission required"]` 추가. ④ `agent_non_substantive_glyphs(agent)` + `OPENCODE_NON_SUBSTANTIVE_GLYPHS`(`◦ • ● ○ ◌`) → `is_substantive_output(agent_name, text)` 가 공통 9종과 합집합으로 판정. ⑤ `uses_project_hook_override(agent)` 신설(codex·gemini 만 true).
    - `src-tauri/src/infra/terminal_scan.rs` — `NormalizedText { bytes, cursor_row }` 도입, `OutputScanner.cursor_row` 로 청크 경계를 넘어 행 추적. CUP/HVP/VPA(`H`·`f`·`d`)는 **같은 행이면 공백 1개, 다른 행이면 개행**, 상대 행 이동(`A`·`B`·`E`·`F`)은 개행 + 추적 행 갱신. 행 미상은 "같은 행"으로 본다. CSI 파라미터 파서(`csi_param`/`csi_row_param`/`csi_row_delta_param`) + `CSI_DEFAULT_ROW`·`CSI_DEFAULT_ROW_DELTA`·`CSI_ROW_PARAM_INDEX` 상수.
    - `src-tauri/src/domain/agent/commands.rs` — `resolve_activity` 폴백 게이트를 `probe.name == AGENT_NAME_CLAUDE` 하드코딩에서 `!service::uses_project_hook_override(probe.name)` 로 재정의.
    - `docs/features/agent-integration.md` — §1.1 에이전트별 신원 판정 표 + `pi` 보조 조건 절, §1.2 에이전트별 글리프·시그니처 표(`[미확인]` 표기 포함) + CUP 정규화 절, §4.4 폴백 게이트 설명.
    - `docs/features/terminal.md` §5.2 — 정규화 규칙에서 절대 행 이동 분기와 행 추적 항목 갱신(코드와 어긋나 있던 "`H` 일괄 개행" 서술 정정).
  - **테스트** — `cargo test --workspace` 1569 pass / 0 fail(변경 전 1552 → 신규 17). 신규: 신원 5건(opencode 절대경로 comm · pi comm 단독 · node+패키지경로 · node+이름만 미탐지 · 타 에이전트 무영향), 표 5건(opencode 스피너 비실질 · 진행 글리프 실질 · 글리프 표 없는 에이전트 · opencode 다이얼로그 문구 · 빈 표 확인), 게이트 1건, 스캐너 5건(codex `Do\e[3;6Hyou\e[3;10Htrust` → `Do you trust` · 다른 행 개행 · opencode 다이얼로그 라인 · 청크 경계 행 추적 · 행 파라미터 기본값), opencode 실물 바이트 e2e 2건(다이얼로그 래치 유지 · 스피너만이면 유휴 복귀). 회귀 기준인 **Claude 경로 기존 테스트(d-54 리플레이 타임라인 3건·시그니처·정규화 포함) 전부 무수정 통과**.
  - **검증** — `cargo fmt --all` · `cargo clippy --workspace --all-targets -- -D warnings`(무경고) · `cargo test --workspace` · bindings 재생성(`typescript_바인딩을_생성한다`, **산출물 무변경** — §1.B 는 IPC 타입을 건드리지 않는다) · `bun run typecheck`(무오류). 앱 실행·CLI 구동·커밋 없음.
  - **이탈(이유)**
    - `NON_SUBSTANTIVE_GLYPHS` 를 전 에이전트 단일 합집합으로 넓히지 않고 **에이전트별 합집합**(공통 표 + `agent_non_substantive_glyphs`)으로 구현했다. 단일 합집합이면 Claude 세션도 `◦ • ● ○ ◌` 를 비실질로 읽게 되어 "Claude 경로 무변경" 회귀 기준을 깬다.
    - `is_substantive_output`/`parse_title_glyph` 의 시그니처에 `agent_name` 을 추가했다(계약은 `parse_title_glyph` 만 명시). 표가 에이전트별이 된 이상 호출부가 에이전트를 넘기지 않으면 표를 고를 수 없다.
    - `docs/ipc-contract.md` 는 건드리지 않았다 — R1 범위(§1.B)에서 IPC 타입·이벤트가 바뀌지 않았다. `blockedReason` 추가는 §1.D 몫이다.
    - `docs/features/terminal.md` §5.2 는 계약 §1.F 목록에 없지만, 내가 바꾼 정규화 규칙을 그대로 서술하고 있어 수정하지 않으면 문서가 코드와 어긋난다. 한 항목만 최소 수정했다.
  - **미확인 · 후속**
    - `pi` 미설치 — 신원 보조 조건은 문서 기반이다. argv 에 `pi-coding-agent` 가 없는 shim 실행(`node /usr/local/bin/pi`)은 **미탐지**로 남는다(오탐보다 미탐을 택한 의도적 선택). 설치 후 실측으로 조건을 좁히거나 넓혀야 한다.
    - codex·pi·gemini 의 `TITLE_GLYPHS_FOR`·`dialog_signatures_for`·비실질 글리프는 빈 표다(`[미확인]`). codex 는 승인 다이얼로그 문구 자체가 미채취고, 단어마다 CUP 으로 옮겨 그려도 이제 같은 행이면 한 문구로 복원되므로 **문구만 실측되면 상수 승격이 가능**하다.
    - opencode 다이얼로그는 사용자 기본 설정(`permission` 키 없음)에서 거의 뜨지 않는다 — 실효 신호는 §1.C 의 인밴드 플러그인이다.
    - 행 추적은 스크롤·클램프에서 어긋날 수 있다(공백 종류만 바뀌고 텍스트는 보존). `\n` 은 행 +1 로만 반영한다.
- **§1.C 완료 (2026-09-15)** — R2 배정(§1.C 전부). §1.D·§1.E 는 미착수.
  - **변경 파일(역할)**
    - `src-tauri/src/domain/agent/service.rs` — ① **`AGENT_SPECS` 표 1개** 신설(`AgentSpec`/`AgentHooks`/`AgentHookBinding` + `HookDelivery{InBandTty,Http}`·`HookInstallShape{JsonEntries,OwnedFile}`). 계약이 지목한 6곳의 match 팔(`map_hook_event_to_activity`·`hook_scope_for_agent`·`managed_hook_events_for`·`user_level_hook_command_timeout`·`user_level_hooks_path`·폴백 게이트 `uses_project_hook_override`)이 전부 표 조회 한 줄이 됐고, `hook_install_shape`·`agent_hooks`·`user_level_hook_agents`(사용자 레벨 순회)를 추가했다. ② 훅 빌더 agent 파라미터화: `claude_agent_event_payload`→`agent_event_payload(agent, event)`, `build_claude_agent_hook_command`→`build_agent_hook_command(agent, …)`, `claude_hook_entries`→`agent_hook_entries(agent, …)`, `CLAUDE_HOOK_BINDINGS` 를 구조체 슬라이스로. JSON 주입/제거/비교도 `inject_taide_agent_hook_entries`/`remove_taide_agent_hook_entries`/`agent_hook_entries_match`/`has_taide_agent_hook_entries`/`installed_taide_entries` 로 일반화. ③ 소유 파일 생성기 `build_owned_hook_file_source(agent)` + `is_owned_hook_file` + `OWNED_HOOK_FILE_MARKER`(`taide-agent-managed-file`) + opencode/pi 본문 상수. ④ 인밴드 이벤트 어휘에 `AgentEvent::PromptSubmit`(`prompt_submit`) 추가 — `tool_complete` 와 같이 래치를 풀고 `ACTIVITY_WORKING_HOLD_MS` 동안 Working. ⑤ `USER_LEVEL_IN_BAND_EMITTER = DevTty`(`terminalSequence` 는 Claude 전용 출력 계약).
    - `src-tauri/src/domain/agent/hooks.rs` — 재조정을 전달 방식별로 분리: `reconcile_in_band_user_level_hooks`(서버 불필요, codex JSON + 소유 파일) / `reconcile_http_user_level_hooks`(구 `reconcile_user_level_hooks`, gemini). 제거 루프도 표 순회 + 형태별 분기(소유 파일은 `remove_owned_hook_file`). `apply_agent_hooks_toggle` 은 두 재조정을 다 거친다.
    - `src-tauri/src/domain/agent/commands.rs` — 소유 파일 I/O 3종(`read_owned_hook_file`/`write_owned_hook_file`/`remove_owned_hook_file`), `install_user_level_hooks`(소유 파일 / 인밴드 JSON / HTTP shim 3분기 — CLI·hooks 서버 전제는 HTTP 분기에만), `agent_hooks_status`·`agent_hooks_uninstall` 의 소유 파일 분기.
    - `src-tauri/src/domain/agent/types.rs` — opencode 버스 이벤트 4종·pi 라이프사이클 이벤트 4종 상수.
    - `docs/features/agent-integration.md` — §4 서두를 스펙 표 설명 + 5행 표로, §4.2 에 `USER_LEVEL_IN_BAND_EMITTER` 항, §4.4 를 "codex 인밴드 · gemini HTTP 유지"로 재작성(이벤트 4종 표·되돌리기 한 줄·`[미확인]`), **§4.5 신설**(opencode·pi 소유 파일 규칙·이벤트 매핑·권한), statusline 을 §4.6 으로. §1.2 신호 1행에 `prompt_submit` 과 에이전트 목록. §7.5 에 표 통합·HTTP 표 잔존 이유·소유 파일 경로·재조정 2분기·CLI 전제 축소·테스트는 임시 HOME.
    - `docs/ipc-contract.md` — 원격 스코프 조건부 거부 절의 User 스코프 목록에 opencode·pi(플러그인 파일 통째 쓰기) 추가. 게이트 자체는 `hook_scope_for_agent` 기반이라 코드 변경 없이 두 에이전트를 덮는다.
  - **테스트** — `cargo test --workspace` 1586 pass / 0 fail(R1 이후 1569 → 신규 17). service 11건: 설치 형태·전달 방식 표 / 스펙 표 불변식(스코프↔경로, 소유 파일↔본문, 설치 이벤트 ⊆ 제거 이벤트) / **claude 7행 명령 바이트 동일**(리터럴 고정) / 페이로드 agent 필드 / codex 인밴드 4행 + `timeout` / claude 행에 `timeout` 없음 / codex 주입 멱등·사용자 항목 보존 / 소유 파일 소스(마커·표·env 게이트·제어바이트 없음 + 전 바인딩 시퀀스 스캐너 왕복) / 소유 판정은 첫 줄만 / 훅 없는 에이전트 / `prompt_submit` 래치 해제+Working. hooks 6건: codex 구 HTTP → 인밴드 교체·멱등 / taide 항목 없는 codex 무접촉 / 소유 파일 설치·토글 해제 제거 / 남의 플러그인 무접촉(재조정·제거 양쪽) / 옛 소유 파일 갱신·멱등 / 미설치 경로는 생성 안 함. **사용자 레벨을 만지는 테스트는 전부 임시 디렉터리를 `home_env` 로 주입한다**(`~/.codex`·`~/.config/opencode`·`~/.pi`·`~/.gemini` 무접촉). 회귀 기준인 Claude 경로 기존 테스트는 전부 통과.
  - **검증** — `cargo fmt --all` · `cargo clippy --workspace --all-targets -- -D warnings`(무경고) · `cargo test --workspace` · bindings 재생성(`typescript_바인딩을_생성한다` pass, **산출물 무변경** — §1.C 는 IPC 타입을 바꾸지 않는다) · `bun run typecheck`(무오류). 생성되는 플러그인 2종은 스크래치에 덤프해 `node --check`(JS)·`bun build`(TS) 로 구문만 확인하고 지웠다. 앱 실행·CLI 구동·커밋 없음.
  - **이탈(이유)**
    - **`AgentEvent::PromptSubmit` 를 새로 만들었다**(계약에 없음). codex `UserPromptSubmit`·pi `agent_start` 를 기존 어휘로 옮기려면 `tool_complete`(도구 결과)로 위장해야 했다. 와이어에 새 이름 하나를 더하는 것이 정직하고, 리더가 모르는 이벤트를 무시하므로 프로토콜 버전은 1 그대로다. Claude 는 이 이벤트를 내지 않아 경로 무변경.
    - **`http_activities` 표를 codex 에도 남겼다.** 인밴드로 옮겼으니 지울 수도 있었지만, 남겨야 ① 구버전 HTTP 설치가 재조정 전까지 계속 해석되고 ② 되돌리기가 `delivery` 필드 하나로 끝난다(계약 §0.1 이 요구한 "설정 1줄").
    - **`uses_project_hook_override(codex)` 가 `true`→`false` 로 바뀌었다**(의도한 동작 변경). 기존 테스트 `프로젝트_오버라이드는_http_훅_에이전트만_쓴다` 를 그에 맞춰 갱신했다 — 표의 `delivery` 를 되돌리면 이 테스트도 같이 되돌아간다.
    - **codex `hooks.json` 스키마는 기존 설치 코드 형태(`{"type","command","timeout"}`)를 그대로 썼다.** 계약 §1.C 가 언급한 바이너리 식별자(`matcher`·`timeoutSec`·`enabled`)는 채택하지 않았다 — 기존 HTTP 설치가 이미 `timeout` 으로 쓰고 있었고, 실측 없이 키 이름을 바꾸면 검증되지 않은 변경이 된다(§1.A 탐침이 codex 신뢰 다이얼로그에서 막혀 훅 파일을 못 봤다).
    - **소유 파일 권한은 `0600` 이 아니라 기본 umask 다.** hooks JSON 은 URL 에 서버 토큰이 실려 `write_hooks_file_preserving_mode` 로 좁히지만(§7.6), 플러그인 파일은 상수 페이로드뿐이고 사용자 자신의 플러그인과 같은 디렉터리에 있어야 한다.
    - **`docs/ipc-contract.md` 를 건드렸다.** IPC 타입·이벤트는 무변경이지만, 원격 거부 문서가 User 스코프를 "codex/gemini"로 열거하고 있어 두 에이전트를 추가하지 않으면 문서가 코드와 어긋난다(게이트는 이미 `hook_scope_for_agent` 기반이라 코드 변경은 없다).
  - **미확인 · 후속**
    - **codex 훅의 `/dev/tty` 도달 `[미확인]`**(탐침 ⑤ 미해결). 실기에서 codex 배지가 안 움직이면 `AGENT_SPECS` 의 codex 행 `delivery` 를 `Http` 로 되돌린다(그 외 코드·문서 변경 불필요).
    - **opencode `permission.asked`/`session.idle` 의 플러그인 도달 `[미확인]`**(탐침 40초 창 안에 미도달 — 기동 이벤트 도달과 stdout→pty 경로는 바이트로 확정). 전역 `~/.config/opencode/plugins/` 자동 로드는 **공식 플러그인 문서로 확인**(등록 불요) — `[미확인]` 아님. 프로젝트 레벨은 `plugins`·`plugin` 둘 다 로드되지만(탐침 §6) TAIDE 는 전역 `plugins` 한 곳만 쓴다.
    - **pi 는 전부 `[미확인]`**(미설치). 파일 형태는 pi 확장 문서의 `export default (pi) => { pi.on(…) }` 만 따랐고, `process.stdout.write` 가 pi TUI 에서 pty 에 도달하는지도 미검증. 모듈 해석 실패가 pi 기동을 깨지 않도록 생성 파일은 아무것도 import 하지 않는다.
    - 소유 파일 경로에 **디렉터리가 아니라 같은 이름의 심링크/디렉터리**가 있는 경우는 별도 처리를 두지 않았다(쓰기가 실패하면 로그 경고 후 넘어간다).
    - `AGENT_HOOKS_AGENTS`(설정 UI)에 opencode·pi 를 노출하는 것은 §1.E(F 웨이브) 몫이라, 지금은 백엔드만 준비된 상태다.
- **§1.D Rust 완료 (2026-09-15)** — R3 배정(§1.D 의 Rust 절 + 알림 카테고리 + bindings). §1.D FE(배지 툴팁·알림 문구 분기)·§1.E 는 미착수.
  - **변경 파일(역할)**
    - `src-tauri/src/domain/agent/types.rs` — `BlockedReason { Permission, Question, Dialog }` 신설(IPC 타입), `DetectedAgent.blocked_reason: Option<BlockedReason>`(`#[serde(default)]`) 추가.
    - `src-tauri/src/domain/agent/service.rs` — ① `blocked_reason(signals)` 순수 함수: `BlockedSource::Dialog` → `dialog`, `BlockedSource::Event` + `last_event == question_asked` → `question`, 그 외 이벤트 래치 → `permission`, 래치 없음 → `None`. ② `SessionState { activity, blocked_reason }` + `classify_session_state(signals, previous, now)` — 활동과 사유를 **한 스냅샷**에서 함께 뽑는다(`classify_session` 자체는 무변경).
    - `src-tauri/src/domain/agent/commands.rs` — `AgentStore::classify_session_activity` → `classify_session_state`(반환 `service::SessionState`), `resolve_activity` → `resolve_state`, `build_detected_agents` 가 `blocked_reason` 을 싣는다. HTTP override 폴백은 `blocked_reason: None`(프로젝트 스코프 override 는 활동 하나만 기억한다).
    - `src-tauri/src/domain/notification/types.rs`·`service.rs` — `NotificationCategory::AgentAwaitingInput` 신설(6종 → 7종), `is_category_enabled` 팔 추가. enum 이 exhaustive match 라 스위치 없는 카테고리는 컴파일이 막는다.
    - `src-tauri/src/domain/settings/types.rs`·`service.rs`, `src-tauri/src/domain/sync/service.rs` — `Settings.notify_agent_awaiting_input`(기본 `true`, `#[serde(default = "default_true")]`) + `SettingsPatch` 필드 + `apply_patch` + `settings_to_sync_patch`.
    - `src/shared/api/bindings.ts` — 재생성 산출물(`BlockedReason` 타입, `DetectedAgent.blockedReason?`, `NotificationCategory` 7종, `Settings`/`SettingsPatch` 의 `notifyAgentAwaitingInput`).
    - `src/entities/settings/settings.ipc.ts` — `emptySettingsPatch()` 에 `notifyAgentAwaitingInput: null` 1줄(`SettingsPatch` 는 전 필드 필수라 typecheck 가 요구한 유일한 TS 수정).
    - `docs/ipc-contract.md` — agent 절에 `blockedReason` 유도·null 규약·발행 조건, notification 절에 카테고리 7종과 동반 설정 스위치.
    - `docs/features/agent-integration.md` — §1.2 에 차단 사유 표(래치 × 마지막 이벤트 → 사유) + 래치 수명 규약, §1.3 의 호출부 서술을 `classify_session_state` 로 갱신.
    - `docs/data-model.md` §21 신설 — `notifyAgentAwaitingInput` 1필드(§19 8필드의 후속).
  - **테스트** — `cargo test --workspace` 1592 pass / 0 fail(R2 이후 1586 → 신규 6). agent service 5건: 다이얼로그 래치 → `dialog` / 인밴드 `permission_request`·`question_asked` → 각 사유 / 래치 해제 시 사유도 `None`(히스테리시스 구간 포함) / 차단 없는 세션은 사유 없음 / `agents_changed` 가 **활동 같고 사유만 다른 전이**를 변경으로 본다. notification 1건: 완료 스위치와 입력 대기 스위치의 독립성(한쪽만 꺼도 다른 쪽은 전달). 기존 테스트 갱신: 카테고리 전수 목록·`disable_category`(7종), 설정/동기화 알림 테스트 8종 → 9종, `세션의_에이전트가_바뀌면…` 이 `SessionState` 로 비교. 회귀 기준인 **Claude 경로 기존 테스트 전부 무수정 통과**(판정 우선순위·래치·d-54 리플레이 타임라인 무변경).
  - **검증** — `cargo fmt --all` · `cargo clippy --workspace --all-targets -- -D warnings`(무경고) · `cargo test --workspace` · bindings 재생성(`typescript_바인딩을_생성한다`) · `bun run typecheck`(수정 후 무오류). 앱 실행·CLI 구동·커밋 없음. 사용자 홈의 에이전트 설정 파일은 이번 절에서 읽지도 쓰지도 않았다(파일 I/O 경로 무변경).
  - **이탈(이유)**
    - **`classify_session` 을 고치지 않고 `classify_session_state` 를 위에 얹었다.** 판정 함수 자체가 사유를 돌려주게 바꾸면 d-54 의 우선순위 테스트 20여 건이 전부 반환 타입 때문에 다시 쓰여야 하고, 그건 "Claude 경로 무변경" 회귀 기준을 흐린다. 판정은 그대로 두고 조립만 한 겹 더 얹었다.
    - **`AgentStore::classify_session_activity` 를 `classify_session_state` 로 rename 했다**(계약은 필드 추가만 명시). 반환이 `SessionState` 가 된 이상 `_activity` 라는 이름은 거짓말이다.
    - **HTTP override 폴백의 사유는 `None` 고정**이다. override 는 `(activity, Instant)` 만 기억하므로 사유를 실으려면 저장 구조와 훅 수신부까지 바꿔야 하는데, 그 경로는 지금 gemini 전용이고 §1.D 범위 밖이다. `awaitingInput` + 사유 없음이 정직한 답이다.
    - **이벤트 래치에서 `question_asked` 가 아닌 경우를 전부 `permission` 으로 접었다.** 래치를 세우는 이벤트는 그 둘뿐이고 래치와 `last_event` 가 같은 호출에서 기록되므로 다른 조합은 도달 불가다. 도달 불가 팔을 만들어 `unreachable!` 을 쓰는 대신 의미가 옳은 기본값을 택했다(사유는 문서 주석에 남겼다).
    - **`docs/data-model.md` §21 을 추가했다** — 계약 §1.F 목록에는 없지만 §19 가 "알림 설정 8필드" 의 정본이라, 9번째 필드를 적지 않으면 정본이 코드와 어긋난다. R1 의 `terminal.md` §5.2 선례와 같은 최소 수정이다.
  - **미확인 · 후속**
    - **`NotificationCategory::AgentAwaitingInput` 은 아직 호출자가 없다.** 활동에 따라 카테고리를 고르는 것(`native-notification-provider.tsx` 의 `notifyNative` 호출)과 사유별 문구(`notification.agentPermissionBody`/`agentQuestionBody`)는 §1.D FE 몫이다. 그때까지 입력 대기 알림은 종전대로 `agentCompleted` 스위치를 탄다.
    - **설정 UI 토글·로케일 3키**(`settings.notificationsAgentAwaitingInput`)와 `docs/features/settings-ui.md` §2.1.1 표(스위치 8개 → 9개)는 §1.E 웨이브에서 함께 넣어야 한다. 지금은 백엔드 스위치만 존재해 UI 에서는 끌 수 없다.
    - **`blockedReason` 은 인밴드 이벤트가 오는 에이전트에서만 `permission`/`question` 으로 갈린다.** 시그니처만 있는 에이전트(현재 codex·pi·gemini 는 표 자체가 비어 있다)는 최대 `dialog` 이고, opencode 는 `Permission required` 문구가 곧 권한이지만 화면 판정만으로는 `dialog` 로 나간다 — 실효 사유는 §1.C 의 플러그인 이벤트에서 온다.
    - 히스테리시스 구간의 `awaitingInput` + `blockedReason: null` 조합을 FE 가 어떻게 그릴지는 §1.D FE 에서 정해야 한다(사유 없는 툴팁 폴백 문구 필요).
- **§1.D FE · §1.E 완료 (2026-09-15)** — F 배정(§1.D 의 프론트 절 + §1.E + 문서). Rust 는 `MESSAGE_NAMESPACES` 키 등재만 건드렸다.
  - **변경 파일(역할)**
    - `src/shared/lib/agent-status-text.ts`(신규) — `agentStatusLabelKey(activity, blockedReason)` 순수 함수. 사유가 있는 `awaitingInput` 만 `agent.blocked.{permission,question,dialog}` 로 가고 그 외는 종전 `agent.status.{activity}` 를 그대로 돌려준다. 문구 선택이 한 곳이라 세 소비처가 갈릴 수 없다.
    - `src/widgets/app-sidebar/sortable-project-icon.tsx` — 툴팁의 세션 줄이 사유를 말한다. `aggregateActivity`(활동만 반환)를 `topPriorityAgent`(대표 에이전트 반환)로 바꿔 배지 도형과 사유가 **같은 에이전트**에서 나오게 했다.
    - `src/features/project/project-icon-button.tsx` — `agentBlockedReason` prop 추가, `aria-label` 이 툴팁과 같은 문구를 쓴다.
    - `src/widgets/editor-area/pane-tab-bar.tsx` — 터미널 탭 툴팁도 같은 helper 경유.
    - `src/shared/lib/native-notification-gate.ts` — `AgentCompletionCandidate.blockedReason?`(바인딩이 optional 이라 optional) → 완료 보고에 `blockedReason: BlockedReason | null` 로 정규화해 실어 나른다. 게이트는 값을 해석하지 않는다.
    - `src/app/providers/native-notification-provider.tsx` — `BLOCKED_REASON_BODY_KEY`(permission·question) + `AWAITING_INPUT_FALLBACK_BODY_KEY`(dialog·미상)로 본문 분기, `category` 를 `awaitingInput` 일 때 `agentAwaitingInput` 으로 전송.
    - `src/widgets/settings-view/settings-notification-section.tsx` — `settings.notificationsAgentAwaitingInput` 스위치(라벨+설명)를 "에이전트 작업 완료" 바로 아래에 추가, `notifyAgentAwaitingInput` 패치.
    - `src/widgets/settings-view/agent-hooks-project-list.tsx` — `AGENT_HOOKS_AGENTS` 에 opencode(user)·pi(user) 추가 + 행마다 `requiresTaideCli`. taide CLI 미설치 경고·토글 비활성화가 `delivery == Http`(gemini)에만 걸린다.
    - `src-tauri/resources/locales/{en,ko,ja}.json` — 신규 9키(`agent.blocked.{permission,question,dialog}` · `notification.agent{Permission,Question}Body` · `settings.agentHooksAgent{Opencode,Pi}` · `settings.notificationsAgentAwaitingInput{,Description}`), 기존 1키 값 정정(`settings.agentHooksUserLevelDescription` 의 경로 목록에 `~/.config/opencode`·`~/.pi` 추가).
    - `src-tauri/src/domain/locale/service.rs` — `MESSAGE_NAMESPACES` 의 `agent`·`notification`·`settings` 네임스페이스에 위 9키 등재(코드 로직 무변경).
    - `docs/features/agent-integration.md` **§1.5 신설** — 프론트 소비처 3곳 표, 대표 에이전트 선택 규칙, 사유 없는 `awaitingInput` 폴백, 알림 카테고리·본문 키 표, 설정 UI 가 `AGENT_SPECS` 의 거울이라는 점.
    - `docs/features/settings-ui.md` — §2.1.1 "스위치 8개"→"9개"(행 추가), §2.1.3 "5카테고리"→"6카테고리"(`agentCompleted` 는 Working→Idle 로 좁히고 `agentAwaitingInput` 행 신설), 두 카테고리로 나눈 이유 항목 추가.
  - **테스트** — `bun test` 전체 2445 pass / 0 fail(변경 전 2438 → 신규 7). 신규 파일 `src/shared/lib/agent-status-text.test.ts` 3건(차단 아닌 활동은 사유 무시 · 사유 3종 분기 · 사유 없는 `awaitingInput` 은 일반 키). `native-notification-gate.test.ts` +2건(사유를 완료 보고에 그대로 실음 · 사유 없는 로스터 항목은 `null` 정규화)과 기존 `toEqual` 2건에 `blockedReason: null` 반영. `native-notification-provider.test.ts` +2건(사유별 본문 3종 — dialog 는 일반 문구 폴백 · idle 은 사유가 있어도 완료 문구)과 awaitingInput 기존 테스트 이름 정정. `notification-text.test.ts` 는 무변경 — 그 모듈(본문 조각·제목 해석)은 이번 변경과 무관해 갱신할 것이 없었다.
  - **검증** — `bun run typecheck`(무오류) · `bun run lint`(0 error / 11 warning, 전부 기존 `useVirtualizer` 경고로 이번 변경 파일과 무관) · `bunx prettier --check "src/**/*.{ts,tsx}"`(전부 통과) · `bun test` 전체 · `cargo test -p taide --lib locale` 19 pass(세 로케일 키 집합 동일·보간 플레이스홀더 동일·`MESSAGE_NAMESPACES` 중복 없음 포함) · `cargo fmt --all -- --check` · `cargo clippy -p taide --all-targets -- -D warnings`(무경고) · bindings 재생성(`typescript_바인딩을_생성한다` pass, **산출물 무변경** — F 는 IPC 타입을 건드리지 않는다). 앱 실행·CLI 구동·커밋 없음. 사용자 홈의 에이전트 설정 파일은 읽지도 쓰지도 않았다.
  - **이탈(이유)**
    - **`features/project/agent-status-badge.tsx` 는 건드리지 않았다**(계약이 그 파일을 지목). 그 컴포넌트는 `aria-hidden` 도형 `<span>` 하나라 문구도 툴팁도 갖고 있지 않다 — 배지를 설명하는 툴팁은 배지를 감싸는 `sortable-project-icon.tsx` 의 `TooltipContent` 이고, 같은 줄을 터미널 탭 툴팁(`pane-tab-bar.tsx`)과 아이콘 `aria-label` 도 쓴다. 그래서 사유는 그 세 곳에 넣고, 문구 선택만 순수 함수로 뽑았다. 배지에 native `title=""` 를 다는 것은 디자인시스템 Tooltip 규칙 위반이라 택하지 않았다.
    - **문구 helper 를 features 가 아니라 `shared/lib` 에 뒀다.** widgets 2곳 + features 1곳이 쓰므로 FSD 의 "2곳 이상이면 shared 로 승격" 이 맞고, features 에 두면 widgets→features 참조는 되지만 같은 문자열 규칙이 레이어에 묶인다. 1파일 1컴포넌트 규칙상 배지 컴포넌트 파일에 비컴포넌트 export 를 얹지도 않았다.
    - **`AGENT_HOOKS_AGENTS` 에 `requiresTaideCli` 를 추가했다**(계약은 이름·라벨·스코프만 명시). §1.C 에서 codex 가 인밴드로 옮겨가 CLI 전제가 `delivery == Http` 에만 남았는데(§7.5 마지막 항), 기존 UI 는 모든 user 레벨 행을 CLI 설치 여부로 잠그고 있었다. opencode·pi 를 그대로 추가하면 CLI 없는 사용자에게 **설치 가능한 훅이 잠기고 거짓 경고**가 뜬다. 스코프와 같은 층위의 필드 하나로 표를 맞췄다(codex 도 함께 풀렸다).
    - **`settings.agentHooksUserLevelDescription` 값을 정정했다**(신규 키가 아니라 기존 키). "사용자 레벨(~/.codex 또는 ~/.gemini)" 이 다섯 에이전트 중 둘만 가리켜 사실과 어긋났다. 네 경로를 모두 적었다.
    - **알림 스위치에 설명 키를 하나 더 만들었다**(`…AgentAwaitingInputDescription`). 나머지 카테고리 스위치는 라벨만 있지만, 이 스위치는 바로 위 "에이전트 작업 완료" 와 무엇이 다른지가 라벨만으로는 보이지 않는다(배정이 "라벨·설명" 을 명시하기도 했다).
    - **`aggregateActivity` → `topPriorityAgent` 로 rename·반환형 변경.** 활동만 돌려주면 사유를 어느 에이전트에서 가져올지 알 수 없어, 도형과 사유가 서로 다른 세션을 가리킬 수 있다. 우선순위 규칙(`ACTIVITY_PRIORITY`)과 결과 활동은 동일하다.
    - **`notification.agentAwaitingInputBody` 문구는 그대로 뒀다.** 이제 dialog·미상 전용 폴백이지만 "권한 요청 또는 질문" 은 그 두 경우에 여전히 정확하고, 바꾸면 기존 문구를 아는 사용자에게 이유 없는 변화가 된다.
  - **미확인 · 후속**
    - **알림 제목의 폴백 키는 여전히 `notification.agentCompleted`("에이전트 작업 완료")다.** 제목은 보통 프로젝트 표시명이고 이 폴백은 `PROJECT.LIST` 캐시가 아직 답하지 않은 순간에만 쓰이는데, 그때 입력 대기 알림의 제목이 "작업 완료" 로 뜬다. 전용 제목 키는 계약에 없어 넣지 않았다 — 필요하면 `notification.agentAwaitingInput` 한 키로 해결된다.
    - **`agent-cli-status-row.tsx` 에는 opencode·pi 를 추가하지 않았다.** 그 행은 TAIDE 자신의 `taide` 심링크 설치 상태(`agent_cli_status`)를 보여주는 자리지 에이전트 CLI 설치 감지가 아니고, 백엔드에 `which opencode`/`which pi` 류의 감지 명령이 없다. 배정 조건("설치 감지가 있으면")이 충족되지 않았고 감지를 새로 만드는 것은 Rust 변경이라 범위 밖이다.
    - **`blockedReason` 의 실효 범위는 §1.C 의 인밴드 이벤트가 오는 에이전트뿐이다.** codex·pi·gemini 는 시그니처 표가 비어 있어 화면 판정으로는 사유가 생기지 않고, opencode 는 `Permission required` 를 봐도 `dialog` 로만 나간다 — 툴팁·알림이 `permission`/`question` 으로 갈리는 것은 플러그인·훅 이벤트가 실제로 도달할 때부터다(사용자 실기 확인 항목).
    - **`agent.blocked.*` 3키의 번역은 검수 전이다**(en/ko/ja 동시 작성). 렌더 화면(라이트·다크)에서의 줄바꿈·길이는 확인하지 않았다 — 앱 실행이 금지된 배정이다.
<!-- d60-record-F -->


- **메인 1차 검증 (2026-09-15, worktree taide-w3)** — `bun run verify` exit 0(bun test 2445/0 · cargo lib 1592 + 통합 30 · clippy 0 · prettier) · `bunx vite build` 0 · `bun run typecheck:e2e` 0.
- **렌즈 검토 (2026-09-15, wf `wf_87acc332`, sonnet·xhigh 3렌즈)** — major 0 · minor 4 · info 2. 메인 판정:

| id | 심각도 | 내용 | 판정 |
|----|------|------|------|
| G-1 | minor | `apply_hook_payload`(HTTP 훅 즉시 반영)가 `activity` 만 덮고 `blocked_reason` 을 남김 → 같은 이름 다중 세션 전환기에 사유·활동 불일치 가능 | **수용** — activity 강제 지점에서 `blocked_reason = None` + 테스트 |
| R-1 / G-2 | minor / info | 스캐너가 행을 모르는 상태의 첫 절대 CUP 를 "같은 행" 으로 봄. 대체화면 진입/이탈 시 행 리셋 없음 | **수용(부분)** — `CSI ?1049h/l` 에서 `cursor_row` 리셋 + 회귀 테스트(Claude 픽스처 첫 CUP 동작 고정). 휴리스틱 자체는 유지(탐침 근거) |
| B-1 | minor | 프론트 `AGENT_HOOKS_AGENTS.requiresTaideCli` 가 Rust `AGENT_SPECS.delivery` 를 수동 미러(되돌리기 시 드리프트) | **수용** — 훅 상태 IPC(`agent_hooks_status` 계열)에 에이전트별 `delivery`/`requiresTaideCli` 를 실어 프론트가 서버 값을 읽게 + bindings 재생성 |
| B-2 | minor | 탐침 문서의 "opencode `plugin/` vs `plugins/` 중복 로드" 캐비어트가 구현·문서에 미반영 | **수용(문서)** — 공식 문서 기준 전역 디렉토리 확인 결과를 agent-integration.md §4 에 1줄 기록, 설치는 `plugins/` 단일 |
| R-2 | info | `pi` comm 완전일치 오탐 가능성 | **기각** — 의도된 설계(미탐보다 오탐 방지는 cmdline 경로에서만), 실측 후 재논의(§4) |

- **검토 수정 완료 — Rust 절 (2026-09-15)** — 수용 4건 중 G-1 · R-1/G-2 · B-1 의 백엔드(타입·IPC·bindings) · B-2(문서). B-1 의 프론트 소비(`AGENT_HOOKS_AGENTS` 의 수동 미러 제거)는 TS 단계 몫이다.
  - **항목별 변경**
    - **G-1** — `service.rs` 에 순수 함수 `apply_hook_activity(agents, agent_name, activity)` 신설: 이름이 같은 행의 `activity` 를 덮으면서 `blocked_reason` 을 `None` 으로 함께 지운다. `hooks.rs::apply_hook_payload` 의 인라인 `map` 을 이 호출로 교체(동작 차이는 사유 초기화뿐). 브리지 페이로드는 활동 하나만 싣고 cwd 로 프로젝트에 매칭되므로, 세션 래치에서 온 사유를 남기면 끝난 차단이나 같은 에이전트의 **다른 세션**의 차단을 설명하게 된다 — `commands::resolve_state` 의 override 폴백이 이미 내놓는 답(`None`)과 같은 값으로 맞췄다.
    - **R-1 / G-2** — `terminal_scan.rs` 에 `NormalizedText::forget_row()` + `switches_screen_buffer(params)` + 상수 `CSI_PRIVATE_PREFIX`·`SCREEN_BUFFER_SWITCH_MODES = [1049, 1047, 47]`. `step_csi` 의 최종 바이트 `h`/`l` 이 화면 버퍼 전환 private 모드를 담고 있을 때만 `cursor_row = None`. 진입·이탈 양쪽 다 리셋한다(떠나온 버퍼의 행이라 어느 방향이든 stale). **휴리스틱 본체는 그대로**다 — 행 미상은 여전히 "같은 행"(공백)이고, 이 변경은 리셋 지점만 추가한다.
    - **B-1** — `AgentHooksStatus` 에 `requires_taide_cli: bool` 추가(specta `Type` 파생 그대로 → bindings `requiresTaideCli: boolean`). 값은 `service::requires_taide_cli(agent)` = 그 `AGENT_SPECS` 행의 `delivery == Http`. 기존 `uses_project_hook_override` 와 새 `requires_taide_cli` 는 **같은 술어를 복붙하지 않고** 사설 `delivers_over_http` 하나를 공유한다(둘은 뜻이 다르다 — 하나는 프로젝트 override 를 빌릴 자격, 하나는 CLI 전제). `agent_hooks_status`·`agent_hooks_install`·`agent_hooks_uninstall` 세 응답이 모두 싣는다.
    - **B-2** — `agent-integration.md` §4.5 opencode 항에 캐비어트 1줄: 공식 문서상 자동 로드되는 전역 디렉터리는 `plugins/` 단수형 하나이고 TAIDE 는 거기에만 쓴다. 프로젝트 레벨 `plugin/`·`plugins/` 중복 로드는 **스크래치 탐침 한정 관측**이라 설치 경로 근거로 쓰지 않는다.
    - 문서: `docs/ipc-contract.md` agent 절에 `AgentHooksStatus.requiresTaideCli`(값의 출처·프론트 표 복제를 없애려는 의도)와 G-1 의 사유 초기화 규약 추가. `agent-integration.md` §7.5 "CLI 존재 전제" 항에 그 전제가 이제 훅 상태 IPC 로 나간다는 한 줄.
  - **변경 파일** — `src-tauri/src/domain/agent/service.rs` · `hooks.rs` · `commands.rs` · `types.rs`, `src-tauri/src/infra/terminal_scan.rs`, `src/shared/api/bindings.ts`(재생성 산출물), `docs/ipc-contract.md`, `docs/features/agent-integration.md`.
  - **테스트** — `cargo test --workspace` **1597 pass / 0 fail**(R3 이후 1592 → 신규 5) + 통합 30. 신규: agent service 2건(`http_훅_활동_적용은_같은_이름_세션의_차단_사유를_함께_지운다` — 사유 `Some` 인 스냅샷에 적용 후 `None`, 같은 에이전트의 둘째 세션도 함께 덮이고 다른 에이전트 행은 불변 / `taide_cli_전제는_http_전달_에이전트에만_붙는다` — gemini 만 true, 그리고 `AGENT_SPECS` 전 행에 대해 `requires_taide_cli == (delivery == Http)` 기계 대조), 스캐너 3건(`대체화면_전환은_추적하던_행을_잊는다` — 진입 후 첫 CUP 는 공백으로 붙고 다음 행 이동은 개행·같은 행 재관측은 공백, 이탈도 같음 / `구형_대체화면_표기도_전환이고_그_밖의_private_모드는_아니다` — `?47h`·`?1047h` 는 리셋, `?25l`·`?2004h` 는 추적 유지 / `대체화면_진입은_claude_점멸_프레임의_첫_cup_판정을_바꾸지_않는다` — 진입 뒤 `BLINK_GLYPH_CHUNK` 의 정규화 텍스트가 새 스캐너의 `scan_once` 결과와 바이트 동일). **세 스캐너 테스트는 픽스를 빼면 전부 FAILED 로 떨어지는 것을 확인**하고 되돌렸다(회귀 포착 검증). 기존 Claude 경로 테스트(d-54 리플레이 타임라인·시그니처·정규화·점멸 프레임)는 **무수정 전부 통과**. `bun test` 2445 pass / 0 fail(신규 0 — 이 절은 TS 코드를 만지지 않는다).
  - **검증** — `cargo fmt --all` + `--check` · `cargo clippy --workspace --all-targets -- -D warnings`(무경고) · `cargo test --workspace` · bindings 재생성(`typescript_바인딩을_생성한다` pass, **산출물 변경 1건** — `AgentHooksStatus.requiresTaideCli`) · `bun run typecheck`(무오류) · `bun run lint`(0 error / 11 warning, 전부 기존 `useVirtualizer` 경고) · `bun run format:check`(통과 — `bindings.ts` 는 `.prettierignore` 대상) · `bun test` 전체. 앱 실행·CLI 구동·커밋 없음. 사용자 홈의 에이전트 설정 파일은 읽지도 쓰지도 않았다.
  - **이탈(이유)**
    - **G-1 을 호출부 인라인 수정이 아니라 순수 함수 추출로 했다.** `apply_hook_payload` 는 `AppHandle`·`AppState`·`AgentStore` 를 쥐고 있어 단위 테스트가 불가능하다. 배정이 요구한 "사유 `Some` 스냅샷 → 적용 후 `None`" 테스트를 쓰려면 덮어쓰는 규칙이 순수 함수여야 했다. 호출부는 세 줄로 줄었다.
    - **`SCREEN_BUFFER_SWITCH_MODES` 에 `1047` 도 넣었다**(배정은 `1049`, 필요 시 `47`). `?1047h/l` 은 xterm 이 같은 뜻으로 해석하는 셋째 표기이고 상수 한 항목이라, 둘만 넣고 셋째만 빠뜨릴 근거가 없다. 세 표기 전부를 테스트가 덮는다.
    - **스캐너 회귀 테스트가 2건이 아니라 3건이다.** 구형 표기(`47`·`1047`)를 상수에 넣은 이상 그 두 표기와 "화면 전환이 아닌 private 모드는 리셋하지 않는다"(`?25l`·`?2004h`)를 함께 고정하지 않으면, `switches_screen_buffer` 가 모든 `h`/`l` 을 리셋하도록 나중에 넓어져도 테스트가 잡지 못한다.
    - **`uses_project_hook_override` 의 본문을 `delivers_over_http` 호출로 바꿨다**(요청 범위 밖의 한 줄). `requires_taide_cli` 를 같은 술어의 복붙으로 만들면 B-1 이 지적한 미러링을 Rust 안에서 재현하게 된다. 동작·시그니처·기존 테스트는 그대로다.
    - **`AgentHooksStatus.requiresTaideCli` 에 `#[serde(default)]` 를 붙이지 않았다.** 이 타입은 Rust 가 항상 채워 내보내는 응답이라 기본값이 필요 없고, 붙이면 bindings 에서 optional 이 돼 프론트가 "값이 없을 때"를 다뤄야 한다 — 미러를 없애려는 필드에 미정의 상태를 만드는 것은 역효과다(형제 필드 `installed` 와도 일치).
    - **`delivery` 자체는 IPC 로 내보내지 않았다**(렌즈 판정 문구는 "`delivery`/`requiresTaideCli`"). 배정이 `requiresTaideCli` 를 지목했고, 프론트가 실제로 묻는 질문은 "이 행에 CLI 경고를 걸어야 하나" 하나뿐이다. `HookDelivery` enum 을 와이어에 올리면 프론트가 전달 방식으로 분기할 수 있게 되어 미러가 다른 모양으로 되돌아온다.
  - **미확인 · 후속**
    - **`AgentHooksStatus.requiresTaideCli` 는 아직 소비처가 없다.** `agent-hooks-project-list.tsx` 의 하드코딩 `requiresTaideCli` 를 이 필드로 바꾸는 것과, 그에 맞춘 `agent-integration.md` §1.5 "설정 UI" 문단(현재 "행마다 `requiresTaideCli` 를 들고 있다" = 프론트 거울 서술) 갱신은 **TS 단계에서 함께** 해야 한다. 그때까지 문서 §1.5 와 §7.5 가 서로 다른 시점을 말한다.
    - **세 훅 상태 커맨드가 필드를 싣는 것 자체는 단위 테스트가 없다**(`State<AppState>`·`AppHandle` 의존). 값의 정확성은 `service::requires_taide_cli` 테스트가, 표와의 일치는 `AGENT_SPECS` 전수 대조가 지킨다.
    - **행 추적 리셋은 화면 전환 시퀀스가 실제로 오는 경우에만 동작한다.** 스크롤·클램프로 인한 드리프트(§1.B 기록의 마지막 항)는 그대로 남아 있고, 이 수정은 그 중 "대체화면 경계" 한 종류만 없앤다.
- **검토 수정 완료 — TS 절 (2026-09-15)** — 수용 4건 중 B-1 의 프론트 소비 하나. G-1 · R-1/G-2 · B-2 는 Rust 절에서 끝났다.
  - **항목별 변경**
    - **B-1** — `agent-hooks-project-list.tsx` 의 `AGENT_HOOKS_AGENTS` 에서 `requiresTaideCli` 컬럼을 제거하고(`AgentHooksAgentOption` 타입·5행·`AgentHooksUserLevelRow` prop 동시), 행이 자기 훅 상태 쿼리에서 읽은 `status?.requiresTaideCli` 로 CLI 게이트를 판정하게 했다. `isCliMissing`·`isCliBlocking` 의 식 모양은 그대로 두고 그 입력만 프론트 표에서 서버 응답으로 바꿨다. 토글 잠금은 `isPending` 대신 `!status` — 응답이 없으면 그 행이 CLI 를 요구하는지 자체를 모른다. 표에 남은 컬럼은 `scope` 뿐이고, JSDoc 을 "거울" 서술에서 "`scope` 만 남는 이유 + CLI 전제는 응답에서 온다"로 다시 썼다.
    - 문서 — `agent-integration.md` §1.5 "설정 UI" 문단(Rust 절 기록이 TS 단계 몫으로 남긴 것)을 같은 내용으로 갱신. §1.5 와 §7.5 가 이제 같은 시점을 말한다.
  - **변경 파일** — `src/widgets/settings-view/agent-hooks-project-list.tsx`, `src/widgets/settings-view/agent-hooks-project-list.test.tsx`(신규), `docs/features/agent-integration.md`.
  - **테스트** — `bun test` **2449 pass / 0 fail**(TS 절 이전 2445 → 신규 4, 246 파일). 신규 `agent-hooks-project-list.test.tsx` 4건: ① 경고·잠금이 응답의 `requiresTaideCli` 를 따른다(gemini true·나머지 false → 경고 1개, gemini 잠김·codex 열림) ② **서버가 값을 옮기면 경고도 옮겨간다**(codex true·gemini false → 경고가 codex 행으로) ③ CLI 가 설치돼 있으면 요구하는 행도 경고 없이 열린다 ④ 상태를 모르는 행은 경고 없이 잠긴다(시드하지 않은 pi). ②는 게이트를 이름 기준으로 되돌리면(`agentName === 'gemini'`), ④는 `!status` 를 빼면 **각각 FAILED 로 떨어지는 것을 확인**하고 되돌렸다. IPC 는 목으로 잡지 않고 `setQueryDefaults({ enabled: false })` + 캐시 시드로 대체했다. `cargo test --workspace` 1597 + 통합 30 **무변경**(이 절은 Rust 를 만지지 않는다).
  - **검증** — `bun run typecheck`(무오류) · `bun run lint`(0 error / 11 warning, 전부 기존 `useVirtualizer` 경고) · `bun run format:check`(전체 통과) · `bun test` 전체 · `cargo fmt --all` 후 `--check`(Rust 파일 바이트 무변경, mtime 으로 확인) · `cargo clippy --workspace --all-targets -- -D warnings`(무경고) · `cargo test --workspace` · bindings 재생성(`typescript_바인딩을_생성한다` pass, `bindings.ts` md5 재생성 전후 동일 = 산출물 무변경). 앱 실행·CLI 구동·커밋 없음. 사용자 홈의 에이전트 설정은 읽지도 쓰지도 않았다(테스트가 IPC 를 타지 않는다).
  - **이탈(이유)**
    - **`isPending` 을 `!status` 로 바꿨다**(배정 문구 밖의 한 줄). 하드코딩일 때는 gemini 행이 상태 조회 실패 시에도 `requiresTaideCli = true` 덕에 잠겨 있었는데, 값을 응답으로 옮기면 그 경로가 열려 서버가 거부할 설치를 눌러 보게 된다. `!status` 는 `isPending`(데이터 없음)을 완전히 포함하면서 실패 경로를 덮고, `isPending` 구조분해가 사라져 미사용 변수도 남지 않는다.
    - **`scope` 는 프론트 표에 남겼다.** `AgentHooksStatus` 에 있긴 하지만, 어느 행 모양(프로젝트 섹션 vs 사용자 레벨 토글)을 마운트할지는 그 에이전트의 쿼리가 생기기 전에 정해져야 한다 — 응답에서 읽을 수 없는 유일한 컬럼이다.
    - **query hook 은 고치지 않았다**(배정은 "관련 query hook·테스트 갱신"). `agentHooksStatusQueryOptions` 가 응답 객체를 그대로 캐시에 싣고 설치·해제 mutation 의 `setQueryData` 도 서버 응답을 그대로 쓰므로, `requiresTaideCli` 는 추가 배선 없이 이미 최신이다. 필요 없는 변경을 만들지 않았다.
    - **테스트가 `mock.module` 을 쓰지 않는다.** `@entities/agent/agent.ipc` 목은 프로세스 전역·마지막 등록 승리라(`memory/test-conventions.md` §3) 다른 파일과 충돌한다. 캐시 시드 + 쿼리 비활성화로 같은 상태를 만든다.
    - **문서 파일을 건드렸다**(TS 도 Rust 도 아님). Rust 절 기록이 §1.5 갱신을 TS 단계 몫으로 명시했고, 두지 않으면 §1.5("행마다 `requiresTaideCli` 를 들고 있다")가 코드와 어긋난 채 남는다.
  - **미확인 · 후속**
    - **실기 렌더 미확인.** 앱 실행이 금지 범위라 happy-dom 하네스까지만 봤다. 이 변경은 클래스·마크업을 바꾸지 않아(경고 `span` 의 등장 조건만 바뀜) 라이트·다크 시각 회귀 대상은 아니지만, 실기 확인은 여전히 남아 있다.
    - **훅 상태 조회가 실패한 행은 잠긴 토글만 보여 준다.** 사유는 화면에 나오지 않는다(기존과 동일 — 상태 쿼리에는 에러 토스트가 없다). §1.E 후속으로 둔다.
    - **`delivery` 자체는 여전히 프론트에 없다**(Rust 절 결정). 전달 방식으로 UI 를 분기해야 하는 요구가 생기면 그때 와이어에 올릴지 다시 판단한다.


- **메인 2차 검증 최종 (2026-09-15, 검토 수정 반영 후, taide-w3)** — `bun run verify` exit 0(bun test 2449/0 · cargo lib 1597 + 통합 30 · clippy 0 · prettier) · `bunx vite build` 0 · `bun run typecheck:e2e` 0. 커밋 4분할(chore(locale)·feat(agent)·feat(agent-ui)·docs) → dev 위로 rebase → dev ff → main ff. 사용자 실기 대상: opencode 세션 배지(권한 다이얼로그 즉시 전환·유휴 복귀·스피너 중 Working 오판 없음), codex 인밴드 훅(`~/.codex/hooks.json` 설치 후 이벤트 도달 — 불가 시 delivery 1줄 롤백), 알림 "입력 대기" 문구·설정 토글, 설정 에이전트 목록 opencode·pi 행, pi·gemini 는 설치 후 실측.

## 4. 후속

- pi·gemini 실측(설치 후) — 시그니처·글리프 표 채우기.
- gemini 인밴드 전환.
- opencode 로컬 HTTP/SSE 구독(플러그인 없는 대안) — 포트 발견 수단 확보 시.
