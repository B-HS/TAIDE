# 05. 작업 방식과 게이트 — lane 병렬, 묶음 절차, 병합, 기능표

> 작성일: 2026-11-03 · 기준: `to_rust_native` HEAD `0c52b438`, Git 영역은 병합 중인 worktree `/Users/hyunseokbyun/development/TAIDE-lanes/git`(`lane/git` HEAD `50892b59`)
> 정본: 운영 방식 `docs/memory/2026-10-28-native-lane-operations.md`, 영역 경계 `docs/research/2026-10-28-native-lane-seams.md`, lane 사양 `docs/research/2026-10-28-lane-*-spec.md`, lane workflow `docs/utils/2026-11-02-native-lane-workflow.js`, 병합 게이트 `docs/utils/2026-11-03-native-merge-gate.sh`, 배치 계획 `docs/quality-assurance/2026-10-10-native-remaining-batch-plan.md`
> 코드 규칙과 보호 규칙은 04 문서, 사용자 결정은 06 문서에 있습니다.

## 0. 현재 상태(2026-11-03 실제 확인)

| 항목 | 상태 | 확인 방법 |
| --- | --- | --- |
| 메인 브랜치 | `to_rust_native` `0c52b438`, 작업 트리 clean | `git -C /Users/hyunseokbyun/development/TAIDE status --short`, `git log --oneline -1` |
| 병합 끝난 lane | preview `e87b053c`, shell `4fa4beff`, terminal-agent `446964b7`, editor `fb43d2ad`, explorer-search `6dba64f9`, settings `3864e1db` | `git -C /Users/hyunseokbyun/development/TAIDE log --merges --format="%h %s"` |
| 진행 중 | git lane: worktree 안에서 `to_rust_native`(`0c52b438`)를 `lane/git`으로 병합하는 중. 코드 충돌은 작업 트리에서 표식이 사라졌고, 기능표에 충돌 표식 3묶음이 남음 | `cat /Users/hyunseokbyun/development/TAIDE/.git/worktrees/git/MERGE_HEAD`, `git -C /Users/hyunseokbyun/development/TAIDE-lanes/git diff --name-only --diff-filter=U` |
| 남은 wave | W2 git 병합 마무리, W3 commands-ui(배치 29, 앞 wave 진입 잔여, shell-89), W4 메인(배치 31 구조·전체 게이트), 32·33은 사용자 허락 뒤 | `sed -n 1,40p /Users/hyunseokbyun/development/TAIDE/docs/PROCESS.md` |
| worktree | 7개가 아직 남아 있음(병합 끝난 6개 포함) | `git -C /Users/hyunseokbyun/development/TAIDE worktree list` |
| 기능표 | 599행: complete 434, partial 59, unwired 47, missing 40, not-applicable 18, frozen 1. 기능 분모 580 중 434 완료(74.8%). Git lane 병합 전 수치 | 10절 명령 |
| 디스크 | 여유 573Gi(69% 사용) | `df -h /Users/hyunseokbyun/development` |

## 1. 전체 그림

### 1.1 왜 lane인가

2026-10-28 이전에는 배치를 3~7 ID 단계로 나눠 구현 → 리뷰 → 수정을 직렬로 돌렸고, 단계 하나에 1.5~7시간이 걸렸습니다. 모든 기능이 `application.rs`·`host.rs`·`shell.rs`를 함께 고쳐 병렬이 막혔기 때문입니다. 사용자가 비효율을 지적해(`docs/feedback/2026-10-28-native-serial-step-inefficiency.md`) 통합 지점을 영역 파일로 나누고(wave 0, `d5559c46`·`87bf1f51`), 영역별 lane을 병렬로 돌리는 방식으로 바꿨습니다. 완료 기준은 낮추지 않았습니다.

### 1.2 wave

| wave | lane | 배치 | 상태 |
| --- | --- | --- | --- |
| 0 | 없음(메인 직렬) | lane 경계 구조 이동, dev 프로필 debuginfo 축소, fmt 정리, target 정리 | 완료 |
| 1 | shell · explorer-search · terminal-agent · editor · preview | 24 잔여, 25, 27, 23+30 | 병합 완료 |
| 2 | git · settings | 26, 28 | settings 병합 완료, git 병합 중 |
| 3 | commands-ui | 29, 앞 wave에서 진입만 남은 행, shell-89 | 대기 |
| 4 | 메인 | 31(구조·전체 게이트), 32·33은 사용자 허락 뒤 | 대기 |

2026-11-02에는 빌드 한 벌이 25GB여서 W1과 W2를 합쳐 7개 lane을 동시에 돌렸습니다(`docs/PROCESS.md` W1 항목). 운영 문서의 동시 수 규칙은 `min(4, (디스크 여유 − 100GB) / 한 벌 크기)`이고 4는 CPU 18코어·메모리 128GB 기준 상한입니다. 7개를 돌릴 때는 작업자가 앱 lib 실제 App 검사를 `--test-threads=2`로 필터 실행해 부하를 줄였습니다.

### 1.3 lane 하나의 구성

| 항목 | 값 |
| --- | --- |
| worktree | `/Users/hyunseokbyun/development/TAIDE-lanes/<lane>` |
| 브랜치 | `lane/<lane>`(로컬 전용, push하지 않음) |
| cargo target | `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`(lane마다 분리. 같은 target을 공유하면 cargo 잠금으로 빌드가 직렬화됨) |
| 사양 | `docs/research/2026-10-28-lane-<spec>-spec.md`. lane 키와 사양 이름: shell→`shell`, explorer-search→`explorer-search`, terminal-agent→`terminal-agent`, editor→`editor-preview`, preview→`editor-preview`, git→`git`, settings→`settings`, commands-ui→`commands-ui` |
| 순서 | 사양(1회) → 묶음 구현(묶음마다 직렬) → lane 리뷰(끝에 1회) → 수정(차단 항목이 있을 때 1회) → 메인이 lane 커밋 → 메인이 병합·게이트 |
| 모델 | 사양·구현·수정 opus·xhigh, 리뷰 sonnet·xhigh(사용자 지시, `docs/acknowledge/2026-10-26-native-autonomous-continuation.md`). 사용자 전역 컨벤션의 Claude Code 기본 배정(fable·sonnet·haiku)보다 이 프로젝트 지시가 우선합니다(04 문서 0절 사다리 2·3번) |
| 위임 수단 | Workflow만. Agent 도구(서브에이전트) 금지(`docs/feedback/2026-08-11-use-workflow-not-agent-tool.md`, `docs/HANDOFF.md` 5절 2번) |

## 2. lane 준비(메인)

1. 기준 커밋을 정합니다. 새 wave의 lane은 직전 병합·push가 끝난 `to_rust_native` HEAD에서 분기합니다. wave 1·2는 `87bf1f51`에서 분기했습니다.
2. worktree와 브랜치를 만듭니다.

