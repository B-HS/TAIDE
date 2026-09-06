# 기능 — AI 에이전트 연동 (Claude Code 중심)

> FR-H. 에이전트 감지·외부 에디터(ctrl+g) 왕복·IDE MCP 서버.
> 확정 근거: `docs/research/claude-code-integration.md` — 공식/커뮤니티 출처 구분이 표기되어 있으며
> **[커뮤니티 출처] 항목은 구현 시 실측 검증 후 채택**한다(문서 함정 13).

## 1. 에이전트 감지 (FR-H1)

> 정본 계약: `docs/acknowledge/2026-09-06-d54-agent-activity-signals-contract.md`.
> **"누가 돌고 있나"(신원)와 "지금 무엇을 하고 있나"(활동)는 완전히 다른 경로**다 — 신원만
> 프로세스 조회로 얻고, 활동은 그 세션이 자기 pty 로 내보낸 것에서만 읽는다.

### 1.1 신원 — 어떤 세션이 어떤 에이전트를 돌리는가

- macOS/Linux: pty master 의 **`process_group_leader()`**(portable-pty 내장, tcgetpgrp) →
  pid 의 프로세스명 해석. `comm` 이 `node` 인 경우(런타임 위 실행) `cmdline` 전체 검사
  (Linux comm 15자 잘림 주의). 감지 대상: `claude`, `codex`, `gemini`(`KNOWN_AGENT_NAMES`).
- **`ps` 는 이름 해석에만 쓴다**(d-54). `AgentStore` 가 `pid -> Option<&'static str>` 캐시를 들고
  (`process_names`), `detect_agents_for_pids_blocking` 은 **캐시에 없는 pid 만** 모아
  `ps -o pid=,comm=,args= -p <pid1,pid2,...>` 를 한 번 부른다. "이 pid 는 에이전트가 아니다"(`None`)도
  캐시되므로 fork 는 세션의 전경 pid 가 실제로 바뀐 틱에만 일어난다(이전에는 프로젝트당 500ms 마다
  1회). 캐시 항목은 전경 pid 집합에서 빠지는 순간 축출된다(`retain_process_names`). pty 세션이 0 인
  프로젝트는 프로세스 조회 자체를 건너뛴다.
- `ps` 의 `state` 열은 **제거했다**. `R`/`S` 는 "모델 응답 대기"·"권한 응답 대기"·"유휴"를 전부
  같은 `S` 로 보여 줘 셋을 구분할 수 없었고, 그것이 권한 다이얼로그가 떠 있는데도 유휴 배지가
  남던 원인이다(계약 §0.1).
- Windows: `sysinfo` 로 셸 pid 의 후손 프로세스 트리 탐색(스냅샷 비용 — 폴링 + pty 출력 있을 때만
  재검사). 후손 탐색이라 pid 단위로 캐시할 답이 없어 unix 의 이름 캐시는 적용하지 않는다.
- 폴링 주기: unix 500ms(`AGENT_POLL_UNIX_MS`), Windows 2s(`AGENT_POLL_WINDOWS_MS`). 상태 변화 시에만
  `agent:state-changed(projectId, agents)` 이벤트 → 앱 사이드바 아이콘/배지(`layout-shell.md` §2.2)·
  터미널 탭 아이콘 갱신. 이벤트·타입 계약은 d-54 에서도 무변경이다.
- 초안의 보조 신호(pty 자기 식별 env·`CLAUDECODE=1` 검사)는 도입하지 않았다 — 신원 판정은
  `domain/agent/service.rs::detect_agent_name` 의 프로세스명(comm)/cmdline 매칭 단독이다.

### 1.2 활동 신호 4종 (+ 사용자 입력)

세션의 pty 출력은 `infra::terminal_scan` 스캐너(`terminal.md` §5.2)가 한 번 훑어
`ScanOutcome { events, text, overlap }` 으로 돌려주고, `lib.rs` 가 조립한
`terminal::commands::PtySessionObservers` 가 그것을 `agent::commands::record_session_scan` 으로
흘려보낸다. 신호는 세션당 하나의 `AgentSessionSignals` 레코드에 쌓인다(`AgentStore.signals`, `session_id` 키 —
그 틱에 에이전트가 감지되지 않은 세션의 레코드는 `prune_signals` 가 지운다).

| # | 신호 | 출처 | 효과 |
|---|------|------|------|
| 1 | **인밴드 이벤트** | OSC 777 `notify;taide-agent;{"v":1,"agent":…,"event":…}`(§4 의 훅이 심는다) | `permission_request`·`question_asked` → 차단 래치 ON(`BlockedSource::Event`) / `tool_complete` → 래치 OFF / `stop`·`stop_failure` → 래치 OFF + 유휴 힌트 / `idle_prompt` → 무시 |
| 2 | **타이틀 글리프** | OSC 0/2 의 첫 글자(`parse_title_glyph`) | `◐`/`◑` → `TITLE_WORKING_FRESH_MS` 동안 Working 증거 / `✳` → 유휴 힌트(단독으로 Idle 을 강제하지 않는다) |
| 3 | **다이얼로그 시그니처** | 정규화 텍스트(`CLAUDE_DIALOG_SIGNATURES`) | 문구가 보이면 차단 래치 ON(`BlockedSource::Dialog`) |
| 4 | **실질 출력** | 정규화 텍스트에서 공백·점멸/스피너 글리프를 뺀 문자 수 ≥ `SUBSTANTIVE_OUTPUT_MIN_CHARS` | Working 증거 + **차단 래치 OFF** |
| — | **사용자 입력** | `pty_write` 바이트 도착(`record_session_input`) | 래치 OFF + 에코 억제 창 시작 |

