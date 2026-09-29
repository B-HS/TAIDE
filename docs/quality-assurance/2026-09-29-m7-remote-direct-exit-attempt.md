# M7 활성 원격 직접 Exit 실측 시도

## 대상

기존 계측용 release 앱 PID 44412, 합성 프로젝트, 격리 원격 설정을 사용했습니다. 앞선 인증·느린 수신자·폐기 실측은 별도 QA에 있습니다.

## 관찰과 한계

- 새 일회용 링크를 외부 종료 관찰기로 전달하려던 첫 시도는 기존 터미널의 깨진 명령행 때문에 성립하지 않았습니다. 앱은 원격 연결 `0 connected`였고 해당 세션을 전체 폐기한 뒤 원격 서버를 껐습니다.
- 두 번째 외부 관찰기는 임시 Unix 소켓 경로를 다시 열었으나, 터미널 입력이 연결되지 않아 대기 시간에 종료됐습니다. 그때 생긴 사용하지 않은 소켓 파일 하나는 listener 부재를 확인한 뒤 해당 정확한 경로만 삭제했습니다.
- 이어 격리 앱에서 원격 서버가 `Running on port 51125`, `0 connected`인 상태로 활성 WebSocket을 만들려 했으나 새 터미널 대신 파일 검색 팝업이 `Loading...`으로 열려 요청을 보내지 못했습니다. 팝업을 닫지 못해 `⌘Q` 일반 종료를 요청했고 앱 종료가 보고됐습니다. PID 44412와 해당 포트 listener는 이후 조회되지 않았습니다.
- 후속 격리 앱 PID 50878에서 같은 합성 프로젝트의 PTY Node 검사기와 원격 WebSocket `1 connected`를 열었습니다. `lsof -nP -iTCP:51566`은 앱 listener와 앱↔Node의 ESTABLISHED 소켓을 보였습니다. 정확한 앱 PID에 OS SIGTERM을 보낸 뒤 앱·PTY 셸 PID 51129·Node PID 51799와 포트는 모두 사라졌습니다. 이 실행은 종료 이벤트 stdout을 수집하지 않았습니다.
- stdout을 캡처한 별도 실행 PID 52046에서도 PTY 셸 PID 52064·Node PID 52151과 `127.0.0.1:52190`의 ESTABLISHED WebSocket을 확인했습니다. 해당 앱 PID에 SIGTERM을 보내자 프로세스는 exit 143으로 끝났고, 캡처된 stdout에 `종료 요청 이벤트`·`직접 종료 이벤트`·drain 완료 로그는 없었습니다. 세 PID와 포트는 종료 후 조회되지 않았습니다. 따라서 SIGTERM에 의한 OS 자원 회수를 Tauri의 직접 `RunEvent::Exit` drain 완료로 해석하지 않습니다.
- 이 격리 프로필은 원격 스위치 `on`을 재기동 뒤에도 유지했습니다. 후속 재기동에서 자동으로 열린 `127.0.0.1:52544`의 연결 0개를 확인한 뒤 스위치를 `off`로 복원했고 화면 `Stopped` 및 해당 포트 listener 부재를 확인했습니다.

## 판정

서버만 활성화된 앱의 일반 종료와, 활성 WebSocket·PTY가 있는 앱의 SIGTERM 자원 회수는 각각 확인했습니다. SIGTERM은 Tauri 종료 이벤트 로그를 남기지 않고 exit 143으로 끝났으므로 이 시도만으로는 선행 `ExitRequested` 없는 직접 `RunEvent::Exit`와 내부 drain 완료를 판정하지 않습니다. LSP까지 동시에 살아 있는 자원 대기도 미검증입니다. 기존 [직접 Exit 실측](../history/2026-09-29-m6-exit-event-trace.md)의 PTY·LSP 결과를 이 시도의 원격 연결 증거로 바꾸지 않습니다. 후속 AppleScript 직접 Exit의 별도 [성공 실측](2026-09-29-m7-remote-direct-exit-verified.md)이 이 공백 중 활성 원격 WebSocket·PTY 경계를 확인했습니다.
