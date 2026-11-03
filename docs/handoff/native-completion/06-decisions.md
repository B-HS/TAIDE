# 06. 사용자 결정 모음 — Rust-native 전환

> 작성일: 2026-11-03 · 기준: `to_rust_native` HEAD `0c52b438`, Git 영역은 worktree `/Users/hyunseokbyun/development/TAIDE-lanes/git`
> 정본은 각 행의 "근거 문서"입니다. 이 문서는 찾기 쉽게 주제별로 모은 색인이며, 원문과 다르면 원문을 따릅니다.

## 0. 읽는 법

- "채택안"은 사용자가 답한 내용 또는 사용자가 "추천안대로"라고 답해 확정된 추천안입니다. 메인이 혼자 정한 계획은 여기에 넣지 않았습니다(`docs/feedback/2026-10-10-native-progress-and-batch-boundaries.md`).
- 일괄 승인 문장: 2026-10-06 "나머지는 추천대로", 2026-10-09 "버그마저 똑같이 구현할 필요는 없어. 나머지는 추천대로", 2026-10-26 "결정사항은 큰 변경사항 아니면 다 권장사항대로", 2026-10-27 "재개, 전부 추천안대로". 이 문장들 때문에 lane 사양 7개의 4.1절 결정은 예외를 빼고 모두 추천안(보통 A안)입니다(`docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`).
- 새 결정이 생기면 `docs/acknowledge/<날짜>-<주제>.md`에 먼저 기록하고 이 표에 한 줄을 추가합니다.
- 경로 약어: `ACK` = `docs/acknowledge/`, `SPEC-<lane>` = `docs/research/2026-10-28-lane-<lane>-spec.md`, `LANE-DEC` = `ACK/2026-10-28-native-lane-spec-decisions.md`.

## 1. 전환 목표와 완료 기준

| 결정 | 채택안 | 근거 문서 | 날짜 |
| --- | --- | --- | --- |
| 전환 목표 | TS·React·Tauri·Monaco·xterm 의존을 장기적으로 없애고 현재 기능 전체를 Rust-native 앱으로 제공 | `ACK/2026-09-23-rust-native-transition-contract.md` 1절 | 2026-09-23 |
| 기능 범위 | 사용자 기능을 축소하지 않음. 기능을 빼서 동등성을 맞추지 않음 | 같은 문서 2절 | 2026-09-23 |
| 전환 방식 | big-bang 재작성 금지, 기존 앱과 병행하는 단계적 전환 | 같은 문서 2절 | 2026-09-23 |
| 기존 Rust 코어 | 파일·프로젝트·레이아웃·Git·PTY·LSP·검색·원격·AI·동기화 로직과 테스트를 우선 재사용 | 같은 문서 2절 | 2026-09-23 |
| 제거 시점 | TS는 모든 기능·성능·보안·패키징 게이트와 beta 뒤 마지막에. Monaco·xterm은 native 편집기·터미널 게이트 통과 후 한 릴리스 fallback 뒤 | 같은 문서 2절, `docs/roadmap-rust-native.md` 10절 | 2026-09-23 |
| LSP 소유권 | process·transport·generation·document mirror·취소·crash replay·표시 상태를 Rust `LspCoordinator` 한 곳이 소유 | 같은 문서 2절 | 2026-09-23 |
| 플랫폼 | macOS Apple Silicon이 첫 완료 대상. 기존 Windows·Linux 조건부 코드는 별도 결정 없이 삭제하지 않음 | 같은 문서 2절 | 2026-09-23 |
| 불변식 | 기존 데이터 읽기·되돌리기 가능, dirty buffer 무손실, 보안 경계 비완화, 편집 transaction·revision, PTY 출력 단일 파싱 등 | 같은 문서 3절 | 2026-09-23 |
| 완료 정의 | parity plan 필수 항목, 성능 비악화, 기존 데이터로 실행·복구, remote·IDE·CLI·장기 session 동등, rollback beta, TS 등 참조 0건 + 서명·공증 | 같은 문서 4절 | 2026-09-23 |
| 구현 우선순위 | 기존 TS view를 최대한 같게 재현하는 구현을 먼저, 보완 검토·실기는 최후 순위. 새 기능·디자인 변경은 목적이 아님 | `ACK/2026-09-30-m8-code-first-parity.md` | 2026-09-30 |
| 전체 재개 | "기존의 배치n까지 하고 멈추다는 지시는 무시한다". 전체 전환 완료까지 진행 | `ACK/2026-10-09-native-full-resume.md` | 2026-10-09 |
| 원본 결함 처리 | 큰 기능을 정확히, 버그 없이, 꼼수 없이 구현. 원본의 오류·결함·내부 수치를 똑같이 옮기지 않음 | `ACK/2026-10-09-native-full-resume.md`, `ACK/2026-10-09-native-find-regex-decisions.md` | 2026-10-09 |
| 완료 판정 기준 | 원본 TS 화면·상태·상호작용 대응 + 실제 앱 경로 + 해당 동작 자동 검사. 기준을 낮추지 않음 | `docs/memory/2026-10-28-native-lane-operations.md` 4절, `ACK/2026-10-26-native-autonomous-continuation.md` | 2026-10-28 |

## 2. 진행 방식과 운영