- **다이얼로그 시그니처는 정규화 텍스트에서만 복원된다.** Claude Code 는 다이얼로그 산문을
  단어마다 `CSI n G`(열 이동)를 끼워 그리므로 원시 바이트 문구 매칭은 불가능하다. 스캐너가 열
  이동을 공백 1개로 치환한 뒤에야 `Do you want to proceed?` 가 하나의 문자열이 된다.
- 표는 `CLAUDE_DIALOG_SIGNATURES`(`Do you want to proceed?` · `Would you like to proceed?` ·
  `Esc to cancel`) 이고 **대소문자 정확 일치**다 — 작업 중 푸터는 소문자 `esc to interrupt` 라
  대소문자 무시 매칭이면 모든 작업 중 세션이 차단으로 읽힌다. codex·gemini 문구는 미검증이라
  표가 비어 있다(`dialog_signatures_for` — 추측으로 채우지 않는다).
- **청크 경계**: 스캐너가 직전 청크의 정규화 텍스트 꼬리(`TEXT_OVERLAP_BYTES`)를
  `ScanOutcome.overlap` 으로 함께 준다. `find_dialog_signature_across_boundary` 는 꼬리·머리를
  각각 그 시그니처 길이 - 1 로 잘라 이어 붙이므로 **경계를 가로지른 문구만** 매치되고, 이미
  꼬리에 통째로 남아 있는(=사용자가 방금 답한) 문구로는 다시 래치되지 않는다.
- **에코 억제**: 마지막 입력으로부터 `ECHO_SUPPRESS_MS` 안의 출력은 사용자가 친 글자를 터미널이
  되그린 것이므로 **실질 출력(4)에서만** 제외한다. 이벤트(1·2)는 에이전트 자신이 쓴 것이라 창과
  무관하고, 다이얼로그 시그니처(3)도 창 안에서 읽는다 — 연쇄 승인에서 다음 다이얼로그는 답변
  직후(에코 창 안)에 그려지고 그 뒤로는 점멸만 흐르므로, 이 프레임을 건너뛰면 그 세션은 영영
  차단으로 읽히지 않는다. 대가는 사용자가 시그니처 문구를 프롬프트에 직접 타이핑하면 래치가
  걸리는 것인데, 그것은 다음 실질 출력이 곧바로 푼다(놓친 다이얼로그는 스스로 낫지 않는다).
- **한 청크 안에서는 차단 증거가 이긴다**(`apply_scan_to_signals`). 실질 출력이 래치를 풀되 **그
  청크의 이벤트가 방금 세운 래치는 풀지 않고**(`PermissionRequest` 훅은 다이얼로그가 그려지는
  순간 발화하므로 OSC 와 프레임 앞부분이 같은 청크에 실린다), 다이얼로그 판정은 맨 마지막에
  건다 — 다이얼로그 프레임 자체가 실질 출력이라 순서를 뒤집으면 방금 세운 래치를 자기가 지운다.
  권한 대기 중에는 600ms 마다 48바이트짜리 `⏺` 점멸만 흐르는데 이것은 비실질 출력이라 래치를
  풀지 못한다 — 이 비대칭이 감지의 핵심이다.
- **OSC 9 알림**(`ScanEvent::Notification9`)은 유휴 힌트로만 기록한다 — 어떤 세션이든 "무언가
  끝났다" 는 뜻으로 읽되, 힌트는 §1.3 의 조용함 기준을 앞당길 뿐 단독으로 Idle 을 만들지 않는다.

### 1.3 판정 — `service::classify_session(signals, previous, now)`

순수 함수이며 위→아래 우선순위로 첫 번째로 맞는 것을 돌려준다.

1. 차단 래치 ON → `AwaitingInput`
2. 타이틀 글리프가 `Working` 이고 `TITLE_WORKING_FRESH_MS` 이내 → `Working`
3. 실질 출력이 `ACTIVITY_WORKING_HOLD_MS` 이내 → `Working`
4. 마지막 이벤트가 `tool_complete` 이고 `ACTIVITY_WORKING_HOLD_MS` 이내 → `Working`
5. 신호가 하나도 없음 → `Unknown`
6. 유휴 힌트가 있고 마지막 신호 이후 `ACTIVITY_WORKING_HOLD_MS` 이상 조용 → `Idle`
7. 마지막 신호 이후 `ACTIVITY_IDLE_QUIET_MS` 이상 조용 → `Idle`
8. 그 외 → `previous`(히스테리시스 — 폴링 틱마다 배지가 떠는 것을 막는 구간)

`poll_agents` 는 기존 틱에서 세션마다 이 함수를 부르고(`AgentStore::classify_session_activity`),
`previous` 로 그 세션의 직전 `DetectedAgent.activity` 를 넘긴다. 결과는 종전대로
`agents_changed` diff 를 지날 때만 `agent:state-changed` 로 나간다.

### 1.4 상수 (`domain/agent/types.rs`)

| 상수 | 값 | 근거 |
|------|-----|------|
| `ACTIVITY_WORKING_HOLD_MS` | 2,000 | 관측 누락 보정 — Working 증거 하나가 다음 틱까지 유효 |
| `ACTIVITY_IDLE_QUIET_MS` | 4,000 | 점멸 600ms·스피너 ≤1s 보다 충분히 크되 유휴 진입은 빠르게(6,000 에서 축소) |
| `TITLE_WORKING_FRESH_MS` | 3,000 | `◐`/`◑` 교대 주기 1~2s 의 2배 — 다이얼로그 동안 동결된 타이틀을 살아 있는 것으로 오인하지 않는다 |
| `ECHO_SUPPRESS_MS` | 300 | 키 입력의 에코가 돌아오는 시간 |
| `SUBSTANTIVE_OUTPUT_MIN_CHARS` | 2 | 점멸 1글자 프레임을 출력으로 세지 않는 최소선 |
| `TEXT_OVERLAP_BYTES` | 128 | 청크 경계 문구 복원용 꼬리(스캐너 소유, `types.rs` 가 재export) |
| `AGENT_PROTOCOL_VERSION` | 1 | 인밴드 이벤트 봉투 버전 = `TAIDE_AGENT_PROTOCOL_VERSION` 값 |
| `AGENT_OSC_SENTINEL` | `taide-agent` | OSC 777 알림 중 TAIDE 이벤트를 고르는 title(스캐너 소유) |
| `AGENT_OSC_MARKER` | `notify;taide-agent;` | hook 항목의 TAIDE 소유 판정에 쓰는 페이로드 접두사(§4.3) |
| `HOOK_OVERRIDE_STALE_MS` | 900,000 | codex·gemini HTTP override 의 유효 기간(§4.4) |

