> d-60 §1.A 실측 탐침 산출(opus·high, expect/pty 캡처). 등급: 확정(바이트 근거) / [미확인]. 시크릿 없음.

## 0. 요약 — 신호 가용성 표

| 항목 | opencode | codex |
|---|---|---|
| 프로세스명 | 단일 네이티브 Mach-O arm64. `ps -o comm=` 절대경로 `/Users/gkn/.opencode/bin/opencode` → basename `opencode` (node/bun 셸 아님, pgid 리더는 항상 opencode 본체) | 전경 pgid 리더는 **node**(`@openai/codex/bin/codex.js`), 네이티브 `codex` 바이너리는 그 자식(`spawn(..., {stdio:"inherit"})`). 현행 node→cmdline basename 폴백으로 신원 판정은 무변경 동작 |
| 타이틀 글리프 | **없음(None)**. OSC 0 + BEL 만 사용, 선행 글리프·작업 중 교대 글리프 없음. `OpenCode`(기동) → `OC \| <요약>`(첫 턴 확정 1회) → 빈 문자열(종료). 권한 다이얼로그 중 갱신 0건 | **[미확인]**. 캡처 구간 OSC 0/2 사용 0건(대체화면도 없음). 바이너리에 `[tui].terminal_title` 능력·`run-state`/`project-name`/`thread-title` 식별자는 존재하나 기본 동작·글리프 미확인 |
| 다이얼로그 시그니처(정확 문구) | 주: `"Permission required"` / 보조: `"Allow once"` (렌더는 `CSI row;col H` 셀 배치 → TAIDE 정규화에서 구 단위 완전일치 가능. 바이너리 옵션 테이블과 교차확인 완료) | 신뢰 다이얼로그만 확정: `"› 1. Yes, continue"` / `"Press enter to continue"` / `"You are in "` (렌더는 `CSI row;col H` **단어마다** 이동 → 정규화 시 단어 사이에 개행 삽입, **띄어쓰기 포함 다구 시그니처는 매치 불가**). 명령 승인 다이얼로그 문구는 strings 후보만 확보, **[미확인]**(상수 승격 금지) |
| 스피너 글리프 | 6프레임 무한 순환: `◦`(U+25E6) `·`(U+00B7) `•`(U+2022) `●`(U+25CF) `○`(U+25CB) `◌`(U+25CC). 유휴·권한 대기 중에도 계속 출력 → `·` 외 5종을 `NON_SUBSTANTIVE_GLYPHS` 에 추가하지 않으면 영구 Working 오판. (작업 중 전용 `⬝`/`■` 는 유휴 시 0건이라 실질 출력으로 유지) | **[미확인]**. 미채취 |
| 인밴드 전제 | **확정(가능)**. 플러그인이 `process.stdout.write` 로 쓴 `OSC 777`(`notify;taide-agent;{...}`)이 pty 에 그대로 도달(raw4.bin, 100건: `probe_load`/`plugin.added`/`catalog.updated`/`reference.updated`/`integration.updated`). opentui 대체화면 렌더와 충돌 없음 → HTTP 훅 폴백 불필요 | **[미확인]**. 훅 이벤트 식별자(`PreToolUse`/`PermissionRequest`/`PostToolUse`/`UserPromptSubmit`)는 바이너리에 실재하나, 훅 프로세스의 `/dev/tty` 쓰기·세션 분리(setsid) 여부 미확인 → 인밴드 vs HTTP 폴백 분기 미결정 |

---

## opencode

# opencode 1.18.29 감지 탐침 결과 (2026-09-15)

