# PRD — TAIDE (Tauri AI-integrated Development Environment)

> 제품 요구사항의 정본. 기능 상세는 `docs/features/`, 기술 결정 근거는 `docs/adr/`, 구현 순서는 `docs/roadmap.md`.
> 결정·합의 이력: `docs/acknowledge/`.

## 1. 비전

터미널 AI 에이전트(Claude Code 등) 시대에 맞는 **가볍고 누수 없는 개인용 커스텀 IDE**.
VSCode/Cursor 의 핵심 편집·Git 경험과 warp 수준의 터미널을 하나의 Tauri 앱에 담되,
Electron 의 무게 없이, 데이터는 Rust 코어가 소유하고 view 는 표시만 하는 구조로 만든다.
프로젝트를 잘 추상화해 두어 추후 프로젝트별 remote-control(자체 서버 + 웹 패널) 등
기능을 계속 부착할 수 있는 **개인 개발 플랫폼의 기반**이 되는 것이 최종 목표다.

## 2. 목표 / 비목표

### 목표

- 폴더 단위 프로젝트를 여러 개 동시에 열고 오가는 멀티 프로젝트 IDE
- VSCode 기본 단축키가 그대로 통하는 코드 편집(Monaco) + LSP(JS/TS·Rust·Python·Markdown)
- VSCode 수준의 Git 통합(gutter, inline blame, changes, split diff, graph, commit/push GUI)
- 사용자 기본 셸을 그대로 쓰는 터미널 + AI 에이전트 감지·연동(Claude Code ctrl+g 외부 편집 등)
- 색상을 세밀하게 나눈 테마 시스템 — 앱 전체와 codeview 가 같은 테마를 따름
- 탭·스플릿·프로젝트 상태가 재시작 후 완전 복원
- 메모리 누수 없는 장시간 구동 — 상태는 Rust 소유, view reload 로도 유실 없음

### 비목표 (현 단계에서 하지 않음)

- VSCode 확장(extension) 생태계 호환 — TAIDE 자체 플러그인 체계만 설계
- 원격 개발(SSH remote), 협업 편집, 디버거(DAP) — 추후 검토
- 5개 언어 외 LSP 내장 — 플러그인으로 확장
- ~~앱 자체의 AI 기능 내장 — 자체 AI 는 추후~~ → **이후 구현됨**(Wave G — 자동완성·Inline
  Edit·커밋 메시지, provider 3종. 정본: `features/ai.md`. 채팅 패널은 여전히 비목표)
- ~~remote-control 실제 구현 — 추상화까지만 설계~~ → **이후 구현됨**(W6~ — `domain/remote`
  WS 서버·인증·정책. 정본: `features/remote-control.md`·d-38 계약)

## 3. 참조 제품

| 제품 | 참조하는 것 |
|------|------------|
| VSCode | 에디터 단축키·gutter·diff·SCM 패널·탭 동작 전반 (`docs/research/vscode-behaviors.md`) |
| Cursor | 사이드바 상단 파일/검색/Git 구성, 전반적 레이아웃 |
| warp | 터미널 UX(명령 블록, 편의 기능) |
| iTerm2 | 터미널 부가 기능(검색, 링크, 폰트 제어 등) |
| GitLens | 인라인 blame 표기 형식 |
| Git Graph(확장) | 커밋 그래프 시각화 |

## 4. 사용자

- 1차 사용자: 개발자 본인(단일 사용자). 다중 사용자·계정·동기화는 고려하지 않는다.
- macOS 를 주 환경으로 개발하되, Windows/Linux 에서 동작 가능한 설계를 유지한다.

## 5. 핵심 설계 원칙

1. **Rust 소유 상태**: 프로젝트·탭·터미널·Git·LSP 등 모든 도메인 상태의 단일 출처는 Rust 코어다.
   view(React)는 조회·표시·입력 전달만 한다. view 를 리로드해도 Rust 상태로부터 즉시 복원된다.
   → `docs/adr/0004-rust-owned-state.md`
