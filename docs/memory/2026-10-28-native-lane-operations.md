# Rust-native 전환 lane 운영 방식 (현행)

> 최종 갱신일: 2026-10-28 · 사용자 지시 "문서부터 바꿔 최대한 효율적으로, 정확하게, 완벽하게, 신속하게"
> `docs/memory/2026-10-07-native-transition-operations.md`의 1절(배치 형태)·3절(작업자 지시)·4절(통합 게이트)을 대체합니다. 5절(화면 캡처)·6절(함정)은 계속 유효합니다.

## 1. 왜 바꾸나 (2026-10-26~28 관찰)

| 병목 | 관찰값 | 원인 |
| --- | --- | --- |
| 직렬 실행 | 단계 하나(3~7 ID)에 1.5~7시간, 배치 24 잔여 42행에 약 2일 | 모든 기능이 `native/taide-native-app/src/application.rs`(11,582줄)·`host.rs`(2,477줄)·`taide-native-ui/src/shell.rs`(2,829줄)를 함께 고쳐 병렬 작업이 충돌합니다. crate는 나뉘어 있지만 앱 crate 안의 통합 지점이 하나입니다 |
| 반복 조사 | 작업자마다 원본 TS·native 코드를 처음부터 다시 읽음 | 영역 단위 사양이 없고 지시가 ID 목록뿐이었습니다 |
| 잘게 나눈 리뷰 | 작업마다 리뷰·수정 왕복 | 리뷰 단위가 작업 단위와 같았습니다 |
| 디스크 | 공용 target 489GB, 여유 307GB | dev 프로필 debuginfo 전체, 테스트 대상 수십 개 |
| 부하 흔들림 | 앱 lib 병렬 검사에서 host 큐(64) 포화·종료 대기 시간 초과 | 실제 App 검사가 동시에 많이 돕니다 |

## 2. 새 구조

### 2.1 단계 0: lane 경계 만들기 (1회, 직렬, 동작 변화 없음)

병렬 lane 전에 통합 지점을 영역별 파일로 나눕니다. 동작은 바꾸지 않는 순수 구조 이동이며 기존 테스트가 그대로 통과해야 합니다.

1. `application.rs`의 영역별 처리(호스트 응답 반영, intent 처리, 프레임 tick, 영역 상태)를 `native/taide-native-app/src/domains/<영역>.rs`로 옮깁니다. `application.rs`는 영역 모듈 호출만 남깁니다. 영역: `shell`, `editor`, `explorer_search`, `git`, `terminal_agent`, `settings`, `commands`, `preview`.
2. `HostCommand`·`HostReply`를 영역별 하위 enum(`HostCommand::Git(GitCommand)` 등)으로 나누고 각 영역 파일에 둡니다. worker의 처리도 영역 파일로 옮깁니다.
3. `ShellIntent`와 `ShellSurfaces`도 영역 하위 enum·영역 trait로 나눕니다. 동결된 `taide-remote-web`의 소스·lockfile이 바뀌지 않아야 합니다.
4. 각 native workspace(`taide-native-{app,ui,editor,syntax,terminal}`)의 `Cargo.toml`에 `[profile.dev] debug = "line-tables-only"`와 `[profile.dev.package."*"] debug = false`를 둡니다. 의존 그래프·lockfile은 바뀌지 않습니다.
5. 기존 `cargo fmt --check` 차이(앱 `application.rs`·`lib.rs` 모듈 순서, `experiments/lsp-coordinator-spike` mock-server)를 정리합니다.
6. `cargo clean --profile dev`로 공용 target을 비우고, 앱 테스트 대상 한 벌의 크기를 측정해 lane 동시 수를 정합니다.

### 2.2 lane

