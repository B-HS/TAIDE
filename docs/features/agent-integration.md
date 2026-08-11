# 기능 — AI 에이전트 연동 (Claude Code 중심)

> FR-H. 에이전트 감지·외부 에디터(ctrl+g) 왕복·IDE MCP 서버.
> 확정 근거: `docs/research/claude-code-integration.md` — 공식/커뮤니티 출처 구분이 표기되어 있으며
> **[커뮤니티 출처] 항목은 구현 시 실측 검증 후 채택**한다(문서 함정 13).

## 1. 에이전트 감지 (FR-H1)

- macOS/Linux: pty master 의 **`process_group_leader()`**(portable-pty 내장, tcgetpgrp) →
  pid 의 프로세스명 해석. `comm` 이 `node` 인 경우(런타임 위 실행) `cmdline` 전체 검사
  (Linux comm 15자 잘림 주의). 감지 대상: `claude`, `codex`, `gemini` (+설정으로 추가 가능한 목록).
- Windows: `sysinfo` 로 셸 pid 의 후손 프로세스 트리 탐색(스냅샷 비용 — 1~2초 폴링 + pty 출력
  있을 때만 재검사).
- 폴링 주기: unix 1s(값싼 syscall), Windows 2s. 상태 변화 시에만 `agent:state-changed(projectId,
  sessionId, agent)` 이벤트 → 앱 사이드바 아이콘/배지(`layout-shell.md` §2.2)·터미널 탭 아이콘 갱신.
- 보조 신호: TAIDE 가 pty 에 심는 env 로 자기 터미널 여부 식별, `CLAUDECODE=1`(공식) 존재 시
  Claude Code 하위 셸로 판정.

## 2. 외부 에디터 왕복 — `taide` CLI (FR-H2)

Claude Code ctrl+g(2.0.10+)는 **$EDITOR/$VISUAL 로 지정된 에디터를 임시파일로 열고, 에디터
프로세스 종료를 완료 신호로 본다**(research §1 — 공식+강한 정황). 따라서 TAIDE 는
"인스턴스 재사용 + 즉시 exit" 런처면 조용히 깨진다. **VSCode 의 마커 파일 방식을 그대로 채택**한다
(research §2 원본 소스 확인).

### 2.1 `taide` CLI 헬퍼 (별도 소형 바이너리, `crates/taide-cli`)

```
taide [--wait|-w] <file> [<file>...]
1) --wait 면 tmpdir/taide-wait-<uuid> 마커 파일 생성
2) 로컬 IPC(유닉스 소켓/네임드 파이프)로 실행 중 앱 탐지
   - 있으면: {absPath, waitMarker} 전달 (자식 spawn 안 함)
   - 없으면: 앱을 detach spawn + argv 로 동일 payload (single-instance 플러그인이 첫 인스턴스로 중계)
3) 마커 파일 삭제까지 300ms 폴링 블록 → exit 0
```

- 요구사항 R1~R9(research §3.1 표)를 전부 만족해야 한다 — 특히 **조기 exit 금지**,
  stdout 오염 금지, 절대경로 정규화, 창 focus/raise.
- 앱 측: payload 수신 → 해당 파일을 **활성(또는 최근) 프로젝트의 새 탭**으로 열고 창 focus →
  **그 탭이 닫힐 때 마커 삭제**(저장이 아니라 탭 닫힘 — VSCode 동일. UI 에
  "저장 후 탭을 닫으면 Claude 에 반영됩니다" 힌트 표시).
- 안전망: 탭 열기 실패 시 즉시 마커 삭제, 앱 종료 시 미해결 마커 전부 삭제,
  CLI 타임아웃 상한(기본 30분) 옵션.
- 임시파일(tmpdir 하위) 탭에는 **포맷터/린터/LSP 를 붙이지 않는다** —
  `externalEditorContext` 의 `#` 주석 블록 구조를 깨면 안 됨(함정 9).
- `taide <file>` (wait 없이)는 단순 파일 열기 CLI 로도 동작 — 일반 용도.
- 설치: 설정 UI 에서 "CLI 설치"(`/usr/local/bin` 심링크 등) + EDITOR 설정 안내
  (`export EDITOR="taide --wait"`, git core.editor, Windows 는 절대경로 슬래시 표기).
- deep-link(`taide://`)는 wait 시그널링이 불가하므로 EDITOR 경로에 쓰지 않는다(브라우저 연동 전용).

### 2.2 Tauri 측