캡처 위치(세션 스크래치, 레포 무변경):
`/private/tmp/claude-501/-Users-gkn-taide/b2c3fcd5-412c-4bed-9f9e-acae08a2832f/scratchpad/probe-opencode/`
- `raw1.bin`(ls 프롬프트, 65687B) · `raw2.bin`(edit 프롬프트, 82098B) · `raw3.bin`(권한 다이얼로그, 41330B) · `raw4.bin`(플러그인 stdout, 18577B)
- `raw*.norm.txt` = `norm.py` 출력. `norm.py` 는 `src-tauri/src/infra/terminal_scan.rs` 의 `push_plain_byte`/`step_csi`/`step_osc`/`step_string_sequence` 규칙(CSI `G`·`C`→공백 1개, `A B E F H d f`→개행, 나머지 CSI 무시, OSC/DCS/APC/PM 통째 제거, C0 제거, 공백 중복 제거)을 그대로 이식한 것이다.

pty 환경: `TERM=xterm-256color`, `TERM_PROGRAM=TAIDE`, 120x40. 프롬프트는 무해(`ls`/`cat`/한 줄 추가), 다이얼로그는 Esc(=`reject`)로 거부했고 어떤 승인도 하지 않았다.

---

### 1. 프로세스 신원 — 확정(바이트 근거)

```
pid=83014 comm=[/Users/gkn/.opencode/bin/opencode] basename=[opencode]
```

- `file` 결과 `Mach-O 64-bit executable arm64`. **node/bun 셸이 아니다** — cmdline basename 폴백 경로를 탈 필요가 없다.
- `ps -o comm=` 가 **절대경로**를 돌려주므로 현행 `service.rs:49-71` 의 "comm basename 완전일치" 규칙으로 `opencode` 가 그대로 매치된다. (배치 `ps` 의 comm 컬럼이 16자로 잘려 `/Users/gkn/.open` 으로 보이는 것은 출력 폭 문제이며, 단일 pid 조회 시 전체 경로가 나온다 — 판정 로직에는 영향 없음.)
- pgid 리더는 항상 opencode 본체. 자식으로 MCP 서버가 붙는다(`node .../lsp-daemon/dist/cli.js mcp`, `docker run -i --rm -e GITHUB_PERSONAL_ACCESS_TOKEN ...` — **환경변수 이름만** 기록, 값은 노출되지 않음). 전경 pgid 리더를 보는 현행 방식이면 자식에 오인될 위험은 없다.

### 2. 타이틀 (OSC 0) — 확정(바이트 근거)

모든 캡처에서 **OSC 0 + BEL** 만 쓴다(OSC 2 미사용, ST 종결 미사용).

| 캡처 | offset | 페이로드 |
|---|---|---|
| raw1 | 6781 / 11760 / 13425 / 13438 | `OpenCode` |
| raw1 | 47784 | `OC \| ls로 디렉토리 파일 목록 보기` |
| raw1 | 65682 | `` (종료 시 빈 타이틀로 리셋) |
| raw2 | 82054 | `OC \| README.md 맨 끝에 x 추가` |
| raw3 | 27295 | `OC \| README.md 파일 내용 확인` |

- **선행 글리프가 없다.** 작업 중/유휴 교대 글리프도 없다. `OC | ` 는 첫 턴 요약이 확정되는 시점에 한 번 나오고 그 뒤로 갱신되지 않는다.
- raw3 에서 타이틀(27295) 은 권한 다이얼로그(31104) **이전**에 나왔고, 다이얼로그가 떠 있는 동안 타이틀 갱신은 0건이다.
- 결론: `TITLE_GLYPHS_FOR("opencode")` 는 비운다(None). 타이틀은 opencode 세션 식별에만 쓸 수 있고 활동 신호로는 못 쓴다.

### 3. 스피너 / 비실질 글리프 — 확정(바이트 근거)

정규화 텍스트 꼬리의 프레임 순서(raw1):

```
◦ · • ● ○ ◌ ◦ · • ● ○ ◌ ◦ · • ● ○ ◌ ...
U+25E6 U+00B7 U+2022 U+25CF U+25CB U+25CC (6프레임 순환)
```