| 결정 | 채택안 | 근거 문서 | 날짜 |
| --- | --- | --- | --- |
| 위임 수단 | 서브에이전트(Agent 도구) 금지, Workflow만 | `ACK/2026-10-06-native-transition-decisions.md` 진행 방식 표, `docs/feedback/2026-08-11-use-workflow-not-agent-tool.md` | 2026-10-06 |
| workflow 모델 | fable 미사용. 구현·설계·수정 opus·xhigh, 리뷰 sonnet·xhigh | `ACK/2026-10-06-native-transition-decisions.md`, `ACK/2026-10-26-native-autonomous-continuation.md` | 2026-10-06, 10-26 |
| 자율 진행 | "태스크가 없어질 때까지 끝까지 진행, 허락이 필요한 건 보류, 결정은 큰 변경이 아니면 권장사항대로". 작은 결정은 원본에 가장 가까운 안으로 정하고 QA에 근거 | `ACK/2026-10-26-native-autonomous-continuation.md` | 2026-10-26 |
| 운영 방식 변경 | 영역 lane 병렬(worktree·브랜치·target 분리). 사양 1회, lane 끝 리뷰 1회. 완료 기준 유지 | 같은 문서 "운영 방식 변경", `docs/memory/2026-10-28-native-lane-operations.md` | 2026-10-28 |
| lane 브랜치·worktree | 메인이 `lane/<영역>`과 `TAIDE-lanes/` worktree를 만들고 병합 뒤 지움 | `ACK/2026-10-26-native-autonomous-continuation.md` | 2026-10-28 |
| 진행 보고 | 기능표 기준 진척률과 남은 범위를 함께 보고 | `docs/feedback/2026-10-28-native-serial-step-inefficiency.md` | 2026-10-28 |
| 중단 해석 | 소요 시간 질문은 정보 요청이며 중단 지시가 아님. 명시적 중단 단어가 있을 때만 정지 | `ACK/2026-10-06-native-transition-decisions.md` 진행 방식 표(10-07 아침), `docs/HANDOFF.md` 5절 3번 | 2026-10-07 |
| 단계 E 정지·재개 | "implement만 끝내고 멈춰" → 구현 뒤 정지, "재개, 전부 추천안대로" → 같은 run을 리뷰부터 재개 | `docs/PROCESS.md` 단계 E | 2026-10-27 |
| 빌드 캐시 | 재개 전 빌드 캐시 정리. `cargo clean --profile dev` 사용(`rm -rf` 대신), `Bash(cargo clean *)` 허용 규칙은 사용자가 추가 | `ACK/2026-10-06-native-transition-decisions.md` 진행 방식 표·기각 표, `docs/feedback/2026-10-07-build-cache-disk-usage.md` | 2026-10-07 |

## 3. 기술과 의존성

| 결정 | 채택안 | 근거 문서 | 날짜 |
| --- | --- | --- | --- |
| UI 프레임워크 | egui 0.36.2를 격리 native 구현에 임시 사용. 최종 dependency 채택·root MSRV 변경 승인은 아님 | `ACK/2026-09-30-m8-code-first-parity.md` | 2026-09-30 |
| 구문 강조 엔진 D1 | `ferriki-textmate =0.12.0` + `ferroni =1.8.1`(적합성 게이트 통과 조건부, 이후 불일치 0으로 통과). 처음의 "syntect 계열" 결정을 대체 | `ACK/2026-10-06-native-transition-decisions.md` "구문 강조 엔진 결정", `docs/HANDOFF.md` 4절 | 2026-10-06 |
| 새 의존성 반입 D2 | 승인. 버전 고정. lockfile 변경은 `native/taide-native-app`과 `native/taide-native-syntax`에 한정 | 같은 문서 | 2026-10-06 |
| 문법 자산 D3 | `@shikijs/langs 4.4.3`의 30종과 임베드 모듈 전부, 회색 지대 4종(elixir·toml·yaml·erb) 포함. `THIRD_PARTY_LICENSES.md` 갱신 | 같은 문서 | 2026-10-06 |
| 정규식 엔진 위치 | 정규식 평가는 `taide-native-syntax`(앱 전용)에서만. editor·UI에 정규식 의존 추가 금지(remote-web 그래프 보호) | 같은 문서 기각 표, `ACK/2026-10-09-native-find-regex-decisions.md` 2번 | 2026-10-06 |
| 보조 창 | eframe 다중 viewport로 구현(추천안 미명시 항목을 원칙에 따라 적용, 정정 가능) | `ACK/2026-10-06-native-transition-decisions.md` 결정 표 | 2026-10-06 |
| native 메뉴바 | muda 계열 의존 추가(현재 `muda =0.21.0`, `native/taide-native-app/Cargo.toml` 92줄). 추천안 미명시 항목을 적용 | 같은 문서 | 2026-10-06 |
| 표면 인자 | `EditorAppearance`·`EditorPresentation`에 필드를 넣지 않고 새 진입 함수(`show_presented`, `show_tokenized`, `show_request`) 추가 | 같은 문서 기각 표 | 2026-10-06 |
| 찾기 정규식 1 | JS 방언 유지, `regress =0.12.0`을 원본 일치·성능 게이트로 평가 → 게이트 통과로 앱 전용 syntax 경계에 채택 | `ACK/2026-10-09-native-find-regex-decisions.md` | 2026-10-09 |
| 찾기 정규식 2 | 새 의존성 1개를 앱 전용 경계에 정확한 버전으로 반입. editor·UI·remote-web 그래프 불변 | 같은 문서 | 2026-10-09 |
| 찾기 정규식 3 | 줄·일치 사이에서 경과 시간 확인해 중단, 한 줄 안 백트래킹 폭주는 TS와 같은 조건으로 수용 | 같은 문서 | 2026-10-09 |
| 찾기 정규식 4 | 대량 모두 바꾸기에서도 검색 옵션 일관 유지(원본의 19,999개 이상 경로 결함 재현 안 함) | 같은 문서 | 2026-10-09 |
| 찾기 정규식 5 | 검색 패널과 찾기 위젯의 방언을 통일하지 않음 | 같은 문서 | 2026-10-09 |
| 찾기 정규식 6 | 정규식 엔진의 영어 오류 문장을 표시 | 같은 문서 | 2026-10-09 |
| 찾기 성능 기준 | 30만 줄·20MiB 문서에서 일치 없는 전체 검색과 19,999개 수집 각각 100ms 이내면 채택, 실제 찾기·치환은 1초 상한 | 같은 문서 "성능 기준과 채택" | 2026-10-09 |
| transpose 단독 surrogate | 잘못된 UTF-16 단위를 U+FFFD로 변환, 유효한 쌍은 보존 | `ACK/2026-10-09-native-transpose-utf16.md` | 2026-10-09 |
| 모니터 목록(shell-3) | 이미 lock에 있는 `winit =0.30.13`을 앱 일반 의존으로 올리고 `eframe::create_native`로 이벤트 루프 생성 | `ACK/2026-10-26-native-autonomous-continuation.md` 단계 E 결정 | 2026-10-27 |
| 로거(shell-3) | 의존성 없는 최소 `log::Log`(stderr, `Warn` 이상)를 `main.rs`에 설치. `env_logger` 추가 안 함. 파일 로그는 shell-15와 함께 | `SPEC-shell` 4.1 결정 3 A, `LANE-DEC` | 2026-10-28 |
| vendored eframe 수정 | 허용(shell-3 종료 코드, shell-5 창 핸들·raw input hook). remote-web은 Wasm check로 확인 | `SPEC-shell` 4.1 결정 6 A | 2026-10-28 |
| 내장 언어 서비스 CSS/SCSS·HTML | 순수 Rust 이식(JSON 방식, oracle 대조, 새 의존 없음) | `SPEC-editor-preview` 4.1 결정 4, `LANE-DEC` 보류 표 | 2026-10-28 |
| 내장 언어 서비스 TS/JS | 추천은 임베디드 JS 엔진(새 의존, 허락 필요) → 보류. 9절 미결 1번 | 같은 근거 | 2026-10-28 |

