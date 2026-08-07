# QA 1차 반영(Phase 7.7) 범위·방식 결정 (2026-08-07)

> 사용자 QA 9그룹 접수 후 리서치 워크플로(8영역) 결과를 바탕으로 사용자와 확정한 결정.
> 리서치 원문: 세션 scratchpad `research/*.md` (요지는 본 문서와 PROCESS.md 에 반영).

## 1. 운영 방식 (사용자 확정)

| 역할 | 담당 |
|------|------|
| 오케스트레이팅·상세플랜·계약 확정 | 메인 세션 (Fable) |
| 리서치·판단·최종 검증 | opus + medium |
| 구현 | sonnet + high |
| 버그 1차 검토·해결 | opus + high |
| 버그 2차 검토·해결 | Fable + medium |

메인 세션은 직접 구현하지 않는다. 판단/리서치/구현/디버그는 workflow 로 진행한다.

## 2. 사용자 확정 결정 3건

| 항목 | 결정 | 근거 |
|------|------|------|
| Claude Code 연동 범위 | **전체 구현** — 연결(WS MCP 서버+lockfile+env 주입)+openFile+openDiff(수락/거부)+선택영역 공유+열린 에디터 목록 + **diff 내 편집 후 저장(FILE_SAVED)** + **진단 push(getDiagnostics)** | 프로토콜은 비공식(리버스 엔지니어링 교차검증)임을 인지하고 진행. CLI 업데이트로 깨질 수 있음 → QA 스모크로 감지 |
| AI 상태 NEED DECISION 판정 | **hooks 브리지 포함** — idle/working 은 휴리스틱, awaitingInput 은 Claude Code hooks(Notification/Stop 등)를 프로젝트 `.claude/settings.local.json` 에 주입해 판정. 동의 게이트(설정 기본 OFF) + 주입/제거 UI 필수 | 휴리스틱만으로는 API 대기와 입력 대기를 구분 불가 — 오탐이 배지 가치를 죽인다 |
| footer CPU/RAM 측정 범위 | **TAIDE 메인 프로세스만** + 툴팁에 범위 명시 | macOS WKWebView 헬퍼는 PPID=1 이라 귀속 불가(실측). 자식 합산은 터미널 작업이 IDE 사용량으로 잡히는 부작용 |

## 3. 메인이 확정한 세부 결정 (추천안 적용 — 이견 시 재논의)

| 항목 | 결정 | 근거 |
|------|------|------|
| `file_delete` 휴지통 전환 | `remove_dir_all` → `trash` 크레이트 | `docs/ipc-contract.md`·`explorer-sidebar.md` 가 이미 휴지통을 명시 — 문서와 코드 불일치 해소. context menu 에 Delete 를 노출하기 전 필수 |
| `TabKind::ClaudeDiff` 휘발성 | variant 는 추가하되 **레이아웃 영속 저장에서 제외**(저장 시 필터링) | 영속 스키마에 남기면 다운그레이드 시 역직렬화 파손. "TabKind 확장 금지" 결정의 취지(영속 리스크)를 지키면서 탭 UX 확보 |
| Find File References | **백로그 유지** | LSP `references` 는 심볼 위치 기반 — "파일에 대한 참조"는 프로토콜상 부재. 억지 구현 = 가짜 UI |
| 파일트리 다중 선택 | **이번 범위 밖** — context menu 는 단일 대상 전제 | 선택 모델 교체가 선행돼야 함 |
| Reveal in Finder 오동작 수정 | `openPath` → `revealItemInDir` + capability `opener:allow-reveal-item-in-dir` 추가. 탭 메뉴·git 패널 3곳 동시 수정 | 현재 "Finder 에 표시"가 "기본 앱으로 열기"로 동작 중 (실측 확인) |
| 파일트리 헤더 | **프로젝트명 행 신설** + 우측 툴바 4버튼(hover 시 표시) | QA 첨부 스크린샷(Cursor)이 프로젝트명 행 형태를 명시 |
| Problems 열림 상태 | 세션 한정 view 상태(`AppShell` useState) — 영속 안 함 | 7.6 `TabKind::Preview` 미신설 결정과 동일 논리 |
| 언어·폰트 선택 UI | 상시 펼침(버튼 그리드/인라인 cmdk) → **Popover 콤보박스** 전환 | QA "열린 채 안 닫힘" 의 실체 — 애초에 닫힘 상태가 없는 구현이었다. `settings-ui.md` §1.2 의 Select 규정 준수 |
| 스크롤바 설정 필드 | 1차는 추가 안 함(auto-hide 하드코딩 상수) | 실수요 확인 전 설정 남발 방지 |
| `iconAgentRunning` 토큰 | 유지 + 상태별 토큰 4종 추가 | 제거는 사용자 커스텀 테마 파괴 |
| 리소스 폴링 | 프론트 TanStack Query `refetchInterval` 3000ms + `enabled`=설정 토글, `refetchIntervalInBackground: false` | OFF 시 측정 원천 중단. 1초 폴링은 측정기가 사용량을 만드는 역설 |
| 신규 의존성 | Rust: `tokio-tungstenite`(WS 서버), tokio feature `net` 추가, 토큰 생성용 CSPRNG(`rand` 또는 기존 uuid 계열 재사용 확인 후) | CC 연동 전체 구현 승인에 따른 필수 의존. 프론트 신규 패키지 없음 |

## 4. 이번에 알게 된 정정 사항

- `ENABLE_IDE_INTEGRATION` 환경변수는 Claude Code 2.1.221 바이너리에 존재하지 않는다(문자열 0회).
  자동 연결 트리거는 `CLAUDE_CODE_SSE_PORT` 단독. 기존 `docs/features/agent-integration.md`·
  `docs/research/claude-code-integration.md` 의 해당 서술은 구버전/커뮤니티 오류로 정정 대상.
- 기존 `DiffPane` 은 git 전용이라 CC diff 에 재사용 불가 — 신규 형제 위젯으로 구현.