- 이 6프레임은 **턴 종료 후 유휴 상태에서도, 권한 다이얼로그 대기 중에도 끊김 없이** 출력된다. `·`(U+00B7) 만 현행 `NON_SUBSTANTIVE_GLYPHS` 에 있으므로, **나머지 5종(U+25E6 ◦, U+2022 •, U+25CF ●, U+25CB ○, U+25CC ◌)을 추가하지 않으면 opencode 세션은 영구 Working 으로 오판**된다. 이것이 opencode 지원의 필수 전제다.
- 작업 중에만 나오는 진행 애니메이션: `⬝`(U+2B1D, raw3 에서 352회) + `■`(U+25A0, 128회). 유휴 구간에는 0건이므로 실질 출력으로 둬도 판정이 옳다 — 비실질 목록에 넣지 말 것.
- 정적 크롬: `▀`(U+2580) `█`(U+2588) `┃`(U+2503) `╹`(U+2579) `▣`(U+25A3). 상태바 재도색 시에만 나온다.

### 4. 권한 다이얼로그 — 확정(바이트 근거)

사용자 기본 설정(`~/.config/opencode/opencode.json` 에 `permission` 키 없음)에서는 `ls`·파일 편집 모두 **무승인 실행**되어 다이얼로그가 뜨지 않았다(raw1·raw2). 사용자 설정을 건드리지 않기 위해 **스크래치 작업 디렉터리에만** 프로젝트 레벨 `opencode.json`(`{"permission":{"edit":"ask","bash":"ask","webfetch":"ask"}}`)을 두고 재실행해 포착했다(raw3).

#### 4.1 정규화 텍스트 (라인 단위, 대소문자 그대로)

```
465 '■'
473 '△'
475 'Permission required'
481 'Shell command'
487 '$ cat README.md'
499 'Allow once'
502 'Allow always'
504 'Reject'
506 'ctrl+f '   507 'fullscreen'
509 '⇆ '       510 'select'
512 'enter '    513 'confirm'
```

#### 4.2 렌더 방식 — Claude 와 다르다

Claude 는 산문을 `CSI n G` 로 단어 분할해 뿌리지만, opencode(opentui)는 **`CSI row;col H`(CUP)로 셀 단위 배치**하고 각 문구는 연속 바이트다. TAIDE 정규화에서 `H` → 개행이므로 각 문구가 독립 라인으로 복원된다 → **구 단위 완전일치 매칭이 그대로 통한다**(단어 사이에 공백이 끼어들 걱정 없음).

`Permission required` 주변 hexdump(raw3, offset 0x7926~, 시크릿 없음 — SGR 색상과 CUP 뿐):

```
00007956  1b 5b 33 32 3b 38 48 1b 5b 33 38 3b 32 3b 32 33  .[32;8H.[38;2;23
00007966  38 3b 32 33 38 3b 32 33 38 6d 1b 5b 34 38 3b 32  8;238;238m.[48;2
00007976  3b 32 30 3b 32 30 3b 32 30 6d 50 65 72 6d 69 73  ;20;20;20mPermis
00007986  73 69 6f 6e 20 72 65 71 75 69 72 65 64 1b 5b 30  sion required.[0
00007996  6d 1b 5b 33 32 3b 32 37 48 1b 5b 33 38 3b 32 3b  m.[32;27H.[38;2;
```

`Allow once`(offset 32865) 도 동일 패턴:
```
...\x1b[38;7H\x1b[38;2;10;10;10m\x1b[48;2;245;167;66mAllow once\x1b[0m\x1b[38;17H...
```

#### 4.3 바이너리 교차확인 (1차 출처)

```
var XA=[{optionId:"once",kind:"allow_once",name:"Allow once"},
        {optionId:"always",kind:"allow_always",name:"Always allow"},
        {optionId:"reject",kind:"reject_once",name:"Reject"}];

L(Va,{title:"Permission required", ..., options:{once:"Allow once",always:"Allow always",reject:"Reject"},
      escapeKey:"reject", fullscreen:!0, ...})
```

