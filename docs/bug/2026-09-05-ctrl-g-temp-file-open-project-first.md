# Claude Code Ctrl+G 가 "먼저 프로젝트를 여세요" 로 끝남 — 임시파일이 루트 밖이라 열리지 않던 문제 (수정)

> 사용자 실기 보고(2026-09-05): "아직도 ctrl + g 를 claude terminal 에서 하면 open a project first 라고 뜬다".
> 기능 정본은 `docs/features/agent-integration.md` §2, 진행 상태는 `docs/PROCESS.md` "라이선스·README·Ctrl+G" 절.

## 증상

TAIDE 내장 터미널에서 `claude` 를 띄우고 Ctrl+G 를 누르면 TAIDE 창이 앞으로 오지만 탭은 열리지 않고
"Open a project first" 토스트만 뜬다. 프로젝트가 분명히 열려 있는데도 그렇다. Claude Code 쪽은
곧바로 편집 종료로 인식해 프롬프트가 그대로 돌아온다.

## 원인 (2축 — 전부 소스로 확인)

1. **프론트가 "파일을 품은 프로젝트" 만 허용했다.** `app/providers/agent-external-open-provider.tsx`
   는 CLI 로 넘어온 경로를 `openProject` 로 먼저 시도하고, 실패하면 열린 프로젝트 중 root 가 그 경로를
   포함하는 것을 찾았다. 없으면 `app.openProjectFirst` 토스트 + 마커 즉시 해제. Claude Code 의
   Ctrl+G 는 `$EDITOR` 에 **OS tmpdir 아래 임시파일**(`/var/folders/.../T/...`)을 넘기므로 어떤
   프로젝트 root 에도 속할 수 없고, 항상 이 분기로 떨어졌다. 설계 문서(§2.1 "활성(또는 최근)
   프로젝트의 새 탭으로 연다")와 구현이 어긋나 있었고, v0.1.6 의 EDITOR 주입은 기계 검증만 됐지
   실기 왕복은 미확증이었다(HANDOFF §7).
2. **Rust root guard 가 루트 밖 파일을 전부 거부했다.** 프론트를 고쳐 활성 프로젝트로 열더라도
   `layout_open_tab`(`ensure_file_tab_target_exists`)·`file_open`·`file_save`·`file_read_raw` 가
   `root_guard::resolve_owning_project` 로 `Forbidden` 을 던진다. 이 경계는 webview·원격 세션이
   임의 경로를 읽지 못하게 하는 보안 장치라 통째로 풀 수 없다.

기각된 가설: EDITOR 주입 실패(§2.3 경로 우선순위 — 로그상 주입은 정상), single-instance 중계 누락
(이벤트·큐 모두 도달), 마커 검증 실패(`validate_wait_marker_path` 는 tmpdir 마커를 허용).

## 해결

- **Rust 허용 목록 (`AppState::cli_opened_paths`)**: CLI 로 명시 전달된 경로만 정규화해 기록한다.
  기록 지점은 OS 수준 진입점 2곳뿐 — cold-start argv 와 single-instance 중계 — 이고 둘 다
  `agent::commands::queue_external_open` 한 함수로 합쳤다(2회 룰). IPC 로는 추가할 수 없어
  webview·원격이 경계를 넓힐 수 없다.
- **`root_guard::resolve_owning_project_or_cli_opened`**: 기존 resolver 를 먼저 타고, 실패 시
  정규화 경로가 허용 목록에 있으면 `(None, resolved)` 로 통과. 나머지는 기존 `Forbidden` 그대로.
  소비처는 `layout_open_tab`/`layout_open_tab_in_split`·`file_open`·`file_save`(미러 정리는
  프로젝트가 있을 때만)·`file_read_raw` 4종. IDE MCP `openFile` 도구·파일 트리 변경 커맨드·미러
  커맨드는 엄격 경계 유지.
- **프론트 `resolveExternalOpenTarget`** (`entities/agent/external-open-target.ts`): 품은 프로젝트
  (가장 긴 root 우선) → 활성 프로젝트 → 첫 프로젝트 순으로 대상 결정. 프로젝트가 0개일 때만
  `openProjectFirst`. `--wait` 요청은 **preview 가 아닌 고정 탭**으로 연다 — `open_tab` 은 기존
  preview 탭을 자리 교체하며, 교체는 닫힘이 아니라 마커가 영영 해제되지 않기 때문. 열리면
  `app.externalEditorTabHint` 토스트("저장한 뒤 이 탭을 닫으면 Claude Code 로 돌아갑니다").
- **에디터 게이팅 (`editor-pane.tsx` `isOutsideProjectRoot`)**: 프로젝트 root 밖 경로의 탭은 LSP
  세션·저장 시 코드 액션·format-on-save·hot-exit 미러를 붙이지 않는다(§2.1 의 "임시파일 탭에는
  포맷터/린터/LSP 를 붙이지 않는다" 를 처음으로 실제 구현).

## 검증

- Rust: `root_guard`(허용 목록 통과·미등록 거부·루트 안 정상)·`state`(정규화 기록)·
  `layout::commands`(허용 목록 + 존재 검증) 테스트 신규, 기존 7건 시그니처 갱신. fmt·clippy 0.
- 프론트: `external-open-target.test.ts` 6건. typecheck·lint(기존 warning 11)·prettier·bun test 통과.
- 실기: dev 인스턴스에 `TAIDE_APP_PATH=target/debug/taide target/debug/taide-cli --wait <tmp 파일>`
  로 열기 — 활성 프로젝트의 고정 탭으로 열려 본문이 표시됐다(스크린샷). 탭 닫힘→CLI 종료는 합성 입력이
  WKWebView 에 닿지 않아 자동화로 확인하지 못했고, 기존 경로(`releaseClosedFileTabPath` 단위 테스트)에
  의존한다. 실제 Claude Code 왕복은 설치본 갱신(v0.1.8) 후 사용자 확인 — 내장 터미널에 주입되는 EDITOR 는
  `/usr/local/bin/taide` 심링크 → 설치본 사이드카 → `/Applications/TAIDE.app` 을 스폰하므로 dev 빌드로는
  왕복이 안 된다.

## 남는 한계

- **SIGTERM/kill/크래시 종료 시 wait 마커가 남는다(기존 동작)**: `cleanup_all_wait_markers` 는
  `RunEvent::Exit`/`ExitRequested`(Cmd+Q·창 닫기)에서만 돌고 시그널 핸들러가 없다. 실기 중 dev 앱을
  SIGTERM 으로 내리자 마커가 남아 CLI 가 계속 대기했다(타임아웃 기본 30분). 수정 범위 밖 — 백로그 후보.

- 앱을 재시작하면 허용 목록은 비므로, 레이아웃에 남아 복원된 임시파일 탭은 `Forbidden` 안내를
  보인다. Claude Code 는 편집이 끝나면 임시파일을 지우므로 실사용에서는 어차피 닫을 탭이다.
- 프론트의 root 포함 판정은 문자열 기준(`isWithinRoot`)이라 심링크로 다른 철자를 가진 루트 안
  파일은 "루트 밖" 으로 분류돼 LSP 가 붙지 않는다. Rust 는 canonicalize 기준이라 열기·저장은 정상.
