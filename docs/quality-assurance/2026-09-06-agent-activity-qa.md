# 에이전트 활동 배지 — 손 QA 체크리스트 (d-54, 2026-09-06)

> 계약 `acknowledge/2026-09-06-d54-agent-activity-signals-contract.md` §0.2(실측)·§1.2(판정)·§3(구현·검토 기록),
> 기능 정본 `features/agent-integration.md` §1·§4·§7, 버그 기록 `bug/2026-09-06-agent-badge-idle-during-permission-prompt.md`.
>
> **왜 손 QA 인가**: 판정 함수(`classify_session`)·스캐너(`OutputScanner`)·훅 명령 빌더는 Rust 단위 테스트로 잠겼고,
> 실물 pty 바이트를 가상 시계로 재생하는 회귀 테스트
> (`domain::agent::service::tests::scenario_timeline`)가 (a)~(i) 단계를 한 번에 단언한다. 그러나 **실제 Claude Code
> 프로세스가 TAIDE pty 안에서 내는 바이트·타이밍, 사이드바·탭 배지의 렌더, hooks 설치 파일의 실물, tmux·외부 터미널
> 같은 환경 차이는 실행해서 눈으로 확인해야 한다.** `bun run tauri dev` 로 앱을 띄우고 진행한다.
>
> 배지 표기: **파란 점 = 작업 중(Working)** · **노란 마름모 = 입력 대기(AwaitingInput)** · **초록 빈 원 = 유휴(Idle)**.
> 판정 주기는 500ms 이므로 "즉시" 는 다음 폴링 틱, 곧 **1초 안** 을 뜻한다. 상수는 `domain/agent/types.rs`
> (`ACTIVITY_WORKING_HOLD_MS=2_000` · `ACTIVITY_IDLE_QUIET_MS=4_000` · `TITLE_WORKING_FRESH_MS=3_000` · `ECHO_SUPPRESS_MS=300`).

## 1. 사전 준비 (픽스처)

- [ ] Claude Code **2.1.141 이상**(`claude --version`)이 PATH 에 있다 — 2.1.263 에서 실측했다
- [ ] 프로젝트 A: `agentHooksEnabled` **off**(기본값), `.claude/settings.local.json` 에 `taide-agent`·`taide=1` 문자열이 0건
- [ ] 프로젝트 B: 설정 UI 에서 hooks 를 **켜고 설치**한다(동의 다이얼로그 승인)
- [ ] 두 프로젝트 모두 TAIDE 터미널 탭에서 `claude --permission-mode default` 로 세션을 연다
- [ ] 권한 다이얼로그를 확실히 띄우는 프롬프트를 준비한다. 예: `touch /tmp/taide-probe-file 을 Bash 로 실행해`
      (자동 승인되지 않도록 `.claude/settings*.json` 의 `permissions.allow` 에 `Bash` 가 없어야 한다)

## 2. hooks 없이 — 휴리스틱 단독 (프로젝트 A)

- [ ] 세션을 연 직후(프롬프트만 보이는 상태) 배지가 **초록 빈 원**이 된다. `Unknown` 은 없어야 한다
      (시작 타이틀 `✳ Claude Code` 뒤 2초 안에 유휴 — 회귀 테스트 (a))
- [ ] 프롬프트를 보내면 배지가 **1초 안에 파란 점**이 된다(타이틀 `◐/◑` + 스피너 프레임)
- [ ] `Do you want to proceed?` 다이얼로그가 뜨면 배지가 **1초 안에 노란 마름모**가 된다 — 이번 수정의 핵심
- [ ] 다이얼로그를 **6초 이상 방치**해도 노란 마름모가 유지된다(`⏺` 점멸만 흐르는 동안 유휴로 떨어지지 않는다 — (e))
- [ ] `1` 또는 Enter 로 답하면 **1초 안에 파란 점**(작업)으로 바뀐다 — (f)
- [ ] 턴이 끝나 프롬프트로 돌아오면 **4초 안에 초록 빈 원**이 된다(타이틀 `✳` + 2초 정적 — (g))
- [ ] 다이얼로그가 뜬 채로 **다른 프로젝트 탭으로 옮겨** 사이드바 프로젝트 배지만 보며 위 순서(노랑 → 파랑 → 초록)가
      동일하게 진행된다(포커스가 없어도 pty 출력은 계속 스캔된다)
- [ ] 다이얼로그에 답한 직후 **곧바로 다음 다이얼로그**가 뜨는 연쇄 승인(예: Bash 두 번 실행 프롬프트)에서도
      두 번째 다이얼로그가 노란 마름모로 잡힌다 — (i), D 단계 f1 회귀
- [ ] 스피너가 도는 동안 입력창에 **글자를 타이핑**(전송하지 않음)해도 배지가 파란 점에서 흔들리지 않는다.
      유휴 상태에서 타이핑해도 파란 점으로 바뀌지 않는다(에코 억제 창 300ms — (b))
- [ ] 작업 중 푸터 `esc to interrupt` 가 보이는 동안 배지가 노란 마름모로 오판되지 않는다(대소문자 정확 일치)
- [ ] 작업 중 **Ctrl-C 로 턴을 중단**하면 4초 안에 초록 빈 원이 된다(중단 후에는 실질 출력이 멈춘다)

## 3. hooks 켠 프로젝트 — 인밴드 command hook (프로젝트 B)

- [ ] 설치 직후 `.claude/settings.local.json` 에 `type: "command"` 항목이 **7행** 생긴다:
      `PermissionRequest` 1 · `Notification` 3(`permission_prompt` · `elicitation_dialog` · `idle_prompt`) ·
      `PostToolUse` 1 · `Stop` 1 · `StopFailure` 1. `UserPromptSubmit` · `SessionStart` 는 없다