- lane = 영역 하나의 남은 기능 묶음입니다. 각 lane은 자기 git worktree(`/Users/hyunseokbyun/development/TAIDE-lanes/<lane>`, 브랜치 `lane/<lane>`)와 자기 target 디렉터리(`<worktree>/target-lane`)를 씁니다. 같은 target을 공유하면 cargo 잠금으로 빌드가 직렬화되므로 나눕니다.
- 동시 lane 수 = min(4, (디스크 여유 − 100GB) / 측정한 한 벌 크기). CPU 18코어·메모리 128GB 기준 4가 상한입니다.
- lane 안의 순서:
    1. **사양**(opus·xhigh, 1회): 영역의 모든 남은 ID에 대해 원본 TS 동작(경로·줄), native 기존 자산, 설계, 작업 묶음 계획을 `docs/research/<날짜>-lane-<영역>-spec.md`에 씁니다. 이후 작업자는 이 문서와 필요한 파일 구간만 읽습니다.
    2. **구현**(opus·xhigh, 묶음마다 직렬): 묶음 하나는 6~10 ID입니다. 실패 테스트 → 구현 → 영역 검사 → 대응표 행 갱신. 묶음이 끝나면 lane 브랜치에 작업자가 아니라 workflow 종료 뒤 메인이 커밋합니다.
    3. **리뷰**(sonnet·xhigh, lane 끝 1회): lane 전체 diff를 원본 대조·정확성·과대 판정 기준으로 봅니다.
    4. **수정**(opus·xhigh, 차단 항목이 있을 때 1회).
- 영역 소유: lane은 자기 영역 파일(`domains/<영역>.rs`, 영역 UI 모듈, 영역 테스트, 영역 QA)만 고칩니다. 공용 파일(`application.rs`, `lib.rs`의 모듈 목록, 공용 enum 루트, locale 카탈로그)은 추가만 하고, 추가 위치는 각 파일의 영역 블록 끝입니다.

### 2.3 통합 (메인)

1. lane workflow가 끝나면 메인이 lane worktree에서 검증 결과를 확인하고 선별 커밋합니다.
2. 메인이 lane 브랜치를 `to_rust_native`에 하나씩 병합합니다(`git merge --no-ff`). 충돌은 메인이 해결합니다.
3. 병합마다 게이트를 1회 실행합니다: 변경 crate 전체 테스트 대상 `--no-fail-fast -- --test-threads=4`(실제 App 검사 동시 수를 제한해 host 큐 포화 흔들림을 없앰), crate별 `cargo fmt --check`, 동결 remote-web Wasm check, lockfile 허용 범위 확인, 디스크 확인.
4. 일반 push 뒤 다음 wave의 lane은 새 HEAD에서 분기합니다. 끝난 worktree는 `git worktree remove`, target은 `cargo clean`으로 회수합니다.

## 3. wave 순서

| wave | lane (동시) | 범위 |
| --- | --- | --- |
| 0 | 없음(직렬) | 2.1 lane 경계, 프로필, fmt, target 정리 |
| 1 | shell · explorer-search · terminal-agent · editor+preview | 배치 24 잔여(단계 G와 F 잔여), 25, 27, 23+30 |
| 2 | git · settings | 배치 26, 28 |
| 3 | commands-ui | 배치 29와 앞 wave에서 진입만 남은 행, shell-89 |
| 4 | 메인 | 배치 31 구조·전체 게이트(편집기 장애 경계 설계 포함), 32·33은 사용자 허락 뒤 |

## 4. 유지하는 기준

- 완료 기준은 그대로입니다: 원본 TS 화면·상태·상호작용 대응 + 실제 앱 경로 + 해당 동작의 자동 검사. 원본 결함·내부 수치는 억지로 재현하지 않습니다.
- 작업자 공통 금지: GUI 실행, 사용자 데이터·OS 설정·클립보드·Keychain·휴지통(`tests/explorer-delete.rs` 실행 금지), 합성 OS 입력, 저장소 밖 탐색, git 상태 변경, 허용 밖 의존성.
- 모델: 사양·구현·수정 opus·xhigh, 리뷰 sonnet·xhigh(사용자 지시).
- 허락이 필요한 것은 보류 목록으로: shell-15(실제 사용자 데이터 경로), 배치 32·33, 사용자 실기.

## 5. 중단·재개

- "implement만 끝내고 멈춰": 진행 중 구현 작업자가 끝나는 즉시 workflow를 정지하고 리뷰 전 변경은 lane worktree에 미커밋으로 둡니다. 재개는 같은 script + `resumeFromRunId`(완료 작업자는 캐시). 재개 전에 script의 프롬프트 문자열을 바꾸면 캐시가 깨지므로, 추가 규칙은 아직 실행되지 않은 단계의 프롬프트에만 넣습니다.
- 소요 시간 질문은 정보 요청이며 중단 지시가 아닙니다.
