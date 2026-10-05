# M8 HWP3 bounded parse와 native host

후속 현재 상태: 이 문서의 마지막 raw snapshot/Core 재파싱은 초기 수명 조사 이력입니다. 현재 production은 `2026-10-02-m8-native-hwp-persistent-core.md`의 공유 Mutex Core·actual retained admission을 사용하며 page 재파싱은 제거됐습니다. 단일 입력 parse·원본 initializer·이 문서의 나머지 형식/corpus/RSS/CPU/isolation gate는 그대로입니다.

## 대상·완료 범위

`native/taide-native-app/src/preview_hwp{,_preflight}.rs`, `vendor/rhwp/src/parser/hwp3/{mod,ole}.rs`, `vendor/rhwp/src/serializer/mini_cfb.rs`, `vendor/rhwp/src/document_core/commands/document.rs`, `tests/preview-hwp3-boundary.rs`입니다. N5-P1g의 확장 형식 기본 입력 연결이며 전체 HWP provider/M8 완료가 아닙니다.

- [x] 앱의 `load_core`가 HWP3를 encoded 20MiB, metadata+decoded body+중첩 stream+재포장 OLE의 합산 64MiB 정책으로 읽습니다. body decode 예산은 실제 metadata 길이를 먼저 뺍니다. node/grid cap과 공유 기본 depth 32를 재사용하고 encrypted 입력은 거절합니다.
- [x] 정확한 Additional id=2와 원본 OLE signature 두 값에만 validation hook을 적용합니다. 임의 body magic 검색은 하지 않습니다. OLE CFB constructor 전에 기존 physical FAT/DIFAT·bounded stream·중첩 container·이름/entry 검사를 실행합니다. unknown OLE signature의 기존 skip은 유지합니다.
- [x] OLE 추출은 borrowed Cursor와 한 번 수집한 bounded entry 목록, declared/actual stream 길이와 aggregate read/output 예산을 사용합니다. mini CFB는 원래 builder와 동일한 결과를 내며 FAT/output vector 생성 전에 정확한 output 크기를 확인합니다. builder 내부의 bounded 입력 복제까지 제거했다는 뜻은 아닙니다.
- [x] `DocumentCore::from_parsed`로 기존 normalization/style/composition/pagination/metadata 초기화 본문을 공유합니다. 앱 HWP3는 bounded parser가 만든 IR을 바로 넘기며 Core의 from_bytes를 다시 호출하지 않습니다. 다른 형식은 기존 byte admission/from_bytes 경로를 유지합니다. public `admit`의 byte-only API가 HWP3를 반환하는 것은 아니며 제품 진입점은 `load_core`입니다.
- [x] actual typed host의 승인 worker·SourceReady·RGBA·공유 snapshot·프로젝트 폐쇄 재승인·disconnect·작업자 회수를 확인했습니다. 합성 fixture 디렉터리만 생성·정리했으며 사용자 앱·bundle·설정·원본 데이터는 유지했습니다.

## 실행 증거

공통 Cargo 인자는 `--manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`입니다. Cargo는 직렬 실행했고 같은 성공을 반복하지 않았습니다.