- `tauri-plugin-single-instance`(2.4.x): 두 번째 인스턴스 argv 수신 → 파일 열기 이벤트.
- 탭 닫힘 → `release_wait_marker(marker)` command 가 마커 삭제.

## 3. IDE MCP 서버 (확장 기능 — Claude Code 를 1급 시민으로)

공식 확인 사실(research §5.1): Claude Code CLI 는 `~/.claude/ide/<port>.lock`(0600/0700) 을 읽고
`ws://127.0.0.1:<port>` + `X-Claude-Code-Ide-Authorization` 헤더로 IDE MCP 서버에 자동 연결한다.
**TAIDE 가 이 서버를 구현하면 내장 터미널에서 `claude` 실행만으로** diff 뷰어·선택 영역 컨텍스트·
진단 공유가 활성화된다.

- Rust `ide_mcp` 모듈: 127.0.0.1 바인드(포트 10000~65535), CSPRNG hex 토큰, lock 파일 수명주기
  (시작 시 stale lock pid 청소, 종료 시 삭제), JSON-RPC 2.0.
- 도구 구현(비공식 프로토콜 — 실측 검증 필수): `openFile`, `openDiff`(TAIDE diff 탭, 블로킹),
  `getCurrentSelection`, `getOpenEditors`, `getWorkspaceFolders`, `getDiagnostics`(LSP 마커),
  `checkDocumentDirty`, `saveDocument`, `close_tab`. 알림: `selection_changed`, `at_mentioned`.
- pty 자식 env 에 `ENABLE_IDE_INTEGRATION=true`, `CLAUDE_CODE_SSE_PORT=<port>` 주입(커뮤니티 스펙 —
  검증 후 적용).
- 프로토콜이 비공식이므로 **실측 스모크 테스트를 CI/체크리스트에 포함**(버전 업 파손 감지).

## 4. hooks / statusline 브리지 (후순위)

- TAIDE 로컬 HTTP 엔드포인트로 Claude Code hook(`PostToolUse(Edit|Write)` → 트리/탭 리로드,
  `Stop`/`Notification` → OS 알림) 수신.
- **사용자 동의 UI 필수 + `.claude/settings.local.json`(gitignore) 에만 주입 + 제거 UI 제공**
  (팀 설정 오염 금지 — 함정 15).
- statusline 바이너리: fire-and-forget POST 후 즉시 종료(300ms 디바운스 함정) —
  컨텍스트 사용률·비용을 TAIDE 상태바에 표시.

## 5. 수명주기

- 감지 폴링은 터미널 세션 소유 태스크에 묶여 세션 종료 시 함께 중단.
- MCP 서버·마커 파일·lock 파일은 앱 종료 시 정리(Drop + 시그널 핸들러 이중화).

## 6. 범위

| 1차 | 2차 | 3차 |
|-----|-----|-----|
| 에이전트 감지(사이드바/탭 반영), `taide` CLI + `--wait` 마커 방식, single-instance 파일 열기 | IDE MCP 서버(diff·선택 컨텍스트·진단) | hooks/statusline 브리지, at-mention 단축키, `claude-cli://` 연계 |

## 7. 구현 세부 제약

> Phase 7.7-W3 컨벤션 정리(코드 주석 금지 원칙 적용)에서 `domain/agent`·`domain/ide` 의
> `///` 주석을 제거하며 옮긴 비자명한 제약 설명. 대상 파일·함수는 괄호로 표기.

### 7.1 활동 판정 히스테리시스 (`domain/agent/types.rs`)

- `ACTIVITY_WORKING_HOLD_MS`(2000ms): 프로세스가 "실행 중"으로 관측된 뒤에도 이 시간 동안은
  Working 을 유지한다(폴링 사이 관측 누락 보정).
- `ACTIVITY_IDLE_QUIET_MS`(6000ms): 마지막 활동 관측 이후 이 시간이 지나면 Idle 로 전이한다.
  그 사이 구간(hold~quiet)은 직전 상태를 유지하는 히스테리시스 구간이다.
- `HOOK_OVERRIDE_STALE_MS`(900000ms=15분): hooks 로 설정된 프로젝트 단위 활동 override 가
  이 시간보다 오래되면 무시하고 프로세스 신호 기반 휴리스틱으로 복귀한다(hooks 미설치·비정상
  종료 등으로 override 가 갱신되지 않는 경우의 안전망).

### 7.2 프로세스 활동 신호 (`domain/agent/service.rs::is_probe_active`)

