# M8 source owner·root resource·media range 경계

## 대상과 상태

후속 상태: primitive 아래의 실제 HTTP transport·policy·ticket/owner 중간 폐기는 `2026-10-02-m8-native-web-http-transport.md`, prepared HTML/host/helper/CSS·cache 함수는 `2026-10-02-m8-native-web-served-document-cache.md`에서 검증했습니다. 아래 최초 primitive 기록과 이후 renderer/플랫폼 미완료를 구분합니다.

`native/taide-native-app/src/preview_web_{file,resource,range}.rs`, 기존 `preview_web.rs`의 source 소유자 연결·lib export·native manifest/lock과 `tests/preview-web-resource.rs`입니다. N5-P1h의 실제 filesystem/stream primitive를 구현했습니다. HTTP server·response header/CSP enforcement·NativeApplication/WebView/File surface에는 아직 연결하지 않았으며 HTML/Audio/Video provider 또는 전체 M8 완료로 처리하지 않습니다.

원본은 기존 `preview-kind.ts`·`html-preview-document.ts`·audio/video preview이며 roadmap §6.2의 root 제한·script/parent bridge 금지 경계를 유지합니다. 기존 성공은 재사용하며 사용자 실기 앱/bundle·제품 TS/root/MSRV·시스템 설정을 변경하지 않았습니다.

## source 소유 수명과 filesystem

1. Prepared HTML은 RAII Owner를 소유하고 resource 작업은 Scope clone만 보유합니다. Owner drop은 모든 clone의 live flag를 해제합니다. Scope는 source 원래 경로·project/root/canonical 승인, source inode/length/mtime/ctime과 root directory identity를 재검사합니다. helper 결과 뒤에도 owner.check를 호출하므로 준비 중 source 파일 identity/content stamp 변경은 결과를 버립니다. 이 stamp는 content hash가 아니며 저정밀 timestamp·승인 제거/복원 ABA·UI generation은 source invalidation/epoch의 후속 게이트입니다.
2. root와 정규 파일의 열린 descriptor를 사용합니다. canonical path의 root-relative normal component를 `openat`의 NOFOLLOW/CLOEXEC/DIRECTORY/NONBLOCK로 순서대로 열며 final regular file만 허용합니다. in-root symlink는 먼저 허용된 canonical target으로 해석하지만, 확인 뒤 descriptor walk의 component가 symlink로 바뀌면 따라가지 않습니다. root identity 교체도 거절합니다. FIFO/device는 regular-file 검사와 nonblocking open으로 거절합니다. authored unsafe/주석/suppression은 추가하지 않았습니다.
3. 표준 File::open만으로 intermediate symlink의 원자적 no-follow를 처리할 수 없어 이미 root/native lock에 있던 rustix 1.1.4의 fs feature를 직접 재사용했습니다. 공식 설치 source의 open/openat/OFlags와 고정 버전을 읽었으며 새 package/version은 추가하지 않았습니다. 최초 manifest edge 갱신만 offline unlocked로 실행했고 이후 locked/offline을 유지했습니다. root manifest/lock·MSRV는 이 경계에서 변경하지 않았습니다.
4. resource 요청은 자체 taide-preview/localhost URL만 받습니다. 표준 URL decode로 파일 경로를 얻고 query는 cache-busting 값으로만 취급합니다. 승인 source의 동일 root 안만 허용하며 다른 열린 프로젝트로 경계를 넓히지 않습니다. CLI source는 그 정확한 파일만 허용하고 sibling 권한을 만들지 않습니다. CSS/image/font/media extension MIME allowlist를 적용하고 HTML/script/unknown type는 resource로 열지 않습니다. 이는 content sniff/codec/renderer security 검사 완료가 아닙니다.

Unix anchor만 구현했습니다. non-Unix는 권한을 우회한 fallback File::open 대신 명시적으로 미구현 오류를 반환합니다. 현재 macOS 성공을 Windows/Linux 또는 전체 플랫폼 제품 gate로 확대하지 않습니다. Windows의 handle/reparse-point 경계를 구현·검증하기 전 해당 preview의 cutover는 불가합니다.

## range와 bounded stream