2. **프로젝트 추상화**: 프로젝트는 "폴더 + 부착 가능한 기능(capability) 집합"으로 모델링한다.
   파일뷰·터미널·Git·LSP·(미래의) remote-control 은 모두 프로젝트에 부착되는 capability 다.
   → `docs/architecture.md` §프로젝트 추상화
3. **누수 금지**: 모든 구독(이벤트 리스너, watcher, pty, LSP 세션)은 소유자와 수명이 명확해야 하고,
   해제 경로가 코드 리뷰에서 확인 가능해야 한다. → `docs/features/` 각 문서의 "수명주기" 절
4. **VSCode 관성 존중**: 단축키·UI 동작은 사용자가 이미 아는 VSCode 동작을 기본값으로 한다.
   임의로 새 UX 를 발명하지 않는다.

## 6. 기능 요구사항 (FR)

> FR A~J 는 기획 시점 분류이고 **전항 구현 완료**됐다. 이후 추가 구현된 대형 기능(AI 3종·IDE
> MCP 서버·remote-control·작업 러너·멀티 윈도우·Zen 모드·VSIX 임포트·스니펫 등)은 FR 로 소급
> 등재하지 않았다 — 각 `features/*.md` 와 `docs/acknowledge/` 계약이 정본이다.

상세 스펙은 각 `docs/features/*.md` 가 정본이다. 여기서는 요구사항 전량을 번호로 고정한다.

### A. 프로젝트 / 레이아웃 (`features/layout-shell.md`)

- **FR-A1** 폴더 단위 프로젝트를 기본으로 한다. 프로젝트 열기 = 폴더 선택.
- **FR-A2** 앱 레이아웃은 `앱 사이드바 | content` 구조다.
- **FR-A3** 앱 사이드바에 여러 프로젝트를 동시에 띄울 수 있고, 클릭으로 활성 프로젝트를 전환한다.
- **FR-A4** 앱 사이드바의 프로젝트 항목 아이콘은 그 프로젝트에서 현재 focus 된 content 타입
  (파일뷰·터미널·설정·code agent 실행 중 등)에 따라 바뀐다.
- **FR-A5** 앱 재시작 시 열린 프로젝트 목록·활성 프로젝트가 복원된다.

### B. 탭 / 스플릿 (`features/tabs.md`)

- **FR-B1** content 상단에 메뉴 탭 바를 표시한다.
- **FR-B2** 새 프로젝트는 기본으로 1 파일뷰 + 1 터미널 탭을 가진다.
- **FR-B3** 탭은 DND 로 순서를 바꿀 수 있다.
- **FR-B4** 탭의 순서·열린 목록·활성 탭은 앱을 껐다 켜도 복원된다.
- **FR-B5** 탭을 content 영역으로 드래그하면 스플릿 뷰(에디터 그룹 분할)가 생성된다.
- **FR-B6** 탭 타입: 파일뷰(에디터), 터미널, 설정, diff 뷰, (확장 가능한 타입 체계).

### C. 탐색 사이드바 (`features/explorer-sidebar.md`)

- **FR-C1** content 좌측에 VSCode/Cursor 와 동일한 구성의 사이드바를 둔다.
- **FR-C2** 상단 전환: 파일(Explorer) / 검색(Search) / Git(SCM) — Cursor 스타일.
- **FR-C3** 파일 트리는 VSCode 처럼 트리 형식, 폴더 접기/펼치기, lazy load, 파일 시스템 변경 실시간 반영.
- **FR-C4** 트리에서 파일 클릭 시 content 에 코드가 열린다(단일 클릭 = preview 탭, 더블 클릭/수정 = 고정).
- **FR-C5** 검색 뷰는 프로젝트 전역 텍스트 검색(결과 트리, 파일 이동)을 제공한다.

### D. 코드 에디터 (`features/editor.md`)

