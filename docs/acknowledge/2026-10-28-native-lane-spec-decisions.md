# lane 사양 결정 (2026-10-28)

근거: 사용자 지시 "결정사항은 큰 변경사항 아니면 다 권장사항대로"(2026-10-26), "재개, 전부 추천안대로"(2026-10-27). 사양 문서는 `docs/research/2026-10-28-lane-<lane>-spec.md` 7개입니다.

## 채택

- 7개 사양의 4.1절 결정은 아래 예외를 빼고 모두 **추천안**을 채택합니다.
- 중복 범위 소유: 알림 생산자·첫 전달 안내·Sync 테마 재로드·보조 창 수명·IDE save/진단·외부 열기는 shell lane이 만듭니다. git 알림·IDE diff·diff 설정 소비는 git lane, reveal·searchOnType·검색 치환 알림·검색 편집기 본문(editor-81, shell-89의 searchEditor)은 explorer-search lane이 만듭니다. 나머지 lane은 진입점을 재사용하고 행 판정만 합니다.
- OS 알림 진입점은 shell lane의 경로 하나(`notify_event`)로 정하고 settings·git·explorer-search가 그것을 씁니다.
- 탐색기 Git 데코레이션: explorer-search lane이 파생·행 렌더·주입점을 만들고 git lane이 `git-cache` 공급을 잇습니다(D1 A). 비교·파일 히스토리 메뉴는 git lane입니다(D2·D3 A).
- 탭 내용 분기 hook은 wave 0 구조 분리 범위에 넣습니다(editor-preview 결정 5 A). wave 0이 이미 끝났으면 shell lane 묶음 1의 첫 작업입니다.
- 실행 중 터미널 scrollback(terminal-agent-11·settings-30)은 terminal-agent lane이 구현하고 settings lane은 소비합니다.
- `/usr/local/bin` CLI 설치 경로의 관리자 대화상자 취소: 원본은 취소해도 성공 문구를 보이지만 결함으로 보고 **성공 문구를 보이지 않습니다**. 명령(commands-ui-33)과 Settings CLI 행(settings-16)에 같은 규칙을 씁니다. commands-ui 결정 8의 A안 대신 terminal-agent 결정 7의 추천을 따릅니다.
- 단일 인스턴스 소켓은 `std::env::temp_dir()` 아래에 만들고 종료 때 지웁니다.
- git 검사는 기존 `remote-git-tests.rs`처럼 git CLI가 사용자 전역 설정을 읽기 전용으로 참조하는 수준을 허용합니다. fixture local config가 hook·서명·자격 증명을 덮습니다.

## 보류 (사용자 허락 필요)

| 항목 | 이유 |
| --- | --- |
| editor-57 TS/JS 내장 서비스의 임베디드 JS 엔진 | 새 대형 의존성. CSS/SCSS·HTML·JSON은 순수 Rust 이식으로 진행합니다 |
| 실제 OS 알림 표시, 시스템 설정 앱 열기 | OS 상태. 자동 검사는 가짜 platform 포트까지만 합니다 |
| `/usr/local/bin/taide` 설치·해제 | 시스템 경로 쓰기와 관리자 암호 |
| Keychain 쓰기(AI 토큰, GitHub PAT, 원격 비밀번호) | 사용자 키체인. 검사는 메모리 SecretStore만 씁니다 |
| 사용자 홈 hooks 파일, 실제 프로젝트 `.claude/settings.local.json` 쓰기 | 사용자 데이터 |
| 네트워크(LSP 서버 다운로드, gist, 모델 목록) | 외부 접속. 검사는 가짜 응답을 씁니다 |
| 실제 휴지통 사용 검사(`tests/explorer-delete.rs`, 추적 안 된 파일 discard) | 사용자 휴지통 |
| 실제 `taide` CLI → native 연결 | shell-15 실제 사용자 데이터 경로 허락 뒤 |
| 저장소 밖 crate 소스 확인(파일 선택 dialog의 부모 창) | 작업자 규칙. 확인 전에는 현재 동작을 유지합니다 |
| 실기(픽셀, IME, VoiceOver, 실제 PTY, 다중 창 focus) | 배치 31 |