## 4. 보호·보안·Git

| 결정 | 채택안 | 근거 문서 | 날짜 |
| --- | --- | --- | --- |
| remote-web 동결 | `native/taide-remote-web`은 데스크톱 native 대응 완료 전까지 동결, 컴파일만 유지 | `ACK/2026-10-06-native-transition-decisions.md` | 2026-10-06 |
| 합성 입력 | 합성 키·마우스 입력으로 화면 조작 안 함(세션 데이터를 미리 만들고 창만 캡처) | 같은 문서 기각 표 | 2026-10-06 |
| 보호 대상 | 실제 앱 데이터·OS 설정·클립보드·Keychain·Trash, 실기용 M8 번들 보호 | `ACK/2026-10-09-native-full-resume.md`, `docs/HANDOFF.md` 5절 8번 | 2026-10-09 |
| 단일 인스턴스 소켓 | `std::env::temp_dir()` 아래(`taide-native-<hash>.sock`)에 만들고 종료 때 지움 | `LANE-DEC`, `SPEC-shell` 4.2절 3번 | 2026-10-28 |
| git 검사의 사용자 설정 | git CLI가 사용자 전역 설정을 읽기 전용으로 참조하는 수준 허용. fixture local config가 hook·서명·자격 증명을 덮음 | `LANE-DEC`, `SPEC-git` 4.2절 7번 | 2026-10-28 |
| Git 정책 | `to_rust_native`에서 관리, force push·AI 트레일러 금지 | `ACK/2026-09-23-rust-native-transition-contract.md` 2절 | 2026-09-23 |
| 원격 push 승인 | GitHub `B-HS/TAIDE`의 `to_rust_native`로 일반 push 승인(force·다른 브랜치·릴리스 제외) | `ACK/2026-09-27-m6-completion-push-approval.md` | 2026-09-27 |
| 체크포인트 커밋 | 미커밋 M8 작업을 체크포인트 커밋하고 이후 검증된 논리 단위로 커밋. "M8 완료 전 commit 보류"를 대체 | `ACK/2026-10-06-native-transition-decisions.md` | 2026-10-06 |
| 자동 커밋·푸시 | `llm-rules.auto-commit`·`auto-push` true. 대화 안 명시 지시가 우선 | `ACK/2026-08-06-git-workflow.md`, `docs/agent-operations.md` 4절 | 2026-08-06 |
| 커밋 형식 | Conventional Commits, type 영어 + 한국어 설명, `git add -A` 금지, force push 금지 | `ACK/2026-08-06-git-workflow.md` | 2026-08-06 |
| 라이선스 | 앱 MIT. 서드파티 고지 정본은 `THIRD_PARTY_LICENSES.md` | `ACK/2026-09-05-license-mit-decision.md` | 2026-09-05 |
| 번들 identifier | `net.gumyo.taide`. Keychain은 이관하지 않음(시크릿 비접촉). native는 격리 서비스명 `net.gumyo.taide.native-isolated` 사용 | `ACK/2026-08-29-bundle-identifier-rename.md`, `docs/architecture.md` 9.3절 | 2026-08-29 |
| 언어 pack 경로 | A안: 경로 검증 추가와 별도 보안 회귀 | `ACK/2026-09-28-locale-path-validation-choice.md` | 2026-09-28 |
| 직접 앱 종료 대기 | A안: 직접 `RunEvent::Exit`도 시간 제한 없이 등록 자원 완료까지 대기(Tauri 쪽 결정) | `ACK/2026-09-28-direct-exit-drain-decision.md` | 2026-09-28 |
| 원격 WS 큐 | 연결별 256프레임 상한, 포화 시 연결 종료. 바이트 상한은 두지 않고 위험 기록 | `ACK/2026-09-29-m7-remote-ws-queue-scope.md` | 2026-09-29 |
| IDE lockfile 예외 | 그 검사 한 번만 임시 lockfile 토큰을 메모리로 읽음. 다른 키 파일 허용 아님 | `ACK/2026-09-29-m7-isolated-ide-lockfile-exception.md` | 2026-09-29 |
| M7 기준선 | 부팅 "화면 표시 준비 시각" 대리지표 채택, GUI 실기는 대표 흐름 1회, 성능 기준선은 지표별 단일 표본 | `ACK/2026-09-29-m7-boot-visible-ready-proxy.md`, `ACK/2026-09-29-m7-one-pass-validation-scope.md` | 2026-09-29 |

