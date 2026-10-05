# M8 HTML 승인 읽기와 독립 host

## 대상과 현재 상태

`native/taide-native-app/src/preview_web.rs`, `preview_web_host.rs`, lib export와 `tests/preview-web-{read,host}.rs`입니다. N5-P1h 안의 AppServices 기반 승인 읽기·별도 host 코드만 연결했습니다. NativeApplication의 composition root/File surface/cache/renderer에는 아직 배선하지 않았으며 전체 HTML/Audio/Video/N5/M8 완료로 계산하지 않습니다. 선행 helper/client 증거는 `2026-10-02-m8-native-html-document-helper.md`입니다.

## 구현

1. 기존 `preview::read_approved`·root_guard·20MiB 정책을 사용합니다. absolute/canonical·open project 또는 정확한 CLI 파일 승인 아래 blocking 읽기를 수행하고, helper로 복사된 bytes만 전달합니다. 최초 read_raw 연결 뒤 stat/read 사이 성장 위험을 확인해 열린 regular File의 크기와 실제 누적 읽기를 모두 제한했습니다. 원본 승인 identity인 project id·canonical root·canonical source를 기록하고 helper 결과 뒤 다시 기존 승인 worker로 검사합니다. 프로젝트/CLI 철회·owning project/root/canonical 변경·shutdown은 결과를 거절합니다. 긴 DOM 처리 동안 공용 mutation lock을 유지하지 않습니다.
2. source_ready callback은 최초 승인 읽기의 post-check 뒤 호출합니다. 결과는 canonical/source URL/준비된 HTML입니다. request token은 응답 식별 계약이며 아직 UI cache/epoch 검증에 연결하지 않았습니다. 파일 내용 변경·tab close·동일 identity의 제거/복원 ABA는 renderer owner/generation과 source invalidation에서 처리해야 합니다. 이 단계에서 모든 stale/epoch 경계를 통과했다고 주장하지 않습니다.
3. 기존 범용 HostBridge를 변경하지 않고 HTML 전용 tracked transient host를 만들었습니다. 명령 큐는 기존 64개 상한, reply는 1개 상한이며 한 host에서 하나의 document만 처리합니다. 큰 HTML을 기존 64개 reply queue에 누적시키지 않고 파일 저장/탐색 명령의 dispatch를 DOM helper 대기와 분리합니다. trusted absolute executable·caller deadline을 명시적으로 주입합니다. project 자료·사용자 입력을 executable 선택이나 shell로 연결하지 않습니다.
4. progress는 oneshot으로 전달하고 bounded sender로 비동기 전송합니다. async context에서 blocking_send를 호출하지 않습니다. read가 같은 poll에서 먼저 완료돼도 oneshot의 이미 전달된 progress를 확인해 SourceReady→Prepared 순서를 유지합니다. receiver disconnect는 select의 우선 분기로 read future를 drop하고 선행 client의 cancel→kill/wait cleanup을 작동시킵니다. worker join과 TaskSupervisor shutdown/drain을 구분합니다. disconnect의 JoinHandle 완료만으로 detached nonabortable helper 회수 완료를 주장하지 않습니다.

## 실행 결과

Cargo는 직렬이며 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`을 공통 사용했습니다.

- [x] 신규 `--test preview-web-read` 1 PASS, compile 4.17초·suite 2.46초입니다. 실제 native helper/합성 프로젝트의 root·CLI 성공, 미승인 파일의 읽기/실행 전 거절, oversized sparse file의 읽기 전 거절, helper 시작 뒤 CLI 철회·symlink 외부 retarget·프로젝트 제거·shutdown의 결과 거절과 tracked count 0입니다. 각기 다른 입력만 검사했고 선행 helper/client/HWP 성공은 반복하지 않았습니다.
- [x] 신규 `--test preview-web-host` 1 PASS, compile 4.25초·suite 2.47초입니다. 실제 AppServices+helper의 progress/result/token 순서·외부 파일 거절·relative executable 거절·source_ready 뒤 disconnect·queued 요청과 전체 operation drain 0을 확인했습니다. 빠른 result와 progress의 같은-poll 경로를 코드에서 보강한 뒤 변경 host 검사만 1회 실행해 compile 3.66초·suite 2.45초에 통과했습니다. source_ready 시점은 실제 child PID가 보장된 시점이 아니므로 이 검사만으로 fork 이후 회수를 주장하지 않습니다. 정확한 실제 PID 이후 회수 증거는 선행 client QA에 있습니다.
- [x] read/host library를 포함한 당시 lib/bin/모든 test strict exit 0(4.81초), 최종 변경 host와 lib/bin strict `-D warnings` exit 0(0.94초)입니다. 대상 rustfmt check·git diff check exit 0입니다. 새 dependency/version·제품 TS/root/MSRV 변경·사용자 앱 실행/재시작/종료·OS 설정 조작은 없습니다.

## 남은 경계

실제 입력 후속: `--lib html_bounded_read` 1 PASS(compile 2.23초·suite 0.01초), 변경 lib strict exit 0(1.55초), 대상 fmt/diff exit 0입니다. stat와 독립적으로 정상/빈/20MiB/초과 reader를 검사했습니다. 원인·범위는 `docs/bug/2026-10-02-native-html-growing-input.md`입니다. 기존 성공은 재사용하고 다른 provider를 이 좁은 수정으로 완료 처리하지 않습니다.

- [ ] NativeApplication composition root의 정확한 helper executable·별도 bridge 연결/종료·close/generation/invalidation/cache admission.
- [x] source RAII owner·Unix root 제한 resource·MIME·single range/큰 media의 filesystem/stream 기반 — `2026-10-02-m8-native-web-resource-boundary.md`입니다. HTTP·강제 CSP·navigation/new-window/권한/bridge·Windows와 실제 renderer는 미완료입니다.
- [ ] HTML/Audio/Video의 별도 WebView와 실제 bounds/focus/hide/close·원본 controls/theme/locale/view parity.
- [ ] OS sandbox·DOM/renderer peak RSS/CPU·crash/runtime teardown·실제 corpus/pixel/GPU/AX 및 상위 M8 gate.
