# M8 native 추가 raster와 정적 SVG 미리보기

## 대상과 구현

대상은 `native/taide-native-app/{Cargo.toml,Cargo.lock,src/preview.rs,src/preview_svg.rs,src/lib.rs,tests/preview-formats.rs}`입니다. 원본 `src/features/preview/image-preview.tsx`의 브라우저 이미지 표시를 기존 승인 read worker와 native texture 경로로 확장합니다.

1. 기존 image 0.25.10의 JPEG/GIF/WebP/BMP feature를 활성화합니다. PNG를 포함한 content magic, encoded 20MiB, renderer dimension과 RGBA 64MiB 제한을 decode 전에 확인하고 JPEG EXIF 방향을 적용합니다. GIF/WebP/PNG는 이 검사 시점에 정지 첫 프레임입니다.
2. SVG 파서와 rasterizer는 기존 표준/설치 패키지로 대체할 수 없어 격리 app에 resvg 0.48.1을 추가했습니다. 최초 0.47.0 조회 뒤 Cargo가 최신 0.48.1을 알려 주어 최신 호환 버전으로 올렸습니다. 해당 upstream은 MIT/Apache-2.0이고 MSRV 1.85입니다. native app 1.95·root 1.89와 제품 manifest는 유지합니다. API는 설치된 정확한 버전의 source와 [공식 resvg 문서](https://docs.rs/resvg/0.48.1/resvg/)로 확인했습니다.
3. SVG 외부 href resolver는 파일과 URL 모두 None을 반환합니다. UTF-8 XML을 먼저 검사하고 시스템 font database를 worker에서 한 번 읽습니다. 외부 entity resolver의 기본 None을 확인했습니다. script 실행 엔진은 없으며 raster alpha는 demultiplied RGBA로 전달합니다. SVGZ 압축 feature는 활성화하지 않았습니다.
4. 인라인 PNG/JPEG/GIF/WebP는 content와 dimensions를 확인하고 aggregate RGBA 추정 64MiB 안에서만 받습니다. 인라인 SVG는 현재 생략되며 동등성 미완료입니다. font·XML tree·filter 중간 버퍼와 raster decoder 내부 메모리를 이 RGBA 상한으로 완전히 제한했다고 주장하지 않습니다.

## 실제 검증

- [x] `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-formats 실제_host -- --nocapture`: 실제 host 1건 PASS, suite 1.13초·compile 11.58초. 승인된 합성 디스크 파일 5형식, 픽셀/alpha, 각 형식의 truncated/renderer 제한 거절, 실제 외부 합성 PNG href 생략, script/URL 생략, EXIF orientation 6의 2×1→1×2, 큰 SVG와 잘못된 root 거절, disconnect/shutdown/tracked 0입니다. OS network syscall 관측이나 실제 GPU 창 검사는 아닙니다.
- [x] `--test preview-formats svg_embedded -- --nocapture`: 1건 PASS, suite 0.23초·compile 1.16초. 인라인 PNG 실제 픽셀, 제한을 넘는 인라인 raster 생략, 인라인 SVG의 현재 생략, 참조된 외부 entity 오류를 확인했습니다. 이 실행은 기존 transitive base64 0.23.1의 dev 직접 edge를 lock에 반영하기 위해 `--locked`만 생략했으며 새로운 base64 버전은 추가하지 않았습니다.
- [x] 위 기본 명령의 `--test preview -- --nocapture`: 변경된 decoder의 PNG host/texture 회귀 1건 PASS, suite 0.01초·compile 0.95초. 기존 메뉴/registry/event/Copy/Cut/PTY 성공은 반복하지 않았습니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app --test preview --test preview-formats -- -D warnings`: exit 0, 1.27초.

초기 lib compile의 E0505는 embedded data의 decoder borrow가 Arc 이동까지 남은 원인이었습니다. dimensions 검증 뒤 decoder를 명시적으로 drop하여 수정했습니다. assertion과 검사기는 완화하지 않았습니다. dependency fetch는 sandbox DNS 실패 후 필요한 registry 조회만 escalation했습니다. 종료 전 fetch와 새 fetch가 잠시 겹친 실행 실수는 모두 terminal 상태로 정리했고 후속 Cargo는 직렬로 실행했습니다.

## 남은 gate

이하 항목은 최초 정지 raster/SVG 검사 시점의 경계입니다. 이후 bounded animation은 `2026-10-02-m8-native-animated-image-preview.md`, macOS AVIF·정지 ICC는 `2026-10-02-m8-native-avif-and-static-icc-preview.md`, bounded inline SVG는 `2026-10-02-m8-native-inline-svg-preview.md`의 실제 결과로 진행됐습니다. 최초 성공 증거를 전체 동등성 완료로 확장하지 않습니다.

- [ ] GIF/APNG/WebP animation의 delay·loop·disposal·정지/close/hidden surface 수명과 bounded decode/texture를 연결합니다.
- [ ] AVIF와 ICC/color management, inline SVG, SVG animation/외부 리소스 parity와 noninteger/DPI 품질을 구현·판정합니다. image의 AVIF encoding feature는 AVIF decode 지원이 아니므로 동일시하지 않습니다. [image 공식 codecs 문서](https://docs.rs/image/0.25.10/image/codecs/index.html)의 decode 경계를 확인했습니다.
- [ ] 큰 파일 tiling/downsample, XML node/필터/전체 임시 할당, 외부 파일 교체·성장 TOCTOU, decoder crash isolation과 aggregate CPU/GPU memory를 검증합니다.
- [ ] 실제 AppSurfaces/OS GPU/AX, editor↔preview와 dirty 상태, 나머지 7종 provider·3개 locale/theme와 전체 N5/M8를 완료합니다. 사용자 실기 앱/bundle·시스템 설정·TS 원본은 변경하지 않았습니다.

정지 raster/SVG의 narrow proof만 완료입니다. N5-P1과 M8 상위 N1부터 N8은 미완료이며 commit/push를 하지 않았습니다.