```
git -C /Users/hyunseokbyun/development/TAIDE worktree add -b lane/<lane> /Users/hyunseokbyun/development/TAIDE-lanes/<lane> <기준 커밋>
```

3. 사양 문서가 있는지 확인합니다. commands-ui lane은 사양 4.1 결정 9 A에 따라 wave 3 시작 전에 메인이 병합 상태 기준으로 "앞 wave에서 진입만 남은 행" 목록을 `docs/research/2026-10-28-lane-commands-ui-spec.md`에 보충 절로 추가해야 합니다(06 문서 6.7절).
4. LSP 검사에 필요한 mock을 lane target에 빌드합니다(없으면 앱 lib 검사 수십 개가 `assertion failed: mock.is_file()`로 실패). 첫 빌드는 캐시가 비어 오래 걸립니다.

```
cargo build --quiet --manifest-path /Users/hyunseokbyun/development/TAIDE-lanes/<lane>/native/taide-native-app/Cargo.toml --example native-lsp-mock --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>
```

5. worktree에는 `node_modules`가 없습니다. 원본 Monaco·xterm 소스는 메인 저장소 `/Users/hyunseokbyun/development/TAIDE/node_modules`를 읽기 전용으로 읽게 합니다.
6. 디스크 여유를 확인합니다. 여유가 100GB 아래로 내려가면 lane을 늘리기 전에 정리를 제안합니다(16절).

## 3. lane workflow 실행

### 3.1 script와 인자

script: `/Users/hyunseokbyun/development/TAIDE/docs/utils/2026-11-02-native-lane-workflow.js`(phases Implement → Review → Fix). Workflow 도구에 이 script와 아래 형태의 args를 넘깁니다. 실제 git lane 인자(실행 기록 `~/.claude/projects/-Users-hyunseokbyun-development-TAIDE/<세션 ID>/workflows/wf_76b229f7-18b.json`)를 줄인 예입니다.

```json
{
  "base": "87bf1f51",
  "date": "2026-11-02",
  "lane": {
    "key": "git",
    "spec": "git",
    "root": "/Users/hyunseokbyun/development/TAIDE-lanes/git",
    "target": "/Users/hyunseokbyun/development/TAIDE-lanes/targets/git",
    "note": "긴 git 작업은 transient task 로 돌려 host worker 를 막지 않습니다. ... git-7 알림도 shell lane 진입점 병합 뒤 연결합니다. ...",
    "bundles": [
      { "key": "g1-base", "ids": ["git-64", "git-1", "git-2", "git-37", "git-38", "git-16", "git-18", "git-19"] }
    ]
  }
}
```

- `bundles[i].key`는 QA 문서 이름(`<date>-native-lane-<lane.key>-<bundle.key>.md`)에 들어갑니다. `bundles[i].brief`(선택)는 묶음 지시에 덧붙는 문장입니다.
- `note`에는 다른 lane과의 경계(누가 무엇을 만들고 무엇은 병합 뒤 연결하는지), 보류 항목을 적습니다.
- 묶음 하나는 6~10 ID이고 묶음 순서는 사양 3절 "작업 묶음 계획"을 따릅니다.

### 3.2 script가 작업자에게 주는 규칙(요지)

`BACKGROUND`·`RULES`·`DONE` 상수(script 26-65줄)가 모든 구현·수정 지시에 들어갑니다. 지시를 손으로 쓸 때도 같은 내용을 넣습니다.

1. 작업 위치는 lane worktree만. 메인 작업 트리는 수정 금지. target은 lane 전용.
2. 기준은 원본 TS. 사양 문서와 필요한 파일 구간만 읽고 원본 전체를 다시 조사하지 않음. 사양이 실제 코드와 다르면 실제 코드를 따르고 QA에 차이를 적음. `docs/PROCESS.md`·`docs/HANDOFF.md`는 읽지 않음(컨텍스트 절약).
3. 파일 수정은 Edit·Write로만. 셸은 절대 경로 한 줄 단순 명령만(grep, rg, ls, wc, sed -n, cat, diff, `git -C <worktree> status/diff/log/show`, cargo). `cd`·파이프·`&&`·`;`·루프·변수 대입·heredoc·python·`sed -i`·`rm`·npm 금지. 이유: 승인 프롬프트 폭주(`docs/feedback/2026-09-05-permission-prompt-storm.md`).
4. git 상태 변경 금지, 다른 에이전트 띄우기 금지.
5. cargo는 `--manifest-path <worktree 절대경로> ... --locked --offline --target-dir <lane target>`, 한 번에 하나, `--quiet`. `Cargo.toml`·`Cargo.lock` 변경 금지(필요하면 unresolved에 적음).
6. 앱 lib 실제 App 검사는 `-- --test-threads=2`로 필터 실행. 시간 초과로 실패한 검사는 단독으로 1회 재실행해 흔들림과 결함을 구분. 필터 없는 앱 전체 테스트는 메인이 병합 때 실행.
7. 보호 규칙(04 문서 2절), 동결 remote-web(04 문서 3절), 코드 규칙(04 문서 1절).
8. 같은 가정의 수정이 세 번 연속 실패하면 멈추고 unresolved에 원인 가설을 적음. 실행하지 않은 검사를 통과했다고 쓰지 않음.
9. 허락이 필요한 일은 하지 않고 `needsUserDecision`에 적음. 작은 결정은 원본에 가장 가까운 안으로 정하고 QA에 근거를 적음.
10. 큰 파일은 Grep으로 위치를 찾아 구간만 읽고, QA 초안을 절반 지점에 씀. 컨텍스트가 부족하면 완성된 부분을 컴파일·테스트 통과 상태로 마무리하고 남은 것을 unresolved에 정확히 적음.

### 3.3 작업자 결과 형식

구현·수정 작업자는 JSON(`IMPL_SCHEMA`)으로 돌려줍니다: `bundle`, `summary`, `idStatus[{id, status, remaining}]`, `changedFiles`, `verification[{command, exitCode, result}]`, `unresolved`, `needsUserDecision`, `qaDocPath`. 리뷰어는 `REVIEW_SCHEMA`: `verdict`(pass / needs-fix), `blocking[{file, line, ids, problem, failureScenario, fix}]`, `nonBlocking`, `statusOverclaims`. 수정 단계는 `verdict`가 needs-fix이거나 `statusOverclaims`가 비어 있지 않을 때만 돕니다.

## 4. workflow를 쓸 수 없을 때: 단일 에이전트 순서

Workflow 도구가 없거나 사용자가 workflow를 쓰지 않기로 하면, 메인 한 명이 같은 절차를 직렬로 합니다. 2026-10-09~10-16에도 메인이 같은 작업 트리에서 직렬로 진행한 선례가 있습니다(`docs/acknowledge/2026-10-09-native-full-resume.md`).