프로세스가 지금 이 순간 CPU 위에서 실행 중인지("R" 상태)만을 활동 신호로 본다. sleeping 상태는
"API 응답 대기"와 "사용자 입력 대기" 양쪽에서 동일하게 관측되어 구분할 수 없으므로, sleeping 은
전부 활동 없음으로 취급한다. (이 한계 때문에 `AwaitingInput` 판정은 hooks 브리지가 전담한다 — §4)

### 7.3 hooks 항목 병합 (`domain/agent/service.rs::remove_taide_hook_entries` / `inject_taide_hook_entries`)

`remove_taide_hook_entries` 는 TAIDE 가 주입한 hook 항목(`HOOKS_URL_MARKER` 포함 URL)만 골라
제거하고, 사용자가 직접 추가한 다른 hook 은 그대로 둔다. `inject_taide_hook_entries` 는 먼저
제거 후 삽입하는 방식으로 **재주입해도 항목이 중복되지 않게(멱등)** 만든다. 두 함수 모두
`.claude/settings.local.json` 의 기존 설정을 보존한 채 병합해야 한다는 §4 의 "팀 설정 오염 금지"
제약을 구현한다.

### 7.4 IDE MCP 서버 (`domain/ide`)

- **토큰 비교** (`service.rs::constant_time_eq`): 인증 토큰 검증에 타이밍 사이드채널을 남기지
  않기 위한 상수시간 비교. `domain::agent::service::constant_time_eq` 와 로직이 동일하지만,
  도메인 경계를 넘어 재사용하지 않고 각 도메인에 독립적으로 둔다.
- **토큰 생성** (`service.rs::generate_auth_token`): CLI 와 동일한 형식(32자 소문자 hex, OS
  CSPRNG 기반)의 인증 토큰을 만든다. 신규 `rand` 의존성을 들이지 않고, 이미 동일 용도(hooks.rs)로
  쓰이는 uuid v4 를 재사용한다(`docs/acknowledge/2026-08-07-qa-batch-decisions.md` §3 신규 의존성
  결정과 일치).
- **경로 접근 차단** (`service.rs::ensure_path_within_any_project`): `openFile`/`saveDocument` 가
  프로젝트 루트 밖 경로에 접근하지 못하도록, file 도메인의 기존 검증(`ensure_within_root` 기반)을
  그대로 재사용한다(임의 파일 접근 차단).
- **열린 에디터 목록** (`service.rs::open_editors_snapshot`): 모든 프로젝트의 레이아웃에서 File
  탭만 모아 `getOpenEditors` 응답을 만든다. `is_active` 는 각 프로젝트 레이아웃의 focused pane 의
  active tab 기준으로 판정한다(여러 프로젝트가 열려 있어도 프로젝트별로 하나씩 active 가 나올 수
  있다).
- **진단 None vs 빈 배열** (`commands.rs::IdeStore::diagnostics`): 진단이 한 번도 push 된 적
  없으면 `None`(= "아직 준비 안 됨")을 반환한다. 빈 배열(진단 0건)과 "아직 모름"을 구분해
  `getDiagnostics` 호출자의 거짓 음성(오탐 없음으로 오인)을 방지한다.
- **탭 닫기 경로와의 정합** (`commands.rs::reconcile_closed_tab`): ClaudeDiff 탭이 (도구 호출
  경로가 아니라) 일반 탭 닫기 경로로 닫혔을 때, 그 탭에 매인 pending `openDiff` 요청을
  `TabClosed` 로 해소한다. layout 도메인의 모든 탭 닫기 경로(Tauri 커맨드·IDE 도구 핸들러 공용
  `close_tab_and_finish`)에서 반드시 호출되어야 하는 불변조건이다 — 누락하면 `openDiff` 가
  무기한 대기 상태로 남는다.
- **종료 시 즉시 중단** (`commands.rs::stop_server`): 앱 종료·`ide_stop` 공용 정리 경로. pending
  요청을 전부 해소하고 lockfile 을 지운 뒤 accept 루프와 커넥션 태스크를 즉시 종료(abort)한다.
  `openDiff`/`saveDocument` 는 무기한 블로킹 커맨드이므로, 이 경로에서 pending 요청이 반드시
  해소되어야 좀비 대기가 남지 않는다.