TUI 렌더는 `Allow always`(실측·`options` 객체와 일치), API 옵션 테이블은 `Always allow`. **시그니처는 렌더 쪽 문구를 쓴다.**

#### 4.4 제안 시그니처 (`dialog_signatures_for("opencode")`)

```
"Permission required"     — 주 시그니처(다이얼로그 전용, 유일)
"Allow once"              — 보조
```

`Reject` 단독은 다른 문맥 오탐 위험이 있어 넣지 않는다. `Esc to cancel` 류 Claude 문구와의 충돌은 없다(opencode 푸터는 `ctrl+f fullscreen` / `⇆ select` / `enter confirm`).

### 5. 유휴 프롬프트 — 확정(바이트 근거)

전용 프롬프트 문자열이 없다. 기동 즉시 대체화면(`CSI ?1049h`)으로 전환되는 전면 TUI 이고, 유휴 상태에서 남는 것은 하단 상태바와 §3 의 6프레임 스피너뿐이다:

```
┃ Sisyphus - Ultraworker · deepseek-v4-flash:0731-cloud  Ollama Cloud ╹
▀▀▀▀▀... (게이지 바)
<cwd>   45.3K (4%   ctrl+p   commands
```

→ **출력 흐름만으로 유휴를 판정할 수 없다.** §3 의 글리프 제외가 반드시 선행돼야 `ACTIVITY_IDLE_QUIET_MS` 가 의미를 갖는다.

### 6. 인밴드 전제 ⑥ — 플러그인 stdout 접근: 확정(가능)

스크래치 디렉터리에만 탐침 플러그인을 두고(`.opencode/plugins/taide-probe.js`), `process.stdout.write` 로 TAIDE 규약의 OSC 777 을 방출했다.

```js
process.stdout.write("]777;notify;taide-agent;{\"v\":1,\"agent\":\"opencode\",\"event\":\"" + name + "\"}")
```

raw4.bin 파싱 결과 — **pty 에 그대로 도달**(100건):

```
OSC777 off=301   notify;taide-agent;{"v":1,"agent":"opencode","event":"probe_load"}
OSC777 off=10737 notify;taide-agent;{"v":1,"agent":"opencode","event":"plugin.added"}
OSC777 off=16304 notify;taide-agent;{"v":1,"agent":"opencode","event":"catalog.updated"}
OSC777 off=17660 notify;taide-agent;{"v":1,"agent":"opencode","event":"reference.updated"}
OSC777 off=17820 notify;taide-agent;{"v":1,"agent":"opencode","event":"integration.updated"}
```

- opentui 의 대체화면 렌더와 **바이트 충돌 없이 통과**한다. TAIDE 스캐너의 `agent_event` 분기가 그대로 파싱한다.
- 부수 확정: 각 이벤트가 정확히 2배로 나왔다 → **`.opencode/plugins/` 와 `.opencode/plugin/` 둘 다 로드된다.** 사용자 레벨 설치 시 한쪽만 써야 중복 방출이 없다(계약 §1.C 의 `~/.config/opencode/plugins/` 선택은 유효하되, 같은 이름의 단수형 디렉터리가 이미 있으면 중복이 난다 — 설치 전 확인 필요).
- 결론: **HTTP 훅 폴백이 필요 없다.** opencode 는 계약 §1.C 대로 인밴드 OSC 777 플러그인 경로로 간다.

### 7. Bus 이벤트 어휘 — 확정(바이너리)

```
"permission.asked"      21회
"permission.replied"    13회
"session.idle"           3회
"tool.execute.before"    8회
"tool.execute.after"     7회
```

d-60 계약 T4 의 이벤트 어휘가 1차 출처로 확인됐다. 다만 **이 이벤트들이 플러그인 `event` 핸들러로 실제 전달되는 순간의 바이트는 미포착**(§8 참조).

