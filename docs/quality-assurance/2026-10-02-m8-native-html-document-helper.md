# M8 HTML prepared document와 독립 프로세스 helper

## 대상과 현재 상태

`native/taide-native-app/src/preview_web_{document,helper,client}.rs`, lib export·main의 helper flag 분기와 helper/client 검사입니다. N5-P1h의 준비·독립 실행·tracked client를 구현했습니다. 실제 native File surface·WebView·resource/range·approved host는 아직 연결하지 않았으며 HTML/Audio/Video provider 또는 전체 M8 완료로 처리하지 않습니다.

원본은 `src/features/preview/html-preview.tsx`, `src/shared/lib/html-preview-document.ts`, `src/widgets/preview-pane/preview-pane.tsx`입니다. 원본 DOMParser/TextDecoder의 HTML 문서·UTF8 BOM 제거/오류 문자 대체·첫 base[href] 결정·기존 base 제거·head 첫 CSP→base·doctype→html outer serialization·상대 resource 경로를 Rust로 재현합니다. 원본은 script/object/frame/form을 금지하며 iframe sandbox에 scripts를 허용하지 않습니다. 실제 renderer에도 별도 강제 policy·JS disabled·parent bridge 없음·root 제한 resource가 필요합니다.

## 구현과 경계

1. 기존 root Cargo.lock의 dom_query 0.27.0(MIT)·html5ever 0.38.0(MIT OR Apache-2.0)만 native manifest의 직접 edge로 재사용했습니다. HTML5 parsing/DOM 변환/escaping을 수동 정규식으로 대체하지 않습니다. dom_query default markdown feature는 끕니다. 추가된 19개 package/version을 root lock과 실제 대조해 불일치 0을 확인했습니다. root manifest/lock·MSRV·제품 TS·사용자 bundle은 이 단위에서 변경하지 않았습니다.
2. source_url은 absolute path를 표준 file URL의 encoding으로 `taide-preview://localhost`에 옮깁니다. 이 함수는 파일을 열거나 인가하지 않습니다. prepare_html은 지정 scheme/host·credentials/port를 확인하고 source.join으로 base를 계산합니다. TextDecoder에 대응하는 기존 encoding_rs UTF8 decode API를 사용합니다. 원본 HTML/script를 지운 척하지 않으며 실행 차단은 원본 meta와 후속 renderer의 강제 정책으로 처리해야 합니다.
3. 입력은 기존 20MiB file 정책, 출력은 실제 write 전에 64MiB로 제한하고 allocation 실패를 오류로 반환합니다. 설치된 dom_query serializer source의 명시적 Open/Close work stack을 확인해 recursive serializer를 새로 만들지 않았습니다. 이 예산은 DOM 생성/parse-error vector/allocator/전체 peak RSS·CPU 상한이 아닙니다. 현재 준비 함수는 실제 UI worker에 직접 호출하지 않습니다.
4. `--html-preview-helper`는 main의 LaunchConfig/restore/Tokio/eframe 초기화 전에 진입합니다. stdio만 사용하며 데이터 디렉터리·파일·OS 설정·GUI를 열지 않습니다. THP1 request/THT1 reply와 little-endian u32 길이를 사용합니다. source 4096B·document 20MiB 길이는 payload 할당 전에 검사하고 trailing data를 거절합니다. output은 bounded UTF8 document이며 초과·truncation·잘못된 URL/추가 argument는 오류입니다. default `--data-dir` 시작 경로는 유지했습니다.

독립 process는 DOM parser의 앱 내 직접 실행을 피할 수 있는 기반이지 OS sandbox/메모리·CPU 한계 또는 실제 host의 child 회수/취소가 완료됐다는 의미가 아닙니다. 웹 renderer는 roadmap의 권한 없는 별도 WebView/helper 경계를 유지해야 합니다. 기존 Wry 0.55.1 source의 child view·JavaScript disable·custom protocol API를 읽었으나 이 단계에서 새 Wry dependency나 runtime WebView를 추가하지 않았습니다.

## 실행 증거

Cargo는 직렬, 공통 인자는 native app manifest와 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. 기존 root dependency edge를 native lock에 처음 넣을 때만 locked 없이 실행했습니다.