## 2. 외부 에디터 왕복 — `taide` CLI (FR-H2)

Claude Code ctrl+g(2.0.10+)는 **$EDITOR/$VISUAL 로 지정된 에디터를 임시파일로 열고, 에디터
프로세스 종료를 완료 신호로 본다**(research §1 — 공식+강한 정황). 따라서 TAIDE 는
"인스턴스 재사용 + 즉시 exit" 런처면 조용히 깨진다. **VSCode 의 마커 파일 방식을 그대로 채택**한다
(research §2 원본 소스 확인).

### 2.1 `taide` CLI 헬퍼 (별도 소형 바이너리, `crates/taide-cli`)

```
taide [--wait|-w] <file> [<file>...]
1) --wait 면 tmpdir/taide-wait-<uuid> 마커 파일 생성
2) 앱을 **항상 detach spawn** + argv 로 {absPath, waitMarker} payload — 이미 실행 중이면
   tauri-plugin-single-instance 가 두 번째 인스턴스의 argv 를 가로채 기존 창으로 중계한다.
   (초안의 "로컬 IPC 로 실행 중 앱 탐지 후 spawn 생략" 최적화는 도입하지 않았다 — 매 호출마다
   앱 바이너리를 한 번 더 spawn 하는 비용을 single-instance 중계의 단순성과 맞바꾼 트레이드오프)
3) 마커 파일 삭제까지 300ms 폴링 블록 → exit 0
```

- 요구사항 R1~R9(research §3.1 표)를 전부 만족해야 한다 — 특히 **조기 exit 금지**,
  stdout 오염 금지, 절대경로 정규화, 창 focus/raise.
- 앱 측: payload 수신 → 해당 파일을 **활성(또는 최근) 프로젝트의 새 탭**으로 열고 창 focus →
  **그 탭이 닫힐 때 마커 삭제**(저장이 아니라 탭 닫힘 — VSCode 동일. UI 에
  "저장 후 탭을 닫으면 Claude 에 반영됩니다" 힌트 표시).
  - **대상 프로젝트 결정(2026-09-05 수정, `entities/agent/external-open-target.ts`)**: 경로를 품은
    프로젝트(가장 긴 root 우선) → 활성 프로젝트 → 첫 프로젝트. 열린 프로젝트가 0개일 때만
    `app.openProjectFirst`. 이전 구현은 "품은 프로젝트" 만 허용해 tmpdir 임시파일이 항상 이 토스트로
    떨어졌다(`bug/2026-09-05-ctrl-g-temp-file-open-project-first.md`).
  - **루트 밖 경로의 Rust 경계**: CLI 로 명시 전달된 경로만 `AppState::cli_opened_paths` 허용 목록에
    정규화해 기록한다(진입점은 cold-start argv·single-instance 중계 2곳 → `queue_external_open` 단일
    함수, IPC 로는 추가 불가). `layout_open_tab`·`file_open`·`file_save`·`file_read_raw` 는
    `root_guard::resolve_owning_project_or_cli_opened` 를 타서 그 경로만 통과시키고, 나머지 커맨드
    (IDE MCP `openFile`·파일 트리 변경·미러)는 엄격 경계를 유지한다. 허용 목록은 프로세스 수명이며
    재시작 후 복원된 임시파일 탭은 `Forbidden` 안내를 보인다(Claude Code 가 지우는 파일이라 닫으면 끝).
  - **`--wait` 요청은 preview 가 아닌 고정 탭**으로 연다. `layout::service::open_tab` 은 새 preview
    탭이 기존 preview 탭을 자리 교체하는데 교체는 닫힘이 아니라 마커가 해제되지 않기 때문. 열리면
    `app.externalEditorTabHint` 토스트를 띄운다.
- 안전망: 탭 열기 실패 시 즉시 마커 삭제, 앱 종료 시 미해결 마커 전부 삭제,
  CLI 타임아웃 상한(기본 30분) 옵션.
- 임시파일(tmpdir 하위) 탭에는 **포맷터/린터/LSP 를 붙이지 않는다** —
  `externalEditorContext` 의 `#` 주석 블록 구조를 깨면 안 됨(함정 9). 구현은
  `editor-pane.tsx` 의 `isOutsideProjectRoot`(경로가 프로젝트 root 밖이면 LSP 세션·저장 시 코드
  액션·format-on-save·hot-exit 미러를 모두 끈다 — 2026-09-05).
- `taide <file>` (wait 없이)는 단순 파일 열기 CLI 로도 동작 — 일반 용도.
- 설치: 설정 UI 에서 "CLI 설치"(`/usr/local/bin` 심링크 등) + EDITOR 설정 안내
  (`export EDITOR="taide --wait"`, git core.editor, Windows 는 절대경로 슬래시 표기).
- deep-link(`taide://`)는 wait 시그널링이 불가하므로 EDITOR 경로에 쓰지 않는다(브라우저 연동 전용).

### 2.2 Tauri 측

- `tauri-plugin-single-instance`(2.4.x): 두 번째 인스턴스 argv 수신 → `queue_external_open`(마커 등록·
  허용 목록 기록·큐 적재) → `agent:external-open` 이벤트. cold-start argv 는 같은 함수를 부르고 이벤트
  없이 부팅 drain 에 맡긴다.