1. 초기 lib `cargo check … --lib`: exit 0, 6.15초입니다.
2. 신규 `cargo test … --test preview-hwp3-boundary hwp3_native`: fixture가 실제 Raster.size/BinDataBytes API 대신 없는 필드를 사용해 E0609/E0599 compile 실패했습니다. 실제 타입을 확인·정정한 뒤 3 PASS, compile 0.91초/suite 2.38초입니다. plain/deflate 페이지·동일 RGBA와 ink, exact OLE/repack stream/크기·entry cap/반복 FAT 거절·unknown signature skip, encoded/decoded/중첩 stream 합산과 encrypted/depth 거절을 각각 덮습니다.
3. 신규 `cargo test … --test preview-hwp3-boundary hwp3_host`: 1 PASS, compile 1.08초/suite 0.61초입니다. 닫힌 project의 cached snapshot은 Forbidden이고 종료 뒤 tracked worker는 0입니다.
4. 신규 `cargo test … --test preview-hwp3-boundary hwp_initializer`: fixture의 font/style prefix 누락으로 InvalidArgument 실패(suite 0.00초)했습니다. 제품 cap을 낮추거나 예외를 열지 않고 완전한 body fixture로 정정한 해당 검사만 1 PASS, compile 1.09초/suite 0.22초입니다. 실제 2MiB thread에서 앱의 최대 허용 중첩 31·compressed decode→Core→raster, 공유 Core initializer의 HML metadata/page/동일 legacy SVG를 확인합니다.
5. strict의 constant chunks_exact 지적은 기존 as_chunks 패턴으로 수정했습니다. lib/bin/관련 test `cargo clippy … --lib --bin taide-native-app --test preview-hwp3-boundary -- -D warnings` exit 0(0.43초), 최종 fixture 수정 뒤 해당 test target strict만 exit 0(0.44초)입니다. dependency의 모든 자체 lint/unit test가 실행됐다는 뜻은 아닙니다.

원본 tag의 Core initializer/mini CFB source와 no-index diff를 대조했습니다. 대상 Rust 파일의 rustfmt check·전체 working diff check는 exit 0입니다. byte-only HWP5/HWPX/ViewText/HML·기존 context/grid/margin 성공은 영향 없는 범위에서 재사용합니다. 새 dependency/package/lock/root MSRV 변경은 없습니다.

## 남은 gate

- [ ] 기존 CFB 0.15 admission과 원본 0.14 lenient parser 차이, 정상/비표준 corpus·legacy 그림/OLE 포함 문서·전체 원본 pixel/font 동등성입니다. nested CFB physical 검사에는 기존 XLS helper의 20MiB encoded 제한도 적용됩니다.
- [ ] logical decoded/node/grid cap은 전체 IR/composed/cache/renderer peak RSS·CPU 상한이 아닙니다. 재포장 builder의 bounded buffer 복제, SVG 생성 이전의 image/OLE 변환, cancellation/crash isolation·긴 session gate가 남습니다.
- [ ] 현재 raw snapshot을 공유하지만 페이지를 바꿀 때 Core를 다시 만듭니다. 원본의 탭 소유 document 수명과 같은 persistent engine/정확한 회수·메모리 admission을 구현해야 합니다. Core는 Send지만 RefCell 때문에 Sync가 아니며 unchecked Sync/Arc 우회는 사용하지 않습니다.
- [ ] 실제 OS/GPU/AX·N1~N8/Rust99%·TS 제거/배포/전체 완료 감사 뒤의 commit/push는 미완료입니다.

## 다음 수명 경계의 source 근거

원본 `src/features/preview/hwp-preview.tsx`는 data 변경 때 document를 한 번 생성하고 page 이동에는 같은 document를 사용하며 cleanup에서 free합니다. 현재 native `preview_hwp::Source`는 canonical/bytes만 보유해 page 요청마다 Core를 다시 만듭니다. `preview_hwp_cache::Cache`의 논리 retained 비용은 deduplicated raw snapshot+texture이며 Core IR을 포함하지 않습니다.

vendor의 DocumentCore Send assertion과 RefCell cache를 직접 확인했습니다. retained size API는 없으며 document/styles/composed/pagination/measured/render-normalization/page-tree 등의 실제 소유 저장소가 있습니다. 따라서 단순 Arc<Core>/임의 고정 추정값으로 persistent cache를 추가하면 안 됩니다. 표준 Mutex의 안전한 단일 접근 소유권과 실제 retained IR/cache admission·close/invalidation 회수를 먼저 구현해야 합니다. 아직 해당 구현이나 검증을 했다는 뜻은 아닙니다.