- [x] 신규 `--lib html_document` 1 PASS, compile 27.11초·suite 0.00초입니다. BOM/잘못된 UTF8 대체·duplicate/target base 제거·first href의 quote encoding·상대 CSS/img·CSP 앞 순서·inert script 보존·fragment 문서·잘못된 source·출력 초과 전 무할당·input cap을 검사했습니다. app lib/모든 test target strict exit 0(18.59초)입니다.
- [x] 신규 실제 `--test preview-web-helper` 1 PASS, compile 6.24초·suite 2.45초입니다. 정확한 native test binary를 helper mode로만 실행해 실제 magic/길이/UTF8/EOF·상대 asset 결과·source/document oversized header의 payload 전 거절·추가 argument 거절·각 child wait 회수를 확인했습니다. 최초 compile E0308은 IPC u32 source length와 usize constant 비교를 찾아 source 상한 타입을 u32로 일치시켰습니다. 성공한 준비 검사와 HWP 검사 등은 반복하지 않았습니다.
- [x] 최종 lib/bin/해당 helper test strict `-D warnings` exit 0(0.88초), 대상 rustfmt check·git diff check exit 0입니다. helper 검사는 실제 native AppServices 복원/창을 띄우는 검사가 아니며 사용자 실기 앱/프로세스/bundle을 조작하지 않았습니다.

## tracked client 후속 결과

`preview_web_client::prepare`는 trusted absolute executable을 주입받고 source URL·4096B/20MiB를 spawn 전에 검사합니다. 기존 Tokio 1.53.1의 process/io-util/macros edge만 활성화했습니다. 설치된 공식 source의 Child drop·kill_on_drop·start_kill·wait 계약을 읽고, TaskSupervisor의 nonabortable operation 안에서 request 취소·deadline·오류의 kill 후 wait를 수행합니다. helper reply는 magic·64MiB length를 할당 전에 검사하고 UTF8·EOF·trailer를 확인합니다. caller 종료는 oneshot drop으로 전달하며 spawn 직전 취소도 검사합니다.

- [x] `--test preview-web-client`의 실제 success/deadline/caller abort/root shutdown/closed admission 1 PASS입니다. 최초 2.48초 통과 뒤 OS signal 0의 실패가 EPERM이 아니라 `No such process`인지 확인하도록 보강하고 spawn 전 취소 가드를 추가했습니다. 변경된 검사만 1회 실행해 compile 1.98초·suite 2.48초에 통과했습니다. 합성 1MiB payload와 직접 생성한 정확한 native helper PID만 사용하며 root shutdown 뒤 tracked count 0을 확인합니다. 보호된 사용자 실기 앱은 실행·종료하지 않았습니다.
- [x] 신규 `--lib html_reply` malformed magic/oversized header/truncated payload/invalid UTF8/trailer 1 PASS, compile 1.60초·suite 0.00초입니다. 최종 lib/bin/모든 test target strict `-D warnings` exit 0(4.10초), 대상 rustfmt·diff check exit 0입니다. 기존 HTML 준비/HWP 성공은 재사용했습니다.

이 단위는 정상 Rust cancellation/error 경로의 child 회수를 증명합니다. trusted on_started callback의 panic/장시간 blocking·runtime teardown·OS crash에는 kill_on_drop fallback만 있으며 모든 비정상 환경의 strict reap 또는 OS sandbox/RSS/CPU 상한을 증명하지 않습니다. actual composition root/host owner와 resource 요청은 다음 연결입니다.

## 다음 연결

- [x] tracked child의 deadline·취소·exit drain·bounded reply decoding.
- [x] 실제 승인 읽기와 별도 host 기본 연결 — `2026-10-02-m8-native-html-approved-host.md`의 read/host 검사·disconnect/root drain 증거입니다. NativeApplication 배선·document owner/close/generation은 미완료입니다.
- [ ] root/CLI 재인가·canonical/epoch 변경 거절, 문서와 resource/range 응답·강제 CSP, navigation/new-window/permission/bridge 차단.
- [ ] 별도 native renderer의 실제 File bounds·focus·hidden/close/invalidation·Audio/Video controls·locale/theme·필요한 codec.
- [ ] 실제 전체 DOM/renderer RSS·CPU/sandbox/crash isolation·malformed/corpus·원본 pixels/OS/GPU/AX·전체 M8 gate.