- 탭 닫힘 → `agent_release_marker(marker)` command 가 마커 삭제.

### 2.3 내장 터미널 `EDITOR`/`VISUAL` 자동 주입

내장 터미널에서 실행한 Claude Code 의 ctrl+g 가 별도 설정 없이 TAIDE 로 열리도록,
PTY 스폰 시 `EDITOR` 와 `VISUAL` 을 같은 값으로 주입한다. 주입 지점은 `pty_spawn` 의 extra-env 훅
(`terminal::commands::PtySpawnEnvProvider`) 하나뿐이며, `lib.rs::pty_spawn_env_provider` 가
`ide::store::claude_terminal_env`(SSE 포트)와 `agent::commands::editor_terminal_env`(EDITOR·VISUAL)를
이어 붙인다 — `infra/pty.rs` 의 `build_command` 는 이 목록을 그대로 적용할 뿐 아무것도 하드코딩하지 않는다.

- **값**: `<taide CLI 절대경로> --wait` — **인용하지 않는다.** 소비자인 Claude Code 는 이 값을
  셸로 파싱하지 않고 공백으로 split 한 뒤 첫 토큰을 그대로 실행 파일로 spawn 하므로
  (`spawnSync(argv0, [...rest, tmpfile])`, shell 옵션 없음 — claude 2.1.251 바이너리 확인),
  따옴표를 붙이면 `'/usr/local/bin/taide'` 라는 이름의 파일을 찾다가 ENOENT 로 실패한다.
  조립은 순수 함수 `agent::service::build_editor_env_entries(cli_path)` 가 담당한다.
- **`VISUAL` 도 같은 값으로 주입한다** (사용자 결정 2026-08-30,
  `acknowledge/2026-08-30-usability-batch-decisions.md`): Claude Code 는 `$VISUAL` 을 `$EDITOR`
  보다 먼저 보므로, EDITOR 만 주입하면 `VISUAL` 을 export 해 둔 사용자에게 ctrl+g 가 계속 vim 으로
  열린다. 부모 프로세스 환경의 기존 값은 의도적으로 덮는다 — 최종 우선권은 아래 "셸 rc" 항목대로
  사용자 rc 에 있다.
- **경로 우선순위**: `/usr/local/bin/taide` 심링크가 유효하고(dangling 아님) 그 타깃이 우리 CLI
  사이드카(`taide-cli`)이면 그 경로, 아니면 실행 중 앱 번들의 CLI 사이드카
  (`.../Contents/MacOS/taide-cli`) 절대경로. 둘 다 없으면(사이드카 없는 dev 빌드)
  **주입을 생략**한다 — 스폰을 실패시키지 않는다. 소유권 판정은 `agent_cli_uninstall` 과 같은
  `service::is_cli_symlink_owned` 이다 — TAIDE 가 설치하지 않은 동명 바이너리를 EDITOR 로 삼지 않는다.
- **공백이 든 경로는 생략**: 소비자가 공백으로 split 하므로 어떤 인용으로도 표현할 수 없다.
  공백 없는 `/usr/local/bin/taide` 심링크가 우선이므로 정상 설치 경로에서는 발생하지 않는다.
- **셸 rc 가 이긴다**: `.zshrc` 등의 `export EDITOR=...`/`export VISUAL=...` 은 스폰 **이후**
  실행되므로 주입값을 덮어쓴다. 이는 의도된 동작이다 — 사용자가 rc 에 명시한 에디터가 항상 우선한다.
  (부모 프로세스 환경의 값은 rc 와 달리 존중하지 않는다 — 위 `VISUAL` 항목의 결정.)
- **외부 터미널**은 이 주입을 받지 않는다. 직접 `export EDITOR="taide --wait"` 를 설정해야 한다
  (`CliInstallStatus.editorEnvHint` 가 이 문자열을 그대로 노출한다).
- **팔레트 커맨드** `cli.connectExternalEditor`("Claude Code Ctrl+G 를 TAIDE 로 연결", macOS 한정):
  `agent_cli_status` 로 확인해 미설치·dangling 이면 `agent_cli_install` 을 실행하고, 그 반환 상태가
  실제로 설치됨(dangling 아님)일 때만 "새로 여는 터미널부터 Ctrl+G 가 TAIDE 로 열립니다"
  (`settings.cliExternalEditorConnected`) 를 안내한다. 관리자 프롬프트를 취소하면 설치는 조용한
  no-op(Ok) 이라 상태가 그대로 돌아오므로, 이 경우 `settings.cliInstallFailed` 를 띄운다.
  설치 성공 후 `QUERY_KEY.AGENT.CLI` 캐시는 갱신하지 않는다(기존 `cli.installShellCommand` 와 동일) —
  설정 화면이 열려 있으면 상태 행이 재조회 전까지 낡은 값을 보인다.

## 3. IDE MCP 서버 (확장 기능 — Claude Code 를 1급 시민으로)

공식 확인 사실(research §5.1): Claude Code CLI 는 `~/.claude/ide/<port>.lock`(0600/0700) 을 읽고
`ws://127.0.0.1:<port>` + `X-Claude-Code-Ide-Authorization` 헤더로 IDE MCP 서버에 자동 연결한다.
**TAIDE 가 이 서버를 구현하면 내장 터미널에서 `claude` 실행만으로** diff 뷰어·선택 영역 컨텍스트·
진단 공유가 활성화된다.

- Rust `domain::ide` 모듈: 127.0.0.1 바인드(포트 10000~65535), CSPRNG hex 토큰, lock 파일 수명주기
  (시작 시 stale lock pid 청소, 종료 시 삭제), JSON-RPC 2.0.