- **FR-D1** Monaco 기반 codeview. 앱 테마를 그대로 따른다(토큰 색까지 테마 시스템에서 파생).
- **FR-D2** git 연동 gutter: 추가/수정/삭제 라인을 VSCode 와 같은 규칙으로 표시한다.
- **FR-D3** 커서가 있는 줄에 인라인 blame 을 표시한다. 형식: `HS, 4 days ago, {commit message}`
  (author 축약, 상대 시각, 커밋 메시지 — GitLens 참조).
- **FR-D4** VSCode 기본 단축키를 전부 지원한다(멀티커서·라인 조작·찾기/바꾸기·go to definition 등).
  Monaco 내장 키맵 + 부족분 보강. 목록: `docs/research/vscode-behaviors.md`.
- **FR-D5** 파일 저장/dirty 표시, 탭 전환 시 편집 상태(커서·스크롤·selection) 보존.

### E. LSP (`features/lsp.md`)

- **FR-E1** 1차 내장 지원: JS/TS, Rust, Python, Markdown (+ JSON 등 Monaco 내장 수준).
- **FR-E2** 자동완성, 진단(에러/경고), hover, go to definition/references, rename, 포맷 등
  LSP 표준 기능을 에디터에 연결한다.
- **FR-E3** 그 외 언어는 플러그인 설치로 LSP 를 추가할 수 있는 구조로 만든다(`features/plugins.md`).
- **FR-E4** LSP 서버는 Rust 코어가 프로세스로 관리한다(기동·재시작·종료·크래시 복구).

### F. Git (`features/git.md`)

- **FR-F1** Git 뷰에 현재 remote 주소, 브랜치, changes 목록, 커밋 그래프를 표시한다.
- **FR-F2** changes 는 git 분류(추가/수정/삭제/이름변경/미추적/충돌)와 staged/unstaged 그룹으로 표시한다.
- **FR-F3** changes 의 파일 클릭 시 split(side-by-side) diff 를 연다.
- **FR-F4** 파일 context menu: 변경 되돌리기(discard), stage(git add), unstage, 해당 파일로 이동(diff 가 아닌 실제 파일 열기).
- **FR-F5** 커밋 메시지 입력 + Commit / Push 를 GUI 로 제공한다.
- **FR-F6** 커밋 그래프는 브랜치 레인·라벨·HEAD 를 표시한다(Git Graph 확장 참조).
- **FR-F7** 그 외 세부 동작은 VSCode SCM 을 참조 기준으로 한다.

### G. 터미널 (`features/terminal.md`)

- **FR-G1** 사용자의 기본 셸을 그대로 사용한다(macOS: 로그인 셸(zsh 등), Windows: PowerShell·Git Bash 등 선택,
  Linux: $SHELL).
- **FR-G2** 터미널 출력의 파일 경로를 cmd(ctrl)+click 하면 해당 프로젝트의 새 탭으로 파일이 열린다.
- **FR-G3** cmd(ctrl) + `+` / `-` 로 터미널 글자 크기를 조절한다.
- **FR-G4** iTerm 수준의 편의 기능(스크롤백 검색, 링크, 복사 등)을 가능한 범위에서 지원한다.
- **FR-G5** 터미널 세션은 탭 전환·view reload 에도 유지된다(pty 는 Rust 가 소유).

### H. AI 에이전트 연동 (`features/agent-integration.md`)

- **FR-H1** 터미널에서 AI 에이전트(claude 등) 실행을 감지해 앱 사이드바 아이콘에 반영한다(FR-A4).
- **FR-H2** Claude Code 의 외부 에디터 편집(ctrl+g)이 TAIDE 의 새 탭으로 열리고, 저장·닫기가
  Claude Code 로 정확히 돌아가는 흐름을 지원한다(EDITOR/CLI `--wait` 메커니즘).

### I. 테마 (`docs/theme-system.md`)

