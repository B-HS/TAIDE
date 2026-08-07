# 2026-08-07 — A3(에이전트 활동 배지) 구현 중 결정 사항

> 대상 플랜: `scratchpad/plan/wave1-a3-agent-activity.md`, 리서치: `docs/research/agent-status-badge.md`

## 1. pty 출력 활동(신호 b) 미구현 — 프로세스 상태(신호 a)만 사용

리서치 proposal 은 "pty 출력 활동 + 프로세스 상태" 조합을 권장했으나, 출력 활동을 관측하려면
`src-tauri/src/infra/pty.rs`(`PtySession` 에 `last_output_at` 추가)와
`src-tauri/src/domain/terminal/commands.rs`(`TerminalStore` 확장)를 수정해야 한다.
이 두 파일은 A3 의 소유 파일 목록에 없고, wave2-b2-ide 플랜도 terminal 도메인에서
`CLAUDE_CODE_SSE_PORT` 주입만 언급할 뿐 출력 활동 계측은 어떤 웨이브에도 배정되어 있지 않다.

**결정**: 이번 웨이브는 `domain/agent/` 내부에서 구할 수 있는 신호만 사용한다 — unix 는 `ps -o comm=,state=,args=`
로 얻은 프로세스 상태(`R`=활동), windows 는 `sysinfo::Process::status()`. `classify_activity`(히스테리시스:
`ACTIVITY_WORKING_HOLD_MS=2000`, `ACTIVITY_IDLE_QUIET_MS=6000`)로 idle/working 을 판정한다.
`awaitingInput` 은 리서치·플랜 확정대로 휴리스틱으로 절대 발행하지 않고 hooks 로만 발행한다.

**한계(문서화)**: CPU/state 신호는 "API 응답 대기"와 "사용자 입력 대기"를 구분하지 못한다(risk 2).
working 판정은 프로세스가 실제로 CPU 를 태우는 순간에만 정확하고, 그 외 구간은 idle/unknown 으로
수렴하는 보수적인 쪽으로 설계했다. pty 출력 활동 계측은 후속 웨이브(terminal/infra 도메인 소유자)에서
추가하는 것을 권장.

## 2. hooks 매핑은 세션이 아니라 프로젝트(cwd) 단위

Claude Code hook 의 `session_id` 는 Claude 세션 id 이며 TAIDE pty 세션 id 와 다르다. 정확한 세션별
매핑을 하려면 pty env 에 `TAIDE_SESSION_ID` 를 주입해야 하는데 이 역시 `infra/pty.rs` 를 건드려야 한다
(리서치 proposal 5단계·2차 항목, 이번 웨이브 범위 밖). 플랜 확정사항도 "cwd → 프로젝트 매칭"까지만
명시하므로, hook 이벤트 수신 시 매칭된 프로젝트의 **모든** 에이전트 세션 activity 를 동일하게
override 한다(`AgentHooksStore::set_project_override` → `AgentStore::agents_for` 전체에 적용).
override 는 15분(`HOOK_OVERRIDE_STALE_MS`) 이상 갱신되지 않으면 자동으로 무시되고 휴리스틱으로 복귀한다
(hooks 제거·비정상 종료 시 영구 고정 방지).

## 3. setQueryData 사용 근거 (query.md "무효화 우선" 예외)

`agent:state-changed` 이벤트는 프로젝트의 완전한 최신 `DetectedAgent[]` 를 push 로 실어 보낸다.
`useAgentStateSync`(`src/entities/agent/agent.query.ts`)는 `invalidateQueries` 대신
`queryClient.setQueryData(QUERY_KEY.AGENT.PROJECT(projectId), payload)` 로 캐시를 직접 채운다 —
서버가 이미 확정한 전체 상태를 그대로 신뢰할 수 있고, 재요청 왕복을 줄이기 위함이다(리서치 risk 7).

## 4. "소유 파일" 목록 밖이지만 불가피하게 수정한 파일

플랜의 "소유 파일" 목록은 `src/widgets/app-sidebar/`, `src/widgets/editor-area/tab-item.tsx 또는
해당 탭 컴포넌트`, `src/entities/agent/`, `src/features/settings/` 신규 컴포넌트로 되어 있었으나,
실제 배지·탭 표시 leaf 컴포넌트는 다음 경로에 있어 최소 변경으로 아래 파일도 함께 수정했다
(다른 웨이브 플랜 어디에도 이 파일들의 소유권이 없음을 확인 후 진행):

- `src/features/project/project-icon-button.tsx` — 사이드바 배지를 실제로 그리는 leaf 컴포넌트.
- `src/features/project/agent-status-badge.tsx` — 신규. 배지 SFC(`src/widgets/app-sidebar/` 산하가 아님).
- `src/widgets/editor-area/sortable-tab.tsx`, `src/widgets/editor-area/pane-tab-bar.tsx` —
  실제 탭 컴포넌트는 `src/features/tab/tab-item.tsx` 이지만(플랜 각주 "tab-item.tsx:47" 이 이 경로를
  가리킴), 세션→에이전트 매핑을 위해서는 이 두 파일에서 `sessionId` 와 프로젝트 에이전트 목록을
  대조해야 한다. `getTabIcon(kind, agent?)` 로 시그니처를 확장했고, 기존 호출부
  (`editor-area.tsx` 의 드래그 프리뷰)는 `agent` 인자가 optional 이라 무변경으로 호환된다.

## 5. i18n 치환 패턴 — `{{status}}`(react-i18next 네이티브) 사용

플랜은 "tab-item.tsx:47 의 `.replace('{title}', ...)` 관례를 따르라"고 안내했으나, 실제로 계약
단계에서 커밋된 `agent.*` 키(`locale/service.rs`)는 전부 이중 브레이스(`{{status}}`, `{{name}}`)로
작성되어 있다 — 이는 `explorer.*`/`search.*`/`git.*` 등 최근에 추가된 키들의 지배적 패턴과 동일하다
(react-i18next 의 네이티브 `t(key, { param })` 보간). `tab.*` 키만 과거에 단일 브레이스로 작성된
예외다. `.replace('{status}', ...)` 를 썼다면 실제 문자열이 치환되지 않는 버그가 됐을 것이므로,
스파인 파일(locale/service.rs)의 실제 표기를 따라 `t('agent.status.${activity}')`,
`t('agent.sessionTooltip', { name, status })` 형태로 구현했다.

## 6. `features/settings/agent-hooks-toggle.tsx` — 프레젠테이셔널 컴포넌트로 한정

`query-key.ts` 가 스파인 파일이라 새 QUERY_KEY 를 추가할 수 없다. 대신 기존 i18n 키
(`settings.agentHooks`, `settings.agentHooksHint`)만 사용하는 `checked/disabled/onCheckedChange`
props 기반 Switch 로 두어, FSD 의 "features 는 비즈니스 로직 없는 근간 컴포넌트" 원칙과도 맞춘다.
실제 설치/해제 API 호출(agent_hooks_install/uninstall, entities/agent/agent.ipc.ts 에 이미 준비됨)과
설정 화면 배선은 A5(후속) 담당.