## 5. 중복 범위 소유(누가 만드나)

`LANE-DEC` "채택"의 정리입니다. 나머지 lane은 진입점을 재사용하고 기능표 행 판정만 합니다.

| 범위 | 소유 lane | 다른 lane이 할 일 |
| --- | --- | --- |
| OS 알림 생산자와 진입점 `notify_event`(순수 로직 `native/taide-native-app/src/native-notifications.rs`), 첫 전달 안내 | shell | settings(settings-22·23), git(git-7), explorer-search(검색 치환 알림, explorer-search-33)는 `notify_event`를 호출만. settings 사양의 `crates/taide-notification/src/producer.rs` 설계는 폐기 |
| provider 소비자: 외부 열기(shell-90 = terminal-agent-57), 알림(shell-91 = terminal-agent-59~63 = settings-23), IDE save·진단(shell-92 = terminal-agent-70·73) | shell | terminal-agent는 행 갱신만 |
| IDE diff(shell-92 diff = terminal-agent-68·69 = git-49), git 알림, diff 설정 소비 | git | settings는 행 비고 갱신 |
| Sync 테마 재로드(`SyncStateChanged`), 보조 창 수명 | shell | settings는 확인·행 갱신 |
| reveal, searchOnType, 검색 치환 알림, 검색 편집기 본문(editor-81, shell-89의 searchEditor) | explorer-search | editor·shell은 행 갱신 |
| 탐색기 Git 장식(explorer-search-7 = git-38) | 파생·행 렌더·주입점은 explorer-search, `git-cache` 공급 연결은 git(D1 A) | 병합 뒤 연결 |
| 탐색기 비교 메뉴(explorer-search-22)와 파일 히스토리 메뉴(explorer-search-23) | git(D2·D3 A) | explorer-search는 메뉴 자리만 |
| 탭 내용 분기 hook(`TabKind::SearchEditor`·`Diff`·`CommitDiff`·`ClaudeDiff`) | wave 0 구조 분리(끝났으면 shell 묶음 1 첫 작업) | editor-preview 결정 5 A |
| 실행 중 터미널 scrollback(terminal-agent-11·49 = settings-30) | terminal-agent(`set_history`와 전파, App 검사) | settings는 소비·행 갱신(settings 결정 7 A를 대체) |
| CLI 백엔드(terminal-agent-56 ↔ settings-16) | terminal-agent가 앱 모듈 `crate::agent_cli`와 host 명령, `cli.*` 명령 3종 | settings S1은 행 UI만, `agent_cli` 재사용(`taide-runtime/src/agent_host.rs` 변경 안 함) |
| terminal-agent-55(훅 설치·제거·상태 설정 UI)·64(설정의 테스트 알림·시스템 설정 열기) | settings | terminal-agent는 행 갱신 |
| 탐색기 상태의 창별 분리(shell-5 잔여) | explorer-search D6 A(mount별 분리) | shell 묶음 3은 검증·행 갱신 |

## 6. lane 사양 4.1 결정(채택안)

모두 2026-10-28 `LANE-DEC`로 확정됐습니다. 예외는 표에 적었습니다.

### 6.1 shell(`SPEC-shell` 4.1)

| # | 결정 | 채택안 |
| --- | --- | --- |
| 1 | 중복 기능 소유 | A: provider 소비자와 알림 진입점은 shell lane 묶음 1(wave 1). 5절 표 |
| 2 | 보조 창 toast가 뜰 창(shell-98) | A: 사용자 동작 결과 toast는 만드는 순간 OS focus를 가진 창, 주 창 전용 provider toast는 주 창(`ToastLayers`, `native/taide-native-app/src/toast-layers.rs`) |
| 3 | 로거(shell-3) | A: 최소 `log::Log`(stderr, `Warn` 이상) |
| 4 | 보조 창 복원 규칙 | 결정 불필요(이미 `taide-window` 앱 의존으로 해소) |
| 5 | drop overlay와 Zen 안내·편집기 popup 그리기 순서(shell-22) | A: 의도된 차이로 유지, 배치 31 실기에서 다시 봄 |
| 6 | vendored eframe 수정 범위 | A: 허용 |

### 6.2 editor·preview(`SPEC-editor-preview` 4.1)

