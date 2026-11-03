# native lane git 묶음 4/8 (g4-commit) — 커밋 박스·단축키·초안·gate·AI 메시지

> 작성일: 2026-11-03 · worktree `TAIDE-lanes/git`(branch `lane/git`) · 사양 `docs/research/2026-10-28-lane-git-spec.md` git-10~15·3.1 G4
> 결정: `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안: 결정 1 A git-15 not-applicable, 결정 2 A 초안 창별 분리), `docs/acknowledge/2026-10-26-native-autonomous-continuation.md`
> 선행: G1·G2·G3의 미커밋 변경 위에서 작업했습니다. 이 묶음 범위의 기존 변경은 없었습니다.

## 1. 대상과 판정

| ID | 판정 | 요약 |
| --- | --- | --- |
| git-10 | complete | 헤더 아래 커밋 박스: 3행 입력(placeholder `git.commitMessagePlaceholder`, 12px, `panel.inputBackground`, 테두리 `panel.inputBorder`·focus `app.focusBorder`, 오른쪽 28px 비움), conflict면 오류 색 11px `git.commitBlockedByConflicts`, 전체 폭 32px primary 버튼(`git.commit`, 진행 중 `git.committing`). 활성 = trim 비어 있지 않음 && 진행 중 아님 && 충돌 없음. 성공 → 초안 비움(toast 없음), 실패 → 오류 toast |
| git-11 | complete | 입력 focus 중 Mod+Enter(mac Cmd, 그 외 Ctrl, egui `Modifiers::COMMAND`)를 TextEdit보다 먼저 소비해 줄바꿈 없음, IME 조합 중(`modal::Composition`)이면 무시, 실행은 버튼과 같은 조건·gate |
| git-12 | complete | 창별 `Drafts`(LRU 8, 빈 문자열이면 삭제, 다시 쓰면 최신). 입력마다 기록, 패널은 매 프레임 그 창·프로젝트 초안을 읽음, 보기 전환·프로젝트 전환 뒤 복원. 닫힌 보조 창의 초안은 버림 |
| git-13 | complete | `commit_gate`(충돌 → 차단, staged 없음 && unstaged 있음 → 확인, 아니면 커밋). 확인 대화상자(`git.stageAllTitle`·`git.stageAllDescription`, 취소·`git.commit`, 첫 focus 취소, Escape 취소) → 확인 시 `Commit { stage_all: true }` 하나 |
| git-14 | complete | 입력 오른쪽 위 Sparkles(진행 중 Loader 회전) 20px 버튼, tooltip 왼쪽 3문구, 활성 = 진행 중 또는 변경 행 있음. 클릭 → UUID·owner(창 label)·최근 20개 요약 → `git_diff_staged_text` → 주입 AI 포트(기본 `ai_commit_message`) transient task. 재클릭 → 취소(`ai_request_cancel`)·진행 해제. 최신 요청만 반영, text 없음 무동작, sanitize 빈 결과 `git.commitMessageEmptyResponse`, 성공 → 초안 교체 + `git.commitMessageGenerated`(설명 notice ` · `), 실패 → `git.generateCommitMessageFailed` + describe_error. 패널이 그 프로젝트를 더 보이지 않으면 요청을 버리고 cancel |
| git-15 | not-applicable | 결정 1 A. 원본 화면 사용처 없음(amend 고정 false, fetch·undo IPC 없음, branch delete 화면 없음). native UI를 만들지 않음. backend는 runtime·remote 경로에 그대로 있음 |

## 2. 원본 근거

사양 git-10~15 절을 따랐고, 이번에 다시 확인한 원본 구간만 적습니다.

- `src/features/git/commit-box.tsx`: textarea `rows=3 py-1.5 pr-7 pl-2 text-xs rounded-sm border bg-panel-input-background border-panel-input-border focus:border-app-focus-border`, AI `IconButton side='left' absolute top-1.5 right-1.5 size-5`(아이콘 `size-3.5`), 사유 `text-status-error text-[11px]`, `Button size='sm'`(h-8, px-3, rounded-md, text-sm medium, default = `bg-primary`(= `button.primaryBackground`) / `text-primary-foreground`, hover `/90`, disabled 50%), 컨테이너 `px-2 py-1.5 gap-1.5 border-b`. `handleKeyDown`: IME 조합이면 반환, `IS_MAC ? metaKey : ctrlKey` + Enter면 `preventDefault` 후 `canCommit`일 때만 실행.
- `src/widgets/git-panel/git-panel-container.tsx:56-81,150-157,159-245`: 초안 읽기·쓰기, `handleCommit`(gate 재평가, `stageAll: gate === 'confirmStageAll'`, 성공 시 `applyCommitMessage('')`), AI 생성·취소·stale 가드·notice, `abandonCommitMessageRequest`(project 변경·unmount).
- `src/widgets/git-panel/git-panel.tsx:335-348,539-548,714-725`: `requestCommit`(차단 → 무동작, 확인 → 대화상자), `canGenerateCommitMessage = staged+unstaged+merge 행 > 0`, 확인 대화상자.
- `src/widgets/git-panel/commit-gate.ts`, `ai-commit-message.ts`, `src/entities/git/commit-message-memory.ts`, `src/entities/git/git.query.ts:375-390`(diff → AI, provider/model null), `src/entities/ai/ai.ipc.ts`(owner = 창 label).
- 반경 토큰: `src/shared/styles/global.css` `--radius 0.375rem`, `--radius-sm = radius - 2px`(4px), `--radius-md = radius`(6px).

## 3. 구현

| 파일 | 내용 |
| --- | --- |
| `native/taide-native-app/src/git-commit.rs`(신규) | `Drafts`(LRU 8), `State { drafts(창별), windows(보조 창 explorer slot → label), committing(프로젝트), requests(slot별 최신 AI 요청), ai(주입 포트) }`, `sanitize`, `recent_commits`, `notices`, 기본 포트 `ai_actions::ai_commit_message` |
| `native/taide-native-app/src/git-scm-model.rs` | `CommitGate`, `commit_gate(rows)` |
| `native/taide-native-app/src/git-scm.rs` | `View.commit: Commit { message, is_committing, is_generating }`, `commit_box`(입력·Mod+Enter·AI 버튼·사유·커밋 버튼, 아래 테두리), `Panel.stage_all`·`request_stage_all`·`take_stage_all`, `stage_all_dialog`. 두 번째 사용이 생겨 공통화: `confirmation`(discard·stage-all 대화상자 공용, discard 동작 불변), `spinner`(sync·AI 공용), `icon_button`이 tooltip 방향을 받음(헤더 두 곳은 BOTTOM 그대로). `Appearance`에 입력·focus·primary 색 5개 |
| `native/taide-native-app/src/domains/git.rs` | `Command::{Commit, AiCommitMessage, AiCancel}`, `MutationOp::Commit { window }`, `Reply::AiCommitMessage`, `State.commit`. dispatch: 커밋은 `change`(transient), AI는 diff 읽기 + 포트를 한 transient task로, cancel은 즉시. intent `SetCommitMessage`·`RequestCommit`·`ResolveStageAll`·`GenerateCommitMessage`. 응답: 커밋 성공 → 캐시 stale + 그 창 초안 비움, 실패 → `ipc_error`; AI → `finish_commit_message`. tick: 보조 창 목록 갱신, 열린 프로젝트만 committing 유지, 보이지 않는 패널의 AI 요청 abandon + cancel. 재연결 시 committing·요청 비움. `Intent::Shown`이 `Key::Log`도 관찰 |
| `native/taide-native-app/src/git-cache.rs` | `Key::Log`(가변, worktree 무관, 60초) / `Value::Log`, `log(project)`. 읽기는 `git_log(0, 100)` |
| `native/taide-native-ui/src/intents/git.rs` | `Intent::{SetCommitMessage, RequestCommit, ResolveStageAll, GenerateCommitMessage}` |
| `native/taide-native-app/src/editor-ai-edit.rs` | `strip_code_fence`를 `pub(crate)`로(두 번째 사용, 사양 G4 지정. 동작 불변) |
| `native/taide-native-app/src/navigation-icons.rs` | `Icon::Sparkles`(기존 `resources/icons/sparkles.svg`) |
| 공용 파일(영역 블록 안 추가만) | `lib.rs` 모듈 한 줄(`git_commit`, 정렬 위치), `application.rs` `git_state` 리터럴에 `commit: Default::default()` 한 줄 |
| 테스트 | `git-commit-tests.rs`(6), `git-scm-model-tests.rs`에 gate 1, `application-git-commit-tests.rs`(3, `domains/git.rs`에 등록), G2 `application-git-changes-tests.rs`의 `View` 직접 생성에 `commit: Default::default()` 한 줄 |

설계 판단(원본에 가장 가까운 안):

- 초안 공유: 원본은 slot마다 입력 state가 따로이고 project 변경 때만 초안을 다시 읽습니다. native는 패널이 매 프레임 창 초안을 읽으므로 같은 창의 두 slot이 같은 프로젝트를 보이면 입력이 실시간으로 같습니다(사양 git-12 "같은 초안을 읽습니다"와 일치).
- 진행 상태: 원본 `useCommitGit`의 pending은 패널(hook)마다지만, G3의 stash 진행과 같이 프로젝트 단위로 둡니다. 확인 대화상자 확인이 진행 중 커밋과 겹치면 두 번째 커밋을 보내지 않습니다.
- 커밋 성공 시 초안 비움: 원본 `applyCommitMessage('')`는 그 사이 프로젝트를 바꿨으면 화면의 다른 프로젝트 입력까지 비웁니다(결함). native는 커밋한 창·프로젝트의 초안만 비웁니다.
- AI owner: 원본 창 label 대응으로 메인 창 `native-main`, 보조 창은 그 창 label을 씁니다. 커밋 초안 창 구분과 같은 값입니다.
- AI 응답 반영 조건: 최신 요청이면서 그 slot 패널이 여전히 그 프로젝트를 보일 때만. 프로젝트 전환은 다음 Shown에서 패널이 바뀌어 응답이 즉시 버려지고, tick이 요청을 지우며 cancel을 보냅니다(원본 layout effect의 동기 abandon 대응).

## 4. 사양·대응표와 실제 코드 차이

- 사양 2.0.1은 커밋 박스 그리기를 `git-scm.rs`, 초안·AI를 `git-commit.rs`에 둡니다. 그대로 따랐습니다(`Appearance` 비공개 필드 공개 없이).
- 사양 2.0.2의 `AiCommitMessage { request_id, owner, recent }` 대신 `AiCommitMessageRequest`(provider·model None, diff 빈 값)와 포트를 넘기고 task 안에서 diff를 채웁니다. 응답 형태는 사양과 같습니다.
- G2 위젯 id(`mounted_id`)는 패널 mount 번호를 씁니다. 패널은 첫 Git 프레임을 그린 뒤 Shown 처리로 mount되므로 첫 프레임과 다음 프레임의 id가 다릅니다. egui는 클릭을 이전 프레임 위젯으로 판정하므로 Git 보기가 나타난 바로 그 프레임의 클릭은 놓칩니다(사람 입력으로는 한 프레임 안이라 관찰이 어렵고, G2·G3 위젯도 같음). 이번 묶음에서는 바꾸지 않고 6장에 남깁니다.

## 5. 검증

명령은 모두 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/git`을 붙여 lane worktree에서 실행했습니다. 테스트는 구현 전에 작성했고, 구현 전에는 실행하지 않았습니다.