- 도구 구현(비공식 프로토콜 — 실측 검증 필수): `openFile`, `openDiff`(TAIDE diff 탭, 블로킹),
  `getCurrentSelection`, `getOpenEditors`, `getWorkspaceFolders`, `getDiagnostics`(LSP 마커),
  `checkDocumentDirty`, `saveDocument`, `close_tab`. 알림: `selection_changed`, `at_mentioned`.
- pty 자식 env 주입은 IDE 서버가 running 일 때 **`CLAUDE_CODE_SSE_PORT=<port>` 단독**이다
  (`domain/ide/store.rs::claude_terminal_env` → `PtySpawnEnvProvider`). 초안의
  `ENABLE_IDE_INTEGRATION=true` 는 검증 결과 불요로 도입하지 않았다.
- 프로토콜이 비공식이므로 **실측 스모크 테스트를 CI/체크리스트에 포함**(버전 업 파손 감지).

## 4. hooks 브리지

두 갈래다. **Claude 는 인밴드 command hook**(그 세션의 pty 로 이벤트를 직접 써 넣는다),
**Codex·Gemini 는 종전의 HTTP command hook**(로컬 서버로 중계)이다. 설치는 양쪽 모두
`agentHooksEnabled` opt-in 토글 + 동의 UI 를 전제로 하고, Claude 는 프로젝트 파일
(`.claude/settings.local.json`, gitignore 대상), Codex·Gemini 는 사용자 레벨 파일에만 쓴다
(§7.5 "설치 스코프는 에이전트 정체성으로 결정" — 팀 설정 오염 금지, 함정 15).

### 4.1 Claude — 인밴드 command hook (d-54)

훅이 내는 것은 OSC 777 알림(urxvt 계열 공개 규격) 한 줄이고, title 이
`AGENT_OSC_SENTINEL`(`taide-agent`) 인 것만 TAIDE 이벤트로 읽힌다. 서버·포트·토큰·cwd 매칭·
프로젝트 단위 override 가 전부 필요 없어지고, 이벤트가 **그 세션의 pty 로** 들어오므로 같은
프로젝트의 두 Claude 세션이 서로를 덮지 않는다.

```
ESC ]777;notify;taide-agent;{"v":1,"agent":"claude","event":"permission_request"} BEL
```

설치 항목은 `service::claude_hook_entries(emitter)` 가 만드는 7행이다(`CLAUDE_HOOK_BINDINGS`).

| 훅 이벤트 | matcher | 인밴드 event |
|-----------|---------|--------------|
| `PermissionRequest` | — | `permission_request` |
| `Notification` | `permission_prompt` | `permission_request` |
| `Notification` | `elicitation_dialog` | `question_asked` |
| `Notification` | `idle_prompt` | `idle_prompt` |
| `PostToolUse` | — | `tool_complete` |
| `Stop` | — | `stop` |
| `StopFailure` | — | `stop_failure` |

`UserPromptSubmit`·`SessionStart` 는 **넣지 않는다** — 두 훅은 stdout 을 프롬프트 컨텍스트로도
해석하는 이벤트이고, Working 판정은 §1.2 의 타이틀·출력이 이미 담당한다.

`PermissionRequest` 는 다이얼로그가 뜨는 즉시 발화하므로 6초 타이머 뒤에야 오는
`Notification(permission_prompt)` 보다 빠르다. 두 훅을 함께 설치하는 이유는 그 6초 안에 답한
경우 후자가 아예 오지 않기 때문이다.

**신뢰 경계 — 이 채널은 인증되지 않는다(수용된 위험).** 세션 신원 증표가 없으므로, 그 pty 에
바이트를 쓸 수 있는 것은 무엇이든(에이전트가 실행한 하위 프로세스, `cat` 한 파일, 빌드 로그)
같은 시퀀스를 흉내 내 배지를 `AwaitingInput` 으로 올리거나(`permission_request`) 실제 차단을
지울 수 있다(`tool_complete`). 배지 전이는 OS 알림으로도 승격되므로 거짓 알림까지 간다. 이는
`docs/research/2026-09-06-terminal-agent-deep-dive.md` SI-7 이 OSC 133 에 대해 이미 기록한
위조 가능성과 **같은 등급**이며, pty 출력 전체가 신뢰할 수 없는 입력이라는 전제(`terminal.md`
§5.2)에서 나오는 구조적 한계다. 완화(세션별 nonce 를 `agent_protocol_env()` 로 발급해 훅
페이로드에 싣고 수신 시 대조)는 와이어 포맷 변경이라 계약 §4 의 후속 결정으로 남긴다.

### 4.2 명령 형태 — env 게이트 · 정적 printf · 항상 exit 0

`service::build_claude_agent_hook_command(event, emitter)` 가 만드는 문자열은 jq·CLI·서버 의존이
0이다. 페이로드는 상수고, 셸이 하는 일은 `printf` 하나뿐이다(아래 두 줄은 실제 설치되는 명령
문자열 그대로 — 백슬래시도 파일에 그대로 들어간다).

```sh
# HookEmitter::TerminalSequence — permission_request
if [ -n "$TAIDE_AGENT_PROTOCOL_VERSION" ]; then printf '%s' '{"terminalSequence":"<ESC>]777;notify;taide-agent;{\"v\":1,\"agent\":\"claude\",\"event\":\"permission_request\"}<BEL>"}'; fi; exit 0

# HookEmitter::DevTty — stop
if [ -n "$TAIDE_AGENT_PROTOCOL_VERSION" ]; then printf '\033]777;notify;taide-agent;{"v":1,"agent":"claude","event":"stop"}\007' > /dev/tty 2>/dev/null; fi; exit 0
```

