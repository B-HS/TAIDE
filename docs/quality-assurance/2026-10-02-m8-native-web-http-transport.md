# M8 loopback HTTP preview transport

## 구현 범위

`preview_web_http.rs`, `preview_web_io.rs`, source owner의 종료 watch와 실제 HTTP 검사 두 파일입니다. 기존 resource primitive를 loopback 127.0.0.1의 임의 port에 연결하고 UUID capability ticket으로 승인 source를 제한합니다. HTML prepared document 제공·실제 NativeApplication/WebView·media codec 재생은 아직 미연결입니다. 상위 N5/M8 완료로 처리하지 않습니다.

설치된 Hyper 1.11.0 HTTP/1 server builder, hyper-util 0.1.20 TokioIo/Timer, http-body-util 0.1.4 StreamBody, futures-util 0.3.33 unfold와 Tokio 1.53.1 I/O/watch source를 읽었습니다. bytes 1.12.1/http 1.5.0을 포함해 기존 root lock 버전을 재사용합니다. native lock에는 기존 root와 동일한 httpdate 1.0.3 한 package만 추가됐고 root manifest/lock·MSRV는 변경하지 않았습니다.

## 수명·전송 경계

- registry source 수와 연결 수는 주입한 Limits로 제한합니다. HTTP header는 32KiB/64개, GET/HEAD만 허용하며 Host/absolute URI authority를 listener의 정확한 주소와 대조합니다. request body·Transfer-Encoding은 거절합니다.
- 승인 root/MIME/range primitive로 200/206/416·Content-Length/Content-Range/Accept-Ranges를 제공합니다. HEAD는 Range를 무시하고 본문 없이 전체 길이를 반환합니다. If-Range가 있으면 validator가 없으므로 Range를 무시합니다.
- 실제 응답에 강제 CSP, nosniff, no-store, no-referrer, same-origin CORP, 장치/clipboard 금지 Permissions-Policy를 붙이며 CORS 권한은 열지 않습니다. 정책 header만으로 renderer 보안 완료를 주장하지 않습니다.
- media는 tracked blocking worker에서 최대 64KiB씩 읽습니다. HTTP backpressure 아래 한 chunk씩 요청하며 전체 파일 Vec는 만들지 않습니다. ticket/owner 폐기는 현재 socket을 끊습니다. 연결 permit과 operation lease는 취소 뒤 남은 blocking worker도 소유하므로 실제 작업 종료 전 슬롯을 재사용하지 않습니다.
- header deadline과 실제 read/write 진행 기반 idle deadline을 사용합니다. 큰 정상 전송의 전체 소요시간·파일 크기를 제한하는 정책으로 바꾸지 않았습니다. server 정상 shutdown은 연결 task와 남은 blocking lease를 drain하며 root TaskSupervisor shutdown은 listener/connection도 회수합니다.

이는 논리적 chunk/동시 연결 경계이며 allocator/kernel/renderer/GPU RSS·총 CPU·실제 느린 receiver/장기 session의 성능 상한은 아닙니다. sparse synthetic MP4는 codec 파일이 아닙니다. capability·파일 경로·토큰을 로그에 출력하지 않았습니다.

## 실행 증거

Cargo 직렬, native manifest와 `--offline --target-dir experiments/native-shell-spike/target`을 사용했습니다. 첫 dependency edge check만 unlocked이며 이후 `--locked`입니다.

- 초기 check exit 0(6.62초). 처음 실제 socket 검사는 기본 sandbox의 bind 거절로 본문 검사를 실행하지 못했습니다. 오류 종류만 드러내는 진단을 추가하고 승인된 synthetic loopback 실행으로 `--test preview-web-http` 1 PASS, compile 1.90초/suite 0.01초입니다. GET/HEAD/200/206/416·정확한 본문/정책·POST/body 거절·다른 root/unknown ticket·ticket/owner 종료·registry/connection 상한·parked connection 종료·listener port 회수/작업 0을 확인했습니다.
- 새 위험만 검사한 `--test preview-web-http-lifetime` 1 PASS, compile 3.18초/suite 0.02초입니다. 1GiB sparse source의 실제 HTTP 전송 중 ticket/owner를 각각 폐기해 전체 길이 전에 EOF/reset이 발생했습니다. idle 0 연결 종료·정상 server drain·root shutdown/작업 0/port 회수를 확인했습니다. reader buffer는 64KiB이며 1GiB를 메모리에 올리지 않았습니다.
- generic I/O refactor의 compile E0403은 중복 타입 매개변수 이름을 수정했습니다. 새 `--lib web_idle_io` 1 PASS, compile 3.34초/suite 0.00초이며 duplex의 정지 read와 꽉 찬 write가 TimedOut으로 끝납니다. 기존 성공 HTTP/owner/helper 검사를 반복하지 않았습니다.
- 최종 lib/bin/모든 test target strict `-D warnings` exit 0(7.00초)입니다. 사용자 실기 bundle·시스템 설정·실제 앱은 조작하지 않았습니다.

## 다음 경계

- [x] helper capability HTTP source/base·prepared document 게시·retained budget·독립 cache 함수는 [후속 실제 검사](2026-10-02-m8-native-web-served-document-cache.md)로 연결했습니다. NativeApplication 배선은 별도 미완료입니다.
- [ ] NativeApplication의 별도 host/cache/generation/source invalidation과 제한된 별도 WebView bounds/focus/close를 연결합니다.
- [ ] HTML/Audio/Video 원본 화면·codec·Windows anchor·실제 renderer 보안/장기 전송/RSS/CPU/OS/AX 및 상위 M8 gate를 검증합니다.
