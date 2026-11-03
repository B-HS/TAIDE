# TAIDE Rust-native 전환 인수인계 문서 (입구)

> 최종 갱신일: 2026-11-04 · 대응 커밋: `34fb88d7` (브랜치 `to_rust_native`)
> 이 폴더는 남은 전환 작업을 처음 보는 AI 모델이 끝까지 구현할 수 있도록 만든 인수인계 묶음입니다. 이 파일을 먼저 읽고, 아래 "읽는 순서"를 따르십시오.

## 1. 이 문서들보다 우선하는 최신 사실

다른 문서는 2026-11-03~04에 영역별로 병렬 작성됐습니다. 작성 중에 아래 일이 끝났으므로, 다른 문서와 이 절이 다르면 **이 절이 맞습니다**.

1. **git lane 병합 완료**: `lane/git`은 충돌 해결 뒤 커밋 `f3a5b5d0`으로 `to_rust_native`에 병합·푸시됐습니다.
    - 문서 곳곳의 "`/Users/hyunseokbyun/development/TAIDE-lanes/git`을 읽으라", "병합 중", "병합 뒤 연결 필요" 표현은 낡았습니다. 모든 영역의 기준은 저장소 루트 `/Users/hyunseokbyun/development/TAIDE`입니다.
    - lane worktree(`/Users/hyunseokbyun/development/TAIDE-lanes/*`)는 정리 대상이며 없을 수 있습니다.
    - 병합에서 끝난 연결: git-38 → explorer-search-7(탐색기 Git 장식이 `git-cache` status를 받음), git-7(pull/push 결과를 shell `notify_event`로 알림), git-49(IDE diff 요청 수신을 `provider_events::Inbox`의 `Event::IdeDiff`로), 제목줄 branch가 `git-cache` status를 읽음(`title_branch` 모듈 삭제). 근거: `docs/quality-assurance/2026-11-03-native-git-lane-merge.md`.
2. **병합 게이트 휴지통 보호 수정**: `docs/utils/2026-11-03-native-merge-gate.sh`는 커밋 `34fb88d7`부터 실제 macOS 휴지통을 쓰는 검사 3개를 `--skip`합니다. 스크립트를 그대로 쓰면 됩니다. 이 3개 검사는 사용자 허락 없이 절대 실행하지 않습니다.
    - `remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다`
    - `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다` (`native/taide-native-app/tests/explorer-delete.rs`)
    - `실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다` (`native/taide-native-app/tests/lsp-workspace-worker.rs`)
3. **현재 진척** (`docs/quality-assurance/2026-10-09-native-function-matrix.md` 행을 직접 센 값, 상단 합계 표는 낡았습니다):

| 상태 | 행 |
| --- | ---: |
| complete | 500 |
| partial | 62 |
| unwired | 12 |
| missing | 5 |
| not-applicable | 19 |
| frozen | 1 |
| 합계 | 599 |

기능 분모는 599 − 19 − 1 = 579행이고 완료율은 500/579 = 86.4%입니다. 미완료 79행입니다. 세는 방법은 `01-product-and-goal.md` 3.4절의 awk 명령이며, `git-61` 행은 셀 안의 `\|` 때문에 열이 밀리므로 상태를 따로 확인합니다.

영역별 미완료: shell 9, editor 9, preview 3, explorer-search 4, terminal-agent 5, git 4, settings 10, commands-ui 35.

## 2. 읽는 순서

| 순서 | 문서 | 언제 |
| --- | --- | --- |
| 1 | `01-product-and-goal.md` | 제품·목표·완료 정의·범위 지도 |
| 2 | `04-rules-and-conventions.md` | 코드·보호·Git 규칙. **구현 전에 반드시** |
| 3 | `02-stack-and-environment.md` | 버전, 빌드·테스트 명령, 알려진 실패 |
| 4 | `03-architecture.md` | 코드 구조와 새 기능을 끝까지 연결하는 실제 예시 |
| 5 | `05-process-and-gates.md` | 작업 절차, 병합, 게이트, 기능표·QA 갱신 |
| 6 | `06-decisions.md` | 이미 정해진 사용자 결정. 다시 묻지 않습니다 |
| 7 | `10`~`16` 중 맡은 영역 | 행별 원본 근거·구현 단계·검사·완료 기준 |
| 8 | `20-gates-31-33.md` | 기능 다음의 구조·실기·출시·제거 게이트 |
| 9 | `30-pending-permissions.md` | 사용자 허락 없이는 하지 않는 일 |

| 영역 문서 | 범위 |
| --- | --- |
| `10-remaining-shell.md` | 창·보조 창·셸 표시·장애 경계·실제 데이터 경로 |
| `11-remaining-editor-preview.md` | 편집기 표시·명령·내장 언어 서비스, 미리보기 |
| `12-remaining-explorer-search.md` | 탐색기·검색 |
| `13-remaining-terminal-agent.md` | 터미널·에이전트·CLI |
| `14-remaining-git.md` | Git |
| `15-remaining-settings.md` | 설정 |
| `16-remaining-commands-ui.md` | 명령·키·팔레트·공용 위젯 (다른 영역 다음에) |

## 3. 바로 시작하는 순서

1. `git -C /Users/hyunseokbyun/development/TAIDE status`가 깨끗한지, `git log --oneline -3`이 `34fb88d7` 이후인지 확인합니다.
2. `docs/PROCESS.md` 맨 위 절(처음 80줄)로 마지막 진행 상태를 확인합니다. 이 파일은 매우 크므로 전체를 읽지 않습니다.
3. 기능표에서 미완료 행을 다시 셉니다(1절 3번과 다르면 그 사이 진행이 있었던 것입니다).
4. 영역을 고르는 기준은 `05-process-and-gates.md`의 lane 절입니다. 권장 순서는 explorer-search·git·terminal-agent·preview처럼 남은 행이 적은 영역을 먼저 닫고, settings·shell·editor를 거쳐, 다른 영역 기능에 기대는 commands-ui를 마지막에 합니다.
5. 영역 문서의 행별 절차대로 구현하고, 묶음이 끝날 때마다 병합 게이트를 돌린 뒤 커밋·푸시합니다.
6. 사용자 허락 항목(`30-pending-permissions.md`)을 만나면 구현하지 말고 대체 검사까지만 한 뒤 보류 목록에 남깁니다.

## 4. 이 묶음이 다루지 않는 것

- 기존 Tauri + React 앱의 기능 추가. 전환은 원본 동작을 재현할 뿐 새 기능·새 디자인을 만들지 않습니다.
- 이전 세션 대화 기록. 필요한 결정과 사실은 모두 `06-decisions.md`와 저장소 `docs/`에 있습니다.
