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
<!-- d60-record-R -->
<!-- d60-record-F -->

## 4. 후속

- pi·gemini 실측(설치 후) — 시그니처·글리프 표 채우기.
- gemini 인밴드 전환.
- opencode 로컬 HTTP/SSE 구독(플러그인 없는 대안) — 포트 발견 수단 확보 시.
