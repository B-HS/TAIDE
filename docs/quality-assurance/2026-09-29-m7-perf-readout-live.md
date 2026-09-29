# M7 계측용 읽기 화면 단일 실측

## 환경과 경계

전용 identifier `net.gumyo.taide.m7perf.0ee5f27e64f94daa82e4789556697b3c`의 새 release 앱 `/private/tmp/taide-m7perf-v2.o5viwq/TAIDE.app`을 `TAIDE_PERF=1`과 격리 `ZDOTDIR`·`CLAUDE_CONFIG_DIR`·`TMPDIR`로 실행했습니다. 사용자 프로젝트 대신 `/private/tmp/taide-m7-fixture.csNotl/project`의 5,002개 파일·1,000개 커밋·변경 20건 합성 저장소만 사용했습니다. 앱은 임시 서명 검증을 통과했지만 배포 서명·공증 산출물은 아닙니다.

설정의 성능 진단은 로컬 데스크톱에서만 보였고 `스냅샷 읽기`가 프런트·Rust 값과 Monaco 모델·TanStack Query 캐시 개수를 표시했습니다. 원격 진단 IPC 차단 코드는 변경하지 않았습니다. 각 조작 전에 화면의 `지표 초기화`로 양쪽 누적치를 함께 0으로 만들었고, 표본당 조작은 한 번만 했습니다.

## 관찰

기존 읽기 화면의 `boot.reveal`은 창 `show()` 호출 전 종료되므로 `src/shared/hooks/use-reveal-window.ts`에서 `show()` 성공 완료 시각을 별도 마크로 기록하고, `src/shared/lib/perf-mark.ts`의 보고서에 WebKit `first-paint`·`first-contentful-paint`와 `performance.timeOrigin`을 함께 노출하도록 보강했습니다. 이는 기존 `boot.reveal` 이름·수치의 뜻을 바꾸지 않고, 페인트가 실제 창 표시 뒤인지 단일 부팅에서 판정하기 위한 경계입니다. 지표 초기화 후에는 부팅 표본이 사라지도록 했습니다. 대상 `bun test src/shared/lib/perf-mark.test.ts` 17건, `bun run typecheck`, 변경 TS 파일 Prettier가 각 exit 0입니다.

후속 `/private/tmp/taide-m7paint.eQ4qtt/TAIDE.app` release 번들의 한 번의 초기화 전 부팅 스냅샷은 `performance.timeOrigin=1790656555448ms`, WebKit `first-contentful-paint=267ms`, `show()` 완료 `268ms`, `first-paint=null`입니다. 사전에 별도 명령으로 얻은 앱 실행 요청 시각 `1790656554633ms`는 실제 프로세스 시작과 떨어져 있으므로 전체 부팅 시간 계산에는 사용하지 않습니다. 같은 스냅샷의 `boot.reveal=61ms`, Rust `setup.main_window=90.770459ms`·`setup.locale_warm=0.558833ms`·`setup.state_restore=2.282375ms`·`setup.deferred_restore=2.025541ms`는 독립 구간으로 보존합니다.

후속 `/private/tmp/taide-m7frame.dwtdRf/TAIDE.app`의 한 번의 초기화 전 부팅에서 OS `proc_pidinfo`의 해당 경로 PID 81090 시작 시각은 `1790657063584.580ms`였습니다. 읽기 화면의 `performance.timeOrigin=1790657064483ms`, WebKit 첫 콘텐츠 페인트 `221ms`, `show()` 완료 `222ms`, `first-paint=null`이므로 프로세스 시작→콘텐츠 페인트는 약 `1119.420ms`, 프로세스 시작→창 표시 완료는 약 `1120.420ms`입니다. 콘텐츠 페인트가 `show()` 완료보다 1ms 앞서고 WebKit이 일반 첫 페인트 항목을 내지 않았으므로 전자는 표시 이후 페인트라고 증명할 수 없으며 후자는 내용이 이미 준비된 창의 표시 완료 상한입니다. 둘을 정확한 첫 가시 페인트 시각으로 바꾸지 않습니다. 이 빌드에서 추가로 시도한 표시 후 두 번째 `requestAnimationFrame` 마크는 같은 PID에서 계속 `null`이어서 부팅 기준선에 쓰지 않고 제품 변경에서도 제거했습니다.

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

후속 M6 결합 종료 준비에서 새 격리 앱의 TypeScript 파일을 열자 vtsls 자식이 실행됐지만, 터미널 입력에 `/bin/sleep 3600`을 제출한 뒤에는 sleep 자식이 생성되지 않았습니다. 앞선 두 번의 대량 출력 실패와 같은 입력 경로에서 세 번째 실행도 성립하지 않아 이 경로의 명령 재시도는 중단했습니다. PTY 셸 존재를 명령 실행 또는 출력 처리량 증거로 바꾸지 않습니다.

