# M8 HWP persistent Core와 논리 retained admission

## 구현과 원본 근거

`native/taide-native-app/src/preview_hwp{,_cache}.rs`, 실제 host 검사와 새 lib unit, local retained helper의 PathBuf adapter가 대상입니다. 원본 `src/features/preview/hwp-preview.tsx`는 data당 HwpDocument를 한 번 생성하고 페이지 이동에 재사용하며 cleanup에서 free합니다. 이제 native Source는 별도 raw Vec가 아니라 `Mutex<Option<Document>>`를 소유하고 Snapshot Arc를 같은 경로 view끼리 공유합니다. Core의 Send/RefCell !Sync를 안전한 Mutex로 보호하며 unsafe Sync/강제 unlock을 쓰지 않습니다.

1. 새 source는 한 번만 bounded parser/Core initializer를 호출합니다. 같은 snapshot의 다음 페이지는 원래 Core를 사용합니다. raw read 임시 Vec는 load가 끝나면 해제되며 Lazy CFB/ZIP이 실제로 소유한 원본 buffer는 비용에 포함합니다. page 선택/texture는 view별 독립입니다.
2. typed Document와 Source의 모든 field를 derive로 연결합니다. 빈 Source Arc를 측정한 base에는 canonical PathBuf의 실제 capacity·Arc layout·Mutex/Option/Atomic/inline field가 포함됩니다. Document 전체 비용에서 이미 Source inline에 들어간 Document inline 크기만 빼고 child 비용을 더합니다. 초기 및 렌더 후 실제 전체 Source 측정과 이 값이 같음을 검사했습니다. 허구의 고정 Core 비용·raw input len으로 대신하지 않습니다.
3. Core와 실제 private cache를 64MiB/262,144 visits의 논리 source 예산으로 확인하고 렌더 뒤 갱신합니다. 같은 source는 cache에서 한 번, 픽셀은 탭별로 계산하며 다른 provider와 기존 128MiB 논리 예산을 공유합니다. overflow는 거절하고 Source 자체 또는 cache budget 실패 시 공유 Core를 폐기합니다. 다른 view의 이미 승인된 픽셀은 유지되지만 폐기된 source로 후속 렌더는 오류가 됩니다. poison recovery는 폐기에만 사용하고 해당 Core로 처리를 재개하지 않습니다.
4. 원본 read→SourceReady→decode 진행 순서는 유지합니다. cached Core도 매 요청 absolute/canonical/root 승인을 앞뒤로 재확인합니다. 파일 invalidation은 모든 해당 view source를 제거하고 page 0으로 초기화합니다. close는 해당 owner만 회수하며 마지막 cache/request/worker owner가 없으면 Core가 해제됩니다. 이미 수행 중인 worker 참조를 강제로 해제하지 않습니다.

source의 post-load/post-render 논리 admission은 할당 전 전체 peak/RSS 상한이 아닙니다. retained traversal 임시 queue·allocator/private HashMap/IndexMap bookkeeping·global font/GPU allocation도 source payload 값에 포함하지 않습니다. 동일 parser/CPU/crash isolation과 corpus/pixel·OS/GPU/AX·N6 성능 gate는 남습니다. 전체 HWP/N5/M8 완료나 제품 TS 제거로 계산하지 않습니다.

## 실제 검증

Cargo는 직렬이며 공통 인자는 정확한 manifest와 `--locked --offline --target-dir experiments/native-shell-spike/target`입니다. app direct local helper dependency edge의 lock 반영 check 한 번만 locked 없이 실행했고 third-party 버전/root manifest/lock/MSRV는 이 단위에서 변경하지 않았습니다.

- [x] PathBuf 실제 capacity 증가 신규 helper 1 PASS, compile 0.42초·suite 0.00초. 설치된 Rust 1.98.1의 PathBuf→OsString→bytes/WTF8 source capacity API를 확인했습니다. helper workspace/all-targets strict exit 0, 0.31초입니다.
- [x] 새 `--lib hwp_persistent_core` 1 PASS, compile 12.78초·suite 0.79초입니다. 초기/렌더 후 정확한 full Source 비용, 같은 Core/render cache 재사용, 공유 view·첫/마지막 close·worker owner의 실제 Weak 생존/해제, 실제 loaded Vec capacity의 source budget 폐기·재렌더 거절, shared source의 전체 cache budget 폐기, canonical PathBuf allocation 상한을 검사했습니다.
- [x] 변경된 `--test preview-hwp-host hwp의_실제_host` 1 PASS, compile 6.62초·suite 0.50초입니다. 실제 typed host의 2페이지 HWPX·공유 비용 1회·독립 page/texture·disk bytes 교체 후 기존 Core 유지·invalidation 뒤 새 Core/page 0·stale reply/close·cached Core 프로젝트 승인 폐기·disconnect·tracked worker 0을 확인했습니다. 원래 raw bytes 비용 assertion만 실제 source 비용으로 교체했습니다.
- [x] app lib check exit 0(6.53초). 최초 strict에서 generic `invalid` 호출의 needless borrow 두 곳이 실패해 빌림만 제거했습니다. 관련 lib/bin 및 모든 test target strict `-D warnings` 최종 exit 0(4.93초)입니다. 성공 runtime 검사는 반복하지 않았습니다. 이전 helper/IR/Core/HWP3/HML/CFB/ZIP/표/암호/locale/surface 결과는 변경되지 않은 경계에서 재사용합니다.

사용자 실기 앱/bundle·OS 입력/VoiceOver/clipboard·사용자 문서/프로필은 조작하지 않았으며 M8 전체 완료 전 commit/push하지 않았습니다.