### 8. [미확인] 목록

- **권한 이벤트의 플러그인 도달** — 탐침 창(40초) 안에 다이얼로그까지 도달하지 못해 `permission.asked` 의 실제 방출 바이트가 없다. 기동 이벤트 도달은 확정이므로 채널 자체는 열려 있으나, 권한 이벤트 연결은 구현 전 재실측 권장.
- **장시간 다이얼로그 대기 시 타이틀 동결 지속** — 1회 관측(수십 초)만 했다.
- **권한 이외 다이얼로그**(질문형·계획 승인 등) 문구 — 미포착. 추측으로 시그니처에 넣지 말 것.
- **OSC 99 negotiation** — 기동 시 `\e]99;i=opentui-notifications:p=?;\e\\` 로 kitty notification 지원을 질의한다(raw 선두 300B 에서 확인). TAIDE 가 응답하면 opencode 가 자체 알림을 인밴드로 낼 가능성이 있으나, 응답을 보내지 않아 미실험이다. 현행 TAIDE 스캐너는 OSC 9 만 처리하고 99 는 버린다 — 후속 후보.
- **사용자 실환경에서의 다이얼로그 발화 빈도** — 사용자 기본 설정에는 `permission` 키가 없어 승인 없이 실행된다. 시그니처 자체는 옳지만, 사용자가 `permission` 을 설정하지 않으면 다이얼로그 경로는 실사용에서 거의 안 탄다. 플러그인 인밴드 경로가 주 신호가 되어야 한다.
- **codex 축** — 이 탐침의 대상이 아니다(⑤ `/dev/tty` 전제 미확인).

### 9. 안전 / 잔여물

- 사용자 `~/.config/opencode`·`~/.codex` 는 읽기만 했고 **무변경**. `auth.json` 은 열지 않았다. 확인한 것은 키 이름뿐(`$schema`·`mcp`·`plugin`·`provider`·`agent`, `plugin: ['oh-my-openagent@latest']`, `permission` 키 없음).
- hexdump·정규화 발췌에 시크릿 없음(SGR 색상·CUP·UI 문구만). ps 로그에 `GITHUB_PERSONAL_ACCESS_TOKEN` 이 보이지만 **docker `-e` 인자의 변수 이름**이며 값은 없다.
- 다이얼로그는 Esc(=`reject`)로 거부했고, 승인은 0건.
- 정리 단계의 `rm -rf .../work/.opencode` 가 권한 거부돼 탐침 플러그인 2개와 `work/opencode.json` 이 스크래치패드에 남아 있다. 세션 격리 영역이며 사용자 레포·설정에는 영향이 없다.

---

## codex

## codex CLI 0.144.6 실측 탐침 결과

산출물: `/Users/gkn/taide/docs/research/2026-09-15-agent-probe-codex.md` (신규 1개. opencode 탐침과의 동시 충돌을 피하려고 계약이 지정한 합본 파일명 대신 codex 전용 파일로 썼습니다 — 메인에서 합치면 됩니다.)
캡처·스크립트: 세션 스크래치 `scratchpad/probe-codex/` (`probe.exp`, `capture-trust.raw` 1,491B, `normalize.py`, `codex-strings.txt`).

### 확정 (바이트 근거)