| 명령 | 결과 |
| --- | --- |
| `cargo check --quiet --all-targets --manifest-path native/taide-native-app/Cargo.toml` | 통과(기존 경고만: vendor wry-preview, `is_applying`, `zen.rs:543`, `fixture_with_emmet_peek`) |
| `cargo test --quiet ... taide-native-app --lib -- git_commit:: git_scm_model:: domains::git::commit_tests --test-threads=2` | 1차 23개 중 21 통과·2 실패: 커밋 박스 `phase=typed`(검사 helper가 0.3초 안에 입력을 두 번 클릭해 egui 더블 클릭으로 공백 단어가 선택·교체됨), 초안 `phase=alpha typed`(초안 빈 값). 커밋 박스 helper를 "focus 유지 중이면 다시 클릭하지 않음"으로 고친 뒤 두 검사 재실행: 커밋 박스 통과, 초안 다시 실패(초안 빈 값) → 원인: 패널 첫 프레임 클릭(4장 마지막 항목) → helper가 입력을 찾은 뒤 settle하고 클릭하도록 정정 → 초안 검사 단독 통과. 구현 변경 없음 |
| `cargo fmt --manifest-path native/taide-native-app/Cargo.toml` 후 `-- --check`, `cargo fmt --manifest-path native/taide-native-ui/Cargo.toml -- --check` | 통과(`cargo fmt`는 이 묶음 파일 4개만 바꿈: `application-git-commit-tests.rs`·`domains/git.rs`·`git-commit-tests.rs`·`git-scm.rs`, `git status` 목록 변화 없음) |
| `cargo check --quiet --all-targets --features native-host,inspection --manifest-path native/taide-native-ui/Cargo.toml` | 통과(기존 경고 `tests/workbench.rs:1816`만) |
| `cargo check --quiet --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib` | 통과(remote-web 기존 dead code 경고만, 소스 변경 없음) |
| `cargo test --quiet ... taide-native-app --lib -- git title_branch explorer shell_keymap command_dispatch editor_ai_edit --test-threads=2` | fmt 뒤 89 통과(G1 base·G2 changes·G3 header·브랜치·캐시·모델·remote-git·title_branch·explorer·editor AI edit와 이번 묶음 10개 포함) |