| # | 결정 | 채택안 |
| --- | --- | --- |
| 1 | 리거처(editor-4)에 epaint API가 없을 때 | A: 조사로 API 없음을 확인하면 "egui 한계" 근거로 not-applicable. epaint vendor 패치 안 함 |
| 2 | 원본 기본 기능이지만 기능표 행이 없는 것(Unicode 강조, 텍스트 링크 감지, 선택 텍스트 발생 강조) | A: 새 행(editor-88~)으로 추가하고 E1에서 구현. 실제로는 2026-11-03 기준 기능표에 editor 행이 87개뿐이고(`grep -cE "^\| editor-[0-9]+ "`), editor lane QA(`docs/quality-assurance/2026-11-02-native-lane-editor-display-commands.md` 59줄)가 남은 범위로 적음. 9절 미결 11번 |
| 3 | 서식 있는 복사의 clipboard 형식 | A: arboard에 HTML 쓰기가 있으면 HTML+텍스트, 없으면 텍스트만 + 비고 |
| 4 | 내장 언어 서비스 방식(editor-57) | CSS/SCSS·HTML = 순수 Rust 이식(채택). TS/JS = 임베디드 JS 엔진 추천이나 새 의존이라 보류(9절 1번) |
| 5 | 탭 내용 분기 hook 소유 | A: 단계 0에서 만들기 |
| 6 | `gotoLine`(⌃G)·`quickOutline`(⇧⌘O) 화면 | A: 기존 팔레트의 `:`·`@` 모드로 열기 |
| 7 | editor-72 닫을 범위 | A: br, 혼합·중첩 code, 문단 안 이미지, kbd/samp, raw input, 첫 블록 여백, details/summary, 앵커, AccessKit까지 구현. 임의 CSS·SVG·MathML·미디어·counter는 비대상으로 근거 기록 |
| 8 | 이미지 해석 실패 화면(preview-6) | A: 파일 이름만 가운데 표시 |
| 9 | 미디어 theme 변경 시 재생 유지(preview-14/15) | A: host `evaluate_script`로 CSS 변수만 갱신, 페이지 JS는 계속 끔 |
| 10 | preview-38 비macOS | A: 원본 출시 대상이 macOS뿐이므로 not-applicable |

### 6.3 explorer-search(`SPEC-explorer-search` 4.1)

| # | 결정 | 채택안 |
| --- | --- | --- |
| D1 | 탐색기 Git 데코레이션 분담 | A: 이 lane이 파생·행 렌더·주입점, git lane이 `git-cache.rs` 공급 연결과 git-38·explorer-search-7 판정 |
| D2 | 파일 히스토리 메뉴(explorer-search-23) | A: git lane이 git-53~55와 함께 |
| D3 | 비교 메뉴(explorer-search-22) | A: git lane이 메뉴·대상 상태·Diff 탭·본문까지 |
| D4 | 검색 편집기 탭 본문(editor-81, shell-89의 searchEditor) | A: explorer-search lane이 구현 |
| D5 | 검색 보기에서 벗어날 때 상태 | A: 원본대로 패널을 내리고 입력·결과 초기화, 실행 취소 |
| D6 | 탐색기 상태 단위 | A: mount별로 나눔(`crate::explorer_mounts::Mounts`) |
| D7 | 선택 행 hover 바탕 | A: 원본 CSS 결과대로 hover가 선택보다 우선. 아웃라인(explorer-search-39) 차이는 후속 기록 |
| D8 | commands-ui-11·12 행 | A: 같은 묶음에서 행 갱신 |
| D9 | 검색 입력 IME | A: 조합 중에는 입력 검색 타이머를 시작하지 않음 |
| D10 | 탐색기 컨텍스트 메뉴 아이콘 | A: 기존 native 메뉴처럼 글자만, 차이를 QA에 기록 |
| 11 | 앱 공통 OS 알림 진입점 | shell lane 경로 하나(`notify_event`) |

### 6.4 git(`SPEC-git` 4.1)

| # | 결정 | 채택안 |
| --- | --- | --- |
| 1 | git-15(원본 화면 없음: amend·fetch·branch delete·undo) | A: not-applicable |
| 2 | 커밋 초안 기억 범위(git-12) | A: 창별 분리 |
| 3 | createTagOnHead 원본 결함 | A: 재현하지 않고 focus slot에 대화상자 하나, 그래프 접힘과 무관하게 표시 |
| 4 | Monaco 기본 diff 장식 재현 범위(git-40·50) | A: 900px 자동 inline, 분할선 끌기, filler 빗금, ± 표시, 문자 단위 강조, overview ruler(`DecorationKind::Overview` 재사용), ClaudeDiff 수정 쪽 revert 화살표 포함 |
| 5 | Alt+\ 적용 범위(git-41) | A: focus pane의 diff 탭만 |
| 6 | showMoves(git-42) | A: Monaco `computeMovedLines` 규칙을 libgit2 hunk 위에 이식 |
| 7 | diff 줄 계산 기준(git-40·43) | A: libgit2 hunk(`diff_hunks`) |
| 4.2-7 | git CLI의 사용자 전역 설정 읽기 | 허용(4절 표) |

### 6.5 terminal-agent(`SPEC-terminal-agent` 4.1)

| # | 결정 | 채택안 |
| --- | --- | --- |
| 1 | 중복 행 소유(57, 59~63, 70, 73 / 55, 64 / 68) | A: shell이 57·59~63·70·73, settings가 55·64, git이 68. 이 lane은 행만 갱신 |
| 2 | CLI 백엔드 위치(56 ↔ settings-16) | A: 앱 모듈 `crate::agent_cli`(공유 crate 무변경), host 명령, `cli.*` 명령 3종 |
| 3 | 실행 중 scrollback 소유(11 ↔ settings-30) | A: 이 lane이 `set_history`와 전파, 49의 App 검사까지 |
| 4 | 원본에 없는 기능 행(6, 26, 45) | A: 6은 셀 단위 그리기 검사 뒤 complete, 26·45는 not-applicable |
| 5 | 터미널 메뉴 아이콘(32) | A: 앱 전반 lucide 행(commands-ui-98, wave 3)에서 처리, 32는 complete |
| 6 | OSC 133 탭 전환 뒤 블록 수명(27) | A: 단일 Core 유지(전환 계약의 단일 파싱), 블록·색이 남는 차이를 비고에 적고 complete. 숨은 세션의 이후 완료는 현재 테마 색 |
| 7 | CLI 관리자 대화상자 취소 toast(56) | A: toast를 띄우지 않음(원본의 취소 시 성공 표시는 결함으로 봄) |

