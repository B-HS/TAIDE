# M8 native GIF·APNG·WebP 기본 애니메이션

## 대상과 연결

대상은 `native/taide-native-app/src/{preview,preview_animation,preview_svg,application,lib}.rs`, `tests/{preview-animation,preview,preview-events}.rs`와 격리 Cargo manifest/lock입니다. 원본 image의 browser animation을 승인된 기존 worker와 실제 File 탭 texture 재생에 연결했습니다. TS 원본·제품 manifest/MSRV와 사용자 실기 bundle·시스템 설정은 유지합니다.

1. GIF·APNG·animated WebP를 content로 구분하고 고정 image 0.25.10의 composited frame iterator를 worker 안에서 소비합니다. 프레임별 orientation·dimension·RGBA를 확인하며 전체 저장 픽셀은 64MiB, frame 수는 4096개로 제한합니다. 정지 PNG/JPEG/WebP/BMP/SVG의 기존 경로는 유지합니다.
2. image의 GIF loop_count는 반복 정보 없는 Finite(0)를 Infinite로 변환합니다. 기존 transitive gif 0.14.2를 직접 재사용하여 LZW decode 없는 frame metadata 순회 후 실제 repeat 값을 읽습니다. 정보 없으면 한 번, 명시적 무한은 무한, 유한 repeat N은 초기 재생을 포함해 N+1번입니다. [공식 WebKit GIF decoder](https://raw.githubusercontent.com/WebKit/WebKit/main/Source/WebCore/platform/image-decoders/gif/GIFImageDecoder.cpp)의 absent/finite 경계와 비교했습니다. APNG/WebP의 num_plays는 해당 iterator metadata를 사용합니다. 실제 macOS ImageIO와의 전체 비교는 남습니다.
3. 11ms 미만 delay는 100ms로 표시합니다. [공식 WebKit macOS decoder](https://raw.githubusercontent.com/WebKit/WebKit/main/Source/WebCore/platform/graphics/cg/ImageDecoderCG.cpp)의 frameDurationAtIndex 경계를 적용했습니다. fractional delay는 image의 rational→Duration 변환을 사용합니다.
4. 실제 AppSurfaces가 Cache::advance를 호출합니다. 첫 표시의 egui monotonic time을 기준으로 누적 duration을 이진 탐색하여 현재 frame을 고르고, 필요한 때 같은 texture ID를 갱신합니다. 유한 종료는 마지막 frame을 유지하며 다음 repaint를 요청하지 않습니다. 긴 표시 공백은 누적 시간으로 건너뛰고, 보이지 않는 surface 자체에서 animation repaint를 돌리지 않습니다.
5. cache는 저장한 frame RGBA와 GPU texture 추정 bytes를 함께 기존 128MiB admission에 셉니다. host reply에서 frame 길이·개수·delay·play metadata를 다시 검증하며 close/invalidation은 frame storage와 texture를 함께 폐기합니다. stale loading token은 upload 전에 거절합니다. png 0.18.1은 합성 APNG 작성용으로 기존 lock의 dev 직접 edge만 사용하며 gif/png 새 버전·package는 추가하지 않았습니다.

## 실제 검증과 정정

- [x] 기본 명령: `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-animation -- --nocapture`. 최초 2건 중 partial disposal/absent·infinite·짧은 delay·불량 frame 회신 검사 1건 PASS, actual host 검사 1건 FAIL, suite 0.02초·compile 4.77초입니다. 실패는 픽셀 helper가 font atlas texture를 선택한 원인이며 제품 frame/loop assertion은 그 지점 전에 통과했습니다.
- [x] helper를 해당 image texture ID로 한정했습니다. 최초 수정의 과도한 dereference는 E0614로 compile 실패했고 실제 TextureId reference로 정정했습니다. 영향받은 `--test preview-animation 실제_host -- --nocapture`만 1회 성공했습니다. 1건 PASS, suite 0.02초·compile 0.82초. 승인된 실제 디스크 GIF/APNG/WebP→host 왕복→합성 frame 시간 0/0.04/0.06/0.16/0.21/0.31/10초의 texture pixel·2회 반복 종료·동일 texture ID·close·renderer 제한과 shutdown/tracked 0입니다. 실제 macOS GPU 창에서 관찰한 검사는 아닙니다.
- [x] 신규 `--test preview-animation animation_admission -- --nocapture`: 1건 PASS, suite 0.01초·compile 0.93초. frame 상한·zero delay·zero plays·불량 metadata의 texture 미생성·다음 admission, 제거된 loading animation의 stale upload 거절을 검사했습니다.
- [x] 변경된 decode/cache의 `--test preview --test preview-events --test preview-formats 실제_host -- --nocapture`: PNG 실제 host 1건 PASS(0.01초), 추가 static 형식 실제 host 1건 PASS(0.23초), compile 1.25초. preview-events는 filter로 0건 실행이므로 새 PASS로 세지 않습니다. 변경 없는 외부 entity/메뉴/registry/event/Copy/Cut/PTY 성공은 재사용했습니다.
- [x] lib/bin 및 preview-animation/preview/preview-events/preview-formats strict clippy exit 0(0.35초). 최초 manual_is_multiple_of 경고는 동일 표준 메서드로 수정했습니다. 검사기를 끄지 않았습니다. 신규 admission test 이후 해당 test의 strict clippy도 exit 0(0.36초)입니다. 변경된 Rust 파일만 rustfmt로 정리했습니다.

초기 app compile의 실패는 존재하지 않는 PngDecoder::new_with_limits와 immutable AppSurfaces cache reference였습니다. 설치된 공식 API with_limits와 실제 mutable cache 소유 경계로 수정한 lib/bin check는 exit 0(1.09초)입니다. 하나의 성공 검사를 통합 이유로 반복하지 않았으며 실제 입력/code가 바뀐 범위만 재검사했습니다.

## 전체 gate와 구분

- [ ] 현 구현은 전체 프레임을 먼저 decode하는 bounded prototype입니다. 64MiB/4096개를 넘는 정상 animation을 거절할 수 있어 큰 animation의 streaming/eviction/downsample과 첫 표시 latency parity가 필요합니다. 한 프레임 추가 decode 시점의 임시 할당, compositing 버퍼·Vec capacity·ColorImage/GPU 업로드 중복·metadata·전체 reply 자원과 decoder crash 격리는 strict process memory 증명에 포함해야 합니다.
- [ ] actual tab close/project close/worker cancellation·shutdown의 decode 중단 지연, reconnect/file-change 전체 수명과 actual OS 여러 창·숨김/복원·sleep/wake·GPU/AX·locale/theme를 확인합니다. 같은 window/path의 여러 pane은 현재 공유 texture/timeline이므로 원본의 개별 DOM image 재생 phase 동등성도 남습니다.
- [ ] GIF Background/alpha·인터레이스·APNG hidden default frame·16-bit/color management·WebP alpha/blend/disposal·fractional timing의 추가 원본/실기 fixture와 AVIF·ICC·inline/dynamic SVG를 완료합니다. Previous disposal의 정확한 합성 픽셀은 확인했지만 모든 format/compositing gate 통과로 확장하지 않습니다.
- [ ] 나머지 7종 preview provider와 M8 N1부터 N8을 완료합니다. 기존 TS/Monaco fallback을 제거하지 않았으며 M8 완료·commit/push는 아닙니다.

현재 완료는 bounded 기본 animation의 코드 연결과 narrow host/texture proof입니다.