1. **전경 프로세스는 `node`** — `expect` pty 에서 `ps -o pid,pgid,stat,comm`: pgid 리더 78743 = `node /Users/gkn/.nvm/.../bin/codex …`, 네이티브 `codex`(78747)는 그 자식. 근거는 `@openai/codex/bin/codex.js:195` 의 `spawn(binaryPath, args, { stdio: "inherit" })`(exec 아님). 현행 `detect_agent_name` 의 node→cmdline basename 폴백이 `codex` 를 잡으므로 **신원 경로는 무변경으로 동작**합니다.
2. **다이얼로그는 `CSI row;col H` 로 단어마다 이동한다** (Claude 의 `CSI n G` 와 다름). 원시 바이트: `Do\e[3;6Hyou\e[3;10Htrust\e[3;16Hthe…`. `terminal_scan.rs` 의 `step_csi` 는 `H` 를 **개행**으로 치환하므로 정규화 결과가 `Do\nyou\ntrust\n…` 입니다 → **띄어쓰기 포함 다구 시그니처는 codex 에 절대 매치되지 않습니다.** codex 용 `dialog_signatures_for` 는 한 번의 쓰기로 나오는 연속 문자열만 써야 합니다.
3. **연속 출력이 확인된 문구(신뢰 다이얼로그 한정)**: `› 1. Yes, continue` · `Press enter to continue` · `You are in `.
4. 설치본 2종 공존: PATH 1위 `~/.bun/bin/codex`=**0.142.0**, `~/.nvm/.../bin/codex`=**0.144.6**(탐침 대상).

### [미확인] 과 사유

- **명령 승인 다이얼로그 문구**: 바이너리 strings 로 후보만 확인(` needs your approval.`, `Yes, and don't ask again for commands that start with \``, `Do you want to approve network access to "`, `Approval requested: `). **렌더 시 단어 분할 여부 미확인이라 상수 승격 금지.** 푸터 조각 ` to interrupt` 는 작업 중 푸터일 수 있어 시그니처 금지.
- **타이틀 OSC 0/2**: 캡처 구간 **0건**(나온 OSC 는 색상 질의 `\e]10;?`·`\e]11;?` 뿐, 대체화면 `?1049h` 도 없음). 단 바이너리에 `[tui].terminal_title` 과 항목 식별자 `run-state`·`project-name`·`thread-title`… 가 존재 → 능력은 있으나 기본 동작 미확인. **`TITLE_GLYPHS_FOR` 의 codex 칸은 비워 두는 것이 안전.**
- **스피너 글리프 / 유휴 프롬프트**: 미채취.
- **hooks command 훅의 `/dev/tty` 쓰기**: 판정 불가. 간접 근거로 codex 자신이 OSC 52 를 `/dev/tty` 에 쓰고, 샌드박스 프로파일이 `(allow file-read* file-write* (literal "/dev/tty"))` 를 명시 허용하지만, 훅 프로세스의 세션 분리(setsid) 여부를 확인 못 했습니다. 훅 이벤트 어휘 `PreToolUse`·`PermissionRequest`·`PostToolUse`·`UserPromptSubmit` 는 바이너리에서 확인(계약 §1.C 전제와 일치). **계약 §1.C 의 "인밴드 vs HTTP 유지" 분기는 아직 결정할 수 없습니다.**

### 막힌 지점 (우회하지 않고 멈춤)

스크래치 저장소에서 codex 를 띄우면 **디렉토리 신뢰 다이얼로그**가 먼저 뜹니다. "1. Yes, continue" 는 사용자 `~/.codex` 신뢰 상태를 변경하므로 고르지 않았고, `-c projects."<dir>".trust_level="trusted"` 오버라이드로는 우회되지 않았습니다(동일 다이얼로그 재현). 그래서 프롬프트 제출 이후 구간(스피너·타이틀·승인 다이얼로그·유휴)을 못 얻었습니다. 훅 시험용 임시 `CODEX_HOME` 경로는 기존 로그인 재사용에 자격증명 재배치가 필요해 환경 정책상 차단되었고, 그대로 중단했습니다.

재개 조건: 스크래치 디렉토리 1회 신뢰를 사용자가 허용하거나, 이미 신뢰된 임시 디렉토리를 지정해 주면 나머지 5개 칸을 한 세션으로 채울 수 있습니다.

레포 변경은 위 문서 1개뿐입니다(다른 `git status` 변경분은 제 작업이 아닙니다). `auth.json` 등 자격증명은 열지 않았고, 캡처 전량을 확인해 시크릿이 없음을 확인했습니다.