### 6.6 settings(`SPEC-settings` 4.1)

| # | 결정 | 채택안 |
| --- | --- | --- |
| 1 | 원격 미러(동결)의 새 section 표시 | A: native 앱에만 표시, 미러는 8 section 유지 |
| 2 | DEV 안내 문구(82)와 알림 발신자 | A: 문구 원문 유지, native dev 빌드는 osascript라 발신자가 "Script Editor"로 보이는 차이를 QA에 기록 |
| 3 | status 라벨 잔여 지점(78, 2026-10-06부터 미결) | A: TS 대응이 있는 지점은 원본 표면(toast·배너·인라인), 대응이 없는 native 내부 오류는 `log::warn!`만 남기고 상태바 라벨 제거 |
| 4 | CLI 설치 대상(16, 17) | A: `/usr/local/bin/taide`, 소유 판정 동일. cutover 전 native 바이너리는 `.app` 밖이라 원본 오류 문구로 실패하는 것이 정상 |
| 5 | 마지막 동기화 시각 형식(46) | A: `sys-locale`이 ko·ja·en이면 각 브라우저 Intl 기본 형식 패턴을 chrono `Local`로, 그 외는 en-US 패턴 |
| 6 | Performance의 TS 전용 블록(54) | A: native 지표 블록만, 나머지 not-applicable |
| 7 | 실행 중 scrollback(30) | 추천 A(settings lane이 추가)는 terminal-agent 결정 3 A로 대체: terminal-agent lane이 구현, settings는 소비 |
| 8 | 겹치는 범위 소유(5, 22, 23, 48, 80, 28) | A: 5절 표 |

### 6.7 commands-ui(`SPEC-commands-ui` 4.1)

| # | 결정 | 채택안 |
| --- | --- | --- |
| 1 | `window.reload`의 native 의미 | A: 실행한 창의 UI만 다시 적재(일시 UI 닫기, 장애 경계 초기화, 테마·로케일·설정·데이터 재조회, 문서·터미널·LSP·레이아웃 유지) |
| 2 | 원본에서 활성으로 보이지만 실행해도 무동작인 명령 행 | A: native는 비활성 행 + 비고(무동작 재현 안 함) |
| 3 | `app.showPerfSnapshot` | A: not-applicable |
| 4 | Monaco quick input 대체와 hook 행 91 | A: 기존 팔레트 `:`·`@` 모드, 91은 not-applicable |
| 5 | 기능표에 없는 편집기 기본 기능(유니코드 강조, URL 링크, 독립 색 선택기, 구문 강조 복사, Paste As) | A: 편집기 영역 새 행 등록, 43은 의존을 비고에 남기고 partial, 구현 배치는 따로 정함(editor-preview 결정 2 A와 시점이 다름, 9절 11번) |
| 6 | 공용 위젯 행(67~79, 84) 완료 기준 | A: 원본 소비자가 쓰는 variant·state만, 기존 native 소비 화면 전부 이전 |
| 7 | 전역 글자 크기 | A: Body만 원본 본문 13px, 나머지는 프리미티브가 명시 |
| 8 | `cli.installShellCommand` 관리자 프롬프트 취소 | **예외**: 추천 A(원본대로 성공 문구) 대신 terminal-agent 결정 7 A 채택. 취소하면 성공 문구를 보이지 않음. Settings CLI 행(settings-16)도 같은 규칙 |
| 9 | 앞 wave에서 진입만 남은 행 | A: wave 3 시작 전에 메인이 병합 상태 기준으로 목록을 확정해 이 사양에 보충 절로 추가 |

## 7. 그 밖의 기능 결정

| 결정 | 채택안 | 근거 문서 | 날짜 |
| --- | --- | --- | --- |
| shell-12 장애 격리 | A: 'snapshot 읽기 + 영역 소유 상태 + intent 출력'으로 바꾼 영역에만 `catch_unwind` 경계. panic 시 그 영역 상태와 그 pass의 intent를 버리고 fallback. Welcome·Activity Bar·상태 표시줄부터, 편집기·사이드바 패널은 공유 상태 변경을 render 밖 트랜잭션으로 옮긴 뒤 | `ACK/2026-10-26-native-autonomous-continuation.md` 단계 E 결정 | 2026-10-27 |
| Edit 메뉴 단축키 표시 | A: 메뉴에 단축키를 달지 않고 기존 egui 키 입력 경로 유지, 메뉴 클릭만 포커스 입력에 전달 | 같은 문서 | 2026-10-27 |
| 세션 중 테마 전환 | 새 테마로 다시 토큰화(TS의 색 섞임은 테마 이름 고정에서 온 결함으로 보고 재현 안 함, 사용자 보고 뒤 반대 지시 없음) | `ACK/2026-10-06-native-transition-decisions.md` 기각 표, `docs/HANDOFF.md` 4절 | 2026-10-06 |
| 터미널 오류 표시 | 화면 도색 없애고 경고 로그(TS는 write·resize 오류를 무시) | `docs/HANDOFF.md` 4절 | 2026-10-06 |
| 작은 결정 일반 | 원본 TS 동작에 가장 가까운 안, QA 문서 "원본과 다르게 둔 점(근거)"에 기록 | `ACK/2026-10-26-native-autonomous-continuation.md` | 2026-10-26 |

## 8. 대체되었거나 폐기된 결정