1. lane 하나를 고릅니다(남은 것: git 병합 마무리 → commands-ui → 배치 31). 동시에 여러 lane을 열지 않습니다.
2. 2절대로 worktree·브랜치·target을 만듭니다. 병합·게이트 절차를 lane 방식과 똑같이 유지하려는 목적입니다(권장). 메인 작업 트리에서 바로 작업해도 되지만 그때는 8절 병합 단계가 없어지고 게이트를 커밋 전에 실행합니다.
3. 사양 문서에서 대상 묶음 절과 3절 묶음 계획만 읽습니다.
4. 묶음마다 5절 표준 절차를 그대로 수행합니다. 묶음이 끝날 때마다 QA 문서와 기능표 행을 갱신합니다.
5. 모든 묶음이 끝나면 리뷰를 따로 한 번 합니다. 구현할 때의 기억이 아니라 `git -C <worktree> diff <기준 커밋> --stat`과 실제 diff를 처음부터 다시 읽으며 6절 리뷰 점검표 여섯 항목을 확인합니다. 발견한 결함은 재현 테스트 → 수정 → QA 문서 "수정" 절 추가 순서로 처리합니다.
6. 7절대로 lane 커밋, 8절대로 병합, 9절대로 게이트, 12절대로 커밋·push를 합니다.
7. 단일 에이전트는 부하가 적으므로 필터 검사에 `--test-threads=4`를 써도 됩니다. 시간 초과 실패는 여전히 단독 재실행으로 구분합니다.

## 5. 묶음 하나의 표준 절차

대상: 묶음의 ID 6~10개. 각 단계의 확인 방법까지 수행해야 다음 단계로 갑니다.

1. **미커밋 변경 확인**: `git -C <worktree> status --short`. 앞 묶음 작업자의 변경이면 유지합니다. 이 묶음 범위의 변경이 이미 있으면(사용량 한도 등으로 끊긴 경우) 읽고 이어서 작업합니다.
2. **사양 확인**: 사양의 대상 ID 절(원본 동작, native 현재 자산, 빠진 것, 구현 설계, 자동 검사, 완료 판정)과 사양 4.1 결정(06 문서)을 읽습니다. 원본 TS 근거 경로를 실제로 열어 확인할 구간만 읽습니다.
3. **실제 코드 위치 확인**: `rg -n "<심볼>" <worktree>/native`로 사용처를 확인합니다. 사양과 실제 코드가 다르면 실제 코드를 따르고 QA에 "사양과 실제 코드가 달라 실제 코드를 따른 곳"으로 적습니다.
4. **실패 테스트 먼저**: 원본 동작을 단언하는 검사를 씁니다. 위치는 crate 단위(`<파일>-tests.rs`), 실제 App(`application-<주제>-tests.rs`, 영역 파일 끝에 선언), UI egui 하네스(`native/taide-native-ui/tests/*.rs`, `--features native-host,inspection`). 구현 전에 실패하는지 실행해 확인했는지 여부를 QA에 그대로 적습니다(확인하지 않았으면 "따로 실행하지 않았음"이라고 씀).
5. **구현**: 영역 경계 규칙(04 문서 1.9절)대로 넣습니다. 공용 파일은 자기 영역 블록 끝에 추가만 합니다.
6. **실제 앱 경로 확인**: `main.rs`의 `eframe::create_native(.. NativeApplication::new(..))`부터 해당 기능까지 호출 체인이 이어지는지 확인합니다. 프레임 흐름은 `background_tick`(호스트 응답 `poll` → `apply_<영역>_reply`, provider 이벤트 소비) → 키맵·`command-dispatch` → `shell.show`(UI) → intent 루프의 `apply_<영역>_intent` → 오버레이 → `HostCommand` 제출입니다(`docs/architecture.md` 9.2절). 모듈만 있고 이 체인에서 닿지 않으면 `unwired`입니다. 실제 App 검사(`launch` + `step`)로 그 경로를 한 번 이상 지나가게 합니다.
7. **검사**: 변경 crate마다 아래를 실행합니다(lane이면 target은 lane target).

```
cargo check --quiet --all-targets --manifest-path <worktree>/native/taide-native-app/Cargo.toml --locked --offline --target-dir <target>
cargo check --quiet --all-targets --features native-host,inspection --manifest-path <worktree>/native/taide-native-ui/Cargo.toml --locked --offline --target-dir <target>
cargo test --manifest-path <worktree>/native/taide-native-app/Cargo.toml --locked --offline --target-dir <target> --lib -- --test-threads=2 <필터1> <필터2>
cargo test --manifest-path <worktree>/native/taide-native-app/Cargo.toml --locked --offline --target-dir <target> --test <통합 테스트 파일 이름> -- --test-threads=2
cargo test --manifest-path <worktree>/native/taide-native-ui/Cargo.toml --features native-host,inspection --locked --offline --target-dir <target> --test <UI 테스트>
cargo fmt --manifest-path <worktree>/native/<crate>/Cargo.toml -- --check
```

- libtest는 필터를 여러 개 받으면 하나라도 맞는 검사를 실행합니다. 영역별 필터 목록은 seams 문서 3절 표에 있습니다.
- UI crate를 바꿨으면 동결 remote-web Wasm check(04 문서 3절)를 실행합니다.
- 같은 입력·코드·환경에서 이미 성공한 검사는 다시 돌리지 않고 결과를 재사용합니다. 실패 원인에 영향을 준 코드를 고친 뒤에는 관련 검사만 1회 다시 돌립니다.
- 사용자에게 보이던 동작을 없애거나 표시 경로를 바꾸면 그 동작을 단언하는 기존 검사를 먼저 `rg`로 찾아 함께 실행합니다(`docs/feedback/2026-10-07-batch-integration-full-test-gate.md`).
8. **QA 문서**: 11절 형식으로 `docs/quality-assurance/<date>-native-lane-<lane>-<묶음키>.md`를 씁니다.
9. **기능표 행 갱신**: 이 묶음 ID 행만 고칩니다(10절). 상단 합계 표는 고치지 않습니다.

## 6. lane 리뷰와 수정

리뷰는 lane 전체 diff(`git -C <worktree> diff <기준 커밋> --stat`, `git -C <worktree> status`의 새 파일)를 대상으로 한 번 합니다. 리뷰어는 파일 수정·git 상태 변경·cargo 실행을 하지 않습니다. 점검표(script `phase("Review")`):

1. 원본 TS 화면·상태·상호작용과 일치하는가. 빠진 분기(오류, 빈 상태, 비활성, 취소, 수명)와 원본에 없는 새 동작이 있는가
2. 실제 앱 경로에 닿는가(unwired 아님)
3. 정확성 버그: 상태 수명, 동시성, 오래된 응답 적용, panic
4. 검사가 실제로 동작을 단언하는가. 동결 remote-web·Cargo 변경, 주석·매직넘버·`#[allow]`·하드코딩 문자열이 있는가
5. 기능표 complete 근거가 충분한가(과대 판정)
6. 공용 파일 영역 블록 규칙을 어겨 다른 lane과의 병합 충돌을 키우는가

