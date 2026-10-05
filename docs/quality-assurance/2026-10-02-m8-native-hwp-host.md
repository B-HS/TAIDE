# M8 HWP 승인 worker·cache·File surface 기본 연결

후속 현재 상태: 아래는 최초 raw snapshot 기본 연결의 실행 이력입니다. 현재 production 경로는 `2026-10-02-m8-native-hwp-persistent-core.md`의 공유 Mutex Core와 실제 typed source 비용으로 대체됐습니다. page 재파싱/encoded-only 비용의 초기 차이는 해소됐으며 원본 corpus·전체 RSS/CPU/crash isolation·실기는 계속 미완료입니다.

## 대상과 구현

`native/taide-native-app/src/preview_hwp{,_cache,_surface}.rs`, `src/preview_status.rs`, `src/host.rs`, `src/application.rs`, `src/lib.rs`와 `tests/preview-hwp-host.rs`입니다. N5-P1g의 HWP5/HWPX 기본 연결이며 전체 형식/corpus/제품 완료는 아닙니다. 엔진 source·admission·page/SVG·표 오류 결과는 같은 날짜의 HWP preview/preflight/engine-boundary QA를 재사용합니다.

1. typed ReadHwpPreview→공유 approved read→SourceReady→engine/page/SVG/raster→HwpPreview를 연결했습니다. 기존 TaskSupervisor/owned guard·absolute/canonical 프로젝트 또는 CLI 승인·작업 전후 승인 재확인과 종료 경계를 재사용합니다. 캐시된 바이트를 요청해도 승인을 다시 확인하고 canonical identity가 다르면 거절합니다. 비신뢰 문서/시크릿을 Debug에 노출하지 않습니다.
2. 동일 경로 split 탭은 Arc 원본 바이트를 공유하되 page 선택·texture는 독립입니다. encoded source는 identity별 한 번, 픽셀은 탭별로 합산하며 다른 image/PDF/presentation/spreadsheet와 128MiB 논리 cache budget을 공유합니다. 결과 generation/path/loading/page와 실제 pixel length·renderer side·animation/page metadata·overflow/cache cap을 검사합니다. empty/no-raster 성공에도 ready marker를 유지해 무한 재요청하지 않습니다.
3. actual application File tab의 Hwp 분기를 연결했습니다. theme는 동일한 PDF preview appearance 값을 실제 두 번째 소비자로 재사용하며 기존 HWP locale catalog·이전/다음·0-based page 선택/1-based 표시·양방향 scroll·자연 크기/max-width/16px 세로 여백·loading/error/no-pages·external-open을 연결합니다. no-pages는 실패/외부 열기 버튼 없이 경고 메시지만 보입니다. render 실패는 빈 화면입니다. image alt는 해당 페이지 표시입니다.
4. 파일/프로젝트 invalidation은 page 0으로 재설정하고 source/texture를 회수합니다. close/reconcile·stale SourceReady/reply·host 제출 실패/pending reset·종료 reply를 처리합니다. native read worker에서 DocumentCore를 만들고 렌더 후 drop하며 UI/cache에 비-Sync core를 unchecked 공유하지 않습니다.

## 실제 검증

Cargo 공통 인자는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 한 번에 하나씩 실행하고 같은 성공은 재사용했습니다.

- [x] 최초 app/bin strict는 surface의 미사용 Color32 import로 실패해 해당 import만 제거했습니다. 검사 억제는 없습니다.
- [x] 최초 `cargo test ... --test preview-hwp-host`: 2건 중 locale/키보드/empty/blank/error/cache 1 PASS, actual host 1 FAIL(compile 5.82초/suite 1.13초). 프로젝트 폐쇄는 실제로 거절됐지만 테스트가 localized Forbidden을 구체 enum variant와 비교했습니다. 기존 root_guard의 공개 error.kind 경계로 테스트를 수정했습니다.
- [x] 변경된 actual host filter만 `cargo test ... --test preview-hwp-host hwp의_실제_host`: 1 PASS(compile 1.06초/suite 0.49초)입니다. actual engine으로 생성한 2-page HWPX·page 렌더·source progress 1회·동일 source 공유/독립 선택·bytes 변경 전 snapshot·invalidation 후 새 HWP/page 0·stale source/reply/close·다른 canonical identity/밖/relative/broken/닫힌 프로젝트 거절·host/task shutdown 0 tracked를 확인했습니다. 성공한 UI 검사는 다시 실행하지 않았습니다.
- [x] 최종 `cargo clippy ... --lib --bin taide-native-app --test preview-hwp-host -- -D warnings` exit 0, 2.08초입니다. 실제 File surface 배선은 app/bin compile로 확인했고 native 화면 함수는 headless egui로 검사했습니다. 실제 OS 창/픽셀/AX 관찰로 주장하지 않습니다.
- [x] 대상 Rust 포맷에서 lib.rs 모듈 순서만 실패해 해당 파일만 skip_children으로 정렬하고 lib.rs만 재검사 exit 0입니다. 나머지 대상 파일의 성공은 재사용했습니다. 대상 HWP QA/bug/vendor provenance MD formatter와 `git diff --check` exit 0입니다. 전체 vendor source 재포맷·사용자 bundle 빌드·commit/push는 하지 않았습니다.

## 남은 gate와 다음 경계

- [ ] 원본 browser는 core를 한 번 유지하지만 현재 native 기본 worker는 page마다 원본 snapshot을 재파싱합니다. raw source/texture 회수·비-Sync core 경계는 명확하나 page 전환 CPU/latency와 engine/layout 재할당을 원본 동등 성능으로 계산하지 않습니다. N6 전에 lifecycle·실제 retained IR/cache 크기를 검토해 persistent worker document 또는 동등한 구조를 완성해야 합니다. encoded/pixel len 합산도 Vec capacity/GPU/worker in-flight/전체 RSS 상한은 아닙니다.
- [x] 후속 배포용 ViewText bounded decrypt/decompress·같은 record/table cap·extended header 수정·native 기본 렌더는 `2026-10-02-m8-native-hwp-distribution.md`에 기록했습니다.
- [ ] HWP3/HML admission, 큰 표/문서/strict-lenient/ZIP 차이·embedded resource/전체 corpus·font/줄바꿈/픽셀 동등성은 남습니다. 기본 거절/상한을 호환 완료로 처리하지 않습니다.
- [ ] 전체 payload 내부 allocation·CPU/cancellation/crash isolation·실제 OS/GPU/AX·scroll CSS 동등성·전체 제품 다른 provider/terminal·N1~N8/Rust99%/TS 제거/배포는 미완료입니다. 사용자 실기 bundle/프로세스/입력기/VoiceOver/설정/데이터를 변경하지 않았습니다. M8 전체 완료 뒤만 commit/push합니다.