| 옛 결정 | 대체한 결정 | 날짜 |
| --- | --- | --- |
| "M8 완료 commit·push는 전체 완료 뒤에만"(`ACK/2026-09-30-m8-code-first-parity.md`) | 체크포인트 커밋 후 논리 단위 커밋(`ACK/2026-10-06-native-transition-decisions.md`) | 2026-10-06 |
| 구문 강조 "syntect 계열" | `ferriki-textmate` + `ferroni` | 2026-10-06 저녁 |
| "일단 배치4까지만", "이번 배치까지만"(배치 7 뒤 정지) | 전체 재개(`ACK/2026-10-09-native-full-resume.md`) | 2026-10-09 |
| 구현 단계 병렬·git worktree 격리 기각(`ACK/2026-10-06-native-transition-decisions.md` 기각 표) | 영역 lane 병렬(wave 0 구조 분리 뒤 가능해짐) | 2026-10-28 |
| 배치 순서 직렬 단계(3~7 ID) 실행 | 영역 lane 병렬, 배치 번호와 ID 배정은 유지(`docs/quality-assurance/2026-10-10-native-remaining-batch-plan.md` "2026-10-28 실행 방식 변경") | 2026-10-28 |
| 2026-10-09 "서브에이전트·workflow 없이 메인이 직렬" | 2026-10-26 workflow 사용 | 2026-10-26 |
| commands-ui 결정 8 A(취소해도 성공 문구) | terminal-agent 결정 7 A(문구 없음) | 2026-10-28 |
| settings 결정 7 A(settings lane이 scrollback 구현) | terminal-agent 결정 3 A | 2026-10-28 |
| settings 사양의 알림 생산자 `crates/taide-notification/src/producer.rs`, `domains::settings::notify` | shell lane `native-notifications.rs` + `notify_event` | 2026-10-28 |
| status 라벨 56곳 처리(2026-10-06 미결) | settings 결정 3 A | 2026-10-28 |
| Rust 크레이트 분리 안 함(`ACK/2026-08-28-no-crate-split-decision.md`, 재론 조건 "도메인 코드의 두 번째 소비자") | native 앱이 두 번째 소비자가 되어 `ACK/2026-09-23-rust-native-crate-migration-contract.md`로 `crates/taide-*` 이전이 진행됨 | 2026-09-23 |

## 9. 미결 결정(답을 받지 않은 것)

추천안이 있으면 먼저 적었습니다. 진행을 막지 않는 것은 현행을 유지합니다. 임의로 확정하지 않습니다.

| # | 결정 지점 | 추천안 / 선택지 | 현재 상태 | 근거 |
| --- | --- | --- | --- | --- |
| 1 | editor-57 TS/JS 내장 언어 서비스 | 추천: 임베디드 JS 엔진(예: QuickJS 바인딩)으로 원본 TS 서비스를 실행하고 Monaco 어댑터만 Rust 이식(새 의존·크기·시작 시간 spike 1회 필요). 허락하지 않으면 B′: 순수 Rust 파서 범위(syntax 진단·심볼·포맷·onType)로 줄이고 editor-57 비고에 checker 기능(완성·hover·정의·참조·rename·code action·inlay) 부재를 적고 판정 | 보류. CSS/SCSS·HTML은 순수 Rust로 진행 | `SPEC-editor-preview` 4.1 결정 4, `LANE-DEC` 보류 표 |
| 2 | 번들 테마의 7자리 색 | 추천안 없음. 올바른 값을 사용자가 정해야 함. `crates/taide-theme/resources/themes/intellij-islands-light.json` 1119줄 `"foreground": "#0083080"`(TS·native 모두 이 토큰 색 적용 실패) | 미해결(파일에 그대로 있음) | `docs/HANDOFF.md` 6절 2번 |
| 3 | `glsl` 문법 포함 유지 | 현행 포함(상류 라이선스 표기 없음, 기존 웹 번들에도 포함). 명시 추천 없음 | 포함 상태 유지 | `docs/HANDOFF.md` 6절 3번, `docs/quality-assurance/2026-10-06-native-batch5-integration.md` 46줄 |
| 4 | 30만 줄 초과 문서의 저장 정리(모든 줄 후행 공백 제거) | 추천: 유지(Monaco와 같음) | 현행 유지 | `docs/HANDOFF.md` 6절 4번 |
| 5 | 전역 ⌘N(새 untitled 파일) | 추천: 제거(TS에는 전역 ⌘N 없음) | 코드에 남아 있음: `native/taide-native-app/src/application.rs` 2130-2140, `consume_key(egui::Modifiers::COMMAND, egui::Key::N)` | `docs/HANDOFF.md` 6절 5번, `ACK/2026-10-06-native-transition-decisions.md` 미결 |
| 6 | 실행 경로 없는 명령을 키바인딩 편집기에 보이는 방식 | 추천: 비활성 표시(TS는 무필터) | 미해결. 팔레트 행은 commands-ui 결정 2 A로 별도 처리됨 | `docs/HANDOFF.md` 6절 6번 |
| 7 | 팔레트가 열린 동안의 배경 | 추천: 실기 확인 뒤 결정(native는 셸 전체를 흐리게, TS는 scrim만) | 미해결 | `docs/HANDOFF.md` 6절 7번 |
| 8 | 컷오버 정책: HTML·미디어 미리보기의 WebView(wry) 의존 유지, macOS 외 지원 범위, 기본 데이터 경로·Keychain 서비스명 승계 시점 | 추천안 미제시. 배치 32 착수 전에 질문 | 일부만 결정됨: preview-38 not-applicable(editor-preview 결정 10 A), Markdown은 native 렌더 유지(결정 7 A), 미디어는 WebView 유지 전제(결정 9 A). 데이터 경로는 shell-15 보류 | `ACK/2026-10-06-native-transition-decisions.md` 미결, `docs/HANDOFF.md` 6절 9번 |
| 9 | egui/eframe 0.36.2 최종 채택과 root MSRV | 배치 31 게이트(다중 창, CJK IME, 접근성, native 메뉴, 외부 drop, GPU fallback 실기) 뒤 사용자 승인 | 임시 후보 | `ACK/2026-09-30-m8-code-first-parity.md`, `docs/HANDOFF.md` 7절 |
| 10 | 테스트 생성 workflow의 fable 사용 | 추천: 2026-10-06·10-26 지시대로 opus·sonnet만 | 사용자 auto-memory `workflow-model-assignment`가 "테스트 = fable" 규칙을 쓰기 전에 확인하라고 적음 | `ACK/2026-10-06-native-transition-decisions.md` 진행 방식 표 |
| 11 | 기능표에 없는 편집기 기본 기능(Unicode 강조, 텍스트 링크 감지·열기, 선택 텍스트 발생 강조, 독립 색 선택기, 구문 강조 복사, Paste As)의 행 추가·구현 시점 | 두 채택안이 시점에서 다름: editor-preview 2 A는 editor lane E1에서 구현, commands-ui 5 A는 행만 등록하고 구현 배치는 따로. editor lane은 이미 병합됐고 행은 아직 없음. 메인 판단(사용자 결정 아님): 행을 먼저 추가하고 구현 배치를 사용자에게 확인 | 행 미추가(editor 87행) | `SPEC-editor-preview` 4.1 결정 2, `SPEC-commands-ui` 4.1 결정 5 |
| 12 | 보조 창 파일 선택 대화상자의 부모 창(shell-5) | 저장소 밖 `tauri-plugin-dialog` 소스 확인 허락 또는 사용자 확인 필요. 확인 전에는 현행(메인 창 부모) | 보류 | `SPEC-shell` 4.2절 4번 |