- **env 게이트**: `TAIDE_AGENT_PROTOCOL_VERSION` 이 없으면 아무것도 내지 않는다. 같은
  `settings.local.json` 을 다른 터미널 앱에서 열어도 화면에 이상한 바이트가 찍히지 않는다는
  뜻이다. 이 변수는 `agent::commands::agent_protocol_env()` 가 `TAIDE_APP_VERSION` 과 함께
  돌려주고 `lib.rs::pty_spawn_env_provider` 가 `editor_terminal_env` 뒤에 이어 붙여
  **pty 스폰에만** 주입한다(§2.3 과 같은 훅 — 원격 미러·다른 표면과는 무관).
- **항상 `exit 0`**: `> /dev/tty` 는 제어 터미널이 없으면 리다이렉트 자체가 실패해 비영 종료가
  되고, 그러면 Claude 트랜스크립트에 훅 실패로 노출된다.
- 위 두 줄의 `<ESC>`·`<BEL>` 는 이 문서의 표기다. 실제 파일에서 `DevTty` 쪽은 `printf` 가 해석할
  `\033`·`\007`(백슬래시를 포함한 문자 그대로)이고, `TerminalSequence` 쪽은 `serde_json` 이 낸
  JSON 유니코드 이스케이프 6글자(백슬래시 + `u001b`, 백슬래시 + `u0007`)다.
- **`terminalSequence` 봉투는 `serde_json` 이 만든다** — 제어 바이트의 이스케이프와 내부 따옴표
  (`\"`)가 소비자(Claude Code 의 hook 출력 디코더)와 같은 규칙의 인코더에서 나오므로 어긋날 수
  없고, 결과에 작은따옴표가 없어 셸 단일 인용이 안전하다.

### 4.3 방출 방식 선택 · 마커 · 멱등성

- **선택**: `commands::resolve_claude_hook_emitter()` 가 `claude --version` 을 블로킹 풀에서 1회
  읽어(`CLAUDE_VERSION_TIMEOUT_SECONDS` 3초 데드라인, `OnceLock` 로 앱 실행당 캐시)
  `>= CLAUDE_TERMINAL_SEQUENCE_MIN_VERSION`(2.1.141)이면 `TerminalSequence`, 그 외(구버전·PATH 에
  없음·타임아웃·파싱 실패)는 전부 `DevTty` 로 떨어진다. `terminalSequence` 필드는 그 버전부터
  hook 출력에서 해석되고, 그 전에는 `/dev/tty` 직접 쓰기만 가능하다. 두 방출기는 사용자 눈에
  차이가 없어 별도 안내를 하지 않는다. 판정은 순수 함수
  `parse_claude_version`/`supports_terminal_sequence` 다.
- **마커 2종**: TAIDE 소유 항목은 `HOOKS_URL_MARKER`(`taide=1`, 폐기된 HTTP 설치)와
  `AGENT_OSC_MARKER`(`notify;taide-agent;`, 현행 인밴드) **둘 다**로 인식한다(`TAIDE_HOOK_MARKERS`).
  구버전 설치본도 계속 탐지·정리된다. 인밴드 쪽 마커가 센티널 단어(`taide-agent`)가 아니라 OSC
  페이로드 접두사인 이유는 소유 판정이 `command`·`url` 문자열의 부분 문자열 매치이기 때문이다 —
  단어만으로 판정하면 그 글자를 경로·문구에 담은 **사용자 자신의 hook 항목**이 TAIDE 소유로
  오판돼 다음 재조정에서 통째로 사라진다(§4 "팀 설정 오염 금지").
- **멱등 + 자가 치유**: `hooks::reconcile_claude_project_hooks`(부팅·토글 ON 전이·프로젝트 열기)가
  이미 TAIDE 항목이 있는 프로젝트만 모아, `claude_hook_entries_match` 로 기대 집합과 **정확히**
  같지 않으면(구 HTTP 항목·빠진 이벤트·다른 방출기의 사본) 제거 후 통째로 재주입한다. 항목이
  없는 프로젝트는 건드리지 않는다 — 설치는 사용자의 결정이다. `installed_claude_entries` 가
  `command` 뿐 아니라 `url` 도 읽으므로 HTTP 잔재는 어떤 기대 명령과도 같을 수 없어 반드시
  교체된다. 대상 프로젝트가 하나도 없으면 `claude --version` 프로브 자체를 건너뛴다.
- 제거(`agent_hooks_uninstall`·토글 OFF)는 `CLAUDE_MANAGED_HOOK_EVENTS`(현행 5종 + 레거시
  `UserPromptSubmit` 의 합집합)를 훑어 두 마커를 모두 지운다.

### 4.4 Codex · Gemini — HTTP 유지

- 사용자 레벨 command hook(`taide hook --url <u>` shim → 로컬 HTTP 서버)은 **이번 배치에서
  변경하지 않았다**. 재부팅으로 포트가 바뀌면 `reconcile_user_level_hooks` 가 새 URL 로 재주입하는
  자가 치유도 그대로다(§7.5).
- 그 페이로드는 종전대로 `(ProjectId, agent_name)` 단위 override 로 쌓이고, **세션 신호가 하나도
  없어 `Unknown` 인 비-claude 세션에만** 적용된다(`commands::resolve_activity`).
- Claude 는 더 이상 이 override 를 참고하지 않는다. 구버전 HTTP 설치가 남아 있는 세션은
  `apply_hook_payload` 의 즉시 발행을 받을 수는 있으나 다음 폴링 틱에 세션 신호 판정으로 덮인다.
  이 수신 분기(`agent=claude`)는 호환을 위해 이번 릴리스까지만 유지하고 제거 예정이다
  (계약 §4 미결 2).

### 4.5 statusline (후순위, 미구현)

statusline 바이너리: fire-and-forget POST 후 즉시 종료(300ms 디바운스 함정) —
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

값의 전체 표는 §1.4, 우선순위는 §1.3. 여기서는 세 상수의 **의미**만 적는다.

