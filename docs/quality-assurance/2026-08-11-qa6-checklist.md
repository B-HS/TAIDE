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

## QA6 후속 1차 — 사용자 보고 5건 반영분 (2026-08-12, 계약: docs/acknowledge/2026-08-12-qa6-followup-contract.md)

- [ ] (IME) 에디터 한글 연속 타이핑 시 상하 떨림 소멸 — 기본 스택/커스텀 폰트 각각, 터미널 한글 회귀 없음
- [ ] (IME) 영→한 전환 직후 자모 분리는 **수정 대상 아님**(WKWebView 층·backlog) — 빈도 변화만 관찰
- [ ] (i18n) 앱 시작 직후 어떤 화면에서도 raw key(`settings.xxx` 형태)가 보이지 않는가 — 특히 영어 로케일
- [ ] (단축키) 설정 KEYMAP 섹션 버튼/커맨드 팔레트로 모달 오픈, 전 커맨드(23행) 나열·미할당 표기
- [ ] (단축키) 텍스트 검색·"키로 검색" 토글, 행별 변경/기본값 복원/해제, 충돌 경고·상대 해제
- [ ] (단축키) 캡처 중 mod+S 등이 실행되지 않고 캡처만 되는가, 캡처 중 Escape 가 모달을 닫지 않는가
- [ ] (단축키) window.reload 등 원래 미할당 커맨드에 키를 바인딩하면 실제 실행되는가, 팔레트 표기 동기
- [ ] (OMLX) 설정 AI 섹션에서 base URL 저장(기본 http://localhost:8000)·해제(빈 값)·선택 API key 저장
- [ ] (OMLX) 실서버 기동 상태에서 모델 목록 조회, auto-tab provider 로 선택 시 FIM 완성 동작
      (qwen-coder 등 코더 모델 권장 — 미지 패밀리는 chat 폴백), 서버 다운 시 UI 에러 표시
- [ ] (툴팁) 아이콘 버튼 hover 시 툴팁 표시(사이드바·검색·git·테마 에디터·프리뷰 등), disabled 버튼 포함
- [ ] (툴팁) 탭 닫기 aria 라벨에 파일명 보간 정상({{title}}), git 패널 문구가 언어 설정 따라 번역되는가

## 기능 확장 1차 — blame·에디터 액션·taide CLI·퀵윈 (2026-08-13, 계약: docs/acknowledge/2026-08-13-feature-expansion-contract.md)

- [ ] (blame) git 저장소 파일에서 커서 정지 시 blame 이 **포커스 줄 위** 작은 글자로 뜨는가,
      auto-tab ghost text 와 더 이상 겹치지 않는가, 커서 이동 시 본문이 튀지 않는가(커서 줄 고정),
      1행에서도 표시되는가, 탭 전환 후 정상 재표시되는가
- [ ] (액션) 커맨드 팔레트에 에디터 액션들이 카테고리와 함께 뜨는가(예: "Trigger Suggest"),
      실행되는가. macOS 에서 제안 위젯은 Cmd+I 로 뜨는가(Ctrl+Space 는 OS 입력 소스 전환이 선점 —
      알려진 OS 제약)
- [ ] (액션) 단축키 모달에 monaco 행이 기본 바인딩(chord 포함 표기)과 함께 나열되는가,
      재바인딩 → 새 키로 실행·기본 키 비활성, 해제·기본값 복원, 재시작 후에도 유지되는가
- [ ] (액션) 에디터 포커스가 없을 때 monaco 액션 키가 오발동하지 않는가, 팔레트에서 에디터 없이
      실행 시 무해한가
- [ ] (CLI) 팔레트 "Shell Command: Install 'taide' command in PATH" → 관리자 프롬프트 → 설치,
      설정 에이전트 섹션 상태 갱신. dev 빌드에서는 거부 안내가 뜨는가(정상 — 번들 빌드에서만 설치 가능)
- [ ] (CLI) 번들 빌드(bun run tauri build) 후: taide <파일> 로 앱에 파일이 열리는가(앱 꺼진
      상태 콜드스타트 포함), taide . 으로 폴더가 프로젝트로 열리는가, taide --wait 가 탭 닫기로
      해제되는가, Uninstall 로 심링크 제거되는가
- [ ] (퀵윈) sticky scroll 이 중첩 코드 스크롤 시 상단 고정으로 보이는가, 설정 토글로 꺼지는가
- [ ] (퀵윈) 상태바에 "Ln N, Col M" 커서 위치가 실시간 갱신되는가(탭 전환 포함)
- [ ] (퀵윈) 커서를 심볼 위에 두면 동일 심볼이 강조되는가(documentHighlight — LSP 언어),
      스마트 선택 확장(⌃⇧⌘→ 또는 팔레트 Expand Selection)이 의미 단위로 커지는가(selectionRange)
- [ ] (회귀) 기존 단축키(⌘S·⌘P·⌘B 등)·앱 커맨드가 이중 실행 없이 1회만 도는가

## 기능 확장 3차 — Hot Exit·Remote 비밀번호 (계약: docs/acknowledge/2026-08-14-hotexit-remote-password-contract.md)

- [ ] (hot-exit) 파일 탭에 입력 → 강제종료(`kill -9` 또는 활동 모니터) → 재시작 → 해당 탭 재활성화
      시 미입력분까지 복원되는가(500ms 디바운스 내 최근 입력 포함), 복원 시 배너("편집 복원됨")가
      뜨는가
- [ ] (hot-exit) 복원 직전 디스크가 외부에서 변경된 상태로 재시작 시 충돌 배너("디스크 변경됨")가
      뜨고 "디스크 보기"/"내 것 유지" 선택이 정상 동작하는가
- [ ] (hot-exit, untitled) 제목 없는 탭에 입력 → 강제종료 → 재시작 시 탭 자체와 내용이 함께
      복원되는가(이전엔 재시작 시 소실이 정상이었음 — 이번에 변경된 동작)
- [ ] (hot-exit) 정상 종료(⌘Q, 창 닫기 버튼 둘 다) 시에도 마지막 입력이 유실 없이 미러에
      반영되는가 — 특히 ⌘Q(앱 메뉴 Quit)로 종료 시 창 X 버튼과 동일하게 저장 후 종료되는지 확인
      (내부적으로 종료 핸드셰이크 경로가 다르므로 별도 확인 필요)
- [ ] (hot-exit) 탭을 닫으면 그 탭의 미러가 사라지고, 재시작 후 다시 열어도 복원 후보에 안 뜨는가
- [ ] (hot-exit) 저장(⌘S) 또는 "디스크 보기" 선택 후에는 재시작해도 더 이상 복원 배너가 뜨지
      않는가(미러가 정리됨)
- [ ] (remote) 설정 REMOTE 섹션에서 비밀번호 설정 → "링크 생성"으로 새 브라우저 접속 시 로그인
      폼이 뜨는가(링크만으로 즉시 접속되면 안 됨), 올바른 비밀번호 입력 시 세션이 발급되어
      TAIDE UI 가 뜨는가
- [ ] (remote) 로그인 폼에서 틀린 비밀번호를 반복 입력 시 잠금(지수 백오프)이 걸리고 남은 시간이
      표시되는가, 시간 경과 후 재시도가 풀리는가
- [ ] (remote) 설정에서 "비밀번호만으로 접속 허용" 토글 on 시, 링크 없이 서버 URL만으로 접속해도
      로그인 폼이 뜨고 비밀번호로 세션이 발급되는가. 토글 off(기본)에서는 링크 없이 URL만으로는
      401/거부되는가
- [ ] (remote) 비밀번호를 변경(또는 해제)하면 기존에 접속해 있던 다른 브라우저 세션이 즉시
      로그아웃(재인증 요구)되는가
- [ ] (remote) 비밀번호를 설정하지 않은 기본 상태에서는 기존처럼 링크 클릭만으로 즉시 접속되는가
      (하위호환 — 링크만 모드가 깨지지 않았는지)
- [ ] (remote) 링크를 2번 발급해 폰·노트북을 동시에 로그인 시도할 때 둘 다 정상적으로 세션을
      받는가(먼저 발급된 기기가 "비밀번호 오류"로 튕기지 않는가)

## 기능 확장 2차 — 재바인딩 수정·blame footer·usage 모달 (2026-08-13)

- [ ] (재바인딩) Trigger Suggest 에 Opt+Space 재바인딩 → 모달에 ⌥SPACE 표시 → 에디터에서
      Opt+Space 로 제안 위젯이 뜨고 공백이 삽입되지 않는가. Opt+문자(예: Opt+K) 캡처도 물리 키로
      저장되는가. 바인딩 불가 키·수식어 없는 키는 토스트가 뜨는가
- [ ] (blame) blame 이 에디터 페인 하단 footer 바에 표시되는가(줄 위 존 소멸), 분할 시 pane 별
      자기 커서 줄, 미커밋 줄은 "You, now" 표기, footer 높이가 커서 이동에도 흔들리지 않는가
- [ ] (usage) 상태바 CPU/RAM 클릭 → 모달: 앱/터미널(프로젝트명)/LSP(서버명·프로젝트명)/에이전트
      그룹별 프로세스·CPU·메모리 표시, 3초 갱신, 첫 표시 직후 CPU 가 "--"(null)에서 값으로
      채워지는가, 모달을 닫았다 열어도 값이 정상인가, **상태바 CPU% 가 모달 사용 중에도 오염되지
      않는가**(System 인스턴스 분리 검증)
- [ ] (suggest 키) ⌥Space 트리거 후 **⌥ 를 유지한 채** ↑/↓ = 위젯 항목 탐색(수정 전엔 라인
      이동·위젯 닫힘), ⌥ 뗀 뒤 ↑/↓ 도 탐색 — 만약 이 경우에도 커서가 이동하면 제안이 1건뿐인
      상황인지 확인(1건+포커스 시 ↑=커서 이동은 VS Code 동일 스펙) 후 아니면 보고. 위젯 닫힌
      상태 ⌥↑ = 라인 이동 원복. 함수 인자 입력 중 suggest+parameter hints 동시 열림에서 ⌥↑/⌥↓
      시그니처 전환이 살아 있는가. Enter/Tab 수락·Esc 닫기 정상

## Wave A — LSP 인텔리전스 (계약: docs/acknowledge/2026-08-14-wave-a-lsp-intelligence-contract.md)

> LSP 세션이 붙는 언어(rust-analyzer·vtsls·basedpyright·marksman 등)의 파일에서 확인. 항목 대부분은
> rust-analyzer(.rs) 로 1차 확인 후 vtsls(.ts)로 교차 확인을 권장(서버 광고 capability 차이 확인).

- [ ] (quickfix) 진단(빨간 줄) 위에서 ⌘. → Quick Fix 목록이 뜨는가, 적용 시 편집이 실제로 반영되고
      해당 진단이 사라지는가. rust-analyzer 미열린 다른 파일에 걸친 fix(예: import 추가)도 정상
      적용되는가(WorkspaceEdit 적용기가 미열린 파일 IPC 경로를 타는지)
- [ ] (source action) ⌘. 또는 우클릭 메뉴에서 source.organizeImports/source.fixAll 계열이 뜨고
      적용되는가(kind 계층 필터 확인 — refactor 등 무관 kind 가 섞이지 않는가)
- [ ] (on-save) 설정 EDITOR 섹션에서 "저장 시 import 정리"/"저장 시 자동 수정" 토글 on → 지저분한
      import 가 있는 파일을 ⌘S 로 명시 저장 시 자동 정리되는가. autoSave(디바운스 자동저장)로는
      **정리되지 않는가**(명시적 저장만 대상 — 의도된 동작). 매우 느린/응답 없는 서버에서 5초
      초과 시 스킵 토스트가 뜨고 저장 자체는 멈추지 않는가
- [ ] (F12/⌘F12) 정의로 이동이 **다른 파일**을 대상일 때 실제로 그 파일 탭이 열리고 해당 위치로
      커서가 이동하는가(cross-file — 오프너 부재 시 무음 실패했던 항목). 같은 파일 내 이동은 탭
      전환 없이 즉시 이동하는가
- [ ] (Peek) ⌥F12(Peek Definition)·Shift+F12(Peek References) 가 **탭이 열려 있지 않은 파일**의
      내용도 위젯 안에서 미리보기로 렌더되는가(빈 화면/에러 없이)
- [ ] (F8) F8/⇧F8(다음/이전 문제로 이동)이 **다른 파일**의 진단으로도 넘어가는가(파일 간 순회)
- [ ] (구현/타입/선언 이동) rust-analyzer 에서 "Go to Implementation"·"Go to Type Definition" 이
      동작하는가. gopls/tsls 처럼 declaration 미지원 서버에서는 해당 메뉴가 조용히 없거나
      비활성인가(에러 없음)
- [ ] (References) Shift+F12 참조 목록에 cross-file 결과가 포함되고, 그 중 하나를 클릭하면 해당
      파일이 열리며 이동하는가
- [ ] (CodeLens) rust-analyzer 에서 함수 위 "N references" 등 CodeLens 가 표시되는가, 클릭 시
      참조 Peek 또는 이동이 실제로 동작하는가(showReferences/gotoLocation 커맨드 배선 확인).
      설정 토글 "CodeLens 표시"를 끄면 다음 재계산 시점(편집 또는 서버 refresh)에 사라지고,
      다시 켜면 다음 재계산 시점에 다시 나타나는가(즉시 반영은 아님 — 알려진 한계)
- [ ] (folding) 함수/블록 좌측 거터의 접기 화살표가 LSP 기반으로 뜨는가(대형 파일은 기존 정책대로
      비활성 유지), 접기/펼치기 정상 동작
- [ ] (ra 파일 생성형 액션) rust-analyzer 의 "Extract to module" 등 파일을 새로 만들거나 이름을
      바꾸는 코드 액션이 에러 없이 적용되는가(CreateFile/RenameFile 리소스 오퍼레이션 경로)
- [ ] (rename) ⇧F6/F2 심볼 rename 이 **미열린 파일**에 걸친 참조까지 포함해 정상 적용되는가
      (기존엔 미열린 파일 rename 이 한계였던 항목 — 적용기 통일로 해소)
- [ ] (didSave) 저장 후 LSP 서버가 반영한 진단이 갱신되는가(didSave 미전송이던 기존 결함 확인)
- [ ] (initializationOptions) rust-analyzer 설정(예: checkOnSave)이 매니페스트값대로 실제 서버에
      전달되는가 — 간접 확인: rust-analyzer 상태가 정상 초기화·인덱싱되는가(무설정 대비 차이가
      없다면 정상, checkOnSave 관련 진단이 예상대로 뜨면 강한 확인)
- [ ] (회귀) 기존 LSP 기능(자동완성·hover·서명 도움말·문서 심볼·rename prepare·formatting·
      documentHighlight·selectionRange·inlayHints)이 이번 변경 후에도 정상 동작하는가
- [ ] (세션 재사용) 같은 프로젝트에서 여러 언어 파일을 오가도 CodeLens/CodeAction/이동 계열이
      중복 등록 징후(같은 액션이 목록에 2번 뜨는 등) 없이 정상 동작하는가

## Wave E — 터미널·태스크 (계약: docs/acknowledge/2026-08-15-wave-e-terminal-tasks-contract.md)

> zsh 를 기본 셸로 우선 확인(macOS 기본값). bash·fish 는 `shell_profiles` 로 프로젝트별 오버라이드
> 후 교차 확인. 셸 통합은 새로 연 터미널 탭에서만 적용된다(이미 열린 세션은 재주입 안 됨).

- [ ] (zsh 셸 통합) 새 터미널 탭에서 `echo $PATH`·`echo $HOMEBREW_PREFIX`(또는 `.zshenv`/`.zprofile`
      에 심어둔 커스텀 env)가 일반 터미널.app 과 동일하게 나오는가 — Homebrew 기본 설치처럼
      `~/.zprofile` 에 `eval "$(brew shellenv)"` 가 있는 환경에서 `brew`·`/opt/homebrew/bin` 경로가
      살아있는가
- [ ] (zsh 명령 블록) 명령 실행 후 좌측 거터에 색 막대(성공=success 색, 실패=failure 색)가 붙는가,
      `⌘↑`/`⌘↓` 로 이전/다음 명령 시작 줄로 스크롤되는가
- [ ] (zsh 사용자 rc 무결성) 기존에 쓰던 `.zshrc`/`.zshenv`/`.zprofile` 의 커스텀 프롬프트·alias·
      함수가 TAIDE 터미널에서도 전부 그대로 동작하는가(깨지거나 사라지는 것 없어야 함)
- [ ] (bash 셸 통합) 프로젝트 셸을 bash 로 오버라이드한 새 터미널 탭에서 명령 블록 데코가
      뜨는가. macOS 기본 bash(3.2) 사용 시 exit code 배지(D 이벤트)는 정상, 명령 실행 시작 시점
      마킹(C, output-start)은 뜨지 않는 것이 **알려진 한계**임을 인지(정오표 §4 참고)
- [ ] (fish 셸 통합) fish 4.0+ 환경에서 별도 주입 없이도 명령 블록 데코가 뜨는가(네이티브 지원)
- [ ] (opt-out) `TAIDE_SHELL_INTEGRATION=1` 을 미리 export 한 뒤 TAIDE 를 실행하면 새 터미널
      탭에서 명령 블록 데코가 전혀 뜨지 않는가(주입 스킵 확인)
- [ ] (Run Task) 팔레트에서 "Run Task" 실행 → package.json scripts·Makefile 타겟·(Cargo 프로젝트면)
      `cargo build/test/run/check/clippy` 5종이 목록에 뜨는가. 선택 시 포커스된 터미널(없으면 신규
      탭)에서 올바른 명령이 실행되는가
- [ ] (Run Task 소스 배지) npm/make/cargo 소스 라벨이 항목별로 올바르게 표시되는가
- [ ] (Run Selected Text) 에디터에서 텍스트를 선택한 뒤 팔레트/컨텍스트 메뉴로 "Run Selected Text
      in Terminal" 실행 → 포커스 터미널(없으면 신규 탭)에 선택 텍스트가 개행과 함께 입력·실행되는가
- [ ] (Run Selected Text 폴백) 아무것도 선택하지 않은 상태로 같은 커맨드를 실행하면 커서가 있는
      현재 줄이 대신 실행되는가
- [ ] (회귀) 기존 터미널 기능(스크롤백 복원·flow control·파일 링크·폰트 크기·검색·복사/붙여넣기)이
      이번 변경 후에도 정상 동작하는가

## Wave F — 에디터 표현 (계약: docs/acknowledge/2026-08-15-wave-f-editor-presentation-contract.md)

> Semantic Tokens·Format on Type/Paste 는 Rust(.rs, rust-analyzer)·TypeScript(.ts, vtsls)·Go
> (.go, gopls) 각 1파일에서 교차 확인(서버별 지원 차이가 크다 — `features/editor.md` §8/§9 표
> 참고). 스니펫·Emmet 은 언어 무관하게 1회 확인으로 충분하다.

- [ ] (semantic 색 on/off) 설정 EDITOR 섹션의 "Semantic Highlighting" 토글이 기본 켜져 있는가.
      .rs 파일에서 로컬 변수·매개변수·타입 등이 syntax 색과 구분되는 semantic 색으로 보이는가
      (워시아웃 없음 — 매핑 안 되는 토큰이 기본 전경색으로 덮이지 않는가). 토글을 끄면 편집 없이도
      즉시 semantic 색이 사라지고 기존 syntax 색만 남는가(재등록 없이 반영, onDidChange 발화로
      즉시 재계산 — 2026-08-15 검토에서 "다음 편집까지 반영 안 됨" 회귀 수정), 다시 켜면 즉시
      다시 나타나는가
- [ ] (일반 구문 색 워시아웃 없음) 여러 번들 테마(예: github-dark·vitesse-dark·vscode-dark-plus)로
      바꿔가며 .ts/.rs 파일을 열어, semantic highlighting 이 꺼져 있어도 일반 구문 강조(변수·
      키워드·문자열·주석) 색이 테마 고유 색과 일치하는가(semantic rule append 가 실제 구문 색을
      바꾸던 회귀가 2026-08-15 검토에서 수정됨 — 특히 github-dark 의 변수 주황색, vitesse-dark 의
      주석 색 확인)
- [ ] (semantic delta) .rs 파일을 열어 둔 채 일부만 편집했을 때도 편집과 무관한 나머지 줄의
      semantic 색이 깜빡이거나 사라지지 않는가(delta 재인코딩 정합성)
- [ ] (semantic Rust) .rs 에서 self·라이프타임·매크로 호출 등 rust-analyzer 특유 토큰이 합리적인
      색으로(최소한 어색한 미매핑 색 없이) 보이는가
- [ ] (semantic TypeScript) .ts 에서 semantic 색이 정상 표시되는가(vtsls, delta 미지원 — 매 편집마다
      전체 재요청되어도 깜빡임 외의 오류가 없는가)
- [ ] (semantic Go) .go 파일에서 semantic 색이 표시되는가(gopls 는 매니페스트
      `initializationOptions.semanticTokens: true` 로 활성화 — 기본값 그대로면 무동작이었을 항목).
      표시되지 않는다면 "조용한 0토큰"(계약 §3.1 마지막 항목) 재현 여부를 보고
- [ ] (format on type) 설정에서 "Format on Type" 켠 뒤 .rs 파일에서 `.`·`=`·`{` 등 트리거 문자
      입력 시 자동 포매팅이 즉시 적용되는가(rust-analyzer). .go 파일에서는 트리거가 없으므로
      아무 일도 일어나지 않는 것이 정상인가(gopls 미지원)
- [ ] (format on paste) 설정에서 "Format on Paste" 켠 뒤 들여쓰기가 흐트러진 코드를 .rs/.ts 파일에
      붙여넣었을 때 자동으로 재포맷되는가. .go 파일에서는 rangeFormatting provider 자체가
      게이팅되어 아무 일도 일어나지 않는가(gopls 미지원 — 알려진 한계)
- [ ] (스니펫 생성→완성 노출→삽입) 설정 SNIPPETS 섹션 "Manage Snippets" 진입 → 새 스니펫 파일
      생성(특정 언어 또는 전역) → prefix/body 입력 후 저장 → 설정 화면을 나가서 해당 언어
      파일에서 그 prefix 를 타이핑하면 completion 목록에 Snippet 항목으로 뜨는가 → 선택 시
      body 가 탭스톱(`$1`, `${1:default}`)과 함께 정상 삽입되는가
- [ ] (스니펫 completion 즉시성) TAIDE 를 재시작한 뒤(설정 화면을 한 번도 열지 않은 새 세션)
      기존에 저장해 둔 스니펫이 첫 파일을 연 순간부터 completion 에 뜨는가(부트스트랩
      `QueryObserver` 구독 확인 — 이 항목이 실패하면 "설정을 먼저 열어야만 동작"하는 회귀)
- [ ] (스니펫 completion 10분 뒤 지속성) 설정 화면을 한 번도 열지 않은 채 10분(전역 `gcTime`)
      이상 기다린 뒤에도 같은 prefix 로 completion 이 계속 뜨는가(옵저버 없는 `prefetchQuery` 만
      쓰던 시절엔 GC 로 캐시가 비어 이 시점부터 영구히 빈 목록이 되는 회귀가 있었다 —
      2026-08-15 검토에서 발견·수정)
- [ ] (스니펫 삭제) 저장한 스니펫/스니펫 파일을 삭제하면 completion 목록에서도 사라지는가
- [ ] (스니펫 파일명 검증) 새 스니펫 파일 생성 시 `.json`/`.code-snippets` 외 확장자나 `/`·`\`·
      `..`·`:` 가 포함된 이름이 다이얼로그 단계에서 확인 버튼 비활성화로 막히는가(`:` 는
      Windows 드라이브 상대경로 탈출 방지 — 2026-08-15 검토에서 추가), 우회해 저장 요청을
      보내면 "파일 이름" 관련 에러 메시지가 뜨는가(JSON 오류 메시지와 구분되는가)
- [ ] (Open Snippets Folder) 설정 SNIPPETS 섹션의 "Open Snippets Folder" 버튼이 Finder 에서
      `snippets/` 디렉토리를 여는가(존재하지 않으면 생성 후 열기)
- [ ] (emmet html) .html 파일에서 `div.container>ul>li*3` 같은 Emmet 약어를 입력하면 completion
      목록에 확장 결과가 뜨고, 선택(Tab/Enter) 시 마크업으로 확장되는가
- [ ] (emmet css) .css/.scss 파일에서 `m10-20-30-40` 같은 약어가 `margin: 10px 20px 30px 40px;`
      로 확장되는가
- [ ] (emmet jsx) .tsx 파일에서 HTML 계열 약어가 JSX 로 확장되는가(className 등 JSX 속성명 사용).
      알려진 한계: `}`/`>`/`*` 등 비-단어 문자로 끝나는 약어는 .tsx/.jsx/.heex 에서 자동으로
      제안이 뜨지 않을 수 있다(`emmet-monaco-es` 의 트리거 문자 테이블에 이 언어들의 별칭이
      없음 — `features/editor.md` §11) — `⌃Space` 로 수동 호출하면 정상 확장되는지만 확인
- [ ] (emmet 토글) 설정에서 "Emmet Abbreviations" 를 끄면 위 확장 completion 이 더 이상 뜨지
      않고, 다시 켜면 재시작 없이 다시 뜨는가(dispose/재등록 확인)
- [ ] (회귀) 기존 LSP 기능(자동완성·hover·정의 이동·rename·formatting·codeAction·codeLens·
      진단)이 이번 변경 후에도 정상 동작하는가

## Wave G — AI: Inline Edit·AI 커밋 메시지 (계약: docs/acknowledge/2026-08-16-wave-g-ai-contract.md)

> Inline Edit 은 ViewZone/ContentWidget 레포 최초 도입 — 스크롤·워드랩·접기·다중 split pane 환경의
> 실제 렌더는 자동 테스트로 커버되지 않으므로 이 실기 확인이 유일한 검증이다. `features/ai.md`
> 가 상세 정본.

- [ ] (진입) 설정 AI 섹션에서 provider(Ollama Cloud 또는 Codex)+모델을 선택해 둔 상태에서, 에디터에서
      텍스트를 선택하고 `⌘I` 를 누르면 선택 영역 위에 입력창(ContentWidget)이 뜨는가. 선택 없이
      `⌘I` 를 누르면 현재 줄이 선택으로 승격되어 같은 위젯이 뜨는가
- [ ] (팔레트) 커맨드 팔레트에서 "AI: Edit Selection" 을 검색해 실행해도 같은 위젯이 뜨는가(⌘I 와
      동일 액션 트리거 확인)
- [ ] (제출→로딩→프리뷰) 입력창에 지시문을 쓰고 Return 을 누르면 스피너+"생성 중" 문구로 전환되고,
      응답이 오면 원본 선택 영역에 삭제(빨간 계열) 데코가 씌워지고 그 아래 제안 코드가 구문 강조된
      채로 나타나는가(ViewZone)
- [ ] (모델 무변경 — 핵심 불변식) 프리뷰가 떠 있는 동안 파일 내용 자체는 바뀌지 않았는가: 탭 제목에
      dirty 점(●)이 뜨지 않는가, 좌측 git gutter 표시가 그대로인가, 프리뷰 상태에서 앱을 종료했다가
      재시작해도(Hot Exit) 그 편집이 반영되지 않은 원본 그대로 복원되는가
- [ ] (수락·undo 1회) `⌘Enter` 또는 Accept 버튼으로 수락하면 실제로 텍스트가 교체되고 dirty 점이
      뜨는가. 그 직후 `⌘Z` 를 **한 번**만 눌렀을 때 수락 이전 원본으로 완전히 되돌아가는가(두 번
      이상 나눠 되돌아가지 않는가)
- [ ] (거절) 프리뷰 상태에서 `Esc` 또는 Reject 버튼을 누르면 데코·제안 코드가 사라지고 원본이 전혀
      바뀌지 않은 채 위젯이 닫히는가
- [ ] (생성 중 취소) 로딩 상태에서 `Esc` 또는 취소(×) 버튼을 누르면 요청이 취소되고 토스트가 뜨는가
- [ ] (빈 응답) 지시문이 모델이 코드를 반환하지 않을 만한 내용일 때(또는 provider 오류 시) 실패/빈
      응답 토스트가 뜨고 위젯이 정리되는가
- [ ] (편집 중 무효화) 생성 중이거나 프리뷰가 떠 있는 상태에서 같은 파일의 다른 곳을 타이핑하면
      위젯·데코·프리뷰가 즉시 사라지는가(stale 프리뷰가 남지 않는가)
- [ ] (세션 1개 제한) 위젯이 열려 있는 상태에서 `⌘I` 를 다시 누르면 새 위젯이 쌓이지 않고 기존
      입력창에 포커스만 이동하는가
- [ ] (탭 전환 정리) 위젯이 열린 채로 다른 탭으로 이동하면 위젯이 자동으로 정리되는가
- [ ] (코드펜스 스트립) 모델이 ```` ```lang ... ``` ```` 로 감싼 응답을 줘도 실제 삽입되는 코드에는
      펜스가 남지 않는가
- [ ] (커밋 메시지 생성) SCM 패널에서 파일을 stage 한 뒤 커밋 입력란 우측 상단 Sparkles 버튼을
      누르면 로딩 스피너로 바뀌고, 완료되면 diff 내용을 반영한 커밋 메시지가 입력란에 채워지는가
      (이미 텍스트가 있어도 덮어써지는가 — 확인 다이얼로그 없음, 의도된 동작)
- [ ] (staged 없음) stage 된 변경이 없으면 Sparkles 버튼이 비활성이고, hover 시 "스테이지된 변경
      사항 없음" 류 툴팁이 뜨는가
- [ ] (생성 중 취소) 커밋 메시지 생성 중 같은 버튼을 다시 누르면 취소되는가
- [ ] (절삭/제외 표기) 매우 큰 diff(예: 대용량 파일 다수 변경)를 stage 한 뒤 생성하면 성공 토스트에
      "diff가 잘렸습니다" 문구가 붙는가. `bun.lock`/`package-lock.json` 등 lock 파일을 stage 에
      포함해 생성하면 "N개 파일 제외" 문구가 붙는가(lock 파일 내용이 커밋 메시지에 언급되지
      않는가)
- [ ] (실패) provider 토큰이 없거나 잘못된 상태로 생성을 시도하면 실패 토스트가 뜨는가, 그
      description 에 원인(예: "AI provider is not configured")이 함께 표시되는가
- [ ] (긴 선택 영역) 60줄 이상의 함수를 선택해 편집을 요청했을 때 응답이 중간에 잘리지 않는가(잘리면
      절삭 에러 토스트로 실패해야 하며, 잘린 코드가 그대로 프리뷰/삽입되면 안 된다)
- [ ] (시크릿 파일 제외) `.env`/`id_rsa` 등을 stage 한 뒤 커밋 메시지를 생성해도 그 내용이 diff 로
      전송되지 않는가(skippedFiles 에 파일명만 나열)
- [ ] (자동완성 회귀) 설정 필드 리네임(`aiAutoTabProvider`→`aiProvider` 등) 이후에도 기존 auto-tab
      ghost text 제안이 그대로 동작하는가(W3 체크리스트 재확인)
- [ ] (프롬프트 오버라이드) `~/Library/Application Support/dev.taide.app/prompts/inline-edit-default.json`
      또는 `commit-message-default.json` 을 직접 작성해 두면(스키마: `{version, system, user}`)
      재시작 후 그 프롬프트로 요청이 나가는가(네트워크 탭/프록시로 직접 확인은 어려우므로, 시스템
      프롬프트에 눈에 띄는 문구를 넣어 응답 스타일이 바뀌는지로 간접 확인해도 됨)
- [ ] (원격 세션) remote-control 로 연결한 세션에서도 ⌘I·커밋 메시지 생성이 동일하게 동작하는가
