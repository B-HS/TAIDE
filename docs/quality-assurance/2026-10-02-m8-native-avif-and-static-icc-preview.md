# M8 native AVIF와 정지 ICC 미리보기

## 대상과 구현

대상은 `native/taide-native-app/{Cargo.toml,Cargo.lock,src/lib.rs,src/preview.rs,src/preview_macos.rs,tests/preview-macos.rs}`입니다. macOS에서 AVIF와 ICC가 있는 정지 raster를 기존 승인 blocking worker·File 탭·RGBA cache에 연결했습니다. 제품 TS와 사용자 실기 bundle은 변경하지 않았습니다.

1. 기존 CoreFoundation/CoreGraphics 0.3.2를 직접 재사용하고 시스템 ImageIO의 Rust binding `objc2-image-io` 0.3.2를 격리 app에 추가합니다. 별도 C AVIF decoder를 설치하지 않습니다. ImageIO는 Zlib/Apache-2.0/MIT 중 선택 가능하고 MSRV 1.71입니다. 정확한 API는 설치된 공식 binding source와 [ImageIO binding 문서](https://docs.rs/objc2-image-io/0.3.2/objc2_image_io/)를 확인했습니다.
2. 기존 image crate에 AVIF encoder/decoder가 활성화돼 있지 않아 합성 AVIF 생성에만 `ravif` 0.13.0을 dev dependency로 사용합니다. BSD-3-Clause, MSRV 1.85이며 asm/threading 기본 feature는 끕니다. [공식 ravif 문서](https://docs.rs/ravif/0.13.0/ravif/)와 설치 source를 확인했습니다. offline lock 갱신에서 encoder와 관련 패키지 39건이 추가됐습니다. production AVIF 경로는 ravif를 사용하지 않습니다. root MSRV 1.89·native 1.95·제품 manifest는 유지합니다.
3. 승인된 메모리의 encoded bytes만 CFData로 넘깁니다. URL/file resolver를 만들지 않습니다. 기존 encoded 20MiB·live owner/root 재검사·shutdown을 유지하며, metadata에서 양의 정수 dimensions와 renderer side·RGBA 64MiB 상한을 실제 decode 전에 검사합니다. ImageIO thumbnail transform으로 EXIF 방향을 적용하고, 명시적 sRGB bitmap으로 ICC 색을 변환합니다. premultiplied RGBA를 cache 계약인 unassociated RGBA로 되돌립니다. malformed ICC는 ImageIO의 기본 색상 fallback입니다.
4. ImageIO가 CGImage와 Complete 상태를 반환하더라도 lazy pixel materialization은 실패할 수 있습니다. 실제 provider data를 먼저 요청하고 stride×height의 checked 길이를 검증합니다. materialization 실패는 오류로 반환하며 빈 투명/검정 이미지를 성공으로 보고하지 않습니다. CFData 검사 복사본은 output allocation 전에 drop하지만 decoder·padded backing·encoded 복사와 모든 임시 할당을 64MiB로 완전히 제한한 것은 아닙니다.

## 신뢰 경계와 unsafe 근거

ImageIO의 properties dictionary는 CFString key·CFType value 계약으로 typed cast하며, dimension 값은 실제 CFNumber downcast와 checked integer 변환 뒤 사용합니다. 직접 CFNumber로 무검증 cast하지 않습니다. options는 CFString key와 CFBoolean/CFNumber value로 생성합니다. bitmap data pointer는 checked RGBA 길이로 초기화한 Vec의 수명 안에서만 사용하며 CGContext를 drop한 뒤 Vec를 읽고 수정합니다. rows는 checked dimensions의 width×4입니다. CoreGraphics/CoreFoundation 참조는 retained wrapper로 유지합니다.

## 실제 실패·원인·검증

- 초기 compile에서 opaque CFDictionary의 get/downcast가 지원되지 않아 E0277/E0599가 발생했습니다. typed dictionary와 CFNumber runtime 검증으로 수정했습니다. deprecated byte-order 상수도 대체했습니다. 이후 lib/bin check는 exit 0(1.21초)입니다.
- 최초 actual host 검사 1건은 AVIF 첫 픽셀 0≠255로 실패했습니다(compile 15.38초·suite 0.37초). 기존 ICC PNG/WebP/JPEG assertion은 앞서 실행됐으나 이 실패한 검사를 전체 PASS로 세지 않습니다.
- 진단에서 원본 decode와 thumbnail 모두 `[0,0,0,0]`을 반환했고 status는 Complete였습니다(0.47초). provider data를 조사한 다음 진단은 None unwrap으로 실패했습니다(0.36초). 그 뒤 큰 fixture의 진단은 실행되지 않았습니다. 임시 diagnostic code는 제거했으며 작은 AVIF fixture·기대 흰색·허용 오차 3을 바꾸지 않았습니다.
- 동일 actual host 검사를 실행 sandbox 밖에서 한 번 실행하니 1건 PASS(compile 1.50초·suite 0.49초)였습니다. 관찰로 입증된 원인은 실행 sandbox 환경의 macOS AVIF pixel materialization 제한입니다. 구체적인 codec 서비스/Mach 이름은 조사하지 않았으므로 특정 서비스가 차단됐다고 단정하지 않습니다.
- [x] materialization guard 수정 뒤 sandbox 검사: `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --test preview-macos avif_코덱 -- --nocapture` — 1건 PASS(compile 2.18초·suite 0.32초). 이용 가능하면 정확한 흰색/알파를, 이용 불가하면 정확한 materialization 오류를 요구합니다. AVIF 성공 픽셀 증거와 구분합니다.
- [x] 수정 뒤 actual host 검사: 위 기본 명령의 `--test preview-macos 실제_host -- --nocapture`를 승인된 sandbox 밖에서 실행 — 1건 PASS(compile 0.16초·suite 0.32초). 합성 AVIF·ICC PNG/WebP/JPEG의 승인 파일 왕복, 독립 linear→sRGB 픽셀·알파, renderer limit·truncated 거절, EXIF 90° 회전, malformed ICC fallback·plain PNG와 disconnect/shutdown/tracked 0입니다. 실제 OS 창/GPU 검사가 아닙니다.
- strict clippy 최초는 constant `chunks_exact_mut`의 새 lint로 실패했습니다. MSRV에 맞는 `as_chunks_mut`로 바꾸고 assertion/검사기는 유지했습니다.
- [x] `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app --test preview-macos -- -D warnings` — 수정 뒤 exit 0(1.21초). 동작이 같은 chunk API 변경 후 앞선 pixel 성공을 재사용했습니다. PNG/menu/animation/PTY 성공 검사를 반복하지 않았습니다.

## 남은 gate

후속 bounded inline SVG 구현/검증은 `2026-10-02-m8-native-inline-svg-preview.md`에 기록합니다. 아래 inline SVG 항목은 해당 최초 AVIF 검사 시점의 잔여 범위이며 전체 SVG gate는 여전히 미완료입니다.

- [ ] animated AVIF·animated ICC·HDR/gain map·고비트/CMYK/wide-gamut 전체 fidelity와 non-macOS decoder를 구현·판정합니다. 현재 ImageIO count가 1이 아닌 파일은 명시적으로 거절합니다. 기존 GIF/APNG/WebP animation은 유지하지만 ICC 변환은 아직 연결되지 않았습니다.
- [ ] source 전체/aggregate decoder memory, streaming·tiling/downsample·cancel·crash isolation, OS codec unavailable/권한 오류 UX·재시도와 실제 GPU/색 표시를 검증합니다.
- [ ] inline SVG·SVG animation/외부 리소스 계약, 나머지 7종 preview provider·editor↔preview/dirty·다중 창·AX·전체 N5/M8를 완료합니다.

macOS AVIF·정지 ICC의 좁은 구현/검증만 완료입니다. M8 상위 N1부터 N8은 0/8 완료이며 전체 완료 전 commit/push하지 않았습니다.
