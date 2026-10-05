# M8 prepared HTML HTTP 제공·cache 세대 경계

## 범위와 상태

후속 코드 배선: actual NativeApplication/lazy host와 macOS child WebView는 `2026-10-02-m8-native-html-application-webview.md`에서 연결했습니다. 아래 독립 함수의 최초 기록과 실제 GUI/OS·Audio/Video 미완료를 구분합니다.

`preview_web_{document,client,http,published,host,cache}.rs`와 `preview_web.rs`, 신규 `tests/preview-web-served.rs`입니다. 실제 승인 read→독립 helper→capability HTTP document/CSS와 source invalidation을 연결했습니다. Cache의 stale generation·공유 path·owner 폐기도 구현했습니다. NativeApplication의 실제 배선·WebView bounds/focus/close·Audio/Video 화면에는 아직 연결하지 않았습니다. N5-P1h·M8 완료로 처리하지 않습니다.

원본 TS HTML DOM/base/CSP·iframe script 차단은 유지하며 부모에서 DOM을 다시 parse하거나 HTML 문자열의 URL을 단순 치환하지 않습니다. 사용자 실기 bundle·제품 TS/root manifest/lock·MSRV·시스템 설정을 변경하지 않았습니다. 서브에이전트·workflow는 사용하지 않았습니다.

## 문서 제공

1. trusted native executable을 주입하는 read_served/connect_served 경로가 source owner의 scope로 ticket을 등록합니다. helper가 실제 loopback capability source URL을 받아 기존 첫 base href를 그 기준으로 해석합니다. 리소스용 validator는 자체 taide-preview/localhost만 계속 허용하고, 별도 document validator만 credential/query/fragment 없는 HTTP literal 127.0.0.1·명시된 양의 port·capability path를 허용합니다. 이 URL 검사는 registry 인증을 대체하지 않으며 helper는 filesystem/network API를 사용하지 않습니다.
2. helper 완료 뒤 실제 read worker에서 project/root/canonical·source owner/stamp를 재검사하고 같은 ticket에 문서를 한 번 게시합니다. source의 HTML 경로만 prepared document로 제공하며 다른 .html은 resource allowlist에서 거절합니다. GET/HEAD/Range와 서버 강제 policy는 기존 transport를 재사용합니다. 문서 전송도 각 최대 64KiB frame 전에 tracked blocking worker에서 source를 확인합니다. body 실패는 파일 경로가 없는 정적 오류로 바꿉니다.
3. Prepared HTML은 Arc<String> 하나를 공유합니다. 공식 설치 bytes 1.12.1 source의 from_owner/slice 계약을 읽고 Bytes owner에 같은 Arc와 RAII budget lease를 넣었습니다. 게시 document는 본문 전체를 복사하지 않으며 전송 slice가 마지막으로 해제될 때 예산을 반환합니다. 64MiB 개별 출력/용량과 128MiB 서버 게시 예산을 검사합니다. 문자열 capacity·path capacity·명시한 inline 구조 비용의 논리적 retained 기준이며 allocator/Arc 내부 bookkeeping·source graph·DOM parse peak·준비 중 출력·OS/renderer RSS의 엄밀한 상한이 아닙니다.

## cache 수명

Cache는 같은 path의 여러 탭이 하나의 Prepared/Owner/ticket을 공유하고 마지막 path 제거·root/file/all invalidation에서 즉시 폐기합니다. 이전 generation의 SourceReady/cancel/result는 현재 요청을 바꾸지 않으며 반환된 stale Prepared는 폐기합니다. 토큰은 checked 증가이고 기존 host queue 상한은 유지합니다. 성공 후 동일 path begin은 다시 읽지 않습니다.

HTML capacity·canonical capacity·URL 길이·명시한 inline 비용을 합산해 기존 128MiB 공유 cache 기준에서 other_bytes와 비교합니다. budget 실패/reader 실패의 Decode/Read 상태를 구분하며 거절된 Owner도 닫힙니다. 실제 NativeApplication의 다른 provider 합산·reconcile/invalidation·renderer 적용은 다음 배선 게이트입니다. 독립 cache 함수의 성공을 모든 native provider의 총 RSS나 실제 UI source epoch 완료로 확대하지 않습니다.

## 실행 증거

Cargo 직렬, native manifest·`--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 변경 없는 선행 HTTP/helper/client/host/HWP 성공은 재사용했습니다.

- [x] 신규 `--lib web_served_document` 2 PASS, compile 2.38초/suite 0.00초입니다. 실제 helper DOM의 loopback relative base와 외부 host/localhost/port 부재·0/credential/capability 부재/query/fragment 거절, zero-copy 주소 동일성과 slice가 남은 동안 budget 유지·마지막 drop 반환·거절 뒤 0을 확인했습니다.
- [x] 신규 실제 `--test preview-web-served` 1 PASS, compile 1.15초/suite 2.29초입니다. host SourceReady 순서→실제 별도 helper→encoded space/#/? source→HTTP HTML 본문 일치/CSP→첫 relative base의 CSS query 제공→HTML HEAD/206→다른 HTML 403→source 수정 403→문서 drop 404→host join/server shutdown/작업 0을 연속 검사했습니다. 최초 E0599는 테스트 fixture가 반환값 없는 TaskSupervisor::shutdown에 unwrap을 쓴 컴파일 오류이며 본문을 실행하지 못했습니다. fixture 반환형만 교정한 해당 검사 1회가 통과했습니다. synthetic loopback socket만 승인 실행했고 앱/renderer/외부 network를 실행하지 않았습니다.
- [x] 신규 `--lib web_cache` 1 PASS, compile 2.00초/suite 0.00초입니다. invalidation 뒤 stale reply·현재 요청 보존·같은 path의 한 탭만 닫기·root 폐기와 owner 종료·공유 budget 거절/owner 종료·cancel 재시도·Read/Decode 분류·최종 reconcile을 실제 합성 filesystem source로 검사했습니다.
- [x] 최종 lib/bin/모든 test target strict `-D warnings` exit 0(5.12초), 대상 rustfmt check·git diff check exit 0입니다. 이 사이 served 배선 strict 4.81초와 cache 전 strict 5.47초는 서로 다른 코드 상태의 근거이며 최종 동일 명령을 반복하지 않았습니다.

## 남은 게이트

- [ ] NativeApplication trusted helper 주입·별도 host/server/cache 조립, 다른 provider budget 합산, 실제 source generation/close/rename/root 변경 배선입니다.
- [ ] 제한된 별도 WebView의 navigation/new-window/download/permission/JS/IPC·bounds/focus/팝업·숨김/종료, HTML/Audio/Video 원본 화면·theme/locale·codec입니다.
- [ ] Windows anchor·ABA/저정밀 stamp·DOM/renderer RSS/CPU·실제 corpus/장기 session/OS/GPU/AX·전체 M8 필수 게이트입니다.
