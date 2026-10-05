# M8 native 인라인 SVG 미리보기

## 대상과 구현

대상은 `native/taide-native-app/{src/preview_svg.rs,tests/preview-formats.rs}`입니다. 기존 정지 SVG decoder의 data resolver에 nested SVG를 연결했습니다. 기존 승인 read worker·RGBA cache·File 탭을 그대로 사용하며 새 dependency·제품 TS·사용자 실기 bundle 변경은 없습니다.

1. 이미 설치된 usvg/resvg 0.48.1의 `Tree::from_data_nested`와 `ImageKind::SVG`를 사용합니다. 정확한 설치 source와 [공식 Tree 문서](https://docs.rs/usvg/0.48.1/usvg/struct.Tree.html#method.from_data_nested)를 확인했습니다. nested tree는 외부 string resolver를 None으로 유지하고 data resolver만 기존 승인 closure로 전달합니다. 파일·file URL·HTTP URL을 읽는 기본 resolver를 사용하지 않습니다.
2. MIME가 `image/svg+xml` 또는 `text/plain`이고 raster magic이 없을 때만 nested SVG를 파싱합니다. PNG/JPEG/GIF/WebP의 기존 content/size 검사를 유지합니다. malformed nested XML·잘못된 MIME·외부 entity·renderer side 제한을 넘는 child는 포함하지 않습니다. 원본 외부 SVG 자체가 잘못된 XML이면 기존처럼 오류입니다.
3. 중첩은 최대 16단계입니다. embedded encoded 합산은 기존 encoded 20MiB 안으로, embedded intrinsic RGBA 추정은 64MiB 안으로 제한합니다. nested vector renderer는 부모 크기의 임시 pixmap을 만드는 upstream source를 확인했으므로, 렌더 전 `root canvas bytes × (peak nesting + 1) + embedded intrinsic bytes`가 64MiB를 넘으면 오류를 반환합니다. 4096×4096의 root에 nested vector가 붙는 합성 문서는 pixel buffer allocation 전에 거절됩니다.
4. 이 상한은 prototype 보호입니다. encoded data의 base64 decode는 resolver 호출 전에 upstream에서 일어나며 XML/tree/font·path/filter·decoder backing·demultiplied output의 모든 임시 할당까지 완전히 제한한 것은 아닙니다. rejected/malformed child의 reservation도 반환하지 않아 보수적으로 계산합니다. 16단계/64MiB를 넘는 정상 문서의 동등성은 아직 미완료입니다.

## 실제 검증

- 먼저 기존 nested SVG의 기대 픽셀을 원본 문서의 green으로 수정했습니다. 최초 검사 1건은 실제 white≠green으로 실패했습니다(compile 1.69초·suite 0.81초). 이전 구현이 inline SVG를 생략하던 동작을 재현한 것입니다. 기대 픽셀을 되돌려 통과시키지 않았습니다.
- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-formats svg_embedded -- --nocapture` — 구현 뒤 1건 PASS(compile 2.01초·suite 0.21초). 실제 nested green 픽셀, 기존 인라인 PNG와 side 제한, 두 단계 vector nesting, 실제 존재하는 합성 외부 PNG·file URL·HTTP URL 생략, script 비실행, nested PNG, text/plain fallback·잘못된 MIME·malformed child·외부 entity, 16단계 허용/17단계 leaf 생략, 큰 canvas aggregate 거절입니다. 네트워크 syscall이나 실제 OS 창을 관찰한 검사는 아닙니다.
- [x] 같은 기본 명령의 `--test preview-formats svg_nested_raster -- --nocapture` — 추가된 독립 child-risk 1건 PASS(compile 0.78초·suite 0.20초). green child background 위의 실제 인라인 PNG가 white로 덮는 픽셀과, 1×1 root 안의 intrinsic 2×1 child가 side 제한으로 생략되는 픽셀을 확인했습니다. 앞선 성공 명령을 재실행하지 않았습니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app --test preview-formats -- -D warnings` — exit 0(0.66초).

앞선 actual static host·PNG/cache·animation·AVIF/ICC 성공은 source 영향이 없는 범위에서 재사용합니다. 이 새 검사는 direct decoder/headless 픽셀 검사이며 actual host·GPU 전체 재검사로 보고하지 않습니다.

## 남은 gate

- [ ] SVG animation·CSS/foreignObject·external resource 계약·embedded ICC/AVIF·noninteger/DPI 품질과 실제 GPU/AX parity를 구현·판정합니다.
- [ ] node/entity expansion·path/filter 임시 메모리, encoded aggregate 실제 거절, streaming/tiling·cancel·crash isolation과 정상 대형 SVG 처리 정책을 검증합니다.
- [ ] 큰 GIF/APNG/WebP animation·animated ICC/AVIF와 나머지 7종 preview provider·dirty/editor 전환·OS 창·전체 N5/M8를 완료합니다.

bounded 인라인 SVG의 좁은 구현/검증만 완료입니다. M8 상위 N1부터 N8은 0/8 완료이며 전체 완료 전 commit/push하지 않았습니다.
