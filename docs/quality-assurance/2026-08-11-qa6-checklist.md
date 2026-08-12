# QA 6회차 체크리스트 — Phase 7.10 W1~W7 일괄 실기 검증

> W7 완료 후 사용자가 `bun run tauri dev` 로 직접 실행해 확인한다. 웨이브 완료 시마다 항목을 누적한다.

## W1 — UI/UX·단축키·dead-end·테마 플래시

- [ ] 메뉴바(신호등 행) 아래 가로선이 레일·파일트리·에디터 탭 위를 오차 없이 관통하는 1자 직선인가
- [ ] explorer 뷰 전환 아이콘 행과 에디터 탭바의 하단 경계선이 같은 높이·같은 색인가
- [ ] 에디터 줄번호 좌측 여백이 좁아졌는가 (glyph margin 제거 체감)
- [ ] 파일트리 hover 가 전 테마에서 보이는가 (특히 Dark+·Darcula·Night Owl·Everforest)
- [ ] 파일트리 선택 행: 트리 포커스 시 더 진하고, 포커스를 잃으면 옅어지는가 (Dark+/Light+ 포함)
- [ ] gitignore 파일 행이 흐려지지 않고 텍스트 색만 회색인가
- [ ] 설정 화면 좌측 여백이 줄었는가
- [ ] reload(⌘⇧P → 창 새로고침) 시 선택 테마 그대로 뜨는가 (TAIDE Dark 플래시 소멸, 짧은 공백은 정상)
- [ ] 단축키: ⌘B 사이드바 / ⌃` 터미널 토글 / ⌘\ 분할 / ⌃Tab·⌃⇧Tab 탭 순환(WKWebView 도달 여부 확인 필요) / ⌘⇧E·⌃⇧G 뷰 전환 / ⌘S 전역 저장(에디터 밖 포커스에서도) / ⌘=·⌘- 폰트 크기(상태바 표시 동기)
- [ ] ⌘S 가 1회만 저장되는가, 같은 파일을 두 pane 에 열고 저장 시 올바른 쪽이 저장되는가
- [ ] 설정 플러그인 섹션: 목록/빈 상태 렌더, 새로고침, 폴더 열기(Finder, 폴더 자동 생성)
- [ ] git 패널 미저장소 상태에서 "저장소 초기화" 버튼 동작
- [ ] 열 수 없는 파일에서 "외부 앱으로 열기", LSP 미설치 행에서 설치 명령 복사
- [ ] 커맨드 팔레트에서 이전 비활성 8개 커맨드 실행 가능

## W2 — LSP 원클릭

- [ ] 설정 LSP 섹션에 18종 표시, 전략별 UI(설치 버튼/툴체인 안내/SDK 감지) 구분
- [ ] 다운로드형 1종 실설치(권장: taplo 소형) — 진행률 표시 → 완료 → 해당 언어 파일에서 LSP 동작
- [ ] 미서명 다운로드 바이너리가 Gatekeeper 에 막히지 않는가 (막히면 보고 — 정책 재설계 필요)
- [ ] clangd 설치 후 .c/.cpp 파일 진단·정의 이동
- [ ] 툴체인형: go 설치된 상태에서 gopls 대행 설치 동작(또는 미설치 안내)
- [ ] SDK 형: Xcode 있는 환경에서 sourcekit-lsp 감지 표시
- [ ] expert(alpha)·kotlin-lsp 행에 실험 표기
- [ ] 신규 확장자(.java·.rb·.dart·.swift·.scala·.ex·.hs·.kt·.lua·.zig·.tf 등) 하이라이팅
- [ ] zls 설치(tar.xz 해제 경로) 후 .zig 파일에서 동작

## W3 — AI Provider·auto-tab·GitHub 동기화
- [ ] 설정 AI 섹션: Ollama Cloud API key 저장(재시작 후 유지 — keychain), Codex access token 저장(whoami 검증)
- [ ] settings.json 파일에 토큰이 없는지 직접 확인 (`~/Library/Application Support/dev.taide.app/settings.json`)
- [ ] auto-tab: 에디터 타이핑 시 ghost text 표시 → Tab 수락, 빠른 타이핑 시 요청 폭주 없음
- [ ] Codex provider 로 모델 목록·완성 실동작 (Business 토큰)
- [ ] GitHub 동기화: PAT 연결 → 업로드 → gist 생성 확인 → 설정 변경 후 다운로드 복원, 충돌 2택
- [ ] gist 내용에 토큰·절대경로가 없는지 확인
- [ ] Codex 토큰 저장 시 잘못된 토큰이면 저장 거부(whoami 실패) 확인
- [ ] auto-tab 제안이 LSP 자동완성 팝업과 충돌하지 않는가
- [ ] 커맨드 팔레트의 동기화 업로드/다운로드 2개 커맨드 동작

## W4 — Codex·Gemini hooks 배지

- [ ] 설정 에이전트 섹션: claude(프로젝트별)·codex/gemini(사용자 레벨) 행 구분, taide-cli 미설치 시 토글 비활성+안내
- [ ] codex 설치 → ~/.codex/hooks.json 에 TAIDE 항목 생성·기존 사용자 항목 보존 확인, 제거 시 원상복구
- [ ] gemini 동일 (~/.gemini/settings.json)
- [ ] 내장 터미널에서 codex 실행 → 프롬프트 제출 시 Working, 승인 요청 시 NEED DECISION, 완료 시 Idle
- [ ] gemini 실행 → BeforeAgent/Notification/AfterAgent 배지 전이
- [ ] 같은 프로젝트에서 claude+codex 동시 실행 시 배지가 섞이지 않는가 (override 에이전트 축)
- [ ] TAIDE 재시작 후(포트 변경) 기존 주입 hook 이 자동 재조정되는가
- [ ] TAIDE 를 끈 상태에서 codex/gemini 턴이 지연되지 않는가 (shim 1.5s 타임아웃·exit 0)

## W5 — VSIX 테마 임포트

- [ ] 설정 테마 섹션 "VSIX 에서 가져오기" → 실제 .vsix(예: One Dark Pro 마켓 vsix)로 테마 추출·목록 표시
- [ ] 선택 저장 후 커스텀 테마 목록·테마 피커에 즉시 나타나고 적용되는가
- [ ] 기존 id 와 충돌하는 vsix 재임포트 시 확인 다이얼로그 → 사본 저장되는가
- [ ] 변환 실패 테마가 사유와 함께 비활성 표시되는가
- [ ] bun run themes:convert (CLI) 가 이식 후에도 동일하게 동작하는가

## W6 — remote-control (원격 접속)

> 서버 기동은 사용자가 `bun run tauri dev` 후 설정 REMOTE 섹션에서. 외부 접속은 cloudflare tunnel 로 직접.
> 주의: 에이전트 셸에서 앱 실행 금지 — 이 항목은 사용자 실기 전용.

- [ ] 설정 하단 REMOTE 섹션: 활성화 토글·상태(중지/포트)·접속 수·보안 경고 문구 표시
- [ ] 토글 ON → 상태가 "포트 N에서 실행 중"으로 바뀌는가 (5초 내 갱신), OFF → 중지
- [ ] "접속 링크 생성" → URL 발급 + 클립보드 복사 토스트(HTTPS/tunnel 환경), URL 이 화면에 1회 표시
- [ ] 발급 URL 을 **같은 기기 다른 브라우저**(http://127.0.0.1:PORT/?t=...)로 열기 → TAIDE UI 가 그대로 렌더되는가
- [ ] 1회용 링크로 최초 접속 후 쿠키 승격 → 새로고침해도 재인증 없이 유지, 링크 재사용은 거부(1회용)
- [ ] 웹에서 파일 트리 탐색·파일 열기·에디터 표시가 데스크톱과 동일 상태로 보이는가 (상태 완전 공유)
- [ ] 웹에서 데스크톱의 변경(파일 저장·git·탭 전환)이 실시간 반영되는가 (이벤트 팬아웃 22종)
- [ ] 웹에서 터미널(pty) 출력이 스트리밍되는가 (바이너리 채널·xterm), 입력이 반영되는가
- [ ] 웹에서 검색(search_run 채널)·LSP(lsp_spawn 채널) 스트림이 동작하는가
- [ ] 이미지/동영상 미리보기(convertFileSrc → /__taide/file Range 라우트)가 웹에서 재생되는가
- [ ] 웹에서 "폴더 열기/저장" 다이얼로그 호출 시 hang 없이 취소 처리되는가 (호스트에 네이티브 창 안 뜸)
- [ ] "모든 기기 로그아웃" → 열려 있던 웹 세션이 무효화되는가
- [ ] cloudflare tunnel(HTTPS) 경유 폰 브라우저 접속 시 clipboard·미디어 등 secure context API 동작
- [ ] 앱 종료 시 원격 서버가 함께 종료되는가 (포트 해제)
- [ ] 데스크톱 앱 자체 동작에 영향이 없는가 (shim self-gate — 원격 미사용 시 무영향)
- [ ] (dev) vite 프록시 경유로 원격이 최신 프론트를 서빙하는가 (HMR 은 미지원 정상)

## W7 — TextMate 문법 엔진 (shiki)

> 계약: docs/acknowledge/2026-08-12-w7-textmate-contract.md. 정적으로 검증 못 한 항목이 많아
> 실기 우선순위가 높다. 특히 첫 항목(CSP)은 실패 시 W7 전체가 무효 — 최우선 확인.

- [ ] **CSP 무변경 동작(최우선)**: 앱 기동 후 에디터 하이라이팅이 나오는가. devtools 콘솔에
      CSP 위반(EvalError·wasm)·shiki 로드 오류가 없는가 (tauri.conf.json csp 무수정 상태에서)
- [ ] **신규 하이라이팅 9종**: .tsx / .jsx / .sh / .toml / .jsonc / .erb / .hs / .zig / .heex 파일이
      plaintext 가 아니라 구문 강조되는가 (W7 전에는 9종 전부 무강조였음 — 각각 개별 확인)
- [ ] 기존 언어(rust·ts·js·py·md·html·css·json 등)의 강조가 W7 전보다 세밀해졌는가
      (예: TS 에서 데코레이터·타입 파라미터·정규식 리터럴 색 구분)
- [ ] .heex 가 html 문법으로 폴백 강조되는가 (계약 결정 — elixir 삽입부는 미강조 정상)
- [ ] 테마 전환(설정 > 외관): 번들 테마 간 전환 시 에디터 구문색이 즉시 함께 바뀌는가,
      원본 테마의 전체 tokenColors 색 다양성이 보이는가 (31토큰 축소 대비 풍부해야 함)
- [ ] Dark+ / Dark Modern: 제어 키워드(if·return)가 보라(#C586C0)로 보이는가 (W7 재변환 수정),
      Dark Modern 파일트리에서 선택 항목 배경이 구분되는가 (W7 전 invisible 버그 수정 확인)
- [ ] 시스템 다크/라이트 추종 전환 시 에디터 색 정상 추종
- [ ] 테마 에디터: syntax 토큰 편집 시 라이브 프리뷰·실제 에디터 반영(오버레이 — 넓은 스코프라
      원본의 세분화 색 일부는 안 바뀔 수 있음이 문서화된 정상), 저장 후 재적용 정상
- [ ] 커스텀 테마(raw tokenColors 없음)가 폴백(31토큰 역생성)으로 정상 강조되는가
- [ ] VSIX 테마 임포트 후 그 테마의 원본 tokenColors 가 보존·적용되는가 (W5 경로 + W7 보존)
- [ ] TS/JS 리치 기능 유지: 진단(빨간 줄)·자동완성·호버가 shiki 교체 후에도 동작하는가 (worker 경로)
- [ ] 에디터 첫 페인트·타이핑 지연 체감 저하 없음 (shiki 는 에디터 로드 후 비동기 초기화),
      대용량 파일(large tier)에서 토큰화로 인한 프리즈 없음 (tokenizeTimeLimit 500ms)
- [ ] 플러그인 grammar: {app_data}/plugins/ 에 tmLanguage 기여 플러그인 배치 → 해당 확장자 파일
      강조, "플러그인 다시 읽기" 로 재생성 반영, 잘못된 grammar(scopeName 없음)는 설정 UI 에
      grammar-invalid 사유로 비활성 표시 (en/ko/ja 문구)
- [ ] 원격(W6) 브라우저에서도 하이라이팅·테마 전환이 데스크톱과 동일하게 동작하는가
- [ ] 번들 36종 전 테마 육안 스팟 확인(최소: one-dark-pro·dracula·github-light·night-owl·
      catppuccin-mocha·vscode-light-plus) — 배경/전경/구문색 이상 없음. night-owl 등 10종은
      W7 재변환으로 일부 색이 W6 시점과 다름(원인 규명·채택 완료 — PROCESS W7-C 참조)
- [ ] vite build 산출물 크기 확인: grammar 청크가 lazy 분리되어 초기 로드에 포함되지 않는가
