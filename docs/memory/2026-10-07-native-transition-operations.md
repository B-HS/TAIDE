# Rust-native 전환 배치 운영 지식

> 최종 갱신일: 2026-10-09 · 대응 커밋: `bf5175ca`
> 2026-10-06~07 세션에서 배치 1~7을 돌리며 굳어진 운영 방법입니다. 항상 적용하는 재사용 지식이며, 사용자 결정은 `docs/acknowledge/2026-10-06-native-transition-decisions.md`가 정본입니다.

## 1. 배치의 형태

한 배치는 구현 2~3단계를 **같은 작업 트리에서 직렬로** 실행합니다. 단계마다 구현(opus·xhigh) → 리뷰(sonnet·xhigh, 읽기 전용) → 차단 항목이 있으면 수정(opus·xhigh) 순서입니다. 리뷰는 다음 단계 구현과 동시에 돌리고, 수정은 모든 구현이 끝난 뒤 직렬로 돌립니다. 읽기 전용 조사·설계는 구현과 병렬로 돌릴 수 있습니다.

직렬인 이유: native 크레이트가 서로를 path로 참조해, 한 작업자가 `taide-native-ui`를 고치는 중이면 다른 작업자의 `taide-native-app` 빌드가 깨집니다. 배치 1 때는 작업이 미추적 상태라 git worktree 격리도 쓸 수 없었습니다. 지금은 전부 커밋돼 있어 worktree를 쓸 수 있지만, `application.rs`·`editor_surface.rs`를 여러 단계가 함께 고쳐 병합 충돌이 커서 쓰지 않았습니다.

관찰값: 배치당 작업자 6~8, 약 280만~390만 토큰, 실행 2~5시간.

## 2. 감사 workflow의 구성

영역 10개(셸, 편집기, 터미널·에이전트, 탐색기·검색, 미리보기, Git, 설정, 명령·UI, IPC 대응, 구조)를 병렬로 돌렸습니다. 판정은 done / partial / unwired(모듈은 있으나 실제 앱 실행 경로에서 도달 불가) / missing / n/a입니다. "연결 여부"는 `main.rs → NativeApplication`까지 호출 체인을 따라가 확인하게 했습니다. 이전 에이전트의 진척 수치가 실제와 크게 달랐던 원인이 unwired였기 때문입니다.

## 3. 작업자 지시에 반드시 넣는 것

- 동작 기준은 TS 소스와 Monaco·Shiki·xterm의 실제 소스이며, 근거 경로를 QA 문서에 적게 합니다. 기준 자료를 구현 출력에 맞춰 만들지 않게 합니다.
- 셸은 절대 경로를 쓴 한 줄 명령만 허용하고 `cd`·파이프·`&&`·`rm`·`python`을 금지합니다. cargo는 한 번에 하나, 정해진 target 디렉터리만 씁니다.
- `Cargo.toml`·`Cargo.lock`은 단계별로 허용 범위를 명시합니다. 동결된 `taide-remote-web`의 lockfile과 의존 그래프는 바뀌면 안 됩니다.
- 구현을 임시로 끄거나 되돌려 실패를 재현하는 편집을 금지합니다(자동 모드에서 거부되고, 원복 실수 위험이 있습니다).
- 범위 밖 수정은 최소로 하고 보고하게 합니다. 지정 범위를 넘은 수정은 거의 매 단계 발생했고 대부분 정당했습니다.
- **컨텍스트 절약**: 큰 파일(`editor_surface.rs`, `application.rs`, `store.rs`, Monaco 소스)은 구간만 읽고, cargo에 `--quiet`를 붙이고, QA 문서 초안을 절반 지점에 쓰게 합니다. 배치 6의 접기 단계가 "Prompt is too long"으로 중단된 뒤 추가한 규칙입니다.
- 중단 후 재시작에 대비해 "범위 파일에 미커밋 변경이 있으면 읽고 이어서 작업"을 실행 순서 1번에 둡니다. 사용량 한도로 작업자가 죽으면 workflow가 같은 지시로 자동 재시작합니다.

## 4. 메인의 통합 검증 게이트

배치가 끝나면 메인이 직접 실행합니다.

1. 변경한 크레이트 전부의 **전체 테스트 대상**을 `--no-fail-fast`로 1회: `taide-native-app`(66개 대상), `taide-native-syntax`, `taide-native-editor`, `taide-native-ui --features inspection`, `taide-remote-web --features inspection`.
2. 크레이트별 `cargo fmt -- --check`.
3. lockfile diff가 허용 범위인지 확인.
4. 디스크 여유 확인.
5. 화면이 켜져 있으면 창 캡처.

