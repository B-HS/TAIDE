# M6 격리 앱 실제 종료 이벤트 계측

## 대상 파일

- `src-tauri/src/lib.rs`의 `RunEvent::ExitRequested`·`RunEvent::Exit`와 자원 대기 완료 로그
- `crates/taide-runtime/src/exit_drain.rs`의 공통 자원 대기 경로
- 테스트 전용 `/private/tmp/taide-m6-isolated.byLYaMWs/project` 프로젝트

## 리포트

`src-tauri/src/lib.rs`에 경로·요청 값 없이 이벤트와 drain 완료 단계만 남기는 info 로그를 추가했습니다. `cargo test --offline -p taide --lib 직접_exit은_전체_자원_drain을_사용한다 --quiet`는 1건 통과했고 `cargo clippy --offline -p taide --lib -- -D warnings`, Rust fmt, diff 검사도 exit 0입니다. 새 코드가 포함된 전용 identifier의 debug `.app`을 오프라인으로 빌드했으며, 빌드가 exit 0인 뒤 로컬 ad hoc 서명·검증도 exit 0이었습니다. 원본 stdout에는 다른 진단 정보도 있어 저장소에 넣지 않고 아래 종료 단계만 선별해 기록합니다.

## 실제 순서와 자원

세 번 모두 격리 설정과 임시 프로젝트로 앱을 실행하고 `⌘Q`로 종료했습니다. 각 실행에서 창 제어가 `App quit`, 앱 프로세스가 exit 0을 반환했습니다. info 로그의 순서는 세 번 모두 같습니다.

1. `종료 요청 이벤트 수신`
2. `종료 요청 자원 대기 완료`
3. `종료 요청 이벤트 수신`
4. `직접 종료 이벤트 수신`
5. `직접 종료 자원 대기 완료`

첫 실행은 기존 `fixture.txt` 탭과 프로젝트가 복원된 상태였습니다. 프로젝트 watcher가 임시 프로젝트의 두 엔트리 인덱싱을 기록했고, 종료 뒤 IDE listener 16727·격리 lockfile이 없어졌습니다.

둘째 실행은 같은 프로젝트의 터미널 탭에서 `/bin/sleep 120`을 실행해 자식 PID 55613의 생존을 확인한 뒤 종료했습니다. 종료 직후 그 PID가 없어졌고 IDE listener 49833·격리 lockfile도 없어졌습니다.

셋째 실행은 임시 `fixture.ts`를 열어 vtsls 자식 PID 55725의 실행을 확인한 뒤 종료했습니다. 종료 직후 그 PID가 없어졌고 IDE listener 40235·격리 lockfile도 없어졌습니다. 세 실행 모두 종료 drain 실패·panic 로그는 없었습니다.

## 아직 증명하지 않은 범위

두 번째 `ExitRequested` 다음 `Exit`는 첫 요청의 비동기 drain 완료가 다시 종료를 요청한 경로입니다. 선행 `ExitRequested` 없이 OS가 바로 `Exit`를 전달하는 경우의 실앱 수명은 이번 조작에서 일어나지 않았으며 합성 직접 Exit 테스트의 근거와 구분합니다. watcher·PTY·LSP를 한 실행에서 모두 동시에 바쁘게 만든 검사는 아니고 원격 서버도 켜지 않았습니다. 이 결과는 IDE·PTY·LSP의 실제 자식/포트 회수에 한정하며, 다른 OS 강제 종료나 모든 nested 작업의 회수를 입증하지 않습니다.