- **프로젝트 닫기 정리를 폴링으로 처리** (`commands.rs::reconcile_stale_pending`): 프로젝트가
  닫혀 더 이상 유효하지 않은 pending diff 요청을 주기적으로 정리한다. `project_close` 커맨드는
  다른 도메인 소유라 직접 후킹할 수 없어, "프로젝트 닫기 시 정리" 요구사항을 폴링 방식으로
  대신 만족시킨다.
- **selection 세팅 범위 제한** (`server.rs::tool_open_file`): `startText`/`endText`/
  `selectToEndOfLine`(텍스트 패턴으로 선택 영역을 지정하는 옵션)는 IDE MCP 1차 구현 범위에서
  구현하지 않는다 — 파일을 열고 프론트마다 유지되는 활성 selection 을 세팅하려면 프론트 에디터
  인스턴스 접근이 필요해, 서버 단독 구현으로는 불가능하다.
- **핸드셰이크 콜백 분리** (`server.rs::auth_callback`): 헤더 콜백을 생산하는 함수를
  `handle_connection` 과 핸드셰이크 단위 테스트가 동일 로직을 쓰도록 별도 함수로 분리했다
  (테스트-운영 드리프트 방지). `#[allow(clippy::result_large_err)]` 의 불가피성 사유는
  `docs/acknowledge/2026-08-07-qa-batch-decisions.md` 참고.
- **lockfile 디렉터리 결정** (`lockfile.rs::resolve_lockfile_dir`): `CLAUDE_CONFIG_DIR` 가
  설정돼 있으면 그 하위 `ide/`, 없으면 홈 디렉터리의 `.claude/ide/` 를 쓴다. env 값을 인자로
  받는 순수 함수로 유지해, 테스트에서 실제 프로세스 env 를 건드리지 않고 양쪽 분기를 검증한다.
- **lockfile 원자적 쓰기** (`lockfile.rs::write_lockfile_atomic`): lockfile 을 tmp 파일에 쓴 뒤
  rename 으로 원자적으로 교체한다. CLI 가 쓰다 만 lockfile 을 읽고 파싱 실패로 삭제해버리는
  경쟁 상태를 피하기 위함이다.

### 7.5 멀티 에이전트 hooks 확장 (`domain/agent/service.rs`, `domain/agent/commands.rs`)

> 7.10-W4 — Codex·Gemini hooks 브리지 확장에서 추가한 비자명한 제약.

- **이벤트 매핑은 에이전트별 분기** (`service.rs::map_hook_event_to_activity`): 같은 활동을 알리는
  이벤트 이름이 에이전트마다 다르다 — Claude 는 `Notification`, Codex 는 `PermissionRequest`,
  Gemini 는 `BeforeAgent`/`AfterAgent` 로 시작·종료를 알린다. 매핑 함수는 `agent_name` 을 1차
  분기 기준으로 삼는다. Codex 의 `PostToolUse` 는 (도구 실행이 재개됐다는 뜻이므로) `Working` 으로
  매핑해 직전의 `AwaitingInput` override 를 자연히 덮어쓴다 — 별도의 "override 해제" 코드 경로를
  두지 않고, 기존 override 값 자체가 최신 이벤트로 대체되는 방식으로 처리한다.
- **hook override 스코프는 (프로젝트, 에이전트) 쌍** (`commands.rs::AgentHooksStore`): 기존에는
  프로젝트 단위로만 override 를 쌓아 같은 프로젝트의 claude·codex 세션이 서로의 활동 판정을
  덮어썼다. 키를 `(ProjectId, agent_name)` 로 확장해 에이전트별로 독립된 override 를 유지한다.
  `resolve_activity` 의 override 소비·해제(`fresh_project_override`/`clear_project_override`)도
  동일 키로 조회한다.
- **설치 스코프는 에이전트 정체성으로 결정** (`service.rs::hook_scope_for_agent`): Claude 는
  프로젝트 파일(`.claude/settings.local.json`, gitignore 대상)에 설치하지만 Codex·Gemini 는
  프로젝트 파일이 커밋 대상이라 오염 위험이 있어 사용자 레벨(`~/.codex/hooks.json`,
  `~/.gemini/settings.json`)에만 설치한다 (`docs/acknowledge/2026-08-11-qa5-batch-decisions.md`).
- **사용자 레벨 경로는 순수 함수** (`service.rs::user_level_hooks_path`): `lockfile.rs::resolve_lockfile_dir`
  와 동일 패턴 — `home_env: Option<&str>` 를 인자로 받아, 테스트에서 실제 프로세스 env 를 건드리지
  않고 양쪽 에이전트 분기를 검증한다.
