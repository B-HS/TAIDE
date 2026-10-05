# M8 HWP native engine 기본 page 렌더

## 대상과 구현

`native/taide-native-app/src/preview_hwp.rs`, `src/preview_hwp_preflight.rs`, `src/lib.rs`, app manifest/lock, `vendor/rhwp`와 `tests/preview-hwp{,-table-budget}.rs`입니다. N5-P1g의 기본 엔진 연결 기록입니다. 후속 approved worker/cache/File surface의 결과는 `2026-10-02-m8-native-hwp-host.md`에 기록했으며 전체 HWP provider 완료는 아닙니다.

1. 원본 @rhwp/core와 같은 v0.8.2 공개 commit `9b16aa9e23f476e2b335d7c029fc9f24a199d63c`의 Rust source·MIT/third-party notice·production blank template를 고정했습니다. 정확한 import 경로·hash·세 가지 국소 packaging/source 변경은 `native/taide-native-app/vendor/rhwp/UPSTREAM.md`에 기록했습니다.
2. path dependency/default features off로 native library를 빌드했습니다. 처음 nested workspace 충돌은 공식 Cargo workspace 규칙에 따라 app의 exclude와 vendor member 목록으로 해결했습니다. offline dependency 부족 후 필요한 패키지를 정상 조회해 native lock에 36개 package를 추가했습니다. root manifest/lock/MSRV는 이 단위에서 변경하지 않았습니다. app의 기존 MSRV 1.95와 source 버전을 유지합니다.
3. admission→DocumentCore→page count→명시적 legacy SVG→기존 제한된 SVG/raster 경로를 구현했습니다. 페이지 상한은 4096, 생성 SVG 상한은 64MiB입니다. 두 검사는 각각 pagination/문자열 생성 후이므로 생성 전 allocation 제한이라고 주장하지 않습니다. render 실패는 원본처럼 빈 페이지, 범위 밖 page는 오류입니다. 외부 파일 이미지 보충 API는 호출하지 않습니다.
4. HWP TABLE payload/HWPX tbl rowCnt·colCnt의 경계 행·열 포함 grid slot을 합산 262144로 제한합니다. upstream의 셀 grid cap만으로 renderer의 행×열 layout 버퍼가 제한되지 않는 경로를 확인했습니다. 셀 u16 row/col/span 덧셈 overflow는 vendor의 widened/clipped 반복으로 수정했습니다. 상세는 `docs/bug/2026-10-02-native-hwp-table-grid.md`입니다.

## 실제 검증

Cargo는 한 번에 하나씩 실행했습니다. 공통 옵션은 `--manifest-path native/taide-native-app/Cargo.toml --target-dir experiments/native-shell-spike/target`이며, 최초 의존성 fetch/check 뒤에는 `--locked --offline`을 사용했습니다.

- [x] `cargo check ... --lib`: native rhwp/app build exit 0, 24.65초입니다. upstream PDF fork patch 미승계와 registry svg2pdf 0.13.0 compile은 확인했지만 PDF export는 검증하지 않았습니다.
- [x] `cargo test ... --test preview-hwp`: 1 PASS, compile 36.59초/suite 1.21초입니다. production blank에서 합성 한글·일본어 텍스트를 삽입해 실제 engine의 HWP/HWPX export를 만들고, 양 형식의 load/page count/SVG/raster·검은 opaque ink·natural 크기 일치·RGBA 길이·잘못된 page·너무 작은 raster 예산·잘린 입력을 확인했습니다. 원본 문서 전체 corpus/pixel 비교는 아닙니다.
- [x] 새 `preview-hwp-table-budget` 2건은 수정 전 모두 RED였습니다. 실제 `Table::rebuild_grid` u16 덧셈 panic과 admission의 거대 tbl 허용을 확인했습니다(compile 1.01초/suite 0.00초). 수정 후 같은 관련 target만 2 PASS(compile 5.99초/suite 0.00초)입니다. HWP5/HWPX 거대 metadata 거절·개별 허용 크기 두 표의 aggregate 거절·정상 작은 표 admission·비정상 span과 정상 셀 grid를 포함합니다. 거대 layout을 실제 할당시키지는 않았습니다.
- [x] 최초 strict의 테스트 chunks_exact 경고는 app MSRV가 지원하는 `as_chunks`로 수정했습니다. 최종 `cargo clippy ... --lib --bin taide-native-app --test preview-hwp --test preview-hwp-table-budget -- -D warnings` exit 0, 3.36초입니다. 검사 억제는 추가하지 않았습니다. 변경 없는 admission/XML/HTML/XLS 성공 결과는 재사용했습니다.

## 남은 gate

- [x] 후속 기본 approved read worker·stale/close·snapshot/cache·File surface·previous/next·no-pages/loading/error/external-open은 host QA에 기록했습니다. native worker는 DocumentCore를 렌더 후 drop하며 비-Sync core를 unchecked 공유하지 않습니다. persistent core 성능·실제 parser 취소/isolation은 별도 잔여입니다.
- [ ] decoded/metadata/page/SVG/raster 논리 상한은 전체 IR/layout/font/engine cache/GPU/peak RSS 상한이 아닙니다. 더 큰 정상 표·문서와 distribution/HWP3/HML·strict/lenient·ZIP 버전 차이·전체 corpus를 거절 정책으로 대체해 완료 처리하지 않습니다.
- [ ] native EmbeddedTextMeasurer와 원본 browser Canvas 경로의 font/줄바꿈/픽셀 동등성·CPU·cancellation/crash isolation·실제 OS/GPU/AX 검증이 남습니다. N1~N8·Rust99%·TS 제거·제품 배포는 미완료이며 사용자 실기 bundle/설정/데이터는 변경하지 않았습니다.