자동 검사 내용:

- `[C]` `git-commit-tests.rs`: LRU 8·빈 문자열 삭제·재기록 최신화·삭제가 다른 초안을 밀어내지 않음, 창별 초안과 닫힌 보조 창 초안 버림, slot별 요청·abandon·finish·take, sanitize 표 14건(펜스·따옴표 5종·한 쌍만·짝 없음·빈 결과), 최근 20개 요약(7자, 짧은 id), notice 순서·구분자. `git-scm-model-tests.rs`: gate 8사례(충돌이 staged·unstaged와 섞여도 차단, staged가 하나라도 있으면 커밋).
- `[A]` `application-git-commit-tests.rs`(main.rs가 만드는 `NativeApplication`과 같은 frame 경로, ⌃⇧G로 Git 보기, 임시 저장소만):
  1. 커밋 박스: 빈 메시지·공백만 비활성, 입력 → 활성·placeholder 사라짐, IME Preedit 중 Mod+Enter 무시(대화상자 없음·초안 불변), Mod+Enter → stage-all 대화상자·줄바꿈 없음, 취소 → HEAD 불변, 버튼 → 대화상자 → 테스트 쪽 mutation 잠금을 쥔 채 확인 → `git.committing`·비활성 → 해제 → HEAD 요약 = 메시지·작업 트리 깨끗·초안 비움·placeholder, 변경 없음 상태 Mod+Enter → 오류 toast·초안 유지·HEAD 불변, `read-tree -m`으로 만든 충돌 → `git.commitBlockedByConflicts`·버튼 비활성·Mod+Enter 무시·`UU` 유지.
  2. 초안: 한 slot·두 프로젝트에서 alpha 입력 → beta 전환(빈 입력) → beta 입력 → alpha 복귀 시 alpha 초안, ⌘⇧E Files 보기 → ⌃⇧G 복귀 뒤 유지, beta 초안 유지.
  3. AI(가짜 포트, 실제 요청 저장소에 등록해 cancel 도달을 기록): 요청 owner `native-main`·provider/model 없음·최근 커밋 요약·diff에 변경 포함·lock 파일 내용 제외, 펜스+따옴표 응답 → 초안 `feat: generated`·성공 toast 설명 = `git.commitMessageUsedUnstaged` · `git.commitMessageFilesSkipped {count 1}`(staged 없음 대체 + 추적 안 된 `Cargo.lock` 제외), 진행 중 재클릭 → 저장소 cancel 도달·버튼 복귀, 늦은 응답 무시(초안·toast 불변), 실패 → `git.generateCommitMessageFailed` + describe_error 설명, 빈 응답 → `git.commitMessageEmptyResponse`, text 없음 → 무동작, 진행 중 프로젝트 전환 → cancel 도달·늦은 응답이 어느 초안에도 들어가지 않음, 변경 없는 beta에서 `git.noChangesForCommitMessage` 비활성.

실행하지 않은 것: 필터 없는 앱 전체 테스트(메인이 병합 때), 실제 AI provider·Keychain(보류 목록), 보조 창 commit 박스의 App 검사(창별 분리는 `[C]`로만 확인).

## 6. 남은 범위

- 이번 묶음 ID는 모두 판정을 마쳤습니다(git-15는 not-applicable).
- 관찰 사항(이번 범위 밖, 수정하지 않음): Git 보기가 처음 나타난 프레임의 위젯 id가 다음 프레임과 달라 그 프레임의 클릭을 놓칩니다(4장). 고치려면 `show_git_view`가 그리기 전에 패널을 mount하거나 첫 프레임 id를 예측해야 해 G2 패널 수명 설계를 바꿉니다. lane 끝 리뷰에서 판단합니다.
- 보조 창의 커밋 박스·AI owner는 보조 창 label을 쓰도록 구현했지만 App 검사는 메인 창만 했습니다.