- `ACTIVITY_WORKING_HOLD_MS`(2000ms): Working 증거(실질 출력·`tool_complete`)를 한 번 본 뒤
  이 시간 동안은 Working 을 유지한다(폴링 틱 사이 관측 누락 보정). 유휴 힌트가 이미 있는
  세션은 이 시간만 조용해도 Idle 로 내려간다 — 힌트가 조용함의 기준선을 앞당기는 셈이다.
- `ACTIVITY_IDLE_QUIET_MS`(4000ms): 힌트가 없어도 마지막 신호 이후 이 시간이 지나면 Idle 이다.
  그 사이 구간(hold~quiet)은 직전 상태를 유지하는 히스테리시스 구간이다. 6000ms 에서 줄인
  근거는 §1.4 — 점멸 600ms·스피너 ≤1s 라 4초 침묵은 "정말 멈췄다" 로 읽어도 안전하다.
- `HOOK_OVERRIDE_STALE_MS`(900000ms=15분): HTTP hooks 가 설정한 **프로젝트 단위** 활동 override 의
  유효 기간. d-54 이후 이 override 를 보는 것은 codex·gemini 뿐이고, 그것도 세션 신호가 전무한
  경우에 한한다(§4.4).

### 7.2 세션 신호의 수집 (`domain/agent/commands.rs::AgentStore` · `service::apply_scan_to_signals`)

- **옵저버는 pty 리더 스레드에서 동기로 돈다** (`terminal::commands::PtySessionObservers`). 청크마다
  불리므로 IO 를 하지 않고, 다른 pty 커맨드가 쥐는 락도 잡지 않는다. 에이전트가 없는 세션의 비용은
  락 1회 + 실패하는 맵 조회 1회로 끝난다(`AgentStore::record_scan` 이 레코드가 없으면 즉시 반환).
- **신호 레코드는 폴링이 만든다**: `classify_session_activity` 가 그 세션의 에이전트를 처음 본 틱에
  `AgentSessionSignals::new(agent_name)` 를 넣는다. 그래서 감지 직전(최대 한 틱 = 500ms)에 도착한
  청크는 버려진다. 기동 직후에 뜨는 다이얼로그(신뢰 다이얼로그 `Yes, I trust this folder`)가 이
  창에 걸릴 수 있는지는 실기로 확인하지 않았다 — 걸리면 그 세션은 다음 실질 출력까지 차단으로
  읽히지 않으므로, 확인되면 첫 감지 틱에서 한 번 더 즉시 재확인하는 콜드스타트 경로를 넣는다
  (계약 §3 "검토·수정" f3).
- `agent_name` 을 레코드에 함께 들고 있는 이유는 리더 스레드에서 에이전트별 시그니처 표를
  O(1) 로 고르기 위해서다(매 청크 `agents` 맵을 전수 탐색하지 않는다). 세션의 에이전트가 바뀌면
  `classify_session_activity` 가 레코드를 새로 만든다 — `prune_signals` 는 그 틱에 에이전트가
  하나도 감지되지 않은 세션만 지우므로, 셸을 거치지 않은 교체는 그것만으로 정리되지 않는다.
- **꼬리 결합 매칭은 경계를 가로지르는 경우로 좁혔다** (`find_dialog_signature_across_boundary`).
  꼬리 전체와 새 텍스트를 통째로 이어 매칭하면, 사용자가 답한 뒤에도 꼬리에 남은 문구가 다시
  래치를 건다 — 점멸 청크만 흐르는 동안 128바이트 꼬리가 밀려나가는 데 십수 초가 걸리므로
  "실질 출력이 래치를 푼다"(§1.2)가 매 청크 무효화되고, 결국 고치려던 화면으로 되돌아간다.
- `AgentEvent`·`BlockedSource`·`TitleGlyph` 는 `service.rs` 의 순수 계층이고, 모든 판정 함수는
  `now: Instant` 를 인자로 받는다 — 대기 없이 단위 테스트로 시간 축을 재현하기 위해서다.

### 7.3 hooks 항목 병합 (`domain/agent/service.rs::remove_taide_hook_entries` / `inject_taide_claude_command_hook_entries`)

`remove_taide_hook_entries` 는 TAIDE 가 주입한 hook 항목(`TAIDE_HOOK_MARKERS` 2종 중 하나를 담은
`command` 또는 `url`)만 골라 제거하고, 사용자가 직접 추가한 다른 hook 은 그대로 둔다.
`inject_taide_claude_command_hook_entries` 는 먼저 제거 후 삽입하는 방식으로 **재주입해도 항목이
중복되지 않게(멱등)** 만들고, 그래서 재설치·HTTP 설치본 승격·방출기 전환이 모두 같은 파일 내용으로
수렴한다. 두 함수 모두 `.claude/settings.local.json` 의 기존 설정을 보존한 채 병합해야 한다는 §4 의
"팀 설정 오염 금지" 제약을 구현한다.

### 7.4 IDE MCP 서버 (`domain/ide`)

- **토큰 비교** (`constant_time_eq`): 인증 토큰 검증에 타이밍 사이드채널을 남기지 않기 위한
  상수시간 비교. 단일 구현이 `infra/crypto.rs` 에 있고 `domain::ide`·`domain::agent` 가 각각
  `pub use crate::infra::crypto::constant_time_eq` 로 재사용한다(중복 구현 아님).
- **토큰 생성** (`service.rs::generate_auth_token`): CLI 와 동일한 형식(32자 소문자 hex, OS
  CSPRNG 기반)의 인증 토큰을 만든다. 신규 `rand` 의존성을 들이지 않고, 이미 동일 용도(hooks.rs)로
  쓰이는 uuid v4 를 재사용한다(`docs/acknowledge/2026-08-07-qa-batch-decisions.md` §3 신규 의존성
  결정과 일치).
