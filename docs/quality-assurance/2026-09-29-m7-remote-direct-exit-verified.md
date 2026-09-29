# M7 활성 원격 WebSocket 직접 Exit 실측

## 대상

- 계측용 release 번들: `/private/tmp/taide-m7perf-bundle.WkuYc5/TAIDE.app`
- 격리 합성 프로젝트: `/private/tmp/taide-m7-fixture.csNotl/project`
- 앱 PID 53006, 통합 PTY의 Node WebSocket 검사기 PID 53054, localhost 원격 포트 52728
- 대상 코드: `src-tauri/src/lib.rs`의 Tauri 종료 이벤트와 `crates/taide-runtime/src/exit_drain.rs`의 자원 대기 경로

## 실행과 관찰

격리 앱의 원격 접근을 일시적으로 켜고 새 일회용 링크를 UI 자동화 세션 메모리에서 검사기에 전달했습니다. 링크·쿠키 값은 셸 명령이나 저장소 파일에 기록하지 않았지만, 접근성 트리를 다시 바인딩한 한 차례의 도구 응답에는 일회용 링크가 표시됐습니다. 해당 링크는 바로 소비됐고 앱 종료·재시작으로 격리 세션 저장소가 초기화됐으며 서버도 껐습니다. `lsof -nP -iTCP:52728`에서 앱의 LISTEN 소켓과 앱·Node 사이 양방향 ESTABLISHED 소켓을 확인했습니다. `ps -p 53006 -o pid=,ppid=,command=`의 실행 경로가 위 번들과 정확히 일치했습니다.

사용자가 이 격리 앱에 한정해 승인한 AppleScript `tell application <정확한 번들 경로> to quit`을 한 번 실행했고 `osascript`는 exit 0이었습니다. 캡처된 앱 stdout의 종료 단계는 다음 순서였습니다.

1. `applicationWillTerminate`
2. `직접 종료 이벤트 수신`
3. `직접 종료 자원 대기 완료`

선행 `종료 요청 이벤트 수신`은 캡처된 종료 로그에 없었고 앱 실행은 exit 0으로 끝났습니다. 종료 뒤 `ps -p 53006,53054`는 두 프로세스를 찾지 못했으며 `lsof -nP -iTCP:52728`도 listener·연결을 찾지 못했습니다. 즉 활성 원격 WebSocket과 통합 PTY 검사기가 함께 있는 실제 직접 Exit 경로에서 자원 대기 완료와 OS 자원 정리를 관찰했습니다.

원격 설정은 재시작 시 `on`으로 복원됐습니다. 같은 격리 앱을 재시작해 자동으로 열린 포트 52957과 연결 0개를 확인한 뒤 스위치를 `off`로 바꿨습니다. 화면에 `Stopped`와 링크 생성 버튼 비활성이 나타났고 `lsof -nP -iTCP:52957`은 출력 없이 exit 1이었습니다.

## 판정과 잔여 범위

활성 원격 WebSocket·PTY가 있는 격리 앱의 선행 `ExitRequested` 없는 직접 Exit는 통과했습니다. 앞선 SIGTERM 실험의 exit 143과 달리 Tauri `Exit` drain 완료가 stdout으로 확인됐습니다. 같은 순간에 LSP 자식과 watcher callback까지 모두 바쁘게 만든 상황, OS stall·강제 종료까지 증명한 것은 아닙니다. PTY·LSP가 함께 있는 직접 Exit는 [별도 실측](../history/2026-09-29-m6-exit-event-trace.md)에 기록돼 있습니다. 이전 실패 시도는 [별도 기록](2026-09-29-m7-remote-direct-exit-attempt.md)으로 유지합니다.
