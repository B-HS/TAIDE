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
