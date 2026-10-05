# M8 HTML NativeApplication·WebView 배선

## 구현 상태

`application.rs`, `main.rs`, `preview_web_{host,cache,view}.rs`, native manifest/lock, `vendor/wry-preview`와 신규 `tests/preview-web-lazy-host.rs`입니다. 실제 native File/HTML surface에서 trusted helper·독립 lazy host·cache와 macOS child WebView를 조립했습니다. 후속 Audio/Video 기본 배선은 `2026-10-02-m8-native-audio-video-webview.md`에 기록했습니다. media 상태 동등성, 다른 플랫폼 renderer와 실제 GUI/OS 보안·pixel·입력·성능은 미완료입니다. N5-P1h·M8 완료로 처리하지 않습니다.

사용자가 요청한 기존 TypeScript 화면·동작 재현을 진행하며 제품 TS를 제거하지 않았습니다. 사용자 실기 `TAIDE M8 Egui Spike.app` bundle을 수정·빌드·재시작하지 않았고 실제 앱/VoiceOver/입력기/OS clipboard·파일 선택 창/카메라·마이크를 실행하지 않았습니다. 메인이 workflow·서브에이전트 없이 수행했습니다.

## 앱 조립·수명

1. main은 helper flag 처리를 GUI/data bootstrap보다 먼저 유지하고 일반 앱 시작 때만 trusted current executable을 NativeApplication에 명시 주입합니다. library가 임의 실행 파일을 추측하지 않습니다. 독립 bridge는 첫 유효 HTML 요청 때 worker에서 listener를 생성합니다. 대기·취소 queue는 listener/helper를 만들지 않습니다. 앱 기본값은 source 256, 연결 16, header 5초, idle/helper 30초의 named constant이며 큰 media 전체 길이 제한으로 바꾸지 않았습니다.
2. actual background poll이 SourceReady/Prepared와 cache token을 연결합니다. 전체/path/project/root invalidation·마지막 file tab 종료·preview mode 전환/rename/project 제거의 reconcile에서 active token까지 cancel watch를 전진시킵니다. helper read future drop은 검증된 client의 kill/wait 경로를 사용합니다. 큐에 남은 이전 token도 읽지 않습니다. stale reply는 현재 cache를 변경하지 않습니다. 실제 in-flight child PID의 취소/reap는 선행 client 증거를 재사용했으며 이번 bridge 검사는 결정적인 queued cancellation입니다.
3. HTML admission에서 다른 다섯 provider 비용을 합산하고, 다섯 provider의 기존 admission에서도 HTML 비용을 포함합니다. 이는 기존 cache의 논리적 retained 기준이며 source graph/준비 peak/분리된 HTTP slice/OS/renderer 전체 RSS를 엄밀히 합한 단일 메모리 상한 완료 주장이 아닙니다.
4. close/on_exit는 cache owner/ticket·WebView를 폐기하고 web worker도 기존 shutdown join에 포함합니다. 종료 저장 실패로 앱을 재개하면 새 bridge를 연결합니다. bridge join과 client nonabortable child drain은 별개이며 최종 ExitDrain/TaskSupervisor가 실제 작업을 회수합니다. 실제 NativeApplication 종료 GUI 실측은 아직 하지 않았습니다.

## renderer 코드 경계

macOS Wry 0.55.1 설치 source의 child builder, Frame의 HasWindowHandle, dpi Rect, visibility/focus/parent focus/background, Darwin process termination/link preview API와 egui popup/drag API를 확인했습니다. current File HTML surface가 clipped bounds·source URL·theme background·focused pane을 전달합니다.

- JavaScript disabled, incognito, devtools/clipboard/autofill 요청 비활성, IPC/custom protocol/initialization script 등록 없음입니다. macOS clipboard 옵션은 upstream에서 미지원이므로 그 옵션 하나로 clipboard 권한을 차단했다고 주장하지 않습니다.
- navigation은 동일 capability document의 fragment만 허용하고 다른 path/query/origin/scheme/credential은 거절합니다. 새 창·download·link preview·back/forward gesture를 막습니다. 실제 resource 전송은 선행 server 강제 CSP/no-referrer/MIME/root boundary를 재사용합니다.
- 활성 tab/source·양수 finite clipped bounds가 일치할 때만 만들고 표시합니다. 닫힌/비활성 tab과 오래된 source의 child는 drop해 숨은 tab마다 WebView를 무한 보관하지 않습니다. popup/drag/dialog/disabled 상태는 활성 child를 숨기고 필요 시 parent focus를 돌려줍니다. 실제 Cocoa/egui focus·단축키·overlay·DPI/pixel 정합성은 실기 게이트입니다.
- theme background 변경과 web content process 종료 콜백을 연결했습니다. 종료/renderer 오류는 해당 HTML cache의 owner/ticket을 폐기하고 실패 상태로 보입니다. 실제 crash 복구/scroll/state parity는 아직 검증하지 않았습니다.

Windows/Linux는 보안 경계를 우회한 renderer fallback 대신 명시적으로 미구현 오류입니다. Linux Wry는 GTK 초기화·event loop 요구가 있어 macOS 코드를 그대로 완료로 확대하지 않았습니다. Windows anchor·renderer 연결은 cutover 전에 구현해야 합니다.