후속 읽기 전용 UI 확인에서 동일 PID의 터미널 탭은 접근성 트리에 `Terminal input`을 노출하지만 렌더 영역은 빈 화면이며, 입력 영역 클릭 뒤에도 화면 변화가 없었습니다. 명령을 다시 제출하지 않았으므로 셸 상태나 WebGL 원인을 단정하지 않습니다. [WebKit의 Paint Timing 안내](https://webkit.org/blog/11648/new-webkit-features-in-safari-14-1/)에 따르면 `first-paint`·`first-contentful-paint` 기록이 가능하지만, 기존 `boot.reveal`은 `show()` 전 종료되므로 해당 paint 값이 창 표시 이후인지 함께 확인해야 전체 첫 페인트 표본으로 인정할 수 있습니다.

새 격리 앱 PID 81090의 유일한 자식 셸 PID 81101이 `/dev/ttys000`을 사용하는 것을 부모 PID·TTY·경로로 확인했습니다. UI 키 입력이 실행되지 않는 경로와 분리하기 위해 `/usr/bin/seq 2000000`의 출력을 그 PTY slave에 직접 보냈습니다. 설정 탭이 전면이었던 첫 출력의 writer wall time은 `2.41s`, Rust `pty.output_bytes=20,777,888`·`pty.output_chunks=856`이며, 터미널 탭 재진입 뒤 프런트가 스크롤백 `5,120,000`바이트·3청크를 받았습니다. 이는 백그라운드 출력이라 전면 프런트 처리량으로 사용하지 않습니다.

같은 PID·TTY를 다시 확인하고 지표 초기화 뒤 터미널 탭을 전면으로 둔 유효 후보 한 번에서는 writer wall time `2.44s`, Rust `pty.output_bytes=20,777,791`·`pty.output_chunks=870`, 프런트 `terminal.output-bytes=25,897,791`·`terminal.output-chunks=873`을 확인했습니다. 프런트 값에는 탭 재진입 직후의 기존 스크롤백 `5,120,000`바이트·3청크가 포함되며, 이를 제외한 새 출력은 `20,777,791`바이트·870청크로 Rust와 일치합니다. writer의 단순 나눗셈은 약 `8.52MB/s`지만 프런트가 마지막 바이트를 그린 완료 시각을 별도로 기록하지 않았으므로 이를 최종 렌더 처리량으로 주장하지 않습니다. 전면 출력 뒤에도 스크린샷의 터미널 영역은 비어 있고 AX에는 `Terminal input`만 보였습니다. 바이트 전달은 입증됐지만 문자 가시성과 렌더 처리량은 미판정이며 원인은 단정하지 않습니다.

WebKit Inspector Elements에는 `xterm-screen` 크기 `1088×800px`와 canvas가 존재했습니다. 컨텍스트 손실 뒤 재로드 대신 DOM 렌더러에 남게 하는 한정 수정을 적용한 별도 서명 검증 release 앱 `/private/tmp/taide-m7webgl.zhBUuC/TAIDE.app`에서 같은 합성 프로필의 터미널 한 줄을 출력했습니다. PID 81922의 자식 셸 PID 81933·`/dev/ttys000`을 확인한 뒤 출력했으며 Rust 출력 328바이트·3청크, 프런트 수신 332바이트·4청크가 관찰됐지만 스크린샷의 터미널은 여전히 비어 있었습니다. 폰트 크기 `13→14→13` 갱신도 가시성을 바꾸지 않았습니다. 따라서 이 표본은 WebGL 컨텍스트 손실 원인을 입증하지 못하며, 효과가 확인되지 않은 수정은 제품 소스에서 되돌렸습니다. 출력 전달과 캔버스 존재만으로 화면 페인트를 통과 처리하지 않습니다.

이후 `TAIDE_PERF`용 계측에 xterm `write` 파서 완료 콜백의 바이트와 `onRender`의 행 렌더 프레임·각 마지막 시각을 분리했습니다. 기존 `terminal.output-bytes`는 `term.write` 호출 시점의 **전달량**임을 문서에서 바로잡았습니다. [xterm 공개 API](https://xtermjs.org/docs/api/terminal/classes/terminal/)는 `write` 콜백을 파서 처리 완료로, `onRender`를 행 렌더 이벤트로 정의합니다. 대상 단위 테스트 19건·TS 타입 검사·Vite 및 Tauri release 빌드·로컬 임시 서명 검증이 exit 0입니다. 새 격리 번들 `/private/tmp/taide-m7render.DSakc4/TAIDE.app`의 자식 셸 PID 83125가 앱 PID 83111에 속하고 `/dev/ttys000`을 점유한 것을 확인했습니다. 지표 초기화 뒤 터미널을 전면에 두고 그 PTY에 합성 200만 줄을 **한 번만** 보냈습니다. writer 시작 `1790659287976.007ms`, 종료 `1790659290485.111ms`, `/usr/bin/time -p` real `2.50s`였습니다. Rust `pty.output_bytes=20,777,795`·`pty.output_chunks=887`, 프런트 전달 `20,778,110`바이트·889청크, 파서 완료 `20,778,110`바이트, 마지막 파서 시각 `1790659290480.0002ms`였습니다. 추가 315바이트·2청크는 터미널 재활성화 출력으로 보이나 별도 명세를 확인하지 않아 200만 줄의 순수 프런트 처리량으로 단정하지 않습니다. `terminal.render-frames` 항목이 없고 마지막 렌더 시각은 `null`이며 스크린샷도 빈 화면입니다. 따라서 파서는 처리했지만 최종 렌더 완료는 입증되지 않았고 지표 8은 미완료로 둡니다. 동일 200만 줄 출력을 재실행하지 않습니다.

WebGL 경로의 관여만 분리하려고 macOS에서 `WebglAddon`을 적재하지 않는 임시 비교 빌드 `/private/tmp/taide-m7dom.vMwIhb/TAIDE.app`을 서명 검증 후 실행했습니다. 정확한 앱 PID 83745의 자식 셸 PID 83761·`/dev/ttys000`에 합성 한 줄만 보낸 결과, 프런트 전달·파서 완료는 각 341바이트였지만 `terminal.render-frames` 항목이 없고 마지막 렌더 시각은 `null`, 화면은 여전히 비어 있었습니다. 따라서 WebGL만을 원인으로 볼 근거가 없고 임시 비활성화는 제품 소스에서 되돌렸습니다. 이전 표시 후 `requestAnimationFrame` 마크도 발생하지 않았다는 관찰과 양립하지만, 이 두 사실만으로 WebKit·디스플레이 루프·터미널 중 어느 층이 원인인지 단정하지 않습니다.

프레임 루프 자체를 분리하기 위해 `TAIDE_PERF` 전용 읽기 화면에 1초 `requestAnimationFrame` probe를 추가했습니다. 대상 TS 타입 검사·Prettier·diff 검사는 exit 0이며 별도 계측 release 앱 `/private/tmp/taide-m7focus.eVkPA8/TAIDE.app`의 빌드·로컬 임시 서명 검증도 exit 0입니다. 설정 화면에서 `Read snapshot`을 누른 첫 probe는 `timed-out`이었고, 창 접근성 `Raise`를 실행한 뒤 다시 누르고 같은 UI 호출에서 1.5초 기다린 probe도 `timed-out`이었습니다. 이전 표시 직후 `requestAnimationFrame` 마크 미발생과 합해 동일한 프레임 실행 가정이 세 차례 성립하지 않아 이 환경의 재시도를 멈춥니다. [WebKit의 비활성 창 전력 정책](https://webkit.org/blog/8970/how-web-content-can-affect-power-usage/)은 뒤에 있거나 비활성인 창에서 `requestAnimationFrame`이 정지할 수 있다고 설명하지만, 이 실측만으로 실제 OS 포커스·디스플레이 정책 중 어느 조건이 적용됐는지 단정하지 않습니다. `Read snapshot`은 로컬 계측 화면이고 원격 진단 명령은 계속 차단됩니다. 첫 가시 페인트와 최종 터미널 렌더 수치는 여전히 미완료입니다.

다음 검사 경로를 코드에서 확인했습니다. `src-tauri/src/remote_gateway.rs`의 허용 테이블과 실제 dispatch에는 `terminal_sessions`·`pty_write`·`project_open`이 모두 있으며, 원격 진단 `perf_snapshot`·`perf_reset`은 계속 거부됩니다. 승인된 격리 앱의 원격 접속을 일시적으로 켠 뒤 기존 PTY 세션에 직접 입력하고 새 합성 프로젝트를 열면, UI 입력 실패·팝업과 분리해서 앱 내 읽기 화면으로 결과를 확인할 수 있습니다. 현재 원격 스위치는 꺼져 있으며, 이 경로를 아직 실행하거나 지표 통과로 판정하지 않았습니다.

첫 부팅 스냅샷의 Monaco 모델은 0개, 쿼리 캐시는 42개였습니다. 이는 20개 파일을 열고 닫은 뒤의 메모리 표본이 아닙니다. 재실행한 동일 PID에서 `files/group-0/file-0000.txt`부터 `file-0019.txt`까지 20개를 실제 탭으로 열고 모두 닫았으며, Rust `file_open` 20회와 남은 파일 탭 0개를 확인했습니다. 직후 읽기 화면은 Monaco 모델 0개·쿼리 캐시 105개·상태 표시 RAM 135MB였고, 이어서 WebKit Inspector Memory 단일 기록의 현재·최대 값은 281.07MB(JavaScript 101.30MB, Page 179.77MB)였습니다. 메모리 수치는 WebKit timeline 값이지 앱 RSS가 아니며, 한 번의 기록으로 장시간 누수 부재를 주장하지 않습니다.

같은 격리 환경에서 기존 합성 저장소의 HEAD를 새 `/private/tmp/taide-m7-fixture.csNotl/cold-project` worktree로 체크아웃하고 앱의 `Open by Path`로 첫 열기를 실행했습니다. 화면 제목과 탐색기가 `cold-project`로 바뀌고 워처 로그에 5,025개 엔트리 인덱싱이 확인됐지만, `Add Project` 메뉴와 경로 대화상자가 닫히지 않아 읽기 화면으로 돌아갈 수 없었습니다. `project_open`·`project_activate` 수치를 확보하지 못했으므로 프로젝트 전환 행은 미완료입니다. 부팅 행 또한 네 구간의 유효 수치는 있지만 전체 첫 페인트 마크가 없어 미완료입니다.

후속 동일 PID의 로컬 스냅샷에서 앞선 전환 이후에도 `project_open` 0회, `project_activate` 1회였으므로 이를 신규 프로젝트 첫 열기 표본으로 소급하지 않았습니다. 지표를 초기화하고 같은 합성 저장소의 새 `first-open-2` worktree를 앱의 `Open by Path`로 한 번 열었습니다. 창 제목·탐색기는 새 경로로 바뀌고 watcher는 5,025개 엔트리 인덱싱 완료를 기록했지만, 메뉴·경로 대화상자가 다시 남았고 닫기·취소·Escape 조작도 화면을 바꾸지 못했습니다. 그 뒤의 로컬 스냅샷을 읽지 못했으므로 `project_open` 시간은 여전히 미판정입니다. 이 UI 멈춤을 프로젝트 열기의 성공 시간으로 간주하거나 같은 경로를 반복 측정하지 않습니다.

새 격리 앱 PID 81922에서는 이미 복원돼 열린 `cold-project`를 native `File > Open Recent`로 한 번 선택했을 때 `project_open=11.905083ms`·1회와 프런트 `project.switch`의 마지막 `13ms`가 기록돼 기존 열린 프로젝트 경계를 확인했습니다. 이어 UI에서 해당 합성 프로젝트만 닫아 사이드바에서 사라진 것을 확인하고 지표를 초기화했습니다. native `File > Open Recent`로 닫힌 항목을 한 번 다시 열자 사이드바·제목·탐색기가 복원됐고 프런트 `project.switch=14ms`·1회, Rust `project_open=44.850166ms`·1회로 완료됐습니다. 이 표본은 닫힌 프로젝트의 capability 재부착을 포함하지만 **같은 프로세스에서 앞서 열었던 경로의 재열기**라 완전한 신규 경로 첫 열기나 차가운 OS 캐시를 증명하지 않습니다. 대화상자 정지의 원인을 여기서 단정하지 않습니다.

해당 합성 프로젝트를 다시 닫고 앱을 종료한 뒤 같은 계측 release 번들을 새 프로세스로 시작했습니다. 사이드바에 `cold-project`가 없음을 확인하고 지표를 초기화한 뒤 native `File > Open Recent > cold-project`를 **이번 프로세스에서 처음** 선택했습니다. 사이드바·창 제목·탐색기가 새로 열렸고 프런트 `project.switch=31.000ms`·1회, Rust `project_open=40.843667ms`·1회, `project_activate=0`회였습니다. 이전 표본과 달리 이번 프로세스의 project-open 경계이며, 동일 디스크 경로의 과거 OS 캐시는 남아 있을 수 있습니다. 조작은 표에 적힌 사이드바 클릭이 아니라 native 최근 항목 선택이므로 UI 진입점 차이를 기준선에 명시했습니다. 유효 계측을 얻었으므로 반복하지 않습니다.
