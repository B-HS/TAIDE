# rhwp native source 출처

## Source와 license

이 디렉터리는 [edwardkim/rhwp](https://github.com/edwardkim/rhwp/tree/9b16aa9e23f476e2b335d7c029fc9f24a199d63c)의 MIT-licensed v0.8.2 commit `9b16aa9e23f476e2b335d7c029fc9f24a199d63c`를 고정한 vendor입니다. 기존 TypeScript HWP preview가 사용하는 엔진 버전과 같습니다.

정확한 revision에서 가져온 경로는 `src`, `Cargo.toml`, `LICENSE`, `THIRD_PARTY_LICENSES.md`, production serializer template `saved/blank2010.hwp`입니다. 움직이는 branch를 사용하지 않으며 나머지 tools/sample/browser assets는 가져오지 않았습니다.

- `LICENSE` SHA-256: `1c3a7d5643b163a3ead4965e1bea33b832caee5bfca265efe42afcd7bc696b5b`.
- `saved/blank2010.hwp` SHA-256: `43f472751fafefb8c66b0f831660ebed51e443a46bebe44c75bba918154ce4c9`; 13824 bytes입니다.

## 국소 변경

1. vendor manifest에서 가져오지 않은 `tools/rhwp-subsecond` workspace member만 제거했습니다. consuming app은 이 nested workspace를 exclude하고 정확한 version/path dependency와 default features off를 사용합니다.
2. `src/document_core/commands/document.rs`의 from_bytes에서 `RHWP_EXP_BODY_FRESH` 환경 실험 블록을 제거했습니다. 원본 browser는 native parent environment를 상속하지 않으므로 부모 변수 때문에 저장된 line segment를 조용히 삭제하면 안 됩니다.
3. `src/model/table.rs::Table::rebuild_grid`의 row/column span을 더하기 전에 넓히고 실제 declared grid까지 반복을 자릅니다. 비정상 u16 span은 기존 debug 경로에서 overflow했고 grid 밖 반복이 가능했습니다. 정상 범위 merged cell 동작은 유지합니다.
4. `src/parser/crypto.rs::decrypt_viewtext_section`의 AES body 시작은 첫 record의 원래 encoded size marker로 결정합니다. extended header에서 parsed payload size 256을 비교하던 원본은 8바이트 대신 4바이트 header로 계산해 AES ciphertext 시작을 잘못 잡았습니다. 정상/extended synthetic ViewText의 plain/deflate/zlib 결과 일치를 확인했습니다.
5. `src/parser/hwp3/{mod,records,ole,drawing}.rs`에서 실제 정보 블록 범위를 확인하고 해당 범위 안에서만 metadata를 읽습니다. 추가 이미지에는 실제 payload offset인 32바이트 header가 필요합니다. 두 slice panic을 합성 입력으로 재현·수정했습니다. 가변 record/그림/textbox/OLE/field는 64KiB씩 실제 읽은 만큼 확대하고 drawing point는 거짓 개수만으로 capacity를 예약하지 않습니다. 공개 `decode_body`가 raw/deflate에 caller 예산을 적용하며 원본 parser는 기존 record hard cap인 256MiB를 본문에도 적용합니다. 이는 아직 앱의 64MiB admission·중첩/grid/전체 IR cap 완료가 아니며 HWP3 app load 연결은 후속 작업입니다.

6. `src/parser/hwp3/limits.rs`의 명시적 parse context를 paragraph/control/drawing group/textbox에 공유합니다. caller 지정 decoded/depth/node/합산 grid 제한을 IR 생성 전에 확인하고 drawing의 LimitExceeded는 전파합니다. 실제 2MiB worker stack의 depth 128 SIGABRT를 재현해 기본 depth를 32로 제한했으며 허용 마지막 경계의 Core/page/SVG도 확인했습니다. 앱 admission은 아직 미연결입니다.
7. HWP3 table/cell/picture/page-border margin의 4배 변환을 명시적 16비트 wrapping으로 구현합니다. 음수·큰 margin의 debug panic을 제거하면서 원본 optimized release 식의 -4 결과와 cell overwrite를 유지합니다. 단순 checked rejection이나 overflow 검사기 비활성화로 동작을 바꾸지 않습니다.
8. HWP3 validation hook은 metadata/decoded body, 정확한 Additional OLE, 재포장 결과를 consuming app에 전달합니다. caller의 remaining budget을 OLE read/output에 적용하고 body decode 전에 metadata 길이를 뺍니다. OLE는 borrowed input·bounded entry/declared+actual stream/output을 사용하며 `mini_cfb::build_cfb_with_limit`는 정확한 출력 크기를 FAT/output 할당 전에 확인합니다. `DocumentCore::from_parsed`는 기존 초기화 본문을 공유해 bounded IR을 재파싱하지 않게 합니다. 기존 from_bytes/serializer API와 metadata/normalization 본문은 유지합니다.
9. 기본 비활성 `native-retained` feature에서 TAIDE local helper를 사용합니다. model과 실제 Core/style/composed/pagination/measured/render/layout·HML metadata의 소유 field에 조건부 derive를 연결했으며 private field·serde skip field·편집 snapshot/clipboard·pending pagination state도 생략하지 않습니다. loaded BinData는 실제 Vec capacity를 더하고 외부 CFB/ZIP lazy resolver는 아직 `Opaque` 오류를 냅니다. 비용은 typed retained payload이며 RSS·allocator/HashMap bucket/GPU 상한이 아닙니다. parser/render semantics나 원본 default feature는 변경하지 않았고 production persistent-cache admission은 아직 미연결입니다.

10. HWP5/HWPX Lazy resolver는 local cfb 0.14.0/zip 8.6.0의 선택적 source 기반 adapter를 방문합니다. resolver Arc identity를 공유하고 Cursor 원본, 내부 FAT/directory/archive metadata와 소유 child를 계산합니다. 기존 custom resolver의 새 선택적 메서드는 기본 Opaque이며 materialize하지 않습니다. public read/decompress/parse semantics는 변경하지 않았습니다. helper feature는 consuming native MSRV 1.95를 요구하며 원본 feature-off MSRV를 새 feature의 보장으로 주장하지 않습니다. 각 fork의 UPSTREAM.md에 정확한 registry source revision·license·변경 범위를 기록합니다.

다른 imported Rust source의 변경은 의도하지 않았습니다. engine 갱신 시 이 차이를 국소 유지하고 vendor tree 전체를 재포맷하지 않습니다.

## Consumer 경계

TAIDE는 bounded HWP5/HWPX·배포용 ViewText·HML byte admission과 HWP3의 단일 bounded parse→shared Core initializer를 연결하고 `render_page_svg_legacy_native`를 명시적으로 호출합니다. HWP3 exact OLE CFB는 engine constructor 전에 consuming app의 physical/stream/중첩 예산으로 확인합니다. file-external-image 보충 API를 호출하지 않으며 기존 제한된 native SVG decoder로 rasterize합니다.

Cargo는 dependency workspace의 root patch를 승계하지 않습니다. upstream svg2pdf Git-fork patch 대신 app lock의 registry svg2pdf 0.13.0으로 native library compile을 확인했습니다. PDF export는 사용/검증하지 않았습니다. Skia/subsecond/default panic-hook features는 켜지 않았으며 binding dependency compile이 이 adapter의 JS/WASM runtime 사용을 뜻하지 않습니다.

논리 input/grid cap은 전체 engine peak RSS/CPU 상한이 아닙니다. native font 측정·legacy 형식·큰 문서 호환·cancellation/crash isolation·원본 전체 렌더 동등성은 M8 gate에 남습니다.