## Wry native 권한 수정

실제 upstream WKUIDelegate는 file upload 요청에 NSOpenPanel을 열고 camera/microphone capture decision을 Grant로 반환했습니다. native preview의 권한 없는 renderer 계약을 위해 설치된 정확한 Wry 0.55.1 source/license를 기계적으로 복사하고 기본 비활성 `native-preview-deny-permissions` feature를 추가했습니다. native macOS dependency에서만 켜고 기존 Tauri/root registry dependency는 유지합니다.

feature가 켜지면 file upload completion을 nil로 끝내 panel을 열지 않고 capture decision은 Deny입니다. 원래 delegate/unsafe 영역을 재사용하며 새 unsafe 영역·IPC·JS·private API·suppression을 추가하지 않았습니다. upstream 일반 기본 동작은 feature가 꺼졌을 때 유지합니다. provenance와 원본 checksum은 `vendor/wry-preview/UPSTREAM.md`에 기록했습니다. 실제 OS permission/prompt 또는 WebKit 모든 권한의 실측 증거는 아닙니다.

처음 native lock에 없던 Wry/OS transitive package 84개가 추가됐습니다. Wry는 root의 기존 0.55.1을 재사용하지만 모든 native transitive version이 root와 같다고 주장하지 않습니다. 예를 들어 Android 전용 crossbeam-channel은 native 0.5.17/root 0.5.16이며 native에 먼저 존재한 다른 UI/wasm graph도 root와 다릅니다. root manifest/lock·MSRV는 이 경계에서 변경하지 않았고 offline compatible resolution 뒤 locked/offline을 유지했습니다.

## 실행 증거

Cargo 직렬, native manifest·`--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 최초 registry/path edge resolution만 unlocked offline check입니다.

- [x] registry Wry source/API 연결 check exit 0(11.93초), actual app lib/bin check exit 0(0.90초), vendor feature check exit 0(2.58초)입니다. `cargo tree -e features -i wry`에서 실제 native deny feature 활성화를 확인했습니다.
- [x] 신규 `--lib web_view_navigation` 1 PASS, compile 15.42초/suite 0.00초입니다. 동일 document/fragment와 외부/file/data/javascript/credential/다른 port·capability/query 거절을 검사했습니다. 실제 WebView를 만들지 않은 pure navigation 정책 검사입니다.
- [x] 신규 `--test preview-web-lazy-host` 1 PASS, compile 7.13초/suite 2.54초입니다. listener 없는 bridge 대기→cancelled queue도 listener 없이 거절→같은 host에서 실제 helper/SourceReady/cache→성공 cache 재읽기 없음→renderer failure의 owner/ticket 폐기/실제 HTTP 404→host join/root shutdown/작업 0/port 회수를 연속 검사했습니다. synthetic loopback만 승인 실행했습니다. 최초 E0282는 새 첫 Request field 접근의 channel 타입 추론 실패이며 `mpsc::channel::<Request>`로 경계 타입을 명시했습니다. 실패한 compile은 test body를 실행하지 않았습니다.
- [x] 신규 `--lib web_view_placement` 1 PASS, compile 1.84초/suite 0.00초입니다. actual renderer의 공용 eligibility 함수에서 닫힌 tab·오래된 source·empty/NaN/infinite bounds를 거절합니다. OS bounds/focus 성공 주장이 아닙니다.
- [x] 최종 app lib/bin/모든 test target strict `-D warnings` exit 0(5.47초), 대상 rustfmt check·git diff check exit 0입니다. 중간 strict의 dispatch 8 인자 경고는 Preparation 타입으로 모아 수정했고 test 미사용 import도 제거했습니다. 동일 8 인자 상태를 한 번 불필요하게 다시 확인한 정적 실패는 새 성공 증거로 계산하지 않습니다. 실제 통과한 lazy runtime body·navigation과 선행 HTTP/helper/client/cache/HWP 검사는 반복하지 않았습니다.

vendored upstream Wry의 기존 deprecated/unused-unsafe 경고 17개는 남습니다. 검사기를 끄거나 무관한 vendor 코드를 재작성하지 않았습니다. 위 strict 성공은 선택한 taide-native-app target의 `-D warnings`이며 전체 vendor graph가 warning-free라는 뜻이 아닙니다. 실제 Cocoa/OS permission/보안·성능·배포 승인 전에 upstream 경고/호환 영향을 대조해야 합니다.

## 다음 검증·구현

- [x] Audio/Video의 승인된 대형 media URL과 별도 wrapper document·controls/file name/간격/크기·theme 기본 코드를 같은 host/cache/renderer에 연결했습니다. 후속 QA의 theme playback·codec/GUI/플랫폼·전체 상태 동등성 게이트는 남습니다.
- [ ] provider 코드 연결 뒤 보호된 사용자 bundle과 다른 합성 앱 한 세션에서 실제 HTML/CSS/차단/파일 선택 거절·media/codec·DPI/bounds/focus/overlay/source 변경·종료를 연속 검사합니다. 코드 우선/사용자 담당 IME·VoiceOver 최후 순위는 유지합니다.
- [ ] Windows/Linux 구현·OS/crash/장기 session·DOM/helper CPU/RSS·총 renderer/allocator/GPU budget과 N1~N8 전체 필수 gate입니다.