구현으로 해소된 옛 미결(결정 불필요): Monaco 기본 편집기 키의 계층(`docs/HANDOFF.md` 6절 10번)은 배치 7에서 `native/taide-native-ui/src/editor-keymap-defaults.json`과 키맵 경로로 연결됐습니다(`docs/quality-assurance/2026-10-06-native-batch7-cursor-commands.md` 18줄). 찾기 정규식(6절 1번)은 2026-10-09에 결정됐습니다.

## 10. 허락 대기(보류) 항목

자동 검사와 작업자는 하지 않습니다. 사용자 허락 뒤 별도로 진행합니다.

| 항목 | 이유 | 관련 ID | 근거 |
| --- | --- | --- | --- |
| shell-15 실제 사용자 데이터 경로와 Keychain 승계 | 사용자 데이터·컷오버 정책 | shell-15, 실제 `taide` CLI → native 연결(shell-90) | `ACK/2026-10-26-native-autonomous-continuation.md`, `LANE-DEC` |
| 배치 32 기본 앱 전환·beta, 배치 33 TS·Tauri 제거 | 파괴적·출시 작업 | - | 같은 문서, `docs/memory/2026-10-28-native-lane-operations.md` 3절 |
| 새 외부 의존성 | 네트워크·lockfile | editor-57 TS/JS JS 엔진 등 | `LANE-DEC` |
| 실제 OS 알림 표시, macOS 알림 권한, 시스템 설정 앱 열기 | OS 상태 | shell-91, settings-21~23, git-7, explorer-search-33 | `LANE-DEC`, 각 사양 4.2 |
| `/usr/local/bin/taide` 설치·해제 | 시스템 경로 쓰기·관리자 암호, 설치된 Tauri 앱 symlink 변경 가능 | settings-16·17, commands-ui-32~34, terminal-agent-56 | 같음 |
| Keychain 쓰기(AI 토큰, GitHub PAT, 원격 비밀번호) | 사용자 키체인 | settings-35·36·45·51, git-14 | 같음 |
| 사용자 홈 hooks 파일, 실제 프로젝트 `.claude/settings.local.json` 쓰기 | 사용자 데이터 | settings-17, terminal-agent-55 | 같음 |
| 네트워크(LSP 서버 다운로드, gist, 모델 목록, 실제 원격 push/pull) | 외부 접속 | settings-33·35·37·45~48, git-7 | 같음 |
| 실제 휴지통 검사(`tests/explorer-delete.rs` 등 3개, 추적 안 된 파일 discard) | 사용자 휴지통 | explorer-search-16, git-20·22·23, preview-5 일부 | `LANE-DEC`, `SPEC-explorer-search` 4.2 P1, `SPEC-git` 4.2 |
| 실제 클립보드, Finder reveal, 실제 파일 선택 창, 외부 앱 열기 | OS 상태 | git-23, settings-34·41·42·53, preview-6, commands-ui-43 | 각 사양 4.2 |
| 실제 저장소에서 `git.revertHead`, 실제 `git init` | 사용자 저장소 변경 | commands-ui-36, git-2 | `SPEC-commands-ui` 4.2, `SPEC-git` 4.2 |
| 원격 접속 서버 실기 기동 | loopback이지만 실기 | settings-49~53 | `SPEC-settings` 4.2 |
| 환경 변경(`rustup target add`로 비macOS 컴파일 확인) | 개발 환경 변경 | preview-38 | `SPEC-editor-preview` 4.2 |
| 저장소 밖 crate 소스 확인 | 작업자 규칙 | shell-5 | `SPEC-shell` 4.2 |
| 실기(배치 31): 픽셀, CJK IME, VoiceOver, 실제 PTY, 다중 창 focus, NSMenu, 실제 Finder drag, WKWebView 재생, 리거처 글꼴, Option-as-Meta | GUI·OS 입력 | shell-3·5·9·22·94·98, preview-14·15, editor-4, terminal-agent 여러 행 | 각 사양 4.2, `ACK/2026-10-26-native-autonomous-continuation.md` |
