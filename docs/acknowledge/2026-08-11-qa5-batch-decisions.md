# QA 5차(Phase 7.10) 범위·방식 결정 (2026-08-11)

> 신규 요구 8그룹 + UI 추가분 접수 후 리서치 워크플로(8영역, opus+medium,
> wf_83d6725a-ecb·wf_b3d1604c-da7) 결과를 바탕으로 사용자와 확정.
> 사용자 응답: "나머지는 다 추천대로" + Codex 는 아래 정정 반영.

## 1. 사용자 확정 결정

| 항목 | 결정 | 비고 |
|------|------|------|
| remote-control | **진행** — 동일 React 번들을 Rust HTTP 서버로 서빙 + `__TAURI_INTERNALS__` shim + WS-IPC 브리지. 웹 클라이언트는 **데스크톱과 동일 전권** | 외부 노출은 사용자가 cloudflare tunnel 로 직접. 잠자기 대응 스코프 밖 |
| 멀티 에이전트 | **B안** — 감지 배지 3종(claude·codex·gemini, 코어 이미 대응) + hooks 브리지 확장으로 Codex·Gemini 3상태 배지. **Codex IDE 연동(터미널→에디터 발견)은 아키텍처상 불가로 명시** | Gemini IDE companion·Codex app-server 패널은 backlog |
| auto-tab provider | **Ollama Cloud + Codex access token 2종** (사용자 원 요구 유지) | Codex 는 b-hub 검증 구현 이식 — §4 정정 참조 |
| extension | "테마/문법/스니펫 임포트"로 명명. 이번 단계는 **VSIX 로컬 임포트 + 테마 추출 + 관리 UI**. TextMate 문법 엔진(테마 파이프라인 개정 동반)은 별도 단계로 분리 | extension host 실행(코드 실행형)은 **미지원 공식 선언**. MS Marketplace 연동 금지(약관) |
| dead-end Edit 버튼 | **B안** — `system_open_app_data_path(kind)` 전용 커맨드 신설(`plugins`\|`themes`\|`locales` 열거값만, on-demand mkdir 후 Finder/기본 앱) + 플러그인 목록 실제 렌더 | 에디터 탭 내 앱데이터 편집(C안)은 보류(backlog) |
| 단축키 | **B안** — 죽은 바인딩 8개 전부 실핸들러(⌘B·⌃`·⌘\\·⌃Tab/⌃⇧Tab·⌘⇧E·⌃⇧G·전역 ⌘S) + ⌘+/⌘- 폰트 크기 신설 | chord(⌘K ⌘S)·when 엔진은 별도 작업(backlog). 편집 계열은 Monaco 내장 유지 |
| LSP 1차 범위 | 필수 9종(java·go·ruby·dart/flutter·swift·scala·haskell·elixir·**C/C++ clangd**) + 프리빌트 추천군 5종(lua-language-server·zls·taplo·terraform-ls·kotlin-lsp) | 전략: 하이브리드(사용자 기승인) — 자체 다운로드+체크섬 / 툴체인 감지+명령 대행 / SDK 탐지. npm 계열은 2차 |
| 신규 의존성 | Rust: **axum**(remote), **reqwest**(rustls-tls — AI·sync·LSP 공용), **keyring**(OS 키체인), **sha2·flate2·tar·zip**(LSP 설치) 승인 | 프론트 신규 없음(TextMate 채택 시에만 shiki 재논의) |

## 2. 메인 확정 세부 (추천안 적용)

- **remote**: 앱당 서버 1개 + session-id 는 프로젝트 매핑 불투명 난수, macro 디스패치 테이블 +
  collect_commands! 파리티 테스트, 데스크톱과 상태 완전 공유, 프로세스 수명 토큰(1회용 URL 토큰 →
  HttpOnly 쿠키 승격) + 설정 UI(링크 복사/폐기/접속 수), convertFileSrc 는 HTTP 라우트(Range 지원)로 대체
- **GitHub 동기화**: secret Gist + PAT(gist 스코프 단독), 페이로드는 **화이트리스트**(settings.json
  +themes/+locales/ — AI 토큰·절대경로 절대 미포함), 수동 업로드/다운로드 + 시작 시 최신 여부 배지,
  충돌은 updatedAt 비교 후 2택 모달, 설정 9번째 섹션(SYNC). "secret gist 는 비공개가 아님" 문구 주의
- **UI/UX**: 공용 헤더 높이 상수 신설 후 explorer 1행을 탭바(36px)에 정렬, 헤더 border 토큰은
  tabBar.tabBorder 로 통일, 메뉴바 하단 전폭 border 신설(레일·트리·탭 관통 직선), 줄번호
  `glyphMargin:false`+`lineNumbersMinChars:3`, 파일트리 hover 강화 + `explorer.itemFocused` 첫 배선 +
  결함 테마만 VSCode 공식 기본 list 색 폴백 보정(계열 교차 금지·대비 검증), 설정 좌측 패딩 축소
- **멀티 에이전트**: hooks 는 **사용자 레벨 설정에만 주입**(~/.codex/hooks.json·~/.gemini/settings.json
  — 프로젝트 파일은 커밋 대상이라 오염 금지), shim 은 `taide-cli` 서브커맨드(stdin→HTTP 중계,
  짧은 타임아웃 + 무조건 exit 0 + stdout 무오염), override 키 (ProjectId, agentName) 확장,
  Codex awaitingInput 해제는 PostToolUse/Stop + 프로세스 활동 신호
- **멀티 에이전트 hooks 파일 안전성(W4 보류분, W5 에서 처리)**: 서드파티 소유 hooks 파일
  (`.claude/settings.local.json`·`~/.codex/hooks.json`·`~/.gemini/settings.json`)은 파싱 실패
  시 빈 객체 대체 없이 항상 쓰기를 거부하고(`AppError` 반환), 재작성 시 기존 파일의 권한(mode)을
  보존하며 신규 생성 시에만 `0600` 을 적용한다 — `write_private_atomic` 의 범용 계약(항상 `0600`)
  은 바꾸지 않고 이 경로 전용 헬퍼로 격리(`agent-integration.md` §7.6).
- **LSP**: 스펙 enum→데이터화(기존 PluginLspContribution 스키마 발판), mason-registry 는 참고용만,
  Elixir 는 expert(alpha 배지 표기), 툴체인 자체 설치는 안 함(감지+대행 실행까지),
  매니페스트에 sha256 고정·`{app_data}/lsp/` 격리 설치
- **dead-end**: 플러그인 섹션은 목록 실렌더 + 새로고침 + 폴더 열기(코어 완비 실측 — "wired up in the
  core" 문구는 거짓이었음), `palette.notRunnable` 고아 키 삭제, plugin 오류 메시지 한국어 하드코딩 →
  i18n 화, 에러 토스트는 대표 건만 sonner action 부착, `git_init` 커맨드 신설
- **테마 reload 플래시 버그**: reveal 가드가 reload 미커버가 원인(메인 특정). 수정은 opus+high 1차
  검토로 확정 — localStorage 도입 지양·ADR-0004 존중 제약 하에 첫 페인트 전 적용 또는 페인트 지연

## 3. 진행 순서 (웨이브)

W1 UI/UX+단축키+dead-end+테마 플래시 → W2 LSP → W3 AI provider+auto-tab+GitHub sync →
W4 멀티 에이전트 hooks → W5 VSIX 임포트 → W6 remote-control.
backlog 추가: TextMate 문법 엔진, Gemini IDE companion, Codex app-server 패널, chord/when 키맵 엔진,
앱데이터 파일 에디터 편집.

## 4. 정정 사항 (중요)

- **Codex access token 으로 completion 호출은 가능하다.** 리서치의 "경로 부재" 판정은 오판 —
  사용자의 b-hub(`~/development/b-hub/service/domain/ai/providers/codex-provider.ts`)가
  `https://chatgpt.com/backend-api/codex/responses`(SSE)를 access token + `chatgpt-account-id`
  (whoami: `auth.openai.com/api/accounts/v1/user-auth-credential/whoami`) + `originator: codex_cli_rs`
  로 이미 운용 중. TAIDE 는 이 구현을 이식한다. 성격은 Claude Code IDE 연동과 동일한
  **비공식 프로토콜**(CLI 업데이트로 깨질 수 있음, QA 스모크로 감지)이며 사용자가 리스크를 인지하고
  확정했다. 상세: `docs/feedback/2026-08-11-codex-token-feasibility-misjudgment.md`
- 플러그인 설정 문구 "The plugin list command will appear here once it is wired up in the core" 는
  사실과 다름 — `plugin_list`/`plugin_reload` 는 코어·bindings·entities 까지 완비, UI 미소비가 유일한 결손.