`blocking`에는 재현 시나리오가 있는 실제 결함만 넣습니다. 수정 단계는 차단 항목마다 실제 코드로 확인해 결함이면 재현 테스트 → 근본 수정, 리뷰가 틀렸으면 "리뷰 반박: 근거"로 적습니다. 과대 판정은 근거를 보강하거나 partial로 낮춥니다. 수정 내역은 해당 묶음 QA 문서에 "수정(리뷰 반영)" 절로 추가합니다.

## 7. lane 커밋(메인, lane worktree에서)

작업자는 커밋하지 않습니다. workflow가 끝나면 메인이 합니다.

1. 작업자 결과 JSON의 `unresolved`·`needsUserDecision`·`verification`을 읽습니다. 사용자 결정이 필요한 항목은 06 문서 형식으로 모아 둡니다.
2. 보고를 그대로 믿지 않고 실제 파일로 확인합니다: `git -C <worktree> status --short`, `git -C <worktree> diff --stat`, 변경 파일 일부 열람, QA 문서에 적힌 명령과 결과 수가 있는지 확인(`docs/agent-operations.md` 3절). 의심되면 그 검사만 lane target에서 다시 실행합니다.
3. 커밋 두 개로 나눕니다. 한 번에 한 커밋분만 경로를 명시해 스테이징하고 `git -C <worktree> diff --cached --stat`로 확인한 뒤 커밋합니다.
    - 코드+테스트: `feat(native): <lane이 연결한 기능 요약>`(예: `51df8a0f feat(native): 알림 진입점·외부 열기·단일 인스턴스·IDE 동기화와 창·표시 층 잔여 연결`)
    - QA+기능표: `docs(native): <lane> lane 검증 기록과 기능표 갱신`(예: `51da81b0`)
4. lane 브랜치는 push하지 않습니다.

## 8. 병합 절차와 충돌 해결

### 8.1 기본 절차(메인 작업 트리)

1. 메인 작업 트리가 clean인지 확인합니다: `git -C /Users/hyunseokbyun/development/TAIDE status --short`.
2. `git -C /Users/hyunseokbyun/development/TAIDE merge --no-ff lane/<lane> -m "merge(native): <lane> lane 병합"`.
3. 충돌 파일 목록: `git -C /Users/hyunseokbyun/development/TAIDE diff --name-only --diff-filter=U`. 8.2·8.3절대로 해결합니다.
4. 표식이 남지 않았는지 확인합니다: 충돌 파일마다 `grep -nE '^(<<<<<<<|=======|>>>>>>>)' <파일>` 결과가 없어야 합니다. 기능표는 `=======`가 표 구분선과 겹치지 않으므로 같은 명령을 씁니다.
5. 의미 충돌을 컴파일로 찾습니다: app·ui crate `cargo check --quiet --all-targets`(ui는 `--features native-host,inspection`). 컴파일 오류가 없어도 8.3절의 "조용한 의미 충돌" 검색을 합니다.
6. 해결한 파일만 경로를 명시해 스테이징하고 병합 커밋을 만듭니다.
7. 9절 게이트를 실행합니다.

2026-11-03 실제 진행: wave 1 다섯 lane을 차례로 병합 커밋한 뒤(`e87b053c` → `4fa4beff` → `446964b7` → `fb43d2ad` → `6dba64f9`) 게이트를 한 번 돌렸고, settings 병합(`3864e1db`)은 그 뒤 따로 게이트를 돌렸습니다. 운영 문서 원칙은 병합마다 게이트 1회입니다. 여러 lane을 묶어 병합할 때도 병합 커밋마다 5번(컴파일 확인)은 하는 것을 권장합니다. 의미 충돌은 컴파일 오류로 먼저 드러나기 때문입니다.

### 8.2 다른 방식: lane worktree 안에서 메인을 병합(git lane의 현재 방식)

git lane은 `lane/git` worktree에서 `to_rust_native`를 병합하는 중입니다(MERGE_MSG `merge(native): to_rust_native를 git lane에 병합`, MERGE_HEAD `0c52b438`). 이 방식은 충돌 해결 동안 메인 작업 트리를 비워 둘 수 있습니다. 마무리 순서:

1. worktree에서 남은 충돌(현재 기능표 3묶음)을 해결하고, 5번 컴파일 확인을 lane target으로 한 뒤 병합 커밋을 만듭니다.
2. 메인 작업 트리에서 `git -C /Users/hyunseokbyun/development/TAIDE merge --no-ff lane/git -m "merge(native): git lane 병합"`을 실행합니다. `lane/git`이 이미 `to_rust_native`를 포함하므로 충돌 없이 끝나고, 다른 lane 병합과 같은 first-parent 이력이 됩니다(권장). `to_rust_native`가 `0c52b438`에서 움직이지 않았다면 `--ff-only`도 가능하지만 그때는 병합 커밋의 첫 부모가 `lane/git`이 됩니다.
3. 게이트는 메인 작업 트리에서 돌립니다. 게이트 스크립트가 `ROOT=/Users/hyunseokbyun/development/TAIDE`와 메인 target을 고정해 쓰기 때문입니다.

### 8.3 충돌 유형과 해결

**텍스트 충돌**(git이 표식으로 알려 줌)

| 유형 | 해결 | 실제 예 |
| --- | --- | --- |
| 같은 위치에 두 lane이 테스트·함수를 추가 | 둘 다 남김(공존) | `446964b7`의 `application-commands-tests.rs`, `application-terminal-agent-tests.rs` |
| 같은 struct에 두 lane이 필드를 추가 | 두 필드를 모두 남기고(합집합), 그 struct를 만드는 모든 생성자에 두 필드의 초기화식을 모두 넣음. test 전용 생성자(`#[cfg(...)]`)도 빠뜨리지 않음 | `446964b7` `host.rs` `HostIntegrations`: shell의 `wait_markers`와 terminal-agent의 `agent_cli`를 모두 두고, terminal-agent가 새로 만든 `connect_with_agent_cli`(`#[cfg(all(test, target_os = "macos"))]`)에도 `wait_markers: Arc::default()`를 추가 |
| `lib.rs` 모듈 목록, `domains/mod.rs`, `intents/mod.rs` | 두 줄 모두 알파벳 순 위치에 둠 | 매 병합의 `native/taide-native-app/src/lib.rs` |
| locale 카탈로그 | 두 키 모두 정렬 위치에 두고 en·ko·ja 세 파일이 같은 키 집합인지 확인 | seams 1.4절 |
| `main.rs` 실행 흐름 | 두 lane의 추가 문장을 원래 순서 의미에 맞게 모두 둠 | `3864e1db`: settings의 `taide_infra::perf::init(std::iter::empty());`와 shell의 `Launch::parse` + `single_instance::acquire` 블록 |
| 기능표 같은 행 | 10.4절 | `shell-89`, `explorer-search-22`, `explorer-search-23` |

**의미 충돌**(텍스트로는 충돌하지 않지만 병합 결과가 컴파일되지 않거나 동작이 틀림)