- **마커 탐지는 문서 전체 재귀 스캔** (`service.rs::has_taide_marker_anywhere`): Codex·Gemini 의
  실제 hook 항목(`type: "command"`, shim 커맨드라인에 hook URL 이 인자로 박힘)은 특정 JSON 키
  구조에 의존하지 않고 문서 전체에서 `HOOKS_URL_MARKER` 문자열을 재귀적으로 찾는 관대한 탐지로
  판정한다. 스키마가 바뀌거나 사용자가 파일을 수기로 편집해도(키 이름·중첩 구조가 달라져도)
  상태 조회(`agent_hooks_status`)가 계속 동작하게 하기 위함이다.
- **주입은 기대 형태가 아니면 교체한다** (`service.rs::inject_taide_managed_entries`): 사용자가
  손으로 편집한 설정 파일은 `hooks` 가 객체가 아니거나 이벤트 값이 배열이 아닐 수 있다. 기대
  형태가 아니면 패닉 대신 빈 객체/배열로 교체한 뒤 주입한다.
- **command hook 커맨드 문자열의 인용 규칙** (`service.rs::build_command_hook_shell_command`):
  Codex·Gemini 는 `type: "http"` 를 지원하지 않고 `type: "command"` 셸 커맨드만 실행하므로,
  shim(`taide-cli hook --url <u>`)이 페이로드를 로컬 서버로 중계한다. 바이너리 경로와 URL 을
  각각 큰따옴표로 감싸는데, 경로는 공백을 포함할 수 있고(Windows `Program Files`) URL 은
  `&`/`?` 를 포함하기 때문이다.
- **command hook timeout 은 에이전트별 단위가 다르다** (`service.rs::user_level_hook_command_timeout`):
  Codex 는 `timeout` 을 초 단위로, Gemini 는 밀리초 단위로 해석한다. 두 사용자 레벨 hooks 파일은
  이 값을 서로 바꿔 쓸 수 없어 별도 상수(`CODEX_HOOK_COMMAND_TIMEOUT_SECONDS`,
  `GEMINI_HOOK_COMMAND_TIMEOUT_MS`)로 분리했다.
- **사용자 레벨 파일 정리는 열린 프로젝트 목록과 무관** (`hooks.rs::remove_taide_hooks_from_user_level_files`):
  Codex·Gemini 는 사용자 레벨 설정 파일을 1개씩만 가지므로(프로젝트별이 아님) OFF 전이 시
  열린 프로젝트가 0개여도 항상 정리 대상이다.
- **재부팅 후 사용자 레벨 hook URL 자가 치유** (`hooks.rs::reconcile_user_level_hooks`): hooks 서버는
  기동마다 랜덤 포트·새 토큰으로 뜨므로(`ensure_hooks_server_started`), 이전 실행에서
  `~/.codex/hooks.json`·`~/.gemini/settings.json` 에 박아 둔 커맨드 문자열은 재시작 후 죽은
  포트를 가리킨다. `reconcile_installed_hooks`(부팅·`agent_hooks_enabled` ON 전이·프로젝트
  열기 시 호출)가 Claude 의 프로젝트 파일 갱신에 이어 Codex·Gemini 의 사용자 레벨 파일도
  훑어, taide 마커가 있는데 현재 서버 기준 커맨드와 다르면(`has_command_hook_entries_for_command`)
  새 URL 로 재주입한다.
- **사용자 레벨 설치는 CLI 존재를 전제로 한다** (`commands.rs::agent_hooks_install`): command
  hook 이 실행하는 shim 커맨드라인은 `TAIDE_CLI_TARGET_PATH`(`/usr/local/bin/taide` 등)를
  절대경로로 참조한다. CLI 가 심볼릭 링크로 설치돼 있지 않으면 모든 hook 이벤트가 실행 실패로
  조용히 죽으므로, 설치 전에 `resolve_cli_install_status().installed` 를 확인해 미설치면
  설치를 거부한다.
- **uninstall 은 taide 항목이 없으면 파일을 새로 만들지 않는다** (`commands.rs::agent_hooks_uninstall`):
  프로젝트·사용자 레벨 양쪽 모두 기록된 taide 마커가 있을 때만 쓰기를 수행한다. 존재하지 않는
  파일(설치한 적 없는 상태)에 uninstall 을 호출해도 빈 JSON 파일이 새로 생기지 않는다.
