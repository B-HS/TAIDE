# M7 계측용 읽기 화면 단일 실측

## 환경과 경계

전용 identifier `net.gumyo.taide.m7perf.0ee5f27e64f94daa82e4789556697b3c`의 새 release 앱 `/private/tmp/taide-m7perf-v2.o5viwq/TAIDE.app`을 `TAIDE_PERF=1`과 격리 `ZDOTDIR`·`CLAUDE_CONFIG_DIR`·`TMPDIR`로 실행했습니다. 사용자 프로젝트 대신 `/private/tmp/taide-m7-fixture.csNotl/project`의 5,002개 파일·1,000개 커밋·변경 20건 합성 저장소만 사용했습니다. 앱은 임시 서명 검증을 통과했지만 배포 서명·공증 산출물은 아닙니다.

설정의 성능 진단은 로컬 데스크톱에서만 보였고 `스냅샷 읽기`가 프런트·Rust 값과 Monaco 모델·TanStack Query 캐시 개수를 표시했습니다. 원격 진단 IPC 차단 코드는 변경하지 않았습니다. 각 조작 전에 화면의 `지표 초기화`로 양쪽 누적치를 함께 0으로 만들었고, 표본당 조작은 한 번만 했습니다.

## 관찰

| 지표 | 단일 표본 | 판정 |
| --- | --- | --- |
| 복원 부팅 | `boot.reveal` 52ms·1회, Rust `setup.main_window` 75.739084ms·`setup.locale_warm` 0.580208ms·`setup.state_restore` 0.568083ms·`setup.deferred_restore` 1.982459ms(각 1회) | 서로 다른 구간이며 전체 첫 페인트 시간으로 합산하지 않습니다. |
| 프로젝트 전환 | 프런트 `project.switch` 11ms·1회, Rust `project_activate` 11.006375ms·1회, `project_open` 0회 | 복원된 기존 프로젝트의 전환입니다. 신규 프로젝트 첫 열기 기준은 미충족입니다. |
| 1KB 파일 첫 열기 | 프런트 `file.open` 91ms·1회, Rust `file_open` 0.145167ms·1회 | 합성 `small.txt` 첫 열기 충족. |
| 1MB 파일 첫 열기 | 프런트 `file.open` 20ms·1회, Rust `file_open` 1.223833ms·1회 | 합성 `large.txt` 첫 열기 충족. |
| 250개 파일 폴더 펼침 | 프런트 `tree.toggle` 8ms·1회, Rust `tree_toggle` 0.304708ms·1회 | `files/group-0` 첫 펼침 충족. |
| Git 상태 | 변경 20건·그래프 100건 표시, Rust `git_status` 15.716708ms·1회 | Git 화면 진입 단일 표본 충족. |
| Enter 전역 검색 | `M7SearchNeedle`로 5,000개 파일·5,000개 결과, 프런트 `search.results` 62ms·1회, Rust `search_run` 83.30625ms·1회, `search_list_files` 0회 | 자동 입력 검색을 일시적으로 끄고 Enter 한 번만 실행했습니다. 파일 목록은 캐시되어 수집 호출이 없었고 자동 검색은 원래의 켜짐으로 복원했습니다. |

터미널은 복원된 탭의 화면이 비어 있었으나 준비 명령 후 Rust `pty.output_bytes` 614바이트·프런트 `terminal.output-bytes` 618바이트가 확인됐습니다. 이후 양쪽을 0으로 초기화한 뒤 `time seq 2000000`을 한 번 제출했지만 종료 스냅샷은 Rust 378바이트·프런트 996바이트에 그쳐 200만 줄 실행이나 처리량 표본으로 인정하지 않습니다. 새 터미널 탭을 만든 뒤 `New Tab Menu` 팝업이 닫히지 않아 격리 앱을 한 번 재실행했습니다. 재실행한 PID에서도 명령 준비 출력은 Rust 820바이트·프런트 824바이트였지만, 양쪽 지표를 다시 초기화한 뒤 `seq 2000000`을 한 번 제출한 종료 스냅샷은 Rust 368바이트·프런트 1192바이트였습니다. `pty_write`는 12회 기록됐어도 200만 줄 출력은 확인되지 않아 처리량으로 인정하지 않습니다. 재시작 없이 같은 명령을 반복하지 않았습니다.

첫 부팅 스냅샷의 Monaco 모델은 0개, 쿼리 캐시는 42개였습니다. 이는 20개 파일을 열고 닫은 뒤의 메모리 표본이 아닙니다. 재실행한 동일 PID에서 `files/group-0/file-0000.txt`부터 `file-0019.txt`까지 20개를 실제 탭으로 열고 모두 닫았으며, Rust `file_open` 20회와 남은 파일 탭 0개를 확인했습니다. 직후 읽기 화면은 Monaco 모델 0개·쿼리 캐시 105개·상태 표시 RAM 135MB였고, 이어서 WebKit Inspector Memory 단일 기록의 현재·최대 값은 281.07MB(JavaScript 101.30MB, Page 179.77MB)였습니다. 메모리 수치는 WebKit timeline 값이지 앱 RSS가 아니며, 한 번의 기록으로 장시간 누수 부재를 주장하지 않습니다.

같은 격리 환경에서 기존 합성 저장소의 HEAD를 새 `/private/tmp/taide-m7-fixture.csNotl/cold-project` worktree로 체크아웃하고 앱의 `Open by Path`로 첫 열기를 실행했습니다. 화면 제목과 탐색기가 `cold-project`로 바뀌고 워처 로그에 5,025개 엔트리 인덱싱이 확인됐지만, `Add Project` 메뉴와 경로 대화상자가 닫히지 않아 읽기 화면으로 돌아갈 수 없었습니다. `project_open`·`project_activate` 수치를 확보하지 못했으므로 프로젝트 전환 행은 미완료입니다. 부팅 행 또한 네 구간의 유효 수치는 있지만 전체 첫 페인트 마크가 없어 미완료입니다.