[RFC 9110 §14.1~14.4](https://www.rfc-editor.org/rfc/rfc9110.html#section-14.1.2)를 읽고 single closed/open/suffix range의 inclusive byte 경계·clipping·unsatisfiable metadata를 구현했습니다. decimal overflow는 포화 비교로 처리합니다. unknown unit·malformed/multiple range는 Range를 무시한 Full 선택이며 빈 파일도 Full입니다. HTTP adapter가 Full/Partial/Unsatisfiable을 200/206/416과 일치시키고 Content-Length/Content-Range를 실제 전송해야 합니다. 현재 wire 상태를 증명하지 않습니다.

media_owner는 bytes 전체를 읽거나 HTML helper를 실행하지 않고 기존 승인 worker 안에서 audio/video source의 descriptor·owner만 만듭니다. 이전 20MiB raw preview 제한을 media 파일 크기 제한으로 잘못 이식하지 않습니다. Body는 열린 파일에서 정확한 선택 range를 최대 64KiB씩 읽고, 각 chunk 전후 source owner/approval·asset canonical/identity/content stamp를 검사합니다. 끝보다 많이 읽거나 초과 bytes를 반환하지 않습니다. Body API는 blocking입니다. 실제 event loop에서 호출하지 않고 tracked worker/HTTP body backpressure에 연결해야 합니다.

logical chunk buffer 경계는 전체 process/allocator/kernel/renderer/GPU RSS·총 transport concurrency·HTTP queue/cancellation/CPU 상한을 증명하지 않습니다. 작은 fixture의 통과를 stream UI 성능으로 주장하지 않습니다.

## 실행 증거

Cargo 직렬, native manifest·`--locked --offline --target-dir experiments/native-shell-spike/target`을 사용했습니다.

- [x] 신규 `--lib web_range` 1 PASS, compile 2.47초·suite 0.00초입니다. closed/open/suffix/clipping/empty/unknown/malformed/multiple/oversized decimal/u64::MAX를 검사했습니다.
- [x] 실제 root/CLI/resource/RAII source/stream 신규 `--test preview-web-resource` 1 PASS(0.45초), 변경 read/host 검사 각 1 PASS(0.05/2.46초), 해당 전체 compile 12.19초입니다. prepared HTML의 encoded space/#/? CSS와 query·closed/suffix/unsatisfied metadata·다른 열린 root·unknown type·외부 scheme·외부 symlink 거절, owner drop 후 in-flight chunk 거절, 60MiB sparse media의 전체 정확한 길이·HEAD/TAIL·각 Vec의 64KiB 이하, seek tail·root 제거·CLI exact/sibling·source 변경을 확인했습니다. read/host는 새 RAII owner/anchor/source stamp 배선의 영향을 검증한 1회이며 변경 없는 helper/client/HWP는 반복하지 않았습니다. sparse synthetic media는 실제 codec 파일/재생 검사가 아닙니다.
- [x] `--lib web_anchor` 1 PASS, compile 3.29초·suite 0.00초입니다. descriptor 캡처 뒤 intermediate directory→외부 symlink 교체, FIFO, root→다른 directory symlink 교체를 검사했습니다. 최초 compile E0425는 rustix의 mkfifoat가 Apple cfg에서 제공되지 않는 것을 확인해 테스트 fixture 생성만 확인된 `/usr/bin/mkfifo`로 교정했습니다. production openat/NONBLOCK 검사는 그대로이며 실패 fixture만 1회 실행했습니다.
- [x] 최종 app lib/bin/모든 test target strict `-D warnings` exit 0(6.41초), 대상 rustfmt check·git diff check exit 0입니다. 선행 offline check exit 0(5.38초)입니다.

## 다음 실제 연결

- [x] 기존 Hyper 고정 source API를 읽고 실제 loopback capability/owner streaming·GET/HEAD/status/header/강제 policy·root shutdown/body cancel/연결 상한을 web-http QA에서 검증했습니다. Wry custom protocol의 전체 media Vec 경로는 만들지 않았고 실제 renderer 보안·성능 gate는 미완료입니다.
- [ ] helper prepared document의 server URL/base mapping과 NativeApplication helper/bridge/cache/generation/source invalidation/close를 실제 조립합니다. secret capability를 로그·다른 window/source로 유출하지 않습니다.
- [ ] 별도 WebView의 JS disabled/no IPC·navigation/new-window/permission·bounds/focus/hide/close와 원본 HTML/Audio/Video controls·theme/locale·codec·pixels를 구현·검증합니다.
- [ ] Windows handle boundary·실제 corpus/concurrency/장기 transfer/crash/isolation·전체 DOM/renderer RSS/CPU·OS/GPU/AX와 M8 상위 필수 gate를 마무리합니다.