- [ ] 각 명령이 `if [ -n "$TAIDE_AGENT_PROTOCOL_VERSION" ]; then printf ...; fi; exit 0` 형태이고 `http://` · `--url` 이 없다
- [ ] 2.1.141 이상이면 명령이 `printf '%s' '{"terminalSequence": ...}'` 이고, 그 미만·버전 판독 실패면 `> /dev/tty` 다
      (동작 차이는 없다 — 둘 중 하나면 된다)
- [ ] 같은 프로젝트의 사용자 정의 hook 항목(있다면)은 그대로 남아 있다
- [ ] 설정 UI 에서 껐다 켜기를 2회 반복해도 항목이 **7행에서 늘지 않는다**(멱등)
- [ ] 세션을 새로 열고 권한 다이얼로그를 띄우면 배지가 **다이얼로그 렌더와 같은 틱**에 노란 마름모가 된다
      (`PermissionRequest` 훅은 즉시 발화 — 6초 지연이 없다)
- [ ] Claude 트랜스크립트에 hook 실패(비영 종료) 경고가 뜨지 않는다
- [ ] hooks 를 끄면 7행이 모두 사라지고, TAIDE 항목만 있었다면 `hooks` 키 자체가 사라진다

## 4. 환경 게이트 · 다른 표면

- [ ] **iTerm 등 외부 터미널**에서 프로젝트 B 의 `claude` 를 실행해 권한 다이얼로그를 띄워도 TAIDE 의 프로젝트 B 배지는
      변하지 않고, 외부 터미널 화면에 이스케이프 문자열이 새어 나오지 않는다(`TAIDE_AGENT_PROTOCOL_VERSION` 미설정 → 출력 0)
- [ ] TAIDE 터미널에서 `echo $TAIDE_AGENT_PROTOCOL_VERSION $TAIDE_APP_VERSION` 이 `1 <앱 버전>` 을 찍는다
- [ ] **tmux 안**에서 `claude` 를 실행한다: 타이틀이 정적 `✳` 로 고정되어도 작업 중에는 파란 점, 다이얼로그에는
      노란 마름모, 턴 종료 뒤에는 초록 빈 원이 된다(출력 흐름이 타이틀 힌트를 이긴다)
- [ ] **codex · gemini** 세션은 파란 점/초록 빈 원만 오간다. 권한 다이얼로그를 띄워도 노란 마름모가 되지 않는 것이
      **의도된 현재 범위**다(다이얼로그 시그니처·인밴드 훅은 Claude 만 — 계약 §1.1.3·§1.7). 사용자 레벨 HTTP hook 을
      설치한 경우에만 그 경로로 노란 마름모가 된다
- [ ] 한 프로젝트에서 **두 세션**을 동시에 열고 한쪽만 다이얼로그를 띄우면 그 세션 탭만 노란 마름모가 된다
      (세션 단위 신호 — 프로젝트 단위 override 가 서로 덮지 않는다)
- [ ] 세션 탭을 닫으면 배지가 사라진다(신호 레코드 제거)

## 5. 탐침 스크립트 — 실물 바이트 재확인 (계약 §0.2)

TAIDE 밖에서 `TERM_PROGRAM=TAIDE` 인 pty 에 Claude Code 를 띄워 원시 출력을 기록하는 `expect` 스크립트다.
회귀 테스트의 픽스처(`RAW_*` 바이트 리터럴)는 이 기록에서 옮긴 것이므로, Claude Code 가 업데이트되어 배지가 어긋나면
먼저 이것을 다시 돌려 바이트가 바뀌었는지 본다.

- 위치: 세션 스크래치패드의 `probe-claude-permission.exp`(저장소에 두지 않는다 — 원시 로그에 세션 URL 등 식별 조각이 남는다)
- 실행은 **프로젝트 밖 임시 디렉터리**에서 한다(`.claude/settings.local.json` 의 allow 규칙·hooks 에 영향받지 않게):

```sh
mkdir -p /tmp/taide-probe-cwd && cd /tmp/taide-probe-cwd
expect -f probe-claude-permission.exp /tmp/claude-raw.log /tmp/taide-probe-cwd \
  'touch /tmp/taide-probe-file 을 Bash 로 실행해' 'proceed\?'
```

- 인자: `<로그 경로> <작업 디렉터리> <프롬프트 문구> <다이얼로그 정규식>`. 정규식이 비면 다이얼로그를 기다리지 않고
  조용해질 때까지만 기록한다(작업 → 유휴 전체 턴)
- 산출물: `<로그>`(원시 바이트) · `<로그>.timeline`(ms 타임스탬프별 청크 길이 · `pattern-seen` · `quiet-1s`)
- 확인 방법:
  - `grep -a -o -b $'\x1b]0;[^\x07]*\x07' <로그>` — 타이틀 시퀀스가 `✳ → ◐/◑ 교대 → ✳` 인가
  - `dd if=<로그> bs=1 skip=<offset> count=200 | od -c` — 다이얼로그가 `Do\e[5Gyou\e[9Gwant...` 열 이동 형태인가
  - timeline 에서 다이얼로그 이후 청크가 **600ms 간격 48바이트**인가, 유휴 구간이 `quiet-1s` 만 이어지는가
- 기대 결과가 계약 §0.2 표와 다르면 `docs/bug` 에 기록하고 회귀 테스트 픽스처를 갱신한다(2KB 이하, 식별 조각 제거)

## 6. 종료 조건

- [ ] §2·§3·§4 의 항목이 모두 체크되었다
- [ ] 어긋난 항목은 `docs/bug/` 에 증상·재현 프롬프트·timeline 발췌와 함께 기록했다
- [ ] `cargo test scenario_timeline` 이 통과한다(픽스처를 갱신했다면 갱신 후 재확인)