1. **ToastLayers**: shell lane이 `NativeApplication.toasts`를 `crate::toast::Toasts`에서 창별 층 `crate::toast_layers::ToastLayers`로 바꿨습니다. 다른 lane 검사의 `application.toasts = crate::toast::Toasts::with_actions().unwrap();`가 타입 오류가 되어 `application.toasts = crate::toast_layers::ToastLayers::new().unwrap();`로 고쳤습니다(`446964b7`, `application-commands-tests.rs` 2곳, `application-terminal-agent-tests.rs`).
2. **PaintSink 필드**: `application.rs`의 `PaintSink`(이벤트를 각 수신함에 기록하는 `EventSink`)에 shell lane이 `providers: Arc<crate::provider_events::Inbox>`, settings lane이 `settings: Arc<domains::settings::Events>`를 추가했습니다. 필드 두 개, `publish` 안 `record` 두 줄, 생성자의 `let provider_events = ...; let settings_events = ...;`와 `providers: provider_events.clone(), settings: settings_events.clone()`를 모두 둡니다(`3864e1db`). settings 상태 literal도 두 lane 필드(`presentation_failures`, `sections_ports`, `sections_events`, `reported_failures`)의 합집합입니다.
3. **탐색기 mount 분리(Mounts API)**: explorer-search lane이 `explorers`를 프로젝트 키 `HashMap`에서 mount(slot) 키 `crate::explorer_mounts::Mounts`로 바꿨습니다(`native/taide-native-app/src/explorer-mounts.rs`). 다른 lane 코드의 옛 호출을 새 API로 옮깁니다(`6dba64f9`).

| 옛 호출 | 새 호출 |
| --- | --- |
| `explorers.contains_key(&project)` | `explorers.of_project(&project).next().is_some()` |
| `explorers.get_mut(&project)` | `explorers.of_project_mut(&project).next()` |
| `explorers.values()`(전체 순회) | `explorers.all_mut()` |
| `explorers.remove(project)` | `explorers.forget_project(project)` |

`.next()`는 그 프로젝트의 첫 mount만 고릅니다. 호출하는 코드가 그 프로젝트의 모든 mount(주 창 slot과 보조 창)를 다뤄야 하면 `.next()` 대신 반복문으로 바꿉니다. 이 판단이 의미 충돌 해결의 핵심입니다.
4. **`DocumentKey::Virtual`**(git lane 병합에서): git lane이 diff 화면용 가상 문서 `DocumentKey::Virtual(VirtualKey)`를 추가했습니다(`native/taide-native-editor/src/document.rs`, git worktree 기준). 두 종류의 영향이 있습니다.
    - 완전한 `match`는 컴파일 오류로 드러납니다. 비슷한 위치의 git lane 처리(예: `persistence.rs` `DocumentKey::Virtual(_) => return Ok(false)`, `ide-sync.rs` `=> None`)를 보고 같은 의미의 arm을 넣습니다.
    - `matches!(snapshot.key, DocumentKey::AppFile(_))` 같은 부정 검사는 컴파일되지만 가상 문서를 내장 언어 서비스·선택 범위·문제 목록에 흘려보냅니다. 다른 lane이 새로 만든 파일까지 찾아 `DocumentKey::AppFile(_) | DocumentKey::Virtual(_)`로 넓힙니다. 검색: `rg -n "DocumentKey::AppFile\(_\)" /Users/hyunseokbyun/development/TAIDE-lanes/git/native`. 현재 worktree에서는 `editor-css.rs` 36, `editor-html.rs` 56, `editor-selection-ranges.rs` 162, `editor-problems.rs` 387이 이미 넓혀져 있습니다. git lane QA(`2026-11-02-native-lane-git-g5-diff.md`)도 `.json` diff 본문의 JSON 서비스 오류 toast를 가상 문서 제외로 고친 사례를 적었습니다.

일반 요령: 한 lane이 공용 타입(enum 변형 추가, 필드 타입 교체, 컬렉션 키 변경)을 바꿨으면 병합 직후 그 타입의 모든 사용처를 `rg`로 찾아 다른 lane이 새로 만든 사용처를 확인합니다. 병합 커밋 메시지나 PROCESS에 "의미 충돌 N종(이름)"으로 적습니다.

## 9. 병합 게이트

### 9.1 스크립트

`/Users/hyunseokbyun/development/TAIDE/docs/utils/2026-11-03-native-merge-gate.sh <로그 디렉터리>`. 로그 디렉터리는 scratchpad 같은 저장소 밖 임시 경로를 씁니다. 하는 일:

1. `native-lsp-mock` example 빌드 → `lsp-mock <exit>`
2. 앱 crate 전체 테스트 대상 `--no-fail-fast -- --test-threads=4` → `app <exit>`, `app.log`. `--test-threads=4`는 실제 App 검사 동시 수를 줄여 host 큐(용량 64) 포화 흔들림을 줄이려는 값입니다.
3. UI crate `--features native-host,inspection --no-fail-fast` → `ui <exit>`
4. editor·syntax·terminal crate `--no-fail-fast` → `<crate> <exit>`
5. 동결 remote-web Wasm check → `remote-web <exit>`
6. app·ui·editor·syntax·terminal `cargo fmt -- --check` → `fmt-<crate> <exit>`
7. 각 로그의 `test result: ok` 줄 수(`ok=N`)와 ` ... FAILED` 줄, `error[`·`error:` 줄

모든 명령이 `--locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`(메인 target)을 씁니다.

**실행 전 반드시 확인(휴지통 보호)**: 이 스크립트는 앱 crate를 `--skip` 없이 실행하므로 실제 휴지통을 쓰는 검사 3개(04 문서 2절)가 포함됩니다. 2026-10-10 배치 게이트는 세 검사를 `--skip`으로 뺐습니다. 스크립트를 그대로 쓰지 말고, 앱 단계 줄의 `--test-threads=4` 뒤에 아래 세 인자를 붙인 사본을 쓰거나 스크립트 수정을 사용자에게 제안합니다(스크립트 변경은 `docs/utils` 커밋 대상).

```
--skip remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다 --skip 실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다 --skip 실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다
```

2026-11-03 게이트 두 번은 이 `--skip` 없이 실행됐습니다. 사용자 휴지통에 `delete-synthetic.txt` 같은 합성 파일이 들어갔을 수 있어 사용자 보고 대상입니다(휴지통 자체는 건드리지 않음).

### 9.2 공용 crate를 바꾼 병합의 추가 검사

lane이 `crates/taide-*`(Tauri 앱과 공유)를 바꿨으면(`git -C /Users/hyunseokbyun/development/TAIDE diff --stat <병합 전 커밋> <병합 커밋> -- crates`) 루트 workspace에서도 확인합니다. settings 병합 때 공용 crate 6개 테스트와 루트 workspace check를 실행했습니다(`docs/PROCESS.md` W1 항목).