- **FR-I1** 전체 레이아웃의 테마(색)를 조절할 수 있다. 색상 토큰은 최대한 세밀하게 나눈다.
- **FR-I2** codeview(구문 토큰 포함)와 터미널(ANSI 16색 포함)이 같은 테마 정의에서 파생된다.
- **FR-I3** 내장 다크/라이트 테마 + 사용자 커스텀 테마(파일 기반) 지원.

### J. 플러그인 / 확장 포인트 (`features/plugins.md`)

- **FR-J1** LSP 추가를 1차 유스케이스로 하는 플러그인 체계를 설계한다.
- **FR-J2** 프로젝트 capability 추상화에 remote-control 등 미래 기능을 부착할 수 있어야 한다.

## 7. 비기능 요구사항 (NFR)

- **NFR-1 (무게)** Electron 배제. 설치 용량·상주 메모리를 최소화한다(Monaco·xterm 자산 포함 합리적 수준).
- **NFR-2 (누수)** 장시간(수일) 구동에도 메모리가 단조 증가하지 않는다. 모든 구독·프로세스·모델의
  해제 경로를 문서·코드에서 추적 가능해야 한다.
- **NFR-3 (복원력)** view 프로세스가 리로드되어도 Rust 상태에서 전체 UI 가 복원된다.
  앱 재시작 시 프로젝트·탭·스플릿·(가능한 범위의) 터미널 표시 내용이 복원된다.
- **NFR-4 (반응성)** 입력 지연 체감 없음: 에디터 타이핑, 터미널 echo, 탭 전환은 즉각 반응해야 한다.
  터미널 대량 출력 시 UI 가 멎지 않아야 한다(flow control).
- **NFR-5 (규모)** 수만 파일 프로젝트에서 파일 트리·검색·git status 가 실용적으로 동작한다
  (가상화, lazy load, 캐시).
- **NFR-6 (크로스플랫폼)** macOS 우선 개발. Windows/Linux 에서 컴파일·동작 가능한 추상화를 유지하고
  플랫폼 분기는 Rust 코어에 격리한다.
- **NFR-7 (보안)** Tauri capabilities 최소 권한. 렌더러에 임의 fs/셸 접근을 열지 않고
  정의된 IPC 명령만 노출한다(`docs/ipc-contract.md`).

## 8. 성공 기준 (문서화 이후 구현 단계에서 검증)

1. TAIDE 로 TAIDE 자신을 개발(dogfooding)할 수 있다 — 편집·터미널·커밋·푸시가 TAIDE 안에서 완결.
2. VSCode 에서 손에 익은 단축키·Git 플로우가 재학습 없이 통한다.
3. 앱 재시작 후 이전 작업 상태(프로젝트·탭·스플릿)가 그대로 돌아온다.
4. Claude Code 를 TAIDE 터미널에서 실행하고 ctrl+g 편집이 TAIDE 탭으로 왕복한다.
5. 24시간 이상 켜 두어도 메모리 사용이 안정적이다.

## 9. 리스크 / 열린 문제

| 리스크 | 대응 |
|--------|------|
| monaco-languageclient 의 Tauri(비 WebSocket) 브릿지 복잡도 | `docs/research/monaco.md` 결과 기반으로 ADR-0007 에서 전송 계층 확정. 실패 시 WebSocket 로컬 서버 fallback |
| Windows 터미널(ConPTY)·셸 다양성 | portable-pty 가 흡수. macOS 우선 개발 원칙으로 리스크를 뒤로 배치 |
| git push 인증(ssh agent/PAT) 엣지 케이스 | 1차는 ssh-agent·credential helper 위임. 실패 UX 를 명확히 |
| Git graph 렌더 자체 구현 비용 | 레인 배치 알고리즘을 research 로 확보, MVP 는 단순 레인부터 |
| Claude Code ctrl+g 동작의 버전 변화 | 표준 $EDITOR + `--wait` 규약에만 의존(특정 앱 내부 구현에 비의존) |
| 대형 파일에서 Monaco 성능 | VSCode 정책 참조한 크기 임계값·기능 축소 정책(`features/editor.md`) |