- **경로 접근 차단** (`service.rs::ensure_path_within_any_project`): `openFile`/`saveDocument` 가
  프로젝트 루트 밖 경로에 접근하지 못하도록, file 도메인의 기존 검증(`ensure_within_root` 기반)을
  그대로 재사용한다(임의 파일 접근 차단).
- **존재 선검증** (`server.rs::resolve_open_file_target`, 2026-09-04): `openFile` 은 경계 검사에 이어
  `infra::root_guard::ensure_existing_file` 로 실제 파일인지 확인한 뒤에야 탭을 만든다. 이 핸들러는
  도메인 경계상 `layout_open_tab` 커맨드를 부를 수 없어 `layout::service::open_tab_and_finish` 를 직접
  호출하므로, 커맨드 층 게이트(`docs/ipc-contract.md` `layout_open_tab` 절)와 같은 함수를 자기 쪽에서
  한 번 더 탄다. 없는 경로면 탭을 만들지 않고 `RPC_INVALID_PARAMS` 로 거절한다(에이전트가 기억하고
  있던 옛 경로로 빈 탭이 열리던 흐름 차단).
- **열린 에디터 목록** (`service.rs::open_editors_snapshot`): 모든 프로젝트의 레이아웃에서 File
  탭만 모아 `getOpenEditors` 응답을 만든다. `is_active` 는 각 프로젝트 레이아웃의 focused pane 의
  active tab 기준으로 판정한다(여러 프로젝트가 열려 있어도 프로젝트별로 하나씩 active 가 나올 수
  있다).
- **진단 None vs 빈 배열** (`store.rs::IdeStore::diagnostics`): 진단이 한 번도 push 된 적
  없으면 `None`(= "아직 준비 안 됨")을 반환한다. 빈 배열(진단 0건)과 "아직 모름"을 구분해
  `getDiagnostics` 호출자의 거짓 음성(오탐 없음으로 오인)을 방지한다.
- **탭 닫기 경로와의 정합** (`store.rs::reconcile_closed_tab`): ClaudeDiff 탭이 (도구 호출
  경로가 아니라) 일반 탭 닫기 경로로 닫혔을 때, 그 탭에 매인 pending `openDiff` 요청을
  `TabClosed` 로 해소한다. layout 도메인의 모든 탭 닫기 경로(Tauri 커맨드·IDE 도구 핸들러 공용
  `close_tab_and_finish`)에서 반드시 호출되어야 하는 불변조건이다 — 누락하면 `openDiff` 가
  무기한 대기 상태로 남는다.
- **종료 시 즉시 중단** (`commands.rs::stop_server`): 앱 종료(`lib.rs`)·`settings_update` 의
  `ideIntegrationEnabled` 토글-off(`ide::commands::apply_ide_integration_toggle` — `lib.rs` 가
  `SettingsToggleObservers` 로 등록하는 관찰자) 공용 정리
  경로(과거엔 `ide_stop` 커맨드도 이 함수를 불렀으나, X-A 배치(2026-08-19)에서 두 경로만 남고
  중복 커맨드로 제거됐다 — `docs/ipc-contract.md` §"ide" 절). pending
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

### 7.6 서드파티 hooks 파일 안전성 (`domain/agent/commands.rs`)

> 7.10-W5-B — W4 보류분. `.claude/settings.local.json`·`~/.codex/hooks.json`·
> `~/.gemini/settings.json` 은 TAIDE 가 아니라 각 CLI 도구가 소유·기록하는 파일이므로, TAIDE 는
> 이 파일들에 대해 "손님"으로서 최소 개입 원칙을 지킨다.

- **파싱 실패는 항상 거부, 빈 객체 대체 금지** (`read_json_file_rejecting_invalid`): 파일이
  존재하지 않으면 빈 객체(`{}`)로 취급해 신규 설치를 허용하지만, 파일이 **존재하는데 유효한
  JSON 이 아니면** `AppError` 를 반환하고 끝낸다. `agent_hooks_install`/`agent_hooks_uninstall`
  은 이 함수의 결과를 `?` 로 즉시 전파하므로, 파싱에 실패한 시점에 쓰기 경로(`write_settings_local`/
  `write_user_level_hooks`) 자체가 호출되지 않는다 — 손상된(비 JSON) 파일을 빈 객체로 되살려
  덮어쓰는 사고를 원천 차단한다. 프론트에는 `agent.hooksFileInvalid` 로케일 키를 스파인으로
  먼저 추가했다(소비하는 UI 는 후속 웨이브).
- **third-party 파일은 기존 권한을 보존한다** (`write_hooks_file_preserving_mode`): 범용
  `infra::persist::write_private_atomic` 은 재작성할 때마다 무조건 `0600` 으로 되돌리는데, 이
  hooks 파일들은 Claude Code·Codex·Gemini CLI 가 직접 만들고 스스로의 규칙으로 권한을 설정할
  수 있는 파일이다(예: 팀 정책으로 그룹 읽기를 허용해 뒀을 수 있다). TAIDE 가 재작성 때마다
  그 권한을 조용히 `0600` 으로 좁히면 그 CLI 도구 입장에서는 예고 없는 권한 변경이 된다. 이
  경로 전용 헬퍼는 재작성 직전 기존 파일의 mode 를 읽어 그대로 유지하고, **파일이 새로
  생성되는 경우에만** `0600`(`NEW_HOOKS_FILE_MODE`, hook URL 에 토큰이 실리므로 소유자 전용)을
  적용한다. `write_private_atomic` 자신의 범용 계약(항상 `0600`)은 바꾸지 않았다 — 다른
  호출자(`lockfile.rs` 등)는 여전히 원래 동작 그대로다. 임시파일 이름 규칙만
  `infra::persist::temp_sibling` 을 `pub(crate)` 로 노출해 재사용했다(atomic rename 규약 중복
  방지).