```
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/Cargo.toml -p <바뀐 공용 crate> --locked --offline --no-fail-fast
cargo check --manifest-path /Users/hyunseokbyun/development/TAIDE/Cargo.toml --workspace --locked --offline
```

### 9.3 결과 판정

1. exit가 모두 0이면 통과입니다.
2. `error[` 줄이 있으면 컴파일 실패입니다. 병합(의미 충돌) 문제이므로 고치고 다시 실행합니다.
3. `FAILED` 줄마다 어느 테스트 대상인지 찾습니다: `grep -nE "^ +Running |FAILED$" <로그 디렉터리>/app.log`에서 FAILED 바로 위의 `Running` 줄(예: `Running unittests src/lib.rs`, `Running tests/terminal-host.rs`)이 대상입니다.
4. 실패한 검사를 하나씩 직렬로 다시 실행합니다(다른 cargo가 돌지 않을 때).

```
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target --lib -- --exact --test-threads=1 <모듈::경로::테스트_이름>
cargo test --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --locked --offline --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target --test <파일 이름> -- --exact --test-threads=1 <테스트_이름>
```

5. 분류합니다.

| 결과 | 분류 | 처리 |
| --- | --- | --- |
| 단독 재실행 통과 | 흔들림(부하) | PROCESS에 "N건 직렬 재실행 통과"와 이름을 적음. 고치지 않음(기대값 변경이 필요한 검사 설계 문제) |
| 단독에서도 실패, 아래 기존 실패 목록에 있음 | 기존 실패 | PROCESS에 "기존 실패"로 적음 |
| 단독에서도 실패, 목록에 없음 | 회귀 후보 | 병합이 관련 파일을 바꿨는지 `git diff --stat <병합 전> <병합 후> -- <검사 파일> <대상 파일>`로 확인. 바꿨으면 병합 회귀로 보고 고친 뒤 영향 검사만 다시 실행. 바꾸지 않았으면 언제부터 실패했는지 근거(커밋)를 찾아 기존 실패로 기록 |

**기존 실패와 흔들림 목록(2026-11-03 기준)**

| 검사 | 종류 | 근거 |
| --- | --- | --- |
| `editor_fonts::tests::editor_fonts는_터미널이_적재한_정적_face를_공유하고_가변_face는_공유하지_않는다`(앱 lib) | 기존 실패 | `docs/PROCESS.md` W0·W1, 분류 근거 `docs/quality-assurance/2026-11-02-native-lane-terminal-agent-terminal-core.md` "editor_fonts 실패의 분류 근거 보강" |
| `crates/taide-runtime/tests/agent_probe.rs`의 소스 문자열 단언 1건(루트 workspace) | 기존 실패(`81e96198`부터) | `docs/PROCESS.md` settings 병합 줄 |
| `context_menu_tests::실제_추가_경계_보조창_메뉴는_클립보드와_터미널의_소유_pane을_보존한다`, `view_state_tests::실제_일괄_닫힘은_주_보조_영역의_독립_위치와_접기를_보존한다`, `file_loading_tests::실제_거절_캐시는_보조_mirror의_수명_파일_변경과_프로젝트_종료를_따른다` | 흔들림(실행마다 0~2개, 보조 창 layout revision 경쟁) | seams 문서 3절 |
| `project_rail` 그룹 검사 | 흔들림(host 큐 64 포화) | `docs/PROCESS.md` 단계 E2 |
| startup 종료 대기 시간 초과 2건 | 2026-10-27 단계 D에서 기존 실패로 분류(HEAD에서도 같은 위치). W0 이후 게이트 기록에는 다시 나오지 않음 | `docs/PROCESS.md` 단계 D, W0 |

6. 디스크 여유를 확인해 PROCESS에 적습니다(`df -h /Users/hyunseokbyun/development`).
7. 게이트 결과를 PROCESS에 적는 실제 형식(`docs/PROCESS.md` W1 항목): "게이트(`docs/utils/2026-11-03-native-merge-gate.sh`): 앱 66 대상 통과·lib 5건 실패 중 4건 직렬 재실행 통과(부하 흔들림), editor_fonts 1건 기존 실패. UI 30·editor 72·syntax 20·terminal 13 대상 통과, remote-web Wasm·fmt 통과".

## 10. 기능표 갱신 규칙

파일: `docs/quality-assurance/2026-10-09-native-function-matrix.md`(Git 병합 전에는 git worktree의 같은 파일이 Git 행의 최신). 영역 절 `## shell` … `## commands-ui`의 표 열은 `ID | 요구사항 | TS 근거 | 현재 판정 | 근거 묶음 | 범위/잔여 동작`입니다.

### 10.1 판정 값

| 값 | 기준 |
| --- | --- |
| complete | 구현 + 실제 앱 경로 + 해당 동작의 자동 검사 근거. 픽셀·OS 입력·접근성·성능·출시 통과를 뜻하지 않음(그것은 배치 31~33) |
| partial | 일부 하위 동작이나 검증이 남음. "범위/잔여 동작"에 남은 것을 구체적으로 적음(예: "남은 것: searchEditor 본문(explorer-search lane)") |
| unwired | 모델·서비스·계약은 있으나 실제 앱 경로의 소비자가 없음 |
| missing | 사용자 구현이 없음 |
| not-applicable | 웹 기술·내부 지표 전용이거나 사용자 결정으로 범위 밖(예: preview-38, 결정 10 A). 근거 결정을 적음 |
| frozen | 사용자 동결 범위(remote-web) |

### 10.2 갱신 규칙

1. lane은 자기 묶음 ID 행만 고칩니다. 사양이 정정을 제안한 행(not-applicable 등)도 근거와 함께 고칩니다.
2. 중복 소유 결정(06 문서 5절)으로 다른 영역 행이 같은 구현으로 닫히면 그 행도 "같은 근거"로 갱신하고 QA에 목록을 적습니다(선례: shell lane이 terminal-agent-57·59~63·70·73, settings-23을 갱신).
3. 근거 열에는 QA 문서 이름과 실제 App 검사 요지를 적습니다. 다른 lane에 남긴 부분은 lane 이름을 적습니다.
4. 상단 합계 표는 lane이 고치지 않습니다(script `DONE`).
5. 행 수로 전체 전환율을 대신하지 않습니다. 진행 보고에서는 "기능표 기준"이라고 밝힙니다.

### 10.3 상단 합계 재계산

현재 상단 표와 문단은 2026-10-14 기준(complete 314, 분모 588)에서 갱신되지 않았고, 최신 합계는 PROCESS에 적어 왔습니다. 메인이 병합 게이트 뒤 docs 커밋에서 같은 수치로 상단 표·문단과 PROCESS를 함께 맞추는 것을 권장합니다. 집계 명령(`M`은 기능표 절대 경로):

