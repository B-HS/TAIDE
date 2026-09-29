# M7 격리 앱 설정·메모리·원격 경계 단일 실측

## 대상과 방법

2026-09-29 전용 identifier의 계측용 release 앱을 기존 PID 44412에서 종료하지 않고 사용했습니다. 합성 프로젝트가 열린 상태에서 사용자 승인에 따라 테마·언어를 각각 한 번 전환 후 복원하고, WebKit Inspector의 Memory timeline을 한 번 기록했습니다. 원격 접근은 격리 앱에서만 잠시 켰다가 껐으며 사용자 프로젝트와 IDE 인증 파일 내용은 읽지 않았습니다.

## 관찰 결과

- Settings Appearance의 `TAIDE Dark`에서 `TAIDE Light`로 전환했을 때 선택 상태와 실제 밝은 화면이 일치했습니다. `TAIDE Dark`로 복원한 뒤 선택 상태가 다시 켜진 것을 확인했습니다.
- Settings Language의 `System`에서 `English`로 전환한 값이 화면에 표시됐습니다. `System`으로 복원한 값도 확인했습니다. 화면 전체 번역·재시작 후 언어 복원은 판정하지 않았습니다.
- WebKit Timelines에서 Memory를 활성화하고 단일 기록을 시작·중단했습니다. 종료 시점 총 메모리는 164.28MB(JavaScript 72.03MB, Page 92.25MB), 기록 중 최대는 233.11MB였습니다. 이것은 WebKit timeline 수치이며 앱 프로세스 RSS·Monaco 모델 수·쿼리 캐시 수와 동일한 지표가 아닙니다. 짧은 단일 기록으로 장시간 누수 부재를 주장하지 않습니다.
- 원격 설정은 `Stopped`에서 `Running on port 64926`, 연결 0개로 바뀌었습니다. 실제 loopback 요청 한 번씩에서 인증 없는 `/`은 HTTP 401, 잘못된 Host는 403, 잘못된 Origin은 403, 인증 없는 WebSocket upgrade는 401이었습니다. 기본 sandbox의 loopback 연결 거부를 확인한 뒤 권한 허용 `curl`로 같은 서버를 확인했습니다.
- `Create Access Link`는 비밀번호 없는 파일·터미널 제어 링크를 새로 만드는 보안상 별도 조치로 auto-review가 거절했습니다. 이 결정을 우회하지 않았으므로 유효 link·session cookie·인증 WebSocket·TTL·느린 수신자 포화는 실측하지 못했습니다. 원격 스위치를 다시 `off`로 돌렸고 화면에 `Stopped`가 표시됐습니다. `lsof -nP -iTCP:64926 -sTCP:LISTEN`은 출력 없이 exit 1이어서 해당 포트 listener 종료를 별도로 확인했습니다.

## 판정

테마·언어 대표 전환과 복원, WebKit Memory 단일 기록, 원격 서버의 무인증 거부 및 중지는 확인했습니다. M7-C1b-2, M7-C4b-2b, M7-C4c-2 전체는 아직 완료가 아닙니다. 이 기록 작성 당시 IDE 인증 성공·tool handler는 미검증이었으나, 이후 사용자 승인 범위에서 수행한 [IDE 단일 실측](2026-09-29-m7-ide-ws-authenticated-open-file.md)에서 확인했습니다.

이 기록 작성 당시 차단됐던 링크 생성은 이후 사용자 승인으로 수행됐습니다. [원격 인증 단일 실측](2026-09-29-m7-remote-authenticated-session.md)에서 실제 쿠키와 WebSocket 대표 명령을 확인했으며 TTL·포화와 기존 연결의 폐기 종료는 여전히 미판정입니다.