lib 테스트만 돌리면 통합 테스트 대상의 회귀를 놓칩니다(`docs/feedback/2026-10-07-batch-integration-full-test-gate.md`). 실패가 배치의 회귀인지 판별할 때는 `git stash`로 변경을 치우고 같은 테스트를 커밋 기준에서 실행했습니다(다른 작업자가 돌지 않을 때만).

알려진 상시 실패: `tests/preview-spreadsheet-xlml.rs` 1건(인수 이전부터). 알려진 간헐 실패: `tests/explorer-delete.rs`의 `실제_탐색기_삭제는_…`(부하 시 1회 관찰).

## 5. 화면 캡처 방법

1. 캡처용 데이터 디렉터리를 만듭니다. `src-tauri/tests/fixtures/rust-native/persistence-v1.json`의 legacy 형식을 따라 `session.json`, `projects/<id>/project.json`, `projects/<id>/layout.json`을 쓰면 프로젝트와 파일 탭이 열린 상태로 시작합니다. 프로젝트 root는 합성 샘플 디렉터리를 가리키게 합니다.
2. `experiments/native-shell-spike/target/debug/taide-native-app --data-dir <디렉터리>`로 실행합니다.
3. `CGWindowListCopyWindowInfo`에서 owner 이름에 `taide-native-app`이 들어간 창 중 제목이 `TAIDE`인 창의 번호를 얻어(Swift 한 파일로 가능) `screencapture -x -o -l <창 번호> <파일>`로 그 창만 캡처합니다.
4. `kill -TERM <pid>`로 종료합니다.

주의: 화면이 잠겨 있으면 창 내용이 그려지지 않아 빈 흰색으로 캡처됩니다. `CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements])`의 첫 창 owner가 `ScreenLock`이면 잠긴 상태입니다. 합성 키·마우스 입력은 사용자의 다른 창으로 갈 수 있어 쓰지 않습니다. 2026-10-07의 legacy 형식 데이터로 실제 프로젝트가 열리는지는 화면 잠금 때문에 확인하지 못했습니다.

## 6. 반복된 함정

- **`cargo check --workspace`가 CPU 0%로 멈춤**: 2026-10-06에 루트 워크스페이스 check가 24분간 멈췄습니다(루트 `target/debug` 457GB, 디스크 여유 70GB 상태). 종료 후 재실행은 1분 21초에 끝났습니다. 원인은 확인하지 못했습니다.
- **`cargo run`이 바이너리를 못 고름**: `taide-native-app`에 bin이 둘(`taide-native-app`, `native-terminal-queue-fixture`)이라 `--bin`이 필요합니다.
- **캐시를 비운 뒤 앱 lib 테스트 10건 실패**: `native-lsp-mock` 예제 실행 파일이 없어서입니다. `cargo build --example native-lsp-mock`을 먼저 실행합니다.
- **새 크레이트의 JSON 자산과 prettier**: 저장소 루트의 `prettier --check .`가 `native/`를 포함합니다. 추출·생성한 JSON 경로는 `.prettierignore`에 넣습니다(문법 파일은 라이선스상 수정하면 안 됩니다).
- **`rm -rf`·`cargo clean` 거부**: 자동 모드가 되돌릴 수 없는 삭제를 막습니다. `cargo clean *`은 사용자가 허용 규칙을 추가했습니다.
- **bun 스크립트**: TS 스택으로 기준 자료를 만들 때 `bun <절대 경로>` 한 줄 실행은 작업자에게서 거부되지 않았습니다. 지시에 허용을 명시합니다.
- **이전 에이전트의 QA 문서 "PASS"**: 인수 시점에 lib 테스트 7건과 ui 통합 테스트 1건이 실제로는 실패했습니다. 과거 문서의 통과 기록을 현재 상태로 쓰지 않습니다.
- **구조상 미리 알아야 할 것**: 전역 키맵 라우팅이 `terminal_surface.rs`의 `Views`에 있고, `native-app/src/keymap.rs` 등 일부 파일은 `cfg(test)`에서 `taide-native-ui` 소스를 `include!`로 복제 컴파일합니다. 테스트가 배포 코드와 다른 컴파일 단위를 검증할 수 있습니다.