```
grep -cE '^\| [a-z-]+-[0-9]+ +\|' M
grep -cE '^\| [a-z-]+-[0-9]+ +\|(\\\||[^|])*\|(\\\||[^|])*\| +complete +\|' M
grep -cE '^\| shell-[0-9]+ +\|(\\\||[^|])*\|(\\\||[^|])*\| +partial +\|' M
```

- 첫 줄은 전체 행 수(현재 599), 둘째 줄은 상태별, 셋째 줄은 영역(`shell`, `editor`, `terminal-agent`, `explorer-search`, `preview`, `git`, `settings`, `commands-ui`)·상태별입니다. 상태는 `complete`, `partial`, `unwired`, `missing`, `not-applicable`, `frozen`.
- 요구사항 칸 안의 이스케이프된 파이프(`\|`, 예: `git-61`)를 열 구분자로 잘못 세지 않도록 `(\\\||[^|])*` 패턴을 씁니다. 단순 `[^|]*`로 세면 `git-61` 한 행이 빠집니다.
- 여섯 상태의 합이 전체 행 수와 같아야 합니다. 기능 분모 = 전체 − not-applicable − frozen(현재 599 − 18 − 1 = 580).
- 2026-11-03 `0c52b438` 직접 집계: complete 434, partial 59, unwired 47, missing 40, not-applicable 18, frozen 1.

### 10.4 기능표 행 충돌 해결

병합에서 같은 행이 양쪽에서 바뀌면 아래 순서로 한 행을 만듭니다.

1. 그 ID의 소유 lane을 06 문서 5절(중복 범위 소유)로 확인합니다.
2. 한쪽이 소유 lane의 구현 근거 행이고 다른 쪽이 비소유 lane이 남긴 "다른 lane 소관" 자리표시 행이면 소유 lane 행을 택합니다. 실제 예(2026-11-03 관찰한 git worktree 기능표 404-413줄의 충돌 묶음. 다른 작업자가 해결 중이라 줄 번호는 바뀔 수 있음. ours = `lane/git`, theirs = `to_rust_native`): `explorer-search-22`·`23`은 ours(git lane)가 complete + 실제 App 근거, theirs(explorer-search lane)가 "결정 D3 A: git lane이 구현" missing이므로 ours를 택합니다.
3. 양쪽이 서로 다른 일부를 구현했으면 근거를 합치고, 남은 것은 양쪽 "남은 것"의 교집합으로 다시 씁니다. 남은 것이 없어지면 complete, 남으면 partial입니다. 실제 예(같은 파일 182-186줄): `shell-89`는 ours가 diff·commit diff·claudeDiff 본문 연결(남은 것: searchEditor, ClaudeDiff revert 화살표), theirs가 searchEditor 본문 연결(남은 것: diff 계열)이므로 합치면 남은 것은 ClaudeDiff revert 화살표(git-50)뿐이고 판정은 그 근거를 확인해 정합니다.
4. "근거 묶음" 열 값은 상단 "현재 근거 지도" 표에 있는 묶음 이름이어야 합니다.
5. 표식 제거 뒤 10.3절 명령으로 전체 행 수가 바뀌지 않았는지(새 행을 의도적으로 추가한 경우 제외), `grep -c "^| <ID> "`가 1인지 확인합니다.

## 11. QA 문서 형식

이름: `docs/quality-assurance/<YYYY-MM-DD>-native-lane-<lane>-<묶음키>.md`. 실제 예: `docs/quality-assurance/2026-11-02-native-lane-shell-providers.md`, `docs/quality-assurance/2026-11-02-native-lane-settings-s2-lsp-ai.md`.

```markdown
# lane <lane> 묶음 <n>: <묶음 이름>

> 작성일: <날짜> · lane: `<lane>`(worktree `TAIDE-lanes/<lane>`, 브랜치 `lane/<lane>`) · 사양: `docs/research/2026-10-28-lane-<spec>-spec.md` <절> · 결정: `docs/acknowledge/2026-10-28-native-lane-spec-decisions.md`(4.1 추천안)

## 대상
| ID | 기능 | 결과 |   (complete / partial(남은 것) / 정정 사유)
같은 근거로 갱신한 다른 영역 행 목록

## 원본 동작
원본 근거 경로 줄-줄. 사양에 있으면 "사양 n절" 참조, 구현 중 다시 확인한 것만 적음

## 구현
### 공통 / ### <ID>별
### 사양과 실제 코드가 달라 실제 코드를 따른 곳
### 원본과 다르게 둔 점(근거)

## 검증
공통 cargo 옵션 한 줄. | 명령 | 결과(통과 수 / 실패와 원인) |
새 검사 목록(crate 단위 / 실제 App / UI 하네스)
실패 선행 확인 여부, 실행하지 않은 것과 이유

## 남은 범위
ID별 남은 동작, 병합 뒤 연결할 것, 허락 대기, 테스트 부채(재현 조건·생략 이유·남은 위험·실행 시점)

## 수정(리뷰 반영, <날짜>)
### 과대 판정 정정 / ### 비차단 반영 / ### 반영하지 않은 비차단 항목(사유) / ### 검증(수정)
```

## 12. 커밋·push 순서

1. lane worktree: `feat(native): ...`(코드+테스트) → `docs(native): <lane> lane 검증 기록과 기능표 갱신`(QA+기능표). 7절.
2. 메인 작업 트리: `merge(native): <lane> lane 병합`. 8절.
3. 게이트(9절). 실패를 고치면 그 수정은 별도 커밋(`fix(native): ...`)입니다.
4. `docs/PROCESS.md` 갱신 → `docs(native): <lane> lane 병합과 게이트 결과 기록`(여러 lane이면 `docs(native): lane 5개 병합과 게이트 결과 기록`).
5. `git -C /Users/hyunseokbyun/development/TAIDE push`(일반 push, upstream `origin/to_rust_native`). force 계열 금지.
6. 병합 뒤 연결할 항목(예: git-38 → explorer-search-7 탐색기 Git 장식 공급, git-7 → shell `notify_event`)은 PROCESS의 "병합 뒤 연결"에 적고, 메인이나 다음 묶음이 처리합니다.
7. 다음 wave lane은 push한 새 HEAD에서 분기합니다(2절).

모든 커밋에서 04 문서 5절(선별 스테이징, `git diff --cached` 확인, AI 트레일러 금지)을 지킵니다.

## 13. PROCESS 기록

- 위치는 `docs/PROCESS.md` 맨 위 "진행: 배치 24 — 셸 상호작용" 절의 W0~W4 체크리스트입니다. 4150줄 파일이므로 처음 80줄만 읽고 그 절만 고칩니다.
- 한 줄에 날짜, 병합 커밋, 충돌 수와 종류, 의미 충돌 이름, 게이트 결과(9.3절 7번 형식), 기능표 합계(10.3절), 디스크, 실행 중인 lane, 병합 뒤 연결 항목을 적습니다.
- 작업자는 PROCESS를 고치지 않습니다. 메인만 고칩니다.

## 14. 중단과 재개

### 14.1 사용자 지시의 해석

