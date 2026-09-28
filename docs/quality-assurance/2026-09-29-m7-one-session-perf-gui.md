# M7 격리 release 단일 세션 성능·GUI 실측

## 범위와 fixture

2026-09-29 macOS 26.5.2 arm64에서 `TAIDE_PERF=1`로 전용 identifier의 계측용 release 앱을 한 번 실행하고, 종료하지 않은 채 조작을 이어 갔습니다. 앱 설정·임시 프로젝트·터미널은 `/private/tmp`의 격리 경로를 사용했습니다. 배포 서명·공증을 받은 제품 빌드는 아닙니다.

`/private/tmp/taide-m7-fixture.csNotl/project`는 텍스트 파일 5,002개, Git 커밋 1,000개, 수정된 추적 파일 20개를 가진 합성 저장소입니다. `small.txt` 1,024바이트와 `large.txt` 1,048,576바이트를 포함합니다. 사용자 프로젝트와 IDE 인증 파일 내용은 읽지 않았습니다.

같은 조건에서 이미 통과한 조작은 재실행하지 않았습니다. Rust 스냅샷은 누적치이며, `maxMs`나 `count`를 개별 조작의 정확한 소요 시간으로 바꿔 주장하지 않습니다. 프런트 `performance` measure는 발생 순서와 해당 UI 조작을 대조했습니다.

## 단일 세션 관찰

| 경로 | 실제 관찰 | 판정 범위 |
| --- | --- | --- |
| 프로젝트 전환 | 작은 격리 프로젝트와 합성 저장소를 순서대로 열었고 `project.switch`가 14ms, 9ms였습니다. Rust `project_open` 2회는 합계 89.776ms, 최대 60.872ms였습니다. | 합성 저장소 전환의 프런트 표본은 9ms입니다. Rust 최대값을 해당 전환의 단독 표본으로 단정하지 않습니다. |
| 파일 열기 | 탐색기에서 1KB와 1MiB 파일을 처음 열었고 `file.open`은 순서대로 96ms, 34ms였습니다. Rust `file_open`은 이후 탭 이동까지 포함해 3회, 최대 0.374ms입니다. | 두 파일의 프런트 표본은 확보했습니다. |
| 명령 팔레트 | 닫힌 상태에서 `⌘⇧P`로 열고 `file` 네 글자를 입력했습니다. 해당 `palette.open`은 3ms, 첫 `palette.filter`는 5ms였습니다. | 한 번의 열기·입력 경로만 표본으로 사용합니다. |
| 트리 | 250개 파일이 있는 `group-0`을 펼쳤고 `tree.toggle`은 5ms였습니다. Rust `tree_toggle`은 상위 폴더를 포함한 2회 중 최대 1.370ms입니다. | 200개 이상 조건을 충족했습니다. |
| Git | Git 화면에 `CHANGES 20`과 최신 1,000개 커밋 중 그래프 100개가 표시됐습니다. Rust `git_status`는 세션 누적 5회 중 최대 18.932ms입니다. | 실제 화면은 통과했습니다. Git 화면 진입만 분리한 시간은 미측정입니다. |
| 전역 검색 | `M7SearchNeedle` 입력의 실시간 검색에서 `5000 results in 5000 files`가 표시됐습니다. Rust `search_run` 69.787ms, `search_list_files` 3.346ms였습니다. | `search.results`는 Enter 제출만 기록하는 구현이라 실시간 검색의 프런트 값은 없습니다. 통과한 검색을 다시 실행하지 않았습니다. |
| 터미널 | 앱 터미널에서 `time seq 2000000`을 한 번 실행하고 셸 프롬프트 복귀·경과 1.458초를 봤습니다. Rust `pty.output_bytes`는 362→20,778,665바이트, `pty.output_chunks`는 3→494개였습니다. 프런트 누적 `terminal.output-bytes`는 20,778,730바이트, `terminal.output-chunks`는 496개였습니다. | Rust 증가분 20,778,303바이트 기준 약 14.25MB/s입니다. 프런트 실행 전 값은 보관하지 않아 프런트 증가분은 단정하지 않습니다. |
| 메모리 | 수정 파일 20개를 연 뒤 `Close Saved`로 일괄 닫고 남은 테스트 파일도 저장·닫았습니다. 앱 상태 표시 RAM은 닫기 전 163MB, 이후 156MB였습니다. | 정확한 WebKit Memory 스냅샷·Monaco 모델·쿼리 캐시 수는 미측정입니다. 콘솔 전역에 `monaco`·`queryClient`·`performance.memory`가 노출되지 않았습니다. |

후속 유휴 상태에서 같은 앱의 RAM 표시가 949MB로 올랐고, PID 41408의 `ps -o rss`도 971,648KiB로 일치했습니다. `vmmap -summary`의 physical footprint는 829.5MB, `MALLOC_LARGE (empty)` resident는 788.4MB였으며 `heap --noContent -s -H`의 live malloc 합계는 15.0MB였습니다. 이 차이는 높은 RSS를 실제 live 객체 누수로 단정할 수 없다는 근거일 뿐, 장시간 유지된 높은 메모리 점유 자체를 통과시키는 근거는 아닙니다. 200만 줄 출력·탭 닫기·검색 중 무엇이 큰 일시 할당을 만들었는지는 분리하지 않았습니다.

같은 PID를 종료하지 않고 `leaks -noContent -quiet -nostacks 41408`로 한 번 더 진단했습니다. 명령은 미참조 할당 314건·15,616바이트를 보고하고 exit 1이었습니다. 이는 관찰된 높은 RSS의 대부분을 설명하지 않지만, WebKit의 JavaScript 객체·캐시나 할당자 보유 영역의 원인과 해제 가능성까지 판정하지는 않습니다. 메모리 기준선은 계속 미완료입니다.

작은 합성 파일에 테스트 입력 한 글자가 미저장 상태로 남은 것을 확인해 undo 후 원문과 같은 내용으로 저장했습니다. 마지막 확인에서 파일은 1,024바이트이고 `s` 외의 바이트는 0개였습니다.

## GUI 경로와 남은 판정

- 탭 우클릭 메뉴에서 1MiB 파일을 새 OS 창으로 이동했고, 새 창의 접근성 트리에서 파일 편집기를 확인한 뒤 `Move back to Main Window`로 되돌렸습니다.
- 네이티브 File 메뉴의 Open Recent 항목, 명령 팔레트의 키보드 열기·필터·닫기, 탐색기·검색·Git·터미널의 접근성 트리와 실제 화면을 각각 확인했습니다. 기존 Open Folder 대화상자·Settings Appearance 부분 실기는 [이전 기록](2026-09-29-m7-release-gui-smoke.md)을 재사용합니다.
- 같은 앱을 유지한 후속 조작에서 프로젝트의 `Add Project → Open by Path…` 메뉴·대화상자를 열었습니다. 존재하지 않는 격리 경로를 제출하자 `Path not found` 알림이 나타났고, `Cancel`로 대화상자를 닫았습니다. 이 검사는 성공적인 프로젝트 열기나 저장 데이터 변경을 주장하지 않습니다.
- 테마·로케일 실제 변경, WebKit Memory 기록, 합성 저장소를 복원한 부팅, 활성 원격 WebSocket을 포함한 직접 Exit는 아직 판정하지 않았습니다. 화면·상태·상호작용·접근성 전수 및 M7 전체 통과를 주장하지 않습니다.
