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