- "멈춰", "중단", "그만" 같은 명시적 단어가 있을 때만 workflow를 멈춥니다. "얼마나 남았어", "언제까지 해" 같은 질문은 정보 요청이므로 단계별 예상 시간 표로 답하고 계속 진행합니다(`docs/acknowledge/2026-10-06-native-transition-decisions.md` 진행 방식 표, 사용자 auto-memory `session-duration-checkpoints`).
- "implement만 끝내고 멈춰": 진행 중인 구현 작업자가 끝나는 즉시 workflow를 멈추고, 리뷰 전 변경은 lane worktree에 미커밋으로 둡니다. PROCESS에 어디서 멈췄는지 적습니다(선례: 단계 E, `docs/PROCESS.md`).
- "여기까지 commit push": 진행분을 검증된 범위에서 체크포인트로 커밋하고 메시지에 상태를 밝힙니다(`docs/HANDOFF.md` 5절 5번).

### 14.2 같은 run 재개

- 같은 script + 같은 args + `resumeFromRunId: <run ID>`로 다시 실행합니다. 완료된 작업자는 캐시로 건너뜁니다.
- 재개 전에 script의 프롬프트 문자열을 바꾸면 캐시가 깨져 처음부터 다시 실행됩니다. 추가 규칙은 아직 실행되지 않은 단계의 프롬프트에만 넣습니다(`docs/memory/2026-10-28-native-lane-operations.md` 5절). `BACKGROUND`·`RULES`·`DONE`은 구현과 수정 프롬프트가 함께 쓰므로 고치면 구현 캐시가 깨집니다.
- run 기록 위치(관찰): 실행 정보 `~/.claude/projects/-Users-hyunseokbyun-development-TAIDE/<세션 ID>/workflows/<run ID>.json`(args 포함), 작업자 기록 `.../<세션 ID>/subagents/workflows/<run ID>/journal.jsonl`. 장애 때 journal을 보고 이어받습니다(`docs/agent-operations.md` 3절).

### 14.3 새 세션에서 재개

1. 사용자 전역 규칙과 이 묶음 문서(04·05·06)를 읽습니다.
2. `sed -n 1,80p /Users/hyunseokbyun/development/TAIDE/docs/PROCESS.md`로 활성 작업을 확인합니다.
3. 실제 상태를 파일로 확인합니다: `git -C /Users/hyunseokbyun/development/TAIDE status --short`, `git -C /Users/hyunseokbyun/development/TAIDE worktree list`, lane마다 `git -C <worktree> status --short`와 `git -C <worktree> log --oneline -3`, 병합 중이면 `.git/worktrees/<lane>/MERGE_HEAD`.
4. 사용자 전역 규칙(`ai-process.md` 1.2절)에 따라 재개 경계(새 세션, resume, clear, compact, handoff)마다 workflow 사용 여부를 한 번 묻고 답을 기다립니다. 이전 선택을 자동 승계하지 않습니다. 단 사용자가 현재 요청에서 이미 정했으면 다시 묻지 않습니다.
5. 미커밋 변경이 있는 lane은 5절 1번처럼 읽고 이어서 작업합니다.

## 15. 사용량 한도로 끊긴 workflow

- 작업자 하나가 사용량 한도로 죽으면 workflow가 같은 지시로 자동 재시작합니다(memory 2026-10-07 3절). 묶음 지시의 실행 순서 1번이 `git status`로 미커밋 변경을 확인하고 이어서 작업하게 되어 있어, 재시작한 작업자가 앞 작업을 이어받습니다.
- workflow 전체가 멈췄으면 한도가 풀린 뒤 14.2절대로 같은 run을 재개합니다. journal에 결과가 없는 작업자는 다시 실행된다는 사실을 사용자에게 그대로 보고합니다.
- 사용자가 한도 초기화 시각에 맞춰 재개를 지시한 선례가 있습니다("limit이 리셋되는 12:30분부터 시작", 2026-10-06).
- 재개 전 lane worktree의 미커밋 변경이 컴파일되는지(`cargo check --all-targets`) 확인하면 재시작 작업자가 깨진 상태에서 시작하지 않습니다(권장).

## 16. 디스크와 target 관리

| 항목 | 값·명령 |
| --- | --- |
| 메인 target | `/Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target`(게이트용). 실기용 번들 `TAIDE M8 Egui Spike.app`·`TAIDE M8 Iced Spike.app`이 같은 디렉터리에 있음 |
| lane target | `/Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>`, 한 벌 약 25GB(W0 측정) |
| dev 프로필 | 각 native crate `Cargo.toml`에 `[profile.dev] debug = "line-tables-only"`, `[profile.dev.package."*"] debug = false`(`72065136`) |
| 확인 | 전체 테스트 대상을 빌드하기 전과 병합 뒤 `df -h /Users/hyunseokbyun/development`. 여유 100GB 미만이면 빌드를 늘리기 전에 정리를 제안 |
| 메인 target 정리 | `cargo clean --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --target-dir /Users/hyunseokbyun/development/TAIDE/experiments/native-shell-spike/target --profile dev`(`debug`만 지워 번들 보존). 정리 뒤 첫 앱 검사 전에 `native-lsp-mock`을 다시 빌드 |
| lane 정리(병합·push 뒤) | `git -C /Users/hyunseokbyun/development/TAIDE worktree remove /Users/hyunseokbyun/development/TAIDE-lanes/<lane>` → `git -C /Users/hyunseokbyun/development/TAIDE branch -d lane/<lane>` → `cargo clean --manifest-path /Users/hyunseokbyun/development/TAIDE/native/taide-native-app/Cargo.toml --target-dir /Users/hyunseokbyun/development/TAIDE-lanes/targets/<lane>` |
| 금지 | `rm -rf`(자동 모드가 거부). `cargo clean *`은 사용자가 허용 규칙을 추가해 둠 |
| 루트 Tauri target | 루트 `target/`는 별도. 2026-10-07에 457GB까지 누적된 적이 있음(`docs/feedback/2026-10-07-build-cache-disk-usage.md`) |

현재 병합이 끝난 lane 6개의 worktree·target이 아직 남아 있습니다. git lane 병합·push가 끝난 뒤 함께 정리하면 lane당 약 25GB를 회수합니다.

## 17. 남은 순서(참고)

1. git lane 병합 마무리(8.2절) → 게이트(휴지통 `--skip` 포함) → PROCESS·push.
2. 병합 뒤 연결: git-38 → explorer-search-7 Git 장식 공급, git-7 → shell `notify_event`(PROCESS W1 항목의 "병합 뒤 연결").
3. 기능표 상단 합계 갱신(10.3절).
4. wave 3: commands-ui 사양 보충 절(결정 9 A) → commands-ui lane.
5. wave 4: 배치 31 구조 정리·전체 게이트(편집기 장애 경계 설계 포함, `docs/PROCESS.md` 단계 E2의 shell-12 보류). 배치 32·33과 허락 대기 항목은 06 문서 9·10절.
